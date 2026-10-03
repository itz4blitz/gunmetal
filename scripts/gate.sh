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

# GATE_OFFLINE=1 runs the dependency checks without network: cargo-deny uses
# its cached advisory database and fails once that is more than 90 days old,
# and cargo-vet uses only the cargo cache. cargo-vet's --locked means "fetch
# no new imported audits"; the lint step's --locked has already checked
# Cargo.lock by the time it runs.
locked=--locked
vet_offline=()
if [[ "${GATE_OFFLINE:-0}" == 1 ]]; then
  locked=--frozen
  vet_offline=(--frozen)
fi

echo "==> format"
cargo fmt --all -- --check

echo "==> lint"
cargo clippy --locked --workspace --all-targets -- -D warnings

echo "==> dependency policy: advisories, bans, licences, sources"
cargo deny "$locked" check

echo "==> dependency audits"
cargo vet --locked "${vet_offline[@]}"

echo "==> tests with 100% coverage"
cargo llvm-cov --locked --workspace \
  --fail-under-lines 100 --fail-under-regions 100 --fail-under-functions 100

echo "==> mutation testing, zero survivors"
cargo mutants --workspace --no-shuffle --cargo-arg=--locked
