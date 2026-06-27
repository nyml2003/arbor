#!/usr/bin/env node
import { resolve } from "node:path";
import { cwd as processCwd, exit } from "node:process";
import { fileURLToPath } from "node:url";
import {
  completeTask,
  createJsonFileTaskRepository,
  createTask,
  listTasks,
  randomTaskId,
  restoreTask,
  systemClock,
  taskId,
  updateTask,
  type Task,
  type TaskId,
  type TaskResult,
} from "@arbor/manage-core";

type TaskCommand = "create" | "list" | "update" | "complete" | "restore";

type CliOptions = Readonly<{
  command: "version" | "task";
  taskCommand: TaskCommand | null;
  args: ReadonlyArray<string>;
  cwd: string;
  storePath: string;
  json: boolean;
}>;

type CliOutput = Readonly<{
  exitCode: number;
  stdout: string;
  stderr: string;
}>;

type ParseResult =
  | Readonly<{ ok: true; options: CliOptions }>
  | Readonly<{ ok: false; message: string }>;

type CommandResult =
  | Readonly<{ command: "version"; version: string }>
  | Readonly<{ command: "create" | "update" | "complete" | "restore"; task: Task }>
  | Readonly<{ command: "list"; tasks: ReadonlyArray<Task> }>;

const packageVersion = "0.1.0";

export async function runCli(argv: ReadonlyArray<string>): Promise<CliOutput> {
  const parsed = parseArgs(argv);
  if (!parsed.ok) {
    return {
      exitCode: 2,
      stdout: "",
      stderr: `${parsed.message}\n${usage()}\n`,
    };
  }

  const result = await runCommand(parsed.options);
  if (!result.ok) {
    return {
      exitCode: result.error.code === "invalid-input" ? 2 : 1,
      stdout: parsed.options.json ? `${JSON.stringify({ ok: false, error: result.error }, null, 2)}\n` : "",
      stderr: parsed.options.json ? "" : `${result.error.message}\n`,
    };
  }

  return {
    exitCode: 0,
    stdout: formatSuccess(result.value, parsed.options.json),
    stderr: "",
  };
}

async function runCommand(options: CliOptions): Promise<TaskResult<CommandResult>> {
  if (options.command === "version") {
    return { ok: true, value: { command: "version", version: packageVersion } };
  }

  const repository = createJsonFileTaskRepository(resolve(options.cwd, options.storePath));
  const firstArg = options.args[0];

  if (options.taskCommand === "create") {
    if (firstArg === undefined) return invalid("create requires a title.");
    const created = await createTask({
      title: firstArg,
      repository,
      generateId: randomTaskId,
      now: systemClock,
    });
    return mapTaskResult(created, "create");
  }

  if (options.taskCommand === "list") {
    const listed = await listTasks({ repository });
    if (!listed.ok) return listed;
    return { ok: true, value: { command: "list", tasks: listed.value } };
  }

  const parsedId = parseTaskId(firstArg);
  if (!parsedId.ok) return parsedId;

  if (options.taskCommand === "update") {
    const title = options.args[1];
    if (title === undefined) return invalid("update requires a title.");
    return mapTaskResult(
      await updateTask({
        id: parsedId.value,
        title,
        repository,
        now: systemClock,
      }),
      "update",
    );
  }

  if (options.taskCommand === "complete") {
    return mapTaskResult(
      await completeTask({
        id: parsedId.value,
        repository,
        now: systemClock,
      }),
      "complete",
    );
  }

  return mapTaskResult(
    await restoreTask({
      id: parsedId.value,
      repository,
      now: systemClock,
    }),
    "restore",
  );
}

