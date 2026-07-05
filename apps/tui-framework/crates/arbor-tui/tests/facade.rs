use arbor_tui::prelude::*;
use arbor_tui_adapters::simulated_backend::SimulatedBackend;
use arbor_tui_adapters::simulated_input::SimulatedInput;

#[derive(Default)]
struct AppState {
    messages: Vec<String>,
    running: bool,
}

enum Action {
    Submit(String),
    Done,
    Quit,
}

fn update(state: &mut AppState, action: Action, ctx: &mut AppContext<Action>) {
    match action {
        Action::Submit(text) => {
            state.messages.push(text);
            state.running = true;
        }
        Action::Done => {
            state.running = false;
        }
        Action::Quit => {
            ctx.quit();
        }
    }
}

fn view(state: &AppState, ui: &Ui<Action>) -> Node<Action> {
    ui.page()
        .title("Arbor Agent Console")
        .body(
            ui.col()
                .fill()
                .child(ui.text(format!("Messages: {}", state.messages.len())))
                .child(ui.text(if state.running { "Running" } else { "Idle" }))
                .build(),
        )
        .footer(
            ui.prompt("ask agent")
                .loading(state.running)
                .loading_phase(1)
                .on_submit(Action::Submit)
                .build(),
        )
        .build()
}

#[test]
fn test_app_dispatches_action_and_renders_updated_view() {
    let mut app = TestApp::new(AppState::default(), update, view).theme(Theme::light());

    app.render(60, 8)
        .assert_text("Arbor Agent Console")
        .assert_text("Messages: 0")
        .assert_text("Idle")
        .assert_no_default_bg();

    app.dispatch(Action::Submit("fix layout".to_string()));

    app.render(60, 8)
        .assert_text("Messages: 1")
        .assert_text("Running")
        .assert_text("◐ ask agent")
        .assert_no_default_bg();

    app.dispatch(Action::Done);
    app.render(60, 8).assert_text("Idle");
}

#[test]
fn test_app_context_can_quit() {
    let mut app = TestApp::new(AppState::default(), update, view);

    app.dispatch(Action::Quit);

    assert!(!app.is_running());
}

#[test]
fn arbor_app_run_with_renders_first_frame() {
    let mut backend = SimulatedBackend::new(40, 5);
    let input = SimulatedInput::new();
    input.push(ctrl_char('c'));

    ArborApp::new(AppState::default())
        .update(update)
        .view(view)
        .run_with(&mut backend, &input)
        .expect("app should run");

    assert!(screen_contains(&backend, "Arbor Agent Console"));
}

fn ctrl_char(c: char) -> arbor_tui_domain::input::KeyEvent {
    arbor_tui_domain::input::KeyEvent {
        key: Key::Char(c),
        modifiers: Modifiers {
            ctrl: true,
            ..Default::default()
        },
        kind: arbor_tui_domain::input::KeyEventKind::Press,
    }
}

fn screen_contains(backend: &SimulatedBackend, needle: &str) -> bool {
    let chars = needle.chars().collect::<Vec<_>>();
    for row in 0..backend.screen().rows() {
        for col in 0..backend.screen().cols() {
            if col + chars.len() as u16 > backend.screen().cols() {
                continue;
            }
            if chars
                .iter()
                .enumerate()
                .all(|(offset, ch)| backend.screen().cell_at(col + offset as u16, row).ch == *ch)
            {
                return true;
            }
        }
    }
    false
}
