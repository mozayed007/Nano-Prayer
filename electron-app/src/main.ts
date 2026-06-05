import {
  app,
  BrowserWindow,
  dialog,
  globalShortcut,
  ipcMain,
  Menu,
  nativeImage,
  net,
  Notification,
  protocol,
  session,
  shell,
  Tray,
} from "electron";
import * as fs from "fs";
import * as path from "path";
import { pathToFileURL } from "url";
import * as log from "electron-log";
import {
  CalculationMethod,
  Coordinates,
  Madhab,
  Prayer,
  PrayerTimes,
  Qibla,
} from "adhan";
import { defaultConfig } from "./defaults";
import type {
  AppConfig,
  PrayerAlertPayload,
  PrayerTimesResponse,
  ReminderConfig,
} from "./types";

type PrayerLogEntry = {
  date: string;
  prayer: string;
  completed: boolean;
};

type CityEntry = {
  name: string;
  country: string;
  country_code: string;
  latitude: number;
  longitude: number;
  timezone: string;
};

const GITHUB_RELEASES_URL =
  "https://api.github.com/repos/mozayed007/Nano-Prayer/releases/latest";

type UpdateCheckResult = {
  available: boolean;
  currentVersion: string;
  latestVersion: string;
  releaseUrl: string;
  releaseNotes: string;
  publishedAt: string;
};

function compareSemver(a: string, b: string): number {
  const strip = (s: string) => s.trim().replace(/^v/i, "");
  const pa = strip(a).split(/[.\-+]/).map((n) => parseInt(n, 10) || 0);
  const pb = strip(b).split(/[.\-+]/).map((n) => parseInt(n, 10) || 0);
  const len = Math.max(pa.length, pb.length);
  for (let i = 0; i < len; i += 1) {
    const na = pa[i] ?? 0;
    const nb = pb[i] ?? 0;
    if (na !== nb) return na - nb;
  }
  return 0;
}

let mainWindow: BrowserWindow | null = null;
let alertWindow: BrowserWindow | null = null;
let tray: Tray | null = null;
let audioWindow: BrowserWindow | null = null;
let activeAlert: PrayerAlertPayload | null = null;
let playingAudio = false;
let isMainWindowVisible = true;
let isQuitting = false;
let muted = false;

const firedReminders = new Set<string>();
let schedulerTimer: ReturnType<typeof setInterval> | null = null;

const configFile = () => path.join(app.getPath("userData"), "config.json");
const prayerLogFile = () => path.join(app.getPath("userData"), "prayer-log.json");

const isDev = !app.isPackaged;

const getAssetPath = (filename: string): string => {
  return isDev
    ? path.join(__dirname, "..", "..", "src-tauri", "assets", filename)
    : path.join(process.resourcesPath, "assets", filename);
};

const rendererDevUrl = process.env.ELECTRON_RENDERER_URL ?? "http://localhost:1420";

const prayerOrder = [
  Prayer.Fajr,
  Prayer.Sunrise,
  Prayer.Dhuhr,
  Prayer.Asr,
  Prayer.Maghrib,
  Prayer.Isha,
];

function getTargetWindow(): BrowserWindow | null {
  return BrowserWindow.getFocusedWindow() ?? mainWindow ?? alertWindow;
}

const citySeed = [
  { name: "Makkah", country: "Saudi Arabia", latitude: 21.4225, longitude: 39.8262, timezone: "Asia/Riyadh" },
  { name: "Madinah", country: "Saudi Arabia", latitude: 24.5247, longitude: 39.5692, timezone: "Asia/Riyadh" },
  { name: "Cairo", country: "Egypt", latitude: 30.0444, longitude: 31.2357, timezone: "Africa/Cairo" },
  { name: "London", country: "United Kingdom", latitude: 51.5074, longitude: -0.1278, timezone: "Europe/London" },
  { name: "New York", country: "United States", latitude: 40.7128, longitude: -74.006, timezone: "America/New_York" },
  { name: "Istanbul", country: "Turkey", latitude: 41.0082, longitude: 28.9784, timezone: "Europe/Istanbul" },
];

let cityDb: CityEntry[] = [];

function loadCityDatabase(): CityEntry[] {
  try {
    const filePath = getAssetPath("cities.json");
    const raw = fs.readFileSync(filePath, "utf8");
    const parsed = JSON.parse(raw) as CityEntry[];
    if (!Array.isArray(parsed)) {
      throw new Error("cities.json did not contain an array");
    }
    return parsed;
  } catch (err) {
    log.error("Failed to load city database, falling back to seed:", err);
    return citySeed.map((c) => ({ ...c, country_code: "" }));
  }
}

function searchCities(query: string): CityEntry[] {
  const q = query.trim().toLowerCase();
  if (!q) return [];
  const seen = new Set<number>();
  const results: CityEntry[] = [];

  const push = (idx: number) => {
    if (seen.has(idx)) return;
    seen.add(idx);
    results.push(cityDb[idx]);
  };

  for (let i = 0; i < cityDb.length; i += 1) {
    if (cityDb[i].name.toLowerCase() === q) push(i);
  }
  for (let i = 0; i < cityDb.length; i += 1) {
    if (cityDb[i].name.toLowerCase().includes(q)) push(i);
  }
  for (let i = 0; i < cityDb.length; i += 1) {
    if (cityDb[i].country.toLowerCase().includes(q)) push(i);
  }
  return results;
}

function readJson<T>(filePath: string, fallback: T): T {
  try {
    if (!fs.existsSync(filePath)) {
      return fallback;
    }
    const raw = fs.readFileSync(filePath, "utf8");
    return JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
}

function writeJson(filePath: string, value: unknown): void {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, JSON.stringify(value, null, 2), "utf8");
}

