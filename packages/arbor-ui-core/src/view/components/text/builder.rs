use super::{Text, TextProps, TextWeight};
use crate::geometry::Rect;
use crate::theme::ColorToken;
use crate::view::components::{Align, Primitive};

pub fn text(id: impl Into<String>, rect: Rect) -> TextBuilder {
    TextBuilder {
        props: TextProps::new(id, rect),
    }
}

#[derive(Debug, Clone)]
pub struct TextBuilder {
    props: TextProps,
}

impl TextBuilder {
    pub fn content(self, content: impl Into<String>) -> Self {
        Self {
            props: TextProps {
                content: content.into(),
                ..self.props
            },
        }
    }

    pub fn color(self, color: ColorToken) -> Self {
        Self {
            props: TextProps {
                style: super::TextStyle {
                    color,
                    ..self.props.style
                },
                ..self.props
            },
        }
    }

    pub fn size(self, size: f32) -> Self {
        Self {
            props: TextProps {
                style: super::TextStyle {
                    size,
                    ..self.props.style
                },
                ..self.props
            },
        }
    }

    pub fn weight(self, weight: TextWeight) -> Self {
        Self {
            props: TextProps {
                style: super::TextStyle {
                    weight,
                    ..self.props.style
                },
                ..self.props
            },
        }
    }

    pub fn align(self, align: Align) -> Self {
        Self {
            props: TextProps {
                align,
                ..self.props
            },
        }
    }

    pub fn build(self) -> Primitive {
        Primitive::Text(Text { props: self.props })
    }
}
