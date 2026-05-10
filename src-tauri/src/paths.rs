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
/// Windows: directory containing the `.exe`.
/// Linux portable zip layout: `<root>/bin/<binary>` with resources under `<root>/lib/<cargo_pkg_name>/resources`.
/// Linux/macOS dev (`target/.../release/`): directory containing the binary.
pub fn install_root() -> PathBuf {
    let Some(exe_dir) = exe_dir() else {
        return PathBuf::from(".");
    };

    #[cfg(target_os = "linux")]
    {
        if exe_dir.file_name().and_then(|n| n.to_str()) == Some("bin") {
            if let Some(root) = exe_dir.parent() {
                let lib_res = root
                    .join("lib")
                    .join(env!("CARGO_PKG_NAME"))
                    .join("resources");
                if lib_res.is_dir() {
                    return root.to_path_buf();
                }
            }
        }
    }

    exe_dir
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
