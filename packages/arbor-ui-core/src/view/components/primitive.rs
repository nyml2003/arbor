use super::{Button, ComponentBase, ComponentNode, Image, Row, Surface, Text};

#[derive(Debug, Clone, PartialEq)]
pub enum Primitive {
    Surface(Surface),
    Row(Row),
    Button(Button),
    Text(Text),
    Image(Image),
}

impl ComponentNode for Primitive {
    fn base(&self) -> &ComponentBase {
        match self {
            Primitive::Surface(surface) => surface.base(),
            Primitive::Row(row) => row.base(),
            Primitive::Button(button) => button.base(),
            Primitive::Text(text) => text.base(),
            Primitive::Image(image) => image.base(),
        }
    }
}
