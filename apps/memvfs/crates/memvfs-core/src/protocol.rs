use serde::{Deserialize, Serialize};

use crate::data::{BlockDebug, BlockId};
use crate::error::FsError;
use crate::fs::{FileSystem, OpenFileDebug};
use crate::index::{DirEntry, InodeDebug};
use crate::types::{FileMode, FileStat, OpenFlags};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DebugKind {
    Inodes,
    Blocks,
    Free,
    OpenFiles,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Request {
    Ping,
    Shutdown,
    Mkdir {
        path: String,
        mode: Option<FileMode>,
    },
    Open {
        path: String,
        flags: OpenFlags,
        mode: Option<FileMode>,
    },
    Close {
        fd: u64,
    },
    Read {
        fd: u64,
        len: usize,
    },
    Write {
        fd: u64,
        bytes: Vec<u8>,
    },
    Seek {
        fd: u64,
        offset: i64,
    },
    Truncate {
        path: String,
        size: u64,
    },
    ReadFile {
        path: String,
    },
    WriteFile {
        path: String,
        bytes: Vec<u8>,
    },
    Ls {
        path: String,
    },
    Stat {
        path: String,
    },
    Rename {
        from: String,
        to: String,
    },
    Unlink {
        path: String,
    },
    Rmdir {
        path: String,
    },
    Debug {
        kind: DebugKind,
    },
    StatFs,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Response {
    Pong,
    Bye,
    Unit,
    Fd(u64),
    Bytes(Vec<u8>),
    Count(usize),
    Offset(u64),
    DirEntries(Vec<DirEntry>),
    Stat(FileStat),
    Inodes(Vec<InodeDebug>),
    Blocks(Vec<BlockDebug>),
    FreeBlocks(Vec<BlockId>),
    OpenFiles(Vec<OpenFileDebug>),
    StatFs(StatFs),
    Error(RpcError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RpcError {
    pub code: String,
    pub message: String,
}

impl From<FsError> for RpcError {
    fn from(error: FsError) -> Self {
        Self {
            code: error.code.as_str().to_string(),
            message: error.message,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatFs {
    pub block_size: usize,
    pub total_blocks: usize,
    pub used_blocks: usize,
    pub free_blocks: usize,
    pub inode_count: usize,
    pub inode_capacity: usize,
}

pub fn handle_request(fs: &mut FileSystem, request: Request) -> Response {
    let result = match request {
        Request::Ping => return Response::Pong,
        Request::Shutdown => return Response::Bye,
        Request::Mkdir { path, mode } => fs.mkdir(&path, mode).map(|()| Response::Unit),
        Request::Open { path, flags, mode } => fs.open(&path, flags, mode).map(Response::Fd),
        Request::Close { fd } => fs.close(fd).map(|()| Response::Unit),
        Request::Read { fd, len } => fs.read(fd, len).map(Response::Bytes),
        Request::Write { fd, bytes } => fs.write(fd, &bytes).map(Response::Count),
        Request::Seek { fd, offset } => fs.seek(fd, offset).map(Response::Offset),
        Request::Truncate { path, size } => fs.truncate_path(&path, size).map(|()| Response::Unit),
        Request::ReadFile { path } => fs.read_file(&path).map(Response::Bytes),
        Request::WriteFile { path, bytes } => fs.write_file(&path, &bytes).map(Response::Count),
        Request::Ls { path } => fs.readdir(&path).map(Response::DirEntries),
        Request::Stat { path } => fs.stat(&path).map(Response::Stat),
        Request::Rename { from, to } => fs.rename(&from, &to).map(|()| Response::Unit),
        Request::Unlink { path } => fs.unlink(&path).map(|()| Response::Unit),
        Request::Rmdir { path } => fs.rmdir(&path).map(|()| Response::Unit),
        Request::Debug { kind } => match kind {
            DebugKind::Inodes => Ok(Response::Inodes(fs.debug_inodes())),
            DebugKind::Blocks => Ok(Response::Blocks(fs.debug_blocks())),
            DebugKind::Free => Ok(Response::FreeBlocks(fs.debug_free_blocks())),
            DebugKind::OpenFiles => Ok(Response::OpenFiles(fs.debug_open_files())),
        },
        Request::StatFs => Ok(Response::StatFs(fs.statfs())),
    };

    match result {
        Ok(response) => response,
        Err(error) => Response::Error(error.into()),
    }
}
