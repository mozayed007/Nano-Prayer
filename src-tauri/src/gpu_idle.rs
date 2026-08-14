//! Release WebView2 compositor / GPU while the main window is hidden.
//!
//! `window.hide()` keeps msedgewebview2.exe alive. On low-end iGPUs that
//! process still contends with games until the user fully quits. Controller
//! SetIsVisible(false) stops the WebView2 compositor.

use serde::Serialize;
use tauri::{Emitter, WebviewWindow};

#[derive(Clone, Serialize)]
struct VisibilityPayload {
    visible: bool,
}

pub fn set_webview_gpu_idle(window: &WebviewWindow, idle: bool) {
    #[cfg(windows)]
    {
        let idle_flag = idle;
        if let Err(e) = window.with_webview(move |webview| {
            apply_webview2_idle(&webview, idle_flag);
        }) {
            tracing::warn!("Failed to reach webview for GPU idle={idle}: {e}");
        }
    }

    let _ = window.emit(
        "np-window-visibility",
        VisibilityPayload { visible: !idle },
    );
}

#[cfg(windows)]
fn apply_webview2_idle(webview: &tauri::webview::PlatformWebview, idle: bool) {
    let controller = webview.controller();
    let result = unsafe { controller.SetIsVisible(!idle) };
    if let Err(e) = result {
        tracing::warn!("WebView2 SetIsVisible({}) failed: {e}", !idle);
    } else {
        tracing::info!("WebView2 SetIsVisible({}) for GPU idle={}", !idle, idle);
    }
}

pub fn hide_main_for_tray(window: &WebviewWindow) -> tauri::Result<()> {
    window.hide()?;
    set_webview_gpu_idle(window, true);
    Ok(())
}

pub fn show_main_from_tray(window: &WebviewWindow) -> tauri::Result<()> {
    set_webview_gpu_idle(window, false);
    window.show()?;
    let _ = window.set_focus();
    Ok(())
}
