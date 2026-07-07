use thorn::prelude::*;

#[derive(Clone)]
enum CounterAction {
    Increment,
    Decrement,
}

fn main() -> thorn::Result<()> {
    ThornApp::new(0i32)
        .theme(Theme::light())
        .update(|count, action| match action {
            CounterAction::Increment => *count += 1,
            CounterAction::Decrement => *count -= 1,
        })
        .view(|_, count| {
            col((
                panel(text(format!("count: {count}"))).id("counter-panel"),
                text("press + / - to change, Ctrl-Q or Esc to quit"),
            ))
            .padding(1)
            .gap(1)
            .bg(Token::Surface)
        })
        .keymap(
            KeyMap::new()
                .bind(Key::Char('+'), CounterAction::Increment)
                .bind(Key::Char('-'), CounterAction::Decrement),
        )
        .run()
}
