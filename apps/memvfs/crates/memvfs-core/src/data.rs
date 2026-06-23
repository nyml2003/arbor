use serde::{Deserialize, Serialize};

use crate::error::{FsError, FsErrorCode, FsResult};

pub type BlockId = u32;

#[derive(Debug, Clone)]
struct Block {
    bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct DataRegion {
    block_size: usize,
    blocks: Vec<Block>,
    free: Vec<BlockId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockDebug {
    pub id: BlockId,
    pub used: bool,
    pub non_zero_bytes: usize,
}

impl DataRegion {
    pub fn new(block_size: usize, total_bytes: usize) -> FsResult<Self> {
        if block_size == 0 || total_bytes < block_size {
            return Err(FsError::new(
                FsErrorCode::Einval,
                "block size must be non-zero and capacity must fit at least one block",
            ));
        }

        let block_count = total_bytes / block_size;
        if block_count > BlockId::MAX as usize {
            return Err(FsError::new(
                FsErrorCode::Einval,
                "block count exceeds supported block id range",
            ));
        }

        let blocks = (0..block_count)
            .map(|_| Block {
                bytes: vec![0; block_size],
            })
            .collect::<Vec<_>>();
        let free = (0..block_count as BlockId).rev().collect::<Vec<_>>();

        Ok(Self {
            block_size,
            blocks,
            free,
        })
    }

    pub fn block_size(&self) -> usize {
        self.block_size
    }

    pub fn total_blocks(&self) -> usize {
        self.blocks.len()
    }

    pub fn free_blocks(&self) -> usize {
        self.free.len()
    }

    pub fn used_blocks(&self) -> usize {
        self.total_blocks() - self.free_blocks()
    }

    pub fn allocate(&mut self) -> FsResult<BlockId> {
        let id = self
            .free
            .pop()
            .ok_or_else(|| FsError::new(FsErrorCode::Enospc, "data region is full"))?;
        self.blocks[id as usize].bytes.fill(0);
        Ok(id)
    }

    pub fn free(&mut self, id: BlockId) -> FsResult<()> {
        let block = self.block_mut(id)?;
        block.bytes.fill(0);
        if !self.free.contains(&id) {
            self.free.push(id);
        }
        Ok(())
    }

    pub fn read(&self, id: BlockId, offset: usize, out: &mut [u8]) -> FsResult<usize> {
        if offset >= self.block_size {
            return Ok(0);
        }
        let block = self.block(id)?;
        let len = out.len().min(self.block_size - offset);
        out[..len].copy_from_slice(&block.bytes[offset..offset + len]);
        Ok(len)
    }

    pub fn write(&mut self, id: BlockId, offset: usize, input: &[u8]) -> FsResult<usize> {
        if offset >= self.block_size {
            return Ok(0);
        }
        let block_size = self.block_size;
        let block = self.block_mut(id)?;
        let len = input.len().min(block_size - offset);
        block.bytes[offset..offset + len].copy_from_slice(&input[..len]);
        Ok(len)
    }

    pub fn zero(&mut self, id: BlockId, offset: usize, len: usize) -> FsResult<usize> {
        if offset >= self.block_size {
            return Ok(0);
        }
        let block_size = self.block_size;
        let block = self.block_mut(id)?;
        let len = len.min(block_size - offset);
        block.bytes[offset..offset + len].fill(0);
        Ok(len)
    }

    pub fn debug_blocks(&self) -> Vec<BlockDebug> {
        self.blocks
            .iter()
            .enumerate()
            .map(|(id, block)| {
                let block_id = id as BlockId;
                BlockDebug {
                    id: block_id,
                    used: !self.free.contains(&block_id),
                    non_zero_bytes: block.bytes.iter().filter(|byte| **byte != 0).count(),
                }
            })
            .collect()
    }

    pub fn debug_free(&self) -> Vec<BlockId> {
        let mut free = self.free.clone();
        free.sort_unstable();
        free
    }

    fn block(&self, id: BlockId) -> FsResult<&Block> {
        self.blocks
            .get(id as usize)
            .ok_or_else(|| FsError::new(FsErrorCode::Einval, "invalid block id"))
    }

    fn block_mut(&mut self, id: BlockId) -> FsResult<&mut Block> {
        self.blocks
            .get_mut(id as usize)
            .ok_or_else(|| FsError::new(FsErrorCode::Einval, "invalid block id"))
    }
}

#[cfg(test)]
mod tests {
    use super::DataRegion;

    #[test]
    fn reuses_freed_blocks_after_zeroing() {
        let mut data = DataRegion::new(4, 8).unwrap();
        let first = data.allocate().unwrap();
        data.write(first, 0, b"abcd").unwrap();
        data.free(first).unwrap();

        let next = data.allocate().unwrap();
        assert_eq!(first, next);

        let mut out = [1; 4];
        data.read(next, 0, &mut out).unwrap();
        assert_eq!(out, [0, 0, 0, 0]);
    }
}
