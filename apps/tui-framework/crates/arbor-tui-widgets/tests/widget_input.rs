// Widget tests: Input rendering and key handling.
// Covers rendering (prompt, placeholder, typed content, password, focus),
// input handling (TypeChar, Backspace, Delete, cursor nav, Activate),
// and edge cases (empty buffer, long text, CJK, callbacks).

use std::cell::RefCell;
use std::rc::Rc;

use arbor_tui_adapters::simulated_input::SimulatedInput;
use arbor_tui_domain::input::{
    InputReader, Key, KeyEvent, KeyEventKind, KeyHandleResult, Modifiers,
};
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::WidgetAction;
use arbor_tui_testing::WidgetHarness;
use arbor_tui_widgets::input::Input;
use arbor_tui_widgets::widget_factory::WidgetFactory;

fn wm_and_theme() -> (WidgetFactory, Theme) {
    (WidgetFactory::new(), Theme::dark())
}

// ══════════════════════════════════════════════════════════════════
// Rendering tests
// ══════════════════════════════════════════════════════════════════

#[test]
fn renders_prompt() {
    let (wm, t) = wm_and_theme();
    let input = Input::new().placeholder("type").build(&wm, &t);
    let h = WidgetHarness::render(&input, 40, 1, &t);
    assert_eq!(h.cell_at(0, 0).ch, '›');
    assert_eq!(h.cell_at(1, 0).ch, ' ');
}

#[test]
fn input_callback_change_does_not_change_widget_revision() {
    let (wm, t) = wm_and_theme();
    let left = Input::new().value("same").on_change(|_| {}).build(&wm, &t);
    let right = Input::new().value("same").on_change(|_| {}).build(&wm, &t);

    assert_eq!(left.props_revision(), right.props_revision());
}

#[test]
fn renders_placeholder_when_empty() {
    let (wm, t) = wm_and_theme();
    let input = Input::new().placeholder("search...").build(&wm, &t);
    let h = WidgetHarness::render(&input, 40, 1, &t);
    assert!(!h.find_text("search...").is_empty());
}

#[test]
fn idle_state_uses_dim_prompt_and_placeholder() {
    let (wm, t) = wm_and_theme();
    let input = Input::new().placeholder("search...").build(&wm, &t);
    let h = WidgetHarness::render(&input, 40, 1, &t);

    assert_eq!(h.cell_at(0, 0).fg.palette, t.text_dim().palette);
    let (col, row) = h.find_text("search...")[0];
    assert_eq!(h.cell_at(col, row).fg.palette, t.text_dim().palette);
}

#[test]
fn focused_state_uses_accent_prompt_and_primary_cursor() {
    let (wm, t) = wm_and_theme();
    let input = Input::new().placeholder("search...").build(&wm, &t);
    let h = WidgetHarness::render_with_focus(&input, 40, 1, &t, Some(input.id()));

    assert_eq!(h.cell_at(0, 0).fg.palette, t.accent().palette);
    assert_eq!(h.cell_at(2, 0).bg.palette, t.primary().palette);
}

#[test]
fn focused_empty_input_keeps_placeholder_visible_after_cursor() {
    let (wm, t) = wm_and_theme();
    let input = Input::new().placeholder("blank").build(&wm, &t);
    let h = WidgetHarness::render_with_focus(&input, 40, 1, &t, Some(input.id()));

    assert!(!h.find_text("blank").is_empty());
    assert_eq!(h.cell_at(2, 0).bg.palette, t.primary().palette);
}

#[test]
fn loading_state_renders_spinner_with_warning_style() {
    let (wm, t) = wm_and_theme();
    let input = Input::new()
        .placeholder("waiting")
        .loading(true)
        .loading_phase(1)
        .build(&wm, &t);
    let h = WidgetHarness::render(&input, 40, 1, &t);

    assert_eq!(h.cell_at(0, 0).ch, '◐');
    assert_eq!(h.cell_at(0, 0).fg.palette, t.warning().palette);
    let (col, row) = h.find_text("waiting")[0];
    assert_eq!(h.cell_at(col, row).fg.palette, t.text_dim().palette);
}

