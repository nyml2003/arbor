import { ipcMain, dialog } from "electron";
import { readFile, readdir, stat, writeFile } from "fs/promises";
import { join, resolve as pathResolve, sep } from "path";
import { z } from "zod";
import type { IpcMainInvokeEvent } from "electron";
import { IpcChannels } from "../../shared/channels";

// --- schema helper ---
const FilePathSchema = z.string().min(1);

function createHandler<I, O>(
  schema: z.ZodSchema<I>,
  fn: (input: I) => Promise<O>,
): (event: IpcMainInvokeEvent, raw: unknown) => Promise<O> {
  return async (_event, raw) => {
    const parsed = schema.safeParse(raw);
    if (!parsed.success) {
      const messages = parsed.error.issues.map((i) => i.message).join("; ");
      throw new Error(`Validation failed: ${messages}`);
    }
    return fn(parsed.data);
  };
}

// --- workspace root ---
let workspaceRoot: string | null = null;

export function getWorkspaceRoot(): string | null {
  return workspaceRoot;
}

export function setWorkspaceRoot(root: string): void {
  workspaceRoot = root;
}

function resolveChecked(input: string): string {
  if (!workspaceRoot) {
    throw new Error("No workspace selected.");
  }
  const resolved = pathResolve(input);
  const normalizedRoot = pathResolve(workspaceRoot);
  if (!resolved.startsWith(normalizedRoot + sep) && resolved !== normalizedRoot) {
    throw new Error(`Access denied: "${input}" is outside the workspace.`);
  }
  return resolved;
}

// --- IPC handlers ---
export function registerFileSystemHandlers(): void {
  ipcMain.handle(
    IpcChannels.FS_LIST_DIRECTORY,
    createHandler(z.object({ path: FilePathSchema }), async ({ path }) => {
      const safe = resolveChecked(path);
      const names = await readdir(safe);
      const results = await Promise.allSettled(
        names.map(async (name) => {
          const fullPath = join(safe, name);
          const s = await stat(fullPath);
          return { name, path: fullPath, isDirectory: s.isDirectory() };
        }),
      );
      return results.filter((r) => r.status === "fulfilled").map((r) => r.value);
    }),
  );

  ipcMain.handle(
    IpcChannels.FS_READ_TEXT,
    createHandler(z.object({ path: FilePathSchema }), async ({ path }) => {
      const safe = resolveChecked(path);
      const buffer = await readFile(safe);
      return new TextDecoder("utf-8").decode(buffer);
    }),
  );

  ipcMain.handle(
    IpcChannels.FS_WRITE_TEXT,
    createHandler(z.object({ path: FilePathSchema, text: z.string() }), async ({ path, text }) => {
      const safe = resolveChecked(path);
      await writeFile(safe, text, "utf-8");
      return null;
    }),
  );

  ipcMain.handle(IpcChannels.DIALOG_SELECT_DIRECTORY, async () => {
    const result = await dialog.showOpenDialog({
      properties: ["openDirectory"],
    });
    if (result.canceled || result.filePaths.length === 0) return null;
    workspaceRoot = result.filePaths[0] ?? null;
    return workspaceRoot;
  });
}
