import { ipcMain } from "electron";
import { join } from "path";
import { z } from "zod";
import {
  completeTask,
  createJsonFileTaskRepository,
  createTask,
  listTasks,
  randomTaskId,
  restoreTask,
  systemClock,
  taskId,
  updateTask,
  type Task,
  type TaskResult,
} from "@arbor/manage-core";
import type { ManageTask, ManageTaskListResult, ManageTaskResult } from "../../shared/manage";
import { IpcChannels } from "../../shared/channels";
import { getWorkspaceRoot } from "./filesystem.ipc";

const CreateTaskSchema = z.object({ title: z.string().min(1) });
const TaskIdSchema = z.object({ id: z.string().min(1) });
const UpdateTaskSchema = z.object({ id: z.string().min(1), title: z.string().min(1) });

export function registerManageHandlers(): void {
  ipcMain.handle(IpcChannels.MANAGE_LIST, async (): Promise<ManageTaskListResult> => {
    return await withRepository(async (repository) => {
      const result = await listTasks({ repository });
      if (!result.ok) return { ok: false, reason: result.error.message };
      return { ok: true, tasks: result.value.map(toManageTask) };
    });
  });

  ipcMain.handle(IpcChannels.MANAGE_CREATE, async (_event, raw): Promise<ManageTaskResult> => {
    const parsed = CreateTaskSchema.safeParse(raw);
    if (!parsed.success) return { ok: false, reason: "Task title is required." };

    return await withRepository(async (repository) => toTaskResult(await createTask({
      title: parsed.data.title,
      repository,
      generateId: randomTaskId,
      now: systemClock,
    })));
  });

  ipcMain.handle(IpcChannels.MANAGE_UPDATE, async (_event, raw): Promise<ManageTaskResult> => {
    const parsed = UpdateTaskSchema.safeParse(raw);
    if (!parsed.success) return { ok: false, reason: "Task id and title are required." };

    const id = taskId(parsed.data.id);
    if (!id.ok) return { ok: false, reason: id.error.message };

    return await withRepository(async (repository) => toTaskResult(await updateTask({
      id: id.value,
      title: parsed.data.title,
      repository,
      now: systemClock,
    })));
  });

  ipcMain.handle(IpcChannels.MANAGE_COMPLETE, async (_event, raw): Promise<ManageTaskResult> => {
    const parsed = TaskIdSchema.safeParse(raw);
    if (!parsed.success) return { ok: false, reason: "Task id is required." };

    const id = taskId(parsed.data.id);
    if (!id.ok) return { ok: false, reason: id.error.message };

    return await withRepository(async (repository) => toTaskResult(await completeTask({
      id: id.value,
      repository,
      now: systemClock,
    })));
  });

  ipcMain.handle(IpcChannels.MANAGE_RESTORE, async (_event, raw): Promise<ManageTaskResult> => {
    const parsed = TaskIdSchema.safeParse(raw);
    if (!parsed.success) return { ok: false, reason: "Task id is required." };

    const id = taskId(parsed.data.id);
    if (!id.ok) return { ok: false, reason: id.error.message };

    return await withRepository(async (repository) => toTaskResult(await restoreTask({
      id: id.value,
      repository,
      now: systemClock,
    })));
  });
}

async function withRepository<T>(
  action: (repository: ReturnType<typeof createJsonFileTaskRepository>) => Promise<T>,
): Promise<T | Readonly<{ ok: false; reason: string }>> {
  const workspaceRoot = getWorkspaceRoot();
  if (workspaceRoot === null) {
    return { ok: false, reason: "No workspace selected." };
  }

  const repository = createJsonFileTaskRepository(join(workspaceRoot, "manage", "tasks.json"));
  return await action(repository);
}

function toTaskResult(result: TaskResult<Task>): ManageTaskResult {
  if (!result.ok) return { ok: false, reason: result.error.message };
  return { ok: true, task: toManageTask(result.value) };
}

function toManageTask(task: Task): ManageTask {
  return {
    id: task.id,
    title: task.title,
    status: task.status,
    createdAt: task.createdAt,
    updatedAt: task.updatedAt,
    completedAt: task.completedAt,
  };
}
