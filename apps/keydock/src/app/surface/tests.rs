use super::animation::{
    advance_ripples, append_ripple, ActiveRipple, RippleStart, RIPPLE_DURATION_MS,
};
use super::interaction::{activate_key, reduce_pointer_event};
use super::*;
use crate::app::input::{KeyCode, Modifier};
use arbor_ui_core::view::components::{Button, ButtonState, ComponentNode, Primitive};

#[test]
fn hit_test_returns_key_inside_rect_and_none_in_gap() {
    let surface = KeyboardSurface::new(KeyboardLayout::qwerty()).unwrap();
    let q_key = surface
        .snapshot
        .keys
        .iter()
        .find(|key| key.spec.id == "key-q")
        .unwrap();

    assert_eq!(
        surface.button_hit_test(Point::new(q_key.rect.x + 2.0, q_key.rect.y + 2.0)),
        Some("key-q")
    );
    assert_eq!(
        surface.button_hit_test(Point::new(q_key.rect.right() + 1.0, q_key.rect.y + 2.0)),
        None
    );
}

#[test]
fn key_generates_button_text_primitive() {
    let surface = KeyboardSurface::new(KeyboardLayout::qwerty()).unwrap();
    let snapshot = surface.snapshot();
    let buttons = collect_buttons(&snapshot.primitive_tree);
    let q_button = buttons
        .iter()
        .find(|button| button.id() == "key-q")
        .unwrap();

    assert!(matches!(q_button.content.as_ref(), Primitive::Text(text) if text.content() == "Q"));
}

#[test]
fn shift_latches_for_one_character_and_updates_label() {
    let mut surface = KeyboardSurface::new(KeyboardLayout::qwerty()).unwrap();
    let shift = center_of(&surface, "key-shift");
    let a = center_of(&surface, "key-a");

    assert!(commands_for(&mut surface, PointerEvent::Down(shift))
        .unwrap()
        .is_empty());
    assert!(commands_for(&mut surface, PointerEvent::Up(shift))
        .unwrap()
        .is_empty());
    assert!(surface.state.shift_latched);

    let snapshot = surface.snapshot();
    let buttons = collect_buttons(&snapshot.primitive_tree);
    let a_button = buttons
        .iter()
        .find(|button| button.id() == "key-a")
        .unwrap();
    assert!(matches!(a_button.content.as_ref(), Primitive::Text(text) if text.content() == "A"));

    commands_for(&mut surface, PointerEvent::Down(a)).unwrap();
    let commands = commands_for(&mut surface, PointerEvent::Up(a)).unwrap();

    assert_eq!(commands, vec![InputCommand::Text('A')]);
    assert!(!surface.state.shift_latched);
}

#[test]
fn ctrl_combination_releases_modifier() {
    let mut surface = KeyboardSurface::new(KeyboardLayout::qwerty()).unwrap();
    let ctrl = center_of(&surface, "key-ctrl");
    let c = center_of(&surface, "key-c");

    commands_for(&mut surface, PointerEvent::Down(ctrl)).unwrap();
    commands_for(&mut surface, PointerEvent::Up(ctrl)).unwrap();
    assert!(surface.state.ctrl_active);

    commands_for(&mut surface, PointerEvent::Down(c)).unwrap();
    let commands = commands_for(&mut surface, PointerEvent::Up(c)).unwrap();

    assert_eq!(
        commands,
        vec![InputCommand::ModifiedKeyTap {
            modifiers: vec![Modifier::Control],
            key: KeyCode::Character('c')
        }]
    );
    assert!(!surface.state.ctrl_active);
}

#[test]
fn pointer_cancel_clears_pressed_and_hovered_state() {
    let mut surface = KeyboardSurface::new(KeyboardLayout::qwerty()).unwrap();
    let q = center_of(&surface, "key-q");

    commands_for(&mut surface, PointerEvent::Down(q)).unwrap();
    assert_eq!(surface.state.pressed_key.as_deref(), Some("key-q"));
    commands_for(&mut surface, PointerEvent::Cancel).unwrap();

    assert_eq!(surface.state.pressed_key, None);
    assert_eq!(surface.state.hovered_key, None);
}

