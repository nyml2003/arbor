use battle_application::{
    Action, BattleError, BattleObservation, BattleOutcome, BattlePhase, ObservedBattleOutcome,
    Participant,
};

use crate::{
    BattleCoordinator, BattleCue, BattleScene, OpponentPolicy, PlaybackStep, ReplayError,
    coordinator::CoordinatorError, reduce_transition, scene_from_observation,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionPrompt {
    observation: BattleObservation,
    legal_actions: Vec<Action>,
}

impl ActionPrompt {
    pub const fn observation(&self) -> &BattleObservation {
        &self.observation
    }

    pub fn legal_actions(&self) -> &[Action] {
        &self.legal_actions
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplacementPrompt {
    observation: BattleObservation,
    legal_actions: Vec<Action>,
}

impl ReplacementPrompt {
    pub const fn observation(&self) -> &BattleObservation {
        &self.observation
    }

    pub fn legal_actions(&self) -> &[Action] {
        &self.legal_actions
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FinishedPrompt {
    outcome: ObservedBattleOutcome,
}

impl FinishedPrompt {
    pub const fn outcome(self) -> ObservedBattleOutcome {
        self.outcome
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BattleSessionPhase {
    AwaitingAction(ActionPrompt),
    Playing,
    AwaitingReplacement(ReplacementPrompt),
    Finished(FinishedPrompt),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BattleInteraction {
    ChooseAction(ActionPrompt),
    PlaybackLocked,
    ChooseReplacement(ReplacementPrompt),
    Finished(FinishedPrompt),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BattleSessionSnapshot {
    scene: BattleScene,
    interaction: BattleInteraction,
    cue: Option<BattleCue>,
}

impl BattleSessionSnapshot {
    pub const fn scene(&self) -> &BattleScene {
        &self.scene
    }

    pub const fn interaction(&self) -> &BattleInteraction {
        &self.interaction
    }

    pub const fn cue(&self) -> Option<&BattleCue> {
        self.cue.as_ref()
    }
}

pub struct BattleSession<P> {
    coordinator: BattleCoordinator<P>,
    scene: BattleScene,
    phase: BattleSessionPhase,
    playback: Vec<PlaybackStep>,
    playback_cursor: usize,
    pending_after: Option<BattleObservation>,
    cue: Option<BattleCue>,
}

impl<P: OpponentPolicy> BattleSession<P> {
    pub fn new(coordinator: BattleCoordinator<P>) -> Self {
        let observation = coordinator.player_observation();
        let scene = scene_from_observation(&observation);
        let phase = settled_phase(&coordinator, observation);
        Self {
            coordinator,
            scene,
            phase,
            playback: Vec::new(),
            playback_cursor: 0,
            pending_after: None,
            cue: None,
        }
    }

    pub const fn phase(&self) -> &BattleSessionPhase {
        &self.phase
    }

    pub fn snapshot(&self) -> BattleSessionSnapshot {
        BattleSessionSnapshot {
            scene: self.scene.clone(),
            interaction: match &self.phase {
                BattleSessionPhase::AwaitingAction(prompt) => {
                    BattleInteraction::ChooseAction(prompt.clone())
                }
                BattleSessionPhase::Playing => BattleInteraction::PlaybackLocked,
                BattleSessionPhase::AwaitingReplacement(prompt) => {
                    BattleInteraction::ChooseReplacement(prompt.clone())
                }
                BattleSessionPhase::Finished(prompt) => BattleInteraction::Finished(*prompt),
            },
            cue: self.cue.clone(),
        }
    }

    pub fn legal_actions(&self) -> &[Action] {
        match &self.phase {
            BattleSessionPhase::AwaitingAction(prompt) => prompt.legal_actions(),
            BattleSessionPhase::AwaitingReplacement(prompt) => prompt.legal_actions(),
            BattleSessionPhase::Playing | BattleSessionPhase::Finished(_) => &[],
        }
    }

    pub fn submit(&mut self, action: Action) -> Result<(), SessionError> {
        let legal_actions = self.legal_actions();
        if legal_actions.is_empty() {
            return Err(SessionError::InputLocked);
        }
        if !legal_actions.contains(&action) {
            return Err(SessionError::ActionNotOffered { action });
        }
        let transition = self.coordinator.resolve_player_action(action)?;
        let playback = reduce_transition(&transition)?;
        self.scene = scene_from_observation(transition.before());
        self.phase = BattleSessionPhase::Playing;
        self.playback = playback;
        self.playback_cursor = 0;
        self.pending_after = Some(transition.after().clone());
        self.cue = None;
        Ok(())
    }

    pub fn advance(&mut self) -> bool {
        if !matches!(self.phase, BattleSessionPhase::Playing) {
            return false;
        }
        if let Some(step) = self.playback.get(self.playback_cursor) {
            self.scene = step.scene().clone();
            self.cue = Some(step.cue().clone());
            self.playback_cursor += 1;
            return true;
        }
        let after = self
            .pending_after
            .take()
            .expect("playing sessions own a final observation");
        self.scene = scene_from_observation(&after);
        self.phase = settled_phase(&self.coordinator, after);
        self.playback.clear();
        self.playback_cursor = 0;
        self.cue = None;
        true
    }

    pub fn has_pending_playback(&self) -> bool {
        matches!(self.phase, BattleSessionPhase::Playing)
    }

    pub fn is_finished(&self) -> bool {
        matches!(self.phase, BattleSessionPhase::Finished(_))
    }

    pub fn settled_observation(&self) -> BattleObservation {
        self.coordinator.player_observation()
    }
}

fn settled_phase<P: OpponentPolicy>(
    coordinator: &BattleCoordinator<P>,
    observation: BattleObservation,
) -> BattleSessionPhase {
    match observation.phase() {
        BattlePhase::Turn => BattleSessionPhase::AwaitingAction(ActionPrompt {
            observation,
            legal_actions: coordinator.player_legal_actions(),
        }),
        BattlePhase::ForcedReplacement(_) => {
            BattleSessionPhase::AwaitingReplacement(ReplacementPrompt {
                observation,
                legal_actions: coordinator.player_legal_actions(),
            })
        }
        BattlePhase::Finished(outcome) => BattleSessionPhase::Finished(FinishedPrompt {
            outcome: observed_outcome(outcome, observation.viewer()),
        }),
    }
}

const fn observed_outcome(
    outcome: BattleOutcome,
    viewer: battle_application::Side,
) -> ObservedBattleOutcome {
    match outcome {
        BattleOutcome::Winner(side) => ObservedBattleOutcome::Winner(participant(side, viewer)),
        BattleOutcome::Escaped(side) => ObservedBattleOutcome::Escaped(participant(side, viewer)),
        BattleOutcome::Draw => ObservedBattleOutcome::Draw,
    }
}

const fn participant(
    side: battle_application::Side,
    viewer: battle_application::Side,
) -> Participant {
    use battle_application::Side;
    match (side, viewer) {
        (Side::One, Side::One) | (Side::Two, Side::Two) => Participant::Own,
        (Side::One, Side::Two) | (Side::Two, Side::One) => Participant::Opponent,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionError {
    InputLocked,
    ActionNotOffered { action: Action },
    Battle(BattleError),
    Transition(battle_application::TransitionError),
    OpponentActionUnavailable,
    Replay(ReplayError),
}

impl From<CoordinatorError> for SessionError {
    fn from(error: CoordinatorError) -> Self {
        match error {
            CoordinatorError::Battle(error) => Self::Battle(error),
            CoordinatorError::Transition(error) => Self::Transition(error),
            CoordinatorError::OpponentActionUnavailable => Self::OpponentActionUnavailable,
        }
    }
}

impl From<ReplayError> for SessionError {
    fn from(error: ReplayError) -> Self {
        Self::Replay(error)
    }
}
