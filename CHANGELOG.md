# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.0] - 2026-04-21

### Added

- Bundled J-Link runtime layout documentation (`src-tauri/README_JLINK_RUNTIME_LAYOUT.md`).
- Release documentation: `docs/RELEASE.md`, `docs/MANUAL_TEST_CHECKLIST.md`, `docs/LINUX.md`.
- Third-party / redistribution notice: `THIRD_PARTY.md`.
- `get_jlink_diagnostics` Tauri command for support snapshots.
- CI workflow (frontend build + Rust check).

### Changed

- Tauri webview **Content-Security-Policy** enabled (stricter default; localhost/WebSocket allowed for `tauri dev`).
- Frontend `AppErrorKind` aligned with Rust `AppError`.
- J-Link DLL version string formatted for UI (e.g. `V9.36`).

### Removed

- Legacy J-Link download/install UI and unused Tauri command constants for that flow.

[1.1.0]: https://github.com/ntgiahuy25d/winusb-switcher-tauri-lite1/releases/tag/v1.1.0
