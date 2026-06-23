# 前端加载策略

## 意图

让截图首屏足够轻，避免把设置页或框架运行时带进 overlay。

## 适用场景

- 修改 capture overlay。
- 修改 settings 页面。
- 调整前端入口和 bundle。

## 必须遵守的规则

- overlay 是单独入口，本地静态 HTML。
- overlay 不做 SSR、不做 hydration、不加载 Solid runtime。
- overlay 只绑定必要事件：pointerdown、pointermove、pointerup、Escape。
- settings 单独窗口，打开时再加载，可以使用 SolidJS。
- settings 代码不能进入 overlay 首屏 bundle。

## 推荐模式

- overlay DOM 层级保持浅。
- CSS 只做遮罩、选区框、必要坐标反馈。
- 不引入重型 UI 库、图标库、状态库。
- 前端构建目标按现代 WebView，不做 legacy polyfill。

## 反模式

- 为 overlay 引入完整前端框架。
- 把 settings 的共享组件打进 overlay。
- 用复杂动画、阴影、模糊增加首屏负担。

## 证据

- `apps/capture/docs/architecture.md` 和 `product.md` 都明确 overlay 静态、settings 按需加载。
- `DECISIONS.md` 决策 10 记录 capture 不复用 Electron 壳。
