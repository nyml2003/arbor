import type {
  MemvfsApi,
  MemvfsBackendStatus,
  MemvfsBlockDebug,
  MemvfsDirEntry,
  MemvfsFileKind,
  MemvfsFileStat,
  MemvfsInodeDebug,
  MemvfsOpenFileDebug,
  MemvfsRequest,
  MemvfsResponse,
  MemvfsResult,
  MemvfsStatFs,
} from "../../../shared/memvfs";

type Inode = {
  id: number;
  kind: MemvfsFileKind;
  mode: number;
  size: number;
  createdAt: number;
  modifiedAt: number;
  accessedAt: number;
  linkCount: number;
  blocks: number[];
  entries: Map<string, number>;
};

type OpenFile = {
  inode: number;
  offset: number;
  read: boolean;
  write: boolean;
  append: boolean;
};

const BLOCK_SIZE = 16;
const TOTAL_BLOCKS = 256;
const INODE_CAPACITY = 96;
const ROOT_INODE = 1;

export function createMemoryMemvfsApi(): MemvfsApi {
  const fs = new MemoryFileSystem();

  const runningStatus = (): MemvfsBackendStatus => ({
    state: "running",
    backend: "memory",
  });

  return {
    async status() {
      return runningStatus();
    },
    async start() {
      return runningStatus();
    },
    async stop() {
      return runningStatus();
    },
    async reset() {
      fs.reset();
      return runningStatus();
    },
    async request(request) {
      return fs.handle(request);
    },
  };
}

class MemoryFileSystem {
  private inodes = new Map<number, Inode>();
  private blocks: Uint8Array[] = [];
  private freeBlocks: number[] = [];
  private openFiles = new Map<number, OpenFile>();
  private nextInode = ROOT_INODE + 1;
  private nextFd = 3;
  private clock = 1;

  constructor() {
    this.reset();
  }

  reset(): void {
    this.inodes = new Map();
    this.blocks = Array.from({ length: TOTAL_BLOCKS }, () => new Uint8Array(BLOCK_SIZE));
    this.freeBlocks = Array.from({ length: TOTAL_BLOCKS }, (_, index) => TOTAL_BLOCKS - index - 1);
    this.openFiles = new Map();
    this.nextInode = ROOT_INODE + 1;
    this.nextFd = 3;
    this.clock = 1;
    this.inodes.set(ROOT_INODE, {
      id: ROOT_INODE,
      kind: "Directory",
      mode: 0o755,
      size: 0,
      createdAt: 1,
      modifiedAt: 1,
      accessedAt: 1,
      linkCount: 1,
      blocks: [],
      entries: new Map(),
    });
    this.mkdir("/docs");
    this.writeFile("/docs/hello.txt", "hello from memvfs");
    this.writeFile("/notes.txt", "This file lives only in memory.");
  }

  handle(request: MemvfsRequest): MemvfsResult {
    try {
      switch (request.type) {
        case "ping":
          return ok({ type: "pong" });
        case "shutdown":
          return ok({ type: "bye" });
        case "mkdir":
          this.mkdir(request.path);
          return ok({ type: "unit" });
        case "open":
          return ok({ type: "fd", fd: this.open(request) });
        case "close":
          this.close(request.fd);
          return ok({ type: "unit" });
        case "read":
          return ok(this.read(request.fd, request.len));
        case "write":
          return ok({ type: "count", count: this.write(request.fd, request.text) });
        case "seek":
          return ok({ type: "offset", offset: this.seek(request.fd, request.offset) });
        case "truncate":
          this.truncate(request.path, request.size);
          return ok({ type: "unit" });
        case "readFile":
          return ok(this.readFile(request.path));
        case "writeFile":
          return ok({ type: "count", count: this.writeFile(request.path, request.text) });
        case "ls":
          return ok({ type: "dirEntries", entries: this.ls(request.path) });
        case "stat":
          return ok({ type: "stat", stat: this.stat(request.path) });
        case "rename":
          this.rename(request.from, request.to);
          return ok({ type: "unit" });
        case "unlink":
          this.unlink(request.path);
          return ok({ type: "unit" });
        case "rmdir":
          this.rmdir(request.path);
          return ok({ type: "unit" });
        case "debug":
          return ok(this.debug(request.kind));
        case "statfs":
          return ok({ type: "statfs", statfs: this.statfs() });
      }
    } catch (error) {
      return {
        ok: false,
        error: {
          code: error instanceof FsFailure ? error.code : "EIO",
          message: error instanceof Error ? error.message : String(error),
        },
      };
    }
  }

  private mkdir(path: string): void {
    this.createNode(path, "Directory", 0o755);
  }

