// TextWidget — styled text display with word wrapping and truncation.

use arbor_tui_primitives::cell::{AnsiColor, Attrs};
use arbor_tui_primitives::layout::{LayoutProps, Size, SizeConstraint};
use arbor_tui_render::screen::VirtualScreen;
use arbor_tui_reactive::signal::ReadSignal;
use arbor_tui_primitives::text::{self, TruncateStrategy, WrapStrategy};
use arbor_tui_render::theme::Theme;
use arbor_tui_widget::widget::{Widget, WidgetId, WidgetNode};

pub struct TextWidget {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub text: ReadSignal<String>,
    pub style: ReadSignal<TextStyle>,
    pub wrap: WrapStrategy,
    pub truncate: TruncateStrategy,
}

#[derive(Clone, PartialEq)]
pub struct TextStyle {
    pub fg: AnsiColor,
    pub bg: AnsiColor,
    pub attrs: Attrs,
}

impl Default for TextStyle {
    /// Matches dark theme: soft white on black.
    fn default() -> Self {
        Self {
            fg: AnsiColor { palette: arbor_tui_primitives::cell::PaletteColor(252), true_color: None },
            bg: AnsiColor { palette: arbor_tui_primitives::cell::PaletteColor(0), true_color: None },
            attrs: Attrs::default(),
        }
    }
}

impl Widget for TextWidget {
    fn id(&self) -> WidgetId { self.id }
    fn layout_props(&self) -> &LayoutProps { &self.props }

    fn on_mount(&mut self) {
        self.text.subscribe(self.id);
        self.style.subscribe(self.id);
    }

    fn on_unmount(&mut self) {
        self.text.unsubscribe(self.id);
        self.style.unsubscribe(self.id);
    }

    fn measure(&self, _available: Size) -> SizeConstraint {
        let text_content = self.text.get();
        let expanded = text::expand_tabs(&text_content);
        let text_w = text::measure_width(&expanded);

        match self.wrap {
            WrapStrategy::None => {
                let w = self.props.width.unwrap_or(text_w).max(1);
                let h = (expanded.lines().count() as u16).max(1);
                SizeConstraint::fixed(w, h)
            }
            _ => {
                // Wrapping width: use available if not fixed-width
                let max_w = self.props.width.unwrap_or(
                    text_w.max(1)
                );
                let lines = text::wrap_lines(&expanded, max_w.max(1), self.wrap);
                SizeConstraint {
                    min_w: 1,
                    min_h: 1,
                    max_w: arbor_tui_primitives::layout::AxisConstraint::Fixed(max_w.max(1)),
                    max_h: arbor_tui_primitives::layout::AxisConstraint::Fixed((lines.len() as u16).max(1)),
                }
            }
        }
    }

    fn render(&self, rect: arbor_tui_primitives::layout::Rect, _theme: &Theme) -> VirtualScreen {
        let mut screen = VirtualScreen::new(rect.w.max(1), rect.h.max(1));
        let text_content = self.text.get();
        let expanded = text::expand_tabs(&text_content);
        let style = self.style.get();

        match self.wrap {
            WrapStrategy::None => {
                for (i, line) in expanded.lines().enumerate() {
                    if i as u16 >= rect.h { break; }
                    let display = text::truncate(line, rect.w, self.truncate);
                    screen.write_str(0, i as u16, &display, style.fg, style.bg, style.attrs);
                }
            }
            _ => {
                let lines = text::wrap_lines(&expanded, rect.w, self.wrap);
                for (i, line) in lines.iter().enumerate() {
                    if i as u16 >= rect.h { break; }
                    let display = text::truncate(line, rect.w, self.truncate);
                    screen.write_str(0, i as u16, &display, style.fg, style.bg, style.attrs);
                }
            }
        }
        screen
    }
}

impl Drop for TextWidget {
    fn drop(&mut self) {
        self.text.unsubscribe(self.id);
        self.style.unsubscribe(self.id);
    }
}
