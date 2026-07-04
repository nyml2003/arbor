---
id: TEP-0006
title: "工程化——crate 拆分、构建、测试与交付"
status: Draft
created: 2026-07-04
updated: 2026-07-04
author: nyml
area: ecosystem
affects: []
related: [TEP-0001, TEP-0002, TEP-0003, TEP-0004, TEP-0005]
---
# TEP-0006: 工程化——crate 拆分、构建、测试与交付

## 摘要

定义 Arbor TUI 框架的工程基础设施：Rust workspace 结构、crate 职责划分、feature flag 分层、交叉编译 CI、测试体系、dogfooding 策略。

## Crate 拆分

```
apps/tui-framework/
├── Cargo.toml                 # workspace root
├── crates/
│   ├── arbor-tui-core/        # domain 层——零外部依赖
│   │   ├── Cargo.toml         # [dependencies] unicode-width
│   │   └── src/
│   │       ├── cell.rs        # Cell, AnsiColor, PaletteColor, Rgb, Attrs
│   │       ├── screen.rs      # VirtualScreen
│   │       ├── diff.rs        # diff()
│   │       ├── text.rs        # measure_width, expand_tabs, truncate, wrap_lines
│   │       ├── layout.rs      # Rect, Size, SizeConstraint, LayoutProps, SizeCalc
│   │       ├── layout_engine.rs  # measure_tree(), layout_tree()
│   │       ├── signal.rs      # Signal<T>, ReadSignal<T>
│   │       ├── dirty.rs       # DirtyTracker
│   │       ├── widget.rs      # Widget trait, WidgetNode, WidgetId, Lifecycle
│   │       ├── focus.rs       # FocusManager
│   │       ├── input.rs       # Key, KeyEvent, Modifiers, InputReader trait, KeyHandleResult
│   │       ├── theme.rs       # Theme, ColorPalette, Style
│   │       ├── events.rs      # FrameworkEvent, EventBus, EventSubscriber trait
│   │       └── backend.rs     # TerminalBackend trait, TerminalGuard trait
│   │
│   ├── arbor-tui-backend/     # infra 层——crossterm 适配器
│   │   ├── Cargo.toml         # [dependencies] arbor-tui-core, crossterm
│   │   └── src/
│   │       ├── lib.rs         # pub use
│   │       ├── crossterm_backend.rs  # CrosstermBackend
│   │       ├── stdin_reader.rs       # StdinReader (crossterm event::read)
│   │       └── simulated_backend.rs  # SimulatedBackend (测试用)
│   │
│   └── arbor-tui/             # app 层 + 公开 API re-export
│       ├── Cargo.toml         # [dependencies] arbor-tui-core, arbor-tui-backend
│       └── src/
│           ├── lib.rs         # pub use arbor_tui_core::* + arbor_tui_backend::*
│           ├── app.rs         # App, AppConfig
│           ├── render_loop.rs # render_if_dirty(), MIN_FRAME_INTERVAL
│           ├── event_loop.rs  # 主循环：recv_timeout → merge → dispatch → render
│           ├── signal_manager.rs  # SignalManager（SIGINT/SIGTSTP/SIGWINCH）
│           ├── terminal.rs    # TerminalGuard
│           └── subprocess.rs  # run_subprocess()
│
├── examples/
│   └── counter.rs             # 最小 demo：计数器 + 按键
│
└── tests/
    └── integration.rs         # 端到端集成测试
```

### 依赖方向

```
arbor-tui (app + re-export)
  ├── arbor-tui-backend (infra, crossterm)
  │     └── arbor-tui-core (domain)
  └── arbor-tui-core (domain)
```

`arbor-tui-core` 零外部依赖（除 `unicode-width`）。`arbor-tui-backend` 引入 `crossterm`。`arbor-tui` 引入前两者，提供 `App::run()` 入口。

### Feature flag 分层

