//! Component facade for application code.
//!
//! A component is an ephemeral value created by `view` for one render pass.
//! Its props are owned snapshots derived from application state. Rendering
//! consumes the component, reads the current `Theme` from `Ui`, and returns a
//! `Node<Action>`.
//!
//! Persistent state belongs in application `State` or lower-level widget
//! internals. Components must not store `WidgetFactory`, cache `Theme` across
//! frames, or mutate business state during rendering.

use arbor_tui_composites::{
    FuzzyPanel as RawFuzzyPanel, FuzzyPanelSelection, Panel as RawPanel, PromptBar as RawPromptBar,
    StatusLine as RawStatusLine, Transcript as RawTranscript, TranscriptMessage, TranscriptNotice,
};
use arbor_tui_domain::cell::{AnsiColor, Attrs};
use arbor_tui_domain::layout::RectOffset;
use arbor_tui_domain::signal::ReadSignal;
use arbor_tui_domain::theme::Theme;
use arbor_tui_widgets::input::Input as RawInput;
use arbor_tui_widgets::stack::{Col as RawCol, Row as RawRow};
use arbor_tui_widgets::text::Text as RawText;

use crate::ui::{Node, Ui};

/// Marker for props that can safely enter a nested component tree.
///
/// This means "owned or otherwise `'static`". Nested components are stored
/// before rendering, so props must not borrow transient application state.
pub trait ComponentProps: 'static {}

impl<T: 'static> ComponentProps for T {}

/// Application-facing component protocol.
///
/// Lifecycle:
///
/// 1. `view` reads application `State`.
/// 2. `view` creates owned props snapshots.
/// 3. `ui.component(component)` consumes the component and calls `render`.
/// 4. Component callbacks enqueue `Action`.
/// 5. `update` handles actions and mutates `State`.
/// 6. The next render pass creates a fresh component tree.
pub trait UiComponent<Action>: 'static {
    fn render(self, ui: &Ui<Action>) -> Node<Action>;
}

/// Component protocol for exported components with explicit props.
pub trait PropsComponent<Action>: UiComponent<Action> {
    type Props: ComponentProps;

    fn from_props(props: Self::Props) -> Self;
    fn into_props(self) -> Self::Props;
}

pub struct AnyComponent<Action> {
    render: Box<dyn FnOnce(&Ui<Action>) -> Node<Action>>,
}

impl<Action: 'static> AnyComponent<Action> {
    pub fn new<Component>(component: Component) -> Self
    where
        Component: UiComponent<Action>,
    {
        Self {
            render: Box::new(move |ui| component.render(ui)),
        }
    }

    fn render(self, ui: &Ui<Action>) -> Node<Action> {
        (self.render)(ui)
    }
}

impl<Action: 'static> UiComponent<Action> for AnyComponent<Action> {
    fn render(self, ui: &Ui<Action>) -> Node<Action> {
        self.render(ui)
    }
}

impl<Action: 'static> UiComponent<Action> for Node<Action> {
    fn render(self, _ui: &Ui<Action>) -> Node<Action> {
        self
    }
}

pub struct TextBlockProps {
    content: String,
    fg: Option<AnsiColor>,
    bg: Option<AnsiColor>,
    attrs: Attrs,
    padding: RectOffset,
    flex: f32,
    width: Option<u16>,
}

impl TextBlockProps {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            fg: None,
            bg: None,
            attrs: Attrs::default(),
            padding: RectOffset::default(),
            flex: 0.0,
            width: None,
        }
    }

    pub fn content(&self) -> &str {
        &self.content
    }
}

pub struct TextBlock {
    props: TextBlockProps,
}

impl TextBlock {
    pub fn new(content: impl Into<String>) -> Self {
        Self::from_props(TextBlockProps::new(content))
    }

    pub fn from_props(props: TextBlockProps) -> Self {
        Self { props }
    }

    pub fn props(&self) -> &TextBlockProps {
        &self.props
    }

    pub fn into_props(self) -> TextBlockProps {
        self.props
    }

