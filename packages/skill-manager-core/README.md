# Skill 管理器设计

日期：2026-06-21

## 结论

Skill 管理器不要直接复用 npm、uv 或 Maven。它们管理的是语言包，不是 agent skill 这种工作流包。

v1 只做一件事：从明确来源拿到 Skill，规范化成 Arbor SkillPackage，再复制到一个明确目录。

核心模型：

```text
SourceSkill -> SkillPackage -> InstalledSkill
```

v1 机器读写三个 JSON 文件：

| 文件 | 职责 |
|------|------|
| `arbor.skills.json` | 安装意图：安装哪些 skill、来自哪里、安装到哪个目录 |
| `arbor.skills.lock.json` | 解析事实：实际版本、revision、内容哈希、安装结果 |
| `skill.package.json` | 单个 Skill 包的元数据：包名、版本、格式、安装文件 |

README 只写规范和示例。管理器不解析 Markdown 代码块。

v1 不做 `local` / `global` 语义。安装目标只接受一个明确目录：`targetDir`。以后可以把 local/global 做成 target alias，但不要放进核心模型。

v1 只支持 `copy` 安装，不支持 `symlink`。开发期链接以后可以单独做 `link` 命令。

v1 不支持 Skill 之间的依赖。所有要安装的 Skill 都必须显式写在 `arbor.skills.json` 里。

v1 要求版本强制、显式、精确。没有显式安装版本不安装；版本号不匹配不安装；版本范围不安装。

## Skill 的定义

本设计里的 Skill 指 agent skill。

Skill 是一个给 agent 按需加载的工作流包。它把任务说明、执行步骤、参考资料、脚本和输出素材组织成一个目录，让 agent 在合适任务里加载并执行。

Skill 不是：

- 语言包，比如 npm、uv、Maven package。
- MCP server。
- plugin。
- shell command。
- 模型内置能力。

这些东西可以和 Skill 组合，但不是 Skill 本身。

## Arbor v1 SkillPackage

Arbor v1 管理的是规范化后的 `SkillPackage`，不是要求所有外部来源都长成同一个样子。

一个 v1 `SkillPackage` 只强制两个入口文件：

```text
skill-name/
  SKILL.md
  skill.package.json
```

`skill.package.json` 可以来自源目录，也可以由管理器在规范化时生成。生成的元数据只写入缓存或安装结果，不回写上游源目录。

`references/`、`scripts/`、`assets/`、`agents/` 不写进固定结构。它们只有被 `skill.package.json.files` 声明后，才属于这个包。

`skill.package.json` 不支持 `dependencies`。v1 不递归安装，也不自动补装缺失 Skill。复用关系由用户在 `arbor.skills.json` 中显式声明。

## `SKILL.md` meta

Arbor v1 读取的 `SKILL.md` front matter 字段只有两个：

```markdown
---
name: skill-name
description: Explain exactly when this skill should be used.
---

Instructions for the agent.
```

字段含义：

| 字段 | 含义 |
|------|------|
| `name` | agent 识别名和安装后的目录名。必须是 kebab-case |
| `description` | 触发说明。必须说明 agent 什么时候应该加载这个 Skill |
| Markdown 正文 | 触发后加载的完整执行说明 |

这些字段不是 Arbor v1 的 `SKILL.md` meta：

- `version`
- `author`
- `source`
- `license`
- `tags`
- `compatibility`
- `permissions`
- `exports`

这些字段属于包管理、来源解析、分发和安全策略，不属于 agent 的触发入口。

Arbor 新建的 Skill 只写 `name` 和 `description`。外部 `SourceSkill` 如果已有其他 front matter，解析器保留原文，但 Arbor 不把这些字段当作包管理字段。

## `skill.package.json`

`skill.package.json` 是 Arbor v1 包元数据。它只描述包自身，不描述来源，也不描述安装目标。

最小示例：

```json
{
  "schema": "arbor.skill-package/v1",
  "id": "nyml/plain-tech-writing-cn",
  "name": "plain-tech-writing-cn",
  "version": "1.0.0",
  "format": "agent-skill",
  "files": [
    "SKILL.md",
    "skill.package.json"
  ]
}
```

字段含义：

