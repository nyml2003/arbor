use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{Display, Formatter};

use battle_data::{DataPack, MoveId, SpeciesId, StatusCondition, TeamTemplate, WeatherKind};
use battle_mechanics::{BattleStats, resolve_battle_stats};

use crate::{BattleAction, Request, SideId};
use crate::log::BattleLog;
use crate::rng::RngState;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BattleInit {
    pub player: TeamTemplate,
    pub opponent: TeamTemplate,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct StatStages {
    pub attack: i8,
    pub defense: i8,
    pub speed: i8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct WeatherState {
    pub kind: WeatherKind,
    pub remaining_turns: u8,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CombatPokemon {
    pub nickname: String,
    pub species: SpeciesId,
    pub moves: [MoveId; 4],
    pub stats: BattleStats,
    pub current_hp: i32,
    pub max_hp: i32,
    pub status: Option<StatusCondition>,
    pub stages: StatStages,
}

impl CombatPokemon {
    pub fn is_fainted(&self) -> bool {
        self.current_hp <= 0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TeamState {
    pub party: Vec<CombatPokemon>,
    pub active: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BattleState {
    pub turn: u16,
    pub teams: [TeamState; 2],
    pub pending: [Option<BattleAction>; 2],
    pub weather: Option<WeatherState>,
    pub winner: Option<SideId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StepResult {
    pub state: BattleState,
    pub rng: RngState,
    pub log: BattleLog,
    pub next_request: Request,
}

#[derive(Debug, Eq, PartialEq)]
pub enum BattleError {
    EmptyTeam,
    InvalidActionOrder { expected: SideId, got: SideId },
    InvalidMoveIndex { side: SideId, index: usize },
    InvalidSwitchIndex { side: SideId, index: usize },
    BattleFinished,
}

impl Display for BattleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyTeam => write!(f, "team must contain at least one pokemon"),
            Self::InvalidActionOrder { expected, got } => {
                write!(f, "expected {:?} to act next, but got {:?}", expected, got)
            }
            Self::InvalidMoveIndex { side, index } => write!(f, "{:?} tried to use invalid move index {}", side, index),
            Self::InvalidSwitchIndex { side, index } => write!(f, "{:?} tried to switch to invalid slot {}", side, index),
            Self::BattleFinished => write!(f, "battle is already finished"),
        }
    }
}

impl Error for BattleError {}

pub fn initialize_battle(init: BattleInit, data: &DataPack) -> Result<BattleState, BattleError> {
    Ok(BattleState {
        turn: 1,
        teams: [team_from_template(init.player, data)?, team_from_template(init.opponent, data)?],
        pending: [None, None],
        weather: None,
        winner: None,
    })
}

pub fn requested_side(state: &BattleState) -> Request {
    if let Some(winner) = state.winner {
        return Request::Finished { winner };
    }

    if state.pending[0].is_none() {
        Request::ChooseAction { side: SideId::Player }
    } else if state.pending[1].is_none() {
        Request::ChooseAction { side: SideId::Opponent }
    } else {
        Request::ChooseAction { side: SideId::Player }
    }
}

fn team_from_template(template: TeamTemplate, data: &DataPack) -> Result<TeamState, BattleError> {
    if template.members.is_empty() {
        return Err(BattleError::EmptyTeam);
    }

    let mut party = Vec::with_capacity(template.members.len());
    for member in template.members {
        let species = data.species(member.species);
        let stats = resolve_battle_stats(
            species.stats,
            member.individual_values,
            member.effort_values,
            member.nature,
            member.level,
        );
        party.push(CombatPokemon {
            nickname: member.nickname.to_string(),
            species: member.species,
            moves: member.moves,
            stats,
            current_hp: i32::from(stats.hp),
            max_hp: i32::from(stats.hp),
            status: None,
            stages: StatStages::default(),
        });
    }

    Ok(TeamState { party, active: 0 })
}

pub(crate) fn active_pokemon(state: &BattleState, side: SideId) -> &CombatPokemon {
    let team = &state.teams[side.index()];
    &team.party[team.active]
}
