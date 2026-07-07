use thorn_core::prelude::*;

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

#[test]
fn mvp_counter_signal_drives_text_layout_render_and_diff() {
    let mut app = TestApp::new(counter).with_theme(Theme::light());

    app.render(40, 8);
    app.assert_text("count: 0");
    let panel_rect = app.layout_of("counter-panel");
    assert_eq!(panel_rect.h, 3);
    let panel_cell = app
        .screen()
        .expect("rendered screen")
        .get(panel_rect.x + 1, panel_rect.y + 1);
    assert_eq!(panel_cell.bg, Theme::light().resolve(Token::SurfaceAlt));
    app.assert_no_default_bg_on_text();

    app.press_button("+1");
    app.render(40, 8);

    app.assert_text("count: 1");
    let changed_digit_x = panel_rect.x + 1 + "count: ".len() as u16;
    let changed_digit_y = panel_rect.y + 1;
    assert!(
        app.dirty_regions()
            .iter()
            .any(|region| region.rect.contains(changed_digit_x, changed_digit_y)),
        "text change should dirty the changed count digit"
    );
}

#[test]
fn application_function_component_can_own_signal() {
    fn counter_panel(cx: &Scope) -> View<Action> {
        let count = cx.create_signal(41usize);
        panel(text(move || format!("answer: {}", count.get())))
    }

    let mut app = TestApp::new(counter_panel);
    app.render(30, 4);

    app.assert_text("answer: 41");
}

#[test]
fn application_component_can_read_parent_signal() {
    fn counter_panel(count: ReadSignal<usize>) -> View<Action> {
        panel(text(move || format!("count: {}", count.get()))).id("counter-panel")
    }

    fn parent(cx: &Scope) -> View<Action> {
        let count = cx.create_signal(0usize);
        col((
            counter_panel(count.read_only()),
            button("+1").on_press(move |_| count.update(|value| *value += 1)),
        ))
    }

    let mut app = TestApp::new(parent);
    app.render(40, 8);
    app.assert_text("count: 0");

    app.press_button("+1");
    app.render(40, 8);
    app.assert_text("count: 1");
}
