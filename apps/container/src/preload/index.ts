import { contextBridge, ipcRenderer } from "electron";
import { IpcChannels } from "../shared/channels";
import type { MemvfsApi } from "../shared/memvfs";

export interface FileEntry {
  name: string;
  path: string;
  isDirectory: boolean;
}

const api = {
  getDefaultWorkspace: (): Promise<string> =>
    ipcRenderer.invoke("getDefaultWorkspace"),
  fs: {
    listDirectory: (path: string): Promise<FileEntry[]> =>
      ipcRenderer.invoke(IpcChannels.FS_LIST_DIRECTORY, { path }),
    readText: (path: string): Promise<string> =>
      ipcRenderer.invoke(IpcChannels.FS_READ_TEXT, { path }),
    writeText: (path: string, text: string): Promise<void> =>
      ipcRenderer.invoke(IpcChannels.FS_WRITE_TEXT, { path, text }),
  },
  dialog: {
    selectDirectory: (): Promise<string | null> =>
      ipcRenderer.invoke(IpcChannels.DIALOG_SELECT_DIRECTORY),
  },
  memvfs: {
    status: () => ipcRenderer.invoke(IpcChannels.MEMVFS_STATUS),
    start: () => ipcRenderer.invoke(IpcChannels.MEMVFS_START),
    stop: () => ipcRenderer.invoke(IpcChannels.MEMVFS_STOP),
    reset: () => ipcRenderer.invoke(IpcChannels.MEMVFS_RESET),
    request: (request) => ipcRenderer.invoke(IpcChannels.MEMVFS_REQUEST, request),
  } satisfies MemvfsApi,
};

contextBridge.exposeInMainWorld("arborAPI", api);
