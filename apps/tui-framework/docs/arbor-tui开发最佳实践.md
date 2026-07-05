# arbor-tui 开发最佳实践

本文写日常开发规则。设计依据是当前代码、TEP-0007 和最近的 E2E 结果。

如果旧 TEP、旧测试和当前代码冲突，以当前代码和 TEP-0007 为准。这个项目仍处在 v0。终态正确优先，不为了旧接口保兼容。

## 先判断改哪一层

改动前先确定归属层。不要把逻辑放到“顺手能拿到数据”的地方。

| 目标 | 应改层 | 不要放到 |
| --- | --- | --- |
| Cell、VirtualScreen、布局计算、diff、Signal、Widget 协议 | `arbor-tui-domain` | adapters、examples |
| runtime step、focus、dirty、resize、render 调度 | `arbor-tui-application` | widgets、adapters |
| crossterm 输出、真实输入、模拟后端、ANSI 编码 | `arbor-tui-adapters` | domain、widgets |
| Text/Input/List/Table/Tabs/Border/Stack 等内置组件 | `arbor-tui-widgets` | application、adapters |
| 高层启动入口，组合 application 和 adapters | `arbor-tui-runtime` | domain、application、adapters |
| E2E driver、ANSI replay、WidgetHarness、断言工具 | `arbor-tui-testing` | examples、production crates |
| demo 和手动验证入口 | `arbor-tui-examples` | core crates |

依赖方向必须保持：

```text
domain <- application
domain <- adapters
domain <- widgets
application/adapters <- runtime
domain/application/adapters/widgets <- testing
runtime/domain/widgets/composites <- examples
```

`application` 不能正式依赖 `adapters` 或 `widgets`。测试里可以用。
`adapters` 也不能正式依赖 `application`。需要一行启动真实终端时，放到 `arbor-tui-runtime`。

## 修改前先补测试入口

TUI 比 GUI 更适合深度 E2E。多数行为都能用脚本输入和屏幕断言覆盖。

优先选择测试层级：

| 行为 | 测试入口 |
| --- | --- |
| 纯函数、尺寸计算、状态机 | 对应 crate 的 unit test |
| 单个 widget 静态渲染 | `WidgetHarness` |
| 输入、焦点、事件冒泡、resize、dirty、runtime step | `TuiTestDriver` |
| 真实 ANSI 输出、颜色、空白背景、终端解释差异 | `AnsiTuiTestDriver` 或 adapter 输出测试 |
| 示例页面布局和 light/dark theme 效果 | examples crate 的 smoke test |

不要只测 `VirtualScreen`。颜色和空白背景必须至少有一条测试经过真实 ANSI 编码或 ANSI replay。

## 渲染规则

渲染的目标是“屏幕上每个可见区域都有明确 Cell”。不要让终端默认色参与 UI。

必须遵守：

- 每个 widget 拿到 `rect` 后，要填满自己的 `VirtualScreen`。
- 空白区域也要有明确背景色。
- light theme 下，可见文字不能落在默认黑底上。
- Border、RichText、Input 这类带背景的组件，先 fill rect，再写文本。
- 宽字符第二列必须标记 `phantom`。backend emit 时跳过 phantom。

不要做：

- 不要只写文本，不填文本右侧和下方空白。
- 不要依赖终端默认背景色。
- 不要用 `ClearUntilNewLine` 补背景。它会把 dirty region 外的区域刷成终端默认语义。
- 不要为了省输出量牺牲背景正确性。

当前 crossterm backend 的输出原则：

- diff 仍然找 dirty region。
- backend 对出现 dirty region 的行做整行重绘。
- 连续同样式 cell 批量输出。
- 空白背景也输出真实空格。
- 每个 run 结束后 reset，不在每个 cell 后 reset。

这个规则是为了兼容 xterm.js、Windows Terminal、WezTerm 等不同终端前端。终端不会替框架填背景。

## 布局规则

布局是两趟：

```text
measure: 子组件报告尺寸约束
layout: 父组件分配实际 rect
```

写组件时要区分两件事：

- `measure` 只说明最小需求和约束。
- `render` 必须按实际 `rect` 画满。

常见规则：

- 想填满剩余空间，用 `.flex(1.0)`。
- 只想固定宽度，用 `.width(n)`。
- 三栏布局里，左右固定宽度，中间 `flex(1.0)`。
- header/body/footer 里，body 通常 `flex(1.0)`。
- Border 想铺满父级高度，Border 本身也要有 `flex(1.0)`，不是只给子容器 flex。
- 文案不要把中心栏最小宽度撑爆。终端 demo 要优先适配 80 列。

不要把布局问题交给渲染层兜底。组件没有拿到高度，就不可能画满那块高度。

## 组件规则

组件只做自己的事。

必须遵守：

- 组件通过 `WidgetAction` 接收动作。
- 可获焦组件实现 `focusable()`。
- 事件处理返回 `Handled` 或 `Bubble`。
- 内部编辑态可以放组件内，例如 Input 的 buffer 和 cursor。
- 外部状态用 `Signal` / `ReadSignal`。
- 持有 `ReadSignal` 的组件，在 `on_mount` 订阅，在 `on_unmount` 和 `Drop` 退订。

不要做：

- 不要让 widget 直接操作 `App`。
- 不要让 widget 依赖 adapters。
- 不要在 widget 内读真实终端尺寸。
- 不要用历史名字描述新语义。布局容器叫 `StackWidget`，不是 `BoxWidget`。
- 不要让 `ReadSignal` 在组件内部被替换成新的常量信号。

