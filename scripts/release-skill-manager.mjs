import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const coreDir = join(repoRoot, "packages", "skill-manager-core");
const cliDir = join(repoRoot, "packages", "skill-manager-cli");
const pnpmTool = resolveNodeTool("pnpm");
const npmTool = resolveNodeTool("npm");
const exactSemver = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/;

const options = parseArgs(process.argv.slice(2));
const corePackage = await readPackageJson(coreDir);
const cliPackage = await readPackageJson(cliDir);
const packages = [
  { label: "core", directory: coreDir, packageJson: corePackage },
  { label: "cli", directory: cliDir, packageJson: cliPackage },
];

log(`Arbor skill manager release (${options.publish ? "publish" : "dry-run"})`);
validateRuntime();
validatePackagePair(corePackage, cliPackage);

if (options.publish) {
  checkGitClean(options.allowDirty);
  checkNpmLogin();
}

checkPublishedVersions(packages);
runVerification();
runPublishCommands(packages, options);

log(options.publish ? "Publish complete." : "Dry-run complete. Use pnpm publish:skill-manager to publish.");

function parseArgs(args) {
  const result = {
    publish: false,
    allowDirty: false,
    tag: "latest",
  };

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--publish") {
      result.publish = true;
      continue;
    }
    if (arg === "--dry-run") {
      result.publish = false;
      continue;
    }
    if (arg === "--allow-dirty") {
      result.allowDirty = true;
      continue;
    }
    if (arg === "--tag") {
      result.tag = requireValue(args, index, arg);
      index += 1;
      continue;
    }
    if (arg.startsWith("--tag=")) {
      result.tag = arg.slice("--tag=".length);
      continue;
    }
    if (arg === "--help" || arg === "-h") {
      printHelp();
      process.exit(0);
    }
    throw new Error(`Unknown argument: ${arg}`);
  }

  if (result.tag.length === 0) {
    throw new Error("Release tag cannot be empty.");
  }

  return result;
}

function requireValue(args, index, flag) {
  const value = args[index + 1];
  if (value === undefined || value.startsWith("--")) {
    throw new Error(`Missing value for ${flag}.`);
  }
  return value;
}

function printHelp() {
  console.log(`Usage:
  pnpm release:skill-manager
  pnpm publish:skill-manager
  pnpm release:skill-manager -- --publish --tag next

Options:
  --publish       Publish to npm. Without this flag the script runs npm publish --dry-run.
  --dry-run       Force dry-run mode.
  --tag <tag>     npm dist-tag. Defaults to latest.
  --allow-dirty   Allow real publish from a dirty git worktree.
`);
}

async function readPackageJson(packageDirectory) {
  return JSON.parse(await readFile(join(packageDirectory, "package.json"), "utf8"));
}

function validateRuntime() {
  const major = Number.parseInt(process.versions.node.split(".")[0] ?? "0", 10);
  if (!Number.isInteger(major) || major < 24) {
    throw new Error(`Node.js 24+ is required. Current version: ${process.version}`);
  }
}

function validatePackagePair(core, cli) {
  assertPackage(core, "@arbor/skill-manager-core");
  assertPackage(cli, "@arbor/skill-manager-cli");

  if (core.version !== cli.version) {
    throw new Error(`Core and CLI versions must match. core=${core.version}, cli=${cli.version}`);
  }

  const coreDependency = cli.dependencies?.["@arbor/skill-manager-core"];
  if (coreDependency !== "workspace:*") {
    throw new Error(`CLI must depend on core through workspace:*. Found: ${coreDependency ?? "<missing>"}`);
  }

  if (core.license === "UNLICENSED" || cli.license === "UNLICENSED") {
    log("Warning: package license is UNLICENSED. Public npm packages will be published without a reuse license.");
  }
}

function assertPackage(packageJson, expectedName) {
  if (packageJson.name !== expectedName) {
    throw new Error(`Unexpected package name. expected=${expectedName}, actual=${packageJson.name}`);
  }

  if (typeof packageJson.version !== "string" || !exactSemver.test(packageJson.version)) {
    throw new Error(`${expectedName} must use an exact SemVer version. Found: ${packageJson.version}`);
  }

  if (packageJson.publishConfig?.access !== "public") {
    throw new Error(`${expectedName} must set publishConfig.access to public.`);
  }
}

