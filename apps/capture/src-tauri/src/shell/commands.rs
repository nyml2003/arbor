use tauri::{AppHandle, Runtime, State};
use std::thread::sleep;
use std::time::Duration;

use crate::domain::{
  AreaSelection,
  CaptureError,
  CaptureResult,
  CaptureSettings,
  ToastPayload,
};
use crate::services::{
  capture_active_display_to_file,
  capture_area_to_file,
  copy_rgba_to_clipboard,
  open_path_with_default_viewer,
};

use image::ImageReader;
use super::hotkeys::apply_updated_hotkey;
use super::state::AppState;
use super::toast::show_capture_toast;
use super::windows::{hide_window, resolve_overlay_capture_area, show_window};

type CommandResult<T> = Result<T, CaptureError>;

fn not_ready(message: &str) -> CaptureError {
  CaptureError::new("not_ready", message)
}

fn read_settings(
  state: &State<'_, AppState>,
) -> CommandResult<CaptureSettings> {
  state
    .settings
    .lock()
    .map(|settings| settings.clone())
    .map_err(|_| CaptureError::new("state_poisoned", "Settings state is poisoned."))
}

fn copy_capture_file_to_clipboard(
  file_path: &str,
) -> CommandResult<()> {
  let image = ImageReader::open(file_path)
    .map_err(|error| CaptureError::new("capture_open_failed", error.to_string()))?
    .decode()
    .map_err(|error| CaptureError::new("capture_decode_failed", error.to_string()))?
    .to_rgba8();

  copy_rgba_to_clipboard(image.width(), image.height(), image.as_raw())
}

fn store_capture_result(
  state: &State<'_, AppState>,
  capture_result: &CaptureResult,
) -> CommandResult<()> {
  {
    let mut last_capture_path = state
      .last_capture_path
      .lock()
      .map_err(|_| CaptureError::new("state_poisoned", "Last capture path state is poisoned."))?;

    *last_capture_path = Some(capture_result.file_path.clone());
  }

  {
    let mut last_capture_result = state
      .last_capture_result
      .lock()
      .map_err(|_| CaptureError::new("state_poisoned", "Last capture result state is poisoned."))?;

    *last_capture_result = Some(capture_result.clone());
  }

  Ok(())
}

fn finalize_capture<R: Runtime>(
  app: &AppHandle<R>,
  state: &State<'_, AppState>,
  mut capture_result: CaptureResult,
) -> CommandResult<CaptureResult> {
  copy_capture_file_to_clipboard(&capture_result.file_path)?;
  capture_result.copied = true;

  show_capture_toast(
    app,
    ToastPayload {
      file_path: capture_result.file_path.clone(),
      width: capture_result.width,
      height: capture_result.height,
    },
  )?;
  capture_result.notified = true;

  store_capture_result(state, &capture_result)?;
  Ok(capture_result)
}

#[tauri::command]
pub async fn begin_area_capture<R: Runtime>(
  app: AppHandle<R>,
) -> CommandResult<()> {
  show_window(&app, "overlay")
}

#[tauri::command]
pub async fn capture_active_display<R: Runtime>(
  app: AppHandle<R>,
  state: State<'_, AppState>,
) -> CommandResult<CaptureResult> {
  let capture_result = capture_active_display_to_file(&app)?;
  finalize_capture(&app, &state, capture_result)
}

#[tauri::command]
pub async fn cancel_capture<R: Runtime>(
  app: AppHandle<R>,
) -> CommandResult<()> {
  hide_window(&app, "overlay")
}

#[tauri::command]
pub async fn get_settings(
  state: State<'_, AppState>,
) -> CommandResult<CaptureSettings> {
  read_settings(&state)
}

#[tauri::command]
pub async fn update_settings<R: Runtime>(
  app: AppHandle<R>,
  settings: CaptureSettings,
  state: State<'_, AppState>,
) -> CommandResult<CaptureSettings> {
  let mut current = state
    .settings
    .lock()
    .map_err(|_| CaptureError::new("state_poisoned", "Settings state is poisoned."))?;

  *current = settings.clone();
  drop(current);

  apply_updated_hotkey(&app, &settings)?;
  Ok(settings)
}

#[tauri::command]
pub async fn open_last_capture(
  state: State<'_, AppState>,
) -> CommandResult<()> {
  let last_capture = state
    .last_capture_path
    .lock()
    .map_err(|_| CaptureError::new("state_poisoned", "Last capture state is poisoned."))?;

  if last_capture.is_none() {
    return Err(not_ready("There is no captured file yet."));
  }

  open_path_with_default_viewer(last_capture.as_deref().unwrap())
}

#[tauri::command]
pub async fn submit_area_selection<R: Runtime>(
  app: AppHandle<R>,
  selection: AreaSelection,
  state: State<'_, AppState>,
) -> CommandResult<CaptureResult> {
  {
    let mut last_selection = state
      .last_selection
      .lock()
      .map_err(|_| CaptureError::new("state_poisoned", "Selection state is poisoned."))?;

    *last_selection = Some(selection.clone());
  }

  let resolved = resolve_overlay_capture_area(&app, &selection)?;
  hide_window(&app, "overlay")?;
  sleep(Duration::from_millis(80));

  let capture_result = capture_area_to_file(&app, &resolved)?;
  let capture_result = finalize_capture(&app, &state, capture_result)?;
  Ok(capture_result)
}

#[tauri::command]
pub async fn get_last_selection(
  state: State<'_, AppState>,
) -> CommandResult<Option<AreaSelection>> {
  state
    .last_selection
    .lock()
    .map(|selection| selection.clone())
    .map_err(|_| CaptureError::new("state_poisoned", "Selection state is poisoned."))
}

#[tauri::command]
pub async fn get_last_capture_result(
  state: State<'_, AppState>,
) -> CommandResult<Option<CaptureResult>> {
  state
    .last_capture_result
    .lock()
    .map(|result| result.clone())
    .map_err(|_| CaptureError::new("state_poisoned", "Capture result state is poisoned."))
}

#[tauri::command]
pub async fn open_capture_path(
  path: String,
) -> CommandResult<()> {
  open_path_with_default_viewer(&path)
}

#[tauri::command]
pub async fn hide_toast<R: Runtime>(
  app: AppHandle<R>,
) -> CommandResult<()> {
  hide_window(&app, "toast")
}
