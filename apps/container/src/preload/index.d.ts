export {};

import type { MemvfsApi } from "../shared/memvfs";
import type { ManageApi } from "../shared/manage";

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
      manage: ManageApi;
      memvfs: MemvfsApi;
    };
  }
}
