import type { PlatformAdapter } from "./types";

function resumePath(workspace: string): string {
  return `${workspace}/show/resume/resume.json`;
}

export function createElectronAdapter(): PlatformAdapter {
  return {
    mode: "electron",
    capabilities: {
      workspaceFiles: { status: "supported" },
      staticPages: { status: "supported" },
      resumeSave: { status: "supported" },
    },
    getInitialRoute() {
      return "show/home";
    },
    getDefaultWorkspace() {
      return window.arborAPI.getDefaultWorkspace();
    },
    listDirectory(path) {
      return window.arborAPI.fs.listDirectory(path);
    },
    readText(path) {
      return window.arborAPI.fs.readText(path);
    },
    selectDirectory() {
      return window.arborAPI.dialog.selectDirectory();
    },
    listStaticPages() {
      return [
        { id: "show/home", title: "Show", kind: "page" },
        { id: "show/resume", title: "Resume", kind: "page" },
      ];
    },
    async readResumeJson() {
      const workspace = await window.arborAPI.getDefaultWorkspace();
      const text = await window.arborAPI.fs.readText(resumePath(workspace));
      return JSON.parse(text) as unknown;
    },
    async saveResumeJson(value) {
      const workspace = await window.arborAPI.getDefaultWorkspace();
      await window.arborAPI.fs.writeText(
        resumePath(workspace),
        `${JSON.stringify(value, null, 2)}\n`,
      );
      return { ok: true };
    },
  };
}
