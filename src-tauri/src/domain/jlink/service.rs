//! Domain service for J-Link operations.
//!
//! Tauri commands should use `crate::domain::probe`; this module is the J-Link backend
//! implementation behind `ProbeBackend`.

use crate::domain::jlink::types::{
    FirmwareUpdateResult, Probe, ProbeProvider, RuntimeStatus, UsbDriverMode, UsbDriverResult,
};
use crate::domain::probe::ProbeBackend;
use crate::error::{AppError, AppResult, BridgeError};
use crate::jlink_ffi::{bridge, ProbeOpenDetails};
use crate::infra::runtime::bundled::JLinkRuntime;

pub struct JLinkService;

impl JLinkService {
    /// Support / diagnostics snapshot for `get_jlink_diagnostics` (and tooling).
    fn diagnostics_json(runtime: Option<&JLinkRuntime>) -> serde_json::Value {
        let bridge_loaded = bridge::is_loaded();
        let env_dll = std::env::var(crate::bundled_jlink::WINUSB_JLINK_DLL_PATH_ENV).ok();
        let target_os = std::env::consts::OS;
        let target_arch = std::env::consts::ARCH;

        if let Some(rt) = runtime {
            let firmwares_dir = rt.runtime_dir.join("Firmwares");
            let version_ui = rt
                .version
                .as_deref()
                .and_then(Self::format_version_for_ui)
                .or_else(|| rt.version.clone());
            return serde_json::json!({
                "runtimePrepared": true,
                "bridgeLoaded": bridge_loaded,
                "runtimeDir": rt.runtime_dir.to_string_lossy(),
                "nativeLibPath": rt.native_lib_path.to_string_lossy(),
                "versionRaw": rt.version,
                "versionUi": version_ui,
                "firmwaresDir": firmwares_dir.to_string_lossy(),
                "firmwaresDirExists": firmwares_dir.is_dir(),
                "winusbJlinkDllPathEnv": env_dll,
                "targetOs": target_os,
                "targetArch": target_arch,
            });
        }

        serde_json::json!({
            "runtimePrepared": false,
            "bridgeLoaded": bridge_loaded,
            "winusbJlinkDllPathEnv": env_dll,
            "targetOs": target_os,
            "targetArch": target_arch,
        })
    }

    fn format_version_for_ui(raw: &str) -> Option<String> {
        // Common SEGGER strings:
        // - "SEGGER J-Link DLL (build 93600)"  -> V9.36
        // - "SEGGER J-Link DLL (build 105200)" -> V10.52
        let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
        let (major, minor, patch) = match digits.len() {
            5 => (
                digits[0..1].parse::<u32>().ok()?,
                digits[1..3].parse::<u32>().ok()?,
                digits[3..5].parse::<u32>().ok()?,
            ),
            6 => (
                digits[0..2].parse::<u32>().ok()?,
                digits[2..4].parse::<u32>().ok()?,
                digits[4..6].parse::<u32>().ok()?,
            ),
            _ => return None,
        };
        if patch == 0 {
            Some(format!("V{}.{}", major, minor))
        } else {
            Some(format!("V{}.{}.{}", major, minor, patch))
        }
    }

    fn ensure_ready(runtime: Option<&JLinkRuntime>) -> AppResult<&JLinkRuntime> {
        runtime.ok_or_else(|| {
            AppError::Runtime("Runtime not prepared. Call prepare_bundled_jlink first.".to_string())
        })
    }

    fn ensure_bridge_loaded() -> AppResult<()> {
        if bridge::is_loaded() {
            Ok(())
        } else {
            Err(AppError::Bridge(
                "Native bridge not loaded. Call prepare_bundled_jlink first.".to_string(),
            ))
        }
    }

    fn detect(runtime: Option<&JLinkRuntime>) -> RuntimeStatus {
        if let Some(rt) = runtime {
            let ui_version = rt
                .version
                .as_deref()
                .and_then(Self::format_version_for_ui)
                .or_else(|| rt.version.clone());
            log::debug!(
                "[jlink] Runtime prepared: dir={} lib={} version={:?}",
                rt.runtime_dir.display(),
                rt.native_lib_path.display(),
                rt.version
            );
            return RuntimeStatus {
                ready: true,
                native_lib_path: Some(rt.native_lib_path.to_string_lossy().into_owned()),
                version: ui_version,
            };
        }
        log::debug!(
            "[jlink] detect: runtime not prepared yet (prepare_bundled_jlink not completed)"
        );
        RuntimeStatus {
            ready: false,
            native_lib_path: None,
            version: None,
        }
    }

