// RichTextWidget — text with inline styling via Spans.
// Each line is a Vec<Span>; each Span has its own fg/bg/attrs.

use arbor_tui_primitives::cell::{Cell, Span};
use arbor_tui_primitives::layout::{LayoutProps, Rect, Size, SizeConstraint};
use arbor_tui_render::screen::VirtualScreen;
use arbor_tui_render::theme::Theme;
use arbor_tui_widget::widget::{Widget, WidgetId};

pub struct RichTextWidget {
    pub id: WidgetId,
    pub props: LayoutProps,
    /// Lines of styled spans. Each inner Vec is one line.
    pub lines: Vec<Vec<Span>>,
    /// If true, text is truncated to content_rect width.
    pub clip: bool,
    /// 背景色，用于填充 spans 未覆盖的空白区域。
    pub bg: Cell,
}

impl Widget for RichTextWidget {
    fn id(&self) -> WidgetId { self.id }
    fn layout_props(&self) -> &LayoutProps { &self.props }

    fn measure(&self, _available: Size) -> SizeConstraint {
        let h = (self.lines.len() as u16).max(1);
        let w = self.lines.iter()
            .flat_map(|spans| {
                let len: usize = spans.iter().map(|s| s.text.chars().count()).sum();
                Some(len as u16)
            })
            .max()
            .unwrap_or(1)
            .max(1);
        SizeConstraint::fixed(w, h)
    }

    fn render(&self, rect: Rect, _theme: &Theme) -> VirtualScreen {
        let mut screen = VirtualScreen::new(rect.w.max(1), rect.h.max(1));

        // 先用组件背景色填充整个区域，避免 Cell::default() (黑底) 在
        // blit 时覆盖父组件的背景。
        screen.fill_rect(Rect::new(0, 0, rect.w.max(1), rect.h.max(1)), &self.bg);

        for (i, spans) in self.lines.iter().enumerate() {
            if i as u16 >= rect.h { break; }
            if self.clip {
                // Write as much as fits
                screen.write_spans(0, i as u16, spans);
            } else {
                screen.write_spans(0, i as u16, spans);
            }
        }
        screen
    }
}
