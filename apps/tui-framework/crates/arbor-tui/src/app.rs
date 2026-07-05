use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use anyhow::{bail, Result};
use arbor_tui_application::app::App;
use arbor_tui_domain::backend::TerminalBackend;
use arbor_tui_domain::input::{InputReader, KeyEvent};
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::WidgetNode;
use arbor_tui_runtime::{run_crossterm_terminal_app, TerminalApp};
use arbor_tui_widgets::widget_factory::WidgetFactory;

use crate::ui::{build_root, ActionSink};
use crate::{Node, Ui};

type UpdateFn<State, Action> = Box<dyn FnMut(&mut State, Action, &mut AppContext<Action>)>;
type ViewFn<State, Action> = Box<dyn FnMut(&State, &Ui<Action>) -> Node<Action>>;
type BeforeEventsFn<State, Action> = Box<
    dyn FnMut(&mut State, &mut AppContext<Action>, &mut App, &Theme, &mut Vec<KeyEvent>) -> bool,
>;
type BeforeRenderFn<State, Action> =
    Box<dyn FnMut(&mut State, &mut AppContext<Action>, &mut App, &Theme) -> bool>;

pub struct AppContext<Action> {
    actions: ActionSink<Action>,
    next_theme: Option<Theme>,
    quit: bool,
}

impl<Action> AppContext<Action> {
    pub(crate) fn new(actions: ActionSink<Action>) -> Self {
        Self {
            actions,
            next_theme: None,
            quit: false,
        }
    }

    pub fn dispatch(&mut self, action: Action) {
        self.actions.push(action);
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.next_theme = Some(theme);
    }

    pub fn quit(&mut self) {
        self.quit = true;
    }

    pub(crate) fn take_theme(&mut self) -> Option<Theme> {
        self.next_theme.take()
    }

    pub(crate) fn should_quit(&self) -> bool {
        self.quit
    }
}

pub struct ArborApp<State, Action> {
    state: State,
    theme: Theme,
    update: Option<UpdateFn<State, Action>>,
    view: Option<ViewFn<State, Action>>,
    before_events: Option<BeforeEventsFn<State, Action>>,
    before_render: Option<BeforeRenderFn<State, Action>>,
    poll_timeout: Duration,
}

