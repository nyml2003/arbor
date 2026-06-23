use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use memvfs_core::{DebugKind, FileKind, OpenFlags, Request, Response};

struct Daemon {
    child: Child,
    addr: String,
}

impl Daemon {
    fn start() -> Self {
        let addr = free_addr();
        let child = Command::new(env!("CARGO_BIN_EXE_memvfsd"))
            .args([
                "start",
                "--addr",
                &addr,
                "--block-size",
                "4",
                "--capacity",
                "64",
                "--inode-capacity",
                "32",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn memvfsd");

        let mut daemon = Self { child, addr };
        daemon.wait_until_ready();
        daemon
    }

    fn request(&self, request: Request) -> Response {
        send_request(&self.addr, &request)
    }

    fn wait_until_ready(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match send_request_result(&self.addr, &Request::Ping) {
                Ok(Response::Pong) => return,
                Ok(other) => panic!("unexpected readiness response: {other:?}"),
                Err(error) if Instant::now() < deadline => {
                    if let Some(status) = self.child.try_wait().expect("poll memvfsd") {
                        panic!("memvfsd exited before readiness: {status}");
                    }
                    let _ = error;
                    thread::sleep(Duration::from_millis(50));
                }
                Err(error) => panic!("memvfsd did not become ready: {error}"),
            }
        }
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(Some(_))) {
            return;
        }

        let _ = send_request_result(&self.addr, &Request::Shutdown);
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn daemon_preserves_filesystem_state_across_rpc_connections() {
    let daemon = Daemon::start();

    assert_eq!(
        daemon.request(Request::Mkdir {
            path: "/docs".to_string(),
            mode: None,
        }),
        Response::Unit
    );
    assert_eq!(
        daemon.request(Request::WriteFile {
            path: "/docs/hello.txt".to_string(),
            bytes: b"hello".to_vec(),
        }),
        Response::Count(5)
    );
    assert_eq!(
        daemon.request(Request::ReadFile {
            path: "/docs/hello.txt".to_string(),
        }),
        Response::Bytes(b"hello".to_vec())
    );

    let Response::DirEntries(entries) = daemon.request(Request::Ls {
        path: "/docs".to_string(),
    }) else {
        panic!("ls should return directory entries");
    };
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "hello.txt");
    assert_eq!(entries[0].kind, FileKind::File);

    let Response::StatFs(statfs) = daemon.request(Request::StatFs) else {
        panic!("statfs should return filesystem stats");
    };
    assert_eq!(statfs.block_size, 4);
    assert_eq!(statfs.used_blocks, 2);
    assert_eq!(statfs.free_blocks, 14);
    assert_eq!(statfs.inode_count, 3);
}

#[test]
fn daemon_keeps_open_file_descriptors_between_rpc_calls() {
    let daemon = Daemon::start();

    let Response::Fd(fd) = daemon.request(Request::Open {
        path: "/notes.txt".to_string(),
        flags: OpenFlags::READ_WRITE_CREATE,
        mode: None,
    }) else {
        panic!("open should return a file descriptor");
    };

    assert_eq!(
        daemon.request(Request::Write {
            fd,
            bytes: b"abcd".to_vec(),
        }),
        Response::Count(4)
    );
    assert_eq!(
        daemon.request(Request::Seek { fd, offset: 1 }),
        Response::Offset(1)
    );
    assert_eq!(
        daemon.request(Request::Write {
            fd,
            bytes: b"Z".to_vec(),
        }),
        Response::Count(1)
    );
    assert_eq!(
        daemon.request(Request::Seek { fd, offset: 0 }),
        Response::Offset(0)
    );
    assert_eq!(
        daemon.request(Request::Read { fd, len: 4 }),
        Response::Bytes(b"aZcd".to_vec())
    );
    assert_eq!(daemon.request(Request::Close { fd }), Response::Unit);
}

#[test]
fn daemon_returns_rpc_errors_and_shuts_down_cleanly() {
    let mut daemon = Daemon::start();

    let Response::Error(error) = daemon.request(Request::ReadFile {
        path: "/missing.txt".to_string(),
    }) else {
        panic!("missing file should return an RPC error");
    };
    assert_eq!(error.code, "ENOENT");

    assert_eq!(daemon.request(Request::Shutdown), Response::Bye);
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if matches!(daemon.child.try_wait(), Ok(Some(_))) {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("memvfsd did not exit after shutdown");
}

#[test]
fn daemon_debug_views_report_allocated_blocks_and_open_files() {
    let daemon = Daemon::start();

    let Response::Fd(fd) = daemon.request(Request::Open {
        path: "/debug.txt".to_string(),
        flags: OpenFlags::READ_WRITE_CREATE,
        mode: None,
    }) else {
        panic!("open should return a file descriptor");
    };
    assert_eq!(
        daemon.request(Request::Write {
            fd,
            bytes: b"abcde".to_vec(),
        }),
        Response::Count(5)
    );

    let Response::Blocks(blocks) = daemon.request(Request::Debug {
        kind: DebugKind::Blocks,
    }) else {
        panic!("debug blocks should return block data");
    };
    assert_eq!(blocks.iter().filter(|block| block.used).count(), 2);

    let Response::OpenFiles(files) = daemon.request(Request::Debug {
        kind: DebugKind::OpenFiles,
    }) else {
        panic!("debug open-files should return open file data");
    };
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].fd, fd);
    assert_eq!(files[0].offset, 5);
}

fn free_addr() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind dynamic port");
    listener
        .local_addr()
        .expect("read dynamic port")
        .to_string()
}

fn send_request(addr: &str, request: &Request) -> Response {
    send_request_result(addr, request).expect("send request")
}

fn send_request_result(
    addr: &str,
    request: &Request,
) -> Result<Response, Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect(addr)?;
    write_frame(&mut stream, request)?;
    read_frame(&mut stream)
}

fn read_frame<T: serde::de::DeserializeOwned>(
    reader: &mut impl Read,
) -> Result<T, Box<dyn std::error::Error>> {
    let mut len = [0; 4];
    reader.read_exact(&mut len)?;
    let len = u32::from_be_bytes(len) as usize;
    let mut bytes = vec![0; len];
    reader.read_exact(&mut bytes)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn write_frame<T: serde::Serialize>(
    writer: &mut impl Write,
    value: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = serde_json::to_vec(value)?;
    writer.write_all(&(bytes.len() as u32).to_be_bytes())?;
    writer.write_all(&bytes)?;
    writer.flush()?;
    Ok(())
}