    fn scan_probes(_rt: &JLinkRuntime) -> AppResult<Vec<Probe>> {
        log::debug!("[jlink] scan_probes: start");
        Self::ensure_bridge_loaded()?;
        let probes = scan_probes_via_bridge()?;
        log::info!("[jlink] scan_probes: {} probe(s)", probes.len());
        Ok(probes)
    }

    fn switch_usb_driver(
        _rt: &JLinkRuntime,
        probe_index: usize,
        mode: UsbDriverMode,
    ) -> AppResult<UsbDriverResult> {
        log::debug!("[jlink] switch_usb_driver: probe[{}] mode={:?}", probe_index, mode);
        Self::ensure_bridge_loaded()?;

        match update_firmware_via_bridge(probe_index) {
            FirmwareUpdateResult::Failed { error } => {
                log::warn!(
                    "[jlink] firmware update failed before USB driver switch (probe[{}]): {}",
                    probe_index,
                    error
                );
                return Ok(UsbDriverResult {
                    success: false,
                    error: Some(format!("Firmware update failed: {}", error)),
                    detail: None,
                    reboot_not_supported: false,
                });
            }
            FirmwareUpdateResult::Updated { .. } => {
                log::info!(
                    "[jlink] Probe[{}] firmware updated; continuing with USB driver switch",
                    probe_index
                );
            }
            FirmwareUpdateResult::Current { .. } => {
                log::info!(
                    "[jlink] Probe[{}] firmware already current; continuing with USB driver switch",
                    probe_index
                );
            }
        }

        Ok(switch_usb_via_bridge(probe_index, mode))
    }

}

impl JLinkService {
    /// More stable switch path for Linux: resolve emulator by USB serial number at call time.
    ///
    /// This avoids stale list indices during reboot/re-enumeration and adds small list retries.
    pub fn switch_usb_driver_by_serial(
        _rt: &JLinkRuntime,
        serial_number: &str,
        mode: UsbDriverMode,
    ) -> AppResult<UsbDriverResult> {
        log::debug!("[jlink] switch_usb_driver_by_serial: sn={} mode={:?}", serial_number, mode);
        Self::ensure_bridge_loaded()?;

        let sn_u32: u32 = serial_number
            .trim()
            .parse()
            .map_err(|_| AppError::Internal(format!("invalid serialNumber: {}", serial_number)))?;

        // During firmware update / reboot, the probe may temporarily disappear from EMU_GetList.
        // Retry list resolution for a short time.
        let retries = 20;
        let retry_delay_ms = 250;

        match update_firmware_via_bridge_by_sn(sn_u32, retries, retry_delay_ms) {
            FirmwareUpdateResult::Failed { error } => {
                log::warn!(
                    "[jlink] firmware update failed before USB driver switch (sn={}): {}",
                    serial_number,
                    error
                );
                return Ok(UsbDriverResult {
                    success: false,
                    error: Some(format!("Firmware update failed: {}", error)),
                    detail: None,
                    reboot_not_supported: false,
                });
            }
            FirmwareUpdateResult::Updated { .. } => {
                log::info!(
                    "[jlink] Probe firmware updated (sn={}); continuing with USB driver switch",
                    serial_number
                );
            }
            FirmwareUpdateResult::Current { .. } => {
                log::info!(
                    "[jlink] Probe firmware already current (sn={}); continuing with USB driver switch",
                    serial_number
                );
            }
        }

        Ok(switch_usb_via_bridge_by_sn(sn_u32, mode, retries, retry_delay_ms))
    }
}

fn update_firmware_via_bridge_by_sn(
    serial_number: u32,
    retries: i32,
    retry_delay_ms: u32,
) -> FirmwareUpdateResult {
    match bridge::update_firmware_json_by_sn(serial_number, retries, retry_delay_ms) {
        Ok(raw) => parse_firmware_update_response(serial_number as usize, &raw),
        Err(e) => {
            log::warn!(
                "[jlink][bridge] update_firmware_json_by_sn failed sn={}: {}",
                serial_number,
                e
            );
            FirmwareUpdateResult::Failed {
                error: e.to_string(),
            }
        }
    }
}

