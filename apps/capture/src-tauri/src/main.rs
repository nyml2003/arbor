mod domain;
mod services;
mod shell;

use shell::commands::{
  begin_area_capture,
  cancel_capture,
  capture_active_display,
  get_last_capture_result,
  get_settings,
  get_last_selection,
  hide_toast,
  open_last_capture,
  open_capture_path,
  submit_area_selection,
  update_settings,
};
use shell::hotkeys::register_hotkey_from_state;
use shell::state::AppState;
use shell::tray::build_tray;
use shell::windows::show_window;
use tauri::WindowEvent;

fn main() {
  tauri::Builder::default()
    .plugin(tauri_plugin_global_shortcut::Builder::new().build())
    .on_window_event(|window, event| {
      if let WindowEvent::CloseRequested { api, .. } = event {
        if window.label() == "settings" {
          api.prevent_close();
          let _ = window.hide();
        }
      }
    })
    .manage(AppState::default())
    .setup(|app| {
      build_tray(app.handle())?;
      register_hotkey_from_state(app.handle())
        .map_err(|error| -> Box<dyn std::error::Error> { Box::new(std::io::Error::other(error.message)) })?;
      show_window(app.handle(), "settings")
        .map_err(|error| -> Box<dyn std::error::Error> { Box::new(std::io::Error::other(error.message)) })?;
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      begin_area_capture,
      capture_active_display,
      cancel_capture,
      get_last_capture_result,
      get_settings,
      get_last_selection,
      hide_toast,
      open_capture_path,
      update_settings,
      submit_area_selection,
      open_last_capture
    ])
    .run(tauri::generate_context!())
    .expect("error while running Capture");
}
