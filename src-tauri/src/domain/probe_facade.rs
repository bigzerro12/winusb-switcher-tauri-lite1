//! Probe facade used by Tauri commands.
//!
//! Commands should depend on this module rather than a concrete backend (J-Link).
//! Today we always route to the J-Link backend; later this becomes the single
//! place where we choose a backend based on probe/provider/runtime selection.

use crate::domain::jlink::service::JLinkService;
use crate::domain::jlink::types::{
    FirmwareUpdateResult, InstallStatus, Probe, ProbeProvider, UsbDriverMode, UsbDriverResult,
};
use crate::domain::probe_backend::{ActiveRuntime, ProbeBackend};
use crate::domain::probe_handle::ProbeHandle;
use crate::error::AppResult;

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

pub fn update_firmware_only(rt: &ActiveRuntime, probe_index: usize) -> AppResult<FirmwareUpdateResult> {
    <JLinkService as ProbeBackend>::update_firmware_only(rt, probe_index)
}

pub fn switch_usb_driver(
    rt: &ActiveRuntime,
    probe_index: usize,
    mode: UsbDriverMode,
) -> AppResult<UsbDriverResult> {
    <JLinkService as ProbeBackend>::switch_usb_driver(rt, probe_index, mode)
}

/// Multi-backend-ready version. Commands can migrate to this without changing domain logic again.
pub fn switch_usb_driver_for(
    rt: &ActiveRuntime,
    handle: ProbeHandle,
    mode: UsbDriverMode,
) -> AppResult<UsbDriverResult> {
    match handle.provider {
        ProbeProvider::JLink => <JLinkService as ProbeBackend>::switch_usb_driver(rt, handle.probe_index, mode),
    }
}

#[allow(dead_code)]
pub fn update_firmware_only_for(
    rt: &ActiveRuntime,
    handle: ProbeHandle,
) -> AppResult<FirmwareUpdateResult> {
    match handle.provider {
        ProbeProvider::JLink => <JLinkService as ProbeBackend>::update_firmware_only(rt, handle.probe_index),
    }
}

