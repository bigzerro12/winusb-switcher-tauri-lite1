# Bundled SEGGER J-Link runtime layout

This app ships a **minimal subset** of SEGGER J-Link runtime files per target OS/CPU.

At runtime, the app:

- selects the correct folder under `src-tauri/resources/jlink-runtime/`
- loads the J-Link shared library **in-process** via the native bridge
- sets process environment so the library can find `Firmwares/`

## Expected folder layout

Two layouts are accepted:

1. **Versioned**

```
src-tauri/resources/jlink-runtime/jlink-v936/<platform>/
```

2. **Unversioned**

```
src-tauri/resources/jlink-runtime/<platform>/
```

Where `<platform>` is one of:

- `windows-64`
- `windows-32`
- `linux-64`
- `linux-32`

## Required contents per platform

Each `<platform>/` directory must contain:

- the J-Link shared library:
  - Windows 64-bit: `JLink_x64.dll`
  - Windows 32-bit: `JLinkARM.dll`
  - Linux: `libjlinkarm.so` (optionally `libjlinkarm.so.9` is also accepted)
- the `Firmwares/` directory next to the library:

```
<platform>/
  JLink_x64.dll|JLinkARM.dll|libjlinkarm.so
  Firmwares/
    *.bin
```

Notes:

- The J-Link library discovers firmware binaries **relative to its own directory**.
- If you remove the `Firmwares/` folder or place it elsewhere, probe firmware update will not work.

