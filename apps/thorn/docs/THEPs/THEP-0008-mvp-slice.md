---
id: THEP-0008
title: "MVP 纵向切片"
status: Accepted
created: 2026-07-07
updated: 2026-07-07
area: planning
---

# THEP-0008: MVP 纵向切片

## Summary

Thorn MVP 只做一个薄纵向切片。

MVP 的目标不是完成整个 TUI 框架。MVP 的目标是证明核心模型成立：signal 能驱动 primitive slot，FlexBox 子集能布局，theme token 能解析，screen diff 能精准更新 cell，测试 harness 能验证结果。

## Decision

MVP 必须覆盖从响应式到内存渲染的完整路径：

```text
Signal write
  -> Effect rerun
  -> Primitive slot update
  -> Flex layout
  -> Theme resolve
  -> Screen render
  -> Diff dirty regions
  -> Test harness assert
```

MVP 包含这些能力：

| 领域 | MVP 范围 |
| --- | --- |
| workspace | `apps/thorn` 可独立 `cargo test` 和 `cargo check` |
| `reactive` | `Signal`、`ReadSignal`、`Effect`、`Scope` cleanup |
| `view` | static text、dynamic text、primitive node、event binding 描述 |
| `layout` | 内部 measure、`Row`、`Col`、fixed width/height、`flex` grow、padding、gap、测试可读 layout info |
| `theme` | `Theme::dark()`、`Theme::light()`、`Token` -> `Color` |
| `render` | `Cell`、`Screen`、`fill_rect`、`write_str`、row diff、single-cell dirty |
| `widgets` | `Text`、`Row`、`Col`、`Panel`、`Button` |
| `testing` | render 到内存 screen，断言文本、背景和 dirty region |
| examples | counter 或 theme switch demo，二选一即可 |

MVP 支持应用层自定义组件。

应用层自定义组件是普通函数。它返回 `View<Action>`，并通过公开 builder 组合内置组件。它可以创建自己的 signal 和 effect。它不需要注册，也不需要实现 trait。

MVP 不支持应用层新增底层 primitive node type。新增基础 primitive、自绘 cell 组件或 renderer adapter，需要后续 THEP 单独定义。

MVP 组件要求：

- `Text` 支持静态文本和动态文本。
- `Row` / `Col` 只负责布局 children。
- `Panel` 填充背景，可选边框。
- `Button` 可 focus，可触发 event binding。
- 所有可见组件必须写背景。

MVP demo 建议：

```rust
fn counter(cx: &Scope) -> View<Action> {
    let count = cx.create_signal(0usize);

    col((
        panel(text(move || format!("count: {}", count.get()))),
        button("+1").on_press(move |_| count.update(|n| *n += 1)),
    ))
    .padding(1)
    .gap(1)
    .bg(Token::Surface)
}
```

MVP 完成标准：

1. 初次 render 能在内存 screen 中看到 `count: 0`。
2. 触发 button event 后，signal 更新为 `1`。
3. dynamic text slot 更新为 `count: 1`。
4. diff 能产生覆盖变化文本的 dirty region。
5. light theme 下没有可见默认黑底。
6. `cargo test --manifest-path apps/thorn/Cargo.toml --workspace` 通过。
7. `cargo check --manifest-path apps/thorn/Cargo.toml --workspace` 通过。

## Non-goals

MVP 不做这些能力：

- 真实终端 runtime。
- `Input`。
- `Show`。
- `For`。
- `Memo`。
- async effect。
- 跨线程 signal 写入。
- terminal raw mode。
- resize。
- mouse。
- render cache。
- dirty rect subtree cache。
- 自定义 primitive node type。
- 自绘 cell 组件 API。
- 应用层自定义 measure。
- 同步 on_layout。
- on_layout 写 signal 后当前帧重布局。
- markdown。
- table。
- scroll area。
- transcript。
- crossterm adapter 完整实现。

这些能力可以后续按 THEP 增量实现。

## API Impact

MVP 只需要暴露最小用户 API：

```rust
use thorn::prelude::*;

fn app(cx: &Scope) -> View<Action> {
    text("hello")
}
```

应用层组合组件必须可用：

```rust
fn counter_panel(cx: &Scope, count: ReadSignal<usize>) -> View<Action> {
    panel(text(move || format!("count: {}", count.get())))
}
```

必须可用的 builder：

- `text(...)`
- `row(...)`
- `col(...)`
- `panel(...)`
- `button(...)`

必须可用的样式方法：

- `.width(u16)`
- `.height(u16)`
- `.flex(u16)`
- `.padding(u16)`
- `.gap(u16)`
- `.fg(Token)`
- `.bg(Token)`

必须可用的测试 API：

```rust
let mut app = TestApp::new(counter);
app.render(40, 8);
app.assert_text("count: 0");
app.press_button("+1");
app.assert_text("count: 1");
app.assert_no_default_bg_on_text();
```

具体签名可以调整，但测试语义必须保留。

测试可以读取 layout 信息：

```rust
let rect = app.layout_of("counter-panel");
assert_eq!(rect.h, 3);
```

MVP 不提供：

```rust
view.on_layout(...)
view.measure(...)
```

后续如需 `on_layout`，它必须是 after-layout hook。它只能观察 rect。它写 signal 时只能调度下一帧。

## Test Requirements

MVP 必须有这些测试：

- signal set 触发 dynamic text effect。
- scope dispose 后 effect 不再运行。
- static text render 到 screen。
- dynamic text signal 更新后 render 到 screen。
- row layout 横向排列。
- col layout 纵向排列。
- flex child 获得剩余空间。
- padding 缩小 content rect。
- gap 分隔 children。
- 测试 harness 可以读取 layout info。
- dark theme token 解析。
- light theme token 解析。
- panel 背景填满 rect。
- button event 可以写 signal。
- 应用层函数组件可以渲染。
- 应用层函数组件内部 signal 可以更新 dynamic text。
- screen diff 支持单 cell dirty。
- 文本变化产生 dirty region。
- light theme 下没有可见默认黑底。

MVP 不需要端到端真实终端测试。所有验收先在内存 screen 和测试 harness 中完成。
