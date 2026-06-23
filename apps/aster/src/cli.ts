#!/usr/bin/env node
import { existsSync } from "node:fs";
import { readFile, stat } from "node:fs/promises";
import { exit, stdin, stdout } from "node:process";
import { createInterface } from "node:readline/promises";
import { fileURLToPath } from "node:url";
import { basename, join, resolve } from "node:path";

const packageVersion = "0.1.0";
const defaultBaseUrl = "https://api.deepseek.com";
const defaultModel = "deepseek-v4-flash";

type CliOptions = Readonly<{
  prompt: string | null;
  model: string;
  system: string | null;
  json: boolean;
  skills: ReadonlyArray<string>;
  skillDirs: ReadonlyArray<string>;
}>;

type ParseResult =
  | Readonly<{ ok: true; options: CliOptions }>
  | Readonly<{ ok: true; output: string }>
  | Readonly<{ ok: false; message: string }>;

type ChatMessage = Readonly<{
  role: "system" | "user" | "assistant";
  content: string;
}>;

type ChatRequest = Readonly<{
  model: string;
  messages: ReadonlyArray<ChatMessage>;
  stream: boolean;
  thinking: Readonly<{ type: "disabled" }>;
}>;

type ChatChoice = Readonly<{
  message?: Readonly<{
    content?: unknown;
  }>;
}>;

type ChatResponse = Readonly<{
  choices?: ReadonlyArray<ChatChoice>;
}>;

type StreamingChatDelta = Readonly<{
  choices?: ReadonlyArray<Readonly<{
    delta?: Readonly<{
      content?: unknown;
    }>;
  }>>;
}>;

type LoadedSkill = Readonly<{
  name: string;
  description: string | null;
  body: string;
  path: string;
}>;

type ReplState = Readonly<{
  baseSystem: string | null;
  skillSearchDirs: ReadonlyArray<string>;
  skills: ReadonlyArray<LoadedSkill>;
}>;

type Runtime = Readonly<{
  env: Readonly<Record<string, string | undefined>>;
  fetch: typeof fetch;
  baseUrl?: string;
  readLine?: (prompt: string) => Promise<string | null>;
  writeStdout?: (text: string) => void;
  writeStderr?: (text: string) => void;
  cwd?: string;
}>;

export async function runCli(
  argv: ReadonlyArray<string>,
  runtime: Runtime = { env: process.env, fetch },
): Promise<Readonly<{
  exitCode: number;
  stdout: string;
  stderr: string;
}>> {
  const parsed = parseArgs(argv);

  if (!parsed.ok) {
    return {
      exitCode: 2,
      stdout: "",
      stderr: `${parsed.message}\n${usage()}\n`,
    };
  }

  if ("output" in parsed) {
    return {
      exitCode: 0,
      stdout: parsed.output,
      stderr: "",
    };
  }

  const apiKey = runtime.env["DEEPSEEK_API_KEY"];
  if (typeof apiKey !== "string" || apiKey.length === 0) {
    return {
      exitCode: 1,
      stdout: "",
      stderr: "Missing DEEPSEEK_API_KEY.\n",
    };
  }

  if (parsed.options.prompt === null) {
    try {
      return runRepl({
        apiKey,
        baseUrl: runtime.baseUrl ?? defaultBaseUrl,
        fetchImpl: runtime.fetch,
        options: await loadSkillsForOptions(parsed.options, runtime.cwd ?? process.cwd()),
        runtime,
      });
    } catch (error) {
      return {
        exitCode: 1,
        stdout: "",
        stderr: `${error instanceof Error ? error.message : "Unknown error"}\n`,
      };
    }
  }

  const prompt = parsed.options.prompt;

  try {
    const output = createOutput(runtime);
    const options = await loadSkillsForOptions(parsed.options, runtime.cwd ?? process.cwd());
    const messages = createInitialMessages(options.system);
    messages.push({
      role: "user",
      content: prompt,
    });

    if (options.json) {
      const response = await createChatCompletion({
        apiKey,
        baseUrl: runtime.baseUrl ?? defaultBaseUrl,
        fetchImpl: runtime.fetch,
        model: options.model,
        messages,
      });

      return {
        exitCode: 0,
        stdout: `${JSON.stringify(response, null, 2)}\n`,
        stderr: "",
      };
    }

    await streamAndRenderCompletion({
      apiKey,
      baseUrl: runtime.baseUrl ?? defaultBaseUrl,
      fetchImpl: runtime.fetch,
      model: options.model,
      messages,
      writeStdout: output.writeStdout,
    });

    return {
      exitCode: 0,
      stdout: output.stdout(),
      stderr: output.stderr(),
    };
  } catch (error) {
    return {
      exitCode: 1,
      stdout: "",
      stderr: `${error instanceof Error ? error.message : "Unknown error"}\n`,
    };
  }
}

