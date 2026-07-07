use crate::view::{button_view, text_view, with_children, IntoText, IntoViewVec, NodeKind, View};

pub fn text<Action>(value: impl IntoText) -> View<Action> {
    text_view(value)
}

pub fn row<Action, C>(children: C) -> View<Action>
where
    C: IntoViewVec<Action>,
{
    with_children(NodeKind::Row, children)
}

pub fn col<Action, C>(children: C) -> View<Action>
where
    C: IntoViewVec<Action>,
{
    with_children(NodeKind::Col, children)
}

pub fn panel<Action>(child: View<Action>) -> View<Action> {
    with_children(NodeKind::Panel, child)
}

pub fn button<Action>(label: impl Into<String>) -> View<Action> {
    button_view(label)
}
