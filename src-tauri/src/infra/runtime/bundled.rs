//! Bundled SEGGER runtime selection + native bridge load.
//!
//! This module centralizes:
//! - locating the correct per-OS/per-arch bundled runtime
//! - setting process environment (PATH/LD_LIBRARY_PATH on Linux, PATH on Windows)
//! - loading the native bridge against the selected library (DLL/.so)

use std::path::PathBuf;

#[cfg(target_os = "windows")]
use std::path::Path;

use tauri::AppHandle;
#[cfg(target_os = "linux")]
use tauri::Manager;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct JLinkRuntime {
    /// Directory containing the runtime (Firmwares/ and the loaded library).
    pub runtime_dir: PathBuf,
    /// Full path to the library mapped by the native bridge (DLL/.so).
    pub native_lib_path: PathBuf,
    /// Best-effort version string from the library.
    pub version: Option<String>,
}

#[cfg(target_os = "windows")]
fn parent_or_self(p: &Path) -> PathBuf {
    p.parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| p.to_path_buf())
}

#[cfg(target_os = "windows")]
pub fn prepare(app: &AppHandle, override_dll: Option<PathBuf>) -> AppResult<JLinkRuntime> {
    let dll_path = if let Some(p) = override_dll {
        log::info!(
            "[bootstrap] WINUSB_JLINK_DLL_OVERRIDE: using {}",
            p.display()
        );
        if p.is_file() {
            p
        } else {
            log::warn!(
                "[bootstrap] WINUSB_JLINK_DLL_OVERRIDE path is not a file: {}",
                p.display()
            );
            return Err(AppError::Internal(format!(
                "WINUSB_JLINK_DLL_OVERRIDE points to a missing file: {}",
                p.display()
            )));
        }
    } else {
        crate::bundled_jlink::resolve_bundled_jlinkarm_dll(app)?
    };

    crate::jlink_ffi::bridge::load(&dll_path).map_err(|e| {
        log::warn!(
            "[bootstrap] bridge_load failed for {}: {}",
            dll_path.display(),
            e
        );
        AppError::Internal(format!("bridge_load: {}", e))
    })?;

    std::env::set_var(
        crate::bundled_jlink::WINUSB_JLINK_DLL_PATH_ENV,
        dll_path.to_string_lossy().as_ref(),
    );

    let runtime_dir = parent_or_self(&dll_path);
    crate::platform::ensure_jlink_runtime_env(&runtime_dir.to_string_lossy());

    let version = crate::jlink_ffi::bridge::dll_version_string();
    log::debug!(
        "[bootstrap] Windows SEGGER bridge ready: dir={} lib={} version={:?}",
        runtime_dir.display(),
        dll_path.display(),
        version
    );
    Ok(JLinkRuntime {
        runtime_dir,
        native_lib_path: dll_path,
        version,
    })
}

#[cfg(target_os = "linux")]
pub fn prepare(app: &AppHandle) -> AppResult<JLinkRuntime> {
    let runtime_dir = crate::bundled_jlink::resolve_bundled_linux_runtime_dir(app)?;
    crate::platform::ensure_jlink_runtime_env(&runtime_dir.to_string_lossy());

    ensure_linux_udev_rules(app)?;

    let candidates = [
        runtime_dir.join("libjlinkarm.so"),
        runtime_dir.join("libjlinkarm.so.9"),
    ];
    let lib_path = candidates
        .iter()
        .find(|p| p.is_file())
        .cloned()
        .ok_or_else(|| {
            log::warn!(
                "[bootstrap] no libjlinkarm.so in {} (candidates checked)",
                runtime_dir.display()
            );
            AppError::Internal(format!(
                "Bundled libjlinkarm not found in {} (expected libjlinkarm.so or libjlinkarm.so.9)",
                runtime_dir.display()
            ))
        })?;

    crate::jlink_ffi::bridge::load(&lib_path).map_err(|e| {
        log::warn!(
            "[bootstrap] bridge_load failed for {}: {}",
            lib_path.display(),
            e
        );
        AppError::Internal(format!("bridge_load: {}", e))
    })?;

    std::env::set_var(
        crate::bundled_jlink::WINUSB_JLINK_DLL_PATH_ENV,
        lib_path.to_string_lossy().as_ref(),
    );

    let version = crate::jlink_ffi::bridge::dll_version_string();
    log::debug!(
        "[bootstrap] Linux SEGGER bridge ready: dir={} lib={} version={:?}",
        runtime_dir.display(),
        lib_path.display(),
        version
    );
    Ok(JLinkRuntime {
        runtime_dir,
        native_lib_path: lib_path,
        version,
    })
}

