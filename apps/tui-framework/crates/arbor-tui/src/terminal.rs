// Terminal lifecycle management.
// RAII guard for raw mode enter/exit, plus panic hook for emergency recovery.

use arbor_tui_core::backend::TerminalBackend;

/// Install a panic hook that restores the terminal before printing the panic.
///
/// Writes ANSI escape sequences directly to stdout (no framework dependency)
/// to exit alternate screen and show cursor. `TerminalGuard::Drop` provides
/// a second layer of protection if the panic hook doesn't run.
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        use std::io::Write;
        let mut stdout = std::io::stdout();
        let _ = write!(stdout, "\x1b[?1049l"); // exit alternate screen
        let _ = write!(stdout, "\x1b[?25h");   // show cursor
        let _ = stdout.flush();
        eprintln!("[arbor-tui] PANIC: {}", info);
    }));
}

/// Wraps a TerminalGuard to manage terminal state lifecycle.
pub struct TerminalHandle {
    guard: Option<Box<dyn arbor_tui_core::backend::TerminalGuard>>,
}

impl TerminalHandle {
    /// Enter raw mode and acquire the terminal.
    pub fn acquire(backend: &dyn TerminalBackend) -> Self {
        let guard = backend.enter_raw_mode();
        Self { guard: Some(guard) }
    }

    /// Temporarily release the terminal (for subprocess execution).
    pub fn release(&mut self) {
        if let Some(mut guard) = self.guard.take() {
            guard.restore();
            drop(guard);
        }
    }

    /// Re-acquire after a temporary release.
    pub fn reacquire(&mut self, backend: &dyn TerminalBackend) {
        self.guard = Some(backend.enter_raw_mode());
    }
}

impl Drop for TerminalHandle {
    fn drop(&mut self) {
        // Guard's Drop handles terminal restoration
        self.guard.take();
    }
}
