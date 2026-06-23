use std::io::{Read, Write};
use std::net::TcpStream;

use clap::{Args, Parser, Subcommand};
use memvfs_core::{DebugKind, OpenFlags, Request, Response};

#[derive(Debug, Parser)]
#[command(name = "memvfs", about = "CLI client for the memvfs daemon")]
struct Cli {
    #[arg(long, global = true, default_value = "127.0.0.1:7878")]
    addr: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Ping,
    Stop,
    Mkdir(PathArgs),
    Ls(PathArgs),
    Stat(PathArgs),
    Read(PathArgs),
    Write(WriteArgs),
    Truncate(TruncateArgs),
    Mv(MvArgs),
    Rm(PathArgs),
    Rmdir(PathArgs),
    Open(OpenArgs),
    Close(FdArgs),
    FdRead(FdReadArgs),
    FdWrite(FdWriteArgs),
    Seek(SeekArgs),
    Debug(DebugArgs),
    Statfs,
}

#[derive(Debug, Args)]
struct PathArgs {
    path: String,
}

#[derive(Debug, Args)]
struct WriteArgs {
    path: String,

    #[arg(long)]
    text: Option<String>,

    #[arg(long)]
    hex: Option<String>,
}

#[derive(Debug, Args)]
struct TruncateArgs {
    path: String,

    #[arg(long)]
    size: u64,
}

#[derive(Debug, Args)]
struct MvArgs {
    from: String,
    to: String,
}

#[derive(Debug, Args)]
struct OpenArgs {
    path: String,

    #[arg(long)]
    read: bool,

    #[arg(long)]
    write: bool,

    #[arg(long)]
    create: bool,

    #[arg(long)]
    truncate: bool,

    #[arg(long)]
    append: bool,
}

#[derive(Debug, Args)]
struct FdArgs {
    fd: u64,
}

#[derive(Debug, Args)]
struct FdReadArgs {
    fd: u64,
    len: usize,
}

#[derive(Debug, Args)]
struct FdWriteArgs {
    fd: u64,
    text: String,
}

#[derive(Debug, Args)]
struct SeekArgs {
    fd: u64,
    offset: i64,
}

#[derive(Debug, Args)]
struct DebugArgs {
    #[command(subcommand)]
    kind: DebugCommand,
}

#[derive(Debug, Subcommand)]
enum DebugCommand {
    Inodes,
    Blocks,
    Free,
    OpenFiles,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("memvfs: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let request = build_request(cli.command)?;
    let response = send_request(&cli.addr, &request)?;
    print_response(response)?;
    Ok(())
}

fn build_request(command: Commands) -> Result<Request, Box<dyn std::error::Error>> {
    Ok(match command {
        Commands::Ping => Request::Ping,
        Commands::Stop => Request::Shutdown,
        Commands::Mkdir(args) => Request::Mkdir {
            path: args.path,
            mode: None,
        },
        Commands::Ls(args) => Request::Ls { path: args.path },
        Commands::Stat(args) => Request::Stat { path: args.path },
        Commands::Read(args) => Request::ReadFile { path: args.path },
        Commands::Write(args) => {
            let WriteArgs { path, text, hex } = args;
            Request::WriteFile {
                path,
                bytes: write_bytes(text, hex)?,
            }
        }
        Commands::Truncate(args) => Request::Truncate {
            path: args.path,
            size: args.size,
        },
        Commands::Mv(args) => Request::Rename {
            from: args.from,
            to: args.to,
        },
        Commands::Rm(args) => Request::Unlink { path: args.path },
        Commands::Rmdir(args) => Request::Rmdir { path: args.path },
        Commands::Open(args) => Request::Open {
            path: args.path,
            flags: OpenFlags {
                read: args.read || !args.write,
                write: args.write,
                create: args.create,
                truncate: args.truncate,
                append: args.append,
            },
            mode: None,
        },
        Commands::Close(args) => Request::Close { fd: args.fd },
        Commands::FdRead(args) => Request::Read {
            fd: args.fd,
            len: args.len,
        },
        Commands::FdWrite(args) => Request::Write {
            fd: args.fd,
            bytes: args.text.into_bytes(),
        },
        Commands::Seek(args) => Request::Seek {
            fd: args.fd,
            offset: args.offset,
        },
        Commands::Debug(args) => Request::Debug {
            kind: match args.kind {
                DebugCommand::Inodes => DebugKind::Inodes,
                DebugCommand::Blocks => DebugKind::Blocks,
                DebugCommand::Free => DebugKind::Free,
                DebugCommand::OpenFiles => DebugKind::OpenFiles,
            },
        },
        Commands::Statfs => Request::StatFs,
    })
}

fn write_bytes(
    text: Option<String>,
    hex: Option<String>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    match (text, hex) {
        (Some(text), None) => Ok(text.into_bytes()),
        (None, Some(hex)) => decode_hex(&hex),
        (None, None) => Err("write requires --text or --hex".into()),
        (Some(_), Some(_)) => Err("write accepts only one of --text or --hex".into()),
    }
}

fn decode_hex(hex: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let normalized = hex.trim();
    if normalized.len() % 2 != 0 {
        return Err("hex input must have an even number of characters".into());
    }
    let mut bytes = Vec::with_capacity(normalized.len() / 2);
    for index in (0..normalized.len()).step_by(2) {
        bytes.push(u8::from_str_radix(&normalized[index..index + 2], 16)?);
    }
    Ok(bytes)
}

fn send_request(addr: &str, request: &Request) -> Result<Response, Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect(addr)?;
    write_frame(&mut stream, request)?;
    read_frame(&mut stream)
}

