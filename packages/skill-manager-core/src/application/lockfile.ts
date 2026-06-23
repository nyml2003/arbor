import { readJsonFile, pathExists, writeJsonFile } from "../adapters/node-fs.js";
import { parseLockJson } from "../domain/lock.js";
import type {
  ArborSkillsLock,
  ReadLockInput,
  SkillLockEntry,
} from "../domain/types.js";
import { SkillManagerError } from "./errors.js";
import { lockPathForManifest, resolveManifestPath } from "./paths.js";

export async function readLock(input: ReadLockInput): Promise<ArborSkillsLock | null> {
  const manifestPath = resolveManifestPath(input);
  const lockPath = lockPathForManifest(manifestPath);

  if (!(await pathExists(lockPath))) {
    return null;
  }

  const parsed = parseLockJson(await readJsonFile(lockPath), lockPath);
  if (!parsed.ok) {
    throw new SkillManagerError("Failed to read arbor.skills.lock.json.", parsed.diagnostics);
  }

  return parsed.value;
}

export async function writeLock(input: Readonly<{
  manifestPath: string;
  nowIso: string;
  entries: Readonly<Record<string, SkillLockEntry>>;
}>): Promise<string> {
  const lockPath = lockPathForManifest(input.manifestPath);
  const lock: ArborSkillsLock = {
    schema: "arbor.skills-lock/v1",
    generatedAt: input.nowIso,
    skills: input.entries,
  };
  await writeJsonFile(lockPath, lock);
  return lockPath;
}