  private open(request: Extract<MemvfsRequest, { type: "open" }>): number {
    if (!request.flags.read && !request.flags.write) {
      fail("EINVAL", "open requires read or write access");
    }

    let inode = this.resolve(pathOrThrow(request.path));
    if (!inode && request.flags.create) {
      inode = this.createNode(request.path, "File", 0o644);
    }
    if (!inode) fail("ENOENT", "path does not exist");
    if (inode.kind !== "File") fail("EISDIR", "cannot open a directory as a file");
    if (request.flags.truncate) {
      if (!request.flags.write) fail("EINVAL", "truncate requires write access");
      this.truncateInode(inode, 0);
    }

    const fd = this.nextFd;
    this.nextFd += 1;
    this.openFiles.set(fd, {
      inode: inode.id,
      offset: request.flags.append ? inode.size : 0,
      read: request.flags.read,
      write: request.flags.write,
      append: request.flags.append,
    });
    return fd;
  }

  private close(fd: number): void {
    if (!this.openFiles.delete(fd)) fail("EBADF", "invalid file descriptor");
  }

  private read(fd: number, len: number): MemvfsResponse {
    const open = this.openFile(fd);
    if (!open.read) fail("EBADF", "file descriptor is not readable");
    const inode = this.inode(open.inode);
    const end = Math.min(inode.size, open.offset + len);
    const bytes = this.readBytes(inode, open.offset, end - open.offset);
    open.offset = end;
    inode.accessedAt = this.tick();
    return {
      type: "text",
      text: new TextDecoder("utf-8", { fatal: false }).decode(bytes),
      byteLength: bytes.length,
    };
  }

  private write(fd: number, text: string): number {
    const open = this.openFile(fd);
    if (!open.write) fail("EBADF", "file descriptor is not writable");
    const inode = this.inode(open.inode);
    const offset = open.append ? inode.size : open.offset;
    const bytes = new TextEncoder().encode(text);
    this.writeBytes(inode, offset, bytes);
    open.offset = offset + bytes.length;
    return bytes.length;
  }

  private seek(fd: number, offset: number): number {
    const open = this.openFile(fd);
    open.offset = offset;
    return offset;
  }

  private truncate(path: string, size: number): void {
    const inode = this.mustResolve(path);
    if (inode.kind !== "File") fail("EISDIR", "cannot truncate a directory");
    this.truncateInode(inode, size);
  }

  private readFile(path: string): MemvfsResponse {
    const inode = this.mustResolve(path);
    if (inode.kind !== "File") fail("EISDIR", "cannot read a directory as a file");
    const bytes = this.readBytes(inode, 0, inode.size);
    inode.accessedAt = this.tick();
    return {
      type: "text",
      text: new TextDecoder("utf-8", { fatal: false }).decode(bytes),
      byteLength: bytes.length,
    };
  }

  private writeFile(path: string, text: string): number {
    let inode = this.resolve(pathOrThrow(path));
    if (!inode) {
      inode = this.createNode(path, "File", 0o644);
    }
    if (inode.kind !== "File") fail("EISDIR", "cannot write a directory as a file");
    this.truncateInode(inode, 0);
    const bytes = new TextEncoder().encode(text);
    this.writeBytes(inode, 0, bytes);
    return bytes.length;
  }

  private ls(path: string): MemvfsDirEntry[] {
    const inode = this.mustResolve(path);
    if (inode.kind !== "Directory") fail("ENOTDIR", "path is not a directory");
    inode.accessedAt = this.tick();
    return [...inode.entries.entries()].map(([name, childId]) => {
      const child = this.inode(childId);
      return { name, inode: child.id, kind: child.kind };
    });
  }

  private stat(path: string): MemvfsFileStat {
    return this.statInode(this.mustResolve(path));
  }

  private rename(from: string, to: string): void {
    const source = this.mustResolve(from);
    const fromParent = this.resolveParent(from);
    const toParent = this.resolveParent(to);
    const replacement = toParent.parent.entries.get(toParent.name);
    if (replacement && this.isOpenInode(replacement)) {
      fail("EBUSY", "cannot replace an open file");
    }
    if (replacement) {
      const old = this.inode(replacement);
      if (old.kind === "Directory" && old.entries.size > 0) {
        fail("ENOTEMPTY", "target directory is not empty");
      }
      this.releaseBlocks(old);
      this.inodes.delete(old.id);
    }
    fromParent.parent.entries.delete(fromParent.name);
    toParent.parent.entries.set(toParent.name, source.id);
    const now = this.tick();
    fromParent.parent.modifiedAt = now;
    toParent.parent.modifiedAt = now;
    source.modifiedAt = now;
  }

