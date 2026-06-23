import type { SkillDiagnostic } from "../domain/diagnostics.js";

export class SkillManagerError extends Error {
  readonly diagnostics: ReadonlyArray<SkillDiagnostic>;

  constructor(message: string, diagnostics: ReadonlyArray<SkillDiagnostic>) {
    super(message);
    this.name = "SkillManagerError";
    this.diagnostics = diagnostics;
  }
}

export function throwIfDiagnostics(message: string, diagnostics: ReadonlyArray<SkillDiagnostic>): void {
  if (diagnostics.some((diagnostic) => diagnostic.severity === "error")) {
    throw new SkillManagerError(message, diagnostics);
  }
}
