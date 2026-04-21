# WinUSB Switcher Lite (Tauri)

A **Tauri 2** desktop app that switches SEGGER J-Link probes to **WinUSB** driver mode. This tree (**Lite 1**) loads SEGGER’s J-Link **shared library in-process** via a small native bridge and ships a **trimmed J-Link runtime** under `src-tauri/resources/jlink-runtime/` (DLL / `.so` plus `Firmwares/`). There is **no** in-app download or SEGGER installer flow.

Stack: **Rust** (`src-tauri`), **React + TypeScript** (`src/renderer`). Comply with SEGGER’s license and redistribution terms for any J-Link binaries you ship.

## Source branch

Ongoing work for this variant is tracked on Git branch **`winusb-switcher-tauri-lite1`**. CI runs on **`main`**, **`master`**, and **`winusb-switcher-tauri-lite1`**.

Canonical personal remote for this line of work: **`https://github.com/ntgiahuy25d/winusb-switcher-tauri-lite1.git`** (branch **`winusb-switcher-tauri-lite1`**). If your GitHub user is different, substitute it in the URL.

**Create the empty GitHub repo once** (no README, no `.gitignore`, no license — you already have those locally):

1. **GitHub CLI** (after `gh auth login`):

   ```bash
   cd /path/to/winusb-switcher-tauri-lite1
   git remote remove origin 2>/dev/null || true
   gh repo create winusb-switcher-tauri-lite1 --public --source=. --remote=origin --push
   git lfs push origin winusb-switcher-tauri-lite1 --all
   ```

