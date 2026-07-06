---
id: TEP-0009
title: "统一组件协议与 Signal 优先状态管理"
status: Draft
created: 2026-07-06
updated: 2026-07-06
author: nyml
area: architecture
---

# TEP-0009: 统一组件协议与 Signal 优先状态管理

## 本质

arbor-tui 组件协议改成 Signal 优先的数据流：

```text
Signals + Props -> mount/update retained RuntimeState -> measure -> render Fragment -> event -> DirtyKind -> unmount
```

组件作者只学习这一条流程。每个组件不再各自定义一套 props、state、signal、measure、render 和事件处理方式。

本 TEP 接在 TEP-0008 后面。TEP-0008 解决 retained tree 和 cache invalidation 的前置问题。本 TEP 规定 retained component 如何接收 Props、订阅 Signal、维护最小 RuntimeState、使用 memo/useMemo/computed，并把性能不劣化作为合入门槛。

核心原则：凡是会影响 UI、需要被观察、需要跨组件共享、需要测试或需要回放的状态，优先建模为 Signal。组件内部 RuntimeState 只保存生命周期资源、memo store 和无法自然外置的局部瞬态。

## 背景

当前组件协议有三个问题：

1. facade 每帧 rebuild widget tree。widget 内部缓存很难稳定复用。
2. Props、builder、widget instance、Signal 和 layout/render 状态混在一起。组件作者需要逐个学习实现细节。
3. DirtyTracker 只能粗略标记脏组件。它不能可靠判断 props 更新、signal 更新、state 更新分别影响 render、layout 还是 structure。

因此，继续在旧协议上叠加局部优化会变成补丁。正确方向是先统一组件生命周期和数据边界，再让 TEP-0008 的 layout cache、render cache 和 dirty rect 有稳定落点。

## 目标

1. 明确组件输入 Props 的形状。
2. 明确组件输出的渲染结构。
3. 明确 Signal 优先的状态归属。
4. 明确 Signal 如何进入组件，如何触发 dirty。
5. 明确 mount、update、unmount、measure、render、event 的调用顺序。
6. 明确 memo、useMemo、computed 的职责边界。
7. 任何默认路径改动都不能让 aster 标准基准劣化。

## 非目标

1. 不在本 TEP 中直接实现完整局部 render。
2. 不要求一次性迁移全部组件。
3. 不引入自动依赖追踪。Signal 仍采用显式依赖声明。
4. 不用深比较解决 props 变化判断。
5. 不把业务状态搬进组件 RuntimeState。
6. 不鼓励组件用普通字段保存可观察 UI 状态。

## 统一数据流

运行时数据流：

```text
app signals / computed signals
  -> view()
  -> NodeSpec tree
  -> reconcile retained ComponentInstance tree
  -> update props/signal subscriptions/runtime resources
  -> measure/layout
  -> render fragments
  -> compose VirtualScreen
  -> diff
  -> emit ANSI
```

类型职责：

| 概念 | 职责 | 生命周期 |
| --- | --- | --- |
| `Props` | 外部输入描述 | 每次 view 可重新创建 |
| `NodeSpec` | view 输出的声明式树 | 可丢弃 |
| `ComponentInstance` | runtime 保留实例 | key/type 相同时跨帧复用 |
| `Signal` | UI 状态和业务派生状态的主载体 | app/view-model 或组件实例拥有 |
| `RuntimeState` | 组件生命周期资源、memo store、本地 Signal handle | mount 创建，unmount 销毁 |
| `Fragment` | render 输出片段 | 单帧产物，可被 cache |

`NodeSpec` 不是 rendered screen。`ComponentInstance` 不是 Props。`RuntimeState` 不是业务数据仓库，也不是第二套状态管理。

## Props

Props 是组件的唯一外部输入。

Props 可以包含：

1. 普通值，例如文本、样式、padding、选项列表。
2. `ReadSignal<T>`，表示外部响应式输入。
3. callback 或 action mapper，表示组件事件如何回到 app。
4. children 的 `NodeSpec`。

