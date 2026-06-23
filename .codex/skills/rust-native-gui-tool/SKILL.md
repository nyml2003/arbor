---
name: rust-native-gui-tool
description: 设计或维护不使用 WebView 的 Rust 原生桌面小工具、组件 DSL、primitive tree、平台适配层、Direct2D/DirectWrite 渲染、DPI、输入注入和 unsafe 边界。用于 KeyDock、ClipDock、arbor-ui-core、arbor-ui-windows 或类似原生小窗口工具。
---

# Rust 原生 GUI 工具

用这个技能维护轻量系统小窗口。它适合虚拟键盘、剪贴板面板、悬浮工具、托盘小窗，不适合复杂内容型工作台。

## 引用路由

- 设计 app/core/platform 分层：读 [native-gui-boundaries.md](references/native-gui-boundaries.md)。
- 修改组件 DSL、primitive tree 或渲染合同：读 [primitive-tree-contract.md](references/primitive-tree-contract.md)。
- 修改 Windows host、DPI、输入、剪贴板或 unsafe 代码：读 [windows-platform-safety.md](references/windows-platform-safety.md)。

## 默认流程

1. 先确认改动属于 app 层、`arbor-ui-core`、`arbor-ui-windows`，还是平台 host。
2. app 层只维护状态、布局、命中测试、输入命令和视图快照。
3. 共享 UI crate 只描述或绘制 primitive，不接管窗口生命周期。
4. 平台层接收系统消息，转换为 app 事件，再绘制快照。
5. `unsafe` 只留在平台适配边界。
6. 改完后跑 Rust 测试、check，并做边界扫描。

## 硬规则

- app 层不能 import `windows::Win32::*`。
- app 层不能出现 `unsafe`、`HWND`、COM 类型或 `SendInput`。
- primitive 节点不持有窗口句柄、平台资源或渲染对象。
- `arbor-ui-windows` 不创建窗口、不处理输入注入、不知道产品业务。
- 不为了“未来跨平台”提前抽空 adapter。
