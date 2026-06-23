---
name: electron-solid-workbench
description: 维护 Electron + SolidJS 工作台、桌面容器、IPC 通道、preload bridge、renderer API、递归文件树和按需加载 UI。用于修改 apps/container、设计 Electron 主进程与渲染进程边界、增加文件系统 IPC、优化 SolidJS 文件树或桌面内容工作台时。
---

# Electron Solid 工作台

用这个技能维护 Arbor 主容器这类复杂内容型桌面工作台。它适合 Electron + SolidJS，不适合截图 overlay 或原生小窗口。

## 引用路由

- 新增或修改 IPC：读 [ipc-contracts.md](references/ipc-contracts.md)。
- 修改文件树、递归 UI、按需加载：读 [solid-file-tree.md](references/solid-file-tree.md)。
- 判断 UI 性能和验证路径：读 [workbench-validation.md](references/workbench-validation.md)。

## 默认流程

1. 先确认改动属于主进程、preload、renderer，还是 shared contract。
2. IPC 改动从通道常量和输入输出类型开始。
3. preload 只暴露窄 API，不放业务逻辑。
4. renderer 只调用 `window.appAPI` 这类桥接 API。
5. 文件树优先按需加载，不一次性读完整棵树。
6. 验证时优先跑容器相关 typecheck、unit 或 e2e。

## 硬规则

- renderer 不直接访问 Node 文件系统。
- IPC 入参必须校验，错误格式要稳定。
- 不可见的树节点不要先挂 DOM 再隐藏。
- 不要让设置、展示或临时页面污染主容器的共享 API。
- Electron + SolidJS 是主容器路线，不要套用到 capture overlay。
