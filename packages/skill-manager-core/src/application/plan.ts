import { join } from "node:path";
import { ensureInsideDirectory, ensureRealDirectory } from "../adapters/node-fs.js";
import { errorDiagnostic, type SkillDiagnostic } from "../domain/diagnostics.js";
import type {
  CreateInstallPlanInput,
  InstallPlan,
  InstallPlanItem,
} from "../domain/types.js";
import { SkillManagerError } from "./errors.js";
import { loadManifestDiagnostics } from "./manifest.js";
import { manifestDir, resolveManifestPath, resolveTargetDir } from "./paths.js";
import { normalizePathSource } from "./normalize.js";

export async function createInstallPlan(input: CreateInstallPlanInput): Promise<InstallPlan> {
  const manifestPath = resolveManifestPath(input);
  const loaded = await loadManifestDiagnostics(input);
  if (loaded.manifest === null) {
    throw new SkillManagerError("Failed to create install plan.", loaded.diagnostics);
  }

  const currentManifestDir = manifestDir(manifestPath);
  const targetDir = resolveTargetDir({
    manifestDir: currentManifestDir,
    targetDir: loaded.manifest.targetDir,
  });
  await ensureRealDirectory(targetDir);

  const diagnostics: SkillDiagnostic[] = [];
  const items: InstallPlanItem[] = [];

  for (const declaration of loaded.manifest.skills) {
    const normalized = await normalizePathSource({
      declaration,
      manifestDir: currentManifestDir,
    });
    diagnostics.push(...normalized.diagnostics);

    if (normalized.skillPackage === null) {
      continue;
    }

    const targetPath = join(targetDir, normalized.skillPackage.manifest.name);
    const insideTargetDir = await ensureInsideDirectory({
      parentDir: targetDir,
      childPath: targetPath,
    });

    if (!insideTargetDir) {
      diagnostics.push(errorDiagnostic({
        code: "path-escape",
        message: `Install target escapes targetDir: ${targetPath}`,
        file: null,
        path: targetPath,
        hint: null,
      }));
      continue;
    }

    items.push({
      declaration,
      skillPackage: normalized.skillPackage,
      sourceDir: normalized.skillPackage.sourceDir,
      targetPath,
    });
  }

  if (diagnostics.some((diagnostic) => diagnostic.severity === "error")) {
    throw new SkillManagerError("Failed to create install plan.", diagnostics);
  }

  return {
    manifest: loaded.manifest,
    manifestPath,
    manifestDir: currentManifestDir,
    targetDir,
    items,
  };
}
