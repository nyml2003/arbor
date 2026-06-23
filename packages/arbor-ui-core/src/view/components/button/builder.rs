use super::{Button, ButtonIntent, ButtonProps, ButtonState, RippleVisual};
use crate::geometry::{Point, Rect};
use crate::theme::ColorToken;
use crate::view::components::{text, ComponentProps, Primitive};

pub fn button(id: impl Into<String>, rect: Rect) -> ButtonBuilder {
    ButtonBuilder {
        props: ButtonProps::new(id, rect),
        content: None,
    }
}

pub fn ripple(origin: Point, radius: f32, opacity: f32, color: ColorToken) -> RippleVisual {
    RippleVisual {
        origin,
        radius,
        opacity,
        color,
    }
}

#[derive(Debug, Clone)]
pub struct ButtonBuilder {
    props: ButtonProps,
    content: Option<Box<Primitive>>,
}

impl ButtonBuilder {
    pub fn state(self, state: ButtonState) -> Self {
        let Self { props, content } = self;
        Self {
            props: ButtonProps { state, ..props },
            content,
        }
    }

    pub fn intent(self, intent: ButtonIntent) -> Self {
        let Self { props, content } = self;
        Self {
            props: ButtonProps { intent, ..props },
            content,
        }
    }

    pub fn ripples(self, ripples: impl IntoIterator<Item = RippleVisual>) -> Self {
        let Self { props, content } = self;
        Self {
            props: ButtonProps {
                ripples: ripples.into_iter().collect(),
                ..props
            },
            content,
        }
    }

    pub fn child(self, content: Primitive) -> Self {
        let Self { props, .. } = self;
        Self {
            props,
            content: Some(Box::new(content)),
        }
    }

    pub fn build(self) -> Primitive {
        let rect = self.props.base().rect;
        let id = self.props.base().id.clone();
        let content = self
            .content
            .unwrap_or_else(|| Box::new(text(format!("{id}-content"), rect).build()));

        Primitive::Button(Button {
            props: self.props,
            content,
        })
    }
}
