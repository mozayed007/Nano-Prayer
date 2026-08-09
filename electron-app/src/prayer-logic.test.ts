/**
 * Unit tests for shipped pure helpers used by Electron main.
 * Run after tsc: node --test dist/prayer-logic.test.js
 */
import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  adaptiveSchedulerSleepSecs,
  compareSemver,
  createAudioDispatchState,
  flushAudioQueue,
  gregorianToHijri,
  hijriDateFromGregorian,
  isQuietHour,
  isSafeExternalUrl,
  localDateStr,
  mergeAppConfig,
  parsePrayerKey,
  prayerDisplayName,
  queueOrDispatchAudio,
  resetAudioDispatch,
  suggestHijriOffset,
  toArabicNumerals,
  upsertCompletedPrayer,
} from "./prayer-logic";

describe("localDateStr", () => {
  it("formats local calendar components as YYYY-MM-DD", () => {
    const d = new Date(2026, 7, 9, 23, 30, 0); // Aug 9 local, near midnight
    assert.equal(localDateStr(d), "2026-08-09");
  });

  it("does not use UTC day for late-evening local times in positive offsets", () => {
    // 1:30 AM local on Jan 2 must not become Jan 1 when local is ahead of UTC
    // (this is the class of bug toISOString().slice(0,10) causes).
    const d = new Date(2026, 0, 2, 1, 30, 0);
    assert.equal(localDateStr(d), "2026-01-02");
  });
});

describe("parsePrayerKey / prayerDisplayName", () => {
  it("normalizes common aliases", () => {
    assert.equal(parsePrayerKey("Zuhr"), "dhuhr");
    assert.equal(parsePrayerKey("ISHA"), "isha");
    assert.equal(parsePrayerKey("fajr"), "fajr");
    assert.equal(parsePrayerKey("not-a-prayer"), null);
  });

  it("maps keys to display names", () => {
    assert.equal(prayerDisplayName("dhuhr"), "Dhuhr");
    assert.equal(prayerDisplayName("fajr"), "Fajr");
  });
});

describe("hijri conversion", () => {
  it("converts 2024-01-01 into 1445 AH range (matches core tabular bounds)", () => {
    const parts = gregorianToHijri(new Date(2024, 0, 1));
    assert.ok(parts.year >= 1445 && parts.year <= 1446);
    assert.ok(parts.month >= 1 && parts.month <= 12);
    assert.ok(parts.day >= 1 && parts.day <= 30);
  });

  it("returns structured hijri fields, not gregorian year", () => {
    const h = hijriDateFromGregorian(new Date(2026, 7, 9), 0);
    assert.ok(h.year < 2000, "hijri year must not be a gregorian year");
    assert.match(h.formatted, /\d+ \S+ \d+/);
    assert.ok(h.formatted_arabic.length > 0);
    assert.equal(toArabicNumerals(15), "١٥");
  });
});

describe("upsertCompletedPrayer", () => {
  it("dedupes same day and prayer regardless of case", () => {
    const first = upsertCompletedPrayer([], "2026-08-09", "Fajr");
    const second = upsertCompletedPrayer(first, "2026-08-09", "fajr");
    assert.equal(second.length, 1);
    assert.equal(second[0].prayer, "fajr");
    assert.equal(second[0].completed, true);
  });
});

describe("isSafeExternalUrl", () => {
  it("allows http(s) only", () => {
    assert.equal(isSafeExternalUrl("https://example.com/x"), true);
    assert.equal(isSafeExternalUrl("http://example.com"), true);
    assert.equal(isSafeExternalUrl("file:///C:/Windows/System32"), false);
    assert.equal(isSafeExternalUrl("javascript:alert(1)"), false);
    assert.equal(isSafeExternalUrl("not a url"), false);
  });
});

describe("compareSemver", () => {
  it("orders versions with optional v prefix", () => {
    assert.ok(compareSemver("0.1.5", "0.1.4") > 0);
    assert.equal(compareSemver("v0.1.5", "0.1.5"), 0);
    assert.ok(compareSemver("0.1.0", "0.2.0") < 0);
  });
});

