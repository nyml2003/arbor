use thorn_core::{IntentMapper, KeyIntent, KeyMap, RuntimeInput, Screen, ThornApp};
use thorn_runtime::AppRuntime;

pub struct TestRuntime<App>
where
    App: ThornApp,
{
    runtime: AppRuntime<App>,
}

impl<App> TestRuntime<App>
where
    App: ThornApp,
{
    pub fn new(app: App, mapper: impl IntentMapper<App::Action> + 'static) -> Self {
        Self {
            runtime: AppRuntime::new(app, mapper),
        }
    }

    pub fn size(mut self, width: u16, height: u16) -> Self {
        self.runtime = self.runtime.size(width, height);
        self
    }

    pub fn keymap(mut self, keymap: KeyMap) -> Self {
        self.runtime = self.runtime.keymap(keymap);
        self
    }

    pub fn render_frame(&mut self) {
        self.runtime.render_frame();
    }

    pub fn send_key(&mut self, ch: char) {
        self.runtime.send_key(ch);
    }

    pub fn send_ctrl_key(&mut self, ch: char) {
        self.runtime.send_ctrl_key(ch);
    }

    pub fn handle_input(&mut self, input: RuntimeInput) {
        self.runtime.handle_input(input);
    }

    pub fn dispatch_intent(&mut self, intent: KeyIntent) {
        self.runtime.dispatch_intent(intent);
    }

    pub fn is_running(&self) -> bool {
        self.runtime.is_running()
    }

    pub fn screen(&self) -> &Screen {
        self.runtime.screen()
    }

    pub fn snapshot(&self) -> ScreenSnapshot {
        ScreenSnapshot {
            text: self.screen().to_plain_text(),
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
    use thorn_core::{column, row, text, AppContext, Element, KeyAction, KeyEvent};

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

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum AgentAction {
        Prompt,
        ModelChunk,
        ToolResult,
        Finish,
    }

    struct AgentHarnessApp {
        prompt_count: u32,
        transcript: Vec<&'static str>,
        status: &'static str,
    }

    impl ThornApp for AgentHarnessApp {
        type Action = AgentAction;

        fn update(&mut self, action: Self::Action, ctx: &mut AppContext<Self::Action>) {
            match action {
                AgentAction::Prompt => {
                    self.prompt_count += 1;
                    self.status = "thinking";
                    self.transcript.push("user: build a plan");
                    ctx.dispatch(AgentAction::ModelChunk);
                }
                AgentAction::ModelChunk => {
                    self.transcript.push("assistant: draft plan");
                    ctx.dispatch(AgentAction::ToolResult);
                }
                AgentAction::ToolResult => {
                    self.status = "tool-ready";
                    self.transcript.push("tool: cargo check passed");
                }
                AgentAction::Finish => {
                    self.status = "done";
                    ctx.quit();
                }
            }
        }

        fn view(&self) -> Element<Self::Action> {
            column((
                row((text("Aster Agent"), text(format!(" [{}]", self.status)))),
                text(format!("prompts: {}", self.prompt_count)),
                text(self.transcript.last().copied().unwrap_or("idle")),
            ))
        }
    }

    struct AgentIntentMapper;

    impl IntentMapper<AgentAction> for AgentIntentMapper {
        fn map_intent(&self, intent: KeyIntent) -> Option<KeyAction<AgentAction>> {
            match intent {
                KeyIntent::RequestQuit => Some(KeyAction::App(AgentAction::Finish)),
                KeyIntent::App("prompt") => Some(KeyAction::App(AgentAction::Prompt)),
                KeyIntent::App(_) => None,
            }
        }
    }

    #[test]
    fn headless_can_drive_agent_like_state_loop() {
        let mut runtime = TestRuntime::new(
            AgentHarnessApp {
                prompt_count: 0,
                transcript: Vec::new(),
                status: "idle",
            },
            AgentIntentMapper,
        )
        .keymap(KeyMap::new().bind(KeyEvent::char('p'), KeyIntent::App("prompt")))
        .size(64, 8);

        runtime.render_frame();
        runtime.assert_text("idle");

        runtime.send_key('p');
        runtime.render_frame();

        runtime.assert_text("tool-ready");
        runtime.assert_text("prompts: 1");
        runtime.assert_text("tool: cargo check passed");

        runtime.dispatch_intent(KeyIntent::RequestQuit);
        assert!(!runtime.is_running());
    }
}
