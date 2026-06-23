import { For, Show, createResource, createSignal } from "solid-js";
import type { JSX } from "solid-js";
import { createStore } from "solid-js/store";
import {
  cloneResumeDocument,
  parseResumeValue,
  serializeResumeDocument,
} from "./resumeData";
import { ResumeDocumentView } from "./ResumeView";
import { resolveResumeThemeId, resumeThemeOptions } from "./themes";
import type { PlatformAdapter } from "../../platform/types";
import type {
  ResumeContact,
  ResumeDocument,
  ResumeEducation,
  ResumeExperience,
  ResumeProject,
  ResumeThemeId,
} from "./types";
import styles from "./ResumeEditor.module.css";

interface ResumeEditorPageProps {
  adapter: PlatformAdapter;
  onBack: () => void;
  onOpenPrint: () => void;
}

type EditorStatus =
  | { kind: "idle"; message: string }
  | { kind: "success"; message: string }
  | { kind: "error"; message: string };

async function loadResume(adapter: PlatformAdapter): Promise<ResumeDocument> {
  const raw = await adapter.readResumeJson();
  const parsed = parseResumeValue(raw);
  if (!parsed.ok) {
    throw new Error(parsed.message);
  }
  return parsed.data;
}

function toLines(items: string[] | undefined): string {
  return (items ?? []).join("\n");
}

function fromLines(value: string): string[] {
  return value
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
}

function fromTags(value: string): string[] {
  return value
    .split(/[,，\n]/)
    .map((tag) => tag.trim())
    .filter(Boolean);
}

function moveItem<T>(items: T[], index: number, direction: -1 | 1): T[] {
  const nextIndex = index + direction;
  if (nextIndex < 0 || nextIndex >= items.length) return items;
  const next = [...items];
  const current = next[index];
  const target = next[nextIndex];
  if (current === undefined || target === undefined) return items;
  next[index] = target;
  next[nextIndex] = current;
  return next;
}

function createContact(): ResumeContact {
  return { label: "Email", value: "" };
}

function createEducation(): ResumeEducation {
  return {
    school: "",
    degree: "",
    major: "",
    range: ["", ""],
    tags: [],
    highlights: [],
  };
}

function createExperience(): ResumeExperience {
  return {
    company: "",
    role: "",
    range: ["", ""],
    bullets: [],
  };
}

function createProject(): ResumeProject {
  return {
    title: "",
    subtitle: "",
    tags: [],
    bullets: [],
  };
}

function downloadResumeJson(resume: ResumeDocument): void {
  const blob = new Blob([serializeResumeDocument(resume)], {
    type: "application/json;charset=utf-8",
  });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = "resume.json";
  document.body.append(link);
  link.click();
  link.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 0);
}

function Field(props: { label: string; children: JSX.Element }) {
  return (
    <label class={styles["field"]}>
      <span>{props.label}</span>
      {props.children}
    </label>
  );
}

function TagEditor(props: {
  label: string;
  tags: string[] | undefined;
  onChange: (tags: string[]) => void;
}) {
  const [nextTag, setNextTag] = createSignal("");
  const tags = () => props.tags ?? [];

  const addTag = () => {
    const additions = fromTags(nextTag()).filter((tag) => !tags().includes(tag));
    if (additions.length === 0) return;
    props.onChange([...tags(), ...additions]);
    setNextTag("");
  };

  const removeTag = (index: number) => {
    props.onChange(tags().filter((_, itemIndex) => itemIndex !== index));
  };

  return (
    <div class={styles["field"]}>
      <span>{props.label}</span>
      <div class={styles["tagEditor"]}>
        <Show
          when={tags().length > 0}
          fallback={<div class={styles["emptyTags"]}>暂无标签</div>}
        >
          <div class={styles["tagList"]}>
            <For each={tags()}>
              {(tag, index) => (
                <button
                  type="button"
                  class={styles["tagChip"]}
                  aria-label={`删除标签 ${tag}`}
                  onClick={() => removeTag(index())}
                >
                  {tag}
                  <span>×</span>
                </button>
              )}
            </For>
          </div>
        </Show>
        <div class={styles["tagInputRow"]}>
          <input
            value={nextTag()}
            placeholder="输入标签后回车"
            onInput={(event) => setNextTag(event.currentTarget.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                addTag();
              }
            }}
          />
          <button type="button" class={styles["ghostButton"]} onClick={addTag}>
            添加标签
          </button>
        </div>
      </div>
    </div>
  );
}

