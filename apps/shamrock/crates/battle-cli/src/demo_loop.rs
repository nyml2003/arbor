use std::error::Error;
use std::io::{self, Write};

use battle_core::{
    BattleAction, BattleInit, BattleState, Request, RngState, SideId, initialize_battle,
    requested_side, step,
};
use battle_data::{DataPack, load_demo_enemy_team, load_demo_player_team, load_gen1_demo_pack};
use battle_format::legal_actions;
use battle_replay::BattleRecord;
use battle_view::{EventTab, UiEventLog, ViewerProfile, build_battle_snapshot, build_public_battle_view};

use crate::ai::choose_enemy_action;
use crate::i18n::Locale;
use crate::rendering::{
    action_menu_lines, append_log_to_ui, describe_action, print_battle_status,
    print_full_history, print_help_commands, print_lines, print_recent_history, render_log_lines,
};
use crate::tui::Tui;

pub(crate) struct DemoResult {
    pub(crate) record: BattleRecord,
    pub(crate) winner: SideId,
}

/**
纯文本模式的 demo 壳层。

它不关心 battle 规则细节，只把“怎么选动作”和“怎么显示日志”接到统一 runner 上。
*/
pub(crate) fn run_plain_demo(
    locale: Locale,
    announce_fallback: bool,
) -> Result<Option<DemoResult>, Box<dyn Error>> {
    if announce_fallback {
        println!("{}", locale.tty_fallback());
    } else {
        println!("{}", locale.plain_mode_selected());
    }

    let data = load_gen1_demo_pack();
    let init = BattleInit {
        player: load_demo_player_team(),
        opponent: load_demo_enemy_team(),
    };
    let mut state = initialize_battle(init.clone(), &data)?;
    let seed = 42;
    let mut rng = RngState::seeded(seed);
    let mut record = BattleRecord::new(init, seed, data.id.clone());
    let mut history: Vec<String> = Vec::new();

    loop {
        match requested_side(&state) {
            Request::ChooseAction {
                side: SideId::Player,
            } => {
                print_recent_history(&history, locale);
                print_battle_status(&state, &data, locale);
                let action = prompt_player_action(&state, &data, locale, &history)?;
                record.push_input(SideId::Player, action);

                let result = step(state, SideId::Player, action, rng, &data)?;
                record_step(&mut record, locale, &result);
                let lines = render_log_lines(&result.log.domain, &result.state, &data, locale);
                print_lines(&lines);
                history.extend(lines);
                state = result.state;
                rng = result.rng;
            }
            Request::ChooseAction {
                side: SideId::Opponent,
            } => {
                let action = choose_enemy_action(&state, &data);
                let choice_line = locale.opponent_chooses(&describe_action(
                    action,
                    &state,
                    SideId::Opponent,
                    &data,
                    locale,
                ));
                println!("{choice_line}");
                history.push(choice_line);
                record.push_input(SideId::Opponent, action);

                let result = step(state, SideId::Opponent, action, rng, &data)?;
                record_step(&mut record, locale, &result);
                let lines = render_log_lines(&result.log.domain, &result.state, &data, locale);
                print_lines(&lines);
                history.extend(lines);
                state = result.state;
                rng = result.rng;
            }
            Request::Finished { winner } => {
                record.add_checkpoint(state.turn, locale.checkpoint_battle_finished());
                return Ok(Some(DemoResult { record, winner }));
            }
        }
    }
}

