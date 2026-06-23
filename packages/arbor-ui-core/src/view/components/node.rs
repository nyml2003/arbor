use crate::geometry::Rect;

#[derive(Debug, Clone, PartialEq)]
pub struct ComponentBase {
    pub id: String,
    pub rect: Rect,
}

impl ComponentBase {
    pub fn new(id: impl Into<String>, rect: Rect) -> Self {
        Self {
            id: id.into(),
            rect,
        }
    }
}

pub trait ComponentNode {
    fn base(&self) -> &ComponentBase;

    fn id(&self) -> &str {
        self.base().id.as_str()
    }

    fn rect(&self) -> Rect {
        self.base().rect
    }
}

pub trait ComponentProps {
    fn base(&self) -> &ComponentBase;
}
