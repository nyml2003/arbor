use std::collections::HashMap;

use crate::layout::{is_wide, layout_tree, Rect};
use crate::theme::{Color, ColorSource, Theme, Token};
use crate::view::{NodeId, NodeKind, PrimitiveNode, View};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
    pub wide_continuation: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: Color::Palette(7),
            bg: Color::Palette(0),
            wide_continuation: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Screen {
    width: u16,
    height: u16,
    cells: Vec<Cell>,
}

impl Screen {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            cells: vec![Cell::default(); width as usize * height as usize],
        }
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn get(&self, x: u16, y: u16) -> Cell {
        self.index(x, y)
            .and_then(|index| self.cells.get(index).copied())
            .unwrap_or_default()
    }

    pub fn set(&mut self, x: u16, y: u16, cell: Cell) {
        if let Some(index) = self.index(x, y) {
            if let Some(target) = self.cells.get_mut(index) {
                *target = cell;
            }
        }
    }

    pub fn fill_rect(&mut self, rect: Rect, cell: Cell) {
        let y_end = rect.y.saturating_add(rect.h).min(self.height);
        let x_end = rect.x.saturating_add(rect.w).min(self.width);
        for y in rect.y..y_end {
            for x in rect.x..x_end {
                self.set(x, y, cell);
            }
        }
    }

    pub fn write_str(&mut self, x: u16, y: u16, text: &str, fg: Color, bg: Color) {
        if y >= self.height {
            return;
        }

        let mut cursor = x;
        for ch in text.chars() {
            if cursor >= self.width {
                break;
            }

            let wide = is_wide(ch);
            self.set(
                cursor,
                y,
                Cell {
                    ch,
                    fg,
                    bg,
                    wide_continuation: false,
                },
            );
            cursor = cursor.saturating_add(1);
            if wide && cursor < self.width {
                self.set(
                    cursor,
                    y,
                    Cell {
                        ch: ' ',
                        fg,
                        bg,
                        wide_continuation: true,
                    },
                );
                cursor = cursor.saturating_add(1);
            }
        }
    }

    pub fn contains_text(&self, needle: &str) -> bool {
        (0..self.height).any(|y| {
            let row: String = (0..self.width).map(|x| self.get(x, y).ch).collect();
            row.contains(needle)
        })
    }

    pub fn to_plain_text(&self) -> String {
        let mut out = String::new();
        for y in 0..self.height {
            if y > 0 {
                out.push('\n');
            }

            let row: String = (0..self.width).map(|x| self.get(x, y).ch).collect();
            out.push_str(row.trim_end());
        }
        out
    }

    fn index(&self, x: u16, y: u16) -> Option<usize> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some(y as usize * self.width as usize + x as usize)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DirtyRegion {
    pub rect: Rect,
}

pub fn diff(old: &Screen, new: &Screen) -> Vec<DirtyRegion> {
    if old.width != new.width || old.height != new.height {
        return vec![DirtyRegion {
            rect: Rect::new(0, 0, new.width, new.height),
        }];
    }

    let mut regions = Vec::new();
    for y in 0..new.height {
        let mut start: Option<u16> = None;
        for x in 0..new.width {
            let dirty = old.get(x, y) != new.get(x, y);
            match (start, dirty) {
                (None, true) => start = Some(x),
                (Some(s), false) => {
                    regions.push(DirtyRegion {
                        rect: Rect::new(s, y, x - s, 1),
                    });
                    start = None;
                }
                _ => {}
            }
        }
        if let Some(s) = start {
            regions.push(DirtyRegion {
                rect: Rect::new(s, y, new.width - s, 1),
            });
        }
    }
    regions
}

pub fn render_tree<Action>(
    root: &View<Action>,
    width: u16,
    height: u16,
    theme: &Theme,
) -> (Screen, HashMap<NodeId, crate::layout::LayoutInfo>) {
    let layout = layout_tree(root, Rect::new(0, 0, width, height));
    let mut screen = Screen::new(width, height);
    let bg = theme.resolve(Token::Surface);
    screen.fill_rect(
        Rect::new(0, 0, width, height),
        Cell {
            bg,
            fg: theme.resolve(Token::Text),
            ..Cell::default()
        },
    );
    render_node(root.node(), &layout, theme, &mut screen, bg);
    (screen, layout)
}

