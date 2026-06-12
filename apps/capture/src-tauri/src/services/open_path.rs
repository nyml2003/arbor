use crate::domain::CaptureError;

pub fn open_path_with_default_viewer(path: &str) -> Result<(), CaptureError> {
  #[cfg(target_os = "windows")]
  {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let wide_path = OsStr::new(path)
      .encode_wide()
      .chain(std::iter::once(0))
      .collect::<Vec<u16>>();

    let result = unsafe {
      ShellExecuteW(
        None,
        PCWSTR::null(),
        PCWSTR(wide_path.as_ptr()),
        PCWSTR::null(),
        PCWSTR::null(),
        SW_SHOWNORMAL,
      )
    };

    let code = result.0 as isize;
    if code <= 32 {
      return Err(CaptureError::new(
        "open_path_failed",
        format!("ShellExecuteW failed with code {code}."),
      ));
    }

    return Ok(());
  }

  #[cfg(target_os = "macos")]
  let mut command = {
    let mut command = Command::new("open");
    command.arg(path);
    command
  };

  #[cfg(all(unix, not(target_os = "macos")))]
  let mut command = {
    use std::process::Command;
    let mut command = Command::new("xdg-open");
    command.arg(path);
    command
  };

  #[cfg(not(target_os = "windows"))]
  command
    .spawn()
    .map_err(|error| CaptureError::new("open_path_failed", error.to_string()))?;

  #[cfg(not(target_os = "windows"))]
  Ok(())
}
