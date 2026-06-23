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
  await expect(page.getByText("Resume save: unsupported")).toBeVisible();
  expect(consoleErrors.join("\n")).not.toContain("getDefaultWorkspace");
  expect(consoleErrors.join("\n")).not.toContain("arborAPI");
});

test("web shell renders the shared resume page", async ({ page }) => {
  await page.goto("/show/resume");

  await expect(page.getByRole("heading", { name: "蒋钦禹" })).toBeVisible();
  await expect(page.getByRole("button", { name: "创作" })).toBeVisible();
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

test("web resume editor edits bundled data, switches theme, and downloads json", async ({ page }) => {
  await page.goto("/show/resume");
  await page.getByRole("button", { name: "创作" }).click();

  await expect(page.getByRole("heading", { name: "创作简历" })).toBeVisible();
  await expect(page).toHaveURL(/\/show\/resume\/edit$/);
  await expect(page.getByText("Web 版读取的是打包进网页的 resume.json")).toBeVisible();
  await expect(page.getByRole("button", { name: "保存到 workspace" })).toBeDisabled();

  await page.getByLabel("姓名").fill("蒋钦禹 Web 预览");
  await expect(page.getByTestId("resume-page").getByRole("heading", { name: "蒋钦禹 Web 预览" })).toBeVisible();

  await page.getByPlaceholder("输入标签后回车").last().fill("SolidJS");
  await page.getByRole("button", { name: "添加标签" }).last().click();
  await expect(page.getByTestId("resume-page").getByText("SolidJS")).toBeVisible();

  const beforeTheme = await page.getByTestId("resume-tag").first().evaluate((node) => {
    return window.getComputedStyle(node).backgroundColor;
  });
  await page.getByLabel("简历主题").selectOption("signal");
  const afterTheme = await page.getByTestId("resume-tag").first().evaluate((node) => {
    return window.getComputedStyle(node).backgroundColor;
  });
  expect(afterTheme).not.toBe(beforeTheme);

  const downloadPromise = page.waitForEvent("download");
  await page.getByRole("button", { name: "下载 JSON" }).click();
  const download = await downloadPromise;
  expect(download.suggestedFilename()).toBe("resume.json");
});

test("web memvfs demo mutates the in-memory file system", async ({ page }) => {
  await page.goto("/show/memvfs");

  await expect(page.getByRole("heading", { name: "memvfs" })).toBeVisible();
  await expect(page.locator("header").getByText("memory backend")).toBeVisible();
  await expect(page.getByRole("button", { name: /file \/docs\/hello\.txt/ })).toBeVisible();

  await page.getByLabel("New file").fill("/docs/playwright.txt");
  await page.getByRole("button", { name: "Write File" }).click();

  await page.getByLabel("memvfs file content").fill("playwright writes to memory");
  await page.getByRole("button", { name: "Save" }).click();
  await page.getByRole("button", { name: "Read" }).click();
  await expect(page.getByLabel("memvfs file content")).toHaveValue("playwright writes to memory");
  await expect(page.getByRole("button", { name: /file \/docs\/playwright\.txt/ })).toBeVisible();

  await page.getByRole("button", { name: "Truncate" }).click();
  await expect(page.getByLabel("memvfs file content")).toHaveValue("playwrig");

  await page.getByRole("button", { name: "Open" }).click();
  await expect(page.getByText(/open fd: 3/i)).toBeVisible();

  const renderedBlocks = await page
    .locator("article", { has: page.getByRole("heading", { name: "data blocks" }) })
    .locator("span[title^='block ']")
    .count();
  expect(renderedBlocks).toBeLessThan(80);
  await expect(page.getByRole("heading", { name: "data blocks" }).locator("..").getByText("256")).toBeVisible();
});

test("web shamrock page renders the Three battle scene", async ({ page }) => {
  await page.goto("/show/shamrock");

  await expect(page.getByRole("heading", { name: "What will Leafy do?" })).toBeVisible();
  await expect(page.getByRole("button", { name: /Vine Whip/ })).toBeVisible();
  await expect(page.getByLabel("Battlefield")).toBeVisible();

  const canvasStats = await page.getByTestId("shamrock-canvas").evaluate(async (node) => {
    const canvas = node as HTMLCanvasElement;
    const context = canvas.getContext("webgl2", { preserveDrawingBuffer: true });
    if (!context) {
      return { hasContext: false, hasPixels: false, hasMotion: false, width: 0, height: 0 };
    }

    const sampleCenter = () => {
      const pixels = new Uint8Array(4);
      const width = Math.max(1, context.drawingBufferWidth);
      const height = Math.max(1, context.drawingBufferHeight);
      context.readPixels(
        Math.floor(width / 2),
        Math.floor(height / 2),
        1,
        1,
        context.RGBA,
        context.UNSIGNED_BYTE,
        pixels,
      );
      return pixels;
    };

    const first = sampleCenter();
    await new Promise((resolve) => window.setTimeout(resolve, 420));
    const second = sampleCenter();

    return {
      hasContext: true,
      hasPixels: first.some((value) => value !== 0),
      hasMotion: first.some((value, index) => value !== second[index]),
      width: context.drawingBufferWidth,
      height: context.drawingBufferHeight,
    };
  });
  expect(canvasStats.hasContext).toBe(true);
  expect(canvasStats.hasPixels).toBe(true);
  expect(canvasStats.hasMotion).toBe(true);
  expect(canvasStats.width).toBeGreaterThan(500);
  expect(canvasStats.height).toBeGreaterThan(250);
  const desktopScreenshot = await page.screenshot();
  expect(desktopScreenshot.length).toBeGreaterThan(20_000);

  await page.setViewportSize({ width: 520, height: 760 });
  await expect(page.getByRole("heading", { name: "What will Leafy do?" })).toBeVisible();
  await expect(page.getByRole("button", { name: /Poison Powder/ })).toBeVisible();

  const narrowCanvasStats = await page.getByTestId("shamrock-canvas").evaluate((node) => {
    const canvas = node as HTMLCanvasElement;
    const context = canvas.getContext("webgl2", { preserveDrawingBuffer: true });
    if (!context) return { hasContext: false, hasPixels: false, width: 0, height: 0 };
    const pixels = new Uint8Array(4);
    context.readPixels(
      Math.max(0, Math.floor(context.drawingBufferWidth / 2)),
      Math.max(0, Math.floor(context.drawingBufferHeight / 2)),
      1,
      1,
      context.RGBA,
      context.UNSIGNED_BYTE,
      pixels,
    );
    return {
      hasContext: true,
      hasPixels: pixels.some((value) => value !== 0),
      width: context.drawingBufferWidth,
      height: context.drawingBufferHeight,
    };
  });
  expect(narrowCanvasStats.hasContext).toBe(true);
  expect(narrowCanvasStats.hasPixels).toBe(true);
  expect(narrowCanvasStats.width).toBeGreaterThan(240);
  expect(narrowCanvasStats.height).toBeGreaterThan(220);
  const narrowScreenshot = await page.screenshot();
  expect(narrowScreenshot.length).toBeGreaterThan(20_000);
});
