/**
 * Stage a portable (no installer) tree and produce a .zip next to the release binary.
 *
 * Windows: exe + DLLs + resources/ at zip root (single folder inside the archive).
 * Linux: bin/<crate> + lib/<crate>/resources/... for Tauri resource_dir resolution.
 *
 * Re-runs stage-jlink-runtime-for-bundle for the given triple and excludes the dev
 * `resources/jlink-runtime/` tree so only `jlink-runtime-bundled/` (one arch) ships.
 *
 * Usage: node scripts/package-portable.mjs [--target <rustc-triple>]
 * Env: TAURI_ENV_TARGET_TRIPLE (optional) — same as --target
 */

import { execFileSync, spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "..");
const crateName = "jlink-winusb-switcher-lite";

/** Dev trees that must never ship inside portable zips (multi-arch / download staging). */
const EXCLUDE_RESOURCE_TOP_LEVEL = new Set(["jlink-runtime", "jlink"]);

function readJson(p) {
  return JSON.parse(fs.readFileSync(p, "utf8"));
}

function parseArgs(argv) {
  let target = process.env.TAURI_ENV_TARGET_TRIPLE || "";
  const out = [];
  for (let i = 0; i < argv.length; i++) {
    if (argv[i] === "--target" && argv[i + 1]) {
      target = argv[++i];
    } else {
      out.push(argv[i]);
    }
  }
  if (out.length) {
    console.warn("package-portable: ignoring extra args:", out.join(" "));
  }
  return String(target).trim();
}

function inferTripleFromPlatform() {
  const { platform, arch } = process;
  if (platform === "win32") {
    if (arch === "x64") return "x86_64-pc-windows-msvc";
    if (arch === "ia32") return "i686-pc-windows-msvc";
  }
  if (platform === "linux") {
    if (arch === "x64") return "x86_64-unknown-linux-gnu";
    if (arch === "ia32" || arch === "x32") return "i686-unknown-linux-gnu";
  }
  return "";
}

function resolveReleaseDir(triple) {
  const ta = path.join(repoRoot, "src-tauri", "target");
  if (triple) {
    const cross = path.join(ta, triple, "release");
    if (fs.existsSync(cross)) return cross;
  }
  const native = path.join(ta, "release");
  if (fs.existsSync(native)) return native;
  throw new Error(`Release dir not found (triple=${triple || "native"}): ${native}`);
}

function rmRf(dir) {
  if (fs.existsSync(dir)) fs.rmSync(dir, { recursive: true, force: true });
}

/**
 * Ensure jlink-runtime-bundled contains only the J-Link tree for `triple`.
 */
function restageJlinkForTriple(triple) {
  const script = path.join(repoRoot, "scripts", "stage-jlink-runtime-for-bundle.mjs");
  const env = { ...process.env, TAURI_ENV_TARGET_TRIPLE: triple };
  const r = spawnSync(process.execPath, [script], {
    cwd: repoRoot,
    env,
    stdio: "inherit",
  });
  if (r.error) throw r.error;
  if (r.status !== 0) {
    process.exit(r.status ?? 1);
  }
}

/**
 * Copy `src-tauri/resources` except dev-only top-level dirs (see EXCLUDE_RESOURCE_TOP_LEVEL).
 */
function copyResourcesPortable(destResourcesDir) {
  const src = path.join(repoRoot, "src-tauri", "resources");
  if (!fs.existsSync(src)) {
    throw new Error(`Missing resources dir: ${src}`);
  }
  fs.mkdirSync(destResourcesDir, { recursive: true });
  for (const ent of fs.readdirSync(src, { withFileTypes: true })) {
    if (EXCLUDE_RESOURCE_TOP_LEVEL.has(ent.name)) {
      console.log(`[package-portable] skip resources/${ent.name} (dev / multi-arch tree)`);
      continue;
    }
    const from = path.join(src, ent.name);
    const to = path.join(destResourcesDir, ent.name);
    if (ent.isDirectory()) {
      fs.cpSync(from, to, { recursive: true });
    } else {
      fs.copyFileSync(from, to);
    }
  }

  const bundled = path.join(destResourcesDir, "jlink-runtime-bundled");
  if (!fs.existsSync(bundled)) {
    throw new Error(
      `Portable staging missing jlink-runtime-bundled under ${destResourcesDir}. Did staging fail?`
    );
  }
}

