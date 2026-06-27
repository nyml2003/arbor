import { createResource, createSignal, For, Show } from "solid-js";
import type { ManageTask } from "../../../shared/manage";
import type { PlatformAdapter } from "../../platform/types";
import styles from "./ManagePage.module.css";

export function ManagePage(props: { adapter: PlatformAdapter }) {
  const [title, setTitle] = createSignal("");
  const [editingId, setEditingId] = createSignal<string | null>(null);
  const [editingTitle, setEditingTitle] = createSignal("");
  const [status, setStatus] = createSignal<string | null>(null);
  const [reloadToken, setReloadToken] = createSignal(0);
  const [tasks] = createResource(reloadToken, async () => props.adapter.manage.list());

  const reload = () => setReloadToken((value) => value + 1);

  const handleCreate = async () => {
    const result = await props.adapter.manage.create(title());
    if (!result.ok) {
      setStatus(result.reason);
      return;
    }
    setTitle("");
    setStatus("Task created.");
    reload();
  };

  const handleSaveTitle = async (task: ManageTask) => {
    const result = await props.adapter.manage.update(task.id, editingTitle());
    if (!result.ok) {
      setStatus(result.reason);
      return;
    }
    setEditingId(null);
    setEditingTitle("");
    setStatus("Task updated.");
    reload();
  };

  const handleToggle = async (task: ManageTask) => {
    const result = task.status === "done"
      ? await props.adapter.manage.restore(task.id)
      : await props.adapter.manage.complete(task.id);
    if (!result.ok) {
      setStatus(result.reason);
      return;
    }
    setStatus(task.status === "done" ? "Task restored." : "Task completed.");
    reload();
  };

  const taskList = () => {
    const result = tasks();
    if (result?.ok) return result.tasks;
    return [];
  };

  const loadError = () => {
    const result = tasks();
    if (result !== undefined && !result.ok) return result.reason;
    return null;
  };

  return (
    <section class={styles["page"]}>
      <header class={styles["header"]}>
        <div>
          <h1>Manage</h1>
          <p>Tasks are stored in the Arbor workspace and shared by the CLI and container.</p>
        </div>
        <span class={styles["runtime"]}>{props.adapter.mode}</span>
      </header>

      <div class={styles["composer"]}>
        <input
          aria-label="New task title"
          value={title()}
          placeholder="Add a task"
          onInput={(event) => setTitle(event.currentTarget.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              void handleCreate();
            }
          }}
        />
        <button type="button" onClick={() => void handleCreate()}>
          Create
        </button>
      </div>

      <Show when={status()}>
        {(message) => <p class={styles["status"]}>{message()}</p>}
      </Show>
      <Show when={loadError()}>
        {(message) => <p class={styles["error"]}>{message()}</p>}
      </Show>

      <div class={styles["list"]}>
        <Show
          when={!tasks.loading}
          fallback={<p class={styles["empty"]}>Loading tasks...</p>}
        >
          <Show
            when={taskList().length > 0}
            fallback={<p class={styles["empty"]}>No tasks yet.</p>}
          >
            <For each={taskList()}>
              {(task) => (
                <article class={styles["task"]} data-status={task.status}>
                  <button
                    type="button"
                    class={styles["check"]}
                    aria-label={task.status === "done" ? `Restore ${task.title}` : `Complete ${task.title}`}
                    onClick={() => void handleToggle(task)}
                  >
                    {task.status === "done" ? "✓" : ""}
                  </button>

                  <div class={styles["taskBody"]}>
                    <Show
                      when={editingId() === task.id}
                      fallback={
                        <>
                          <h2>{task.title}</h2>
                          <p>{task.id}</p>
                        </>
                      }
                    >
                      <input
                        aria-label={`Edit ${task.title}`}
                        value={editingTitle()}
                        onInput={(event) => setEditingTitle(event.currentTarget.value)}
                        onKeyDown={(event) => {
                          if (event.key === "Enter") {
                            void handleSaveTitle(task);
                          }
                          if (event.key === "Escape") {
                            setEditingId(null);
                            setEditingTitle("");
                          }
                        }}
                      />
                    </Show>
                  </div>

                  <div class={styles["actions"]}>
                    <Show
                      when={editingId() === task.id}
                      fallback={
                        <button
                          type="button"
                          onClick={() => {
                            setEditingId(task.id);
                            setEditingTitle(task.title);
                          }}
                        >
                          Edit
                        </button>
                      }
                    >
                      <button type="button" onClick={() => void handleSaveTitle(task)}>
                        Save
                      </button>
                    </Show>
                  </div>
                </article>
              )}
            </For>
          </Show>
        </Show>
      </div>
    </section>
  );
}