    pub fn fg(mut self, color: AnsiColor) -> Self {
        self.props.fg = Some(color);
        self
    }

    pub fn bg(mut self, color: AnsiColor) -> Self {
        self.props.bg = Some(color);
        self
    }

    pub fn bold(mut self) -> Self {
        self.props.attrs.bold = true;
        self
    }

    pub fn italic(mut self) -> Self {
        self.props.attrs.italic = true;
        self
    }

    pub fn dim(mut self) -> Self {
        self.props.attrs.dim = true;
        self
    }

    pub fn underline(mut self) -> Self {
        self.props.attrs.underline = true;
        self
    }

    pub fn padding(mut self, padding: RectOffset) -> Self {
        self.props.padding = padding;
        self
    }

    pub fn flex(mut self, flex: f32) -> Self {
        self.props.flex = flex;
        self
    }

    pub fn fill(self) -> Self {
        self.flex(1.0)
    }

    pub fn width(mut self, width: u16) -> Self {
        self.props.width = Some(width);
        self
    }
}

impl<Action: 'static> PropsComponent<Action> for TextBlock {
    type Props = TextBlockProps;

    fn from_props(props: Self::Props) -> Self {
        Self::from_props(props)
    }

    fn into_props(self) -> Self::Props {
        self.into_props()
    }
}

impl<Action: 'static> UiComponent<Action> for TextBlock {
    fn render(self, ui: &Ui<Action>) -> Node<Action> {
        let props = self.props;
        let mut text = RawText::new(props.content)
            .padding(props.padding)
            .flex(props.flex);
        if let Some(color) = props.fg {
            text = text.fg(color);
        }
        if let Some(color) = props.bg {
            text = text.bg(color);
        }
        if props.attrs.bold {
            text = text.bold();
        }
        if props.attrs.italic {
            text = text.italic();
        }
        if props.attrs.dim {
            text = text.dim();
        }
        if props.attrs.underline {
            text = text.underline();
        }
        if let Some(width) = props.width {
            text = text.width(width);
        }
        Node::from_widget(text.build(ui.factory(), ui.theme()))
    }
}

pub struct StatusLineProps {
    text: String,
    fg: Option<AnsiColor>,
    bg: Option<AnsiColor>,
    padding: Option<RectOffset>,
    flex: f32,
}

impl StatusLineProps {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            fg: None,
            bg: None,
            padding: None,
            flex: 0.0,
        }
    }
}

pub struct StatusLine {
    props: StatusLineProps,
}

impl StatusLine {
    pub fn new(text: impl Into<String>) -> Self {
        Self::from_props(StatusLineProps::new(text))
    }

    pub fn from_props(props: StatusLineProps) -> Self {
        Self { props }
    }

    pub fn props(&self) -> &StatusLineProps {
        &self.props
    }

    pub fn into_props(self) -> StatusLineProps {
        self.props
    }

    pub fn fg(mut self, color: AnsiColor) -> Self {
        self.props.fg = Some(color);
        self
    }

    pub fn bg(mut self, color: AnsiColor) -> Self {
        self.props.bg = Some(color);
        self
    }

    pub fn padding(mut self, padding: RectOffset) -> Self {
        self.props.padding = Some(padding);
        self
    }

    pub fn flex(mut self, flex: f32) -> Self {
        self.props.flex = flex;
        self
    }
}

impl<Action: 'static> PropsComponent<Action> for StatusLine {
    type Props = StatusLineProps;

    fn from_props(props: Self::Props) -> Self {
        Self::from_props(props)
    }

    fn into_props(self) -> Self::Props {
        self.into_props()
    }
}

impl<Action: 'static> UiComponent<Action> for StatusLine {
    fn render(self, ui: &Ui<Action>) -> Node<Action> {
        let props = self.props;
        let mut status = RawStatusLine::new(props.text).flex(props.flex);
        if let Some(color) = props.fg {
            status = status.fg(color);
        }
        if let Some(color) = props.bg {
            status = status.bg(color);
        }
        if let Some(padding) = props.padding {
            status = status.padding(padding);
        }
        Node::from_widget(status.build(ui.factory(), ui.theme()))
    }
}

