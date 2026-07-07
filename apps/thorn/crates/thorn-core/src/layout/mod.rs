use std::collections::HashMap;

use crate::view::{NodeId, NodeKind, PrimitiveNode, View};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl Rect {
    pub const fn new(x: u16, y: u16, w: u16, h: u16) -> Self {
        Self { x, y, w, h }
    }

    pub const fn contains(self, x: u16, y: u16) -> bool {
        x >= self.x
            && y >= self.y
            && x < self.x.saturating_add(self.w)
            && y < self.y.saturating_add(self.h)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Size {
    pub w: u16,
    pub h: u16,
}

impl Size {
    pub const fn new(w: u16, h: u16) -> Self {
        Self { w, h }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Edge {
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub left: u16,
}

impl Edge {
    pub const fn all(value: u16) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    pub const fn horizontal(self) -> u16 {
        self.left + self.right
    }

    pub const fn vertical(self) -> u16 {
        self.top + self.bottom
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Direction {
    Row,
    Column,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Justify {
    Start,
    Center,
    End,
    SpaceBetween,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Clone, Debug)]
pub struct LayoutStyle {
    pub direction: Direction,
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub min_width: u16,
    pub min_height: u16,
    pub flex: u16,
    pub padding: Edge,
    pub margin: Edge,
    pub gap: u16,
    pub justify: Justify,
    pub align: Align,
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            direction: Direction::Column,
            width: None,
            height: None,
            min_width: 0,
            min_height: 0,
            flex: 0,
            padding: Edge::default(),
            margin: Edge::default(),
            gap: 0,
            justify: Justify::Start,
            align: Align::Stretch,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LayoutInfo {
    pub rect: Rect,
    pub content_rect: Rect,
}

pub fn layout_tree<Action>(root: &View<Action>, rect: Rect) -> HashMap<NodeId, LayoutInfo> {
    let mut out = HashMap::new();
    layout_node(root.node(), rect, &mut out);
    out
}

pub fn measure_node<Action>(node: &PrimitiveNode<Action>) -> Size {
    let style = node.layout();
    let mut size = match node.kind() {
        NodeKind::Text => Size::new(node.text().as_deref().map(display_width).unwrap_or(0), 1),
        NodeKind::Panel | NodeKind::Row | NodeKind::Col => measure_container(node),
    };

    if matches!(node.kind(), NodeKind::Panel) {
        size.w = size.w.saturating_add(2);
        size.h = size.h.saturating_add(2);
    }

    if let Some(width) = style.width {
        size.w = width;
    }
    if let Some(height) = style.height {
        size.h = height;
    }
    size.w = size.w.max(style.min_width);
    size.h = size.h.max(style.min_height);
    Size::new(
        size.w.saturating_add(style.padding.horizontal()),
        size.h.saturating_add(style.padding.vertical()),
    )
}

fn measure_container<Action>(node: &PrimitiveNode<Action>) -> Size {
    let style = node.layout();
    let children = node.children();
    if children.is_empty() {
        return Size::new(1, 1);
    }

    match style.direction {
        Direction::Row => {
            let mut width = 0u16;
            let mut height = 0u16;
            for child in children {
                let child_size = measure_node(child);
                width = width.saturating_add(child_size.w);
                height = height.max(child_size.h);
            }
            width = width.saturating_add(
                style
                    .gap
                    .saturating_mul(children.len().saturating_sub(1) as u16),
            );
            Size::new(width, height)
        }
        Direction::Column => {
            let mut width = 0u16;
            let mut height = 0u16;
            for child in children {
                let child_size = measure_node(child);
                width = width.max(child_size.w);
                height = height.saturating_add(child_size.h);
            }
            height = height.saturating_add(
                style
                    .gap
                    .saturating_mul(children.len().saturating_sub(1) as u16),
            );
            Size::new(width, height)
        }
    }
}

fn layout_node<Action>(
    node: &PrimitiveNode<Action>,
    rect: Rect,
    out: &mut HashMap<NodeId, LayoutInfo>,
) {
    let style = node.layout();
    let outer = apply_margin(rect, style.margin);
    let content = node_content_rect(node, outer);
    out.insert(
        node.id(),
        LayoutInfo {
            rect: outer,
            content_rect: content,
        },
    );

    if node.children().is_empty() {
        return;
    }

    match style.direction {
        Direction::Row => layout_children_row(node, content, out),
        Direction::Column => layout_children_col(node, content, out),
    }
}

fn layout_children_row<Action>(
    node: &PrimitiveNode<Action>,
    content: Rect,
    out: &mut HashMap<NodeId, LayoutInfo>,
) {
    let style = node.layout();
    let children = node.children();
    let gap_total = style
        .gap
        .saturating_mul(children.len().saturating_sub(1) as u16);
    let fixed_total: u16 = children
        .iter()
        .filter(|child| child.layout().flex == 0)
        .map(|child| measure_node(child).w)
        .sum();
    let flex_total: u16 = children.iter().map(|child| child.layout().flex).sum();
    let remaining = content
        .w
        .saturating_sub(fixed_total.saturating_add(gap_total));
    let mut cursor = content.x
        + justify_offset(
            content.w,
            fixed_total.saturating_add(gap_total),
            style.justify,
        );
    let between_gap = if matches!(style.justify, Justify::SpaceBetween) && children.len() > 1 {
        content
            .w
            .saturating_sub(fixed_total)
            .saturating_div(children.len().saturating_sub(1) as u16)
    } else {
        style.gap
    };
    let flex_floor_total: u16 = if flex_total > 0 {
        children
            .iter()
            .filter(|child| child.layout().flex > 0)
            .map(|child| remaining.saturating_mul(child.layout().flex) / flex_total)
            .sum()
    } else {
        0
    };
    let mut flex_remainder = remaining.saturating_sub(flex_floor_total);

    for child in children {
        let child_style = child.layout();
        let measured = measure_node(child);
        let mut width = if child_style.flex > 0 && flex_total > 0 {
            let mut value = remaining.saturating_mul(child_style.flex) / flex_total;
            if flex_remainder > 0 {
                value = value.saturating_add(1);
                flex_remainder -= 1;
            }
            value.max(child_style.min_width).max(1)
        } else {
            measured.w
        };
        if let Some(fixed) = child_style.width {
            width = fixed;
        }
        let height = child_cross_size(content.h, measured.h, child_style.height, style.align);
        let y = cross_position(content.y, content.h, height, style.align);
        layout_node(
            child,
            Rect::new(cursor, y, width.min(content.w), height),
            out,
        );
        cursor = cursor.saturating_add(width).saturating_add(between_gap);
    }
}

fn layout_children_col<Action>(
    node: &PrimitiveNode<Action>,
    content: Rect,
    out: &mut HashMap<NodeId, LayoutInfo>,
) {
    let style = node.layout();
    let children = node.children();
    let gap_total = style
        .gap
        .saturating_mul(children.len().saturating_sub(1) as u16);
    let fixed_total: u16 = children
        .iter()
        .filter(|child| child.layout().flex == 0)
        .map(|child| measure_node(child).h)
        .sum();
    let flex_total: u16 = children.iter().map(|child| child.layout().flex).sum();
    let remaining = content
        .h
        .saturating_sub(fixed_total.saturating_add(gap_total));
    let mut cursor = content.y
        + justify_offset(
            content.h,
            fixed_total.saturating_add(gap_total),
            style.justify,
        );
    let between_gap = if matches!(style.justify, Justify::SpaceBetween) && children.len() > 1 {
        content
            .h
            .saturating_sub(fixed_total)
            .saturating_div(children.len().saturating_sub(1) as u16)
    } else {
        style.gap
    };
    let flex_floor_total: u16 = if flex_total > 0 {
        children
            .iter()
            .filter(|child| child.layout().flex > 0)
            .map(|child| remaining.saturating_mul(child.layout().flex) / flex_total)
            .sum()
    } else {
        0
    };
    let mut flex_remainder = remaining.saturating_sub(flex_floor_total);

    for child in children {
        let child_style = child.layout();
        let measured = measure_node(child);
        let width = child_cross_size(content.w, measured.w, child_style.width, style.align);
        let mut height = if child_style.flex > 0 && flex_total > 0 {
            let mut value = remaining.saturating_mul(child_style.flex) / flex_total;
            if flex_remainder > 0 {
                value = value.saturating_add(1);
                flex_remainder -= 1;
            }
            value.max(child_style.min_height).max(1)
        } else {
            measured.h
        };
        if let Some(fixed) = child_style.height {
            height = fixed;
        }
        let x = cross_position(content.x, content.w, width, style.align);
        layout_node(
            child,
            Rect::new(x, cursor, width, height.min(content.h)),
            out,
        );
        cursor = cursor.saturating_add(height).saturating_add(between_gap);
    }
}

fn apply_margin(rect: Rect, margin: Edge) -> Rect {
    Rect::new(
        rect.x.saturating_add(margin.left),
        rect.y.saturating_add(margin.top),
        rect.w.saturating_sub(margin.horizontal()),
        rect.h.saturating_sub(margin.vertical()),
    )
}

fn content_rect(rect: Rect, padding: Edge) -> Rect {
    Rect::new(
        rect.x.saturating_add(padding.left),
        rect.y.saturating_add(padding.top),
        rect.w.saturating_sub(padding.horizontal()),
        rect.h.saturating_sub(padding.vertical()),
    )
}

fn node_content_rect<Action>(node: &PrimitiveNode<Action>, outer: Rect) -> Rect {
    let after_border = if matches!(node.kind(), NodeKind::Panel) {
        Rect::new(
            outer.x.saturating_add(1),
            outer.y.saturating_add(1),
            outer.w.saturating_sub(2),
            outer.h.saturating_sub(2),
        )
    } else {
        outer
    };

    content_rect(after_border, node.layout().padding)
}

fn justify_offset(available: u16, used: u16, justify: Justify) -> u16 {
    let free = available.saturating_sub(used);
    match justify {
        Justify::Start | Justify::SpaceBetween => 0,
        Justify::Center => free / 2,
        Justify::End => free,
    }
}

fn child_cross_size(available: u16, measured: u16, fixed: Option<u16>, align: Align) -> u16 {
    if let Some(fixed) = fixed {
        return fixed.min(available);
    }

    match align {
        Align::Stretch => available,
        Align::Start | Align::Center | Align::End => measured.min(available),
    }
}

fn cross_position(origin: u16, available: u16, size: u16, align: Align) -> u16 {
    match align {
        Align::Start | Align::Stretch => origin,
        Align::Center => origin + available.saturating_sub(size) / 2,
        Align::End => origin + available.saturating_sub(size),
    }
}

pub fn display_width(text: &str) -> u16 {
    text.chars().map(|ch| if is_wide(ch) { 2 } else { 1 }).sum()
}

pub fn is_wide(ch: char) -> bool {
    matches!(
        ch as u32,
        0x1100..=0x115F
            | 0x2E80..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE10..=0xFE19
            | 0xFE30..=0xFE6F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    #[test]
    fn row_and_col_layout_children_on_main_axis() {
        let row_view: View = row((text("a").width(2), text("b").width(3))).gap(1);
        let row_layout = layout_tree(&row_view, Rect::new(0, 0, 10, 2));
        let row_children = row_view.node().children();

        assert_eq!(row_layout[&row_children[0].id()].rect.x, 0);
        assert_eq!(row_layout[&row_children[1].id()].rect.x, 3);

        let col_view: View = col((text("a").height(1), text("b").height(1))).gap(1);
        let col_layout = layout_tree(&col_view, Rect::new(0, 0, 10, 4));
        let col_children = col_view.node().children();

        assert_eq!(col_layout[&col_children[0].id()].rect.y, 0);
        assert_eq!(col_layout[&col_children[1].id()].rect.y, 2);
    }

    #[test]
    fn rect_contains_points_inside_bounds_only() {
        let rect = Rect::new(2, 3, 4, 5);

        assert!(rect.contains(2, 3));
        assert!(rect.contains(5, 7));
        assert!(!rect.contains(6, 7));
        assert!(!rect.contains(5, 8));
    }

    #[test]
    fn flex_child_gets_remaining_space_with_deterministic_remainder() {
        let view: View = row((
            text("a").width(2),
            panel(text("fill")).flex(1),
            panel(text("more")).flex(2),
        ))
        .gap(1);
        let layout = layout_tree(&view, Rect::new(0, 0, 12, 2));
        let children = view.node().children();

        assert_eq!(layout[&children[1].id()].rect.w, 3);
        assert_eq!(layout[&children[2].id()].rect.w, 5);
    }

    #[test]
    fn padding_shrinks_content_rect_and_wide_text_measures_two_cells() {
        let view: View = col((text("界"),)).padding(1);
        let layout = layout_tree(&view, Rect::new(0, 0, 8, 4));

        assert_eq!(
            layout[&view.node().id()].content_rect,
            Rect::new(1, 1, 6, 2)
        );
        assert_eq!(measure_node(&view.node().children()[0]).w, 2);
    }
}
