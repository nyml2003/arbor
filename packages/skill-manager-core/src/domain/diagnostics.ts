export type SkillDiagnosticSeverity = "error" | "warning";

export type SkillDiagnosticCode =
  | "invalid-json"
  | "invalid-schema"
  | "invalid-skill-id"
  | "invalid-skill-name"
  | "invalid-version"
  | "invalid-source"
  | "invalid-front-matter"
  | "version-mismatch"
  | "unsupported-feature"
  | "missing-file"
  | "path-escape"
  | "unsafe-symlink"
  | "io-error";

export type SkillDiagnostic = Readonly<{
  code: SkillDiagnosticCode;
  severity: SkillDiagnosticSeverity;
  message: string;
  file: string | null;
  path: string | null;
  hint: string | null;
}>;

export type Result<T> =
  | Readonly<{ ok: true; value: T; diagnostics: ReadonlyArray<SkillDiagnostic> }>
  | Readonly<{ ok: false; diagnostics: ReadonlyArray<SkillDiagnostic> }>;

export function ok<T>(value: T): Result<T> {
  return { ok: true, value, diagnostics: [] };
}

export function fail<T>(diagnostic: SkillDiagnostic): Result<T> {
  return { ok: false, diagnostics: [diagnostic] };
}

export function errorDiagnostic(input: Readonly<{
  code: SkillDiagnosticCode;
  message: string;
  file: string | null;
  path: string | null;
  hint: string | null;
}>): SkillDiagnostic {
  return {
    code: input.code,
    severity: "error",
    message: input.message,
    file: input.file,
    path: input.path,
    hint: input.hint,
  };
}

export function warningDiagnostic(input: Readonly<{
  code: SkillDiagnosticCode;
  message: string;
  file: string | null;
  path: string | null;
  hint: string | null;
}>): SkillDiagnostic {
  return {
    code: input.code,
    severity: "warning",
    message: input.message,
    file: input.file,
    path: input.path,
    hint: input.hint,
  };
}

export function hasErrors(diagnostics: ReadonlyArray<SkillDiagnostic>): boolean {
  return diagnostics.some((diagnostic) => diagnostic.severity === "error");
}
