# Capture

独立桌面截图工具。技术路线定为 `Tauri 2 + Rust-first + 静态 overlay + 延迟加载的 SolidJS settings`。当前版本只支持 Windows。

## 当前状态

当前完成了三件事：

- 写清产品、架构和测试规格
- 建好目录骨架，给后续实现留位置
- 建好真正能跑的空壳：Tauri 窗口、托盘、overlay 静态页、settings 独立页

还没有开始真实的抓屏、剪贴板、通知、打开文件和缓存逻辑。

## v1 主流程

1. 用户按全局快捷键，或从托盘菜单触发截图
2. 应用拉起透明 overlay，让用户框选区域
3. Rust 侧完成抓屏、裁剪、PNG 编码
4. 结果先写入缓存目录
5. 图片复制到系统剪贴板
6. 应用发送系统通知
7. 用户点击通知时，用系统默认图片查看器打开这张缓存图

## v1 非目标

- 标注、箭头、文字、模糊
- OCR
- 滚动截图
- 录屏
- 历史资料库、标签、搜索
- CLI 入口

## 前端策略

- `overlay`：本地静态 HTML 页面 + 原生事件绑定
- `settings`：独立设置页，按需打开时再加载 SolidJS

这样做的目标很直接：

- 截图首屏尽量少启动 JS
- overlay 不做 hydration
- 设置页不参与截图首帧

## 目录说明

- `docs/`：产品、架构、测试规格
- `src-tauri/`：Tauri 和 Rust 空壳
- `src/`：前端页面。overlay 是静态页，settings 是独立页

## 开发约束

- 截图主链路全部放在 Rust
- TS 不处理图像，不管理缓存，不承载主状态机
- overlay 不上框架，不做 hydration
- settings 才使用 SolidJS，而且按需加载
