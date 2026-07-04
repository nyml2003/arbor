use arbor_tui_primitives::layout::LayoutProps;
use arbor_tui_widget::widget::WidgetNode;
use crate::input::widget::InputWidget;
use crate::widget_manager::WidgetManager;

pub struct Input {
    placeholder: String,
    password: bool,
    width: Option<u16>,
    on_change: Option<Box<dyn Fn(String)>>,
    on_submit: Option<Box<dyn Fn(String)>>,
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}

impl Input {
    pub fn new() -> Self { Self { placeholder: String::new(), password: false, width: None, on_change: None, on_submit: None } }
    pub fn placeholder(mut self, p: impl Into<String>) -> Self { self.placeholder = p.into(); self }
    pub fn password(mut self) -> Self { self.password = true; self }
    pub fn width(mut self, w: u16) -> Self { self.width = Some(w); self }
    pub fn on_change(mut self, f: impl Fn(String) + 'static) -> Self { self.on_change = Some(Box::new(f)); self }
    pub fn on_submit(mut self, f: impl Fn(String) + 'static) -> Self { self.on_submit = Some(Box::new(f)); self }
    pub fn build(self, wm: &WidgetManager, _t: &arbor_tui_render::theme::Theme) -> WidgetNode {
        wm.wrap(|id| InputWidget {
            id, props: LayoutProps { width: self.width, ..Default::default() },
            buffer: String::new(), cursor: 0,
            placeholder: self.placeholder, password: self.password,
            on_change: self.on_change, on_submit: self.on_submit,
        })
    }
}