Props 不允许包含：

1. runtime id。
2. layout/render cache。
3. 已渲染的 `VirtualScreen`。
4. 组件内部 RuntimeState。
5. 需要热路径深比较的大对象。

Props 必须能提供廉价 revision：

```rust
trait ComponentProps {
    fn revision(&self) -> u64;
    fn signal_deps(&self) -> &[SignalDep];
}
```

revision 规则：

1. revision 只表示会影响 UI 的 props 内容版本。
2. callback 不参与 revision。
3. 大文本、大列表、children 不在热路径深比较。
4. 大对象 revision 由构造方或数据源生成。
5. Signal 值变化不直接改变 props revision。Signal 使用自己的 generation。

例子：

```text
TextProps {
  text: TextSource,
  style: TextStyle,
  wrap: WrapMode,
  revision: u64,
}

InputProps {
  value: Option<ReadSignal<String>>,
  placeholder: String,
  on_change: ActionMapper,
  revision: u64,
}

ScrollViewProps {
  axis: Axis,
  viewport_policy: ViewportPolicy,
  child: NodeSpec,
  revision: u64,
}
```

## 输出结构

组件输出分两种：

1. 组合组件输出 `NodeSpec` children。
2. leaf 组件输出 `Fragment`。

用户 `view()` 只产生 `NodeSpec tree`：

```text
view() -> NodeSpec
```

runtime 负责：

1. 根据 `key + component type` reconcile。
2. 保留或替换 `ComponentInstance`。
3. 调用 lifecycle。
4. 持有 layout/render cache。
5. 合成 `VirtualScreen`。

组件不能直接控制全局 diff。组件只能返回自己的尺寸、渲染片段和 dirty hint。

## 状态归属

arbor-tui 采用 Signal 优先的状态模型。

状态分三类：

| 类型 | 归属 | 例子 |
| --- | --- | --- |
| App Signal | app/view-model | 聊天消息、当前模型、theme、命令面板列表 |
| Component-owned Signal | 组件实例 | 非受控 input value、cursor、scroll offset、selected row、tabs active index |
| RuntimeState | 组件实例 | 订阅句柄、memo store、本地 Signal handle、短生命周期运行时资源 |

优先级：

```text
App Signal > Component-owned Signal > RuntimeState field
```

RuntimeState 只保存不能合理表达为 Signal 的运行时资源。只要状态会影响 UI，默认先考虑 Signal。

Signal 保存：

1. 输入框 value。
2. 输入框 cursor 和 selection。
3. ScrollView offset。
4. List 当前选中项。
5. Tabs active index。
6. loading phase。
7. Transcript 消息列表。
8. 当前模型。
9. theme 全局配置。

RuntimeState 可以保存：

1. Signal subscription token。
2. memo store。
3. 组件本地 Signal handle。
4. 上一次 layout/render revision。
5. 临时事件聚合状态。

RuntimeState 不保存：

1. 可观察业务状态。
2. 可回放 UI 状态。
3. 跨组件共享状态。
4. 可以用 Signal 表达的交互状态。

创建和销毁规则：

```text
mount    -> create RuntimeState and component-owned Signals
update   -> keep RuntimeState, update Signal subscriptions, apply controlled props
unmount  -> unsubscribe and drop RuntimeState and component-owned Signals
replace  -> unmount old RuntimeState, mount new RuntimeState
```

复用条件：

```text
same key
same component type
compatible state model
```

替换条件：

```text
key changed
type changed
state model incompatible
duplicate sibling key in debug/test
```

## 本地 Signal 生命周期

Component-owned Signal 是 RuntimeState 的一部分。

复用规则：

1. `key + component type + state model` 都相同时，RuntimeState 复用，本地 Signal 也完整复用。
2. unmount 时，runtime 先退订 Signal，再销毁 RuntimeState，最后释放组件本地 Signal。
3. key 改变、type 改变或 state model 不兼容时，旧本地 Signal 必须销毁，新实例重新创建本地 Signal。
4. unmount 后的本地 Signal 不能再产生 dirty。debug/test 下发现已销毁 Signal 被写入，必须报错。

