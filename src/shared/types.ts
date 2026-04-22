// ─── Probe Types ──────────────────────────────────────────────────────────────

export type DriverType = "SEGGER" | "WinUSB" | "Unknown";
export type ProviderType = "JLink";

export type Probe = {
  id: string;
  serialNumber: string;
  productName: string;
  nickName: string;
  provider: ProviderType;
  connection: string;
  driver: DriverType;
  firmware?: string;
};

export type InstallStatus = {
  installed: boolean;
  path?: string;
  version?: string;
};

// ─── Error Types ──────────────────────────────────────────────────────────────
// Matches AppError enum in Rust (src-tauri/src/error.rs) — serde tag "kind", content "message"

export type AppErrorKind =
  | "runtime"
  | "bridge"
  | "io"
  | "internal";

export type AppError = {
  kind: AppErrorKind;
  message: string;
};

export function isAppError(e: unknown): e is AppError {
  return typeof e === "object" && e !== null && "kind" in e;
}

// ─── Diagnostics (get_jlink_diagnostics) ─────────────────────────────────────

/** Snapshot from `get_jlink_diagnostics` for support. */
export type JLinkDiagnostics = {
  runtimePrepared: boolean;
  bridgeLoaded: boolean;
  runtimeDir?: string;
  nativeLibPath?: string;
  versionRaw?: string | null;
  versionUi?: string | null;
  firmwaresDir?: string;
  firmwaresDirExists?: boolean;
  winusbJlinkDllPathEnv?: string | null;
  targetOs: string;
  targetArch: string;
};

// ─── Result Types ─────────────────────────────────────────────────────────────

export type InstallResult = {
  success: boolean;
  cancelled?: boolean;
  message: string;
  path?: string;
};

export type UsbDriverMode = "winUsb" | "segger";

export type UsbDriverResult = {
  success: boolean;
  error?: string;
  detail?: string;
  /** When true, reboot is not available on probe firmware — omit "may reboot briefly" in UI */
  rebootNotSupported?: boolean;
};

/** Returned by the `get_arch_info` command. Values match `std::env::consts` on the Rust side. */
export type ArchInfo = {
  /** e.g. "windows" | "macos" | "linux" */
  os: string;
  /** e.g. "x86_64" | "x86" | "aarch64" | "arm" */
  arch: string;
};

// ─── Tauri Command Names ──────────────────────────────────────────────────────

export const COMMANDS = {
  /** Lite only: load bundled J-Link runtime before scanning */
  PREPARE_BUNDLED_JLINK: "prepare_bundled_jlink",
  DETECT_AND_SCAN: "detect_and_scan",
  SCAN_PROBES: "scan_probes",
  /** Payload: `{ probeIndex, mode, provider? }` */
  SWITCH_USB_DRIVER: "switch_usb_driver",
  GET_ARCH_INFO: "get_arch_info",
  GET_JLINK_DIAGNOSTICS: "get_jlink_diagnostics",
} as const;
