use std::collections::{HashSet, VecDeque};

use punctum_tetris::{
    ActivePiece, BOARD_HEIGHT, BOARD_WIDTH, TetrisCommand, TetrisState, transition,
};

const POSITIONING_COMMANDS: [(TetrisCommand, &str); 3] = [
    (TetrisCommand::MoveLeft, "/tetris/piece left"),
    (TetrisCommand::MoveRight, "/tetris/piece right"),
    (TetrisCommand::RotateClockwise, "/tetris/piece rotate"),
];
const HARD_DROP: &str = "/tetris/piece hard-drop";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacementCandidate {
    pub invocations: Vec<String>,
    pub description: String,
    pub score: i64,
    pub cleared_lines: u32,
}

pub fn placement_candidates(state: &TetrisState) -> Vec<PlacementCandidate> {
    let Some(active) = state.active_piece() else {
        return Vec::new();
    };
    let mut queue = VecDeque::from([(state.clone(), Vec::<String>::new())]);
    let mut visited = HashSet::from([active]);
    let mut outcomes = HashSet::new();
    let mut candidates = Vec::new();

    while let Some((positioned, path)) = queue.pop_front() {
        let landing = positioned
            .active_piece()
            .expect("positioning states retain the active piece");
        let result = transition(&positioned, TetrisCommand::HardDrop);
        let signature = board_rows(&result).join("/");
        if outcomes.insert(signature) {
            let mut invocations = path.clone();
            invocations.push(HARD_DROP.into());
            let metrics = BoardMetrics::from_state(&result);
            let lines = result.cleared_lines().saturating_sub(state.cleared_lines());
            let score = metrics.score(lines);
            candidates.push(PlacementCandidate {
                description: describe_candidate(
                    &result,
                    landing,
                    &invocations,
                    lines,
                    metrics,
                    score,
                ),
                invocations,
                score,
                cleared_lines: lines,
            });
        }

        for (command, invocation) in POSITIONING_COMMANDS {
            let next = transition(&positioned, command);
            let Some(next_active) = next.active_piece() else {
                continue;
            };
            if next_active == landing || !visited.insert(next_active) {
                continue;
            }
            let mut next_path = path.clone();
            next_path.push(invocation.into());
            queue.push_back((next, next_path));
        }
    }

    candidates.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.invocations.len().cmp(&right.invocations.len()))
            .then_with(|| left.invocations.cmp(&right.invocations))
    });
    candidates
}

pub fn preferred_candidates(state: &TetrisState, limit: usize) -> Vec<PlacementCandidate> {
    let mut candidates = placement_candidates(state);
    let max_cleared_lines = candidates
        .iter()
        .map(|candidate| candidate.cleared_lines)
        .max()
        .unwrap_or(0);
    if max_cleared_lines > 0 {
        candidates.retain(|candidate| candidate.cleared_lines == max_cleared_lines);
    }
    candidates.truncate(limit);
    candidates
}

fn describe_candidate(
    after: &TetrisState,
    landing: ActivePiece,
    invocations: &[String],
    lines: u32,
    metrics: BoardMetrics,
    score: i64,
) -> String {
    format!(
        "actions={invocations:?}; heuristic_score={score}; landing_col={}; rotation={:?}; cleared_lines={lines}; \
         holes={}; aggregate_height={}; max_height={}; bumpiness={}; resulting_board={:?}",
        landing.col(),
        landing.rotation(),
        metrics.holes,
        metrics.aggregate_height,
        metrics.max_height,
        metrics.bumpiness,
        board_rows(after),
    )
}

fn board_rows(state: &TetrisState) -> Vec<String> {
    (0..BOARD_HEIGHT)
        .map(|row| {
            (0..BOARD_WIDTH)
                .map(|col| state.locked_cell(col, row).map_or('.', |_| '#'))
                .collect()
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BoardMetrics {
    aggregate_height: u32,
    max_height: u32,
    holes: u32,
    bumpiness: u32,
}

impl BoardMetrics {
    fn from_state(state: &TetrisState) -> Self {
        let mut heights = [0_u32; BOARD_WIDTH as usize];
        let mut holes = 0_u32;
        for col in 0..BOARD_WIDTH {
            let mut occupied_seen = false;
            for row in 0..BOARD_HEIGHT {
                if state.locked_cell(col, row).is_some() {
                    if !occupied_seen {
                        heights[col as usize] = BOARD_HEIGHT - row;
                        occupied_seen = true;
                    }
                } else if occupied_seen {
                    holes += 1;
                }
            }
        }
        let bumpiness = heights
            .windows(2)
            .map(|pair| pair[0].abs_diff(pair[1]))
            .sum();
        Self {
            aggregate_height: heights.iter().sum(),
            max_height: heights.into_iter().max().unwrap_or(0),
            holes,
            bumpiness,
        }
    }

    fn score(self, cleared_lines: u32) -> i64 {
        i64::from(cleared_lines) * 10_000
            - i64::from(self.holes) * 1_000
            - i64::from(self.aggregate_height) * 45
            - i64::from(self.max_height) * 60
            - i64::from(self.bumpiness) * 30
    }
}

#[cfg(test)]
mod tests {
    use punctum_tetris::{PieceKind, TetrisCommand, TetrisState, transition};

    use super::{HARD_DROP, placement_candidates, preferred_candidates};

    #[test]
    fn candidates_are_reachable_unique_landings_with_stable_metrics() {
        let state = TetrisState::new(vec![PieceKind::I]).unwrap();
        let candidates = placement_candidates(&state);

        assert!(candidates.len() >= 10);
        assert!(candidates.iter().all(|candidate| {
            candidate.invocations.last().map(String::as_str) == Some(HARD_DROP)
                && candidate.description.contains("holes=")
                && candidate.description.contains("heuristic_score=")
                && candidate.description.contains("resulting_board=")
        }));
        assert!(candidates.iter().any(|candidate| {
            candidate
                .invocations
                .iter()
                .any(|invocation| invocation == "/tetris/piece left")
        }));
        assert!(candidates.iter().any(|candidate| {
            candidate
                .invocations
                .iter()
                .any(|invocation| invocation == "/tetris/piece right")
        }));
    }

    #[test]
    fn immediate_line_clears_exclude_non_clearing_candidates() {
        let mut state = TetrisState::new(vec![PieceKind::O]).unwrap();
        for horizontal_moves in [-4_i32, -2, 0, 2] {
            let command = if horizontal_moves < 0 {
                TetrisCommand::MoveLeft
            } else {
                TetrisCommand::MoveRight
            };
            for _ in 0..horizontal_moves.unsigned_abs() {
                state = transition(&state, command);
            }
            state = transition(&state, TetrisCommand::HardDrop);
        }

        let candidates = placement_candidates(&state);
        assert!(
            candidates
                .iter()
                .any(|candidate| candidate.cleared_lines == 2)
        );
        assert!(
            candidates
                .iter()
                .any(|candidate| candidate.cleared_lines == 0)
        );
        let preferred = preferred_candidates(&state, 8);
        assert!(!preferred.is_empty());
        assert!(candidate_all_clear(&preferred, 2));
    }

    fn candidate_all_clear(candidates: &[super::PlacementCandidate], lines: u32) -> bool {
        candidates
            .iter()
            .all(|candidate| candidate.cleared_lines == lines)
    }
}
