//! Tauri command handlers

use crate::audio::AudioState;
use chrono::{Datelike, Duration, Local, NaiveDate};
use nano_pray_core::agent::{self, AgentNextPrayer};
use nano_pray_core::config::ReminderConfig;
use nano_pray_core::location::CityDatabase;
use nano_pray_core::prelude::*;
use nano_pray_core::statistics::PrayerLog;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, State};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::Notify;

pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub city_db: Mutex<CityDatabase>,
    pub prayer_log: Mutex<PrayerLog>,
    pub active_alert: Mutex<Option<ActiveAlertPayload>>,
    pub muted: Mutex<bool>,
    pub scheduler_wakeup: Arc<Notify>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActiveAlertPayload {
    pub prayer: String,
    pub alert_type: String,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Serialize)]
pub struct PrayerTimesResponse {
    pub date: String,
    pub location_name: Option<String>,
    pub fajr: String,
    pub sunrise: String,
    pub dhuhr: String,
    pub asr: String,
    pub maghrib: String,
    pub isha: String,
    pub current_prayer: Option<String>,
    pub next_prayer: Option<String>,
    pub minutes_to_next: Option<i32>,
    pub qibla_direction: f64,
}

#[tauri::command]
pub async fn get_prayer_times(
    state: State<'_, AppState>,
    latitude: Option<f64>,
    longitude: Option<f64>,
) -> std::result::Result<PrayerTimesResponse, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;

    let (lat, lng, name) = if let (Some(lat), Some(lng)) = (latitude, longitude) {
        (lat, lng, None)
    } else if let Some(loc) = config.current_location() {
        (
            loc.coordinates.latitude,
            loc.coordinates.longitude,
            Some(loc.name.clone()),
        )
    } else {
        (21.4225, 39.8262, Some("Makkah".to_string()))
    };

    let tz = if latitude.is_some() && longitude.is_some() {
        // Explicit coords: no city timezone unless current location matches.
        config.effective_timezone()
    } else {
        config.effective_timezone()
    };
    let calc = PrayerCalculator::new()
        .with_method(config.effective_calculation_method())
        .with_madhab(config.asr_madhab)
        .with_high_latitude_rule(config.high_latitude_rule)
        .with_adjustments(config.prayer_adjustments)
        .with_timezone(tz);

    let times = calc
        .calculate_today(lat, lng, name)
        .map_err(|e| e.to_string())?;
    let qibla = QiblaDirection::calculate(lat, lng)
        .map(|q| q.degrees_from_north)
        .unwrap_or(0.0);

    Ok(PrayerTimesResponse {
        fajr: times
            .get_time(Prayer::Fajr)
            .map(|t| calc.format_prayer_hm(t.time_utc))
            .unwrap_or_default(),
        sunrise: times
            .get_time(Prayer::Sunrise)
            .map(|t| calc.format_prayer_hm(t.time_utc))
            .unwrap_or_default(),
        dhuhr: times
            .get_time(Prayer::Dhuhr)
            .map(|t| calc.format_prayer_hm(t.time_utc))
            .unwrap_or_default(),
        asr: times
            .get_time(Prayer::Asr)
            .map(|t| calc.format_prayer_hm(t.time_utc))
            .unwrap_or_default(),
        maghrib: times
            .get_time(Prayer::Maghrib)
            .map(|t| calc.format_prayer_hm(t.time_utc))
            .unwrap_or_default(),
        isha: times
            .get_time(Prayer::Isha)
            .map(|t| calc.format_prayer_hm(t.time_utc))
            .unwrap_or_default(),
        current_prayer: times.current_prayer.map(|p| p.name().to_string()),
        next_prayer: times.next_prayer.map(|p| p.name().to_string()),
        minutes_to_next: times.minutes_to_next,
        date: times.date.to_string(),
        location_name: times.location_name,
        qibla_direction: qibla,
    })
}

