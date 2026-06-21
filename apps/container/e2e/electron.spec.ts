import { _electron as electron, expect, test } from "@playwright/test";
import { resolve } from "node:path";
import { readFileSync } from "node:fs";

const electronPackageDir = resolve(process.cwd(), "node_modules/electron");
const electronExecutableName = readFileSync(resolve(electronPackageDir, "path.txt"), "utf-8").trim();
const electronExecutablePath = resolve(electronPackageDir, "dist", electronExecutableName);

test("electron shell renders the shared resume page from the workspace tree", async () => {
  const app = await electron.launch({
    executablePath: electronExecutablePath,
    args: [resolve(process.cwd(), "dist/main/index.js")],
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
  }
});
