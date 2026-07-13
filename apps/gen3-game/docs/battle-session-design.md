# 对战会话与回放状态机设计

状态：已实现（2026-07-14）

适用项目：`apps/gen3-game`

## 结论

当前项目需要新增一个纯逻辑 `battle-session` crate。

这个 crate 不实现伤害、命中、速度、换人优先级等对战规则。它只消费玩家视角的 `BattleTransition`，按事件推进可渲染的 `BattleScene`，并管理玩家何时可以输入。

本次重构要解决一个核心问题：领域结算状态和画面播放状态属于两个不同时间点，不能同时作为 UI 的事实来源。

重构完成后：

- `battle-domain` 决定发生什么。
- `battle-application` 决定玩家能看见什么。
- `battle-session` 决定何时呈现什么，以及何时允许输入。
- `game-ui` 决定画面和菜单如何显示。
- `game-host` 只负责窗口、时钟、输入、资源和组合根。

未来的天气、特性、道具和状态规则应生长在 `battle-domain`。它们通过语义事件进入 `battle-session`，不能在 session 中重新实现规则。

## 实现状态

本设计的主链路已经落地：

- `battle-domain` 的换人事件记录换人瞬间 HP。
- `battle-application` 提供固定视角的 `BattleCheckpoint` 和 `BattleTransition`。
- application 事件使用 `Participant::Own` 和 `Participant::Opponent`，不向呈现层暴露绝对 `Side`。
- 换人事件使用独立 `RevealedCombatant`。它不包含招式列表，避免提前公开同一 transition 后段才出现的信息。
- `battle-session` 已包含 coordinator、reducer 和 session 状态机。
- reducer 只使用 before 和 events 生成完整 `PlaybackStep`，并校验最终 Scene 等于 after 投影。
- `game-ui` 只通过 `BattleSessionSnapshot` 渲染战斗，不再调用领域 phase 推导页面。
- `game-host` 已删除 `BattleDisplayState`、`PlaybackFrame` 和后续伤害反推逻辑。

已覆盖主动换人后受击、击倒对手后自动换人、己方倒下后强制替换、回放期间拒绝输入、逃走和完整战斗流程。

对战相关 crate 的测试和 `-D warnings` Clippy 已通过。全 workspace 验证仍受未完成的 `map-editor` 改动阻塞：`layout::asset_position` 和 `layout::material_position` 尚不存在。该问题不属于本次对战重构。

## 背景

当前一次行动提交后，领域层会立即完成整个回合的结算。`BattleObservation` 随即变成回合结束后的最终状态。画面则需要按顺序播放出招、伤害、倒下、换人和结束消息。

因此系统同时存在两个时间点：

- 领域当前状态：完整结算后的状态。
- 呈现当前状态：回放正在播放的中间状态。

当前实现没有为呈现状态建立独立、完整的模型。`game-host` 和 `game-ui` 同时读取最终 `BattleObservation`、局部 `BattleDisplayState`、动画字段和页面字段。结果是多个对象共同决定同一帧画面。

已经出现的问题包括：

- 对手倒下后，贴图先切换，HP 仍属于上一只宝可梦。
- 玩家换人事件曾使用对手视角生成回放，导致事件归属反转。
- 新宝可梦换上时提前显示受伤后的 HP。
- 玩家宝可梦倒下时，倒下回放尚未结束，UI 已进入强制替换页面。
- 宿主需要根据后续伤害反推换人瞬间 HP。

这些问题不是独立的渲染缺陷。它们都来自同一个结构问题：中间回放缺少单一事实来源。

## 目标

- 为一场玩家视角的战斗建立明确会话状态机。
- 用完整 `BattleScene` 表示每个可渲染时间点。
- 用原子 `BattleTransition` 表示一次结算前、事件序列和结算后状态。
- 保证回放期间不读取领域最终状态构造中间帧。
- 保证 UI 页面只响应显式交互阶段，不自行推导领域阶段。
- 保证玩家视角在一次 transition 中固定，不能混入对手视角事件。
- 让回放归约、交互阶段和关键故事可以脱离 GPU 做纯单测。
- 为天气、特性、道具和状态效果保留稳定的语义事件通道。
- 明确未来何时需要继续拆分，而不是把所有扩展能力提前实现。

## 非目标

