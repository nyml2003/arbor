use thorn_core::{
    render_to_screen, AppContext, IntentMapper, KeyAction, KeyEvent, KeyIntent, KeyMap,
    RuntimeInput, Screen, Size, ThornApp,
};

pub struct TestRuntime<App>
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
}

impl<App> TestRuntime<App>
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
        }
    }

    pub fn size(mut self, width: u16, height: u16) -> Self {
        self.size = Size::new(width, height);
        self.screen = Screen::new(self.size);
        self
    }

    pub fn render_frame(&mut self) {
        if !self.running {
            return;
        }
        let element = self.app.view();
        self.screen = render_to_screen(&element, self.size);
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
            RuntimeInput::Shutdown => self.running = false,
            RuntimeInput::Resize(size) => {
                self.size = size;
                self.ctx.request_render();
            }
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
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    pub fn snapshot(&self) -> ScreenSnapshot {
        ScreenSnapshot {
            text: self.screen.to_plain_text(),
        }
    }

    pub fn assert_text(&self, expected: &str) -> &Self {
        self.snapshot().assert_text(expected);
        self
    }

    pub fn assert_not_text(&self, unexpected: &str) -> &Self {
        self.snapshot().assert_not_text(unexpected);
        self
    }

    pub fn assert_line(&self, line_index: usize, expected: &str) -> &Self {
        self.snapshot().assert_line(line_index, expected);
        self
    }

    fn drain_actions(&mut self) {
        while let Some(action) = self.ctx.pop_action() {
            self.app.update(action, &mut self.ctx);
        }
        if self.ctx.is_quit_requested() {
            self.running = false;
        }
    }
}

pub struct ScreenSnapshot {
    text: String,
}

impl ScreenSnapshot {
    pub fn to_plain_text(&self) -> &str {
        &self.text
    }

    pub fn assert_text(&self, expected: &str) -> &Self {
        assert!(
            self.text.contains(expected),
            "expected screen to contain {expected:?}, got:\n{}",
            self.text
        );
        self
    }

    pub fn assert_not_text(&self, unexpected: &str) -> &Self {
        assert!(
            !self.text.contains(unexpected),
            "expected screen to not contain {unexpected:?}, got:\n{}",
            self.text
        );
        self
    }

    pub fn assert_line(&self, line_index: usize, expected: &str) -> &Self {
        let actual = self.text.lines().nth(line_index).unwrap_or("");
        assert_eq!(actual, expected);
        self
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
    }

    struct CounterApp {
        count: i32,
    }

    impl ThornApp for CounterApp {
        type Action = CounterAction;

        fn update(&mut self, action: Self::Action, _ctx: &mut AppContext<Self::Action>) {
            match action {
                CounterAction::Increment => self.count += 1,
                CounterAction::Decrement => self.count -= 1,
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
                KeyIntent::App(_) => None,
            }
        }
    }

    fn counter_runtime() -> TestRuntime<CounterApp> {
        TestRuntime::new(CounterApp { count: 0 }, CounterIntentMapper).size(40, 8)
    }

    #[test]
    fn counter_initial_render_shows_zero() {
        let mut runtime = counter_runtime();

        runtime.render_frame();

        runtime.assert_text("count: 0");
    }

    #[test]
    fn plus_key_increments_counter() {
        let mut runtime = counter_runtime();

        runtime.send_key('+');
        runtime.render_frame();

        runtime.assert_text("count: 1");
    }

    #[test]
    fn minus_key_decrements_counter() {
        let mut runtime = counter_runtime();

        runtime.send_key('-');
        runtime.render_frame();

        runtime.assert_text("count: -1");
    }

    #[test]
    fn q_key_requests_quit() {
        let mut runtime = counter_runtime();

        runtime.send_key('q');

        assert!(!runtime.is_running());
    }

    #[test]
    fn ctrl_c_requests_quit() {
        let mut runtime = counter_runtime();

        runtime.send_ctrl_key('c');

        assert!(!runtime.is_running());
    }

    #[test]
    fn runtime_does_not_render_after_quit() {
        let mut runtime = counter_runtime();
        runtime.render_frame();
        runtime.send_key('q');
        runtime.send_key('+');
        runtime.render_frame();

        runtime.assert_text("count: 0");
        assert!(!runtime.is_running());
    }

    #[test]
    fn intent_resolver_maps_increment_to_app_action() {
        let mapper = CounterIntentMapper;

        assert_eq!(
            mapper.map_intent(KeyIntent::App("increment")),
            Some(KeyAction::App(CounterAction::Increment))
        );
    }

    #[test]
    fn intent_resolver_maps_quit_to_runtime_quit() {
        let mapper = CounterIntentMapper;

        assert_eq!(
            mapper.map_intent(KeyIntent::RequestQuit),
            Some(KeyAction::RuntimeQuit)
        );
    }

    #[test]
    fn headless_assert_text_passes_when_text_exists() {
        let mut runtime = counter_runtime();
        runtime.render_frame();

        runtime.assert_text("Counter");
    }

    #[test]
    fn headless_assert_not_text_passes_after_update() {
        let mut runtime = counter_runtime();
        runtime.render_frame();
        runtime.send_key('+');
        runtime.render_frame();

        runtime.assert_not_text("count: 0");
    }
}
