use punctum_grid::{GridPos, GridSize, Surface};
use punctum_input::{KeyEvent, KeyPhase, LogicalKey, NamedKey, TextEvent};
use punctum_terminal::{TerminalCell, TerminalColor, write_text};
use punctum_tetris::{PieceKind, TetrisCell, TetrisState, paint};

const TERMINAL_COLS: u32 = 46;
const BOARD_CELL_WIDTH: u32 = 2;
const INFO_COL: i32 = 26;

pub(crate) fn terminal_surface(state: &TetrisState) -> Surface<TerminalCell> {
    let logical = paint(state);
    let mut surface = Surface::filled(
        GridSize::new(TERMINAL_COLS, logical.size().rows),
        TerminalCell::new(' ', TerminalColor::White, TerminalColor::Black),
    )
    .expect("the terminal surface has fixed dimensions");

    for row in 0..logical.size().rows {
        for col in 0..logical.size().cols {
            let cell = terminal_cell(
                *logical
                    .get(GridPos::new(col as i32, row as i32))
                    .expect("painted board coordinates are in bounds"),
                state.is_game_over(),
            );
            let terminal_col = col * BOARD_CELL_WIDTH;
            surface
                .set(GridPos::new(terminal_col as i32, row as i32), cell.clone())
                .expect("expanded board coordinates are in bounds");
            surface
                .set(GridPos::new(terminal_col as i32 + 1, row as i32), cell)
                .expect("expanded board coordinates are in bounds");
        }
    }

    write_label(
        &mut surface,
        1,
        "Punctum 方块 e\u{301} 🎮",
        TerminalColor::Cyan,
    );
    write_label(
        &mut surface,
        3,
        &format!("Lines {}", state.cleared_lines()),
        TerminalColor::White,
    );
    write_label(
        &mut surface,
        5,
        if state.is_game_over() {
            "Game over"
        } else {
            "Playing"
        },
        if state.is_game_over() {
            TerminalColor::Red
        } else {
            TerminalColor::Green
        },
    );

    surface
}

fn write_label(
    surface: &mut Surface<TerminalCell>,
    row: i32,
    text: &str,
    foreground: TerminalColor,
) {
    let event = TextEvent::new(text).expect("terminal labels are non-empty");
    write_text(
        surface,
        GridPos::new(INFO_COL, row),
        &event,
        foreground,
        TerminalColor::Black,
    )
    .expect("terminal labels fit inside the fixed information area");
}

fn terminal_cell(cell: TetrisCell, game_over: bool) -> TerminalCell {
    let color = match cell {
        TetrisCell::Empty => TerminalColor::Black,
        TetrisCell::Border if game_over => TerminalColor::Red,
        TetrisCell::Border => TerminalColor::Gray,
        TetrisCell::Tetromino(kind) => piece_color(kind),
    };
    TerminalCell::new(' ', color, color)
}

const fn piece_color(kind: PieceKind) -> TerminalColor {
    match kind {
        PieceKind::I => TerminalColor::Cyan,
        PieceKind::O => TerminalColor::Yellow,
        PieceKind::T => TerminalColor::Magenta,
        PieceKind::S => TerminalColor::Green,
        PieceKind::Z => TerminalColor::Red,
        PieceKind::J => TerminalColor::Blue,
        PieceKind::L => TerminalColor::Rgb {
            red: 255,
            green: 145,
            blue: 48,
        },
    }
}

pub(crate) fn should_quit(event: &KeyEvent) -> bool {
    event.phase == KeyPhase::Press
        && match &event.logical {
            LogicalKey::Named(NamedKey::Escape) => true,
            LogicalKey::Character(character) => character.eq_ignore_ascii_case("q"),
            _ => false,
        }
}

#[cfg(test)]
mod tests {
    use punctum_grid::{GridPos, GridSize};
    use punctum_input::{Modifiers, PhysicalKeyCode};
    use punctum_tetris::{TetrisCommand, transition};

    use super::*;

    fn key(logical: LogicalKey, phase: KeyPhase) -> KeyEvent {
        KeyEvent {
            physical: None,
            logical,
            modifiers: Modifiers::default(),
            phase,
        }
    }

