use super::{Surface, SurfaceProps};
use crate::geometry::Rect;
use crate::theme::ColorToken;
use crate::view::components::{Border, Primitive};

pub fn surface(id: impl Into<String>, rect: Rect) -> SurfaceBuilder {
    SurfaceBuilder {
        props: SurfaceProps::new(id, rect),
    }
}

#[derive(Debug, Clone)]
pub struct SurfaceBuilder {
    props: SurfaceProps,
}

impl SurfaceBuilder {
    pub fn background(self, background: ColorToken) -> Self {
        Self {
            props: SurfaceProps {
                background,
                ..self.props
            },
        }
    }

    pub fn border(self, color: ColorToken, width: f32) -> Self {
        Self {
            props: SurfaceProps {
                border: Some(Border { color, width }),
                ..self.props
            },
        }
    }

    pub fn radius(self, radius: f32) -> Self {
        Self {
            props: SurfaceProps {
                radius,
                ..self.props
            },
        }
    }

    pub fn children(self, children: impl IntoIterator<Item = Primitive>) -> Self {
        Self {
            props: SurfaceProps {
                children: children.into_iter().collect(),
                ..self.props
            },
        }
    }

    pub fn build(self) -> Primitive {
        Primitive::Surface(Surface { props: self.props })
    }
}
