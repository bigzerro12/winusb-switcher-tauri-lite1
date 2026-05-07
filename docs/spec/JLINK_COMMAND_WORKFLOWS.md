# JLink Low-Level Command Workflows

## Purpose

Provide an implementation-aligned, low-level command view of JLink interactions for:

- app startup
- `Refresh list`
- WinUSB Switch Flow

This document is intentionally close to the current native implementation in:

- `src-tauri/native/jlink/commander_exec.cpp`
- `src-tauri/native/jlink/jlink_bridge_api.cpp`

---

## Command primitives used by current implementation

- Runtime Preparation load step: `jlink_bridge_load()` -> `LoadLibrary` / `dlopen`
- Probe list: `JLINKARM_EMU_GetList` (Commander-equivalent: `ShowEmuList`)
- Pre-open firmware policy toggles:
  - `exec DisableAutoUpdateFW` (scan/connect paths)
  - `exec EnableAutoUpdateFW` (switch/update paths)
- Probe select:
  - `JLINKARM_EMU_SelectByUSBSN` (preferred for USB)
  - fallback `JLINKARM_EMU_SelectByIndex`
- Connect/disconnect:
  - `JLINKARM_OpenEx`
  - `JLINKARM_Close`
- USB mode read:
  - `JLINKARM_ReadEmuConfigMem(&cfg, 0x8E, 1)` (bit 3 indicates SEGGER vs WinUSB mode)
- USB mode switch:
  - preferred: `exec WebUSBEnable` / `exec WebUSBDisable`
  - fallback: direct config-memory write via `JLINKARM_WriteEmuConfigMem`
- Reboot:
  - preferred `exec ScheduleReboot`
  - fallback `exec reboot`

---

## Backend call-chain mapping (Rust -> Native)

This section maps the exact backend call path for the three main user actions.

## 1) `detect_and_scan`

```mermaid
flowchart LR
  C["commands.rs::detect_and_scan"] --> P["domain/probe::detect_and_scan"]
  P --> S1["JLinkService::detect"]
  P --> S2["JLinkService::scan_probes"]
  S2 --> B1["Native Bridge::list_probes_json<br/>jlink_bridge_list_probes_json"]
  S2 --> B2["Native Bridge::probe_firmware_json<br/>jlink_bridge_probe_firmware"]
  B1 --> N1["_ExecShowEmuList"]
  B2 --> N2["_ConnectToJLinkCapture<br/>pre-open: DisableAutoUpdateFW"]
  B2 --> N3["JLINKARM_ReadEmuConfigMem(0x8E,1)"]
```

## 2) `scan_probes`

```mermaid
flowchart LR
  C["commands.rs::scan_probes"] --> E["probe::ensure_ready"]
  E --> P["probe::scan_probes"]
  P --> S["JLinkService::scan_probes"]
  S --> B["Native Bridge list/read helpers"]
  B --> N["_ExecShowEmuList + OpenEx path"]
```

## 3) `switch_usb_driver`

```mermaid
flowchart LR
  C["commands.rs::switch_usb_driver"] --> E["probe::ensure_ready"]
  E --> Q{"serialNumber provided?"}
  Q -->|yes| SS["JLinkService::switch_usb_driver_by_serial"]
  Q -->|no| P["probe::switch_usb"]
  P --> S["JLinkService::switch_usb_driver"]

  SS --> U["update_firmware_via_bridge_by_sn"]
  SS --> W["switch_usb_via_bridge_by_sn"]
  S --> U2["update_firmware_via_bridge(index)"]
  S --> W2["switch_usb_via_bridge(index, mode)"]

  W --> B["jlink_bridge_switch_usb_driver_by_sn"]
  W2 --> B2["jlink_bridge_switch_usb_driver"]
  B --> N["ExecWinUSBConfig -> _ExecWebUSBEnable/_Disable"]
  B2 --> N
  N --> R["EnableAutoUpdateFW -> SelectProbe -> OpenEx -> WebUSBEnable/Disable -> Sleep 100 -> Reboot -> Sleep 100"]
```

---

## Startup and refresh workflow (scan/probe info)

## A. Linux startup gate (udev policy)

```mermaid
flowchart TB
  Start[App starts on Linux] --> Check[Check JLink udev rules]
  Check -->|rules missing| Prompt[Request permission to setup rules]
  Prompt -->|Yes| Setup[Install udev rules]
  Setup --> Continue[Continue to probe table flow]
  Prompt -->|No| Exit[Close app]
  Check -->|rules already set| Continue
```

Expected behavior:

- If user refuses setup (`No`), app exits.
- If setup succeeds or rules already exist, app continues to scan/probe table.

## B. Low-level scan sequence (open app and `Refresh list`)

