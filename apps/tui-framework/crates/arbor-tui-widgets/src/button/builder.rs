use arbor_tui_primitives::layout::{LayoutProps, RectOffset};
use arbor_tui_reactive::signal::ReadSignal;
use arbor_tui_widget::widget::WidgetNode;
use crate::button::widget::{ButtonStyle, ButtonWidget};
use crate::widget_manager::WidgetManager;

pub struct Button {
    label: String,
    style: ButtonStyle,
    padding: RectOffset,
    width: Option<u16>,
}

impl Button {
    pub fn new(label: impl Into<String>) -> Self { Self { label: label.into(), style: ButtonStyle::Default, padding: RectOffset::default(), width: None } }
    pub fn style(mut self, s: ButtonStyle) -> Self { self.style = s; self }
    pub fn primary(mut self) -> Self { self.style = ButtonStyle::Primary; self }
    pub fn danger(mut self) -> Self { self.style = ButtonStyle::Danger; self }
    pub fn padding(mut self, p: RectOffset) -> Self { self.padding = p; self }
    pub fn width(mut self, w: u16) -> Self { self.width = Some(w); self }
    pub fn build(self, wm: &WidgetManager, _t: &arbor_tui_render::theme::Theme) -> WidgetNode {
        wm.wrap(|id| ButtonWidget { id, props: LayoutProps { padding: self.padding, width: self.width, ..Default::default() }, label: ReadSignal::constant(self.label), style: self.style, on_click: None })
    }
}
