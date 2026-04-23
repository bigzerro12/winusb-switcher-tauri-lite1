/**
 * Thin wrappers around Tauri `invoke()` for the backend commands.
 *
 * Each wrapper validates and normalizes the response via `./contracts` so the
 * rest of the renderer always sees well-typed, UI-ready values.
 */

import { invoke } from "@tauri-apps/api/core";
import type {
  DetectAndScanResult,
  Probe,
  ProviderType,
  UsbDriverMode,
  UsbDriverResult,
} from "@shared/types";
import { COMMANDS } from "@shared/types";
import {
  parseBundledJlinkPath,
  parseDetectAndScanResult,
  parseProbeList,
  parseUsbDriverResult,
} from "./contracts";

export async function prepareBundledJlink(): Promise<string> {
  return parseBundledJlinkPath(await invoke(COMMANDS.PREPARE_BUNDLED_JLINK));
}

export async function detectAndScan(): Promise<DetectAndScanResult> {
  return parseDetectAndScanResult(await invoke(COMMANDS.DETECT_AND_SCAN));
}

export async function scanProbes(): Promise<Probe[]> {
  return parseProbeList(await invoke(COMMANDS.SCAN_PROBES));
}

export async function switchUsbDriver(
  probeIndex: number,
  mode: UsbDriverMode,
  provider?: ProviderType,
  serialNumber?: string,
): Promise<UsbDriverResult> {
  const request = {
    probeIndex,
    mode,
    ...(provider ? { provider } : {}),
    ...(serialNumber ? { serialNumber } : {}),
  };
  return parseUsbDriverResult(
    await invoke(COMMANDS.SWITCH_USB_DRIVER, { request }),
  );
}