#[test]
fn pointer_update_marks_render_only_when_visual_state_changes() {
    let mut surface = KeyboardSurface::new(KeyboardLayout::qwerty()).unwrap();
    let q = center_of(&surface, "key-q");

    let idle_cancel = surface.handle_pointer_event(PointerEvent::Cancel).unwrap();
    assert!(!idle_cancel.needs_render);

    let first_hover = surface.handle_pointer_event(PointerEvent::Move(q)).unwrap();
    assert!(first_hover.needs_render);

    let repeated_hover = surface.handle_pointer_event(PointerEvent::Move(q)).unwrap();
    assert!(!repeated_hover.needs_render);

    let cancel_hover = surface.handle_pointer_event(PointerEvent::Cancel).unwrap();
    assert!(cancel_hover.needs_render);

    let down = surface.handle_pointer_event(PointerEvent::Down(q)).unwrap();
    assert!(down.needs_render);

    let up = surface.handle_pointer_event(PointerEvent::Up(q)).unwrap();
    assert!(up.needs_render);
    assert_eq!(up.commands, vec![InputCommand::Text('q')]);

    let cancel_after_release = surface.handle_pointer_event(PointerEvent::Cancel).unwrap();
    assert!(cancel_after_release.needs_render);

    let repeated_cancel = surface.handle_pointer_event(PointerEvent::Cancel).unwrap();
    assert!(!repeated_cancel.needs_render);
}

#[test]
fn modifier_active_maps_to_active_button() {
    let mut surface = KeyboardSurface::new(KeyboardLayout::qwerty()).unwrap();
    let alt = center_of(&surface, "key-alt");

    commands_for(&mut surface, PointerEvent::Down(alt)).unwrap();
    commands_for(&mut surface, PointerEvent::Up(alt)).unwrap();

    let snapshot = surface.snapshot();
    let buttons = collect_buttons(&snapshot.primitive_tree);
    let alt_button = buttons
        .iter()
        .find(|button| button.id() == "key-alt")
        .unwrap();
    assert_eq!(alt_button.state(), ButtonState::Active);
}

#[test]
fn title_close_uses_image_capable_action_button_slot() {
    let surface = KeyboardSurface::new(KeyboardLayout::qwerty()).unwrap();
    let close_rect = surface.close_title_rect();

    let snapshot = surface.snapshot();
    let buttons = collect_buttons(&snapshot.primitive_tree);
    let close_button = buttons
        .iter()
        .find(|button| button.id() == "key-close-title")
        .unwrap();

    assert_eq!(close_button.rect(), close_rect);
    assert!(matches!(close_button.content.as_ref(), Primitive::Image(_)));
}

#[test]
fn title_close_is_a_clickable_command() {
    let mut surface = KeyboardSurface::new(KeyboardLayout::qwerty()).unwrap();
    let close_rect = surface.close_title_rect();
    let close_point = Point::new(
        close_rect.x + close_rect.width / 2.0,
        close_rect.y + close_rect.height / 2.0,
    );

    assert_eq!(
        surface.chrome_hit_test(close_point),
        ChromeHit::Command(InputCommand::CloseApp)
    );
    assert!(commands_for(&mut surface, PointerEvent::Down(close_point))
        .unwrap()
        .is_empty());
    assert_eq!(
        commands_for(&mut surface, PointerEvent::Up(close_point)).unwrap(),
        vec![InputCommand::CloseApp]
    );
}

#[test]
fn title_drag_area_excludes_close_button() {
    let surface = KeyboardSurface::new(KeyboardLayout::qwerty()).unwrap();
    let drag_rect = surface.drag_title_rect();
    let drag_point = Point::new(drag_rect.x + 8.0, drag_rect.y + drag_rect.height / 2.0);
    let close_rect = surface.close_title_rect();
    let close_point = Point::new(
        close_rect.x + close_rect.width / 2.0,
        close_rect.y + close_rect.height / 2.0,
    );

    assert_eq!(surface.chrome_hit_test(drag_point), ChromeHit::Drag);
    assert_ne!(surface.chrome_hit_test(close_point), ChromeHit::Drag);
}

#[test]
fn button_down_starts_ripple_without_blocking_key_command() {
    let mut surface = KeyboardSurface::new(KeyboardLayout::qwerty()).unwrap();
    let q = center_of(&surface, "key-q");

    assert!(commands_for(&mut surface, PointerEvent::Down(q))
        .unwrap()
        .is_empty());
    assert!(surface.has_active_animations());

    let snapshot = surface.snapshot();
    let buttons = collect_buttons(&snapshot.primitive_tree);
    let q_button = buttons
        .iter()
        .find(|button| button.id() == "key-q")
        .unwrap();
    assert_eq!(q_button.ripples().len(), 1);
    assert_eq!(q_button.ripples()[0].origin, q);

    let commands = commands_for(&mut surface, PointerEvent::Up(q)).unwrap();
    assert_eq!(commands, vec![InputCommand::Text('q')]);
}

