# Shamrock 日志与运行时设计

## 1. 目标

这份文档解决四个问题：

- 日志分几层，每层负责什么
- 回放存什么，怎么重放
- 核心怎么保持函数式，不受具体系统影响
- Rust 里怎么把这套东西做快，是否要用 arena

先给结论：

- 单场对战核心默认单线程
- 日志要分层，不要混成一种
- 回放以“输入重放”为权威，以“事件流和 checkpoint”做加速
- 核心保持纯函数语义，不直接依赖日志系统、网络系统、存储系统
- 不把 arena 当主内存模型，也不引入 GC

## 2. 单场运行模型

单场对战本质上是严格有序的状态机。

建议原则：

- 一场 battle 只在一个线程里推进
- 并行只发生在多场 battle 之间
- 所有日志写入当前 battle 的本地缓冲区
- 核心不加锁，不共享可变状态

原因很直接：

- 更容易保证确定性
- 更容易回放
- 更容易调试
- 更容易做属性测试和对拍

## 3. 日志分层

日志不要只有一种。至少拆成四层。

### 3.1 DomainEvent

这是对战语义事件。它是回放和协议的主数据。

例子：

- `TurnStarted`
- `MoveChosen`
- `MoveUsed`
- `MoveMissed`
- `DamageDealt`
- `StatusApplied`
- `PokemonSwitched`
- `FieldEffectStarted`
- `BattleEnded`

要求：

- 结构化
- 稳定
- 带版本
- 不拼展示文案

这层要尽量少改字段语义。它是长期契约。

### 3.2 TraceEvent

这是内部调试日志。它服务开发，不服务外部协议。

例子：

- `HookEnter`
- `HookExit`
- `RngRolled`
- `TargetResolved`
- `DamageFormulaEvaluated`
- `OpQueued`
- `OpApplied`
- `VisibilityProjected`

要求：

- 结构化
- 可以频繁调整
- 默认可以关闭

这层可以深埋在实现里。后面定位 bug 主要靠它。

### 3.3 ProtocolEvent

这是外部消费者看到的事件视图。它给 CLI、观战、客户端使用。

它通常从 `DomainEvent` 投影而来。必要时也允许补少量协议专用字段。

还要区分可见性：

- `Public`
- `Private(SideId)`
- `Internal`

这样以后接隐藏信息、观战和录像都不会乱。

### 3.4 MetricsEvent

这是性能和统计事件，不参与规则，也不参与回放。

例子：

- 本次 `step` 耗时
- 本回合触发 hook 数
- 本回合 RNG 次数
- 本回合 `BattleOp` 数量

这层只给 profiling 和性能回归看。

## 4. 推荐的数据流

日志和状态变化的主链路建议固定成这样：

```text
Input
-> Intent
-> Hook Dispatch
-> Effect Evaluation
-> BattleOp Queue
-> BattleOp Apply
-> DomainEvent
-> Protocol Projection
```

这里最关键的是中间三层：

- 效果逻辑先产出 `BattleOp`
- 核心只允许 `apply_op` 修改状态
- `apply_op` 顺手产出 `DomainEvent`

这样日志不会和状态变化脱节。

## 5. 回放设计

### 5.1 两种回放都要有

建议同时支持两种回放。

第一种是输入回放。它是权威记录。

需要保存：

- `engine_api_version`
- `data_pack_hash`
- `mechanics_pack_id`
- `format_pack_id`
- `battle_init`
- `seed`
- `input_frames`

优点：

- 体积小
- 最严格
- 最适合校验确定性

第二种是事件回放。它是展示加速层。

需要保存：

- `DomainEvent` 序列
- 可选的 `ProtocolEvent`
- 可选 checkpoint

优点：

- 播放快
- 适合 seek
- 不需要每次重跑所有规则

### 5.2 推荐混合方案

实际实现里建议两种一起做。

```rust
struct BattleRecord {
    manifest: ReplayManifest,
    init: BattleInit,
    seed: Seed,
    input_frames: Vec<InputFrame>,
    event_frames: Vec<EventFrame>,
    checkpoints: Vec<Checkpoint>,
}
```

说明：

- `input_frames` 是权威
- `event_frames` 用于快速展示
- `checkpoints` 用于跳转和断点恢复

