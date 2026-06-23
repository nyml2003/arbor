---
name: arbor-repo-maintainer
description: 维护 Arbor monorepo 的项目治理、TypeScript 包结构、验证习惯和经验沉淀流程。用于新增或修改 app、package、workspace 文档、pnpm 脚本、TypeScript core/cli 模块，或维护仓库级文档时。
---

# Arbor 仓库维护器

把这个技能作为 Arbor 仓库级维护入口。只读取当前任务需要的 reference。

## 引用路由

- 新增 app/package、项目归属、拆仓判断或根文档：读 [repository-governance.md](references/repository-governance.md)。
- TypeScript core/cli 包、pnpm workspace、TS 模块边界：读 [typescript-package-rules.md](references/typescript-package-rules.md)。
- 测试、类型检查、构建、lint、安装校验或完成证据：读 [validation-rules.md](references/validation-rules.md)。
- 学习笔记、模式抽取，或把重复经验变成 skill/rule：读 [experience-capture.md](references/experience-capture.md)。如果要细分沉淀位置，使用 `$knowledge-pattern-maintainer`。

## 默认流程

1. 先判断改动属于 `apps/`、`packages/` 还是 `workspace/`。
2. 读取最小匹配的 reference。
3. 新建结构前先看相邻成熟模块。
4. 把 diff 限制在请求涉及的域内。
5. 运行最窄但有意义的验证命令。
6. 只有改动影响公开状态、项目归属、决策、约定或可复用经验时，才更新文档。

## 硬规则

- Arbor 默认保持孵化器 monorepo。项目满足拆仓条件后才迁出。
- 内部包使用 `@arbor/` scope。
- core 逻辑要和 CLI/UI 外壳分开。
- 个人工具和知识资产优先使用文件作为事实来源。
- 不要为了小便利新增依赖。
- 不要把一次性历史写成长期规则。