function parseArgs(argv: ReadonlyArray<string>): ParseResult {
  const args = [...argv];

  let model = defaultModel;
  let system: string | null = null;
  let json = false;
  const skills: string[] = [];
  const skillDirs: string[] = [];
  const promptParts: string[] = [];

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index] ?? "";

    if (arg === "--help" || arg === "-h") {
      return { ok: true, output: `${usage()}\n` };
    }

    if (arg === "--version" || arg === "-v") {
      return { ok: true, output: `${packageVersion}\n` };
    }

    if (arg === "--json") {
      json = true;
      continue;
    }

    if (arg === "--skill") {
      const value = args[index + 1];
      if (typeof value !== "string" || value.length === 0) {
        return { ok: false, message: "--skill requires a value." };
      }
      skills.push(value);
      index += 1;
      continue;
    }

    if (arg === "--skill-dir") {
      const value = args[index + 1];
      if (typeof value !== "string" || value.length === 0) {
        return { ok: false, message: "--skill-dir requires a value." };
      }
      skillDirs.push(value);
      index += 1;
      continue;
    }

    if (arg === "--model") {
      const value = args[index + 1];
      if (typeof value !== "string" || value.length === 0) {
        return { ok: false, message: "--model requires a value." };
      }
      model = value;
      index += 1;
      continue;
    }

    if (arg === "--system") {
      const value = args[index + 1];
      if (typeof value !== "string" || value.length === 0) {
        return { ok: false, message: "--system requires a value." };
      }
      system = value;
      index += 1;
      continue;
    }

    if (arg.startsWith("-")) {
      return { ok: false, message: `Unknown option: ${arg}` };
    }

    promptParts.push(arg);
  }

  const prompt = promptParts.join(" ").trim();

  return {
    ok: true,
    options: {
      prompt: prompt.length === 0 ? null : prompt,
      model,
      system,
      json,
      skills,
      skillDirs,
    },
  };
}

async function loadSkillsForOptions(options: CliOptions, cwd: string): Promise<CliOptions> {
  if (options.skills.length === 0) {
    return options;
  }

  const skillSearchDirs = createSkillSearchDirs(options.skillDirs, cwd);
  const loadedSkills: LoadedSkill[] = [];

  for (const skill of options.skills) {
    loadedSkills.push(await loadSkill(skill, skillSearchDirs, cwd));
  }

  return {
    ...options,
    system: mergeSystemAndSkills(options.system, loadedSkills),
  };
}

function createSkillSearchDirs(skillDirs: ReadonlyArray<string>, cwd: string): ReadonlyArray<string> {
  const dirs = [
    ...skillDirs.map((dir) => resolve(cwd, dir)),
    resolve(cwd, ".codex", "skills"),
    resolve(cwd, "packages", "arbor-skills", "skills"),
    resolve(cwd, ".installed-skills"),
  ];
  return [...new Set(dirs)];
}

async function loadSkill(skillRef: string, searchDirs: ReadonlyArray<string>, cwd: string): Promise<LoadedSkill> {
  const directPath = resolve(cwd, skillRef);
  const checkedPaths: string[] = [];

  const directSkillPath = await resolveSkillPath(directPath, checkedPaths);
  if (directSkillPath !== null) {
    return readSkillFile(directSkillPath);
  }

  for (const searchDir of searchDirs) {
    const candidate = join(searchDir, skillRef);
    const skillPath = await resolveSkillPath(candidate, checkedPaths);
    if (skillPath !== null) {
      return readSkillFile(skillPath);
    }
  }

  throw new Error([
    `Skill not found: ${skillRef}`,
    "Checked:",
    ...checkedPaths.map((checkedPath) => `  ${checkedPath}`),
  ].join("\n"));
}

