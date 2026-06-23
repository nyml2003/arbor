import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import {
  access,
  copyFile,
  cp,
  lstat,
  mkdir,
  mkdtemp,
  readdir,
  readFile,
  realpath,
  rm,
  rename,
  stat,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, isAbsolute, join, relative, resolve, sep } from "node:path";
import { finished } from "node:stream/promises";
import type { ContentHash, SkillPackageManifest } from "../domain/types.js";

export async function readJsonFile(filePath: string): Promise<unknown> {
  return JSON.parse(await readTextFile(filePath));
}

export async function readTextFile(filePath: string): Promise<string> {
  return await readFile(filePath, "utf8");
}

export async function writeJsonFile(filePath: string, value: unknown): Promise<void> {
  await mkdir(dirname(filePath), { recursive: true });
  await writeFile(filePath, `${JSON.stringify(value, null, 2)}\n`, "utf8");
}

export async function pathExists(path: string): Promise<boolean> {
  try {
    await access(path);
    return true;
  } catch {
    return false;
  }
}

export function resolveFrom(baseDir: string, path: string): string {
  return isAbsolute(path) ? resolve(path) : resolve(baseDir, path);
}

export async function realpathIfExists(path: string): Promise<string | null> {
  if (!(await pathExists(path))) {
    return null;
  }

  return await realpath(path);
}

export async function ensureInsideDirectory(input: Readonly<{
  parentDir: string;
  childPath: string;
}>): Promise<boolean> {
  const parentRealpath = await ensureRealDirectory(input.parentDir);
  const childParent = await nearestExistingParent(input.childPath);
  const childParentRealpath = await realpath(childParent);
  const childRelative = relative(parentRealpath, resolve(childParentRealpath, relative(childParent, input.childPath)));

  return childRelative === "" || (!childRelative.startsWith("..") && !isAbsolute(childRelative));
}

export async function ensureRealDirectory(path: string): Promise<string> {
  await mkdir(path, { recursive: true });
  return await realpath(path);
}

export async function listRegularFiles(input: Readonly<{ rootDir: string }>): Promise<ReadonlyArray<string>> {
  const files: string[] = [];
  await walkRegularFiles(input.rootDir, input.rootDir, files);
  return files.sort((left, right) => left.localeCompare(right));
}

export async function assertNoSymlink(input: Readonly<{ rootDir: string }>): Promise<string | null> {
  return await findSymlink(input.rootDir, input.rootDir);
}

export async function stageSkillPackage<T>(input: Readonly<{
  sourceDir: string;
  packageManifest: SkillPackageManifest;
  generatedPackageManifest: SkillPackageManifest | null;
  useStagedDir: (stagedDir: string) => Promise<T>;
}>): Promise<T> {
  const tempRoot = await mkdtemp(join(tmpdir(), "arbor-skill-stage-"));
  const stagedDir = join(tempRoot, basename(input.sourceDir));

  try {
    await mkdir(stagedDir, { recursive: true });

    for (const file of input.packageManifest.files) {
      if (file === "skill.package.json" && input.generatedPackageManifest !== null) {
        await writeJsonFile(join(stagedDir, "skill.package.json"), input.generatedPackageManifest);
        continue;
      }

      await copyPackageEntry({
        sourceDir: input.sourceDir,
        stagedDir,
        entryPath: file,
      });
    }

    return await input.useStagedDir(stagedDir);
  } finally {
    await rm(tempRoot, { recursive: true, force: true });
  }
}

export async function copyDirectoryAtomic(input: Readonly<{
  sourceDir: string;
  targetDir: string;
  targetPath: string;
}>): Promise<void> {
  await mkdir(input.targetDir, { recursive: true });
  const tempRoot = await mkdtemp(join(tmpdir(), "arbor-skill-install-"));
  const tempTarget = join(tempRoot, basename(input.targetPath));

  try {
    await cp(input.sourceDir, tempTarget, {
      recursive: true,
      dereference: false,
      force: true,
      errorOnExist: false,
    });

    await rm(input.targetPath, { recursive: true, force: true });
    await rename(tempTarget, input.targetPath);
  } finally {
    await rm(tempRoot, { recursive: true, force: true });
  }
}

async function copyPackageEntry(input: Readonly<{
  sourceDir: string;
  stagedDir: string;
  entryPath: string;
}>): Promise<void> {
  const sourcePath = join(input.sourceDir, input.entryPath);
  const targetPath = join(input.stagedDir, input.entryPath);
  const sourceStat = await lstat(sourcePath);

  await mkdir(dirname(targetPath), { recursive: true });

  if (sourceStat.isDirectory()) {
    await cp(sourcePath, targetPath, {
      recursive: true,
      dereference: false,
      force: true,
      errorOnExist: false,
    });
    return;
  }

  await copyFile(sourcePath, targetPath);
}

export async function removePath(path: string): Promise<void> {
  await rm(path, { recursive: true, force: true });
}

export async function isEmptyDirectory(path: string): Promise<boolean> {
  const pathStat = await stat(path).catch(() => null);
  if (pathStat === null || !pathStat.isDirectory()) {
    return false;
  }

  const entries = await readdir(path);
  return entries.length === 0;
}

export async function listChildDirectories(path: string): Promise<ReadonlyArray<string>> {
  const entries = await readdir(path, { withFileTypes: true }).catch(() => []);
  return entries
    .filter((entry) => entry.isDirectory())
    .map((entry) => join(path, entry.name))
    .sort((left, right) => left.localeCompare(right));
}

export async function hashDirectory(rootDir: string): Promise<ContentHash> {
  const hash = createHash("sha256");
  const files = await listRegularFiles({ rootDir });

  for (const file of files) {
    const fullPath = join(rootDir, file);
    hash.update(file.replaceAll(sep, "/"));
    hash.update("\0");
    await hashFileInto(fullPath, hash);
    hash.update("\0");
  }

  return `sha256-${hash.digest("hex")}` as ContentHash;
}

async function hashFileInto(filePath: string, hash: ReturnType<typeof createHash>): Promise<void> {
  const stream = createReadStream(filePath);
  stream.on("data", (chunk) => {
    hash.update(chunk);
  });
  await finished(stream);
}

async function walkRegularFiles(rootDir: string, currentDir: string, files: string[]): Promise<void> {
  const entries = await readdir(currentDir, { withFileTypes: true });

  for (const entry of entries) {
    const fullPath = join(currentDir, entry.name);
    const relativePath = relative(rootDir, fullPath);

    if (entry.isSymbolicLink()) {
      continue;
    }

    if (entry.isDirectory()) {
      await walkRegularFiles(rootDir, fullPath, files);
      continue;
    }

    if (entry.isFile()) {
      files.push(relativePath);
    }
  }
}

async function findSymlink(rootDir: string, currentDir: string): Promise<string | null> {
  const entries = await readdir(currentDir, { withFileTypes: true });

  for (const entry of entries) {
    const fullPath = join(currentDir, entry.name);

    if (entry.isSymbolicLink()) {
      return relative(rootDir, fullPath);
    }

    if (entry.isDirectory()) {
      const nested = await findSymlink(rootDir, fullPath);
      if (nested !== null) {
        return nested;
      }
    }
  }

  return null;
}

async function nearestExistingParent(path: string): Promise<string> {
  let current = resolve(path);

  while (!(await pathExists(current))) {
    const parent = dirname(current);
    if (parent === current) {
      return current;
    }
    current = parent;
  }

  const currentStat = await lstat(current);
  return currentStat.isDirectory() ? current : dirname(current);
}
