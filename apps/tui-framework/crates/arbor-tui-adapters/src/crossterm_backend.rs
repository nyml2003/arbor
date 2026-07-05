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

use arbor_tui_domain::backend::{BackendError, BackendResult, TerminalBackend, TerminalGuard};
use arbor_tui_domain::cell::{AnsiColor, Attrs};
use arbor_tui_domain::diff::DirtyRegion;
use arbor_tui_domain::screen::VirtualScreen;

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

        emit_regions_to(&mut self.stdout, self.no_color, regions, screen)?;

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

fn queue_style_to<W: Write>(
    writer: &mut W,
    no_color: bool,
    fg: &AnsiColor,
    bg: &AnsiColor,
    attrs: &Attrs,
) -> BackendResult<()> {
    if !no_color {
        queue!(
            writer,
            SetForegroundColor(CrosstermBackend::to_color(fg)),
            SetBackgroundColor(CrosstermBackend::to_color(bg)),
        )?;
    }

    if attrs.bold {
        queue!(writer, SetAttribute(Attribute::Bold))?;
    }
    if attrs.dim {
        queue!(writer, SetAttribute(Attribute::Dim))?;
    }
    if attrs.italic {
        queue!(writer, SetAttribute(Attribute::Italic))?;
    }
    if attrs.underline {
        queue!(writer, SetAttribute(Attribute::Underlined))?;
    }
    if attrs.reverse {
        queue!(writer, SetAttribute(Attribute::Reverse))?;
    }
    Ok(())
}

fn write_run_to<W: Write>(
    writer: &mut W,
    no_color: bool,
    text: &str,
    fg: &AnsiColor,
    bg: &AnsiColor,
    attrs: &Attrs,
) -> BackendResult<()> {
    if text.is_empty() {
        return Ok(());
    }

    queue_style_to(writer, no_color, fg, bg, attrs)?;
    queue!(writer, Print(text))?;
    queue!(writer, SetAttribute(Attribute::Reset))?;
    Ok(())
}

fn emit_regions_to<W: Write>(
    writer: &mut W,
    no_color: bool,
    regions: &[DirtyRegion],
    screen: &VirtualScreen,
) -> BackendResult<()> {
    let mut sorted = expand_to_full_rows(regions, screen);
    sorted.sort_by(|a, b| a.row.cmp(&b.row).then(a.start_col.cmp(&b.start_col)));

    let merged = merge_adjacent(&sorted);

    let mut current_row: Option<u16> = None;
    let mut current_col: Option<u16> = None;

    for region in &merged {
        if current_row != Some(region.row) || current_col != Some(region.start_col) {
            queue!(writer, MoveTo(region.start_col, region.row))?;
            current_row = Some(region.row);
        }

        let mut col = region.start_col;
        while col < region.end_col {
            let cell = screen.cell_at(col, region.row);
            if cell.phantom {
                col += 1;
                continue;
            }

            let mut run_text = String::new();
            run_text.push(cell.ch);
            let mut run_end = col + 1;
            while run_end < region.end_col {
                let next = screen.cell_at(run_end, region.row);
                if next.phantom {
                    run_end += 1;
                    continue;
                }
                if next.fg != cell.fg || next.bg != cell.bg || next.attrs != cell.attrs {
                    break;
                }
                run_text.push(next.ch);
                run_end += 1;
            }

            write_run_to(writer, no_color, &run_text, &cell.fg, &cell.bg, &cell.attrs)?;
            col = run_end;
        }
        current_col = Some(region.end_col);
    }

    Ok(())
}

fn expand_to_full_rows(regions: &[DirtyRegion], screen: &VirtualScreen) -> Vec<DirtyRegion> {
    let mut rows: Vec<u16> = regions
        .iter()
        .filter(|region| region.row < screen.rows())
        .map(|region| region.row)
        .collect();
    rows.sort_unstable();
    rows.dedup();

    rows.into_iter()
        .map(|row| DirtyRegion {
            row,
            start_col: 0,
            end_col: screen.cols(),
        })
        .collect()
}

