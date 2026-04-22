//! Probe domain: backend trait, routing, and shared handles.
//!
//! Tauri commands should depend on this module instead of `JLinkService` directly.
//! J-Link remains the only backend today; `ProbeHandle::provider` is the extension point.

use serde::{Deserialize, Serialize};

use crate::domain::jlink::service::JLinkService;
use crate::domain::jlink::types::{
    FirmwareUpdateResult, InstallStatus, Probe, ProbeProvider, UsbDriverMode, UsbDriverResult,
};
use crate::error::AppResult;
use crate::infra::runtime::bundled::JLinkRuntime;

// ─── Handle (IPC / routing) ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProbeHandle {
    pub provider: ProbeProvider,
    pub probe_index: usize,
}

// ─── Backend trait ───────────────────────────────────────────────────────────

pub trait ProbeBackend {
    type Runtime;

    fn diagnostics_json(runtime: Option<&Self::Runtime>) -> serde_json::Value;
    fn detect(runtime: Option<&Self::Runtime>) -> InstallStatus;
    fn ensure_ready(runtime: Option<&Self::Runtime>) -> AppResult<&Self::Runtime>;

    fn scan_probes(rt: &Self::Runtime) -> AppResult<Vec<Probe>>;
    fn update_firmware_only(rt: &Self::Runtime, probe_index: usize) -> AppResult<FirmwareUpdateResult>;
    fn switch_usb_driver(
        rt: &Self::Runtime,
        probe_index: usize,
        mode: UsbDriverMode,
    ) -> AppResult<UsbDriverResult>;
}

/// Prepared runtime for the active backend(s). Becomes an enum when multiple backends exist.
pub type ActiveRuntime = JLinkRuntime;

// ─── Dispatch (single place for `match provider { ... }` later) ──────────────

pub fn diagnostics_json(runtime: Option<&ActiveRuntime>) -> serde_json::Value {
    <JLinkService as ProbeBackend>::diagnostics_json(runtime)
}

pub fn detect(runtime: Option<&ActiveRuntime>) -> InstallStatus {
    <JLinkService as ProbeBackend>::detect(runtime)
}

pub fn ensure_ready(runtime: Option<&ActiveRuntime>) -> AppResult<&ActiveRuntime> {
    <JLinkService as ProbeBackend>::ensure_ready(runtime)
}

pub fn scan_probes(rt: &ActiveRuntime) -> AppResult<Vec<Probe>> {
    <JLinkService as ProbeBackend>::scan_probes(rt)
}

/// Combined detect + scan (optionally bootstraps firmware once per session when missing).
///
/// Kept here so `commands/` stays thin and policy lives in the domain layer.
pub fn detect_and_scan(
    runtime: Option<&ActiveRuntime>,
    run_firmware_bootstrap: bool,
) -> AppResult<(InstallStatus, Vec<Probe>, serde_json::Value)> {
    let status = detect(runtime);
    if !status.installed {
        return Ok((
            status,
            vec![],
            serde_json::json!({ "attempted": false, "updated": 0, "current": 0, "failed": 0 }),
        ));
    }

    let rt = ensure_ready(runtime)?;

    let mut update_attempted = false;
    let mut updated = 0usize;
    let mut current = 0usize;
    let mut failed = 0usize;

    let mut probes = scan_probes(rt)?;

    // Important: UpdateFirmwareIfNewer can take seconds even when "current".
    // To keep app startup snappy, only run the startup firmware ensure when firmware info
    // is missing (common right after udev changes / first attach) and only once per session.
    let has_missing_fw = probes
        .iter()
        .any(|p| p.firmware.as_deref().unwrap_or("").trim().is_empty());

    if run_firmware_bootstrap && has_missing_fw {
        update_attempted = true;
        for i in 0..probes.len() {
            match update_firmware_only(rt, i) {
                Ok(FirmwareUpdateResult::Updated { .. }) => updated += 1,
                Ok(FirmwareUpdateResult::Current { .. }) => current += 1,
                Ok(FirmwareUpdateResult::Failed { .. }) => failed += 1,
                Err(_) => failed += 1,
            }
        }

        // Re-scan after any maintenance so firmware strings reflect reality post-reboot.
        probes = scan_probes(rt)?;
    }

    let summary = serde_json::json!({
        "attempted": update_attempted,
        "updated": updated,
        "current": current,
        "failed": failed
    });

    Ok((status, probes, summary))
}

pub fn update_firmware_only(rt: &ActiveRuntime, probe_index: usize) -> AppResult<FirmwareUpdateResult> {
    <JLinkService as ProbeBackend>::update_firmware_only(rt, probe_index)
}

pub fn switch_usb(rt: &ActiveRuntime, handle: ProbeHandle, mode: UsbDriverMode) -> AppResult<UsbDriverResult> {
    match handle.provider {
        ProbeProvider::JLink => <JLinkService as ProbeBackend>::switch_usb_driver(rt, handle.probe_index, mode),
    }
}
