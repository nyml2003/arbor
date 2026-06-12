use serde::{Deserialize, Serialize};
#[cfg(windows)]
use windows::Win32::Graphics::Gdi::HMONITOR;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureSettings {
  pub hotkey: String,
  pub notification_enabled: bool,
  pub cache_limit: u32,
  pub launch_on_login: bool,
}

impl Default for CaptureSettings {
  fn default() -> Self {
    Self {
      hotkey: "CommandOrControl+Shift+4".to_string(),
      notification_enabled: true,
      cache_limit: 50,
      launch_on_login: false,
    }
  }
}

#[derive(Debug, Clone, Serialize)]
pub struct CaptureResult {
  pub file_path: String,
  pub width: u32,
  pub height: u32,
  pub copied: bool,
  pub notified: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToastPayload {
  pub file_path: String,
  pub width: u32,
  pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AreaSelection {
  pub x: f64,
  pub y: f64,
  pub width: f64,
  pub height: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaptureError {
  pub code: &'static str,
  pub message: String,
}

impl CaptureError {
  pub fn new(code: &'static str, message: impl Into<String>) -> Self {
    Self {
      code,
      message: message.into(),
    }
  }
}

#[derive(Debug, Clone)]
pub struct NormalizedAreaSelection {
  pub x: u32,
  pub y: u32,
  pub width: u32,
  pub height: u32,
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy)]
pub struct ResolvedCaptureArea {
  pub monitor_handle: HMONITOR,
  pub relative_x: u32,
  pub relative_y: u32,
  pub width: u32,
  pub height: u32,
}

impl AreaSelection {
  pub fn normalize(&self) -> Result<NormalizedAreaSelection, CaptureError> {
    let width = self.width.round().max(1.0);
    let height = self.height.round().max(1.0);

    if !width.is_finite() || !height.is_finite() {
      return Err(CaptureError::new(
        "invalid_selection",
        "Selection width or height is not finite.",
      ));
    }

    let x = self.x.floor();
    let y = self.y.floor();

    if !x.is_finite() || !y.is_finite() {
      return Err(CaptureError::new(
        "invalid_selection",
        "Selection position is not finite.",
      ));
    }

    let x = if x < 0.0 { 0 } else { x as u32 };
    let y = if y < 0.0 { 0 } else { y as u32 };

    Ok(NormalizedAreaSelection {
      x,
      y,
      width: width as u32,
      height: height as u32,
    })
  }
}
