//! Battle application composition shared by the native host and end-to-end tests.

#![forbid(unsafe_code)]

mod roster;

use std::{
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use battle_application::{Action, BattleApplication, BattleError, BattleObservation, PokemonId};
use battle_session::{BattleCoordinator, BattleSession, OpponentPolicy, SessionError};
use game_data::{CurrentDataSet, DataLoadError};
use game_ui::{
    BattleSpriteResources, BattleUiOutcome, BattleUiState, GameView, WorldAnimation,
    project_battle, project_world_presented, world_command_for_key,
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
    Battle(SessionError),
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

impl From<SessionError> for GameError {
    fn from(error: SessionError) -> Self {
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
    roster_seed: u64,
}

impl DemoGame {
    pub fn new() -> Result<Self, GameError> {
        Self::new_with_seed(DEFAULT_ROSTER_SEED)
    }

    pub fn new_random() -> Result<Self, GameError> {
        Self::new_with_seed(random_roster_seed())
    }

    pub fn new_random_with_world(world: WorldApplication) -> Result<Self, GameError> {
        Self::with_world_and_seed(world, random_roster_seed())
    }

    pub fn new_with_seed(roster_seed: u64) -> Result<Self, GameError> {
        Self::with_world_and_seed(WorldApplication::demo()?, roster_seed)
    }

    fn with_world_and_seed(world: WorldApplication, roster_seed: u64) -> Result<Self, GameError> {
        roster::demo_teams(demo_data()?, roster_seed).map_err(DemoSetupError::from)?;
        Ok(Self {
            world,
            battle: None,
            scene: GameScene::World,
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

    pub const fn world_position(&self) -> world_application::Position {
        self.world.player()
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
                if battle.is_finished() && is_enter_press(key) {
                    self.battle = None;
                    self.scene = GameScene::World;
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
    session: BattleSession<DemoOpponentPolicy>,
    ui: BattleUiState,
    own_sprite_ids: Vec<PokemonId>,
    opponent_sprite_ids: Vec<PokemonId>,
}

struct DemoOpponentPolicy;

impl OpponentPolicy for DemoOpponentPolicy {
    fn choose_action(
        &mut self,
        _observation: &BattleObservation,
        legal_actions: &[Action],
    ) -> Option<Action> {
        legal_actions
            .iter()
            .copied()
            .find(|action| matches!(action, Action::UseMove(_)))
            .or_else(|| legal_actions.first().copied())
    }
}

impl DemoBattle {
    pub fn new() -> Result<Self, DemoSetupError> {
        Self::new_with_seed(DEFAULT_ROSTER_SEED)
    }

    fn new_with_seed(roster_seed: u64) -> Result<Self, DemoSetupError> {
        let data = demo_data()?;
        let (player_team, opponent_team) = roster::demo_teams(data, roster_seed)?;
        let own_sprite_ids = player_team
            .members()
            .iter()
            .map(|pokemon| pokemon.id().clone())
            .collect();
        let opponent_sprite_ids = opponent_team
            .members()
            .iter()
            .map(|pokemon| pokemon.id().clone())
            .collect();
        let application =
            BattleApplication::new(player_team, opponent_team, roster_seed ^ 0xA2B3_C4D5)?;
        let session = BattleSession::new(BattleCoordinator::new(application, DemoOpponentPolicy));
        Ok(Self {
            session,
            ui: BattleUiState::default(),
            own_sprite_ids,
            opponent_sprite_ids,
        })
    }

    pub fn observation(&self) -> BattleObservation {
        self.session.settled_observation()
    }

    pub fn legal_actions(&self) -> Vec<Action> {
        self.session.legal_actions().to_vec()
    }

    pub fn view(&mut self) -> GameView {
        self.view_with_sprite_frame(0)
    }

    pub fn view_with_sprite_frame(&mut self, sprite_frame: usize) -> GameView {
        let snapshot = self.session.snapshot();
        self.ui.sync_interaction(snapshot.interaction());
        let own_slot = self
            .own_sprite_ids
            .iter()
            .position(|id| id == snapshot.scene().own().id())
            .expect("the displayed player pokemon belongs to the generated roster");
        let opponent_slot = self
            .opponent_sprite_ids
            .iter()
            .position(|id| id == snapshot.scene().opponent().id())
            .expect("the displayed opponent belongs to the generated roster");
        project_battle(
            &snapshot,
            self.ui,
            BattleSpriteResources::for_slots(own_slot, opponent_slot),
            sprite_frame,
        )
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> Result<bool, SessionError> {
        let snapshot = self.session.snapshot();
        match self.ui.handle_key(key, snapshot.interaction()) {
            BattleUiOutcome::Updated => Ok(true),
            BattleUiOutcome::Submit(action) => {
                self.submit_player(action)?;
                Ok(true)
            }
            BattleUiOutcome::Ignored => Ok(false),
        }
    }

    pub fn submit_player(&mut self, action: Action) -> Result<(), SessionError> {
        self.session.submit(action)
    }

    pub fn has_pending_playback(&self) -> bool {
        self.session.has_pending_playback()
    }

    pub fn advance_playback(&mut self) -> bool {
        self.session.advance()
    }

    pub fn is_finished(&self) -> bool {
        self.session.is_finished()
    }

    fn is_playing(&self) -> bool {
        self.session.has_pending_playback()
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
        let initial_scene = battle.session.snapshot().scene().clone();
        let action = battle
            .legal_actions()
            .into_iter()
            .find(|action| matches!(action, Action::UseMove(_)))
            .unwrap();

        battle.submit_player(action).unwrap();

        assert_eq!(battle.session.snapshot().scene(), &initial_scene);
        while battle.has_pending_playback() {
            battle.advance_playback();
        }
        let observation = battle.observation();
        let active = &observation.own().members()[observation.own().active_slot().index()];
        let final_scene = battle.session.snapshot();
        assert_eq!(final_scene.scene().own().id(), active.id());
        assert_eq!(final_scene.scene().own().current_hp(), active.current_hp());
        assert_eq!(
            final_scene.scene().opponent().current_hp(),
            observation.opponent().active().current_hp()
        );
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
        assert_eq!(
            view.labels()
                .iter()
                .filter(|label| matches!(label.role, TextRole::Action(_)))
                .map(|label| label.content.as_str())
                .collect::<Vec<_>>(),
            ["战斗", "宝可梦", "包包", "逃走"]
        );
        assert!((4..=6).contains(&view.images().len()));
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
    fn switching_the_active_pokemon_changes_identity_and_sprite_on_the_same_frame() {
        let mut battle = DemoBattle::new().unwrap();
        let previous_id = battle.session.snapshot().scene().own().id().clone();
        let action = battle
            .legal_actions()
            .into_iter()
            .find(|action| matches!(action, Action::Switch(_)))
            .expect("the generated team has a legal bench switch");
        let Action::Switch(slot) = action else {
            unreachable!("the selected action is a switch")
        };
        let before = battle.observation();
        let entry_hp = before.own().members()[slot.index()].current_hp();
        let replacement_name = before.own().members()[slot.index()].name().to_owned();
        let opponent_name = before.opponent().active().name().to_owned();

        battle.submit_player(action).unwrap();
        let submitted = battle.view_with_sprite_frame(0);
        assert_eq!(battle.session.snapshot().scene().own().id(), &previous_id);
        assert_eq!(submitted.images()[0].resource, player_back_resource(0, 0));

        let mut switched = false;
        let mut damage_presented = false;
        while battle.has_pending_playback() {
            battle.advance_playback();
            let view = battle.view_with_sprite_frame(0);
            let displayed_name = view
                .labels()
                .iter()
                .find(|label| label.role == TextRole::PlayerName)
                .unwrap();
            let message = view
                .labels()
                .iter()
                .find(|label| label.role == TextRole::Message)
                .unwrap();
            let snapshot = battle.session.snapshot();
            assert_eq!(displayed_name.content, snapshot.scene().own().name());
            if view.images()[0].resource == player_back_resource(slot.index(), 0) {
                assert_ne!(snapshot.scene().own().id(), &previous_id);
                switched = true;
                if !damage_presented && !message.content.contains("受到") {
                    assert_eq!(snapshot.scene().own().current_hp(), entry_hp);
                }
            }
            if message.content.contains("使用了") {
                assert!(
                    switched,
                    "the replacement must appear before the attack: {} / displayed {} / replacement {} / opponent {}",
                    message.content,
                    snapshot.scene().own().name(),
                    replacement_name,
                    opponent_name
                );
                assert_eq!(snapshot.scene().own().current_hp(), entry_hp);
            }
            damage_presented |= message.content.contains("受到");
        }
        assert!(
            switched,
            "the switch event must update the displayed sprite"
        );
    }

    #[test]
    fn knocked_out_opponent_keeps_its_sprite_until_the_replacement_frame() {
        let mut battle = DemoBattle::new().unwrap();

        for _ in 0..500 {
            while battle.has_pending_playback() {
                battle.advance_playback();
            }
            if battle.is_finished() {
                break;
            }
            let previous_id = battle.observation().opponent().active().id().clone();
            let previous_slot = battle
                .opponent_sprite_ids
                .iter()
                .position(|id| id == &previous_id)
                .unwrap();
            let action = battle
                .legal_actions()
                .into_iter()
                .find(|action| matches!(action, Action::UseMove(_)))
                .or_else(|| battle.legal_actions().into_iter().next())
                .unwrap();

            battle.submit_player(action).unwrap();
            let current_id = battle.observation().opponent().active().id().clone();
            if current_id == previous_id {
                continue;
            }

            let submitted = battle.view_with_sprite_frame(0);
            assert_eq!(
                battle.session.snapshot().scene().opponent().id(),
                &previous_id
            );
            assert_eq!(
                submitted.images()[1].resource,
                opponent_front_resource(previous_slot, 0)
            );

            let mut replacement_presented = false;
            while battle.has_pending_playback() {
                battle.advance_playback();
                let displayed_slot = battle
                    .opponent_sprite_ids
                    .iter()
                    .position(|id| id == battle.session.snapshot().scene().opponent().id())
                    .unwrap();
                let view = battle.view_with_sprite_frame(0);
                let displayed_name = view
                    .labels()
                    .iter()
                    .find(|label| label.role == TextRole::OpponentName)
                    .unwrap();
                let snapshot = battle.session.snapshot();
                assert_eq!(displayed_name.content, snapshot.scene().opponent().name());
                assert_eq!(
                    view.images()[1].resource,
                    opponent_front_resource(displayed_slot, 0)
                );
                replacement_presented |= snapshot.scene().opponent().id() == &current_id;
            }
            assert!(replacement_presented);
            return;
        }

        panic!("the deterministic demo battle must present an opponent replacement");
    }

    #[test]
    fn faint_playback_finishes_before_the_forced_replacement_page_opens() {
        let mut battle = DemoBattle::new().unwrap();

        for _ in 0..500 {
            while battle.has_pending_playback() {
                battle.advance_playback();
            }
            if battle.is_finished() {
                break;
            }
            let action = battle
                .legal_actions()
                .into_iter()
                .find(|action| matches!(action, Action::UseMove(_)))
                .or_else(|| battle.legal_actions().into_iter().next())
                .unwrap();
            battle.submit_player(action).unwrap();

            let observation = battle.observation();
            if !observation
                .phase()
                .requires_replacement(observation.viewer())
            {
                continue;
            }

            assert!(battle.has_pending_playback());
            let first_frame = battle.view();
            assert!(
                !first_frame
                    .labels()
                    .iter()
                    .any(|label| label.role == TextRole::PageTitle)
            );
            while battle.has_pending_playback() {
                battle.advance_playback();
                let still_playing = battle.has_pending_playback();
                let view = battle.view();
                if still_playing {
                    assert!(
                        !view
                            .labels()
                            .iter()
                            .any(|label| label.role == TextRole::PageTitle)
                    );
                }
            }
            let replacement = battle.view();
            assert!(replacement.labels().iter().any(|label| {
                label.role == TextRole::PageTitle && label.content == "宝可梦"
            }));
            return;
        }

        panic!("the deterministic demo battle must require a player replacement");
    }
}