```toml
# arbor-tui-backend/Cargo.toml
[features]
default = ["crossterm"]
crossterm = ["dep:crossterm"]     # 生产后端
simulated = []                     # 测试用模拟后端（始终编译）

# arbor-tui-core/Cargo.toml
[features]
default = []
profile = []  # 编译 EventBus，默认关闭。release 不开启时所有 emit 优化为零开销
```

默认编译 crossterm。测试编译可用 `--no-default-features` 只开 simulated，减少 CI 编译时间。`profile` feature 控制事件总线是否编译——不带此 feature 时 `EventBus::emit()` 内联为空操作。未来 Windows XP legacy 后端作为独立 feature `winapi`。

## 二进制体积控制

Release 静态链接目标 ≤ 8MB。控制策略：

- LTO = fat（跨 crate 内联 + 死代码消除）
- `opt-level = "z"`（体积优先）
- `strip = "symbols"`
- `codegen-units = 1`（最大化内联，代价编译时间）
- panic = "abort"（去掉 unwind 表的体积开销）

```toml
# Cargo.toml (workspace)
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = "symbols"
panic = "abort"
```

不引入 `log` / `env_logger` / `clap` 等大依赖。crossterm 是唯一 >100KB 的外部依赖。

## 交叉编译

四个目标平台：

| 目标                  | triple                        | 用途               |
| --------------------- | ----------------------------- | ------------------ |
| x86_64 Linux (glibc)  | `x86_64-unknown-linux-gnu`  | 服务器、CI、Docker |
| aarch64 Linux (glibc) | `aarch64-unknown-linux-gnu` | ARM 服务器、树莓派 |
| x86_64 Windows        | `x86_64-pc-windows-msvc`    | 消费者 Windows     |
| aarch64 macOS         | `aarch64-apple-darwin`      | Apple Silicon Mac  |

CI 使用 `cross` 工具链（Docker 内交叉编译，无需物理机）。Windows 目标用 GitHub Actions `windows-latest` runner 原生编译。

## 测试体系

### 分层

> 单元测试覆盖率要达到80%

```
domain 单元测试（arbor-tui-core/）
  ├── cell::tests          # Cell Default/PartialEq
  ├── diff::tests          # diff 尺寸一致/缩小/放大/零散脏区
  ├── text::tests          # tab展开/宽度/截断/换行
  ├── layout::tests        # SizeCalc/SizeConstraint
  ├── layout_engine::tests # 固定尺寸/flex弹性/Justify/Align
  ├── signal::tests        # Signal set/get/subscribe/unsubscribe
  ├── dirty::tests         # DirtyTracker mark/drain
  ├── focus::tests         # tab排序/next/prev/边界
  └── input::tests         # 事件合并节流

infra 集成测试（arbor-tui-backend/）
  ├── simulated_backend::tests  # emit 输出校验
  └── stdin_reader::tests       # crossterm→KeyEvent 映射

端到端测试（tests/integration.rs）
  └── 完整链路：SimulatedInput → 事件分发 → signal.set →
      布局 → render → diff → SimulatedBackend.output 校验
```

### 端到端测试示例

```rust
#[test]
fn counter_app_keypress_increments_display() {
    let mut input = SimulatedInput::new();
    let mut backend = SimulatedBackend::new(80, 24);
    let mut app = App::new(counter_widget(), &mut backend);

    input.enqueue(KeyEvent::char('a'));  // 模拟按键
    app.process_events(&mut input);
    app.render(&mut backend);

    // 校验 backend.output 中的 ANSI 序列包含预期文本
    assert!(backend.output_contains("Count: 1"));
}
```

模拟后端 `SimulatedBackend` 不写真实终端，只在内存中记录 ANSI 序列。`SimulatedInput` 回放预设按键序列。端到端测试不需要真实终端。

### CI 矩阵（先不做）

```yaml
# .github/workflows/ci.yml
jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --workspace
      - run: cargo test --workspace --no-default-features  # 仅 simulated

  cross-compile:
    strategy:
      matrix:
        target: [x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu,
                 x86_64-pc-windows-msvc, aarch64-apple-darwin]
    steps:
      - uses: actions/checkout@v4
      - uses: rust-cross/cross@main
        with:
          target: ${{ matrix.target }}
          command: build
          args: --release

  memory-stress:
    steps:
      - run: |
          # 24h 运行：持续滚动 100k 行日志 + 频繁切换组件
          # 每分钟采样 RSS，验证波动 ≤ 10%
```

