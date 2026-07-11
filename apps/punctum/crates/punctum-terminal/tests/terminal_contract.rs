use crossterm::event::{
    Event as RawEvent, KeyCode, KeyEvent as RawKeyEvent, KeyEventKind, KeyModifiers, MediaKeyCode,
    ModifierKeyCode,
};
use punctum_grid::{GridPos, GridSize, Surface, diff};
use punctum_input::{KeyPhase, LogicalKey, Modifiers, NamedKey, TextEvent, TextEventError};
use punctum_terminal::{
    TerminalCell, TerminalCellError, TerminalColor, TerminalPlanError, TerminalPresenter,
    TerminalTextError, normalize_key_event, normalize_text_event, plan_patch, resize_text_surface,
    write_text,
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

    assert_eq!(cell.grapheme(), Some("x"));
    assert_eq!(cell.foreground(), TerminalColor::White);
    assert_eq!(cell.background(), TerminalColor::Blue);
    assert!(!cell.is_continuation());
}

#[test]
fn terminal_cell_defaults_to_a_blank_with_default_colors() {
    assert_eq!(
        TerminalCell::default(),
        TerminalCell::new(' ', TerminalColor::Default, TerminalColor::Default)
    );
}

#[test]
fn terminal_cell_accepts_exactly_one_grapheme() {
    let combining =
        TerminalCell::from_grapheme("e\u{301}", TerminalColor::White, TerminalColor::Black)
            .unwrap();
    let emoji =
        TerminalCell::from_grapheme("👩‍💻", TerminalColor::White, TerminalColor::Black).unwrap();

    assert_eq!(combining.grapheme(), Some("e\u{301}"));
    assert_eq!(emoji.grapheme(), Some("👩‍💻"));
    assert_eq!(
        TerminalCell::from_grapheme("", TerminalColor::White, TerminalColor::Black),
        Err(TerminalCellError::EmptyGrapheme)
    );
    assert_eq!(
        TerminalCell::from_grapheme("ab", TerminalColor::White, TerminalColor::Black),
        Err(TerminalCellError::MultipleGraphemes)
    );
    assert!(
        TerminalCellError::EmptyGrapheme
            .to_string()
            .contains("empty")
    );
    assert!(
        TerminalCellError::MultipleGraphemes
            .to_string()
            .contains("one grapheme")
    );
}

#[test]
fn write_text_expands_unicode_into_lead_and_continuation_cells() {
    let mut surface = Surface::filled(GridSize::new(8, 1), TerminalCell::default()).unwrap();
    let event = TextEvent::new("Ae\u{301}界👩‍💻").unwrap();

    let cursor = write_text(
        &mut surface,
        GridPos::new(0, 0),
        &event,
        TerminalColor::White,
        TerminalColor::Black,
    )
    .unwrap();

    assert_eq!(cursor, GridPos::new(6, 0));
    assert_eq!(
        surface.get(GridPos::new(0, 0)).unwrap().grapheme(),
        Some("A")
    );
    assert_eq!(
        surface.get(GridPos::new(1, 0)).unwrap().grapheme(),
        Some("e\u{301}")
    );
    assert_eq!(
        surface.get(GridPos::new(2, 0)).unwrap().grapheme(),
        Some("界")
    );
    assert!(surface.get(GridPos::new(3, 0)).unwrap().is_continuation());
    assert_eq!(
        surface.get(GridPos::new(4, 0)).unwrap().grapheme(),
        Some("👩‍💻")
    );
    assert!(surface.get(GridPos::new(5, 0)).unwrap().is_continuation());
}

#[test]
fn overwriting_either_wide_grapheme_slot_clears_the_other_slot() {
    for overwrite_col in [0, 1] {
        let mut surface = Surface::filled(GridSize::new(3, 1), TerminalCell::default()).unwrap();
        write_text(
            &mut surface,
            GridPos::new(0, 0),
            &TextEvent::new("界").unwrap(),
            TerminalColor::White,
            TerminalColor::Black,
        )
        .unwrap();

        write_text(
            &mut surface,
            GridPos::new(overwrite_col, 0),
            &TextEvent::new("x").unwrap(),
            TerminalColor::Red,
            TerminalColor::Black,
        )
        .unwrap();

        let other_col = 1 - overwrite_col;
        assert_eq!(
            surface
                .get(GridPos::new(overwrite_col, 0))
                .unwrap()
                .grapheme(),
            Some("x")
        );
        assert_eq!(
            surface.get(GridPos::new(other_col, 0)).unwrap(),
            &TerminalCell::default()
        );
    }
}