function getConfig(): AppConfig {
  const loaded = readJson<AppConfig | null>(configFile(), null);
  const config = loaded ?? {
    ...defaultConfig,
    locations: defaultConfig.locations.map((loc) => ({ ...loc })),
    reminders: Object.fromEntries(
      Object.entries(defaultConfig.reminders).map(([k, v]) => [k, { ...v }])
    ),
    prayer_adjustments: { ...defaultConfig.prayer_adjustments },
    audio: { ...defaultConfig.audio },
    notifications: { ...defaultConfig.notifications },
    appearance: { ...defaultConfig.appearance },
    advanced: { ...defaultConfig.advanced },
  };
  if (config.advanced.muted === undefined) {
    config.advanced.muted = false;
  }
  return config;
}

function setMuted(value: boolean): void {
  muted = value;
  const config = getConfig();
  config.advanced.muted = value;
  saveConfig(config);
  rebuildTrayMenu();
}

function saveConfig(config: AppConfig): void {
  writeJson(configFile(), config);
  app.setLoginItemSettings({ openAtLogin: !!config.advanced.auto_start });
}

function getPrayerLog(): PrayerLogEntry[] {
  return readJson<PrayerLogEntry[]>(prayerLogFile(), []);
}

function savePrayerLog(entries: PrayerLogEntry[]): void {
  writeJson(prayerLogFile(), entries);
}

function computeStatistics(config: AppConfig) {
  const entries = getPrayerLog();
  const completed = entries.filter((e) => e.completed);

  const prayerNames = ["Fajr", "Sunrise", "Dhuhr", "Asr", "Maghrib", "Isha"];

  function dateStr(d: Date): string {
    return d.toISOString().slice(0, 10);
  }

  function startOfWeek(d: Date): Date {
    const s = new Date(d);
    s.setDate(d.getDate() - d.getDay());
    s.setHours(0, 0, 0, 0);
    return s;
  }

  function startOfMonth(d: Date): Date {
    return new Date(d.getFullYear(), d.getMonth(), 1);
  }

  function startOfYear(d: Date): Date {
    return new Date(d.getFullYear(), 0, 1);
  }

  function computePeriodStats(startDate: Date, endDate: Date, label: string) {
    const sd = dateStr(startDate);
    const ed = dateStr(endDate);
    const today = dateStr(new Date());

    const periodEntries = entries.filter(
      (e) => e.date >= sd && e.date <= ed
    );
    const periodCompleted = periodEntries.filter((e) => e.completed);

    const perCompleted: Record<string, number> = {};
    const perExpected: Record<string, number> = {};
    const perCompletion: Record<string, number> = {};

    for (const name of prayerNames) {
      perCompleted[name] = 0;
      perExpected[name] = 0;
    }

    for (const entry of periodEntries) {
      const isPast = entry.date < today;
      if (isPast) {
        perExpected[entry.prayer] = (perExpected[entry.prayer] || 0) + 1;
      }
      if (entry.completed) {
        perCompleted[entry.prayer] = (perCompleted[entry.prayer] || 0) + 1;
      }
    }

    // For today, check which prayers have passed
    if (startDate <= new Date() && new Date() <= endDate) {
      const now = new Date();
      const times = buildPrayerTimesResponse(now, config);
      const timeKeys = ["fajr", "sunrise", "dhuhr", "asr", "maghrib", "isha"];
      for (let i = 0; i < prayerNames.length; i++) {
        const timeStr = prayerTimeByKey(times, timeKeys[i]);
        if (timeStr) {
          const prayerTime = new Date(`${times.date}T${timeStr}:00`);
          if (now >= prayerTime) {
            perExpected[prayerNames[i]] = (perExpected[prayerNames[i]] || 0) + 1;
          }
        }
      }
    }

    for (const name of prayerNames) {
      perCompletion[name] =
        perExpected[name] > 0
          ? Math.round((perCompleted[name] / perExpected[name]) * 100) / 100
          : 0;
    }

    const completedCount = periodCompleted.length;
    const expectedCount = Object.values(perExpected).reduce((a, b) => a + b, 0);

    return {
      label,
      start_date: sd,
      end_date: ed,
      completed_count: completedCount,
      expected_count: Math.max(expectedCount, 1),
      completion_rate_percentage:
        expectedCount > 0
          ? Math.round((completedCount / expectedCount) * 100)
          : 0,
      per_prayer_completion: perCompletion,
      per_prayer_completed: perCompleted,
      per_prayer_expected: expectedCount > 0 ? perExpected : {
        Fajr: 1, Sunrise: 1, Dhuhr: 1, Asr: 1, Maghrib: 1, Isha: 1,
      },
      timeline: buildTimeline(startDate, endDate, config),
    };
  }

  function buildTimeline(startDate: Date, endDate: Date, config: AppConfig) {
    const rangeDays = Math.ceil(
      (endDate.getTime() - startDate.getTime()) / (86400_000)
    );
    const points: Array<{
      label: string;
      completed_count: number;
      expected_count: number;
      completion_rate_percentage: number;
    }> = [];

    if (rangeDays <= 14) {
      // Per-day
      const cursor = new Date(startDate);
      while (cursor <= endDate) {
        const d = dateStr(cursor);
        const dayEntries = entries.filter((e) => e.date === d);
        const done = dayEntries.filter((e) => e.completed).length;
        const exp = Math.min(6, Math.max(1, dayEntries.length || 1));
        points.push({
          label: cursor.toLocaleDateString("en-US", { month: "short", day: "numeric" }),
          completed_count: done,
          expected_count: exp,
          completion_rate_percentage: exp > 0 ? Math.round((done / exp) * 100) : 0,
        });
        cursor.setDate(cursor.getDate() + 1);
      }
    } else if (rangeDays <= 62) {
      // Per-week
      let cursor = startOfWeek(new Date(startDate));
      while (cursor <= endDate) {
        const weekEnd = new Date(cursor);
        weekEnd.setDate(weekEnd.getDate() + 6);
        const ws = dateStr(cursor);
        const we = dateStr(weekEnd > endDate ? endDate : weekEnd);
        const weekEntries = entries.filter((e) => e.date >= ws && e.date <= we);
        const done = weekEntries.filter((e) => e.completed).length;
        const exp = Math.max(weekEntries.length, 1);
        points.push({
          label: `Week of ${cursor.toLocaleDateString("en-US", { month: "short", day: "numeric" })}`,
          completed_count: done,
          expected_count: exp,
          completion_rate_percentage: exp > 0 ? Math.round((done / exp) * 100) : 0,
        });
        cursor.setDate(cursor.getDate() + 7);
      }
    } else {
      // Per-month
      let cursor = startOfMonth(new Date(startDate));
      while (cursor <= endDate) {
        const monthEnd = new Date(cursor.getFullYear(), cursor.getMonth() + 1, 0);
        const ms = dateStr(cursor);
        const me = dateStr(monthEnd > endDate ? endDate : monthEnd);
        const monthEntries = entries.filter((e) => e.date >= ms && e.date <= me);
        const done = monthEntries.filter((e) => e.completed).length;
        const exp = Math.max(monthEntries.length, 1);
        points.push({
          label: cursor.toLocaleDateString("en-US", { month: "long", year: "numeric" }),
          completed_count: done,
          expected_count: exp,
          completion_rate_percentage: exp > 0 ? Math.round((done / exp) * 100) : 0,
        });
        cursor.setMonth(cursor.getMonth() + 1);
      }
    }

    return points;
  }

  function computeStreaks() {
    const today = dateStr(new Date());

    // Collect days that have entries
    const dayMap = new Map<string, { completed: number }>();
    for (const entry of entries) {
      if (!dayMap.has(entry.date)) {
        dayMap.set(entry.date, { completed: 0 });
      }
      if (entry.completed) {
        dayMap.get(entry.date)!.completed += 1;
      }
    }

    // Sort dates
    const sortedDates = Array.from(dayMap.keys()).sort();

    let currentStreak = 0;
    let longestStreak = 0;
    let runningStreak = 0;

    for (const d of sortedDates) {
      const stats = dayMap.get(d)!;
      if (stats.completed >= 1) {
        runningStreak += 1;
        longestStreak = Math.max(longestStreak, runningStreak);
      } else {
        runningStreak = 0;
      }
    }

    // Compute current streak: walk backward from today
    const cursor = new Date();
    while (true) {
      const d = dateStr(cursor);
      const stats = dayMap.get(d);
      if (stats && stats.completed >= 1) {
        currentStreak += 1;
      } else {
        break;
      }
      cursor.setDate(cursor.getDate() - 1);
    }

    return { currentStreak, longestStreak };
  }

  const now = new Date();
  const today = computePeriodStats(now, now, "Today");
  const weekStart = startOfWeek(now);
  const week = computePeriodStats(weekStart, now, "This Week");
  const monthStart = startOfMonth(now);
  const month = computePeriodStats(monthStart, now, "This Month");
  const yearStart = startOfYear(now);
  const year = computePeriodStats(yearStart, now, "This Year");

  // All time: from first entry to now
  const allDates = entries.map((e) => e.date).sort();
  const allStart = allDates.length > 0 ? new Date(allDates[0]) : now;
  const allTime = computePeriodStats(allStart, now, "All Time");

  const streaks = computeStreaks();

  return {
    today,
    week,
    month,
    year,
    all_time: allTime,
    total_prayers_logged: completed.length,
    current_streak: streaks.currentStreak,
    longest_streak: streaks.longestStreak,
  };
}

