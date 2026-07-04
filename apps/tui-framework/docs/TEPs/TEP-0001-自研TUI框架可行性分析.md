---
id: TEP-0001
title: "自研 TUI 框架可行性分析"
status: Review
created: 2026-07-04
updated: 2026-07-04
author: nyml
area: architecture
affects: []
related: []
---

# TEP-0001: 自研 TUI 框架可行性分析

## 目标

为 Arbor 提供一个 TUI 交付面。终端是所有平台共有的 UI——本地桌面、SSH 远程、Docker 容器、无头服务器、tmux 多路复用——全都能跑。不取代 Electron GUI，而是作为对等交付面覆盖 GUI 到不了的地方。

具体产出：一个 Rust TUI 框架，声明式组件树 + 布局引擎 + < 50ms 冷启动。不做通用框架，只覆盖 Arbor 自身场景。

## 做什么

字符网格上的七层栈：

```
文本测量 → 布局 → 渲染 → diff → ANSI 写入 → 键盘输入 → 焦点管理
```

每层职责和边界在各自 TEP 中展开。在此基础上内置 8-10 个基础组件（Box、Text、Input、Button、List、Table、Tabs、ScrollView）。

### 文本测量

`unicode-width` crate（UAX #11）。不同字符占不同列宽：ASCII 1 列、CJK 2 列、组合字符零宽。框架只处理 tab 展开和截断/省略号。

## 不做的事

每个"不做"都是刻意的减法，背后有具体原因。

### 不做 pixel-perfect CSS 布局

终端是等宽字符网格，不是像素画布。`float`、`z-index`、百分比圆角没有对应概念。Textual 为此从 50ms 膨胀到 800ms 启动。我们要的是整数格点上的弹性空间分配。

### 不做 mouse 支持

终端 SGR mouse protocol 在 SSH、tmux、mosh 下经常不转发。如果框架依赖鼠标交互，远程会话直接不可用。键盘是终端唯一可靠的输入设备。

### 不做 IME 输入法支持

raw mode 下输入法组合窗口不工作——终端不转发未完成的组合序列。这不是框架层能解决的问题，是终端协议的固有限制。需要 CJK 输入时在应用层用 non-raw 的独立输入行绕过。

### 不做复杂文字排版

不引入 harfbuzz、不引入 ICU。只做 ASCII + 基础 CJK（通过 unicode-width 测量列宽）。不做双向文本（bidi）、竖排、连字、复杂字形组合。Arbor 运维工具只需简单日志、表单、表格，没有出版级排版需求。省掉 FFI 依赖，控制二进制体积和编译时间。

### 不做真彩色依赖

框架默认 256 色调色板，TrueColor 作为可选增强。SSH 会话经常降级到 8/16 色，tmux 默认也不转发 TrueColor。不能假设用户端支持。

### 不做 kitty image protocol / sixel / 渐变

终端增强特性，只有少数现代终端支持。Windows Terminal 全不支持。一旦引入，框架可达到的终端范围大幅缩窄。

### 不做 bracketed paste / focus in/out events

DEC 私有扩展（`CSI ? 2004 h`、`CSI ? 1004 h`），不是所有终端实现。粘贴不需要区分事件——快速打字和粘贴在框架层都是"收到一批 key event"。focus in/out 在单窗口终端里没场景。

### 不做终端快捷键覆盖

框架快捷键只在框架内部生效，不拦截原生终端全局绑定（Ctrl+C、Ctrl+Z、Ctrl+D 等由 shell/OS 处理）。不与 tmux、ssh、shell 快捷键冲突。框架的输入事件不试图"劫持"终端。

### 不做多窗口 / 多面板管理

框架只管一个终端窗口内的布局。tmux 的多 pane、多窗口不是框架职责。用户可以在多个 tmux pane 里各自跑 TUI 实例。

### 不做 inline 模式（Phase 1）

Phase 1 只做 alternate screen（占据整个终端，类似 vim）。Inline 模式（`fzf` 风格的下拉面板）对 API 设计影响完全不同，单独评估。

### 不做 terminfo 数据库式的兼容穷举

