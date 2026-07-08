use thorn::prelude::*;
use thorn_terminal::TerminalRuntime;

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
        column(vec![
            text("Counter"),
            text(format!("count: {}", self.count)),
            text("+/- change, q quit"),
            text("press a key, then Enter"),
        ])
    }
}

struct CounterIntentMapper;

impl IntentMapper<CounterAction> for CounterIntentMapper {
    fn map_intent(&self, intent: KeyIntent) -> Option<KeyAction<CounterAction>> {
        match intent {
            KeyIntent::RequestQuit => Some(KeyAction::RuntimeQuit),
            KeyIntent::App("increment") => Some(KeyAction::App(CounterAction::Increment)),
            KeyIntent::App("decrement") => Some(KeyAction::App(CounterAction::Decrement)),
            _ => None,
        }
    }
}

fn main() -> std::io::Result<()> {
    TerminalRuntime::new(CounterApp { count: 0 }, CounterIntentMapper)
        .size(40, 8)
        .run()
}
