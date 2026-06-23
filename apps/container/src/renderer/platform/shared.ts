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

export function isResumeEditRoute(route: string): boolean {
  return route === "show/resume/edit" || route === "resume/edit";
}

export function isResumeEntry(entry: FileEntry): boolean {
  const path = normalizePath(entry.path);
  return (
    (entry.isDirectory && (path === "show/resume" || path.endsWith("/show/resume"))) ||
    (!entry.isDirectory &&
      (path === "show/resume/resume.json" || path.endsWith("/show/resume/resume.json")))
  );
}

export function isMemvfsEntry(entry: FileEntry): boolean {
  const path = normalizePath(entry.path);
  return path === "show/memvfs" || path.endsWith("/show/memvfs");
}

export function isShamrockEntry(entry: FileEntry): boolean {
  const path = normalizePath(entry.path);
  return path === "show/shamrock" || path.endsWith("/show/shamrock");
}

export function routeFromEntry(entry: FileEntry): string {
  const path = normalizePath(entry.path);
  if (isResumeEntry(entry)) return "show/resume";
  if (isMemvfsEntry(entry)) return "show/memvfs";
  if (isShamrockEntry(entry)) return "show/shamrock";
  if (entry.isDirectory && (path === "show" || path.endsWith("/show"))) return "show/home";
  return `file:${entry.path}`;
}

export function routeToWebPath(route: string): string {
  if (route === "show/home") return "/";
  if (route === "show/resume") return "/show/resume";
  if (route === "show/resume/edit") return "/show/resume/edit";
  if (route === "show/resume/print") return "/show/resume/print";
  if (route === "show/memvfs") return "/show/memvfs";
  if (route === "show/shamrock") return "/show/shamrock";
  return "/";
}
