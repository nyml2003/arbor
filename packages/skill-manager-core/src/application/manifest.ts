import { readJsonFile } from "../adapters/node-fs.js";
import { errorDiagnostic, type SkillDiagnostic } from "../domain/diagnostics.js";
import type { ArborSkillsManifest, LoadManifestInput } from "../domain/types.js";
import { parseManifestJson } from "../domain/validators.js";
import { SkillManagerError } from "./errors.js";
import { resolveManifestPath } from "./paths.js";

export async function loadManifest(input: LoadManifestInput): Promise<ArborSkillsManifest> {
  const manifestPath = resolveManifestPath(input);
  const result = await loadManifestResult(manifestPath);

  if (!result.ok) {
    throw new SkillManagerError("Failed to load arbor.skills.json.", result.diagnostics);
  }

  return result.value;
}

export async function loadManifestDiagnostics(input: LoadManifestInput): Promise<Readonly<{
  manifest: ArborSkillsManifest | null;
  manifestPath: string;
  diagnostics: ReadonlyArray<SkillDiagnostic>;
}>> {
  const manifestPath = resolveManifestPath(input);
  const result = await loadManifestResult(manifestPath);

  if (!result.ok) {
    return { manifest: null, manifestPath, diagnostics: result.diagnostics };
  }

  return { manifest: result.value, manifestPath, diagnostics: [] };
}

async function loadManifestResult(manifestPath: string): Promise<ReturnType<typeof parseManifestJson>> {
  try {
    const json = await readJsonFile(manifestPath);
    return parseManifestJson(json, manifestPath);
  } catch (error) {
    return {
      ok: false,
      diagnostics: [
        errorDiagnostic({
          code: "invalid-json",
          message: error instanceof Error ? error.message : "Failed to read manifest.",
          file: manifestPath,
          path: null,
          hint: "Check that arbor.skills.json exists and contains valid JSON.",
        }),
      ],
    };
  }
}
