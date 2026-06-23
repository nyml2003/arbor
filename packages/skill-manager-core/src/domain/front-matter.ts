import { errorDiagnostic, fail, ok, type Result } from "./diagnostics.js";
import { asSkillName } from "./validators.js";
import type { SkillMarkdown } from "./types.js";

export function parseSkillMarkdown(input: Readonly<{ filePath: string; text: string }>): Result<SkillMarkdown> {
  const lines = input.text.split(/\r?\n/);

  if (lines[0] !== "---") {
    return fail(errorDiagnostic({
      code: "invalid-front-matter",
      message: "SKILL.md must start with front matter.",
      file: input.filePath,
      path: null,
      hint: "Start the file with --- and include name and description.",
    }));
  }

  const endIndex = lines.findIndex((line, index) => index > 0 && line === "---");
  if (endIndex === -1) {
    return fail(errorDiagnostic({
      code: "invalid-front-matter",
      message: "SKILL.md front matter is not closed.",
      file: input.filePath,
      path: null,
      hint: "Close front matter with --- before the instruction body.",
    }));
  }

  const frontMatter: Record<string, string> = {};
  for (let index = 1; index < endIndex; index += 1) {
    const line = lines[index] ?? "";
    const separatorIndex = line.indexOf(":");
    if (separatorIndex <= 0) {
      return fail(errorDiagnostic({
        code: "invalid-front-matter",
        message: `Invalid front matter line: ${line}`,
        file: input.filePath,
        path: null,
        hint: "Use simple key: value lines in v1.",
      }));
    }

    const key = line.slice(0, separatorIndex).trim();
    const value = line.slice(separatorIndex + 1).trim().replace(/^"|"$/g, "");
    frontMatter[key] = value;
  }

  const name = frontMatter["name"];
  const description = frontMatter["description"];

  if (typeof name !== "string" || name.length === 0) {
    return fail(errorDiagnostic({
      code: "invalid-front-matter",
      message: "SKILL.md front matter must include name.",
      file: input.filePath,
      path: "name",
      hint: null,
    }));
  }

  if (typeof description !== "string" || description.length === 0) {
    return fail(errorDiagnostic({
      code: "invalid-front-matter",
      message: "SKILL.md front matter must include description.",
      file: input.filePath,
      path: "description",
      hint: null,
    }));
  }

  const nameResult = asSkillName(name, input.filePath, "name");
  if (!nameResult.ok) {
    return nameResult;
  }

  return ok({
    filePath: input.filePath,
    name: nameResult.value,
    description,
    body: lines.slice(endIndex + 1).join("\n"),
    frontMatter,
  });
}
