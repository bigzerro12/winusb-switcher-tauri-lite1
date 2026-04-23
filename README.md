# WinUSB Switcher Lite

Desktop utility built with **Tauri 2** for switching SEGGER **J-Link** USB probes between **WinUSB** and the default SEGGER USB stack (where supported). The application loads SEGGER’s J-Link **shared library in-process** through a small native bridge and ships a **trimmed runtime** under `src-tauri/resources/jlink-runtime/` (Windows DLLs or Linux `.so`, plus an adjacent **`Firmwares/`** tree). There is **no** in-app download, installer, or auto-update flow for SEGGER software.

**Stack:** Rust (`src-tauri`), React 18, TypeScript, Vite, Tailwind.  
**Compliance:** You are responsible for adhering to **SEGGER’s license and redistribution terms** for any J-Link binaries, firmware images, or documentation you bundle or ship.

---

## Features

- **Bootstrap** — Locates the bundled runtime, loads the native bridge, and configures process environment (`PATH` on Windows; `PATH` and `LD_LIBRARY_PATH` on Linux as needed).
- **Probe discovery** — Lists connected probes (serial, product, nickname, connection, firmware string).
- **USB driver mode** — Initiates driver switch workflow (including firmware check/update step via the SEGGER API where applicable).
- **Diagnostics** — `get_jlink_diagnostics` IPC command exposes runtime paths, bridge state, and version fields for support.

---

## Supported platforms

| Platform | Status | Notes |
|----------|--------|--------|
| **Windows** (x64, x86) | Supported | Bundled `JLink_x64.dll` or `JLinkARM.dll` under `jlink-runtime/windows-*`. |
| **Linux** (x64, x86) | Supported | Bundled `libjlinkarm.so` (or `.so.9`); **udev** rules required for reliable USB access. |
| **macOS** | Limited | Packaging may rely on stubs unless a full Darwin runtime is supplied; validate before release. |

---

## Bundled runtime layout

Under `src-tauri/resources/jlink-runtime/`, each target uses a per-platform directory, for example:

- `windows-64`, `windows-32`, `linux-64`, `linux-32`

A **versioned** layout is also supported, e.g. `jlink-v936/<platform>/` (see `bundled_jlink.rs` for resolution order).

**Each platform directory must contain:**

1. The J-Link library for that OS (`JLink_x64.dll`, `JLinkARM.dll`, or `libjlinkarm.so` / `libjlinkarm.so.9`).
2. A **`Firmwares/`** directory next to that library, with the `.bin` files the SEGGER stack expects.

**Optional zip staging:** If you maintain `src-tauri/jlink-bundles/<os>/<arch>/JLink_V930a.zip` (see [`.gitattributes`](.gitattributes)), run `yarn stage-jlink` before a raw `tauri` / `cargo` build. The default [`tauri.conf.json`](src-tauri/tauri.conf.json) bundles **`resources/jlink-runtime/**/*`**; keep that aligned with your packaging strategy.

---

## Requirements