#[tauri::command]
pub async fn get_monthly_prayer_times(
    state: State<'_, AppState>,
    year: i32,
    month: u32,
) -> std::result::Result<Vec<PrayerTimesResponse>, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;

    let (lat, lng, name) = if let Some(loc) = config.current_location() {
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

    let mut responses = Vec::new();
    let qibla = QiblaDirection::calculate(lat, lng)
        .map(|q| q.degrees_from_north)
        .unwrap_or(0.0);

    let start_date = chrono::NaiveDate::from_ymd_opt(year, month, 1).ok_or("Invalid date")?;
    let next_month = if month == 12 {
        chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .ok_or_else(|| format!("Invalid year/month: {}-{}", year, month))?;

    let days = next_month.signed_duration_since(start_date).num_days();

    for i in 0..days {
        let date = start_date + chrono::Duration::days(i);
        match calc.calculate(date, lat, lng, name.clone()) {
            Ok(times) => {
                responses.push(PrayerTimesResponse {
                    fajr: times
                        .get_time(Prayer::Fajr)
                        .map(|t| calc.format_prayer_hm(t.time_utc))
                        .unwrap_or_default(),
                    sunrise: times
                        .get_time(Prayer::Sunrise)
                        .map(|t| calc.format_prayer_hm(t.time_utc))
                        .unwrap_or_default(),
                    dhuhr: times
                        .get_time(Prayer::Dhuhr)
                        .map(|t| calc.format_prayer_hm(t.time_utc))
                        .unwrap_or_default(),
                    asr: times
                        .get_time(Prayer::Asr)
                        .map(|t| calc.format_prayer_hm(t.time_utc))
                        .unwrap_or_default(),
                    maghrib: times
                        .get_time(Prayer::Maghrib)
                        .map(|t| calc.format_prayer_hm(t.time_utc))
                        .unwrap_or_default(),
                    isha: times
                        .get_time(Prayer::Isha)
                        .map(|t| calc.format_prayer_hm(t.time_utc))
                        .unwrap_or_default(),
                    current_prayer: None,
                    next_prayer: None,
                    minutes_to_next: None,
                    date: times.date.to_string(),
                    location_name: times.location_name.clone(),
                    qibla_direction: qibla,
                });
            }
            Err(e) => tracing::error!("Failed to calc for {}: {}", date, e),
        }
    }

    Ok(responses)
}

#[tauri::command]
pub async fn play_adhan(
    state: State<'_, AudioState>,
    custom_path: Option<String>,
    volume: Option<f32>,
    is_fajr: Option<bool>,
) -> std::result::Result<(), String> {
    let volume = volume.unwrap_or(1.0).clamp(0.0, 1.0);

    if let Some(path_str) = custom_path.filter(|p| !p.trim().is_empty()) {
        let path = PathBuf::from(path_str);
        if !path.exists() {
            return Err(format!("Audio file not found: {}", path.display()));
        }
        state.inner().0.play_file(path, volume)?;
    } else {
        let bytes = if is_fajr.unwrap_or(false) {
            include_bytes!("../../assets/adhan_fajr.mp3").as_slice()
        } else {
            include_bytes!("../../assets/adhan.mp3").as_slice()
        };
        state.inner().0.play_embedded(bytes, volume)?;
    }

    Ok(())
}

#[tauri::command]
pub async fn stop_audio(state: State<'_, AudioState>) -> std::result::Result<(), String> {
    state.inner().0.stop();
    Ok(())
}

#[tauri::command]
pub async fn play_reminder_sound(
    state: State<'_, AudioState>,
    custom_path: Option<String>,
    volume: Option<f32>,
) -> std::result::Result<(), String> {
    let volume = volume.unwrap_or(1.0).clamp(0.0, 1.0);

    if let Some(path_str) = custom_path.filter(|p| !p.trim().is_empty()) {
        let path = PathBuf::from(path_str);
        if !path.exists() {
            return Err(format!("Audio file not found: {}", path.display()));
        }
        state.inner().0.play_file(path, volume)?;
    } else {
        let bytes = include_bytes!("../../assets/classic_alarm.mp3").as_slice();
        state.inner().0.play_embedded(bytes, volume)?;
    }

    Ok(())
}

#[tauri::command]
pub async fn dismiss_alert(
    app: tauri::AppHandle,
    app_state: State<'_, AppState>,
    state: State<'_, AudioState>,
) -> std::result::Result<(), String> {
    if let Ok(mut guard) = app_state.active_alert.lock() {
        *guard = None;
    }
    state.inner().0.stop();
    app.emit("prayer-alert-dismissed", ())
        .map_err(|e| e.to_string())?;
    if let Some(alert_win) = app.get_webview_window("alert") {
        let _ = alert_win.hide();
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct StatisticsResponse {
    pub today: PeriodStatistics,
    pub week: PeriodStatistics,
    pub month: PeriodStatistics,
    pub year: PeriodStatistics,
    pub all_time: PeriodStatistics,
    pub total_prayers_logged: u32,
    pub current_streak: u32,
    pub longest_streak: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct PeriodStatistics {
    pub label: String,
    pub start_date: String,
    pub end_date: String,
    pub completed_count: u32,
    pub expected_count: u32,
    pub completion_rate_percentage: f32,
    pub per_prayer_completion: HashMap<String, f32>,
    pub per_prayer_completed: HashMap<String, u32>,
    pub per_prayer_expected: HashMap<String, u32>,
    pub timeline: Vec<TimelinePoint>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelinePoint {
    pub label: String,
    pub completed_count: u32,
    pub expected_count: u32,
    pub completion_rate_percentage: f32,
}

fn parse_prayer_name(prayer: &str) -> Option<Prayer> {
    Prayer::parse(prayer)
}

#[tauri::command]
pub fn mark_prayer_completed(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    audio_state: State<'_, AudioState>,
    prayer: String,
) -> std::result::Result<(), String> {
    let parsed = parse_prayer_name(&prayer).ok_or_else(|| "Invalid prayer name".to_string())?;
    let today = chrono::Local::now().date_naive();
    let mut log = state.prayer_log.lock().map_err(|e| e.to_string())?;
    log.mark_completed(today, parsed);
    log.save().map_err(|e| e.to_string())?;
    drop(log);
    if let Ok(mut guard) = state.active_alert.lock() {
        *guard = None;
    }
    audio_state.inner().0.stop();
    app.emit("statistics-updated", ())
        .map_err(|e| e.to_string())?;
    app.emit("prayer-alert-dismissed", ())
        .map_err(|e| e.to_string())?;
    if let Some(alert_win) = app.get_webview_window("alert") {
        let _ = alert_win.hide();
    }
    Ok(())
}

#[tauri::command]
pub fn get_active_alert(
    state: State<'_, AppState>,
) -> std::result::Result<Option<ActiveAlertPayload>, String> {
    state
        .active_alert
        .lock()
        .map_err(|e| e.to_string())
        .map(|payload| payload.clone())
}

#[tauri::command]
pub fn get_statistics(
    state: State<'_, AppState>,
) -> std::result::Result<StatisticsResponse, String> {
    let log = state.prayer_log.lock().map_err(|e| e.to_string())?;
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();
    let today = Local::now().date_naive();
    let start_of_week = today - Duration::days(today.weekday().num_days_from_monday() as i64);
    let start_of_month = today.with_day(1).ok_or("Invalid current month")?;
    let start_of_year = today.with_ordinal(1).ok_or("Invalid current year")?;
    let all_time_start = log
        .entries()
        .iter()
        .map(|entry| entry.date)
        .min()
        .unwrap_or(today);

    let day_stats = build_day_statistics(log.entries(), &config, all_time_start, today)?;

    let today_stats = build_period_statistics("Today", &day_stats, today, today);
    let week_stats = build_period_statistics("This Week", &day_stats, start_of_week, today);
    let month_stats = build_period_statistics("This Month", &day_stats, start_of_month, today);
    let year_stats = build_period_statistics("This Year", &day_stats, start_of_year, today);
    let all_time_stats = build_period_statistics("All Time", &day_stats, all_time_start, today);
    let (current_streak, longest_streak) = build_streaks(&day_stats, all_time_start, today);

    Ok(StatisticsResponse {
        today: today_stats,
        week: week_stats,
        month: month_stats,
        year: year_stats,
        all_time: all_time_stats,
        total_prayers_logged: log.completed_count(),
        current_streak,
        longest_streak,
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct DayStats {
    completed_count: u32,
    expected_count: u32,
    per_prayer_completed: [u32; 6],
    per_prayer_expected: [u32; 6],
}

const TRACKED_PRAYERS: [Prayer; 6] = [
    Prayer::Fajr,
    Prayer::Sunrise,
    Prayer::Dhuhr,
    Prayer::Asr,
    Prayer::Maghrib,
    Prayer::Isha,
];

const fn prayer_index(prayer: Prayer) -> usize {
    match prayer {
        Prayer::Fajr => 0,
        Prayer::Sunrise => 1,
        Prayer::Dhuhr => 2,
        Prayer::Asr => 3,
        Prayer::Maghrib => 4,
        Prayer::Isha => 5,
    }
}

const _: () = {
    assert!(
        TRACKED_PRAYERS.len() == 6,
        "TRACKED_PRAYERS must have exactly 6 entries"
    );
    let prayers = [
        Prayer::Fajr,
        Prayer::Sunrise,
        Prayer::Dhuhr,
        Prayer::Asr,
        Prayer::Maghrib,
        Prayer::Isha,
    ];
    let mut i = 0;
    while i < prayers.len() {
        assert!(
            prayer_index(prayers[i]) == i,
            "prayer_index must be consistent with TRACKED_PRAYERS order"
        );
        i += 1;
    }
};

fn build_day_statistics(
    entries: &[nano_pray_core::statistics::PrayerEntry],
    config: &AppConfig,
    start: NaiveDate,
    end: NaiveDate,
) -> std::result::Result<BTreeMap<NaiveDate, DayStats>, String> {
    let (lat, lng) = if let Some(loc) = config.current_location() {
        (loc.coordinates.latitude, loc.coordinates.longitude)
    } else {
        (21.4225, 39.8262)
    };

    let calc = PrayerCalculator::new()
        .with_method(config.effective_calculation_method())
        .with_madhab(config.asr_madhab)
        .with_high_latitude_rule(config.high_latitude_rule)
        .with_adjustments(config.prayer_adjustments)
        .with_timezone(config.effective_timezone());

    let mut stats = BTreeMap::new();
    let mut date = start;

    while date <= end {
        let mut day = DayStats::default();
        let prayer_times = calc
            .calculate(date, lat, lng, None)
            .map_err(|e| e.to_string())?;

        let now = Local::now();
        for prayer_info in &prayer_times.prayers {
            let due = if date < end {
                true
            } else {
                prayer_info.time <= now
            };

            if due {
                let idx = prayer_index(prayer_info.prayer);
                day.expected_count += 1;
                day.per_prayer_expected[idx] += 1;
            }
        }

        for entry in entries
            .iter()
            .filter(|entry| entry.date == date && entry.completed)
        {
            let idx = prayer_index(entry.prayer);
            day.completed_count += 1;
            day.per_prayer_completed[idx] += 1;
        }

        stats.insert(date, day);
        date += Duration::days(1);
    }

    Ok(stats)
}

fn build_period_statistics(
    label: &str,
    day_stats: &BTreeMap<NaiveDate, DayStats>,
    start: NaiveDate,
    end: NaiveDate,
) -> PeriodStatistics {
    let mut completed_count = 0_u32;
    let mut expected_count = 0_u32;
    let mut per_prayer_completed = [0_u32; 6];
    let mut per_prayer_expected = [0_u32; 6];

    let mut days: Vec<(NaiveDate, DayStats)> = Vec::new();
    for (date, stats) in day_stats.range(start..=end) {
        completed_count += stats.completed_count;
        expected_count += stats.expected_count;

        for idx in 0..TRACKED_PRAYERS.len() {
            per_prayer_completed[idx] += stats.per_prayer_completed[idx];
            per_prayer_expected[idx] += stats.per_prayer_expected[idx];
        }

        days.push((*date, *stats));
    }

    let completion_rate_percentage = percentage(completed_count, expected_count);
    let per_prayer_completion = TRACKED_PRAYERS
        .iter()
        .enumerate()
        .map(|(idx, prayer)| {
            (
                prayer.name().to_string(),
                percentage(per_prayer_completed[idx], per_prayer_expected[idx]),
            )
        })
        .collect::<HashMap<_, _>>();

    let per_prayer_completed_map = TRACKED_PRAYERS
        .iter()
        .enumerate()
        .map(|(idx, prayer)| (prayer.name().to_string(), per_prayer_completed[idx]))
        .collect::<HashMap<_, _>>();

    let per_prayer_expected_map = TRACKED_PRAYERS
        .iter()
        .enumerate()
        .map(|(idx, prayer)| (prayer.name().to_string(), per_prayer_expected[idx]))
        .collect::<HashMap<_, _>>();

    PeriodStatistics {
        label: label.to_string(),
        start_date: start.to_string(),
        end_date: end.to_string(),
        completed_count,
        expected_count,
        completion_rate_percentage,
        per_prayer_completion,
        per_prayer_completed: per_prayer_completed_map,
        per_prayer_expected: per_prayer_expected_map,
        timeline: build_timeline(&days),
    }
}

fn build_timeline(days: &[(NaiveDate, DayStats)]) -> Vec<TimelinePoint> {
    if days.is_empty() {
        return Vec::new();
    }

    if days.len() <= 14 {
        return days
            .iter()
            .map(|(date, stats)| TimelinePoint {
                label: date.format("%b %-d").to_string(),
                completed_count: stats.completed_count,
                expected_count: stats.expected_count,
                completion_rate_percentage: percentage(stats.completed_count, stats.expected_count),
            })
            .collect();
    }

    if days.len() <= 62 {
        let mut buckets: BTreeMap<NaiveDate, (u32, u32)> = BTreeMap::new();
        for (date, stats) in days {
            let week_start = *date - Duration::days(date.weekday().num_days_from_monday() as i64);
            let bucket = buckets.entry(week_start).or_insert((0, 0));
            bucket.0 += stats.completed_count;
            bucket.1 += stats.expected_count;
        }

        return buckets
            .into_iter()
            .map(
                |(week_start, (completed_count, expected_count))| TimelinePoint {
                    label: format!("Week of {}", week_start.format("%b %-d")),
                    completed_count,
                    expected_count,
                    completion_rate_percentage: percentage(completed_count, expected_count),
                },
            )
            .collect();
    }

    let mut buckets: BTreeMap<(i32, u32), (u32, u32)> = BTreeMap::new();
    for (date, stats) in days {
        let bucket = buckets.entry((date.year(), date.month())).or_insert((0, 0));
        bucket.0 += stats.completed_count;
        bucket.1 += stats.expected_count;
    }

    buckets
        .into_iter()
        .map(|((year, month), (completed_count, expected_count))| {
            let label = NaiveDate::from_ymd_opt(year, month, 1)
                .map(|date| date.format("%b %Y").to_string())
                .unwrap_or_else(|| format!("{year}-{month:02}"));

            TimelinePoint {
                label,
                completed_count,
                expected_count,
                completion_rate_percentage: percentage(completed_count, expected_count),
            }
        })
        .collect()
}

fn build_streaks(
    day_stats: &BTreeMap<NaiveDate, DayStats>,
    start: NaiveDate,
    end: NaiveDate,
) -> (u32, u32) {
    let mut longest = 0_u32;
    let mut running = 0_u32;

    let mut date = start;
    while date <= end {
        let stats = day_stats.get(&date).copied().unwrap_or_default();
        if stats.expected_count > 0 && stats.completed_count == stats.expected_count {
            running += 1;
            longest = longest.max(running);
        } else if stats.expected_count > 0 {
            running = 0;
        }
        date += Duration::days(1);
    }

    let mut current = 0_u32;
    let mut cursor = end;
    loop {
        let stats = day_stats.get(&cursor).copied().unwrap_or_default();
        if stats.expected_count == 0 {
            if cursor == start {
                break;
            }
            cursor -= Duration::days(1);
            continue;
        }

        if stats.completed_count == stats.expected_count {
            current += 1;
        } else {
            break;
        }

        if cursor == start {
            break;
        }
        cursor -= Duration::days(1);
    }

    (current, longest)
}

fn percentage(completed: u32, expected: u32) -> f32 {
    if expected == 0 {
        0.0
    } else {
        (((completed as f32 / expected as f32) * 1000.0).round()) / 10.0
    }
}

#[tauri::command]
pub async fn pause_audio(state: State<'_, AudioState>) -> std::result::Result<(), String> {
    state.inner().0.pause();
    Ok(())
}

#[tauri::command]
pub async fn resume_audio(state: State<'_, AudioState>) -> std::result::Result<(), String> {
    state.inner().0.resume();
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct QiblaResponse {
    pub degrees: f64,
    pub cardinal: String,
    pub distance_km: f64,
}

#[tauri::command]
pub fn get_qibla_direction(
    latitude: f64,
    longitude: f64,
) -> std::result::Result<QiblaResponse, String> {
    let qibla = QiblaDirection::calculate(latitude, longitude).map_err(|e| e.to_string())?;
    Ok(QiblaResponse {
        degrees: qibla.degrees_from_north,
        cardinal: qibla.cardinal_direction,
        distance_km: qibla.distance_km,
    })
}

#[derive(Debug, Serialize)]
pub struct CityResult {
    pub name: String,
    pub country: String,
    pub latitude: f64,
    pub longitude: f64,
    pub timezone: String,
}

#[tauri::command]
pub fn search_cities(
    state: State<'_, AppState>,
    query: String,
) -> std::result::Result<Vec<CityResult>, String> {
    let db = state.city_db.lock().map_err(|e| e.to_string())?;
    Ok(db
        .search(&query)
        .into_iter()
        .map(|c| CityResult {
            name: c.name.clone(),
            country: c.country.clone(),
            latitude: c.coordinates.latitude,
            longitude: c.coordinates.longitude,
            timezone: c.timezone.clone(),
        })
        .collect())
}

#[tauri::command]
pub fn hide_main_window(app: tauri::AppHandle) -> std::result::Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    crate::gpu_idle::hide_main_for_tray(&window).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn show_main_window(app: tauri::AppHandle) -> std::result::Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    crate::gpu_idle::show_main_from_tray(&window).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> std::result::Result<AppConfig, String> {
    state
        .config
        .lock()
        .map_err(|e| e.to_string())
        .map(|c| c.clone())
}

#[tauri::command]
pub fn save_config(
    state: State<'_, AppState>,
    config: AppConfig,
) -> std::result::Result<(), String> {
    let muted = config.advanced.muted;
    let mut current = state.config.lock().map_err(|e| e.to_string())?;
    *current = config;
    current.save().map_err(|e| e.to_string())?;
    drop(current);
    if let Ok(mut m) = state.muted.lock() {
        *m = muted;
    }
    state.scheduler_wakeup.notify_one();
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct HijriResponse {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub month_name: String,
    pub formatted: String,
    pub formatted_arabic: String,
}

#[tauri::command]
pub fn get_hijri_date(
    state: State<'_, AppState>,
    offset_days: Option<i32>,
) -> std::result::Result<HijriResponse, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    // DST-aware civil day for the city + moon-sighting offset
    let agent = nano_pray_core::agent::hijri_date_for_config(&config, offset_days);
    Ok(HijriResponse {
        year: agent.year,
        month: agent.month,
        day: agent.day,
        month_name: agent.month_name,
        formatted: agent.formatted,
        formatted_arabic: agent.formatted_arabic,
    })
}

#[derive(Debug, Serialize)]
pub struct HijriAlignResult {
    pub location_index: usize,
    pub location_name: String,
    pub offset: i32,
    pub observed: String,
    pub calculated: String,
    pub source: String,
}

/// Align a location's Hijri offset to an observed authority date (moon sighting).
/// Pass `observed_*` from Aladhan / local council; if omitted, uses no-op error.
#[tauri::command]
pub fn align_hijri_for_location(
    state: State<'_, AppState>,
    location_index: Option<usize>,
    observed_year: i32,
    observed_month: u8,
    observed_day: u8,
    auto_align: Option<bool>,
) -> std::result::Result<HijriAlignResult, String> {
    use nano_pray_core::suggest_offset_for_observed;
    use nano_pray_core::time_zone::{civil_date_at, parse_timezone};
    use chrono::Utc;

    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    let idx = location_index.unwrap_or(config.current_location_index);
    if idx >= config.locations.len() {
        return Err(format!("Invalid location index: {idx}"));
    }

    // Align against the city's civil date (summer time / DST included).
    let tz = parse_timezone(&config.locations[idx].timezone);
    let today = civil_date_at(Utc::now(), tz);
    let observed = HijriDate::new(observed_year, observed_month, observed_day);
    let calculated = HijriDate::from_gregorian(today);
    let offset = suggest_offset_for_observed(today, observed).ok_or_else(|| {
        format!(
            "Could not match observed Hijri {}-{}-{} within ±3 days of tabular {}",
            observed_year,
            observed_month,
            observed_day,
            calculated.format()
        )
    })?;

    let name = config.locations[idx].name.clone();
    config.locations[idx].hijri_offset = Some(offset);
    if let Some(auto) = auto_align {
        config.locations[idx].hijri_auto_align = auto;
    }
    // Keep global in sync when aligning the active city for simple UIs.
    if idx == config.current_location_index {
        config.hijri_offset = offset;
    }
    config.save().map_err(|e| e.to_string())?;
    state.scheduler_wakeup.notify_one();

    Ok(HijriAlignResult {
        location_index: idx,
        location_name: name,
        offset,
        observed: observed.format(),
        calculated: calculated.format(),
        source: "observed_authority".into(),
    })
}

#[tauri::command]
pub fn send_notification(
    app: tauri::AppHandle,
    title: String,
    body: String,
) -> std::result::Result<(), String> {
    app.notification()
        .builder()
        .title(&title)
        .body(&body)
        .show()
        .map_err(|e: tauri_plugin_notification::Error| e.to_string())
}

#[tauri::command]
pub fn get_next_prayer(state: State<'_, AppState>) -> std::result::Result<AgentNextPrayer, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();
    agent::next_prayer(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_reminders_muted(
    state: State<'_, AppState>,
    muted: bool,
) -> std::result::Result<(), String> {
    if let Ok(mut muted_guard) = state.muted.lock() {
        *muted_guard = muted;
    }
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    config.advanced.muted = muted;
    config.save().map_err(|e| e.to_string())?;
    state.scheduler_wakeup.notify_one();
    Ok(())
}

#[tauri::command]
pub fn update_reminder_settings(
    state: State<'_, AppState>,
    prayer: String,
    reminder: ReminderConfig,
) -> std::result::Result<(), String> {
    let parsed = parse_prayer_name(&prayer).ok_or_else(|| "Invalid prayer name".to_string())?;
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    config.set_reminder(&parsed.name().to_lowercase(), reminder);
    config.save().map_err(|e| e.to_string())?;
    state.scheduler_wakeup.notify_one();
    Ok(())
}

/// Unified update-check result for the About tab (Tauri + Electron shape).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopUpdateCheck {
    pub available: bool,
    pub current_version: String,
    pub latest_version: String,
    pub release_url: String,
    pub release_notes: String,
    pub published_at: String,
    pub is_prerelease: bool,
    /// "update" | "current" | "ahead" | "empty" | "error"
    pub status: String,
    pub message: String,
    pub error: Option<String>,
}

fn strip_version(v: &str) -> String {
    v.trim().trim_start_matches('v').trim_start_matches('V').to_string()
}

/// Compare semver-ish strings. Returns -1 / 0 / 1.
fn compare_semver(a: &str, b: &str) -> i32 {
    let strip = |s: &str| {
        strip_version(s)
            .split(|c| c == '.' || c == '-' || c == '+')
            .map(|p| p.parse::<u32>().unwrap_or(0))
            .collect::<Vec<_>>()
    };
    let pa = strip(a);
    let pb = strip(b);
    let len = pa.len().max(pb.len());
    for i in 0..len {
        let na = *pa.get(i).unwrap_or(&0);
        let nb = *pb.get(i).unwrap_or(&0);
        if na < nb {
            return -1;
        }
        if na > nb {
            return 1;
        }
    }
    0
}

#[derive(Debug, serde::Deserialize)]
struct GhRelease {
    tag_name: Option<String>,
    html_url: Option<String>,
    body: Option<String>,
    published_at: Option<String>,
    draft: Option<bool>,
    prerelease: Option<bool>,
}

/// Check GitHub releases (including pre-releases). Runs in the native process so
/// WebView CSP cannot block it (renderer fetch fails with "Failed to fetch").
#[tauri::command]
pub async fn desktop_check_update(
    app: tauri::AppHandle,
) -> std::result::Result<DesktopUpdateCheck, String> {
    let current_version = app.package_info().version.to_string();
    let client = reqwest::Client::builder()
        .user_agent("NanoPrayReminder-Tauri")
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get("https://api.github.com/repos/mozayed007/Nano-Prayer/releases")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                "Request timed out. Check your network connection.".to_string()
            } else if e.is_connect() {
                "Could not reach GitHub. Check your network connection.".to_string()
            } else {
                format!("Network error: {e}")
            }
        })?;

    let status = response.status();
    if status.as_u16() == 403 {
        return Ok(DesktopUpdateCheck {
            available: false,
            current_version: current_version.clone(),
            latest_version: String::new(),
            release_url: "https://github.com/mozayed007/Nano-Prayer/releases".into(),
            release_notes: String::new(),
            published_at: String::new(),
            is_prerelease: false,
            status: "error".into(),
            message: "GitHub API rate limited. Try again later.".into(),
            error: Some("rate_limited".into()),
        });
    }
    if status.as_u16() == 404 {
        return Ok(DesktopUpdateCheck {
            available: false,
            current_version: current_version.clone(),
            latest_version: String::new(),
            release_url: "https://github.com/mozayed007/Nano-Prayer/releases".into(),
            release_notes: String::new(),
            published_at: String::new(),
            is_prerelease: false,
            status: "empty".into(),
            message: "No releases found yet.".into(),
            error: None,
        });
    }
    if !status.is_success() {
        return Err(format!(
            "GitHub API returned {status}: {}",
            status.canonical_reason().unwrap_or("error")
        ));
    }

    let releases: Vec<GhRelease> = response.json().await.map_err(|e| e.to_string())?;
    let published: Vec<&GhRelease> = releases
        .iter()
        .filter(|r| r.draft != Some(true) && r.tag_name.as_ref().is_some_and(|t| !t.is_empty()))
        .collect();

    if published.is_empty() {
        return Ok(DesktopUpdateCheck {
            available: false,
            current_version: current_version.clone(),
            latest_version: String::new(),
            release_url: "https://github.com/mozayed007/Nano-Prayer/releases".into(),
            release_notes: String::new(),
            published_at: String::new(),
            is_prerelease: false,
            status: "empty".into(),
            message: "No public releases available yet.".into(),
            error: None,
        });
    }

    // GitHub list is newest-first; pick first non-draft as "latest" (stable or pre).
    let latest = published[0];
    let latest_version = strip_version(latest.tag_name.as_deref().unwrap_or(""));
    let is_prerelease = latest.prerelease.unwrap_or(false);
    let cmp = compare_semver(&current_version, &latest_version);

    let (available, status_code, message) = if cmp < 0 {
        let kind = if is_prerelease {
            " (pre-release)"
        } else {
            ""
        };
        (
            true,
            "update",
            format!("Version {latest_version}{kind} is available!"),
        )
    } else if cmp == 0 {
        let kind = if is_prerelease {
            " You are on the latest pre-release."
        } else {
            ""
        };
        (
            false,
            "current",
            format!("You are on the latest version.{kind}"),
        )
    } else {
        (
            false,
            "ahead",
            format!(
                "You are on {current_version}, which is newer than the latest public release ({latest_version})."
            ),
        )
    };

    Ok(DesktopUpdateCheck {
        available,
        current_version,
        latest_version,
        release_url: latest
            .html_url
            .clone()
            .unwrap_or_else(|| "https://github.com/mozayed007/Nano-Prayer/releases".into()),
        release_notes: latest.body.clone().unwrap_or_default(),
        published_at: latest.published_at.clone().unwrap_or_default(),
        is_prerelease,
        status: status_code.into(),
        message,
        error: None,
    })
}

#[cfg(test)]
mod update_tests {
    use super::compare_semver;

    #[test]
    fn compare_semver_orders_versions() {
        assert_eq!(compare_semver("0.1.5", "0.1.6"), -1);
        assert_eq!(compare_semver("0.1.6", "0.1.6"), 0);
        assert_eq!(compare_semver("v0.1.6", "0.1.6"), 0);
        assert_eq!(compare_semver("0.2.0", "0.1.9"), 1);
    }
}
