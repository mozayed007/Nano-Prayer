//! Tauri command handlers

use nano_pray_core::location::CityDatabase;
use nano_pray_core::prelude::*;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};
use tauri_plugin_notification::NotificationExt;
use crate::audio::AudioState;

pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub city_db: Mutex<CityDatabase>,
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
pub async fn get_prayer_times(state: State<'_, AppState>, latitude: Option<f64>, longitude: Option<f64>) -> std::result::Result<PrayerTimesResponse, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    
    let (lat, lng, name) = if let (Some(lat), Some(lng)) = (latitude, longitude) {
        (lat, lng, None)
    } else if let Some(loc) = config.current_location() {
        (loc.coordinates.latitude, loc.coordinates.longitude, Some(loc.name.clone()))
    } else {
        (21.4225, 39.8262, Some("Makkah".to_string()))
    };

    let calc = PrayerCalculator::new()
        .with_method(config.calculation_method)
        .with_madhab(config.asr_madhab)
        .with_high_latitude_rule(config.high_latitude_rule)
        .with_adjustments(config.prayer_adjustments);

    let times = calc.calculate_today(lat, lng, name).map_err(|e| e.to_string())?;
    let qibla = QiblaDirection::calculate(lat, lng).map(|q| q.degrees_from_north).unwrap_or(0.0);

    Ok(PrayerTimesResponse {
        fajr: times.get_time(Prayer::Fajr).map(|t| t.time.format("%H:%M").to_string()).unwrap_or_default(),
        sunrise: times.get_time(Prayer::Sunrise).map(|t| t.time.format("%H:%M").to_string()).unwrap_or_default(),
        dhuhr: times.get_time(Prayer::Dhuhr).map(|t| t.time.format("%H:%M").to_string()).unwrap_or_default(),
        asr: times.get_time(Prayer::Asr).map(|t| t.time.format("%H:%M").to_string()).unwrap_or_default(),
        maghrib: times.get_time(Prayer::Maghrib).map(|t| t.time.format("%H:%M").to_string()).unwrap_or_default(),
        isha: times.get_time(Prayer::Isha).map(|t| t.time.format("%H:%M").to_string()).unwrap_or_default(),
        current_prayer: times.current_prayer.map(|p| p.name().to_string()),
        next_prayer: times.next_prayer.map(|p| p.name().to_string()),
        minutes_to_next: times.minutes_to_next,
        date: times.date.to_string(),
        location_name: times.location_name,
        qibla_direction: qibla,
    })
}

