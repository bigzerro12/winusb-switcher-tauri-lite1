import { invoke } from "@tauri-apps/api/core";
import type {
  InstallStatus,
  Probe,
  ProviderType,
  UsbDriverMode,
  UsbDriverResult,
} from "@shared/types";
import { COMMANDS } from "@shared/types";

export type DetectAndScanResult = {
  status: InstallStatus;
  probes: Probe[];
};

export async function prepareBundledJlink(): Promise<string> {
  return invoke<string>(COMMANDS.PREPARE_BUNDLED_JLINK);
}

export async function detectAndScan(): Promise<DetectAndScanResult> {
  return invoke<DetectAndScanResult>(COMMANDS.DETECT_AND_SCAN);
}

export async function scanProbes(): Promise<Probe[]> {
  return invoke<Probe[]>(COMMANDS.SCAN_PROBES);
}

export async function switchUsbDriver(
  probeIndex: number,
  mode: UsbDriverMode,
  provider?: ProviderType,
  serialNumber?: string,
): Promise<UsbDriverResult> {
  return invoke<UsbDriverResult>(COMMANDS.SWITCH_USB_DRIVER, {
    request: {
      probeIndex,
      mode,
      ...(provider ? { provider } : {}),
      ...(serialNumber ? { serialNumber } : {}),
    },
  });
}
