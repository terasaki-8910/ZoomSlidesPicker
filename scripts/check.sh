#!/usr/bin/env sh
# Run every machine check. Nothing runs these automatically; run this before calling
# a feature done. Parts that do not exist yet are skipped.
set -eu
cd "$(dirname "$0")/.."

step() { printf '\n== %s\n' "$*"; }

if [ -f Cargo.toml ]; then
  step "cargo fmt --check"
  cargo fmt --all --check
  step "cargo clippy"
  cargo clippy --workspace --all-targets -- -D warnings
  step "cargo test"
  cargo test --workspace
  if [ -f crates/zsp-detect/tests/timing.rs ]; then
    step "detection timing (release, ACCEPTANCE A11)"
    cargo test --release -p zsp-detect --test timing
  fi
fi

if [ -f package.json ]; then
  step "pnpm lint"
  pnpm lint
  step "pnpm test"
  pnpm test
  if [ -f scripts/ui-check.sh ]; then
    step "ui check"
    sh scripts/ui-check.sh
  fi
fi

printf '\nall checks passed\n'
