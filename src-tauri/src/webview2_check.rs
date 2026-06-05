//! WebView2 runtime check for Windows

#[cfg(target_os = "windows")]
use windows::{
    core::{HSTRING, PCWSTR},
    Win32::{
        Foundation::HWND,
        System::LibraryLoader::{GetModuleHandleW, GetProcAddress},
        UI::WindowsAndMessaging::{IDYES, MB_ICONERROR, MB_YESNO, SW_SHOWNORMAL},
    },
};

#[cfg(target_os = "windows")]
type MessageBoxWFn = unsafe extern "system" fn(HWND, PCWSTR, PCWSTR, u32) -> i32;

#[cfg(target_os = "windows")]
type ShellExecuteWFn =
    unsafe extern "system" fn(HWND, PCWSTR, PCWSTR, PCWSTR, PCWSTR, i32) -> isize;

/// WebView2 download URL
const WEBVIEW2_DOWNLOAD_URL: &str =
    "https://developer.microsoft.com/en-us/microsoft-edge/webview2/";

/// Check if WebView2 runtime is installed on Windows
#[cfg(target_os = "windows")]
pub fn is_webview2_installed() -> bool {
    use std::path::PathBuf;

    let program_files = std::env::var("ProgramFiles(x86)").unwrap_or_else(|_| {
        std::env::var("ProgramFiles").unwrap_or_else(|_| "C:\\Program Files".to_string())
    });

    let possible_paths = [
        PathBuf::from(&program_files).join("Microsoft\\EdgeWebView\\Application"),
        PathBuf::from(&program_files).join("Microsoft\\Edge\\Application"),
    ];

    for path in possible_paths {
        if path.exists() {
            if let Ok(entries) = std::fs::read_dir(&path) {
                for entry in entries.flatten() {
                    let version_path = entry.path().join("msedge.dll");
                    if version_path.exists() {
                        return true;
                    }
                }
            }
        }
    }

    false
}

#[cfg(not(target_os = "windows"))]
pub fn is_webview2_installed() -> bool {
    true
}

/// Show a native Windows message box asking user to install WebView2
#[cfg(target_os = "windows")]
pub fn show_webview2_missing_dialog() -> bool {
    unsafe {
        let h_user32 = GetModuleHandleW(windows::core::w!("user32.dll"))
            .expect("user32.dll should always be loaded");

        let mb_ptr = GetProcAddress(h_user32, windows::core::s!("MessageBoxW"));
        let Some(mb_ptr) = mb_ptr else {
            tracing::error!("Failed to load MessageBoxW from user32.dll");
            return false;
        };
        let message_box: MessageBoxWFn = std::mem::transmute(mb_ptr);

        let title: HSTRING = "WebView2 Required".into();
        let message: HSTRING = "NanoPrayer requires Microsoft Edge WebView2 runtime to run.\n\nWould you like to download it now?\n\nYou can also install it from Microsoft Store.".into();

        let result = message_box(
            HWND::default(),
            PCWSTR(message.as_ptr()),
            PCWSTR(title.as_ptr()),
            (MB_ICONERROR | MB_YESNO).0,
        );

        result == IDYES.0
    }
}

#[cfg(not(target_os = "windows"))]
pub fn show_webview2_missing_dialog() -> bool {
    false
}

/// Open the WebView2 download page in the default browser
#[cfg(target_os = "windows")]
pub fn open_webview2_download_page() {
    unsafe {
        let h_shell32 = GetModuleHandleW(windows::core::w!("shell32.dll"))
            .expect("shell32.dll should always be loaded");

        let se_ptr = GetProcAddress(h_shell32, windows::core::s!("ShellExecuteW"));
        let Some(se_ptr) = se_ptr else {
            tracing::error!("Failed to load ShellExecuteW from shell32.dll");
            return;
        };
        let shell_execute: ShellExecuteWFn = std::mem::transmute(se_ptr);

        let url: HSTRING = WEBVIEW2_DOWNLOAD_URL.into();
        let operation: HSTRING = "open".into();

        shell_execute(
            HWND::default(),
            PCWSTR(operation.as_ptr()),
            PCWSTR(url.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL.0,
        );
    }
}

#[cfg(not(target_os = "windows"))]
pub fn open_webview2_download_page() {}

/// Run WebView2 check before starting the application.
/// Returns true if WebView2 is installed or user declined to install.
/// Returns false if application should continue (user clicked Yes to download).
pub fn check_webview2_runtime() {
    #[cfg(target_os = "windows")]
    {
        if !is_webview2_installed() {
            if show_webview2_missing_dialog() {
                open_webview2_download_page();
            }
            std::process::exit(1);
        }
    }
}
