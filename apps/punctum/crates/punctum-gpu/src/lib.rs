//! Logical GPU submission planning and a thin `wgpu` presenter.

#![forbid(unsafe_code)]

mod input;
mod model;
mod plan;
mod runtime;

pub use input::{WinitKeyEventSnapshot, normalize_key_event};
pub use model::{
    GpuAtlas, GpuAtlasError, GpuCell, GpuClip, GpuResource, PixelOffset, PixelRect, PixelSize,
    ResourceId, Rgba8, Viewport, ViewportError,
};
pub use plan::{
    GpuPlanError, INSTANCE_STRIDE, InstanceData, InstanceUpload, SubmissionMode, SubmissionPlan,
    plan_patch, plan_surface,
};
pub use runtime::{GpuRuntime, GpuRuntimeError, PresentOutcome};
