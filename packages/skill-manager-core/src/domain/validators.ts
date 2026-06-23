import {
  type ArborSkillsManifest,
  type NpmSourceSpec,
  type SkillDeclaration,
  type SkillFormat,
  type SkillId,
  type SkillName,
  type SkillPackageManifest,
  type SkillVersion,
  type SourceSpec,
} from "./types.js";
import { errorDiagnostic, fail, ok, type Result, type SkillDiagnostic } from "./diagnostics.js";

const EXACT_SEMVER_PATTERN =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|[0-9A-Za-z-]*[A-Za-z-][0-9A-Za-z]*)(?:\.(?:0|[1-9]\d*|[0-9A-Za-z-]*[A-Za-z-][0-9A-Za-z]*))*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/;

const SKILL_ID_PATTERN = /^[a-z0-9][a-z0-9._-]*\/[a-z0-9][a-z0-9._-]*$/;
const SKILL_NAME_PATTERN = /^[a-z0-9][a-z0-9-]*$/;

export function asSkillId(value: string, file: string | null, path: string | null): Result<SkillId> {
  if (!SKILL_ID_PATTERN.test(value)) {
    return fail(errorDiagnostic({
      code: "invalid-skill-id",
      message: `Invalid skill id "${value}".`,
      file,
      path,
      hint: "Use namespace/name with lowercase letters, numbers, dots, underscores, or dashes.",
    }));
  }

  return ok(value as SkillId);
}

export function asSkillName(value: string, file: string | null, path: string | null): Result<SkillName> {
  if (!SKILL_NAME_PATTERN.test(value)) {
    return fail(errorDiagnostic({
      code: "invalid-skill-name",
      message: `Invalid skill name "${value}".`,
      file,
      path,
      hint: "Use kebab-case, for example plain-tech-writing-cn.",
    }));
  }

  return ok(value as SkillName);
}

export function asSkillVersion(value: string, file: string | null, path: string | null): Result<SkillVersion> {
  if (!EXACT_SEMVER_PATTERN.test(value)) {
    return fail(errorDiagnostic({
      code: "invalid-version",
      message: `Invalid version "${value}".`,
      file,
      path,
      hint: "Use exact SemVer. Ranges, latest, empty values, and loose versions are not allowed.",
    }));
  }

  return ok(value as SkillVersion);
}

export function parseManifestJson(value: unknown, file: string): Result<ArborSkillsManifest> {
  if (!isRecord(value)) {
    return invalidSchema("Manifest must be a JSON object.", file, "$");
  }

  const schema = value["schema"];
  const targetDir = value["targetDir"];
  const skills = value["skills"];

  if (schema !== "arbor.skills/v1") {
    return invalidSchema("Manifest schema must be arbor.skills/v1.", file, "$.schema");
  }

  if (typeof targetDir !== "string" || targetDir.length === 0) {
    return invalidSchema("targetDir must be a non-empty string.", file, "$.targetDir");
  }

  if (!Array.isArray(skills)) {
    return invalidSchema("skills must be an array.", file, "$.skills");
  }

  const declarations: SkillDeclaration[] = [];
  const diagnostics: SkillDiagnostic[] = [];

  for (let index = 0; index < skills.length; index += 1) {
    const declarationResult = parseSkillDeclaration(skills[index], file, `$.skills[${index}]`);
    if (declarationResult.ok) {
      declarations.push(declarationResult.value);
    }
    diagnostics.push(...declarationResult.diagnostics);
  }

  if (diagnostics.length > 0) {
    return { ok: false, diagnostics };
  }

  return ok({
    schema: "arbor.skills/v1",
    targetDir,
    skills: declarations,
  });
}

export function parseSkillPackageJson(value: unknown, file: string): Result<SkillPackageManifest> {
  if (!isRecord(value)) {
    return invalidSchema("skill.package.json must be a JSON object.", file, "$");
  }

  const schema = value["schema"];
  const id = value["id"];
  const name = value["name"];
  const version = value["version"];
  const format = value["format"];
  const files = value["files"];

  if (schema !== "arbor.skill-package/v1") {
    return invalidSchema("Package schema must be arbor.skill-package/v1.", file, "$.schema");
  }

  if (typeof id !== "string") {
    return invalidSchema("Package id must be a string.", file, "$.id");
  }

  if (typeof name !== "string") {
    return invalidSchema("Package name must be a string.", file, "$.name");
  }

  if (typeof version !== "string") {
    return invalidSchema("Package version must be a string.", file, "$.version");
  }

  if (format !== "agent-skill") {
    return invalidSchema("Package format must be agent-skill.", file, "$.format");
  }

  if (!Array.isArray(files) || !files.every((entry) => typeof entry === "string")) {
    return invalidSchema("Package files must be an array of strings.", file, "$.files");
  }

  if (Object.hasOwn(value, "dependencies")) {
    return fail(errorDiagnostic({
      code: "unsupported-feature",
      message: "skill.package.json must not contain dependencies in v1.",
      file,
      path: "$.dependencies",
      hint: "List every required Skill explicitly in arbor.skills.json.",
    }));
  }

  const idResult = asSkillId(id, file, "$.id");
  if (!idResult.ok) {
    return idResult;
  }

  const nameResult = asSkillName(name, file, "$.name");
  if (!nameResult.ok) {
    return nameResult;
  }

  const versionResult = asSkillVersion(version, file, "$.version");
  if (!versionResult.ok) {
    return versionResult;
  }

  return ok({
    schema: "arbor.skill-package/v1",
    id: idResult.value,
    name: nameResult.value,
    version: versionResult.value,
    format: format as SkillFormat,
    files,
  });
}

