use thorn_core::prelude::*;

enum Action {}

fn counter(cx: &Scope) -> View<Action> {
    let count = cx.create_signal(0usize);
    count.set(1);
    col((panel(text(move || format!("count: {}", count.get()))).id("counter-panel"),))
        .padding(1)
        .gap(1)
        .bg(Token::Surface)
}

#[test]
fn mvp_counter_signal_drives_text_layout_render_and_diff() {
    let mut app = TestApp::new(counter).with_theme(Theme::light());

    app.render(40, 8);
    app.assert_text("count: 1");
    let panel_rect = app.layout_of("counter-panel");
    assert_eq!(panel_rect.h, 3);
    let panel_cell = app
        .screen()
        .expect("rendered screen")
        .get(panel_rect.x + 1, panel_rect.y + 1);
    assert_eq!(panel_cell.bg, Theme::light().resolve(Token::SurfaceAlt));
    app.assert_no_default_bg_on_text();

    assert_eq!(app.dirty_regions()[0].rect, Rect::new(0, 0, 40, 8));
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
        count.set(1);
        col((counter_panel(count.read_only()),))
    }

    let mut app = TestApp::new(parent);
    app.render(40, 8);
    app.assert_text("count: 1");
}
