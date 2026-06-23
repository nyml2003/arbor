---
name: tauri-rust-system-tool
description: 设计或维护 Tauri 2 + Rust-first 系统工具、托盘、全局快捷键、静态 overlay、按需 settings、截图/剪贴板/通知/缓存等系统能力边界。用于 apps/capture 或类似常驻小工具，不用于 Arbor Electron 主容器。
---

# Tauri Rust 系统工具

用这个技能维护 Tauri + Rust 主链路的小型系统工具。目标是轻、稳、系统能力边界清楚。

## 引用路由

- 判断产品范围、v1 主流程和不做什么：读 [product-boundary.md](references/product-boundary.md)。
- 修改 Rust domain/services/shell 或 Tauri 命令：读 [rust-tauri-boundaries.md](references/rust-tauri-boundaries.md)。
- 修改 overlay、settings 或前端加载：读 [frontend-loading.md](references/frontend-loading.md)。
- 做验收和失败路径：读 [capture-validation.md](references/capture-validation.md)。

## 默认流程

1. 先确认这是系统小工具，不是内容工作台。
2. 热路径放 Rust：系统调用、图像处理、剪贴板、通知、缓存。
3. overlay 保持静态入口，只处理框选和取消。
4. settings 单独窗口，按需加载 SolidJS。
5. 正常失败结构化返回，不让应用崩溃。
6. 验收以 Windows v1 场景为准。

## 硬规则

- overlay 不加载 SolidJS，不做 hydration。
- settings 代码不能进入 overlay 首屏。
- TypeScript 不做图像处理，不维护截图历史主数据。
- Rust 不把 UI 状态回推成复杂前端状态机。
- v1 不做标注、OCR、录屏、历史资料库、云同步、CLI。
