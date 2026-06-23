# 工作台验证

## 意图

让桌面工作台改动通过合适粒度验证，不把所有问题都推给人工点开看。

## 适用场景

- IPC 合同变化。
- renderer 组件变化。
- Electron 启动、窗口、preload 或 e2e 行为变化。
- 文件树和本地文件访问变化。

## 必须遵守的规则

- shared contract 改动后运行 typecheck。
- IPC 行为改动要覆盖成功和失败路径。
- 文件树改动要验证根目录、空目录、目录展开、文件选择。
- UI 改动要确认不会一次性加载大树。

## 推荐模式

- 小改动先跑容器相关 test/typecheck。
- Electron 外壳变化跑 e2e 或最窄启动验证。
- 失败时先看 preload 和 shared 类型是否不同步。
- 对性能改动，用“挂载多少 DOM、什么时候读目录”判断。

## 反模式

- 只在 renderer 改 UI，却忘了 preload 类型声明。
- IPC 改了通道名但没有同步桥接 API。
- 为了 UI 方便把 Node 能力暴露进 renderer。

## 证据

- `workspace/learn/patterns/ipc-layer-pattern.md` 的新增 IPC checklist。
- `README.md` 和 `PLAN.md` 确认 `apps/container` 是 Arbor 主容器路线。
