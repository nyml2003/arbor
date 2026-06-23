import { registerFileSystemHandlers } from "./filesystem.ipc";
import { registerMemvfsHandlers } from "./memvfs.ipc";
export { setWorkspaceRoot } from "./filesystem.ipc";
export { disposeMemvfsDaemon } from "./memvfs.ipc";

export function registerAllIpcHandlers(): void {
  registerFileSystemHandlers();
  registerMemvfsHandlers();
}
