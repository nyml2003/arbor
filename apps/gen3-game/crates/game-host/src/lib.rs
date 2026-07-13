//! Battle application composition shared by the native host and end-to-end tests.

#![forbid(unsafe_code)]

mod roster;

use std::{
    collections::VecDeque,
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use battle_application::{
    Action, BattleApplication, BattleError, BattleEvent, BattleObservation, BattleOutcome,
    BattlePerspective, BattlePhase, PokemonId, Side, TypeEffectiveness, UsedMove,
};
use game_data::{CurrentDataSet, DataLoadError};
use game_ui::{
    BattleAnimation, BattleDisplayState, BattleSpriteResources, BattleUiOutcome, BattleUiState,
    GameView, WorldAnimation, phase_message, project_battle, project_world_presented,
    world_command_for_key,
};
use punctum_gpu::PixelOffset;
use punctum_input::{KeyEvent, KeyPhase, LogicalKey, NamedKey};
use world_application::{Direction, WorldApplication, WorldError, WorldEvent, WorldObservation};

pub use roster::DemoSpriteManifest;

const DEFAULT_ROSTER_SEED: u64 = 0xA2B3_C4D5_1020_3040;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameScene {
    World,
    Battle,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameError {
    World(WorldError),
    Battle(BattleError),
    Setup(DemoSetupError),
    PlayerActionUnavailable,
    WrongScene {
        expected: GameScene,
        actual: GameScene,
    },
}

impl From<WorldError> for GameError {
    fn from(error: WorldError) -> Self {
        Self::World(error)
    }
}

impl From<BattleError> for GameError {
    fn from(error: BattleError) -> Self {
        Self::Battle(error)
    }
}

impl From<DemoSetupError> for GameError {
    fn from(error: DemoSetupError) -> Self {
        Self::Setup(error)
    }
}

impl From<roster::RosterError> for GameError {
    fn from(error: roster::RosterError) -> Self {
        Self::Setup(error.into())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DemoSetupError {
    Data(DataLoadError),
    Roster(roster::RosterError),
    Battle(BattleError),
}

impl From<roster::RosterError> for DemoSetupError {
    fn from(error: roster::RosterError) -> Self {
        Self::Roster(error)
    }
}

impl From<BattleError> for DemoSetupError {
    fn from(error: BattleError) -> Self {
        Self::Battle(error)
    }
}

pub struct DemoGame {
    world: WorldApplication,
    battle: Option<DemoBattle>,
    scene: GameScene,
    world_message: String,
    roster_seed: u64,
}

impl DemoGame {
    pub fn new() -> Result<Self, GameError> {
        Self::new_with_seed(DEFAULT_ROSTER_SEED)
    }

    pub fn new_random() -> Result<Self, GameError> {
        Self::new_with_seed(random_roster_seed())
    }

    pub fn new_with_seed(roster_seed: u64) -> Result<Self, GameError> {
        roster::demo_teams(demo_data()?, roster_seed).map_err(DemoSetupError::from)?;
        Ok(Self {
            world: WorldApplication::demo()?,
            battle: None,
            scene: GameScene::World,
            world_message: "风吹过草地。".into(),
            roster_seed,
        })
    }

    pub fn sprite_manifest(&self) -> Result<DemoSpriteManifest, GameError> {
        roster::sprite_manifest(demo_data()?, self.roster_seed)
            .map_err(DemoSetupError::from)
            .map_err(Into::into)
    }

    pub const fn scene(&self) -> GameScene {
        self.scene
    }

    pub fn world_observation(&self) -> WorldObservation {
        self.world.observe()
    }

    pub fn view(&mut self) -> GameView {
        self.view_with_sprite_frame(0)
    }

    pub fn view_with_sprite_frame(&mut self, sprite_frame: usize) -> GameView {
        self.view_with_animation(sprite_frame, WorldAnimation::Stand)
    }

    pub fn view_with_animation(
        &mut self,
        sprite_frame: usize,
        world_animation: WorldAnimation,
    ) -> GameView {
        self.view_with_presentation(sprite_frame, world_animation, PixelOffset::new(0, 0))
    }

    pub fn view_with_presentation(
        &mut self,
        sprite_frame: usize,
        world_animation: WorldAnimation,
        world_pixel_offset: PixelOffset,
    ) -> GameView {
        match self.scene {
            GameScene::World => project_world_presented(
                &self.world.observe(),
                &self.world_message,
                world_animation,
                sprite_frame,
                world_pixel_offset,
            ),
            GameScene::Battle => self
                .battle
                .as_mut()
                .expect("the battle scene owns a battle")
                .view_with_sprite_frame(sprite_frame),
        }
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> Result<bool, GameError> {
        match self.scene {
            GameScene::World => {
                let Some(command) = world_command_for_key(key) else {
                    return Ok(false);
                };
                let world_application::WorldCommand::Move(direction) = command else {
                    return Ok(false);
                };
                self.step_world(direction)?;
                Ok(true)
            }
            GameScene::Battle => {
                let battle = self
                    .battle
                    .as_mut()
                    .expect("the battle scene owns a battle");
                if battle.is_finished() && !battle.is_playing() && is_enter_press(key) {
                    self.battle = None;
                    self.scene = GameScene::World;
                    self.world_message = "战斗结束，回到了原野。".into();
                    return Ok(true);
                }
                battle.handle_key(key).map_err(Into::into)
            }
        }
    }

    pub fn move_world(&mut self, direction: Direction) -> Result<WorldEvent, GameError> {
        if self.scene != GameScene::World {
            return Err(GameError::WrongScene {
                expected: GameScene::World,
                actual: self.scene,
            });
        }
        let outcome = self
            .world
            .submit(world_application::WorldCommand::Move(direction));
        let event = outcome.event();
        self.world_message = match event {
            WorldEvent::Turned { .. } => "风吹过草地。",
            WorldEvent::Moved { .. } => "风吹过草地。",
            WorldEvent::Blocked { .. } => "前面过不去。",
            WorldEvent::EncounterTriggered { .. } => "草丛里有动静！",
        }
        .into();
        if outcome.starts_battle() {
            self.battle = Some(DemoBattle::new_with_seed(self.roster_seed)?);
            self.scene = GameScene::Battle;
        }
        Ok(event)
    }

    pub fn step_world(&mut self, direction: Direction) -> Result<WorldEvent, GameError> {
        if self.world_observation().facing() == direction {
            self.move_world(direction)
        } else {
            self.face_world(direction)
        }
    }

    pub fn face_world(&mut self, direction: Direction) -> Result<WorldEvent, GameError> {
        if self.scene != GameScene::World {
            return Err(GameError::WrongScene {
                expected: GameScene::World,
                actual: self.scene,
            });
        }
        let event = self
            .world
            .submit(world_application::WorldCommand::Face(direction))
            .event();
        Ok(event)
    }

    pub fn has_pending_playback(&self) -> bool {
        self.battle
            .as_ref()
            .is_some_and(DemoBattle::has_pending_playback)
    }

    pub fn legal_player_actions(&self) -> Vec<Action> {
        self.battle
            .as_ref()
            .filter(|battle| !battle.is_playing() && !battle.is_finished())
            .map_or_else(Vec::new, DemoBattle::legal_actions)
    }

    pub fn submit_player_action(&mut self, action: Action) -> Result<(), GameError> {
        if self.scene != GameScene::Battle {
            return Err(GameError::WrongScene {
                expected: GameScene::Battle,
                actual: self.scene,
            });
        }
        let battle = self
            .battle
            .as_mut()
            .expect("the battle scene owns a battle");
        if battle.is_playing() || battle.is_finished() {
            return Err(GameError::PlayerActionUnavailable);
        }
        battle.submit_player(action).map_err(Into::into)
    }

    pub fn advance_playback(&mut self) -> bool {
        self.battle
            .as_mut()
            .is_some_and(DemoBattle::advance_playback)
    }
}

fn is_enter_press(key: &KeyEvent) -> bool {
    key.phase == KeyPhase::Press && key.logical == LogicalKey::Named(NamedKey::Enter)
}

pub struct DemoBattle {
    application: BattleApplication,
    player: BattlePerspective,
    opponent: BattlePerspective,
    ui: BattleUiState,
    message: String,
    animation: BattleAnimation,
    display: BattleDisplayState,
    playback: VecDeque<PlaybackFrame>,
    opponent_sprite_ids: Vec<PokemonId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PlaybackFrame {
    message: String,
    animation: BattleAnimation,
    display: BattleDisplayState,
}

impl DemoBattle {
    pub fn new() -> Result<Self, DemoSetupError> {
        Self::new_with_seed(DEFAULT_ROSTER_SEED)
    }

    fn new_with_seed(roster_seed: u64) -> Result<Self, DemoSetupError> {
        let data = demo_data()?;
        let (player_team, opponent_team) = roster::demo_teams(data, roster_seed)?;
        let opponent_sprite_ids = opponent_team
            .members()
            .iter()
            .map(|pokemon| pokemon.id().clone())
            .collect();
        let application =
            BattleApplication::new(player_team, opponent_team, roster_seed ^ 0xA2B3_C4D5)?;
        let (player, opponent) = application.perspectives();
        let display = display_from_observation(&application.observe(&player));
        Ok(Self {
            application,
            player,
            opponent,
            ui: BattleUiState::default(),
            message: "请选择行动".into(),
            animation: BattleAnimation::Idle,
            display,
            playback: VecDeque::new(),
            opponent_sprite_ids,
        })
    }

    pub fn observation(&self) -> BattleObservation {
        self.application.observe(&self.player)
    }

    pub fn legal_actions(&self) -> Vec<Action> {
        self.application.legal_actions(&self.player)
    }

    pub fn view(&mut self) -> GameView {
        self.view_with_sprite_frame(0)
    }

    pub fn view_with_sprite_frame(&mut self, sprite_frame: usize) -> GameView {
        let observation = self.observation();
        let actions = if self.is_playing() {
            Vec::new()
        } else {
            self.legal_actions()
        };
        self.ui.reconcile(&actions);
        let opponent_slot = self
            .opponent_sprite_ids
            .iter()
            .position(|id| id == observation.opponent().active().id())
            .expect("the active opponent belongs to the generated roster");
        project_battle(
            &observation,
            &actions,
            self.ui,
            &self.message,
            self.animation,
            &self.display,
            BattleSpriteResources::for_slots(
                observation.own().active_slot().index(),
                opponent_slot,
            ),
            sprite_frame,
        )
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> Result<bool, BattleError> {
        if self.is_playing() {
            return Ok(false);
        }
        let actions = self.legal_actions();
        match self.ui.handle_key(key, &actions) {
            BattleUiOutcome::Updated => Ok(true),
            BattleUiOutcome::Submit(action) => {
                self.submit_player(action)?;
                Ok(true)
            }
            BattleUiOutcome::Ignored => Ok(false),
        }
    }

    pub fn submit_player(&mut self, action: Action) -> Result<(), BattleError> {
        self.display = display_from_observation(&self.observation());
        self.playback.clear();
        self.animation = BattleAnimation::Idle;
        let mut events = Vec::new();
        let outcome = self.application.submit(&self.player, action)?;
        events.extend_from_slice(outcome.events());
        if outcome.is_waiting_for_opponent() {
            events.extend(self.submit_opponent()?);
        }
        events.extend(self.resolve_opponent_replacement()?);
        self.start_playback(&events);
        self.ui.reconcile(&self.legal_actions());
        Ok(())
    }

    pub fn has_pending_playback(&self) -> bool {
        !self.playback.is_empty()
    }

    pub fn advance_playback(&mut self) -> bool {
        let Some(frame) = self.playback.pop_front() else {
            return false;
        };
        self.message = frame.message;
        self.animation = frame.animation;
        self.display = frame.display;
        true
    }

    pub fn is_finished(&self) -> bool {
        matches!(self.observation().phase(), BattlePhase::Finished(_))
    }

    fn is_playing(&self) -> bool {
        self.has_pending_playback() || self.animation != BattleAnimation::Idle
    }

    fn submit_opponent(&mut self) -> Result<Vec<BattleEvent>, BattleError> {
        let Some(action) = choose_opponent_action(self.application.legal_actions(&self.opponent))
        else {
            return Ok(Vec::new());
        };
        let outcome = self.application.submit(&self.opponent, action)?;
        Ok(outcome.events().to_vec())
    }

    fn resolve_opponent_replacement(&mut self) -> Result<Vec<BattleEvent>, BattleError> {
        let mut events = Vec::new();
        loop {
            let phase = self.application.observe(&self.opponent).phase();
            if !phase.requires_replacement(Side::Two) || phase.requires_replacement(Side::One) {
                return Ok(events);
            }
            events.extend(self.submit_opponent()?);
        }
    }

    fn start_playback(&mut self, events: &[BattleEvent]) {
        let mut display = self.display.clone();
        let mut frames = events
            .iter()
            .filter_map(|event| self.event_frame(event, &mut display))
            .collect::<VecDeque<_>>();
        frames.push_back(PlaybackFrame {
            message: phase_message(self.observation().phase()).into(),
            animation: BattleAnimation::Idle,
            display: display_from_observation(&self.observation()),
        });
        if let Some(frame) = frames.pop_front() {
            self.message = frame.message;
            self.animation = frame.animation;
            self.display = frame.display;
        }
        self.playback = frames;
    }

    fn event_frame(
        &self,
        event: &BattleEvent,
        display: &mut BattleDisplayState,
    ) -> Option<PlaybackFrame> {
        let (message, animation) = match event {
            BattleEvent::TurnStarted { turn } => (format!("第 {turn} 回合"), BattleAnimation::Idle),
            BattleEvent::MoveUsed {
                side,
                pokemon,
                used_move,
            } => (
                format!(
                    "{} 使用了 {}！",
                    self.pokemon_name(pokemon),
                    self.move_name(used_move),
                ),
                BattleAnimation::Acting(*side),
            ),
            BattleEvent::Damage {
                target_side,
                target,
                amount,
                ..
            } => (
                format!("{} 受到 {} 点伤害。", self.pokemon_name(target), amount),
                BattleAnimation::Hit(*target_side),
            ),
            BattleEvent::Missed { .. } => ("攻击没有命中。".into(), BattleAnimation::Idle),
            BattleEvent::Critical { target_side, .. } => {
                ("会心一击！".into(), BattleAnimation::Hit(*target_side))
            }
            BattleEvent::Effectiveness { effectiveness, .. } => (
                effectiveness_message(*effectiveness).into(),
                BattleAnimation::Idle,
            ),
            BattleEvent::Fainted { side, pokemon } => (
                format!("{} 倒下了。", self.pokemon_name(pokemon)),
                BattleAnimation::Fainted(*side),
            ),
            BattleEvent::ForcedReplacement { .. } => {
                ("请选择下一只精灵".into(), BattleAnimation::Idle)
            }
            BattleEvent::OwnSwitched { pokemon, .. }
            | BattleEvent::OpponentSwitched { pokemon } => (
                format!("{} 上场了。", self.pokemon_name(pokemon)),
                BattleAnimation::Idle,
            ),
            BattleEvent::BattleFinished { outcome } => (
                match outcome {
                    BattleOutcome::Winner(Side::One) => "你赢了！",
                    BattleOutcome::Winner(Side::Two) => "对手赢了。",
                    BattleOutcome::Draw => "战斗平局。",
                }
                .into(),
                BattleAnimation::Idle,
            ),
            BattleEvent::OwnCommandAccepted { .. }
            | BattleEvent::OpponentCommandCommitted
            | BattleEvent::OwnPpSpent { .. } => return None,
        };
        match event {
            BattleEvent::Damage {
                target_side,
                remaining_hp,
                ..
            } => match target_side {
                Side::One => display.own_hp = *remaining_hp,
                Side::Two => display.opponent_hp = *remaining_hp,
            },
            BattleEvent::OwnSwitched { pokemon, .. } => {
                update_own_display(display, &self.observation(), pokemon);
            }
            BattleEvent::OpponentSwitched { pokemon } => {
                update_opponent_display(display, &self.observation(), pokemon);
            }
            _ => {}
        }
        Some(PlaybackFrame {
            message,
            animation,
            display: display.clone(),
        })
    }

    fn pokemon_name(&self, id: &battle_application::PokemonId) -> String {
        let observation = self.observation();
        observation
            .own()
            .members()
            .iter()
            .find(|pokemon| pokemon.id() == id)
            .map(|pokemon| pokemon.name().to_owned())
            .or_else(|| {
                let opponent = observation.opponent();
                opponent
                    .active()
                    .id()
                    .eq(id)
                    .then(|| opponent.active().name().to_owned())
                    .or_else(|| {
                        opponent
                            .revealed_bench()
                            .iter()
                            .find(|pokemon| pokemon.id() == id)
                            .map(|pokemon| pokemon.name().to_owned())
                    })
            })
            .unwrap_or_else(|| id.as_str().to_owned())
    }

    fn move_name(&self, used_move: &UsedMove) -> String {
        let UsedMove::Move { id } = used_move else {
            return "挣扎".into();
        };
        let observation = self.observation();
        observation
            .own()
            .members()
            .iter()
            .flat_map(|pokemon| pokemon.moves())
            .find(|battle_move| battle_move.id() == id)
            .map(|battle_move| battle_move.name().to_owned())
            .or_else(|| {
                let opponent = observation.opponent();
                opponent
                    .active()
                    .revealed_moves()
                    .iter()
                    .find(|battle_move| battle_move.id() == id)
                    .map(|battle_move| battle_move.name().to_owned())
            })
            .unwrap_or_else(|| id.as_str().to_owned())
    }
}

fn display_from_observation(observation: &BattleObservation) -> BattleDisplayState {
    let own = &observation.own().members()[observation.own().active_slot().index()];
    let opponent = observation.opponent().active();
    BattleDisplayState {
        own_name: own.name().into(),
        own_hp: own.current_hp(),
        own_max_hp: own.max_hp(),
        opponent_name: opponent.name().into(),
        opponent_hp: opponent.current_hp(),
        opponent_max_hp: opponent.max_hp(),
    }
}

fn update_own_display(
    display: &mut BattleDisplayState,
    observation: &BattleObservation,
    id: &PokemonId,
) {
    if let Some(pokemon) = observation
        .own()
        .members()
        .iter()
        .find(|pokemon| pokemon.id() == id)
    {
        display.own_name = pokemon.name().into();
        display.own_hp = pokemon.current_hp();
        display.own_max_hp = pokemon.max_hp();
    }
}

fn update_opponent_display(
    display: &mut BattleDisplayState,
    observation: &BattleObservation,
    id: &PokemonId,
) {
    let opponent = observation.opponent();
    let pokemon = std::iter::once(opponent.active())
        .chain(opponent.revealed_bench().iter())
        .find(|pokemon| pokemon.id() == id);
    if let Some(pokemon) = pokemon {
        display.opponent_name = pokemon.name().into();
        display.opponent_hp = pokemon.current_hp();
        display.opponent_max_hp = pokemon.max_hp();
    }
}

fn choose_opponent_action(actions: Vec<Action>) -> Option<Action> {
    actions
        .iter()
        .copied()
        .find(|action| matches!(action, Action::UseMove(_)))
        .or_else(|| actions.first().copied())
}

fn effectiveness_message(effectiveness: TypeEffectiveness) -> &'static str {
    match effectiveness {
        TypeEffectiveness::Immune => "没有效果。",
        TypeEffectiveness::Quarter | TypeEffectiveness::Half => "效果不太好……",
        TypeEffectiveness::Normal => "命中了。",
        TypeEffectiveness::Double | TypeEffectiveness::Quadruple => "效果绝佳！",
    }
}

fn demo_data() -> Result<&'static CurrentDataSet, DemoSetupError> {
    static DATA: OnceLock<Result<CurrentDataSet, DataLoadError>> = OnceLock::new();
    DATA.get_or_init(CurrentDataSet::embedded)
        .as_ref()
        .map_err(|error| DemoSetupError::Data(error.clone()))
}

fn random_roster_seed() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    (elapsed.as_nanos() as u64) ^ COUNTER.fetch_add(1, Ordering::Relaxed).rotate_left(17)
}

#[cfg(test)]
mod tests {
    use battle_application::Action;
    use game_ui::{
        CANVAS_HEIGHT, CANVAS_WIDTH, TextRole, opponent_front_resource, player_back_resource,
    };
    use punctum_grid::{GridPos, GridRect, GridSize};

    use super::DemoBattle;

    #[test]
    fn submitting_a_move_resolves_both_sides_and_updates_the_observation() {
        let mut battle = DemoBattle::new().unwrap();
        let before = battle.observation();
        let action = battle
            .legal_actions()
            .into_iter()
            .find(|action| matches!(action, Action::UseMove(_)))
            .unwrap();

        battle.submit_player(action).unwrap();
        let after = battle.observation();

        assert_eq!(after.turn(), before.turn() + 1);
        let Action::UseMove(slot) = action else {
            unreachable!("the selected action is a move")
        };
        let before_pp = before.own().members()[before.own().active_slot().index()].moves()
            [slot.index()]
        .current_pp();
        let after_pp = after.own().members()[after.own().active_slot().index()].moves()
            [slot.index()]
        .current_pp();
        assert_eq!(after_pp + 1, before_pp);
        let mut messages = Vec::new();
        while battle.has_pending_playback() {
            battle.advance_playback();
            messages.push(
                battle
                    .view()
                    .labels()
                    .iter()
                    .find(|label| label.role == TextRole::Message)
                    .unwrap()
                    .content
                    .clone(),
            );
        }
        assert!(messages.iter().any(|message| message.contains("使用了")));
    }

    #[test]
    fn displayed_hp_changes_only_when_the_damage_frame_is_presented() {
        let mut battle = DemoBattle::new().unwrap();
        let initial_display = battle.display.clone();
        let action = battle
            .legal_actions()
            .into_iter()
            .find(|action| matches!(action, Action::UseMove(_)))
            .unwrap();

        battle.submit_player(action).unwrap();

        assert_eq!(battle.display, initial_display);
        while battle.has_pending_playback() {
            battle.advance_playback();
        }
        let final_display = super::display_from_observation(&battle.observation());
        assert_eq!(battle.display, final_display);
    }

    #[test]
    fn projected_battle_view_contains_the_fixed_canvas_status_and_actions() {
        let mut battle = DemoBattle::new().unwrap();
        let observation = battle.observation();
        let active = &observation.own().members()[observation.own().active_slot().index()];
        assert_eq!(active.moves().len(), 4);
        let view = battle.view();

        assert_eq!(
            view.surface().size(),
            GridSize::new(CANVAS_WIDTH, CANVAS_HEIGHT)
        );
        assert!(
            view.labels()
                .iter()
                .any(|label| label.role == TextRole::OpponentName
                    && label.content == observation.opponent().active().name())
        );
        assert!(
            view.labels()
                .iter()
                .filter(|label| matches!(label.role, TextRole::Action(_)))
                .count()
                == 4
        );
        assert!(view.labels().iter().any(|label| {
            matches!(label.role, TextRole::Action(_))
                && label.content.starts_with(active.moves()[0].name())
        }));
        assert_eq!(view.images().len(), 2);
        assert_eq!(view.images()[0].resource, player_back_resource(0, 0));
        assert_eq!(
            view.images()[0].bounds,
            GridRect::new(GridPos::new(5, 10), GridSize::new(8, 8))
        );
        assert_eq!(view.images()[1].resource, opponent_front_resource(0, 0));
        assert_eq!(
            view.images()[1].bounds,
            GridRect::new(GridPos::new(22, 4), GridSize::new(8, 8))
        );

        let next_frame = battle.view_with_sprite_frame(1);
        assert_eq!(next_frame.images()[0].resource, player_back_resource(0, 1));
        assert_eq!(
            next_frame.images()[1].resource,
            opponent_front_resource(0, 1)
        );
    }

    #[test]
    fn switching_the_active_pokemon_switches_the_back_sprite_resource() {
        let mut battle = DemoBattle::new().unwrap();
        let action = battle
            .legal_actions()
            .into_iter()
            .find(|action| matches!(action, Action::Switch(_)))
            .expect("the generated team has a legal bench switch");
        let Action::Switch(slot) = action else {
            unreachable!("the selected action is a switch")
        };

        battle.submit_player(action).unwrap();
        let view = battle.view_with_sprite_frame(0);

        assert_eq!(view.images()[0].resource, player_back_resource(slot.index(), 0));
    }
}
