# 核心 CLI 架构

## 意图

把 skill manager 维护成一组 TypeScript DDD 风格的 core/cli 包。

## 适用场景

- 修改 `packages/skill-manager-core`。
- 修改 `packages/skill-manager-cli`。
- 新增命令或 source adapter。
- 重构 install、lint、lock 或 prune 行为。

## 必须遵守的规则

- `skill-manager-core` 不依赖 CLI 细节。
- `skill-manager-cli` 只作为 core use case 的薄外壳。
- 领域类型和 validator 放在 `src/domain`。
- use case 编排放在 `src/application`。
- Node 文件系统操作放在 `src/adapters`。
- 从 `src/index.ts` 导出稳定 API。
- 使用 `Readonly` 输入/输出类型和显式可判别联合。
- 除非 source adapter 真实需要，不要新增运行时依赖。

## 推荐模式

- 先把 source type 建成 domain spec，再添加 adapter 实现。
- 校验失败返回 diagnostic。
- use case 无法继续时，在 application 边界 throw `SkillManagerError`。
- 替换目标目录前先暂存，保持 copy/install 原子性。
- lock entry 来自规范化后的 package，不来自原始 source declaration。
- CLI 负责解析 flag 和格式化 report，不负责决定业务规则。

## 反模式

- 直接在 CLI command 里添加远程 fetch 逻辑。
- package 已有 lint 级错误时仍允许 install 继续。
- 把 lockfile 数据当作包元数据事实来源。
- source adapter 还不需要稳定扩展边界时，就添加 plugin system。
- path resolution 逃逸 manifest 或 target 边界。

## 脚手架影响

新增 source adapter 时：

- 明确扩展 `SourceSpec` 和 lock source type。
- runtime fetch 前增加 manifest 校验。
- 把 fetched source 规范化成 `SkillPackage`。
- 暴露 CLI 路径前，先补 lint 和 install 测试。
- 为路径穿越、integrity、auth 或 cache 行为补安全故事。

## 证据

- `src/domain/types.ts` 定义 `SourceSkill`、`SkillPackage`、`InstalledSkillReport`、source spec 和 lock entry。
- `src/application/normalize.ts` 把 path source 转成规范化包元数据。
- `src/application/install.ts` 负责暂存、hash、复制和写入 lock entry。
- `src/application/lint.ts` 运行 manifest diagnostic 和 source normalization。
- `packages/skill-manager-cli/src/cli.ts` 只解析命令，并委托给 core。

## 推断说明

当前实现刻意只在 runtime 支持 path source。远程 source spec 的存在，是为了让校验和未来 adapter 边界更明确。
