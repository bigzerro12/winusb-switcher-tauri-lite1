# Release Checklist

Use this checklist for each tagged release (`vX.Y.Z`) of WinUSB Switcher Lite.

## 1) Source and Version Integrity

- [ ] `git status` is clean (no unintended local changes)
- [ ] Version is aligned across:
  - `package.json`
  - `src-tauri/Cargo.toml`
  - `src-tauri/tauri.conf.json`
- [ ] Latest commit messages contain no forbidden attribution trailers
- [ ] Release tag (`vX.Y.Z`) points to the intended commit

## 2) Required Build and Test Gates

- [ ] `yarn lint`
- [ ] `yarn test:run`
- [ ] `yarn build`
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --locked --all-targets -- -D warnings`
- [ ] `cargo test --locked`
- [ ] `cargo build --locked --release`

## 3) Packaging Validation

- [ ] Windows packages build successfully (`.exe`, `.msi`)
- [ ] Linux packages build successfully (`.deb`, `.AppImage`)
- [ ] `SHA256SUMS.txt` is generated and included in release assets
- [ ] Installers run correctly on clean test machines

## 4) Hardware-in-the-Loop Validation (Critical)

- [ ] Windows probe flow: detect -> switch SEGGER to WinUSB -> re-detect confirms state
- [ ] Windows probe flow: switch WinUSB to SEGGER -> re-detect confirms state
- [ ] Linux udev authorization flow succeeds with expected prompt behavior
- [ ] Linux unplug/replug and re-enumeration recovery is stable
- [ ] Failure paths produce actionable UI messages

## 5) Runtime and Security Sanity

- [ ] Sidecar timeout/recovery behavior works as expected
- [ ] Logs do not expose sensitive identifiers beyond approved scope
- [ ] Linux privileged temp-file behavior validated on target distro
- [ ] No CSP or permission regressions in packaged builds

## 6) Release Workflow and Metadata

- [ ] `.github/workflows/ci.yml` is green on release commit
- [ ] `.github/workflows/build.yml` is green for release tag
- [ ] Release notes are complete (changes, risks, known limits)
- [ ] `README.md` reflects shipped behavior and platform support
- [ ] SEGGER redistribution/licensing compliance re-confirmed

## 7) Rollback Readiness

- [ ] Previous stable tag identified
- [ ] Rollback steps documented (which tag/artifacts to restore)
- [ ] Owner on-call for release/hotfix triage is assigned

## Go/No-Go Rule

- **GO**: all items in sections 2, 3, and 4 pass, and no high-severity blocker remains.
- **NO-GO**: any build/test gate fails, any hardware-in-the-loop check fails, or a critical risk is unresolved.

