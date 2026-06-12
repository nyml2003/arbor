use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::domain::{CaptureError, CaptureSettings};

use super::state::AppState;
use super::windows::show_window;

pub fn register_global_hotkey<R: Runtime>(
  app: &AppHandle<R>,
  shortcut: &str,
) -> Result<(), CaptureError> {
  if app.global_shortcut().is_registered(shortcut) {
    app.global_shortcut()
      .unregister(shortcut)
      .map_err(|error| CaptureError::new("hotkey_unregister_failed", error.to_string()))?;
  }

  app.global_shortcut()
    .on_shortcut(shortcut, |app, _shortcut, event| {
      if event.state != ShortcutState::Pressed {
        return;
      }

      if let Err(error) = show_window(app, "overlay") {
        eprintln!("failed to show overlay from hotkey: {}", error.message);
      }
    })
    .map_err(|error| CaptureError::new("hotkey_register_failed", error.to_string()))
}

pub fn register_hotkey_from_state<R: Runtime>(
  app: &AppHandle<R>,
) -> Result<(), CaptureError> {
  let state = app.state::<AppState>();
  let settings = state
    .settings
    .lock()
    .map_err(|_| CaptureError::new("state_poisoned", "Settings state is poisoned."))?
    .clone();

  register_global_hotkey(app, &settings.hotkey)
}

pub fn apply_updated_hotkey<R: Runtime>(
  app: &AppHandle<R>,
  settings: &CaptureSettings,
) -> Result<(), CaptureError> {
  register_global_hotkey(app, &settings.hotkey)
}
