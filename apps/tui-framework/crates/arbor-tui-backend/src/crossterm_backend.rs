// CrosstermBackend — production terminal backend using crossterm.
// Handles raw mode, alternate screen, ANSI emission, cursor control.

use std::io::{stdout, Stdout, Write};

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::style::{Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, size, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, queue};

use arbor_tui_core::backend::{TerminalBackend, TerminalGuard};
use arbor_tui_core::cell::{AnsiColor, Attrs};
use arbor_tui_core::diff::DirtyRegion;
use arbor_tui_core::screen::VirtualScreen;

pub struct CrosstermBackend {
    stdout: Stdout,
    cursor_hidden: bool,
    alt_screen: bool,
    no_color: bool,
}

impl CrosstermBackend {
    pub fn new() -> Self {
        let no_color = std::env::var("NO_COLOR").map(|v| v == "1").unwrap_or(false);
        Self {
            stdout: stdout(),
            cursor_hidden: false,
            alt_screen: false,
            no_color,
        }
    }

    /// Convert framework AnsiColor to crossterm Color.
    fn to_color(color: &AnsiColor) -> Color {
        if let Some(rgb) = &color.true_color {
            // TrueColor available — use it directly
            Color::Rgb { r: rgb.0, g: rgb.1, b: rgb.2 }
        } else {
            // 256-color palette
            Color::AnsiValue(color.palette.0)
        }
    }

    /// Write a single cell to the current cursor position.
    /// When NO_COLOR=1, foreground/background colors are skipped; text attributes are preserved.
    fn write_cell(&mut self, ch: char, fg: &AnsiColor, bg: &AnsiColor, attrs: &Attrs) {
        if !self.no_color {
            let _ = queue!(
                self.stdout,
                SetForegroundColor(Self::to_color(fg)),
                SetBackgroundColor(Self::to_color(bg)),
            );
        }

        if attrs.bold {
            let _ = queue!(self.stdout, SetAttribute(Attribute::Bold));
        }
        if attrs.dim {
            let _ = queue!(self.stdout, SetAttribute(Attribute::Dim));
        }
        if attrs.italic {
            let _ = queue!(self.stdout, SetAttribute(Attribute::Italic));
        }
        if attrs.underline {
            let _ = queue!(self.stdout, SetAttribute(Attribute::Underlined));
        }
        if attrs.reverse {
            let _ = queue!(self.stdout, SetAttribute(Attribute::Reverse));
        }

        let _ = queue!(self.stdout, Print(ch));

        // Reset attributes after each cell to avoid leaking styles
        let _ = queue!(
            self.stdout,
            SetAttribute(Attribute::Reset),
        );
    }
}

struct CrosstermGuard;

impl TerminalGuard for CrosstermGuard {
    fn restore(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, Show, Clear(ClearType::All));
    }
}

impl TerminalBackend for CrosstermBackend {
    fn enter_raw_mode(&self) -> Box<dyn TerminalGuard> {
        let _ = enable_raw_mode();
        Box::new(CrosstermGuard)
    }

    fn size(&self) -> (u16, u16) {
        size().unwrap_or((80, 24))
    }

    fn emit(&mut self, regions: &[DirtyRegion], screen: &VirtualScreen) {
        if regions.is_empty() {
            return;
        }

        // Sort and merge regions for optimal cursor movement
        let mut sorted: Vec<_> = regions.to_vec();
        sorted.sort_by(|a, b| a.row.cmp(&b.row).then(a.start_col.cmp(&b.start_col)));

        let merged = merge_adjacent(&sorted);

        let mut current_row: Option<u16> = None;
        let mut current_col: Option<u16> = None;

        for region in &merged {
            // Move to region start if needed
            if current_row != Some(region.row) || current_col != Some(region.start_col) {
                let _ = queue!(self.stdout, MoveTo(region.start_col, region.row));
                current_row = Some(region.row);
                current_col = Some(region.start_col);
            }

            // Write cells in this region, skipping phantom (wide-char continuation) cells
            for col in region.start_col..region.end_col {
                let cell = screen.cell_at(col, region.row);
                if cell.phantom {
                    // Phantom column of a wide char — skip emission to avoid
                    // overwriting the second half of the CJK character.
                    current_col = Some(col + 1);
                    continue;
                }
                self.write_cell(cell.ch, &cell.fg, &cell.bg, &cell.attrs);
            }
            current_col = Some(region.end_col);
        }

        let _ = self.stdout.flush();
    }

    fn hide_cursor(&mut self) {
        if !self.cursor_hidden {
            let _ = execute!(self.stdout, Hide);
            self.cursor_hidden = true;
        }
    }

    fn show_cursor(&mut self) {
        if self.cursor_hidden {
            let _ = execute!(self.stdout, Show);
            self.cursor_hidden = false;
        }
    }

    fn enter_alternate_screen(&mut self) {
        if !self.alt_screen {
            let _ = execute!(self.stdout, EnterAlternateScreen);
            self.alt_screen = true;
        }
    }

    fn exit_alternate_screen(&mut self) {
        if self.alt_screen {
            let _ = execute!(self.stdout, LeaveAlternateScreen);
            self.alt_screen = false;
        }
    }

    fn clear(&mut self) {
        let _ = execute!(self.stdout, Clear(ClearType::All));
    }

    fn flush(&mut self) {
        let _ = self.stdout.flush();
    }
}

/// Merge adjacent dirty regions on the same row.
fn merge_adjacent(regions: &[DirtyRegion]) -> Vec<DirtyRegion> {
    if regions.is_empty() {
        return vec![];
    }

    let mut merged: Vec<DirtyRegion> = vec![regions[0].clone()];

    for next in &regions[1..] {
        let last = merged.last_mut().unwrap();
        if next.row == last.row && next.start_col <= last.end_col {
            // Touching or overlapping — extend
            last.end_col = last.end_col.max(next.end_col);
        } else {
            merged.push(next.clone());
        }
    }

    merged
}
