/**
 * Pure helpers shared by Electron main process logic.
 * Kept free of Electron imports so unit tests can import the compiled JS.
 */

export type HijriDateParts = {
  year: number;
  month: number;
  day: number;
  month_name: string;
  formatted: string;
  formatted_arabic: string;
};

const HIJRI_MONTHS_EN = [
  "Muharram",
  "Safar",
  "Rabi al-Awwal",
  "Rabi al-Thani",
  "Jumada al-Awwal",
  "Jumada al-Thani",
  "Rajab",
  "Shaban",
  "Ramadan",
  "Shawwal",
  "Dhu al-Qadah",
  "Dhu al-Hijjah",
] as const;

const HIJRI_MONTHS_AR = [
  "محرم",
  "صفر",
  "ربيع الأول",
  "ربيع الثاني",
  "جمادى الأولى",
  "جمادى الثانية",
  "رجب",
  "شعبان",
  "رمضان",
  "شوال",
  "ذو القعدة",
  "ذو الحجة",
] as const;

const ARABIC_DIGITS = ["٠", "١", "٢", "٣", "٤", "٥", "٦", "٧", "٨", "٩"] as const;

/** Local calendar day as YYYY-MM-DD (not UTC). */
export function localDateStr(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

/**
 * Civil YYYY-MM-DD in an IANA zone (DST / summer time aware via Intl).
 * Falls back to system local when zone is missing/invalid.
 */
export function civilDateStrInZone(d: Date, timeZone?: string | null): string {
  if (!timeZone || !String(timeZone).trim()) {
    return localDateStr(d);
  }
  try {
    const parts = new Intl.DateTimeFormat("en-CA", {
      timeZone: String(timeZone).trim(),
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
    }).formatToParts(d);
    const y = parts.find((p) => p.type === "year")?.value;
    const m = parts.find((p) => p.type === "month")?.value;
    const day = parts.find((p) => p.type === "day")?.value;
    if (y && m && day) return `${y}-${m}-${day}`;
  } catch {
    // invalid zone
  }
  return localDateStr(d);
}

/** Format instant as HH:MM in IANA zone (DST-aware). */
export function formatHmInZone(d: Date, timeZone?: string | null): string {
  if (!timeZone || !String(timeZone).trim()) {
    const h = String(d.getHours()).padStart(2, "0");
    const m = String(d.getMinutes()).padStart(2, "0");
    return `${h}:${m}`;
  }
  try {
    const parts = new Intl.DateTimeFormat("en-GB", {
      timeZone: String(timeZone).trim(),
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    }).formatToParts(d);
    const h = parts.find((p) => p.type === "hour")?.value ?? "00";
    const m = parts.find((p) => p.type === "minute")?.value ?? "00";
    return `${h.padStart(2, "0")}:${m.padStart(2, "0")}`;
  } catch {
    const h = String(d.getHours()).padStart(2, "0");
    const m = String(d.getMinutes()).padStart(2, "0");
    return `${h}:${m}`;
  }
}

/**
 * Quiet hours from whole-hour config (matches core `is_quiet_hour`).
 * Range is [start, end); supports midnight wrap. Equal start/end = never quiet.
 */
export function isQuietHour(hour: number, startHour: number, endHour: number): boolean {
  const h = Math.min(23, Math.max(0, Math.trunc(hour)));
  const start = Math.min(23, Math.max(0, Math.trunc(startHour)));
  const end = Math.min(23, Math.max(0, Math.trunc(endHour)));
  if (start === end) return false;
  if (start < end) return h >= start && h < end;
  return h >= start || h < end;
}

/** Deep-merge partial persisted config onto defaults (old installs miss new fields). */
export function mergeAppConfig<T>(defaults: T, loaded: unknown): T {
  if (loaded == null || typeof loaded !== "object" || Array.isArray(loaded)) {
    return structuredClone(defaults);
  }
  if (defaults == null || typeof defaults !== "object" || Array.isArray(defaults)) {
    return structuredClone(defaults);
  }
  const out = structuredClone(defaults) as Record<string, unknown>;
  for (const [key, value] of Object.entries(loaded as Record<string, unknown>)) {
    if (value === undefined || value === null) continue;
    const baseVal = out[key];
    if (
      value &&
      typeof value === "object" &&
      !Array.isArray(value) &&
      baseVal &&
      typeof baseVal === "object" &&
      !Array.isArray(baseVal)
    ) {
      out[key] = mergeAppConfig(baseVal, value);
    } else {
      out[key] = value;
    }
  }
  return out as T;
}

export function toArabicNumerals(n: number): string {
  return String(Math.trunc(n))
    .split("")
    .map((ch) => {
      if (ch >= "0" && ch <= "9") {
        return ARABIC_DIGITS[Number(ch)];
      }
      return ch;
    })
    .join("");
}

function isLeapYearInCycle(yearInCycle: number): boolean {
  return [2, 5, 7, 10, 13, 16, 18, 21, 24, 26, 29].includes(yearInCycle);
}

function hijriMonthLen(month: number, yearInCycle: number): number {
  if (month % 2 !== 0 || (month === 12 && isLeapYearInCycle(yearInCycle))) {
    return 30;
  }
  return 29;
}

/**
 * Tabular Islamic calendar conversion (same epoch as nano-pray-core).
 * Epoch: 1 Muharram 1 AH = 622-07-19 proleptic Gregorian.
 */
export function gregorianToHijri(date: Date): { year: number; month: number; day: number } {
  const epoch = Date.UTC(622, 6, 19);
  const utc = Date.UTC(date.getFullYear(), date.getMonth(), date.getDate());
  let daysSinceEpoch = Math.floor((utc - epoch) / 86_400_000);

  if (daysSinceEpoch < 0) {
    return { year: 1, month: 1, day: 1 };
  }

  const cycles = Math.floor(daysSinceEpoch / 10631);
  let remainingDays = daysSinceEpoch % 10631;

  let yearInCycle = 1;
  for (let y = 1; y <= 30; y += 1) {
    const yearLen = isLeapYearInCycle(y) ? 355 : 354;
    if (remainingDays < yearLen) {
      yearInCycle = y;
      break;
    }
    remainingDays -= yearLen;
  }

  let month = 1;
  let day = remainingDays + 1;
  for (let m = 1; m <= 12; m += 1) {
    const monthLen = hijriMonthLen(m, yearInCycle);
    if (day <= monthLen) {
      month = m;
      break;
    }
    day -= monthLen;
  }

  return {
    year: cycles * 30 + yearInCycle,
    month,
    day,
  };
}

export function formatHijriDate(parts: { year: number; month: number; day: number }): HijriDateParts {
  const monthIndex = Math.min(12, Math.max(1, parts.month)) - 1;
  const monthName = HIJRI_MONTHS_EN[monthIndex] ?? "Unknown";
  const monthNameAr = HIJRI_MONTHS_AR[monthIndex] ?? "غير معروف";
  return {
    year: parts.year,
    month: parts.month,
    day: parts.day,
    month_name: monthName,
    formatted: `${parts.day} ${monthName} ${parts.year}`,
    formatted_arabic: `${toArabicNumerals(parts.day)} ${monthNameAr} ${toArabicNumerals(parts.year)}`,
  };
}

export function hijriDateFromGregorian(date: Date, offsetDays = 0): HijriDateParts {
  const adjusted = new Date(date.getFullYear(), date.getMonth(), date.getDate() + offsetDays);
  return formatHijriDate(gregorianToHijri(adjusted));
}

export function parsePrayerKey(value: string): string | null {
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

export function prayerDisplayName(key: string): string {
  switch (key) {
    case "fajr":
      return "Fajr";
    case "sunrise":
      return "Sunrise";
    case "dhuhr":
      return "Dhuhr";
    case "asr":
      return "Asr";
    case "maghrib":
      return "Maghrib";
    case "isha":
      return "Isha";
    default:
      return key;
  }
}

/** Allow only http(s) external opens from the renderer. */
export function isSafeExternalUrl(url: string): boolean {
  try {
    const parsed = new URL(url);
    return parsed.protocol === "http:" || parsed.protocol === "https:";
  } catch {
    return false;
  }
}

/** Adaptive scheduler poll interval (seconds) – mirrors core adaptive_scheduler_sleep_secs. */
export function adaptiveSchedulerSleepSecs(secondsToNextEvent: number | null | undefined): number {
  if (secondsToNextEvent == null || !Number.isFinite(secondsToNextEvent)) return 120;
  const s = secondsToNextEvent;
  if (s <= 120) return 5;
  if (s <= 900) return 15;
  if (s <= 3600) return 30;
  return Math.min(300, Math.max(60, Math.floor(s / 2)));
}

/**
 * Day offset in [-3,3] so tabular Hijri(g+offset) matches observed authority date.
 * Returns null if no match (caller should keep manual offset).
 */
export function suggestHijriOffset(
  gregorian: Date,
  observed: { year: number; month: number; day: number },
  fromGregorian: (d: Date) => { year: number; month: number; day: number },
): number | null {
  const base = new Date(gregorian.getFullYear(), gregorian.getMonth(), gregorian.getDate());
  for (let off = -3; off <= 3; off += 1) {
    const d = new Date(base);
    d.setDate(d.getDate() + off);
    const cand = fromGregorian(d);
    if (
      cand.year === observed.year &&
      cand.month === observed.month &&
      cand.day === observed.day
    ) {
      return off;
    }
  }
  return null;
}

export function compareSemver(a: string, b: string): number {
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

/**
 * Decide update UI status from current app version vs newest public GitHub release.
 * Includes pre-releases (must not use /releases/latest alone).
 */
export function resolveUpdateStatus(
  currentVersion: string,
  latestVersion: string,
  isPrerelease: boolean,
): { available: boolean; status: "update" | "current" | "ahead"; message: string } {
  const cmp = compareSemver(currentVersion, latestVersion);
  if (cmp < 0) {
    return {
      available: true,
      status: "update",
      message: `Version ${latestVersion}${isPrerelease ? " (pre-release)" : ""} is available!`,
    };
  }
  if (cmp === 0) {
    return {
      available: false,
      status: "current",
      message: isPrerelease
        ? "You are on the latest version. You are on the latest pre-release."
        : "You are on the latest version.",
    };
  }
  return {
    available: false,
    status: "ahead",
    message: `You are on ${currentVersion}, which is newer than the latest public release (${latestVersion}).`,
  };
}

/**
 * Upsert a completed prayer log entry for a local calendar day.
 * Returns a new array (does not mutate input).
 */
export function upsertCompletedPrayer(
  entries: Array<{ date: string; prayer: string; completed: boolean }>,
  date: string,
  prayerDisplay: string,
): Array<{ date: string; prayer: string; completed: boolean }> {
  const next = entries.filter(
    (e) => !(e.date === date && e.prayer.toLowerCase() === prayerDisplay.toLowerCase()),
  );
  next.push({ date, prayer: prayerDisplay, completed: true });
  return next;
}

/** Pending audio control for the hidden Electron audio BrowserWindow. */
export type PendingAudioEvent = { event: string; payload?: unknown };

/**
 * Dispatch state for cold-start audio: loadURL is async and preload listeners
 * only exist after the page script runs. Events must queue until ready.
 */
export type AudioDispatchState = {
  ready: boolean;
  queue: PendingAudioEvent[];
};

export function createAudioDispatchState(): AudioDispatchState {
  return { ready: false, queue: [] };
}

export function resetAudioDispatch(state: AudioDispatchState): void {
  state.ready = false;
  state.queue = [];
}

/**
 * If ready, returns the event to send immediately.
 * If not ready, enqueues (coalescing play/stop) and returns null.
 */
export function queueOrDispatchAudio(
  state: AudioDispatchState,
  event: string,
  payload?: unknown,
): PendingAudioEvent | null {
  if (state.ready) {
    return { event, payload };
  }

  if (event === "audio:play") {
    // Keep only the latest play; drop earlier plays still waiting for load.
    state.queue = state.queue.filter((item) => item.event !== "audio:play");
    state.queue.push({ event, payload });
    return null;
  }

  if (event === "audio:stop") {
    // Cancel pending plays; keep a single stop for after load if needed.
    state.queue = state.queue.filter(
      (item) => item.event !== "audio:play" && item.event !== "audio:stop",
    );
    state.queue.push({ event, payload });
    return null;
  }

  state.queue.push({ event, payload });
  return null;
}

/** Mark ready and return drained queue (call on did-finish-load). */
export function flushAudioQueue(state: AudioDispatchState): PendingAudioEvent[] {
  state.ready = true;
  const pending = state.queue.slice();
  state.queue = [];
  return pending;
}
