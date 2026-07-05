use super::widget::{FuzzyPanelSelection, FuzzyPanelWidget};
use arbor_tui_domain::cell::AnsiColor;
use arbor_tui_domain::layout::{LayoutProps, RectOffset};
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::WidgetNode;
use arbor_tui_widgets::widget_factory::WidgetFactory;

/// A self-contained fzf-like picker panel.
pub struct FuzzyPanel {
    items: Vec<String>,
    title: Option<String>,
    placeholder: String,
    empty_text: String,
    rounded: bool,
    fg: Option<AnsiColor>,
    bg: Option<AnsiColor>,
    accent: Option<AnsiColor>,
    padding: RectOffset,
    flex: f32,
    on_query_change: Option<Box<dyn Fn(String)>>,
    on_submit: Option<Box<dyn Fn(FuzzyPanelSelection)>>,
}

impl FuzzyPanel {
    pub fn new(items: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            items: items.into_iter().map(Into::into).collect(),
            title: None,
            placeholder: "Search".to_string(),
            empty_text: "No matches".to_string(),
            rounded: false,
            fg: None,
            bg: None,
            accent: None,
            padding: RectOffset::default(),
            flex: 0.0,
            on_query_change: None,
            on_submit: None,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn empty_text(mut self, empty_text: impl Into<String>) -> Self {
        self.empty_text = empty_text.into();
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

    pub fn accent(mut self, color: AnsiColor) -> Self {
        self.accent = Some(color);
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

    pub fn on_query_change(mut self, callback: impl Fn(String) + 'static) -> Self {
        self.on_query_change = Some(Box::new(callback));
        self
    }

    pub fn on_submit(mut self, callback: impl Fn(FuzzyPanelSelection) + 'static) -> Self {
        self.on_submit = Some(Box::new(callback));
        self
    }

    pub fn build(self, factory: &WidgetFactory, _theme: &Theme) -> WidgetNode {
        factory.wrap(|id| FuzzyPanelWidget {
            id,
            props: LayoutProps {
                padding: self.padding,
                flex: self.flex,
                ..Default::default()
            },
            items: self.items,
            title: self.title,
            placeholder: self.placeholder,
            empty_text: self.empty_text,
            query: String::new(),
            selected_match: 0,
            rounded: self.rounded,
            fg: self.fg,
            bg: self.bg,
            accent: self.accent,
            on_query_change: self.on_query_change,
            on_submit: self.on_submit,
        })
    }
}
