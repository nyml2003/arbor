use arbor_tui_domain::input::{Key, KeyEvent, KeyEventKind, Modifiers};
use arbor_tui_domain::theme::Theme;
use arbor_tui_testing::TuiTestDriver;
use arbor_tui_widgets::border::Border;
use arbor_tui_widgets::input::Input;
use arbor_tui_widgets::stack::{Col, Row};
use arbor_tui_widgets::text::Text;
use arbor_tui_widgets::widget_factory::WidgetFactory;

#[test]
fn key_script_updates_input_and_terminal_screen() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Input::new().placeholder("type").build(&factory, &theme);
    let mut driver = TuiTestDriver::new(root, 30, 1, theme);

    driver.render_initial().unwrap();
    assert!(!driver.find_text("type").is_empty());

    driver.focus_next().unwrap();
    driver.send_chars("abc").unwrap();

    assert!(!driver.find_text("abc").is_empty());
    assert!(driver.find_text("type").is_empty());
    assert!(driver.last_step().should_render);
    assert!(driver.last_render().is_some());
}

#[test]
fn idle_tick_after_initial_render_does_not_emit_output() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Text::new("stable").build(&factory, &theme);
    let mut driver = TuiTestDriver::new(root, 30, 1, theme);

    driver.render_initial().unwrap();
    assert!(driver.output_contains("CSI"));
    let output_len = driver.output_len();

    let step = driver.tick([]).unwrap();

    assert!(!step.should_render);
    assert_eq!(driver.last_render(), None);
    assert_eq!(driver.output_len(), output_len);
}

#[test]
fn repeated_characters_are_not_merged_by_runtime() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Input::new().placeholder("type").build(&factory, &theme);
    let mut driver = TuiTestDriver::new(root, 30, 1, theme);

    driver.render_initial().unwrap();
    driver.focus_next().unwrap();
    driver.send_chars("bookkeeper").unwrap();

    assert!(!driver.find_text("bookkeeper").is_empty());
}

#[test]
fn tab_focus_marks_runtime_for_render() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Row::new()
        .children([
            Input::new().placeholder("left").build(&factory, &theme),
            Input::new().placeholder("right").build(&factory, &theme),
        ])
        .build(&factory, &theme);
    let mut driver = TuiTestDriver::new(root, 40, 1, theme);

    driver.render_initial().unwrap();
    assert_eq!(driver.focused_widget(), None);

    driver.focus_next().unwrap();
    let first = driver.focused_widget();
    driver.focus_next().unwrap();
    let second = driver.focused_widget();

    assert_ne!(first, second);
    assert!(driver.last_step().should_render);
}

#[test]
fn light_theme_e2e_has_no_default_black_on_visible_text() {
    let theme = Theme::light();
    let factory = WidgetFactory::new();
    let root = Col::new()
        .children([
            Border::new()
                .title(" Header ")
                .child(Text::new("visible").build(&factory, &theme))
                .build(&factory, &theme),
            Input::new().placeholder("command").build(&factory, &theme),
        ])
        .build(&factory, &theme);
    let mut driver = TuiTestDriver::new(root, 50, 6, theme);

    driver.render_initial().unwrap();

    driver
        .assert_no_default_black_on_visible_text()
        .expect("visible text should not leak Cell::default black backgrounds");
}

#[test]
fn escape_quits_through_runtime_step() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Text::new("bye").build(&factory, &theme);
    let mut driver = TuiTestDriver::new(root, 20, 1, theme);

    driver.render_initial().unwrap();
    let step = driver
        .tick([KeyEvent {
            key: Key::Escape,
            modifiers: Modifiers::default(),
            kind: KeyEventKind::Press,
        }])
        .unwrap();

    assert!(step.should_quit);
    assert!(!driver.is_running());
}

#[test]
fn mocked_resize_updates_terminal_screen_size() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Text::new("resized").build(&factory, &theme);
    let mut driver = TuiTestDriver::new(root, 20, 1, theme);

    driver.render_initial().unwrap();
    driver.resize(40, 3).unwrap();

    assert!(driver.last_step().resized);
    assert!(driver.last_step().should_clear);
    assert_eq!(driver.screen().cols(), 40);
    assert_eq!(driver.screen().rows(), 3);
    assert!(!driver.find_text("resized").is_empty());
}
