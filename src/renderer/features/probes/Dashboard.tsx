import type { Probe, DriverType } from "@shared/types";
import { useProbeStore } from "../../store/probeStore";

function ProbeTable(props: {
  probes: Probe[];
  selectedProbeId: string | null;
  onSelectProbe: (id: string) => void;
}) {
  const getDriverBadge = (driver: DriverType) => {
    switch (driver) {
      case "SEGGER":
        return <span className="driver-badge driver-segger">SEGGER</span>;
      case "WinUSB":
        return <span className="driver-badge driver-winusb">WinUSB</span>;
      default:
        return <span className="driver-badge driver-unknown">Unknown</span>;
    }
  };

  return (
    <table id="probe-table" className="probe-table">
      <thead>
        <tr>
          <th>SERIAL</th>
          <th>PRODUCT</th>
          <th>NICKNAME</th>
          <th>CONNECTION</th>
          <th>USB DRIVER</th>
          <th>PROBE FIRMWARE</th>
        </tr>
      </thead>
      <tbody>
        {props.probes.length > 0 ? (
          props.probes.map((probe) => (
            <tr
              key={probe.id}
              onClick={() => props.onSelectProbe(probe.id)}
              className={`probe-row ${props.selectedProbeId === probe.id ? "selected" : ""}`}
            >
              <td>{probe.serialNumber}</td>
              <td>{probe.productName}</td>
              <td>
                {probe.nickName && probe.nickName !== "<not set>" ? (
                  probe.nickName
                ) : (
                  <span style={{ color: "#adb5bd", fontStyle: "italic" }}>&lt;not set&gt;</span>
                )}
              </td>
              <td>
                <div className="connection-status">
                  <span className="connection-dot"></span>
                  {probe.connection}
                </div>
              </td>
              <td>{getDriverBadge(probe.driver)}</td>
              <td>
                {probe.firmware ? (
                  <span className="firmware-date">{probe.firmware}</span>
                ) : (
                  <span className="firmware-unknown">—</span>
                )}
              </td>
            </tr>
          ))
        ) : (
          <tr>
            <td colSpan={6} className="text-center text-gray-500 py-8">
              No J-Link probes detected
            </td>
          </tr>
        )}
      </tbody>
    </table>
  );
}

export default function Dashboard() {
  const {
    probes,
    isLoading,
    isFirmwareRefreshing,
    isRuntimeReady,
    runtimeVersion,
    selectedProbeId,
    error,
    scanProbes,
    selectProbe,
    switchUsbDriver,
    usbDriverStatus,
    usbDriverMessage,
  } = useProbeStore();

  const selectedProbe = probes.find((p) => p.id === selectedProbeId);
  const isProbeBusy = usbDriverStatus === "switching";
  const canSwitchToWinUSB =
    !!selectedProbe &&
    selectedProbe.driver !== "WinUSB" &&
    !isProbeBusy &&
    !isLoading &&
    !isFirmwareRefreshing;

  const handleRefresh = () => scanProbes();

  const handleSwitchToWinUSB = async () => {
    if (!selectedProbe) return;
    await switchUsbDriver(selectedProbe.id, "winUsb");
  };

  return (
    <div className="container">
      <div className="app-card">
        <header className="app-header">
          <h1>WinUSB Switcher</h1>
        </header>

        {error && <div className="error-message">{error}</div>}

        <section className="software-section">
          <h2>J-LINK RUNTIME</h2>
          <div className="software-info">
            <div className="software-details">
              <div className="software-version">{runtimeVersion || "SEGGER J-Link (version unknown)"}</div>
              <div className="software-status">
                <span className={`status-indicator ${isRuntimeReady ? "detected" : "error"}`}></span>
                <span className="status-text">{isRuntimeReady ? "Ready" : "Not ready"}</span>
              </div>
            </div>
          </div>
        </section>

        <section className="probes-section">
          <h2>CONNECTED J-LINK PROBES</h2>
          <div className="probes-info">
            <div className="probes-status">
              <span className={`status-indicator ${probes.length > 0 ? "detected" : "error"}`}></span>
              <span className="status-text">{probes.length > 0 ? `${probes.length} detected` : "No probes found"}</span>
            </div>
            <div className="probes-selection">
              {selectedProbe
                ? `Selected: ${selectedProbe.serialNumber} - ${selectedProbe.productName}`
                : "No probe selected"}
            </div>
            <button
              id="refresh-button"
              onClick={handleRefresh}
              disabled={isLoading || isProbeBusy}
              className="btn btn-secondary"
            >
              {isLoading ? "Scanning..." : "Refresh list"}
            </button>
          </div>

          {isFirmwareRefreshing && (
            <div style={{ marginTop: "10px", fontSize: "12px", color: "#6b7280" }}>
              Reading probe firmware… this may take a few seconds after first launch or after permissions changes.
            </div>
          )}

          <ProbeTable probes={probes} selectedProbeId={selectedProbeId} onSelectProbe={selectProbe} />
        </section>

        <section className="driver-section">
          <h2>DRIVER CONFIGURATION</h2>
          <div className="driver-info">
            <div className="driver-actions">
              <button
                id="switch-button"
                onClick={handleSwitchToWinUSB}
                disabled={!canSwitchToWinUSB}
                className="btn btn-primary"
              >
                {usbDriverStatus === "switching" ? "Switching..." : "Switch to WinUSB"}
              </button>
            </div>

            {usbDriverStatus !== "idle" && usbDriverMessage && (
              <div
                style={{
                  marginTop: "12px",
                  padding: "10px 14px",
                  borderRadius: "6px",
                  backgroundColor:
                    usbDriverStatus === "success" ? "#d4edda" : usbDriverStatus === "failed" ? "#fff0f0" : "#f8f9fa",
                  border: `1px solid ${
                    usbDriverStatus === "success" ? "#c3e6cb" : usbDriverStatus === "failed" ? "#ffcccc" : "#e9ecef"
                  }`,
                  fontSize: "13px",
                  color: usbDriverStatus === "success" ? "#155724" : usbDriverStatus === "failed" ? "#721c24" : "#495057",
                }}
              >
                {usbDriverStatus === "success" && `✅ ${usbDriverMessage}`}
                {usbDriverStatus === "failed" && `❌ ${usbDriverMessage}`}
                {usbDriverStatus === "switching" && `⏳ ${usbDriverMessage}`}
              </div>
            )}

            <div className="driver-note">
              {selectedProbe
                ? selectedProbe.driver === "WinUSB"
                  ? "This probe is currently using WinUSB."
                  : selectedProbe.driver === "SEGGER"
                    ? "This probe is currently using SEGGER."
                    : "The current driver is unknown. You can still try switching."
                : "Select a probe from the list, then click “Switch to WinUSB” to change its USB driver to WinUSB mode."}
            </div>
          </div>
        </section>
      </div>
    </div>
  );
}

