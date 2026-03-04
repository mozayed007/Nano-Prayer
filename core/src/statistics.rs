//! Prayer statistics module

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::prayer::Prayer;

/// Prayer entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrayerEntry {
    pub date: NaiveDate,
    pub prayer: Prayer,
    pub completed: bool,
    pub logged_at: DateTime<Utc>,
}

impl PrayerEntry {
    pub fn new(date: NaiveDate, prayer: Prayer, completed: bool) -> Self {
        Self { date, prayer, completed, logged_at: Utc::now() }
    }

    pub fn completed(date: NaiveDate, prayer: Prayer) -> Self {
        Self::new(date, prayer, true)
    }

    pub fn missed(date: NaiveDate, prayer: Prayer) -> Self {
        Self::new(date, prayer, false)
    }
}

/// Statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Statistics {
    pub total_tracked: u32,
    pub completed: u32,
    pub missed: u32,
    pub completion_rate: f32,
    pub current_streak: u32,
    pub longest_streak: u32,
}

/// Prayer log
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PrayerLog {
    entries: Vec<PrayerEntry>,
}

impl PrayerLog {
    pub fn new() -> Self { Self::default() }

    pub fn log(&mut self, entry: PrayerEntry) {
        self.entries.retain(|e| !(e.date == entry.date && e.prayer == entry.prayer));
        self.entries.push(entry);
        self.entries.sort_by(|a, b| a.date.cmp(&b.date));
    }

    pub fn mark_completed(&mut self, date: NaiveDate, prayer: Prayer) {
        self.log(PrayerEntry::completed(date, prayer));
    }

    pub fn mark_missed(&mut self, date: NaiveDate, prayer: Prayer) {
        self.log(PrayerEntry::missed(date, prayer));
    }

    pub fn for_date(&self, date: NaiveDate) -> Vec<&PrayerEntry> {
        self.entries.iter().filter(|e| e.date == date).collect()
    }

    pub fn is_completed(&self, date: NaiveDate, prayer: Prayer) -> bool {
        self.entries.iter().any(|e| e.date == date && e.prayer == prayer && e.completed)
    }

    pub fn calculate_statistics(&self, start: NaiveDate, end: NaiveDate) -> Statistics {
        let entries: Vec<_> = self.entries.iter().filter(|e| e.date >= start && e.date <= end).collect();

        let total = entries.len() as u32;
        let completed = entries.iter().filter(|e| e.completed).count() as u32;
        let missed = total - completed;
        let rate = if total > 0 { (completed as f32 / total as f32) * 100.0 } else { 0.0 };

        Statistics {
            total_tracked: total,
            completed,
            missed,
            completion_rate: rate,
            current_streak: 0,
            longest_streak: 0,
        }
    }

    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }

    pub fn entries(&self) -> &[PrayerEntry] { &self.entries }

    pub fn load() -> Result<Self> {
        let log: Self = confy::load("nano-pray-reminder", "prayer-log")?;
        Ok(log)
    }

    pub fn save(&self) -> Result<()> {
        confy::store("nano-pray-reminder", "prayer-log", self)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log() {
        let mut log = PrayerLog::new();
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        log.mark_completed(date, Prayer::Fajr);
        assert!(log.is_completed(date, Prayer::Fajr));
    }
}
