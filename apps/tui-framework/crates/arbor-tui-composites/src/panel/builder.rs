use arbor_tui_domain::cell::AnsiColor;
use arbor_tui_domain::layout::RectOffset;
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::WidgetNode;
use arbor_tui_widgets::border::Border;
use arbor_tui_widgets::widget_factory::WidgetFactory;

/// A common bordered container for pages and regions.
pub struct Panel {
    child: WidgetNode,
    title: Option<String>,
    rounded: bool,
    fg: Option<AnsiColor>,
    bg: Option<AnsiColor>,
    padding: RectOffset,
    flex: f32,
}

impl Panel {
    pub fn new(child: WidgetNode) -> Self {
        Self {
            child,
            title: None,
            rounded: false,
            fg: None,
            bg: None,
            padding: RectOffset::default(),
            flex: 0.0,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn rounded(mut self) -> Self {
        self.rounded = true;
        self
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
        let mut border = Border::new()
            .child(self.child)
            .padding(self.padding)
            .flex(self.flex);
        if let Some(title) = self.title {
            border = border.title(title);
        }
        if self.rounded {
            border = border.rounded();
        }
        if let Some(fg) = self.fg {
            border = border.fg(fg);
        }
        if let Some(bg) = self.bg {
            border = border.bg(bg);
        }
        border.build(factory, theme)
    }
}
