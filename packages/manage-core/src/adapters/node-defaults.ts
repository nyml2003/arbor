import { randomUUID } from "node:crypto";
import type { TaskId, TaskIdGenerator } from "../domain/types.js";

export const randomTaskId: TaskIdGenerator = () => `task-${randomUUID()}` as TaskId;
