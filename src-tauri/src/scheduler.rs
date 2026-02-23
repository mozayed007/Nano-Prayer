use crate::audio::AudioState;
use crate::commands::AppState;
use chrono::{DateTime, Local};
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
    last_prayer: Option<Prayer>,
    last_reminder: Option<(Prayer, i32)>, // Prayer and diff minutes
}

impl Scheduler {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            last_prayer: None,
            last_reminder: None,
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
            // Default or return early? Let's use Makkah as default if nothing set
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

        // 1. Update Tray Menu / Tooltip with next prayer
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

        // Time difference in minutes
        let diff = info.time.signed_duration_since(now).num_minutes();

        // 1. Check "Before" reminder
        if reminder_config.minutes_before > 0
            && diff > 0
            && diff <= reminder_config.minutes_before as i64
        {
            let key = (info.prayer, diff as i32);
            if self.last_reminder != Some(key) {
                let title = format!("{} Reminder", info.prayer);
                let body = format!(
                    "{} is in {} minutes at {}",
                    info.prayer,
                    diff,
                    info.time.format("%H:%M")
                );

                // Only send UI overlay events for before/after
                let _ = self.app.emit(
                    "prayer-alert",
                    PrayerAlertPayload {
                        prayer: info.prayer.name().to_string(),
                        alert_type: "before".to_string(),
                        title: title.clone(),
                        body: body.clone(),
                    },
                );
                
                // Auto-show windows on alert
                if let Some(window) = self.app.get_webview_window("main") {
                    let _ = window.unminimize();
                    let _ = window.show();
                    // Optional: only focus the main window if not close-to-tray, or prefer the alert window taking focus
                }
                
                if let Some(alert_win) = self.app.get_webview_window("alert") {
                    let _ = alert_win.unminimize();
                    let _ = alert_win.show();
                    let _ = alert_win.set_focus();
                }

                if reminder_config.play_sound_before {
                    let _ = self.trigger_audio(config, info, &reminder_config);
                }

                self.last_reminder = Some(key);
            }
        }

        // 2. Check "On Time" (Adhan)
        // We allow a small window (e.g., 0 to -1 minute) to catch it if we missed the exact second
        if diff <= 0 && diff > -2 {
            if self.last_prayer != Some(info.prayer) {
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

                // Auto-show windows on alert
                if let Some(window) = self.app.get_webview_window("main") {
                    let _ = window.unminimize();
                    let _ = window.show();
                }
                
                if let Some(alert_win) = self.app.get_webview_window("alert") {
                    let _ = alert_win.unminimize();
                    let _ = alert_win.show();
                    let _ = alert_win.set_focus();
                }

                if reminder_config.play_adhan {
                    let _ = self.trigger_audio(config, info, &reminder_config);
                }

                self.last_prayer = Some(info.prayer);
                self.last_reminder = None; // Reset reminder for next cycle
            }
        }

        // 3. Check "After" reminder
        if reminder_config.minutes_after > 0
            && diff < 0
            && diff >= -(reminder_config.minutes_after as i64)
        {
            let key = (info.prayer, diff as i32);
            if self.last_reminder != Some(key) {
                let past_minutes = -diff;
                let title = format!("{} Passed", info.prayer);
                let body = format!(
                    "{} was {} minutes ago at {}",
                    info.prayer,
                    past_minutes,
                    info.time.format("%H:%M")
                );

                // Only send UI overlay events for before/after
                let _ = self.app.emit(
                    "prayer-alert",
                    PrayerAlertPayload {
                        prayer: info.prayer.name().to_string(),
                        alert_type: "after".to_string(),
                        title: title.clone(),
                        body: body.clone(),
                    },
                );

                // Auto-show windows on alert
                if let Some(window) = self.app.get_webview_window("main") {
                    let _ = window.unminimize();
                    let _ = window.show();
                }
                
                if let Some(alert_win) = self.app.get_webview_window("alert") {
                    let _ = alert_win.unminimize();
                    let _ = alert_win.show();
                    let _ = alert_win.set_focus();
                }

                if reminder_config.play_sound_after {
                    let _ = self.trigger_audio(config, info, &reminder_config);
                }

                self.last_reminder = Some(key);
            }
        }

        Ok(())
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
        // Use custom sound if set, otherwise default adhan
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
