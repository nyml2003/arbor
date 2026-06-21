import { expect, test } from "@playwright/test";

test("web shell renders the shared home page without Electron globals", async ({ page }) => {
  const consoleErrors: string[] = [];

  page.on("console", (msg) => {
    if (msg.type() === "error") {
      consoleErrors.push(msg.text());
    }
  });

  await page.goto("/");

  await expect(page.getByRole("heading", { name: "Arbor Show" })).toBeVisible();
  await expect(page.getByText("Runtime: web")).toBeVisible();
  await expect(page.getByText("Workspace files: unsupported")).toBeVisible();
  expect(consoleErrors.join("\n")).not.toContain("getDefaultWorkspace");
  expect(consoleErrors.join("\n")).not.toContain("arborAPI");
});

test("web shell renders the shared resume page", async ({ page }) => {
  await page.goto("/show/resume");

  await expect(page.getByRole("heading", { name: "蒋钦禹" })).toBeVisible();
  await expect(page.getByRole("button", { name: "打印版" })).toBeVisible();
  await expect(page.getByRole("button", { name: "打印", exact: true })).toBeVisible();
  await expect(page.getByText("教育经历")).toBeVisible();
  await expect(page.getByText("项目经验")).toBeVisible();
});

test("web file tree routes resume data entries to the shared resume page", async ({ page }) => {
  await page.goto("/");

  await page.getByRole("button", { name: /show/ }).click();
  await page.getByRole("button", { name: /resume/ }).click();
  await expect(page.getByRole("heading", { name: "蒋钦禹" })).toBeVisible();

  await page.getByRole("button", { name: /resume\.json/ }).click();
  await expect(page.getByRole("heading", { name: "蒋钦禹" })).toBeVisible();
  await expect(page.getByText("当前运行时不支持读取此文件")).not.toBeVisible();
});

test("web resume print route renders without the app navigation", async ({ page }) => {
  await page.goto("/show/resume");
  await page.getByRole("button", { name: "打印版" }).click();

  await expect(page).toHaveURL(/\/show\/resume\/print$/);
  await expect(page.getByRole("heading", { name: "蒋钦禹" })).toBeVisible();
  await expect(page.getByRole("button", { name: "返回简历" })).toBeVisible();
  await expect(page.getByRole("button", { name: "打印", exact: true })).toBeVisible();
  await expect(page.getByText("Arbor Show")).not.toBeVisible();
  await expect(page.getByRole("button", { name: /show/ })).not.toBeVisible();

  const tagStyle = await page.getByTestId("resume-tag").first().evaluate((node) => {
    const style = window.getComputedStyle(node);
    return {
      backgroundColor: style.backgroundColor,
      color: style.color,
    };
  });
  expect(tagStyle.backgroundColor).not.toBe("rgb(31, 41, 55)");
  expect(tagStyle.backgroundColor).not.toBe("rgba(0, 0, 0, 0)");

  await page.emulateMedia({ media: "print" });
  await expect(page.getByTestId("resume-toolbar")).not.toBeVisible();
  await expect(page.getByTestId("resume-page")).toBeVisible();
});
