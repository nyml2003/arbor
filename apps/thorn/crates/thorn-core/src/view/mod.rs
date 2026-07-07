use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::layout::{Align, Direction, Edge, Justify, LayoutStyle};
use crate::reactive::Scope;
use crate::runtime::{Key, KeyEvent};
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
    Input,
    ScrollArea,
    Transcript,
    FuzzyPanel,
}

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

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.node.title = Some(title.into());
        self
    }
}

pub struct PrimitiveNode<Action = ()> {
    id: NodeId,
    key: Option<String>,
    title: Option<String>,
    kind: NodeKind,
    text: Option<Rc<RefCell<String>>>,
    children: Vec<PrimitiveNode<Action>>,
    layout: LayoutStyle,
    style: Style,
    input: Option<InputNode<Action>>,
    transcript: Option<TranscriptNode>,
    fuzzy: Option<FuzzyPanelNode<Action>>,
    scroll_y: Option<Rc<dyn Fn() -> u16>>,
}

pub struct InputNode<Action> {
    pub value: String,
    pub placeholder: String,
    pub loading: bool,
    pub loading_phase: usize,
    pub password: bool,
    pub on_change: Option<Box<dyn Fn(String) -> Action>>,
    pub on_submit: Option<Box<dyn Fn(String) -> Action>>,
}

#[derive(Clone)]
pub struct TranscriptMessage {
    pub label: String,
    pub label_color: ColorSource,
    pub body: String,
}

impl TranscriptMessage {
    pub fn new(
        label: impl Into<String>,
        label_color: impl Into<ColorSource>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            label_color: label_color.into(),
            body: body.into(),
        }
    }
}

#[derive(Clone)]
pub struct TranscriptNotice {
    pub title: String,
    pub detail: String,
    pub color: ColorSource,
}

impl TranscriptNotice {
    pub fn new(
        title: impl Into<String>,
        detail: impl Into<String>,
        color: impl Into<ColorSource>,
    ) -> Self {
        Self {
            title: title.into(),
            detail: detail.into(),
            color: color.into(),
        }
    }
}

pub struct TranscriptNode {
    pub messages: Vec<TranscriptMessage>,
    pub empty_text: String,
    pub notice: Option<TranscriptNotice>,
}

