# Changelog

All notable changes to this project are documented in this file.

## [0.1.2] - 2026-02-28

### Fixed

- **Infinite reminder loop** — Completely rewrote the scheduler state machine. Now uses `HashSet`-based phase tracking (`fired_reminders`, `fired_prayers`) instead of a single `last_reminder` slot. State resets at midnight so next-day reminders always work correctly.
- **Autostart not working (portable & installed)** — On every app launch, the current exe path is now re-registered with the Windows Registry via `tauri-plugin-autostart`. Moving the portable `.exe` to a new folder no longer leaves a broken/stale registry entry.
- **Alert window ACL error** — Fixed `Command plugin:event|listen not allowed by ACL` by correctly applying `core:event:allow-listen` and `core:event:allow-unlisten` permissions in `capabilities/default.json` for both `main` and `alert` Windows.

### Added

- GitHub Actions CI/CD release workflow (`.github/workflows/release.yml`) — automatically builds Windows NSIS installer + portable `.exe` and publishes them as a pre-release when a `v*` tag is pushed.
- `@tauri-apps/plugin-autostart` frontend integration — the "Start on System Startup" toggle in Settings now directly calls `enable()`/`disable()` on the OS.

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
