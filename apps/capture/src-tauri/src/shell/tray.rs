use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Runtime};

use super::commands::{capture_active_display, open_last_capture};
use super::windows::show_window;

pub fn build_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
  let area_capture = MenuItem::with_id(app, "area_capture", "区域截图", true, None::<&str>)?;
  let current_display = MenuItem::with_id(
    app,
    "current_display",
    "当前屏幕截图",
    true,
    None::<&str>,
  )?;
  let open_last = MenuItem::with_id(
    app,
    "open_last_capture",
    "打开最近一次截图",
    true,
    None::<&str>,
  )?;
  let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
  let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

  let menu = Menu::with_items(
    app,
    &[&area_capture, &current_display, &open_last, &settings, &quit],
  )?;

  TrayIconBuilder::new()
    .menu(&menu)
    .show_menu_on_left_click(true)
    .on_menu_event(|app, event| match event.id.as_ref() {
      "area_capture" => {
        if let Err(error) = show_window(app, "overlay") {
          eprintln!("failed to show overlay: {}", error.message);
        }
      }
      "settings" => {
        if let Err(error) = show_window(app, "settings") {
          eprintln!("failed to show settings: {}", error.message);
        }
      }
      "current_display" => {
        if let Err(error) = tauri::async_runtime::block_on(capture_active_display(
          app.clone(),
          app.state(),
        )) {
          eprintln!("failed to capture current display: {}", error.message);
        }
      }
      "open_last_capture" => {
        if let Err(error) = tauri::async_runtime::block_on(open_last_capture(app.state())) {
          eprintln!("failed to open last capture: {}", error.message);
        }
      }
      "quit" => app.exit(0),
      _ => {}
    })
    .build(app)?;

  Ok(())
}
