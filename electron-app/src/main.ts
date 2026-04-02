import {
  app,
  BrowserWindow,
  dialog,
  globalShortcut,
  ipcMain,
  Menu,
  nativeImage,
  Notification,
  shell,
  Tray,
} from "electron";
import * as fs from "fs";
import * as path from "path";
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

let mainWindow: any = null;
let alertWindow: any = null;
let tray: any = null;
let activeAlert: PrayerAlertPayload | null = null;
let playingAudio = false;
let isMainWindowVisible = true;

const firedReminders = new Set<string>();
let schedulerTimer: ReturnType<typeof setInterval> | null = null;

const prayerLogFile = () => path.join(app.getPath("userData"), "prayer-log.json");
const configFile = () => path.join(app.getPath("userData"), "config.json");

const isDev = !app.isPackaged;

const getAssetPath = (filename: string): string => {
  return isDev
    ? path.join(__dirname, "..", "..", "src-tauri", "assets", filename)
    : path.join(process.resourcesPath, "assets", filename);
};

const rendererDevUrl = process.env.ELECTRON_RENDERER_URL ?? "http://localhost:1420";

const prayerOrder: any[] = [
  Prayer.Fajr,
  Prayer.Sunrise,
  Prayer.Dhuhr,
  Prayer.Asr,
  Prayer.Maghrib,
  Prayer.Isha,
];

function getTargetWindow(): any {
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
  return loaded ?? JSON.parse(JSON.stringify(defaultConfig)) as AppConfig;
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

function getLocation(config: AppConfig): { lat: number; lng: number; name: string | null } {
  const loc = config.locations[config.current_location_index];
  if (loc) {
    return {
      lat: loc.coordinates.latitude,
      lng: loc.coordinates.longitude,
      name: loc.name,
    };
  }
  return { lat: 21.4225, lng: 39.8262, name: "Makkah" };
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

  const todayPrayers = [
    { name: "Fajr", time: times.fajr },
    { name: "Sunrise", time: times.sunrise },
    { name: "Dhuhr", time: times.dhuhr },
    { name: "Asr", time: times.asr },
    { name: "Maghrib", time: times.maghrib },
    { name: "Isha", time: times.isha },
  ];

  let currentPrayer: string | null = null;
  let nextPrayer: string | null = null;
  let minutesToNext: number | null = null;

  for (let i = 0; i < todayPrayers.length; i += 1) {
    const current = todayPrayers[i];
    const next = todayPrayers[i + 1];

    if (current && now >= current.time) {
      currentPrayer = current.name;
    }

    if (next && now < next.time) {
      nextPrayer = next.name;
      minutesToNext = Math.max(0, Math.floor((next.time.getTime() - now.getTime()) / 60000));
      break;
    }
  }

  const qiblaDirection = Qibla(coordinates);

  return {
    date: date.toISOString().slice(0, 10),
    location_name: loc.name,
    fajr: formatHM(times.fajr),
    sunrise: formatHM(times.sunrise),
    dhuhr: formatHM(times.dhuhr),
    asr: formatHM(times.asr),
    maghrib: formatHM(times.maghrib),
    isha: formatHM(times.isha),
    current_prayer: currentPrayer,
    next_prayer: nextPrayer,
    minutes_to_next: minutesToNext,
    qibla_direction: qiblaDirection,
  };
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

function ensureAlertWindow(): any {
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
    },
  });

  if (isDev) {
    void alertWindow.loadURL(`${rendererDevUrl}/alert`);
  } else {
    void alertWindow.loadFile(path.join(process.resourcesPath, "frontend", "alert", "index.html"));
  }

  return alertWindow;
}

function showAlert(payload: PrayerAlertPayload): void {
  activeAlert = payload;
  const win = ensureAlertWindow();
  if (win.isMinimized()) {
    win.restore();
  }
  win.show();
  emitEvent("prayer-alert", payload);
}

