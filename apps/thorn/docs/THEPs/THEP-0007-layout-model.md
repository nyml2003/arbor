---
id: THEP-0007
title: "Layout 模型"
status: Accepted
created: 2026-07-09
updated: 2026-07-09
area: layout
---

# THEP-0007: Layout 模型

## 摘要

Thorn 的 layout 在协议层后端无关，在单位层后端相关。

TUI 后端使用 cell 单位。Native GUI 后端可以使用 pixels 或 logical points。

## 决策

Layout input：

```text
Host Tree + Root Constraints + Backend Metrics
```

Layout output：

```text
Layout Tree
```

Layout Tree 记录：

- Host node ID。
- Final rect。
- Content rect。
- Clip rect。
- Measured size。
- Overflow metadata。
- 后端支持时记录 baseline 或 text metrics。

Layout 必须确定。

第一版 layout model 支持：

- Row 和 column direction。
- Fixed size。
- Min size。
- Flex grow。
- Gap。
- Padding。
- Margin。
- Main-axis alignment。
- Cross-axis alignment。
- Clip 和 scroll viewport。

Backend metrics 定义：

- Unit type。
- Text measurement。
- Font 或 cell metrics。
- Rounding policy。

对于 TUI：

- Unit 是 terminal cell。
- 所有 final rect 都是整数 cell rectangles。
- Text measurement 使用 display width，不使用 byte length。

## 非目标

- 不实现完整 CSS。
- 不要求 core protocol 使用 pixel layout。
- 不让 layout 读取 backend input events。
- 不让 layout 修改 application state。
- 不支持 layout callback 在当前 frame 重入 layout。

## API 影响

内部 API 应分离 constraints 和 backend metrics：

```text
LayoutConstraints
BackendMetrics
LayoutNode
LayoutTree
```

Public components 通过 host props 表达 layout intent，不直接调用 backend。

## 测试要求

测试必须覆盖：

- Row 和 column placement。
- Fixed 和 flex sizing。
- Padding、margin 和 gap。
- 通过 backend metrics 测量 text。
- TUI integer rounding。
- Resize 产出确定 layout。
- Scroll viewport 裁切 paint，但不删除 logical content。