不穷举所有终端的 ANSI 差异。只支持现代 ANSI 终端（Windows Terminal、iTerm2、Kitty、Alacritty、Linux tty），通过 crossterm 接入。XP legacy console 的 Win32 API 适配器架构预留但当前不实现。

## 已知难点

这些是 TUI 框架的固有坑点，不是"做不做"的决策问题，而是无论怎么做都要面对的技术问题。列在这里是为了后续 TEP 逐项展开时不被遗漏。

### 窗口大小变化 (SIGWINCH)

用户随时 resize 终端。布局全量重算，但所有组件状态不能丢。crossterm 提供 resize 事件，但如何高效通知组件树"可用空间变了"并由组件自行决定重绘策略，是布局引擎和组件模型要一起解的。

### 闪烁

全量清屏重绘产生可感知的闪烁。Alternate screen 天然双缓冲——写屏和显示是两块独立区域，写完再切过去——大部分场景不会闪。但 cursor 反复显示/隐藏可能引起局部闪烁，不是所有终端支持 `CSI ? 25 h/l`（隐藏/显示 cursor）。

### 终端恢复

raw mode 接管终端后，Ctrl+C 不会自动终止。框架无论以什么方式退出（正常、panic、SIGINT），必须恢复：echo、canonical mode、cursor 显示、alternate screen。没恢复 = 用户终端废了。

Rust 的 ownership 在这里是天然优势：`Drop` guard 保证 panic unwinding 也能恢复终端。需要设 `panic::set_hook` 和 signal handler 做双保险。

### 颜色与主题

- 256 色 vs TrueColor：SSH 降级、tmux 不转发
- 用户终端可能是浅色主题——不能硬编码深色背景、不能只用亮色文字
- ANSI color index 在不同终端渲染出不同颜色（"red" 在 iTerm2 和 Windows Terminal 不是同一个红）
- `NO_COLOR=1` 环境变量标准：应禁用所有颜色输出
- 红绿色盲用户看不到 error/success 的颜色差异——关键状态需要前缀符号（✓ ✗ ⚠ →）

框架约束：
- 内置两套基础主题（浅色 / 深色），自动检测终端背景色后选择，不硬编码背景
- 主题全部使用 256 色索引映射；TrueColor RGB 值作为 Cell 的附加属性，终端不支持时直接丢弃，框架不依赖 TrueColor 传递信息
- 高对比度模式作为主题变体提供

### 滚动与视口

终端没有真正的滚动——只有回滚缓冲区。ScrollView 需要自己维护虚拟滚动位置（offset + viewport_height）。10,000 行日志列表：只渲染可见行还是全量 layout 再裁切？前者快但组件模型复杂，后者简单但浪费。滚动条用字符模拟（█ ▌ ▐）。

### 异步事件与渲染循环

单主线程渲染，独立 stdin 阻塞读线程，mpsc channel 推送按键事件到主线程。

硬性约束：**后台任务禁止直接操作组件状态**。所有状态变更必须走 Signal `set()`。后台 I/O / 日志任务仅允许写入共享只读缓冲区，Signal 批量刷新列表。不允许多线程同时修改组件树——避免布局锁竞争和竞态条件。

### Ctrl+Z / 进程挂起与恢复

用户 Ctrl+Z 挂起 TUI → 框架恢复终端 → shell 出现。`fg` 恢复 → 框架重新进入 raw mode → 全量重绘。需要正确处理 SIGTSTP 和 SIGCONT。

### 嵌套子进程

用户在 TUI 里打开 `git diff` 或 `less`。框架需要：退出 alternate screen → 恢复终端 → spawn 子进程 → 等子进程结束 → 重新进入 raw mode → 全量重绘。不支持则用户只能退出 TUI → 跑命令 → 重新进 TUI。

### Windows XP legacy console（deferred）

XP console 没有 ANSI 支持，需要独立 Win32 Console API 后端。当前 arbor 阶段先 defer——先在消费者 Windows（Win10+）上跑通，XP 适配器的 `trait TerminalBackend` 实现留到后续。架构上预留了适配器接口，不会阻塞。