- 本次不实现天气、特性、道具或异常状态。
- 本次不实现通用规则插件系统。
- 本次不实现录像文件格式、网络同步、回滚或观战。
- 本次不实现任意动画编排引擎。
- 本次不改变现有伤害公式和动作规则。
- 本次不要求增加新的视觉素材。
- 本次不把 `battle-domain` 改造成完整事件溯源系统。

## 术语

### 领域状态

`battle-domain::Battle` 持有的真实战斗状态。它包含队伍、当前上场成员、HP、PP、回合、待提交命令和战斗阶段。

### 观察状态

`battle-application::BattleObservation` 是某个固定视角可见的领域状态。它负责隐藏对手未公开的信息。

### Transition

一次原子结算产生的结果。它包含结算前观察、按顺序排列的玩家视角事件和结算后观察。

### Scene

某一个可渲染时间点的完整战斗画面状态。名称、稳定 ID、HP、属性和当前表现状态必须来自同一个 Scene。

### Cue

一条语义化呈现提示。例如“某方使用招式”“某方受到伤害”“某方倒下”。Cue 不包含中文文案、颜色、GPU 资源或动画时长。

### Prompt

session 向 UI 发出的显式交互请求。例如选择行动或选择替换成员。

## 设计原则

### 一个时间点只有一个可渲染事实来源

渲染函数只能读取 `BattleSessionSnapshot`。它不能同时读取领域最终 observation 和回放中的局部 display。

### 中间帧只能从 before 和 events 推进

`transition.after` 只用于：

- 生成回放结束后的下一个交互阶段。
- 校验事件归约后的最终 Scene。

禁止读取 `after` 来填补中间帧缺失的数据。

### 事件记录语义事实，不记录表现形式

合法事件：

```rust
DamageApplied {
    target: PokemonId,
    amount: u32,
    remaining_hp: u32,
}
```

非法事件：

```rust
ShakeSpriteAndDrawRedHpBar {
    resource: ResourceId,
    duration_ms: u32,
}
```

删除当前 GPU UI 后，领域和应用事件仍应保持合理。

### session 不理解具体规则 ID

`battle-session` 可以穷举处理 `DamageApplied`、`Switched` 和 `WeatherChanged` 等事件种类。

`battle-session` 不能匹配具体 `MoveId`、`AbilityId`、`ItemId` 或物种 ID。出现这种代码说明 session 正在变成第二套规则引擎。

### UI 不推导战斗生命周期

UI 不能调用 `phase()` 或 `legal_actions()` 来猜测应该显示哪个页面。session 必须通过类型化 Prompt 明确告诉 UI 当前允许的交互。

## 目标依赖关系

下图中 `A -> B` 表示 A 的 Cargo 依赖指向 B：

```text
battle-application -> battle-domain
battle-session     -> battle-application
game-ui            -> battle-session

game-host -> battle-session
game-host -> game-ui
game-host -> game-data
```

`game-data` 和 `battle-domain` 不建立依赖。`game-host` 作为组合根，把静态数据转换成领域模型。

允许 `game-host` 作为组合根同时依赖 application、session 和 UI。禁止出现以下反向依赖：

- `battle-domain -> battle-session`
- `battle-session -> game-ui`
- `game-ui -> game-host`
- 任何纯逻辑 crate 依赖 `winit`、`wgpu` 或 GPU 资源类型

## Crate 职责

### battle-domain

负责：

- 动作合法性。
- 命令提交。
- 行动排序。
- 命中、伤害和效果结算。
- HP、PP、倒下、替换和结束状态。
- 完整、绝对视角的领域事件。

不负责：

- 玩家视角裁剪。
- 中文名称和提示文案。
- 动画、播放时长和菜单。
- 图片和声音资源。

领域事件必须携带重放规则事实所需的动态值。例如换人事件需要记录换人瞬间的当前 HP。否则下游只能从最终状态反推历史。

### battle-application

负责：

- 创建固定 `BattlePerspective`。
- 生成玩家可见 observation。
- 把绝对 `Side` 转换为玩家相对参与者。
- 隐藏对手 PP、未公开成员和未公开招式。
- 创建同一玩家视角的原子 `BattleTransition`。

不负责：

- 动画帧。
- 菜单页面。
- GPU 资源。
- 具体规则计算。

### battle-session

负责：

