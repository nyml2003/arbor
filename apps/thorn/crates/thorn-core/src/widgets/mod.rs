use std::rc::Rc;

use crate::view::{
    fuzzy_panel_view, input_view, scroll_area_view, text_view, transcript_view, with_children,
    FuzzyPanelSelection, IntoText, IntoViewVec, NodeKind, TranscriptMessage, TranscriptNotice,
    View,
};

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

pub fn input<Action>() -> Input<Action> {
    Input::new()
}

pub struct Input<Action> {
    value: String,
    placeholder: String,
    loading: bool,
    loading_phase: usize,
    password: bool,
    on_change: Option<Box<dyn Fn(String) -> Action>>,
    on_submit: Option<Box<dyn Fn(String) -> Action>>,
}

impl<Action> Input<Action> {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            placeholder: String::new(),
            loading: false,
            loading_phase: 0,
            password: false,
            on_change: None,
            on_submit: None,
        }
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    pub fn loading_phase(mut self, phase: usize) -> Self {
        self.loading_phase = phase;
        self
    }

    pub fn password(mut self) -> Self {
        self.password = true;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(String) -> Action + 'static) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }

    pub fn on_submit(mut self, callback: impl Fn(String) -> Action + 'static) -> Self {
        self.on_submit = Some(Box::new(callback));
        self
    }

    pub fn build(self) -> View<Action> {
        input_view(
            self.value,
            self.placeholder,
            self.loading,
            self.loading_phase,
            self.password,
            self.on_change,
            self.on_submit,
        )
    }
}

impl<Action> Default for Input<Action> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Action> From<Input<Action>> for View<Action> {
    fn from(value: Input<Action>) -> Self {
        value.build()
    }
}

pub fn scroll_area<Action>(child: View<Action>) -> ScrollArea<Action> {
    ScrollArea {
        child,
        scroll_y: None,
    }
}

pub struct ScrollArea<Action> {
    child: View<Action>,
    scroll_y: Option<Rc<dyn Fn() -> u16>>,
}

impl<Action> ScrollArea<Action> {
    pub fn scroll_y(mut self, read: impl Fn() -> u16 + 'static) -> Self {
        self.scroll_y = Some(Rc::new(read));
        self
    }

    pub fn build(self) -> View<Action> {
        scroll_area_view(self.child, self.scroll_y)
    }
}

impl<Action> From<ScrollArea<Action>> for View<Action> {
    fn from(value: ScrollArea<Action>) -> Self {
        value.build()
    }
}

pub fn transcript<Action>() -> Transcript<Action> {
    Transcript::new()
}

pub struct Transcript<Action> {
    messages: Vec<TranscriptMessage>,
    empty_text: String,
    notice: Option<TranscriptNotice>,
    scroll_y: Option<Rc<dyn Fn() -> u16>>,
    _action: std::marker::PhantomData<Action>,
}

impl<Action> Transcript<Action> {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            empty_text: String::new(),
            notice: None,
            scroll_y: None,
            _action: std::marker::PhantomData,
        }
    }

    pub fn messages(mut self, messages: impl IntoIterator<Item = TranscriptMessage>) -> Self {
        self.messages = messages.into_iter().collect();
        self
    }

    pub fn empty_text(mut self, empty_text: impl Into<String>) -> Self {
        self.empty_text = empty_text.into();
        self
    }

    pub fn notice(mut self, notice: Option<TranscriptNotice>) -> Self {
        self.notice = notice;
        self
    }

    pub fn scroll_y(mut self, read: impl Fn() -> u16 + 'static) -> Self {
        self.scroll_y = Some(Rc::new(read));
        self
    }

    pub fn line_count(&self) -> usize {
        transcript_line_count(&self.messages, self.notice.as_ref(), &self.empty_text)
    }

    pub fn build(self) -> View<Action> {
        transcript_view(self.messages, self.empty_text, self.notice, self.scroll_y)
    }
}

impl<Action> Default for Transcript<Action> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Action> From<Transcript<Action>> for View<Action> {
    fn from(value: Transcript<Action>) -> Self {
        value.build()
    }
}

pub fn transcript_line_count(
    messages: &[TranscriptMessage],
    notice: Option<&TranscriptNotice>,
    empty_text: &str,
) -> usize {
    let notice_lines = notice.map_or(0, |_| 2);
    if messages.is_empty() {
        return empty_text.lines().count().max(1) + notice_lines;
    }
    messages
        .iter()
        .map(|message| 1 + message.body.lines().count().max(1) + 1)
        .sum::<usize>()
        + notice_lines
}

pub fn fuzzy_panel<Action>(
    items: impl IntoIterator<Item = impl Into<String>>,
) -> FuzzyPanel<Action> {
    FuzzyPanel::new(items)
}

pub struct FuzzyPanel<Action> {
    items: Vec<String>,
    title: Option<String>,
    placeholder: String,
    empty_text: String,
    query: String,
    selected: usize,
    on_move: Option<Box<dyn Fn(i32) -> Action>>,
    on_query_change: Option<Box<dyn Fn(String) -> Action>>,
    on_submit: Option<Box<dyn Fn(FuzzyPanelSelection) -> Action>>,
}

impl<Action> FuzzyPanel<Action> {
    pub fn new(items: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            items: items.into_iter().map(Into::into).collect(),
            title: None,
            placeholder: String::new(),
            empty_text: String::new(),
            query: String::new(),
            selected: 0,
            on_move: None,
            on_query_change: None,
            on_submit: None,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn empty_text(mut self, empty_text: impl Into<String>) -> Self {
        self.empty_text = empty_text.into();
        self
    }

    pub fn query(mut self, query: impl Into<String>) -> Self {
        self.query = query.into();
        self
    }

    pub fn selected_index(mut self, selected: usize) -> Self {
        self.selected = selected;
        self
    }

    pub fn on_move_selection(mut self, callback: impl Fn(i32) -> Action + 'static) -> Self {
        self.on_move = Some(Box::new(callback));
        self
    }

    pub fn on_query_change(mut self, callback: impl Fn(String) -> Action + 'static) -> Self {
        self.on_query_change = Some(Box::new(callback));
        self
    }

    pub fn on_submit(mut self, callback: impl Fn(FuzzyPanelSelection) -> Action + 'static) -> Self {
        self.on_submit = Some(Box::new(callback));
        self
    }

    pub fn build(self) -> View<Action> {
        fuzzy_panel_view(
            self.items,
            self.title,
            self.placeholder,
            self.empty_text,
            self.query,
            self.selected,
            self.on_move,
            self.on_query_change,
            self.on_submit,
        )
    }
}

impl<Action> From<FuzzyPanel<Action>> for View<Action> {
    fn from(value: FuzzyPanel<Action>) -> Self {
        value.build()
    }
}