### 性能边界

Signal 驱动的脏区渲染，不是每帧全量。性能目标按最坏情况（全屏脏）设定，常规情况远低于此。

- 80×24 = 1,920 Cell 全屏脏 → layout + diff + emit < 1ms
- **240×60 = 14,400 Cell（1080p）全屏脏 → layout + diff + emit < 5ms**
- 帧率上限 120fps（节流），正常交互（按键/刷新）远低于此
- 10,000 行 List：只 layout 可见行，viewport 外零开销
- 递归嵌入：TUI 里嵌另一个 TUI 实例，渲染管道会递归

### 文本换行与截断

文本超出分配宽度后的行为分两条路，各归各层：

**换行（布局层）**：文本溢出时折到下一行 → 组件变高 → 父级需要重新分配空间。这是布局引擎的事，需要 measure 阶段知道"这段文本在 20 列宽下占 3 行"。

**截断（渲染层）**：收到布局分配好的格子（比如 20×3）后，填入文字。被限制在分配区域内，最后一行超出部分裁掉加省略号。组件声明策略，框架提供内置实现：
- `End`：硬截断 "hello w…"
- `Middle`：中间截断 "/usr/…/file.txt"（长路径常用）
- `None`：溢出到右侧空列（如果右侧为空白）

**换行规则（组件声明，布局层消费）**：
- `None`：不换行，超出就由截断策略处理
- `Word`：按空格/标点边界断开（拉丁文本）
- `Char`：任意字符处断开（CJK 文本，不需要空格）

分层职责：布局引擎负责"换行后需要多少行"（measure），渲染层负责"在分配的格子内填入文字并处理截断"（render），组件负责声明策略。

## 业务身份

**Arbor 终端运维面板框架**。不是通用 TUI 框架。建模的领域是：运维人员在一台普通电脑的终端上，实时查看网络状态、WiFi 扫描结果、系统配置——用键盘操作，用字符网格展示结构化数据。

核心领域概念（Ubiquitous Language）：Cell（字符单元）、Widget（组件）、Signal（响应式值）、Screen（字符画布）、Viewport（视口）、Focus（焦点）、Theme（主题）、DirtyRegion（脏区）。

## 架构边界

### 水平分层

依赖方向向内：`cli → app → domain ← infra`

```
cli/          → 二进制入口、参数解析、App 构造启动
app/          → 用例编排：渲染循环（RenderLoop）、事件循环（EventLoop）、终端生命周期（TerminalGuard）
domain/       → 纯逻辑，零副作用，零外部依赖。全部类型和 trait 在此定义
infra/        → 副作用适配器，实现 domain 定义的 trait。可替换
```

跨层规则：
1. domain 不 import infra / app / cli
2. infra 实现 domain trait，不反向依赖
3. app 持有 domain 状态 + infra 适配器实例，只做编排不做逻辑
4. Signal 是唯一的可变状态入口。不直接修改 widget tree

### 竖直分层（功能切片）

每个切片横跨水平层。切片之间无直接依赖——通过 domain trait 通信。

| 切片 | domain（纯逻辑） | infra（副作用） | app（编排） |
|------|------------------|-----------------|-------------|
| **渲染** | Cell, AnsiColor, VirtualScreen, Diff 算法 | `TerminalBackend` trait → CrosstermBackend | RenderLoop：signal 脏了 → diff → emit |
| **布局** | Rect, Size, LayoutConstraint, Flexbox 引擎, TextMeasure | — | — |
| **输入** | KeyEvent, FocusPath, KeyMap | `InputReader` trait → StdinReader | EventLoop：stdin → signal.set() |
| **组件** | Widget trait, WidgetNode enum, Lifecycle 状态机 | — | WidgetTree 管理 |
| **信号** | `Signal<T>`, `ReadSignal<T>`, DirtyTracker | — | 连接 Widget 属性到渲染触发 |
| **主题** | Theme, ColorPalette, Style, 浅色/深色检测逻辑 | — | 主题注入到 WidgetTree |

### 函数式核心，命令式外壳

