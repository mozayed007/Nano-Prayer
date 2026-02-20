# NanoPrayReminder

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Platform: Windows](https://img.shields.io/badge/Platform-Windows-blue.svg)](https://www.microsoft.com/windows)
[![Tauri: v2](https://img.shields.io/badge/Tauri-v2-blue.svg)](https://tauri.app)
[![Status: Pre-release](https://img.shields.io/badge/Status-Pre--release-orange.svg)](https://github.com)

**A modern, lightweight Islamic prayer time reminder app** built with Rust + Tauri + Svelte. NanoPrayReminder provides accurate prayer times, customizable reminders, and a beautiful native desktop experience.

---

## ✨ Features

- 🕐 **High-precision prayer times** via `salah` library
- 🕌 **Multiple calculation methods**, madhab options, high-latitude rules, and manual offsets
- 📍 **Multi-location management** with search and active location switching
- 📅 **Hijri date conversion**
- 🧭 **Qibla direction** with compass view
- 📊 **Monthly schedule view** and basic prayer analytics/statistics
- 🔔 **Per-prayer reminder settings**:
  - Enable/disable toggle
  - Minutes before notification
  - System notifications
  - Adhan playback
  - Custom audio file with preview controls
- 🎨 **Theme support**: Light/dark/system themes
- 🖥️ **System tray integration** with autostart and global shortcut support

---

## 📸 Screenshots

> Screenshots coming soon. The app features a responsive dashboard, schedule view, statistics, Qibla compass, and settings screens.

---

## 💻 Tech Stack

| Technology | Purpose |
|------------|---------|
| Rust workspace (`core` + `src-tauri`) | Backend logic & native shell |
| Tauri 2 | Desktop application framework |
| Svelte 5 + SvelteKit + Vite | Frontend UI |
| Bun (recommended) | Package manager & build tool |

---

## 📁 Project Structure

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

# Desktop release build
bun run tauri:build
```

Installers are generated under:

- `target/release/bundle/msi/`
- `target/release/bundle/nsis/`

### Build Size Management

Release/debug artifacts can grow quickly. Use:

```bash
# Prune release + debug heavy build artifacts
bun run prune:target

# Or build and prune in one command
bun run tauri:build:compact
```

Manual options:

```powershell
# Keep debug artifacts, prune release only
pwsh -NoProfile -File ./scripts/prune-target.ps1 -KeepDebug
```

---

## 📖 Documentation

- 📋 Current project status: [`docs/PROGRESS.md`](docs/PROGRESS.md)
- 📝 Original planning/spec reference: [`docs/NanoPrayer.md`](docs/NanoPrayer.md)
- 🐛 UI issue screenshots used during debugging: `docs/debug/screenshot/`

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

```
MIT License

Copyright (c) 2024 Muhammad Z. Ahmed

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

---

<p align="center">
  Made with ❤️ for the Muslim community
</p>
