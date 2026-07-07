use thorn::prelude::*;

enum Action {}

fn counter(cx: &Scope) -> View<Action> {
    let count = cx.create_signal(0usize);
    count.set(1);

    col((panel(text(move || format!("count: {}", count.get()))).id("counter-panel"),))
        .padding(1)
        .gap(1)
        .bg(Token::Surface)
}

fn main() -> thorn::Result<()> {
    thorn::app(counter).theme(Theme::light()).run()
}
