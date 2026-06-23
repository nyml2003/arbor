import type { ArborSkillsLock } from "./types.js";
import { errorDiagnostic, fail, ok, type Result } from "./diagnostics.js";

export function parseLockJson(value: unknown, file: string): Result<ArborSkillsLock> {
  if (!isRecord(value)) {
    return fail(errorDiagnostic({
      code: "invalid-schema",
      message: "Lockfile must be a JSON object.",
      file,
      path: "$",
      hint: null,
    }));
  }

  if (value["schema"] !== "arbor.skills-lock/v1") {
    return fail(errorDiagnostic({
      code: "invalid-schema",
      message: "Lockfile schema must be arbor.skills-lock/v1.",
      file,
      path: "$.schema",
      hint: null,
    }));
  }

  const generatedAt = value["generatedAt"];
  const skills = value["skills"];

  if (typeof generatedAt !== "string" || !isRecord(skills)) {
    return fail(errorDiagnostic({
      code: "invalid-schema",
      message: "Lockfile must include generatedAt and skills.",
      file,
      path: "$",
      hint: null,
    }));
  }

  return ok(value as ArborSkillsLock);
}

function isRecord(value: unknown): value is Readonly<Record<string, unknown>> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