## 系统事件总线

框架内置一个轻量事件总线，各层在关键节点 emit 事件，任意 subscriber 订阅。domain 层定义 trait，app 层持有 bus 实例并注入到渲染管道。

### 事件类型

```rust
// domain/events.rs

pub enum FrameworkEvent {
    // ── 渲染管道 ──
    FrameStart { seq: u64 },
    LayoutStart { widget_count: usize },
    LayoutEnd { duration_us: u64 },
    DiffStart { screen_size: (u16, u16) },
    DiffEnd { duration_us: u64, dirty_regions: usize, dirty_cells: usize },
    EmitStart { region_count: usize },
    EmitEnd { duration_us: u64 },
    FrameEnd(FrameStats),

    // ── 输入 ──
    InputReceived(KeyEvent),
    InputMerged { before: usize, after: usize },
    FocusChanged { from: Option<WidgetId>, to: Option<WidgetId> },

    // ── Signal ──
    SignalSet { widget_id: WidgetId, generation: u64 },

    // ── 生命周期 ──
    WidgetMounted(WidgetId),
    WidgetUnmounted(WidgetId),
    AppStart,
    AppQuit,

    // ── 错误 ──
    Warning { widget_id: Option<WidgetId>, message: String },
    Error { message: String },
}

pub struct FrameStats {
    pub seq: u64,
    pub layout_us: u64,
    pub render_us: u64,
    pub diff_us: u64,
    pub emit_us: u64,
    pub total_us: u64,
    pub dirty_widgets: usize,
    pub dirty_cells: usize,
}

/// 事件订阅者——实现此 trait 即可接入事件总线
pub trait EventSubscriber: Send + Sync {
    fn on_event(&self, event: &FrameworkEvent);
}
```

### EventBus

```rust
// domain/events.rs

pub struct EventBus {
    subscribers: Vec<Box<dyn EventSubscriber>>,
    enabled: bool,   // release 可关闭，零开销
}

impl EventBus {
    pub fn new() -> Self { Self { subscribers: Vec::new(), enabled: true } }
    pub fn enabled(mut self, enabled: bool) -> Self { self.enabled = enabled; self }
    pub fn subscribe(&mut self, sub: Box<dyn EventSubscriber>) { self.subscribers.push(sub); }
    pub fn emit(&self, event: FrameworkEvent) {
        if !self.enabled { return; }
        for sub in &self.subscribers { sub.on_event(&event); }
    }
}
```

### 内置 Subscriber

框架提供两个内置 subscriber，用户可自定义：

```rust
// app/profiler.rs

/// 性能统计 subscriber——每 N 帧输出摘要到 stderr
pub struct ProfilerSubscriber {
    frame_stats: Vec<FrameStats>,
    log_interval: u64,  // 每 N 帧输出一次
}

impl EventSubscriber for ProfilerSubscriber {
    fn on_event(&self, event: &FrameworkEvent) {
        match event {
            FrameworkEvent::FrameEnd(stats) => { /* 累积统计 */ }
            _ => {}
        }
    }
}

// infra/simulated_backend.rs

/// 测试断言 subscriber——端到端测试捕获事件做断言
pub struct AssertionSubscriber {
    events: Mutex<Vec<FrameworkEvent>>,
}
```

### 使用方式

```rust
let mut bus = EventBus::new()
    .enabled(std::env::var("ARBOR_TUI_PROFILE").is_ok());

// 附加内置 profiler
bus.subscribe(Box::new(ProfilerSubscriber::new()));

// 测试时附加断言 subscriber
bus.subscribe(Box::new(AssertionSubscriber::new()));

let mut app = App::new(root_widget, backend, bus);
app.run();
```

渲染管道中 emit：

