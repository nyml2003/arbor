use serde::{Deserialize, Serialize};

/**
`RngState` 把随机数状态显式放进 battle 流程里。

这样同一组输入和同一个 seed 就能复现同一场 battle。
这对 replay、测试和差分调试都很重要。
*/
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RngState {
    state: u64,
}

impl RngState {
    pub fn seeded(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.state >> 32) as u32
    }

    pub fn roll_percent(&mut self) -> u8 {
        ((self.next_u32() % 100) + 1) as u8
    }

    pub fn roll_range_inclusive(&mut self, low: u16, high: u16) -> u16 {
        low + (self.next_u32() % u32::from(high - low + 1)) as u16
    }
}