pub struct FuzzyPanelNode<Action> {
    pub items: Vec<String>,
    pub title: Option<String>,
    pub placeholder: String,
    pub empty_text: String,
    pub query: String,
    pub selected: usize,
    pub on_move: Option<Box<dyn Fn(i32) -> Action>>,
    pub on_query_change: Option<Box<dyn Fn(String) -> Action>>,
    pub on_submit: Option<Box<dyn Fn(FuzzyPanelSelection) -> Action>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FuzzyPanelSelection {
    pub index: usize,
    pub item: String,
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
            title: None,
            kind,
            text: None,
            children: Vec::new(),
            layout,
            style: Style::default_for(kind),
            input: None,
            transcript: None,
            fuzzy: None,
            scroll_y: None,
        }
    }

    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
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

    pub fn input(&self) -> Option<&InputNode<Action>> {
        self.input.as_ref()
    }

    pub fn transcript(&self) -> Option<&TranscriptNode> {
        self.transcript.as_ref()
    }

    pub fn fuzzy(&self) -> Option<&FuzzyPanelNode<Action>> {
        self.fuzzy.as_ref()
    }

    pub fn scroll_y(&self) -> u16 {
        self.scroll_y.as_ref().map(|read| read()).unwrap_or(0)
    }

    pub fn focusable(&self) -> bool {
        matches!(self.kind, NodeKind::Input | NodeKind::FuzzyPanel)
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
            NodeKind::Text | NodeKind::Input | NodeKind::Transcript | NodeKind::FuzzyPanel => {
                Self {
                    fg: Some(Token::Text.into()),
                    bg: None,
                    border: None,
                }
            }
            NodeKind::Panel => Self {
                fg: Some(Token::Text.into()),
                bg: Some(Token::SurfaceAlt.into()),
                border: Some(Token::Border.into()),
            },
            NodeKind::Row | NodeKind::Col | NodeKind::ScrollArea => Self {
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

impl<Action> IntoViewVec<Action> for Vec<View<Action>> {
    fn into_vec(self) -> Vec<PrimitiveNode<Action>> {
        self.into_iter().map(|view| view.node).collect()
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

impl<Action> IntoViewVec<Action> for (View<Action>, View<Action>, View<Action>, View<Action>) {
    fn into_vec(self) -> Vec<PrimitiveNode<Action>> {
        vec![self.0.node, self.1.node, self.2.node, self.3.node]
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

pub trait IntoView<Action> {
    fn into_view(self) -> View<Action>;
}

impl<Action> IntoView<Action> for View<Action> {
    fn into_view(self) -> View<Action> {
        self
    }
}

pub(crate) fn text_view<Action>(text: impl IntoText) -> View<Action> {
    let mut view = View::new(NodeKind::Text);
    view.node_mut().text = Some(text.into_text_slot());
    view
}

pub(crate) fn input_view<Action>(
    value: impl Into<String>,
    placeholder: impl Into<String>,
    loading: bool,
    loading_phase: usize,
    password: bool,
    on_change: Option<Box<dyn Fn(String) -> Action>>,
    on_submit: Option<Box<dyn Fn(String) -> Action>>,
) -> View<Action> {
    let mut view = View::new(NodeKind::Input);
    view.node_mut().input = Some(InputNode {
        value: value.into(),
        placeholder: placeholder.into(),
        loading,
        loading_phase,
        password,
        on_change,
        on_submit,
    });
    view
}

pub(crate) fn scroll_area_view<Action>(
    child: View<Action>,
    scroll_y: Option<Rc<dyn Fn() -> u16>>,
) -> View<Action> {
    let mut view = with_children(NodeKind::ScrollArea, child);
    view.node_mut().scroll_y = scroll_y;
    view
}

pub(crate) fn transcript_view<Action>(
    messages: Vec<TranscriptMessage>,
    empty_text: impl Into<String>,
    notice: Option<TranscriptNotice>,
    scroll_y: Option<Rc<dyn Fn() -> u16>>,
) -> View<Action> {
    let mut view = View::new(NodeKind::Transcript);
    view.node_mut().transcript = Some(TranscriptNode {
        messages,
        empty_text: empty_text.into(),
        notice,
    });
    view.node_mut().scroll_y = scroll_y;
    view
}

pub(crate) fn fuzzy_panel_view<Action>(
    items: Vec<String>,
    title: Option<String>,
    placeholder: impl Into<String>,
    empty_text: impl Into<String>,
    query: impl Into<String>,
    selected: usize,
    on_move: Option<Box<dyn Fn(i32) -> Action>>,
    on_query_change: Option<Box<dyn Fn(String) -> Action>>,
    on_submit: Option<Box<dyn Fn(FuzzyPanelSelection) -> Action>>,
) -> View<Action> {
    let mut view = View::new(NodeKind::FuzzyPanel);
    view.node_mut().fuzzy = Some(FuzzyPanelNode {
        items,
        title,
        placeholder: placeholder.into(),
        empty_text: empty_text.into(),
        query: query.into(),
        selected,
        on_move,
        on_query_change,
        on_submit,
    });
    view
}

pub fn handle_key<Action>(root: &PrimitiveNode<Action>, event: &KeyEvent) -> Option<Action> {
    if !event.kind.is_press() {
        return None;
    }
    let focusable = find_first_focusable(root)?;
    match focusable.kind() {
        NodeKind::Input => handle_input_key(focusable.input()?, event),
        NodeKind::FuzzyPanel => handle_fuzzy_key(focusable.fuzzy()?, event),
        _ => None,
    }
}

fn find_first_focusable<Action>(node: &PrimitiveNode<Action>) -> Option<&PrimitiveNode<Action>> {
    if node.focusable() {
        return Some(node);
    }
    node.children().iter().find_map(find_first_focusable)
}

fn handle_input_key<Action>(input: &InputNode<Action>, event: &KeyEvent) -> Option<Action> {
    if input.loading {
        return None;
    }
    match event.key {
        Key::Char(ch) if event.modifiers.is_empty() => {
            let mut next = input.value.clone();
            next.push(ch);
            input.on_change.as_ref().map(|callback| callback(next))
        }
        Key::Backspace => {
            let mut next = input.value.clone();
            next.pop();
            input.on_change.as_ref().map(|callback| callback(next))
        }
        Key::Delete => input
            .on_change
            .as_ref()
            .map(|callback| callback(String::new())),
        Key::Enter => input
            .on_submit
            .as_ref()
            .map(|callback| callback(input.value.clone())),
        Key::ArrowLeft | Key::ArrowRight | Key::Home | Key::End => None,
        _ => None,
    }
}

fn handle_fuzzy_key<Action>(panel: &FuzzyPanelNode<Action>, event: &KeyEvent) -> Option<Action> {
    match event.key {
        Key::Char(ch) if event.modifiers.is_empty() => {
            let mut next = panel.query.clone();
            next.push(ch);
            panel
                .on_query_change
                .as_ref()
                .map(|callback| callback(next))
        }
        Key::Backspace => {
            let mut next = panel.query.clone();
            next.pop();
            panel
                .on_query_change
                .as_ref()
                .map(|callback| callback(next))
        }
        Key::Delete => panel
            .on_query_change
            .as_ref()
            .map(|callback| callback(String::new())),
        Key::ArrowUp => panel.on_move.as_ref().map(|callback| callback(-1)),
        Key::ArrowDown => panel.on_move.as_ref().map(|callback| callback(1)),
        Key::Enter => panel
            .on_submit
            .as_ref()
            .and_then(|callback| current_fuzzy_selection(panel).map(callback)),
        _ => None,
    }
}

pub fn current_fuzzy_selection<Action>(
    panel: &FuzzyPanelNode<Action>,
) -> Option<FuzzyPanelSelection> {
    let matches = fuzzy_matches(&panel.items, &panel.query);
    let matched = matches.get(panel.selected.min(matches.len().saturating_sub(1)))?;
    Some(FuzzyPanelSelection {
        index: matched.index,
        item: panel.items[matched.index].clone(),
    })
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FuzzyMatch {
    pub index: usize,
    pub score: usize,
}

pub fn fuzzy_matches(items: &[String], query: &str) -> Vec<FuzzyMatch> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return items
            .iter()
            .enumerate()
            .map(|(index, _)| FuzzyMatch {
                index,
                score: index,
            })
            .collect();
    }
    let mut matches = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            score_item(&item.to_lowercase(), &query).map(|score| FuzzyMatch {
                index,
                score: score * items.len() + index,
            })
        })
        .collect::<Vec<_>>();
    matches.sort_by_key(|matched| matched.score);
    matches
}

fn score_item(item: &str, query: &str) -> Option<usize> {
    if let Some(pos) = item.find(query) {
        return Some(pos);
    }
    let mut score = 0usize;
    let mut last_pos = 0usize;
    for needle in query.chars() {
        let found = item[last_pos..].find(needle)?;
        score += found + 8;
        last_pos += found + needle.len_utf8();
    }
    Some(score)
}
