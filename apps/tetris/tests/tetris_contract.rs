use punctum_grid::{GridPos, GridSize};
use punctum_input::{KeyEvent, KeyPhase, LogicalKey, Modifiers, NamedKey, PhysicalKeyCode};
use punctum_tetris::{
    BOARD_HEIGHT, BOARD_WIDTH, PieceKind, Rotation, TetrisCell, TetrisCommand, TetrisError,
    TetrisState, command_for_key, ghost_piece, paint, transition,
};

fn state(sequence: &[PieceKind]) -> TetrisState {
    TetrisState::new(sequence.to_vec()).unwrap()
}

fn apply(mut state: TetrisState, command: TetrisCommand, times: usize) -> TetrisState {
    for _ in 0..times {
        state = transition(&state, command);
    }
    state
}

fn key(physical: Option<PhysicalKeyCode>, logical: LogicalKey, phase: KeyPhase) -> KeyEvent {
    KeyEvent {
        physical,
        logical,
        modifiers: Modifiers::default(),
        phase,
    }
}

#[test]
fn new_rejects_an_empty_piece_sequence() {
    assert_eq!(
        TetrisState::new(Vec::new()),
        Err(TetrisError::EmptyPieceSequence)
    );
}

#[test]
fn empty_piece_sequence_error_has_an_actionable_message() {
    assert!(
        TetrisError::EmptyPieceSequence
            .to_string()
            .contains("piece sequence")
    );
}

#[test]
fn new_spawns_the_first_piece_centered_at_the_top() {
    let state = state(&[PieceKind::T]);
    let active = state.active_piece().unwrap();

    assert_eq!(active.kind(), PieceKind::T);
    assert_eq!(active.rotation(), Rotation::Spawn);
    assert_eq!(active.col(), 3);
    assert_eq!(active.row(), 0);
    assert!(!state.is_game_over());
}

#[test]
fn transition_returns_a_new_state_without_mutating_its_input() {
    let before = state(&[PieceKind::T]);

    let after = transition(&before, TetrisCommand::MoveLeft);

    assert_eq!(before.active_piece().unwrap().col(), 3);
    assert_eq!(after.active_piece().unwrap().col(), 2);
}

#[test]
fn horizontal_movement_stops_at_both_board_edges() {
    let state = apply(state(&[PieceKind::I]), TetrisCommand::MoveLeft, 4);
    assert_eq!(state.active_piece().unwrap().col(), 0);

    let state = apply(state, TetrisCommand::MoveRight, 7);
    assert_eq!(state.active_piece().unwrap().col(), 6);
}

#[test]
fn clockwise_rotation_is_rejected_when_it_would_cross_a_wall() {
    let state = transition(&state(&[PieceKind::I]), TetrisCommand::RotateClockwise);
    let state = apply(state, TetrisCommand::MoveRight, 6);

    let rotated = transition(&state, TetrisCommand::RotateClockwise);

    assert_eq!(state.active_piece().unwrap().col(), 9);
    assert_eq!(rotated.active_piece().unwrap().rotation(), Rotation::Right);
}

#[test]
fn every_piece_can_complete_a_full_rotation_cycle() {
    for kind in PieceKind::ALL {
        let state = apply(state(&[kind]), TetrisCommand::RotateClockwise, 4);
        assert_eq!(state.active_piece().unwrap().rotation(), Rotation::Spawn);
    }
}

#[test]
fn tick_descends_the_active_piece_by_one_row() {
    let before = state(&[PieceKind::O]);

    let after = transition(&before, TetrisCommand::Tick);

    assert_eq!(after.active_piece().unwrap().row(), 1);
}

#[test]
fn blocked_soft_drop_locks_the_piece_and_spawns_the_next_one() {
    let state = apply(
        state(&[PieceKind::O, PieceKind::I]),
        TetrisCommand::SoftDrop,
        18,
    );
    assert_eq!(state.active_piece().unwrap().row(), 18);

    let state = transition(&state, TetrisCommand::SoftDrop);

    assert_eq!(state.active_piece().unwrap().kind(), PieceKind::I);
    assert_eq!(state.locked_cell(4, 18), Some(PieceKind::O));
    assert_eq!(state.locked_cell(5, 19), Some(PieceKind::O));
}

#[test]
fn hard_drop_locks_at_the_lowest_available_row() {
    let state = transition(
        &state(&[PieceKind::O, PieceKind::T]),
        TetrisCommand::HardDrop,
    );

    assert_eq!(state.active_piece().unwrap().kind(), PieceKind::T);
    assert_eq!(state.locked_cell(4, 18), Some(PieceKind::O));
    assert_eq!(state.locked_cell(5, 19), Some(PieceKind::O));
}

#[test]
fn ghost_piece_projects_to_the_empty_board_floor_without_mutating_state() {
    let state = state(&[PieceKind::O]);
    let before = state.clone();

    let ghost = ghost_piece(&state).unwrap();

    assert_eq!(ghost.kind(), PieceKind::O);
    assert_eq!(ghost.col(), 4);
    assert_eq!(ghost.row(), 18);
    assert_eq!(state, before);
}

