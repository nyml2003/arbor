import { createSignal, Show, createResource } from "solid-js";
import { ArborLayout } from "./layouts/ArborLayout";
import { FileTree } from "./components/FileTree";
import type { FileEntry } from "../preload/index";
import styles from "./App.module.css";

async function fetchText(path: string | null): Promise<string> {
  if (!path) return "";
  try {
    return await window.arborAPI.fs.readText(path);
  } catch {
    return "(无法读取文件)";
  }
}

export function App() {
  const [workspaceRoot, setWorkspaceRoot] = createSignal<string>("");
  const [selectedEntry, setSelectedEntry] = createSignal<FileEntry | null>(null);

  // Load default workspace on mount
  window.arborAPI.getDefaultWorkspace().then((root: string) => {
    setWorkspaceRoot(root);
  });

  const [fileContent] = createResource(
    () => {
      const entry = selectedEntry();
      if (!entry || entry.isDirectory) return null;
      return entry.path;
    },
    fetchText,
  );

  const handleSelect = (entry: FileEntry) => {
    setSelectedEntry(entry);
  };

  return (
    <ArborLayout
      sidebar={
        <Show when={workspaceRoot()}>
          <FileTree
            workspaceRoot={workspaceRoot()}
            onSelect={handleSelect}
          />
        </Show>
      }
    >
      <Show
        when={selectedEntry()}
        fallback={
          <div class={styles.placeholder}>
            <div class={styles.logo}>🌳</div>
            <h2>Arbor</h2>
            <p>从左侧树中选择文件或目录</p>
          </div>
        }
      >
        <Show
          when={!selectedEntry()!.isDirectory}
          fallback={
            <div class={styles.placeholder}>
              <div class={styles.logo}>📁</div>
              <h2>{selectedEntry()!.name}</h2>
              <p>选择一个文件查看内容</p>
            </div>
          }
        >
          <div class={styles.viewer}>
            <div class={styles.viewerHeader}>{selectedEntry()!.name}</div>
            <pre class={styles.viewerContent}>{fileContent()}</pre>
          </div>
        </Show>
      </Show>
    </ArborLayout>
  );
}