- 协调一次玩家动作和对手策略，产生完整 transition。
- 从 `before` 开始按顺序归约 observed events。
- 生成完整、不可变的 `PlaybackStep`。
- 管理播放游标。
- 管理等待行动、播放、强制替换和结束阶段。
- 向 UI 提供单一 `BattleSessionSnapshot`。

不负责：

- 决定伤害和命中。
- 修改动作优先级。
- 识别具体招式、特性或道具。
- 生成中文文案。
- 决定 GPU 动画和资源。

### game-ui

负责：

- 主菜单、招式页和队伍页的局部导航。
- 中文文案。
- HP 条、属性、招式信息和队伍信息布局。
- 把 semantic Cue 映射为可见提示和动画意图。
- 把 `BattleSessionSnapshot` 投影成 `GameView`。

不负责：

- 调用领域合法性规则。
- 判断回放是否结束。
- 推导是否需要强制替换。
- 修改领域状态。

### game-host

负责：

- 窗口和事件循环。
- 键盘事件转换。
- 定时调用 `session.advance()`。
- 根据 Scene 中的稳定 ID 解析贴图资源。
- 创建 application、session、对手策略和 UI。

不负责：

- 保存中间 HP。
- 根据领域事件反推回放状态。
- 生成玩家视角事件。
- 判断应该打开哪个菜单页面。

## Application 数据契约

### 相对参与者

领域层继续使用绝对 `Side::One` 和 `Side::Two`。应用层对外事件不应继续暴露绝对 Side。

```rust
pub enum Participant {
    Own,
    Opponent,
}
```

这样玩家使用 Side Two 或未来增加观战视角时，呈现层不需要重新解释绝对 Side。

### BattleTransition

```rust
pub struct BattleTransition {
    before: BattleObservation,
    events: Vec<ObservedBattleEvent>,
    after: BattleObservation,
}
```

三个字段必须来自同一个 perspective。字段对外只读。

transition 必须满足：

- `before` 是本次结算前的玩家观察。
- `events` 只包含 `before` 到 `after` 之间新增的玩家视角事件。
- `events` 保持领域发生顺序。
- `after` 是所有本次自动结算完成后的玩家观察。
- transition 中不能混入另一个 perspective 生成的 SubmitOutcome。

### Checkpoint

当前 host 通过事件日志长度手工截取新增事件。新 API 应把这一行为封装在 application 中。

建议使用私有字段 checkpoint：

```rust
pub struct BattleCheckpoint {
    viewer: Side,
    event_offset: usize,
    before: BattleObservation,
}

impl BattleApplication {
    pub fn checkpoint(
        &self,
        perspective: &BattlePerspective,
    ) -> BattleCheckpoint;

    pub fn transition_since(
        &self,
        checkpoint: BattleCheckpoint,
    ) -> Result<BattleTransition, TransitionError>;
}
```

`BattleCheckpoint` 字段不能公开。调用方不能替换 viewer 或事件位置。

后续如果 transition 需要跨网络或落盘，再把事件位置升级为类型安全 `BattleRevision`。本次不提前实现持久化协议。

### ObservedBattleEvent

应用事件必须完整使用玩家相对视角。

```rust
pub enum ObservedBattleEvent {
    TurnStarted {
        turn: u32,
    },
    OwnSwitched {
        from: TeamSlot,
        to: TeamSlot,
        pokemon: RevealedCombatant,
    },
    OpponentSwitched {
        pokemon: RevealedCombatant,
    },
    MoveUsed {
        actor: Participant,
        pokemon: PokemonId,
        used_move: RevealedMove,
    },
    DamageApplied {
        target: Participant,
        pokemon: PokemonId,
        amount: u32,
        remaining_hp: u32,
    },
    Fainted {
        participant: Participant,
        pokemon: PokemonId,
    },
    ReplacementRequired {
        participant: Participant,
    },
    BattleFinished {
        outcome: ObservedBattleOutcome,
    },
}
```

示例省略会心、未命中、属性效果、PP 和反作用力等现有事件。正式实现时必须迁移全部现有语义，不能通过 `_ => {}` 忽略。

### RevealedCombatant

换人事件必须携带换人时间点公开且可呈现的数据。

