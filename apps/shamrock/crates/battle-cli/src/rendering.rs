use battle_core::{BattleAction, BattleState, SideId};
use battle_data::{DataPack, MoveId};
use battle_view::UiEventLog;

use crate::i18n::Locale;

/**
把当前 battle 状态打印成几行简洁概览。

纯文本模式下，玩家每次输入前都先看到这几行。
它不是最终 UI 协议，只是最小可玩的看盘摘要。
*/
pub(crate) fn print_battle_status(state: &BattleState, data: &DataPack, locale: Locale) {
    for line in battle_status_lines(state, data, locale) {
        println!("{line}");
    }
}

/**
生成纯文本模式下的 battle 概览行。

先生成字符串，再统一输出，有利于测试直接断言内容，
不需要捕获 stdout 才能验证渲染逻辑。
*/
pub(crate) fn battle_status_lines(
    state: &BattleState,
    data: &DataPack,
    locale: Locale,
) -> Vec<String> {
    vec![
        locale.turn_header(state.turn),
        locale.weather_line(state.weather.map(|weather| (weather.kind, weather.remaining_turns))),
        side_status_line(state, SideId::Player, data, locale),
        side_type_status_line(state, SideId::Player, data, locale),
        side_status_line(state, SideId::Opponent, data, locale),
        side_type_status_line(state, SideId::Opponent, data, locale),
    ]
}

/**
生成某一边 active 宝可梦的状态行。

这是一条面向人的摘要，不追求完整，只保留当前决策最需要的信息。
*/
fn side_status_line(state: &BattleState, side: SideId, data: &DataPack, locale: Locale) -> String {
    let team = &state.teams[side.index()];
    let active = &team.party[team.active];
    let species = data.species(active.species);
    locale.active_status_line(
        side,
        &active.nickname,
        &species.name,
        active.current_hp.max(0),
        active.max_hp,
    )
}

fn side_type_status_line(state: &BattleState, side: SideId, data: &DataPack, locale: Locale) -> String {
    let team = &state.teams[side.index()];
    let active = &team.party[team.active];
    let species = data.species(active.species);
    locale.side_type_status_line(
        &locale.pokemon_type_line(species.primary_type, species.secondary_type),
        &locale.status_line(active.status),
    )
}

pub(crate) fn action_menu_lines(
    actions: &[BattleAction],
    state: &BattleState,
    data: &DataPack,
    locale: Locale,
) -> Vec<String> {
    let team = &state.teams[SideId::Player.index()];
    let active = &team.party[team.active];
    actions
        .iter()
        .enumerate()
        .map(|(index, action)| match action {
            BattleAction::UseMove(move_index) => {
                let move_def = data.move_def(active.moves[*move_index]);
                format!(
                    "  {}. {} [{}] {} {}  {} {}",
                    index + 1,
                    locale.action_use(&move_def.name),
                    locale.action_kind_move(),
                    locale.action_meta_type(),
                    locale.element_type_name(move_def.element_type),
                    locale.action_meta_power(),
                    move_def.power
                )
            }
            BattleAction::Switch(slot) => {
                let pokemon = &team.party[*slot];
                let species = data.species(pokemon.species);
                format!(
                    "  {}. {} [{}] {}",
                    index + 1,
                    locale.action_switch(&pokemon.nickname),
                    locale.action_kind_switch(),
                    locale.pokemon_type_line(species.primary_type, species.secondary_type),
                )
            }
        })
        .collect()
}

pub(crate) fn print_recent_history(history: &[String], locale: Locale) {
    if history.is_empty() {
        return;
    }
    println!("== {} ==", locale.recent_log_title());
    for line in recent_history_lines(history, 8) {
        println!("{line}");
    }
}

pub(crate) fn recent_history_lines(history: &[String], limit: usize) -> Vec<String> {
    let start = history.len().saturating_sub(limit);
    history[start..].to_vec()
}

pub(crate) fn print_full_history(history: &[String], locale: Locale) {
    println!("== {} ==", locale.history_title());
    if history.is_empty() {
        println!("{}", locale.history_empty());
    } else {
        print_lines(history);
    }
}

pub(crate) fn print_help_commands(locale: Locale) {
    for line in locale.help_commands_plain() {
        println!("{line}");
    }
}

pub(crate) fn print_lines(lines: &[String]) {
    for line in lines {
        println!("{line}");
    }
}

/**
把规则动作翻译成当前 CLI 展示给人的短文案。

外层界面不应该直接在渲染代码里到处拼 `BattleAction`。
把文案转换收拢到这里，后面切成 token 或更复杂 UI 时更好改。
*/
pub(crate) fn describe_action(
    action: BattleAction,
    state: &BattleState,
    side: SideId,
    data: &DataPack,
    locale: Locale,
) -> String {
    match action {
        BattleAction::UseMove(index) => {
            let active = &state.teams[side.index()].party[state.teams[side.index()].active];
            let move_name = move_name(active.moves[index], data);
            locale.action_use(move_name)
        }
        BattleAction::Switch(index) => {
            let pokemon = &state.teams[side.index()].party[index];
            locale.action_switch(&pokemon.nickname)
        }
    }
}

