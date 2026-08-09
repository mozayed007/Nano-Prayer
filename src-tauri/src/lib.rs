//! NanoPrayReminder Tauri Application

use std::sync::Mutex;
use tauri::menu::{CheckMenuItemBuilder, Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;
use tokio::sync::Notify;

mod audio;
mod commands;
mod scheduler;
mod webview2_check;

use audio::AudioState;
use commands::AppState;
use nano_pray_core::statistics::PrayerLog;
use scheduler::Scheduler;

pub fn run() {
    webview2_check::check_webview2_runtime();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            config: Mutex::new(nano_pray_core::config::AppConfig::default()),
            city_db: Mutex::new(nano_pray_core::location::CityDatabase::new()),
            prayer_log: Mutex::new(PrayerLog::default()),
            active_alert: Mutex::new(None),
            muted: Mutex::new(false),
            scheduler_wakeup: std::sync::Arc::new(Notify::new()),
        })
        .manage(AudioState(std::sync::Arc::new(audio::AudioPlayer::new())))
        .setup(|app| {
            tracing_subscriber::fmt::init();

            let app_handle = app.handle();
            let state = app_handle.state::<AppState>();
            let loaded_config = match nano_pray_core::config::AppConfig::load() {
                Ok(loaded_config) => {
                    if let Ok(mut config_guard) = state.config.lock() {
                        *config_guard = loaded_config.clone();
                        tracing::info!("Configuration loaded successfully");
                    }
                    if let Ok(mut muted_guard) = state.muted.lock() {
                        *muted_guard = loaded_config.advanced.muted;
                    }
                    Some(loaded_config)
                }
                Err(_) => {
                    tracing::warn!("Failed to load configuration, using defaults");
                    None
                }
            };
            let initial_muted = loaded_config
                .as_ref()
                .map(|config| config.advanced.muted)
                .unwrap_or(false);

            // Initialize Tray
            let mute_i = CheckMenuItemBuilder::with_id("mute", "Mute Reminders")
                .checked(initial_muted)
                .build(app)?;
            let mute_menu_item = mute_i.clone();
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&show_i, &mute_i, &quit_i])?;

            if let Some(icon) = app.default_window_icon() {
                let _tray = TrayIconBuilder::with_id("tray")
                    .icon(icon.clone())
                    .menu(&menu)
                    .on_menu_event(move |app, event| match event.id.as_ref() {
                        "quit" => {
                            app.exit(0);
                        }
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "mute" => {
                            let state = app.state::<AppState>();
                            if let Ok(mut muted) = state.muted.lock() {
                                *muted = !*muted;
                                let new_muted = *muted;
                                let _ = mute_menu_item.set_checked(new_muted);
                                tracing::info!("Reminders muted: {}", new_muted);
                                if let Ok(mut config) = state.config.lock() {
                                    config.advanced.muted = new_muted;
                                    if let Err(e) = config.save() {
                                        tracing::warn!(
                                            "Failed to save config after mute toggle: {}",
                                            e
                                        );
                                    }
                                }
                            } else {
                                tracing::warn!("Failed to update muted state after tray toggle");
                            };
                        }
                        _ => {}
                    })
                    .build(app)?;
            } else {
                tracing::warn!("Default window icon not found; tray icon disabled.");
            }

            if let Some(loaded_config) = loaded_config {
                // Sync the OS autostart entry with the stored config preference.
                // This is critical for portable builds: each launch re-registers the current
                // exe path, so moving the exe never leaves a stale/broken registry entry.
                use tauri_plugin_autostart::ManagerExt;
                let autostart = app_handle.autolaunch();
                if loaded_config.advanced.auto_start {
                    if let Err(e) = autostart.enable() {
                        tracing::warn!("Failed to enable autostart: {}", e);
                    } else {
                        tracing::info!("Autostart enabled (synced from config).");
                    }
                } else {
                    // Only disable if currently enabled to avoid errors on fresh installs
                    if autostart.is_enabled().unwrap_or(false) {
                        if let Err(e) = autostart.disable() {
                            tracing::warn!("Failed to disable autostart: {}", e);
                        } else {
                            tracing::info!("Autostart disabled (synced from config).");
                        }
                    }
                }

                // Honor start_minimized and autostart --hidden flag.
                let launch_hidden = loaded_config.advanced.start_minimized
                    || std::env::args().any(|arg| arg == "--hidden");
                if launch_hidden {
                    if let Some(window) = app.get_webview_window("main") {
                        if let Err(err) = window.hide() {
                            tracing::warn!("Failed to start minimized: {}", err);
                        } else {
                            tracing::info!("Started minimized to tray/background.");
                        }
                    }
                }
            }

            if let Ok(loaded_log) = PrayerLog::load() {
                if let Ok(mut log_guard) = state.prayer_log.lock() {
                    *log_guard = loaded_log;
                    tracing::info!("Prayer log loaded successfully");
                }
            } else {
                tracing::warn!("Failed to load prayer log, using defaults");
            }

            // Start Scheduler
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let scheduler = Scheduler::new(handle);
                scheduler.run().await;
            });

            if let Some(_window) = app.get_webview_window("main") {
                tracing::info!("Application started");
            } else {
                tracing::warn!("Main window not found!");
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() != "main" {
                    return;
                }

                let app_handle = window.app_handle();
                let state = app_handle.state::<AppState>();

                let minimize_to_tray = state
                    .config
                    .lock()
                    .map(|cfg| cfg.advanced.minimize_to_tray)
                    .unwrap_or(false);

                let has_tray = app_handle.tray_by_id("tray").is_some();

                if minimize_to_tray && has_tray {
                    api.prevent_close();
                    if let Err(err) = window.hide() {
                        tracing::error!("Failed to hide window to tray: {}", err);
                    } else {
                        tracing::info!("Window close intercepted; minimized to tray.");
                    }
                } else {
                    app_handle.exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_prayer_times,
            commands::get_monthly_prayer_times,
            commands::get_qibla_direction,
            commands::search_cities,
            commands::get_config,
            commands::save_config,
            commands::get_hijri_date,
            commands::send_notification,
            commands::play_adhan,
            commands::play_reminder_sound,
            commands::stop_audio,
            commands::dismiss_alert,
            commands::get_active_alert,
            commands::mark_prayer_completed,
            commands::pause_audio,
            commands::resume_audio,
            commands::get_statistics,
            commands::get_next_prayer,
            commands::set_reminders_muted,
            commands::update_reminder_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
