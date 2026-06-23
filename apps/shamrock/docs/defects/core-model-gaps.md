# 核心模型与规则层缺陷

## 0. 种族值 / 个体值 / 努力值 / Nature / 最终能力值链路还不完整

现状：

- 当前数据层已经有 species stats
- `PokemonTemplate` 已支持可选 `individual_values` / `effort_values` / `nature` 和 `level`
- `battle-core` 已开始把它们解析成运行时 `BattleStats`
- 但还没有更完整的最终能力值链路。[defs.rs](../../crates/battle-data/src/defs.rs) [state.rs](../../crates/battle-core/src/state.rs) [lib.rs](../../crates/battle-mechanics/src/lib.rs)

问题：

- 现在只是第一版能力值链路
- species stats 的语义还需要继续收敛
- 还不是完整对战引擎应该有的数据模型

风险：

- 以后补更真实规则时，会同时侵入数据层、状态层和 mechanics 层

## 1. RNG 仍然只有单流

现状：

- `RngState` 目前只有一条线性随机流，提供 `roll_percent` 和 `roll_range_inclusive`。[lib.rs](../../crates/battle-core/src/lib.rs)

问题：

- 没有 RNG 隔离域
- 没有子流
- 没有局部随机覆写
- 没有快照 / 分支推演接口

风险：

- AI 多路径模拟会很难做
- 某些技能或规则想局部接管随机，会被迫侵入核心
- 不同世代规则的随机路径差异，很容易把 `step` 签名继续做大

## 2. 规则注入仍然直接绑定 `DataPack`

现状：

- 当前核心接口仍然是 `step(state, side, action, rng, data)`。[lib.rs](../../crates/battle-core/src/lib.rs)

问题：

- 规则包没有 trait 隔离层
- mechanics / format 的切换还不是核心接口的一等能力

风险：

- 多格式并行
- 动态切换规则
- 单元测试 mock 规则

这些场景后面都会逼着改大量函数签名。

## 3. BattleState 还没把静态 / 动态 / 延迟触发器彻底分开

现状：

- `PokemonTemplate` 和 `CombatPokemon` 已经分开，这是好的起点。[lib.rs](../../crates/battle-data/src/lib.rs) [lib.rs](../../crates/battle-core/src/lib.rs)
- 但 `BattleState` 仍然主要是当前回合和队伍状态的直接组合，没有独立的延迟触发器或调度容器。[lib.rs](../../crates/battle-core/src/lib.rs)

问题：

- 延迟结算和长期效果没有标准化容器
- 动态状态的分层还不够细

风险：

- 后续加持续效果、回合末触发器、房间类效果时，容易继续往 `BattleState` 塞字段

## 4. Slot 仍然没有完整契约

现状：

- 设计文档已经明确要抽象 slot，但代码层目前还是单打直推模型。[system-design.md](../architecture/system-design.md)

问题：

- 没有完整的位置寻址规则
- 没有双打目标选择契约
- 没有换位和站位关系模型

风险：

- 一旦做双打，目标选择和换人逻辑很可能要大改

## 5. Hook 方向有了，但执行模型还没有

现状：

- 文档定义了 hook 方向，但当前代码没有统一 hook 调度器。[system-design.md](../architecture/system-design.md)

问题：

- 缺执行顺序
- 缺权重层级
- 缺覆盖 / 互斥模型
- 缺可中断 / 可取消的统一拦截点

风险：

- 复杂机制最终会退化成一堆 intrinsic 特判

## 6. BattleOp 没事务能力

现状：

- `BattleOp` 仍然是直接应用到状态上的命令集合。[lib.rs](../../crates/battle-core/src/lib.rs)

问题：

- 没有预提交
- 没有事务
- 没有回滚
- 没有补偿层

风险：

- 以后碰到“半段有效、条件失败回退、反射/反弹”时，会快速变成补丁式逻辑

## 7. 声明式 / intrinsic 的边界还没有硬规范

现状：

- 文档表达了“声明式优先，复杂逻辑 intrinsic 兜底”。[system-design.md](../architecture/system-design.md)

问题：

- 哪些机制禁止写死，没有明确规则
- intrinsic 注册中心、白名单、版本隔离还没落地

风险：

- 会慢慢退化回面条式结算代码

## 8. `battle-format` 仍然偏薄

现状：

- 当前 `battle-format` 已经有 `FormatPhase` / `FormatContext` 入口，但主体能力仍然主要是合法动作枚举和包含判断。[lib.rs](../../crates/battle-format/src/lib.rs)

问题：

- 没有完整回合阶段规则
- 没有更强的队伍构建合法性入口
- 没有更复杂格式的统一约束模型

风险：

- 多格式继续做下去时，逻辑会回流进 `battle-core`

## 9. 速度同速判定仍然写死偏向 Player

现状：

- 当前同优先级、同速度时，顺序固定偏向 `SideId::Player`。[lib.rs](../../crates/battle-core/src/lib.rs)

问题：

- 这虽然保证了确定性，但它把“速度同速如何处理”写死成当前实现细节
- 既不是格式层规则，也不是可替换 mechanics 规则

风险：

- 后续如果需要更贴近真实规则、或不同格式想采用不同 tie-break 策略，就会继续侵入核心流程

## 10. 流程协调层仍然主要在 core

现状：

- 现在已经拆出 `battle-mechanics` 和 `battle-view`，但回合阶段调度仍主要留在 `battle-core`。[lib.rs](../../crates/battle-core/src/lib.rs) [lib.rs](../../crates/battle-mechanics/src/lib.rs)

问题：

- 内容层、规则层、格式层之间还缺一个更明确的流程协调抽象

风险：

- 格式增强时，容易继续侵入 `battle-core`

## 11. 依赖边界主要靠约定

现状：

- 现在已经有 7 个 crate，边界比之前好很多。[status.md](../current/status.md)

问题：

- 依赖方向仍然主要靠人为约束
- 没有更强的架构守卫

风险：

- 后续继续扩时，隐性循环依赖风险还在
