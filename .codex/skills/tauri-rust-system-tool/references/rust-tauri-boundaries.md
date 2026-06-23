# Rust 与 Tauri 边界

## 意图

把系统能力和截图主链路放到 Rust，把 Tauri shell 当作集成边界。

## 适用场景

- 修改 `src-tauri/src/domain`、`services`、`shell`。
- 新增 Tauri 命令、托盘、快捷键、通知、缓存能力。
- 处理截图、剪贴板和打开文件。

## 必须遵守的规则

- `domain/` 放纯逻辑：状态机、设置模型、结果模型、错误码。
- `services/` 放副作用：抓屏、裁剪、编码、剪贴板、通知、打开文件、缓存清理。
- `shell/` 放 Tauri 集成：命令、托盘、快捷键、窗口、设置落盘。
- 正常失败走结构化结果，不走 panic 式流程。
- 前端不承载图像处理和缓存管理。

## 推荐模式

- Tauri 命令保持窄：开始区域截图、当前屏幕截图、取消、读取/更新设置、打开最近截图。
- `CaptureResult` 包含文件路径、尺寸、是否复制、是否通知。
- `CaptureError` 包含 code 和 message。
- 剪贴板失败不阻止缓存文件生成。

## 反模式

- 在 TypeScript 里裁剪、编码图片。
- shell 层直接塞大量业务状态机。
- 剪贴板或通知失败导致整个应用崩溃。
- Rust 把 overlay 交互细节建成复杂 UI 状态。

## 证据

- `apps/capture/docs/architecture.md` 定义 Rust domain/services/shell 分层和 Tauri 命令草案。
- `apps/capture/docs/test-spec.md` 定义剪贴板、通知、缓存失败的验收要求。
