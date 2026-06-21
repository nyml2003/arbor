import { createSignal, Show, createResource, onMount } from "solid-js";
import { ArborLayout } from "./layouts/ArborLayout";
import { FileTree } from "./components/FileTree";
import { ResumePrintPage, ResumeView } from "./features/resume/ResumeView";
import { createElectronAdapter } from "./platform/electronAdapter";
import { isResumePrintRoute, routeFromEntry, routeToWebPath } from "./platform/shared";
import type { PlatformAdapter } from "./platform/types";
import type { FileEntry } from "./types";
import styles from "./App.module.css";

async function fetchText(adapter: PlatformAdapter, path: string | null): Promise<string> {
  if (!path) return "";
  try {
    return await adapter.readText(path);
  } catch {
    return "(无法读取文件)";
  }
}

function filePathFromRoute(route: string): string | null {
  return route.startsWith("file:") ? route.slice("file:".length) : null;
}

function HomePage(props: { adapter: PlatformAdapter }) {
  return (
    <div class={styles["home"]}>
      <h1>Arbor Show</h1>
      <p>这些页面共用同一套 Solid 页面组件，并通过 platform adapter 在 Electron 和 Web 中读取数据。</p>
      <div class={styles["capabilities"]}>
        <span>Runtime: {props.adapter.mode}</span>
        <span>Workspace files: {props.adapter.capabilities.workspaceFiles.status}</span>
        <span>Static pages: {props.adapter.capabilities.staticPages.status}</span>
      </div>
    </div>
  );
}

function UnsupportedFilePage(props: { adapter: PlatformAdapter; path: string }) {
  return (
    <div class={styles["placeholder"]}>
      <div class={styles["logo"]}>⚠</div>
      <h2>当前运行时不支持读取此文件</h2>
      <p>{props.path}</p>
      <Show when={props.adapter.capabilities.workspaceFiles.reason}>
        <p>{props.adapter.capabilities.workspaceFiles.reason}</p>
      </Show>
    </div>
  );
}

export function ArborApp(props: { adapter: PlatformAdapter }) {
  const [workspaceRoot, setWorkspaceRoot] = createSignal<string | null>(null);
  const [route, setRoute] = createSignal(props.adapter.getInitialRoute());
  const [autoPrint, setAutoPrint] = createSignal(false);

  onMount(() => {
    props.adapter.getDefaultWorkspace().then((root) => {
      setWorkspaceRoot(root);
    });
  });

  const [fileContent] = createResource(
    () => {
      const path = filePathFromRoute(route());
      return path;
    },
    (path) => fetchText(props.adapter, path),
  );

  const navigateToRoute = (nextRoute: string, options?: { autoPrint?: boolean }) => {
    setRoute(nextRoute);
    if (props.adapter.mode === "web") {
      window.history.pushState(null, "", routeToWebPath(nextRoute));
    }
    setAutoPrint(options?.autoPrint === true);
  };

  const handleSelect = (entry: FileEntry) => {
    navigateToRoute(routeFromEntry(entry));
  };

  return (
    <Show
      when={isResumePrintRoute(route())}
      fallback={
        <ArborLayout
          sidebar={
            <Show when={workspaceRoot()}>
              <FileTree
                adapter={props.adapter}
                workspaceRoot={workspaceRoot() ?? ""}
                onSelect={handleSelect}
              />
            </Show>
          }
        >
          <Show
            when={route()}
            fallback={
              <div class={styles["placeholder"]}>
                <div class={styles["logo"]}>🌳</div>
                <h2>Arbor</h2>
                <p>选择左侧页面或文件</p>
              </div>
            }
          >
            <Show when={route() === "show/home"}>
              <HomePage adapter={props.adapter} />
            </Show>
            <Show when={route() === "show/resume"}>
              <ResumeView
                adapter={props.adapter}
                onOpenPrint={() => navigateToRoute("show/resume/print")}
                onPrint={() => navigateToRoute("show/resume/print", { autoPrint: true })}
              />
            </Show>
            <Show when={filePathFromRoute(route())} keyed>
              {(path) => (
                <Show
                  when={props.adapter.capabilities.workspaceFiles.status === "supported"}
                  fallback={<UnsupportedFilePage adapter={props.adapter} path={path} />}
                >
                  <div class={styles["viewer"]}>
                    <div class={styles["viewerHeader"]}>{path}</div>
                    <pre class={styles["viewerContent"]}>{fileContent()}</pre>
                  </div>
                </Show>
              )}
            </Show>
            <Show when={route() !== "show/home" && route() !== "show/resume" && !filePathFromRoute(route())}>
              <UnsupportedFilePage adapter={props.adapter} path={route()} />
            </Show>
          </Show>
        </ArborLayout>
      }
    >
      <ResumePrintPage
        adapter={props.adapter}
        autoPrint={autoPrint()}
        onAutoPrintDone={() => setAutoPrint(false)}
        onBack={() => navigateToRoute("show/resume")}
        onPrint={() => window.print()}
      />
    </Show>
  );
}

export function App() {
  return <ArborApp adapter={createElectronAdapter()} />;
}
