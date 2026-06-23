import { ipcMain } from "electron";
import { spawn, type ChildProcessWithoutNullStreams } from "child_process";
import { existsSync } from "fs";
import { Socket } from "net";
import { resolve } from "path";
import { z } from "zod";
import { IpcChannels } from "../../shared/channels";
import type {
  MemvfsBackendStatus,
  MemvfsDebugKind,
  MemvfsRequest,
  MemvfsResult,
} from "../../shared/memvfs";

const DEFAULT_ADDRESS = "127.0.0.1:7878";
const REQUEST_TIMEOUT_MS = 3_000;
const STARTUP_TIMEOUT_MS = 30_000;
const STARTUP_POLL_MS = 100;

type RustRequest =
  | "Ping"
  | "Shutdown"
  | { Mkdir: { path: string; mode: null } }
  | { Open: { path: string; flags: RustOpenFlags; mode: null } }
  | { Close: { fd: number } }
  | { Read: { fd: number; len: number } }
  | { Write: { fd: number; bytes: number[] } }
  | { Seek: { fd: number; offset: number } }
  | { Truncate: { path: string; size: number } }
  | { ReadFile: { path: string } }
  | { WriteFile: { path: string; bytes: number[] } }
  | { Ls: { path: string } }
  | { Stat: { path: string } }
  | { Rename: { from: string; to: string } }
  | { Unlink: { path: string } }
  | { Rmdir: { path: string } }
  | { Debug: { kind: MemvfsDebugKind } }
  | "StatFs";

type RustOpenFlags = Readonly<{
  read: boolean;
  write: boolean;
  create: boolean;
  truncate: boolean;
  append: boolean;
}>;

type DaemonState = {
  process: ChildProcessWithoutNullStreams | null;
  attached: boolean;
  lastError: string | null;
};

type DaemonLaunch = Readonly<{
  command: string;
  args: string[];
  cwd: string;
}>;

const state: DaemonState = {
  process: null,
  attached: false,
  lastError: null,
};

const AbsolutePathSchema = z.string().min(1).startsWith("/");
const DebugKindSchema = z.enum(["Inodes", "Blocks", "Free", "OpenFiles"]);

const RequestSchema: z.ZodType<MemvfsRequest> = z.discriminatedUnion("type", [
  z.object({ type: z.literal("ping") }),
  z.object({ type: z.literal("shutdown") }),
  z.object({ type: z.literal("mkdir"), path: AbsolutePathSchema }),
  z.object({
    type: z.literal("open"),
    path: AbsolutePathSchema,
    flags: z.object({
      read: z.boolean(),
      write: z.boolean(),
      create: z.boolean(),
      truncate: z.boolean(),
      append: z.boolean(),
    }),
  }),
  z.object({ type: z.literal("close"), fd: z.number().int().nonnegative() }),
  z.object({ type: z.literal("read"), fd: z.number().int().nonnegative(), len: z.number().int().min(0) }),
  z.object({ type: z.literal("write"), fd: z.number().int().nonnegative(), text: z.string() }),
  z.object({ type: z.literal("seek"), fd: z.number().int().nonnegative(), offset: z.number().int().min(0) }),
  z.object({ type: z.literal("truncate"), path: AbsolutePathSchema, size: z.number().int().min(0) }),
  z.object({ type: z.literal("readFile"), path: AbsolutePathSchema }),
  z.object({ type: z.literal("writeFile"), path: AbsolutePathSchema, text: z.string() }),
  z.object({ type: z.literal("ls"), path: AbsolutePathSchema }),
  z.object({ type: z.literal("stat"), path: AbsolutePathSchema }),
  z.object({ type: z.literal("rename"), from: AbsolutePathSchema, to: AbsolutePathSchema }),
  z.object({ type: z.literal("unlink"), path: AbsolutePathSchema }),
  z.object({ type: z.literal("rmdir"), path: AbsolutePathSchema }),
  z.object({ type: z.literal("debug"), kind: DebugKindSchema }),
  z.object({ type: z.literal("statfs") }),
]);

