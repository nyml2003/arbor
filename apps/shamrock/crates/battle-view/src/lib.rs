mod public;
mod snapshot;
mod text;

pub use public::{ActionView, PublicBattleView, SidePanelView, UiEventLog, build_public_battle_view};
pub use snapshot::{ActionDescriptor, ActionKind, BattleSnapshot, SideSnapshot, ViewerProfile, build_battle_snapshot};
pub use text::BattleViewText;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventTab {
    Domain,
    Trace,
    System,
}

impl EventTab {
    pub fn title<L: BattleViewText>(self, locale: L) -> &'static str {
        locale.event_tab_title(self)
    }
}

#[cfg(test)]
mod tests {
    use battle_core::{BattleAction, BattleInit, Request, SideId, initialize_battle};
    use battle_data::{ElementType, StatusCondition, WeatherKind, load_demo_enemy_team, load_demo_player_team, load_gen1_demo_pack};

    use super::{
        BattleViewText, EventTab, UiEventLog, ViewerProfile, build_battle_snapshot,
        build_public_battle_view,
    };

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum StubLocale {
        En,
    }

    impl BattleViewText for StubLocale {
        fn event_tab_title(self, tab: EventTab) -> &'static str {
            match tab {
                EventTab::Domain => "Events",
                EventTab::Trace => "Trace",
                EventTab::System => "System",
            }
        }
        fn mode_human_vs_ai(self) -> &'static str { "Human vs AI" }
        fn weather_line(self, weather: Option<(WeatherKind, u8)>) -> String {
            match weather {
                Some((WeatherKind::Sunny, turns)) => format!("Weather Sun ({turns})"),
                Some((WeatherKind::Rainy, turns)) => format!("Weather Rain ({turns})"),
                None => "Weather None".to_string(),
            }
        }
        fn side_label(self, side: SideId) -> &'static str {
            match side {
                SideId::Player => "Player",
                SideId::Opponent => "Opponent",
            }
        }
        fn pokemon_type_line(self, primary: ElementType, _secondary: Option<ElementType>) -> String {
            format!("Type {}", self.element_type_name(primary))
        }
        fn hp_line(self, current_hp: i32, max_hp: i32) -> String { format!("HP {current_hp}/{max_hp}") }
        fn status_line(self, status: Option<StatusCondition>) -> String {
            match status {
                Some(StatusCondition::Paralyzed) => "Status PAR".to_string(),
                Some(StatusCondition::Poisoned) => "Status PSN".to_string(),
                None => "Status OK".to_string(),
            }
        }
        fn alive_line(self, alive_count: usize, party_size: usize) -> String {
            format!("Alive {alive_count}/{party_size}")
        }
        fn latest_side_summary(self, side: SideId) -> String { format!("Summary {:?}", side) }
        fn request_label(self, request: Request) -> String { format!("{:?}", request) }
        fn action_kind_move(self) -> &'static str { "Move" }
        fn action_kind_switch(self) -> &'static str { "Switch" }
        fn action_switch(self, nickname: &str) -> String { format!("Switch to {nickname}") }
        fn element_type_name(self, element_type: ElementType) -> &'static str {
            match element_type {
                ElementType::Normal => "Normal",
                ElementType::Electric => "Electric",
                ElementType::Fire => "Fire",
                ElementType::Water => "Water",
                ElementType::Grass => "Grass",
            }
        }
        fn agent_name_line(self, agent_name: &str) -> String { agent_name.to_string() }
        fn fallback_none(self) -> &'static str { "None" }
    }

    #[test]
    fn public_view_builds_stable_action_order() {
        let data = load_gen1_demo_pack();
        let state = initialize_battle(BattleInit { player: load_demo_player_team(), opponent: load_demo_enemy_team() }, &data).unwrap();
        let actions = battle_format::legal_actions(&state, battle_core::SideId::Player);
        let player_team = &state.teams[battle_core::SideId::Player.index()];
        let active = &player_team.party[player_team.active];
        let first_move = data.move_def(active.moves[0]);
        let snapshot = build_battle_snapshot(&state, &data, "demo", 42, &actions, ViewerProfile::LocalPlayer(SideId::Player));
        let view = build_public_battle_view(
            &snapshot,
            StubLocale::En,
            &UiEventLog::default(),
            EventTab::Domain,
            "ready",
        );

        assert_eq!(view.legal_actions[0].token, "M1");
        assert_eq!(view.legal_actions[0].hotkey, "1");
        assert_eq!(view.legal_actions[0].element.as_deref(), Some(StubLocale::En.element_type_name(first_move.element_type)));
        assert_eq!(view.legal_actions[0].power, Some(first_move.power));
        assert!(view.legal_actions.iter().any(|action| action.kind == "Switch"));
        assert_eq!(view.player.primary_type, data.species(active.species).primary_type);
        assert_eq!(view.viewer, ViewerProfile::LocalPlayer(SideId::Player));
    }

    #[test]
    fn snapshot_keeps_structured_action_metadata() {
        let data = load_gen1_demo_pack();
        let state = initialize_battle(BattleInit { player: load_demo_player_team(), opponent: load_demo_enemy_team() }, &data).unwrap();
        let actions = battle_format::legal_actions(&state, battle_core::SideId::Player);
        let snapshot = build_battle_snapshot(&state, &data, "demo", 42, &actions, ViewerProfile::LocalPlayer(SideId::Player));

        assert_eq!(snapshot.legal_actions[0].token, "M1");
        assert!(snapshot.legal_actions[0].move_id.is_some());
        assert_eq!(snapshot.legal_actions[0].switch_slot, None);
        assert!(snapshot.legal_actions.iter().any(|action| action.switch_slot.is_some()));
    }

    #[test]
    fn snapshot_uses_viewer_side_for_action_context() {
        let data = load_gen1_demo_pack();
        let state = initialize_battle(BattleInit { player: load_demo_player_team(), opponent: load_demo_enemy_team() }, &data).unwrap();
        let state = battle_core::step(
            state,
            SideId::Player,
            BattleAction::UseMove(0),
            battle_core::RngState::seeded(5),
            &data,
        )
        .unwrap()
        .state;
        let actions = battle_format::legal_actions(&state, SideId::Opponent);
        let snapshot = build_battle_snapshot(&state, &data, "demo", 42, &actions, ViewerProfile::Agent(SideId::Opponent));

        let opponent_team = &state.teams[SideId::Opponent.index()];
        let opponent_active = &opponent_team.party[opponent_team.active];
        let expected_move_name = data.move_def(opponent_active.moves[0]).name.clone();

        assert_eq!(snapshot.legal_actions[0].name, expected_move_name);
        assert_eq!(snapshot.viewer, ViewerProfile::Agent(SideId::Opponent));
    }

    #[test]
    fn public_view_varies_action_presentation_by_viewer_profile() {
        let data = load_gen1_demo_pack();
        let state = initialize_battle(BattleInit { player: load_demo_player_team(), opponent: load_demo_enemy_team() }, &data).unwrap();
        let actions = battle_format::legal_actions(&state, battle_core::SideId::Player);

        let local_snapshot = build_battle_snapshot(
            &state,
            &data,
            "demo",
            42,
            &actions,
            ViewerProfile::LocalPlayer(SideId::Player),
        );
        let local_view = build_public_battle_view(
            &local_snapshot,
            StubLocale::En,
            &UiEventLog::default(),
            EventTab::Domain,
            "ready",
        );

        let agent_snapshot = build_battle_snapshot(
            &state,
            &data,
            "demo",
            42,
            &actions,
            ViewerProfile::Agent(SideId::Player),
        );
        let agent_view = build_public_battle_view(
            &agent_snapshot,
            StubLocale::En,
            &UiEventLog::default(),
            EventTab::Domain,
            "ready",
        );

        assert_eq!(local_view.legal_actions[0].hotkey, "1");
        assert_eq!(agent_view.legal_actions[0].hotkey, "");
        assert_eq!(local_view.legal_actions[0].kind, "Move");
        assert_eq!(agent_view.legal_actions[0].kind, "move");
        assert!(local_view.legal_actions[0].element.is_some());
        assert!(agent_view.legal_actions[0].element.is_none());
    }

    #[test]
    fn selected_tab_picks_recent_lines() {
        let mut log = UiEventLog::default();
        log.domain.push("MOVE Player used Thunder Shock".to_string());
        log.trace.push("TRACE Damage roll 61".to_string());
        log.system.push("SYSTEM Replay saved".to_string());

        assert_eq!(log.selected_lines(EventTab::Domain, 10), vec!["MOVE Player used Thunder Shock".to_string()]);
        assert_eq!(log.selected_lines(EventTab::Trace, 10), vec!["TRACE Damage roll 61".to_string()]);
    }
}