Backend entrypoints:

- App open path: `commands.rs::detect_and_scan`
- Refresh path: `commands.rs::scan_probes`

```mermaid
sequenceDiagram
  autonumber
  participant Cmd as Rust command (commands.rs)
  participant Svc as Rust service (domain/probe + jlink/service)
  participant Bridge as Native Bridge API
  participant DLL as JLink DLL/SO

  Note over Bridge,DLL: 0) Runtime Preparation loads .dll/.so during prepare_bundled_jlink
  Cmd->>Svc: detect_and_scan(...) OR scan_probes(...)
  Svc->>Bridge: list_probes_json() [jlink_bridge_list_probes_json]
  Bridge->>DLL: JLINKARM_EMU_GetList (ShowEmuList equivalent)
  loop for each listed probe index
    Svc->>Bridge: probe_open_details(index) [jlink_bridge_probe_firmware]
    Bridge->>DLL: exec DisableAutoUpdateFW (before select/open)
    Bridge->>DLL: SelectProbe (USBSN preferred, index fallback)
    Bridge->>DLL: JLINKARM_OpenEx
    Bridge->>DLL: JLINKARM_GetFirmwareString
    Bridge->>DLL: JLINKARM_ReadEmuConfigMem(0x8E, 1)
    Bridge->>DLL: JLINKARM_Close
  end
  Svc-->>Cmd: normalized probes
```

### Notes for scan

- Firmware auto-update is intentionally disabled for scan/connect path via `DisableAutoUpdateFW`.
- Probe info shown in table originates from list + per-probe open/read steps.

---

## WinUSB Switch Flow

Backend entrypoint:

- `commands.rs::switch_usb_driver`
- Branch:
  - with `serialNumber` -> `JLinkService::switch_usb_driver_by_serial`
  - without `serialNumber` -> `probe::switch_usb` -> `JLinkService::switch_usb_driver`

```mermaid
sequenceDiagram
  autonumber
  participant Cmd as Rust command (commands.rs)
  participant Svc as Rust service (jlink/service.rs)
  participant Bridge as Native Bridge API
  participant DLL as JLink DLL/SO

  Cmd->>Svc: switch_usb_driver(...) or switch_usb_driver_by_serial(...)
  Note over Svc,Bridge: service runs best-effort firmware update first
  Svc->>Bridge: update_firmware_json(...) / update_firmware_json_by_sn(...)
  Bridge->>DLL: exec DisableAutoUpdateFW (update connect path)
  Bridge->>DLL: SelectProbe + OpenEx
  Bridge->>DLL: (if updated) Sleep 100 -> ScheduleReboot/reboot -> Sleep 100
  Svc->>Bridge: switch_usb_json(...) / switch_usb_json_by_sn(...)
  Bridge->>DLL: exec EnableAutoUpdateFW (switch connect path)
  Bridge->>DLL: SelectProbe
  Bridge->>DLL: JLINKARM_OpenEx
  Bridge->>DLL: exec WebUSBEnable
  Bridge->>DLL: Sleep 100ms
  Bridge->>DLL: exec ScheduleReboot (fallback: reboot)
  Bridge->>DLL: Sleep 100ms
  Bridge->>DLL: JLINKARM_Close
  Note over Bridge,DLL: WinUSB Switch Flow wrapper then waits and attempts post-reboot reconnect
  Bridge->>DLL: Sleep 5000ms
  Bridge->>DLL: OpenEx once, then Close
  Svc-->>Cmd: success/failure + reboot details
```

### Notes for switch

- Current implementation is two-phase in Rust service:
  1. best-effort firmware update
  2. USB mode switch
- Switch phase matches Commander-style order:
  - `EnableAutoUpdateFW` -> select/open -> `WebUSBEnable` -> `Sleep 100` -> reboot -> `Sleep 100`
- After WinUSB Switch Flow, Native Bridge performs additional reconnect wait/check (`Sleep 5000` + one reconnect attempt).

---

## Mapping to your requested workflow

Requested items and implementation status:

- Runtime Preparation load (`.dll/.so`) -> implemented in `jlink_bridge_load`.
- `ShowEmuList` -> implemented via `JLINKARM_EMU_GetList`.
- `exec DisableAutoUpdateFW` on open/refresh -> implemented in connect helpers.
- `SelectProbe` -> implemented (USBSN preferred, index fallback).
- `JLINKARM_ReadEmuConfigMem(..., 0x8E, 1)` for USB mode -> implemented.
- WinUSB Switch Flow `EnableAutoUpdateFW` -> `SelectProbe` -> `WebUSBEnable` -> `Sleep 100` -> `Reboot` -> `Sleep 100` -> implemented (with `ScheduleReboot` preferred and `reboot` fallback).