/**
TUI 模式的 demo 壳层。

这个函数负责维护一场 battle 的外层运行环境：

- 当前 battle 状态和 RNG
- replay 记录
- UI 文本日志缓存
- 当前 agent 摘要
- TUI 生命周期

规则推进仍然全部交给 `battle-core::step`。
*/
pub(crate) fn run_tui_demo(locale: Locale) -> Result<Option<DemoResult>, Box<dyn Error>> {
    let data = load_gen1_demo_pack();
    let init = BattleInit {
        player: load_demo_player_team(),
        opponent: load_demo_enemy_team(),
    };
    let mut state = initialize_battle(init.clone(), &data)?;
    let seed = 42;
    let mut rng = RngState::seeded(seed);
    let mut record = BattleRecord::new(init, seed, data.id.clone());
    let mut ui_log = UiEventLog::default();
    ui_log.system.push(locale.tui_ready().to_string());
    ui_log.system.push(locale.hotkey_summary().to_string());
    let mut latest_agent_summary = locale.waiting_for_player_input().to_string();
    let mut tui = Tui::enter()?;

    loop {
        match requested_side(&state) {
            Request::ChooseAction {
                side: SideId::Player,
            } => {
                let action = loop {
                    let actions = legal_actions(&state, SideId::Player);
                    let snapshot = build_battle_snapshot(
                        &state,
                        &data,
                        "first-playable-demo",
                        seed,
                        &actions,
                        ViewerProfile::LocalPlayer(SideId::Player),
                    );
                    let view = build_public_battle_view(
                        &snapshot,
                        locale,
                        &ui_log,
                        tui.selected_tab(),
                        &latest_agent_summary,
                    );
                    match tui.wait_for_player_action(&view)? {
                        None => return Ok(None),
                        Some(index) => {
                            if let Some(action) = actions.get(index).copied() {
                                break action;
                            }
                            ui_log.system.push(locale.invalid_action_index(index));
                        }
                    }
                };

                record.push_input(SideId::Player, action);
                let result = step(state, SideId::Player, action, rng, &data)?;
                record_step(&mut record, locale, &result);
                append_log_to_ui(&mut ui_log, &result.log, &result.state, &data, locale);
                latest_agent_summary = locale.player_command_accepted().to_string();
                state = result.state;
                rng = result.rng;
            }
            Request::ChooseAction {
                side: SideId::Opponent,
            } => {
                let snapshot = build_battle_snapshot(
                    &state,
                    &data,
                    "first-playable-demo",
                    seed,
                    &[],
                    ViewerProfile::Spectator,
                );
                let view = build_public_battle_view(
                    &snapshot,
                    locale,
                    &ui_log,
                    tui.selected_tab(),
                    &latest_agent_summary,
                );
                tui.draw(&view)?;
                let action = choose_enemy_action(&state, &data);
                latest_agent_summary = locale.ai_chose(&describe_action(
                    action,
                    &state,
                    SideId::Opponent,
                    &data,
                    locale,
                ));
                ui_log.system.push(latest_agent_summary.clone());
                record.push_input(SideId::Opponent, action);

                let result = step(state, SideId::Opponent, action, rng, &data)?;
                record_step(&mut record, locale, &result);
                append_log_to_ui(&mut ui_log, &result.log, &result.state, &data, locale);
                state = result.state;
                rng = result.rng;

                ui_log.system.push(locale.continue_prompt().to_string());
                let snapshot = build_battle_snapshot(
                    &state,
                    &data,
                    "first-playable-demo",
                    seed,
                    &[],
                    ViewerProfile::Spectator,
                );
                let view = build_public_battle_view(
                    &snapshot,
                    locale,
                    &ui_log,
                    tui.selected_tab(),
                    &latest_agent_summary,
                );
                let proceed = tui.wait_for_continue(&view)?;
                let _ = ui_log.system.pop();
                if !proceed {
                    return Ok(None);
                }
            }
            Request::Finished { winner } => {
                record.add_checkpoint(state.turn, locale.checkpoint_battle_finished());
                ui_log.system.push(locale.finished_line(winner));
                ui_log.system.push(locale.leave_battle_prompt().to_string());
                let snapshot = build_battle_snapshot(
                    &state,
                    &data,
                    "first-playable-demo",
                    seed,
                    &[],
                    ViewerProfile::Spectator,
                );
                let view = build_public_battle_view(
                    &snapshot,
                    locale,
                    &ui_log,
                    EventTab::Domain,
                    &latest_agent_summary,
                );
                if !tui.wait_for_continue(&view)? {
                    return Ok(None);
                }
                return Ok(Some(DemoResult { record, winner }));
            }
        }
    }
}

