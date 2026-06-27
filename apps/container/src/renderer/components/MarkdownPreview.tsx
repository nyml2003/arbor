import { For } from "solid-js";
import styles from "./MarkdownPreview.module.css";

type InlinePart =
  | Readonly<{ kind: "text"; text: string }>
  | Readonly<{ kind: "code"; text: string }>;

type MarkdownBlock =
  | Readonly<{ kind: "heading"; level: 1 | 2 | 3; text: string }>
  | Readonly<{ kind: "paragraph"; parts: ReadonlyArray<InlinePart> }>
  | Readonly<{ kind: "list"; items: ReadonlyArray<ReadonlyArray<InlinePart>> }>
  | Readonly<{ kind: "code"; text: string }>;

export function MarkdownPreview(props: { text: string }) {
  const blocks = () => parseMarkdown(props.text);

  return (
    <article class={styles["preview"]} data-testid="markdown-preview">
      <For each={blocks()}>
        {(block) => <MarkdownBlockView block={block} />}
      </For>
    </article>
  );
}

function MarkdownBlockView(props: { block: MarkdownBlock }) {
  if (props.block.kind === "heading") {
    return <DynamicHeading level={props.block.level} text={props.block.text} />;
  }

  if (props.block.kind === "paragraph") {
    return (
      <p class={styles["paragraph"]}>
        <InlineParts parts={props.block.parts} />
      </p>
    );
  }

  if (props.block.kind === "list") {
    return (
      <ul class={styles["list"]}>
        <For each={props.block.items}>
          {(item) => (
            <li>
              <InlineParts parts={item} />
            </li>
          )}
        </For>
      </ul>
    );
  }

  return <pre class={styles["codeBlock"]}>{props.block.text}</pre>;
}

function DynamicHeading(props: { level: 1 | 2 | 3; text: string }) {
  if (props.level === 1) return <h1 class={styles["heading1"]}>{props.text}</h1>;
  if (props.level === 2) return <h2 class={styles["heading2"]}>{props.text}</h2>;
  return <h3 class={styles["heading3"]}>{props.text}</h3>;
}

function InlineParts(props: { parts: ReadonlyArray<InlinePart> }) {
  return (
    <For each={props.parts}>
      {(part) => part.kind === "code" ? <code class={styles["inlineCode"]}>{part.text}</code> : part.text}
    </For>
  );
}

function parseMarkdown(text: string): MarkdownBlock[] {
  const lines = text.replace(/\r\n/g, "\n").split("\n");
  const blocks: MarkdownBlock[] = [];
  let paragraph: string[] = [];
  let listItems: ReadonlyArray<InlinePart>[] = [];
  let codeLines: string[] | null = null;

  const flushParagraph = () => {
    if (paragraph.length === 0) return;
    blocks.push({ kind: "paragraph", parts: parseInline(paragraph.join(" ")) });
    paragraph = [];
  };

  const flushList = () => {
    if (listItems.length === 0) return;
    blocks.push({ kind: "list", items: listItems });
    listItems = [];
  };

  for (const line of lines) {
    if (line.trimStart().startsWith("```")) {
      if (codeLines === null) {
        flushParagraph();
        flushList();
        codeLines = [];
      } else {
        blocks.push({ kind: "code", text: codeLines.join("\n") });
        codeLines = null;
      }
      continue;
    }

    if (codeLines !== null) {
      codeLines.push(line);
      continue;
    }

    const trimmed = line.trim();
    if (trimmed.length === 0) {
      flushParagraph();
      flushList();
      continue;
    }

    const headingMatch = /^(#{1,3})\s+(.+)$/.exec(trimmed);
    if (headingMatch) {
      flushParagraph();
      flushList();
      const marker = headingMatch[1] ?? "#";
      blocks.push({
        kind: "heading",
        level: marker.length as 1 | 2 | 3,
        text: headingMatch[2] ?? "",
      });
      continue;
    }

    const listMatch = /^[-*]\s+(.+)$/.exec(trimmed);
    if (listMatch) {
      flushParagraph();
      listItems = [...listItems, parseInline(listMatch[1] ?? "")];
      continue;
    }

    flushList();
    paragraph.push(trimmed);
  }

  if (codeLines !== null) {
    blocks.push({ kind: "code", text: codeLines.join("\n") });
  }
  flushParagraph();
  flushList();
  return blocks;
}

function parseInline(text: string): ReadonlyArray<InlinePart> {
  const parts: InlinePart[] = [];
  let remaining = text;

  while (remaining.length > 0) {
    const start = remaining.indexOf("`");
    if (start === -1) {
      parts.push({ kind: "text", text: remaining });
      break;
    }

    const end = remaining.indexOf("`", start + 1);
    if (end === -1) {
      parts.push({ kind: "text", text: remaining });
      break;
    }

    if (start > 0) {
      parts.push({ kind: "text", text: remaining.slice(0, start) });
    }
    parts.push({ kind: "code", text: remaining.slice(start + 1, end) });
    remaining = remaining.slice(end + 1);
  }

  return parts;
}
