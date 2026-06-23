# 仓库治理

## 意图

让 Arbor 保持为可用的个人工具孵化器，而不是一堆互不相干的 app。

## 适用场景

- 新增 app 或 package。
- 在 `apps/`、`packages/`、`workspace/` 之间移动代码。
- 修改 `README.md`、`PLAN.md`、`DECISIONS.md` 或 `CONVENTIONS.md`。
- 判断项目应该留在 Arbor，还是迁到独立仓库。

## 必须遵守的规则

- 创建新 app/package 前，先分类：Arbor core、孵化产品、技术样例、可复用库、经验沉淀。
- 可运行产品和实验放在 `apps/`。
- 可复用基础设施放在 `packages/`。
- 知识、治理、计划和展示数据放在 `workspace/`。
- 不要只为了目录好看就拆独立仓库。
- 只有项目有独立用户或发布目标、能独立构建/测试/阅读、不依赖私有 `workspace/` 数据，并且独立发布、复用、权限或历史管理有收益时，才拆仓。
- `README.md` 保持短且更新。详细 roadmap 和状态放到 `PLAN.md` 或 `workspace/manage/*`。

## 推荐模式

- 可见项目清单变化时，更新 `README.md`。
- 阶段、孵化线或基础设施状态变化时，更新 `PLAN.md`。
- 持久技术选择变化时，更新 `DECISIONS.md`。
- 新编码约定稳定后，更新 `CONVENTIONS.md`。
- 可复用工程模式放在 `workspace/learn/patterns/`。
- 迭代历史放在 `workspace/learn/iteration-log/NNN-short-slug.md`。

## 反模式

- 项目的构建、测试、README 和使用边界还没稳定，就迁出 Arbor。
- 一个领域明明适合 `apps/`、`packages/` 或 `workspace/`，却新增顶层目录。
- 让 `README.md` 变成长篇计划文档。
- 没有实际项目证据，就把泛泛建议写进 `workspace/learn`。

## 脚手架影响

新增项目时：

- 可运行项目放到 `apps/<name>`。
- 可复用基础设施放到 `packages/<domain>-<layer>`。
- 项目进入 Arbor 活跃清单后，在 `README.md` 和 `PLAN.md` 中增加或更新状态。
- 考虑拆仓前，先补独立 README、构建命令和测试命令。

## 证据

- `README.md` defines Arbor as a build/learn/manage/show loop.
- `PLAN.md` keeps incubated products in `apps/` and infrastructure in `packages/`.
- `CONVENTIONS.md` defines `apps/`, `packages/`, `workspace/`, and package ownership rules.
- `DECISIONS.md` decision 12 records the incubator monorepo policy.

## 推断说明

这个仓库偏向务实的孵化器模型。新增结构需要比“目录看起来更整齐”更强的证据。
