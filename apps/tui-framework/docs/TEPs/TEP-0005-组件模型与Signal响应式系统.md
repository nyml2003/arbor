---
id: TEP-0005
title: "组件模型与 Signal 响应式系统"
status: Review
created: 2026-07-04
updated: 2026-07-04
author: nyml
area: widgets
affects: ["ecosystem"]
related: [TEP-0001, TEP-0002, TEP-0003, TEP-0004]
---

# TEP-0005: 组件模型与 Signal 响应式系统

## 摘要

定义 Widget trait（组件接口）、内置组件集（9 种基础组件）、组件生命周期、以及显式订阅细粒度 Signal 响应式系统。核心差异于 Ratatui/Textual：**显式订阅、无自动依赖追踪、单线程局部重绘**——组件 mount 时手动订阅 signal，unmount 时退订，signal 变更仅标记对应 widget 脏，渲染循环批量重绘脏区。

## Widget trait

```rust
// domain/widget.rs

pub trait Widget: Send + Sync {
    fn id(&self) -> WidgetId;
    fn layout_props(&self) -> &LayoutProps;
    fn children(&self) -> &[WidgetNode];

    fn measure(&self, available: Size) -> SizeConstraint;
    fn render(&self, rect: Rect) -> VirtualScreen;

    // ── 聚焦与输入（见 TEP-0004）──
    fn focusable(&self) -> bool { false }
    fn tab_index(&self) -> u16 { 0 }
    fn on_key(&mut self, event: &KeyEvent) -> KeyHandleResult { KeyHandleResult::Bubble }

    // ── 生命周期 ──
    fn on_mount(&mut self) {}      // 插入组件树，注册 signal 订阅
    fn on_unmount(&mut self) {}    // 从树移除，退订 signal
}
```

所有方法都有默认实现——用户自定组件只需实现需要的方法。内置组件完整实现全部接口。

### WidgetId

```rust
/// 全局自增 u64，组件树内唯一。App 构造器自动分配
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct WidgetId(pub u64);
```

## 内置组件

所有组件通过构造器创建（`Text::new(...)`、`Box::new(...)`），构造器内部自动分配 `WidgetId`。不对外暴露结构体直接实例化。

### Box

容器组件。不渲染内容，只做布局——把 children 按 flexbox 规则排列。

```rust
pub struct Box {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub children: Vec<WidgetNode>,
}
```

使用场景：所有需要排列多个子组件的容器——页面根布局、水平/垂直排列、嵌套分组。

### Text

单行或多行文本。根据 `wrap` 策略决定是否换行。

```rust
pub struct Text {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub text: ReadSignal<String>,         // 只读文本内容
    pub style: ReadSignal<TextStyle>,     // 只读样式
    pub wrap: WrapStrategy,
    pub truncate: TruncateStrategy,
}

pub struct TextStyle {
    pub fg: AnsiColor,
    pub bg: AnsiColor,
    pub attrs: Attrs,
}
```

使用场景：标签、日志行、状态信息、帮助文本、标题。

### Input

单行文本输入框。持有内部编辑缓冲区（cursor 位置、当前文本），但**不直接修改上游 Signal**。用户每次输入触发 `on_change(new_text)`，业务层在回调中写 Signal，再驱动重渲染。

```rust
pub struct Input {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub value: ReadSignal<String>,     // 只读，组件无权修改
    pub placeholder: String,
    pub password: bool,
    pub on_change: Option<Box<dyn Fn(String)>>,   // 每次输入变更触发
    pub on_submit: Option<Box<dyn Fn(String)>>,   // Enter 触发
}
```

交互流程：用户按键 → Input 内部 buffer 更新 → 触发 `on_change(new_text)` → 业务层 `signal.set(new_text)` → ReadSignal 通知 Input 脏 → 重渲染。组件内部从不调用 `.set()`。

使用场景：搜索框、表单输入、命令输入行。

### Button

可点击的文本按钮。

