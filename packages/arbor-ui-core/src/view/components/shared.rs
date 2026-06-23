use crate::theme::ColorToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Align {
    Start,
    Center,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Border {
    pub color: ColorToken,
    pub width: f32,
}
