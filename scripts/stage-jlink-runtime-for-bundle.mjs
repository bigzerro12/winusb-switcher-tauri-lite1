#!/usr/bin/env node
/**
 * Populates src-tauri/resources/jlink-runtime-bundled/ with only the J-Link
 * runtime tree for the active Tauri target (windows-64/32, linux-64/32).
 *
 * Reads TAURI_ENV_TARGET_TRIPLE when present (tauri build / beforeBuildCommand).
 * macOS / universal targets get a stub tree so bundle globs still match.
 *
 * Dev (`tauri dev`) continues to use the full src-tauri/resources/jlink-runtime/
 * tree via bundled_jlink debug paths.
 */
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(__dirname, "..");
const tauriDir = path.join(root, "src-tauri");
const srcRuntime = path.join(tauriDir, "resources", "jlink-runtime");
const destRuntime = path.join(tauriDir, "resources", "jlink-runtime-bundled");

const JLINK_VERSION_PREFIX = "jlink-v";
const GLOB_ANCHOR = ".tauri-bundle-glob-anchor";

/** Clear staged payloads but keep the committed Tauri glob anchor file. */
function cleanDestStagedPayloads() {
  mkdirp(destRuntime);
  if (!fs.existsSync(destRuntime)) return;
  for (const ent of fs.readdirSync(destRuntime)) {
    if (ent === GLOB_ANCHOR) continue;
    fs.rmSync(path.join(destRuntime, ent), { recursive: true, force: true });
  }
}

function mkdirp(p) {
  fs.mkdirSync(p, { recursive: true });
}

function copyDir(from, to) {
  mkdirp(to);
  for (const ent of fs.readdirSync(from, { withFileTypes: true })) {
    const s = path.join(from, ent.name);
    const d = path.join(to, ent.name);
    if (ent.isDirectory()) copyDir(s, d);
    else fs.copyFileSync(s, d);
  }
}

function inferTripleFromNode() {
  const { platform, arch } = process;
  if (platform === "win32") {
    if (arch === "x64") return "x86_64-pc-windows-msvc";
    if (arch === "ia32") return "i686-pc-windows-msvc";
  }
  if (platform === "linux") {
    if (arch === "x64") return "x86_64-unknown-linux-gnu";
    if (arch === "ia32" || arch === "x32") return "i686-unknown-linux-gnu";
  }
  if (platform === "darwin") {
    return arch === "arm64" ? "aarch64-apple-darwin" : "x86_64-apple-darwin";
  }
  return null;
}

function getTriple() {
  const raw = process.env.TAURI_ENV_TARGET_TRIPLE;
  if (raw && String(raw).trim()) return String(raw).trim();
  const t = inferTripleFromNode();
  if (t) return t;
  throw new Error(
    "stage-jlink-runtime-for-bundle: set TAURI_ENV_TARGET_TRIPLE or run from a supported host."
  );
}

/**
 * @param {string} triple
 * @returns {{ platform: string } | null} null → macOS/universal stub only
 */
function mapTripleToRuntimePlatform(triple) {
  const t = triple.toLowerCase();

  if (t === "universal-apple-darwin" || t.includes("apple-darwin")) {
    return null;
  }

  if (t.includes("windows") || t.includes("pc-windows-msvc")) {
    if (t.startsWith("i686") || t.startsWith("i586")) return { platform: "windows-32" };
    return { platform: "windows-64" };
  }

  if (t.includes("linux")) {
    if (t.startsWith("i686") || t.startsWith("i586")) return { platform: "linux-32" };
    return { platform: "linux-64" };
  }

  throw new Error(`stage-jlink-runtime-for-bundle: unsupported triple ${triple}`);
}

function listVersionDirs() {
  if (!fs.existsSync(srcRuntime)) return [];
  return fs
    .readdirSync(srcRuntime, { withFileTypes: true })
    .filter((e) => e.isDirectory() && e.name.startsWith(JLINK_VERSION_PREFIX))
    .map((e) => e.name);
}

function stageMacStub() {
  cleanDestStagedPayloads();
  const note = path.join(destRuntime, "BUNDLED_RUNTIME_SKIPPED_ON_MACOS.txt");
  fs.writeFileSync(
    note,
    "macOS builds do not ship the in-repo jlink-runtime tree in this configuration.\n",
    "utf8"
  );
  console.log("[stage-jlink-runtime-for-bundle] macOS/universal: stub only.");
}

const triple = getTriple();
console.log(
  `[stage-jlink-runtime-for-bundle] triple=${triple} TAURI_ENV_TARGET_TRIPLE=${process.env.TAURI_ENV_TARGET_TRIPLE || ""}`
);

const mapped = mapTripleToRuntimePlatform(triple);

if (mapped === null) {
  stageMacStub();
  process.exit(0);
}

const { platform } = mapped;
cleanDestStagedPayloads();

let copied = false;
for (const vd of listVersionDirs()) {
  const from = path.join(srcRuntime, vd, platform);
  if (fs.existsSync(from) && fs.statSync(from).isDirectory()) {
    copyDir(from, path.join(destRuntime, vd, platform));
    copied = true;
    console.log(`[stage-jlink-runtime-for-bundle] copied ${vd}/${platform}`);
  }
}

const unversioned = path.join(srcRuntime, platform);
if (fs.existsSync(unversioned) && fs.statSync(unversioned).isDirectory()) {
  copyDir(unversioned, path.join(destRuntime, platform));
  copied = true;
  console.log(`[stage-jlink-runtime-for-bundle] copied ${platform} (unversioned layout)`);
}

if (!copied) {
  console.error(
    `[stage-jlink-runtime-for-bundle] No J-Link runtime found for ${platform} under ${srcRuntime}.`
  );
  console.error("Expected e.g. jlink-v936/windows-64/ or windows-64/ with DLL/.so + Firmwares/.");
  process.exit(1);
}