fn switch_usb_via_bridge_by_sn(
    serial_number: u32,
    mode: UsbDriverMode,
    retries: i32,
    retry_delay_ms: u32,
) -> UsbDriverResult {
    let winusb = matches!(mode, UsbDriverMode::WinUsb);
    match bridge::switch_usb_json_by_sn(serial_number, winusb, retries, retry_delay_ms) {
        Ok(raw) => parse_switch_usb_response(serial_number as usize, &raw),
        Err(e) => UsbDriverResult {
            success: false,
            error: Some(format!(
                "{}\n\n(native detail)\n{}",
                e,
                bridge::last_error()
            )),
            detail: Some(bridge::last_error()),
            reboot_not_supported: false,
        },
    }
}

// Backend abstraction implementation (enables future probe families without scattering branching logic).
impl ProbeBackend for JLinkService {
    type Runtime = JLinkRuntime;

    fn diagnostics_json(runtime: Option<&Self::Runtime>) -> serde_json::Value {
        Self::diagnostics_json(runtime)
    }

    fn detect(runtime: Option<&Self::Runtime>) -> RuntimeStatus {
        Self::detect(runtime)
    }

    fn ensure_ready(runtime: Option<&Self::Runtime>) -> AppResult<&Self::Runtime> {
        Self::ensure_ready(runtime)
    }

    fn scan_probes(rt: &Self::Runtime) -> AppResult<Vec<Probe>> {
        Self::scan_probes(rt)
    }

    fn switch_usb_driver(rt: &Self::Runtime, probe_index: usize, mode: UsbDriverMode) -> AppResult<UsbDriverResult> {
        Self::switch_usb_driver(rt, probe_index, mode)
    }
}

fn parse_discovery_firmware_string(s: &str) -> String {
    let s = s.trim();
    if s.is_empty() {
        return String::new();
    }
    if let Some(pos) = s.find("compiled ") {
        let rest = s[pos + 9..].trim();
        let end = rest
            .find('\r')
            .or_else(|| rest.find('\n'))
            .unwrap_or(rest.len());
        rest[..end].trim().to_string()
    } else {
        s.to_string()
    }
}

fn log_probes_summary(source: &str, probes: &[Probe]) {
    log::debug!(
        "[jlink] scan summary [{}]: {} probe(s)",
        source,
        probes.len()
    );
    for (i, p) in probes.iter().enumerate() {
        let fw = p.firmware.as_deref().unwrap_or("(none)");
        log::debug!(
            "[jlink]   [{}] sn={} nick={} product={} conn={} driver={} firmware={}",
            i,
            p.serial_number,
            if p.nick_name.is_empty() { "-" } else { &p.nick_name },
            if p.product_name.is_empty() { "-" } else { &p.product_name },
            if p.connection.is_empty() { "-" } else { &p.connection },
            if p.driver.is_empty() { "-" } else { &p.driver },
            fw
        );
    }
}

