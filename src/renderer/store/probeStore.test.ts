import { beforeEach, describe, expect, it, vi } from "vitest";
import type { DetectAndScanResult, Probe, UsbDriverResult } from "@shared/types";

vi.mock("../api/commands", () => ({
  detectAndScan: vi.fn(),
  scanProbes: vi.fn(),
  switchUsbDriver: vi.fn(),
}));

import {
  detectAndScan,
  scanProbes as invokeScanProbes,
  switchUsbDriver as invokeSwitchUsbDriver,
} from "../api/commands";
import { useProbeStore } from "./probeStore";

const detectAndScanMock = vi.mocked(detectAndScan);
const scanProbesMock = vi.mocked(invokeScanProbes);
const switchUsbDriverMock = vi.mocked(invokeSwitchUsbDriver);

function probe(overrides: Partial<Probe> = {}): Probe {
  return {
    id: "123456789",
    serialNumber: "123456789",
    productName: "J-Link",
    nickName: "Main Probe",
    provider: "JLink",
    connection: "USB",
    driver: "SEGGER",
    firmware: "Sep 29 2020 12:34:56",
    ...overrides,
  };
}

function detectResult(probes: Probe[]): DetectAndScanResult {
  return {
    status: { ready: true, nativeLibPath: "/tmp/libjlinkarm.so", version: "V10.52" },
    probes,
  };
}

function resetStore() {
  useProbeStore.setState({
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
    loadRequestId: 0,
    scanRequestId: 0,
    switchRequestId: 0,
  });
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((res) => {
    resolve = res;
  });
  return { promise, resolve };
}

describe("probeStore", () => {
  beforeEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
    resetStore();
  });

  it("loads runtime/probes and preserves selected probe when present", async () => {
    const p = probe();
    detectAndScanMock.mockResolvedValue(detectResult([p]));
    useProbeStore.setState({ selectedProbeId: p.id });

    await useProbeStore.getState().loadRuntimeAndProbes();
    const state = useProbeStore.getState();

    expect(state.isRuntimeReady).toBe(true);
    expect(state.runtimeVersion).toBe("V10.52");
    expect(state.probes).toEqual([p]);
    expect(state.selectedProbeId).toBe(p.id);
    expect(state.driverOverrides).toEqual({});
  });

  it("switch flow keeps list stable through reboot window and ends in success", async () => {
    vi.useFakeTimers();
    const original = [probe({ id: "sn-1", serialNumber: "111" }), probe({ id: "sn-2", serialNumber: "222" })];
    useProbeStore.setState({ probes: original, selectedProbeId: "sn-1" });

    const switchResult: UsbDriverResult = { success: true, rebootNotSupported: false };
    switchUsbDriverMock.mockResolvedValue(switchResult);
    scanProbesMock
      .mockResolvedValueOnce([
        // First polling read (do not shrink visible list).
        probe({ id: "sn-1", serialNumber: "111", driver: "WinUSB" }),
      ])
      .mockResolvedValueOnce([
        // Final consistency read.
        probe({ id: "sn-1", serialNumber: "111", driver: "WinUSB" }),
        probe({ id: "sn-2", serialNumber: "222", driver: "SEGGER" }),
      ]);

    const run = useProbeStore.getState().switchUsbDriver("sn-1", "winUsb");
    await vi.runAllTimersAsync();
    await run;

    const state = useProbeStore.getState();
    expect(state.usbDriverStatus).toBe("success");
    expect(state.probes).toHaveLength(2);
    expect(state.probes[0].driver).toBe("WinUSB");
    expect(state.driverOverrides).toEqual({});
  });

  it("ignores stale loadRuntimeAndProbes responses", async () => {
    const first = deferred<DetectAndScanResult>();
    const second = deferred<DetectAndScanResult>();
    detectAndScanMock
      .mockImplementationOnce(() => first.promise)
      .mockImplementationOnce(() => second.promise);

    const run1 = useProbeStore.getState().loadRuntimeAndProbes();
    const run2 = useProbeStore.getState().loadRuntimeAndProbes();

    second.resolve(detectResult([probe({ id: "newer" })]));
    await run2;
    first.resolve(detectResult([probe({ id: "older" })]));
    await run1;

    const state = useProbeStore.getState();
    expect(state.probes).toHaveLength(1);
    expect(state.probes[0].id).toBe("newer");
  });
});