| 字段 | 含义 |
|------|------|
| `schema` | 元数据版本。v1 固定为 `arbor.skill-package/v1` |
| `id` | 稳定包名。建议用 `namespace/name` |
| `name` | 安装目录名。必须和 `SKILL.md` front matter 的 `name` 一致 |
| `version` | Skill 包版本。使用 SemVer |
| `format` | Skill 格式。v1 固定为 `agent-skill` |
| `files` | 安装 payload。列出要复制的文件或 glob |

不要把 Arbor 自有元数据命名为 `package.json`。这个名字会让人误以为 Skill 是 npm package。使用 `skill.package.json`。

不要在 `skill.package.json` 里放 source。source 属于安装清单，也就是 `arbor.skills.json`。同一个 Skill 包可能从 Git、tarball、本地目录或 npm 包获取，source 不应写死在包内。

不要在 `skill.package.json` 里声明具体 host 兼容性。v1 只声明 `format: "agent-skill"`。具体 Codex、Claude 或其他 host 的适配文件可以放在 `agents/` 下，并通过 `files` 纳入包。

不要在 `skill.package.json` 里声明 `dependencies`。如果某个 Skill 需要另一个 Skill，使用者必须把两个 Skill 都写进 `arbor.skills.json`。管理器只校验显式清单，不推断依赖图。

## 核心对象

| 对象 | 含义 |
|------|------|
| `SourceSpec` | 从哪里拿 Skill，比如本地路径、Git 子目录、tarball、npm 包 |
| `SourceSkill` | 来源里的原生 skill 目录，必须有 `SKILL.md`，可以是 Codex、Claude 或 OMX 的原生结构 |
| `SkillPackage` | Arbor 解析后的 v1 包，必须有 `SKILL.md` 和 `skill.package.json`；`skill.package.json` 可以是源内文件，也可以是生成结果 |
| `InstalledSkill` | 安装到 `targetDir/<name>/` 的目录 |
| `SkillLockEntry` | 实际安装事实，包括 resolved version、package metadata 来源、revision、content hash、target path |

外部来源不需要先改成 Arbor v1。解析器负责把 `SourceSkill` 转成 `SkillPackage`：

- 来源已有 `skill.package.json` 时，读取并校验，`packageMetadataSource` 记为 `source`。
- 来源只有 `SKILL.md` 时，根据 `arbor.skills.json` 生成 `skill.package.json`，`packageMetadataSource` 记为 `generated`。

生成规则：

1. `id` 来自 `arbor.skills.json.skills[].id`。
2. `version` 来自 `arbor.skills.json.skills[].version`。
3. `name` 来自 `SKILL.md` front matter。
4. `files` 由 SourceSkill 目录下的普通文件生成，至少包含 `SKILL.md` 和生成后的 `skill.package.json`。
5. 生成结果只能写入缓存或安装结果，不能回写 SourceSkill。

## `arbor.skills.json`

`arbor.skills.json` 是安装意图文件。v1 只支持一个 `targetDir`，所有 Skill 都安装到这个目录下。

示例：

```json
{
  "schema": "arbor.skills/v1",
  "targetDir": ".codex/skills",
  "skills": [
    {
      "id": "nyml/plain-tech-writing-cn",
      "version": "1.0.0",
      "source": {
        "type": "git",
        "repo": "https://github.com/nyml/work-context.git",
        "path": "skills/plain-tech-writing-cn",
        "ref": "v1.0.0"
      }
    },
    {
      "id": "local/frontend-engineer",
      "version": "0.0.0-local",
      "source": {
        "type": "path",
        "path": "C:/Users/nyml/code/work-context/skills/frontend-project-driver/references/execution/frontend-engineer"
      }
    }
  ]
}
```

字段含义：

| 字段 | 含义 |
|------|------|
| `schema` | 安装清单版本。v1 固定为 `arbor.skills/v1` |
| `targetDir` | 安装目标目录。最终目录是 `targetDir/<name>/` |
| `skills[].id` | 稳定包名。建议用 `namespace/name` |
| `skills[].version` | 显式安装版本。必填。必须是精确 SemVer，例如 `1.2.0`、`1.2.0-beta.1`、`0.0.0-local`、`0.0.0-vendor.20260621` |
| `skills[].source` | 来源描述 |

`arbor.skills.json` 不写 `name`。安装目录名来自 `SKILL.md` 和 `skill.package.json.name`，避免三处重复。

`skills[].version` 不支持 range，不支持 `latest`，不支持空值。下面这些值都必须拒绝：

