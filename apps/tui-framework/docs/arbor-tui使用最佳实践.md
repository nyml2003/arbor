# arbor-tui 使用最佳实践

本文面向 arbor-tui 的使用者。也就是用这个框架写 TUI 应用的人。

本文不讲框架内部实现。内部维护规则见 `docs/arbor-tui开发最佳实践.md`。

## 适合用 arbor-tui 的场景

arbor-tui 适合做键盘驱动的终端工具：

- 状态面板。
- 表格和列表浏览。
- 配置编辑。
- 命令输入。
- 运维和诊断工具。
- 可在 SSH、tmux、本地终端和 xterm.js 中运行的轻量 TUI。

不适合的场景：

- 复杂鼠标交互。
- 富文本排版。
- IME 输入重度依赖。
- 像素级图形界面。
- 需要完整 CSS 布局的应用。

## 推荐应用结构

把应用分成三部分：

```text
状态模型 -> build_ui -> TerminalApp
```

推荐文件结构：

```text
src/
├── main.rs        # TerminalApp 启动入口
├── ui.rs          # build_ui，组件树构建
├── state.rs       # 应用状态和命令处理
└── tests/         # E2E 场景
```

小 demo 可以放在一个文件里。正式工具不要把状态、UI 和事件循环全塞进 `main.rs`。

## 从 build_ui 开始

`build_ui` 应该是一个纯构建函数。输入状态和 theme，输出 `WidgetNode`。

推荐形式：

```rust
fn build_ui(
    factory: &WidgetFactory,
    theme: &Theme,
    state: &AppState,
) -> WidgetNode {
    // build widget tree
}
```

规则：

- `build_ui` 不读终端。
- `build_ui` 不轮询输入。
- `build_ui` 不修改运行时主状态。
- UI 文案、布局和主题都集中在这里。
- 回调只把用户动作交给状态层，不在组件里写复杂业务逻辑。

## 布局实践

arbor-tui 使用终端版 Flexbox。

常用布局：

```text
Col
├── Header
├── Body (flex)
└── Footer
```

三栏布局：

```text
Row
├── Left  (fixed width)
├── Main  (flex)
└── Right (fixed width)
```

规则：

- 页面根节点设置 `.size(cols, rows)`。
- 主内容区域通常 `.flex(1.0)`。
- 想铺满高度的 panel，本身也要 `.flex(1.0)`。
- 固定宽度只用于侧栏、短按钮、输入框等明确尺寸。
- 文案不要把最小宽度撑爆。默认按 80 列可读设计。
- 使用 `TerminalApp::with_builder` 时，builder 会拿到真实 `cols` 和 `rows`。
- resize 后让 `TerminalApp` 调用 builder 重建 UI 树。

常见错误：

```rust
Col::new().flex(1.0).children([border]).build(factory, theme)
```

如果 `border` 自己没有 `.flex(1.0)`，它可能只按内容高度渲染。想让 panel 铺满，给 Border 也加 flex：

```rust
let panel = Border::new()
    .rounded()
    .flex(1.0)
    .title(" Logs ")
    .child(content)
    .build(factory, theme);
```

## 颜色实践

总是使用 `Theme` 的语义色。

推荐：

- 普通文字：`theme.text()`。
- 次要文字：`theme.text_dim()`。
- 页面背景：`theme.surface()`。
- panel 背景：`theme.surface_alt()` 或 demo 自己定义的 panel 背景。
- 焦点、主操作：`theme.primary()`。
- 选中、高亮：`theme.accent()`。
- 成功：`theme.success()`。
- 警告：`theme.warning()`。
- 危险：`theme.danger()`。
- 边框：`theme.border()`。

不要直接用裸 palette index 写业务含义。裸颜色只适合测试或临时实验。

light theme 下要特别检查：

- 可见文字不能出现默认黑底。
- 输入框尾部空白要有背景。
- 选中行整行要有背景。
- panel 内空白区域要有背景。
- xterm.js 和本地终端显示要一致。

## 组件使用建议

### Text

用于短文本、标题、状态值。

推荐：

```rust
Text::new("ready")
    .fg(theme.success())
    .bg(theme.surface_alt())
    .build(factory, theme)
```

如果 Text 放在有背景的 panel 里，显式设置 `.bg(panel_bg)`。

### RichText

用于多行文本和局部样式。

推荐先设置整体背景：

```rust
RichText::new()
    .bg(Cell {
        bg: panel_bg,
        ..Default::default()
    })
    .line(vec![Span::new("Status", theme.text(), panel_bg, Attrs::default())])
    .build(factory, theme)
```

每个 `Span` 都要给背景色。不要只给前景色。

### Border

用于 panel，不要把所有东西都包成 Border。

推荐：

```rust
Border::new()
    .rounded()
    .flex(1.0)
    .fg(theme.border())
    .bg(panel_bg)
    .title(" Jobs ")
    .child(content)
    .build(factory, theme)
```

