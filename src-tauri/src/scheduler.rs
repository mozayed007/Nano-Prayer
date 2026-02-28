use crate::audio::AudioState;
use crate::commands::AppState;
use chrono::{DateTime, Local, NaiveDate};
use nano_pray_core::config::{AppConfig, ReminderConfig};
use nano_pray_core::prayer::{Prayer, PrayerCalculator, PrayerInfo};
use std::time::Duration;
use tauri::Emitter;
use tauri::{AppHandle, Manager};
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
    fired_reminders: std::collections::HashSet<(Prayer, &'static str)>,
    /// Tracks which prayers have already had their on-time adhan fired today.
    fired_prayers: std::collections::HashSet<Prayer>,
}

impl Scheduler {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            last_check_date: None,
            fired_reminders: std::collections::HashSet::new(),
            fired_prayers: std::collections::HashSet::new(),
        }
    }

    pub async fn run(mut self) {
        loop {
            if let Err(e) = self.check().await {
                tracing::error!("Scheduler error: {}", e);
            }
            sleep(Duration::from_secs(60)).await;
        }
    }

    async fn check(&mut self) -> Result<(), String> {
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
            .with_method(config.calculation_method)
            .with_madhab(config.asr_madhab)
            .with_high_latitude_rule(config.high_latitude_rule)
            .with_adjustments(config.prayer_adjustments);

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
                let diff = times.minutes_to_next.unwrap_or(0);
                let tooltip = format!("Next: {} in {}m", next.name(), diff);
                let _ = tray.set_tooltip(Some(tooltip));
            } else {
                let _ = tray.set_tooltip(Some("No more prayers today"));
            }
        }

        for prayer_info in &times.prayers {
            self.check_prayer(&config, prayer_info, now).await?;
        }

        Ok(())
    }

    async fn check_prayer(
        &mut self,
        config: &AppConfig,
        info: &PrayerInfo,
        now: DateTime<Local>,
    ) -> Result<(), String> {
        let prayer_name = info.prayer.name().to_lowercase();
        let reminder_config = config.reminder_for(&prayer_name);

        if !reminder_config.enabled {
            return Ok(());
        }

        // Time difference in minutes (positive = prayer is in the future)
        let diff = info.time.signed_duration_since(now).num_minutes();

        // 1. "Before" reminder — fires once per prayer per day
        if reminder_config.minutes_before > 0
            && diff > 0
            && diff <= reminder_config.minutes_before as i64
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

                let _ = self.app.emit(
                    "prayer-alert",
                    PrayerAlertPayload {
                        prayer: info.prayer.name().to_string(),
                        alert_type: "before".to_string(),
                        title: title.clone(),
                        body: body.clone(),
                    },
                );

                self.show_alert_window();

                if reminder_config.play_sound_before {
                    let _ = self.trigger_audio(config, info, &reminder_config);
                }

                self.fired_reminders.insert(key);
                tracing::info!("Fired 'before' reminder for {}", info.prayer);
            }
        }

        // 2. "On time" reminder — fires once per prayer per day.
        // diff will be 0 or go slightly negative within the same scheduler tick.
        // We allow a 2-minute window to avoid missing it due to scheduler jitter.
        if diff <= 0 && diff > -2 {
            if !self.fired_prayers.contains(&info.prayer) {
                let title = format!("Time for {}", info.prayer);
                let body = format!("It is now time for {}", info.prayer);

                if reminder_config.show_notification {
                    let _ = self.send_notification(&title, &body);
                }

                let _ = self.app.emit(
                    "prayer-alert",
                    PrayerAlertPayload {
                        prayer: info.prayer.name().to_string(),
                        alert_type: "on_time".to_string(),
                        title: title.clone(),
                        body: body.clone(),
                    },
                );

                self.show_alert_window();

                if reminder_config.play_adhan {
                    let _ = self.trigger_audio(config, info, &reminder_config);
                }

                self.fired_prayers.insert(info.prayer);
                // Also mark "before" as done so it doesn't fire if scheduler catches up
                self.fired_reminders.insert((info.prayer, "before"));
                tracing::info!("Fired 'on_time' reminder for {}", info.prayer);
            }
        }

        // 3. "After" reminder — fires once per prayer per day
        if reminder_config.minutes_after > 0
            && diff < 0
            && diff >= -(reminder_config.minutes_after as i64)
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

                let _ = self.app.emit(
                    "prayer-alert",
                    PrayerAlertPayload {
                        prayer: info.prayer.name().to_string(),
                        alert_type: "after".to_string(),
                        title: title.clone(),
                        body: body.clone(),
                    },
                );

                self.show_alert_window();

                if reminder_config.play_sound_after {
                    let _ = self.trigger_audio(config, info, &reminder_config);
                }

                self.fired_reminders.insert(key);
                tracing::info!("Fired 'after' reminder for {}", info.prayer);
            }
        }

        Ok(())
    }

    /// Shows the alert window, bringing it to the foreground.
    fn show_alert_window(&self) {
        if let Some(alert_win) = self.app.get_webview_window("alert") {
            let _ = alert_win.unminimize();
            let _ = alert_win.show();
            let _ = alert_win.set_focus();
        }
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

    fn trigger_audio(
        &self,
        _config: &AppConfig,
        info: &PrayerInfo,
        reminder: &ReminderConfig,
    ) -> Result<(), String> {
        let audio_state = self.app.state::<AudioState>();
        if let Some(path) = &reminder.custom_sound {
            if let Ok(path_buf) = std::path::PathBuf::from(path).canonicalize() {
                let _ = audio_state.0.play_file(path_buf, reminder.volume);
            }
        } else {
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
}
