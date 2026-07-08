# Thorn Enhancement Proposals

THEP 用来记录 Thorn 的架构和协议决策。

Thorn 当前处于概念重置阶段。本目录里的文档描述新的目标模型。旧 Thorn 代码和旧 THEP 不再作为约束。

## 状态

- `Draft`：正在讨论。
- `Accepted`：已经采纳，是当前设计约束。
- `Implemented`：已有代码和测试覆盖。
- `Superseded`：已被后续 THEP 替代。
- `Rejected`：明确不采用。

## 必填章节

每篇 THEP 必须包含：

- `摘要`
- `决策`
- `非目标`
- `API 影响`
- `测试要求`

## 索引

| ID | 标题 | 状态 |
| --- | --- | --- |
| THEP-0001 | Thorn 目标 | Accepted |
| THEP-0002 | UI 分层模型 | Accepted |
| THEP-0003 | Component 与 Element | Accepted |
| THEP-0004 | Host Tree | Accepted |
| THEP-0005 | App, State 与 Action Runtime | Accepted |
| THEP-0006 | 树变换管线 | Accepted |
| THEP-0007 | Layout 模型 | Accepted |
| THEP-0008 | Paint 与 Backend | Accepted |
| THEP-0009 | 优化与观测 | Accepted |
| THEP-0010 | 实现路线 | Accepted |
| THEP-0011 | 输入线程、KeyIntent 与 KeyMap | Accepted |
| THEP-0012 | MVP Slice | Accepted |
| THEP-0013 | Crate 分层 | Accepted |

## 编号规则

- 编号递增，使用四位数字。
- 不复用编号。
- 文件名使用 `THEP-0001-short-slug.md`。
- 决策变化时，新增 THEP，或把旧 THEP 标记为 `Superseded`。
- 一篇 THEP 只决定一个协议区域。
