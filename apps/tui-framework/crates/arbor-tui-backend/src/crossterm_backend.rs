// CrosstermBackend — production terminal backend using crossterm.
// Handles raw mode, alternate screen, ANSI emission, cursor control.
// All I/O errors are propagated via BackendResult, not silently ignored.

use std::io::{stdout, Stdout, Write};
use std::time::Instant;

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::style::{
    Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, size, Clear, ClearType, EnterAlternateScreen,
    LeaveAlternateScreen,
};
use crossterm::{execute, queue};

use arbor_tui_primitives::cell::{AnsiColor, Attrs};
use arbor_tui_render::backend::{BackendError, BackendResult, TerminalBackend, TerminalGuard};
use arbor_tui_render::diff::DirtyRegion;
use arbor_tui_render::screen::VirtualScreen;

pub struct CrosstermBackend {
    stdout: Stdout,
    cursor_hidden: bool,
    alt_screen: bool,
    no_color: bool,
    last_queue_us: u64,
    last_flush_us: u64,
}

impl Default for CrosstermBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CrosstermBackend {
    pub fn new() -> Self {
        let no_color = std::env::var("NO_COLOR").map(|v| v == "1").unwrap_or(false);
        Self {
            stdout: stdout(),
            cursor_hidden: false,
            alt_screen: false,
            no_color,
            last_queue_us: 0,
            last_flush_us: 0,
        }
    }

    /// Convert framework AnsiColor to crossterm Color.
    fn to_color(color: &AnsiColor) -> Color {
        if let Some(rgb) = &color.true_color {
            Color::Rgb {
                r: rgb.0,
                g: rgb.1,
                b: rgb.2,
            }
        } else {
            Color::AnsiValue(color.palette.0)
        }
    }

    /// Write a single cell to the current cursor position, propagating errors.
    fn write_cell(
        &mut self,
        ch: char,
        fg: &AnsiColor,
        bg: &AnsiColor,
        attrs: &Attrs,
    ) -> BackendResult<()> {
        if !self.no_color {
            queue!(
                self.stdout,
                SetForegroundColor(Self::to_color(fg)),
                SetBackgroundColor(Self::to_color(bg)),
            )?;
        }

        if attrs.bold {
            queue!(self.stdout, SetAttribute(Attribute::Bold))?;
        }
        if attrs.dim {
            queue!(self.stdout, SetAttribute(Attribute::Dim))?;
        }
        if attrs.italic {
            queue!(self.stdout, SetAttribute(Attribute::Italic))?;
        }
        if attrs.underline {
            queue!(self.stdout, SetAttribute(Attribute::Underlined))?;
        }
        if attrs.reverse {
            queue!(self.stdout, SetAttribute(Attribute::Reverse))?;
        }

        queue!(self.stdout, Print(ch))?;
        // Reset attributes after each cell to avoid leaking styles
        queue!(self.stdout, SetAttribute(Attribute::Reset))?;
        Ok(())
    }
}

struct CrosstermGuard;

impl TerminalGuard for CrosstermGuard {
    fn restore(&mut self) {
        // Best-effort restoration — we're already in a signal handler or
        // panic path. The RAII Drop provides a second layer of protection.
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, Show, Clear(ClearType::All));
    }
}

impl TerminalBackend for CrosstermBackend {
    fn enter_raw_mode(&self) -> BackendResult<Box<dyn TerminalGuard>> {
        enable_raw_mode().map_err(|e| BackendError::with_source("failed to enter raw mode", e))?;
        Ok(Box::new(CrosstermGuard))
    }

    fn size(&self) -> BackendResult<(u16, u16)> {
        size().map_err(|e| BackendError::with_source("failed to query terminal size", e))
    }

    fn emit(&mut self, regions: &[DirtyRegion], screen: &VirtualScreen) -> BackendResult<()> {
        self.last_queue_us = 0;
        self.last_flush_us = 0;

        if regions.is_empty() {
            return Ok(());
        }

        let t0 = Instant::now();

        let mut sorted: Vec<_> = regions.to_vec();
        sorted.sort_by(|a, b| a.row.cmp(&b.row).then(a.start_col.cmp(&b.start_col)));

        let merged = merge_adjacent(&sorted);

        let mut current_row: Option<u16> = None;
        let mut current_col: Option<u16> = None;
        let region_count = merged.len();

        for (i, region) in merged.iter().enumerate() {
            // Move cursor if row changed OR col is not contiguous
            if current_row != Some(region.row) || current_col != Some(region.start_col) {
                queue!(self.stdout, MoveTo(region.start_col, region.row))?;
                current_row = Some(region.row);
            }

            for col in region.start_col..region.end_col {
                let cell = screen.cell_at(col, region.row);
                if cell.phantom {
                    continue;
                }
                self.write_cell(cell.ch, &cell.fg, &cell.bg, &cell.attrs)?;
            }
            current_col = Some(region.end_col);

            // Clear to end of line after the LAST region on each row.
            // Prevents stale content from persisting after resize or when
            // new content is shorter than old content on the same row.
            let is_last_on_row = i + 1 >= region_count || merged[i + 1].row != region.row;
            if is_last_on_row {
                queue!(self.stdout, Clear(ClearType::UntilNewLine))?;
            }
        }

        self.last_queue_us = t0.elapsed().as_micros() as u64;

        let t1 = Instant::now();
        self.stdout.flush()?;
        self.last_flush_us = t1.elapsed().as_micros() as u64;

        Ok(())
    }

    fn hide_cursor(&mut self) -> BackendResult<()> {
        if !self.cursor_hidden {
            execute!(self.stdout, Hide)?;
            self.cursor_hidden = true;
        }
        Ok(())
    }

    fn show_cursor(&mut self) -> BackendResult<()> {
        if self.cursor_hidden {
            execute!(self.stdout, Show)?;
            self.cursor_hidden = false;
        }
        Ok(())
    }

    fn enter_alternate_screen(&mut self) -> BackendResult<()> {
        if !self.alt_screen {
            execute!(self.stdout, EnterAlternateScreen)?;
            self.alt_screen = true;
        }
        Ok(())
    }

    fn exit_alternate_screen(&mut self) -> BackendResult<()> {
        if self.alt_screen {
            execute!(self.stdout, LeaveAlternateScreen)?;
            self.alt_screen = false;
        }
        Ok(())
    }

    fn clear(&mut self) -> BackendResult<()> {
        execute!(self.stdout, Clear(ClearType::All))?;
        Ok(())
    }

    fn flush(&mut self) -> BackendResult<()> {
        self.stdout.flush()?;
        Ok(())
    }

    fn last_emit_queue_us(&self) -> u64 {
        self.last_queue_us
    }
    fn last_emit_flush_us(&self) -> u64 {
        self.last_flush_us
    }
}

/// Merge adjacent dirty regions on the same row.
fn merge_adjacent(regions: &[DirtyRegion]) -> Vec<DirtyRegion> {
    if regions.is_empty() {
        return vec![];
    }

    let mut merged: Vec<DirtyRegion> = vec![regions[0].clone()];

    for next in &regions[1..] {
        let last = merged
            .last_mut()
            .expect("merged must be non-empty after initial push");
        if next.row == last.row && next.start_col <= last.end_col {
            last.end_col = last.end_col.max(next.end_col);
        } else {
            merged.push(next.clone());
        }
    }

    merged
}
