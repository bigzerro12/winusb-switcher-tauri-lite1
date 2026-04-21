//! Tauri commands for probe operations.
//! Thin wrappers that delegate to the jlink subsystem.

use tauri::State;
use crate::domain::jlink::service::JLinkService;
use crate::domain::jlink::types::{Probe, UsbDriverMode, UsbDriverResult};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Combined detect + scan — called on app startup and after install.
#[tauri::command]
pub async fn detect_and_scan(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, AppError> {
    log::debug!("[cmd] detect_and_scan: enter");
    let rt = state.get_runtime();
    let status = JLinkService::detect(rt.as_ref());
    let run_firmware_bootstrap = status.installed && state.take_firmware_bootstrap_slot();

    let (probes, firmware_update) = if status.installed {
        let rt2 = rt.clone();
        match tokio::task::spawn_blocking(move || -> AppResult<(Vec<Probe>, serde_json::Value)> {
            let rt_ref = JLinkService::ensure_ready(rt2.as_ref())?;

            let mut update_attempted = false;
            let mut updated = 0usize;
            let mut current = 0usize;
            let mut failed = 0usize;

            let mut probes = JLinkService::scan_probes(rt_ref)?;

            // Important: UpdateFirmwareIfNewer can take seconds even when "current".
            // To keep app startup snappy, only run the startup firmware ensure when firmware info
            // is missing (common right after udev changes / first attach) and only once per session.
            let has_missing_fw = probes.iter().any(|p| {
                p.firmware
                    .as_deref()
                    .unwrap_or("")
                    .trim()
                    .is_empty()
            });

            if run_firmware_bootstrap && has_missing_fw {
                update_attempted = true;
                for i in 0..probes.len() {
                    match JLinkService::update_firmware_only(rt_ref, i) {
                        Ok(crate::domain::jlink::types::FirmwareUpdateResult::Updated { .. }) => {
                            updated += 1;
                        }
                        Ok(crate::domain::jlink::types::FirmwareUpdateResult::Current { .. }) => {
                            current += 1;
                        }
                        Ok(crate::domain::jlink::types::FirmwareUpdateResult::Failed { .. }) => {
                            failed += 1;
                        }
                        Err(_) => {
                            failed += 1;
                        }
                    }
                }

                // Re-scan after any maintenance so firmware strings reflect reality post-reboot.
                probes = JLinkService::scan_probes(rt_ref)?;
            }

            let summary = serde_json::json!({
                "attempted": update_attempted,
                "updated": updated,
                "current": current,
                "failed": failed
            });

            Ok((probes, summary))
        })
        .await
        {
            Ok(Ok((probes, fw_summary))) => (probes, fw_summary),
            Ok(Err(e)) => {
                log::warn!("[cmd] detect_and_scan: scan failed: {}", e);
                return Err(e);
            }
            Err(e) => {
                log::warn!("[cmd] detect_and_scan: blocking task failed: {}", e);
                return Err(e.into());
            }
        }
    } else {
        (
            vec![],
            serde_json::json!({ "attempted": false, "updated": 0, "current": 0, "failed": 0 }),
        )
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
pub async fn scan_probes(
    state: State<'_, AppState>,
) -> Result<Vec<Probe>, AppError> {
    log::debug!("[cmd] scan_probes: enter");
    let rt = state.get_runtime();
    let probes = match tokio::task::spawn_blocking(move || -> AppResult<Vec<Probe>> {
        let rt_ref = JLinkService::ensure_ready(rt.as_ref())?;
        JLinkService::scan_probes(rt_ref)
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

/// Switch USB driver for the probe at given index.
/// mode: "winUsb" → WebUSBEnable, "segger" → WebUSBDisable
#[tauri::command]
pub async fn switch_usb_driver(
    probe_index: usize,
    mode: UsbDriverMode,
    state: State<'_, AppState>,
) -> Result<UsbDriverResult, AppError> {
    log::info!(
        "[cmd] switch_usb_driver: probe_index={} mode={:?}",
        probe_index,
        mode
    );
    let rt = state.get_runtime();
    let result = match tokio::task::spawn_blocking(move || -> AppResult<UsbDriverResult> {
        let rt_ref = JLinkService::ensure_ready(rt.as_ref())?;
        JLinkService::switch_usb_driver(rt_ref, probe_index, mode)
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
        log::debug!("[cmd] switch_usb_driver: probe_index={} ok", probe_index);
    } else {
        log::warn!(
            "[cmd] switch_usb_driver: probe_index={} reported failure: {:?}",
            probe_index,
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
    JLinkService::diagnostics_json(rt.as_ref())
}