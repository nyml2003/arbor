# 天气与回合结束结算设计

状态：提案，待审查

适用项目：`apps/gen3-game`

## 结论

当前架构可以引入天气。

`battle-application`、`battle-session` 和 `game-ui` 已经具备稳定的事件、Scene 和回放边界。天气不需要建立新的 crate，也不需要修改现有层级关系。

真正的前置工作在 `battle-domain`：

- 当前 `Move` 只允许有威力的攻击招式。
- 当前所有 `UseMove` 都直接进入伤害流程。
- 当前回合只有行动阶段，没有独立的回合结束阶段。
- 当前伤害函数没有类型化的天气输入。

第一版应先建立状态招式、天气状态和回合结束结算。随后再把天气事件接入 application、session 和 UI。

本次不实现通用 `EffectScheduler`。天气只增加明确的领域类型和私有结算函数。等特性、道具和异常状态同时需要插入相同阶段时，再设计类型化效果调度器。

## 当前基础

现有各层的准备情况如下：

| 层 | 当前状态 | 天气接入工作 |
|---|---|---|
| `battle-domain` | 未就绪 | 状态招式、天气状态、回合结束结算、天气伤害 |
| `battle-application` | 基本就绪 | 天气观察和相对视角事件 |
| `battle-session` | 已就绪 | Scene 天气字段、Cue 和 reducer 分支 |
| `game-ui` | 已就绪 | 天气名称、消息和常驻状态显示 |
| `game-host` | 基本就绪 | 把数据招式映射为领域招式 |
| `game-data` | 已有必要原始字段 | 已包含 identifier、伤害类别、威力和 PP |

当前 `Move` 使用必填 `power: u16`，并拒绝零威力。当前 `use_regular_move` 在消耗 PP 后直接调用伤害结算。天气招式无法通过这条路径表达。

`game-data` 已区分 `Physical`、`Special` 和 `Status`。当前随机队伍只选择有威力招式，因此状态招式不会进入领域模型。

## 目标

- 支持雨天、大晴天、沙暴和冰雹。
- 支持通过招式开始和替换天气。
- 支持五回合天气持续时间。
- 支持雨天和大晴天的属性伤害修正。
- 支持沙暴和冰雹的回合结束伤害。
- 支持天气导致倒下、强制替换、胜负和平局。
- 保持无天气战斗的既有结果和事件顺序。
- 保持 transition 归约定律成立。
- 为未来的天气特性和延长天气道具保留类型边界。
- 让天气规则可以脱离 UI 和 GPU 做纯单测。

## 非目标

- 本次不实现天气特性。
- 本次不实现潮湿岩石、炽热岩石等延长天气道具。
- 本次不实现打雷、日光束、气象球和光合作用等天气特例。
- 本次不实现天气影响命中率。
- 本次不实现特性带来的天气免疫或天气回血。
- 本次不实现雾、乱流、原始回归等后世代天气。
- 本次不实现任意回调插件。
- 本次不把 PokeAPI 的 effect ID 当成可执行规则。

## 术语

### 天气种类

天气的稳定身份。第一版只包含雨天、大晴天、沙暴和冰雹。

### 天气状态

战斗当前持有的天气种类和剩余持续时间。没有天气使用 `Option::None`，不增加 `Clear` 伪天气。

### 天气来源

引发天气变化的规则来源。第一版只有招式来源。未来可以增加特性和战斗环境来源。

### 回合结束阶段

双方行动完成后、战斗进入下一次输入前的领域结算阶段。天气持续时间和天气伤害都在此阶段处理。

### 主效果

一次招式成功执行时必须发生的核心效果。第一版只区分直接伤害和开始天气。

## 领域边界

### battle-domain

负责：

- 天气类型和值域。
- 天气开始、替换、计时和结束。
- 天气招式是否成功。
- 天气对伤害的修正。
- 回合结束天气伤害和属性免疫。
- 天气事件顺序。
- 天气导致的倒下、替换和胜负。

不负责：

- 中文天气名称。
- 天气图标、颜色和背景动画。
- 玩家视角转换。
- 播放持续时间。

### battle-application

负责：

- 把天气来源中的绝对 `Side` 转成相对 `Participant`。
- 在 observation 中公开当前天气。
- 生成同一玩家视角的天气事件。

