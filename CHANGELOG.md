# Changelog

All notable changes to this project are documented in this file.

## [Unreleased]

### Changed
- Ignore `docs/` directories across the workspace to keep local debugging materials out of git history.

## [0.1.0] - 2026-02-20

### Added
- Initial public desktop release built with Tauri + Rust + Svelte.
- Main application sections: Praytime, Schedule, Stats, Settings, and Qibla.
- Per-prayer reminders with configurable notification and sound behavior.
- Audio controls in Settings for Adhan preview, pause/resume, and stop.
- “Close/minimize to system tray” behavior option in Settings.

### Changed
- Refined desktop responsiveness and dynamic layout behavior for wide and narrow windows.
- Improved prayer-time card scaling so content remains readable at different sizes.

### Fixed
- Daily Times overflow/scroll behavior in narrower desktop widths.
- Oversized active item rendering and row clipping issues in Daily Times.
- Missing/non-working Adhan preview controls in reminder settings.

### Docs
- Expanded roadmap with MVP (next 30 days), quarterly goals, and long-term vision.

