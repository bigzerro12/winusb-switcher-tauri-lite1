#[cfg(any(target_os = "windows", target_os = "linux"))]
use std::io::{BufRead, BufReader, BufWriter, Write};
#[cfg(any(target_os = "windows", target_os = "linux"))]
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
#[cfg(any(target_os = "windows", target_os = "linux"))]
use std::sync::{Mutex, OnceLock};

#[cfg(any(target_os = "windows", target_os = "linux"))]
#[derive(serde::Serialize, serde::Deserialize)]
struct RpcRequest {
    op: String,
    args: serde_json::Value,
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
        self.stdin.flush().map_err(|e| format!("flush req: {}", e))?;

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
            Err(resp.error.unwrap_or_else(|| "sidecar op failed".to_string()))
        }
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn sidecar_slot() -> &'static Mutex<Option<SidecarProcess>> {
    static SLOT: OnceLock<Mutex<Option<SidecarProcess>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn call(op: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
    let req = RpcRequest {
        op: op.to_string(),
        args,
    };

    // Retry once by respawning if the sidecar died/crashed.
    for attempt in 0..2 {
        let mut slot = sidecar_slot()
            .lock()
            .map_err(|_| "sidecar mutex poisoned".to_string())?;
        if slot.is_none() {
            *slot = Some(SidecarProcess::spawn()?);
        }
        if let Some(proc_) = slot.as_mut() {
            match proc_.call(&req) {
                Ok(v) => {
                    log::trace!("[sidecar] op={} attempt={} ok", op, attempt + 1);
                    return Ok(v);
                }
                Err(e) => {
                    log::warn!(
                        "[sidecar] op={} attempt={} failed: {} (respawning)",
                        op,
                        attempt + 1,
                        e
                    );
                    *slot = None;
                    if attempt == 1 {
                        return Err(e);
                    }
                }
            }
        }
    }
    Err("sidecar unavailable".to_string())
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
            serde_json::to_string(&fallback).unwrap_or_else(|_| {
                "{\"ok\":false,\"error\":\"serialize response\"}".to_string()
            })
        }
    };
    let _ = writer.write_all(line.as_bytes());
    let _ = writer.write_all(b"\n");
    let _ = writer.flush();
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn handle_request(req: RpcRequest) -> Result<serde_json::Value, String> {
    match req.op.as_str() {
        "load" => {
            let p = req.args["path"]
                .as_str()
                .ok_or_else(|| "missing path".to_string())?;
            crate::jlink_ffi::bridge_load(std::path::Path::new(p))?;
            Ok(serde_json::Value::Bool(true))
        }
        "is_loaded" => Ok(serde_json::Value::Bool(crate::jlink_ffi::bridge_is_loaded())),
        "last_error" => Ok(serde_json::Value::String(crate::jlink_ffi::last_native_error())),
        "dll_version" => Ok(crate::jlink_ffi::dll_version_string()
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null)),
        "list_probes_json" => Ok(serde_json::Value::String(crate::jlink_ffi::list_probes_json()?)),
        "probe_open_details" => {
            let index = req.args["index"]
                .as_u64()
                .ok_or_else(|| "missing index".to_string())? as usize;
            let d = crate::jlink_ffi::probe_open_details(index)?;
            Ok(serde_json::json!({
                "firmware": d.firmware,
                "usbDriver": d.usb_driver,
            }))
        }
        "update_firmware_json" => {
            let index = req.args["index"]
                .as_u64()
                .ok_or_else(|| "missing index".to_string())? as usize;
            Ok(serde_json::Value::String(
                crate::jlink_ffi::update_firmware_json(index)?,
            ))
        }
        "update_firmware_json_by_sn" => {
            let serial_number = req.args["serialNumber"]
                .as_u64()
                .ok_or_else(|| "missing serialNumber".to_string())? as u32;
            let retries = req.args["retries"].as_i64().unwrap_or(0) as i32;
            let retry_delay_ms = req.args["retryDelayMs"].as_u64().unwrap_or(0) as u32;
            Ok(serde_json::Value::String(
                crate::jlink_ffi::update_firmware_json_by_sn(serial_number, retries, retry_delay_ms)?,
            ))
        }
        "switch_usb_json" => {
            let index = req.args["index"]
                .as_u64()
                .ok_or_else(|| "missing index".to_string())? as usize;
            let winusb = req.args["winusb"].as_bool().unwrap_or(false);
            Ok(serde_json::Value::String(crate::jlink_ffi::switch_usb_json(
                index, winusb,
            )?))
        }
        "switch_usb_json_by_sn" => {
            let serial_number = req.args["serialNumber"]
                .as_u64()
                .ok_or_else(|| "missing serialNumber".to_string())? as u32;
            let winusb = req.args["winusb"].as_bool().unwrap_or(false);
            let retries = req.args["retries"].as_i64().unwrap_or(0) as i32;
            let retry_delay_ms = req.args["retryDelayMs"].as_u64().unwrap_or(0) as u32;
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
