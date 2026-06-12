mod capture;

pub use capture::{
  AreaSelection,
  CaptureError,
  CaptureResult,
  CaptureSettings,
  ToastPayload,
};
#[cfg(windows)]
pub use capture::ResolvedCaptureArea;
