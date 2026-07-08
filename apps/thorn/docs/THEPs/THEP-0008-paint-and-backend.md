---
id: THEP-0008
title: "Paint 与 Backend"
status: Accepted
created: 2026-07-09
updated: 2026-07-09
area: backend
---

# THEP-0008: Paint 与 Backend

## 摘要

Paint primitives 是面向 renderer 的绘制命令。

Backend output 是后端自己的形态。Terminal output 应从 cell grid 和 dirty patch 构建。

## 决策

Paint input：

```text
Host Tree + Layout Tree + Theme + Runtime View State
```

Paint output：

```text
Paint Primitive Tree or Paint Primitive List
```

Core paint primitives：

- `FillRect`
- `TextRun`
- `Border`
- `Cursor`
- `Clip`
- `Layer`

Paint primitives 不做这些事：

- 持有 backend handles。
- 读取 terminal input。
- Dispatch actions。
- 拥有 business state。

Terminal backend lowering：

```text
Paint Primitive
  -> Cell Grid
  -> Dirty Patch
  -> ANSI or terminal backend calls
```

Terminal cell：

```text
Cell = char + foreground + background + attrs + wide-char-continuation
```

其他后端可以用不同方式下降 paint primitives：

- Native GUI 可以下降到 display lists 或 draw calls。
- Web 可以把 Host Tree 或 paint primitives 下降到 DOM、Canvas 或 WASM renderer output。
- Headless backend 可以下降到 snapshots。

Backends 必须声明 capabilities。

## 非目标

- 不把 terminal cells 作为通用 paint IR。
- 不让 components 看到 ANSI output。
- 不要求所有后端支持每个视觉特性。
- 不让 backend adapters 从 node id 推断业务含义。

## API 影响

Backend adapters 应实现 capability 和 presentation 边界：

```text
BackendCapabilities
BackendPresenter
PresentedFrame
```

Terminal-specific APIs 属于 terminal backend crate 或 module，不进入 core component APIs。

## 测试要求

测试必须覆盖：

- Paint primitives 可以不依赖 backend 构建。
- Terminal backend 能把 text 和 fill primitives 下降到 cells。
- Dirty patch 只包含变化的 terminal regions。
- Unsupported capabilities 返回结构化 errors。
- Headless backend 可以 snapshot paint output。
