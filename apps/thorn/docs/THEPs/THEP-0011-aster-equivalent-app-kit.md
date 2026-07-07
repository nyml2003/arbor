---
id: THEP-0011
title: "Aster 等价替代目标"
status: Accepted
created: 2026-07-08
updated: 2026-07-08
area: planning
---

# THEP-0011: Aster 等价替代目标

## Summary

Thorn 的下一阶段目标是替代 `arbor-tui`，成为 Aster 的 TUI 基座。

替代目标不是复制 `arbor-tui` 的旧组件协议。旧协议不作为兼容目标。Thorn 已经统一了 primitive tree、layout、theme、render 和 Action Runtime 的方向，后续应沿 Thorn 的模型补齐应用级能力。

Markdown 渲染可以暂缓。除 Markdown 外，Aster 当前依赖的 TUI 行为必须等价。

等价的含义是：

- Aster 可以迁到 Thorn，不丢失当前聊天 TUI 的核心交互。
- Aster 的 stream、scroll、input、palette、theme 和测试路径都能继续工作。
- Aster 不需要回到 `WidgetFactory`、`WidgetNode` 或旧 `Widget` trait 协议。
- 普通 Aster 页面代码应继续表达为 `State -> Action -> update -> view -> run`。

## Decision

### 1. 替代范围

Thorn 必须补齐 Aster 真实使用的应用级 TUI 能力。

必须等价的能力：

- 非阻塞运行循环。
- 每帧 tick。
- 可覆盖默认退出语义。
- `before_events` 和 `before_render`。
- Action queue。
- runtime context。
- theme 切换。
- signal 写入和局部重绘触发。
- 输入框。
- 滚动视口。
- 聊天 transcript。
- command palette。
- headless 测试入口。
- 性能统计入口。

暂缓的能力：

- Markdown block 解析。
- 代码块高亮。
- Markdown line count 精确估算。

不继承的能力：

- `arbor-tui` 旧组件协议。
- `WidgetFactory` 作为应用侧入口。
- `WidgetNode` 作为应用侧页面拼装类型。
- mount / unmount / `perform()` 暴露给普通应用代码。

### 2. Runtime 必须先补齐 Aster 语义

Aster 不能使用阻塞输入循环。stream token、loading phase、scroll clamp 都需要无输入时继续推进。

Runtime 必须支持：

- 输入读取和 UI runtime 分线程。
- input thread 可以阻塞读取终端输入。
- UI/runtime thread 使用非阻塞 poll 或 timeout poll。
- 无输入帧仍调用 `before_render`。
- `before_events` 可以截获 Esc、Enter、方向键和 PageUp/PageDown。
- 应用可以阻止默认退出。
- 应用可以在 `before_events` 中产出 Action。
- 应用可以在 `before_render` 中轮询外部状态并请求重绘。
- context 可以 `dispatch(action)`、`set_theme(theme)`、`quit()`、`request_render()`。
- context 可以读取当前 screen size。
- resize 后重建 view，并触发 full dirty。

Aster 语义要求：

- stream 中 Esc/Enter 先取消 stream，不能直接退出程序。
- palette 打开时 Esc 先关闭 palette。
- error 状态下 Esc 先 dismiss error。
- 普通空闲状态下 Esc 才允许作为退出键。
- Ctrl-C 和 Ctrl-Q 保留为全局退出键。

线程所有权要求：

- input thread 只负责读取 terminal event。
- input thread 只发送平台无关 `RuntimeInput`。
- input thread 不拥有应用 state。
- input thread 不写 signal。
- input thread 不 render。
- input thread 不写 terminal output。
- UI/runtime thread 拥有 state、Action queue、signal 写入、view、layout、render、diff 和 emit。
- input 和 UI/runtime 之间使用有界 channel。
- channel 满时可以丢弃过量输入或合并可合并导航输入，不能阻塞 UI/runtime thread。
- runtime 退出时必须通知 input thread shutdown。

这个模型是 Aster 替代目标的一部分。Aster 的 stream 输出不能被阻塞式 stdin 读取卡住。

### 3. 输入框必须成为一等组件

Thorn 必须提供应用级 `Input`。

最低能力：

- 单行文本。
- placeholder。
- value 由 State 提供。
- `on_change(String) -> Action`。
- `on_submit(String) -> Action`。
- focused 和 idle 视觉状态。
- loading 状态。
- loading phase。
- 光标。
- Backspace。
- Delete。
- Left / Right。
- Home / End。
- Enter submit。
- password 模式可以后补，但接口要预留。

输入框不能要求 Aster 直接处理字符插入和光标移动。Aster 只处理 Action 和状态。

### 4. 滚动视口必须支持 transcript

Thorn 必须提供 `ScrollArea` 或等价能力。

最低能力：

- 子内容可高于 viewport。
- `scroll_y` 由 Signal 或 State 控制。
- render 时只显示 viewport。
- 支持 content height。
- resize 后 clamp scroll。
- 支持 PageUp / PageDown / Home / End 由应用侧控制。

Scroll 不需要在第一阶段处理鼠标滚轮。鼠标仍是非目标。

### 5. Transcript 先做纯文本版

Thorn 必须提供不依赖 Markdown 的 `Transcript`。

最低能力：

- 多条消息。
- role label。
- label 颜色。
- message body 多行显示。
- 空态文本。
- notice。
- background 填满。
- `scroll_y` 接入。
- `line_count()`。
- `.fill()` 或等价 flex。

第一阶段按纯文本处理 message body。换行按 `\n` 展开。宽字符按 Thorn 的 cell width 规则处理。

Markdown 后续单独写 THEP。Markdown 不阻塞 Aster 迁移。

### 6. Command Palette 必须等价

