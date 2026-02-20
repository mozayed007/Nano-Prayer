# NanoPrayReminder

Native desktop prayer reminder built with Rust + Tauri + Svelte.

## Current Status

The app is functional and packaged for Windows (`msi` + `nsis`) with:

- Responsive dashboard, schedule, statistics, qibla, and settings screens
- Prayer reminders with per-prayer audio/notification options
- Working Adhan preview controls in Settings (`Preview`/`Pause`/`Resume`/`Stop`)
- System tray integration, autostart plugin wiring, and global shortcut support

## Features

- High-precision prayer times via `salah`
- Multiple calculation methods, madhab, high-latitude rules, and manual offsets
- Multi-location management with search and active location switching
- Hijri date conversion
- Qibla direction + compass view
- Monthly schedule view
- Basic prayer analytics/statistics view
- Reminder settings per prayer:
  - enabled toggle
  - minutes before
  - system notification
  - adhan playback
  - custom audio file + preview controls
- Light/dark/system theme support

## Tech Stack

- Rust workspace (`core` + `src-tauri`)
- Tauri 2 desktop shell
- Svelte 5 + SvelteKit + Vite
- Bun (recommended) for frontend scripts

## Project Structure

```text
NanoPrayer/
├── core/                  # Pure Rust domain library
├── src-tauri/             # Tauri app (commands, scheduler, audio, tray)
├── src/                   # Svelte UI routes
├── scripts/               # Build utility scripts (target pruning)
├── docs/                  # Progress, design, and debug screenshots
├── assets/                # Shared assets
└── README.md
```

## Prerequisites

- Rust toolchain with `cargo` in `PATH`
- Bun (`https://bun.sh`) or npm
- Windows WebView2 runtime (for Tauri on Windows)

## Development

```bash
# install JS deps
bun install

# run desktop app in dev mode
bun run tauri:dev
```

If you prefer npm:

```bash
npm install
npm run tauri:dev
```

## Build

```bash
# frontend production build
bun run build

# desktop release build
bun run tauri:build
```

Installers are generated under:

- `target/release/bundle/msi/`
- `target/release/bundle/nsis/`

## Build Size Management

Release/debug artifacts can grow quickly. Use:

```bash
# prune release + debug heavy build artifacts
bun run prune:target

# or build and prune in one command
bun run tauri:build:compact
```

Manual options:

```powershell
# keep debug artifacts, prune release only
pwsh -NoProfile -File ./scripts/prune-target.ps1 -KeepDebug
```

## Docs

- Current project status: `docs/PROGRESS.md`
- Original planning/spec reference: `docs/NanoPrayer.md`
- UI issue screenshots used during debugging: `docs/debug/screenshot/`

## Product Roadmap (Planned)

This roadmap is a planning guide and can evolve based on user feedback and release priorities.

### MVP Next 30 Days

- Improve core UI/UX polish across all desktop pages (layout consistency, spacing, navigation flow, accessibility)
- Finalize responsive behavior for compact windows and split-screen usage
- Add first expanded theme pack (beyond light/dark/system) with safer contrast defaults
- Add initial font customization options (readability-first + modern display option)
- Add desktop taskbar widget / mini next-prayer countdown surface
- Harden reminders and background behavior (tray/minimize lifecycle, notification reliability)
- Prepare release-quality installer cadence and regression checklist

### Quarterly Goals (Next 3 Months)

- Deliver full personalization suite: theme presets, custom accents, font families, and density modes
- Expand desktop shell features: richer tray quick actions, quick mute/snooze, and one-click prayer shortcuts
- Improve analytics and daily workflow UX (faster log interactions, clearer progress views)
- Build automated UI regression + backend integration test coverage for critical flows
- Define and ship mobile architecture baseline (shared models, API boundaries, state sync strategy)

### Long-Term Vision (6-18 Months)

- Launch mobile companion apps (Android/iOS) with shared reminder and schedule experience
- Add optional cross-device sync for settings, locations, and user preferences
- Ship IoT support path (smart displays/speakers/home automation endpoints)
- Provide local network integrations (webhooks/lightweight local API) for smart-home routines
- Evolve NanoPrayer into a multi-surface prayer platform (desktop, mobile, and ambient devices)

### Ongoing Engineering Track

- Continue binary size and startup-time optimization
- Strengthen updater/release channels and rollback safety
- Maintain quality bar with accessibility and performance audits on each release cycle

## License

MIT
