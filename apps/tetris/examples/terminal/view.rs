use punctum_grid::Surface;
use punctum_input::{KeyEvent, KeyPhase, LogicalKey, NamedKey};
use punctum_terminal::{TerminalCell, TerminalColor};
use punctum_tetris::{PieceKind, TetrisCell, TetrisState, paint};

pub(crate) fn terminal_surface(state: &TetrisState) -> Surface<TerminalCell> {
    let logical = paint(state);
    let cells = logical
        .cells()
        .iter()
        .copied()
        .map(|cell| terminal_cell(cell, state.is_game_over()))
        .collect();

    Surface::from_cells(logical.size(), cells).expect("mapping a surface preserves its dimensions")
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

        assert_eq!(surface.size(), GridSize::new(12, 22));
        assert_eq!(
            surface.get(GridPos::new(0, 0)).unwrap().background(),
            TerminalColor::Gray
        );
        assert_eq!(
            surface.get(GridPos::new(4, 1)).unwrap().background(),
            TerminalColor::Magenta
        );
        assert_eq!(
            surface.get(GridPos::new(1, 20)).unwrap().background(),
            TerminalColor::Black
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