async function resolveSkillPath(path: string, checkedPaths: string[]): Promise<string | null> {
  checkedPaths.push(path);
  if (!existsSync(path)) {
    return null;
  }

  const pathStat = await stat(path);
  if (pathStat.isDirectory()) {
    const skillPath = join(path, "SKILL.md");
    checkedPaths.push(skillPath);
    return existsSync(skillPath) ? skillPath : null;
  }

  if (pathStat.isFile()) {
    return path;
  }

  return null;
}

async function readSkillFile(skillPath: string): Promise<LoadedSkill> {
  const content = await readFile(skillPath, "utf8");
  const parsed = parseSkillMarkdown(content, basename(resolve(skillPath, "..")));
  return {
    ...parsed,
    path: skillPath,
  };
}

function parseSkillMarkdown(content: string, fallbackName: string): Omit<LoadedSkill, "path"> {
  const frontMatterMatch = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?/.exec(content);
  const frontMatter = frontMatterMatch?.[1] ?? "";
  const body = frontMatterMatch === null ? content.trim() : content.slice(frontMatterMatch[0].length).trim();
  const metadata = parseSimpleYamlFrontMatter(frontMatter);

  return {
    name: metadata["name"] ?? fallbackName,
    description: metadata["description"] ?? null,
    body,
  };
}

function parseSimpleYamlFrontMatter(frontMatter: string): Readonly<Record<string, string>> {
  const metadata: Record<string, string> = {};
  for (const line of frontMatter.split(/\r?\n/)) {
    const match = /^([A-Za-z0-9_-]+):\s*(.*)$/.exec(line);
    if (match === null) {
      continue;
    }
    const key = match[1] ?? "";
    const rawValue = match[2] ?? "";
    metadata[key] = rawValue.replace(/^["']|["']$/g, "").trim();
  }
  return metadata;
}

function mergeSystemAndSkills(system: string | null, skills: ReadonlyArray<LoadedSkill>): string {
  const parts = system === null ? [] : [system];
  parts.push(formatSkillsForSystem(skills));
  return parts.join("\n\n");
}

function formatSkillsForSystem(skills: ReadonlyArray<LoadedSkill>): string {
  const formattedSkills = skills.map((skill, index) => [
    `## Skill ${index + 1}: ${skill.name}`,
    skill.description === null ? null : `Description: ${skill.description}`,
    `Source: ${skill.path}`,
    "",
    skill.body,
  ].filter((part): part is string => part !== null).join("\n"));

  return [
    "Active skills:",
    "Use the following local SKILL.md instructions as additional operating guidance for this answer.",
    "",
    ...formattedSkills,
  ].join("\n");
}

async function createChatCompletion(input: Readonly<{
  apiKey: string;
  baseUrl: string;
  fetchImpl: typeof fetch;
  model: string;
  messages: ReadonlyArray<ChatMessage>;
}>): Promise<ChatResponse> {
  const request: ChatRequest = {
    model: input.model,
    messages: input.messages,
    stream: false,
    thinking: { type: "disabled" },
  };

  const response = await input.fetchImpl(`${input.baseUrl}/chat/completions`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${input.apiKey}`,
    },
    body: JSON.stringify(request),
  });

  const text = await response.text();
  const parsed = parseJsonObject(text);

  if (!response.ok) {
    throw new Error(formatApiError(response.status, parsed, text));
  }

  return parsed as ChatResponse;
}

async function streamChatCompletion(input: Readonly<{
  apiKey: string;
  baseUrl: string;
  fetchImpl: typeof fetch;
  model: string;
  messages: ReadonlyArray<ChatMessage>;
  writeDelta: (text: string) => void;
}>): Promise<string> {
  const request: ChatRequest = {
    model: input.model,
    messages: input.messages,
    stream: true,
    thinking: { type: "disabled" },
  };

  const response = await input.fetchImpl(`${input.baseUrl}/chat/completions`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${input.apiKey}`,
    },
    body: JSON.stringify(request),
  });

  if (!response.ok) {
    const text = await response.text();
    throw new Error(formatApiError(response.status, parseJsonObject(text), text));
  }

  if (response.body === null) {
    throw new Error("DeepSeek stream response did not include a body.");
  }

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";
  let assistantText = "";

  try {
    while (true) {
      const chunk = await reader.read();
      if (chunk.done) {
        break;
      }

      buffer += decoder.decode(chunk.value, { stream: true });
      const lines = buffer.split(/\r?\n/);
      buffer = lines.pop() ?? "";

      for (const line of lines) {
        const delta = parseStreamLine(line);
        if (delta === null) {
          continue;
        }
        input.writeDelta(delta);
        assistantText += delta;
      }
    }

    buffer += decoder.decode();
    if (buffer.length > 0) {
      for (const line of buffer.split(/\r?\n/)) {
        const delta = parseStreamLine(line);
        if (delta === null) {
          continue;
        }
        input.writeDelta(delta);
        assistantText += delta;
      }
    }
  } finally {
    reader.releaseLock();
  }

  if (assistantText.length === 0) {
    throw new Error("DeepSeek stream did not include assistant content.");
  }

  return assistantText;
}