controlled/uncontrolled 是 state model 的一部分：

| 变化 | 处理 |
| --- | --- |
| uncontrolled -> uncontrolled | 复用本地 Signal |
| controlled -> controlled | 复用订阅，更新外部 SignalDep |
| uncontrolled -> controlled | state model 不兼容，替换实例并销毁本地 Signal |
| controlled -> uncontrolled | state model 不兼容，替换实例并创建本地 Signal |

如果 app 需要重置 uncontrolled 组件状态，必须改变 key，或传入明确的 reset revision。不能依赖 props 普通变化隐式清空本地 Signal。

## 生命周期

统一生命周期：

```rust
mount(props, ctx) -> RuntimeState
update(runtime, old_props, new_props, ctx) -> DirtyKind
measure(runtime, props, constraints, ctx) -> Size
render(runtime, props, layout, ctx) -> Fragment
event(runtime, props, event, ctx) -> EventResult<Action>
unmount(runtime, props, ctx)
```

调用顺序：

1. mount：父组件先 mount，子组件后 mount。
2. update：父组件先 update，再 reconcile children。
3. measure：父组件给约束，子组件返回自然尺寸或约束结果。
4. layout：父组件分配 rect。
5. render：按 layout rect 输出 fragment。
6. event：焦点组件先处理，未处理则冒泡到父组件。
7. unmount：子组件先 unmount，父组件后 unmount。

约束：

1. `mount` 可以订阅 Signal，不能 emit ANSI。
2. `update` 可以更新 RuntimeState 和组件本地 Signal，必须返回 DirtyKind。
3. `measure` 不允许产生 Action。
4. `render` 不允许订阅 Signal，不允许修改业务状态。
5. `event` 优先写 Signal，可以更新 RuntimeState，可以返回 Action。
6. `unmount` 必须退订 Signal，释放 memo store。

## Signal

Signal 是状态管理的主路径。它既可以由 app/view-model 拥有，也可以由组件实例拥有。

Signal 进入组件的方式：

```text
Props contains ReadSignal<T>
Props declares SignalDep
runtime subscribes on mount/update
Signal::set increments generation
runtime marks subscribed components dirty
```

组件内部需要非受控状态时，也使用 component-owned Signal：

```text
mount uncontrolled component
  -> create local WriteSignal<T>
  -> expose ReadSignal<T> to component logic
event
  -> local WriteSignal::set(...)
  -> runtime receives normal generation change
```

controlled 与 uncontrolled 规则：

| 模式 | 状态来源 | 事件结果 |
| --- | --- | --- |
| controlled | Props 中的外部 `ReadSignal<T>` | emit Action，由 app 写外部 Signal |
| uncontrolled | 组件实例创建本地 Signal | event 直接写本地 Signal，可选 emit Action |

同一个状态不能同时由外部 Signal 和本地 Signal 写入。controlled props 出现时，以外部 Signal 为唯一真源。

Signal 需要稳定身份：

```rust
struct SignalId(...);

trait SignalSource {
    fn id(&self) -> SignalId;
    fn generation(&self) -> u64;
}
```

Signal dirty 规则：

| Signal 用途 | DirtyKind |
| --- | --- |
| 只改变颜色、光标、选中态 | Render |
| 可能改变文本宽高 | Layout |
| 改变 children 数量或可见结构 | Structure |
| 改变 theme | Theme |

组件声明 signal dependency：

```text
SignalDep {
  signal_id,
  generation,
  dirty_kind,
}
```

运行时根据 `SignalId` 管理订阅。组件 unmount 后必须退订，避免已销毁组件继续被 dirty。

## Signal 调度与批处理

Signal 写入不直接重入渲染。runtime 使用批处理队列。

写入流程：

```text
Signal::set
  -> generation += 1
  -> enqueue SignalId into pending_signals
frame boundary
  -> deduplicate SignalId
  -> lookup subscribers
  -> merge DirtyKind per component
  -> update DirtyTracker once
```

