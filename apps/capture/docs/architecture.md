# Capture 架构说明

## 技术路线

- 宿主：`Tauri 2`
- 系统能力和主链路：`Rust`
- overlay：静态 HTML + 原生事件
- settings：`SolidJS`，按需加载
- 平台范围：当前只支持 `Windows`

这个工具的目标不是“把 UI 写得多花”，而是把截图主链路做轻、做稳。抓屏、裁剪、编码、剪贴板、通知、打开文件、缓存清理都放在 Rust。

## 为什么不用 Electron

- 这是常驻小工具，不是内容容器
- 截图主链路是系统调用和图像处理，放在 Rust 更合理
- Tauri 足够提供托盘、窗口、快捷键、通知和默认程序打开能力

这条决策只适用于 `apps/capture/`。Arbor 当前的 `apps/container/` 仍然保留 Electron 路线。
当前截图服务不再保留跨端 fallback。Windows 单独维护自己的截图后端。

## 分层

### Rust

- `domain/`：纯逻辑。定义截图任务状态机、设置模型、结果模型、错误码
- `services/`：副作用层。抓屏、裁剪、编码、剪贴板、通知、打开文件、缓存清理
- `shell/`：Tauri 集成。命令、托盘、全局快捷键、窗口、设置落盘

### 前端

- `overlay/`：静态页面和原生事件。只负责矩形选择和取消
- `settings/`：按需加载的设置页
- `shared/`：前端局部共享类型和常量

## 目录骨架

```text
apps/capture/
├── README.md
├── docs/
│   ├── README.md
│   ├── product.md
│   ├── architecture.md
│   └── test-spec.md
├── src-tauri/
│   ├── README.md
│   └── src/
│       ├── README.md
│       ├── domain/
│       │   └── README.md
│       ├── services/
│       │   └── README.md
│       └── shell/
│           └── README.md
└── src/
    ├── README.md
    ├── overlay/
    │   └── README.md
    ├── settings/
    │   └── README.md
    └── shared/
        └── README.md
```

## 前端加载策略

### Overlay

- 单独入口
- 本地静态 HTML
- 不做 SSR
- 不做 hydration
- 不引入框架运行时
- 只绑定最小事件：
  - `pointerdown`
  - `pointermove`
  - `pointerup`
  - `keydown(Escape)`

### Settings

- 单独入口
- 打开设置窗口时再加载
- 可以使用 SolidJS
- 不与 overlay 共用大 bundle

## 首屏约束

- overlay 首屏不加载 SolidJS
- overlay 的 DOM 深度保持浅
- overlay 的 CSS 只做必要布局和遮罩
- 不引入重型 UI 库、图标库、状态库
- 前端构建目标按现代 WebView 设定，不做 legacy polyfill

## 窗口模型

### 托盘

- 应用启动后默认常驻托盘
- 托盘菜单提供：
  - 区域截图
  - 当前屏幕截图
  - 打开最近一次截图
  - 设置
  - 退出

### Overlay 窗口

- 透明
- 无边框
- 置顶
- 全屏
- 只在截图时存在
- 内容为静态 overlay 页面

### Settings 窗口

- 普通小窗口
- 按需打开
- 不承载截图流程

## 主数据流

```text
快捷键 / 托盘菜单
  -> shell 触发截图会话
  -> overlay 收集矩形
  -> services 抓屏
  -> services 裁剪并编码 PNG
  -> services 写缓存文件
  -> services 写剪贴板
  -> services 发通知
  -> 用户点击通知
  -> services 用系统默认图片查看器打开文件
```

## Tauri 命令草案

- `begin_area_capture`
- `capture_active_display`
- `cancel_capture`
- `get_settings`
- `update_settings`
- `open_last_capture`

## 数据类型草案

### CaptureSettings

- `hotkey`
- `notification_enabled`
- `cache_limit`
- `launch_on_login`

### CaptureResult

- `file_path`
- `width`
- `height`
- `copied`
- `notified`

### CaptureError

- `code`
- `message`

## 边界约束

- TS 不做图像处理
- TS 不维护截图历史主数据
- Rust 不把 UI 状态回推成复杂前端状态机
- 正常失败走结构化结果，不走 panic 式流程
- overlay 不上框架，不做 hydration
- settings 的框架代码不能进入 overlay 首屏

## 当前阶段已经完成的壳层

- Tauri 配置文件
- Cargo 清单
- 多窗口配置
- 托盘菜单
- overlay 静态入口
- settings 独立入口
- Rust 命令空实现

## 当前阶段还没做的事

- 真实抓屏
- 图像裁剪和编码
- 剪贴板图片写入
- 系统通知
- 点击通知后打开文件
- 缓存目录管理