function getLocation(config: AppConfig): { lat: number; lng: number; name: string | null; timezone: string | null } {
  const loc = config.locations[config.current_location_index];
  if (loc) {
    return {
      lat: loc.coordinates.latitude,
      lng: loc.coordinates.longitude,
      name: loc.name,
      timezone: loc.timezone,
    };
  }
  return { lat: 21.4225, lng: 39.8262, name: "Makkah", timezone: "Asia/Riyadh" };
}

function parsePrayerKey(value: string): string | null {
  switch (value.trim().toLowerCase().replace(/[-_]/g, "")) {
    case "fajr":
      return "fajr";
    case "sunrise":
      return "sunrise";
    case "dhuhr":
    case "zuhr":
      return "dhuhr";
    case "asr":
      return "asr";
    case "maghrib":
      return "maghrib";
    case "isha":
    case "ishaa":
      return "isha";
    default:
      return null;
  }
}

function resolveMethod(name: string) {
  switch (name) {
    case "Egyptian":
      return CalculationMethod.Egyptian();
    case "Karachi":
      return CalculationMethod.Karachi();
    case "UmmAlQura":
      return CalculationMethod.UmmAlQura();
    case "Dubai":
      return CalculationMethod.Dubai();
    case "MoonsightingCommittee":
      return CalculationMethod.MoonsightingCommittee();
    case "NorthAmerica":
      return CalculationMethod.NorthAmerica();
    case "Kuwait":
      return CalculationMethod.Kuwait();
    case "Qatar":
      return CalculationMethod.Qatar();
    case "Singapore":
      return CalculationMethod.Singapore();
    case "Tehran":
      return CalculationMethod.Tehran();
    case "Turkey":
      return CalculationMethod.Turkey();
    case "MuslimWorldLeague":
    default:
      return CalculationMethod.MuslimWorldLeague();
  }
}

