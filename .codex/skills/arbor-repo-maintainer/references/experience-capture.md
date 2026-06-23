# 经验沉淀

## 意图

把反复出现的工程经验变成未来可执行的约束。

## 适用场景

- 某个模式出现在多个模块或项目里。
- 一个设计决策已经稳定，足以指导后续工作。
- 用户要求保留经验。
- 一个 skill 需要承载仓库特定维护知识。

## 必须遵守的规则

- 只沉淀稳定、可复用、有证据支持的经验。
- 直接证据和推断要分开。
- 广义仓库治理放到 Arbor 文档或 `arbor-repo-maintainer` reference。
- 狭窄的工作流或包维护知识放到对应 skill。
- `SKILL.md` 保持短，把详细规则包放到 `references/`。
- 不要把一次性 bug 或临时 workaround 写成长期规则。

## 推荐模式

- 可复用规则 reference 使用这个结构：
  - 意图
  - 适用场景
  - 必须遵守的规则
  - 推荐模式
  - 反模式
  - 脚手架影响
  - 证据
  - 推断说明
- 项目无关工程模式放在 `workspace/learn/patterns/`。
- agent 可执行维护规则放在 `packages/arbor-skills/skills/*/references/`。
- 模式在 Arbor 外也有用时，不要绑定 Arbor 特定路径。
- 模式只服务 Arbor 维护时，路径和命令要写具体。

## 反模式

- 在 `SKILL.md` 里写长篇文章。
- 把历史叙事和可执行规则混在一起。
- 只有一个例子且没有用户确认时，就宣称规则稳定。
- 同一条规则复制到多个 skill，而不是路由到一个聚焦 reference。

## 脚手架影响

新增用于沉淀经验的 skill 时：

- 用 `SKILL.md` 作为触发和路由入口。
- 详细规则放在一层深度的 `references/` 文件里。
- 添加带精确版本的 `skill.package.json`。
- 把 skill 加到 `arbor.skills.json`。
- 运行 lint、dry-run 和真实 install，刷新 lock。

## 证据

- `workspace/learn/patterns/README.md` indexes reusable engineering patterns by source project and topic.
- `workspace/learn/patterns/ts-runtime-performance-rules.md` 区分范围、规则、反模式、清单和来源。
- `workspace/learn/patterns/skill-validation-pipeline.md` 识别跨语言模式，并列出来源证据。
- `packages/arbor-skills/skills/skill-manager-maintainer` keeps skill manager rules in an installable skill.

## 推断说明

Arbor 有两类知识表面：给人读的学习文档，以及给 agent 执行的 skill。会影响未来编码行为的规则，优先沉淀成 skill。