```rust
pub struct Button {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub label: ReadSignal<String>,
    pub style: ReadSignal<ButtonStyle>,
    pub on_click: Option<Box<dyn Fn()>>,
}

pub enum ButtonStyle { Primary, Secondary, Danger, Default }
```

使用场景：确认/取消操作、触发动作、Tab 切换按钮。

### List

可滚动的列表。只渲染可见行——10,000 行数据源只有 viewport 内 ~10-20 行参与 layout + render。

```rust
pub struct List<T: Clone + 'static> {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub items: ReadSignal<Vec<T>>,            // 只读
    pub selected: ReadSignal<Option<usize>>,  // 只读
    pub render_item: Box<dyn Fn(&T, bool) -> WidgetNode>,
    pub scroll_offset: ReadSignal<usize>,     // 只读
    pub on_select: Option<Box<dyn Fn(Option<usize>)>>,  // 行选中时触发
    pub on_scroll: Option<Box<dyn Fn(usize)>>,          // 滚动偏移变更时触发
}
```

使用场景：日志列表、文件浏览器、搜索结果、配置项列表。

### Table

多列表格。列宽可固定或按比例（Flex）分配。

```rust
pub struct Table<T: Clone + 'static> {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub columns: Vec<ColumnDef>,
    pub rows: ReadSignal<Vec<T>>,
    pub selected: ReadSignal<Option<usize>>,
    pub render_cell: Box<dyn Fn(&T, usize) -> WidgetNode>,
    pub scroll_offset: ReadSignal<usize>,
    pub on_select: Option<Box<dyn Fn(Option<usize>)>>,
    pub on_scroll: Option<Box<dyn Fn(usize)>>,
}

pub struct ColumnDef {
    pub header: String,
    pub width: ColumnWidth,
}

pub enum ColumnWidth {
    Fixed(u16),
    Flex(f32),
}
```

使用场景：进程列表、网络连接表、WiFi 扫描结果、键值对配置。

### Tabs

Tab 切换容器。一个时刻只渲染一个 active child。

```rust
pub struct Tabs {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub tabs: Vec<TabDef>,
    pub active: ReadSignal<usize>,
    pub on_switch: Option<Box<dyn Fn(usize)>>,
}

pub struct TabDef {
    pub label: String,
    pub content: WidgetNode,
}
```

使用场景：多面板切换、设置页面导航、信息分类展示。

### ScrollView

通用滚动容器。包装一个超过视口的 child，提供虚拟滚动。

```rust
pub struct ScrollView {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub child: WidgetNode,
    pub scroll_x: ReadSignal<u16>,
    pub scroll_y: ReadSignal<u16>,
    pub on_scroll: Option<Box<dyn Fn(u16, u16)>>,
}
```

使用场景：长文本阅读、大表格容器、代码预览。

### WidgetNode

```rust
// domain/widget.rs

pub enum WidgetNode {
    Box(Box),
    Text(Text),
    Input(Input),
    Button(Button),
    List(Box<List<dyn Any>>),      // 类型擦除
    Table(Box<Table<dyn Any>>),
    Tabs(Tabs),
    ScrollView(ScrollView),
}

impl WidgetNode {
    /// 类型安全的 List 访问
    pub fn as_list<T: Any>(&self) -> Option<&List<T>> { /* downcast */ }
    /// 类型安全的 Table 访问
    pub fn as_table<T: Any>(&self) -> Option<&Table<T>> { /* downcast */ }
}
```

`WidgetNode` 枚举使组件树同构——每个节点都是 `WidgetNode`，可统一遍历。泛型 List/Table 通过 `dyn Any` 类型擦除，辅助方法 `as_list`/`as_table` 封装 downcast 逻辑。Enum 方案优于 trait object：终端组件集固定（9 种），编译期单态收益高。

## 生命周期（完整时序）