function checkGitClean(allowDirty) {
  const result = runCapture("git", ["status", "--porcelain"], repoRoot);
  if (result.status !== 0) {
    throw new Error("Could not read git status before publish.");
  }
  if (result.stdout.trim().length === 0) {
    return;
  }

  if (allowDirty) {
    log("Warning: publishing from a dirty git worktree because --allow-dirty was set.");
    return;
  }

  throw new Error("Refusing to publish from a dirty git worktree. Commit or stash changes, or pass --allow-dirty deliberately.");
}

function checkNpmLogin() {
  const result = runToolCapture(npmTool, ["whoami"], repoRoot);
  if (result.status !== 0) {
    throw new Error("npm login is required before publishing. Run npm login, then rerun this script.");
  }
  log(`npm user: ${result.stdout.trim()}`);
}

function checkPublishedVersions(packageInfos) {
  for (const info of packageInfos) {
    const spec = `${info.packageJson.name}@${info.packageJson.version}`;
    const result = runToolCapture(npmTool, ["view", spec, "version"], repoRoot);
    if (result.status === 0 && result.stdout.trim() === info.packageJson.version) {
      throw new Error(`${spec} is already published. Bump both package versions before releasing.`);
    }

    if (result.status === 0) {
      continue;
    }

    const output = `${result.stdout}\n${result.stderr}`;
    if (output.includes("E404") || output.includes("404")) {
      log(`${spec} is not published yet.`);
      continue;
    }

    throw new Error(`Could not check npm version for ${spec}.\n${output}`);
  }
}

function runVerification() {
  runStep("core tests", pnpmTool, ["--filter", "@arbor/skill-manager-core", "test"], repoRoot);
  runStep("cli tests", pnpmTool, ["--filter", "@arbor/skill-manager-cli", "test"], repoRoot);
  runStep("pack check", pnpmTool, ["--filter", "@arbor/skill-manager-cli", "pack:check"], repoRoot);
  runStep("temporary global install smoke", pnpmTool, ["--filter", "@arbor/skill-manager-cli", "install:global:local"], repoRoot);
}

function runPublishCommands(packageInfos, releaseOptions) {
  for (const info of packageInfos) {
    const args = ["publish", "--access", "public", "--tag", releaseOptions.tag];
    if (!releaseOptions.publish) {
      args.push("--dry-run", "--no-git-checks");
    } else if (releaseOptions.allowDirty) {
      args.push("--no-git-checks");
    }

    runStep(`${releaseOptions.publish ? "publish" : "publish dry-run"} ${info.packageJson.name}`, pnpmTool, args, info.directory);
  }
}

function runStep(label, tool, args, cwd) {
  log(label);
  const result = runTool(tool, args, cwd);
  if (result.status !== 0) {
    throw new Error(`Step failed: ${label}`);
  }
}

function runTool(tool, args, cwd) {
  return spawnSync(tool.command, [...tool.prefixArgs, ...args], {
    cwd,
    stdio: "inherit",
  });
}

function runToolCapture(tool, args, cwd) {
  return spawnSync(tool.command, [...tool.prefixArgs, ...args], {
    cwd,
    encoding: "utf8",
  });
}

function runCapture(command, args, cwd) {
  return spawnSync(command, args, {
    cwd,
    encoding: "utf8",
  });
}

function resolveNodeTool(command) {
  const fromNpmExecPath = process.env.npm_execpath;
  if (fromNpmExecPath !== undefined && fromNpmExecPath.endsWith(`${command}.cjs`) && existsSync(fromNpmExecPath)) {
    return { command: process.execPath, prefixArgs: [fromNpmExecPath] };
  }

  const candidate = join(dirname(process.execPath), "node_modules", command, "bin", `${command}-cli.js`);
  if (existsSync(candidate)) {
    return { command: process.execPath, prefixArgs: [candidate] };
  }

  if (command === "pnpm") {
    const pnpmCandidate = join(dirname(process.execPath), "node_modules", "pnpm", "bin", "pnpm.cjs");
    if (existsSync(pnpmCandidate)) {
      return { command: process.execPath, prefixArgs: [pnpmCandidate] };
    }
  }

  const executable = process.platform === "win32" ? `${command}.cmd` : command;
  return { command: executable, prefixArgs: [] };
}

function log(message) {
  console.log(`[release-skill-manager] ${message}`);
}