describe("isQuietHour", () => {
  it("handles same-day and midnight-wrap ranges", () => {
    assert.equal(isQuietHour(14, 13, 17), true);
    assert.equal(isQuietHour(12, 13, 17), false);
    assert.equal(isQuietHour(23, 22, 6), true);
    assert.equal(isQuietHour(5, 22, 6), true);
    assert.equal(isQuietHour(6, 22, 6), false);
    assert.equal(isQuietHour(22, 22, 22), false);
  });
});

describe("mergeAppConfig", () => {
  it("fills missing nested advanced fields from defaults", () => {
    const defaults = {
      advanced: { muted: false, auto_start: false, quiet_hours_enabled: false },
      audio: { global_volume: 0.7 },
    };
    const loaded = {
      advanced: { muted: true },
      audio: { global_volume: 0.2 },
    };
    const merged = mergeAppConfig(defaults, loaded);
    assert.equal(merged.advanced.muted, true);
    assert.equal(merged.advanced.auto_start, false);
    assert.equal(merged.advanced.quiet_hours_enabled, false);
    assert.equal(merged.audio.global_volume, 0.2);
  });
});

describe("adaptiveSchedulerSleepSecs", () => {
  it("polls densely near events and sparsely when far", () => {
    assert.equal(adaptiveSchedulerSleepSecs(30), 5);
    assert.equal(adaptiveSchedulerSleepSecs(300), 15);
    assert.equal(adaptiveSchedulerSleepSecs(1800), 30);
    const far = adaptiveSchedulerSleepSecs(7200);
    assert.ok(far >= 60 && far <= 300);
    assert.equal(adaptiveSchedulerSleepSecs(null), 120);
  });
});

describe("suggestHijriOffset", () => {
  it("finds offset so tabular conversion matches observed", () => {
    const g = new Date(2024, 0, 1);
    const base = gregorianToHijri(g);
    assert.equal(suggestHijriOffset(g, base, gregorianToHijri), 0);
    const nextDay = new Date(2024, 0, 2);
    const observed = gregorianToHijri(nextDay);
    assert.equal(suggestHijriOffset(g, observed, gregorianToHijri), 1);
  });
});

describe("audio dispatch queue (cold-start race)", () => {
  it("queues play until flush (did-finish-load), then sends immediately", () => {
    const state = createAudioDispatchState();
    const first = queueOrDispatchAudio(state, "audio:play", { path: "a", volume: 0.5 });
    assert.equal(first, null);
    assert.equal(state.queue.length, 1);

    const pending = flushAudioQueue(state);
    assert.equal(pending.length, 1);
    assert.equal(pending[0].event, "audio:play");
    assert.deepEqual(pending[0].payload, { path: "a", volume: 0.5 });
    assert.equal(state.ready, true);
    assert.equal(state.queue.length, 0);

    const second = queueOrDispatchAudio(state, "audio:play", { path: "b", volume: 0.8 });
    assert.ok(second);
    assert.equal(second!.event, "audio:play");
    assert.deepEqual(second!.payload, { path: "b", volume: 0.8 });
  });

  it("coalesces multiple cold-start plays to the latest only", () => {
    const state = createAudioDispatchState();
    queueOrDispatchAudio(state, "audio:play", { path: "first" });
    queueOrDispatchAudio(state, "audio:play", { path: "second" });
    const pending = flushAudioQueue(state);
    assert.equal(pending.length, 1);
    assert.deepEqual(pending[0].payload, { path: "second" });
  });

  it("play then stop before ready cancels play and queues stop", () => {
    const state = createAudioDispatchState();
    queueOrDispatchAudio(state, "audio:play", { path: "x" });
    queueOrDispatchAudio(state, "audio:stop");
    const pending = flushAudioQueue(state);
    assert.equal(pending.length, 1);
    assert.equal(pending[0].event, "audio:stop");
  });

  it("reset drops ready and pending (window recreated)", () => {
    const state = createAudioDispatchState();
    flushAudioQueue(state);
    queueOrDispatchAudio(state, "audio:play", { path: "live" });
    resetAudioDispatch(state);
    assert.equal(state.ready, false);
    assert.equal(state.queue.length, 0);
    const again = queueOrDispatchAudio(state, "audio:play", { path: "queued" });
    assert.equal(again, null);
    assert.equal(state.queue.length, 1);
  });
});
