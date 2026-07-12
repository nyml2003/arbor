//! Cross-layer stories for the playable creature-battle slice.

#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    use battle_application::{Action, BattlePhase};
    use game_host::{DemoBattle, DemoGame, GameScene};
    use punctum_input::{KeyEvent, KeyPhase, LogicalKey, Modifiers, NamedKey, PhysicalKeyCode};
    use world_application::Position;

    fn key(name: NamedKey) -> KeyEvent {
        KeyEvent {
            physical: Some(PhysicalKeyCode::Unidentified),
            logical: LogicalKey::Named(name),
            modifiers: Modifiers::default(),
            phase: KeyPhase::Press,
        }
    }

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

    #[test]
    fn keyboard_world_slice_enters_battle_and_returns_to_the_same_map_position() {
        let mut game = DemoGame::new().unwrap();
        let right = key(NamedKey::ArrowRight);
        let enter = key(NamedKey::Enter);

        for _ in 0..3 {
            game.handle_key(&right).unwrap();
        }
        assert_eq!(game.scene(), GameScene::Battle);
        assert_eq!(game.world_observation().player(), Position::new(6, 6));

        let mut commands = 0;
        while game.scene() == GameScene::Battle {
            while game.has_pending_playback() {
                game.advance_playback();
            }
            game.handle_key(&enter).unwrap();
            commands += 1;
            assert!(commands < 500, "the keyboard game story must converge");
        }

        assert_eq!(game.scene(), GameScene::World);
        assert_eq!(game.world_observation().player(), Position::new(6, 6));
    }
}
