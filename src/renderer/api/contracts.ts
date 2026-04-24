/**
 * IPC contract validators and normalizers.
 *
 * Each `parseX(raw)` helper:
 *   1. Validates that `raw` matches the expected Rust response shape.
 *   2. Normalizes it into the UI-facing type (e.g. arbitrary `driver` string → `DriverType`).
 *
 * Throwing here indicates a Rust ↔ TypeScript schema drift, not a user error.
 */

import type {
  DetectAndScanResult,
  DriverType,
  RuntimeStatus,
  Probe,
  ProviderType,
  UsbDriverResult,
} from "@shared/types";

// ─── Primitive guards ────────────────────────────────────────────────────────

function isRecord(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null;
}

function isString(v: unknown): v is string {
  return typeof v === "string";
}

function isBoolean(v: unknown): v is boolean {
  return typeof v === "boolean";
}

/** Accept `undefined` or `null` (how Rust `Option<T>` serializes) in addition to `T`. */
function isOptional<T>(v: unknown, isT: (x: unknown) => x is T): v is T | null | undefined {
  return v === undefined || v === null || isT(v);
}

// ─── Domain guards ───────────────────────────────────────────────────────────

function isProviderType(v: unknown): v is ProviderType {
  return v === "JLink";
}

function isRuntimeStatus(v: unknown): v is RuntimeStatus {
  return (
    isRecord(v) &&
    isBoolean(v.ready) &&
    isOptional(v.nativeLibPath, isString) &&
    isOptional(v.version, isString)
  );
}

/**
 * Raw probe as it arrives from Rust.
 *
 * `driver` is left as an arbitrary string here; the backend sends `SEGGER`, `WinUSB`, or
 * `Unknown`, and the renderer maps to {@link DriverType} via {@link toDriverType}.
 */
type RawProbe = Omit<Probe, "driver"> & { driver: string };

function isRawProbe(v: unknown): v is RawProbe {
  return (
    isRecord(v) &&
    isString(v.id) &&
    isString(v.serialNumber) &&
    isString(v.productName) &&
    isString(v.nickName) &&
    isProviderType(v.provider) &&
    isString(v.connection) &&
    isString(v.driver) &&
    isOptional(v.firmware, isString)
  );
}

function isUsbDriverResult(v: unknown): v is UsbDriverResult {
  return (
    isRecord(v) &&
    isBoolean(v.success) &&
    isOptional(v.error, isString) &&
    isOptional(v.detail, isString) &&
    isOptional(v.rebootNotSupported, isBoolean)
  );
}

// ─── Normalization ───────────────────────────────────────────────────────────

function toDriverType(driver: string): DriverType {
  return driver === "SEGGER" || driver === "WinUSB" ? driver : "Unknown";
}

function normalizeProbe(raw: RawProbe): Probe {
  return { ...raw, driver: toDriverType(raw.driver) };
}

// ─── Public parsers (validate + normalize) ───────────────────────────────────

function contractError(command: string): Error {
  return new Error(`internal: invalid ${command} response shape`);
}

export function parseProbeList(raw: unknown): Probe[] {
  if (!Array.isArray(raw) || !raw.every(isRawProbe)) {
    throw contractError("scan_probes");
  }
  return raw.map(normalizeProbe);
}

export function parseDetectAndScanResult(raw: unknown): DetectAndScanResult {
  if (
    !isRecord(raw) ||
    !isRuntimeStatus(raw.status) ||
    !Array.isArray(raw.probes) ||
    !raw.probes.every(isRawProbe)
  ) {
    throw contractError("detect_and_scan");
  }
  return {
    status: raw.status,
    probes: raw.probes.map(normalizeProbe),
  };
}

export function parseUsbDriverResult(raw: unknown): UsbDriverResult {
  if (!isUsbDriverResult(raw)) {
    throw contractError("switch_usb_driver");
  }
  return raw;
}

export function parseBundledJlinkPath(raw: unknown): string {
  if (!isString(raw) || raw.length === 0) {
    throw contractError("prepare_bundled_jlink");
  }
  return raw;
}
