import { configDefaults, defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// Frontend-only build: the webview never talks to Rust directly (see docs/impl/ipc.md).
// VITE_IPC_MOCK=1 (set by the build:mock/preview:mock scripts) switches src/ipc/index.ts
// to the in-memory mock instead of @tauri-apps/api, so `pnpm dev`, `pnpm build:mock` and
// the Playwright-driven scripts/ui-check.sh all run without a Tauri host.
export default defineConfig({
  plugins: [react()],
  // Matches src-tauri/tauri.conf.json's build.devUrl (docs/impl/shell.md).
  server: {
    port: 1420,
    strictPort: true,
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    css: false,
    mockReset: true,
    // tests/ui/*.spec.ts are Playwright specs (run via scripts/ui-check.sh), not vitest's
    // -- exclude them, since vitest's default include glob also matches "*.spec.ts".
    exclude: [...configDefaults.exclude, "dist-mock/**", "tests/ui/**"],
  },
});
