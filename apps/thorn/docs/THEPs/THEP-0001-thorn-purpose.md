---
id: THEP-0001
title: "Thorn 目标"
status: Accepted
created: 2026-07-09
updated: 2026-07-09
area: architecture
---

# THEP-0001: Thorn 目标

## 摘要

Thorn 是后端无关的 UI runtime。

它负责把 app、state、actions 和 component DSL 转成后端输出。TUI 是第一个后端，但不能成为唯一心智模型。

## 决策

Thorn 使用这条核心管线：

```text
App / State / Signals
  -> Component
  -> Element Tree
  -> Host Tree
  -> Layout Tree
  -> Paint Primitive
  -> Backend Output
```

对于第一个 TUI 后端：

```text
Paint Primitive
  -> Cell Grid
  -> Dirty Patch
  -> Terminal Backend
```

Thorn 负责这些能力：

- Component DSL。
- Element 构造。
- Host Tree 规范化。
- State 与 Action runtime。
- Layout 协议。
- Paint Primitive 协议。
- Backend capability 协商。
- Headless test backend。
- 性能观测。

Thorn 不继承旧 `arbor-tui` 协议。

正式术语是：

- `Component`
- `Element`
- `Host Tree`
- `Layout Tree`
- `Paint Primitive`
- `Backend Output`
- `App`

`原子组件` 和 `分子组件` 可以用于产品或设计系统讨论，但不是 Thorn runtime 术语。

## 非目标

- 不复刻浏览器 DOM。
- 不把 terminal 细节暴露到 core component API。
- 不把 TUI 写成唯一后端模型。
- 不保留旧 Thorn 或 `arbor-tui` API。
- 协议文档稳定前，不开始实现代码。

## API 影响

未来公开 API 的目标形态：

```rust
thorn::app(initial_state)
    .update(update)
    .view(view)
    .backend(thorn::terminal())
    .run()
```

这只是目标形态，不是最终签名承诺。

API 必须让应用代码位于后端细节之上。用户写 state、actions 和 view components。backend adapter 处理 terminal、native GUI、Web 或 test output。

## 测试要求

开始实现后，测试必须证明：

- Core UI 逻辑不依赖真实终端。
- 同一棵 component tree 可以通过 headless backend 渲染。
- Terminal backend 通过 backend adapter 实现。
- Backend-specific capability 不泄漏进 core components。
