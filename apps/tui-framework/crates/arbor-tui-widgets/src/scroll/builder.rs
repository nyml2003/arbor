use crate::scroll::widget::ScrollViewWidget;
use crate::widget_factory::WidgetFactory;
use arbor_tui_primitives::layout::{LayoutProps, RectOffset};
use arbor_tui_reactive::signal::ReadSignal;
use arbor_tui_widget::widget::WidgetNode;

pub struct Scroll {
    child: Option<WidgetNode>,
    padding: RectOffset,
    flex: f32,
    content_h: u16,
    scroll_y: Option<ReadSignal<u16>>,
}

impl Default for Scroll {
    fn default() -> Self {
        Self::new()
    }
}

impl Scroll {
    pub fn new() -> Self {
        Self {
            child: None,
            padding: RectOffset::default(),
            flex: 0.0,
            content_h: 0,
            scroll_y: None,
        }
    }
    pub fn child(mut self, c: WidgetNode) -> Self {
        self.child = Some(c);
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
    pub fn content_h(mut self, h: u16) -> Self {
        self.content_h = h;
        self
    }
    pub fn scroll_y(mut self, s: ReadSignal<u16>) -> Self {
        self.scroll_y = Some(s);
        self
    }
    pub fn build(
        self,
        factory: &WidgetFactory,
        _theme: &arbor_tui_render::theme::Theme,
    ) -> WidgetNode {
        let child = self.child.expect("Scroll::child must be set before build");
        factory.wrap(|id| ScrollViewWidget {
            id,
            props: LayoutProps {
                padding: self.padding,
                flex: self.flex,
                ..Default::default()
            },
            child: Box::new(child),
            scroll_x: ReadSignal::constant(0),
            scroll_y: self.scroll_y.unwrap_or_else(|| ReadSignal::constant(0)),
            content_h: self.content_h,
            on_scroll: None,
        })
    }
}
