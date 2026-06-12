import { contextBridge, ipcRenderer } from "electron";
import { IpcChannels } from "../shared/channels";

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
  },
  dialog: {
    selectDirectory: (): Promise<string | null> =>
      ipcRenderer.invoke(IpcChannels.DIALOG_SELECT_DIRECTORY),
  },
};

contextBridge.exposeInMainWorld("arborAPI", api);
