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

### Phase 1: UI/UX Improvements

- Improve visual hierarchy, spacing, and layout consistency across all pages
- Expand responsive behavior for compact desktop windows and split-screen use
- Improve navigation flow and interaction polish (transitions, micro-feedback)
- Run accessibility improvements (contrast, keyboard flow, focus states)

### Phase 2: Personalization (Themes and Fonts)

- Add richer theme system beyond current light/dark/system options
- Add user-selectable theme presets and custom accent palettes
- Add font options (readability-focused + modern display options)
- Add UI density modes (comfortable/compact)

### Phase 3: Desktop Enhancements

- Add taskbar widget / mini countdown surface for next prayer
- Expand tray quick actions (mute, snooze, jump to prayer page)
- Improve startup/minimize behavior and background notification experience

### Phase 4: Mobile App Version

- Build a mobile companion app (Android/iOS)
- Share settings and reminder profile model between desktop and mobile
- Keep offline-first behavior on mobile with optional cloud sync

### Phase 5: IoT and Smart Device Support

- Add IoT support path (smart displays, speakers, automation endpoints)
- Provide local network integrations (webhooks or lightweight local API)
- Add optional integrations for smart home routines around prayer times

### Ongoing Engineering Track

- Expand automated testing (UI regression + backend integration)
- Improve updater/release channel workflow
- Continue binary size and performance optimization

## License

MIT