#[test]
fn loading_state_blocks_submit() {
    let (wm, t) = wm_and_theme();
    let submitted = Rc::new(RefCell::new(String::new()));
    let submitted2 = submitted.clone();
    let mut input = Input::new()
        .loading(true)
        .on_submit(move |s| {
            let _ = submitted2.replace(s);
        })
        .build(&wm, &t);

    input.perform(&WidgetAction::TypeChar('x'));
    let result = input.perform(&WidgetAction::Activate);

    assert_eq!(result, KeyHandleResult::Handled);
    assert_eq!(submitted.borrow().as_str(), "");
}

#[test]
fn renders_typed_content_after_input() {
    let (wm, t) = wm_and_theme();
    let mut input = Input::new().placeholder("ph").build(&wm, &t);
    input.perform(&WidgetAction::TypeChar('h'));
    input.perform(&WidgetAction::TypeChar('i'));
    let h = WidgetHarness::render(&input, 40, 1, &t);
    // Placeholder should be gone; typed text visible
    assert!(h.find_text("ph").is_empty());
    assert!(!h.find_text("hi").is_empty());
}

#[test]
fn renders_initial_value_from_builder() {
    let (wm, t) = wm_and_theme();
    let input = Input::new().value("/theme light").build(&wm, &t);
    let h = WidgetHarness::render(&input, 40, 1, &t);

    assert!(!h.find_text("/theme light").is_empty());
}

#[test]
fn renders_empty_buffer_without_placeholder() {
    let (wm, t) = wm_and_theme();
    let input = Input::new().build(&wm, &t);
    let h = WidgetHarness::render(&input, 40, 1, &t);
    // Prompt "› " should be visible; nothing else
    assert_eq!(h.cell_at(0, 0).ch, '›');
}

#[test]
fn renders_long_text_truncated_to_width() {
    let (wm, t) = wm_and_theme();
    let mut input = Input::new().build(&wm, &t);
    let long = "A".repeat(100);
    for ch in long.chars() {
        input.perform(&WidgetAction::TypeChar(ch));
    }
    let h = WidgetHarness::render(&input, 20, 1, &t);
    // Content is truncated, but should not panic
    // Column 19 (last visible) should have content, not be blank
    assert_ne!(h.cell_at(19, 0).ch, '\0');
}

#[test]
fn light_theme_no_black_bg() {
    let (wm, _) = wm_and_theme();
    let t = Theme::light();
    let input = Input::new().placeholder("hello").build(&wm, &t);
    let h = WidgetHarness::render(&input, 40, 1, &t);
    h.assert_no_black_bg_on_text().unwrap();
}

#[test]
fn dark_theme_bg_is_surface_alt() {
    let (wm, t) = wm_and_theme();
    let input = Input::new().placeholder("hello").build(&wm, &t);
    let h = WidgetHarness::render(&input, 40, 1, &t);
    // The ">" prompt should use surface_alt background
    assert_eq!(h.cell_at(0, 0).bg.palette, t.surface_alt().palette);
    // Empty space to the right should also be surface_alt
    if 30 < h.cols() {
        assert_eq!(h.cell_at(30, 0).bg.palette, t.surface_alt().palette);
    }
}

#[test]
fn no_black_bg_when_buffer_has_content() {
    let (wm, _) = wm_and_theme();
    let t = Theme::light();
    let mut input = Input::new().placeholder("ph").build(&wm, &t);
    input.perform(&WidgetAction::TypeChar('x'));
    let h = WidgetHarness::render(&input, 40, 1, &t);
    h.assert_no_black_bg_on_text().unwrap();
}

// ══════════════════════════════════════════════════════════════════
// TypeChar
// ══════════════════════════════════════════════════════════════════

#[test]
fn typechar_inserts_at_cursor() {
    let (wm, t) = wm_and_theme();
    let mut input = Input::new().build(&wm, &t);
    assert_eq!(
        input.perform(&WidgetAction::TypeChar('a')),
        KeyHandleResult::Handled
    );
    let h = WidgetHarness::render(&input, 40, 1, &t);
    assert!(!h.find_text("a").is_empty());
}

#[test]
fn multiple_typechar_builds_string() {
    let (wm, t) = wm_and_theme();
    let mut input = Input::new().build(&wm, &t);
    for ch in "hello".chars() {
        input.perform(&WidgetAction::TypeChar(ch));
    }
    let h = WidgetHarness::render(&input, 40, 1, &t);
    assert!(!h.find_text("hello").is_empty());
}

