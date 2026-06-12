import { createSignal, createResource, For, Show } from "solid-js";
import type { FileEntry } from "../../preload/index";
import styles from "./FileTree.module.css";

interface FileTreeProps {
  workspaceRoot: string;
  onSelect: (entry: FileEntry) => void;
}

async function fetchDirectory(path: string): Promise<FileEntry[]> {
  try {
    return await window.arborAPI.fs.listDirectory(path);
  } catch {
    return [];
  }
}

export function FileTree(props: FileTreeProps) {
  const [entries] = createResource(
    () => props.workspaceRoot,
    fetchDirectory,
  );

  const [expanded, setExpanded] = createSignal<Set<string>>(new Set());
  const [selected, setSelected] = createSignal<string | null>(null);

  const toggleExpand = (path: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  };

  const handleClick = (entry: FileEntry) => {
    setSelected(entry.path);
    props.onSelect(entry);
    if (entry.isDirectory) {
      toggleExpand(entry.path);
    }
  };

  const handleSwitchWorkspace = async () => {
    const dir = await window.arborAPI.dialog.selectDirectory();
    if (dir) {
      window.location.reload();
    }
  };

  return (
    <div class={styles.tree}>
      <div class={styles.header}>
        <span class={styles.headerTitle}>Arbor</span>
        <button
          class={styles.switchBtn}
          onClick={handleSwitchWorkspace}
          title="切换工作区"
        >
          📂
        </button>
      </div>
      <div class={styles.nodes}>
        <Show when={entries()} fallback={<div class={styles.empty}>加载中...</div>}>
          <TreeNodeList
            entries={entries() ?? []}
            expanded={expanded()}
            selected={selected()}
            depth={0}
            onClick={handleClick}
          />
        </Show>
      </div>
    </div>
  );
}

// --- recursive tree nodes ---
function TreeNodeList(props: {
  entries: FileEntry[];
  expanded: Set<string>;
  selected: string | null;
  depth: number;
  onClick: (entry: FileEntry) => void;
}) {
  // Sort: directories first, then files, alphabetically
  const sorted = [...props.entries].sort((a, b) => {
    if (a.isDirectory !== b.isDirectory) return a.isDirectory ? -1 : 1;
    return a.name.localeCompare(b.name);
  });

  return (
    <For each={sorted}>
      {(entry) => <TreeNode entry={entry} {...props} />}
    </For>
  );
}

function TreeNode(props: {
  entry: FileEntry;
  expanded: Set<string>;
  selected: string | null;
  depth: number;
  onClick: (entry: FileEntry) => void;
}) {
  const [children] = createResource(
    () =>
      props.entry.isDirectory && props.expanded.has(props.entry.path)
        ? props.entry.path
        : null,
    fetchDirectory,
  );

  const isExpanded = () => props.expanded.has(props.entry.path);
  const isSelected = () => props.selected === props.entry.path;
  const isDir = props.entry.isDirectory;

  const icon = () => {
    if (!isDir) return "📄";
    return isExpanded() ? "📂" : "📁";
  };

  return (
    <>
      <button
        class={`${styles.node} ${isSelected() ? styles.nodeSelected : ""}`}
        style={{ "padding-left": `${0.75 + props.depth * 1.25}rem` }}
        onClick={() => props.onClick(props.entry)}
      >
        <span class={styles.icon}>{icon()}</span>
        <span class={styles.name}>{props.entry.name}</span>
      </button>
      <Show when={isDir && isExpanded()}>
        <TreeNodeList
          entries={children() ?? []}
          expanded={props.expanded}
          selected={props.selected}
          depth={props.depth + 1}
          onClick={props.onClick}
        />
      </Show>
    </>
  );
}
