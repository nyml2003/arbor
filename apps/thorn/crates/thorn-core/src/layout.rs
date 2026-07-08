use crate::{Axis, HostKind, HostNode, HostNodeId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub width: u16,
    pub height: u16,
}

impl Size {
    pub const fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutNode {
    pub host_id: HostNodeId,
    pub rect: Rect,
}

pub fn layout_tree<Action>(host: &HostNode<Action>, size: Size) -> Vec<LayoutNode> {
    let mut layout = Vec::new();
    layout_node(host, Rect::new(0, 0, size.width, size.height), &mut layout);
    layout
}

fn layout_node<Action>(host: &HostNode<Action>, rect: Rect, layout: &mut Vec<LayoutNode>) {
    layout.push(LayoutNode {
        host_id: host.id,
        rect,
    });
    match host.kind {
        HostKind::Text => {}
        HostKind::View {
            axis: Axis::Vertical,
        } => layout_vertical_children(host, rect, layout),
        HostKind::View {
            axis: Axis::Horizontal,
        } => layout_horizontal_children(host, rect, layout),
    }
}

fn layout_vertical_children<Action>(
    host: &HostNode<Action>,
    rect: Rect,
    layout: &mut Vec<LayoutNode>,
) {
    let mut y = rect.y;
    let bottom = rect.y.saturating_add(rect.height);
    for child in &host.children {
        if y >= bottom {
            break;
        }
        let child_height = intrinsic_block_height(child).min(bottom.saturating_sub(y));
        layout_node(
            child,
            Rect::new(rect.x, y, rect.width, child_height),
            layout,
        );
        y = y.saturating_add(child_height);
    }
}

fn layout_horizontal_children<Action>(
    host: &HostNode<Action>,
    rect: Rect,
    layout: &mut Vec<LayoutNode>,
) {
    let mut x = rect.x;
    let right = rect.x.saturating_add(rect.width);
    for child in &host.children {
        if x >= right {
            break;
        }
        let child_width = intrinsic_inline_width(child).min(right.saturating_sub(x));
        layout_node(
            child,
            Rect::new(x, rect.y, child_width, rect.height),
            layout,
        );
        x = x.saturating_add(child_width);
    }
}

fn intrinsic_inline_width<Action>(host: &HostNode<Action>) -> u16 {
    match host.kind {
        HostKind::Text => host
            .text
            .as_deref()
            .map(|text| text.chars().count().min(usize::from(u16::MAX)) as u16)
            .unwrap_or(0),
        HostKind::View {
            axis: Axis::Horizontal,
        } => host.children.iter().fold(0u16, |width, child| {
            width.saturating_add(intrinsic_inline_width(child))
        }),
        HostKind::View {
            axis: Axis::Vertical,
        } => host
            .children
            .iter()
            .map(intrinsic_inline_width)
            .max()
            .unwrap_or(0),
    }
}

fn intrinsic_block_height<Action>(host: &HostNode<Action>) -> u16 {
    match host.kind {
        HostKind::Text => 1,
        HostKind::View {
            axis: Axis::Vertical,
        } => host.children.iter().fold(0u16, |height, child| {
            height.saturating_add(intrinsic_block_height(child))
        }),
        HostKind::View {
            axis: Axis::Horizontal,
        } => host
            .children
            .iter()
            .map(intrinsic_block_height)
            .max()
            .unwrap_or(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{column, lower_element, row, text};

    #[test]
    fn column_layout_stacks_children_vertically() {
        let element = column((text::<()>("a"), text::<()>("b")));
        let host = lower_element(&element);
        let layout = layout_tree(&host, Size::new(10, 5));

        assert_eq!(layout[1].rect, Rect::new(0, 0, 10, 1));
        assert_eq!(layout[2].rect, Rect::new(0, 1, 10, 1));
    }

    #[test]
    fn row_layout_stacks_children_horizontally() {
        let element = row((text::<()>("a"), text::<()>("bb")));
        let host = lower_element(&element);
        let layout = layout_tree(&host, Size::new(10, 3));

        assert_eq!(layout[1].rect, Rect::new(0, 0, 1, 3));
        assert_eq!(layout[2].rect, Rect::new(1, 0, 2, 3));
    }
}
