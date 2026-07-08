---
id: THEP-0012
title: "MVP Slice"
status: Accepted
created: 2026-07-09
updated: 2026-07-09
area: planning
---

# THEP-0012: MVP Slice

## 摘要

Thorn 的第一个 MVP 只验证一条最小闭环。

MVP 目标不是做完整 TUI 框架。目标是证明 `App -> Element -> Host Tree -> Layout -> Paint -> Cell Grid -> Headless snapshot` 这条管线可实现、可测试、可扩展。

MVP 完成后，Thorn 才进入 real terminal、TextInput、ScrollView、KeyMap presets 和性能优化。

## 决策

### MVP 场景

第一个 E2E 应用是 `CounterApp`。

行为：

- 初始 state：`count = 0`，`running = true`。
- `+` 对应 `KeyIntent::App("increment")`。
- `-` 对应 `KeyIntent::App("decrement")`。
- `q` 在 default mode 下对应 `KeyIntent::RequestQuit`。
- `Ctrl-C` 对应 runtime reserved quit。
- `increment` 让 `count += 1`。
- `decrement` 让 `count -= 1`。
- `quit` 让 runtime 停止。

输出：

```text
Counter
count: 0
+/- change, q quit
```

MVP 只使用 headless backend。它不打开真实 terminal。

### MVP 管线

MVP 必须跑通：

```text
CounterApp
  -> RuntimeInput::Key
  -> KeyMap
  -> KeyIntent
  -> KeyAction
  -> AppAction
  -> App::update
  -> App::view
  -> Element Tree
  -> Host Tree
  -> Layout Tree
  -> Paint Primitive
  -> Cell Grid
  -> Headless snapshot
```

### MVP crate

MVP 创建三个 crate：

```text
crates/thorn-core
crates/thorn-headless
crates/thorn
```

暂不创建 `thorn-terminal`。真实 terminal backend 等 headless MVP 通过后再加。

职责：

| Crate | 职责 |
| --- | --- |
| `thorn-core` | App contract、Element、Host Tree、Layout、Paint、RuntimeInput、KeyIntent、KeyAction、Cell、Screen |
| `thorn-headless` | Memory backend、snapshot、test helpers |
| `thorn` | facade、prelude、example API |

### MVP 类型

MVP 只定义这些必要类型。

App：

```rust
trait ThornApp {
    type Action;

    fn update(&mut self, action: Self::Action, ctx: &mut AppContext<Self::Action>);
    fn view(&self) -> Element<Self::Action>;
}
```

Runtime：

```rust
struct AppContext<Action> {
    // dispatch queue, request_render, quit
}

enum RuntimeInput {
    Key(KeyEvent),
    Resize(Size),
    Tick,
    Shutdown,
}
```

Input：

```rust
struct KeyEvent {
    key: Key,
    modifiers: KeyModifiers,
    kind: KeyEventKind,
}

enum KeyIntent {
    RequestQuit,
    App(&'static str),
}

enum KeyAction<Action> {
    RuntimeQuit,
    App(Action),
}
```

Element：

```rust
enum Element<Action> {
    Text(TextElement),
    Column(Vec<Element<Action>>),
}
```

Host Tree：

```rust
enum HostKind {
    Text,
    View,
}

struct HostNode<Action> {
    id: HostNodeId,
    kind: HostKind,
    children: Vec<HostNode<Action>>,
}
```

Layout：

```rust
struct Size { width: u16, height: u16 }
struct Rect { x: u16, y: u16, width: u16, height: u16 }
struct LayoutNode { host_id: HostNodeId, rect: Rect }
```

Paint：

```rust
enum PaintPrimitive {
    TextRun { x: u16, y: u16, text: String },
}
```

Cell：

```rust
struct Cell {
    ch: char,
}

struct Screen {
    size: Size,
    cells: Vec<Cell>,
}
```

MVP 暂不实现 color、style、border、cursor、clip、theme。

### MVP API

Facade 目标：

