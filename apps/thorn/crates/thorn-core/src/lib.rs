use std::collections::VecDeque;
use std::marker::PhantomData;

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

pub trait ThornApp {
    type Action;

    fn update(&mut self, action: Self::Action, ctx: &mut AppContext<Self::Action>);
    fn view(&self) -> Element<Self::Action>;
}

#[derive(Debug, Clone)]
pub struct AppContext<Action> {
    actions: VecDeque<Action>,
    render_requested: bool,
    quit_requested: bool,
}

impl<Action> Default for AppContext<Action> {
    fn default() -> Self {
        Self {
            actions: VecDeque::new(),
            render_requested: false,
            quit_requested: false,
        }
    }
}

impl<Action> AppContext<Action> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn dispatch(&mut self, action: Action) {
        self.actions.push_back(action);
    }

    pub fn pop_action(&mut self) -> Option<Action> {
        self.actions.pop_front()
    }

    pub fn request_render(&mut self) {
        self.render_requested = true;
    }

    pub fn take_render_requested(&mut self) -> bool {
        let requested = self.render_requested;
        self.render_requested = false;
        requested
    }

    pub fn quit(&mut self) {
        self.quit_requested = true;
    }

    pub fn is_quit_requested(&self) -> bool {
        self.quit_requested
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeInput {
    Key(KeyEvent),
    Resize(Size),
    Tick,
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEvent {
    pub key: Key,
    pub modifiers: KeyModifiers,
    pub kind: KeyEventKind,
}

impl KeyEvent {
    pub const fn char(ch: char) -> Self {
        Self {
            key: Key::Char(ch),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
        }
    }

    pub const fn ctrl(ch: char) -> Self {
        Self {
            key: Key::Char(ch),
            modifiers: KeyModifiers::CTRL,
            kind: KeyEventKind::Press,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyModifiers {
    bits: u8,
}

impl KeyModifiers {
    pub const CTRL: Self = Self { bits: 0b0000_0001 };

    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.bits & other.bits) == other.bits
    }
}

impl Default for KeyModifiers {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEventKind {
    Press,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyIntent {
    RequestQuit,
    App(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyAction<Action> {
    RuntimeQuit,
    App(Action),
}

pub trait IntentMapper<Action> {
    fn map_intent(&self, intent: KeyIntent) -> Option<KeyAction<Action>>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyMap {
    bindings: Vec<(KeyEvent, KeyIntent)>,
}

impl Default for KeyMap {
    fn default() -> Self {
        Self::new()
            .bind(KeyEvent::char('+'), KeyIntent::App("increment"))
            .bind(KeyEvent::char('-'), KeyIntent::App("decrement"))
            .bind(KeyEvent::char('q'), KeyIntent::RequestQuit)
            .bind(KeyEvent::ctrl('c'), KeyIntent::RequestQuit)
    }
}

impl KeyMap {
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    pub fn bind(mut self, event: KeyEvent, intent: KeyIntent) -> Self {
        self.bindings.push((event, intent));
        self
    }

    pub fn resolve(&self, event: &KeyEvent) -> Option<KeyIntent> {
        self.bindings
            .iter()
            .find_map(|(bound_event, intent)| (bound_event == event).then(|| intent.clone()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element<Action> {
    node: ElementNode,
    _action: PhantomData<fn() -> Action>,
}

impl<Action> Element<Action> {
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
            node: ElementNode::Column(children.into_iter().map(|child| child.node).collect()),
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
    Column(Vec<ElementNode>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextElement {
    pub content: String,
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
    View,
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
        ElementNode::Column(children) => HostNode {
            id,
            kind: HostKind::View,
            text: None,
            children: children
                .iter()
                .map(|child| lower_element_node(child, next_id))
                .collect(),
            _action: PhantomData,
        },
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
    if host.kind == HostKind::View {
        for (index, child) in host.children.iter().enumerate() {
            let y = rect.y.saturating_add(index as u16);
            layout_node(child, Rect::new(rect.x, y, rect.width, 1), layout);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaintPrimitive {
    TextRun { x: u16, y: u16, text: String },
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
            paint.push(PaintPrimitive::TextRun {
                x: layout_node.rect.x,
                y: layout_node.rect.y,
                text: text.clone(),
            });
        }
    }

    for child in &host.children {
        paint_node(child, layout, paint);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
}

impl Default for Cell {
    fn default() -> Self {
        Self { ch: ' ' }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Screen {
    pub size: Size,
    pub cells: Vec<Cell>,
}

impl Screen {
    pub fn new(size: Size) -> Self {
        Self {
            size,
            cells: vec![Cell::default(); usize::from(size.width) * usize::from(size.height)],
        }
    }

    pub fn write_text(&mut self, x: u16, y: u16, text: &str) {
        if y >= self.size.height {
            return;
        }

        for (offset, ch) in text.chars().enumerate() {
            let x = x.saturating_add(offset as u16);
            if x >= self.size.width {
                break;
            }
            let index = usize::from(y) * usize::from(self.size.width) + usize::from(x);
            if let Some(cell) = self.cells.get_mut(index) {
                cell.ch = ch;
            }
        }
    }

    pub fn apply(&mut self, paint: &[PaintPrimitive]) {
        for primitive in paint {
            match primitive {
                PaintPrimitive::TextRun { x, y, text } => self.write_text(*x, *y, text),
            }
        }
    }

    pub fn to_plain_text(&self) -> String {
        let width = usize::from(self.size.width);
        let mut lines = Vec::with_capacity(usize::from(self.size.height));
        for row in self.cells.chunks(width) {
            let line = row.iter().map(|cell| cell.ch).collect::<String>();
            lines.push(line.trim_end().to_string());
        }
        lines.join("\n")
    }
}

pub fn render_to_screen<Action>(element: &Element<Action>, size: Size) -> Screen {
    let host = lower_element(element);
    let layout = layout_tree(&host, size);
    let paint = paint_tree(&host, &layout);
    let mut screen = Screen::new(size);
    screen.apply(&paint);
    screen
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn element_text_lowers_to_host_text() {
        let element = text::<()>("hello");
        let host = lower_element(&element);

        assert_eq!(host.kind, HostKind::Text);
        assert_eq!(host.text.as_deref(), Some("hello"));
    }

    #[test]
    fn column_lowers_to_host_view_with_children() {
        let element = column((text::<()>("a"), text::<()>("b")));
        let host = lower_element(&element);

        assert_eq!(host.kind, HostKind::View);
        assert_eq!(host.children.len(), 2);
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
    fn column_layout_stacks_children_vertically() {
        let element = column((text::<()>("a"), text::<()>("b")));
        let host = lower_element(&element);
        let layout = layout_tree(&host, Size::new(10, 5));

        assert_eq!(layout[1].rect, Rect::new(0, 0, 10, 1));
        assert_eq!(layout[2].rect, Rect::new(0, 1, 10, 1));
    }

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
    fn paint_text_run_writes_cells() {
        let mut screen = Screen::new(Size::new(10, 2));
        screen.apply(&[PaintPrimitive::TextRun {
            x: 0,
            y: 0,
            text: "hello".to_string(),
        }]);

        assert!(screen.to_plain_text().contains("hello"));
    }

    #[test]
    fn screen_plain_text_contains_written_text() {
        let screen = render_to_screen(&text::<()>("hello"), Size::new(10, 2));

        assert!(screen.to_plain_text().contains("hello"));
    }

    #[test]
    fn keymap_maps_plus_to_increment_intent() {
        assert_eq!(
            KeyMap::default().resolve(&KeyEvent::char('+')),
            Some(KeyIntent::App("increment"))
        );
    }

    #[test]
    fn keymap_maps_q_to_quit_intent() {
        assert_eq!(
            KeyMap::default().resolve(&KeyEvent::char('q')),
            Some(KeyIntent::RequestQuit)
        );
    }
}
