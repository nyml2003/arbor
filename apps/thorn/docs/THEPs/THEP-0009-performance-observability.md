---
id: THEP-0009
title: "性能观测与性能上限"
status: Accepted
created: 2026-07-07
updated: 2026-07-07
area: performance
---

# THEP-0009: 性能观测与性能上限

## Summary

Thorn 必须从 MVP 阶段预留性能观测入口。

性能观测不是日志系统。它是 framework 内部的低开销统计接口，用来回答一帧慢在哪里：响应式、布局、主题解析、渲染、diff，还是终端输出。

MVP 的性能模型是全屏 compose + row diff。它可以很快，但不是最终上限。最终上限来自局部 dirty、render cache、layout cache 和更少的终端输出字节。

## Decision

Thorn 预留一个 no-op by default 的性能接口：

```rust
trait PerfSink {
    fn on_frame(&mut self, stats: &FrameStats) {}
    fn on_effect(&mut self, stats: &EffectStats) {}
    fn after_present(&mut self, frame: &PresentedFrame) {}
}
```

默认 runtime 使用 no-op sink。没有用户安装 sink 时，不分配日志字符串，不写文件，不输出 stderr。

MVP 必须定义这些统计结构。字段可以增减，但语义不能丢。

```rust
struct FrameStats {
    frame_index: u64,
    total_us: u64,
    reactive_us: u64,
    layout_us: u64,
    theme_us: u64,
    render_us: u64,
    diff_us: u64,
    emit_us: u64,
    node_count: usize,
    effect_reruns: usize,
    dirty_slots: usize,
    screen_cells: usize,
    dirty_regions: usize,
    dirty_cells: usize,
    emitted_bytes: usize,
}

struct EffectStats {
    effect_id: u64,
    rerun_us: u64,
    dependency_count: usize,
    cleanup_count: usize,
}

struct PresentedFrame {
    frame_index: u64,
    dirty_regions: usize,
    dirty_cells: usize,
    emitted_bytes: usize,
    emit_us: u64,
}
```

统计边界：

| 字段 | 含义 |
| --- | --- |
| `reactive_us` | signal 写入到 effect rerun 完成 |
| `layout_us` | primitive tree 到 layout tree |
| `theme_us` | token 到 concrete color/style |
| `render_us` | layout tree 到 new screen |
| `diff_us` | old/new screen 到 dirty regions |
| `emit_us` | backend 输出 dirty regions |
| `dirty_slots` | 被 effect 更新的 primitive slot 数 |
| `dirty_regions` | diff 后的连续行区间数量 |
| `dirty_cells` | dirty regions 覆盖的 cell 数 |
| `emitted_bytes` | backend 实际写出的字节数 |

MVP 不要求真实 terminal runtime，但 `emit_us` 和 `emitted_bytes` 仍保留。内存 backend 可以把它们填成 0 或模拟值。

性能 sink 入口：

```rust
thorn::app(root)
    .perf_sink(MyPerfSink::new())
    .run()
```

测试入口：

```rust
let mut app = TestApp::new(counter).with_perf();
app.render(80, 24);
let stats = app.last_frame_stats();
```

低开销规则：

1. 默认 no-op sink 不分配。
2. 计时只在 frame 边界和阶段边界做。
3. effect 级统计默认可关闭。
4. hot path 不格式化字符串。
5. hot path 不构造 JSON。
6. 统计结构使用数字和 enum，不使用动态 map。
7. `PerfSink` 调用发生在阶段结束后，不能重入 UI 更新。
8. `PerfSink` 失败不能影响渲染。需要 fallible sink 时，错误由 sink 自己保存。

`after_present` 语义：

- 它在 backend `emit()` 和 `flush()` 成功返回后调用。
- 它表示本进程已经把本帧写给终端 backend。
- 它不表示终端已经完成物理显示。
- 它不能修改当前 frame。
- 它可以记录指标，或投递下一帧 action。

真实显示器回刷不属于 terminal perf sink。需要 display-present 时，必须走 `THEP-0010` 定义的 native host 边界。

## Performance Ceiling

### MVP 上限

MVP 路径：

