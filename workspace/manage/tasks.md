# Arbor 任务清单

最后更新：2026-06-07

---

## 第一优先级：经验沉淀到 learn/（agent 执行）

旧项目的价值在**经验**，不在代码。删之前，agent 读代码、抽模式、写成 learn/ 下的可复用记录。经验没抽出来之前不动项目。

### 值得抽经验的项目（代码里有设计）

| 项目 | 要抽的经验 | 预估 | 状态 |
|------|-----------|------|------|
| ObolosFS | VFS 纯逻辑核心设计、Driver/Capability 模型、Result-based error | 大 | ✅ `vfs-pure-logic.md` |
| workshop (Rust) | 任务实体建模、状态机、CLI 命令体系 | 中 | ✅ `task-domain-model.md` |
| work-context-2 | Python 六边形架构、投影模式、知识管道 | 中 | ✅ `projection-and-knowledge-pipeline.md` |
| work-context (Python) | Skill 校验管线、Result/Option、DI 容器 | 中 | ✅ `skill-validation-pipeline.md` |
| jue | 自研 VDOM 的核心设计思路 | 小 | ✅ `minimal-reactivity-vdom.md` |
| Aura | SSR 内联数据模式 | 小 | ✅ `lightweight-ssr-inline-data.md` |
| WatchDesk | PageRegistry、Terminal 流式管道 | 小 | ⊘ 底层模式已被 Phase 1 覆盖 |
| ventus (C:) | BFF 设计、静态生成链 | 小 | ⊘ 与 Aura 模式重复（Go 版 inline data） |
| egui | 立即模式 GUI 模板 | 小 | ⊘ 脚手架模板，30 行包装，无原创 |

### 明确不需要抽的项目（纯练习/模板/实验，代码里没有独特设计）

这些项目是学习过程的产物——写一遍就懂了，代码本身没有值得留的模式。**经验不在代码里，在脑子里。**

| 项目 | 类型 | 说明 |
|------|------|------|
| leetcode | 算法练习 | 刷题记录，无架构 |
| untitled~3 | Kotlin 实验 | 语法熟悉阶段 |
| demo-nest / koa-backend | 框架演示 | 官方 tutorial 级别 |
| project/vue-tsx-tailwind | Vue 模板 | 脚手架生成 |
| project/java | Java 实验 | 语法练习 |
| ai (Python ML) | ML 实验 | 跑过就算 |
| js (TS) | JS 工具 | 零散脚本 |
| quickjs | C 编译 | 编译别人的代码，无原创设计 |
| porygon / rust-script / rustify_web | Rust 早期 | 语法学习期产物 |
| uemerald-memhack / my-mh | 内存修改 | 特定游戏工具，无通用性 |
| common-event-bus | C++ 事件总线 | 练习模式实现 |
| zipfiles | C++ 文件备份 | 工具脚本 |
| visual-frame | 虚拟滚动 | 库使用练习 |
| project/game | Django 游戏 | 框架练习 |
| project/deneb | 微前端壳 | 框架实验 |
| MatrixSlow | Python ML | 跟书实现 |

---

## 第二优先级：旧项目清理（用户执行）

> ⚠️ **agent 不能删文件。以下所有删除操作由用户自己在终端或资源管理器中执行。**

### 2.1 立即可删（空目录，零价值）

```powershell
Remove-Item -Recurse -Force C:\Users\nyml\code\materials
Remove-Item -Recurse -Force C:\Users\nyml\code\repos
Remove-Item -Recurse -Force C:\Users\nyml\code\knowledge-candidates
```

### 2.2 可删（学习产物，经验在脑子里不在代码里）

```powershell
# C 盘 — 全部删除
Remove-Item -Recurse -Force C:\Users\nyml\code\dll_csv_transformer
Remove-Item -Recurse -Force C:\Users\nyml\code\my-mh
Remove-Item -Recurse -Force C:\Users\nyml\code\toml_resume
Remove-Item -Recurse -Force C:\Users\nyml\code\hub
Remove-Item -Recurse -Force C:\Users\nyml\code\untitled
Remove-Item -Recurse -Force C:\Users\nyml\code\untitled2
Remove-Item -Recurse -Force C:\Users\nyml\code\untitled3

# D 盘 — 全部删除
Remove-Item -Recurse -Force D:\code\project\blog_back
Remove-Item -Recurse -Force D:\code\project\next-ventus-container
Remove-Item -Recurse -Force D:\code\project\deneb
Remove-Item -Recurse -Force D:\code\project\vue-tsx-tailwind
Remove-Item -Recurse -Force D:\code\project\changfen
Remove-Item -Recurse -Force D:\code\project\tensorslow_release
Remove-Item -Recurse -Force D:\code\project\others
Remove-Item -Recurse -Force D:\code\project\demo-nest
Remove-Item -Recurse -Force D:\code\project\koa-backend
Remove-Item -Recurse -Force D:\code\project\game
Remove-Item -Recurse -Force D:\code\project\java
Remove-Item -Recurse -Force D:\code\project\MatrixSlow
Remove-Item -Recurse -Force D:\code\common-event-bus
Remove-Item -Recurse -Force D:\code\zipfiles
Remove-Item -Recurse -Force D:\code\uemerald-memhack
Remove-Item -Recurse -Force D:\code\my-mh
Remove-Item -Recurse -Force D:\code\porygon
Remove-Item -Recurse -Force D:\code\rust-script
Remove-Item -Recurse -Force D:\code\rustify_web
Remove-Item -Recurse -Force D:\code\quickjs
Remove-Item -Recurse -Force D:\code\visual-frame
Remove-Item -Recurse -Force D:\code\ai
Remove-Item -Recurse -Force D:\code\js
Remove-Item -Recurse -Force D:\code\leetcode
```

### 2.3 经验抽完再删

| 项目 | 先由 agent 抽经验 | 抽完后处理 |
|------|------------------|-----------|
| ObolosFS | → `learn/patterns/vfs-pure-logic.md` | 不动（活跃项目，移入 packages/） |
| workshop | → `learn/patterns/task-domain-model.md` | 删除 |
| work-context | → `learn/patterns/skill-validation.md` | 删除 |
| work-context-2 | → 与 work-context 合并为一条 | 删除 |
| WatchDesk | → `learn/patterns/page-registry-pattern.md` 等 | Phase 4 后删除 |
| ventus (C:) | → `learn/patterns/static-generation.md` | Phase 1 后归档 |
| ventus- (D:) | → 与 ventus 合并 | Phase 4 后删除 |
| Aura | → `learn/patterns/ssr-monorepo.md` | Phase 1 后归档 |
| egui | → `learn/patterns/immediate-mode-gui.md` | 归档 |
| jue | → `learn/patterns/vdom-design.md` | 归档 |
| tasks/ my-task-1/ skills/ | → 与 workshop 合并，记录 workc 任务空间概念 | 删除 |

---

## 第三优先级：Arbor 容器建设

- [ ] **Markdown 渲染预览** — 点 `.md` 文件时渲染为 HTML 而非纯文本
- [ ] 语法高亮（代码块）
- [ ] **静态站点导出**（Phase 4 核心交付物）— 文件树 → 可浏览网页

---

## 第四优先级：build/ 分支规划

- [ ] 决定 ObolosFS 是否移入 Arbor 的 `packages/`
- [ ] 决定透出的最终形态（网站？桌面应用导出？）
- [ ] 决定 build/ 下项目的纳入标准
