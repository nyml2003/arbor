mod capture_area;
mod open_path;
#[cfg(windows)]
mod clipboard;
#[cfg(windows)]
mod windows_hdr_capture;

pub use capture_area::{
  capture_active_display_to_file,
  capture_area_to_file,
};
#[cfg(windows)]
pub use clipboard::copy_rgba_to_clipboard;
pub use open_path::open_path_with_default_viewer;