```text
changed signals
  -> changed effects
  -> full layout
  -> full screen render
  -> full screen diff
  -> dirty region emit
```

MVP 的复杂度：

| 阶段 | 复杂度 |
| --- | --- |
| reactive | `O(changed_effects + changed_deps)` |
| layout | `O(node_count)` |
| render | `O(screen_cells + node_count)` |
| diff | `O(screen_cells)` |
| emit | `O(dirty_regions + emitted_bytes)` |

这意味着 MVP 即使只改一个字符，也会重新 compose 整个 screen，再 diff 出一个小 dirty region。

这个上限对常见 TUI 仍然够用。原因是终端 screen 很小：

- 80x24 = 1,920 cells。
- 120x40 = 4,800 cells。
- 200x60 = 12,000 cells。

在这些尺寸下，全屏内存 compose 和 diff 通常不是瓶颈。真实瓶颈更可能是终端输出、flush、过多分散 dirty regions，以及组件层产生过多字符串分配。

MVP 不应追求复杂 cache。MVP 要先把统计打准。

### 优化后上限

后续加入 dirty subtree、render cache 和 layout cache 后，目标复杂度应变成：

```text
O(changed_effects + dirty_subtree_nodes + dirty_cells + emitted_bytes)
```

优化后的理想路径：

- signal 只触发依赖 effect。
- effect 只更新对应 primitive slot。
- layout 只重算受 layout dirty 影响的子树。
- render 只重画 dirty rect 或 dirty fragment。
- diff 只比较 old/new dirty rect。
- backend 只写 dirty cells。

这套设计的理论上限接近“改几个 cell 就输出几个 cell”。但只有在后续实现局部 layout/render/diff 后才成立。MVP 不能宣称达到这个上限。

### 主要瓶颈

需要优先盯这些指标：

- `effect_reruns` 过高：说明 signal fanout 或依赖追踪不准。
- `layout_us` 过高：说明 layout 全树重算或节点过多。
- `render_us` 过高：说明 screen compose 或文本处理太重。
- `diff_us` 过高：说明 screen 太大，或后续需要 dirty rect diff。
- `dirty_regions` 过高：说明变化太分散，terminal emit 会变慢。
- `emitted_bytes` 过高：说明样式重置或 region 合并不够好。
- `theme_us` 过高：说明 token 解析没有缓存或样式太碎。

## Non-goals

- 不在 MVP 中做复杂 benchmark runner。
- 不在 MVP 中做 flamegraph 集成。
- 不在 MVP 中写 JSONL 性能日志。
- 不在 MVP 中做 render cache。
- 不在 MVP 中做 layout cache。
- 不在 MVP 中做 dirty subtree diff。
- 不让性能统计改变渲染结果。
- 不把性能 sink 变成业务日志系统。

## API Impact

公共 API 需要预留：

```rust
trait PerfSink {
    fn on_frame(&mut self, stats: &FrameStats) {}
    fn on_effect(&mut self, stats: &EffectStats) {}
    fn after_present(&mut self, frame: &PresentedFrame) {}
}

struct NoopPerfSink;
```

测试 API 需要暴露：

```rust
app.last_frame_stats()
app.take_frame_stats()
```

如果后续支持 bench 模式，入口可以是：

```rust
thorn::bench(root)
    .scenario("counter")
    .frames(1000)
    .run()
```

bench API 不是 MVP 必需项。

## Test Requirements

MVP 必须测试：

- no-op perf sink 不影响 render。
- frame stats 记录 `node_count`。
- frame stats 记录 `screen_cells`。
- dynamic text 更新后 `effect_reruns >= 1`。
- 单字符变化后 `dirty_cells >= 1`。
- `dirty_regions` 与 diff 输出一致。
- `last_frame_stats()` 返回最近一帧。
- perf sink 回调不能触发 UI 重入。
- `after_present` 在 emit + flush 后触发。
- `after_present` 不承诺真实显示器刷新完成。

后续性能测试应覆盖：

- counter 单 slot 更新。
- theme switch 全局 render。
- 100 item list 更新 1 item。
- 200x60 screen diff。
- scattered dirty regions。
- contiguous dirty region 合并。