function splitAddress(address: string): { host: string; port: number } {
  const [host, portText] = address.split(":");
  const port = Number(portText);
  if (!host || !Number.isInteger(port)) {
    throw new Error(`Invalid memvfs address: ${address}`);
  }
  return { host, port };
}

function repoRoot(): string {
  return resolve(__dirname, "..", "..", "..", "..");
}

function daemonLaunch(): DaemonLaunch {
  const binaryName = process.platform === "win32" ? "memvfsd.exe" : "memvfsd";
  const memvfsRoot = resolve(repoRoot(), "apps", "memvfs");
  const binary = resolve(memvfsRoot, "target", "debug", binaryName);
  const args = [
    "start",
    "--addr",
    DEFAULT_ADDRESS,
    "--block-size",
    "16",
    "--capacity",
    "4096",
    "--inode-capacity",
    "96",
  ];
  if (existsSync(binary)) {
    return { command: binary, args, cwd: memvfsRoot };
  }

  return {
    command: process.platform === "win32" ? "cargo.cmd" : "cargo",
    args: ["run", "-q", "-p", "memvfsd", "--", ...args],
    cwd: memvfsRoot,
  };
}

function status(): MemvfsBackendStatus {
  const child = state.process;
  if (child || state.attached) {
    if (child?.pid !== undefined) {
      return {
        state: "running",
        backend: "daemon",
        address: DEFAULT_ADDRESS,
        pid: child.pid,
      };
    }
    return {
      state: "running",
      backend: "daemon",
      address: DEFAULT_ADDRESS,
    };
  }

  if (state.lastError) {
    return {
      state: "unavailable",
      backend: "daemon",
      reason: state.lastError,
    };
  }

  return {
    state: "stopped",
    backend: "daemon",
  };
}

async function startDaemon(): Promise<MemvfsBackendStatus> {
  if (state.process || state.attached) return status();

  const existing = await sendRustRequest("Ping").catch(() => null);
  if (existing?.ok && existing.response.type === "pong") {
    state.attached = true;
    state.lastError = null;
    return status();
  }

  const launch = daemonLaunch();
  state.lastError = null;
  const child = spawn(launch.command, launch.args, {
    cwd: launch.cwd,
    stdio: "pipe",
    windowsHide: true,
  });
  state.process = child;

  child.once("exit", (code, signal) => {
    if (state.process === child) {
      state.process = null;
      state.attached = false;
      if (code !== 0 && signal !== "SIGTERM") {
        state.lastError = `memvfsd exited with code ${String(code)} signal ${String(signal)}`;
      }
    }
  });
  child.once("error", (error) => {
    if (state.process === child) {
      state.process = null;
      state.attached = false;
      state.lastError = error.message;
    }
  });
  child.stderr.on("data", (chunk: Buffer) => {
    const message = chunk.toString("utf-8").trim();
    if (message.length > 0) state.lastError = message;
  });

  const ready = await waitForDaemon();
  if (!ready) {
    await stopDaemon();
    state.lastError = state.lastError ?? "memvfsd did not become ready before timeout.";
  }
  return status();
}

async function waitForDaemon(): Promise<boolean> {
  const startedAt = Date.now();
  while (Date.now() - startedAt < STARTUP_TIMEOUT_MS) {
    const result = await sendRustRequest("Ping").catch(() => null);
    if (result && result.ok && result.response.type === "pong") {
      return true;
    }
    await delay(STARTUP_POLL_MS);
  }
  return false;
}

async function stopDaemon(): Promise<MemvfsBackendStatus> {
  const child = state.process;
  if (!child && !state.attached) {
    state.lastError = null;
    return status();
  }

  await sendRustRequest("Shutdown").catch(() => null);
  state.attached = false;
  if (!child) {
    state.lastError = null;
    return status();
  }
  await waitForExit(child, 800);
  if (!child.killed && child.exitCode === null) {
    child.kill();
  }
  if (state.process === child) {
    state.process = null;
  }
  state.lastError = null;
  return status();
}

