import { defineConfig } from "@playwright/test";

// scripts/ui-check.sh builds the mock (`pnpm build:mock`) and then runs this config.
// Server lifecycle (start dist-mock on a fixed port, wait for it, tear it down after the
// run) is delegated to Playwright's own webServer option rather than hand-rolled shell
// job control -- see docs/impl/frontend.md > Checks.
const PORT = 4317;

export default defineConfig({
  testDir: "./tests/ui",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  reporter: [["list"]],
  use: {
    baseURL: `http://127.0.0.1:${PORT}`,
    locale: "ja-JP",
  },
  webServer: {
    // Bind to the same address the tests use: "localhost" can resolve to [::1] only.
    command: `pnpm exec vite preview --outDir dist-mock --host 127.0.0.1 --port ${PORT} --strictPort`,
    url: `http://127.0.0.1:${PORT}`,
    reuseExistingServer: !process.env.CI,
    timeout: 30_000,
  },
  projects: [
    {
      // Deliberately not spreading Playwright's devices["Desktop Chrome"]: that preset
      // pins a Windows-flavoured userAgent/userAgentData regardless of the host OS,
      // which made Settings render Windows shortcut notation even when run on macOS.
      // Each spec file sets its own viewport per window anyway.
      name: "chromium",
      use: { isMobile: false, hasTouch: false },
    },
  ],
});
