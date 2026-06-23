# 来源安全

## 意图

让来源解析可复现，并保证安装 payload 留在目标目录内。

## 适用场景

- 增加 local path 之外的 source 支持。
- 评审 source 元数据。
- 修改复制、暂存、hash、lock 或 prune 行为。
- 处理第三方或非受管 skill。

## 必须遵守的规则

- 只安装 manifest 中的精确版本。
- 拒绝缺失版本、版本范围和 `latest`。
- 受管 source package 的 package version 必须等于 manifest version。
- 非受管 source package 的安装包元数据由 manifest id/version 生成。
- 拒绝符号链接 payload。
- 拒绝绝对路径、空路径、包含 `..` 或逃逸 source root 的 package file path。
- 安装阶段不执行脚本。
- tarball 自动安装前必须有 integrity。
- Git branch ref 属于低信任来源，因为 update 会移动它。

## 推荐模式

- 远程来源优先使用 Git tag、Git commit、tarball integrity 或精确 npm version。
- manifest 中保持显式 `targetDir`。
- 真实安装前先运行 dry-run。
- 对暂存包内容做 hash，并把 content hash 记录到 lock。
- 使用 prune 清理 stale lock entry、空目录和未变化的受管安装目录。

## 反模式

- 从会移动的远程 ref 安装，却不暴露这次移动。
- 比起 manifest 意图，更信任 source package 元数据。
- 不经过暂存和 hash，直接把 source folder 复制到 target。
- 打包或复制时跟随符号链接。
- 仅因为某个已安装 skill 不是 Arbor 管理的，就把它当成坏目录。

## 脚手架影响

新增远程来源支持时：

- 定义 auth、cache、integrity 和 resolved revision 如何出现在 diagnostic 和 lock 中。
- 增加 “manifest 意图 versus resolved package version” 校验。
- 为 archive payload 增加路径穿越测试。
- 在 update 行为自动化前，补 cache invalidation 或 refetch 故事。

## 证据

- `normalize.ts` rejects non-path runtime sources with `unsupported-feature` until adapters exist.
- `normalize.ts` rejects unsafe package file paths and missing package files.
- `install.ts` stages package contents before hashing and copying.
- `core.test.ts` 覆盖 tarball 缺失 integrity 和符号链接 payload 拒绝。
- 设计讨论拒绝了 v1 依赖支持，并要求精确、显式版本。

## 推断说明

Arbor v1 的安全模型刻意保守。早拒绝一个来源，比安装一个模糊或会移动的 artifact 更好。
