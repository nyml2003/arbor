use tauri::{
  AppHandle,
  Emitter,
  Manager,
  PhysicalPosition,
  Runtime,
};

use crate::domain::{CaptureError, ToastPayload};

pub fn show_capture_toast<R: Runtime>(
  app: &AppHandle<R>,
  payload: ToastPayload,
) -> Result<(), CaptureError> {
  let toast = app
    .get_webview_window("toast")
    .ok_or_else(|| CaptureError::new("toast_window_missing", "Toast window does not exist."))?;

  if let Some(monitor) = app
    .primary_monitor()
    .map_err(|error| CaptureError::new("monitor_query_failed", error.to_string()))?
  {
    let work_area = monitor.work_area();
    let x = work_area.position.x + work_area.size.width as i32 - 360 - 18;
    let y = work_area.position.y + work_area.size.height as i32 - 108 - 18;

    toast
      .set_position(PhysicalPosition::new(x, y))
      .map_err(|error| CaptureError::new("toast_position_failed", error.to_string()))?;
  }

  toast
    .show()
    .map_err(|error| CaptureError::new("toast_show_failed", error.to_string()))?;
  let _ = toast.set_focus();
  toast
    .emit("capture-toast", payload)
    .map_err(|error| CaptureError::new("toast_emit_failed", error.to_string()))?;

  Ok(())
}
