#!/usr/bin/env bash
# Runs every quality gate. CI runs exactly this script, so passing here means
# passing there.
#
# Every cargo command that resolves dependencies runs with --locked, so a
# manifest change without a matching Cargo.lock fails the gate (SEC-SUP-020).
set -euo pipefail
cd "$(dirname "$0")/.."

# A distro-packaged Rust has no rustup llvm-tools component, so point
# cargo-llvm-cov at the system LLVM instead.
if ! command -v rustup >/dev/null; then
  export LLVM_COV="${LLVM_COV:-$(command -v llvm-cov)}"
  export LLVM_PROFDATA="${LLVM_PROFDATA:-$(command -v llvm-profdata)}"
fi

echo "==> format"
cargo fmt --all -- --check

echo "==> lint"
cargo clippy --locked --workspace --all-targets -- -D warnings

echo "==> tests with 100% coverage"
cargo llvm-cov --locked --workspace \
  --fail-under-lines 100 --fail-under-regions 100 --fail-under-functions 100

echo "==> mutation testing, zero survivors"
cargo mutants --workspace --no-shuffle --cargo-arg=--locked
