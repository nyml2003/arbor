mod log;
mod move_resolution;
mod ops;
mod rng;
mod state;
#[cfg(test)]
mod tests;
mod turn;

use battle_data::DataPack;

pub use log::{BattleLog, BufferRecorder, DomainEvent, MetricsEvent, NoopRecorder, Recorder, TraceEvent};
use log::TeeRecorder;
pub use rng::RngState;
pub use state::{
    BattleError, BattleInit, BattleState, CombatPokemon, StatStages, StepResult, TeamState,
    WeatherState, initialize_battle, requested_side,
};
use turn::{resolve_turn, validate_action};

/**
这个 crate 是当前项目唯一的权威结算层。

它负责三件事：

- 持有 battle 的运行时状态
- 接受双方输入并推进一回合
- 产出 typed log，供 CLI、回放和测试使用

它故意不做 IO，也不直接依赖外部进程、终端界面和存储。
*/
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SideId {
    Player,
    Opponent,
}

impl SideId {
    pub fn index(self) -> usize {
        match self {
            Self::Player => 0,
            Self::Opponent => 1,
        }
    }

    pub fn foe(self) -> Self {
        match self {
            Self::Player => Self::Opponent,
            Self::Opponent => Self::Player,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BattleAction {
    UseMove(usize),
    Switch(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Request {
    ChooseAction { side: SideId },
    Finished { winner: SideId },
}

pub fn step(
    state: BattleState,
    side: SideId,
    action: BattleAction,
    rng: RngState,
    data: &DataPack,
) -> Result<StepResult, BattleError> {
    let mut recorder = BufferRecorder::default();
    step_with_recorder(state, side, action, rng, data, &mut recorder)
}

pub fn step_with_recorder<R: Recorder>(
    mut state: BattleState,
    side: SideId,
    action: BattleAction,
    mut rng: RngState,
    data: &DataPack,
    rec: &mut R,
) -> Result<StepResult, BattleError> {
    if state.winner.is_some() {
        return Err(BattleError::BattleFinished);
    }

    let expected = match requested_side(&state) {
        Request::ChooseAction { side } => side,
        Request::Finished { .. } => return Err(BattleError::BattleFinished),
    };

    if expected != side {
        return Err(BattleError::InvalidActionOrder { expected, got: side });
    }

    validate_action(&state, side, action)?;

    let mut recorder = TeeRecorder::new(rec);
    recorder.trace(TraceEvent::ChoiceAccepted { side });
    recorder.domain(DomainEvent::ChoiceCommitted { side, action });
    state.pending[side.index()] = Some(action);

    if state.pending[side.foe().index()].is_none() {
        let next_request = requested_side(&state);
        return Ok(StepResult { state, rng, log: recorder.into_log(), next_request });
    }

    resolve_turn(&mut state, data, &mut rng, &mut recorder);
    let next_request = requested_side(&state);
    let turn = state.turn.saturating_sub(1);
    let log = recorder.into_log();
    let metric = MetricsEvent { turn, domain_events: log.domain.len(), trace_events: log.trace.len() };
    let mut full_log = log;
    full_log.metrics.push(metric);

    Ok(StepResult { state, rng, log: full_log, next_request })
}