pub struct InputProps<Action> {
    value: String,
    placeholder: String,
    password: bool,
    width: Option<u16>,
    loading: bool,
    loading_phase: usize,
    on_change: Option<Box<dyn Fn(String) -> Action>>,
    on_submit: Option<Box<dyn Fn(String) -> Action>>,
}

impl<Action> Default for InputProps<Action> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Action> InputProps<Action> {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            placeholder: String::new(),
            password: false,
            width: None,
            loading: false,
            loading_phase: 0,
            on_change: None,
            on_submit: None,
        }
    }
}

pub struct Input<Action> {
    props: InputProps<Action>,
}

impl<Action> Default for Input<Action> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Action> Input<Action> {
    pub fn new() -> Self {
        Self::from_props(InputProps::new())
    }

    pub fn from_props(props: InputProps<Action>) -> Self {
        Self { props }
    }

    pub fn props(&self) -> &InputProps<Action> {
        &self.props
    }

    pub fn into_props(self) -> InputProps<Action> {
        self.props
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.props.value = value.into();
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.props.placeholder = placeholder.into();
        self
    }

    pub fn password(mut self) -> Self {
        self.props.password = true;
        self
    }

    pub fn width(mut self, width: u16) -> Self {
        self.props.width = Some(width);
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.props.loading = loading;
        self
    }

    pub fn loading_phase(mut self, phase: usize) -> Self {
        self.props.loading_phase = phase;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(String) -> Action + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }

    pub fn on_submit(mut self, callback: impl Fn(String) -> Action + 'static) -> Self {
        self.props.on_submit = Some(Box::new(callback));
        self
    }
}

impl<Action: 'static> PropsComponent<Action> for Input<Action> {
    type Props = InputProps<Action>;

    fn from_props(props: Self::Props) -> Self {
        Self::from_props(props)
    }

    fn into_props(self) -> Self::Props {
        self.into_props()
    }
}

impl<Action: 'static> UiComponent<Action> for Input<Action> {
    fn render(self, ui: &Ui<Action>) -> Node<Action> {
        let props = self.props;
        let mut input = RawInput::new()
            .value(props.value)
            .placeholder(props.placeholder)
            .loading(props.loading)
            .loading_phase(props.loading_phase);
        if props.password {
            input = input.password();
        }
        if let Some(width) = props.width {
            input = input.width(width);
        }
        if let Some(callback) = props.on_change {
            input = input.on_change(ui.action_callback(callback));
        }
        if let Some(callback) = props.on_submit {
            input = input.on_submit(ui.action_callback(callback));
        }
        Node::from_widget(input.build(ui.factory(), ui.theme()))
    }
}

pub struct PromptBarProps<Action> {
    placeholder: String,
    title: Option<String>,
    rounded: bool,
    on_submit: Option<Box<dyn Fn(String) -> Action>>,
    fg: Option<AnsiColor>,
    bg: Option<AnsiColor>,
    loading: bool,
    loading_phase: usize,
    padding: RectOffset,
    flex: f32,
}

impl<Action> Default for PromptBarProps<Action> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Action> PromptBarProps<Action> {
    pub fn new() -> Self {
        Self {
            placeholder: String::new(),
            title: None,
            rounded: false,
            on_submit: None,
            fg: None,
            bg: None,
            loading: false,
            loading_phase: 0,
            padding: RectOffset::default(),
            flex: 0.0,
        }
    }
}

pub struct PromptBar<Action> {
    props: PromptBarProps<Action>,
}

impl<Action> Default for PromptBar<Action> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Action> PromptBar<Action> {
    pub fn new() -> Self {
        Self::from_props(PromptBarProps::new())
    }

    pub fn from_props(props: PromptBarProps<Action>) -> Self {
        Self { props }
    }

    pub fn props(&self) -> &PromptBarProps<Action> {
        &self.props
    }

    pub fn into_props(self) -> PromptBarProps<Action> {
        self.props
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.props.placeholder = placeholder.into();
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.props.title = Some(title.into());
        self
    }

