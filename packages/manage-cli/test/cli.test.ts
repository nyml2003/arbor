import assert from "node:assert/strict";
import { mkdir, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { runCli } from "../src/cli.js";

test("--version prints package version", async () => {
  const result = await runCli(["--version"]);
  assert.equal(result.exitCode, 0);
  assert.equal(result.stdout, "0.1.0\n");
});

test("creates, lists, updates, completes, and restores a task", async () => {
  const workspace = await createWorkspace();
  try {
    const create = await runCli([
      "task",
      "create",
      "Ship manage CLI",
      "--cwd",
      workspace,
      "--json",
    ]);
    assert.equal(create.exitCode, 0);
    const created = JSON.parse(create.stdout) as {
      task?: { id?: string };
    };
    const id = created.task?.id;
    assert.equal(typeof id, "string");

    const list = await runCli(["task", "list", "--cwd", workspace]);
    assert.equal(list.exitCode, 0);
    assert.match(list.stdout, /Ship manage CLI/);

    const update = await runCli([
      "task",
      "update",
      id ?? "",
      "Ship manage GUI",
      "--cwd",
      workspace,
    ]);
    assert.equal(update.exitCode, 0);
    assert.match(update.stdout, /Ship manage GUI/);

    const complete = await runCli(["task", "complete", id ?? "", "--cwd", workspace]);
    assert.equal(complete.exitCode, 0);
    assert.match(complete.stdout, /\[done\]/);

    const restore = await runCli(["task", "restore", id ?? "", "--cwd", workspace]);
    assert.equal(restore.exitCode, 0);
    assert.match(restore.stdout, /\[todo\]/);

    const persisted = await readFile(join(workspace, "workspace", "manage", "tasks.json"), "utf8");
    assert.match(persisted, /Ship manage GUI/);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});

test("reports invalid task ids with a nonzero exit code", async () => {
  const workspace = await createWorkspace();
  try {
    const result = await runCli(["task", "complete", "bad", "--cwd", workspace]);
    assert.equal(result.exitCode, 2);
    assert.match(result.stderr, /Task id must be/);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});

async function createWorkspace(): Promise<string> {
  const dir = join(tmpdir(), `arbor-manage-cli-${crypto.randomUUID()}`);
  await mkdir(dir, { recursive: true });
  return dir;
}
