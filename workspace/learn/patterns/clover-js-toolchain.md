# 模式：JS 工具链框架 monorepo（Clover）

## 一句话

一个自建的 JS/TS 工具链框架，包含 CLI、ESLint 配置、ESLint 插件、协议层、标准库、TypeScript 配置——全部作为 `@clover.js/*` 包的集合，pnpm workspace 管理。

## 核心架构

```
clover-workspace/
├── packages/
│   ├── cli/              ← 命令行入口
│   ├── eslint-config/    ← 共享 ESLint 配置
│   ├── eslint-plugin/    ← 自定义 ESLint 规则
│   ├── protocol/         ← 协议定义层
│   ├── std/              ← 标准库
│   ├── tsconfig/         ← 共享 TypeScript 配置
│   ├── http/             ← HTTP 层
│   └── zod/              ← Zod schema 扩展
```

每个包是独立 npm 包，通过 `workspace:*` 相互引用。

## 关键设计

### 1. 自建 ESLint 生态

```
@clover.js/eslint-config   ← extends 这个就能用
@clover.js/eslint-plugin   ← 自定义规则
```

不是用 `eslint-config-airbnb` 或 `@antfu/eslint-config`——自己定义规则和配置。可以精确控制每个规则的语义，不依赖第三方配置的更新节奏。

### 2. Protocol 层

```
@clover.js/protocol   ← 协议定义（类型 + 校验）
```

放在独立包里——CLI 依赖它，HTTP 依赖它，客户端如果拆出去也依赖它。保证所有通信方用的是同一份类型定义，不会出现"CLI 理解的 Request 和 HTTP 层理解的 Request 不一样"。

### 3. Std 层

```
@clover.js/std   ← 标准库：类型工具、常用函数、共享抽象
```

`@clover.js/std` 是所有包的公共依赖。不依赖任何框架，只依赖 TypeScript 本身。和 Rust 的 `std` 概念一样——平台的基础设施。

### 4. tsconfig 作为独立包

```
@clover.js/tsconfig   ← 共享 tsconfig 配置
```

其他包在自己的 `tsconfig.json` 里：
```json
{
  "extends": "@clover.js/tsconfig/base.json"
}
```

新增一个包不需要手动复制 tsconfig 选项。tsconfig 变更一次，所有包生效。

### 5. pnpm workspace 管理

7+ 个包，pnpm 的 workspace protocol (`workspace:*`) 管理内部依赖。和 Arbor 的 `apps/ + packages/` 结构完全相同的理念。

## 和 ObolosFS/Arbor 的对照

| 工具链层 | Clover | ObolosFS | Arbor |
|---------|--------|----------|-------|
| 构建 | pnpm + tsc | pnpm + nx + swc | electron-vite + tsc |
| Lint | 自建 eslint-plugin | eslint | eslint |
| tsconfig | 独立包 | tsconfig.base.json | tsconfig.base.json |
| Protocol | `@clover.js/protocol` | 内联类型 | `shared/channels.ts` |
| CLI | `@clover.js/cli` | `@obolosfs/ofsh` | 待建 |

## 来源

- Clover 源码（`packages/` 目录结构、`package.json`）
- 2026-06-07 agent 阅读后提炼
