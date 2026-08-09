import { defineConfig } from "@playwright/test";
import path from "path";

/**
 * Electron E2E via Playwright's _electron launcher.
 * Requires: frontend build served (webServer) + main process tsc build.
 */
export default defineConfig({
  testDir: "./e2e",
  timeout: 90_000,
  expect: { timeout: 20_000 },
  fullyParallel: false,
  workers: 1,
  retries: process.env.CI ? 1 : 0,
  reporter: [["list"], ["html", { open: "never", outputFolder: "e2e-report" }]],
  outputDir: "e2e-results",
  use: {
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    video: "off",
  },
  // Serve static SvelteKit output so unpackaged Electron can load the UI.
  webServer: {
    command: "npx --yes serve ../build -l 4173 --no-port-switching",
    cwd: __dirname,
    url: "http://127.0.0.1:4173",
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
  metadata: {
    electronAppRoot: path.join(__dirname),
  },
});
