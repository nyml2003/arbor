# 技能包结构

## 意图

让 Arbor Skill 包兼容常见 agent skill 目录，同时增加严格安装元数据。

## 适用场景

- 创建或更新 `SKILL.md`。
- 创建或更新 `skill.package.json`。
- 把非受管第三方 skill 加到 `arbor.skills.json`。
- 判断元数据应该放在 front matter、package JSON 还是 references。

## 必须遵守的规则

- `SKILL.md` front matter 只包含 `name` 和 `description`。
- `description` 必须包含触发条件。不要只在正文里写 “什么时候使用”。
- 包元数据放在 `skill.package.json`。
- `skill.package.json` 必须包含精确的 `schema`、`id`、`name`、`version`、`format` 和 `files`。
- `arbor.skills.json` 中每个 skill 都必须包含精确 `id`、`version` 和 `source`。
- 版本没有可选项。manifest 没有精确版本时，不安装。
- 不要在 skill package metadata 中添加 `dependencies`。
- reference 文件只放在 `references/` 下一层，并从 `SKILL.md` 路由过去。

## 推荐模式

- 使用 `agents/openai.yaml` 存 UI 元数据。
- `agents/openai.yaml` 的 `interface.default_prompt` 要提到 `$skill-name`。
- 非受管第三方 skill 安装时，由 manifest id/version 在安装副本里生成 `skill.package.json`。
- vendored 非受管来源可以使用 `0.0.0-vendor.20260621` 这类预发布版本。
- `SKILL.md` 保持简短；大型规则包放进 `references/`。

## 反模式

- 把 Arbor 包元数据写进 Markdown front matter。
- 让 `SKILL.md` 变成 README 或设计历史。
- 在受管 source 里把包元数据标成可选。
- 使用 `^1.0.0`、`~1.0.0` 或 `latest` 这类范围。
- Arbor 还没有依赖模型时，就添加依赖声明。

## 脚手架影响

新增 Arbor 受管 skill 时：

- 创建 `SKILL.md`。
- 创建 `agents/openai.yaml`。
- 创建 `skill.package.json`。
- 把 skill 加到 `packages/arbor-skills/arbor.skills.json`。
- 运行 `pnpm --filter @arbor/skills skill:lint`。
- 运行 `pnpm --filter @arbor/skills skill:install:dry-run`。
- 运行 `pnpm --filter @arbor/skills skill:install` 刷新 lock。

## 证据

- `packages/arbor-skills/skills/plain-tech-writing-cn` 使用 `SKILL.md`、`agents/openai.yaml` 和 `skill.package.json`。
- `packages/arbor-skills/arbor.skills.json` 固定精确版本。
- `packages/skill-manager-core/src/application/normalize.ts` 只为非受管来源生成包元数据。
- `packages/skill-manager-core/src/domain/front-matter.ts` 只把 `name` 和 `description` 解析为必需的 skill 触发元数据。

## 推断说明

这个结构是兼容桥：普通 `SKILL.md` 目录仍然可用，同时 Arbor 安装结果可复现。
