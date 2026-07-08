---
id: THEP-0010
title: "实现路线"
status: Accepted
created: 2026-07-09
updated: 2026-07-09
area: planning
---

# THEP-0010: 实现路线

## 摘要

实现顺序要跟架构层级一致。

不要从 terminal demo 开始。先写类型、纯变换和 headless tests。

## 决策

按这些阶段推进。

### 阶段 1：类型和协议合同

创建 core crate，定义：

- Component-facing element types。
- Host Tree types。
- Layout types。
- Paint primitive types。
- Backend capability types。
- App trait 或等价 app contract。
- Runtime input 和 action queue types。
- KeyIntent、KeyAction 和 KeyMap types。

这个阶段不需要真实 terminal backend。

### 阶段 2：Element 到 Host Tree

实现：

- DSL helpers。
- Element lowering。
- Host Tree normalization。
- Debug provenance。
- Headless Host Tree snapshots。

### 阶段 3：Layout

实现：

- Row 和 column layout。
- Fixed size、flex、gap、padding、margin。
- 通过 backend metrics 测量 text。
- TUI cell metrics 作为第一个 backend metrics provider。

### 阶段 4：Paint

实现：

- Fill、text run、border、cursor、clip。
- Host Tree 和 Layout Tree 到 paint primitives 的 lowering。
- Headless paint snapshots。

### 阶段 5：Terminal Backend

实现：

- Paint primitive 到 cell grid。
- Cell diff。
- Dirty patch。
- Memory terminal backend。
- 后续再做 real terminal backend。

### 阶段 6：App, State 与 Action Runtime

实现：

- `App<State, Action>` 或等价 app struct contract。
- UI thread 和 input thread 边界。
- Runtime input normalization。
- KeyMap resolution。
- KeyIntent resolution。
- KeyAction dispatch。
- Action queue。
- `update`。
- `view`。
- Request-render。
- Quit。
- Headless runtime tests。

### 阶段 7：真实应用组件

实现真实应用需要的组件：

- `View`
- `Text`
- `TextInput`
- `ScrollView`
- `Panel`
- `Transcript`
- `FuzzyPanel`

Composite components 必须通过 Element 和 Host Tree 下降。它们不能绕过管线。

### 阶段 8：观测和优化

实现：

- Frame stats。
- Dirty kind tracking。
- 合法 tree flattening。
- Text width cache。
- Layout cache。前提是已有 correctness tests。
- Paint cache。前提是已有 correctness tests。

## 非目标

- 不恢复旧 Thorn 代码。
- 不迁移旧 `arbor-tui` widget 协议。
- Headless output 可用前，不实现 real terminal backend。
- 第一阶段不做 mouse、IME 或 browser DOM support。
- 不让 cache correctness 依赖人工 visual inspection。

## API 影响

Crate 拆分遵守 `THEP-0013`。

MVP 使用：

```text
thorn-core       pure types, lowering, layout, paint, runtime model
thorn-headless   memory backend, snapshot, test runtime
thorn            public facade
```

后续当 headless runtime 逻辑膨胀时，再抽 `thorn-runtime`。真实 terminal backend 进入开发时，再加 `thorn-terminal`。

## 测试要求

每个阶段进入下一步前，都要有聚焦测试：

- Stage 1：type construction 和 invariants。
- Stage 2：element lowering 和 host snapshots。
- Stage 3：layout snapshots。
- Stage 4：paint snapshots。
- Stage 5：cell grid 和 dirty patch tests。
- Stage 6：action runtime tests。
- Stage 6：app struct owns state tests。
- Stage 6：input thread cannot mutate state tests。
- Stage 6：keymap composition tests。
- Stage 6：key intent resolution tests。
- Stage 7：component behavior tests。
- Stage 8：stats 和 cache correctness tests。

第一个 end-to-end 目标是：

```text
App / State
  -> view
  -> Element Tree
  -> Host Tree
  -> Layout Tree
  -> Paint Primitive
  -> Cell Grid
  -> Headless snapshot
```

第一个可开发 MVP 的精确定义见 `THEP-0012`。