function prayerName(prayer: any): string {
  switch (prayer) {
    case Prayer.Fajr:
      return "Fajr";
    case Prayer.Sunrise:
      return "Sunrise";
    case Prayer.Dhuhr:
      return "Dhuhr";
    case Prayer.Asr:
      return "Asr";
    case Prayer.Maghrib:
      return "Maghrib";
    case Prayer.Isha:
      return "Isha";
    default:
      return "Unknown";
  }
}

function formatHM(date: Date): string {
  const h = String(date.getHours()).padStart(2, "0");
  const m = String(date.getMinutes()).padStart(2, "0");
  return `${h}:${m}`;
}

function buildPrayerTimesResponse(
  date: Date,
  config: AppConfig,
  latitude?: number,
  longitude?: number,
): PrayerTimesResponse {
  const loc = latitude != null && longitude != null
    ? { lat: latitude, lng: longitude, name: null }
    : getLocation(config);

  const params = resolveMethod(config.calculation_method);
  params.madhab = config.asr_madhab === "Hanafi" ? Madhab.Hanafi : Madhab.Shafi;

  const coordinates = new Coordinates(loc.lat, loc.lng);
  const times = new PrayerTimes(coordinates, date, params);
  const now = new Date();

  const adjustments = config.prayer_adjustments;
  const todayPrayers = [
    { key: "fajr", name: "Fajr", time: applyMinuteOffset(times.fajr, adjustments.fajr) },
    { key: "sunrise", name: "Sunrise", time: applyMinuteOffset(times.sunrise, adjustments.sunrise) },
    { key: "dhuhr", name: "Dhuhr", time: applyMinuteOffset(times.dhuhr, adjustments.dhuhr) },
    { key: "asr", name: "Asr", time: applyMinuteOffset(times.asr, adjustments.asr) },
    { key: "maghrib", name: "Maghrib", time: applyMinuteOffset(times.maghrib, adjustments.maghrib) },
    { key: "isha", name: "Isha", time: applyMinuteOffset(times.isha, adjustments.isha) },
  ];

  let currentPrayer: string | null = null;
  let nextPrayer: string | null = null;
  let minutesToNext: number | null = null;

  for (let i = 0; i < todayPrayers.length; i += 1) {
    const current = todayPrayers[i];

    if (now < current.time) {
      nextPrayer = current.name;
      minutesToNext = Math.max(0, Math.floor((current.time.getTime() - now.getTime()) / 60000));
      break;
    }

    if (now >= current.time) {
      currentPrayer = current.name;
    }
  }

  if (nextPrayer === "Fajr" && currentPrayer === null) {
    currentPrayer = "Isha";
  }

  if (nextPrayer === null) {
    const tomorrowFajr = new Date(todayPrayers[0].time);
    tomorrowFajr.setDate(tomorrowFajr.getDate() + 1);
    currentPrayer = "Isha";
    nextPrayer = "Fajr";
    minutesToNext = Math.max(0, Math.floor((tomorrowFajr.getTime() - now.getTime()) / 60000));
  }

  const qiblaDirection = Qibla(coordinates);

  return {
    date: date.toISOString().slice(0, 10),
    location_name: loc.name,
    fajr: formatHM(todayPrayers[0].time),
    sunrise: formatHM(todayPrayers[1].time),
    dhuhr: formatHM(todayPrayers[2].time),
    asr: formatHM(todayPrayers[3].time),
    maghrib: formatHM(todayPrayers[4].time),
    isha: formatHM(todayPrayers[5].time),
    current_prayer: currentPrayer,
    next_prayer: nextPrayer,
    minutes_to_next: minutesToNext,
    qibla_direction: qiblaDirection,
  };
}

function applyMinuteOffset(date: Date, minutes: number): Date {
  const adjusted = new Date(date);
  adjusted.setMinutes(adjusted.getMinutes() + (Number.isFinite(minutes) ? minutes : 0));
  return adjusted;
}

function emitEvent(event: string, payload: unknown, isCritical = false): void {
  if (!isMainWindowVisible && !isCritical) {
    return;
  }
  BrowserWindow.getAllWindows().forEach((window: any) => {
    window.webContents.send("desktop:event", { event, payload });
  });
}

function prayerTimeByKey(times: PrayerTimesResponse, prayerKey: string): string | null {
  switch (prayerKey) {
    case "fajr":
      return times.fajr;
    case "sunrise":
      return times.sunrise;
    case "dhuhr":
      return times.dhuhr;
    case "asr":
      return times.asr;
    case "maghrib":
      return times.maghrib;
    case "isha":
      return times.isha;
    default:
      return null;
  }
}

function ensureAlertWindow(): BrowserWindow | null {
  if (alertWindow && !alertWindow.isDestroyed()) {
    return alertWindow;
  }

  alertWindow = new BrowserWindow({
    width: 380,
    height: 220,
    frame: false,
    alwaysOnTop: true,
    skipTaskbar: true,
    show: false,
    backgroundColor: "#1a1a1a",
    webPreferences: {
      preload: path.join(__dirname, "preload.js"),
      contextIsolation: true,
      nodeIntegration: false,
      offscreen: false,
      enablePreferredSizeMode: false,
      webSecurity: true,
      allowRunningInsecureContent: false,
      sandbox: true,
    },
  });

  let alertReady = false;
  alertWindow.webContents.once("did-finish-load", () => {
    alertReady = true;
  });

  // Store ready state on the window for checking later
  (alertWindow as any).__alertReady = () => alertReady;

  if (isDev) {
    alertWindow.loadURL(`${rendererDevUrl}/alert`).catch((err: Error) => log.error("Failed to load alert window:", err));
  } else {
    alertWindow.loadURL('app://./alert/index.html').catch((err: Error) => log.error("Failed to load alert window:", err));
  }

  return alertWindow;
}

