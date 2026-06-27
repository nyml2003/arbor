export type ManageTaskStatus = "todo" | "done";

export type ManageTask = Readonly<{
  id: string;
  title: string;
  status: ManageTaskStatus;
  createdAt: string;
  updatedAt: string;
  completedAt: string | null;
}>;

export type ManageTaskResult =
  | Readonly<{ ok: true; task: ManageTask }>
  | Readonly<{ ok: false; reason: string }>;

export type ManageTaskListResult =
  | Readonly<{ ok: true; tasks: ReadonlyArray<ManageTask> }>
  | Readonly<{ ok: false; reason: string }>;

export type ManageApi = Readonly<{
  list(): Promise<ManageTaskListResult>;
  create(title: string): Promise<ManageTaskResult>;
  update(id: string, title: string): Promise<ManageTaskResult>;
  complete(id: string): Promise<ManageTaskResult>;
  restore(id: string): Promise<ManageTaskResult>;
}>;
