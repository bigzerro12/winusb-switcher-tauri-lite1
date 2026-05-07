# Architecture Specification

## Purpose

This document defines the system structure, boundaries, and dependency rules for maintainers and future contributors.

## Scope

In scope:

- Renderer (React/TypeScript)
- Tauri backend (Rust commands and domain)
- Native Bridge integration layer (Rust FFI + Sidecar Process + C++)
- Runtime bundling strategy and process boundaries

Out of scope:

- Detailed UI/UX behavior
- Hardware validation procedures (see release checklist/testing docs)

---

## Preconditions

- Repository structure and paths referenced in this document match the current codebase.
- Runtime artifacts are staged using the documented build/runtime scripts.
- Sidecar Process execution model remains enabled for Native Bridge calls.

---

## Flow

### Layered architecture (authoritative view)

Dependency rule: **higher layers may depend on lower layers; lower layers must not depend on higher layers**.

```mermaid
flowchart TB
  subgraph L1["Layer 1 - Presentation (Renderer)"]
    UI["React UI<br/>screens + components"]
    Store["State management<br/>Zustand"]
    Client["Command client<br/>invoke wrappers"]
    UI --> Store --> Client
  end

  subgraph L2["Layer 2 - Application (Rust/Tauri)"]
    Cmd["Tauri commands<br/>commands.rs"]
    Domain["Domain services<br/>src-tauri/src/domain/*"]
    Cmd --> Domain
  end

  subgraph L3["Layer 3 - Integration and Runtime"]
    SidecarCtl["Sidecar Process orchestration<br/>bridge_sidecar.rs"]
    Runtime["Runtime selection/staging<br/>infra/runtime/*"]
    Udev["Linux udev setup<br/>bundled.rs"]
    Domain --> SidecarCtl
    Domain --> Runtime
    Runtime --> Udev
  end

  subgraph L4["Layer 4 - Native and External"]
    FFI["Rust FFI boundary<br/>jlink_ffi.rs"]
    Sidecar["Sidecar Process<br/>--jlink-sidecar"]
    Native["Native Bridge (C++)<br/>native/jlink/*"]
    Segger["SEGGER runtime<br/>DLL/SO + Firmwares"]
    OS["Host OS services<br/>udev/drivers/USB"]
    SidecarCtl --> FFI --> Sidecar --> Native --> Segger
    Udev --> OS
  end

  Client --> Cmd
```

---

### Deployment/process architecture

```mermaid
flowchart LR
  Main["Main process<br/>WebView + Rust backend"] -->|spawn + supervise| Side["Sidecar Process"]
  Main -->|stdio JSON-RPC| Side
  Side -->|FFI/native calls| Bridge["Native Bridge (C++)"]
  Bridge -->|LoadLibrary / dlopen| Segger["SEGGER J-Link runtime"]
```

Rationale:

- Native faults are isolated from the main process.
- Sidecar Process can be restarted without tearing down the UI.

---

### Runtime bundling model

There are two runtime trees by design:

- `src-tauri/resources/jlink-runtime/` (full source runtime set)
- `src-tauri/resources/jlink-runtime-bundled/` (target-filtered payload included in installers)

```mermaid
sequenceDiagram
  autonumber
  participant Build as tauri build / CI
  participant Stage as stage-jlink-runtime-for-bundle.mjs
  participant Full as jlink-runtime
  participant Bundled as jlink-runtime-bundled

  Build->>Stage: beforeBuildCommand (TAURI_ENV_TARGET_TRIPLE)
  Stage->>Bundled: clear previous staged payloads
  Stage->>Full: resolve platform subtree
  Stage->>Bundled: copy target runtime + Firmwares
  Build->>Build: package resources/jlink-runtime-bundled/**/*
```

---

## Failure handling

- Violations of layer dependency direction must be treated as architecture regressions.
- Native Bridge instability is contained by Sidecar Process isolation and restart boundaries.
- Runtime staging mismatches must fail early in build/release validation.

---

## Ownership

### Ownership map

- Renderer: `src/renderer/*`
- Command layer: `src-tauri/src/commands.rs`
- Domain logic: `src-tauri/src/domain/*`
- Sidecar Process orchestration: `src-tauri/src/bridge_sidecar.rs`
- FFI boundary: `src-tauri/src/jlink_ffi.rs`
- Native Bridge: `src-tauri/native/jlink/*`
- Runtime and platform bootstrapping: `src-tauri/src/infra/runtime/*`