function showAlert(payload: PrayerAlertPayload): void {
  activeAlert = payload;
  const win = ensureAlertWindow();
  if (!win) return;

  const isReady = win.isDestroyed() ? false : ((win as any).__alertReady?.() ?? false);

  if (isReady) {
    if (win.isMinimized()) {
      win.restore();
    }
    win.show();
  } else {
    // Defer showing until content loads
    win.webContents.once("did-finish-load", () => {
      if (win.isDestroyed()) return;
      (win as any).__alertReady = () => true;
      if (win.isMinimized()) win.restore();
      win.show();
    });
  }

emitEvent("prayer-alert", payload);
}

function createTray(): void {
  const iconPath = isDev
    ? path.join(__dirname, "..", "..", "src-tauri", "icons", "icon.ico")
    : path.join(process.resourcesPath, "icons", "icon.ico");

  let trayIcon = nativeImage.createEmpty();
  if (fs.existsSync(iconPath)) {
    trayIcon = nativeImage.createFromPath(iconPath);
  } else {
    // Create a minimal fallback icon (16x16 green square) so the tray isn't invisible
    log.warn(`Tray icon not found at ${iconPath}, using fallback`);
    const buffer = Buffer.alloc(16 * 16 * 4); // RGBA
    for (let i = 0; i < 16 * 16; i++) {
      buffer[i * 4] = 0x00;     // R
      buffer[i * 4 + 1] = 0x80; // G (green fallback)
      buffer[i * 4 + 2] = 0x40; // B
      buffer[i * 4 + 3] = 0xFF; // A
    }
    trayIcon = nativeImage.createFromBuffer(buffer, { width: 16, height: 16 });
  }

  tray = new Tray(trayIcon);
  rebuildTrayMenu();
  tray.setToolTip("NanoPrayReminder");
}

function buildTrayMenu(): Menu {
  return Menu.buildFromTemplate([
    {
      label: "Show",
      click: () => {
        if (!mainWindow || mainWindow.isDestroyed()) return;
        mainWindow.show();
        mainWindow.focus();
      },
    },
    { type: "separator" },
    {
      label: "Mute Reminders",
      type: "checkbox",
      checked: muted,
      click: () => setMuted(!muted),
    },
    { type: "separator" },
    {
      label: "Quit",
      click: () => app.quit(),
    },
  ]);
}

function rebuildTrayMenu(): void {
  if (!tray) return;
  tray.setContextMenu(buildTrayMenu());
}

function updateTrayTooltip(times: PrayerTimesResponse): void {
  if (!tray) {
    return;
  }
  if (!times.next_prayer || times.minutes_to_next == null) {
    tray.setToolTip("No more prayers today");
    return;
  }

  const minutes = times.minutes_to_next;
  if (minutes <= 0) {
    tray.setToolTip(`Now: ${times.next_prayer}`);
    return;
  }
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  tray.setToolTip(h > 0 ? `${h}h ${m}m till ${times.next_prayer}` : `${m}m till ${times.next_prayer}`);
}

function maybeNotify(title: string, body: string): void {
  if (!Notification.isSupported()) {
    return;
  }
  new Notification({ title, body }).show();
}

function triggerReminderCheck(): void {
  const config = getConfig();
  const now = new Date();
  const times = buildPrayerTimesResponse(now, config);

  if (isMainWindowVisible || !config.advanced.minimize_to_tray) {
    updateTrayTooltip(times);
  }

  if (muted) {
    return;
  }

  prayerOrder.forEach((prayer) => {
    const key = prayerName(prayer).toLowerCase();
    const reminder = config.reminders[key] as ReminderConfig | undefined;
    if (!reminder || !reminder.enabled) {
      return;
    }

    const reminderTime = prayerTimeByKey(times, key);
    if (!reminderTime) {
      return;
    }
    const prayerTime = new Date(`${times.date}T${reminderTime}:00`);
    const diff = Math.floor((prayerTime.getTime() - now.getTime()) / 60000);

    const beforeKey = `${times.date}:${key}:before`;
    if (
      reminder.before_enabled &&
      reminder.minutes_before > 0 &&
      diff > 0 &&
      diff <= reminder.minutes_before &&
      !firedReminders.has(beforeKey)
    ) {
      const payload: PrayerAlertPayload = {
        prayer: prayerName(prayer),
        alert_type: "before",
        title: `${prayerName(prayer)} Reminder`,
        body: `${prayerName(prayer)} is in ${diff} minutes at ${reminderTime}`,
      };
      firedReminders.add(beforeKey);
      showAlert(payload);
    }

    const onTimeKey = `${times.date}:${key}:on_time`;
    if (diff <= 0 && diff > -2 && !firedReminders.has(onTimeKey)) {
      const payload: PrayerAlertPayload = {
        prayer: prayerName(prayer),
        alert_type: "on_time",
        title: `Time for ${prayerName(prayer)}`,
        body: `It is now time for ${prayerName(prayer)}`,
      };
      firedReminders.add(onTimeKey);
      showAlert(payload);
      if (reminder.show_notification) {
        maybeNotify(payload.title, payload.body);
      }
    }

    const afterKey = `${times.date}:${key}:after`;
    if (
      reminder.after_enabled &&
      reminder.minutes_after > 0 &&
      diff < 0 &&
      diff >= -reminder.minutes_after &&
      !firedReminders.has(afterKey)
    ) {
      const payload: PrayerAlertPayload = {
        prayer: prayerName(prayer),
        alert_type: "after",
        title: `${prayerName(prayer)} Passed`,
        body: `${prayerName(prayer)} was ${-diff} minutes ago at ${reminderTime}`,
      };
      firedReminders.add(afterKey);
      showAlert(payload);
    }
  });
}