  private unlink(path: string): void {
    const inode = this.mustResolve(path);
    if (inode.kind !== "File") fail("EISDIR", "cannot unlink a directory");
    if (this.isOpenInode(inode.id)) fail("EBUSY", "cannot unlink an open file");
    const { parent, name } = this.resolveParent(path);
    parent.entries.delete(name);
    parent.modifiedAt = this.tick();
    this.releaseBlocks(inode);
    this.inodes.delete(inode.id);
  }

  private rmdir(path: string): void {
    const inode = this.mustResolve(path);
    if (inode.id === ROOT_INODE) fail("EINVAL", "cannot remove root");
    if (inode.kind !== "Directory") fail("ENOTDIR", "path is not a directory");
    if (inode.entries.size > 0) fail("ENOTEMPTY", "directory is not empty");
    const { parent, name } = this.resolveParent(path);
    parent.entries.delete(name);
    parent.modifiedAt = this.tick();
    this.inodes.delete(inode.id);
  }

  private debug(kind: "Inodes" | "Blocks" | "Free" | "OpenFiles"): MemvfsResponse {
    if (kind === "Inodes") {
      const inodes: MemvfsInodeDebug[] = [...this.inodes.values()].map((inode) => ({
        inode: inode.id,
        kind: inode.kind,
        size: inode.size,
        mode: inode.mode,
        created_at: inode.createdAt,
        modified_at: inode.modifiedAt,
        accessed_at: inode.accessedAt,
        link_count: inode.linkCount,
        blocks: [...inode.blocks],
        entries: [...inode.entries.entries()].map(([name, childId]) => {
          const child = this.inode(childId);
          return { name, inode: child.id, kind: child.kind };
        }),
      }));
      return { type: "inodes", inodes };
    }
    if (kind === "Blocks") {
      const blocks: MemvfsBlockDebug[] = this.blocks.map((block, id) => ({
        id,
        used: !this.freeBlocks.includes(id),
        non_zero_bytes: block.filter((byte) => byte !== 0).length,
      }));
      return { type: "blocks", blocks };
    }
    if (kind === "Free") {
      return { type: "freeBlocks", blocks: [...this.freeBlocks].sort((a, b) => a - b) };
    }
    const files: MemvfsOpenFileDebug[] = [...this.openFiles.entries()].map(([fd, open]) => ({
      fd,
      inode: open.inode,
      offset: open.offset,
      read: open.read,
      write: open.write,
      append: open.append,
    }));
    return { type: "openFiles", files };
  }

  private statfs(): MemvfsStatFs {
    return {
      block_size: BLOCK_SIZE,
      total_blocks: TOTAL_BLOCKS,
      used_blocks: TOTAL_BLOCKS - this.freeBlocks.length,
      free_blocks: this.freeBlocks.length,
      inode_count: this.inodes.size,
      inode_capacity: INODE_CAPACITY,
    };
  }

  private createNode(path: string, kind: MemvfsFileKind, mode: number): Inode {
    if (this.inodes.size >= INODE_CAPACITY) fail("ENOSPC", "inode table is full");
    const { parent, name } = this.resolveParent(path);
    if (parent.entries.has(name)) fail("EEXIST", "path already exists");
    const now = this.tick();
    const inode: Inode = {
      id: this.nextInode,
      kind,
      mode,
      size: 0,
      createdAt: now,
      modifiedAt: now,
      accessedAt: now,
      linkCount: 1,
      blocks: [],
      entries: new Map(),
    };
    this.nextInode += 1;
    this.inodes.set(inode.id, inode);
    parent.entries.set(name, inode.id);
    parent.modifiedAt = this.tick();
    return inode;
  }

  private resolveParent(path: string): { parent: Inode; name: string } {
    const parts = pathOrThrow(path);
    const name = parts.at(-1);
    if (!name) fail("EINVAL", "root has no parent");
    let parent = this.inode(ROOT_INODE);
    for (const part of parts.slice(0, -1)) {
      const child = parent.entries.get(part);
      if (!child) fail("ENOENT", "parent path does not exist");
      parent = this.inode(child);
      if (parent.kind !== "Directory") fail("ENOTDIR", "parent component is not a directory");
    }
    return { parent, name };
  }

  private mustResolve(path: string): Inode {
    const inode = this.resolve(pathOrThrow(path));
    if (!inode) fail("ENOENT", "path does not exist");
    return inode;
  }

