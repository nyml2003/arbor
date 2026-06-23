use crate::geometry::Rect;
use crate::view::components::{Align, ComponentBase, ComponentProps, Primitive};

#[derive(Debug, Clone, PartialEq)]
pub struct RowProps {
    pub base: ComponentBase,
    pub gap: f32,
    pub align: Align,
    pub children: Vec<Primitive>,
}

impl RowProps {
    pub fn new(id: impl Into<String>, rect: Rect) -> Self {
        Self {
            base: ComponentBase::new(id, rect),
            gap: 0.0,
            align: Align::Start,
            children: Vec::new(),
        }
    }
}

impl ComponentProps for RowProps {
    fn base(&self) -> &ComponentBase {
        &self.base
    }
}
