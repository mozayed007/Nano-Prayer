# NanoPrayer AX

NanoPrayer exposes an agent-facing layer for tools, MCP hosts, local automations, and products that want accurate prayer context without driving the desktop UI.

## Surfaces

- `nano-pray-core`: shared Rust domain engine for prayer times, Qibla, Hijri date, locations, config, reminders, and logs.
- `nano-pray`: JSON CLI for shell tools and simple local agents.
- `nano-pray-mcp`: MCP server exposing tools, resources, and prompts.
- Desktop commands: Tauri and Electron both expose matching agent actions for next prayer, muting, and reminder updates.

## CLI

```powershell
cargo run -p nano-pray-cli -- times --lat 30.0444 --lng 31.2357 --date 2026-06-05
cargo run -p nano-pray-cli -- next
cargo run -p nano-pray-cli -- qibla --lat 30.0444 --lng 31.2357
cargo run -p nano-pray-cli -- cities search Cairo
cargo run -p nano-pray-cli -- config get
cargo run -p nano-pray-cli -- reminders mute
cargo run -p nano-pray-cli -- reminders set --prayer fajr --minutes-before 20
cargo run -p nano-pray-cli -- log complete Maghrib
```

All CLI commands return JSON and write errors as JSON with `ok: false`.

## MCP Tools

- `get_prayer_times`
- `get_monthly_prayer_times`
- `get_next_prayer`
- `get_qibla_direction`
- `search_cities`
- `get_config`
- `get_hijri_date`
- `mute_reminders`
- `update_reminder_settings`
- `mark_prayer_completed`

Run the server:

```powershell
cargo run -p nano-pray-mcp
```

## MCP Resources

- `nanoprayer://schedule/today`
- `nanoprayer://schedule/month`
- `nanoprayer://config/current`
- `nanoprayer://locations`

## Safety Rules

- Read-only tools can be used directly.
- Mutating tools require clear user intent: `mute_reminders`, `update_reminder_settings`, and `mark_prayer_completed`.
- Agents should not infer calculation method, madhab, location, or reminder changes from casual context.
- Returned config is summarized by default so agents do not need raw local paths or unrelated desktop preferences.

## Build And Cleanup

```powershell
npm run build:all
npm run build:all:compact
npm run prune:target -- -DryRun
```

`scripts/prune-target.ps1` only prunes known generated entries inside the workspace `target` folder.
