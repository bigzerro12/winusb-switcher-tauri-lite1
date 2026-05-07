# Documentation Home

Use this page as the entry point for design, behavior, and operations documentation.

## Start here

- Read `spec/README.md` for architecture and behavior specs.
- Read `ops/README.md` for debugging, platform, and release runbooks.
- Read `adr/README.md` for key architecture decisions and rationale.

## Pick docs by task

- Understand architecture boundaries: `ARCHITECTURE.md`
- Understand runtime behavior and user flows: `WORKFLOWS.md`
- Understand renderer-backend contracts: `API.md`
- Understand low-level J-Link command paths: `spec/JLINK_COMMAND_WORKFLOWS.md`
- Troubleshoot issues and failures: `DEBUGGING.md`
- Prepare and publish releases: `RELEASE_AND_CI.md`, `../RELEASE_CHECKLIST.md`

## Documentation model

- `spec/` provides specification indexes and low-level behavior details.
- `ops/` provides operational indexes and runbooks.
- `adr/` captures long-term technical decisions and trade-offs.
- Top-level docs (`ARCHITECTURE.md`, `WORKFLOWS.md`, `API.md`, `DEBUGGING.md`, `PLATFORM_NOTES.md`, `RELEASE_AND_CI.md`) are the source-of-truth documents.

## Canonical terminology

- **Runtime Preparation**: the startup path that stages and loads J-Link runtime artifacts.
- **Runtime Prepared**: the state after Runtime Preparation succeeds and command paths may proceed.
- **Sidecar Process**: the `--jlink-sidecar` process hosting bridge-side execution.
- **Native Bridge**: the C++ integration layer under `src-tauri/native/jlink/*`.
- **WinUSB Switch Flow**: the end-to-end USB mode switch operation initiated from `switch_usb_driver`.