Aster 当前用 palette 做 slash command 补全。Thorn 必须提供 `FuzzyPanel` 或等价组件。

最低能力：

- items。
- query。
- selected index。
- empty text。
- title。
- placeholder。
- 简单 fuzzy / substring 排序。
- Up / Down 移动。
- Enter submit。
- query change Action。
- submit selection Action。
- selected row 高亮。
- footer 状态行可以后补，但不能影响可用性。

### 7. 组合组件可以薄，但必须稳定

Thorn 应提供 Aster 迁移需要的最小 facade。

最低组件：

- `TextBlock`。
- `Panel`。
- `Col`。
- `Row`。
- `Input`。
- `ScrollArea`。
- `Transcript`。
- `FuzzyPanel`。

`Panel` 必须支持：

- title。
- border。
- padding。
- foreground。
- background。
- fill / flex。

旧 `ComponentProps` 不作为目标。Thorn 可以使用自己的 owned props、builder 或 primitive 组合方式。

### 8. 测试和性能必须跟迁移一起补

Aster 迁移不能只靠人工打开终端。

Thorn 必须提供 headless app 测试入口：

- 创建 state/update/view。
- 安装 `before_events`。
- 安装 `before_render`。
- 发送 key batch。
- 发送 resize。
- tick 一帧。
- 读取 screen。
- 断言文本。
- 断言 light theme 不漏默认黑底。
- 读取 frame stats。

性能统计至少覆盖：

- events。
- update。
- before_render。
- render。
- diff。
- emit / flush。
- dirty regions。
- total frame time。

`arbor-tui` 的 cache shadow 不是第一阶段目标。Aster bench 需要的统计字段必须能映射到 Thorn 的 frame stats。

## Non-goals

本 THEP 不做：

- Markdown 渲染。
- 代码高亮。
- 鼠标。
- IME。
- 复制 `arbor-tui` 旧组件协议。
- 复制 `WidgetFactory` / `WidgetNode` 应用侧 API。
- 兼容 `arbor-tui` crate 名称。
- 兼容旧 facade 的所有方法名。
- layout cache。
- render cache。
- retained widget tree 优化。
- 子 Agent、工具系统、权限系统。

Agent 能力属于 Aster 应用层，不属于 Thorn TUI 基座。

## API Impact

Thorn 后续公开 API 应朝这个形态收敛：

```rust
ThornApp::new(initial_state)
    .theme(Theme::dark())
    .update(update)
    .view(view)
    .before_events(before_events)
    .before_render(before_render)
    .run()
```

`update` 目标形态：

```rust
fn update(state: &mut State, action: Action, ctx: &mut AppContext<Action>)
```

`view` 目标形态：

```rust
fn view(state: &State, ui: &Ui<Action>) -> View<Action>
```

`before_events` 目标形态：

```rust
fn before_events(
    state: &mut State,
    ctx: &mut AppContext<Action>,
    runtime: &mut RuntimeContext,
    inputs: &mut Vec<RuntimeInput>,
) -> bool
```

`before_render` 目标形态：

```rust
fn before_render(
    state: &mut State,
    ctx: &mut AppContext<Action>,
    runtime: &mut RuntimeContext,
) -> bool
```

`AppContext` 至少提供：

- `dispatch(action)`。
- `set_theme(theme)`。
- `quit()`。
- `request_render()`。

`RuntimeContext` 至少提供：

- `screen_size()`。
- `update_signal(signal, value)` 或等价 signal 写入桥。
- `request_render()`。
- `is_running()`。

组件 API 不要求兼容 `arbor-tui` 方法名。迁移时可以改 Aster UI 代码，但不能让 Aster 重新接触旧底层协议。

## Test Requirements

实现本 THEP 时必须补这些测试。

Runtime：

- 无输入时 `before_render` 仍会执行。
- input thread 可以阻塞读输入，但 UI/runtime thread 不会被阻塞。
- input thread 不修改 state、不写 signal、不 render。
- input 到 UI/runtime 之间使用有界 channel。
- channel 满时不会阻塞 UI/runtime thread。
- runtime 退出时会通知 input thread shutdown。
- `before_render` 可以请求重绘。
- `before_events` 可以消费 Esc，阻止默认退出。
- stream 状态下 Esc/Enter 产出取消 Action。
- palette 状态下 Esc 关闭 palette。
- idle 状态下 Esc 可以退出。
- Ctrl-C 和 Ctrl-Q 可以退出。
- resize 后下一帧 full dirty。

Input：

- 输入字符触发 `on_change`。
- Backspace 删除字符。
- Delete 删除光标后字符。
- Left / Right 移动光标。
- Home / End 移动到边界。
- Enter 触发 `on_submit`。
- loading 状态显示 loading frame。
- placeholder 在空值时可见。

Scroll / Transcript：

- transcript 空态可见。
- 多条消息可见。
- notice 可见。
- `scroll_y` 改变 viewport。
- `line_count()` 随消息数量和换行变化。
- 短内容 scroll clamp 为 0。
- resize 后 scroll clamp 生效。

Palette：

- slash draft 可以显示匹配项。
- query 改变后重算 matches。
- Up / Down 改变 selected。
- Enter 提交 selected item。
- empty text 在无匹配时可见。

Theme / render：

- dark theme 首屏可读。
- light theme 不漏默认黑底。
- panel 背景填满。
- transcript 背景填满。
- input 背景填满。

Headless / bench：

- headless app 可以跑 Aster 的标准交互脚本。
- frame stats 包含事件、更新、render、diff 和 emit 阶段。
- benchmark 可以输出每帧 JSONL 或等价结构。
- Aster 迁移后可以保留当前 bench 场景：idle、streaming、scrolling、palette open、model switch、exit。
