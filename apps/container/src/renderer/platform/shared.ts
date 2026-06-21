import type { FileEntry } from "../types";

export function normalizePath(path: string): string {
  return path.replaceAll("\\", "/");
}

export function isResumeRoute(route: string): boolean {
  return route === "show/resume" || route === "resume";
}

export function isResumePrintRoute(route: string): boolean {
  return route === "show/resume/print" || route === "resume/print";
}

export function isResumeEntry(entry: FileEntry): boolean {
  const path = normalizePath(entry.path);
  return (
    (entry.isDirectory && (path === "show/resume" || path.endsWith("/show/resume"))) ||
    (!entry.isDirectory &&
      (path === "show/resume/resume.json" || path.endsWith("/show/resume/resume.json")))
  );
}

export function routeFromEntry(entry: FileEntry): string {
  const path = normalizePath(entry.path);
  if (isResumeEntry(entry)) return "show/resume";
  if (entry.isDirectory && (path === "show" || path.endsWith("/show"))) return "show/home";
  return `file:${entry.path}`;
}

export function routeToWebPath(route: string): string {
  if (route === "show/home") return "/";
  if (route === "show/resume") return "/show/resume";
  if (route === "show/resume/print") return "/show/resume/print";
  return "/";
}