impl<State, Action> ArborApp<State, Action>
where
    State: 'static,
    Action: 'static,
{
    pub fn new(state: State) -> Self {
        Self {
            state,
            theme: Theme::dark(),
            update: None,
            view: None,
            before_events: None,
            before_render: None,
            poll_timeout: Duration::from_millis(100),
        }
    }

    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn update(
        mut self,
        update: impl FnMut(&mut State, Action, &mut AppContext<Action>) + 'static,
    ) -> Self {
        self.update = Some(Box::new(update));
        self
    }

    pub fn view(mut self, view: impl FnMut(&State, &Ui<Action>) -> Node<Action> + 'static) -> Self {
        self.view = Some(Box::new(view));
        self
    }

    pub fn before_events(
        mut self,
        callback: impl FnMut(&mut State, &mut AppContext<Action>, &mut App, &Theme, &mut Vec<KeyEvent>) -> bool
            + 'static,
    ) -> Self {
        self.before_events = Some(Box::new(callback));
        self
    }

    pub fn before_render(
        mut self,
        callback: impl FnMut(&mut State, &mut AppContext<Action>, &mut App, &Theme) -> bool + 'static,
    ) -> Self {
        self.before_render = Some(Box::new(callback));
        self
    }

    pub fn poll_timeout(mut self, timeout: Duration) -> Self {
        self.poll_timeout = timeout;
        self
    }

    pub fn run(self) -> Result<()> {
        let app = self.into_terminal_app()?;
        run_crossterm_terminal_app(app)
    }

    pub fn run_with(
        self,
        backend: &mut dyn TerminalBackend,
        input: &dyn InputReader,
    ) -> Result<()> {
        let app = self.into_terminal_app()?;
        app.run(backend, input)
    }

    fn into_terminal_app(self) -> Result<TerminalApp> {
        let Some(update) = self.update else {
            bail!("ArborApp requires update(update_fn) before run");
        };
        let Some(view) = self.view else {
            bail!("ArborApp requires view(view_fn) before run");
        };

        let state = Rc::new(RefCell::new(self.state));
        let update = Rc::new(RefCell::new(update));
        let view = Rc::new(RefCell::new(view));
        let actions = ActionSink::new();

        let build_state = Rc::clone(&state);
        let build_view = Rc::clone(&view);
        let build_actions = actions.clone();
        let mut app = TerminalApp::with_builder(self.theme, move |cols, rows, theme| {
            let build_factory = Rc::new(WidgetFactory::new());
            build_app_root(
                &build_state,
                &build_view,
                &build_factory,
                build_actions.clone(),
                cols,
                rows,
                theme,
            )
        })
        .poll_timeout(self.poll_timeout);

        if let Some(mut before_events) = self.before_events {
            let event_state = Rc::clone(&state);
            let event_actions = actions.clone();
            app = app.before_events(move |runtime, theme, events| {
                let mut ctx = AppContext::new(event_actions.clone());
                let changed = before_events(
                    &mut event_state.borrow_mut(),
                    &mut ctx,
                    runtime,
                    theme,
                    events,
                );
                if ctx.should_quit() {
                    runtime.quit();
                }
                changed
            });
        }

        let update_state = Rc::clone(&state);
        let update_view = Rc::clone(&view);
        let update_actions = actions.clone();
        let frame_actions = actions.clone();
        let mut frame_hook = self.before_render;
        app = app.before_render(move |runtime, root, theme| {
            let mut processed =
                process_actions(&update_state, &update, &update_actions, runtime, theme);

            let mut hook_changed = false;
            if let Some(before_render) = frame_hook.as_mut() {
                let mut ctx = AppContext::new(frame_actions.clone());
                hook_changed =
                    before_render(&mut update_state.borrow_mut(), &mut ctx, runtime, theme);
                if let Some(next_theme) = ctx.take_theme() {
                    *theme = next_theme;
                    hook_changed = true;
                }
                if ctx.should_quit() {
                    runtime.quit();
                }
            }

            processed |= process_actions(&update_state, &update, &update_actions, runtime, theme);

            if !processed && !hook_changed {
                return false;
            }

            let (cols, rows) = runtime.screen_size();
            let update_factory = Rc::new(WidgetFactory::new());
            *root = build_app_root(
                &update_state,
                &update_view,
                &update_factory,
                update_actions.clone(),
                cols,
                rows,
                theme,
            );
            true
        });

        Ok(app)
    }
}

fn process_actions<State, Action>(
    state: &Rc<RefCell<State>>,
    update: &Rc<RefCell<UpdateFn<State, Action>>>,
    actions: &ActionSink<Action>,
    runtime: &mut App,
    theme: &mut Theme,
) -> bool {
    let mut processed = false;
    while let Some(action) = actions.pop() {
        processed = true;
        let mut ctx = AppContext::new(actions.clone());
        (update.borrow_mut())(&mut state.borrow_mut(), action, &mut ctx);
        if let Some(next_theme) = ctx.take_theme() {
            *theme = next_theme;
        }
        if ctx.should_quit() {
            runtime.quit();
        }
    }
    processed
}

fn build_app_root<State, Action>(
    state: &Rc<RefCell<State>>,
    view: &Rc<RefCell<ViewFn<State, Action>>>,
    factory: &Rc<WidgetFactory>,
    actions: ActionSink<Action>,
    cols: u16,
    rows: u16,
    theme: &Theme,
) -> WidgetNode
where
    Action: 'static,
{
    let ui = Ui::new(Rc::clone(factory), theme.clone(), actions);
    let node = (view.borrow_mut())(&state.borrow(), &ui);
    build_root(factory, theme, cols, rows, node)
}
