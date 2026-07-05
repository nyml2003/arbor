mod common;

use std::cell::{Cell as StdCell, RefCell};
use std::rc::Rc;

use arbor_tui_domain::input::{Key, KeyEvent, Modifiers};
use arbor_tui_domain::theme::Theme;
use arbor_tui_widgets::button::Button;
use arbor_tui_widgets::input::Input;
use arbor_tui_widgets::stack::{Col, Row};
use arbor_tui_widgets::widget_factory::WidgetFactory;

use common::{assert_has_text, assert_not_text, mounted};

#[test]
fn tab_focuses_first_input() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Input::new().placeholder("first").build(&factory, &theme);
    let mut driver = mounted(root, 20, 1, theme);

    assert_eq!(driver.focused_widget(), None);
    driver.focus_next().unwrap();

    assert!(driver.focused_widget().is_some());
}

#[test]
fn tab_cycles_focus_between_inputs() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Row::new()
        .children([
            Input::new().placeholder("left").build(&factory, &theme),
            Input::new().placeholder("right").build(&factory, &theme),
        ])
        .build(&factory, &theme);
    let mut driver = mounted(root, 40, 1, theme);

    driver.focus_next().unwrap();
    let first = driver.focused_widget();
    driver.focus_next().unwrap();
    let second = driver.focused_widget();
    driver.focus_next().unwrap();

    assert_ne!(first, second);
    assert_eq!(driver.focused_widget(), first);
}

#[test]
fn shift_tab_cycles_focus_backward() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Col::new()
        .children([
            Input::new().placeholder("one").build(&factory, &theme),
            Input::new().placeholder("two").build(&factory, &theme),
            Input::new().placeholder("three").build(&factory, &theme),
        ])
        .build(&factory, &theme);
    let mut driver = mounted(root, 30, 3, theme);

    driver.focus_next().unwrap();
    let first = driver.focused_widget();
    driver.send_shift_tab().unwrap();

    assert_ne!(driver.focused_widget(), first);
}

#[test]
fn focused_input_accepts_text() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Input::new().placeholder("type").build(&factory, &theme);
    let mut driver = mounted(root, 30, 1, theme);

    driver.focus_next().unwrap();
    driver.send_chars("abc").unwrap();

    assert_has_text(&driver, "abc");
    assert_not_text(&driver, "type");
}

#[test]
fn repeated_characters_are_not_merged() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Input::new().build(&factory, &theme);
    let mut driver = mounted(root, 30, 1, theme);

    driver.focus_next().unwrap();
    driver.send_chars("bookkeeper").unwrap();

    assert_has_text(&driver, "bookkeeper");
}

#[test]
fn arrow_left_inserts_text_in_the_middle() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Input::new().build(&factory, &theme);
    let mut driver = mounted(root, 30, 1, theme);

    driver.focus_next().unwrap();
    driver.send_chars("ac").unwrap();
    driver.send_key(Key::ArrowLeft).unwrap();
    driver.send_chars("b").unwrap();

    assert_has_text(&driver, "abc");
}

#[test]
fn backspace_deletes_character_before_cursor() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Input::new().build(&factory, &theme);
    let mut driver = mounted(root, 30, 1, theme);

    driver.focus_next().unwrap();
    driver.send_chars("abc").unwrap();
    driver.send_key(Key::Backspace).unwrap();

    assert_has_text(&driver, "ab");
    assert_not_text(&driver, "abc");
}

#[test]
fn delete_deletes_character_at_cursor() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Input::new().build(&factory, &theme);
    let mut driver = mounted(root, 30, 1, theme);

    driver.focus_next().unwrap();
    driver.send_chars("abc").unwrap();
    driver.send_key(Key::ArrowLeft).unwrap();
    driver.send_key(Key::ArrowLeft).unwrap();
    driver.send_key(Key::Delete).unwrap();

    assert_has_text(&driver, "ac");
    assert_not_text(&driver, "abc");
}

#[test]
fn home_moves_cursor_to_start_before_typing() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Input::new().build(&factory, &theme);
    let mut driver = mounted(root, 30, 1, theme);

    driver.focus_next().unwrap();
    driver.send_chars("bc").unwrap();
    driver.send_key(Key::Home).unwrap();
    driver.send_chars("a").unwrap();

    assert_has_text(&driver, "abc");
}

