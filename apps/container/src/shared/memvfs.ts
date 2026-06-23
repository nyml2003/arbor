export type MemvfsFileKind = "File" | "Directory";

export type MemvfsDebugKind = "Inodes" | "Blocks" | "Free" | "OpenFiles";

export type MemvfsBackendStatus =
  | {
      state: "stopped";
      backend: "daemon" | "memory";
      reason?: string;
    }
  | {
      state: "starting";
      backend: "daemon" | "memory";
      address?: string;
    }
  | {
      state: "running";
      backend: "daemon" | "memory";
      address?: string;
      pid?: number;
    }
  | {
      state: "unavailable";
      backend: "daemon" | "memory";
      reason: string;
    };

export type MemvfsOpenFlags = Readonly<{
  read: boolean;
  write: boolean;
  create: boolean;
  truncate: boolean;
  append: boolean;
}>;

export type MemvfsRequest =
  | { type: "ping" }
  | { type: "shutdown" }
  | { type: "mkdir"; path: string }
  | { type: "open"; path: string; flags: MemvfsOpenFlags }
  | { type: "close"; fd: number }
  | { type: "read"; fd: number; len: number }
  | { type: "write"; fd: number; text: string }
  | { type: "seek"; fd: number; offset: number }
  | { type: "truncate"; path: string; size: number }
  | { type: "readFile"; path: string }
  | { type: "writeFile"; path: string; text: string }
  | { type: "ls"; path: string }
  | { type: "stat"; path: string }
  | { type: "rename"; from: string; to: string }
  | { type: "unlink"; path: string }
  | { type: "rmdir"; path: string }
  | { type: "debug"; kind: MemvfsDebugKind }
  | { type: "statfs" };

export type MemvfsDirEntry = Readonly<{
  name: string;
  inode: number;
  kind: MemvfsFileKind;
}>;

export type MemvfsFileStat = Readonly<{
  inode: number;
  kind: MemvfsFileKind;
  size: number;
  mode: number;
  created_at: number;
  modified_at: number;
  accessed_at: number;
  block_count: number;
  link_count: number;
}>;

export type MemvfsInodeDebug = Readonly<{
  inode: number;
  kind: MemvfsFileKind;
  size: number;
  mode: number;
  created_at: number;
  modified_at: number;
  accessed_at: number;
  link_count: number;
  blocks: number[];
  entries: MemvfsDirEntry[];
}>;

export type MemvfsBlockDebug = Readonly<{
  id: number;
  used: boolean;
  non_zero_bytes: number;
}>;

export type MemvfsOpenFileDebug = Readonly<{
  fd: number;
  inode: number;
  offset: number;
  read: boolean;
  write: boolean;
  append: boolean;
}>;

export type MemvfsStatFs = Readonly<{
  block_size: number;
  total_blocks: number;
  used_blocks: number;
  free_blocks: number;
  inode_count: number;
  inode_capacity: number;
}>;

export type MemvfsResponse =
  | { type: "pong" }
  | { type: "bye" }
  | { type: "unit" }
  | { type: "fd"; fd: number }
  | { type: "text"; text: string; byteLength: number }
  | { type: "count"; count: number }
  | { type: "offset"; offset: number }
  | { type: "dirEntries"; entries: MemvfsDirEntry[] }
  | { type: "stat"; stat: MemvfsFileStat }
  | { type: "inodes"; inodes: MemvfsInodeDebug[] }
  | { type: "blocks"; blocks: MemvfsBlockDebug[] }
  | { type: "freeBlocks"; blocks: number[] }
  | { type: "openFiles"; files: MemvfsOpenFileDebug[] }
  | { type: "statfs"; statfs: MemvfsStatFs };

export type MemvfsRpcError = Readonly<{
  code: string;
  message: string;
}>;

export type MemvfsResult =
  | { ok: true; response: MemvfsResponse }
  | { ok: false; error: MemvfsRpcError };

export type MemvfsApi = Readonly<{
  status(): Promise<MemvfsBackendStatus>;
  start(): Promise<MemvfsBackendStatus>;
  stop(): Promise<MemvfsBackendStatus>;
  reset(): Promise<MemvfsBackendStatus>;
  request(request: MemvfsRequest): Promise<MemvfsResult>;
}>;
