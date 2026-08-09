//! Reminder scheduling module

use chrono::{DateTime, Duration, Local, NaiveTime};
use serde::{Deserialize, Serialize};

use crate::config::ReminderConfig;
use crate::prayer::{Prayer, PrayerTimes};

/// Reminder type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReminderType {
    Before,
    OnTime,
    After,
}

/// A scheduled reminder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reminder {
    pub prayer: Prayer,
    pub reminder_type: ReminderType,
    pub scheduled_time: DateTime<Local>,
    pub minutes_offset: i32,
    pub triggered: bool,
}

impl Reminder {
    pub fn new(
        prayer: Prayer,
        reminder_type: ReminderType,
        scheduled_time: DateTime<Local>,
        minutes_offset: i32,
    ) -> Self {
        Self {
            prayer,
            reminder_type,
            scheduled_time,
            minutes_offset,
            triggered: false,
        }
    }

    pub fn before(prayer: Prayer, prayer_time: DateTime<Local>, minutes: i32) -> Self {
        Self::new(
            prayer,
            ReminderType::Before,
            prayer_time - Duration::minutes(minutes as i64),
            -minutes,
        )
    }

    pub fn on_time(prayer: Prayer, prayer_time: DateTime<Local>) -> Self {
        Self::new(prayer, ReminderType::OnTime, prayer_time, 0)
    }

    pub fn should_trigger(&self, now: DateTime<Local>) -> bool {
        !self.triggered && now >= self.scheduled_time
    }
}

/// Reminder event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderEvent {
    pub reminder: Reminder,
    pub triggered_at: DateTime<Local>,
}

/// Reminder scheduler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderScheduler {
    reminders: Vec<Reminder>,
    pub muted_until: Option<DateTime<Local>>,
    pub quiet_hours_enabled: bool,
    pub quiet_hours_start: NaiveTime,
    pub quiet_hours_end: NaiveTime,
}

impl Default for ReminderScheduler {
    fn default() -> Self {
        Self {
            reminders: Vec::new(),
            muted_until: None,
            quiet_hours_enabled: false,
            quiet_hours_start: NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
            quiet_hours_end: NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
        }
    }
}

impl ReminderScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn schedule_for_day(
        &mut self,
        prayer_times: &PrayerTimes,
        config: &std::collections::HashMap<String, ReminderConfig>,
    ) {
        self.reminders.clear();

        let default_config = ReminderConfig::default();

        for prayer_info in &prayer_times.prayers {
            if prayer_info.prayer == Prayer::Sunrise {
                continue;
            }

            let prayer_name = prayer_info.prayer.name().to_lowercase();
            let prayer_config = config.get(&prayer_name).unwrap_or(&default_config);

            if !prayer_config.enabled {
                continue;
            }

            if prayer_config.minutes_before > 0 {
                self.reminders.push(Reminder::before(
                    prayer_info.prayer,
                    prayer_info.time,
                    prayer_config.minutes_before,
                ));
            }

            if prayer_config.play_adhan || prayer_config.show_notification {
                self.reminders
                    .push(Reminder::on_time(prayer_info.prayer, prayer_info.time));
            }
        }

        self.reminders.sort_by_key(|r| r.scheduled_time);
    }

    pub fn next_reminder(&self) -> Option<&Reminder> {
        let now = Local::now();
        self.reminders
            .iter()
            .filter(|r| !r.triggered && r.scheduled_time > now)
            .min_by_key(|r| r.scheduled_time)
    }

    pub fn check_reminders(&mut self) -> Vec<ReminderEvent> {
        let now = Local::now();
        let mut events = Vec::new();

        if let Some(muted_until) = self.muted_until {
            if now < muted_until {
                return events;
            } else {
                self.muted_until = None;
            }
        }

        if self.quiet_hours_enabled && self.is_quiet_hours(now) {
            return events;
        }

        for reminder in &mut self.reminders {
            if reminder.should_trigger(now) {
                reminder.triggered = true;
                events.push(ReminderEvent {
                    reminder: reminder.clone(),
                    triggered_at: now,
                });
            }
        }

        events
    }

    fn is_quiet_hours(&self, now: DateTime<Local>) -> bool {
        is_quiet_hours_range(now.time(), self.quiet_hours_start, self.quiet_hours_end)
    }

    pub fn mute_for(&mut self, duration: Duration) {
        self.muted_until = Some(Local::now() + duration);
    }

    pub fn unmute(&mut self) {
        self.muted_until = None;
    }

    pub fn is_muted(&self) -> bool {
        self.muted_until.is_some_and(|until| Local::now() < until)
    }

    pub fn clear(&mut self) {
        self.reminders.clear();
    }
}

