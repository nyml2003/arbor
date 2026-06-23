use super::{ButtonProps, ButtonState, RippleVisual};
use crate::view::components::{ComponentBase, ComponentNode, Primitive};

#[derive(Debug, Clone, PartialEq)]
pub struct Button {
    pub props: ButtonProps,
    pub content: Box<Primitive>,
}

impl ComponentNode for Button {
    fn base(&self) -> &ComponentBase {
        &self.props.base
    }
}

impl Button {
    pub fn state(&self) -> ButtonState {
        self.props.state
    }

    pub fn ripples(&self) -> &[RippleVisual] {
        &self.props.ripples
    }
}
