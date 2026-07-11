use crossterm::event::{
    KeyCode, KeyEvent as RawKeyEvent, KeyEventKind, KeyModifiers, MediaKeyCode, ModifierKeyCode,
};
use punctum_grid::{GridPos, GridSize, Surface, diff};
use punctum_input::{KeyPhase, LogicalKey, Modifiers, NamedKey};
use punctum_terminal::{
    TerminalCell, TerminalColor, TerminalPlanError, normalize_key_event, plan_patch,
};

fn raw_key(code: KeyCode, modifiers: KeyModifiers, kind: KeyEventKind) -> RawKeyEvent {
    RawKeyEvent::new_with_kind(code, modifiers, kind)
}

fn patch_with_one_changed_cell(
    size: GridSize,
    position: GridPos,
    cell: TerminalCell,
) -> punctum_grid::Patch<TerminalCell> {
    let previous = Surface::filled(size, TerminalCell::default()).unwrap();
    let mut next = previous.clone();
    next.set(position, cell).unwrap();
    diff(&previous, &next)
}

#[test]
fn terminal_cell_preserves_symbol_and_colors() {
    let cell = TerminalCell::new('x', TerminalColor::White, TerminalColor::Blue);

    assert_eq!(cell.symbol(), 'x');
    assert_eq!(cell.foreground(), TerminalColor::White);
    assert_eq!(cell.background(), TerminalColor::Blue);
}

#[test]
fn terminal_cell_defaults_to_a_blank_with_default_colors() {
    assert_eq!(
        TerminalCell::default(),
        TerminalCell::new(' ', TerminalColor::Default, TerminalColor::Default)
    );
}

#[test]
fn plan_patch_scales_logical_columns_without_changing_rows() {
    let changed = TerminalCell::new('x', TerminalColor::Red, TerminalColor::Black);
    let patch = patch_with_one_changed_cell(GridSize::new(3, 2), GridPos::new(1, 1), changed);

    let runs = plan_patch(&patch, 2).unwrap();

    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].col(), 2);
    assert_eq!(runs[0].row(), 1);
    assert_eq!(runs[0].cells(), &[changed]);
}

#[test]
fn plan_patch_keeps_replacement_rows_in_order() {
    let empty = Surface::filled(GridSize::new(0, 0), TerminalCell::default()).unwrap();
    let next = Surface::from_cells(
        GridSize::new(2, 2),
        vec![
            TerminalCell::new('a', TerminalColor::White, TerminalColor::Black),
            TerminalCell::new('b', TerminalColor::White, TerminalColor::Black),
            TerminalCell::new('c', TerminalColor::White, TerminalColor::Black),
            TerminalCell::new('d', TerminalColor::White, TerminalColor::Black),
        ],
    )
    .unwrap();

    let runs = plan_patch(&diff(&empty, &next), 1).unwrap();

    assert_eq!(runs.len(), 2);
    assert_eq!((runs[0].col(), runs[0].row()), (0, 0));
    assert_eq!((runs[1].col(), runs[1].row()), (0, 1));
    assert_eq!(runs[0].cells()[0].symbol(), 'a');
    assert_eq!(runs[1].cells()[1].symbol(), 'd');
}

#[test]
fn plan_patch_rejects_zero_width_cells() {
    let patch = patch_with_one_changed_cell(
        GridSize::new(1, 1),
        GridPos::new(0, 0),
        TerminalCell::default(),
    );

    assert_eq!(plan_patch(&patch, 0), Err(TerminalPlanError::ZeroCellWidth));
}

#[test]
fn plan_patch_rejects_a_scaled_column_that_exceeds_terminal_coordinates() {
    let patch = patch_with_one_changed_cell(
        GridSize::new(32_769, 1),
        GridPos::new(32_768, 0),
        TerminalCell::new('x', TerminalColor::White, TerminalColor::Black),
    );

    assert_eq!(
        plan_patch(&patch, 2),
        Err(TerminalPlanError::CoordinateOverflow {
            col: 32_768,
            row: 0,
            cell_width: 2,
        })
    );
}

#[test]
fn plan_patch_rejects_a_row_that_exceeds_terminal_coordinates() {
    let patch = patch_with_one_changed_cell(
        GridSize::new(1, 65_537),
        GridPos::new(0, 65_536),
        TerminalCell::new('x', TerminalColor::White, TerminalColor::Black),
    );

    assert_eq!(
        plan_patch(&patch, 1),
        Err(TerminalPlanError::CoordinateOverflow {
            col: 0,
            row: 65_536,
            cell_width: 1,
        })
    );
}

#[test]
fn terminal_plan_errors_have_actionable_messages() {
    assert!(
        TerminalPlanError::ZeroCellWidth
            .to_string()
            .contains("width")
    );
    assert!(
        TerminalPlanError::CoordinateOverflow {
            col: 1,
            row: 2,
            cell_width: 3,
        }
        .to_string()
        .contains("1")
    );
}

