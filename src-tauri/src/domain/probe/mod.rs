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
    Probe, ProbeProvider, RuntimeStatus, UsbDriverMode, UsbDriverResult,
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
    fn detect(runtime: Option<&Self::Runtime>) -> RuntimeStatus;
    fn ensure_ready(runtime: Option<&Self::Runtime>) -> AppResult<&Self::Runtime>;

    fn scan_probes(rt: &Self::Runtime) -> AppResult<Vec<Probe>>;
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

    // NOTE: We intentionally do not run any firmware update routine during detect+scan.
    // Firmware update is performed only when explicitly invoked by higher-level workflows.
    let _ = run_firmware_bootstrap; // keep signature stable; flag is interpreted by caller.

    let summary = serde_json::json!({
        "attempted": update_attempted,
        "updated": updated,
        "current": current,
        "failed": failed
    });

    Ok((status, probes, summary))
}

pub fn switch_usb(rt: &ActiveRuntime, handle: ProbeHandle, mode: UsbDriverMode) -> AppResult<UsbDriverResult> {
    match handle.provider {
        ProbeProvider::JLink => <JLinkService as ProbeBackend>::switch_usb_driver(rt, handle.probe_index, mode),
    }
}
