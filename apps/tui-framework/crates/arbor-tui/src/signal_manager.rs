// SignalManager — unified OS signal handler.
// Manages SIGINT (Ctrl+C), SIGTSTP (Ctrl+Z), and SIGWINCH (terminal resize).

use crate::app::App;
use arbor_tui_render::backend::{BackendError, TerminalBackend};

/// Callbacks for OS signals.
pub struct SignalManager {
    pub on_quit: Option<Box<dyn FnOnce()>>,
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
///
/// This is the ONLY unsafe site in the crate — POSIX job control via kill(2).
#[allow(unsafe_code)]
pub fn handle_sigtstp(_app: &mut App, backend: &mut dyn TerminalBackend) {
    // SAFETY: sending SIGSTOP to the current process via kill(2) is a
    // well-defined POSIX operation. The kernel suspends this process;
    // no memory or aliasing invariants are violated.
    #[cfg(unix)]
    unsafe {
        libc::kill(libc::getpid(), libc::SIGSTOP);
    }
    let _ = backend.enter_raw_mode();
    let _ = backend.enter_alternate_screen();
    let _ = backend.hide_cursor();
}

/// Check for terminal resize with debounce. Returns true if a resize was applied.
/// `stable_ms`: how long the size must be stable before applying (50ms recommended).
pub fn check_resize(
    app: &mut App,
    backend: &dyn TerminalBackend,
    stable_ms: u64,
) -> Result<bool, BackendError> {
    let (cols, rows) = backend.size()?;
    Ok(app.check_resize(cols, rows, stable_ms))
}
