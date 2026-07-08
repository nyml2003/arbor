use crate::{HostKind, HostNode, LayoutNode, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PaintStyle {
    pub foreground: Option<PaintColor>,
    pub background: Option<PaintColor>,
    pub attrs: PaintAttrs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintColor {
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaintAttrs {
    bits: u8,
}

impl PaintAttrs {
    pub const BOLD: Self = Self { bits: 0b0000_0001 };
    pub const UNDERLINE: Self = Self { bits: 0b0000_0010 };
    pub const REVERSED: Self = Self { bits: 0b0000_0100 };

    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.bits & other.bits) == other.bits
    }
}

impl Default for PaintAttrs {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaintPrimitive {
    FillRect {
        rect: Rect,
        style: PaintStyle,
    },
    TextRun {
        x: u16,
        y: u16,
        text: String,
    },
    Border {
        rect: Rect,
        style: PaintStyle,
    },
    Cursor {
        x: u16,
        y: u16,
    },
    Clip {
        rect: Rect,
        children: Vec<PaintPrimitive>,
    },
    Layer {
        z_index: i16,
        children: Vec<PaintPrimitive>,
    },
}

pub fn paint_tree<Action>(host: &HostNode<Action>, layout: &[LayoutNode]) -> Vec<PaintPrimitive> {
    let mut paint = Vec::new();
    paint_node(host, layout, &mut paint);
    paint
}

fn paint_node<Action>(
    host: &HostNode<Action>,
    layout: &[LayoutNode],
    paint: &mut Vec<PaintPrimitive>,
) {
    if host.kind == HostKind::Text {
        if let (Some(text), Some(layout_node)) = (
            host.text.as_ref(),
            layout.iter().find(|node| node.host_id == host.id),
        ) {
            let text = text
                .chars()
                .take(usize::from(layout_node.rect.width))
                .collect::<String>();
            paint.push(PaintPrimitive::TextRun {
                x: layout_node.rect.x,
                y: layout_node.rect.y,
                text,
            });
        }
    }

    for child in &host.children {
        paint_node(child, layout, paint);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{layout_tree, lower_element, text, Size};

    #[test]
    fn text_paint_produces_text_run() {
        let element = text::<()>("hello");
        let host = lower_element(&element);
        let layout = layout_tree(&host, Size::new(10, 2));
        let paint = paint_tree(&host, &layout);

        assert_eq!(
            paint,
            vec![PaintPrimitive::TextRun {
                x: 0,
                y: 0,
                text: "hello".to_string(),
            }]
        );
    }

    #[test]
    fn paint_primitives_are_backend_independent() {
        let primitives = vec![
            PaintPrimitive::FillRect {
                rect: Rect::new(0, 0, 4, 2),
                style: PaintStyle::default(),
            },
            PaintPrimitive::Border {
                rect: Rect::new(0, 0, 4, 2),
                style: PaintStyle::default(),
            },
            PaintPrimitive::Cursor { x: 1, y: 1 },
            PaintPrimitive::Clip {
                rect: Rect::new(0, 0, 2, 1),
                children: vec![PaintPrimitive::TextRun {
                    x: 0,
                    y: 0,
                    text: "hello".to_string(),
                }],
            },
            PaintPrimitive::Layer {
                z_index: 1,
                children: Vec::new(),
            },
        ];

        assert_eq!(primitives.len(), 5);
    }
}
