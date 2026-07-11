#!/usr/bin/env node
import { existsSync, realpathSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { cwd as processCwd, exit } from "node:process";
import { fileURLToPath } from "node:url";
import {
  installSkills,
  lintManifest,
  pruneSkills,
  SkillManagerError,
  type InstallReport,
  type PruneReport,
  type SkillDiagnostic,
} from "@arbor/skill-manager-core";

type SkillCommand = "lint" | "install" | "prune";
type TopLevelCommand = "skill" | "doctor" | "version";

type CliOptions = Readonly<{
  command: TopLevelCommand;
  skillCommand: SkillCommand | null;
  manifestPath: string;
  cwd: string;
  json: boolean;
  dryRun: boolean;
  pruneLock: boolean;
}>;

type PackageInfo = Readonly<{
  name: string;
  version: string;
}>;

type DoctorReport = Readonly<{
  ok: boolean;
  packageName: string;
  version: string;
  nodeVersion: string;
  binPath: string;
  cwd: string;
  manifestPath: string;
  manifestFound: boolean;
  diagnostics: ReadonlyArray<SkillDiagnostic>;
}>;

type CliResult =
  | Readonly<{ ok: true; command: "lint"; diagnostics: ReadonlyArray<SkillDiagnostic> }>
  | Readonly<{ ok: true; command: "install"; report: InstallReport }>
  | Readonly<{ ok: true; command: "prune"; report: PruneReport }>
  | Readonly<{ ok: true; command: "doctor"; report: DoctorReport }>
  | Readonly<{ ok: true; command: "version"; packageInfo: PackageInfo }>;

export async function runCli(argv: ReadonlyArray<string>): Promise<Readonly<{
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

  try {
    const result = await runCommand(parsed.options);
    const hasLintErrors = result.command === "lint"
      && result.diagnostics.some((diagnostic) => diagnostic.severity === "error");
    const hasDoctorErrors = result.command === "doctor" && !result.report.ok;

    return {
      exitCode: hasLintErrors || hasDoctorErrors ? 1 : 0,
      stdout: formatSuccess(result, parsed.options.json),
      stderr: "",
    };
  } catch (error) {
    if (error instanceof SkillManagerError) {
      return {
        exitCode: 1,
        stdout: parsed.options.json ? `${JSON.stringify({ ok: false, diagnostics: error.diagnostics }, null, 2)}\n` : "",
        stderr: parsed.options.json ? "" : formatDiagnostics(error.diagnostics),
      };
    }

    return {
      exitCode: 1,
      stdout: "",
      stderr: `${error instanceof Error ? error.message : "Unknown error"}\n`,
    };
  }
}

async function runCommand(options: CliOptions): Promise<CliResult> {
  if (options.command === "version") {
    return {
      ok: true,
      command: "version",
      packageInfo: await readPackageInfo(),
    };
  }

  if (options.command === "doctor") {
    return {
      ok: true,
      command: "doctor",
      report: await createDoctorReport({
        manifestPath: options.manifestPath,
        cwd: options.cwd,
      }),
    };
  }

  if (options.skillCommand === "lint") {
    return {
      ok: true,
      command: "lint",
      diagnostics: await lintManifest({
        manifestPath: options.manifestPath,
        cwd: options.cwd,
      }),
    };
  }

  if (options.skillCommand === "install") {
    return {
      ok: true,
      command: "install",
      report: await installSkills({
        manifestPath: options.manifestPath,
        cwd: options.cwd,
        dryRun: options.dryRun,
        pruneLock: options.pruneLock,
        nowIso: new Date().toISOString(),
      }),
    };
  }

  return {
    ok: true,
    command: "prune",
    report: await pruneSkills({
      manifestPath: options.manifestPath,
      cwd: options.cwd,
      dryRun: options.dryRun,
      nowIso: new Date().toISOString(),
    }),
  };
}

type ParseResult =
  | Readonly<{ ok: true; options: CliOptions }>
  | Readonly<{ ok: false; message: string }>;

function parseArgs(argv: ReadonlyArray<string>): ParseResult {
  const args = [...argv];
  const firstToken = args.shift();

  if (firstToken === undefined || firstToken === "--help" || firstToken === "-h") {
    return { ok: false, message: "Expected a command." };
  }

  let command: TopLevelCommand;
  let skillCommand: SkillCommand | null = null;

  if (firstToken === "--version" || firstToken === "-v" || firstToken === "version") {
    command = "version";
  } else if (firstToken === "doctor") {
    command = "doctor";
  } else if (firstToken === "skill") {
    command = "skill";
    const commandToken = args.shift();
    if (commandToken !== "lint" && commandToken !== "install" && commandToken !== "prune") {
      return { ok: false, message: "Command must be lint, install, or prune." };
    }
    skillCommand = commandToken;
  } else {
    return { ok: false, message: "Expected: arbor <command>." };
  }

  let manifestPath = "arbor.skills.json";
  let cwd = processCwd();
  let json = false;
  let dryRun = false;
  let pruneLock = false;

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index] ?? "";

    if (arg === "--manifest") {
      const value = args[index + 1];
      if (typeof value !== "string" || value.length === 0) {
        return { ok: false, message: "--manifest requires a value." };
      }
      manifestPath = value;
      index += 1;
      continue;
    }

    if (arg === "--cwd") {
      const value = args[index + 1];
      if (typeof value !== "string" || value.length === 0) {
        return { ok: false, message: "--cwd requires a value." };
      }
      cwd = value;
      index += 1;
      continue;
    }

    if (arg === "--json") {
      json = true;
      continue;
    }

    if (arg === "--dry-run") {
      dryRun = true;
      continue;
    }

    if (arg === "--prune-lock") {
      pruneLock = true;
      continue;
    }

    return { ok: false, message: `Unknown option: ${arg}` };
  }

  if (command !== "skill" && (dryRun || pruneLock)) {
    return { ok: false, message: `${command} does not accept --dry-run or --prune-lock.` };
  }

  if (command === "version" && json) {
    return { ok: false, message: "version does not accept --json." };
  }

  if (skillCommand === "lint" && (dryRun || pruneLock)) {
    return { ok: false, message: "lint does not accept --dry-run or --prune-lock." };
  }

  if (skillCommand === "prune" && pruneLock) {
    return { ok: false, message: "prune does not accept --prune-lock." };
  }

  return {
    ok: true,
    options: {
      command,
      skillCommand,
      manifestPath,
      cwd,
      json,
      dryRun,
      pruneLock,
    },
  };
}

