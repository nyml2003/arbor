---
id: THEP-0002
title: "UI 分层模型"
status: Accepted
created: 2026-07-09
updated: 2026-07-09
area: architecture
---

# THEP-0002: UI 分层模型

## 摘要

Thorn 分离作者 UI、框架语义、布局、绘制和后端输出。

这个分离让 component model 保持稳定，同时允许后端实现演化。

## 决策

Thorn 有六层。

| 层 | 职责 |
| --- | --- |
| Component Layer | 用户和框架组件。组件返回 elements。 |
| Element Layer | 声明式 UI 树。可以包含语法糖和控制节点。 |
| Host Tree Layer | 后端无关的规范 UI 对象模型。 |
| Layout Layer | 测量并分配 rect 或后端特定布局单位。 |
| Paint Layer | 生成后端无关或后端族相关的 paint primitives。 |
| Backend Layer | 把 paint output 转成终端 cells、原生 draw calls、Web nodes 或 test snapshots。 |

数据向下流动。用户输入和后端事件向上流动，先转成 normalized runtime input，再转成 application actions。

层边界必须严格：

- Components 不绘制。
- Elements 不持有平台资源。
- Host Tree 不保存 terminal handle、native window、DOM node 或 GPU resource。
- Layout 不修改 application state。
- Paint 不 dispatch actions。
- Backend adapters 不从 node id 推断业务含义。

## 非目标

- 不把所有层压成一个 widget trait。
- 不让 backend adapter 直接调用 application update function。
- 不把 terminal cell grid 当成跨后端共享 IR。
- 不把 browser DOM 当成共享对象模型。

## API 影响

内部 API 应显式表达层转换：

```text
build_element_tree
normalize_host_tree
compute_layout
build_paint
present_backend_output
```

具体 Rust 函数名可以调整，但实现必须让这些转换能被测试观察到。

## 测试要求

测试必须覆盖：

- Element tree 可以不依赖 backend 构造。
- Host Tree 可以在 headless test 中检查。
- Layout output 可以不经渲染直接比较。
- Paint primitives 可以做 snapshot test。
- Backend adapter 可以和 component logic 分开测试。