#[cfg(target_os = "linux")]
pub fn ensure_linux_udev_rules_on_startup(app: &AppHandle) -> AppResult<()> {
    ensure_linux_udev_rules(app)
}

#[cfg(target_os = "linux")]
fn ensure_linux_udev_rules(app: &AppHandle) -> AppResult<()> {
    use std::fs;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    const RULES_DST: &str = "/etc/udev/rules.d/99-jlink.rules";
    const RULES_BUNDLED: &str = "resources/segger-99-jlink.rules";

    let res_dir = app
        .path()
        .resource_dir()
        .map_err(|e| AppError::Internal(format!("resource_dir: {}", e)))?;
    let bundled_rules = res_dir.join(RULES_BUNDLED);
    if !bundled_rules.is_file() {
        return Err(AppError::Runtime(format!(
            "Bundled udev rules file is missing: {}",
            bundled_rules.display()
        )));
    }
    let bytes = fs::read(&bundled_rules)?;

    // If the system already has the expected rules, startup should continue without prompting.
    if let Ok(existing) = fs::read(RULES_DST) {
        if existing == bytes {
            log::debug!(
                "[bootstrap] udev rules already installed and up-to-date ({})",
                RULES_DST
            );
            return Ok(());
        }
        log::info!(
            "[bootstrap] existing udev rules differ from bundled copy; attempting update ({})",
            RULES_DST,
        );
    }

    // Use securely created temp files so local users cannot race path creation in /tmp.
    let mut tmp_file = tempfile::Builder::new()
        .prefix("winusb-switcher-lite-99-jlink-")
        .suffix(".rules")
        .tempfile()
        .map_err(|e| AppError::Internal(format!("create temp rules file: {}", e)))?;
    tmp_file
        .as_file_mut()
        .write_all(&bytes)
        .map_err(|e| AppError::Internal(format!("write temp rules file: {}", e)))?;
    tmp_file
        .as_file()
        .sync_all()
        .map_err(|e| AppError::Internal(format!("sync temp rules file: {}", e)))?;

    // If we can install directly (e.g. running as root), do so; otherwise attempt pkexec.
    let install_direct = || -> std::io::Result<()> {
        fs::create_dir_all("/etc/udev/rules.d")?;
        fs::copy(tmp_file.path(), RULES_DST)?;
        Ok(())
    };

    let direct_ok = install_direct().is_ok();
    if direct_ok {
        log::info!("[bootstrap] installed udev rules to {}", RULES_DST);
        let _ = Command::new("udevadm")
            .args(["control", "--reload-rules"])
            .status();
        let _ = Command::new("udevadm").arg("trigger").status();
        return Ok(());
    }

    // pkexec prompt (interactive). Run install + reload + trigger in a single
    // privileged invocation so desktop environments only prompt once.
    log::info!(
        "[bootstrap] udev rules not present ({}). Attempting pkexec install...",
        RULES_DST
    );
    let mut helper_file = tempfile::Builder::new()
        .prefix("winusb-switcher-lite-udev-install-")
        .suffix(".sh")
        .tempfile()
        .map_err(|e| AppError::Internal(format!("create temp helper script: {}", e)))?;
    helper_file
        .as_file_mut()
        .write_all(
            br#"#!/bin/sh
set -eu
install -m 0644 "$1" /etc/udev/rules.d/99-jlink.rules
udevadm control --reload-rules
udevadm trigger
"#,
        )
        .map_err(|e| AppError::Internal(format!("write temp helper script: {}", e)))?;
    helper_file
        .as_file()
        .sync_all()
        .map_err(|e| AppError::Internal(format!("sync temp helper script: {}", e)))?;

    // On Linux, executing a file that is still open for writing can fail with ETXTBSY ("Text file busy").
    // Close the file handle before calling pkexec.
    let helper_path = helper_file.into_temp_path();
    fs::set_permissions(&helper_path, fs::Permissions::from_mode(0o700))
        .map_err(|e| AppError::Internal(format!("chmod helper script: {}", e)))?;

    let status = Command::new("pkexec")
        .arg(&helper_path)
        .arg(tmp_file.path())
        .status();

    match status {
        Ok(s) if s.success() => {
            log::info!("[bootstrap] pkexec installed udev rules to {}", RULES_DST);
        }
        Ok(s) => {
            return Err(AppError::Runtime(format!(
                "J-Link udev setup was not completed (pkexec exit code: {:?})",
                s.code()
            )));
        }
        Err(e) => {
            return Err(AppError::Runtime(format!(
                "Failed to run pkexec for J-Link udev setup: {}",
                e
            )));
        }
    }
    Ok(())
}