#[test]
fn normalize_key_event_maps_common_named_keys() {
    let cases = [
        (KeyCode::Enter, NamedKey::Enter),
        (KeyCode::Esc, NamedKey::Escape),
        (KeyCode::Backspace, NamedKey::Backspace),
        (KeyCode::Tab, NamedKey::Tab),
        (KeyCode::BackTab, NamedKey::Tab),
        (KeyCode::Left, NamedKey::ArrowLeft),
        (KeyCode::Right, NamedKey::ArrowRight),
        (KeyCode::Up, NamedKey::ArrowUp),
        (KeyCode::Down, NamedKey::ArrowDown),
        (KeyCode::Home, NamedKey::Home),
        (KeyCode::End, NamedKey::End),
        (KeyCode::PageUp, NamedKey::PageUp),
        (KeyCode::PageDown, NamedKey::PageDown),
        (KeyCode::Insert, NamedKey::Insert),
        (KeyCode::Delete, NamedKey::Delete),
        (KeyCode::F(12), NamedKey::Function(12)),
    ];

    for (raw, expected) in cases {
        let normalized =
            normalize_key_event(raw_key(raw, KeyModifiers::empty(), KeyEventKind::Press));
        assert_eq!(normalized.logical, LogicalKey::Named(expected));
        assert_eq!(normalized.physical, None);
    }
}

#[test]
fn normalize_key_event_maps_characters_and_space_without_inventing_physical_keys() {
    let character = normalize_key_event(raw_key(
        KeyCode::Char('界'),
        KeyModifiers::empty(),
        KeyEventKind::Press,
    ));
    let space = normalize_key_event(raw_key(
        KeyCode::Char(' '),
        KeyModifiers::empty(),
        KeyEventKind::Press,
    ));

    assert_eq!(character.logical, LogicalKey::Character("界".into()));
    assert_eq!(character.physical, None);
    assert_eq!(space.logical, LogicalKey::Named(NamedKey::Space));
    assert_eq!(space.physical, None);
}

#[test]
fn normalize_key_event_preserves_press_repeat_and_release() {
    let cases = [
        (KeyEventKind::Press, KeyPhase::Press),
        (KeyEventKind::Repeat, KeyPhase::Repeat),
        (KeyEventKind::Release, KeyPhase::Release),
    ];

    for (raw, expected) in cases {
        let normalized = normalize_key_event(raw_key(KeyCode::Left, KeyModifiers::empty(), raw));
        assert_eq!(normalized.phase, expected);
    }
}

#[test]
fn normalize_key_event_preserves_supported_modifiers() {
    let normalized = normalize_key_event(raw_key(
        KeyCode::Char('x'),
        KeyModifiers::SHIFT | KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER,
        KeyEventKind::Press,
    ));

    assert_eq!(
        normalized.modifiers,
        Modifiers {
            shift: true,
            control: true,
            alt: true,
            super_key: true,
        }
    );
}

#[test]
fn normalize_key_event_maps_modifier_keys_when_the_terminal_identifies_them() {
    let cases = [
        (ModifierKeyCode::LeftShift, NamedKey::Shift),
        (ModifierKeyCode::RightShift, NamedKey::Shift),
        (ModifierKeyCode::LeftControl, NamedKey::Control),
        (ModifierKeyCode::RightControl, NamedKey::Control),
        (ModifierKeyCode::LeftAlt, NamedKey::Alt),
        (ModifierKeyCode::RightAlt, NamedKey::Alt),
        (ModifierKeyCode::LeftSuper, NamedKey::Super),
        (ModifierKeyCode::RightSuper, NamedKey::Super),
    ];

    for (raw, expected) in cases {
        let normalized = normalize_key_event(raw_key(
            KeyCode::Modifier(raw),
            KeyModifiers::empty(),
            KeyEventKind::Press,
        ));
        assert_eq!(normalized.logical, LogicalKey::Named(expected));
    }
}

#[test]
fn normalize_key_event_maps_unsupported_keys_to_unidentified() {
    let keys = [
        KeyCode::Null,
        KeyCode::CapsLock,
        KeyCode::ScrollLock,
        KeyCode::NumLock,
        KeyCode::PrintScreen,
        KeyCode::Pause,
        KeyCode::Menu,
        KeyCode::KeypadBegin,
        KeyCode::Media(MediaKeyCode::Play),
        KeyCode::Modifier(ModifierKeyCode::LeftHyper),
        KeyCode::Modifier(ModifierKeyCode::LeftMeta),
        KeyCode::Modifier(ModifierKeyCode::IsoLevel3Shift),
        KeyCode::Modifier(ModifierKeyCode::IsoLevel5Shift),
    ];

    for raw in keys {
        let normalized =
            normalize_key_event(raw_key(raw, KeyModifiers::empty(), KeyEventKind::Press));
        assert_eq!(normalized.logical, LogicalKey::Unidentified);
    }
}
