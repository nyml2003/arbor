use crate::theme::ColorToken;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    pub color: ColorToken,
    pub size: f32,
    pub weight: TextWeight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextWeight {
    Regular,
    Semibold,
}
