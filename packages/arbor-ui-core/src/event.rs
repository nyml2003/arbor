use crate::geometry::Point;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PointerEvent {
    Move(Point),
    Down(Point),
    Up(Point),
    Cancel,
}
