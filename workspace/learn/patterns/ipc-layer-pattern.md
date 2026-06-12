# 模式：Electron IPC 层设计

## 一句话

用 **zod schema → createHandler → preload bridge → renderer API** 四层管线构建类型安全的 IPC，每层职责单一，每层可独立测试。

## 为什么是这个模式

Electron IPC 的原生 API（`ipcMain.handle` + `ipcRenderer.invoke`）是弱类型的——入参是 `unknown`，返回值是 `Promise<unknown>`。不做封装的话：
- 每个 handler 都要手写参数校验
- 错误格式不一致
- renderer 调用处没有类型提示
- 新增一个 IPC 方法需要改 3 个地方，容易不同步

## 模式结构

```
shared/channels.ts          ← 1. 通道常量（单一真相源）
main/ipc/schemas.ts         ← 2. zod schema + createHandler 工厂
main/ipc/xxx.ipc.ts         ← 3. 注册 handler（调业务逻辑）
preload/index.ts            ← 4. contextBridge 暴露类型化 API
preload/index.d.ts          ← 5. Window 类型声明
renderer/                   ← 6. 直接用 window.appAPI.xxx()
```

## 代码骨架

### 1. channels.ts — 通道常量

```ts
export const IpcChannels = {
  FS_LIST_DIRECTORY: "fs:listDirectory",
  FS_READ_TEXT: "fs:readText",
} as const;
```

- 用 `as const` 确保值类型是字面量而非 `string`
- 命名：`域:动作`

### 2. schemas.ts — 验证工厂

```ts
import { z } from "zod";
import type { IpcMainInvokeEvent } from "electron";

export function createHandler<I, O>(
  schema: z.ZodSchema<I>,
  fn: (input: I) => Promise<O>,
): (event: IpcMainInvokeEvent, raw: unknown) => Promise<O> {
  return async (_event, raw) => {
    const parsed = schema.safeParse(raw);
    if (!parsed.success) {
      const msgs = parsed.error.issues.map(i => i.message).join("; ");
      throw new Error(`Validation failed: ${msgs}`);
    }
    return fn(parsed.data);
  };
}
```

- 泛型 `I, O` 从 schema 推断，不需要显式标注
- `safeParse` 不抛异常，错误消息可控

### 3. xxx.ipc.ts — 注册 handler

```ts
ipcMain.handle(
  IpcChannels.FS_LIST_DIRECTORY,
  createHandler(
    z.object({ path: z.string().min(1) }),
    async ({ path }) => {
      const names = await readdir(path);
      return names.map(/* ... */);
    },
  ),
);
```

- 每个 handler 自包含：schema + 业务逻辑在同一处
- 不需要额外的路由注册表

### 4. preload/index.ts — Bridge

```ts
const api = {
  fs: {
    listDirectory: (path: string): Promise<FileEntry[]> =>
      ipcRenderer.invoke(IpcChannels.FS_LIST_DIRECTORY, { path }),
    readText: (path: string): Promise<string> =>
      ipcRenderer.invoke(IpcChannels.FS_READ_TEXT, { path }),
  },
};

contextBridge.exposeInMainWorld("appAPI", api);
```

- 按域分组（`fs.xxx`, `dialog.xxx`），renderer 调用时自然命名空间
- 这里做参数适配，不做业务逻辑

### 5. preload/index.d.ts — 类型声明

```ts
declare global {
  interface Window {
    readonly appAPI: {
      fs: {
        listDirectory(path: string): Promise<FileEntry[]>;
        readText(path: string): Promise<string>;
      };
    };
  }
}
```

- 这是 renderer 的类型入口 —— renderer 的 `window.appAPI.fs.listDirectory(...)` 有完整的智能提示
- 类型和实现在同一层，好同步

### 6. Renderer 调用

```tsx
const entries = await window.appAPI.fs.listDirectory("/some/path");
```

## 新增 IPC 方法的 checklist

1. `channels.ts` 加通道常量
2. `xxx.ipc.ts` 加 `ipcMain.handle(...)` 
3. `preload/index.ts` 加 bridge 方法
4. `preload/index.d.ts` 加类型声明
5. Renderer 直接用

顺序漏了任何一步，编译期就报错（类型 + 通道名都有检查）。

## 常见坑

### 模块间的状态同步

如果 IPC handler 内部有模块级状态（如工作区根目录），必须在 app 启动时显式初始化，不能依赖隐式副作用。

```
❌  app.ts 设了 workspaceRoot，filesystem.ipc.ts 读不到 → "No workspace selected"
✅  导出 setWorkspaceRoot()，app.whenReady() 中主动调用
```

### 路径安全检查

文件系统 IPC 必须做路径遍历防护：

```ts
function resolveChecked(input: string): string {
  const resolved = pathResolve(input);
  const normalizedRoot = pathResolve(workspaceRoot);
  if (!resolved.startsWith(normalizedRoot + sep) && resolved !== normalizedRoot) {
    throw new Error("Access denied");
  }
  return resolved;
}
```

## 来源

- Arbor Phase 1 实战（2026-06-07）
- WatchDesk IPC 层设计
- ObolosFS 的 Result-based error model（概念上有相似性——预期失败不抛异常）