function parseStreamLine(line: string): string | null {
  const trimmed = line.trim();
  if (trimmed.length === 0 || !trimmed.startsWith("data:")) {
    return null;
  }

  const payload = trimmed.slice("data:".length).trim();
  if (payload === "[DONE]") {
    return null;
  }

  const parsed = parseJsonObject(payload) as StreamingChatDelta;
  const content = parsed.choices?.[0]?.delta?.content;
  return typeof content === "string" && content.length > 0 ? content : null;
}

async function runRepl(input: Readonly<{
  apiKey: string;
  baseUrl: string;
  fetchImpl: typeof fetch;
  options: CliOptions;
  runtime: Runtime;
}>): Promise<Readonly<{
  exitCode: number;
  stdout: string;
  stderr: string;
}>> {
  const output = createOutput(input.runtime);
  const lineReader = createLineReader(input.runtime);
  const cwd = input.runtime.cwd ?? process.cwd();
  let state: ReplState = {
    baseSystem: input.options.system,
    skillSearchDirs: createSkillSearchDirs(input.options.skillDirs, cwd),
    skills: [],
  };
  let messages = createInitialMessages(createSystemWithSkills(state));

  output.writeStdout("Aster chat. Type /exit or /quit to leave. Use /skill <name-or-path> to load a skill.\n");

  try {
    while (true) {
      const line = await lineReader.read("aster> ");
      if (line === null) {
        break;
      }

      const prompt = line.trim();
      if (prompt === "/exit" || prompt === "/quit") {
        break;
      }
      if (prompt === "/skills") {
        output.writeStdout(formatLoadedSkills(state.skills));
        continue;
      }
      if (prompt.startsWith("/skill ")) {
        try {
          const skillRef = prompt.slice("/skill ".length).trim();
          if (skillRef.length === 0) {
            output.writeStderr("/skill requires a value.\n");
            continue;
          }
          const skill = await loadSkill(skillRef, state.skillSearchDirs, cwd);
          state = {
            ...state,
            skills: [...state.skills, skill],
          };
          messages = createInitialMessages(createSystemWithSkills(state));
          output.writeStdout(`Loaded skill: ${skill.name}\n`);
        } catch (error) {
          output.writeStderr(`${error instanceof Error ? error.message : "Unknown error"}\n`);
        }
        continue;
      }
      if (prompt.length === 0) {
        continue;
      }

      const nextMessages = [
        ...messages,
        {
          role: "user",
          content: prompt,
        } satisfies ChatMessage,
      ];

      try {
        const assistantText = input.options.json
          ? await writeJsonCompletion({
              apiKey: input.apiKey,
              baseUrl: input.baseUrl,
              fetchImpl: input.fetchImpl,
              model: input.options.model,
              messages: nextMessages,
              writeStdout: output.writeStdout,
            })
          : await streamAndRenderCompletion({
              apiKey: input.apiKey,
              baseUrl: input.baseUrl,
              fetchImpl: input.fetchImpl,
              model: input.options.model,
              messages: nextMessages,
              writeStdout: output.writeStdout,
            });
        messages = [
          ...nextMessages,
          {
            role: "assistant",
            content: assistantText,
          },
        ];
      } catch (error) {
        output.writeStderr(`${error instanceof Error ? error.message : "Unknown error"}\n`);
      }
    }

    return {
      exitCode: 0,
      stdout: output.stdout(),
      stderr: output.stderr(),
    };
  } finally {
    lineReader.close();
  }
}

