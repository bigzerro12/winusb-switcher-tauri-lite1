#!/usr/bin/env node
/**
 * Copies only the J-Link zip that matches the current Tauri build target into
 * src-tauri/resources/jlink/<os>/<arch>/ so installers do not bundle every OS.
 *
 * Canonical payloads (tracked in Git LFS) live under src-tauri/jlink-bundles/.
 * The staged tree under src-tauri/resources/jlink/ is gitignored.
 */
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(__dirname, "..");
const tauriDir = path.join(root, "src-tauri");
const bundlesDir = path.join(tauriDir, "jlink-bundles");
const stageRoot = path.join(tauriDir, "resources", "jlink");
const zipName = "JLink_V930a.zip";

function rimraf(p) {
  if (fs.existsSync(p)) fs.rmSync(p, { recursive: true, force: true });
}

function copyZip(src, dst) {
  fs.mkdirSync(path.dirname(dst), { recursive: true });
  fs.copyFileSync(src, dst);
}

function inferTripleFromNode() {
  const { platform, arch } = process;
  if (platform === "win32") {
    if (arch === "x64") return "x86_64-pc-windows-msvc";
    if (arch === "arm64") return "aarch64-pc-windows-msvc";
  }
  if (platform === "linux") {
    if (arch === "x64") return "x86_64-unknown-linux-gnu";
    if (arch === "arm64") return "aarch64-unknown-linux-gnu";
  }
  return null;
}

function getTriple() {
  const raw = process.env.TAURI_ENV_TARGET_TRIPLE;
  if (raw && String(raw).trim()) return String(raw).trim();

  const t = inferTripleFromNode();
  if (t) return t;

  const hint =
    process.platform === "darwin"
      ? " macOS bundle targets are not supported for this project."
      : "";
  throw new Error(
    `Cannot determine build target. Set TAURI_ENV_TARGET_TRIPLE or run on Windows or Linux.${hint}`
  );
}

/**
 * @param {string} triple
 * @returns {{ os: string, archDir: string }}
 */
function mapTripleToBundleLayout(triple) {
  const t = triple.toLowerCase();

  if (t === "universal-apple-darwin" || t.includes("apple-darwin")) {
    throw new Error(
      "stage-jlink: macOS is not a supported target (no J-Link zip layout for Darwin). Build Windows or Linux only."
    );
  }

  if (t.includes("windows") || t.includes("pc-windows-msvc")) {
    if (t.startsWith("aarch64")) return { os: "windows", archDir: "aarch64" };
    return { os: "windows", archDir: "x86_64" };
  }

  if (t.includes("linux")) {
    if (t.startsWith("aarch64")) return { os: "linux", archDir: "aarch64" };
    return { os: "linux", archDir: "x86_64" };
  }

  throw new Error(`stage-jlink: unsupported triple ${triple}`);
}

const triple = getTriple();
const mapped = mapTripleToBundleLayout(triple);

console.log(
  `[stage-jlink] triple=${triple} (TAURI_ENV_TARGET_TRIPLE=${process.env.TAURI_ENV_TARGET_TRIPLE || ""})`
);

rimraf(stageRoot);

const src = path.join(bundlesDir, mapped.os, mapped.archDir, zipName);
const dst = path.join(stageRoot, mapped.os, mapped.archDir, zipName);

if (!fs.existsSync(src)) {
  console.error(`[stage-jlink] Missing source zip: ${src}`);
  console.error(
    "[stage-jlink] Add the matching J-Link payload under jlink-bundles/ or build on a supported host."
  );
  process.exit(1);
}

copyZip(src, dst);
console.log(`[stage-jlink] Staged ${path.relative(root, dst)}`);