合并规则：

1. 同一 Signal 在一帧内多次 set，只保留最后值，generation 至少增加一次。
2. 同一组件被多个 Signal dirty 时，取最高 DirtyKind。
3. 同一帧内的 dirty 写入只触发一次 update/measure/render 调度。
4. Signal 写入发生在 event/action dispatch 中时，进入当前 frame 的 pending 队列；如果 snapshot 已创建，则进入下一帧。

大 fanout 兜底：

1. 单个 Signal 订阅者超过阈值时，runtime 可以把多个组件 dirty 合并到最近公共 retained ancestor。
2. dirty 组件数超过当前 mounted node 数量的固定比例时，runtime 可以提升为 root 对应等级 dirty。
3. 合并只能扩大重绘范围，不能降低 DirtyKind。
4. threshold 必须可配置，并在 bench 日志中输出命中次数。

## 事件与 Action

事件从焦点组件开始：

```text
focused component
  -> event()
  -> Handled(dirty, action?)
  -> or Bubble
  -> parent event()
```

事件结果：

```text
Handled {
  dirty: DirtyKind,
  action: Option<Action>,
}

Bubble

Ignored
```

规则：

1. `Handled` 停止冒泡。
2. `Bubble` 继续交给父组件。
3. `Ignored` 到根后丢弃。
4. 组件只通过 Action 或 callback 把意图交给 app。
5. 组件不直接修改 app state。
6. controlled 组件只 emit Action，不写外部 Signal。
7. uncontrolled 组件可以写组件本地 Signal。

例子：

| 事件 | 状态变化 | DirtyKind |
| --- | --- | --- |
| 输入框左右移动 | cursor Signal 变化 | Render |
| 输入框输入字符 | value/cursor Signal 变化 | Layout |
| ScrollView 下滚 | offset Signal 变化 | Render |
| Tabs 切换 active child | active index Signal 变化 | Structure |
| Button Enter | 无或 pressed frame | Render |

## Action 调度层

组件不直接修改 app state。所有外部状态写入都经过 Action 调度层。

接口语义：

```rust
trait ActionDispatcher<Action> {
    fn dispatch(&mut self, action: Action, ctx: &mut DispatchCtx) -> DispatchResult;
}
```

调度顺序：

```text
event()
  -> EventResult::Handled { action }
  -> ActionDispatcher::dispatch
  -> app writes Signals
  -> pending signals drained before next FrameSnapshot
```

规则：

1. controlled 组件只 emit Action。
2. app 在 dispatcher 中写外部 Signal。
3. dispatcher 可以合并多个 Action。
4. dispatcher 不允许直接调用 render/layout。
5. dispatch 失败必须返回结构化结果，由 app 决定是否显示错误状态。

## measure

measure 只回答尺寸问题：

```text
(props, runtime, constraints, child measurements) -> Size
```

measure 可以读取：

1. Props。
2. Signal snapshot。
3. RuntimeState。
4. constraints。
5. child measurement。
6. theme metrics。
7. useMemo 缓存。

measure 不允许：

1. emit Action。
2. 订阅 Signal。
3. 修改业务状态。
4. 访问终端 IO。

Signal 快照时机：

```text
drain input/events
dispatch actions
drain pending signals
create FrameSnapshot
update
measure
layout
render
diff
```

规则：

1. `FrameSnapshot` 在每帧 update 前创建。
2. update、measure、layout、render 读取同一个 `FrameSnapshot`。
3. measure 不允许实时读取 Signal 当前值，只能读取 snapshot。
4. render 不允许实时读取 Signal 当前值，只能读取 snapshot。
5. snapshot 创建后发生的 Signal 写入进入下一帧。

这样可以避免多帧连续 Signal 更新时，measure 和 render 读到不同状态。

measure 的 cache key：

```text
props revision
signal deps generations that affect layout
runtime layout revision
constraints
theme metrics revision
children measure revision
```

## render

render 只回答“这个组件在这个 rect 中画什么”：

```text
(props, runtime, layout rect, theme, focus state) -> Fragment
```

