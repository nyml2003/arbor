# learn/ — 经验沉淀

## 当前状态（STATUS）

| 环节 | 完成度 | 说明 |
|------|--------|------|
| build | 10% | 容器能跑，文件树能看。build/ 分支空，没有工具 |
| learn | 60% | 8 个旧项目的设计模式已抽取，10 篇 pattern + 4 篇 iteration-log。结构整理完成 |
| manage | 20% | 任务清单、项目清单、迁移计划都在 manage/——但还没开始执行 |
| show | 0% | 空 |

关键待做：
- 容器加 Markdown 渲染（让 learn/ 内容可读）
- 执行旧项目清理（manage/tasks.md 和 manage/migration.md 里的清单）
- 静态站点导出（让 show/ 有东西）

## 阅读路径

```
learn/
├── README.md                   ← 你在这里
├── iteration-log/              ← 按时间线的过程记录
│   ├── 000-arbor-成立
│   ├── 001-Phase1-容器搭建
│   ├── 002-manage-任务规划
│   └── 003-经验沉淀（合并）
├── patterns/                   ← 从各种项目中提取的设计模式
│   └── README.md               ← 模式索引
├── sops/                       ← Arbor 四环节的操作流程
│   └── README.md               ← SOP 目录说明
└── retrospectives/             ← 认知修正类复盘
    ├── agent-rush-to-code.md
    └── structure-review-20260607.md
```

新读者建议：先看 `iteration-log/000` 了解 Arbor 怎么来的 → 再按兴趣选择 patterns/ 或 retrospectives/。
