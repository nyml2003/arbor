use crate::rich_text::widget::RichTextWidget;
use crate::widget_factory::WidgetFactory;
use arbor_tui_primitives::cell::{Cell, Span};
use arbor_tui_primitives::layout::{LayoutProps, RectOffset};
use arbor_tui_widget::widget::WidgetNode;

pub struct RichText {
    lines: Vec<Vec<Span>>,
    padding: RectOffset,
    flex: f32,
    bg: Option<Cell>,
}

impl Default for RichText {
    fn default() -> Self {
        Self::new()
    }
}

impl RichText {
    pub fn new() -> Self {
        Self {
            lines: vec![],
            padding: RectOffset::default(),
            flex: 0.0,
            bg: None,
        }
    }
    pub fn line(mut self, spans: Vec<Span>) -> Self {
        self.lines.push(spans);
        self
    }
    pub fn lines(mut self, all: Vec<Vec<Span>>) -> Self {
        self.lines = all;
        self
    }
    pub fn padding(mut self, p: RectOffset) -> Self {
        self.padding = p;
        self
    }
    pub fn flex(mut self, f: f32) -> Self {
        self.flex = f;
        self
    }
    pub fn bg(mut self, cell: Cell) -> Self {
        self.bg = Some(cell);
        self
    }
    pub fn build(self, factory: &WidgetFactory, t: &arbor_tui_render::theme::Theme) -> WidgetNode {
        let default_bg = Cell {
            bg: t.surface(),
            ..Default::default()
        };
        factory.wrap(|id| RichTextWidget {
            id,
            props: LayoutProps {
                padding: self.padding,
                flex: self.flex,
                ..Default::default()
            },
            lines: self.lines,
            clip: false,
            bg: self.bg.unwrap_or(default_bg),
        })
    }
}