#[test]
fn typechar_triggers_on_change() {
    let (wm, t) = wm_and_theme();
    let changed = Rc::new(RefCell::new(String::new()));
    let changed2 = changed.clone();
    let mut input = Input::new()
        .on_change(move |s| {
            let _ = changed2.replace(s);
        })
        .build(&wm, &t);
    input.perform(&WidgetAction::TypeChar('z'));
    assert_eq!(changed.borrow().clone(), "z");
}

#[test]
fn typechar_cjk_character() {
    let (wm, t) = wm_and_theme();
    let mut input = Input::new().build(&wm, &t);
    input.perform(&WidgetAction::TypeChar('你'));
    let h = WidgetHarness::render(&input, 40, 1, &t);
    assert!(!h.find_text("你").is_empty());
    // CJK char occupies 2 columns, next column should be a phantom
    // (we can't easily check phantom from outside, but verify it renders)
}

// ══════════════════════════════════════════════════════════════════
// Backspace
// ══════════════════════════════════════════════════════════════════

#[test]
fn backspace_deletes_before_cursor() {
    let (wm, t) = wm_and_theme();
    let mut input = Input::new().build(&wm, &t);
    input.perform(&WidgetAction::TypeChar('a'));
    input.perform(&WidgetAction::TypeChar('b'));
    input.perform(&WidgetAction::Backspace);
    let h = WidgetHarness::render(&input, 40, 1, &t);
    assert!(!h.find_text("a").is_empty());
    // "b" should be gone
    assert_eq!(find_count(&h, "b"), 0, "backspace should remove last char");
}

#[test]
fn backspace_at_start_is_noop() {
    let (wm, t) = wm_and_theme();
    let mut input = Input::new().build(&wm, &t);
    // Backspace on empty buffer — should not panic
    let result = input.perform(&WidgetAction::Backspace);
    assert_eq!(result, KeyHandleResult::Handled);
    let h = WidgetHarness::render(&input, 40, 1, &t);
    // Still just the prompt
    assert_eq!(h.cell_at(0, 0).ch, '›');
}

#[test]
fn backspace_triggers_on_change() {
    let (wm, t) = wm_and_theme();
    let changed = Rc::new(RefCell::new(String::new()));
    let changed2 = changed.clone();
    let mut input = Input::new()
        .on_change(move |s| {
            let _ = changed2.replace(s);
        })
        .build(&wm, &t);
    input.perform(&WidgetAction::TypeChar('x'));
    input.perform(&WidgetAction::Backspace);
    assert_eq!(changed.borrow().clone(), "");
}

// ══════════════════════════════════════════════════════════════════
// Delete
// ══════════════════════════════════════════════════════════════════

#[test]
fn delete_removes_at_cursor() {
    let (wm, t) = wm_and_theme();
    let mut input = Input::new().build(&wm, &t);
    input.perform(&WidgetAction::TypeChar('a'));
    input.perform(&WidgetAction::TypeChar('b'));
    // Move cursor left so it's between 'a' and 'b'
    input.perform(&WidgetAction::NavigateLeft);
    // Delete the 'b' (at cursor)
    input.perform(&WidgetAction::Delete);
    let h = WidgetHarness::render(&input, 40, 1, &t);
    assert!(!h.find_text("a").is_empty());
    assert_eq!(find_count(&h, "b"), 0);
}

#[test]
fn delete_at_end_is_noop() {
    let (wm, t) = wm_and_theme();
    let mut input = Input::new().build(&wm, &t);
    input.perform(&WidgetAction::TypeChar('x'));
    // Cursor is at end — Delete should do nothing
    let result = input.perform(&WidgetAction::Delete);
    assert_eq!(result, KeyHandleResult::Handled);
    // 'x' still there
    let h = WidgetHarness::render(&input, 40, 1, &t);
    assert!(!h.find_text("x").is_empty());
}

// ══════════════════════════════════════════════════════════════════
// Cursor navigation
// ══════════════════════════════════════════════════════════════════