function purgeStaleReminders(): void {
  const today = new Date().toISOString().slice(0, 10);
  for (const key of firedReminders) {
    if (!key.startsWith(today)) {
      firedReminders.delete(key);
    }
  }
}

function startScheduler(): void {
  if (schedulerTimer) {
    clearInterval(schedulerTimer);
  }
  triggerReminderCheck();
  schedulerTimer = setInterval(() => {
    triggerReminderCheck();
    purgeStaleReminders();
  }, 60_000);
}

function stopScheduler(): void {
  if (!schedulerTimer) {
    return;
  }
  clearInterval(schedulerTimer);
  schedulerTimer = null;
}

function createMainWindow(): void {
  mainWindow = new BrowserWindow({
    width: 1200,
    height: 800,
    minWidth: 800,
    minHeight: 600,
    show: false,
    backgroundColor: '#0f172a',
    paintWhenInitiallyHidden: false,
    webPreferences: {
      preload: path.join(__dirname, "preload.js"),
      contextIsolation: true,
      nodeIntegration: false,
      offscreen: false,
      enablePreferredSizeMode: false,
      webSecurity: true,
      allowRunningInsecureContent: false,
      sandbox: true,
    },
  });

  mainWindow.once('ready-to-show', () => {
    mainWindow?.show();
  });

  if (isDev) {
    mainWindow.loadURL(rendererDevUrl).catch((err: Error) => log.error("Failed to load main window:", err));
  } else {
    mainWindow.loadURL('app://./').catch((err: Error) => log.error("Failed to load main window:", err));
  }

  mainWindow.on("close", (event: any) => {
    const config = getConfig();
    if (config.advanced.minimize_to_tray && !isQuitting) {
      event.preventDefault();
      mainWindow?.hide();
    }
  });

  mainWindow.on("show", () => {
    isMainWindowVisible = true;
    const config = getConfig();
    const now = new Date();
    const times = buildPrayerTimesResponse(now, config);
    updateTrayTooltip(times);
  });

  mainWindow.on("hide", () => {
    isMainWindowVisible = false;
  });
}

function ensureAudioWindow(): BrowserWindow | null {
  if (audioWindow && !audioWindow.isDestroyed()) {
    return audioWindow;
  }
  try {
    audioWindow = new BrowserWindow({
      width: 1,
      height: 1,
      show: false,
      skipTaskbar: true,
      webPreferences: {
        preload: path.join(__dirname, "preload.js"),
        contextIsolation: true,
        nodeIntegration: false,
      },
    });
    audioWindow.loadURL(`data:text/html,
      <!DOCTYPE html>
      <html>
      <head><meta charset="utf-8"></head>
      <body>
      <audio id="player" loop></audio>
      <script>
      const player = document.getElementById("player");
      let onPlayDone = null;
      window.electronAPI.listen("audio:play", (payload) => {
        player.src = payload.path;
        player.volume = payload.volume ?? 0.7;
        player.play().catch(() => {});
        if (payload.onDone) {
          onPlayDone = payload.onDone;
        }
      });
      player.onended = () => {
        if (onPlayDone) {
          window.electronAPI.invoke("audio:done");
          onPlayDone = null;
        }
      };
      window.electronAPI.listen("audio:stop", () => {
        player.pause();
        player.currentTime = 0;
        player.src = "";
        onPlayDone = null;
      });
      window.electronAPI.listen("audio:pause", () => player.pause());
      window.electronAPI.listen("audio:resume", () => player.play().catch(() => {}));
      </script>
      </body>
      </html>
    `.replace(/\n\s*/g, "")).catch((err: Error) => log.error("Failed to load audio window:", err));
  } catch (err) {
    log.error("Failed to create audio window:", err);
    audioWindow = null;
  }
  return audioWindow;
}

function playAudioFile(filePath: string, volume: number): void {
  const win = ensureAudioWindow();
  if (!win) {
    return;
  }
  try {
    const data = fs.readFileSync(filePath);
    const ext = path.extname(filePath).toLowerCase().replace('.', '');
    const mimeMap: Record<string, string> = { mp3: 'audio/mpeg', wav: 'audio/wav', ogg: 'audio/ogg' };
    const mime = mimeMap[ext] || 'audio/mpeg';
    const base64 = `data:${mime};base64,${data.toString('base64')}`;
    playingAudio = true;
    win.webContents.send("audio:play", { path: base64, volume });
  } catch (err) {
    log.error(`Failed to play audio file ${filePath}:`, err);
  }
}

function stopAudio(): void {
  playingAudio = false;
  if (audioWindow && !audioWindow.isDestroyed()) {
    audioWindow.webContents.send("audio:stop");
  }
}

function setupAudioDoneHandler(): void {
  ipcMain.handle("audio:done", () => {
    playingAudio = false;
  });
}