```rust
bus.emit(FrameworkEvent::LayoutStart { widget_count: tree.len() });
measure_tree(root, screen_size);
bus.emit(FrameworkEvent::LayoutEnd { duration_us: elapsed });
```

### 编译期剔除

Release 构建可通过 feature flag 完全剔除事件总线——所有 `bus.emit()` 调用被编译器优化掉：

- 带 `profile` feature：EventBus 编译，默认 disabled，环境变量开启
- 不带 `profile` feature：EventBus 退化为空壳，所有 emit 内联为空操作，零运行时开销

```toml
[features]
default = []
profile = []  # 开启事件总线
```

等价于 `tracing` crate 的编译期剔除策略，但不引入外部依赖。

## 环境变量规范

优先级（从高到低）：

1. 启动参数（AppConfig 显式传入）
2. 环境变量
3. 默认值

| 变量                  | 默认     | 说明                                  |
| --------------------- | -------- | ------------------------------------- |
| `ARBOR_TUI_THEME`   | `dark` | `light` / `dark`                  |
| `NO_COLOR`          | 未设置   | `1` 时禁用所有颜色，保留文本属性    |
| `ARBOR_TUI_PROFILE` | 未设置   | `1` 时启用 EventBus + ProfilerSubscriber，每 60 帧输出统计到 stderr |
| `ARBOR_TUI_FPS`     | `60`   | 帧率上限，可为`30`/`60`/`120`   |

`NO_COLOR=1` 优先级高于 `ARBOR_TUI_THEME`——无颜色模式下主题配色无意义。

## Dogfooding 策略

第一个 dogfooding 目标：`apps/netmon/`。用 Arbor TUI 框架重写网络监控面板。验证清单：

- [ ] 实时数据刷新（每秒 signal 更新 → 60fps 节流正常）
- [ ] 列表虚拟滚动（100+ 网络连接，只渲染可见行）
- [ ] Tab 切换（概览/详情/配置三个面板）
- [ ] 键盘导航（j/k 移动选中行，Tab 切换面板）
- [ ] SIGWINCH resize 后布局不崩
- [ ] 消费者 Windows 二进制双击运行，无额外配置

## API 兼容策略

v1 不承诺 API 稳定。每个版本在 CHANGELOG 中记录破坏性变更。Semver: `0.x.y`——minor version bump 允许破坏性变更。

TEP 变更流程：新 TEP 提案 → Review → Accepted → 实现 → TEP 状态更新为 Implemented。对已有 TEP 的修改通过新的补充 TEP（如 `TEP-0003-补充-百分比尺寸`）。

## 已决议

- [X] Crate 拆分：3 个 crate（core / backend / app），依赖方向 `app → backend → core`
- [X] Feature flag：默认 `crossterm`，可选 `simulated`。未来 `winapi`。core 无 feature flag
- [X] 二进制体积：LTO + opt=z + strip + panic=abort ≤ 8MB
- [X] 交叉编译：4 目标，`cross` 工具链
- [X] 测试体系：domain 单元 + infra 集成 + 端到端（SimulatedBackend + SimulatedInput）
- [X] 性能埋点：`ARBOR_TUI_PROFILE=1` 环境变量，stderr 输出 FrameStats
- [X] 环境变量：4 个变量，优先级：参数 > 环境变量 > 默认值
- [X] Dogfooding：`apps/netmon/` 面板
- [X] API 兼容：v1 不承诺稳定，0.x.y semver

## 开放问题

- [ ] `arbor-tui` 作为独立 Git 仓库发布 crates.io，还是留在 Arbor monorepo 内只作为 workspace member？前者独立版本管理和 CI，但增加维护成本；后者简单但对外不可见
- [ ] 是否在 v1 提供 `cargo generate` 模板或 `arbor-tui new` 脚手架？建议不做——先狗食 netmon，模板在 dogfooding 后按需提供

## 参考

- [cross](https://github.com/cross-rs/cross) — Rust 交叉编译工具
- [crossterm](https://github.com/crossterm-rs/crossterm) — 终端控制
- [napi-rs](https://napi.rs/) — 预留（未来如有 TS 绑定需求）
