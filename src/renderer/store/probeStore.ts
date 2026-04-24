import { create } from "zustand";
import {
  detectAndScan,
  scanProbes,
  switchUsbDriver as invokeSwitchUsbDriver,
} from "../api/commands";
import { normalizeTauriError } from "../api/errors";
import type { Probe, UsbDriverMode, DriverType } from "@shared/types";

function applyDriverOverrides(
  probes: Probe[],
  overrides: Record<string, DriverType>
): Probe[] {
  return probes.map((probe) => ({
    ...probe,
    driver: overrides[probe.id] ?? probe.driver,
  }));
}

function preserveSelection(
  probes: Probe[],
  selectedProbeId: string | null
): string | null {
  if (!selectedProbeId) return null;
  return probes.some((probe) => probe.id === selectedProbeId) ? selectedProbeId : null;
}

function resetUsbOperationStatus() {
  return {
    usbDriverStatus: "idle" as const,
    usbDriverMessage: "",
  };
}

interface ProbeState {
  probes: Probe[];
  driverOverrides: Record<string, DriverType>;
  isLoading: boolean;
  isFirmwareRefreshing: boolean;
  isRuntimeReady: boolean | null;
  runtimeLibPath: string | undefined;
  runtimeVersion: string;
  selectedProbeId: string | null;
  error: string | null;
  usbDriverStatus: "idle" | "switching" | "success" | "failed";
  usbDriverMessage: string;

  loadRuntimeAndProbes: () => Promise<void>;
  scanProbesSilent: () => Promise<void>;
  scanProbes: () => Promise<void>;
  selectProbe: (id: string | null) => void;
  switchUsbDriver: (probeId: string, mode: UsbDriverMode) => Promise<void>;
}

