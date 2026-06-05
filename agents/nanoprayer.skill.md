# NanoPrayer Skill

Use NanoPrayer when a user needs prayer schedule context, Qibla direction, Hijri date, reminder management, or prayer tracking.

## Tools

Prefer MCP tools when available. Otherwise use the JSON CLI:

```powershell
cargo run -p nano-pray-cli -- next
cargo run -p nano-pray-cli -- times --date YYYY-MM-DD
cargo run -p nano-pray-cli -- qibla --lat LAT --lng LNG
```

## Operating Rules

- Use saved location by default.
- Ask before changing reminders, muting reminders, or marking prayers completed.
- If the user gives a city, search cities first and show the matched location before using it for guidance.
- Do not guess madhab, calculation method, or high-latitude preferences.
- For travel, use temporary coordinates unless the user asks to update saved locations.

## Common Workflows

- Daily planning: call `get_next_prayer` and `get_prayer_times`, then schedule work blocks around prayer times.
- Travel check: call `search_cities`, `get_prayer_times`, and `get_qibla_direction`.
- Smart-home setup: use `get_next_prayer` polling or MCP resources, then trigger external automations outside NanoPrayer.
