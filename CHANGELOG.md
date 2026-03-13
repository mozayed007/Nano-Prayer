# Changelog

All notable changes to this project are documented in this file.

## [0.1.4] - 2026-03-13

### Fixed

- **Popout alert window icon mismatch** - The independent alert window now uses the same native app icon as the main window.
- **Alert recovery after late-open/reload** - The alert popout now hydrates the currently active alert on load, so an in-flight alert is still shown if the window opens after emission.
- **Dismiss/completion state consistency** - Active alert state is now cleared centrally when dismissing alerts or marking a prayer as completed, keeping both alert UIs in sync.
- **Reminder sound settings visibility** - The custom reminder sound file section is now always visible in reminder settings instead of only appearing after specific toggles are enabled.
- **Reminder/adhan settings separation** - Clearing an adhan custom file no longer resets before/after reminder sound options.
- **Edited reminder tests not re-evaluating immediately** - Saving reminder changes now wakes the scheduler right away, so before/after reminder tests near the current countdown are rechecked without waiting for the next minute tick.

### Added

- **Dedicated before/after reminder sound support** - Reminder settings now support a separate custom beep/alarm file from the on-time adhan audio.
- **Reminder sound preview** - Settings now let you preview the before/after reminder sound, including the embedded classic alarm fallback.

### Updated

- **Before/after reminder audio behavior** - Before/after alerts now intentionally play a reminder beep/alarm instead of reusing adhan audio logic.
- **Alert route/layout handling** - Shared layout logic now treats all `/alert` routes consistently, improving the standalone alert window flow.
- **Settings reminders layout** - The reminders tab now uses a wider, more responsive desktop layout instead of compressing controls into narrow horizontal rows.

---

## [0.1.3] - 2026-03-10

### Fixed

- **Alert dismiss reliability** — Dismiss now consistently closes the external alert popout and guards against duplicate dismiss actions.
- **Alert window controls** — Added an explicit `✕` close control on active alert popouts for faster dismissal.
- **Unhandled promise noise in alert flow** — Hardened alert listener/hide paths to avoid unhandled rejection cascades during dismiss cycles.
- **Adhan not stopping on prayer completion** — Marking **I Prayed** now stops active audio immediately before emitting UI updates.
- **Reminder retrigger after edits** — Scheduler now resets same-prayer on-time fired state when reminder configuration changes, so edited reminders can trigger again.
- **Before/after sound behavior** — Before/after reminder audio now only plays when a custom sound is configured, preventing unintended default adhan playback.

### Updated

- **Fullscreen/focus behavior** — Alert popout is shown without stealing focus, improving non-intrusive behavior while users remain in their current app.
- **Reminder customization UX** — Settings now disable before/after sound toggles unless a custom audio file is selected, and automatically clear those toggles when custom audio is removed.
- **Sunrise reminder availability** — Sunrise reminder defaults are restored for new configs and backfilled for existing configs missing that prayer key.

---

## [0.1.2] - 2026-02-28

### Fixed

- **Infinite reminder loop** — Completely rewrote the scheduler state machine. Now uses `HashSet`-based phase tracking (`fired_reminders`, `fired_prayers`) instead of a single `last_reminder` slot. State resets at midnight so next-day reminders always work correctly.
- **Autostart not working (portable & installed)** — On every app launch, the current exe path is now re-registered with the Windows Registry via `tauri-plugin-autostart`. Moving the portable `.exe` to a new folder no longer leaves a broken/stale registry entry.
- **Alert window ACL error** — Fixed `Command plugin:event|listen not allowed by ACL` by correctly applying `core:event:allow-listen` and `core:event:allow-unlisten` permissions in `capabilities/default.json` for both `main` and `alert` Windows.

### Added

- GitHub Actions CI/CD release workflow (`.github/workflows/release.yml`) — automatically builds Windows NSIS installer + portable `.exe` and publishes them as a pre-release when a `v*` tag is pushed.
- `@tauri-apps/plugin-autostart` frontend integration — the "Start on System Startup" toggle in Settings now directly calls `enable()`/`disable()` on the OS.
- Unified alert lifecycle across both UIs — alert window and in-app toast now share the same dismiss event flow (`dismiss_alert`), so one dismiss stops audio and clears both.
- Prayer completion logging from alerts — both UIs now include an **I Prayed** action that records completion to persistent prayer logs and feeds Statistics.

### Updated

- Reminder edit behavior for same prayer/day — changing before/after reminder settings now resets that prayer’s fired before/after state so new edited reminder timing can trigger correctly.
- On-time alert behavior for focused/fullscreen apps — alert window shows without forcing focus, reducing app-switch interruptions while still surfacing reminders.
- Statistics backend wiring — implemented `get_statistics` and persistent prayer log load/save, so the Statistics page now reflects real tracked data.
- Reminder controls for before/after windows — added dedicated toggles (`before_enabled`, `after_enabled`) and enforced zero-minute behavior as disabled for before/after reminders.
- System tray next-prayer tooltip formatting — now displays clear `Xh Ym till Prayer` text instead of only raw total minutes.
- WebView2 idle memory optimization — alert window is no longer created at startup and is now lazily created only when an alert is fired.

---

## [0.1.1] - 2026-02-23

### Fixed

- ACL Error with Alerts: Resolved the `Command plugin:event|listen not allowed by ACL` error that occurred in the external alert/notification window.
- Event Listener Reliability: Added a retry mechanism and explicit webview-scoped permissions to ensure stable event listening in secondary windows.

### Features

- UI layout scaling fixes.
- Custom Prayer Alert window.
- Minimize-to-tray functionality.

---

## [0.1.0] - 2026-02-20

### Added

- Initial public desktop release built with Tauri + Rust + Svelte.
- Main application sections: Praytime, Schedule, Stats, Settings, and Qibla.
- Per-prayer reminders with configurable notification and sound behavior.
- Audio controls in Settings for Adhan preview, pause/resume, and stop.
- "Close/minimize to system tray" behavior option in Settings.

### Changed

- Refined desktop responsiveness and dynamic layout behavior for wide and narrow windows.
- Improved prayer-time card scaling so content remains readable at different sizes.

### Fixed

- Daily Times overflow/scroll behavior in narrower desktop widths.
- Oversized active item rendering and row clipping issues in Daily Times.
- Missing/non-working Adhan preview controls in reminder settings.

### Docs

- Expanded roadmap with MVP (next 30 days), quarterly goals, and long-term vision.
