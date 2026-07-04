// TerminalBackend trait — defined in domain layer, zero I/O dependencies.
// Infrastructure adapters (crossterm, simulated) implement this trait.

use crate::diff::DirtyRegion;
use crate::screen::VirtualScreen;

/// Terminal output backend.
///
/// Implementations handle all terminal I/O: raw mode, alternate screen,
/// ANSI escape code emission, cursor control.
pub trait TerminalBackend {
    /// Enter raw mode and return a RAII guard.
    /// The guard restores: raw mode, echo, canonical mode, cursor state,
    /// and alternate screen on drop.
    fn enter_raw_mode(&self) -> Box<dyn TerminalGuard>;

    /// Current terminal size in (cols, rows).
    fn size(&self) -> (u16, u16);

    /// Emit ANSI sequences for dirty regions.
    ///
    /// The backend is responsible for:
    /// 1. Sorting regions by (row, start_col)
    /// 2. Merging adjacent regions on the same row
    /// 3. Generating optimized cursor-move sequences
    /// 4. Flushing stdout at the end of the call
    fn emit(&mut self, regions: &[DirtyRegion], screen: &VirtualScreen);

    /// Hide the terminal cursor.
    fn hide_cursor(&mut self);
    /// Show the terminal cursor.
    fn show_cursor(&mut self);

    /// Enter the alternate screen buffer.
    fn enter_alternate_screen(&mut self);
    /// Exit the alternate screen buffer, restoring the original screen content.
    fn exit_alternate_screen(&mut self);

    /// Clear the entire screen.
    fn clear(&mut self);

    /// Flush buffered output to the terminal.
    fn flush(&mut self);
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