#[test]
fn title_close_starts_ripple_and_remains_clickable() {
    let mut surface = KeyboardSurface::new(KeyboardLayout::qwerty()).unwrap();
    let close_rect = surface.close_title_rect();
    let close_point = Point::new(
        close_rect.x + close_rect.width / 2.0,
        close_rect.y + close_rect.height / 2.0,
    );

    commands_for(&mut surface, PointerEvent::Down(close_point)).unwrap();
    let snapshot = surface.snapshot();
    let buttons = collect_buttons(&snapshot.primitive_tree);
    let close_button = buttons
        .iter()
        .find(|button| button.id() == "key-close-title")
        .unwrap();
    assert_eq!(close_button.ripples().len(), 1);

    assert_eq!(
        commands_for(&mut surface, PointerEvent::Up(close_point)).unwrap(),
        vec![InputCommand::CloseApp]
    );
}

#[test]
fn status_indicator_does_not_start_ripple() {
    let mut surface = KeyboardSurface::new(KeyboardLayout::qwerty()).unwrap();
    let status_point = Point::new(
        surface.snapshot.status_rect.x + 4.0,
        surface.snapshot.status_rect.y + 4.0,
    );

    commands_for(&mut surface, PointerEvent::Down(status_point)).unwrap();

    assert!(!surface.has_active_animations());
}

#[test]
fn ripple_is_removed_after_duration() {
    let mut surface = KeyboardSurface::new(KeyboardLayout::qwerty()).unwrap();
    let q = center_of(&surface, "key-q");

    commands_for(&mut surface, PointerEvent::Down(q)).unwrap();
    surface.advance_animations(RIPPLE_DURATION_MS + 1.0);

    assert!(!surface.has_active_animations());
}

#[test]
fn reduce_pointer_event_returns_next_state_and_activation_without_mutating_input() {
    let state = KeyboardState {
        hovered_key: Some("old-hover".to_string()),
        pressed_key: Some("key-q".to_string()),
        ..KeyboardState::default()
    };

    let moved = reduce_pointer_event(
        &state,
        PointerEvent::Move(Point::new(1.0, 1.0)),
        Some("key-w"),
    );
    assert_eq!(moved.next_state.hovered_key.as_deref(), Some("key-w"));
    assert_eq!(moved.next_state.pressed_key.as_deref(), Some("key-q"));
    assert_eq!(state.hovered_key.as_deref(), Some("old-hover"));

    let released = reduce_pointer_event(
        &state,
        PointerEvent::Up(Point::new(1.0, 1.0)),
        Some("key-q"),
    );
    assert_eq!(released.next_state.pressed_key, None);
    assert_eq!(released.next_state.hovered_key.as_deref(), Some("key-q"));
    assert_eq!(released.activation.as_deref(), Some("key-q"));

    let cancelled = reduce_pointer_event(&state, PointerEvent::Cancel, None);
    assert_eq!(cancelled.next_state.pressed_key, None);
    assert_eq!(cancelled.next_state.hovered_key, None);
    assert_eq!(state.pressed_key.as_deref(), Some("key-q"));
}

#[test]
fn reduce_pointer_down_describes_ripple_start() {
    let point = Point::new(3.0, 4.0);
    let outcome = reduce_pointer_event(
        &KeyboardState::default(),
        PointerEvent::Down(point),
        Some("key-q"),
    );

    assert_eq!(outcome.next_state.pressed_key.as_deref(), Some("key-q"));
    assert_eq!(outcome.next_state.hovered_key.as_deref(), Some("key-q"));
    assert_eq!(
        outcome.ripple,
        Some(RippleStart {
            target_id: "key-q".to_string(),
            origin: point,
        })
    );
}

