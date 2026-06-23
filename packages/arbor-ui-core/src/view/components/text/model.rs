use super::{TextProps, TextStyle};
use crate::view::components::{Align, ComponentBase, ComponentNode};

#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    pub props: TextProps,
}

impl ComponentNode for Text {
    fn base(&self) -> &ComponentBase {
        &self.props.base
    }
}

impl Text {
    pub fn content(&self) -> &str {
        self.props.content.as_str()
    }

    pub fn style(&self) -> TextStyle {
        self.props.style
    }

    pub fn align(&self) -> Align {
        self.props.align
    }
}
