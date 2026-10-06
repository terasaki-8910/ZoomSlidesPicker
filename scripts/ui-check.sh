#!/usr/bin/env sh
# Builds the frontend against the IPC mock and runs the Playwright + axe checks against
# it. Playwright's own `webServer` option (playwright.config.ts) starts `vite preview` on
# a fixed port, waits for it, and stops it again after the run -- see
# docs/impl/frontend.md > Checks.
set -eu
cd "$(dirname "$0")/.."

printf '\n== build (mock) ==\n'
pnpm build:mock

printf '\n== playwright + axe ==\n'
pnpm exec playwright test

printf '\nui-check passed\n'
