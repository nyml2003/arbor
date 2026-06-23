# Shamrock 对战系统设计

## 1. 目标

Shamrock 是一个对战模拟引擎。当前主线是继续扩格式、AI 和更强外壳，同时维持已经落地的 core / mechanics / view / replay 边界。

当前目标：

- 保持 Rust 纯函数式结算内核
- 继续把宝可梦、招式、天气、状态等内容放到数据表
- 在当前 7-crate 结构上继续演进，优先守住已经抽出的 `battle-mechanics` / `battle-view` 边界
- 保证同一输入序列和同一随机种子一定得到同一结果
- 把 replay 维持在可导出、可重放、可恢复、可回归校验的状态

当前不做：

- 直接兼容全部官方代际规则
- 运行时加载任意 Rust 动态库
- 一开始就支持所有复杂技能和双打规则

## 2. 核心原则

### 2.1 纯函数内核

当前核心推进接口保持纯函数形态：

```rust
fn step(
    state: BattleState,
    side: SideId,
    action: BattleAction,
    rng: RngState,
    data: &DataPack,
) -> StepResult
```

约束：

- 不读全局状态
- 不做 IO
- 不直接访问系统时间
- 随机数状态显式传入和传出
- 所有状态变化都通过统一操作管线完成

### 2.2 数据优先，代码兜底

优先使用数据表定义行为。少量复杂机制允许落到内建 handler。

设计目标不是追求 `100% 纯表驱动`，而是保证：

- 大部分常规效果可声明式表达
- 少量特例通过受控 intrinsic 扩展
- 所有效果仍走同一套 hook 和 `BattleOp` 管线

### 2.3 公开视图和权威状态分离

引擎内部维护完整权威状态。玩家、观战者、AI 看到的是投影后的视图。

这样做有三个作用：

- 支持隐藏信息
- 支持观战和回放
- 支持后续网络协议和断线重连

### 2.4 内容和规则分离

`Gen1` 只是当前内容范围，不应该写死在核心内核里。

要分开四层：

- 内容层：宝可梦、招式、属性、天气、状态等静态定义
- 规则层：命中、伤害、状态结算、速度判定等纯规则计算
- 格式层：单打、双打、队伍人数、禁限和选出规则
- 协议与外壳层：玩家输入、事件日志、公开视图、CLI/TUI、回放

## 3. 模块划分

### 3.1 当前真实 crate 边界

当前 workspace 有 7 个 crate：

- `battle-core`
  - 状态机推进
  - 回合排序
  - 招式效果解析
  - `BattleOp` 应用
  - 权威日志
- `battle-data`
  - ID 类型
  - 数据表 schema
  - 静态数据与 demo 数据包
- `battle-format`
  - 合法动作枚举
  - 格式层输入约束
- `battle-mechanics`
  - 纯规则计算
  - 速度、优先级、伤害、天气倍率、持续伤害
- `battle-replay`
  - 输入帧、事件帧、metrics、checkpoint
  - replay JSON 导入导出
  - replay 重放、校验和 checkpoint 恢复
- `battle-view`
  - `PublicBattleView`
  - `UiEventLog`
  - 公开界面投影
- `battle-cli`
  - plain CLI
  - TUI
  - demo AI 和运行壳层

### 3.2 当前剩余主线

当前剩余主线是更完整格式、AI 和更强外壳。

## 4. 核心运行时模型

### 4.1 BattleState

`BattleState` 是权威状态。它至少包含：

- 对局元数据：回合数、阶段、胜负状态
- 随机种子或外部传入的 RNG 状态引用
- 双方队伍、出场位、后排信息
- 当前 HP、能力阶段、异常状态、临时状态
- 场地效果、天气、房间类效果、边侧效果
- 已公开信息和未公开信息
- 等待中的触发器或延迟结算对象

建议状态结构不要写死成“单打只有 1 个位置”。  
即便第一版只做单打，也应该把“战斗位置”抽象成 slot。

### 4.2 输入与输出

输入分两类：

- 外部输入：玩家选择招式、换人、预览选择
- 系统输入：继续推进队列、处理强制换人、开始回合

输出至少包含：

- 新状态
- 新 RNG 状态
- 结构化事件日志
- 下一步请求
- 可选的胜负结果

当前合法动作接口：

```rust
fn legal_actions(
    state: &BattleState,
    side: SideId,
) -> Vec<BattleAction>
```

`step` 负责推进，`legal_actions` 负责列出当前允许的选择。  
后续如果格式层继续增强，也应优先扩这两个边界，而不是让 CLI 或 AI 自己推导规则。

## 5. 效果系统

### 5.1 统一 hook

所有招式、天气、状态、道具、特性都通过统一触发点接入：

```rust
enum Hook {
    OnBattleStart,
    OnSwitchIn,
    OnTurnStart,
    BeforeChoiceLock,
    BeforeMove,
    OnTryHit,
    OnDamageCalc,
    AfterDamage,
    AfterMove,
    OnFaint,
    EndTurn,
}
```

第一版不需要把 hook 一次性列完，但必须先定“统一触发入口”这个方向。

### 5.2 统一操作