#[test]
fn ghost_piece_stops_on_top_of_locked_cells() {
    let state = transition(&state(&[PieceKind::O]), TetrisCommand::HardDrop);

    let ghost = ghost_piece(&state).unwrap();

    assert_eq!(ghost.col(), 4);
    assert_eq!(ghost.row(), 16);
}

#[test]
fn ghost_piece_respects_the_board_edge_after_horizontal_movement() {
    let state = apply(state(&[PieceKind::I]), TetrisCommand::MoveLeft, 4);

    let ghost = ghost_piece(&state).unwrap();

    assert_eq!(ghost.col(), 0);
    assert_eq!(ghost.row(), 19);
}

#[test]
fn ghost_piece_uses_the_active_piece_rotation() {
    let state = transition(&state(&[PieceKind::I]), TetrisCommand::RotateClockwise);

    let ghost = ghost_piece(&state).unwrap();

    assert_eq!(ghost.rotation(), Rotation::Right);
    assert_eq!(ghost.col(), 3);
    assert_eq!(ghost.row(), 16);
}

#[test]
fn ghost_piece_is_hidden_when_the_active_piece_is_already_landed() {
    let state = apply(state(&[PieceKind::O]), TetrisCommand::SoftDrop, 18);

    assert_eq!(state.active_piece().unwrap().row(), 18);
    assert_eq!(ghost_piece(&state), None);
}

#[test]
fn ghost_piece_is_hidden_after_game_over() {
    let game_over = apply(state(&[PieceKind::O]), TetrisCommand::HardDrop, 10);

    assert!(game_over.is_game_over());
    assert_eq!(ghost_piece(&game_over), None);
}

#[test]
fn hard_drop_locks_exactly_the_cells_shown_by_the_ghost_piece() {
    let before = transition(&state(&[PieceKind::T]), TetrisCommand::RotateClockwise);
    let ghost_cells = projected_cells(&paint(&before), |cell| {
        matches!(cell, TetrisCell::Ghost(PieceKind::T))
    });

    let after = transition(&before, TetrisCommand::HardDrop);
    let locked_cells = projected_cells(&paint(&after), |cell| {
        matches!(cell, TetrisCell::Locked(PieceKind::T))
    });

    assert_eq!(locked_cells, ghost_cells);
}

#[test]
fn locked_cell_returns_none_outside_the_board() {
    let state = state(&[PieceKind::O]);

    assert_eq!(state.locked_cell(BOARD_WIDTH, 0), None);
    assert_eq!(state.locked_cell(0, BOARD_HEIGHT), None);
}

#[test]
fn locking_clears_all_completed_rows() {
    let mut state = state(&[PieceKind::O]);
    for target_col in [0, 2, 4, 6, 8] {
        while state.active_piece().unwrap().col() > target_col {
            state = transition(&state, TetrisCommand::MoveLeft);
        }
        while state.active_piece().unwrap().col() < target_col {
            state = transition(&state, TetrisCommand::MoveRight);
        }
        state = transition(&state, TetrisCommand::HardDrop);
    }

    assert_eq!(state.cleared_lines(), 2);
    assert!((0..BOARD_WIDTH).all(|col| state.locked_cell(col, BOARD_HEIGHT - 1).is_none()));
}

#[test]
fn spawn_collision_ends_the_game() {
    let state = apply(state(&[PieceKind::O]), TetrisCommand::HardDrop, 10);

    assert!(state.is_game_over());
    assert_eq!(state.active_piece(), None);
}

#[test]
fn restart_recovers_from_game_over() {
    let game_over = apply(state(&[PieceKind::O]), TetrisCommand::HardDrop, 10);

    let restarted = transition(&game_over, TetrisCommand::Restart);

    assert!(!restarted.is_game_over());
    assert_eq!(restarted.active_piece().unwrap().kind(), PieceKind::O);
}

#[test]
fn game_over_ignores_every_command_except_restart() {
    let game_over = apply(state(&[PieceKind::O]), TetrisCommand::HardDrop, 10);

    for command in [
        TetrisCommand::MoveLeft,
        TetrisCommand::MoveRight,
        TetrisCommand::RotateClockwise,
        TetrisCommand::SoftDrop,
        TetrisCommand::HardDrop,
        TetrisCommand::Tick,
    ] {
        assert_eq!(transition(&game_over, command), game_over);
    }
}

#[test]
fn restart_clears_the_board_and_resets_the_piece_sequence() {
    let state = transition(
        &state(&[PieceKind::O, PieceKind::I]),
        TetrisCommand::HardDrop,
    );

    let restarted = transition(&state, TetrisCommand::Restart);

    assert_eq!(restarted.active_piece().unwrap().kind(), PieceKind::O);
    assert_eq!(restarted.cleared_lines(), 0);
    assert!(!restarted.is_game_over());
    assert!(
        (0..BOARD_HEIGHT)
            .flat_map(|row| (0..BOARD_WIDTH).map(move |col| (col, row)))
            .all(|(col, row)| restarted.locked_cell(col, row).is_none())
    );
}

