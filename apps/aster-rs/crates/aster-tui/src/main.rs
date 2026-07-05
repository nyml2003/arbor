// Aster - AI chat TUI.

mod frame_stats;
mod runner;
mod state;
mod ui;

fn main() {
    if let Err(error) = runner::run() {
        eprintln!("[aster] fatal error: {error:?}");
        std::process::exit(1);
    }
}