fn print_response(response: Response) -> Result<(), Box<dyn std::error::Error>> {
    match response {
        Response::Pong => println!("pong"),
        Response::Bye => println!("bye"),
        Response::Unit => println!("ok"),
        Response::Fd(fd) => println!("{fd}"),
        Response::Bytes(bytes) => print!("{}", String::from_utf8_lossy(&bytes)),
        Response::Count(count) => println!("{count}"),
        Response::Offset(offset) => println!("{offset}"),
        Response::DirEntries(entries) => {
            for entry in entries {
                println!("{}\t{:?}\t{}", entry.inode, entry.kind, entry.name);
            }
        }
        Response::Stat(stat) => println!("{}", serde_json::to_string_pretty(&stat)?),
        Response::Inodes(inodes) => println!("{}", serde_json::to_string_pretty(&inodes)?),
        Response::Blocks(blocks) => println!("{}", serde_json::to_string_pretty(&blocks)?),
        Response::FreeBlocks(blocks) => println!("{}", serde_json::to_string_pretty(&blocks)?),
        Response::OpenFiles(files) => println!("{}", serde_json::to_string_pretty(&files)?),
        Response::StatFs(statfs) => println!("{}", serde_json::to_string_pretty(&statfs)?),
        Response::Error(error) => {
            return Err(format!("{}: {}", error.code, error.message).into());
        }
    }
    Ok(())
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
    if bytes.len() > u32::MAX as usize {
        return Err("frame is too large".into());
    }
    writer.write_all(&(bytes.len() as u32).to_be_bytes())?;
    writer.write_all(&bytes)?;
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use memvfs_core::{DebugKind, OpenFlags};

    use super::{build_request, Cli, Request};

    #[test]
    fn write_text_builds_write_file_request() {
        let cli = Cli::parse_from(["memvfs", "write", "/docs/hello.txt", "--text", "hello"]);

        let request = build_request(cli.command).unwrap();

        assert_eq!(
            request,
            Request::WriteFile {
                path: "/docs/hello.txt".to_string(),
                bytes: b"hello".to_vec(),
            }
        );
    }

    #[test]
    fn write_hex_builds_binary_write_file_request() {
        let cli = Cli::parse_from(["memvfs", "write", "/bytes", "--hex", "00ff7a"]);

        let request = build_request(cli.command).unwrap();

        assert_eq!(
            request,
            Request::WriteFile {
                path: "/bytes".to_string(),
                bytes: vec![0x00, 0xff, 0x7a],
            }
        );
    }

    #[test]
    fn write_rejects_missing_payload() {
        let cli = Cli::parse_from(["memvfs", "write", "/empty"]);

        let error = build_request(cli.command).unwrap_err();

        assert_eq!(error.to_string(), "write requires --text or --hex");
    }

    #[test]
    fn open_defaults_to_read_when_write_is_absent() {
        let cli = Cli::parse_from(["memvfs", "open", "/notes.txt"]);

        let request = build_request(cli.command).unwrap();

        assert_eq!(
            request,
            Request::Open {
                path: "/notes.txt".to_string(),
                flags: OpenFlags {
                    read: true,
                    write: false,
                    create: false,
                    truncate: false,
                    append: false,
                },
                mode: None,
            }
        );
    }

    #[test]
    fn open_write_create_truncate_disables_implicit_read() {
        let cli = Cli::parse_from([
            "memvfs",
            "open",
            "/notes.txt",
            "--write",
            "--create",
            "--truncate",
        ]);

        let request = build_request(cli.command).unwrap();

        assert_eq!(
            request,
            Request::Open {
                path: "/notes.txt".to_string(),
                flags: OpenFlags {
                    read: false,
                    write: true,
                    create: true,
                    truncate: true,
                    append: false,
                },
                mode: None,
            }
        );
    }

    #[test]
    fn debug_free_builds_debug_request() {
        let cli = Cli::parse_from(["memvfs", "debug", "free"]);

        let request = build_request(cli.command).unwrap();

        assert_eq!(
            request,
            Request::Debug {
                kind: DebugKind::Free,
            }
        );
    }
}
