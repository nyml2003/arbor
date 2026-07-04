use arbor_tui_primitives::cell::AnsiColor;
use arbor_tui_primitives::layout::{LayoutProps, RectOffset};
use arbor_tui_widget::widget::WidgetNode;
use super::widget::BorderWidget;
use crate::widget_manager::WidgetManager;

pub struct Border {
    child: Option<WidgetNode>,
    title: Option<String>,
    rounded: bool,
    fg: Option<AnsiColor>,
    bg: Option<AnsiColor>,
    padding: RectOffset,
}

impl Border {
    pub fn new() -> Self {
        Self { child: None, title: None, rounded: false, fg: None, bg: None, padding: RectOffset::default() }
    }
    pub fn child(mut self, c: WidgetNode) -> Self { self.child = Some(c); self }
    pub fn title(mut self, t: impl Into<String>) -> Self { self.title = Some(t.into()); self }
    pub fn rounded(mut self) -> Self { self.rounded = true; self }
    pub fn fg(mut self, c: AnsiColor) -> Self { self.fg = Some(c); self }
    pub fn bg(mut self, c: AnsiColor) -> Self { self.bg = Some(c); self }
    pub fn padding(mut self, p: RectOffset) -> Self { self.padding = p; self }
    pub fn build(self, wm: &WidgetManager, t: &arbor_tui_render::theme::Theme) -> WidgetNode {
        wm.wrap(|id| BorderWidget {
            id, props: LayoutProps { padding: self.padding, ..Default::default() },
            child: Box::new(self.child.expect("Border::child must be set")),
            title: self.title, rounded: self.rounded,
            fg: self.fg.unwrap_or(t.border()),
            bg: self.bg.unwrap_or(t.surface()),
        })
    }
}
