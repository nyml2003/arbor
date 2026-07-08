use thorn_core::{
    render_to_screen, AppContext, IntentMapper, KeyAction, KeyEvent, KeyIntent, KeyMap,
    RuntimeInput, Screen, ScreenPatch, Size, ThornApp,
};

pub struct AppRuntime<App>
where
    App: ThornApp,
{
    app: App,
    ctx: AppContext<App::Action>,
    keymap: KeyMap,
    mapper: Box<dyn IntentMapper<App::Action>>,
    size: Size,
    screen: Screen,
    running: bool,
    render_requested: bool,
}

impl<App> AppRuntime<App>
where
    App: ThornApp,
{
    pub fn new(app: App, mapper: impl IntentMapper<App::Action> + 'static) -> Self {
        let size = Size::new(80, 24);
        Self {
            app,
            ctx: AppContext::new(),
            keymap: KeyMap::default(),
            mapper: Box::new(mapper),
            size,
            screen: Screen::new(size),
            running: true,
            render_requested: true,
        }
    }

    pub fn size(mut self, width: u16, height: u16) -> Self {
        self.resize(Size::new(width, height));
        self
    }

    pub fn resize(&mut self, size: Size) {
        self.size = size;
        self.screen = Screen::new(size);
        self.request_render();
    }

    pub fn request_render(&mut self) {
        self.render_requested = true;
    }

    pub fn is_render_requested(&self) -> bool {
        self.render_requested
    }

    pub fn render_frame(&mut self) -> &Screen {
        if self.running {
            let element = self.app.view();
            self.screen = render_to_screen(&element, self.size);
            self.render_requested = false;
        }
        &self.screen
    }

    pub fn render_patch(&mut self) -> ScreenPatch {
        let previous = self.screen.clone();
        let next = self.render_frame();
        previous.diff(next)
    }

    pub fn render_if_requested(&mut self) -> Option<&Screen> {
        self.render_requested.then(|| self.render_frame())
    }

    pub fn send_key(&mut self, ch: char) {
        self.handle_input(RuntimeInput::Key(KeyEvent::char(ch)));
    }

    pub fn send_ctrl_key(&mut self, ch: char) {
        self.handle_input(RuntimeInput::Key(KeyEvent::ctrl(ch)));
    }

    pub fn handle_input(&mut self, input: RuntimeInput) {
        if !self.running {
            return;
        }

        match input {
            RuntimeInput::Key(event) => {
                if let Some(intent) = self.keymap.resolve(&event) {
                    self.dispatch_intent(intent);
                }
            }
            RuntimeInput::Resize(size) => self.resize(size),
            RuntimeInput::Shutdown => self.running = false,
            RuntimeInput::Tick => {}
        }
        self.drain_actions();
    }

    pub fn dispatch_intent(&mut self, intent: KeyIntent) {
        match self.mapper.map_intent(intent) {
            Some(KeyAction::RuntimeQuit) => {
                self.ctx.quit();
                self.running = false;
            }
            Some(KeyAction::App(action)) => self.ctx.dispatch(action),
            None => {}
        }
        self.drain_actions();
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    fn drain_actions(&mut self) {
        let mut updated = false;
        while let Some(action) = self.ctx.pop_action() {
            self.app.update(action, &mut self.ctx);
            updated = true;
        }
        if updated || self.ctx.take_render_requested() {
            self.render_requested = true;
        }
        if self.ctx.is_quit_requested() {
            self.running = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use thorn_core::{column, text, Element};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum CounterAction {
        Increment,
        Decrement,
        RequestRender,
    }

    struct CounterApp {
        count: i32,
    }

    impl ThornApp for CounterApp {
        type Action = CounterAction;

        fn update(&mut self, action: Self::Action, ctx: &mut AppContext<Self::Action>) {
            match action {
                CounterAction::Increment => self.count += 1,
                CounterAction::Decrement => self.count -= 1,
                CounterAction::RequestRender => ctx.request_render(),
            }
        }

        fn view(&self) -> Element<Self::Action> {
            column((
                text("Counter"),
                text(format!("count: {}", self.count)),
                text("+/- change, q quit"),
            ))
        }
    }

    struct CounterIntentMapper;

    impl IntentMapper<CounterAction> for CounterIntentMapper {
        fn map_intent(&self, intent: KeyIntent) -> Option<KeyAction<CounterAction>> {
            match intent {
                KeyIntent::RequestQuit => Some(KeyAction::RuntimeQuit),
                KeyIntent::App("increment") => Some(KeyAction::App(CounterAction::Increment)),
                KeyIntent::App("decrement") => Some(KeyAction::App(CounterAction::Decrement)),
                KeyIntent::App("request_render") => {
                    Some(KeyAction::App(CounterAction::RequestRender))
                }
                KeyIntent::App(_) => None,
            }
        }
    }

    fn counter_runtime() -> AppRuntime<CounterApp> {
        AppRuntime::new(CounterApp { count: 0 }, CounterIntentMapper).size(40, 8)
    }

    #[test]
    fn initial_runtime_requests_render() {
        let runtime = counter_runtime();

        assert!(runtime.is_render_requested());
    }

    #[test]
    fn render_frame_updates_screen_and_clears_request() {
        let mut runtime = counter_runtime();

        runtime.render_frame();

        assert!(runtime.screen().to_plain_text().contains("count: 0"));
        assert!(!runtime.is_render_requested());
    }

    #[test]
    fn app_action_updates_state_and_requests_render() {
        let mut runtime = counter_runtime();
        runtime.render_frame();

        runtime.send_key('+');

        assert!(runtime.is_render_requested());
        assert!(runtime.render_frame().to_plain_text().contains("count: 1"));
    }

    #[test]
    fn render_patch_reports_changed_cells_between_frames() {
        let mut runtime = counter_runtime();
        runtime.render_frame();
        runtime.send_key('+');

        let patch = runtime.render_patch();

        assert!(!patch.full);
        assert_eq!(patch.cells.len(), 1);
        assert_eq!(patch.cells[0].cell.ch, '1');
    }

    #[test]
    fn resize_replaces_screen_and_requests_render() {
        let mut runtime = counter_runtime();
        runtime.render_frame();

        runtime.handle_input(RuntimeInput::Resize(Size::new(20, 4)));

        assert!(runtime.is_render_requested());
        assert_eq!(runtime.screen().size, Size::new(20, 4));
    }

    #[test]
    fn quit_intent_stops_runtime() {
        let mut runtime = counter_runtime();

        runtime.send_key('q');

        assert!(!runtime.is_running());
    }
}
