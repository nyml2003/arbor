use thorn::prelude::*;

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
                text("press + / - to change, q or Esc to quit"),
            ))
            .padding(1)
            .gap(1)
            .bg(Token::Surface)
        })
        .before_events(|inputs, actions| {
            for input in inputs {
                match input {
                    RuntimeInput::Key(event) if event.key == Key::Char('+') => {
                        actions.push(CounterAction::Increment);
                    }
                    RuntimeInput::Key(event) if event.key == Key::Char('-') => {
                        actions.push(CounterAction::Decrement);
                    }
                    _ => {}
                }
            }
        })
        .run()
}
