use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::layout::{Align, Direction, Edge, Justify, LayoutStyle};
use crate::reactive::Scope;
use crate::theme::{ColorSource, Token};

static NEXT_NODE_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(u64);

impl NodeId {
    fn next() -> Self {
        Self(NEXT_NODE_ID.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Text,
    Row,
    Col,
    Panel,
}

#[derive(Clone)]
pub struct View<Action = ()> {
    node: PrimitiveNode<Action>,
}

impl<Action> View<Action> {
    pub fn new(kind: NodeKind) -> Self {
        Self {
            node: PrimitiveNode::new(kind),
        }
    }

    pub fn node(&self) -> &PrimitiveNode<Action> {
        &self.node
    }

    pub(crate) fn node_mut(&mut self) -> &mut PrimitiveNode<Action> {
        &mut self.node
    }

    pub fn id(mut self, key: impl Into<String>) -> Self {
        self.node.key = Some(key.into());
        self
    }

    pub fn width(mut self, width: u16) -> Self {
        self.node.layout.width = Some(width);
        self
    }

    pub fn height(mut self, height: u16) -> Self {
        self.node.layout.height = Some(height);
        self
    }

    pub fn min_width(mut self, width: u16) -> Self {
        self.node.layout.min_width = width;
        self
    }

    pub fn min_height(mut self, height: u16) -> Self {
        self.node.layout.min_height = height;
        self
    }

    pub fn flex(mut self, flex: u16) -> Self {
        self.node.layout.flex = flex;
        self
    }

    pub fn padding(mut self, padding: u16) -> Self {
        self.node.layout.padding = Edge::all(padding);
        self
    }

    pub fn gap(mut self, gap: u16) -> Self {
        self.node.layout.gap = gap;
        self
    }

    pub fn margin(mut self, margin: u16) -> Self {
        self.node.layout.margin = Edge::all(margin);
        self
    }

    pub fn justify(mut self, justify: Justify) -> Self {
        self.node.layout.justify = justify;
        self
    }

    pub fn align(mut self, align: Align) -> Self {
        self.node.layout.align = align;
        self
    }

    pub fn fg(mut self, source: impl Into<ColorSource>) -> Self {
        self.node.style.fg = Some(source.into());
        self
    }

    pub fn bg(mut self, source: impl Into<ColorSource>) -> Self {
        self.node.style.bg = Some(source.into());
        self
    }
}

#[derive(Clone)]
pub struct PrimitiveNode<Action = ()> {
    id: NodeId,
    key: Option<String>,
    kind: NodeKind,
    text: Option<Rc<RefCell<String>>>,
    children: Vec<PrimitiveNode<Action>>,
    layout: LayoutStyle,
    style: Style,
    _action: PhantomData<Action>,
}

impl<Action> PrimitiveNode<Action> {
    fn new(kind: NodeKind) -> Self {
        let mut layout = LayoutStyle::default();
        if matches!(kind, NodeKind::Row) {
            layout.direction = Direction::Row;
        }
        if matches!(kind, NodeKind::Col | NodeKind::Panel) {
            layout.direction = Direction::Column;
        }

        Self {
            id: NodeId::next(),
            key: None,
            kind,
            text: None,
            children: Vec::new(),
            layout,
            style: Style::default_for(kind),
            _action: PhantomData,
        }
    }

    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }

    pub fn kind(&self) -> NodeKind {
        self.kind
    }

    pub fn text(&self) -> Option<String> {
        self.text.as_ref().map(|text| text.borrow().clone())
    }

    pub fn children(&self) -> &[PrimitiveNode<Action>] {
        &self.children
    }

    pub fn layout(&self) -> &LayoutStyle {
        &self.layout
    }

    pub fn style(&self) -> &Style {
        &self.style
    }
}

#[derive(Clone, Debug)]
pub struct Style {
    pub fg: Option<ColorSource>,
    pub bg: Option<ColorSource>,
    pub border: Option<ColorSource>,
}

impl Style {
    fn default_for(kind: NodeKind) -> Self {
        match kind {
            NodeKind::Text => Self {
                fg: Some(Token::Text.into()),
                bg: None,
                border: None,
            },
            NodeKind::Panel => Self {
                fg: Some(Token::Text.into()),
                bg: Some(Token::SurfaceAlt.into()),
                border: Some(Token::Border.into()),
            },
            NodeKind::Row | NodeKind::Col => Self {
                fg: Some(Token::Text.into()),
                bg: Some(Token::Surface.into()),
                border: None,
            },
        }
    }
}

pub trait IntoText {
    fn into_text_slot(self) -> Rc<RefCell<String>>;
}

impl IntoText for &str {
    fn into_text_slot(self) -> Rc<RefCell<String>> {
        Rc::new(RefCell::new(self.to_string()))
    }
}

impl IntoText for String {
    fn into_text_slot(self) -> Rc<RefCell<String>> {
        Rc::new(RefCell::new(self))
    }
}

impl<F> IntoText for F
where
    F: Fn() -> String + 'static,
{
    fn into_text_slot(self) -> Rc<RefCell<String>> {
        let slot = Rc::new(RefCell::new(self()));
        if let Some(scope) = Scope::current() {
            let slot_in_effect = slot.clone();
            scope.create_effect(move || {
                *slot_in_effect.borrow_mut() = self();
            });
        }
        slot
    }
}

pub trait IntoViewVec<Action> {
    fn into_vec(self) -> Vec<PrimitiveNode<Action>>;
}

impl<Action> IntoViewVec<Action> for View<Action> {
    fn into_vec(self) -> Vec<PrimitiveNode<Action>> {
        vec![self.node]
    }
}

impl<Action> IntoViewVec<Action> for (View<Action>,) {
    fn into_vec(self) -> Vec<PrimitiveNode<Action>> {
        vec![self.0.node]
    }
}

impl<Action> IntoViewVec<Action> for (View<Action>, View<Action>) {
    fn into_vec(self) -> Vec<PrimitiveNode<Action>> {
        vec![self.0.node, self.1.node]
    }
}

impl<Action> IntoViewVec<Action> for (View<Action>, View<Action>, View<Action>) {
    fn into_vec(self) -> Vec<PrimitiveNode<Action>> {
        vec![self.0.node, self.1.node, self.2.node]
    }
}

pub(crate) fn with_children<Action, C>(kind: NodeKind, children: C) -> View<Action>
where
    C: IntoViewVec<Action>,
{
    let mut view = View::new(kind);
    view.node_mut().children = children.into_vec();
    view
}

pub(crate) fn text_view<Action>(text: impl IntoText) -> View<Action> {
    let mut view = View::new(NodeKind::Text);
    view.node_mut().text = Some(text.into_text_slot());
    view
}