function Panel(props: { title: string; children: JSX.Element }) {
  return (
    <section class={styles["panel"]}>
      <h2>{props.title}</h2>
      <div class={styles["panelBody"]}>{props.children}</div>
    </section>
  );
}

function ItemActions(props: {
  label: string;
  canMoveUp: boolean;
  canMoveDown: boolean;
  onMoveUp: () => void;
  onMoveDown: () => void;
  onRemove: () => void;
}) {
  return (
    <div class={styles["itemActions"]}>
      <span>{props.label}</span>
      <div>
        <button
          type="button"
          class={styles["ghostButton"]}
          disabled={!props.canMoveUp}
          onClick={props.onMoveUp}
        >
          上移
        </button>
        <button
          type="button"
          class={styles["ghostButton"]}
          disabled={!props.canMoveDown}
          onClick={props.onMoveDown}
        >
          下移
        </button>
        <button type="button" class={styles["dangerButton"]} onClick={props.onRemove}>
          删除
        </button>
      </div>
    </div>
  );
}

function ResumeEditor(props: ResumeEditorPageProps & { initialResume: ResumeDocument }) {
  const initialResume = cloneResumeDocument(props.initialResume);
  initialResume.theme = resolveResumeThemeId(initialResume.theme);

  const [draft, setDraft] = createStore<ResumeDocument>(initialResume);
  const [status, setStatus] = createSignal<EditorStatus>({
    kind: "idle",
    message:
      props.adapter.capabilities.resumeSave.status === "supported"
        ? "修改会先在预览中生效，点击保存后写回 workspace。"
        : "Web 版读取的是打包进网页的 resume.json，修改后请下载 JSON 再重新构建。",
  });

  const canSave = () => props.adapter.capabilities.resumeSave.status === "supported";

  const setEducationRange = (itemIndex: number, rangeIndex: 0 | 1, value: string) => {
    const current = draft.education[itemIndex]?.range ?? ["", ""];
    setDraft("education", itemIndex, "range", [
      rangeIndex === 0 ? value : current[0],
      rangeIndex === 1 ? value : current[1],
    ]);
  };

  const setExperienceRange = (itemIndex: number, rangeIndex: 0 | 1, value: string) => {
    const current = draft.experiences[itemIndex]?.range ?? ["", ""];
    setDraft("experiences", itemIndex, "range", [
      rangeIndex === 0 ? value : current[0],
      rangeIndex === 1 ? value : current[1],
    ]);
  };

  const validatedDraft = (): ResumeDocument | null => {
    const plain = cloneResumeDocument(draft);
    const parsed = parseResumeValue(plain);
    if (!parsed.ok) {
      setStatus({ kind: "error", message: parsed.message });
      return null;
    }
    return parsed.data;
  };

  const save = async () => {
    const next = validatedDraft();
    if (!next) return;

    const result = await props.adapter.saveResumeJson(next);
    if (result.ok) {
      setStatus({ kind: "success", message: "已保存到 workspace/show/resume/resume.json。" });
      return;
    }
    setStatus({ kind: "error", message: result.reason });
  };

  const download = () => {
    const next = validatedDraft();
    if (!next) return;
    downloadResumeJson(next);
    setStatus({ kind: "success", message: "已下载 resume.json。" });
  };

  return (
    <main class={styles["shell"]}>
      <section class={styles["editor"]} aria-label="简历创作页">
        <div class={styles["editorHeader"]}>
          <div>
            <p class={styles["eyebrow"]}>Resume Studio</p>
            <h1>创作简历</h1>
          </div>
          <div class={styles["headerActions"]}>
            <button type="button" class={styles["secondaryButton"]} onClick={props.onBack}>
              返回预览
            </button>
            <button type="button" class={styles["secondaryButton"]} onClick={props.onOpenPrint}>
              打印版
            </button>
            <button type="button" class={styles["secondaryButton"]} onClick={download}>
              下载 JSON
            </button>
            <button
              type="button"
              class={styles["primaryButton"]}
              disabled={!canSave()}
              onClick={save}
              title={props.adapter.capabilities.resumeSave.reason}
            >
              保存到 workspace
            </button>
          </div>
        </div>

        <div class={`${styles["status"]} ${styles[status().kind]}`}>{status().message}</div>

        <div class={styles["formScroll"]}>
          <Panel title="基础信息">
            <div class={styles["grid2"]}>
              <Field label="姓名">
                <input
                  value={draft.profile.name}
                  onInput={(event) => setDraft("profile", "name", event.currentTarget.value)}
                />
              </Field>
              <Field label="标题">
                <input
                  value={draft.profile.title ?? ""}
                  onInput={(event) =>
                    setDraft("profile", "title", event.currentTarget.value || undefined)
                  }
                />
              </Field>
            </div>
            <Field label="简历主题">
              <select
                aria-label="简历主题"
                value={resolveResumeThemeId(draft.theme)}
                onInput={(event) =>
                  setDraft("theme", event.currentTarget.value as ResumeThemeId)
                }
              >
                <For each={resumeThemeOptions}>
                  {(theme) => <option value={theme.id}>{theme.label}</option>}
                </For>
              </select>
            </Field>
            <p class={styles["helpText"]}>
              {resumeThemeOptions.find((theme) => theme.id === resolveResumeThemeId(draft.theme))
                ?.description}
            </p>
          </Panel>

          <Panel title="联系方式">
            <For each={draft.profile.contacts}>
              {(contact, index) => (
                <div class={styles["item"]}>
                  <ItemActions
                    label={`联系方式 ${index() + 1}`}
                    canMoveUp={index() > 0}
                    canMoveDown={index() < draft.profile.contacts.length - 1}
                    onMoveUp={() =>
                      setDraft("profile", "contacts", (items) => moveItem(items, index(), -1))
                    }
                    onMoveDown={() =>
                      setDraft("profile", "contacts", (items) => moveItem(items, index(), 1))
                    }
                    onRemove={() =>
                      setDraft("profile", "contacts", (items) =>
                        items.filter((_, itemIndex) => itemIndex !== index()),
                      )
                    }
                  />
                  <div class={styles["grid3"]}>
                    <Field label="类型">
                      <input
                        value={contact.label}
                        onInput={(event) =>
                          setDraft(
                            "profile",
                            "contacts",
                            index(),
                            "label",
                            event.currentTarget.value,
                          )
                        }
                      />
                    </Field>
                    <Field label="内容">
                      <input
                        value={contact.value}
                        onInput={(event) =>
                          setDraft(
                            "profile",
                            "contacts",
                            index(),
                            "value",
                            event.currentTarget.value,
                          )
                        }
                      />
                    </Field>
                    <Field label="链接">
                      <input
                        value={contact.href ?? ""}
                        onInput={(event) =>
                          setDraft(
                            "profile",
                            "contacts",
                            index(),
                            "href",
                            event.currentTarget.value || undefined,
                          )
                        }
                      />
                    </Field>
                  </div>
                </div>
              )}
            </For>
            <button
              type="button"
              class={styles["addButton"]}
              onClick={() =>
                setDraft("profile", "contacts", (items) => [...items, createContact()])
              }
            >
              添加联系方式
            </button>
          </Panel>

          <Panel title="教育经历">
            <For each={draft.education}>
              {(education, index) => (
                <div class={styles["item"]}>
                  <ItemActions
                    label={`教育经历 ${index() + 1}`}
                    canMoveUp={index() > 0}
                    canMoveDown={index() < draft.education.length - 1}
                    onMoveUp={() =>
                      setDraft("education", (items) => moveItem(items, index(), -1))
                    }
                    onMoveDown={() =>
                      setDraft("education", (items) => moveItem(items, index(), 1))
                    }
                    onRemove={() =>
                      setDraft("education", (items) =>
                        items.filter((_, itemIndex) => itemIndex !== index()),
                      )
                    }
                  />
                  <div class={styles["grid2"]}>
                    <Field label="学校">
                      <input
                        value={education.school}
                        onInput={(event) =>
                          setDraft("education", index(), "school", event.currentTarget.value)
                        }
                      />
                    </Field>
                    <Field label="专业">
                      <input
                        value={education.major}
                        onInput={(event) =>
                          setDraft("education", index(), "major", event.currentTarget.value)
                        }
                      />
                    </Field>
                    <Field label="学位">
                      <input
                        value={education.degree}
                        onInput={(event) =>
                          setDraft("education", index(), "degree", event.currentTarget.value)
                        }
                      />
                    </Field>
                    <TagEditor
                      label="标签"
                      tags={education.tags}
                      onChange={(tags) => setDraft("education", index(), "tags", tags)}
                    />
                    <Field label="开始时间">
                      <input
                        value={education.range[0]}
                        onInput={(event) =>
                          setEducationRange(index(), 0, event.currentTarget.value)
                        }
                      />
                    </Field>
                    <Field label="结束时间">
                      <input
                        value={education.range[1]}
                        onInput={(event) =>
                          setEducationRange(index(), 1, event.currentTarget.value)
                        }
                      />
                    </Field>
                  </div>
                  <Field label="亮点">
                    <textarea
                      rows="3"
                      value={toLines(education.highlights)}
                      onInput={(event) =>
                        setDraft(
                          "education",
                          index(),
                          "highlights",
                          fromLines(event.currentTarget.value),
                        )
                      }
                    />
                  </Field>
                </div>
              )}
            </For>
            <button
              type="button"
              class={styles["addButton"]}
              onClick={() => setDraft("education", (items) => [...items, createEducation()])}
            >
              添加教育经历
            </button>
          </Panel>

          <Panel title="专业技能">
            <Field label="技能列表">
              <textarea
                rows="5"
                value={toLines(draft.skills)}
                onInput={(event) => setDraft("skills", fromLines(event.currentTarget.value))}
              />
            </Field>
          </Panel>

          <Panel title="实习经历">
            <For each={draft.experiences}>
              {(experience, index) => (
                <div class={styles["item"]}>
                  <ItemActions
                    label={`实习经历 ${index() + 1}`}
                    canMoveUp={index() > 0}
                    canMoveDown={index() < draft.experiences.length - 1}
                    onMoveUp={() =>
                      setDraft("experiences", (items) => moveItem(items, index(), -1))
                    }
                    onMoveDown={() =>
                      setDraft("experiences", (items) => moveItem(items, index(), 1))
                    }
                    onRemove={() =>
                      setDraft("experiences", (items) =>
                        items.filter((_, itemIndex) => itemIndex !== index()),
                      )
                    }
                  />
                  <div class={styles["grid2"]}>
                    <Field label="公司">
                      <input
                        value={experience.company}
                        onInput={(event) =>
                          setDraft("experiences", index(), "company", event.currentTarget.value)
                        }
                      />
                    </Field>
                    <Field label="角色">
                      <input
                        value={experience.role}
                        onInput={(event) =>
                          setDraft("experiences", index(), "role", event.currentTarget.value)
                        }
                      />
                    </Field>
                    <Field label="开始时间">
                      <input
                        value={experience.range[0]}
                        onInput={(event) =>
                          setExperienceRange(index(), 0, event.currentTarget.value)
                        }
                      />
                    </Field>
                    <Field label="结束时间">
                      <input
                        value={experience.range[1]}
                        onInput={(event) =>
                          setExperienceRange(index(), 1, event.currentTarget.value)
                        }
                      />
                    </Field>
                  </div>
                  <Field label="经历描述">
                    <textarea
                      rows="5"
                      value={toLines(experience.bullets)}
                      onInput={(event) =>
                        setDraft(
                          "experiences",
                          index(),
                          "bullets",
                          fromLines(event.currentTarget.value),
                        )
                      }
                    />
                  </Field>
                </div>
              )}
            </For>
            <button
              type="button"
              class={styles["addButton"]}
              onClick={() =>
                setDraft("experiences", (items) => [...items, createExperience()])
              }
            >
              添加实习经历
            </button>
          </Panel>

          <Panel title="项目经验">
            <For each={draft.projects}>
              {(project, index) => (
                <div class={styles["item"]}>
                  <ItemActions
                    label={`项目经验 ${index() + 1}`}
                    canMoveUp={index() > 0}
                    canMoveDown={index() < draft.projects.length - 1}
                    onMoveUp={() => setDraft("projects", (items) => moveItem(items, index(), -1))}
                    onMoveDown={() => setDraft("projects", (items) => moveItem(items, index(), 1))}
                    onRemove={() =>
                      setDraft("projects", (items) =>
                        items.filter((_, itemIndex) => itemIndex !== index()),
                      )
                    }
                  />
                  <div class={styles["grid2"]}>
                    <Field label="项目名">
                      <input
                        value={project.title}
                        onInput={(event) =>
                          setDraft("projects", index(), "title", event.currentTarget.value)
                        }
                      />
                    </Field>
                    <Field label="副标题">
                      <input
                        value={project.subtitle ?? ""}
                        onInput={(event) =>
                          setDraft(
                            "projects",
                            index(),
                            "subtitle",
                            event.currentTarget.value || undefined,
                          )
                        }
                      />
                    </Field>
                  </div>
                  <TagEditor
                    label="技术标签"
                    tags={project.tags}
                    onChange={(tags) => setDraft("projects", index(), "tags", tags)}
                  />
                  <Field label="项目简介">
                    <textarea
                      rows="5"
                      value={toLines(project.bullets)}
                      onInput={(event) =>
                        setDraft("projects", index(), "bullets", fromLines(event.currentTarget.value))
                      }
                    />
                  </Field>
                </div>
              )}
            </For>
            <button
              type="button"
              class={styles["addButton"]}
              onClick={() => setDraft("projects", (items) => [...items, createProject()])}
            >
              添加项目经验
            </button>
          </Panel>
        </div>
      </section>

      <section class={styles["preview"]} aria-label="简历实时预览">
        <ResumeDocumentView resume={draft} />
      </section>
    </main>
  );
}

export function ResumeEditorPage(props: ResumeEditorPageProps) {
  const [resume] = createResource(() => props.adapter, loadResume);

  return (
    <>
      <Show when={resume.loading}>
        <main class={styles["loading"]}>正在读取简历数据...</main>
      </Show>
      <Show when={resume.error}>
        <main class={styles["loading"]}>
          <div class={styles["loadError"]}>
            {resume.error instanceof Error ? resume.error.message : "未知错误"}
          </div>
        </main>
      </Show>
      <Show when={resume()} keyed>
        {(data) => (
          <ResumeEditor
            adapter={props.adapter}
            initialResume={data}
            onBack={props.onBack}
            onOpenPrint={props.onOpenPrint}
          />
        )}
      </Show>
    </>
  );
}
