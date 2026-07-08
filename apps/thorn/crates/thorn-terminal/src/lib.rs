use std::io::{self, BufRead, Write};

use thorn_core::{IntentMapper, KeyMap, RuntimeInput, ThornApp};
use thorn_runtime::AppRuntime;

pub struct TerminalRuntime<App>
where
    App: ThornApp,
{
    runtime: AppRuntime<App>,
}

impl<App> TerminalRuntime<App>
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

    pub fn run(&mut self) -> io::Result<()> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        self.run_with_io(stdin.lock(), stdout.lock())
    }

    pub fn run_with_io(
        &mut self,
        mut input: impl BufRead,
        mut output: impl Write,
    ) -> io::Result<()> {
        while self.runtime.is_running() {
            self.draw(&mut output)?;
            let mut line = String::new();
            if input.read_line(&mut line)? == 0 {
                self.runtime.handle_input(RuntimeInput::Shutdown);
                break;
            }

            if let Some(ch) = line.chars().find(|ch| !ch.is_whitespace()) {
                self.send_key(ch);
            }
        }

        Ok(())
    }

    pub fn send_key(&mut self, ch: char) {
        self.runtime.send_key(ch);
    }

    pub fn handle_input(&mut self, input: RuntimeInput) {
        self.runtime.handle_input(input);
    }

    pub fn render_text(&mut self) -> String {
        self.runtime.render_frame().to_plain_text()
    }

    pub fn is_running(&self) -> bool {
        self.runtime.is_running()
    }

    fn draw(&mut self, output: &mut impl Write) -> io::Result<()> {
        let screen = self.runtime.render_frame();
        write!(output, "\x1b[2J\x1b[H{}", screen.to_plain_text())?;
        write!(output, "\n\npress +, -, q then Enter > ")?;
        output.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use thorn_core::{column, text, AppContext, Element, KeyAction, KeyEvent, KeyIntent};

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

    fn counter_runtime() -> TerminalRuntime<CounterApp> {
        TerminalRuntime::new(CounterApp { count: 0 }, CounterIntentMapper).size(40, 8)
    }

    #[test]
    fn render_text_shows_initial_counter() {
        let mut runtime = counter_runtime();

        assert!(runtime.render_text().contains("count: 0"));
    }

    #[test]
    fn plus_key_updates_render_text() {
        let mut runtime = counter_runtime();

        runtime.send_key('+');

        assert!(runtime.render_text().contains("count: 1"));
    }

    #[test]
    fn run_with_io_exits_on_q() {
        let mut runtime = counter_runtime();
        let mut output = Vec::new();

        runtime.run_with_io(&b"q\n"[..], &mut output).unwrap();

        assert!(!runtime.is_running());
        assert!(String::from_utf8(output).unwrap().contains("Counter"));
    }

    #[test]
    fn run_with_io_renders_after_increment_before_quit() {
        let mut runtime = counter_runtime();
        let mut output = Vec::new();

        runtime.run_with_io(&b"+\nq\n"[..], &mut output).unwrap();

        assert!(String::from_utf8(output).unwrap().contains("count: 1"));
    }

    #[test]
    fn custom_keymap_smoke_updates_terminal_output() {
        let mut runtime = TerminalRuntime::new(CounterApp { count: 0 }, CounterIntentMapper)
            .keymap(KeyMap::new().bind(KeyEvent::char('n'), KeyIntent::App("increment")))
            .size(40, 8);
        let mut output = Vec::new();

        runtime.run_with_io(&b"n\nq\n"[..], &mut output).unwrap();

        assert!(String::from_utf8(output).unwrap().contains("count: 1"));
    }
}