render 可以读取：

1. Props。
2. Signal snapshot。
3. RuntimeState。
4. layout rect。
5. theme。
6. focus state。
7. useMemo 缓存。

render 不允许：

1. 订阅 Signal。
2. 修改 app state。
3. 触发 layout。
4. 访问真实终端。

render 的 cache key：

```text
props revision
signal deps generations that affect render
runtime render revision
rect
theme revision
focus state
```

## memo

memo 是组件级跳过机制，由 runtime 执行。

命中条件：

```text
props revision unchanged
signal dependency generations unchanged
runtime revision for this stage unchanged
constraints unchanged for measure
rect/theme/focus unchanged for render
dirty kind lower than current stage requirement
```

memo 命中时：

1. update 可跳过。
2. measure 可复用 layout cache。
3. render 可复用 render cache。
4. diff 可限制到 dirty rect。

组件作者不手写 memo 判定。组件只提供 revision、signal deps 和 dirty hint。

## useMemo

useMemo 是组件实例内缓存。它不是状态管理工具。

用途：

1. Markdown parse result。
2. RichText wrapped lines。
3. Transcript message line count。
4. Text measured width。
5. ScrollView visible range。

接口语义：

```text
useMemo(slot, deps: &[u64], compute)
```

规则：

1. deps 只能放小值，例如 revision、generation、width、theme_rev。
2. deps 不做大对象深比较。
3. key/type 相同复用实例时保留 memo store。
4. unmount 时释放 memo store。
5. type 替换时清空 memo store。

useMemo 适合 UI 局部纯计算。它不适合保存可观察状态，也不适合跨组件共享业务派生数据。

## useMemo 容量与淘汰

useMemo 必须有容量策略。缓存不能无限随组件实例增长。

默认策略：

1. 每个组件实例默认最多 16 个 memo entry。
2. 每个组件实例默认 memo 预算为 256 KiB。
3. runtime 全局 memo 预算默认 2 MiB。
4. 超出预算时按 LRU 淘汰。
5. 固定小缓存可以显式声明 `MemoPolicy::Pinned`，但必须有 slot 数量上限。

长列表规则：

1. Transcript、Markdown message、table row 这类大量重复组件，不能把大对象缓存放进每个 row 的无界 useMemo。
2. 大对象缓存必须进入有容量上限的共享 cache，key 使用 message id、body revision、theme revision。
3. row unmount 时必须释放 row 私有 memo entry。
4. debug/bench 日志要输出 memo entry 数、估算字节数、淘汰次数。

## computed

computed 是 Signal 图里的派生节点，优先放在 app/view-model 层。

用途：

1. 从全局状态派生当前可见消息列表。
2. 从模型配置派生 UI label。
3. 从多个 Signal 派生 command palette items。
4. 多组件共享同一个派生结果。

computed 行为：

```text
deps generation changed
  -> computed marked stale
first get()
  -> recompute
  -> computed generation increments if output changed
component reads computed as ReadSignal
  -> runtime receives normal SignalDep
```

computed 不用于：

1. 依赖 layout width 的换行缓存。
2. 依赖 viewport 的 visible range。
3. 依赖 focus state 的样式。
4. 单组件私有 render cache。
5. 事件处理中临时聚合的运行时资源。

判断规则：

```text
可观察状态 -> Signal
业务派生、跨组件共享 -> computed Signal
组件内部、依赖尺寸或 focus 的纯计算 -> useMemo
整组件阶段跳过 -> memo
```

混合派生规则：

如果派生数据同时依赖全局 Signal 和组件布局尺寸，必须拆成两层：

```text
global signals
  -> computed: layout-free derived data
  -> component useMemo: layout projection with deps [computed generation, width, theme_rev]
```

禁止把 layout width、viewport rect、focus state 放进 app/view-model computed。它们属于组件局部投影，应该由 useMemo 处理。

例子：

```text
messages Signal
  -> computed visible_messages_without_wrap
  -> useMemo(width, theme_rev, visible_messages_generation): wrapped_lines
```

