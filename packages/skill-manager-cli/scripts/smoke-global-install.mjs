import { mkdtemp, readFile, rm } from "node:fs/promises";
import { existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { delimiter, join } from "node:path";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { spawnSync } from "node:child_process";

const packageDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = resolve(packageDir, "../..");
const coreDir = resolve(repoRoot, "packages/skill-manager-core");
const prefix = await mkdtemp(join(tmpdir(), "arbor-skill-manager-global-"));
const pnpmCli = resolvePnpmCli();
const npmCli = resolveNpmCli();
const corePackage = await readPackageJson(coreDir);
const cliPackage = await readPackageJson(packageDir);

try {
  run("pnpm", ["--dir", coreDir, "pack", "--pack-destination", prefix], repoRoot);
  run("pnpm", ["--dir", packageDir, "pack", "--pack-destination", prefix], repoRoot);

  const coreTgz = join(prefix, archiveName(corePackage));
  const cliTgz = join(prefix, archiveName(cliPackage));
  run("npm", ["install", "--global", "--prefix", prefix, coreTgz, cliTgz], repoRoot);

  const binDir = process.platform === "win32" ? prefix : join(prefix, "bin");
  const arborBin = process.platform === "win32" ? join(binDir, "arbor.cmd") : join(binDir, "arbor");
  const env = {
    ...process.env,
    PATH: `${binDir}${delimiter}${process.env.PATH ?? ""}`,
  };

  runBin(arborBin, ["--version"], repoRoot, env);
  runBin(arborBin, ["doctor", "--manifest", ".codex/arbor.skills.json", "--cwd", repoRoot], repoRoot, env);
} finally {
  await rm(prefix, { recursive: true, force: true });
}

async function readPackageJson(packageDirectory) {
  return JSON.parse(await readFile(join(packageDirectory, "package.json"), "utf8"));
}

function archiveName(packageJson) {
  return `${packageJson.name.replace(/^@/, "").replace("/", "-")}-${packageJson.version}.tgz`;
}

function run(command, args, cwd, env = process.env) {
  const result = spawnSync(process.execPath, [resolveTool(command), ...args], {
    cwd,
    env,
    stdio: "inherit",
  });

  if (result.status !== 0) {
    throw new Error(`Command failed: ${command} ${args.join(" ")}`);
  }
}

function runBin(command, args, cwd, env) {
  const result = process.platform === "win32"
    ? spawnSync("cmd.exe", ["/d", "/c", command, ...args], {
      cwd,
      env,
      stdio: "inherit",
    })
    : spawnSync(command, args, {
      cwd,
      env,
      stdio: "inherit",
    });

  if (result.status !== 0) {
    throw new Error(`Command failed: ${command} ${args.join(" ")}`);
  }
}

function resolveTool(command) {
  if (command === "pnpm") {
    return pnpmCli;
  }
  if (command === "npm") {
    return npmCli;
  }
  throw new Error(`Unsupported tool: ${command}`);
}

function resolvePnpmCli() {
  const npmExecPath = process.env.npm_execpath;
  if (npmExecPath !== undefined && npmExecPath.endsWith("pnpm.cjs") && existsSync(npmExecPath)) {
    return npmExecPath;
  }

  const candidate = join(dirname(process.execPath), "node_modules/pnpm/bin/pnpm.cjs");
  if (existsSync(candidate)) {
    return candidate;
  }

  throw new Error("Could not locate pnpm.cjs for smoke test.");
}

function resolveNpmCli() {
  const candidate = join(dirname(process.execPath), "node_modules/npm/bin/npm-cli.js");
  if (existsSync(candidate)) {
    return candidate;
  }

  throw new Error("Could not locate npm-cli.js for smoke test.");
}
