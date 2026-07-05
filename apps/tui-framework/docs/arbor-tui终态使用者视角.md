# arbor-tui 终态使用者视角

本文说明 arbor-tui 从应用使用者角度应该长成什么样。

它不是当前 API 说明。当前用法见 `arbor-tui使用最佳实践.md`。本文描述终态方向，供后续框架收敛时参考。

## 定位

arbor-tui 的终态应是 Arbor 内部工具的 TUI app kit。

它不应只是底层终端绘图库。它也不应复刻完整 React、Solid 或 CSS。

使用者应该用它写终端应用，而不是直接处理终端细节。普通应用作者不应该主要感知这些概念：

- `WidgetFactory`
- `WidgetNode`
- `VirtualScreen`
- `DirtyTracker`
- crossterm backend
- raw mode
- alternate screen
- ANSI emit

这些概念属于框架内核。使用者应该主要感知应用模型：

```text
State -> Action -> update -> view -> run
```

## 当前架子的价值

当前架子已经成立。

它的价值不在具体 widget 是否完整，也不在当前代码没有 bug。它的价值在于应用框架的关键边界已经出现。

当前已有的有效部分：

- `domain / application / adapters / runtime / widgets / composites / testing / examples` 分层清楚。
- `TerminalApp` 已经承担终端生命周期。
- `PromptBar`、`Transcript`、`FuzzyPanel`、`Panel`、`StatusLine` 这类 composites 更接近真实应用需求。
- `WidgetHarness`、`TuiTestDriver`、ANSI replay 测试已经覆盖 TUI 最容易坏的区域。
- `layout_demo2` 和 `aster-rs` 已经能作为真实使用样本。

这说明 arbor-tui 不是只会画字符。它已经有应用组织方式。

## 使用者痛点

当前 API 还偏框架作者视角。

### 入口太散

一个应用现在通常需要同时使用多个 crate：

```rust
arbor_tui_domain
arbor_tui_widgets
arbor_tui_composites
arbor_tui_runtime
```

这暴露了内部结构。终态应该提供一个使用者入口：

```rust
use arbor_tui::prelude::*;
```

### 状态更新太手工

当前真实应用会写出这类结构：

```text
Rc<RefCell<State>> + changed flag + before_events + before_render
```

这能跑，但容易出错。

常见风险：

- 漏设 changed flag。
- 多次重建 root tree。
- 输入事件和业务状态同步不清楚。
- stream、timer、外部 IO 各自发明接入方式。

终态应该提供正式的状态模型。

### 布局和背景容易用错

当前文档需要反复提醒：

- panel 背景要填满。
- light theme 不能漏默认黑底。
- `Border` 自己也要 `.flex(1.0)`。
- 空白区域要真实写空格。
- resize 后要重建 UI。

这些规则说明框架方向是对的，但默认值还不够安全。

终态应该让正确用法变成默认行为。但是底层组件要保证每个属性足够原子。这是有一定gap的，需要框架在上层封装。

### Signal 还不是主要开发模型

当前 `Signal` 是有效的底层机制。它适合 dirty 标记和局部更新。

但应用作者现在主要还是按状态重建 UI。终态不要急着把 arbor-tui 宣传成响应式框架。

更准确的说法是：

> arbor-tui 是保留模式 TUI 应用框架。它支持 signal 驱动的局部更新。

### 自定义 widget 成本偏高

`Widget` trait 能表达完整协议，但它要求使用者理解太多内核概念：

- `measure`
- `measure_subtree`
- `children_rect`
- `render_with_focus`
- `perform`
- mount / unmount

这些能力应该保留给框架作者和高级使用者。普通应用作者应优先使用 composites 和更高层的 component builder。

## 终态模型

终态应该把应用写法收敛成六层。

### 1. 用户入口层

提供统一入口：

```rust
use arbor_tui::prelude::*;
```

普通应用不需要知道 crate 拆分。

高级使用者仍可按需使用底层 crate。

