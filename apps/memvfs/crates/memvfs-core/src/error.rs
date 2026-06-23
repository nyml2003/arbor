use serde::{Deserialize, Serialize};

pub type FsResult<T> = Result<T, FsError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FsErrorCode {
    Enoent,
    Eexist,
    Enotdir,
    Eisdir,
    Enotempty,
    Eacces,
    Einval,
    Enospc,
    Ebadf,
    Ebusy,
    Erofs,
}

impl FsErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Enoent => "ENOENT",
            Self::Eexist => "EEXIST",
            Self::Enotdir => "ENOTDIR",
            Self::Eisdir => "EISDIR",
            Self::Enotempty => "ENOTEMPTY",
            Self::Eacces => "EACCES",
            Self::Einval => "EINVAL",
            Self::Enospc => "ENOSPC",
            Self::Ebadf => "EBADF",
            Self::Ebusy => "EBUSY",
            Self::Erofs => "EROFS",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FsError {
    pub code: FsErrorCode,
    pub message: String,
}

impl FsError {
    pub fn new(code: FsErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for FsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for FsError {}
