# 013 - 主线闭环回收

日期：2026-06-23

## 做了什么

- 修复根验证入口，让 `pnpm typecheck` 和 `pnpm test` 重新可用。
- 在容器文件查看路径中加入无新依赖的 Markdown 预览。
- 新增 `@arbor/manage-core` 和 `@arbor/manage-cli`，任务数据落到 `workspace/manage/tasks.json`。
- 把 Manage 面板接入容器，并让 Web/Electron E2E 覆盖主路径。

## 学到了什么

- Arbor 已经不再只是 Phase 1 空壳，文档必须跟着主线闭环更新。
- 人读任务文档和机器任务数据要分开：`tasks.md` 记录路线，`tasks.json` 承载应用状态。
- Web 版可以读取构建时静态 workspace，用来验证展示能力；写回仍只放在 Electron。

## 决策

- `arbor-manage` 暂时作为管理域 CLI bin，避免覆盖 skill manager 的 `arbor` bin。
- Markdown v1 不新增依赖，只覆盖标题、段落、列表、代码块和内联代码。

## 下一步

- 做静态站点导出。
- 评估统一 CLI 入口。
- 把仍有效的人工待办迁移成 `tasks.json` 任务。
