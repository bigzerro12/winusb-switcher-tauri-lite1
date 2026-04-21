//! WinUSB Switcher Lite–only commands.

use tauri::{AppHandle, State};

use crate::state::AppState;

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
        // Test override: allow forcing the DLL from a full SEGGER installation
        // (useful to rule out missing resources in the bundled tree).
        // If set, should point to `JLink_x64.dll` (recommended) or `JLinkARM.dll` (32-bit).
        const OVERRIDE_ENV: &str = "WINUSB_JLINK_DLL_OVERRIDE";
        let override_path = std::env::var(OVERRIDE_ENV).ok();

        let runtime = tokio::task::spawn_blocking(move || {
            let override_dll = override_path
                .as_deref()
                .map(std::path::PathBuf::from);
            crate::infra::runtime::bundled::prepare(&app_clone, override_dll)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

        state.set_runtime(runtime.clone());
        Ok(runtime.native_lib_path.to_string_lossy().into_owned())
    }

    #[cfg(not(target_os = "windows"))]
    {
        #[cfg(target_os = "linux")]
        {
            let runtime = tokio::task::spawn_blocking(move || {
                crate::infra::runtime::bundled::prepare(&app_clone)
            })
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;

            state.set_runtime(runtime.clone());
            Ok(runtime.native_lib_path.to_string_lossy().into_owned())
        }

        #[cfg(not(target_os = "linux"))]
        {
            Err("Bundled J-Link runtime is not implemented for this OS yet".to_string())
        }
    }
}
