use std::sync::Mutex;

use crate::domain::{AreaSelection, CaptureResult, CaptureSettings};

#[derive(Default)]
pub struct AppState {
  pub settings: Mutex<CaptureSettings>,
  pub last_capture_path: Mutex<Option<String>>,
  pub last_capture_result: Mutex<Option<CaptureResult>>,
  pub last_selection: Mutex<Option<AreaSelection>>,
}
