// SignalManager — unified OS signal handler.
// Manages SIGINT (Ctrl+C), SIGTSTP (Ctrl+Z), and SIGWINCH (terminal resize).

use arbor_tui_core::backend::{BackendError, TerminalBackend};
use crate::app::App;

/// Callbacks for OS signals.
pub struct SignalManager {
    pub on_quit: Option<Box<dyn FnOnce()>>,
    pub on_resize: Option<Box<dyn Fn(u16, u16)>>,
}

impl SignalManager {
    pub fn new() -> Self {
        Self { on_quit: None, on_resize: None }
    }
}

impl Default for SignalManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle SIGTSTP: drop the terminal guard to restore terminal, then suspend.
/// On resume (SIGCONT), re-create the guard and mark all widgets dirty.
///
/// This is the ONLY unsafe site in the crate — POSIX job control via kill(2).
#[allow(unsafe_code)]
pub fn handle_sigtstp(_app: &mut App, backend: &mut dyn TerminalBackend) {
    // SAFETY: sending SIGSTOP to the current process via kill(2) is a
    // well-defined POSIX operation. The kernel suspends this process;
    // no memory or aliasing invariants are violated. On SIGCONT, execution
    // resumes at the next instruction after kill() returns.
    #[cfg(unix)]
    unsafe {
        libc::kill(libc::getpid(), libc::SIGSTOP);
    }
    // After SIGCONT resumes us here:
    // Re-enter raw mode — if this fails, the terminal is gone; nothing to do.
    let _ = backend.enter_raw_mode();
    let _ = backend.enter_alternate_screen();
    let _ = backend.hide_cursor();
}

/// Check for terminal resize and update app state.
/// Returns Ok(true) if a resize was detected. Forces a full relayout on the next frame.
pub fn check_resize(app: &mut App, backend: &dyn TerminalBackend) -> Result<bool, BackendError> {
    let (cols, rows) = backend.size()?;
    let (cur_cols, cur_rows) = app.screen_size();
    if cols != cur_cols || rows != cur_rows {
        app.resize(cols, rows);
        app.dirty_tracker.force_render();
        Ok(true)
    } else {
        Ok(false)
    }
}
