use serde::{Deserialize, Serialize};

use crate::ids::{MoveId, SpeciesId};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ElementType {
    Normal,
    Electric,
    Fire,
    Water,
    Grass,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum StatId {
    Attack,
    Defense,
    Speed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum StatusCondition {
    Paralyzed,
    Poisoned,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WeatherKind {
    Sunny,
    Rainy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EffectTarget {
    User,
    Opponent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MoveEffect {
    Damage,
    ApplyStatus {
        target: EffectTarget,
        status: StatusCondition,
    },
    ModifyStat {
        target: EffectTarget,
        stat: StatId,
        stages: i8,
    },
    HealPercent {
        target: EffectTarget,
        percent: u8,
    },
    SetWeather {
        weather: WeatherKind,
        turns: u8,
    },
    ForceSwitch {
        target: EffectTarget,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SpeciesBaseStats {
    pub hp: u16,
    pub attack: u16,
    pub defense: u16,
    pub speed: u16,
}

pub type BaseStats = SpeciesBaseStats;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct IndividualValues {
    pub hp: u8,
    pub attack: u8,
    pub defense: u8,
    pub speed: u8,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct EffortValues {
    pub hp: u16,
    pub attack: u16,
    pub defense: u16,
    pub speed: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum Nature {
    #[default]
    Neutral,
    Adamant,
    Bold,
    Timid,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SpeciesDef {
    pub id: SpeciesId,
    pub name: String,
    pub primary_type: ElementType,
    pub secondary_type: Option<ElementType>,
    pub stats: SpeciesBaseStats,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MoveDef {
    pub id: MoveId,
    pub name: String,
    pub element_type: ElementType,
    pub power: u16,
    pub accuracy: u8,
    pub priority: i8,
    pub effect: MoveEffect,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PokemonTemplate {
    pub nickname: String,
    pub species: SpeciesId,
    pub moves: [MoveId; 4],
    #[serde(default)]
    pub level: Option<u8>,
    #[serde(default)]
    pub individual_values: Option<IndividualValues>,
    #[serde(default)]
    pub effort_values: Option<EffortValues>,
    #[serde(default)]
    pub nature: Option<Nature>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TeamTemplate {
    pub members: Vec<PokemonTemplate>,
}
