//! Backend abstraction for probe operations.
//!
//! Today the app only ships a SEGGER J-Link backend, but we intentionally keep an
//! abstraction layer here so supporting additional probe families later is additive
//! (new backend implementation) rather than scattering `if probe_type == ...` across
//! commands + domain + native code.

use crate::domain::jlink::types::{
    FirmwareUpdateResult, InstallStatus, Probe, UsbDriverMode, UsbDriverResult,
};
use crate::error::AppResult;
use crate::infra::runtime::bundled::JLinkRuntime;

/// Common probe backend surface used by the Tauri commands / domain services.
///
/// Notes:
/// - The DTOs (`Probe`, `UsbDriverResult`, ...) are currently under `domain::jlink::types`
///   for historical reasons; they are intentionally UI-stable and can be reused by
///   future backends.
/// - `Runtime` is backend-specific. For now it’s `JLinkRuntime`.
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

/// Type alias for the currently selected backend runtime.
/// When we add a second backend, this becomes an enum that can hold different runtimes.
pub type ActiveRuntime = JLinkRuntime;

