use thorn::prelude::*;

enum Action {}

fn counter(cx: &Scope) -> View<Action> {
    let count = cx.create_signal(0usize);

    col((
        panel(text({
            let count = count.clone();
            move || format!("count: {}", count.get())
        }))
        .id("counter-panel"),
        button("+1").on_press(move |_| count.update(|value| *value += 1)),
    ))
    .padding(1)
    .gap(1)
    .bg(Token::Surface)
}

fn main() -> thorn::Result<()> {
    thorn::app(counter).theme(Theme::light()).run()
}