#[tauri::command]
pub async fn get_monthly_prayer_times(state: State<'_, AppState>, year: i32, month: u32) -> std::result::Result<Vec<PrayerTimesResponse>, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    
    let (lat, lng, name) = if let Some(loc) = config.current_location() {
        (loc.coordinates.latitude, loc.coordinates.longitude, Some(loc.name.clone()))
    } else {
        (21.4225, 39.8262, Some("Makkah".to_string()))
    };

    let calc = PrayerCalculator::new()
        .with_method(config.calculation_method)
        .with_madhab(config.asr_madhab)
        .with_high_latitude_rule(config.high_latitude_rule)
        .with_adjustments(config.prayer_adjustments);

    let mut responses = Vec::new();
    let qibla = QiblaDirection::calculate(lat, lng).map(|q| q.degrees_from_north).unwrap_or(0.0);
    
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
                    fajr: times.get_time(Prayer::Fajr).map(|t| t.time.format("%H:%M").to_string()).unwrap_or_default(),
                    sunrise: times.get_time(Prayer::Sunrise).map(|t| t.time.format("%H:%M").to_string()).unwrap_or_default(),
                    dhuhr: times.get_time(Prayer::Dhuhr).map(|t| t.time.format("%H:%M").to_string()).unwrap_or_default(),
                    asr: times.get_time(Prayer::Asr).map(|t| t.time.format("%H:%M").to_string()).unwrap_or_default(),
                    maghrib: times.get_time(Prayer::Maghrib).map(|t| t.time.format("%H:%M").to_string()).unwrap_or_default(),
                    isha: times.get_time(Prayer::Isha).map(|t| t.time.format("%H:%M").to_string()).unwrap_or_default(),
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

fn resolve_default_adhan_path(app: &tauri::AppHandle, is_fajr: bool) -> Option<PathBuf> {
    let adhan_file = if is_fajr { "adhan_fajr.mp3" } else { "adhan.mp3" };

    if let Ok(resource_dir) = app.path().resolve("assets", tauri::path::BaseDirectory::Resource) {
        let bundled_path = resource_dir.join(adhan_file);
        if bundled_path.exists() {
            return Some(bundled_path);
        }
    }

    // Dev fallback when running from source.
    let dev_path = PathBuf::from("src-tauri").join("assets").join(adhan_file);
    if dev_path.exists() {
        return Some(dev_path);
    }

    None
}

#[tauri::command]
pub async fn play_adhan(
    app: tauri::AppHandle,
    state: State<'_, AudioState>,
    custom_path: Option<String>,
    volume: Option<f32>,
    is_fajr: Option<bool>,
) -> std::result::Result<(), String> {
    let volume = volume.unwrap_or(1.0).clamp(0.0, 1.0);

    let path = if let Some(path_str) = custom_path.filter(|p| !p.trim().is_empty()) {
        PathBuf::from(path_str)
    } else {
        resolve_default_adhan_path(&app, is_fajr.unwrap_or(false))
            .ok_or_else(|| "No bundled adhan audio file found.".to_string())?
    };

    if !path.exists() {
        return Err(format!("Audio file not found: {}", path.display()));
    }

    state.inner().0.play_file(path, volume)?;
    Ok(())
}

#[tauri::command]
pub async fn stop_audio(state: State<'_, AudioState>) -> std::result::Result<(), String> {
    state.inner().0.stop();
    Ok(())
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
pub fn get_qibla_direction(latitude: f64, longitude: f64) -> std::result::Result<QiblaResponse, String> {
    let qibla = QiblaDirection::calculate(latitude, longitude).map_err(|e| e.to_string())?;
    Ok(QiblaResponse { degrees: qibla.degrees_from_north, cardinal: qibla.cardinal_direction, distance_km: qibla.distance_km })
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
pub fn search_cities(state: State<'_, AppState>, query: String) -> std::result::Result<Vec<CityResult>, String> {
    let db = state.city_db.lock().map_err(|e| e.to_string())?;
    Ok(db.search(&query).into_iter().map(|c| CityResult {
        name: c.name.clone(),
        country: c.country.clone(),
        latitude: c.coordinates.latitude,
        longitude: c.coordinates.longitude,
        timezone: c.timezone.clone(),
    }).collect())
}

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> std::result::Result<AppConfig, String> {
    state.config.lock().map_err(|e| e.to_string()).map(|c| c.clone())
}

#[tauri::command]
pub fn save_config(state: State<'_, AppState>, config: AppConfig) -> std::result::Result<(), String> {
    let mut current = state.config.lock().map_err(|e| e.to_string())?;
    *current = config;
    current.save().map_err(|e| e.to_string())
}

#[derive(Debug, Serialize)]
pub struct HijriResponse {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub month_name: String,
    pub formatted: String,
}

#[tauri::command]
pub fn get_hijri_date(offset_days: Option<i32>) -> std::result::Result<HijriResponse, String> {
    use chrono::{Local, Duration};
    let hijri = if let Some(offset) = offset_days {
        let today = Local::now().date_naive();
        let adjusted = today + Duration::days(offset as i64);
        HijriDate::from_gregorian(adjusted)
    } else {
        HijriDate::today()
    };
    Ok(HijriResponse { year: hijri.year, month: hijri.month, day: hijri.day, month_name: hijri.month_name_english().to_string(), formatted: hijri.format() })
}

#[tauri::command]
pub fn send_notification(app: tauri::AppHandle, title: String, body: String) -> std::result::Result<(), String> {
    app.notification().builder().title(&title).body(&body).show().map_err(|e: tauri_plugin_notification::Error| e.to_string())
}
