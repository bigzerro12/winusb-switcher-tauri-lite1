//! Bridge-backed backend adapter.
//!
//! Initially thin: just wraps `crate::jlink_ffi` so the rest of the app can depend on an
//! adapter module rather than calling FFI directly.

use crate::infra::jlink_backend::errors::BridgeError;

pub fn is_loaded() -> bool {
    crate::jlink_ffi::bridge_is_loaded()
}

pub fn last_error() -> String {
    crate::jlink_ffi::last_native_error()
}

pub fn list_probes_json() -> Result<String, BridgeError> {
    crate::jlink_ffi::list_probes_json().map_err(BridgeError::Failed)
}

pub fn probe_firmware(index: usize) -> Result<String, BridgeError> {
    crate::jlink_ffi::probe_firmware(index).map_err(BridgeError::Failed)
}

pub fn update_firmware_json(index: usize) -> Result<String, BridgeError> {
    crate::jlink_ffi::update_firmware_json(index).map_err(BridgeError::Failed)
}

pub fn switch_usb_json(index: usize, winusb: bool) -> Result<String, BridgeError> {
    crate::jlink_ffi::switch_usb_json(index, winusb).map_err(BridgeError::Failed)
}

