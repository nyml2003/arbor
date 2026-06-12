import { app, BrowserWindow, ipcMain, shell } from "electron";
import { join, resolve } from "path";
import { existsSync } from "fs";
import { is } from "@electron-toolkit/utils";
import { registerAllIpcHandlers, setWorkspaceRoot } from "./ipc/index";

function resolveDefaultWorkspace(): string {
  // In dev mode, __dirname = apps/container/dist/main
  // Go up to project root: dist/main → dist → container → apps → arbor
  const candidates = [
    resolve(__dirname, "..", "..", "..", "..", "workspace"),
    resolve(__dirname, "..", "..", "workspace"),
    resolve(app.getAppPath(), "..", "workspace"),
  ];
  for (const c of candidates) {
    if (existsSync(c)) return c;
  }
  return candidates[0] ?? "";
}

function createWindow(): BrowserWindow {
  const mainWindow = new BrowserWindow({
    width: 1200,
    height: 800,
    minWidth: 800,
    minHeight: 600,
    show: false,
    backgroundColor: "#1a1b2e",
    webPreferences: {
      preload: join(__dirname, "../preload/index.js"),
      sandbox: true,
      contextIsolation: true,
      nodeIntegration: false,
      webviewTag: false,
    },
  });

  mainWindow.on("ready-to-show", () => {
    mainWindow.show();
  });

  mainWindow.webContents.setWindowOpenHandler((details) => {
    shell.openExternal(details.url).catch((err: unknown) => {
      console.error(err);
    });
    return { action: "deny" };
  });

  if (is.dev && process.env["ELECTRON_RENDERER_URL"]) {
    mainWindow.loadURL(process.env["ELECTRON_RENDERER_URL"]).catch((err: unknown) => {
      console.error(err);
    });
  } else {
    mainWindow.loadFile(join(__dirname, "../renderer/index.html")).catch((err: unknown) => {
      console.error(err);
    });
  }

  return mainWindow;
}

app.whenReady().then(
  () => {
    const root = resolveDefaultWorkspace();
    setWorkspaceRoot(root);
    registerAllIpcHandlers();

    // Tell renderer where the default workspace is
    ipcMain.handle("getDefaultWorkspace", () => root);

    createWindow();

    app.on("activate", () => {
      if (BrowserWindow.getAllWindows().length === 0) {
        createWindow();
      }
    });
  },
  (err: unknown) => {
    console.error(err);
  },
);

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});
