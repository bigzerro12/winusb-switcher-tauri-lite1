//! Tauri commands exposed to the frontend.
//!
//! Principles:
//! - Keep orchestration here (IPC types, spawn_blocking, shaping responses).
//! - Keep policy/logic in `domain/*` so adding providers/backends stays localized.

use serde::Deserialize;
use tauri::{AppHandle, State};

use crate::domain::jlink::types::{Probe, ProbeProvider, UsbDriverMode, UsbDriverResult};
use crate::domain::probe::{self, ProbeHandle};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Env var: optional full path to `JLink_x64.dll` / `JLinkARM.dll` for debugging (Windows).
///
/// This constant is referenced indirectly by end-user documentation; keep it stable even if
/// some build targets do not use it.
#[allow(dead_code)]
pub const WINUSB_JLINK_DLL_OVERRIDE_ENV: &str = "WINUSB_JLINK_DLL_OVERRIDE";

/// Prepare J-Link runtime.
///
/// - Windows: load bundled SEGGER DLL (`JLink_x64.dll` for 64-bit builds).
/// - Linux: load bundled `libjlinkarm.so` directly via the native bridge (no extraction/installer).
#[tauri::command]
pub async fn prepare_bundled_jlink(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    let app_clone = app.clone();

    #[cfg(target_os = "windows")]
    {
        log::info!("[cmd] prepare_bundled_jlink (Windows)");
        let override_path = std::env::var(WINUSB_JLINK_DLL_OVERRIDE_ENV).ok();

        let blocking = tokio::task::spawn_blocking(move || {
            let override_dll = override_path.as_deref().map(std::path::PathBuf::from);
            crate::infra::runtime::bundled::prepare(&app_clone, override_dll)
        })
        .await;

        let runtime = match blocking {
            Ok(Ok(rt)) => rt,
            Ok(Err(e)) => {
                log::warn!("[cmd] prepare_bundled_jlink failed: {}", e);
                return Err(e);
            }
            Err(e) => {
                log::warn!("[cmd] prepare_bundled_jlink task join error: {}", e);
                return Err(AppError::Internal(e.to_string()));
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
                    return Err(e);
                }
                Err(e) => {
                    log::warn!("[cmd] prepare_bundled_jlink task join error: {}", e);
                    return Err(AppError::Internal(e.to_string()));
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
            Err(AppError::Runtime(
                "Bundled J-Link runtime is not implemented for this OS yet".to_string(),
            ))
        }
    }
}

/// Single payload for `switch_usb_driver` IPC (`probeIndex`, `mode`, optional `provider`).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchUsbRequest {
    pub probe_index: usize,
    #[serde(default)]
    pub serial_number: Option<String>,
    #[serde(default)]
    pub provider: ProbeProvider,
    pub mode: UsbDriverMode,
}

/// Combined detect + scan — called on app startup and after install.
#[tauri::command]
pub async fn detect_and_scan(state: State<'_, AppState>) -> Result<serde_json::Value, AppError> {
    log::debug!("[cmd] detect_and_scan: enter");
    let rt = state.get_runtime();
    let run_firmware_bootstrap = state.take_firmware_bootstrap_slot();

    let (status, probes, firmware_update) =
        match tokio::task::spawn_blocking(move || -> AppResult<(crate::domain::jlink::types::InstallStatus, Vec<Probe>, serde_json::Value)> {
            probe::detect_and_scan(rt.as_ref(), run_firmware_bootstrap)
        })
        .await
        {
            Ok(Ok(v)) => v,
            Ok(Err(e)) => {
                log::warn!("[cmd] detect_and_scan: scan failed: {}", e);
                return Err(e);
            }
            Err(e) => {
                log::warn!("[cmd] detect_and_scan: blocking task failed: {}", e);
                return Err(e.into());
            }
        };

    log::debug!(
        "[cmd] detect_and_scan: ok (installed={}, probes={})",
        status.installed,
        probes.len()
    );
    Ok(serde_json::json!({ "status": status, "probes": probes, "firmwareUpdate": firmware_update }))
}

/// Scan probes only (J-Link already known to be installed).
#[tauri::command]
pub async fn scan_probes(state: State<'_, AppState>) -> Result<Vec<Probe>, AppError> {
    log::debug!("[cmd] scan_probes: enter");
    let rt = state.get_runtime();
    let probes = match tokio::task::spawn_blocking(move || -> AppResult<Vec<Probe>> {
        let rt_ref = probe::ensure_ready(rt.as_ref())?;
        probe::scan_probes(rt_ref)
    })
    .await
    {
        Ok(Ok(probes)) => probes,
        Ok(Err(e)) => {
            log::warn!("[cmd] scan_probes failed: {}", e);
            return Err(e);
        }
        Err(e) => {
            log::warn!("[cmd] scan_probes: blocking task failed: {}", e);
            return Err(e.into());
        }
    };
    log::debug!("[cmd] scan_probes: ok (count={})", probes.len());
    Ok(probes)
}

