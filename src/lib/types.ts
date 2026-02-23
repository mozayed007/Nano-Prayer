export interface Coordinates {
  latitude: number;
  longitude: number;
  elevation?: number | null;
}

export interface SavedLocation {
  id: string;
  name: string;
  coordinates: Coordinates;
  timezone: string;
  is_favorite: boolean;
  is_default: boolean;
  calculation_method: string | null;
  notes: string | null;
}

export interface ReminderConfig {
  enabled: boolean;
  minutes_before: number;
  play_sound_before: boolean;
  play_adhan: boolean;
  minutes_after: number;
  play_sound_after: boolean;
  custom_sound: string | null;
  volume: number;
  show_notification: boolean;
}

export interface AudioSettings {
  default_adhan: string | null;
  global_volume: number;
  fade_in_seconds: number;
  adhan_for_all: boolean;
}

export interface NotificationSettings {
  enabled: boolean;
  show_time: boolean;
  show_countdown: boolean;
  play_sound: boolean;
}

export interface AppearanceSettings {
  theme: "light" | "dark" | "system";
  accent_color: string;
  clock_format: "hour12" | "hour24";
  show_arabic: boolean;
  animations_enabled: boolean;
  font_scale: number;
}

export interface AdvancedSettings {
  auto_start: boolean;
  start_minimized: boolean;
  minimize_to_tray: boolean;
  auto_update_check: boolean;
  time_sync_detection: boolean;
  quiet_hours_enabled: boolean;
  quiet_hours_start: number;
  quiet_hours_end: number;
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
  audio: AudioSettings;
  notifications: NotificationSettings;
  appearance: AppearanceSettings;
  advanced: AdvancedSettings;
  last_backup_path: string | null;
}

