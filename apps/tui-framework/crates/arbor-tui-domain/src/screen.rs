// VirtualScreen — the character grid buffer. Row-major flat array of Cell.

use crate::cell::{AnsiColor, Attrs, Cell, Span};
use crate::layout::Rect;

/// A character grid of `cols × rows` cells, stored as a row-major flat vector.
///
/// All rendering happens into a VirtualScreen. The diff algorithm compares two
/// VirtualScreens and the backend emits only the changed regions.
#[derive(Clone)]
pub struct VirtualScreen {
    cells: Vec<Cell>,
    cols: u16,
    rows: u16,
}

impl VirtualScreen {
    /// Create a new screen filled with `Cell::default()` (blank spaces).
    pub fn new(cols: u16, rows: u16) -> Self {
        let len = cols as usize * rows as usize;
        Self {
            cells: vec![Cell::default(); len],
            cols,
            rows,
        }
    }

    pub fn cols(&self) -> u16 {
        self.cols
    }
    pub fn rows(&self) -> u16 {
        self.rows
    }

    /// Read a cell at (col, row). Returns `Cell::default()` for out-of-bounds access.
    pub fn cell_at(&self, col: u16, row: u16) -> Cell {
        if col >= self.cols || row >= self.rows {
            return Cell::default();
        }
        let idx = row as usize * self.cols as usize + col as usize;
        self.cells.get(idx).cloned().unwrap_or_default()
    }

    /// Borrow a cell at (col, row). Returns None for out-of-bounds access.
    pub fn cell_at_ref(&self, col: u16, row: u16) -> Option<&Cell> {
        if col >= self.cols || row >= self.rows {
            return None;
        }
        let idx = row as usize * self.cols as usize + col as usize;
        self.cells.get(idx)
    }

    /// Get a mutable reference to a cell. Returns None for out-of-bounds.
    pub fn cell_at_mut(&mut self, col: u16, row: u16) -> Option<&mut Cell> {
        if col >= self.cols || row >= self.rows {
            return None;
        }
        let idx = row as usize * self.cols as usize + col as usize;
        self.cells.get_mut(idx)
    }

