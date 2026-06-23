use battle_data::{MoveId, StatId, StatusCondition, WeatherKind};
use serde::{Deserialize, Serialize};

use crate::{BattleAction, SideId};

/**
`DomainEvent` 是面向产品语义的对战日志。

这层日志服务三类消费者：

- CLI 文本渲染
- replay 持久化
- 测试断言

它不追求覆盖所有内部细节，而是保留外层真正关心的 battle 语义。
*/
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DomainEvent {
    ChoiceCommitted { side: SideId, action: BattleAction },
    TurnStarted { turn: u16 },
    MoveUsed { side: SideId, move_id: MoveId },
    MoveMissed { side: SideId, move_id: MoveId },
    WeatherStarted { weather: WeatherKind, remaining_turns: u8 },
    WeatherEnded { weather: WeatherKind },
    ForcedSwitch { side: SideId },
    DamageDealt { side: SideId, target: SideId, amount: u16, remaining_hp: u16 },
    ResidualDamage { target: SideId, status: StatusCondition, amount: u16, remaining_hp: u16 },
    Healed { side: SideId, amount: u16, remaining_hp: u16 },
    StatusApplied { side: SideId, status: StatusCondition },
    StatStageChanged { side: SideId, stat: StatId, new_stage: i8 },
    ActionBlockedByStatus { side: SideId, status: StatusCondition },
    PokemonFainted { side: SideId, slot: usize },
    PokemonSwitched { side: SideId, slot: usize },
    BattleEnded { winner: SideId },
}

/**
`TraceEvent` 是面向开发和调试的内部日志。

它比 `DomainEvent` 更贴近结算过程，例如命中骰子和行动顺序。
这层事件可以帮助定位 bug，但不应该成为玩家 UI 的主数据源。
*/
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TraceEvent {
    ChoiceAccepted { side: SideId },
    TurnResolved { turn: u16 },
    MoveOrderCalculated { first: SideId, second: SideId },
    AccuracyRolled { side: SideId, roll: u8, needed: u8 },
    StatusRolled { side: SideId, status: StatusCondition, roll: u8, needed: u8 },
    WeatherAppliedToDamage { weather: WeatherKind, move_id: MoveId },
    DamageRolled { side: SideId, move_id: MoveId, damage: u16 },
    ActionSkipped { side: SideId },
}

/**
`MetricsEvent` 是当前这一步结算的轻量统计。

它不参与规则，也不参与玩家可见语义。
保留这层是为了后面观察单回合日志规模和热路径复杂度。
*/
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MetricsEvent {
    pub turn: u16,
    pub domain_events: usize,
    pub trace_events: usize,
}

/**
`BattleLog` 把一次 `step` 产出的多层日志打包返回。

这样外层不需要理解 recorder 的内部实现，只要消费这个稳定结果即可。
*/
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct BattleLog {
    pub domain: Vec<DomainEvent>,
    pub trace: Vec<TraceEvent>,
    pub metrics: Vec<MetricsEvent>,
}

/**
`Recorder` 是核心向外发日志的最小接口。

它的设计重点是让核心仍然保持纯函数语义：
调用方把 recorder 传进来，核心只负责写 typed event，不负责决定这些事件要不要落盘、打印还是忽略。
*/
pub trait Recorder {
    fn domain(&mut self, event: DomainEvent);
    fn trace(&mut self, event: TraceEvent);
    fn metric(&mut self, event: MetricsEvent);
}

#[derive(Default)]
pub struct NoopRecorder;

impl Recorder for NoopRecorder {
    fn domain(&mut self, _event: DomainEvent) {}
    fn trace(&mut self, _event: TraceEvent) {}
    fn metric(&mut self, _event: MetricsEvent) {}
}

#[derive(Default)]
pub struct BufferRecorder {
    log: BattleLog,
}

impl BufferRecorder {
    pub fn into_log(self) -> BattleLog {
        self.log
    }
}

impl Recorder for BufferRecorder {
    fn domain(&mut self, event: DomainEvent) {
        self.log.domain.push(event);
    }

    fn trace(&mut self, event: TraceEvent) {
        self.log.trace.push(event);
    }

    fn metric(&mut self, event: MetricsEvent) {
        self.log.metrics.push(event);
    }
}

pub(crate) struct TeeRecorder<'a, R> {
    external: &'a mut R,
    buffer: BufferRecorder,
}

impl<'a, R: Recorder> TeeRecorder<'a, R> {
    pub(crate) fn new(external: &'a mut R) -> Self {
        Self { external, buffer: BufferRecorder::default() }
    }

    pub(crate) fn into_log(self) -> BattleLog {
        self.buffer.into_log()
    }
}

impl<R: Recorder> Recorder for TeeRecorder<'_, R> {
    fn domain(&mut self, event: DomainEvent) {
        self.external.domain(event.clone());
        self.buffer.domain(event);
    }

    fn trace(&mut self, event: TraceEvent) {
        self.external.trace(event.clone());
        self.buffer.trace(event);
    }

    fn metric(&mut self, event: MetricsEvent) {
        self.external.metric(event);
        self.buffer.metric(event);
    }
}
