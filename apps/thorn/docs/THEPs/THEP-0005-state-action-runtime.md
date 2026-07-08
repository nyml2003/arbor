---
id: THEP-0005
title: "App, State 与 Action Runtime"
status: Accepted
created: 2026-07-09
updated: 2026-07-09
area: runtime
---

# THEP-0005: App, State 与 Action Runtime

## 摘要

Thorn 使用显式 app、state 和 action 表达应用行为。

Input events 先变成 normalized runtime input。Runtime input 再变成 application actions。Actions 更新 state。State 产出 UI。

## 决策

应用循环是：

```text
Backend Event
  -> Runtime Input
  -> Action
  -> App::update(State, Action)
  -> App::view(State)
  -> Element Tree
```

Components 不直接修改 business state。它们通过 actions 或 action mappers 表达意图。

State 可以是普通应用状态、signals，或二者组合。Signals 是精确失效的实现机制。Signals 不能成为 components 绕过 application update boundary 的理由。

`App` 是一等结构体。它拥有 state、update、view 和 runtime 配置。

这个设计同时保留两种风格：

- 面向对象：应用有明确的 `App<State, Action>` 对象，runtime 操作这个对象。
- 函数式：`update` 和 `view` 仍然是显式、可测试、无隐式后端副作用的转换。

Runtime 拥有：

- Event normalization。
- Action queue。
- UI thread ownership。
- Input thread boundary。
- KeyMap resolution。
- KeyIntent resolution。
- Update ordering。
- Render scheduling。
- Request-render flag。
- Quit handling。
- Backend lifecycle。

Application 拥有：

- App struct。
- State。
- Action enum。
- Update method 或 update function。
- View method 或 view function。
- 通过明确 ports 或 services 执行业务副作用。

## 非目标

- 不把 widget-local events 当成主要 application protocol。
- 不让 backend events 直接修改 state。
- 不要求每个 app 都使用 signals。
- 不让 action dispatch 触发立即递归 render。

## API 影响

目标形态：

```rust
struct ChatState;

enum AppAction {
    DraftChanged(String),
    Submit,
    Quit,
}

struct ChatApp {
    state: ChatState,
}

impl ThornApp for ChatApp {
    type State = ChatState;
    type Action = AppAction;

    fn state(&self) -> &Self::State {
        &self.state
    }

    fn state_mut(&mut self) -> &mut Self::State {
        &mut self.state
    }

    fn update(&mut self, action: Self::Action, ctx: &mut AppContext<Self::Action>) {
        // 修改 self.state，或请求 runtime 行为
    }

    fn view(&self) -> Element<Self::Action> {
        // 从 self.state 返回 UI
    }
}
```

Facade 也可以提供函数式 builder，把 state、update 和 view 包装成 `App`：

```rust
thorn::app(initial_state)
    .update(update)
    .view(view)
    .run()
```

`AppContext` 应支持：

- `dispatch(action)`
- `dispatch_key_intent(key_intent)`
- `dispatch_key_action(key_action)`
- `request_render()`
- `quit()`
- `set_theme(theme)`
- `backend_capabilities()`

输入线程、UI 线程、`KeyIntent`、`KeyAction` 和 `KeyMap` 的具体协议见 `THEP-0011`。

## 测试要求

测试必须覆盖：

- Runtime input 可以产出 actions。
- 输入线程不能修改 App state。
- UI 线程拥有 App state、layout、paint 和 backend present。
- Actions 按顺序更新 state。
- App struct 拥有 state。
- App method 风格和 builder function 风格能使用同一套 runtime contract。
- Components emit actions，而不是修改 app state。
- Request-render 会调度后续 frame。
- Application action 可以请求 quit。
- Backend event conversion 可以不依赖真实 terminal 测试。
