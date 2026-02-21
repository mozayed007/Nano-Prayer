//! Configuration and settings module
//!
//! Manages application configuration with persistence.

use crate::error::Result;
use crate::location::SavedLocation;
use crate::prayer::{AsrMadhab, CalculationMethod, PrayerAdjustments, HighLatitudeRule};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Theme preference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Light,
    Dark,
    #[default]
    System,
}

/// Clock format preference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ClockFormat {
    #[default]
    Hour12,
    Hour24,
}

/// Reminder settings for a single prayer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderConfig {
    pub enabled: bool,
    pub minutes_before: i32,
    #[serde(default)]
    pub play_sound_before: bool,
    pub play_adhan: bool,
    #[serde(default)]
    pub minutes_after: i32,
    #[serde(default)]
    pub play_sound_after: bool,
    pub custom_sound: Option<PathBuf>,
    pub volume: f32,
    pub show_notification: bool,
}

impl Default for ReminderConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            minutes_before: 15,
            play_sound_before: false,
            play_adhan: true,
            minutes_after: 0,
            play_sound_after: false,
            custom_sound: None,
            volume: 0.7,
            show_notification: true,
        }
    }
}

/// Audio settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSettings {
    pub default_adhan: Option<PathBuf>,
    pub global_volume: f32,
    pub fade_in_seconds: u32,
    pub adhan_for_all: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            default_adhan: None,
            global_volume: 0.7,
            fade_in_seconds: 3,
            adhan_for_all: false,
        }
    }
}

/// Notification settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub enabled: bool,
    pub show_time: bool,
    pub show_countdown: bool,
    pub play_sound: bool,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            show_time: true,
            show_countdown: true,
            play_sound: true,
        }
    }
}

/// Appearance settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceSettings {
    pub theme: Theme,
    pub accent_color: String,
    pub clock_format: ClockFormat,
    pub show_arabic: bool,
    pub animations_enabled: bool,
    pub font_scale: f32,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            accent_color: "#2196F3".to_string(),
            clock_format: ClockFormat::default(),
            show_arabic: true,
            animations_enabled: true,
            font_scale: 1.0,
        }
    }
}

/// Advanced settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedSettings {
    pub auto_start: bool,
    pub start_minimized: bool,
    pub minimize_to_tray: bool,
    pub auto_update_check: bool,
    pub time_sync_detection: bool,
    pub quiet_hours_enabled: bool,
    pub quiet_hours_start: u8,
    pub quiet_hours_end: u8,
}

impl Default for AdvancedSettings {
    fn default() -> Self {
        Self {
            auto_start: false,
            start_minimized: false,
            minimize_to_tray: true,
            auto_update_check: true,
            time_sync_detection: true,
            quiet_hours_enabled: false,
            quiet_hours_start: 22,
            quiet_hours_end: 6,
        }
    }
}

/// Complete application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub locations: Vec<SavedLocation>,
    pub current_location_index: usize,
    pub calculation_method: CalculationMethod,
    pub asr_madhab: AsrMadhab,
    #[serde(default)]
    pub high_latitude_rule: HighLatitudeRule,
    
    pub prayer_adjustments: PrayerAdjustments,
    pub hijri_offset: i32,
    pub show_hijri: bool,
    pub reminders: HashMap<String, ReminderConfig>,
    pub audio: AudioSettings,
    pub notifications: NotificationSettings,
    pub appearance: AppearanceSettings,
    pub advanced: AdvancedSettings,
    pub last_backup_path: Option<PathBuf>,
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut reminders = HashMap::new();
        reminders.insert("fajr".to_string(), ReminderConfig::default());
        reminders.insert("dhuhr".to_string(), ReminderConfig::default());
        reminders.insert("asr".to_string(), ReminderConfig::default());
        reminders.insert("maghrib".to_string(), ReminderConfig::default());
        reminders.insert("isha".to_string(), ReminderConfig::default());

        Self {
            locations: Vec::new(),
            current_location_index: 0,
            calculation_method: CalculationMethod::default(),
            asr_madhab: AsrMadhab::default(),
            high_latitude_rule: HighLatitudeRule::default(),
            
            prayer_adjustments: PrayerAdjustments::default(),
            hijri_offset: 0,
            show_hijri: true,
            reminders,
            audio: AudioSettings::default(),
            notifications: NotificationSettings::default(),
            appearance: AppearanceSettings::default(),
            advanced: AdvancedSettings::default(),
            last_backup_path: None,
        }
    }
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let path = std::env::current_exe()
            .unwrap_or_default()
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("config.toml");
        
        let config: Self = confy::load_path(&path)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = std::env::current_exe()
            .unwrap_or_default()
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("config.toml");
            
        confy::store_path(&path, self)?;
        Ok(())
    }

    pub fn current_location(&self) -> Option<&SavedLocation> {
        self.locations.get(self.current_location_index)
    }

    pub fn set_current_location(&mut self, index: usize) {
        if index < self.locations.len() {
            self.current_location_index = index;
        }
    }

    pub fn add_location(&mut self, location: SavedLocation) {
        self.locations.push(location);
        if self.locations.len() == 1 {
            self.current_location_index = 0;
        }
    }

    pub fn remove_location(&mut self, index: usize) {
        if index < self.locations.len() {
            self.locations.remove(index);
            if self.current_location_index >= self.locations.len() && !self.locations.is_empty() {
                self.current_location_index = self.locations.len() - 1;
            }
        }
    }

    pub fn reminder_for(&self, prayer: &str) -> ReminderConfig {
        self.reminders.get(prayer).cloned().unwrap_or_default()
    }

    pub fn set_reminder(&mut self, prayer: &str, config: ReminderConfig) {
        self.reminders.insert(prayer.to_string(), config);
    }

    pub fn export_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn import_json(json: &str) -> Result<Self> {
        let config: Self = serde_json::from_str(json)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert!(config.locations.is_empty());
        assert_eq!(config.calculation_method, CalculationMethod::default());
    }

    #[test]
    fn test_reminder_config() {
        let config = AppConfig::default();
        let fajr_reminder = config.reminder_for("fajr");
        assert!(fajr_reminder.enabled);
        assert_eq!(fajr_reminder.minutes_before, 15);
    }
}

