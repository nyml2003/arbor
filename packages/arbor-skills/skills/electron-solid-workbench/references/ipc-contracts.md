# IPC 合同

## 意图

让 Electron 主进程和渲染进程之间的调用有固定合同，不让弱类型 IPC 漏到业务层。

## 适用场景

- 新增文件系统、对话框、设置、工作区等 IPC。
- 修改 preload 暴露的 API。
- 调整 renderer 调用主进程能力的方式。

## 必须遵守的规则

- 通道名集中定义，不能在 handler 和 renderer 里散写字符串。
- handler 入参必须校验。
- preload 只做桥接和参数适配，不写业务规则。
- renderer 只通过暴露的 API 调主进程，不直接 import Electron。
- 类型声明要覆盖 `window` 上暴露的 API。

## 推荐模式

- 通道常量使用 `域:动作` 命名。
- handler 同处写 schema 和调用逻辑。
- preload 按域分组，例如 `fs.listDirectory`。
- 错误消息保留可诊断信息，不把原始 unknown 直接抛给 UI。

## 反模式

- 在 renderer 中调用 `ipcRenderer.invoke`。
- 每个 handler 自己写一套参数解析和错误格式。
- preload 暴露过宽的通用执行函数。
- 主进程模块靠隐式副作用共享状态。

## 证据

- `workspace/learn/patterns/ipc-layer-pattern.md` 记录 Arbor Phase 1 的 IPC 四层管线。
- `apps/container/src/main/ipc`、`src/preload`、`src/shared` 是当前容器 IPC 边界。
