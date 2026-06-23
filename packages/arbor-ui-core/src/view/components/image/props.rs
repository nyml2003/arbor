use crate::geometry::Rect;
use crate::theme::ColorToken;
use crate::view::components::{ComponentBase, ComponentProps};

#[derive(Debug, Clone, PartialEq)]
pub struct ImageProps {
    pub base: ComponentBase,
    pub tint: Option<ColorToken>,
    pub opacity: f32,
}

impl ImageProps {
    pub fn new(id: impl Into<String>, rect: Rect) -> Self {
        Self {
            base: ComponentBase::new(id, rect),
            tint: None,
            opacity: 1.0,
        }
    }
}

impl ComponentProps for ImageProps {
    fn base(&self) -> &ComponentBase {
        &self.base
    }
}
