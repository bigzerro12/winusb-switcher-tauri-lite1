# Workflow Specification

## Purpose

Define runtime behavior for the key operational paths:

- Startup/bootstrap
- Probe refresh/scan
- WinUSB Switch Flow

---

## Scope

In scope:

- Startup bootstrap behavior, including Linux strict udev policy handling.
- Probe discovery and refresh behavior across startup/manual/post-switch paths.
- Switch-to-WinUSB operational flow and result propagation.

Out of scope:

- UI layout and component-level rendering behavior.
- Low-level command primitive details (covered in `spec/JLINK_COMMAND_WORKFLOWS.md`).

---

## Preconditions

- Runtime Preparation has been wired into startup flow.
- Sidecar Process can be spawned from the main process.
- Command handlers remain mapped through `commands.rs` into domain services.

---

## Flow

### Workflow map

```mermaid
flowchart LR
  Start[Application start] --> Boot[Runtime Preparation + Sidecar Process bootstrap]
  Boot --> Scan[Detect and scan probes]
  Scan --> UIReady[Dashboard ready]
  UIReady -->|user refresh| Scan
  UIReady -->|switch action| Switch[WinUSB Switch Flow]
  Switch --> Verify[Re-scan and verify state]
  Verify --> UIReady
```

---

### WF-01 Startup/bootstrap

### Preconditions

- Bundled runtime resources are present in the app package.

### Sequence

```mermaid
sequenceDiagram
  autonumber
  participant OS as OS
  participant App as Main process
  participant Runtime as Runtime Preparation
  participant Udev as Linux udev flow
  participant Sidecar as Sidecar Process
  participant Segger as SEGGER runtime

  OS->>App: Launch executable
  App->>Runtime: Initialize config/log/state
  alt Linux
    App->>Udev: Ensure rules (may prompt via pkexec)
    alt Refused or failed
      Udev-->>App: error
      App-->>OS: terminate startup
    else Success
      Udev-->>App: ready
    end
  end
  App->>Sidecar: Spawn/preload
  Sidecar->>Segger: Load DLL/SO
  Segger-->>Sidecar: ok/error
  Sidecar-->>App: ready/error
```

### Failure handling

- Runtime Preparation failure -> startup blocked with actionable error.
- Linux rules refusal/failure -> app exits by policy (strict environment).
- If another app instance is already running, startup is blocked and the user is notified.

### Design notes

- Runtime initialization opens a single append-only user log file and writes explicit session start/end markers.
- App runtime data for `com.winusbswitcher.lite` (logs/cache) is stored under the installation directory, not under user profile locations such as `AppData`.

---

### WF-02 Refresh and scan

### Trigger

- App startup, manual refresh, or post-switch verification.

### Sequence

```mermaid
sequenceDiagram
  autonumber
  participant UI as Renderer
  participant Cmd as Tauri command
  participant Svc as Domain service
  participant Sidecar as Sidecar Process
  participant Native as Native Bridge (C++)

  UI->>Cmd: detect_and_scan / scan_probes
  Cmd->>Svc: validate Runtime Prepared state + build request
  Svc->>Sidecar: JSON-RPC scan
  Sidecar->>Native: native scan call
  Native-->>Sidecar: probe list + details
  Sidecar-->>Svc: response payload
  Svc-->>Cmd: normalized domain model
  Cmd-->>UI: probe state
```

### Design notes

- Parsing/normalization belongs to backend, not UI.
- UI should treat returned state as canonical and avoid stale index assumptions.

---

### WF-03 WinUSB Switch Flow

### Trigger

- User initiates WinUSB Switch Flow from dashboard.

### Sequence

```mermaid
sequenceDiagram
  autonumber
  participant UI as Renderer
  participant Cmd as switch_usb_driver
  participant Svc as Domain service
  participant Sidecar as Sidecar Process
  participant Native as Native Bridge (C++)
  participant OS as Host driver stack

  UI->>Cmd: switch request (probe + target mode)
  Cmd->>Svc: validate request + Runtime Prepared state
  Svc->>Sidecar: JSON-RPC switch
  Sidecar->>Native: execute switch sequence
  Native->>OS: apply mode/driver change
  Native->>Native: reconnect wait window
  Native-->>Sidecar: success/failure + details
  Sidecar-->>Svc: switch result
  Svc-->>Cmd: structured response
  Cmd-->>UI: operation result
```

### Failure handling

- Runtime not prepared -> operation rejected early.
- Sidecar Process interruption -> error propagated with restart/retry path.
- Re-enumeration timing miss -> failure surfaced; user can retry after refresh.

---

## Ownership

- Command entrypoints: `src-tauri/src/commands.rs`
- Workflow orchestration: `src-tauri/src/domain/probe/*`, `src-tauri/src/domain/jlink/*`
- Sidecar Process/Native Bridge execution: `src-tauri/src/bridge_sidecar.rs`, `src-tauri/native/jlink/*`

