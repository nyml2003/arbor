pub mod components;

use crate::geometry::Rect;
use components::Primitive;

#[derive(Debug, Clone, PartialEq)]
pub struct ViewSnapshot {
    pub surface_rect: Rect,
    pub primitive_tree: Primitive,
}