    pub fn rounded(mut self) -> Self {
        self.props.rounded = true;
        self
    }

    pub fn on_submit(mut self, callback: impl Fn(String) -> Action + 'static) -> Self {
        self.props.on_submit = Some(Box::new(callback));
        self
    }

    pub fn fg(mut self, color: AnsiColor) -> Self {
        self.props.fg = Some(color);
        self
    }

    pub fn bg(mut self, color: AnsiColor) -> Self {
        self.props.bg = Some(color);
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.props.loading = loading;
        self
    }

    pub fn loading_phase(mut self, phase: usize) -> Self {
        self.props.loading_phase = phase;
        self
    }

    pub fn padding(mut self, padding: RectOffset) -> Self {
        self.props.padding = padding;
        self
    }

    pub fn flex(mut self, flex: f32) -> Self {
        self.props.flex = flex;
        self
    }
}

impl<Action: 'static> PropsComponent<Action> for PromptBar<Action> {
    type Props = PromptBarProps<Action>;

    fn from_props(props: Self::Props) -> Self {
        Self::from_props(props)
    }

    fn into_props(self) -> Self::Props {
        self.into_props()
    }
}

impl<Action: 'static> UiComponent<Action> for PromptBar<Action> {
    fn render(self, ui: &Ui<Action>) -> Node<Action> {
        let props = self.props;
        let mut prompt = RawPromptBar::new()
            .placeholder(props.placeholder)
            .loading(props.loading)
            .loading_phase(props.loading_phase)
            .padding(props.padding)
            .flex(props.flex);
        if let Some(title) = props.title {
            prompt = prompt.title(title);
        }
        if props.rounded {
            prompt = prompt.rounded();
        }
        if let Some(color) = props.fg {
            prompt = prompt.fg(color);
        }
        if let Some(color) = props.bg {
            prompt = prompt.bg(color);
        }
        if let Some(callback) = props.on_submit {
            prompt = prompt.on_submit(ui.action_callback(callback));
        }
        Node::from_widget(prompt.build(ui.factory(), ui.theme()))
    }
}

pub struct FuzzyPanelProps<Action> {
    items: Vec<String>,
    title: Option<String>,
    placeholder: String,
    empty_text: String,
    query: String,
    selected_index: usize,
    rounded: bool,
    fg: Option<AnsiColor>,
    bg: Option<AnsiColor>,
    accent: Option<AnsiColor>,
    padding: RectOffset,
    flex: f32,
    on_query_change: Option<Box<dyn Fn(String) -> Action>>,
    on_submit: Option<Box<dyn Fn(FuzzyPanelSelection) -> Action>>,
}

impl<Action> FuzzyPanelProps<Action> {
    pub fn new(items: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            items: items.into_iter().map(Into::into).collect(),
            title: None,
            placeholder: "Search".to_string(),
            empty_text: "No matches".to_string(),
            query: String::new(),
            selected_index: 0,
            rounded: false,
            fg: None,
            bg: None,
            accent: None,
            padding: RectOffset::default(),
            flex: 0.0,
            on_query_change: None,
            on_submit: None,
        }
    }
}

pub struct FuzzyPanel<Action> {
    props: FuzzyPanelProps<Action>,
}

impl<Action> FuzzyPanel<Action> {
    pub fn new(items: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::from_props(FuzzyPanelProps::new(items))
    }

    pub fn from_props(props: FuzzyPanelProps<Action>) -> Self {
        Self { props }
    }

    pub fn props(&self) -> &FuzzyPanelProps<Action> {
        &self.props
    }

    pub fn into_props(self) -> FuzzyPanelProps<Action> {
        self.props
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.props.title = Some(title.into());
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.props.placeholder = placeholder.into();
        self
    }

    pub fn empty_text(mut self, empty_text: impl Into<String>) -> Self {
        self.props.empty_text = empty_text.into();
        self
    }

    pub fn query(mut self, query: impl Into<String>) -> Self {
        self.props.query = query.into();
        self
    }

    pub fn selected_index(mut self, index: usize) -> Self {
        self.props.selected_index = index;
        self
    }

