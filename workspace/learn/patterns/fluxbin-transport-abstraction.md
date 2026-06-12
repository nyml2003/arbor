# 模式：多传输后端抽象（FluxBin）

## 一句话

用独立的 transport 包（websocket + tcp）实现同一个数据传输协议，不同环境（browser/node）自动选择对应的 transport。core 定义数据格式，transport 负责传输，client 是用户入口。

## 核心架构

```
@fluxbin/core              ← 数据格式：frame/reader/writer/registry/shape/stream
    │
    ├── @fluxbin/transport-websocket   ← WebSocket 传输
    ├── @fluxbin/transport-tcp         ← TCP 传输
    │
    ├── @fluxbin/env-browser           ← 浏览器环境适配
    ├── @fluxbin/env-node              ← Node 环境适配
    │
    └── @fluxbin/client               ← 用户入口（组合 core + transport）
         │
         └── @fluxbin/devtools         ← 开发工具
```

7 个包，依赖方向：env → transport → core ← client。core 零环境依赖。

## 关键设计

### 1. Transport 可替换

```typescript
// 同一份 core 数据格式，不同的传输层
// WebSocket transport
import { WebSocketTransport } from '@fluxbin/transport-websocket';

// TCP transport
import { TcpTransport } from '@fluxbin/transport-tcp';
```

传输层和后端抽象完全一致——都是"发数据、收数据"。`client` 不关心底层用的是 WebSocket 还是 TCP。

### 2. 环境隔离

```
@fluxbin/env-browser    → 用浏览器的 WebSocket API
@fluxbin/env-node       → 用 Node.js 的 ws 库
```

环境差异被局限在 env-* 包里——不污染 client 和 core。这是 ObolosFS 的 `mem-driver/windows-adapter` 模式在传输层的再现。

### 3. Core 无平台依赖

```
packages/core/src/
├── frame/      ← 数据帧格式
├── reader/     ← 从字节流读帧
├── writer/     ← 往字节流写帧
├── registry/   ← 消息类型注册
├── shape/      ← 数据形状/校验
├── stream/     ← 流抽象
└── types/      ← 共享类型
```

core 不 import `ws`、`net`、`window`。只定义数据格式和序列化逻辑。测试可以在任何环境跑。

### 4. 分层测试

```
core        → 纯数据测试（序列化/反序列化）
transport   → 传输层测试（mock 连接）
client      → 集成测试（实际 WebSocket/TCP）
devtools    → 手动验证
```

每个包独立可测。core 的测试不依赖网络。

## 和 ObolosFS 模式的对照

| ObolosFS | FluxBin |
|----------|---------|
| `@obolosfs/core` (VFS 引擎) | `@fluxbin/core` (数据格式) |
| `Driver` interface | Transport 接口 |
| `windows-adapter` | `env-node` / `env-browser` |
| `mem-driver` | mock transport |
| `ofsh` (shell) | `client` + `devtools` |

不同领域（文件系统 vs 数据传输），相同的架构模式。

## 来源

- FluxBin 源码（`packages/core/src/index.ts`、`packages/` 目录结构）
- 2026-06-07 agent 阅读后提炼
