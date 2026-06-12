# 模式：纯逻辑虚拟文件系统（ObolosFS VFS）

## 一句话

把文件系统抽象成一个**纯函数**——core 零 IO 依赖，所有副作用由 Driver 接口隔离，路径安全和权限在 VFS 引擎层统一执行。

## 为什么需要这个模式

文件系统是许多应用（桌面应用、agent 运行时）中最底层、被调用最频繁的抽象。如果耦合了 Node.js `fs`：

- 单元测试必须 mock `fs` 模块
- 无法在内存中运行（测试慢、不可靠）
- 不同环境（Windows/macOS/browser）行为不一致
- agent 的文件访问权限控制需要散落在各处

ObolosFS 的方案：**core 完全不知道 Node.js 的存在**。文件系统的一切行为由 `Driver` 接口定义，由 `Vfs` 引擎编排。

## 核心架构

```
                     ┌──────────────┐
                     │   VFS Engine  │  ← 纯逻辑，零 IO
                     │ (vfs.ts)     │
                     └──────┬───────┘
                            │ Driver interface
            ┌───────────────┼───────────────┐
            ▼               ▼               ▼
      ┌──────────┐   ┌──────────┐   ┌──────────────┐
      │ MemDriver│   │PipeDriver│   │WindowsAdapter│  ← 副作用在这里
      │ (纯内存)  │   │ (管道)    │   │ (Node.js fs)│
      └──────────┘   └──────────┘   └──────────────┘
```

**关键约束**：Core 包不 import Node.js 任何模块。`import { readFile } from "fs"` 永远不会出现在 core 里。

## 关键设计

### 1. Driver 接口

```typescript
export type Driver = Readonly<{
  capabilities: DriverCapabilities;       // ← 驱动声明自己会什么
  open(path: DriverPath, flags: OpenFlag): Promise<Result<DriverFile>>;
  mkdir(path: DriverPath): Promise<Result<void>>;
  rmdir(path: DriverPath): Promise<Result<void>>;
  unlink(path: DriverPath): Promise<Result<void>>;
  rename(from: DriverPath, to: DriverPath): Promise<Result<void>>;
  stat(path: DriverPath): Promise<Result<FileStat>>;
  readdir(path: DriverPath): Promise<Result<ReadonlyArray<DirEntry>>>;
  openDir?: (path: DriverPath) => Promise<Result<DriverDir>>;
}>;
```

- 全部返回 `Promise<Result<T>>` —— 异步但绝不抛异常
- `openDir` 是可选方法 —— 驱动可以不实现，VFS 自动返回 ENOTSUP
- `DriverPath` 是 brand type，和 `VirtualPath` 类型不兼容，防止混用

### 2. Result-based Error（不抛异常）

```typescript
export type Result<T, E = FsError> =
  | Readonly<{ ok: true; value: T }>
  | Readonly<{ ok: false; error: E }>;

// 正常失败用 err()
function open(path: string): Promise<Result<FileHandle>> { ... }

// 调用方必须检查
const result = await vfs.open("/mem/test.txt", OpenFlags.RDONLY);
if (!result.ok) {
  // result.error.code === "ENOENT" | "EACCES" | ...
  return;
}
const file = result.value;
```

**好处**：
- 正常失败（文件不存在、权限不足）不是异常——是 Result
- 只有编程错误（assertion failure、invariant violation）才抛异常
- 类型系统强制调用方处理失败情况

**和 Rust 的 Result 一致**，TS 用 discriminated union 实现。

### 3. Capability 声明

每个 Driver 声明自己的能力和语义：

```typescript
export type DriverCapabilities = Readonly<{
  methods: {             // ← 这个方法存在吗
    read: boolean;
    write: boolean;
    seek: boolean;
    mkdir: boolean;
    // ...
  };
  semantics: {           // ← 这个方法的行为特征
    seekable: boolean;   //    seek 后位置是否可靠
    durable: boolean;    //    write 后是否持久化
    atomicRename: boolean;
    // ...
  };
}>;
```