function createTray(): void {
  const iconPath = isDev
    ? path.join(__dirname, "..", "..", "src-tauri", "icons", "icon.ico")
    : path.join(process.resourcesPath, "icons", "icon.ico");
  const trayIcon = fs.existsSync(iconPath)
    ? nativeImage.createFromPath(iconPath)
    : nativeImage.createEmpty();

  tray = new Tray(trayIcon);
  const menu = Menu.buildFromTemplate([
    {
      label: "Show",
      click: () => {
        if (!mainWindow) {
          return;
        }
        mainWindow.show();
        mainWindow.focus();
      },
    },
    { type: "separator" },
    {
      label: "Quit",
      click: () => app.quit(),
    },
  ]);

  tray.setContextMenu(menu);
  tray.setToolTip("NanoPrayReminder");
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

function startScheduler(): void {
  if (schedulerTimer) {
    clearInterval(schedulerTimer);
  }
  triggerReminderCheck();
  schedulerTimer = setInterval(triggerReminderCheck, 60_000);
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
    },
  });

  mainWindow.once('ready-to-show', () => {
    mainWindow?.show();
  });

  if (isDev) {
    void mainWindow.loadURL(rendererDevUrl);
  } else {
    void mainWindow.loadFile(path.join(process.resourcesPath, "frontend", "index.html"));
  }

  mainWindow.on("close", (event: any) => {
    const config = getConfig();
    if (config.advanced.minimize_to_tray && !app.isQuitting) {
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

async function invokeCommand(command: string, args: Record<string, unknown>): Promise<unknown> {
  const config = getConfig();

  switch (command) {
    case "get_config":
      return config;
    case "save_config": {
      const next = args.config as AppConfig;
      saveConfig(next);
      startScheduler();
      return null;
    }
    case "search_cities": {
      const query = String(args.query ?? "").trim().toLowerCase();
      if (!query) {
        return [];
      }
      return citySeed.filter((city) => `${city.name} ${city.country}`.toLowerCase().includes(query));
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
    case "play_adhan":
    case "play_reminder_sound":
      playingAudio = true;
      return null;
    case "pause_audio":
    case "resume_audio":
      return null;
    case "stop_audio":
      playingAudio = false;
      return null;
    case "dismiss_alert": {
      activeAlert = null;
      playingAudio = false;
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
      playingAudio = false;
      emitEvent("statistics-updated", null);
      emitEvent("prayer-alert-dismissed", null);
      alertWindow?.hide();
      return null;
    }
    case "get_statistics": {
      const entries = getPrayerLog();
      const completed = entries.filter((entry) => entry.completed);
      const base = {
        label: "Current",
        start_date: new Date().toISOString().slice(0, 10),
        end_date: new Date().toISOString().slice(0, 10),
        completed_count: completed.length,
        expected_count: Math.max(completed.length, 1),
        completion_rate_percentage: completed.length ? 100 : 0,
        per_prayer_completion: {
          Fajr: 0,
          Sunrise: 0,
          Dhuhr: 0,
          Asr: 0,
          Maghrib: 0,
          Isha: 0,
        },
        per_prayer_completed: {
          Fajr: 0,
          Sunrise: 0,
          Dhuhr: 0,
          Asr: 0,
          Maghrib: 0,
          Isha: 0,
        },
        per_prayer_expected: {
          Fajr: 1,
          Sunrise: 1,
          Dhuhr: 1,
          Asr: 1,
          Maghrib: 1,
          Isha: 1,
        },
        timeline: [],
      };

      return {
        today: base,
        week: { ...base, label: "This Week" },
        month: { ...base, label: "This Month" },
        year: { ...base, label: "This Year" },
        all_time: { ...base, label: "All Time" },
        total_prayers_logged: completed.length,
        current_streak: 0,
        longest_streak: 0,
      };
    }
    case "desktop_check_update": {
      return null;
    }
    case "desktop_register_shortcut": {
      const shortcut = String(args.shortcut ?? "CommandOrControl+Shift+P");
      const already = globalShortcut.isRegistered(shortcut);
      if (!already) {
        globalShortcut.register(shortcut, () => {
          if (!mainWindow) {
            return;
          }
          if (mainWindow.isVisible()) {
            mainWindow.hide();
          } else {
            mainWindow.show();
            mainWindow.focus();
          }
        });
      }
      return { registered: true };
    }
    case "desktop_is_shortcut_registered": {
      const shortcut = String(args.shortcut ?? "CommandOrControl+Shift+P");
      return globalShortcut.isRegistered(shortcut);
    }
    case "desktop_open_dialog": {
      const result = await dialog.showOpenDialog(mainWindow ?? undefined, {
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
  ipcMain.handle("desktop:invoke", async (_event: unknown, payload: { command: string; args?: Record<string, unknown> }) => {
    return invokeCommand(payload.command, payload.args ?? {});
  });
}

app.on("before-quit", () => {
  app.isQuitting = true;
  stopScheduler();
  globalShortcut.unregisterAll();
});

declare global {
  namespace Electron {
    interface App {
      isQuitting?: boolean;
    }
  }
}

app.whenReady().then(() => {
  registerIpc();
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
