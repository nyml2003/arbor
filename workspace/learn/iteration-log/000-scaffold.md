# 000 - Arbor 项目脚手架搭建

日期：2026-06-07

## 做了什么

1. 与 opencode 进行了多轮讨论，梳理了 C:\Users\nyml\code（33 个项目）和 D:\code（16 个顶层目录）的全部项目
2. 识别出核心痛点：项目分散、不成系统、没有统一迭代计划、产物丢失
3. 提炼出四个环节的引擎模型：迭代 → 沉淀 → 管理 → 透出
4. 确定了产品形态：Electron + SolidJS 桌面容器，文件树 UI，四个分支
5. 选定了项目名：Arbor（拉丁语「树」）
6. 建立了目录骨架和全套文档：
   - README.md — 项目门面
   - VISION.md — 终态全景和引擎模型
   - PLAN.md — 四阶段迭代路线图
   - MIGRATION.md — 50+ 旧项目的迁移/清理计划
   - DECISIONS.md — 8 项技术选型及理由
   - CONVENTIONS.md — 编码和协作规范
   - .gitignore

## 决策

1. **名字选 Arbor** — 树的隐喻贯穿全局（文件树 UI、四个分支、每次迭代长叶子）
2. **容器用 Electron + SolidJS** — 而非 Tauri，因为用户有经验且 Rust 迭代慢
3. **管理/沉淀用 TypeScript** — 而非 Rust，因为快速迭代优于性能
4. **文件存储而非数据库** — 与文件树隐喻一致，零依赖
5. **monorepo 用 pnpm workspace** — 用户已有成熟经验
6. **CLI 用 bun compile 分发** — 满足零运行时依赖
7. **core/cli/ui 三层拆分** — 逻辑与交互壳解耦
8. **暂不引入 Rust** — 等引擎跑通后再说

## 下一步（交给下一轮 agent）

Phase 1：搭建容器应用

- 初始化 pnpm workspace monorepo
- 搭建 Electron + SolidJS + Vite 的最小骨架
- 实现左侧文件树组件（读取本地目录，显示四个空分支）
- 实现右侧内容区（文本占位，选中节点有反应）
- 确保 `pnpm install && pnpm dev` 能跑起来

参考项目：
- `C:\Users\nyml\code\WatchDesk` — Electron + SolidJS 架构，pnpm workspace，分层模式
- 不直接复制代码，只借架构思路

## 上下文衔接

本日志写给下一个 agent。读完以下文档即可理解 Arbor 的全局：

1. README.md — 是什么
2. VISION.md — 终态什么样
3. PLAN.md — 怎么分步走（当前 Phase 1）
4. DECISIONS.md — 技术为什么这么选
5. CONVENTIONS.md — 代码怎么写