/**
把领域事件转换成文本行。

这里的职责是“如何让人看懂这批事件”，不是“定义事件语义”。
真正的语义边界仍然在 `battle-core` 的 `DomainEvent`。
*/
pub(crate) fn render_log_lines(
    events: &[battle_core::DomainEvent],
    state: &BattleState,
    data: &DataPack,
    locale: Locale,
) -> Vec<String> {
    let mut lines = Vec::new();
    for event in events {
        match event {
            battle_core::DomainEvent::ChoiceCommitted { .. } => {}
            battle_core::DomainEvent::TurnStarted { turn } => {
                lines.push(locale.resolving_turn(*turn))
            }
            battle_core::DomainEvent::WeatherStarted {
                weather,
                remaining_turns,
            } => {
                lines.push(locale.weather_started(*weather, *remaining_turns));
            }
            battle_core::DomainEvent::WeatherEnded { weather } => {
                lines.push(locale.weather_ended(*weather));
            }
            battle_core::DomainEvent::ForcedSwitch { side } => {
                lines.push(locale.forced_switch(*side));
            }
            battle_core::DomainEvent::MoveUsed { side, move_id } => {
                lines.push(locale.used_move(*side, move_name(*move_id, data)));
            }
            battle_core::DomainEvent::MoveMissed { side, move_id } => {
                lines.push(locale.missed_move(*side, move_name(*move_id, data)));
            }
            battle_core::DomainEvent::DamageDealt {
                target,
                amount,
                remaining_hp,
                ..
            } => {
                lines.push(locale.damage_taken(*target, *amount, *remaining_hp));
            }
            battle_core::DomainEvent::ResidualDamage {
                target,
                status,
                amount,
                remaining_hp,
            } => {
                lines.push(locale.residual_damage(*target, *status, *amount, *remaining_hp));
            }
            battle_core::DomainEvent::Healed {
                side,
                amount,
                remaining_hp,
            } => {
                lines.push(locale.healed(*side, *amount, *remaining_hp));
            }
            battle_core::DomainEvent::StatusApplied { side, status } => {
                lines.push(locale.status_applied(*side, *status));
            }
            battle_core::DomainEvent::StatStageChanged {
                side,
                stat,
                new_stage,
            } => {
                lines.push(locale.stat_stage_changed(*side, *stat, *new_stage));
            }
            battle_core::DomainEvent::ActionBlockedByStatus { side, status } => {
                lines.push(locale.action_blocked_by_status(*side, *status));
            }
            battle_core::DomainEvent::PokemonFainted { side, .. } => {
                lines.push(locale.active_fainted(*side));
            }
            battle_core::DomainEvent::PokemonSwitched { side, .. } => {
                let team = &state.teams[side.index()];
                lines.push(locale.sent_out(*side, &team.party[team.active].nickname));
            }
            battle_core::DomainEvent::BattleEnded { winner } => {
                lines.push(locale.battle_wins(*winner));
            }
        }
    }
    lines
}

/**
把调试事件渲染成文本行，供 TUI 的 trace tab 使用。

和领域事件不同，这里优先保留调试密度，而不是自然语言可读性。
*/
pub(crate) fn render_trace_lines(
    events: &[battle_core::TraceEvent],
    data: &DataPack,
    locale: Locale,
) -> Vec<String> {
    let mut lines = Vec::new();
    for event in events {
        match event {
            battle_core::TraceEvent::ChoiceAccepted { side } => {
                lines.push(locale.trace_choice_accepted(*side))
            }
            battle_core::TraceEvent::TurnResolved { turn } => {
                lines.push(locale.trace_turn_resolved(*turn))
            }
            battle_core::TraceEvent::MoveOrderCalculated { first, second } => {
                lines.push(locale.trace_move_order(*first, *second));
            }
            battle_core::TraceEvent::AccuracyRolled { side, roll, needed } => {
                lines.push(locale.trace_accuracy(*side, *roll, *needed));
            }
            battle_core::TraceEvent::StatusRolled {
                side,
                status,
                roll,
                needed,
            } => {
                lines.push(locale.trace_status_roll(*side, *status, *roll, *needed));
            }
            battle_core::TraceEvent::WeatherAppliedToDamage { weather, move_id } => {
                lines.push(locale.trace_weather_damage(*weather, move_name(*move_id, data)));
            }
            battle_core::TraceEvent::DamageRolled {
                side,
                move_id,
                damage,
            } => {
                lines.push(locale.trace_damage(*side, move_name(*move_id, data), *damage));
            }
            battle_core::TraceEvent::ActionSkipped { side } => {
                lines.push(locale.trace_action_skipped(*side));
            }
        }
    }
    lines
}

/**
把一步结算产生的多层日志追加到 UI 文本缓存。

这样 TUI 不需要每次重新从 typed log 全量渲染，而是只维护一个不断增长的文本窗口。
*/
pub(crate) fn append_log_to_ui(
    ui_log: &mut UiEventLog,
    log: &battle_core::BattleLog,
    state: &BattleState,
    data: &DataPack,
    locale: Locale,
) {
    ui_log
        .domain
        .extend(render_log_lines(&log.domain, state, data, locale));
    ui_log
        .trace
        .extend(render_trace_lines(&log.trace, data, locale));
    for metric in &log.metrics {
        ui_log.system.push(locale.metric_line(
            metric.turn,
            metric.domain_events,
            metric.trace_events,
        ));
    }
}

/**
通过 `MoveId` 读取招式名。

这个小函数把“查表取招式名”集中起来，减少外层渲染代码直接碰数据定义。
*/
pub(crate) fn move_name<'a>(id: MoveId, data: &'a DataPack) -> &'a str {
    &data.move_def(id).name
}
