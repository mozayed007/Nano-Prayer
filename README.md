# NanoPrayReminder

![NanoPrayReminder Banner](static/banner.png)
> **Credits:** Banner and App Icon Design by [AhmedV19](https://x.com/A7medV19)

[![GitHub release (latest)](https://img.shields.io/github/v/release/mozayed007/Nano-Prayer?include_prereleases&label=Latest&color=blue)](https://github.com/mozayed007/Nano-Prayer/releases/latest)
[![GitHub downloads](https://img.shields.io/github/downloads/mozayed007/Nano-Prayer/total?label=Downloads&color=brightgreen)](https://github.com/mozayed007/Nano-Prayer/releases)
[![GitHub stars](https://img.shields.io/github/stars/mozayed007/Nano-Prayer?style=flat&color=yellow)](https://github.com/mozayed007/Nano-Prayer/stargazers)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Platform: Windows](https://img.shields.io/badge/Platform-Windows-blue.svg)](https://www.microsoft.com/windows)
[![Built with Tauri](https://img.shields.io/badge/Built%20with-Tauri%20v2-purple.svg)](https://tauri.app)
[![Built with Rust](https://img.shields.io/badge/Backend-Rust-orange.svg)](https://www.rust-lang.org)
[![Status: Pre-release](https://img.shields.io/badge/Status-Pre--release-orange.svg)](https://github.com/mozayed007/Nano-Prayer/releases)

**A modern, lightweight Islamic prayer time reminder app** built with Rust + Tauri + Svelte. NanoPrayReminder provides accurate prayer times, customizable reminders, and a beautiful native desktop experience.

---

## ✨ Features

- 🕐 **High-precision prayer times** via `salah` library
- 🕌 **Multiple calculation methods**, madhab options, high-latitude rules, and manual offsets
- 📍 **Multi-location management** with search and active location switching
- 📅 **Hijri date conversion**
- 🧭 **Qibla direction** with compass view
- 📊 **Monthly schedule view** and basic prayer analytics/statistics
- 🤖 **Agent experience (AX)** via JSON CLI, MCP tools/resources/prompts, and reusable agent profiles
- 🔔 **Per-prayer reminder settings**:
  - Enable/disable toggle
  - Minutes before notification
  - System notifications
  - Adhan playback
  - Custom audio file with preview controls
- 🎨 **Theme support**: Light/dark/system themes
- 🖥️ **System tray integration** with autostart and global shortcut support
- 🔄 **Professional update system**: Dual-channel updates (stable/pre-release) with in-app downloads
- 📦 **Portable & Installed versions**: Choose your preferred deployment method

---

## 📸 Screenshots

> Screenshots coming soon. The app features a responsive dashboard, schedule view, statistics, Qibla compass, and settings screens.

---

## 💻 Tech Stack

| Technology                                    | Purpose                                |
| --------------------------------------------- | -------------------------------------- |
| Rust workspace (`core` + `src-tauri` + AX binaries) | Backend logic, native shell, CLI, and MCP |
| Tauri 2                                       | Desktop application framework          |
| Svelte 5 + SvelteKit + Vite                   | Frontend UI                            |
| Bun (recommended)                             | Package manager & build tool           |

---

## 📁 Project Structure

```text
NanoPrayer/
├── core/                  # Rust domain logic (prayer, config, qibla, stats)
├── cli/                   # JSON CLI for agents and automation
├── mcp/                   # MCP server for agent hosts
├── agents/                # Agent skills and profiles
├── docs/                  # AX and integration documentation
├── src-tauri/             # Tauri backend (commands, scheduler, tray, audio)
├── src/                   # SvelteKit frontend routes + components
├── static/                # Web/static assets (banner, favicon)
├── scripts/               # Build and environment scripts
├── .github/workflows/     # CI/CD release pipeline
├── CHANGELOG.md           # Release notes by version
└── README.md
```

---

## 🚀 Installation

### Prerequisites

- **Rust toolchain** with `cargo` in `PATH`
- **Bun** (`https://bun.sh`) or npm
- **Windows WebView2 runtime** (for Tauri on Windows)

### Development

```bash
# Install JS dependencies
bun install

# Run desktop app in dev mode
bun run tauri:dev
```

Using npm:

```bash
npm install
npm run tauri:dev
```

### Build

```bash
# Frontend production build
bun run build

# Frontend, Rust workspace, and Electron TypeScript checks
bun run build:all

# Desktop release build
bun run tauri:build
```

Installers are generated under:

- `target/release/bundle/nsis/`

Portable executable:

- `target/release/nano-pray-reminder.exe`

### Build Size Management

Release/debug artifacts can grow quickly. Use:

```bash
# Prune release + debug heavy build artifacts
bun run prune:target

# Or build and prune in one command
bun run tauri:build:compact

# Or run frontend/Rust/Electron checks and prune target
bun run build:all:compact
```

Manual options:

```powershell
# Keep debug artifacts, prune release only
pwsh -NoProfile -File ./scripts/prune-target.ps1 -KeepDebug

# Preview cleanup without deleting files
pwsh -NoProfile -File ./scripts/prune-target.ps1 -DryRun
```

### Agent Experience (AX)

NanoPrayer can be used by agents without driving the desktop UI.

```bash
# JSON CLI
cargo run -p nano-pray-cli -- next
cargo run -p nano-pray-cli -- times --lat 30.0444 --lng 31.2357

# MCP server
cargo run -p nano-pray-mcp
```

See [`docs/AX.md`](docs/AX.md) and the profiles in [`agents/`](agents/).

### Complete Clean Build

If your `target` directory has accumulated gigabytes of old compilation artifacts (or you are running into bizarre build errors), you can perform a complete clean build. This will delete all compiled Rust binaries and cached dependencies.

```bash
# 1. Stop any running dev servers (Ctrl+C)
# 2. Clear the entire target directory
cargo clean
# 3. Rebuild from scratch
bun run tauri:dev
```

> **Note:** The first build after a `cargo clean` will take significantly longer because it has to fetch and compile all ~600 Rust dependencies from scratch.

---

## 📖 Documentation

- 📋 Release history: [`CHANGELOG.md`](CHANGELOG.md)
- ⚙️ Desktop app config: [`src-tauri/tauri.conf.json`](src-tauri/tauri.conf.json)
- 🚀 Release automation: [`.github/workflows/release.yml`](.github/workflows/release.yml)

---

## 🗺️ Product Roadmap

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

---

## 🤝 Contributing

We welcome contributions from the community! Here's how you can help:

### How to Contribute

1. **Fork** the repository
2. **Create a feature branch**: `git checkout -b feature/amazing-feature`
3. **Make your changes** and commit them: `git commit -m 'Add amazing feature'`
4. **Push to the branch**: `git push origin feature/amazing-feature`
5. **Open a Pull Request**

### Guidelines

- 📝 Follow the existing code style and conventions
- ✅ Test your changes thoroughly before submitting
- 📖 Update documentation if adding new features
- 🔍 Keep PRs focused and reasonably sized

### Code of Conduct

Be respectful and inclusive. We expect all contributors to:

- Use welcoming and inclusive language
- Be respectful of differing viewpoints and experiences
- Accept constructive criticism gracefully
- Focus on what is best for the community
- Show empathy towards other community members

### Reporting Issues

Found a bug or have a feature request? Please open an issue with:

- A clear, descriptive title
- Steps to reproduce (for bugs)
- Expected vs. actual behavior
- Your environment (OS, app version, etc.)

---

## 📄 License

This project is licensed under the **MIT License**.

---

<p align="center">
  Made with ❤️ for the Muslim community
</p>
