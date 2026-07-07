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

fn main() {
    let mut app = TestApp::new(counter).with_theme(Theme::light());

    app.render(32, 6);
    println!("== initial ==");
    println!("{}", app.screen().expect("rendered screen").to_plain_text());

    app.press_button("+1");
    app.render(32, 6);
    println!();
    println!("== after +1 ==");
    println!("{}", app.screen().expect("rendered screen").to_plain_text());
    println!();
    println!("dirty regions: {:?}", app.dirty_regions());
}
