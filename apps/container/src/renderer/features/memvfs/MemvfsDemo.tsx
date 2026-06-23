import {
  createEffect,
  createMemo,
  createSignal,
  For,
  onCleanup,
  onMount,
  Show,
} from "solid-js";
import type {
  MemvfsApi,
  MemvfsBackendStatus,
  MemvfsBlockDebug,
  MemvfsDirEntry,
  MemvfsInodeDebug,
  MemvfsOpenFileDebug,
  MemvfsResponse,
  MemvfsStatFs,
} from "../../../shared/memvfs";
import styles from "./MemvfsDemo.module.css";

type TreeNode = MemvfsDirEntry & {
  path: string;
  children?: TreeNode[];
};

type OperationLog = Readonly<{
  id: number;
  command: string;
  ok: boolean;
  message: string;
}>;

type DebugSnapshot = Readonly<{
  statfs: MemvfsStatFs | null;
  inodes: MemvfsInodeDebug[];
  blocks: MemvfsBlockDebug[];
  openFiles: MemvfsOpenFileDebug[];
}>;

const emptyDebug: DebugSnapshot = {
  statfs: null,
  inodes: [],
  blocks: [],
  openFiles: [],
};

const BLOCK_CELL_MIN_WIDTH_PX = 36;
const BLOCK_GRID_GAP_PX = 4;
const BLOCK_ROW_HEIGHT_PX = 36;
const BLOCK_OVERSCAN_ROWS = 3;

