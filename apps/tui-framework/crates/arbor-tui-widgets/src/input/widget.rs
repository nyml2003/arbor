// InputWidget — single-line text input with cursor and visual states.

use arbor_tui_domain::cell::{AnsiColor, Attrs, Cell};
use arbor_tui_domain::input::KeyHandleResult;
use arbor_tui_domain::layout::{LayoutProps, Rect, Size, SizeCalc, SizeConstraint};
use arbor_tui_domain::screen::VirtualScreen;
use arbor_tui_domain::text::{self, TruncateStrategy};
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::{Widget, WidgetAction, WidgetId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum InputVisualState {
    Idle,
    Focused,
    Loading,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct InputStyle {
    pub bg: AnsiColor,
    pub prompt_fg: AnsiColor,
    pub text_fg: AnsiColor,
    pub placeholder_fg: AnsiColor,
    pub cursor_fg: AnsiColor,
    pub cursor_bg: AnsiColor,
}

pub struct InputWidget {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub buffer: String,
    pub cursor: usize,
    pub placeholder: String,
    pub password: bool,
    pub loading: bool,
    pub loading_phase: usize,
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
        self.render_state(rect, theme, InputVisualState::Idle)
    }

    fn render_focused(&self, rect: Rect, theme: &Theme) -> VirtualScreen {
        if self.loading {
            self.render_state(rect, theme, InputVisualState::Loading)
        } else {
            self.render_state(rect, theme, InputVisualState::Focused)
        }
    }

    fn perform(&mut self, action: &WidgetAction) -> KeyHandleResult {
        match action {
            WidgetAction::Activate => {
                if !self.loading {
                    if let Some(ref cb) = self.on_submit {
                        cb(self.buffer.clone());
                    }
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
    fn render_state(
        &self,
        rect: Rect,
        theme: &Theme,
        requested: InputVisualState,
    ) -> VirtualScreen {
        let state = if self.loading {
            InputVisualState::Loading
        } else {
            requested
        };
        let width = rect.w.max(1);
        let mut screen = VirtualScreen::new(width, 1);
        let style = input_style(theme, state);
        let fill = Cell {
            bg: style.bg,
            ..Default::default()
        };
        screen.fill_rect(Rect::new(0, 0, width, 1), &fill);

        let prompt = self.prompt_text(state);
        screen.write_str(0, 0, &prompt, style.prompt_fg, style.bg, Attrs::default());

        let content_start = text::measure_width(&prompt);
        let content_w = width.saturating_sub(content_start);
        let display = self.display_text();
        let display_fg = if self.buffer.is_empty() {
            style.placeholder_fg
        } else {
            style.text_fg
        };

        if state == InputVisualState::Focused && self.buffer.is_empty() {
            self.draw_cursor(&mut screen, width, content_start, &display, style);
            let placeholder_start = content_start.saturating_add(1);
            let placeholder_w = width.saturating_sub(placeholder_start);
            let placeholder = text::truncate(&display, placeholder_w, TruncateStrategy::End);
            screen.write_str(
                placeholder_start,
                0,
                &placeholder,
                display_fg,
                style.bg,
                Attrs::default(),
            );
        } else if state == InputVisualState::Focused {
            let (visible, cursor_offset) =
                visible_slice_around_cursor(&display, self.cursor, content_w);
            screen.write_str(
                content_start,
                0,
                &visible,
                display_fg,
                style.bg,
                Attrs::default(),
            );
            self.draw_cursor(
                &mut screen,
                width,
                content_start.saturating_add(cursor_offset),
                &display,
                style,
            );
        } else {
            let truncated = text::truncate(&display, content_w, TruncateStrategy::End);
            screen.write_str(
                content_start,
                0,
                &truncated,
                display_fg,
                style.bg,
                Attrs::default(),
            );
        }

        screen
    }

    fn prompt_text(&self, state: InputVisualState) -> String {
        if state == InputVisualState::Loading {
            format!("{} ", loading_frame(self.loading_phase))
        } else if state == InputVisualState::Focused {
            "▸ ".to_string()
        } else {
            "› ".to_string()
        }
    }

    fn draw_cursor(
        &self,
        screen: &mut VirtualScreen,
        width: u16,
        cursor_col: u16,
        display: &str,
        style: InputStyle,
    ) {
        if cursor_col >= width {
            return;
        }

        let cursor_ch = if self.buffer.is_empty() {
            ' '
        } else {
            display.chars().nth(self.cursor).unwrap_or(' ')
        };
        if let Some(cell) = screen.cell_at_mut(cursor_col, 0) {
            cell.ch = cursor_ch;
            cell.bg = style.cursor_bg;
            cell.fg = style.cursor_fg;
        }
    }

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

pub fn input_style(theme: &Theme, state: InputVisualState) -> InputStyle {
    match state {
        InputVisualState::Idle => InputStyle {
            bg: theme.surface_alt(),
            prompt_fg: theme.text_dim(),
            text_fg: theme.text(),
            placeholder_fg: theme.text_dim(),
            cursor_fg: theme.surface(),
            cursor_bg: theme.primary(),
        },
        InputVisualState::Focused => InputStyle {
            bg: theme.surface_alt(),
            prompt_fg: theme.accent(),
            text_fg: theme.text(),
            placeholder_fg: theme.text_dim(),
            cursor_fg: theme.surface(),
            cursor_bg: theme.primary(),
        },
        InputVisualState::Loading => InputStyle {
            bg: theme.surface_alt(),
            prompt_fg: theme.warning(),
            text_fg: theme.text_dim(),
            placeholder_fg: theme.text_dim(),
            cursor_fg: theme.surface(),
            cursor_bg: theme.warning(),
        },
    }
}

fn loading_frame(phase: usize) -> char {
    const FRAMES: [char; 4] = ['◜', '◐', '◝', '◑'];
    FRAMES[phase % FRAMES.len()]
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