#[cfg(any(test, feature = "simulated"))]
pub fn encode_regions_for_testing(
    regions: &[DirtyRegion],
    screen: &VirtualScreen,
    no_color: bool,
) -> BackendResult<Vec<u8>> {
    let mut output = Vec::new();
    emit_regions_to(&mut output, no_color, regions, screen)?;
    Ok(output)
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

#[cfg(test)]
mod tests {
    use super::*;
    use arbor_tui_domain::cell::Cell;

    #[test]
    fn emit_sets_foreground_and_background_for_each_visible_cell() {
        let mut screen = VirtualScreen::new(2, 1);
        *screen.cell_at_mut(0, 0).unwrap() = Cell {
            ch: 'A',
            fg: AnsiColor::from_palette(2),
            bg: AnsiColor::from_palette(3),
            ..Default::default()
        };
        *screen.cell_at_mut(1, 0).unwrap() = Cell {
            ch: 'B',
            fg: AnsiColor::from_palette(4),
            bg: AnsiColor::from_palette(5),
            ..Default::default()
        };
        let regions = [DirtyRegion {
            row: 0,
            start_col: 0,
            end_col: 2,
        }];
        let mut out = Vec::new();

        emit_regions_to(&mut out, false, &regions, &screen).unwrap();
        let ansi = String::from_utf8_lossy(&out);

        assert!(ansi.contains("\x1b[38;5;2m"));
        assert!(ansi.contains("\x1b[48;5;3m"));
        assert!(ansi.contains("\x1b[38;5;4m"));
        assert!(ansi.contains("\x1b[48;5;5m"));
    }

    #[test]
    fn emit_partial_region_does_not_clear_rest_of_line_with_terminal_default() {
        let mut screen = VirtualScreen::new(6, 1);
        *screen.cell_at_mut(2, 0).unwrap() = Cell {
            ch: 'X',
            fg: AnsiColor::from_palette(10),
            bg: AnsiColor::from_palette(11),
            ..Default::default()
        };
        let regions = [DirtyRegion {
            row: 0,
            start_col: 2,
            end_col: 3,
        }];
        let mut out = Vec::new();

        emit_regions_to(&mut out, false, &regions, &screen).unwrap();
        let ansi = String::from_utf8_lossy(&out);

        assert!(!ansi.contains("\x1b[K"));
        assert!(!ansi.contains("\x1b[0K"));
        assert!(ansi.contains("\x1b[48;5;11m"));
    }

    #[test]
    fn emit_blank_background_run_writes_styled_spaces_without_line_clear() {
        let mut screen = VirtualScreen::new(4, 1);
        for col in 0..4 {
            *screen.cell_at_mut(col, 0).unwrap() = Cell {
                ch: ' ',
                fg: AnsiColor::from_palette(8),
                bg: AnsiColor::from_palette(15),
                ..Default::default()
            };
        }
        let regions = [DirtyRegion {
            row: 0,
            start_col: 0,
            end_col: 4,
        }];
        let mut out = Vec::new();

        emit_regions_to(&mut out, false, &regions, &screen).unwrap();
        let ansi = String::from_utf8_lossy(&out);

        assert!(ansi.contains("\x1b[48;5;15m"));
        assert!(ansi.contains("    "));
        assert_eq!(ansi.matches("\x1b[0m").count(), 1);
        assert!(!ansi.contains("\x1b[K"));
    }

    #[test]
    fn emit_expands_dirty_cell_to_full_row_background_repaint() {
        let mut screen = VirtualScreen::new(5, 1);
        for col in 0..5 {
            *screen.cell_at_mut(col, 0).unwrap() = Cell {
                ch: ' ',
                fg: AnsiColor::from_palette(8),
                bg: AnsiColor::from_palette(15),
                ..Default::default()
            };
        }
        screen.cell_at_mut(2, 0).unwrap().ch = 'X';
        let regions = [DirtyRegion {
            row: 0,
            start_col: 2,
            end_col: 3,
        }];
        let mut out = Vec::new();

        emit_regions_to(&mut out, false, &regions, &screen).unwrap();
        let ansi = String::from_utf8_lossy(&out);

        assert!(ansi.contains("\x1b[1;1H"));
        assert!(ansi.contains("  X  "));
        assert!(!ansi.contains("\x1b[K"));
        assert!(!ansi.contains("\x1b[X"));
    }
}
