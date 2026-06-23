use super::{Image, ImageProps};
use crate::geometry::Rect;
use crate::theme::ColorToken;
use crate::view::components::Primitive;

pub fn image(id: impl Into<String>, rect: Rect) -> ImageBuilder {
    ImageBuilder {
        props: ImageProps::new(id, rect),
    }
}

#[derive(Debug, Clone)]
pub struct ImageBuilder {
    props: ImageProps,
}

impl ImageBuilder {
    pub fn tint(self, tint: ColorToken) -> Self {
        Self {
            props: ImageProps {
                tint: Some(tint),
                ..self.props
            },
        }
    }

    pub fn opacity(self, opacity: f32) -> Self {
        Self {
            props: ImageProps {
                opacity,
                ..self.props
            },
        }
    }

    pub fn build(self) -> Primitive {
        Primitive::Image(Image { props: self.props })
    }
}
