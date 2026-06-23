mod ai;
mod demo_loop;
mod i18n;
mod rendering;
mod replay_io;
mod theme;
mod tui;

use std::error::Error;
use std::io::{self, IsTerminal};

use demo_loop::{run_plain_demo, run_tui_demo};
use i18n::Locale;
use replay_io::save_replay;

/**
CLI 入口只负责选择运行方式，然后把控制权交给外层壳层。

当前版本只有一个 demo，所以入口保持得很薄。
等后面扩成真正的命令系统，这里也应该继续只做模式分发，不把业务细节堆在 `main` 里。
*/
fn main() -> Result<(), Box<dyn Error>> {
    run_demo()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UiMode {
    Auto,
    Plain,
    Tui,
}

fn detect_ui_mode() -> UiMode {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--plain") {
        UiMode::Plain
    } else if args.iter().any(|arg| arg == "--tui") {
        UiMode::Tui
    } else {
        match std::env::var("SHAMROCK_UI").ok().as_deref() {
            Some("plain") => UiMode::Plain,
            Some("tui") => UiMode::Tui,
            _ => UiMode::Auto,
        }
    }
}

/**
运行当前 first playable demo。

这里先判断当前进程是否真的连着交互式终端：

- 是终端，就进入 TUI 模式
- 不是终端，就退回纯文本 CLI

这样 demo 既能手动玩，也能在没有 TTY 的环境里继续跑通。
*/
fn run_demo() -> Result<(), Box<dyn Error>> {
    let locale = Locale::detect();
    let result = match detect_ui_mode() {
        UiMode::Plain => run_plain_demo(locale, false)?,
        UiMode::Tui => {
            if io::stdin().is_terminal() && io::stdout().is_terminal() {
                run_tui_demo(locale)?
            } else {
                return Err("TUI 模式需要交互式终端；请改用 --plain 或 SHAMROCK_UI=plain".into());
            }
        }
        UiMode::Auto => {
            if io::stdin().is_terminal() && io::stdout().is_terminal() {
                run_tui_demo(locale)?
            } else {
                run_plain_demo(locale, true)?
            }
        }
    };
    if let Some(result) = result {
        println!("{}", locale.battle_finished_message(result.winner));
        save_replay(&result.record, locale)?;
    } else {
        println!("{}", locale.battle_aborted_message());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use battle_core::{BattleInit, SideId, initialize_battle};
    use battle_replay::BattleRecord;
    use battle_view::UiEventLog;

    use crate::ai::choose_enemy_action;
    use crate::demo_loop::{play_demo_with, resolve_menu_choice};
    use crate::i18n::Locale;
    use crate::rendering::{
        action_menu_lines, append_log_to_ui, battle_status_lines, describe_action,
        render_log_lines, render_trace_lines,
    };
    use crate::replay_io::save_replay_to;
    use crate::UiMode;

    use battle_format::legal_actions;
    use battle_data::{load_demo_enemy_team, load_demo_player_team, load_gen1_demo_pack};

    #[test]
    fn ui_mode_variants_stay_distinct() {
        assert_eq!(UiMode::Auto, UiMode::Auto);
        assert_ne!(UiMode::Plain, UiMode::Tui);
    }

    #[test]
    fn enemy_ai_picks_a_legal_action() {
        let data = load_gen1_demo_pack();
        let state = initialize_battle(
            BattleInit {
                player: load_demo_player_team(),
                opponent: load_demo_enemy_team(),
            },
            &data,
        )
        .unwrap();
        let state = battle_core::step(
            state,
            SideId::Player,
            battle_core::BattleAction::UseMove(0),
            battle_core::RngState::seeded(5),
            &data,
        )
        .unwrap()
        .state;

        let action = choose_enemy_action(&state, &data);
        assert!(legal_actions(&state, SideId::Opponent).contains(&action));
    }

    #[test]
    fn action_description_is_human_readable() {
        let data = load_gen1_demo_pack();
        let state = initialize_battle(
            BattleInit {
                player: load_demo_player_team(),
                opponent: load_demo_enemy_team(),
            },
            &data,
        )
        .unwrap();

        let text = describe_action(
            battle_core::BattleAction::UseMove(0),
            &state,
            SideId::Player,
            &data,
            Locale::EnUs,
        );
        assert!(text.contains("Use"));
    }

    #[test]
    fn battle_status_lines_include_active_names() {
        let data = load_gen1_demo_pack();
        let state = initialize_battle(
            BattleInit {
                player: load_demo_player_team(),
                opponent: load_demo_enemy_team(),
            },
            &data,
        )
        .unwrap();
        let lines = battle_status_lines(&state, &data, Locale::EnUs);
        let player_name = &state.teams[SideId::Player.index()].party
            [state.teams[SideId::Player.index()].active]
            .nickname;
        let opponent_name = &state.teams[SideId::Opponent.index()].party
            [state.teams[SideId::Opponent.index()].active]
            .nickname;

        assert!(lines.iter().any(|line| line.contains(player_name)));
        assert!(lines.iter().any(|line| line.contains(opponent_name)));
        assert!(lines.iter().any(|line| line.contains("Weather") || line.contains("天气")));
        assert!(lines.iter().any(|line| line.contains("Type") || line.contains("属性")));
    }

    #[test]
    fn action_menu_lines_show_type_and_power() {
        let data = load_gen1_demo_pack();
        let state = initialize_battle(
            BattleInit {
                player: load_demo_player_team(),
                opponent: load_demo_enemy_team(),
            },
            &data,
        )
        .unwrap();
        let actions = legal_actions(&state, SideId::Player);
        let lines = action_menu_lines(&actions, &state, &data, Locale::EnUs);

        assert!(lines.iter().any(|line| line.contains("Type")));
        assert!(lines.iter().any(|line| line.contains("Power")));
    }

    #[test]
    fn render_log_lines_cover_all_visible_events() {
        let data = load_gen1_demo_pack();
        let state = initialize_battle(
            BattleInit {
                player: load_demo_player_team(),
                opponent: load_demo_enemy_team(),
            },
            &data,
        )
        .unwrap();
        let events = vec![
            battle_core::DomainEvent::TurnStarted { turn: 1 },
            battle_core::DomainEvent::MoveUsed {
                side: SideId::Player,
                move_id: battle_data::MoveId(2),
            },
            battle_core::DomainEvent::MoveMissed {
                side: SideId::Opponent,
                move_id: battle_data::MoveId(3),
            },
            battle_core::DomainEvent::DamageDealt {
                side: SideId::Player,
                target: SideId::Opponent,
                amount: 10,
                remaining_hp: 29,
            },
            battle_core::DomainEvent::PokemonFainted {
                side: SideId::Opponent,
                slot: 0,
            },
            battle_core::DomainEvent::PokemonSwitched {
                side: SideId::Opponent,
                slot: 1,
            },
            battle_core::DomainEvent::BattleEnded {
                winner: SideId::Player,
            },
        ];

        let lines = render_log_lines(&events, &state, &data, Locale::EnUs);
        assert!(lines.iter().any(|line| line.contains("Resolving turn 1")));
        assert!(
            lines
                .iter()
                .any(|line| line.contains("Player used Thunder Shock"))
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("Opponent wins") || line.contains("Player wins"))
        );
    }

    #[test]
    fn replay_writer_creates_target_file() {
        let data = load_gen1_demo_pack();
        let init = BattleInit {
            player: load_demo_player_team(),
            opponent: load_demo_enemy_team(),
        };
        let record = BattleRecord::new(init, 42, data.id.clone());
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("shamrock-replay-{unique}.json"));

        save_replay_to(&path, &record).unwrap();
        let written = std::fs::read_to_string(&path).unwrap();

        assert!(written.contains("\"seed\": 42"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn menu_choice_parser_accepts_valid_indices() {
        let actions = vec![
            battle_core::BattleAction::UseMove(0),
            battle_core::BattleAction::Switch(1),
        ];
        assert_eq!(
            resolve_menu_choice(&actions, "2"),
            Some(battle_core::BattleAction::Switch(1))
        );
        assert_eq!(resolve_menu_choice(&actions, "9"), None);
    }

    #[test]
    fn scripted_demo_reaches_a_terminal_result() {
        let result = play_demo_with(
            |_state, _data| Ok(battle_core::BattleAction::UseMove(0)),
            |_log, _state, _data| {},
            Locale::EnUs,
        )
        .unwrap();

        assert!(matches!(result.winner, SideId::Player | SideId::Opponent));
        assert!(!result.record.domain_events.is_empty());
    }

    #[test]
    fn trace_rendering_is_human_readable() {
        let data = load_gen1_demo_pack();
        let lines = render_trace_lines(
            &[
                battle_core::TraceEvent::ChoiceAccepted {
                    side: SideId::Player,
                },
                battle_core::TraceEvent::TurnResolved { turn: 3 },
                battle_core::TraceEvent::MoveOrderCalculated {
                    first: SideId::Player,
                    second: SideId::Opponent,
                },
                battle_core::TraceEvent::AccuracyRolled {
                    side: SideId::Opponent,
                    roll: 91,
                    needed: 100,
                },
                battle_core::TraceEvent::StatusRolled {
                    side: SideId::Player,
                    status: battle_data::StatusCondition::Paralyzed,
                    roll: 12,
                    needed: 25,
                },
                battle_core::TraceEvent::DamageRolled {
                    side: SideId::Player,
                    move_id: battle_data::MoveId(2),
                    damage: 61,
                },
                battle_core::TraceEvent::ActionSkipped {
                    side: SideId::Opponent,
                },
            ],
            &data,
            Locale::EnUs,
        );

        assert!(lines.iter().any(|line| line.contains("choice accepted")));
        assert!(lines.iter().any(|line| line.contains("Thunder Shock")));
        assert!(lines.iter().any(|line| line.contains("PAR")));
        assert!(lines.iter().any(|line| line.contains("action skipped")));
    }

    #[test]
    fn append_log_to_ui_routes_domain_trace_and_metric_lines() {
        let data = load_gen1_demo_pack();
        let state = initialize_battle(
            BattleInit {
                player: load_demo_player_team(),
                opponent: load_demo_enemy_team(),
            },
            &data,
        )
        .unwrap();
        let mut ui_log = UiEventLog::default();
        let battle_log = battle_core::BattleLog {
            domain: vec![battle_core::DomainEvent::TurnStarted { turn: 1 }],
            trace: vec![battle_core::TraceEvent::TurnResolved { turn: 1 }],
            metrics: vec![battle_core::MetricsEvent {
                turn: 1,
                domain_events: 1,
                trace_events: 1,
            }],
        };

        append_log_to_ui(&mut ui_log, &battle_log, &state, &data, Locale::EnUs);

        assert!(
            ui_log
                .domain
                .iter()
                .any(|line| line.contains("Resolving turn 1"))
        );
        assert!(
            ui_log
                .trace
                .iter()
                .any(|line| line.contains("turn 1 resolved"))
        );
        assert!(
            ui_log
                .system
                .iter()
                .any(|line| line.contains("METRIC turn 1"))
        );
    }

    #[test]
    fn recent_history_lines_keep_latest_entries() {
        let history = vec![
            "line-1".to_string(),
            "line-2".to_string(),
            "line-3".to_string(),
            "line-4".to_string(),
        ];
        let recent = crate::rendering::recent_history_lines(&history, 2);
        assert_eq!(recent, vec!["line-3".to_string(), "line-4".to_string()]);
    }
}
