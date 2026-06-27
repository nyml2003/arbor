import { registerFileSystemHandlers } from "./filesystem.ipc";
import { registerManageHandlers } from "./manage.ipc";
import { registerMemvfsHandlers } from "./memvfs.ipc";
export { setWorkspaceRoot } from "./filesystem.ipc";
export { disposeMemvfsDaemon } from "./memvfs.ipc";

export function registerAllIpcHandlers(): void {
  registerFileSystemHandlers();
  registerManageHandlers();
  registerMemvfsHandlers();
}
