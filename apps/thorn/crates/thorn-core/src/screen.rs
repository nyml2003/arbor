use crate::{layout_tree, lower_element, paint_tree, Element, PaintPrimitive, Size};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
}

impl Default for Cell {
    fn default() -> Self {
        Self { ch: ' ' }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Screen {
    pub size: Size,
    pub cells: Vec<Cell>,
}

impl Screen {
    pub fn new(size: Size) -> Self {
        Self {
            size,
            cells: vec![Cell::default(); usize::from(size.width) * usize::from(size.height)],
        }
    }

    pub fn write_text(&mut self, x: u16, y: u16, text: &str) {
        if y >= self.size.height {
            return;
        }

        for (offset, ch) in text.chars().enumerate() {
            let x = x.saturating_add(offset as u16);
            if x >= self.size.width {
                break;
            }
            let index = usize::from(y) * usize::from(self.size.width) + usize::from(x);
            if let Some(cell) = self.cells.get_mut(index) {
                cell.ch = ch;
            }
        }
    }

    pub fn apply(&mut self, paint: &[PaintPrimitive]) {
        for primitive in paint {
            match primitive {
                PaintPrimitive::TextRun { x, y, text } => self.write_text(*x, *y, text),
            }
        }
    }

    pub fn to_plain_text(&self) -> String {
        let width = usize::from(self.size.width);
        if width == 0 {
            return String::new();
        }
        let mut lines = Vec::with_capacity(usize::from(self.size.height));
        for row in self.cells.chunks(width) {
            let line = row.iter().map(|cell| cell.ch).collect::<String>();
            lines.push(line.trim_end().to_string());
        }
        lines.join("\n")
    }

    pub fn diff(&self, next: &Self) -> ScreenPatch {
        diff_screens(self, next)
    }

    pub fn full_patch(&self) -> ScreenPatch {
        ScreenPatch {
            size: self.size,
            full: true,
            cells: screen_cells_as_patches(self),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellPatch {
    pub x: u16,
    pub y: u16,
    pub cell: Cell,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenPatch {
    pub size: Size,
    pub full: bool,
    pub cells: Vec<CellPatch>,
}

pub fn diff_screens(previous: &Screen, next: &Screen) -> ScreenPatch {
    let full = previous.size != next.size;
    let cells = if full {
        screen_cells_as_patches(next)
    } else {
        diff_same_size_screens(previous, next)
    };

    ScreenPatch {
        size: next.size,
        full,
        cells,
    }
}

fn screen_cells_as_patches(screen: &Screen) -> Vec<CellPatch> {
    let width = usize::from(screen.size.width);
    if width == 0 {
        return Vec::new();
    }
    screen
        .cells
        .iter()
        .enumerate()
        .map(|(index, cell)| CellPatch {
            x: (index % width) as u16,
            y: (index / width) as u16,
            cell: *cell,
        })
        .collect()
}

fn diff_same_size_screens(previous: &Screen, next: &Screen) -> Vec<CellPatch> {
    let width = usize::from(next.size.width);
    if width == 0 {
        return Vec::new();
    }
    previous
        .cells
        .iter()
        .zip(&next.cells)
        .enumerate()
        .filter_map(|(index, (previous, next))| {
            (previous != next).then_some(CellPatch {
                x: (index % width) as u16,
                y: (index / width) as u16,
                cell: *next,
            })
        })
        .collect()
}

pub fn render_to_screen<Action>(element: &Element<Action>, size: Size) -> Screen {
    let host = lower_element(element);
    let layout = layout_tree(&host, size);
    let paint = paint_tree(&host, &layout);
    let mut screen = Screen::new(size);
    screen.apply(&paint);
    screen
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{row, text};

    #[test]
    fn paint_text_run_writes_cells() {
        let mut screen = Screen::new(Size::new(10, 2));
        screen.apply(&[PaintPrimitive::TextRun {
            x: 0,
            y: 0,
            text: "hello".to_string(),
        }]);

        assert!(screen.to_plain_text().contains("hello"));
    }

    #[test]
    fn screen_plain_text_contains_written_text() {
        let screen = render_to_screen(&text::<()>("hello"), Size::new(10, 2));

        assert!(screen.to_plain_text().contains("hello"));
    }

    #[test]
    fn row_render_writes_text_on_one_line() {
        let screen = render_to_screen(&row((text::<()>("a"), text::<()>("bb"))), Size::new(10, 2));

        assert!(screen.to_plain_text().contains("abb"));
    }

    #[test]
    fn text_render_is_clipped_to_layout_width() {
        let screen = render_to_screen(&text::<()>("hello"), Size::new(3, 1));

        assert_eq!(screen.to_plain_text(), "hel");
    }

    #[test]
    fn screen_diff_reports_changed_cells() {
        let mut previous = Screen::new(Size::new(3, 1));
        previous.write_text(0, 0, "abc");
        let mut next = Screen::new(Size::new(3, 1));
        next.write_text(0, 0, "axc");

        let patch = previous.diff(&next);

        assert!(!patch.full);
        assert_eq!(
            patch.cells,
            vec![CellPatch {
                x: 1,
                y: 0,
                cell: Cell { ch: 'x' },
            }]
        );
    }

    #[test]
    fn screen_diff_reports_full_patch_on_resize() {
        let previous = Screen::new(Size::new(2, 1));
        let mut next = Screen::new(Size::new(3, 1));
        next.write_text(0, 0, "abc");

        let patch = previous.diff(&next);

        assert!(patch.full);
        assert_eq!(patch.size, Size::new(3, 1));
        assert_eq!(patch.cells.len(), 3);
    }

    #[test]
    fn full_patch_contains_all_cells() {
        let mut screen = Screen::new(Size::new(2, 1));
        screen.write_text(0, 0, "ab");

        let patch = screen.full_patch();

        assert!(patch.full);
        assert_eq!(patch.cells.len(), 2);
    }
}
