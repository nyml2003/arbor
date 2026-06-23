import {
  copyDirectoryAtomic,
  hashDirectory,
  stageSkillPackage,
} from "../adapters/node-fs.js";
import type {
  ContentHash,
  InstallReport,
  InstallSkillsInput,
  InstalledSkillReport,
  SkillLockEntry,
  SkillPackageManifest,
} from "../domain/types.js";
import { createInstallPlan } from "./plan.js";
import { lockPathForManifest } from "./paths.js";
import { readLock, writeLock } from "./lockfile.js";
import { pruneSkills } from "./prune.js";

export async function installSkills(input: InstallSkillsInput): Promise<InstallReport> {
  const plan = await createInstallPlan(input);
  const existingLock = await readLock({ manifestPath: input.manifestPath, cwd: input.cwd });
  const entries: Record<string, SkillLockEntry> = existingLock === null ? {} : { ...existingLock.skills };
  const installed: InstalledSkillReport[] = [];

  for (const item of plan.items) {
    const generatedPackageManifest =
      item.skillPackage.packageMetadataSource === "generated" ? item.skillPackage.manifest : null;

    const stagedResult = await stageSkillPackage({
      sourceDir: item.sourceDir,
      packageManifest: item.skillPackage.manifest,
      generatedPackageManifest,
      useStagedDir: async (stagedDir) => {
        const contentHash = await hashDirectory(stagedDir);

        if (!input.dryRun) {
          await copyDirectoryAtomic({
            sourceDir: stagedDir,
            targetDir: plan.targetDir,
            targetPath: item.targetPath,
          });
        }

        return {
          contentHash,
          packageManifest: item.skillPackage.manifest,
        };
      },
    });

    const report: InstalledSkillReport = {
      id: item.declaration.id,
      name: stagedResult.packageManifest.name,
      version: stagedResult.packageManifest.version,
      packageMetadataSource: item.skillPackage.packageMetadataSource,
      targetPath: item.targetPath,
      contentHash: stagedResult.contentHash,
      action: input.dryRun ? "planned" : "installed",
    };
    installed.push(report);

    if (!input.dryRun) {
      entries[item.declaration.id] = toLockEntry({
        packageManifest: stagedResult.packageManifest,
        packageMetadataSource: item.skillPackage.packageMetadataSource,
        contentHash: stagedResult.contentHash,
        targetDir: plan.targetDir,
        targetPath: item.targetPath,
        sourcePath: item.declaration.source.type === "path" ? item.declaration.source.path : "",
        sourceResolvedPath: item.sourceDir,
      });
    }
  }

  const lockPath = lockPathForManifest(plan.manifestPath);
  if (!input.dryRun) {
    await writeLock({
      manifestPath: plan.manifestPath,
      nowIso: input.nowIso,
      entries,
    });

    if (input.pruneLock) {
      await pruneSkills({
        manifestPath: plan.manifestPath,
        cwd: plan.manifestDir,
        dryRun: false,
        nowIso: input.nowIso,
      });
    }
  }

  return {
    dryRun: input.dryRun,
    installed,
    lockPath,
  };
}

function toLockEntry(input: Readonly<{
  packageManifest: SkillPackageManifest;
  packageMetadataSource: SkillLockEntry["packageMetadataSource"];
  contentHash: ContentHash;
  targetDir: string;
  targetPath: string;
  sourcePath: string;
  sourceResolvedPath: string;
}>): SkillLockEntry {
  return {
    name: input.packageManifest.name,
    version: input.packageManifest.version,
    packageMetadataSource: input.packageMetadataSource,
    source: {
      type: "path",
      path: input.sourcePath,
      resolvedPath: input.sourceResolvedPath,
    },
    contentHash: input.contentHash,
    install: {
      targetDir: input.targetDir,
      path: input.targetPath,
      mode: "copy",
    },
  };
}
