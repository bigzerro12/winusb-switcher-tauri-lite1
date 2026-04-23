//! Bundled SEGGER J-Link runtime resolution helpers.
//!
//! Product decision: **no install/download/extract**.
//! We load J-Link runtime files from `src-tauri/resources/jlink-runtime/...` in **dev**
//! (full tree), and from **`jlink-runtime-bundled/`** first in **packaged** builds
//! (single-arch subtree staged at build time).

use std::path::{Path, PathBuf};

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

/// Slim install tree (preferred in release) then full repo-style tree.
const JLINK_RUNTIME_TREE_NAMES: [&str; 2] = ["jlink-runtime-bundled", "jlink-runtime"];

#[cfg(target_os = "windows")]
fn windows_dll_candidates(res_dir: &Path, platform: &str, dll_name: &str) -> Vec<PathBuf> {
    let mut v = Vec::with_capacity(8);
    for tree in JLINK_RUNTIME_TREE_NAMES {
        v.push(
            res_dir
                .join("resources")
                .join(tree)
                .join(JLINK_BUNDLED_VERSION_DIR)
                .join(platform)
                .join(dll_name),
        );
        v.push(
            res_dir
                .join(tree)
                .join(JLINK_BUNDLED_VERSION_DIR)
                .join(platform)
                .join(dll_name),
        );
        v.push(
            res_dir
                .join("resources")
                .join(tree)
                .join(platform)
                .join(dll_name),
        );
        v.push(res_dir.join(tree).join(platform).join(dll_name));
    }
    v
}

#[cfg(target_os = "linux")]
fn linux_runtime_dir_candidates(res_dir: &Path, platform: &str) -> Vec<PathBuf> {
    let mut v = Vec::with_capacity(8);
    for tree in JLINK_RUNTIME_TREE_NAMES {
        v.push(
            res_dir
                .join("resources")
                .join(tree)
                .join(JLINK_BUNDLED_VERSION_DIR)
                .join(platform),
        );
        v.push(
            res_dir
                .join(tree)
                .join(JLINK_BUNDLED_VERSION_DIR)
                .join(platform),
        );
        v.push(res_dir.join("resources").join(tree).join(platform));
        v.push(res_dir.join(tree).join(platform));
    }
    v
}

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

    let candidates = windows_dll_candidates(&res_dir, platform, dll_name);

    for c in &candidates {
        if c.is_file() {
            return Ok(c.clone());
        }
    }

    Err(AppError::Runtime(format!(
        "Bundled J-Link DLL not found.\nFirst tried:\n  {}\n  {}\n\nExpected layout (versioned or flat) under jlink-runtime-bundled/ or jlink-runtime/:\n  {}/{}/{}\n",
        candidates.first().map(|p| p.display().to_string()).unwrap_or_default(),
        candidates.get(1).map(|p| p.display().to_string()).unwrap_or_default(),
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

    let candidates = linux_runtime_dir_candidates(&res_dir, platform);

    for c in &candidates {
        if c.is_dir() {
            return Ok(c.clone());
        }
    }

    Err(AppError::Runtime(format!(
        "Bundled J-Link runtime dir not found.\nFirst tried:\n  {}\n  {}\n\nExpected layout under jlink-runtime-bundled/ or jlink-runtime/:\n  {}/{}/\n",
        candidates.first().map(|p| p.display().to_string()).unwrap_or_default(),
        candidates.get(1).map(|p| p.display().to_string()).unwrap_or_default(),
        JLINK_BUNDLED_VERSION_DIR,
        platform
    )))
}

