import { basename } from "node:path";
import {
  hashDirectory,
  isEmptyDirectory,
  listChildDirectories,
  pathExists,
  removePath,
} from "../adapters/node-fs.js";
import type {
  ArborSkillsLock,
  PruneAction,
  PruneReport,
  PruneSkillsInput,
  SkillId,
  SkillLockEntry,
} from "../domain/types.js";
import { SkillManagerError } from "./errors.js";
import { loadManifestDiagnostics } from "./manifest.js";
import { readLock, writeLock } from "./lockfile.js";
import { lockPathForManifest, manifestDir, resolveManifestPath, resolveTargetDir } from "./paths.js";

export async function pruneSkills(input: PruneSkillsInput): Promise<PruneReport> {
  const manifestPath = resolveManifestPath(input);
  const loaded = await loadManifestDiagnostics({
    manifestPath,
    cwd: input.cwd,
  });

  if (loaded.manifest === null) {
    throw new SkillManagerError("Failed to prune skills.", loaded.diagnostics);
  }

  const currentManifestDir = manifestDir(manifestPath);
  const targetDir = resolveTargetDir({
    manifestDir: currentManifestDir,
    targetDir: loaded.manifest.targetDir,
  });
  const lock = await readLock({ manifestPath, cwd: input.cwd });
  const manifestIds = new Set(loaded.manifest.skills.map((skill) => skill.id as string));
  const lockEntries = lock === null ? {} : { ...lock.skills };
  const actions: PruneAction[] = [];
  const reports: string[] = [];

  for (const [id, entry] of Object.entries(lockEntries)) {
    if (manifestIds.has(id)) {
      continue;
    }

    actions.push({ type: "remove-lock-entry", id: id as SkillId });

    if (await canRemoveManagedDirectory(entry)) {
      actions.push({
        type: "remove-managed-dir",
        id: id as SkillId,
        path: entry.install.path,
      });
    } else if (await pathExists(entry.install.path)) {
      reports.push(`Skipped changed managed directory: ${entry.install.path}`);
    }

    delete lockEntries[id];
  }

  for (const childDir of await listChildDirectories(targetDir)) {
    if (await isEmptyDirectory(childDir)) {
      actions.push({ type: "remove-empty-dir", path: childDir });
      continue;
    }

    if (!isLockedInstallPath(lock, childDir)) {
      reports.push(`Unmanaged directory remains: ${basename(childDir)}`);
    }
  }

  if (!input.dryRun) {
    for (const action of actions) {
      if (action.type === "remove-empty-dir" || action.type === "remove-managed-dir") {
        await removePath(action.path);
      }
    }

    await writeLock({
      manifestPath,
      nowIso: input.nowIso,
      entries: lockEntries,
    });
  }

  return {
    dryRun: input.dryRun,
    actions,
    reports,
    lockPath: lockPathForManifest(manifestPath),
  };
}

async function canRemoveManagedDirectory(entry: SkillLockEntry): Promise<boolean> {
  if (!(await pathExists(entry.install.path))) {
    return false;
  }

  return (await hashDirectory(entry.install.path)) === entry.contentHash;
}

function isLockedInstallPath(lock: ArborSkillsLock | null, path: string): boolean {
  if (lock === null) {
    return false;
  }

  return Object.values(lock.skills).some((entry) => entry.install.path === path);
}