```
1. 组件插入组件树 → on_mount()
   - 订阅所有依赖的 Signal（signal.subscribe(self.id)）
   - 初始化内部状态

2. 布局流水线
   measure(Pass1) → layout(Pass2) → render → 生成初始 VirtualScreen

3. 进入事件循环：
   a. 读取终端输入事件，分发给焦点 Widget 的 on_key/on_click
   b. 事件回调执行 signal.set()，仅标记对应 WidgetId 脏，不渲染
   c. 一轮事件处理完毕，统一执行 render_if_dirty：
      i.   取出全部脏 WidgetId
      ii.  逐个重算 layout + render，生成局部变更 Cell 区域
      iii. 合并所有脏区域 → 一次 backend.emit()

4. 组件从树移除 → on_unmount()
   - 取消所有 Signal 订阅（signal.unsubscribe(self.id)）

5. Widget Drop
   - 兜底退订所有 Signal（防止 on_unmount 未被调用）
```

## Drop 兜底

所有内置 Widget 实现 `Drop` trait，内部复用 `on_unmount` 的退订逻辑。自定义组件推荐在 `Drop` 中同样退订。框架不引入 proc macro 自动生成——手动退订足够简单，宏增加编译复杂度。

## Signal 响应式系统

### 设计原则

不用 thread-local，不用自动依赖追踪，不用 Effect 抽象。TUI 只有一个主线程，同步渲染——不需要 SolidJS 那套并发追踪机制。

与 SolidJS 的核心取舍：放弃自动 effect 追踪以适配同步单线程终端事件循环。显式订阅在 TUI 场景下更简单——组件知道自己依赖哪些 signal，mount 时订阅，unmount 时退订。没有隐式依赖图，debug 直接看 `subscribers` 列表。

### 核心类型

```rust
// domain/signal.rs

/// 可写可变响应式值——仅业务层持有，组件禁止持有
pub struct Signal<T: Clone + PartialEq> {
    value: T,
    subscribers: Vec<WidgetId>,
    generation: u64,
}

impl<T: Clone + PartialEq> Signal<T> {
    pub fn new(initial: T) -> Self {
        Self { value: initial, subscribers: Vec::new(), generation: 0 }
    }

    /// 读取当前值。纯无副作用
    pub fn get(&self) -> T { self.value.clone() }

    /// 设置新值。值变化时通知所有订阅者标记 dirty
    pub fn set(&mut self, new_value: T) {
        if new_value != self.value {
            self.value = new_value;
            self.generation += 1;
            self.notify_subscribers();
        }
    }

    /// 创建只读视图——传递给组件
    pub fn read_only(&self) -> ReadSignal<T> {
        ReadSignal { source_id: self.id(), generation: self.generation, _phantom: PhantomData }
    }

    pub fn subscribe(&mut self, widget_id: WidgetId) { /* ... */ }
    pub fn unsubscribe(&mut self, widget_id: WidgetId) { /* ... */ }

    fn notify_subscribers(&self) { /* 通过 App 上下文的 DirtyTracker 标记 widget 脏 */ }
}

/// 只读响应式值——组件仅通过此类型消费数据，无 set 方法，编译期禁止回写
#[derive(Clone)]
pub struct ReadSignal<T: Clone + PartialEq> {
    source_id: SignalId,
    generation: u64,
    _phantom: PhantomData<T>,
}

impl<T: Clone + PartialEq> ReadSignal<T> {
    /// 读取当前值。内部委托给源 Signal
    pub fn get(&self) -> T { /* 从源 Signal 读取 */ }

    /// 订阅——组件 mount 时调用，内部委托给源 Signal
    pub fn subscribe(&self, widget_id: WidgetId) { /* ... */ }

    /// 退订——组件 unmount/Drop 时调用
    pub fn unsubscribe(&self, widget_id: WidgetId) { /* ... */ }
}
```

分层规则：
- `Signal<T>`：业务层持有，可 `get()` / `set()`。**组件绝不允许持有 `Signal<T>`**
- `ReadSignal<T>`：组件持有，仅 `get()` / `subscribe()` / `unsubscribe()`。无 `set` 方法，编译期拦截回写

