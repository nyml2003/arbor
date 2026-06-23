use crate::view::components::{ComponentBase, ComponentNode, Primitive};

use super::RowProps;

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub props: RowProps,
}

impl ComponentNode for Row {
    fn base(&self) -> &ComponentBase {
        &self.props.base
    }
}

impl Row {
    pub fn children(&self) -> &[Primitive] {
        &self.props.children
    }
}