如果 panel 要铺满父级剩余高度，Border 自己要 `.flex(1.0)`。

### Divider

Divider 用于一行分隔。默认样式是 `╭-------╯`。

推荐：

```rust
Divider::new()
    .flex(1.0)
    .fg(theme.border())
    .bg(panel_bg)
    .build(factory, theme)
```

规则：

- 不要用 `Text::new("------")` 手写分隔线。
- 需要铺满父级宽度时使用 `.flex(1.0)`。
- 需要固定宽度时使用 `.width(n)`。
- 分隔线所在区域也要设置背景色。
- 可以用 `.glyphs(left, fill, right)` 改成项目自己的样式。

如果分隔线带标题，优先用 composites：

```rust
SectionDivider::new("Files")
    .divider_width(8)
    .bg(panel_bg)
    .build(factory, theme)
```

如果分隔线后面总是跟一个内容区，使用 `DividerBlock`：

```rust
DividerBlock::new("Files", file_list)
    .divider_width(8)
    .bg(panel_bg)
    .build(factory, theme)
```

如果多个文本分区需要共用一个外框，并且中间要用 `╰────╭╯` 这种连接线，使用 `SectionedPanel`：

```rust
SectionedPanel::new([
    SectionedPanelSection::new("上方主信息区")
        .line("系统名称：TUI 控制面板")
        .line("连接状态：在线"),
    SectionedPanelSection::new("下方详情分区")
        .line("CPU 占用：27%")
        .line("在线客户端：5 台"),
])
.fg(theme.border())
.bg(panel_bg)
.build(factory, theme)
```

规则：

- 单行标题分隔用 `SectionDivider`。
- 标题后跟任意 widget 内容用 `DividerBlock`。
- 多段文本共享一个边框用 `SectionedPanel`。
- 不要在业务代码里手写整块 box drawing 字符串。

### Transcript

Transcript 用于聊天记录、Agent 输出和带 Markdown 的消息流。

推荐：

```rust
let transcript = Transcript::new()
    .messages(messages.iter().map(|message| {
        TranscriptMessage::new(message.role_label(), theme.primary(), message.body())
    }))
    .empty_text("No messages")
    .scroll_y(scroll_y.read_only())
    .bg(panel_bg)
    .flex(1.0)
    .build(factory, theme);
```

规则：

- 不要在应用里重复写 Markdown 解析、代码块边框和消息行数估算。
- Markdown 到 `Span` 的转换放在 `arbor-tui-markdown`。
- 聊天记录布局放在 `arbor-tui-composites::Transcript`。
- 应用只负责把业务消息转换成 `TranscriptMessage`。
- 错误、流中断等提示用 `TranscriptNotice`，不要混进业务消息列表。

### Input

Input 是非受控组件。用户输入先存在组件内部。

使用 `on_submit` 接收最终命令：

```rust
Input::new()
    .placeholder("type command")
    .on_submit(move |cmd| {
        // handle command
    })
    .build(factory, theme)
```

使用建议：

- 命令行输入放 footer。
- 表单字段用明确 placeholder。
- 不要把长篇帮助文本塞进 placeholder。
- password 输入使用 `.password()`。

### List 和 Table

List 适合单列对象。Table 适合结构化数据。

规则：

- 长列表要放在可滚动区域里。
- 选中状态要用 theme 高亮。
- 表格列宽先用固定宽度，避免窄屏布局抖动。
- 表格内容要短。详细信息放右侧 Info panel。

### Tabs

Tabs 适合少量视图切换。

规则：

- tab label 要短。
- 每个 tab 的内容区域要能独立渲染。
- tab 内有 Input/Button/List 时，要用 E2E 测焦点。
- 不要用 Tabs 做复杂导航树。

## 输入和快捷键

默认行为：

- `Tab`：焦点前进。
- `Shift+Tab`：焦点后退。
- `Enter`：激活或提交。
- `Esc`：退出。
- `Ctrl+C`：退出。
- `Ctrl+Q`：退出。
- 方向键：由焦点组件处理。

使用建议：

- 不要覆盖全局退出键。
- 表单提交用 Enter。
- 列表移动用方向键。
- 命令输入用 footer Input。
- 复杂快捷键先写到状态层，不要散落在 widget 回调里。

## 启动入口建议

普通应用不要手写终端生命周期和事件循环。

推荐使用 `TerminalApp`：

```rust
use arbor_tui_runtime::{run_crossterm_terminal_app, TerminalApp};

fn run() -> anyhow::Result<()> {
    let theme = Theme::dark();

    let app = TerminalApp::with_builder(theme, move |cols, rows, theme| {
        build_ui(cols, rows, theme)
    });

    run_crossterm_terminal_app(app)
}
```

`TerminalApp` 负责：

