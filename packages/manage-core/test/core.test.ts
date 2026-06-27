import assert from "node:assert/strict";
import { mkdir, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import {
  completeTask,
  createJsonFileTaskRepository,
  createMemoryTaskRepository,
  createTask,
  listTasks,
  restoreTask,
  taskId,
  updateTask,
  type TaskId,
} from "../src/index.js";

test("creates, lists, updates, completes, and restores tasks", async () => {
  const repository = createMemoryTaskRepository();
  const id = parseTaskId("task_demo");
  let tick = 0;
  const now = () => `2026-06-23T00:00:0${tick += 1}.000Z`;

  const created = await createTask({
    title: "  Write manage core  ",
    repository,
    generateId: () => id,
    now,
  });
  assert.equal(created.ok, true);
  if (!created.ok) return;
  assert.equal(created.value.title, "Write manage core");
  assert.equal(created.value.status, "todo");

  const renamed = await updateTask({
    id,
    title: "Wire manage CLI",
    repository,
    now,
  });
  assert.equal(renamed.ok, true);
  if (!renamed.ok) return;
  assert.equal(renamed.value.title, "Wire manage CLI");
  assert.notEqual(renamed.value.updatedAt, renamed.value.createdAt);

  const completed = await completeTask({ id, repository, now });
  assert.equal(completed.ok, true);
  if (!completed.ok) return;
  assert.equal(completed.value.status, "done");
  assert.equal(completed.value.completedAt, completed.value.updatedAt);

  const restored = await restoreTask({ id, repository, now });
  assert.equal(restored.ok, true);
  if (!restored.ok) return;
  assert.equal(restored.value.status, "todo");
  assert.equal(restored.value.completedAt, null);

  const listed = await listTasks({ repository });
  assert.equal(listed.ok, true);
  if (!listed.ok) return;
  assert.deepEqual(listed.value.map((task) => task.title), ["Wire manage CLI"]);
});

test("rejects empty titles and missing tasks as structured errors", async () => {
  const repository = createMemoryTaskRepository();
  const missingId = parseTaskId("task_missing");

  const empty = await createTask({
    title: "   ",
    repository,
    generateId: () => parseTaskId("task_empty"),
    now: () => "2026-06-23T00:00:00.000Z",
  });
  assert.equal(empty.ok, false);
  if (empty.ok) return;
  assert.equal(empty.error.code, "invalid-input");

  const missing = await completeTask({
    id: missingId,
    repository,
    now: () => "2026-06-23T00:00:01.000Z",
  });
  assert.equal(missing.ok, false);
  if (missing.ok) return;
  assert.equal(missing.error.code, "not-found");
  assert.equal(missing.error.taskId, missingId);
});

test("json repository persists tasks to a workspace file", async () => {
  const workspace = await createWorkspace();
  try {
    const storePath = join(workspace, "workspace", "manage", "tasks.json");
    const repository = createJsonFileTaskRepository(storePath);
    const id = parseTaskId("task_file");

    const created = await createTask({
      title: "Persist task",
      repository,
      generateId: () => id,
      now: () => "2026-06-23T00:00:00.000Z",
    });
    assert.equal(created.ok, true);

    const raw = JSON.parse(await readFile(storePath, "utf8")) as Readonly<Record<string, unknown>>;
    assert.equal(raw["schema"], "arbor.manage-tasks/v1");

    const reloaded = await listTasks({
      repository: createJsonFileTaskRepository(storePath),
    });
    assert.equal(reloaded.ok, true);
    if (!reloaded.ok) return;
    assert.equal(reloaded.value[0]?.id, id);
    assert.equal(reloaded.value[0]?.title, "Persist task");
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});

function parseTaskId(value: string): TaskId {
  const parsed = taskId(value);
  if (parsed.ok) {
    return parsed.value;
  }
  throw new Error(`Invalid test task id: ${value}`);
}

async function createWorkspace(): Promise<string> {
  const dir = join(tmpdir(), `arbor-manage-core-${crypto.randomUUID()}`);
  await mkdir(dir, { recursive: true });
  return dir;
}
