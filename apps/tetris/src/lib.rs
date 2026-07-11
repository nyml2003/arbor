//! Pure Tetris rules and Punctum projections.

#![forbid(unsafe_code)]

use std::{error::Error, fmt};

use punctum_grid::{GridSize, Surface};
use punctum_input::{KeyEvent, KeyPhase, LogicalKey, NamedKey, PhysicalKeyCode};

pub const BOARD_WIDTH: u32 = 10;
pub const BOARD_HEIGHT: u32 = 20;
pub const SURFACE_WIDTH: u32 = BOARD_WIDTH + 2;
pub const SURFACE_HEIGHT: u32 = BOARD_HEIGHT + 2;

const BOARD_CELL_COUNT: usize = BOARD_WIDTH as usize * BOARD_HEIGHT as usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PieceKind {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

impl PieceKind {
    pub const ALL: [Self; 7] = [
        Self::I,
        Self::O,
        Self::T,
        Self::S,
        Self::Z,
        Self::J,
        Self::L,
    ];

    const fn index(self) -> usize {
        match self {
            Self::I => 0,
            Self::O => 1,
            Self::T => 2,
            Self::S => 3,
            Self::Z => 4,
            Self::J => 5,
            Self::L => 6,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Rotation {
    #[default]
    Spawn,
    Right,
    Reverse,
    Left,
}

impl Rotation {
    const fn clockwise(self) -> Self {
        match self {
            Self::Spawn => Self::Right,
            Self::Right => Self::Reverse,
            Self::Reverse => Self::Left,
            Self::Left => Self::Spawn,
        }
    }

    const fn index(self) -> usize {
        match self {
            Self::Spawn => 0,
            Self::Right => 1,
            Self::Reverse => 2,
            Self::Left => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TetrisCommand {
    MoveLeft,
    MoveRight,
    RotateClockwise,
    SoftDrop,
    HardDrop,
    Tick,
    Restart,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TetrisCell {
    Empty,
    Border,
    Tetromino(PieceKind),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ActivePiece {
    kind: PieceKind,
    rotation: Rotation,
    col: i32,
    row: i32,
}

impl ActivePiece {
    pub const fn kind(self) -> PieceKind {
        self.kind
    }

    pub const fn rotation(self) -> Rotation {
        self.rotation
    }

    pub const fn col(self) -> i32 {
        self.col
    }

    pub const fn row(self) -> i32 {
        self.row
    }

    fn translated(self, col_delta: i32, row_delta: i32) -> Self {
        Self {
            col: self.col + col_delta,
            row: self.row + row_delta,
            ..self
        }
    }

    fn rotated_clockwise(self) -> Self {
        Self {
            rotation: self.rotation.clockwise(),
            ..self
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TetrisState {
    board: [Option<PieceKind>; BOARD_CELL_COUNT],
    active: Option<ActivePiece>,
    sequence: Vec<PieceKind>,
    sequence_cursor: usize,
    cleared_lines: u32,
    game_over: bool,
}

impl TetrisState {
    pub fn new(sequence: Vec<PieceKind>) -> Result<Self, TetrisError> {
        if sequence.is_empty() {
            return Err(TetrisError::EmptyPieceSequence);
        }

        Ok(Self::fresh(sequence))
    }

    pub const fn active_piece(&self) -> Option<ActivePiece> {
        self.active
    }

    pub const fn cleared_lines(&self) -> u32 {
        self.cleared_lines
    }

    pub const fn is_game_over(&self) -> bool {
        self.game_over
    }

    pub fn locked_cell(&self, col: u32, row: u32) -> Option<PieceKind> {
        if col >= BOARD_WIDTH || row >= BOARD_HEIGHT {
            return None;
        }

        self.board[board_index(col, row)]
    }

    fn fresh(sequence: Vec<PieceKind>) -> Self {
        let mut state = Self {
            board: [None; BOARD_CELL_COUNT],
            active: None,
            sequence,
            sequence_cursor: 0,
            cleared_lines: 0,
            game_over: false,
        };
        state.spawn_next();
        state
    }

    fn restart(&self) -> Self {
        Self::fresh(self.sequence.clone())
    }

    fn spawn_next(&mut self) {
        let kind = self.sequence[self.sequence_cursor];
        self.sequence_cursor = (self.sequence_cursor + 1) % self.sequence.len();
        let piece = ActivePiece {
            kind,
            rotation: Rotation::Spawn,
            col: (BOARD_WIDTH as i32 - piece_width(kind, Rotation::Spawn)) / 2,
            row: 0,
        };

        if self.can_place(piece) {
            self.active = Some(piece);
        } else {
            self.active = None;
            self.game_over = true;
        }
    }

    fn try_replace_active(&mut self, candidate: ActivePiece) {
        if self.can_place(candidate) {
            self.active = Some(candidate);
        }
    }

    fn descend_or_lock(&mut self) {
        let active = self
            .active
            .expect("a running game always has an active piece");
        let descended = active.translated(0, 1);
        if self.can_place(descended) {
            self.active = Some(descended);
        } else {
            self.lock_active();
        }
    }

    fn hard_drop(&mut self) {
        let mut dropped = self
            .active
            .expect("a running game always has an active piece");
        while self.can_place(dropped.translated(0, 1)) {
            dropped = dropped.translated(0, 1);
        }
        self.active = Some(dropped);
        self.lock_active();
    }

    fn lock_active(&mut self) {
        let active = self.active.take().expect("only an active piece can lock");
        for (col, row) in occupied_cells(active) {
            self.board[board_index(col as u32, row as u32)] = Some(active.kind);
        }
        self.clear_full_rows();
        self.spawn_next();
    }

    fn clear_full_rows(&mut self) {
        let mut compacted = [None; BOARD_CELL_COUNT];
        let mut target_row = BOARD_HEIGHT as i32 - 1;
        let mut cleared = 0_u32;

        for source_row in (0..BOARD_HEIGHT).rev() {
            let is_full =
                (0..BOARD_WIDTH).all(|col| self.board[board_index(col, source_row)].is_some());
            if is_full {
                cleared += 1;
                continue;
            }

            for col in 0..BOARD_WIDTH {
                compacted[board_index(col, target_row as u32)] =
                    self.board[board_index(col, source_row)];
            }
            target_row -= 1;
        }

        self.board = compacted;
        self.cleared_lines = self.cleared_lines.saturating_add(cleared);
    }

    fn can_place(&self, piece: ActivePiece) -> bool {
        occupied_cells(piece).into_iter().all(|(col, row)| {
            col >= 0
                && col < BOARD_WIDTH as i32
                && row >= 0
                && row < BOARD_HEIGHT as i32
                && self.board[board_index(col as u32, row as u32)].is_none()
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TetrisError {
    EmptyPieceSequence,
}

impl fmt::Display for TetrisError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPieceSequence => formatter.write_str("piece sequence must not be empty"),
        }
    }
}

impl Error for TetrisError {}

pub fn transition(state: &TetrisState, command: TetrisCommand) -> TetrisState {
    if state.game_over && command != TetrisCommand::Restart {
        return state.clone();
    }

    let mut next = state.clone();
    let active = next.active;
    match command {
        TetrisCommand::MoveLeft => {
            next.try_replace_active(
                active
                    .expect("a running game has an active piece")
                    .translated(-1, 0),
            );
        }
        TetrisCommand::MoveRight => {
            next.try_replace_active(
                active
                    .expect("a running game has an active piece")
                    .translated(1, 0),
            );
        }
        TetrisCommand::RotateClockwise => {
            next.try_replace_active(
                active
                    .expect("a running game has an active piece")
                    .rotated_clockwise(),
            );
        }
        TetrisCommand::SoftDrop | TetrisCommand::Tick => next.descend_or_lock(),
        TetrisCommand::HardDrop => next.hard_drop(),
        TetrisCommand::Restart => return next.restart(),
    }
    next
}

pub fn paint(state: &TetrisState) -> Surface<TetrisCell> {
    let mut cells = Vec::with_capacity((SURFACE_WIDTH * SURFACE_HEIGHT) as usize);
    for row in 0..SURFACE_HEIGHT {
        for col in 0..SURFACE_WIDTH {
            let cell =
                if col == 0 || row == 0 || col == SURFACE_WIDTH - 1 || row == SURFACE_HEIGHT - 1 {
                    TetrisCell::Border
                } else {
                    state
                        .locked_cell(col - 1, row - 1)
                        .map_or(TetrisCell::Empty, TetrisCell::Tetromino)
                };
            cells.push(cell);
        }
    }

    if let Some(active) = state.active {
        for (col, row) in occupied_cells(active) {
            let surface_index = ((row as u32 + 1) * SURFACE_WIDTH + col as u32 + 1) as usize;
            cells[surface_index] = TetrisCell::Tetromino(active.kind);
        }
    }

    Surface::from_cells(GridSize::new(SURFACE_WIDTH, SURFACE_HEIGHT), cells)
        .expect("fixed Tetris surface dimensions are valid")
}

pub fn command_for_key(event: &KeyEvent) -> Option<TetrisCommand> {
    let is_physical_restart = event.physical == Some(PhysicalKeyCode::KeyR);
    let is_logical_restart = event.physical.is_none()
        && matches!(
            &event.logical,
            LogicalKey::Character(character) if character.eq_ignore_ascii_case("r")
        );
    if event.phase == KeyPhase::Press && (is_physical_restart || is_logical_restart) {
        return Some(TetrisCommand::Restart);
    }

    match (&event.logical, event.phase) {
        (LogicalKey::Named(NamedKey::ArrowLeft), KeyPhase::Press | KeyPhase::Repeat) => {
            Some(TetrisCommand::MoveLeft)
        }
        (LogicalKey::Named(NamedKey::ArrowRight), KeyPhase::Press | KeyPhase::Repeat) => {
            Some(TetrisCommand::MoveRight)
        }
        (LogicalKey::Named(NamedKey::ArrowDown), KeyPhase::Press | KeyPhase::Repeat) => {
            Some(TetrisCommand::SoftDrop)
        }
        (LogicalKey::Named(NamedKey::ArrowUp), KeyPhase::Press) => {
            Some(TetrisCommand::RotateClockwise)
        }
        (LogicalKey::Named(NamedKey::Space), KeyPhase::Press) => Some(TetrisCommand::HardDrop),
        _ => None,
    }
}

fn board_index(col: u32, row: u32) -> usize {
    row as usize * BOARD_WIDTH as usize + col as usize
}

fn piece_width(kind: PieceKind, rotation: Rotation) -> i32 {
    shape(kind, rotation)
        .into_iter()
        .map(|(col, _)| col + 1)
        .max()
        .expect("tetrominoes always contain four cells")
}

fn occupied_cells(piece: ActivePiece) -> [(i32, i32); 4] {
    shape(piece.kind, piece.rotation).map(|(col, row)| (piece.col + col, piece.row + row))
}

fn shape(kind: PieceKind, rotation: Rotation) -> [(i32, i32); 4] {
    SHAPES[kind.index()][rotation.index()]
}

const SHAPES: [[[(i32, i32); 4]; 4]; 7] = [
    [
        [(0, 0), (1, 0), (2, 0), (3, 0)],
        [(0, 0), (0, 1), (0, 2), (0, 3)],
        [(0, 0), (1, 0), (2, 0), (3, 0)],
        [(0, 0), (0, 1), (0, 2), (0, 3)],
    ],
    [
        [(0, 0), (1, 0), (0, 1), (1, 1)],
        [(0, 0), (1, 0), (0, 1), (1, 1)],
        [(0, 0), (1, 0), (0, 1), (1, 1)],
        [(0, 0), (1, 0), (0, 1), (1, 1)],
    ],
    [
        [(0, 0), (1, 0), (2, 0), (1, 1)],
        [(0, 0), (0, 1), (1, 1), (0, 2)],
        [(1, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (0, 1), (1, 1), (1, 2)],
    ],
    [
        [(1, 0), (2, 0), (0, 1), (1, 1)],
        [(0, 0), (0, 1), (1, 1), (1, 2)],
        [(1, 0), (2, 0), (0, 1), (1, 1)],
        [(0, 0), (0, 1), (1, 1), (1, 2)],
    ],
    [
        [(0, 0), (1, 0), (1, 1), (2, 1)],
        [(1, 0), (0, 1), (1, 1), (0, 2)],
        [(0, 0), (1, 0), (1, 1), (2, 1)],
        [(1, 0), (0, 1), (1, 1), (0, 2)],
    ],
    [
        [(0, 0), (0, 1), (1, 1), (2, 1)],
        [(0, 0), (1, 0), (0, 1), (0, 2)],
        [(0, 0), (1, 0), (2, 0), (2, 1)],
        [(1, 0), (1, 1), (0, 2), (1, 2)],
    ],
    [
        [(2, 0), (0, 1), (1, 1), (2, 1)],
        [(0, 0), (0, 1), (0, 2), (1, 2)],
        [(0, 0), (1, 0), (2, 0), (0, 1)],
        [(0, 0), (1, 0), (1, 1), (1, 2)],
    ],
];
