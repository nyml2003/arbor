import { createSignal, createResource, For, Show } from "solid-js";
import type { PlatformAdapter } from "../platform/types";
import type { FileEntry } from "../types";
import styles from "./FileTree.module.css";

interface FileTreeProps {
  adapter: PlatformAdapter;
  workspaceRoot: string;
  onSelect: (entry: FileEntry) => void;
}

export function FileTree(props: FileTreeProps) {
  const [entries] = createResource(
    () => props.workspaceRoot,
    (path) => props.adapter.listDirectory(path),
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
    const dir = await props.adapter.selectDirectory();
    if (dir) {
      setExpanded(new Set<string>());
    }
  };

  return (
    <div class={styles["tree"]}>
      <div class={styles["header"]}>
        <span class={styles["headerTitle"]}>Arbor</span>
        <Show when={props.adapter.capabilities.workspaceFiles.status === "supported"}>
          <button
            class={styles["switchBtn"]}
            onClick={handleSwitchWorkspace}
            title="切换工作区"
          >
            📂
          </button>
        </Show>
      </div>
      <div class={styles["nodes"]}>
        <Show when={entries()} fallback={<div class={styles["empty"]}>加载中...</div>}>
          <TreeNodeList
            entries={entries() ?? []}
            expanded={expanded()}
            selected={selected()}
            depth={0}
            onClick={handleClick}
            adapter={props.adapter}
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
  adapter: PlatformAdapter;
}) {
  const sortedEntries = () => [...props.entries].sort((a, b) => {
    if (a.isDirectory !== b.isDirectory) return a.isDirectory ? -1 : 1;
    return a.name.localeCompare(b.name);
  });

  return (
    <For each={sortedEntries()}>
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
  adapter: PlatformAdapter;
}) {
  const [children] = createResource(
    () =>
      props.entry.isDirectory && props.expanded.has(props.entry.path)
        ? props.entry.path
        : null,
    (path) => props.adapter.listDirectory(path),
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
        class={`${styles["node"]} ${isSelected() ? styles["nodeSelected"] : ""}`}
        style={{ "padding-left": `${0.75 + props.depth * 1.25}rem` }}
        onClick={() => props.onClick(props.entry)}
      >
        <span class={styles["icon"]}>{icon()}</span>
        <span class={styles["name"]}>{props.entry.name}</span>
      </button>
      <Show when={isDir && isExpanded()}>
        <TreeNodeList
          entries={children() ?? []}
          expanded={props.expanded}
          selected={props.selected}
          depth={props.depth + 1}
          onClick={props.onClick}
          adapter={props.adapter}
        />
      </Show>
    </>
  );
}
