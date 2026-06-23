import { join, relative } from "node:path";
import {
  assertNoSymlink,
  listRegularFiles,
  pathExists,
  readJsonFile,
  readTextFile,
  realpathIfExists,
  resolveFrom,
} from "../adapters/node-fs.js";
import { errorDiagnostic, type SkillDiagnostic } from "../domain/diagnostics.js";
import { parseSkillMarkdown } from "../domain/front-matter.js";
import type {
  SkillDeclaration,
  SkillPackage,
  SkillPackageManifest,
  SourceSkill,
} from "../domain/types.js";
import { parseSkillPackageJson } from "../domain/validators.js";

export async function normalizePathSource(input: Readonly<{
  declaration: SkillDeclaration;
  manifestDir: string;
}>): Promise<Readonly<{
  skillPackage: SkillPackage | null;
  sourceSkill: SourceSkill | null;
  diagnostics: ReadonlyArray<SkillDiagnostic>;
}>> {
  if (input.declaration.source.type !== "path") {
    return {
      skillPackage: null,
      sourceSkill: null,
      diagnostics: [
        errorDiagnostic({
          code: "unsupported-feature",
          message: `Source type "${input.declaration.source.type}" is not implemented in v1 runtime.`,
          file: null,
          path: null,
          hint: "Use path source first. Git, tarball, and npm are planned next.",
        }),
      ],
    };
  }

  const sourceDir = resolveFrom(input.manifestDir, input.declaration.source.path);
  const sourceRealpath = await realpathIfExists(sourceDir);

  if (sourceRealpath === null) {
    return {
      skillPackage: null,
      sourceSkill: null,
      diagnostics: [
        errorDiagnostic({
          code: "missing-file",
          message: `Source path does not exist: ${sourceDir}`,
          file: null,
          path: sourceDir,
          hint: null,
        }),
      ],
    };
  }

  const skillFilePath = join(sourceRealpath, "SKILL.md");
  const packageFilePath = join(sourceRealpath, "skill.package.json");
  const sourceSkill: SourceSkill = {
    sourceDir: sourceRealpath,
    skillFilePath,
    packageFilePath,
    packageFileExists: await pathExists(packageFilePath),
  };

  const diagnostics: SkillDiagnostic[] = [];
  const symlinkPath = await assertNoSymlink({ rootDir: sourceRealpath });
  if (symlinkPath !== null) {
    diagnostics.push(errorDiagnostic({
      code: "unsafe-symlink",
      message: `Skill payload contains a symlink: ${symlinkPath}`,
      file: null,
      path: symlinkPath,
      hint: "v1 rejects symlink payloads to prevent targetDir escape.",
    }));
  }

  if (!(await pathExists(skillFilePath))) {
    diagnostics.push(errorDiagnostic({
      code: "missing-file",
      message: "SourceSkill must contain SKILL.md.",
      file: skillFilePath,
      path: null,
      hint: null,
    }));
    return { skillPackage: null, sourceSkill, diagnostics };
  }

  const skillText = await readTextFile(skillFilePath);
  const skillResult = parseSkillMarkdown({ filePath: skillFilePath, text: skillText });
  if (!skillResult.ok) {
    return { skillPackage: null, sourceSkill, diagnostics: [...diagnostics, ...skillResult.diagnostics] };
  }

  const packageResult = sourceSkill.packageFileExists
    ? await readSourcePackage(packageFilePath)
    : await generatePackage({ declaration: input.declaration, sourceDir: sourceRealpath, name: skillResult.value.name });

  if (!packageResult.ok) {
    return { skillPackage: null, sourceSkill, diagnostics: [...diagnostics, ...packageResult.diagnostics] };
  }

  diagnostics.push(...await validateNormalizedPackage({
    declaration: input.declaration,
    sourceDir: sourceRealpath,
    packageManifest: packageResult.value.manifest,
    skillName: skillResult.value.name,
    generated: packageResult.value.packageMetadataSource === "generated",
  }));

  if (diagnostics.some((diagnostic) => diagnostic.severity === "error")) {
    return { skillPackage: null, sourceSkill, diagnostics };
  }

  return {
    sourceSkill,
    diagnostics,
    skillPackage: {
      sourceDir: sourceRealpath,
      skill: skillResult.value,
      manifest: packageResult.value.manifest,
      packageMetadataSource: packageResult.value.packageMetadataSource,
    },
  };
}

