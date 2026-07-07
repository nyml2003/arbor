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

fn main() {
    let mut app = TestApp::new(counter).with_theme(Theme::light());

    app.render(32, 6);
    println!("== render ==");
    println!("{}", app.screen().expect("rendered screen").to_plain_text());
    println!();
    println!("dirty regions: {:?}", app.dirty_regions());
}