```rust
use thorn::prelude::*;

struct CounterApp {
    count: i32,
}

enum CounterAction {
    Increment,
    Decrement,
}

impl ThornApp for CounterApp {
    type Action = CounterAction;

    fn update(&mut self, action: Self::Action, ctx: &mut AppContext<Self::Action>) {
        match action {
            CounterAction::Increment => self.count += 1,
            CounterAction::Decrement => self.count -= 1,
        }
    }

    fn view(&self) -> Element<Self::Action> {
        column((
            text("Counter"),
            text(format!("count: {}", self.count)),
            text("+/- change, q quit"),
        ))
    }
}

let mut app = TestRuntime::new(CounterApp { count: 0 }).size(40, 8);
app.render_frame();
app.assert_text("count: 0");
app.send_key('+');
app.render_frame();
app.assert_text("count: 1");
```

### KeyMap MVP

MVP 只实现一层 app keymap 和一层 runtime reserved keymap。

默认绑定：

| Key | Intent |
| --- | --- |
| `+` | `KeyIntent::App("increment")` |
| `-` | `KeyIntent::App("decrement")` |
| `q` | `KeyIntent::RequestQuit` |
| `Ctrl-C` | `KeyIntent::RequestQuit` |

MVP 不实现 mode keymap、focused control keymap、preset keymap。

但类型命名必须保留 `KeyIntent` 和 `KeyAction`，避免后续重构。

### Headless Snapshot

MVP snapshot 使用纯文本断言。

要求：

- `screen.to_plain_text()` 返回所有行。
- `assert_text("count: 1")` 检查文本存在。
- `assert_line(0, "Counter")` 检查指定行。
- `assert_not_text("count: 0")` 检查旧文本消失。

## 非目标

MVP 不做：

- Real terminal backend。
- Input thread。
- 多线程。
- TextInput。
- ScrollView。
- Panel。
- Border。
- Theme。
- Color。
- Cursor。
- Clip。
- Dirty diff。
- Layout cache。
- Paint cache。
- 多套 KeyMap preset。
- Mode keymap。
- Focus manager。
- Mouse。
- IME。
- Web backend。
- Native GUI backend。

这些能力必须在 MVP 通过后单独推进。

## API 影响

MVP 必须让 `thorn-core` 的 public API 尽量小。

优先公开：

- `ThornApp`
- `AppContext`
- `Element`
- `RuntimeInput`
- `KeyEvent`
- `KeyIntent`
- `KeyAction`
- `Size`
- `Rect`
- `Screen`

`thorn` facade 公开：

- `prelude`
- `text`
- `column`

`thorn-headless` 公开：

- `TestRuntime`
- `ScreenSnapshot`
- assertion helpers

## 测试要求

MVP 必须包含这些测试。

Core pipeline：

- `element_text_lowers_to_host_text`
- `column_lowers_to_host_view_with_children`
- `host_tree_assigns_stable_ids`
- `column_layout_stacks_children_vertically`
- `text_paint_produces_text_run`
- `paint_text_run_writes_cells`
- `screen_plain_text_contains_written_text`

Runtime：

- `counter_initial_render_shows_zero`
- `plus_key_increments_counter`
- `minus_key_decrements_counter`
- `q_key_requests_quit`
- `ctrl_c_requests_quit`
- `runtime_does_not_render_after_quit`

Key input：

- `keymap_maps_plus_to_increment_intent`
- `keymap_maps_q_to_quit_intent`
- `intent_resolver_maps_increment_to_app_action`
- `intent_resolver_maps_quit_to_runtime_quit`

Headless：

- `headless_assert_text_passes_when_text_exists`
- `headless_assert_not_text_passes_after_update`

验收命令：

```powershell
cargo test --manifest-path apps/thorn/Cargo.toml --workspace
cargo check --manifest-path apps/thorn/Cargo.toml --workspace
```

MVP 实现开始前，应先让 `Cargo.toml` 恢复 workspace members：

```toml
members = [
    "crates/thorn-core",
    "crates/thorn-headless",
    "crates/thorn",
]
```
