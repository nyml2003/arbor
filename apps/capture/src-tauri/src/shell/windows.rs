use tauri::{AppHandle, Manager, Runtime};
#[cfg(windows)]
use windows::Win32::{
  Foundation::POINT,
  Graphics::Gdi::{
    GetMonitorInfoW,
    MONITOR_DEFAULTTONEAREST,
    MONITORINFOEXW,
    MonitorFromPoint,
  },
};

use crate::domain::{AreaSelection, CaptureError};
#[cfg(windows)]
use crate::domain::ResolvedCaptureArea;

fn get_window<R: Runtime>(
  app: &AppHandle<R>,
  label: &str,
) -> Result<tauri::WebviewWindow<R>, CaptureError> {
  app
    .get_webview_window(label)
    .ok_or_else(|| CaptureError::new("window_missing", format!("Window `{label}` does not exist.")))
}

pub fn show_window<R: Runtime>(
  app: &AppHandle<R>,
  label: &str,
) -> Result<(), CaptureError> {
  let window = get_window(app, label)?;
  window
    .show()
    .map_err(|error| CaptureError::new("window_show_failed", error.to_string()))?;
  window
    .set_focus()
    .map_err(|error| CaptureError::new("window_focus_failed", error.to_string()))?;
  Ok(())
}

pub fn hide_window<R: Runtime>(
  app: &AppHandle<R>,
  label: &str,
) -> Result<(), CaptureError> {
  let window = get_window(app, label)?;
  window
    .hide()
    .map_err(|error| CaptureError::new("window_hide_failed", error.to_string()))
}

#[cfg(windows)]
pub fn resolve_overlay_capture_area<R: Runtime>(
  app: &AppHandle<R>,
  selection: &AreaSelection,
) -> Result<ResolvedCaptureArea, CaptureError> {
  let normalized = selection.normalize()?;
  let overlay = get_window(app, "overlay")?;
  let overlay_position = overlay
    .outer_position()
    .map_err(|error| CaptureError::new("overlay_position_failed", error.to_string()))?;

  let global_x = overlay_position.x + normalized.x as i32;
  let global_y = overlay_position.y + normalized.y as i32;
  let h_monitor = unsafe {
    MonitorFromPoint(
      POINT {
        x: global_x + 1,
        y: global_y + 1,
      },
      MONITOR_DEFAULTTONEAREST,
    )
  };

  if h_monitor.0.is_null() {
    return Err(CaptureError::new(
      "monitor_not_found",
      "No monitor found for the selected point.",
    ));
  }

  let mut monitor_info = MONITORINFOEXW::default();
  monitor_info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

  unsafe {
    if !GetMonitorInfoW(h_monitor, &mut monitor_info.monitorInfo as *mut _).as_bool() {
      return Err(CaptureError::new(
        "monitor_info_failed",
        "GetMonitorInfoW failed while resolving the capture area.",
      ));
    }
  }

  let monitor_rect = monitor_info.monitorInfo.rcMonitor;
  let relative_x = global_x - monitor_rect.left;
  let relative_y = global_y - monitor_rect.top;
  let monitor_width = (monitor_rect.right - monitor_rect.left).max(0) as u32;
  let monitor_height = (monitor_rect.bottom - monitor_rect.top).max(0) as u32;

  if relative_x < 0
    || relative_y < 0
    || relative_x as u32 + normalized.width > monitor_width
    || relative_y as u32 + normalized.height > monitor_height
  {
    return Err(CaptureError::new(
      "selection_crosses_monitor_boundary",
      "The selection crosses the current monitor boundary.",
    ));
  }

  Ok(ResolvedCaptureArea {
    monitor_handle: h_monitor,
    relative_x: relative_x as u32,
    relative_y: relative_y as u32,
    width: normalized.width,
    height: normalized.height,
  })
}
