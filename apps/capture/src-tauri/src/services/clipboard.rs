use arboard::{Clipboard, ImageData};

use crate::domain::CaptureError;

pub fn copy_rgba_to_clipboard(
  width: u32,
  height: u32,
  bytes: &[u8],
) -> Result<(), CaptureError> {
  let mut clipboard = Clipboard::new()
    .map_err(|error| CaptureError::new("clipboard_unavailable", error.to_string()))?;

  clipboard
    .set_image(ImageData {
      width: width as usize,
      height: height as usize,
      bytes: bytes.into(),
    })
    .map_err(|error| CaptureError::new("clipboard_write_failed", error.to_string()))
}