export const useProbeStore = create<ProbeState>((set, get) => ({
  probes: [],
  driverOverrides: {},
  isLoading: false,
  isFirmwareRefreshing: false,
  isRuntimeReady: null,
  runtimeLibPath: undefined,
  runtimeVersion: "",
  selectedProbeId: null,
  error: null,
  usbDriverStatus: "idle",
  usbDriverMessage: "",

  loadRuntimeAndProbes: async () => {
    set({ isLoading: true, error: null });
    try {
      const result = await detectAndScan();

      // Fresh scan from the bridge — drop UI-only driver overrides so USB DRIVER / button state
      // match JLINKARM_ReadEmuConfigMem again (stale WinUSB overrides used to keep the switch disabled).
      const probes = result.probes;
      const selectedProbeId = preserveSelection(probes, get().selectedProbeId);

      set({
        isRuntimeReady: result.status.ready,
        runtimeLibPath: result.status.nativeLibPath,
        runtimeVersion: result.status.version ?? "",
        probes,
        driverOverrides: {},
        isLoading: false,
        selectedProbeId,
        ...resetUsbOperationStatus(),
      });

      // If firmware strings are missing on the first scan (common right after Linux udev setup
      // or during cold start), retry silently a few times so the UI fills in without requiring
      // the user to hit Refresh.
      const needsFirmwareRetry = probes.some((p) => !p.firmware);
      if (result.status.ready && needsFirmwareRetry) {
        set({ isFirmwareRefreshing: true });
        const selectedBefore = get().selectedProbeId;
        for (const delayMs of [600, 1400, 2600]) {
          await new Promise((r) => setTimeout(r, delayMs));
          try {
            const cur = await scanProbes();
            const selectedProbeId2 = preserveSelection(cur, selectedBefore);
            set({ probes: cur, selectedProbeId: selectedProbeId2 });
            if (cur.every((p) => !!p.firmware)) break;
          } catch {
            // ignore and keep retrying
          }
        }
        set({ isFirmwareRefreshing: false });
      }
    } catch (err) {
      set({
        isRuntimeReady: false,
        error: normalizeTauriError(err),
        isLoading: false,
      });
    }
  },

  scanProbes: async () => {
    set({
      isLoading: true,
      error: null,
      ...resetUsbOperationStatus(),
    });
    try {
      const probes = await scanProbes();
      const selectedProbeId = preserveSelection(probes, get().selectedProbeId);
      set({
        probes,
        selectedProbeId,
        isLoading: false,
        driverOverrides: {},
      });
    } catch (err) {
      set({
        error: normalizeTauriError(err),
        isLoading: false,
      });
    }
  },

  scanProbesSilent: async () => {
    try {
      const probes = await scanProbes();
      const selectedProbeId = preserveSelection(probes, get().selectedProbeId);
      set({ probes, selectedProbeId, driverOverrides: {} });
    } catch { /* ignore */ }
  },

  selectProbe: (id) => {
    const current = get().selectedProbeId;
    set({
      selectedProbeId: current === id ? null : id,
      ...resetUsbOperationStatus(),
    });
  },

  switchUsbDriver: async (probeId, mode) => {
    set({
      usbDriverStatus: "switching",
      usbDriverMessage: mode === "winUsb"
        ? "Updating probe firmware and switching the USB driver to WinUSB..."
        : "Updating probe firmware and switching the USB driver to SEGGER...",
      error: null,
    });

    try {
      const currentProbes = get().probes;
      const probeIndex = currentProbes.findIndex((p) => p.id === probeId);
      if (probeIndex === -1) {
        set({
          usbDriverStatus: "failed",
          usbDriverMessage: "Selected probe was not found. Please refresh and try again.",
        });
        return;
      }
      const probe = currentProbes[probeIndex];
      const result = await invokeSwitchUsbDriver(
        probeIndex,
        mode,
        probe?.provider,
        probe?.serialNumber,
      );

      if (!result.success) {
        set({
          usbDriverStatus: "failed",
          usbDriverMessage: result.error ?? "Could not switch the USB driver.",
        });
        return;
      }

      const probes = get().probes;
      const expectedCount = probes.length; // after reboot, transient scans can be incomplete
      const probeRow = probes.find((p) => p.id === probeId);
      if (probeRow) {
        const newDriver: DriverType = mode === "winUsb" ? "WinUSB" : "SEGGER";
        set({
          driverOverrides: { ...get().driverOverrides, [probeRow.id]: newDriver },
          probes: probes.map((p) => (p.id === probeId ? { ...p, driver: newDriver } : p)),
        });
      }

      // After switching, the probe usually reboots. During this window, scanning may only
      // return a subset of probes (e.g. 1/2). Poll for a short time and keep the largest
      // scan result so the UI doesn't get stuck showing fewer rows.
      //
      // Important UX: never shrink the visible list during the reboot window.
      // Keep showing the pre-switch list until we get >= that many probes back.
      await new Promise((resolve) => setTimeout(resolve, 900));
      const overrides = get().driverOverrides;
      const selectedBefore = get().selectedProbeId;
      let best: Probe[] = get().probes;
      const deadline = Date.now() + 8000;
      while (Date.now() < deadline) {
        try {
          const cur = applyDriverOverrides(await scanProbes(), overrides);
          // Only update if we are not shrinking the list.
          if (cur.length >= best.length) {
            best = cur;
            const selectedProbeId = preserveSelection(best, selectedBefore);
            set({ probes: best, selectedProbeId });
            if (best.length >= expectedCount) break;
          }
        } catch {
          // ignore and retry
        }
        await new Promise((r) => setTimeout(r, 350));
      }

      // Re-read USB driver (and firmware) from the bridge so the WinUSB button matches hardware truth.
      try {
        const fresh = await scanProbes();
        if (fresh.length >= best.length) {
          set({
            probes: fresh,
            selectedProbeId: preserveSelection(fresh, selectedBefore),
            driverOverrides: {},
          });
        } else if (fresh.length > 0) {
          const byId = new Map(fresh.map((p) => [p.id, p]));
          const merged = best.map((p) => {
            const u = byId.get(p.id);
            return u ? { ...p, driver: u.driver, firmware: u.firmware ?? p.firmware } : p;
          });
          set({
            probes: merged,
            selectedProbeId: preserveSelection(merged, selectedBefore),
            driverOverrides: {},
          });
        } else {
          set({ driverOverrides: {} });
        }
      } catch {
        set({ driverOverrides: {} });
      }

      const unplug = " You may need to unplug and replug your probe to apply the configuration changes.";
      const withRebootBrief = " The probe may reboot briefly.";
      const winMsg =
        "Switched to WinUSB." +
        (result.rebootNotSupported ? "" : withRebootBrief) +
        unplug;
      const seggerMsg =
        "Switched to SEGGER." +
        (result.rebootNotSupported ? "" : withRebootBrief) +
        unplug;
      set({
        usbDriverStatus: "success",
        usbDriverMessage: mode === "winUsb" ? winMsg : seggerMsg,
      });
    } catch (err) {
      set({
        usbDriverStatus: "failed",
        usbDriverMessage: normalizeTauriError(err),
      });
    }
  },
}));
