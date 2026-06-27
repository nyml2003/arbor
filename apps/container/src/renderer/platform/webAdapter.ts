import resumeJson from "../../../../../workspace/show/resume/resume.json";
import { createMemoryMemvfsApi } from "../features/memvfs/memoryBackend";
import type { ManageApi, ManageTask } from "../../shared/manage";
import type { FileEntry } from "../types";
import type { PlatformAdapter } from "./types";

const overviewMarkdown = `# Arbor Web Preview

这个页面来自浏览器版打包的静态 workspace。

- Markdown 文件走同一套预览组件
- 普通文本和 JSON 仍然走纯文本查看
- Web 版不直接写回本地文件

\`\`\`text
build -> learn -> manage -> show
\`\`\`
`;

const staticEntries: FileEntry[] = [
  { name: "learn", path: "learn", isDirectory: true },
  { name: "overview.md", path: "learn/overview.md", isDirectory: false },
  { name: "manage", path: "manage", isDirectory: true },
  { name: "tasks.json", path: "manage/tasks.json", isDirectory: false },
  { name: "show", path: "show", isDirectory: true },
  { name: "resume", path: "show/resume", isDirectory: true },
  { name: "resume.json", path: "show/resume/resume.json", isDirectory: false },
  { name: "memvfs", path: "show/memvfs", isDirectory: false },
  { name: "shamrock", path: "show/shamrock", isDirectory: false },
];

const webMemvfs = createMemoryMemvfsApi();
const webManage = createWebManageApi();

export function createWebAdapter(): PlatformAdapter {
  return {
    mode: "web",
    capabilities: {
      workspaceFiles: {
        status: "supported",
        reason: "浏览器版读取构建时打包的静态 workspace，不直接访问本地文件系统。",
      },
      staticPages: { status: "supported" },
      resumeSave: {
        status: "unsupported",
        reason: "Web 版使用构建时打包的数据，不能直接写回源码文件。",
      },
    },
    getInitialRoute() {
      const path = window.location.pathname.replace(/^\/+/, "").replace(/\/+$/, "");
      if (path === "resume/print" || path === "show/resume/print") return "show/resume/print";
      if (path === "resume/edit" || path === "show/resume/edit") return "show/resume/edit";
      if (path === "resume" || path === "show/resume") return "show/resume";
      if (path === "manage" || path === "manage/tasks") return "manage/tasks";
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
      if (path === "learn") {
        return staticEntries.filter((entry) => entry.path === "learn/overview.md");
      }
      if (path === "manage") {
        return staticEntries.filter((entry) => entry.path === "manage/tasks.json");
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
      if (path === "learn/overview.md") {
        return overviewMarkdown;
      }
      if (path === "manage/tasks.json") {
        const listed = await webManage.list();
        return JSON.stringify(listed.ok ? listed.tasks : [], null, 2);
      }
      throw new Error(`Web adapter cannot read "${path}".`);
    },
    async selectDirectory() {
      return null;
    },
    listStaticPages() {
      return [
        { id: "show/home", title: "Show", kind: "page" },
        { id: "manage/tasks", title: "Manage", kind: "page" },
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
    manage: webManage,
    memvfs: webMemvfs,
  };
}

function createWebManageApi(): ManageApi {
  let tasks: ManageTask[] = [
    {
      id: "task_static_demo",
      title: "Review Arbor's static workspace preview",
      status: "todo",
      createdAt: "2026-06-23T00:00:00.000Z",
      updatedAt: "2026-06-23T00:00:00.000Z",
      completedAt: null,
    },
  ];

  const now = () => new Date().toISOString();
  const cloneTasks = () => tasks.map((task) => ({ ...task }));

  return {
    async list() {
      return { ok: true, tasks: cloneTasks() };
    },
    async create(title) {
      const trimmed = title.trim();
      if (trimmed.length === 0) return { ok: false, reason: "Task title cannot be empty." };
      const timestamp = now();
      const task: ManageTask = {
        id: `task_web_${Math.random().toString(36).slice(2, 10)}`,
        title: trimmed,
        status: "todo",
        createdAt: timestamp,
        updatedAt: timestamp,
        completedAt: null,
      };
      tasks = [task, ...tasks];
      return { ok: true, task };
    },
    async update(id, title) {
      const trimmed = title.trim();
      if (trimmed.length === 0) return { ok: false, reason: "Task title cannot be empty." };
      return mutateTask(id, (task) => ({
        ...task,
        title: trimmed,
        updatedAt: now(),
      }));
    },
    async complete(id) {
      const timestamp = now();
      return mutateTask(id, (task) => ({
        ...task,
        status: "done",
        updatedAt: timestamp,
        completedAt: timestamp,
      }));
    },
    async restore(id) {
      return mutateTask(id, (task) => ({
        ...task,
        status: "todo",
        updatedAt: now(),
        completedAt: null,
      }));
    },
  };

  function mutateTask(
    id: string,
    update: (task: ManageTask) => ManageTask,
  ): ReturnType<ManageApi["update"]> {
    const index = tasks.findIndex((task) => task.id === id);
    if (index === -1) return Promise.resolve({ ok: false, reason: `Task not found: ${id}` });
    const current = tasks[index];
    if (current === undefined) return Promise.resolve({ ok: false, reason: `Task not found: ${id}` });
    const next = update(current);
    tasks = tasks.map((task) => task.id === id ? next : task);
    return Promise.resolve({ ok: true, task: next });
  }
}
