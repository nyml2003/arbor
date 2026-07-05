use crate::button::widget::{ButtonStyle, ButtonWidget};
use crate::widget_factory::WidgetFactory;
use arbor_tui_domain::layout::{LayoutProps, RectOffset};
use arbor_tui_domain::signal::{ReadSignal, Signal};
use arbor_tui_domain::widget::WidgetNode;

pub struct Button {
    label: ReadSignal<String>,
    style: ButtonStyle,
    padding: RectOffset,
    width: Option<u16>,
    on_click: Option<Box<dyn Fn()>>,
}

impl Button {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: ReadSignal::constant(label.into()),
            style: ButtonStyle::Default,
            padding: RectOffset::default(),
            width: None,
            on_click: None,
        }
    }
    pub fn label_signal(mut self, signal: ReadSignal<String>) -> Self {
        self.label = signal;
        self
    }
    pub fn label_from(mut self, signal: &Signal<String>) -> Self {
        self.label = signal.read_only();
        self
    }
    pub fn on_click(mut self, f: impl Fn() + 'static) -> Self {
        self.on_click = Some(Box::new(f));
        self
    }
    pub fn style(mut self, s: ButtonStyle) -> Self {
        self.style = s;
        self
    }
    pub fn primary(mut self) -> Self {
        self.style = ButtonStyle::Primary;
        self
    }
    pub fn danger(mut self) -> Self {
        self.style = ButtonStyle::Danger;
        self
    }
    pub fn padding(mut self, p: RectOffset) -> Self {
        self.padding = p;
        self
    }
    pub fn width(mut self, w: u16) -> Self {
        self.width = Some(w);
        self
    }
    pub fn build(
        self,
        factory: &WidgetFactory,
        _theme: &arbor_tui_domain::theme::Theme,
    ) -> WidgetNode {
        factory.wrap(|id| ButtonWidget {
            id,
            props: LayoutProps {
                padding: self.padding,
                width: self.width,
                ..Default::default()
            },
            label: self.label,
            style: self.style,
            on_click: self.on_click,
        })
    }
}
