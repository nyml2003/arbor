// SimulatedBackend — in-memory terminal for testing.
// Records all ANSI output and allows assertion on rendered content.

use arbor_tui_core::backend::{BackendResult, TerminalBackend, TerminalGuard};
use arbor_tui_core::diff::DirtyRegion;
use arbor_tui_core::screen::VirtualScreen;

/// An in-memory terminal backend for CI/testing.
///
/// Instead of writing to a real terminal, it:
/// - Maintains an internal VirtualScreen (the "display")
/// - Records all emitted ANSI sequences as bytes
/// - Allows tests to assert on rendered content
pub struct SimulatedBackend {
    screen: VirtualScreen,
    /// Accumulated ANSI output from all emit calls.
    pub output: Vec<u8>,
    alt_screen: bool,
}

impl SimulatedBackend {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            screen: VirtualScreen::new(cols, rows),
            output: Vec::new(),
            alt_screen: false,
        }
    }

    /// Check if the output contains the given UTF-8 string.
    pub fn output_contains(&self, needle: &str) -> bool {
        let output_str = String::from_utf8_lossy(&self.output);
        output_str.contains(needle)
    }

    /// Get a reference to the internal screen (the current "displayed" state).
    pub fn screen(&self) -> &VirtualScreen {
        &self.screen
    }
}

struct SimulatedGuard;

impl TerminalGuard for SimulatedGuard {
    fn restore(&mut self) {}
}

impl Drop for SimulatedGuard {
    fn drop(&mut self) {}
}

impl TerminalBackend for SimulatedBackend {
    fn enter_raw_mode(&self) -> BackendResult<Box<dyn TerminalGuard>> {
        Ok(Box::new(SimulatedGuard))
    }

    fn size(&self) -> BackendResult<(u16, u16)> {
        Ok((self.screen.cols(), self.screen.rows()))
    }

    fn emit(&mut self, regions: &[DirtyRegion], screen: &VirtualScreen) -> BackendResult<()> {
        use std::fmt::Write as FmtWrite;
        let mut buf = String::new();
        for region in regions {
            let _ = writeln!(
                buf,
                "CSI {};{}H..{};{}H",
                region.row + 1,
                region.start_col + 1,
                region.row + 1,
                region.end_col
            );
        }
        self.output.extend(buf.as_bytes());
        // Blit the dirty cells into our internal screen
        for region in regions {
            for col in region.start_col..region.end_col {
                let src = screen.cell_at(col, region.row);
                if let Some(dest) = self.screen.cell_at_mut(col, region.row) {
                    *dest = src;
                }
            }
        }
        Ok(())
    }

    fn hide_cursor(&mut self) -> BackendResult<()> {
        self.output.extend(b"CSI ? 25 l");
        Ok(())
    }

    fn show_cursor(&mut self) -> BackendResult<()> {
        self.output.extend(b"CSI ? 25 h");
        Ok(())
    }

    fn enter_alternate_screen(&mut self) -> BackendResult<()> {
        self.alt_screen = true;
        self.output.extend(b"CSI ? 1049 h");
        Ok(())
    }

    fn exit_alternate_screen(&mut self) -> BackendResult<()> {
        self.alt_screen = false;
        self.output.extend(b"CSI ? 1049 l");
        Ok(())
    }

    fn clear(&mut self) -> BackendResult<()> {
        self.output.extend(b"CSI 2 J");
        Ok(())
    }

    fn flush(&mut self) -> BackendResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbor_tui_core::diff::{diff, merge_regions};

    #[test]
    fn simulated_backend_records_output() {
        let mut backend = SimulatedBackend::new(80, 24);
        backend.enter_alternate_screen().unwrap();
        assert!(backend.output_contains("1049 h"));
    }

    #[test]
    fn emit_updates_internal_screen() {
        let mut backend = SimulatedBackend::new(10, 5);

        let old = backend.screen().clone();
        let mut new_screen = VirtualScreen::new(10, 5);
        new_screen.cell_at_mut(0, 0).unwrap().ch = 'X';

        let mut regions = diff(&old, &new_screen);
        merge_regions(&mut regions);
        backend.emit(&regions, &new_screen).unwrap();

        assert_eq!(backend.screen().cell_at(0, 0).ch, 'X');
    }
}
