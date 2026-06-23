use battle_core::{
    BattleAction, BattleError, BattleInit, BattleLog, BattleState, DomainEvent, MetricsEvent,
    Recorder, RngState, SideId, TraceEvent, initialize_battle, step,
};
use battle_data::DataPack;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{Display, Formatter};

/**
这个 crate 负责把一场 battle 的“可复现信息”和“可展示信息”收集起来。

它不重新实现结算规则，只负责把核心已经产出的输入帧和日志整理成可保存、可导出、可重放的记录结构。
*/
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReplayManifest {
    pub engine_api_version: String,
    pub replay_schema_version: u16,
    pub data_pack_id: String,
}

/**
`InputFrame` 记录一边在某次请求下提交了什么动作。

输入帧比文本日志更重要，因为它和 seed 一起构成了最严格的 replay 基础。
只要输入序列和版本信息不变，battle 就应该能被重新跑出来。
*/
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InputFrame {
    pub side: SideId,
    pub action: BattleAction,
}

/**
`Checkpoint` 用来给 replay 打上人工可读的定位点。

checkpoint 不保存完整状态快照。
恢复时仍然从开局开始重放，但它提供了一个稳定的“停到哪里”的定位坐标。
*/
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub turn: u16,
    pub note: String,
}

/**
`BattleRecord` 是当前项目的 replay 主结构。

它同时保存三类信息：

- 重放 battle 需要的权威输入
- 外层展示需要的事件流
- 调试和定位问题需要的附加信息
*/
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BattleRecord {
    pub manifest: ReplayManifest,
    pub init: BattleInit,
    pub seed: u64,
    pub input_frames: Vec<InputFrame>,
    pub domain_events: Vec<DomainEvent>,
    pub trace_events: Vec<TraceEvent>,
    pub metrics: Vec<MetricsEvent>,
    pub checkpoints: Vec<Checkpoint>,
}

impl BattleRecord {
    /**
    创建一份新的 battle 记录。

    这里把版本号、seed、初始队伍和数据包 id 一次写进去，
    是为了让后面的输入帧和事件帧都能挂在一个明确的 replay 上下文里。
    */
    pub fn new(init: BattleInit, seed: u64, data_pack_id: impl Into<String>) -> Self {
        Self {
            manifest: ReplayManifest {
                engine_api_version: "0.1.0".to_string(),
                replay_schema_version: 1,
                data_pack_id: data_pack_id.into(),
            },
            init,
            seed,
            input_frames: Vec::new(),
            domain_events: Vec::new(),
            trace_events: Vec::new(),
            metrics: Vec::new(),
            checkpoints: Vec::new(),
        }
    }

    /**
    追加一条玩家输入。

    这层记录的是“外部提交了什么动作”，而不是“动作最终如何结算”。
    把输入帧和事件帧分开，是 replay 可验证性的关键。
    */
    pub fn push_input(&mut self, side: SideId, action: BattleAction) {
        self.input_frames.push(InputFrame { side, action });
    }

    /**
    把一次 `step` 产出的日志并入 replay。

    这里不重新解释日志语义，只做复制和拼接。
    replay 层应该尽量薄，避免和核心日志定义产生第二套语义。
    */
    pub fn append_log(&mut self, log: &BattleLog) {
        self.domain_events.extend(log.domain.clone());
        self.trace_events.extend(log.trace.clone());
        self.metrics.extend(log.metrics.iter().copied());
    }

    /**
    增加一个人工标注的检查点。

    这类检查点不参与 battle 正确性，只服务调试、阅读和 seek/restore。
    */
    pub fn add_checkpoint(&mut self, turn: u16, note: impl Into<String>) {
        self.checkpoints.push(Checkpoint {
            turn,
            note: note.into(),
        });
    }

    /**
    导出为格式化 JSON。

    当前 CLI 直接把 replay 写成 JSON 文件，优先保证可读和易调试。
    等格式稳定后，再考虑压缩或二进制表达。
    */
    pub fn to_pretty_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    /**
    从 JSON 字符串恢复 replay。
    */
    pub fn from_json_str(input: &str) -> serde_json::Result<Self> {
        serde_json::from_str(input)
    }
}

