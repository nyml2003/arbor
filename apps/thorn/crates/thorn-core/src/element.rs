use std::marker::PhantomData;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element<Action> {
    node: ElementNode,
    _action: PhantomData<fn() -> Action>,
}

impl<Action> Element<Action> {
    pub fn view(children: Vec<Self>) -> Self {
        Self {
            node: ElementNode::View(ViewElement {
                axis: Axis::Vertical,
                children: children.into_iter().map(|child| child.node).collect(),
            }),
            _action: PhantomData,
        }
    }

    pub fn text(content: impl Into<String>) -> Self {
        Self {
            node: ElementNode::Text(TextElement {
                content: content.into(),
            }),
            _action: PhantomData,
        }
    }

    pub fn column(children: Vec<Self>) -> Self {
        Self {
            node: ElementNode::Stack(StackElement {
                axis: Axis::Vertical,
                children: children.into_iter().map(|child| child.node).collect(),
            }),
            _action: PhantomData,
        }
    }

    pub fn row(children: Vec<Self>) -> Self {
        Self {
            node: ElementNode::Stack(StackElement {
                axis: Axis::Horizontal,
                children: children.into_iter().map(|child| child.node).collect(),
            }),
            _action: PhantomData,
        }
    }

    pub fn node(&self) -> &ElementNode {
        &self.node
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementNode {
    Text(TextElement),
    View(ViewElement),
    Stack(StackElement),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextElement {
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewElement {
    pub axis: Axis,
    pub children: Vec<ElementNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackElement {
    pub axis: Axis,
    pub children: Vec<ElementNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Vertical,
    Horizontal,
}

pub fn view<Action>(children: impl IntoChildren<Action>) -> Element<Action> {
    Element::view(children.into_children())
}

pub fn text<Action>(content: impl Into<String>) -> Element<Action> {
    Element::text(content)
}

pub trait IntoChildren<Action> {
    fn into_children(self) -> Vec<Element<Action>>;
}

impl<Action> IntoChildren<Action> for Vec<Element<Action>> {
    fn into_children(self) -> Vec<Element<Action>> {
        self
    }
}

impl<Action, const N: usize> IntoChildren<Action> for [Element<Action>; N] {
    fn into_children(self) -> Vec<Element<Action>> {
        self.into()
    }
}

impl<Action> IntoChildren<Action> for (Element<Action>,) {
    fn into_children(self) -> Vec<Element<Action>> {
        vec![self.0]
    }
}

impl<Action> IntoChildren<Action> for (Element<Action>, Element<Action>) {
    fn into_children(self) -> Vec<Element<Action>> {
        vec![self.0, self.1]
    }
}

impl<Action> IntoChildren<Action> for (Element<Action>, Element<Action>, Element<Action>) {
    fn into_children(self) -> Vec<Element<Action>> {
        vec![self.0, self.1, self.2]
    }
}

pub fn column<Action>(children: impl IntoChildren<Action>) -> Element<Action> {
    Element::column(children.into_children())
}

pub fn row<Action>(children: impl IntoChildren<Action>) -> Element<Action> {
    Element::row(children.into_children())
}