  private resolve(parts: string[]): Inode | null {
    let current = this.inode(ROOT_INODE);
    for (const part of parts) {
      if (current.kind !== "Directory") fail("ENOTDIR", "path component is not a directory");
      const child = current.entries.get(part);
      if (!child) return null;
      current = this.inode(child);
    }
    return current;
  }

  private inode(id: number): Inode {
    const inode = this.inodes.get(id);
    if (!inode) fail("ENOENT", "inode does not exist");
    return inode;
  }

  private openFile(fd: number): OpenFile {
    const open = this.openFiles.get(fd);
    if (!open) fail("EBADF", "invalid file descriptor");
    return open;
  }

  private statInode(inode: Inode): MemvfsFileStat {
    return {
      inode: inode.id,
      kind: inode.kind,
      size: inode.size,
      mode: inode.mode,
      created_at: inode.createdAt,
      modified_at: inode.modifiedAt,
      accessed_at: inode.accessedAt,
      block_count: inode.blocks.length,
      link_count: inode.linkCount,
    };
  }

  private readBytes(inode: Inode, offset: number, len: number): Uint8Array {
    const output = new Uint8Array(Math.max(0, len));
    for (let i = 0; i < output.length; i += 1) {
      const cursor = offset + i;
      const blockIndex = Math.floor(cursor / BLOCK_SIZE);
      const blockOffset = cursor % BLOCK_SIZE;
      const blockId = inode.blocks[blockIndex];
      if (blockId === undefined) break;
      output[i] = this.blocks[blockId]?.[blockOffset] ?? 0;
    }
    return output;
  }

  private writeBytes(inode: Inode, offset: number, bytes: Uint8Array): void {
    this.ensureBlocks(inode, offset + bytes.length);
    for (let i = 0; i < bytes.length; i += 1) {
      const cursor = offset + i;
      const blockIndex = Math.floor(cursor / BLOCK_SIZE);
      const blockOffset = cursor % BLOCK_SIZE;
      const blockId = inode.blocks[blockIndex];
      if (blockId === undefined) fail("EIO", "missing data block");
      const block = this.blocks[blockId];
      if (!block) fail("EIO", "invalid data block");
      block[blockOffset] = bytes[i] ?? 0;
    }
    inode.size = Math.max(inode.size, offset + bytes.length);
    inode.modifiedAt = this.tick();
    inode.accessedAt = inode.modifiedAt;
  }

  private truncateInode(inode: Inode, size: number): void {
    const keep = size === 0 ? 0 : Math.ceil(size / BLOCK_SIZE);
    this.ensureBlocks(inode, size);
    const removed = inode.blocks.splice(keep);
    for (const id of removed) {
      this.blocks[id]?.fill(0);
      if (!this.freeBlocks.includes(id)) this.freeBlocks.push(id);
    }
    if (size > 0 && size % BLOCK_SIZE !== 0) {
      const blockId = inode.blocks[keep - 1];
      const block = blockId === undefined ? undefined : this.blocks[blockId];
      block?.fill(0, size % BLOCK_SIZE);
    }
    inode.size = size;
    inode.modifiedAt = this.tick();
  }

  private ensureBlocks(inode: Inode, targetSize: number): void {
    const needed = targetSize === 0 ? 0 : Math.ceil(targetSize / BLOCK_SIZE);
    while (inode.blocks.length < needed) {
      const blockId = this.freeBlocks.pop();
      if (blockId === undefined) fail("ENOSPC", "data region is full");
      this.blocks[blockId]?.fill(0);
      inode.blocks.push(blockId);
    }
  }

  private releaseBlocks(inode: Inode): void {
    for (const id of inode.blocks) {
      this.blocks[id]?.fill(0);
      if (!this.freeBlocks.includes(id)) this.freeBlocks.push(id);
    }
    inode.blocks = [];
    inode.size = 0;
  }

  private isOpenInode(inode: number): boolean {
    return [...this.openFiles.values()].some((open) => open.inode === inode);
  }

  private tick(): number {
    this.clock += 1;
    return this.clock;
  }
}

class FsFailure extends Error {
  constructor(
    readonly code: string,
    message: string,
  ) {
    super(message);
  }
}

function pathOrThrow(path: string): string[] {
  if (!path.startsWith("/")) fail("EINVAL", "path must be absolute");
  const parts: string[] = [];
  for (const part of path.split("/")) {
    if (!part || part === ".") continue;
    if (part === ".." || part.includes("\\") || part.includes("\0")) {
      fail("EINVAL", "invalid path component");
    }
    parts.push(part);
  }
  return parts;
}

function fail(code: string, message: string): never {
  throw new FsFailure(code, message);
}

function ok(response: MemvfsResponse): MemvfsResult {
  return { ok: true, response };
}
