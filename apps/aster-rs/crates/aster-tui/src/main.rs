// Aster - AI chat TUI.

use anyhow::Result;

#[cfg(feature = "bench-log")]
mod bench;
mod runner;
mod state;
mod ui;

fn main() {
    if let Err(error) = run() {
        eprintln!("[aster] fatal error: {error:?}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    #[cfg(feature = "bench-log")]
    if bench::should_run_bench() {
        return bench::run_from_env();
    }

    #[cfg(not(feature = "bench-log"))]
    if std::env::args().any(|arg| arg == "--bench") {
        anyhow::bail!("--bench requires building aster-tui with --features bench-log");
    }

    runner::run()
}
