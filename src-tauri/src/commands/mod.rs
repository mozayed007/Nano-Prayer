//! Tauri command handlers

use crate::audio::AudioState;
use nano_pray_core::location::CityDatabase;
use nano_pray_core::prelude::*;
use nano_pray_core::statistics::PrayerLog;
use serde::Serialize;
use std::collections::HashMap;
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

    let calc = PrayerCalculator::new()
        .with_method(config.calculation_method)
        .with_madhab(config.asr_madhab)
        .with_high_latitude_rule(config.high_latitude_rule)
        .with_adjustments(config.prayer_adjustments);

    let times = calc
        .calculate_today(lat, lng, name)
        .map_err(|e| e.to_string())?;
    let qibla = QiblaDirection::calculate(lat, lng)
        .map(|q| q.degrees_from_north)
        .unwrap_or(0.0);

    Ok(PrayerTimesResponse {
        fajr: times
            .get_time(Prayer::Fajr)
            .map(|t| t.time.format("%H:%M").to_string())
            .unwrap_or_default(),
        sunrise: times
            .get_time(Prayer::Sunrise)
            .map(|t| t.time.format("%H:%M").to_string())
            .unwrap_or_default(),
        dhuhr: times
            .get_time(Prayer::Dhuhr)
            .map(|t| t.time.format("%H:%M").to_string())
            .unwrap_or_default(),
        asr: times
            .get_time(Prayer::Asr)
            .map(|t| t.time.format("%H:%M").to_string())
            .unwrap_or_default(),
        maghrib: times
            .get_time(Prayer::Maghrib)
            .map(|t| t.time.format("%H:%M").to_string())
            .unwrap_or_default(),
        isha: times
            .get_time(Prayer::Isha)
            .map(|t| t.time.format("%H:%M").to_string())
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
        .with_method(config.calculation_method)
        .with_madhab(config.asr_madhab)
        .with_high_latitude_rule(config.high_latitude_rule)
        .with_adjustments(config.prayer_adjustments);

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
    .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(year, 12, 31).unwrap());

    let days = next_month.signed_duration_since(start_date).num_days();

    for i in 0..days {
        let date = start_date + chrono::Duration::days(i);
        match calc.calculate(date, lat, lng, name.clone()) {
            Ok(times) => {
                responses.push(PrayerTimesResponse {
                    fajr: times
                        .get_time(Prayer::Fajr)
                        .map(|t| t.time.format("%H:%M").to_string())
                        .unwrap_or_default(),
                    sunrise: times
                        .get_time(Prayer::Sunrise)
                        .map(|t| t.time.format("%H:%M").to_string())
                        .unwrap_or_default(),
                    dhuhr: times
                        .get_time(Prayer::Dhuhr)
                        .map(|t| t.time.format("%H:%M").to_string())
                        .unwrap_or_default(),
                    asr: times
                        .get_time(Prayer::Asr)
                        .map(|t| t.time.format("%H:%M").to_string())
                        .unwrap_or_default(),
                    maghrib: times
                        .get_time(Prayer::Maghrib)
                        .map(|t| t.time.format("%H:%M").to_string())
                        .unwrap_or_default(),
                    isha: times
                        .get_time(Prayer::Isha)
                        .map(|t| t.time.format("%H:%M").to_string())
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
    _app: tauri::AppHandle,
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
    pub total_prayers_logged: u32,
    pub completion_rate_percentage: f32,
    pub current_streak: u32,
    pub longest_streak: u32,
    pub per_prayer_completion: HashMap<String, f32>,
}

fn parse_prayer_name(prayer: &str) -> Option<Prayer> {
    match prayer.trim().to_lowercase().as_str() {
        "fajr" => Some(Prayer::Fajr),
        "sunrise" => Some(Prayer::Sunrise),
        "dhuhr" => Some(Prayer::Dhuhr),
        "asr" => Some(Prayer::Asr),
        "maghrib" => Some(Prayer::Maghrib),
        "isha" => Some(Prayer::Isha),
        _ => None,
    }
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
pub fn get_statistics(state: State<'_, AppState>) -> std::result::Result<StatisticsResponse, String> {
    let log = state.prayer_log.lock().map_err(|e| e.to_string())?;
    let entries = log.entries();
    let total = entries.len() as u32;
    let completed = entries.iter().filter(|entry| entry.completed).count() as u32;
    let rate = if total > 0 {
        (completed as f32 / total as f32) * 100.0
    } else {
        0.0
    };

    let mut per_prayer_totals: HashMap<String, u32> = HashMap::new();
    let mut per_prayer_completed: HashMap<String, u32> = HashMap::new();
    for entry in entries {
        let name = entry.prayer.name().to_string();
        *per_prayer_totals.entry(name.clone()).or_insert(0) += 1;
        if entry.completed {
            *per_prayer_completed.entry(name).or_insert(0) += 1;
        }
    }
    let all_names = ["Fajr", "Sunrise", "Dhuhr", "Asr", "Maghrib", "Isha"];
    let mut per_prayer_completion: HashMap<String, f32> = HashMap::new();
    for name in all_names {
        let t = per_prayer_totals.get(name).copied().unwrap_or(0);
        let c = per_prayer_completed.get(name).copied().unwrap_or(0);
        let value = if t > 0 {
            (c as f32 / t as f32) * 100.0
        } else {
            0.0
        };
        per_prayer_completion.insert(name.to_string(), value);
    }

    let mut by_day: HashMap<chrono::NaiveDate, (u32, u32)> = HashMap::new();
    for entry in entries {
        let day = by_day.entry(entry.date).or_insert((0, 0));
        day.0 += 1;
        if entry.completed {
            day.1 += 1;
        }
    }
    let mut days: Vec<chrono::NaiveDate> = by_day.keys().copied().collect();
    days.sort_unstable();

    let mut longest_streak = 0_u32;
    let mut running_longest = 0_u32;
    let mut previous: Option<chrono::NaiveDate> = None;
    for day in &days {
        let (tracked, done) = by_day.get(day).copied().unwrap_or((0, 0));
        let successful = tracked > 0 && tracked == done;
        if !successful {
            running_longest = 0;
            previous = Some(*day);
            continue;
        }
        let continues = previous
            .map(|prev| *day == prev + chrono::Duration::days(1))
            .unwrap_or(false);
        if continues {
            running_longest += 1;
        } else {
            running_longest = 1;
        }
        if running_longest > longest_streak {
            longest_streak = running_longest;
        }
        previous = Some(*day);
    }

    let mut current_streak = 0_u32;
    let mut expected = chrono::Local::now().date_naive();
    for day in days.iter().rev() {
        if *day > expected {
            continue;
        }
        if *day != expected {
            break;
        }
        let (tracked, done) = by_day.get(day).copied().unwrap_or((0, 0));
        if tracked == 0 || tracked != done {
            break;
        }
        current_streak += 1;
        expected -= chrono::Duration::days(1);
    }

    Ok(StatisticsResponse {
        total_prayers_logged: total,
        completion_rate_percentage: (rate * 10.0).round() / 10.0,
        current_streak,
        longest_streak,
        per_prayer_completion,
    })
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
    let mut current = state.config.lock().map_err(|e| e.to_string())?;
    *current = config;
    current.save().map_err(|e| e.to_string())?;
    drop(current);
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
pub fn get_hijri_date(offset_days: Option<i32>) -> std::result::Result<HijriResponse, String> {
    use chrono::{Duration, Local};
    let hijri = if let Some(offset) = offset_days {
        let today = Local::now().date_naive();
        let adjusted = today + Duration::days(offset as i64);
        HijriDate::from_gregorian(adjusted)
    } else {
        HijriDate::today()
    };
    Ok(HijriResponse {
        year: hijri.year,
        month: hijri.month,
        day: hijri.day,
        month_name: hijri.month_name_english().to_string(),
        formatted: hijri.format(),
        formatted_arabic: hijri.format_arabic(),
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
