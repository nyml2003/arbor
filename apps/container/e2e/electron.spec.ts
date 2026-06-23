import { _electron as electron, expect, test } from "@playwright/test";
import { join, resolve } from "node:path";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";

const electronPackageDir = resolve(process.cwd(), "node_modules/electron");
const electronExecutableName = readFileSync(resolve(electronPackageDir, "path.txt"), "utf-8").trim();
const electronExecutablePath = resolve(electronPackageDir, "dist", electronExecutableName);
const sourceResumeJson = resolve(process.cwd(), "..", "..", "workspace", "show", "resume", "resume.json");

function createElectronEnv(workspaceRoot: string): Record<string, string> {
  const env: Record<string, string> = {};
  for (const [key, value] of Object.entries(process.env)) {
    if (value !== undefined) env[key] = value;
  }
  env["ARBOR_WORKSPACE_ROOT"] = workspaceRoot;
  return env;
}

function createTempWorkspace(): string {
  const root = mkdtempSync(join(tmpdir(), "arbor-resume-e2e-"));
  const resumeDir = join(root, "show", "resume");
  mkdirSync(resumeDir, { recursive: true });
  copyFileSync(sourceResumeJson, join(resumeDir, "resume.json"));
  return root;
}

test("electron shell renders the shared resume page from the workspace tree", async () => {
  const workspaceRoot = createTempWorkspace();
  const app = await electron.launch({
    executablePath: electronExecutablePath,
    args: [resolve(process.cwd(), "dist/main/index.js")],
    env: createElectronEnv(workspaceRoot),
  });
  const page = await app.firstWindow();

  try {
    await expect(page.getByRole("heading", { name: "Arbor Show" })).toBeVisible();
    await page.getByRole("button", { name: /show/ }).click();
    await page.getByRole("button", { name: /resume/ }).click();
    await expect(page.getByRole("heading", { name: "蒋钦禹" })).toBeVisible();
    await expect(page.getByText("项目经验")).toBeVisible();
    await page.getByRole("button", { name: "打印版" }).click();
    await expect(page.getByRole("button", { name: "返回简历" })).toBeVisible();
    await expect(page.getByRole("button", { name: /show/ })).not.toBeVisible();
    await expect(page.getByRole("heading", { name: "蒋钦禹" })).toBeVisible();
  } finally {
    await app.close();
    rmSync(workspaceRoot, { recursive: true, force: true });
  }
});

test("electron resume editor saves changes back to the workspace json", async () => {
  const workspaceRoot = createTempWorkspace();
  const resumeJsonPath = join(workspaceRoot, "show", "resume", "resume.json");
  const app = await electron.launch({
    executablePath: electronExecutablePath,
    args: [resolve(process.cwd(), "dist/main/index.js")],
    env: createElectronEnv(workspaceRoot),
  });
  const page = await app.firstWindow();

  try {
    await page.getByRole("button", { name: /show/ }).click();
    await page.getByRole("button", { name: /resume/ }).click();
    await page.getByRole("button", { name: "创作" }).click();
    await expect(page.getByRole("heading", { name: "创作简历" })).toBeVisible();
    await expect(page.getByRole("button", { name: "保存到 workspace" })).toBeEnabled();

    await page.getByLabel("姓名").fill("Electron 保存测试");
    await page.getByLabel("简历主题").selectOption("editorial");
    await page.getByRole("button", { name: "保存到 workspace" }).click();
    await expect(page.getByText("已保存到 workspace/show/resume/resume.json。")).toBeVisible();

    const saved = JSON.parse(readFileSync(resumeJsonPath, "utf-8")) as {
      theme?: string;
      profile?: { name?: string };
    };
    expect(saved.theme).toBe("editorial");
    expect(saved.profile?.name).toBe("Electron 保存测试");
  } finally {
    await app.close();
    rmSync(workspaceRoot, { recursive: true, force: true });
  }
});

test("electron memvfs demo connects to the daemon bridge", async () => {
  const workspaceRoot = createTempWorkspace();
  const app = await electron.launch({
    executablePath: electronExecutablePath,
    args: [resolve(process.cwd(), "dist/main/index.js")],
    env: createElectronEnv(workspaceRoot),
  });
  const page = await app.firstWindow();

  try {
    await page.getByRole("button", { name: "memvfs" }).click();
    await expect(page.getByRole("heading", { name: "memvfs" })).toBeVisible();
    await expect(page.locator("header").getByText(/daemon 127\.0\.0\.1:7878/)).toBeVisible({
      timeout: 30_000,
    });
    await expect(page.getByRole("button", { name: /file \/docs\/hello\.txt/ })).toBeVisible();

    await page.getByLabel("memvfs file content").fill("electron writes to daemon memory");
    await page.getByRole("button", { name: "Save" }).click();
    await page.getByRole("button", { name: "Read" }).click();
    await expect(page.getByLabel("memvfs file content")).toHaveValue(
      "electron writes to daemon memory",
    );
  } finally {
    await app.close();
    rmSync(workspaceRoot, { recursive: true, force: true });
  }
});
