import { dirname, join, resolve } from "node:path";
import { resolveFrom } from "../adapters/node-fs.js";

export function resolveManifestPath(input: Readonly<{ manifestPath: string; cwd: string }>): string {
  return resolveFrom(input.cwd, input.manifestPath);
}

export function manifestDir(manifestPath: string): string {
  return dirname(manifestPath);
}

export function resolveTargetDir(input: Readonly<{ manifestDir: string; targetDir: string }>): string {
  return resolveFrom(input.manifestDir, input.targetDir);
}

export function lockPathForManifest(manifestPath: string): string {
  return join(dirname(manifestPath), "arbor.skills.lock.json");
}

export function displayPath(path: string): string {
  return resolve(path);
}