async function readSourcePackage(packageFilePath: string): Promise<Readonly<{
  ok: true;
  value: Readonly<{ manifest: SkillPackageManifest; packageMetadataSource: "source" }>;
  diagnostics: ReadonlyArray<SkillDiagnostic>;
}> | Readonly<{ ok: false; diagnostics: ReadonlyArray<SkillDiagnostic> }>> {
  try {
    const parsed = parseSkillPackageJson(await readJsonFile(packageFilePath), packageFilePath);
    if (!parsed.ok) {
      return parsed;
    }
    return { ok: true, value: { manifest: parsed.value, packageMetadataSource: "source" }, diagnostics: [] };
  } catch (error) {
    return {
      ok: false,
      diagnostics: [
        errorDiagnostic({
          code: "invalid-json",
          message: error instanceof Error ? error.message : "Failed to read skill.package.json.",
          file: packageFilePath,
          path: null,
          hint: null,
        }),
      ],
    };
  }
}

async function generatePackage(input: Readonly<{
  declaration: SkillDeclaration;
  sourceDir: string;
  name: SkillPackageManifest["name"];
}>): Promise<Readonly<{
  ok: true;
  value: Readonly<{ manifest: SkillPackageManifest; packageMetadataSource: "generated" }>;
  diagnostics: ReadonlyArray<SkillDiagnostic>;
}>> {
  const files = [...await listRegularFiles({ rootDir: input.sourceDir }), "skill.package.json"]
    .map((file) => file.replaceAll("\\", "/"))
    .filter((file, index, all) => all.indexOf(file) === index)
    .sort((left, right) => left.localeCompare(right));

  return {
    ok: true,
    diagnostics: [],
    value: {
      packageMetadataSource: "generated",
      manifest: {
        schema: "arbor.skill-package/v1",
        id: input.declaration.id,
        name: input.name,
        version: input.declaration.version,
        format: "agent-skill",
        files,
      },
    },
  };
}

async function validateNormalizedPackage(input: Readonly<{
  declaration: SkillDeclaration;
  sourceDir: string;
  packageManifest: SkillPackageManifest;
  skillName: SkillPackageManifest["name"];
  generated: boolean;
}>): Promise<ReadonlyArray<SkillDiagnostic>> {
  const diagnostics: SkillDiagnostic[] = [];

  if (input.packageManifest.id !== input.declaration.id) {
    diagnostics.push(errorDiagnostic({
      code: "invalid-schema",
      message: "skill.package.json id must equal arbor.skills.json skills[].id.",
      file: null,
      path: "id",
      hint: null,
    }));
  }

  if (input.packageManifest.version !== input.declaration.version) {
    diagnostics.push(errorDiagnostic({
      code: "version-mismatch",
      message: "skill.package.json version must equal arbor.skills.json skills[].version.",
      file: null,
      path: "version",
      hint: null,
    }));
  }

  if (input.packageManifest.name !== input.skillName) {
    diagnostics.push(errorDiagnostic({
      code: "invalid-schema",
      message: "skill.package.json name must equal SKILL.md front matter name.",
      file: null,
      path: "name",
      hint: null,
    }));
  }

  const fileSet = new Set(input.packageManifest.files);
  if (!fileSet.has("SKILL.md")) {
    diagnostics.push(errorDiagnostic({
      code: "missing-file",
      message: "skill.package.json files must include SKILL.md.",
      file: null,
      path: "files",
      hint: null,
    }));
  }

  if (!fileSet.has("skill.package.json")) {
    diagnostics.push(errorDiagnostic({
      code: "missing-file",
      message: "skill.package.json files must include skill.package.json.",
      file: null,
      path: "files",
      hint: null,
    }));
  }

  for (const file of input.packageManifest.files) {
    if (file.length === 0 || file.startsWith("/") || file.includes("..")) {
      diagnostics.push(errorDiagnostic({
        code: "path-escape",
        message: `Package file path is unsafe: ${file}`,
        file: null,
        path: file,
        hint: "Use relative payload paths inside the Skill directory.",
      }));
      continue;
    }

    if (input.generated && file === "skill.package.json") {
      continue;
    }

    const fullPath = join(input.sourceDir, file);
    const relativePath = relative(input.sourceDir, fullPath);
    if (relativePath.startsWith("..")) {
      diagnostics.push(errorDiagnostic({
        code: "path-escape",
        message: `Package file path escapes source directory: ${file}`,
        file: null,
        path: file,
        hint: null,
      }));
      continue;
    }

    if (!(await pathExists(fullPath))) {
      diagnostics.push(errorDiagnostic({
        code: "missing-file",
        message: `Package file does not exist: ${file}`,
        file: null,
        path: file,
        hint: null,
      }));
    }
  }

  return diagnostics;
}
