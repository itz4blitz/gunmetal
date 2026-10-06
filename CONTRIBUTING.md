# Contributing

## Testing rules

These are not guidelines. A change that breaks one does not merge.

1. **Red, then green.** Write the failing test first and watch it fail for
   the right reason. Then write the minimum code that passes. Refactor only
   while green.
2. **Never weaken a test to pass.** If the code and a test disagree, the code
   is presumed wrong. No skipped tests, no loosened assertions.
3. **Assert deeply.** Compare whole values: the full decoded struct, the
   exact error variant with its fields. "It didn't panic" and "it returned
   `Ok`" are not assertions.
4. **Never derive an expected value from the code under test.** Use literal
   values, or an independent reference written only for the test.
5. **100% coverage** of lines, regions and functions. Rust has no stable
   branch coverage yet, so regions stand in for branches. No exclusions.
6. **Zero surviving mutants.** Every survivor is either killed with a real
   behavioural assertion or removed by restructuring the code so the
   equivalent mutant cannot be generated. Do not write a test whose only
   purpose is to match one mutant's syntax, and do not skip mutants. A
   mutant that times out also fails the gate: a hang is not a detection, so
   assert termination (see `collect_all` in `ebml.rs`) and make it fail fast.
7. **Prove behaviour at the lowest layer that can observe it.** Parsing and
   decision logic are proven by unit and property tests in the core crate.
   Higher layers test wiring only.

Run everything with:

```bash
scripts/gate.sh
```

On a machine without network, `GATE_OFFLINE=1 scripts/gate.sh` runs the
dependency checks against cached data.

The mutation run is the slow step. `GATE_SKIP_MUTANTS=1` runs every step
except it, and `GATE_MUTANTS_SHARD=k/n` runs only it, on shard `k` of `n`
(counting from 0) of the mutants it would otherwise test, with or without
`GATE_MUTANTS_DIFF`. The `n` shards together test every one of those
mutants exactly once, so a gate may be run as one command or as those
`n + 1`. The script refuses a switch it cannot read, and
`GATE_SKIP_MUTANTS=1` together with either of the other two.

## Security

Read the [secure-coding guide](docs/security/secure-coding.md) before
writing code. It lists the sinks only one module may touch, how untrusted
input is handled and how security code is tested. The full baseline is in
[docs/security](docs/security/README.md), and vulnerabilities are reported
as [SECURITY.md](SECURITY.md) describes.

### Tests name the requirements they verify

Every security requirement in `docs/security` is proved by a test or a
recorded review, and the test says which requirement it proves (SEC-STD-004).
Put one line in the test function's doc comment, starting with `Verifies:`
and followed by the requirement IDs, separated by commas:

```rust
/// Verifies: SEC-MED-028
#[test]
fn replays_the_empty_input() {
```

```rust
proptest! {
    /// Verifies: SEC-MED-001, SEC-MED-004
    #[test]
    fn never_panics_on_any_input(bytes in vec(any::<u8>(), 0..256)) {
```

- Write the IDs exactly as the requirement tables do (`SEC-` area, number).
  Cite only live requirements, never withdrawn ones.
- Name the test after the behaviour, not the ID; the `Verifies:` line is
  how the traceability check finds it.
- Add the line only where the test really proves the requirement. A test
  that merely exercises the code is not proof.
- The regression test for a fixed vulnerability is also named after its
  advisory ID (SEC-TM-003).

## Source of truth

