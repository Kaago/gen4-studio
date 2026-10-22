import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
// @ts-expect-error type error without @types/node package
import process from "node:process";
const host = process.env.TAURI_DEV_HOST;
// Agent harness: with both variables set, `/__bridge/*` is proxied to the dev bridge.
const bridgePort = process.env.G4_BRIDGE_PORT;
const bridgeToken = process.env.G4_BRIDGE_TOKEN;

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [react()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
    proxy:
      bridgePort && bridgeToken
        ? {
            "/__bridge": {
              target: `http://127.0.0.1:${bridgePort}`,
              // The bridge checks `Host`: it must read 127.0.0.1:<bridge port>, not the Vite port.
              changeOrigin: true,
              rewrite: (path: string) => path.replace(/^\/__bridge/, ""),
              headers: { "X-G4-Bridge-Token": bridgeToken },
            },
          }
        : undefined,
  },
}));
