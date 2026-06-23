use crate::geometry::Rect;
use crate::theme::ColorToken;
use crate::view::components::{Border, ComponentBase, ComponentProps, Primitive};

#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceProps {
    pub base: ComponentBase,
    pub background: ColorToken,
    pub border: Option<Border>,
    pub radius: f32,
    pub children: Vec<Primitive>,
}

impl SurfaceProps {
    pub fn new(id: impl Into<String>, rect: Rect) -> Self {
        Self {
            base: ComponentBase::new(id, rect),
            background: ColorToken::Surface,
            border: None,
            radius: 0.0,
            children: Vec::new(),
        }
    }
}

impl ComponentProps for SurfaceProps {
    fn base(&self) -> &ComponentBase {
        &self.base
    }
}
