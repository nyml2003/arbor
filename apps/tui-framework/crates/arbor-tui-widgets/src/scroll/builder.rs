use arbor_tui_primitives::layout::{LayoutProps, RectOffset};
use arbor_tui_reactive::signal::ReadSignal;
use arbor_tui_widget::widget::WidgetNode;
use crate::scroll::widget::ScrollViewWidget;
use crate::widget_manager::WidgetManager;

pub struct Scroll {
    child: Option<WidgetNode>,
    padding: RectOffset,
    flex: f32,
}

impl Scroll {
    pub fn new() -> Self { Self { child: None, padding: RectOffset::default(), flex: 0.0 } }
    pub fn child(mut self, c: WidgetNode) -> Self { self.child = Some(c); self }
    pub fn padding(mut self, p: RectOffset) -> Self { self.padding = p; self }
    pub fn flex(mut self, f: f32) -> Self { self.flex = f; self }
    pub fn build(self, wm: &WidgetManager, _t: &arbor_tui_render::theme::Theme) -> WidgetNode {
        let child = self.child.expect("Scroll::child must be set before build");
        wm.wrap(|id| ScrollViewWidget { id, props: LayoutProps { padding: self.padding, flex: self.flex, ..Default::default() }, child: Box::new(child), scroll_x: ReadSignal::constant(0), scroll_y: ReadSignal::constant(0), on_scroll: None })
    }
}
