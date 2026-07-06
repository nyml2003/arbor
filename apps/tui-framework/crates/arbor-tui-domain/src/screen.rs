// VirtualScreen — the character grid buffer. Row-major flat array of Cell.

use crate::cell::{AnsiColor, Attrs, Cell, Span};
use crate::layout::Rect;

/// A character grid of `cols × rows` cells, stored as a row-major flat vector.
///
/// All rendering happens into a VirtualScreen. The diff algorithm compares two
/// VirtualScreens and the backend emits only the changed regions.
#[derive(Clone, PartialEq, Eq, Debug)]
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
        self.cells.get(idx).copied().unwrap_or_default()
    }

    /// Borrow a cell at (col, row). Returns None for out-of-bounds access.
    pub fn cell_at_ref(&self, col: u16, row: u16) -> Option<&Cell> {
        if col >= self.cols || row >= self.rows {
            return None;
        }
        let idx = row as usize * self.cols as usize + col as usize;
        self.cells.get(idx)
    }

    pub(crate) fn row_cells(&self, row: u16) -> Option<&[Cell]> {
        if row >= self.rows {
            return None;
        }
        let row_len = self.cols as usize;
        let start = row as usize * row_len;
        Some(&self.cells[start..start + row_len])
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
        let x0 = rect.x.min(self.cols);
        let y0 = rect.y.min(self.rows);
        let x1 = rect.x.saturating_add(rect.w).min(self.cols);
        let y1 = rect.y.saturating_add(rect.h).min(self.rows);
        if x0 >= x1 || y0 >= y1 {
            return;
        }

        let row_len = self.cols as usize;
        for row in y0..y1 {
            let start = row as usize * row_len + x0 as usize;
            let end = row as usize * row_len + x1 as usize;
            self.cells[start..end].fill(*cell);
        }
    }

    /// Blit (copy) a smaller VirtualScreen into a region of this one.
    pub fn blit(&mut self, dest: Rect, source: &VirtualScreen) {
        if dest.x >= self.cols || dest.y >= self.rows {
            return;
        }

        let copy_cols = source.cols.min(self.cols - dest.x) as usize;
        let copy_rows = source.rows.min(self.rows - dest.y);
        let source_row_len = source.cols as usize;
        let dest_row_len = self.cols as usize;

        for row in 0..copy_rows {
            let source_start = row as usize * source_row_len;
            let dest_start = (dest.y + row) as usize * dest_row_len + dest.x as usize;
            self.cells[dest_start..dest_start + copy_cols]
                .copy_from_slice(&source.cells[source_start..source_start + copy_cols]);
        }
    }

    /// Blit a rectangular source region into this screen.
    pub fn blit_region(&mut self, dest: Rect, source: &VirtualScreen, source_origin: (u16, u16)) {
        if dest.x >= self.cols || dest.y >= self.rows {
            return;
        }

        let (source_x, source_y) = source_origin;
        if source_x >= source.cols || source_y >= source.rows {
            return;
        }

        let copy_cols = dest.w.min(source.cols - source_x).min(self.cols - dest.x) as usize;
        let copy_rows = dest.h.min(source.rows - source_y).min(self.rows - dest.y);
        if copy_cols == 0 || copy_rows == 0 {
            return;
        }

        let source_row_len = source.cols as usize;
        let dest_row_len = self.cols as usize;
        for row in 0..copy_rows {
            let source_start = (source_y + row) as usize * source_row_len + source_x as usize;
            let dest_start = (dest.y + row) as usize * dest_row_len + dest.x as usize;
            self.cells[dest_start..dest_start + copy_cols]
                .copy_from_slice(&source.cells[source_start..source_start + copy_cols]);
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
        let old_row_len = self.cols as usize;
        let new_row_len = cols as usize;
        let copy_cols_usize = copy_cols as usize;
        for row in 0..copy_rows {
            let old_start = row as usize * old_row_len;
            let new_start = row as usize * new_row_len;
            new_screen.cells[new_start..new_start + copy_cols_usize]
                .copy_from_slice(&self.cells[old_start..old_start + copy_cols_usize]);
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
    fn fill_rect_clips_to_screen_edges() {
        let mut s = VirtualScreen::new(4, 3);
        let fill = Cell {
            ch: 'x',
            bg: AnsiColor::from_palette(4),
            ..Default::default()
        };

        s.fill_rect(Rect::new(2, 1, 5, 5), &fill);

        assert_eq!(s.cell_at(1, 1).ch, ' ');
        assert_eq!(s.cell_at(2, 1).ch, 'x');
        assert_eq!(s.cell_at(3, 1).ch, 'x');
        assert_eq!(s.cell_at(2, 2).ch, 'x');
        assert_eq!(s.cell_at(3, 2).bg, AnsiColor::from_palette(4));
    }

    #[test]
    fn blit_clips_to_destination_edges() {
        let mut dest = VirtualScreen::new(4, 2);
        let mut source = VirtualScreen::new(4, 3);
        source.write_str(
            0,
            0,
            "abcd",
            AnsiColor::from_palette(1),
            AnsiColor::from_palette(2),
            Attrs::default(),
        );

        dest.blit(Rect::new(2, 1, 4, 3), &source);

        assert_eq!(dest.cell_at(1, 1).ch, ' ');
        assert_eq!(dest.cell_at(2, 1).ch, 'a');
        assert_eq!(dest.cell_at(3, 1).ch, 'b');
        assert_eq!(dest.cell_at(2, 1).fg, AnsiColor::from_palette(1));
    }

    #[test]
    fn blit_region_copies_source_window() {
        let mut dest = VirtualScreen::new(4, 2);
        let mut source = VirtualScreen::new(5, 3);
        source.write_str(
            0,
            1,
            "abcde",
            AnsiColor::from_palette(1),
            AnsiColor::from_palette(2),
            Attrs::default(),
        );

        dest.blit_region(Rect::new(1, 0, 3, 1), &source, (2, 1));

        assert_eq!(dest.cell_at(0, 0).ch, ' ');
        assert_eq!(dest.cell_at(1, 0).ch, 'c');
        assert_eq!(dest.cell_at(2, 0).ch, 'd');
        assert_eq!(dest.cell_at(3, 0).ch, 'e');
        assert_eq!(dest.cell_at(1, 0).bg, AnsiColor::from_palette(2));
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