```text
^1.2.0
~1.2.0
>=1 <2
latest
```

## 来源类型

v1 只支持四种来源：`path`、`git`、`tarball`、`npm`。

### 本地路径

```json
{
  "source": {
    "type": "path",
    "path": "../skills/my-skill"
  }
}
```

本地路径适合开发和测试。v1 仍然使用 `copy` 安装，不做 symlink。

### Git 子目录

```json
{
  "source": {
    "type": "git",
    "repo": "https://github.com/org/repo.git",
    "path": "skills/my-skill",
    "ref": "v1.2.0"
  }
}
```

解析规则：

- `ref` 可以是 tag、branch 或 commit。
- lock 时必须解析成 commit SHA。
- 如果 `ref` 是 branch，lock 仍然固定到当时的 commit。
- `path` 指向 `SourceSkill` 目录。解析器先把它规范化成 `SkillPackage`，再安装。
- 如果 SourceSkill 没有 `skill.package.json`，推荐使用 tag 或 commit，不推荐使用会漂移的 branch。

URI 简写可以以后再做：

```text
github:org/repo#v1.2.0&path:/skills/my-skill
```

v1 文档先使用展开后的 JSON，不把 URI 简写作为主格式。

### Tarball

```json
{
  "source": {
    "type": "tarball",
    "url": "https://example.com/skills/my-skill-1.2.0.tgz",
    "path": "package/skills/my-skill",
    "integrity": "sha256-..."
  }
}
```

压缩包必须有 `integrity`。没有哈希时只允许人工检查，不进入自动安装。

### npm 包

```json
{
  "source": {
    "type": "npm",
    "package": "@org/codex-skills",
    "version": "1.2.0",
    "path": "skills/my-skill"
  }
}
```

npm 包只作为分发容器。Skill 管理器从包里取出 `path` 指向的 `SourceSkill`，不会把 Skill 当成 npm package 安装到 `node_modules`。

## 未来来源

这些来源不进入 v1 自动安装：

| 来源 | 原因 |
|------|------|
| Python / uv package | 管的是 Python package，不是 Skill |
| Maven artifact | 管的是 JVM artifact，不是 Skill |
| command installer | 会执行任意代码，需要显式信任、审计和沙箱策略 |

## 版本规则

Skill 管理器需要区分四个版本概念：

| 概念 | 例子 | 用途 |
|------|------|------|
| intent version | `arbor.skills.json` 里的 `1.0.0` | 用户显式声明要安装的版本 |
| source package version | 源内 `skill.package.json` 里的 `1.0.0` | 包作者声明的版本 |
| generated package version | 生成的 `skill.package.json` 里的 `0.0.0-vendor.20260621` | 安装方给非受管 Skill 指定的包装版本 |
| locked revision | commit SHA、tarball hash、npm integrity | 实际安装了什么 |

规则：

1. `arbor.skills.json.skills[].version` 必填。
2. `version` 必须是精确 SemVer。允许 `1.2.0`、`1.2.0-beta.1`、`0.0.0-local`、`0.0.0-vendor.20260621`。
3. 禁止 version range、`latest` 和空版本。
4. 如果源内有 `skill.package.json`，它的 `version` 必须等于 intent version。
5. 如果源内没有 `skill.package.json`，管理器用 intent version 生成 package version。
6. 本地开发可以用 `0.0.0-local`，但它仍然是显式版本。
7. 第三方无版本 Skill 可以用包装版本，例如 `0.0.0-vendor.20260621`。
8. Git tag 推荐和版本一致，如 `v1.2.0`。
9. Git branch 只能出现在 `arbor.skills.json`。`arbor.skills.lock.json` 必须写入 commit SHA。
10. 更新时先解析新版本，再校验，再替换安装目录。
11. `arbor.skills.json` 是意图，`arbor.skills.lock.json` 是事实。事实不能覆盖意图。

版本冲突必须阻断安装。例子：

```text
用户声明安装 nyml/review-helper@1.2.0。
管理器拉到的 skill.package.json 声明 version: 1.3.0。
即使内容 hash 可记录，也必须拒绝安装。
```

原因：`arbor.skills.json` 是人的意图，lock 是机器事实。事实只能记录安装结果，不能偷偷改写人的意图。

### 非受管外部 Skill

