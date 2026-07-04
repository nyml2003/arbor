use arbor_tui_primitives::layout::{LayoutProps, RectOffset};
use arbor_tui_widget::widget::WidgetNode;
use crate::tabs::widget::{TabDef, TabsWidget};
use crate::widget_manager::WidgetManager;

pub struct Tabs {
    tabs: Vec<TabDef>,
    active: usize,
    padding: RectOffset,
    flex: f32,
}

impl Tabs {
    pub fn new(active: usize) -> Self { Self { tabs: vec![], active, padding: RectOffset::default(), flex: 0.0 } }
    pub fn tabs(mut self, t: Vec<TabDef>) -> Self { self.tabs = t; self }
    pub fn flex(mut self, f: f32) -> Self { self.flex = f; self }
    pub fn padding(mut self, p: RectOffset) -> Self { self.padding = p; self }
    pub fn build(self, wm: &WidgetManager, _t: &arbor_tui_render::theme::Theme) -> WidgetNode {
        wm.wrap(|id| TabsWidget { id, props: LayoutProps { padding: self.padding, flex: self.flex, ..Default::default() }, tabs: self.tabs, active: self.active, on_switch: None })
    }
}
