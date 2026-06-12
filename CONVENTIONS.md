# Conventions — 协作规范

## 包命名

所有 Arbor 内部包使用 `@arbor/` scope：

- `@arbor/manage-core` — 管理域核心逻辑
- `@arbor/manage-cli` — 管理域 CLI
- `@arbor/knowl-core` — 沉淀域核心逻辑
- `@arbor/knowl-cli` — 沉淀域 CLI
- 后续类推

不使用 `@toolset/` 或其他 scope。

## 目录规范

```
arbor/
├── packages/                # 可发布的 npm 包
│   └── [domain]-[layer]/    # 如 manage-core、knowl-cli
│       ├── src/
│       │   ├── index.ts
│       │   └── ...
│       ├── package.json
│       └── tsconfig.json
├── apps/                    # 不可发布的应用程序
│   └── container/           # Electron + SolidJS 容器
│       ├── src/
│       │   ├── main/        # Electron 主进程
│       │   ├── renderer/    # SolidJS 渲染进程
│       │   └── preload/     # 预加载脚本
│       ├── package.json
│       └── electron-builder.yml
├── pnpm-workspace.yaml
├── tsconfig.base.json
└── package.json             # 根 package.json（workspace scripts）
```

## 包分层规则

每个工具域按三层拆分：

| 层 | 依赖规则 | 包举例 |
|----|---------|--------|
| core | 可以依赖其他 core，不能依赖 CLI/UI | `@arbor/manage-core` |
| cli | 依赖 core，不能依赖 UI | `@arbor/manage-cli` |
| ui | 依赖 core，不能依赖 CLI（UI 在容器内，不是独立包） | 容器内的 SolidJS 组件 |

## 代码风格

- TypeScript strict mode
- ESM only（`"type": "module"`）
- 不使用 default export（全部 named export）
- 函数优先，少用 class（除非需要封装状态）
- 类型和接口用 `type` 而非 `interface`（除非需要 declaration merging）
- 文件名：kebab-case（`task-store.ts`）
- 测试文件：`*.test.ts`，和源文件同目录

## 提交风格

```
[domain] 简短描述

详细说明（可选）
```

- domain 标注影响的域：`[container]`、`[manage]`、`[learn]`、`[show]`、`[build]`、`[framework]`、`[docs]`
- 示例：`[container] 添加文件树基础组件`
- 示例：`[docs] 更新 Phase 1 完成状态`

## 文档更新规则

- 每完成一个 Phase，更新 `PLAN.md` 标记状态
- 每做一次迭代，在 `learn/iteration-log/` 加一条记录
- 每做一个技术决策，追加到 `DECISIONS.md`
- 每当出现新的编码约定，更新 `CONVENTIONS.md`
- `README.md` 保持简洁，只反映当前状态

## 迭代日志格式

每个日志文件命名：`learn/iteration-log/NNN-short-slug.md`

模板：
```markdown
# NNN - 标题

日期：YYYY-MM-DD

## 做了什么

## 学到了什么

## 决策

## 下一步
```

## 包管理

- 包管理器：pnpm 10+
- Node 版本：24+
- 不使用 yarn / npm / bun
- lockfile：`pnpm-lock.yaml`（提交到 git）

## 构建工具链

参考 ObolosFS 方案：

- **SWC** — JS/TS 转译（`swc src --out-dir dist`）
- **tsc** — 类型声明生成（`--emitDeclarationOnly`）
- **nx** — monorepo 任务编排（`nx run-many -t build|test|typecheck`）
- **Vitest** — 测试框架
- **eslint** — 代码风格检查
- 不使用 bun compile 或其他独立二进制打包方案；CLI 以 package.json `bin` 入口分发

## 测试

- 测试框架：Vitest
- core 包必须有测试
- CLI 包可以依赖 smoke test
- UI 组件测试用 Vitest + SolidJS testing utils
