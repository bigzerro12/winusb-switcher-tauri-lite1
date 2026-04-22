//! Domain service for J-Link operations.
//!
//! Tauri commands should use `crate::domain::probe`; this module is the J-Link backend
//! implementation behind `ProbeBackend`.

use crate::domain::jlink::types::{
    FirmwareUpdateResult, InstallStatus, Probe, ProbeProvider, UsbDriverMode, UsbDriverResult,
};
use crate::domain::probe::ProbeBackend;
use crate::error::{AppError, AppResult, BridgeError};
use crate::jlink_ffi::bridge;
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

    fn detect(runtime: Option<&JLinkRuntime>) -> InstallStatus {
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
            return InstallStatus {
                installed: true,
                path: Some(rt.native_lib_path.to_string_lossy().into_owned()),
                version: ui_version,
            };
        }
        log::debug!(
            "[jlink] detect: runtime not prepared yet (prepare_bundled_jlink not completed)"
        );
        InstallStatus {
            installed: false,
            path: None,
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

    fn update_firmware_only(_rt: &JLinkRuntime, probe_index: usize) -> AppResult<FirmwareUpdateResult> {
        log::debug!("[jlink] update_firmware_only: probe[{}]", probe_index);
        Self::ensure_bridge_loaded()?;
        Ok(update_firmware_via_bridge(probe_index))
    }
}

// Backend abstraction implementation (enables future probe families without scattering branching logic).
impl ProbeBackend for JLinkService {
    type Runtime = JLinkRuntime;

    fn diagnostics_json(runtime: Option<&Self::Runtime>) -> serde_json::Value {
        Self::diagnostics_json(runtime)
    }

    fn detect(runtime: Option<&Self::Runtime>) -> InstallStatus {
        Self::detect(runtime)
    }

    fn ensure_ready(runtime: Option<&Self::Runtime>) -> AppResult<&Self::Runtime> {
        Self::ensure_ready(runtime)
    }

    fn scan_probes(rt: &Self::Runtime) -> AppResult<Vec<Probe>> {
        Self::scan_probes(rt)
    }

    fn update_firmware_only(rt: &Self::Runtime, probe_index: usize) -> AppResult<FirmwareUpdateResult> {
        Self::update_firmware_only(rt, probe_index)
    }

    fn switch_usb_driver(rt: &Self::Runtime, probe_index: usize, mode: UsbDriverMode) -> AppResult<UsbDriverResult> {
        Self::switch_usb_driver(rt, probe_index, mode)
    }
}

impl JLinkService {
    /// Resolve the current bridge index for a probe by serial number.
    ///
    /// Indices can reorder during reboot/re-enumeration; serial numbers remain stable.
    pub fn resolve_probe_index_by_serial(serial_number: &str) -> AppResult<usize> {
        use serde_json::Value;

        let json = bridge::list_probes_json().map_err(AppError::from)?;
        let rows: Vec<Value> =
            serde_json::from_str(&json).map_err(|e| AppError::Internal(e.to_string()))?;

        for row in rows {
            let sn = row["serialNumber"].as_str().unwrap_or("");
            if sn == serial_number {
                return Ok(row["index"].as_u64().unwrap_or(0) as usize);
            }
        }

        Err(AppError::Bridge(format!(
            "Probe not found (serialNumber={})",
            serial_number
        )))
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

        let try_read_fw = || -> Result<String, BridgeError> { bridge::probe_firmware(index) };

        let (firmware, fw_src): (Option<String>, &'static str) = match try_read_fw() {
            Ok(s) if !s.is_empty() => (Some(s), "bridge_openex"),
            Ok(_) => (discovery_fw.clone(), "discovery_only"),
            Err(e) => {
                // On cold start (or right after udev install), OpenEx may fail transiently.
                // Retry once quickly to avoid "firmware missing until refresh".
                let msg = e.to_string();
                let transient = msg.contains("Cannot connect")
                    || msg.contains("Could not read J-Link capabilities")
                    || msg.contains("Communication timed out");

                if transient {
                    log::debug!(
                        "[jlink] probe_firmware transient OpenEx error index={} sn={} — {} (retrying once)",
                        index,
                        serial,
                        msg
                    );
                    std::thread::sleep(std::time::Duration::from_millis(250));
                    match try_read_fw() {
                        Ok(s) if !s.is_empty() => (Some(s), "bridge_openex_retry"),
                        Ok(_) => (discovery_fw.clone(), "discovery_only_retry"),
                        Err(e2) => {
                            log::warn!(
                                "[jlink] probe_firmware failed after retry index={} sn={} — {} (using discovery if present)",
                                index,
                                serial,
                                e2
                            );
                            (discovery_fw.clone(), "discovery_after_err")
                        }
                    }
                } else {
                    log::warn!(
                        "[jlink] probe_firmware failed index={} sn={} — {} (using discovery if present)",
                        index,
                        serial,
                        msg
                    );
                    (discovery_fw.clone(), "discovery_after_err")
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
            driver: "Unknown".to_string(),
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
        Ok(s) => match serde_json::from_str::<serde_json::Value>(&s) {
            Ok(v) => {
                let reboot_attempted = v["rebootAttempted"].as_bool().unwrap_or(false);
                let reboot_not_supported = v["rebootNotSupported"].as_bool().unwrap_or(false);
                let reboot_command = v["rebootCommand"].as_str().unwrap_or("");
                let sleep_ms = v["sleepMs"].as_u64().unwrap_or(0);

                if let Some(detail) = v["detail"].as_str() {
                    if !detail.trim().is_empty() {
                        // Can be multi-line and noisy; keep at debug by default.
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
            Err(e) => {
                log::warn!(
                    "[jlink][bridge] firmware update response is not valid JSON: {}",
                    e
                );
                FirmwareUpdateResult::Failed {
                    error: e.to_string(),
                }
            }
        },
        Err(e) => {
            log::warn!("[jlink][bridge] update_firmware_json failed: {}", e);
            FirmwareUpdateResult::Failed {
                error: e.to_string(),
            }
        },
    }
}

fn switch_usb_via_bridge(probe_index: usize, mode: UsbDriverMode) -> UsbDriverResult {
    let winusb = matches!(mode, UsbDriverMode::WinUsb);
    match bridge::switch_usb_json(probe_index, winusb) {
        Ok(s) => match serde_json::from_str::<serde_json::Value>(&s) {
            Ok(v) => {
                let success = v["success"].as_bool().unwrap_or(false);
                let reboot_attempted = v["rebootAttempted"].as_bool().unwrap_or(false);
                let reboot_not_supported = v["rebootNotSupported"].as_bool().unwrap_or(false);
                let reboot_command = v["rebootCommand"].as_str().unwrap_or("");
                let _sleep_ms = v["sleepMs"].as_u64().unwrap_or(0);

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
            Err(e) => {
                log::warn!(
                    "[jlink][bridge] switch_usb response is not valid JSON: {}",
                    e
                );
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
        },
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
        },
    }
}