The canonical repository is the Premier Studio Forgejo on the tailnet
(`https://git.taild1bbf.ts.net/PremierStudio/gunmetal`). GitHub is a
mirror ([decision D-89](docs/decisions.md#d-89-source-of-truth-forge)).
Open pull requests on Forgejo when you can reach it. Vulnerability
reports still go through GitHub, as [SECURITY.md](SECURITY.md) says.

## Working in parallel

The build runs as waves of work packages, many of them at once
([work-packages.md](docs/plan/work-packages.md)). These rules keep that
work mergeable. The owner accepted them on 2026-10-02
([decision D-01](docs/decisions.md#owner-answers-2026-10-02)).

### Branches and merges

1. Each wave has an integration branch: `wave-0`, `wave-1` and so on. A
   package branches from it as `wp/wp-NNN` and opens its pull request back
   into it.
2. Before it merges, a package rebases onto the current wave branch and the
   gate passes on the result. On a pull request into a wave branch, CI
   mutation-tests only the code the pull request changes
   (`GATE_MUTANTS_DIFF`). Every push to a wave branch or `main`, every pull
   request into `main` and a nightly run on `main` run the full gate.
   CI splits the mutation run, diff-scoped or full, across ten jobs
   (`GATE_MUTANTS_SHARD`) that together test every mutant. The required
   check is still the one named `gate`, which fails unless every job
   succeeded.
3. Each wave has one integrator, a dedicated agent working under the
   owner's review. It merges package pull requests into the wave branch once
   the gate passes, and resolves registry conflicts.
4. A wave closes when every package in it has merged and the full gate
   passes on the wave branch. The owner reviews and merges the wave's one
   pull request into `main`. Agents never merge into `main`, tag or release.

### Who may change which file

- Every path has one owner per wave, named in the package's "Owns" field.
  Do not edit a path you do not own.
- The root `Cargo.toml`, `scripts/gate.sh`, `.github/workflows/ci.yml`,
  `clippy.toml`, `deny.toml` and `supply-chain/` belong to the integrator
  (to WP-001 in wave 0). A package that needs a change there, such as a new
  crate, a gate step or an exception to a ban, asks for it in its pull
  request, and the integrator applies it between merges. A package that
  needs a CI job of its own adds its own workflow file.
- A new crate needs a dependency request: the dependency checklist in
  [supply-chain-and-release.md](docs/security/supply-chain-and-release.md#5-dependency-policy)
  written in the pull request, and the owner's approval. The integrator then
  adds its line to `[workspace.dependencies]`, its cargo-vet audit or
  exemption, and, for a normal dependency of the core, its entry in
  `supply-chain/core-allowlist.toml` (SEC-SUP-024, SEC-SUP-025). Crates use
  it with `workspace = true`.
- Registry files (`lib.rs`, every `mod.rs`, and the others the plan lists)
  hold one line per entry, sorted. Resolve a conflict in one by keeping
  every line and re-sorting. Resolve a conflict in `Cargo.lock` by taking
  either side and running `cargo update --workspace`, never a plain
  `cargo update`.
- A conflict anywhere else means two packages own one path: stop and ask.
  A change to an interface that a merged package already uses is its own
  small package, made in the original owner's files.

### Lints and doors

- `gunmetal-core` denies `unwrap`, `expect`, `panic!`, `unreachable!`,
  `todo!`, `unimplemented!`, indexing and slicing, unchecked arithmetic and
  large stack arrays in non-test code (SEC-MED-002). The only exception is
  an `#[expect(lint, reason = "...")]` whose reason states the invariant
  that makes it safe, with a test covering that invariant. Review refuses
  `#[allow]`.
- `clippy.toml` and `deny.toml` refuse each risky operation outside the one
  module that owns it (the doors in the
  [secure-coding guide](docs/security/secure-coding.md#one-door-per-risk)).
  The owning module marks its sanctioned call with a narrowly scoped
  `#[expect(clippy::disallowed_methods, reason = "...")]`; any other
  exception needs the integrator and a place on the xtask exception list.
- Clippy reads only the `clippy.toml` nearest to a crate, so no crate may
  add its own. The core's file repeats the workspace's word for word and
  adds the core's allocation rules (SEC-MED-003); the gate and the core's
  tests check both.

## Property tests

Parsers get property tests as well as examples. When random input is unlikely
to reach an edge case, force it in the generator (see the "unknown size" case
in `ebml.rs`). Commit the `proptest-regressions` files; they replay past
failures first.

## Sign-off

Contributions are accepted under the Developer Certificate of Origin. Add a
`Signed-off-by` line to each commit with `git commit -s`. There is no
contributor licence agreement, and the project will not be relicensed.

Coding agents cannot certify the DCO for anyone, so their commits carry no
`Signed-off-by`. When the owner merges a wave branch into `main`, the merge
commit carries the owner's `Signed-off-by`, which covers that wave's
agent-written commits ([decision register](docs/decisions.md#owner-answers-2026-10-02)).
