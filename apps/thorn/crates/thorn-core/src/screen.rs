use crate::{
    layout_tree, lower_element, paint_tree, Element, HostNode, LayoutNode, PaintAttrs, PaintColor,
    PaintPrimitive, PaintStyle, Rect, Size,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub foreground: Option<Color>,
    pub background: Option<Color>,
    pub attrs: CellAttrs,
    pub wide: WideCell,
}

impl Default for Cell {
    fn default() -> Self {
        Self::new(' ')
    }
}

impl Cell {
    pub const fn new(ch: char) -> Self {
        Self {
            ch,
            foreground: None,
            background: None,
            attrs: CellAttrs::empty(),
            wide: WideCell::Normal,
        }
    }

    pub const fn with_background(mut self, background: Color) -> Self {
        self.background = Some(background);
        self
    }

    pub fn with_style(mut self, style: PaintStyle) -> Self {
        self.foreground = style.foreground.map(Color::from);
        self.background = style.background.map(Color::from);
        self.attrs = CellAttrs::from(style.attrs);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

impl From<PaintColor> for Color {
    fn from(value: PaintColor) -> Self {
        match value {
            PaintColor::Default => Self::Default,
            PaintColor::Indexed(index) => Self::Indexed(index),
            PaintColor::Rgb(r, g, b) => Self::Rgb(r, g, b),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellAttrs {
    bits: u8,
}

impl CellAttrs {
    pub const BOLD: Self = Self { bits: 0b0000_0001 };
    pub const UNDERLINE: Self = Self { bits: 0b0000_0010 };
    pub const REVERSED: Self = Self { bits: 0b0000_0100 };

    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.bits & other.bits) == other.bits
    }
}

impl Default for CellAttrs {
    fn default() -> Self {
        Self::empty()
    }
}

impl From<PaintAttrs> for CellAttrs {
    fn from(value: PaintAttrs) -> Self {
        let mut attrs = Self::empty();
        if value.contains(PaintAttrs::BOLD) {
            attrs.bits |= Self::BOLD.bits;
        }
        if value.contains(PaintAttrs::UNDERLINE) {
            attrs.bits |= Self::UNDERLINE.bits;
        }
        if value.contains(PaintAttrs::REVERSED) {
            attrs.bits |= Self::REVERSED.bits;
        }
        attrs
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WideCell {
    Normal,
    Continuation,
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
                cell.wide = WideCell::Normal;
            }
        }
    }

    pub fn apply(&mut self, paint: &[PaintPrimitive]) {
        for primitive in paint {
            self.apply_primitive(primitive);
        }
    }

    fn apply_primitive(&mut self, primitive: &PaintPrimitive) {
        match primitive {
            PaintPrimitive::FillRect { rect, style } => self.fill_rect(*rect, *style),
            PaintPrimitive::TextRun { x, y, text } => self.write_text(*x, *y, text),
            PaintPrimitive::Border { rect, style } => self.draw_border(*rect, *style),
            PaintPrimitive::Cursor { .. } => {}
            PaintPrimitive::Clip { rect, children } => {
                for child in children {
                    self.apply_clipped(*rect, child);
                }
            }
            PaintPrimitive::Layer { children, .. } => {
                for child in sorted_layer_children(children) {
                    self.apply_primitive(child);
                }
            }
        }
    }

    fn apply_clipped(&mut self, clip: Rect, primitive: &PaintPrimitive) {
        match primitive {
            PaintPrimitive::FillRect { rect, style } => {
                if let Some(rect) = intersect_rects(*rect, clip) {
                    self.fill_rect(rect, *style);
                }
            }
            PaintPrimitive::TextRun { x, y, text } => {
                if *y < clip.y
                    || *y >= clip.y.saturating_add(clip.height)
                    || *x >= clip.x.saturating_add(clip.width)
                {
                    return;
                }
                let skip = clip.x.saturating_sub(*x) as usize;
                let take = clip
                    .x
                    .saturating_add(clip.width)
                    .saturating_sub((*x).max(clip.x)) as usize;
                let clipped = text.chars().skip(skip).take(take).collect::<String>();
                self.write_text((*x).max(clip.x), *y, &clipped);
            }
            PaintPrimitive::Border { rect, style } => {
                self.draw_border_clipped(*rect, *style, clip);
            }
            PaintPrimitive::Cursor { .. } => {}
            PaintPrimitive::Clip { rect, children } => {
                if let Some(rect) = intersect_rects(*rect, clip) {
                    for child in children {
                        self.apply_clipped(rect, child);
                    }
                }
            }
            PaintPrimitive::Layer { children, .. } => {
                for child in sorted_layer_children(children) {
                    self.apply_clipped(clip, child);
                }
            }
        }
    }

    pub fn fill_rect(&mut self, rect: Rect, style: PaintStyle) {
        let bottom = rect.y.saturating_add(rect.height).min(self.size.height);
        let right = rect.x.saturating_add(rect.width).min(self.size.width);
        for y in rect.y..bottom {
            for x in rect.x..right {
                let index = usize::from(y) * usize::from(self.size.width) + usize::from(x);
                if let Some(target) = self.cells.get_mut(index) {
                    *target = target.with_style(style);
                }
            }
        }
    }

    fn draw_border(&mut self, rect: Rect, style: PaintStyle) {
        if rect.width == 0 || rect.height == 0 {
            return;
        }
        let right = rect.x.saturating_add(rect.width).saturating_sub(1);
        let bottom = rect.y.saturating_add(rect.height).saturating_sub(1);
        for x in rect.x..=right.min(self.size.width.saturating_sub(1)) {
            self.set_styled_char(x, rect.y, '#', style);
            self.set_styled_char(x, bottom, '#', style);
        }
        for y in rect.y..=bottom.min(self.size.height.saturating_sub(1)) {
            self.set_styled_char(rect.x, y, '#', style);
            self.set_styled_char(right, y, '#', style);
        }
    }

    fn draw_border_clipped(&mut self, rect: Rect, style: PaintStyle, clip: Rect) {
        if rect.width == 0 || rect.height == 0 {
            return;
        }
        let right = rect.x.saturating_add(rect.width).saturating_sub(1);
        let bottom = rect.y.saturating_add(rect.height).saturating_sub(1);
        for x in rect.x..=right {
            if point_in_rect(x, rect.y, clip) {
                self.set_styled_char(x, rect.y, '#', style);
            }
            if point_in_rect(x, bottom, clip) {
                self.set_styled_char(x, bottom, '#', style);
            }
        }
        for y in rect.y..=bottom {
            if point_in_rect(rect.x, y, clip) {
                self.set_styled_char(rect.x, y, '#', style);
            }
            if point_in_rect(right, y, clip) {
                self.set_styled_char(right, y, '#', style);
            }
        }
    }

    fn set_styled_char(&mut self, x: u16, y: u16, ch: char, style: PaintStyle) {
        if x >= self.size.width || y >= self.size.height {
            return;
        }
        let index = usize::from(y) * usize::from(self.size.width) + usize::from(x);
        if let Some(target) = self.cells.get_mut(index) {
            *target = Cell::new(ch).with_style(style);
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
            regions: full_regions(self.size),
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
    pub regions: Vec<DirtyRegion>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirtyRegion {
    pub rect: Rect,
}

pub fn diff_screens(previous: &Screen, next: &Screen) -> ScreenPatch {
    let full = previous.size != next.size;
    let cells = if full {
        screen_cells_as_patches(next)
    } else {
        diff_same_size_screens(previous, next)
    };

    let regions = if full {
        full_regions(next.size)
    } else {
        dirty_regions_from_cells(next.size, &cells)
    };

    ScreenPatch {
        size: next.size,
        full,
        cells,
        regions,
    }
}

fn full_regions(size: Size) -> Vec<DirtyRegion> {
    (size.width > 0 && size.height > 0)
        .then_some(DirtyRegion {
            rect: Rect::new(0, 0, size.width, size.height),
        })
        .into_iter()
        .collect()
}

fn dirty_regions_from_cells(size: Size, cells: &[CellPatch]) -> Vec<DirtyRegion> {
    if size.width == 0 || cells.is_empty() {
        return Vec::new();
    }
    let mut regions = Vec::new();
    let mut sorted = cells.to_vec();
    sorted.sort_by_key(|cell| (cell.y, cell.x));
    let mut iter = sorted.into_iter();
    let Some(first) = iter.next() else {
        return regions;
    };
    let mut current_y = first.y;
    let mut start_x = first.x;
    let mut last_x = first.x;
    for cell in iter {
        if cell.y == current_y && cell.x == last_x.saturating_add(1) {
            last_x = cell.x;
            continue;
        }
        regions.push(DirtyRegion {
            rect: Rect::new(start_x, current_y, last_x.saturating_sub(start_x) + 1, 1),
        });
        current_y = cell.y;
        start_x = cell.x;
        last_x = cell.x;
    }
    regions.push(DirtyRegion {
        rect: Rect::new(start_x, current_y, last_x.saturating_sub(start_x) + 1, 1),
    });
    regions
}

fn intersect_rects(a: Rect, b: Rect) -> Option<Rect> {
    let x1 = a.x.max(b.x);
    let y1 = a.y.max(b.y);
    let x2 = a.x.saturating_add(a.width).min(b.x.saturating_add(b.width));
    let y2 =
        a.y.saturating_add(a.height)
            .min(b.y.saturating_add(b.height));
    (x2 > x1 && y2 > y1).then(|| Rect::new(x1, y1, x2 - x1, y2 - y1))
}

fn point_in_rect(x: u16, y: u16, rect: Rect) -> bool {
    x >= rect.x
        && y >= rect.y
        && x < rect.x.saturating_add(rect.width)
        && y < rect.y.saturating_add(rect.height)
}

fn sorted_layer_children(children: &[PaintPrimitive]) -> Vec<&PaintPrimitive> {
    let mut children = children.iter().collect::<Vec<_>>();
    children.sort_by_key(|primitive| match primitive {
        PaintPrimitive::Layer { z_index, .. } => *z_index,
        _ => 0,
    });
    children
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedFrame<Action> {
    pub host: HostNode<Action>,
    pub layout: Vec<LayoutNode>,
    pub paint: Vec<PaintPrimitive>,
    pub screen: Screen,
}

pub fn render_pipeline<Action>(element: &Element<Action>, size: Size) -> RenderedFrame<Action> {
    let host = lower_element(element);
    let layout = layout_tree(&host, size);
    let paint = paint_tree(&host, &layout);
    let mut screen = Screen::new(size);
    screen.apply(&paint);
    RenderedFrame {
        host,
        layout,
        paint,
        screen,
    }
}

pub fn render_to_screen<Action>(element: &Element<Action>, size: Size) -> Screen {
    render_pipeline(element, size).screen
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
    fn render_pipeline_exposes_intermediate_outputs() {
        let frame = render_pipeline(&text::<()>("hello"), Size::new(10, 2));

        assert_eq!(frame.layout.len(), 1);
        assert_eq!(frame.paint.len(), 1);
        assert!(frame.screen.to_plain_text().contains("hello"));
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
                cell: Cell::new('x'),
            }]
        );
        assert_eq!(
            patch.regions,
            vec![DirtyRegion {
                rect: Rect::new(1, 0, 1, 1),
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
        assert_eq!(
            patch.regions,
            vec![DirtyRegion {
                rect: Rect::new(0, 0, 3, 1),
            }]
        );
    }

    #[test]
    fn full_patch_contains_all_cells() {
        let mut screen = Screen::new(Size::new(2, 1));
        screen.write_text(0, 0, "ab");

        let patch = screen.full_patch();

        assert!(patch.full);
        assert_eq!(patch.cells.len(), 2);
    }

    #[test]
    fn fill_rect_paints_cells_with_style() {
        let mut screen = Screen::new(Size::new(3, 2));

        screen.apply(&[PaintPrimitive::FillRect {
            rect: Rect::new(1, 0, 2, 2),
            style: PaintStyle {
                background: Some(PaintColor::Indexed(4)),
                ..PaintStyle::default()
            },
        }]);

        assert_eq!(screen.cells[1].background, Some(Color::Indexed(4)));
        assert_eq!(screen.cells[2].background, Some(Color::Indexed(4)));
        assert_eq!(screen.cells[4].background, Some(Color::Indexed(4)));
        assert_eq!(screen.cells[5].background, Some(Color::Indexed(4)));
    }

    #[test]
    fn text_run_preserves_existing_background() {
        let mut screen = Screen::new(Size::new(3, 1));
        screen.apply(&[PaintPrimitive::FillRect {
            rect: Rect::new(0, 0, 3, 1),
            style: PaintStyle {
                background: Some(PaintColor::Indexed(4)),
                ..PaintStyle::default()
            },
        }]);

        screen.apply(&[PaintPrimitive::TextRun {
            x: 1,
            y: 0,
            text: "x".to_string(),
        }]);

        assert_eq!(screen.cells[1].ch, 'x');
        assert_eq!(screen.cells[1].background, Some(Color::Indexed(4)));
    }

    #[test]
    fn clipped_border_keeps_original_edges_not_clip_edges() {
        let mut screen = Screen::new(Size::new(5, 3));

        screen.apply(&[PaintPrimitive::Clip {
            rect: Rect::new(1, 1, 3, 1),
            children: vec![PaintPrimitive::Border {
                rect: Rect::new(0, 0, 5, 3),
                style: PaintStyle::default(),
            }],
        }]);

        assert!(!screen.to_plain_text().contains('#'));
    }

    #[test]
    fn layer_children_are_applied_by_z_index() {
        let mut screen = Screen::new(Size::new(1, 1));

        screen.apply(&[PaintPrimitive::Layer {
            z_index: 0,
            children: vec![
                PaintPrimitive::Layer {
                    z_index: 10,
                    children: vec![PaintPrimitive::TextRun {
                        x: 0,
                        y: 0,
                        text: "b".to_string(),
                    }],
                },
                PaintPrimitive::Layer {
                    z_index: 1,
                    children: vec![PaintPrimitive::TextRun {
                        x: 0,
                        y: 0,
                        text: "a".to_string(),
                    }],
                },
            ],
        }]);

        assert_eq!(screen.to_plain_text(), "b");
    }

    #[test]
    fn dirty_patch_merges_adjacent_cells_into_regions() {
        let mut previous = Screen::new(Size::new(5, 1));
        previous.write_text(0, 0, "abcde");
        let mut next = Screen::new(Size::new(5, 1));
        next.write_text(0, 0, "abXYe");

        let patch = previous.diff(&next);

        assert_eq!(patch.cells.len(), 2);
        assert_eq!(
            patch.regions,
            vec![DirtyRegion {
                rect: Rect::new(2, 0, 2, 1),
            }]
        );
    }
}