function parseArgs(argv: ReadonlyArray<string>): ParseResult {
  const args = [...argv];
  const firstToken = args.shift();

  if (firstToken === undefined || firstToken === "--help" || firstToken === "-h") {
    return { ok: false, message: "Expected a command." };
  }

  let command: "version" | "task";
  let taskCommand: TaskCommand | null = null;

  if (firstToken === "--version" || firstToken === "-v" || firstToken === "version") {
    command = "version";
  } else if (firstToken === "task") {
    command = "task";
    const commandToken = args.shift();
    if (
      commandToken !== "create" &&
      commandToken !== "list" &&
      commandToken !== "update" &&
      commandToken !== "complete" &&
      commandToken !== "restore"
    ) {
      return { ok: false, message: "Task command must be create, list, update, complete, or restore." };
    }
    taskCommand = commandToken;
  } else {
    return { ok: false, message: "Expected: arbor-manage <command>." };
  }

  let cwd = processCwd();
  let storePath = "workspace/manage/tasks.json";
  let json = false;
  const positional: string[] = [];

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index] ?? "";

    if (arg === "--cwd") {
      const value = args[index + 1];
      if (typeof value !== "string" || value.length === 0) {
        return { ok: false, message: "--cwd requires a value." };
      }
      cwd = value;
      index += 1;
      continue;
    }

    if (arg === "--store") {
      const value = args[index + 1];
      if (typeof value !== "string" || value.length === 0) {
        return { ok: false, message: "--store requires a value." };
      }
      storePath = value;
      index += 1;
      continue;
    }

    if (arg === "--json") {
      json = true;
      continue;
    }

    positional.push(arg);
  }

  if (command === "version" && (positional.length > 0 || json)) {
    return { ok: false, message: "version does not accept arguments or --json." };
  }

  if (taskCommand === "list" && positional.length > 0) {
    return { ok: false, message: "list does not accept positional arguments." };
  }

  return {
    ok: true,
    options: {
      command,
      taskCommand,
      args: positional,
      cwd,
      storePath,
      json,
    },
  };
}

function parseTaskId(value: string | undefined): TaskResult<TaskId> {
  if (value === undefined) {
    return invalid("A task id is required.");
  }
  return taskId(value);
}

function mapTaskResult(
  result: TaskResult<Task>,
  command: "create" | "update" | "complete" | "restore",
): TaskResult<CommandResult> {
  if (!result.ok) return result;
  return {
    ok: true,
    value: {
      command,
      task: result.value,
    },
  };
}

function invalid<T>(message: string): TaskResult<T> {
  return {
    ok: false,
    error: {
      code: "invalid-input",
      message,
      taskId: null,
    },
  };
}

function formatSuccess(result: CommandResult, json: boolean): string {
  if (json) {
    return `${JSON.stringify({ ok: true, ...result }, null, 2)}\n`;
  }

  if (result.command === "version") {
    return `${result.version}\n`;
  }

  if (result.command === "list") {
    if (result.tasks.length === 0) return "No tasks.\n";
    return `${result.tasks.map(formatTaskLine).join("\n")}\n`;
  }

  return `${result.command}: ${formatTaskLine(result.task)}\n`;
}

function formatTaskLine(task: Task): string {
  const status = task.status === "done" ? "done" : "todo";
  return `${task.id} [${status}] ${task.title}`;
}

function usage(): string {
  return [
    "Usage:",
    "  arbor-manage --version",
    "  arbor-manage task create <title> [--store workspace/manage/tasks.json] [--cwd <dir>] [--json]",
    "  arbor-manage task list [--store workspace/manage/tasks.json] [--cwd <dir>] [--json]",
    "  arbor-manage task update <id> <title> [--store workspace/manage/tasks.json] [--cwd <dir>] [--json]",
    "  arbor-manage task complete <id> [--store workspace/manage/tasks.json] [--cwd <dir>] [--json]",
    "  arbor-manage task restore <id> [--store workspace/manage/tasks.json] [--cwd <dir>] [--json]",
  ].join("\n");
}

if (process.argv[1] !== undefined && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  const result = await runCli(process.argv.slice(2));
  if (result.stdout.length > 0) {
    process.stdout.write(result.stdout);
  }
  if (result.stderr.length > 0) {
    process.stderr.write(result.stderr);
  }
  exit(result.exitCode);
}
