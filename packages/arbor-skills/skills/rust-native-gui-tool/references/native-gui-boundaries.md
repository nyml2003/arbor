# 原生 GUI 分层边界

## 意图

让业务状态、组件描述、平台渲染和系统 API 各在自己的边界内演化。

## 适用场景

- 修改 KeyDock、ClipDock 或类似原生小工具。
- 新增使用 `arbor-ui-core` 的原生窗口应用。
- 判断逻辑应放在 app、shared UI crate，还是平台 host。

## 必须遵守的规则

- app 层只做业务状态、布局、命中测试、输入命令和 view 组合。
- `arbor-ui-core` 提供 geometry、event、theme、组件 DSL、primitive tree。
- `arbor-ui-windows` 只消费 view snapshot 并绘制。
- 应用自己的 `platform/windows` 保留窗口、DPI、消息循环和平台能力。
- 不要把 host、输入注入和业务窗口策略塞进共享 renderer crate。

## 推荐模式

- `app/state` 维护业务状态。
- `app/layout` 产出逻辑像素布局。
- `app/input` 产出平台无关命令。
- `app/view` 组合 `arbor-ui-core` primitive tree。
- `platform/windows` 把 Win32 消息转换为 app 事件。

## 反模式

- app 层直接创建窗口。
- UI 组件 trait 里包含渲染行为。
- renderer crate 反向依赖具体应用状态。
- 平台 host 和共享 renderer 边界混在一起。

## 证据

- `workspace/learn/patterns/rust-native-gui-dsl.md` 记录 KeyDock/ClipDock 的分层。
- `apps/keydock/docs/architecture.md` 规定 app 层和 `platform/windows` 边界。
- `apps/clipdock/docs/architecture.md` 规定 app 层不得引入 Win32、unsafe 或 native renderer。
