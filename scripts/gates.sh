#!/usr/bin/env sh
# Machine gates. Sourced by run.sh; run inside the feature worktree CWD.
# Put OS-specific TEXT scans (no-emoji, hardcoded-color) in your LINTER
# (eslint/stylelint plugins), NOT here -- keeps this portable. See README.
# This project has two halves: a Cargo workspace (detection, capture, output,
# src-tauri) and a pnpm frontend. Each half that exists must pass; a half that
# does not exist yet is skipped, so early stages are not blocked.
gate_tests() {
  ran=0
  if [ -f Cargo.toml ]; then ran=1; cargo test --workspace --quiet || return 1; fi
  if [ -f package.json ] && grep -q '"test"' package.json; then ran=1; pnpm test || return 1; fi
  [ "$ran" = 1 ] || echo "gate_tests: no runner detected -- edit gates.sh"
  return 0
}
gate_lint() {
  # no-emoji / design-tokens-only / a11y rules live in the eslint config.
  ran=0
  if [ -f Cargo.toml ]; then
    ran=1
    cargo fmt --all --check || return 1
    cargo clippy --workspace --all-targets --quiet -- -D warnings || return 1
  fi
  if [ -f package.json ] && grep -q '"lint"' package.json; then ran=1; pnpm lint || return 1; fi
  [ "$ran" = 1 ] || echo "gate_lint: no lint script -- see README to add UI rules"
  return 0
}
gate_ui() {
  # responsive/contrast: playwright screenshot-diff + axe, wired by the criteria stage.
  if [ -x scripts/ui-check.sh ]; then scripts/ui-check.sh; else return 0; fi
}
gate_all() { gate_tests && gate_lint && gate_ui; }