    /// Write a string at (col, row). Text is clipped to the available width.
    /// Characters that extend beyond the right edge are silently dropped.
    /// CJK characters occupy 2 columns — the column after a wide char is skipped.
    /// Tab characters must be pre-expanded by the caller.
    pub fn write_str(
        &mut self,
        col: u16,
        row: u16,
        text: &str,
        fg: AnsiColor,
        bg: AnsiColor,
        attrs: Attrs,
    ) {
        if row >= self.rows {
            return;
        }
        let mut c = col;
        for ch in text.chars() {
            let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1) as u16;
            if c + cw > self.cols {
                break; // would overflow right edge
            }
            if let Some(cell) = self.cell_at_mut(c, row) {
                cell.ch = ch;
                cell.fg = fg;
                cell.bg = bg;
                cell.attrs = attrs;
                cell.phantom = false;
            }
            // Mark phantom columns for wide chars (CJK)
            if cw > 1 {
                for offset in 1..cw {
                    if let Some(ghost) = self.cell_at_mut(c + offset, row) {
                        ghost.phantom = true;
                    }
                }
            }
            c += cw;
        }
    }

    /// Write a sequence of styled spans at (col, row).
    /// Each span has its own fg/bg/attrs — inline rich text.
    pub fn write_spans(&mut self, col: u16, row: u16, spans: &[Span]) {
        if row >= self.rows {
            return;
        }
        let mut c = col;
        for span in spans {
            for ch in span.text.chars() {
                let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1) as u16;
                if c + cw > self.cols {
                    break;
                }
                if let Some(cell) = self.cell_at_mut(c, row) {
                    cell.ch = ch;
                    cell.fg = span.fg;
                    cell.bg = span.bg;
                    cell.attrs = span.attrs;
                    cell.phantom = false;
                }
                if cw > 1 {
                    for offset in 1..cw {
                        if let Some(ghost) = self.cell_at_mut(c + offset, row) {
                            ghost.phantom = true;
                        }
                    }
                }
                c += cw;
            }
        }
    }

    /// Fill a rectangular region with a single cell.
    pub fn fill_rect(&mut self, rect: Rect, cell: &Cell) {
        for row in rect.y..rect.y.saturating_add(rect.h) {
            if row >= self.rows {
                break;
            }
            for col in rect.x..rect.x.saturating_add(rect.w) {
                if col >= self.cols {
                    break;
                }
                if let Some(target) = self.cell_at_mut(col, row) {
                    *target = cell.clone();
                }
            }
        }
    }

    /// Blit (copy) a smaller VirtualScreen into a region of this one.
    pub fn blit(&mut self, dest: Rect, source: &VirtualScreen) {
        for row in 0..source.rows {
            let dest_row = dest.y + row;
            if dest_row >= self.rows {
                break;
            }
            for col in 0..source.cols {
                let dest_col = dest.x + col;
                if dest_col >= self.cols {
                    break;
                }
                let src_idx = row as usize * source.cols as usize + col as usize;
                if let Some(dest_cell) = self.cell_at_mut(dest_col, dest_row) {
                    *dest_cell = source.cells[src_idx].clone();
                }
            }
        }
    }

    /// Resize the screen. Newly visible areas are filled with default cells.
    /// Shrinking discards cells outside the new bounds.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        if cols == self.cols && rows == self.rows {
            return;
        }
        let mut new_screen = VirtualScreen::new(cols, rows);
        let copy_cols = self.cols.min(cols);
        let copy_rows = self.rows.min(rows);
        for row in 0..copy_rows {
            for col in 0..copy_cols {
                let old_idx = row as usize * self.cols as usize + col as usize;
                let new_idx = row as usize * cols as usize + col as usize;
                new_screen.cells[new_idx] = self.cells[old_idx].clone();
            }
        }
        *self = new_screen;
    }

    /// Iterate over all cells in row-major order with their (col, row) coordinates.
    pub fn iter_cells(&self) -> impl Iterator<Item = (u16, u16, &Cell)> {
        self.cells.iter().enumerate().map(move |(i, cell)| {
            let col = (i % self.cols as usize) as u16;
            let row = (i / self.cols as usize) as u16;
            (col, row, cell)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_screen_is_blank() {
        let s = VirtualScreen::new(10, 5);
        assert_eq!(s.cols(), 10);
        assert_eq!(s.rows(), 5);
        let c = s.cell_at(0, 0);
        assert_eq!(c.ch, ' ');
    }

    #[test]
    fn oob_read_returns_default() {
        let s = VirtualScreen::new(10, 5);
        let c = s.cell_at(100, 100);
        assert_eq!(c.ch, ' ');
    }

    #[test]
    fn oob_ref_returns_none() {
        let s = VirtualScreen::new(10, 5);
        assert!(s.cell_at_ref(100, 100).is_none());
    }

    #[test]
    fn oob_write_is_noop() {
        let mut s = VirtualScreen::new(10, 5);
        assert!(s.cell_at_mut(100, 100).is_none());
    }

    #[test]
    fn resize_preserves_content() {
        let mut s = VirtualScreen::new(10, 5);
        s.cell_at_mut(0, 0).unwrap().ch = 'X';
        s.resize(20, 10);
        assert_eq!(s.cell_at(0, 0).ch, 'X');
        assert_eq!(s.cell_at(15, 5).ch, ' '); // new area is blank
    }

    #[test]
    fn resize_shrink_discards() {
        let mut s = VirtualScreen::new(10, 5);
        s.cell_at_mut(9, 4).unwrap().ch = 'Z';
        s.resize(3, 2);
        assert_eq!(s.cols(), 3);
        assert_eq!(s.rows(), 2);
        assert_eq!(s.cell_at(0, 0).ch, ' ');
    }
}