组件内部从不调用 `.set()`——所有用户交互通过回调抛出事件，业务层在回调中写 Signal，单向流动。

### 单向数据流约束

**框架不提供任何隐式双向绑定。所有状态流转遵循单向数据流：**

```
用户交互 → 组件回调(on_change/on_select/...) → 业务层 signal.set() → 标记脏 → 重渲染
```

组件职责单一：仅渲染、仅派发事件，不持有状态修改权。从类型系统杜绝回写——组件只存 `ReadSignal<T>`，编译期无法调用 `.set()`。

### 辅助绑定工具

减少回调胶水代码，不恢复双向绑定：

```rust
/// 从 Signal 创建 (ReadSignal, 写回调)，简化单向绑定
fn bind_signal<T: Clone + PartialEq + 'static>(
    sig: &Signal<T>,
) -> (ReadSignal<T>, Box<dyn Fn(T)>) {
    let read = sig.read_only();
    let write = {
        let sig = sig.clone(); // 实际实现中通过 Rc/Arc 共享
        Box::new(move |v| sig.set(v))
    };
    (read, write)
}

// 使用示例
let (input_read, input_write) = bind_signal(&value);
Input {
    value: input_read,
    on_change: Some(input_write),
    ..
}
```

### 脏区追踪

`DirtyTracker` 绑定到 `App` 上下文，不做全局静态单例：

```rust
// domain/dirty.rs

pub struct DirtyTracker {
    dirty_widgets: HashSet<WidgetId>,
}

impl DirtyTracker {
    pub fn new() -> Self { Self { dirty_widgets: HashSet::new() } }

    pub fn mark_dirty(&mut self, widget_id: WidgetId) {
        self.dirty_widgets.insert(widget_id);
    }

    pub fn drain(&mut self) -> HashSet<WidgetId> {
        std::mem::take(&mut self.dirty_widgets)
    }
}
```

每个 `App` 实例持有自己的 `DirtyTracker`。Signal 通知订阅者时通过所属 App 上下文的 tracker 标记。单实例场景下与全局单例等价，多实例场景互不干扰。

### Widget 订阅流程

```rust
impl Widget for Text {
    fn on_mount(&mut self) {
        // ReadSignal 的 subscribe 内部委托给源 Signal
        self.text.subscribe(self.id);
        self.style.subscribe(self.id);
    }

    fn on_unmount(&mut self) {
        self.text.unsubscribe(self.id);
        self.style.unsubscribe(self.id);
    }
}

impl Drop for Text {
    fn drop(&mut self) {
        self.text.unsubscribe(self.id);
        self.style.unsubscribe(self.id);
    }
}
```

组件只持有 `ReadSignal<T>`，`subscribe`/`unsubscribe` 内部委托给源 `Signal<T>`。subscriber 列表只存 `WidgetId`（Copy 类型），无生命周期问题。

### 与渲染的集成

```rust
// app/render_loop.rs

pub fn render_if_dirty(
    app: &mut App,
    screen: &mut VirtualScreen,
    backend: &mut dyn TerminalBackend,
) {
    let dirty = app.dirty_tracker.drain();
    if dirty.is_empty() { return; }

    let mut dirty_regions = Vec::new();
    for widget_id in &dirty {
        if let Some(widget) = app.widget_tree.get(widget_id) {
            // 重算 layout → render → diff → 收集脏区
        }
    }

    dirty_regions = merge_regions(dirty_regions);
    backend.emit(&dirty_regions, screen);
}
```

细粒度：`Text("hello").text` → "world"：只标记该 Text 的 WidgetId，Layout 不变（宽度相同），仅 render → diff → emit 几列 Cell。`Box.flex` 改变：整个子树的 widget 全部标记 dirty → 子树重新 layout → 更大脏区。

### 批处理

天然批处理——不需要显式 batch API：