function formatSuccess(result: CliResult, json: boolean): string {
  if (json && result.command === "doctor") {
    return `${JSON.stringify(result.report, null, 2)}\n`;
  }

  if (json) {
    return `${JSON.stringify(result, null, 2)}\n`;
  }

  if (result.command === "version") {
    return `${result.packageInfo.version}\n`;
  }

  if (result.command === "doctor") {
    return formatDoctorReport(result.report);
  }

  if (result.command === "lint") {
    if (result.diagnostics.length === 0) {
      return "No skill issues found.\n";
    }
    return formatDiagnostics(result.diagnostics);
  }

  if (result.command === "install") {
    const lines = result.report.installed.map((item) =>
      `${item.action}: ${item.id}@${item.version} -> ${item.targetPath}`,
    );
    return `${lines.join("\n")}${lines.length > 0 ? "\n" : ""}`;
  }

  const actionLines = result.report.actions.map((action) => {
    if (action.type === "remove-lock-entry") {
      return `${result.report.dryRun ? "would remove" : "removed"} lock entry: ${action.id}`;
    }
    if (action.type === "remove-empty-dir") {
      return `${result.report.dryRun ? "would remove" : "removed"} empty dir: ${action.path}`;
    }
    return `${result.report.dryRun ? "would remove" : "removed"} managed dir: ${action.path}`;
  });
  const reportLines = result.report.reports.map((line) => `reported: ${line}`);
  const lines = [...actionLines, ...reportLines];
  return `${lines.join("\n")}${lines.length > 0 ? "\n" : ""}`;
}