export function MemvfsDemo(props: { memvfs: MemvfsApi }) {
  const [status, setStatus] = createSignal<MemvfsBackendStatus>({
    state: "starting",
    backend: "memory",
  });
  const [tree, setTree] = createSignal<TreeNode[]>([]);
  const [selectedPath, setSelectedPath] = createSignal("/docs/hello.txt");
  const [editorText, setEditorText] = createSignal("");
  const [newPath, setNewPath] = createSignal("/docs/session.txt");
  const [directoryPath, setDirectoryPath] = createSignal("/scratch");
  const [renameToPath, setRenameToPath] = createSignal("/docs/hello-renamed.txt");
  const [openFd, setOpenFd] = createSignal<number | null>(null);
  const [debug, setDebug] = createSignal<DebugSnapshot>(emptyDebug);
  const [logs, setLogs] = createSignal<OperationLog[]>([]);
  const [busy, setBusy] = createSignal(false);

  const selectedStat = createMemo(() => {
    const path = selectedPath();
    return debug().inodes.find((inode) => inode.entries.some((entry) => childPathFor(inode, entry) === path));
  });

  onMount(() => {
    void boot();
  });

  createEffect(() => {
    const path = selectedPath();
    setRenameToPath(path.endsWith(".txt") ? path.replace(/\.txt$/, "-renamed.txt") : `${path}-renamed`);
  });

  const boot = async () => {
    await withBusy(async () => {
      const nextStatus = await props.memvfs.start();
      setStatus(nextStatus);
      if (nextStatus.state === "running") {
        await seedDemo(props.memvfs);
        await refreshAll();
        await readSelected();
        pushLog("start", true, statusLabel(nextStatus));
      } else {
        pushLog("start", false, statusLabel(nextStatus));
      }
    });
  };

  const reset = async () => {
    await withBusy(async () => {
      const nextStatus = await props.memvfs.reset();
      setStatus(nextStatus);
      setOpenFd(null);
      setSelectedPath("/docs/hello.txt");
      if (nextStatus.state === "running") {
        await seedDemo(props.memvfs);
      }
      await refreshAll();
      await readSelected();
      pushLog("reset", nextStatus.state === "running", statusLabel(nextStatus));
    });
  };

  const stop = async () => {
    await withBusy(async () => {
      const nextStatus = await props.memvfs.stop();
      setStatus(nextStatus);
      setOpenFd(null);
      pushLog("stop", true, statusLabel(nextStatus));
    });
  };

  const mkdir = async () => {
    const path = directoryPath().trim();
    if (!path) return;
    await runCommand(`mkdir ${path}`, async () => {
      return props.memvfs.request({ type: "mkdir", path });
    });
  };

  const createFile = async () => {
    const path = newPath().trim();
    if (!path) return;
    setSelectedPath(path);
    await runCommand(`write ${path}`, async () => {
      return props.memvfs.request({
        type: "writeFile",
        path,
        text: "# memvfs scratch\n\nEdit this buffer and save it back into the in-memory file system.\n",
      });
    });
    await readSelected();
  };

  const saveSelected = async () => {
    const path = selectedPath();
    await runCommand(`write ${path}`, async () => {
      return props.memvfs.request({ type: "writeFile", path, text: editorText() });
    });
  };

  const readSelected = async () => {
    const path = selectedPath();
    const result = await props.memvfs.request({ type: "readFile", path });
    if (result.ok && result.response.type === "text") {
      setEditorText(result.response.text);
      pushLog(`read ${path}`, true, `${result.response.byteLength} bytes`);
    } else if (!result.ok) {
      pushLog(`read ${path}`, false, `${result.error.code}: ${result.error.message}`);
    }
  };

  const renameSelected = async () => {
    const from = selectedPath();
    const to = renameToPath().trim();
    if (!to) return;
    await runCommand(`mv ${from} ${to}`, async () => {
      return props.memvfs.request({ type: "rename", from, to });
    });
    setSelectedPath(to);
    await readSelected();
  };

  const unlinkSelected = async () => {
    const path = selectedPath();
    await runCommand(`rm ${path}`, async () => {
      return props.memvfs.request({ type: "unlink", path });
    });
    setSelectedPath("/notes.txt");
    await readSelected();
  };

  const truncateSelected = async () => {
    const path = selectedPath();
    await runCommand(`truncate ${path} --size 8`, async () => {
      return props.memvfs.request({ type: "truncate", path, size: 8 });
    });
    await readSelected();
  };

  const openSelected = async () => {
    const path = selectedPath();
    await withBusy(async () => {
      const result = await props.memvfs.request({
        type: "open",
        path,
        flags: {
          read: true,
          write: true,
          create: true,
          truncate: false,
          append: false,
        },
      });
      if (result.ok && result.response.type === "fd") {
        setOpenFd(result.response.fd);
        pushLog(`open ${path}`, true, `fd ${result.response.fd}`);
      } else if (!result.ok) {
        pushLog(`open ${path}`, false, `${result.error.code}: ${result.error.message}`);
      }
      await refreshAll();
    });
  };

  const appendViaFd = async () => {
    const fd = openFd();
    if (fd === null) return;
    await runCommand(`fd-write ${fd}`, async () => {
      return props.memvfs.request({ type: "write", fd, text: "\nappend via fd" });
    });
    await readSelected();
  };

  const closeFd = async () => {
    const fd = openFd();
    if (fd === null) return;
    await runCommand(`close ${fd}`, async () => {
      return props.memvfs.request({ type: "close", fd });
    });
    setOpenFd(null);
  };

  const selectNode = async (node: TreeNode) => {
    if (node.kind === "Directory") return;
    setSelectedPath(node.path);
    await readSelected();
  };

  const runCommand = async (
    command: string,
    fn: () => ReturnType<MemvfsApi["request"]>,
  ) => {
    await withBusy(async () => {
      const result = await fn();
      if (result.ok) {
        pushLog(command, true, summarizeResponse(result.response));
      } else {
        pushLog(command, false, `${result.error.code}: ${result.error.message}`);
      }
      await refreshAll();
    });
  };

  const refreshAll = async () => {
    const [treeResult, statfsResult, inodesResult, blocksResult, openFilesResult] = await Promise.all([
      buildTree(props.memvfs, "/"),
      props.memvfs.request({ type: "statfs" }),
      props.memvfs.request({ type: "debug", kind: "Inodes" }),
      props.memvfs.request({ type: "debug", kind: "Blocks" }),
      props.memvfs.request({ type: "debug", kind: "OpenFiles" }),
    ]);

    setTree(treeResult);
    setDebug({
      statfs: responseOf(statfsResult, "statfs")?.statfs ?? null,
      inodes: responseOf(inodesResult, "inodes")?.inodes ?? [],
      blocks: responseOf(blocksResult, "blocks")?.blocks ?? [],
      openFiles: responseOf(openFilesResult, "openFiles")?.files ?? [],
    });
  };

  const withBusy = async (fn: () => Promise<void>) => {
    setBusy(true);
    try {
      await fn();
    } finally {
      setBusy(false);
    }
  };

  const pushLog = (command: string, ok: boolean, message: string) => {
    setLogs((prev) => [
      { id: Date.now() + Math.random(), command, ok, message },
      ...prev.slice(0, 7),
    ]);
  };

  return (
    <main class={styles["page"]}>
      <header class={styles["toolbar"]}>
        <div>
          <h1>memvfs</h1>
        </div>
        <div class={styles["toolbarActions"]}>
          <span class={styles["status"]} data-state={status().state}>
            {statusLabel(status())}
          </span>
          <button type="button" onClick={boot} disabled={busy()}>
            Start
          </button>
          <button type="button" onClick={reset} disabled={busy()}>
            Reset
          </button>
          <button type="button" onClick={stop} disabled={busy()}>
            Stop
          </button>
        </div>
      </header>

      <section class={styles["workspace"]}>
        <aside class={styles["treePane"]} aria-label="memvfs file tree">
          <div class={styles["paneTitle"]}>Index</div>
          <div class={styles["treeRoot"]}>
            <TreeNodeList nodes={tree()} selectedPath={selectedPath()} onSelect={selectNode} />
          </div>
        </aside>

        <section class={styles["editorPane"]}>
          <div class={styles["editorHeader"]}>
            <div>
              <span class={styles["eyebrow"]}>File Buffer</span>
              <h2>{selectedPath()}</h2>
            </div>
            <div class={styles["editorActions"]}>
              <button type="button" onClick={readSelected} disabled={busy()}>
                Read
              </button>
              <button type="button" onClick={saveSelected} disabled={busy()}>
                Save
              </button>
              <button type="button" onClick={truncateSelected} disabled={busy()}>
                Truncate
              </button>
            </div>
          </div>
          <textarea
            class={styles["editor"]}
            value={editorText()}
            onInput={(event) => setEditorText(event.currentTarget.value)}
            aria-label="memvfs file content"
            spellcheck={false}
          />
        </section>

        <aside class={styles["opsPane"]}>
          <div class={styles["paneTitle"]}>Operations</div>
          <label class={styles["field"]}>
            New file
            <input value={newPath()} onInput={(event) => setNewPath(event.currentTarget.value)} />
          </label>
          <button type="button" onClick={createFile} disabled={busy()}>
            Write File
          </button>

          <label class={styles["field"]}>
            Directory
            <input
              value={directoryPath()}
              onInput={(event) => setDirectoryPath(event.currentTarget.value)}
            />
          </label>
          <button type="button" onClick={mkdir} disabled={busy()}>
            Mkdir
          </button>

          <label class={styles["field"]}>
            Rename selected
            <input
              value={renameToPath()}
              onInput={(event) => setRenameToPath(event.currentTarget.value)}
            />
          </label>
          <div class={styles["buttonRow"]}>
            <button type="button" onClick={renameSelected} disabled={busy()}>
              Rename
            </button>
            <button type="button" onClick={unlinkSelected} disabled={busy()}>
              Unlink
            </button>
          </div>

          <div class={styles["fdBox"]}>
            <span>Open fd: {openFd() ?? "none"}</span>
            <div class={styles["buttonRow"]}>
              <button type="button" onClick={openSelected} disabled={busy()}>
                Open
              </button>
              <button type="button" onClick={appendViaFd} disabled={busy() || openFd() === null}>
                FD Write
              </button>
              <button type="button" onClick={closeFd} disabled={busy() || openFd() === null}>
                Close
              </button>
            </div>
          </div>
        </aside>
      </section>

      <section class={styles["debugGrid"]}>
        <DebugStatFs statfs={debug().statfs} />
        <DebugInodes inodes={debug().inodes} selectedStat={selectedStat()} />
        <DebugBlocks blocks={debug().blocks} />
        <DebugOpenFiles files={debug().openFiles} />
        <LogPanel logs={logs()} />
      </section>
    </main>
  );
}

