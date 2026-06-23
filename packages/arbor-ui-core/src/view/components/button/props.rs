use super::{ButtonIntent, ButtonState, RippleVisual};
use crate::geometry::Rect;
use crate::view::components::{ComponentBase, ComponentProps};

#[derive(Debug, Clone, PartialEq)]
pub struct ButtonProps {
    pub base: ComponentBase,
    pub state: ButtonState,
    pub intent: ButtonIntent,
    pub ripples: Vec<RippleVisual>,
}

impl ButtonProps {
    pub fn new(id: impl Into<String>, rect: Rect) -> Self {
        Self {
            base: ComponentBase::new(id, rect),
            state: ButtonState::Normal,
            intent: ButtonIntent::Standard,
            ripples: Vec::new(),
        }
    }
}

impl ComponentProps for ButtonProps {
    fn base(&self) -> &ComponentBase {
        &self.base
    }
}
