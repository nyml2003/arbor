# Kaubo 分层约束

## 编译器管道

```
源码 → kaubo-syntax → AST → kaubo-infer → 类型化 AST → kaubo-cps → CPS IR → kaubo-ir → 优化 IR → kaubo-vm → 执行
                                   ↗ kaubo-token（词法定义）
```

## 层间规则

### 词法/语法前端（kaubo-token, kaubo-ast, kaubo-syntax）

- 只负责把源码变成结构化 token、AST、span 和诊断信息
- 不依赖任何后端（infer/cps/ir/vm）
- AST 节点不包含类型信息（类型由 infer 层附加）

### 类型推断（kaubo-infer）

- 消费 AST，产出类型化 AST
- 不依赖 CPS/IR/VM
- 不依赖 Web 或 VSCode 适配层

### CPS/IR/优化（kaubo-cps, kaubo-ir）

- 消费类型化 AST，产出中间表示
- 不依赖 VM 执行细节
- 不依赖 Web 或 VSCode 适配层

### 虚拟机（kaubo-vm）

- 只消费已编译 IR
- 不知道源码解析细节
- 不持有 AST 引用

### 适配层（kaubo-web-api, kaubo-wasm, kaubo-language-service）

- Web 和 VSCode 适配层共享稳定的 JSON/DTO 结构（kaubo-web-api）
- 不各自重新推导编译器逻辑
- WASM 编译目标（kaubo-wasm）是 kaubo-web-api 的消费者

### 基础设施（kaubo-log, kaubo-log-handlers, kaubo-vfs）

- 可被任意层依赖（单向）
- 不依赖业务层

### 入口（kaubo-driver, kaubo2-cli）

- 组合所有层，提供统一 API
- CLI 不直接调用各 crate 内部 API（通过 driver）

## 依赖方向

```
kaubo2-cli
  └→ kaubo-driver
       ├→ kaubo-syntax → kaubo-ast → kaubo-token
       ├→ kaubo-infer
       ├→ kaubo-cps
       ├→ kaubo-ir
       ├→ kaubo-vm
       ├→ kaubo-language-service
       ├→ kaubo-web-api → kaubo-wasm
       └→ kaubo-vfs
```

## 跨层禁止事项

- 不要为了测试方便在 crate 之间新增直接依赖
- 不要在低层 import 高层 crate
- 不要在 vm 中引用 syntax 的类型
- 不要在 infer 中引用 web-api 的类型
- 不要把 WASM 特定的构建逻辑写在 kaubo-wasm 以外
