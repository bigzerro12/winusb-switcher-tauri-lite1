//! WinUSB Switcher Lite–only commands.

use tauri::{AppHandle, State};

use crate::state::AppState;

/// Env var: optional full path to `JLink_x64.dll` / `JLinkARM.dll` for debugging (Windows).
pub const WINUSB_JLINK_DLL_OVERRIDE_ENV: &str = "WINUSB_JLINK_DLL_OVERRIDE";

/// Prepare J-Link runtime.
///
/// - Windows: load bundled SEGGER DLL (`JLink_x64.dll` for 64-bit builds).
/// - Linux: load bundled `libjlinkarm.so` directly via the native bridge (no extraction/installer).
#[tauri::command]
pub async fn prepare_bundled_jlink(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let app_clone = app.clone();

    #[cfg(target_os = "windows")]
    {
        log::info!("[cmd] prepare_bundled_jlink (Windows)");
        let override_path = std::env::var(WINUSB_JLINK_DLL_OVERRIDE_ENV).ok();

        let blocking = tokio::task::spawn_blocking(move || {
            let override_dll = override_path
                .as_deref()
                .map(std::path::PathBuf::from);
            crate::infra::runtime::bundled::prepare(&app_clone, override_dll)
        })
        .await;

        let runtime = match blocking {
            Ok(Ok(rt)) => rt,
            Ok(Err(e)) => {
                log::warn!("[cmd] prepare_bundled_jlink failed: {}", e);
                return Err(e.to_string());
            }
            Err(e) => {
                log::warn!("[cmd] prepare_bundled_jlink task join error: {}", e);
                return Err(e.to_string());
            }
        };

        log::info!(
            "[cmd] prepare_bundled_jlink ok: lib={} version={:?}",
            runtime.native_lib_path.display(),
            runtime.version
        );
        state.set_runtime(runtime.clone());
        Ok(runtime.native_lib_path.to_string_lossy().into_owned())
    }

    #[cfg(not(target_os = "windows"))]
    {
        #[cfg(target_os = "linux")]
        {
            log::info!("[cmd] prepare_bundled_jlink (Linux)");
            let blocking =
                tokio::task::spawn_blocking(move || crate::infra::runtime::bundled::prepare(&app_clone))
                    .await;

            let runtime = match blocking {
                Ok(Ok(rt)) => rt,
                Ok(Err(e)) => {
                    log::warn!("[cmd] prepare_bundled_jlink failed: {}", e);
                    return Err(e.to_string());
                }
                Err(e) => {
                    log::warn!("[cmd] prepare_bundled_jlink task join error: {}", e);
                    return Err(e.to_string());
                }
            };

            log::info!(
                "[cmd] prepare_bundled_jlink ok: lib={} version={:?}",
                runtime.native_lib_path.display(),
                runtime.version
            );
            state.set_runtime(runtime.clone());
            Ok(runtime.native_lib_path.to_string_lossy().into_owned())
        }

        #[cfg(not(target_os = "linux"))]
        {
            log::warn!("[cmd] prepare_bundled_jlink: unsupported OS");
            Err("Bundled J-Link runtime is not implemented for this OS yet".to_string())
        }
    }
}
