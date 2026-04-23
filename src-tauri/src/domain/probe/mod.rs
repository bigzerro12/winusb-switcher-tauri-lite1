//! Probe domain: backend trait, routing, and shared handles.
//!
//! Tauri commands should depend on this module instead of `JLinkService` directly.
//! J-Link remains the only backend today; `ProbeHandle::provider` is the extension point.
//!
//! ## Adding another probe family (e.g. E2)
//!
//! - Add a `ProbeProvider` variant, `domain/<vendor>/` with `ProbeBackend`, and parallel
//!   `native/<vendor>/` + Rust FFI if the vendor ships a C/C++ API.
//! - Replace [`ActiveRuntime`] `type` alias with an `enum` once more than one runtime
//!   must be prepared in the same session; extend [`switch_usb`] (and commands) with
//!   `match handle.provider { ... }` arms.
//!
//! ## Operational limits (intentional for this codebase)
//!
//! - **In-process native bridge:** SEGGER code runs in the app process; a hard crash in
//!   the DLL/native layer can take down the UI. A sidecar binary would isolate that risk.
//! - **No CI hardware tests:** `cargo test` covers pure Rust and JSON parsing; USB/probe
//!   flows still need manual or lab automation with real devices.
//! - **Platform feature gaps:** USB stack switching is centered on what J-Link exposes;
//!   Linux paths differ from Windows (udev, permissions) and are documented in UX copy.

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

fn startup_firmware_ensure_forced_from_env() -> bool {
    std::env::var("WINUSB_STARTUP_FIRMWARE_ENSURE")
        .map(|v| {
            let v = v.trim().to_ascii_lowercase();
            v == "1" || v == "true" || v == "yes"
        })
        .unwrap_or(false)
}

/// Combined detect + scan (optionally bootstraps firmware once per session when missing).
///
/// Kept here so `commands/` stays thin and policy lives in the domain layer.
///
/// **Startup `UpdateFirmwareIfNewer`:** By default this runs **only on the first
/// `detect_and_scan` of the session** when **any** probe lacks a non-empty `firmware`
/// string after the initial scan. If enumeration already filled `firmware` (discovery
/// string and/or OpenEx), the ensure step is **skipped** to keep startup fast—even if
/// the probe could still accept a newer bundled `.bin`. That can make **release vs dev**
/// look different when dev sometimes sees empty firmware on the first pass (timing /
/// OpenEx). Set **`WINUSB_STARTUP_FIRMWARE_ENSURE=1`** to force the ensure step whenever
/// the bootstrap slot is still available.
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
    let force_ensure = startup_firmware_ensure_forced_from_env();
    let should_ensure_fw = has_missing_fw || force_ensure;

    if run_firmware_bootstrap && should_ensure_fw {
        if force_ensure && !has_missing_fw {
            log::info!(
                "[probe] startup firmware ensure: WINUSB_STARTUP_FIRMWARE_ENSURE set — running UpdateFirmwareIfNewer for all probes despite non-empty firmware strings"
            );
        }
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
    } else if run_firmware_bootstrap && !should_ensure_fw {
        log::info!(
            "[probe] startup firmware ensure skipped: every probe already has a firmware string (set WINUSB_STARTUP_FIRMWARE_ENSURE=1 to force UpdateFirmwareIfNewer on open)"
        );
    } else if !run_firmware_bootstrap && should_ensure_fw {
        log::debug!(
            "[probe] firmware ensure not run: one-shot bootstrap slot already used this session (retry needs Refresh or new app launch)"
        );
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
