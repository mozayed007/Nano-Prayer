//! NanoPrayReminder Core Library
//!
//! A high-precision Islamic prayer time calculation library built on the `salah` crate.
//! Provides prayer times, Qibla direction, Hijri calendar, location management,
//! reminders, and prayer statistics.

pub mod agent;
pub mod config;
pub mod error;
pub mod hijri;
pub mod location;
pub mod prayer;
pub mod qibla;
pub mod reminder;
pub mod statistics;
pub mod time_zone;

// Re-exports for convenience
pub use config::AppConfig;
pub use error::{Error, Result};
pub use hijri::{suggest_offset_for_observed, HijriDate};
pub use location::{City, Coordinates, LocationManager};
pub use prayer::{PrayerCalculator, PrayerInfo, PrayerTimes};
pub use qibla::QiblaDirection;
pub use reminder::{
    adaptive_scheduler_sleep_secs, is_quiet_hour, is_quiet_hours_range, Reminder, ReminderScheduler,
};
pub use statistics::{PrayerLog, Statistics};
pub use time_zone::{civil_date_at, civil_date_now, format_hm, parse_timezone};

/// Prelude for common imports
pub mod prelude {
    pub use crate::config::AppConfig;
    pub use crate::error::{Error, Result};
    pub use crate::hijri::HijriDate;
    pub use crate::location::{City, Coordinates, LocationManager};
    pub use crate::prayer::{Prayer, PrayerCalculator, PrayerTimes};
    pub use crate::qibla::QiblaDirection;
    pub use crate::reminder::{Reminder, ReminderScheduler};
    pub use crate::statistics::{PrayerLog, Statistics};
    pub use chrono::{DateTime, Local, NaiveDate, NaiveTime, TimeZone, Utc};
}
