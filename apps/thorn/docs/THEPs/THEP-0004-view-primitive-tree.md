---
id: THEP-0004
title: "View 与 Primitive Tree"
status: Accepted
created: 2026-07-07
updated: 2026-07-07
area: view
---

# THEP-0004: View 与 Primitive Tree

## Summary

Thorn 的组件不是 widget 实例。

组件是普通函数。它在 mount 时创建响应式资源，并返回一棵 primitive tree。primitive tree 是平台无关的 UI 描述。它只描述布局、样式、文本、事件绑定、focus 和 children。

## Decision

核心类型：

```text
View<Action>
PrimitiveNode<Action>
NodeId
NodeKey
EventBinding<Action>
```

组件函数：

```rust
fn component(cx: &Scope) -> View<Action>
```

组件函数规则：

1. 只在 mount 时执行一次。
2. 可以创建 signal、memo、effect。
3. 可以组合 child view。
4. 不能直接访问终端。
5. 不能 emit ANSI。
6. 不能依赖组件函数重复执行来更新 UI。

primitive node 内容：

- node type。
- key。
- layout style。
- visual style。
- text slot。
- children。
- event bindings。
- focusable flag。

primitive node 不允许持有：

- terminal handle。
- crossterm 类型。
- file handle。
- thread handle。
- platform resource。
- render cache。

动态绑定：

- `text(move || ...)` 创建 dynamic text slot。
- `style(move || ...)` 创建 dynamic style slot。
- `show(when, fallback, child)` 管理分支 scope。
- `for_each(items, key, render)` 管理 keyed item scope。

事件绑定：

1. primitive node 保存事件绑定描述。
2. event handler 写 signal 或 emit app action。
3. event handler 不直接触发 render。
4. runtime 根据 signal/effect 变化更新 primitive slot。

应用层组件扩展：

1. 应用可以定义自己的函数组件。
2. 应用组件只要返回 `View<Action>`，就能和内置组件组合。
3. 应用组件默认通过组合已有 primitive 和基础组件扩展 UI。
4. 应用组件可以拥有自己的 signal、memo、effect 和 child scope。
5. 应用组件不需要注册到 framework。
6. 应用组件不能直接新增底层 primitive node type，除非后续 THEP 定义 custom primitive 扩展协议。

示例：

```rust
fn task_card(cx: &Scope, task: ReadSignal<Task>) -> View<Action> {
    panel(col((
        text(move || task.get().title),
        text(move || task.get().status_label()).fg(Token::TextMuted),
    )))
    .bg(Token::SurfaceAlt)
}
```

扩展层级：

| 层级 | MVP 是否支持 | 说明 |
| --- | --- | --- |
| 应用函数组件 | 支持 | 组合已有 view 和基础组件 |
| 应用 hook/helper | 支持 | 封装 signal、memo、effect 和业务逻辑 |
| 自定义基础组件 | 暂缓 | 需要公开更低层 component/primitive 协议 |
| 自定义 primitive node type | 暂缓 | 需要 runtime render/layout adapter 协议 |
| 自绘 cell 组件 | 暂缓 | 需要安全 `Frame` 或 `Canvas` API |

`Show` 规则：

- 条件为 true 时 mount child branch scope。
- 条件为 false 时 dispose child branch scope。
- fallback 有自己的 branch scope。
- 切换分支必须释放旧分支 effect。

`For` 规则：

- `For` 必须 keyed。
- key 相同的 item 复用 scope。
- 删除 item 时 dispose scope。
- 重排 item 时保留 item scope。
- item render 函数只在 item mount 时执行。

## Non-goals

- 不做 virtual DOM。
- 不做 React 风格 reconciliation。
- 不把组件建成 trait object widget。
- 不支持未 keyed 的动态列表。
- 不允许组件 render 阶段操作终端。
- 不在 MVP 中做自定义 renderer 插件。
- 不在 MVP 中开放自定义 primitive node type。
- 不在 MVP 中开放自绘 cell 组件 API。

## API Impact

建议用户 API：

```rust
col((
    text("static"),
    text(move || title.get()),
    show(
        move || is_loading.get(),
        || text("idle"),
        || text("loading"),
    ),
    for_each(
        move || items.get(),
        |item| item.id,
        |cx, item| text(move || item.label.clone()),
    ),
))
```

基础 builders：

- `text(...)`
- `row(...)`
- `col(...)`
- `panel(...)`
- `button(...)`
- `input(...)`
- `show(...)`
- `for_each(...)`

builder 只构造 view，不执行渲染。

应用层可以直接写组合组件：

```rust
fn header_bar(_cx: &Scope, title: impl Into<String>) -> View<Action> {
    row((
        text(title.into()).flex(1),
        button("Quit").on_press(|_| Action::Quit),
    ))
    .height(1)
    .bg(Token::SurfaceAlt)
}
```

这类组件不需要 framework 注册。它和内置组件的差别只在于：内置组件可以访问 core 内部 primitive 构造细节，应用组件只能使用公开 builder 和 hook。

## Test Requirements

必须测试：

- 静态 view mount 后生成 primitive tree。
- dynamic text signal 变化后只更新对应 text slot。
- dynamic style signal 变化后只更新对应 style slot。
- button action 可以写 signal。
- event handler 不直接调用 render。
- `Show` 初始 true/false 渲染正确。
- `Show` 切换释放旧分支 scope。
- `For` keyed insert/delete/reorder 行为正确。
- primitive node 不依赖 terminal adapter。
- 应用层函数组件可以组合内置组件并渲染。
- 应用层函数组件内部 signal 能驱动自己的 dynamic slot。
