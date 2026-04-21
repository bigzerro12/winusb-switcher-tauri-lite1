# Third-party components and redistribution

## SEGGER J-Link

This application may ship **SEGGER J-Link** runtime files (for example `JLink_x64.dll`, `JLinkARM.dll`, `libjlinkarm.so`, and contents of the `Firmwares/` directory) as bundled resources under `src-tauri/resources/jlink-runtime/`.

**You are responsible for compliance** with SEGGER’s license terms, redistribution rules, and any trademark or attribution requirements that apply to your distribution channel (internal tools, commercial product, open source, etc.).

- Official downloads and documentation: https://www.segger.com/downloads/jlink/
- Do not assume that bundling these binaries is permitted for all use cases; verify against the license that applies to the SEGGER package you copied into this repository.

## Tauri, Rust, and npm ecosystem

Runtime dependencies (Tauri, React, Vite, etc.) are governed by their respective licenses. See each package’s `LICENSE` in `node_modules` and Rust crate metadata for your compliance records.

## This project’s license

See the root `LICENSE` file (if present) or `package.json` `"license"` field for the license applying to **your** application source code (excluding third-party binaries).
