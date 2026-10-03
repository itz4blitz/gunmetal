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

## Property tests

Parsers get property tests as well as examples. When random input is unlikely
to reach an edge case, force it in the generator (see the "unknown size" case
in `ebml.rs`). Commit the `proptest-regressions` files; they replay past
failures first.

## Sign-off

Contributions are accepted under the Developer Certificate of Origin. Add a
`Signed-off-by` line to each commit with `git commit -s`. There is no
contributor licence agreement, and the project will not be relicensed.
