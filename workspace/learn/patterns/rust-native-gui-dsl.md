# Rust Native GUI DSL

## 场景

这个模式适合不需要 WebView 的桌面小工具。

典型例子：

- 虚拟键盘
- 截图 overlay
- 悬浮控制条
- 托盘工具面板
- 常驻系统小窗口

这些工具的核心压力不是复杂内容排版，而是窗口行为、输入、DPI、低开销渲染和平台能力边界。

## 核心结构

```text
packages/
  arbor-ui-core      # geometry/event/theme/component DSL/primitive tree
  arbor-ui-windows   # Windows Direct2D/DirectWrite renderer

app/
  state          # 业务状态机
  layout         # 逻辑像素布局
  input          # 平台无关输入命令
  view           # 组合 arbor-ui-core primitive tree

platform/
  windows        # Win32 host / DPI / message loop / SendInput
  macos          # 后续适配
  linux          # 后续适配
```

app 层只生成不可变视图快照。`arbor-ui-windows` 消费快照并绘制，应用自己的 platform 层保留窗口生命周期、DPI、消息循环和输入注入。

## Rust DSL

组件 DSL 是安全 Rust 的 builder 层。

示例：

```rust
surface("keyboard-surface", rect)
    .background(ColorToken::Surface)
    .children([
        row("title", title_rect)
            .children([
                text("title-text", title_rect).content("KeyDock").build(),
                button("close", close_rect).child(image("close-icon", close_rect).build()).build(),
            ])
            .build(),
    ])
    .build()
```

DSL 的输出不是平台控件。输出是 `Primitive` tree。

## Primitive Tree

primitive tree 是 app 层和平台层之间的合同。

常见节点：

- `Surface`
- `Row`
- `Button`
- `Text`
- `Image`

节点只描述要画什么，不持有窗口句柄、COM 对象、系统资源或平台错误。

## Platform Adapter

平台适配层负责：

- 创建窗口
- 接收系统消息
- 把指针事件转换为 app 事件
- 把 `Primitive` tree 画到屏幕
- 把平台无关输入命令转换为系统输入
- 处理 DPI 和窗口生命周期

Windows 当前样本拆成两层：

- app platform host：Win32 窗口、DPI、消息循环、`SendInput`
- `arbor-ui-windows`：Direct2D、DirectWrite、renderer resource cache

## Unsafe 边界

`unsafe` 只能出现在平台适配层。

允许区域：

- 窗口类注册
- window proc
- 消息循环
- Direct2D / DirectWrite COM 调用
- 输入注入
- 必要的 Win32 union 字段写入

禁止区域：

- app 状态机
- 组件 DSL
- 布局计算
- 命中测试
- 输入命令生成

## 和 Electron / Tauri 的关系

Electron + SolidJS 适合 Arbor 主容器。它适合复杂内容、文件树、Markdown 预览、丰富 Web UI 和快速迭代。

Tauri 适合 Web UI 加 Rust 能力。截图工具就是这个方向：overlay 可以静态，settings 可以用 SolidJS。

Rust Native GUI 适合完全不需要 WebView 的工具。它牺牲 Web 生态，换来更小的运行时、更直接的平台控制和更清楚的系统 API 边界。

## 反模式警示

- 不要在 app 层引入 `windows::Win32::*`
- 不要让组件节点持有平台资源
- 不要把渲染行为塞进业务组件 trait
- 不要为了“未来跨平台”提前抽空 adapter
- 不要把 host、输入注入和业务窗口策略塞进 `arbor-ui-windows`

## 来源

来源项目：KeyDock、ClipDock。

KeyDock 当前验证了 Windows 虚拟键盘路线：`arbor-ui-core` 提供安全 Rust 组件 DSL 和 primitive tree，`arbor-ui-windows` 负责 Direct2D/DirectWrite 渲染，KeyDock 的 Windows host 负责窗口、DPI、`SendInput` 输入注入和 unsafe host 边界。

ClipDock 继续验证第二应用路线：业务层只维护文本剪贴板历史、布局、命中测试和 `AppCommand`，Windows host 负责 `AddClipboardFormatListener` / `WM_CLIPBOARDUPDATE`、`CF_UNICODETEXT` 读写和 `Ctrl+V` 输入模拟。这个样本说明 `arbor-ui-core` 与 `arbor-ui-windows` 已经可以服务不同行为域，但 host crate 还没有足够压力，暂时不抽。
