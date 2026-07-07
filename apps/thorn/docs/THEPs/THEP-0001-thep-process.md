---
id: THEP-0001
title: "THEP 流程"
status: Accepted
created: 2026-07-07
updated: 2026-07-07
area: governance
---

# THEP-0001: THEP 流程

## Summary

THEP 是 Thorn 的长期协议源。

Thorn 不复用 `arbor-tui` 的 TEP。Thorn 是新项目，有自己的组件模型、响应式模型、布局边界和主题系统。所有会影响公共 API、核心协议、测试要求或分层边界的决策，都必须记录在 THEP 中。

## Decision

THEP 全称是 `Thorn Enhancement Proposal`。

THEP 存放在：

```text
apps/thorn/docs/THEPs/
```

THEP 状态只允许这 5 个值：

- `Draft`
- `Accepted`
- `Implemented`
- `Superseded`
- `Rejected`

状态语义：

- `Draft` 只能作为讨论材料。实现不能依赖 Draft。
- `Accepted` 是约束。实现必须遵守。
- `Implemented` 表示代码和测试已经覆盖该 THEP。
- `Superseded` 表示被后续 THEP 替代。
- `Rejected` 表示明确不采用。

每个 THEP 必须有 front matter：

```yaml
---
id: THEP-0001
title: "THEP 流程"
status: Accepted
created: 2026-07-07
updated: 2026-07-07
area: governance
---
```

每个 THEP 正文必须包含：

- `Summary`
- `Decision`
- `Non-goals`
- `API Impact`
- `Test Requirements`

实现规则：

1. 代码不能和 `Accepted` 或 `Implemented` THEP 冲突。
2. 需要改变协议时，先更新 THEP。
3. 新增行为如果只属于实现细节，不需要 THEP。
4. 新增公共 API、crate 边界、组件生命周期、布局规则、主题规则、渲染协议时，必须写 THEP。
5. THEP 编号递增，不复用。

## Non-goals

- 不把 THEP 做成流程负担。
- 不要求每个 bug fix 都写 THEP。
- 不要求 THEP 覆盖代码内部所有私有结构。
- 不追求和 Rust RFC、React RFC 或 arbor-tui TEP 格式一致。

## API Impact

THEP 本身不引入运行时 API。

THEP 会约束后续 API：

- `thorn-core` 的响应式和 view 协议。
- `thorn-terminal` 的终端适配协议。
- `thorn` facade 的用户入口。
- 组件、主题、布局和测试的公共命名。

## Test Requirements

需要增加文档检查。

最低检查项：

- `apps/thorn/docs/THEPs/README.md` 存在。
- THEP 文件名匹配 `THEP-NNNN-*.md`。
- `id` 和文件编号一致。
- `status` 是允许值。
- 必填章节存在。
- 编号唯一。

第一阶段可以用脚本或人工检查。进入代码阶段后，应把检查纳入 `cargo test` 或独立 docs lint。

