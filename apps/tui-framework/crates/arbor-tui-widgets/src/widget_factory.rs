use std::cell::Cell;

use arbor_tui_primitives::widget_id::WidgetId;
use arbor_tui_widget::widget::{Widget, WidgetNode};

pub struct WidgetFactory {
    next_id: Cell<u64>,
}

impl WidgetFactory {
    pub fn new() -> Self {
        Self {
            next_id: Cell::new(1),
        }
    }

    pub fn alloc_id(&self) -> WidgetId {
        let id = WidgetId(self.next_id.get());
        self.next_id.set(id.0 + 1);
        id
    }

    pub fn wrap<W: Widget + 'static>(&self, f: impl FnOnce(WidgetId) -> W) -> WidgetNode {
        WidgetNode::new(f(self.alloc_id()))
    }
}

impl Default for WidgetFactory {
    fn default() -> Self {
        Self::new()
    }
}
