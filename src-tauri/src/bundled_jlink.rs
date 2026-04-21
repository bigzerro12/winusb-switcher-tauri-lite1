//! Bundled SEGGER J-Link runtime resolution helpers.
//!
//! Product decision: **no install/download/extract**.
//! We only load J-Link runtime files shipped under `src-tauri/resources/jlink-runtime/...`.

use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use crate::error::{AppError, AppResult};

/// Set by `prepare_bundled_jlink` after the J-Link library is loaded (UTF-8 path).
pub const WINUSB_JLINK_DLL_PATH_ENV: &str = "WINUSB_JLINK_DLL_PATH";

// Supported bundled layouts:
//  A) Versioned:
//     resources/jlink-runtime/jlink-v936/{windows-64,windows-32,linux-64,linux-32}/...
//  B) Unversioned:
//     resources/jlink-runtime/{windows-64,windows-32,linux-64,linux-32}/...
const JLINK_BUNDLED_VERSION_DIR: &str = "jlink-v936";

fn resource_dir(app: &AppHandle) -> AppResult<PathBuf> {
    app.path()
        .resource_dir()
        .map_err(|e| AppError::Internal(format!("resource_dir: {}", e)))
}

#[cfg(target_os = "windows")]
pub fn resolve_bundled_jlinkarm_dll(app: &AppHandle) -> AppResult<PathBuf> {
    #[cfg(debug_assertions)]
    {
        let dev_platform = if cfg!(target_pointer_width = "64") {
            "windows-64"
        } else {
            "windows-32"
        };
        let dll_name = if cfg!(target_pointer_width = "64") {
            "JLink_x64.dll"
        } else {
            "JLinkARM.dll"
        };
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("jlink-runtime");
        let candidates = [
            base.join(JLINK_BUNDLED_VERSION_DIR).join(dev_platform).join(dll_name),
            base.join(dev_platform).join(dll_name),
        ];
        for c in candidates {
            if c.is_file() {
                return Ok(c);
            }
        }
    }

    let res_dir = resource_dir(app)?;
    let platform = if cfg!(target_pointer_width = "64") {
        "windows-64"
    } else {
        "windows-32"
    };
    let dll_name = if cfg!(target_pointer_width = "64") {
        "JLink_x64.dll"
    } else {
        "JLinkARM.dll"
    };

    let candidates = [
        // versioned (resource_dir may already be .../resources)
        res_dir
            .join("resources")
            .join("jlink-runtime")
            .join(JLINK_BUNDLED_VERSION_DIR)
            .join(platform)
            .join(dll_name),
        res_dir
            .join("jlink-runtime")
            .join(JLINK_BUNDLED_VERSION_DIR)
            .join(platform)
            .join(dll_name),
        // unversioned
        res_dir
            .join("resources")
            .join("jlink-runtime")
            .join(platform)
            .join(dll_name),
        res_dir.join("jlink-runtime").join(platform).join(dll_name),
    ];

    for c in &candidates {
        if c.is_file() {
            return Ok(c.clone());
        }
    }

    Err(AppError::Runtime(format!(
        "Bundled J-Link DLL not found.\nLooked for:\n  {}\n  {}\n\nExpected layout:\n  src-tauri/resources/jlink-runtime/{}/{}/{}\n",
        candidates[0].display(),
        candidates[1].display(),
        JLINK_BUNDLED_VERSION_DIR,
        platform,
        dll_name
    )))
}

#[cfg(target_os = "linux")]
pub fn resolve_bundled_linux_runtime_dir(app: &AppHandle) -> AppResult<PathBuf> {
    #[cfg(debug_assertions)]
    {
        let platform = if cfg!(target_pointer_width = "64") {
            "linux-64"
        } else {
            "linux-32"
        };
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("jlink-runtime");
        let candidates = [
            base.join(JLINK_BUNDLED_VERSION_DIR).join(platform),
            base.join(platform),
        ];
        for c in candidates {
            if c.is_dir() {
                return Ok(c);
            }
        }
    }

    let res_dir = resource_dir(app)?;
    let platform = if cfg!(target_pointer_width = "64") {
        "linux-64"
    } else {
        "linux-32"
    };

    let candidates = [
        // versioned
        res_dir
            .join("resources")
            .join("jlink-runtime")
            .join(JLINK_BUNDLED_VERSION_DIR)
            .join(platform),
        res_dir
            .join("jlink-runtime")
            .join(JLINK_BUNDLED_VERSION_DIR)
            .join(platform),
        // unversioned
        res_dir
            .join("resources")
            .join("jlink-runtime")
            .join(platform),
        res_dir.join("jlink-runtime").join(platform),
    ];

    for c in &candidates {
        if c.is_dir() {
            return Ok(c.clone());
        }
    }

    Err(AppError::Runtime(format!(
        "Bundled J-Link runtime dir not found.\nLooked for:\n  {}\n  {}\n\nExpected layout:\n  src-tauri/resources/jlink-runtime/{}/{}/\n",
        candidates[0].display(),
        candidates[1].display(),
        JLINK_BUNDLED_VERSION_DIR,
        platform
    )))
}

