use arbor_tui_domain::cell::{AnsiColor, Attrs, Cell};
use arbor_tui_domain::input::KeyHandleResult;
use arbor_tui_domain::layout::{AxisConstraint, LayoutProps, Rect, Size, SizeConstraint};
use arbor_tui_domain::screen::VirtualScreen;
use arbor_tui_domain::text::{self, TruncateStrategy};
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::{Widget, WidgetAction, WidgetId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FuzzyPanelSelection {
    pub index: usize,
    pub item: String,
}

pub(crate) struct FuzzyPanelWidget {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub items: Vec<String>,
    pub title: Option<String>,
    pub placeholder: String,
    pub empty_text: String,
    pub query: String,
    pub selected_match: usize,
    pub rounded: bool,
    pub fg: Option<AnsiColor>,
    pub bg: Option<AnsiColor>,
    pub accent: Option<AnsiColor>,
    pub on_query_change: Option<Box<dyn Fn(String)>>,
    pub on_submit: Option<Box<dyn Fn(FuzzyPanelSelection)>>,
}

impl Widget for FuzzyPanelWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout_props(&self) -> &LayoutProps {
        &self.props
    }

    fn focusable(&self) -> bool {
        true
    }

    fn measure(&self, available: Size) -> SizeConstraint {
        SizeConstraint {
            min_w: 16,
            min_h: 5,
            max_w: AxisConstraint::Fixed(available.w),
            max_h: AxisConstraint::Fixed(available.h),
        }
    }

    fn render(&self, rect: Rect, theme: &Theme) -> VirtualScreen {
        self.render_panel(rect, theme, false)
    }

    fn render_focused(&self, rect: Rect, theme: &Theme) -> VirtualScreen {
        self.render_panel(rect, theme, true)
    }

    fn perform(&mut self, action: &WidgetAction) -> KeyHandleResult {
        match action {
            WidgetAction::TypeChar(ch) => {
                self.query.push(*ch);
                self.selected_match = 0;
                self.emit_query_change();
                KeyHandleResult::Handled
            }
            WidgetAction::Backspace => {
                self.query.pop();
                self.selected_match = 0;
                self.emit_query_change();
                KeyHandleResult::Handled
            }
            WidgetAction::Delete => {
                self.query.clear();
                self.selected_match = 0;
                self.emit_query_change();
                KeyHandleResult::Handled
            }
            WidgetAction::NavigateDown => {
                let match_count = self.matches().len();
                if match_count > 0 {
                    self.selected_match = (self.selected_match + 1).min(match_count - 1);
                }
                KeyHandleResult::Handled
            }
            WidgetAction::NavigateUp => {
                self.selected_match = self.selected_match.saturating_sub(1);
                KeyHandleResult::Handled
            }
            WidgetAction::Activate => {
                if let Some(selection) = self.current_selection() {
                    if let Some(ref callback) = self.on_submit {
                        callback(selection);
                    }
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Bubble,
        }
    }
}

impl FuzzyPanelWidget {
    fn emit_query_change(&self) {
        if let Some(ref callback) = self.on_query_change {
            callback(self.query.clone());
        }
    }

    fn current_selection(&self) -> Option<FuzzyPanelSelection> {
        let matches = self.matches();
        let matched = matches.get(self.selected_match.min(matches.len().saturating_sub(1)))?;
        Some(FuzzyPanelSelection {
            index: matched.index,
            item: self.items[matched.index].clone(),
        })
    }

    fn render_panel(&self, rect: Rect, theme: &Theme, focused: bool) -> VirtualScreen {
        let w = rect.w.max(16);
        let h = rect.h.max(5);
        let mut screen = VirtualScreen::new(w, h);
        let bg = self.bg.unwrap_or_else(|| theme.surface());
        let fg = self.fg.unwrap_or_else(|| theme.border());
        let accent = self.accent.unwrap_or_else(|| theme.accent());
        let text_fg = theme.text();
        let dim = theme.text_dim();
        let style = FuzzyPanelStyle {
            theme,
            bg,
            text_fg,
            dim,
            accent,
        };

        screen.fill_rect(
            Rect::new(0, 0, w, h),
            &Cell {
                bg,
                ..Default::default()
            },
        );

        self.draw_border(&mut screen, w, h, fg, bg);
        self.draw_query(&mut screen, w, style, focused);
        self.draw_matches(&mut screen, w, h, style);
        self.draw_status(&mut screen, w, h, theme, bg, dim);

        screen
    }

    fn draw_border(
        &self,
        screen: &mut VirtualScreen,
        w: u16,
        h: u16,
        fg: AnsiColor,
        bg: AnsiColor,
    ) {
        let (tl, tr, bl, br) = if self.rounded {
            ('\u{256D}', '\u{256E}', '\u{2570}', '\u{256F}')
        } else {
            ('\u{250C}', '\u{2510}', '\u{2514}', '\u{2518}')
        };
        let h_line = '\u{2500}';
        let v_line = '\u{2502}';

        for x in 1..w - 1 {
            set_cell(screen, x, 0, h_line, fg, bg, Attrs::default());
            set_cell(screen, x, h - 1, h_line, fg, bg, Attrs::default());
        }
        for y in 1..h - 1 {
            set_cell(screen, 0, y, v_line, fg, bg, Attrs::default());
            set_cell(screen, w - 1, y, v_line, fg, bg, Attrs::default());
        }
        set_cell(screen, 0, 0, tl, fg, bg, Attrs::default());
        set_cell(screen, w - 1, 0, tr, fg, bg, Attrs::default());
        set_cell(screen, 0, h - 1, bl, fg, bg, Attrs::default());
        set_cell(screen, w - 1, h - 1, br, fg, bg, Attrs::default());

        if let Some(ref title) = self.title {
            let max_title_w = (w as usize).saturating_sub(4);
            let display = title.chars().take(max_title_w).collect::<String>();
            for (i, ch) in display.chars().enumerate() {
                set_cell(screen, 2 + i as u16, 0, ch, fg, bg, Attrs::default());
            }
        }
    }

    fn draw_query(
        &self,
        screen: &mut VirtualScreen,
        w: u16,
        style: FuzzyPanelStyle<'_>,
        focused: bool,
    ) {
        let query_row = 1;
        let query_w = w.saturating_sub(4);
        screen.write_str(2, query_row, "> ", style.accent, style.bg, Attrs::default());

        let (display, color) = if self.query.is_empty() {
            (self.placeholder.as_str(), style.dim)
        } else {
            (self.query.as_str(), style.text_fg)
        };
        let display = text::truncate(display, query_w, TruncateStrategy::End);
        screen.write_str(4, query_row, &display, color, style.bg, Attrs::default());

        if focused {
            let cursor_col = 4 + text::measure_width(&display).min(query_w.saturating_sub(1));
            if cursor_col < w - 1 {
                let cursor_ch = if self.query.is_empty() {
                    ' '
                } else {
                    '\u{2588}'
                };
                set_cell(
                    screen,
                    cursor_col,
                    query_row,
                    cursor_ch,
                    style.theme.surface(),
                    style.accent,
                    Attrs::default(),
                );
            }
        }
    }

    fn draw_matches(&self, screen: &mut VirtualScreen, w: u16, h: u16, style: FuzzyPanelStyle<'_>) {
        let matches = self.matches();
        let list_start = 2u16;
        let list_rows = h.saturating_sub(4);
        if list_rows == 0 {
            return;
        }
        if matches.is_empty() {
            screen.write_str(
                2,
                list_start,
                &self.empty_text,
                style.dim,
                style.bg,
                Attrs::default(),
            );
            return;
        }

        let selected = self.selected_match.min(matches.len().saturating_sub(1));
        let scroll_offset = selected.saturating_sub(list_rows.saturating_sub(1) as usize);
        let row_w = w.saturating_sub(2);
        let text_w = w.saturating_sub(5);

        for row in 0..list_rows {
            let Some(matched) = matches.get(scroll_offset + row as usize) else {
                break;
            };
            let is_selected = scroll_offset + row as usize == selected;
            let y = list_start + row;
            let row_bg = if is_selected { style.accent } else { style.bg };
            let row_fg = if is_selected {
                style.theme.surface()
            } else {
                style.text_fg
            };
            let prefix = if is_selected { ">" } else { " " };

            if is_selected {
                screen.fill_rect(
                    Rect::new(1, y, row_w, 1),
                    &Cell {
                        bg: row_bg,
                        ..Default::default()
                    },
                );
            }
            screen.write_str(2, y, prefix, row_fg, row_bg, Attrs::default());
            let display = text::truncate(&self.items[matched.index], text_w, TruncateStrategy::End);
            screen.write_str(4, y, &display, row_fg, row_bg, Attrs::default());
        }
    }

    fn draw_status(
        &self,
        screen: &mut VirtualScreen,
        w: u16,
        h: u16,
        _theme: &Theme,
        bg: AnsiColor,
        dim: AnsiColor,
    ) {
        let matches = self.matches();
        let selected = if matches.is_empty() {
            0
        } else {
            self.selected_match.min(matches.len() - 1) + 1
        };
        let status = format!(
            "{selected}/{} matches  Enter: select  Up/Down: move  Del: clear",
            matches.len()
        );
        let display = text::truncate(&status, w.saturating_sub(4), TruncateStrategy::End);
        screen.write_str(2, h - 2, &display, dim, bg, Attrs::default());
    }

    fn matches(&self) -> Vec<FuzzyMatch> {
        rank_items(&self.items, &self.query)
    }
}

#[derive(Copy, Clone)]
struct FuzzyPanelStyle<'a> {
    theme: &'a Theme,
    bg: AnsiColor,
    text_fg: AnsiColor,
    dim: AnsiColor,
    accent: AnsiColor,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct FuzzyMatch {
    index: usize,
    score: usize,
}

fn rank_items(items: &[String], query: &str) -> Vec<FuzzyMatch> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return items
            .iter()
            .enumerate()
            .map(|(index, _)| FuzzyMatch {
                index,
                score: index,
            })
            .collect();
    }

    let mut matches = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            score_item(&item.to_lowercase(), &query).map(|score| FuzzyMatch {
                index,
                score: score * items.len() + index,
            })
        })
        .collect::<Vec<_>>();
    matches.sort_by_key(|matched| matched.score);
    matches
}

