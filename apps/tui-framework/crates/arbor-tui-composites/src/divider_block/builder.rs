use arbor_tui_domain::cell::AnsiColor;
use arbor_tui_domain::layout::RectOffset;
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::WidgetNode;
use arbor_tui_widgets::stack::Col;
use arbor_tui_widgets::widget_factory::WidgetFactory;

use crate::SectionDivider;

/// A section header followed by a body widget.
pub struct DividerBlock {
    title: String,
    body: WidgetNode,
    divider_width: u16,
    divider_fg: Option<AnsiColor>,
    title_fg: Option<AnsiColor>,
    bg: Option<AnsiColor>,
    padding: RectOffset,
    flex: f32,
}

impl DividerBlock {
    pub fn new(title: impl Into<String>, body: WidgetNode) -> Self {
        Self {
            title: title.into(),
            body,
            divider_width: 10,
            divider_fg: None,
            title_fg: None,
            bg: None,
            padding: RectOffset::default(),
            flex: 0.0,
        }
    }

    pub fn divider_width(mut self, width: u16) -> Self {
        self.divider_width = width.max(1);
        self
    }

    pub fn divider_fg(mut self, color: AnsiColor) -> Self {
        self.divider_fg = Some(color);
        self
    }

    pub fn title_fg(mut self, color: AnsiColor) -> Self {
        self.title_fg = Some(color);
        self
    }

    pub fn bg(mut self, color: AnsiColor) -> Self {
        self.bg = Some(color);
        self
    }

    pub fn padding(mut self, padding: RectOffset) -> Self {
        self.padding = padding;
        self
    }

    pub fn flex(mut self, flex: f32) -> Self {
        self.flex = flex;
        self
    }

    pub fn build(self, factory: &WidgetFactory, theme: &Theme) -> WidgetNode {
        let mut header = SectionDivider::new(self.title).divider_width(self.divider_width);
        if let Some(color) = self.divider_fg {
            header = header.divider_fg(color);
        }
        if let Some(color) = self.title_fg {
            header = header.label_fg(color);
        }
        if let Some(color) = self.bg {
            header = header.bg(color);
        }

        Col::new()
            .padding(self.padding)
            .flex(self.flex)
            .children([header.build(factory, theme), self.body])
            .build(factory, theme)
    }
}