### 5.3 ReplayManifest

回放文件必须带版本和依赖信息。

最低要求：

- `engine_api_version`
- `replay_schema_version`
- `data_pack_hash`
- `mechanics_pack_id`
- `format_pack_id`
- `build_metadata`

不做这层，后面文件会无法兼容。

### 5.4 Checkpoint

checkpoint 不需要每一步都存。建议按回合或固定帧间隔存。

checkpoint 里可以只放：

- `turn_index`
- `step_index`
- 压缩后的 `BattleState`
- 对应 RNG 状态

这样可以支持：

- 快速 seek
- 大回放文件分段加载
- 出错时定点重放

## 6. 日志如何深埋实现但不拖慢性能

这里不要用全局字符串 logger 做热路径记录。  
核心要直接记录 typed event。

### 6.1 不在热路径拼字符串

不要这样做：

- 在伤害计算里 `format!`
- 在 hook 调度里生成展示文案
- 在核心里直接调用全局 `tracing` 输出长文本

这样会增加分配和格式化成本，还会把展示逻辑和规则逻辑搅在一起。

正确做法：

- 核心只产出结构化事件
- 文本渲染放到外围
- 如果不开 trace，就不分配 trace 缓冲

### 6.2 Recorder 接口

建议在核心里用受控 recorder，而不是全局 logger。

```rust
trait Recorder {
    fn domain(&mut self, event: DomainEvent);
    fn trace(&mut self, event: TraceEvent);
    fn metric(&mut self, event: MetricsEvent);
}
```

至少准备两个实现：

- `NoopRecorder`
- `BufferRecorder`

这样可以把埋点写进核心函数：

```rust
rec.trace(TraceEvent::HookEnter { hook, source });
rec.domain(DomainEvent::MoveUsed { user, move_id, target });
```

不开日志时传 `NoopRecorder`。  
开调试时传 `BufferRecorder`。

### 6.3 对外保持纯函数接口

公共接口仍然保持纯函数风格：

```rust
fn step(
    state: BattleState,
    input: BattleInput,
    rng: RngState,
    data: &DataPack,
    mechanics: &MechanicsPack,
    format: &FormatPack,
) -> StepResult
```

内部可以暴露一个带 recorder 的版本：

```rust
fn step_with_recorder<R: Recorder>(
    state: BattleState,
    input: BattleInput,
    rng: RngState,
    data: &DataPack,
    mechanics: &MechanicsPack,
    format: &FormatPack,
    rec: &mut R,
) -> StepResult
```

这样做的结果是：

- 外部 API 仍然像纯函数
- 内部仍然可以高密度埋点
- 埋点系统不会反过来污染规则接口

## 7. 核心如何保持函数式，不受特定系统影响

这里说的“函数式”，重点不是追求语言级纯函数教条。  
重点是让核心具备纯函数语义。

也就是：

- 输入确定
- 输出确定
- 没有隐式外部依赖
- 没有隐藏副作用

### 7.1 核心禁止直接接触的系统

核心不要直接依赖这些东西：

- 文件系统
- 网络
- 系统时间
- 全局随机数
- 全局日志器
- 线程局部状态
- 数据库

这些都放在外围壳层。

### 7.2 显式传入的依赖

核心只接收显式参数：

- `BattleState`
- `BattleInput`
- `RngState`
- `DataPack`
- `MechanicsPack`
- `FormatPack`

需要的东西都走参数。  
这样同一组参数一定得到同一组结果。

### 7.3 用值和 ID，不用系统对象

核心层尽量只用这些东西：

- 结构体
- 枚举
- 新类型 ID
- 不可变引用

尽量不要把这些东西带进核心：

- socket
- 文件句柄
- 异步 runtime handle
- `Arc<Mutex<_>>`
- 任意外部 callback

一旦把这些带进去，规则就开始和系统耦合。

### 7.4 纯函数语义不等于完全不可变实现

Rust 里最实用的写法不是强行做 persistent immutable structure。  
更好的做法是：

- 对外接口表现为纯函数
- 内部拿到状态所有权后，局部原地修改
- 返回新的完整状态

例如：

```rust
fn step(...) -> StepResult {
    let mut next = state;
    // 局部修改 next
    StepResult { state: next, ... }
}
```

