use std::collections::HashMap;

use crate::layout::{LayoutInfo, Rect, Size};
use crate::reactive::Scope;
use crate::render::{diff, render_tree, DirtyRegion, Screen};
use crate::runtime::{Key, KeyEvent, KeyMap, RuntimeInput};
use crate::theme::{Color, Theme};
use crate::view::{handle_key, NodeId, PrimitiveNode, View};

pub struct TestApp<Action = ()> {
    scope: Scope,
    root: View<Action>,
    theme: Theme,
    screen: Option<Screen>,
    layout: HashMap<NodeId, LayoutInfo>,
    dirty_regions: Vec<DirtyRegion>,
}

type BeforeEvents<Action> = Box<dyn FnMut(&mut Vec<RuntimeInput>, &mut Vec<Action>)>;
type BeforeRender<State> = Box<dyn FnMut(&mut State)>;
type Update<State, Action> = Box<dyn FnMut(&mut State, Action)>;
type RuntimeView<State, Action> = Box<dyn FnMut(&Scope, &State) -> View<Action>>;

pub struct TestRuntime<State, Action = ()> {
    state: State,
    update: Update<State, Action>,
    view: RuntimeView<State, Action>,
    keymap: KeyMap<Action>,
    before_events: BeforeEvents<Action>,
    before_render: BeforeRender<State>,
    theme: Theme,
    width: u16,
    height: u16,
    pending_inputs: Vec<RuntimeInput>,
    screen: Option<Screen>,
    layout: HashMap<NodeId, LayoutInfo>,
    dirty_regions: Vec<DirtyRegion>,
    scope: Option<Scope>,
    root: Option<View<Action>>,
    should_exit: bool,
}

impl<State, Action: Clone> TestRuntime<State, Action> {
    pub fn new(
        initial_state: State,
        update: impl FnMut(&mut State, Action) + 'static,
        view: impl FnMut(&Scope, &State) -> View<Action> + 'static,
    ) -> Self {
        Self {
            state: initial_state,
            update: Box::new(update),
            view: Box::new(view),
            keymap: KeyMap::new(),
            before_events: Box::new(|_, _| {}),
            before_render: Box::new(|_| {}),
            theme: Theme::dark(),
            width: 80,
            height: 24,
            pending_inputs: Vec::new(),
            screen: None,
            layout: HashMap::new(),
            dirty_regions: Vec::new(),
            scope: None,
            root: None,
            should_exit: false,
        }
    }

    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn before_events(
        mut self,
        before_events: impl FnMut(&mut Vec<RuntimeInput>, &mut Vec<Action>) + 'static,
    ) -> Self {
        self.before_events = Box::new(before_events);
        self
    }

    pub fn with_keymap(mut self, keymap: KeyMap<Action>) -> Self {
        self.keymap = keymap;
        self
    }

