//! C++ bridge to SEGGER J-Link API:
//! - Windows: `JLink_x64.dll` / `JLinkARM.dll`
//! - Linux: `libjlinkarm.so`
//!
//! The static library links `native/jlink/` and keeps **process-global** state in C++
//! (mutex + loaded API). Only one J-Link “session” semantics apply at a time; a future
//! second vendor should use a separate FFI surface (and ideally a separate `native/*`
//! tree), not a shared generic handle, until both SDK shapes are known.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;

#[cfg(any(target_os = "windows", target_os = "linux"))]
#[link(name = "jlink_arm_bridge", kind = "static")]
unsafe extern "C" {
    fn jlink_bridge_load(path: *const c_char) -> i32;
    fn jlink_bridge_is_loaded() -> i32;
    fn jlink_bridge_last_error() -> *const c_char;
    fn jlink_bridge_free_string(s: *mut c_char);
    fn jlink_bridge_list_probes_json() -> *mut c_char;
    fn jlink_bridge_probe_firmware(index: i32) -> *mut c_char;
    fn jlink_bridge_update_firmware(index: i32) -> *mut c_char;
    fn jlink_bridge_switch_usb_driver(index: i32, winusb: i32) -> *mut c_char;
    fn jlink_bridge_dll_version_string() -> *mut c_char;
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
unsafe fn c_err_msg() -> String {
    let e = jlink_bridge_last_error();
    if e.is_null() {
        "unknown native error".to_string()
    } else {
        CStr::from_ptr(e).to_string_lossy().into_owned()
    }
}

/// Get the most recent detailed native error/debug output captured by the bridge.
/// Useful for diagnosing why a command like WebUSBEnable failed.
#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn last_native_error() -> String {
    unsafe { c_err_msg() }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
unsafe fn take_c_str(ptr: *mut c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let s = CStr::from_ptr(ptr).to_string_lossy().into_owned();
    jlink_bridge_free_string(ptr);
    Some(s)
}

/// Load SEGGER J-Link API library from an absolute UTF-8 path.
/// - Windows: `JLink_x64.dll` or `JLinkARM.dll`
/// - Linux: `libjlinkarm.so` (or versioned equivalent)
#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn bridge_load(dll: &Path) -> Result<(), String> {
    let s = CString::new(
        dll.to_str()
            .ok_or_else(|| "DLL path must be valid UTF-8".to_string())?,
    )
    .map_err(|e| e.to_string())?;
    unsafe {
        if jlink_bridge_load(s.as_ptr()) != 0 {
            return Err(c_err_msg());
        }
    }
    Ok(())
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn bridge_is_loaded() -> bool {
    unsafe { jlink_bridge_is_loaded() != 0 }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn list_probes_json() -> Result<String, String> {
    unsafe {
        let p = jlink_bridge_list_probes_json();
        take_c_str(p).ok_or_else(|| c_err_msg())
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn probe_firmware(index: usize) -> Result<String, String> {
    unsafe {
        let p = jlink_bridge_probe_firmware(index as i32);
        take_c_str(p).ok_or_else(|| c_err_msg())
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn update_firmware_json(index: usize) -> Result<String, String> {
    unsafe {
        let p = jlink_bridge_update_firmware(index as i32);
        take_c_str(p).ok_or_else(|| c_err_msg())
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn switch_usb_json(index: usize, winusb: bool) -> Result<String, String> {
    unsafe {
        let p = jlink_bridge_switch_usb_driver(index as i32, if winusb { 1 } else { 0 });
        take_c_str(p).ok_or_else(|| c_err_msg())
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn dll_version_string() -> Option<String> {
    unsafe {
        let p = jlink_bridge_dll_version_string();
        take_c_str(p)
    }
}

// ─── Unsupported OS stubs ──────────────────────────────────────────────────

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn bridge_load(_dll: &Path) -> Result<(), String> {
    Err("Native J-Link bridge is only available on Windows and Linux.".to_string())
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn bridge_is_loaded() -> bool {
    false
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn list_probes_json() -> Result<String, String> {
    Err("Native J-Link bridge is only available on Windows and Linux.".to_string())
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn probe_firmware(_index: usize) -> Result<String, String> {
    Err("Native J-Link bridge is only available on Windows and Linux.".to_string())
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn update_firmware_json(_index: usize) -> Result<String, String> {
    Err("Native J-Link bridge is only available on Windows and Linux.".to_string())
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn switch_usb_json(_index: usize, _winusb: bool) -> Result<String, String> {
    Err("Native J-Link bridge is only available on Windows and Linux.".to_string())
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn dll_version_string() -> Option<String> {
    None
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn last_native_error() -> String {
    String::new()
}

/// Thin adapter over raw FFI helpers so domain code can depend on `BridgeError` instead of `String`.
pub mod bridge {
    use crate::error::BridgeError;

    pub fn is_loaded() -> bool {
        super::bridge_is_loaded()
    }

    pub fn last_error() -> String {
        super::last_native_error()
    }

    pub fn list_probes_json() -> Result<String, BridgeError> {
        super::list_probes_json().map_err(BridgeError::Failed)
    }

    pub fn probe_firmware(index: usize) -> Result<String, BridgeError> {
        super::probe_firmware(index).map_err(BridgeError::Failed)
    }

    pub fn update_firmware_json(index: usize) -> Result<String, BridgeError> {
        super::update_firmware_json(index).map_err(BridgeError::Failed)
    }

    pub fn switch_usb_json(index: usize, winusb: bool) -> Result<String, BridgeError> {
        super::switch_usb_json(index, winusb).map_err(BridgeError::Failed)
    }
}
