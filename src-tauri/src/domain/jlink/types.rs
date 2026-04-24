//! Domain DTOs exposed to the frontend.
//!
//! Kept stable intentionally: UI and command responses depend on this shape.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "PascalCase")]
pub enum ProbeProvider {
    #[default]
    JLink,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Probe {
    pub id: String,
    #[serde(rename = "serialNumber")]
    pub serial_number: String,
    #[serde(rename = "productName")]
    pub product_name: String,
    #[serde(rename = "nickName")]
    pub nick_name: String,
    pub provider: ProbeProvider,
    pub connection: String,
    pub driver: String,
    pub firmware: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStatus {
    pub ready: bool,
    #[serde(rename = "nativeLibPath")]
    pub native_lib_path: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum FirmwareUpdateResult {
    Updated { firmware: String },
    Current { firmware: String },
    Failed { error: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsbDriverResult {
    pub success: bool,
    pub error: Option<String>,
    #[serde(default)]
    pub detail: Option<String>,
    /// True when reboot command returned "Command not supported by connected probe."
    #[serde(default)]
    pub reboot_not_supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum UsbDriverMode {
    WinUsb,
    Segger,
}

