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
        let current_time = now.time();

        if self.quiet_hours_start < self.quiet_hours_end {
            current_time >= self.quiet_hours_start && current_time < self.quiet_hours_end
        } else {
            current_time >= self.quiet_hours_start || current_time < self.quiet_hours_end
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reminder() {
        let time = Local::now() + Duration::hours(2);
        let reminder = Reminder::before(Prayer::Fajr, time, 15);
        assert_eq!(reminder.prayer, Prayer::Fajr);
    }
}