#[test]
fn end_moves_cursor_to_end_before_typing() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Input::new().build(&factory, &theme);
    let mut driver = mounted(root, 30, 1, theme);

    driver.focus_next().unwrap();
    driver.send_chars("ac").unwrap();
    driver.send_key(Key::ArrowLeft).unwrap();
    driver.send_key(Key::End).unwrap();
    driver.send_chars("d").unwrap();

    assert_has_text(&driver, "acd");
}

#[test]
fn cjk_character_input_is_visible() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Input::new().build(&factory, &theme);
    let mut driver = mounted(root, 30, 1, theme);

    driver.focus_next().unwrap();
    driver.tick([KeyEvent::char('\u{754c}')]).unwrap();

    assert_has_text(&driver, "\u{754c}");
}

#[test]
fn password_input_masks_original_text() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Input::new().password().build(&factory, &theme);
    let mut driver = mounted(root, 30, 1, theme);

    driver.focus_next().unwrap();
    driver.send_chars("secret").unwrap();

    assert_not_text(&driver, "secret");
    assert_ne!(driver.cell_at(2, 0).ch, 's');
}

#[test]
fn enter_triggers_input_submit_callback() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let submitted = Rc::new(RefCell::new(String::new()));
    let submitted_for_cb = Rc::clone(&submitted);
    let root = Input::new()
        .on_submit(move |value| {
            *submitted_for_cb.borrow_mut() = value;
        })
        .build(&factory, &theme);
    let mut driver = mounted(root, 30, 1, theme);

    driver.focus_next().unwrap();
    driver.send_chars("deploy").unwrap();
    driver.send_key(Key::Enter).unwrap();

    assert_eq!(submitted.borrow().as_str(), "deploy");
}

#[test]
fn on_change_records_each_buffer_change() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let changes = Rc::new(RefCell::new(Vec::<String>::new()));
    let changes_for_cb = Rc::clone(&changes);
    let root = Input::new()
        .on_change(move |value| {
            changes_for_cb.borrow_mut().push(value);
        })
        .build(&factory, &theme);
    let mut driver = mounted(root, 30, 1, theme);

    driver.focus_next().unwrap();
    driver.send_chars("abc").unwrap();

    assert_eq!(
        changes.borrow().as_slice(),
        ["a".to_string(), "ab".to_string(), "abc".to_string()]
    );
}

#[test]
fn ctrl_c_quits_runtime() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Input::new().build(&factory, &theme);
    let mut driver = mounted(root, 20, 1, theme);

    driver.send_ctrl_char('c').unwrap();

    assert!(!driver.is_running());
    assert!(driver.last_step().should_quit);
}

#[test]
fn ctrl_q_quits_runtime() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Input::new().build(&factory, &theme);
    let mut driver = mounted(root, 20, 1, theme);

    driver.send_ctrl_char('q').unwrap();

    assert!(!driver.is_running());
    assert!(driver.last_step().should_quit);
}

#[test]
fn escape_quits_runtime() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Input::new().build(&factory, &theme);
    let mut driver = mounted(root, 20, 1, theme);

    driver.send_key(Key::Escape).unwrap();

    assert!(!driver.is_running());
    assert!(driver.last_step().should_quit);
}

#[test]
fn ctrl_and_alt_modified_chars_do_not_enter_input_buffer() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Input::new().placeholder("blank").build(&factory, &theme);
    let mut driver = mounted(root, 30, 1, theme);

    driver.focus_next().unwrap();
    driver
        .send_modified_key(
            Key::Char('x'),
            Modifiers {
                ctrl: true,
                ..Default::default()
            },
        )
        .unwrap();
    driver
        .send_modified_key(
            Key::Char('y'),
            Modifiers {
                alt: true,
                ..Default::default()
            },
        )
        .unwrap();

    assert_not_text(&driver, "x");
    assert_not_text(&driver, "y");
    assert_has_text(&driver, "blank");
}

#[test]
fn focused_button_enter_triggers_click_callback() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let clicks = Rc::new(StdCell::new(0usize));
    let clicks_for_cb = Rc::clone(&clicks);
    let root = Button::new("Run")
        .on_click(move || clicks_for_cb.set(clicks_for_cb.get() + 1))
        .build(&factory, &theme);
    let mut driver = mounted(root, 20, 1, theme);

    driver.focus_next().unwrap();
    driver.send_key(Key::Enter).unwrap();

    assert_eq!(clicks.get(), 1);
}
