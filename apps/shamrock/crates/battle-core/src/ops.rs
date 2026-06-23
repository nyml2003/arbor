use battle_data::{StatId, StatusCondition, WeatherKind};

use crate::SideId;
use crate::log::{DomainEvent, Recorder};
use crate::state::{BattleState, CombatPokemon, TeamState, WeatherState};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BattleOp {
    Damage { target: SideId, amount: u16 },
    ResidualDamage { target: SideId, status: StatusCondition, amount: u16 },
    Heal { target: SideId, amount: u16 },
    ApplyStatus { target: SideId, status: StatusCondition },
    ModifyStatStage { target: SideId, stat: StatId, delta: i8 },
    SetWeather { weather: WeatherKind, turns: u8 },
    Switch { side: SideId, slot: usize },
    ForceSwitch { target: SideId },
}

pub(crate) fn current_stat_stage(pokemon: &CombatPokemon, stat: StatId) -> i8 {
    match stat {
        StatId::Attack => pokemon.stages.attack,
        StatId::Defense => pokemon.stages.defense,
        StatId::Speed => pokemon.stages.speed,
    }
}

pub(crate) fn set_stat_stage(pokemon: &mut CombatPokemon, stat: StatId, value: i8) {
    match stat {
        StatId::Attack => pokemon.stages.attack = value,
        StatId::Defense => pokemon.stages.defense = value,
        StatId::Speed => pokemon.stages.speed = value,
    }
}

pub(crate) fn clamp_stage(value: i8) -> i8 {
    value.clamp(-6, 6)
}

pub(crate) fn apply_op(state: &mut BattleState, op: BattleOp, rec: &mut impl Recorder) -> bool {
    match op {
        BattleOp::Switch { side, slot } => {
            state.teams[side.index()].active = slot;
            rec.domain(DomainEvent::PokemonSwitched { side, slot });
            true
        }
        BattleOp::Damage { target, amount } => {
            let team = &mut state.teams[target.index()];
            let active_slot = team.active;
            let (remaining_hp, fainted) = {
                let pokemon = &mut team.party[active_slot];
                pokemon.current_hp = (pokemon.current_hp - i32::from(amount)).max(0);
                (pokemon.current_hp as u16, pokemon.is_fainted())
            };
            rec.domain(DomainEvent::DamageDealt {
                side: target.foe(),
                target,
                amount,
                remaining_hp,
            });

            let next_slot = if fainted { first_alive_bench(team) } else { None };
            let no_pokemon_left = fainted && next_slot.is_none();
            let fainted_slot = active_slot;
            let _ = team;

            if fainted {
                rec.domain(DomainEvent::PokemonFainted { side: target, slot: fainted_slot });
                if let Some(next_slot) = next_slot {
                    apply_op(state, BattleOp::Switch { side: target, slot: next_slot }, rec);
                } else if no_pokemon_left {
                    state.winner = Some(target.foe());
                    rec.domain(DomainEvent::BattleEnded { winner: target.foe() });
                }
            }
            true
        }
        BattleOp::ResidualDamage { target, status, amount } => {
            let team = &mut state.teams[target.index()];
            let active_slot = team.active;
            let (remaining_hp, fainted) = {
                let pokemon = &mut team.party[active_slot];
                pokemon.current_hp = (pokemon.current_hp - i32::from(amount)).max(0);
                (pokemon.current_hp as u16, pokemon.is_fainted())
            };
            rec.domain(DomainEvent::ResidualDamage { target, status, amount, remaining_hp });

            let next_slot = if fainted { first_alive_bench(team) } else { None };
            let no_pokemon_left = fainted && next_slot.is_none();
            let fainted_slot = active_slot;
            let _ = team;

            if fainted {
                rec.domain(DomainEvent::PokemonFainted { side: target, slot: fainted_slot });
                if let Some(next_slot) = next_slot {
                    apply_op(state, BattleOp::Switch { side: target, slot: next_slot }, rec);
                } else if no_pokemon_left {
                    state.winner = Some(target.foe());
                    rec.domain(DomainEvent::BattleEnded { winner: target.foe() });
                }
            }
            true
        }
        BattleOp::Heal { target, amount } => {
            let team = &mut state.teams[target.index()];
            let active_slot = team.active;
            let healed = {
                let pokemon = &mut team.party[active_slot];
                let before = pokemon.current_hp;
                pokemon.current_hp = (pokemon.current_hp + i32::from(amount)).min(pokemon.max_hp);
                (pokemon.current_hp - before) as u16
            };

            if healed > 0 {
                let remaining_hp = team.party[active_slot].current_hp as u16;
                rec.domain(DomainEvent::Healed { side: target, amount: healed, remaining_hp });
            }
            healed > 0
        }
        BattleOp::ApplyStatus { target, status } => {
            let team = &mut state.teams[target.index()];
            let active_slot = team.active;
            let pokemon = &mut team.party[active_slot];
            if pokemon.status.is_none() {
                pokemon.status = Some(status);
                rec.domain(DomainEvent::StatusApplied { side: target, status });
                true
            } else {
                false
            }
        }
        BattleOp::ModifyStatStage { target, stat, delta } => {
            let team = &mut state.teams[target.index()];
            let active_slot = team.active;
            let pokemon = &mut team.party[active_slot];
            let current = current_stat_stage(pokemon, stat);
            let next = clamp_stage(current + delta);
            if next != current {
                set_stat_stage(pokemon, stat, next);
                rec.domain(DomainEvent::StatStageChanged { side: target, stat, new_stage: next });
                true
            } else {
                false
            }
        }
        BattleOp::SetWeather { weather, turns } => {
            state.weather = Some(WeatherState { kind: weather, remaining_turns: turns });
            rec.domain(DomainEvent::WeatherStarted { weather, remaining_turns: turns });
            true
        }
        BattleOp::ForceSwitch { target } => {
            let team = &state.teams[target.index()];
            let Some(next_slot) = first_alive_bench(team) else {
                return false;
            };
            rec.domain(DomainEvent::ForcedSwitch { side: target });
            apply_op(state, BattleOp::Switch { side: target, slot: next_slot }, rec)
        }
    }
}

pub(crate) fn first_alive_bench(team: &TeamState) -> Option<usize> {
    team.party
        .iter()
        .enumerate()
        .find(|(index, pokemon)| *index != team.active && !pokemon.is_fainted())
        .map(|(index, _)| index)
}
