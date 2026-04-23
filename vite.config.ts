import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "path";

const fromEnv = Number(process.env.VITE_DEV_PORT);
const devPort =
  Number.isFinite(fromEnv) && fromEnv > 0 && fromEnv < 65536 ? fromEnv : 5173;
const devHost = process.env.VITE_DEV_HOST || "127.0.0.1";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@shared": resolve(__dirname, "src/shared"),
    },
  },
  // `yarn tauri:dev` sets VITE_DEV_PORT / VITE_DEV_HOST to a free port on the
  // IPv4 loopback and merges devUrl via tauri.conf.autogen.json. Plain
  // `yarn dev` keeps the default 5173 on 127.0.0.1.
  server: {
    host: devHost,
    port: devPort,
    strictPort: true,
    watch: {
      // Tell Vite to ignore watching src-tauri
      ignored: ["**/src-tauri/**"],
    },
  },
  // Output to out/renderer to match tauri.conf.json frontendDist
  build: {
    outDir: "out/renderer",
    emptyOutDir: true,
    rollupOptions: {
      input: resolve(__dirname, "index.html"),
    },
  },
  // Prevent Vite from obscuring Rust errors
  clearScreen: false,
});