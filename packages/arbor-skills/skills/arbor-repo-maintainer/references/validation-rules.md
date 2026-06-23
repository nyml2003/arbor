# 验证规则

## 意图

验证这次改动影响的行为，但不要把每个任务都变成全仓构建。

## 适用场景

- 完成任何代码或 skill 改动。
- 修改包脚本、清单、锁文件或安装行为。
- 决定收尾时应该报告哪些证据。

## 必须遵守的规则

- 先运行范围最窄且相关的验证。
- 改了哪个包，就运行那个包的测试。
- 改了 TypeScript 包，就运行对应类型检查。
- 改了 skill 清单，就运行 lint 和安装预演。
- 根命令因为无关历史错误失败时，要明确说明，并保留聚焦验证证据。
- 清单或 payload 改动后，要刷新锁文件。

## 推荐模式

- `@arbor/skill-manager-core`：运行 `pnpm --filter @arbor/skill-manager-core test`；API 形状变化时再运行类型检查或构建。
- `@arbor/skill-manager-cli`：运行 `pnpm --filter @arbor/skill-manager-cli test` 和构建。
- `@arbor/skills`：运行 `pnpm --filter @arbor/skills skill:lint`、`skill:install:dry-run`；内容变化后再运行真实 `skill:install`。
- 真实安装后检查生成的 lock 输出。
- 优先写故事测试：非受管来源、受管来源、版本不匹配、dry-run、不支持的远程来源、符号链接拒绝、prune。

## 反模式

- 只读了文件就宣称完成。
- 本可以用包级测试更快定位问题，却只跑根命令。
- 改了来源 payload 后忽略锁文件变化。
- 把无关的根级类型检查错误当成当前改动失败的证据。
- 改 skill payload 时跳过 dry-run，直接真实安装。

## 脚手架影响

新增包时：

- 包含源码的包要提供 `build`、`typecheck` 和 `test` 脚本。
- CLI 包要补 smoke test。
- core 行为要补故事级测试。
- 这个包需要被直接维护时，在 README 里写清验证命令。

## 证据

- `packages/skill-manager-core/test/core.test.ts` covers install, generated package metadata, version mismatch, invalid versions, dry-run, unsupported remote sources, symlink rejection, and prune.
- `packages/skill-manager-cli/test/cli.test.ts` 覆盖 JSON lint 输出和 install dry-run 命令形状。
- `packages/arbor-skills/package.json` provides `skill:lint`, `skill:install:dry-run`, and `skill:install`.
- Previous root `pnpm typecheck` can fail on unrelated app errors, so focused package verification is necessary.

## 推断说明

Arbor 重视验证证据，但它也是一个有历史状态的孵化器仓库。默认先做聚焦验证；只有改动跨越包边界时，全仓验证才更有价值。
