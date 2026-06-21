import type { FileEntry } from "../types";

export type CapabilityStatus = "supported" | "unsupported";

export interface CapabilityState {
  status: CapabilityStatus;
  reason?: string;
}

export interface StaticPageEntry {
  id: string;
  title: string;
  kind: "page";
}

export interface PlatformAdapter {
  mode: "electron" | "web";
  capabilities: {
    workspaceFiles: CapabilityState;
    staticPages: CapabilityState;
  };
  getInitialRoute(): string;
  getDefaultWorkspace(): Promise<string | null>;
  listDirectory(path: string): Promise<FileEntry[]>;
  readText(path: string): Promise<string>;
  selectDirectory(): Promise<string | null>;
  listStaticPages(): StaticPageEntry[];
  readResumeJson(): Promise<unknown>;
}
