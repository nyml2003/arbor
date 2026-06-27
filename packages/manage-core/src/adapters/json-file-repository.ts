import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname } from "node:path";
import type { Task, TaskRepository, TaskStoreDocument } from "../domain/types.js";
import { emptyTaskDocument, taskId } from "../domain/types.js";

export function createJsonFileTaskRepository(filePath: string): TaskRepository {
  return {
    async load() {
      const text = await readFile(filePath, "utf8").catch((error: unknown) => {
        if (isMissingFile(error)) return null;
        throw error;
      });
      if (text === null) return emptyTaskDocument();
      return parseTaskDocument(JSON.parse(text) as unknown);
    },
    async save(document) {
      await mkdir(dirname(filePath), { recursive: true });
      await writeFile(filePath, `${JSON.stringify(document, null, 2)}\n`, "utf8");
    },
  };
}

function parseTaskDocument(value: unknown): TaskStoreDocument {
  if (!isRecord(value) || value["schema"] !== "arbor.manage-tasks/v1") {
    throw new Error("tasks store must use schema arbor.manage-tasks/v1.");
  }

  const tasksValue = value["tasks"];
  if (!Array.isArray(tasksValue)) {
    throw new Error("tasks store must contain a tasks array.");
  }

  return {
    schema: "arbor.manage-tasks/v1",
    tasks: tasksValue.map(parseTask),
  };
}

function parseTask(value: unknown): Task {
  if (!isRecord(value)) {
    throw new Error("task must be an object.");
  }

  const idValue = readString(value, "id");
  const parsedId = taskId(idValue);
  if (!parsedId.ok) {
    throw new Error(parsedId.error.message);
  }

  const status = readString(value, "status");
  if (status !== "todo" && status !== "done") {
    throw new Error(`Unsupported task status: ${status}`);
  }

  const completedAt = value["completedAt"];
  if (completedAt !== null && typeof completedAt !== "string") {
    throw new Error("task completedAt must be a string or null.");
  }

  return {
    id: parsedId.value,
    title: readString(value, "title"),
    status,
    createdAt: readString(value, "createdAt"),
    updatedAt: readString(value, "updatedAt"),
    completedAt,
  };
}

function readString(value: Readonly<Record<string, unknown>>, key: string): string {
  const field = value[key];
  if (typeof field !== "string" || field.trim().length === 0) {
    throw new Error(`task ${key} must be a non-empty string.`);
  }
  return field;
}

function isRecord(value: unknown): value is Readonly<Record<string, unknown>> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isMissingFile(error: unknown): boolean {
  return isRecord(error) && error["code"] === "ENOENT";
}
