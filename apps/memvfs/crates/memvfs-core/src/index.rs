use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::data::BlockId;
use crate::error::{FsError, FsErrorCode, FsResult};
use crate::types::{FileKind, FileMode, FileStat};

pub type InodeId = u64;

const ROOT_INODE: InodeId = 1;

#[derive(Debug, Clone)]
pub struct IndexRegion {
    inodes: BTreeMap<InodeId, Inode>,
    next_inode: InodeId,
    inode_capacity: usize,
    clock: u64,
}

#[derive(Debug, Clone)]
struct Inode {
    id: InodeId,
    kind: FileKind,
    mode: FileMode,
    size: u64,
    created_at: u64,
    modified_at: u64,
    accessed_at: u64,
    link_count: u32,
    data_blocks: Vec<BlockId>,
    entries: BTreeMap<String, InodeId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirEntry {
    pub name: String,
    pub inode: InodeId,
    pub kind: FileKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InodeDebug {
    pub inode: InodeId,
    pub kind: FileKind,
    pub size: u64,
    pub mode: u16,
    pub created_at: u64,
    pub modified_at: u64,
    pub accessed_at: u64,
    pub link_count: u32,
    pub blocks: Vec<BlockId>,
    pub entries: Vec<DirEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovedInode {
    pub inode: InodeId,
    pub blocks: Vec<BlockId>,
}

impl IndexRegion {
    pub fn new(inode_capacity: usize) -> Self {
        let mut inodes = BTreeMap::new();
        let root = Inode {
            id: ROOT_INODE,
            kind: FileKind::Directory,
            mode: FileMode::DIR_DEFAULT,
            size: 0,
            created_at: 1,
            modified_at: 1,
            accessed_at: 1,
            link_count: 1,
            data_blocks: Vec::new(),
            entries: BTreeMap::new(),
        };
        inodes.insert(ROOT_INODE, root);
        Self {
            inodes,
            next_inode: ROOT_INODE + 1,
            inode_capacity: inode_capacity.max(1),
            clock: 1,
        }
    }

    pub fn inode_count(&self) -> usize {
        self.inodes.len()
    }

    pub fn inode_capacity(&self) -> usize {
        self.inode_capacity
    }

    pub fn resolve_path(&self, path: &str) -> FsResult<InodeId> {
        let parts = normalize_path(path)?;
        let mut current = ROOT_INODE;
        for part in parts {
            let inode = self.inode(current)?;
            if inode.kind != FileKind::Directory {
                return Err(FsError::new(
                    FsErrorCode::Enotdir,
                    "path component is not a directory",
                ));
            }
            current = *inode
                .entries
                .get(part)
                .ok_or_else(|| FsError::new(FsErrorCode::Enoent, "path does not exist"))?;
        }
        Ok(current)
    }

    pub fn mkdir(&mut self, path: &str, mode: FileMode) -> FsResult<InodeId> {
        self.create_node(path, FileKind::Directory, mode)
    }

    pub fn create_file(&mut self, path: &str, mode: FileMode) -> FsResult<InodeId> {
        self.create_node(path, FileKind::File, mode)
    }

    pub fn stat_path(&self, path: &str) -> FsResult<FileStat> {
        let inode = self.resolve_path(path)?;
        self.stat_inode(inode)
    }

    pub fn stat_inode(&self, id: InodeId) -> FsResult<FileStat> {
        let inode = self.inode(id)?;
        Ok(FileStat {
            inode: inode.id,
            kind: inode.kind,
            size: inode.size,
            mode: inode.mode.0,
            created_at: inode.created_at,
            modified_at: inode.modified_at,
            accessed_at: inode.accessed_at,
            block_count: inode.data_blocks.len(),
            link_count: inode.link_count,
        })
    }

    pub fn readdir(&mut self, path: &str) -> FsResult<Vec<DirEntry>> {
        let id = self.resolve_path(path)?;
        self.touch_accessed(id)?;
        let inode = self.inode(id)?;
        if inode.kind != FileKind::Directory {
            return Err(FsError::new(
                FsErrorCode::Enotdir,
                "path is not a directory",
            ));
        }
        let mut entries = Vec::with_capacity(inode.entries.len());
        for (name, child_id) in &inode.entries {
            let child = self.inode(*child_id)?;
            entries.push(DirEntry {
                name: name.clone(),
                inode: child.id,
                kind: child.kind,
            });
        }
        Ok(entries)
    }

    pub fn unlink(&mut self, path: &str) -> FsResult<RemovedInode> {
        let (parent, name) = self.resolve_parent(path)?;
        let child_id = self.child_id(parent, &name)?;
        let child = self.inode(child_id)?;
        if child.kind == FileKind::Directory {
            return Err(FsError::new(
                FsErrorCode::Eisdir,
                "cannot unlink a directory",
            ));
        }
        self.remove_child(parent, &name, child_id)
    }

    pub fn rmdir(&mut self, path: &str) -> FsResult<()> {
        let (parent, name) = self.resolve_parent(path)?;
        let child_id = self.child_id(parent, &name)?;
        let child = self.inode(child_id)?;
        if child.kind != FileKind::Directory {
            return Err(FsError::new(
                FsErrorCode::Enotdir,
                "path is not a directory",
            ));
        }
        if !child.entries.is_empty() {
            return Err(FsError::new(
                FsErrorCode::Enotempty,
                "directory is not empty",
            ));
        }
        self.remove_child(parent, &name, child_id)?;
        Ok(())
    }

    pub fn rename(&mut self, from: &str, to: &str) -> FsResult<Option<RemovedInode>> {
        let from_id = self.resolve_path(from)?;
        if from_id == ROOT_INODE {
            return Err(FsError::new(FsErrorCode::Einval, "cannot rename root"));
        }
        let (from_parent, from_name) = self.resolve_parent(from)?;
        let (to_parent, to_name) = self.resolve_parent(to)?;
        if from_parent == from_id || to_parent == from_id {
            return Err(FsError::new(
                FsErrorCode::Einval,
                "cannot move a directory into itself",
            ));
        }
        if self.inode(from_id)?.kind == FileKind::Directory
            && self.is_descendant(to_parent, from_id)?
        {
            return Err(FsError::new(
                FsErrorCode::Einval,
                "cannot move a directory into its descendant",
            ));
        }

        let replacement = self.inode(to_parent)?.entries.get(&to_name).copied();
        if let Some(replacement_id) = replacement {
            if replacement_id == from_id {
                return Ok(None);
            }
            let replacement_inode = self.inode(replacement_id)?;
            let source_kind = self.inode(from_id)?.kind;
            match (source_kind, replacement_inode.kind) {
                (FileKind::File, FileKind::Directory) => {
                    return Err(FsError::new(FsErrorCode::Eisdir, "target is a directory"));
                }
                (FileKind::Directory, FileKind::File) => {
                    return Err(FsError::new(
                        FsErrorCode::Enotdir,
                        "target is not a directory",
                    ));
                }
                (FileKind::Directory, FileKind::Directory)
                    if !replacement_inode.entries.is_empty() =>
                {
                    return Err(FsError::new(
                        FsErrorCode::Enotempty,
                        "target directory is not empty",
                    ));
                }
                _ => {}
            }
            let removed = self.remove_child(to_parent, &to_name, replacement_id)?;
            self.inode_mut(from_parent)?.entries.remove(&from_name);
            self.inode_mut(to_parent)?.entries.insert(to_name, from_id);
            self.touch_modified(from_parent)?;
            self.touch_modified(to_parent)?;
            self.touch_modified(from_id)?;
            return Ok(Some(removed));
        }

        self.inode_mut(from_parent)?.entries.remove(&from_name);
        self.inode_mut(to_parent)?.entries.insert(to_name, from_id);
        self.touch_modified(from_parent)?;
        self.touch_modified(to_parent)?;
        self.touch_modified(from_id)?;
        Ok(None)
    }

    pub fn kind(&self, id: InodeId) -> FsResult<FileKind> {
        Ok(self.inode(id)?.kind)
    }

    pub fn mode(&self, id: InodeId) -> FsResult<FileMode> {
        Ok(self.inode(id)?.mode)
    }

    pub fn file_size(&self, id: InodeId) -> FsResult<u64> {
        Ok(self.inode(id)?.size)
    }

    pub fn file_blocks(&self, id: InodeId) -> FsResult<&[BlockId]> {
        let inode = self.inode(id)?;
        if inode.kind != FileKind::File {
            return Err(FsError::new(FsErrorCode::Eisdir, "inode is not a file"));
        }
        Ok(&inode.data_blocks)
    }

    pub fn file_blocks_mut(&mut self, id: InodeId) -> FsResult<&mut Vec<BlockId>> {
        let inode = self.inode_mut(id)?;
        if inode.kind != FileKind::File {
            return Err(FsError::new(FsErrorCode::Eisdir, "inode is not a file"));
        }
        Ok(&mut inode.data_blocks)
    }

    pub fn set_file_size(&mut self, id: InodeId, size: u64) -> FsResult<()> {
        let inode = self.inode_mut(id)?;
        if inode.kind != FileKind::File {
            return Err(FsError::new(FsErrorCode::Eisdir, "inode is not a file"));
        }
        inode.size = size;
        self.touch_modified(id)
    }

    pub fn touch_accessed(&mut self, id: InodeId) -> FsResult<()> {
        let time = self.tick();
        self.inode_mut(id)?.accessed_at = time;
        Ok(())
    }

    pub fn touch_modified(&mut self, id: InodeId) -> FsResult<()> {
        let time = self.tick();
        let inode = self.inode_mut(id)?;
        inode.modified_at = time;
        inode.accessed_at = time;
        Ok(())
    }

    pub fn debug_inodes(&self) -> Vec<InodeDebug> {
        self.inodes
            .values()
            .map(|inode| InodeDebug {
                inode: inode.id,
                kind: inode.kind,
                size: inode.size,
                mode: inode.mode.0,
                created_at: inode.created_at,
                modified_at: inode.modified_at,
                accessed_at: inode.accessed_at,
                link_count: inode.link_count,
                blocks: inode.data_blocks.clone(),
                entries: inode
                    .entries
                    .iter()
                    .filter_map(|(name, child_id)| {
                        let child = self.inodes.get(child_id)?;
                        Some(DirEntry {
                            name: name.clone(),
                            inode: child.id,
                            kind: child.kind,
                        })
                    })
                    .collect(),
            })
            .collect()
    }

    fn create_node(&mut self, path: &str, kind: FileKind, mode: FileMode) -> FsResult<InodeId> {
        if self.inodes.len() >= self.inode_capacity {
            return Err(FsError::new(FsErrorCode::Enospc, "inode table is full"));
        }
        let (parent, name) = self.resolve_parent(path)?;
        if self.inode(parent)?.entries.contains_key(&name) {
            return Err(FsError::new(FsErrorCode::Eexist, "path already exists"));
        }
        let id = self.next_inode;
        self.next_inode += 1;
        let now = self.tick();
        let inode = Inode {
            id,
            kind,
            mode,
            size: 0,
            created_at: now,
            modified_at: now,
            accessed_at: now,
            link_count: 1,
            data_blocks: Vec::new(),
            entries: BTreeMap::new(),
        };
        self.inodes.insert(id, inode);
        self.inode_mut(parent)?.entries.insert(name, id);
        self.touch_modified(parent)?;
        Ok(id)
    }

    fn resolve_parent(&self, path: &str) -> FsResult<(InodeId, String)> {
        let parts = normalize_path(path)?;
        let (name, parent_parts) = parts
            .split_last()
            .ok_or_else(|| FsError::new(FsErrorCode::Einval, "root has no parent"))?;
        let mut parent = ROOT_INODE;
        for part in parent_parts {
            let inode = self.inode(parent)?;
            if inode.kind != FileKind::Directory {
                return Err(FsError::new(
                    FsErrorCode::Enotdir,
                    "parent component is not a directory",
                ));
            }
            parent = *inode
                .entries
                .get(*part)
                .ok_or_else(|| FsError::new(FsErrorCode::Enoent, "parent path does not exist"))?;
        }
        Ok((parent, (*name).to_string()))
    }

    fn child_id(&self, parent: InodeId, name: &str) -> FsResult<InodeId> {
        self.inode(parent)?
            .entries
            .get(name)
            .copied()
            .ok_or_else(|| FsError::new(FsErrorCode::Enoent, "path does not exist"))
    }

    fn remove_child(
        &mut self,
        parent: InodeId,
        name: &str,
        child_id: InodeId,
    ) -> FsResult<RemovedInode> {
        self.inode_mut(parent)?.entries.remove(name);
        self.touch_modified(parent)?;
        let removed = self
            .inodes
            .remove(&child_id)
            .ok_or_else(|| FsError::new(FsErrorCode::Enoent, "inode does not exist"))?;
        Ok(RemovedInode {
            inode: child_id,
            blocks: removed.data_blocks,
        })
    }

    fn is_descendant(&self, candidate: InodeId, ancestor: InodeId) -> FsResult<bool> {
        if candidate == ancestor {
            return Ok(true);
        }
        let mut stack = vec![ancestor];
        while let Some(current) = stack.pop() {
            let inode = self.inode(current)?;
            for child_id in inode.entries.values() {
                if *child_id == candidate {
                    return Ok(true);
                }
                if self.inode(*child_id)?.kind == FileKind::Directory {
                    stack.push(*child_id);
                }
            }
        }
        Ok(false)
    }

    fn inode(&self, id: InodeId) -> FsResult<&Inode> {
        self.inodes
            .get(&id)
            .ok_or_else(|| FsError::new(FsErrorCode::Enoent, "inode does not exist"))
    }

    fn inode_mut(&mut self, id: InodeId) -> FsResult<&mut Inode> {
        self.inodes
            .get_mut(&id)
            .ok_or_else(|| FsError::new(FsErrorCode::Enoent, "inode does not exist"))
    }

    fn tick(&mut self) -> u64 {
        self.clock += 1;
        self.clock
    }
}

fn normalize_path(path: &str) -> FsResult<Vec<&str>> {
    if !path.starts_with('/') {
        return Err(FsError::new(FsErrorCode::Einval, "path must be absolute"));
    }
    let mut parts = Vec::new();
    for part in path.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return Err(FsError::new(
                FsErrorCode::Einval,
                "parent traversal is not allowed",
            ));
        }
        if part.contains('\\') || part.contains('\0') {
            return Err(FsError::new(FsErrorCode::Einval, "invalid path component"));
        }
        parts.push(part);
    }
    Ok(parts)
}

#[cfg(test)]
mod tests {
    use crate::error::FsErrorCode;
    use crate::types::{FileKind, FileMode};

    use super::IndexRegion;

    #[test]
    fn directory_metadata_never_uses_file_blocks() {
        let mut index = IndexRegion::new(16);
        let dir = index.mkdir("/docs", FileMode::DIR_DEFAULT).unwrap();
        assert_eq!(index.kind(dir).unwrap(), FileKind::Directory);
        assert!(index
            .debug_inodes()
            .iter()
            .all(|inode| { inode.kind == FileKind::File || inode.blocks.is_empty() }));
    }

    #[test]
    fn rejects_parent_traversal() {
        let index = IndexRegion::new(16);
        let err = index.resolve_path("/../x").unwrap_err();
        assert_eq!(err.code, FsErrorCode::Einval);
    }

    #[test]
    fn cannot_remove_non_empty_directory() {
        let mut index = IndexRegion::new(16);
        index.mkdir("/a", FileMode::DIR_DEFAULT).unwrap();
        index
            .create_file("/a/file", FileMode::FILE_DEFAULT)
            .unwrap();
        let err = index.rmdir("/a").unwrap_err();
        assert_eq!(err.code, FsErrorCode::Enotempty);
    }
}
