use std::cell::RefCell;
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::rc::Rc;

use arbor_tui_composites::{Panel, PromptBar, StatusLine};
use arbor_tui_domain::cell::AnsiColor;
use arbor_tui_domain::layout::RectOffset;
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::WidgetNode;
use arbor_tui_widgets::stack::{Col, Row};
use arbor_tui_widgets::text::Text;
use arbor_tui_widgets::widget_factory::WidgetFactory;

pub(crate) struct ActionSink<Action> {
    queue: Rc<RefCell<VecDeque<Action>>>,
}

impl<Action> Clone for ActionSink<Action> {
    fn clone(&self) -> Self {
        Self {
            queue: Rc::clone(&self.queue),
        }
    }
}

impl<Action> ActionSink<Action> {
    pub(crate) fn new() -> Self {
        Self {
            queue: Rc::new(RefCell::new(VecDeque::new())),
        }
    }

    pub(crate) fn push(&self, action: Action) {
        self.queue.borrow_mut().push_back(action);
    }

    pub(crate) fn pop(&self) -> Option<Action> {
        self.queue.borrow_mut().pop_front()
    }
}

pub struct Node<Action> {
    widget: WidgetNode,
    action: PhantomData<fn() -> Action>,
}

impl<Action> Node<Action> {
    pub fn from_widget(widget: WidgetNode) -> Self {
        Self {
            widget,
            action: PhantomData,
        }
    }

    pub fn into_widget(self) -> WidgetNode {
        self.widget
    }
}

pub struct Ui<Action> {
    factory: Rc<WidgetFactory>,
    theme: Theme,
    actions: ActionSink<Action>,
}

impl<Action> Clone for Ui<Action> {
    fn clone(&self) -> Self {
        Self {
            factory: Rc::clone(&self.factory),
            theme: self.theme.clone(),
            actions: self.actions.clone(),
        }
    }
}

impl<Action: 'static> Ui<Action> {
    pub(crate) fn new(
        factory: Rc<WidgetFactory>,
        theme: Theme,
        actions: ActionSink<Action>,
    ) -> Self {
        Self {
            factory,
            theme,
            actions,
        }
    }

    pub fn text(&self, text: impl Into<String>) -> Node<Action> {
        Node::from_widget(Text::new(text.into()).build(&self.factory, &self.theme))
    }

    pub fn status_line(&self, text: impl Into<String>) -> Node<Action> {
        Node::from_widget(StatusLine::new(text.into()).build(&self.factory, &self.theme))
    }

    pub fn prompt(&self, placeholder: impl Into<String>) -> PromptBuilder<Action> {
        PromptBuilder {
            ui: self.clone(),
            placeholder: placeholder.into(),
            loading: false,
            loading_phase: 0,
            on_submit: None,
        }
    }

    pub fn row(&self) -> RowBuilder<Action> {
        RowBuilder {
            ui: self.clone(),
            children: Vec::new(),
            flex: 0.0,
            width: None,
            padding: RectOffset::default(),
        }
    }

    pub fn col(&self) -> ColBuilder<Action> {
        ColBuilder {
            ui: self.clone(),
            children: Vec::new(),
            flex: 0.0,
            width: None,
            padding: RectOffset::default(),
        }
    }

    pub fn panel(&self, body: Node<Action>) -> PanelBuilder<Action> {
        PanelBuilder {
            ui: self.clone(),
            body,
            title: None,
            flex: 0.0,
            fg: None,
            bg: None,
            rounded: true,
        }
    }

    pub fn page(&self) -> PageBuilder<Action> {
        PageBuilder {
            ui: self.clone(),
            title: None,
            header: None,
            body: None,
            footer: None,
        }
    }
}

pub struct PromptBuilder<Action> {
    ui: Ui<Action>,
    placeholder: String,
    loading: bool,
    loading_phase: usize,
    on_submit: Option<Box<dyn Fn(String) -> Action>>,
}

impl<Action: 'static> PromptBuilder<Action> {
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    pub fn loading_phase(mut self, phase: usize) -> Self {
        self.loading_phase = phase;
        self
    }

    pub fn on_submit(mut self, callback: impl Fn(String) -> Action + 'static) -> Self {
        self.on_submit = Some(Box::new(callback));
        self
    }

    pub fn build(self) -> Node<Action> {
        let mut prompt = PromptBar::new()
            .placeholder(self.placeholder)
            .loading(self.loading)
            .loading_phase(self.loading_phase);
        if let Some(callback) = self.on_submit {
            let actions = self.ui.actions.clone();
            prompt = prompt.on_submit(move |text| {
                actions.push(callback(text));
            });
        }
        Node::from_widget(prompt.build(&self.ui.factory, &self.ui.theme))
    }
}

