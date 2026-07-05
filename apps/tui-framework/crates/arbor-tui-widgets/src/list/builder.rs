use crate::list::widget::ListWidget;
use crate::widget_factory::WidgetFactory;
use arbor_tui_domain::layout::{LayoutProps, RectOffset};
use arbor_tui_domain::widget::WidgetNode;

pub struct List {
    items: Vec<String>,
    padding: RectOffset,
    flex: f32,
    on_select: Option<Box<dyn Fn(Option<usize>)>>,
}

impl Default for List {
    fn default() -> Self {
        Self::new()
    }
}

impl List {
    pub fn new() -> Self {
        Self {
            items: vec![],
            padding: RectOffset::default(),
            flex: 0.0,
            on_select: None,
        }
    }
    pub fn items(mut self, items: Vec<String>) -> Self {
        self.items = items;
        self
    }
    pub fn flex(mut self, f: f32) -> Self {
        self.flex = f;
        self
    }
    pub fn padding(mut self, p: RectOffset) -> Self {
        self.padding = p;
        self
    }
    pub fn on_select(mut self, f: impl Fn(Option<usize>) + 'static) -> Self {
        self.on_select = Some(Box::new(f));
        self
    }
    pub fn build(
        self,
        factory: &WidgetFactory,
        _theme: &arbor_tui_domain::theme::Theme,
    ) -> WidgetNode {
        factory.wrap(|id| ListWidget {
            id,
            props: LayoutProps {
                padding: self.padding,
                flex: self.flex,
                ..Default::default()
            },
            items: self.items,
            selected: None,
            scroll_offset: std::cell::Cell::new(0),
            viewport_rows: std::cell::Cell::new(1),
            on_select: self.on_select,
            on_scroll: None,
            render_item: None,
        })
    }
}
