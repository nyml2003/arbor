import resumeJson from "../../../../../workspace/show/resume/resume.json";
import { createMemoryMemvfsApi } from "../features/memvfs/memoryBackend";
import type { FileEntry } from "../types";
import type { PlatformAdapter } from "./types";

const staticEntries: FileEntry[] = [
  { name: "show", path: "show", isDirectory: true },
  { name: "resume", path: "show/resume", isDirectory: true },
  { name: "resume.json", path: "show/resume/resume.json", isDirectory: false },
  { name: "memvfs", path: "show/memvfs", isDirectory: false },
  { name: "shamrock", path: "show/shamrock", isDirectory: false },
];

const webMemvfs = createMemoryMemvfsApi();

export function createWebAdapter(): PlatformAdapter {
  return {
    mode: "web",
    capabilities: {
      workspaceFiles: {
        status: "unsupported",
        reason: "浏览器版暂不读取本地 workspace，只展示静态注册页面。",
      },
      staticPages: { status: "supported" },
      resumeSave: {
        status: "unsupported",
        reason: "Web 版使用构建时打包的 resume.json，浏览器不能直接写回源码文件。",
      },
    },
    getInitialRoute() {
      const path = window.location.pathname.replace(/^\/+/, "").replace(/\/+$/, "");
      if (path === "resume/print" || path === "show/resume/print") return "show/resume/print";
      if (path === "resume/edit" || path === "show/resume/edit") return "show/resume/edit";
      if (path === "resume" || path === "show/resume") return "show/resume";
      if (path === "memvfs" || path === "show/memvfs") return "show/memvfs";
      if (path === "shamrock" || path === "show/shamrock") return "show/shamrock";
      return "show/home";
    },
    async getDefaultWorkspace() {
      return "web-static";
    },
    async listDirectory(path) {
      if (path === "web-static" || path === "") {
        return staticEntries.filter((entry) => !entry.path.includes("/"));
      }
      if (path === "show") {
        return staticEntries.filter(
          (entry) =>
            entry.path === "show/resume" ||
            entry.path === "show/memvfs" ||
            entry.path === "show/shamrock",
        );
      }
      if (path === "show/resume") {
        return staticEntries.filter((entry) => entry.path === "show/resume/resume.json");
      }
      return [];
    },
    async readText(path) {
      if (path === "show/resume/resume.json") {
        return JSON.stringify(resumeJson, null, 2);
      }
      throw new Error(`Web adapter cannot read "${path}".`);
    },
    async selectDirectory() {
      return null;
    },
    listStaticPages() {
      return [
        { id: "show/home", title: "Show", kind: "page" },
        { id: "show/resume", title: "Resume", kind: "page" },
        { id: "show/memvfs", title: "memvfs", kind: "page" },
        { id: "show/shamrock", title: "Shamrock", kind: "page" },
      ];
    },
    async readResumeJson() {
      return resumeJson;
    },
    async saveResumeJson() {
      return {
        ok: false,
        reason: "Web 版不能直接保存到 workspace，请下载 JSON 后替换源文件并重新构建。",
      };
    },
    memvfs: webMemvfs,
  };
}
