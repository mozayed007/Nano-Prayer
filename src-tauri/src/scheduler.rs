use crate::audio::AudioState;
use crate::commands::{ActiveAlertPayload, AppState};
use chrono::{DateTime, Local, NaiveDate};
use nano_pray_core::config::{AppConfig, ReminderConfig};
use nano_pray_core::prayer::{Prayer, PrayerCalculator, PrayerInfo};
use nano_pray_core::reminder::{adaptive_scheduler_sleep_secs, is_quiet_hour};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tauri::Emitter;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_notification::NotificationExt;
use tokio::time::sleep;

#[derive(Clone, serde::Serialize)]
pub struct PrayerAlertPayload {
    pub prayer: String,
    pub alert_type: String, // "before", "on_time", "after"
    pub title: String,
    pub body: String,
}

pub struct Scheduler {
    app: AppHandle,
    /// Tracks the last day we ran, so we can reset reminder state at midnight.
    last_check_date: Option<NaiveDate>,
    /// Tracks which (Prayer, phase) reminder has already been fired today so we don't loop.
    /// phase is one of "before", "on_time", "after".
    fired_reminders: HashSet<(Prayer, &'static str)>,
    /// Tracks which prayers have already had their on-time adhan fired today.
    fired_prayers: HashSet<Prayer>,
    last_reminder_signatures: HashMap<Prayer, ReminderSignature>,
}

#[derive(Clone, PartialEq, Eq)]
struct ReminderSignature {
    enabled: bool,
    before_enabled: bool,
    minutes_before: i32,
    play_sound_before: bool,
    play_adhan: bool,
    after_enabled: bool,
    minutes_after: i32,
    play_sound_after: bool,
    custom_sound: Option<String>,
    custom_reminder_sound: Option<String>,
    volume_bits: u32,
    show_notification: bool,
}

impl ReminderSignature {
    fn from_config(config: &ReminderConfig) -> Self {
        Self {
            enabled: config.enabled,
            before_enabled: config.before_enabled,
            minutes_before: config.minutes_before,
            play_sound_before: config.play_sound_before,
            play_adhan: config.play_adhan,
            after_enabled: config.after_enabled,
            minutes_after: config.minutes_after,
            play_sound_after: config.play_sound_after,
            custom_sound: config
                .custom_sound
                .as_ref()
                .map(|path| path.to_string_lossy().to_string()),
            custom_reminder_sound: config
                .custom_reminder_sound
                .as_ref()
                .map(|path| path.to_string_lossy().to_string()),
            volume_bits: config.volume.to_bits(),
            show_notification: config.show_notification,
        }
    }
}

impl Scheduler {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            last_check_date: None,
            fired_reminders: HashSet::new(),
            fired_prayers: HashSet::new(),
            last_reminder_signatures: HashMap::new(),
        }
    }

    pub async fn run(mut self) {
        let scheduler_wakeup = self.app.state::<AppState>().scheduler_wakeup.clone();

        loop {
            let sleep_secs = match self.check().await {
                Ok(secs) => secs,
                Err(e) => {
                    tracing::error!("Scheduler error: {}", e);
                    30
                }
            };

            if tokio::time::timeout(
                Duration::from_secs(sleep_secs),
                scheduler_wakeup.notified(),
            )
            .await
            .is_ok()
            {
                tracing::info!("Scheduler woken after config update");
            }
        }
    }

    /// Runs one reminder pass. Returns adaptive sleep seconds until next poll.
    async fn check(&mut self) -> Result<u64, String> {
        let state = self.app.state::<AppState>();
        let config = state.config.lock().map_err(|e| e.to_string())?.clone();

        // Get location
        let (lat, lng, _) = if let Some(loc) = config.current_location() {
            (
                loc.coordinates.latitude,
                loc.coordinates.longitude,
                Some(loc.name.clone()),
            )
        } else {
            (21.4225, 39.8262, Some("Makkah".to_string()))
        };

        let calc = PrayerCalculator::new()
            .with_method(config.effective_calculation_method())
            .with_madhab(config.asr_madhab)
            .with_high_latitude_rule(config.high_latitude_rule)
            .with_adjustments(config.prayer_adjustments)
            .with_timezone(config.effective_timezone());

        let times = calc
            .calculate_today(lat, lng, None)
            .map_err(|e| e.to_string())?;
        let now = Local::now();
        let today = now.date_naive();

        // --- Reset all state at the start of each new day ---
        if self.last_check_date != Some(today) {
            tracing::info!("New day detected ({}), resetting reminder state.", today);
            self.fired_reminders.clear();
            self.fired_prayers.clear();
            self.last_check_date = Some(today);
        }

        // Update Tray tooltip with next prayer
        if let Some(tray) = self.app.tray_by_id("tray") {
            if let Some(next) = times.next_prayer {
                let tooltip = format_next_prayer_tooltip(next.name(), times.minutes_to_next);
                if let Err(e) = tray.set_tooltip(Some(tooltip)) {
                    tracing::warn!("Failed to update tray tooltip: {}", e);
                }
            } else if let Err(e) = tray.set_tooltip(Some("No more prayers today")) {
                tracing::warn!("Failed to update tray tooltip: {}", e);
            }
        }

        let mut nearest_event_secs = times.seconds_to_next;

        // Quiet hours in the *city* wall clock (DST/summer time aware).
        let quiet_hour = {
            use nano_pray_core::time_zone::{hour_in_zone, parse_timezone};
            let tz = config
                .effective_timezone()
                .as_deref()
                .and_then(parse_timezone);
            hour_in_zone(chrono::Utc::now(), tz) as u8
        };
        if config.advanced.quiet_hours_enabled
            && is_quiet_hour(
                quiet_hour,
                config.advanced.quiet_hours_start,
                config.advanced.quiet_hours_end,
            )
        {
            tracing::debug!("Quiet hours active; skipping reminder alerts");
            return Ok(adaptive_scheduler_sleep_secs(nearest_event_secs));
        }

        for prayer_info in &times.prayers {
            if let Some(event_in) = seconds_to_reminder_edge(prayer_info, &config, now) {
                nearest_event_secs = Some(match nearest_event_secs {
                    Some(n) => n.min(event_in),
                    None => event_in,
                });
            }
            self.check_prayer(&config, prayer_info, now).await?;
        }

        Ok(adaptive_scheduler_sleep_secs(nearest_event_secs))
    }

    async fn check_prayer(
        &mut self,
        config: &AppConfig,
        info: &PrayerInfo,
        _now: DateTime<Local>,
    ) -> Result<(), String> {
        let prayer_name = info.prayer.name().to_lowercase();
        let reminder_config = config.reminder_for(&prayer_name);
        let signature = ReminderSignature::from_config(&reminder_config);

        if self
            .last_reminder_signatures
            .get(&info.prayer)
            .is_some_and(|previous| previous != &signature)
        {
            self.fired_reminders.remove(&(info.prayer, "before"));
            self.fired_reminders.remove(&(info.prayer, "after"));
            self.fired_prayers.remove(&info.prayer);
            tracing::info!("Reminder config updated for {}", info.prayer);
        }
        self.last_reminder_signatures.insert(info.prayer, signature);

        if !reminder_config.enabled {
            return Ok(());
        }

        let muted = {
            let state = self.app.state::<AppState>();
            state.muted.lock().map(|m| *m).unwrap_or(false)
        };
        if muted {
            return Ok(());
        }

        // Absolute countdown from UTC (DST-safe); do not use wall-clock Local diffs.
        let diff_secs = info
            .time_utc
            .signed_duration_since(chrono::Utc::now())
            .num_seconds();
        let diff = diff_secs / 60; // minutes for message text

        // 1. "Before" reminder — fires once per prayer per day
        let before_window_secs = (reminder_config.minutes_before as i64).saturating_mul(60);
        if reminder_config.before_enabled
            && reminder_config.minutes_before > 0
            && diff_secs > 0
            && diff_secs <= before_window_secs
        {
            let key = (info.prayer, "before");
            if !self.fired_reminders.contains(&key) {
                let title = format!("{} Reminder", info.prayer);
                let body = format!(
                    "{} is in {} minutes at {}",
                    info.prayer,
                    diff,
                    info.time.format("%H:%M")
                );

                let payload = PrayerAlertPayload {
                    prayer: info.prayer.name().to_string(),
                    alert_type: "before".to_string(),
                    title: title.clone(),
                    body: body.clone(),
                };
                if reminder_config.show_notification {
                    let _ = self.send_notification(&title, &body);
                }

                self.emit_alert(payload).await;

                if reminder_config.play_sound_before {
                    let _ = self.trigger_beep(config, info, &reminder_config);
                }

                self.fired_reminders.insert(key);
                tracing::info!("Fired 'before' reminder for {}", info.prayer);
            }
        }

        // 2. "On time" reminder — fires once per prayer per day.
        // Use a 90s window so adaptive polling (5s near edge) does not miss the moment.
        if diff_secs <= 0 && diff_secs > -90 && !self.fired_prayers.contains(&info.prayer) {
            let title = format!("Time for {}", info.prayer);
            let body = format!("It is now time for {}", info.prayer);

            if reminder_config.show_notification {
                let _ = self.send_notification(&title, &body);
            }

            let payload = PrayerAlertPayload {
                prayer: info.prayer.name().to_string(),
                alert_type: "on_time".to_string(),
                title: title.clone(),
                body: body.clone(),
            };
            self.emit_alert(payload).await;

            if reminder_config.play_adhan {
                let _ = self.trigger_adhan(config, info, &reminder_config);
            }

            self.fired_prayers.insert(info.prayer);
            // Also mark "before" as done so it doesn't fire if scheduler catches up
            self.fired_reminders.insert((info.prayer, "before"));
            tracing::info!("Fired 'on_time' reminder for {}", info.prayer);
        }

        // 3. "After" reminder — fires once per prayer per day
        let after_window_secs = (reminder_config.minutes_after as i64).saturating_mul(60);
        if reminder_config.after_enabled
            && reminder_config.minutes_after > 0
            && diff_secs < 0
            && diff_secs >= -after_window_secs
        {
            let key = (info.prayer, "after");
            if !self.fired_reminders.contains(&key) {
                let past_minutes = -diff;
                let title = format!("{} Passed", info.prayer);
                let body = format!(
                    "{} was {} minutes ago at {}",
                    info.prayer,
                    past_minutes,
                    info.time.format("%H:%M")
                );

                let payload = PrayerAlertPayload {
                    prayer: info.prayer.name().to_string(),
                    alert_type: "after".to_string(),
                    title: title.clone(),
                    body: body.clone(),
                };

                if reminder_config.show_notification {
                    let _ = self.send_notification(&title, &body);
                }

                self.emit_alert(payload).await;

                if reminder_config.play_sound_after {
                    let _ = self.trigger_beep(config, info, &reminder_config);
                }

                self.fired_reminders.insert(key);
                tracing::info!("Fired 'after' reminder for {}", info.prayer);
            }
        }

        Ok(())
    }

    /// Shows the alert window, bringing it to the foreground.
    fn show_alert_window(&self) -> Result<bool, String> {
        let mut created = false;
        if self.app.get_webview_window("alert").is_none() {
            let builder =
                WebviewWindowBuilder::new(&self.app, "alert", WebviewUrl::App("/alert".into()))
                    .title("Prayer Alert")
                    .inner_size(380.0, 180.0)
                    .resizable(false)
                    .decorations(false)
                    .always_on_top(true)
                    .focused(false)
                    .skip_taskbar(true)
                    .visible(false);

            created = if let Some(icon) = self.app.default_window_icon() {
                builder
                    .icon(icon.clone())
                    .and_then(|builder| builder.build())
                    .is_ok()
            } else {
                builder.build().is_ok()
            };

            if !created {
                return Err("Failed to create alert window".to_string());
            }
            if self.app.get_webview_window("alert").is_none() {
                tracing::warn!(
                    "Alert window was not created despite builder not returning an error"
                );
                return Err("Alert window not found after creation attempt".to_string());
            }
        }

        if let Some(alert_win) = self.app.get_webview_window("alert") {
            let _ = alert_win.unminimize();
            let _ = alert_win.show();
        }
        Ok(created)
    }

    async fn emit_alert(&self, payload: PrayerAlertPayload) {
        if let Ok(mut guard) = self.app.state::<AppState>().active_alert.lock() {
            *guard = Some(ActiveAlertPayload {
                prayer: payload.prayer.clone(),
                alert_type: payload.alert_type.clone(),
                title: payload.title.clone(),
                body: payload.body.clone(),
            });
        }
        let created = self.show_alert_window();
        match &created {
            Ok(true) => {
                sleep(Duration::from_millis(250)).await;
            }
            Err(e) => {
                tracing::error!("Failed to show alert window: {}", e);
            }
            _ => {}
        }
        let _ = self.app.emit("prayer-alert", payload);
    }

    fn send_notification(&self, title: &str, body: &str) -> Result<(), String> {
        self.app
            .notification()
            .builder()
            .title(title)
            .body(body)
            .show()
            .map_err(|e| e.to_string())
    }

    /// Plays the adhan sound (for on-time prayer alerts).
    fn trigger_adhan(
        &self,
        _config: &AppConfig,
        info: &PrayerInfo,
        reminder: &ReminderConfig,
    ) -> Result<(), String> {
        let audio_state = self.app.state::<AudioState>();
        let mut played_custom = false;
        if let Some(path) = &reminder.custom_sound {
            match std::path::PathBuf::from(path).canonicalize() {
                Ok(path_buf) => {
                    let _ = audio_state.0.play_file(path_buf, reminder.volume);
                    played_custom = true;
                }
                Err(_) => {
                    tracing::warn!(
                        "Custom adhan sound '{:?}' not found for {}, falling back to default",
                        path,
                        info.prayer
                    );
                }
            }
        }
        if !played_custom {
            tracing::info!("Playing Default Adhan for {}", info.prayer);
            let bytes = if info.prayer == nano_pray_core::prelude::Prayer::Fajr {
                include_bytes!("../assets/adhan_fajr.mp3").as_slice()
            } else {
                include_bytes!("../assets/adhan.mp3").as_slice()
            };
            let _ = audio_state.0.play_embedded(bytes, reminder.volume);
        }
        Ok(())
    }

    /// Plays a beep/alarm sound (for before/after reminders).
    fn trigger_beep(
        &self,
        _config: &AppConfig,
        info: &PrayerInfo,
        reminder: &ReminderConfig,
    ) -> Result<(), String> {
        let audio_state = self.app.state::<AudioState>();
        if let Some(path) = &reminder.custom_reminder_sound {
            if let Ok(path_buf) = std::path::PathBuf::from(path).canonicalize() {
                tracing::info!("Playing custom reminder sound for {}", info.prayer);
                let _ = audio_state.0.play_file(path_buf, reminder.volume);
            } else {
                tracing::warn!(
                    "Custom reminder sound not found for {}, falling back to classic alarm",
                    info.prayer
                );
                let bytes = include_bytes!("../assets/classic_alarm.mp3").as_slice();
                let _ = audio_state.0.play_embedded(bytes, reminder.volume);
            }
        } else {
            tracing::info!("Playing classic alarm reminder for {}", info.prayer);
            let bytes = include_bytes!("../assets/classic_alarm.mp3").as_slice();
            let _ = audio_state.0.play_embedded(bytes, reminder.volume);
        }
        Ok(())
    }
}