fn scan_probes_via_bridge() -> AppResult<Vec<Probe>> {
    use serde_json::Value;

    let json = bridge::list_probes_json().map_err(AppError::from)?;
    log::trace!("[jlink] list_probes_json: {} bytes", json.len());
    let rows: Vec<Value> = serde_json::from_str(&json).map_err(|e| {
        log::warn!(
            "[jlink][bridge] list_probes JSON parse failed ({} bytes): {}",
            json.len(),
            e
        );
        AppError::Internal(e.to_string())
    })?;

    let mut probes = Vec::new();
    for row in rows {
        let index = row["index"].as_u64().unwrap_or(0) as usize;
        let serial = row["serialNumber"].as_str().unwrap_or("").to_string();
        if serial.is_empty() {
            continue;
        }

        let discovery_fw = row["discoveryFirmware"]
            .as_str()
            .map(parse_discovery_firmware_string)
            .filter(|s| !s.is_empty());

        let t0 = std::time::Instant::now();

        let try_read = || -> Result<ProbeOpenDetails, BridgeError> { bridge::probe_open_details(index) };

        let (firmware, fw_src, driver_label): (Option<String>, &'static str, String) = match try_read() {
            Ok(d) if !d.firmware.is_empty() => (Some(d.firmware), "bridge_openex", d.usb_driver),
            Ok(d) => (discovery_fw.clone(), "discovery_only", d.usb_driver),
            Err(e) => {
                // On cold start (or right after udev install), OpenEx may fail transiently.
                // Retry once quickly to avoid "firmware missing until refresh".
                let msg = e.to_string();
                let transient = msg.contains("Cannot connect")
                    || msg.contains("Could not read J-Link capabilities")
                    || msg.contains("Communication timed out");

                if transient {
                    log::debug!(
                        "[jlink] probe_open_details transient OpenEx error index={} sn={} — {} (retrying once)",
                        index,
                        serial,
                        msg
                    );
                    std::thread::sleep(std::time::Duration::from_millis(250));
                    match try_read() {
                        Ok(d) if !d.firmware.is_empty() => {
                            (Some(d.firmware), "bridge_openex_retry", d.usb_driver)
                        }
                        Ok(d) => (discovery_fw.clone(), "discovery_only_retry", d.usb_driver),
                        Err(e2) => {
                            log::warn!(
                                "[jlink] probe_open_details failed after retry index={} sn={} — {} (using discovery if present)",
                                index,
                                serial,
                                e2
                            );
                            (discovery_fw.clone(), "discovery_after_err", "Unknown".to_string())
                        }
                    }
                } else {
                    log::warn!(
                        "[jlink] probe_open_details failed index={} sn={} — {} (using discovery if present)",
                        index,
                        serial,
                        msg
                    );
                    (discovery_fw.clone(), "discovery_after_err", "Unknown".to_string())
                }
            }
        };

        log::debug!(
            "[jlink] probe[{}] sn={} fw_source={} read_ms={:.1}",
            index,
            serial,
            fw_src,
            t0.elapsed().as_secs_f64() * 1000.0
        );

        probes.push(Probe {
            id: serial.clone(),
            serial_number: serial,
            product_name: row["productName"].as_str().unwrap_or("").to_string(),
            nick_name: row["nickName"].as_str().unwrap_or("").to_string(),
            provider: ProbeProvider::JLink,
            connection: row["connection"].as_str().unwrap_or("").to_string(),
            driver: driver_label,
            firmware,
        });
    }

    log::debug!("[jlink] scan_probes: enumerated {} row(s)", probes.len());
    log_probes_summary("bridge", &probes);
    Ok(probes)
}

fn update_firmware_via_bridge(probe_index: usize) -> FirmwareUpdateResult {
    log::info!(
        "[jlink] Updating firmware for probe[{}] via native J-Link bridge...",
        probe_index
    );

    match bridge::update_firmware_json(probe_index) {
        Ok(raw) => parse_firmware_update_response(probe_index, &raw),
        Err(e) => {
            log::warn!("[jlink][bridge] update_firmware_json failed: {}", e);
            FirmwareUpdateResult::Failed {
                error: e.to_string(),
            }
        }
    }
}

/// Pure JSON-to-domain mapper for the bridge's `update_firmware` response.
///
/// Separated from the bridge call so it can be unit-tested with canned payloads.
fn parse_firmware_update_response(probe_index: usize, raw: &str) -> FirmwareUpdateResult {
    let v: serde_json::Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(e) => {
            log::warn!(
                "[jlink][bridge] firmware update response is not valid JSON: {}",
                e
            );
            return FirmwareUpdateResult::Failed {
                error: e.to_string(),
            };
        }
    };

    let reboot_attempted = v["rebootAttempted"].as_bool().unwrap_or(false);
    let reboot_not_supported = v["rebootNotSupported"].as_bool().unwrap_or(false);
    let reboot_command = v["rebootCommand"].as_str().unwrap_or("");
    let sleep_ms = v["sleepMs"].as_u64().unwrap_or(0);

    if let Some(detail) = v["detail"].as_str() {
        if !detail.trim().is_empty() {
            log::debug!("[jlink][bridge] firmware update detail:\n{}", detail);
        }
    }

    let status = v["status"].as_str().unwrap_or("");
    let fw = v["firmware"].as_str().unwrap_or("").to_string();

    match status {
        "failed" => {
            let msg = v["error"]
                .as_str()
                .unwrap_or("firmware update failed")
                .to_string();
            log::warn!(
                "[jlink][bridge] firmware update reported failed (probe[{}]): {}",
                probe_index,
                msg
            );
            FirmwareUpdateResult::Failed { error: msg }
        }
        "updated" => {
            log::info!(
                "[jlink] Probe[{}] firmware updated; post-update sleep={}ms reboot_attempted={} reboot_cmd={} reboot_not_supported={}",
                probe_index,
                sleep_ms,
                reboot_attempted,
                if reboot_command.is_empty() { "(none)" } else { reboot_command },
                reboot_not_supported
            );
            FirmwareUpdateResult::Updated { firmware: fw }
        }
        _ => {
            log::info!(
                "[jlink] Probe[{}] firmware current; post-update sleep={}ms reboot_attempted={} (skipped)",
                probe_index,
                sleep_ms,
                reboot_attempted
            );
            FirmwareUpdateResult::Current {
                firmware: if fw.is_empty() { "n/a".to_string() } else { fw },
            }
        }
    }
}

