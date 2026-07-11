import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdir, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { runCli } from "../src/cli.js";

test("--version prints package version", async () => {
  const result = await runCli(["--version"]);
  assert.equal(result.exitCode, 0);
  assert.match(result.stdout, /^0\.1\.0\n$/);
});

test("doctor reports runtime and manifest status", async () => {
  const workspace = await createWorkspace();
  try {
    await writeFile(
      join(workspace, "arbor.skills.json"),
      JSON.stringify({ schema: "arbor.skills/v1", targetDir: "installed", skills: [] }),
      "utf8",
    );

    const result = await runCli(["doctor", "--cwd", workspace]);
    assert.equal(result.exitCode, 0);
    assert.match(result.stdout, /Arbor Skill Manager 0\.1\.0/);
    assert.match(result.stdout, /manifest found:/);
    assert.match(result.stdout, /doctor ok/);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});

test("doctor --json returns machine-readable report", async () => {
  const workspace = await createWorkspace();
  try {
    const result = await runCli(["doctor", "--cwd", workspace, "--json"]);
    assert.equal(result.exitCode, 0);
    const report = JSON.parse(result.stdout) as Readonly<Record<string, unknown>>;
    assert.equal(report["version"], "0.1.0");
    assert.equal(report["manifestFound"], false);
    assert.equal(Array.isArray(report["diagnostics"]), true);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});

test("lint --json returns diagnostics payload", async () => {
  const workspace = await createWorkspace();
  try {
    await writeFile(
      join(workspace, "arbor.skills.json"),
      JSON.stringify({ schema: "arbor.skills/v1", targetDir: "installed", skills: [] }),
      "utf8",
    );

    const result = await runCli(["skill", "lint", "--cwd", workspace, "--json"]);
    assert.equal(result.exitCode, 0);
    assert.match(result.stdout, /"command": "lint"/);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});

test("install dry-run uses skill command shape", async () => {
  const workspace = await createWorkspace();
  try {
    await mkdir(join(workspace, "source", "demo-skill"), { recursive: true });
    await writeFile(
      join(workspace, "source", "demo-skill", "SKILL.md"),
      "---\nname: demo-skill\ndescription: Use in CLI tests.\n---\n\nBody.\n",
      "utf8",
    );
    await writeFile(
      join(workspace, "arbor.skills.json"),
      JSON.stringify({
        schema: "arbor.skills/v1",
        targetDir: "installed",
        skills: [
          {
            id: "local/demo-skill",
            version: "0.0.0-local",
            source: { type: "path", path: "source/demo-skill" },
          },
        ],
      }),
      "utf8",
    );

    const result = await runCli(["skill", "install", "--cwd", workspace, "--dry-run"]);
    assert.equal(result.exitCode, 0);
    assert.match(result.stdout, /planned: local\/demo-skill@0\.0\.0-local/);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});

test("symlinked executable entry runs the CLI", async () => {
  const workspace = await createWorkspace();
  const packageDir = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
  const linkedPackageDir = join(workspace, "linked-cli");
  const cliPath = join(linkedPackageDir, "dist-test", "src", "cli.js");

  try {
    await symlink(packageDir, linkedPackageDir, process.platform === "win32" ? "junction" : "dir");
    await writeFile(
      join(workspace, "arbor.skills.json"),
      JSON.stringify({ schema: "arbor.skills/v1", targetDir: "installed", skills: [] }),
      "utf8",
    );

    const version = spawnCli(cliPath, ["--version"]);
    assert.ifError(version.error);
    assert.equal(version.status, 0, version.stderr);
    assert.equal(version.stdout, "0.1.0\n");

    const lint = spawnCli(cliPath, ["skill", "lint", "--cwd", workspace]);
    assert.ifError(lint.error);
    assert.equal(lint.status, 0, lint.stderr);
    assert.equal(lint.stdout, "No skill issues found.\n");
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});

function spawnCli(cliPath: string, args: ReadonlyArray<string>) {
  return spawnSync(process.execPath, [cliPath, ...args], {
    encoding: "utf8",
  });
}

async function createWorkspace(): Promise<string> {
  const dir = join(tmpdir(), `arbor-skill-cli-test-${crypto.randomUUID()}`);
  await mkdir(dir, { recursive: true });
  return dir;
}