```
事件循环:
  1. recv_timeout(100ms) 拿到输入事件
  2. 逐事件分发 → signal.set() → 积累在 DirtyTracker
  3. 检查定时器回调 → signal.set() → 积累在 DirtyTracker
  4. 事件循环末尾 → render_if_dirty() 一次性处理所有脏 widget
```

Signal 的 `set()` 只更新值 + 标记 dirty。不触发渲染。`render_if_dirty()` 统一渲染。

### 渲染帧率上限

后台高频数据推送（日志流、监控指标每秒数十次更新）会不断触发 `signal.set()` → 每次事件循环都执行 `render_if_dirty()`。无节流上限时 CPU 会持续被 layout+diff+emit 占满。

**60fps 硬上限**：

```rust
// app/render_loop.rs

const MIN_FRAME_INTERVAL: Duration = Duration::from_millis(16); // ~60fps

pub fn render_if_dirty(
    app: &mut App,
    screen: &mut VirtualScreen,
    backend: &mut dyn TerminalBackend,
) {
    let now = Instant::now();
    if now - app.last_frame_time < MIN_FRAME_INTERVAL {
        return;  // 距上一帧不足 16ms，跳过本次渲染
    }

    let dirty = app.dirty_tracker.drain();
    if dirty.is_empty() { return; }

    // ... layout → render → diff → emit

    app.last_frame_time = now;
}
```

规则：
- 任意 16ms 窗口内只渲染一次。高频 signal 更新全部合并到窗口内的最后一帧
- 16ms 窗口等分 60fps——输入延迟上限 16ms，人眼无法感知
- 如果无 signal 变更，`dirty.is_empty()` 返回 true，帧率降为零（不渲染）
- 60fps 不是目标帧率，是**上限**。实际帧率 = min(事件频率, 60)

后台疯狂推数据（每秒 100 次 `signal.set()`）→ 每秒只渲染最多 60 次 → CPU 可控。

## 截断与换行策略

```rust
pub enum TruncateStrategy {
    End,       // "hello w…"
    Middle,    // "/usr/…/file.txt"
    None,      // 溢出到右侧空列
}

pub enum WrapStrategy {
    None,      // 不换行
    Word,      // 空格边界断开，CJK fallback 到 Char
    Char,      // 任意字符断开
}
```

组件声明策略，渲染层执行。布局层在 measure 阶段根据 `WrapStrategy` 计算行数。

## 子进程嵌入

用户在 TUI 里打开外部命令（`git diff`、`less`、`$EDITOR`）时，标准化四步流程：

```rust
// app/subprocess.rs

pub fn run_subprocess(
    cmd: &str,
    args: &[&str],
    backend: &mut dyn TerminalBackend,
    app: &mut App,
) -> io::Result<()> {
    // Step 1: 退出 TUI 渲染环境
    backend.exit_alternate_screen();
    backend.show_cursor();
    drop(app.terminal_guard.take());  // 恢复终端设置（echo, canonical mode）

    // Step 2: spawn 子进程，阻塞等待结束
    let status = std::process::Command::new(cmd).args(args).status()?;

    // Step 3: 重新初始化 TUI 渲染环境
    app.terminal_guard = Some(backend.enter_raw_mode());
    backend.enter_alternate_screen();
    backend.hide_cursor();

    // Step 4: 全屏重绘
    app.mark_all_dirty();
    app.render(backend);

    Ok(())
}
```

框架仅处理终端模式切换。子进程的 stdin/stdout 重定向由上层业务自行管理（通过 `Command::stdin/stdout` 配置）。

## 错误处理与 panic 恢复

### 框架层 panic 保底

App 启动时注册 `std::panic::set_hook`：

```rust
// app/main.rs

std::panic::set_hook(Box::new(|info| {
    // 1. 紧急恢复终端（直接写 fd，不依赖任何框架状态）
    let mut stdout = std::io::stdout();
    let _ = write!(stdout, "\x1b[?1049l");  // 退出 alternate screen
    let _ = write!(stdout, "\x1b[?25h");    // 显示光标
    // 2. 打印 panic 信息到 stderr
    eprintln!("[arbor-tui] PANIC: {}", info);
    // 3. backtrace 由 Rust 标准 panic hook 输出
}));
```