/**
`ReplayResult` 是一次 replay/restore 之后的完整结果。

它保留最终状态和 RNG，同时返回一份重建出来的记录结构，
这样调用方既可以拿它做断言，也可以把它再导出成新的 goldens。
*/
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayResult {
    pub state: BattleState,
    pub rng: RngState,
    pub regenerated_record: BattleRecord,
}

/**
`ReplayError` 表示 replay 层自己的失败类型。
*/
#[derive(Debug)]
pub enum ReplayError {
    Core(BattleError),
    Json(serde_json::Error),
    DomainEventsMismatch { expected: usize, actual: usize },
    TraceEventsMismatch { expected: usize, actual: usize },
    MetricsMismatch { expected: usize, actual: usize },
    CheckpointIndexOutOfRange { index: usize },
    CheckpointTurnNotReached { turn: u16 },
}

impl Display for ReplayError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Core(error) => write!(f, "{error}"),
            Self::Json(error) => write!(f, "{error}"),
            Self::DomainEventsMismatch { expected, actual } => {
                write!(f, "replay domain events diverged: expected {expected}, got {actual}")
            }
            Self::TraceEventsMismatch { expected, actual } => {
                write!(f, "replay trace events diverged: expected {expected}, got {actual}")
            }
            Self::MetricsMismatch { expected, actual } => {
                write!(f, "replay metrics diverged: expected {expected}, got {actual}")
            }
            Self::CheckpointIndexOutOfRange { index } => {
                write!(f, "checkpoint index {index} is out of range")
            }
            Self::CheckpointTurnNotReached { turn } => {
                write!(f, "replay never reached checkpoint turn {turn}")
            }
        }
    }
}

impl Error for ReplayError {}

impl From<BattleError> for ReplayError {
    fn from(value: BattleError) -> Self {
        Self::Core(value)
    }
}

impl From<serde_json::Error> for ReplayError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

/**
从开局开始完整重跑一份 replay。
*/
pub fn replay_battle(record: &BattleRecord, data: &DataPack) -> Result<ReplayResult, ReplayError> {
    let mut state = initialize_battle(record.init.clone(), data)?;
    let mut rng = RngState::seeded(record.seed);
    let mut regenerated_record = BattleRecord::new(
        record.init.clone(),
        record.seed,
        record.manifest.data_pack_id.clone(),
    );

    for frame in &record.input_frames {
        regenerated_record.push_input(frame.side, frame.action);
        let result = step(state, frame.side, frame.action, rng, data)?;
        regenerated_record.append_log(&result.log);
        state = result.state;
        rng = result.rng;
    }

    regenerated_record.checkpoints = record.checkpoints.clone();

    Ok(ReplayResult {
        state,
        rng,
        regenerated_record,
    })
}

/**
重跑 replay，并在记录里已经带有事件/metrics 时校验是否完全一致。
*/
pub fn verify_replay(record: &BattleRecord, data: &DataPack) -> Result<ReplayResult, ReplayError> {
    let result = replay_battle(record, data)?;

    if !record.domain_events.is_empty()
        && record.domain_events != result.regenerated_record.domain_events
    {
        return Err(ReplayError::DomainEventsMismatch {
            expected: record.domain_events.len(),
            actual: result.regenerated_record.domain_events.len(),
        });
    }

    if !record.trace_events.is_empty()
        && record.trace_events != result.regenerated_record.trace_events
    {
        return Err(ReplayError::TraceEventsMismatch {
            expected: record.trace_events.len(),
            actual: result.regenerated_record.trace_events.len(),
        });
    }

    if !record.metrics.is_empty() && record.metrics != result.regenerated_record.metrics {
        return Err(ReplayError::MetricsMismatch {
            expected: record.metrics.len(),
            actual: result.regenerated_record.metrics.len(),
        });
    }

    Ok(result)
}

