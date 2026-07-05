use arbor_tui_domain::cell::AnsiColor;
use arbor_tui_domain::layout::RectOffset;
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::WidgetNode;
use arbor_tui_widgets::divider::Divider;
use arbor_tui_widgets::stack::Row;
use arbor_tui_widgets::text::Text;
use arbor_tui_widgets::widget_factory::WidgetFactory;

/// A one-line section marker: `╭------╯ Label`.
pub struct SectionDivider {
    label: String,
    divider_width: u16,
    divider_fg: Option<AnsiColor>,
    label_fg: Option<AnsiColor>,
    bg: Option<AnsiColor>,
    padding: RectOffset,
    flex: f32,
}

impl SectionDivider {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            divider_width: 10,
            divider_fg: None,
            label_fg: None,
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

    pub fn label_fg(mut self, color: AnsiColor) -> Self {
        self.label_fg = Some(color);
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
        let bg = self.bg.unwrap_or_else(|| theme.surface());
        let divider = Divider::new()
            .width(self.divider_width)
            .fg(self.divider_fg.unwrap_or_else(|| theme.border()))
            .bg(bg)
            .build(factory, theme);
        let label = Text::new(format!(" {}", self.label))
            .fg(self.label_fg.unwrap_or_else(|| theme.text_dim()))
            .bg(bg)
            .build(factory, theme);

        Row::new()
            .padding(self.padding)
            .flex(self.flex)
            .children([divider, label])
            .build(factory, theme)
    }
}
