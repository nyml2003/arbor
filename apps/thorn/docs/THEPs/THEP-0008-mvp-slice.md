---
id: THEP-0008
title: "MVP 后下一阶段计划"
status: Draft
created: 2026-07-07
updated: 2026-07-08
area: planning
---

# THEP-0008: MVP 后下一阶段计划

## Summary

Thorn 下一阶段不继续补 `Button` 或鼠标事件。

下一阶段先做键盘 Action Runtime。目标是把 Thorn 从“能渲染静态 demo”推进到“能写一个键盘驱动的小型 TUI 应用”。

依据：

- Aster 已经证明 `KeyEvent -> Action -> AppState -> view` 适合真实 TUI。
- arbor-tui 的终态文档已经证明 runtime step、adapter 分层、模拟输入输出测试是正确边界。
- Thorn 当前的 `Button/on_press` 方向会把框架拉回鼠标和 widget 事件模型，不符合当前目标。

协议细节见 `THEP-0010: 键盘事件与 Action Runtime`。

## Decision

下一阶段按四个增量推进。

### 1. 写清并实现键盘输入协议

目标：

- `thorn-core` 定义平台无关 `RuntimeInput`、`KeyEvent`、`Key`、modifier 和 key kind。
- `thorn-terminal` 把 crossterm keyboard/resize event 转成 core input。
- runtime 不启用 mouse capture。
- mouse event 不进入 core。
- `Escape`、`Ctrl-C` 和 `Ctrl-Q` 作为 runtime 默认退出键，普通 `q` 留给 keymap 绑定应用动作。
- `Enter` 在没有应用 handler 时是 no-op。

完成标准：

- 单测覆盖 key conversion。
- 单测覆盖 resize conversion。
- 单测覆盖 `Enter` no-op。
- 单测或 adapter 测试证明 runtime 不启用 mouse capture。

### 2. 增加状态化 Action Runtime

目标：

- 增加状态化应用入口。
- 应用提供 state、update、view。
- runtime 先处理输入批次，再执行 Action，再 render。
- `before_events` 可以把输入批次转成 Action。
- `before_render` 可以在每帧渲染前推进非输入状态。

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

完成标准：

- 一个测试应用能通过键盘输入改变状态。
- `update` 修改状态后，view 使用新状态渲染。
- `before_render` 可以触发下一帧变化。
- runtime 退出时恢复 terminal 状态。

### 3. 改造测试 harness

目标：

- 测试能脚本化发送键盘事件。
- 测试能脚本化 resize。
- 测试能读取最后一帧 screen 和 dirty 信息。
- 测试不依赖真实终端。

完成标准：

- `send_key(Key::Char('x'))` 可用。
- `resize(width, height)` 可用。
- resize 后下一帧 full dirty。
- Action Runtime 的核心路径能在内存测试里跑通。

### 4. Demo 变成真实验收入口

目标：

- `counter_live` 保留真实终端 smoke demo。
- 新增一个键盘驱动 demo，展示 Action Runtime。
- README 写清 demo 的启动方式、退出键和不支持鼠标。

建议 demo：

- `keyboard_counter`：`+`/`-` 改变计数，`q`/`Esc` 退出。
- 后续再做 Aster 风格 input/palette demo。

完成标准：

```powershell
cargo run --manifest-path apps/thorn/Cargo.toml -p thorn --example keyboard_counter
cargo run --manifest-path apps/thorn/Cargo.toml -p thorn --example counter_live
cargo run --manifest-path apps/thorn/Cargo.toml -p thorn --example counter_demo
```

三个命令都能运行。`keyboard_counter` 是交互验收入口。`counter_demo` 是快速 screen 输出入口。

## Non-goals

下一阶段不做这些能力：

- 鼠标。
- `Button`。
- hover/click/drag。
- 完整 focus manager。
- Tab/Shift+Tab focus navigation。
- Input 文本编辑。
- IME。
- `Show`。
- `For`。
- `Memo`。
- async effect。
- render cache。
- layout cache。
- dirty subtree diff。
- 自定义 primitive node type。
- 自绘 cell API。
- 完整性能统计系统。

这些能力后续单独写 THEP。

## API Impact

允许新增这些入口：

```rust
ThornApp::new(initial_state)
    .theme(Theme::light())
    .update(update)
    .view(view)
    .run()
```

允许保留静态 smoke 入口：

```rust
thorn::app(root)
    .theme(Theme::light())
    .run()
```

`TestApp` 或新 `TestRuntime` 应提供键盘脚本能力：

```rust
app.send_key(Key::Char('+'));
app.send_key(Key::Enter);
app.resize(80, 24);
app.render_frame();
```

暂不增加：

- `button(...)`。
- `on_press(...)`。
- `view.press_first_focusable()`。
- mouse event API。

## Test Requirements

下一阶段必须补这些测试：

- core key event 类型不依赖 crossterm。
- terminal adapter 能转换 key event。
- terminal adapter 能转换 resize event。
- runtime 不启用 mouse capture。
- `Escape`、`Ctrl-C` 和 `Ctrl-Q` 默认退出，普通 `q` 不被 runtime 截获。
- `Enter` 默认 no-op。
- `before_events` 能产出 Action。
- `update` 能修改 state。
- `before_render` 能推进 state。
- view 使用最新 state 渲染。
- resize 后下一帧 full dirty。
- terminal backend `enter()` 返回 guard。
- guard drop 后尝试恢复 terminal 状态。
- memory backend 记录 `emit()` 和 `flush()` 调用。
- `keyboard_counter` 至少通过 `cargo check`。
- `counter_live` 至少通过 `cargo check`。

验收命令：

```powershell
cargo fmt --all
cargo check --manifest-path apps/thorn/Cargo.toml --workspace
cargo test --manifest-path apps/thorn/Cargo.toml --workspace
cargo check --manifest-path apps/thorn/Cargo.toml -p thorn --example keyboard_counter
cargo check --manifest-path apps/thorn/Cargo.toml -p thorn --example counter_live
```