#[test]
fn cursor_left_right() {
    let (wm, t) = wm_and_theme();
    let mut input = Input::new().build(&wm, &t);
    for ch in "abc".chars() {
        input.perform(&WidgetAction::TypeChar(ch));
    }
    // Cursor at end; move left then type 'X' between 'b' and 'c'
    input.perform(&WidgetAction::NavigateLeft);
    input.perform(&WidgetAction::TypeChar('X'));
    let h = WidgetHarness::render(&input, 40, 1, &t);
    assert!(!h.find_text("abXc").is_empty());
}

#[test]
fn cursor_left_at_start_is_noop() {
    let (wm, t) = wm_and_theme();
    let mut input = Input::new().build(&wm, &t);
    input.perform(&WidgetAction::NavigateLeft);
    // Should not panic; then type at pos 0
    input.perform(&WidgetAction::TypeChar('X'));
    let h = WidgetHarness::render(&input, 40, 1, &t);
    assert!(!h.find_text("X").is_empty());
}

#[test]
fn cursor_right_at_end_is_noop() {
    let (wm, t) = wm_and_theme();
    let mut input = Input::new().build(&wm, &t);
    input.perform(&WidgetAction::NavigateRight);
    // Should not panic
    input.perform(&WidgetAction::TypeChar('X'));
    let h = WidgetHarness::render(&input, 40, 1, &t);
    assert!(!h.find_text("X").is_empty());
}

#[test]
fn home_jumps_to_start() {
    let (wm, t) = wm_and_theme();
    let mut input = Input::new().build(&wm, &t);
    for ch in "hello".chars() {
        input.perform(&WidgetAction::TypeChar(ch));
    }
    // Cursor is at end — Home, then insert at start
    input.perform(&WidgetAction::Home);
    input.perform(&WidgetAction::TypeChar('!'));
    let h = WidgetHarness::render(&input, 40, 1, &t);
    assert!(!h.find_text("!hello").is_empty());
}

#[test]
fn end_jumps_to_end() {
    let (wm, t) = wm_and_theme();
    let mut input = Input::new().build(&wm, &t);
    for ch in "hello".chars() {
        input.perform(&WidgetAction::TypeChar(ch));
    }
    // Move to start, then End, then append
    input.perform(&WidgetAction::Home);
    input.perform(&WidgetAction::End);
    input.perform(&WidgetAction::TypeChar('!'));
    let h = WidgetHarness::render(&input, 40, 1, &t);
    assert!(!h.find_text("hello!").is_empty());
}

// ══════════════════════════════════════════════════════════════════
// Activate (Enter) — submit
// ══════════════════════════════════════════════════════════════════

#[test]
fn activate_triggers_on_submit_with_buffer() {
    let (wm, t) = wm_and_theme();
    let submitted = Rc::new(RefCell::new(String::new()));
    let submitted2 = submitted.clone();
    let mut input = Input::new()
        .on_submit(move |s| {
            let _ = submitted2.replace(s);
        })
        .build(&wm, &t);
    input.perform(&WidgetAction::TypeChar('c'));
    input.perform(&WidgetAction::TypeChar('m'));
    input.perform(&WidgetAction::TypeChar('d'));
    let result = input.perform(&WidgetAction::Activate);
    assert_eq!(result, KeyHandleResult::Handled);
    assert_eq!(submitted.borrow().clone(), "cmd");
}

#[test]
fn activate_with_empty_buffer() {
    let (wm, t) = wm_and_theme();
    let submitted = Rc::new(RefCell::new(String::new()));
    let submitted2 = submitted.clone();
    let mut input = Input::new()
        .on_submit(move |s| {
            let _ = submitted2.replace(s);
        })
        .build(&wm, &t);
    input.perform(&WidgetAction::Activate);
    assert_eq!(submitted.borrow().clone(), "");
}

// ══════════════════════════════════════════════════════════════════
// Password mode
// ══════════════════════════════════════════════════════════════════

#[test]
fn password_mode_masks_content() {
    let (wm, t) = wm_and_theme();
    let mut input = Input::new().password().build(&wm, &t);
    for ch in "secret".chars() {
        input.perform(&WidgetAction::TypeChar(ch));
    }
    let h = WidgetHarness::render(&input, 40, 1, &t);
    // Should NOT show "secret"
    assert_eq!(find_count(&h, "secret"), 0);
    // Should show "●●●●●●" (6 bullets, one for each char)
    let bullets: String = h
        .find_text("●●●●●●")
        .first()
        .map(|(col, row)| {
            (0..6)
                .map(|i| h.cell_at(col + i, *row).ch)
                .collect::<String>()
        })
        .unwrap_or_default();
    assert_eq!(bullets, "●●●●●●");
}

