import type { SkillDiagnostic } from "../domain/diagnostics.js";
import type { LintManifestInput } from "../domain/types.js";
import { loadManifestDiagnostics } from "./manifest.js";
import { normalizePathSource } from "./normalize.js";
import { manifestDir, resolveManifestPath } from "./paths.js";

export async function lintManifest(input: LintManifestInput): Promise<ReadonlyArray<SkillDiagnostic>> {
  const manifestPath = resolveManifestPath(input);
  const loaded = await loadManifestDiagnostics(input);
  const diagnostics: SkillDiagnostic[] = [...loaded.diagnostics];

  if (loaded.manifest === null) {
    return diagnostics;
  }

  const currentManifestDir = manifestDir(manifestPath);
  for (const declaration of loaded.manifest.skills) {
    const normalized = await normalizePathSource({
      declaration,
      manifestDir: currentManifestDir,
    });
    diagnostics.push(...normalized.diagnostics);
  }

  return diagnostics;
}
