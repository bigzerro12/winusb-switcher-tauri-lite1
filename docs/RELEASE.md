# Release guide

## Version alignment

Before tagging a release, ensure these versions match:

| Location | Field |
|----------|--------|
| `package.json` | `version` |
| `src-tauri/tauri.conf.json` | `version` |
| `src-tauri/Cargo.toml` | `version` |

Update `CHANGELOG.md` with the new version and date.

## Windows code signing (recommended)

Unsigned installers trigger **Windows SmartScreen** warnings for many users.

1. Obtain a **code signing certificate** (Authenticode) from a trusted CA.
2. Sign the built `.exe` and installer (`.msi` / NSIS) with `signtool` (Windows SDK) or your CI’s signing step.
3. Optionally use **timestamping** so signatures remain valid after the certificate expires.

Exact commands depend on your certificate store (hardware token vs. PFX). Document your org’s procedure here when finalized.

## Content Security Policy

`src-tauri/tauri.conf.json` sets a **Content-Security-Policy** for the webview. If you add new origins (e.g. remote APIs from the UI), update the `csp` string accordingly.

## Bundled J-Link layout

See `src-tauri/README_JLINK_RUNTIME_LAYOUT.md` and `THIRD_PARTY.md`.

## Pre-release verification

Run through `docs/MANUAL_TEST_CHECKLIST.md` on each target OS you ship.