pub struct RowBuilder<Action> {
    ui: Ui<Action>,
    children: Vec<Node<Action>>,
    flex: f32,
    width: Option<u16>,
    padding: RectOffset,
}

impl<Action: 'static> RowBuilder<Action> {
    pub fn child(mut self, child: Node<Action>) -> Self {
        self.children.push(child);
        self
    }

    pub fn fill(mut self) -> Self {
        self.flex = 1.0;
        self
    }

    pub fn width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    pub fn padding(mut self, padding: RectOffset) -> Self {
        self.padding = padding;
        self
    }

    pub fn build(self) -> Node<Action> {
        let mut row = Row::new()
            .children(self.children.into_iter().map(Node::into_widget))
            .padding(self.padding)
            .flex(self.flex);
        if let Some(width) = self.width {
            row = row.width(width);
        }
        Node::from_widget(row.build(&self.ui.factory, &self.ui.theme))
    }
}

pub struct ColBuilder<Action> {
    ui: Ui<Action>,
    children: Vec<Node<Action>>,
    flex: f32,
    width: Option<u16>,
    padding: RectOffset,
}

impl<Action: 'static> ColBuilder<Action> {
    pub fn child(mut self, child: Node<Action>) -> Self {
        self.children.push(child);
        self
    }

    pub fn fill(mut self) -> Self {
        self.flex = 1.0;
        self
    }

    pub fn width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    pub fn padding(mut self, padding: RectOffset) -> Self {
        self.padding = padding;
        self
    }

    pub fn build(self) -> Node<Action> {
        let mut col = Col::new()
            .children(self.children.into_iter().map(Node::into_widget))
            .padding(self.padding)
            .flex(self.flex);
        if let Some(width) = self.width {
            col = col.width(width);
        }
        Node::from_widget(col.build(&self.ui.factory, &self.ui.theme))
    }
}

pub struct PanelBuilder<Action> {
    ui: Ui<Action>,
    body: Node<Action>,
    title: Option<String>,
    flex: f32,
    fg: Option<AnsiColor>,
    bg: Option<AnsiColor>,
    rounded: bool,
}

impl<Action: 'static> PanelBuilder<Action> {
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn fill(mut self) -> Self {
        self.flex = 1.0;
        self
    }

    pub fn fg(mut self, color: AnsiColor) -> Self {
        self.fg = Some(color);
        self
    }

    pub fn bg(mut self, color: AnsiColor) -> Self {
        self.bg = Some(color);
        self
    }

    pub fn sharp(mut self) -> Self {
        self.rounded = false;
        self
    }

    pub fn build(self) -> Node<Action> {
        let mut panel = Panel::new(self.body.into_widget()).flex(self.flex);
        if self.rounded {
            panel = panel.rounded();
        }
        if let Some(title) = self.title {
            panel = panel.title(title);
        }
        if let Some(color) = self.fg {
            panel = panel.fg(color);
        }
        if let Some(color) = self.bg {
            panel = panel.bg(color);
        }
        Node::from_widget(panel.build(&self.ui.factory, &self.ui.theme))
    }
}

pub struct PageBuilder<Action> {
    ui: Ui<Action>,
    title: Option<String>,
    header: Option<Node<Action>>,
    body: Option<Node<Action>>,
    footer: Option<Node<Action>>,
}

impl<Action: 'static> PageBuilder<Action> {
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn header(mut self, header: Node<Action>) -> Self {
        self.header = Some(header);
        self
    }

    pub fn body(mut self, body: Node<Action>) -> Self {
        self.body = Some(body);
        self
    }

    pub fn footer(mut self, footer: Node<Action>) -> Self {
        self.footer = Some(footer);
        self
    }

    pub fn build(self) -> Node<Action> {
        let mut children = Vec::new();
        if let Some(title) = self.title {
            children.push(StatusLine::new(title).build(&self.ui.factory, &self.ui.theme));
        }
        if let Some(header) = self.header {
            children.push(header.into_widget());
        }
        if let Some(body) = self.body {
            children.push(body.into_widget());
        }
        if let Some(footer) = self.footer {
            children.push(footer.into_widget());
        }

        Node::from_widget(
            Col::new()
                .flex(1.0)
                .children(children)
                .build(&self.ui.factory, &self.ui.theme),
        )
    }
}

pub(crate) fn build_root<Action: 'static>(
    factory: &Rc<WidgetFactory>,
    theme: &Theme,
    cols: u16,
    rows: u16,
    node: Node<Action>,
) -> WidgetNode {
    Col::new()
        .size(cols, rows)
        .children([node.into_widget()])
        .build(factory, theme)
}
