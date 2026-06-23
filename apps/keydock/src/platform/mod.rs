#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub use windows::run;

#[cfg(target_os = "windows")]
pub use windows::report_error;

#[cfg(not(target_os = "windows"))]
pub fn run() -> Result<(), String> {
    Err("KeyDock v1 only supports Windows 10 22H2+ / Windows 11.".to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn report_error(_message: &str) {}
