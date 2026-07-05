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
    UseLightTheme,
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
        Action::UseLightTheme => {
            ctx.set_theme(Theme::light());
        }
    }
}

fn view(state: &AppState, ui: &Ui<Action>) -> Node<Action> {
    ui.component(
        Page::new()
            .title("Arbor Agent Console")
            .body(
                Col::new()
                    .fill()
                    .child(TextBlock::new(format!(
                        "Messages: {}",
                        state.messages.len()
                    )))
                    .child(TextBlock::new(if state.running {
                        "Running"
                    } else {
                        "Idle"
                    })),
            )
            .footer(
                PromptBar::new()
                    .placeholder("ask agent")
                    .loading(state.running)
                    .loading_phase(1)
                    .on_submit(Action::Submit),
            ),
    )
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

fn theme_view(_state: &AppState, ui: &Ui<Action>) -> Node<Action> {
    let label = match ui.theme().variant {
        ThemeVariant::Dark => "theme: dark",
        ThemeVariant::Light => "theme: light",
        ThemeVariant::HighContrast => "theme: high-contrast",
    };
    ui.component(TextBlock::new(label).bg(ui.theme().surface()))
}

#[test]
fn component_reads_current_theme_on_each_render() {
    let mut app = TestApp::new(AppState::default(), update, theme_view);

    app.render(40, 4).assert_text("theme: dark");
    app.dispatch(Action::UseLightTheme);
    app.render(40, 4)
        .assert_text("theme: light")
        .assert_no_default_bg();
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

#[derive(Default)]
struct HookState {
    hits: usize,
}

enum HookAction {
    Hit,
}

fn hook_update(state: &mut HookState, action: HookAction, _ctx: &mut AppContext<HookAction>) {
    match action {
        HookAction::Hit => state.hits += 1,
    }
}

fn hook_view(state: &HookState, ui: &Ui<HookAction>) -> Node<HookAction> {
    ui.component(
        Page::new()
            .title("Hook App")
            .body(TextBlock::new(format!("Hits: {}", state.hits))),
    )
}

struct HitCounter {
    hits: usize,
}

struct HitCounterProps {
    hits: usize,
}

impl HitCounter {
    fn new(hits: usize) -> Self {
        Self::from_props(HitCounterProps { hits })
    }
}

impl PropsComponent<HookAction> for HitCounter {
    type Props = HitCounterProps;

    fn from_props(props: Self::Props) -> Self {
        Self { hits: props.hits }
    }

    fn into_props(self) -> Self::Props {
        HitCounterProps { hits: self.hits }
    }
}

impl UiComponent<HookAction> for HitCounter {
    fn render(self, ui: &Ui<HookAction>) -> Node<HookAction> {
        let theme = ui.theme();
        ui.component(
            Panel::new(TextBlock::new(format!("Hits: {}", self.hits)))
                .fg(theme.border())
                .bg(theme.surface()),
        )
    }
}

#[test]
fn business_component_renders_through_component_entrypoint() {
    let mut app = TestApp::new(HookState { hits: 3 }, hook_update, |state, ui| {
        ui.component(HitCounter::new(state.hits))
    })
    .theme(Theme::light());

    app.render(40, 5)
        .assert_text("Hits: 3")
        .assert_no_default_bg();
}

fn assert_props_component<Action, Component>(_component: Component)
where
    Component: PropsComponent<Action>,
{
}

fn assert_component_props<Props>(_props: Props)
where
    Props: ComponentProps,
{
}

#[test]
fn built_in_components_expose_props_contract() {
    assert_props_component::<Action, _>(TextBlock::new("text"));
    assert_props_component::<Action, _>(StatusLine::new("status"));
    assert_props_component::<Action, _>(Input::new());
    assert_props_component::<Action, _>(PromptBar::new());
    assert_props_component::<Action, _>(FuzzyPanel::new(["/theme"]));
    assert_props_component::<Action, _>(Transcript::new());
    assert_props_component::<Action, _>(Col::new());
    assert_props_component::<Action, _>(Row::new());
    assert_props_component::<Action, _>(Panel::new(TextBlock::new("panel")));
    assert_props_component::<Action, _>(Page::new());

    assert_component_props(TextBlock::new("text").into_props());
    assert_component_props(Input::<Action>::new().into_props());
    assert_component_props(Panel::<Action>::new(TextBlock::new("panel")).into_props());
}

#[test]
fn arbor_app_before_events_can_consume_default_escape() {
    let mut backend = SimulatedBackend::new(40, 5);
    let input = SimulatedInput::new();
    input.push_batch([key(Key::Escape), ctrl_char('c')]);

    ArborApp::new(HookState::default())
        .update(hook_update)
        .view(hook_view)
        .before_events(|_state, ctx, _app, _theme, events| {
            let before = events.len();
            events.retain(|event| {
                if event.key == Key::Escape {
                    ctx.dispatch(HookAction::Hit);
                    false
                } else {
                    true
                }
            });
            events.len() != before
        })
        .run_with(&mut backend, &input)
        .expect("app should run");

    assert!(screen_contains(&backend, "Hits: 1"));
}

#[test]
fn arbor_app_before_render_can_update_first_frame() {
    let mut backend = SimulatedBackend::new(40, 5);
    let input = SimulatedInput::new();
    input.push(ctrl_char('c'));
    let mut once = false;

    ArborApp::new(HookState::default())
        .update(hook_update)
        .view(hook_view)
        .before_render(move |state, _ctx, _app, _theme| {
            if once {
                return false;
            }
            once = true;
            state.hits = 7;
            true
        })
        .run_with(&mut backend, &input)
        .expect("app should run");

    assert!(screen_contains(&backend, "Hits: 7"));
}

fn ctrl_char(c: char) -> KeyEvent {
    KeyEvent {
        key: Key::Char(c),
        modifiers: Modifiers {
            ctrl: true,
            ..Default::default()
        },
        kind: KeyEventKind::Press,
    }
}

fn key(key: Key) -> KeyEvent {
    KeyEvent {
        key,
        modifiers: Modifiers::default(),
        kind: KeyEventKind::Press,
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
