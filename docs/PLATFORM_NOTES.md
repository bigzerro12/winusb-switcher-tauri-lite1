# Platform Notes and Constraints

## Purpose

Document platform-specific behavior, prerequisites, and operational constraints.

---

## Scope

In scope:

- Linux and Windows runtime/driver constraints.
- Build/runtime prerequisites that affect startup, scan, and WinUSB Switch Flow behavior.
- Explicit non-support position for macOS in this repository.

Out of scope:

- End-user installation guides.
- OS-level policy administration outside app/runtime requirements.

---

## Preconditions

- Runtime Preparation artifacts are staged for the current platform/architecture.
- Required OS packages and permissions are available.
- Device reconnect/re-enumeration behavior is understood during validation.

---

## Flow

### Linux

### Access control (udev)

Requirement:

- Non-root USB access requires installed/active udev rules for the target device class.

Operational behavior:

- Startup checks rules presence.
- Missing rules trigger privileged installation (for example via `pkexec`).
- In strict policy mode, refusing/failed setup blocks app continuation.

Maintenance implications:

- Any change to rule content or installer path must be validated on supported distros.
- USB re-enumeration tests should include unplug/replug after rule updates.

### Runtime Preparation and loading

- Sidecar Process/Native Bridge path loads shared libraries using `dlopen`.
- Runtime folder must contain the expected `.so` and `Firmwares/` sibling directory.

### Build prerequisites

- Linux CI/build needs GTK/WebKitGTK/libsoup development packages in place.
- Install-root write access is required for append-only logs, the single-instance lock file, and local WebView cache.

---

### Windows

### Driver switch semantics

- Switching to WinUSB depends on host driver policy and permissions.
- Corporate endpoint controls can block or alter expected installation behavior.

### Runtime loading

- x64 uses `JLink_x64.dll`.
- x86 uses `JLinkARM.dll`.
- Runtime location must match Runtime Preparation output for the selected target.
- Install-root write access is required for append-only logs, the single-instance lock file, and local WebView cache.

---

### macOS

- Not supported in this repository (no bundled Darwin runtime and no maintained Native Bridge path).

---

## Failure handling

- Missing Linux udev setup in strict mode blocks continuation by design.
- Missing Runtime Preparation libraries (`.so`/`.dll`) must fail early with explicit diagnostics.
- Unsupported platforms should fail with clear, non-ambiguous messaging.

---

## Ownership

- Platform bootstrapping/runtime setup: `src-tauri/src/infra/runtime/*`
- Linux udev and startup policy: `src-tauri/src/infra/runtime/bundled.rs`
- Native Bridge loading behavior: `src-tauri/native/jlink/*`