async function createDoctorReport(input: Readonly<{ manifestPath: string; cwd: string }>): Promise<DoctorReport> {
  const packageInfo = await readPackageInfo();
  const absoluteCwd = resolve(input.cwd);
  const manifestPath = resolve(absoluteCwd, input.manifestPath);
  const manifestFound = existsSync(manifestPath);
  const diagnostics: SkillDiagnostic[] = [];
  const nodeVersion = process.versions.node;

  if (!isNodeVersionSupported(nodeVersion)) {
    diagnostics.push({
      code: "unsupported-feature",
      severity: "error",
      message: `Node ${nodeVersion} is not supported.`,
      file: null,
      path: null,
      hint: "Install Node 24 or newer, then reinstall @arbor/skill-manager-cli.",
    });
  }

  if (!manifestFound) {
    diagnostics.push({
      code: "missing-file",
      severity: "warning",
      message: `No arbor.skills.json found at ${manifestPath}.`,
      file: manifestPath,
      path: null,
      hint: "Pass --manifest and --cwd when checking a project-specific skill manifest.",
    });
  }

  return {
    ok: !diagnostics.some((diagnostic) => diagnostic.severity === "error"),
    packageName: packageInfo.name,
    version: packageInfo.version,
    nodeVersion,
    binPath: fileURLToPath(import.meta.url),
    cwd: absoluteCwd,
    manifestPath,
    manifestFound,
    diagnostics,
  };
}

function formatDoctorReport(report: DoctorReport): string {
  const lines = [
    `Arbor Skill Manager ${report.version}`,
    `Node ${report.nodeVersion}`,
    `bin ${report.binPath}`,
    `cwd ${report.cwd}`,
    `manifest ${report.manifestFound ? "found" : "missing"}: ${report.manifestPath}`,
  ];

  if (report.diagnostics.length > 0) {
    lines.push(formatDiagnostics(report.diagnostics).trimEnd());
  }

  lines.push(report.ok ? "doctor ok" : "doctor failed");
  return `${lines.join("\n")}\n`;
}

async function readPackageInfo(): Promise<PackageInfo> {
  const packagePath = await findPackageJson(dirname(fileURLToPath(import.meta.url)));
  const parsed = JSON.parse(await readFile(packagePath, "utf8")) as Readonly<Record<string, unknown>>;
  const name = typeof parsed["name"] === "string" ? parsed["name"] : "@arbor/skill-manager-cli";
  const version = typeof parsed["version"] === "string" ? parsed["version"] : "0.0.0";
  return { name, version };
}

async function findPackageJson(startDir: string): Promise<string> {
  let current = startDir;
  for (let index = 0; index < 8; index += 1) {
    const candidate = join(current, "package.json");
    if (existsSync(candidate)) {
      return candidate;
    }
    const parent = dirname(current);
    if (parent === current) {
      break;
    }
    current = parent;
  }
  throw new Error("Could not locate package.json for @arbor/skill-manager-cli.");
}

function isNodeVersionSupported(version: string): boolean {
  const majorText = version.split(".")[0];
  if (majorText === undefined) {
    return false;
  }
  return Number.parseInt(majorText, 10) >= 24;
}

function formatDiagnostics(diagnostics: ReadonlyArray<SkillDiagnostic>): string {
  const lines = diagnostics.map((diagnostic) => {
    const location = diagnostic.file === null ? "" : ` ${diagnostic.file}`;
    const path = diagnostic.path === null ? "" : ` ${diagnostic.path}`;
    const hint = diagnostic.hint === null ? "" : `\n  hint: ${diagnostic.hint}`;
    return `${diagnostic.severity.toUpperCase()} ${diagnostic.code}${location}${path}: ${diagnostic.message}${hint}`;
  });
  return `${lines.join("\n")}${lines.length > 0 ? "\n" : ""}`;
}

function usage(): string {
  return [
    "Usage:",
    "  arbor --version",
    "  arbor doctor [--manifest arbor.skills.json] [--cwd <dir>] [--json]",
    "  arbor skill lint [--manifest arbor.skills.json] [--cwd <dir>] [--json]",
    "  arbor skill install [--manifest arbor.skills.json] [--cwd <dir>] [--dry-run] [--prune-lock] [--json]",
    "  arbor skill prune [--manifest arbor.skills.json] [--cwd <dir>] [--dry-run] [--json]",
  ].join("\n");
}

function isDirectInvocation(entryPath: string | undefined): boolean {
  if (entryPath === undefined) {
    return false;
  }

  try {
    return realpathSync.native(fileURLToPath(import.meta.url)) === realpathSync.native(resolve(entryPath));
  } catch {
    return false;
  }
}

if (isDirectInvocation(process.argv[1])) {
  const result = await runCli(process.argv.slice(2));
  if (result.stdout.length > 0) {
    process.stdout.write(result.stdout);
  }
  if (result.stderr.length > 0) {
    process.stderr.write(result.stderr);
  }
  exit(result.exitCode);
}