#[test]
fn password_empty_buffer_shows_placeholder() {
    let (wm, t) = wm_and_theme();
    let input = Input::new()
        .password()
        .placeholder("enter password")
        .build(&wm, &t);
    let h = WidgetHarness::render(&input, 40, 1, &t);
    assert!(!h.find_text("enter password").is_empty());
}

// ══════════════════════════════════════════════════════════════════
// Focused rendering
// ══════════════════════════════════════════════════════════════════

#[test]
fn focused_shows_cursor_highlight() {
    let (wm, t) = wm_and_theme();
    let mut input = Input::new().placeholder("input").build(&wm, &t);
    input.perform(&WidgetAction::TypeChar('a'));
    // Render with focus on this widget
    let h = WidgetHarness::render_with_focus(&input, 40, 1, &t, Some(input.id()));
    // Cursor should be highlighted. After typing 'a' the cursor is at col 3
    // ("▸ a"). The cursor cell bg should be the primary color.
    let cursor_col: u16 = 3; // "> " + 'a' + cursor
    assert_eq!(
        h.cell_at(cursor_col, 0).bg.palette,
        t.primary().palette,
        "cursor cell should have primary bg when focused"
    );
}

#[test]
fn focused_long_text_keeps_cursor_visible_at_right_edge() {
    let (wm, t) = wm_and_theme();
    let mut input = Input::new().build(&wm, &t);
    for ch in "abcdefghij".chars() {
        input.perform(&WidgetAction::TypeChar(ch));
    }

    let h = WidgetHarness::render_with_focus(&input, 8, 1, &t, Some(input.id()));

    assert_eq!(
        h.cell_at(6, 0).ch,
        'j',
        "rightmost visible text cell should show the end of the input"
    );
    assert_eq!(
        h.cell_at(7, 0).bg.palette,
        t.primary().palette,
        "cursor should remain visible in the final input cell"
    );
}

#[test]
fn unfocused_has_no_cursor_highlight() {
    let (wm, t) = wm_and_theme();
    let mut input = Input::new().placeholder("x").build(&wm, &t);
    input.perform(&WidgetAction::TypeChar('a'));
    // Render WITHOUT focus
    let h = WidgetHarness::render(&input, 40, 1, &t);
    // The cell at cursor position should NOT have primary bg
    let cursor_col: u16 = 3;
    assert_ne!(h.cell_at(cursor_col, 0).bg.palette, t.primary().palette);
}

// ══════════════════════════════════════════════════════════════════
// Complex sequences
// ══════════════════════════════════════════════════════════════════

#[test]
fn edit_in_middle_of_text() {
    let (wm, t) = wm_and_theme();
    let mut input = Input::new().build(&wm, &t);
    // Type "abdef" → cursor at end (5)
    for ch in "abdef".chars() {
        input.perform(&WidgetAction::TypeChar(ch));
    }
    // Navigate left 3 times: cursor=2 (between 'b' and 'd')
    for _ in 0..3 {
        input.perform(&WidgetAction::NavigateLeft);
    }
    // Backspace removes 'b' at cursor-1 (index 1)
    input.perform(&WidgetAction::Backspace);
    // Type 'c' inserts at cursor=1
    input.perform(&WidgetAction::TypeChar('c'));
    let h = WidgetHarness::render(&input, 40, 1, &t);
    assert!(!h.find_text("acdef").is_empty());
}

#[test]
fn rapid_typing_and_backspace() {
    let (wm, t) = wm_and_theme();
    let mut input = Input::new().build(&wm, &t);
    // Type 10 chars
    for ch in "0123456789".chars() {
        input.perform(&WidgetAction::TypeChar(ch));
    }
    // Backspace 5 times
    for _ in 0..5 {
        input.perform(&WidgetAction::Backspace);
    }
    let h = WidgetHarness::render(&input, 40, 1, &t);
    assert!(!h.find_text("01234").is_empty());
    assert_eq!(find_count(&h, "56789"), 0);
}

