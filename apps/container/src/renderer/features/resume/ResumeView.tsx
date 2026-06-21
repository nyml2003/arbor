import { For, Show, createEffect, createResource, createSignal } from "solid-js";
import type { JSX } from "solid-js";
import { parseResumeValue } from "./resumeData";
import type { PlatformAdapter } from "../../platform/types";
import type { ResumeDocument } from "./types";
import styles from "./ResumeView.module.css";

interface ResumeViewProps {
  adapter: PlatformAdapter;
  onOpenPrint?: () => void;
  onPrint?: () => void;
}

interface ResumePrintPageProps {
  adapter: PlatformAdapter;
  autoPrint: boolean;
  onAutoPrintDone: () => void;
  onBack: () => void;
  onPrint: () => void;
}

async function loadResume(adapter: PlatformAdapter): Promise<ResumeDocument> {
  const raw = await adapter.readResumeJson();
  const parsed = parseResumeValue(raw);
  if (!parsed.ok) {
    throw new Error(parsed.message);
  }
  return parsed.data;
}

function formatRange(range: [string, string]): string {
  return `${range[0]} - ${range[1]}`;
}

function ContactLabel(props: { label: string }) {
  const labelMap: Record<string, string> = {
    Email: "Mail",
    GitHub: "GitHub",
    Phone: "Phone",
    Site: "Site",
  };

  return <span class={styles["contactLabel"]}>{labelMap[props.label] ?? props.label}</span>;
}

function Tags(props: { tags: string[] | undefined }) {
  return (
    <Show when={props.tags && props.tags.length > 0}>
      <span class={styles["tags"]}>
        <For each={props.tags}>
          {(tag) => (
            <span class={styles["tag"]} data-testid="resume-tag">
              {tag}
            </span>
          )}
        </For>
      </span>
    </Show>
  );
}

function Section(props: { title: string; children: JSX.Element }) {
  return (
    <section class={styles["section"]}>
      <div class={styles["sectionTitle"]}>{props.title}</div>
      <div class={styles["sectionBody"]}>{props.children}</div>
    </section>
  );
}

function BulletList(props: { items: string[]; compact?: boolean }) {
  return (
    <ul class={props.compact ? styles["compactList"] : styles["bulletList"]}>
      <For each={props.items}>{(item) => <li>{item}</li>}</For>
    </ul>
  );
}

function ResumeToolbar(props: {
  variant: "screen" | "print";
  onBack?: (() => void) | undefined;
  onOpenPrint?: (() => void) | undefined;
  onPrint: () => void;
}) {
  return (
    <div
      class={`${styles["toolbar"]} ${props.variant === "print" ? styles["toolbarPrint"] : ""}`}
      data-testid="resume-toolbar"
    >
      <div class={styles["toolbarTitle"]}>{props.variant === "print" ? "打印版" : "简历"}</div>
      <div class={styles["toolbarActions"]}>
        {props.onBack ? (
          <button type="button" class={styles["secondaryAction"]} onClick={props.onBack}>
            返回简历
          </button>
        ) : null}
        {props.onOpenPrint ? (
          <button type="button" class={styles["secondaryAction"]} onClick={props.onOpenPrint}>
            打印版
          </button>
        ) : null}
        <button type="button" class={styles["primaryAction"]} onClick={props.onPrint}>
          打印
        </button>
      </div>
    </div>
  );
}