这在语义上仍然是纯的，因为：

- 调用者拿不到旧状态的可变引用
- 核心没有修改外部共享对象
- 所有变化都体现在返回值里

这种写法比 persistent immutable 结构更符合 Rust，也更快。

## 8. Rust 里怎么做高效实现

### 8.1 优先用拥有所有权的局部可变

这是最重要的一条。

不要为了“看起来函数式”频繁 clone 状态。  
既然 `step` 已经拿到 `BattleState` 的所有权，就应该在函数内部直接修改它。

收益：

- 避免大量中间分配
- 避免整棵状态树复制
- 保持外部纯函数语义

### 8.2 用密集存储和索引

对战实体数量有限，适合用密集结构。

推荐：

- `Vec<T>`
- `Box<[T]>`
- 小范围数组
- 新类型索引，比如 `PokemonIndex(u8)`

不推荐一开始就大量用：

- `HashMap`
- `Rc<RefCell<_>>`
- 图状指针结构

核心状态越像“密集表 + 索引”，缓存命中越好，序列化也越简单。

### 8.3 小对象用 SmallVec 或栈上数组

很多热路径集合其实很小：

- 当步 `BattleOp`
- 当步 `TraceEvent`
- 可选目标列表
- hook 命中列表

这些可以考虑：

- `SmallVec`
- `ArrayVec`

前提是先量，再上。  
没有 profiling，不要过早做微优化。

### 8.4 尽量用整数，不用浮点

对战规则更适合整数和定点规则。

原因：

- 行为更稳定
- 不容易出现平台差异
- 更利于确定性回放

### 8.5 静态分发优先

对频繁执行的核心路径，优先：

- `enum + match`
- 泛型
- 编译期已知的函数表

少用运行时多态和深层 trait object 链。

这样更容易内联，也更容易看清热点。

## 9. Arena 和 GC 的取舍

先说结论：

- 不要引入 GC
- 不要把 arena 当作核心状态的主模型
- arena 只在局部场景下考虑

### 9.1 为什么不需要 GC

Rust 已经用所有权解决了大部分生命周期问题。  
你的核心状态机又是单线程、确定性的，没必要再加 GC。

引入 GC 会带来这些问题：

- 生命周期语义变复杂
- 暂停和回收时机难以分析
- 序列化和回放更绕
- 和 Rust 主流生态不一致

### 9.2 为什么不建议 arena 承载整个 BattleState

arena 适合“成批创建，整批释放”的对象。  
但 battle state 不是纯追加结构，它会频繁更新：

- HP 变化
- 状态变化
- 天气覆盖
- 临时效果增加和移除
- 出场位切换

如果把这些都丢进长寿命 arena，会遇到几个问题：

- 旧对象很难清理
- 容易产生悬空 ID 语义问题
- snapshot 和差分比较变复杂
- 状态结构不够直观

### 9.3 arena 适合放哪

arena 在这几类地方可以考虑：

- 每个 `step` 内的临时 scratch 分配
- 数据包加载阶段的中间对象
- DSL 解析后的只读表达式树
- 调试 trace 的短命批量对象

这种场景的共同点是：

- 生命周期短
- 不需要单独释放
- 不构成权威状态

### 9.4 更适合的主模型

BattleState 建议这样建：

- 固定大小或小规模的数组和 `Vec`
- 用索引连接实体
- 用枚举表示状态机分支
- 用受控的 `Vec<BattleOp>` 和 `Vec<Event>` 记录当步变化

如果以后真的出现大量动态实体，再考虑：

- `slotmap`
- generational index
- slab

但第一版没必要先上这些。

## 10. 当前建议的工程约束

日志和运行时这块，建议先锁这些约束：

- 单场 battle 单线程推进
- 回放以输入重放为权威
- 核心只产出 typed event，不拼展示文本
- `DomainEvent` 和 `TraceEvent` 分开
- 核心不依赖文件、网络、时钟、全局日志器
- 对外接口保持纯函数语义
- 内部允许拿到所有权后的局部可变
- 不引入 GC
- 不把 arena 用作核心状态容器

如果后面性能真有问题，先做 profiling。  
不要在没有数据的情况下先上 arena、复杂池化和高度抽象的日志框架。
