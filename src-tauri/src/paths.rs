//! Writable runtime paths for logs, lock files, and WebView data.

#[cfg(not(target_os = "linux"))]
use std::path::Path;
use std::path::PathBuf;

/// Matches `identifier` in `tauri.conf.json`.
pub const APP_DIR_NAME: &str = "com.winusbswitcher.lite";

#[cfg(not(target_os = "linux"))]
fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
}

#[cfg(target_os = "linux")]
fn env_path(var: &str) -> Option<PathBuf> {
    std::env::var_os(var).map(PathBuf::from)
}

#[cfg(target_os = "linux")]
fn xdg_data_root() -> PathBuf {
    env_path("XDG_DATA_HOME").unwrap_or_else(|| {
        env_path("HOME")
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".local")
            .join("share")
    })
}

/// Windows keeps writable data beside the installed executable.
#[cfg(not(target_os = "linux"))]
pub fn install_root() -> PathBuf {
    exe_dir().unwrap_or_else(|| PathBuf::from("."))
}

pub fn app_data_dir() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        xdg_data_root().join(APP_DIR_NAME)
    }
    #[cfg(not(target_os = "linux"))]
    {
        install_root().join(APP_DIR_NAME)
    }
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
