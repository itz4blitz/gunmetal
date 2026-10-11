# 29. Mutation testing shares one build cache across shards

Date: 2026-10-10
Status: accepted, through the owner's request on 2026-10-10

## Context

CI runs mutation testing as ten parallel jobs, each calling `scripts/gate.sh`
with a different `GATE_MUTANTS_SHARD`. Each job compiles the workspace's test
binaries before cargo-mutants can run. Without a shared cache, every shard
paid that compile cost on its own.

Swatinem/rust-cache keys the cache on the GitHub Actions job id unless
`shared-key` is set and `add-job-id-key` is false. The mutants matrix had
neither, so each shard had its own cache key. Saving only from shard 0 wrote
a key the other shards never restored.

cargo-mutants 27.1.0 copies the tree into a scratch directory by default and
does not reuse `target/` there. Incremental reuse requires `--in-place`,
which is incompatible with `--jobs`. The mutation jobs run on GitHub-hosted
runners with a six-hour limit. A copied tree per parallel job would spend
that limit on copies and rebuilds. The gate therefore runs mutants in place,
one shard per job, with no `--jobs`.

cargo-mutants can skip mutants it already caught when run with `--iterate`.
The gate's rule is that every mutant is tested on every run that covers it.
`--iterate` is not used.

When both `RUSTFLAGS` and `CARGO_ENCODED_RUSTFLAGS` are unset, cargo-mutants
27.1.0 injects `CARGO_ENCODED_RUSTFLAGS=--cap-lints=warn`. The checks job
uses different flags through llvm-cov and clippy, so its cache must not share
a key with the mutants cache.

## Decisions

1. **A `mutants-warm` job runs before the mutants matrix.** It installs
   `cargo-mutants` 27.1.0 when the cache does not already hold that version,
   then calls `scripts/gate.sh` with `GATE_MUTANTS_WARM=1`, which compiles the
   workspace test binaries with `CARGO_ENCODED_RUSTFLAGS=--cap-lints=warn` and
   runs no mutants. That job is the only one that saves the mutants cache,
   including the workspace crates and `~/.cargo/bin`.
2. **Every mutants shard restores that cache and does not save.** Shards use
   `shared-key: mutants`, `add-job-id-key: false`, and `cache-workspace-crates:
   true`, with `save-if: false`, so they do not upload a `target/` full of
   mutated incremental state or race the warm job's save.
3. **Real mutation runs pass `--in-place` to cargo-mutants.** No `--jobs`,
   `--iterate`, or baseline skip. In-place is what lets rustc reuse `target/`
   on the runner. CI uses a disposable checkout. An interrupted local run can
   leave a cargo-mutants change marker in a source file.
4. **Checks, core-i686, and locked-self-test use separate shared keys.** Checks
   and locked-self-test share `shared-key: checks`; locked-self-test sets
   `save-if: false` so it does not clobber the checks cache. core-32-bit uses
   `shared-key: core-i686`. The other workflows that run Cargo each get their
   own `shared-key`.
5. **Release workflows stay unchanged.** They must not restore or save these
   CI caches (SEC-SUP-015).

## Consequences

`workspace_rules` pins the warm switch, the `--in-place` mutants invocation,
the workflow job order, and the gate job's `needs` list. Turning `--iterate`
on or dropping `--in-place` fails those tests. The warm job adds one compile
per CI run and removes that compile from the ten shards. If the warm job
fails, no shard runs until it is fixed, because the matrix `needs` it.
