import { defineConfig, devices } from "@playwright/test";

const webPort = Number(process.env["ARBOR_E2E_WEB_PORT"] ?? 5174);
const webBaseURL = `http://127.0.0.1:${webPort}`;

export default defineConfig({
  testDir: "./e2e",
  timeout: 30_000,
  expect: {
    timeout: 5_000,
  },
  outputDir: "./test-results",
  reporter: "list",
  webServer: {
    command: `vite --config web.vite.config.ts --host 127.0.0.1 --port ${webPort} --strictPort`,
    url: webBaseURL,
    reuseExistingServer: true,
    timeout: 120_000,
  },
  projects: [
    {
      name: "web",
      testMatch: /web\.spec\.ts/,
      use: {
        ...devices["Desktop Chrome"],
        baseURL: webBaseURL,
      },
    },
    {
      name: "electron",
      testMatch: /electron\.spec\.ts/,
    },
  ],
});
