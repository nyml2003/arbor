export {};

import type { MemvfsApi } from "../shared/memvfs";

interface FileEntry {
  name: string;
  path: string;
  isDirectory: boolean;
}

declare global {
  interface Window {
    readonly arborAPI: {
      getDefaultWorkspace(): Promise<string>;
      fs: {
        listDirectory(path: string): Promise<FileEntry[]>;
        readText(path: string): Promise<string>;
        writeText(path: string, text: string): Promise<void>;
      };
      dialog: {
        selectDirectory(): Promise<string | null>;
      };
      memvfs: MemvfsApi;
    };
  }
}