function createSystemWithSkills(state: ReplState): string | null {
  if (state.skills.length === 0) {
    return state.baseSystem;
  }
  return mergeSystemAndSkills(state.baseSystem, state.skills);
}

function formatLoadedSkills(skills: ReadonlyArray<LoadedSkill>): string {
  if (skills.length === 0) {
    return "No skills loaded.\n";
  }
  const lines = skills.map((skill) => `- ${skill.name}: ${skill.path}`);
  return `${lines.join("\n")}\n`;
}

async function writeJsonCompletion(input: Readonly<{
  apiKey: string;
  baseUrl: string;
  fetchImpl: typeof fetch;
  model: string;
  messages: ReadonlyArray<ChatMessage>;
  writeStdout: (text: string) => void;
}>): Promise<string> {
  const response = await createChatCompletion({
    apiKey: input.apiKey,
    baseUrl: input.baseUrl,
    fetchImpl: input.fetchImpl,
    model: input.model,
    messages: input.messages,
  });
  const assistantText = extractAssistantText(response);
  input.writeStdout(`${JSON.stringify(response, null, 2)}\n`);
  return assistantText;
}

async function streamAndRenderCompletion(input: Readonly<{
  apiKey: string;
  baseUrl: string;
  fetchImpl: typeof fetch;
  model: string;
  messages: ReadonlyArray<ChatMessage>;
  writeStdout: (text: string) => void;
}>): Promise<string> {
  const assistantText = await streamChatCompletion({
    apiKey: input.apiKey,
    baseUrl: input.baseUrl,
    fetchImpl: input.fetchImpl,
    model: input.model,
    messages: input.messages,
    writeDelta: input.writeStdout,
  });
  writeRenderedMarkdown(input.writeStdout, assistantText);
  return assistantText;
}

function createInitialMessages(system: string | null): ChatMessage[] {
  return system === null
    ? []
    : [{
        role: "system",
        content: system,
      }];
}

function extractAssistantText(response: ChatResponse): string {
  const content = response.choices?.[0]?.message?.content;
  if (typeof content !== "string" || content.length === 0) {
    throw new Error("DeepSeek response did not include assistant content.");
  }
  return content;
}

function writeRenderedMarkdown(writeStdout: (text: string) => void, markdown: string): void {
  writeStdout(`\n\n--- rendered markdown ---\n${renderTerminalMarkdown(markdown)}\n`);
}