```rust
pub struct RevealedCombatant {
    pub id: PokemonId,
    pub name: String,
    pub level: u8,
    pub primary_type: PokemonType,
    pub secondary_type: Option<PokemonType>,
    pub current_hp: u32,
    pub max_hp: u32,
}
```

这里不包含 GPU 贴图 ID。贴图由 host 根据稳定 `PokemonId` 查询资源表。

对手换入后被同回合攻击时，`current_hp` 必须是换入瞬间的 HP，不是整个回合结束后的 HP。

## battle-session 内部结构

建议一个 crate 内先拆成三个模块，不再增加更多 crate。

```text
crates/battle-session/src/
  coordinator.rs
  reducer.rs
  session.rs
  lib.rs
```

### coordinator

负责把一次玩家操作完成为一个 transition：

1. 创建玩家 perspective checkpoint。
2. 提交玩家 Action。
3. 如需对手行动，调用注入的对手策略。
4. 如只需要对手自动替换，继续调用对手策略。
5. 所有自动结算停止后，从 checkpoint 创建玩家 transition。

对手策略是纯端口：

```rust
pub trait OpponentPolicy {
    fn choose_action(
        &mut self,
        observation: &BattleObservation,
        legal_actions: &[Action],
    ) -> Option<Action>;
}
```

策略只选择已有合法动作，不能直接修改 Battle。

### reducer

reducer 是纯函数或纯状态对象。它从 `before` 对应的 Scene 开始，只应用 events。

```rust
pub struct BattleSceneReducer {
    scene: BattleScene,
}

impl BattleSceneReducer {
    pub fn apply(
        &mut self,
        event: &ObservedBattleEvent,
    ) -> PlaybackStep;
}
```

每个 `PlaybackStep` 都包含完整 Scene。它不是局部 patch。

```rust
pub struct PlaybackStep {
    pub scene: BattleScene,
    pub cue: BattleCue,
}
```

`PlaybackStep` 不包含持续时间。host 或后续独立 `PresentationPolicy` 决定何时推进下一步。

### session

```rust
pub enum BattleSessionPhase {
    AwaitingAction(ActionPrompt),
    Playing(PlaybackCursor),
    AwaitingReplacement(ReplacementPrompt),
    Finished(FinishedPrompt),
}
```

阶段含义：

- `AwaitingAction`：玩家可以打开主菜单、招式页或主动换人页。
- `Playing`：输入锁定，只能推进回放。
- `AwaitingReplacement`：回放已经结束，玩家必须选择合法替换成员。
- `Finished`：结束回放已经完成，允许返回地图或关闭战斗。

不使用单独的 `is_playing`、`has_pending_playback` 和 `animation != Idle` 共同推导阶段。阶段 enum 是唯一生命周期事实来源。

## Scene 模型

```rust
pub struct BattleScene {
    pub own: CombatantScene,
    pub opponent: CombatantScene,
}

pub struct CombatantScene {
    pub id: PokemonId,
    pub name: String,
    pub level: u8,
    pub primary_type: PokemonType,
    pub secondary_type: Option<PokemonType>,
    pub current_hp: u32,
    pub max_hp: u32,
    pub condition: CombatantCondition,
}
```

`CombatantCondition` 只表达稳定的战斗状态，例如可战斗、倒下和未来的异常状态。受击属于瞬时 Cue，不能保存在稳定 Scene 中。这个类型不能引用 GPU 资源。

Scene 必须满足：

- 名称、HP、属性和贴图查询 ID 属于同一只宝可梦。
- `current_hp <= max_hp`。
- `Fainted` 时 `current_hp == 0`。
- 发生换人事件前，Scene 仍显示旧成员。
- 发生换人事件时，Scene 一次性切换新成员的完整状态。
- 发生伤害事件前，Scene 保持伤害前 HP。
- 发生伤害事件时，Scene 更新为事件中的 `remaining_hp`。

## Session 状态转换

```text
创建战斗
    |
    v
AwaitingAction
    |
    | submit(action)
    v
Playing
    |
    | advance()，仍有步骤
    +--------------------> Playing
    |
    | 最后一步完成
    v
根据 transition.after 选择：
    AwaitingAction
    AwaitingReplacement
    Finished
```

强制替换流程：

```text
Playing: 伤害
-> Playing: HP 归零
-> Playing: 倒下
-> Playing: 强制替换提示
-> 回放结束
-> AwaitingReplacement
```

