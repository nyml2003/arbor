# 模式：Vite + TSX 作品集前端工程（sysfolio）

## 一句话

用 Vite + TSX + Vitest + Playwright 搭建一个完整的作品集前端——entities/features/shared/app 四层拆分，E2E + 单元测试双覆盖。

## 核心架构

```
sysfolio-frontend/
├── src/
│   ├── app/              ← 应用壳（路由、布局、全局状态）
│   ├── entities/         ← 领域实体（Project、Article 等）
│   ├── features/         ← 功能模块（按业务域拆分）
│   ├── shared/           ← 共享 UI 组件 + 工具函数
│   ├── site/             ← 站点配置（SEO、路由映射）
│   └── main.tsx          ← 入口
├── tests/                ← Playwright E2E
├── vitest.config.ts      ← 单元测试
├── playwright.config.ts  ← E2E 测试
└── vite.config.ts        ← 构建配置
```

## 关键设计

### 1. 四层拆分

```
app/        ← 路由 + Layout。薄壳，组合 features
entities/   ← 纯类型 + 校验。不依赖 React/SolidJS
features/   ← 独立功能模块。一个 feature = 一个业务域
shared/     ← 可复用 UI + utils。不包含业务逻辑
```

依赖方向：`entities ← features ← app`。entities 不 import React，可以独立测试。

### 2. 双测试层

```
vitest (单元)     ← 测试 entities、shared、features 的逻辑
playwright (E2E)  ← 测试完整用户流程（导航、表单提交、页面渲染）
```

单元测试覆盖纯逻辑，E2E 覆盖交互流程。不需要在 Vitest 里 mock DOM——直接让 Playwright 在真实浏览器里跑。

### 3. Design 文档与实现并存

```
sysfolio/
├── design/               ← 设计文档
│   ├── frontend-style-handoff/
│   └── frontend-style-handoff-layered/
└── frontend/             ← 实现
```

设计文档和代码在同一仓库。改动代码前可以先看设计意图，不需要跳到外部工具（Figma、Notion）。

### 4. Vite 构建

```json
{
  "scripts": {
    "dev": "vite",
    "build": "tsc -b && vite build",
    "preview": "vite preview"
  }
}
```

构建前先 `tsc -b`——类型错误在构建阶段就暴露，而不是等运行时。Vite 的 dev server 提供 HMR，但不跳过类型检查。

### 5. site/ 层：SEO 和路由配置独立

```
site/   ← 站点级别的配置
  ├── routes.ts     ← 路由定义
  └── seo.ts        ← meta 标签、OG 标签
```

不是把路由散落在 `app/` 各处——集中管理。改路由结构不需要翻遍每个组件。

## 和 Untitled 的对比

| 维度 | sysfolio | Untitled |
|------|---------|----------|
| 框架 | Vite + TSX（React/SolidJS） | Astro SSG |
| 类型 | entities/ 手动定义 | Zod content schema |
| 测试 | Vitest + Playwright | 无显式测试 |
| 构建 | SPA (tsc + vite) | SSG (getStaticPaths) |
| 适合 | 单页应用作品集 | 多页面文件树作品集 |

sysfolio 是 SPA 模式，Untitled 是 SSG 模式。Arbor 的 show/ 分支可能两者都用——SPA 做交互式浏览，SSG 做静态导出。

## 来源

- sysfolio 源码（`frontend/src/` 目录结构、`package.json`、`vitest.config.ts`、`playwright.config.ts`）
- 2026-06-07 agent 阅读后提炼
