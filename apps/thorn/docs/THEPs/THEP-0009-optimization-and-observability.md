---
id: THEP-0009
title: "优化与观测"
status: Accepted
created: 2026-07-09
updated: 2026-07-09
area: performance
---

# THEP-0009: 优化与观测

## 摘要

Thorn 的优化分两层：

- View 和 tree 优化。
- Render 和 backend 优化。

性能优化要先可观测，再变复杂。

## 决策

View 和 tree 优化包括：

- DSL desugaring。
- Element lowering。
- Transparent wrapper flattening。
- Default prop pruning。
- Static subtree detection。
- Adjacent compatible text merge。
- Dirty kind classification。
- Layout cache。
- Paint cache。
- Debug provenance。

Render 和 backend 优化包括：

- Text width cache。
- Viewport clipping。
- Paint primitive cache。
- Cell grid diff。
- Dirty region merge。
- ANSI span merge。
- Style reset minimization。
- Backend byte 和 flush metrics。

Dirty kinds：

```text
Render < Layout < Structure < Theme < Full
```

Dirty kind 含义：

- `Render`：视觉输出改变，但 layout 不移动。
- `Layout`：尺寸或位置可能改变。
- `Structure`：node identity 或 child structure 改变。
- `Theme`：style token resolution 大范围改变。
- `Full`：backend reset、root resize 或无法局部恢复的失效。

性能统计应记录：

- Frame index。
- Total frame time。
- Component/update time。
- Lowering time。
- Layout time。
- Paint time。
- Backend lowering time。
- Diff time。
- Emit 或 present time。
- Host node count。
- Paint primitive count。
- Dirty node count。
- Dirty regions。
- Backend output size。

默认 instrumentation 必须低开销。关闭时使用 no-op 路径。

## 非目标

- 协议不可测试前，不加缓存。
- 不用优化破坏语义。
- 不把 backend work 藏进 components。
- 不用人工看终端来证明性能。

## API 影响

Runtime 应支持可选 performance sink：

```text
PerfSink
FrameStats
PresentedFrame
```

Test runtime 应暴露最近一帧 stats。

## 测试要求

测试必须覆盖：

- Dirty kind merge order。
- Render dirty 不强制 layout。
- Layout dirty 会失效受影响 layout。
- Flattening 保留边界。
- 启用 stats 时能收集统计。
- No-op stats sink 不在 hot path 分配字符串。
- Dirty patch output 与 backend diff 一致。
