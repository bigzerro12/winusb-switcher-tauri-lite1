# ADR-0001: Isolate Native Bridge in a Sidecar Process

- Status: Accepted
- Date: 2026-05-07

## Context

The project integrates with external native libraries and low-level USB operations. A failure in native code can crash the host process if executed in-process.

## Decision

Run Native Bridge calls in a dedicated Sidecar Process (`--jlink-sidecar`) and communicate via stdio RPC from the main Tauri process.

## Consequences

- Positive:
  - Native Bridge crashes are isolated from UI/backend process
  - host can restart Sidecar Process and preserve app session
- Trade-offs:
  - additional process lifecycle management
  - extra serialization layer for requests/responses

## Alternatives considered

- In-process FFI only: simpler architecture, but unacceptable crash blast radius.

