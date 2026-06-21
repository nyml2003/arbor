export const RESUME_THEME_IDS = ["classic", "editorial", "signal"] as const;

export type ResumeThemeId = (typeof RESUME_THEME_IDS)[number];

export function isResumeThemeId(value: unknown): value is ResumeThemeId {
  return (
    typeof value === "string" &&
    RESUME_THEME_IDS.includes(value as ResumeThemeId)
  );
}

export interface ResumeContact {
  label: string;
  value: string;
  href?: string;
}

export interface ResumeProfile {
  name: string;
  title?: string;
  contacts: ResumeContact[];
}

export interface ResumeEducation {
  school: string;
  tags?: string[];
  degree: string;
  major: string;
  range: [string, string];
  highlights?: string[];
}

export interface ResumeExperience {
  company: string;
  role: string;
  range: [string, string];
  bullets: string[];
}

export interface ResumeProject {
  title: string;
  subtitle?: string;
  tags: string[];
  bullets: string[];
}

export interface ResumeDocument {
  theme?: ResumeThemeId;
  profile: ResumeProfile;
  education: ResumeEducation[];
  skills: string[];
  experiences: ResumeExperience[];
  projects: ResumeProject[];
}
