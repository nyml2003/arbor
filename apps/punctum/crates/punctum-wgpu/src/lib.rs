//! Winit input and wgpu runtime integration.

#![forbid(unsafe_code)]

mod input;
mod runtime;

pub use input::{WinitKeyEventSnapshot, normalize_key_event};
pub use runtime::{GpuRuntime, GpuRuntimeError, PresentOutcome};
