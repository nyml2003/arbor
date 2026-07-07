---
id: THEP-0010
title: "键盘事件与 Action Runtime"
status: Draft
created: 2026-07-08
updated: 2026-07-08
area: runtime
---

# THEP-0010: 键盘事件与 Action Runtime

## Summary

Thorn 下一阶段采用键盘优先的运行时协议。

运行时不支持鼠标。core 不提供 `Button` 组件。交互先走键盘事件到 Action，再由应用状态或 signal 驱动 view 更新。

这个方向来自两个已有项目：

- `apps/aster-rs`：Aster TUI 已经使用 `KeyEvent -> AsterAction -> AppState -> view` 的结构。`before_events` 负责把输入批次转成动作，`before_render` 负责处理流式状态、loading phase 和滚动边界。
- `apps/tui-framework`：arbor-tui 的终态文档已经把系统分成 domain、application、adapters、widgets、testing。runtime state 只能通过显式输入进入一步状态转移。E2E 用模拟输入和输出验证键盘、resize 和渲染结果。

Thorn 不复制旧 API。但 Thorn 要采用这两个项目已经证明有效的边界：键盘输入、Action、状态更新、纯 view、终端适配分层。

## Decision

### 1. 交互范围

MVP 后的第一个交互协议只支持键盘和 resize。

支持的输入：

- 普通字符键。
- `Enter`。
- `Escape`。
- `Backspace`。
- `Tab`。
- 方向键。
- `PageUp` / `PageDown`。
- `Home` / `End`。
- 常用 modifier：`Ctrl`、`Alt`、`Shift`。
- terminal resize。
- runtime tick。

不支持：

- mouse capture。
- mouse event。
- hover。
- click。
- drag。
- scroll wheel。
- `Button` 组件。

`Enter` 在没有应用 handler 时是 no-op。退出键先保留为 runtime 默认行为：`q` 和 `Escape` 退出。

### 2. 数据流

目标数据流：

```text
TerminalBackend
  -> RuntimeInput batch
  -> before_events
  -> Action
  -> update
  -> signal writes / app state writes
  -> before_render
  -> view
  -> layout
  -> render
  -> diff
  -> emit
```

规则：

1. terminal adapter 只把平台输入转成 core runtime input。
2. `before_events` 可以读取输入批次，并产出应用 Action。
3. `update` 只处理 Action 和应用状态。
4. `update` 不直接 render。
5. `before_render` 可以推进 tick、异步轮询结果、scroll clamp 和 loading 状态。
6. view 从应用状态或 signal 读取数据。
7. layout/render/diff 不读取终端输入。

### 3. Core 输入类型

`thorn-core` 应该拥有平台无关输入类型：

```rust
pub enum RuntimeInput {
    Key(KeyEvent),
    Resize(Size),
    Tick,
}

pub struct KeyEvent {
    pub key: Key,
    pub modifiers: KeyModifiers,
    pub kind: KeyEventKind,
}

pub enum Key {
    Char(char),
    Enter,
    Escape,
    Backspace,
    Tab,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    PageUp,
    PageDown,
    Home,
    End,
}
```

`thorn-terminal` 负责把 crossterm event 转成这些类型。crossterm 类型不能进入 `thorn-core`。

### 4. 应用入口

真实应用入口应从静态 root view 进化到状态化 app builder。

目标形态：

```rust
ThornApp::new(initial_state)
    .theme(Theme::dark())
    .update(update)
    .view(view)
    .before_events(before_events)
    .before_render(before_render)
    .run()
```

其中：

- `initial_state` 是应用状态。
- `update` 接收 `&mut State` 和 `Action`。
- `view` 接收 `&State` 或状态快照，并返回 `View<Action>`。
- `before_events` 处理输入批次。
- `before_render` 在每帧渲染前推进非输入状态。

`thorn::app(root)` 可以保留为静态 smoke 入口。它不应该成为复杂交互应用的主入口。

### 5. Action

Action 是应用交互协议，不是 widget 事件协议。

组件未来可以产生 Action。例如 `Input` 可以产生 `DraftChanged` 和 `SubmitInput`。但是当前阶段不增加 `Button`，也不增加鼠标点击 Action。

Action 处理规则：

1. Action 由应用定义。
2. core 不解析应用 Action。
3. runtime 按顺序把 Action 交给 `update`。
4. `update` 可以改变状态、写 signal、请求退出或切换 theme。
5. Action 执行后由 dirty/signal/render 流程决定是否重绘。

### 6. 测试入口

测试 harness 应支持脚本化键盘输入和 resize。

目标形态：

```rust
let mut app = TestRuntime::new(initial_state, update, view);
app.send_key(Key::Char('/'));
app.send_key(Key::Enter);
app.resize(80, 24);
app.render_frame();
app.assert_text("theme");
```

测试不使用真实终端。测试不注入鼠标事件。

## Non-goals

本 THEP 不做：

- 鼠标支持。
- `Button`。
- hover/click/drag。
- 完整 focus manager。
- Tab/Shift+Tab focus navigation。
- 文本编辑器级输入。
- IME。
- async scheduler。
- retained tree 优化。
- render cache。
- layout cache。
- 自定义 primitive node type。

这些能力必须单独写 THEP。

## API Impact

新增或调整这些公开方向：

- `thorn-core` 增加平台无关 `RuntimeInput`、`KeyEvent`、`Key`、`KeyModifiers`、`KeyEventKind`。
- `thorn-terminal` 增加 crossterm event 到 core input 的转换。
- `thorn` 增加状态化 app builder。
- `TestApp` 或新 `TestRuntime` 增加 `send_key`、`resize`、`render_frame`。

不增加这些 API：

- `button(...)`。
- `on_press(...)`。
- `press_first_focusable()`。
- mouse event 类型。
- mouse capture 配置。

## Test Requirements

必须测试：

- crossterm key event 能转换成 core `KeyEvent`。
- crossterm resize event 能转换成 core `RuntimeInput::Resize`。
- `q` 和 `Escape` 默认退出。
- `Enter` 没有 handler 时不改变 UI。
- runtime 不启用 mouse capture。
- resize 后下一帧 full dirty。
- `before_events` 可以把键盘输入转成 Action。
- `update` 可以通过 Action 修改状态。
- `before_render` 可以在无输入时推进状态。
- view 从最新状态渲染。
- 测试 harness 可以脚本化发送按键和 resize。
