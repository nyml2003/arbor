use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager, Runtime};

use crate::domain::{
  CaptureError,
  CaptureResult,
};

pub(super) fn resolve_capture_file_path<R: Runtime>(
  app: &AppHandle<R>,
) -> Result<PathBuf, CaptureError> {
  let cache_dir = app
    .path()
    .app_cache_dir()
    .map_err(|error| CaptureError::new("cache_dir_unavailable", error.to_string()))?;

  let capture_dir = cache_dir.join("captures");
  fs::create_dir_all(&capture_dir)
    .map_err(|error| CaptureError::new("cache_dir_create_failed", error.to_string()))?;

  let timestamp = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map_err(|error| CaptureError::new("clock_error", error.to_string()))?
    .as_millis();

  Ok(capture_dir.join(format!("capture-{timestamp}.png")))
}

pub(super) fn save_image_to_cache<R: Runtime>(
  app: &AppHandle<R>,
  image: image::RgbaImage,
) -> Result<CaptureResult, CaptureError> {
  let width = image.width();
  let height = image.height();
  let file_path = resolve_capture_file_path(app)?;

  image
    .save(&file_path)
    .map_err(|error| CaptureError::new("save_failed", error.to_string()))?;

  Ok(CaptureResult {
    file_path: file_path.to_string_lossy().into_owned(),
    width,
    height,
    copied: false,
    notified: false,
  })
}

pub fn capture_area_to_file<R: Runtime>(
  app: &AppHandle<R>,
  resolved: &crate::domain::ResolvedCaptureArea,
) -> Result<CaptureResult, CaptureError> {
  #[cfg(not(windows))]
  {
    let _ = (app, resolved);
    return Err(CaptureError::new(
      "platform_unsupported",
      "Capture is Windows-only in the current MVP build.",
    ));
  }

  #[cfg(windows)]
  {
    super::windows_hdr_capture::capture_area_hdr_to_file(app, resolved)
  }
}

pub fn capture_active_display_to_file<R: Runtime>(
  app: &AppHandle<R>,
) -> Result<CaptureResult, CaptureError> {
  #[cfg(not(windows))]
  {
    let _ = app;
    return Err(CaptureError::new(
      "platform_unsupported",
      "Capture is Windows-only in the current MVP build.",
    ));
  }

  #[cfg(windows)]
  {
    super::windows_hdr_capture::capture_primary_monitor_hdr_to_file(app)
  }
}