async function invokeCommand(command: string, args: Record<string, unknown>): Promise<unknown> {
  const config = getConfig();

  switch (command) {
    case "get_config":
      return config;
    case "save_config": {
      try {
        const next = args.config as AppConfig;
        muted = next.advanced.muted ?? false;
        saveConfig(next);
        firedReminders.clear();
        startScheduler();
        return { ok: true };
      } catch (err) {
        log.error("Failed to save config:", err);
        return { ok: false, error: String(err) };
      }
    }
    case "search_cities": {
      const query = String(args.query ?? "");
      return searchCities(query);
    }
    case "get_prayer_times": {
      const latitude = typeof args.latitude === "number" ? args.latitude : undefined;
      const longitude = typeof args.longitude === "number" ? args.longitude : undefined;
      const result = buildPrayerTimesResponse(new Date(), config, latitude, longitude);
      updateTrayTooltip(result);
      return result;
    }
    case "get_monthly_prayer_times": {
      const year = Number(args.year);
      const month = Number(args.month);
      const daysInMonth = new Date(year, month, 0).getDate();
      const rows: PrayerTimesResponse[] = [];
      for (let day = 1; day <= daysInMonth; day += 1) {
        rows.push(buildPrayerTimesResponse(new Date(year, month - 1, day), config));
      }
      return rows;
    }
    case "get_qibla_direction": {
      const latitude = Number(args.latitude);
      const longitude = Number(args.longitude);
      const qibla = Qibla(new Coordinates(latitude, longitude));
      return {
        degrees: qibla,
        cardinal: qibla >= 337.5 || qibla < 22.5
          ? "N"
          : qibla < 67.5
            ? "NE"
            : qibla < 112.5
              ? "E"
              : qibla < 157.5
                ? "SE"
                : qibla < 202.5
                  ? "S"
                  : qibla < 247.5
                    ? "SW"
                    : qibla < 292.5
                      ? "W"
                      : "NW",
        distance_km: 0,
      };
    }
    case "get_hijri_date": {
      const offset = Number(args.offsetDays ?? 0);
      const d = new Date();
      d.setDate(d.getDate() + offset);
      const formatted = new Intl.DateTimeFormat("en-TN-u-ca-islamic", {
        day: "numeric",
        month: "long",
        year: "numeric",
      }).format(d);
      return {
        year: d.getFullYear(),
        month: d.getMonth() + 1,
        day: d.getDate(),
        month_name: formatted.split(" ")[1] ?? "",
        formatted,
        formatted_arabic: formatted,
      };
    }
    case "send_notification": {
      maybeNotify(String(args.title ?? "NanoPrayer"), String(args.body ?? ""));
      return null;
    }
    case "play_adhan": {
      const isFajr = !!(args.is_fajr ?? args.isFajr);
      const adhanPath = getAssetPath(isFajr ? "adhan_fajr.mp3" : "adhan.mp3");
      playAudioFile(adhanPath, config.audio.global_volume ?? 0.7);
      return null;
    }
    case "play_reminder_sound": {
      const soundPath = String(args.path ?? args.customPath ?? args.custom_path ?? getAssetPath("classic_alarm.mp3"));
      playAudioFile(soundPath, config.audio.global_volume ?? 0.7);
      return null;
    }
    case "pause_audio":
      if (audioWindow && !audioWindow.isDestroyed()) {
        audioWindow.webContents.send("audio:pause");
      }
      return null;
    case "resume_audio":
      if (audioWindow && !audioWindow.isDestroyed()) {
        audioWindow.webContents.send("audio:resume");
      }
      return null;
    case "stop_audio":
      stopAudio();
      return null;
    case "dismiss_alert": {
      activeAlert = null;
      stopAudio();
      emitEvent("prayer-alert-dismissed", null);
      alertWindow?.hide();
      return null;
    }
    case "get_active_alert":
      return activeAlert;
    case "mark_prayer_completed": {
      const prayer = String(args.prayer ?? "");
      const today = new Date().toISOString().slice(0, 10);
      const entries = getPrayerLog();
      entries.push({ date: today, prayer, completed: true });
      savePrayerLog(entries);
      activeAlert = null;
      stopAudio();
      emitEvent("statistics-updated", null);
      emitEvent("prayer-alert-dismissed", null);
      alertWindow?.hide();
      return null;
    }
    case "get_statistics": {
      return computeStatistics(config);
    }
    case "get_next_prayer": {
      const result = buildPrayerTimesResponse(new Date(), config);
      const time = result.next_prayer
        ? prayerTimeByKey(result, result.next_prayer.toLowerCase())
        : null;
      const location = getLocation(config);
      let nextPrayerTime = time ? new Date(`${result.date}T${time}:00`) : null;
      if (nextPrayerTime && nextPrayerTime.getTime() <= Date.now() && result.next_prayer === "Fajr") {
        nextPrayerTime.setDate(nextPrayerTime.getDate() + 1);
      }
      return {
        date: result.date,
        location: {
          name: result.location_name,
          latitude: location.lat,
          longitude: location.lng,
          timezone: location.timezone,
        },
        current_prayer: result.current_prayer,
        next_prayer: result.next_prayer,
        next_prayer_time: nextPrayerTime ? nextPrayerTime.toISOString() : null,
        minutes_to_next: result.minutes_to_next,
      };
    }
    case "set_reminders_muted": {
      setMuted(!!args.muted);
      return null;
    }
    case "update_reminder_settings": {
      const prayer = parsePrayerKey(String(args.prayer ?? ""));
      const reminder = args.reminder as ReminderConfig;
      if (!prayer) {
        throw new Error(`Invalid prayer: ${String(args.prayer ?? "")}`);
      }
      const current = getConfig();
      current.reminders[prayer] = reminder;
      saveConfig(current);
      firedReminders.clear();
      startScheduler();
      return null;
    }
    case "desktop_check_update": {
      try {
        const response = await net.fetch(GITHUB_RELEASES_URL, {
          headers: { Accept: "application/vnd.github+json", "User-Agent": "NanoPrayReminder-Electron" },
        });
        if (!response.ok) {
          log.warn(`Update check failed: ${response.status} ${response.statusText}`);
          return null;
        }
        const data = (await response.json()) as {
          tag_name?: string;
          html_url?: string;
          body?: string;
          published_at?: string;
          draft?: boolean;
          prerelease?: boolean;
        };
        if (!data.tag_name || data.draft) {
          return null;
        }
        const currentVersion = app.getVersion();
        const latestVersion = data.tag_name.replace(/^v/i, "");
        const result: UpdateCheckResult = {
          available: compareSemver(latestVersion, currentVersion) > 0,
          currentVersion,
          latestVersion,
          releaseUrl: data.html_url ?? "",
          releaseNotes: data.body ?? "",
          publishedAt: data.published_at ?? "",
        };
        return result;
      } catch (err) {
        log.warn("Update check error:", err);
        return null;
      }
    }
    case "desktop_register_shortcut": {
      const shortcut = String(args.shortcut ?? "CommandOrControl+Shift+P");
      const already = globalShortcut.isRegistered(shortcut);
      if (already) {
        return { registered: true, error: null };
      }
      const ok = globalShortcut.register(shortcut, () => {
        if (!mainWindow) return;
        if (mainWindow.isVisible()) {
          mainWindow.hide();
        } else {
          mainWindow.show();
          mainWindow.focus();
        }
      });
      if (!ok) {
        log.warn(`Failed to register global shortcut: ${shortcut}`);
        return { registered: false, error: `Shortcut "${shortcut}" is already in use by another application` };
      }
      return { registered: true, error: null };
    }
    case "desktop_is_shortcut_registered": {
      const shortcut = String(args.shortcut ?? "CommandOrControl+Shift+P");
      return globalShortcut.isRegistered(shortcut);
    }
    case "desktop_open_dialog": {
      const result = await dialog.showOpenDialog(mainWindow!, {
        properties: ["openFile"],
        filters: [{ name: "Audio Files", extensions: ["mp3", "wav", "ogg"] }],
      });
      if (result.canceled || result.filePaths.length === 0) {
        return null;
      }
      return result.filePaths[0];
    }
    case "desktop_open_external": {
      const url = String(args.url ?? "");
      if (url) {
        await shell.openExternal(url);
      }
      return null;
    }
    case "desktop_get_version":
      return app.getVersion();
    case "desktop_set_autostart": {
      const enabled = !!args.enabled;
      app.setLoginItemSettings({ openAtLogin: enabled });
      const current = getConfig();
      current.advanced.auto_start = enabled;
      saveConfig(current);
      return null;
    }
    case "desktop_window_hide": {
      getTargetWindow()?.hide();
      return null;
    }
    case "desktop_window_show": {
      getTargetWindow()?.show();
      return null;
    }
    case "desktop_window_focus": {
      getTargetWindow()?.focus();
      return null;
    }
    case "desktop_window_is_visible": {
      const win = getTargetWindow();
      return win ? win.isVisible() : true;
    }
    default:
      throw new Error(`Unknown command: ${command}`);
  }
}