function parseSkillDeclaration(value: unknown, file: string, path: string): Result<SkillDeclaration> {
  if (!isRecord(value)) {
    return invalidSchema("Skill declaration must be a JSON object.", file, path);
  }

  const id = value["id"];
  const version = value["version"];
  const source = value["source"];

  if (typeof id !== "string") {
    return invalidSchema("Skill id must be a string.", file, `${path}.id`);
  }

  if (typeof version !== "string") {
    return invalidSchema("Skill version must be a string.", file, `${path}.version`);
  }

  const idResult = asSkillId(id, file, `${path}.id`);
  if (!idResult.ok) {
    return idResult;
  }

  const versionResult = asSkillVersion(version, file, `${path}.version`);
  if (!versionResult.ok) {
    return versionResult;
  }

  const sourceResult = parseSourceSpec(source, file, `${path}.source`);
  if (!sourceResult.ok) {
    return sourceResult;
  }

  return ok({
    id: idResult.value,
    version: versionResult.value,
    source: sourceResult.value,
  });
}

function parseSourceSpec(value: unknown, file: string, path: string): Result<SourceSpec> {
  if (!isRecord(value)) {
    return invalidSchema("source must be a JSON object.", file, path);
  }

  const type = value["type"];

  if (type === "path") {
    return parsePathSource(value, file, path);
  }

  if (type === "git") {
    return parseGitSource(value, file, path);
  }

  if (type === "tarball") {
    return parseTarballSource(value, file, path);
  }

  if (type === "npm") {
    return parseNpmSource(value, file, path);
  }

  return fail(errorDiagnostic({
    code: "invalid-source",
    message: "source.type must be path, git, tarball, or npm.",
    file,
    path: `${path}.type`,
    hint: "v1 implements path source first. Other source types are validated but installed later.",
  }));
}

function parsePathSource(value: Readonly<Record<string, unknown>>, file: string, path: string): Result<SourceSpec> {
  const sourcePath = value["path"];
  if (typeof sourcePath !== "string" || sourcePath.length === 0) {
    return invalidSchema("path source requires a non-empty path.", file, `${path}.path`);
  }

  return ok({ type: "path", path: sourcePath });
}

function parseGitSource(value: Readonly<Record<string, unknown>>, file: string, path: string): Result<SourceSpec> {
  const repo = value["repo"];
  const sourcePath = value["path"];
  const ref = value["ref"];

  if (typeof repo !== "string" || repo.length === 0) {
    return invalidSchema("git source requires repo.", file, `${path}.repo`);
  }
  if (typeof sourcePath !== "string" || sourcePath.length === 0) {
    return invalidSchema("git source requires path.", file, `${path}.path`);
  }
  if (typeof ref !== "string" || ref.length === 0) {
    return invalidSchema("git source requires ref.", file, `${path}.ref`);
  }

  return ok({ type: "git", repo, path: sourcePath, ref });
}

function parseTarballSource(value: Readonly<Record<string, unknown>>, file: string, path: string): Result<SourceSpec> {
  const url = value["url"];
  const sourcePath = value["path"];
  const integrity = value["integrity"];

  if (typeof url !== "string" || url.length === 0) {
    return invalidSchema("tarball source requires url.", file, `${path}.url`);
  }
  if (typeof sourcePath !== "string" || sourcePath.length === 0) {
    return invalidSchema("tarball source requires path.", file, `${path}.path`);
  }
  if (typeof integrity !== "string" || integrity.length === 0) {
    return invalidSchema("tarball source requires integrity.", file, `${path}.integrity`);
  }

  return ok({ type: "tarball", url, path: sourcePath, integrity });
}

function parseNpmSource(value: Readonly<Record<string, unknown>>, file: string, path: string): Result<SourceSpec> {
  const packageName = value["package"];
  const version = value["version"];
  const sourcePath = value["path"];

  if (typeof packageName !== "string" || packageName.length === 0) {
    return invalidSchema("npm source requires package.", file, `${path}.package`);
  }
  if (typeof version !== "string") {
    return invalidSchema("npm source requires exact version.", file, `${path}.version`);
  }
  if (typeof sourcePath !== "string" || sourcePath.length === 0) {
    return invalidSchema("npm source requires path.", file, `${path}.path`);
  }

  const versionResult = asSkillVersion(version, file, `${path}.version`);
  if (!versionResult.ok) {
    return versionResult;
  }

  const npmSource: NpmSourceSpec = {
    type: "npm",
    package: packageName,
    version: versionResult.value,
    path: sourcePath,
  };

  return ok(npmSource);
}

function invalidSchema<T>(message: string, file: string, path: string): Result<T> {
  return fail(errorDiagnostic({
    code: "invalid-schema",
    message,
    file,
    path,
    hint: null,
  }));
}

function isRecord(value: unknown): value is Readonly<Record<string, unknown>> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