#[test]
fn complex_cursor_dance() {
    let (wm, t) = wm_and_theme();
    let mut input = Input::new().build(&wm, &t);
    // Type "ABCD"
    for ch in "ABCD".chars() {
        input.perform(&WidgetAction::TypeChar(ch));
    }
    // Home, right, right (cursor after 'B'), backspace, type 'X'
    input.perform(&WidgetAction::Home);
    input.perform(&WidgetAction::NavigateRight);
    input.perform(&WidgetAction::NavigateRight);
    input.perform(&WidgetAction::Backspace);
    input.perform(&WidgetAction::TypeChar('X'));
    let h = WidgetHarness::render(&input, 40, 1, &t);
    assert!(!h.find_text("AXCD").is_empty());
}

// ══════════════════════════════════════════════════════════════════
// SimulatedInput — full pipeline: key events → actions → render
// ══════════════════════════════════════════════════════════════════

/// Map a KeyEvent to a WidgetAction, mirroring the app's default_keymap.
fn map_key_to_action(event: &KeyEvent) -> Option<WidgetAction> {
    if event.modifiers.ctrl {
        return match event.key {
            Key::Char('c') => Some(WidgetAction::Activate), // Ctrl+C ≈ Enter for testing
            _ => None,
        };
    }
    match &event.key {
        Key::Char(c) => Some(WidgetAction::TypeChar(*c)),
        Key::Enter => Some(WidgetAction::Activate),
        Key::Backspace => Some(WidgetAction::Backspace),
        Key::Delete => Some(WidgetAction::Delete),
        Key::ArrowLeft => Some(WidgetAction::NavigateLeft),
        Key::ArrowRight => Some(WidgetAction::NavigateRight),
        Key::Home => Some(WidgetAction::Home),
        Key::End => Some(WidgetAction::End),
        _ => None,
    }
}

#[test]
fn simulated_input_type_and_render() {
    let (wm, t) = wm_and_theme();
    let sim = SimulatedInput::new();

    // Push key events as if from a real terminal
    sim.push(KeyEvent::char('h'));
    sim.push(KeyEvent::char('e'));
    sim.push(KeyEvent::char('l'));
    sim.push(KeyEvent::char('l'));
    sim.push(KeyEvent::char('o'));

    let mut input = Input::new().placeholder("ph").build(&wm, &t);

    // Drain simulated events and feed to the widget
    for event in sim.poll() {
        if let Some(action) = map_key_to_action(&event) {
            input.perform(&action);
        }
    }

    let h = WidgetHarness::render(&input, 40, 1, &t);
    assert!(!h.find_text("hello").is_empty());
    assert!(h.find_text("ph").is_empty()); // placeholder gone
}

#[test]
fn simulated_input_backspace_and_enter() {
    let (wm, t) = wm_and_theme();
    let sim = SimulatedInput::new();
    let submitted = Rc::new(RefCell::new(String::new()));
    let submitted2 = submitted.clone();

    sim.push(KeyEvent::char('t'));
    sim.push(KeyEvent::char('x'));
    sim.push(KeyEvent::char('t')); // "txt"
    sim.push(KeyEvent {
        key: Key::Backspace,
        modifiers: Modifiers::default(),
        kind: KeyEventKind::Press,
    }); // backspace → "tx"
    sim.push(KeyEvent::char('p')); // "txp"
    sim.push(KeyEvent {
        key: Key::Enter,
        modifiers: Modifiers::default(),
        kind: KeyEventKind::Press,
    }); // submit

    let mut input = Input::new()
        .on_submit(move |s| {
            let _ = submitted2.replace(s);
        })
        .build(&wm, &t);

    for event in sim.poll() {
        if let Some(action) = map_key_to_action(&event) {
            input.perform(&action);
        }
    }

    let h = WidgetHarness::render(&input, 40, 1, &t);
    assert!(!h.find_text("txp").is_empty());
    assert_eq!(submitted.borrow().as_str(), "txp");
}