- 进入和退出 alternate screen。
- 进入和退出 raw mode。
- 隐藏和恢复 cursor。
- 安装 panic 恢复 hook。
- 轮询输入。
- 处理首帧渲染。
- 处理 resize。
- 调用 runtime step。
- 根据 dirty 状态渲染。
- 退出时关闭输入源并恢复终端。

只有需要接入自定义 backend 或输入源时，才直接调用：

```rust
app.run(&mut backend, &input)?;
```

不要在业务页面里重复写 `EnterAlternateScreen`、`LeaveAlternateScreen`、`runtime_step` 循环和 `first_frame` 标记。

如果 theme、尺寸或全局状态变化，需要重建 root tree。

状态变化可以用 `before_render` 收敛：

```rust
let app = app.before_render(move |app, root, theme| {
    if !changed.get() {
        return false;
    }

    changed.set(false);
    *theme = state.borrow().theme.clone();
    let (cols, rows) = app.screen_size();
    *root = build_ui(cols, rows, theme);
    true
});
```

回调返回 `true` 时，`TerminalApp` 会重新挂载 root，并请求下一帧渲染。

## 状态管理建议

简单应用可以用 `Rc<RefCell<State>>`。

规则：

- UI 只读状态。
- 回调发出动作或命令。
- 状态层处理动作。
- 需要重建 UI 时设置一个 changed flag。

示例：

```rust
let state = Rc::new(RefCell::new(AppState::default()));
let changed = Rc::new(Cell::new(false));

Input::new()
    .on_submit({
        let state = state.clone();
        let changed = changed.clone();
        move |cmd| {
            state.borrow_mut().handle_command(&cmd);
            changed.set(true);
        }
    })
    .build(factory, theme)
```

复杂应用再引入更明确的 action enum：

```rust
enum AppAction {
    SubmitCommand(String),
    SelectJob(usize),
    Refresh,
}
```

## 测试建议

使用者也应该写 E2E。不要只手动看终端。

推荐测试：

- 首帧渲染有核心文本。
- Tab 能聚焦到目标输入。
- 输入脚本能更新屏幕。
- resize 后布局仍可读。
- light theme 没有默认黑底。
- placeholder 被输入替换后，尾部背景仍正确。
- 复杂 dashboard 中，List、Table、Footer 同时工作。

选择工具：

- `WidgetHarness`：测单个 widget 或静态页面。
- `TuiTestDriver`：测输入、焦点、resize、runtime。
- `AnsiTuiTestDriver`：测颜色、空白背景、真实 ANSI 输出效果。

颜色相关测试优先用 `AnsiTuiTestDriver`。

## 手动验收清单

每个 TUI 应用至少检查：

- 80x24 可用。
- 120x40 可用。
- 40x12 不 panic，内容能合理裁切。
- light theme 可读。
- dark theme 可读。
- Tab 顺序符合视觉顺序。
- 退出后终端恢复。
- 输入长文本不会撑破布局。
- resize 后不会留下旧画面。
- xterm.js 中空白背景不会露出终端默认色。

## xterm.js 使用建议

xterm.js 对“没有写入的单元格”更敏感。它不会替应用补背景。

使用时注意：

- 后端要输出完整脏行背景。
- 空白区域必须是真实空格，不是“什么都不写”。
- 不要依赖宿主页面背景色。
- light theme 下先测 Header、Nav、Content、Info 和 Footer 的空白区域。
- 如果本地终端正常但 xterm.js 异常，优先查 ANSI 输出，而不是先改主题色。

## 常见应用反模式

| 反模式 | 正确做法 |
| --- | --- |
| 把业务逻辑写在 `build_ui` 里 | `build_ui` 只构建组件树 |
| 只给文字设置背景 | 整个 panel 先填背景 |
| Border 子项 flex，Border 自己不 flex | 需要铺满时 Border 也 flex |
| placeholder 写很长 | placeholder 写短提示 |
| 表格列自适应全部内容 | 先用固定列宽 |
| 只在 dark theme 下看效果 | light/dark 都验收 |
| 只用本地终端验收 | 至少补 ANSI replay；有条件再看 xterm.js |
| 手写终端初始化和事件循环 | 使用 `TerminalApp` 和 `run_crossterm_terminal_app` |
| 退出时不恢复终端 | 让 `TerminalApp` 管理 raw mode、cursor 和 alternate screen |
| resize 后不重建 UI | 使用 `TerminalApp::with_builder` 按新尺寸重建 root tree |

## 最小完成标准

一个 arbor-tui 应用完成前，至少满足：

- 能在真实终端启动和退出。
- 首屏有明确 Header、Body、Footer 或等价结构。
- 所有可见区域有明确背景。
- 键盘主路径可用。
- 80x24 下布局稳定。
- light theme 无默认黑底。
- 有一组 E2E 覆盖核心路径。
- `cargo test` 通过。
