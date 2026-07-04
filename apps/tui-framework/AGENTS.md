# TUI Framework 项目指南

## 概述

Arbor 下自研 TUI 框架的孵化项目。用 TEP（TUI Enhancement Proposal）流程驱动设计讨论和决策。

## 目录结构

```
apps/tui-framework/
├── tep-ops/           # TEP 管理工具（Python stdlib，DDD 四层架构）
│   ├── cli/           # argparse 子命令路由
│   ├── app/           # 用例层（ManageTep CRUD）
│   ├── domain/        # 领域模型（TepProposal + TepStatus 状态机）
│   ├── infra/         # 基础设施抽象（FileSystem, EventBus, CommandRunner）
│   └── config.json    # 路径配置
├── docs/
│   └── TEPs/          # TEP 提案文件（Markdown + YAML frontmatter）
└── AGENTS.md          # 本文件
```

## TEP 管理命令

```bash
cd apps/tui-framework
python tep-ops tep create "提案标题"         # 从模板创建新 TEP
python tep-ops tep list                      # 列出所有 TEP
python tep-ops tep list --status Draft       # 按状态过滤
python tep-ops tep show TEP-0001             # 显示完整内容
python tep-ops tep update TEP-0001 --status Review  # 更新状态
```

## TEP 状态机

```
Draft → Review → Accepted → Implemented → Final
任意节点 → Rejected / Withdrawn
```

## TEP Area 枚举

| area | 含义 |
|------|------|
| `architecture` | 整体架构、模块划分、数据流 |
| `rendering` | 字符网格渲染、ANSI escape、diff 算法 |
| `input` | 键盘/鼠标事件、粘贴检测、焦点系统 |
| `layout` | 布局引擎、flexbox、尺寸计算 |
| `widgets` | 组件设计、生命周期、组合模式 |
| `styling` | 主题系统、颜色、样式继承 |
| `ecosystem` | 工具链、CLI、文档、测试框架 |

## 代码风格

- Python 3.12+，纯 stdlib，不引入第三方依赖
- DDD 四层架构：cli → app → domain ← infra
- 命名：文件 snake_case，类 PascalCase，函数 snake_case
- 所有文件 I/O 通过 `infra/filesystem.py` 的 `FileSystem` 抽象

## 提交风格

```
[tui] 简短描述
```