效果逻辑不要直接修改状态。  
效果先产生操作，再由核心统一应用。

```rust
enum BattleOp {
    Damage { target: EntityId, amount: u16 },
    Heal { target: EntityId, amount: u16 },
    ApplyStatus { target: EntityId, status: StatusId },
    CureStatus { target: EntityId },
    ModifyStatStage { target: EntityId, stat: Stat, delta: i8 },
    AddVolatile { target: EntityId, effect: VolatileId },
    RemoveVolatile { target: EntityId, effect: VolatileId },
    SetFieldEffect { effect: FieldEffectId, duration: u8 },
    ClearFieldEffect { effect: FieldEffectId },
    ForceSwitch { side: SideId },
    Faint { target: EntityId },
}
```

好处：

- 日志来源统一
- 回放更稳定
- 特例不会散到状态各处
- 属性测试更容易写

### 5.3 声明式效果和 intrinsic

效果定义分两层：

- 声明式效果：由数据表描述，覆盖常规技能和状态
- intrinsic 效果：由受控 Rust handler 实现，处理复杂特例

一个实际可行的策略是：

- 第一版先做内建 handler，接口和注册表先定好
- 第二版把高频通用效果抽成 DSL
- 第三版只保留少数 intrinsic 特例

这样迭代成本最低。

## 6. 插件和可插拔边界

这里的“可插拔”不是任意代码热插拔。  
当前建议的可插拔范围是三类包和一类注册表。

### 6.1 DataPack

负责承载：

- 宝可梦
- 招式
- 类型表
- 状态
- 天气
- 临时效果
- 学习表

要求：

- 数据定义可序列化
- 带 schema version
- 带 pack id 和依赖关系

### 6.2 MechanicsPack

负责承载：

- 伤害公式
- 命中和暴击规则
- 速度和优先级规则
- 状态结算规则
- intrinsic 注册表

要求：

- 所有实现保持纯函数
- 可以声明所需能力，比如 `weather`、`abilities`、`double_battle`
- 不直接依赖 UI 或网络层

### 6.3 FormatPack

负责承载：

- 对战人数和出场位配置
- 单打/双打等格式定义
- 队伍合法性校验
- 选出和禁限规则
- 玩家公开信息规则

### 6.4 版本和兼容性

每个包都应声明：

- `pack_id`
- `engine_api_version`
- `schema_version`
- `dependencies`
- `capabilities`

如果不提前做版本元数据，后面一旦引入多个内容包和机制包，兼容性会立刻失控。

## 7. 天气、状态和技能的扩展策略

天气系统不要写死成一个小枚举。  
更合理的模型是“场地上的长期效果实例”。

建议：

- 天气在数据表里定义为一种 `FieldEffectDef`
- 进入场地后注册一组 hook 绑定
- 持续回合、覆盖关系、结算时机由机制包解释

同理：

- 招式是 `ActionDef + EffectRef`
- 状态是 `StatusDef + HookBinding`
- 特性和道具未来也复用同一入口

这样以后加新天气，不需要改核心状态机，只需要新增定义和少量规则解释。

## 8. 日志、回放和测试

### 8.1 结构化日志

日志至少分三层：

- 内部事件：hook 触发、操作生成、操作应用
- 对外事件：玩家可见的对战事件
- 文本渲染：给 CLI 或 UI 展示

不要只保留文本日志。  
文本日志只适合展示，不适合测试和回放。

### 8.2 回放

回放至少支持两种模式：

- 事件重放：用事件日志重建展示
- 输入重放：用初始状态、输入序列和 seed 重跑引擎

输入重放更严格。  
如果同一 seed 和同一输入不能复现同一结果，说明内核已经丢掉确定性。

### 8.3 测试

建议固定三类测试：

- 金样测试：已知对局案例，断言事件流和终局
- 属性测试：HP 不为负，终局后不能再推进，同 seed 同结果
- 差分测试：和参考实现或历史基线对拍

## 9. 当前阶段建议的取舍

### 9.1 现在先做什么

当前阶段先做这些：

- 更完整的格式层
- 可替换 AI 策略
- 更强的观战 / 脚本 / 外壳能力
- 在保持当前 7-crate 结构下继续扩展

### 9.2 先不做什么

先不要做：

- 运行时执行外部 Rust 插件
- 全量 Gen1 招式
- 双打
- 全量隐藏信息协议
- 复杂 AI

### 9.3 必须继续守住的口子

即便当前还没完全展开，也要继续守住：

- `ItemSlot`
- `AbilitySlot`
- `FieldEffect`
- `SideEffect`
- 多个 battle slot
- 公开视图投影

这些口子后补，代价会比现在高很多。

## 10. 当前文档要锁定的工程决策

当前先把下面几条当作项目约束：

- 核心结算接口使用纯函数风格
- 核心状态只允许通过 `BattleOp` 改动
- 内容、规则、格式三层必须分开
- 第一版允许 intrinsic handler，但禁止到处散落特判
- 每个对局都必须可重放
- 数据表和回放格式都要带版本号
- 当前未落地的边界必须标注为 planned，而不是 existing

如果后面有更具体的规则需求，优先补到当前的规则边界和格式边界，不要直接改坏核心契约。