function registerIpc(): void {
  ipcMain.handle("desktop:invoke", async (event, payload: { command: string; args?: Record<string, unknown> }) => {
    const senderUrl = event.senderFrame?.url ?? '';
    if (!senderUrl.startsWith('file://') && !senderUrl.startsWith('data:') && !senderUrl.startsWith('http://localhost:') && !senderUrl.startsWith('app://')) {
      log.warn(`Rejected IPC from untrusted sender: ${senderUrl}`);
      return null;
    }
    return invokeCommand(payload.command, payload.args ?? {});
  });
}

app.on("before-quit", () => {
  log.info("NanoPrayer Electron shutting down");
  isQuitting = true;
  stopScheduler();
  globalShortcut.unregisterAll();
});

protocol.registerSchemesAsPrivileged([
  { scheme: 'app', privileges: { standard: true, supportFetchAPI: true, corsEnabled: true, stream: true } }
]);

app.whenReady().then(() => {
  cityDb = loadCityDatabase();
  log.info(`Loaded ${cityDb.length} cities from database`);

  muted = getConfig().advanced.muted;

  const frontendRoot = isDev ? path.join(__dirname, "..", "..", "build") : path.join(process.resourcesPath, "frontend");
  protocol.handle('app', (request) => {
    const url = new URL(request.url);
    let relativePath = url.pathname.replace(/^\//, '');
    if (!relativePath) relativePath = '200.html';
    const resolved = path.resolve(path.join(frontendRoot, relativePath));
    const rootResolved = path.resolve(frontendRoot);
    const resolvedNorm = path.normalize(resolved).toLowerCase();
    const rootNorm = path.normalize(rootResolved).toLowerCase();
    if (!resolvedNorm.startsWith(rootNorm + path.sep) && resolvedNorm !== rootNorm) {
      log.warn(`Blocked path-traversal request: ${resolved}`);
      return new Response('Forbidden', { status: 403 });
    }
    return net.fetch(pathToFileURL(resolved).href);
  });
  Menu.setApplicationMenu(null);
  log.info("NanoPrayer Electron starting");
  setupAudioDoneHandler();
  registerIpc();
  app.on('web-contents-created', (_event, contents) => {
    contents.on('will-navigate', (event, _navigationUrl) => {
      event.preventDefault();
    });
    contents.setWindowOpenHandler(() => {
      return { action: 'deny' };
    });
  });
  createMainWindow();
  createTray();
  startScheduler();

  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createMainWindow();
    }
  });
});

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    // keep tray experience; explicit quit from tray only
  }
});
