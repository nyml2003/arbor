# 012 — Kaubo 展品架构设计

日期：2026-06-09

## 做了什么

为 kaubo 可交互编译器展品设计了完整架构。这是一个 WASM Rust 引擎 + TypeScript 接口层 + SolidJS UI 的浏览器项目，放在 `apps/kaubo/` 下。

核心设计决策：
- Rust 侧：`kaubo-engine`，一个 crate，四个 WASM 导出函数（lex/parse/codegen/execute），JSON string 进出
- TS 侧：三层——WASM Adapter → Orchestrator → Visualizer，纯逻辑零 DOM
- UI 侧：SolidJS，createResource 驱动流水线，Show 条件挂载，For 列表渲染
- 不做 TypeChecker，流水线四阶段：Lexer → Parser → CodeGen → VM

详细架构见 `workspace/build/kaubo-architecture.md`。

## 学到了什么

- 愿景对齐比实现方案重要十倍。在搞清楚"这到底是什么"之前，画任何架构图都是浪费时间。
- kaubo 是展品，不是产品。展品追求"来访者碰一下就能感受到你的理解有多深"，产品追求"解决别人的问题"。
- 架构分层不是技术问题，是边界问题。每一层不知道上一层或下一层的存在——这不是为了"解耦"，是为了让每一层可以独立思考和独立验证。
- WASM boundary 用 JSON 协议，不是二进制——牺牲一点性能，换取 Rust 和 TS 两边的完全独立。

## 决策

- Rust engine 不搬到新 repo，作为 `apps/kaubo/kaubo-engine/` 放在 Arbor monorepo 内
- 不做类型系统（TypeChecker/TypedAst 跳过）
- 不做多文件/模块系统（MVP 单文件）
- 不做 CLI（Web 优先）
- 方案文档写在 `workspace/build/` 下，用迭代日志引用

## 下一步

| 文档 | 位置 |
|------|------|
| 架构设计 | `workspace/build/kaubo-architecture.md` |
| 路线图 | `workspace/build/kaubo-roadmap.md` |
| UI 设计 | `workspace/build/kaubo-ui-design.md` |
| 语法规格 | `workspace/build/kaubo-grammar.md` |
| WASM 类型合同 | `workspace/build/kaubo-wasm-types.ts` |
| 测试用例规格 | `workspace/build/kaubo-test-spec.md` |
| 字节码指令集 | `workspace/build/kaubo-bytecode.md` |
| VM 架构 | `workspace/build/kaubo-vm.md` |
| CodeGen 规格 | `workspace/build/kaubo-codegen.md` |

五阶段路线图：项目骨架 + Lexer → Parser + AST → CodeGen + 字节码 → VM + 全流水线 → 收尾。

语法从 kaubo-features 源码提取，只收录取已验证可用的特性。WASM 类型是 Rust 和 TS 之间的唯一合同，两边对着同一份文件写代码。