## DirtyKind

DirtyKind 继续使用 TEP-0008 的分级：

```text
Full > Theme > Structure > Layout > Render
```

本 TEP 对来源做补充：

| 来源 | DirtyKind |
| --- | --- |
| props 样式变化 | Render |
| props 文本长度变化且影响 wrap/height | Layout |
| props children 变化 | Structure |
| cursor Signal 变化 | Render |
| scroll offset Signal 变化 | Render |
| selected row Signal 改变且只影响高亮 | Render |
| signal generation 变化 | 由 SignalDep 声明 |
| theme revision 变化 | Theme |
| terminal resize | Full |

update 必须返回当前组件自己的 dirty。runtime 负责向祖先传播 layout/structure 影响。

## Structure 与 Resize 失效范围

Structure dirty 不等于每次都全树重建。

Structure 规则：

1. 子节点增删、key/type 替换、Tabs active child 改变，标记最近 structure root。
2. runtime 只 reconcile 受影响子树，并把 layout 影响向祖先传播。
3. focus map 优先做子树重建和全局 focus order splice。
4. 只有 root structure dirty、focus order 无法局部修复、或 debug 校验失败时，才重建整棵 focus map。
5. 如果局部修复成本超过全量成本阈值，runtime 可以退化为全量重建，并在 bench 日志记录 fallback。

resize 分级：

| resize 来源 | DirtyKind | 范围 |
| --- | --- | --- |
| 终端尺寸变化 | Full | root |
| 父容器 layout 分配变化 | Layout | 受影响子树 + old/new rect |
| ScrollView viewport 变化 | Layout 或 Render | viewport 子树 |
| leaf rect 变化但内容尺寸不变 | Render | leaf old/new rect |

局部容器 resize 不能直接提升为 Full。只有终端全局 resize 或 backend reset 才使用 Full。

## 性能约束

本 TEP 不承诺第一阶段立刻变快。第一阶段目标是协议清晰和缓存边界正确。

默认路径必须不劣化。

硬规则：

1. 热路径只比较 `u64 revision/generation`。
2. 不做 props 深比较。
3. callback 不参与 revision。
4. Signal dependency 显式声明。
5. memo/useMemo/computed 都使用显式 deps。
6. layout/render cache 必须先 shadow mode 验证。
7. old rect 和 new rect 都必须进入 dirty rect。
8. ScrollView 不渲染完整 `content_h` 大屏。

性能门槛：

```text
P95 <= baseline P95 * 1.05
P99 <= baseline P99 * 1.05
C frames == 0
streaming scene 不稳定变慢
scroll scene 不稳定变慢
palette scene 不稳定变慢
```

任一阶段出现稳定负向结果：

1. 默认路径回滚。
2. 保留能证明问题的测试。
3. 如有价值，可保留 shadow mode。
4. 优化报告必须记录负向原因和回滚点。

## Shadow 与 Bench 自动化

shadow mode 必须由脚本自动验证，不能依赖人工读日志。

新增自动化入口：

```bash
cargo run -p aster-tui --features bench-log -- --bench --bench-cache-shadow --bench-compare <baseline.jsonl>
```

要求：

1. 自动运行标准路径。
2. 自动输出优化后 jsonl。
3. 自动解析 baseline 和 current。
4. 自动比较 P95、P99、C frames、分 scene 均值。
5. 自动检查 cache shadow mismatch。
6. 自动生成 markdown 优化报告。
7. 任一硬门槛失败时，命令返回非零退出码。

报告路径：

```text
apps/tui-framework/docs/perf-optimization-report-<topic>-<date>.md
```

## 实施阶段

### 阶段 1：文档和测试骨架

新增本 TEP。把 TEP-0005 标记为旧协议说明，不作为终态依据。

验收：

1. TEP-0009 说明 Props、Signal-first state、RuntimeState、lifecycle、memo/useMemo/computed。
2. 测试清单覆盖 mount/update/unmount 和缓存失效。

### 阶段 2：Props revision 和 SignalDep

