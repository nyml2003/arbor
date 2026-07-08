---
id: THEP-0011
title: "输入线程、KeyIntent 与 KeyMap"
status: Accepted
created: 2026-07-09
updated: 2026-07-09
area: runtime
---

# THEP-0011: 输入线程、KeyIntent 与 KeyMap

## 摘要

Thorn 的 runtime 要分离 UI 线程和输入线程。

输入线程只读取 backend events，并把它们转成 runtime input。UI 线程拥有 app state、action queue、Host Tree、layout、paint 和 backend present。

键盘输入不应直接变成业务 action。它先解析成 `KeyIntent`，再由当前 mode、focused host control 和 app binding 解释成具体 `KeyAction`。`KeyAction` 最后才可能产生 application action。

## 决策

### 线程边界

Thorn 使用两个逻辑线程：

```text
Input Thread
  -> read backend events
  -> normalize RuntimeInput
  -> send bounded queue

UI Thread
  -> drain RuntimeInput
  -> resolve KeyMap
  -> produce KeyIntent
  -> resolve KeyIntent
  -> dispatch KeyAction
  -> dispatch App Action
  -> update App state
  -> view / host / layout / paint
  -> present backend output
```

输入线程可以阻塞。UI 线程不能因为等待单个输入事件而长期阻塞。

输入线程只允许：

- 读取 backend input。
- 转换为 `RuntimeInput`。
- 写入 bounded input queue。
- 响应 shutdown signal。

输入线程禁止：

- 修改 `App` state。
- 写 signals。
- 调用 `update`。
- 调用 `view`。
- 执行 layout 或 paint。
- 写 backend output。

UI 线程拥有：

- `App`。
- State。
- Action queue。
- Host Tree。
- Layout Tree。
- Paint primitives。
- Backend present。
- Runtime lifecycle。

### RuntimeInput

`RuntimeInput` 是后端输入的规范形式：

```rust
enum RuntimeInput {
    Key(KeyEvent),
    Resize(Size),
    Tick,
    BackendWake,
    Shutdown,
}
```

`KeyEvent` 不使用 backend 类型：

```rust
struct KeyEvent {
    key: Key,
    modifiers: KeyModifiers,
    kind: KeyEventKind,
}
```

Terminal、Web、Native GUI 后端都必须先转换到 `RuntimeInput`。

### KeyIntent

`KeyIntent` 是抽象键盘意图。

它回答“用户想表达什么”，不直接回答“哪个对象要执行什么操作”。

例子：

```rust
enum KeyIntent {
    RequestSubmit,
    RequestCancel,
    RequestEscape,
    RequestQuit,
    Move(Direction),
    Page(Direction),
    GoHome,
    GoEnd,
    DeleteBackward,
    DeleteForward,
    InsertText(String),
    FocusNext,
    FocusPrev,
    App(&'static str),
}
```

`KeyIntent` 可以随 mode 改变。

例子：

```text
Default mode:
  q -> KeyIntent::RequestQuit

Game mode:
  q -> KeyIntent::App("cast_ultimate")
```

这说明 `q` 不是天然等于退出。`q` 只是物理输入。当前 keymap 和 mode 决定它表达哪个 intent。

`KeyIntent` 的作用：

- 隔离物理按键和具体操作。
- 支持 mode-specific keymap。
- 支持测试直接注入意图。
- 让 app 可以在 intent 层覆盖键位，而不需要处理 backend key event。

### KeyAction

`KeyAction` 是解析后的具体操作。

它回答“当前应该执行什么”。

例子：

```rust
enum KeyAction {
    RuntimeQuit,
    RuntimeCancel,
    FocusNext,
    FocusPrev,
    Control {
        target: HostNodeId,
        action: ControlKeyAction,
    },
    App(AppAction),
}
```

`ControlKeyAction` 表示 host control 的具体操作：

```rust
enum ControlKeyAction {
    Submit,
    Cancel,
    Move(Direction),
    Page(Direction),
    GoHome,
    GoEnd,
    DeleteBackward,
    DeleteForward,
    InsertText(String),
}
```

`KeyAction` 的来源：

- Runtime 保留的全局操作，例如紧急退出。
- Focused host control 对 intent 的解释。
- App 对 intent 的解释。
- Mode 对 intent 的解释。

### KeyMap

`KeyMap` 把 `KeyEvent` 解析成 `KeyIntent`。

KeyMap 支持多层组合：