VFS 引擎在执行操作前检查：

```typescript
// vfs.ts - open 方法
const capability = requireOpenCapabilities(route.value.mount.driver, access);
if (!capability.ok) return capability;  // 返回 ENOTSUP，不抛异常
```

**Capability 是声明式的**——驱动声明，VFS 检查。驱动不需要自己写 if-else 判断是否支持某个操作。

### 4. Mount Table（虚拟路径路由）

```
VFS
├── /mem      → MemDriver       (内存，不持久)
├── /pipe     → PipeDriver      (管道，用于进程间)
└── /host     → WindowsAdapter  (映射到真实目录)
```

- 最长前缀匹配：`/mem/a/b/c.txt` 匹配 `/mem`
- 嵌套 mount 被显式拒绝（防止歧义）
- 路径安全在 `parseVirtualPath()` 中一步完成：禁止 `..`、反斜杠、Windows 盘符、UNC 路径

### 5. Brand Types 防混用

```typescript
declare const virtualPathBrand: unique symbol;
export type VirtualPath = string & { [virtualPathBrand]: true };

declare const driverPathBrand: unique symbol;
export type DriverPath = string & { [driverPathBrand]: true };
```

- `/mem/hello.txt` 是 `VirtualPath`
- `/hello.txt` 是 `DriverPath`（去掉 mount 前缀后的相对路径）
- TS 不允许它们互相赋值，编译器就拦住了一层 bug

### 6. Const Objects 代替 Enums

```typescript
// ✅ ObolosFS
export const FsErrorCode = {
  ENOENT: 'ENOENT',
  EACCES: 'EACCES',
} as const;

// ❌ 传统写法
enum FsErrorCode { ENOENT = 'ENOENT', EACCES = 'EACCES' }
```

- const object 编译后是 `{ ENOENT: 'ENOENT', EACCES: 'EACCES' }`，无运行时开销
- TypeScript enum 编译后产生额外的 IIFE 代码
- 配合 `as const` 保持字面量类型推断

## ofsh：编译器架构的 Shell

```
用户输入 → Lexer → Parser → Executor
             │        │         │
          token流   AST树    VFS操作
```

### Lexer（手写，无正则）

- 单 pass 扫描，`position` 指针推进
- 逐字符判断 token 边界，不依赖 `split()` 或正则
- 支持三重引号 `"""..."""`、转义字符
- 错误包含行号/列号信息

### Parser（递归下降）

- `Statement → Pipeline → Command → Argument`
- 支持管道 `|` 和重定向 `>` `>>`
- 每个 `parse*` 函数返回 `Result<T, ParseError>`

### Executor（管道执行）

- 管道命令**串行**执行：前一个命令的输出作为后一个命令的 `stdin`
- 重定向：打开 VFS 文件，写入内容
- 每个命令通过 `CommandRegistry` 查找 handler

```typescript
// 命令注册
registry.register("ls", async (ctx, args) => {
  const entries = await ctx.vfs.readdir(args[0] ?? "/");
  // ...
});
```

## 反模式警示

### ❌ 把 IO 放进 "core"

```typescript
// 不要这样做
export function loadConfig() {
  return readFileSync("./config.json");  // ← 直接调 Node.js fs
}
```

### ❌ 抛异常表达正常失败

```typescript
// 不要这样做
async function open(path: string): Promise<FileHandle> {
  if (!exists(path)) throw new Error("File not found");  // ← 调用方必然漏 catch
}
```

### ❌ 用字符串做路径安全检查

```typescript
// 不要这样做
if (path.includes("..")) return error;  // ← 可能被编码绕过
```

ObolosFS 的做法：`//` 和 `..` 各自独立检查，Windows 盘符用正则 `/^[A-Za-z]:[\\/]/`

## 来源

- ObolosFS 源码（`packages/core/`、`packages/ofsh/`）
- 2026-06-07 agent 深度阅读后提炼
