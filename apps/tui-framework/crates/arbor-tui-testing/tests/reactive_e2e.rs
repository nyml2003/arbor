mod common;

use arbor_tui_domain::cell::Attrs;
use arbor_tui_domain::signal::Signal;
use arbor_tui_domain::theme::Theme;
use arbor_tui_widgets::button::Button;
use arbor_tui_widgets::scroll::Scroll;
use arbor_tui_widgets::stack::Col;
use arbor_tui_widgets::text::Text;
use arbor_tui_widgets::widget_factory::WidgetFactory;
use arbor_tui_widgets::TextStyle;

use common::{assert_has_text, assert_not_text, mounted};

#[test]
fn text_signal_update_renders_new_value() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let label = Signal::new("before".to_string());
    let root = Text::new("").content_from(&label).build(&factory, &theme);
    let mut driver = mounted(root, 30, 1, theme);
    let output_len = driver.output_len();

    let step = driver.update_signal(&label, "after".to_string()).unwrap();

    assert!(step.should_render);
    assert!(driver.output_len() > output_len);
    assert_has_text(&driver, "after");
    assert_not_text(&driver, "before");
}

#[test]
fn same_text_signal_value_does_not_emit_output() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let label = Signal::new("same".to_string());
    let root = Text::new("").content_from(&label).build(&factory, &theme);
    let mut driver = mounted(root, 30, 1, theme);
    let output_len = driver.output_len();

    let step = driver.update_signal(&label, "same".to_string()).unwrap();

    assert!(!step.should_render);
    assert_eq!(driver.output_len(), output_len);
}

#[test]
fn button_label_signal_update_renders_new_label() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let label = Signal::new("Run".to_string());
    let root = Button::new("").label_from(&label).build(&factory, &theme);
    let mut driver = mounted(root, 30, 1, theme);

    driver.update_signal(&label, "Stop".to_string()).unwrap();

    assert_has_text(&driver, "Stop");
    assert_not_text(&driver, "Run");
}

#[test]
fn shared_signal_updates_multiple_text_widgets() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let label = Signal::new("old".to_string());
    let root = Col::new()
        .children([
            Text::new("").content_from(&label).build(&factory, &theme),
            Text::new("").content_from(&label).build(&factory, &theme),
        ])
        .build(&factory, &theme);
    let mut driver = mounted(root, 30, 2, theme);

    driver.update_signal(&label, "new".to_string()).unwrap();

    assert_eq!(driver.find_text("new").len(), 2);
}

#[test]
fn text_style_signal_update_changes_cell_background() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let initial = TextStyle {
        fg: theme.text(),
        bg: theme.surface(),
        attrs: Attrs::default(),
    };
    let changed = TextStyle {
        fg: theme.text(),
        bg: theme.warning(),
        attrs: Attrs::default(),
    };
    let style = Signal::new(initial);
    let root = Text::new("styled")
        .style_from(&style)
        .build(&factory, &theme);
    let mut driver = mounted(root, 30, 1, theme.clone());

    driver.update_signal(&style, changed).unwrap();

    let (col, row) = driver.find_text("styled")[0];
    assert_eq!(driver.cell_at(col, row).bg, theme.warning());
}

#[test]
fn scroll_signal_update_marks_runtime_for_render() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let scroll_y = Signal::new(0u16);
    let root = Scroll::new()
        .content_h(4)
        .scroll_y(scroll_y.read_only())
        .child(Text::new("a\nb\nc\nd").build(&factory, &theme))
        .build(&factory, &theme);
    let mut driver = mounted(root, 20, 2, theme);

    let step = driver.update_signal(&scroll_y, 2).unwrap();

    assert!(step.should_render);
    assert_has_text(&driver, "c");
}

#[test]
fn same_button_label_signal_value_does_not_emit_output() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let label = Signal::new("Hold".to_string());
    let root = Button::new("").label_from(&label).build(&factory, &theme);
    let mut driver = mounted(root, 30, 1, theme);
    let output_len = driver.output_len();

    let step = driver.update_signal(&label, "Hold".to_string()).unwrap();

    assert!(!step.should_render);
    assert_eq!(driver.output_len(), output_len);
}

#[test]
fn text_signal_update_after_resize_uses_current_screen_size() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let label = Signal::new("short".to_string());
    let root = Text::new("").content_from(&label).build(&factory, &theme);
    let mut driver = mounted(root, 10, 1, theme);

    driver.resize(30, 2).unwrap();
    driver
        .update_signal(&label, "expanded after resize".to_string())
        .unwrap();

    assert_eq!(driver.screen().cols(), 30);
    assert_has_text(&driver, "expanded after resize");
}

#[test]
fn two_signal_updates_in_two_ticks_update_screen_twice() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let label = Signal::new("one".to_string());
    let root = Text::new("").content_from(&label).build(&factory, &theme);
    let mut driver = mounted(root, 30, 1, theme);

    driver.update_signal(&label, "two".to_string()).unwrap();
    assert_has_text(&driver, "two");
    driver.update_signal(&label, "three".to_string()).unwrap();

    assert_has_text(&driver, "three");
    assert_not_text(&driver, "two");
}

#[test]
fn same_style_signal_value_does_not_emit_output() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let style_value = TextStyle {
        fg: theme.text(),
        bg: theme.surface(),
        attrs: Attrs::default(),
    };
    let style = Signal::new(style_value.clone());
    let root = Text::new("styled")
        .style_from(&style)
        .build(&factory, &theme);
    let mut driver = mounted(root, 30, 1, theme);
    let output_len = driver.output_len();

    let step = driver.update_signal(&style, style_value).unwrap();

    assert!(!step.should_render);
    assert_eq!(driver.output_len(), output_len);
}