    pub fn before_render(mut self, before_render: impl FnMut(&mut State) + 'static) -> Self {
        self.before_render = Box::new(before_render);
        self
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn should_exit(&self) -> bool {
        self.should_exit
    }

    pub fn send_key(&mut self, key: Key) {
        self.send_input(RuntimeInput::Key(KeyEvent::new(key)));
    }

    pub fn send_input(&mut self, input: RuntimeInput) {
        self.pending_inputs.push(input);
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.send_input(RuntimeInput::Resize(Size::new(width, height)));
    }

    pub fn render_frame(&mut self) {
        self.process_pending_inputs();
        if self.should_exit {
            return;
        }

        (self.before_render)(&mut self.state);

        if let Some(scope) = self.scope.take() {
            scope.dispose();
        }

        let scope = Scope::new();
        let root = scope.enter(|| (self.view)(&scope, &self.state));
        let (next_screen, next_layout) = render_tree(&root, self.width, self.height, &self.theme);
        self.dirty_regions = self
            .screen
            .as_ref()
            .map(|old| diff(old, &next_screen))
            .unwrap_or_else(|| {
                vec![DirtyRegion {
                    rect: Rect::new(0, 0, self.width, self.height),
                }]
            });
        self.screen = Some(next_screen);
        self.layout = next_layout;
        self.root = Some(root);
        self.scope = Some(scope);
    }

    pub fn assert_text(&self, text: &str) {
        let Some(screen) = &self.screen else {
            panic!("render_frame must run before asserting text");
        };
        assert!(
            screen.contains_text(text),
            "screen did not contain text `{text}`"
        );
    }

    pub fn dirty_regions(&self) -> &[DirtyRegion] {
        &self.dirty_regions
    }

    pub fn screen(&self) -> Option<&Screen> {
        self.screen.as_ref()
    }

    fn process_pending_inputs(&mut self) {
        let mut inputs = std::mem::take(&mut self.pending_inputs);
        if inputs.is_empty() {
            return;
        }

        for input in &inputs {
            if let RuntimeInput::Resize(size) = input {
                self.width = size.w;
                self.height = size.h;
                self.screen = None;
            }
        }

        let mut actions = inputs
            .iter()
            .filter_map(|input| match input {
                RuntimeInput::Key(event) => self.keymap.action_for(event),
                RuntimeInput::Resize(_) | RuntimeInput::Tick => None,
            })
            .collect::<Vec<_>>();
        (self.before_events)(&mut inputs, &mut actions);
        if let Some(root) = &self.root {
            for input in &inputs {
                if let RuntimeInput::Key(event) = input {
                    if let Some(action) = handle_key(root.node(), event) {
                        actions.push(action);
                    }
                }
            }
        }
        if actions.is_empty() && inputs.iter().copied().any(RuntimeInput::is_default_exit) {
            self.should_exit = true;
            return;
        }
        for action in actions {
            (self.update)(&mut self.state, action);
        }
    }
}

impl<Action> TestApp<Action> {
    pub fn new(root: impl FnOnce(&Scope) -> View<Action>) -> Self {
        let scope = Scope::new();
        let root = scope.enter(|| root(&scope));
        Self {
            scope,
            root,
            theme: Theme::dark(),
            screen: None,
            layout: HashMap::new(),
            dirty_regions: Vec::new(),
        }
    }

    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn render(&mut self, width: u16, height: u16) {
        let (next_screen, next_layout) = render_tree(&self.root, width, height, &self.theme);
        self.dirty_regions = self
            .screen
            .as_ref()
            .map(|old| diff(old, &next_screen))
            .unwrap_or_else(|| {
                vec![DirtyRegion {
                    rect: Rect::new(0, 0, width, height),
                }]
            });
        self.screen = Some(next_screen);
        self.layout = next_layout;
    }

    pub fn assert_text(&self, text: &str) {
        let Some(screen) = &self.screen else {
            panic!("render must run before asserting text");
        };
        assert!(
            screen.contains_text(text),
            "screen did not contain text `{text}`"
        );
    }

    pub fn layout_of(&self, key_or_text: &str) -> Rect {
        let Some(node) = find_by_key_or_text(self.root.node(), key_or_text) else {
            panic!("node `{key_or_text}` not found");
        };
        self.layout
            .get(&node.id())
            .map(|info| info.rect)
            .unwrap_or_else(|| panic!("node `{key_or_text}` has no layout info"))
    }

    pub fn dirty_regions(&self) -> &[DirtyRegion] {
        &self.dirty_regions
    }

    pub fn screen(&self) -> Option<&Screen> {
        self.screen.as_ref()
    }

    pub fn assert_no_default_bg_on_text(&self) {
        let Some(screen) = &self.screen else {
            panic!("render must run before asserting backgrounds");
        };
        for y in 0..screen.height() {
            for x in 0..screen.width() {
                let cell = screen.get(x, y);
                if cell.ch != ' ' {
                    assert_ne!(
                        cell.bg,
                        Color::Palette(0),
                        "text cell at ({x}, {y}) leaked default black background"
                    );
                }
            }
        }
    }