async function resetDaemon(): Promise<MemvfsBackendStatus> {
  await stopDaemon();
  return startDaemon();
}

function waitForExit(child: ChildProcessWithoutNullStreams, timeoutMs: number): Promise<void> {
  return new Promise((resolveWait) => {
    if (child.exitCode !== null) {
      resolveWait();
      return;
    }

    const timer = setTimeout(() => {
      child.off("exit", onExit);
      resolveWait();
    }, timeoutMs);
    const onExit = () => {
      clearTimeout(timer);
      resolveWait();
    };
    child.once("exit", onExit);
  });
}

function delay(ms: number): Promise<void> {
  return new Promise((resolveDelay) => {
    setTimeout(resolveDelay, ms);
  });
}

function toRustRequest(request: MemvfsRequest): RustRequest {
  switch (request.type) {
    case "ping":
      return "Ping";
    case "shutdown":
      return "Shutdown";
    case "mkdir":
      return { Mkdir: { path: request.path, mode: null } };
    case "open":
      return { Open: { path: request.path, flags: request.flags, mode: null } };
    case "close":
      return { Close: { fd: request.fd } };
    case "read":
      return { Read: { fd: request.fd, len: request.len } };
    case "write":
      return { Write: { fd: request.fd, bytes: textToBytes(request.text) } };
    case "seek":
      return { Seek: { fd: request.fd, offset: request.offset } };
    case "truncate":
      return { Truncate: { path: request.path, size: request.size } };
    case "readFile":
      return { ReadFile: { path: request.path } };
    case "writeFile":
      return { WriteFile: { path: request.path, bytes: textToBytes(request.text) } };
    case "ls":
      return { Ls: { path: request.path } };
    case "stat":
      return { Stat: { path: request.path } };
    case "rename":
      return { Rename: { from: request.from, to: request.to } };
    case "unlink":
      return { Unlink: { path: request.path } };
    case "rmdir":
      return { Rmdir: { path: request.path } };
    case "debug":
      return { Debug: { kind: request.kind } };
    case "statfs":
      return "StatFs";
  }
}

function textToBytes(text: string): number[] {
  return Array.from(new TextEncoder().encode(text));
}

function bytesToText(bytes: unknown): { text: string; byteLength: number } {
  const values = z.array(z.number().int().min(0).max(255)).parse(bytes);
  return {
    text: new TextDecoder("utf-8", { fatal: false }).decode(Uint8Array.from(values)),
    byteLength: values.length,
  };
}