天气本身属于公开战场信息。第一版不裁剪天气种类和剩余时间。

### battle-session

负责：

- 从天气事件更新 `BattleScene`。
- 为需要展示的天气事件生成 `BattleCue`。
- 在天气回放结束后再进入替换或行动 Prompt。
- 继续验证最终 Scene 与 transition.after 投影一致。

session 不计算天气伤害，不判断属性免疫，也不修改持续时间。

### game-ui

负责：

- 天气中文名称。
- 天气开始、结束和伤害消息。
- 当前天气的常驻文字显示。
- 后续天气图标和视觉效果。

### game-host

负责把 `game-data::MoveRecord` 转换为领域 `Move`。host 不执行天气规则。

## 天气模型

建议在 `battle-domain/src/weather.rs` 中定义：

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WeatherKind {
    Rain,
    HarshSunlight,
    Sandstorm,
    Hail,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WeatherDuration {
    TurnsRemaining(u8),
    Persistent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WeatherState {
    kind: WeatherKind,
    duration: WeatherDuration,
}
```

第一版招式天气使用 `TurnsRemaining(5)`。

`Persistent` 第一版不创建，但保留给后续天气特性和战斗环境。领域构造器必须拒绝 `TurnsRemaining(0)`。

`Battle` 增加：

```rust
weather: Option<WeatherState>,
```

并提供只读访问器：

```rust
pub const fn weather(&self) -> Option<WeatherState>;
```

外部不能直接修改天气。天气只能通过领域结算方法变化。

## 招式主效果

当前招式模型把“招式”和“伤害招式”视为同一个概念。天气接入前必须拆开。

建议模型：

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MovePrimaryEffect {
    Damage {
        power: u16,
    },
    StartWeather {
        weather: WeatherKind,
    },
}
```

`Move` 改为持有：

```rust
primary_effect: MovePrimaryEffect,
```

构造入口分开：

```rust
Move::damaging(..., power, ...)
Move::weather(..., weather, ...)
```

现有 `Move::new` 可以暂时作为 `Move::damaging` 的兼容入口。兼容入口应在调用方迁移完成后删除。

访问器调整为：

```rust
pub const fn primary_effect(&self) -> MovePrimaryEffect;
pub const fn power(&self) -> Option<u16>;
```

UI 对 `None` 显示 `威--`。

第一版不使用 `Vec<MoveEffect>`。任意效果列表会提前引入效果排序、重复效果和失败语义。等出现“造成伤害并附加第二效果”的真实需求后，再增加类型化次要效果。

## 招式执行

`use_regular_move` 继续统一处理：

1. 发布 `MoveUsed`。
2. 消耗 PP。
3. 发布 `PpSpent`。
4. 根据 `MovePrimaryEffect` 分派。

建议结构：

```rust
match battle_move.primary_effect() {
    MovePrimaryEffect::Damage { power } => {
        self.resolve_damage_move(..., power);
    }
    MovePrimaryEffect::StartWeather { weather } => {
        self.resolve_weather_move(..., weather);
    }
}
```

天气招式作用于战场，不对对手进行命中判定。第一版使用 `Accuracy::AlwaysHit`。

同种天气已经生效时，天气招式失败。PP 仍然消耗，并产生类型化失败事件：

```rust
pub enum MoveFailureReason {
    WeatherAlreadyActive {
        weather: WeatherKind,
    },
}

BattleEvent::MoveFailed {
    side: Side,
    pokemon: PokemonId,
    used_move: UsedMove,
    reason: MoveFailureReason,
}
```

不同天气生效时，新天气替换旧天气，持续时间重新设为五回合。

## 第一版天气规则

### 雨天

- 水属性招式伤害乘以 `3 / 2`。
- 火属性招式伤害乘以 `1 / 2`。
- 不产生回合结束伤害。

### 大晴天

- 火属性招式伤害乘以 `3 / 2`。
- 水属性招式伤害乘以 `1 / 2`。
- 不产生回合结束伤害。

### 沙暴

- 回合结束时对不含岩石、地面或钢属性的在场成员造成伤害。
- 伤害为最大 HP 的 `1 / 16`，向下取整，最少 1 点。
- 不实现后世代的岩石属性特防提升。

### 冰雹

- 回合结束时对不含冰属性的在场成员造成伤害。
- 伤害为最大 HP 的 `1 / 16`，向下取整，最少 1 点。

### 暂不实现的特例

- 打雷的命中修正。
- 日光束的蓄力变化。
- 气象球的属性和威力变化。
- 晨光、月光和光合作用的回复变化。
- 冰冻和解冻概率变化。
- 特性免疫、回血和速度变化。

这些规则以后进入对应招式或特性实现，不能写进 session 或 UI。

## 伤害上下文

当前伤害函数参数已经较多。天气不应继续增加松散参数或布尔值。

建议增加类型化上下文：

```rust
pub(crate) struct DamageContext {
    pub power: u16,
    pub move_type: Option<PokemonType>,
    pub category: DamageCategory,
    pub critical: bool,
    pub random_percent: u8,
    pub weather: Option<WeatherKind>,
}
```

项目第一版固定以下整数运算顺序：

```text
基础伤害
-> 会心
-> 天气
-> STAB
-> 属性克制
-> 随机数
-> 非免疫伤害至少为 1
```

每一步都使用整数乘除。无天气时必须与现有 fixture 完全一致。

`DamageContext` 不是通用 modifier 列表。它只表达当前伤害公式的明确输入。特性和道具加入后，如果修正项明显增多，再设计类型化 modifier pipeline。

## 回合结算顺序

建议把 `resolve_turn` 拆成明确阶段：

```text
发布 TurnStarted
-> 计算行动顺序
-> 执行第一个行动
-> 执行第二个合法行动
-> 检查逃走或一方队伍全灭
-> 结算回合结束天气
-> 推进天气持续时间
-> 检查天气导致的全灭
-> 生成强制替换或下一回合阶段
```

### 直接击倒最后一只成员

如果行动阶段已经使一方没有可战斗成员，战斗立即结束，不再结算天气伤害。

这避免胜方在战斗已经结束后继续受到沙暴或冰雹伤害。

### 击倒当前成员但仍有后备成员

倒下成员不再受到天气伤害。另一方仍在场且未倒下的成员正常结算天气。天气结束后再进入强制替换。

### 双方天气伤害

回合结束阶段开始时先冻结两个天气伤害候选。然后按稳定顺序发布事件并应用伤害。

第一版使用 `Side::One`、`Side::Two` 的稳定事件顺序，但结果按同一阶段处理。第一方倒下不能阻止第二方已经确定的天气伤害。

这样双方最后一只成员可以同时因天气倒下并形成平局。后续出现依赖速度的回合结束特性时，再把同阶段排序升级为领域规则。

### 逃走

逃走立即进入 `Finished`。不结算天气伤害，不推进天气持续时间。

## 天气持续时间

天气招式成功后创建 `TurnsRemaining(5)`。

开始天气的当前回合计入五回合：

1. 执行天气招式。
2. 当前回合结束时应用天气伤害。
3. 剩余时间从 5 变为 4。

当剩余时间为 1 时：

1. 先结算最后一次天气伤害。
2. 再发布天气结束事件。
3. observation 和 Scene 中天气变为 `None`。

`Persistent` 不递减，也不产生 `WeatherAdvanced`。

## 领域事件

建议增加：

```rust
pub enum WeatherSource {
    Move {
        side: Side,
        pokemon: PokemonId,
        used_move: UsedMove,
    },
}

pub enum BattleEvent {
    WeatherChanged {
        previous: Option<WeatherKind>,
        current: WeatherState,
        source: WeatherSource,
    },
    WeatherAdvanced {
        weather: WeatherKind,
        remaining_turns: u8,
    },
    WeatherEnded {
        weather: WeatherKind,
    },
    MoveFailed {
        side: Side,
        pokemon: PokemonId,
        used_move: UsedMove,
        reason: MoveFailureReason,
    },
}
```

扩展伤害来源：

```rust
pub enum DamageSource {
    Move { ... },
    Recoil { ... },
    Weather {
        weather: WeatherKind,
    },
}
```

天气伤害继续使用现有 `Damage` 和 `Fainted` 事件。这样 HP、倒下和替换仍走同一条链路。

事件顺序示例：

```text
WeatherChanged
Damage { source: Weather, target: Side One }
Fainted（如果需要）
Damage { source: Weather, target: Side Two }
Fainted（如果需要）
WeatherAdvanced 或 WeatherEnded
ForcedReplacement 或 BattleFinished
```

不增加带中文文案或动画时间的领域事件。

## Application 契约

`BattleObservation` 增加：

```rust
weather: Option<WeatherState>,
```

application 事件使用相对来源：

```rust
pub enum ObservedWeatherSource {
    Move {
        participant: Participant,
        pokemon: PokemonId,
        used_move: UsedMove,
    },
}
```

`MoveFailed` 中的绝对 Side 同样转换为 `Participant`。

天气是公开战场状态。before 和 after 都包含各自时间点的天气，不能从 after 回填中间天气步骤。

## Session 契约

`BattleScene` 增加战场状态：

```rust
pub struct BattleScene {
    own: CombatantScene,
    opponent: CombatantScene,
    field: FieldScene,
}

pub struct FieldScene {
    weather: Option<WeatherScene>,
}

pub struct WeatherScene {
    kind: WeatherKind,
    duration: WeatherDuration,
}
```

建议 Cue：

```rust
pub enum BattleCue {
    WeatherChanged {
        previous: Option<WeatherKind>,
        current: WeatherKind,
    },
    WeatherEnded {
        weather: WeatherKind,
    },
    MoveFailed {
        participant: Participant,
        reason: MoveFailureReason,
    },
    DamageApplied {
        participant: Participant,
        amount: u32,
        source: DamageCueSource,
    },
}
```

`WeatherAdvanced` 更新 Scene，但默认不生成独立 Cue 和播放步骤。它必须被 reducer 显式处理，不能通过 `_ => {}` 忽略。

天气伤害、倒下和天气结束回放完成后，session 才能进入 `AwaitingReplacement` 或 `Finished`。

归约定律保持不变：

```rust
reduce(project(transition.before), transition.events)
    == project(transition.after)
```

## UI 设计

第一版只增加文字，不增加天气素材。

战斗主画面增加一个稳定的小型天气标签，例如：

```text
天气 雨天 4
```

`Persistent` 不显示数字。

建议文案：

- 雨开始下了。
- 阳光变得强烈了。
- 沙暴刮了起来。
- 开始下冰雹了。
- 沙暴伤害了妙蛙种子。
- 雨停了。
- 阳光恢复了正常。
- 沙暴平息了。
- 冰雹停了。
- 天气没有发生变化。

文案由 UI 根据 Cue 和 Scene 生成。领域事件不保存这些字符串。

后续提供图标后，UI 根据 `WeatherKind` 查找资源。天气图标 ID 不能进入领域或 session。

## 数据接入

第一版不需要修改 PokeAPI CSV 来源。当前数据已经包含：

- 稳定招式 identifier。
- `DamageClass::Status`。
- PP、属性和优先级。

组合根显式映射四个 identifier：

```text
rain-dance -> Rain
sunny-day  -> HarshSunlight
sandstorm  -> Sandstorm
hail       -> Hail
```

不能根据 PokeAPI effect 文本动态执行规则，也不能把 effect ID 当成领域行为。

随机演示队伍第一阶段可以继续只选择伤害招式。天气使用手工 fixture 队伍验证。等队伍配置接入后，再允许租借队伍显式携带天气招式。

如果随机队伍以后纳入状态招式，必须保证：

- 每只成员至少有一个伤害招式。
- 每只成员最多选择一个天气招式。
- 状态招式不能因为缺少 power 被误判为坏数据。

只有当招式规则映射明显挤占 roster 职责，或第二个客户端需要相同映射时，才拆独立的规则目录 adapter。第一版不新增 crate。

## 错误与失败

区分模型错误、命令错误和正常招式失败。

### 模型错误

- 零回合天气持续时间。
- 伤害主效果的威力为零。
- 状态主效果错误携带伤害威力。

这些错误由构造器返回 `ValidationError`。

### 命令错误

- 招式不存在。
- PP 已耗尽。
- 当前阶段不能行动。

继续使用 `BattleError`。

### 正常招式失败

同种天气已经生效属于合法命令产生的正常战斗结果。它不返回 `BattleError`，而是发布 `MoveFailed`。

这样回放、录像和 AI 都能观察到失败结果。

## 永久不变量

- 天气状态最多存在一个。
- `TurnsRemaining` 始终大于零。
- 无天气时既有战斗结果不变。
- 天气伤害不能超过当前 HP。
- 倒下成员不承受天气伤害。
- 属性免疫只由 domain 判断。
- 逃走后不结算天气。
- Finished 后天气不再推进。
- 天气导致的倒下必须在替换 Prompt 前完成回放。
- session 不匹配具体天气招式 ID。
- UI 不计算天气剩余时间和伤害。
- host 不保存独立天气状态。
- reducer 应用全部事件后仍满足 Scene 与 after 一致。

## 测试策略

### battle-domain

- 四种天气招式成功并消耗 PP。
- 天气招式不产生直接伤害。
- 同种天气重复使用发布 `MoveFailed`。
- 新天气替换旧天气并重置为五回合。
- 雨天和大晴天修正正确属性伤害。
- 非相关属性伤害不变。
- 无天气 fixture 完全不变。
- 沙暴对岩石、地面和钢属性免疫。
- 冰雹对冰属性免疫。
- 双属性包含免疫属性时不受伤害。
- 天气伤害按最大 HP 的十六分之一取整，最少 1 点。
- 天气从 5 递减到结束。
- 开始天气的回合计入持续时间。
- 直接击倒最后成员后不触发天气。
- 天气导致单方强制替换。
- 天气导致双方强制替换。
- 天气导致一方获胜或双方平局。
- 逃走不推进天气。
- 相同种子和命令产生相同事件。

### battle-application

- Side One 和 Side Two 观察到相同天气。
- 天气来源正确转换为 Own 和 Opponent。
- `MoveFailed` 不泄漏隐藏信息。
- transition.before 和 transition.after 持有正确时间点天气。

### battle-session

- WeatherChanged 步骤一次性更新完整 FieldScene。
- WeatherAdvanced 更新剩余时间但不产生多余提示帧。
- WeatherEnded 清除 Scene 天气。
- 天气伤害更新正确参与者 HP。
- 天气倒下回放结束后才进入替换页。
- 天气结束后的最终 Scene 等于 after 投影。

### game-ui

- 四种天气显示正确中文名称。
- 状态招式显示 `威--`。
- 天气伤害文案包含正确成员名称。
- `Persistent` 不显示剩余回合。
- PlaybackLocked 时不显示可操作菜单。

### E2E

- 玩家使用天气招式，天气持续并自然结束。
- 沙暴或冰雹击倒成员后完成强制替换。

## 规则 fixture

现有 `battle-rules-v0.1.json` 明确写有 `no weather`。不能直接修改这份已批准 fixture 的含义。

天气实现时应新增：

```text
fixtures/battle-rules-v0.2.json
```

v0.2 应记录：

- 四种天气的第一版规则。
- 伤害修正顺序。
- 五回合持续时间语义。
- 回合结束天气伤害顺序。
- 同天气重复使用的失败行为。
- 暂不实现的天气特例。
- 至少一组天气伤害向量。
- 至少一条完整天气事件日志。

v0.1 测试继续保留，证明无天气行为没有回归。

## 不引入 EffectScheduler 的原因

天气目前只需要两个明确入口：

```text
伤害计算
回合结束
```

可以先使用：

```rust
weather_damage_factor(...)
resolve_end_of_turn_weather(...)
```

这两个函数有明确输入、输出和调用位置。通用触发器暂时只会增加注册顺序、优先级和生命周期问题。

当至少三类独立来源需要共享同一结算阶段时，再设计：

```rust
pub enum EffectTrigger {
    BeforeMove,
    BeforeDamage,
    AfterDamage,
    OnSwitchIn,
    OnSwitchOut,
    EndOfTurn,
}
```

候选来源包括天气、特性、道具、异常状态和场地效果。同阶段排序必须由 domain 明确规定，不能依赖注册顺序。

## 腐化信号

出现以下情况时应停止扩展并检查边界：

- `battle-session` 判断沙暴属性免疫。
- `game-ui` 计算剩余天气回合。
- `game-host` 保存独立天气状态。
- `game-host` 直接扣除天气 HP。
- `calculate_damage` 增加多个天气布尔参数。
- 具体天气招式 ID 出现在 reducer 或 UI。
- PokeAPI effect 文本被当成脚本执行。
- 新天气事件通过 wildcard 静默忽略。
- 同一天气持续时间在 domain、session 和 UI 分别递减。
- 为每个天气增加跨层布尔字段。
- 为了一个天气特例提前增加任意回调插件。

## 实施阶段

### 阶段 1：扩展招式模型

- 增加 `MovePrimaryEffect`。
- 增加伤害招式和天气招式构造器。
- 把 `power()` 改为可选值。
- 迁移现有招式 fixture 和调用方。
- 保证无天气测试不变。

完成标准：domain 可以合法构造和使用一个不造成伤害的天气招式。

### 阶段 2：实现领域天气

- 增加 `weather.rs`。
- 让 `Battle` 持有天气。
- 增加天气招式结算和失败事件。
- 增加 `DamageContext` 和雨晴修正。
- 增加回合结束阶段和沙暴、冰雹伤害。
- 增加天气持续时间。

完成标准：纯 domain 测试可以覆盖完整五回合天气故事。

### 阶段 3：接入 application

- observation 公开天气。
- 天气来源转换为相对参与者。
- 补充双方 perspective 对称测试。

完成标准：一个固定视角 transition 完整包含天气开始、伤害、推进和结束。

### 阶段 4：接入 battle-session

- Scene 增加 FieldScene。
- reducer 处理全部天气事件。
- Cue 区分天气变化、结束、失败和天气伤害。
- 增加天气归约定律测试。

完成标准：不依赖 UI 即可重放完整天气回合。

### 阶段 5：接入 UI 和数据转换

- UI 显示天气文字和状态招式威力。
- host 映射四个天气招式 identifier。
- 增加天气 E2E 故事。

完成标准：玩家可以使用天气招式，并看到持续、伤害和结束全过程。

### 阶段 6：建立 v0.2 规则 fixture

- 新增 `battle-rules-v0.2.json`。
- 保留 v0.1 fixture 和测试。
- 固定天气规则和事件 oracle。

完成标准：天气语义经过审查后成为可回归规则。

## 验证命令

```powershell
cargo test -p battle-domain
cargo test -p battle-application
cargo test -p battle-session
cargo test -p game-ui -p game-host -p game-e2e
cargo clippy -p battle-domain -p battle-application -p battle-session -p game-ui -p game-host -p game-e2e --all-targets -- -D warnings
```

天气跨层接入完成后运行：

```powershell
cargo test --workspace
```

## 完成标准

- 四种天气具有类型安全领域模型。
- 状态招式不再伪装成零威力伤害招式。
- `Battle` 是天气状态的唯一事实来源。
- 天气伤害和持续时间只在 domain 计算。
- 无天气 v0.1 fixture 保持不变。
- 天气 v0.2 fixture 固定第一版规则。
- application 只暴露相对视角天气来源。
- session 只通过事件归约天气 Scene。
- UI 不读取领域 phase 或计算天气规则。
- host 不保存天气状态。
- 天气导致的倒下不会提前打开替换页。
- 天气 transition 满足最终 Scene 归约定律。
- 相关测试和严格 Clippy 通过。

## 待审查决策

### 1. 同种天气重复使用

建议判定为正常招式失败。消耗 PP，发布 `MoveFailed`，不刷新持续时间。

### 2. 天气伤害事件顺序

建议第一版固定 Side One 后 Side Two，但在同一个回合结束阶段处理双方候选。未来需要速度相关触发时再升级排序规则。

### 3. Persistent 是否现在加入类型

建议加入枚举但不创建实例。它是已知的天气特性需求，类型成本很低，也能避免后续把特殊值塞进剩余回合。

### 4. 是否让随机演示队伍立即携带天气

建议不立即加入。先用明确 fixture 验证规则。队伍配置接入后，再由配置选择天气招式。

### 5. 是否现在新增规则目录 crate

建议不新增。第一版四个 identifier 由组合根显式映射。出现第二个调用方或规则映射明显膨胀后，再抽 adapter。

### 6. 是否现在实现 EffectScheduler

不实现。天气使用明确的伤害输入和回合结束函数。满足多来源、同阶段触发条件后再设计调度器。