#[test]
fn paint_returns_a_bordered_punctum_surface() {
    let surface = paint(&state(&[PieceKind::T]));

    assert_eq!(surface.size(), GridSize::new(12, 22));
    assert_eq!(surface.get(GridPos::new(0, 0)), Ok(&TetrisCell::Border));
    assert_eq!(surface.get(GridPos::new(11, 21)), Ok(&TetrisCell::Border));
    assert_eq!(surface.get(GridPos::new(1, 20)), Ok(&TetrisCell::Empty));
    assert_eq!(
        surface.get(GridPos::new(4, 1)),
        Ok(&TetrisCell::Active(PieceKind::T))
    );
    assert_eq!(
        surface.get(GridPos::new(5, 20)),
        Ok(&TetrisCell::Ghost(PieceKind::T))
    );
}

#[test]
fn paint_contains_locked_and_active_pieces() {
    let state = transition(
        &state(&[PieceKind::O, PieceKind::I]),
        TetrisCommand::HardDrop,
    );
    let surface = paint(&state);

    assert_eq!(
        surface.get(GridPos::new(5, 19)),
        Ok(&TetrisCell::Locked(PieceKind::O))
    );
    assert_eq!(
        surface.get(GridPos::new(4, 1)),
        Ok(&TetrisCell::Active(PieceKind::I))
    );
}

#[test]
fn paint_handles_a_game_over_state_without_an_active_piece() {
    let game_over = apply(state(&[PieceKind::O]), TetrisCommand::HardDrop, 10);

    let surface = paint(&game_over);

    assert_eq!(surface.size(), GridSize::new(12, 22));
    assert_eq!(
        surface.get(GridPos::new(5, 1)),
        Ok(&TetrisCell::Locked(PieceKind::O))
    );
}

fn projected_cells(
    surface: &punctum_grid::Surface<TetrisCell>,
    predicate: impl Fn(TetrisCell) -> bool,
) -> Vec<GridPos> {
    (0..surface.size().rows)
        .flat_map(|row| {
            (0..surface.size().cols).map(move |col| GridPos::new(col as i32, row as i32))
        })
        .filter(|&position| predicate(*surface.get(position).unwrap()))
        .collect()
}

#[test]
fn arrow_press_and_repeat_map_to_movement_commands() {
    let cases = [
        (NamedKey::ArrowLeft, TetrisCommand::MoveLeft),
        (NamedKey::ArrowRight, TetrisCommand::MoveRight),
        (NamedKey::ArrowDown, TetrisCommand::SoftDrop),
    ];

    for (logical, command) in cases {
        for phase in [KeyPhase::Press, KeyPhase::Repeat] {
            assert_eq!(
                command_for_key(&key(None, LogicalKey::Named(logical), phase)),
                Some(command)
            );
        }
    }
}

#[test]
fn up_and_space_press_map_to_rotation_and_hard_drop() {
    assert_eq!(
        command_for_key(&key(
            None,
            LogicalKey::Named(NamedKey::ArrowUp),
            KeyPhase::Press,
        )),
        Some(TetrisCommand::RotateClockwise)
    );
    assert_eq!(
        command_for_key(&key(
            Some(PhysicalKeyCode::Space),
            LogicalKey::Named(NamedKey::Space),
            KeyPhase::Press,
        )),
        Some(TetrisCommand::HardDrop)
    );
}

#[test]
fn physical_r_press_maps_to_restart_independent_of_layout() {
    assert_eq!(
        command_for_key(&key(
            Some(PhysicalKeyCode::KeyR),
            LogicalKey::Character("к".into()),
            KeyPhase::Press,
        )),
        Some(TetrisCommand::Restart)
    );
}

#[test]
fn logical_r_press_maps_to_restart_when_physical_identity_is_unavailable() {
    assert_eq!(
        command_for_key(&key(
            None,
            LogicalKey::Character("R".into()),
            KeyPhase::Press,
        )),
        Some(TetrisCommand::Restart)
    );
}

#[test]
fn release_repeat_and_unrelated_keys_do_not_create_commands() {
    let events = [
        key(
            None,
            LogicalKey::Named(NamedKey::ArrowLeft),
            KeyPhase::Release,
        ),
        key(None, LogicalKey::Named(NamedKey::ArrowUp), KeyPhase::Repeat),
        key(
            Some(PhysicalKeyCode::Space),
            LogicalKey::Named(NamedKey::Space),
            KeyPhase::Repeat,
        ),
        key(
            Some(PhysicalKeyCode::KeyA),
            LogicalKey::Character("a".into()),
            KeyPhase::Press,
        ),
    ];

    assert!(events.iter().all(|event| command_for_key(event).is_none()));
}