- **Node.js** 20 (or compatible LTS)
- **Yarn** classic (v1) — `yarn.lock` in repo
- **Rust** stable (via [rustup](https://rustup.rs/))
- OS packages for **Tauri / WebView** per [Tauri prerequisites](https://tauri.app/start/prerequisites/)

---

## Development

```bash
git clone <repository-url>
cd <repository-root>

yarn install
yarn tauri:dev    # Full app: Vite + Tauri (required for IPC and J-Link)
yarn tauri:build  # Release-style bundle
```

- **`yarn dev`** — Frontend only (Vite). Tauri commands and J-Link integration will not run.
- **`yarn stage-jlink`** — Stages the zip for the current target when using the `jlink-bundles/` workflow outside `tauri dev` / `tauri build`.

---

## Configuration and debugging

| Item | Description |
|------|-------------|
| **`WINUSB_JLINK_DLL_OVERRIDE`** (Windows) | Optional absolute path to `JLink_x64.dll` or `JLinkARM.dll` to force-load a specific SEGGER build (e.g. compare against the bundled tree). |
| **Logging** | Backend uses `log` + `tauri-plugin-log` (stdout, app log directory, and webview target in debug). Adjust levels in [`src-tauri/src/lib.rs`](src-tauri/src/lib.rs). |
| **CSP** | Content Security Policy for the webview is set in [`src-tauri/tauri.conf.json`](src-tauri/tauri.conf.json); update it if the UI needs additional origins. |

---

## Version control (large binaries)

Bundled J-Link libraries and firmware under `src-tauri/resources/jlink-runtime/` (and optional `jlink-bundles/`) are stored **in normal Git**—not Git LFS—so clones and **GitHub Actions** do not need `git lfs pull` or LFS bandwidth. [`.gitattributes`](.gitattributes) marks those paths as **`-text`** to avoid CRLF normalization corrupting binaries.

The history of this repository was migrated off LFS for CI reliability. If you fork an older branch that still used LFS pointers, run `git lfs pull` on that branch only, or rebase onto `main`.

---

## Continuous integration and releases

| Workflow | Purpose |
|----------|---------|
| [`ci.yml`](.github/workflows/ci.yml) | Frontend `yarn build`; Rust `clippy`, tests, and release build on Ubuntu and Windows. Triggers on pushes/PRs to `main`, `master`, and `winusb-switcher-tauri-lite1`. |
| [`build.yml`](.github/workflows/build.yml) | Multi-platform installers; **release** job runs on `v*` tag pushes (and manual dispatch). |

**Release checklist (maintainers):**

1. Align **semver** in `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml` (no `v` prefix in those files).
2. Run `cargo check --manifest-path src-tauri/Cargo.toml` after changing `Cargo.toml` so `Cargo.lock` stays consistent.
3. Tag `vX.Y.Z`, push the tag, and confirm workflow artifacts attach to the release.
4. Grant GitHub Actions **workflow read/write** permission where needed so release assets can be published.
5. For Windows, use **Authenticode** signing in CI or post-build if you want fewer SmartScreen warnings for unsigned builds.

---

## Repository layout

```text
.
├── scripts/
│   ├── stage-jlink-for-build.mjs
│   └── push-testing-remote.sh    # Optional: push main to a second remote
├── src/renderer/                 # React UI
├── src/shared/types.ts           # IPC contracts / shared types
└── src-tauri/
    ├── icons/                    # Application icons
    ├── resources/
    │   ├── jlink-runtime/        # Bundled SEGGER runtime (~tens–hundreds of MB in Git)
    │   └── segger-99-jlink.rules
    ├── native/                   # C++: `common/` (shared PAL, JSON helpers, CWD) + `jlink/` (SEGGER bridge)
    ├── jlink-bundles/            # Optional per-OS zips, if used
    └── src/                      # Rust: commands, domain, FFI
```

---

## Troubleshooting

| Symptom | What to try |
|---------|-------------|
| Corrupt DLL / missing firmware / invalid zip | Confirm a full `git clone` (not a sparse checkout omitting `jlink-runtime`); verify `Firmwares/` sits beside the loaded library. |
| Linux: permission denied on USB | Install SEGGER-compatible **udev** rules; replug probe or reload rules (`udevadm`). The app may prompt via **`pkexec`** to install rules—canceling leaves setup incomplete. |
| Linux: wrong ELF / `.so` not found | Match app architecture to `linux-32` vs `linux-64`; check `LD_LIBRARY_PATH` and `ldd` on `JLinkExe` / bundled libs if you customize layout. |
| “Runtime not prepared” | Call **`prepare_bundled_jlink`** from the UI bootstrap before probe operations. |
| CI checkout errors | Ensure workflows use a standard `actions/checkout@v4` (no special LFS setup required for this repo). |

---

## Scope and limitations

- **J-Link only today:** The bundled runtime and native bridge target SEGGER’s library. A second probe family needs its own `native/*` tree and Rust domain module; see `src-tauri/src/domain/probe/mod.rs` module docs.
- **In-process native code:** The J-Link DLL/SO runs in the app process; a native crash can exit the whole app. A sidecar process would isolate that (not implemented here).
- **CI without hardware:** Workflows run `yarn lint`, TypeScript build, Clippy, and Rust unit tests. USB enumeration and driver switching are not exercised automatically.
- **Capabilities** apply only to windows that exist in `tauri.conf.json` (default label `main`).

---

## License

MIT — see [`LICENSE`](LICENSE).
