use std::marker::PhantomData;

use crate::{Axis, Element, ElementNode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HostNodeId(u32);

impl HostNodeId {
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostKind {
    Text,
    View { axis: Axis },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostNode<Action> {
    pub id: HostNodeId,
    pub kind: HostKind,
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
            text: Some(text.content.clone()),
            children: Vec::new(),
            _action: PhantomData,
        },
        ElementNode::View(view) => HostNode {
            id,
            kind: HostKind::View { axis: view.axis },
            text: None,
            children: lower_children(view.axis, &view.children, next_id),
            _action: PhantomData,
        },
        ElementNode::Stack(stack) => HostNode {
            id,
            kind: HostKind::View { axis: stack.axis },
            text: None,
            children: lower_children(stack.axis, &stack.children, next_id),
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
            ElementNode::Stack(stack) if stack.axis == parent_axis => {
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
    use crate::{column, row, text};

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
}