    pub fn rounded(mut self) -> Self {
        self.props.rounded = true;
        self
    }

    pub fn fg(mut self, color: AnsiColor) -> Self {
        self.props.fg = Some(color);
        self
    }

    pub fn bg(mut self, color: AnsiColor) -> Self {
        self.props.bg = Some(color);
        self
    }

    pub fn accent(mut self, color: AnsiColor) -> Self {
        self.props.accent = Some(color);
        self
    }

    pub fn padding(mut self, padding: RectOffset) -> Self {
        self.props.padding = padding;
        self
    }

    pub fn flex(mut self, flex: f32) -> Self {
        self.props.flex = flex;
        self
    }

    pub fn fill(self) -> Self {
        self.flex(1.0)
    }

    pub fn on_query_change(mut self, callback: impl Fn(String) -> Action + 'static) -> Self {
        self.props.on_query_change = Some(Box::new(callback));
        self
    }

    pub fn on_submit(mut self, callback: impl Fn(FuzzyPanelSelection) -> Action + 'static) -> Self {
        self.props.on_submit = Some(Box::new(callback));
        self
    }
}

impl<Action: 'static> PropsComponent<Action> for FuzzyPanel<Action> {
    type Props = FuzzyPanelProps<Action>;

    fn from_props(props: Self::Props) -> Self {
        Self::from_props(props)
    }

    fn into_props(self) -> Self::Props {
        self.into_props()
    }
}

impl<Action: 'static> UiComponent<Action> for FuzzyPanel<Action> {
    fn render(self, ui: &Ui<Action>) -> Node<Action> {
        let props = self.props;
        let theme = ui.theme();
        let mut panel = RawFuzzyPanel::new(props.items)
            .placeholder(props.placeholder)
            .empty_text(props.empty_text)
            .query(props.query)
            .selected_index(props.selected_index)
            .fg(props.fg.unwrap_or_else(|| theme.border()))
            .bg(props.bg.unwrap_or_else(|| theme.surface()))
            .accent(props.accent.unwrap_or_else(|| theme.accent()))
            .padding(props.padding)
            .flex(props.flex);
        if let Some(title) = props.title {
            panel = panel.title(title);
        }
        if props.rounded {
            panel = panel.rounded();
        }
        if let Some(callback) = props.on_query_change {
            panel = panel.on_query_change(ui.action_callback(callback));
        }
        if let Some(callback) = props.on_submit {
            panel = panel.on_submit(ui.action_callback(callback));
        }
        Node::from_widget(panel.build(ui.factory(), theme))
    }
}

pub struct TranscriptProps {
    messages: Vec<TranscriptMessage>,
    empty_text: String,
    notice: Option<TranscriptNotice>,
    scroll_y: Option<ReadSignal<u16>>,
    bg: Option<AnsiColor>,
    flex: f32,
}

impl Default for TranscriptProps {
    fn default() -> Self {
        Self::new()
    }
}

impl TranscriptProps {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            empty_text: String::new(),
            notice: None,
            scroll_y: None,
            bg: None,
            flex: 0.0,
        }
    }

    fn raw(&self) -> RawTranscript {
        let mut transcript = RawTranscript::new()
            .messages(self.messages.clone())
            .empty_text(self.empty_text.clone())
            .notice(self.notice.clone())
            .flex(self.flex);
        if let Some(scroll_y) = self.scroll_y.clone() {
            transcript = transcript.scroll_y(scroll_y);
        }
        if let Some(bg) = self.bg {
            transcript = transcript.bg(bg);
        }
        transcript
    }
}

pub struct Transcript {
    props: TranscriptProps,
}

impl Default for Transcript {
    fn default() -> Self {
        Self::new()
    }
}

impl Transcript {
    pub fn new() -> Self {
        Self::from_props(TranscriptProps::new())
    }

    pub fn from_props(props: TranscriptProps) -> Self {
        Self { props }
    }

    pub fn props(&self) -> &TranscriptProps {
        &self.props
    }

    pub fn into_props(self) -> TranscriptProps {
        self.props
    }

