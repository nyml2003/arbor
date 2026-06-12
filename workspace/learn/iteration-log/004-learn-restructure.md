# 004 - learn/ 结构整理

日期：2026-06-07

## 做了什么

1. Pattern 脱耦：删除 6 篇 pattern 中所有的 Arbor 引用段（"可以在 Arbor 中复制什么"、"为什么对 Arbor 有用"等）。模式文档现在独立描述每个项目
2. iteration-log 合并：003/004/005 三篇经验提取日志合并为 `003-experience-extraction.md`
3. 删除 `sops/electron-solid-pnpm-setup.md`——是技术操作手册，不是 Arbor 四环节 SOP
4. 新建索引：`learn/README.md`（入口+STATUS）、`patterns/README.md`（模式索引）、`sops/README.md`（SOP 定义）
5. MIGRATION.md 移至 `manage/migration.md`，docs/ 目录删除
6. build/ 和 show/ 暂未加 README——等确定内容方向再写

## 决策

1. Pattern 独立——描述项目本身，不通过任何其他项目的视角筛选
2. SOP 先不写——走通流程前写的 SOP 是倒置的
3. iteration-log 保持按时间线——不做总结性索引（总结在 STATUS 里）

## learn/ 最终结构

```
learn/
├── README.md                   入口 + STATUS
├── iteration-log/              4 篇过程记录
├── patterns/                   10 篇独立模式文档 + 索引
├── sops/                       四环节 SOP 定义（待写）
└── retrospectives/             2 篇认知修正
```
