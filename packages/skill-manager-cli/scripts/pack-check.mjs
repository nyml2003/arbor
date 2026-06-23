import { mkdtemp, readFile, rm } from "node:fs/promises";
import { existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { dirname } from "node:path";
import { spawnSync } from "node:child_process";

const packageDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = resolve(packageDir, "../..");
const coreDir = resolve(repoRoot, "packages/skill-manager-core");
const destination = await mkdtemp(join(tmpdir(), "arbor-skill-manager-pack-"));
const pnpmCli = resolvePnpmCli();
const corePackage = await readPackageJson(coreDir);
const cliPackage = await readPackageJson(packageDir);

try {
  run("pnpm", ["--dir", coreDir, "pack", "--pack-destination", destination], repoRoot);
  run("pnpm", ["--dir", packageDir, "pack", "--pack-destination", destination], repoRoot);

  const expected = [
    archiveName(corePackage),
    archiveName(cliPackage),
  ];

  for (const file of expected) {
    const path = join(destination, file);
    if (!existsSync(path)) {
      throw new Error(`Expected package was not created: ${path}`);
    }
  }
} finally {
  await rm(destination, { recursive: true, force: true });
}

async function readPackageJson(packageDirectory) {
  return JSON.parse(await readFile(join(packageDirectory, "package.json"), "utf8"));
}

function archiveName(packageJson) {
  return `${packageJson.name.replace(/^@/, "").replace("/", "-")}-${packageJson.version}.tgz`;
}

function run(command, args, cwd) {
  const result = spawnSync(process.execPath, [resolveTool(command), ...args], {
    cwd,
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

  throw new Error("Could not locate pnpm.cjs for pack check.");
}
