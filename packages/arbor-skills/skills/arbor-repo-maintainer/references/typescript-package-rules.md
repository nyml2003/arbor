# TypeScript 包规则

## 意图

让 Arbor 的 TypeScript 包保持显式、可测试、易修改。

## 适用场景

- 创建或修改 `packages/*` TypeScript 模块。
- 新增 core/cli 包。
- 修改根 pnpm 脚本、包脚本或 workspace 依赖。
- 编写 TypeScript 的 domain/application/adapter 代码。

## 必须遵守的规则

- 内部包依赖使用 pnpm workspace 和 `workspace:*`。
- 只使用 ESM，并设置 `"type": "module"`。
- 使用命名导出。不要新增 default export。
- 类型声明优先使用 `type`，少用 `interface`。
- 领域合同优先使用 `Readonly` 对象类型和可判别联合。
- core 包不能依赖 CLI 或 UI。
- CLI 包只作为 core use case 的薄外壳。
- adapter 留在边界层。不要把文件系统或进程细节混进领域类型。
- 除非问题明确需要，不要新增运行时依赖。

## 推荐模式

- `src/domain/*` 放 value type、validator 和 diagnostic。
- `src/application/*` 放 use case 和编排。
- `src/adapters/*` 放 Node 文件系统/进程集成。
- 从 `src/index.ts` 导出公开 API。
- 正常、可预期失败用 diagnostic 或 result-like 返回值表示。
- 只在 use case 失败边界或不变量被破坏时 throw。
- plain string 会混淆不同概念时，用 branded string 表示领域 id。
- 优先使用精确版本和显式输入校验，不依赖隐式解析。

## 反模式

- CLI 参数解析直接调用文件系统内部逻辑，而不是调用 core application function。
- 把来源规范化、lock 写入、复制/安装行为塞进一个大模块。
- 为单个调用点创建 helper 或抽象，但没有减少真实复杂度。
- 系统要求可复现安装产物时，还使用 SemVer range。
- 为 Node 已覆盖的 JSON 解析、参数解析或简单文件系统任务新增依赖。

## 脚手架影响

新增 TS core/cli 组合时：

- 创建 `packages/<domain>-core`，包含 `src/domain`、`src/application`、`src/adapters` 和 `src/index.ts`。
- core use case 已经存在后，再创建 `packages/<domain>-cli`。
- 在包测试结构附近添加聚焦的 `*.test.ts`。
- 先在包内添加 `build`、`typecheck` 和 `test` 脚本，再接入根脚本。

## 证据

- `packages/skill-manager-core/src/domain/types.ts` uses branded strings, `Readonly`, and discriminated source specs.
- `packages/skill-manager-core/src/application/*` separates manifest, normalize, install, lint, lockfile, and prune use cases.
- `packages/skill-manager-core/src/adapters/node-fs.ts` isolates Node filesystem operations.
- `packages/skill-manager-cli/src/cli.ts` parses args and delegates to core functions.
- Root `package.json` uses pnpm filters for workspace commands.
- `CONVENTIONS.md` defines ESM, named exports, `type`, kebab-case, and core/cli/ui layering.

## 推断说明

当前 skill-manager 包是 Arbor 里最清晰的 TS DDD 风格样例。未来 TS 基础设施包应优先套用这些规则。
