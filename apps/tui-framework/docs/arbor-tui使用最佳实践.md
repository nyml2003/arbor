# arbor-tui 使用最佳实践

本文面向使用 arbor-tui 写应用的人。

普通应用只使用 facade crate：

```rust
use arbor_tui::prelude::*;
```

不要在业务页面里直接使用 `WidgetFactory`。不要写 `.build(factory, theme)`。

## 推荐模型

应用按这个结构组织：

```text
State -> Action -> update -> view -> ArborApp::run()
```

最小结构：

```rust
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
        AppAction::Quit => ctx.quit(),
    }
}

fn view(state: &AppState, ui: &Ui<AppAction>) -> Node<AppAction> {
    ui.component(
        Page::new()
            .title("Arbor Agent Console")
            .body(TextBlock::new(state.status_text()))
            .footer(
                PromptBar::new()
                    .placeholder("ask agent")
                    .on_submit(AppAction::Submit),
            ),
    )
}
```

`Ui` 的普通入口只有两个：

- `ui.component(component)`：把声明式组件渲染成 `Node<Action>`。
- `ui.theme()`：读取当前主题，用于选择业务语义颜色。

`Ui` 不公开 `factory()`。`factory` 是框架内部实现细节。

## 组件模型

所有内置组件、布局组件和业务自定义组件都实现 `UiComponent<Action>`。

推荐写法：

```rust
ui.component(
    Panel::new(
        Col::new()
            .fill()
            .child(TextBlock::new("Status: running"))
            .child(
                Input::new()
                    .placeholder("type command")
                    .on_submit(AppAction::Submit),
            ),
    )
    .title(" Console ")
    .fill(),
)
```

规则：

- 页面入口统一用 `ui.component(...)`。
- 不新增 `ui.input()`、`ui.panel()`、`ui.col()` 这类快捷方法。
- 组件对象只保存声明式参数、数据和 action mapper。
- 组件可以在 `render` 阶段读取 `ui.theme()`。
- 组件不能保存或依赖 `WidgetFactory`。

## Component 协议和 Props 生命周期

`UiComponent<Action>` 是应用层唯一组件协议：

```rust
pub trait UiComponent<Action>: 'static {
    fn render(self, ui: &Ui<Action>) -> Node<Action>;
}
```

生命周期固定为：

1. `view` 读取 `State`。
2. `view` 从 `State` 创建 owned props。
3. `ui.component(component)` 消费 component。
4. `render` 读取 `ui.theme()` 并返回 `Node<Action>`。
5. 子组件回调发出 `Action`。
6. `update` 处理 `Action` 并修改 `State`。
7. 下一帧重新创建 component tree。

Props 规则：

- Props 是每帧的声明式快照。
- Props 必须是拥有型数据，或满足 `'static`。
- 不要把 `&State`、`&mut State`、`&Theme` 存进 props。
- 需要文本时传 `String`，不要借用临时 `&str`。
- 需要列表时传 `Vec<T>`，不要让组件保存业务集合引用。
- 需要回调时传 `Fn(input) -> Action` mapper。
- 持久状态放在应用 `State`，不要放在 component 对象里。

`ComponentProps` 是 marker trait。它表示 props 可以安全进入嵌套 component tree：

```rust
pub trait ComponentProps: 'static {}
```

通常不需要手动实现。拥有型 props 会自动满足这个约束。

内置 facade 组件都按同一协议导出：

```text
TextBlock      + TextBlockProps
StatusLine     + StatusLineProps
Input          + InputProps<Action>
PromptBar      + PromptBarProps<Action>
FuzzyPanel     + FuzzyPanelProps<Action>
Transcript     + TranscriptProps
Col / Row      + ColProps<Action> / RowProps<Action>
Panel          + PanelProps<Action>
Page           + PageProps<Action>
```

这些组件都实现 `PropsComponent<Action>`。构造器只是 props builder 的快捷写法。

