#![cfg_attr(not(test), allow(dead_code))]

#[path = "gpu/ramus_palette.rs"]
mod ramus_palette;

fn main() {}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::{Arc, Mutex};

    use punctum_tetris::TetrisCommand;

    use super::ramus_palette::{
        CommandQueue, DiagnosticStage, PaletteIntent, PaletteOutcome, PaletteState, RamusPalette,
    };

    const AUTHORIZED: [&str; 6] = [
        "/tetris/game restart",
        "/tetris/piece hard-drop",
        "/tetris/piece left",
        "/tetris/piece right",
        "/tetris/piece rotate",
        "/tetris/piece soft-drop",
    ];

    fn fixture() -> (CommandQueue, RamusPalette, PaletteState) {
        let queue = Arc::new(Mutex::new(VecDeque::new()));
        let palette = RamusPalette::new(Arc::clone(&queue));
        (queue, palette, PaletteState::default())
    }

    #[test]
    fn local_player_discovers_and_completes_only_the_six_authorized_commands() {
        let (_, palette, _) = fixture();

        assert_eq!(palette.discover_invocations(), AUTHORIZED);
        assert_eq!(palette.complete_invocations(""), AUTHORIZED);
        assert!(
            !palette
                .discover_invocations()
                .contains(&"/developer/tetris inspect".to_owned())
        );
        assert!(palette.complete_invocations("/developer").is_empty());
    }

    #[test]
    fn fuzzy_filter_handles_empty_query_matches_and_zero_results_stably() {
        let (_, palette, mut state) = fixture();

        assert_eq!(
            palette.handle(&mut state, PaletteIntent::Open),
            PaletteOutcome::Updated
        );
        assert_eq!(state.items(), AUTHORIZED);
        assert_eq!(state.selected_index(), Some(0));
        assert_eq!(state.query(), "");

        palette.handle(&mut state, PaletteIntent::InsertText("hard".into()));
        assert_eq!(state.items(), ["/tetris/piece hard-drop"]);
        assert_eq!(state.selected_index(), Some(0));
        assert_eq!(state.query(), "hard");

        palette.handle(&mut state, PaletteIntent::InsertText("zzz".into()));
        assert!(state.items().is_empty());
        assert_eq!(state.selected_index(), None);
        assert_eq!(
            palette.handle(&mut state, PaletteIntent::Execute),
            PaletteOutcome::NoSelection
        );
        let diagnostic = state.diagnostic().expect("missing selection diagnostic");
        assert_eq!(diagnostic.stage, DiagnosticStage::Selection);
        assert_eq!(diagnostic.code, "no-selection");
    }

    #[test]
    fn navigation_wraps_and_query_changes_keep_selection_valid() {
        let (_, palette, mut state) = fixture();
        palette.handle(&mut state, PaletteIntent::Open);

        palette.handle(&mut state, PaletteIntent::Previous);
        assert_eq!(state.selected_index(), Some(AUTHORIZED.len() - 1));
        palette.handle(&mut state, PaletteIntent::Next);
        assert_eq!(state.selected_index(), Some(0));

        palette.handle(&mut state, PaletteIntent::Next);
        palette.handle(&mut state, PaletteIntent::InsertText("right".into()));
        assert_eq!(state.items(), ["/tetris/piece right"]);
        assert_eq!(state.selected_index(), Some(0));
        palette.handle(&mut state, PaletteIntent::Backspace);
        assert!(state.selected_index().is_some());
    }

    #[test]
    fn every_authorized_invocation_enqueues_exactly_one_tetris_command() {
        let (queue, palette, _) = fixture();
        let expected = [
            TetrisCommand::Restart,
            TetrisCommand::HardDrop,
            TetrisCommand::MoveLeft,
            TetrisCommand::MoveRight,
            TetrisCommand::RotateClockwise,
            TetrisCommand::SoftDrop,
        ];

        for invocation in AUTHORIZED {
            palette
                .execute_invocation(invocation)
                .expect("authorized invocation should execute");
        }

        let commands = queue.lock().unwrap().iter().copied().collect::<Vec<_>>();
        assert_eq!(commands, expected);
    }

    #[test]
    fn selected_execution_uses_ramus_and_closes_after_one_queue_write() {
        let (queue, palette, mut state) = fixture();
        palette.handle(&mut state, PaletteIntent::Open);
        palette.handle(&mut state, PaletteIntent::InsertText("soft-drop".into()));

        assert_eq!(
            palette.handle(&mut state, PaletteIntent::Execute),
            PaletteOutcome::Executed
        );
        assert!(!state.is_open());
        assert_eq!(
            queue.lock().unwrap().iter().copied().collect::<Vec<_>>(),
            [TetrisCommand::SoftDrop]
        );
    }

    #[test]
    fn parse_seal_and_provider_failures_are_structured_and_do_not_write_extra_commands() {
        let (queue, palette, _) = fixture();

        let parse = palette.execute_invocation("").unwrap_err();
        assert_eq!(parse.stage, DiagnosticStage::Parse);
        assert_eq!(parse.code, "empty-input");

        let seal = palette
            .execute_invocation("/developer/tetris inspect")
            .unwrap_err();
        assert_eq!(seal.stage, DiagnosticStage::Seal);
        assert_eq!(seal.code, "operation-unavailable");
        assert!(queue.lock().unwrap().is_empty());

        let poisoned_queue = Arc::clone(&queue);
        let _ = catch_unwind(AssertUnwindSafe(move || {
            let _guard = poisoned_queue.lock().unwrap();
            panic!("poison queue for provider failure fixture");
        }));
        let provider = palette
            .execute_invocation("/tetris/piece left")
            .unwrap_err();
        assert_eq!(provider.stage, DiagnosticStage::Provider);
        assert_eq!(provider.code, "command-queue-unavailable");
    }

    #[test]
    fn close_and_closed_state_intents_are_explicit() {
        let (_, palette, mut state) = fixture();

        assert_eq!(
            palette.handle(&mut state, PaletteIntent::Next),
            PaletteOutcome::Ignored
        );
        palette.handle(&mut state, PaletteIntent::Open);
        assert_eq!(
            palette.handle(&mut state, PaletteIntent::Close),
            PaletteOutcome::Closed
        );
        assert!(!state.is_open());
    }
}
