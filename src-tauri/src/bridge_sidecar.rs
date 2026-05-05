#[cfg(any(target_os = "windows", target_os = "linux"))]
use std::io::{BufRead, BufReader, BufWriter, Write};
#[cfg(any(target_os = "windows", target_os = "linux"))]
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
#[cfg(any(target_os = "windows", target_os = "linux"))]
use std::sync::mpsc::{self, RecvTimeoutError};
#[cfg(any(target_os = "windows", target_os = "linux"))]
use std::sync::{Mutex, OnceLock};
#[cfg(any(target_os = "windows", target_os = "linux"))]
use std::time::Duration;

#[cfg(any(target_os = "windows", target_os = "linux"))]
const SIDECAR_CALL_TIMEOUT: Duration = Duration::from_secs(8);
#[cfg(any(target_os = "windows", target_os = "linux"))]
const SIDECAR_LONG_CALL_TIMEOUT: Duration = Duration::from_secs(30);

#[cfg(any(target_os = "windows", target_os = "linux"))]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct RpcRequest {
    op: String,
    args: serde_json::Value,
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
mod op {
    pub const LOAD: &str = "load";
    pub const IS_LOADED: &str = "is_loaded";
    pub const LAST_ERROR: &str = "last_error";
    pub const DLL_VERSION: &str = "dll_version";
    pub const LIST_PROBES_JSON: &str = "list_probes_json";
    pub const PROBE_OPEN_DETAILS: &str = "probe_open_details";
    pub const UPDATE_FIRMWARE_JSON: &str = "update_firmware_json";
    pub const UPDATE_FIRMWARE_JSON_BY_SN: &str = "update_firmware_json_by_sn";
    pub const SWITCH_USB_JSON: &str = "switch_usb_json";
    pub const SWITCH_USB_JSON_BY_SN: &str = "switch_usb_json_by_sn";
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
#[derive(serde::Serialize, serde::Deserialize)]
struct RpcResponse {
    ok: bool,
    #[serde(default)]
    data: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<String>,
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
struct SidecarProcess {
    _child: Child,
    stdin: BufWriter<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
impl SidecarProcess {
    fn spawn() -> Result<Self, String> {
        let exe = std::env::current_exe().map_err(|e| format!("current_exe: {}", e))?;
        let mut cmd = Command::new(exe);
        log::info!("[sidecar] spawning bridge sidecar process");
        cmd.arg("--jlink-sidecar")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        let mut child = cmd.spawn().map_err(|e| format!("spawn sidecar: {}", e))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "spawn sidecar: stdin not piped".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "spawn sidecar: stdout not piped".to_string())?;
        Ok(Self {
            _child: child,
            stdin: BufWriter::new(stdin),
            stdout: BufReader::new(stdout),
        })
    }

    fn call(&mut self, req: &RpcRequest) -> Result<serde_json::Value, String> {
        let mut line = serde_json::to_string(req).map_err(|e| format!("serialize req: {}", e))?;
        line.push('\n');
        self.stdin
            .write_all(line.as_bytes())
            .map_err(|e| format!("write req: {}", e))?;
        self.stdin
            .flush()
            .map_err(|e| format!("flush req: {}", e))?;

        let mut resp_line = String::new();
        let n = self
            .stdout
            .read_line(&mut resp_line)
            .map_err(|e| format!("read resp: {}", e))?;
        if n == 0 {
            return Err("sidecar closed stdout".to_string());
        }
        let resp: RpcResponse =
            serde_json::from_str(resp_line.trim_end()).map_err(|e| format!("parse resp: {}", e))?;
        if resp.ok {
            Ok(resp.data.unwrap_or(serde_json::Value::Null))
        } else {
            Err(resp
                .error
                .unwrap_or_else(|| "sidecar op failed".to_string()))
        }
    }

