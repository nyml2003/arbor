---
id: THEP-0006
title: "树变换管线"
status: Accepted
created: 2026-07-09
updated: 2026-07-09
area: pipeline
---

# THEP-0006: 树变换管线

## 摘要

渲染是一串树和 IR 的变换。

Thorn 要让每个变换步骤显式、可测试。

## 决策

完整管线是：

```text
App / State / Signals
  -> Component evaluation
  -> Element Tree
  -> Element lowering
  -> Host Tree
  -> Host normalization
  -> Layout Tree
  -> Paint Primitive Tree or List
  -> Backend Output
```

Element lowering 可以：

- 展开 DSL 语法糖。
- 展开 composite components。
- 解析 `If`、`For`、`Slot` 和 `Fragment`。
- 把 layout helpers 转成 host layout props。

Host normalization 可以：

- 删除 transparent wrappers。
- 合并等价 style scopes。
- 裁剪 default props。
- 在合法时合并相邻 text。
- 保留 identity 和 debug provenance。

只有不跨越语义边界时，才允许 flatten。

不能跨越这些边界：

- Key 或 identity。
- Reactive scope。
- Cleanup lifetime。
- Focus 或 cursor。
- Clip 或 scroll viewport。
- Layout measurement boundary。
- Event 或 action boundary。
- Theme boundary。
- Accessibility boundary。
- 诊断所需的 debug provenance。

## 非目标

- 不把全量重建 component tree 作为唯一 update 策略。
- 不为了树形好看盲目 flatten。
- 不让 renderer 理解所有 DSL 语法糖。
- 不要求从 normalized tree 完美还原源码。

## API 影响

内部 debug tooling 应暴露这些快照：

- Element Tree。
- Host Tree。
- Layout Tree。
- Paint primitives。
- Backend output summary。

Public API 默认不需要暴露所有 IR。Test API 应暴露它们。

## 测试要求

测试必须覆盖：

- DSL sugar 能下降到 canonical elements。
- Transparent wrappers 会被移除。
- Boundaries 会阻止非法 flattening。
- Debug provenance 能穿过 lowering。
- 等价输入能产出确定的 normalized trees。