    pub fn messages(mut self, messages: impl IntoIterator<Item = TranscriptMessage>) -> Self {
        self.props.messages = messages.into_iter().collect();
        self
    }

    pub fn empty_text(mut self, empty_text: impl Into<String>) -> Self {
        self.props.empty_text = empty_text.into();
        self
    }

    pub fn notice(mut self, notice: Option<TranscriptNotice>) -> Self {
        self.props.notice = notice;
        self
    }

    pub fn scroll_y(mut self, signal: ReadSignal<u16>) -> Self {
        self.props.scroll_y = Some(signal);
        self
    }

    pub fn bg(mut self, color: AnsiColor) -> Self {
        self.props.bg = Some(color);
        self
    }

    pub fn flex(mut self, flex: f32) -> Self {
        self.props.flex = flex;
        self
    }

    pub fn fill(self) -> Self {
        self.flex(1.0)
    }

    pub fn line_count(&self, theme: &Theme) -> usize {
        self.props.raw().line_count(theme)
    }
}

impl<Action: 'static> PropsComponent<Action> for Transcript {
    type Props = TranscriptProps;

    fn from_props(props: Self::Props) -> Self {
        Self::from_props(props)
    }

    fn into_props(self) -> Self::Props {
        self.into_props()
    }
}

impl<Action: 'static> UiComponent<Action> for Transcript {
    fn render(self, ui: &Ui<Action>) -> Node<Action> {
        Node::from_widget(self.props.raw().build(ui.factory(), ui.theme()))
    }
}

pub struct ColProps<Action> {
    children: Vec<AnyComponent<Action>>,
    padding: RectOffset,
    flex: f32,
    width: Option<u16>,
}

impl<Action> Default for ColProps<Action> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Action> ColProps<Action> {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            padding: RectOffset::default(),
            flex: 0.0,
            width: None,
        }
    }
}

pub struct Col<Action> {
    props: ColProps<Action>,
}

impl<Action> Default for Col<Action> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Action> Col<Action> {
    pub fn new() -> Self {
        Self::from_props(ColProps::new())
    }

    pub fn from_props(props: ColProps<Action>) -> Self {
        Self { props }
    }

    pub fn props(&self) -> &ColProps<Action> {
        &self.props
    }

    pub fn into_props(self) -> ColProps<Action> {
        self.props
    }
}

impl<Action: 'static> Col<Action> {
    pub fn child<Component>(mut self, child: Component) -> Self
    where
        Component: UiComponent<Action>,
    {
        self.props.children.push(AnyComponent::new(child));
        self
    }

    pub fn children<Component>(mut self, children: impl IntoIterator<Item = Component>) -> Self
    where
        Component: UiComponent<Action>,
    {
        self.props
            .children
            .extend(children.into_iter().map(AnyComponent::new));
        self
    }

    pub fn padding(mut self, padding: RectOffset) -> Self {
        self.props.padding = padding;
        self
    }

    pub fn flex(mut self, flex: f32) -> Self {
        self.props.flex = flex;
        self
    }

    pub fn fill(self) -> Self {
        self.flex(1.0)
    }

    pub fn width(mut self, width: u16) -> Self {
        self.props.width = Some(width);
        self
    }
}

impl<Action: 'static> PropsComponent<Action> for Col<Action> {
    type Props = ColProps<Action>;

    fn from_props(props: Self::Props) -> Self {
        Self::from_props(props)
    }

    fn into_props(self) -> Self::Props {
        self.into_props()
    }
}

impl<Action: 'static> UiComponent<Action> for Col<Action> {
    fn render(self, ui: &Ui<Action>) -> Node<Action> {
        let ColProps {
            children,
            padding,
            flex,
            width,
        } = self.props;
        let mut col = RawCol::new()
            .children(
                children
                    .into_iter()
                    .map(|child| child.render(ui).into_widget()),
            )
            .padding(padding)
            .flex(flex);
        if let Some(width) = width {
            col = col.width(width);
        }
        Node::from_widget(col.build(ui.factory(), ui.theme()))
    }
}

