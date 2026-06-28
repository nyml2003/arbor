---
name: kaubo-dev
description: 开发 Kaubo 编程语言（Rust workspace + Web Playground + VSCode 扩展）。用于修改词法/语法/类型推断/CPS/IR/VM/WASM/VFS 等 Rust crate、维护 SolidJS Web GUI、开发 VSCode 扩展、运行测试/CI/benchmark/覆盖率、发布部署，或理解 Kaubo 的 DDD 分层工程约束时。
---

# Kaubo 开发

用这个技能维护 Kaubo 编程语言的 monorepo。它适合修改编译器管道（token → AST → infer → CPS → IR → VM）、Web GUI（SolidJS + CodeMirror）、VSCode 扩展（语法高亮 + diagnostics），不适合修改 Arbor 容器或其他 packages。

## 构建入口

**唯一入口：`python kaubo-ops <cmd>`**。不要手写 `cd next_kaubo && cargo check ...` 这类裸命令。

所有任务定义在 `kaubo-ops/`（DDD 四层架构，纯 Python stdlib）：

```
kaubo-ops/
├── cli/main.py         ← 表示层：argparse + 命令路由
├── app/                 ← 应用层：用例编排
├── domain/              ← 领域层：聚合根 + 值对象
├── infra/               ← 基础设施层：命令执行/文件系统/事件
└── config.json          ← 集中配置
```

常用命令：

```bash
python kaubo-ops ci           # 标准 CI（check + clippy + fmt + test + WASM + Web + VSCode）
python kaubo-ops check        # 快速类型检查（无测试）
python kaubo-ops build-wasm   # WASM 双目标构建（web + nodejs）
python kaubo-ops test-rust    # Rust 测试
python kaubo-ops test-web     # Web 测试
python kaubo-ops lint         # 全部 lint（clippy + eslint）
python kaubo-ops fmt          # 全部格式化
python kaubo-ops dev          # 开发服务器（长驻进程，Ctrl-C 停止）
python kaubo-ops release      # 发布
python kaubo-ops bench        # 跨语言性能对比
python kaubo-ops coverage     # 覆盖率
```

## 引用路由

- 改 Rust crate 边界或分层约束：读 [layer-constraints.md](references/layer-constraints.md)。
- 改构建/CI/部署流程：读 `packages/kaubo-features/docs/tooling-review.md`。
- 详细的子项目规则和测试策略：读 `packages/kaubo-features/AGENTS.md`。

## 默认流程

1. 先确认改动属于哪个 crate/层（见分层约束）。
2. 优先 TDD：先写失败测试，再实现，再重构。
3. 用 `python kaubo-ops check` 做快速类型检查。
4. 用 `python kaubo-ops test-rust`（或对应目标）跑测试。
5. 用 `python kaubo-ops lint` 做 lint 检查。
6. 提交前跑 `python kaubo-ops ci`。

## 硬规则

- 词法/语法前端只负责源码 → token/AST/span/diagnostics。
- 类型推断、CPS/IR、优化和 VM 不依赖 Web 或 VSCode 适配层。
- VM 只消费已编译 IR，不知道源码解析细节。
- Web 和 VSCode 适配层共享稳定的 JSON/DTO 结构，不各自重新推导编译器逻辑。
- 把 crate 边界当作架构边界，不要为了测试方便新增跨层依赖。
- 改 WASM 导出 API 后必须跑 `python kaubo-ops build-wasm`（一次构建 web + nodejs 双目标）。
- 旧代码、实验代码不要变成新工作的默认路径。
- 子项目 `package.json` 禁止新增 `scripts`（所有构建行为定义在 Ops2 中）。
- 新增 Kaubo 语言特性时，同时补 benchmark 用例到 `next_kaubo/ops/benchmark/suites/`。

## Rust crate 地图

| Crate | 职责 |
|-------|------|
| `kaubo-token` | 词法 token 定义 |
| `kaubo-ast` | 抽象语法树 |
| `kaubo-syntax` | 语法解析（token → AST） |
| `kaubo-infer` | 类型推断 |
| `kaubo-cps` | CPS 转换 |
| `kaubo-ir` | 中间表示 |
| `kaubo-vm` | 虚拟机执行 |
| `kaubo-driver` | 编译器驱动 |
| `kaubo-language-service` | LSP 语言服务 |
| `kaubo-web-api` | Web/VSCode 共享 DTO |
| `kaubo-wasm` | WASM 编译目标 |
| `kaubo-log` | 日志基础设施 |
| `kaubo-log-handlers` | 日志处理器 |
| `kaubo-vfs` | 虚拟文件系统 |
| `kaubo2-cli` | CLI 二进制入口 |

## 测试策略

- 词法/语法/span/diagnostics 测 syntax
- lowering 和 optimization 测 IR/CPS
- 执行行为测 VM
- 适配层和 UI glue 测 app（Web + VSCode）
- 跨语言回归测 benchmark
- 修 bug 时优先补回归测试，最好和修复一起提交