function TreeNodeList(props: {
  nodes: TreeNode[];
  selectedPath: string;
  onSelect: (node: TreeNode) => void | Promise<void>;
}) {
  return (
    <ul class={styles["treeList"]}>
      <For each={props.nodes}>
        {(node) => (
          <li>
            <button
              type="button"
              class={styles["treeNode"]}
              data-selected={node.path === props.selectedPath}
              onClick={() => void props.onSelect(node)}
            >
              <span>{node.kind === "Directory" ? "dir" : "file"}</span>
              {node.path}
            </button>
            <Show when={node.children && node.children.length > 0}>
              <TreeNodeList
                nodes={node.children ?? []}
                selectedPath={props.selectedPath}
                onSelect={props.onSelect}
              />
            </Show>
          </li>
        )}
      </For>
    </ul>
  );
}

function DebugStatFs(props: { statfs: MemvfsStatFs | null }) {
  const usedPercent = () => {
    if (!props.statfs || props.statfs.total_blocks === 0) return 0;
    return Math.round((props.statfs.used_blocks / props.statfs.total_blocks) * 100);
  };

  return (
    <article class={styles["debugPanel"]}>
      <h3>statfs</h3>
      <Show when={props.statfs} fallback={<p>no statfs</p>}>
        {(statfs) => (
          <>
            <div class={styles["meter"]}>
              <span style={{ width: `${usedPercent()}%` }} />
            </div>
            <dl class={styles["kv"]}>
              <div>
                <dt>block size</dt>
                <dd>{statfs().block_size}</dd>
              </div>
              <div>
                <dt>blocks</dt>
                <dd>
                  {statfs().used_blocks}/{statfs().total_blocks}
                </dd>
              </div>
              <div>
                <dt>inodes</dt>
                <dd>
                  {statfs().inode_count}/{statfs().inode_capacity}
                </dd>
              </div>
            </dl>
          </>
        )}
      </Show>
    </article>
  );
}

