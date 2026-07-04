// TerminalBackend trait — defined in domain layer, zero I/O dependencies.
// Infrastructure adapters (crossterm, simulated) implement this trait.
//
// All I/O methods return Result so the caller can decide how to handle failure
// rather than panicking deep in the backend.

use crate::diff::DirtyRegion;
use crate::screen::VirtualScreen;

/// Errors from terminal backend operations.
/// Re-exported from arbor-tui-backend via type alias in the app crate.
pub type BackendResult<T> = Result<T, BackendError>;

/// Errors that can occur during terminal I/O.
#[derive(Debug)]
pub struct BackendError {
    pub message: String,
    #[allow(dead_code)]
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl BackendError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into(), source: None }
    }

    pub fn with_source(message: impl Into<String>, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self { message: message.into(), source: Some(Box::new(source)) }
    }
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for BackendError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref() as &dyn std::error::Error)
    }
}

impl From<std::io::Error> for BackendError {
    fn from(e: std::io::Error) -> Self {
        BackendError::with_source("I/O error", e)
    }
}

/// Terminal output backend.
///
/// Implementations handle all terminal I/O: raw mode, alternate screen,
/// ANSI escape code emission, cursor control.
pub trait TerminalBackend {
    /// Enter raw mode and return a RAII guard.
    /// The guard restores: raw mode, echo, canonical mode, cursor state,
    /// and alternate screen on drop.
    fn enter_raw_mode(&self) -> BackendResult<Box<dyn TerminalGuard>>;

    /// Current terminal size in (cols, rows).
    fn size(&self) -> BackendResult<(u16, u16)>;

    /// Emit ANSI sequences for dirty regions.
    ///
    /// The backend is responsible for:
    /// 1. Sorting regions by (row, start_col)
    /// 2. Merging adjacent regions on the same row
    /// 3. Generating optimized cursor-move sequences
    /// 4. Flushing stdout at the end of the call
    fn emit(&mut self, regions: &[DirtyRegion], screen: &VirtualScreen) -> BackendResult<()>;

    /// Hide the terminal cursor.
    fn hide_cursor(&mut self) -> BackendResult<()>;
    /// Show the terminal cursor.
    fn show_cursor(&mut self) -> BackendResult<()>;

    /// Enter the alternate screen buffer.
    fn enter_alternate_screen(&mut self) -> BackendResult<()>;
    /// Exit the alternate screen buffer, restoring the original screen content.
    fn exit_alternate_screen(&mut self) -> BackendResult<()>;

    /// Clear the entire screen.
    fn clear(&mut self) -> BackendResult<()>;

    /// Flush buffered output to the terminal.
    fn flush(&mut self) -> BackendResult<()>;

    /// Time spent queuing ANSI sequences (pure memory) in the last `emit()` call, in µs.
    fn last_emit_queue_us(&self) -> u64 { 0 }
    /// Time spent in the stdout `flush()` syscall in the last `emit()` call, in µs.
    fn last_emit_flush_us(&self) -> u64 { 0 }
}

/// RAII guard for terminal raw mode.
///
/// On drop, restores: echo, canonical mode, cursor visibility,
/// and exits the alternate screen.
pub trait TerminalGuard {
    /// Explicitly restore terminal state before drop.
    /// Called by signal handlers (SIGTSTP) before suspending.
    fn restore(&mut self);
}
