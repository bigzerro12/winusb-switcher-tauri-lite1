/**
 * Picks a free TCP port (starting at 5173), writes a tiny Tauri config merge with
 * matching devUrl, sets VITE_DEV_PORT for Vite, then runs `yarn tauri dev`.
 *
 * Fixes "Port 5173 is already in use" when another Vite/Tauri session (or any
 * process) still holds 5173 — including on Windows where the stale listener
 * is often on `::1:5173` (IPv6 loopback).
 */

import { spawnSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import net from "node:net";
import { platform } from "node:os";
import { resolve } from "node:path";

const PREFERRED = 5173;
const APP_EXE_BASENAME = "winusb-switcher-lite";

/**
 * Kill any previously-launched dev build of this app. Cargo can't overwrite
 * the `.exe` on Windows while a prior run is still alive, which produces:
 *   error: failed to remove file `...\target\debug\winusb-switcher-lite.exe`
 *   Caused by: Access is denied. (os error 5)
 */
function killStaleAppProcesses() {
  try {
    if (platform() === "win32") {
      const r = spawnSync(
        "taskkill",
        ["/F", "/IM", `${APP_EXE_BASENAME}.exe`, "/T"],
        { stdio: "pipe", shell: false },
      );
      if (r.status === 0) {
        console.log(`[tauri:dev] killed stale ${APP_EXE_BASENAME}.exe`);
      }
    } else {
      spawnSync("pkill", ["-f", APP_EXE_BASENAME], { stdio: "ignore" });
    }
  } catch {
    // Best-effort; cargo will surface a clear error if a lock persists.
  }
}

killStaleAppProcesses();

// Probe the same hosts Vite resolves `localhost` to, so we detect IPv4 and
// IPv6 loopback binds reliably on Windows (Node's default listen() binds
// to `::` which does NOT conflict with an existing bind on `::1`).
const PROBE_HOSTS = ["127.0.0.1", "::1"];

/**
 * @param {number} port
 * @param {string} host
 */
function tryBind(port, host) {
  return new Promise((resolvePromise) => {
    const s = net.createServer();
    s.unref();
    s.once("error", () => resolvePromise(false));
    s.listen({ port, host, exclusive: true }, () => {
      s.close(() => resolvePromise(true));
    });
  });
}

/** @param {number} port */
async function isFree(port) {
  for (const host of PROBE_HOSTS) {
    if (!(await tryBind(port, host))) return false;
  }
  return true;
}

/** @param {number} start */
async function pickPort(start) {
  for (let p = start; p < start + 100; p++) {
    if (await isFree(p)) return p;
  }
  throw new Error(`No free TCP port found in range ${start}-${start + 99}`);
}

const port = await pickPort(PREFERRED);
console.log(`[tauri:dev] selected dev port ${port}`);

const autogenPath = resolve("src-tauri/tauri.conf.autogen.json");
writeFileSync(
  autogenPath,
  `${JSON.stringify({ build: { devUrl: `http://127.0.0.1:${port}` } }, null, 2)}\n`,
  "utf8",
);

process.env.VITE_DEV_PORT = String(port);
// Force Vite's dev server onto the IPv4 loopback so the host resolution can't
// drift between `127.0.0.1` and `::1` and race with stale listeners.
process.env.VITE_DEV_HOST = "127.0.0.1";

const result = spawnSync(
  "yarn",
  [
    "tauri",
    "dev",
    "-c",
    "./src-tauri/tauri.conf.dev.json",
    "-c",
    "./src-tauri/tauri.conf.autogen.json",
  ],
  {
    stdio: "inherit",
    env: process.env,
    shell: true,
    cwd: resolve("."),
  },
);

if (result.error) throw result.error;
process.exit(result.status === null ? 1 : result.status);