为 facade/component 层增加统一 props revision 和 signal dependency 描述。

验收：

1. leaf props 能产生稳定 revision。
2. Signal dependency 能记录 signal id、generation、dirty kind。
3. callback 改变不导致 revision 改变。

### 阶段 3：ActionDispatcher 和 FrameSnapshot

新增 Action 调度层和每帧 Signal snapshot。

验收：

1. controlled 组件 event 只 emit Action。
2. dispatcher 写 Signal 后进入 pending queue。
3. update、measure、render 读取同一个 FrameSnapshot。
4. snapshot 创建后的 Signal 写入进入下一帧。

### 阶段 4：ComponentInstance lifecycle

在 retained tree 中引入统一 lifecycle 调用。

验收：

1. key/type 相同复用 RuntimeState 和组件本地 Signal。
2. key 相同 type 不同替换实例。
3. mount/update/unmount 顺序正确。
4. unmount 退订 Signal。
5. controlled/uncontrolled 切换触发 state model replacement。

### 阶段 5：旧组件桥接适配器

新增旧 widget/component 协议桥接层，用于渐进迁移。

桥接规则：

1. `LegacyWidgetAdapter` 把旧 widget 包装成 ComponentInstance。
2. legacy adapter 不启用 memo、layout cache、render cache。
3. legacy adapter 默认把 props 更新标记为 `DirtyKind::Full` 或组件声明的保守 dirty。
4. legacy adapter 不能创建 component-owned Signal。
5. 迁移完成一个组件后移除对应 adapter 覆盖。

验收：

1. 新旧组件可以在同一页面共存。
2. adapter 不改变旧组件行为。
3. adapter 路径的性能指标单独输出，避免污染新协议 cache 命中率。

### 阶段 6：leaf 组件迁移

先迁移低风险组件：

1. Text。
2. Button。
3. Input。
4. RichText。

验收：

1. 现有行为不变。
2. props revision 不变时 update 可跳过。
3. useMemo 能缓存 text/rich text 的局部纯计算。
4. aster bench 不劣化。

### 阶段 7：Props derive 配套

在手写 trait 稳定后，新增 derive 宏减少模板代码。

目标形式：

```rust
#[derive(ComponentProps)]
struct TextProps {
    #[prop(revision)]
    text: TextSource,
    #[prop(revision)]
    style: TextStyle,
    #[prop(signal, dirty = "Layout")]
    text_signal: Option<ReadSignal<String>>,
    #[prop(skip_revision)]
    on_action: ActionMapper,
}
```

规则：

1. derive 只能生成 `revision()` 和 `signal_deps()`。
2. derive 不改变运行时语义。
3. callback 字段默认要求显式 `skip_revision`。
4. 大对象字段必须提供 revision source，不能自动深 hash。
5. derive 不是第一批实现的前置条件。

验收：

1. 手写实现和 derive 实现输出一致。
2. callback 改变不影响 revision。
3. signal 字段能生成正确 SignalDep。

### 阶段 8：容器组件迁移

迁移 Panel、Row、Col、Tabs。

验收：

1. children reconciliation 正确。
2. Tabs active child 切换产生 Structure dirty。
3. focus map 只在 Structure dirty 时重建。
4. aster bench 不劣化。

### 阶段 9：高负载组件迁移

迁移 ScrollView、Transcript、Markdown builder。

验收：

1. ScrollView 只渲染 viewport。
2. Transcript/Markdown 按 message body/theme 缓存 parse result。
3. 流式 token 只重算最后一条 assistant message。
4. streaming 和 scroll 场景不劣化，并有可测收益。

### 阶段 10：启用 cache 默认路径

在 shadow mode 无差异后，逐步启用 layout cache、render cache 和 dirty rect diff。

验收：

1. cache shadow mismatch 为 0。
2. P95/P99 不劣化。
3. C frames 为 0。
4. 优化报告记录收益和未覆盖场景。

## 测试要求

必须新增专项测试：