### 2. 应用模型层

推荐模型固定为：

```text
State
Action
update()
view()
Effect / Command
```

简单应用只写 `State`、`Action`、`update` 和 `view`。

复杂应用用 `Effect` 或 `Command` 接入 stream、timer、外部 IO 和后台任务。

方向性 API 示例：

```rust
use arbor_tui::prelude::*;

fn main() -> Result<()> {
    ArborApp::new(AppState::default())
        .theme(Theme::dark())
        .update(update)
        .view(view)
        .run()
}

fn update(state: &mut AppState, action: AppAction, ctx: &mut AppContext<AppAction>) {
    match action {
        AppAction::Submit(text) => state.submit(text),
        AppAction::ToggleTheme => ctx.set_theme(state.next_theme()),
        AppAction::Quit => ctx.quit(),
    }
}

fn view(state: &AppState, ui: &Ui<AppAction>) -> Node<AppAction> {
    ui.component(
        Page::new()
            .header(StatusLine::new("Aster"))
            .body(
                Transcript::new()
                    .messages(state.messages.iter().map(to_transcript_message))
                    .fill(),
            )
            .footer(
                PromptBar::new()
                    .placeholder("Type a message")
                    .on_submit(AppAction::Submit),
            ),
    )
}
```

这个 API 不是当前实现承诺。它表达终态心智模型。

### 3. 组件层

使用者主路径应该是 composites。

推荐一等组件：

- `Page`
- `Panel`
- `StatusLine`
- `PromptBar`
- `Transcript`
- `List`
- `Table`
- `Tabs`
- `Form`
- `FuzzyPanel`
- `ScrollArea`

底层 `Text`、`Border`、`Row`、`Col` 仍然保留。它们用于定制布局和构建新 composite，不应是普通应用的主要负担。

### 4. 自定义组件层

应用作者仍然需要画自己的业务组件。

终态不能只提供预制 composites。它还要提供稳定的自定义组件路径，让使用者能把自定义组件和内置组件拼成一个页面。

自定义组件应分成两类。

第一类是组合型组件。它不直接画 cell，只组合现有组件。

示例：

```rust
struct JobCard {
    props: JobCardProps,
}

struct JobCardProps {
    name: String,
    status_label: String,
    status_color: AnsiColor,
    last_log_line: String,
}

impl PropsComponent<AppAction> for JobCard {
    type Props = JobCardProps;

    fn from_props(props: Self::Props) -> Self {
        Self { props }
    }

    fn into_props(self) -> Self::Props {
        self.props
    }
}

impl UiComponent<AppAction> for JobCard {
    fn render(self, ui: &Ui<AppAction>) -> Node<AppAction> {
        ui.component(
            Panel::new(
                Col::new()
                    .child(TextBlock::new(self.props.status_label).fg(self.props.status_color))
                    .child(TextBlock::new(self.props.last_log_line).dim()),
            )
            .title(self.props.name),
        )
    }
}
```

组合型组件应该是默认推荐方式。它复用布局、主题、焦点、背景和测试能力。

第二类是自绘型组件。它直接按 rect 画字符网格。

适合场景：

- sparkline。
- 进度条。
- 状态灯矩阵。
- 拓扑图。
- 小型甘特图。
- 终端专用仪表盘。
- 特定业务的紧凑表格。

终态应提供比裸 `Widget` trait 更顺手的自绘接口。

方向性 API 示例：

```rust
struct Sparkline {
    values: Vec<f64>,
}

impl Component<AppAction> for Sparkline {
    fn measure(&self, _ctx: &MeasureCtx) -> SizeHint {
        SizeHint::fixed_height(3).min_width(12)
    }

    fn draw(&self, frame: &mut Frame, rect: Rect, theme: &Theme) {
        frame.fill(rect, theme.surface());
        frame.sparkline(rect, &self.values, theme.accent());
    }
}
```

这里的 `Frame` 是安全绘制上下文。它负责边界裁切、宽字符、背景填充和 ANSI cell 语义。

