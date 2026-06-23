# KeyDock 技术架构

## 技术路线

- 语言：Rust stable
- 目标：`x86_64-pc-windows-msvc`
- 平台：Windows 10 22H2+ / Windows 11
- API：Win32 API via `windows` crate
- 渲染：Direct2D + DirectWrite
- 输入注入：`SendInput`

KeyDock v1 是 Windows-only 原生工具，不引入 GUI 框架，不引入 WebView。

它也是 Arbor Native GUI 储备线的第一份样本。当前已经把通用 UI 模型拆到 `packages/arbor-ui-core`，把 Windows Direct2D/DirectWrite 渲染适配拆到 `packages/arbor-ui-windows`。KeyDock 自己保留键盘业务、Win32 host、DPI、消息循环和 `SendInput` 输入注入。

## 为什么不用老 API

- 输入注入使用 `SendInput`，不用 `keybd_event`
- 主渲染使用 Direct2D/DirectWrite，不把 GDI 作为主绘制路径
- DPI 使用 Per-Monitor DPI v2，不按固定 96 DPI 写死
- 触控/鼠标统一走指针或鼠标消息语义，不依赖旧式键盘钩子

## 模块边界

```text
packages/
  arbor-ui-core/        # geometry/event/theme/ViewSnapshot/component DSL/primitive tree
  arbor-ui-windows/     # Direct2D/DirectWrite renderer for arbor-ui-core snapshots

apps/keydock/src/
  app/                  # keyboard layout/state/input commands/view composition
  platform/windows/     # Win32 window host, DPI, message loop, SendInput, diagnostics
  main.rs
```

## app 层

纯安全 Rust。

职责：

- 键盘布局
- 状态机
- 命中测试
- 输入命令生成
- 使用 `arbor-ui-core` 生成 primitive tree 和渲染快照

禁止：

- `unsafe`
- `windows::Win32::*`
- `HWND`
- `WPARAM`
- `LPARAM`
- COM 类型
- `SendInput`

### arbor-ui-core

`arbor-ui-core` 是 app 层和平台渲染层之间的边界。

它包含两类内容：

- Rust DSL：`button()`、`text()`、`image()`、`row()`、`surface()`
- 平台无关绘制描述：`Primitive` tree

- `Surface`
- `Row`
- `Button`
- `Text`
- `Image`

规则：

- `ComponentNode` 只抽 `id` 和 `rect` 这类真实共性，不抽渲染行为
- `Button` 只描述可点击视觉状态，不直接产生输入命令
- `Text` 只描述文字和样式 token，不持有 DirectWrite 对象
- `Image` 只引用内置资源 ID，不持有文件路径、解码结果或 Direct2D bitmap。当前资源 ID 使用组件 `id`
- components 不知道窗口句柄、DPI API、COM 或 Win32 错误码

## platform::windows 层

KeyDock 自己的 Windows host 边界。

职责：

- 设置 DPI awareness
- 注册窗口类
- 创建不抢焦点窗口
- 运行消息循环
- 转换 Win32 输入消息为 app 层事件
- 调用 `arbor-ui-windows::Renderer` 绘制快照
- 调用 `SendInput` 执行输入命令
- 把 Win32 错误转换为 `PlatformError`

## arbor-ui-windows 层

`arbor-ui-windows` 是共享 Windows renderer crate。

职责：

- 接收 `arbor_ui_core::ViewSnapshot`
- 创建和缓存 Direct2D brush
- 创建和缓存 DirectWrite text format
- 把 `Surface`、`Row`、`Button`、`Text`、`Image` 映射到 Direct2D/DirectWrite 调用

不负责：

- 创建窗口
- 处理 Win32 消息
- DPI 策略
- 输入注入
- KeyDock 键盘业务

## 窗口策略

窗口目标：

- 置顶
- 工具窗口
- 点击不激活
- 不抢目标窗口焦点

Win32 策略：

- 扩展样式包含 `WS_EX_NOACTIVATE`
- 扩展样式包含 `WS_EX_TOPMOST`
- 扩展样式包含 `WS_EX_TOOLWINDOW`
- 显示和移动时使用 `SWP_NOACTIVATE`
- 不注册全局键盘钩子作为 v1 主路径

## 输入注入策略

只允许平台层把 `InputCommand` 转换为 Win32 输入。

规则：

- 普通键发送 key down + key up
- 修饰组合按 modifier down -> key down/up -> modifier up 顺序发送
- 失败时返回结构化错误
- 不尝试绕过 Windows UIPI 限制
- 不向更高完整性级别窗口承诺可输入

## 渲染策略

Direct2D 负责形状：

- 背景
- 按键矩形
- hover/pressed/active 状态
- 分隔线

DirectWrite 负责文字：

- key label
- 状态指示
- 标题

渲染层只消费 `ViewSnapshot` 里的 primitive tree，不修改 `KeyboardState`，也不判断键盘业务语义。

映射关系：

- `Surface` -> Direct2D filled/outlined rectangle
- `Row` -> app 层已经算好的 child rects，平台层不重新排版
- `Button` -> Direct2D rectangle + state brush
- `Text` -> DirectWrite text layout/draw
- `Image` -> 内置资源解码后的 Direct2D bitmap draw

v1 的核心键盘可以完全由 `Button(Text)` 实现；`Image` 只作为内置图标能力预留。

## DPI 策略

- 启动时设置 Per-Monitor DPI v2
- 保存逻辑像素布局
- 平台层负责 DPI 换算
- 收到 DPI 变化后重算窗口尺寸和布局快照
- 不缓存跨 DPI 的像素尺寸

## unsafe 边界

`unsafe` 只允许出现在小型 wrapper 内。每个 `unsafe` 块必须写明不变量。

允许的 unsafe 区域：

- 注册窗口类
- 创建和销毁窗口
- Win32 window proc
- 消息循环
- Direct2D/DirectWrite COM 资源创建
- `SendInput`
- 必要的 Win32 union 字段写入

不允许：

- 在 app 层出现 `unsafe`
- 在业务状态机里持有 Win32 句柄
- 把裸指针传出平台层
- 把 Win32 错误码作为业务错误直接传播

## 错误模型

平台层错误统一转换为：

```text
PlatformError
  WindowCreation
  MessageLoop
  Rendering
  InputInjection
  Dpi
```

应用层错误统一转换为：

```text
AppError
  InvalidLayout
  UnknownKey
  UnsupportedCommand
```

主入口只负责展示或记录错误，不吞掉失败。

## 线程模型

v1 单 UI 线程。

- 窗口消息
- 状态更新
- 渲染
- 输入注入

不引入后台线程，除非后续有配置落盘、日志或资源加载需求。

## 依赖原则

- 首选标准库和 `windows`
- 不引入 GUI 框架
- 不引入 async runtime
- 不引入全局状态库
- 如果需要错误派生，可以使用轻量错误库；实现阶段再决定是否值得