#[test]
fn simulated_input_cursor_navigation() {
    let (wm, t) = wm_and_theme();
    let sim = SimulatedInput::new();

    // Type "abc"
    sim.push(KeyEvent::char('a'));
    sim.push(KeyEvent::char('b'));
    sim.push(KeyEvent::char('c'));
    // Left arrow to go between 'b' and 'c'
    sim.push(KeyEvent {
        key: Key::ArrowLeft,
        modifiers: Modifiers::default(),
        kind: KeyEventKind::Press,
    });
    // Type 'X' → "abXc"
    sim.push(KeyEvent::char('X'));

    let mut input = Input::new().build(&wm, &t);
    for event in sim.poll() {
        if let Some(action) = map_key_to_action(&event) {
            input.perform(&action);
        }
    }

    let h = WidgetHarness::render(&input, 40, 1, &t);
    assert!(!h.find_text("abXc").is_empty());
}

#[test]
fn simulated_input_ctrl_handling() {
    let (wm, t) = wm_and_theme();
    let sim = SimulatedInput::new();
    let submitted = Rc::new(RefCell::new(String::new()));
    let submitted2 = submitted.clone();

    sim.push(KeyEvent::char('q'));
    sim.push(KeyEvent::char('u'));
    sim.push(KeyEvent::char('i'));
    sim.push(KeyEvent::char('t'));

    let mut input = Input::new()
        .on_submit(move |s| {
            let _ = submitted2.replace(s);
        })
        .build(&wm, &t);

    // No Enter pressed — simulate poll_timeout returning empty
    let events = sim.poll();
    for event in &events {
        if let Some(action) = map_key_to_action(event) {
            input.perform(&action);
        }
    }

    let h = WidgetHarness::render(&input, 40, 1, &t);
    assert!(!h.find_text("quit").is_empty());
    // Not submitted yet (no Enter)
    assert_eq!(submitted.borrow().as_str(), "");
}

#[test]
fn simulated_input_empty_poll_does_nothing() {
    let (wm, t) = wm_and_theme();
    let sim = SimulatedInput::new();
    // No events pushed — poll returns empty
    let events = sim.poll();
    assert!(events.is_empty());

    let mut input = Input::new().placeholder("default").build(&wm, &t);
    for event in events {
        if let Some(action) = map_key_to_action(&event) {
            input.perform(&action);
        }
    }

    // Placeholder still visible
    let h = WidgetHarness::render(&input, 40, 1, &t);
    assert!(!h.find_text("default").is_empty());
}

#[test]
fn simulated_input_poll_timeout_returns_events() {
    let sim = SimulatedInput::new();
    sim.push(KeyEvent::char('a'));
    sim.push(KeyEvent::char('b'));

    // poll_timeout should return immediately when events are queued
    let events = sim.poll_timeout(std::time::Duration::from_secs(10));
    assert_eq!(events.len(), 2);
}

#[test]
fn simulated_input_realistic_typing_session() {
    let (wm, t) = wm_and_theme();
    let sim = SimulatedInput::new();
    let submitted = Rc::new(RefCell::new(String::new()));
    let submitted2 = submitted.clone();

    // Simulate typing "/theme dark" then Enter
    for ch in "/theme dark".chars() {
        sim.push(KeyEvent::char(ch));
    }
    sim.push(KeyEvent {
        key: Key::Enter,
        modifiers: Modifiers::default(),
        kind: KeyEventKind::Press,
    });

    let mut input = Input::new()
        .placeholder("type command...")
        .on_submit(move |s| {
            let _ = submitted2.replace(s);
        })
        .build(&wm, &t);

    // This mirrors what the app's event loop does:
    // 1. poll input
    // 2. map to actions
    // 3. dispatch to focused widget
    let events = sim.poll();
    for event in &events {
        if let Some(action) = map_key_to_action(event) {
            assert_eq!(input.perform(&action), KeyHandleResult::Handled);
        }
    }

    let h = WidgetHarness::render(&input, 40, 1, &t);
    assert!(!h.find_text("/theme dark").is_empty());
    assert_eq!(submitted.borrow().as_str(), "/theme dark");
    // Placeholder should be gone
    assert!(h.find_text("type command").is_empty());
}

// ══════════════════════════════════════════════════════════════════
// Helpers
// ══════════════════════════════════════════════════════════════════

fn find_count(h: &WidgetHarness, needle: &str) -> usize {
    h.find_text(needle).len()
}
