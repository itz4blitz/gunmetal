#!/usr/bin/env bash
# Runs every quality gate. CI runs exactly this script, so passing here means
# passing there.
#
# Every cargo command that resolves dependencies runs with --locked, so a
# manifest change without a matching Cargo.lock fails the gate (SEC-SUP-020).
#
# GATE_MUTANTS_DIFF=<git ref> limits mutation testing to the code changed
# since the merge base with that ref, as pull requests into a wave branch do
# (decision D-01). Every other step always covers the whole workspace, and so
# does mutation testing without it.
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

echo "==> lint configuration"
# Clippy reads the clippy.toml nearest to each crate and ignores the rest, so
# a crate with a file of its own would switch the workspace's bans off for
# itself. Only the workspace's file and the core's may exist; the core's
# tests check that the core's repeats every workspace rule.
clippy_configs=$(git ls-files --cached --others --exclude-standard -- '*clippy.toml' | LC_ALL=C sort)
if [[ "$clippy_configs" != $'clippy.toml\ncrates/gunmetal-core/clippy.toml' ]]; then
  printf 'only clippy.toml and crates/gunmetal-core/clippy.toml may exist; found:\n%s\n' "$clippy_configs" >&2
  exit 1
fi

echo "==> lint"
cargo clippy --locked --workspace --all-targets -- -D warnings

echo "==> doors Clippy cannot see"
# SEC-OPS-037: only the listener and the core's forwarding-header parser read
# forwarding headers; everything else takes the ClientContext they produce.
# Tests may send the headers, to prove they are ignored.
status=0
git grep --untracked -n -i -E '"(forwarded|x-forwarded-[a-z-]+|x-real-ip|x-client-ip|true-client-ip|cf-connecting-ip)"|header::FORWARDED' -- 'crates/*.rs' ':!crates/*/tests/*' ':!crates/gunmetal-server/src/listener.rs' ':!crates/gunmetal-core/src/http/forwarded.rs' || status=$?
if ((status == 0)); then
  echo "forwarding headers are read outside the listener (SEC-OPS-037)" >&2
  exit 1
elif ((status != 1)); then
  echo "the forwarding-header search failed (exit $status)" >&2
  exit 1
fi
# SEC-SUP-026: the workspace has no build script. The first one that is
# needed comes with a check that it reads no test fixture and touches no
# network.
if git ls-files --cached --others --exclude-standard -- 'build.rs' '*/build.rs' | grep .; then
  echo "a workspace build script needs a reviewed exception (SEC-SUP-026)" >&2
  exit 1
fi

echo "==> dependency policy: advisories, bans, licences, sources"
cargo deny "$locked" check

echo "==> dependency audits"
cargo vet --locked "${vet_offline[@]}"

echo "==> the core's dependencies are reviewed"
# SEC-SUP-025: every crate the core uses at run time, on any target, through
# any other crate and with any of its features on, is on the reviewed
# allowlist, and the allowlist names no crate the core does not use. The
# core's tests read its manifest; xtask core-deps compares what cargo
# resolves from it with supply-chain/core-allowlist.toml. --all-features
# matters: another member can turn on an optional dependency of the core
# through one of the core's features.
mkdir -p target
cargo tree "$locked" -p gunmetal-core -e normal --target all --all-features --prefix none --format '{p}' >target/core-deps.txt
cargo run "$locked" -q -p xtask -- core-deps target/core-deps.txt

echo "==> native and unsafe code that ships is on the allow-list"
# SEC-TM-034: every crate linked into what ships that is a -sys crate,
# declares `links` or uses `unsafe` is on supply-chain/native-allowlist.toml
# with a reason and a requirement, and the list names no other crate. The
# targets are the ones deny.toml names.
cargo metadata "$locked" --format-version 1 --all-features --filter-platform x86_64-unknown-linux-gnu --filter-platform aarch64-unknown-linux-gnu --filter-platform i686-unknown-linux-gnu --filter-platform wasm32-unknown-unknown >target/native-code.json
cargo run "$locked" -q -p xtask -- native-code target/native-code.json

echo "==> tests with 100% coverage"
cargo llvm-cov --locked --workspace \
  --fail-under-lines 100 --fail-under-regions 100 --fail-under-functions 100

echo "==> documentation tests"
# cargo llvm-cov does not run doctests. Some of them are compile-fail tests
# that prove a door cannot be opened at all (SEC-HIS-012, SEC-PRV-050,
# SEC-API-066), so they run here on their own.
cargo test --locked --workspace --doc

echo "==> mutation testing, zero survivors"
scope=()
if [[ -n "${GATE_MUTANTS_DIFF:-}" ]]; then
  diff_file=$(mktemp)
  trap 'rm -f "$diff_file"' EXIT
  git diff --no-ext-diff "${GATE_MUTANTS_DIFF}...HEAD" >"$diff_file"
  echo "only mutants in code changed since ${GATE_MUTANTS_DIFF}"
  scope=(--in-diff "$diff_file")
fi
cargo mutants --workspace --no-shuffle "${scope[@]}" --cargo-arg=--locked
