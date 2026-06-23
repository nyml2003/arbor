---
name: arbor-skill-manager-usage
description: 使用 Arbor 技能管理器管理智能体技能安装清单、显式版本、校验、预演安装、真实安装、锁文件和清理。用于需要编写 arbor.skills.json、安装本地 path 来源技能、检查锁文件、清理无效技能，或向用户说明 Arbor 技能管理器用法时。
---

# Arbor 技能管理器用法

这个技能只说明怎么使用 Arbor 技能管理器。不要解释命令行工具从哪里获取，也不要替用户安装命令行工具。

## 核心概念

Arbor 技能管理器管三类对象：

- `SourceSkill`：来源目录。必须有 `SKILL.md`。
- `SkillPackage`：规范化后的包。必须有 `SKILL.md` 和 `skill.package.json`。
- `InstalledSkill`：复制到目标目录后的技能。

当前运行时先支持 `path` 来源。Git、tarball、npm 可以写入规范，但运行时安装还没有接入。

## 写安装清单

在目标工作区写 `arbor.skills.json`：

```json
{
  "schema": "arbor.skills/v1",
  "targetDir": ".codex/skills",
  "skills": [
    {
      "id": "arbor/arbor-skill-manager-usage",
      "version": "1.0.0",
      "source": {
        "type": "path",
        "path": "packages/arbor-skills/skills/arbor-skill-manager-usage"
      }
    }
  ]
}
```

规则：

- `schema` 必须是 `arbor.skills/v1`。
- `targetDir` 是安装目标目录。
- `id` 使用 `namespace/name`。
- `version` 必须是写死的 SemVer。不能省略，不能写 `latest`，不能写范围。
- `source.type` 当前优先写 `path`。
- `source.path` 相对清单所在目录解析，除非写绝对路径。

## 写包元数据

受 Arbor 管理的技能要写 `skill.package.json`：

```json
{
  "schema": "arbor.skill-package/v1",
  "id": "arbor/example-skill",
  "name": "example-skill",
  "version": "1.0.0",
  "format": "agent-skill",
  "files": [
    "SKILL.md",
    "agents/openai.yaml",
    "skill.package.json"
  ]
}
```

规则：

- `id` 必须等于 `arbor.skills.json` 里的 `skills[].id`。
- `version` 必须等于 `arbor.skills.json` 里的 `skills[].version`。
- `name` 必须等于 `SKILL.md` 元信息里的 `name`。
- `files` 必须包含 `SKILL.md` 和 `skill.package.json`。
- 不要写 `dependencies`。v1 不支持依赖。

没有 `skill.package.json` 的第三方技能也能安装，但清单里必须显式写 `id` 和 `version`。安装副本会生成 `skill.package.json`，来源目录不会被修改。

## 常用命令

先校验清单和来源：

```powershell
arbor skill lint --manifest arbor.skills.json --cwd .
```

预演安装，不写目标目录和锁文件：

```powershell
arbor skill install --manifest arbor.skills.json --cwd . --dry-run
```

真实安装：

```powershell
arbor skill install --manifest arbor.skills.json --cwd .
```

安装后会生成或更新 `arbor.skills.lock.json`。锁文件记录实际安装版本、来源、内容哈希和目标路径。

清理无效安装和陈旧锁文件：

```powershell
arbor skill prune --manifest arbor.skills.json --cwd .
```

需要先看会删什么，就加 `--dry-run`：

```powershell
arbor skill prune --manifest arbor.skills.json --cwd . --dry-run
```

## 推荐流程

1. 写或更新 `arbor.skills.json`。
2. 确认每个技能都有写死的 `version`。
3. 对受管理技能，确认 `skill.package.json` 和清单的 `id/version/name` 对齐。
4. 运行 `arbor skill lint`。
5. 运行 `arbor skill install --dry-run`。
6. 确认计划安装的目标目录正确。
7. 运行 `arbor skill install`。
8. 检查 `arbor.skills.lock.json`。

## 不要这样做

- 不要写 `latest`。
- 不要写 `^1.0.0`、`~1.0.0` 这类范围。
- 不要把包元数据塞进 `SKILL.md` 元信息。
- 不要依赖隐式安装。需要的技能都要显式列在 `arbor.skills.json`。
- 不要在安装阶段执行 `scripts/`。
- 不要把手动临时文件放进安装目标目录。更新安装会覆盖目标技能。