export function ResumeDocumentView(props: {
  resume: ResumeDocument;
  toolbar?: JSX.Element;
  printMode?: boolean;
}) {
  return (
    <main class={`${styles["shell"]} ${props.printMode ? styles["printShell"] : ""}`}>
      {props.toolbar}
      <article class={styles["page"]} data-testid="resume-page">
        <header class={styles["header"]}>
          <h1>{props.resume.profile.name}</h1>
          <Show when={props.resume.profile.title}>
            <p class={styles["profileTitle"]}>{props.resume.profile.title}</p>
          </Show>
          <div class={styles["contacts"]}>
            <For each={props.resume.profile.contacts}>
              {(contact) => (
                <span class={styles["contact"]}>
                  <ContactLabel label={contact.label} />
                  <Show
                    when={contact.href}
                    fallback={<span class={styles["contactValue"]}>{contact.value}</span>}
                  >
                    <a href={contact.href} title={contact.label}>
                      {contact.value}
                    </a>
                  </Show>
                </span>
              )}
            </For>
          </div>
        </header>

        <Section title="教育经历">
          <For each={props.resume.education}>
            {(education) => (
              <div class={styles["block"]}>
                <div class={styles["row"]}>
                  <div class={styles["rowMain"]}>
                    <strong>{education.school}</strong>
                    <Tags tags={education.tags} />
                    <span>{education.major}</span>
                    <span>{education.degree}</span>
                  </div>
                  <time>{formatRange(education.range)}</time>
                </div>
                <Show when={education.highlights}>
                  <BulletList items={education.highlights ?? []} compact />
                </Show>
              </div>
            )}
          </For>
        </Section>

        <Section title="专业技能">
          <BulletList items={props.resume.skills} compact />
        </Section>

        <Section title="实习经历">
          <For each={props.resume.experiences}>
            {(experience) => (
              <div class={styles["block"]}>
                <div class={styles["row"]}>
                  <div class={styles["rowMain"]}>
                    <strong>{experience.company}</strong>
                    <span>{experience.role}</span>
                  </div>
                  <time>{formatRange(experience.range)}</time>
                </div>
                <BulletList items={experience.bullets} />
              </div>
            )}
          </For>
        </Section>

        <Section title="项目经验">
          <For each={props.resume.projects}>
            {(project) => (
              <div class={styles["block"]}>
                <div class={styles["row"]}>
                  <div class={styles["projectTitle"]}>
                    <strong>{project.title}</strong>
                    <Show when={project.subtitle}>
                      <span>{project.subtitle}</span>
                    </Show>
                  </div>
                  <Tags tags={project.tags} />
                </div>
                <BulletList items={project.bullets} />
              </div>
            )}
          </For>
        </Section>
      </article>
    </main>
  );
}

export function ResumeView(props: ResumeViewProps) {
  const [resume] = createResource(() => props.adapter, loadResume);
  const print = () => {
    if (props.onPrint) {
      props.onPrint();
      return;
    }
    window.print();
  };

  return (
    <>
      <Show when={resume.loading}>
        <main class={styles["shell"]}>
          <div class={styles["status"]}>正在读取简历数据...</div>
        </main>
      </Show>
      <Show when={resume.error}>
        <main class={styles["shell"]}>
          <div class={styles["error"]}>
            <strong>简历数据无法渲染</strong>
            <span>{resume.error instanceof Error ? resume.error.message : "未知错误"}</span>
          </div>
        </main>
      </Show>
      <Show when={resume()}>
        {(data) => (
          <ResumeDocumentView
            resume={data()}
            toolbar={
              <ResumeToolbar
                variant="screen"
                onOpenPrint={props.onOpenPrint}
                onPrint={print}
              />
            }
          />
        )}
      </Show>
    </>
  );
}

export function ResumePrintPage(props: ResumePrintPageProps) {
  const [resume] = createResource(() => props.adapter, loadResume);
  const [hasPrinted, setHasPrinted] = createSignal(false);

  createEffect(() => {
    if (!props.autoPrint || hasPrinted() || !resume()) return;
    setHasPrinted(true);
    window.setTimeout(() => {
      window.print();
      props.onAutoPrintDone();
    }, 0);
  });

  return (
    <>
      <Show when={resume.loading}>
        <main class={`${styles["shell"]} ${styles["printShell"]}`}>
          <ResumeToolbar variant="print" onBack={props.onBack} onPrint={props.onPrint} />
          <div class={styles["status"]}>正在读取简历数据...</div>
        </main>
      </Show>
      <Show when={resume.error}>
        <main class={`${styles["shell"]} ${styles["printShell"]}`}>
          <ResumeToolbar variant="print" onBack={props.onBack} onPrint={props.onPrint} />
          <div class={styles["error"]}>
            <strong>简历数据无法渲染</strong>
            <span>{resume.error instanceof Error ? resume.error.message : "未知错误"}</span>
          </div>
        </main>
      </Show>
      <Show when={resume()}>
        {(data) => (
          <ResumeDocumentView
            resume={data()}
            printMode
            toolbar={<ResumeToolbar variant="print" onBack={props.onBack} onPrint={props.onPrint} />}
          />
        )}
      </Show>
    </>
  );
}
