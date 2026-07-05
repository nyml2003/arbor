use arbor_tui_domain::cell::AnsiColor;
use arbor_tui_domain::layout::RectOffset;
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::WidgetNode;
use arbor_tui_widgets::border::Border;
use arbor_tui_widgets::input::Input;
use arbor_tui_widgets::widget_factory::WidgetFactory;

/// A bordered single-line prompt input.
pub struct PromptBar {
    placeholder: String,
    title: Option<String>,
    rounded: bool,
    on_submit: Option<Box<dyn Fn(String)>>,
    fg: Option<AnsiColor>,
    bg: Option<AnsiColor>,
    padding: RectOffset,
    flex: f32,
}

impl Default for PromptBar {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptBar {
    pub fn new() -> Self {
        Self {
            placeholder: String::new(),
            title: None,
            rounded: false,
            on_submit: None,
            fg: None,
            bg: None,
            padding: RectOffset::default(),
            flex: 0.0,
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn rounded(mut self) -> Self {
        self.rounded = true;
        self
    }

    pub fn on_submit(mut self, callback: impl Fn(String) + 'static) -> Self {
        self.on_submit = Some(Box::new(callback));
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
        let mut input = Input::new().placeholder(self.placeholder);
        if let Some(callback) = self.on_submit {
            input = input.on_submit(callback);
        }
        let mut border = Border::new()
            .child(input.build(factory, theme))
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
