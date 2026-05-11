# ADR-0002: Keep full runtime source tree and stage target-filtered bundled runtime

- Status: Accepted
- Date: 2026-05-07

## Context

The repository must support multiple OS/architecture runtime payloads, but shipping every architecture’s binaries inside each installer or bundle would bloat artifacts and confuse runtime selection.

## Decision

Maintain:

- full source runtime tree: `src-tauri/resources/jlink-runtime/`
- target-filtered packaged tree: `src-tauri/resources/jlink-runtime-bundled/`

Populate the bundled tree during build using `stage-jlink-runtime-for-bundle.mjs` based on `TAURI_ENV_TARGET_TRIPLE`.

## Consequences

- Positive:
  - smaller release artifacts per target
  - deterministic target-specific packaging
- Trade-offs:
  - build-time staging step must always run correctly
  - dual-tree structure must be documented to avoid confusion

## Alternatives considered

- Bundle all runtimes in every installer: rejected due to artifact bloat and unnecessary payload.

