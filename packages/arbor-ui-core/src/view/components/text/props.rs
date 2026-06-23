use super::{TextStyle, TextWeight};
use crate::geometry::Rect;
use crate::theme::ColorToken;
use crate::view::components::{Align, ComponentBase, ComponentProps};

#[derive(Debug, Clone, PartialEq)]
pub struct TextProps {
    pub base: ComponentBase,
    pub content: String,
    pub style: TextStyle,
    pub align: Align,
}

impl TextProps {
    pub fn new(id: impl Into<String>, rect: Rect) -> Self {
        Self {
            base: ComponentBase::new(id, rect),
            content: String::new(),
            style: TextStyle {
                color: ColorToken::TextPrimary,
                size: 14.0,
                weight: TextWeight::Regular,
            },
            align: Align::Start,
        }
    }
}

impl ComponentProps for TextProps {
    fn base(&self) -> &ComponentBase {
        &self.base
    }
}