```rust
let props = TextBlockProps::new("ready");
let node = ui.component(TextBlock::from_props(props));
```

业务组件也应该采用同样形态：

```rust
struct JobCard {
    props: JobCardProps,
}

struct JobCardProps {
    title: String,
    status: String,
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
```

## 业务自定义组件

业务自定义组件实现 `PropsComponent<Action>` 和 `UiComponent<Action>`。

示例：

```rust
struct ChatInput {
    props: ChatInputProps,
}

struct ChatInputProps {
    draft: String,
    loading: bool,
}

impl PropsComponent<AppAction> for ChatInput {
    type Props = ChatInputProps;

    fn from_props(props: Self::Props) -> Self {
        Self { props }
    }

    fn into_props(self) -> Self::Props {
        self.props
    }
}

impl UiComponent<AppAction> for ChatInput {
    fn render(self, ui: &Ui<AppAction>) -> Node<AppAction> {
        let theme = ui.theme();

        ui.component(
            Panel::new(
                Input::new()
                    .value(self.props.draft)
                    .placeholder("Type a message")
                    .loading(self.props.loading)
                    .on_submit(AppAction::Submit),
            )
            .fg(theme.border())
            .bg(theme.surface()),
        )
    }
}
```

业务组件适合做：

- 页面局部区域。
- 业务数据到 UI 组件的映射。
- 状态文案和颜色选择。
- 多个 facade component 的组合。

不适合在业务组件里做：

- 直接创建 `WidgetFactory`。
- 调用 raw widget 的 `.build(factory, theme)`。
- 解析终端尺寸或轮询输入。
- 把复杂业务逻辑塞进组件回调。

如果业务需要一个新底层 widget，优先在框架层补 facade component。不要让应用 crate 绕过 facade。

## 状态和回调

组件回调只产生 `Action`。

```rust
Input::new()
    .value(state.draft.clone())
    .placeholder("command")
    .on_change(AppAction::DraftChanged)
    .on_submit(AppAction::Submit)
```

状态修改放在 `update`：

```rust
fn update(state: &mut AppState, action: AppAction, ctx: &mut AppContext<AppAction>) {
    match action {
        AppAction::DraftChanged(text) => state.draft = text,
        AppAction::Submit(text) => state.submit(text),
        AppAction::ThemeLight => ctx.set_theme(Theme::light()),
        AppAction::Quit => ctx.quit(),
    }
}
```

规则：

- UI 只读状态。
- 回调只发 action。
- `update` 修改状态。
- 主题切换用 `ctx.set_theme(...)`。
- 退出用 `ctx.quit()`。

## 布局实践

常用页面：

```text
Page
├── Header
├── Body (flex)
└── Footer
```

常用三栏：

```text
Row
├── Left  (fixed width)
├── Main  (flex)
└── Right (fixed width)
```

示例：

```rust
Page::new()
    .title("Dashboard")
    .body(
        Row::new()
            .fill()
            .child(
                Col::new()
                    .width(24)
                    .child(Panel::new(TextBlock::new("Nav")).title(" Nav ").fill()),
            )
            .child(Panel::new(TextBlock::new("Content")).title(" Main ").fill())
            .child(
                Col::new()
                    .width(28)
                    .child(Panel::new(TextBlock::new("Info")).title(" Info ").fill()),
            ),
    )
    .footer(StatusLine::new("Enter: submit  Esc: quit"))
```

规则：

- 主内容区域通常 `.fill()`。
- 需要固定列时用 `.width(n)`。
- Panel 要铺满父级剩余空间时，Panel 自己也要 `.fill()`。
- 窄屏下文案要能裁切，不要靠长 placeholder 撑布局。
- 不在业务代码里手写 box drawing 字符串。

## 颜色实践

使用 `Theme` 的语义色：