- **domain/** 全部纯函数 + 不可变结构。`layout(input) → Rect`、`diff(old, new) → Vec<DirtyRegion>`、`measure(text, width) → LineCount`。零 I/O。
- **infra/** 封装全部副作用，通过 trait 注入 app。
- **app/** 纯胶水：读 infra 输入 → 写 domain signal → 调 domain 纯函数 → 调 infra emit。

### 可测试性

- domain 纯函数：单元测试，零依赖，CI 直接跑
- infra：`SimulatedBackend` 替换 crossterm，`SimulatedInput` 回放按键，无屏 CI
- app：注入模拟 infra，验证完整渲染循环和信号传播

## 技术栈

纯 Rust。不引入 FFI 绑定层。核心决策：

- **布局引擎**：手写 flexbox 子集。不引入 taffy。
- **终端控制**：适配器模式——`trait TerminalBackend`。第一个实现 crossterm（现代 ANSI 终端），第二个 winapi（XP legacy console）。编译期 feature flag 或运行时检测切换。
- **渲染管道**：Signal 驱动。状态变更 emit signal → 受影响的脏子树重算 layout → diff 脏区域 → emit ANSI。不是每帧全量跑，没变更就不渲染。120fps 是节流上限，不是目标帧率。

## 约束条件

- 零外部运行时依赖（crate 依赖按需评估）
- 冷启动 < 50ms
- 1080p 全屏终端（~240×60 = 14,400 Cell）：脏区 diff 后 emit < 5ms，帧率上限 120fps（节流，非目标）
- 内存占用基线（空应用）< 5MB。无 GC 泄漏，可量化标准：
  - 连续运行 24 小时、持续滚动 10 万行日志、频繁切换组件 → RSS 内存波动不超过 10%
  - 所有子组件 Drop 后：Cell 网格释放、Signal 监听自动解绑、无悬停引用
- Release 静态链接单文件，主流平台二进制 ≤ 8MB
- 无标准库外系统依赖：不依赖系统 terminfo、字体文件、系统颜色配置
- 交叉编译：x86_64 Linux、aarch64 Linux、x86_64 Windows、aarch64 macOS
- 测试：单元测试覆盖文本测量、布局计算、diff 脏区；集成测试覆盖 SSH/tmux 嵌套场景；模拟终端后端做无屏幕自动化测试
- Rust workspace，独立于 kaubo-features
- 仅 UTF-8 编码，支持中文和 ASCII。不做 GBK/Shift-JIS 等其他编码转换
- 消费者 Windows 可运行：默认 Windows Terminal 或 cmd，无定制字体/配色，二进制直接双击
- 第一阶段不引入 async runtime——同步渲染管道，stdin 用独立线程阻塞读
- 退出时终端状态 100% 恢复（Drop guard + panic hook 双保险）

## 后续 TEP

| TEP | 模块 | 要讨论的核心问题 |
|-----|------|-----------------|
| TEP-0002 | 渲染层 | 文本测量、Cell 数据结构（含颜色模型：256 色 + TrueColor 可选）、diff 策略、`trait TerminalBackend`（crossterm 适配器）、闪烁对策 |
| TEP-0003 | 布局引擎 | 手写 flexbox 子集（direction/justify/align/flex/padding/margin）；SIGWINCH 重排策略；换行 measure |
| TEP-0004 | 输入系统 | stdin 读取、CSI 解析、事件节流、信号处理（SIGINT/SIGTSTP/SIGCONT）、终端快捷键不覆盖边界 |
| TEP-0005 | 组件模型 + Signal | Widget trait、生命周期、细粒度 signal 系统设计、截断策略、子进程嵌入；待决：signal 监听回收策略、主题单例 vs 多实例 |
| TEP-0006 | 工程化 | crate 拆分、模拟终端后端（无屏测试）、交叉编译 CI、二进制体积控制、netmon dogfooding |

## 开放问题

- [x] Signal 粒度：细粒度（属性级）。TEP-0005 已定
- [x] 讨论顺序：TEP → demo → 调整。自下而上原型驱动
- [x] Signal 监听回收：显式订阅 + `on_unmount` 退订 + Drop 兜底。不用弱引用。TEP-0005 已定
- [x] 主题存储：全局单例 `Theme::global()`。render 签名预留参数，多实例需求后切换。TEP-0005 已定
- [x] 双向绑定：彻底移除。全架构单向数据流——组件只消费 `ReadSignal<T>`，交互通过回调抛出事件。TEP-0005 已定
- [x] crossterm 接入：v1 必须接入，仅作 infra 适配器。domain 零耦合。TEP-0002 已定
- [x] CPU 闲置节流：`recv_timeout(100ms)` 阻塞等待，零 CPU 空转。TEP-0004 已定
- [x] 事件缓冲区上限：bounded channel 256 事件，满时丢弃旧事件。TEP-0004 已定
- [x] ANSI flush 策略：每次 emit 末尾统一 flush，不跨 emit 缓冲。TEP-0002 已定
- [x] ScrollView 视口裁切：measure 用视口 available，render 剪裁可见区。TEP-0003 已定
- [x] 错误处理：panic hook 恢复终端 + TerminalGuard Drop 兜底 + Widget fallback 不 panic。TEP-0005 已定
- [x] 子进程嵌入：四步标准化流程（退出→spawn→重进→全屏重绘）。TEP-0005 已定
- [x] 渲染帧率上限：60fps 硬上限（16ms 最小帧间隔），高频信号批量合并。TEP-0005 已定
- [x] Flex 余数分配：按 flex 权重降序依次补给前 N 个弹性子项，填满分隔空隙。TEP-0003 已定
- [x] 尺寸计算工具：`SizeCalc` 统一封装 margin/padding 扣除 + 饱和减法。TEP-0003 已定
- [x] CSI 解析：全部由 crossterm `event::read()` 处理，不自研状态机。TEP-0004 已定
- [x] 信号管理：`SignalManager` 统一管理 SIGINT/SIGTSTP/SIGWINCH 回调。TEP-0004 已定
- [x] SIGTSTP 恢复：重新创建 TerminalGuard 后强制 `mark_all_dirty()`，杜绝界面空白。TEP-0004 已定

## 已记录、defer 到 TEP-0006 或后续的项

以下项目已知但不阻塞 v1，在对应 TEP 中留有扩展点：

| 项 | defer 原因 |
|----|-----------|
| 端到端集成测试框架 | 需要 TEP-0006 定义 CI 基础设施和模拟终端标准 |
| 性能埋点（diff/layout/帧耗时） | 需要 TEP-0006 定义 instrumentation 方案 |
| 日志分级/过滤/持久化 | 当前 `eprintln!` 够用。高级日志留 TEP-0006 |
| Widget 异步批量销毁 | 同步退订在当前组件量级下无瓶颈。后续异步清理独立 TEP |
| 自定义 tab 宽度 | 硬编码 4 足够。可通过 `TabStop(u16)` 配置项扩展 |
| Text 工具统一封装 | `measure_width`/`truncate`/`wrap_lines` 已在 domain/text.rs 集中 |
| DirtyRegion 关联 Widget 来源 | 当前设计不需要——脏区合并后不再追溯来源。如调试需求出现再加 `source_id: Option<WidgetId>` 字段 |
| 组件 visible/hidden 控制 | 通过 `LayoutProps { width: Some(0), height: Some(0) }` 模拟。独立属性后续 TEP |
| 灰度/高对比度主题 | 主题系统预留了变体机制。具体实现后续 TEP

## 参考

- [Textual](https://textual.textualize.io/) — Python TUI 框架
- [Ratatui](https://ratatui.rs/) — Rust TUI 渲染库，立即模式
- [taffy](https://github.com/DioxusLabs/taffy) — Rust flexbox 布局库
- [crossterm](https://github.com/crossterm-rs/crossterm) — Rust 跨平台终端控制
- [unicode-width](https://crates.io/crates/unicode-width) — Unicode UAX #11
- [ANSI Escape Codes](https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797)
- [NO_COLOR](https://no-color.org/) — 颜色禁用标准
