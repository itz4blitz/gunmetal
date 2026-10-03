# Secure coding guide

Date: 2026-10-02. For everyone who writes code or opens a pull request,
people and coding agents alike (SEC-STD-037).

This is the short version of the [first principles](README.md#first-principles)
and the requirement tables in this directory. Where they disagree, the
requirement wins: each rule names its ID so you can read the full text.

## Before you start

- Find the requirements your change touches, and plan the tests that will
  prove them. Each test names its requirement with a `Verifies:` line
  ([CONTRIBUTING.md](../../CONTRIBUTING.md#tests-name-the-requirements-they-verify),
  SEC-STD-004).
- A new dependency needs a test that uses it and the dependency checklist
  in [supply-chain-and-release.md](supply-chain-and-release.md#5-dependency-policy):
  why our own code would not do, who maintains it, build scripts,
  proc-macros, `unsafe`, licence, transitive crates. `cargo deny` and
  `cargo vet` must pass (SEC-SUP-021, SEC-SUP-024). If a coding agent
  suggested the crate, check that it exists and is the project you meant.
- `gunmetal-core` does no I/O and has no `unsafe` ([AGENTS.md](../../AGENTS.md)),
  and has no runtime dependency outside a reviewed allowlist, which is
  empty today (SEC-SUP-025).

## Every input is hostile

Media files, tags, artwork, subtitles, playlists, provider responses,
plugins, backups, other servers and our own clients are all untrusted.

- Untrusted data enters as an untrusted type and becomes a validated
  domain type in `gunmetal-core` before anything uses it. Untrusted
  strings never reach a file path, a process argument, SQL, an outbound URL,
  a log format string or HTML (SEC-TM-031).
- A parser returns `Ok` or a typed error for every possible input and never
  panics, aborts or exhausts the stack (SEC-MED-001).
- Combine offsets and lengths with checked arithmetic and convert them with
  `try_from`, never `as` (SEC-MED-004). Never size an allocation from a
  length or count the input declares (SEC-MED-003); borrow slices of the
  input instead, as `ebml::elements` does.
- Count nesting depth and stop at the limit; never recurse without a bound
  (SEC-MED-005). Every loop over elements strictly advances or stops
  (SEC-MED-008).
- Regular expressions over untrusted input or user patterns use only the
  linear-time `regex` crate (SEC-STD-011).
- Every parser entry point gets a harness in `crates/gunmetal-fuzz`, seeds
  in `fuzz/seeds/<harness>/` and a replay test for each seed (SEC-MED-027,
  SEC-MED-028). A fuzzing finding is fixed test first: commit the minimised
  input as a seed with a test asserting the exact typed error, watch it
  fail, then fix (SEC-MED-030).

## One door per risk

These operations go through exactly one module. Clippy (`clippy.toml`) and
cargo-deny (`deny.toml`) reject them anywhere else. Inside the owning
module, mark the one sanctioned call with a narrowly scoped
`#[expect(clippy::disallowed_methods, reason = "...")]` that review can see.

| Operation | Only through | Requirement |
|---|---|---|
| Starting another program | The sandbox launcher | SEC-MED-063 |
| Outbound connections and HTTP clients | The egress client | SEC-EXT-001, SEC-TM-048 |
| Opening files | Descriptor-relative opens beneath a configured root; never a path built from untrusted data | SEC-TM-043, SEC-HIS-015 |
| SQL | Static statements with bound parameters; sort orders and filters from enums | SEC-TM-039, SEC-API-066 |
| Security randomness | One function over the OS CSPRNG; no seeded or non-cryptographic generators | SEC-STD-022 |
| Cryptography | The reviewed allow-list; AEAD only; MD5 and SHA-1 only behind a non-security type | SEC-STD-019 |
| Authorisation | One deny-by-default policy function; every route declares its policy | SEC-IAM-068, SEC-TM-005 |
| Router port mapping, UPnP, SSDP | Nowhere | SEC-TM-006 |

## Identity and access

- Anything not explicitly allowed is refused (SEC-TM-005).
- Location is never identity. Ignore forwarding headers unless the
  immediate peer is a configured trusted proxy (SEC-NET-016), and never
  grant anything for being on the LAN.
- There are no passwords, security questions, emailed codes or TOTP
  (SEC-IAM-025).
- Disabling a user, revoking a session or narrowing a grant takes effect on
  the next request (SEC-TM-028).

## Secrets, logs and errors

- A secret never goes in a URL query, a log line or an error message. Keep
  it in the secret wrapper type, which cannot be printed or compared
  (SEC-IAM-095, SEC-OPS-013).
- Logs make secrets unrepresentable and write request-derived values as
  structured, escaped fields (SEC-TM-057, SEC-IAM-096).
- Clients get generic typed error codes, never stack traces, paths, SQL or
  versions (SEC-TM-040). When isolation a feature needs is missing, the
  feature turns off and says so; it never runs unconfined.

## Concurrency

- Never hold a lock across an `.await`; clippy denies it (SEC-STD-029).
- Consume a single-use secret with one conditional update inside one
  transaction, so two concurrent redemptions cannot both succeed
  (SEC-STD-029).

## Privacy

- Nothing leaves the server unless the owner chose it: no telemetry, and
  no outbound connection missing from the egress inventory (SEC-TM-053,
  SEC-TM-048).
- Listening and viewing history is sensitive personal data; privacy
  settings start at their most private (SEC-PRV-023).

## Testing security code

The [testing rules](../../CONTRIBUTING.md#testing-rules) apply in full.
For security code in particular:

- Prove the rule where it lives: policy, parsers, token and capability
  logic are pure functions in `gunmetal-core`, tested there with unit and
  property tests and zero surviving mutants.
- Test the refusal, not just the success: the anonymous request, another
  user's object ID, the oversized or malformed input. Assert the exact error
  variant.
- A security fix starts with a failing regression test that reproduces the
  problem, named after its advisory ID (SEC-TM-003, SEC-OPS-066).

## Workflows

- `permissions: {}` at the top of every workflow, and only what each job
  needs (SEC-SUP-012).
- Pin every action to a full commit SHA with the version in a comment
  (SEC-SUP-010), and every tool to an exact version installed with
  checksum verification (SEC-SUP-011).
- Never use `pull_request_target` or `workflow_run` for anything that
  touches pull-request code (SEC-SUP-013). Pass event text through `env:`
  and quote it; never expand it with `${{ }}` inside `run:` (SEC-SUP-014).
- Every cargo command runs with `--locked` (SEC-SUP-020).

## Reporting

Found a vulnerability? Follow [SECURITY.md](../../SECURITY.md), not the
public issue tracker.
