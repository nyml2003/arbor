use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileKind {
    File,
    Directory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileMode(pub u16);

impl FileMode {
    pub const FILE_DEFAULT: Self = Self(0o644);
    pub const DIR_DEFAULT: Self = Self(0o755);

    pub fn can_read(self) -> bool {
        self.0 & 0o444 != 0
    }

    pub fn can_write(self) -> bool {
        self.0 & 0o222 != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenFlags {
    pub read: bool,
    pub write: bool,
    pub create: bool,
    pub truncate: bool,
    pub append: bool,
}

impl OpenFlags {
    pub const READ: Self = Self {
        read: true,
        write: false,
        create: false,
        truncate: false,
        append: false,
    };

    pub const WRITE_CREATE_TRUNCATE: Self = Self {
        read: false,
        write: true,
        create: true,
        truncate: true,
        append: false,
    };

    pub const READ_WRITE_CREATE: Self = Self {
        read: true,
        write: true,
        create: true,
        truncate: false,
        append: false,
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileStat {
    pub inode: u64,
    pub kind: FileKind,
    pub size: u64,
    pub mode: u16,
    pub created_at: u64,
    pub modified_at: u64,
    pub accessed_at: u64,
    pub block_count: usize,
    pub link_count: u32,
}
