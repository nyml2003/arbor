use crate::text::widget::{TextStyle, TextWidget};
use crate::widget_factory::WidgetFactory;
use arbor_tui_domain::cell::{AnsiColor, Attrs};
use arbor_tui_domain::layout::{LayoutProps, RectOffset};
use arbor_tui_domain::signal::{ReadSignal, Signal};
use arbor_tui_domain::text::{TruncateStrategy, WrapStrategy};
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::WidgetNode;

pub struct Text {
    content: ReadSignal<String>,
    fg: Option<AnsiColor>,
    bg: Option<AnsiColor>,
    attrs: Attrs,
    padding: RectOffset,
    flex: f32,
    width: Option<u16>,
    wrap: WrapStrategy,
    truncate: TruncateStrategy,
    style_signal: Option<ReadSignal<TextStyle>>,
}

impl Text {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: ReadSignal::constant(content.into()),
            fg: None,
            bg: None,
            attrs: Attrs::default(),
            padding: RectOffset::default(),
            flex: 0.0,
            width: None,
            wrap: WrapStrategy::None,
            truncate: TruncateStrategy::End,
            style_signal: None,
        }
    }
    pub fn content_signal(mut self, signal: ReadSignal<String>) -> Self {
        self.content = signal;
        self
    }
    pub fn content_from(mut self, signal: &Signal<String>) -> Self {
        self.content = signal.read_only();
        self
    }
    pub fn style_signal(mut self, signal: ReadSignal<TextStyle>) -> Self {
        self.style_signal = Some(signal);
        self
    }
    pub fn style_from(mut self, signal: &Signal<TextStyle>) -> Self {
        self.style_signal = Some(signal.read_only());
        self
    }
    pub fn fg(mut self, c: AnsiColor) -> Self {
        self.fg = Some(c);
        self
    }
    pub fn bg(mut self, c: AnsiColor) -> Self {
        self.bg = Some(c);
        self
    }
    pub fn bold(mut self) -> Self {
        self.attrs.bold = true;
        self
    }
    pub fn italic(mut self) -> Self {
        self.attrs.italic = true;
        self
    }
    pub fn dim(mut self) -> Self {
        self.attrs.dim = true;
        self
    }
    pub fn underline(mut self) -> Self {
        self.attrs.underline = true;
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
    pub fn width(mut self, w: u16) -> Self {
        self.width = Some(w);
        self
    }

    pub fn build(self, factory: &WidgetFactory, t: &Theme) -> WidgetNode {
        let style = TextStyle {
            fg: self.fg.unwrap_or(t.text()),
            bg: self.bg.unwrap_or(t.surface()),
            attrs: self.attrs,
        };
        let style_signal = self
            .style_signal
            .unwrap_or_else(|| ReadSignal::constant(style));
        factory.wrap(|id| TextWidget {
            id,
            props: LayoutProps {
                padding: self.padding,
                flex: self.flex,
                width: self.width,
                ..Default::default()
            },
            text: self.content,
            style: style_signal,
            wrap: self.wrap,
            truncate: self.truncate,
        })
    }
}