pub struct RowProps<Action> {
    children: Vec<AnyComponent<Action>>,
    padding: RectOffset,
    flex: f32,
    width: Option<u16>,
}

impl<Action> Default for RowProps<Action> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Action> RowProps<Action> {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            padding: RectOffset::default(),
            flex: 0.0,
            width: None,
        }
    }
}

pub struct Row<Action> {
    props: RowProps<Action>,
}

impl<Action> Default for Row<Action> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Action> Row<Action> {
    pub fn new() -> Self {
        Self::from_props(RowProps::new())
    }

    pub fn from_props(props: RowProps<Action>) -> Self {
        Self { props }
    }

    pub fn props(&self) -> &RowProps<Action> {
        &self.props
    }

    pub fn into_props(self) -> RowProps<Action> {
        self.props
    }
}

impl<Action: 'static> Row<Action> {
    pub fn child<Component>(mut self, child: Component) -> Self
    where
        Component: UiComponent<Action>,
    {
        self.props.children.push(AnyComponent::new(child));
        self
    }

    pub fn children<Component>(mut self, children: impl IntoIterator<Item = Component>) -> Self
    where
        Component: UiComponent<Action>,
    {
        self.props
            .children
            .extend(children.into_iter().map(AnyComponent::new));
        self
    }

    pub fn padding(mut self, padding: RectOffset) -> Self {
        self.props.padding = padding;
        self
    }

    pub fn flex(mut self, flex: f32) -> Self {
        self.props.flex = flex;
        self
    }

    pub fn fill(self) -> Self {
        self.flex(1.0)
    }

    pub fn width(mut self, width: u16) -> Self {
        self.props.width = Some(width);
        self
    }
}

impl<Action: 'static> PropsComponent<Action> for Row<Action> {
    type Props = RowProps<Action>;

    fn from_props(props: Self::Props) -> Self {
        Self::from_props(props)
    }

    fn into_props(self) -> Self::Props {
        self.into_props()
    }
}

impl<Action: 'static> UiComponent<Action> for Row<Action> {
    fn render(self, ui: &Ui<Action>) -> Node<Action> {
        let RowProps {
            children,
            padding,
            flex,
            width,
        } = self.props;
        let mut row = RawRow::new()
            .children(
                children
                    .into_iter()
                    .map(|child| child.render(ui).into_widget()),
            )
            .padding(padding)
            .flex(flex);
        if let Some(width) = width {
            row = row.width(width);
        }
        Node::from_widget(row.build(ui.factory(), ui.theme()))
    }
}

pub struct PanelProps<Action> {
    body: AnyComponent<Action>,
    title: Option<String>,
    rounded: bool,
    fg: Option<AnsiColor>,
    bg: Option<AnsiColor>,
    padding: RectOffset,
    flex: f32,
}

impl<Action: 'static> PanelProps<Action> {
    pub fn new<Component>(body: Component) -> Self
    where
        Component: UiComponent<Action>,
    {
        Self {
            body: AnyComponent::new(body),
            title: None,
            rounded: true,
            fg: None,
            bg: None,
            padding: RectOffset::default(),
            flex: 0.0,
        }
    }
}

pub struct Panel<Action> {
    props: PanelProps<Action>,
}

impl<Action: 'static> Panel<Action> {
    pub fn new<Component>(body: Component) -> Self
    where
        Component: UiComponent<Action>,
    {
        Self::from_props(PanelProps::new(body))
    }

    pub fn from_props(props: PanelProps<Action>) -> Self {
        Self { props }
    }

    pub fn props(&self) -> &PanelProps<Action> {
        &self.props
    }

    pub fn into_props(self) -> PanelProps<Action> {
        self.props
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.props.title = Some(title.into());
        self
    }

    pub fn rounded(mut self) -> Self {
        self.props.rounded = true;
        self
    }

    pub fn sharp(mut self) -> Self {
        self.props.rounded = false;
        self
    }

    pub fn fg(mut self, color: AnsiColor) -> Self {
        self.props.fg = Some(color);
        self
    }

    pub fn bg(mut self, color: AnsiColor) -> Self {
        self.props.bg = Some(color);
        self
    }

    pub fn padding(mut self, padding: RectOffset) -> Self {
        self.props.padding = padding;
        self
    }

    pub fn flex(mut self, flex: f32) -> Self {
        self.props.flex = flex;
        self
    }

    pub fn fill(self) -> Self {
        self.flex(1.0)
    }
}

