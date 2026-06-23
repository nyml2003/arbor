use crate::theme::ColorToken;
use crate::view::components::{ComponentBase, ComponentNode};

use super::ImageProps;

#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    pub props: ImageProps,
}

impl ComponentNode for Image {
    fn base(&self) -> &ComponentBase {
        &self.props.base
    }
}

impl Image {
    pub fn tint(&self) -> Option<ColorToken> {
        self.props.tint
    }

    pub fn opacity(&self) -> f32 {
        self.props.opacity
    }
}