禁止从伤害帧或倒下帧直接进入 `AwaitingReplacement`。

主动换人流程：

```text
AwaitingAction
-> 提交 Switch
-> Playing: 换人
-> Playing: 对手出招
-> Playing: 新成员受到伤害
-> 回放结束
-> AwaitingAction 或 AwaitingReplacement
```

## UI 契约

UI 只接收一个 session 快照：

```rust
pub struct BattleSessionSnapshot {
    pub scene: BattleScene,
    pub interaction: BattleInteraction,
    pub cue: Option<BattleCue>,
}
```

`BattleInteraction` 明确表示当前允许的操作：

```rust
pub enum BattleInteraction {
    ChooseAction(ActionPrompt),
    PlaybackLocked,
    ChooseReplacement(ReplacementPrompt),
    Finished(FinishedPrompt),
}
```

`game-ui` 可以保留局部菜单状态：

```rust
pub enum BattleMenuState {
    Main { selected: usize },
    Fight { selected: usize },
    Party {
        selected: TeamSlot,
        purpose: PartyPurpose,
    },
    Hidden,
}
```

UI 只能在 interaction 显式变化时切换页面：

- `ChooseAction` 进入 Main。
- 玩家选择“战斗”进入 Fight。
- 玩家选择“宝可梦”进入主动 Party。
- `PlaybackLocked` 进入 Hidden。
- `ChooseReplacement` 进入强制 Party。
- `Finished` 进入结束状态。

删除每帧调用 `reconcile(observation, actions)` 的设计。

## 文案与动画边界

`BattleCue` 是语义提示：

```rust
pub enum BattleCue {
    TurnStarted { turn: u32 },
    Switched { participant: Participant },
    MoveUsed { participant: Participant, move_id: MoveId },
    DamageApplied { participant: Participant, amount: u32 },
    Missed { participant: Participant },
    Fainted { participant: Participant },
    BattleFinished,
}
```

`game-ui` 根据 Cue 和 Scene 生成中文文本。host 根据 Cue 决定当前固定播放间隔和动画资源。

当未来出现终端客户端、录像播放器或高速模拟器时，再把 Cue 到播放时长和动画的映射抽成独立 `PresentationPolicy`。本次不新增该抽象。

## 永久不变量

这些不变量不能因为加入新招式、天气、特性或道具而修改：

- HP 始终位于 `0..=max_hp`。
- 倒下成员不能行动或被选为替换目标。
- Finished 后不再接受动作。
- 非法动作不能改变状态、事件日志或 RNG。
- 相同种子、初始状态和命令产生相同 transition。
- transition 的 before、events 和 after 使用同一个 perspective。
- 每个 observed event 引用的 ID 都已经公开或在该事件中公开。
- session 在 Playing 时拒绝玩家动作。
- session 在回放队列未结束时不能进入 AwaitingReplacement。
- 每个 PlaybackStep 中名称、HP、属性和稳定 ID 属于同一成员。
- reducer 应用全部事件后的最终 Scene 与 after 投影一致。

核心归约定律：

```rust
reduce(project(transition.before), transition.events)
    == project(transition.after)
```

## 可变规则与永久不变量的区别

动作顺序不是永久不变量。

当前规则规定普通换人先于普通招式。但未来可能存在拦截换人的招式或特性。此时应修改领域动作计划和对应规则测试，不能修改 session 来调整播放顺序。

测试必须区分：

- 永久不变量：任何规则下都成立。
- 默认规则：在没有特殊效果时成立。
- 特例规则：满足明确触发条件时覆盖默认规则。

## 测试策略

### battle-domain

测试规则和事件顺序：

- 普通攻击。
- 主动换人。
- 双方速度和优先级。
- 击倒和强制替换。
- 反作用力和双方同时倒下。
- 战斗结束和逃走。
- 新规则与既有规则的交互。

### battle-application

测试视角和隐私：

- Side One 和 Side Two 分别得到正确 Own/Opponent。
- 对手未公开 PP 不泄漏。
- 对手首次换入时在事件中公开必要快照。
- 一个 transition 不能混入另一个 perspective 的事件。
- checkpoint 只能使用创建它的 application 状态。

### battle-session

测试归约定律和阶段：