/// Seconds until the next before/on-time/after edge for this prayer (for adaptive sleep).
fn seconds_to_reminder_edge(
    info: &PrayerInfo,
    config: &AppConfig,
    _now: DateTime<Local>,
) -> Option<i64> {
    let prayer_name = info.prayer.name().to_lowercase();
    let reminder = config.reminder_for(&prayer_name);
    if !reminder.enabled {
        return None;
    }
    let diff_secs = info
        .time_utc
        .signed_duration_since(chrono::Utc::now())
        .num_seconds();
    let mut candidates: Vec<i64> = Vec::new();
    // On-time edge
    candidates.push(diff_secs);
    if reminder.before_enabled && reminder.minutes_before > 0 {
        candidates.push(diff_secs - (reminder.minutes_before as i64) * 60);
    }
    if reminder.after_enabled && reminder.minutes_after > 0 {
        candidates.push(diff_secs + (reminder.minutes_after as i64) * 60);
    }
    candidates
        .into_iter()
        .filter(|s| *s >= 0)
        .min()
}

// silence unused param for API stability of seconds_to_reminder_edge
#[allow(dead_code)]
fn _now_placeholder(_now: DateTime<Local>) {}

fn format_next_prayer_tooltip(next_prayer: &str, minutes_to_next: Option<i32>) -> String {
    let Some(minutes_to_next) = minutes_to_next else {
        return format!("Next: {}", next_prayer);
    };

    if minutes_to_next <= 0 {
        return format!("Now: {}", next_prayer);
    }

    let hours = minutes_to_next / 60;
    let minutes = minutes_to_next % 60;

    if hours > 0 && minutes > 0 {
        format!("{}h {}m till {}", hours, minutes, next_prayer)
    } else if hours > 0 {
        format!("{}h till {}", hours, next_prayer)
    } else {
        format!("{}m till {}", minutes, next_prayer)
    }
}

#[cfg(test)]
mod tests {
    use super::format_next_prayer_tooltip;

    #[test]
    fn tooltip_formats_hours_and_minutes() {
        assert_eq!(
            format_next_prayer_tooltip("Dhuhr", Some(75)),
            "1h 15m till Dhuhr"
        );
        assert_eq!(format_next_prayer_tooltip("Asr", Some(60)), "1h till Asr");
        assert_eq!(format_next_prayer_tooltip("Maghrib", Some(9)), "9m till Maghrib");
        assert_eq!(format_next_prayer_tooltip("Isha", Some(0)), "Now: Isha");
        assert_eq!(format_next_prayer_tooltip("Fajr", None), "Next: Fajr");
    }
}
