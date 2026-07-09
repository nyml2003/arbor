use std::marker::PhantomData;

use crate::layout::{
    CrossAxisAlignment, LayoutStyle, MainAxisAlignment, Margin, Padding, ScrollOffset, Size,
};

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
                layout_style: LayoutStyle::default(),
                children: children.into_iter().map(|child| child.node).collect(),
            }),
            _action: PhantomData,
        }
    }

    pub fn text(content: impl Into<String>) -> Self {
        Self {
            node: ElementNode::Text(TextElement {
                content: content.into(),
                layout_style: LayoutStyle::default(),
            }),
            _action: PhantomData,
        }
    }

    pub fn column(children: Vec<Self>) -> Self {
        Self {
            node: ElementNode::Stack(StackElement {
                axis: Axis::Vertical,
                layout_style: LayoutStyle::default(),
                children: children.into_iter().map(|child| child.node).collect(),
            }),
            _action: PhantomData,
        }
    }

    pub fn row(children: Vec<Self>) -> Self {
        Self {
            node: ElementNode::Stack(StackElement {
                axis: Axis::Horizontal,
                layout_style: LayoutStyle::default(),
                children: children.into_iter().map(|child| child.node).collect(),
            }),
            _action: PhantomData,
        }
    }

    pub fn scroll_view(children: Vec<Self>) -> Self {
        Self {
            node: ElementNode::ScrollView(ViewElement {
                axis: Axis::Vertical,
                layout_style: LayoutStyle::default(),
                children: children.into_iter().map(|child| child.node).collect(),
            }),
            _action: PhantomData,
        }
    }

    pub fn clip(children: Vec<Self>) -> Self {
        Self {
            node: ElementNode::Clip(ViewElement {
                axis: Axis::Vertical,
                layout_style: LayoutStyle::default(),
                children: children.into_iter().map(|child| child.node).collect(),
            }),
            _action: PhantomData,
        }
    }

    pub fn layer(z_index: i16, children: Vec<Self>) -> Self {
        Self {
            node: ElementNode::Layer(LayerElement {
                axis: Axis::Vertical,
                z_index,
                layout_style: LayoutStyle::default(),
                children: children.into_iter().map(|child| child.node).collect(),
            }),
            _action: PhantomData,
        }
    }

    pub fn layout_style(mut self, layout_style: LayoutStyle) -> Self {
        self.update_layout_style(|style| *style = layout_style);
        self
    }

    pub fn gap(mut self, gap: u16) -> Self {
        self.update_layout_style(|style| style.gap = gap);
        self
    }

    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        let padding = padding.into();
        self.update_layout_style(|style| style.padding = padding);
        self
    }

    pub fn margin(mut self, margin: impl Into<Margin>) -> Self {
        let margin = margin.into();
        self.update_layout_style(|style| style.margin = margin);
        self
    }

    pub fn fixed_size(mut self, size: Size) -> Self {
        self.update_layout_style(|style| style.fixed_size = Some(size));
        self
    }

    pub fn min_size(mut self, size: Size) -> Self {
        self.update_layout_style(|style| style.min_size = Some(size));
        self
    }

    pub fn flex_grow(mut self, flex_grow: u16) -> Self {
        self.update_layout_style(|style| style.flex_grow = flex_grow);
        self
    }

    pub fn main_axis_alignment(mut self, alignment: MainAxisAlignment) -> Self {
        self.update_layout_style(|style| style.main_axis_alignment = alignment);
        self
    }

    pub fn cross_axis_alignment(mut self, alignment: CrossAxisAlignment) -> Self {
        self.update_layout_style(|style| style.cross_axis_alignment = alignment);
        self
    }

    pub fn scroll_offset(mut self, offset: ScrollOffset) -> Self {
        self.update_layout_style(|style| style.scroll_offset = Some(offset));
        self
    }

    pub fn node(&self) -> &ElementNode {
        &self.node
    }

    fn update_layout_style(&mut self, update: impl FnOnce(&mut LayoutStyle)) {
        match &mut self.node {
            ElementNode::View(view) => update(&mut view.layout_style),
            ElementNode::ScrollView(view) => update(&mut view.layout_style),
            ElementNode::Clip(view) => update(&mut view.layout_style),
            ElementNode::Stack(stack) => update(&mut stack.layout_style),
            ElementNode::Layer(layer) => update(&mut layer.layout_style),
            ElementNode::Text(text) => update(&mut text.layout_style),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementNode {
    Text(TextElement),
    View(ViewElement),
    ScrollView(ViewElement),
    Clip(ViewElement),
    Stack(StackElement),
    Layer(LayerElement),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextElement {
    pub content: String,
    pub layout_style: LayoutStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewElement {
    pub axis: Axis,
    pub layout_style: LayoutStyle,
    pub children: Vec<ElementNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackElement {
    pub axis: Axis,
    pub layout_style: LayoutStyle,
    pub children: Vec<ElementNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerElement {
    pub axis: Axis,
    pub z_index: i16,
    pub layout_style: LayoutStyle,
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

pub fn scroll_view<Action>(children: impl IntoChildren<Action>) -> Element<Action> {
    Element::scroll_view(children.into_children())
}

pub fn clip<Action>(children: impl IntoChildren<Action>) -> Element<Action> {
    Element::clip(children.into_children())
}

pub fn layer<Action>(z_index: i16, children: impl IntoChildren<Action>) -> Element<Action> {
    Element::layer(z_index, children.into_children())
}
