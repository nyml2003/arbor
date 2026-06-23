use battle_core::{BattleAction, BattleState, Request, SideId, requested_side};
use battle_data::{DataPack, ElementType, MoveId, StatusCondition, WeatherKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViewerProfile {
    LocalPlayer(SideId),
    Spectator,
    Agent(SideId),
    Debug,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionKind {
    Move,
    Switch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionDescriptor {
    pub index: usize,
    pub action: BattleAction,
    pub token: String,
    pub name: String,
    pub kind: ActionKind,
    pub element_type: Option<ElementType>,
    pub power: Option<u16>,
    pub move_id: Option<MoveId>,
    pub switch_slot: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SideSnapshot {
    pub side: SideId,
    pub active_name: String,
    pub species_name: String,
    pub primary_type: ElementType,
    pub secondary_type: Option<ElementType>,
    pub current_hp: i32,
    pub max_hp: i32,
    pub status: Option<StatusCondition>,
    pub alive_count: usize,
    pub party_size: usize,
    pub is_waiting: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BattleSnapshot {
    pub battle_id: String,
    pub viewer: ViewerProfile,
    pub turn: u16,
    pub request: Request,
    pub weather: Option<(WeatherKind, u8)>,
    pub seed: u64,
    pub player: SideSnapshot,
    pub opponent: SideSnapshot,
    pub legal_actions: Vec<ActionDescriptor>,
}

pub fn build_battle_snapshot(
    state: &BattleState,
    data: &DataPack,
    battle_id: &str,
    seed: u64,
    legal_actions: &[BattleAction],
    viewer: ViewerProfile,
) -> BattleSnapshot {
    let request = requested_side(state);
    let action_owner = viewer_action_side(viewer, request);
    BattleSnapshot {
        battle_id: battle_id.to_string(),
        viewer,
        turn: state.turn,
        request,
        weather: state.weather.map(|weather| (weather.kind, weather.remaining_turns)),
        seed,
        player: side_snapshot(state, SideId::Player, data),
        opponent: side_snapshot(state, SideId::Opponent, data),
        legal_actions: legal_actions
            .iter()
            .enumerate()
            .map(|(index, action)| action_descriptor(*action, index, state, data, action_owner))
            .collect(),
    }
}

fn side_snapshot(state: &BattleState, side: SideId, data: &DataPack) -> SideSnapshot {
    let team = &state.teams[side.index()];
    let active = &team.party[team.active];
    let species = data.species(active.species);
    let alive_count = team.party.iter().filter(|pokemon| !pokemon.is_fainted()).count();

    SideSnapshot {
        side,
        active_name: active.nickname.clone(),
        species_name: species.name.to_string(),
        primary_type: species.primary_type,
        secondary_type: species.secondary_type,
        current_hp: active.current_hp.max(0),
        max_hp: active.max_hp,
        status: active.status,
        alive_count,
        party_size: team.party.len(),
        is_waiting: matches!(requested_side(state), Request::ChooseAction { side: request_side } if request_side == side),
    }
}

fn action_descriptor(
    action: BattleAction,
    index: usize,
    state: &BattleState,
    data: &DataPack,
    action_owner: SideId,
) -> ActionDescriptor {
    match action {
        BattleAction::UseMove(move_index) => {
            let team = &state.teams[action_owner.index()];
            let active = &team.party[team.active];
            let move_id = active.moves[move_index];
            let move_def = data.move_def(move_id);
            ActionDescriptor {
                index,
                action,
                token: format!("M{}", move_index + 1),
                name: move_def.name.to_string(),
                kind: ActionKind::Move,
                element_type: Some(move_def.element_type),
                power: Some(move_def.power),
                move_id: Some(move_id),
                switch_slot: None,
            }
        }
        BattleAction::Switch(slot) => {
            let pokemon = &state.teams[action_owner.index()].party[slot];
            ActionDescriptor {
                index,
                action,
                token: format!("S{}", slot + 1),
                name: pokemon.nickname.clone(),
                kind: ActionKind::Switch,
                element_type: None,
                power: None,
                move_id: None,
                switch_slot: Some(slot),
            }
        }
    }
}

fn viewer_action_side(viewer: ViewerProfile, request: Request) -> SideId {
    match viewer {
        ViewerProfile::LocalPlayer(side) | ViewerProfile::Agent(side) => side,
        ViewerProfile::Spectator | ViewerProfile::Debug => match request {
            Request::ChooseAction { side } => side,
            Request::Finished { .. } => SideId::Player,
        },
    }
}
