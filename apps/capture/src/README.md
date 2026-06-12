# 前端骨架

这里放 Capture 的薄 UI。

前端只负责两件事：

- overlay 的框选交互
- settings 的设置表单

前端不处理图像，不管理缓存，不承载主截图流程。

## 页面分工

- `overlay/`：静态页面 + 原生事件，不上框架，不做 hydration
- `settings/`：独立页面，按需加载 SolidJS
