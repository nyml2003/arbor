//! Pure application boundary shared by human and agent battle clients.

#![forbid(unsafe_code)]

mod observation;

pub use battle_domain::{
    Accuracy, Action, BattleError, BattleOutcome, BattlePhase, BattleStats, CalculatedStats,
    EffortValues, IllegalActionReason, IndividualValues, MAX_EFFORT_VALUE, MAX_INDIVIDUAL_VALUE,
    MAX_MOVES, MAX_TOTAL_EFFORT_VALUE, Move, MoveId, MoveSlot, Nature, NonHpStat, Pokemon,
    PokemonId, PokemonType, ReplacementSides, Side, StatBlock, StatName, StatProjectionError,
    TEAM_SIZE, Team, TeamSlot, TrainingValues, TypeEffectiveness, ValidationError,
    calculate_gen3_stats,
};
pub use observation::{
    BattleEvent, BattleObservation, DamageSource, OpponentSideObservation, OwnSideObservation,
    RevealedMoveObservation, RevealedPokemonObservation, SubmitOutcome, UsedMove,
};

use battle_domain::{Battle, BattleCommand};

pub struct BattlePerspective {
    side: Side,
}

pub struct BattleApplication {
    battle: Battle,
}

impl BattleApplication {
    pub fn new(team_one: Team, team_two: Team, seed: u64) -> Result<Self, BattleError> {
        Ok(Self {
            battle: Battle::new(team_one, team_two, seed)?,
        })
    }

    pub fn perspectives(&self) -> (BattlePerspective, BattlePerspective) {
        (
            BattlePerspective { side: Side::One },
            BattlePerspective { side: Side::Two },
        )
    }

    pub fn observe(&self, perspective: &BattlePerspective) -> BattleObservation {
        observation::observe(&self.battle, perspective.side)
    }

    pub fn legal_actions(&self, perspective: &BattlePerspective) -> Vec<Action> {
        self.battle.legal_actions(perspective.side)
    }

    pub fn submit(
        &mut self,
        perspective: &BattlePerspective,
        action: Action,
    ) -> Result<SubmitOutcome, BattleError> {
        let viewer = perspective.side;
        self.battle
            .submit(BattleCommand::new(viewer, action))
            .map(|outcome| SubmitOutcome::from_domain(outcome, viewer))
    }

    pub fn event_log(&self, perspective: &BattlePerspective) -> Vec<BattleEvent> {
        observation::event_log(&self.battle, perspective.side)
    }
}

#[cfg(test)]
mod tests;
