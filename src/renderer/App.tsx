import { useCallback, useEffect, useState } from "react";
import { prepareBundledJlink } from "./api/commands";
import { normalizeTauriError, withPlatformSetupHint } from "./api/errors";
import { BootstrapError, BootstrapPending, ProbeAccessPending } from "./features/bootstrap/screens";
import { resizeHeightToContent } from "./lib/windowSizing";
import { useProbeStore } from "./store/probeStore";
import Dashboard from "./features/probes/Dashboard";

export default function App() {
  const { isRuntimeReady, isLoading, isFirmwareRefreshing, loadRuntimeAndProbes } = useProbeStore();
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
    loadRuntimeAndProbes().catch((err) => {
      console.error("[App] loadRuntimeAndProbes failed:", err);
    });
  }, [bootstrap, loadRuntimeAndProbes]);

  useEffect(() => {
    resizeHeightToContent();
  }, [bootstrap, isRuntimeReady]);

  if (bootstrap === "pending") {
    return <BootstrapPending />;
  }

  if (bootstrap === "error") {
    return <BootstrapError error={bootstrapError} onRetry={() => void runBootstrap()} />;
  }

  if (isRuntimeReady === null) {
    return <ProbeAccessPending isLoading={isLoading} isFirmwareRefreshing={isFirmwareRefreshing} />;
  }

  return <Dashboard />;
}