#[test]
fn write_text_clips_a_wide_grapheme_as_a_whole() {
    let mut surface = Surface::filled(GridSize::new(3, 1), TerminalCell::default()).unwrap();

    let cursor = write_text(
        &mut surface,
        GridPos::new(2, 0),
        &TextEvent::new("界").unwrap(),
        TerminalColor::White,
        TerminalColor::Black,
    )
    .unwrap();

    assert_eq!(cursor, GridPos::new(3, 0));
    assert_eq!(
        surface.get(GridPos::new(2, 0)).unwrap(),
        &TerminalCell::default()
    );
}

#[test]
fn write_text_rejects_each_out_of_bounds_direction() {
    let event = TextEvent::new("x").unwrap();
    let size = GridSize::new(2, 2);
    for position in [
        GridPos::new(-1, 0),
        GridPos::new(0, -1),
        GridPos::new(2, 0),
        GridPos::new(0, 2),
    ] {
        let mut surface = Surface::filled(size, TerminalCell::default()).unwrap();
        let error = write_text(
            &mut surface,
            position,
            &event,
            TerminalColor::White,
            TerminalColor::Black,
        )
        .unwrap_err();

        assert_eq!(
            error,
            TerminalTextError::PositionOutOfBounds { position, size }
        );
        assert!(error.to_string().contains("outside"));
    }
}

#[test]
fn write_text_ignores_zero_width_graphemes_and_stops_at_the_row_end() {
    let mut surface = Surface::filled(GridSize::new(1, 1), TerminalCell::default()).unwrap();

    let cursor = write_text(
        &mut surface,
        GridPos::new(0, 0),
        &TextEvent::new("\u{301}xy").unwrap(),
        TerminalColor::White,
        TerminalColor::Black,
    )
    .unwrap();

    assert_eq!(cursor, GridPos::new(1, 0));
    assert_eq!(surface.cells()[0].grapheme(), Some("x"));
}

#[test]
fn resize_never_keeps_half_of_a_wide_grapheme() {
    let mut surface = Surface::filled(GridSize::new(3, 1), TerminalCell::default()).unwrap();
    write_text(
        &mut surface,
        GridPos::new(1, 0),
        &TextEvent::new("界").unwrap(),
        TerminalColor::White,
        TerminalColor::Black,
    )
    .unwrap();

    let clipped = resize_text_surface(&surface, GridSize::new(2, 1)).unwrap();
    let expanded = resize_text_surface(&clipped, GridSize::new(4, 2)).unwrap();

    assert_eq!(
        clipped.cells(),
        &[TerminalCell::default(), TerminalCell::default()]
    );
    assert!(expanded.cells().iter().all(|cell| !cell.is_continuation()));
}

#[test]
fn resize_preserves_complete_pairs_and_cleans_orphan_continuations() {
    let mut surface = Surface::filled(GridSize::new(2, 1), TerminalCell::default()).unwrap();
    write_text(
        &mut surface,
        GridPos::new(0, 0),
        &TextEvent::new("界").unwrap(),
        TerminalColor::White,
        TerminalColor::Black,
    )
    .unwrap();

    let preserved = resize_text_surface(&surface, GridSize::new(3, 1)).unwrap();
    assert_eq!(
        preserved.get(GridPos::new(0, 0)).unwrap().grapheme(),
        Some("界")
    );
    assert!(preserved.get(GridPos::new(1, 0)).unwrap().is_continuation());

    surface
        .set(GridPos::new(0, 0), TerminalCell::default())
        .unwrap();
    let cleaned = resize_text_surface(&surface, GridSize::new(2, 1)).unwrap();
    assert!(cleaned.cells().iter().all(|cell| !cell.is_continuation()));
}

#[test]
fn resize_reports_surface_capacity_overflow() {
    let surface = Surface::filled(GridSize::new(0, 0), TerminalCell::default()).unwrap();

    assert!(resize_text_surface(&surface, GridSize::new(u32::MAX, u32::MAX)).is_err());
}

#[test]
fn plan_patch_scales_logical_columns_without_changing_rows() {
    let changed = TerminalCell::new('x', TerminalColor::Red, TerminalColor::Black);
    let patch =
        patch_with_one_changed_cell(GridSize::new(3, 2), GridPos::new(1, 1), changed.clone());

    let runs = plan_patch(&patch, 2).unwrap();

    assert_eq!(runs.runs().len(), 1);
    assert_eq!(runs.runs()[0].col(), 2);
    assert_eq!(runs.runs()[0].row(), 1);
    assert_eq!(runs.runs()[0].cells(), &[changed]);
    assert_eq!(runs.final_cursor(), (0, 0));
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

    assert_eq!(runs.runs().len(), 2);
    assert_eq!((runs.runs()[0].col(), runs.runs()[0].row()), (0, 0));
    assert_eq!((runs.runs()[1].col(), runs.runs()[1].row()), (0, 1));
    assert_eq!(runs.runs()[0].cells()[0].grapheme(), Some("a"));
    assert_eq!(runs.runs()[1].cells()[1].grapheme(), Some("d"));
}

