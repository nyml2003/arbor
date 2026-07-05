// InputWidget — single-line text input with cursor.

use arbor_tui_domain::cell::{Attrs, Cell};
use arbor_tui_domain::input::KeyHandleResult;
use arbor_tui_domain::layout::{LayoutProps, Rect, Size, SizeCalc, SizeConstraint};
use arbor_tui_domain::screen::VirtualScreen;
use arbor_tui_domain::text::{self, TruncateStrategy};
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::{Widget, WidgetAction, WidgetId};

pub struct InputWidget {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub buffer: String,
    pub cursor: usize,
    pub placeholder: String,
    pub password: bool,
    pub on_change: Option<Box<dyn Fn(String)>>,
    pub on_submit: Option<Box<dyn Fn(String)>>,
}

impl Widget for InputWidget {
    fn id(&self) -> WidgetId {
        self.id
    }
    fn layout_props(&self) -> &LayoutProps {
        &self.props
    }
    fn focusable(&self) -> bool {
        true
    }

    fn on_mount(&mut self) {
        // InputWidget buffer is owned; no signal subscriptions in v1
    }

    fn measure(&self, available: Size) -> SizeConstraint {
        if let Some(w_val) = self.props.width {
            SizeConstraint::fixed(w_val, 1)
        } else {
            let avail =
                SizeCalc::content_available(available, self.props.padding, self.props.margin);
            SizeConstraint {
                min_w: 1,
                min_h: 1,
                max_w: arbor_tui_domain::layout::AxisConstraint::Fixed(avail.w.max(1)),
                max_h: arbor_tui_domain::layout::AxisConstraint::Fixed(1),
            }
        }
    }

    fn render(&self, rect: Rect, theme: &Theme) -> VirtualScreen {
        let mut screen = VirtualScreen::new(rect.w.max(1), 1);
        let border_fg = theme.border();
        let bg = theme.surface_alt();
        let text_fg = theme.text();

        // 先用背景色填充整个区域，避免 Cell::default() 黑底覆盖父组件。
        let fill = Cell {
            bg,
            ..Default::default()
        };
        screen.fill_rect(Rect::new(0, 0, rect.w.max(1), 1), &fill);

        screen.write_str(0, 0, "> ", border_fg, bg, Attrs::default());

        let content_start: u16 = 2;
        let content_w = rect.w.saturating_sub(content_start);

        let display = self.display_text();

        let truncated = text::truncate(&display, content_w, TruncateStrategy::End);
        screen.write_str(content_start, 0, &truncated, text_fg, bg, Attrs::default());
        screen
    }

    fn render_focused(&self, rect: Rect, theme: &Theme) -> VirtualScreen {
        let mut screen = VirtualScreen::new(rect.w.max(1), 1);
        let border_fg = theme.border();
        let bg = theme.surface_alt();
        let text_fg = theme.text();
        let cursor_bg = theme.primary();

        // 先用背景色填充整个区域，避免 Cell::default() 黑底覆盖父组件。
        let fill = Cell {
            bg,
            ..Default::default()
        };
        screen.fill_rect(Rect::new(0, 0, rect.w.max(1), 1), &fill);

        // "> " prompt with accent when focused
        screen.write_str(0, 0, "> ", border_fg, bg, Attrs::default());

        let content_start: u16 = 2;
        let content_w = rect.w.saturating_sub(content_start);

        let display = self.display_text();
        let (visible, cursor_offset) =
            visible_slice_around_cursor(&display, self.cursor, content_w);
        screen.write_str(content_start, 0, &visible, text_fg, bg, Attrs::default());

        // Cursor position in columns (chars may be CJK = 2 cols wide)
        let cursor_col = content_start + cursor_offset;
        if cursor_col < rect.w {
            let cursor_ch = display.chars().nth(self.cursor).unwrap_or(' ');
            if let Some(cell) = screen.cell_at_mut(cursor_col, 0) {
                cell.ch = cursor_ch;
                cell.bg = cursor_bg;
                cell.fg = theme.surface();
            }
        }
        screen
    }

    fn perform(&mut self, action: &WidgetAction) -> KeyHandleResult {
        match action {
            WidgetAction::Activate => {
                if let Some(ref cb) = self.on_submit {
                    cb(self.buffer.clone());
                }
                KeyHandleResult::Handled
            }
            WidgetAction::Backspace => {
                if self.cursor > 0 {
                    let idx = self
                        .buffer
                        .char_indices()
                        .nth(self.cursor - 1)
                        .map(|(i, _)| i)
                        .expect("cursor-1 must be a valid char boundary");
                    self.buffer.remove(idx);
                    self.cursor -= 1;
                    if let Some(ref cb) = self.on_change {
                        cb(self.buffer.clone());
                    }
                }
                KeyHandleResult::Handled
            }
            WidgetAction::TypeChar(c) => {
                let insert_idx = self
                    .buffer
                    .char_indices()
                    .nth(self.cursor)
                    .map(|(i, _)| i)
                    .unwrap_or_else(|| self.buffer.len());
                self.buffer.insert(insert_idx, *c);
                self.cursor += 1;
                if let Some(ref cb) = self.on_change {
                    cb(self.buffer.clone());
                }
                KeyHandleResult::Handled
            }
            WidgetAction::NavigateLeft => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
                KeyHandleResult::Handled
            }
            WidgetAction::NavigateRight => {
                if self.cursor < self.buffer.chars().count() {
                    self.cursor += 1;
                }
                KeyHandleResult::Handled
            }
            WidgetAction::Home => {
                self.cursor = 0;
                KeyHandleResult::Handled
            }
            WidgetAction::End => {
                self.cursor = self.buffer.chars().count();
                KeyHandleResult::Handled
            }
            WidgetAction::Delete => {
                let len = self.buffer.chars().count();
                if self.cursor < len {
                    let idx = self
                        .buffer
                        .char_indices()
                        .nth(self.cursor)
                        .map(|(i, _)| i)
                        .expect("cursor must be a valid char boundary when cursor < len");
                    self.buffer.remove(idx);
                    if let Some(ref cb) = self.on_change {
                        cb(self.buffer.clone());
                    }
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Bubble,
        }
    }
}

impl InputWidget {
    fn display_text(&self) -> String {
        if self.password && !self.buffer.is_empty() {
            "●".repeat(self.buffer.chars().count())
        } else if self.buffer.is_empty() {
            self.placeholder.clone()
        } else {
            self.buffer.clone()
        }
    }
}

fn visible_slice_around_cursor(display: &str, cursor: usize, content_w: u16) -> (String, u16) {
    if content_w == 0 {
        return (String::new(), 0);
    }

    let cursor_col = text::column_offset(display, cursor);
    let max_cursor_col = content_w.saturating_sub(1);
    let start_col = cursor_col.saturating_sub(max_cursor_col);
    let end_col = start_col + content_w;
    let mut visible = String::new();
    let mut col = 0u16;

    for ch in display.chars() {
        let ch_w = text::char_width(ch);
        let next_col = col.saturating_add(ch_w);
        if col >= start_col && next_col <= end_col {
            visible.push(ch);
        }
        col = next_col;
    }

    let cursor_offset = cursor_col.saturating_sub(start_col).min(max_cursor_col);
    (visible, cursor_offset)
}

impl Drop for InputWidget {
    fn drop(&mut self) {}
}
