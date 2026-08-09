# E2E testing & performance profiling

What is needed for rigorous, legitimate end-to-end tests and flame-graph style profiling of **both** NanoPrayer stacks (Tauri/Rust and Electron).

## 1. E2E / integration testing – what you need

| Need | Why | Suggested setup |
|------|-----|-----------------|
| **Windows 10/11 desktop** | Production target; tray, notifications, WebView2, autostart | Physical PC or full Windows VM with GUI |
| **WebView2 Runtime** | Tauri shell | Evergreen WebView2 |
| **Rust toolchain + MSVC** | `cargo test`, `tauri build` | `rustup` stable + Build Tools |
| **Node LTS + npm** | Electron + Svelte frontend | Node 20+ |
| **Audio device** | Adhan/reminder path (can be virtual) | Real speakers or VAC; mute OK if logs prove play |
| **Network (optional)** | Hijri auto-align (Aladhan), updater | Or mock with offline fixtures |
| **Display session** | GUI launch is not headless | Logged-in interactive session (not pure SSH) |

### Recommended layers (rigorous pyramid)

1. **Unit (already in-repo)**  
   - `cargo test --workspace` – prayer, Hijri, quiet hours, adaptive sleep, timezone/DST  
   - `npm --prefix electron-app test` – pure logic + DST helpers  
   - Run on every change: `npm run check:all`

2. **Integration (CLI / IPC without full GUI)**  
   - `cargo run -p nano-pray-cli -- times --lat … --lng …`  
   - `cargo run -p nano-pray-cli -- hijri`  
   - Electron: thin script that loads `dist/main.js` helpers after `tsc` (no BrowserWindow)  
   - Assert JSON shapes, countdown signs, timezone wall clocks

3. **E2E GUI (legitimate user path)**  
   - **Tauri:** Playwright is weak for WebView2; prefer:  
     - Manual checklist + scripted `cargo run` / packaged exe  
     - Or **FlaUI** / **WinAppDriver** against the window title  
   - **Electron:** **Playwright** / **Spectron-successor** with `_ELECTRON_RUN_AS_NODE` disabled; launch `electron .` with env pointing at built frontend  
   - Cover: open app → set city → see times → toggle mute → save config → minimize to tray → wait for / simulate prayer edge → alert + audio log

4. **Soak / resource**  
   - 4–8 h idle minimized to tray  
   - Sample CPU/RSS every 30s  
   - Assert adaptive scheduler sleeps lengthen when far from next prayer

### What I need from you (or the environment) to run full E2E here

- Interactive Windows session with display (already mostly true)  
- Permission to install: WebView2 (if missing), optional `cargo-instruments` / Windows Performance Toolkit  
- Optional: dedicated audio sink  
- Optional: network for live Aladhan align tests  
- For CI: Windows runner + GUI-capable self-hosted agent if full GUI E2E is required

This agent environment can run **unit + CLI + short process launch**, but **cannot** replace multi-hour soak or real user notification UX without a dedicated test machine.

---

## 2. Performance profiling – flame graphs & friends

### Tauri / Rust

| Tool | Use |
|------|-----|
| **`cargo flamegraph`** (or `samply` / `cargo-instruments` on macOS) | CPU flamegraph of `nano-pray-reminder` while idle + near adhan |
| **Windows Performance Recorder / Analyzer** | System-wide CPU, disk, GPU |
| **Process Explorer** / Task Manager details | RSS, handles, GPU |
| **`tracy` / `tracing` spans** | Instrument scheduler tick, prayer calc, tray update |

Suggested workflow:

```powershell
# Install once
cargo install flamegraph
# Run under profiler (release!)
cargo build -p nano-pray-reminder --release
# Then attach WPR or flamegraph while app is tray-minimized for 2+ minutes
```

Focus:

- Scheduler wake rate (should be sparse when far from next prayer)  
- Config disk I/O (Electron: cache hit; Tauri: lock only)  
- UI interval when window hidden (frontend should not hammer IPC)

### Electron

| Tool | Use |
|------|-----|
| **Chrome DevTools** on main (`--inspect`) + renderer | CPU profiler, heap snapshots |
| **`clinic flame` / `0x`** | Node main-process flamegraphs |
| **Task Manager “Open process explorer”** | Per-process GPU/CPU for GPU process |

```powershell
cd electron-app
npm run build
# Main process inspect
npx electron --inspect=9229 .
# Open chrome://inspect → profile main while minimized
```

### Metrics that define “good enough”

| Metric | Idle (tray) target | Near prayer (±2 min) |
|--------|--------------------|----------------------|
| CPU (package) | mostly &lt; 1–2% average | short spikes OK for audio |
| Scheduler wake | multi-minute when far | ~5s adaptive |
| Main window timers | paused when hidden | active when visible |
| Memory growth | flat over 4h soak | no unbounded growth |

---

## 3. Summer time (DST) and alignment

Implemented in core:

- Location **IANA timezone** drives civil “today” and `HH:MM` display (`chrono-tz`)  
- London winter vs summer wall clocks differ for the same UTC instant  
- Hijri auto-align uses the **city civil date**, not raw system local  
- Scheduler countdowns use **UTC instants** (`time_utc`), not wall-clock Local diffs  

Electron mirrors this with `Intl` (`civilDateStrInZone`, `formatHmInZone`).

---

## 4. Minimal commands checklist

```powershell
# Unit
npm run check:all

# CLI accuracy samples
cargo run -p nano-pray-cli -- times --lat 51.5074 --lng -0.1278 --name London
cargo run -p nano-pray-cli -- hijri

# Electron unit
npm --prefix electron-app test

# Frontend
npm run build
```
