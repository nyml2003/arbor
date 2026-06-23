# KeyDock

KeyDock 是一个 Windows 原生虚拟键盘。它的定位是克制、稳定、系统工具感的桌面屏幕键盘，不替代输入法，也不做宏平台。

## 当前状态

当前已经进入 Rust 原生实现阶段：

- 已有 `apps/keydock` Cargo 应用
- 已有安全 Rust 的键盘布局、组件原语、状态机和输入命令
- 已有 Windows-only 平台壳、置顶不抢焦点窗口、Direct2D/DirectWrite 基础渲染和 `SendInput` 输入注入边界
- 已有单元测试覆盖布局、命中测试、修饰键状态和 primitive tree

## v1 主流程

1. 用户打开 KeyDock
2. 应用显示一个置顶但不抢焦点的键盘面板
3. 用户点击虚拟按键
4. KeyDock 向当前前台窗口注入对应键盘输入
5. 目标窗口收到输入，KeyDock 继续保持可用

## v1 范围

- 英文 QWERTY 全键盘
- 数字行和常用符号
- Backspace、Enter、Space、Esc
- Shift、Ctrl、Alt 基础修饰键
- 置顶、不抢焦点、高 DPI 适配

## v1 非目标

- IME 或中文候选词
- 宏系统
- 复杂主题市场
- 系统服务
- 管理员权限注入绕过
- 跨平台兼容

## 技术方向

- Rust stable + MSVC target
- `windows` crate 直连 Win32 API
- Direct2D + DirectWrite 渲染
- `SendInput` 注入键盘事件
- Win32 和 `unsafe` 只允许在 `platform::windows` 边界内

## 开发命令

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo check --target x86_64-pc-windows-msvc
```

## 文档

- `docs/product.md`：产品定位、用户流程、范围边界
- `docs/components.md`：原子组件、状态、交互语义
- `docs/architecture.md`：Rust/Win32 技术方案和 unsafe 边界
- `docs/test-spec.md`：后续实现验收场景