fn switch_usb_via_bridge(probe_index: usize, mode: UsbDriverMode) -> UsbDriverResult {
    let winusb = matches!(mode, UsbDriverMode::WinUsb);
    match bridge::switch_usb_json(probe_index, winusb) {
        Ok(raw) => parse_switch_usb_response(probe_index, &raw),
        Err(e) => {
            log::warn!("[jlink][bridge] switch_usb_json failed: {}", e);
            UsbDriverResult {
                success: false,
                error: Some(format!(
                    "{}\n\n(native detail)\n{}",
                    e,
                    bridge::last_error()
                )),
                detail: Some(bridge::last_error()),
                reboot_not_supported: false,
            }
        }
    }
}

/// Pure JSON-to-domain mapper for the bridge's `switch_usb_driver` response.
///
/// Separated from the bridge call so it can be unit-tested with canned payloads.
fn parse_switch_usb_response(probe_index: usize, raw: &str) -> UsbDriverResult {
    let v: serde_json::Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(e) => {
            log::warn!(
                "[jlink][bridge] switch_usb response is not valid JSON: {}",
                e
            );
            return UsbDriverResult {
                success: false,
                error: Some(format!(
                    "{}\n\n(native detail)\n{}",
                    e,
                    bridge::last_error()
                )),
                detail: Some(bridge::last_error()),
                reboot_not_supported: false,
            };
        }
    };

    let success = v["success"].as_bool().unwrap_or(false);
    let reboot_attempted = v["rebootAttempted"].as_bool().unwrap_or(false);
    let reboot_not_supported = v["rebootNotSupported"].as_bool().unwrap_or(false);
    let reboot_command = v["rebootCommand"].as_str().unwrap_or("");

    if !success {
        let detail = v["detail"].as_str().unwrap_or("");
        if !detail.trim().is_empty() {
            log::warn!("[jlink][bridge] switch failed. detail:\n{}", detail);
        } else {
            log::warn!(
                "[jlink][bridge] switch failed. detail empty; lastNativeError:\n{}",
                bridge::last_error()
            );
        }
    } else {
        let cmd = if reboot_command.is_empty() {
            "(none)"
        } else {
            reboot_command
        };
        log::info!(
            "[jlink] switch_usb_driver probe[{}] ok; post-switch: Sleep(100) -> Reboot({}) -> Sleep(100) (attempted={}, not_supported={})",
            probe_index,
            cmd,
            reboot_attempted,
            reboot_not_supported
        );
    }

    UsbDriverResult {
        success,
        error: v["error"]
            .as_str()
            .filter(|e| !e.is_empty())
            .map(|e| e.to_string()),
        detail: v["detail"]
            .as_str()
            .filter(|e| !e.is_empty())
            .map(|e| e.to_string()),
        reboot_not_supported,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        parse_discovery_firmware_string, parse_firmware_update_response,
        parse_switch_usb_response, JLinkService,
    };
    use crate::domain::jlink::types::FirmwareUpdateResult;

    // ─── Version + firmware string helpers ───────────────────────────────

    #[test]
    fn format_version_for_ui_parses_build_numbers() {
        assert_eq!(
            JLinkService::format_version_for_ui("SEGGER J-Link DLL (build 93600)").as_deref(),
            Some("V9.36")
        );
        assert_eq!(
            JLinkService::format_version_for_ui("SEGGER J-Link DLL (build 105200)").as_deref(),
            Some("V10.52")
        );
        assert_eq!(
            JLinkService::format_version_for_ui("SEGGER J-Link DLL (build 105201)").as_deref(),
            Some("V10.52.1")
        );
    }

    #[test]
    fn parse_discovery_firmware_string_extracts_compiled_date() {
        let s = "J-Link OB-STM32F072-128K V2 compiled Sep 29 2020 12:34:56\r\n";
        assert_eq!(
            parse_discovery_firmware_string(s),
            "Sep 29 2020 12:34:56".to_string()
        );
    }

    #[test]
    fn parse_discovery_firmware_string_passes_through_when_no_compiled_marker() {
        assert_eq!(
            parse_discovery_firmware_string("FW 1.2.3"),
            "FW 1.2.3".to_string()
        );
        assert_eq!(parse_discovery_firmware_string("   "), "".to_string());
    }

    // ─── Firmware update response parser ─────────────────────────────────

    #[test]
    fn firmware_update_response_updated_status_returns_updated() {
        let raw = r#"{
            "status": "updated",
            "firmware": "Sep 29 2020",
            "detail": "",
            "rebootAttempted": true,
            "rebootNotSupported": false,
            "rebootCommand": "rnh",
            "sleepMs": 100
        }"#;
        match parse_firmware_update_response(0, raw) {
            FirmwareUpdateResult::Updated { firmware } => {
                assert_eq!(firmware, "Sep 29 2020");
            }
            other => panic!("expected Updated, got {:?}", other),
        }
    }

    #[test]
    fn firmware_update_response_current_status_uses_fallback_when_firmware_missing() {
        let raw = r#"{
            "status": "current",
            "firmware": "",
            "rebootAttempted": false,
            "sleepMs": 0
        }"#;
        match parse_firmware_update_response(1, raw) {
            FirmwareUpdateResult::Current { firmware } => {
                assert_eq!(firmware, "n/a");
            }
            other => panic!("expected Current, got {:?}", other),
        }
    }

    #[test]
    fn firmware_update_response_failed_status_surfaces_error() {
        let raw = r#"{
            "status": "failed",
            "firmware": "",
            "error": "OpenEx timeout"
        }"#;
        match parse_firmware_update_response(2, raw) {
            FirmwareUpdateResult::Failed { error } => {
                assert_eq!(error, "OpenEx timeout");
            }
            other => panic!("expected Failed, got {:?}", other),
        }
    }

    #[test]
    fn firmware_update_response_invalid_json_reports_failed() {
        match parse_firmware_update_response(0, "not json") {
            FirmwareUpdateResult::Failed { error } => {
                assert!(!error.is_empty());
            }
            other => panic!("expected Failed, got {:?}", other),
        }
    }

    // ─── Switch USB response parser ──────────────────────────────────────

    #[test]
    fn switch_usb_response_success_normalizes_empty_strings_to_none() {
        let raw = r#"{
            "success": true,
            "error": "",
            "detail": "",
            "rebootAttempted": true,
            "rebootNotSupported": false,
            "rebootCommand": "rnh",
            "sleepMs": 100
        }"#;
        let r = parse_switch_usb_response(0, raw);
        assert!(r.success);
        assert!(r.error.is_none());
        assert!(r.detail.is_none());
        assert!(!r.reboot_not_supported);
    }

    #[test]
    fn switch_usb_response_failure_preserves_detail_and_flags() {
        let raw = r#"{
            "success": false,
            "error": "Failed to switch probe config.",
            "detail": "could not write config word",
            "rebootNotSupported": true
        }"#;
        let r = parse_switch_usb_response(0, raw);
        assert!(!r.success);
        assert_eq!(r.error.as_deref(), Some("Failed to switch probe config."));
        assert_eq!(r.detail.as_deref(), Some("could not write config word"));
        assert!(r.reboot_not_supported);
    }

    #[test]
    fn switch_usb_response_invalid_json_returns_failure_with_error_context() {
        let r = parse_switch_usb_response(0, "not json");
        assert!(!r.success);
        assert!(r.error.is_some());
    }
}

