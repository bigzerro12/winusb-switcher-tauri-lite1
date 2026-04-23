import { useCallback, useEffect, useState } from "react";
import { prepareBundledJlink } from "./api/commands";
import { normalizeTauriError, withPlatformSetupHint } from "./api/errors";
import { BootstrapError, BootstrapPending, ProbeAccessPending } from "./features/bootstrap/screens";
import { resizeHeightToContent } from "./lib/windowSizing";
import { useProbeStore } from "./store/probeStore";
import Dashboard from "./features/probes/Dashboard";

export default function App() {
  const { isInstalled, isLoading, isFirmwareRefreshing, checkInstallation } = useProbeStore();
  const [bootstrap, setBootstrap] = useState<"pending" | "ok" | "error">("pending");
  const [bootstrapError, setBootstrapError] = useState<string>("");

  const runBootstrap = useCallback(async () => {
    setBootstrap("pending");
    setBootstrapError("");
    try {
      await prepareBundledJlink();
      setBootstrap("ok");
    } catch (err) {
      setBootstrap("error");
      setBootstrapError(withPlatformSetupHint(normalizeTauriError(err)));
    }
  }, []);

  useEffect(() => {
    runBootstrap();
  }, [runBootstrap]);

  useEffect(() => {
    if (bootstrap !== "ok") return;
    checkInstallation().catch((err) => {
      console.error("[App] checkInstallation failed:", err);
    });
  }, [bootstrap, checkInstallation]);

  useEffect(() => {
    resizeHeightToContent();
  }, [bootstrap, isInstalled]);

  if (bootstrap === "pending") {
    return <BootstrapPending />;
  }

  if (bootstrap === "error") {
    return <BootstrapError error={bootstrapError} onRetry={() => void runBootstrap()} />;
  }

  if (isInstalled === null) {
    return <ProbeAccessPending isLoading={isLoading} isFirmwareRefreshing={isFirmwareRefreshing} />;
  }

  return <Dashboard />;
}