function DebugInodes(props: {
  inodes: MemvfsInodeDebug[];
  selectedStat: MemvfsInodeDebug | undefined;
}) {
  return (
    <article class={styles["debugPanel"]}>
      <h3>inodes</h3>
      <div class={styles["scrollTable"]}>
        <table>
          <thead>
            <tr>
              <th>id</th>
              <th>kind</th>
              <th>size</th>
              <th>blocks</th>
            </tr>
          </thead>
          <tbody>
            <For each={props.inodes}>
              {(inode) => (
                <tr data-selected={props.selectedStat?.inode === inode.inode}>
                  <td>{inode.inode}</td>
                  <td>{inode.kind}</td>
                  <td>{inode.size}</td>
                  <td>{inode.blocks.join(",") || "-"}</td>
                </tr>
              )}
            </For>
          </tbody>
        </table>
      </div>
    </article>
  );
}

function DebugBlocks(props: { blocks: MemvfsBlockDebug[] }) {
  let viewportRef: HTMLDivElement | undefined;
  const [scrollTop, setScrollTop] = createSignal(0);
  const [viewportHeight, setViewportHeight] = createSignal(160);
  const [columnCount, setColumnCount] = createSignal(4);

  const rowCount = createMemo(() => Math.ceil(props.blocks.length / columnCount()));
  const firstRow = createMemo(() =>
    Math.max(0, Math.floor(scrollTop() / BLOCK_ROW_HEIGHT_PX) - BLOCK_OVERSCAN_ROWS),
  );
  const visibleRowCount = createMemo(
    () => Math.ceil(viewportHeight() / BLOCK_ROW_HEIGHT_PX) + BLOCK_OVERSCAN_ROWS * 2,
  );
  const visibleRows = createMemo(() => {
    const columns = columnCount();
    const start = firstRow();
    const end = Math.min(rowCount(), start + visibleRowCount());
    const rows: Array<Readonly<{ index: number; blocks: MemvfsBlockDebug[] }>> = [];
    for (let index = start; index < end; index += 1) {
      rows.push({
        index,
        blocks: props.blocks.slice(index * columns, index * columns + columns),
      });
    }
    return rows;
  });

  onMount(() => {
    const node = viewportRef;
    if (!node) return;

    const measure = () => {
      setViewportHeight(node.clientHeight || 160);
      setColumnCount(
        Math.max(
          1,
          Math.floor((node.clientWidth + BLOCK_GRID_GAP_PX) / (BLOCK_CELL_MIN_WIDTH_PX + BLOCK_GRID_GAP_PX)),
        ),
      );
    };

    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(node);
    onCleanup(() => observer.disconnect());
  });

  return (
    <article class={styles["debugPanel"]}>
      <div class={styles["panelHeading"]}>
        <h3>data blocks</h3>
        <span>{props.blocks.length}</span>
      </div>
      <div
        ref={viewportRef}
        class={styles["blockVirtualViewport"]}
        onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
      >
        <div
          class={styles["blockVirtualCanvas"]}
          style={{ height: `${rowCount() * BLOCK_ROW_HEIGHT_PX}px` }}
        >
          <div
            class={styles["blockVirtualRows"]}
            style={{
              "--block-columns": String(columnCount()),
              transform: `translateY(${firstRow() * BLOCK_ROW_HEIGHT_PX}px)`,
            }}
          >
            <For each={visibleRows()}>
              {(row) => (
                <div class={styles["blockVirtualRow"]} data-row={row.index}>
                  <For each={row.blocks}>
                    {(block) => (
                      <span
                        class={styles["block"]}
                        data-used={block.used}
                        title={`block ${block.id}: ${block.non_zero_bytes} non-zero bytes`}
                      >
                        {block.id}
                      </span>
                    )}
                  </For>
                </div>
              )}
            </For>
          </div>
        </div>
      </div>
    </article>
  );
}

