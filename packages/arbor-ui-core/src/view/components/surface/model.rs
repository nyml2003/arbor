use crate::theme::ColorToken;
use crate::view::components::{Border, ComponentBase, ComponentNode, Primitive};

use super::SurfaceProps;

#[derive(Debug, Clone, PartialEq)]
pub struct Surface {
    pub props: SurfaceProps,
}

impl ComponentNode for Surface {
    fn base(&self) -> &ComponentBase {
        &self.props.base
    }
}

impl Surface {
    pub fn background(&self) -> ColorToken {
        self.props.background
    }

    pub fn border(&self) -> Option<Border> {
        self.props.border
    }

    pub fn radius(&self) -> f32 {
        self.props.radius
    }

    pub fn children(&self) -> &[Primitive] {
        &self.props.children
    }
}
