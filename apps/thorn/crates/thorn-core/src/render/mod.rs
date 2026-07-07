use std::collections::HashMap;

use crate::layout::{is_wide, layout_tree, Rect};
use crate::theme::{Color, ColorSource, Theme, Token};
use crate::view::{fuzzy_matches, NodeId, NodeKind, PrimitiveNode, TranscriptMessage, View};

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
                draw_border(screen, info.rect, bg, border, node.title());
            }
        }
        NodeKind::Text => {
            if let Some(text) = node.text() {
                screen.write_str(info.content_rect.x, info.content_rect.y, &text, fg, bg);
            }
        }
        NodeKind::Input => render_input(node, info.content_rect, theme, screen, fg, bg),
        NodeKind::Transcript => render_transcript(node, info.content_rect, theme, screen, bg),
        NodeKind::FuzzyPanel => render_fuzzy_panel(node, info.content_rect, theme, screen, bg),
        NodeKind::Row | NodeKind::Col | NodeKind::ScrollArea => {}
    }

    if !matches!(
        node.kind(),
        NodeKind::Transcript | NodeKind::FuzzyPanel | NodeKind::Input
    ) {
        for child in node.children() {
            render_node(child, layout, theme, screen, bg);
        }
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

fn draw_border(screen: &mut Screen, rect: Rect, bg: Color, border: Color, title: Option<&str>) {
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

    if let Some(title) = title {
        let max = rect.w.saturating_sub(4) as usize;
        let title = title.chars().take(max).collect::<String>();
        screen.write_str(rect.x.saturating_add(2), rect.y, &title, border, bg);
    }
}

fn render_input<Action>(
    node: &PrimitiveNode<Action>,
    rect: Rect,
    theme: &Theme,
    screen: &mut Screen,
    fg: Color,
    bg: Color,
) {
    let Some(input) = node.input() else {
        return;
    };
    if rect.w == 0 || rect.h == 0 {
        return;
    }
    let prompt = if input.loading {
        let frames = ['|', '/', '-', '\\'];
        format!("{} ", frames[input.loading_phase % frames.len()])
    } else {
        "> ".to_string()
    };
    let display = if input.password && !input.value.is_empty() {
        "*".repeat(input.value.chars().count())
    } else if input.value.is_empty() {
        input.placeholder.clone()
    } else {
        input.value.clone()
    };
    let text_color = if input.value.is_empty() {
        theme.resolve(Token::TextMuted)
    } else {
        fg
    };
    screen.write_str(rect.x, rect.y, &prompt, theme.resolve(Token::Accent), bg);
    screen.write_str(
        rect.x.saturating_add(prompt.chars().count() as u16),
        rect.y,
        &display,
        text_color,
        bg,
    );
}

fn render_transcript<Action>(
    node: &PrimitiveNode<Action>,
    rect: Rect,
    theme: &Theme,
    screen: &mut Screen,
    bg: Color,
) {
    let Some(transcript) = node.transcript() else {
        return;
    };
    let mut lines: Vec<(String, Color)> = Vec::new();
    if transcript.messages.is_empty() {
        push_body_lines(
            &mut lines,
            &transcript.empty_text,
            theme.resolve(Token::Text),
        );
    } else {
        for message in &transcript.messages {
            push_message_lines(&mut lines, message, theme);
            lines.push((String::new(), theme.resolve(Token::Text)));
        }
    }
    if let Some(notice) = &transcript.notice {
        lines.push((notice.title.clone(), theme.resolve(notice.color)));
        lines.push((notice.detail.clone(), theme.resolve(Token::TextMuted)));
    }
    let scroll_y = usize::from(node.scroll_y());
    for (row, (text, color)) in lines
        .into_iter()
        .skip(scroll_y)
        .take(rect.h as usize)
        .enumerate()
    {
        screen.write_str(rect.x, rect.y.saturating_add(row as u16), &text, color, bg);
    }
}

fn push_message_lines(
    lines: &mut Vec<(String, Color)>,
    message: &TranscriptMessage,
    theme: &Theme,
) {
    lines.push((
        format!("{}:", message.label),
        theme.resolve(message.label_color),
    ));
    push_body_lines(lines, &message.body, theme.resolve(Token::Text));
}

fn push_body_lines(lines: &mut Vec<(String, Color)>, body: &str, color: Color) {
    if body.is_empty() {
        lines.push((String::new(), color));
        return;
    }
    for line in body.lines() {
        lines.push((format!("  {line}"), color));
    }
}

fn render_fuzzy_panel<Action>(
    node: &PrimitiveNode<Action>,
    rect: Rect,
    theme: &Theme,
    screen: &mut Screen,
    bg: Color,
) {
    let Some(panel) = node.fuzzy() else {
        return;
    };
    if rect.w == 0 || rect.h == 0 {
        return;
    }
    let border = theme.resolve(Token::Border);
    draw_border(screen, rect, bg, border, panel.title.as_deref());
    let inner = Rect::new(
        rect.x.saturating_add(1),
        rect.y.saturating_add(1),
        rect.w.saturating_sub(2),
        rect.h.saturating_sub(2),
    );
    let query_text = if panel.query.is_empty() {
        panel.placeholder.as_str()
    } else {
        panel.query.as_str()
    };
    screen.write_str(inner.x, inner.y, "> ", theme.resolve(Token::Accent), bg);
    screen.write_str(
        inner.x.saturating_add(2),
        inner.y,
        query_text,
        if panel.query.is_empty() {
            theme.resolve(Token::TextMuted)
        } else {
            theme.resolve(Token::Text)
        },
        bg,
    );

    let matches = fuzzy_matches(&panel.items, &panel.query);
    if matches.is_empty() {
        screen.write_str(
            inner.x,
            inner.y.saturating_add(1),
            &panel.empty_text,
            theme.resolve(Token::TextMuted),
            bg,
        );
        return;
    }

    let selected = panel.selected.min(matches.len().saturating_sub(1));
    for (row, matched) in matches
        .iter()
        .skip(selected.saturating_sub(inner.h.saturating_sub(2) as usize))
        .take(inner.h.saturating_sub(2) as usize)
        .enumerate()
    {
        let is_selected = matched.index == matches[selected].index;
        let y = inner.y.saturating_add(1).saturating_add(row as u16);
        let row_bg = if is_selected {
            theme.resolve(Token::Selection)
        } else {
            bg
        };
        screen.fill_rect(
            Rect::new(inner.x, y, inner.w, 1),
            Cell {
                bg: row_bg,
                ..Cell::default()
            },
        );
        let prefix = if is_selected { "> " } else { "  " };
        screen.write_str(
            inner.x,
            y,
            &format!("{prefix}{}", panel.items[matched.index]),
            theme.resolve(Token::Text),
            row_bg,
        );
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