fn score_item(item: &str, query: &str) -> Option<usize> {
    if let Some(pos) = item.find(query) {
        return Some(pos);
    }

    let mut score = 0usize;
    let mut last_pos = 0usize;
    for needle in query.chars() {
        let found = item[last_pos..].find(needle)?;
        score += found + 8;
        last_pos += found + needle.len_utf8();
    }
    Some(score)
}

fn set_cell(
    screen: &mut VirtualScreen,
    col: u16,
    row: u16,
    ch: char,
    fg: AnsiColor,
    bg: AnsiColor,
    attrs: Attrs,
) {
    if let Some(cell) = screen.cell_at_mut(col, row) {
        cell.ch = ch;
        cell.fg = fg;
        cell.bg = bg;
        cell.attrs = attrs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks_substring_before_subsequence() {
        let items = vec![
            "src/main.rs".to_string(),
            "apps/tui-framework".to_string(),
            "README.md".to_string(),
        ];

        let matches = rank_items(&items, "tu");

        assert_eq!(matches[0].index, 1);
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn empty_query_preserves_item_order() {
        let items = vec!["b".to_string(), "a".to_string()];

        let matches = rank_items(&items, "");

        assert_eq!(
            matches
                .iter()
                .map(|matched| matched.index)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
    }
}
