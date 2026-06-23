# 验证故事

## 意图

用真实安装故事锁住行为，而不是只做 schema 检查。

## 适用场景

- 新增或修改 skill manager 行为。
- 修改 manifest 或包元数据。
- 判断某个故事是否已经测试。
- 报告完成证据。

## 必须遵守的规则

- 测试受管和非受管来源 skill。
- 测试精确版本强制规则。
- 测试 dry-run 行为。
- 不支持的 source type 在实现前必须明确失败。
- 测试符号链接拒绝。
- 测试 stale lock 和受管安装目录的 prune 行为。
- 修改 `packages/arbor-skills` 后，运行 skill 集合的 lint/dry-run/install。

## 推荐模式

- core 测试使用临时 workspace。
- 测试名使用故事语言，例如 “安装非受管 path skill，并生成包元数据”。
- 检查可观察产物，例如生成的 `skill.package.json`、安装路径和 lock 内容。
- workspace 存在无关错误时，优先使用聚焦包命令。

## 反模式

- 安装行为变化时，只测试纯 validator。
- dry-run 写入安装目录或 lockfile。
- 更新 `arbor.skills.json` 后不刷新 `arbor.skills.lock.json`。
- 不支持的 Git/npm/tarball runtime adapter 被静默忽略。

## 脚手架影响

新增行为时：

- 在 `packages/skill-manager-core/test/core.test.ts` 添加 core 测试。
- 命令面变化时，添加 CLI smoke test。
- 从 `@arbor/skills` 调用 CLI 前，先构建 CLI。
- 运行：
  - `pnpm --filter @arbor/skill-manager-core test`
  - `pnpm --filter @arbor/skill-manager-cli test`
  - `pnpm --filter @arbor/skill-manager-cli build`
  - `pnpm --filter @arbor/skills skill:lint`
  - `pnpm --filter @arbor/skills skill:install:dry-run`
  - `pnpm --filter @arbor/skills skill:install`

## 证据

- `packages/skill-manager-core/test/core.test.ts` 包含这些故事测试：非受管安装、受管安装、版本不匹配、非法版本、dry-run、不支持的远程来源、符号链接拒绝和 prune。
- `packages/skill-manager-cli/test/cli.test.ts` 覆盖 JSON lint 输出和 install dry-run 命令形状。
- `packages/arbor-skills/package.json` 提供本地 skill 验证脚本。

## 推断说明

这里最有价值的测试不是宽泛 snapshot，而是编码 package manager 不能静默退化的承诺。