- 主动换人后被攻击。
- 对手主动换人后被攻击。
- 击倒对手并自动替换。
- 玩家倒下并进入强制替换。
- 双方同时倒下。
- 未命中、会心、属性效果和反作用力。
- 逃走和正常结束。
- 回放期间拒绝输入。
- 回放结束后才发出下一个 Prompt。

每个步骤至少断言：

```text
session phase
own id / hp
opponent id / hp
cue
interaction
```

### game-ui

测试纯投影和菜单：

- 每种 interaction 进入正确菜单。
- PlaybackLocked 时不显示可操作菜单。
- 强制替换不能退出 Party。
- 招式属性、威力和 PP 来自 prompt 数据。
- 队伍缩略图和 HP 来自同一个成员快照。

### E2E

保留少量完整故事：

- 从地图进入战斗，完成行动并返回地图。
- 玩家逃走。
- 玩家倒下、选择替换并继续战斗。
- 一场完整六对六战斗结束。

E2E 不承担所有规则组合。大部分时序问题应在 `battle-session` 纯测试中发现。

### 生成式检查

第一阶段不增加测试依赖。先使用固定种子循环检查永久不变量：

```rust
for seed in 0..1024 {
    verify_transition_laws(seed);
}
```

当天气、特性和道具导致组合数量明显增加后，再评估仅测试依赖 `proptest`。不要为了本次重构提前引入。

## 天气、特性和插件的演化方向

### 天气

天气状态和效果属于 `battle-domain`。领域层产生 `WeatherStarted`、`WeatherDamage` 和 `WeatherEnded` 等语义事件。

应用层决定天气和相关来源是否公开。session 只按事件更新 Scene 和 Cue。UI 决定天气文案和视觉表现。

### 特性和道具

触发条件、效果和执行顺序属于 `battle-domain`。隐藏特性的公开策略属于 `battle-application`。

session 不能出现：

```rust
if ability_id == INTIMIDATE { ... }
```

### 规则插件

本次不建立任意回调插件：

```rust
trait Plugin {
    fn on_any_event(...);
}
```

当至少三种独立规则来源需要插入同一个结算阶段时，再在领域层设计类型化效果调度器。候选触发阶段包括：

```rust
pub enum EffectTrigger {
    BeforeActionOrder,
    BeforeMove,
    BeforeDamage,
    AfterDamage,
    AfterFaint,
    OnSwitchIn,
    OnSwitchOut,
    EndOfTurn,
}
```

触发阶段和同阶段排序必须由领域层明确规定，不能依赖插件注册顺序。

### 多客户端

当出现第二种具有不同播放需求的客户端时，例如终端、高速模拟器或录像播放器，再拆分：

```text
BattleTransition
      |
SemanticReducer
      |
BattleScene
      |
PresentationPolicy
  /       |        \
GPU UI   终端     高速模拟器
```

在此之前，`battle-session::reducer` 可以同时承担 semantic reducer 的职责。

## 新方案的腐化信号

出现下面任一情况时，应停止加功能并检查边界：

- 增加一条战斗规则后，多个 crate 分别重新实现它的触发条件、执行顺序或数值计算。
- `battle-session` 匹配具体 MoveId、AbilityId、ItemId 或物种 ID。
- reducer 读取 transition.after 来构造中间步骤。
- game-ui 调用 phase 或 legal_actions 推导页面。
- game-host 再次保存独立 HP、当前成员或战斗阶段。
- host 到处出现 `if is_playing()`。
- 领域或应用事件包含中文文案、GPU ResourceId、颜色或动画时长。
- 新事件通过 `_ => {}` 被静默忽略。
- 同一个替换、倒下或结束条件在多个 crate 重复判断。
- 为特殊招式增加跨层布尔字段。
- 修改规则时需要批量接受无法解释的黄金快照变化。

不同压力应留在对应边界：

```text
规则复杂度增加       -> battle-domain
信息公开策略增加     -> battle-application
回放和交互增加       -> battle-session
客户端和视觉增加     -> game-ui / PresentationPolicy
窗口和资源增加       -> game-host
```

## 迁移计划

### 阶段 1：冻结现有行为

- 保留已经覆盖换人、击倒和强制替换时序的回归测试。
- 补充双方 perspective 对称测试。
- 明确当前所有 application event 的公开语义。

完成标准：现有行为有测试保护，重构失败可以定位到具体边界。

### 阶段 2：建立 BattleTransition

