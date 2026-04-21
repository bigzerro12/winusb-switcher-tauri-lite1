# Manual test checklist

Use before each release (adjust per platforms you ship).

## Environment

- [ ] Clean install or uninstall previous version if testing upgrades.
- [ ] At least one J-Link probe connected; ideally **two** USB probes for multi-probe cases.

## Bootstrap

- [ ] App starts; bootstrap / `prepare_bundled_jlink` succeeds.
- [ ] If resources are missing, user sees a clear error (paths / layout).

## J-Link software row

- [ ] **Detected** when runtime loaded; version shows **V9.xx** style string (not raw build only).
- [ ] Path points at bundled `JLink_x64.dll` / `libjlinkarm.so` (or your packaged layout).

## Probe list

- [ ] **Refresh** lists all connected probes.
- [ ] Serial, product, nickname, connection, firmware columns look correct.
- [ ] With two probes, both appear; selection highlights expected row.

## USB driver switch

- [ ] **Switch to WinUSB** (or SEGGER) completes without uncaught errors.
- [ ] After switch, list may briefly show fewer probes; UI should **not** stay stuck with a shrunk list (polling behavior).
- [ ] Driver badge updates for the affected probe (may rely on overrides until rescan).

## Firmware

- [ ] With bundled `Firmwares/` present, firmware update path runs when switching (if your product always updates before switch).
- [ ] Intentionally break `Firmwares/` path → failure is understandable in logs / UI.

## Diagnostics

- [ ] Invoke `get_jlink_diagnostics` (from devtools or a temporary button): `runtimePrepared`, `bridgeLoaded`, `firmwaresDirExists` match expectations.

## Linux (if applicable)

- [ ] See `docs/LINUX.md` for udev / permissions.
- [ ] App loads `.so` from correct arch folder (`linux-64` / `linux-32`).

## Regression

- [ ] `yarn build` succeeds.
- [ ] `yarn tauri build` succeeds for your target(s).