自绘组件不应该直接构造 `VirtualScreen`，也不应该直接操作 backend。

自定义组件需要满足这些规则：

- `measure` 只声明尺寸需求。
- `draw` 必须画满拿到的 rect，或者显式声明 transparent。
- 默认背景来自 theme 或父容器。
- 组件只发出 `Action`，不直接改应用主状态。
- 自绘组件可以嵌入 `row`、`col`、`panel`、`tabs`、`scroll_area`。
- 测试可以用同一个 `TestApp` 和屏幕断言覆盖。

拼装方式应和内置组件一致：

```rust
fn view(state: &AppState, ui: &mut Ui) -> Node<Action> {
    ui.component(
        Page::new()
        .body(
            Row::new()
                .child(Panel::new(CpuGauge::from_state(&state.cpu)).title("CPU"))
                .child(
                    Panel::new(
                        Transcript::new()
                            .messages(state.logs.iter().map(to_transcript_message))
                            .fill(),
                    )
                    .title("Logs"),
                )
        )
        .footer(PromptBar::new().placeholder("command").on_submit(Action::Submit))
    )
}
```

这个能力很关键。

如果没有稳定的自定义组件路径，arbor-tui 会变成只能拼预制块的 demo 框架。真实运维工具一定会有业务图形、紧凑状态块和特殊交互。

### 5. 布局和主题默认安全层

终态默认值应该尽量防止画坏屏幕。

默认要求：

- 每个组件填满自己的 rect。
- 每个可见 cell 有明确背景。
- light theme 不露默认黑底。
- resize 后框架自动触发必要重建。
- panel、prompt、transcript 默认继承合理背景。
- `.fill()` 和 `.flex()` 的语义清楚，不靠文档猜。

正确用法应短：

```rust
ui.component(
    Panel::new(JobTable::from_jobs(&state.jobs))
        .title("Jobs")
        .fill(),
)
```

### 6. 测试和调试层

测试能力应成为框架能力的一部分。

终态测试应面向应用行为：

```rust
#[test]
fn submit_prompt_starts_streaming() {
    let mut app = TestApp::new(AppState::default(), update, view);

    app.render(80, 24).assert_text("Arbor Agent Console");
    app.type_text("fix layout").press(Key::Enter);

    app.assert_text("fix layout");
    app.assert_text("Running");
    app.assert_no_default_bg();
}
```

测试入口应该覆盖：

- 首帧文本。
- Tab 顺序。
- 输入提交。
- resize。
- light / dark theme。
- ANSI replay。
- 默认背景泄漏。
- stream 和后台任务状态。

## 应用用例：Agent Console

一个合适的目标应用是本地 Agent 运行面板。

它不是聊天玩具。它是日常终端工作台。

界面形态：

```text
┌ Arbor Agent Console ─────────────────────────────┐
│ Session: aster-rs   Model: deepseek   Status: Run │
├──────────────┬───────────────────────┬────────────┤
│ Tasks        │ Transcript             │ Context    │
│ > build ui   │ You: fix layout         │ Files      │
│   run tests  │ Agent: running tests... │ Cargo.toml │
│   review     │                         │ ui.rs      │
├──────────────┴───────────────────────┴────────────┤
│ › ask agent / run test / switch task               │
└────────────────────────────────────────────────────┘
```

应用状态：

```rust
struct AppState {
    tasks: Vec<Task>,
    selected_task: usize,
    messages: Vec<Message>,
    context_files: Vec<PathBuf>,
    input_mode: InputMode,
    running: bool,
    error: Option<String>,
}

enum Action {
    SubmitPrompt(String),
    SelectNextTask,
    SelectPrevTask,
    ToggleRunning,
    StreamToken(String),
    StreamDone,
    DismissError,
    Quit,
}
```

终态使用方式：