- 在 `battle-application` 增加 checkpoint 和 transition API。
- 把 observed event 全部改成玩家相对参与者。
- 补全换人瞬间快照所需的领域事件事实。
- 禁止 host 直接拼接不同 perspective 的 SubmitOutcome。

完成标准：一次玩家操作只产生一个固定玩家视角的 transition。

### 阶段 3：新增 battle-session reducer

- 新增 `battle-session` crate。
- 实现 Scene、Cue、PlaybackStep 和纯 reducer。
- 实现核心归约定律。
- 使用现有故事覆盖主动换人、击倒和强制替换。

完成标准：不依赖 game-host 和 game-ui 即可完整验证回放状态。

### 阶段 4：实现显式 session 状态机

- 实现 AwaitingAction、Playing、AwaitingReplacement 和 Finished。
- 注入对手策略。
- 让 Prompt 持有 UI 所需的合法动作和队伍快照。
- 回放结束后才根据 after 进入下一个阶段。

完成标准：阶段 enum 是输入权限和回放生命周期的唯一来源。

### 阶段 5：替换 UI 契约

- `project_battle` 只接收 session snapshot 和资源解析结果。
- UI 菜单响应 Prompt，不再 reconcile observation。
- PlaybackLocked 时隐藏菜单。
- 队伍页面继续使用占位缩略图。

完成标准：UI 不再读取领域阶段或合法动作推导页面。

### 阶段 6：缩减 host

- host 只转发输入和定时推进。
- host 根据 Scene 的 PokemonId 查找贴图。
- 删除 display、message、animation 和 playback 的重复状态。
- 删除后续伤害反推逻辑。

完成标准：host 不再实现战斗状态转换或回放归约。

### 阶段 7：删除兼容路径

删除：

- `BattleDisplayState`。
- host 内部 `PlaybackFrame`。
- `later_damage_to_switched_pokemon()`。
- `BattleUiState::reconcile()`。
- 同时接收 observation 和 display 的投影接口。
- 通过多个布尔值推导 Playing 的逻辑。

完成标准：项目只有一条战斗呈现数据链路。

## 验证命令

实现期间优先运行相关 crate：

```powershell
cargo test -p battle-domain
cargo test -p battle-application
cargo test -p battle-session
cargo test -p game-ui -p game-host -p game-e2e
cargo clippy -p battle-domain -p battle-application -p battle-session -p game-ui -p game-host -p game-e2e --all-targets -- -D warnings
```

跨 crate 迁移完成后运行：

```powershell
cargo test --workspace
```

## 完成标准

- 新增 `battle-session` 纯逻辑 crate。
- 一个玩家动作只产生一个固定玩家视角的 `BattleTransition`。
- 回放每一步都有完整 `BattleScene`。
- reducer 不读取 after 构造中间步骤。
- 回放最终 Scene 与 after 投影一致。
- UI 只读取 `BattleSessionSnapshot`。
- host 不再保存独立战斗显示状态。
- 回放期间不能打开菜单。
- 回放结束前不能进入强制替换页。
- 现有 workspace 测试通过。
- 相关 crate Clippy 在 `-D warnings` 下通过。

## 已采用决策

### 1. crate 名称

建议使用 `battle-session`。它表达一场玩家视角战斗的生命周期，不把职责限定为动画播放器。

备选名称是 `battle-presentation`。这个名称无法覆盖输入权限、对手策略和 Prompt，因此不建议。

### 2. session 是否协调对手策略

建议协调。这样一次玩家操作可以原子地产生完整 transition，host 不再接触两个 perspective 的 SubmitOutcome。

对手策略本身通过 `OpponentPolicy` 注入，不写死在 session。

### 3. 播放时长放在哪里

建议第一阶段继续由 host 根据 Cue 使用固定间隔。`PlaybackStep` 不保存毫秒数。

出现第二种客户端后，再抽 `PresentationPolicy`。

### 4. 换人事件携带多少数据

建议应用事件携带完整公开 `RevealedCombatant`，领域事件只补充事件时间点所需的动态事实。

不要在领域事件中加入 GPU 资源和本地化文本。

### 5. 是否现在实现 EffectScheduler

不实现。等天气、特性、道具等至少三类规则需要插入同一结算阶段时再设计。

本次只保留明确边界和腐化检测标准。
