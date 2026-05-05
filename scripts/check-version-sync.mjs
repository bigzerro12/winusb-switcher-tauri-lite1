import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const read = (p) => fs.readFileSync(path.join(root, p), "utf8");

const pkgVersion = JSON.parse(read("package.json")).version;
const tauriConfVersion = JSON.parse(read("src-tauri/tauri.conf.json")).version;
const cargoToml = read("src-tauri/Cargo.toml");
const cargoMatch = cargoToml.match(/^version\s*=\s*"([^"]+)"/m);

if (!cargoMatch) {
  console.error("Unable to parse version from src-tauri/Cargo.toml");
  process.exit(1);
}

const cargoVersion = cargoMatch[1];
const versions = {
  "package.json": pkgVersion,
  "src-tauri/tauri.conf.json": tauriConfVersion,
  "src-tauri/Cargo.toml": cargoVersion,
};

const unique = new Set(Object.values(versions));
if (unique.size !== 1) {
  console.error("Version mismatch detected:");
  for (const [file, value] of Object.entries(versions)) {
    console.error(`- ${file}: ${value}`);
  }
  process.exit(1);
}

console.log(`Version sync ok: ${pkgVersion}`);
