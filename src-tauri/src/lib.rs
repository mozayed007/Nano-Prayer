//! NanoPrayReminder Tauri Application

use std::sync::Mutex;
use tauri::Manager;
use tauri::menu::{Menu, MenuItem, CheckMenuItemBuilder};
use tauri::tray::TrayIconBuilder;

mod commands;
mod audio;
mod scheduler;

use commands::AppState;
use audio::AudioState;
use scheduler::Scheduler;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState {
            config: Mutex::new(nano_pray_core::config::AppConfig::default()),
            city_db: Mutex::new(nano_pray_core::location::CityDatabase::new()),
        })
        .manage(AudioState(std::sync::Arc::new(audio::AudioPlayer::new())))
        .setup(|app| {
            tracing_subscriber::fmt::init();
            
            // Initialize Tray
            let mute_i = CheckMenuItemBuilder::with_id("mute", "Mute Reminders")
                .checked(false)
                .build(app)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            
            let menu = Menu::with_items(app, &[&show_i, &mute_i, &quit_i])?;
            
            if let Some(icon) = app.default_window_icon() {
                let _tray = TrayIconBuilder::with_id("tray")
                    .icon(icon.clone())
                    .menu(&menu)
                    .on_menu_event(move |app, event| {
                        match event.id.as_ref() {
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
                                // Will be handled globally or we can toggle state in AppState
                                tracing::info!("Mute toggled in tray");
                            }
                            _ => {}
                        }
                    })
                    .build(app)?;
            } else {
                tracing::warn!("Default window icon not found; tray icon disabled.");
            }

            // Load configuration
            let app_handle = app.handle();
            let state = app_handle.state::<AppState>();
            if let Ok(loaded_config) = nano_pray_core::config::AppConfig::load() {
                if let Ok(mut config_guard) = state.config.lock() {
                    *config_guard = loaded_config;
                    tracing::info!("Configuration loaded successfully");
                }
            } else {
                tracing::warn!("Failed to load configuration, using defaults");
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

                if minimize_to_tray {
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
            commands::stop_audio,
            commands::pause_audio,
            commands::resume_audio,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
