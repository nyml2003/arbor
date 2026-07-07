use std::collections::VecDeque;
use std::time::Duration;

pub use thorn_core::*;
pub use thorn_terminal as terminal;

use layout::Rect;
use reactive::Scope;
use render::{diff, render_tree, DirtyRegion, Screen};
use runtime::{KeyMap, RuntimeInput};
use terminal::TerminalBackend;
use theme::Theme;
use view::{handle_key, View};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    MissingRoot,
    MissingView,
    MissingUpdate,
    Terminal(terminal::TerminalError),
}

impl From<terminal::TerminalError> for Error {
    fn from(value: terminal::TerminalError) -> Self {
        Self::Terminal(value)
    }
}

type Root<Action> = Box<dyn FnOnce(&Scope) -> View<Action>>;
type BeforeEvents<State, Action> =
    Box<dyn FnMut(&mut State, &mut AppContext<Action>, &RuntimeContext, &mut Vec<RuntimeInput>)>;
type BeforeRender<State, Action> =
    Box<dyn FnMut(&mut State, &mut AppContext<Action>, &RuntimeContext)>;
type Update<State, Action> = Box<dyn FnMut(&mut State, Action, &mut AppContext<Action>)>;
type StatefulView<State, Action> = Box<dyn FnMut(&Scope, &State) -> View<Action>>;

pub fn app<Action>(root: impl FnOnce(&Scope) -> View<Action> + 'static) -> App<Action> {
    App {
        root: Some(Box::new(root)),
        theme: Theme::dark(),
    }
}

pub struct App<Action> {
    root: Option<Root<Action>>,
    theme: Theme,
}

pub struct ThornApp<State, Action = ()> {
    state: State,
    update: Option<Update<State, Action>>,
    view: Option<StatefulView<State, Action>>,
    keymap: KeyMap<Action>,
    before_events: BeforeEvents<State, Action>,
    before_render: BeforeRender<State, Action>,
    theme: Theme,
    poll_timeout: Duration,
}

pub struct AppContext<Action> {
    actions: Vec<Action>,
    next_theme: Option<Theme>,
    quit: bool,
    render_requested: bool,
}

impl<Action> AppContext<Action> {
    fn new() -> Self {
        Self {
            actions: Vec::new(),
            next_theme: None,
            quit: false,
            render_requested: false,
        }
    }

    pub fn dispatch(&mut self, action: Action) {
        self.actions.push(action);
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.next_theme = Some(theme);
        self.request_render();
    }

    pub fn quit(&mut self) {
        self.quit = true;
    }

    pub fn request_render(&mut self) {
        self.render_requested = true;
    }
}

#[derive(Copy, Clone, Debug)]
pub struct RuntimeContext {
    screen_size: (u16, u16),
}

impl RuntimeContext {
    pub fn screen_size(&self) -> (u16, u16) {
        self.screen_size
    }
}

impl<State> ThornApp<State, ()> {
    pub fn new(initial_state: State) -> Self {
        Self {
            state: initial_state,
            update: Some(Box::new(|_, (), _| {})),
            view: None,
            keymap: KeyMap::new(),
            before_events: Box::new(|_, _, _, _| {}),
            before_render: Box::new(|_, _, _| {}),
            theme: Theme::dark(),
            poll_timeout: Duration::from_millis(50),
        }
    }
}

impl<State, Action> ThornApp<State, Action> {
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn update<NextAction>(
        self,
        update: impl FnMut(&mut State, NextAction, &mut AppContext<NextAction>) + 'static,
    ) -> ThornApp<State, NextAction> {
        ThornApp {
            state: self.state,
            update: Some(Box::new(update)),
            view: None,
            keymap: KeyMap::new(),
            before_events: Box::new(|_, _, _, _| {}),
            before_render: Box::new(|_, _, _| {}),
            theme: self.theme,
            poll_timeout: self.poll_timeout,
        }
    }

    pub fn view(mut self, view: impl FnMut(&Scope, &State) -> View<Action> + 'static) -> Self {
        self.view = Some(Box::new(view));
        self
    }

    pub fn before_events(
        mut self,
        before_events: impl FnMut(&mut State, &mut AppContext<Action>, &RuntimeContext, &mut Vec<RuntimeInput>)
            + 'static,
    ) -> Self {
        self.before_events = Box::new(before_events);
        self
    }

    pub fn keymap(mut self, keymap: KeyMap<Action>) -> Self {
        self.keymap = keymap;
        self
    }

    pub fn before_render(
        mut self,
        before_render: impl FnMut(&mut State, &mut AppContext<Action>, &RuntimeContext) + 'static,
    ) -> Self {
        self.before_render = Box::new(before_render);
        self
    }

    pub fn poll_timeout(mut self, poll_timeout: Duration) -> Self {
        self.poll_timeout = poll_timeout;
        self
    }

