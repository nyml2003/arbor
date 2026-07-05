use crate::tabs::widget::{TabDef, TabsWidget};
use crate::widget_factory::WidgetFactory;
use arbor_tui_domain::layout::{LayoutProps, RectOffset};
use arbor_tui_domain::widget::WidgetNode;

pub struct Tabs {
    tabs: Vec<TabDef>,
    active: usize,
    padding: RectOffset,
    flex: f32,
}

impl Tabs {
    pub fn new(active: usize) -> Self {
        Self {
            tabs: vec![],
            active,
            padding: RectOffset::default(),
            flex: 0.0,
        }
    }
    pub fn tabs(mut self, t: Vec<TabDef>) -> Self {
        self.tabs = t;
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
    pub fn build(
        self,
        factory: &WidgetFactory,
        _theme: &arbor_tui_domain::theme::Theme,
    ) -> WidgetNode {
        factory.wrap(|id| TabsWidget {
            id,
            props: LayoutProps {
                padding: self.padding,
                flex: self.flex,
                ..Default::default()
            },
            tabs: self.tabs,
            active: self.active,
            on_switch: None,
        })
    }
}
