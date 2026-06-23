import assert from "node:assert/strict";
import { mkdir, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import {
  installSkills,
  lintManifest,
  pruneSkills,
  SkillManagerError,
} from "../src/index.js";

test("installs unmanaged path skill with generated package metadata", async () => {
  const workspace = await createWorkspace();
  try {
    await writeSkill({
      dir: join(workspace, "source", "summarizer"),
      name: "summarizer",
      packageJson: null,
    });
    await writeManifest(workspace, {
      targetDir: "installed",
      skills: [
        {
          id: "vendor/summarizer",
          version: "0.0.0-vendor.20260621",
          source: { type: "path", path: "source/summarizer" },
        },
      ],
    });

    const diagnostics = await lintManifest({
      manifestPath: "arbor.skills.json",
      cwd: workspace,
    });
    assert.equal(diagnostics.length, 0);

    const report = await installSkills({
      manifestPath: "arbor.skills.json",
      cwd: workspace,
      dryRun: false,
      pruneLock: false,
      nowIso: "2026-06-22T00:00:00.000Z",
    });

    assert.equal(report.installed.length, 1);
    assert.equal(report.installed[0]?.packageMetadataSource, "generated");

    const generatedPackage = JSON.parse(
      await readFile(join(workspace, "installed", "summarizer", "skill.package.json"), "utf8"),
    ) as Readonly<Record<string, unknown>>;
    assert.equal(generatedPackage["id"], "vendor/summarizer");
    assert.equal(generatedPackage["version"], "0.0.0-vendor.20260621");

    const sourcePackageExists = await readFile(join(workspace, "source", "summarizer", "skill.package.json"), "utf8")
      .then(() => true)
      .catch(() => false);
    assert.equal(sourcePackageExists, false);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});

test("installs managed path skill with source package metadata", async () => {
  const workspace = await createWorkspace();
  try {
    await writeSkill({
      dir: join(workspace, "source", "review-helper"),
      name: "review-helper",
      packageJson: {
        schema: "arbor.skill-package/v1",
        id: "team/review-helper",
        name: "review-helper",
        version: "1.2.0",
        format: "agent-skill",
        files: ["SKILL.md", "skill.package.json"],
      },
    });
    await writeManifest(workspace, {
      targetDir: "installed",
      skills: [
        {
          id: "team/review-helper",
          version: "1.2.0",
          source: { type: "path", path: "source/review-helper" },
        },
      ],
    });

    const report = await installSkills({
      manifestPath: "arbor.skills.json",
      cwd: workspace,
      dryRun: false,
      pruneLock: false,
      nowIso: "2026-06-22T00:00:00.000Z",
    });

    assert.equal(report.installed.length, 1);
    assert.equal(report.installed[0]?.packageMetadataSource, "source");
    assert.equal(await exists(join(workspace, "installed", "review-helper", "skill.package.json")), true);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});

test("rejects source package version mismatch", async () => {
  const workspace = await createWorkspace();
  try {
    await writeSkill({
      dir: join(workspace, "source", "review-helper"),
      name: "review-helper",
      packageJson: {
        schema: "arbor.skill-package/v1",
        id: "team/review-helper",
        name: "review-helper",
        version: "1.3.0",
        format: "agent-skill",
        files: ["SKILL.md", "skill.package.json"],
      },
    });
    await writeManifest(workspace, {
      targetDir: "installed",
      skills: [
        {
          id: "team/review-helper",
          version: "1.2.0",
          source: { type: "path", path: "source/review-helper" },
        },
      ],
    });

    await assert.rejects(
      installSkills({
        manifestPath: "arbor.skills.json",
        cwd: workspace,
        dryRun: false,
        pruneLock: false,
        nowIso: "2026-06-22T00:00:00.000Z",
      }),
      (error) => error instanceof SkillManagerError
        && error.diagnostics.some((diagnostic) => diagnostic.code === "version-mismatch"),
    );
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});

test("rejects range and latest manifest versions", async () => {
  const workspace = await createWorkspace();
  try {
    await writeManifest(workspace, {
      targetDir: "installed",
      skills: [
        {
          id: "team/range-skill",
          version: "^1.2.0",
          source: { type: "path", path: "source/range-skill" },
        },
        {
          id: "team/latest-skill",
          version: "latest",
          source: { type: "path", path: "source/latest-skill" },
        },
      ],
    });

    const diagnostics = await lintManifest({
      manifestPath: "arbor.skills.json",
      cwd: workspace,
    });

    assert.equal(diagnostics.filter((diagnostic) => diagnostic.code === "invalid-version").length, 2);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});

test("dry-run does not write installed directory or lock", async () => {
  const workspace = await createWorkspace();
  try {
    await writeSkill({
      dir: join(workspace, "source", "local-skill"),
      name: "local-skill",
      packageJson: null,
    });
    await writeManifest(workspace, {
      targetDir: "installed",
      skills: [
        {
          id: "local/local-skill",
          version: "0.0.0-local",
          source: { type: "path", path: "source/local-skill" },
        },
      ],
    });

    const report = await installSkills({
      manifestPath: "arbor.skills.json",
      cwd: workspace,
      dryRun: true,
      pruneLock: false,
      nowIso: "2026-06-22T00:00:00.000Z",
    });

    assert.equal(report.installed[0]?.action, "planned");
    assert.equal(await exists(join(workspace, "installed", "local-skill")), false);
    assert.equal(await exists(join(workspace, "arbor.skills.lock.json")), false);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});

test("rejects tarball source without integrity", async () => {
  const workspace = await createWorkspace();
  try {
    await writeManifest(workspace, {
      targetDir: "installed",
      skills: [
        {
          id: "team/security-review",
          version: "1.0.0",
          source: {
            type: "tarball",
            url: "https://downloads.acme.com/skills/security-review-1.0.0.tgz",
            path: "package/skills/security-review",
          },
        },
      ],
    });

    const diagnostics = await lintManifest({
      manifestPath: "arbor.skills.json",
      cwd: workspace,
    });

    assert.equal(diagnostics.some((diagnostic) => diagnostic.message.includes("tarball source requires integrity")), true);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});

test("remote source stories fail explicitly until runtime adapters exist", async () => {
  const workspace = await createWorkspace();
  try {
    await writeManifest(workspace, {
      targetDir: "installed",
      skills: [
        {
          id: "team/git-skill",
          version: "1.0.0",
          source: {
            type: "git",
            repo: "https://github.com/acme/team-skills.git",
            path: "skills/git-skill",
            ref: "v1.0.0",
          },
        },
        {
          id: "team/npm-skill",
          version: "1.0.0",
          source: {
            type: "npm",
            package: "@acme/agent-skills",
            version: "1.0.0",
            path: "skills/npm-skill",
          },
        },
      ],
    });

    const diagnostics = await lintManifest({
      manifestPath: "arbor.skills.json",
      cwd: workspace,
    });

    assert.equal(diagnostics.filter((diagnostic) => diagnostic.code === "unsupported-feature").length, 2);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});

test("rejects symlink payloads", async () => {
  const workspace = await createWorkspace();
  try {
    const skillDir = join(workspace, "source", "unsafe-skill");
    await writeSkill({ dir: skillDir, name: "unsafe-skill", packageJson: null });
    await symlink(join(workspace, "outside.txt"), join(skillDir, "outside-link.txt"));
    await writeManifest(workspace, {
      targetDir: "installed",
      skills: [
        {
          id: "local/unsafe-skill",
          version: "0.0.0-local",
          source: { type: "path", path: "source/unsafe-skill" },
        },
      ],
    });

    const diagnostics = await lintManifest({ manifestPath: "arbor.skills.json", cwd: workspace });
    assert.equal(diagnostics.some((diagnostic) => diagnostic.code === "unsafe-symlink"), true);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});

test("prune removes stale lock and unchanged managed directory", async () => {
  const workspace = await createWorkspace();
  try {
    await writeSkill({
      dir: join(workspace, "source", "old-skill"),
      name: "old-skill",
      packageJson: null,
    });
    await writeManifest(workspace, {
      targetDir: "installed",
      skills: [
        {
          id: "local/old-skill",
          version: "0.0.0-local",
          source: { type: "path", path: "source/old-skill" },
        },
      ],
    });
    await installSkills({
      manifestPath: "arbor.skills.json",
      cwd: workspace,
      dryRun: false,
      pruneLock: false,
      nowIso: "2026-06-22T00:00:00.000Z",
    });
    await writeManifest(workspace, {
      targetDir: "installed",
      skills: [],
    });

    const report = await pruneSkills({
      manifestPath: "arbor.skills.json",
      cwd: workspace,
      dryRun: false,
      nowIso: "2026-06-22T00:01:00.000Z",
    });

    assert.equal(report.actions.some((action) => action.type === "remove-lock-entry"), true);
    assert.equal(report.actions.some((action) => action.type === "remove-managed-dir"), true);
    assert.equal(await exists(join(workspace, "installed", "old-skill")), false);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});

async function createWorkspace(): Promise<string> {
  const dir = join(tmpdir(), `arbor-skill-test-${crypto.randomUUID()}`);
  await mkdir(dir, { recursive: true });
  return dir;
}

async function writeSkill(input: Readonly<{
  dir: string;
  name: string;
  packageJson: Readonly<Record<string, unknown>> | null;
}>): Promise<void> {
  await mkdir(input.dir, { recursive: true });
  await writeFile(
    join(input.dir, "SKILL.md"),
    `---\nname: ${input.name}\ndescription: Use this skill in tests.\n---\n\nInstructions.\n`,
    "utf8",
  );

  if (input.packageJson !== null) {
    await writeFile(join(input.dir, "skill.package.json"), `${JSON.stringify(input.packageJson, null, 2)}\n`, "utf8");
  }
}

async function writeManifest(workspace: string, manifest: Readonly<{
  targetDir: string;
  skills: ReadonlyArray<Readonly<Record<string, unknown>>>;
}>): Promise<void> {
  await writeFile(
    join(workspace, "arbor.skills.json"),
    `${JSON.stringify({ schema: "arbor.skills/v1", ...manifest }, null, 2)}\n`,
    "utf8",
  );
}

async function exists(path: string): Promise<boolean> {
  return await readFile(path)
    .then(() => true)
    .catch(() => false);
}
