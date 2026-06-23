use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::data::{BlockDebug, DataRegion};
use crate::error::{FsError, FsErrorCode, FsResult};
use crate::index::{DirEntry, IndexRegion, InodeDebug, InodeId};
use crate::protocol::StatFs;
use crate::types::{FileKind, FileMode, FileStat, OpenFlags};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileSystemConfig {
    pub block_size: usize,
    pub data_capacity: usize,
    pub inode_capacity: usize,
}

impl Default for FileSystemConfig {
    fn default() -> Self {
        Self {
            block_size: 4096,
            data_capacity: 64 * 1024 * 1024,
            inode_capacity: 16_384,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileSystem {
    index: IndexRegion,
    data: DataRegion,
    open_files: BTreeMap<u64, OpenFile>,
    next_fd: u64,
}

#[derive(Debug, Clone, Copy)]
struct OpenFile {
    inode: InodeId,
    offset: u64,
    flags: OpenFlags,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenFileDebug {
    pub fd: u64,
    pub inode: InodeId,
    pub offset: u64,
    pub read: bool,
    pub write: bool,
    pub append: bool,
}

impl FileSystem {
    pub fn new(config: FileSystemConfig) -> FsResult<Self> {
        Ok(Self {
            index: IndexRegion::new(config.inode_capacity),
            data: DataRegion::new(config.block_size, config.data_capacity)?,
            open_files: BTreeMap::new(),
            next_fd: 3,
        })
    }

    pub fn mkdir(&mut self, path: &str, mode: Option<FileMode>) -> FsResult<()> {
        self.index
            .mkdir(path, mode.unwrap_or(FileMode::DIR_DEFAULT))?;
        Ok(())
    }

    pub fn open(&mut self, path: &str, flags: OpenFlags, mode: Option<FileMode>) -> FsResult<u64> {
        if !flags.read && !flags.write {
            return Err(FsError::new(
                FsErrorCode::Einval,
                "open requires read or write access",
            ));
        }

        let inode = match self.index.resolve_path(path) {
            Ok(inode) => inode,
            Err(error) if error.code == FsErrorCode::Enoent && flags.create => self
                .index
                .create_file(path, mode.unwrap_or(FileMode::FILE_DEFAULT))?,
            Err(error) => return Err(error),
        };

        if self.index.kind(inode)? != FileKind::File {
            return Err(FsError::new(
                FsErrorCode::Eisdir,
                "cannot open a directory as a file",
            ));
        }

        let inode_mode = self.index.mode(inode)?;
        if flags.read && !inode_mode.can_read() {
            return Err(FsError::new(FsErrorCode::Eacces, "file is not readable"));
        }
        if flags.write && !inode_mode.can_write() {
            return Err(FsError::new(FsErrorCode::Eacces, "file is not writable"));
        }
        if flags.truncate {
            if !flags.write {
                return Err(FsError::new(
                    FsErrorCode::Einval,
                    "truncate requires write access",
                ));
            }
            self.truncate_inode(inode, 0)?;
        }

        let fd = self.next_fd;
        self.next_fd += 1;
        let offset = if flags.append {
            self.index.file_size(inode)?
        } else {
            0
        };
        self.open_files.insert(
            fd,
            OpenFile {
                inode,
                offset,
                flags,
            },
        );
        Ok(fd)
    }

    pub fn close(&mut self, fd: u64) -> FsResult<()> {
        self.open_files
            .remove(&fd)
            .map(|_| ())
            .ok_or_else(|| FsError::new(FsErrorCode::Ebadf, "invalid file descriptor"))
    }

    pub fn read(&mut self, fd: u64, len: usize) -> FsResult<Vec<u8>> {
        let open = *self.open_file(fd)?;
        if !open.flags.read {
            return Err(FsError::new(
                FsErrorCode::Ebadf,
                "file descriptor is not readable",
            ));
        }

        let file_size = self.index.file_size(open.inode)?;
        if open.offset >= file_size || len == 0 {
            return Ok(Vec::new());
        }

        let readable = len.min((file_size - open.offset) as usize);
        let mut output = vec![0; readable];
        self.read_from_inode(open.inode, open.offset, &mut output)?;
        self.open_file_mut(fd)?.offset += readable as u64;
        self.index.touch_accessed(open.inode)?;
        Ok(output)
    }

    pub fn write(&mut self, fd: u64, bytes: &[u8]) -> FsResult<usize> {
        let open = *self.open_file(fd)?;
        if !open.flags.write {
            return Err(FsError::new(
                FsErrorCode::Ebadf,
                "file descriptor is not writable",
            ));
        }

        let offset = if open.flags.append {
            self.index.file_size(open.inode)?
        } else {
            open.offset
        };
        let written = self.write_to_inode(open.inode, offset, bytes)?;
        self.open_file_mut(fd)?.offset = offset + written as u64;
        Ok(written)
    }

    pub fn seek(&mut self, fd: u64, offset: i64) -> FsResult<u64> {
        if offset < 0 {
            return Err(FsError::new(
                FsErrorCode::Einval,
                "negative offsets are not supported",
            ));
        }
        let open = self.open_file_mut(fd)?;
        open.offset = offset as u64;
        Ok(open.offset)
    }

    pub fn truncate_path(&mut self, path: &str, size: u64) -> FsResult<()> {
        let inode = self.index.resolve_path(path)?;
        if self.index.kind(inode)? != FileKind::File {
            return Err(FsError::new(
                FsErrorCode::Eisdir,
                "cannot truncate a directory",
            ));
        }
        self.truncate_inode(inode, size)
    }

    pub fn read_file(&mut self, path: &str) -> FsResult<Vec<u8>> {
        let fd = self.open(path, OpenFlags::READ, None)?;
        let size = self.index.file_size(self.open_file(fd)?.inode)? as usize;
        let bytes = self.read(fd, size)?;
        self.close(fd)?;
        Ok(bytes)
    }

    pub fn write_file(&mut self, path: &str, bytes: &[u8]) -> FsResult<usize> {
        let fd = self.open(path, OpenFlags::WRITE_CREATE_TRUNCATE, None)?;
        let written = self.write(fd, bytes)?;
        self.close(fd)?;
        Ok(written)
    }

    pub fn readdir(&mut self, path: &str) -> FsResult<Vec<DirEntry>> {
        self.index.readdir(path)
    }

    pub fn stat(&self, path: &str) -> FsResult<FileStat> {
        self.index.stat_path(path)
    }

    pub fn rename(&mut self, from: &str, to: &str) -> FsResult<()> {
        let source = self.index.resolve_path(from)?;
        let target = self.index.resolve_path(to).ok();
        if target.is_some_and(|target| target != source && self.is_open_inode(target)) {
            return Err(FsError::new(
                FsErrorCode::Ebusy,
                "cannot replace an open file in this v1 model",
            ));
        }

        if let Some(removed) = self.index.rename(from, to)? {
            for block in removed.blocks {
                self.data.free(block)?;
            }
        }
        Ok(())
    }

    pub fn unlink(&mut self, path: &str) -> FsResult<()> {
        let inode = self.index.resolve_path(path)?;
        if self.is_open_inode(inode) {
            return Err(FsError::new(
                FsErrorCode::Ebusy,
                "cannot unlink an open file in this v1 model",
            ));
        }

        let removed = self.index.unlink(path)?;
        for block in removed.blocks {
            self.data.free(block)?;
        }
        Ok(())
    }

    pub fn rmdir(&mut self, path: &str) -> FsResult<()> {
        self.index.rmdir(path)
    }

    pub fn debug_inodes(&self) -> Vec<InodeDebug> {
        self.index.debug_inodes()
    }

    pub fn debug_blocks(&self) -> Vec<BlockDebug> {
        self.data.debug_blocks()
    }

    pub fn debug_free_blocks(&self) -> Vec<u32> {
        self.data.debug_free()
    }

    pub fn debug_open_files(&self) -> Vec<OpenFileDebug> {
        self.open_files
            .iter()
            .map(|(fd, open)| OpenFileDebug {
                fd: *fd,
                inode: open.inode,
                offset: open.offset,
                read: open.flags.read,
                write: open.flags.write,
                append: open.flags.append,
            })
            .collect()
    }

    pub fn statfs(&self) -> StatFs {
        StatFs {
            block_size: self.data.block_size(),
            total_blocks: self.data.total_blocks(),
            used_blocks: self.data.used_blocks(),
            free_blocks: self.data.free_blocks(),
            inode_count: self.index.inode_count(),
            inode_capacity: self.index.inode_capacity(),
        }
    }

    fn read_from_inode(&self, inode: InodeId, offset: u64, output: &mut [u8]) -> FsResult<usize> {
        let block_size = self.data.block_size() as u64;
        let blocks = self.index.file_blocks(inode)?;
        let mut remaining = output.len();
        let mut total = 0;
        let mut cursor = offset;

        while remaining > 0 {
            let block_index = (cursor / block_size) as usize;
            if block_index >= blocks.len() {
                break;
            }
            let block_offset = (cursor % block_size) as usize;
            let read = self
                .data
                .read(blocks[block_index], block_offset, &mut output[total..])?;
            if read == 0 {
                break;
            }
            remaining -= read;
            total += read;
            cursor += read as u64;
        }
        Ok(total)
    }

    fn write_to_inode(&mut self, inode: InodeId, offset: u64, bytes: &[u8]) -> FsResult<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }

        let end = offset
            .checked_add(bytes.len() as u64)
            .ok_or_else(|| FsError::new(FsErrorCode::Einval, "write offset overflow"))?;
        self.ensure_blocks(inode, end)?;

        let block_size = self.data.block_size() as u64;
        let blocks = self.index.file_blocks(inode)?.to_vec();
        let mut remaining = bytes.len();
        let mut total = 0;
        let mut cursor = offset;

        while remaining > 0 {
            let block_index = (cursor / block_size) as usize;
            let block_offset = (cursor % block_size) as usize;
            let written = self
                .data
                .write(blocks[block_index], block_offset, &bytes[total..])?;
            if written == 0 {
                break;
            }
            remaining -= written;
            total += written;
            cursor += written as u64;
        }

        let new_size = self.index.file_size(inode)?.max(offset + total as u64);
        self.index.set_file_size(inode, new_size)?;
        Ok(total)
    }

    fn truncate_inode(&mut self, inode: InodeId, size: u64) -> FsResult<()> {
        self.ensure_blocks(inode, size)?;
        let block_size = self.data.block_size() as u64;
        let keep_blocks = blocks_for_size(size, block_size);
        let blocks = self.index.file_blocks_mut(inode)?;
        let removed = blocks.split_off(keep_blocks);
        for block in removed {
            self.data.free(block)?;
        }

        if size > 0 && size % block_size != 0 {
            let last_block_index = keep_blocks - 1;
            let zero_start = (size % block_size) as usize;
            let last_block = self.index.file_blocks(inode)?[last_block_index];
            self.data
                .zero(last_block, zero_start, self.data.block_size() - zero_start)?;
        }

        self.index.set_file_size(inode, size)
    }

    fn ensure_blocks(&mut self, inode: InodeId, target_size: u64) -> FsResult<()> {
        let block_size = self.data.block_size() as u64;
        let needed = blocks_for_size(target_size, block_size);
        let current = self.index.file_blocks(inode)?.len();
        if needed <= current {
            return Ok(());
        }

        let mut allocated = Vec::new();
        for _ in current..needed {
            match self.data.allocate() {
                Ok(block) => allocated.push(block),
                Err(error) => {
                    for block in allocated {
                        self.data.free(block)?;
                    }
                    return Err(error);
                }
            }
        }
        self.index.file_blocks_mut(inode)?.extend(allocated);
        Ok(())
    }

    fn open_file(&self, fd: u64) -> FsResult<&OpenFile> {
        self.open_files
            .get(&fd)
            .ok_or_else(|| FsError::new(FsErrorCode::Ebadf, "invalid file descriptor"))
    }

    fn open_file_mut(&mut self, fd: u64) -> FsResult<&mut OpenFile> {
        self.open_files
            .get_mut(&fd)
            .ok_or_else(|| FsError::new(FsErrorCode::Ebadf, "invalid file descriptor"))
    }

    fn is_open_inode(&self, inode: InodeId) -> bool {
        self.open_files.values().any(|open| open.inode == inode)
    }
}

fn blocks_for_size(size: u64, block_size: u64) -> usize {
    if size == 0 {
        0
    } else {
        size.div_ceil(block_size) as usize
    }
}

#[cfg(test)]
mod tests {
    use crate::error::FsErrorCode;
    use crate::fs::{FileSystem, FileSystemConfig};
    use crate::types::OpenFlags;

    #[test]
    fn write_splits_bytes_across_fixed_blocks() {
        let mut fs = tiny_fs();
        fs.write_file("/file", b"abcdef").unwrap();

        let stat = fs.stat("/file").unwrap();
        assert_eq!(stat.size, 6);
        assert_eq!(stat.block_count, 2);
        assert_eq!(fs.read_file("/file").unwrap(), b"abcdef");
        assert_eq!(fs.statfs().used_blocks, 2);
    }

    #[test]
    fn truncate_reclaims_blocks() {
        let mut fs = tiny_fs();
        fs.write_file("/file", b"abcdef").unwrap();
        fs.truncate_path("/file", 2).unwrap();

        assert_eq!(fs.read_file("/file").unwrap(), b"ab");
        assert_eq!(fs.stat("/file").unwrap().block_count, 1);
        assert_eq!(fs.statfs().used_blocks, 1);
    }

    #[test]
    fn metadata_operations_do_not_allocate_blocks() {
        let mut fs = tiny_fs();
        fs.mkdir("/dir", None).unwrap();
        fs.mkdir("/dir/nested", None).unwrap();
        fs.rename("/dir/nested", "/renamed").unwrap();

        assert_eq!(fs.statfs().used_blocks, 0);
        assert!(fs.debug_inodes().iter().all(|inode| {
            inode.kind == crate::types::FileKind::File || inode.blocks.is_empty()
        }));
    }

    #[test]
    fn open_read_write_seek_tracks_offset() {
        let mut fs = tiny_fs();
        let fd = fs
            .open("/file", OpenFlags::READ_WRITE_CREATE, None)
            .unwrap();
        fs.write(fd, b"abcd").unwrap();
        fs.seek(fd, 1).unwrap();
        fs.write(fd, b"Z").unwrap();
        fs.seek(fd, 0).unwrap();
        assert_eq!(fs.read(fd, 4).unwrap(), b"aZcd");
        fs.close(fd).unwrap();
    }

    #[test]
    fn reports_enospc_when_data_region_is_full() {
        let mut fs = tiny_fs();
        let err = fs
            .write_file("/file", b"012345678901234567890")
            .unwrap_err();
        assert_eq!(err.code, FsErrorCode::Enospc);
        assert_eq!(fs.statfs().used_blocks, 0);
    }

    #[test]
    fn rename_same_path_is_noop() {
        let mut fs = tiny_fs();
        fs.write_file("/file", b"abc").unwrap();
        fs.rename("/file", "/file").unwrap();

        assert_eq!(fs.read_file("/file").unwrap(), b"abc");
        assert_eq!(fs.statfs().used_blocks, 1);
    }

    #[test]
    fn refuses_to_unlink_open_file() {
        let mut fs = tiny_fs();
        let fd = fs
            .open("/file", OpenFlags::READ_WRITE_CREATE, None)
            .unwrap();
        let err = fs.unlink("/file").unwrap_err();

        assert_eq!(err.code, FsErrorCode::Ebusy);
        fs.close(fd).unwrap();
        fs.unlink("/file").unwrap();
    }

    #[test]
    fn refuses_to_replace_open_file() {
        let mut fs = tiny_fs();
        fs.write_file("/from", b"new").unwrap();
        fs.write_file("/to", b"old").unwrap();
        let fd = fs.open("/to", OpenFlags::READ, None).unwrap();
        let err = fs.rename("/from", "/to").unwrap_err();

        assert_eq!(err.code, FsErrorCode::Ebusy);
        assert_eq!(fs.read_file("/from").unwrap(), b"new");
        fs.close(fd).unwrap();
    }

    fn tiny_fs() -> FileSystem {
        FileSystem::new(FileSystemConfig {
            block_size: 4,
            data_capacity: 16,
            inode_capacity: 32,
        })
        .unwrap()
    }
}
