use chrono::{DateTime, Local};
use nano_pray_core::config::{AppConfig, ReminderConfig};
use nano_pray_core::prayer::{Prayer, PrayerCalculator, PrayerInfo};
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;
use tokio::time::sleep;
use crate::audio::AudioState;
use crate::commands::AppState;

pub struct Scheduler {
    app: AppHandle,
    last_prayer: Option<Prayer>,
    last_reminder: Option<(Prayer, i32)>, // Prayer and minutes before
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
            (loc.coordinates.latitude, loc.coordinates.longitude, Some(loc.name.clone()))
        } else {
            // Default or return early? Let's use Makkah as default if nothing set
            (21.4225, 39.8262, Some("Makkah".to_string()))
        };

        let calc = PrayerCalculator::new()
            .with_method(config.calculation_method)
            .with_madhab(config.asr_madhab)
            .with_high_latitude_rule(config.high_latitude_rule)
            .with_adjustments(config.prayer_adjustments);

        let times = calc.calculate_today(lat, lng, None).map_err(|e| e.to_string())?;
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

    async fn check_prayer(&mut self, config: &AppConfig, info: &PrayerInfo, now: DateTime<Local>) -> Result<(), String> {
        let prayer_name = info.prayer.name().to_lowercase();
        let reminder_config = config.reminder_for(&prayer_name);

        if !reminder_config.enabled {
            return Ok(());
        }

        // Time difference in minutes
        let diff = info.time.signed_duration_since(now).num_minutes();

        // 1. Check "Before" reminder
        if diff > 0 && diff <= reminder_config.minutes_before as i64 {
            // Check if we already reminded for this prayer at this time
            let key = (info.prayer, diff as i32);
            if self.last_reminder != Some(key) {
                self.send_notification(
                    &format!("{} Reminder", info.prayer),
                    &format!("{} is in {} minutes at {}", info.prayer, diff, info.time.format("%H:%M")),
                )?;
                self.last_reminder = Some(key);
            }
        }

        // 2. Check "On Time" (Adhan)
        // We allow a small window (e.g., 0 to -1 minute) to catch it if we missed the exact second
        if diff <= 0 && diff > -2 {
            if self.last_prayer != Some(info.prayer) {
                self.trigger_adhan(config, info, &reminder_config)?;
                self.last_prayer = Some(info.prayer);
                self.last_reminder = None; // Reset reminder for next cycle
            }
        }

        Ok(())
    }

    fn send_notification(&self, title: &str, body: &str) -> Result<(), String> {
        self.app.notification()
            .builder()
            .title(title)
            .body(body)
            .show()
            .map_err(|e| e.to_string())
    }

    fn trigger_adhan(&self, _config: &AppConfig, info: &PrayerInfo, reminder: &ReminderConfig) -> Result<(), String> {
        // Notification
        if reminder.show_notification {
            self.send_notification(
                &format!("Time for {}", info.prayer),
                &format!("It is now time for {}", info.prayer),
            )?;
        }

        // Audio
        if reminder.play_adhan {
            let audio_state = self.app.state::<AudioState>();
            // Use custom sound if set, otherwise default adhan
            // For now, we only have placeholder for default
            if let Some(path) = &reminder.custom_sound {
                if let Ok(path_buf) = std::path::PathBuf::from(path).canonicalize() {
                    let _ = audio_state.0.play_file(path_buf, reminder.volume);
                }
            } else {
                tracing::info!("Playing Default Adhan for {}", info.prayer);
                let adhan_file = if info.prayer == nano_pray_core::prelude::Prayer::Fajr { "adhan_fajr.mp3" } else { "adhan.mp3" };
                
                let mut played = false;
                
                // 1. Try Resource directory (installed apps)
                if let Ok(resource_dir) = self.app.path().resolve("assets", tauri::path::BaseDirectory::Resource) {
                    let bundled_path = resource_dir.join(adhan_file);
                    if bundled_path.exists() {
                        let _ = audio_state.0.play_file(bundled_path, reminder.volume);
                        
                    }
                }
                
                // 2. Try portable fallback
                if !played {
                    if let Ok(exe_path) = std::env::current_exe() {
                        if let Some(exe_dir) = exe_path.parent() {
                            let portable_path = exe_dir.join("resources").join("assets").join(adhan_file);
                            if portable_path.exists() {
                                let _ = audio_state.0.play_file(portable_path, reminder.volume);
                                
                            } else {
                                let alt_portable_path = exe_dir.join("assets").join(adhan_file);
                                if alt_portable_path.exists() {
                                    let _ = audio_state.0.play_file(alt_portable_path, reminder.volume);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}


