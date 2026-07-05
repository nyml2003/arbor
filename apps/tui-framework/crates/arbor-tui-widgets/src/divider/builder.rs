use crate::divider::widget::{DividerStyle, DividerWidget};
use crate::widget_factory::WidgetFactory;
use arbor_tui_domain::cell::{AnsiColor, Attrs};
use arbor_tui_domain::layout::LayoutProps;
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::WidgetNode;

pub struct Divider {
    left: char,
    fill: char,
    right: char,
    fg: Option<AnsiColor>,
    bg: Option<AnsiColor>,
    attrs: Attrs,
    width: Option<u16>,
    flex: f32,
}

impl Default for Divider {
    fn default() -> Self {
        Self::new()
    }
}

impl Divider {
    pub fn new() -> Self {
        Self {
            left: '\u{256D}',
            fill: '-',
            right: '\u{256F}',
            fg: None,
            bg: None,
            attrs: Attrs::default(),
            width: None,
            flex: 0.0,
        }
    }

    pub fn left(mut self, ch: char) -> Self {
        self.left = ch;
        self
    }

    pub fn fill(mut self, ch: char) -> Self {
        self.fill = ch;
        self
    }

    pub fn right(mut self, ch: char) -> Self {
        self.right = ch;
        self
    }

    pub fn glyphs(mut self, left: char, fill: char, right: char) -> Self {
        self.left = left;
        self.fill = fill;
        self.right = right;
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

    pub fn bold(mut self) -> Self {
        self.attrs.bold = true;
        self
    }

    pub fn dim(mut self) -> Self {
        self.attrs.dim = true;
        self
    }

    pub fn width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    pub fn flex(mut self, flex: f32) -> Self {
        self.flex = flex;
        self
    }

    pub fn build(self, factory: &WidgetFactory, theme: &Theme) -> WidgetNode {
        factory.wrap(|id| DividerWidget {
            id,
            props: LayoutProps {
                width: self.width,
                flex: self.flex,
                ..Default::default()
            },
            style: DividerStyle {
                left: self.left,
                fill: self.fill,
                right: self.right,
                fg: self.fg.unwrap_or(theme.border()),
                bg: self.bg.unwrap_or(theme.surface()),
                attrs: self.attrs,
            },
        })
    }
}