impl<Action: 'static> PropsComponent<Action> for Panel<Action> {
    type Props = PanelProps<Action>;

    fn from_props(props: Self::Props) -> Self {
        Self::from_props(props)
    }

    fn into_props(self) -> Self::Props {
        self.into_props()
    }
}

impl<Action: 'static> UiComponent<Action> for Panel<Action> {
    fn render(self, ui: &Ui<Action>) -> Node<Action> {
        let PanelProps {
            body,
            title,
            rounded,
            fg,
            bg,
            padding,
            flex,
        } = self.props;
        let mut panel = RawPanel::new(body.render(ui).into_widget())
            .padding(padding)
            .flex(flex);
        if rounded {
            panel = panel.rounded();
        }
        if let Some(title) = title {
            panel = panel.title(title);
        }
        if let Some(color) = fg {
            panel = panel.fg(color);
        }
        if let Some(color) = bg {
            panel = panel.bg(color);
        }
        Node::from_widget(panel.build(ui.factory(), ui.theme()))
    }
}

pub struct PageProps<Action> {
    title: Option<String>,
    header: Option<AnyComponent<Action>>,
    body: Option<AnyComponent<Action>>,
    footer: Option<AnyComponent<Action>>,
}

impl<Action> Default for PageProps<Action> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Action> PageProps<Action> {
    pub fn new() -> Self {
        Self {
            title: None,
            header: None,
            body: None,
            footer: None,
        }
    }
}

pub struct Page<Action> {
    props: PageProps<Action>,
}

impl<Action> Default for Page<Action> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Action> Page<Action> {
    pub fn new() -> Self {
        Self::from_props(PageProps::new())
    }

    pub fn from_props(props: PageProps<Action>) -> Self {
        Self { props }
    }

    pub fn props(&self) -> &PageProps<Action> {
        &self.props
    }

    pub fn into_props(self) -> PageProps<Action> {
        self.props
    }
}

impl<Action: 'static> Page<Action> {
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.props.title = Some(title.into());
        self
    }

    pub fn header<Component>(mut self, header: Component) -> Self
    where
        Component: UiComponent<Action>,
    {
        self.props.header = Some(AnyComponent::new(header));
        self
    }

    pub fn body<Component>(mut self, body: Component) -> Self
    where
        Component: UiComponent<Action>,
    {
        self.props.body = Some(AnyComponent::new(body));
        self
    }

    pub fn footer<Component>(mut self, footer: Component) -> Self
    where
        Component: UiComponent<Action>,
    {
        self.props.footer = Some(AnyComponent::new(footer));
        self
    }
}

impl<Action: 'static> PropsComponent<Action> for Page<Action> {
    type Props = PageProps<Action>;

    fn from_props(props: Self::Props) -> Self {
        Self::from_props(props)
    }

    fn into_props(self) -> Self::Props {
        self.into_props()
    }
}

impl<Action: 'static> UiComponent<Action> for Page<Action> {
    fn render(self, ui: &Ui<Action>) -> Node<Action> {
        let PageProps {
            title,
            header,
            body,
            footer,
        } = self.props;
        let mut children = Vec::new();
        if let Some(title) = title {
            children.push(RawStatusLine::new(title).build(ui.factory(), ui.theme()));
        }
        if let Some(header) = header {
            children.push(header.render(ui).into_widget());
        }
        if let Some(body) = body {
            children.push(body.render(ui).into_widget());
        }
        if let Some(footer) = footer {
            children.push(footer.render(ui).into_widget());
        }

        Node::from_widget(
            RawCol::new()
                .flex(1.0)
                .children(children)
                .build(ui.factory(), ui.theme()),
        )
    }
}
