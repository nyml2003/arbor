# Repo Strategy — Arbor 仓库维护策略

日期：2026-06-21

## 结论

Arbor 继续作为个人工具孵化 monorepo。它不是单一产品仓库，也不是所有项目都要发布的包仓库。

当前目标是先把工具、经验、计划和展示内容放在同一棵树里演化。项目成熟后再拆独立 git 仓库。

## 三层资产

| 层级 | 定义 | 当前内容 | 处理 |
|------|------|----------|------|
| 主线层 | Arbor 本体和四环节数据 | `apps/container`、`workspace/learn`、`workspace/manage`、`workspace/show` | 长期留在本仓库 |
| 孵化层 | 有独立产品方向，但边界还在变化 | `apps/capture`、`apps/keydock`、`apps/clipdock`、`apps/memvfs`、`apps/shamrock` | 先留在本仓库 |
| 基础设施层 | 多个孵化项目共享的能力或未来包 | `packages/arbor-ui-core`、`packages/arbor-ui-windows`、`packages/skill-manager-core` | 跟随使用方演化 |

## 拆仓条件

一个项目同时满足以下条件，才考虑拆独立 git 仓库：

1. 有独立用户或独立发布目标。
2. 可以独立构建、测试和阅读 README。
3. 不依赖 Arbor 私有 `workspace/` 数据。
4. 未来数次迭代大概率不需要和 Arbor 主线一起修改。
5. 拆仓能带来明确收益：发布、复用、权限隔离或历史清晰度。

不为目录整洁拆仓。

## 当前项目处置

| 项目 | 类型 | 当前判断 | 下一步 |
|------|------|----------|--------|
| `apps/container` | Arbor 本体 | 不拆 | 继续补 Markdown 预览、展示页和静态导出 |
| `workspace/learn` | 经验沉淀 | 不拆 | 继续维护 pattern 索引和复盘 |
| `workspace/manage` | 治理层 | 不拆 | 维护任务、迁移表和本策略 |
| `workspace/show` | 展示数据 | 不拆 | 继续放简历和未来作品集数据 |
| `apps/capture` | 孵化产品 | v1 主链路跑通后优先评估拆仓 | 先完成截图、缓存、剪贴板、通知和打开文件 |
| `apps/keydock` | 孵化产品 / native GUI 样本 | 暂不拆 | 先验证日常可用性和 Windows 边界 |
| `apps/clipdock` | 孵化产品 / native GUI 样本 | 暂不拆 | 继续作为第二个 native GUI 使用方 |
| `apps/memvfs` | 系统工具实验 | 暂不拆 | 先确定是实验、日常工具还是可复用库 |
| `apps/shamrock` | Rust 游戏引擎实验 | 暂不拆 | 先保持 core/view/replay/CLI 边界，后续再评估 container 接入 |
| `packages/arbor-ui-core` | Rust GUI 基础层 | 暂不拆 | 等两个以上工具持续依赖稳定 API 后再评估 |
| `packages/arbor-ui-windows` | Windows GUI adapter | 暂不拆 | 继续把 Win32/unsafe 限制在平台边界 |
| `packages/skill-manager-core` | Skill 管理器规范 | 暂不拆 | 先实现 core/cli 和真实安装场景 |

## 维护规则

- 每个 `apps/*` 至少要有 README，说明目标、范围、非目标和开发命令。
- 每个可运行项目要有测试或检查命令。没有自动化测试时，README 要写清手动验收。
- 每个 `packages/*` 要说明使用方。没有使用方的 package 只能作为规范或实验存在。
- 每次新增 app/package，要在 `CONVENTIONS.md` 的归属规则下能解释它的位置。
- 每次项目从孵化层进入可提取层，先更新本文件，再做迁移计划。

## 经验沉淀优先级

优先沉淀这些经验：

1. Electron/SolidJS 容器：IPC contract、文件树、web/electron 双宿主、Playwright 验收。
2. Tauri 截图工具：Rust 主链路、静态 overlay、settings 延迟加载、Windows shell 边界。
3. Rust native GUI：safe app DSL、primitive tree、Direct2D adapter、unsafe boundary。
4. Rust daemon/CLI：memvfs 的 core/daemon/cli 分层、协议边界和测试方式。
5. Rust 游戏引擎：Shamrock 的 data/core/mechanics/format/view/replay/CLI 分层和 replay 回归。
6. Agent skill 包管理：manifest/lock 分离、安装目标、安全校验。

## 拆仓流程

拆仓前先做这些检查：

1. 项目 README 可以独立解释项目，不依赖 Arbor 顶层文档。
2. 构建、测试、开发命令可以在项目目录内运行。
3. 依赖路径里没有 `workspace/` 私有数据。
4. 已经把可复用经验写入 `workspace/learn/patterns` 或迭代日志。
5. 已经确定迁出后本仓库只保留文档索引、子模块引用或删除原目录。

拆仓后，本仓库只保留必要入口，不保留两份活跃代码。
