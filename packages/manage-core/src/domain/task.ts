import type { Task, TaskId, TaskResult } from "./types.js";
import { fail, ok } from "./types.js";

export function createTaskRecord(input: Readonly<{
  id: TaskId;
  title: string;
  nowIso: string;
}>): TaskResult<Task> {
  const title = input.title.trim();
  if (title.length === 0) {
    return fail({
      code: "invalid-input",
      message: "Task title cannot be empty.",
      taskId: null,
    });
  }

  return ok({
    id: input.id,
    title,
    status: "todo",
    createdAt: input.nowIso,
    updatedAt: input.nowIso,
    completedAt: null,
  });
}

export function renameTask(task: Task, title: string, nowIso: string): TaskResult<Task> {
  const nextTitle = title.trim();
  if (nextTitle.length === 0) {
    return fail({
      code: "invalid-input",
      message: "Task title cannot be empty.",
      taskId: task.id,
    });
  }

  return ok({
    ...task,
    title: nextTitle,
    updatedAt: nowIso,
  });
}

export function completeTaskRecord(task: Task, nowIso: string): TaskResult<Task> {
  if (task.status === "done") {
    return fail({
      code: "conflict",
      message: "Task is already complete.",
      taskId: task.id,
    });
  }

  return ok({
    ...task,
    status: "done",
    updatedAt: nowIso,
    completedAt: nowIso,
  });
}

export function restoreTaskRecord(task: Task, nowIso: string): TaskResult<Task> {
  if (task.status === "todo") {
    return fail({
      code: "conflict",
      message: "Task is already active.",
      taskId: task.id,
    });
  }

  return ok({
    ...task,
    status: "todo",
    updatedAt: nowIso,
    completedAt: null,
  });
}