    #[test]
    fn terminal_surface_uses_punctum_dimensions_and_piece_palette() {
        let state = TetrisState::new(vec![PieceKind::T]).unwrap();

        let surface = terminal_surface(&state);

        assert_eq!(surface.size(), GridSize::new(46, 22));
        assert_eq!(
            surface.get(GridPos::new(0, 0)).unwrap().background(),
            TerminalColor::Gray
        );
        assert_eq!(
            surface.get(GridPos::new(1, 0)).unwrap().background(),
            TerminalColor::Gray
        );
        assert_eq!(
            surface.get(GridPos::new(8, 1)).unwrap().background(),
            TerminalColor::Magenta
        );
        assert_eq!(
            surface.get(GridPos::new(9, 1)).unwrap().background(),
            TerminalColor::Magenta
        );
        assert_eq!(
            surface.get(GridPos::new(2, 20)).unwrap().background(),
            TerminalColor::Black
        );
    }

    #[test]
    fn terminal_surface_shows_unicode_title_and_state_text() {
        let state = TetrisState::new(vec![PieceKind::T]).unwrap();

        let surface = terminal_surface(&state);

        assert_eq!(
            surface.get(GridPos::new(34, 1)).unwrap().grapheme(),
            Some("方")
        );
        assert!(surface.get(GridPos::new(35, 1)).unwrap().is_continuation());
        assert_eq!(
            surface.get(GridPos::new(39, 1)).unwrap().grapheme(),
            Some("e\u{301}")
        );
        assert_eq!(
            surface.get(GridPos::new(41, 1)).unwrap().grapheme(),
            Some("🎮")
        );
        assert!(surface.get(GridPos::new(42, 1)).unwrap().is_continuation());
        assert_eq!(
            surface.get(GridPos::new(26, 3)).unwrap().grapheme(),
            Some("L")
        );
        assert_eq!(
            surface.get(GridPos::new(26, 5)).unwrap().grapheme(),
            Some("P")
        );
    }

    #[test]
    fn game_over_surface_marks_the_border_red() {
        let mut state = TetrisState::new(vec![PieceKind::O]).unwrap();
        for _ in 0..10 {
            state = transition(&state, TetrisCommand::HardDrop);
        }

        let surface = terminal_surface(&state);

        assert_eq!(
            surface.get(GridPos::new(0, 0)).unwrap().background(),
            TerminalColor::Red
        );
    }

    #[test]
    fn every_piece_has_a_terminal_color() {
        let colors = PieceKind::ALL.map(piece_color);

        assert_eq!(colors[0], TerminalColor::Cyan);
        assert_eq!(colors[1], TerminalColor::Yellow);
        assert_eq!(colors[2], TerminalColor::Magenta);
        assert_eq!(colors[3], TerminalColor::Green);
        assert_eq!(colors[4], TerminalColor::Red);
        assert_eq!(colors[5], TerminalColor::Blue);
        assert_eq!(
            colors[6],
            TerminalColor::Rgb {
                red: 255,
                green: 145,
                blue: 48,
            }
        );
    }

    #[test]
    fn escape_and_q_press_quit_the_host() {
        assert!(should_quit(&key(
            LogicalKey::Named(NamedKey::Escape),
            KeyPhase::Press,
        )));
        assert!(should_quit(&key(
            LogicalKey::Character("Q".into()),
            KeyPhase::Press,
        )));
    }

    #[test]
    fn release_and_other_keys_do_not_quit_the_host() {
        assert!(!should_quit(&key(
            LogicalKey::Named(NamedKey::Escape),
            KeyPhase::Release,
        )));
        assert!(!should_quit(&key(
            LogicalKey::Character("r".into()),
            KeyPhase::Press,
        )));
        assert!(!should_quit(&KeyEvent {
            physical: Some(PhysicalKeyCode::ArrowLeft),
            logical: LogicalKey::Named(NamedKey::ArrowLeft),
            modifiers: Modifiers::default(),
            phase: KeyPhase::Press,
        }));
    }
}
