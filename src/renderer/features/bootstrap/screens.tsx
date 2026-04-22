function isLinux() {
  return navigator.userAgent.toLowerCase().includes("linux");
}

export function BootstrapPending() {
  return (
    <div className="container">
      <div className="app-card flex items-center justify-center">
        <div className="w-full max-w-[520px] px-6 py-10">
          <h1 className="text-center text-base font-semibold tracking-tight text-slate-800">
            Initializing WinUSB Switcher Lite
          </h1>
          <p className="mt-3 text-center text-[13px] leading-relaxed text-slate-600">
            Preparing the bundled J-Link runtime and probe access. This usually completes in under a minute.
          </p>
          <p className="mt-5 text-center text-xs text-slate-500">
            {isLinux()
              ? "Administrator authorization may be requested to complete setup."
              : "Please keep this window open."}
          </p>
          <div
            className="mt-8 h-1 overflow-hidden rounded-full bg-slate-100"
            role="progressbar"
            aria-label="Setup in progress"
          >
            <div className="bootstrap-lite-progress h-full w-2/5 rounded-full bg-slate-500/85" />
          </div>
          <style>{`
            .bootstrap-lite-progress {
              animation: bootstrapLiteShimmer 1.35s ease-in-out infinite;
            }
            @keyframes bootstrapLiteShimmer {
              0% { transform: translateX(-120%); }
              100% { transform: translateX(320%); }
            }
          `}</style>
        </div>
      </div>
    </div>
  );
}

export function BootstrapError(props: { error: string; onRetry: () => void }) {
  return (
    <div className="container">
      <div className="app-card flex items-center justify-center">
        <div className="w-full max-w-[520px] px-6 py-10">
          <h1 className="text-center text-[15px] font-semibold tracking-tight text-red-900">
            Setup could not finish
          </h1>
          <p className="mt-3 text-center text-[13px] leading-relaxed text-red-800/90 break-words">
            {props.error}
          </p>
          <div className="mt-8 flex justify-center">
            <button type="button" className="btn btn-primary" onClick={props.onRetry}>
              Try again
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

export function ProbeAccessPending(props: { isLoading: boolean; isFirmwareRefreshing: boolean }) {
  const primary = props.isLoading ? "Scanning connected probes…" : "Checking J-Link runtime…";
  const secondary = props.isFirmwareRefreshing
    ? "Reading probe firmware details. Some probes may take a few seconds to respond after first launch or after permissions changes."
    : "This can take longer on first launch or if multiple probes are connected.";

  return (
    <div className="container">
      <div className="app-card flex items-center justify-center">
        <div className="w-full max-w-[560px] px-6 py-10">
          <h1 className="text-center text-base font-semibold tracking-tight text-slate-800">
            Preparing probe access
          </h1>
          <p className="mt-3 text-center text-[13px] leading-relaxed text-slate-600">
            {primary}
          </p>
          <p className="mt-3 text-center text-xs leading-relaxed text-slate-500">
            {secondary}
          </p>
          <p className="mt-5 text-center text-xs text-slate-500">
            {isLinux()
              ? "On Linux, device permissions (udev rules) may be installed on first run and you may be prompted for administrator authorization."
              : "Please keep this window open while setup completes."}
          </p>
          <div
            className="mt-8 h-1 overflow-hidden rounded-full bg-slate-100"
            role="progressbar"
            aria-label="Probe setup in progress"
          >
            <div className="bootstrap-lite-progress h-full w-2/5 rounded-full bg-slate-500/85" />
          </div>
          <style>{`
            .bootstrap-lite-progress {
              animation: bootstrapLiteShimmer 1.35s ease-in-out infinite;
            }
            @keyframes bootstrapLiteShimmer {
              0% { transform: translateX(-120%); }
              100% { transform: translateX(320%); }
            }
          `}</style>
        </div>
      </div>
    </div>
  );
}