```rust
use arbor_tui::prelude::*;

fn main() -> Result<()> {
    ArborApp::new(AppState::load()?)
        .theme(Theme::dark())
        .update(update)
        .view(view)
        .run()
}

fn update(state: &mut AppState, action: Action, ctx: &mut AppContext<Action>) {
    match action {
        Action::SubmitPrompt(text) => {
            state.messages.push(Message::user(text.clone()));
            state.running = true;
            ctx.spawn_stream(agent_stream(text), Action::StreamToken, Action::StreamDone);
        }
        Action::SelectNextTask => state.select_next_task(),
        Action::SelectPrevTask => state.select_prev_task(),
        Action::StreamToken(token) => state.append_agent_token(token),
        Action::StreamDone => state.running = false,
        Action::DismissError => state.error = None,
        Action::Quit => ctx.quit(),
        Action::ToggleRunning => state.running = !state.running,
    }
}

fn view(state: &AppState, ui: &Ui<Action>) -> Node<Action> {
    ui.component(
        Page::new()
        .title("Arbor Agent Console")
        .header(StatusLine::new(format!(
            "Task: {}  Status: {}",
            state.current_task_name(),
            if state.running { "Running" } else { "Idle" }
        )))
        .body(
            Row::new()
                .child(TaskList::new(state.tasks.clone()).selected(state.selected_task).width(24))
                .child(
                    Transcript::new()
                        .messages(state.messages.iter().map(to_transcript_message))
                        .fill(),
                )
                .child(FileContext::new(state.context_files.clone()).width(28))
        )
        .footer(
            PromptBar::new()
                .placeholder("ask agent / run test / switch task")
                .loading(state.running)
                .on_submit(Action::SubmitPrompt)
        )
    )
}
```

这个用例说明 arbor-tui 的目标不是帮应用画框线。

目标是让应用作者快速获得这些能力：

- 有状态。
- 有输入。
- 有后台任务。
- 有 stream 输出。
- 有滚动区域。
- 有键盘主路径。
- 有 resize 支持。
- 有主题。
- 有 E2E 测试。
- 退出时终端恢复。

## 演进重点

后续演进应优先打磨黄金路径。

优先级如下：

1. 提供 facade 或 prelude，隐藏常规 crate 拆分。
2. 固化 `State / Action / update / view` 应用模型。
3. 提供组合型组件和自绘型组件两条稳定扩展路径。
4. 把 `before_events`、`before_render`、changed flag 收敛成更高层 API。
5. 强化 composites，让普通应用少写裸 `Border`、`Row`、`Col`。
6. 让背景、fill、resize 的默认行为更安全。
7. 把 `layout_demo2`、`aster-rs` 和 Agent Console 作为 API 验收样本。
8. 把应用级测试 driver 做成推荐入口。

具体 widget bug 可以逐步修。它们不应改变终态方向。

## 非目标

arbor-tui 不应追求所有场景。

明确非目标：

- 不做复杂鼠标 UI。
- 不做 IME 重度输入。
- 不做像素级图形。
- 不做完整 CSS。
- 不做完整 React/Solid 响应式系统。
- 不把所有底层能力暴露给普通应用作者。
- 不为了成为通用 crate 牺牲 Arbor 内部工具效率。

## 完成标准

当一个普通 Arbor TUI 应用满足下面条件时，arbor-tui 的终态方向才算实现：

- 应用入口只需要一个 prelude。
- 应用主逻辑能表达为 `State / Action / update / view`。
- 普通页面主要使用 composites。
- 业务组件可以用组合型组件表达。
- 特殊业务视图可以用安全 `Frame` 自绘，并能和内置组件拼装。
- 应用不手写终端生命周期。
- 应用不直接处理 ANSI 输出。
- light theme 默认不露黑底。
- resize 不需要应用作者手写重复逻辑。
- stream 和后台任务能通过框架上下文接入。
- 应用测试能覆盖输入、渲染、主题和 resize。

这时 arbor-tui 才从“框架作者能用”变成“应用作者不容易用错”。
