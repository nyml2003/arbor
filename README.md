# Arbor

个人数字工作台，也是一座工具孵化仓库。

## 一句话

Arbor 用一棵文件树组织个人工具、经验、计划和展示内容。它先作为个人孵化器运转，成熟项目再拆成独立仓库。

```
  build（做出来）
    ↓
  learn（学到了什么）
    ↓
  manage（下一步做什么）
    ↓
  show（给外界看）
    ↓  [反馈/动力]
  build（继续做）
```

## 四个环节

| 环节 | 要解决的问题 | 当前承载 |
|------|------------|---------|
| **build** | 怎么把东西做出来 | `apps/`、`packages/` |
| **learn** | 怎么把经验留下来 | `workspace/learn/` |
| **manage** | 怎么让一切有序 | `workspace/manage/` |
| **show** | 怎么把结果展示出去 | `workspace/show/`、容器内展示页 |

## 当前状态

Arbor 已经进入多项目孵化阶段：

- `apps/container`：Arbor 主容器，Electron + SolidJS，已支持文件树、Markdown 预览、Manage 面板、Resume、memvfs 和 Shamrock 展示页。
- `apps/capture`：Windows 截图工具，Tauri 2 + Rust-first。
- `apps/keydock`、`apps/clipdock`：Rust 原生 GUI 工具样本。
- `apps/memvfs`：Rust in-memory VFS daemon/CLI 实验。
- `apps/aster`：调用 DeepSeek API 的本地 agent CLI，支持流式输出、连续对话、终端 Markdown 渲染和本地 skill 注入。
- `apps/shamrock`：Rust 宝可梦对战模拟引擎，保留 core/view/replay/CLI 边界，后续再评估接入容器。
- `apps/thorn`：Rust TUI 框架实验，已有响应式到内存渲染的 MVP 纵向切片。
- `packages/arbor-ui-core`、`packages/arbor-ui-windows`：Rust 原生 GUI 基础层。
- `packages/manage-core`、`packages/manage-cli`：管理域 v1，提供任务 core、JSON 文件存储和 `arbor-manage` CLI。
- `packages/skill-manager-core`、`packages/skill-manager-cli`：agent skill 管理器 v1，已支持 path source、显式版本校验、安装、lock 和 prune。
- `packages/arbor-skills`：Arbor 自维护 Skill 集合，用于试运行 skill manager。
- `workspace/learn`、`workspace/manage`、`workspace/show`：经验、治理和展示内容。

## 怎么开始

1. 读 `VISION.md` 了解终态和引擎模型
2. 读 `PLAN.md` 了解迭代路线图
3. 读 `DECISIONS.md` 了解技术选择
4. 读 `CONVENTIONS.md` 了解协作规范
5. 读 `workspace/manage/repo-strategy.md` 了解哪些项目留在本仓库，哪些满足条件后拆仓

## 常用命令

```powershell
pnpm dev
pnpm build
pnpm typecheck
pnpm test
pnpm test:e2e
```
