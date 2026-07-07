# Thorn Enhancement Proposals

THEP 是 Thorn 的协议和架构决策记录。

Thorn 还没有稳定代码时，THEP 是实现入口。实现代码必须服从已接受的 THEP。需要改变协议时，先更新或新增 THEP，再改代码。

## 状态

- `Draft`：正在讨论，不能作为实现约束。
- `Accepted`：已经采纳，后续实现必须遵守。
- `Implemented`：已经有代码和测试覆盖。
- `Superseded`：被后续 THEP 替代。
- `Rejected`：明确不采用。

## 必填字段

每个 THEP 文件必须包含 front matter：

```yaml
---
id: THEP-0001
title: "标题"
status: Accepted
created: 2026-07-07
updated: 2026-07-07
area: architecture
---
```

正文必须包含这些章节：

- `Summary`
- `Decision`
- `Non-goals`
- `API Impact`
- `Test Requirements`

## 初始 THEP

| ID | 标题 | 状态 |
| --- | --- | --- |
| THEP-0001 | THEP 流程 | Accepted |
| THEP-0002 | 项目架构和 `thorn-core` 领域分组 | Accepted |
| THEP-0003 | 响应式与 Scope | Accepted |
| THEP-0004 | View 与 Primitive Tree | Accepted |
| THEP-0005 | FlexBox 子集布局 | Accepted |
| THEP-0006 | 主题系统 | Accepted |
| THEP-0007 | 终端渲染与适配器 | Accepted |
| THEP-0008 | MVP 后下一阶段计划 | Draft |
| THEP-0009 | 性能观测与性能上限 | Accepted |
| THEP-0010 | 键盘事件与 Action Runtime | Draft |
| THEP-0011 | Aster 等价替代目标 | Accepted |

## Core 领域

`thorn-core` 是纯核心 crate。它内部按领域分组，不把所有能力放进一个平铺模块。

核心领域：

- `reactive`：Signal、Memo、Effect、Scope。
- `view`：View、Primitive Tree、动态文本和样式。
- `layout`：几何类型和 FlexBox 子集。
- `theme`：Theme、Token、Color、样式解析。
- `render`：Cell、Screen、Diff、screen compose。
- `widgets`：基础布局、文本、面板和后续控制流组件。
- `testing`：测试 harness 和断言工具。

## 编号规则

- 编号递增，不复用。
- 文件名使用 `THEP-0001-short-slug.md`。
- 替代旧 THEP 时，不改旧编号。把旧 THEP 状态改成 `Superseded`，并在正文写明替代者。
- 一个 THEP 只解决一个协议问题。不要把无关决议塞进同一篇。