1. mount 创建 RuntimeState 和组件本地 Signal。
2. update 保留 RuntimeState 和组件本地 Signal。
3. key/type 不同触发 unmount + mount。
4. unmount 退订 Signal。
5. props revision 不变时跳过 update。
6. callback 改变不影响 revision。
7. Signal generation 变化只 dirty 订阅组件。
8. SignalDep 的 DirtyKind 正确传播。
9. useMemo deps 不变时命中。
10. useMemo deps 变化时重算。
11. computed lazy recompute。
12. computed generation 正确传播到组件。
13. measure 不产生 Action。
14. render 不订阅 Signal。
15. event 优先写 Signal，并返回 DirtyKind。
16. focus 切换只重绘 old/new focus。
17. ScrollView 长列表只渲染 viewport。
18. widget 变小后 old rect 被清理。
19. theme 切换后 render cache 失效。
20. resize 后 Full dirty。
21. key/type/state model 相同时本地 Signal 保留。
22. unmount 后本地 Signal 写入在 debug/test 下报错。
23. controlled/uncontrolled 切换触发 state model replacement。
24. 多个 Signal 同帧写入只触发一次 dirty 合并。
25. 大 fanout Signal 触发 dirty 合并兜底并写入 bench 日志。
26. measure 和 render 读取同一个 FrameSnapshot。
27. snapshot 创建后的 Signal 写入进入下一帧。
28. 混合派生按 computed + useMemo 两层拆分。
29. useMemo 超出容量后按 LRU 淘汰。
30. 长列表 unmount 后释放 row 私有 memo。
31. Structure dirty 优先局部重建 focus map。
32. 局部容器 resize 不提升为 Full。
33. ActionDispatcher 写 Signal 后进入 pending queue。
34. LegacyWidgetAdapter 不启用新 cache。
35. ComponentProps derive 与手写实现输出一致。

每阶段验证：

```bash
cargo test --workspace
cargo test -p aster-tui --features bench-log
cargo run -p aster-tui --features bench-log -- --bench
```

## 优化报告要求

每次启用默认路径优化后，必须输出本地报告：

```text
apps/tui-framework/docs/perf-optimization-report-<topic>-<date>.md
```

报告必须包含：

1. 基线日志路径。
2. 优化后日志路径。
3. P95/P99/C frames 对比。
4. 分 scene 对比。
5. cache hit/miss。
6. 负向指标。
7. 是否回滚。
8. 剩余风险。

## Crate 边界

建议归属：

| 内容 | crate |
| --- | --- |
| revision、SignalDep、DirtyKind、ComponentIdentity | `arbor-tui-domain` |
| retained tree、lifecycle runner、cache owner、组件本地 Signal owner | `arbor-tui-application` |
| Props facade、NodeSpec builder、用户 API | `arbor-tui` |
| leaf/container widgets 的具体行为 | `arbor-tui-widgets` |
| bench driver 和日志报告 | `aster-tui` |

`widgets` 不拥有全局 cache。widget 可以声明 revision、dirty hint 和 useMemo slot。cache 生命周期属于 application runtime。

## 已决议

1. Props 是输入描述。
2. Signal 是状态管理主路径。
3. NodeSpec 是 view 输出。
4. ComponentInstance 是 runtime 保留对象。
5. RuntimeState 只保存生命周期资源、memo store 和组件本地 Signal handle。
6. memo 是 runtime 级阶段跳过机制。
7. useMemo 是组件实例局部缓存。
8. computed 是 app/view-model 层响应式派生数据。
9. 本地 Signal 跟随 RuntimeState 复用和销毁。
10. controlled/uncontrolled 切换视为 state model 不兼容。
11. update、measure、render 读取同一个 FrameSnapshot。
12. 全局 Signal + layout size 的混合派生必须拆成 computed + useMemo。
13. useMemo 必须有容量上限和淘汰策略。
14. Structure dirty 优先局部修复，必要时才全量 fallback。
15. 局部容器 resize 不等同于终端 Full resize。
16. 第一目标是不劣化，性能提升必须由 bench 证明。
17. 旧 TEP-0005 不再作为终态组件协议依据。
