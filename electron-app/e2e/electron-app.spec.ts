/**
 * Legitimate Electron E2E: real main process + renderer + IPC.
 * Does not mock get_prayer_times / get_config – drives shipped preload + main.
 */
import { test, expect, _electron as electron, type ElectronApplication, type Page } from "@playwright/test";
import path from "path";
import fs from "fs";

const appRoot = path.join(__dirname, "..");
const mainJs = path.join(appRoot, "dist", "main.js");
// Prefer explicit path – avoids broken postinstall path helper when binary is present.
const electronBinary = (() => {
  const direct = path.join(appRoot, "node_modules", "electron", "dist", "electron.exe");
  if (fs.existsSync(direct)) return direct;
  try {
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    return require("electron") as string;
  } catch {
    throw new Error(
      `Electron binary missing at ${direct}. Run: npm --prefix electron-app install electron`,
    );
  }
})();

let app: ElectronApplication;
let page: Page;

test.beforeAll(async () => {
  if (!fs.existsSync(mainJs)) {
    throw new Error(`Missing ${mainJs} – run: npm --prefix electron-app run build`);
  }
  const frontendIndex = path.join(appRoot, "..", "build", "index.html");
  if (!fs.existsSync(frontendIndex)) {
    throw new Error(`Missing frontend build at ${frontendIndex} – run: npm run build`);
  }

  app = await electron.launch({
    executablePath: electronBinary,
    args: ["."],
    cwd: appRoot,
    env: {
      ...process.env,
      // Unpackaged Electron loads this URL in isDev mode
      ELECTRON_RENDERER_URL: "http://127.0.0.1:4173",
      // Isolate userData so E2E never touches real config
      NANOPRAYER_E2E: "1",
    },
    timeout: 60_000,
  });

  // First window is usually the main BrowserWindow
  page = await app.firstWindow({ timeout: 45_000 });
  await page.waitForLoadState("domcontentloaded", { timeout: 45_000 }).catch(() => {
    // Fallback shell may still expose electronAPI after load error
  });
});

test.afterAll(async () => {
  if (app) {
    await app.close().catch(() => undefined);
  }
});

test("main process starts and exposes electronAPI in renderer", async () => {
  const hasApi = await page.evaluate(() => {
    return typeof (window as unknown as { electronAPI?: { invoke: unknown } }).electronAPI?.invoke === "function";
  });
  expect(hasApi).toBe(true);
});

test("get_config returns a valid AppConfig-shaped object", async () => {
  const config = await page.evaluate(async () => {
    const api = (window as unknown as { electronAPI: { invoke: (c: string) => Promise<unknown> } })
      .electronAPI;
    return api.invoke("get_config");
  });
  expect(config).toBeTruthy();
  expect(typeof config).toBe("object");
  const c = config as Record<string, unknown>;
  expect(c).toHaveProperty("reminders");
  expect(c).toHaveProperty("advanced");
  expect(c).toHaveProperty("hijri_offset");
  expect(c).toHaveProperty("calculation_method");
});

test("get_prayer_times returns six prayer fields and countdown metadata", async () => {
  const times = await page.evaluate(async () => {
    const api = (window as unknown as { electronAPI: { invoke: (c: string, a?: object) => Promise<unknown> } })
      .electronAPI;
    return api.invoke("get_prayer_times", {});
  });
  expect(times).toBeTruthy();
  const t = times as Record<string, unknown>;
  for (const key of ["fajr", "sunrise", "dhuhr", "asr", "maghrib", "isha"]) {
    expect(typeof t[key]).toBe("string");
    expect(String(t[key])).toMatch(/^\d{2}:\d{2}$/);
  }
  expect(t).toHaveProperty("date");
  expect(t).toHaveProperty("next_prayer");
  // minutes_to_next may be null after Isha on historical edge; when set must be >= 0
  if (t.minutes_to_next != null) {
    expect(Number(t.minutes_to_next)).toBeGreaterThanOrEqual(0);
  }
});

test("get_hijri_date returns non-gregorian year components", async () => {
  const hijri = await page.evaluate(async () => {
    const api = (window as unknown as { electronAPI: { invoke: (c: string, a?: object) => Promise<unknown> } })
      .electronAPI;
    return api.invoke("get_hijri_date", {});
  });
  const h = hijri as { year: number; month: number; day: number; formatted: string };
  expect(h.year).toBeLessThan(2000);
  expect(h.month).toBeGreaterThanOrEqual(1);
  expect(h.month).toBeLessThanOrEqual(12);
  expect(h.day).toBeGreaterThanOrEqual(1);
  expect(h.formatted.length).toBeGreaterThan(0);
});

test("desktop_get_version returns package version string", async () => {
  const version = await page.evaluate(async () => {
    const api = (window as unknown as { electronAPI: { invoke: (c: string) => Promise<unknown> } })
      .electronAPI;
    return api.invoke("desktop_get_version");
  });
  expect(typeof version).toBe("string");
  expect(String(version)).toMatch(/^\d+\.\d+\.\d+/);
});

test("process stays alive after IPC traffic (smoke soak 8s)", async () => {
  const before = Date.now();
  for (let i = 0; i < 5; i += 1) {
    await page.evaluate(async () => {
      const api = (window as unknown as { electronAPI: { invoke: (c: string) => Promise<unknown> } })
        .electronAPI;
      await api.invoke("get_prayer_times");
      await api.invoke("get_config");
    });
  }
  await page.waitForTimeout(8000);
  const stillAlive = await page.evaluate(() => true);
  expect(stillAlive).toBe(true);
  expect(Date.now() - before).toBeGreaterThanOrEqual(8000);
});
