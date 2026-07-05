// Terminal lifecycle management.
// RAII guard for raw mode enter/exit, plus panic hook for emergency recovery.

use arbor_tui_render::backend::{BackendError, TerminalBackend};

/// Install a panic hook that restores the terminal before printing the panic.
///
/// Writes ANSI escape sequences directly to stdout (no framework dependency)
/// to exit alternate screen and show cursor. `TerminalGuard::Drop` provides
/// a second layer of protection if the panic hook doesn't run.
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        use std::io::Write;
        let mut stdout = std::io::stdout();
        // Best-effort — we're already panicking, so silently ignore write errors.
        let _ = write!(stdout, "\x1b[?1049l"); // exit alternate screen
        let _ = write!(stdout, "\x1b[?25h"); // show cursor
        let _ = stdout.flush();
        eprintln!("[arbor-tui] PANIC: {info}");
    }));
}

/// Wraps a TerminalGuard to manage terminal state lifecycle.
pub struct TerminalHandle {
    guard: Option<Box<dyn arbor_tui_render::backend::TerminalGuard>>,
}

impl TerminalHandle {
    /// Enter raw mode and acquire the terminal.
    pub fn acquire(backend: &dyn TerminalBackend) -> Result<Self, BackendError> {
        let guard = backend.enter_raw_mode()?;
        Ok(Self { guard: Some(guard) })
    }

    /// Temporarily release the terminal (for subprocess execution).
    pub fn release(&mut self) {
        if let Some(mut guard) = self.guard.take() {
            guard.restore();
            drop(guard);
        }
    }

    /// Re-acquire after a temporary release.
    pub fn reacquire(&mut self, backend: &dyn TerminalBackend) -> Result<(), BackendError> {
        self.guard = Some(backend.enter_raw_mode()?);
        Ok(())
    }
}

impl Drop for TerminalHandle {
    fn drop(&mut self) {
        self.guard.take();
    }
}
