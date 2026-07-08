---
id: THEP-0003
title: "Component 与 Element"
status: Accepted
created: 2026-07-09
updated: 2026-07-09
area: component
---

# THEP-0003: Component 与 Element

## 摘要

Component 是作者面对的组合单位。Element 是 component 产出的声明式节点。

Thorn 必须区分 component、host node、paint primitive 和 terminal cell。

## 决策

`Component` 的含义是：

```text
Component = props / state / signals -> Element
```

Component 分两类：

- `Business Component`：应用专属 UI，例如 `ChatPage`、`TaskList`。
- `Composite Component`：可复用 UI 组合，例如 `Panel`、`Transcript`、`FuzzyPanel`、`InputShell`。

`Element` 是 component 返回的声明式 UI 节点。

Element 分两类：

- `Structural Element`：控制或结构节点。例如 `Fragment`、`If`、`For`、`Slot`、`Row`、`Column`、`ThemeScope`。
- `Host Element`：框架语义节点。例如 `View`、`Text`、`TextInput`、`ScrollView`、`Image`、`Layer`、`Clip`。

Structural elements 可以在不破坏语义边界时被 lowering、拍平或删除。

Host elements 会进入 Host Tree，或下降为等价 host nodes。它们有框架语义，不是 paint primitives。

`TextInput` 是 host control。它不是纯绘制原语。它可以下降为 text runs、cursor primitives、focus state 和 actions。

## 非目标

- 不使用 `原子组件` 和 `分子组件` 作为 runtime 分类。
- 不让 components 持有 backend handles。
- 不让 paint primitives dispatch business actions。
- 不把每个 DSL helper 都建成持久 host node。

## API 影响

作者 API 应优先使用 component 和 element 概念：

```rust
fn chat_page(state: &ChatState) -> Element<AppAction> {
    column((
        transcript(state.messages()),
        text_input(state.draft()).on_submit(AppAction::Submit),
    ))
}
```

Builder helpers 可以是语法糖。runtime 决定哪些 elements 在 normalization 后保留。

## 测试要求

测试必须覆盖：

- Business components 可以组合 framework components。
- Composite components 可以下降为 elements，且不持有 backend resources。
- Structural elements 在不需要边界时可以被规范化移除。
- Host controls 在 normalization 后保留 identity 和 focus 语义。
