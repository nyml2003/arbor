use std::marker::PhantomData;

use crate::{Axis, Element, ElementNode, LayoutStyle, PaintStyle};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HostNodeId(u32);

impl HostNodeId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostKind {
    Text,
    View { axis: Axis },
    ScrollView { axis: Axis },
    Clip { axis: Axis },
    Border { axis: Axis, style: PaintStyle },
    Layer { axis: Axis, z_index: i16 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostNode<Action> {
    pub id: HostNodeId,
    pub kind: HostKind,
    pub layout_style: LayoutStyle,
    pub text: Option<String>,
    pub children: Vec<HostNode<Action>>,
    _action: PhantomData<fn() -> Action>,
}

pub fn lower_element<Action>(element: &Element<Action>) -> HostNode<Action> {
    let mut next_id = 0;
    lower_element_node(element.node(), &mut next_id)
}

fn lower_element_node<Action>(node: &ElementNode, next_id: &mut u32) -> HostNode<Action> {
    let id = HostNodeId(*next_id);
    *next_id += 1;
    match node {
        ElementNode::Text(text) => HostNode {
            id,
            kind: HostKind::Text,
            layout_style: text.layout_style,
            text: Some(text.content.clone()),
            children: Vec::new(),
            _action: PhantomData,
        },
        ElementNode::View(view) => HostNode {
            id,
            kind: HostKind::View { axis: view.axis },
            layout_style: view.layout_style,
            text: None,
            children: lower_children(view.axis, &view.children, next_id),
            _action: PhantomData,
        },
        ElementNode::ScrollView(view) => HostNode {
            id,
            kind: HostKind::ScrollView { axis: view.axis },
            layout_style: view.layout_style,
            text: None,
            children: lower_children(view.axis, &view.children, next_id),
            _action: PhantomData,
        },
        ElementNode::Clip(view) => HostNode {
            id,
            kind: HostKind::Clip { axis: view.axis },
            layout_style: view.layout_style,
            text: None,
            children: lower_children(view.axis, &view.children, next_id),
            _action: PhantomData,
        },
        ElementNode::Stack(stack) => HostNode {
            id,
            kind: HostKind::View { axis: stack.axis },
            layout_style: stack.layout_style,
            text: None,
            children: lower_children(stack.axis, &stack.children, next_id),
            _action: PhantomData,
        },
        ElementNode::Layer(layer) => HostNode {
            id,
            kind: HostKind::Layer {
                axis: layer.axis,
                z_index: layer.z_index,
            },
            layout_style: layer.layout_style,
            text: None,
            children: lower_children(layer.axis, &layer.children, next_id),
            _action: PhantomData,
        },
        ElementNode::Border(border) => HostNode {
            id,
            kind: HostKind::Border {
                axis: border.axis,
                style: border.border_style,
            },
            layout_style: border.layout_style,
            text: None,
            children: lower_children(border.axis, &border.children, next_id),
            _action: PhantomData,
        },
    }
}

fn lower_children<Action>(
    parent_axis: Axis,
    children: &[ElementNode],
    next_id: &mut u32,
) -> Vec<HostNode<Action>> {
    let mut host_children = Vec::new();
    for child in children {
        match child {
            ElementNode::Stack(stack)
                if stack.axis == parent_axis && stack.layout_style == LayoutStyle::default() =>
            {
                host_children.extend(lower_children(parent_axis, &stack.children, next_id));
            }
            _ => host_children.push(lower_element_node(child, next_id)),
        }
    }
    host_children
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        border, clip, column, layer, row, scroll_view, text, CrossAxisAlignment, MainAxisAlignment,
        Margin, Padding, PaintColor, PaintStyle, ScrollOffset, Size,
    };

    #[test]
    fn element_text_lowers_to_host_text() {
        let element = text::<()>("hello");
        let host = lower_element(&element);

        assert_eq!(host.kind, HostKind::Text);
        assert_eq!(host.text.as_deref(), Some("hello"));
    }

    #[test]
    fn column_lowers_to_vertical_host_view_with_children() {
        let element = column((text::<()>("a"), text::<()>("b")));
        let host = lower_element(&element);

        assert_eq!(
            host.kind,
            HostKind::View {
                axis: Axis::Vertical
            }
        );
        assert_eq!(host.children.len(), 2);
    }

    #[test]
    fn row_lowers_to_horizontal_host_view() {
        let element = row((text::<()>("a"), text::<()>("b")));
        let host = lower_element(&element);

        assert_eq!(
            host.kind,
            HostKind::View {
                axis: Axis::Horizontal
            }
        );
        assert_eq!(host.children.len(), 2);
    }

    #[test]
    fn same_axis_stack_sugar_is_flattened() {
        let element = column((text::<()>("a"), column((text::<()>("b"), text::<()>("c")))));
        let host = lower_element(&element);

        assert_eq!(host.children.len(), 3);
        assert!(host
            .children
            .iter()
            .all(|child| child.kind == HostKind::Text));
    }

    #[test]
    fn styled_same_axis_stack_keeps_layout_boundary() {
        let element = column((
            text::<()>("a"),
            column((text::<()>("b"), text::<()>("c"))).gap(1),
        ));
        let host = lower_element(&element);

        assert_eq!(host.children.len(), 2);
        assert_eq!(
            host.children[1].kind,
            HostKind::View {
                axis: Axis::Vertical
            }
        );
        assert_eq!(host.children[1].layout_style.gap, 1);
    }

    #[test]
    fn different_axis_stack_keeps_layout_boundary() {
        let element = column((text::<()>("a"), row((text::<()>("b"), text::<()>("c")))));
        let host = lower_element(&element);

        assert_eq!(host.children.len(), 2);
        assert_eq!(
            host.children[1].kind,
            HostKind::View {
                axis: Axis::Horizontal
            }
        );
    }

    #[test]
    fn host_tree_assigns_stable_ids() {
        let element = column((text::<()>("a"), text::<()>("b")));
        let host = lower_element(&element);

        assert_eq!(host.id.as_u32(), 0);
        assert_eq!(host.children[0].id.as_u32(), 1);
        assert_eq!(host.children[1].id.as_u32(), 2);
    }

    #[test]
    fn layout_style_lowers_from_element_to_host() {
        let element = row((text::<()>("a"), text::<()>("b")))
            .gap(2)
            .padding(Padding::symmetric(1, 3))
            .margin(Margin::new(0, 1, 2, 3))
            .fixed_size(Size::new(9, 4))
            .min_size(Size::new(7, 2))
            .flex_grow(3)
            .main_axis_alignment(MainAxisAlignment::Center)
            .cross_axis_alignment(CrossAxisAlignment::End);
        let host = lower_element(&element);

        assert_eq!(host.layout_style.gap, 2);
        assert_eq!(host.layout_style.padding, Padding::symmetric(1, 3));
        assert_eq!(host.layout_style.margin, Margin::new(0, 1, 2, 3));
        assert_eq!(host.layout_style.fixed_size, Some(Size::new(9, 4)));
        assert_eq!(host.layout_style.min_size, Some(Size::new(7, 2)));
        assert_eq!(host.layout_style.flex_grow, 3);
        assert_eq!(
            host.layout_style.main_axis_alignment,
            MainAxisAlignment::Center
        );
        assert_eq!(
            host.layout_style.cross_axis_alignment,
            CrossAxisAlignment::End
        );
    }

    #[test]
    fn child_margin_lowers_to_host_and_blocks_same_axis_flattening() {
        let element = column((
            text::<()>("a"),
            column((text::<()>("b"), text::<()>("c"))).margin(Margin::new(1, 0, 0, 0)),
        ));
        let host = lower_element(&element);

        assert_eq!(host.children.len(), 2);
        assert_eq!(
            host.children[1].kind,
            HostKind::View {
                axis: Axis::Vertical
            }
        );
        assert_eq!(
            host.children[1].layout_style.margin,
            Margin::new(1, 0, 0, 0)
        );
    }

    #[test]
    fn fixed_size_lowers_to_host_and_blocks_same_axis_flattening() {
        let element = column((
            text::<()>("a"),
            column((text::<()>("b"), text::<()>("c"))).fixed_size(Size::new(4, 3)),
        ));
        let host = lower_element(&element);

        assert_eq!(host.children.len(), 2);
        assert_eq!(
            host.children[1].kind,
            HostKind::View {
                axis: Axis::Vertical
            }
        );
        assert_eq!(
            host.children[1].layout_style.fixed_size,
            Some(Size::new(4, 3))
        );
    }

    #[test]
    fn min_size_lowers_to_host_and_blocks_same_axis_flattening() {
        let element = column((
            text::<()>("a"),
            column((text::<()>("b"), text::<()>("c"))).min_size(Size::new(4, 3)),
        ));
        let host = lower_element(&element);

        assert_eq!(host.children.len(), 2);
        assert_eq!(
            host.children[1].kind,
            HostKind::View {
                axis: Axis::Vertical
            }
        );
        assert_eq!(
            host.children[1].layout_style.min_size,
            Some(Size::new(4, 3))
        );
    }

    #[test]
    fn flex_grow_lowers_to_host_and_blocks_same_axis_flattening() {
        let element = column((
            text::<()>("a"),
            column((text::<()>("b"), text::<()>("c"))).flex_grow(2),
        ));
        let host = lower_element(&element);

        assert_eq!(host.children.len(), 2);
        assert_eq!(
            host.children[1].kind,
            HostKind::View {
                axis: Axis::Vertical
            }
        );
        assert_eq!(host.children[1].layout_style.flex_grow, 2);
    }

    #[test]
    fn alignment_lowers_to_host_and_blocks_same_axis_flattening() {
        let element = column((
            text::<()>("a"),
            column((text::<()>("b"), text::<()>("c")))
                .main_axis_alignment(MainAxisAlignment::End)
                .cross_axis_alignment(CrossAxisAlignment::Center),
        ));
        let host = lower_element(&element);

        assert_eq!(host.children.len(), 2);
        assert_eq!(
            host.children[1].kind,
            HostKind::View {
                axis: Axis::Vertical
            }
        );
        assert_eq!(
            host.children[1].layout_style.main_axis_alignment,
            MainAxisAlignment::End
        );
        assert_eq!(
            host.children[1].layout_style.cross_axis_alignment,
            CrossAxisAlignment::Center
        );
    }

    #[test]
    fn scroll_offset_lowers_to_host_and_blocks_same_axis_flattening() {
        let element = column((
            text::<()>("a"),
            column((text::<()>("b"), text::<()>("c"))).scroll_offset(ScrollOffset::new(1, 2)),
        ));
        let host = lower_element(&element);

        assert_eq!(host.children.len(), 2);
        assert_eq!(
            host.children[1].kind,
            HostKind::View {
                axis: Axis::Vertical
            }
        );
        assert_eq!(
            host.children[1].layout_style.scroll_offset,
            Some(ScrollOffset::new(1, 2))
        );
    }

    #[test]
    fn scroll_view_helper_lowers_to_distinct_host_boundary() {
        let element = column((
            text::<()>("a"),
            scroll_view((text::<()>("b"), text::<()>("c"))).scroll_offset(ScrollOffset::new(0, 1)),
        ));
        let host = lower_element(&element);

        assert_eq!(host.children.len(), 2);
        assert_eq!(
            host.children[1].kind,
            HostKind::ScrollView {
                axis: Axis::Vertical
            }
        );
        assert_eq!(
            host.children[1].layout_style.scroll_offset,
            Some(ScrollOffset::new(0, 1))
        );
    }

    #[test]
    fn clip_helper_lowers_to_distinct_host_boundary() {
        let element = column((
            text::<()>("a"),
            clip((text::<()>("b"), text::<()>("c"))).padding(1),
        ));
        let host = lower_element(&element);

        assert_eq!(host.children.len(), 2);
        assert_eq!(
            host.children[1].kind,
            HostKind::Clip {
                axis: Axis::Vertical
            }
        );
        assert_eq!(host.children[1].layout_style.padding, Padding::all(1));
    }

    #[test]
    fn border_helper_lowers_to_distinct_host_boundary() {
        let style = PaintStyle {
            foreground: Some(PaintColor::Indexed(2)),
            ..PaintStyle::default()
        };
        let element = column((
            text::<()>("a"),
            border((text::<()>("b"), text::<()>("c"))).border_style(style),
        ));
        let host = lower_element(&element);

        assert_eq!(host.children.len(), 2);
        assert_eq!(
            host.children[1].kind,
            HostKind::Border {
                axis: Axis::Vertical,
                style
            }
        );
    }

    #[test]
    fn layer_helper_lowers_to_distinct_host_boundary() {
        let element = column((
            text::<()>("a"),
            layer(10, (text::<()>("b"), text::<()>("c"))).gap(1),
        ));
        let host = lower_element(&element);

        assert_eq!(host.children.len(), 2);
        assert_eq!(
            host.children[1].kind,
            HostKind::Layer {
                axis: Axis::Vertical,
                z_index: 10
            }
        );
        assert_eq!(host.children[1].layout_style.gap, 1);
    }
}
