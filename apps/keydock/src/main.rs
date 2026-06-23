#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod platform;

fn main() {
    if let Err(error) = platform::run() {
        let message = format!("KeyDock failed: {error}");
        platform::report_error(&message);
        #[cfg(not(target_os = "windows"))]
        eprintln!("{message}");
        std::process::exit(1);
    }
}