#[test]
fn activate_key_returns_next_state_and_commands() {
    let layout = KeyboardLayout::qwerty();
    let shift = layout
        .rows
        .iter()
        .flatten()
        .find(|key| key.id == "key-shift")
        .unwrap();
    let a = layout
        .rows
        .iter()
        .flatten()
        .find(|key| key.id == "key-a")
        .unwrap();

    let shifted = activate_key(&KeyboardState::default(), shift);
    assert!(shifted.commands.is_empty());
    assert!(shifted.next_state.shift_latched);

    let typed = activate_key(&shifted.next_state, a);
    assert_eq!(typed.commands, vec![InputCommand::Text('A')]);
    assert!(!typed.next_state.shift_latched);
}

#[test]
fn activate_key_releases_ctrl_alt_after_modified_command() {
    let layout = KeyboardLayout::qwerty();
    let c = layout
        .rows
        .iter()
        .flatten()
        .find(|key| key.id == "key-c")
        .unwrap();
    let state = KeyboardState {
        ctrl_active: true,
        alt_active: true,
        ..KeyboardState::default()
    };

    let outcome = activate_key(&state, c);

    assert_eq!(
        outcome.commands,
        vec![InputCommand::ModifiedKeyTap {
            modifiers: vec![Modifier::Control, Modifier::Alt],
            key: KeyCode::Character('c'),
        }]
    );
    assert_eq!(outcome.next_state, KeyboardState::default());
    assert!(state.ctrl_active);
    assert!(state.alt_active);
}

#[test]
fn append_ripple_keeps_latest_two_for_same_target() {
    let first = RippleStart {
        target_id: "key-q".to_string(),
        origin: Point::new(1.0, 1.0),
    };
    let second = RippleStart {
        target_id: "key-q".to_string(),
        origin: Point::new(2.0, 2.0),
    };
    let third = RippleStart {
        target_id: "key-q".to_string(),
        origin: Point::new(3.0, 3.0),
    };

    let ripples = append_ripple(&[], first);
    let ripples = append_ripple(&ripples, second);
    let ripples = append_ripple(&ripples, third);

    assert_eq!(ripples.len(), 2);
    assert_eq!(ripples[0].origin, Point::new(2.0, 2.0));
    assert_eq!(ripples[1].origin, Point::new(3.0, 3.0));
}

#[test]
fn advance_ripples_clamps_negative_delta_and_filters_expired_items() {
    let ripples = vec![
        ActiveRipple {
            target_id: "key-q".to_string(),
            origin: Point::new(1.0, 1.0),
            age_ms: 20.0,
        },
        ActiveRipple {
            target_id: "key-w".to_string(),
            origin: Point::new(2.0, 2.0),
            age_ms: RIPPLE_DURATION_MS - 1.0,
        },
    ];

    let unchanged = advance_ripples(&ripples, -10.0);
    assert_eq!(unchanged[0].age_ms, 20.0);
    assert_eq!(unchanged[1].age_ms, RIPPLE_DURATION_MS - 1.0);

    let advanced = advance_ripples(&ripples, 2.0);
    assert_eq!(advanced.len(), 1);
    assert_eq!(advanced[0].target_id, "key-q");
    assert_eq!(advanced[0].age_ms, 22.0);
}

fn center_of(surface: &KeyboardSurface, key_id: &str) -> Point {
    let key = surface
        .snapshot
        .keys
        .iter()
        .find(|key| key.spec.id == key_id)
        .unwrap();
    Point::new(
        key.rect.x + key.rect.width / 2.0,
        key.rect.y + key.rect.height / 2.0,
    )
}

fn commands_for(
    surface: &mut KeyboardSurface,
    event: PointerEvent,
) -> AppResult<Vec<InputCommand>> {
    surface
        .handle_pointer_event(event)
        .map(|update| update.commands)
}

fn collect_buttons(primitive: &Primitive) -> Vec<&Button> {
    let mut buttons = Vec::new();
    collect_buttons_into(primitive, &mut buttons);
    buttons
}

fn collect_buttons_into<'a>(primitive: &'a Primitive, buttons: &mut Vec<&'a Button>) {
    match primitive {
        Primitive::Surface(surface) => {
            for child in surface.children() {
                collect_buttons_into(child, buttons);
            }
        }
        Primitive::Row(row) => {
            for child in row.children() {
                collect_buttons_into(child, buttons);
            }
        }
        Primitive::Button(button) => {
            buttons.push(button);
            collect_buttons_into(&button.content, buttons);
        }
        Primitive::Text(_) | Primitive::Image(_) => {}
    }
}
