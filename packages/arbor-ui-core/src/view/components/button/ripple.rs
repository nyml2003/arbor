use crate::geometry::Point;
use crate::theme::ColorToken;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RippleVisual {
    pub origin: Point,
    pub radius: f32,
    pub opacity: f32,
    pub color: ColorToken,
}