    fn pid(&self) -> u32 {
        self._child.id()
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn sidecar_slot() -> &'static Mutex<Option<SidecarProcess>> {
    // Keep one sidecar process per app process to avoid repeated spawn overhead.
    static SLOT: OnceLock<Mutex<Option<SidecarProcess>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn loaded_lib_path_slot() -> &'static Mutex<Option<String>> {
    static SLOT: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn remember_loaded_lib_path(path: &str) {
    if let Ok(mut slot) = loaded_lib_path_slot().lock() {
        *slot = Some(path.to_string());
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn last_loaded_lib_path() -> Option<String> {
    loaded_lib_path_slot()
        .lock()
        .ok()
        .and_then(|slot| slot.as_ref().cloned())
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn maybe_preload_sidecar(proc_: &mut SidecarProcess, current_op: &str) -> Result<(), String> {
    if current_op == op::LOAD {
        return Ok(());
    }
    let Some(path) = last_loaded_lib_path() else {
        return Ok(());
    };
    log::debug!(
        "[sidecar] preloading bridge library after respawn: {}",
        path
    );
    let preload_req = RpcRequest {
        op: op::LOAD.to_string(),
        args: serde_json::json!({ "path": path }),
    };
    proc_.call(&preload_req).map(|_| ())
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn timeout_for_op(current_op: &str) -> Duration {
    match current_op {
        op::UPDATE_FIRMWARE_JSON
        | op::UPDATE_FIRMWARE_JSON_BY_SN
        | op::SWITCH_USB_JSON
        | op::SWITCH_USB_JSON_BY_SN => SIDECAR_LONG_CALL_TIMEOUT,
        _ => SIDECAR_CALL_TIMEOUT,
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn call(op: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
    let req = RpcRequest {
        op: op.to_string(),
        args,
    };

    // Retry once after a respawn if the child exits mid-request.
    for attempt in 0..2 {
        let proc_ = {
            let mut slot = sidecar_slot()
                .lock()
                .map_err(|_| "sidecar mutex poisoned".to_string())?;
            if slot.is_none() {
                let mut spawned = SidecarProcess::spawn()?;
                maybe_preload_sidecar(&mut spawned, op)?;
                *slot = Some(spawned);
            }
            slot.take()
                .ok_or_else(|| "sidecar unavailable".to_string())?
        };

        let pid = proc_.pid();
        let timeout = timeout_for_op(op);
        match call_with_timeout(proc_, req.clone(), timeout) {
            Ok((returned_proc, Ok(v))) => {
                let mut slot = sidecar_slot()
                    .lock()
                    .map_err(|_| "sidecar mutex poisoned".to_string())?;
                *slot = Some(returned_proc);
                if op == op::LOAD {
                    if let Some(path) = req.args["path"].as_str() {
                        remember_loaded_lib_path(path);
                    }
                }
                log::trace!("[sidecar] op={} attempt={} ok", op, attempt + 1);
                return Ok(v);
            }
            Ok((_, Err(e))) => {
                log::warn!(
                    "[sidecar] op={} attempt={} failed: {} (respawning)",
                    op,
                    attempt + 1,
                    e
                );
                let mut slot = sidecar_slot()
                    .lock()
                    .map_err(|_| "sidecar mutex poisoned".to_string())?;
                *slot = None;
                if attempt == 1 {
                    return Err(e);
                }
            }
            Err(e) => {
                log::warn!(
                    "[sidecar] op={} attempt={} timeout/channel error: {} (killing pid={} and respawning)",
                    op,
                    attempt + 1,
                    e,
                    pid
                );
                kill_sidecar_process(pid);
                let mut slot = sidecar_slot()
                    .lock()
                    .map_err(|_| "sidecar mutex poisoned".to_string())?;
                *slot = None;
                if attempt == 1 {
                    return Err(e);
                }
            }
        }
    }
    Err("sidecar unavailable".to_string())
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn call_with_timeout(
    mut proc_: SidecarProcess,
    req: RpcRequest,
    timeout: Duration,
) -> Result<(SidecarProcess, Result<serde_json::Value, String>), String> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = proc_.call(&req);
        let _ = tx.send((proc_, result));
    });

    match rx.recv_timeout(timeout) {
        Ok(v) => Ok(v),
        Err(RecvTimeoutError::Timeout) => Err(format!(
            "sidecar call timed out after {}ms",
            timeout.as_millis()
        )),
        Err(RecvTimeoutError::Disconnected) => Err("sidecar worker disconnected".to_string()),
    }
}

#[cfg(all(any(target_os = "windows", target_os = "linux"), target_os = "windows"))]
fn kill_sidecar_process(pid: u32) {
    let status = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match status {
        Ok(s) if s.success() => {
            log::debug!("[sidecar] taskkill pid={} succeeded", pid);
        }
        Ok(s) => {
            log::debug!(
                "[sidecar] taskkill pid={} non-success exit={:?} (process may have already exited)",
                pid,
                s.code()
            );
        }
        Err(e) => {
            log::debug!(
                "[sidecar] taskkill pid={} failed to execute: {} (continuing)",
                pid,
                e
            );
        }
    }
}

#[cfg(all(any(target_os = "windows", target_os = "linux"), target_os = "linux"))]
fn kill_sidecar_process(pid: u32) {
    let status = Command::new("kill")
        .args(["-9", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match status {
        Ok(s) if s.success() => {
            log::debug!("[sidecar] kill -9 pid={} succeeded", pid);
        }
        Ok(s) => {
            log::debug!(
                "[sidecar] kill -9 pid={} non-success exit={:?} (process may have already exited)",
                pid,
                s.code()
            );
        }
        Err(e) => {
            log::debug!(
                "[sidecar] kill -9 pid={} failed to execute: {} (continuing)",
                pid,
                e
            );
        }
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn respond<W: Write>(writer: &mut W, response: RpcResponse) {
    let line = match serde_json::to_string(&response) {
        Ok(s) => s,
        Err(e) => {
            let fallback = RpcResponse {
                ok: false,
                data: None,
                error: Some(format!("serialize response: {}", e)),
            };
            serde_json::to_string(&fallback)
                .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"serialize response\"}".to_string())
        }
    };
    let _ = writer.write_all(line.as_bytes());
    let _ = writer.write_all(b"\n");
    let _ = writer.flush();
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn arg_str(args: &serde_json::Value, key: &str) -> Result<String, String> {
    args[key]
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("missing {}", key))
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn arg_u64(args: &serde_json::Value, key: &str) -> Result<u64, String> {
    args[key].as_u64().ok_or_else(|| format!("missing {}", key))
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn arg_i32_or(args: &serde_json::Value, key: &str, default: i32) -> i32 {
    args[key].as_i64().unwrap_or(default as i64) as i32
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn arg_u32_or(args: &serde_json::Value, key: &str, default: u32) -> u32 {
    args[key].as_u64().unwrap_or(default as u64) as u32
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn arg_bool_or(args: &serde_json::Value, key: &str, default: bool) -> bool {
    args[key].as_bool().unwrap_or(default)
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn handle_request(req: RpcRequest) -> Result<serde_json::Value, String> {
    match req.op.as_str() {
        op::LOAD => {
            let p = arg_str(&req.args, "path")?;
            crate::jlink_ffi::bridge_load(std::path::Path::new(&p))?;
            Ok(serde_json::Value::Bool(true))
        }
        op::IS_LOADED => Ok(serde_json::Value::Bool(crate::jlink_ffi::bridge_is_loaded())),
        op::LAST_ERROR => Ok(serde_json::Value::String(
            crate::jlink_ffi::last_native_error(),
        )),
        op::DLL_VERSION => Ok(crate::jlink_ffi::dll_version_string()
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null)),
        op::LIST_PROBES_JSON => Ok(serde_json::Value::String(
            crate::jlink_ffi::list_probes_json()?,
        )),
        op::PROBE_OPEN_DETAILS => {
            let index = arg_u64(&req.args, "index")? as usize;
            let d = crate::jlink_ffi::probe_open_details(index)?;
            Ok(serde_json::json!({
                "firmware": d.firmware,
                "usbDriver": d.usb_driver,
            }))
        }
        op::UPDATE_FIRMWARE_JSON => {
            let index = arg_u64(&req.args, "index")? as usize;
            Ok(serde_json::Value::String(
                crate::jlink_ffi::update_firmware_json(index)?,
            ))
        }
        op::UPDATE_FIRMWARE_JSON_BY_SN => {
            let serial_number = arg_u64(&req.args, "serialNumber")? as u32;
            let retries = arg_i32_or(&req.args, "retries", 0);
            let retry_delay_ms = arg_u32_or(&req.args, "retryDelayMs", 0);
            Ok(serde_json::Value::String(
                crate::jlink_ffi::update_firmware_json_by_sn(
                    serial_number,
                    retries,
                    retry_delay_ms,
                )?,
            ))
        }
        op::SWITCH_USB_JSON => {
            let index = arg_u64(&req.args, "index")? as usize;
            let winusb = arg_bool_or(&req.args, "winusb", false);
            Ok(serde_json::Value::String(
                crate::jlink_ffi::switch_usb_json(index, winusb)?,
            ))
        }
        op::SWITCH_USB_JSON_BY_SN => {
            let serial_number = arg_u64(&req.args, "serialNumber")? as u32;
            let winusb = arg_bool_or(&req.args, "winusb", false);
            let retries = arg_i32_or(&req.args, "retries", 0);
            let retry_delay_ms = arg_u32_or(&req.args, "retryDelayMs", 0);
            Ok(serde_json::Value::String(
                crate::jlink_ffi::switch_usb_json_by_sn(
                    serial_number,
                    winusb,
                    retries,
                    retry_delay_ms,
                )?,
            ))
        }
        _ => Err(format!("unknown op: {}", req.op)),
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn run_stdio_sidecar() -> i32 {
    log::info!("[sidecar] stdio sidecar started");
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = BufWriter::new(stdout.lock());

    loop {
        let mut line = String::new();
        let n = match reader.read_line(&mut line) {
            Ok(n) => n,
            Err(_) => return 1,
        };
        if n == 0 {
            return 0;
        }

        // The parent sends one JSON request per line.
        let req: RpcRequest = match serde_json::from_str(line.trim_end()) {
            Ok(v) => v,
            Err(e) => {
                log::warn!("[sidecar] invalid request: {}", e);
                respond(
                    &mut writer,
                    RpcResponse {
                        ok: false,
                        data: None,
                        error: Some(format!("invalid request: {}", e)),
                    },
                );
                continue;
            }
        };

        let op_name = req.op.clone();
        match handle_request(req) {
            Ok(v) => respond(
                &mut writer,
                RpcResponse {
                    ok: true,
                    data: Some(v),
                    error: None,
                },
            ),
            Err(e) => {
                log::warn!("[sidecar] op={} failed: {}", op_name, e);
                respond(
                    &mut writer,
                    RpcResponse {
                        ok: false,
                        data: None,
                        error: Some(e),
                    },
                )
            }
        }
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn call(_op: &str, _args: serde_json::Value) -> Result<serde_json::Value, String> {
    Err("sidecar bridge is only available on Windows and Linux".to_string())
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn run_stdio_sidecar() -> i32 {
    1
}
