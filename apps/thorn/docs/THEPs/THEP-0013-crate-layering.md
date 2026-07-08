---
id: THEP-0013
title: "Crate 分层"
status: Accepted
created: 2026-07-09
updated: 2026-07-09
area: architecture
---

# THEP-0013: Crate 分层

## 摘要

Thorn 按水平架构拆 crate，按竖直 UI pipeline 拆 module。

Rust crate 是编译、依赖和发布边界。它不等同于文件夹边界。不要把每个概念都拆成 crate。

## 决策

Thorn 使用两种分层。

水平分层对应 Clean Architecture：

```text
domain / model
application / runtime orchestration
adapter / backend
facade / public API
```

竖直分层对应 UI pipeline：

```text
Component
Element
Host Tree
Layout
Paint
Backend Output
```

Crate 按水平分层拆。

Module 按竖直 pipeline 拆。

不要反过来。否则会产生太多小 crate，核心类型每次变化都要跨 crate 修改。

### 当前 MVP crate

当前 MVP 使用五个 crate：

```text
thorn-core
thorn-runtime
thorn-headless
thorn-terminal
thorn
```

职责：

| Crate | 水平层 | 职责 |
| --- | --- | --- |
| `thorn-core` | domain / model | 纯类型、纯变换、Element、Host Tree、Layout、Paint、Screen、Input model |
| `thorn-runtime` | application / runtime orchestration | `AppRuntime`、RuntimeInput handling、KeyIntent resolution、Action dispatch、render scheduling、App lifecycle |
| `thorn-headless` | adapter / test runtime | Memory adapter、snapshot、`TestRuntime` |
| `thorn-terminal` | adapter / demo runtime | 标准输入输出 adapter、可见 Counter demo。当前不包含 raw mode、alternate screen 或真实 input thread |
| `thorn` | facade | `prelude`、re-export、用户入口 |

### 后续目标 crate

当前 `thorn-terminal` 是 stdio demo adapter。当真实终端后端进入开发时，扩展它的职责：

```text
thorn-terminal
```

职责：

```text
crossterm adapter
input thread
terminal presenter
raw mode guard
alternate screen guard
```

目标形态：

```text
thorn-core
thorn-runtime
thorn-headless
thorn-terminal
thorn
```

### thorn-core 内部 module

`thorn-core` 内部按竖直 pipeline 拆 module：

```text
app
element
host
layout
paint
screen
input
```

这些暂时是 module，不是 crate。

### 不立即拆的 crate

暂不拆：

```text
thorn-element
thorn-host
thorn-layout
thorn-paint
thorn-input
thorn-keymap
thorn-screen
```

这些概念会快速变化。现在拆 crate 会降低迭代速度。

只有满足这些条件，才考虑拆出新 crate：

- API 稳定。
- 有独立测试价值。
- 有独立复用价值。
- 依赖方向清楚。
- 拆分能减少编译或维护成本，而不是只让目录更整齐。

## DDD 和整洁架构解释

Thorn 的领域不是用户应用的业务领域。Thorn 的领域是 UI runtime。

领域对象包括：

- `Element`
- `HostNode`
- `LayoutNode`
- `PaintPrimitive`
- `Screen`
- `RuntimeInput`
- `KeyIntent`
- `KeyAction`

应用层对象包括：

- `Runtime`
- `FrameLoop`
- `ActionQueue`
- `IntentResolver`

适配器包括：

- `HeadlessBackend`
- `TerminalBackend`
- `WebBackend`
- `NativeBackend`

Facade 包括：

- `thorn::prelude`
- `thorn::app(...)`
- 高层 builder API

依赖方向：

```text
thorn
  -> thorn-headless / thorn-terminal / thorn-runtime
  -> thorn-core

thorn-headless -> thorn-core
thorn-terminal -> thorn-runtime -> thorn-core
thorn-runtime  -> thorn-core
```

禁止反向依赖：

- `thorn-core` 不能依赖 `thorn-runtime`。
- `thorn-core` 不能依赖 `thorn-headless`。
- `thorn-core` 不能依赖 `thorn-terminal`。
- Backend adapter 不能污染 core 类型。

## 非目标

- 不按每个 UI pipeline 节点拆 crate。
- 不为了目录美观拆 crate。
- 不在 headless MVP 前实现真实 terminal backend。
- 不让 backend 反向依赖进入 core。

## API 影响

当前 workspace members：

```toml
members = [
    "crates/thorn-core",
    "crates/thorn-runtime",
    "crates/thorn-headless",
    "crates/thorn-terminal",
    "crates/thorn",
]
```

新增 adapter crate 前，需要先确认对应后端职责已经超过当前 crate 的合理范围。

`thorn-core` 的 public API 要小。竖直 pipeline module 可以先内部可见，等使用场景稳定后再公开。

## 测试要求

测试必须覆盖：

- `thorn-core` 不依赖 headless 或 terminal crate。
- `thorn-runtime` 只依赖 `thorn-core`。
- `thorn-headless` 通过 `thorn-runtime` 和 core public API 构建 snapshot。
- `thorn-terminal` 通过 `thorn-runtime` 和 core public API 做终端展示。
- `thorn` facade 不实现核心规则，只 re-export 或组合 API。
- 新增 crate 时，workspace test 和 check 必须通过。
- 如果新增 `thorn-terminal`，core/headless 测试不能依赖真实终端。
