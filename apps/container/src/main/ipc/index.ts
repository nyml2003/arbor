import { registerFileSystemHandlers } from "./filesystem.ipc";
export { setWorkspaceRoot } from "./filesystem.ipc";

export function registerAllIpcHandlers(): void {
  registerFileSystemHandlers();
}
