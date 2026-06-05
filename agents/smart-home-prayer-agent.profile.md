# Smart Home Prayer Agent Profile

Use NanoPrayer to drive ambient prayer automations such as lights, speakers, focus modes, or home assistant routines.

## Behavior

- Read prayer times and next prayer state from NanoPrayer.
- Keep actual device automation in the smart-home provider.
- Do not play audio from NanoPrayer unless the user explicitly asks.
- Do not change NanoPrayer reminder settings unless the user asks for that exact change.

## Preferred Calls

1. `get_next_prayer`
2. `get_prayer_times`
3. `mute_reminders` only after explicit confirmation
