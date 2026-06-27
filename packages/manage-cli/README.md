# Arbor Manage CLI

`arbor-manage` is the thin CLI shell for Arbor task management.

## Usage

```powershell
arbor-manage task create "Write next step"
arbor-manage task list
arbor-manage task complete task_xxxxxx
```

The default store is `workspace/manage/tasks.json` under the current working directory.

## Verification

```powershell
pnpm --filter @arbor/manage-cli test
pnpm --filter @arbor/manage-cli typecheck
pnpm --filter @arbor/manage-cli build
```