/**
把 replay 恢复到某个 turn 值对应的位置。

这里的 `turn` 和 `BattleState.turn` 保持同一语义：
例如一个 checkpoint 标在 `turn = 2`，表示恢复后的状态应该已经完成第 1 回合，
正处在第 2 回合开始前。
*/
pub fn restore_to_turn(
    record: &BattleRecord,
    data: &DataPack,
    turn: u16,
) -> Result<ReplayResult, ReplayError> {
    let mut state = initialize_battle(record.init.clone(), data)?;
    let mut rng = RngState::seeded(record.seed);
    let mut regenerated_record = BattleRecord::new(
        record.init.clone(),
        record.seed,
        record.manifest.data_pack_id.clone(),
    );

    if state.turn == turn {
        regenerated_record.checkpoints = record
            .checkpoints
            .iter()
            .filter(|checkpoint| checkpoint.turn <= turn)
            .cloned()
            .collect();
        return Ok(ReplayResult {
            state,
            rng,
            regenerated_record,
        });
    }

    for frame in &record.input_frames {
        regenerated_record.push_input(frame.side, frame.action);
        let result = step(state, frame.side, frame.action, rng, data)?;
        regenerated_record.append_log(&result.log);
        state = result.state;
        rng = result.rng;

        if state.turn == turn {
            regenerated_record.checkpoints = record
                .checkpoints
                .iter()
                .filter(|checkpoint| checkpoint.turn <= turn)
                .cloned()
                .collect();
            return Ok(ReplayResult {
                state,
                rng,
                regenerated_record,
            });
        }
    }

    Err(ReplayError::CheckpointTurnNotReached { turn })
}

/**
把 replay 恢复到某个 checkpoint。
*/
pub fn restore_checkpoint(
    record: &BattleRecord,
    data: &DataPack,
    checkpoint_index: usize,
) -> Result<ReplayResult, ReplayError> {
    let checkpoint = record
        .checkpoints
        .get(checkpoint_index)
        .ok_or(ReplayError::CheckpointIndexOutOfRange {
            index: checkpoint_index,
        })?;
    restore_to_turn(record, data, checkpoint.turn)
}

/**
`ReplayRecorder` 是一个专门面向 replay 的简单 recorder。

如果某个调用方不需要 `BattleLog` 这种一步一步返回的结构，只想持续收集事件，
可以直接把这个 recorder 传进核心。
*/
#[derive(Default)]
pub struct ReplayRecorder {
    pub domain_events: Vec<DomainEvent>,
    pub trace_events: Vec<TraceEvent>,
    pub metrics: Vec<MetricsEvent>,
}

impl Recorder for ReplayRecorder {
    fn domain(&mut self, event: DomainEvent) {
        self.domain_events.push(event);
    }

    fn trace(&mut self, event: TraceEvent) {
        self.trace_events.push(event);
    }

