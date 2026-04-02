export type ReminderAlertType = "before" | "on_time" | "after";

export interface Coordinates {
  latitude: number;
  longitude: number;
}

export interface SavedLocation {
  id: string;
  name: string;
  coordinates: Coordinates;
  timezone: string;
}

export interface ReminderConfig {
  enabled: boolean;
  before_enabled: boolean;
  minutes_before: number;
  play_sound_before: boolean;
  play_adhan: boolean;
  after_enabled: boolean;
  minutes_after: number;
  play_sound_after: boolean;
  custom_sound: string | null;
  custom_reminder_sound: string | null;
  volume: number;
  show_notification: boolean;
}

export interface AppConfig {
  locations: SavedLocation[];
  current_location_index: number;
  calculation_method: string;
  asr_madhab: string;
  high_latitude_rule: string;
  prayer_adjustments: {
    fajr: number;
    sunrise: number;
    dhuhr: number;
    asr: number;
    maghrib: number;
    isha: number;
  };
  hijri_offset: number;
  show_hijri: boolean;
  reminders: Record<string, ReminderConfig>;
  audio: {
    default_adhan: string | null;
    global_volume: number;
    fade_in_seconds: number;
    adhan_for_all: boolean;
  };
  notifications: {
    enabled: boolean;
    show_time: boolean;
    show_countdown: boolean;
    play_sound: boolean;
  };
  appearance: {
    theme: "light" | "dark" | "system";
    accent_color: string;
    clock_format: "hour12" | "hour24";
    show_arabic: boolean;
    animations_enabled: boolean;
    font_scale: number;
  };
  advanced: {
    auto_start: boolean;
    start_minimized: boolean;
    minimize_to_tray: boolean;
    auto_update_check: boolean;
    time_sync_detection: boolean;
    quiet_hours_enabled: boolean;
    quiet_hours_start: number;
    quiet_hours_end: number;
  };
  last_backup_path: string | null;
}

export interface PrayerAlertPayload {
  prayer: string;
  alert_type: ReminderAlertType;
  title: string;
  body: string;
}

export interface PrayerTimesResponse {
  date: string;
  location_name: string | null;
  fajr: string;
  sunrise: string;
  dhuhr: string;
  asr: string;
  maghrib: string;
  isha: string;
  current_prayer: string | null;
  next_prayer: string | null;
  minutes_to_next: number | null;
  qibla_direction: number;
}
