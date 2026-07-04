use arbor_tui_primitives::layout::{LayoutProps, RectOffset};
use arbor_tui_widget::widget::WidgetNode;
use crate::list::widget::ListWidget;
use crate::widget_manager::WidgetManager;

pub struct List {
    items: Vec<String>,
    padding: RectOffset,
    flex: f32,
}

impl Default for List {
    fn default() -> Self {
        Self::new()
    }
}

impl List {
    pub fn new() -> Self { Self { items: vec![], padding: RectOffset::default(), flex: 0.0 } }
    pub fn items(mut self, items: Vec<String>) -> Self { self.items = items; self }
    pub fn flex(mut self, f: f32) -> Self { self.flex = f; self }
    pub fn padding(mut self, p: RectOffset) -> Self { self.padding = p; self }
    pub fn build(self, wm: &WidgetManager, _t: &arbor_tui_render::theme::Theme) -> WidgetNode {
        wm.wrap(|id| ListWidget { id, props: LayoutProps { padding: self.padding, flex: self.flex, ..Default::default() }, items: self.items, selected: None, scroll_offset: 0, on_select: None, on_scroll: None, render_item: None })
    }
}
