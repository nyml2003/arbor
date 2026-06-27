import {
  completeTaskRecord,
  createTaskRecord,
  renameTask,
  restoreTaskRecord,
} from "../domain/task.js";
import type {
  CompleteTaskInput,
  CreateTaskInput,
  ListTasksInput,
  RestoreTaskInput,
  Task,
  TaskId,
  TaskResult,
  TaskStoreDocument,
  UpdateTaskInput,
} from "../domain/types.js";
import { fail, ok } from "../domain/types.js";

export async function listTasks(input: ListTasksInput): Promise<TaskResult<ReadonlyArray<Task>>> {
  return withRepository(async () => {
    const document = await input.repository.load();
    return ok([...document.tasks].sort(compareTasks));
  });
}

export async function createTask(input: CreateTaskInput): Promise<TaskResult<Task>> {
  return withRepository(async () => {
    const document = await input.repository.load();
    const task = createTaskRecord({
      id: input.generateId(),
      title: input.title,
      nowIso: input.now(),
    });
    if (!task.ok) return task;

    if (document.tasks.some((candidate) => candidate.id === task.value.id)) {
      return fail({
        code: "conflict",
        message: `Task id already exists: ${task.value.id}`,
        taskId: task.value.id,
      });
    }

    const nextDocument = replaceTasks(document, [...document.tasks, task.value]);
    await input.repository.save(nextDocument);
    return ok(task.value);
  });
}

export async function updateTask(input: UpdateTaskInput): Promise<TaskResult<Task>> {
  return updateById({
    id: input.id,
    repository: input.repository,
    update: (task) => renameTask(task, input.title, input.now()),
  });
}

export async function completeTask(input: CompleteTaskInput): Promise<TaskResult<Task>> {
  return updateById({
    id: input.id,
    repository: input.repository,
    update: (task) => completeTaskRecord(task, input.now()),
  });
}

export async function restoreTask(input: RestoreTaskInput): Promise<TaskResult<Task>> {
  return updateById({
    id: input.id,
    repository: input.repository,
    update: (task) => restoreTaskRecord(task, input.now()),
  });
}

async function updateById(input: Readonly<{
  id: TaskId;
  repository: ListTasksInput["repository"];
  update: (task: Task) => TaskResult<Task>;
}>): Promise<TaskResult<Task>> {
  return withRepository(async () => {
    const document = await input.repository.load();
    const index = document.tasks.findIndex((task) => task.id === input.id);
    if (index === -1) {
      return fail({
        code: "not-found",
        message: `Task not found: ${input.id}`,
        taskId: input.id,
      });
    }

    const current = document.tasks[index];
    if (current === undefined) {
      return fail({
        code: "not-found",
        message: `Task not found: ${input.id}`,
        taskId: input.id,
      });
    }

    const next = input.update(current);
    if (!next.ok) return next;

    const tasks = [...document.tasks];
    tasks[index] = next.value;
    await input.repository.save(replaceTasks(document, tasks));
    return ok(next.value);
  });
}

async function withRepository<T>(
  action: () => Promise<TaskResult<T>>,
): Promise<TaskResult<T>> {
  try {
    return await action();
  } catch (error) {
    return fail({
      code: "persistence-failed",
      message: error instanceof Error ? error.message : "Task persistence failed.",
      taskId: null,
    });
  }
}

function replaceTasks(
  document: TaskStoreDocument,
  tasks: ReadonlyArray<Task>,
): TaskStoreDocument {
  return {
    ...document,
    tasks,
  };
}

function compareTasks(left: Task, right: Task): number {
  if (left.status !== right.status) return left.status === "todo" ? -1 : 1;
  return right.updatedAt.localeCompare(left.updatedAt);
}