function toFrontendResponse(raw: unknown): MemvfsResult {
  if (raw === "Pong") return { ok: true, response: { type: "pong" } };
  if (raw === "Bye") return { ok: true, response: { type: "bye" } };
  if (raw === "Unit") return { ok: true, response: { type: "unit" } };

  if (isRecord(raw)) {
    if ("Fd" in raw) return { ok: true, response: { type: "fd", fd: z.number().parse(raw["Fd"]) } };
    if ("Bytes" in raw) {
      const decoded = bytesToText(raw["Bytes"]);
      return { ok: true, response: { type: "text", ...decoded } };
    }
    if ("Count" in raw) {
      return { ok: true, response: { type: "count", count: z.number().parse(raw["Count"]) } };
    }
    if ("Offset" in raw) {
      return { ok: true, response: { type: "offset", offset: z.number().parse(raw["Offset"]) } };
    }
    if ("DirEntries" in raw) {
      return { ok: true, response: { type: "dirEntries", entries: z.array(z.any()).parse(raw["DirEntries"]) } };
    }
    if ("Stat" in raw) {
      return { ok: true, response: { type: "stat", stat: z.any().parse(raw["Stat"]) } };
    }
    if ("Inodes" in raw) {
      return { ok: true, response: { type: "inodes", inodes: z.array(z.any()).parse(raw["Inodes"]) } };
    }
    if ("Blocks" in raw) {
      return { ok: true, response: { type: "blocks", blocks: z.array(z.any()).parse(raw["Blocks"]) } };
    }
    if ("FreeBlocks" in raw) {
      return { ok: true, response: { type: "freeBlocks", blocks: z.array(z.number()).parse(raw["FreeBlocks"]) } };
    }
    if ("OpenFiles" in raw) {
      return { ok: true, response: { type: "openFiles", files: z.array(z.any()).parse(raw["OpenFiles"]) } };
    }
    if ("StatFs" in raw) {
      return { ok: true, response: { type: "statfs", statfs: z.any().parse(raw["StatFs"]) } };
    }
    if ("Error" in raw) {
      const error = z.object({ code: z.string(), message: z.string() }).parse(raw["Error"]);
      return { ok: false, error };
    }
  }

  return {
    ok: false,
    error: {
      code: "EPROTO",
      message: `Unsupported memvfs response: ${JSON.stringify(raw)}`,
    },
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function sendRustRequest(request: RustRequest): Promise<MemvfsResult> {
  return new Promise((resolveRequest, rejectRequest) => {
    const { host, port } = splitAddress(DEFAULT_ADDRESS);
    const socket = new Socket();
    const chunks: Buffer[] = [];
    let expectedBytes: number | null = null;
    let settled = false;

    const finish = (fn: () => void) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      socket.destroy();
      fn();
    };

    const timer = setTimeout(() => {
      finish(() => {
        rejectRequest(new Error("memvfs request timed out"));
      });
    }, REQUEST_TIMEOUT_MS);

    socket.once("error", (error) => {
      finish(() => {
        rejectRequest(error);
      });
    });

    socket.on("data", (chunk) => {
      chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk, "utf-8"));
      const buffer = Buffer.concat(chunks);
      if (expectedBytes === null && buffer.length >= 4) {
        expectedBytes = buffer.readUInt32BE(0);
      }
      if (expectedBytes !== null && buffer.length >= expectedBytes + 4) {
        const payload = buffer.subarray(4, expectedBytes + 4);
        finish(() => {
          resolveRequest(toFrontendResponse(JSON.parse(payload.toString("utf-8"))));
        });
      }
    });

    socket.connect(port, host, () => {
      const payload = Buffer.from(JSON.stringify(request), "utf-8");
      const header = Buffer.alloc(4);
      header.writeUInt32BE(payload.length);
      socket.write(Buffer.concat([header, payload]));
    });
  });
}

export function registerMemvfsHandlers(): void {
  ipcMain.handle(IpcChannels.MEMVFS_STATUS, () => status());
  ipcMain.handle(IpcChannels.MEMVFS_START, () => startDaemon());
  ipcMain.handle(IpcChannels.MEMVFS_STOP, () => stopDaemon());
  ipcMain.handle(IpcChannels.MEMVFS_RESET, () => resetDaemon());
  ipcMain.handle(IpcChannels.MEMVFS_REQUEST, async (_event, raw: unknown) => {
    const parsed = RequestSchema.safeParse(raw);
    if (!parsed.success) {
      const message = parsed.error.issues.map((issue) => issue.message).join("; ");
      return {
        ok: false,
        error: {
          code: "EINVAL",
          message,
        },
      } satisfies MemvfsResult;
    }

    if (!state.process && parsed.data.type !== "ping") {
      const current = await startDaemon();
      if (current.state !== "running") {
        return {
          ok: false,
          error: {
            code: "EDAEMON",
            message:
              current.state === "unavailable"
                ? current.reason
                : "memvfs daemon is not running",
          },
        } satisfies MemvfsResult;
      }
    }

    return sendRustRequest(toRustRequest(parsed.data)).catch((error: unknown) => {
      return {
        ok: false,
        error: {
          code: "EIO",
          message: error instanceof Error ? error.message : String(error),
        },
      } satisfies MemvfsResult;
    });
  });
}

export function disposeMemvfsDaemon(): void {
  const child = state.process;
  state.process = null;
  state.attached = false;
  if (child && child.exitCode === null) {
    child.kill();
  }
}