/// True when `current` falls in quiet hours `[start, end)`.
/// Supports ranges that wrap midnight (e.g. 22:00 → 06:00).
/// Equal start/end is treated as disabled (never quiet).
pub fn is_quiet_hours_range(current: NaiveTime, start: NaiveTime, end: NaiveTime) -> bool {
    if start == end {
        return false;
    }
    if start < end {
        current >= start && current < end
    } else {
        current >= start || current < end
    }
}

/// Adaptive poll interval: dense near prayer/reminder edges, sparse when idle.
/// Cuts idle CPU/disk while keeping on-time accuracy within a few seconds.
pub fn adaptive_scheduler_sleep_secs(seconds_to_next_event: Option<i64>) -> u64 {
    match seconds_to_next_event {
        Some(s) if s <= 120 => 5,
        Some(s) if s <= 900 => 15,
        Some(s) if s <= 3600 => 30,
        Some(s) => ((s / 2).clamp(60, 300)) as u64,
        None => 120,
    }
}

/// Quiet-hours check from whole-hour settings stored in `AppConfig.advanced`.
pub fn is_quiet_hour(hour: u8, start_hour: u8, end_hour: u8) -> bool {
    let hour = hour.min(23);
    let start_hour = start_hour.min(23);
    let end_hour = end_hour.min(23);
    let current = NaiveTime::from_hms_opt(hour as u32, 0, 0).unwrap_or(NaiveTime::MIN);
    let start = NaiveTime::from_hms_opt(start_hour as u32, 0, 0).unwrap_or(NaiveTime::MIN);
    let end = NaiveTime::from_hms_opt(end_hour as u32, 0, 0).unwrap_or(NaiveTime::MIN);
    is_quiet_hours_range(current, start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reminder() {
        let time = Local::now() + Duration::hours(2);
        let reminder = Reminder::before(Prayer::Fajr, time, 15);
        assert_eq!(reminder.prayer, Prayer::Fajr);
    }

    #[test]
    fn quiet_hours_same_day_range() {
        assert!(is_quiet_hour(14, 13, 17));
        assert!(!is_quiet_hour(12, 13, 17));
        assert!(!is_quiet_hour(17, 13, 17));
    }

    #[test]
    fn quiet_hours_wrap_midnight() {
        // 22:00–06:00
        assert!(is_quiet_hour(23, 22, 6));
        assert!(is_quiet_hour(0, 22, 6));
        assert!(is_quiet_hour(5, 22, 6));
        assert!(!is_quiet_hour(6, 22, 6));
        assert!(!is_quiet_hour(12, 22, 6));
        assert!(!is_quiet_hour(21, 22, 6));
    }

    #[test]
    fn quiet_hours_equal_start_end_is_never_quiet() {
        assert!(!is_quiet_hour(22, 22, 22));
        assert!(!is_quiet_hour(0, 0, 0));
    }

    #[test]
    fn quiet_hours_blocks_scheduler_events() {
        let mut scheduler = ReminderScheduler::default();
        scheduler.quiet_hours_enabled = true;
        scheduler.quiet_hours_start = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
        scheduler.quiet_hours_end = NaiveTime::from_hms_opt(23, 59, 59).unwrap();
        let past = Local::now() - Duration::minutes(1);
        scheduler.reminders.push(Reminder::on_time(Prayer::Dhuhr, past));
        assert!(scheduler.check_reminders().is_empty());
    }

    #[test]
    fn adaptive_sleep_is_dense_near_events_and_sparse_when_far() {
        assert_eq!(adaptive_scheduler_sleep_secs(Some(30)), 5);
        assert_eq!(adaptive_scheduler_sleep_secs(Some(300)), 15);
        assert_eq!(adaptive_scheduler_sleep_secs(Some(1800)), 30);
        let far = adaptive_scheduler_sleep_secs(Some(7200));
        assert!((60..=300).contains(&far));
        assert_eq!(adaptive_scheduler_sleep_secs(None), 120);
    }
}
