/**
 * Stage a portable (no installer) tree and produce a .zip next to the release binary.
 *
 * Windows: exe + DLLs + resources/ at zip root (single folder inside the archive).
 * Linux: bin/<crate> + lib/<crate>/resources/... for Tauri resource_dir resolution.
 *
 * Usage: node scripts/package-portable.mjs [--target <rustc-triple>]
 * Env: TAURI_ENV_TARGET_TRIPLE (optional) — same as --target
 */

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "..");
const crateName = "jlink-winusb-switcher-lite";

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
  return target;
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

function copyResources(src, dest) {
  if (!fs.existsSync(src)) {
    throw new Error(`Missing resources dir: ${src}`);
  }
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  fs.cpSync(src, dest, { recursive: true });
}

function stageWindows(releaseDir, resourcesSrc, stagingRoot) {
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

  copyResources(resourcesSrc, path.join(stagingRoot, "resources"));
}

function stageLinux(releaseDir, resourcesSrc, stagingRoot) {
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

  copyResources(resourcesSrc, libRes);
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
  const triple = parseArgs(process.argv.slice(2));
  const pkg = readJson(path.join(repoRoot, "package.json"));
  const version = pkg.version || "0.0.0";
  const triLabel = triple || (process.platform === "win32" ? "x86_64-pc-windows-msvc" : "x86_64-unknown-linux-gnu");

  const releaseDir = resolveReleaseDir(triple);
  const resourcesSrc = path.join(repoRoot, "src-tauri", "resources");

  const outDir = path.join(repoRoot, "release-zips");
  const folderName = `J-Link-WinUSB-Switcher-${version}-${triLabel.replace(/[^a-zA-Z0-9._-]+/g, "_")}-portable`;
  const staging = path.join(outDir, ".staging", folderName);
  const zipPath = path.join(outDir, `${folderName}.zip`);

  if (process.platform === "win32") {
    stageWindows(releaseDir, resourcesSrc, staging);
  } else {
    stageLinux(releaseDir, resourcesSrc, staging);
  }

  zipTree(staging, zipPath);
  rmRf(path.join(outDir, ".staging"));

  console.log("Wrote", zipPath);
}

main();