/// Execute a J-Link Commander-style `exec <Command>` string against a probe index.
/// Returns combined stdout + captured callback output.
#[tauri::command]
pub async fn jlink_exec_command(
    probe_index: usize,
    exec_cmd: String,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    let _rt = state.get_runtime();
    // We rely on "bridge loaded" invariant established by prepare_bundled_jlink.
    let out = tokio::task::spawn_blocking(move || -> AppResult<String> {
        if !crate::jlink_ffi::bridge::is_loaded() {
            return Err(AppError::Bridge(
                "Native bridge not loaded. Call prepare_bundled_jlink first.".to_string(),
            ));
        }
        crate::jlink_ffi::bridge::exec_command(probe_index, &exec_cmd).map_err(AppError::from)
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))??;

    Ok(out)
}

/// Switch USB driver. Payload: `{ probeIndex, mode, provider? }` (camelCase).
/// `provider` defaults to J-Link when omitted.
#[tauri::command]
pub async fn switch_usb_driver(
    request: SwitchUsbRequest,
    state: State<'_, AppState>,
) -> Result<UsbDriverResult, AppError> {
    let mut handle = ProbeHandle {
        provider: request.provider,
        probe_index: request.probe_index,
    };
    let mode = request.mode;
    log::info!(
        "[cmd] switch_usb_driver: probe={:?} mode={:?}",
        handle,
        mode
    );
    let rt = state.get_runtime();
    let result = match tokio::task::spawn_blocking(move || -> AppResult<UsbDriverResult> {
        let rt_ref = probe::ensure_ready(rt.as_ref())?;

        // Probe indices can reorder during reboot / re-enumeration. If the frontend provides
        // a stable serial number, re-resolve the index right before switching.
        if let Some(sn) = request.serial_number.as_deref() {
            if !sn.trim().is_empty() {
                match handle.provider {
                    ProbeProvider::JLink => {
                        if let Ok(idx) = crate::domain::jlink::service::JLinkService::resolve_probe_index_by_serial(sn) {
                            handle.probe_index = idx;
                        }
                    }
                }
            }
        }

        probe::switch_usb(rt_ref, handle, mode)
    })
    .await
    {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            log::warn!("[cmd] switch_usb_driver failed: {}", e);
            return Err(e);
        }
        Err(e) => {
            log::warn!("[cmd] switch_usb_driver: blocking task failed: {}", e);
            return Err(e.into());
        }
    };

    if result.success {
        log::debug!("[cmd] switch_usb_driver: ok");
    } else {
        log::warn!(
            "[cmd] switch_usb_driver: reported failure: {:?}",
            result.error
        );
    }
    Ok(result)
}

/// Returns the compiled OS and CPU architecture of this binary.
/// Values come from `std::env::consts` so they always match the actual build target.
#[tauri::command]
pub fn get_arch_info() -> serde_json::Value {
    serde_json::json!({
        "os":   std::env::consts::OS,
        "arch": std::env::consts::ARCH,
    })
}

/// Runtime / bridge / bundle layout snapshot for support and debugging.
#[tauri::command]
pub fn get_jlink_diagnostics(state: State<'_, AppState>) -> serde_json::Value {
    log::trace!("[cmd] get_jlink_diagnostics");
    let rt = state.get_runtime();
    probe::diagnostics_json(rt.as_ref())
}

#[cfg(test)]
mod tests {
    use super::SwitchUsbRequest;
    use crate::domain::jlink::types::{ProbeProvider, UsbDriverMode};

    #[test]
    fn switch_usb_request_defaults_provider_when_omitted() {
        let v = serde_json::json!({
            "probeIndex": 3,
            "mode": "winUsb"
        });
        let req: SwitchUsbRequest = serde_json::from_value(v).expect("request must deserialize");
        assert_eq!(req.probe_index, 3);
        assert_eq!(req.mode, UsbDriverMode::WinUsb);
        assert_eq!(req.provider, ProbeProvider::JLink);
        assert!(req.serial_number.is_none());
    }

    #[test]
    fn switch_usb_request_accepts_serial_and_provider() {
        let v = serde_json::json!({
            "probeIndex": 1,
            "mode": "segger",
            "provider": "JLink",
            "serialNumber": "123456789"
        });
        let req: SwitchUsbRequest = serde_json::from_value(v).expect("request must deserialize");
        assert_eq!(req.probe_index, 1);
        assert_eq!(req.mode, UsbDriverMode::Segger);
        assert_eq!(req.provider, ProbeProvider::JLink);
        assert_eq!(req.serial_number.as_deref(), Some("123456789"));
    }
}