    pub fn run(mut self) -> Result<()>
    where
        Action: Clone,
    {
        let mut view = self.view.take().ok_or(Error::MissingView)?;
        let mut backend = terminal::CrosstermBackend::new();
        let _guard = backend.enter()?;
        let input_reader = terminal::InputReader::spawn();
        let mut previous = None;

        loop {
            let runtime = RuntimeContext {
                screen_size: backend.size()?,
            };
            let mut ctx = AppContext::new();
            (self.before_render)(&mut self.state, &mut ctx, &runtime);
            if self.apply_context(ctx) {
                input_reader.shutdown();
                return Ok(());
            }

            let scope = Scope::new();
            let root = scope.enter(|| view(&scope, &self.state));
            render_frame(&mut backend, &root, &self.theme, &mut previous)?;

            let mut inputs = input_reader.poll_timeout(self.poll_timeout);
            if inputs.is_empty() {
                inputs.push(RuntimeInput::Tick);
            }
            if inputs
                .iter()
                .any(|input| matches!(input, RuntimeInput::Resize(_)))
            {
                previous = None;
            }
            let mut ctx = AppContext::new();
            (self.before_events)(&mut self.state, &mut ctx, &runtime, &mut inputs);
            let AppContext {
                mut actions,
                next_theme,
                quit,
                render_requested,
            } = ctx;
            if self.apply_context(AppContext {
                actions: Vec::new(),
                next_theme,
                quit,
                render_requested,
            }) {
                input_reader.shutdown();
                return Ok(());
            }

            for runtime_input in &inputs {
                match runtime_input {
                    RuntimeInput::Key(event) => {
                        let mut handled = false;
                        if let Some(action) = self.keymap.action_for(event) {
                            actions.push(action);
                            handled = true;
                        }
                        if let Some(action) = handle_key(root.node(), event) {
                            actions.push(action);
                            handled = true;
                        }
                        if !handled && runtime_input.is_default_exit() {
                            input_reader.shutdown();
                            return Ok(());
                        }
                    }
                    RuntimeInput::Resize(_) | RuntimeInput::Tick => {}
                }
            }

            if self.process_actions(actions)? {
                input_reader.shutdown();
                return Ok(());
            }
            scope.dispose();
        }
    }

    fn apply_context(&mut self, ctx: AppContext<Action>) -> bool {
        if let Some(theme) = ctx.next_theme {
            self.theme = theme;
        }
        ctx.quit
    }

    fn process_actions(&mut self, actions: Vec<Action>) -> Result<bool> {
        if actions.is_empty() {
            return Ok(false);
        }
        let update = self.update.as_mut().ok_or(Error::MissingUpdate)?;
        let mut pending = VecDeque::from(actions);
        while let Some(action) = pending.pop_front() {
            let mut ctx = AppContext::new();
            update(&mut self.state, action, &mut ctx);
            if let Some(theme) = ctx.next_theme {
                self.theme = theme;
            }
            if ctx.quit {
                return Ok(true);
            }
            pending.extend(ctx.actions);
        }
        Ok(false)
    }
}

impl<Action> App<Action> {
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn run(mut self) -> Result<()> {
        let root = self.root.take().ok_or(Error::MissingRoot)?;
        let scope = Scope::new();
        let root = scope.enter(|| root(&scope));
        let mut backend = terminal::CrosstermBackend::new();
        let _guard = backend.enter()?;
        let mut previous = None;

        render_frame(&mut backend, &root, &self.theme, &mut previous)?;

        loop {
            if let Some(input) = backend.read_input()? {
                if input.is_default_exit() {
                    break;
                }
                if matches!(input, RuntimeInput::Resize(_)) {
                    previous = None;
                    render_frame(&mut backend, &root, &self.theme, &mut previous)?;
                }
            }
        }

        Ok(())
    }
}

fn render_frame<Action>(
    backend: &mut impl TerminalBackend,
    root: &View<Action>,
    theme: &Theme,
    previous: &mut Option<Screen>,
) -> Result<()> {
    let (width, height) = backend.size()?;
    let (next_screen, _) = render_tree(root, width, height, theme);
    let dirty_regions = previous
        .as_ref()
        .map(|old| diff(old, &next_screen))
        .unwrap_or_else(|| full_screen_dirty(width, height));

    backend.emit(&dirty_regions, &next_screen)?;
    backend.flush()?;
    *previous = Some(next_screen);
    Ok(())
}

fn full_screen_dirty(width: u16, height: u16) -> Vec<DirtyRegion> {
    vec![DirtyRegion {
        rect: Rect::new(0, 0, width, height),
    }]
}

pub mod prelude {
    pub use crate::{AppContext, RuntimeContext, ThornApp};
    pub use thorn_core::prelude::*;
}

#[cfg(test)]
mod tests {
    use super::{full_screen_dirty, prelude::*};

    enum Action {}

    #[test]
    fn facade_exports_minimal_user_api() {
        fn root(_: &Scope) -> View<Action> {
            text("hello")
        }

        let mut app = TestApp::new(root);
        app.render(20, 4);
        app.assert_text("hello");
    }

    #[test]
    fn initial_frame_dirty_region_covers_full_screen() {
        assert_eq!(full_screen_dirty(4, 2)[0].rect, Rect::new(0, 0, 4, 2));
    }

    #[test]
    fn stateful_app_builder_accepts_action_runtime_shape() {
        #[derive(Clone)]
        enum CounterAction {
            Increment,
        }

        let _app = ThornApp::new(0i32)
            .update(|state, action, _ctx| match action {
                CounterAction::Increment => *state += 1,
            })
            .view(|_, state| text(format!("count: {state}")))
            .keymap(KeyMap::new().bind(Key::Char('+'), CounterAction::Increment));
    }

    #[test]
    fn runtime_processes_dispatched_actions_in_fifo_order() {
        enum Action {
            Parent,
            FirstChild,
            SecondChild,
        }

        let mut app =
            ThornApp::new(Vec::<&'static str>::new()).update(|state, action, ctx| match action {
                Action::Parent => {
                    state.push("parent");
                    ctx.dispatch(Action::FirstChild);
                    ctx.dispatch(Action::SecondChild);
                }
                Action::FirstChild => state.push("first"),
                Action::SecondChild => state.push("second"),
            });

        app.process_actions(vec![Action::Parent]).unwrap();

        assert_eq!(app.state, ["parent", "first", "second"]);
    }
}