有些 Skill 不在 Arbor 管理范围内，源目录里只有 `SKILL.md`，没有 `skill.package.json`。这种 Skill 可以安装，但必须由 `arbor.skills.json` 显式指定包装版本。

示例：

```json
{
  "id": "vendor/summarizer",
  "version": "0.0.0-vendor.20260621",
  "source": {
    "type": "git",
    "repo": "https://github.com/acme/agent-skills.git",
    "path": "skills/summarizer",
    "ref": "8f3a2c10123456789abcdef0123456789abcdef0"
  }
}
```

这时 `version` 不是上游作者声明的版本，而是安装方声明的 Arbor 包装版本。管理器读取 `SKILL.md`，生成 `skill.package.json`，再安装到目标目录。

如果 manifest 没写 `version`，拒绝安装。如果源内已有 `skill.package.json.version`，但它和 manifest 不一致，也拒绝安装。

非受管外部 Skill 推荐使用不可变来源，例如 git commit、git tag、tarball integrity 或精确 npm package version。不要用会移动的 git branch 搭配生成版本。

## 典型使用场景

### 1. 团队维护一个受管 Skill 仓库

团队把常用 Skill 放在一个 Git 仓库里。每个 Skill 目录都有 `SKILL.md` 和 `skill.package.json`。发布时打 tag，例如 `v1.2.0`。

```json
{
  "id": "team/review-helper",
  "version": "1.2.0",
  "source": {
    "type": "git",
    "repo": "https://github.com/acme/team-skills.git",
    "path": "skills/review-helper",
    "ref": "v1.2.0"
  }
}
```

管理器拉取 tag，读取源内 `skill.package.json`。如果源内版本也是 `1.2.0`，安装成功。lock 记录 tag 解析出的 commit 和 content hash。

如果源内 `skill.package.json.version` 是 `1.3.0`，安装失败。manifest 说的是人的意图，源内版本不能偷偷覆盖它。

### 2. 安装第三方无版本 Skill

用户想安装一个别人仓库里的 Skill。这个目录只有 `SKILL.md`，没有 `skill.package.json`。这个 Skill 不在 Arbor 管理范围内。

```json
{
  "id": "vendor/summarizer",
  "version": "0.0.0-vendor.20260621",
  "source": {
    "type": "git",
    "repo": "https://github.com/acme/agent-skills.git",
    "path": "skills/summarizer",
    "ref": "8f3a2c10123456789abcdef0123456789abcdef0"
  }
}
```

管理器把 manifest 里的 `id` 和 `version` 当作 Arbor 包装元数据，读取 `SKILL.md` 里的 `name` 和 `description`，生成 `skill.package.json`，再安装。

这个版本号不是上游版本。它的含义是：当前项目把这份外部 Skill 包装成 `vendor/summarizer@0.0.0-vendor.20260621` 使用。

### 3. 本地开发一个新 Skill

用户正在写一个本地 Skill，还没有发布。source 使用 `path`，版本写成 `0.0.0-local`。

```json
{
  "id": "local/frontend-engineer",
  "version": "0.0.0-local",
  "source": {
    "type": "path",
    "path": "C:/Users/nyml/code/work-context/skills/frontend-engineer"
  }
}
```

v1 仍然使用 copy 安装。用户改了本地 `SKILL.md` 或 `scripts/` 后，需要重新执行 install 才能同步到 `targetDir`。

`0.0.0-local` 只适合本机调试。团队共享清单里不要依赖别人机器上的绝对路径。

### 4. 用 tarball 分发经过审计的 Skill

团队想把一组已经审计过的 Skill 作为压缩包分发。tarball 必须带 `integrity`。

```json
{
  "id": "team/security-review",
  "version": "1.0.0",
  "source": {
    "type": "tarball",
    "url": "https://downloads.acme.com/skills/security-review-1.0.0.tgz",
    "path": "package/skills/security-review",
    "integrity": "sha256-..."
  }
}
```

管理器先校验 tarball hash，再读取 `path` 指向的 SourceSkill。没有 `integrity` 的 tarball 只能人工检查，不能进入自动安装。

### 5. 用 npm 包当分发容器

团队已经有 npm registry，希望复用它来分发 Skill 文件。但 Skill 不是 npm runtime package。

```json
{
  "id": "team/codegen-helper",
  "version": "1.2.0",
  "source": {
    "type": "npm",
    "package": "@acme/agent-skills",
    "version": "1.2.0",
    "path": "skills/codegen-helper"
  }
}
```

