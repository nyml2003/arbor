use std::collections::HashMap;

use crate::layout::{LayoutInfo, Rect, Size};
use crate::reactive::Scope;
use crate::render::{diff, render_tree, DirtyRegion, Screen};
use crate::runtime::{Key, KeyEvent, RuntimeInput};
use crate::theme::{Color, Theme};
use crate::view::{NodeId, PrimitiveNode, View};

pub struct TestApp<Action = ()> {
    scope: Scope,
    root: View<Action>,
    theme: Theme,
    screen: Option<Screen>,
    layout: HashMap<NodeId, LayoutInfo>,
    dirty_regions: Vec<DirtyRegion>,
}

type BeforeEvents<Action> = Box<dyn FnMut(&[RuntimeInput], &mut Vec<Action>)>;
type BeforeRender<State> = Box<dyn FnMut(&mut State)>;
type Update<State, Action> = Box<dyn FnMut(&mut State, Action)>;
type RuntimeView<State, Action> = Box<dyn FnMut(&Scope, &State) -> View<Action>>;

pub struct TestRuntime<State, Action = ()> {
    state: State,
    update: Update<State, Action>,
    view: RuntimeView<State, Action>,
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

impl<State, Action> TestRuntime<State, Action> {
    pub fn new(
        initial_state: State,
        update: impl FnMut(&mut State, Action) + 'static,
        view: impl FnMut(&Scope, &State) -> View<Action> + 'static,
    ) -> Self {
        Self {
            state: initial_state,
            update: Box::new(update),
            view: Box::new(view),
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
        before_events: impl FnMut(&[RuntimeInput], &mut Vec<Action>) + 'static,
    ) -> Self {
        self.before_events = Box::new(before_events);
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
        let inputs = std::mem::take(&mut self.pending_inputs);
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

        if inputs.iter().copied().any(RuntimeInput::is_default_exit) {
            self.should_exit = true;
            return;
        }

        let mut actions = Vec::new();
        (self.before_events)(&inputs, &mut actions);
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
        .before_events(|inputs, actions| {
            if inputs.iter().any(
                |input| matches!(input, RuntimeInput::Key(event) if event.key == Key::Char('+')),
            ) {
                actions.push(CounterAction::Increment);
            }
        });

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
}
