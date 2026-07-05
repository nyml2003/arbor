mod common;

use arbor_tui_domain::input::Key;
use arbor_tui_domain::theme::Theme;
use arbor_tui_widgets::input::Input;
use arbor_tui_widgets::stack::Col;
use arbor_tui_widgets::tabs::{TabDef, Tabs};
use arbor_tui_widgets::text::Text;
use arbor_tui_widgets::widget_factory::WidgetFactory;

use common::{assert_has_text, assert_not_text, mounted};

#[test]
fn resize_larger_updates_screen_and_renders() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Text::new("resized").build(&factory, &theme);
    let mut driver = mounted(root, 10, 1, theme);

    driver.resize(30, 3).unwrap();

    assert!(driver.last_step().resized);
    assert_eq!(driver.screen().cols(), 30);
    assert_eq!(driver.screen().rows(), 3);
    assert_has_text(&driver, "resized");
}

#[test]
fn resize_smaller_clips_old_content() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Text::new("very-long-content").build(&factory, &theme);
    let mut driver = mounted(root, 30, 1, theme);

    driver.resize(4, 1).unwrap();

    assert_eq!(driver.screen().cols(), 4);
    assert_not_text(&driver, "very-long-content");
}

#[test]
fn resize_smaller_then_larger_restores_content() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Text::new("restore me").build(&factory, &theme);
    let mut driver = mounted(root, 20, 1, theme);

    driver.resize(4, 1).unwrap();
    driver.resize(20, 1).unwrap();

    assert_has_text(&driver, "restore me");
}

#[test]
fn resize_emits_clear_output() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Text::new("clear").build(&factory, &theme);
    let mut driver = mounted(root, 20, 1, theme);
    driver.clear_output();

    driver.resize(25, 2).unwrap();

    assert!(driver.output_contains("CSI 2 J"));
}

#[test]
fn resize_preserves_focused_input() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Col::new()
        .children([
            Input::new().placeholder("first").build(&factory, &theme),
            Input::new().placeholder("second").build(&factory, &theme),
        ])
        .build(&factory, &theme);
    let mut driver = mounted(root, 30, 2, theme);

    driver.focus_next().unwrap();
    let focused = driver.focused_widget();
    driver.resize(40, 3).unwrap();

    assert_eq!(driver.focused_widget(), focused);
}

#[test]
fn resize_preserves_input_buffer() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Input::new().placeholder("cmd").build(&factory, &theme);
    let mut driver = mounted(root, 20, 1, theme);

    driver.focus_next().unwrap();
    driver.send_chars("hello").unwrap();
    driver.resize(30, 2).unwrap();

    assert_has_text(&driver, "hello");
}

#[test]
fn resize_preserves_active_tab() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Tabs::new(0)
        .tabs(vec![
            TabDef {
                label: "One".to_string(),
                content: Text::new("one content").build(&factory, &theme),
            },
            TabDef {
                label: "Two".to_string(),
                content: Text::new("two content").build(&factory, &theme),
            },
        ])
        .build(&factory, &theme);
    let mut driver = mounted(root, 30, 5, theme);

    driver.focus_next().unwrap();
    driver.send_key(Key::ArrowRight).unwrap();
    driver.resize(40, 5).unwrap();

    assert_has_text(&driver, "two content");
    assert_not_text(&driver, "one content");
}

#[test]
fn repeated_resize_uses_last_size() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Text::new("final").build(&factory, &theme);
    let mut driver = mounted(root, 10, 1, theme);

    driver.resize(20, 2).unwrap();
    driver.resize(8, 1).unwrap();
    driver.resize(50, 4).unwrap();

    assert_eq!(driver.screen().cols(), 50);
    assert_eq!(driver.screen().rows(), 4);
    assert_has_text(&driver, "final");
}