管理器下载 npm 包，只把它当成文件容器。它不会把 Skill 安装到 `node_modules`，也不会执行 npm lifecycle scripts。

### 6. 删除 Skill 后清理残留

用户从 `arbor.skills.json` 删除了一个 Skill。lock 里可能还留着旧条目，`targetDir` 下也可能还留着旧目录。

运行 `arbor skill prune` 后：

- lock 中不在 manifest 的条目会被删除。
- 空安装目录会被删除。
- lock 记录过、manifest 已删除、且内容 hash 未变化的旧安装目录会被删除。

如果旧目录被用户手动改过，管理器只报告，不自动删除。这样可以避免误删用户临时保存的内容。

## `arbor.skills.lock.json`

`arbor.skills.lock.json` 记录可复现安装事实。
它是生成文件。用户可以删除它让管理器重建，但不应该手写条目作为正常工作流。

示例：

```json
{
  "schema": "arbor.skills-lock/v1",
  "generatedAt": "2026-06-21T00:00:00Z",
  "skills": {
    "nyml/plain-tech-writing-cn": {
      "name": "plain-tech-writing-cn",
      "version": "1.0.0",
      "packageMetadataSource": "source",
      "source": {
        "type": "git",
        "repo": "https://github.com/nyml/work-context.git",
        "path": "skills/plain-tech-writing-cn",
        "ref": "v1.0.0",
        "resolvedCommit": "0123456789abcdef0123456789abcdef01234567"
      },
      "contentHash": "sha256-...",
      "install": {
        "targetDir": ".codex/skills",
        "path": ".codex/skills/plain-tech-writing-cn",
        "mode": "copy"
      }
    }
  }
}
```

非受管外部 Skill 的 lock 条目必须标记生成来源：

```json
{
  "name": "summarizer",
  "version": "0.0.0-vendor.20260621",
  "packageMetadataSource": "generated",
  "source": {
    "type": "git",
    "repo": "https://github.com/acme/agent-skills.git",
    "path": "skills/summarizer",
    "ref": "8f3a2c10123456789abcdef0123456789abcdef0",
    "resolvedCommit": "8f3a2c10123456789abcdef0123456789abcdef0"
  },
  "contentHash": "sha256-..."
}
```

## 安装流程

```text
read arbor.skills.json
  -> resolve source
  -> fetch into cache
  -> locate SourceSkill
  -> normalize into SkillPackage by reading or generating package metadata
  -> validate SkillPackage
  -> copy files to targetDir/<name>/
  -> write arbor.skills.lock.json
```

安装时可以执行 lock cleanup。cleanup 是兜底能力，不是依赖能力。它只清理已经不属于当前显式清单的安装残留。

缓存位置使用平台 cache dir，例如：

```text
<cacheDir>/arbor-skills/
  git/
  tarball/
  npm/
```

不要在规范里硬编码 `~/.cache/arbor-skills/`。

## 校验规则

每个安装目录必须通过这些检查：

1. `SourceSkill` 必须存在 `SKILL.md`。
2. 规范化后的 `SkillPackage` 必须存在 `SKILL.md` 和 `skill.package.json`。
3. `SKILL.md` front matter 必须有 `name` 和 `description`。
4. Arbor 新建的 `SKILL.md` front matter 只能写 `name` 和 `description`。
5. `name` 必须是 kebab-case。
6. 源内或生成的 `skill.package.json` 必须是合法 JSON，且 `schema` 必须是 `arbor.skill-package/v1`。
7. `skill.package.json.name` 必须等于 `SKILL.md` front matter 的 `name`。
8. `skill.package.json.version` 必须等于 `arbor.skills.json.skills[].version`。
9. `skill.package.json.files` 引用的文件必须存在。
10. `skill.package.json` 不能包含 `dependencies`。
11. 安装时不能写出 `targetDir` 之外。
12. `scripts/` 默认只安装，不执行。
13. 生成的 `skill.package.json` 只能写入缓存或安装结果，不能回写 SourceSkill。

## 清理规则

清理只处理 `targetDir` 和 `arbor.skills.lock.json`，不删除 source 目录、Git checkout、npm cache 或 tarball cache。

`prune` 可以自动做这些事：