## 状态规则

运行时状态只能通过明确入口改变：

```text
App + RuntimeInput -> RuntimeStepResult
```

事件循环只做三件事：

1. 从输入 adapter 读事件。
2. 调用 `runtime_step`。
3. 按结果 clear、render、emit 或 quit。

新增 runtime 行为时，优先改 `runtime_step` 或 `App` 的明确方法。不要把焦点、dirty、resize、quit 状态分散到多个临时入口里。

## 颜色规则

颜色是 UI 语义，不是终端配置。

必须遵守：

- 每个 Cell 都要有明确 `fg` 和 `bg`。
- light theme 不能露出默认黑底。
- 组件背景和 span 背景要一致，除非明确需要高亮。
- selected row、cursor、danger、warning 等状态必须用 theme 语义色。
- `NO_COLOR=1` 只在 backend 层处理。组件仍然正常写颜色语义。

复杂场景必须测：

- light theme。
- 嵌套 Border + Tabs + Input。
- placeholder 被短文本替换。
- 选中行和 footer 同时更新。
- 局部更新后，旧背景不能泄漏。

## 新增 widget 流程

新增 widget 按这个顺序做：

1. 在 `arbor-tui-widgets` 中建 builder 和 widget 实现。
2. 在 builder 中只接收配置，不保存 runtime 主状态。
3. 在 widget 中实现 `measure` 和 `render`。
4. 如果有子组件，实现 `children()`、`children_mut()` 和 `children_rect()`。
5. 如果可获焦，实现 `focusable()`、`render_focused()` 和 `perform()`。
6. 如果持有 signal，实现 mount/unmount/drop 订阅退订。
7. 用 `WidgetHarness` 测静态渲染。
8. 用 `TuiTestDriver` 测焦点、按键、事件冒泡和 dirty。
9. 如果涉及颜色或空白背景，用 `AnsiTuiTestDriver` 或 adapter 输出测试。
10. 更新文档或 TEP。

完成标准：

- light theme 下可见文字没有默认黑底。
- resize 后布局合理。
- 空闲 tick 不 emit 新输出。
- 焦点切换只重绘必要组件。
- `cargo test` 通过。

## 修改 backend 流程

backend 是最容易被普通模拟测试漏掉的地方。

改 backend 时必须覆盖：

- 是否写入 foreground。
- 是否写入 background。
- 是否写入 attrs。
- 空白 cell 是否真实输出。
- partial dirty region 是否会污染 region 外区域。
- ANSI replay 后屏幕是否与期望一致。
- `NO_COLOR=1` 是否仍然工作。

不要只看内存屏幕。`SimulatedBackend` 会直接 blit dirty cell，不能代表真实终端。

## 修改 layout_demo2 流程

`layout_demo2` 是手动验收入口，不是随便堆组件的 playground。

它要满足：

- 80 列可读。
- light theme 不刺眼。
- header、body、footer 都在首屏。
- Nav、Content、Info 三栏高度一致。
- footer input 能切换 theme。
- 所有 panel 背景一致。
- 终端默认背景不参与 UI。

改 demo 后至少跑：

```powershell
cargo test -p arbor-tui-examples --bin layout_demo2
```

然后在真实终端手动跑：

```powershell
cargo run -p arbor-tui-examples --bin layout_demo2
```

至少看三种环境中的一种：

- Windows Terminal + PowerShell 7。
- WezTerm。
- xterm.js。

如果 xterm.js 和本地终端不一致，优先怀疑输出层和空白背景，不要先归因到主题。

## 文档规则

文档分工：

- TEP 写设计决策和长期协议。
- 偏差分析写设计与实现差距。
- 最佳实践写日常操作规则。
- 示例注释只写必要上下文，不重复解释代码。

文档要直接写规则和完成标准。不要写泛泛原则。

改架构、测试策略或输出语义时，至少检查：

- `docs/TEPs/TEP-0007-终态分层与测试策略.md`
- `docs/设计实现偏差分析.md`
- `docs/arbor-tui开发最佳实践.md`

## 提交前检查

常规改动跑：

```powershell
cargo fmt --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features
cargo test
```

只改某个行为时，先跑目标测试，再跑全量检查。

如果改了输出层，必须至少跑：

```powershell
cargo test -p arbor-tui-adapters
cargo test -p arbor-tui-testing --test color_e2e
```

如果改了 demo，必须至少跑：

```powershell
cargo test -p arbor-tui-examples --bin layout_demo2
```

## 常见反模式

| 反模式 | 正确做法 |
| --- | --- |
| 为了少改代码保留旧 API | 直接改到终态，更新调用方 |
| 只测组件内存屏幕 | 对颜色和输出补 ANSI replay 或 adapter 测试 |
| 空白区域不填背景 | 先 fill rect，再写内容 |
| 用 clear line 修背景 | 输出真实空格，避免终端默认色 |
| 每个 cell 后 reset | 同样式 run 批量输出，run 后 reset |
| Border 子项 flex，但 Border 自己不 flex | 需要铺满时 Border 本身也 flex |
| widget 直接依赖 crossterm | 通过 domain port 和 adapters |
| runtime 状态散落在 widget 回调里 | 通过 `runtime_step` 和 `App` 明确入口 |
| 文档只写原则 | 写具体规则、文件、命令和完成标准 |
