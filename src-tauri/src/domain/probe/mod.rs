//! Probe domain entry point.
//!
//! Commands should call this module, not backend implementations directly.
//! Today there is one backend (J-Link), but the provider field keeps routing extensible.
//!
//! To add another probe family later:
//! - add a `ProbeProvider` variant
//! - add a `domain/<vendor>/` backend implementation
//! - wire dispatch in `switch_usb` (and related command paths)
//!
//! Current limits:
//! - CI covers parsing/unit behavior but not hardware-in-the-loop switching.
//! - USB behavior still depends on platform policy and connected probe firmware.

use serde::{Deserialize, Serialize};

use crate::domain::jlink::service::JLinkService;
use crate::domain::jlink::types::{
    Probe, ProbeProvider, RuntimeStatus, UsbDriverMode, UsbDriverResult,
};
use crate::error::AppResult;
use crate::infra::runtime::bundled::JLinkRuntime;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProbeHandle {
    pub provider: ProbeProvider,
    pub probe_index: usize,
}

pub trait ProbeBackend {
    type Runtime;

    fn diagnostics_json(runtime: Option<&Self::Runtime>) -> serde_json::Value;
    fn detect(runtime: Option<&Self::Runtime>) -> RuntimeStatus;
    fn ensure_ready(runtime: Option<&Self::Runtime>) -> AppResult<&Self::Runtime>;

    fn scan_probes(rt: &Self::Runtime) -> AppResult<Vec<Probe>>;
    fn switch_usb_driver(
        rt: &Self::Runtime,
        probe_index: usize,
        mode: UsbDriverMode,
    ) -> AppResult<UsbDriverResult>;
}

/// Runtime for the active backend.
pub type ActiveRuntime = JLinkRuntime;

pub fn diagnostics_json(runtime: Option<&ActiveRuntime>) -> serde_json::Value {
    <JLinkService as ProbeBackend>::diagnostics_json(runtime)
}

pub fn detect(runtime: Option<&ActiveRuntime>) -> RuntimeStatus {
    <JLinkService as ProbeBackend>::detect(runtime)
}

pub fn ensure_ready(runtime: Option<&ActiveRuntime>) -> AppResult<&ActiveRuntime> {
    <JLinkService as ProbeBackend>::ensure_ready(runtime)
}

pub fn scan_probes(rt: &ActiveRuntime) -> AppResult<Vec<Probe>> {
    <JLinkService as ProbeBackend>::scan_probes(rt)
}

/// Combined detect + scan.
///
/// Kept here so `commands/` stays thin and policy lives in the domain layer.
pub fn detect_and_scan(
    runtime: Option<&ActiveRuntime>,
    run_firmware_bootstrap: bool,
) -> AppResult<(RuntimeStatus, Vec<Probe>, serde_json::Value)> {
    let status = detect(runtime);
    if !status.ready {
        return Ok((
            status,
            vec![],
            serde_json::json!({ "attempted": false, "updated": 0, "current": 0, "failed": 0 }),
        ));
    }

    let rt = ensure_ready(runtime)?;

    let update_attempted = false;
    let updated = 0usize;
    let current = 0usize;
    let failed = 0usize;

    let probes = scan_probes(rt)?;

    // Detect-and-scan is read-only. Firmware updates happen only in explicit switch/update flows.
    let _ = run_firmware_bootstrap; // keep signature stable; flag is interpreted by caller.

    let summary = serde_json::json!({
        "attempted": update_attempted,
        "updated": updated,
        "current": current,
        "failed": failed
    });

    Ok((status, probes, summary))
}

pub fn switch_usb(
    rt: &ActiveRuntime,
    handle: ProbeHandle,
    mode: UsbDriverMode,
) -> AppResult<UsbDriverResult> {
    match handle.provider {
        ProbeProvider::JLink => {
            <JLinkService as ProbeBackend>::switch_usb_driver(rt, handle.probe_index, mode)
        }
    }
}
