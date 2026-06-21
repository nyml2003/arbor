import type {
  ResumeContact,
  ResumeDocument,
  ResumeEducation,
  ResumeExperience,
  ResumeProject,
} from "./types";

export type ResumeParseResult =
  | { ok: true; data: ResumeDocument }
  | { ok: false; message: string };

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === "string");
}

function readString(record: Record<string, unknown>, key: string): string | null {
  const value = record[key];
  return typeof value === "string" && value.trim().length > 0 ? value : null;
}

function readOptionalString(record: Record<string, unknown>, key: string): string | undefined {
  const value = record[key];
  return typeof value === "string" && value.trim().length > 0 ? value : undefined;
}

function readRange(record: Record<string, unknown>, key: string): [string, string] | null {
  const value = record[key];
  if (!Array.isArray(value) || value.length !== 2) return null;
  const start = value[0];
  const end = value[1];
  return typeof start === "string" && typeof end === "string" ? [start, end] : null;
}

function parseContact(value: unknown): ResumeContact | null {
  if (!isRecord(value)) return null;
  const label = readString(value, "label");
  const contactValue = readString(value, "value");
  if (!label || !contactValue) return null;
  const href = readOptionalString(value, "href");
  return href ? { label, value: contactValue, href } : { label, value: contactValue };
}

function parseEducation(value: unknown): ResumeEducation | null {
  if (!isRecord(value)) return null;
  const school = readString(value, "school");
  const degree = readString(value, "degree");
  const major = readString(value, "major");
  const range = readRange(value, "range");
  if (!school || !degree || !major || !range) return null;

  const tags = isStringArray(value["tags"]) ? value["tags"] : undefined;
  const highlights = isStringArray(value["highlights"]) ? value["highlights"] : undefined;

  return {
    school,
    degree,
    major,
    range,
    ...(tags ? { tags } : {}),
    ...(highlights ? { highlights } : {}),
  };
}

function parseExperience(value: unknown): ResumeExperience | null {
  if (!isRecord(value)) return null;
  const company = readString(value, "company");
  const role = readString(value, "role");
  const range = readRange(value, "range");
  const bullets = value["bullets"];
  if (!company || !role || !range || !isStringArray(bullets)) return null;
  return { company, role, range, bullets };
}

function parseProject(value: unknown): ResumeProject | null {
  if (!isRecord(value)) return null;
  const title = readString(value, "title");
  const tags = value["tags"];
  const bullets = value["bullets"];
  if (!title || !isStringArray(tags) || !isStringArray(bullets)) return null;

  const subtitle = readOptionalString(value, "subtitle");
  return {
    title,
    tags,
    bullets,
    ...(subtitle ? { subtitle } : {}),
  };
}

function parseArray<T>(
  value: unknown,
  parser: (item: unknown) => T | null,
  label: string,
): { ok: true; data: T[] } | { ok: false; message: string } {
  if (!Array.isArray(value)) {
    return { ok: false, message: `${label} 必须是数组。` };
  }

  const parsed: T[] = [];
  for (const item of value) {
    const result = parser(item);
    if (!result) {
      return { ok: false, message: `${label} 中存在格式不正确的条目。` };
    }
    parsed.push(result);
  }
  return { ok: true, data: parsed };
}

export function parseResumeJson(text: string): ResumeParseResult {
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch (error) {
    const detail = error instanceof Error ? error.message : "未知解析错误";
    return { ok: false, message: `resume.json 不是合法 JSON：${detail}` };
  }

  return parseResumeValue(raw);
}

export function parseResumeValue(raw: unknown): ResumeParseResult {
  if (!isRecord(raw) || !isRecord(raw["profile"])) {
    return { ok: false, message: "resume.json 缺少 profile 对象。" };
  }

  const profileRecord = raw["profile"];
  const name = readString(profileRecord, "name");
  const contacts = parseArray(profileRecord["contacts"], parseContact, "profile.contacts");
  if (!name) return { ok: false, message: "profile.name 必须是非空字符串。" };
  if (!contacts.ok) return contacts;

  const education = parseArray(raw["education"], parseEducation, "education");
  if (!education.ok) return education;

  if (!isStringArray(raw["skills"])) {
    return { ok: false, message: "skills 必须是字符串数组。" };
  }

  const experiences = parseArray(raw["experiences"], parseExperience, "experiences");
  if (!experiences.ok) return experiences;

  const projects = parseArray(raw["projects"], parseProject, "projects");
  if (!projects.ok) return projects;

  const title = readOptionalString(profileRecord, "title");
  return {
    ok: true,
    data: {
      profile: {
        name,
        contacts: contacts.data,
        ...(title ? { title } : {}),
      },
      education: education.data,
      skills: raw["skills"],
      experiences: experiences.data,
      projects: projects.data,
    },
  };
}