```text
RuntimeKeyMap
  + PlatformKeyMap
  + ThemeOrPresetKeyMap
  + AppKeyMap
  + FocusedControlKeyMap
```

优先级从高到低：

1. Focused control keymap。
2. App keymap。
3. Mode keymap。
4. Runtime keymap。
5. Platform fallback keymap。

高优先级 keymap 可以：

- 处理输入并停止传播。
- 返回 `KeyIntent`。
- 明确放行给下一层。

KeyMap 组合规则：

- 同一层内禁止静默重复绑定。
- 不同层重复绑定按优先级解析。
- Runtime 必须保留紧急退出通道，例如 `Ctrl-C`。
- App 可以覆盖普通 `Esc`、`Enter`、方向键等非紧急行为。
- Focused control 优先处理文本编辑键。
- Mode keymap 可以改变同一个物理键的 intent。例如默认模式下 `q` 是 `RequestQuit`，游戏模式下 `q` 是 `App("cast_ultimate")`。

### 内置 KeyMap

Thorn 应提供多套内置 KeyMap：

- `DefaultKeyMap`：通用 TUI 默认键位。
- `EmacsTextKeyMap`：常见命令行编辑键。
- `VimNavigationKeyMap`：可选导航预设。
- `ReadOnlyNavigationKeyMap`：只读滚动和移动。
- `TextInputKeyMap`：文本输入控件内部键位。

这些 KeyMap 都只是预设。App 可以组合、禁用或覆盖。

### Action Dispatch

输入到业务 action 的路径是：

```text
KeyEvent
  -> KeyMap
  -> KeyIntent
  -> IntentResolver
  -> KeyAction
  -> App Action
  -> App::update
```

`IntentResolver` 根据当前 context 解释 `KeyIntent`。

Context 包括：

- Active mode。
- Focused host node。
- App key binding。
- Runtime reserved intents。
- Backend capabilities。

`KeyIntent` 可以由 focused host control 解释成 `KeyAction::Control`。例如 `TextInput` 把 `InsertText`、`DeleteBackward`、`MoveLeft` 解释成文本编辑操作，并发出 `DraftChanged` 或 `Submit`。

App 可以把 `KeyIntent` 或 `KeyAction` 映射到业务 action。例如 command palette 把 `MoveDown` 映射为 `SelectNextCommand`。游戏模式把 `App("cast_ultimate")` 映射为 `CastUltimate`。

## 非目标

- 不让 input thread 修改 state。
- 不让 backend key event 直接进入 component 业务逻辑。
- 不把所有键盘行为硬编码进 runtime。
- 不要求第一版支持 mouse、IME 或复杂组合输入。
- 不把 `KeyIntent` 或 `KeyAction` 设计成业务 action 的替代品。

## API 影响

需要公开或内部稳定的类型：

```rust
RuntimeInput
KeyEvent
Key
KeyModifiers
KeyEventKind
KeyIntent
KeyAction
ControlKeyAction
KeyMap
KeyMapLayer
KeyMapResult
IntentResolver
```

App facade 应支持：

```rust
thorn::app(app)
    .keymap(DefaultKeyMap::new())
    .keymap(AppKeyMap::new().bind("ctrl-n", KeyIntent::Move(Direction::Down)))
    .mode_keymap("game", AppKeyMap::new().bind("q", KeyIntent::App("cast_ultimate")))
    .run()
```

Focused host controls 可以声明自己的 keymap：

```text
TextInput -> TextInputKeyMap
ScrollView -> ReadOnlyNavigationKeyMap
```

## 测试要求

测试必须覆盖：

- Input thread 只发送 `RuntimeInput`。
- UI thread 拥有并修改 App state。
- Bounded input queue 满时不会阻塞 UI thread。
- Runtime shutdown 会通知 input thread。
- Backend key event 能转换为 `KeyEvent`。
- `KeyMap` 能把 `KeyEvent` 转成 `KeyIntent`。
- `IntentResolver` 能把 `KeyIntent` 转成 `KeyAction`。
- 多层 KeyMap 按优先级解析。
- 同层重复绑定能被检测出来。
- Focused control keymap 优先于 app keymap。
- App keymap 可以覆盖普通 `Esc`。
- Mode keymap 可以让 `q` 在默认模式下请求退出，在游戏模式下触发大招 intent。
- Runtime 保留紧急退出键。
- `TextInput` 能把文本编辑 `KeyIntent` 解释成 control action。
- App 可以把 `KeyIntent` 或 `KeyAction` 映射为业务 action。
