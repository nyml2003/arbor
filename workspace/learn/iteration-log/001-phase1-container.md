# 001 - Phase 1 容器应用搭建

日期：2026-06-07

## 做了什么

1. 从 WatchDesk 迁移 Electron + SolidJS 架构到 Arbor
2. 搭建 pnpm workspace monorepo（apps/container + 预留 packages/）
3. 实现递归文件树组件（展开/折叠、目录优先排序、选中高亮）
4. 实现 IPC 层：列目录、读文件、选择工作区
5. 创建 workspace/ 目录，四个空分支（build/learn/manage/show）
6. 文档全量更新：分支名中→英、决策修订、工具链更新

## 学到了什么

### WatchDesk 架构迁移非常顺畅

Electron + electron-vite + SolidJS 这套组合在迁移中几乎零摩擦。IPC 层（zod schema → createHandler → preload bridge → renderer API）是一个干净的模式，值得复用。CSS 变量体系（tokens.css）换了个 `--arbor-` 前缀就直接能用了。

### pnpm 的 `onlyBuiltDependencies` 是个必要配置

electron 和 esbuild 需要运行 postinstall 脚本来下载/编译原生二进制。pnpm 默认不执行这些脚本，导致 `electron-vite dev` 启动时报 "Electron uninstall"。解决方案是在根 package.json 加 `pnpm.onlyBuiltDependencies: ["electron", "esbuild"]`。这是 pnpm 10 的安全默认——严格但需要知道怎么解。

### 模块间状态同步是 IPC 层的常见坑

工作区根目录在两个模块中各有一个变量：
- `app.ts` 的 `defaultWorkspaceRoot`（启动时解析默认路径）
- `filesystem.ipc.ts` 的 `workspaceRoot`（文件操作时的安全检查锚点）

第一版只设了前者，后者仍为 null → renderer 调 listDirectory 时 `resolveChecked` 报 "No workspace selected"。修法是加 `setWorkspaceRoot()` 导出，在 `app.whenReady()` 中同步设值。

**模式**：IPC 模块内有状态变量时，需要显式提供初始化入口，不能依赖隐式的跨模块副作用。

### SolidJS 的细粒度响应式很适合文件树

- `createResource` 按需加载子目录
- `createSignal` 管理展开/选中状态，整个树只重渲染变化的部分
- `Show` 条件渲染让加载态/空态处理很干净

### 文件系统作为数据源在桌面应用中极其简单

不需要 API server，不需要数据库，不需要状态管理层。`readdir` + `stat` → IPC → renderer，十几行代码就完成了数据管线。这和 DECISIONS.md 决策 #4（文件存储而非数据库）高度一致——先验证了最简路径是可行的。

## 决策

1. **分支名用英文** `build/learn/manage/show` —— 中文作为文件系统目录名在终端/脚本中用起来不方便，用户反馈后立即修正
2. **工具链确认** pnpm + electron-vite + SWC(内建) + tsc —— 没出现需要 bun 的场景
3. **容器独立** —— Phase 1 的容器不依赖任何 `@arbor/*` 内部包。这个独立性让后续包可以自由演进

## 下一步

Phase 2：管理工具。
- `@arbor/manage-core`（Task 实体、状态机、存储接口）
- `@arbor/manage-cli`（npm bin 入口，调 core）
- 容器内 manage 分支的 GUI 面板