fn render_node<Action>(
    node: &PrimitiveNode<Action>,
    layout: &HashMap<NodeId, crate::layout::LayoutInfo>,
    theme: &Theme,
    screen: &mut Screen,
    inherited_bg: Color,
) {
    let Some(info) = layout.get(&node.id()) else {
        return;
    };

    let fg = resolve_style(theme, node.style().fg, Token::Text);
    let bg = resolve_bg(theme, node.style().bg, inherited_bg);
    screen.fill_rect(
        info.rect,
        Cell {
            fg,
            bg,
            ..Cell::default()
        },
    );

    match node.kind() {
        NodeKind::Panel => {
            if node.style().border.is_some() {
                let border = resolve_style(theme, node.style().border, Token::Border);
                draw_border(screen, info.rect, bg, border);
            }
        }
        NodeKind::Text => {
            if let Some(text) = node.text() {
                screen.write_str(info.content_rect.x, info.content_rect.y, &text, fg, bg);
            }
        }
        NodeKind::Button => {
            if let Some(text) = node.text() {
                let label = format!("[{}]", text);
                screen.write_str(info.content_rect.x, info.content_rect.y, &label, fg, bg);
            }
        }
        NodeKind::Row | NodeKind::Col => {}
    }

    for child in node.children() {
        render_node(child, layout, theme, screen, bg);
    }
}

fn resolve_style(theme: &Theme, source: Option<ColorSource>, fallback: Token) -> Color {
    theme.resolve(source.unwrap_or(ColorSource::Token(fallback)))
}

fn resolve_bg(theme: &Theme, source: Option<ColorSource>, inherited_bg: Color) -> Color {
    source
        .map(|source| theme.resolve(source))
        .unwrap_or(inherited_bg)
}

fn draw_border(screen: &mut Screen, rect: Rect, bg: Color, border: Color) {
    if rect.w == 0 || rect.h == 0 {
        return;
    }

    let border_cell = |ch| Cell {
        ch,
        fg: border,
        bg,
        wide_continuation: false,
    };

    let x_end = rect.x.saturating_add(rect.w).saturating_sub(1);
    let y_end = rect.y.saturating_add(rect.h).saturating_sub(1);

    screen.set(rect.x, rect.y, border_cell('+'));
    screen.set(x_end, rect.y, border_cell('+'));
    screen.set(rect.x, y_end, border_cell('+'));
    screen.set(x_end, y_end, border_cell('+'));

    for x in rect.x.saturating_add(1)..x_end {
        screen.set(x, rect.y, border_cell('-'));
        screen.set(x, y_end, border_cell('-'));
    }
    for y in rect.y.saturating_add(1)..y_end {
        screen.set(rect.x, y, border_cell('|'));
        screen.set(x_end, y, border_cell('|'));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_fill_and_write_clip_to_bounds() {
        let mut screen = Screen::new(4, 2);
        screen.fill_rect(
            Rect::new(2, 0, 10, 1),
            Cell {
                bg: Color::Palette(4),
                ..Cell::default()
            },
        );
        screen.write_str(3, 1, "abc", Color::Palette(1), Color::Palette(2));

        assert_eq!(screen.get(2, 0).bg, Color::Palette(4));
        assert_eq!(screen.get(3, 1).ch, 'a');
        assert_eq!(screen.get(0, 1).ch, ' ');
    }

    #[test]
    fn wide_char_marks_continuation_cell() {
        let mut screen = Screen::new(4, 1);
        screen.write_str(0, 0, "界", Color::Palette(1), Color::Palette(2));

        assert_eq!(screen.get(0, 0).ch, '界');
        assert!(screen.get(1, 0).wide_continuation);
    }

    #[test]
    fn diff_detects_resize_single_cell_and_identical_screens() {
        let old = Screen::new(2, 1);
        assert!(diff(&old, &old).is_empty());

        let mut new = old.clone();
        new.set(
            1,
            0,
            Cell {
                ch: 'x',
                ..Cell::default()
            },
        );
        assert_eq!(
            diff(&old, &new),
            vec![DirtyRegion {
                rect: Rect::new(1, 0, 1, 1)
            }]
        );

        assert_eq!(
            diff(&old, &Screen::new(3, 1)),
            vec![DirtyRegion {
                rect: Rect::new(0, 0, 3, 1)
            }]
        );
    }

    #[test]
    fn plain_text_dump_trims_trailing_spaces_per_row() {
        let mut screen = Screen::new(4, 2);
        screen.write_str(0, 0, "hi", Color::Palette(1), Color::Palette(2));
        screen.write_str(1, 1, "x", Color::Palette(1), Color::Palette(2));

        assert_eq!(screen.to_plain_text(), "hi\n x");
    }
}
