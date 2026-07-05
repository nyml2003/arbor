use arbor_tui_domain::cell::AnsiColor;
use arbor_tui_domain::layout::RectOffset;
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::WidgetNode;
use arbor_tui_widgets::text::Text;
use arbor_tui_widgets::widget_factory::WidgetFactory;

/// A single-line status region with conventional left padding.
pub struct StatusLine {
    text: String,
    fg: Option<AnsiColor>,
    bg: Option<AnsiColor>,
    padding: RectOffset,
    flex: f32,
}

impl StatusLine {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            fg: None,
            bg: None,
            padding: RectOffset {
                left: 1,
                ..Default::default()
            },
            flex: 0.0,
        }
    }

    pub fn fg(mut self, color: AnsiColor) -> Self {
        self.fg = Some(color);
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
        Text::new(self.text)
            .fg(self.fg.unwrap_or_else(|| theme.text_dim()))
            .bg(self.bg.unwrap_or_else(|| theme.surface()))
            .padding(self.padding)
            .flex(self.flex)
            .build(factory, theme)
    }
}