function stageWindows(releaseDir, stagingRoot) {
  rmRf(stagingRoot);
  fs.mkdirSync(stagingRoot, { recursive: true });

  for (const ent of fs.readdirSync(releaseDir, { withFileTypes: true })) {
    if (!ent.isFile()) continue;
    const n = ent.name;
    if (n.endsWith(".exe") || n.endsWith(".dll")) {
      fs.copyFileSync(path.join(releaseDir, n), path.join(stagingRoot, n));
    }
  }

  const exes = fs.readdirSync(stagingRoot).filter((f) => f.endsWith(".exe"));
  if (exes.length === 0) {
    throw new Error(`No .exe found under ${releaseDir}`);
  }

  copyResourcesPortable(path.join(stagingRoot, "resources"));
}

function stageLinux(releaseDir, stagingRoot) {
  rmRf(stagingRoot);
  const binDir = path.join(stagingRoot, "bin");
  const libRes = path.join(stagingRoot, "lib", crateName, "resources");
  fs.mkdirSync(binDir, { recursive: true });
  fs.mkdirSync(libRes, { recursive: true });

  const binSrc = path.join(releaseDir, crateName);
  if (!fs.existsSync(binSrc)) {
    throw new Error(`Missing Linux binary: ${binSrc}`);
  }
  fs.copyFileSync(binSrc, path.join(binDir, crateName));
  try {
    fs.chmodSync(path.join(binDir, crateName), 0o755);
  } catch {
    /* windows FS or CI */
  }

  copyResourcesPortable(libRes);
}

function zipTree(folderToZip, zipFile) {
  const absFolder = path.resolve(folderToZip);
  const absZip = path.resolve(zipFile);
  fs.mkdirSync(path.dirname(absZip), { recursive: true });
  if (fs.existsSync(absZip)) fs.rmSync(absZip);

  if (process.platform === "win32") {
    const inner = path.basename(absFolder);
    const parent = path.dirname(absFolder);
    const cmd = `Compress-Archive -LiteralPath '${inner.replace(/'/g, "''")}' -DestinationPath '${absZip.replace(/'/g, "''")}' -Force`;
    execFileSync("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command", cmd], {
      cwd: parent,
      stdio: "inherit",
    });
  } else {
    const inner = path.basename(absFolder);
    const parent = path.dirname(absFolder);
    execFileSync("zip", ["-r", "-q", absZip, inner], { cwd: parent, stdio: "inherit" });
  }
}

function main() {
  let triple = parseArgs(process.argv.slice(2));
  if (!triple) {
    triple = inferTripleFromPlatform();
  }
  if (!triple) {
    throw new Error(
      "package-portable: pass --target <rustc-triple> or set TAURI_ENV_TARGET_TRIPLE (could not infer from this OS)."
    );
  }

  const pkg = readJson(path.join(repoRoot, "package.json"));
  const version = pkg.version || "0.0.0";
  const triLabel = triple.replace(/[^a-zA-Z0-9._-]+/g, "_");

  console.log(`[package-portable] triple=${triple} (re-staging J-Link bundle)`);
  restageJlinkForTriple(triple);

  const releaseDir = resolveReleaseDir(triple);

  const outDir = path.join(repoRoot, "release-zips");
  const folderName = `J-Link-WinUSB-Switcher-${version}-${triLabel}-portable`;
  const staging = path.join(outDir, ".staging", folderName);
  const zipPath = path.join(outDir, `${folderName}.zip`);

  if (process.platform === "win32") {
    stageWindows(releaseDir, staging);
  } else {
    stageLinux(releaseDir, staging);
  }

  zipTree(staging, zipPath);
  rmRf(path.join(outDir, ".staging"));

  console.log("Wrote", zipPath);
}

main();
