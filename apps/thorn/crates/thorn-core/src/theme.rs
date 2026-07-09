use crate::PaintStyle;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Theme {
    pub canvas: PaintStyle,
}

impl Theme {
    pub const fn new(canvas: PaintStyle) -> Self {
        Self { canvas }
    }
}
