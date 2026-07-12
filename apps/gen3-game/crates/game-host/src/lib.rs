//! Battle application composition shared by the native host and end-to-end tests.

#![forbid(unsafe_code)]

use std::collections::VecDeque;

use battle_application::{
    Accuracy, Action, BattleApplication, BattleError, BattleEvent, BattleObservation,
    BattleOutcome, BattlePerspective, BattleStats, Move, MoveId, Pokemon, PokemonId, PokemonType,
    Side, Team, TypeEffectiveness, UsedMove,
};
use game_ui::{
    BattleAnimation, BattleUiOutcome, BattleUiState, BattleView, phase_message, project_battle,
};
use punctum_input::KeyEvent;

pub struct DemoBattle {
    application: BattleApplication,
    player: BattlePerspective,
    opponent: BattlePerspective,
    ui: BattleUiState,
    message: String,
    animation: BattleAnimation,
    playback: VecDeque<PlaybackFrame>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PlaybackFrame {
    message: String,
    animation: BattleAnimation,
}

impl DemoBattle {
    pub fn new() -> Result<Self, BattleError> {
        let application = BattleApplication::new(
            demo_team("player", true),
            demo_team("rival", false),
            0xA2B3_C4D5,
        )?;
        let (player, opponent) = application.perspectives();
        Ok(Self {
            application,
            player,
            opponent,
            ui: BattleUiState::default(),
            message: "请选择行动".into(),
            animation: BattleAnimation::Idle,
            playback: VecDeque::new(),
        })
    }

    pub fn observation(&self) -> BattleObservation {
        self.application.observe(&self.player)
    }

    pub fn legal_actions(&self) -> Vec<Action> {
        self.application.legal_actions(&self.player)
    }

    pub fn view(&mut self) -> BattleView {
        let observation = self.observation();
        let actions = self.legal_actions();
        self.ui.reconcile(&actions);
        project_battle(
            &observation,
            &actions,
            self.ui,
            &self.message,
            self.animation,
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
        true
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
        let mut frames = events
            .iter()
            .filter_map(|event| self.event_frame(event))
            .collect::<VecDeque<_>>();
        frames.push_back(PlaybackFrame {
            message: phase_message(self.observation().phase()).into(),
            animation: BattleAnimation::Idle,
        });
        if let Some(frame) = frames.pop_front() {
            self.message = frame.message;
            self.animation = frame.animation;
        }
        self.playback = frames;
    }

    fn event_frame(&self, event: &BattleEvent) -> Option<PlaybackFrame> {
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
        Some(PlaybackFrame { message, animation })
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

fn demo_team(prefix: &str, player: bool) -> Team {
    let names = if player {
        ["电蜥", "苔芽", "砾仔", "雾翎", "炽崽", "萤蛾"]
    } else {
        ["焰角兽", "荆棘芽", "潮鳍", "岩龙", "夜绒", "铁耳狐"]
    };
    let types = if player {
        [
            PokemonType::Electric,
            PokemonType::Grass,
            PokemonType::Rock,
            PokemonType::Flying,
            PokemonType::Fire,
            PokemonType::Bug,
        ]
    } else {
        [
            PokemonType::Fire,
            PokemonType::Grass,
            PokemonType::Water,
            PokemonType::Ground,
            PokemonType::Dark,
            PokemonType::Steel,
        ]
    };
    Team::new(
        names
            .into_iter()
            .zip(types)
            .enumerate()
            .map(|(index, (name, pokemon_type))| {
                demo_pokemon(prefix, index, name, pokemon_type, player)
            })
            .collect(),
    )
    .expect("the fixed demo team is valid")
}

fn demo_pokemon(
    prefix: &str,
    index: usize,
    name: &str,
    pokemon_type: PokemonType,
    player: bool,
) -> Pokemon {
    let move_type = if player && index == 0 {
        PokemonType::Electric
    } else if !player && index == 0 {
        PokemonType::Fire
    } else {
        pokemon_type
    };
    let moves = [
        ("strike", "快速撞击", PokemonType::Normal, 45),
        ("pulse", "属性脉冲", move_type, 65),
        ("rush", "猛烈冲锋", PokemonType::Normal, 75),
        ("focus", "聚能光束", move_type, 55),
    ]
    .into_iter()
    .map(|(suffix, move_name, move_type, power)| {
        Move::new(
            MoveId::new(format!("{prefix}-{index}-{suffix}")).unwrap(),
            move_name,
            move_type,
            power,
            Accuracy::Percent(100),
            20,
            20,
            0,
        )
        .unwrap()
    })
    .collect();
    Pokemon::new(
        PokemonId::new(format!("{prefix}-{index}")).unwrap(),
        name,
        24,
        pokemon_type,
        None,
        110,
        110,
        BattleStats::new(58, 52, 61, 54, if player { 62 } else { 55 }).unwrap(),
        moves,
    )
    .expect("the fixed demo creature is valid")
}

#[cfg(test)]
mod tests {
    use battle_application::Action;
    use game_ui::{CANVAS_HEIGHT, CANVAS_WIDTH, TextRole};
    use punctum_grid::GridSize;

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
        assert!(
            after.own().members()[after.own().active_slot().index()].current_hp()
                < before.own().members()[before.own().active_slot().index()].current_hp()
        );
        assert!(after.opponent().active().current_hp() < before.opponent().active().current_hp());
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
        assert!(messages.iter().any(|message| message.contains("受到")));
    }

    #[test]
    fn projected_battle_view_contains_the_fixed_canvas_status_and_actions() {
        let mut battle = DemoBattle::new().unwrap();
        let view = battle.view();

        assert_eq!(
            view.surface().size(),
            GridSize::new(CANVAS_WIDTH, CANVAS_HEIGHT)
        );
        assert!(
            view.labels()
                .iter()
                .any(|label| label.role == TextRole::OpponentName && label.content == "焰角兽")
        );
        assert!(
            view.labels()
                .iter()
                .filter(|label| matches!(label.role, TextRole::Action(_)))
                .count()
                == 4
        );
    }
}
