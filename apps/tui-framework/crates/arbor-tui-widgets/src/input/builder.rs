use arbor_tui_primitives::layout::{LayoutProps};
use arbor_tui_widget::widget::WidgetNode;
use crate::input::widget::InputWidget;
use crate::widget_manager::WidgetManager;

pub struct Input {
    placeholder: String,
    password: bool,
    width: Option<u16>,
}

impl Input {
    pub fn new() -> Self { Self { placeholder: String::new(), password: false, width: None } }
    pub fn placeholder(mut self, p: impl Into<String>) -> Self { self.placeholder = p.into(); self }
    pub fn password(mut self) -> Self { self.password = true; self }
    pub fn width(mut self, w: u16) -> Self { self.width = Some(w); self }
    pub fn build(self, wm: &WidgetManager, _t: &arbor_tui_render::theme::Theme) -> WidgetNode {
        wm.wrap(|id| InputWidget { id, props: LayoutProps { width: self.width, ..Default::default() }, buffer: String::new(), cursor: 0, placeholder: self.placeholder, password: self.password, on_change: None, on_submit: None })
    }
}
