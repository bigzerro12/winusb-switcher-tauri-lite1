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
    let rt = state.get_runtime();
    let status = JLinkService::detect(rt.as_ref());

    let probes = if status.installed {
        let rt2 = rt.clone();
        tokio::task::spawn_blocking(move || -> AppResult<Vec<Probe>> {
            let rt_ref = JLinkService::ensure_ready(rt2.as_ref())?;
            JLinkService::scan_probes(rt_ref)
        })
        .await??
    } else {
        vec![]
    };

    Ok(serde_json::json!({ "status": status, "probes": probes }))
}

/// Scan probes only (J-Link already known to be installed).
#[tauri::command]
pub async fn scan_probes(
    state: State<'_, AppState>,
) -> Result<Vec<Probe>, AppError> {
    let rt = state.get_runtime();
    let probes = tokio::task::spawn_blocking(move || -> AppResult<Vec<Probe>> {
        let rt_ref = JLinkService::ensure_ready(rt.as_ref())?;
        JLinkService::scan_probes(rt_ref)
    })
    .await??;
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
    let rt = state.get_runtime();
    tokio::task::spawn_blocking(move || -> AppResult<UsbDriverResult> {
        let rt_ref = JLinkService::ensure_ready(rt.as_ref())?;
        JLinkService::switch_usb_driver(rt_ref, probe_index, mode)
    })
    .await?
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
    let rt = state.get_runtime();
    JLinkService::diagnostics_json(rt.as_ref())
}