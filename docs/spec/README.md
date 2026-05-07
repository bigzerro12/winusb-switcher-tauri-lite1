# Specification Index

Use this index to find authoritative behavior and design specifications.

## Core specifications

- `../ARCHITECTURE.md` - system layers, process model, and ownership boundaries
- `../WORKFLOWS.md` - user-visible runtime workflows and failure handling
- `../API.md` - renderer-backend command contracts and error model
- `JLINK_COMMAND_WORKFLOWS.md` - backend-to-native J-Link command-level execution paths

## Writing standard for specs

- Specify expected behavior, not temporary implementation details.
- State preconditions, invariants, and failure/rollback handling.
- Keep terminology consistent across architecture, API, and workflow docs.
- Keep diagrams aligned with current module boundaries and call chains.

Canonical terms to use across spec docs:

- Runtime Preparation / Runtime Prepared
- Sidecar Process
- Native Bridge
- WinUSB Switch Flow