    fn metric(&mut self, event: MetricsEvent) {
        self.metrics.push(event);
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use battle_core::{BattleAction, BattleInit, Recorder, RngState, SideId, initialize_battle, step};
    use battle_data::{load_demo_enemy_team, load_demo_player_team, load_gen1_demo_pack};

    use super::{
        BattleRecord, ReplayRecorder, replay_battle, restore_checkpoint, verify_replay,
    };

    #[test]
    fn battle_record_keeps_inputs_and_checkpoints() {
        let init = BattleInit {
            player: load_demo_player_team(),
            opponent: load_demo_enemy_team(),
        };
        let mut record = BattleRecord::new(init, 13, "gen1-demo");
        record.push_input(SideId::Player, BattleAction::UseMove(0));
        record.add_checkpoint(1, "after first choice");

        assert_eq!(record.input_frames.len(), 1);
        assert_eq!(record.checkpoints.len(), 1);
    }

    #[test]
    fn replay_record_serializes_to_json() {
        let init = BattleInit {
            player: load_demo_player_team(),
            opponent: load_demo_enemy_team(),
        };
        let record = BattleRecord::new(init, 13, "gen1-demo");
        let json = record.to_pretty_json().unwrap();

        assert!(json.contains("\"data_pack_id\": \"gen1-demo\""));
        assert!(json.contains("\"seed\": 13"));
    }

    #[test]
    fn replay_record_round_trips_json() {
        let init = BattleInit {
            player: load_demo_player_team(),
            opponent: load_demo_enemy_team(),
        };
        let mut record = BattleRecord::new(init, 13, "gen1-demo");
        record.push_input(SideId::Player, BattleAction::UseMove(0));
        record.add_checkpoint(2, "after turn 1");

        let json = record.to_pretty_json().unwrap();
        let restored = BattleRecord::from_json_str(&json).unwrap();

        assert_eq!(restored, record);
    }

    #[test]
    fn replay_recorder_collects_core_events() {
        let mut recorder = ReplayRecorder::default();
        recorder.domain(battle_core::DomainEvent::TurnStarted { turn: 1 });
        recorder.trace(battle_core::TraceEvent::TurnResolved { turn: 1 });

        assert_eq!(recorder.domain_events.len(), 1);
        assert_eq!(recorder.trace_events.len(), 1);
    }

    #[test]
    fn verify_replay_reproduces_recorded_events() {
        let data = load_gen1_demo_pack();
        let init = BattleInit {
            player: load_demo_player_team(),
            opponent: load_demo_enemy_team(),
        };
        let mut state = initialize_battle(init.clone(), &data).unwrap();
        let mut rng = RngState::seeded(42);
        let mut record = BattleRecord::new(init, 42, data.id.clone());

        for (side, action) in [
            (SideId::Player, BattleAction::UseMove(0)),
            (SideId::Opponent, BattleAction::UseMove(0)),
            (SideId::Player, BattleAction::UseMove(1)),
            (SideId::Opponent, BattleAction::UseMove(0)),
        ] {
            record.push_input(side, action);
            let result = step(state, side, action, rng, &data).unwrap();
            if !result.log.metrics.is_empty() {
                record.add_checkpoint(
                    result.state.turn,
                    format!("after turn {}", result.state.turn.saturating_sub(1)),
                );
            }
            record.append_log(&result.log);
            state = result.state;
            rng = result.rng;
        }

        let replayed = verify_replay(&record, &data).unwrap();

        assert_eq!(record.domain_events, replayed.regenerated_record.domain_events);
        assert_eq!(record.trace_events, replayed.regenerated_record.trace_events);
        assert_eq!(record.metrics, replayed.regenerated_record.metrics);
    }

    #[test]
    fn restore_checkpoint_returns_state_at_recorded_turn() {
        let data = load_gen1_demo_pack();
        let init = BattleInit {
            player: load_demo_player_team(),
            opponent: load_demo_enemy_team(),
        };
        let mut state = initialize_battle(init.clone(), &data).unwrap();
        let mut rng = RngState::seeded(42);
        let mut record = BattleRecord::new(init, 42, data.id.clone());

        for (side, action) in [
            (SideId::Player, BattleAction::UseMove(0)),
            (SideId::Opponent, BattleAction::UseMove(0)),
            (SideId::Player, BattleAction::UseMove(1)),
            (SideId::Opponent, BattleAction::UseMove(0)),
        ] {
            record.push_input(side, action);
            let result = step(state, side, action, rng, &data).unwrap();
            record.append_log(&result.log);
            if !result.log.metrics.is_empty() {
                record.add_checkpoint(
                    result.state.turn,
                    format!("after turn {}", result.state.turn.saturating_sub(1)),
                );
            }
            state = result.state;
            rng = result.rng;
        }

        let restored = restore_checkpoint(&record, &data, 0).unwrap();

        assert_eq!(restored.state.turn, 2);
        assert!(!restored.regenerated_record.domain_events.is_empty());
    }

    #[test]
    fn golden_first_playable_demo_replays_cleanly() {
        let data = load_gen1_demo_pack();
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../replays/first-playable-demo.json"
        );
        let json = fs::read_to_string(path).unwrap();
        let record = BattleRecord::from_json_str(&json).unwrap();

        let replayed = replay_battle(&record, &data).unwrap();

        assert_eq!(record.input_frames.len(), replayed.regenerated_record.input_frames.len());
        assert_eq!(record.checkpoints, replayed.regenerated_record.checkpoints);
        assert!(replayed.state.turn >= 2);
    }

    #[test]
    fn replay_battle_preserves_final_state_turn_for_golden() {
        let data = load_gen1_demo_pack();
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../replays/first-playable-demo.json"
        );
        let json = fs::read_to_string(path).unwrap();
        let record = BattleRecord::from_json_str(&json).unwrap();

        let replayed = replay_battle(&record, &data).unwrap();

        assert!(replayed.state.turn >= 2);
        assert_eq!(replayed.regenerated_record.input_frames.len(), record.input_frames.len());
    }
}
