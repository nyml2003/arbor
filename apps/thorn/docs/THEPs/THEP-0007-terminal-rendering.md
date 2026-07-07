---
id: THEP-0007
title: "终端渲染与适配器"
status: Accepted
created: 2026-07-07
updated: 2026-07-07
area: rendering
---

# THEP-0007: 终端渲染与适配器

## Summary

Thorn 使用 cell grid 作为渲染目标。

Core 只生成屏幕快照和 dirty regions。真实终端能力放在 `thorn-terminal`。这样 core 可以在测试中运行，不依赖 crossterm 和真实终端。

## Decision

核心渲染类型放在 `thorn-core`：

```rust
struct Cell
struct Screen
struct DirtyRegion
fn diff(old: &Screen, new: &Screen) -> Vec<DirtyRegion>
```

`Cell` 包含：

- `char`
- foreground color
- background color
- attrs
- wide-char phantom flag

`Screen` 行为：

- row-major cell storage。
- out-of-bounds read 返回默认 cell。
- out-of-bounds write no-op。
- `write_str` 按显示宽度写入。
- wide char 占两个 cell。
- `fill_rect` 裁切到 screen。
- `blit` 用于组合 child screen 或 fragment。

Diff 行为：

1. 对比 old/new screen。
2. 输出按行的 dirty regions。
3. 相邻或重叠 regions 合并。
4. resize 时按新 screen 全量 dirty。
5. MVP 先做全屏 compose + row diff。

终端适配器放在 `thorn-terminal`：

```rust
trait TerminalBackend {
    fn size(&self) -> Result<(u16, u16)>;
    fn emit(&mut self, regions: &[DirtyRegion], screen: &Screen) -> Result<()>;
    fn enter(&mut self) -> Result<TerminalGuard>;
    fn flush(&mut self) -> Result<()>;
}
```

上屏后回调：

Thorn 可以提供 `after_present` 回调，但它的语义是“本帧 dirty regions 已经交给 backend，并且 flush 已返回”。

它不是硬件显示确认，也不是 vsync。普通终端、PTY、SSH 和 Windows Terminal 都不会给应用一个可靠的“用户已经看到这一帧”的确认。

真实显示器回刷边界见 `THEP-0010`。`thorn-terminal` 不能提供 display-present 回调。

回调时机：

```text
render Screen
  -> diff
  -> backend.emit
  -> backend.flush
  -> after_present(frame)
```

建议接口：

```rust
struct PresentedFrame {
    frame_index: u64,
    dirty_regions: usize,
    dirty_cells: usize,
    emitted_bytes: usize,
}

trait PresentHook {
    fn after_present(&mut self, frame: &PresentedFrame) {}
}
```

约束：

1. `after_present` 不能直接重入 render。
2. `after_present` 不能修改当前 frame 的 screen。
3. 如果需要更新 UI，必须投递 action 或 signal write，让它进入下一帧。
4. `after_present` 失败不能破坏终端恢复。
5. 测试 backend 可以同步触发该回调。

真实 adapter：

- 使用 crossterm。
- 管理 raw mode。
- 管理 alternate screen。
- hide/show cursor。
- 处理 resize。
- 处理 key input。

模拟 adapter：

- 维护内存 screen。
- 记录输出。
- 用于测试。

渲染循环：

```text
input event
  -> runtime dispatch
  -> signal/effect update primitive slots
  -> layout
  -> render Screen
  -> diff old/new
  -> backend.emit
```

MVP 不做局部 subtree render cache。先保证协议正确和测试稳定。

## Non-goals

- 不在 core 中依赖 crossterm。
- 不让组件直接 emit ANSI。
- 不让 primitive node 持有 backend。
- 不在 MVP 中做 terminal capability detection。
- 不在 MVP 中做鼠标 UI。
- 不在 MVP 中做 IME 重度输入。
- 不在 MVP 中做 GPU 或像素绘制。
- 不承诺拿到真实显示器刷新后的确认。

## API Impact

高层运行：

```rust
thorn::app(root)
    .theme(Theme::dark())
    .run()
```

测试运行：

```rust
let mut app = TestApp::new(root).size(80, 24);
app.render();
app.assert_text("Ready");
app.assert_no_default_bg();
```

底层 adapter 对普通用户隐藏。

高级用户可以自定义 backend，但只能实现 terminal port，不能绕过 core 协议。

需要上屏后通知时：

```rust
thorn::app(root)
    .after_present(|frame| {
        // frame 已经 emit + flush。这里只能记录指标或投递下一帧消息。
    })
    .run()
```

具体签名可以调整，但语义必须保持：回调发生在 backend flush 返回之后。

## Test Requirements

必须测试：

- `Screen::new` 创建空白 screen。
- `fill_rect` 裁切正确。
- `write_str` 裁切正确。
- CJK 宽字符占两个 cell。
- wide char phantom 不导致错误 diff。
- identical screen diff 为空。
- 单 cell 改动产生一个 dirty region。
- 同行相邻 dirty region 合并。
- resize 触发全量 dirty。
- simulated backend emit 后内部 screen 更新。
- adapter 不影响 core 测试。
- root render 后没有默认背景泄漏。
- `after_present` 在 emit + flush 后触发。
- `after_present` 看到的 dirty region 数和本帧 diff 一致。
- `after_present` 不能重入当前 render。
