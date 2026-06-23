# Windows 平台安全边界

## 意图

把 Win32、Direct2D、剪贴板、DPI 和输入注入限制在可审计的平台边界内。

## 适用场景

- 修改 `platform/windows`。
- 修改窗口、DPI、消息循环、剪贴板监听、输入注入。
- 增加 unsafe 代码。

## 必须遵守的规则

- `unsafe` 只允许出现在平台适配层和小型 wrapper 内。
- 每个 `unsafe` 块要能说明不变量。
- Win32 handle、buffer、COM 对象必须在边界内转换成安全 Rust 值。
- 输入注入只由平台层执行。
- 不承诺绕过 Windows UIPI 限制。

## 推荐模式

- 使用现代 API，例如剪贴板监听用 `AddClipboardFormatListener`。
- 主渲染路径保持 Direct2D/DirectWrite，不回退成 GDI 主路径。
- DPI 使用 Per-Monitor DPI v2，不写死 96 DPI。
- 失败转换为 `PlatformError`，不要把裸 Win32 错误码传进 app 层。

## 反模式

- app 状态机里出现 `unsafe`。
- 裸指针或 HWND 穿过平台边界。
- 剪贴板内存锁定逻辑散落到 app 层。
- 用旧剪贴板 viewer chain 作为新实现主路径。

## 证据

- `apps/keydock/docs/architecture.md` 规定 `SendInput`、DPI、window proc 和 COM 边界。
- `apps/clipdock/docs/architecture.md` 规定剪贴板监听 API 和 unsafe scope。