function DebugOpenFiles(props: { files: MemvfsOpenFileDebug[] }) {
  return (
    <article class={styles["debugPanel"]}>
      <h3>open files</h3>
      <Show when={props.files.length > 0} fallback={<p>No file descriptors are open.</p>}>
        <For each={props.files}>
          {(file) => (
            <div class={styles["fdLine"]}>
              <span>fd {file.fd}</span>
              <span>inode {file.inode}</span>
              <span>offset {file.offset}</span>
            </div>
          )}
        </For>
      </Show>
    </article>
  );
}

function LogPanel(props: { logs: OperationLog[] }) {
  return (
    <article class={`${styles["debugPanel"]} ${styles["logPanel"]}`}>
      <h3>trace</h3>
      <For each={props.logs} fallback={<p>No operations yet.</p>}>
        {(log) => (
          <div class={styles["logLine"]} data-ok={log.ok}>
            <code>{log.command}</code>
            <span>{log.message}</span>
          </div>
        )}
      </For>
    </article>
  );
}

async function buildTree(memvfs: MemvfsApi, path: string): Promise<TreeNode[]> {
  const result = await memvfs.request({ type: "ls", path });
  if (!result.ok || result.response.type !== "dirEntries") return [];

  const nodes = await Promise.all(
    result.response.entries
      .slice()
      .sort((a, b) => {
        if (a.kind !== b.kind) return a.kind === "Directory" ? -1 : 1;
        return a.name.localeCompare(b.name);
      })
      .map(async (entry) => {
        const childPath = joinMemPath(path, entry.name);
        const node: TreeNode = { ...entry, path: childPath };
        if (entry.kind === "Directory") {
          node.children = await buildTree(memvfs, childPath);
        }
        return node;
      }),
  );
  return nodes;
}

async function seedDemo(memvfs: MemvfsApi): Promise<void> {
  const root = await memvfs.request({ type: "ls", path: "/" });
  const hasDocs =
    root.ok &&
    root.response.type === "dirEntries" &&
    root.response.entries.some((entry) => entry.kind === "Directory" && entry.name === "docs");
  if (!hasDocs) {
    await memvfs.request({ type: "mkdir", path: "/docs" });
  }

  const hello = await memvfs.request({ type: "readFile", path: "/docs/hello.txt" });
  if (!hello.ok) {
    await memvfs.request({
      type: "writeFile",
      path: "/docs/hello.txt",
      text: "hello from memvfs\n\nThis buffer is stored in fixed-size memory blocks.",
    });
  }

  const notes = await memvfs.request({ type: "readFile", path: "/notes.txt" });
  if (!notes.ok) {
    await memvfs.request({
      type: "writeFile",
      path: "/notes.txt",
      text: "Index region: inode table and directory entries.\nData region: fixed-size blocks and recycled free space.",
    });
  }
}

function joinMemPath(parent: string, name: string): string {
  return parent === "/" ? `/${name}` : `${parent}/${name}`;
}

function summarizeResponse(response: MemvfsResponse): string {
  switch (response.type) {
    case "count":
      return `${response.count} bytes`;
    case "fd":
      return `fd ${response.fd}`;
    case "offset":
      return `offset ${response.offset}`;
    case "dirEntries":
      return `${response.entries.length} entries`;
    case "stat":
      return `${response.stat.kind} ${response.stat.size} bytes`;
    case "text":
      return `${response.byteLength} bytes`;
    case "unit":
      return "ok";
    default:
      return response.type;
  }
}

function statusLabel(status: MemvfsBackendStatus): string {
  if (status.state === "running") {
    return status.backend === "daemon"
      ? `daemon ${status.address ?? ""}`.trim()
      : "memory backend";
  }
  if (status.state === "unavailable") return status.reason;
  return `${status.backend} ${status.state}`;
}

function responseOf<T extends MemvfsResponse["type"]>(
  result: Awaited<ReturnType<MemvfsApi["request"]>>,
  type: T,
): Extract<MemvfsResponse, { type: T }> | null {
  if (!result.ok || result.response.type !== type) return null;
  return result.response as Extract<MemvfsResponse, { type: T }>;
}

function childPathFor(parent: MemvfsInodeDebug, entry: MemvfsDirEntry): string {
  if (parent.inode === 1) return `/${entry.name}`;
  return entry.name;
}
