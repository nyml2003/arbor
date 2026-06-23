use super::{Row, RowProps};
use crate::geometry::Rect;
use crate::view::components::{Align, Primitive};

pub fn row(id: impl Into<String>, rect: Rect) -> RowBuilder {
    RowBuilder {
        props: RowProps::new(id, rect),
    }
}

#[derive(Debug, Clone)]
pub struct RowBuilder {
    props: RowProps,
}

impl RowBuilder {
    pub fn gap(self, gap: f32) -> Self {
        Self {
            props: RowProps { gap, ..self.props },
        }
    }

    pub fn align(self, align: Align) -> Self {
        Self {
            props: RowProps {
                align,
                ..self.props
            },
        }
    }

    pub fn children(self, children: impl IntoIterator<Item = Primitive>) -> Self {
        Self {
            props: RowProps {
                children: children.into_iter().collect(),
                ..self.props
            },
        }
    }

    pub fn build(self) -> Primitive {
        Primitive::Row(Row { props: self.props })
    }
}
