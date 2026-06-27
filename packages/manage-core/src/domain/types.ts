declare const taskIdBrand: unique symbol;

export type TaskId = string & Readonly<{ [taskIdBrand]: "TaskId" }>;
export type TaskStatus = "todo" | "done";

export type Task = Readonly<{
  id: TaskId;
  title: string;
  status: TaskStatus;
  createdAt: string;
  updatedAt: string;
  completedAt: string | null;
}>;

export type TaskStoreDocument = Readonly<{
  schema: "arbor.manage-tasks/v1";
  tasks: ReadonlyArray<Task>;
}>;

export type TaskErrorCode =
  | "invalid-input"
  | "not-found"
  | "conflict"
  | "persistence-failed";

export type TaskError = Readonly<{
  code: TaskErrorCode;
  message: string;
  taskId: TaskId | null;
}>;

export type TaskResult<T> =
  | Readonly<{ ok: true; value: T }>
  | Readonly<{ ok: false; error: TaskError }>;

export type TaskRepository = Readonly<{
  load(): Promise<TaskStoreDocument>;
  save(document: TaskStoreDocument): Promise<void>;
}>;

export type TaskIdGenerator = () => TaskId;
export type Clock = () => string;

export type CreateTaskInput = Readonly<{
  title: string;
  repository: TaskRepository;
  generateId: TaskIdGenerator;
  now: Clock;
}>;

export type ListTasksInput = Readonly<{
  repository: TaskRepository;
}>;

export type UpdateTaskInput = Readonly<{
  id: TaskId;
  title: string;
  repository: TaskRepository;
  now: Clock;
}>;

export type CompleteTaskInput = Readonly<{
  id: TaskId;
  repository: TaskRepository;
  now: Clock;
}>;

export type RestoreTaskInput = Readonly<{
  id: TaskId;
  repository: TaskRepository;
  now: Clock;
}>;

export function taskId(value: string): TaskResult<TaskId> {
  const trimmed = value.trim();
  if (!/^[a-z0-9][a-z0-9_-]{5,63}$/.test(trimmed)) {
    return {
      ok: false,
      error: {
        code: "invalid-input",
        message: "Task id must be 6-64 lowercase letters, digits, underscores, or hyphens.",
        taskId: null,
      },
    };
  }
  return { ok: true, value: trimmed as TaskId };
}

export function ok<T>(value: T): TaskResult<T> {
  return { ok: true, value };
}

export function fail<T>(error: TaskError): TaskResult<T> {
  return { ok: false, error };
}

export function emptyTaskDocument(): TaskStoreDocument {
  return {
    schema: "arbor.manage-tasks/v1",
    tasks: [],
  };
}
