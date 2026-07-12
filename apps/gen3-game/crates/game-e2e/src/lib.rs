//! Cross-layer stories for the playable creature-battle slice.

#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    use battle_application::{Action, BattlePhase};
    use game_host::DemoBattle;

    #[test]
    fn keyboard_battle_slice_can_reach_a_deterministic_finish() {
        let mut battle = DemoBattle::new().unwrap();
        let opening = battle.observation();
        let mut submitted_actions = 0_usize;

        while !matches!(battle.observation().phase(), BattlePhase::Finished(_)) {
            let actions = battle.legal_actions();
            let action = actions
                .iter()
                .copied()
                .find(|action| matches!(action, Action::UseMove(_)))
                .or_else(|| actions.first().copied())
                .expect("an unfinished battle always offers a legal player action");
            battle.submit_player(action).unwrap();
            submitted_actions += 1;
            assert!(submitted_actions < 500, "the demo battle must converge");
        }

        let finished = battle.observation();
        assert!(finished.turn() > opening.turn());
        assert!(submitted_actions > 1);
        assert!(matches!(finished.phase(), BattlePhase::Finished(_)));
    }
}