`TerminalGuard::Drop` 做第二层保底——即使 panic hook 未执行，guard drop 时也会恢复终端设置。

### Widget 层错误处理

Widget 的 `render()` / `on_key()` 不 panic。内置组件对无效状态（零尺寸、越界索引）使用 fallback 渲染（空白 Cell 区域），不传播错误。用户自定义组件自行保证不 panic。

### 日志分级

框架层日志通过 `eprintln!` 输出到 stderr。TUI 使用 alternate screen 占据 stdout，stderr 独立于 TUI 渲染面，不会污染界面。不做日志框架集成（不引入 log/env_logger）——用户应用自行选择日志方案。

错误分类：
- **框架内部错误**（Cell 越界、layout 异常输入）→ eprintln! + fallback 渲染，不 panic
- **用户回调 panic** → panic hook 捕获，终端恢复后 exit
- **OS 信号**（SIGINT/SIGTSTP）→ 见 TEP-0004 信号处理

## 主题方案

**决议：全局单例**。`Theme::global()` 返回 `&Theme`。Widget 的 `render(rect, theme: &Theme)` 签名预留 theme 参数。多实例需求出现后切换独立实例不破坏 trait 接口。

终端深浅色：v1 不做自动检测。用户通过 `THEME=light/dark` 环境变量或启动参数手动指定。

## 已决议的开放问题

- [x] **WidgetNode：Enum vs Trait Object**：Enum + 类型擦除。组件集固定（9 种），编译期单态收益高。辅助方法 `as_list`/`as_table` 抹平 downcast 繁琐度
- [x] **Signal::get 无副作用**：永久固定。不引入自动依赖追踪
- [x] **Signal 监听回收**：显式订阅 + `on_unmount` 退订 + `Drop` 兜底。WidgetId Copy 类型，无悬垂引用
- [x] **双向绑定**：彻底移除。全架构单向数据流——组件仅消费 `ReadSignal<T>`，交互通过回调抛出事件，业务层在回调中写 `Signal<T>`。编译期禁止组件回写
- [x] **主题方案**：全局单例。render 签名预留 theme 参数，多实例需求出现后切换

## 落地优先级

### P0（当前 TEP 必须实现）
- `Signal<T>` / `ReadSignal<T>` 双类型分层，组件只存 ReadSignal，编译期禁止回写
- 所有交互组件（Input/List/Table/Tabs/ScrollView）新增变更回调，移除内部 `set()` 逻辑
- DirtyTracker 绑定 App 上下文，不作全局静态
- 所有内置 Widget 实现 Drop 兜底退订
- WidgetId 全局自增分配、构造器模式
- `bind_signal()` 辅助绑定工具
- 完整生命周期时序闭环

### P1（后续迭代，不阻塞当前版本）
- `Signal::map` 派生信号（`ReadSignal` 本身已实现）
- Widget 可选 `render_diff` 增量渲染方法
- `StyleSheet` 样式复用独立 TEP

### P2（远期）
- 多 TUI App 实例隔离
- 组件样式继承、主题热更新
- 表单校验 Signal

## 开放问题

- [ ] `WidgetNode` 是否需要 `Custom(Box<dyn Widget>)` variant 支持完全自定义组件？当前设计用户只能通过内置组件的组合实现自定义 UI。variant 会破坏 enum 穷举性。建议当前不做，实际需求出来后独立 TEP 评估

## 参考

- [SolidJS Reactivity](https://www.solidjs.com/guides/reactivity) — 细粒度 signal 参考（区别：无自动追踪）
- [Ratatui Widget trait](https://docs.rs/ratatui/latest/ratatui/widgets/trait.Widget.html) — Rust TUI 组件接口
- [Textual Widget](https://textual.textualize.io/guide/widgets/) — Python TUI 组件模型
