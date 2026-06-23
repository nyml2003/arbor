---
name: skill-manager-maintainer
description: 维护 Arbor 智能体技能包和 Arbor 技能管理器。用于创建或更新 SKILL.md、skill.package.json、arbor.skills.json、arbor.skills.lock.json，或验证 SourceSkill -> SkillPackage -> InstalledSkill 行为时。
---

# 技能管理器维护器

按下面的规则维护 Arbor 管理的技能。

## 引用路由

- 包结构、元数据文件和 Codex 风格 skill 兼容：读 [references/skill-package-structure.md](references/skill-package-structure.md)。
- core/CLI 实现工作：读 [references/core-cli-architecture.md](references/core-cli-architecture.md)。
- 验证故事和命令：读 [references/validation-stories.md](references/validation-stories.md)。
- 来源类型和安全边界：读 [references/source-safety.md](references/source-safety.md)。

## 核心模型

保持这个模型不变：

```text
SourceSkill -> SkillPackage -> InstalledSkill
```

`SourceSkill` 是来源目录。它必须包含 `SKILL.md`。

`SkillPackage` 是规范化后的包。它必须包含 `SKILL.md` 和 `skill.package.json`。

`InstalledSkill` 是复制到清单 `targetDir` 下的结果。

## 包规则

- `SKILL.md` 元信息只保留 `name` 和 `description`。
- 包元数据放在 `skill.package.json`。
- `arbor.skills.json` 必须使用精确 SemVer。
- 拒绝版本范围、`latest` 和缺失版本。
- 如果来源有 `skill.package.json`，它的版本必须等于清单版本。
- 如果来源只有 `SKILL.md`，安装时用清单的 id/version 生成包元数据。
- 不要给 `skill.package.json` 加 `dependencies`。
- 安装阶段不要执行 `scripts/`。
- 没有包元数据的非受管技能，只有在 `arbor.skills.json` 提供精确 id/version 时才可安装。

## 验证流程

1. 校验本地技能前，先构建技能管理器 CLI。
2. 对清单运行 `arbor skill lint`。
3. 运行 `arbor skill install --dry-run`。
4. lint 和 dry-run 通过后再真实安装。
5. 来源、版本或包元数据变化后，检查 lock 输出。

## 安全

- 用 `targetDir` 显式指定安装目标。
- 不要写出 `targetDir`。
- 拒绝带符号链接的 payload。
- 把 Git branch 来源当作低信任来源。
- 远程来源优先使用 git tag、commit、tarball integrity 或精确 npm package version。
