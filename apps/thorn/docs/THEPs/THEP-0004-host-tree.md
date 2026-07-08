---
id: THEP-0004
title: "Host Tree"
status: Accepted
created: 2026-07-09
updated: 2026-07-09
area: host-tree
---

# THEP-0004: Host Tree

## 摘要

Host Tree 是 Thorn 的 DOM-like 对象模型。

它后端无关。它不是浏览器 DOM。

## 决策

Host Tree 是 Element Tree 和 Layout Tree 之间的规范 UI 语义树。

它携带：

- Node identity。
- Optional key。
- Scope identity。
- Node kind。
- Children。
- Layout constraints。
- Style tokens。
- Accessibility metadata。
- Focus 和 input affordances。
- Action binding。
- Debug provenance。

Host Tree 不携带：

- Terminal handles。
- Native window handles。
- Browser DOM nodes。
- GPU resources。
- Backend caches。
- Business state。
- Rendered cells。

Host Tree 让多个后端共享 component semantics。

Backend adapters 把 Host Tree 和 Layout Tree 下降到 backend output。某个后端可以只支持部分 host semantics。不支持的语义必须通过明确定义的规则降级，或返回结构化 unsupported-capability error。

正式术语：

```text
Host Tree
```

非正式类比：

```text
Host Tree 承担类似 DOM 的职责，但它不是 DOM。
```

## 非目标

- 不暴露 mutable DOM-style node API。
- 不把 CSS selectors、cascade 或 mutation observers 纳入 Host Tree。
- 不要求 Web 兼容。
- 不把每个 DSL helper 都塞进 Host Tree。

## API 影响

内部 host nodes 应有稳定身份：

```text
HostNodeId
HostKey
HostKind
HostProps
HostChildren
```

第一批 host kinds 应保持很小：

- `View`
- `Text`
- `TextInput`
- `ScrollView`
- `Clip`
- `Layer`

新增更多 host kinds 需要单独 THEP。

## 测试要求

测试必须覆盖：

- Element Tree 能规范化成 Host Tree。
- Host node identity 能穿过 normalization。
- Debug provenance 能从 Host Tree 指回来源 elements。
- Host Tree 可以不依赖 backend 做 snapshot test。
- Unsupported backend capability errors 能指出失败的 host feature。
