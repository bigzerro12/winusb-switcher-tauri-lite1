# Linux notes

## Bundled runtime

The app loads **SEGGER `libjlinkarm.so`** from the bundled tree under `src-tauri/resources/jlink-runtime/<linux-64|linux-32>/` (or a versioned parent folder). See `src-tauri/README_JLINK_RUNTIME_LAYOUT.md`.

## USB access (udev)

J-Link probes are USB devices. Without proper permissions, enumeration or `OpenEx` may fail or be flaky.

- Use SEGGER’s **udev rules** from a full J-Link install, or rules shipped with your SEGGER package, so your user can access the device (commonly via the `plugdev` group or a dedicated `jlink` group, depending on the rules file).
- After installing rules, **replug** the probe or run `udevadm control --reload-rules && udevadm trigger` (as root) if appropriate.

## `LD_LIBRARY_PATH`

The app prepends the resolved runtime directory to `LD_LIBRARY_PATH` so dependent shared objects next to `libjlinkarm.so` resolve correctly. If you customize the bundle layout, keep libraries and `Firmwares/` consistent with a normal SEGGER install tree.

## 32-bit vs 64-bit

Match the app binary architecture to the bundled `linux-32` vs `linux-64` folder. Loading a wrong-ELF `.so` fails at runtime with a clear dlopen-style error.
