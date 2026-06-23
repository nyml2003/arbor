use battle_data::{DataPack, StatusCondition};
use battle_mechanics::{compare_action_order, effective_speed, residual_status_damage, switch_priority};

use crate::{BattleAction, SideId};
use crate::log::{DomainEvent, Recorder, TraceEvent};
use crate::move_resolution::resolve_move;
use crate::ops::{BattleOp, apply_op};
use crate::rng::RngState;
use crate::state::{BattleError, BattleState, active_pokemon};

pub(crate) fn validate_action(state: &BattleState, side: SideId, action: BattleAction) -> Result<(), BattleError> {
    match action {
        BattleAction::UseMove(index) => {
            let pokemon = active_pokemon(state, side);
            if index >= pokemon.moves.len() {
                Err(BattleError::InvalidMoveIndex { side, index })
            } else {
                Ok(())
            }
        }
        BattleAction::Switch(index) => {
            let team = &state.teams[side.index()];
            if index >= team.party.len() || index == team.active || team.party[index].is_fainted() {
                Err(BattleError::InvalidSwitchIndex { side, index })
            } else {
                Ok(())
            }
        }
    }
}

pub(crate) fn resolve_turn(state: &mut BattleState, data: &DataPack, rng: &mut RngState, rec: &mut impl Recorder) {
    let player_action = state.pending[0].take().expect("player choice present");
    let enemy_action = state.pending[1].take().expect("opponent choice present");
    let mut forced_out = [false, false];

    rec.domain(DomainEvent::TurnStarted { turn: state.turn });

    let mut order = [
        TurnIntent { side: SideId::Player, action: player_action },
        TurnIntent { side: SideId::Opponent, action: enemy_action },
    ];
    order.sort_by(|left, right| compare_intents(state, data, *left, *right).cmp(&0));
    rec.trace(TraceEvent::MoveOrderCalculated { first: order[0].side, second: order[1].side });

    for intent in order {
        if state.winner.is_some() {
            break;
        }

        if active_pokemon(state, intent.side).is_fainted() {
            rec.trace(TraceEvent::ActionSkipped { side: intent.side });
            continue;
        }

        if forced_out[intent.side.index()] {
            rec.trace(TraceEvent::ActionSkipped { side: intent.side });
            continue;
        }

        if is_action_blocked_by_status(state, intent.side, rng, rec) {
            continue;
        }

        match intent.action {
            BattleAction::Switch(slot) => {
                let _ = apply_op(state, BattleOp::Switch { side: intent.side, slot }, rec);
            }
            BattleAction::UseMove(move_index) => {
                if let Some(side) = resolve_move(state, intent.side, move_index, data, rng, rec) {
                    forced_out[side.index()] = true;
                }
            }
        }
    }

    if state.winner.is_none() {
        resolve_end_of_turn(state, rec);
    }

    rec.trace(TraceEvent::TurnResolved { turn: state.turn });
    state.turn += 1;
}

fn compare_intents(state: &BattleState, data: &DataPack, left: TurnIntent, right: TurnIntent) -> i32 {
    let left_priority = action_priority(state, left, data);
    let right_priority = action_priority(state, right, data);
    let left_speed = active_speed(state, left.side, data);
    let right_speed = active_speed(state, right.side, data);
    compare_action_order(
        left_priority,
        right_priority,
        left_speed,
        right_speed,
        left.side == SideId::Player,
    )
}

fn action_priority(state: &BattleState, intent: TurnIntent, data: &DataPack) -> i8 {
    match intent.action {
        BattleAction::Switch(_) => switch_priority(),
        BattleAction::UseMove(index) => data.move_def(active_pokemon(state, intent.side).moves[index]).priority,
    }
}

fn active_speed(state: &BattleState, side: SideId, data: &DataPack) -> u16 {
    let pokemon = active_pokemon(state, side);
    let _ = data;
    effective_speed(pokemon.stats.speed, pokemon.stages.speed, pokemon.status)
}

fn is_action_blocked_by_status(state: &BattleState, side: SideId, rng: &mut RngState, rec: &mut impl Recorder) -> bool {
    match active_pokemon(state, side).status {
        Some(StatusCondition::Paralyzed) => {
            let roll = rng.roll_percent();
            rec.trace(TraceEvent::StatusRolled { side, status: StatusCondition::Paralyzed, roll, needed: 25 });
            if roll <= 25 {
                rec.domain(DomainEvent::ActionBlockedByStatus { side, status: StatusCondition::Paralyzed });
                rec.trace(TraceEvent::ActionSkipped { side });
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

fn resolve_end_of_turn(state: &mut BattleState, rec: &mut impl Recorder) {
    for side in [SideId::Player, SideId::Opponent] {
        if state.winner.is_some() || active_pokemon(state, side).is_fainted() {
            continue;
        }

        if let Some(status) = active_pokemon(state, side).status {
            let Some(amount) = residual_status_damage(status, active_pokemon(state, side).max_hp) else {
                continue;
            };
            apply_op(
                state,
                BattleOp::ResidualDamage {
                    target: side,
                    status,
                    amount,
                },
                rec,
            );
        }
    }

    if let Some(mut weather) = state.weather {
        if weather.remaining_turns > 0 {
            weather.remaining_turns -= 1;
        }

        if weather.remaining_turns == 0 {
            state.weather = None;
            rec.domain(DomainEvent::WeatherEnded { weather: weather.kind });
        } else {
            state.weather = Some(weather);
        }
    }
}

#[derive(Clone, Copy)]
struct TurnIntent {
    side: SideId,
    action: BattleAction,
}
