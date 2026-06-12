export {};

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
      };
      dialog: {
        selectDirectory(): Promise<string | null>;
      };
    };
  }
}