/**
读取玩家输入，并把数字菜单映射成一个合法动作。

这里故意只做很薄的一层：

- 从 `battle-format` 拿合法动作
- 用稳定数字菜单展示给玩家
- 把输入解析成 `BattleAction`

真正“哪些动作合法”的规则，不写在这里。
*/
fn prompt_player_action(
    state: &BattleState,
    data: &DataPack,
    locale: Locale,
    history: &[String],
) -> Result<BattleAction, Box<dyn Error>> {
    let actions = legal_actions(state, SideId::Player);
    print_lines(&action_menu_lines(&actions, state, data, locale));
    println!("{}", locale.plain_commands_hint());

    loop {
        print!("{}", locale.choose_action_prompt());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();
        if trimmed.eq_ignore_ascii_case("history") || trimmed.eq_ignore_ascii_case("h") {
            print_full_history(history, locale);
            continue;
        }
        if trimmed.eq_ignore_ascii_case("help") {
            print_help_commands(locale);
            continue;
        }
        if let Some(action) = resolve_menu_choice(&actions, &input) {
            return Ok(action);
        }

        println!("{}", locale.invalid_choice());
    }
}

/**
把一行菜单输入解析成一个候选动作。

这个函数刻意和 IO 循环分开，方便单独测试输入解析边界。
*/
pub(crate) fn resolve_menu_choice(actions: &[BattleAction], input: &str) -> Option<BattleAction> {
    input
        .trim()
        .parse::<usize>()
        .ok()
        .and_then(|choice| actions.get(choice.saturating_sub(1)).copied())
}

/**
这是真正驱动整场 demo battle 的主循环。

它把外壳能力抽成两个回调：

- `choose_player` 决定玩家怎么选动作
- `on_step` 决定每一步日志怎么消费

这样纯文本 CLI、TUI 和测试都可以复用同一套 battle 编排。
*/
pub(crate) fn play_demo_with<F, G>(
    mut choose_player: F,
    mut on_step: G,
    locale: Locale,
) -> Result<DemoResult, Box<dyn Error>>
where
    F: FnMut(&BattleState, &DataPack) -> Result<BattleAction, Box<dyn Error>>,
    G: FnMut(&battle_core::BattleLog, &BattleState, &DataPack),
{
    let data = load_gen1_demo_pack();
    let init = BattleInit {
        player: load_demo_player_team(),
        opponent: load_demo_enemy_team(),
    };
    let mut state = initialize_battle(init.clone(), &data)?;
    let seed = 42;
    let mut rng = RngState::seeded(seed);
    let mut record = BattleRecord::new(init, seed, data.id.clone());

    loop {
        match requested_side(&state) {
            Request::ChooseAction {
                side: SideId::Player,
            } => {
                let action = choose_player(&state, &data)?;
                record.push_input(SideId::Player, action);

                let result = step(state, SideId::Player, action, rng, &data)?;
                on_step(&result.log, &result.state, &data);
                record_step(&mut record, locale, &result);
                state = result.state;
                rng = result.rng;
            }
            Request::ChooseAction {
                side: SideId::Opponent,
            } => {
                let action = choose_enemy_action(&state, &data);
                println!(
                    "{}",
                    locale.opponent_chooses(&describe_action(
                        action,
                        &state,
                        SideId::Opponent,
                        &data,
                        locale
                    ))
                );
                record.push_input(SideId::Opponent, action);

                let result = step(state, SideId::Opponent, action, rng, &data)?;
                on_step(&result.log, &result.state, &data);
                record_step(&mut record, locale, &result);
                state = result.state;
                rng = result.rng;
            }
            Request::Finished { winner } => {
                record.add_checkpoint(state.turn, locale.checkpoint_battle_finished());
                return Ok(DemoResult { record, winner });
            }
        }
    }
}

fn record_step(
    record: &mut BattleRecord,
    locale: Locale,
    result: &battle_core::StepResult,
) {
    record.append_log(&result.log);
    if !result.log.metrics.is_empty() {
        record.add_checkpoint(
            result.state.turn,
            locale.checkpoint_turn_resolved(result.state.turn.saturating_sub(1)),
        );
    }
}