- 普通文字：`theme.text()`。
- 次要文字：`theme.text_dim()`。
- 页面背景：`theme.surface()`。
- 焦点和主操作：`theme.primary()`。
- 选中和高亮：`theme.accent()`。
- 成功：`theme.success()`。
- 警告：`theme.warning()`。
- 危险：`theme.danger()`。
- 边框：`theme.border()`。

规则：

- 不用裸 palette index 表达业务含义。
- light theme 下必须检查默认黑底。
- Panel、Input、选中行和空白区域都要有明确背景。

## 常用组件

### TextBlock

用于短文本、状态值和普通多行文本。

```rust
TextBlock::new("ready")
    .fg(theme.success())
    .bg(theme.surface())
```

### Panel

用于有边框的区域。

```rust
Panel::new(TextBlock::new("Logs"))
    .title(" Logs ")
    .fg(theme.border())
    .bg(theme.surface())
    .fill()
```

### Input 和 PromptBar

`Input` 用于单行输入。`PromptBar` 用于 footer 命令栏。

```rust
PromptBar::new()
    .placeholder("ask agent")
    .loading(state.waiting_for_agent)
    .loading_phase(state.loading_phase)
    .on_submit(AppAction::Submit)
```

规则：

- loading 态用于等待 Agent 或后端回复。
- loading 态要由状态层推进 `loading_phase`。
- placeholder 写短提示，不写帮助文档。
- password 输入使用 `Input::password()`。

### Transcript

用于聊天记录、Agent 输出和 Markdown 消息流。

```rust
Transcript::new()
    .messages(messages.iter().map(|message| {
        TranscriptMessage::new(message.role_label(), theme.primary(), message.body())
    }))
    .empty_text("No messages")
    .scroll_y(scroll_y)
    .bg(theme.surface())
    .fill()
```

规则：

- 应用只负责把业务消息转换成 `TranscriptMessage`。
- 错误和中断提示用 `TranscriptNotice`。
- 不在应用里重复写 Markdown 解析、代码块边框和行数估算。

## Advanced API

`arbor_tui::advanced` 只给框架内部、迁移代码和底层 widget 作者使用。

普通应用不要从 `advanced` 导入 raw widget 来拼页面。

如果必须写 raw widget：

- 把它放在框架层或独立组件 crate。
- 同时提供 facade `UiComponent<Action>` adapter。
- 业务页面继续通过 `ui.component(...)` 使用。

## 测试建议

使用者也应该写 E2E。

推荐覆盖：

- 首帧渲染核心文本。
- 输入脚本能更新状态。
- loading 态可见。
- resize 后布局仍可读。
- light theme 没有默认黑底。
- 主题切换后组件读取新主题。

常用工具：

- `TestApp`：测试 facade 应用。
- `WidgetHarness`：测试单个 raw widget 或静态页面。
- `TuiTestDriver`：测试输入、焦点、resize 和 runtime。
- `AnsiTuiTestDriver`：测试颜色和 ANSI 输出。

## 常见反模式

| 反模式 | 正确做法 |
| --- | --- |
| 在业务页面拿 `ui.factory()` | 用 `ui.component(component)` |
| 写 `.build(factory, theme)` | 写 facade `UiComponent` |
| 增加 `ui.xxx()` 快捷入口 | 增加 component 类型 |
| 业务组件保存 `Theme` 引用 | 在 `render` 阶段读 `ui.theme()` |
| 回调里写复杂业务逻辑 | 回调只返回 `Action` |
| 手写终端初始化和事件循环 | 使用 `ArborApp::run()` |
| 手写 box drawing 字符串 | 使用框架组件或补 facade component |
| 只在 dark theme 下看效果 | light/dark 都验收 |

## 最小完成标准

一个 arbor-tui 应用完成前，至少满足：

- 能启动和退出。
- 首屏有明确 Header、Body、Footer 或等价结构。
- 所有可见区域有明确背景。
- 键盘主路径可用。
- 80x24 下布局稳定。
- light theme 无默认黑底。
- 核心路径有测试。
- 相关 `cargo test` 通过。
