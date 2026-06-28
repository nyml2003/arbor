# Arbor 仓库指南

## 概述

Arbor 是一个个人产出引擎的孵化器 monorepo。两条线并行：
- **主线**：容器应用 + 经验沉淀 + 管理展示
- **孵化线**：`apps/` 和 `packages/` 下的独立工具

## 目录结构

```
apps/           # 可运行应用和孵化工具
  container/    # Arbor 主容器 (Electron + SolidJS)
  capture/      # 截图工具 (Tauri + Rust)
  ...
packages/       # 可复用基础设施
  kaubo-features/  # Kaubo 编程语言 (Rust + Web + VSCode)
  manage-core/     # 管理域核心
  manage-cli/      # 管理域 CLI
  ...
workspace/      # 经验、治理和展示数据
scripts/        # 发布/构建脚本
```

## 子项目指引

部分子项目有自己的 AGENTS.md，进入对应目录后优先读取：

| 目录 | 指引文件 | 关键构建工具 |
|------|----------|-------------|
| `packages/kaubo-features/` | `AGENTS.md` | **`python kaubo-ops`** |
| `apps/container/` | — | pnpm |
| `packages/manage-core/` | — | pnpm + vitest |
| `packages/manage-cli/` | — | pnpm + vitest |

## Kaubo 开发

当你在 `packages/kaubo-features/` 下工作时：

**必须优先用 `python kaubo-ops <cmd>`，不要手写 `cd next_kaubo && cargo check ...` 这类裸命令。** 所有任务定义在 `packages/kaubo-features/kaubo-ops/`（DDD 四层架构，纯 Python stdlib）。

常用命令：

```bash
python kaubo-ops ci           # 标准 CI（check + clippy + fmt + test + web + vscode）
python kaubo-ops check        # 快速类型检查（无测试）
python kaubo-ops build-wasm   # WASM 双目标构建（web + nodejs）
python kaubo-ops test-rust    # Rust 测试
python kaubo-ops test-web     # Web 测试
python kaubo-ops lint         # 全部 lint（cliopy + eslint）
python kaubo-ops fmt          # 全部格式化
python kaubo-ops dev          # 开发服务器
python kaubo-ops coverage     # 覆盖率
```

Ops2 架构：`cli/`（表示层）→ `app/`（用例）→ `domain/`（领域模型）← `infra/`（基础设施），配置集中在 `kaubo-ops/config.json`。

进入 kaubo 子项目时，先读 `packages/kaubo-features/AGENTS.md` 了解详细工作规则、分层约束和测试策略。

## 构建工具链（Arbor 本体）

- **pnpm 10+** — 包管理
- **Vitest** — 测试
- **eslint** — 代码检查
- **tsc** — 类型声明生成
- **SWC** — JS/TS 转译

根 `package.json` 定义了 workspace 级别的脚本：`pnpm dev`、`pnpm build`、`pnpm test`、`pnpm lint`。

## 代码风格

- TypeScript strict mode，ESM only
- 全部 named export，不用 default export
- 函数优先，少用 class
- 文件名 kebab-case，测试和源文件同目录
- 不使用 yarn / npm / bun

## 重要文档

| 文档 | 内容 |
|------|------|
| `PLAN.md` | 迭代路线图和 Phase 状态 |
| `DECISIONS.md` | 技术决策记录 |
| `CONVENTIONS.md` | 完整协作规范 |
| `VISION.md` | 项目愿景 |
| `workspace/learn/` | 经验沉淀和迭代日志 |

## 提交风格

```
[domain] 简短描述
```

domain 示例：`[container]`、`[manage]`、`[learn]`、`[kaubo]`、`[docs]`、`[framework]`。

## 默认行为

- 默认用中文回复
- 做改动前先看相关代码和文档
- 优先做最小、局部的修改
- 新增 app/package 先标注归属，默认留在本仓库
- 完成 Phase 后更新 `PLAN.md`
- 技术决策追加到 `DECISIONS.md`