function renderTerminalMarkdown(markdown: string): string {
  const rendered: string[] = [];
  let inCodeBlock = false;

  for (const line of markdown.split(/\r?\n/)) {
    if (line.trim().startsWith("```")) {
      inCodeBlock = !inCodeBlock;
      continue;
    }

    if (inCodeBlock) {
      rendered.push(`${ansi.dim}  ${line}${ansi.reset}`);
      continue;
    }

    const heading = /^(#{1,6})\s+(.+)$/.exec(line);
    if (heading !== null) {
      rendered.push(`${ansi.bold}${ansi.cyan}${heading[2] ?? ""}${ansi.reset}`);
      continue;
    }

    rendered.push(renderInlineMarkdown(line.replace(/^(\s*)[-*]\s+/, "$1- ")));
  }

  return rendered.join("\n").trimEnd();
}

function renderInlineMarkdown(line: string): string {
  return line
    .replace(/\*\*([^*]+)\*\*/g, `${ansi.bold}$1${ansi.reset}`)
    .replace(/`([^`]+)`/g, `${ansi.inverse}$1${ansi.reset}`);
}

const ansi = {
  bold: "\x1b[1m",
  cyan: "\x1b[36m",
  dim: "\x1b[2m",
  inverse: "\x1b[7m",
  reset: "\x1b[0m",
} as const;

function parseJsonObject(text: string): Readonly<Record<string, unknown>> {
  if (text.length === 0) {
    return {};
  }

  try {
    const parsed = JSON.parse(text) as unknown;
    if (typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)) {
      return parsed as Readonly<Record<string, unknown>>;
    }
  } catch {
    return {};
  }

  return {};
}

function formatApiError(
  status: number,
  parsed: Readonly<Record<string, unknown>>,
  rawText: string,
): string {
  const error = parsed["error"];
  if (typeof error === "object" && error !== null && !Array.isArray(error)) {
    const message = (error as Readonly<Record<string, unknown>>)["message"];
    if (typeof message === "string" && message.length > 0) {
      return `DeepSeek API error ${status}: ${message}`;
    }
  }

  const message = parsed["message"];
  if (typeof message === "string" && message.length > 0) {
    return `DeepSeek API error ${status}: ${message}`;
  }

  const suffix = rawText.length > 0 ? ` ${rawText}` : "";
  return `DeepSeek API error ${status}.${suffix}`;
}

function createOutput(runtime: Runtime): Readonly<{
  writeStdout: (text: string) => void;
  writeStderr: (text: string) => void;
  stdout: () => string;
  stderr: () => string;
}> {
  const stdoutBuffer: string[] = [];
  const stderrBuffer: string[] = [];

  return {
    writeStdout: (text) => {
      if (runtime.writeStdout !== undefined) {
        runtime.writeStdout(text);
        return;
      }
      stdoutBuffer.push(text);
    },
    writeStderr: (text) => {
      if (runtime.writeStderr !== undefined) {
        runtime.writeStderr(text);
        return;
      }
      stderrBuffer.push(text);
    },
    stdout: () => stdoutBuffer.join(""),
    stderr: () => stderrBuffer.join(""),
  };
}

function createLineReader(runtime: Runtime): Readonly<{
  read: (prompt: string) => Promise<string | null>;
  close: () => void;
}> {
  if (runtime.readLine !== undefined) {
    return {
      read: runtime.readLine,
      close: () => {},
    };
  }

  const interfaceHandle = createInterface({
    input: stdin,
    output: stdout,
  });

  return {
    read: async (prompt) => {
      try {
        return await interfaceHandle.question(prompt);
      } catch (error) {
        if (error instanceof Error && error.name === "AbortError") {
          return null;
        }
        throw error;
      }
    },
    close: () => {
      interfaceHandle.close();
    },
  };
}

function usage(): string {
  return [
    "Usage:",
    "  aster [--model <name>] [--system <text>] [--skill <name-or-path>] [--skill-dir <path>] [--json] [prompt]",
    "  aster --version",
    "  aster --help",
    "",
    "Chat:",
    "  Run without a prompt to enter multi-turn chat.",
    "  Type /exit or /quit to leave.",
    "  In chat, use /skill <name-or-path> to load a skill and /skills to list loaded skills.",
    "",
    "Skills:",
    "  --skill reads a local SKILL.md and injects it as guidance.",
    "  Default skill dirs: .codex/skills, packages/arbor-skills/skills, .installed-skills.",
    "",
    "Environment:",
    "  DEEPSEEK_API_KEY  DeepSeek API key.",
  ].join("\n");
}

if (process.argv[1] !== undefined && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  const result = await runCli(process.argv.slice(2), {
    env: process.env,
    fetch,
    writeStdout: (text) => {
      process.stdout.write(text);
    },
    writeStderr: (text) => {
      process.stderr.write(text);
    },
  });
  if (result.stdout.length > 0) {
    process.stdout.write(result.stdout);
  }
  if (result.stderr.length > 0) {
    process.stderr.write(result.stderr);
  }
  exit(result.exitCode);
}