2. **Or in the browser:** [github.com/new](https://github.com/new) → Repository name **`winusb-switcher-tauri-lite1`** → **Create repository**, then:

   ```bash
   git remote set-url origin https://github.com/ntgiahuy25d/winusb-switcher-tauri-lite1.git
   git push -u origin winusb-switcher-tauri-lite1
   git lfs push origin winusb-switcher-tauri-lite1 --all
   ```

If `origin` was wrong (e.g. another user’s repo), fix it with `git remote set-url origin <your-clone-url>` before pushing.

## Bundled J-Link runtime

Under `src-tauri/resources/jlink-runtime/`, each supported layout uses a directory per platform, for example `windows-64`, `windows-32`, `linux-64`, or `linux-32` (sometimes under a versioned parent such as `jlink-v936/<platform>/`). That directory must contain the J-Link shared library for the OS (`JLink_x64.dll`, `JLinkARM.dll`, or `libjlinkarm.so`) and a **`Firmwares/`** folder next to it with SEGGER firmware `.bin` files. The library resolves firmware relative to its own directory.

**Optional zip-based staging:** If you maintain **`src-tauri/jlink-bundles/<os>/<arch>/JLink_V930a.zip`** (see [`.gitattributes`](.gitattributes)), run `yarn stage-jlink` before `tauri build` so `resources/jlink/...` is populated. The default **bundle resources** in `tauri.conf.json` follow **`resources/jlink-runtime/**/*`**; align resources with your packaging approach before release builds.

## Platforms (summary)

| Platform | Bundled runtime | Notes |
|----------|-----------------|--------|
| **Windows x64 / x86** | `jlink-runtime/windows-*` | Native bridge loads `JLink_x64.dll` / `JLinkARM.dll`. |
| **Linux x64 / x86** | `jlink-runtime/linux-*` | Uses `libjlinkarm.so`; USB access needs correct **udev** rules (see **Troubleshooting**). |
| **macOS** | Stub / partial | Universal/mac builds may use an empty zip stub from `stage-jlink` where no Darwin payload exists; full macOS support depends on supplying a real runtime. |

## Release and versioning

- **CI:** `.github/workflows/ci.yml` — frontend `yarn build`, Rust `clippy` / `test` / `release` build (Ubuntu + Windows).
- **Installers / GitHub Release:** `.github/workflows/build.yml` — tag `v*` (and manual dispatch); release job uploads artifacts when a tag is pushed.

**Maintainers:** Keep the same semver in `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml` (no `v` prefix in those files). After editing `Cargo.toml`, run `cargo check --manifest-path src-tauri/Cargo.toml` so `Cargo.lock` stays in sync.

```bash
git checkout main && git pull
# bump versions, commit
git tag v1.1.0
git push origin main
git push origin v1.1.0
```

GitHub **Actions** → **Workflow permissions** → **Read and write** so the release job can upload assets.

For Windows releases, plan **Authenticode signing** (`signtool` or your CI) if you want to reduce SmartScreen warnings. If you change the webview **CSP** in `tauri.conf.json`, update it whenever the UI talks to new origins.

## Git LFS

Large binaries under **`src-tauri/resources/jlink-runtime/`** and optional **`src-tauri/jlink-bundles/`** zips use **Git LFS** (see [`.gitattributes`](.gitattributes)).

```bash
git lfs install
git clone <repo-url>
cd <repo>
git lfs pull
```

If runtime files appear as tiny pointer text files or the app reports corrupt payloads, run **`git lfs pull`** and rebuild.

## Development

Prerequisites: **Node.js 20** (or compatible), **Yarn classic (v1)**, **Rust** (stable), and [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS.

```bash
yarn install
yarn tauri:dev      # Vite + Tauri; use this for IPC and J-Link (not `yarn dev` alone)
yarn tauri:build    # production bundle
```

- **`yarn dev`** — Vite only; Tauri IPC and backend commands will not work.
- **`yarn stage-jlink`** — Run when using the **zip** staging path (`jlink-bundles/`) with raw `tauri` / `cargo` invocations.

## Project layout (high level)

```text
.
├── scripts/
│   ├── stage-jlink-for-build.mjs
│   └── push-testing-remote.sh  # Optional second remote (main + LFS)
├── src/renderer/               # React UI
├── src/shared/types.ts         # IPC names / shared types
└── src-tauri/
    ├── icons/                  # App icons + R365-icon.png
    ├── resources/
    │   ├── jlink-runtime/      # Bundled J-Link libs + Firmwares (Git LFS)
    │   └── segger-99-jlink.rules
    ├── native/                 # C++ bridge (Pal, J-Link DLL wrapper)
    ├── jlink-bundles/          # Optional per-OS zips (Git LFS), if used
    └── src/                    # Rust commands, J-Link service
```

## CI and LFS quota

Workflows that need bundles or `jlink-runtime` should use **`actions/checkout@v4` with `lfs: true`** (see `build.yml`). If GitHub reports **LFS budget exceeded**, add LFS data on the owning account or host large artifacts outside GitHub and adjust workflows.

## Mirroring (optional)

To push **`main`** and tags to a second remote (e.g. testing account), use a **separate `git remote`**, authenticate as a user with **write** access, then:

```bash
git push <remote> main
git push <remote> v1.1.0
git lfs push <remote> main
```

Use SSH host aliases if two GitHub identities share one host name (see GitHub SSH docs).

## Troubleshooting

- **LFS pointer / invalid zip / missing DLL** — `git lfs install && git lfs pull`, rebuild.
- **Linux USB / udev** — Install SEGGER-compatible **udev** rules so your user can access the probe; reload rules or replug after changes. The app may use **`pkexec`** once to place rules under `/etc`; if you dismiss it, setup stays incomplete—try again from the app or copy rules manually, then `udevadm control --reload-rules && udevadm trigger` as appropriate.
- **Linux `LD_LIBRARY_PATH`** — The app prepends the resolved runtime directory so dependent `.so` files next to `libjlinkarm.so` resolve; keep libraries and `Firmwares/` in a normal SEGGER-like layout.
- **32-bit vs 64-bit on Linux** — Match the app binary to the `linux-32` vs `linux-64` runtime folder.
- **“Could not open J-Link shared library”** — Confirm real binaries under `jlink-runtime` (not LFS pointers), `Firmwares/` present, and on Linux use `ldd` / ensure `libusb` and related deps are installed.

## License

MIT — see [`LICENSE`](LICENSE).
