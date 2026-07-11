//! Deterministic, platform-independent rules for a generation-three-style battle.

#![forbid(unsafe_code)]

mod battle;
mod model;
mod rules;

pub use battle::{
    Action, Battle, BattleCommand, BattleError, BattleEvent, BattleOutcome, BattlePhase,
    DamageSource, IllegalActionReason, ReplacementSides, SubmitOutcome, UsedMove,
};
pub use model::{
    Accuracy, BattleStats, MAX_MOVES, Move, MoveId, MoveSlot, Pokemon, PokemonId, PokemonType,
    Side, TEAM_SIZE, Team, TeamSlot, ValidationError,
};
pub use rules::{DamageCategory, TypeEffectiveness, damage_category, type_effectiveness};

#[cfg(test)]
mod tests;
