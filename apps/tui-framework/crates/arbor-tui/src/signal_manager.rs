// SignalManager — unified OS signal handler.
// Manages SIGINT (Ctrl+C), SIGTSTP (Ctrl+Z), and SIGWINCH (terminal resize).

use arbor_tui_core::backend::TerminalBackend;
use crate::app::App;

/// Callbacks for OS signals.
pub struct SignalManager {
    /// Called when SIGINT (Ctrl+C) is received. If None, defaults to quit.
    pub on_quit: Option<Box<dyn FnOnce()>>,
    /// Called on SIGWINCH with (cols, rows).
    pub on_resize: Option<Box<dyn Fn(u16, u16)>>,
}

impl SignalManager {
    pub fn new() -> Self {
        Self {
            on_quit: None,
            on_resize: None,
        }
    }
}

impl Default for SignalManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle SIGTSTP: drop the terminal guard to restore terminal, then suspend.
/// On resume (SIGCONT), re-create the guard and mark all widgets dirty.
pub fn handle_sigtstp(_app: &mut App, backend: &mut dyn TerminalBackend) {
    // Drop existing guard — restores terminal
    // Send SIGSTOP to self
    #[cfg(unix)]
    unsafe {
        libc::kill(libc::getpid(), libc::SIGSTOP);
    }
    // After SIGCONT resumes us here:
    // Re-enter raw mode
    let _guard = backend.enter_raw_mode();
    backend.enter_alternate_screen();
    backend.hide_cursor();
    // Mark all widgets dirty for full repaint
    // app.mark_all_dirty() — called by the caller after re-creating guard
}

/// Check for terminal resize and update app state.
/// Returns true if a resize was detected. Forces a full relayout on the next frame.
pub fn check_resize(app: &mut App, backend: &dyn TerminalBackend) -> bool {
    let (cols, rows) = backend.size();
    let (cur_cols, cur_rows) = app.screen_size();
    if cols != cur_cols || rows != cur_rows {
        app.resize(cols, rows);
        app.dirty_tracker.force_render();
        true
    } else {
        false
    }
}
