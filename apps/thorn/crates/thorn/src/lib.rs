pub use thorn_core::*;
pub use thorn_terminal as terminal;

use layout::Rect;
use reactive::Scope;
use render::{diff, render_tree, DirtyRegion, Screen};
use runtime::RuntimeInput;
use terminal::TerminalBackend;
use theme::Theme;
use view::View;

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
type BeforeEvents<Action> = Box<dyn FnMut(&[RuntimeInput], &mut Vec<Action>)>;
type BeforeRender<State> = Box<dyn FnMut(&mut State)>;
type Update<State, Action> = Box<dyn FnMut(&mut State, Action)>;
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
    before_events: BeforeEvents<Action>,
    before_render: BeforeRender<State>,
    theme: Theme,
}

impl<State> ThornApp<State, ()> {
    pub fn new(initial_state: State) -> Self {
        Self {
            state: initial_state,
            update: Some(Box::new(|_, ()| {})),
            view: None,
            before_events: Box::new(|_, _| {}),
            before_render: Box::new(|_| {}),
            theme: Theme::dark(),
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
        update: impl FnMut(&mut State, NextAction) + 'static,
    ) -> ThornApp<State, NextAction> {
        ThornApp {
            state: self.state,
            update: Some(Box::new(update)),
            view: None,
            before_events: Box::new(|_, _| {}),
            before_render: self.before_render,
            theme: self.theme,
        }
    }

    pub fn view(mut self, view: impl FnMut(&Scope, &State) -> View<Action> + 'static) -> Self {
        self.view = Some(Box::new(view));
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

    pub fn run(mut self) -> Result<()> {
        let mut view = self.view.take().ok_or(Error::MissingView)?;
        let mut backend = terminal::CrosstermBackend::new();
        let _guard = backend.enter()?;
        let mut previous = None;

        loop {
            (self.before_render)(&mut self.state);
            render_stateful_frame(
                &mut backend,
                &mut view,
                &self.state,
                &self.theme,
                &mut previous,
            )?;

            let Some(input) = backend.read_input()? else {
                continue;
            };
            if input.is_default_exit() {
                break;
            }
            if matches!(input, RuntimeInput::Resize(_)) {
                previous = None;
            }

            let inputs = [input];
            let mut actions = Vec::new();
            (self.before_events)(&inputs, &mut actions);
            if !actions.is_empty() {
                let update = self.update.as_mut().ok_or(Error::MissingUpdate)?;
                for action in actions {
                    update(&mut self.state, action);
                }
            }
        }

        Ok(())
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

fn render_stateful_frame<State, Action>(
    backend: &mut impl TerminalBackend,
    view: &mut StatefulView<State, Action>,
    state: &State,
    theme: &Theme,
    previous: &mut Option<Screen>,
) -> Result<()> {
    let scope = Scope::new();
    let root = scope.enter(|| view(&scope, state));
    render_frame(backend, &root, theme, previous)?;
    scope.dispose();
    Ok(())
}

fn full_screen_dirty(width: u16, height: u16) -> Vec<DirtyRegion> {
    vec![DirtyRegion {
        rect: Rect::new(0, 0, width, height),
    }]
}

pub mod prelude {
    pub use crate::ThornApp;
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
        enum CounterAction {
            Increment,
        }

        let _app = ThornApp::new(0i32)
            .update(|state, action| match action {
                CounterAction::Increment => *state += 1,
            })
            .view(|_, state| text(format!("count: {state}")))
            .before_events(|inputs, actions| {
                if inputs
                    .iter()
                    .any(|input| matches!(input, RuntimeInput::Key(event) if event.key == Key::Char('+')))
                {
                    actions.push(CounterAction::Increment);
                }
            });
    }
}
