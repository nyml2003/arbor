# Arbor Skills

Arbor 自维护 Skill 集合。这个包用于试运行 `@arbor/skill-manager-core` 和 `@arbor/skill-manager-cli`。

## 当前 Skill

| Skill | 用途 |
|------|------|
| `arbor-repo-maintainer` | 维护 Arbor monorepo 的项目归属、TS 包结构、验证和经验沉淀规则 |
| `arbor-skill-manager-usage` | 说明 Arbor 技能管理器的清单、校验、安装、锁文件和清理用法 |
| `domain-core-architect` | 设计纯领域核心、状态机、Repository/Driver 边界和结构化错误模型 |
| `electron-solid-workbench` | 维护 Electron + SolidJS 工作台、IPC、preload bridge 和递归文件树 |
| `knowledge-pattern-maintainer` | 判断项目经验该写入 pattern、决策、迭代日志还是 skill |
| `plain-tech-writing-cn` | 写和改中文技术文档，保持直接、具体、可执行 |
| `rust-native-gui-tool` | 维护 Rust 原生 GUI 工具、primitive tree、平台适配层和 unsafe 边界 |
| `skill-manager-maintainer` | 维护 Arbor Skill 包结构、manifest、lock 和验证流程 |
| `tauri-rust-system-tool` | 维护 Tauri 2 + Rust-first 系统工具、静态 overlay 和系统能力边界 |

## 使用

先构建 CLI：

```powershell
pnpm --filter @arbor/skill-manager-cli build
```

校验：

```powershell
pnpm --filter @arbor/skills skill:lint
```

预演安装：

```powershell
pnpm --filter @arbor/skills skill:install:dry-run
```

真实安装会写入 `.installed-skills/` 和 `arbor.skills.lock.json`：

```powershell
pnpm --filter @arbor/skills skill:install
```

安装仓库本地 Codex skill 到 `.codex/skills/`：

```powershell
pnpm --filter @arbor/skills skill:codex:install
```