1. 删除 lock 中已经不在 `arbor.skills.json` 的条目。
2. 删除 `targetDir` 下的空目录。
3. 删除 lock 记录过、manifest 已删除、且当前内容 hash 仍匹配 lock 的旧安装目录。

`prune` 只报告，不自动删除这些目录：

1. 非空但缺少 `SKILL.md` 的目录。
2. 有 `SKILL.md` 但校验失败的目录。
3. lock 记录过但当前内容 hash 已经变化的目录。
4. 不在 lock 中的目录。

这条规则保护用户手动修改过的内容。能证明是 managed install 且未被修改，才自动删；其它情况只报告。

## 安全策略

默认信任等级：

| 来源 | 默认信任 | 原因 |
|------|----------|------|
| local path | 中 | 用户本机内容，但仍要防止路径逃逸 |
| Git tag / commit | 中 | 可复现，但仍要校验内容 |
| Git branch | 低 | branch 会移动 |
| tarball + integrity | 中 | 有哈希可以复现 |
| tarball 无 integrity | 低 | 不能证明内容 |
| npm package | 中 | 有 registry 元数据，但不代表内容安全 |

默认策略：

- 自动安装只允许 `path`、`git commit/tag`、`tarball + integrity`、精确 npm package version。
- `git branch` 可以解析，但安装后 lock 必须固定 commit。
- 非受管 SourceSkill 不推荐使用 `git branch`。如果没有源内 `skill.package.json`，优先使用 commit、tag 或 integrity。
- 更新前先 diff 新旧 `SKILL.md` 和 `scripts/`。
- command installer 不进入 v1。

## 不做什么

v1 不做这些：

- 不做公开 registry。
- 不做复杂依赖解析。
- 不支持 Skill 之间的依赖。
- 不支持 `dependencies` 字段。
- 不自动执行远程安装脚本。
- 不把 skill 当成 Node/Python/Java 运行时包。
- 不要求每个 skill 都有 `package.json`、`pyproject.toml` 或 `pom.xml`。
- 不内建 local/global 语义。
- 不支持 per-skill target。
- 不支持 symlink/link。
- 不把 `.codex/skills`、`.agents/skills`、`.claude/skills` 写死到核心模型里。

## 文件布局建议

```text
arbor.skills.json
arbor.skills.lock.json
packages/
  skill-manager-core/
    README.md
```

Skill 安装目标由 `targetDir` 决定，例如：

```text
.codex/skills/
.agents/skills/
~/.codex/skills/
~/.agents/skills/
~/.claude/skills/
tmp/test-skills/
```

## CLI 路线

后续可以做一个 `@arbor/skill-manager`：

```text
packages/skill-manager-core/
packages/skill-manager-cli/
```

命令：

```text
arbor skill install
arbor skill install --prune-lock
arbor skill update
arbor skill list
arbor skill lint
arbor skill prune
arbor skill remove <id>
```

core 包负责：

- manifest 解析。
- source 解析。
- lockfile 更新。
- 目录校验。
- 安装计划生成。

cli 包负责：

- 参数解析。
- 调用 core。
- 打印 JSON 或人类可读结果。

## 推荐实现顺序

1. 支持 `arbor.skills.json`、`arbor.skills.lock.json` 和本地 path source。
2. 加严格版本校验：缺 manifest version、range version、source package version 不匹配都失败。
3. 生成 content hash 和 lock entry。
4. 支持无 `skill.package.json` 的 SourceSkill，并生成 package metadata。
5. 加目录校验。
6. 加 `prune`，清理 lock 残留和空安装目录。
7. 支持 Git source：`repo + path + ref`。
8. 支持 tarball source。
9. 支持 npm package source。
10. CLI 有显式信任和审计机制后，再考虑 command installer。
11. 最后再考虑 Python/uv 和 Maven。

## 参考资料

- pnpm Supported package sources: https://pnpm.io/package-sources
- pnpm Workspace protocol: https://pnpm.io/workspaces
- npm package.json dependency syntax: https://docs.npmjs.com/cli/v10/configuring-npm/package-json
- uv dependency sources: https://docs.astral.sh/uv/concepts/projects/dependencies/
- Maven POM reference: https://maven.apache.org/pom.html
- OpenAI Codex Agent Skills: https://developers.openai.com/codex/skills
- Agent Skills specification: https://agentskills.io/specification
- Claude Code Skills: https://code.claude.com/docs/en/skills
