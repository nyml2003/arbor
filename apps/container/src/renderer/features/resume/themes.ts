import type { ResumeThemeId } from "./types";

export interface ResumeThemeOption {
  id: ResumeThemeId;
  label: string;
  description: string;
}

export const defaultResumeThemeId: ResumeThemeId = "classic";

export const resumeThemeOptions: ResumeThemeOption[] = [
  {
    id: "classic",
    label: "Classic Blue",
    description: "保留当前蓝绿 badge 和暖白纸张，适合通用投递。",
  },
  {
    id: "editorial",
    label: "Editorial Red",
    description: "深墨文字配酒红标题，更像编辑排版。",
  },
  {
    id: "signal",
    label: "Signal Green",
    description: "石墨正文配青绿色强调，技术感更强。",
  },
];

export function resolveResumeThemeId(theme: ResumeThemeId | undefined): ResumeThemeId {
  return theme ?? defaultResumeThemeId;
}
