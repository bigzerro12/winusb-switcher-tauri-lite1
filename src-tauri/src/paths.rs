//! Install-relative paths for logs and WebView data (avoid `%LOCALAPPDATA%` / XDG footprint).

use std::path::{Path, PathBuf};

/// Matches `identifier` in `tauri.conf.json`.
pub const APP_DIR_NAME: &str = "com.winusbswitcher.lite";

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
}

/// Directory the user considers the “install root” (folder that should contain `com.winusbswitcher.lite/`).
///
/// Currently the parent directory of the main executable (installer layout and local `cargo` output).
pub fn install_root() -> PathBuf {
    exe_dir().unwrap_or_else(|| PathBuf::from("."))
}

pub fn app_data_dir() -> PathBuf {
    install_root().join(APP_DIR_NAME)
}

pub fn log_dir() -> PathBuf {
    app_data_dir().join("logs")
}

pub fn webview_data_dir() -> PathBuf {
    app_data_dir().join("webview")
}

pub fn ensure_app_dirs() -> std::io::Result<()> {
    std::fs::create_dir_all(log_dir())?;
    std::fs::create_dir_all(webview_data_dir())?;
    Ok(())
}

pub fn instance_lock_path() -> PathBuf {
    app_data_dir().join("single-instance.lock")
}