    pub fn dispose(&self) {
        self.scope.dispose();
    }
}

fn find_by_key_or_text<'a, Action>(
    node: &'a PrimitiveNode<Action>,
    key_or_text: &str,
) -> Option<&'a PrimitiveNode<Action>> {
    if node.key() == Some(key_or_text) || node.text().as_deref() == Some(key_or_text) {
        return Some(node);
    }

    node.children()
        .iter()
        .find_map(|child| find_by_key_or_text(child, key_or_text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    #[test]
    fn harness_renders_text_and_reads_layout() {
        let mut app: TestApp = TestApp::new(|_| panel(text("hello")).id("panel"));
        app.render(20, 4);

        app.assert_text("hello");
        assert_eq!(app.layout_of("panel").w, 20);
    }

    #[test]
    fn light_theme_has_no_default_black_text_background() {
        let mut app: TestApp = TestApp::new(|_| text("hello")).with_theme(Theme::light());
        app.render(20, 4);

        app.assert_no_default_bg_on_text();
    }

    #[derive(Clone)]
    enum CounterAction {
        Increment,
    }

    #[test]
    fn test_runtime_maps_key_input_to_action_state_and_view() {
        let mut app = TestRuntime::new(
            0i32,
            |state, action| match action {
                CounterAction::Increment => *state += 1,
            },
            |_, state| text(format!("count: {state}")),
        )
        .with_keymap(KeyMap::new().bind(Key::Char('+'), CounterAction::Increment));

        app.render_frame();
        app.assert_text("count: 0");

        app.send_key(Key::Char('+'));
        app.render_frame();

        assert_eq!(*app.state(), 1);
        app.assert_text("count: 1");
    }

    #[test]
    fn before_render_can_advance_state_without_input() {
        let mut app = TestRuntime::new(
            0usize,
            |_, ()| {},
            |_, state| text(format!("tick: {state}")),
        )
        .before_render(|state| *state += 1);

        app.render_frame();
        app.assert_text("tick: 1");
        app.render_frame();
        app.assert_text("tick: 2");
    }

    #[test]
    fn enter_without_handler_is_noop_and_escape_requests_exit() {
        let mut app = TestRuntime::new(
            0usize,
            |_, ()| {},
            |_, state| text(format!("count: {state}")),
        );

        app.render_frame();
        app.send_key(Key::Enter);
        app.render_frame();

        assert_eq!(*app.state(), 0);
        app.assert_text("count: 0");

        app.send_key(Key::Escape);
        app.render_frame();

        assert!(app.should_exit());
    }

    #[test]
    fn resize_makes_next_frame_fully_dirty() {
        let mut app = TestRuntime::new((), |_, ()| {}, |_, _| text("resize"));

        app.render_frame();
        app.resize(40, 8);
        app.render_frame();

        assert_eq!(app.dirty_regions()[0].rect, Rect::new(0, 0, 40, 8));
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum InputAction {
        DraftChanged(String),
        Submitted(String),
    }

    #[test]
    fn input_component_emits_change_and_submit_actions() {
        #[derive(Default)]
        struct State {
            draft: String,
            submitted: String,
        }

        let mut app = TestRuntime::new(
            State::default(),
            |state, action| match action {
                InputAction::DraftChanged(draft) => state.draft = draft,
                InputAction::Submitted(value) => state.submitted = value,
            },
            |_, state| {
                input()
                    .value(state.draft.clone())
                    .placeholder("Type")
                    .on_change(InputAction::DraftChanged)
                    .on_submit(InputAction::Submitted)
                    .build()
            },
        );

        app.render_frame();
        app.assert_text("Type");
        app.send_key(Key::Char('h'));
        app.render_frame();
        app.send_key(Key::Char('i'));
        app.render_frame();
        app.send_key(Key::Enter);
        app.render_frame();

        assert_eq!(app.state().draft, "hi");
        assert_eq!(app.state().submitted, "hi");
        app.assert_text("hi");
    }

    #[test]
    fn transcript_renders_plain_messages_notice_and_scroll() {
        let messages = vec![
            TranscriptMessage::new("You", Token::Accent, "first\nsecond"),
            TranscriptMessage::new("Aster", Token::Primary, "reply"),
        ];
        let scroll = 2u16;
        let mut app: TestApp = TestApp::new(move |_| {
            transcript()
                .messages(messages.clone())
                .notice(Some(TranscriptNotice::new(
                    "Error",
                    "try again",
                    Token::Danger,
                )))
                .scroll_y(move || scroll)
                .build()
        });

        app.render(40, 8);

        app.assert_text("second");
        app.assert_text("Aster:");
        app.assert_text("Error");
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum PaletteAction {
        Query(String),
        Move(i32),
        Submit(String),
    }

    #[test]
    fn fuzzy_panel_filters_and_submits_selection() {
        #[derive(Default)]
        struct State {
            query: String,
            selected: usize,
            submitted: String,
        }

        let mut app = TestRuntime::new(
            State::default(),
            |state, action| match action {
                PaletteAction::Query(query) => {
                    state.query = query;
                    state.selected = 0;
                }
                PaletteAction::Move(delta) => {
                    state.selected = if delta < 0 {
                        state.selected.saturating_sub(1)
                    } else {
                        state.selected.saturating_add(1).min(1)
                    };
                }
                PaletteAction::Submit(item) => state.submitted = item,
            },
            |_, state| {
                fuzzy_panel(["/theme", "/model"])
                    .title(" Commands ")
                    .placeholder("Filter")
                    .empty_text("No command matches")
                    .query(state.query.clone())
                    .selected_index(state.selected)
                    .on_move_selection(PaletteAction::Move)
                    .on_query_change(PaletteAction::Query)
                    .on_submit(|selection| PaletteAction::Submit(selection.item))
                    .build()
            },
        );

        app.render_frame();
        app.assert_text("/theme");
        app.send_key(Key::ArrowDown);
        app.render_frame();
        app.send_key(Key::Enter);
        app.render_frame();
        assert_eq!(app.state().submitted, "/model");

        app.send_key(Key::Char('m'));
        app.render_frame();
        app.assert_text("/model");
        app.send_key(Key::Enter);
        app.render_frame();

        assert_eq!(app.state().query, "m");
        assert_eq!(app.state().submitted, "/model");
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum ShellAction {
        DraftChanged(String),
        SubmitPrompt(String),
        TogglePalette,
        PaletteQuery(String),
        PaletteMove(i32),
        PaletteSubmit(String),
        Tick,
    }

    #[test]
    fn aster_like_shell_flow_composes_transcript_input_and_palette() {
        struct ShellState {
            draft: String,
            messages: Vec<TranscriptMessage>,
            palette_open: bool,
            palette_query: String,
            palette_selected: usize,
            command: String,
            loading_phase: usize,
        }

        let commands = ["/clear", "/model", "/theme"];
        let mut app = TestRuntime::new(
            ShellState {
                draft: String::new(),
                messages: vec![TranscriptMessage::new("Aster", Token::Primary, "ready")],
                palette_open: false,
                palette_query: String::new(),
                palette_selected: 0,
                command: String::new(),
                loading_phase: 0,
            },
            move |state, action| match action {
                ShellAction::DraftChanged(draft) => state.draft = draft,
                ShellAction::SubmitPrompt(prompt) => {
                    if !prompt.is_empty() {
                        state.messages.push(TranscriptMessage::new(
                            "You",
                            Token::Accent,
                            prompt.clone(),
                        ));
                        state.messages.push(TranscriptMessage::new(
                            "Aster",
                            Token::Primary,
                            format!("echo: {prompt}"),
                        ));
                        state.draft.clear();
                    }
                }
                ShellAction::TogglePalette => {
                    state.palette_open = !state.palette_open;
                    state.palette_query.clear();
                    state.palette_selected = 0;
                }
                ShellAction::PaletteQuery(query) => {
                    state.palette_query = query;
                    state.palette_selected = 0;
                }
                ShellAction::PaletteMove(delta) => {
                    state.palette_selected = if delta < 0 {
                        state.palette_selected.saturating_sub(1)
                    } else {
                        state
                            .palette_selected
                            .saturating_add(1)
                            .min(commands.len().saturating_sub(1))
                    };
                }
                ShellAction::PaletteSubmit(command) => {
                    state.command = command.clone();
                    state.draft = format!("{command} ");
                    state.palette_open = false;
                }
                ShellAction::Tick => state.loading_phase = state.loading_phase.wrapping_add(1),
            },
            move |_, state| {
                let history = transcript()
                    .messages(state.messages.clone())
                    .empty_text("No messages")
                    .build()
                    .height(8);
                let prompt = input()
                    .value(state.draft.clone())
                    .placeholder("Message Aster")
                    .loading_phase(state.loading_phase)
                    .on_change(ShellAction::DraftChanged)
                    .on_submit(ShellAction::SubmitPrompt)
                    .build();
                if state.palette_open {
                    col((
                        fuzzy_panel(commands)
                            .title(" Commands ")
                            .placeholder("Filter")
                            .empty_text("No command matches")
                            .query(state.palette_query.clone())
                            .selected_index(state.palette_selected)
                            .on_move_selection(ShellAction::PaletteMove)
                            .on_query_change(ShellAction::PaletteQuery)
                            .on_submit(|selection| ShellAction::PaletteSubmit(selection.item))
                            .build()
                            .height(6),
                        history,
                        prompt,
                    ))
                } else {
                    col((history, prompt))
                }
            },
        )
        .with_keymap(KeyMap::new().bind(Key::Tab, ShellAction::TogglePalette))
        .before_events(|inputs, actions| {
            if inputs
                .iter()
                .any(|input| matches!(input, RuntimeInput::Tick))
            {
                actions.push(ShellAction::Tick);
            }
        });

        app.render_frame();
        app.assert_text("ready");
        app.assert_text("Message Aster");

        app.send_key(Key::Tab);
        app.render_frame();
        app.assert_text("/clear");

        app.send_key(Key::ArrowDown);
        app.render_frame();
        app.send_key(Key::Enter);
        app.render_frame();

        assert_eq!(app.state().command, "/model");
        app.assert_text("/model ");

        app.send_key(Key::Char('h'));
        app.render_frame();
        app.send_key(Key::Char('i'));
        app.render_frame();
        app.send_key(Key::Enter);
        app.render_frame();

        assert_eq!(app.state().draft, "");
        app.assert_text("You:");
        app.assert_text("echo: /model hi");
    }
}