#[test]
fn plan_patch_replaces_an_unpaired_wide_grapheme() {
    let wide =
        TerminalCell::from_grapheme("界", TerminalColor::Yellow, TerminalColor::Blue).unwrap();
    let patch = patch_with_one_changed_cell(GridSize::new(1, 1), GridPos::new(0, 0), wide);

    let plan = plan_patch(&patch, 1).unwrap();
    let fallback = &plan.runs()[0].cells()[0];

    assert_eq!(fallback.grapheme(), Some("\u{fffd}"));
    assert_eq!(fallback.foreground(), TerminalColor::Yellow);
    assert_eq!(fallback.background(), TerminalColor::Blue);
}

#[test]
fn plan_patch_replaces_zero_width_and_orphan_continuation_cells() {
    let zero_width =
        TerminalCell::from_grapheme("\u{301}", TerminalColor::White, TerminalColor::Black).unwrap();
    let zero_width_patch =
        patch_with_one_changed_cell(GridSize::new(1, 1), GridPos::new(0, 0), zero_width);
    assert_eq!(
        plan_patch(&zero_width_patch, 1).unwrap().runs()[0].cells()[0].grapheme(),
        Some("\u{fffd}")
    );

    let mut next = Surface::filled(GridSize::new(2, 1), TerminalCell::default()).unwrap();
    write_text(
        &mut next,
        GridPos::new(0, 0),
        &TextEvent::new("界").unwrap(),
        TerminalColor::White,
        TerminalColor::Black,
    )
    .unwrap();
    let mut previous = next.clone();
    previous
        .set(GridPos::new(1, 0), TerminalCell::default())
        .unwrap();
    let plan = plan_patch(&diff(&previous, &next), 1).unwrap();

    assert_eq!(plan.runs()[0].cells()[0].grapheme(), Some("\u{fffd}"));
}

#[test]
fn presenter_parks_the_cursor_at_the_origin_after_unicode_output() {
    let mut surface = Surface::filled(GridSize::new(2, 1), TerminalCell::default()).unwrap();
    write_text(
        &mut surface,
        GridPos::new(0, 0),
        &TextEvent::new("界").unwrap(),
        TerminalColor::White,
        TerminalColor::Black,
    )
    .unwrap();
    let mut presenter = TerminalPresenter::new(Vec::new(), 1).unwrap();

    presenter.present(&surface).unwrap();
    let output = presenter.into_inner();

    assert!(output.ends_with(b"\x1b[0m\x1b[1;1H"));
}

#[test]
fn normalize_text_event_accepts_paste_and_unmodified_character_commits() {
    let text = normalize_text_event(&RawEvent::Paste("你好👩‍💻".into())).unwrap();
    let pressed = normalize_text_event(&RawEvent::Key(raw_key(
        KeyCode::Char('x'),
        KeyModifiers::empty(),
        KeyEventKind::Press,
    )))
    .unwrap();
    let repeated = normalize_text_event(&RawEvent::Key(raw_key(
        KeyCode::Char('X'),
        KeyModifiers::SHIFT,
        KeyEventKind::Repeat,
    )))
    .unwrap();

    assert_eq!(text.unwrap().text(), "你好👩‍💻");
    assert_eq!(pressed.unwrap().text(), "x");
    assert_eq!(repeated.unwrap().text(), "X");
    assert_eq!(
        normalize_text_event(&RawEvent::Paste(String::new())),
        Err(TextEventError::EmptyText)
    );
}

#[test]
fn normalize_text_event_ignores_non_committed_key_events() {
    for event in [
        raw_key(
            KeyCode::Char('x'),
            KeyModifiers::empty(),
            KeyEventKind::Release,
        ),
        raw_key(
            KeyCode::Char('x'),
            KeyModifiers::CONTROL,
            KeyEventKind::Press,
        ),
        raw_key(KeyCode::Char('x'), KeyModifiers::ALT, KeyEventKind::Press),
        raw_key(KeyCode::Char('x'), KeyModifiers::SUPER, KeyEventKind::Press),
        raw_key(KeyCode::Enter, KeyModifiers::empty(), KeyEventKind::Press),
    ] {
        assert_eq!(normalize_text_event(&RawEvent::Key(event)).unwrap(), None);
    }
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
