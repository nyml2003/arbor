use battle_data::{DataPack, EffectTarget, MoveEffect};
use battle_mechanics::{DamageInput, calculate_damage};

use crate::SideId;
use crate::log::{DomainEvent, Recorder, TraceEvent};
use crate::ops::{BattleOp, apply_op};
use crate::rng::RngState;
use crate::state::{BattleState, active_pokemon};

pub(crate) fn resolve_move(
    state: &mut BattleState,
    side: SideId,
    move_index: usize,
    data: &DataPack,
    rng: &mut RngState,
    rec: &mut impl Recorder,
) -> Option<SideId> {
    let attacker = active_pokemon(state, side).clone();
    let move_id = attacker.moves[move_index];
    let move_def = data.move_def(move_id);
    rec.domain(DomainEvent::MoveUsed { side, move_id });

    let accuracy_roll = rng.roll_percent();
    rec.trace(TraceEvent::AccuracyRolled { side, roll: accuracy_roll, needed: move_def.accuracy });
    if accuracy_roll > move_def.accuracy {
        rec.domain(DomainEvent::MoveMissed { side, move_id });
        return None;
    }

    let defender = active_pokemon(state, side.foe()).clone();
    let attacker_species = data.species(attacker.species);
    let defender_species = data.species(defender.species);

    match move_def.effect {
        MoveEffect::Damage => {
            let weather_kind = state.weather.map(|weather| weather.kind);
            if weather_kind.is_some() {
                if let Some(weather) = state.weather {
                    rec.trace(TraceEvent::WeatherAppliedToDamage { weather: weather.kind, move_id });
                }
            }
            let damage = calculate_damage(DamageInput {
                power: move_def.power,
                attack: attacker.stats.attack,
                attack_stage: attacker.stages.attack,
                defense: defender.stats.defense,
                defense_stage: defender.stages.defense,
                stab: move_def.element_type == attacker_species.primary_type
                    || Some(move_def.element_type) == attacker_species.secondary_type,
                move_type: move_def.element_type,
                defender_primary: defender_species.primary_type,
                defender_secondary: defender_species.secondary_type,
                weather: weather_kind,
                variance: rng.roll_range_inclusive(85, 100),
            });
            rec.trace(TraceEvent::DamageRolled { side, move_id, damage });

            apply_op(state, BattleOp::Damage { target: side.foe(), amount: damage }, rec);
            None
        }
        MoveEffect::ApplyStatus { target, status } => {
            apply_op(
                state,
                BattleOp::ApplyStatus {
                    target: effect_target_side(side, target),
                    status,
                },
                rec,
            );
            None
        }
        MoveEffect::ModifyStat { target, stat, stages } => {
            apply_op(
                state,
                BattleOp::ModifyStatStage {
                    target: effect_target_side(side, target),
                    stat,
                    delta: stages,
                },
                rec,
            );
            None
        }
        MoveEffect::HealPercent { target, percent } => {
            let heal_target = effect_target_side(side, target);
            let heal_amount = ((active_pokemon(state, heal_target).max_hp.max(1) as u16) * u16::from(percent) / 100).max(1);
            apply_op(state, BattleOp::Heal { target: heal_target, amount: heal_amount }, rec);
            None
        }
        MoveEffect::SetWeather { weather, turns } => {
            apply_op(state, BattleOp::SetWeather { weather, turns }, rec);
            None
        }
        MoveEffect::ForceSwitch { target } => {
            let target_side = effect_target_side(side, target);
            if apply_op(state, BattleOp::ForceSwitch { target: target_side }, rec) {
                Some(target_side)
            } else {
                None
            }
        }
    }
}

fn effect_target_side(user: SideId, target: EffectTarget) -> SideId {
    match target {
        EffectTarget::User => user,
        EffectTarget::Opponent => user.foe(),
    }
}
