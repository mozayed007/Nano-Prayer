//! Agent-facing data shapes and helpers.
//!
//! These helpers keep desktop commands, CLI output, and MCP tools anchored to the
//! same Rust prayer engine instead of duplicating prayer logic per integration.

use chrono::{Datelike, Local, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::config::{AppConfig, ReminderConfig};
use crate::error::{Error, Result};
use crate::location::{CityDatabase, Coordinates};
use crate::prayer::{Prayer, PrayerCalculator};
use crate::qibla::QiblaDirection;
use crate::statistics::PrayerLog;
use crate::HijriDate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLocation {
    pub name: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPrayerEntry {
    pub prayer: String,
    pub time: String,
    pub time_local: String,
    pub time_utc: String,
    pub has_passed: bool,
    pub minutes_until: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPrayerTimes {
    pub date: String,
    pub location: AgentLocation,
    pub prayers: Vec<AgentPrayerEntry>,
    pub current_prayer: Option<String>,
    pub next_prayer: Option<String>,
    pub minutes_to_next: Option<i32>,
    pub qibla_direction: AgentQibla,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNextPrayer {
    pub date: String,
    pub location: AgentLocation,
    pub current_prayer: Option<String>,
    pub next_prayer: Option<String>,
    pub next_prayer_time: Option<String>,
    pub minutes_to_next: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentQibla {
    pub degrees: f64,
    pub cardinal: String,
    pub distance_km: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCityResult {
    pub name: String,
    pub country: String,
    pub country_code: String,
    pub latitude: f64,
    pub longitude: f64,
    pub timezone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHijriDate {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub month_name: String,
    pub formatted: String,
    pub formatted_arabic: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMutation {
    pub ok: bool,
    pub message: String,
}

pub fn prayer_times(
    config: &AppConfig,
    date: NaiveDate,
    latitude: Option<f64>,
    longitude: Option<f64>,
    location_name: Option<String>,
) -> Result<AgentPrayerTimes> {
    let location = resolve_location(config, latitude, longitude, location_name)?;
    let calc = calculator_from_config(config);
    let times = calc.calculate(
        date,
        location.latitude,
        location.longitude,
        location.name.clone(),
    )?;
    let qibla = qibla_direction(location.latitude, location.longitude)?;

    Ok(AgentPrayerTimes {
        date: times.date.to_string(),
        location,
        prayers: times
            .prayers
            .iter()
            .map(|prayer| AgentPrayerEntry {
                prayer: prayer.prayer.name().to_string(),
                time: prayer.time.format("%H:%M").to_string(),
                time_local: prayer.time.to_rfc3339(),
                time_utc: prayer.time_utc.to_rfc3339(),
                has_passed: prayer.has_passed,
                minutes_until: prayer.minutes_until,
            })
            .collect(),
        current_prayer: times.current_prayer.map(|prayer| prayer.name().to_string()),
        next_prayer: times.next_prayer.map(|prayer| prayer.name().to_string()),
        minutes_to_next: times.minutes_to_next,
        qibla_direction: qibla,
    })
}

pub fn monthly_prayer_times(
    config: &AppConfig,
    year: i32,
    month: u32,
    latitude: Option<f64>,
    longitude: Option<f64>,
) -> Result<Vec<AgentPrayerTimes>> {
    let start = NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or_else(|| Error::DateTime(format!("Invalid year/month: {year}-{month:02}")))?;
    let next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .ok_or_else(|| Error::DateTime(format!("Invalid year/month: {year}-{month:02}")))?;

    let mut rows = Vec::new();
    let mut date = start;
    while date < next_month {
        rows.push(prayer_times(config, date, latitude, longitude, None)?);
        date += chrono::Duration::days(1);
    }
    Ok(rows)
}

pub fn next_prayer(config: &AppConfig) -> Result<AgentNextPrayer> {
    let today = Local::now().date_naive();
    let times = prayer_times(config, today, None, None, None)?;
    let mut next_prayer_time = times
        .next_prayer
        .as_ref()
        .and_then(|next| {
            times
                .prayers
                .iter()
                .find(|entry| entry.prayer == *next && !entry.has_passed)
        })
        .map(|entry| entry.time_local.clone());

    if next_prayer_time.is_none() && times.next_prayer.as_deref() == Some("Fajr") {
        let tomorrow = prayer_times(
            config,
            today + chrono::Duration::days(1),
            Some(times.location.latitude),
            Some(times.location.longitude),
            times.location.name.clone(),
        )?;
        next_prayer_time = tomorrow
            .prayers
            .iter()
            .find(|entry| entry.prayer == "Fajr")
            .map(|entry| entry.time_local.clone());
    }

    Ok(AgentNextPrayer {
        date: times.date,
        location: times.location,
        current_prayer: times.current_prayer,
        next_prayer: times.next_prayer,
        next_prayer_time,
        minutes_to_next: times.minutes_to_next,
    })
}

pub fn qibla_direction(latitude: f64, longitude: f64) -> Result<AgentQibla> {
    validate_coordinates(latitude, longitude)?;
    let qibla = QiblaDirection::calculate(latitude, longitude)?;
    Ok(AgentQibla {
        degrees: qibla.degrees_from_north,
        cardinal: qibla.cardinal_direction,
        distance_km: qibla.distance_km,
    })
}

pub fn search_cities(query: &str, limit: usize) -> Vec<AgentCityResult> {
    let db = CityDatabase::new();
    db.search(query)
        .into_iter()
        .take(limit)
        .map(|city| AgentCityResult {
            name: city.name.clone(),
            country: city.country.clone(),
            country_code: city.country_code.clone(),
            latitude: city.coordinates.latitude,
            longitude: city.coordinates.longitude,
            timezone: city.timezone.clone(),
        })
        .collect()
}

pub fn hijri_date(offset_days: Option<i32>) -> AgentHijriDate {
    let date = offset_days
        .map(|offset| Local::now().date_naive() + chrono::Duration::days(offset as i64))
        .unwrap_or_else(|| Local::now().date_naive());
    let hijri = HijriDate::from_gregorian(date);
    AgentHijriDate {
        year: hijri.year,
        month: hijri.month,
        day: hijri.day,
        month_name: hijri.month_name_english().to_string(),
        formatted: hijri.format(),
        formatted_arabic: hijri.format_arabic(),
    }
}

/// Hijri for today using config effective offset (per-city moon alignment).
/// Uses the **location civil date** (DST-aware) when a timezone is configured.
pub fn hijri_date_for_config(config: &AppConfig, offset_override: Option<i32>) -> AgentHijriDate {
    use crate::time_zone::{civil_date_at, parse_timezone};
    let offset = offset_override.unwrap_or_else(|| config.effective_hijri_offset());
    let tz = config
        .effective_timezone()
        .as_deref()
        .and_then(parse_timezone);
    let civil = civil_date_at(Utc::now(), tz);
    let date = civil + chrono::Duration::days(offset as i64);
    let hijri = HijriDate::from_gregorian(date);
    AgentHijriDate {
        year: hijri.year,
        month: hijri.month,
        day: hijri.day,
        month_name: hijri.month_name_english().to_string(),
        formatted: hijri.format(),
        formatted_arabic: hijri.format_arabic(),
    }
}

pub fn set_muted(muted: bool) -> Result<AgentMutation> {
    let mut config = AppConfig::load().unwrap_or_default();
    config.advanced.muted = muted;
    config.save()?;
    Ok(AgentMutation {
        ok: true,
        message: if muted {
            "Reminders muted".to_string()
        } else {
            "Reminders unmuted".to_string()
        },
    })
}

pub fn update_reminder(prayer_name: &str, reminder: ReminderConfig) -> Result<AgentMutation> {
    let prayer = Prayer::parse(prayer_name)
        .ok_or_else(|| Error::ReminderScheduling(format!("Invalid prayer: {prayer_name}")))?;
    let mut config = AppConfig::load().unwrap_or_default();
    config.set_reminder(&prayer.name().to_lowercase(), reminder);
    config.save()?;
    Ok(AgentMutation {
        ok: true,
        message: format!("Updated {} reminder", prayer.name()),
    })
}

pub fn mark_prayer_completed(prayer_name: &str, date: Option<NaiveDate>) -> Result<AgentMutation> {
    let prayer = Prayer::parse(prayer_name)
        .ok_or_else(|| Error::ReminderScheduling(format!("Invalid prayer: {prayer_name}")))?;
    let mut log = PrayerLog::load().unwrap_or_default();
    log.mark_completed(date.unwrap_or_else(|| Local::now().date_naive()), prayer);
    log.save()?;
    Ok(AgentMutation {
        ok: true,
        message: format!("Marked {} completed", prayer.name()),
    })
}

pub fn config_summary(config: &AppConfig) -> serde_json::Value {
    serde_json::json!({
        "current_location": config.current_location(),
        "calculation_method": config.calculation_method,
        "asr_madhab": config.asr_madhab,
        "high_latitude_rule": config.high_latitude_rule,
        "hijri_offset": config.hijri_offset,
        "show_hijri": config.show_hijri,
        "notifications_enabled": config.notifications.enabled,
        "muted": config.advanced.muted,
        "reminders": config.reminders,
    })
}

fn calculator_from_config(config: &AppConfig) -> PrayerCalculator {
    PrayerCalculator::new()
        .with_method(config.effective_calculation_method())
        .with_madhab(config.asr_madhab)
        .with_high_latitude_rule(config.high_latitude_rule)
        .with_adjustments(config.prayer_adjustments)
        .with_timezone(config.effective_timezone())
}

fn resolve_location(
    config: &AppConfig,
    latitude: Option<f64>,
    longitude: Option<f64>,
    location_name: Option<String>,
) -> Result<AgentLocation> {
    match (latitude, longitude) {
        (Some(lat), Some(lng)) => {
            validate_coordinates(lat, lng)?;
            Ok(AgentLocation {
                name: location_name,
                latitude: lat,
                longitude: lng,
                timezone: None,
            })
        }
        (None, None) => {
            if let Some(location) = config.current_location() {
                validate_coordinates(
                    location.coordinates.latitude,
                    location.coordinates.longitude,
                )?;
                Ok(AgentLocation {
                    name: Some(location.name.clone()),
                    latitude: location.coordinates.latitude,
                    longitude: location.coordinates.longitude,
                    timezone: Some(location.timezone.clone()),
                })
            } else {
                Ok(AgentLocation {
                    name: Some("Makkah".to_string()),
                    latitude: 21.4225,
                    longitude: 39.8262,
                    timezone: Some("Asia/Riyadh".to_string()),
                })
            }
        }
        _ => Err(Error::InvalidCoordinates {
            lat: latitude.unwrap_or(f64::NAN),
            long: longitude.unwrap_or(f64::NAN),
        }),
    }
}

fn validate_coordinates(latitude: f64, longitude: f64) -> Result<()> {
    let coordinates = Coordinates::new(latitude, longitude);
    if coordinates.is_valid() {
        Ok(())
    } else {
        Err(Error::InvalidCoordinates {
            lat: latitude,
            long: longitude,
        })
    }
}

pub fn current_month(config: &AppConfig) -> Result<Vec<AgentPrayerTimes>> {
    let today = Local::now().date_naive();
    monthly_prayer_times(config, today.year(), today.month(), None, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prayer_times_for_makkah_returns_six_entries() {
        let config = AppConfig::default();
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).expect("date");
        let times = prayer_times(&config, date, Some(21.4225), Some(39.8262), Some("Makkah".into()))
            .expect("times");
        assert_eq!(times.prayers.len(), 6);
        assert_eq!(times.location.latitude, 21.4225);
        assert!(times.qibla_direction.distance_km < 1.0);
    }

    #[test]
    fn qibla_cairo_is_southeastish() {
        let q = qibla_direction(30.0444, 31.2357).expect("qibla");
        assert!(q.degrees > 90.0 && q.degrees < 180.0);
        assert!(q.distance_km > 1000.0);
    }

    #[test]
    fn hijri_date_is_not_gregorian_year() {
        let h = hijri_date(None);
        assert!(h.year < 2000);
        assert!(!h.formatted.is_empty());
        assert!(!h.formatted_arabic.is_empty());
    }

    #[test]
    fn search_cities_finds_cairo() {
        let hits = search_cities("Cairo", 5);
        assert!(!hits.is_empty());
        assert!(hits.iter().any(|c| c.name.contains("Cairo")));
    }
}
