use battle_domain::{
    Action, Battle, BattleEvent as DomainEvent, BattlePhase, DamageSource as DomainDamageSource,
    Move, MoveId, MoveSlot, Pokemon, PokemonId, PokemonType, Side,
    SubmitOutcome as DomainSubmitOutcome, TEAM_SIZE, TeamSlot, TypeEffectiveness,
    UsedMove as DomainUsedMove,
};

use crate::Accuracy;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BattleObservation {
    viewer: Side,
    turn: u32,
    phase: BattlePhase,
    own: OwnSideObservation,
    opponent: OpponentSideObservation,
}

impl BattleObservation {
    pub const fn viewer(&self) -> Side {
        self.viewer
    }

    pub const fn turn(&self) -> u32 {
        self.turn
    }

    pub const fn phase(&self) -> BattlePhase {
        self.phase
    }

    pub const fn own(&self) -> &OwnSideObservation {
        &self.own
    }

    pub const fn opponent(&self) -> &OpponentSideObservation {
        &self.opponent
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnSideObservation {
    active_slot: TeamSlot,
    members: [Pokemon; TEAM_SIZE],
}

impl OwnSideObservation {
    pub const fn active_slot(&self) -> TeamSlot {
        self.active_slot
    }

    pub const fn members(&self) -> &[Pokemon; TEAM_SIZE] {
        &self.members
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpponentSideObservation {
    active: RevealedPokemonObservation,
    revealed_bench: Vec<RevealedPokemonObservation>,
    unrevealed_count: usize,
}

impl OpponentSideObservation {
    pub const fn active(&self) -> &RevealedPokemonObservation {
        &self.active
    }

    pub fn revealed_bench(&self) -> &[RevealedPokemonObservation] {
        &self.revealed_bench
    }

    pub const fn unrevealed_count(&self) -> usize {
        self.unrevealed_count
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevealedPokemonObservation {
    id: PokemonId,
    name: String,
    level: u8,
    primary_type: PokemonType,
    secondary_type: Option<PokemonType>,
    max_hp: u32,
    current_hp: u32,
    revealed_moves: Vec<RevealedMoveObservation>,
}

impl RevealedPokemonObservation {
    pub fn id(&self) -> &PokemonId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn level(&self) -> u8 {
        self.level
    }

    pub const fn primary_type(&self) -> PokemonType {
        self.primary_type
    }

    pub const fn secondary_type(&self) -> Option<PokemonType> {
        self.secondary_type
    }

    pub const fn max_hp(&self) -> u32 {
        self.max_hp
    }

    pub const fn current_hp(&self) -> u32 {
        self.current_hp
    }

    pub const fn is_fainted(&self) -> bool {
        self.current_hp == 0
    }

    pub fn revealed_moves(&self) -> &[RevealedMoveObservation] {
        &self.revealed_moves
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevealedMoveObservation {
    id: MoveId,
    name: String,
    move_type: PokemonType,
    power: u16,
    accuracy: Accuracy,
    priority: i8,
}

impl RevealedMoveObservation {
    pub fn id(&self) -> &MoveId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn move_type(&self) -> PokemonType {
        self.move_type
    }

    pub const fn power(&self) -> u16 {
        self.power
    }

    pub const fn accuracy(&self) -> Accuracy {
        self.accuracy
    }

    pub const fn priority(&self) -> i8 {
        self.priority
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UsedMove {
    Move { id: MoveId },
    Struggle,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DamageSource {
    Move {
        side: Side,
        pokemon: PokemonId,
        used_move: UsedMove,
    },
    Recoil {
        side: Side,
        pokemon: PokemonId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BattleEvent {
    OwnCommandAccepted {
        action: Action,
    },
    OpponentCommandCommitted,
    TurnStarted {
        turn: u32,
    },
    OwnSwitched {
        from: TeamSlot,
        to: TeamSlot,
        pokemon: PokemonId,
    },
    OpponentSwitched {
        pokemon: PokemonId,
    },
    MoveUsed {
        side: Side,
        pokemon: PokemonId,
        used_move: UsedMove,
    },
    OwnPpSpent {
        pokemon: PokemonId,
        move_slot: MoveSlot,
        remaining: u8,
    },
    Missed {
        side: Side,
        target_side: Side,
        target: PokemonId,
    },
    Critical {
        side: Side,
        target_side: Side,
        target: PokemonId,
    },
    Effectiveness {
        side: Side,
        target_side: Side,
        target: PokemonId,
        effectiveness: TypeEffectiveness,
    },
    Damage {
        source: DamageSource,
        target_side: Side,
        target: PokemonId,
        amount: u32,
        remaining_hp: u32,
    },
    Fainted {
        side: Side,
        pokemon: PokemonId,
    },
    ForcedReplacement {
        side: Side,
    },
    BattleFinished {
        outcome: crate::BattleOutcome,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubmitOutcome {
    events: Vec<BattleEvent>,
    phase: BattlePhase,
    waiting_for_opponent: bool,
}

impl SubmitOutcome {
    pub fn events(&self) -> &[BattleEvent] {
        &self.events
    }

    pub const fn phase(&self) -> BattlePhase {
        self.phase
    }

    pub const fn is_waiting_for_opponent(&self) -> bool {
        self.waiting_for_opponent
    }

    pub(crate) fn from_domain(outcome: DomainSubmitOutcome, viewer: Side) -> Self {
        Self {
            events: observe_events(outcome.events(), viewer),
            phase: outcome.phase(),
            waiting_for_opponent: outcome.is_waiting_for_opponent(),
        }
    }
}

pub(crate) fn observe(battle: &Battle, viewer: Side) -> BattleObservation {
    let opponent = viewer.opponent();
    BattleObservation {
        viewer,
        turn: battle.turn_number(),
        phase: battle.phase(),
        own: OwnSideObservation {
            active_slot: battle.active_slot(viewer),
            members: battle.team(viewer).members().clone(),
        },
        opponent: opponent_observation(battle, opponent),
    }
}

pub(crate) fn event_log(battle: &Battle, viewer: Side) -> Vec<BattleEvent> {
    observe_events(battle.events(), viewer)
}

fn opponent_observation(battle: &Battle, opponent: Side) -> OpponentSideObservation {
    let active = battle.active(opponent);
    let revealed = revealed_pokemon_ids(battle, opponent);
    let revealed_bench = revealed
        .iter()
        .filter(|id| *id != active.id())
        .map(|id| revealed_pokemon(battle, opponent, id))
        .collect();
    OpponentSideObservation {
        active: revealed_pokemon(battle, opponent, active.id()),
        revealed_bench,
        unrevealed_count: TEAM_SIZE - revealed.len(),
    }
}

fn revealed_pokemon_ids(battle: &Battle, side: Side) -> Vec<PokemonId> {
    let mut revealed = Vec::new();
    for event in battle.events() {
        match event {
            DomainEvent::Switched {
                side: event_side,
                from,
                pokemon,
                ..
            } if *event_side == side => {
                push_unique(&mut revealed, battle.team(side).member(*from).id().clone());
                push_unique(&mut revealed, pokemon.clone());
            }
            DomainEvent::MoveUsed {
                side: event_side,
                pokemon,
                ..
            }
            | DomainEvent::Fainted {
                side: event_side,
                pokemon,
            } if *event_side == side => push_unique(&mut revealed, pokemon.clone()),
            _ => {}
        }
    }
    push_unique(&mut revealed, battle.active(side).id().clone());
    revealed
}

fn push_unique(revealed: &mut Vec<PokemonId>, pokemon: PokemonId) {
    if !revealed.contains(&pokemon) {
        revealed.push(pokemon);
    }
}

fn revealed_pokemon(
    battle: &Battle,
    side: Side,
    pokemon_id: &PokemonId,
) -> RevealedPokemonObservation {
    let pokemon = battle
        .team(side)
        .members()
        .iter()
        .find(|pokemon| pokemon.id() == pokemon_id)
        .expect("a revealed pokemon belongs to the observed team");
    RevealedPokemonObservation {
        id: pokemon.id().clone(),
        name: pokemon.name().to_owned(),
        level: pokemon.level(),
        primary_type: pokemon.primary_type(),
        secondary_type: pokemon.secondary_type(),
        max_hp: pokemon.max_hp(),
        current_hp: pokemon.current_hp(),
        revealed_moves: revealed_moves(battle, side, pokemon),
    }
}

fn revealed_moves(battle: &Battle, side: Side, pokemon: &Pokemon) -> Vec<RevealedMoveObservation> {
    pokemon
        .moves()
        .iter()
        .enumerate()
        .filter_map(|(index, battle_move)| {
            let slot = MoveSlot::new(index).expect("move index is within the move set");
            move_was_used(battle, side, pokemon.id(), slot).then(|| reveal_move(battle_move))
        })
        .collect()
}

fn move_was_used(battle: &Battle, side: Side, pokemon: &PokemonId, slot: MoveSlot) -> bool {
    battle.events().iter().any(|event| {
        matches!(
            event,
            DomainEvent::MoveUsed {
                side: event_side,
                pokemon: event_pokemon,
                used_move: DomainUsedMove::Move { slot: event_slot, .. },
            } if *event_side == side && event_pokemon == pokemon && *event_slot == slot
        )
    })
}

fn reveal_move(battle_move: &Move) -> RevealedMoveObservation {
    RevealedMoveObservation {
        id: battle_move.id().clone(),
        name: battle_move.name().to_owned(),
        move_type: battle_move.move_type(),
        power: battle_move.power(),
        accuracy: battle_move.accuracy(),
        priority: battle_move.priority(),
    }
}

fn observe_events(events: &[DomainEvent], viewer: Side) -> Vec<BattleEvent> {
    events
        .iter()
        .filter_map(|event| observe_event(event, viewer))
        .collect()
}

fn observe_event(event: &DomainEvent, viewer: Side) -> Option<BattleEvent> {
    Some(match event {
        DomainEvent::CommandAccepted { side, action } if *side == viewer => {
            BattleEvent::OwnCommandAccepted { action: *action }
        }
        DomainEvent::CommandAccepted { .. } => BattleEvent::OpponentCommandCommitted,
        DomainEvent::TurnStarted { turn } => BattleEvent::TurnStarted { turn: *turn },
        DomainEvent::Switched {
            side,
            from,
            to,
            pokemon,
        } if *side == viewer => BattleEvent::OwnSwitched {
            from: *from,
            to: *to,
            pokemon: pokemon.clone(),
        },
        DomainEvent::Switched { pokemon, .. } => BattleEvent::OpponentSwitched {
            pokemon: pokemon.clone(),
        },
        DomainEvent::MoveUsed {
            side,
            pokemon,
            used_move,
        } => BattleEvent::MoveUsed {
            side: *side,
            pokemon: pokemon.clone(),
            used_move: observe_used_move(used_move),
        },
        DomainEvent::PpSpent { side, .. } if *side != viewer => return None,
        DomainEvent::PpSpent {
            side: _,
            pokemon,
            move_slot,
            remaining,
        } => BattleEvent::OwnPpSpent {
            pokemon: pokemon.clone(),
            move_slot: *move_slot,
            remaining: *remaining,
        },
        DomainEvent::Missed {
            side,
            target_side,
            target,
        } => BattleEvent::Missed {
            side: *side,
            target_side: *target_side,
            target: target.clone(),
        },
        DomainEvent::Critical {
            side,
            target_side,
            target,
        } => BattleEvent::Critical {
            side: *side,
            target_side: *target_side,
            target: target.clone(),
        },
        DomainEvent::Effectiveness {
            side,
            target_side,
            target,
            effectiveness,
        } => BattleEvent::Effectiveness {
            side: *side,
            target_side: *target_side,
            target: target.clone(),
            effectiveness: *effectiveness,
        },
        DomainEvent::Damage {
            source,
            target_side,
            target,
            amount,
            remaining_hp,
        } => BattleEvent::Damage {
            source: observe_damage_source(source),
            target_side: *target_side,
            target: target.clone(),
            amount: *amount,
            remaining_hp: *remaining_hp,
        },
        DomainEvent::Fainted { side, pokemon } => BattleEvent::Fainted {
            side: *side,
            pokemon: pokemon.clone(),
        },
        DomainEvent::ForcedReplacement { side } => BattleEvent::ForcedReplacement { side: *side },
        DomainEvent::BattleFinished { outcome } => {
            BattleEvent::BattleFinished { outcome: *outcome }
        }
    })
}

fn observe_used_move(used_move: &DomainUsedMove) -> UsedMove {
    match used_move {
        DomainUsedMove::Move { id, .. } => UsedMove::Move { id: id.clone() },
        DomainUsedMove::Struggle => UsedMove::Struggle,
    }
}

fn observe_damage_source(source: &DomainDamageSource) -> DamageSource {
    match source {
        DomainDamageSource::Move {
            side,
            pokemon,
            used_move,
        } => DamageSource::Move {
            side: *side,
            pokemon: pokemon.clone(),
            used_move: observe_used_move(used_move),
        },
        DomainDamageSource::Recoil { side, pokemon } => DamageSource::Recoil {
            side: *side,
            pokemon: pokemon.clone(),
        },
    }
}
