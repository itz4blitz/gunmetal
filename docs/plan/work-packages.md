# Backend work packages

Written on 2026-10-02. Status: draft for the project owner's review,
revised the same day after an adversarial review and again to conform to
the [security baseline](../security/README.md) (see
[Review notes](#review-notes)).

**Security comes first in this plan.** The security baseline's first
principles are absolute, and where this plan and the baseline disagreed,
the plan was changed to follow the baseline. Every package names the trust
boundaries and threats it touches and the requirement IDs its tests must
verify, and [the R1 security coverage table](#security-coverage-every-r1-requirement-has-a-package)
at the end maps every R1 requirement to a package.

This is the build plan for the backend: the shared Rust core, the server and
the thin layer that hands the core to the web client. It turns the
[feature map](../features/README.md), the [interface documents](../ui/README.md)
and the capability list in [api-needs.md](api-needs.md) into work packages
that many coding agents can build at the same time, each under the rules in
[CONTRIBUTING.md](../../CONTRIBUTING.md) and [AGENTS.md](../../AGENTS.md).
Release R1 (music) is planned in full, and waves 1 to 6 build exactly the
R1 the owner adopted on 2026-10-02 (register decision D-10): the smaller
R1 in the register's [R1 scope](../decisions.md#r1-scope) section. The
packages that serve only the point releases R1.1, R1.2 and R1.3, or a
later release, are kept with their full specifications in
[After R1](#after-r1-point-releases-and-later), grouped by release, so
they can be scheduled later. Release R2 (video) is planned in outline at
the end.

The client itself (React Native, TypeScript) is not in this plan. Where a
package exists only so the client can call the core on the device, it says
so.
The web player's packages are in [client-packages.md](client-packages.md).

Claims this plan could not check against a source are marked "(unverified)".
Choices that go beyond the feature map, the ADRs or the security baseline
are marked **Proposal**, and the ones that need the owner are collected in
[Decisions the owner must make](#decisions-the-owner-must-make).

## Contents

1. [How to use this plan](#how-to-use-this-plan)
2. [Ground rules every package follows](#ground-rules-every-package-follows)
3. [Crate layout](#crate-layout)
4. [Databases, durable files and migrations](#databases-durable-files-and-migrations)
5. [Proposed external crates](#proposed-external-crates)
6. [Working in parallel: shared files and merging](#working-in-parallel-shared-files-and-merging)
7. [Waves at a glance](#waves-at-a-glance)
8. [Wave 0: what everyone agrees on first](#wave-0-what-everyone-agrees-on-first)
9. [Wave 1: core leaves and infrastructure crates](#wave-1-core-leaves-and-infrastructure-crates)
10. [Wave 2: tag mapping, probing, search and the server's spine](#wave-2-tag-mapping-probing-search-and-the-servers-spine)
11. [Wave 3: the music model, sign-in, streaming and sync](#wave-3-the-music-model-sign-in-streaming-and-sync)
12. [Wave 4: scanning and the features that need it](#wave-4-scanning-and-the-features-that-need-it)
13. [Waves 5 and 6: health, jobs, benchmark and acceptance](#waves-5-and-6-health-jobs-benchmark-and-acceptance)
14. [Coverage check: every R1 capability has an owner](#coverage-check-every-r1-capability-has-an-owner)
15. [After R1: point releases and later](#after-r1-point-releases-and-later)
16. [R2 (video) in outline](#r2-video-in-outline)
17. [Decisions the owner must make](#decisions-the-owner-must-make)
18. [Review notes](#review-notes)
19. [Security coverage: every R1 requirement has a package](#security-coverage-every-r1-requirement-has-a-package)

## How to use this plan

### What a package is

A work package is one unit of test-first work that one agent can finish and
merge on its own. Each package lists:

- **Wave.** The earliest point it can start. Packages in the same wave own
  disjoint files and depend only on packages in earlier waves, so they can
  run at the same time. A package may start as soon as every package it
  depends on has merged; it does not have to wait for its whole wave.
- **Size.** S is a few hundred lines of code and tests, about a day of
  focused work. M is roughly 500 to 1,500 lines. L is roughly 1,500 to
  3,000 lines. Anything bigger has been split.
- **Owns.** The exact paths the package creates or changes. A package
  creates a crate, module or file only when its first failing test needs it
  (AGENTS.md). It never edits a path another package owns, except the
  shared registry lines described in
  [Working in parallel](#working-in-parallel-shared-files-and-merging).
- **Serves.** The feature IDs and the API capability IDs from
  [api-needs.md](api-needs.md) it delivers, in whole or in part.
- **Security.** The trust boundaries (TB) and threats (TM-T) from the
  [threat model](../security/threat-model.md) the package touches, which
  SEC-TM-001 requires to be non-empty, and the security requirement IDs
  its tests must verify. Each of those tests carries a `Verifies:` line
  naming the IDs it proves, as [CONTRIBUTING.md](../../CONTRIBUTING.md#tests-name-the-requirements-they-verify)
  describes, and the traceability check (WP-127) fails the release when an
  R1 requirement has no such test (SEC-STD-004). A test names an ID only
  when it really proves it; exercising the code is not proof. Where a
  requirement is listed under more than one package, each proves the part
  that lives in its own files.
- **Scope and not in scope.** What it does and, as important, what it
  leaves to another package.
- **Interface sketch.** The main types and signatures, as guidance. The
  first failing test is free to change them; a change that another package
  already relies on goes through the merge protocol.
- **Tests.** Unit examples, properties, the edge cases that must be named in
  tests, and for server packages which integration tests run against a real
  SQLite database in a temporary directory.
- **Risks and decisions.** What could go wrong and what needs the owner.

### Definition of done

A package is done when `scripts/gate.sh` passes on its branch rebased on the
current integration branch, with the real output reported (AGENTS.md), and
when every owner decision it depends on has been made. A package whose
decision is still open does not start.

A package is also not done until every requirement in its **Security**
field has a test with a `Verifies:` line (or, where the requirement names
a manual review, a dated review record), and until every route-table
security check present on the integration branch when it rebases passes
with its routes registered. From wave 2 that means WP-118's allow-list
check and anonymous-request suite; from the merge of WP-131 it also means
WP-131's generated suites. A package never waits for a same-wave peer's
suite. When WP-131, or a later suite, fails on a route another package
already merged, the failure goes back to that route's owner as a new small
package under the merge protocol; WP-131 does not edit the route's files.
A security
requirement is never weakened to make a package fit; if a feature cannot
be built securely as written, the package changes how it works, moves to
a later release, or is dropped, and says which.

### Where the plan comes from

The build order follows the feature map's R1 order: the music model and
scan, the player, the queue and playlists, lyrics, search and home, then
sign-in, backups and the security baseline. Two things pull work earlier
than that order suggests. First, almost every server write lands in the
user log or the identity store, so ADR 3 (feature map open decision 1) is
wave 0. Second, the security baseline in [docs/security](../security/README.md)
puts parsing in separate worker processes from R1 (SEC-MED-018) and makes
authorisation one pure function in the core (SEC-IAM-068), so those are
foundations here, not finishing touches.

The baseline's principle "one door per risk" decides the rest of the early
order. Each risky operation has exactly one module that may perform it,
and that module lands before any package that needs the operation, so no
package ever has a reason to open a second door. The table in
[One door per risk](#one-door-per-risk) lists the doors and the packages
that build them.

## Ground rules every package follows

These restate the project's rules where they bite on parallel work, and add
the conventions every package must share. They are not repeated in the
packages.

### Testing

- Red, then green; never weaken a test; assert whole values; never derive an
  expected value from the code under test (CONTRIBUTING.md rules 1 to 4).
- 100% line, region and function coverage and zero surviving mutants
  (rules 5 and 6). This applies to every crate in the workspace, including
  the test-support crate, because the gate runs over the whole workspace.
- Prove behaviour at the lowest layer that can observe it (rule 7). Parsing,
  identity, grouping, queue verbs, rules, merge rules, gain, the decision
  engine and authorisation are proven in the core. Server packages test
  wiring, authorisation on every route, persistence and concurrency.
- **Synthetic media only.** Every test builds the bytes it parses, in code,
  through the builders in `gunmetal-testkit` or a reference encoder written
  for that test. No copyrighted audio, no recorded files, no large fixtures.
  A FLAC file in a test is a few hundred bytes: a STREAMINFO block, a
  comment block and one or two frames of constant samples. Fuzz corpora and
  minimised reproducers are the one kind of committed binary, as SEC-MED-028
  and SEC-MED-030 require, and they are small.
- **Independent oracles.** A builder in the testkit shares no code with the
  parser it feeds, as `encode_vint` in `ebml.rs` shares none with
  `decode_vint`. Each builder is itself tested against literal byte strings
  taken from the format's specification.
- **Termination.** Every iterator test collects through a ceiling helper
  like `collect_all` in `ebml.rs`, so a hang fails instead of timing out.
- **Integration tests use real SQLite.** Any server test that touches data
  opens real database files in a temporary directory created by the testkit.
  Nothing mocks the database (security baseline, web-and-api-security.md
  "Requirements").
- **Time and randomness are inputs.** Core functions take `now` and seeds as
  arguments. Server code reads time through a `Clock` and randomness through
  one `Rng` handle, both replaceable in tests.
- **Host facts are inputs too.** The user ID the process runs as, the
  filesystem type of the data directory, free space and the kernel's
  Landlock ABI are read through small probes and passed to pure checks, so
  "running as root" or "network filesystem" is tested without needing
  either. Tests that rely on file permissions (an unreadable directory, a
  mode-0644 secret) assert at their start that they are not running as
  UID 0 and fail loudly if they are, because root ignores permissions and
  the test would pass for the wrong reason.
- **Measurements are not tests.** Timings and sizes at 100,000 tracks are
  recorded by the benchmark runner (WP-115), not by the test suite. The
  gate reruns the whole suite for every mutant, so a heavy test multiplies
  the gate's time; tests use small inputs and assert exact results.
  Statistical timing assertions are not used, because they flake; where a
  constant-time or uniform-time property matters, the test asserts the
  structure that produces it (for example, that the credential verifier
  runs the same check for an unknown account as for a wrong code, WP-064).
- **Keys never leave the secrets crate.** Core functions that need a keyed
  MAC (capability URLs, token hashes, recovery-code hashes) take a
  `MacProvider` trait object (WP-031) instead of key bytes, and
  `gunmetal-secrets` implements the keyed operation itself. No package
  needs `Secret::expose` outside `gunmetal-secrets` to sign, verify or
  hash. Each server package that calls such a core function writes a
  short adapter from the secrets crate's `KeyRing::mac` to the core trait.
- **Security tests are tests like any other.** They follow every rule
  above, and they test the refusal as well as the success: the anonymous
  request, another person's object ID, the oversized or malformed input,
  each asserting the exact error (secure-coding.md). A test that proves a
  requirement carries a `Verifies:` line. A security fix starts with a
  failing regression test named after its advisory ID (SEC-TM-003).

### Parsers

Every parser follows the parsing contract in
[media-and-parser-safety.md](../security/media-and-parser-safety.md),
section 2: sans-I/O, `Limits`, `Budget` and `Depth` on every parse,
allocations bounded by bytes rather than declarations, checked arithmetic,
no `as` casts from wide to narrow integers, iteration that always advances,
typed errors that carry offsets, partial results for optional parts, lossy
text decoding with controls stripped, and typed values. Every public parse
entry point gets a fuzz harness in `crates/gunmetal-fuzz`, seeds in
`fuzz/seeds/<harness>/` and a stable replay test, as the
[secure-coding guide](../security/secure-coding.md) lays out (SEC-MED-027,
SEC-MED-028), and a property test over arbitrary bytes that asserts it
returns rather than panics (SEC-MED-001). Any decompression goes through
the one streaming helper (WP-128, SEC-MED-009).

### Error conventions (Proposal)

- **In the core,** every error is an enum that derives `Debug`, `Clone`,
  `PartialEq` and `Eq`, and every variant carries the offset and the values
  involved. Core errors do not implement `Display`. Instead each error type
  implements `Describe`, which maps it to a stable problem code from the
  problem catalogue (WP-006). The catalogue holds the plain-language text
  admins and users see, so wording lives in one tested place.
- **Shared variants.** Parsers reuse the contract's error kinds
  (`Truncated`, `BudgetExceeded`, `TooDeep`, `LimitExceeded`) by wrapping
  them, rather than inventing their own spelling of the same thing.
- **In the server,** every handler returns `Result<T, ApiError>`. `ApiError`
  carries a problem code and an HTTP status and renders as an RFC 9457
  problem document. Internal causes are logged as structured fields and
  never sent to the client (SEC-API errors section). There is no `anyhow`,
  no `Box<dyn Error>` across module boundaries, and no `unwrap` or `expect`
  outside tests.
- **Not found and not visible are the same.** An object the caller may not
  see returns exactly the response of an object that does not exist
  (SEC-API-011).

### Logging

All logs are one JSON object per line, written through one logger with
escaped, length-capped fields (SEC-OPS-022, SEC-IAM-096, SEC-MED-062). No
package formats a log line by string concatenation, and no secret type can
be logged (WP-047). Every log event type is a variant of one typed enum
listed in the log inventory (SEC-PRV-044); at the default level no event
carries a media title, file path, search term or play (SEC-PRV-043). The
security audit log is a separate store with its own rules (WP-069).

### One door per risk

The security baseline's eighth first principle: one authorisation layer,
one egress client, one command builder, one credential verifier, one
owner per control and one value per parameter. Each door below is built by
the named packages before any package that needs it, and `clippy.toml` or
`deny.toml` (WP-001) rejects the operation everywhere else.

| Door | Built by (wave) | Requirements |
|---|---|---|
| Authorisation: one deny-by-default policy function, a `Permit` only it can mint, and every storage reader that returns a user-visible object takes the `Permit` (catalogue, identity store, user log, audit log), with a short written list of pre-principal lookups | WP-033 (1), WP-046 (1), WP-065 (2), WP-067 (2), WP-068 (2), WP-069 (2) | SEC-IAM-068, SEC-TM-024, SEC-API-010, SEC-IAM-070 |
| Route policies checked at compile time: a route without an access class, capability, route tag and audit event does not compile | WP-044 (1), WP-118 (2); generated suites WP-131 (3) | SEC-TM-005, SEC-API-001, SEC-IAM-041, SEC-IAM-067, SEC-OPS-020 |
| The public-route allow-list and the anonymous-request suite | WP-118 (2), extended by WP-131 (3) | SEC-API-002, SEC-API-003, SEC-IAM-067, SEC-TM-004 |
| Fresh user verification: the session records its last user-verified assertion and the credential that made it, and the pipeline refuses a fresh-uv route when that assertion is older than 5 minutes or came from OIDC | WP-062 (2); step-up renewal WP-106 (4) | SEC-IAM-041, SEC-IAM-107, SEC-TM-017 |
| Egress: the only outbound client, per-purpose host allowlists, resolve-then-check | WP-048 (2) | SEC-EXT-001, SEC-API-076, SEC-TM-048 |
| Command builder: the sandbox launcher, the only place a process starts | WP-045 (1) | SEC-HIS-020, SEC-MED-063, SEC-TM-046 |
| Credential verifier: every authentication pathway, one inventory, one limiter | WP-064 (2) | SEC-HIS-046, SEC-TM-014 |
| Secrets and keys: the root secret, purpose keys, `Secret<T>`, the vault, the CSPRNG and the only function that mints public IDs | WP-047 (1) | SEC-OPS-012, SEC-OPS-013, SEC-STD-022, SEC-HIS-012 |
| Cryptography: SHA-256 for the core in `gunmetal-core/src/crypto.rs`; every other algorithm, keyed or keyless (HMAC, HKDF, XChaCha20-Poly1305, Argon2id, Ed25519, P-256, age) and the rustls provider and configuration constructors in `gunmetal-secrets/src/crypto/`; no cryptographic crate is used anywhere else (owner decision 38) | WP-122 (0), WP-047 (1); lint WP-001 (0) | SEC-STD-018, SEC-STD-019 |
| Files: the data-root handle and the library-root handles | WP-126 (0), WP-060 (2) | SEC-HIS-016, SEC-MED-033, SEC-TM-043 |
| Rate limits and concurrency gates, the server-wide counter, and the limits register | WP-032 (1), WP-130 (2) | SEC-API-057, SEC-NET-052, SEC-IAM-101, SEC-STD-030 |
| The audit log: the typed security-event catalogue and sink trait, the store, and the one server-side sink that records every security event from the bus | WP-006 (0), WP-069 (2) | SEC-OPS-020, SEC-OPS-023, SEC-IAM-094 |
| Retention schedule: one pure table of retention periods and the purge decision | WP-138 (1) | SEC-PRV-005, SEC-TM-055 |
| Parser budgets | WP-004 (0) | SEC-TM-032, SEC-MED-003, SEC-MED-007 |
| Decompression | WP-128 (1) | SEC-MED-009 |
| Worker isolation | WP-045 (1), WP-061 (2), WP-078 (3) | SEC-MED-018, SEC-MED-022, SEC-MED-024 |
| The client address and its path class: the pure path-class function in the core, and the one server-side resolver that reads the socket peer and yields a `ClientContext` | WP-005 and WP-006 (0), WP-023 (1), WP-118 (2); lint WP-001 (0) | SEC-NET-016, SEC-NET-068, SEC-OPS-037 |
| SQL: the static-query type, and the one SQLite connection opener that sets every pragma | WP-126 (0) | SEC-API-066, SEC-TM-039, SEC-PRV-050 |

A later package that needs one of these operations calls the door; it
never adds an exception to the lint. A change to a door is its own small
package under the merge protocol.

## Crate layout

**Proposal.** Fourteen crates under `crates/`, each with a reason to exist.
`gunmetal-fuzz` already exists; `gunmetal-names` was added for the
project's name service when the plan was aligned with the security
baseline, and is created only in R2, because the owner moved the name
service out of R1 (register D-07), so R1 builds thirteen.
ADR 1 decision 2 puts protocol types, the decision engine, the parsers and
the remuxer in one core crate, so the core stays one crate. Everything that
does I/O is split by the boundary it guards, because the security baseline
enforces several rules per crate ("only this crate may open files", "only
this crate may start a process", "only this crate may make an outbound
request"), and because separate crates let parallel agents own whole
`Cargo.toml` files and keep mutation runs small.

| Crate | Kind | Holds | Why separate |
|---|---|---|---|
| `gunmetal-core` | library, pure | Parsers, tag mapping, the music model, identity and scan diff, queue, shuffle, rules, search, radio, home rows, gain and loudness arithmetic, the decision engine, the player state, tokens, authorisation policy, user-event merge rules, log framing, the wire codec, protocol types | ADR 1: one core compiled into the server, the WASM web client and (R2) the native apps. No I/O, no `unsafe`, no panics. |
| `gunmetal-fuzz` | library, dev-only (exists) | One plain-function fuzz harness per parse entry point, replayed by stable `cargo test` over `fuzz/seeds/`; the nightly cargo-fuzz project in `fuzz/` calls the same functions | SEC-MED-027 and SEC-MED-028 ask for harnesses callable from stable tests; keeping them out of the core keeps fuzz-only code out of the shipped crate. |
| `gunmetal-testkit` | library, dev-only | Builders for synthetic FLAC, ID3, MP3, MP4, Ogg, WAV and AIFF; checksums; temporary directories; a manual clock; a synthetic library generator | Shared oracles for core and server tests. `publish = false`, used only as a dev-dependency, still under the gate. |
| `gunmetal-store` | library, I/O | The rebuildable SQLite cache: connections, the single writer, schema registration, the change log and the catalogue tables | ADM-080's one writer lives in one place; schema digest and rebuild logic are tested once. |
| `gunmetal-durable` | library, I/O | The identity store (SQLite with migrations), the user log segment files, the audit log, the derived-data store | Durable state has different rules from the cache (fsync, migrations, never discarded). ADR 3 defines it. |
| `gunmetal-secrets` | library, I/O | The root secret, key derivation and rotation, the `Secret<T>` wrapper, the vault for third-party secrets | `expose()` is reachable only here, enforced by a Clippy `disallowed-methods` entry (operations-and-incident-response.md, section 2). |
| `gunmetal-fs` | library, I/O | The data-root handle every data-directory module uses (WP-126), library root handles, the open rules, symlink policy, walking, fingerprints, watching and polling | `std::fs` path functions are banned everywhere else (SEC-MED-033), and every file access goes through a directory handle beneath the library or data root (SEC-HIS-016). |
| `gunmetal-worker` | library, I/O | The worker host loop, the IPC framing, the Linux sandbox, the worker pool and quarantine, the probe, artwork and loudness jobs | `std::process::Command` is allowed only here (SEC-MED-063); the image decoder and any audio decoder link only here. |
| `gunmetal-http` | library | The route table type, the request pipeline, security headers, body limits and the problem renderer | The route table is the single source for the allow-list, role-matrix and header tests and for the API reference (web-and-api-security.md, "The route table"). |
| `gunmetal-egress` | library, I/O | The only outbound HTTP client, with the egress gate and its record of connections | Makes the ban on other outbound clients enforceable (SEC-API-076). |
| `gunmetal-names` (R2) | library plus a service binary | The per-server name service: the label codec, the pure DNS-answer function and the registration rules (WP-129, R2) | A project service, not part of the server binary; the server's naming client (WP-135, R2) reuses its label codec, so both sides agree by construction. Not built in R1. |
| `gunmetal-server` | library plus the `gunmetal` binary | Application state, every route handler, jobs, the command line | The composition root. Each feature package owns one module directory in it. |
| `gunmetal-wasm` | library, `cdylib` | A thin `wasm-bindgen` facade over the core for the web client | Keeps WASM bindings out of the core so the core stays dependency-light. |
| `xtask` | binary, dev-only | Repository checks: the harness registry, the lockfile-age check, the docs lints and requirement traceability (WP-127), repository-settings and release checks (WP-124, WP-136), the limits register check (WP-130), the benchmark runner | The security baseline asks for these checks as Rust under the same gate (supply-chain-and-release.md). |

The worker is not a separate binary. It is the `gunmetal` binary started
with a hidden subcommand, so one binary ships (media-and-parser-safety.md,
section 4).

**Dependency direction.** `core` depends on nothing in the workspace. The
I/O crates depend on `core`. `gunmetal-server` depends on all of them.
`gunmetal-wasm` depends only on `core`. `gunmetal-fuzz` depends on the
core and, for one purpose only, on `gunmetal-worker`: SEC-MED-026 requires
Gunmetal's wrappers around third-party decoders (the artwork job's image
decoder and the audio decoder of record 5) to be fuzzed, and those
wrappers live in the worker, which exports them for that purpose only.
`gunmetal-testkit` depends on nothing in the workspace, so it can never
borrow the code it is meant to check.

**Inside the core.** Formats live under `src/formats/`, tag mapping under
`src/tags/`, the music model under `src/music/`, and each other subsystem in
its own module. The existing `src/ebml.rs` stays where it is; it moves under
`src/formats/` only when the Matroska work in R2 needs it, and that move is
owned by the R2 package that makes it.

**Risk: gate time.** One growing core crate means the workspace mutation run
will take longer with every package. See owner decision 6 for how the gate
scales.

## Databases, durable files and migrations

The data directory follows the layout in
[operations-and-incident-response.md](../security/operations-and-incident-response.md),
section 2: `secrets/`, `durable/`, `cache/`, `snapshots/` and `backups/`, with
`derived/` added for ADM-141.

### Four stores, four rules

| Store | Where | Format | Changed how | Survives |
|---|---|---|---|---|
| **The cache** | `cache/library.db` | SQLite, WAL mode | Never migrated. If its schema digest differs from the binary's, it is discarded and rebuilt from the files, the user log and the derived-data store (ADM-058, ADM-077). | Nothing; it is rebuildable by definition (ADR 1, decision 5). |
| **The identity store** | `durable/identity.db` | SQLite, WAL mode, `synchronous=FULL` | Numbered forward migrations after R1 ships; a snapshot before every migration; invariants checked before commit; an older binary refuses a newer file (SEC-OPS-051). | Everything. Backed up. |
| **The user log** | `durable/log/<stream>/<yyyy-mm>.seg` | Append-only segment files of framed, checksummed records (WP-035) | Only appended. Unknown record types are kept and passed through. Erasure is the one sanctioned rewrite (privacy-and-data-protection.md, section 4). | Everything. Backed up. |
| **The derived-data store** (from R1.1, WP-071) | `derived/derived.db` | SQLite | Keyed by content identity, producer kind and producer version, so it never needs migrating: a new producer version simply writes new keys. R1 has no producer that needs it (provider lookups, the parser-upgrade re-read and loudness analysis are all after R1), so R1 creates the empty `derived/` directory (WP-126) and no file in it. | Rebuilds. Optional in backups (ADM-141, R1.3). |

The audit log is a fifth, simpler store: JSON-lines segments of 16 MB in
`durable/audit/`, with `fsync` before critical actions return
(operations-and-incident-response.md, section 3). Each record carries a
sequence number and the hash of its predecessor, the server signs
checkpoints with a dedicated audit key, every backup carries the latest
checkpoint, and a pre-allocated reserve lets the recovery actions write
their records on a full disk (SEC-OPS-020, SEC-OPS-021, SEC-OPS-023,
SEC-OPS-024, SEC-IAM-094; WP-069). A record never holds a source address
itself: it holds a keyed commitment to the address and a per-record
random salt, and the address lives in a side store that is coarsened
after 30 days and deleted at 90, each run recorded as a signed
checkpoint, so the privacy schedule and the hash chain do not conflict
(SEC-PRV-003, SEC-PRV-005).

Every column in every store, and every log and event field, carries one
data class from the privacy baseline, declared next to its schema part,
and that class decides logging, backup encryption, export and admin
visibility (SEC-TM-050, SEC-PRV-001; WP-122). Public identifiers are
random, minted once from the CSPRNG and kept in the identity store's
public-ID mapping, so they survive cache rebuilds without being derived
from paths or names (SEC-HIS-012, SEC-API-023; owner decision 11). Only
the secrets crate's minting function (WP-047) can produce the value a
`PublicId` is built from.

### The cache

- **One writer.** A dedicated thread owns the only read-write connection
  and receives work as boxed closures over a channel, each run in one
  transaction, with the result returned on a one-shot channel. Readers use a
  small pool of read-only connections from blocking tasks (ADM-080).
- **Batches.** The scan commits in batches bounded by count and by time
  (proposed: 500 files or 250 ms, whichever comes first), so readers never
  wait long and the library fills in while the first scan runs (LIB-021).
- **Pragmas.** `journal_mode=WAL`, `synchronous=NORMAL` for the cache,
  `foreign_keys=ON`, `secure_delete=ON` (SEC-PRV-050), and a busy timeout
  on readers only. Every database, the cache, the identity store and the
  derived-data store alike, is opened through the one connection opener in
  `gunmetal-fs` (WP-126), which sets these pragmas and checks them on every
  pooled connection.
- **Schema by parts.** Each package that needs cache tables owns a SQL file
  in its own module and registers it as a `SchemaPart { name, sql }`. The
  store computes the schema digest as SHA-256 over the parts in name order.
  Any change to any part changes the digest, so the cache rebuilds on the
  next start without anyone numbering a migration. This is what lets twenty
  packages add tables in parallel.
- **Generation.** Each cache build gets a random generation ID. The change
  log's cursors carry it, so a device holding a cursor from an older
  generation is told to take a fresh snapshot rather than receiving a delta
  that no longer means anything.

### The identity store

- **Before R1 ships,** there is no released file to migrate, so the identity
  store is built from per-domain schema parts in the same way as the cache,
  and a development build that finds a different digest refuses to start
  and says to delete the development data directory.
- **At the R1 release,** the release package freezes the parts into
  migration `0001`. From then on every change is a numbered migration
  assigned at merge time by the integrator, never chosen by the package
  author, so parallel packages cannot collide on a number.
- **The runner** follows operations-and-incident-response.md, section 8:
  snapshot with the SQLite online backup API, `integrity_check` on the
  snapshot, migrate in one transaction, check invariants (an owner exists;
  every security setting is valid), commit, write the audit event. Each
  migration is tested against a fixture database built by running every
  earlier migration on synthetic rows.

### The user log

Its format is set by ADR 3 (WP-002). This plan assumes the shape the
security baseline and the feature map already describe: one stream per
profile and one household curation stream; monthly segments; each record
framed with a length, a CRC-32C checksum and a version; each event with a
random 128-bit ID, a hybrid logical clock, the device ID and a typed body.
A torn record at the end of a segment is cut back at startup; damage
elsewhere is reported with what it held (ADM-078). The cache holds
projections of the log (play counts, the current queue, playlists) and is
rebuilt by replaying it.

**Proposal: versioned documents in the log.** The queue, playlists, Home
layouts and rule trees are stored as operations in the log, with a full
snapshot record every so often (proposed: every 100 operations), so replay
stays fast and the cache can be rebuilt from the latest snapshot plus the
operations after it. High-rate position updates are not individual events:
the playing device sends them at state changes and periodically, and the
server coalesces them into one record per batch window (API-QUE-04).

## Proposed external crates

The project prefers pure Rust and few dependencies. Each crate below is a
proposal with its reason; each goes through the review checklist in
[supply-chain-and-release.md](../security/supply-chain-and-release.md)
(cargo-deny, cargo-vet, the seven-day age rule) before it lands. Crates the
plan deliberately does not use are listed after the table. Versions are not
pinned here; WP-001 pins them once the owner approves the list.

| Crate | Used by | Reason | Alternative considered |
|---|---|---|---|
| `proptest` | all, dev | Already in use for property tests. | None. |
| `serde` (with derive) | core, server | Protocol types must serialise to JSON for the public API and to a compact binary form for sync and IPC. Derived code is not hand-written code, so it adds nothing to the mutation burden. | A hand-written codec for every type, which multiplies the code under the gate. |
| `postcard` | core | Compact, `no_std`, serde-based binary encoding for sync frames and worker messages, decoded inside the core's 32 MiB frame cap and then revalidated through typed constructors (SEC-MED-023). | A hand-written length-prefixed codec (owner decision 4). |
| `serde_json` | server, durable, worker | JSON for the HTTP API, the audit log and exports. Uploaded history exports are hostile input, so from R1.1 they are decoded only in the worker's import job (WP-145), never in the server process (SEC-MED-018). | None reasonable. |
| `sha2` (RustCrypto) | core (only in `crypto.rs`, WP-122), secrets (only in `crypto/`, WP-047) | SHA-256 for content identity windows and the schema digest. Pure Rust. `sha1` was dropped: it was only for TOTP, which the security baseline rules out (SEC-IAM-025). | `ring`, which includes C and assembly. |
| `hmac` (RustCrypto) | secrets (only in `crypto/`, WP-047) | HMAC-SHA-256 for capability URLs, token hashes and the recovery-code pepper. It left the core: core functions take a `MacProvider` (WP-031) and never compute a MAC themselves, so the core needs no MAC crate (SEC-STD-018). | `ring`. |
| `unicode-normalization` | core | Diacritic-insensitive search and natural sort (DIS-085, MUS-020) need canonical decomposition. Pure Rust tables. | A hand-maintained folding table, which is smaller but will be wrong for scripts nobody tested. |
| `tokio` | server, egress, fs | The async runtime under the HTTP server, timers and the blocking pool. | None realistic for axum. |
| `axum` (on `hyper` and `tower`) | http, server | The security baseline names it as a fit: composable middleware and an enumerable router. Its server-sent events support covers the R1 event channel without a WebSocket dependency. | Hand-rolled `hyper` service, which is more code under the gate for no security gain. |
| `rusqlite` with `bundled` | fs (the one connection opener, WP-126), store, durable | SQLite, with the amalgamation built in so every platform runs the same version. The supply-chain document already expects this bundled C code. | A pure-Rust SQLite reimplementation, which is not mature enough for the only durable store (unverified for every candidate). |
| `cap-std` | fs | Opening files beneath a library root or the data root with `openat2` and `RESOLVE_BENEATH` (SEC-MED-033, SEC-HIS-016). | Hand-written `openat2` calls through `rustix`. |
| `rustix` | fs, worker | Safe wrappers for `pread`, `fstat`, descriptor passing with `SCM_RIGHTS`, rlimits and `prctl`, with no `unsafe` in Gunmetal's code. | `nix`, which is broader. |
| `landlock` | worker | Filesystem and network confinement of the worker (SEC-MED-022). Maintained under the Landlock project. | None. |
| `seccompiler` | worker | Pure-Rust seccomp-bpf filters (SEC-MED-022). It does not support 32-bit ARM (media-and-parser-safety.md, section 4). | `libseccomp`, which is C. |
| `argon2` (RustCrypto) | secrets (only in `crypto/`) | Argon2id (RFC 9106) at or above the second recommended parameter set, for any key or verifier derived from a human-chosen secret (SEC-STD-024). R1 has no such secret: there are no account passwords (SEC-IAM-025), backups need no passphrase (ADM-068), and the one human-chosen secret, the optional share-link password (SEC-STD-008), arrives with share links in R1.2 (WP-134). The helper and its parameter floor still land in R1 (WP-047), because SEC-STD-024 stays an R1 requirement and the floor must be in one place before any caller exists. | None. |
| `hkdf`, `chacha20poly1305` (RustCrypto) | secrets (only in `crypto/`) | Deriving purpose keys from the root secret; encrypting third-party secrets at rest with XChaCha20-Poly1305 and 192-bit random nonces, the nonce strategy SEC-STD-020 allows. | `ring`. |
| `ed25519-dalek` | secrets (only in `crypto/`) | Signing backups and audit checkpoints; verifying the update feed's TUF metadata through a keyless verify function the server calls. As first written the server used it directly, outside any crypto module (SEC-STD-018). | `ring`. |
| `age` (or a reviewed age v1 implementation) | secrets (only in `crypto/`) | Backups encrypted in age v1 to the server's backup key and the owner's recovery key, which the baseline requires (SEC-OPS-042, SEC-PRV-039). The backup package (WP-090) calls the wrapper. | A new envelope format, which the baseline rules out. |
| `p256` (RustCrypto) | secrets (only in `crypto/`) | Verifying ES256 passkey assertions, through a keyless verify function that the passkey package (WP-081) calls. | See owner decision 9 for RS256. |
| `getrandom` | secrets (the one CSPRNG function, WP-047) | The operating system's CSPRNG. The server calls the secrets crate's function, never `getrandom` itself (SEC-STD-022). Already in the lock file through `proptest`. | None. |
| `toml` | server | The configuration file (ADM-007). | A hand-written key-value format, which users would have to learn. |
| `image` (JPEG, PNG, WebP, GIF only) | worker | Decoding artwork with `image::Limits` and re-encoding derivatives (SEC-MED-044 to SEC-MED-046, SEC-API-086). Runs only inside the worker, after the dimension check and, for PNG, after the decompression helper has bounded the inflated size (WP-128). Admitted only after the review SEC-MED-026 requires. | None in pure Rust with this coverage. |
| An inflate crate such as `miniz_oxide` | core | The one streaming decompression helper (WP-128, SEC-MED-009). Pure Rust. Joins the core's reviewed dependency allowlist (SEC-SUP-025) only if the owner approves it under decision 4. | A hand-written inflater, which is more code under the gate. |
| `notify` | fs | File-change notification on Linux, macOS and Windows (LIB-014). | `rustix` inotify, Linux only (owner decision 18). |
| `rustls`, `tokio-rustls` | server, egress; provider and configuration constructors only in secrets' `crypto/` | HTTPS with the owner's certificate (ACC-098) and outbound TLS, verified against the WebPKI with no "dangerous" configuration (SEC-NET-009). The crypto provider comes from the cryptography allow-list record (WP-125, SEC-STD-019; owner decision 8), and the server and egress client receive ready-made `ServerConfig` and `ClientConfig` values from the secrets crate rather than choosing a provider themselves (SEC-STD-018). | None in pure Rust. |
| `wasm-bindgen` | wasm | The web client calls the core through it (ADR 1, decision 2). | None. |
| `symphonia` | worker | Only if ADR 5 is accepted, and not before R1.3, where loudness measurement lands (WP-029, WP-114): decoding untagged FLAC, MP3, AAC and Vorbis for loudness measurement. The research notes it lacks Opus and HE-AAC (music.md, MUS-086). | No measurement in R1; tags plus the fallback gain (MUS-089). |

**Missing from this table, and needed.** Two R1 packages and one R1.2
package need something the table does not list. Each is part of owner
decision 4 or the decision named:

- **RSA signature verification for single sign-on (WP-096, R1.2).** The OpenID
  Connect Core specification makes RS256 the algorithm providers must
  support, and common self-hosted providers sign ID tokens with it by
  default (unverified per provider). Without an RSA verifier, R1.2's single
  sign-on works only with providers configured for ES256 or EdDSA. The
  candidate is the RustCrypto `rsa` crate, used for verification only;
  its past timing advisory concerned decryption (unverified whether it
  affects verification). Owner decision 10.
- **A backup archive container (WP-090, WP-109).** Encrypting with `age`
  still needs an archive format inside it, either a small hand-written
  one or the `tar` crate. SEC-HIS-019 forbids extracting archives
  received from outside until an architecture record allows it with
  ignored entry paths, no symlink entries and limits on entry count and
  unpacked size; WP-125 writes that record, and restore (WP-109) waits for
  it. Owner decision 12.
- **ACME (WP-101).** Account keys, certificate requests and signed ACME
  messages need either a crate such as `instant-acme` with `rcgen`, or a
  hand-written client on the RustCrypto primitives above. Neither is
  sized here. Owner decision 21.

**Not used, on purpose (Proposal, part of owner decision 4).** No `chrono`, `time` or `jiff`: the server needs
only Unix timestamps and RFC 3339 formatting, which the core implements and
tests (WP-005), and history by date is computed on the device in the
person's time zone. No `clap`: the command line has a handful of fixed
subcommands. No `tracing-subscriber`, `log` or `env_logger`: one small
structured logger is easier to make satisfy the escaping rules than a
configurable one. No `anyhow` or `thiserror`. No `uuid`: MusicBrainz IDs are
parsed into 16 bytes by hand. No `base64` crate: two small codecs (standard
for embedded pictures, URL-safe for tokens) are cheaper to own. No
`tempfile`: the testkit creates temporary directories. No `reqwest`: the
egress crate uses `hyper`'s client directly. Compressed ID3v2 frames are
rare, so R1 skips and records them rather than decompressing them (owner
decision 20). PNG still inflates, inside `image` in the worker, and the
security baseline requires every decompression to go through one bounded
helper (SEC-MED-009), so WP-128 adds that helper and the PNG path checks
the inflated size through it before `image` decodes.

**Open on purpose.** WebAuthn verification and OIDC each have a crate that
would do the job (`webauthn-rs`, `openidconnect`) and a smaller
hand-written path on top of the RustCrypto primitives above. The plan
sizes the hand-written paths and leaves the choice to the owner
(decisions 9 and 10). One fact matters to the choice and was not checked
here: whether `webauthn-rs` still depends on OpenSSL (unverified). The
update feed and backup encryption are no longer open: the baseline fixes
TUF 1.0 metadata for the feed (SEC-SUP-049, SEC-OPS-019) and age v1 for
backups (SEC-OPS-042); the remaining choice is whether `tough` passes
review or a minimal TUF client is written (owner decision 12).

## Working in parallel: shared files and merging

### Ownership

Every path in the repository has at most one owner per wave. The packages
list their owned paths exactly. The only files that more than one package in
a wave may touch are the **registry files** below, and only in the narrow
way described.

| Shared file | Owner | What other packages may do |
|---|---|---|
| Root `Cargo.toml` | WP-001 only, in every wave | Nothing. A package that needs a crate not already in `[workspace.dependencies]` stops and files a dependency request; the owner approves it and the integrator adds it between merges. New crates need no root edit, because `members = ["crates/*"]` already picks them up. |
| `supply-chain/` (cargo-vet `audits.toml`, `config.toml` and exemptions, `exceptions.toml`, the core dependency allowlist) | WP-001 in wave 0, then the integrator | Nothing. The cargo-vet audit or exemption for a new crate, and any change to the core allowlist, are part of the dependency request above: the integrator adds them in the same step as the root `Cargo.toml` line, so the eight or so wave 1 packages that add crates never edit these files in parallel (SEC-SUP-024, SEC-SUP-025). The one exception is `supply-chain/js-direct-deps.toml`, which WP-124 creates and which a package adding a direct JavaScript dependency extends by one sorted line with its reason (SEC-SUP-035). |
| `.github/CODEOWNERS` | WP-124 | WP-124 lists every security-sensitive directory the plan creates before it exists (see WP-124). A package that creates a security-sensitive path the list does not cover adds one sorted line for it (SEC-STD-035). |
| `REUSE.toml` | WP-124 | WP-124 writes it with a `fuzz/seeds/**` annotation from the start, so the parser packages' binary seeds are covered without an edit. A package that adds another binary file outside a covered glob adds a `.license` sidecar next to it, which it owns (SEC-SUP-030). |
| `crates/gunmetal-core/src/audit_event.rs` | WP-006 | Add one security-event variant per line, sorted, with its OWASP Logging Vocabulary name. The audit log (WP-069) records every variant; route specs (WP-044) name them. |
| `crates/gunmetal-server/openapi.json` | Generated; nobody edits it | WP-118 generates it from the route table and CI fails when the committed copy differs (SEC-API-091). On a conflict, regenerate it with the xtask command; never merge it by hand. |
| `crates/gunmetal-server/tests/rivals/` | WP-131 owns the harness (`main.rs`) | A registry directory: a feature package adds one replay file per rival incident its feature touches, named after the incident and citing its SEC-HIS requirement, and lists SEC-HIS-066 in its own Security field. |
| A crate's `Cargo.toml` | The package that creates the crate | Add one dependency line, in sorted position, using `workspace = true`. Never change another line. |
| `lib.rs` and every `mod.rs` | Nobody; these are registries | They hold only `mod` and `pub mod` lines and re-exports, never code, so a module's types live in a named file beside it (for example `formats/flac/metadata.rs`, never `formats/flac/mod.rs`). Any package may create a missing one; two packages that both create it resolve the add/add conflict by keeping every line and re-sorting. This is what lets WP-012 and WP-013, or WP-017 and WP-018, share a directory in the same wave. |
| `crates/gunmetal-fuzz/src/lib.rs`, `crates/gunmetal-fuzz/src/registry.rs` and `fuzz/Cargo.toml` | WP-008 | Add one `pub mod` line per harness module, one sorted name in `registry.rs`'s `harnesses!` list, and one `[[bin]]` block per cargo-fuzz target. Each parser package owns its own `crates/gunmetal-fuzz/src/<parser>.rs`, `crates/gunmetal-fuzz/tests/<parser>_corpus.rs`, `fuzz/fuzz_targets/<parser>.rs` and `fuzz/seeds/<parser>/`. |
| `crates/xtask/src/main.rs` | WP-008 | Add one dispatch line per subcommand; each check lives in its own file owned by the package that adds it. |
| `crates/gunmetal-server/security/public-routes.txt` | WP-118 (wave 2) | A route package that adds a public, credential-exchange or capability route adds its line, sorted, and runs WP-118's allow-list check and anonymous suite before merging. The file exists from wave 2, so no wave 3 route package depends on a same-wave peer to create it. It is under CODEOWNERS, so every change needs a maintainer's review (SEC-API-002). |
| `crates/gunmetal-server/limits.toml` | Nobody; a registry | Any package may create it, as for `mod.rs`; WP-130's xtask check reads it. A package that enforces a business limit adds its entry, sorted, with the test that enforces it (SEC-STD-030). WP-064 and WP-130 both add entries in wave 2. |
| `crates/gunmetal-worker/src/sandbox/programs.rs` | WP-045 | The closed list of programs the launcher may start. R1 has one, the worker itself; a package that needs another (R2's transcoder) adds one variant with its review. |
| `crates/gunmetal-testkit/src/lib.rs` | WP-007 | Add one `pub mod` line per builder module. |
| `crates/gunmetal-server/src/app.rs` | WP-043 | Add one line to register a module's state, schema parts or jobs. |
| `crates/gunmetal-server/src/routes.rs` | WP-118 | Add one line to register a module's routes. |
| `crates/gunmetal-server/src/cli.rs` | WP-043 | WP-043 parses every subcommand; later packages add one dispatch line that sends their subcommand to their own module (`doctor` to WP-116, `admin recover` to WP-106, `migrate`, `rebuild` and `snapshot restore` to WP-095, `restore` to WP-109, the hidden worker entry to WP-061, `service` to WP-121, `audit verify` to WP-069, `keys rotate` to WP-106). No subcommand accepts a secret as an argument (SEC-OPS-014). |
| `crates/gunmetal-core/src/problem.rs` | WP-006 | Add one problem code per line, sorted. |
| `scripts/gate.sh`, `.github/workflows/ci.yml` | The integrator | Packages do not edit them. A package that needs a gate change (a cargo feature turned on, a new target) files a request and the integrator applies it between merges, as for the root `Cargo.toml`. A package that needs its own CI job adds its own workflow file instead (WP-008 owns `fuzz.yml`, WP-088 owns `wasm.yml`, WP-121 owns `release.yml` until WP-136 takes it over in wave 5, WP-124 owns the repository-security workflows, WP-127 owns `docs.yml`). Every workflow sets `permissions: {}` at the top, pins actions by commit SHA and passes zizmor (SEC-SUP-010 to SEC-SUP-014). |
| `clippy.toml`, `deny.toml` | WP-001, then the integrator | A package that needs a sanctioned exception to a ban asks for it; the exception is a narrowly scoped `#[expect]` in the one owning module, and the xtask exception list (WP-008) names it. |
| `Cargo.lock` | Nobody | On a conflict, take either side and run `cargo update --workspace`, which re-resolves only the workspace members. Never run a plain `cargo update`. |

**Routes need WP-118.** Every package that registers an HTTP route depends
on WP-118, which owns the route registry and the listener. That is why the
route-owning packages that were once in wave 2 (WP-072 to WP-074) are now in
wave 3.

### The merge protocol (Proposal)

1. Each wave has an integration branch. Packages branch from it and merge
   back into it.
2. Before merging, the package rebases onto the current integration branch
   and runs the full gate. A conflict in a registry file is resolved by
   keeping both lines and re-sorting. A conflict anywhere else means two
   packages own the same path, which is a bug in this plan: stop and ask.
3. A package that needs to change an interface another merged package
   already uses opens the change as its own small package, owned by the
   original author's paths, rather than editing them in passing.
4. A wave closes when every package in it has merged and the full gate
   passes on the integration branch. The integration branch then merges to
   `main`.
5. The integrator is one named person or agent per wave. They own the root
   `Cargo.toml`, resolve registry conflicts, and run the full gate. Who that
   is, is owner decision 25.

### Gate time while many agents work

While developing, an agent may run the mutation tool on its own files only.
The definition of done is still the full `scripts/gate.sh`. Owner decision 6
proposes how the gate itself scales once the workspace grows. Since wave 1,
CI runs the gate's mutation step, diff-scoped or full, as ten shards in
parallel jobs (`GATE_MUTANTS_SHARD`), which together test every mutant
([technical answer of 2026-10-03](../decisions.md#technical-answer-on-how-the-full-gate-scales-2026-10-03)).

## Waves at a glance

| Wave | Packages | Count | What it produces |
|---|---|---:|---|
| 0 | WP-001 to WP-008, WP-122, WP-125, WP-126 | 11 | Lints and merge rules, ADR 3 to 6, the security architecture records, the parse contract, text and typed values, identifiers and the problem catalogue, the testkit, the fuzz harness registry, the schema digest with data classes, the data-root handle (unchanged by the adopted R1) |
| 1 | WP-009 to WP-021, WP-023 to WP-026, WP-028, WP-030 to WP-047, WP-124, WP-127, WP-128, WP-138, WP-139 | 41 | Every R1 container parser and lyrics, the pure logic of the queue, shuffle, gain, tokens, authorisation, retention and the user log; the store, server, HTTP, worker, identity-store and secrets crates; repository protections, docs lints and traceability, the decompression helper, the project site's security files |
| 2 | WP-048 to WP-056, WP-059, WP-060 to WP-062, WP-064 to WP-070, WP-118, WP-119, WP-130, WP-235, WP-236, WP-238 to WP-240, WP-242 | 29 | The egress client; tag mapping, the file probe, search, the decision engine, the audio packager, Home rows; file access, worker IPC, sessions and fresh user verification, the credential verifier, the authorisation layer, the change log, the catalogue store, the user log, audit log and its sink, the task runner; the route registry, the listener, the client-address resolver, the public-route allow-list and the anonymous suite; the synthetic library generator; request limits; the WASM facade crate and its playback slice |
| 3 | WP-063, WP-072 to WP-090, WP-093 to WP-095, WP-097 to WP-100, WP-120, WP-131, WP-132, WP-237, WP-241 | 32 | Recovery codes, web assets, owner HTTPS and the proxy and tailnet recipes, the update check, the music model, identity and scan diff, the worker pool and jobs, setup, passkeys, browser pairing, streaming, the event channel, sync, the queue and listening services, accounts, the rest of the WASM facade, server facts, backups, playlists, users and invitations, startup, alerts, triggers, library administration, job activity and the audit routes; the route-table security suites; postures and the cleartext rule |
| 4 | WP-101 to WP-106, WP-108, WP-121, WP-133 | 9 | ACME for the owner's own domain, the scan pipeline and what needs it (artwork serving, the playback registry and stream limits, the packaging route and its worker job, account recovery), each person's data export, release builds and service install, history deletion and retention |
| 5 | WP-109 to WP-111, WP-115, WP-136 | 5 | Restore from the command line and the welcome screen, library health, the trash and purge, the scan benchmark and the speed budget tests, release provenance and signing |
| 6 | WP-116, WP-117 | 2 | Doctor and the security summary, and the R1 flow acceptance tests |

129 packages build R1 (eight of them, WP-235 to WP-242, were added on
2026-10-03 at the client plan's request: the WASM facade, once WP-088
alone, is now built in slices, and five small core packages hold rules
the web client must not write in TypeScript). The 40 packages that serve only R1.1, R1.2, R1.3
or R2 (18 moved whole, counting WP-091, which was already R2, and 22 new
packages split from R1 packages, WP-140 to WP-161) are in
[After R1](#after-r1-point-releases-and-later), grouped by release; R2's
video packages are in [R2 (video) in outline](#r2-video-in-outline).

The adopted R1 (register D-10, D-07) changed the waves in four ways. The
packages that serve only a point release or R2 left the waves.
Thirty-three R1 packages kept their R1 part and gave their later part to
a split package or to a moved one (for example WP-024's exclusion patterns to WP-140, WP-104's admin live
view to WP-153, WP-116's diagnostic bundle to WP-155). WP-059 (Home rows)
moved back to wave 2 and WP-088 (the WASM facade) back to wave 3, because
the radio and rule packages that held them a wave later left R1. And
WP-129's project-site files split off to WP-139 in wave 1, while the name
service itself went to R2. WP-101 (ACME) stays in wave 4: it is now the
R1 path to HTTPS for an owner's own domain, but setup (WP-080) needs only
a secure context and does not call it, and WP-101 hands certificates to
WP-073's listener configuration, which is in wave 3.

WP-091 (scoped tokens) moved to R2 when the plan was aligned with the
security baseline, and its entry is now in the R2 group of
[After R1](#after-r1-point-releases-and-later).

Wave 1 is the widest point: forty-one packages that need nothing but wave 0.
That is where most parallel agents are useful. The critical path to a
running music server is WP-004 → WP-012 → WP-052 → WP-075 → WP-102, with the
worker chain WP-045 → WP-061 → WP-078 → WP-102 and the store chain
WP-122 → WP-042 → WP-067 → WP-102 beside it.

WP-048 (the egress client) is numbered in the wave 1 section but runs in
wave 2, WP-059 (Home rows) is numbered in the wave 2 section and runs in
wave 2, WP-088 (the WASM facade) is numbered in the wave 3 section and
runs in wave 3, WP-109 (restore) is numbered in the wave 4 section but
runs in wave 5, and WP-138 (the retention schedule) and WP-139 (the
project site's security files) are at the end of the wave 1 section.

**Wave 0 and the lints.** WP-004 to WP-008, WP-122 and WP-126 run beside WP-001,
which turns on the core's deny-level lints. They write their code to the
lint list in WP-001 from the first commit (the list is spelled out there),
so the order in which wave 0 merges does not matter; the gate run on rebase
is what catches a mismatch.

## Wave 0: what everyone agrees on first

### WP-001 Workspace rules, lints and the merge protocol

- **Wave** 0 · **Size** M · **Depends on** nothing.
- **Owns** `Cargo.toml`, `clippy.toml`, `deny.toml`, `rust-toolchain.toml`,
  `supply-chain/` (cargo-vet audits and `exceptions.toml`; in wave 0 only,
  after which the integrator updates it with each approved dependency, see
  the shared-file table),
  `.github/workflows/ci.yml`, `.github/workflows/supply-chain.yml`,
  `scripts/gate.sh`,
  `crates/gunmetal-core/Cargo.toml`, `crates/gunmetal-core/src/lib.rs`,
  `crates/gunmetal-core/src/ebml.rs`, and a new "Working in parallel"
  section of `CONTRIBUTING.md`. Several of these already exist (the
  pinned toolchain, cargo-deny, cargo-vet, the first `clippy.toml` bans
  and `overflow-checks`); this package extends them rather than starting
  over.
- **Serves** the engineering standards in the README; the "add to the
  repository now" items 6 to 9 of the security baseline.
- **Security.** Boundaries TB12; threats TM-T20, TM-T41, TM-T45. Verifies
  SEC-MED-002, SEC-MED-003, SEC-MED-004, SEC-MED-025, SEC-MED-033,
  SEC-MED-063, SEC-SUP-020, SEC-SUP-021, SEC-SUP-022, SEC-SUP-023,
  SEC-SUP-024, SEC-SUP-025, SEC-SUP-026, SEC-SUP-029, SEC-SUP-038,
  SEC-EXT-001, SEC-EXT-018, SEC-API-076, SEC-TM-006, SEC-TM-034, SEC-TM-038,
  SEC-TM-039, SEC-HIS-015, SEC-HIS-019, SEC-HIS-026, SEC-HIS-034,
  SEC-HIS-035, SEC-HIS-052, SEC-HIS-056, SEC-NET-030, SEC-NET-059,
  SEC-OPS-039, SEC-STD-010, SEC-STD-011, SEC-STD-018, SEC-STD-019,
  SEC-STD-022, SEC-STD-033, SEC-PRV-009, SEC-OPS-037 (the lint part),
  SEC-NET-054 and SEC-OPS-040 (the R1 absence proof).
- **Scope.** Turn on the core's deny-level lints (`unwrap_used`,
  `expect_used`, `panic`, `unreachable`, `todo`, `unimplemented`,
  `indexing_slicing`, `arithmetic_side_effects`, `large_stack_arrays`) and
  bring `ebml.rs` into line with them, replacing each flagged slice with
  `get`, `split_at_checked` or an `#[expect]` naming its invariant, as
  media-and-parser-safety.md section 2 describes. Add `clippy.toml` entries
  that ban pre-sizing calls in the core, path-based `std::fs` outside
  `gunmetal-fs`, `std::process::Command` outside the worker's sandbox
  launcher, HTTP clients and outbound sockets outside `gunmetal-egress`,
  rustls "dangerous" verifier APIs everywhere, non-cryptographic and
  seeded randomness outside the one CSPRNG function, SQL built from
  strings, `rusqlite::Connection` opens outside the one connection opener
  in `gunmetal-fs` (WP-126), and `Secret::expose` outside
  `gunmetal-secrets`. The cryptography door (SEC-STD-018): the types and
  functions of `sha2` outside `gunmetal-core/src/crypto.rs` and
  `gunmetal-secrets/src/crypto/`, and of `hmac`, `hkdf`,
  `chacha20poly1305`, `argon2`, `p256`, `ed25519-dalek` and `age`, and
  the rustls crypto-provider and configuration-builder constructors,
  outside `gunmetal-secrets/src/crypto/`, as `disallowed-types` and
  `disallowed-methods` entries, so every cryptographic call site is in
  one of two named files and the inventory (WP-125, WP-136) can be
  checked against them mechanically. The client-address door
  (SEC-OPS-037): `axum::extract::ConnectInfo`, `peer_addr` on every
  socket type and direct reads of forwarding headers outside
  `crates/gunmetal-server/src/listener.rs` (WP-118) and the core's
  `http/forwarded.rs` (WP-023), so from wave 2 no module can take a
  client address except from the `ClientContext` the listener attaches.
  The public-ID door (SEC-HIS-012): the constructor of the `Minted` value
  a `PublicId` is built from, outside the secrets crate's minting
  function (WP-006, WP-047). A
  `disallowed-methods` entry applies to the whole workspace, so each
  sanctioned exception is a module-level `#[expect(clippy::disallowed_methods, reason = "…")]`
  and an xtask check (WP-008) fails when an exception appears in any module
  not on a short written list. The security baseline bans path-based
  `std::fs` outside the filesystem module and requires every file access,
  library or data directory, to go through a directory handle beneath its
  root (SEC-MED-033, SEC-HIS-016), so the data directory's own files (the
  cache and identity databases, log and audit segments, the root secret,
  the data-directory layout) are written through the data-root handle in
  `gunmetal-fs` (WP-126), not through `std::fs`. SQLite's own open, which
  takes a path the data-root handle builds from constants, now happens
  only inside `gunmetal-fs` (WP-126). The first draft said the written
  exception list would hold one entry, the dev-only `gunmetal-testkit`
  (owner decision 33). The xtask list (`EXCEPTIONS` in
  `crates/xtask/src/lint_exceptions.rs`) now has eleven entries: the
  doors that exist, plus the testkit, the fuzz corpus replay, the
  xtask's tree module, the filesystem tests, the core's minting test
  helper and the core's bounded-capacity helper. Record 6 decision 8
  names them. Add
  `deny.toml` bans for port-mapping and UPnP, SSDP, archive-extraction,
  dynamic-loading, plugin-runtime, XML-with-DTD, unsafe-deserialisation,
  SAML, LDAP, GraphQL, WebRTC, telemetry and backtracking-regex crates,
  for C media, image, font and subtitle libraries, and, in R1, for `iroh`
  and its port-mapper and relay crates; in R1 these bans are how the
  requirements for features that do not exist yet are verified (no XML
  parser, no archive extraction, no plugin host, and no iroh endpoint,
  so SEC-NET-054 and SEC-OPS-040 hold by absence until WP-221 lifts the
  iroh ban in R2 and verifies them on the real endpoint).
  `overflow-checks = true` is already in the release profile. Add a 32-bit
  CI job that runs the core's tests on `i686-unknown-linux-gnu`. Keep
  `cargo-deny` (advisories, bans, licences, sources, build scripts) and
  `cargo-vet` passing while the graph is tiny, and write the core's
  reviewed dependency allowlist into `supply-chain/` (WP-008's xtask
  checks the core's normal dependencies against it). Write the
  approved crate list into `[workspace.dependencies]` once the owner
  approves it. Write the
  merge protocol into CONTRIBUTING.md once the owner accepts it (owner
  decision 25); CONTRIBUTING.md holds the project's binding rules, so an
  agent does not change them on its own say-so. After wave 0, `gate.sh`
  and `ci.yml` pass to the integrator (see the registry table).
- **Not in scope.** Any new parser or module. The harness registry
  (WP-008).
- **Interface sketch.** No code interface. The lint table in
  `crates/gunmetal-core/Cargo.toml` and the bans in `clippy.toml` are the
  contract every later package compiles against.
- **Tests.** Every existing `ebml.rs` test passes unchanged; a refactor that
  needed a test change is wrong. The 32-bit job runs the existing property
  tests. A deliberately bad commit on a scratch branch (an `unwrap` in the
  core, a `std::fs::read` in a stand-in server module, a `ConnectInfo`
  extractor or a `peer_addr` call outside `listener.rs`, a `sha2` or
  `hmac` call outside the crypto modules, a `Minted` built outside the
  minting function, an `iroh` dependency) fails the lint or deny step;
  record the result in the pull request.
- **Risks and decisions.** The gate change for scale is owner decision 6.
  Turning on `arithmetic_side_effects` may make some later parser code
  noisy; the rule stands because the alternative is silent wrapping on
  32-bit targets.

### WP-002 ADR 3: durable user state

- **Wave** 0 · **Size** M · **Depends on** nothing.
- **Owns** `docs/adr/0003-durable-user-state.md`.
- **Serves** feature map open decision 1; ADM-077, ADM-078, ADM-141,
  ACC-013, LAT-006, LAT-007; api-needs.md Flags item 1.
- **Security.** Boundaries TB10; threats TM-T60, TM-T59. Verifies
  SEC-IAM-004, SEC-TM-051.
- **Scope.** Write the record that ADR 1 decision 5 needs extended: the user
  log (per-profile streams and the household curation stream, monthly
  segments, record framing, event envelope, hybrid logical clock, the
  documents-as-operations rule with periodic snapshots, the rule that
  unknown record types are preserved, erasure as the one sanctioned
  rewrite) and the identity store (accounts, credentials' public keys,
  sessions, devices, grants, invitations, tokens, settings, the public-ID
  mapping). It states one backup, export and replay path for both, and how
  the cache is rebuilt from them. It records which data lives where, using
  the table in [Databases, durable files and migrations](#databases-durable-files-and-migrations)
  as the starting point.
- **Not in scope.** Code. The rating model (owner decision 17) and the
  public-ID choice (owner decision 11), which the record names as inputs.
- **Tests.** Not code. The review checks that every R1 row in api-needs.md
  marked "Feed" or that writes user data has a home in the record.
- **Risks and decisions.** WP-034, WP-035, WP-046 and WP-068, and through
  them most of R1, wait for the owner to accept this record (owner
  decision 1).

### WP-003 ADRs 4 to 6: the audio packager worker, audio decoders in the scan worker, and the workspace

- **Wave** 0 · **Size** M · **Depends on** nothing.
- **Owns** `docs/adr/0004-audio-packager.md`,
  `docs/adr/0005-audio-decoders-in-the-scan-worker.md`,
  `docs/adr/0006-workspace-and-dependencies.md`.
- **Serves** feature map open decisions 8 and 9; MUS-067, MUS-086, MUS-230;
  owner decisions 4 and 5; register decision D-09.
- **Security.** Boundaries TB6, TB12; threats TM-T21, TM-T41. Verifies
  SEC-MED-018, SEC-MED-024, SEC-MED-026 (as review records: each ADR must
  state how its component meets them before it is accepted).
- **ADR 6.** Owner decision 5 asks for the crate layout to be recorded in
  an ADR that extends ADR 1 rather than rewriting it, and no package owned
  that record. ADR 6 records the crate table, the dependency direction, the
  approved external crates with their reasons, and the scoped exceptions to
  the `clippy.toml` bans (owner decision 33). Like ADRs 3 to 5, it is a
  draft until the owner accepts it; none of these packages edits ADR 1 or
  ADR 2 (AGENTS.md).
- **Scope.** ADR 4 amends ADR 2 decision 2 to name the light, audio-only
  packager (FLAC, Opus and MP3 frames copied into fragmented MP4 without
  re-encoding) as the one exception to "no remuxer for music", and records
  that it runs in a worker process, never in the server process: the
  server passes the file by descriptor, the worker runs the core packager
  under the step budget, a per-stream memory cap and a watchdog, and
  streams each segment back over its socket pair, where the server
  revalidates it before serving (SEC-MED-018, SEC-MED-023, SEC-MED-081).
  The first draft of this package named an in-process packager, which
  principle 2 of the security baseline rules out (register decision D-09).
  ADR 4 also records that CUE slices (MUS-041, R2) use the same worker
  path. ADR 5 decides whether a third-party pure-Rust decoder (the
  candidate is Symphonia, MPL-2.0) may decode untrusted audio for loudness
  measurement, and records that it runs only in the scan worker, at the
  isolation tier SEC-MED-024 gives memory-safe parsing, with its licence,
  fuzzing and resource limits, as SEC-MED-026 requires.
- **Not in scope.** The packager itself (WP-056), its worker job and route
  (WP-105) and the measurement job (WP-114).
- **Tests.** Not code.
- **Risks and decisions.** If ADR 4 is rejected, WP-056 and WP-105 drop out
  and browser gapless (MUS-067) rests on whatever each browser's Media
  Source Extensions accept natively, which is unverified per browser. If
  ADR 5 is rejected, WP-029 and WP-114 drop out and R1 uses loudness tags
  and the fallback gain (MUS-089), as D-09 says; measurement returns only
  with a later decoder record. SEC-MED-081 and SEC-MED-032 are R2 in the
  baseline while the packager worker ships in R1; D-09 and D-80 ask the
  security lead to move them to R1 or add an R1 row, and until then WP-105
  verifies them as if they were R1. Owner decisions 2 and 3.

### WP-004 Core parse contract

- **Wave** 0 · **Size** M · **Depends on** nothing.
- **Owns** `crates/gunmetal-core/src/parse/` (`crates/gunmetal-core/src/parse/mod.rs`, `crates/gunmetal-core/src/parse/limits.rs`,
  `crates/gunmetal-core/src/parse/budget.rs`, `crates/gunmetal-core/src/parse/cursor.rs`, `crates/gunmetal-core/src/parse/sansio.rs`, `crates/gunmetal-core/src/parse/capacity.rs`).
- **Serves** LIB-019, LIB-020; the parse contract in
  media-and-parser-safety.md, section 2.
- **Security.** Boundaries TB6, TB9; threats TM-T20, TM-T29. Verifies
  SEC-MED-001, SEC-MED-003, SEC-MED-004, SEC-MED-005, SEC-MED-006,
  SEC-MED-007, SEC-MED-008, SEC-MED-010, SEC-TM-032.
- **Scope.** The shared machinery every parser uses: `Limits` with the
  default values from media-and-parser-safety.md section 3 and compiled-in
  ceilings; a deterministic `Budget`; `Depth`; the one `bounded_capacity`
  helper; a checked byte `Cursor` for big- and little-endian integers,
  syncsafe integers and sub-slices; the sans-I/O protocol (a parser answers
  either "read this many bytes at this offset" or a final result); and an
  in-memory driver that tests and the worker host both use.
- **Not in scope.** Any format. Wall-clock deadlines, which belong to the
  worker host (WP-061).
- **Interface sketch.**

  ```rust
  pub struct Limits { pub max_depth: u8, pub max_children: u32, pub max_tag_fields: u32,
      pub short_text: u32, pub long_text: u32, pub max_picture: u32, pub max_pictures: u8,
      pub max_index_entries: u32, pub max_read: u32, pub max_bytes_per_file: u64, /* … */ }
  impl Limits { pub const DEFAULT: Self; pub fn with_override(self, o: &LimitOverrides) -> Result<Self, LimitError>; }
  pub struct Budget { /* steps left */ }
  impl Budget { pub fn for_input(len: u64, per_byte: u64, fixed: u64) -> Self;
      pub fn charge(&mut self, steps: u64, offset: u64) -> Result<(), ParseFault>; }
  pub struct Depth(u8);
  impl Depth { pub fn descend(self, limits: &Limits, offset: u64) -> Result<Self, ParseFault>; }
  pub fn bounded_capacity(declared: u64, min_item_len: u64, remaining: u64, ceiling: u64) -> usize;
  pub enum ParseFault { Truncated { offset: u64, needed: u64, available: u64 },
      BudgetExceeded { offset: u64 }, TooDeep { depth: u8, offset: u64 },
      LimitExceeded { limit: LimitKind, offset: u64, value: u64 } }
  pub struct ReadRequest { pub offset: u64, pub len: u32 }
  pub enum Step<T> { Need(ReadRequest), Done(T) }
  pub trait SansIo { type Output; fn resume(&mut self, window: Window<'_>) -> Step<Self::Output>; }
  pub fn drive<P: SansIo>(parser: P, file: &[u8], limits: &Limits) -> Result<P::Output, DriveError>;
  ```

- **Tests.** Unit: each default limit; an override above the ceiling is
  refused; an override below the default is accepted. `bounded_capacity`
  at each of its three bounds, at zero, and with a declared count of
  `u64::MAX`. `Budget` runs out at exactly the step it should. `Depth`
  refuses at the limit and not one below. `Cursor` reads every integer
  width in both byte orders and reports `Truncated` with exact offsets.
  `drive` refuses a request over 16 MiB, a request past the end of the
  file, and a parser that asks for more than the per-file byte cap.
  Properties: `bounded_capacity` never exceeds any of its three inputs; a
  `Cursor` never reads past its slice for arbitrary sequences of reads.
  Edge cases: zero-length input; a request for zero bytes; a file of length
  `u64::MAX` declared; a 32-bit `usize`.
- **Risks and decisions.** The default limits are owner decision 13. The
  per-parser step constants (k and c in SEC-MED-007) are set by each format
  package and documented next to its parser.

### WP-005 Core text, typed values, time, addresses and untrusted input

- **Wave** 0 · **Size** L (was M; it gained the untrusted-input wrapper
  and the link validators) · **Depends on** nothing.
- **Owns** `crates/gunmetal-core/src/text.rs`, `crates/gunmetal-core/src/values.rs`,
  `crates/gunmetal-core/src/time.rs`, `crates/gunmetal-core/src/base64.rs`, `crates/gunmetal-core/src/net.rs`,
  `crates/gunmetal-core/src/untrusted.rs`, `crates/gunmetal-core/src/link.rs`.
- **Serves** MUS-036, INT-009, MUS-013; CLI-150 and ACC-080's address
  warnings (the classifier).
- **Security.** Boundaries TB4, TB6, TB9; threats TM-T07, TM-T27, TM-T05.
  Verifies SEC-MED-013, SEC-MED-014, SEC-TM-031, SEC-API-047, SEC-API-048,
  SEC-API-070, SEC-CLI-002, SEC-HIS-032, SEC-STD-015, SEC-NET-025,
  SEC-API-077.
- **Scope.** Lossy decoding of UTF-8, UTF-16 with and without a byte-order
  mark, UTF-16BE and Latin-1; stripping C0 and C1 controls except tab and
  line feed; capping length after decoding and flagging truncation. Typed
  values that every parser and the music model share: MusicBrainz IDs,
  ISRCs, sample rates, channel counts, bit depths, track and disc numbers
  with totals, partial dates, ReplayGain and R128 gains and peaks, and
  durations. Time: a millisecond `Timestamp`, conversion between days and
  civil dates, RFC 3339 formatting and parsing, and the `Clock` trait the
  server implements. Two small shared codecs: standard and URL-safe base64.
  Addresses: IP network parsing and a classifier (loopback, private,
  link-local, shared address space, public) used by the forwarded-header
  parser, the egress guard, the secure-context report and invite warnings;
  it converts IPv4-mapped IPv6 to IPv4 first and classifies NAT64, 6to4,
  Teredo, multicast, unspecified and documentation ranges as non-local
  (SEC-NET-025), and the egress deny set is the IANA special-purpose
  registries (SEC-API-077). Untrusted input: the `Untrusted<T>` wrapper
  that every byte or string from outside arrives in, with no conversion to
  a path, a process argument, a log format string or SQL, so the only way
  out is a validating constructor of a domain type (SEC-TM-031); the
  ingest normaliser that also removes bidirectional overrides and
  isolates from single-line fields (SEC-API-048). Links: the one URL
  validator the clients use through WASM, which accepts only `http` and
  `https` with a non-empty host after a WHATWG-conformant parse
  (SEC-API-047, SEC-CLI-002, SEC-STD-015), and the post-sign-in return
  target validator, which accepts only a relative path starting with
  exactly one `/` that names a known client route (SEC-API-070,
  SEC-HIS-032).
- **Not in scope.** Collation and folding (WP-036). Time zones, which the
  device applies.
- **Interface sketch.**

  ```rust
  pub enum Encoding { Utf8, Utf16Bom, Utf16Be, Utf16Le, Latin1 }
  pub struct Text { pub value: String, pub truncated: bool, pub replaced: bool }
  pub fn decode(bytes: &[u8], enc: Encoding, cap: u32) -> Text;
  pub struct Mbid([u8; 16]); impl Mbid { pub fn parse(s: &str) -> Result<Self, ValueError>; }
  pub struct SampleRate(NonZeroU32); // at most 768,000
  pub struct NumberOf { pub n: u16, pub of: Option<u16> } // parses "3", "3/12", "03 of 12"
  pub struct PartialDate { pub year: i16, pub month: Option<u8>, pub day: Option<u8> }
  pub struct GainDb(f32); pub struct PeakRatio(f32); // finite, ranged
  pub struct Timestamp(i64); pub trait Clock { fn now(&self) -> Timestamp; }
  pub fn format_rfc3339(t: Timestamp) -> String; pub fn parse_rfc3339(s: &str) -> Result<Timestamp, ValueError>;
  pub fn b64_decode(input: &[u8], alphabet: Alphabet, max_out: usize) -> Result<Vec<u8>, B64Error>;
  pub fn classify(ip: IpAddr) -> AddrClass; pub struct IpNet { /* address and prefix */ }
  ```

- **Tests.** Unit: each encoding with valid text, invalid sequences, an odd
  byte count in UTF-16, a lone surrogate, a BOM of each order, and Latin-1
  bytes 0x80 to 0x9F (C1 controls, which must be stripped). Cap applied
  after decoding, shown with Latin-1 text that doubles in size. Each typed
  value at and either side of its range; MBIDs with upper case, braces and
  wrong lengths; ReplayGain strings such as "-6.5 dB", "+3.0 dB",
  "-6,5 dB" and "nan". Civil dates at 1970-01-01, 2000-02-29, 2100-02-28,
  a negative timestamp and the largest supported year. Properties: decoded
  text never contains a stripped control; never exceeds the cap; civil
  date conversion round-trips for every day in a wide range, checked
  against an independent day-counting oracle written in the test. Base64:
  the RFC 4648 test vectors, padding and no padding, whitespace, invalid
  characters, output over the cap; property: encode then decode returns
  the input. Addresses: every special-purpose range at its first and last
  address, IPv4-mapped IPv6, and malformed prefixes, checked against an
  independent table written from the IANA registries in the test.
  Untrusted: compile-fail tests (trybuild) that `Untrusted<T>` has no
  path, argument or display conversion. Links: property tests over
  `javascript:`, `data:`, `file:`, mixed-case schemes, `//host`, `/\host`
  and whitespace-padded values, each with its literal verdict; the
  normaliser is idempotent and its output holds no bidirectional control
  in a single-line field.
- **Risks.** Sample-rate and gain ranges become user-visible behaviour; the
  ranges come from SEC-MED-014 and SEC-MED-015.

### WP-006 Core identifiers and problem catalogue

- **Wave** 0 · **Size** M · **Depends on** nothing.
- **Owns** `crates/gunmetal-core/src/id.rs`, `crates/gunmetal-core/src/problem.rs`,
  `crates/gunmetal-core/src/client_context.rs`,
  `crates/gunmetal-core/src/audit_event.rs` (a registry file; see the
  shared-file table).
- **Serves** INT-008, ACC-121 (identifier format), the reasons admins can
  read, the problem-type catalogue in web-and-api-security.md.
- **Security.** Boundaries TB4, TB11; threats TM-T10, TM-T12, TM-T61.
  Verifies SEC-API-023, SEC-API-024, SEC-API-072, SEC-PRV-021 and
  SEC-HIS-012 (each for the encoding, the typing and "never derived from
  a path or name"; the "from the OS CSPRNG" part is WP-047's).
- **Scope.** Public identifiers: 16 random bytes from the CSPRNG (the
  server mints them; the core only encodes, parses and types them). A
  `PublicId` is built only from a `Minted` value, and the only
  constructor of `Minted` is called by the secrets crate's minting
  function (WP-047), enforced by a `disallowed-methods` entry (WP-001)
  checked by WP-008's exception list; parsing an identifier a client sent
  back is the only other way to hold one. As first written,
  `PublicId::new(kind, bytes)` was public, so any code could build an ID
  from a path hash or from zeros, which SEC-HIS-012 forbids. IDs are
  written as a kind prefix (`trk_`,
  `alb_`, `art_`, `rgp_`, `pls_`, `usr_`, `prf_`, `dev_`, `lib_`, `inv_`,
  `tok_`, and room for R2 kinds) plus 26 lowercase Crockford base32
  characters, so kind confusion is a parse error. The problem catalogue:
  a stable code for every error the system can report, its HTTP status
  where it has one, and its plain-language English text, plus the
  `Describe` trait every core error implements. The `ClientContext`
  type: a resolved client address and its path class (loopback, home,
  unknown, internet, and whether a trusted proxy carried it), with only a
  `pub(crate)` constructor, so that outside the core only the path-class
  function (WP-023) can produce one. The sessions, the credential
  verifier, the audit log and the rate limiters (WP-062, WP-064, WP-069,
  WP-130) accept only this type as their address input, so none of them
  can read a socket peer or a header itself (SEC-OPS-037). It lives here
  rather than beside the address classifier (WP-005) because the
  security events below carry it and both packages are in wave 0. The
  security-event catalogue: one closed enum of typed security events with
  OWASP Logging Vocabulary names, whose fields take a `ClientContext` and
  public IDs but no credential, and a `SecuritySink` trait that every
  producer writes to. It lives here, in wave 0, because WP-044's route
  specs name audit events in wave 1, and because the egress crate
  (WP-048) and the server's producers must emit the same typed events
  without depending on the audit store; as first written the catalogue
  belonged to the audit log (WP-069, wave 2), after the packages that
  named it. `SecuritySink::record` returns `Err(AuditUnavailable)` when
  the record cannot be written, and the producer then does not let its
  action take effect (SEC-OPS-020); `AuditUnavailable` describes itself
  as the `audit_unavailable` problem (503). A producer whose event records
  a refusal still refuses. As first sketched, `record` returned nothing,
  so no producer could learn that its record was lost. The sink the
  server wires to its bus (WP-043, WP-069) passes the audit log's answer
  back to the producer.
- **Not in scope.** Minting and storing IDs (WP-046 keeps the public-ID
  mapping; owner decision 11). Translations of the problem text (CLI-146
  builds on the codes).
- **Interface sketch.**

  ```rust
  pub enum IdKind { Track, Album, Artist, ReleaseGroup, Playlist, User, Profile, Device, Library, Invite, Token /* … */ }
  pub struct PublicId { kind: IdKind, bytes: [u8; 16] }
  pub struct Minted([u8; 16]); impl Minted { pub fn from_os_random(bytes: [u8; 16]) -> Self; } // disallowed outside WP-047's minting function
  impl PublicId { pub fn new(kind: IdKind, m: Minted) -> Self;
      pub fn parse(s: &str, expected: IdKind) -> Result<Self, IdError>; }
  pub struct ClientContext { addr: IpAddr, class: PathClass, via_proxy: bool } // pub(crate) constructor
  pub enum SecurityEvent { /* one variant per line, sorted: AuthnLoginFail { .. }, AuthzFail { .. }, ... */ }
  pub trait SecuritySink { fn record(&self, e: SecurityEvent) -> Result<(), AuditUnavailable>; }
  pub struct AuditUnavailable; // Describe: audit_unavailable, 503
  impl core::fmt::Display for PublicId { /* prefix + base32 */ }
  pub struct ProblemCode(&'static str);
  pub struct Problem { pub code: ProblemCode, pub status: Option<u16>, pub args: Vec<(&'static str, String)> }
  pub trait Describe { fn problem(&self) -> Problem; }
  pub fn text(code: ProblemCode) -> &'static str;
  ```

- **Tests.** Unit: base32 encoding of all-zero and all-one bytes against
  literal strings; parsing refuses the wrong prefix, wrong length, upper
  case if the format says lower, the excluded letters I, L, O and U, and a
  trailing newline. Every catalogue code has text, a unique code string
  and, where set, a status from an allowed list; every problem `type` is
  from the closed enumeration and no catalogue text holds a version, path
  or SQL (SEC-API-072). An identifier of the wrong kind parses to the
  same error as a malformed one, so the server can answer both with its
  not-found response (SEC-API-024). Properties: display then parse returns
  the same identifier for any bytes and kind; parsing never panics on
  arbitrary strings; no exported identifier type wraps an integer. A
  compile-fail test that `PublicId` has no constructor taking raw bytes,
  and one that a `ClientContext` cannot be built outside the core.
  Every security-event variant has a unique vocabulary name, and a
  compile-fail test shows no variant can hold a `Secret` (WP-047's type is
  not available to the core, so the test uses a stand-in marker type).
- **Risks.** The catalogue becomes a hotspot: every later package adds codes.
  It is a registry file under the merge protocol (one line per code, sorted),
  and so is the security-event catalogue.

### WP-007 Testkit foundation

- **Wave** 0 · **Size** M · **Depends on** nothing.
- **Owns** `crates/gunmetal-testkit/` (creates the crate; `crates/gunmetal-testkit/Cargo.toml`,
  `crates/gunmetal-testkit/src/lib.rs`, `crates/gunmetal-testkit/src/bytes.rs`, `crates/gunmetal-testkit/src/checksum.rs`, `crates/gunmetal-testkit/src/tempdir.rs`,
  `crates/gunmetal-testkit/src/clock.rs`).
- **Serves** CONTRIBUTING.md rules 3 and 4; the synthetic-media rule of this
  plan.
- **Security.** Boundaries TB6; threats TM-T20. Verifies no requirement of
  its own: it holds no security control, and the rules it relies on are
  proved by the packages that own them.
- **Scope.** The pieces every format builder needs: big- and little-endian
  writers, syncsafe integers, a box-and-chunk writer helper; independent
  implementations of CRC-8 and CRC-16 (FLAC), CRC-32 (IEEE, for PNG and
  others), the Ogg CRC-32 variant and Adler-32, written for the tests and
  sharing nothing with the core; a temporary directory that removes itself;
  a manual clock. Format builders are added later by the packages that
  parse each format.
- **Not in scope.** Any format builder. Production code of any kind.
- **Interface sketch.**

  ```rust
  pub struct Bytes(Vec<u8>); impl Bytes { pub fn u16_be(&mut self, v: u16) -> &mut Self; /* … */ pub fn syncsafe32(&mut self, v: u32) -> &mut Self; }
  pub fn crc8_flac(b: &[u8]) -> u8; pub fn crc16_flac(b: &[u8]) -> u16;
  pub fn crc32_ieee(b: &[u8]) -> u32; pub fn crc32_ogg(b: &[u8]) -> u32; pub fn adler32(b: &[u8]) -> u32;
  pub struct TempDir { /* path */ } impl TempDir { pub fn new(label: &str) -> std::io::Result<Self>; pub fn path(&self) -> &Path; }
  pub struct ManualClock { /* … */ } impl ManualClock { pub fn at(ms: i64) -> Self; pub fn advance(&self, ms: i64); }
  ```

- **Tests.** Every checksum against published check values (for example the
  CRC catalogue's check value for the ASCII string "123456789"), and
  against literal checksums from real headers written out by hand in the
  test. Writers against literal byte strings. `TempDir` is removed on drop,
  including after a panic in the test body, and two instances never
  collide.
- **Risks.** The testkit is under the gate like any crate (owner
  decision 26). The `ManualClock` must implement the core's `Clock` trait,
  which would make the testkit depend on the core; to keep the oracle rule,
  the testkit only provides the time source and each test crate implements
  the trait in a two-line adapter.

### WP-008 Fuzz harness registry, corpus replay and xtask

- **Wave** 0 · **Size** M · **Depends on** nothing.
- **Owns** `crates/gunmetal-fuzz/` (it exists, with the EBML harness in
  `src/lib.rs` and its replay test), `fuzz/` (outside the workspace, with
  `fuzz/seeds/`), `.github/workflows/fuzz.yml`, `crates/xtask/` (later
  packages add their own files under `crates/xtask/src/`; WP-115 adds
  `bench.rs`).
- **Serves** the "add to the repository now" item 10 of the security
  baseline.
- **Security.** Boundaries TB6, TB12; threats TM-T20, TM-T29, TM-T69.
  Verifies SEC-MED-027, SEC-MED-028, SEC-MED-029, SEC-MED-030, SEC-MED-031,
  SEC-TM-033, SEC-HIS-036, SEC-SUP-025, SEC-SUP-027.
- **Scope.** The layout the secure-coding guide already describes: one
  harness module per parser in `crates/gunmetal-fuzz/src/`, each a plain
  function from bytes that calls every entry point of one parser, asserts
  the invariants that must hold for any input and returns what the parser
  reported; seeds in `fuzz/seeds/<harness>/`; a stable `cargo test` per
  harness that replays every seed and every committed reproducer; a
  `cargo-fuzz` target per harness under `fuzz/fuzz_targets/`. Move the
  EBML harness out of `lib.rs` into `ebml.rs`, so both registries exist:
  `lib.rs` holds `pub mod` lines, and `registry.rs` holds the
  `harnesses!` list. Fuzz each changed harness for at least 10
  minutes on pull requests that touch `gunmetal-core`, and every harness
  for at least one CPU-hour nightly, with a 5 s per-input timeout and a
  memory cap (SEC-MED-029). An `xtask check-harnesses` fails when a public
  parse function in a parser module has no registered harness; an
  `xtask lockfile-age` check enforces the seven-day rule; an
  `xtask lint-exceptions` check fails when a `disallowed_methods`
  exception appears outside the written list (WP-001); an
  `xtask core-deps` check fails when `gunmetal-core`'s normal dependencies
  differ from the reviewed allowlist WP-001 keeps in `supply-chain/`
  (SEC-SUP-025).
- **Not in scope.** Harnesses for later parsers (each format package
  registers its own). Nightly fuzzing infrastructure beyond the CI job
  definition.
- **Interface sketch.**

  ```rust
  pub struct Harness { pub name: &'static str, pub run: fn(&[u8]) }
  pub fn all() -> &'static [Harness];
  ```

- **Tests.** The replay test runs every harness over its seeds and over a
  handful of literal inputs, asserting each seed's exact outcome. A canary
  harness with a planted bug, run in a scratch branch, shows that the PR
  fuzz job finds it (SEC-MED-029). The xtask's own tests use small fake source
  trees built in a temporary directory: one with a missing harness fails,
  one with every harness registered passes; one lock file with a crate
  published three days ago fails, one with eight days passes (publish
  times are injected, never fetched in tests).
- **Risks.** The rule for what counts as a "public parse entry point" must
  be simple enough for xtask to check without a Rust parser; proposal: any
  `pub fn` in `src/formats/**` or in a module on a parser-module list kept
  in the xtask (initially `lyrics.rs`, `m3u.rs`, `http/`, `path.rs`,
  `wire.rs`, `logframe.rs`, `webauthn/`, `import/`, `inflate.rs`,
  `deeplink.rs`, `provider/`). An opt-in constant
  alone would let a new parser outside `formats/` skip its harness
  silently, which is the failure the check exists to prevent.

### WP-122 Schema parts and digest (added in review)

- **Wave** 0 · **Size** S · **Depends on** nothing (needs `sha2`, owner
  decision 4).
- **Owns** `crates/gunmetal-core/src/schema.rs`,
  `crates/gunmetal-core/src/crypto.rs`.
- **Serves** ADM-077, ADM-058; the "schema by parts" rule in
  [The cache](#the-cache) and [The identity store](#the-identity-store).
- **Security.** Boundaries TB10, TB12; threats TM-T57, TM-T59, TM-T60.
  Verifies SEC-TM-050, SEC-PRV-001, SEC-STD-018 (the core's half of the
  crypto door).
- **Why it exists.** Both the cache (WP-042) and the identity store
  (WP-046) build their schema from named parts and compare a digest, and
  both are wave 1. Defining `SchemaPart` in `gunmetal-store` would have
  made WP-046 depend on WP-042 in the same wave. The part type and the
  digest are pure, so they belong in the core.
- **Scope.** `SchemaPart { name, sql, columns }`; validation (names
  unique, non-empty, ASCII); the SHA-256 digest over the parts in name
  order with an unambiguous framing (each name and body length-prefixed,
  so moving text from one part to the next changes the digest). Each
  part also declares a data class (Public, Library, Activity, Identity,
  Secret, as the privacy baseline names them) for every column it
  creates, and a pure check compares a list of live columns (which the
  stores read from SQLite at open) with the declared classes, failing on
  an unclassified or doubly classified column (SEC-TM-050, SEC-PRV-001).
  The class is what logging, export, backup encryption and admin views
  read, rather than guessing. The core's crypto module, `crypto.rs`: the
  one place in the core that uses a cryptographic crate, holding SHA-256
  only, for the schema digest and the content-identity windows (WP-077);
  every other algorithm lives in the secrets crate's crypto module
  (WP-047), and WP-001's lint keeps both confined (SEC-STD-018).
- **Interface sketch.** `pub struct SchemaPart { pub name: &'static str, pub sql: &'static str, pub columns: &'static [(&'static str, &'static str, DataClass)] }`;
  `pub fn digest(parts: &[SchemaPart]) -> Result<[u8; 32], SchemaError>`;
  `pub fn check_classes(parts: &[SchemaPart], live: &[(String, String)]) -> Result<(), ClassError>`;
  `pub fn sha256(bytes: &[u8]) -> [u8; 32]` (in `crypto.rs`).
- **Tests.** SHA-256 against the FIPS 180-4 example vectors. The digest of a fixed set of parts against a literal value
  computed by an independent SHA-256 run over literally written framed
  bytes; input order does not change the digest; a duplicate name is
  refused; moving one character between two parts changes the digest;
  changing a column's class changes the digest; an unclassified live
  column and a class declared for a column that does not exist are each
  reported with their names.

### WP-125 Security architecture records (added for the security baseline)

- **Wave** 0 · **Size** M · **Depends on** nothing.
- **Owns** `docs/adr/0007-identity-and-sessions.md`,
  `docs/adr/0008-https-and-naming.md`, `docs/adr/0009-cryptography.md`,
  `docs/adr/0010-backup-archives.md`.
- **Serves** the "add to the repository now" item 17 of the security
  baseline; README open decisions 1, 2 and 9 of the baseline as inputs.
- **Security.** Boundaries TB1, TB2, TB10, TB12; threats TM-T57, TM-T63,
  TM-T69. Verifies SEC-STD-006, SEC-STD-018, SEC-STD-019, SEC-HIS-019,
  SEC-NET-057.
- **Why it exists.** SEC-STD-006 forbids any server code from storing a
  user before an architecture record decides the identity model, and the
  baseline asks for records on HTTPS and naming and on the cryptography
  allow-list before server code. No package wrote them. SEC-HIS-019 also
  forbids extracting an archive from outside until a record allows it,
  which restore (WP-109) needs.
- **Scope.** Four records, accepted on 2026-10-03 through the owner's
  technical answers (AGENTS.md: agents never edit ADR 1 or ADR 2). Record
  7, identity and sessions: no passwords and no TOTP (SEC-IAM-025),
  passkeys, OIDC and browser pairing in R1, one session-lifetime table
  matching the baseline's parameters table, the principal kinds, the
  host-equivalent action list and where identity data lives. Record 8,
  HTTPS and naming: the per-server name service as the install-time
  default with own domain, tailnet and localhost as tested alternatives,
  the cleartext rule, the label scheme. Record 9, cryptography: the
  reviewed implementation allow-list, the rustls provider, the AEAD nonce
  strategy and the first cryptographic inventory (SEC-STD-018,
  SEC-STD-019, SEC-NET-057). Record 10, backup archives: the archive
  format inside the age envelope, with entry paths ignored, symlink
  entries refused and entry count and unpacked size limited, which is
  what SEC-HIS-019 asks before extraction is allowed.
- **Not in scope.** Code. The remuxer placement record (SEC-MED-081) and
  the client security record (SEC-STD-039), which are R2.
- **Tests.** Not code. The docs lint of WP-127 fails while SEC-IAM-025 and
  a password feature row are both live (SEC-STD-006), and the review
  checks each record against the requirements it names.
- **Risks and decisions.** The four records are accepted. Record 7's
  SEC-STD-006 "every file agrees" clause is still Not met: the stale
  narrative in `standards-coverage.md` and `rival-security-history.md`
  remains for the baseline's owner. Record 8 still has two unanswered
  items: the outbound exception before the claim (decision 4) and the
  release of the name service (decision 8). WP-046 may store a user
  against the accepted identity record; WP-109 may extract against the
  accepted archive format; WP-073 and WP-101 follow record 9's accepted
  provider choice (aws-lc-rs if it builds cleanly, otherwise ring).

### WP-126 Data-root handle and the filesystem crate (added for the security baseline)

- **Wave** 0 · **Size** L (was M; it gained the one SQLite connection
  opener) · **Depends on** nothing (needs `cap-std` and `rusqlite`, owner
  decision 4).
- **Owns** `crates/gunmetal-fs/` (creates the crate:
  `crates/gunmetal-fs/Cargo.toml`, `crates/gunmetal-fs/src/lib.rs`,
  `crates/gunmetal-fs/src/dataroot.rs`, `crates/gunmetal-fs/src/sqlite.rs`).
- **Serves** ADM-007 (the data directory), ADM-079; the data-directory
  layout in operations-and-incident-response.md, section 2.
- **Security.** Boundaries TB9, TB10; threats TM-T25, TM-T56, TM-T57,
  TM-T58. Verifies SEC-HIS-016, SEC-MED-033, SEC-OPS-012, SEC-TM-043,
  SEC-PRV-045, SEC-PRV-050, SEC-API-066, SEC-TM-039, SEC-HIS-038.
- **Why it exists.** As first written, the store, the identity store, the
  secrets crate and the server's data-directory module wrote their own
  files with path-based `std::fs`, as exceptions to the ban. The baseline
  bans path-based `std::fs` outside the filesystem module and requires
  every file access to go through a directory handle beneath the library
  or data root (SEC-MED-033, SEC-HIS-016, SEC-TM-043). Those modules are
  wave 1, so the handle they use has to exist in wave 0. WP-060 adds the
  library-root handles to the same crate in wave 2.
- **Scope.** `DataRoot`: open the data directory once as a directory
  handle; create the layout (`secrets/`, `durable/`, `cache/`,
  `snapshots/`, `backups/`, `derived/`, `tmp/`) with mode 0700; create
  files with mode 0600 and `O_EXCL`; check modes and owner at open and
  repair or refuse as SEC-OPS-012 says; atomic replace by write-then-rename
  inside the handle; append handles for log segments; read and write
  beneath the root only, refusing `..`, absolute paths and symlinks out;
  and the one sanctioned way to give SQLite a path, built from the root
  and a constant file name. The one SQLite connection opener, which every
  store uses (the cache, WP-042; the identity store and the audit
  address side store, WP-046 and WP-069; the derived-data store, WP-071):
  it opens a `DbFile` beneath the data root and sets the pragmas each
  store declares from a closed set, always including
  `secure_delete=ON`, `foreign_keys=ON` and `trusted_schema=OFF`, then
  reads every pragma back and refuses a connection where one did not
  take (SEC-PRV-050). The static-query type: SQL text only as
  `&'static str`, bound parameters as a closed `Param` enum, and sorting
  and filtering as enums each module declares; the opener's connections
  run statements only as a `Query`, so SQL built from request text is a
  compile error in every store (SEC-API-066, SEC-TM-039, SEC-HIS-038).
  The opener's second, read-only profile is for a database the server
  did not create (`trusted_schema` off, triggers and views refused,
  `cell_size_check` on, memory mapping off), which restore's jailed
  worker job calls (WP-109, SEC-STD-031), so that open is not a second
  door either. This door is in wave 0 because WP-042 and WP-046 both need it in
  wave 1; as first written, the store's `sql.rs` was the SQL door and the
  identity store, in the same wave, could not depend on it, so it would
  have needed a second one. Host facts (UID, file-system type) are probed and passed
  to pure checks, as the ground rules say.
- **Not in scope.** Library roots (WP-060). What lives in each file (each
  store's package).
- **Interface sketch.** `pub struct DataRoot; impl DataRoot { pub fn open(path: &Path, host: &HostFacts) -> Result<Self, DataRootError>; pub fn create_new(&self, rel: DataPath) -> Result<File, DataRootError>; pub fn replace(&self, rel: DataPath, bytes: &[u8]) -> Result<(), DataRootError>; pub fn append(&self, rel: DataPath) -> Result<File, DataRootError>; pub fn sqlite_path(&self, db: DbFile) -> PathBuf; }`
  where `DataPath` and `DbFile` are closed types built from constants,
  never from request or media data;
  `pub struct Query { text: &'static str, params: Vec<Param> }` with `pub const fn new(text: &'static str) -> Query` and `pub fn bind(self, p: Param) -> Query`;
  `pub fn open_db(root: &DataRoot, db: DbFile, p: &Pragmas) -> Result<Db, DbError>` and
  `impl Db { pub fn query(&self, q: &Query) -> Result<Rows, DbError>; pub fn execute(&self, q: &Query) -> Result<u64, DbError>; }`.
- **Tests (real filesystem in a temporary directory).** The layout is
  created with exact modes; a secret file with mode 0644 is repaired or
  refused as the configuration says; a symlink planted at `secrets/` to a
  directory outside is refused; `..` and absolute components cannot be
  expressed in a `DataPath` (compile-fail) and are refused by the
  runtime check underneath; two creates of the same file fail the second
  time. The opener (real SQLite): `secure_delete`, `foreign_keys` and
  `trusted_schema` read back as set on every connection of a pool of
  eight; a pragma set that leaves out `secure_delete` cannot be built
  (compile-fail); a statement can be run only as a `Query`, and a
  `String` or a non-static `&str` cannot become one (compile-fail). Permission
  tests assert at their start that they are not running as UID 0.

## Wave 1: core leaves and infrastructure crates

Every format package in this wave follows the same pattern: a sans-I/O
parser in `crates/gunmetal-core/src/formats/`, a builder for that format in
`crates/gunmetal-testkit/src/`, a fuzz harness with its seeds and replay
test in `crates/gunmetal-fuzz` and `fuzz/` (the files are named after the
parser and owned by the format package; only the registry lines are
shared), a structure-aware generator for SEC-MED-031, and a property test
that the parser returns for arbitrary bytes. The `Security` field of each
format package lists the parse-contract requirements its own tests prove
for its own parser. Container parsers return **raw** tag
blocks and technical facts; turning tags into the music model is wave 2 and
3. To keep the entries short, the common tests are not repeated: each
format package must also show that every truncation of a valid file yields
the exact `Truncated` offset, that the budget and depth limits fire at
their boundary, and that a parse of a valid file built by the testkit
returns the whole expected value.

### WP-009 Format detection

- **Wave** 1 · **Size** S · **Depends on** WP-004.
- **Owns** `crates/gunmetal-core/src/formats/mod.rs`,
  `crates/gunmetal-core/src/formats/detect.rs`.
- **Serves** MUS-032.
- **Security.** Boundaries TB6, TB9; threats TM-T20, TM-T28. Verifies
  SEC-MED-001, SEC-MED-005, SEC-MED-006, SEC-MED-007, SEC-MED-008,
  SEC-TM-032, SEC-HIS-036, SEC-MED-011, SEC-HIS-017.
- **Scope.** Decide a file's format from its content against a closed
  allowlist (FLAC, MP3, MP4 audio, Ogg, WAV, AIFF, and the sidecars JPEG,
  PNG, WebP, GIF, LRC, M3U), skipping a leading ID3v2 tag by its declared
  size. The extension only chooses which signatures to try first.
- **Not in scope.** Parsing any format.
- **Interface sketch.** `pub enum Format { Flac, Mpeg, Mp4, Ogg, Wav, Aiff, Jpeg, Png, Webp, Gif, Lrc, M3u }`
  and `pub fn detect(head: &[u8], ext_hint: Option<&str>) -> Detection`
  where `Detection` is a format, `NeedMore(ReadRequest)` (after an ID3v2
  header) or `Unknown`.
- **Tests.** A FLAC named `.mp3`, an MP3 named `.flac`, a text file named
  `.flac`, a PNG named `cover.jpg`, an ID3v2 tag followed by FLAC (it
  happens), an ID3v2 tag whose declared size runs past the file, an empty
  file, and a file of one byte. Property: for any file a testkit builder
  writes, `detect` returns exactly the format that builder wrote, whatever
  extension hint is given (a weaker "never `Unknown`" would pass for a
  detector that always answered FLAC); never panics.
- **Note.** `formats/mod.rs` is a registry file under the merge rules, so
  the other wave 1 format packages do not depend on this one.

### WP-010 ID3v2 container and frames

- **Wave** 1 · **Size** L · **Depends on** WP-004, WP-005, WP-007.
- **Owns** `crates/gunmetal-core/src/formats/id3v2/`,
  `crates/gunmetal-testkit/src/id3v2.rs`.
- **Serves** MUS-032, MUS-034, MUS-039, LIB-059, LIB-134, MUS-154;
  API-CAT-01 to API-CAT-06, API-CAT-11.
- **Security.** Boundaries TB6, TB9; threats TM-T20, TM-T29. Verifies
  SEC-MED-001, SEC-MED-005, SEC-MED-006, SEC-MED-007, SEC-MED-008,
  SEC-TM-032, SEC-HIS-036, SEC-MED-031.
- **Scope.** ID3v2.2, 2.3 and 2.4: header, extended header, footer, the
  unsynchronisation scheme at tag and frame level, frame iteration with the
  tag-field limit, and typed decoding of text frames (with the 2.4 null
  separator for multiple values and each encoding byte), `TXXX`, `COMM`,
  `USLT`, `SYLT`, `APIC` (as an offset and length plus picture type and
  MIME, never decoded), `UFID` (MusicBrainz recording ID), `TIPL` and
  `TMCL`, `POPM` kept raw, and every other frame kept raw for the file
  inspector. Compressed and encrypted frames are skipped and recorded.
  `CHAP` and `CTOC` are walked within the depth limit of 4 and kept raw.
- **Not in scope.** What a frame means for the music model (WP-049).
  Decompressing frames (owner decision 20).
- **Interface sketch.**
  `pub fn parse(tag: &[u8], limits: &Limits, budget: &mut Budget) -> Result<Id3v2Tag, Id3v2Error>`;
  `pub struct Id3v2Tag { pub version: (u8, u8), pub frames: Vec<Frame>, pub problems: Vec<FrameProblem> }`;
  `pub enum FrameBody { Text(Vec<Text>), UserText { desc: Text, values: Vec<Text> }, Comment { lang: [u8; 3], desc: Text, text: Text }, Lyrics(..), SyncedLyrics(..), Picture(PictureRef), Ufid { owner: Text, id: Vec<u8> }, Raw(Range<u32>) }`.
- **Tests.** One test per version and per text encoding; a v2.3 tag with
  unsynchronisation and a v2.4 frame with per-frame unsynchronisation;
  multiple values separated by nulls with and without a trailing null; a
  `TXXX` with an empty description; a frame whose size is a non-syncsafe
  integer written into a v2.4 tag (a common writer bug, handled by the
  documented fallback); padding of zeros; a frame ID with lower-case or
  non-alphanumeric characters (stops iteration with a reason); a frame
  larger than the tag; 4,097 frames (limit); an `APIC` of 33 MiB declared
  (limit); a nested `CHAP` five levels deep (depth). The testkit builder is
  checked against a literal 2.4 tag written out byte by byte from the
  specification.
- **Risks.** ID3 in the wild is messy; the fallbacks for common writer bugs
  must be documented in the code and each needs a test. Real-world quirks
  the specification does not cover are guesses until a corpus says
  otherwise (unverified).

### WP-011 ID3v1 and APEv2

- **Wave** 1 · **Size** M · **Depends on** WP-004, WP-005, WP-007.
- **Owns** `crates/gunmetal-core/src/formats/id3v1.rs`, `crates/gunmetal-core/src/formats/ape.rs`,
  `crates/gunmetal-testkit/src/id3v1.rs`, `crates/gunmetal-testkit/src/ape.rs`.
- **Serves** MUS-034, LIB-028 (finding the regions the identity hash skips),
  MUS-084 (ReplayGain in APE tags).
- **Security.** Boundaries TB6, TB9; threats TM-T20. Verifies SEC-MED-001,
  SEC-MED-005, SEC-MED-006, SEC-MED-007, SEC-MED-008, SEC-TM-032,
  SEC-HIS-036.
- **Scope.** The 128-byte ID3v1 and ID3v1.1 tag at the end of a file
  (track number in the comment field, genre byte), and APEv2 tags with
  header and footer, items with UTF-8 values, binary items kept as ranges.
  Report each tag's byte range so the identity window can skip it.
- **Not in scope.** Mapping to the music model (WP-049, WP-051).
- **Interface sketch.** `pub fn find_v1(tail: &[u8]) -> Option<Id3v1Tag>`;
  `pub fn parse_ape(tail: &[u8], file_len: u64, limits: &Limits, budget: &mut Budget) -> Result<Option<ApeTag>, ApeError>`
  with `ApeTag { pub range: Range<u64>, pub items: Vec<ApeItem> }`.
- **Tests.** v1 with and without the v1.1 track byte; fields padded with
  spaces versus nulls; an APE tag before an ID3v1 tag; an APE footer whose
  declared size exceeds the file; an item count that cannot fit; an item
  key with forbidden characters; an APE header-only variant.

### WP-012 FLAC metadata blocks

- **Wave** 1 · **Size** M · **Depends on** WP-004, WP-005, WP-007.
- **Owns** `crates/gunmetal-core/src/formats/flac/metadata.rs`,
  `crates/gunmetal-testkit/src/flac.rs`. (`formats/flac/mod.rs` is a
  registry file shared with WP-013 in the same wave.)
- **Serves** MUS-032, MUS-021, LIB-028 (STREAMINFO MD5), MUS-039,
  MUS-154; API-CAT-04, API-CAT-05.
- **Security.** Boundaries TB6, TB9; threats TM-T20. Verifies SEC-MED-001,
  SEC-MED-005, SEC-MED-006, SEC-MED-007, SEC-MED-008, SEC-TM-032,
  SEC-HIS-036, SEC-MED-031.
- **Scope.** The `fLaC` marker and the metadata block chain: STREAMINFO
  (block sizes, sample rate, channels, bits, total samples, MD5), SEEKTABLE
  (with placeholder points), VORBIS_COMMENT returned as a raw range,
  PICTURE (type, MIME, description, dimensions, data range), PADDING,
  APPLICATION and CUESHEET kept raw. Report where audio frames start.
- **Not in scope.** Frames (WP-013). Comment contents (WP-014). CUE sheets
  as virtual tracks (MUS-041, R2).
- **Interface sketch.** `pub fn parse_metadata(..) -> Step<Result<FlacMetadata, FlacError>>` as a
  sans-I/O parser; `FlacMetadata { stream_info: StreamInfo, seek_table: Vec<SeekPoint>, comment: Option<Range<u64>>, pictures: Vec<PictureRef>, audio_start: u64 }`.
- **Tests.** A minimal valid file; a STREAMINFO that is not the first block;
  two STREAMINFO blocks; a sample rate of zero; an MD5 of all zeros
  (meaning "not set", which identity must not use); total samples of zero
  (meaning unknown); a last-block flag never set; a block length that runs
  past the file; a seek table whose length is not a multiple of 18; 65,537
  blocks (limit).

### WP-013 FLAC frame index

- **Wave** 1 · **Size** M · **Depends on** WP-004, WP-007.
- **Owns** `crates/gunmetal-core/src/formats/flac/frames.rs`,
  `crates/gunmetal-testkit/src/flac_frames.rs`.
- **Serves** MUS-071, MUS-230, MUS-067; API-CAT-05.
- **Security.** Boundaries TB6, TB9; threats TM-T20. Verifies SEC-MED-001,
  SEC-MED-005, SEC-MED-006, SEC-MED-007, SEC-MED-008, SEC-TM-032,
  SEC-HIS-036.
- **Scope.** Find and validate frame headers (sync code, blocking strategy,
  block size, sample rate and bit-depth codes, the UTF-8-style coded
  number, CRC-8), and build a frame index: the byte offset and first sample
  of each frame, thinned to the index limit. The packager (WP-056) copies
  frames using this index; seeking uses it to pick a byte range.
- **Not in scope.** Decoding samples. Checking the CRC-16 of whole frames
  (proposal: only when the scan's byte budget allows; recorded otherwise).
- **Interface sketch.** `pub struct FrameContext { pub stream_sample_rate: Option<SampleRate>, pub stream_bits: Option<u8> }`;
  `pub fn index_frames(ctx: FrameContext, audio: Window<'_>, limits: &Limits, budget: &mut Budget) -> Step<Result<FrameIndex, FlacFrameError>>`.
- **Tests.** The testkit writes frames with constant subframes (a valid
  frame with no real audio is a few bytes). Fixed and variable blocking;
  every block-size and sample-rate code, including "from STREAMINFO" and
  the end-of-header values; a false sync code inside frame data that fails
  its CRC-8 and is skipped; a coded number at each UTF-8 width; a truncated
  final frame; ten hours of frames thinned to the index limit. Property:
  the index's sample positions strictly increase.

### WP-014 Vorbis comments and embedded pictures

- **Wave** 1 · **Size** M · **Depends on** WP-004, WP-005, WP-007.
- **Owns** `crates/gunmetal-core/src/formats/vorbis_comment.rs`,
  `crates/gunmetal-testkit/src/vorbis_comment.rs`.
- **Serves** MUS-034, MUS-039, MUS-154; LIB-059.
- **Security.** Boundaries TB6, TB9; threats TM-T20, TM-T29. Verifies
  SEC-MED-001, SEC-MED-005, SEC-MED-006, SEC-MED-007, SEC-MED-008,
  SEC-TM-032, SEC-HIS-036.
- **Scope.** The comment block shared by FLAC, Ogg Vorbis and Opus: vendor
  string, comment count checked against the bytes, `KEY=value` pairs with
  case-insensitive keys, multiple values per key kept in order. Decode
  `METADATA_BLOCK_PICTURE` from base64 into a picture reference within the
  4/3 size allowance, using the base64 codec from WP-005.
- **Not in scope.** Meaning of keys (WP-050).
- **Interface sketch.** `pub fn parse(block: &[u8], limits: &Limits, budget: &mut Budget) -> Result<Comments, CommentError>`.
- **Tests.** A count of 2^32−1 in a 20-byte block; an entry without `=`; an
  empty key; a key with a byte outside 0x20 to 0x7D; values with newlines;
  invalid UTF-8; base64 with padding, without padding, with whitespace, and
  with an invalid character inside a picture block.

### WP-015 Ogg pages and packets

- **Wave** 1 · **Size** M · **Depends on** WP-004, WP-007.
- **Owns** `crates/gunmetal-core/src/formats/ogg.rs`,
  `crates/gunmetal-testkit/src/ogg.rs`.
- **Serves** MUS-032, MUS-069, MUS-071, MUS-230.
- **Security.** Boundaries TB6, TB9; threats TM-T20. Verifies SEC-MED-001,
  SEC-MED-005, SEC-MED-006, SEC-MED-007, SEC-MED-008, SEC-TM-032,
  SEC-HIS-036, SEC-MED-031.
- **Scope.** Page header parsing with the Ogg CRC, the segment table,
  packet reassembly across pages with the continuation flag, one logical
  stream (others are skipped and recorded), granule positions, and reading
  the last page from the end of the file for the duration. A page index for
  seeking and packaging.
- **Not in scope.** Codec headers (WP-016).
- **Interface sketch.** `pub fn pages(input: &[u8]) -> Pages<'_>` (an iterator
  like `ebml::elements`); `pub fn packets(..) -> Packets<'_>`;
  `pub fn last_granule(tail: &[u8]) -> Option<(u32, u64)>` (serial, granule).
- **Tests.** A packet split over three pages; a packet of exactly 255 bytes
  (needs a zero lacing value); a page with a bad CRC; capture pattern found
  inside data; a granule of −1 (no packet ends on the page); two
  interleaved logical streams; a final page with no end-of-stream flag.
  Property: packets reassembled from pages written by the testkit equal
  the packets written.

### WP-016 Opus and Vorbis stream headers

- **Wave** 1 · **Size** S · **Depends on** WP-004, WP-005, WP-007.
- **Owns** `crates/gunmetal-core/src/formats/opus.rs`,
  `crates/gunmetal-core/src/formats/vorbis.rs`, `crates/gunmetal-testkit/src/opus.rs`.
- **Serves** MUS-069 (pre-skip), MUS-085 (output gain), MUS-021.
- **Security.** Boundaries TB6, TB9; threats TM-T20. Verifies SEC-MED-001,
  SEC-MED-005, SEC-MED-006, SEC-MED-007, SEC-MED-008, SEC-TM-032,
  SEC-HIS-036.
- **Scope.** `OpusHead` (version, channels, pre-skip, input sample rate,
  output gain in Q7.8 dB, mapping family and table) and the `OpusTags`
  framing; the Vorbis identification header (sample rate, channels,
  bitrates) and comment-header framing. Returns the comment bytes for
  WP-014.
- **Not in scope.** Audio packets.
- **Interface sketch.** `pub fn opus_head(packet: &[u8]) -> Result<OpusHead, OpusError>`;
  `pub fn vorbis_ident(packet: &[u8]) -> Result<VorbisIdent, VorbisError>`.
- **Tests.** Version 0 and 1 accepted, 16 refused; mapping family 1 with a
  table; family 255; channel count 0; negative output gain; a pre-skip
  larger than a typical file; Vorbis framing bit unset.

### WP-017 MP4 boxes, audio sample entries and item lists

- **Wave** 1 · **Size** L · **Depends on** WP-004, WP-005, WP-007.
- **Owns** `crates/gunmetal-core/src/formats/mp4/boxes.rs`,
  `crates/gunmetal-core/src/formats/mp4/audio.rs`, `crates/gunmetal-core/src/formats/mp4/ilst.rs`,
  `crates/gunmetal-core/src/formats/mp4/probe.rs`, `crates/gunmetal-testkit/src/mp4.rs`.
  (`formats/mp4/mod.rs` is a registry file shared with WP-018 in the same
  wave; the `probe` entry point lives in `probe.rs`.)
- **Serves** MUS-032 (AAC and ALAC in MP4), MUS-034, MUS-039, MUS-069
  (`iTunSMPB`), MUS-084; API-CAT-01 to API-CAT-05.
- **Security.** Boundaries TB6, TB9; threats TM-T20. Verifies SEC-MED-001,
  SEC-MED-005, SEC-MED-006, SEC-MED-007, SEC-MED-008, SEC-TM-032,
  SEC-HIS-036, SEC-MED-031.
- **Scope.** A box walker that handles size 0 (to the end of the parent),
  size 1 (64-bit size), sizes below the header length (error), full-box
  version and flags, and the depth and child limits; `ftyp`; `moov`,
  `trak`, `mdia`, `mdhd` (timescale guarded as non-zero, duration), `hdlr`,
  `stsd` with `mp4a` and its `esds` AudioSpecificConfig, `alac`, `fLaC`
  with `dfLa`, and `Opus` with `dOps`; `udta`, `meta` and `ilst` items,
  including freeform `----` items with mean and name (where MusicBrainz IDs
  and `iTunSMPB` live) and `covr` as a picture reference. The file may put
  `moov` after `mdat`; the parser asks for it by offset.
- **Not in scope.** Sample tables (WP-018). Video tracks (R2). Fragmented
  input files.
- **Interface sketch.** `pub fn probe(..) -> Step<Result<Mp4Audio, Mp4Error>>`;
  `Mp4Audio { pub track: AudioTrack, pub items: Vec<IlstItem>, pub sample_tables: SampleTableRanges }`.
- **Tests.** `moov` before and after `mdat`; a size-0 box at the end of the
  file; a 64-bit box size; a box of size 7; a `meta` box written as a full
  box and as a plain box (both exist in the wild, unverified how often);
  a timescale of zero; an `esds` with each descriptor length encoding;
  freeform items with and without `mean`; two audio tracks (first is used,
  second recorded); 33 levels of nesting.

### WP-018 MP4 sample tables

- **Wave** 1 · **Size** M · **Depends on** WP-004, WP-007.
- **Owns** `crates/gunmetal-core/src/formats/mp4/sample_table.rs`,
  `crates/gunmetal-testkit/src/mp4_samples.rs`.
- **Serves** MUS-071, MUS-230.
- **Security.** Boundaries TB6, TB9; threats TM-T20. Verifies SEC-MED-001,
  SEC-MED-005, SEC-MED-006, SEC-MED-007, SEC-MED-008, SEC-TM-032,
  SEC-HIS-036.
- **Scope.** Parse the bodies of `stts`, `stsc`, `stsz` and `stz2`, `stco`
  and `co64` (each count checked with `checked_mul` against the box
  length), and join them into a seek index of (time, byte offset) points
  thinned to the index limit.
- **Not in scope.** Finding the boxes (WP-017 returns their ranges).
- **Interface sketch.** `pub fn seek_index(tables: SampleTableBodies<'_>, timescale: NonZeroU32, limits: &Limits, budget: &mut Budget) -> Result<SeekIndex, SampleTableError>`.
- **Tests.** The Stagefright pattern: an entry count whose product with the
  entry size overflows; `stsc` runs that reference chunk 0; a chunk offset
  past the end of the file; `stsz` with a constant sample size; tables
  that disagree on the sample count. Property: the index is monotonic in
  both time and offset.

### WP-019 MPEG audio frames and encoder headers

- **Wave** 1 · **Size** M · **Depends on** WP-004, WP-005, WP-007.
- **Owns** `crates/gunmetal-core/src/formats/mpa.rs`,
  `crates/gunmetal-testkit/src/mpa.rs`.
- **Serves** MUS-032, MUS-069 (LAME delay and padding), MUS-071, MUS-084
  (LAME ReplayGain), MUS-230.
- **Security.** Boundaries TB6, TB9; threats TM-T20. Verifies SEC-MED-001,
  SEC-MED-005, SEC-MED-006, SEC-MED-007, SEC-MED-008, SEC-TM-032,
  SEC-HIS-036.
- **Scope.** MPEG-1, 2 and 2.5 Layer III frame headers (bitrate and sample
  rate tables, padding, free format refused), the Xing or Info header with
  frame count, byte count and TOC, the LAME extension (encoder delay and
  padding, ReplayGain fields, CRC), the VBRI header, and a frame scan with
  a sync search bounded by the budget to confirm a stream and build a
  seek index when no TOC exists.
- **Not in scope.** Layers I and II beyond detecting and recording them.
- **Interface sketch.** `pub fn frame_header(b: [u8; 4]) -> Result<FrameHeader, MpaError>`;
  `pub fn stream_info(..) -> Step<Result<MpegStream, MpaError>>` with
  `MpegStream { header: FrameHeader, xing: Option<Xing>, lame: Option<LameTag>, frames: Option<u32>, seek: SeekIndex }`.
- **Tests.** Every bitrate index including the forbidden 15; sample-rate
  index 3; a Xing header in a mono MPEG-2 frame (different offset); an
  Info header with zero frames; a LAME tag with delay 576 and padding 1,152;
  a false sync inside an ID3 tag; a stream of free-format frames; an MP3
  preceded by 8 KB of zeros.

### WP-020 RIFF WAV and AIFF

- **Wave** 1 · **Size** M · **Depends on** WP-004, WP-005, WP-007.
- **Owns** `crates/gunmetal-core/src/formats/riff.rs`,
  `crates/gunmetal-core/src/formats/aiff.rs`, `crates/gunmetal-testkit/src/riff.rs`.
- **Serves** MUS-032 (PCM WAV, AIFF where the browser allows).
- **Security.** Boundaries TB6, TB9; threats TM-T20. Verifies SEC-MED-001,
  SEC-MED-005, SEC-MED-006, SEC-MED-007, SEC-MED-008, SEC-TM-032,
  SEC-HIS-036.
- **Scope.** RIFF and RF64 chunk walking with odd-size padding, `fmt `
  (PCM and extensible), `data` range, `LIST`/`INFO` and an `id3 ` chunk as
  raw ranges; AIFF and AIFF-C `COMM` (with the 80-bit extended sample rate,
  converted with checked arithmetic), `SSND`, `ID3 ` chunk.
- **Not in scope.** Compressed WAV formats beyond recording their codec.
- **Interface sketch.** `pub fn wav(..) -> Step<Result<WavFile, RiffError>>`;
  `pub fn aiff(..) -> Step<Result<AiffFile, AiffError>>`.
- **Tests.** An odd-length chunk with its pad byte; a missing pad byte at
  the end of a file; a `data` size of 0xFFFFFFFF (written by streaming
  tools); RF64 with a `ds64` chunk; extended sample rates of 44,100 and
  zero and infinity; `fmt ` after `data`.

### WP-021 Lyrics parser

- **Wave** 1 · **Size** M · **Depends on** WP-004, WP-005.
- **Owns** `crates/gunmetal-core/src/lyrics.rs`.
- **Serves** MUS-154, MUS-155; API-CAT-06. Word stamps are parsed
  because R1 libraries already contain Enhanced LRC files and the parser
  must handle them as untrusted input either way; showing lyrics word by
  word (MUS-156) is R1.1 and needs only the client.
- **Security.** Boundaries TB6, TB9; threats TM-T07, TM-T20. Verifies
  SEC-MED-001, SEC-MED-005, SEC-MED-006, SEC-MED-007, SEC-MED-008,
  SEC-TM-032, SEC-HIS-036, SEC-MED-049, SEC-API-090.
- **Scope.** One typed lyrics model (plain, line-timed, word-timed, with
  the source) and parsers for LRC and Enhanced LRC (`[mm:ss.xx]`,
  `<mm:ss.xx>` word stamps, `[offset:]` clamped to ±1 hour, several stamps
  on one line, metadata tags kept), plus constructors from ID3 `USLT` and
  `SYLT` bodies and Vorbis `LYRICS`. A lookup from a playback position to
  the current line and word, used by the device.
- **Not in scope.** TTML (MUS-157, R2). Fetching lyrics (R2 plugin).
- **Interface sketch.** `pub enum Lyrics { Plain(Vec<String>), Lines(Vec<Line>), Words(Vec<WordLine>) }`;
  `pub fn parse_lrc(text: &str, limits: &Limits) -> Result<Parsed, LrcError>`;
  `pub fn position(l: &Lyrics, ms: u32) -> Option<Cursor>`.
- **Limits.** Two baseline rows set caps for lyrics: SEC-MED-049 (1 MiB,
  20,000 lines, 4 KiB per line) and SEC-API-090 (256 KiB, 10,000 lines).
  The parser applies the stricter of each, 256 KiB, 10,000 lines and
  4 KiB per line, which satisfies both; the first draft of this package
  tested at 20,001 lines.
- **Tests.** Lines out of order (sorted); identical timestamps; a stamp of
  99:59.99 and one past 24 hours; hundredths and thousandths; an offset of
  plus and minus; a line of 4,097 bytes; 10,001 lines; 256 KiB plus one
  byte; blank lines between
  timed lines; a file mixing timed and untimed lines. `position` at each
  line's exact timestamp, one millisecond before it, and before the first
  line, each with its literal expected cursor. Property: `position` is
  monotonic as the time increases (on its own this would pass for a
  function that always returned `None`, so the examples carry the weight).

### WP-023 HTTP header parsers

- **Wave** 1 · **Size** M · **Depends on** WP-004, WP-005.
- **Owns** `crates/gunmetal-core/src/http/` (`crates/gunmetal-core/src/http/mod.rs`, `crates/gunmetal-core/src/http/range.rs`,
  `crates/gunmetal-core/src/http/forwarded.rs`).
- **Serves** ACC-097; API-STR-03, API-SET-01. (ACC-134, the path prefix,
  is R1.2, WP-151; it reuses these parsers unchanged.)
- **Security.** Boundaries TB1, TB2; threats TM-T05, TM-T09. Verifies
  SEC-MED-001, SEC-MED-005, SEC-MED-006, SEC-MED-007, SEC-MED-008,
  SEC-TM-032, SEC-HIS-036, SEC-MED-060, SEC-NET-050, SEC-NET-016,
  SEC-NET-018, SEC-HIS-003, SEC-API-031.
- **Scope.** The `Range` header (one range only; a header naming two or
  more ranges is answered 416, as SEC-API-031 and SEC-NET-050 require; a
  malformed header means the full representation, as SEC-MED-060 says;
  unsatisfiable means 416), `Forwarded` and `X-Forwarded-For` chains
  walked from the right against a trusted-proxy list, taking the first
  address that is not a trusted proxy and refusing a malformed or
  over-long chain (SEC-NET-018, SEC-HIS-003), using the address types and
  classifier from WP-005. The headers are read only when the immediate
  peer is a configured trusted proxy; otherwise they are ignored
  (SEC-NET-016). The one path-class function, which turns the peer, the
  chain, the trusted-proxy list and the server's own interface prefixes
  into a client address and a path class, lives here so that exposure
  detection, the rate limiters and the audit log all take the address
  from one place (SEC-OPS-037).
- **Not in scope.** Applying any of these to requests (WP-044, WP-073,
  WP-082).
- **Interface sketch.** `pub fn range(header: &[u8], len: u64) -> RangeOutcome`;
  `pub fn client_addr(peer: IpAddr, headers: ForwardedHeaders<'_>, trusted: &[IpNet]) -> IpAddr`.
- **Tests.** `bytes=0-`, `bytes=-500`, `bytes=500-100`, `bytes=0-0`,
  `bytes=0-1,5-9` (416; the first draft served the full representation,
  which SEC-API-031 forbids), the overlapping-range pattern of
  CVE-2011-3192 (416), a range starting at the length (416), numbers with
  30 digits, spaces, upper-case unit. Forwarded chains with an untrusted
  hop in the middle, spoofed left-most entries, IPv6 in brackets with
  ports, obfuscated identifiers, and a chain over the length cap (refused).
  Properties: a satisfiable range is always inside `0..len`; for any
  attacker-chosen prefix followed by any sequence of trusted proxies, the
  client address is the last untrusted hop; with an empty trusted list,
  the address is always the peer.

### WP-024 Path rules

- **Wave** 1 · **Size** S (was M; the exclusion pattern language moved to
  WP-140 in R1.1, because exclusion rules, LIB-006, are R1.1) · **Depends
  on** WP-005.
- **Owns** `crates/gunmetal-core/src/path.rs`.
- **Serves** ADM-025 (the root refusal behind the live checks), LIB-007,
  LIB-204 (containment for links); API-CAT-10, API-LIB-02. (LIB-006 is
  R1.1, WP-140; folder view, LIB-008, is R1.3 and reuses the display
  names.)
- **Security.** Boundaries TB9; threats TM-T22, TM-T25, TM-T56. Verifies
  SEC-MED-034, SEC-MED-037, SEC-MED-039, SEC-MED-040, SEC-TM-043,
  SEC-HIS-015.
- **Scope.** Paths as raw byte components: relative-path normalisation,
  containment ("is this chain beneath that root"), the root refusal rules
  (filesystem root, system directories, the data directory, overlaps), and
  display names decoded lossily with controls escaped.
- **Not in scope.** Touching the filesystem (WP-060). The exclusion
  pattern language (WP-140, R1.1).
- **Interface sketch.** `pub struct RelPath(Vec<Vec<u8>>)`;
  `pub fn normalise(components: &[&[u8]]) -> Result<RelPath, PathError>`;
  `pub fn refuse_root(candidate: &RawPath, data_dirs: &[RawPath]) -> Option<RootRefusal>`.
- **Tests.** Non-UTF-8 names; names with newlines and escape characters;
  `.` and `..` at every position; a root that equals, contains and lies
  inside the data directory; `/`, `/etc`, `/proc`. Property: a normalised
  path never contains `..` and containment agrees with an independent
  prefix-on-components oracle.

### WP-025 Queue document and verbs

- **Wave** 1 · **Size** L · **Depends on** WP-005, WP-006.
- **Owns** `crates/gunmetal-core/src/queue/`.
- **Serves** MUS-116 to MUS-119, MUS-122, MUS-123, MUS-077, LAT-009;
  API-QUE-01 to API-QUE-03. (Reordering while shuffled, MUS-120, is R1.1,
  WP-142; the suggestions that fill the Continue with lane, MUS-129, and
  the start-radio verb are R1.3, WP-058; handing playback to another
  device, CLI-103, is R1.1 and uses the active-device field below
  unchanged.)
- **Security.** Boundaries TB4; threats TM-T09, TM-T17. Verifies no
  requirement of its own: it holds no security control, and the rules it
  relies on are proved by the packages that own them.
- **Scope.** The versioned queue document: three lanes (Up next, From with
  its source, Continue with), named listening contexts, the insertion
  cursor, repeat and stop-after modes, the current item and position, and
  the active device (so two of a person's devices never both play one
  queue; owner decision 14). In R1 the Continue with lane stays empty,
  because the suggestions that fill it (MUS-129) are R1.3.
  The verbs from player.md (play, play next, add, play last, move, remove,
  clear, clear Up next) as operations
  applied to a version, including multi-item operations as one. Stale
  operations are rejected with the current version so the client can
  rebase, and the rebase function itself, so client and server agree.
  "Picks survive a new Play" is implemented as the player.md proposal
  (owner decision 28).
- **Not in scope.** Shuffle orders (WP-026); the start-radio verb and
  radio picks (WP-058, R1.3); storage (WP-085).
- **Interface sketch.**

  ```rust
  pub struct Queue { pub version: u64, pub context: ContextId, pub up_next: Vec<Entry>, pub from: Lane,
      pub continue_with: ContinueLane, pub cursor: usize, pub current: Option<EntryId>, pub repeat: Repeat,
      pub stop_after: StopAfter, pub active_device: Option<DeviceRef> }
  pub enum Op { Play { items: Vec<ItemRef>, source: Source, start: usize }, PlayNext(Vec<ItemRef>), Add(Vec<ItemRef>),
      PlayLast(Vec<ItemRef>), Move { entry: EntryId, to: Position }, Remove(Vec<EntryId>), ClearUpNext, Clear, /* … */ }
  pub fn apply(q: &Queue, based_on: u64, op: &Op) -> Result<Queue, QueueReject>;
  pub fn rebase(local: &[PendingOp], server: &Queue) -> Rebased;
  ```

- **Tests.** Each verb with its exact resulting queue. Three play-next picks
  play in the order chosen; the cursor resets when the current item
  changes. Moving an item between lanes keeps its own source. Removing the
  current item. Repeat one, repeat all and stop after this item at the end
  of the From lane. Multi-item drops. A stale version is rejected with the
  current version. Properties: no operation loses or duplicates an entry it
  did not mean to remove; version strictly increases; `rebase` of an empty
  pending list is the server queue; applying the same operation to the same
  version gives the same queue (determinism, needed because client and
  server both run it).
- **Risks.** Contradiction 2 in docs/ui/README.md (undo in R1 or not) does
  not change this package; undo is R2 (MUS-121).

### WP-026 Shuffle modes

- **Wave** 1 · **Size** S (was M; shuffle by album and reshuffle the rest
  moved to WP-142 in R1.1) · **Depends on** WP-005.
- **Owns** `crates/gunmetal-core/src/shuffle.rs`.
- **Serves** MUS-126 (random and spread out). (MUS-127, MUS-128 and
  MUS-120 are R1.1, WP-142.)
- **Security.** Boundaries TB4; threats TM-T09. Verifies no requirement of
  its own: it holds no security control, and the rules it relies on are
  proved by the packages that own them.
- **Scope.** A small seeded generator owned by the core (so the order is
  the same on every device and every platform), and the two R1 modes over
  the From lane: random, and spread out (the same artist or album never
  bunches, recent plays later). Turning shuffle off restores source order
  from the current item.
- **Not in scope.** Lane mechanics (WP-025). Shuffle by album, reshuffle
  the rest and reordering while shuffled (WP-142, R1.1).
- **Interface sketch.** `pub fn order(items: &[ShuffleItem], mode: Mode, seed: u64, recent: &RecentPlays) -> Vec<usize>`.
- **Tests.** The generator is a published algorithm with published
  reference outputs (for example SplitMix64; which algorithms publish
  vectors is unverified), and is tested against those outputs. "Random"
  order is checked against a Fisher–Yates shuffle written independently in
  the test and fed the same generator outputs. Literal orders written down
  from a run of the code under test are not used, because that derives the
  expected value from the code (CONTRIBUTING.md rule 4). Properties: the output is a
  permutation; spread out never puts the same artist adjacent when another
  artist is available at that point; the same seed and input give the
  same order.
- **Risks.** "Spread out" as default is a proposal from player.md.

### WP-028 Gain decision

- **Wave** 1 · **Size** S · **Depends on** WP-005.
- **Owns** `crates/gunmetal-core/src/gain.rs`.
- **Serves** MUS-084, MUS-085, MUS-087, MUS-088, MUS-089. (The decision
  already reports its source and clamp, so showing the gain applied,
  MUS-090, and mono and balance, CLI-151, both R1.1, need only the
  client.)
- **Security.** Boundaries TB9; threats TM-T20. Verifies SEC-MED-015.
- **Scope.** Given a track's tag gains and peaks, its Opus header gain, the
  mode (auto, track, album, off), the target level and whether the
  previous and next items are from the same album in the From lane, return
  the gain to apply, its source (tagged, measured, estimated) and the
  clamp that was applied. Implements RFC 7845's rule that the Opus header
  gain always applies and R128 tags add to it.
- **Not in scope.** Measuring loudness (WP-029, R1.3). Applying gain
  (client).
- **Interface sketch.** `pub fn decide(input: &GainInput, mode: Mode, target: Lufs) -> GainDecision`.
- **Tests.** A grid of gain and peak values, each row with its literal
  expected decision worked out by hand from RFC 7845 and the feature map's
  rules, including +60 dB, −60 dB, NaN
  rejected earlier, a peak of exactly 1.0 and of 1.5; positive gain with no
  peak known (not applied); album gain chosen only inside an album run;
  Opus header gain plus R128 track gain. Property: applied gain never
  takes the recorded peak above full scale.

### WP-030 Player state machine

- **Wave** 1 · **Size** S · **Depends on** WP-001.
- **Owns** `crates/gunmetal-core/src/player.rs`.
- **Serves** player.md "One playback model" (a proposal); MUS-079,
  ACC-069. The "stopped by the owner" transition is part of player.md's
  diagram and stays here; the admin action that triggers it (ADM-102) is
  R1.2, WP-153.
- **Security.** Boundaries TB4; threats TM-T16. Verifies no requirement of
  its own: it holds no security control, and the rules it relies on are
  proved by the packages that own them.
- **Scope.** The states and transitions in player.md's diagram as a pure
  transition function, including Stopped for "stopped by the owner" and
  "device revoked".
- **Interface sketch.** `pub fn next(state: State, event: Event) -> Result<State, Illegal>`.
- **Tests.** Every legal transition in the diagram; every illegal pair
  rejected (generated from the full product of states and events, with the
  legal set written out literally in the test).

### WP-031 Token and secret formats

- **Wave** 1 · **Size** M · **Depends on** WP-005, WP-006.
- **Owns** `crates/gunmetal-core/src/token/`.
- **Serves** ACC-122, INT-023; API-STR-01.
- **Security.** Boundaries TB4; threats TM-T16, TM-T67. Verifies
  SEC-API-026, SEC-API-027.
- **Scope.** The capability-URL token layout from web-and-api-security.md
  (version, key ID, expiry, operation, representation from a fixed
  server-defined set, object, session or share handle, and an
  HMAC-SHA-256 tag of at least 128 bits over a domain-separation label and
  the fields), carried in the path, never the query, with verification in
  the order parse, MAC in constant time, expiry (SEC-API-026); the
  lifetime calculator from SEC-API-027 (a stream lasts the item's duration
  plus 10 minutes, at most 4 hours; artwork lasts 1 hour aligned to a
  fixed time bucket, so repeat requests in a bucket reuse one URL);
  hashing of session tokens with a derived key. The API key format
  (`gmk_…`) moved to WP-091 in R2 with API keys themselves (owner
  decision 8 of the security baseline). The core never
  sees key bytes: it hands the message to a `MacProvider` and gets the tag
  back, so the server can sign and verify without calling
  `Secret::expose` outside `gunmetal-secrets` (see the ground rules).
- **Not in scope.** Session lookup and visibility checks (WP-082).
- **Interface sketch.** `pub trait MacProvider { fn current_kid(&self) -> u8; fn mac(&self, kid: u8, msg: &[u8]) -> Option<[u8; 32]>; }`;
  `pub fn sign(fields: &CapFields, mac: &dyn MacProvider) -> String`;
  `pub fn verify(token: &str, mac: &dyn MacProvider, now: Timestamp) -> Result<CapFields, CapError>`;
  `pub fn lifetime(kind: CapKind, item_duration: Option<Duration>, now: Timestamp) -> Expiry`.
- **Tests.** Through a fake `MacProvider`, written in the test, that
  returns the literal RFC 4231 HMAC-SHA-256 tags for the RFC's messages:
  the token layer passes exactly the expected message bytes and places
  the returned tag exactly where the layout says. The core has no MAC
  helper of its own: the first draft tested one here, which duplicated
  `KeyRing::mac` (WP-047) and put a MAC crate in the core outside the one
  crypto module (SEC-STD-018); the real RFC 4231 vectors are WP-047's
  test. A token signed with key 1 verified during rotation overlap; a flipped bit
  in every field rejected; one field substituted from another valid token
  rejected; expiry at the boundary second; an unknown version; a
  representation outside the fixed set; truncation at every length. The
  lifetime calculator against the literal table: a 3-minute track, a
  5-hour audiobook (capped at 4 hours), and artwork requested twice in one
  bucket (same expiry) and once across the boundary. Properties: sign then
  verify returns the fields; any single-bit change fails; the token
  parser has a fuzz harness.

### WP-032 Rate-limit arithmetic

- **Wave** 1 · **Size** S · **Depends on** WP-005.
- **Owns** `crates/gunmetal-core/src/ratelimit.rs`.
- **Serves** ACC-063, INT-012; API-AUTH-07.
- **Security.** Boundaries TB1, TB2; threats TM-T04, TM-T09. Verifies
  SEC-API-056, SEC-API-057, SEC-NET-051, SEC-NET-052, SEC-IAM-101 (the
  arithmetic of the per-source and server-wide limits).
- **Scope.** A GCRA limiter over an injected clock; the server-wide
  counter as the same GCRA under a `LimitKey::Global` key with its own
  ceiling, so the credential verifier (WP-064) and the request limits
  (WP-130) use one implementation of both per-source and server-wide
  limits rather than each building its own (SEC-IAM-101); the one delay schedule
  for guessable secrets (30 seconds, 1 minute, 5 minutes, then 15 minutes,
  never longer and never permanent, SEC-API-056); limiter key derivation
  (the principal when there is one, otherwise the IPv4 address or the
  /64, /56 and /48 IPv6 prefixes, each counter with its own ceiling,
  SEC-API-057, SEC-NET-052); and a fixed-capacity keyed store with
  eviction, so state for unauthenticated clients stays bounded at any
  request volume (SEC-NET-051).
- **Interface sketch.** `pub fn check(state: Option<Tat>, now: Timestamp, rate: Rate) -> (Decision, Tat)`;
  `pub fn keys(principal: Option<PrincipalKey>, addr: IpAddr) -> LimitKeys`;
  `pub struct BoundedStore<K, V>`.
- **Tests.** Bursts at the limit and one over; recovery after exactly the
  emission interval; clock going backwards (treated as no time passed);
  the delay schedule at each step, with the literal delays. Properties:
  over any sequence of requests, allowed requests never exceed the burst
  plus rate times elapsed time; every address in one /64 maps to the same
  /64 key, and an attacker rotating through a /48 exhausts the /48
  ceiling; the bounded store never exceeds its capacity under any
  sequence of operations; for any interleaving of sources, the requests
  allowed under the global key never exceed its ceiling, whatever each
  per-source key allows.

### WP-033 Authorisation policy

- **Wave** 1 · **Size** M · **Depends on** WP-006.
- **Owns** `crates/gunmetal-core/src/authz/`.
- **Serves** ACC-120, ACC-121, ACC-030, ACC-037; every route. (Roles are
  capability presets, so a second administrator, ACC-040, R1.2, WP-152,
  needs no change here; the key no-escalation rule, INT-022, is R2 with
  WP-091.)
- **Security.** Boundaries TB4, TB11; threats TM-T12, TM-T13, TM-T14,
  TM-T15. Verifies SEC-IAM-001, SEC-IAM-002, SEC-IAM-013, SEC-IAM-068,
  SEC-IAM-073, SEC-IAM-074, SEC-IAM-075, SEC-TM-005, SEC-TM-017, SEC-TM-024,
  SEC-TM-027, SEC-API-020, SEC-API-075, SEC-HIS-001, SEC-HIS-013.
- **Scope.** One deny-by-default pure function from a principal (its
  kind from a closed set, its capabilities and ceilings, grants, and the
  scope of a narrower credential such as a share link or a paired
  browser), an action from a closed enumeration, a resource description
  and a context to allow or a typed denial reason, with exhaustive matches
  so a new action cannot compile without a decision (SEC-IAM-068,
  SEC-TM-005). Authorisation tests capabilities, never role names; roles
  are named presets of capabilities, and the owner-only capabilities can
  never be granted to anyone else (SEC-IAM-074, SEC-IAM-075). Context
  (network location, path class, posture) may only add a restriction,
  never widen what a principal may do (SEC-IAM-013, SEC-API-075,
  SEC-HIS-001). The no-escalation rule for invitations and roles
  (SEC-IAM-073, SEC-TM-027) and the scope intersection rule (SEC-API-020).
  The host-equivalent action list as one constant, each carrying the
  fresh-uv tag (SEC-TM-017, SEC-IAM-041). Three plain types that the
  server packages share, defined here so that WP-062, WP-065 and WP-067 in
  wave 2 need nothing from each other: `Principal` (kind, account, profile,
  device and its class, session handle, epoch, credential scope and the
  `PrincipalFacts`), `Permit` (the result of an allowing decision,
  carrying the `LibrarySet` the principal may see, constructible only by
  `decide`) and the `HasLibrary` trait that every catalogue row
  implements. Because only `decide` can mint a `Permit`, and storage
  readers take a `&Permit`, a handler that has not asked the policy cannot
  read storage; this replaces the first draft's public `LibrarySet`
  constructor, which was a convention rather than a guarantee
  (SEC-API-010).
- **Not in scope.** Database queries (WP-065).
- **Open follow-up.** WF-002 added `PathClass::UntrustedProxy` to the
  core's client context. `Context::is_home` already treats it as
  non-local, but the tests in `authz/context.rs`, `authz/decide.rs` and
  `authz/properties.rs` still enumerate four path classes; they must name
  the fifth, so that a test proves a request from an untrusted proxy is
  never local (review of pull request #68, finding 5).
- **Interface sketch.** `pub fn decide(p: &PrincipalFacts, a: Action, r: &ResourceFacts, c: &Context) -> Result<Permit, Denial>`;
  `pub fn may_issue(creator: &PrincipalFacts, requested: &Scope) -> Result<Scope, Denial>`;
  `pub const HOST_EQUIVALENT: &[Action]`.
- **Tests.** A capability matrix written out literally as a table of
  expected results for every principal kind and action. Compile-fail
  tests (trybuild) that `Permit` cannot be built outside `decide`.
  Properties: a principal with no grants is denied everything except
  public actions; an unknown action is denied; adding a grant never
  removes an allowance; adding or changing any context signal never turns
  a denial into an allowance; `may_issue` never returns a scope larger
  than the creator's; no sequence of grants gives an owner-only
  capability to a non-owner; every action in `HOST_EQUIVALENT` requires
  fresh user verification. Zero surviving mutants, as for all core code.

### WP-034 User events: envelope, clock and merge rules

- **Wave** 1 · **Size** M · **Depends on** WP-002 (accepted), WP-005,
  WP-006.
- **Owns** `crates/gunmetal-core/src/userdata/` (`crates/gunmetal-core/src/userdata/event.rs`, `crates/gunmetal-core/src/userdata/hlc.rs`,
  `crates/gunmetal-core/src/userdata/merge.rs`).
- **Serves** CLI-093, MUS-180, MUS-182 to MUS-184, LAT-006, LAT-007;
  API-LOG-01 to API-LOG-04; the conflict table in api-needs.md. (Ratings,
  MUS-181, and dismissals, DIS-022 and DIS-023, are R1.1, WP-141; the
  curation bodies are R1.3, WP-107.)
- **Security.** Boundaries TB4, TB10; threats TM-T18. Verifies SEC-PRV-002.
- **Scope.** The event envelope (event ID, hybrid logical clock, device,
  profile, schema version, typed body), the clock itself, the body types
  for R1 (play, skip, love and unlove, remove play, settings change,
  document operation and snapshot) and the merge rules from api-needs.md's
  table as pure functions over event sets. The envelope keeps unknown body
  types and passes them through, so the bodies the point releases add
  (rate, dismiss and undo in R1.1, WP-141; the household curation
  operations in R1.3, WP-107) need no change to the format.
- **Not in scope.** Storage (WP-068). The rate, dismiss and undo bodies
  (WP-141, R1.1; the rating scale, owner decision 17, is fixed before that
  package starts). The curation bodies (WP-107, R1.3).
- **Interface sketch.** `pub struct Hlc { wall_ms: u64, logical: u32 }` with
  `send` and `receive`; `pub fn derive_counts(events: &[Event]) -> Counts`;
  `pub fn current_love(events: &[Event], item: ItemRef) -> bool`.
- **Tests.** Clock: receive from a node ahead, behind and equal; logical
  overflow. Merge: plays de-duplicated by ID; a removal hides a play
  whatever the arrival order; latest clock wins for loves, with a tie
  broken by device ID; an event of a body type this version does not know
  is kept and passed through unchanged. A play event's serialised form holds exactly
  the profile ID, the item's content identity, the device ID, UTC times,
  the position and the completion state, compared with a literal
  encoding, and has no field that could carry an address, a location, a
  title or a path (SEC-PRV-002). Properties: merging is commutative,
  associative and idempotent over event sets (the property that makes
  offline merging safe).

### WP-035 Log segment framing

- **Wave** 1 · **Size** M · **Depends on** WP-002 (accepted), WP-004.
- **Owns** `crates/gunmetal-core/src/logframe.rs`.
- **Serves** ADM-078, ADM-058; LAT-007.
- **Security.** Boundaries TB10; threats TM-T60. Verifies SEC-MED-001,
  SEC-MED-005, SEC-MED-006, SEC-MED-007, SEC-MED-008, SEC-TM-032,
  SEC-HIS-036.
- **Scope.** Encode and decode segment headers and records (length,
  CRC-32C, version, payload), recovery from a torn tail by cutting to the
  last whole record, and reporting of damage in the middle with the byte
  ranges lost and the next good record found by resynchronising.
- **Not in scope.** Files and `fsync` (WP-068).
- **Interface sketch.** `pub fn encode(payload: &[u8]) -> Vec<u8>`;
  `pub fn records(segment: &[u8]) -> Records<'_>` yielding records or
  `Damaged { range }`; `pub fn recover_tail(segment: &[u8]) -> usize`.
- **Tests.** CRC-32C against published check values; a record cut at every
  byte; a flipped bit in a middle record; a length field claiming 4 GB;
  two damaged regions. Property: for any sequence of payloads, encode then
  `records` yields them in order; for any truncation, `recover_tail` keeps
  exactly the whole records before the cut.

### WP-036 Collation: folding and natural sort

- **Wave** 1 · **Size** M · **Depends on** WP-005.
- **Owns** `crates/gunmetal-core/src/collate.rs`.
- **Serves** MUS-020, DIS-085. (The alphabet jump, DIS-101 and CLI-040, is
  R1.1, WP-147, which adds the jump letter on top of these sort keys.)
- **Security.** Boundaries TB4; threats TM-T09. Verifies no requirement of
  its own: it holds no security control, and the rules it relies on are
  proved by the packages that own them.
- **Scope.** Search folding (case, diacritics through canonical
  decomposition, width, ligatures, punctuation), sort keys that honour
  sort-name tags and natural number order, and leading-article handling
  with a per-language list.
- **Not in scope.** The letter used by the alphabet jump (WP-147, R1.1).
- **Interface sketch.** `pub fn fold(s: &str) -> String`; `pub fn sort_key(display: &str, sort_tag: Option<&str>, lang: Lang) -> SortKey`.
- **Tests.** "Björk" and "Bjork"; "AC/DC"; "The The"; "Track 2" before
  "Track 10"; Japanese, Greek and Cyrillic titles; an empty string; a title
  of only punctuation. Property: `sort_key` order is a total order and
  stable under folding.

### WP-037 Artwork placeholder and palette

- **Wave** 1 · **Size** M · **Depends on** WP-001.
- **Owns** `crates/gunmetal-core/src/imagedata.rs`.
- **Serves** LIB-142, MUS-110; API-SYNC-05, API-CAT-07.
- **Security.** Boundaries TB6; threats TM-T28. Verifies no requirement of
  its own: it holds no security control, and the rules it relies on are
  proved by the packages that own them.
- **Scope.** From a small RGBA buffer the worker has already decoded and
  shrunk: a compact placeholder hash in the style of ThumbHash or
  BlurHash (the exact scheme is a choice for this package, recorded in the
  code), and up to three palette candidates with contrast information for
  the tint rules in design-language.md.
- **Not in scope.** Decoding images (WP-079).
- **Interface sketch.** `pub fn placeholder(rgba: &[u8], w: u16, h: u16) -> Result<Placeholder, ImageDataError>`;
  `pub fn palette(rgba: &[u8], w: u16, h: u16) -> Result<Vec<Swatch>, ImageDataError>`.
- **Tests.** Solid colours, a two-colour split, a gradient, a 1×1 image, a
  buffer whose length does not match the dimensions, and fully transparent
  pixels. Each placeholder is checked by decoding it with a decoder written
  independently in the test from the chosen scheme's published description
  (and against the scheme's published reference outputs if it has any,
  unverified): a solid colour decodes to that colour within a stated
  tolerance, and the two-colour split decodes to two regions. The palette
  of a solid image is that colour; of the split, both colours. Properties:
  the placeholder always fits its fixed size; palette candidates are
  distinct. Without the decoding checks, a placeholder of all zeros would
  pass every property.

### WP-038 One-time codes

- **Wave** 1 · **Size** S · **Depends on** WP-005, WP-006.
- **Owns** `crates/gunmetal-core/src/otp.rs`.
- **Serves** ACC-001, ACC-064; API-AUTH-01.
- **Security.** Boundaries TB1, TB4; threats TM-T04, TM-T06, TM-T64.
  Verifies SEC-IAM-007, SEC-IAM-056.
- **Scope.** The formats of every human-handled code, generated from
  random bytes the caller supplies (the server reads them from the one
  CSPRNG function): the setup claim code (128 bits, grouped, with a
  checksum character and ambiguity folding, SEC-IAM-007), recovery codes
  (80 bits each, SEC-IAM-089), and pairing user codes (8 characters from
  the RFC 8628 section 6.1 base-20 alphabet, SEC-IAM-056). TOTP was
  removed: the security baseline has no passwords and no TOTP
  (SEC-IAM-025), so ACC-053's authenticator codes are withdrawn in R1.
- **Interface sketch.** `pub fn claim_code(random: [u8; 16]) -> ClaimCode`;
  `pub fn recovery_code(random: [u8; 10]) -> RecoveryCode`;
  `pub fn pairing_code(random: [u8; 8]) -> PairingCode`;
  `pub fn parse_code(s: &str, kind: CodeKind) -> Result<Code, CodeError>`.
- **Tests.** Each generator's length, alphabet and checksum against literal
  expected codes for literal random inputs; a pairing code uses only the
  base-20 alphabet for every input byte (property, with the alphabet
  written out in the test); codes typed with O for 0, with spaces and in
  lower case parse; a wrong checksum is refused; a code of the wrong kind
  is refused.

### WP-039 Wire codec

- **Wave** 1 · **Size** M · **Depends on** WP-004, WP-006.
- **Owns** `crates/gunmetal-core/src/wire.rs`.
- **Serves** API-SYS-03, API-SYNC-01, API-SYNC-02. The version negotiation
  lets the server refuse a mismatched client with a clear answer in R1;
  keeping old native clients working across versions (CLI-032) is R2.
- **Security.** Boundaries TB4, TB6; threats TM-T20, TM-T62. Verifies
  SEC-MED-001, SEC-MED-005, SEC-MED-006, SEC-MED-007, SEC-MED-008,
  SEC-TM-032, SEC-HIS-036, SEC-MED-023, SEC-MED-077, SEC-CLI-021,
  SEC-HIS-035.
- **Scope.** Length-prefixed frames with a protocol version and a frame
  kind, capped at 32 MiB, carrying `postcard`-encoded protocol types (or a
  hand-written codec if owner decision 4 goes that way); the version
  negotiation rule (the server speaks the current and previous protocol
  version).
- **Interface sketch.** `pub fn write_frame<T: Serialize>(kind: FrameKind, v: &T) -> Result<Vec<u8>, WireError>`;
  `pub fn read_frame(input: &[u8], max: u32) -> Result<(FrameKind, &[u8], usize), WireError>`;
  `pub fn negotiate(client: &[ProtocolVersion], server: &[ProtocolVersion]) -> Result<ProtocolVersion, Upgrade>`.
- **Tests.** Frames at and over the cap; a length prefix of `u32::MAX`; a
  frame cut at every byte; an unknown kind; negotiation with disjoint,
  overlapping and identical version sets. Property: write then read returns
  the same bytes and kind.

### WP-040 Catalogue types

- **Wave** 1 · **Size** M · **Depends on** WP-005, WP-006.
- **Owns** `crates/gunmetal-core/src/catalog/`.
- **Serves** API-CAT-01 to API-CAT-10 (the types); LAT-001; the synced
  fields behind MUS-051, MUS-054, MUS-060 and LIB-146 (artist and album
  aggregates, sort fields, the genre index and technical fields, which the
  client reads from the synced copy; MUS-060 browses by genre in R1, and
  by mood and label from R1.1 with MUS-019, register D-88). The R1.1 views and filters over the
  same fields (MUS-056, DIS-102) need no further types. People with typed
  roles and typed links between items (LAT-002, LAT-008) are R1.3,
  WP-161.
- **Security.** Boundaries TB4; threats TM-T15. Verifies no requirement of
  its own: it holds no security control, and the rules it relies on are
  proved by the packages that own them.
- **Scope.** The shared vocabulary of the music model, as plain data:
  `TrackTags` (every canonical tag the mappers fill, multi-valued where the
  feature map says so), `Credit` and `Role`, `ReleaseType`, `TechInfo`
  (codec, container, sample rate, bit depth, channels, bitrate, duration),
  `Trim`, `GainTags`, `ArtworkRef`, `LyricsSource`, `FileFacts` (what a
  probe returns), and the synced-library record types (track, album,
  artist) with their field IDs for search and, from R1.1, the rule
  format (WP-027, register D-85; the release-group record is added by
  WP-146 in R1.1), and
  `CatalogChange` (what changed: kind, ID, upsert or removal),
  which WP-067 returns and WP-066 records, so those two wave 2 packages do
  not depend on each other. Item kinds leave room for video and later media (LAT-001).
- **Not in scope.** Any logic beyond constructors and validation.
- **Tests.** Constructors refuse invalid combinations (a disc number of 0;
  a track count less than the track number is kept but flagged). Field-ID
  table: every field has a stable ID and a type, checked against a literal
  list.
- **Risks.** Every wave 2 and 3 core package reads these types. Changes
  after merge go through the merge protocol's interface-change rule.

### WP-041 WebAuthn data parsing

- **Wave** 1 · **Size** M · **Depends on** WP-004.
- **Owns** `crates/gunmetal-core/src/webauthn/` (`crates/gunmetal-core/src/webauthn/cbor.rs`, `crates/gunmetal-core/src/webauthn/cose.rs`,
  `crates/gunmetal-core/src/webauthn/authdata.rs`).
- **Serves** ACC-050; API-AUTH-03, API-AUTH-04.
- **Security.** Boundaries TB1, TB4; threats TM-T03, TM-T20. Verifies
  SEC-MED-001, SEC-MED-005, SEC-MED-006, SEC-MED-007, SEC-MED-008,
  SEC-TM-032, SEC-HIS-036, SEC-IAM-018, SEC-IAM-019, SEC-IAM-020,
  SEC-IAM-021.
- **Scope.** A bounded CBOR reader for the subset WebAuthn uses, COSE keys
  for ES256 and EdDSA (and RS256 if owner decision 9 includes it), and the
  authenticator data structure (RP ID hash, flags, sign count, attested
  credential data). Attestation format "none" only.
- **Not in scope.** Signature verification and ceremonies (WP-081).
- **Interface sketch.** `pub fn auth_data(b: &[u8]) -> Result<AuthData, WebauthnError>`;
  `pub fn cose_key(b: &[u8]) -> Result<CoseKey, WebauthnError>`.
- **Tests.** Authenticator data built by hand in the test (it is a simple
  layout); CBOR maps with duplicate keys, indefinite lengths (refused),
  nesting past the limit, integers at every width; flags combinations.

### WP-042 Store foundation

- **Wave** 1 · **Size** M · **Depends on** WP-007, WP-122, WP-126.
- **Owns** `crates/gunmetal-store/` (creates the crate: `crates/gunmetal-store/Cargo.toml`,
  `crates/gunmetal-store/src/lib.rs`, `crates/gunmetal-store/src/writer.rs`, `crates/gunmetal-store/src/readers.rs`, `crates/gunmetal-store/src/schema.rs`).
- **Serves** ADM-080, ADM-077, ADM-058.
- **Security.** Boundaries TB10; threats TM-T58, TM-T60. Verifies
  SEC-API-066, SEC-TM-039, SEC-HIS-038, SEC-PRV-050, SEC-TM-050.
- **Scope.** Open the cache through the one connection opener in
  `gunmetal-fs` (WP-126) with the pragmas in this plan, `secure_delete=ON`
  on every connection; the single writer thread; the reader pool; schema
  registration by parts and the digest; the data-class check of WP-122
  against the live schema at open; discard-and-rebuild when the digest or
  generation changes; a write-queue depth counter for Admin >
  Diagnostics. The storage API accepts statements only as WP-126's
  `Query`, with dynamic sorting and filtering from enums, so SQL built
  from request text is a compile error (SEC-API-066, SEC-TM-039,
  SEC-HIS-038). The SQL door itself is WP-126's, which the identity store
  (WP-046) and, from R1.1, the derived-data store (WP-071) use too; as first written
  this package's `sql.rs` was the door, which WP-046, in the same wave,
  could not depend on.
- **Not in scope.** Any domain table.
- **Interface sketch.** `pub struct Store; impl Store { pub fn open(dir: &Path, parts: &[SchemaPart]) -> Result<Opened, StoreError>; pub async fn write<R: Send + 'static>(&self, f: impl FnOnce(&Transaction) -> Result<R, StoreError> + Send + 'static) -> Result<R, StoreError>; pub async fn read<R>(&self, f: impl FnOnce(&Connection) -> R + Send + 'static) -> Result<R, StoreError>; }`
  where `Opened` says whether the cache was fresh, reused or rebuilt.
- **Tests (real SQLite).** Opening twice reuses; changing one part's SQL
  causes a rebuild; a corrupt file (bytes overwritten in the test) causes a
  rebuild rather than a crash; readers see committed data and never a
  half-written batch; a hundred concurrent writes from tasks are
  serialised with no "database is locked" error; `secure_delete` is on for
  every pooled connection; a schema part with an unclassified column
  refuses to open; a compile-fail test shows a `String` cannot reach the
  store's write or read API as query text.
  Load test (kept short in CI): readers' latency while a long batch
  commits stays under a stated bound.

### WP-043 Server skeleton, command line and configuration

- **Wave** 1 · **Size** M · **Depends on** WP-005, WP-006, WP-007, WP-126.
- **Owns** `crates/gunmetal-server/` (creates the crate: `crates/gunmetal-server/Cargo.toml`,
  `crates/gunmetal-server/src/main.rs`, `crates/gunmetal-server/src/lib.rs`, `crates/gunmetal-server/src/cli.rs`, `crates/gunmetal-server/src/config.rs`,
  `crates/gunmetal-server/src/datadir.rs`, `crates/gunmetal-server/src/app.rs`, `crates/gunmetal-server/src/bus.rs`,
  `crates/gunmetal-server/src/clock.rs`, `crates/gunmetal-server/src/log.rs`).
  The route registry and the HTTP listener moved to WP-118 in review: they
  need `gunmetal-http` (WP-044), which is in this same wave.
- **Serves** ADM-001, ADM-006, ADM-007, ADM-079, ADM-090, ADM-119 (the
  logger); API-SYS-07 (the hooks).
- **Security.** Boundaries TB10, TB11; threats TM-T27, TM-T54, TM-T55,
  TM-T57. Verifies SEC-OPS-014, SEC-OPS-022, SEC-OPS-029 (the logging
  rules, the automatic revert and the emitted event; the stored audit
  record is WP-069's), SEC-OPS-053, SEC-IAM-047, SEC-IAM-096,
  SEC-MED-062, SEC-TM-041, SEC-TM-046, SEC-TM-057, SEC-HIS-004 (the
  configuration schema; the sign-in pathways are WP-064's), SEC-HIS-021,
  SEC-PRV-042, SEC-PRV-043, SEC-PRV-044, SEC-STD-023.
- **Scope.** The `gunmetal` binary and its subcommands (`serve`, `doctor`,
  `admin recover`, `migrate`, `rebuild`, `snapshot restore`, and the hidden
  worker entry), parsed by hand, with no subcommand or flag that takes a
  secret (SEC-OPS-014); the configuration file and environment, including
  the `_FILE` rule and systemd credentials and refusing plain secret
  variables; a configuration schema with no key that turns
  authentication off and no field that accepts an executable path,
  command template or hook (SEC-HIS-004, SEC-HIS-021, SEC-TM-046); the
  data directory layout through the data-root handle (WP-126); refusing to
  run as root or with any effective or permitted capability, with no
  override (SEC-OPS-053), and refusing a data directory on a network
  filesystem, both as pure checks over probed host facts; disabling core
  dumps at start (`RLIMIT_CORE` 0 and `PR_SET_DUMPABLE` 0, SEC-STD-023);
  application state; the subcommand dispatch table that later packages
  extend by one line each; an in-process event bus; the system clock; the
  structured logger. The logger writes one JSON object per line with every
  value escaped and length-capped (SEC-OPS-022), takes every event from
  one typed enum that matches the log inventory (SEC-PRV-044), redacts
  capability tokens, invitation, share and pairing secrets and OAuth codes
  and states from any URL it logs (SEC-IAM-047), never records request
  bodies, query strings, cookies, authorisation headers or media titles
  (SEC-OPS-029, SEC-PRV-043), defaults to info level, and switches debug
  level off by itself within 24 hours. Switching debug level on emits the
  typed security event (WP-006) into the injected `SecuritySink`; the bus
  carries every security event to the audit log's sink (WP-069), which is
  what stores the record.
- **Not in scope.** Any route or feature.
- **Interface sketch.** `pub struct AppState { /* one line per module, appended by later packages */ }`;
  `pub fn parse_args(args: &[OsString]) -> Result<Command, UsageError>`;
  `pub fn load_config(text: &str, env: &Env) -> Result<Config, ConfigError>`.
- **Tests.** Every subcommand and every malformed argument; a config with an
  unknown key (refused with its name); a secret in a plain variable
  (refused, message shows the `_FILE` form); a data directory with group
  read permission (refused with the exact `chmod` hint); a probed UID of 0
  and a probed non-empty capability set (each refused, with no override:
  the first draft allowed a documented override, which SEC-OPS-053 and the
  baseline's owner decision 14 rule out) and a probed network filesystem
  type (refused), passed to the checks as inputs; a snapshot of the
  configuration schema that fails when a key could disable authentication
  or name an executable; the logger escaping CR, LF, NUL, U+2028, U+2029
  and ANSI escapes and capping length, with any Unicode input producing
  exactly one parseable JSON line (property); a log-canary test with
  marker values in bodies, query strings, cookies and titles; the debug
  level switching off after 24 hours on a manual clock, and switching it
  on emitting exactly one debug-level event into a recording sink,
  compared as a whole value. Integration:
  `gunmetal serve` on an empty directory creates the layout with the right
  modes and reads its own dumpable flag as 0.

### WP-044 HTTP foundation

- **Wave** 1 · **Size** L · **Depends on** WP-006.
- **Owns** `crates/gunmetal-http/` (creates the crate).
- **Serves** ACC-120, ACC-123, ACC-124 (cookie handling primitives),
  INT-001, INT-007, INT-023; the route table and request pipeline of
  web-and-api-security.md; API-SYS-09.
- **Security.** Boundaries TB1, TB2, TB4; threats TM-T02, TM-T03, TM-T08,
  TM-T10, TM-T11, TM-T14. Verifies SEC-API-001, SEC-API-004, SEC-API-007,
  SEC-API-008, SEC-API-013, SEC-API-019, SEC-API-025, SEC-API-033,
  SEC-API-034, SEC-API-035, SEC-API-036, SEC-API-038, SEC-API-040,
  SEC-API-053, SEC-API-054, SEC-API-055, SEC-API-060, SEC-API-063,
  SEC-API-065, SEC-API-067, SEC-API-068, SEC-API-072, SEC-API-073,
  SEC-API-092, SEC-API-095, SEC-IAM-010, SEC-IAM-014, SEC-IAM-040,
  SEC-IAM-041, SEC-IAM-067, SEC-IAM-072, SEC-TM-005, SEC-TM-009, SEC-TM-040,
  SEC-NET-014, SEC-CLI-004, SEC-CLI-007, SEC-HIS-005, SEC-HIS-009,
  SEC-HIS-037, SEC-HIS-041, SEC-EXT-006, SEC-PRV-018, SEC-PRV-020,
  SEC-STD-013.
- **Scope.** The `RouteSpec` table type and a router built only from it.
  A `RouteSpec` cannot be built without every one of: an access class
  (public, credential exchange, capability, signed-in capability, admin
  capability), the capability it requires, one route tag from the closed
  set none, elevated, fresh-uv, whether it mutates state, its methods, its
  body cap and accepted content types, where its object IDs sit (path,
  query or body, which the generated suites of WP-131 read), its
  rate-limit class and, for a mutating admin route, the audit event it
  emits; a route registered without a policy does not compile (SEC-TM-005,
  SEC-API-001, SEC-API-019, SEC-IAM-041, SEC-IAM-067, SEC-HIS-005,
  SEC-OPS-020). Routes live under `/api/v1` and are matched exactly, so no
  suffix, delimiter or encoding variant reaches a handler (SEC-API-092,
  SEC-API-055). The request pipeline in web-and-api-security.md's order:
  the posture hook (WP-132), Host allow-list answering 421 (SEC-API-007,
  SEC-IAM-010, SEC-NET-014), exact route match and method check with 405
  and method-override headers ignored (SEC-API-008), credential
  extraction from the session cookie and the `Authorization` header only,
  refusing any credential-shaped query parameter or path segment and any
  request carrying two credentials (SEC-API-004, SEC-EXT-006,
  SEC-HIS-041), the `Gunmetal-Request: 1` header on every
  cookie-authenticated request whatever its method, checked with 403
  before authentication (SEC-API-033), `Sec-Fetch-Site` and `Origin`
  checks for cookie credentials (SEC-API-034, SEC-IAM-040, SEC-CLI-007),
  the access-class check hook, the rate-limit hook (WP-130),
  `Content-Type: application/json` only (415 otherwise) and no
  `Content-Encoding` (415) (SEC-API-035, SEC-API-065), and decoding into a
  per-action typed request that rejects unknown fields, duplicate keys and
  repeated or multiply-located parameters, with bounded types for every
  parameter that sizes work (SEC-API-067, SEC-IAM-072, SEC-HIS-037). The
  acting principal comes only from the credential; a body or query field
  naming a user, profile or owner is refused with 400 except on admin
  routes that target another principal (SEC-API-013, SEC-HIS-009). The
  response layer: `X-Content-Type-Options`, `Referrer-Policy`,
  `Cross-Origin-Resource-Policy` and, for HTML, the CSP of SEC-API-044 and
  `Cross-Origin-Opener-Policy`, on every response including errors
  (SEC-API-053, SEC-CLI-004, SEC-PRV-018); `Strict-Transport-Security`
  under a hostname over HTTPS (SEC-API-038); `Cache-Control: no-store` on
  authenticated JSON and on any response carrying Activity, Identity or
  Secret data (SEC-API-055, SEC-PRV-020); a `Content-Type` that matches
  the body (SEC-API-054, SEC-STD-013); no CORS headers (SEC-API-040,
  SEC-IAM-014); no `Server` header with a version. The problem renderer:
  RFC 9457 objects whose `type` comes from the closed catalogue (WP-006),
  with no stack trace, path, SQL or version, and a last-resort layer that
  turns any panic or unexpected error into the generic 500, rolls back and
  grants nothing (SEC-API-072, SEC-API-073, SEC-TM-040). Response types
  are explicit per role and never serialised storage records
  (SEC-API-068). The access log records the route template, never a raw
  path (SEC-API-095). The list paging convention with a page cap of 500
  (SEC-API-063) and opaque cursors (SEC-API-025). An in-process test
  client.
- **Not in scope.** Sessions, principals and authorisation decisions
  (WP-062, WP-065), which plug into the hooks. Connection-level limits and
  timeouts (WP-118). The generated suites over the full route table
  (WP-131).
- **Interface sketch.** As in web-and-api-security.md: `pub enum Access`,
  `pub enum RouteTag { None, Elevated, FreshUv }`, `pub struct RouteSpec`;
  plus `pub fn router(specs: &[RouteEntry], hooks: Hooks) -> axum::Router`
  and `pub struct TestClient`.
- **Tests.** Compile-fail tests (trybuild) for a route without an access
  class, a tag or, when it is a mutating admin route, an audit event.
  Over a stand-in route table: every route appears in the table and
  nowhere else; an unlisted public route fails a test; wrong method gives
  405 with `Allow`, and `X-HTTP-Method-Override` is ignored; every
  response, including errors and 404, carries the exact header set from a
  golden table; bodies over the cap give 413 before decoding; a gzip body
  gives 415; a credential in a query string or path segment gives 400; a
  `Host` not on the allow-list gives 421; a cookie request without the
  `Gunmetal-Request` header gives 403 on every method; a cross-site
  `Sec-Fetch-Site` gives 403; extra, duplicate and repeated fields give
  400; a forced panic in a test-only route gives the generic 500 body,
  compared whole.

### WP-045 Worker sandbox

- **Wave** 1 · **Size** M · **Depends on** WP-006.
- **Owns** `crates/gunmetal-worker/` (creates the crate: `crates/gunmetal-worker/Cargo.toml`,
  `crates/gunmetal-worker/src/lib.rs`, `crates/gunmetal-worker/src/sandbox/`).
- **Serves** the worker isolation the security baseline puts in R1
  (TB6).
- **Security.** Boundaries TB6; threats TM-T21, TM-T23, TM-T52, TM-T53.
  Verifies SEC-MED-018, SEC-MED-021, SEC-MED-022, SEC-MED-024, SEC-MED-063,
  SEC-TM-044, SEC-TM-045, SEC-TM-046, SEC-HIS-020, SEC-HIS-021, SEC-OPS-014,
  SEC-STD-040.
- **Scope.** The sandbox launcher, which is the one typed command builder
  in the codebase (SEC-HIS-020, SEC-MED-063): it starts only programs from
  a closed list in `sandbox/programs.rs` (in R1 only the worker, which is
  the same binary), takes the executable path only from the install
  (SEC-HIS-021, SEC-TM-046), passes input and output as descriptors and
  never paths, builds arguments only from typed values, clears the
  environment and passes no secret as an argument or variable
  (SEC-OPS-014), and connects to the child only over an inherited socket
  pair, never TCP (SEC-STD-040). Inside the worker, in order: rlimits
  (memory, `RLIMIT_CORE` 0, `RLIMIT_NOFILE` of 32 or less, CPU time, no
  new processes) and `PR_SET_DUMPABLE` 0, closing other descriptors,
  `no_new_privs`, Landlock with no filesystem rules and network
  restrictions where the ABI allows, seccomp with the allowlist,
  single-threaded (SEC-MED-021, SEC-MED-022); a self-test that reports
  the isolation tier reached, applying the tier table of SEC-MED-024:
  memory-safe parsing runs at the documented floor (process separation,
  rlimits, `no_new_privs`) with a "reduced isolation" notice when
  seccomp, Landlock or namespaces are missing, and native decoders, of
  which R1 has none, would need the full jail and are otherwise off; no
  setting runs jailed work unconfined (SEC-TM-045). 32-bit ARM builds report
  the reduced tier, and are not claimed as supported until the seccomp
  answer for them is recorded (SEC-MED-024; owner decision 24).
- **Not in scope.** The protocol (WP-061), the pool (WP-078).
- **Interface sketch.** `pub enum Program { Worker }` (closed);
  `pub fn launch(program: Program, args: TypedArgs, fds: Inherited) -> Result<Child, SpawnError>`;
  `pub fn confine(profile: Profile) -> Result<Tier, ConfineError>`;
  `pub fn self_test() -> TierReport`.
- **Tests.** Integration on Linux CI with test hooks behind a cargo feature,
  using a hostile test worker: one that tries to open a path is killed or
  refused; one that tries a network socket is refused; one that tries to
  execute a program is refused; one that exceeds its memory limit dies and
  the parent sees the reason; the environment is empty and the child's
  descriptors are exactly the socket pair; the dumpable flag is 0. Unit:
  the tier table from SEC-MED-024 as a pure function of what was
  enforced, with each probe forced to fail in turn and the exact notice
  text asserted; the launcher's argument builder accepts no string from
  outside (compile-fail). A CI matrix job on a kernel without Landlock
  asserts the reduced tier and its notice.
  The worker crate has no binary of its own, so the tests spawn the test
  executable itself (`std::env::current_exe()`) with a hidden argument that
  enters the child path; no extra binary ships. The gate must build with
  the test-hook feature for these tests to run at all, which is a gate
  change request to the integrator, not an edit to `gate.sh`.
- **Owns, in detail.** The syscall allowlist lives in its own file,
  `crates/gunmetal-worker/src/sandbox/syscalls.rs`, so that its ownership
  can pass to WP-079 in wave 3. The program list,
  `crates/gunmetal-worker/src/sandbox/programs.rs`, is a registry file
  (see the shared-file table).
- **Risks.** CI runners' kernels decide which Landlock ABI is testable
  (unverified for GitHub's runners). The seccomp allowlist is derived from
  running real jobs in audit mode. As first planned, this package could
  only finish after WP-079 in a later wave, which no package may wait for.
  It now ships the filter mechanism and a minimal allowlist proven against
  the host-loop stub; WP-079 takes over `sandbox/syscalls.rs` and extends
  it from audit runs of the probe and artwork jobs.

### WP-046 Identity store

- **Wave** 1 · **Size** M · **Depends on** WP-002 (accepted), WP-125
  (identity record accepted, SEC-STD-006), WP-005, WP-007, WP-122, WP-126.
- **Owns** `crates/gunmetal-durable/` (creates the crate: `crates/gunmetal-durable/Cargo.toml`,
  `crates/gunmetal-durable/src/lib.rs`, `crates/gunmetal-durable/src/identity/`).
- **Serves** ACC-013, ADM-056, ADM-057, ADM-058; INT-008.
- **Security.** Boundaries TB4, TB10; threats TM-T12, TM-T57, TM-T60.
  Verifies SEC-IAM-001, SEC-IAM-004, SEC-TM-050, SEC-TM-051, SEC-OPS-048,
  SEC-OPS-049, SEC-OPS-051, SEC-API-023, SEC-HIS-012 and SEC-PRV-021
  (each for storing and never deriving; the CSPRNG source is WP-047's),
  SEC-PRV-023, SEC-API-066, SEC-TM-039, SEC-PRV-050, SEC-TM-024,
  SEC-API-010.
- **Scope.** Open `durable/identity.db` through the one connection opener
  (WP-126) with `synchronous=FULL` and `secure_delete=ON`, and run
  statements only as `Query` values (SEC-API-066, SEC-TM-039,
  SEC-PRV-050); schema by parts before R1,
  numbered migrations after; the data-class check at open; the migration
  runner with snapshot, integrity check, invariants and refusal of newer
  versions, never discarding identity, audit or security configuration to
  start (SEC-OPS-048, SEC-OPS-051); no migration may make an installation
  less strict or change an existing person's privacy setting, so a new
  setting defaults to its strictest value on upgraded installs
  (SEC-OPS-049, SEC-PRV-023). Accounts, credential public keys, devices,
  grants, invitations and shares live here and never in the cache
  (SEC-IAM-004, SEC-TM-051). The public-ID mapping table and its API: an
  identifier is minted once for a content identity or a stored object and
  kept here, never derived from a path or a name (SEC-HIS-012,
  SEC-API-023, SEC-PRV-021; owner decision 11). This package takes a
  freshly minted `PublicId` as input and never generates one: the bytes
  come only from WP-047's minting function, which this wave-1 package
  cannot depend on, and a second call to the OS random source here would
  be a second door. Readers: no public read returns a user-visible object
  (an account, device, grant, invitation or share) without a `Permit`
  from the policy (WP-033), so a handler that has not asked the policy
  cannot reach the identity store (SEC-TM-024, SEC-API-010). Because
  `Permit` is WP-033's, in this same wave, this package keeps its general
  reader `pub(crate)`, and WP-065 (wave 2) adds the public
  `Permit`-taking reader in its own file of this crate. The only public
  reads without a `Permit` are the pre-principal lookups on a short
  written list, a closed `PrePrincipal` enum whose method a
  `disallowed-methods` entry confines to three server modules: the
  session-token lookup (WP-062, `session/`), the credential lookup handed
  to pathway checks through the verifier's pre-authentication handle
  (WP-064, `verifier/`), and the grant read that builds the `Permit`
  (WP-065, `access/`).
- **Not in scope.** Accounts, sessions and other tables, which their
  packages register as parts. The `Permit`-taking reader (WP-065).
- **Interface sketch.** `pub struct IdentityStore; impl IdentityStore { pub fn open(root: &DataRoot, parts: &[SchemaPart], invariants: &[Invariant]) -> Result<Self, IdentityError>; pub async fn write<R>(..); pub(crate) async fn read<R>(..); pub async fn read_pre_principal<R>(&self, lookup: PrePrincipal, ..); }`,
  where `PrePrincipal` is a closed enum of the three exempt lookups.
- **Tests (real SQLite).** A migration that violates an invariant rolls back
  and leaves the snapshot; a file from a "newer" version (built in the
  test) is refused with the restore command in the message; the snapshot
  passes `integrity_check`; a crash simulated between snapshot and commit
  leaves a usable file; a test migration that loosens a setting fails the
  strictness property over generated pre-migration configurations. Two
  fresh stores given different minted IDs for the same content
  identities keep them, a cache rebuild keeps them, and no API derives
  an ID from an identity, a path or a name. `secure_delete` reads back as
  on for every pooled connection. A compile-fail test that the general
  reader is not callable from outside the crate; `PrePrincipal` has
  exactly the three listed variants (a literal list in the test).

### WP-047 Secrets and keys

- **Wave** 1 · **Size** L (was M; it gained the CSPRNG door, the KDF
  floor, key revocation, the crypto module and the public-ID minting
  function) · **Depends on** WP-005, WP-006, WP-008, WP-122, WP-126.
- **Owns** `crates/gunmetal-secrets/` (creates the crate; the crypto door
  is `crates/gunmetal-secrets/src/crypto/`),
  `crates/xtask/src/crypto_inventory.rs`.
- **Serves** ACC-123, ACC-122 (key rotation).
- **Security.** Boundaries TB10, TB12; threats TM-T57, TM-T10. Verifies SEC-OPS-011,
  SEC-OPS-012, SEC-OPS-013, SEC-OPS-015, SEC-OPS-017, SEC-IAM-095,
  SEC-TM-012, SEC-TM-049, SEC-TM-057, SEC-API-030, SEC-HIS-011, SEC-HIS-043,
  SEC-HIS-044, SEC-PRV-038, SEC-PRV-042, SEC-STD-018, SEC-STD-020,
  SEC-STD-021, SEC-STD-022, SEC-STD-024, and the "from the OS CSPRNG"
  part of SEC-HIS-012, SEC-API-023 and SEC-PRV-021.
- **Scope.** The one function that reads security randomness from the
  operating system CSPRNG, which stops the operation (and the server at
  start-up) rather than fall back when randomness is unavailable
  (SEC-STD-022, SEC-HIS-043). The one function that mints public IDs:
  it draws 16 bytes through that function and is the only caller of
  `Minted::from_os_random` (WP-006), so every `PublicId` in the system
  came from the OS CSPRNG (SEC-HIS-012, SEC-API-023, SEC-PRV-021). The
  crypto module, `src/crypto/`: the only place outside the core's
  SHA-256 file (WP-122) that uses a cryptographic crate, holding HMAC,
  HKDF, XChaCha20-Poly1305, Argon2id, Ed25519 signing and verification,
  P-256 verification for passkeys, age encryption and decryption, and
  the constructors of the rustls provider and of the server and client
  TLS configurations; keyless verifications are plain functions that
  take a public key, so the passkey, update-feed and egress packages call
  them without holding any secret (SEC-STD-018, SEC-STD-019). Generate and load the root secret (256
  bits) and every other key from it, through the data-root handle, as
  0600 files in a 0700 directory with repair-or-refuse checks, from a
  file, systemd credentials or container secrets (SEC-OPS-011, SEC-OPS-012,
  SEC-PRV-038, SEC-TM-012); derive one key per purpose with HKDF and a
  key ID (SEC-OPS-015); the key ring for URL-signing keys, rotated at
  least every 30 days with an overlap selected by key ID and revocable so
  that revoking a key invalidates every URL it signed (SEC-API-030; the
  plan rotates daily, which is stricter); `Secret<T>` with no `Display`,
  `Serialize` or `PartialEq`, a fixed `Debug`, zeroising on drop and
  `expose()` reachable only here (SEC-IAM-095, SEC-OPS-013, SEC-HIS-011);
  the vault for secrets the server must replay to others, in R1 the DNS
  provider token that ACME DNS-01 needs for the owner's own domain
  (WP-101), and from R1.2 the OIDC client secret (WP-096)
  (XChaCha20-Poly1305 with 192-bit random nonces and
  the record ID as associated data, one opaque error for every
  decryption failure, SEC-OPS-017, SEC-STD-020, SEC-STD-021); an Argon2id
  helper with the RFC 9106 parameter floor for any key or verifier derived
  from a human secret (SEC-STD-024). Keyed operations happen inside this
  crate: `root.key_ring(purpose).mac(kid, msg)` returns an HMAC-SHA-256
  tag, so the server signs capability URLs, hashes session tokens and
  applies the recovery-code pepper without any key leaving the crate
  (SEC-HIS-044). This replaces an earlier sketch in which core functions
  took key bytes, which would have needed `expose()` in the server.
  Replayed secrets are decrypted only for the one call that needs them
  (SEC-STD-023); the egress client receives them as a `Secret` header
  value and is the one other module allowed to expose one, to write it
  into the request (see WP-101; WP-096 in R1.2).
- **Interface sketch.** `pub struct Root; impl Root { pub fn load_or_create(dir: &Path) -> Result<Self, SecretsError>; pub fn key_ring(&self, purpose: Purpose) -> KeyRing; }`;
  `impl KeyRing { pub fn current_kid(&self) -> u8; pub fn mac(&self, kid: u8, msg: &[u8]) -> Option<[u8; 32]>; }`;
  `pub struct Vault; impl Vault { pub fn seal(&self, id: &[u8], plain: &[u8]) -> Vec<u8>; pub fn open(&self, id: &[u8], sealed: &[u8]) -> Result<Secret<Vec<u8>>, SecretsError>; }`.
- **Tests.** HKDF against RFC 5869 vectors; `KeyRing::mac` against RFC 4231
  HMAC-SHA-256 vectors with a test root; rotation keeps the previous kid
  answerable for the overlap and refuses it after; a revoked kid is
  refused at once; vault round trip; a sealed blob moved to another record
  ID fails with the same opaque error as a flipped tag; a root file with
  mode 0644 refused or repaired as configured; two fresh data directories
  share no key bytes; a randomness source forced to fail stops start-up;
  the nonce generator never repeats over a large sample (property);
  `format!("{:?}", secret)` prints the placeholder, and compile-fail tests
  show `Secret` has no `Display`, `Serialize` or `PartialEq`; the Argon2id
  helper refuses parameters below the floor. The minting function with a
  randomness source forced to fail returns the error and mints nothing;
  100,000 IDs minted across threads are distinct. Ed25519 against the RFC
  8032 vectors; P-256 verification against published ECDSA P-256 SHA-256
  test vectors; the client TLS configuration refuses an untrusted
  certificate. An xtask check, with fixture trees, fails when the
  cryptographic inventory of WP-125's record and the functions in the two
  crypto modules disagree (SEC-STD-018). The canary test from
  SEC-OPS-013 is set up here and extended by every later package that
  handles a secret.

### WP-048 Egress client

- **Wave** 2 (moved from 1 in the second security pass: its outbound TLS
  configuration comes from the secrets crate's crypto module, WP-047,
  which is wave 1; no wave 2 package depends on it) · **Size** M ·
  **Depends on** WP-005, WP-006, WP-047.
- **Owns** `crates/gunmetal-egress/` (creates the crate).
- **Serves** ACC-113, LIB-108, ADM-129; API-SET-02.
- **Security.** Boundaries TB8; threats TM-T30, TM-T32, TM-T33, TM-T36.
  Verifies SEC-EXT-001, SEC-EXT-002, SEC-EXT-003, SEC-EXT-004, SEC-EXT-005,
  SEC-API-076, SEC-API-077, SEC-API-078, SEC-API-079, SEC-TM-048,
  SEC-TM-075, SEC-NET-009, SEC-PRV-008 (the egress rules and the emitted
  denial event; the stored audit record is WP-069's), SEC-PRV-012,
  SEC-PRV-013, SEC-HIS-023, SEC-HIS-026, SEC-OPS-060, and
  SEC-PRV-034, SEC-PRV-035 and SEC-PRV-036 (the R1 absence proof: no
  scrobbling purpose). SEC-PRV-017, the provider User-Agent, moved to
  R1.1 with the providers and is proved by WP-137 through this client's
  generic header (SEC-EXT-005).
- **Scope.** The only outbound HTTP client (SEC-EXT-001, SEC-API-076). Each
  call names a purpose from a closed enumeration generated from the
  baseline's egress inventory. In R1 the enumeration holds only the
  purposes R1 uses, ACME and the update feed; each later purpose joins
  with the package that uses it (metadata providers in R1.1, WP-137;
  OIDC in R1.2, WP-096; naming and CT monitoring in R2, WP-135), one
  variant under the merge protocol. There is no scrobbling or
  external-account purpose in R1, so no R1 code can reach a scrobbling
  service (SEC-PRV-034 to SEC-PRV-036 hold by absence until WP-226 adds
  the purpose in R2). Each purpose has its default and its exact allowed
  hosts and ports (SEC-API-079, SEC-TM-075); a purpose the owner has not
  granted is refused without a connection, and the default configuration
  grants none (SEC-TM-048, SEC-PRV-013). The ACME purpose is granted when
  the owner configures built-in HTTPS for a domain they own, and it is
  the one purpose allowed before the claim (SEC-OPS-007, WP-101). The client resolves names
  itself, refuses any resolved address outside the global ranges of the
  IANA special-purpose registries unless the purpose's grant names it,
  and connects to the address it checked (SEC-EXT-002, SEC-API-077);
  follows no redirects unless the purpose allows them, and then at most
  three, each re-checked (SEC-EXT-003); allows only `https` (plain `http`
  only for an admin-granted LAN destination), verifies TLS against the
  WebPKI with hostname checks, and enforces connect and total timeouts
  and a maximum body size (SEC-EXT-004, SEC-API-078, SEC-NET-009,
  SEC-HIS-023, SEC-HIS-026); sends a generic `User-Agent` naming only the
  project, its version and contact URL, no `Referer` and no cookies
  (SEC-EXT-005); can route everything through an
  admin-configured HTTP CONNECT or SOCKS5 proxy with remote DNS, and has
  an offline mode that blocks all egress (SEC-PRV-012); emits a typed
  security event (WP-006) into its injected `SecuritySink` for every
  denial, which the server wires to the bus and the audit log's sink
  (WP-069) stores, and keeps a record of every connection for the network
  activity page (SEC-PRV-008, SEC-OPS-060). Its TLS client configuration
  comes ready-made from the secrets crate's crypto module (WP-047); this
  crate constructs no rustls provider (SEC-STD-018).
- **Not in scope.** Any caller (ACME, updates; OIDC and providers after
  R1). Storing the audit
  record (WP-069).
- **Interface sketch.** `pub struct Egress; impl Egress { pub async fn get(&self, purpose: Purpose, url: &Url) -> Result<Response, EgressError>; pub fn activity(&self) -> Vec<Connection>; }`.
- **Tests.** Against a local test server bound in the test. The resolver
  and the address policy are inputs, so a test can grant loopback to reach
  its own server for the cases that should succeed and leave it ungranted
  for the cases that should be refused; a test DNS map sends allowed names
  to private and metadata addresses (SEC-TM-048). Cases: a purpose not
  granted is refused without a connection; a name resolving to 127.0.0.1,
  169.254.169.254, an IPv4-mapped or a NAT64 form of either without a
  grant is refused; a name that resolves to a public address first and a
  private one second is refused (rebinding); a redirect to a private
  address is refused, and a fourth redirect is refused; a slow-drip
  server hits the total deadline; a body over the cap is cut; an
  untrusted certificate fails closed; the outbound request bytes for each
  purpose equal a literal expected request with no identifier of the
  server; requests through a local SOCKS5 proxy resolve names remotely;
  offline mode refuses every purpose; every attempt, refused or not,
  appears in the activity record, and every refusal emits exactly one
  denial event into a recording sink, compared as a whole value. The
  default configuration grants no purpose, checked against the
  inventory's rows, and configuring an own-domain HTTPS name grants
  exactly the ACME purpose. The purpose enum equals a literal list in the
  test (ACME and the update feed), which has no scrobbling,
  external-account, naming, CT-monitoring, OIDC or provider purpose. The
  whole-server
  network-namespace test is WP-117's.

### WP-124 Repository protections and the security process (added for the security baseline)

- **Wave** 1 · **Size** M · **Depends on** WP-008 (the xtask crate).
- **Owns** `SECURITY.md`, `.github/CODEOWNERS`, `.github/dependabot.yml`,
  `.github/pull_request_template.md`, `.github/workflows/scorecard.yml`,
  `.github/workflows/workflow-lint.yml`, `.github/workflows/codeql.yml`,
  `.github/workflows/settings-drift.yml`, `REUSE.toml`, `LICENSES/`,
  `docs/runbooks/`, `crates/xtask/src/repo.rs`,
  `crates/xtask/src/js_deps.rs`, `supply-chain/js-direct-deps.toml` (the
  one file in `supply-chain/` that is not WP-001's or the integrator's).
  Several of these exist already (SECURITY.md, CODEOWNERS, Dependabot,
  Scorecard); this package extends them. `CODEOWNERS` and `REUSE.toml`
  are registry files after this package merges (see the shared-file
  table).
- **Serves** ACC-126; the "add to the repository now" items 1 to 5, 11,
  14 and 15 of the security baseline.
- **Security.** Boundaries TB12; threats TM-T41, TM-T42, TM-T69, TM-T70.
  Verifies SEC-SUP-001, SEC-SUP-002, SEC-SUP-003, SEC-SUP-004, SEC-SUP-005,
  SEC-SUP-006, SEC-SUP-007, SEC-SUP-009, SEC-SUP-010, SEC-SUP-011,
  SEC-SUP-012, SEC-SUP-013, SEC-SUP-014, SEC-SUP-015, SEC-SUP-018,
  SEC-SUP-019, SEC-SUP-028, SEC-SUP-030, SEC-SUP-032, SEC-SUP-052,
  SEC-SUP-054, SEC-SUP-055, SEC-SUP-056, SEC-TM-003, SEC-OPS-064,
  SEC-OPS-065, SEC-OPS-066, SEC-OPS-067, SEC-OPS-069, SEC-OPS-070,
  SEC-OPS-072, SEC-HIS-058, SEC-HIS-064, SEC-HIS-065, SEC-STD-035,
  SEC-STD-036, SEC-STD-037, SEC-SUP-035.
- **Why it exists.** The baseline puts repository protections, workflow
  hardening and the disclosure process in R1, and several of them need
  code (xtask checks under the gate). No package owned them.
- **Scope.** `SECURITY.md` with scope, supported versions, response
  times, the private reporting channel, an email alias reaching two
  people, and the named security lead, release managers and incident
  lead with deputies (SEC-SUP-007, SEC-STD-036, SEC-OPS-064); CODEOWNERS
  for `.github/`, `scripts/`, `deny.toml`, `supply-chain/`, the toolchain
  file, lockfiles, release and container files, `docs/security/`,
  `docs/adr/`, the public-route allow-list and every source path that
  implements authentication, the policy layer, cryptography, parsers, the
  sandbox, the egress client and the release process (SEC-SUP-005,
  SEC-STD-035). Most of those paths are created in later waves, so this
  package lists every planned one before it exists: in the server,
  `listener.rs`, `routes.rs`, `security/`, `session/`, `verifier/`,
  `limiter/`, `limits/`, `access/`, `audit_sink/`, `audit_cli/`,
  `passkey/`, `oidc/`, `signin/`, `recovery/`, `recovery_codes/`,
  `setup/`, `users/`, `shares/`, `keys/`, `posture/`, `stream/`,
  `backup/`, `restore/`, `erasure/` and `tests/route_security/`; in the
  core, `authz/`, `token/`, `crypto.rs`, `audit_event.rs`,
  `client_context.rs`, `retention.rs`, `formats/`, `tags/`, `webauthn/`,
  `http/`, `inflate.rs` and `provider/`; `gunmetal-secrets/`,
  `gunmetal-egress/`, `gunmetal-durable/src/audit/`,
  `gunmetal-fs/src/sqlite.rs`, `gunmetal-worker/src/sandbox/` and
  `gunmetal-names/`. A package that creates a security-sensitive path not
  on the list adds one sorted line. `supply-chain/js-direct-deps.toml`,
  empty until the web client adds a direct dependency, and an xtask
  check that fails when a `package.json` in the repository lists a
  direct dependency the file does not (SEC-SUP-035); zizmor, actionlint and CodeQL on every pull request
  (SEC-SUP-010 to SEC-SUP-015, SEC-SUP-018); Scorecard with a threshold
  check (SEC-SUP-019); Dependabot with a cooldown (SEC-SUP-028); REUSE
  metadata for every file, with a `fuzz/seeds/**` annotation from the
  start so the wave 1 parser packages' binary seeds are covered without
  editing `REUSE.toml` (SEC-SUP-030); a scheduled settings-drift job
  comparing the live rulesets, tag protection, secret scanning and the
  organisation's 2FA setting with the expected state (SEC-SUP-001 to
  SEC-SUP-004, SEC-SUP-006); the pull-request template's security-fix
  checklist (regression test named after the advisory, class review,
  root cause) and a CI check that every published advisory maps to a test
  name (SEC-TM-003, SEC-OPS-066, SEC-HIS-064); runbooks for supply-chain
  incidents, signing-key compromise and owners whose server was
  compromised, with the owners' runbook commands run by a documentation
  test (SEC-SUP-054, SEC-OPS-071, SEC-OPS-072); the agent rule that coding
  agents run without release credentials (SEC-SUP-055).
- **Not in scope.** Release signing and provenance (WP-136). The docs
  lints (WP-127).
- **Tests.** The xtask checks run against recorded GitHub API responses
  and fixture files: a `SECURITY.md` missing a required section fails; a
  protected path with no code owner fails; a drifted ruleset is reported
  with the setting's name; a Scorecard result under the threshold fails;
  an advisory with no matching test fails; a fixture `package.json` with a
  direct dependency missing from `js-direct-deps.toml` fails and one with
  every dependency listed and reasoned passes. Workflow linting is proven by a
  deliberately unpinned action on a scratch branch.
- **Risks and decisions.** Several requirements here are settings and
  people, not code (two maintainers, hardware keys, an email alias); the
  package verifies them but cannot create them (baseline owner decisions
  19 and 20).
- **Added for the client plan (2026-10-03).** Two changes to `xtask js-deps`, made in
  this package if it has not merged and otherwise as one small follow-up
  in its files under the interface-change rule. It skips a dependency
  whose version starts with `workspace:` and whose name is a member of
  the pnpm workspace, so workspace packages can name each other. And a
  fixture manifest gets no skip list: every file named `package.json`
  is a manifest, so the client keeps its fixture manifests under another
  name and writes them out in a temporary directory when a test needs
  one.

### WP-127 Docs lints and requirement traceability (added for the security baseline)

- **Wave** 1 · **Size** M · **Depends on** WP-008.
- **Owns** `crates/xtask/src/docs_lint.rs`, `crates/xtask/src/trace.rs`,
  `crates/xtask/standards/` (the vendored ID lists of the pinned
  standards), `.github/workflows/docs.yml`,
  `.github/workflows/standards-watch.yml`.
- **Serves** the "add to the repository now" items 12, 13 and 18 of the
  security baseline.
- **Security.** Boundaries TB12; threats TM-T69. Verifies SEC-TM-001,
  SEC-TM-002, SEC-TM-072, SEC-TM-073, SEC-TM-074, SEC-TM-075, SEC-STD-001,
  SEC-STD-002, SEC-STD-003, SEC-STD-004, SEC-STD-006, SEC-IAM-025,
  SEC-HIS-066.
- **Why it exists.** The baseline makes every requirement a test
  (first principle 14), and the traceability check that enforces it, the
  docs lints and the check that this plan names its boundaries and
  threats had no package.
- **Scope.** The traceability check: scan every test for `Verifies:` lines
  and fail the release when a requirement due in that release has neither
  a test nor a dated review record, publishing the report (SEC-STD-004);
  each point release (R1.1, R1.2, R1.3) is a release for this check, so a
  requirement that moved with its surface to a point release blocks that
  release, not R1, and every requirement due in an earlier release must
  still pass;
  it also fails when a `SEC-HIS` incident has no rival-replay test
  (SEC-HIS-066). The docs lints: every cited ID exists and is live;
  Release values are R1, R1.1, R1.2, R1.3, R2, R3, Later or No (register
  D-10), with Withdrawn kept for withdrawn requirement rows; no live row
  cites a withdrawn one; no requirement restates a parameter or an owned control
  differently from its owner; the egress inventory and release scope are
  the single sources; every feature file and every work package in this
  plan names at least one TB and one TM-T (SEC-TM-001, SEC-TM-072 to
  SEC-TM-075, SEC-STD-001, SEC-STD-006, SEC-IAM-025's docs check). The
  "Last reviewed" check on the threat model, which the release workflow
  calls (SEC-TM-002). The standards coverage tables regenerated in CI
  (SEC-STD-002) and a monthly job that checks for new standard versions
  (SEC-STD-003).
- **Tests.** Fixture documents for each lint: a work package with no
  threat IDs, a citation of a withdrawn ID, a conflicting `SameSite`
  value, an unknown Release value, a password feature row while
  SEC-IAM-025 is live; each fails with its message. Fixture test trees
  for the traceability check: an R1 requirement with no test fails, one
  with a `Verifies:` line passes, one with a review record passes; an R1.1
  requirement with no test fails the R1.1 release and not R1.
- **Added for the client plan (2026-10-03).** The traceability check reads
  TypeScript as well as Rust: a `// Verifies:` comment on the line above
  a test, in files under `clients/`. The docs lint for SEC-TM-001 reads
  `CP-NNN` entries in `docs/plan/client-packages.md` as work packages.
  Made in this package if it has not merged, otherwise as one small
  follow-up in its files.

### WP-128 Streaming decompression helper (added for the security baseline)

- **Wave** 1 · **Size** S · **Depends on** WP-004; conditional on owner
  decision 4 admitting a pure-Rust inflate crate to the core's reviewed
  allowlist (SEC-SUP-025).
- **Owns** `crates/gunmetal-core/src/inflate.rs`,
  `crates/gunmetal-fuzz/src/inflate.rs`, `fuzz/fuzz_targets/inflate.rs`,
  `fuzz/seeds/inflate/`.
- **Serves** LIB-142 (artwork from PNG), the parse contract.
- **Security.** Boundaries TB6; threats TM-T29. Verifies SEC-MED-009,
  SEC-MED-001.
- **Why it exists.** The baseline requires every decompression to go
  through one streaming helper that stops at a cap (SEC-MED-009). The
  plan had decided on no inflate in the core and left PNG inflate to the
  `image` crate, which is a second, unbounded door.
- **Scope.** A streaming inflate (zlib and raw deflate) that stops as soon
  as output exceeds the lower of the caller's cap and the declared size,
  with a typed error, and refuses Matroska's bzlib and lzo1x when R2
  needs it. In R1 its one caller is the artwork job (WP-079), which runs
  a PNG's image data through it to bound the inflated size before
  `image` decodes; compressed ID3v2 frames stay skipped (owner decision
  20).
- **Interface sketch.** `pub fn inflate(input: &[u8], cap: u64, out: &mut BoundedBuf) -> Result<u64, InflateError>`.
- **Tests.** Zlib bombs generated inside the test (1 KiB expanding to
  1 GiB) give the typed error with peak allocation no more than 64 KiB
  over the cap; a stream that declares less than it inflates to stops at
  the declared size; a valid stream round-trips against an independent
  encoder's output written in the test; truncation at every byte gives the
  typed error. A fuzz harness and seeds.
- **Risks and decisions.** If the owner rejects an inflate dependency in
  the core, the helper moves to the worker crate with the same contract,
  which still gives one door.

### WP-139 Project site security files (split from WP-129 for the adopted R1)

- **Wave** 1 · **Size** S · **Depends on** nothing.
- **Owns** `site/.well-known/security.txt`, `site/_headers`,
  `site/privacy/` (as WP-129 first owned them).
- **Serves** the "add to the repository now" item 16 of the baseline;
  ACC-080's invitation landing on the project site.
- **Security.** Boundaries TB1, TB12; threats TM-T70. Verifies
  SEC-SUP-008, SEC-STD-016, SEC-PRV-054, SEC-PRV-055, SEC-HIS-061 (in R1:
  gunmetal.tv serves only static content, and no other project-run service
  exists without a new architecture record; the name-service clauses
  apply from R2 with WP-129).
- **Why it exists.** WP-129 held both the project's per-server name
  service and the project site's security files. The owner moved the name
  service out of R1 (register D-07), but the site and its security files
  are R1 surfaces whatever happens to the name service, so they split off
  here and WP-129 keeps only the service, in R2.
- **Scope.** `security.txt` with every required field (SEC-SUP-008); HSTS
  with preload on every domain the project operates, which in R1 is
  gunmetal.tv only (SEC-STD-016); a privacy notice for every project
  service and no third-party analytics (SEC-PRV-054); invite landing pages
  that keep the secret in the fragment and run no script that reads it
  (SEC-PRV-055; the share landing pages join in R1.2 with WP-134); the
  site serves only static content (SEC-HIS-061).
- **Not in scope.** The name service and its zone (WP-129, R2).
  Deployment and hosting accounts, which are the owner's.
- **Tests.** Site checks in CI: `security.txt` fields and an `Expires` less
  than a year ahead, HSTS headers, no third-party origin in the build, no
  dynamic endpoint in the build output, and no script reading
  `location.hash` on landing pages.
- **Risks and decisions.** None of its own; the name-service zone's HSTS
  entry is added with WP-129 in R2.

### WP-138 Retention schedule (added in the second security pass; split from WP-133)

- **Wave** 1 · **Size** S · **Depends on** WP-005.
- **Owns** `crates/gunmetal-core/src/retention.rs` (moved from WP-133).
- **Serves** ACC-118, ADM-110 (retention shown in settings); the privacy
  notice's retention section.
- **Security.** Boundaries TB10; threats TM-T18, TM-T59. Verifies
  SEC-PRV-005 (the one schedule and its core property test), SEC-TM-055.
- **Why it exists.** SEC-PRV-005 requires one retention schedule in code,
  and the first principles require one value per parameter. The schedule
  was in WP-133 (wave 4), but the audit log's pruning and address
  coarsening (WP-069, wave 2), log rotation and deletion (WP-097, wave 3)
  and backup expiry (WP-090, wave 3) all need it earlier, and each would
  have hard-coded its own values.
- **Scope.** The retention schedule as one pure table in the core with
  the baseline's defaults: security events 365 days; source addresses in
  them coarsened to /24 or /48 after 30 days and removed at 90 days;
  diagnostic logs 14 days or 100 MB; backups 14 days by default
  (SEC-PRV-041); history until the person deletes it (baseline owner
  decision 15). Overrides are validated against the floors and ceilings
  the baseline allows. The idempotent purge decision: given a data class,
  an item's age and size and `now`, keep, coarsen or remove. Each consumer
  reads its periods from this table: WP-069, WP-090, WP-097 and the sweep
  job of WP-133.
- **Not in scope.** Running any purge (WP-069, WP-090, WP-097, WP-133).
- **Interface sketch.** `pub enum DataClassRetention { SecurityEvent, EventAddress, DiagnosticLog, Backup, History, /* … */ }`;
  `pub fn decide(class: DataClassRetention, age: Duration, size: u64, s: &Schedule) -> Purge` where `Purge` is `Keep`, `Coarsen` or `Remove`;
  `pub const DEFAULT: Schedule`.
- **Tests.** The default table against a literal copy of the baseline's
  values. Properties: purging twice equals purging once; nothing younger
  than its limit is removed or coarsened; an address is coarsened before
  it is removed, never the reverse; an override outside the allowed
  range is refused with its name.

## Wave 2: tag mapping, probing, search and the server's spine

Four packages numbered in this section run in wave 3 because of
same-wave dependencies found in review: WP-063 (needs the credential
verifier, WP-064) and WP-072, WP-073 and WP-074 (they register routes, so
they need WP-118). They stay here so their numbers keep their place; their
own entries give the wave. WP-059 (Home rows) is back in this wave: it
had moved to wave 3 only because it needed the neighbour table (WP-058),
and WP-058 and the rule language (WP-027) left R1 (WP-058 for R1.3, and
WP-027 for R1.1 under register D-85). WP-057 (import
parsers) and WP-071 (the derived-data store) left R1 for R1.1, and WP-058
for R1.3; their specifications are in
[After R1](#after-r1-point-releases-and-later). WP-118 and WP-119, added
in review, and WP-130, added for the security baseline, are at the end of
this section. WP-048 (the egress client), numbered in the wave 1 section,
also runs in this wave.

### WP-049 ID3 tag mapping

- **Wave** 2 · **Size** M · **Depends on** WP-010, WP-011, WP-040.
- **Owns** `crates/gunmetal-core/src/tags/id3.rs` (`tags/mod.rs` is a
  registry file shared with WP-050 and WP-051).
- **Serves** MUS-001 to MUS-004, MUS-011, MUS-012, MUS-017, MUS-020,
  MUS-034, MUS-036, MUS-084, LIB-059; API-CAT-01 to API-CAT-03. Every
  frame is mapped, including roles, release type, original date, moods,
  labels and the explicit flag, because R1 keeps every tag (LIB-059); the
  R1.1 features that show them (MUS-005, MUS-010, MUS-013, MUS-019,
  MUS-047) need no further mapping.
- **Security.** Boundaries TB6, TB9; threats TM-T07, TM-T20. Verifies
  SEC-MED-006, SEC-MED-014.
- **Scope.** Map ID3v2 frames and ID3v1 fields to `TrackTags`, with a
  recorded source for every field (API-CAT-08). Covers title, artists
  (`TPE1` multi-value), album artist (`TPE2`), album, track and disc with
  totals, set subtitle (`TSST`), dates (`TDRC`, `TDOR`, v2.3 `TYER` and
  `TDAT`), genres (including the v1 genre byte and `(nn)` references),
  composer, conductor (`TPE3`), lyricist (`TEXT`), remixer (`TPE4`),
  involved people (`TIPL`, `TMCL`), sort names (`TSOP`, `TSO2`, `TSOA`),
  compilation (`TCMP`), grouping (`GRP1` and `TIT1`), mood, label, ISRC,
  MusicBrainz IDs from `UFID` and `TXXX`, ReplayGain from `TXXX`, lyrics
  sources from `USLT` and `SYLT`. ID3v1 fills only fields v2 left empty.
- **Not in scope.** Splitting artist strings (WP-053). Deciding albums
  (WP-076).
- **Interface sketch.** `pub fn from_id3(v2: Option<&Id3v2Tag>, v1: Option<&Id3v1Tag>, limits: &Limits) -> Mapped`
  where `Mapped { tags: TrackTags, sources: FieldSources, problems: Vec<TagProblem> }`.
- **Tests.** One per field and per ID3 version; v2.3 slash-separated
  artists left as one string for the splitter; `TXXX:MusicBrainz Album Id`
  with odd casing; a ReplayGain value outside the range dropped with a
  reason; v1 and v2 disagreeing (v2 wins); 4,097 genre values (limit).

### WP-050 Vorbis comment tag mapping

- **Wave** 2 · **Size** M · **Depends on** WP-014, WP-040.
- **Owns** `crates/gunmetal-core/src/tags/vorbis.rs`.
- **Serves** as WP-049, for FLAC, Ogg Vorbis and Opus; MUS-085 (R128
  tags).
- **Security.** Boundaries TB6, TB9; threats TM-T07, TM-T20. Verifies
  SEC-MED-006, SEC-MED-014.
- **Scope.** The common keys and their variants (`ARTIST` repeated,
  `ARTISTS`, `ALBUMARTIST` and `ALBUM ARTIST`, `TRACKNUMBER` with or
  without a total, `TOTALTRACKS` and `TRACKTOTAL`, `DISCNUMBER`,
  `DISCSUBTITLE`, `DATE`, `ORIGINALDATE`, `RELEASETYPE`, `COMPILATION`,
  `MUSICBRAINZ_*`, `REPLAYGAIN_*`, `R128_TRACK_GAIN` and
  `R128_ALBUM_GAIN` in Q7.8, `LYRICS` and `UNSYNCEDLYRICS`, `PERFORMER`
  with a role in brackets).
- **Tests.** Every key variant; the same key in two cases; a total in both
  `TRACKNUMBER` ("3/12") and `TRACKTOTAL` that disagree (recorded); R128
  values at the Q7.8 limits.

### WP-051 MP4, APE and RIFF tag mapping

- **Wave** 2 · **Size** M · **Depends on** WP-011, WP-017, WP-020, WP-040.
- **Owns** `crates/gunmetal-core/src/tags/mp4.rs`, `crates/gunmetal-core/src/tags/ape.rs`,
  `crates/gunmetal-core/src/tags/riff.rs`.
- **Serves** as WP-049, for MP4, APE and WAV or AIFF files; MUS-069 (via
  `iTunSMPB`).
- **Security.** Boundaries TB6, TB9; threats TM-T07, TM-T20. Verifies
  SEC-MED-006, SEC-MED-014.
- **Scope.** MP4 atoms (`©nam`, `©ART`, `aART`, `©alb`, `trkn`, `disk`,
  `©day`, `©gen` and `gnre`, `cpil`, `rtng` for explicit, `soar` and the
  other sort atoms, `©wrt`, freeform MusicBrainz and ReplayGain items,
  `iTunSMPB` parsed into trim); APEv2 keys; RIFF `INFO` keys. Embedded
  `id3 ` and `ID3 ` chunks in WAV and AIFF are not mapped here: the probe
  (WP-052) parses them with WP-010 like any ID3v2 tag, and WP-075 maps them
  with WP-049's mapper. As first written, this package called WP-049's
  mapper, which is in the same wave.
- **Tests.** `trkn` with a zero total; `rtng` values 0, 1, 2 and 4;
  `iTunSMPB` with the usual spacing and with extra fields; an APE key that
  differs only in case; an `INFO` chunk with an odd-length value.

### WP-052 File probe

- **Wave** 2 · **Size** L · **Depends on** WP-009 to WP-020, WP-040.
- **Owns** `crates/gunmetal-core/src/probe/`.
- **Serves** MUS-021, MUS-032, MUS-069, MUS-071, LIB-019, LIB-028 (identity
  inputs), LIB-059, LIB-193; API-CAT-04, API-CAT-05, API-CAT-11.
- **Security.** Boundaries TB6, TB9; threats TM-T20, TM-T29. Verifies
  SEC-MED-001, SEC-MED-005, SEC-MED-006, SEC-MED-007, SEC-MED-008,
  SEC-TM-032, SEC-HIS-036, SEC-MED-010, SEC-MED-017.
- **Scope.** One sans-I/O entry point that a worker drives for any file:
  detect the format, run the container parser, gather the raw tag blocks
  (all of them, in a defined precedence order), technical facts, trim
  values, the seek or frame index, artwork references, lyrics sources, the
  identity inputs (FLAC MD5 when set, and the byte range of the audio
  window that excludes ID3v2, APE and ID3v1 regions), the parser version
  per format, and per-part problems. A failure in an optional part keeps
  the rest; only a failure in what playability needs fails the file.
- **Not in scope.** Mapping tags (the mappers run in WP-075 after the
  worker returns raw blocks, so the probe stays independent of the
  mapping rules and the worker's message stays small). Hashing the audio
  window (the worker hashes the bytes it is told to; see WP-079).
- **Interface sketch.** `pub fn probe(ext_hint: Option<&str>, limits: Limits) -> Probe` where
  `Probe: SansIo<Output = Result<FileFacts, ProbeError>>`;
  `pub const PARSER_VERSIONS: &[(Format, u16)]`.
- **Tests.** For each format, a complete synthetic file through `drive`,
  compared with a whole expected `FileFacts` written literally. A FLAC with
  an ID3v2 tag in front; an MP3 with ID3v2, APE and ID3v1 (the identity
  window excludes all three); an MP4 with `moov` at the end (the probe asks
  for it); a file whose artwork is damaged (facts kept, problem recorded);
  a file whose audio header is damaged (fails with the offset). Property:
  bytes requested never exceed the per-file cap; every request is inside
  the file. Bytes read per file are reported and asserted for each format,
  because LIB-019 promises header-only reads.

### WP-053 Artist credits and splitting

- **Wave** 2 · **Size** M · **Depends on** WP-034, WP-036, WP-040.
- **Owns** `crates/gunmetal-core/src/music/credits.rs` (`music/mod.rs` is a
  registry file).
- **Serves** MUS-001 to MUS-004, MUS-006, MUS-035; API-CAT-01, API-LIB-07.
  (Roles shown as roles, MUS-005, and one artist page across libraries,
  LIB-187, are R1.1, WP-146.)
- **Security.** Boundaries TB9; threats TM-T20. Verifies SEC-MED-006.
- **Scope.** Turn tagged artist strings and multi-value fields into linked
  credits while keeping the display credit exactly as tagged: separator
  rules (", ", " & ", " feat. ", " ft. ", " x ", " / ", "; ") with
  per-library exceptions (an artist whose name contains a separator, such
  as a duo with an ampersand in its name), multi-value fields taking
  precedence over splitting, MusicBrainz artist IDs pairing with names by
  position, and same-name artists kept apart when their IDs differ.
  Artist merge and alias overrides from the curation log apply on top only
  from R1.3, when WP-107 adds the curation bodies and the overrides.
- **Interface sketch.** `pub fn credits(tags: &TrackTags, rules: &SplitRules) -> CreditSet`;
  `pub struct SplitRules { pub separators: Vec<String>, pub exceptions: Vec<String> }`.
- **Tests.** "Simon & Garfunkel" with and without the exception; "A feat.
  B"; three artists with three IDs; three names with two IDs (no pairing;
  recorded); the same name with two different IDs; a credit of only
  separators. Property: joining the credited names never loses a
  character of the display credit except separators.

### WP-054 Search index

- **Wave** 2 · **Size** L · **Depends on** WP-036, WP-040.
- **Owns** `crates/gunmetal-core/src/search/`.
- **Serves** DIS-083 to DIS-085, MUS-061; API-HOME-05, API-SYNC-07 (the
  fallback path, if needed). (Mood search, role filters, scoping to one
  library and finding inside a list, DIS-086 to DIS-088 and DIS-091, are
  R1.1, WP-147; recent searches, DIS-089, are R1.1 and client only.)
- **Security.** Boundaries TB4; threats TM-T09. Verifies SEC-API-063,
  SEC-STD-011 (the search part: a query is matched by folded tokens and
  prefixes, never compiled into a regular expression, under the length
  and term caps).
- **Scope.** An in-memory index built on the device from the synced
  library: folded prefix and token matching, a small typo tolerance,
  the fields MUS-061 names (title, artist, album, credits, genre, label),
  results grouped by type (artists, albums, tracks, playlists), ranking that
  prefers exact and prefix matches and the person's own plays, and a
  compact serialised form so the server can ship a prebuilt segment if the
  device misses its budget.
- **Not in scope.** Recent searches, which stay on the device (client).
  The mood field, role filters and library scope (WP-147, R1.1).
- **Interface sketch.** `pub struct Index; impl Index { pub fn build(items: impl Iterator<Item = SearchDoc>) -> Self; pub fn query(&self, q: &str, kind: KindFilter, limit: u16) -> Vec<Hit>; pub fn to_bytes(&self) -> Vec<u8>; pub fn from_bytes(b: &[u8]) -> Result<Self, IndexError>; }`.
- **Tests.** Diacritics, case, a missing letter, a swapped pair, a query of
  one character, a query of 256 characters and 17 terms (capped), a type
  filter, a composer found through the credits field, and a query full of
  regular-expression metacharacters matched literally. Each example asserts the exact ordered hit list
  for a small hand-built index. Properties: a document whose title is
  unique in the index is the first hit for a query of that exact title
  with a limit of 1 (a bare "is found" would pass for an index that
  returned everything); adding documents never removes an existing exact
  hit; the serialised form round-trips. Build time, memory and query time
  at 100,000 synthetic tracks are held to the DIS-019 budget by WP-115's
  budget tests in the R1 gate (register D-87), not by this package's own
  tests; until the owner names the reference device (owner decision 15),
  those tests run on the reference low-end profile.

### WP-055 Playback decision engine

- **Wave** 2 · **Size** S · **Depends on** WP-028, WP-040.
- **Owns** `crates/gunmetal-core/src/decision.rs`.
- **Serves** MUS-099, MUS-229, MUS-236 (the R1 track details view,
  register D-83); API-CAT-09, API-SES-01. (The same reasons shown to
  admins per session, ADM-100 and INT-134, are R1.2, WP-153. The full
  track info sheet, MUS-114, is R1.1 and adds its fields to the same
  summary.)
- **Security.** Boundaries TB4; threats TM-T15. Verifies no requirement of
  its own: it holds no security control, and the rules it relies on are
  proved by the packages that own them. The details summary is built only
  from the synced track record, which carries no file path and nothing
  the profile may not see (SEC-API-068, proved by WP-044 and WP-131;
  SEC-CLI-020, proved by WP-084).
- **Scope.** For music in R1: given a track's technical facts and a device
  capability report, return play directly, play through the packager, or
  cannot play here, with a structured reason list the badge and the admin
  session view both render. For the minimal, read-only track details view
  (MUS-236): a `TrackDetails` summary holding the title, the credit as
  tagged with each credited artist, the album, the format with its
  technical facts, this decision with its reasons in the badge's words,
  whether the track joins the next one without a gap (its trim is known
  and the chosen path keeps it), and the gain source from WP-028's
  decision (track tags, album tags, or "estimated").
- **Not in scope.** Video, remux and transcode decisions (R2). The track
  info sheet's own fields: tags as read, provenance, MusicBrainz IDs and
  the gain applied (MUS-114, MUS-090, R1.1).
- **Interface sketch.** `pub fn decide_audio(t: &TechInfo, d: &DeviceCaps) -> Decision`;
  `pub enum Decision { Direct, Packaged(PackageFormat), CannotPlay(Vec<Reason>) }`;
  `pub fn track_details(t: &SyncedTrack, d: &Decision, g: &GainDecision) -> TrackDetails`.
- **Tests.** Each core format against a capability report that supports it,
  lacks it, supports the codec but not the container, and supports it only
  in Media Source Extensions; ALAC in a browser that cannot decode it
  ("Cannot play here: this browser cannot decode ALAC"). The details
  summary for a FLAC played directly, an MP3 with LAME delay and padding
  through the packager, an Opus file with pre-skip, a track with no gain
  tags ("estimated") and that ALAC file, each against a literal expected
  summary; the summary's field list, checked against a literal list, has
  no path.

### WP-056 Audio packager (conditional on ADR 4)

- **Wave** 2 · **Size** L · **Depends on** WP-003 (ADR 4 accepted), WP-012,
  WP-013, WP-015, WP-016, WP-017, WP-019.
- **Owns** `crates/gunmetal-core/src/package/`,
  `crates/gunmetal-testkit/src/fmp4.rs`.
- **Serves** MUS-230, MUS-067, MUS-069, MUS-070; API-STR-04.
- **Security.** Boundaries TB6; threats TM-T20, TM-T21. Verifies
  SEC-MED-001, SEC-MED-007 (the packager takes the step budget and
  stops with a typed error when it runs out), and, together with WP-105,
  SEC-MED-018, SEC-MED-023 and SEC-MED-024: this code is a sans-I/O core
  function that is linked into the worker and called only there, never
  from the server crate. WP-001's dependency check fails if
  `gunmetal-server` calls `package::media_segment` or `package::init_segment`
  directly.
- **Scope.** Write an initialisation segment and media segments of
  fragmented MP4 audio from frame indexes, copying FLAC frames (`dfLa`),
  Opus packets (`dOps`, keeping pre-skip) and MP3 frames, with edit lists
  or equivalent trim signalling so gapless survives. Segment boundaries
  come from the index, so any segment can be produced independently.
- **Not in scope.** The worker job and serving (WP-105). Video (R2).
- **Interface sketch.** `pub fn init_segment(track: &PackTrack) -> Vec<u8>`;
  `pub fn media_segment(track: &PackTrack, index: &FrameIndex, n: u32, source: &[u8], budget: &mut StepBudget) -> Result<Vec<u8>, PackError>`
  where `source` is the byte range the worker read for that segment from
  the descriptor it was passed.
- **Tests.** Round trip: every segment this module writes is read back and
  the frames copied in are recovered byte for byte (SEC-MED-032's rule for
  every writer). WP-017's walker reads the box structure and the
  initialisation segment's sample entry, but it does not read fragments
  (`moof`, `traf`, `tfhd`, `tfdt`, `trun`), which it lists as out of scope,
  so as first written this test could not run. This package therefore adds
  a small fragment reader to the testkit, written from ISO/IEC 14496-12
  and sharing no code with the writer, and checks that reader against
  literal fragment bytes written out by hand. MP3 in MP4 uses an `mp4a`
  entry whose object type marks MPEG-1 audio; the test confirms WP-017's
  `esds` reading accepts it. Segment boundaries at the first
  and last frame; a one-frame file; trim values carried. Per-browser
  acceptance cannot be tested here; it is part of the client's gapless
  fixtures (player.md, "How the player is tested").
- **Risks.** Which containers each browser's Media Source Extensions accept,
  including Safari's ManagedMediaSource, is unverified; this package may
  turn out to need fewer codecs than planned.
- **Added for the client plan (2026-10-03).** An xtask command, in this package's
  own xtask file, runs the packager over generated tones and writes
  fragmented MP4 fixtures with a SHA-256 manifest, for the web player's
  Media Source path (CP-034).

### WP-059 Home rows

- **Wave** 2 (moved back from 3: it had moved only because "because you
  played" read the neighbour table, WP-058, and both that row and
  rule-backed custom rows, which needed the rule language, WP-027, left R1)
  · **Size** S (was M) · **Depends on** WP-034, WP-040.
- **Owns** `crates/gunmetal-core/src/home/`.
- **Serves** DIS-001, DIS-004, DIS-020, DIS-021, DIS-035, DIS-036, DIS-038,
  DIS-046, MUS-050, MUS-059, MUS-149 (loved tracks, read from the person's
  loves); API-HOME-03, API-HOME-04, API-LOG-04. (Dismissing from Continue
  rows, DIS-022 and DIS-023, and your top tracks by an artist, DIS-071, are
  R1.1, WP-141 and WP-147; an arrangeable Home, DIS-003, DIS-009, DIS-012,
  DIS-015 and MUS-049, is R1.2, WP-154; "because you played", DIS-061, is
  R1.3, WP-058; rule-backed custom rows built in the rule editor are R1.3,
  WP-092, on the rule format WP-027 delivers in R1.1.)
- **Security.** Boundaries TB4; threats TM-T15, TM-T18. Verifies no
  requirement of its own: it holds no security control, and the rules it
  relies on are proved by the packages that own them.
- **Scope.** The default Home layout and the built-in R1 row sources
  (continue listening by album or playlist, recently played, recently
  added grouped by album and ignoring upgrades, loved songs), each
  evaluated on the device from the synced library and the person's
  events, each with a reason, and the designed empty states.
- **Not in scope.** Dismissals and the Hidden page (WP-141, R1.1); top
  tracks by an artist (WP-147, R1.1); layouts, pins and row settings
  (WP-154, R1.2); "because you played" (WP-058, R1.3); rule-backed rows
  (R1.3).
- **Interface sketch.** `pub fn evaluate_row(row: &RowSpec, lib: &LibraryView, mine: &MyEvents, now: Timestamp) -> Row`.
- **Tests.** An upgrade (same identity, new file) does not appear in
  recently added; loved songs list exactly the loved tracks in love
  order, with a removed love gone; empty library rows give the designed
  empty state value. Property: row evaluation is deterministic for a
  given input and `now`.

### WP-060 Filesystem roots, opening and walking

- **Wave** 2 · **Size** L · **Depends on** WP-024, WP-005, WP-126 (which
  creates the crate).
- **Owns** `crates/gunmetal-fs/src/root.rs`, `crates/gunmetal-fs/src/open.rs`, `crates/gunmetal-fs/src/walk.rs`, `crates/gunmetal-fs/src/fingerprint.rs`,
  `crates/gunmetal-fs/src/pool.rs` (the crate's `Cargo.toml` and `lib.rs`
  are WP-126's; this package adds dependency and module lines).
- **Serves** LIB-007, LIB-016, ADM-089; API-LIB-02, API-LIB-03.
  (Exclusion rules, LIB-006, are R1.1: WP-140 adds them to the walk.)
- **Security.** Boundaries TB9; threats TM-T22, TM-T25, TM-T56. Verifies
  SEC-MED-033, SEC-MED-034, SEC-MED-035, SEC-MED-036, SEC-MED-038,
  SEC-MED-040, SEC-MED-041, SEC-TM-042, SEC-TM-043, SEC-HIS-016,
  SEC-OPS-054, SEC-OPS-055, SEC-API-018.
- **Scope.** Root handles opened once per configured root; opening beneath
  a root with `O_NONBLOCK`, `O_NOCTTY` and `O_CLOEXEC`; checking the file
  type on the open handle; the symlink policy; walking with raw-byte
  names (exclusions join the walk in R1.1, WP-140); fingerprints (size, modification time, file ID or inode,
  a short hash of the first and last few kilobytes) and directory
  summaries for no-change short-circuits; the identity check before
  serving bytes (SEC-MED-036); a bounded blocking pool per root that
  pauses a root after two lost workers. Roots are opened read-only, and
  nothing in this crate can create, modify, rename or delete inside a
  library root: the open options it exposes are read-only by type
  (SEC-MED-038, SEC-TM-042). Links are refused by default and followed
  only when the whole chain stays beneath the same root or an approved
  extra root (SEC-MED-034, SEC-TM-043).
- **Not in scope.** Watching (WP-098). Any database.
- **Interface sketch.** `pub struct Root; impl Root { pub fn open(path: &Path, policy: LinkPolicy) -> Result<Self, FsError>; pub fn open_file(&self, rel: &RelPath) -> Result<MediaFile, FsError>; pub fn walk(&self) -> Walk; }`;
  `pub fn fingerprint(f: &MediaFile) -> Result<Fingerprint, FsError>`.
- **Tests (real filesystem in a temporary directory).** A FIFO named
  `track.flac` and a socket named `cover.jpg` are skipped without blocking;
  a symlink inside the root followed, one to another library refused and
  listed, one to `/etc/passwd` refused; a hard link to a file outside the
  root (refused by the identity rules, listed); non-UTF-8 names and names
  with newlines; a directory that becomes unreadable mid-walk; a file
  swapped after fingerprinting is caught by the serve check; a symlink
  into the data directory's secrets and backups is refused; a write-capable
  open cannot be requested (compile-fail); a hung root (a FUSE mount that
  never answers, where CI allows it) stalls only that root's pool. The
  disallowed-methods lint keeps `std::fs` out of every other crate.
- **Risks.** Windows and macOS behaviour of `cap-std` beneath-root opens
  needs its own tests when those server builds are planned (owner
  decision 18).

### WP-061 Worker IPC and host loop

- **Wave** 2 · **Size** M · **Depends on** WP-004, WP-039, WP-045.
- **Owns** `crates/gunmetal-worker/src/ipc.rs`, `crates/gunmetal-worker/src/host.rs`.
- **Serves** the worker protocol of media-and-parser-safety.md, section 4.
- **Security.** Boundaries TB6; threats TM-T20, TM-T21, TM-T24. Verifies
  SEC-MED-010, SEC-MED-018, SEC-MED-020, SEC-MED-023, SEC-STD-040.
- **Scope.** The request and response messages between server and worker as
  core wire frames over the socket pair; passing the media file and
  resolved sidecars as descriptors with `SCM_RIGHTS`; the worker's host
  loop that drives any sans-I/O parser by answering read requests with
  `pread`, enforcing the 16 MiB request and per-file caps; the server side
  that revalidates every response.
- **Not in scope.** Which jobs exist (WP-079), supervision (WP-078).
- **Jobs beyond probing.** Every hostile input the server receives is
  parsed here, not in the server process (principle 2, SEC-MED-018), so
  the enum also carries the packaging job (`Package`, WP-105) in R1, and
  after R1 the import-file job (`ImportFile`, WP-145, R1.1) and the
  playlist-file job (`PlaylistFile`, WP-112, R1.1). This package defines
  the enum with the three scan jobs; each of those packages adds its
  variant in its own wave, one line under the merge protocol, and owns
  its job file. The CUE slice job
  (`CueSlice`) joins in R2 with MUS-041 (WP-213). A job's output is typed
  rows or bytes that the server revalidates (SEC-MED-023); a streamed job
  (packaging) returns its output in frames over the same socket pair, each
  under the 32 MiB cap.
- **Interface sketch.** `pub enum Job { Probe { hint: Option<String> }, Artwork { sizes: Vec<Size> }, HashWindow { range: Range<u64> }, Package { track: PackTrack, segment: SegmentRequest } /* R1.1 adds ImportFile and PlaylistFile */ }`;
  `pub fn serve_one<P: SansIo>(fd: BorrowedFd<'_>, parser: P, caps: &ReadCaps) -> Result<P::Output, HostError>`.
- **Tests.** A parser that asks for 17 MiB is refused; one that asks past the
  end is refused; a response frame over 32 MiB from a fake worker is
  refused by the server; a worker sent a closed descriptor reports it;
  messages round-trip. Integration: a real worker process drives a small
  sans-I/O test parser (one that asks for two windows of a file and
  returns their bytes) over a file passed by descriptor, and the bytes
  returned equal the file's. The real probe is WP-052, in this same wave,
  so the probe-through-a-worker integration belongs to WP-079.

### WP-062 Sessions and revocation epochs

- **Wave** 2 · **Size** L (was M; it gained the fresh-uv check) ·
  **Depends on** WP-006, WP-031, WP-033, WP-043, WP-044, WP-046, WP-047.
- **Owns** `crates/gunmetal-server/src/session/`.
- **Serves** ACC-124, ACC-079, ACC-065, ACC-068, ACC-069, ACC-070, ACC-008;
  API-AUTH-08, API-AUTH-09, API-DEV-01.
- **Security.** Boundaries TB1, TB4, TB5; threats TM-T16, TM-T38, TM-T40,
  TM-T13. Verifies SEC-IAM-002, SEC-IAM-037, SEC-IAM-038, SEC-IAM-041,
  SEC-IAM-043, SEC-IAM-076, SEC-IAM-107, SEC-API-017, SEC-API-032,
  SEC-OPS-016, SEC-HIS-044, SEC-TM-017 (the enforcement on every fresh-uv
  route), SEC-TM-028, SEC-TM-058 and SEC-CLI-010 (the server part: the
  cookie attributes and the shared-browser idle timeout; the browser
  storage rules are the client's), SEC-CLI-024, SEC-EXT-007, SEC-STD-014,
  SEC-PRV-003 (the session-record clause), and the "from the OS CSPRNG"
  part of SEC-HIS-012 for device IDs, the first public IDs the server
  mints.
- **Scope.** Browser sessions in one cookie, `__Host-gm_session` with
  `Secure`, `HttpOnly`, `SameSite=Lax`, `Path=/` and no `Domain`
  (SEC-API-032), ending after 7 days without use or 30 days in total; a
  shared-browser session uses a cookie with no `Max-Age` or `Expires` and
  ends after 30 minutes idle (SEC-CLI-010). A separate admin session in
  its own `__Host-` cookie with `SameSite=Strict`, created only by a
  user-verifying sign-in, ending after 15 minutes without an admin action
  or 1 hour in total; a media or ordinary session never authorises an
  admin route (SEC-IAM-041). Fresh user verification: each session
  records the time of its last user-verified assertion and the kind of
  credential that made it, set only through
  `record_user_verification`, which the passkey ceremony (WP-081), the
  step-up route (WP-106) and owner creation at setup call after checking
  the UV flag; this package provides the pipeline check for WP-044's
  `RouteTag::FreshUv`, which refuses the route, with no state change,
  when that assertion is older than 5 minutes or was made by an OIDC
  sign-in (SEC-IAM-041, SEC-IAM-107, SEC-TM-017). As first written this
  check was built only in WP-106 (wave 4), so the fresh-uv routes that
  wave 3 packages register (backup download, ownership transfer, adding a
  library root, leaving the home posture, trusted proxies, OIDC linking)
  would have had nothing enforcing the tag and their refusal tests could
  not pass. Wave 3 packages test fresh-uv with a test enroller that calls
  `record_user_verification` with a chosen time and credential kind.
  Session tokens are opaque 256-bit values
  from the CSPRNG, stored only as keyed hashes through the secrets
  crate's `KeyRing::mac`, and checked by lookup on every request; a new
  token is issued at sign-in and at elevation and the previous one stops
  working (SEC-IAM-037, SEC-IAM-038, SEC-OPS-016). A per-account and
  per-device epoch that every session and signed URL carries; bumping it
  on a credential added or removed, sign out everywhere, a disabled
  account and a revoked device; an in-memory revocation cache invalidated
  by the write that changes it, so revocation bites on the next request
  (SEC-IAM-043, SEC-API-017, SEC-TM-028). The session table holds the
  client address only as the `ClientContext` the listener resolved
  (WP-118), the only address input this package accepts, and removes it
  from the record when the session ends (SEC-PRV-003, SEC-OPS-037). The
  devices table (a schema part in the identity store), with each enrolled
  device's class, personal or limited, which decides what it may do
  (SEC-CLI-024), and device IDs minted through WP-047's minting function
  (SEC-HIS-012); every credential
  carries an immutable kind, and each listener accepts only its own kinds
  (SEC-EXT-007). The principal extractor that plugs into WP-044's pipeline
  and yields the core's `authz::Principal` (WP-033), resolving every
  request to exactly one principal of a closed set (SEC-IAM-002).
- **Interface sketch.** `pub async fn issue(store: &IdentityStore, account: AccountId, device: DeviceInfo, kind: Lifetime, from: &ClientContext) -> Result<(Cookie, Principal), SessionError>`;
  `pub async fn authenticate(req: &RequestParts) -> Result<Principal, ApiError>`;
  `pub async fn record_user_verification(session: SessionHandle, kind: CredentialKind, at: Timestamp) -> Result<(), SessionError>`;
  `pub fn fresh_uv_hook(clock: &dyn Clock) -> Hook`;
  `pub async fn bump_epoch(scope: EpochScope) -> Result<(), SessionError>`.
- **Tests (real SQLite).** A session ends at 7 days idle and at 30 days
  total, and an admin session at 15 minutes idle and 1 hour total, each on
  a manual clock; a shared-browser session ends after 30 minutes idle; an
  ordinary session cookie is refused on an admin route; a bumped epoch
  fails the next request immediately; revoking one device leaves the
  others; the exact `Set-Cookie` strings for both cookies, compared
  literally, and no cookie outside the inventory is ever set
  (SEC-STD-014); a token in the database is never the raw token (the
  canary test); a request presenting two credentials, or a credential of a
  kind the listener does not accept, is refused (property over every
  combination of presented credentials). Fresh-uv, over a stand-in
  fresh-uv route on a manual clock: refused with no user verification,
  allowed at 4 minutes 59 seconds after a passkey assertion and refused
  at 5 minutes 1 second; refused at any age when the assertion came from
  OIDC; a property over credential kinds, assertion ages and route tags
  that only a passkey or device-key assertion under 5 minutes passes a
  fresh-uv route, with every refusal leaving the stand-in route's state
  unchanged (SEC-IAM-107, SEC-TM-017). A sentinel client address is gone
  from a raw-byte scan of the identity database and its WAL after the
  session ends (SEC-PRV-003).

### WP-063 Recovery codes (was: passwords, two-factor and recovery codes)

- **Wave** 3 (moved from 2, because it is a pathway of the credential
  verifier, WP-064, which is wave 2) · **Size** S (was M) · **Depends
  on** WP-038, WP-043, WP-046, WP-047, WP-064. WP-080 no longer depends
  on it: recovery codes are offered at the owner's first passkey
  enrolment, which WP-106 adds to setup in wave 4.
- **Owns** `crates/gunmetal-server/src/recovery_codes/` (the first draft
  owned `password/`, which is no longer created).
- **Serves** ACC-064; API-USR-03 (storage side).
- **Security.** Boundaries TB1, TB4; threats TM-T04, TM-T63. Verifies
  SEC-IAM-089, SEC-STD-029.
- **Why it changed.** The security baseline has no account passwords and
  no TOTP (SEC-IAM-025; baseline owner decision 1), so ACC-052 and ACC-053
  are withdrawn from R1 and this package keeps only recovery codes.
- **Scope.** Ten single-use recovery codes of 80 bits each, offered to
  owners and administrators at their first enrolment and available to
  everyone under Account > Recovery; stored only as peppered hashes
  through `KeyRing::mac`; consumed by one conditional update in one
  transaction, so two concurrent redemptions cannot both succeed
  (SEC-STD-029); usable only to enrol a new credential, which starts the
  recovery hold (WP-106); every use goes through the credential verifier
  (WP-064) and its limiter (SEC-IAM-089).
- **Interface sketch.** `pub async fn issue(account: AccountId) -> Result<[RecoveryCode; 10], RecoveryError>`;
  `pub async fn redeem(code: &RecoveryCode) -> Result<EnrolOnly, SignInError>`.
- **Tests (real SQLite).** A used code fails the second time; 64
  concurrent redemptions of one code give exactly one success; a recovery
  session can do nothing but enrol a credential (every other route
  refused); the stored hash never contains the code (canary); a wrong
  code, a used code and an unknown account give byte-identical responses.
- **Not in scope.** The enrolment flow and the hold (WP-106).

### WP-064 Credential verifier and guessing limiter (was: guessing limiter)

- **Wave** 2 · **Size** M (was S) · **Depends on** WP-006, WP-032, WP-033,
  WP-043, WP-046.
- **Owns** `crates/gunmetal-server/src/limiter/`,
  `crates/gunmetal-server/src/verifier/`; its entries in the
  `limits.toml` registry.
- **Serves** ACC-063, ADM-122, INT-012; API-AUTH-07.
- **Security.** Boundaries TB1, TB2, TB4; threats TM-T04, TM-T05, TM-T10,
  TM-T63. Verifies SEC-HIS-046, SEC-HIS-047, SEC-TM-014 (for SEC-HIS-046
  and SEC-TM-014, the one verifier and the emitted events; the stored
  audit records are WP-069's and WP-117's), SEC-API-056, SEC-API-058,
  SEC-IAM-008, SEC-IAM-022, SEC-IAM-069 (the denial and the emitted
  event), SEC-IAM-099, SEC-IAM-101, SEC-OPS-028, SEC-EXT-007,
  SEC-HIS-004 (every sign-in pathway refuses an empty or missing
  credential), SEC-TM-024 and SEC-API-010 (the pre-authentication
  handle is the only pre-principal credential lookup).
- **Why it changed.** The baseline requires one credential verifier that
  every authentication pathway goes through, with the same rate limits,
  the same unknown-account handling and the same audit logging
  (SEC-HIS-046, SEC-TM-014). As first written, each pathway (passwords,
  passkeys, codes, sessions) verified on its own, with a limiter beside
  them.
- **Scope.** The pathway inventory as one closed enum: in R1, passkey,
  paired browser, claim code, recovery code, admin recovery link,
  invitation and pairing code; the share-link password (WP-134) and OIDC
  (WP-096) join in R1.2, and device keys, API keys and adapter credentials
  in R2, each as one variant added with the package that brings the
  pathway, so the R1 inventory lists only pathways R1 has. The verifier takes a pathway and its
  presented secret, applies the per-account and per-source limits before
  checking, delegates the check to the pathway's implementation (each
  pathway's package implements one trait), runs the same work whether
  the account or code exists, is disabled or is wrong, and returns one
  indistinguishable failure (SEC-API-058, SEC-IAM-022, SEC-HIS-047). An
  empty or missing credential is refused on every pathway before the
  pathway's check runs (SEC-HIS-004). A pathway reads credentials before
  a principal exists only through the pre-authentication handle the
  verifier passes into its check, which wraps the identity store's
  `PrePrincipal` credential lookup (WP-046); no pathway module calls that
  lookup itself (SEC-TM-024, SEC-API-010). Any error, timeout or missing
  data ends in denial and a security event, never a weaker check
  (SEC-IAM-069). The limiter: the delay schedule of SEC-API-056 for
  guessable secrets (claim code and pairing code in R1, and the
  share-link password from R1.2), never permanent; the claim code's delays per source with no
  server-wide limit one source can use up (SEC-IAM-008); per-source and
  server-wide limits on every endpoint that checks a secret or starts a
  sign-in ceremony, using WP-032's keyed GCRA and its global key rather
  than a counter of its own, and registered in `limits.toml`
  (SEC-IAM-101). The source is always the `ClientContext` the listener
  resolved (WP-118), never a socket peer or header read here
  (SEC-OPS-037). Every failure is written to the diagnostic log as one
  line in the documented fail2ban format with that address (SEC-IAM-099,
  SEC-OPS-028), and emitted as a typed authentication event into the
  injected `SecuritySink` (WP-006), which the server wires to the bus;
  the audit log's sink (WP-069) stores the record.
- **Interface sketch.** `pub enum Pathway { Passkey, PairedBrowser, ClaimCode, RecoveryCode, RecoveryLink, Invitation, PairingCode }` (R1.2 adds `SharePassword` and `Oidc`);
  `pub trait PathwayCheck { fn pathway(&self) -> Pathway; async fn check(&self, presented: Presented) -> Result<Verified, Failure>; }`;
  `pub async fn verify(p: &dyn PathwayCheck, presented: Presented, source: &ClientContext) -> Result<Verified, SignInError>`.
- **Tests.** The back-off schedule with a manual clock; a different account
  from the same address; the exact log line format against the documented
  one; a unit test that enumerates `Pathway` and asserts each variant is
  wired to the verifier (adding a pathway without wiring fails); a test
  that enumerates `Pathway` and asserts that an empty and a missing
  credential are each refused for every variant, with the stand-in check
  never called (SEC-HIS-004); with a stand-in pathway, unknown, disabled
  and wrong give byte-identical failures and run the same check (observed
  through a counting hook, not timing); a fault injected into the check
  gives denial and exactly one security event in a recording sink; a
  hostile source's failures never delay the claim code for another
  source; requests from many sources reach the server-wide ceiling. The
  per-pathway integration tests, driving each pathway past its limit, run
  in the pathway's own package and in WP-117.

### WP-065 Authorisation layer

- **Wave** 2 · **Size** M · **Depends on** WP-006, WP-033, WP-042, WP-043,
  WP-044, WP-046.
- **Owns** `crates/gunmetal-server/src/access/`,
  `crates/gunmetal-server/tests/storage_access.rs` (the architecture
  test), `crates/gunmetal-durable/src/identity/permitted.rs` (the
  identity store's `Permit`-taking reader, one `mod` line in that
  directory's registry).
- **Serves** ACC-121, ACC-030, ACC-037.
- **Security.** Boundaries TB4, TB10; threats TM-T12, TM-T15, TM-T16.
  Verifies SEC-API-010, SEC-API-011, SEC-API-012, SEC-API-014, SEC-API-016,
  SEC-IAM-069 (the denial and the emitted event; the stored record is
  WP-069's), SEC-IAM-070, SEC-IAM-076, SEC-TM-024, SEC-TM-026,
  SEC-HIS-010, SEC-MED-051.
- **Scope.** The library grants table (a schema part in the identity
  store, where ADR 3 puts grants) and the API to read and change grants,
  which WP-099 calls; asking the core policy for a `Permit` (WP-033), the
  only value storage readers accept, so every fetch, list, search and
  mutation passes through the one visibility predicate built from the
  principal's grants (SEC-IAM-070, SEC-TM-024, SEC-HIS-010); the
  `Visible<T>` and `Editable<T>` wrappers whose constructors only this
  module can call, produced by a generic `check<T: HasLibrary>` that any
  row type implementing the core trait can pass through (SEC-API-010); a
  hidden object answers exactly as a missing one (SEC-API-011); the batch
  rule (reject the whole request if any identifier is not visible,
  SEC-API-012); visibility applied on every read path, including search,
  artwork, lyrics, playlists, now playing, events and sync (SEC-API-014,
  SEC-TM-026, SEC-MED-051); grant changes taking effect on the next
  request (SEC-IAM-076); any error or missing grant data ending in denial
  and a typed security event emitted into the injected `SecuritySink`
  (SEC-IAM-069); and the per-recipient fan-out check for pushed events,
  with no broadcast-to-all path (SEC-API-016). The single authorisation
  door beyond the catalogue: the identity store's public reader, added
  here in `permitted.rs` because `Permit` (WP-033) and the store (WP-046)
  were in the same wave, takes a `&Permit`; the user log's and the audit
  log's readers take one too (WP-068, WP-069). As first written only the
  catalogue readers (WP-067) did, so a handler could read another
  person's Activity or Identity data without asking the policy
  (SEC-TM-024, SEC-API-010). The architecture test: no module of the
  server crate outside `access/`, `session/` and `verifier/` calls a
  pre-principal lookup or a reader that takes no `Permit`, and the only
  other callers of the user log's rebuild replay and the audit log's
  verifier are the startup module (WP-095) and the host-only
  `audit_cli/` (WP-069), each on the same written list; the test scans
  the server crate's source on every gate run, so a module a later
  package adds is checked the moment it merges.
- **Why it changed in review.** As first written, this package owned the
  `Principal` type (which WP-062, in the same wave, must produce),
  returned catalogue rows (which WP-067, in the same wave, defines) and
  relied on a grants table that only WP-099, a wave later, created.
  `Principal`, `LibrarySet` and `HasLibrary` now live in the core
  (WP-033); catalogue readers in WP-067 take a `&Permit` (a `&LibrarySet`
  until the security alignment) and have no unfiltered form; and the
  grants table is owned here.
- **Interface sketch.** `pub struct Visible<T>(T);`
  `pub fn permit(p: &Principal, action: Action, grants: &Grants) -> Result<Permit, ApiError>;`
  `pub fn check<T: HasLibrary>(permit: &Permit, row: Option<T>) -> Result<Visible<T>, ApiError>;`
  `pub async fn grant(..)`, `pub async fn revoke(..)`.
- **Tests (real SQLite).** With a fixture row type and a test schema part
  holding rows in two libraries: a row in a library the caller cannot see
  gives exactly the same 404 body as a nonexistent ID; a batch with one
  hidden item is rejected whole (property over generated lists with one
  forbidden ID); revoking a grant takes effect on the next request; a
  database error injected while reading grants gives denial and a
  security event in a recording sink, never an allowance; a compile-fail
  test (doc test) shows `Visible::new` is not callable outside the
  module, and another that a storage reader cannot be called without a
  `Permit`; the identity store's `Permit`-taking reader never returns a
  device or invitation of another account (real SQLite); the
  architecture test fails on a fixture source tree in which a stand-in
  handler module calls a pre-principal lookup, and passes on the real
  tree. The same 404
  test against real catalogue rows runs in WP-082 and WP-084, which have
  both pieces, and over every route in WP-131.
- **Risks.** Resolved for the security baseline: the first draft's
  `LibrarySet` had a public constructor in the core, which made the rule
  a convention; storage now takes a `Permit` that only the policy
  function can mint (SEC-API-010).

### WP-066 Change log

- **Wave** 2 · **Size** M · **Depends on** WP-006, WP-040 (`CatalogChange`),
  WP-042.
- **Owns** `crates/gunmetal-store/src/changelog.rs`,
  `crates/gunmetal-store/src/changelog.sql`.
- **Serves** LIB-018, CLI-022; API-SYNC-02. (The tool change feed,
  INT-006 and API-TOK-02, is R2 with WP-091; sync status on the device,
  CLI-024, is R1.1 and client only.)
- **Security.** Boundaries TB4, TB10; threats TM-T15. Verifies SEC-API-025.
- **Scope.** The ordered change log in the cache: a sequence number per
  change to items, artwork, relations and profile data, written in the
  same transaction as the change; cursors that carry the cache generation;
  reading after a cursor with a page size; compaction to a horizon, with
  "resnapshot needed" for older cursors.
- **Interface sketch.** `pub fn append(tx: &Transaction, c: &CatalogChange) -> Result<Seq, StoreError>`;
  `pub fn since(conn: &Connection, cursor: Cursor, limit: u32) -> Result<Page, CursorError>`.
- **Tests (real SQLite).** A change and its log entry commit or roll back
  together; a cursor from an older generation is refused; a cursor older
  than the horizon is told to resnapshot; pages are contiguous and never
  repeat. Property: concatenated pages equal the full log after any
  sequence of appends and compactions above the cursor.

### WP-067 Catalogue store

- **Wave** 2 · **Size** M · **Depends on** WP-033, WP-040, WP-042.
- **Owns** `crates/gunmetal-store/src/catalog.rs`,
  `crates/gunmetal-store/src/catalog.sql`.
- **Serves** the persistence of API-CAT-01 to API-CAT-10; LIB-016, LIB-021.
- **Security.** Boundaries TB10; threats TM-T15, TM-T58. Verifies
  SEC-IAM-070, SEC-API-066.
- **Scope.** Tables for files, tracks, albums, release groups, artists,
  credits, artwork records, lyrics, technical data, availability, and the
  per-file seek and frame indexes (which the packaging route checks
  segment numbers against and the seek-index fetch in WP-084 serves; as
  first written, no package stored them), with batch upserts that touch a
  row only when it really changed and return the list of
  `CatalogChange`s (so a no-change rescan writes nothing), and readers for
  the sync builder (the file inspector's reader joins with WP-156 in
  R1.2). Every reader that returns library content takes a `&Permit`
  (WP-033) and filters in SQL by the `LibrarySet` inside it; there is no
  unfiltered reader, and the inspector's reader will take a `Permit` for
  the inspect action, which only an administrator's principal can obtain
  (SEC-IAM-070). Every
  row type implements `HasLibrary`. The
  caller writes the returned changes to the change log in the same
  transaction (WP-102), because the change log (WP-066) is in this same
  wave.
- **Not in scope.** Deciding what the rows contain (wave 3 core packages).
  Writing change-log entries (WP-066, called by WP-102).
- **Interface sketch.** `pub fn apply_batch(tx: &Transaction, b: &CatalogBatch) -> Result<Vec<CatalogChange>, StoreError>`.
- **Tests (real SQLite).** Applying the same batch twice returns no changes
  the second time and leaves every row byte-identical (the LIB-016
  property, here at the storage layer); each kind of real change returns
  exactly the expected `CatalogChange` list, written literally; a removal
  cascades credits but not the curation-log projections; a reader given a
  `Permit` without a row's library never returns that row. Commit time
  for 500 files is recorded by the benchmark, not asserted in a test.

### WP-068 User log service

- **Wave** 2 · **Size** M · **Depends on** WP-033, WP-034, WP-035, WP-046.
- **Owns** `crates/gunmetal-durable/src/userlog/`.
- **Serves** ADM-078, CLI-093, LAT-007, MUS-182, MUS-184; API-LOG-01,
  API-LOG-02.
- **Security.** Boundaries TB4, TB10; threats TM-T12, TM-T18, TM-T60.
  Verifies SEC-PRV-050, SEC-TM-024, SEC-API-010.
- **Scope.** One writer task per data directory that appends events to the
  right stream and segment, de-duplicates by event ID, `fsync`s at the
  policy ADR 3 sets, recovers torn tails at startup and reports damage,
  replays streams for projections and for cache rebuilds, and performs the
  sanctioned erasure rewrite (per event, per range, per profile). Every
  reader that returns a person's history, playlists, queue or loves takes
  a `&Permit` for that profile's stream (WP-033), so a handler cannot read
  another person's Activity data without asking the policy (SEC-TM-024,
  SEC-API-010); as first written these readers took no `Permit`. The one
  exception is the rebuild replay, which yields events only into the
  projection builders modules register and returns nothing a handler can
  serve; it is on the architecture test's written list (WP-065), and
  only the startup module (WP-095) calls it.
- **Interface sketch.** `pub struct UserLog; impl UserLog { pub async fn append(&self, e: Event) -> Result<Appended, LogError>; pub fn read(&self, permit: &Permit, stream: StreamId, range: Range) -> Result<Page, LogError>; pub fn replay_into(&self, stream: StreamId, sink: &mut dyn ProjectionBuilder) -> Result<(), LogError>; pub async fn erase(&self, sel: Erasure) -> Result<ErasureReport, LogError>; }`.
- **Tests (real files).** A torn final record (the test truncates the file)
  is cut back at reopen and reported; a corrupt middle record is reported
  with its range and the rest replays; the same event appended twice is
  stored once; a month boundary starts a new segment; an erasure leaves no
  trace of the erased payload in the segment bytes; a reader given a
  `Permit` for profile A never returns an event of profile B (property
  over generated streams), and a compile-fail test shows `read` cannot be
  called without one. Power-loss behaviour
  beyond what truncation simulates is out of reach in CI and stated as
  such.

### WP-069 Audit log

- **Wave** 2 · **Size** L (was M; the baseline makes the log
  tamper-evident, and the second security pass added the bus sink, the
  `Permit` on readers and the address side store) · **Depends on**
  WP-005, WP-006 (the event catalogue, `SecuritySink` and
  `ClientContext`), WP-033 (`Permit`), WP-043 (the bus, `app.rs` and
  `cli.rs`, which this package already extended without depending on
  it), WP-046, WP-047 (the audit signing key and the address-commitment
  key), WP-126, WP-138 (the retention schedule).
- **Owns** `crates/gunmetal-durable/src/audit/`,
  `crates/gunmetal-server/src/audit_sink/` (the one server-side sink that
  records every security event from the bus, registered by one line in
  `app.rs`), `crates/gunmetal-server/src/audit_cli/` (the
  `gunmetal audit verify` subcommand, one dispatch line in `cli.rs`).
- **Serves** ADM-110 (the security part), ACC-078; API-SCAN-05,
  API-USR-04.
- **Security.** Boundaries TB4, TB10, TB11; threats TM-T12, TM-T27,
  TM-T61. Verifies SEC-OPS-020, SEC-OPS-021, SEC-OPS-023, SEC-OPS-026,
  SEC-OPS-027 (the readers' truncation; the routes are WP-100's),
  SEC-OPS-075 (the checkpoint-head reader; serving and exporting it are
  WP-100's and WP-090's), SEC-OPS-037 (the address comes only from the
  `ClientContext`), SEC-IAM-094, SEC-IAM-097, SEC-PRV-003 and SEC-PRV-005
  (the audit parts: addresses coarsened at 30 days and removed at 90),
  SEC-TM-024 and SEC-API-010 (the readers take a `Permit`), and the
  stored-record part of SEC-OPS-029, SEC-PRV-008, SEC-IAM-069, SEC-TM-014
  and SEC-HIS-046, whose producers (WP-043, WP-048, WP-064, WP-065) prove
  only that they emit the typed event.
- **Scope.** A security audit log separate from diagnostic logs and from
  the cache (SEC-OPS-020): append-only JSON-lines segments of 16 MB
  written through the data-root handle, one canonical object per line,
  `fsync` before critical actions return and refusal of the action if it
  fails; each record carries a sequence number, UTC time, the event's
  name from the catalogue (WP-006), actor (account, profile, device and
  credential IDs), the transport and path class from the request's
  `ClientContext` (WP-118), targets and outcome, with every field encoded
  against injection and no credential (SEC-OPS-021, SEC-OPS-037); each
  record commits to its predecessor by hash, the server signs checkpoints
  with the audit key, and `gunmetal audit verify` reports the first
  record that breaks the chain (SEC-IAM-094, SEC-OPS-023).

  **The bus sink.** `audit_sink/` subscribes to the bus (WP-043) and
  records every security event any producer emits, including the egress
  client's denials (WP-048), the credential verifier's authentication
  events (WP-064), the authorisation layer's denials (WP-065) and the
  debug-level switch (WP-043). As first written, those packages said the
  audit log "records" their events, but this package owned no subscriber
  and did not depend on the package that owns the bus, so nothing
  connected them.

  **Addresses and retention.** The hash chain and the privacy schedule
  would otherwise conflict: SEC-PRV-003 and SEC-PRV-005 require source
  addresses to be coarsened after 30 days and removed at 90, while
  SEC-IAM-094 and SEC-OPS-023 require `verify` to detect any modified
  record. So a record never holds an address. It holds a keyed commitment
  (HMAC under a dedicated audit-address key in the secrets crate) over the
  address and a per-record random salt, and the address and salt live in
  a side store, `durable/audit/addresses.db`, opened through the one
  connection opener with `secure_delete=ON` (WP-126). On the schedule in
  WP-138, a coarsening run replaces each address older than 30 days with
  its /24 or /48 prefix, and a removal run deletes the address and salt
  of every entry older than 90 days, with the WAL checkpointed and
  truncated; each run is recorded as a signed checkpoint, like a prune,
  so verification still passes and the runs themselves are evident.
  Once the salt is gone, nothing left can be brute-forced back to an
  address. Pruning whole records at 365 days is recorded the same way
  (SEC-OPS-026). WP-133's sweep job calls this package's retention API
  and never edits audit files. This applies the baseline's
  recommendation for owner decision 15 (addresses coarsened after 30
  days and removed at 90); the owner confirms it.

  **Readers.** Every reader takes a `&Permit` (WP-033) (SEC-TM-024,
  SEC-API-010): the per-person readers for "your sign-in history" and
  "admin accessed your data" (SEC-IAM-097, SEC-IAM-077), which show the
  person their own full addresses while the side store still holds them;
  the full reader for a `Permit` carrying the audit capability, which
  only the owner and holders of that capability can obtain, with other
  people's addresses truncated to /24, /48 or a country in the response
  type itself (SEC-OPS-027); and the latest signed checkpoint head, for
  WP-100 to serve and WP-090 to export (SEC-OPS-075). Appends and the
  host-only `audit verify` are the only entry points without a `Permit`,
  and both are on WP-065's written list. A pre-allocated reserve lets a
  fixed list of recovery actions write their records on a full disk.
- **Not in scope.** Audit routes and the investigation mode (WP-100).
  Exporting checkpoints (WP-090). The admin client's check of the head at
  sign-in (client plan).
- **Interface sketch.** `pub struct AuditLog; impl AuditLog { pub async fn append(&self, e: SecurityEvent, from: Option<&ClientContext>) -> Result<Seq, AuditError>; pub fn read_own(&self, permit: &Permit, range: Range) -> Result<Page<OwnRecord>, AuditError>; pub fn read_all(&self, permit: &Permit, range: Range) -> Result<Page<TruncatedRecord>, AuditError>; pub fn head(&self, permit: &Permit) -> Result<SignedHead, AuditError>; pub async fn apply_retention(&self, s: &Schedule, now: Timestamp) -> Result<RetentionReport, AuditError>; pub fn verify(&self) -> Result<(), BrokenAt>; }`;
  `pub struct AuditSink; impl SecuritySink for AuditSink { .. }`.
- **Tests (real files and real SQLite).** A critical event whose write
  fails (read-only directory in the test) refuses the action; on a
  simulated full disk the recovery actions are recorded and every other
  admin action is refused; rotation at the size limit; control
  characters in a user agent are escaped; a reader for one person never
  returns another's events; the owner's reader shows other people's
  addresses truncated, and the response type has no field that could
  hold a full address of another person. The sink, with literal bus
  events: one of each producer's event types (an egress denial, an
  authentication failure, an authorisation denial, the debug-level
  switch) published on a real bus each yields exactly one record,
  compared as a whole value. Retention on a manual clock with a sentinel
  address: at day 31 the side store holds only the /24 (or /48) prefix;
  at day 91 a raw-byte scan of the audit directory, the side store and
  its WAL finds neither the sentinel address nor its salt; `audit verify`
  passes after each run, and each run left one signed checkpoint.
  Properties: any random set of edits, removals or reorderings of a valid
  log is detected by `verify`, which names the first bad record; a log
  pruned, coarsened or with addresses removed still verifies; every
  record's fields round-trip as whole values; a compile-fail test shows
  no reader can be called without a `Permit`.

### WP-070 Task runner

- **Wave** 2 · **Size** M · **Depends on** WP-042 (checkpoints and task
  state live in the cache), WP-043.
- **Owns** `crates/gunmetal-server/src/tasks/`.
- **Serves** ADM-095; API-SCAN-04 (the engine). (The admin's task list
  with run, cancel and history, ADM-093, is R1.2, WP-155, over this
  engine; background analysis, LIB-024, is R1.3.)
- **Security.** Boundaries TB4; threats TM-T09. Verifies SEC-API-064.
- **Scope.** Named task kinds registered by modules; schedules; one-off
  requests (with de-duplication of path-scoped requests); progress,
  cancel, last run, duration and error; throttling and checkpoints so heavy
  jobs resume after a restart; expiry sweeps as a built-in schedule other
  modules register into. The R1 task kinds are a closed enum declared here
  (library scan, path refresh, backup, purge, rebuild), so a package can
  request a kind before the package that handles it has merged (WP-082
  and WP-099 request scans that WP-102 handles a wave later); a request
  for a kind with no handler stays queued and is reported as waiting.
  The point releases add their kinds with the packages that handle them,
  one variant each under the merge protocol: import matching and the
  parser-upgrade re-read in R1.1 (WP-145, WP-123), and analysis, neighbour
  rebuild and rule re-evaluation in R1.3 (WP-114, WP-113).
- **Interface sketch.** `pub trait Task { fn kind(&self) -> TaskKind; async fn run(&self, ctx: TaskCtx) -> Result<Outcome, TaskError>; }`;
  `pub fn request(&self, kind: TaskKind, input: TaskInput) -> TaskHandle`.
- **Tests.** Two requests for the same path collapse into one; cancel
  stops at the next checkpoint; a task that panics is recorded as failed
  and the runner keeps going; a checkpointed task resumes from its
  checkpoint after the runner restarts (state persisted in the cache).

### WP-072 Web client asset serving

- **Wave** 3 (moved from 2 in review) · **Size** S · **Depends on**
  WP-043, WP-044, WP-118. As first written it depended only on WP-044,
  but it lives in the server crate (WP-043) and registers routes (WP-118).
- **Owns** `crates/gunmetal-server/src/webapp/`.
- **Serves** CLI-001. (Installing the web app and loading it offline,
  CLI-003 and CLI-024 to CLI-026, are R1.1: WP-148 adds the web app
  manifest and the service-worker rules.)
- **Security.** Boundaries TB2, TB4; threats TM-T07, TM-T08. Verifies
  SEC-API-044, SEC-IAM-014, SEC-IAM-015, SEC-CLI-011, SEC-CLI-012,
  SEC-NET-058, SEC-STD-017, SEC-SUP-037, SEC-HIS-028, SEC-API-049 (the
  server's half: a CSP that lets the bundle load nothing from another
  origin; the all-screens network recording is the client's).
- **Scope.** Serve the web client's built bundle from bytes embedded in the
  binary, from a build-time manifest of exact paths and allowed
  extensions, never a directory listing, version-control metadata or
  source maps (SEC-STD-017), on the same origin as the API (SEC-IAM-014,
  SEC-NET-058, SEC-CLI-012, SEC-SUP-037); every HTML response with the
  exact Content Security Policy of SEC-API-044 (`default-src 'none'`, no
  inline script, `frame-ancestors 'none'`) and the headers of
  SEC-IAM-015; cache headers by content hash; no service worker is served
  or registered in R1 (WP-148 adds one in R1.1, with its scope rules); and
  the build identifier check, which answers an API call from a bundle whose build differs from
  the server's with a typed "reload required" error (SEC-CLI-011). The
  bundle itself comes from the client work; tests use a two-file stand-in
  bundle.
- **Tests.** No directory listing; unknown paths fall back to the app shell
  only for navigation requests; hashed assets are immutable-cacheable and
  the shell is not; the exact CSP string on every HTML route; a request
  for `.git/HEAD`, a `.map` file or an environment file is 404; a
  mismatched build header gets the typed error; the stand-in bundle's
  manifest has no service-worker script and no response carries
  `Service-Worker-Allowed`.
- **Added for the client plan (2026-10-03).** This package owns putting the web
  client's bundle into the server binary. The workspace allows no build
  script (SEC-SUP-026), so the mechanism is an xtask command, in this
  package's own xtask file, that reads the manifest the client's
  production build writes (CP-056: exact paths, types and SHA-256
  digests) and writes the asset table the server includes, behind a
  cargo feature; without the feature the server serves the two-file
  stand-in, so the Rust half of the gate never needs the bundle. The
  build order is: the facade for `wasm32`, the client bundle, this
  xtask step, the server. Its tests move from the stand-in to the real
  manifest when CP-056 lands. The content security policy spike the
  register mentions under D-74 is the client plan's CP-003, not this
  package.

### WP-073 Network settings and owner HTTPS

- **Wave** 3 (moved from 2 in review) · **Size** M · **Depends on**
  WP-023, WP-043, WP-044, WP-047 (the rustls server configuration comes
  from its crypto module), WP-118 (its settings routes, and the TLS
  configuration of the listener WP-118 owns).
- **Owns** `crates/gunmetal-server/src/network/`, `packaging/proxies/`
  (reverse-proxy configurations), `.github/workflows/proxies.yml`.
- **Serves** ACC-098, ADM-022 (owner's certificate part), CLI-150, and
  the recipes behind ACC-097; API-SET-01, API-SYS-02. Remote access in R1
  goes through the owner's own reverse proxy or a tailnet (owner answer,
  2026-10-02; built-in remote access is R2), so the shipped proxy
  configurations and the Tailscale Serve recipe here are R1's documented
  remote paths. Trusted proxies (ACC-097) moved to WP-132, which owns
  everything that decides where a request came from. Serving under a path
  prefix (ACC-134) is R1.2 and split off to WP-151.
- **Security.** Boundaries TB1, TB2; threats TM-T11, TM-T46. Verifies
  SEC-NET-002, SEC-NET-013, SEC-NET-015, SEC-NET-022, SEC-API-038,
  SEC-API-069, SEC-TM-010.
- **Scope.** The canonical origin for each path a request can arrive on, from which every
  absolute URL the server emits is built, never from `Host`,
  `:authority` or forwarding headers (SEC-NET-015, SEC-API-069); HTTPS
  with the owner's certificate and key (reloaded on change), negotiating
  only TLS 1.3, or TLS 1.2 with ECDHE and an AEAD suite, and never
  answering cleartext on the TLS port (SEC-NET-002, SEC-TM-010), with
  `Strict-Transport-Security` under a hostname (SEC-API-038); the
  secure-context report for the client; shipped configurations for Caddy,
  nginx, Traefik and Cloudflare Tunnel and a Tailscale Serve recipe, with
  a CI job that runs each real proxy except Tailscale in front of the
  server and asserts the client address, the scheme and the absence of
  identity-header trust (SEC-NET-022, SEC-NET-013).
- **Not in scope.** A path prefix for every route and cookie path
  (WP-151, R1.2). Certificates by ACME (WP-101).
- **Tests.** A forged `Host` and forged forwarding headers never change an
  emitted URL; a
  certificate and key that do not match are refused at load; recorded
  ClientHello fixtures for SSL 3.0, TLS 1.0, TLS 1.1 and TLS 1.2 without
  ECDHE are refused; the secure-context report for `localhost`, a private
  address over HTTP and a domain over HTTPS; the proxy CI job per proxy.
- **Risks.** The rustls crypto provider comes from the cryptography record
  (WP-125; owner decision 8).

### WP-074 Update check and advisories

- **Wave** 3 (moved from 2 in review) · **Size** M · **Depends on**
  WP-005, WP-043, WP-047 (TUF signatures are verified through its crypto
  module's keyless Ed25519 function), WP-048, WP-118. As first written it
  lacked WP-043, although it lives in the server crate.
- **Owns** `crates/gunmetal-server/src/updates/`.
- **Serves** ADM-053, ADM-054, ADM-060, ACC-126, ACC-128, ACC-127
  (reference to ADM-054); API-SET-06.
- **Security.** Boundaries TB8, TB12; threats TM-T01, TM-T43, TM-T44.
  Verifies SEC-OPS-019, SEC-OPS-047, SEC-OPS-068, SEC-OPS-070, SEC-SUP-049,
  SEC-SUP-050, SEC-SUP-051, SEC-TM-067, SEC-HIS-059, SEC-API-081.
- **Scope.** The check is off until the owner answers the required
  first-run question, which has two explicit answers and no preselection
  (SEC-OPS-047; the question itself is asked by WP-080); while it is off,
  the dashboard shows a quiet status and `doctor` flags it. When allowed,
  fetch the feed once a day with jitter through the egress gate as a plain
  GET of static files with no query string, cookie, instance identifier
  or installed version (SEC-SUP-051); verify it as TUF 1.0 metadata from
  the root compiled into the binary, replaced only through signed root
  rotation (SEC-OPS-019, SEC-SUP-049, SEC-SUP-050); refuse rollback,
  expired, inconsistent or oversized metadata; decode it into typed
  structures with size limits (SEC-API-081); match advisories and the
  supported-version policy against the running version locally
  (SEC-OPS-070); and show "Can't confirm you're up to date" after seven
  days without a fresh feed. The feed schema has no command or action
  field, so nothing in it can disable, change or run code on a server
  (SEC-OPS-068, SEC-TM-067).
- **Tests.** Against a local feed server in the test: a valid feed; a feed
  signed by the wrong key; a feed with a lower version than one already
  seen; an expired feed; a feed of 10 MB; the seven-day message with a
  manual clock. Nothing is fetched when the owner has not allowed it.
  ACC-128 (nobody can switch the server off remotely): a validly signed
  feed whose advisory or any other field asks for anything beyond
  displaying a banner changes nothing but the banner; the server keeps
  serving whatever the feed says.
  The exact outbound request bytes are captured and compared literally;
  fixture feeds drive the banner (affected, unaffected, OSV range edges,
  out of support) and the off-state status; a rotated root chain is
  accepted and a root signed by too few keys is refused.
- **Risks.** The baseline requires TUF 1.0 (SEC-SUP-049), where the first
  draft left TUF or a simpler signed document open; the remaining choice
  is `tough` or a minimal client (owner decision 12). The feed is
  published by WP-136. The root compiled into the binary presumes the
  offline TUF keys exist (owner decision 32; baseline decision 19).

### WP-118 Route registry and HTTP listener (added in review)

- **Wave** 2 · **Size** L (was S, then M; it gained the connection
  limits, and in the second security pass the client-address resolver,
  the public-route allow-list, the anonymous suite and the OpenAPI
  generation) · **Depends on** WP-005, WP-006, WP-008, WP-023, WP-032,
  WP-043, WP-044.
- **Owns** `crates/gunmetal-server/src/routes.rs`,
  `crates/gunmetal-server/src/listener.rs`,
  `crates/gunmetal-server/security/public-routes.txt` (a registry file;
  see the shared-file table), `crates/gunmetal-server/tests/anonymous.rs`,
  `crates/gunmetal-server/openapi.json` (generated),
  `crates/xtask/src/openapi.rs`.
- **Serves** ADM-001 (the one binary actually serving); INT-001 (the route
  table the API reference is generated from); API-SYS-09 (the source).
- **Security.** Boundaries TB1, TB2, TB4; threats TM-T02, TM-T03, TM-T05,
  TM-T09. Verifies SEC-API-009, SEC-API-060, SEC-API-061, SEC-API-062,
  SEC-NET-021, SEC-NET-048, SEC-NET-049, SEC-IAM-067, SEC-API-002,
  SEC-API-003 and SEC-TM-004 (the anonymous suite for no and malformed
  credentials; WP-131 adds the expired, revoked and disabled-account
  states), SEC-API-091, SEC-OPS-037 (the one resolver), SEC-NET-016 (the
  server's ignore-unless-trusted rule and the header matrix), SEC-NET-068
  (the "unknown" classification of a gateway peer).
- **Why it exists.** WP-043 owned the route registry, but a registry of
  `gunmetal-http` route entries cannot compile before `gunmetal-http`
  exists, and WP-044 is in the same wave as WP-043. In the second
  security pass it took three doors that wave 2 and wave 3 packages need
  before the packages that first held them: the client-address resolver
  (from WP-132, wave 3, after the sessions, verifier, audit log and
  request limits that need a resolved address in wave 2), the
  public-route allow-list and anonymous suite (from WP-131, wave 3,
  which the twenty or so wave 3 route packages could not depend on), and
  OpenAPI generation (from WP-089, wave 3, which WP-131's API fuzzing job
  needs).
- **Scope.** The server's route registry (one line per module, the
  registry file in the table above), building the router from it through
  WP-044, binding the configured address, graceful shutdown, and the
  `serve` subcommand's dispatch line. TLS configuration is a hook WP-073
  fills. The connection-level limits, applied before anything is
  buffered: a global connection cap and a per-client-key cap; header
  reading timed out after 10 seconds; a header section of at most 16 KiB
  and 64 fields, a request target of at most 4 KiB; a minimum body data
  rate, idle bodies timed out after 30 seconds and non-streaming handlers
  after 30 seconds; streaming responses closed when the client stops
  reading for 60 seconds (SEC-API-060, SEC-API-061, SEC-NET-048). Ambiguous
  HTTP/1.1 framing is rejected (both `Content-Length` and
  `Transfer-Encoding`, repeated lengths, obsolete folding, bare CR or LF;
  SEC-API-009, SEC-NET-021). If HTTP/2 is enabled, at most 100 concurrent
  streams per connection, bounded header lists and CONTINUATION frames,
  and connections closed when resets come too fast (SEC-API-062,
  SEC-NET-049). Every route in the registry carries its policy, and a
  route registered outside it cannot be served (SEC-IAM-067).

  **The client-address resolver.** The listener is the only module that
  sees the socket, so it is the only module that reads the peer address
  (WP-001's lint bans `ConnectInfo` and `peer_addr` everywhere else). For
  every connection it calls the core's path-class function (WP-023) with
  the peer, the forwarding headers, the trusted-proxy list (empty by
  default, read from configuration; WP-132 adds the routes that change
  it) and the probed host facts (the server's interface prefixes, its
  default gateway and bridge addresses, any configured container
  gateway), and attaches the resulting `ClientContext` (WP-006) to the
  request before WP-044's pipeline runs. Forwarding headers are believed
  only from a trusted proxy, and through one the address is the first
  untrusted hop from the right (SEC-NET-016, SEC-NET-018); a peer equal
  to the gateway, a bridge or a container gateway is "unknown" and
  non-local, with its own limiter bucket (SEC-NET-068). The sessions
  (WP-062), the credential verifier (WP-064), the audit log (WP-069),
  the request limits (WP-130) and this package's per-client cap all take
  this one `ClientContext` (SEC-OPS-037). A loopback peer that sends
  forwarding headers without being a declared trusted proxy, such as a
  reverse proxy on the same host, is not classified as loopback: its
  headers are ignored and its path class is internet, so a request
  through it can never use a loopback exemption (SEC-IAM-013,
  SEC-NET-017). Postures, the cleartext rule and the security event for
  untrusted forwarding headers stay with WP-132.

  **The public-route allow-list and the anonymous suite.** This package
  creates `security/public-routes.txt` and the check that the set of
  public, credential-exchange and capability routes in the registry
  equals it exactly, so a public route that is not on the reviewed list
  fails the build (SEC-API-002, SEC-IAM-067), and the anonymous-request
  suite generated from the registry: every route outside the list answers
  401 with a byte-identical body, before reading the body or running the
  handler, for no credential and a malformed one (SEC-TM-004,
  SEC-API-003). They need only the registry and WP-044's test client, so
  from wave 2 every route package adds its line to an existing file and
  runs both checks before it merges. WP-131 extends the suite with the
  states that need sessions and its fixture world.

  **The OpenAPI description.** Generated from the registry at build time
  into `openapi.json`, with a CI check that fails when the committed
  copy differs and a test that the release-profile route list equals the
  description, so no debug or test route ships (SEC-API-091). WP-089
  serves it as the API reference, and WP-131's API fuzzing job drives the
  server through it.
- **Tests.** With two stand-in modules registered in the test: both route
  sets are served; a duplicate path and method across modules fails at
  startup with both module names; `serve` on a free port answers a request
  through the full WP-044 pipeline over a real socket; shutdown completes
  in-flight requests within a stated bound. Raw request bytes written to a
  socket for each smuggling case of a corpus are rejected; each limit is
  tested at the value and one past it with test-scaled timeouts
  (slow-header and slow-body clients); an HTTP/2 client that floods
  resets and CONTINUATION frames is disconnected. The resolver over real
  sockets: the header matrix (each forwarding header, from a trusted and
  an untrusted peer) asserts the resolved address and class as whole
  values; with an empty trusted list, a spoofed header from any peer
  leaves the `ClientContext` exactly as the bare peer gives it; a
  loopback peer sending `X-Forwarded-For` is classified internet, not
  loopback; a peer equal to a probed gateway address is "unknown"; a
  stand-in handler sees only the attached `ClientContext`. A deliberately
  broken fixture route (public but not on the list, and a non-public
  route that answers anonymously) fails the allow-list check and the
  anonymous suite with the route's name. The generated description of
  the stand-in routes equals a literal expected document, and a route
  added without regenerating fails the diff check.

### WP-119 Synthetic library generator (added in review)

- **Wave** 2 · **Size** M · **Depends on** WP-007, WP-010, WP-011, WP-012,
  WP-013, WP-014, WP-015, WP-016, WP-017, WP-019, WP-020.
- **Owns** `crates/gunmetal-testkit/src/library.rs`.
- **Serves** the synthetic-media rule; the test needs of WP-084, WP-102,
  WP-110, WP-111 and WP-117 (and of WP-092 in R1.3); the benchmark
  (WP-115).
- **Security.** Boundaries TB9; threats TM-T20. Verifies no requirement of
  its own: it holds no security control, and the rules it relies on are
  proved by the packages that own them.
- **Why it exists.** As first written, the generator was part of the
  benchmark in wave 5, while the scan pipeline (wave 4) and other packages
  needed a synthetic library for their own tests a wave or more earlier.
- **Scope.** From a seed and a shape (albums, multi-disc sets,
  compilations, same-titled albums, artwork as tiny hand-built PNG and
  JPEG files, `.lrc` sidecars, odd and conflicting tags, a damaged file of
  each format), write a library of tiny files to a temporary directory with
  the format builders already in the testkit, plus a manifest of what each
  file should yield. The manifest is written by the generator from its
  own inputs, never by running Gunmetal's parsers.
- **Tests.** The same seed and shape give byte-identical trees; a small
  library's manifest is checked literally; each file the generator writes
  is accepted by the builder's own literal checks (WP-007's checksums).

### WP-130 Request limits, concurrency gates and the limits register (added for the security baseline)

- **Wave** 2 · **Size** M · **Depends on** WP-006, WP-032, WP-043, WP-044.
- **Owns** `crates/gunmetal-server/src/limits/`,
  `crates/xtask/src/limits.rs`, and its entries in
  `crates/gunmetal-server/limits.toml`, a registry file that any package
  may create (see the shared-file table; WP-064 adds entries in this
  same wave).
- **Serves** INT-012, ACC-111 (per-person limits); the resource rules of
  web-and-api-security.md and network-and-remote-access.md.
- **Security.** Boundaries TB1, TB2, TB4; threats TM-T04, TM-T09. Verifies
  SEC-API-057, SEC-API-064, SEC-API-088, SEC-NET-051, SEC-NET-052,
  SEC-NET-053, SEC-TM-068, SEC-IAM-101, SEC-IAM-102, SEC-STD-030.
- **Why it exists.** WP-044 had a rate-limit hook and WP-064 a guessing
  limiter, but nothing implemented the request rate limits, the
  per-user and global concurrency limits for expensive work, or the one
  register of business limits the baseline requires (SEC-STD-030).
- **Scope.** The rate-limit hook for WP-044's pipeline: limits keyed on
  the principal when there is one and otherwise on the address key that
  WP-032 derives from the request's `ClientContext` (WP-118), the only
  address input this package accepts (SEC-OPS-037), with a server-wide
  ceiling for unauthenticated routes under WP-032's global key, answered
  with a typed "too many" error (SEC-API-057, SEC-NET-052, SEC-TM-068);
  state for unauthenticated clients in WP-032's fixed-capacity stores
  (SEC-NET-051). Concurrency gates with queueing, per user and global, for
  search, artwork resizing, scans, exports and uploads, so one person
  cannot starve the household, with a repeated request for running work
  treated as the same request (SEC-NET-053, SEC-API-064); per-principal
  upload quotas by count and bytes (SEC-API-088); device and concurrent
  stream limits per account as data the session and stream packages
  enforce (SEC-IAM-102). The register: one machine-readable file of every
  business limit, per user and server-wide, with its default and the test
  that enforces it, and an xtask check that every entry names an
  enforcing test (SEC-STD-030). Rate limits on endpoints that check a
  secret are WP-064's; both packages use WP-032's one implementation of
  the per-source and server-wide counters, so neither builds its own
  (SEC-IAM-101). As first written this package "provided the server-wide
  counters" that WP-064, in the same wave and not depending on it, was
  to share.
- **Interface sketch.** `pub struct Gate; impl Gate { pub async fn enter(&self, kind: Work, who: &Principal) -> Result<Permit, TooMany>; }`;
  `pub fn rate_hook(limits: &Limits) -> Hook`.
- **Tests.** A route over its per-principal rate gets the typed error
  while another principal's requests still pass; a /48 rotation hits its
  ceiling; one person saturating search does not delay another's;
  uploads past each quota are refused; the register check fails for an
  entry without a test (fixture register). Load figures are measured by
  WP-117's nightly job, not asserted here.

### WP-235 WASM facade: the crate, the type mechanism and the lint answer (added for the client plan)

- **Wave** 2 · **Size** M · **Depends on** WP-005, WP-006, WP-008,
  WP-040. (First placed in wave 1; moved on 2026-10-03 because wave 1
  was closing.)
- **Owns** `crates/gunmetal-wasm/Cargo.toml`,
  `crates/gunmetal-wasm/src/links.rs`, `text.rs`, `ids.rs`,
  `problems.rs`, `catalog.rs` (`lib.rs` is a registry),
  `crates/gunmetal-wasm/tests/workspace_rules.rs`,
  `crates/gunmetal-wasm/types/` (generated declarations, committed),
  `crates/xtask/src/wasm_types.rs` and
  `crates/xtask/src/facade_unsafe.rs` (each with its dispatch line).
  One edit to a file another package owns, under the interface-change
  rule and made here: `crates/xtask/src/lint_exceptions.rs` (WP-008's
  check) gains a second manifest that may carry its own lint tables,
  beside the core's.
- **Serves** ADR 1 decision 2; ADR 12 decisions on how types cross and
  on `unsafe`; the web player's first packages
  ([client-packages.md](client-packages.md)).
- **Security.** Boundaries TB4; threats TM-T62. Verifies SEC-MED-077,
  SEC-CLI-021, and SEC-CLI-002 and SEC-API-047 for the browser (the
  core's link filter is the one the web client calls).
- **Why it exists.** The web player is built in parallel with the server
  (owner answer, 2026-10-03) and needs the core in the browser before
  WP-088. Three things about the facade were unverified and sat on that
  path: whether `wasm-bindgen`'s generated code passes the workspace's
  `unsafe_code` lint, how rich types cross to TypeScript, and how
  coverage counts the glue. This package settles them before any other
  slice is written.
- **Scope.** Create the crate record 6 names. Export WP-005's link
  filter and text normalisation, WP-006's public IDs and problem codes,
  and mirror types for WP-040's catalogue records, so the web client's
  demo library is typed by the core.

  **The type mechanism (record 12, decision 13).** For each core type
  that crosses, a mirror type in this crate with a conversion from and
  to the core type. For a core type with public fields the conversion
  takes the value apart field by field with no catch-all, so a field
  renamed, retyped, added or removed fails to compile. A core type with
  private fields (`Link`, `PublicId`, `Lufs`) can only be converted
  through its accessors, so a removed or retyped accessor fails to
  compile but an added one is not caught; the mirror then lists the
  accessors it reads, and review of the core change is what catches a
  new one. The mirrors derive `serde` and `tsify`, values cross through
  `serde-wasm-bindgen`, and `xtask wasm-types` regenerates the
  TypeScript declarations into `crates/gunmetal-wasm/types/` and fails
  when the committed copy differs.

  **The lint answer (owner decision 34).** The workspace sets
  `unsafe_code = "forbid"`, which no attribute in source can lower, so
  the exception is in the manifest: this crate does not take
  `[lints] workspace = true`. Its `Cargo.toml` repeats every workspace
  lint and lowers only `unsafe_code`, as far as the generated code needs
  and no further, with the reason in a comment.
  `tests/workspace_rules.rs`, like the core's, fails when a workspace
  lint is missing from the repeated table or any lint other than
  `unsafe_code` differs. `xtask lint-exceptions` already refuses any
  manifest that does not take the workspace lints, with the core as its
  one permitted case (`STRICTER`); the edit to `lint_exceptions.rs`
  permits this crate's manifest as the second, by exact path. The
  `EXCEPTIONS` list in that file holds Clippy bans per module and is not
  changed. `xtask facade-unsafe` fails on the `unsafe` keyword anywhere
  in this crate's source files, so hand-written `unsafe` stays refused
  while generated code is allowed.

  **The `wasm32` build.** `wasm-bindgen`'s generated items are compiled
  only for `wasm32`, so the host build never shows the lint firing. This
  package files a gate change request to the integrator: a step in
  `scripts/gate.sh` that builds this crate for `wasm32-unknown-unknown`
  with `--locked`, and the target in `ci.yml`. WP-088's `wasm.yml` in
  wave 3 keeps the size check.
- **Dependencies.** `tsify` (crates.io; MIT or Apache-2.0;
  github.com/madonoharu/tsify; not `tsify-next`) and
  `serde-wasm-bindgen` (crates.io; MIT;
  github.com/RReverser/serde-wasm-bindgen), both approved by the owner
  on 2026-10-03 (register, owner answers); the integrator adds them to
  `[workspace.dependencies]` with their cargo-vet entries, as for any
  new crate. `serde`'s derive feature for this crate. None is a
  dependency of the core.
- **Not in scope.** Every other export (WP-236, WP-237, WP-241, WP-088).
- **Tests.** Native unit tests of each conversion, with literal values.
  A fixture mirror with a field removed makes the drift check fail. A
  fixture source file holding `unsafe` makes the keyword check fail. A
  fixture manifest missing one workspace lint, and one lowering a lint
  other than `unsafe_code`, each fail `workspace_rules`. A third
  manifest with its own lint tables still fails `lint-exceptions`. The
  `wasm32` build passes under the lint. The committed declarations for
  the exported types equal a literal expected text.
- **Stop condition for others.** WP-236, WP-237, WP-241, WP-088 and
  every client package that depends on one do not start until this
  package has merged with its `wasm32` build passing in the gate. If the
  exception has to be wider than generated items need, or `tsify` does
  not compile under it, the mechanism goes back to record 12 before
  anything else is built.

### WP-238 Tint surfaces from an artwork colour (added for the client plan)

- **Wave** 2 · **Size** S · **Depends on** WP-005, WP-037.
- **Owns** `crates/gunmetal-core/src/tint.rs`.
- **Serves** MUS-110; design-language section 5 and accessibility
  requirement A1's hue sweep.
- **Security.** Boundaries TB4; threats TM-T62. Verifies no requirement
  of its own: it holds no security control. Its contrast check is what
  keeps a buggy or hostile server from making text unreadable
  (design-language, "Where the colour is computed").
- **Why it exists.** design-language has the client derive tinted
  surfaces from WP-037's base candidate by fixed rules. Written in each
  client that would be the same rule several times; record 1 puts one
  copy in the core.
- **Scope.** A pure function from a palette candidate (OKLCH numbers),
  the theme and the colours of the tokens that will sit on the surface,
  all passed in as numbers, to the surface colours of design-language's
  table: lightness and chroma cap by theme; out-of-gamut values brought
  into sRGB by reducing chroma first; the contrast of the resulting sRGB
  colour checked against every token passed in; lightness stepped by
  0.01 toward the canvas until every pair passes; no tint below the
  chroma floor; nothing for the high-contrast theme. And the placeholder
  hue from a hash of an item's ID.
- **Not in scope.** The token values themselves (the client's token
  files pass them in). Writing colours to a page (client).
- **Interface sketch.** `pub fn surfaces(base: Oklch, theme: Theme, on_surface: &[Srgb]) -> Option<Surfaces>`;
  `pub fn placeholder_hue(id: &PublicId) -> Hue`.
- **Tests.** Literal cases worked by hand for each rule. The sweep of
  design-language's "The guarantee" table, in steps finer than 5
  degrees at zero, half and the full chroma cap, with the document's
  token values as literals: every pair meets its floor. Properties: the
  result is always in sRGB; every token passed in meets its floor on the
  returned surface or the function returns none; the same input gives
  the same output.

### WP-239 Inbound-link parser (split from WP-089 for the client plan)

- **Wave** 2 · **Size** S · **Depends on** WP-005, WP-006, WP-008,
  WP-038.
- **Owns** `crates/gunmetal-core/src/deeplink.rs`,
  `crates/gunmetal-fuzz/src/deeplink.rs`,
  `crates/gunmetal-fuzz/tests/deeplink_corpus.rs`,
  `fuzz/fuzz_targets/deeplink.rs`, `fuzz/seeds/deeplink/`.
- **Serves** ACC-001, ACC-062, ACC-080; API-TOK-03 (the parser).
- **Security.** Boundaries TB1, TB4; threats TM-T47, TM-T65. Verifies
  SEC-CLI-025.
- **Why it exists.** The parser was part of WP-089 in wave 3. It is a
  pure function over typed values, and the web client may not parse an
  address itself (SEC-CLI-025), so it has to reach the browser before
  the screens that read a claim link, an invitation or a pairing link.
- **Scope.** One pure function that parses every inbound link, QR
  payload and fragment into a closed set of typed routes, each marked
  with whether it needs a confirmation screen; unrecognised input is
  "not recognised". In R1 the set holds the links R1 issues: the claim
  link, invitations, browser pairing and recovery enrolment. WP-159
  extends the set in R1.2.
- **Not in scope.** Resolving a route on the server (WP-089). Redeeming
  anything.
- **Tests.** Unit tests per route with literal inputs; a property that
  unrecognised input maps to "not recognised" and that no route is
  state-changing without its confirmation flag; the fuzz harness.

### WP-240 User events: the builder and the sink (added for the client plan)

- **Wave** 2 · **Size** S · **Depends on** WP-034.
- **Owns** `crates/gunmetal-core/src/userdata/compose.rs`,
  `crates/gunmetal-core/src/userdata/sink.rs` (two new files in
  WP-034's directory, under the interface-change rule; WP-034's own
  files are not changed).
- **Serves** ACC-117, CLI-093, MUS-180, MUS-182; API-LOG-01,
  API-USR-07.
- **Security.** Boundaries TB4, TB5; threats TM-T18. Verifies
  SEC-PRV-002 and the device's half of SEC-PRV-024.
- **Why it exists.** api-needs.md has the device append events with a
  client-made event ID and a hybrid logical clock, and has "the core's
  event sink" take a mode and drop events in private mode. WP-034 built
  the envelope, the clock and the merge rules; no package built the
  function that makes an event or the sink. Without them a client would
  write both, and the private-mode rule would be written once per
  client.
- **Scope.** The event builder: given a body (play, skip, love or
  unlove), the device and profile, the current clock, and the wall time
  and random bytes the caller passes in as plain values, return the
  event with its ID and the advanced clock. The core reads no clock and
  no random source itself. The sink: it takes a mode. In normal mode it
  returns the event for the outbound queue. In private mode it drops
  play, skip and completion events and recommendation signals before
  they can be queued, and returns nothing (SEC-PRV-024).
- **Not in scope.** Queueing, storage and upload (the clients; WP-068
  on the server). The server's own refusal of private plays (WP-086).
- **Interface sketch.** `pub fn compose(body: Body, by: Author, clock: Hlc, wall_ms: u64, entropy: [u8; 16]) -> (Event, Hlc)`;
  `pub fn admit(event: Event, mode: Mode) -> Option<Event>`.
- **Tests.** A composed play event serialises to WP-034's literal
  encoding with exactly the allowed fields. The same inputs give the
  same event. The clock advances as WP-034's `send` does. In private
  mode each droppable body returns none, and a love returns the event
  (api-needs.md lists plays, skips, completions and recommendation
  signals as dropped, not loves). Property: nothing admitted in private mode is
  a play, a skip or a completion.

### WP-242 QR code matrix (added for the client plan)

- **Wave** 2 · **Size** M · **Depends on** WP-005.
- **Owns** `crates/gunmetal-core/src/qr.rs`.
- **Serves** ACC-062, ACC-064, ACC-080 (the QR form of pairing,
  help-sign-in and invitation links).
- **Security.** Boundaries TB4; threats TM-T64. Verifies no requirement
  of its own: it holds no security control; what a code carries is
  decided by the packages that issue it (SEC-IAM-057, SEC-CLI-013).
- **Why it exists.** The web client shows invitations, pairing codes
  and the "help sign in" code as QR codes, and the server prints the
  claim link as one on the console (WP-080). No package owned an
  encoder, and the client plan may not add a rule in TypeScript. One
  encoder in the core gives both the same matrix; the client only draws
  squares.
- **Scope.** A pure encoder from a byte string to a matrix of dark and
  light modules, byte mode, with error correction level M and automatic
  version choice, capped at the size the R1 links need. No image
  output.
- **Not in scope.** Drawing (client; WP-080 for the terminal). Decoding.
- **Tests.** Literal matrices for short inputs, worked from the
  standard's own published examples (which examples are published is
  unverified); an independent decoder written in the test reads back
  every generated matrix; an input over the cap is a typed error.
  Property: encode then decode returns the input.
- **Risks.** A hand-written encoder is the choice here because the core
  takes no dependency without review; if the owner prefers a reviewed
  crate, it is a dependency request and this package shrinks to a
  wrapper.

## Wave 3: the music model, sign-in, streaming and sync

WP-063, WP-072, WP-073 and WP-074 also run in this wave (numbered in the
wave 2 section), and WP-120, WP-131 and WP-132 are at the end of this
section. WP-088, numbered here, is back in this wave; WP-101, numbered
here, runs in wave 4. WP-091 moved to R2, single sign-on (WP-096) to
R1.2 and the rule store (WP-092) to R1.3; their specifications are in
[After R1](#after-r1-point-releases-and-later).

### WP-075 Track record derivation

- **Wave** 3 · **Size** M · **Depends on** WP-021, WP-028, WP-049, WP-050,
  WP-051, WP-052, WP-053.
- **Owns** `crates/gunmetal-core/src/music/track.rs`.
- **Serves** MUS-021, MUS-034, MUS-037, MUS-084, MUS-154, MUS-155,
  LIB-059, LIB-097; API-CAT-01 to API-CAT-08. (The explicit flag,
  word-by-word lyrics and the "why" panel, MUS-047, MUS-156 and LIB-098,
  are R1.1 views over fields and provenance this record already
  carries.)
- **Security.** Boundaries TB6, TB9; threats TM-T07, TM-T20. Verifies
  SEC-MED-017.
- **Scope.** From a probe's `FileFacts`: run the mappers in a fixed
  precedence (for example ID3v2 over APE over ID3v1 in an MP3), merge into
  one `TrackTags` with sources (including ID3v2 tags embedded in WAV and
  AIFF chunks, mapped with WP-049's mapper), parse lyrics sources into the lyrics model
  (embedded first, then a matched `.lrc` sidecar, with the source shown),
  compute credits, attach technical, trim, gain and index data, and produce
  the synced track record with its provenance and its problems.
- **Interface sketch.** `pub fn derive_track(facts: &FileFacts, sidecars: &Sidecars, rules: &LibraryRules) -> Derived`.
- **Tests.** One whole expected record per format; conflicting values
  between tag formats resolved by the documented precedence and recorded;
  an `.lrc` sidecar and an embedded lyric both present; a malformed `.lrc`
  falls back to the embedded lyric with a problem recorded.

### WP-076 Album grouping (was: album and release grouping)

- **Wave** 3 · **Size** M (was L; release groups and grouping reasons
  moved to WP-146 in R1.1, and review items and curation overrides to
  WP-107 in R1.3) · **Depends on** WP-040, WP-053. (It no longer depends
  on WP-034: the curation bodies it applied are R1.3.)
- **Owns** `crates/gunmetal-core/src/music/albums.rs`.
- **Serves** MUS-011, MUS-012, LIB-045, LIB-051; API-CAT-02. (Release
  groups and editions, release types, original dates, one artist page
  across libraries and the reason for every decision, MUS-008, MUS-010,
  MUS-013, LIB-187 and LIB-098, are R1.1, WP-146; the review queue,
  LIB-099 and API-HLTH-03, is R1.3, WP-107; works and movements, MUS-014,
  are R2.)
- **Security.** Boundaries TB9; threats TM-T20. Verifies no requirement of
  its own: it holds no security control, and the rules it relies on are
  proved by the packages that own them.
- **Scope.** Group tracks into albums by MusicBrainz release ID, then by
  album artist and album title, erring on the side of not merging: when
  the rules are unsure, the tracks stay in separate albums; keep
  same-titled albums apart by release ID, year or track list; mark
  compilations and Various Artists; order discs and keep disc titles.
- **Not in scope.** Release groups, editions and the recorded reason for
  each decision (WP-146, R1.1). Review items and curation-log overrides
  (WP-107, R1.3).
- **Interface sketch.** `pub fn group(tracks: &[GroupInput]) -> Grouping`
  where `Grouping { albums }`; R1.1 and R1.3 extend the result with
  release groups, reasons and review items under the merge protocol's
  interface-change rule.
- **Tests.** An album split across two folders; two albums in one folder;
  "Disc 1" and "Disc 2" folders of one album (the Jellyfin #5605 case);
  two "Greatest Hits" by one artist with different years; a compilation
  with a different artist per track and no album artist; one track missing
  the album artist that every sibling has; an ambiguous pair the rules
  keep apart. Properties: grouping is independent of input order; every
  track is in exactly one album; tracks with different release IDs are
  never in the same album.

### WP-077 Content identity and scan diff

- **Wave** 3 · **Size** L · **Depends on** WP-006, WP-040, WP-052.
- **Owns** `crates/gunmetal-core/src/music/identity.rs`,
  `crates/gunmetal-core/src/music/diff.rs`.
- **Serves** LIB-016, LIB-017, LIB-028, LIB-029, LIB-030, LIB-032,
  LIB-033, INT-008, DIS-038; API-HOME-03, API-LIB-05. (The parser-upgrade
  re-read and moving a root, LIB-025 and LIB-031, are R1.1, WP-123 and
  WP-150, and reuse this diff unchanged.)
- **Security.** Boundaries TB9, TB10; threats TM-T56, TM-T60. Verifies
  SEC-TM-069.
- **Scope.** LIB-028's identity order as versioned rules (MusicBrainz
  recording ID; FLAC MD5 when set; the hash of the audio window; the path
  stem within the root), and the scan diff: given the previous index and fresh
  results for a set of paths, classify each file as unchanged, changed,
  added, removed, moved, or replaced by a better copy, keep "date added" at
  the first arrival, and never guess an ambiguous match (same tags and
  duration, new audio, new path): in R1, which has no review queue, the
  diff reports it as a removal and an addition, so the old item goes to
  the trash with its grace period (LIB-033) and nothing is merged; from
  R1.3 it is sent to the review queue instead (WP-107). Files under an offline
  root are never removed (SEC-TM-069). Public IDs are not derived here:
  the first draft derived them from content identity with a keyed MAC,
  but the baseline requires identifiers from the CSPRNG that are never
  derived from paths or names (SEC-HIS-012), and the identity order can
  fall back to the path stem. The diff reports each identity, and the
  scan (WP-102) looks up or mints its random public ID in the identity
  store's mapping (WP-046; owner decision 11).
- **Interface sketch.** `pub fn identity(facts: &IdentityInputs, rules: IdentityRules) -> Identity`;
  `pub fn diff(prev: &IndexView, fresh: &[Observed], scope: &PathScope, roots: &RootStates) -> Diff`.
- **Tests.** Retagging an MP3 keeps its identity (the window excludes the
  tags); moving a file keeps identity and history; renaming a folder of
  200 files gives 200 moves, not 200 removals and additions; an MP3
  replaced by a FLAC of the same recording with MBIDs is an upgrade; the
  same without MBIDs is a removal and an addition, never a merge; an
  offline root removes nothing.
  Properties: diffing an index against itself yields no changes (LIB-016);
  the diff of a scope never touches paths outside the scope (LIB-017);
  applying a diff and diffing again yields no changes.

### WP-078 Worker pool, deadlines and quarantine

- **Wave** 3 · **Size** M · **Depends on** WP-060, WP-061. Pausing a root
  after two lost workers appears in both WP-060 and this package; WP-060
  owns the root's paused state and this package reports lost workers to
  it, so the rule is implemented once.
- **Owns** `crates/gunmetal-worker/src/pool.rs`, `crates/gunmetal-worker/src/quarantine.rs`.
- **Serves** LIB-020.
- **Security.** Boundaries TB6; threats TM-T20, TM-T21, TM-T53. Verifies
  SEC-MED-018, SEC-MED-019, SEC-MED-021, SEC-MED-041.
- **Scope.** A pool of single-threaded workers (one per core by default,
  at least one), reused for many files; a per-file wall-clock deadline;
  killing and replacing a worker that misses it; abandoning a worker stuck
  in uninterruptible I/O; pausing a root after two lost workers; recording
  failures and quarantining a file after two consecutive crashes or
  timeouts until its size or modification time changes or an admin
  retries.
- **Interface sketch.** `pub struct Pool; impl Pool { pub async fn run(&self, job: Job, files: FileSet) -> Result<JobOutput, PoolError>; }`.
- **Tests.** With the test-hook feature: a worker that panics, one that
  aborts, one that loops forever, one that recurses until its stack
  overflows, and one that allocates past its limit; each is recorded with
  the right reason and the next file still processes. Two failures
  quarantine; a changed modification time lifts it.

### WP-079 Worker jobs: probe and artwork

- **Wave** 3 · **Size** M · **Depends on** WP-037, WP-045, WP-052, WP-061,
  WP-119, WP-128.
- **Owns** `crates/gunmetal-worker/src/jobs/` (`crates/gunmetal-worker/src/jobs/mod.rs`, `crates/gunmetal-worker/src/jobs/probe.rs`,
  `crates/gunmetal-worker/src/jobs/artwork.rs`, `crates/gunmetal-worker/src/jobs/hash.rs`),
  and from this wave `crates/gunmetal-worker/src/sandbox/syscalls.rs`,
  handed over by WP-045. Later job files in the same directory belong to
  the packages that add them: `jobs/package.rs` (WP-105) and
  `jobs/sqlite_check.rs` (WP-109) in R1, and `jobs/import_file.rs`
  (WP-145) and `jobs/playlist_file.rs` (WP-112) in R1.1;
  each registers its module in the `jobs/mod.rs` registry, adds one arm to
  this package's job dispatch under the merge protocol, and adds its job's
  syscalls to the allowlist in audit mode, reviewed by this package's
  owner.
- **Serves** LIB-134, LIB-135, LIB-136, LIB-142, LIB-143, MUS-039, MUS-040,
  MUS-110; API-SYNC-05, API-CAT-07. (The job decodes any allowed sidecar;
  attaching `artist.jpg` to the artist, MUS-024, is R1.1, WP-146.)
- **Security.** Boundaries TB6; threats TM-T21, TM-T28, TM-T29. Verifies
  SEC-MED-026, SEC-MED-044, SEC-MED-045, SEC-MED-046, SEC-API-086,
  SEC-TM-035, SEC-HIS-030, SEC-CLI-005.
- **Scope.** The jobs a worker runs: probe a file; hash an audio window for
  identity; decode artwork (embedded or a sidecar such as `cover.jpg`,
  `folder.jpg` or `artist.jpg`), accepting only JPEG, PNG, WebP and the
  first frame of a GIF, detected by content (SEC-MED-044), after checking
  dimensions from the header before any pixel buffer is allocated, with
  `image::Limits` set; for PNG, the inflated size is first bounded through
  the decompression helper (WP-128); then produce the fixed sizes as
  re-encoded JPEG (PNG where alpha matters) with EXIF, XMP, ICC and text
  chunks stripped, the placeholder and the palette from one decode
  (SEC-MED-046, SEC-API-086, SEC-TM-035, SEC-HIS-030). Two baseline rows
  set different image limits: SEC-MED-045 (16,384 pixels per side, 64
  megapixels, 32 MiB encoded, `max_alloc` 256 MiB) and SEC-API-086 (8,192
  pixels per side, 40 megapixels). The job applies the stricter of each
  (8,192 per side, 40 megapixels, 32 MiB encoded, `max_alloc` 256 MiB),
  which satisfies both. `image` is admitted only after the recorded review
  SEC-MED-026 requires.
- **Not in scope.** The derivative cache and serving (WP-103). Choosing
  which image is the album's cover (WP-102 applies LIB-136's order).
- **Interface sketch.** `pub fn run(job: Job, fds: Fds) -> JobResult` (inside the worker).
- **Tests.** A real worker process probes a synthetic file from WP-119
  passed by descriptor, and the result equals the in-process `drive` of
  the same probe (the integration WP-061 could not run). Images built in
  the test with the `image` crate's encoders, which are third-party code
  and so do not break the independent-oracle rule:
  a 1×1 PNG; a JPEG with an EXIF segment spliced in by hand as literal
  bytes (whether the encoder can write EXIF itself is unverified), which
  must be absent from the output; PNG, JPEG and WebP headers declaring
  65,535 × 65,535 and 8,193 × 100 pixels (refused before decoding, with
  peak allocation under 1 MiB); a PNG whose image data inflates past the
  cap (refused by the helper); an animated GIF (first frame only); an SVG,
  a TIFF and an HTML file named `cover.jpg` (refused by detection); a
  truncated JPEG (problem recorded, file still probed). Output dimensions
  are exactly the fixed set, and a property test shows no output carries
  an EXIF, XMP or text chunk. A fuzz harness covers the image pipeline.
- **Risks.** The seccomp allowlist from WP-045 is completed by running
  these jobs in audit mode; this package owns that file from this wave.

### WP-080 Owner claim and setup

- **Wave** 3 · **Size** M · **Depends on** WP-038, WP-062, WP-064, WP-069,
  WP-118. (It no longer depends on WP-063: the password branch is gone and
  recovery codes are offered at the passkey enrolment WP-106 adds.)
- **Owns** `crates/gunmetal-server/src/setup/` (except `setup/passkey.rs`,
  which WP-106 adds in wave 4).
- **Serves** ACC-001, ACC-002, ADM-018 to ADM-021, ADM-028, ACC-113;
  API-AUTH-01 to API-AUTH-03. (The locale, server-name and welcome-message
  steps, ADM-027 and ADM-140, are R1.2, WP-159; the "Coming from another
  server?" step, ADM-030, is R1.1 with the imports it offers, WP-145 and
  WP-112; the importer framework for other servers is R2, WP-229.)
- **Security.** Boundaries TB1, TB2, TB11; threats TM-T06, TM-T08, TM-T11.
  Verifies SEC-IAM-005, SEC-IAM-006, SEC-IAM-007, SEC-IAM-008, SEC-IAM-009,
  SEC-OPS-001, SEC-OPS-003, SEC-OPS-004, SEC-OPS-005, SEC-OPS-006,
  SEC-OPS-047, SEC-OPS-050, SEC-PRV-013, SEC-PRV-040, SEC-STD-029,
  SEC-IAM-013 (the claim's loopback exemption is never granted through a
  proxy).
- **Scope.** While no owner exists, answer only the setup page, its
  assets, the claim and restore-at-setup endpoints and a health check
  without version, and refuse every other route (SEC-IAM-006,
  SEC-OPS-003); accept no sign-in of any kind (SEC-OPS-001); ship and hold
  no default account or credential (SEC-IAM-005). Issue a 128-bit,
  single-use claim code valid for 24 hours, unchanged by a restart, shown
  only through host channels: the console and journal, a terminal QR code,
  a claim URL that carries the code in its fragment, and a file only the
  service user can read (SEC-IAM-007). Accept the code only from loopback
  or from a secure context on a configured origin with a recognised
  `Host`, never over plain HTTP from a LAN peer, compared in constant time
  and delayed per source through the credential verifier, with each
  failure reported on the console with its source (SEC-IAM-008,
  SEC-OPS-004). "Loopback" is the path class in the request's
  `ClientContext` from WP-118's resolver, never the socket peer read
  here: a reverse proxy on the same host makes internet requests arrive
  from loopback, and the resolver classifies such a peer as internet
  whenever it sends forwarding headers without being a declared trusted
  proxy, so the claim's loopback exemption cannot be reached from the
  internet (SEC-IAM-013, SEC-NET-016, SEC-NET-017). As first written,
  this package decided "loopback" in wave 3 beside WP-132, which then
  owned the resolver, and the case was deferred to WP-117. Bind an accepted code to the browser session that entered
  it (flows G1). The claim is one atomic transaction that consumes the
  code, creates the owner, enrols the owner's first credential and writes
  a marker file in the secrets directory (SEC-IAM-009, SEC-OPS-005,
  SEC-STD-029); after it, setup never becomes reachable again, not after a
  restart, a failed migration, a deleted identity store or removal of the
  last owner credential (SEC-OPS-006). Walk the R1 welcome steps (owner,
  the required question, privacy choices, first library); the point
  releases add their steps through the same step list (WP-145 the import
  offer in R1.1, WP-137 the provider choices in R1.1, WP-159 locale,
  server name and welcome message in R1.2). The required question cannot
  be skipped and has no preselected answer: the update and advisory
  check (SEC-OPS-047). The provider step: R1 has no metadata provider, so
  the privacy step says that nothing is looked up online and no provider
  is enabled (SEC-PRV-013, ACC-113); from R1.1, WP-137 turns it into the
  required question that lists each provider with the exact fields it
  would receive and its privacy policy, with "Turn on" and "Not now". The
  owner is created with a passkey, through the credential-enrolment
  interface; a page that is not a secure context gets no sign-in at all,
  so a home install claims from the server itself over `http://localhost`,
  or over HTTPS through the owner's own domain with an automatic
  certificate (WP-101), a tailnet name or the owner's reverse proxy
  (SEC-IAM-025, SEC-NET-001; owner answer to D-07). R1 has no project
  name service, so no claim URL names one. Setup issues the one printable recovery kit holding the
  owner's recovery codes and the backup recovery key, and the dashboard
  keeps a reminder until the owner confirms it is saved (SEC-PRV-040; the
  codes come from WP-063 through WP-106, the backup key from WP-090).
  Unauthenticated setup and error pages reveal no version, path or stack
  trace (SEC-OPS-050).
- **Tests (real SQLite).** Every non-setup route refuses while unclaimed; a
  fresh data directory holds zero accounts and credentials; a wrong code
  goes through the verifier's per-source delays, and a hostile source's
  failures do not delay a second source; the eleventh attempt in a minute
  is refused; a code from a plain-HTTP LAN peer or with a rebinding-style
  `Host` is refused; a code over plain HTTP from a loopback peer that
  sends `X-Forwarded-For` or `Forwarded` (a same-host proxy that is not
  declared trusted) is refused, the claim stays open, and the same code
  then succeeds from a bare loopback peer; 64 concurrent claims with the valid code give
  exactly one owner; a code used twice fails; a second browser cannot
  finish a claim the first started; after the owner exists, setup routes
  answer 404, including after a restart and after the owner row is
  deleted by fault injection; restarting before expiry shows the same
  code and after expiry a fresh one; setup cannot finish without an
  answer to each required question. Owner creation takes a
  credential-enrolment interface, tested here with a test enroller; the
  first draft's password-and-two-factor branch for pages that are not a
  secure context is removed (SEC-IAM-025, SEC-NET-001). WP-106 (wave 4)
  adds the passkey branch in `setup/passkey.rs` and tests it.
- **Risks.** Without a domain, a tailnet or a reverse proxy, a household
  claims from `localhost` on the server; the copy that explains the
  choices is the client's. The project name service that would have given
  every household an HTTPS name is R2 (WP-129, WP-135; owner answer to
  D-07), so R1's first run depends on the owner having one of the three
  paths, and the claim acceptance test runs each of them (WP-117). The
  first draft's risk note expected many home installs to set a password
  first, which the baseline rules out (baseline owner decisions 1 and 2).

### WP-081 Passkeys

- **Wave** 3 · **Size** L · **Depends on** WP-041, WP-047 (P-256
  verification through its crypto module), WP-062, WP-064, WP-118.
- **Owns** `crates/gunmetal-server/src/passkey/`.
- **Serves** ACC-050, ACC-055 (passkey part), ACC-003 (passkey sign-in
  needs nothing outside the house; WP-096, which also named it, is R1.2);
  API-AUTH-03, API-AUTH-04.
- **Security.** Boundaries TB1, TB4; threats TM-T03, TM-T47. Verifies
  SEC-IAM-018, SEC-IAM-019, SEC-IAM-020, SEC-IAM-021, SEC-IAM-022,
  SEC-API-058.
- **Scope.** Registration and authentication ceremonies with a relying-party
  ID fixed at setup, challenges of at least 128 bits from the CSPRNG that
  are single use, bound to the pre-session and valid for five minutes,
  discoverable credentials with user verification required, verification
  of type, challenge, origin, RP ID hash, flags and signature, the counter
  rule and its alert, and the backup-eligible and backup-state flags
  stored. The passkey sign-in route, usernameless, is the passkey pathway
  of the credential verifier (WP-064), so it reveals nothing about which
  accounts or credentials exist through content, status or timing
  (SEC-IAM-022, SEC-API-058); state for unauthenticated ceremonies lives
  in fixed-capacity stores (SEC-NET-051). A sign-in whose assertion has
  the UV flag set calls WP-062's `record_user_verification` with the
  passkey credential kind, which is what later satisfies fresh-uv routes.
- **Not in scope.** CBOR and COSE parsing (WP-041). Pairing devices (R2).
- **Tests (real SQLite).** Ceremonies constructed in the test with a
  software authenticator written for the test (a P-256 key pair signing the
  exact bytes the specification describes); a replayed challenge; a
  challenge from another pre-session; an expired challenge; the UV flag
  unset; a wrong origin; each verified field tampered in turn; a counter
  that goes backwards (alert raised); unknown, revoked and valid
  credentials compared as whole responses for the failure cases; 1,000
  forged assertions for a real credential run into the limiter.
  Recorded ceremonies from real platform authenticators, which the
  security baseline asks for, are added when available and are not
  synthetic media, so they need the owner's agreement (owner decision 9).

### WP-082 Stream URLs and byte serving

- **Wave** 3 · **Size** L · **Depends on** WP-023, WP-031, WP-047, WP-060,
  WP-062, WP-065, WP-067 (identity check against the index), WP-070 (the
  rescan request), WP-118.
- **Owns** `crates/gunmetal-server/src/stream/`.
- **Serves** MUS-066, MUS-070, ACC-122; API-STR-01 to API-STR-03. (The
  byte ranges also cover fetching ahead on patchy signal, CLI-099, an
  R1.1 client feature that needs nothing more from the server.)
- **Security.** Boundaries TB1, TB4, TB9; threats TM-T02, TM-T16, TM-T22,
  TM-T67. Verifies SEC-API-018, SEC-API-026, SEC-API-027, SEC-API-028,
  SEC-API-029, SEC-API-031, SEC-API-051, SEC-IAM-043, SEC-IAM-046,
  SEC-MED-012, SEC-MED-036, SEC-MED-059, SEC-OPS-055, SEC-TM-028,
  SEC-HIS-006, SEC-HIS-031, SEC-HIS-042.
- **Scope.** Sign short-lived, session-bound URLs for an item and a
  representation, with the lifetimes of SEC-API-027; a refresh route for
  silent renewal; the capability route, which authenticates only by the
  URL capability, ignores cookies and `Authorization` and never sets a
  cookie (SEC-API-029), and verifies in order (parse, MAC, expiry, session
  alive, object still visible) on every request, so revocation stops
  playback on the next range request (SEC-API-028, SEC-IAM-046,
  SEC-HIS-042); opens only the file path recorded at scan time, beneath
  its root, as a regular file, with its identity checked against the
  index, and only for items that content detection indexed as an
  allowlisted type (SEC-API-018, SEC-MED-012, SEC-MED-036, SEC-OPS-055);
  serves one byte range, answering two or more with 416 (SEC-API-031),
  with the `Content-Type` from the format detected at scan time (never
  the extension), `nosniff`, a restrictive CSP and `Cache-Control:
  private` no longer than the URL's remaining life (SEC-MED-059,
  SEC-API-051, SEC-HIS-031); caps each range response and the concurrent
  streams per session and per principal (SEC-API-031); tracks open
  responses per session and aborts them within 5 seconds when the epoch
  changes (SEC-IAM-043, SEC-TM-028).
- **Interface sketch.** `pub async fn sign(p: &Principal, item: PublicId, rep: Representation) -> Result<SignedUrl, ApiError>`;
  route `/m/v1/<token>`.
- **Tests (real SQLite and real files).** Full and ranged requests over a
  synthetic FLAC with exact bytes compared; `bytes=0-` capped; an expired
  token; a token for an item the session can no longer see, and a token
  naming an item in a library the session never could see, both answering
  exactly as an unknown item does; a file swapped
  on disk after scanning (refused, and a library-scan request for that
  path is queued with the task runner); a non-media file in a
  library is never served (a copy, a symlink and a hard link of a config
  file); a symlink that leads out of the root and one into the data
  directory are refused; revocation cuts an in-flight response (tested
  over a real TCP socket with a slow reader); a valid session cookie with
  an unsigned or wrongly signed URL fails, and a valid URL never sets a
  cookie; an MP3 with HTML appended is served with the scan-time type and
  `nosniff`; two ranges give 416 with no body; the stream above the cap
  gives 429; the "URL expired during playback is invisible" acceptance
  test from flows G10 at the server's level (refresh then continue from
  the same offset).

### WP-083 Client event channel

- **Wave** 3 · **Size** M · **Depends on** WP-062, WP-065, WP-118.
- **Owns** `crates/gunmetal-server/src/events/`.
- **Serves** api-needs.md Flags item 12; LIB-021, ACC-069, MUS-122 (the
  active player changing); API-SYS-10. (The admin live view's events and
  the "stopped by the owner" message, ADM-099 and ADM-102, are R1.2,
  WP-153, and handing playback to another device, CLI-103, is R1.1; both
  publish on the bus and this channel forwards them unchanged.)
- **Security.** Boundaries TB4; threats TM-T09, TM-T15, TM-T16. Verifies
  SEC-API-016, SEC-API-017, SEC-API-041, SEC-API-042, SEC-API-043,
  SEC-IAM-016, SEC-IAM-043, SEC-NET-020, SEC-TM-028.
- **Scope.** One authenticated server-sent events stream per signed-in
  session that carries sync nudges, scan progress, a stopped session, a
  revoked device and the active player changing; each event built per
  recipient through the authorisation layer, with no broadcast-to-all path
  (SEC-API-016); heartbeats; the stream closes within 5 seconds when the
  session's epoch changes (SEC-API-017, SEC-IAM-043, SEC-TM-028). The
  baseline's rules for WebSocket upgrades apply to this stream in the form
  that fits it: the credential is checked before anything is sent, a
  cookie-authenticated stream is refused unless `Origin` is one of the
  server's own origins, connections per principal are capped and idle
  ones closed (SEC-IAM-016, SEC-API-041, SEC-API-043, SEC-NET-020). R1
  registers no WebSocket route, which a route-table test asserts, so the
  WebSocket-only rules (the first-message ticket of SEC-API-042, the 64
  KiB message cap) are verified here as "no upgrade is accepted" and
  again by WP-219 when the R2 control channel adds one. Other modules publish to the
  in-process bus (WP-043) and this package forwards; so WP-085, WP-104 and
  the scan publish without depending on this package, and their tests
  assert the bus event.
- **Not in scope.** The R2 control channel (WebSocket, two-way).
- **Tests.** A recipient who cannot see a playlist does not receive its
  change event, with several principals connected; a revoked session's
  stream ends with a final "signed out" event within 5 seconds; a foreign,
  `null` or missing `Origin` on a cookie-authenticated stream is refused;
  an upgrade request to any path is refused; a slow consumer is dropped
  rather than buffering without bound; the per-principal connection cap.
- **Risks.** Server-sent events instead of a WebSocket is owner decision 7.

### WP-084 Sync snapshot and delta

- **Wave** 3 · **Size** L · **Depends on** WP-039, WP-065, WP-066, WP-067,
  WP-068, WP-118, WP-119.
- **Owns** `crates/gunmetal-server/src/sync/`.
- **Serves** CLI-022, ACC-030, ACC-037, DIS-002, DIS-140, MUS-208, MUS-071
  (seek index fetch), LIB-146 (the technical and sort fields in the
  feed); API-SYNC-01 to API-SYNC-04, API-DEV-02, API-CAT-05 (seek index).
  (Offline loading and sync status, CLI-024 to CLI-026, and the views and
  filters over synced fields, DIS-102 and MUS-056, are R1.1 client
  features over this feed; old native clients, CLI-032, are R2.)
- **Security.** Boundaries TB4; threats TM-T15. Verifies SEC-API-014,
  SEC-API-015, SEC-CLI-020, SEC-TM-026, SEC-PRV-016 (the sync part: every
  image URL in a payload is server-relative).
- **Scope.** The snapshot (the profile's catalogue, filtered by grants as it
  is built, plus the profile's slice of the user log and the server facts)
  as a stream of wire frames; the delta from a cursor; grant changes sent
  as a fresh snapshot of the affected library (the api-needs proposal,
  because a missed removal is a disclosure); per-device cursor and last
  sync time; the payload budget rules for seek indexes, lyrics and history
  (owner decision 15), including the route that fetches one track's seek
  index when it enters the queue, if the owner accepts that
  recommendation.
- **Tests (real SQLite).** Two profiles with different grants receive
  different snapshots, compared as whole decoded payloads against literal
  expected payloads for a small library from WP-119; a grant removal
  produces a resnapshot that omits the library; a delta after
  re-applying an identical catalogue batch is empty (the scan itself is
  WP-102, a wave later); an old cursor gets "resnapshot"; a cursor
  from an older cache generation gets "resnapshot"; a seek index for a
  track in an ungranted library answers as unknown; when an item becomes
  invisible, the delta carries only its identifier as a removal, never its
  metadata (SEC-API-015); no payload holds another person's history,
  queue or settings (SEC-CLI-020); every image URL in a snapshot and a
  delta is server-relative (SEC-PRV-016). Property: for a generated
  library and subject, the snapshot and every delta are subsets of what
  the policy allows that subject (SEC-TM-026). Snapshot bytes and time at
  100,000 synthetic tracks are held to the DIS-019 budget by WP-115's
  budget tests in the R1 gate (register D-87), not by this package's own
  tests.

### WP-085 Queue service

- **Wave** 3 · **Size** M · **Depends on** WP-025, WP-026, WP-065, WP-068,
  WP-118.
- **Owns** `crates/gunmetal-server/src/queue/`.
- **Serves** MUS-116 to MUS-119, MUS-122, MUS-123, MUS-126; API-QUE-01 to
  API-QUE-04, API-SES-03. (Reordering while shuffled, shuffle by album and
  reshuffle the rest, MUS-120, MUS-127 and MUS-128, are R1.1, WP-142; save
  queue as playlist, MUS-125, is R1.1, WP-143; handing playback to another
  device, CLI-103, is R1.1 and client only over the active-device event
  below; undo and queue history, MUS-121 and MUS-124, are R2.)
- **Security.** Boundaries TB4; threats TM-T17. Verifies SEC-HIS-014.
- **Scope.** Store each profile's queue as operations and snapshots in the
  user log; accept operations against a version and return the new
  version or the stale-version rejection; take coalesced position updates;
  set the active device when a device presses Play and publish that on the
  in-process bus, which the event channel (WP-083, same wave) forwards so
  the other device pauses (owner decision 14). A device can make itself
  the active player only for its own person's queue; no route lets one
  person drive another's queue or player, and queue operations are a
  closed set with no free text (SEC-HIS-014). "Save
  queue as playlist" (R1.1) is not here: the client already holds the
  queue and sends its items to WP-093's create route (WP-143).
- **Tests (real SQLite and log files).** Two clients racing on one version:
  one wins, the other is rejected with the current version; positions
  arriving every second are written at most once per batch window, with a
  manual clock; Play from a second device publishes exactly one
  active-device event on the bus; replaying the log into an empty cache
  restores the exact queue (the rebuild orchestration is WP-095, in the
  same wave, so this package provides and tests its own projection
  rebuilder, which WP-095 calls).

### WP-086 Listening activity service

- **Wave** 3 · **Size** M · **Depends on** WP-034, WP-065, WP-068, WP-118.
- **Owns** `crates/gunmetal-server/src/listening/`.
- **Serves** MUS-180, MUS-182 to MUS-185, MUS-109 (love events), ACC-117,
  ACC-118, DIS-045, DIS-046, DIS-050 to DIS-052, MUS-149 (loves as a
  list), CLI-093; API-LOG-01 to API-LOG-04, API-SES-04, API-USR-07.
  (Ratings, MUS-181 and DIS-047, and dismissals with their Hidden page,
  DIS-022, DIS-023 and API-LOG-05, are R1.1, WP-141; the watchlist rows,
  DIS-048 and DIS-049, are R2.)
- **Security.** Boundaries TB4, TB10; threats TM-T18. Verifies SEC-PRV-022,
  SEC-PRV-024, SEC-HIS-060, SEC-TM-054.
- **Scope.** One write path for plays, skips, loves and their reversals,
  and play removals, which WP-141 extends with ratings and dismissals in
  R1.1; idempotent by event ID; plays queued
  offline accepted with their real timestamps and merged; the private
  session flag that drops play and recommendation events, starts from the
  player in at most two interactions and hides the title from the admin
  live view (SEC-PRV-024; with the proposed expiry after a period without
  playback, owner decision 27); derived counts projected into the cache.
  One person's plays, loves and queue are never returned to
  another person, and no route offers them to another non-admin user or
  in a notification (SEC-PRV-022, SEC-HIS-060).
- **Tests (real SQLite and log files).** The same offline batch uploaded
  twice is counted once; a removal hides the play from counts and history
  whatever the order; a private session records nothing; loves from two
  devices resolve to the later clock; replaying the log into an empty
  cache through this package's projection rebuilder gives the same counts
  and loves, compared with literal expected values.

### WP-087 Accounts, profiles, settings and devices

- **Wave** 3 · **Size** M · **Depends on** WP-062, WP-065, WP-068, WP-118.
- **Owns** `crates/gunmetal-server/src/account/`.
- **Serves** ACC-012, ACC-017 (profiles, each with the display name its
  account was given), ACC-068 to ACC-070, ACC-114 (the per-profile
  activity-visibility setting), ACC-115; API-SYS-05, API-USR-01 (names),
  API-USR-02, API-DEV-01. (A chosen name and picture for each profile,
  ACC-011, is R1.1; WP-144 adds the pictures. The settings records also carry the R1.1
  client settings, CLI-030, DIS-103 and MUS-158, and the R1.1 saved
  filters, DIS-105: a saved filter is a named view stored as a versioned
  rule document in the core rule format, which WP-027 delivers in R1.1
  (register D-85) and which validates every document under its parser
  budgets before it is stored; the smart playlists and rule editor that
  read the same documents are R1.3. Home layouts and pins, DIS-003, DIS-007, DIS-013, API-HOME-01 and
  API-HOME-02, are R1.2, WP-154. The tools' "who am I" check, INT-026, is
  R2.)
- **Security.** Boundaries TB4, TB5, TB11; threats TM-T18, TM-T38. Verifies
  SEC-IAM-042, SEC-IAM-077, SEC-IAM-102, SEC-IAM-104, SEC-PRV-023,
  SEC-PRV-027, SEC-PRV-033, SEC-HIS-033, SEC-TM-054.
- **Scope.** Who am I; profile names; settings records with person and
  device scope (latest wins per key), where every privacy setting
  defaults to its most private value and no feature that sends activity
  off the server is on for anyone by default or can be turned on by an
  admin for someone else (SEC-PRV-023, SEC-PRV-033); the list of the
  person's own sessions and
  devices with client name, device class, network type, coarse location
  and last use, revoke one, or all but the current one (SEC-IAM-042;
  calling WP-062's epoch bump); device names and other client-reported
  metadata validated to a bounded length and printable characters and
  stored as untrusted text (SEC-HIS-033); the limit on enrolled devices
  per account from the limits register (SEC-IAM-102); and the "what your
  admin can see" page, generated from the same policy table the server
  enforces (SEC-IAM-104, SEC-PRV-027, SEC-TM-054). An administrator's
  access to another person's data is written to that person's own
  security log (SEC-IAM-077).
- **Not in scope.** Profile pictures, which need the artwork job's
  re-encode (WP-144 adds the upload route in R1.1). Home layouts and pins
  (WP-154, R1.2).
- **Tests (real SQLite).** A device-scope setting does not follow the
  person to another device; a person-scope one does; revoking a device
  ends its session immediately and its stream URLs fail on the next
  request; another person's device cannot be revoked (404, not 403); a
  device name with control and bidirectional characters is stored
  bounded and returned as text; every privacy setting reads its most
  private value on a fresh account; a snapshot test shows the "what your
  admin can see" page changes when the visibility policy changes.

### WP-088 WASM facade

- **Wave** 3 (moved back from 4: it was in wave 4 because WP-059 had
  moved to wave 3, and WP-059 is back in wave 2 now that the radio and
  rule packages it read, WP-058 and WP-027, left R1) · **Size** M ·
  **Depends on** WP-039, WP-040, WP-054, WP-055, WP-059, WP-235. (On
  2026-10-03 the facade was split at the client plan's request: WP-235
  creates the crate in wave 2, with the catalogue types, WP-236 exports
  the wave 1 playback modules in wave 2 after it, and WP-237 exports the
  remaining conversions in wave 3. This
  package no longer creates the crate and no longer depends on WP-021,
  WP-025, WP-026, WP-028 or WP-030.)
- **Owns** In `crates/gunmetal-wasm/src/`, one file per module it
  exports (`sync.rs`, `library.rs`, `search.rs`, `decision.rs`,
  `home.rs`, `decode.rs`), with their generated declarations under
  `crates/gunmetal-wasm/types/`; `.github/workflows/wasm.yml` (the size
  check and the `wasm32` job of its own; the `wasm32` build step in the
  gate is WP-235's request to the integrator).
- **Serves** ADR 1 decision 2; CLI-022, DIS-084, DIS-002, MUS-208,
  MUS-236 (the details summary, WP-055).
- **Security.** Boundaries TB4; threats TM-T62. Verifies SEC-MED-077,
  SEC-CLI-021.
- **Scope.** The rest of the thin `wasm-bindgen` layer, in the form
  WP-235 sets (mirror types, generated declarations, the lint
  exception): apply sync frames to an in-memory library; load that
  library from catalogue records too, which is what applying a decoded
  frame does and what the web client's demo and tests use; read it (an
  item by ID, a list in a named order), returning WP-235's catalogue
  mirrors; query search; evaluate Home rows; read the decision engine
  (with its track details summary); and decode route responses as well
  as sync frames, with a mirror type per response, so the web client
  holds no hand-written copy of one (SEC-CLI-021). Every exported
  function is a direct call into the core with conversion only.
- **Not in scope.** Creating the crate, the type mechanism, the lint
  exception and the catalogue mirrors (WP-235). The queue, shuffle,
  gain with its factor, player state and lyrics (WP-236). User events,
  inbound links, tints and the QR matrix (WP-237). The audit-head check
  (WP-241). Persistent storage in the browser (client). Rule evaluation
  and radio picks, which the later packages that build them (WP-027 in
  R1.1, WP-058 in R1.3) add to this facade, one function each.
- **Tests.** Native unit tests of each conversion (the facade must compile
  and be covered on the host target); a `wasm32` build in CI; a size check
  of the `.wasm` file reported.
- **Risks.** Coverage tooling on the facade's generated glue may count code
  the tests cannot reach (unverified); WP-235 meets that first and records
  the answer. The `unsafe` question (owner decision 34) is answered and
  proved by WP-235.

### WP-089 Server facts: health, discovery, negotiation and API reference

- **Wave** 3 · **Size** M · **Depends on** WP-039 (protocol versions),
  WP-044, WP-062, WP-065, WP-118, WP-239 (the inbound-link parser, split
  out on 2026-10-03 so the web client can reach it a wave earlier).
- **Owns** `crates/gunmetal-server/src/meta/`.
- **Serves** ADM-128, INT-001, INT-005, INT-013, ADM-090, ADM-001;
  API-SYS-01, API-SYS-03, API-SYS-04, API-SYS-06, API-SYS-09, API-SET-09,
  API-TOK-03 (resolution / R1's links). (A custom server name, ADM-140, and deep links
  into the app, CLI-034 and INT-147, are R1.2, WP-159, which extends the
  parser's route set; published footprint numbers, ADM-010, are R1.2
  documentation from WP-115's measurements; old native clients, CLI-032,
  are R2.)
- **Security.** Boundaries TB1, TB2, TB4; threats TM-T02, TM-T10. Verifies
  SEC-API-005, SEC-NET-046, SEC-NET-047, SEC-OPS-059, SEC-SUP-031,
  SEC-CLI-025. (SEC-API-091 moved to WP-118 with the OpenAPI generation.)
- **Scope.** A health route that returns only liveness, and no metrics,
  profiling or debug endpoints, which are off by default and absent from
  release builds (SEC-NET-046, SEC-OPS-059); protocol negotiation;
  capability discovery scoped to the caller; `GET /api/v1/server`, which
  returns to an unauthenticated caller only the supported protocol-version
  range, the instance identifier, whether the server is claimed and the
  enabled sign-in methods, and never the software version, operating
  system, server name, user names or library statistics (SEC-API-005,
  SEC-NET-047); the API reference route, which serves the OpenAPI
  description WP-118 generates from the route table (generation, the CI
  diff and the release-build check moved to WP-118 in the second
  security pass, because WP-131's API fuzzing job in this same wave
  needs the description and could not depend on this package); server identity
  and about (name, version, build, the source repository and exact
  commit, storage locations, live footprint) for signed-in callers only
  (SEC-SUP-031); deep-link resolution that never grants access by itself,
  with every inbound link, QR payload and fragment parsed by one pure
  function in the core into a closed set of typed routes (SEC-CLI-025).
  In R1 the set holds the links R1 issues: the claim link, invitations,
  browser pairing and recovery enrolment links.
- **Tests.** The exact JSON of unauthenticated `GET /api/v1/server` against
  a literal schema with no additional properties, and no `Server` or
  `X-Powered-By` header on any route (the first draft let unauthenticated
  discovery reveal the version, which SEC-API-005 forbids); the health
  route's body is liveness only; the API reference lists exactly the
  routes in the table; an old client gets the "update needed" response
  with the protocol range it needs, not a software version; a link the
  parser (WP-239) recognises resolves to its route and grants nothing by
  itself. The parser's own unit, property and fuzz tests are WP-239's.

### WP-090 Backups and verification

- **Wave** 3 · **Size** L · **Depends on** WP-046, WP-047, WP-062 (the
  fresh-uv check), WP-068, WP-069, WP-070, WP-118, WP-138 (the
  retention schedule). (It no longer depends on WP-071: the derived-data
  store arrives in R1.1, and R1 has nothing in it to leave out.)
- **Owns** `crates/gunmetal-server/src/backup/`,
  `crates/gunmetal-secrets/src/export.rs` (a sealed export of the server
  keys for the backup, so the root secret is never exposed outside the
  secrets crate; one `pub mod` line in that crate's `lib.rs`).
- **Serves** ADM-065, ADM-066, ADM-069, ADM-072, ACC-013; API-SET-04.
- **Security.** Boundaries TB10; threats TM-T57, TM-T59. Verifies
  SEC-OPS-024, SEC-OPS-041, SEC-OPS-042, SEC-OPS-043, SEC-OPS-045,
  SEC-OPS-075 (the checkpoint export), SEC-IAM-105, SEC-TM-052,
  SEC-PRV-039, SEC-PRV-040, SEC-PRV-041.
- **Scope.** A daily backup by default of all irreplaceable state: the
  identity store (through the online backup API), the user log, the audit
  log, configuration and server keys, from a consistent snapshot
  (SEC-OPS-041), with a manifest and the latest signed audit checkpoint
  (SEC-OPS-024); encrypted in age v1 to the server's backup key and the
  owner's recovery key, with the secret-store key only in wrapped form,
  including automatic backups kept on the host (SEC-OPS-042, SEC-PRV-039,
  SEC-IAM-105, SEC-TM-052); signed with the server's backup-signing key
  (SEC-OPS-043); verified after writing by reading it back; kept for the
  period in the one retention schedule (WP-138), 14 days by default, and
  deleted on schedule (the first draft kept a configured count;
  SEC-PRV-041); contents described plainly; never written under a
  web-served path or into a media root. The audit log's signed
  checkpoints, read through WP-069's checkpoint-head reader, can be
  exported by the owner to a second backup destination and as a
  printable checkpoint sheet for the recovery kit, so the log has an
  anchor off the host (SEC-OPS-075). Downloading a backup is an
  owner-only, fresh-uv action, enforced by WP-062's check, that is
  audited and alerted (SEC-OPS-045); uploading one is restore's (WP-109). The
  backup recovery key is generated here and handed to setup's recovery kit
  (SEC-PRV-040).
- **Not in scope.** Restore (WP-109). The server export in documented
  formats (ADM-074), which moved to WP-108 in review and is now R1.2
  (WP-158). Leaving the derived-data store out of backups unless chosen
  (ADM-141), which WP-071 adds when the store arrives in R1.1.
- **Tests (real SQLite and files).** A backup taken while writes continue is
  consistent; a backup with one flipped byte fails verification; the
  manifest lists every file with its hash; retention deletes archives
  older than the configured period on a manual clock; the backup bytes
  contain no plaintext
  canary and never the raw root secret; the backup decrypts with either
  the server's key or the recovery key alone; a member, an admin and an
  owner without fresh user verification are each refused the download,
  and the owner's download is audited and alerted; a checkpoint exported
  to a second destination matches the head WP-069 reports, and a log
  re-signed and truncated afterwards no longer extends it.
- **Risks.** The encryption format is fixed by the baseline as age v1
  (SEC-OPS-042); the archive container inside it is recorded by WP-125
  and remains part of owner decision 12.

### WP-093 Manual playlists (was: manual playlists and pins)

- **Wave** 3 · **Size** M · **Depends on** WP-065, WP-067, WP-068, WP-118.
- **Owns** `crates/gunmetal-server/src/playlists/`.
- **Serves** MUS-132, MUS-133; API-PL-01, API-PL-08. (The duplicate
  warning, sorting and searching inside a playlist, automatic covers,
  pinning and loving playlists and saving the queue as a playlist,
  MUS-134, MUS-135, MUS-137, MUS-139, MUS-125, API-PL-02 and API-QUE-05,
  are R1.1, WP-143; read-only folder playlists are R1.1, WP-112; the
  tools' write API, INT-138 and API-PL-07, is R2 with WP-091.)
- **Security.** Boundaries TB4; threats TM-T12. Verifies SEC-API-012.
- **Scope.** Create, rename, add, reorder and remove, as log operations on
  entries with their own IDs that refer to tracks by content identity;
  concurrent adds both kept, a remove beats a concurrent move, reorders
  relative to neighbours; missing entries kept with their last known
  title (owner decision 29); adding several items in one request (the
  add-to-playlist sheet with a multi-item choice), authorising every item
  in the list and rejecting the whole request if one is not visible
  (SEC-API-012). The write API for tools under a scoped token moved to R2
  with WP-091.
- **Not in scope.** The R1.1 playlist features (WP-143) and the read-only
  flag for folder playlists (WP-112, R1.1), both of which extend this
  module after R1.
- **Tests (real SQLite and log files).** Two adds at once keep both; a
  remove and a move at once keep the remove; a track removed from the
  catalogue (the test removes the row; the trash and purge are WP-111, two
  waves later, which repeats the case end to end) becomes a "missing"
  entry and rematches when a row with the same identity returns; an add
  of several items with one invisible item is rejected whole and changes
  nothing.

### WP-094 Users and invitations

- **Wave** 3 · **Size** L (was M; invitations gained the pending
  confirmation and the privacy notice) · **Depends on** WP-062, WP-064
  (invitations are a pathway of the credential verifier), WP-065, WP-069,
  WP-118. (The first draft also listed WP-063 for passwords, which no
  longer exist.)
- **Owns** `crates/gunmetal-server/src/users/`.
- **Serves** ACC-005, ACC-006, ACC-008, ACC-080, ADM-052; API-ADM-01 to
  API-ADM-03. (Several administrators, ACC-040, are R1.2: WP-152 adds
  making an administrator. In R1 the owner is the only administrator.)
- **Security.** Boundaries TB4, TB11; threats TM-T13, TM-T14, TM-T65.
  Verifies SEC-IAM-003, SEC-IAM-044, SEC-IAM-073, SEC-IAM-078, SEC-IAM-079,
  SEC-IAM-080, SEC-IAM-103, SEC-API-058, SEC-API-096, SEC-NET-036,
  SEC-PRV-026, SEC-PRV-031 (the invite landing, the R1 page reachable
  without signing in that names a person; share pages join in R1.2 with
  WP-134), SEC-PRV-053, SEC-STD-029.
- **Scope.** List users with their libraries; enable and disable without
  deleting, a disable ending the account's sessions at once
  (SEC-IAM-103); end any or all sessions of any non-owner account
  (SEC-IAM-044); hand over ownership by a transfer that
  the current owner and the recipient each confirm with user verification
  in the previous 5 minutes (the route carries the fresh-uv tag that
  WP-062 enforces), so exactly one account holds the owner role
  at all times (SEC-IAM-003; the first draft handed over by adding an
  admin and removing yourself, which leaves a window with no owner);
  invitations carrying a secret of at least 128 bits in the URL fragment,
  sent to the server in a `POST` body to redeem, with an expiry of 7 days
  and a use limit of 1 by default, revocable until used, and a preset no
  greater than the inviter's own capabilities and grants (SEC-IAM-078,
  SEC-IAM-073, SEC-API-096, SEC-NET-036); redemption enrols the invitee's
  own passkey in the same transaction (an OIDC link as well from R1.2,
  WP-096), never a shared password, consumes the invitation with one conditional update
  (SEC-STD-029), and, for an invitation that gives member rights or more
  than one library, leaves the account pending with no grants until the
  inviter confirms it after comparing a short matching code (SEC-IAM-079);
  the guest preset has no household-device access, no admin capabilities,
  no view of other accounts and only the invited library (SEC-IAM-080);
  before redeeming, the invitee sees a privacy notice generated from the
  server's actual configuration (SEC-PRV-053); the link and QR payload
  with the address it will carry and a warning when it is private (flows
  G7); the invite landing that reveals nothing else: no user name, other
  users, library size or activity, with `X-Robots-Tag: noindex` and no
  link-preview metadata (SEC-PRV-031); a wrong, expired, used
  or revoked invitation gives one indistinguishable failure through the
  credential verifier (SEC-API-058); redemption logged. No route lets an
  admin sign in as another person, and a change an admin makes to
  someone's credentials, role or access is shown to that person
  (SEC-PRV-026).
- **Tests (real SQLite).** An invite used past its count or expiry, or
  revoked, fails identically to a wrong code; 64 concurrent redemptions
  of a one-use invite give exactly one account; a pending account holds no
  grants until confirmed, and a wrong matching code fails; a guest
  single-library invitation completes in one step; an invite whose preset
  exceeds the inviter's rights is refused; a disabled user's sessions end
  at once; the owner cannot be demoted, disabled or deleted, and a
  transfer without both fresh confirmations fails (property: no operation
  sequence yields zero or two owners); an admin's attempt on the owner's
  sessions is refused; the anonymous invite landing, compared whole,
  names no user and carries `X-Robots-Tag: noindex` and no Open Graph
  tags; the log canary shows the invitation secret never reaches a server
  log.

### WP-095 Startup page, snapshots and cache rebuild

- **Wave** 3 · **Size** M · **Depends on** WP-042, WP-046, WP-068, WP-069,
  WP-070, WP-118. (It no longer depends on WP-071: the derived-data store
  arrives in R1.1, and WP-071 then registers it as a rebuild input.)
- **Owns** `crates/gunmetal-server/src/startup/`.
- **Serves** ADM-032, ADM-056 to ADM-059, ADM-077; API-SYS-07, API-LIB-06.
  (Restart and shut down from the UI, ADM-112 and API-SET-10, are R1.2,
  WP-155; the derived-data store kept across rebuilds, ADM-141, arrives
  with WP-071.)
- **Security.** Boundaries TB10; threats TM-T60. Verifies SEC-IAM-004,
  SEC-TM-051, SEC-OPS-048, SEC-OPS-050.
- **Scope.** The page the server renders itself while it starts, migrates,
  rebuilds or restores, before the database opens; the pre-upgrade
  snapshot and the migration check; rebuilding the cache from the files
  and the log on request or when an older binary meets newer data, by
  calling the projection rebuilders that modules register (in R1 the
  queue, listening and playlists; rules and curation register theirs in
  R1.3) and requesting a full library scan from the task runner.
- **Tests (real SQLite).** Starting a binary whose cache digest differs
  discards the cache, calls every registered rebuilder in order (a fake
  rebuilder registered by the test records its calls and the stream it
  replayed) and queues one full scan; the startup page answers while the
  rebuild runs; an identity-store file from a newer version stops the
  start with the restore command shown. As first written, the test
  expected the queue and loves to come back identical, which needs WP-085
  and WP-086 (same wave) and the scan (WP-102, a wave later); each
  projection's replay is tested in its own package, and the whole round
  trip is in WP-117.
- **Added for the client plan (2026-10-03).** The startup page uses the small
  stylesheet with no script that the client plan's token package writes
  for server-rendered pages (CP-004), once it exists, so the page shares
  the design tokens. Until then it uses its own minimal styles.

### WP-097 Alerts, log rotation and free space (was: alerts, log rotation, free space and crash records)

- **Wave** 3 · **Size** M · **Depends on** WP-048, WP-062 (revoking a
  device and bumping its epoch), WP-069, WP-070, WP-118, WP-138 (the
  retention schedule).
- **Owns** `crates/gunmetal-server/src/ops/`.
- **Serves** ADM-083, ADM-116, ADM-119, ADM-129; API-SET-02 (network
  activity page), API-SET-07. (Local crash records, ADM-130, are R1.2,
  WP-155.)
- **Security.** Boundaries TB8, TB11; threats TM-T16, TM-T61. Verifies
  SEC-OPS-031, SEC-OPS-032, SEC-OPS-033 (the action revokes the device or
  credential, ends its sessions and bumps the epoch; that the next range
  request then fails is WP-117's), SEC-OPS-034, SEC-IAM-098,
  SEC-HIS-063, SEC-PRV-030, SEC-PRV-045.
- **Scope.** In-app owner alerts, the only destination in R1 (outbound
  alert channels such as email and webhooks are R2 and Later in the
  baseline's release scope, SEC-OPS-035; the first draft offered "others
  through the egress gate" in R1). Alerts at least for every trigger
  SEC-OPS-032 lists, never for re-authentication with an existing
  credential on a known device, with non-critical alerts batched into a
  daily summary; alerts for a new admin, a new credential or device on an
  owner or admin account, owner recovery, internet exposure and an audit
  integrity failure can never be switched off or suppressed
  (SEC-OPS-034); every alert about a device or credential offers a
  one-step "This wasn't me" that revokes it, ends its sessions and makes
  its stream URLs fail (SEC-OPS-033). Notices to a person's own devices
  for a new device, a new or removed credential, recovery use, a role
  change and bursts of failed attempts, and to every admin when an admin
  is created or a role is raised (SEC-IAM-098, SEC-HIS-063); no
  notification carries another person's activity or, unless the
  recipient chose it, a title (SEC-PRV-030). At startup, a configuration
  change made outside the server writes an audit event with the old and
  new value of every security setting that changed and alerts the owner
  (SEC-OPS-031). Log rotation and deletion on the one retention schedule
  (WP-138), with log files readable only by the service account
  (SEC-PRV-045); the
  free-space guard that alerts and refuses writes that would fill the
  disk; the network activity page reading the egress record.
- **Tests.** One integration test per alert trigger: exactly one alert with
  the expected recipients and message key, and none for a known device
  re-authenticating; a flood of failed sign-ins followed by a new admin
  still raises the new-admin alert; "This wasn't me" revokes the device,
  ends its sessions and bumps its epoch, asserted through WP-062's API
  (the stream route is WP-082's, in this same wave, so the "next range
  request fails within one URL lifetime" half runs in WP-117); a configuration file edited
  between runs gives the audit event and the alert; a notification
  template cannot take another person's activity (type-level test). Free
  space simulated through an injected probe: an alert at the threshold,
  and the guard's decision refusing a write below the floor; rotation and
  deletion at size and age on a manual clock. Mutation testing covers
  every threshold.
- **Gap.** The guard decides, but the writers that fill a disk (the cache
  writer, the user log, backups, the artwork cache, uploads) belong to
  other packages, several already merged by this wave. Which of them must
  consult the guard, and who adds that call, is not assigned (see Review
  notes).

### WP-098 Change-detection triggers

- **Wave** 3 · **Size** M · **Depends on** WP-060, WP-065, WP-070, WP-118.
- **Owns** `crates/gunmetal-fs/src/watch.rs`,
  `crates/gunmetal-server/src/triggers/`.
- **Serves** LIB-013, LIB-014, LIB-015, LIB-017, LIB-012 (an admin's
  refresh of one folder); API-SCAN-02, API-CAT-13 (rescanning one item is
  an admin's path-scoped refresh of its folder, since WP-107, which first
  served it, is R1.3). (Refresh by tools under a scoped key, INT-011, is
  R2 with WP-091.)
- **Security.** Boundaries TB9; threats TM-T09. Verifies SEC-API-064.
- **Scope.** Watch local disks with debounce, poll shares and cloud mounts
  at the configured interval and parallelism, run the scheduled safety-net
  scan, accept path-scoped refreshes from admins (scoped tokens moved to
  R2 with WP-091), and turn every trigger into a de-duplicated path set
  for the task runner, so a repeated request joins the running one rather
  than starting another (SEC-API-064); warn when the OS watch limit is
  too low.
- **Tests (real filesystem).** Ten files written in a burst give one
  request; a rename gives a request for both directories; a watch limit
  reported low by an injected probe produces the warning; a path outside
  every root is refused; a member's refresh request answers 404; many
  concurrent refreshes of one path give one job.

### WP-099 Library administration and grants

- **Wave** 3 · **Size** M · **Depends on** WP-024, WP-060, WP-062 (the
  fresh-uv check), WP-065, WP-067, WP-070, WP-118.
- **Owns** `crates/gunmetal-server/src/libraries/`.
- **Serves** LIB-001, LIB-003 to LIB-005, LIB-007, LIB-011, ADM-025,
  ADM-089, ACC-037, MUS-027, MUS-035 (rules storage); API-LIB-01 to
  API-LIB-04, API-LIB-07, API-SCAN-01. (Exclusion rules, LIB-006, are
  R1.1, WP-140; changing a root's location with a preview, LIB-031 and
  API-LIB-05, is R1.1, WP-150; keeping a library off Home, DIS-012, is
  R1.2, WP-154; the spoken-word flag, LAT-010, is R1.3, WP-161.)
- **Security.** Boundaries TB9, TB11; threats TM-T13, TM-T56. Verifies
  SEC-API-022, SEC-MED-037, SEC-TM-042.
- **Scope.** Create and configure libraries (kind, roots, splitting
  rules, watch and poll settings; the point releases add exclusions, the
  keep-off-Home switch and the spoken-word flag to the same settings);
  the folder browser, admin-only, listing directories only and confined
  to admin-configured browse roots after canonicalising and resolving
  links (SEC-API-022), with live checks (readable, empty, storage type,
  read-only); adding or removing a root and browsing the file system are
  fresh-uv actions (SEC-IAM-041); refusing a root that is a filesystem
  root or system directory, or that equals, contains or lies inside the
  data, cache, configuration or log directories, with the reason shown
  (SEC-MED-037, SEC-TM-042); grant and revoke routes, written through WP-065's grants API
  (WP-065 owns the table), with new libraries visible only to the owner and admins
  until granted (owner decision 19); starting a scan by requesting the
  scan task kind from the task runner, which WP-102 registers.
- **Tests (real SQLite and filesystem).** The data directory as a root is
  refused, and so are `/`, `/etc` and a parent of the data directory; a
  non-admin calling the folder browser gets 404; the browser refuses
  `..`, percent-encoded and symlinked escapes from its browse roots and
  never returns file contents; adding a root without fresh user
  verification is refused; a new library is invisible to members until
  granted, and a grant takes effect on the next request.

### WP-100 Job activity and the activity log (was: task list and activity log)

- **Wave** 3 · **Size** M (was S in effect; it gained the audit routes,
  the investigation mode and the checkpoint head) · **Depends on**
  WP-062, WP-065, WP-069, WP-070, WP-118.
- **Owns** `crates/gunmetal-server/src/tasklog/`.
- **Serves** ADM-110, LIB-022; API-SCAN-04 (reading), API-SCAN-05. (The
  task list with run, cancel and history, ADM-093, is R1.2, WP-155, over
  the same routes; showing the bytes each scan read, ADM-088, is R1.3 and
  needs only the counts WP-102 already writes into its activity entry.)
- **Security.** Boundaries TB4, TB11; threats TM-T12, TM-T61. Verifies
  SEC-HIS-013, SEC-OPS-027 (the audit routes, their per-role response
  types and the investigation mode; the alert reaching the person
  end to end is WP-117's), SEC-OPS-075 (serving the checkpoint head).
- **Scope.** Read-only routes for the jobs that are running or waiting
  (progress, last run, duration, errors) and the activity log (scans,
  "0 changed" rescans, moves, and from R1.1 re-reads after a parser
  update and imports) alongside the security events from the audit log;
  running and cancelling a task by hand is R1.2 (WP-155); an activity-entry interface the scan pipeline
  writes to. The audit routes, which no package owned although
  API-SCAN-05's capability row cites SEC-OPS-027: only the owner and
  holders of the audit capability read the full log, through WP-069's
  `Permit`-taking reader, and the response type for that audience holds
  other people's addresses only truncated (/24, /48 or a country), while
  every person reads their own records with full addresses
  (SEC-OPS-027). The security-investigation mode, which no package
  planned: an owner, fresh-uv action that reveals full addresses for a
  stated period and scope, is itself recorded in the audit log, and
  publishes one alert event on the bus for each person whose addresses
  it reveals, which WP-097 turns into their alert. The route serving the
  latest signed checkpoint head to admins' clients, which keep it off
  the host (SEC-OPS-075; the client's check at sign-in belongs to the
  client plan).
- **Tests (real SQLite).** A member cannot read the job list; activity
  entries page in order; an entry written by a fake producer appears with
  its counts. Every audit route replayed as owner, audit-capability
  holder, administrator, member, guest and anonymous caller against a
  literal table of expected statuses; the owner's response for another
  person's record holds only the truncated address, compared whole;
  entering investigation mode without fresh verification is refused,
  and with it writes exactly one audit record and publishes exactly one
  alert event per affected person on a recording bus; the head route
  returns the head WP-069 reports and is refused to a member.

### WP-101 Built-in HTTPS by ACME

- **Wave** 4 (moved from 3 in review, because WP-073 moved to wave 3; it
  stays in wave 4 for the adopted R1, see below) · **Size** L (was M) ·
  **Depends on** WP-047, WP-048, WP-073, WP-097, WP-125 (record 8).
- **Owns** `crates/gunmetal-server/src/acme/`.
- **Serves** ADM-022, ACC-098, ACC-099, ADM-021.
- **Security.** Boundaries TB1, TB8; threats TM-T11, TM-T32. Verifies
  SEC-NET-003, SEC-NET-004, SEC-NET-005, SEC-NET-006, SEC-NET-013,
  SEC-NET-072, SEC-TM-010, SEC-OPS-007 (before the claim, the only
  outbound connections are the ACME purpose's, and none at all with
  tailnet or localhost naming; WP-117 repeats it over the whole server in
  a network namespace).
- **R1, and now the R1 path to HTTPS for a household with its own
  domain.** The owner's answer to D-07 gives R1 three ways to HTTPS: the
  owner's own domain with automatic certificates (this package), a
  tailnet, or the same machine; the project name service and its client
  moved to R2 (WP-129, WP-135). This package was already R1 (the first
  draft left R1 or a point release open, this plan's decision 21). It
  does not need to move earlier: setup (WP-080, wave 3) needs only a
  secure context and does not call it, the first claim over an
  own-domain name is proven end to end in WP-117, and it cannot move: it
  hands certificates to WP-073's listener configuration, which is also
  wave 3.
- **Scope.** Obtain and renew a certificate by ACME DNS-01 for a domain the
  owner controls, through the egress gate's ACME purpose (the CA and the
  owner's DNS provider API), which is granted when the owner configures
  the domain and is the one purpose allowed before the claim
  (SEC-OPS-007), with the DNS provider token held in the secrets crate's
  vault (WP-047) and decrypted only for the call; generate the TLS key and the ACME account key from
  the CSPRNG on the server and store them as 0600 files under
  `secrets/tls/`, never in the database or a backup in clear
  (SEC-NET-006); activate a new certificate only when its chain validates
  against the bundled WebPKI roots without fetching intermediates, and
  present the complete chain (SEC-NET-003); renew automatically, following
  ACME Renewal Information when offered and otherwise by two-thirds of
  the lifetime, for lifetimes from 6 to 398 days (SEC-NET-004,
  SEC-TM-010); alert the owner 30 and 7 days before expiry with a
  one-click "renew now" (SEC-NET-072); when no valid certificate is
  available, never serve the web client, sign-in or API in plaintext to
  non-loopback peers, and say so on the console, the log and the
  plaintext help page (SEC-NET-005); keep own domain, tailnet and
  localhost working, each with a CI job (SEC-NET-013), with no dependence
  on a project service. Hand the key to WP-073's listener.
- **Tests.** Against a local ACME test server (Pebble with a test DNS
  provider) if one can run in CI (unverified); renewal before expiry with
  a manual clock, and a property test of the renewal scheduler over random
  lifetimes and renewal windows; fixture chains (complete, missing
  intermediate, wrong root) for activation; key file modes; expired and
  missing certificate fixtures giving the help page and no sign-in; in an
  isolated network namespace with an egress recorder, an unclaimed server
  configured for its own domain contacts only the test CA and the test DNS
  provider, and one configured for localhost contacts nothing
  (SEC-OPS-007). If no ACME test server can run inside the gate, the protocol steps are tested
  against a fake written in the test from RFC 8555, and talking to a real
  ACME server becomes a manual check outside the gate, which is a weaker
  guarantee the owner should accept knowingly.
- **Risks.** The crate choice for ACME messages is unsized (see "Missing
  from this table"; owner decision 21 now covers only that choice).

### WP-120 Sign-out and browser pairing (added in review; was: sign-in and sign-out routes)

- **Wave** 3 · **Size** M · **Depends on** WP-038, WP-062, WP-064, WP-069,
  WP-118.
- **Owns** `crates/gunmetal-server/src/signin/`.
- **Serves** ACC-062 (approve a browser from a signed-in device), ACC-063,
  ACC-079, ADM-122; API-AUTH-07, API-AUTH-08 (the route side).
- **Security.** Boundaries TB4, TB5; threats TM-T04, TM-T47, TM-T64.
  Verifies SEC-IAM-056, SEC-IAM-057, SEC-IAM-058, SEC-IAM-059, SEC-IAM-060,
  SEC-IAM-108, SEC-API-039, SEC-API-058, SEC-NET-036, SEC-CLI-024,
  SEC-STD-027, SEC-STD-029, SEC-CLI-009 (the `Clear-Site-Data` clause on
  the sign-out response; clearing the browser's stores is the client's).
- **Why it changed.** As added in review, this package owned the password
  sign-in route and its two-factor step. The baseline has no passwords
  and no TOTP (SEC-IAM-025; baseline owner decision 1); passkey sign-in
  is WP-081's, and a person whose browser cannot use a passkey signs it
  in by approval from one of their own signed-in devices (SEC-IAM-108),
  which no package owned. ACC-052 and ACC-053 are withdrawn from R1.
- **Scope.** Sign out: the session is invalidated on the server and the
  response carries `Clear-Site-Data: "cache", "cookies", "storage"` on
  secure origins (SEC-API-039). Browser pairing: the browser that cannot
  use a passkey shows an 8-character base-20 pairing code, valid once for
  10 minutes and dead after 5 wrong guesses (SEC-IAM-056), and a QR code
  carrying the server's identity key and an approval URL on the server's
  own HTTPS origin (SEC-IAM-057); the secrets travel only in fragments, QR
  payloads or request bodies (SEC-NET-036). The approval always starts on
  the approving device, which is signed in and personal-class; nothing a
  person without a session does can make a prompt appear on someone's
  device (SEC-STD-027). The approval screen's data names the requesting
  browser's self-reported name marked as unverified, its type, "In this
  home" or "Somewhere else" as decided by the path class, how long ago
  the request started and exactly what is being granted (SEC-IAM-058); an
  approval counts as local only on the same local path class, and
  otherwise needs the typed code and a matching code (SEC-IAM-060). The
  paired browser generates a non-extractable Web Crypto key, renews its
  session only by signing a fresh server challenge, and is a limited-class
  device that can never hold administrator capabilities, approve other
  devices or change account security (SEC-IAM-059, SEC-IAM-108,
  SEC-CLI-024). Every attempt goes through the credential verifier, with
  the same failure for an unknown, expired or wrong code (SEC-API-058),
  and a code is consumed by one conditional update (SEC-STD-029); audit
  entries for success and failure; no user list before sign-in.
- **Tests (real SQLite).** A full pairing over WP-044's test client: the
  approving device approves, the browser's next request is authenticated
  as a limited-class device, and its session is refused on every admin
  and account-security route; renewal without the browser's key fails; a
  code past 10 minutes, a code after 5 wrong guesses and a used code each
  fail identically to a wrong code; 64 concurrent approvals of one code
  give one session; an unauthenticated pairing request creates no prompt
  anywhere; the matrix of path classes decides local or remote, and a
  remote approval without the matching code fails; a hostile device name
  with bidirectional controls is returned marked unverified and as text;
  sign out makes the old cookie fail on the next request and carries the
  exact `Clear-Site-Data` header; each attempt leaves exactly one audit
  entry. The end-to-end test with WebAuthn disabled in a real browser is
  part of the client's journeys.

### WP-131 Route-table security suites (added for the security baseline)

- **Wave** 3 · **Size** L · **Depends on** WP-044, WP-062, WP-065, WP-118.
- **Owns** `crates/gunmetal-server/tests/route_security/`, the rival
  replay harness `crates/gunmetal-server/tests/rivals/main.rs` (the
  directory is a registry; see the shared-file table), the replay files
  for the R1 incidents whose feature has landed by wave 3,
  `.github/workflows/api-fuzz.yml`. `security/public-routes.txt` moved to
  WP-118, so that wave 3 route packages add their lines to a file that
  already exists.
- **Serves** ACC-120, ACC-121; the generated suites the baseline's
  "security in the test-first process" section asks for.
- **Security.** Boundaries TB1, TB2, TB4, TB11; threats TM-T02, TM-T12,
  TM-T13, TM-T14, TM-T15. Verifies SEC-TM-004, SEC-TM-009, SEC-TM-017,
  SEC-TM-025, SEC-TM-027, SEC-API-002, SEC-API-003, SEC-API-004,
  SEC-API-005, SEC-API-008, SEC-API-011, SEC-API-013, SEC-API-019,
  SEC-API-024, SEC-API-053, SEC-API-055, SEC-API-068, SEC-API-092,
  SEC-IAM-015, SEC-IAM-025, SEC-IAM-041, SEC-IAM-071, SEC-IAM-072,
  SEC-IAM-081, SEC-EXT-006, SEC-HIS-006, SEC-HIS-007, SEC-HIS-009,
  SEC-HIS-010, SEC-HIS-013, SEC-HIS-024, SEC-HIS-028, SEC-HIS-041,
  SEC-HIS-056, SEC-HIS-060, SEC-HIS-066, SEC-CLI-004, SEC-CLI-007,
  SEC-CLI-015, SEC-NET-047, SEC-OPS-020, SEC-OPS-050, SEC-PRV-018,
  SEC-PRV-020, SEC-PRV-022, SEC-PRV-026, SEC-STD-013, SEC-STD-014,
  SEC-STD-026, SEC-STD-038, SEC-PRV-034, SEC-PRV-035 and SEC-PRV-036 (the
  R1 absence proof: no external-account linking or scrobbling route).
  SEC-HIS-066 is proved jointly: this package owns the harness and the
  replays for incidents whose feature exists by wave 3, and each later
  feature package adds the replays for its own incidents.
- **Why it exists.** The baseline's authorisation and route rules are
  proved by suites generated from the route table, so that a new route
  cannot ship untested. Each route package in the first draft tested its
  own routes by hand, and nothing covered the routes as a whole.
- **Scope.** A fixture world (owner, admin, members A and B, a guest, a
  revoked session, a disabled account, an anonymous caller) seeded into a
  real server with a real SQLite file, and suites generated from the
  registered route table, so every route a later package registers is
  covered the moment it merges: the extension of WP-118's anonymous
  suite with the states that need sessions and the fixture world, so
  every route outside the allow-list answers 401 with a byte-identical
  body for expired and revoked credentials and for a disabled account as
  well as for no and malformed ones (SEC-TM-004, SEC-API-003); the
  allow-list check itself is WP-118's, and runs here again over the full
  fixture world (SEC-API-002); the cross-principal matrix,
  which replays A's object IDs as B, the guest, the revoked session and
  the anonymous caller and expects the not-found response, including
  identifiers of the wrong kind (SEC-TM-025, SEC-IAM-071, SEC-API-011,
  SEC-API-024, SEC-HIS-010); the capability and role matrix against a
  literal file of expected statuses, including that every admin route
  needs an explicit admin capability and that a client-side restriction
  is enforced on the server (SEC-API-019, SEC-HIS-013, SEC-CLI-015); the
  route-tag check (every route has exactly one tag, a media or ordinary
  session is refused on every admin route, and the host-equivalent list
  is fresh-uv; SEC-IAM-041, SEC-TM-017); the extra-field and
  principal-injection suites (SEC-TM-027, SEC-IAM-072, SEC-API-013,
  SEC-HIS-009); credentials in queries, reserved parameter names and two
  credentials at once (SEC-API-004, SEC-EXT-006, SEC-HIS-041); methods
  (SEC-API-008); the `Gunmetal-Request`, `Sec-Fetch-Site` and `Origin`
  matrix on every cookie-capable route (SEC-CLI-007); header golden tables
  by response class and status, with `Cache-Control` by data class
  (SEC-API-053, SEC-API-055, SEC-CLI-004, SEC-IAM-015, SEC-HIS-028,
  SEC-PRV-018, SEC-PRV-020, SEC-STD-013); the cookie inventory
  (SEC-STD-014); unauthenticated responses revealing no version or server
  name in any server state (SEC-NET-047, SEC-OPS-050); per-role response
  schema snapshots (SEC-API-068); a mutating route that declares no audit
  event failing the build (SEC-OPS-020); routes that must not exist: no
  password or TOTP route, no federation route, no OAuth or OpenID
  Provider endpoints, no impersonation route, no route that accepts code
  to install, no anonymous route that takes a URL to fetch, no endpoint
  at two routes, no removed route still answering, and no route that
  links an external account or submits scrobbles, so SEC-PRV-034 to
  SEC-PRV-036 hold by absence in R1 (SEC-IAM-025, SEC-IAM-081,
  SEC-STD-026, SEC-PRV-026, SEC-HIS-056, SEC-HIS-024, SEC-HIS-007,
  SEC-API-092, SEC-PRV-034, SEC-PRV-035, SEC-PRV-036); every byte route
  requiring a credential bound to its object (SEC-HIS-006); another
  person's activity never returned (SEC-PRV-022, SEC-HIS-060). The rival
  exploit replay suite: the harness, which runs every file in `rivals/`
  in the normal gate, and one replay per R1 incident whose feature exists
  by wave 3, named after the incident and citing its SEC-HIS requirement.
  This includes the replay for SEC-HIS-042's token incident in the form
  R1 has, a stream URL whose session was revoked (WP-082's feature,
  which exists by wave 3); WP-134 adds the share-link form in R1.2.
  Incidents in features that land later get their replay from the
  package that builds the feature, which adds a file to the registry
  directory and lists SEC-HIS-066 in its own Security field: in R1 at
  least the scanner (WP-102, SEC-HIS-017), artwork (WP-103, SEC-HIS-030)
  and restore (WP-109, SEC-HIS-019); after R1, playlist files (WP-112,
  SEC-HIS-018, R1.1) and share links (WP-134, SEC-HIS-042, R1.2). WP-127's check fails the release
  while any SEC-HIS ID has no test (SEC-HIS-066). As first written, this
  package claimed every replay in wave 3, before the scanner, artwork,
  restore, playlist-file and share-link packages existed. The API fuzzing
  job, which drives the running server through the OpenAPI description
  WP-118 generates, with a short budget on pull requests and a longer one
  nightly (SEC-STD-038).
- **Not in scope.** The cleartext replay and posture matrix (WP-132).
  Route-specific behaviour, which each route's package tests.
- **Tests.** These are the tests. A deliberately broken fixture route
  (public but unlisted, missing a tag, leaking another person's object)
  shows each suite fails for the right reason.
- **Risks.** The suites need every route to declare where its object IDs
  sit (WP-044); a route that hides an ID somewhere undeclared escapes the
  matrix, which is why `RouteSpec` makes the declaration mandatory.

### WP-132 Postures, the cleartext rule and exposure alerts (added for the security baseline)

- **Wave** 3 · **Size** L · **Depends on** WP-005, WP-023, WP-044, WP-062
  (the fresh-uv check on leaving the home posture and on trusted-proxy
  changes), WP-069, WP-118.
- **Owns** `crates/gunmetal-server/src/posture/`,
  `crates/gunmetal-server/tests/posture/`.
- **Serves** ACC-097 (trusted reverse proxies, moved here from WP-073),
  ACC-106 (no automatic port forwarding), ADM-021 (the help page),
  CLI-150.
- **Security.** Boundaries TB1, TB2; threats TM-T05, TM-T11, TM-T46, TM-T48.
  Verifies SEC-NET-001, SEC-NET-005, SEC-NET-017, SEC-NET-019,
  SEC-NET-023, SEC-NET-024, SEC-NET-027, SEC-NET-028, SEC-NET-030,
  SEC-NET-031, SEC-NET-045, SEC-NET-056, SEC-NET-059, SEC-NET-068 (the
  internet posture, alert and home-network label for an "unknown" peer;
  the classification is WP-118's), SEC-OPS-038, SEC-OPS-039, SEC-IAM-013,
  SEC-API-075, SEC-HIS-001, SEC-HIS-053, SEC-TM-006, SEC-TM-017 (leaving
  the home posture and changing trusted proxies are fresh-uv routes),
  SEC-STD-040. SEC-NET-016 and SEC-OPS-037 moved to WP-118 with the
  resolver.
- **Why it exists.** The baseline's first principles 3 and 4 (location is
  never identity; no credential crosses cleartext) are decided where a
  request enters, before any route. The first draft had trusted proxies
  in WP-073 and no package for the cleartext rule, the home posture or
  exposure detection.
- **Scope.** The posture hook WP-044's pipeline calls first. Over
  plaintext HTTP, every peer other than loopback gets only a static
  redirect or help page that sets no cookie: no web client, API, upgrade
  or credential of any kind (SEC-NET-001), and the same help page when no
  valid certificate exists (SEC-NET-005). The home posture, the default:
  any request whose client address is outside loopback, RFC 1918, RFC
  6598 (only when one of the server's interfaces holds an address in the
  same prefix), RFC 4193 and IPv6 link-local gets only a static help page
  with no sign-in, server name or version (SEC-NET-024); a peer whose
  address is the default gateway, a bridge or a configured container
  gateway, which WP-118's resolver classifies "unknown", is treated as
  non-local, with no home-network label (SEC-NET-068). Trusted proxies:
  the routes and the `gunmetal trust-proxy` command that change the list
  WP-118's resolver reads (empty by default), each an owner, fresh-uv
  action (SEC-TM-017); when a peer that is not a trusted proxy sends
  forwarding headers, which the resolver has already ignored, the
  request gets internet posture and a security event, and the owner is
  offered the fix (SEC-NET-017); each trusted proxy is declared "private
  overlay" or "public", public being the default, and a public proxy's
  requests get internet posture (SEC-NET-019); no proxy-supplied
  identity header, Tailscale's included, is ever authentication
  (SEC-NET-023). The client address and its class come from the
  `ClientContext` that WP-118's resolver attaches, the same value the
  sessions, the credential verifier, the audit log and the rate limiters
  take from wave 2 (SEC-OPS-037); as first written this package owned
  the server-side resolver in wave 3, after those packages, which would
  each have read the socket peer themselves. Context only ever adds friction: no location grants
  anything (SEC-IAM-013, SEC-API-075, SEC-HIS-001). Admin operations are
  refused on internet-posture paths unless the owner turned remote
  administration on (SEC-NET-045). Exposure: a request from a non-local
  address reaching a home-posture listener records a security event and
  raises the owner alert within a minute (SEC-NET-027); at startup and
  when interfaces change, the admin sees which listeners are bound to
  globally routable addresses (SEC-NET-028). The home posture is the
  default in every configuration and survives upgrades and restores;
  leaving it is an owner, fresh-uv action (SEC-OPS-038). The listening
  sockets in each documented configuration equal the documented list,
  with no SSDP, UPnP, DLNA or mDNS responder in R1 and no router port
  mapping requested (SEC-NET-031, SEC-NET-059, SEC-NET-030, SEC-OPS-039,
  SEC-HIS-053, SEC-TM-006), and the worker is reached only over socket
  pairs (SEC-STD-040). Network security events are structured,
  secret-free records (SEC-NET-056).
- **Tests.** The cleartext replay: every route in the route table over
  plain HTTP from a non-loopback peer gets the literal help response and
  no `Set-Cookie`. The posture matrix: the whole route table from each
  address class (real loopback and ULA addresses where CI allows,
  otherwise through a trusted test proxy), asserting the literal help
  response for every non-local class. Properties of the 100.64.0.0/10
  rule with and without a matching interface, and of the policy: adding
  any context signal never turns a denial into an allowance. Forwarding
  headers from an untrusted peer give internet posture and exactly one
  security event; adding a trusted proxy or leaving the home posture
  without fresh user verification is refused with no state change; an
  undeclared proxy parses as public; Tailscale identity headers from loopback and
  from a trusted proxy grant nothing. In a network namespace that SNATs
  clients to the gateway address, the peer is "unknown". A non-local
  request in home posture gives exactly one event and one alert on a
  manual clock. The listening-socket inventory for each documented
  configuration. Admin routes from every path class with remote
  administration on and off.
- **Risks.** CI runners may not provide real ULA or RFC 6598 addresses;
  the trusted test proxy covers those classes, which is weaker, and the
  package says so.
- **Added for the client plan (2026-10-03).** The help pages use the same stylesheet
  as the startup page (CP-004, through WP-095's note), once it exists.

### WP-236 WASM facade: queue, shuffle, gain, player state and lyrics (added for the client plan)

- **Wave** 2 · **Size** M · **Depends on** WP-021, WP-025, WP-026,
  WP-028, WP-030, WP-235.
- **Owns** `crates/gunmetal-wasm/src/queue.rs`, `shuffle.rs`, `gain.rs`,
  `player.rs`, `lyrics.rs`, with their generated declarations under
  `crates/gunmetal-wasm/types/`.
- **Serves** ADR 1 decision 2; MUS-116 to MUS-119, MUS-122, MUS-126,
  MUS-077, MUS-084, MUS-085, MUS-087 to MUS-089, MUS-154, MUS-155 (each
  as the core's rule reaching the web client).
- **Security.** Boundaries TB4; threats TM-T62. Verifies SEC-MED-077,
  SEC-CLI-021.
- **Scope.** In the form WP-235 sets: exports for applying and rebasing
  queue operations, the shuffle order, the gain decision with the factor
  to apply, the player state's transition function and the lyric
  position lookup. Each export is a direct call into the core with
  conversion only. It may start as soon as WP-235 has merged, and it
  merges into `wave-2` after WP-235; it is the package the web player's
  first clickable build waits for.
- **Not in scope.** User events, inbound links, tints and the QR matrix
  (WP-237). Sync, the library, search, the decision and Home rows
  (WP-088).
- **Tests.** Native unit tests of each conversion with literal values;
  the drift check; the `wasm32` build.

### WP-237 WASM facade: user events, inbound links, tints and the QR matrix (added for the client plan)

- **Wave** 3 · **Size** S · **Depends on** WP-034, WP-235, WP-238,
  WP-239, WP-240, WP-242.
- **Owns** `crates/gunmetal-wasm/src/userdata.rs`, `deeplink.rs`,
  `tint.rs`, `qr.rs`, with their generated declarations under
  `crates/gunmetal-wasm/types/`.
- **Serves** ACC-062, ACC-080, CLI-093, MUS-110, MUS-180, MUS-182;
  API-LOG-01 (each as the core's rule reaching the web client).
- **Security.** Boundaries TB4, TB5; threats TM-T18, TM-T65. Verifies
  SEC-MED-077 and SEC-CLI-021 (conversion failures are typed errors).
  The rules it exports are proved by the packages that own them:
  SEC-PRV-002 and SEC-PRV-024 by WP-240, SEC-CLI-025 by WP-239.
- **Scope.** Conversion only, in the form WP-235 sets: the event
  builder and the sink (WP-240) and receiving a clock (WP-034); the
  inbound-link parser (WP-239); the tint rule (WP-238); the QR matrix
  (WP-242). The wall time and the random bytes the event builder needs
  are arguments, supplied by the web client from one browser adapter;
  this crate asks for no random source of its own. No rule is written
  here.
- **Not in scope.** Storage and upload (client; WP-068 on the server).
- **Tests.** Native unit tests of each conversion with literal values;
  an event built through the facade equals the one WP-240 builds from
  the same inputs; the drift check; the `wasm32` build.

### WP-241 Audit-head extension check (added for the client plan)

- **Wave** 3 · **Size** S · **Depends on** WP-035, WP-069, WP-235.
- **Owns** `crates/gunmetal-core/src/audit_head.rs`,
  `crates/gunmetal-wasm/src/audit_head.rs` with its generated
  declarations.
- **Serves** ADM-145.
- **Security.** Boundaries TB10, TB11; threats TM-T61. Verifies the
  check's half of SEC-OPS-075 (a client holding an older head detects a
  log that does not extend it).
- **Why it exists.** SEC-OPS-075 has each admin's client keep the
  latest signed checkpoint head and check at sign-in that the log
  extends it. WP-069 makes the head and WP-100 serves it; no package
  built the check, and it is a rule, so the web client may not write
  it.
- **Scope.** A pure function that, given the head a client stored, the
  head the server now serves and what the server returns to link them,
  answers "extends", "does not extend" or "cannot tell", after checking
  the newer head's signature against the audit key the stored head
  names. It parses heads in the encoding WP-069 writes, under the parse
  contract. Its one facade export, in the form WP-235 sets.
- **Not in scope.** Storing the head (client, CP-052). Serving it
  (WP-100). Exporting checkpoints (WP-090).
- **Tests.** A log extended by appended records gives "extends"; a
  re-signed, truncated log gives "does not extend"; a head signed by
  another key, and a malformed head, are typed errors; literal heads
  taken from WP-069's own test encodings parse to literal values.
- **Risks.** Whether the core's crypto module already holds the
  signature check this needs is unverified; if not, it is added there
  by its owner under the interface-change rule, not here.

## Wave 4: scanning and the features that need it

WP-101 (numbered in the wave 3 section) also runs in this wave, and
WP-121 and WP-133 are at the end of this section. WP-109 (restore) is
numbered here but runs in wave 5, after WP-106. Curation (WP-107) left R1
for R1.3, with its file inspector split off for R1.2 (WP-156), and music
share links (WP-134) for R1.2; their specifications are in
[After R1](#after-r1-point-releases-and-later).

### WP-102 Scan pipeline

- **Wave** 4 · **Size** L · **Depends on** WP-060, WP-066, WP-067, WP-070,
  WP-075, WP-076, WP-077, WP-078, WP-079, WP-083, WP-100, WP-119. (It no
  longer depends on WP-071: the derived-data store arrives in R1.1.)
- **Owns** `crates/gunmetal-server/src/scan/`.
- **Serves** LIB-012, LIB-016, LIB-017, LIB-019 to LIB-022, LIB-029,
  LIB-030, LIB-032, LIB-136, MUS-043, MUS-044 (inputs); API-SCAN-01,
  API-SCAN-03, API-CAT-09, API-CAT-10; the "Library scan" job. The
  parser-upgrade re-read (LIB-025) was split out to WP-123 in review and
  is R1.1. (Folder playlists, LIB-192, are R1.1: WP-112 takes the hand-off
  this scan records. Showing the bytes read, ADM-088, is R1.3 over the
  counts this scan records.)
- **Security.** Boundaries TB6, TB9; threats TM-T20, TM-T22, TM-T30, TM-T56.
  Verifies SEC-MED-016, SEC-MED-017, SEC-HIS-017, SEC-HIS-024, SEC-TM-069,
  SEC-API-080, SEC-HIS-066 (the replay for its incident), and the "from
  the OS CSPRNG" part of SEC-HIS-012, SEC-API-023 and SEC-PRV-021 for
  content IDs.
- **Scope.** The scan task kind. For a path set: walk, fingerprint and
  short-circuit unchanged files and directories; send changed files to the
  worker pool for probing and identity hashing; derive tracks; regroup the
  affected albums; diff against the index; apply the artwork order of
  LIB-136; commit in batches with change-log entries so the library fills
  in during the first scan, writing the `CatalogChange`s WP-067 returns
  into the change log (WP-066) in the same transaction; report files
  found, bytes read per root and an estimate on the bus for the event
  channel; record activity ("0 changed",
  "214 files moved"); mark availability (playable, drive offline, damaged,
  missing), never deleting items, history, playlists or grants because a
  root went offline (SEC-TM-069); store each file's parser versions so
  the R1.1 re-read (WP-123) can find stale ones; never launch a helper
  per file. A playlist file found in a library is detected by content and
  recorded as a sidecar that R1 does not read, so nothing in it is
  opened, parsed or fetched until WP-112 handles it in R1.1. Look up
  each identity's public ID in the identity store's mapping (WP-046),
  and mint a missing one only through WP-047's minting function. Files are classified by the content of the resolved target,
  never by name or extension (SEC-HIS-017), and nothing the scan reads
  (W-frames, URLs in tags, playlist URLs, `#EXTIMG` directives, artwork
  links) is ever fetched or opened (SEC-MED-016, SEC-API-080,
  SEC-HIS-024); a limit or error in an optional part keeps the rest of the
  file and records the skipped part (SEC-MED-017).
- **Not in scope.** Triggers (WP-098). Health reports (WP-110). Trash
  purges (WP-111). Re-reading after a parser upgrade (WP-123).
- **Tests (real SQLite, real files, real workers).** A synthetic library of
  a few dozen files from WP-119: the first scan produces the albums,
  artists and tracks in the generator's manifest as whole records; a second scan
  writes nothing and sends no changes (LIB-016); adding one album rescans
  only its folder; moving a folder keeps history and playlist entries;
  replacing an MP3 album with FLAC keeps "date added"; unplugging a root
  (the test makes it unreadable) greys items instead of removing them; a
  file that crashes the worker is quarantined and the scan finishes; bytes
  read per file stay under the header-only bound for each format; a
  fixture library full of URL-bearing tags and playlists causes no
  outbound connection (checked through the egress record); a file with
  oversized artwork and malformed lyrics stays playable with both
  problems recorded; `passwd.flac` linking outside the root is skipped.
  Readers keep answering during a long scan (a timed read in parallel).
  Two fresh data directories scanning the same synthetic library give
  disjoint public IDs. The rival replay for the scanner incident of
  SEC-HIS-017, a file this package adds to
  `crates/gunmetal-server/tests/rivals/`.

### WP-103 Artwork cache and image serving

- **Wave** 4 · **Size** M · **Depends on** WP-065, WP-067, WP-079,
  WP-082. (WP-065 and WP-067 answer which library an image belongs to.
  It no longer depends on WP-071, which arrives in R1.1, or on WP-078,
  which it needed only to re-encode uploads; profile picture uploads
  moved to WP-144 in R1.1.)
- **Owns** `crates/gunmetal-server/src/artwork/`.
- **Serves** LIB-142, LIB-143, MUS-040, ACC-125 (extracted artwork is
  stored only under server-generated names); API-STR-05, API-SYNC-05.
  (Profile pictures, ACC-011 and API-USR-01's picture part, are R1.1,
  WP-144.)
- **Security.** Boundaries TB4, TB6; threats TM-T09, TM-T28, TM-T29.
  Verifies SEC-MED-046, SEC-MED-047, SEC-MED-048, SEC-API-027,
  SEC-API-051, SEC-API-087 (extracted artwork named by content hash),
  SEC-PRV-016 (the server's own origin is the only source of artwork),
  SEC-TM-035, SEC-HIS-006, SEC-HIS-031, SEC-HIS-037, SEC-CLI-005,
  SEC-HIS-066 (the replay for the artwork incident of SEC-HIS-030, a file
  this package adds to `tests/rivals/`). The upload rules it verified
  (SEC-MED-061, SEC-PRV-006, SEC-API-085, SEC-API-088) went with profile
  pictures to WP-144; in R1, SEC-API-085 is proved by restore's upload
  (WP-109) and SEC-API-088 by WP-130.
- **Scope.** The derivative cache keyed by content hash and size, bounded in
  bytes with least-recently-used eviction (SEC-MED-048), stored in the
  data directory under server-generated content-hash names, never in a
  library folder and never under a name taken from a request or a file
  (SEC-API-087); image routes that
  accept a size only from the fixed enumeration (SEC-MED-047,
  SEC-HIS-037); artwork served only from capability URLs, like media,
  with the 1-hour lifetime aligned to a fixed time bucket so repeat
  requests in a bucket reuse one URL and the browser can cache it
  (SEC-API-026, SEC-API-027, SEC-HIS-006). The first draft proposed
  content-addressed paths authorised by the session cookie (api-needs.md
  Flags item 7, this plan's owner decision 31); the baseline's bucketed
  capability URLs meet the caching need without a second authentication
  mechanism for media, so that proposal is withdrawn. Only derivatives the
  server generated are served, as an allow-listed raster type chosen from
  the content, never the original bytes (SEC-MED-046, SEC-CLI-005,
  SEC-TM-035), with the server-chosen `Content-Type` and `nosniff`
  (SEC-API-051, SEC-HIS-031); every image URL the server hands out is
  relative to its own origin (SEC-PRV-016).
- **Not in scope.** Profile picture uploads (WP-144, R1.1).
- **Tests (real SQLite and files).** Sizes −100000, 0, 2^31, `abc` and a
  valid size with an invalid shape give 400; the cache never exceeds its
  bound; a cached derivative's file name is its content hash whatever the
  source file was called, including a source named with `..` and
  separators; a member cannot fetch artwork for a library they cannot
  see; two requests in one time bucket get the same URL and one across
  the boundary does not; a revoked session's artwork URL fails on the
  next request; every image URL in the artwork routes' responses is
  server-relative.

### WP-104 Playback session registry and stream limits (was: playback session registry and stop)

- **Wave** 4 · **Size** S (was M; the admin live view and stopping a
  session moved to WP-153 in R1.2) · **Depends on** WP-082, WP-083.
- **Owns** `crates/gunmetal-server/src/playback/`.
- **Serves** ACC-075; API-SES-01 (the registry). (The admin's live view,
  the decision reason per session, stopping a session with a message and
  each person's opt-in for titles, ADM-099, ADM-100, ADM-102, ACC-072,
  ACC-073, ACC-116, MUS-235, INT-134, MUS-190 and API-SES-02, are R1.2,
  WP-153.)
- **Security.** Boundaries TB4, TB11; threats TM-T17, TM-T18. Verifies
  SEC-IAM-102, SEC-HIS-014, SEC-PRV-025 (the R1 part: no admin route
  offers a view, search or export of another adult's history or of what
  they are playing; WP-153 proves the live view's part in R1.2).
- **Scope.** Record who plays what on which device, the delivery path and
  the decision reason the device reported, alongside what the server
  actually served, for the person's own devices; enforce the per-account
  limit on concurrent playback streams, counting playback and not
  browsing (SEC-IAM-102). In R1 no route shows the registry to anyone but
  the person it belongs to: an admin sees no other person's sessions or
  titles, and no other-person Activity route exists (SEC-PRV-025). Nobody
  can control another person's playback (SEC-HIS-014).
- **Not in scope.** The admin live view, the title opt-in and stopping a
  session with a message (WP-153, R1.2).
- **Tests (real SQLite).** The stream above the per-account limit is
  refused, and browsing while at the limit is not; an admin token gets
  403 or 404 on every other-person Activity route and on every route that
  would list another person's sessions; a member cannot see or control
  another person's session. What members and admins see of other people's
  sessions was this plan's owner decision 30; the owner answered it on
  2026-10-02 (who is playing and totals, not what, unless each person
  opts in), and the live view that shows it is R1.2.

### WP-105 Audio packaging route (conditional on ADR 4)

- **Wave** 4 · **Size** M (was S; it gained the worker job) · **Depends
  on** WP-056, WP-061, WP-078, WP-079, WP-082.
- **Owns** `crates/gunmetal-server/src/packaging/`,
  `crates/gunmetal-worker/src/jobs/package.rs`.
- **Serves** MUS-230, MUS-067; API-STR-04.
- **Security.** Boundaries TB4, TB6; threats TM-T16, TM-T20, TM-T21.
  Verifies SEC-API-026, SEC-API-028, SEC-MED-018, SEC-MED-020,
  SEC-MED-023, SEC-MED-024, and SEC-MED-081 and SEC-MED-032 for the audio
  packager (R2 in the baseline; verified here because the packager ships
  in R1, D-09 and D-80).
- **Scope.** The `Package` worker job: the server opens the file under the
  root handle and passes it to a worker by read-only descriptor
  (SEC-MED-020); the worker runs the core packager (WP-056) under the
  SEC-MED-007 step budget, a per-stream memory cap and a watchdog, and
  streams the initialisation segment and each requested media segment back
  over the socket pair (SEC-MED-081). The server never parses or packages
  the file itself (SEC-MED-018). It treats each returned segment as
  untrusted: the frame is capped at 32 MiB, decoded with the core's
  limits, and the segment's box structure, sample count and duration are
  checked against the stored frame index before a byte is served
  (SEC-MED-023). Packaging runs at the isolation tier SEC-MED-024 gives
  memory-safe parsing; if that floor is not met, packaging is off and the
  player falls back to the original file with the reduced-isolation
  notice, never to in-process packaging. Packaging workers come from
  their own small pool, so a slow scan never starves playback, and the
  route serves the initialisation segment and numbered media segments
  under one capability token per playback, with the segment number checked
  against the stored frame index.
- **Tests (real files).** The segments served for a synthetic FLAC parse
  back to the original frames; a segment number past the end gives 404; an
  expired token refreshes like a byte stream. Hostile input, with the
  test-hook feature: a packaging worker that panics, one that loops past
  its watchdog and one that allocates past its memory cap each end that
  one stream with a typed error, while a second stream on another file
  keeps serving segments without a gap and the next request for the
  failed file gets a fresh worker; a fake worker that returns a segment
  whose sample count disagrees with the frame index, or a frame over
  32 MiB, is refused and nothing is served; WP-001's dependency check
  shows the server crate has no call path to the core packager.

### WP-106 Account recovery and step-up

- **Wave** 4 · **Size** L (was M; recovery gained the hold, the issuer
  rules and key rotation) · **Depends on** WP-047, WP-062, WP-063,
  WP-069, WP-080, WP-081, WP-097.
- **Owns** `crates/gunmetal-server/src/recovery/`,
  `crates/gunmetal-server/src/keys/` (the `keys rotate` subcommand and its
  route),
  `crates/gunmetal-server/src/setup/passkey.rs` (the passkey branch of
  owner creation at setup, added here because WP-080 and WP-081 were in
  the same wave).
- **Serves** ACC-002 (passkey part), ACC-004, ACC-055, ACC-056, ACC-064,
  ADM-034, ACC-078; API-AUTH-03 (passkey part), API-AUTH-10 to
  API-AUTH-12, API-USR-03, API-USR-04.
- **Security.** Boundaries TB1, TB4, TB11; threats TM-T13, TM-T55, TM-T63.
  Verifies SEC-IAM-023, SEC-IAM-024, SEC-IAM-038, SEC-IAM-041, SEC-IAM-090,
  SEC-IAM-091, SEC-IAM-092, SEC-IAM-097, SEC-IAM-106, SEC-IAM-107,
  SEC-OPS-009, SEC-OPS-018, SEC-TM-017.
- **Scope.** The step-up route: when a fresh-uv route is refused for want
  of a recent verification, the client runs a user-verifying passkey
  assertion here, and on success this route calls WP-062's
  `record_user_verification`, which renews the session's assertion time;
  only a passkey assertion renews it (an OIDC sign-in, from R1.2, never
  can, and WP-096 then keeps the owner on at least one non-OIDC
  credential) (SEC-IAM-041, SEC-IAM-107, SEC-TM-017).
  The check itself, on every fresh-uv route, is WP-062's from wave 2; as
  first written it was built here, in wave 4, after the wave 3 packages
  whose fresh-uv routes needed it. Elevation issues a new session
  token (SEC-IAM-038). The person's sign-in methods page behind step-up:
  list, add and remove passkeys (OIDC links join the page in R1.2 with
  WP-096; there are no passwords or two-factor codes, SEC-IAM-025), adding or removing one notifying the
  person's other devices (SEC-IAM-023), and removing the last one refused
  unless the account is being deleted (SEC-IAM-024). Recovery: recovery
  codes (WP-063) offered at the owner's and administrators' first
  enrolment; an admin's one-time recovery enrolment link for members and
  guests only, the owner's for administrators, and never one for the
  owner, redeemed in person (a QR code on the issuer's screen) or on a
  device the person already approved (SEC-IAM-091); `gunmetal admin
  recover` on the host, reaching the server over a local socket open only
  to the service user, printing a single-use enrolment link valid for 15
  minutes and alerting every admin, with no network route for owner
  recovery (SEC-IAM-092, SEC-OPS-009). A credential enrolled through a
  code or a link starts the recovery hold (72 hours by default, 24 to 72
  at the owner's choice), during which it cannot remove other
  credentials, elevate, export history or create invitations, the
  person's existing devices can cancel it with one tap, and after an
  admin's link the person's history stays hidden (SEC-IAM-106); every
  recovery notifies all the person's devices, writes a security event and
  shows every existing credential and device to review (SEC-IAM-090). The
  person's own sign-in history (SEC-IAM-097). Rotating every server secret
  in one fresh-uv action from the dashboard and `gunmetal keys rotate`:
  stored secrets are re-encrypted, every session and signed URL is
  invalidated, and the action is audited and alerted (SEC-OPS-018).
- **Tests (real SQLite).** Removing the last sign-in method is refused; a
  sensitive change without a recent verification asks for step-up, the
  step-up route with a valid UV assertion from WP-081's software
  authenticator makes the same change succeed within 5 minutes, and a
  session whose last verification was not a passkey assertion (a test
  credential kind) is never renewed; a recovery link works once and
  expires; an admin cannot issue one for the owner, and a property over
  issuer and target roles gives the literal allowed set; an admin who
  redeems a link they issued cannot read the member's history before the
  hold ends; each restricted action is refused during the hold and
  allowed after it on a manual clock; the host command's socket has the
  service user's permissions and no network route starts owner recovery;
  its use appears in the audit log and raises the banner flag; after key
  rotation, old sessions and URLs fail; at setup on a secure context, the
  owner is created with a passkey from WP-081's software authenticator and
  offered recovery codes, and on an insecure context no sign-in is
  offered.

### WP-108 Personal data export (was: history import and data export)

- **Wave** 4 · **Size** M (was L; the history import moved to WP-145 in
  R1.1 and the owner's server export to WP-158 in R1.2, the split this
  package already foresaw) · **Depends on** WP-062 (the fresh
  verification), WP-068, WP-070, WP-086, WP-087, WP-093, WP-118. (The
  export covers settings from WP-087 as well as plays, loves and
  playlists; saved rules join it in R1.3 with WP-092.)
- **Owns** `crates/gunmetal-server/src/history_io/`.
- **Serves** MUS-188, ACC-010, DIS-058, INT-151; API-USR-05. (Importing
  Last.fm and ListenBrainz files, ADM-042, MUS-189, INT-107, API-USR-06
  and API-SET-11, is R1.1, WP-145; the owner's server export, ADM-074 and
  API-SET-04's export part, is R1.2, WP-158.)
- **Security.** Boundaries TB4, TB10; threats TM-T18. Verifies
  SEC-PRV-047, SEC-PRV-048, SEC-API-071, SEC-STD-029, SEC-TM-055.
- **Scope.** Build the person's documented export of everything they told
  the server, without admin involvement, as a versioned archive (native
  JSON Lines with a field-by-field README, and ListenBrainz-format
  listens) (SEC-PRV-047, SEC-TM-055); starting an export needs
  authentication within the last 5 minutes and is rate-limited per
  person, and the download is single-use, bound to the requesting session
  and short-lived (SEC-PRV-048, SEC-STD-029); any CSV quotes fields as RFC
  4180 describes and neutralises cells beginning with `=`, `+`, `-`, `@`,
  tab or carriage return (SEC-API-071). Each person exports their own
  data themselves (SEC-PRV-047), and moving the whole server uses the
  encrypted backup (WP-090).
- **Not in scope.** Uploading and importing history files (WP-145, R1.1).
  The owner's server export (WP-158, R1.2).
- **Tests (real SQLite and log files).** The export contains a removed
  play only as its removal, and another person's data never; an export
  started with stale authentication is refused; the download link works
  once and only for its session; a title starting with `=` is neutralised
  in CSV (property over generated titles); the export of a person's
  generated activity, read back by an independent parser written in the
  test from the documented format, gives the same activity.

### WP-109 Restore

- **Wave** 5 (was 4; it now uses WP-106's recovery links, from wave 4) ·
  **Size** L · **Depends on** WP-078 (the jailed worker that
  opens restored databases), WP-080, WP-085, WP-086, WP-087, WP-090,
  WP-093, WP-095, WP-106 (recovery enrolment links and the hold), WP-125
  (record 10 allows the archive to be extracted).
  (The round-trip test compares queues, loves, playlists and accounts,
  which those packages project.)
- **Owns** `crates/gunmetal-server/src/restore/`,
  `crates/gunmetal-worker/src/jobs/sqlite_check.rs`.
- **Serves** ADM-029, ADM-071, ADM-069 (the upload half), ACC-013,
  ACC-125 (the uploaded backup); API-SET-04 (restore), API-SET-05.
  (Restore from the UI with a restore point and a preview, ADM-070, is
  R1.2, WP-157; remapping library roots onto a new machine, ADM-051 and
  LIB-031, is R1.1, WP-150.)
- **Security.** Boundaries TB10, TB11; threats TM-T59, TM-T60. Verifies
  SEC-OPS-008, SEC-OPS-024, SEC-OPS-043, SEC-OPS-044, SEC-STD-031,
  SEC-TM-052, SEC-HIS-019, SEC-HIS-066 (the replay for the archive
  incident of SEC-HIS-019, a file this package adds to `tests/rivals/`),
  SEC-API-085 (the backup upload, R1's one upload route, now that profile
  pictures and history imports are R1.1), and, for the links offered
  after a domain change, SEC-IAM-091 and SEC-IAM-106, and SEC-NET-072's
  origin-migration part (the changed-domain warning and re-enrolment).
- **Scope.** Restore from the command line, and from the welcome screen
  behind the same setup code as claiming (flows G15, SEC-OPS-008); the
  welcome screen takes the backup through an upload route that declares
  its one type and its byte cap, decides the type from the file's own
  header and ignores the client's file name and `Content-Type`, and
  stores the upload in scratch space under a server-generated name
  (SEC-API-085); restoring from the UI of a running server, with a
  restore point and a preview, is R1.2 (WP-157);
  verify the backup signature and the age encryption tags, and show which
  server made the backup and when, before anything is written
  (SEC-OPS-043, SEC-TM-052); extract the archive only under record 10's
  rules (entry paths ignored, symlink entries refused, entry count and
  unpacked size limited; SEC-HIS-019); open every restored SQLite file,
  which the server did not create itself, through the one connection
  opener's read-only profile for untrusted files (WP-126), read-only in a jailed worker
  with `trusted_schema` off, triggers and views disabled,
  `cell_size_check` on, memory mapping off and `quick_check` passed, so
  its contents reach the server only as typed rows (SEC-STD-031); check
  that the live audit log extends the backup's signed checkpoint
  (SEC-OPS-024); keep each library root's path as the backup recorded it,
  so a root that is not present on the new machine shows as offline and
  nothing under it is removed (SEC-TM-069; remapping roots in a dry run is
  R1.1, WP-150); show the old address from the backup, recommend keeping it, and warn when the domain changed
  (flows G3, SEC-NET-072). When it changed, the restore offers recovery
  enrolment links, which are WP-106's links under its rules, not a
  separate sign-in path: administrators issue them for members and guests,
  the owner for administrators, and nobody for the owner, who uses host
  recovery (SEC-IAM-091, SEC-IAM-092); each link is redeemed in person (a
  QR code on the issuer's screen) or on a device the person already
  approved, and the credential it enrols starts the SEC-IAM-106 recovery
  hold with the person's history hidden until the hold ends. The first
  draft offered "one-time sign-in links" without these rules, which would
  have been a recovery path around the in-person rule and the hold
  (api-needs flag 6, D-07). After any restore,
  rotate every symmetric key, invalidate every session and signed URL,
  and give the owner a "review devices and access" alert listing every
  restored device, credential and admin (SEC-OPS-044); rebuild the cache
  after. The home posture and every security setting survive the restore
  (SEC-OPS-038).
- **Tests (real SQLite and files).** A backup made by WP-090 restores to a
  fresh directory and the queue, playlists, loves and accounts match; a
  backup with a bad signature or a flipped encryption tag is refused
  before any write; a hostile archive (too many entries, a path with
  `..`, a symlink entry, an entry larger than declared) is refused; a
  hostile database (a crafted schema, a trigger, a view, an oversized
  cell) at restore-at-setup changes nothing, and the database fuzz
  target runs in CI; a restore attempted on an unclaimed server without
  the setup code is refused; an upload whose name and `Content-Type`
  claim a backup but whose bytes are not one, an oversized upload and a
  truncated one are each refused before anything is written, and the
  stored upload's name contains nothing from the request; a backup whose
  library roots are absent restores with those roots offline and every
  item kept; back up, revoke a device, restore: the
  device's old session fails and the review alert names it; rewriting
  audit history after a backup makes the checkpoint check fail. Restore
  under a new domain: the restore offers no link for the owner; a link
  for a member, redeemed on a browser that is neither the issuer's
  in-person QR session nor a device the member already approved, is
  refused; a link redeemed in person enrols a credential that is in the
  recovery hold, cannot export history, and shows none of the member's
  history until the hold ends.

### WP-121 Release builds, container image and service install (added in review)

- **Wave** 4 · **Size** M · **Depends on** WP-043, WP-045, WP-089, WP-118.
- **Owns** `.github/workflows/release.yml` (until WP-136 takes it over in
  wave 5), `packaging/` except `packaging/proxies/` (the container build
  file, a compose example, NAS templates, the systemd unit template and
  an install script), `crates/gunmetal-server/src/service/`.
- **Serves** ADM-002, ADM-003, ADM-005; ADM-128 (the container health
  check uses WP-089's route). No package served these R1 rows. (Builds
  for small ARM boards, including 32-bit, ADM-004, are R1.3, WP-160: the
  owner's answer to D-09 makes R1 servers Linux on x86-64 and ARM64 plus
  a Docker image.)
- **Security.** Boundaries TB12; threats TM-T54. Verifies SEC-TM-012,
  SEC-TM-041, SEC-IAM-005, SEC-MED-042, SEC-OPS-038, SEC-OPS-046,
  SEC-OPS-053, SEC-OPS-056, SEC-OPS-057, SEC-SUP-045, SEC-SUP-046.
- **Scope.** Linux release builds only, as the baseline's release scope
  says (macOS and Windows server builds wait for their sandbox profiles,
  SEC-MED-082 in R2; the first draft's owner decision 18 would have built
  them with a reduced tier): x86-64 and 64-bit ARM only (owner answer to
  D-09; the 32-bit ARM build is R1.3, WP-160); the release profile's
  `overflow-checks`; no default
  account, password, key or secret in any binary, image or package, and a
  secret scan over every built artefact (SEC-TM-012, SEC-IAM-005). A
  container image built from distroless static or cc, or scratch, pinned
  by digest, with no shell or package manager, running as a fixed non-root
  UID and GID and working under any other non-root UID, with a read-only
  root filesystem, all capabilities dropped, `no-new-privileges`, the
  default seccomp profile and library folders mounted read-only
  (SEC-SUP-045, SEC-SUP-046, SEC-MED-042, SEC-OPS-057, SEC-OPS-053); the
  same for the compose example and the NAS templates, none of which may
  need `privileged`; documented volumes for data, cache and media, and a
  health check; the home posture as the default in every package and
  image (SEC-OPS-038); the binary installed so the service account cannot
  write it or its install directory (SEC-OPS-046); version-pinned tags; a
  `gunmetal service print-unit` subcommand that renders the systemd unit
  for the configured paths and user, under a dedicated system account
  with no login shell, scoring 2.0 or lower in `systemd-analyze security`
  (SEC-OPS-056, SEC-TM-041). The privileged steps
  (creating the service user, installing and enabling the unit) are in
  `packaging/install.sh`, not in the binary, because SEC-MED-063 forbids
  the server from starting any program outside the sandbox launcher, so
  the binary cannot run `useradd` or `systemctl` itself. Whether that
  still meets ADM-005's "one command" is owner decision 35.
- **Not in scope.** Signing, provenance, SBOMs, reproducibility and image
  scanning, which WP-136 adds in wave 5. Windows services and launchd
  (R2, ADM-013). The hardware guide and install docs (ADM-011, ADM-003's
  release notes), which are documentation, not backend code.
- **Tests.** The rendered unit for a given configuration equals a literal
  expected unit file and sets the hardening options the security baseline
  names; paths with spaces and non-ASCII characters are escaped;
  `systemd-analyze security --offline=yes` on the rendered unit scores 2.0
  or lower in CI. The install script and the image are outside the Rust
  gate's coverage and mutation runs: they get `shellcheck`, an xtask lint
  of the shipped templates, a container test that starts the image with
  `--read-only --cap-drop=ALL --security-opt=no-new-privileges` and an
  arbitrary non-root UID and asserts a non-zero UID, empty capability
  sets, the home posture and a passing health check, a test that the image
  started as root refuses to run, and a smoke run of the 64-bit ARM build
  under emulation if CI can provide one (unverified). That is weaker than
  the gate, and this package says so in its pull request rather than
  hiding it.

### WP-133 History deletion, retention and account deletion (added for the security baseline)

- **Wave** 4 · **Size** M · **Depends on** WP-062, WP-068, WP-069, WP-070,
  WP-086, WP-094, WP-138.
- **Owns** `crates/gunmetal-server/src/erasure/`. The retention schedule,
  `crates/gunmetal-core/src/retention.rs`, moved to WP-138 (wave 1) in
  the second security pass, because the audit log, log rotation and
  backups need it two waves earlier.
- **Serves** ACC-118 (remove plays from history), ACC-010 (account
  deletion); API-LOG-03.
- **Security.** Boundaries TB10; threats TM-T18, TM-T59. Verifies
  SEC-PRV-003 and SEC-PRV-005 (the daily sweep job and the clock-injected
  test for each data class; the schedule's core property test is
  WP-138's, the session clause WP-062's and the audit clause WP-069's),
  SEC-PRV-048, SEC-PRV-049, SEC-PRV-050, SEC-PRV-051, SEC-PRV-052,
  SEC-IAM-103, SEC-OPS-026, SEC-TM-055.
- **Why it exists.** As first written, removing a play hid it through a
  removal event, and the user log could erase, but nothing deleted
  history on request, enforced retention or deleted an account. The
  baseline requires real erasure within 24 hours (SEC-PRV-049) and one
  retention schedule (SEC-PRV-005).
- **Scope.** The daily sweep job that enforces WP-138's retention
  schedule for every data class, and the schedule's display in admin
  settings and the privacy notice (SEC-PRV-005, SEC-TM-055); for the
  audit log, the sweep calls WP-069's retention API, which coarsens
  addresses at 30 days, removes them at 90 and prunes records at 365,
  each run recorded as a signed checkpoint, and this package never edits
  audit files (SEC-PRV-003, SEC-OPS-026); session records lose their
  address when the session ends, which WP-062 does, and the sweep removes
  any left by a crash. History deletion of one entry, a
  time range or everything: the data leaves the user log (WP-068's
  erasure rewrite), the cache, derived tables, search indexes and caches
  within 24 hours, with the WAL checkpointed and truncated so nothing
  survives in free pages (SEC-PRV-049, SEC-PRV-050); tombstones sent to
  devices name erased events only by ID or ID range (SEC-PRV-052).
  Account deletion: needs authentication within the last 5 minutes and is
  rate-limited (SEC-PRV-048); disables the account and ends every session
  and device at once, blocks sign-in, keeps the data restorable for a
  7-day grace period during which the person can still export it, then
  erases it through the history-deletion pipeline (SEC-PRV-051,
  SEC-IAM-103).
- **Tests (real SQLite and log files).** With sentinel bytes in history,
  deleting one entry, a range and everything leaves no sentinel in any
  file of the data directory after the job (a raw-byte scan); a tombstone
  carries no erased content (compared with a literal encoding); account
  deletion across the grace period on a manual clock (restorable on day
  6, erased on day 8); retention expiry for each class on a manual clock,
  with the audit log's address coarsening and removal observed through
  WP-069's API and `audit verify` passing after the sweep; running the
  sweep twice equals running it once.

## Waves 5 and 6: health, jobs, benchmark and acceptance

WP-109 (restore, numbered in the wave 4 section) also runs in wave 5.
Playlist files (WP-112), the parser-upgrade re-read (WP-123) and metadata
providers (WP-137) left R1 for R1.1; the neighbour and rule jobs (WP-113)
and loudness analysis (WP-114) for R1.3; and the naming client with CT
monitoring (WP-135) for R2. Their specifications are in
[After R1](#after-r1-point-releases-and-later).

### WP-110 Library health and root health

- **Wave** 5 · **Size** M · **Depends on** WP-045, WP-060, WP-078, WP-097,
  WP-102, WP-119. (The admin home's roll-up, which reported backups,
  advisories and recovery use and so needed WP-090, WP-074 and WP-106, is
  R1.2 and moved to WP-155; token expiry left with WP-091 for R2.)
- **Owns** `crates/gunmetal-server/src/health/`.
- **Serves** MUS-044, LIB-014 (warning), LIB-032, LIB-193, ADM-108,
  MUS-229; API-HLTH-01, API-HLTH-02. (Tag problems with suggested fixes
  and the missing-files list, LIB-194 and LIB-034, are R1.1, WP-149; the
  health summary on the admin home, ADM-109 and API-HLTH-05, is R1.2,
  WP-155.)
- **Security.** Boundaries TB6, TB9; threats TM-T53. Verifies SEC-MED-019,
  SEC-MED-024, SEC-TM-045.
- **Scope.** The health report (damaged and unreadable files, sidecar
  problems, formats a supported browser cannot decode, offline roots,
  watch warnings) and each root's reachability. The report lists
  quarantined files with the reason (SEC-MED-019) and shows the isolation
  tier the worker reached in plain language, with the "reduced isolation"
  notice when a control is missing (SEC-MED-024, SEC-TM-045). The report
  is a list of typed entries, so WP-149 adds its entry kinds in R1.1
  without changing the others.
- **Not in scope.** Tag problems, same-name collisions and missing or
  moved files (WP-149, R1.1). The admin home's roll-up (WP-155, R1.2).
- **Tests (real SQLite).** A synthetic library with one of each R1 problem
  yields exactly one entry of each kind; an offline root shows as offline
  and its items as greyed, not missing; a quarantined file appears with
  its reason; with each isolation probe forced to fail in turn, the page
  shows the exact notice text.

### WP-111 Trash and purge (was: review queue, trash and purge)

- **Wave** 5 · **Size** S (was M; the review queue moved to WP-107 in
  R1.3, because the review queue, LIB-099, is R1.3) · **Depends on**
  WP-070, WP-093 (the purged-track test reads a playlist), WP-102.
- **Owns** `crates/gunmetal-server/src/trash/` (the first draft's
  `review/` is created by WP-107 in R1.3).
- **Serves** LIB-033, ADM-086; API-HLTH-04. (The review queue, LIB-099
  and API-HLTH-03, is R1.3, WP-107.)
- **Security.** Boundaries TB9; threats TM-T56. Verifies SEC-TM-069.
- **Scope.** The trash of items whose files went missing with their purge
  date; restore and purge now; the purge job, which never runs while a
  root is offline.
- **Not in scope.** Doubtful decisions and the review queue (WP-107, R1.3).
- **Tests (real SQLite and log files).** A purge is skipped while a root
  is offline; an item restored from the trash keeps its history; a purged
  track becomes a "missing" entry in a playlist.

### WP-115 Scan benchmark and speed budget tests (was: scan benchmark)

- **Wave** 5 · **Size** M · **Depends on** WP-054, WP-067, WP-084, WP-102,
  WP-119.
- **Owns** `crates/xtask/src/bench.rs`. (The generator moved to WP-119 in
  wave 2, because wave 3 and 4 tests needed it.)
- **Serves** README roadmap item "Benchmark: scan time against Jellyfin";
  ADR 1 consequences; LIB-019; the R1 budget tests behind DIS-084 and
  CLI-022, and the data side of DIS-100's (register D-87). Its
  measurements also feed two later rows that need no further code: the
  published footprint numbers (ADM-010, R1.2) and the published speed
  numbers (DIS-019, R1.1).
- **Security.** Boundaries TB9; threats TM-T20. Verifies no requirement of
  its own: it holds no security control, and the rules it relies on are
  proved by the packages that own them.
- **Scope.** Generate a library of a chosen size with WP-119; run
  Gunmetal's scan on it and record time, bytes read, peak memory and
  database size; record batch commit time for WP-067; document how to run
  the same library through Jellyfin for the comparison, which is a manual
  run outside CI. The speed budget tests (register D-87): the budgets are
  enforced in the R1 gate, and only publishing the numbers waits for
  DIS-019 in R1.1. On the reference low-end profile (2 cores and 1 GiB
  enforced with cgroups, the profile WP-117's load test uses, until the
  owner names the reference devices under owner decision 15), the gate
  fails when, at 100,000 synthetic tracks, the search index build time
  or memory or a search or browse query (WP-054), or the sync snapshot's
  bytes or time (WP-084), exceed DIS-019's design goals. The render budget
  for long lists (DIS-100) is the client plan's test in the same gate,
  against the same goals and on a library from WP-119.
- **Tests.** The benchmark's report format, written literally; the runner
  on a ten-file library produces a report with every field present. Each
  budget test is checked to fail: run against a deliberately slowed
  stand-in for the index build and for the snapshot, it reports the
  budget it exceeded.
- **Risks.** A synthetic library made of tiny files is not a real library:
  real files are larger and real tags messier, so the published comparison
  needs the owner's choice of library (owner decision 22). Jellyfin may be
  faster on some steps; the README promises to publish the numbers either
  way.

### WP-136 Release provenance, signing, SBOMs and the feed publisher (added for the security baseline)

- **Wave** 5 · **Size** L · **Depends on** WP-074 (the feed format),
  WP-121, WP-124, WP-127.
- **Owns** `.github/workflows/release.yml` (handed over by WP-121 at the
  start of wave 5), `.github/workflows/build-reusable.yml`,
  `.github/workflows/post-release.yml`,
  `.github/workflows/release-watch.yml`, `crates/xtask/src/release.rs`,
  `crates/xtask/src/feed.rs`, `crates/xtask/src/js_install.rs` (the lints
  of the package-manager configuration the release workflow uses).
- **Serves** ADM-003 (release notes), ACC-126 (an advisory for every fix);
  the baseline's "every release" checks.
- **Security.** Boundaries TB12; threats TM-T41, TM-T42, TM-T43. Verifies
  SEC-SUP-009, SEC-SUP-015, SEC-SUP-016, SEC-SUP-017, SEC-SUP-039,
  SEC-SUP-040, SEC-SUP-041, SEC-SUP-042, SEC-SUP-043, SEC-SUP-044,
  SEC-SUP-048, SEC-SUP-049, SEC-SUP-053, SEC-SUP-056, SEC-STD-005,
  SEC-STD-018, SEC-STD-033, SEC-STD-034, SEC-HIS-059, SEC-HIS-065,
  SEC-OPS-052, SEC-OPS-068, SEC-OPS-071, SEC-NET-057, SEC-CLI-016,
  SEC-CLI-017, and, for the release workflow's install of the web
  bundle's JavaScript packages, SEC-SUP-033, SEC-SUP-034, SEC-SUP-036,
  SEC-CLI-018 and the JavaScript licence check of SEC-SUP-029 (the same
  rules in the client's own CI belong to the client plan).
- **Why it exists.** WP-121 left signing to an owner decision on keys
  (this plan's decision 32). The baseline does not need long-lived keys
  for artefacts: it requires Sigstore keyless signing, SLSA Build Level 3
  provenance, reproducible Linux builds and SBOMs in R1 (SEC-SUP-040 to
  SEC-SUP-044), and TUF metadata with offline keys only for the feed
  (SEC-SUP-049). No package built any of it.
- **Scope.** Release compilation in an isolated reusable workflow, with no
  network after the locked fetch, no Actions cache, no artefacts from
  pull-request workflows, and only short-lived credentials, in a protected
  `release` environment that accepts only protected tags and needs a
  maintainer's approval (SEC-SUP-015 to SEC-SUP-017, SEC-SUP-039). The
  release build compiles the web bundle that the server embeds, so it
  installs npm packages, and that install follows the baseline's
  JavaScript rules rather than waiting for a client plan: frozen to the
  committed lockfile, resolving only from `registry.npmjs.org`, with
  dependency lifecycle scripts and implicit `node-gyp` builds disabled
  except for a reviewed allow-list (SEC-SUP-033, SEC-CLI-018); refusing
  versions published less than 7 days earlier (SEC-SUP-034); verifying
  registry signatures and provenance attestations of every installed
  package (SEC-SUP-036); and checking every JavaScript component's
  licence against the same allowlist file `cargo deny` uses
  (SEC-SUP-029). An xtask lints the package-manager configuration that
  `build-reusable.yml` uses for each of these;
  reproducible Linux binaries checked by two builds on separate runners
  (SEC-SUP-040); SLSA Build Level 3 provenance for every artefact in the
  Sigstore transparency log (SEC-SUP-041); a `SHA256SUMS` manifest with a
  Sigstore bundle and copy-paste verification commands that a post-release
  job runs (SEC-SUP-042); container images signed by digest with
  provenance and SBOM attestations, and no exact-version tag pushed twice
  (SEC-SUP-043); CycloneDX 1.7 SBOMs for the Rust crates, the web bundle's
  JavaScript packages and bundled C code, with the cryptographic
  inventory as cryptographic assets (SEC-SUP-044, SEC-STD-018,
  SEC-NET-057, SEC-CLI-017); image and dependency scanning before
  publishing and daily (SEC-SUP-048); a secret scan of every artefact,
  web bundle included (SEC-CLI-016); the binary hardening check (PIE, full
  RELRO, non-executable stack, `overflow-checks`; SEC-STD-033). Release
  gates: the security evidence bundle (coverage, mutation, fuzz
  statistics, cargo-deny and cargo-vet, CodeQL, Scorecard, traceability
  and standards coverage) published with provenance (SEC-STD-005); a
  dated non-author review record before the R1 tag (SEC-STD-034); release
  notes saying whether the release migrates data, can be rolled back and
  changes security defaults (SEC-OPS-052); a unique version and a
  changelog Security section listing fixed advisories, each published as
  a GitHub Security Advisory with a CVE (SEC-SUP-056, SEC-SUP-009,
  SEC-HIS-065). The feed publisher: TUF 1.0 metadata for update notices
  and advisories with severity, affected ranges, fixed version and an
  exploited flag, signed by the offline keys (SEC-SUP-049, SEC-OPS-068),
  and a root-rotation rehearsal whose rotated test feed the latest
  release build accepts (SEC-OPS-071). A scheduled job compares published
  tags, releases and images with the release workflow's own log and
  alerts within an hour (SEC-SUP-053). Every client and server download
  verifies a signature before use (SEC-HIS-059).
- **Tests.** The xtask checks against fixture releases: a missing SBOM, a
  provenance subject that does not match, a changelog without the
  Security section, a migration without the release-notes section and a
  missing review record each fail; the hardening checker on a deliberately
  non-PIE binary fails; the post-release job verifies a real test release
  with the documented commands; the feed publisher's output verifies
  through WP-074's client, and a rotated root is accepted. The
  package-manager lint against fixture configurations: a missing frozen
  lockfile flag, a non-registry source, lifecycle scripts enabled, a
  release-age floor under 7 days, a missing signature-verification step
  and a disallowed JavaScript licence each fail with their names; a CI
  canary installs a local test package whose `preinstall` script and
  `binding.gyp` each try to write a marker file, and asserts neither
  marker exists.
- **Risks and decisions.** The offline TUF keys and the second maintainer
  are people and hardware, not code (baseline owner decision 19; this
  plan's decision 32). Reproducibility of the 32-bit ARM build, which
  joins in R1.3 (WP-160), is unverified.
- **Added for the client plan (2026-10-03).** The release workflow's check of the
  package-manager settings is the client plan's CP-001 check, run here;
  this package does not write a second one.

### WP-116 Doctor and the security summary (was: doctor, diagnostics bundle and emergency page)

- **Wave** 6 · **Size** S (was M; the diagnostic bundle, device reports
  and the emergency page moved to WP-155 in R1.2) · **Depends on** WP-074,
  WP-090, WP-097, WP-101, WP-110, WP-132.
- **Owns** `crates/gunmetal-server/src/diagnostics/`.
- **Serves** ADM-123, ADM-142. (The emergency page, the diagnostic bundle,
  diagnostics a person reads first, local crash records and the
  write-queue view, ADM-113, ADM-124, CLI-033, ADM-130, the ADM-080 view,
  API-SYS-08, API-SET-08 and API-DEV-03, are R1.2, WP-155; the
  derived-data store's part, ADM-141, arrives with WP-071.)
- **Security.** Boundaries TB10, TB11; threats TM-T53, TM-T57. Verifies
  SEC-OPS-054, SEC-OPS-061, SEC-HIS-055, SEC-MED-024. (SEC-OPS-030 and
  SEC-PRV-046 moved with the diagnostic bundle to R1.2, WP-155;
  SEC-OPS-050 and SEC-OPS-059 stay proved in R1 by WP-080, WP-089, WP-095
  and WP-131.)
- **Scope.** `gunmetal doctor --security` and the dashboard's security
  status: root and capability state, file permissions, a media root or
  the binary writable by the service account, internet exposure, trusted
  proxies, backup age and encryption, audit verification, the version and
  its advisories, the update check's state, and the isolation tier
  (SEC-OPS-061, SEC-OPS-054, SEC-MED-024), plus `--fix-perms`; the
  security summary logged at every start and shown on the admin home
  (ADM-142): listening addresses, whether HTTPS is active and through
  which path (own domain, tailnet, reverse proxy or localhost), trusted
  proxies, enabled providers (none in R1) and admin accounts
  (SEC-HIS-055).
- **Not in scope.** The diagnostic bundle, accepting a person's device
  report and the emergency page (WP-155, R1.2).
- **Tests.** Doctor's checks each have a passing and a failing case, one
  per failure mode; a snapshot of the start-up summary for fixed
  configurations, one per HTTPS path.

### WP-117 R1 flow acceptance tests

- **Wave** 6 · **Size** M · **Depends on** every R1 package (waves 0 to
  5; none of the packages in [After R1](#after-r1-point-releases-and-later)).
- **Owns** `crates/gunmetal-server/tests/flows/`.
- **Serves** flows.md F01 to F03, F05, F06, F10, F12 to F16 (server side),
  each in the form the adopted R1 has (for example F14 restores with the
  same library paths, and F01 claims over each of R1's HTTPS paths).
- **Security.** Boundaries TB1, TB2, TB4, TB8, TB9; threats TM-T33, TM-T56.
  Verifies SEC-TM-042, SEC-TM-048, SEC-TM-053 (the server part: no
  outbound socket over a full session; the client run behind a deny-all
  proxy belongs to the client plan), SEC-NET-032, SEC-NET-055,
  SEC-OPS-007, SEC-OPS-060, SEC-PRV-004, SEC-PRV-007, SEC-PRV-009,
  SEC-IAM-047, SEC-OPS-033 (the next range request fails after "This
  wasn't me"), SEC-OPS-027 (the investigation-mode alert reaching the
  person), SEC-OPS-037 (a spoofed header leaves the audit record, the
  limiter key and the exposure state unchanged), and the end-to-end
  audit records of SEC-OPS-029, SEC-PRV-008, SEC-IAM-069, SEC-TM-014 and
  SEC-HIS-046.
- **Scope.** One end-to-end test per flow's main path against a real
  server process on a real port with a real data directory and a synthetic
  library: claim and owner creation, run once for each R1 way to a
  secure context (localhost; an own domain with a certificate from a
  local ACME test server through WP-101; a trusted reverse proxy that
  terminates TLS in front of the server, standing in for the owner's
  proxy or a tailnet); add a library and scan; a new device
  signs in and takes a snapshot; play an album (sign, range, refresh,
  packaging); build a playlist; invite a friend who sees only their
  libraries; an unplayable file explained; back up, wipe and restore. Their
  failure branches stay at lower layers, as flows.md asks. Review added
  the cross-package round trips that no single package could test because
  their parts landed in the same wave: a cache rebuild brings back the
  queue, loves and playlists unchanged; the active device change
  reaching a second browser over the event channel. (The artist-merge
  round trip moved with curation to WP-107 in R1.3.) (The path-scoped refresh under a scoped token left
  with WP-091 for R2.) The whole-system security tests, which need every
  package: in a network namespace with a test DNS server and an egress
  recorder, a fresh server completing setup, scanning, browsing,
  searching and playing makes no outbound connection or non-local DNS
  lookup beyond the egress inventory's rows for its configuration
  (SEC-TM-048, SEC-TM-053, SEC-NET-032, SEC-OPS-007, SEC-OPS-060,
  SEC-PRV-007, SEC-PRV-009); the flows run against read-only bind mounts
  and nothing under a media root changes (SEC-TM-042); a sentinel search
  term never reaches the database or a log (SEC-PRV-004); a log-canary
  scan over the whole run finds no stream signature, invitation, share or
  pairing secret (SEC-IAM-047; share secrets and OAuth codes and states
  join the canary when WP-134 and WP-096 land in R1.2); with a stream
  playing, "This wasn't me" on the new-device alert makes the next range
  request fail within one URL lifetime (SEC-OPS-033); entering the
  audit investigation mode puts an alert in front of each affected
  person (SEC-OPS-027); a forwarding header spoofed from an untrusted
  peer leaves the audit record's source, the limiter key and the
  exposure state exactly as the bare peer gives them (SEC-OPS-037); each
  pathway in the verifier's inventory driven past its limit, a debug-level
  switch, an egress denial and an authorisation fault each leave exactly
  the expected audit record, read back through `audit verify` and the
  owner's reader (SEC-TM-014, SEC-HIS-046, SEC-OPS-029, SEC-PRV-008,
  SEC-IAM-069); and a nightly load
  test on the reference low-end profile (2 cores and 1 GiB enforced with
  cgroups) where a 2 Mbit/s stream plays without underrun while the server
  receives 2,000 idle slow connections (SEC-NET-055).
- **Tests.** These are the tests.

## Coverage check: every R1 capability has an owner

This table maps every capability in [api-needs.md](api-needs.md) that
api-needs.md puts in R1 to the packages that deliver it. Where the
adopted R1 (register D-10) moved a capability, or part of one, to a point
release, the row says so and names the package in
[After R1](#after-r1-point-releases-and-later) that carries it; R1's waves
hold only the R1 part. The R2 rows in that document are covered in the
outline below.

| Capability | Packages |
|---|---|
| API-SYS-01 health; SYS-03 negotiation; SYS-04 discovery; SYS-06 sign-in facts; SYS-09 API reference | WP-089 (with WP-039, WP-044, WP-118) |
| API-SYS-02 secure-context report | WP-073 |
| API-SYS-05 who am I | WP-087 |
| API-SYS-07 startup page | WP-095 |
| API-SYS-08 emergency page | R1.2: WP-155 |
| API-SYS-10 client event channel | WP-083 |
| API-AUTH-01 to AUTH-03 claim, setup, owner | WP-080; the passkey branch of owner creation and the recovery codes offered with it in WP-106 (with WP-081 and WP-063) |
| API-AUTH-04 passkeys | WP-081 (with WP-041) |
| API-AUTH-05 password and two-factor | Withdrawn for R1 by the security baseline (SEC-IAM-025; baseline owner decision 1). A browser that cannot use a passkey signs in by approval from the person's own device: WP-120 (with WP-038) |
| API-AUTH-06 single sign-on | R1.2: WP-096 |
| API-AUTH-07 guessing limiter | WP-064, the credential verifier (with WP-032), applied to every R1 pathway: WP-080, WP-081, WP-063, WP-094, WP-120; the R1.2 pathways join with WP-096 and WP-134 |
| API-AUTH-08, AUTH-09 sessions and epochs | WP-062 (passkey sign-in in WP-081, sign-out in WP-120, cut-off in WP-082, push in WP-083) |
| API-AUTH-10 to AUTH-12 step-up and recovery | WP-106 |
| API-AUTH-13 pairing (R1: browsers) | WP-120 (with WP-038); native device keys R2 |
| API-AUTH-15 recovery codes | WP-063, WP-106 (ACC-137) |
| API-USR-01 profile identity | WP-087 (names); R1.1: WP-144 (pictures) |
| API-USR-02 settings | WP-087 |
| API-USR-03, USR-04 sign-in methods and history | WP-106 (with WP-069) |
| API-USR-05 export | WP-108 |
| API-USR-06 history import | R1.1: WP-145 (with WP-057) |
| API-USR-07 private session | WP-086 |
| API-USR-09 what the admin can see | WP-087 (ACC-115) |
| API-USR-10 delete my account | WP-133 (ACC-136) |
| API-DEV-01 device registry | WP-087 (with WP-062) |
| API-DEV-02 sync status | WP-084 |
| API-DEV-03 diagnostics from a device | R1.2: WP-155 |
| API-DEV-05 new-device notice | WP-097 (ACC-071) |
| API-SYNC-01 to SYNC-04 snapshot, delta, removals, profile data | WP-084 (with WP-066, WP-039) |
| API-SYNC-05 artwork sizes | WP-079, WP-103, WP-037 |
| API-SYNC-06 neighbour table | R1.3: WP-058, WP-113 |
| API-SYNC-07 prebuilt search index, if needed | WP-054 (serialised form); shipping it is a small follow-up if the budget is missed |
| API-SYNC-10 erasure tombstones | WP-133, WP-084 |
| API-SYNC-11 purge on revocation | WP-062, WP-120, WP-084 |
| API-LIB-01 to LIB-04, LIB-07 libraries, folders, roots, grants, splitting rules | WP-099 (with WP-053, WP-060, WP-098; the grants table in WP-065); exclusions R1.1: WP-140 |
| API-LIB-05 location | R1.1: WP-150 |
| API-LIB-06 rebuild | WP-095 |
| API-CAT-01 to CAT-08 catalogue fields | WP-040, WP-049 to WP-053, WP-075, WP-076, WP-077; seek and frame indexes stored by WP-067 and fetched through WP-084 (CAT-05); release groups R1.1: WP-146 |
| API-CAT-09, CAT-10 availability and folder paths | WP-102, WP-110 |
| API-CAT-11 inspect | R1.2: WP-156 |
| API-CAT-12 merge and split | R1.3: WP-107 |
| API-CAT-13 rescan one | WP-098 (an admin's path-scoped refresh) |
| API-STR-01 to STR-03 signing, refresh, byte ranges | WP-082 |
| API-STR-04 audio packaging | WP-056, WP-105 |
| API-STR-05 artwork bytes | WP-103 |
| API-SES-01 session registry | WP-104 |
| API-SES-02 stop | R1.2: WP-153 |
| API-SES-03 active player | WP-085 (with WP-083) |
| API-SES-04 play reporting | WP-086 |
| API-SES-08 stream limits (R1 part) | WP-104, WP-130; policy alternatives R2 |
| API-QUE-01 to QUE-04 queue document, operations, rebase, positions | WP-025, WP-026, WP-085 |
| API-QUE-05 save queue as playlist | R1.1: WP-143 |
| API-PL-01, PL-08 playlists and missing entries | WP-093 |
| API-PL-02 pins | R1.1: WP-143 |
| API-PL-07 tool writes | R2: WP-091 |
| API-PL-03, PL-04 M3U and folder playlists | R1.1: WP-022, WP-112 |
| API-PL-05, PL-06 rule store and server evaluation | R1.1: WP-027 (the core rule format and its parser budgets, register D-85) and WP-087 (saved filters as rule documents in the settings records); R1.2: WP-154 (saved filters as Home rows); R1.3: WP-092, WP-113 (the rule store for smart playlists, and server-side evaluation and re-evaluation jobs); tools reading results R2: WP-091 |
| API-LOG-01 to LOG-04 events, offline merge, removal, counts | WP-034, WP-068, WP-086 |
| API-LOG-05 hides | R1.1: WP-141 |
| API-HOME-01, HOME-02 layout and pins | R1.2: WP-154 (the default layout in R1 is WP-059's) |
| API-HOME-03, HOME-04 recently added without upgrades, reasons | WP-077, WP-059 |
| API-HOME-05 search on the device | WP-054, WP-088 |
| API-SCAN-01, SCAN-03 scan and progress | WP-099, WP-102 |
| API-SCAN-02 path-scoped refresh | WP-098 |
| API-SCAN-04, SCAN-05 tasks and activity | WP-070, WP-100 (with WP-069 for the audit store); running and cancelling tasks R1.2: WP-155 |
| API-HLTH-01, HLTH-02 health | WP-110 |
| API-HLTH-03 review queue | R1.3: WP-107 |
| API-HLTH-04 trash | WP-111 |
| API-HLTH-05 health roll-up | WP-116 (the R1 security summary, ADM-142); the full roll-up R1.2: WP-155 |
| API-ADM-01 to ADM-03 users and invitations | WP-094 |
| API-SET-01 network settings | WP-073 (owner certificate), WP-132 (trusted proxies, posture), WP-101 (HTTPS by ACME for an own domain); path prefix R1.2: WP-151; the project name service R2: WP-129, WP-135 |
| API-SET-02 egress gate and network activity | WP-048, WP-097 |
| API-SET-03 sign-in settings | WP-062; OIDC R1.2: WP-096 |
| API-SET-04, SET-05 backups and restore | WP-090, WP-109; restore from the UI R1.2: WP-157; the owner's server export R1.2: WP-158 |
| API-SET-06 updates | WP-074 |
| API-SET-07 alerts and logs | WP-097, WP-043 |
| API-SET-08 diagnostics | WP-116 (the doctor, ADM-123), WP-095 (cache rebuild, ADM-077); the diagnostic bundle, crash records and write-queue view R1.2: WP-155; the derived-data store R1.3 (ADM-141) |
| API-SET-09 server identity | WP-089 |
| API-SET-10 restart and shut down | R1.2: WP-155 |
| API-SET-11 listening-service and playlist imports | R1.1: WP-145, WP-112 |
| API-SET-13 rotate server secrets | WP-106 (ADM-144) |
| API-SHR-01 to SHR-03 music share links | R1.2: WP-134; video share links R2 |
| API-TOK-01, TOK-02 tokens and tool change feed | R2: WP-091 (API keys are R2; baseline owner decision 8) |
| API-TOK-03 deep links | WP-239 (the parser); WP-089 (resolution / R1's links); the app's deep links R1.2: WP-159 |

The security baseline added R1 surfaces that api-needs.md now carries as
capabilities and this table maps above: browser pairing (API-AUTH-13,
WP-120), recovery codes (API-AUTH-15), the new-device notice
(API-DEV-05), stream limits (API-SES-08), secret rotation (API-SET-13),
erasure tombstones and purge on revocation (API-SYNC-10, API-SYNC-11),
what the admin can see (API-USR-09) and account deletion (API-USR-10,
WP-133). Music share links (API-SHR-01 to SHR-03) are R1.2 (WP-134). The
security process and release work (WP-124, WP-127, WP-136) has no API
capability. Metadata providers (R1.1, WP-137) and the name service and
its client (R2, WP-129, WP-135) are later releases.

The background jobs in api-needs.md map the same way: library scan to
WP-102 and the parser-upgrade re-read to WP-123 (R1.1); change detection
and path refresh to WP-098; artwork processing to WP-079 and WP-103;
loudness to WP-114 (R1.3); neighbours and server-side rules to WP-113
(R1.3); playlist files to WP-112 (R1.1); import matching to WP-145 and
WP-112 (R1.1); change-log compaction to WP-066; root health to WP-110;
trash purge to WP-111; backups to WP-090; pre-upgrade snapshots and cache
rebuilds to WP-095; user-log recovery to WP-068; the update check to
WP-074; the free-space guard, alerts and log rotation to WP-097, and
crash records to WP-155 (R1.2); expiry sweeps to WP-070 with each owner
registering its own; open-response tracking to WP-082; the retention
schedule to WP-138, and the retention sweep and erasure to WP-133;
certificate renewal to WP-101; CT monitoring to WP-135 (R2).

### R1 feature rows with no backend package

Review compared every R1 ID in the register's adopted R1 (D-10, 266
owning rows and 30 reference rows) with the IDs the R1 packages serve.
The row D-83 added later, MUS-236 (the minimal track details view), is
served by WP-055 and WP-088. The rows that no package names fall into
these groups.

- **Served, but not named in a Serves field.** The security alignment
  added rows to the feature map after most packages were written, and the
  packages carry them under the security requirements they verify: ACC-007
  (no user list before sign-in) by WP-120 and WP-131; ACC-009 and ACC-136
  (deletion with a grace period, deleting your own account) by WP-133;
  ACC-071 (new-device alerts) by WP-097; ACC-076 (device limits) by WP-087
  and WP-130; ACC-137 and ACC-138 (recovery codes, the recovery hold) by
  WP-063 and WP-106; ACC-139 (your own security log) by WP-069 and
  WP-106; ADM-068 (encrypted backups) by WP-090; ADM-111 (retention and
  anonymisation of activity) by WP-069, WP-133 and WP-138; ADM-121
  (secrets that cannot reach logs) by WP-043 and WP-047; ADM-143 (the
  recovery kit) by WP-080 and WP-090; ADM-144 (rotate every server key) by
  WP-106; ADM-145 (audit verification and an anchor off the server) by
  WP-069, WP-090 and WP-100; ADM-146 (outbound proxy and offline mode) by
  WP-048; ADM-147 (configuration changes made outside the server) by
  WP-097; ADM-148 (the compromise runbook) by WP-124; DIS-186, DIS-187,
  MUS-233 and MUS-234 (clearing history, choosing how long it is kept) by
  WP-133 and WP-138; DIS-188 (history held during account recovery) by
  WP-106; DIS-189 ("only you can see this") by WP-086 and WP-087; LIB-205
  and LIB-206 (quarantined files, scanner isolation status) by WP-078 and
  WP-110; LIB-207 (unresponsive storage pauses one folder) by WP-060 and
  WP-078; CLI-155 (personal or shared browser) by WP-062; CLI-156
  (signing out leaves nothing behind) by WP-120's `Clear-Site-Data`, with
  the browser half in the client plan; CLI-157 (private session on every
  device) by WP-086; CLI-159 (outside links say where they go) by WP-005's
  link validator, with the display in the client plan. A later edit of
  those packages should name the rows; the coverage is unchanged.
- **Client only.** These need nothing from the server beyond the synced
  copy and the routes above, so they belong to the client plan, not this
  one: CLI-031, CLI-060, CLI-070, CLI-135, CLI-136, CLI-138 to CLI-142,
  CLI-149, DIS-100, DIS-104, DIS-109, DIS-111, DIS-112, MUS-052, MUS-073,
  MUS-108, MUS-113, MUS-227. (CLI-062, DIS-107, MUS-072 and MUS-076, client
  only too, are R1.1.)
- **Documentation, not code.** CLI-002 (the published browser list) and the
  release notes ADM-003 implies. ADM-011 (the hardware guide, from WP-115's
  numbers) is now R1.2. No package owns them; they need a documentation
  owner (see Review notes).
- **Reference rows.** The 30 references the register lists with the R1
  cut ship with their owning rows and need no package of their own.
- **Partly served, now later.** MUS-114 (the track info sheet) names the
  admin-only inspect API as its source; both are R1.2 or later now (the
  sheet is R1.1, the inspector R1.2, WP-156), so the question in Review
  notes is for the point releases. In R1 the minimal track details view
  (MUS-236, register D-83) shows the R1 rows' details from the synced
  record and the device's own decisions (WP-055), with no inspect API.
- **Changed by the security baseline and the owner's answers.** ACC-052
  (password sign-in) and ACC-053 (two-factor codes) are withdrawn from R1
  (SEC-IAM-025); their plan rows were removed from WP-038, WP-063 and
  WP-120. Music share links (ACC-086 to ACC-089, MUS-151), which the
  baseline moved into R1, are R1.2 in the adopted R1 (WP-134). ACC-049 and
  the INT rows WP-091 served are R2 with it. ADM-023 (a per-server HTTPS
  name from the project name service) is R2 in the register, the feature
  map and this plan, with WP-129 and WP-135 (owner answer to D-07);
  ADM-021, ADM-022, ACC-097 to ACC-099 and CLI-150 carry R1's HTTPS
  through the owner's domain, a tailnet, a reverse proxy or the same
  machine.

## After R1: point releases and later

On 2026-10-02 the owner adopted the smaller R1 proposed in the
register's [R1 scope](../decisions.md#r1-scope) section, with the rest in
the point releases R1.1, R1.2 and R1.3, exactly as that section lists them
(register D-10), and moved the project-run name service, its naming client
and its Certificate Transparency monitoring to R2 (D-07). Waves 1 to 6
build that R1 and nothing else. This section keeps every package, or part
of a package, that serves only a point release or a later release, with
its full specification, grouped by release, so it can be scheduled later.

How to read it:

- **Release** replaces the wave until the release is scheduled. "Was"
  gives the wave the package had when it was planned as R1. A point
  release's packages run after R1 has shipped, so every R1 package is
  merged before any of them starts; their own dependencies on each other
  run forwards only (R1.1, then R1.2, then R1.3, then R2).
- **Moved** packages keep their IDs and their text, with what changed
  marked. **Split** packages are new IDs (WP-140 to WP-161) that carry the
  part of an R1 package that is not R1 (WP-139, split from WP-129, is the
  one new package that stays in R1); the R1 package keeps its ID and
  its R1 part, and both entries name each other.
- A package here that changes a file an R1 package owns does so under the
  merge protocol's interface-change rule, as a later wave's owner of that
  file. Ownership is per wave, so this is not a conflict with R1.
- **Security.** Every requirement in docs/security that protects only a
  surface moving here moves with it, is due in the release that ships the
  surface, and that release cannot ship without it (SEC-STD-004; the
  register's [Security requirements for the proposed R1](../decisions.md#security-requirements-for-the-proposed-r1)).
  None is weakened. The R1 security coverage table at the end of this plan
  lists only R1 requirements and R1 packages, and its last part lists the
  requirements that moved, with the release and package that now carry
  them.
- R1 security requirements that a package here also verifies stay R1
  requirements with an R1 package; the package here re-proves them for
  its own surface.

| Release | Packages moved whole | Packages split from an R1 package |
|---|---|---|
| R1.1 | WP-022, WP-027 (moved from R1.3 by register D-85), WP-057, WP-071, WP-112, WP-123, WP-137 | WP-140 to WP-150 |
| R1.2 | WP-096, WP-134 | WP-151 to WP-159 |
| R1.3 | WP-029, WP-058, WP-092, WP-107, WP-113, WP-114 | WP-160, WP-161 |
| R2 | WP-091 (moved earlier), WP-129, WP-135 | none (WP-139, the project site's security files, split from WP-129 and stays in R1, wave 1) |

### R1.1, bring your music in

Playlist files and history imports, built-in MusicBrainz and cover-art
lookups, ratings, richer credits and browsing, offline loading and
installing the web app, avatars, and continue-on-this-device (register
D-10, "R1.1, bring your music in"). Requirements due here with their
surfaces: SEC-MED-050 and SEC-HIS-018 (playlist files, WP-022, WP-112);
SEC-PRV-014, SEC-PRV-015 and SEC-PRV-017 (providers, WP-137); SEC-MED-061
and SEC-PRV-006 (image uploads, WP-144).

The core rule format and its parser budgets are R1.1 work too (register
D-85): saved filters (DIS-105) are stored in that format from R1.1, so
WP-027 moved here from R1.3 and re-proves SEC-TM-032, SEC-STD-011,
SEC-API-066 and SEC-IAM-070 for rule documents. The rule editor and smart
playlists stay R1.3, with the rule store and server-side evaluation
(WP-092, WP-113).

When R1.1's waves are set, three pairs of its packages edit the same
R1-owned directory under the interface-change rule and must not share a
wave: WP-150 follows WP-140 (both edit
`crates/gunmetal-server/src/libraries/`, WP-099's), WP-147 follows
WP-141 (both edit `crates/gunmetal-core/src/home/`, WP-059's), and
WP-147 follows WP-027 (both add functions to `crates/gunmetal-wasm/`,
the facade crate WP-235 creates).

#### WP-022 M3U and M3U8 parser and writer

- **Release** R1.1 · **Wave** not yet scheduled (was 1) · **Size** M ·
  **Depends on** WP-004, WP-005.
- **Owns** `crates/gunmetal-core/src/m3u.rs`.
- **Serves** MUS-140, LIB-192; API-PL-03, API-PL-04.
- **Security.** Boundaries TB6, TB9; threats TM-T22, TM-T30. Verifies
  SEC-MED-001, SEC-MED-005, SEC-MED-006, SEC-MED-007, SEC-MED-008,
  SEC-TM-032, SEC-HIS-036, SEC-MED-050, SEC-MED-016.
- **Scope.** Parse M3U and M3U8 with `#EXTM3U` and `#EXTINF` duration and
  title, into entries that are either a relative path, an absolute path or
  a URL, each classified so the caller can drop what SEC-MED-050 forbids.
  A URL entry is kept only as a dropped entry with its reason; nothing
  ever fetches it (SEC-MED-016).
  Normalise separators and `.` segments; flag `..` escapes. Write M3U8
  with paths relative to a library root.
- **Not in scope.** Resolving entries to items (WP-112 uses the matcher).
  IPTV M3U (R3).
- **Interface sketch.** `pub fn parse(bytes: &[u8], limits: &Limits) -> Result<Playlist, M3uError>`;
  `pub enum EntryTarget { Relative(Vec<Vec<u8>>), Absolute(Vec<u8>), Url, Dropped(DropReason) }`;
  `pub fn write(entries: &[ExportEntry]) -> String`.
- **Tests.** Windows separators; a UTF-8 BOM; Latin-1 in a `.m3u`;
  `#EXTALBUMARTURL` and `#EXTIMG` dropped; `file://` URLs; `../../etc`;
  100,001 entries; an 8 KiB line. Property: writing then parsing returns
  the same relative paths and titles.

#### WP-027 Rule language (moved from R1.3 by register D-85)

- **Release** R1.1 (was R1.3; moved by register D-85) · **Wave** not yet
  scheduled (was 1) · **Size** L · **Depends on** WP-005.
- **Owns** `crates/gunmetal-core/src/rules/`.
- **Serves** DIS-105 (saved filters are rule documents from R1.1) and the
  core of DIS-119 (the rule format and its parser budgets); API-PL-05 (the
  format of saved rules). It also binds the rule fields to the synced
  records (WP-040's field IDs), adds rule evaluation to the WASM facade
  (WP-088) so the web client applies saved filters, and adds validation of
  saved-filter documents to the settings records (WP-087), one function
  each, under the merge protocol's interface-change rule. The rule editor,
  smart playlists and rule-backed custom rows (DIS-119 to DIS-122,
  MUS-143 to MUS-146, API-HOME-01) stay R1.3 on this format, with WP-092
  and WP-113; saved filters as Home rows are R1.2, WP-154. (Loved tracks
  as a playlist, MUS-149, is R1 and needs no rule: WP-059 and WP-086
  serve it from the person's loves.)
- **Security.** Boundaries TB4; threats TM-T09. Verifies, for rule
  documents, SEC-TM-032 (a rule is parsed under budgets for document
  size, depth and node count, with a typed error and no recursion past
  the limit), SEC-STD-011 (text conditions match through a linear-time
  matcher under a length cap and never compile a backtracking pattern),
  SEC-API-066 (fields, operators and sorts are closed enumerations, so a
  rule carries no query text and the server-side compile in R1.3, WP-092,
  can bind only parameters to static statements) and SEC-IAM-070
  (evaluation sees only the items the caller's visibility predicate
  passed, and a rule that names a playlist or item its viewer may not see
  matches nothing). These remain R1 requirements with their R1 packages;
  this package re-proves them for rule documents.
- **Scope.** A versioned rule tree (all, any, not; comparisons on typed
  fields; "in the last N days"; membership in playlists and loved items;
  limits by count, duration or percentage; sorts; seeded random order),
  its validation (depth, node count, known fields for this version),
  forward-compatible serialisation that keeps unknown nodes so a rule
  written by a newer client survives an older one, and evaluation over any
  record type that implements a field-access trait.
- **Not in scope.** The binding to the catalogue tables for server-side
  evaluation (WP-092, R1.3). The visual rule editor (client, R1.3).
- **Field IDs.** This package defines `FieldId` as a plain `u16` newtype
  and the value types it compares, and imports nothing from `catalog/`.
  WP-040's field table lists numeric codes without importing `rules/`; the
  bindings join the two. (This was first written when WP-040 shared its
  wave, and the separation stays.)
- **Interface sketch.** `pub enum Rule { All(Vec<Rule>), Any(Vec<Rule>), Not(Box<Rule>), Cmp { field: FieldId, op: CmpOp, value: Value }, Unknown(RawNode) }`;
  `pub trait Fields { fn get(&self, f: FieldId) -> FieldValue<'_>; }`;
  `pub fn evaluate<'a, R: Fields>(q: &Query, items: impl Iterator<Item = &'a R>, ctx: &EvalCtx) -> Vec<usize>`.
- **Tests.** Each operator on each value type, including missing fields and
  multi-valued fields ("genre is Jazz" on a track with three genres);
  percentage limits rounding; "last 30 days" at the boundary using an
  injected `now`; seeded random limit. A document past each budget (size,
  depth, node count) is refused with the exact typed error; a text
  condition full of regular-expression metacharacters matches them
  literally; a rule naming a playlist outside the viewer's visibility
  matches nothing. Properties: `All([r])` equals `r`; `Not(Not(r))` equals
  `r`; evaluation is deterministic; a rule with an unknown node
  round-trips byte for byte; validation rejects trees deeper than the
  limit without recursing past it.
- **Risks and decisions.** Saving a filter (DIS-105) is R1.1. When this
  package was R1.3, R1.1 had to save filters in a subset of the rule
  format and this package had to read them later. Since register D-85 the
  rule format ships with saved filters, so a saved filter is a rule
  document from the start and nothing is translated; the rule editor and
  smart playlists in R1.3 read the same documents, which the round-trip
  property above covers.

#### WP-057 Import parsers and matcher

- **Release** R1.1 · **Wave** not yet scheduled (was 2) · **Size** M ·
  **Depends on** WP-036, WP-040, WP-005.
- **Owns** `crates/gunmetal-core/src/import/`.
- **Serves** ADM-042, ADM-043, ADM-044, MUS-189, INT-107; API-USR-06,
  API-SET-11, API-PL-03.
- **Security.** Boundaries TB4, TB6; threats TM-T09, TM-T21, TM-T31.
  Verifies SEC-MED-001, SEC-MED-005, SEC-MED-006, SEC-MED-007,
  SEC-MED-008, SEC-TM-032, SEC-HIS-036, and, together with WP-145 and
  WP-112, which run these parsers in the worker, SEC-MED-018 and
  SEC-MED-020.
- **Scope.** Parsers for the Last.fm and ListenBrainz export files people
  can download (the exact formats must be confirmed against each service's
  current export and are unverified here), and one matcher that maps
  (artist, album, title, duration, MBIDs) to library items with a
  confidence, a reason and an "unmatched" result. Imported plays are
  marked as imported so a scrobbler never resends them.
- **JSON outside the server process.** Some export files are JSON
  (unverified for each service's current export). The core's proposed
  dependencies have no JSON reader. An uploaded export is hostile input,
  so it is never decoded in the server process (principle 2,
  SEC-MED-018). **Proposal:** WP-145's `ImportFile` worker job receives
  the uploaded file by read-only descriptor (SEC-MED-020) and decodes its
  JSON or CSV into plain rows there, with `serde_json` linked only into
  the worker for this job, under a size cap and the step budget; this
  package's parsers, also run in the worker, validate every field into
  typed values (SEC-TM-031), and the server revalidates the typed rows it
  gets back (SEC-MED-023). The first draft decoded the JSON in the server
  process, which principle 2 rules out. The alternative, `serde_json` in
  the core, is part of owner decision 4 and D-02; it would still run only
  in the worker.
- **Interface sketch.** `pub fn match_track(q: &MatchQuery, index: &MatchIndex) -> MatchResult`.
- **Tests.** Exact MBID match; title with "(Remastered 2011)"; differing
  case and diacritics; two candidates with equal scores (unmatched, not a
  guess); a duration off by more than the tolerance. Property: in a
  library where no two items share artist, album, title and duration, an
  item always matches a query built from its own tags, and the match is
  that item. (As first written, the property ignored duplicates, for which
  the rules above require "unmatched".)

#### WP-071 Derived-data store

- **Release** R1.1 · **Wave** not yet scheduled (was 2) · **Size** S ·
  **Depends on** WP-046, WP-126.
- **Owns** `crates/gunmetal-durable/src/derived/`.
- **Serves** LIB-025 (with WP-123), and the stores behind provider
  results (WP-137) in R1.1; ADM-141 and LIB-024 in R1.3, which use the
  same store.
- **Why R1.1.** R1 has no producer that needs the store; its first users,
  the provider lookups (WP-137) and the parser-upgrade re-read (WP-123),
  are both R1.1, so the store arrives with them rather than in R1.3 with
  ADM-141.
- **Security.** Boundaries TB10; threats TM-T60. Verifies SEC-PRV-050
  (every connection of this store; as first written nothing checked its
  connections).
- **Scope.** A SQLite file keyed by (content identity, producer kind,
  producer version) for analysis results and artwork derivatives' metadata,
  read on rebuild so work is never repeated, opened through the one
  connection opener (WP-126) with `secure_delete=ON` and queried only
  through `Query` values. Because WP-090 and WP-095 shipped in R1 without
  it, this package also adds the store to the cache rebuild's inputs
  (WP-095's module) and to the backup's optional parts, left out unless
  chosen (WP-090's module), each under the merge protocol's
  interface-change rule.
- **Tests (real SQLite).** A new producer version misses the old key; a
  rebuild reads existing results; the store reports its file as optional
  for backups, and a backup leaves it out unless chosen; `secure_delete`
  reads back as on for every pooled connection.

#### WP-112 Playlist files: import, export and folder playlists

- **Release** R1.1 · **Wave** not yet scheduled (was 5) · **Size** M ·
  **Depends on** WP-022, WP-057, WP-061, WP-078, WP-079, WP-093, WP-102.
- **Owns** `crates/gunmetal-server/src/playlist_files/`,
  `crates/gunmetal-worker/src/jobs/playlist_file.rs`.
- **Serves** MUS-140, LIB-192, ADM-043, ADM-044; API-PL-03, API-PL-04.
- **Security.** Boundaries TB6, TB9; threats TM-T21, TM-T22. Verifies
  SEC-MED-050, SEC-MED-051, SEC-HIS-018, SEC-API-085, SEC-MED-018,
  SEC-MED-020, SEC-MED-023, SEC-HIS-066 (the replay for the
  playlist-file incident of SEC-HIS-018, a file this package adds to
  `tests/rivals/`).
- **Scope.** Every playlist file is parsed in a worker, never in the
  server process (principle 2, SEC-MED-018; a playlist found in a library
  is a sidecar). Upload M3U and M3U8 files through an upload route that
  declares its types and cap (SEC-API-085); the server stores the upload
  in scratch space without reading it, and the `PlaylistFile` worker job
  receives it, or a `.m3u` file the scan found, by read-only descriptor
  (SEC-MED-020), runs WP-022's parser under the step budget and the
  pool's deadline, and returns typed entries (normalised relative paths,
  with dropped entries and their reasons) that the server revalidates
  (SEC-MED-023). The server then resolves entries with the matcher and
  reports matches and misses; exports as M3U8 with paths relative to a
  library root (writing is server code and needs no parsing); turns
  `.m3u` files found during a scan (the sidecars WP-102 records and
  leaves unread in R1) into read-only playlists with "Duplicate to edit"
  (owner decision 29), adding the read-only flag to WP-093's playlists
  under the merge protocol's interface-change rule, since R1's playlists
  have none. Entries resolve
  only to items already indexed in the same library, by normalised
  relative path; URLs, absolute paths outside the library, `..` escapes
  and artwork directives are dropped and listed in library health, never
  opened or fetched (SEC-MED-050, SEC-HIS-018); playlist entries are
  returned only for items the requester can access, evaluated at request
  time (SEC-MED-051).
- **Tests (real SQLite and files).** Entries pointing outside the library,
  at `/etc/passwd`, at URLs or with `..` are dropped and reported; a
  folder playlist in one library naming items of another library resolves
  nothing there; a second user without access to a playlist's library
  gets none of its entries; a folder playlist is read-only; export then
  import of a playlist gives the same tracks. Property: for generated
  entries, the resolver never returns an item outside the playlist's
  library. Hostile input, with the test-hook feature: a playlist worker
  that panics on a hostile uploaded M3U, and one that loops past its
  deadline on a hostile `.m3u` found during a scan, each fail that file
  with a typed problem in library health while the server keeps serving
  and the scan finishes; a fake worker returning an entry with `..` or an
  absolute path is refused by the server's revalidation; WP-001's
  dependency check shows the server crate has no call path to the M3U
  parser.

#### WP-123 Parser-upgrade re-read (split from WP-102 in review)

- **Release** R1.1 · **Wave** not yet scheduled (was 5) · **Size** S ·
  **Depends on** WP-052, WP-070, WP-100, WP-102.
- **Owns** `crates/gunmetal-server/src/reread/`.
- **Serves** LIB-025, ADM-095; the "Parser-upgrade re-read" job.
- **Security.** Boundaries TB9; threats TM-T20. Verifies no requirement of
  its own: it holds no security control, and the rules it relies on are
  proved by the packages that own them.
- **Scope.** At startup after an upgrade, compare each file's stored
  parser versions (WP-102 stores them) with `PARSER_VERSIONS` (WP-052),
  and request a throttled, resumable re-read of only the stale files
  through the scan's path-set entry point; record an activity entry with
  the count.
- **Tests (real SQLite).** With the parser-version table overridden in the
  test: bumping the FLAC version re-reads every FLAC and no other file;
  an unchanged table re-reads nothing; a restart in the middle resumes
  from the checkpoint and re-reads no file twice.

#### WP-137 Metadata and cover-art providers (added for the security baseline; R1.1 by owner decision D-10)

- **Release** R1.1 · **Wave** not yet scheduled (was 5) · **Size** M ·
  **Depends on** WP-048, WP-071, WP-079, WP-080, WP-102, and on an ADR
  that amends ADR 2's consequence that metadata lookups belong in
  plugins. The owner answered baseline decision 22 on 2026-10-02 (register
  D-10): MusicBrainz and cover-art lookups are built in, in R1.1, behind
  the setup question that lists what each provider receives.
- **Owns** `crates/gunmetal-core/src/provider/` (the lookup-evidence type
  and the response decoders, each with a fuzz harness),
  `crates/gunmetal-server/src/providers/`.
- **Serves** LIB-111, LIB-112, LIB-108 (nothing leaves by default),
  LIB-142 (artwork when none is embedded).
- **Security.** Boundaries TB8; threats TM-T30, TM-T31, TM-T33. Verifies
  SEC-PRV-013, SEC-PRV-014, SEC-PRV-015, SEC-PRV-016, SEC-PRV-017,
  SEC-API-079, SEC-API-080, SEC-API-081, SEC-HIS-024.
- **Why it exists.** The baseline's egress inventory has metadata,
  artwork and lyrics providers off until the owner turns one on, with
  rules for what they may send and when. The plan had no provider
  package, because ADR 2 put providers in plugins (R2). The owner put the
  built-in MusicBrainz and Cover Art Archive lookups in R1.1, so SEC-PRV-014,
  SEC-PRV-015 and SEC-PRV-017 become due in R1.1 with this package, and
  R1 ships with no provider at all.
- **What it adds to R1's packages.** One egress purpose per provider in
  WP-048's closed enumeration; the provider step in setup's step list
  (WP-080), which becomes a required question with "Turn on" and "Not
  now" for each provider and no answer preselected (SEC-PRV-013); the
  provider entries in the security summary (WP-116); each under the merge
  protocol's interface-change rule.
- **Scope.** Each provider is an egress purpose with exact hosts, off by
  default, listed in the setup step with the exact fields it receives and
  a link to its privacy policy (SEC-PRV-013, SEC-API-079). Requests are
  built only from a typed lookup-evidence value holding normalised
  title, artist, album, year and external IDs, so no path, file name or
  folder name can leave (SEC-PRV-014); lookups run only during scans,
  scheduled refreshes or an explicit action naming the provider, never on
  playback, browsing or search (SEC-PRV-015); the User-Agent names only
  the project (SEC-PRV-017). Responses are decoded into typed structures
  with size limits and normalised, never interpreted as HTML, templates,
  commands or paths (SEC-API-081); URLs in responses are fetched only
  through this provider's purpose, never because a request or a file
  supplied them (SEC-API-080, SEC-HIS-024); provider artwork is
  downloaded once, re-encoded by the worker like any artwork, cached and
  served from the server's own origin under an opaque ID, so clients never
  contact a provider (SEC-PRV-016).
- **Tests.** A core property that sentinel tokens in paths, file names
  and folder names never appear in any request built from the evidence;
  the default configuration enables no provider; with a provider enabled
  and the egress recorder on, playback, browsing and search make no
  request; each response decoder rejects oversized and malformed bodies
  with the typed error, and has a fuzz harness; every image URL in
  responses and sync payloads is server-relative; the User-Agent equals a
  literal string.

#### WP-140 Exclusion patterns (split from WP-024, WP-060 and WP-099)

- **Release** R1.1 · **Wave** not yet scheduled · **Size** S ·
  **Depends on** WP-024, WP-060, WP-099.
- **Owns** `crates/gunmetal-core/src/exclude.rs`, and, under the merge
  protocol's interface-change rule, the exclusion step in
  `crates/gunmetal-fs/src/walk.rs` (WP-060) and the exclusions setting in
  `crates/gunmetal-server/src/libraries/` (WP-099).
- **Serves** LIB-006; API-LIB-02.
- **Security.** Boundaries TB9; threats TM-T22, TM-T09. Verifies
  SEC-STD-011 (the exclusion pattern language: no regular-expression
  engine, a pattern length cap and linear matching).
- **Why it exists.** Exclusion rules (LIB-006) are R1.1 in the adopted R1
  (register D-10), so the pattern language left WP-024 and walking with
  exclusions left WP-060 and WP-099. R1 walks every file beneath a root.
- **Scope.** The exclusion pattern language as WP-024 first specified it:
  a small glob subset (`*`, `**`, `?`, literal names, a case-insensitive
  option) over raw-byte path components, matched in linear time with a
  cap on pattern length and count (SEC-STD-011); applying it in the walk,
  so an excluded directory is never descended; the per-library setting,
  with a preview of what a pattern would exclude.
- **Interface sketch.** `pub struct Exclusions; impl Exclusions { pub fn parse(lines: &str) -> Result<Self, PatternError>; pub fn excludes(&self, p: &RelPath) -> bool; }`;
  `Root::walk` gains an `&Exclusions` argument.
- **Tests.** Patterns `**/*.tmp`, `Extras/`, `?.flac`, a pattern over the
  length cap, and a pathological pattern of many `*` against a long name
  (matched without backtracking), each with its literal verdict; a walk
  over a real directory tree never opens an excluded directory. Property:
  matching agrees with an independent matcher written in the test.

#### WP-141 Ratings, dismissals and the Hidden page (split from WP-034, WP-059 and WP-086)

- **Release** R1.1 · **Wave** not yet scheduled · **Size** M ·
  **Depends on** WP-034, WP-059, WP-068, WP-086; owner decision 17 (the
  rating scale).
- **Owns** `crates/gunmetal-core/src/userdata/ratings.rs` (the rate,
  dismiss and undo bodies and their merge rules, beside WP-034's files),
  and, under the merge protocol's interface-change rule, the rating and
  dismissal paths in `crates/gunmetal-server/src/listening/` (WP-086)
  and the dismissal filter in `crates/gunmetal-core/src/home/` (WP-059).
- **Serves** MUS-181, DIS-047, DIS-022, DIS-023; API-LOG-05.
- **Security.** Boundaries TB4, TB10; threats TM-T18. Verifies
  SEC-PRV-022 and SEC-HIS-060 for the new events (one person's ratings
  and dismissals never reach another person).
- **Why it exists.** Star ratings, dismissing items from Continue rows and
  the Hidden page are R1.1 (register D-10). WP-034, WP-086 and WP-059 had
  them in their R1 scope; the event envelope keeps unknown bodies, so
  they can arrive later without a format change.
- **Scope.** The rate, dismiss and undo event bodies; latest clock wins for
  ratings, with a tie broken by device ID; ratings and dismissals and
  their reversals through the one listening write path, idempotent by
  event ID; dismissed items left out of the Continue rows until undone;
  the Hidden page listing what was dismissed.
- **Tests.** Ratings from two devices resolve to the later clock and the
  tie to the device ID; a dismissed item stays out of continue listening
  until undone (moved from WP-059's tests); an undo restores it; another
  person never receives the ratings or dismissals of the first, through
  any route or the sync feed; the merge stays commutative, associative and
  idempotent with the new bodies (property).

#### WP-142 Shuffle by album, reshuffle and reorder while shuffled (split from WP-025, WP-026 and WP-085)

- **Release** R1.1 · **Wave** not yet scheduled · **Size** S ·
  **Depends on** WP-025, WP-026, WP-085.
- **Owns**, under the merge protocol's interface-change rule,
  `crates/gunmetal-core/src/shuffle.rs` (WP-026's) and the move-while-
  shuffled operation in `crates/gunmetal-core/src/queue/` (WP-025's).
- **Serves** MUS-120, MUS-127, MUS-128.
- **Security.** Boundaries TB4; threats TM-T09. Verifies no requirement of
  its own: it holds no security control, and the rules it relies on are
  proved by the packages that own them.
- **Why it exists.** These shuffle modes and reordering while shuffled
  are R1.1 (register D-10); WP-026 and WP-025 keep random and spread-out
  shuffle in R1.
- **Scope.** Shuffle by album (random albums, each in order); reshuffle
  the rest, which re-seeds from the current item; moving an item while
  shuffled, so the shuffled order and the source order both keep it.
- **Tests.** By album keeps each album's tracks in order and contiguous
  (property); reshuffling the rest never moves the current item or
  anything before it; an item moved while shuffled is where the person
  put it after shuffle is turned off and on again, checked against an
  independent model written in the test.

#### WP-143 Playlist extras (split from WP-093)

- **Release** R1.1 · **Wave** not yet scheduled · **Size** M ·
  **Depends on** WP-093, WP-103.
- **Owns**, under the merge protocol's interface-change rule,
  `crates/gunmetal-server/src/playlists/` (WP-093's), and
  `crates/gunmetal-server/src/playlists/covers.rs`.
- **Serves** MUS-125, MUS-134, MUS-135, MUS-137, MUS-139; API-PL-02,
  API-QUE-05.
- **Security.** Boundaries TB4; threats TM-T12, TM-T09. Verifies
  SEC-API-012 for saving the queue (a list with one invisible item is
  rejected whole).
- **Why it exists.** The duplicate warning, sorting and searching inside a
  playlist, automatic covers, pinning and loving playlists and saving the
  queue as a playlist are R1.1 (register D-10).
- **Scope.** The duplicate warning when an item is already in the
  playlist; pin and love for playlists; an automatic cover built from the
  first distinct album covers through WP-103's derivatives, so no new
  image is decoded; creating a playlist from a list of items, which is how
  the client saves its queue, authorising every item and rejecting the
  whole request if one is not visible (SEC-API-012). Sorting and searching
  inside a playlist (MUS-135) is client work over the synced playlist.
- **Tests.** Adding a track already present returns the warning and adds
  nothing unless confirmed; a saved queue with one item the person cannot
  see is rejected whole; a cover is rebuilt when the first albums change
  and never references a library the viewer cannot see.

#### WP-144 Profile pictures (split from WP-087 and WP-103)

- **Release** R1.1 · **Wave** not yet scheduled · **Size** M ·
  **Depends on** WP-078, WP-079, WP-087, WP-103, WP-130.
- **Owns** `crates/gunmetal-server/src/avatars/`.
- **Serves** ACC-011 (pictures); API-USR-01 (pictures).
- **Security.** Boundaries TB4, TB6; threats TM-T09, TM-T13, TM-T28.
  Verifies SEC-MED-061, SEC-PRV-006 (both due in R1.1 with this surface),
  SEC-API-085, SEC-API-087, SEC-API-088 (for this upload route; in R1 they
  are proved by WP-109, WP-103 and WP-130).
- **Why it exists.** Avatars and other image uploads are R1.1 (register
  D-10), so the upload half of WP-103 moved here, with the requirements
  whose only surface it is (SEC-MED-061, SEC-PRV-006).
- **Scope.** As WP-103 first specified it: the route declares its types,
  byte cap and pixel cap, decides the type from magic bytes and ignores
  the client's filename and `Content-Type` (SEC-API-085); the body is
  size-capped while it streams (SEC-MED-061), re-encoded through the
  worker with EXIF, XMP and IPTC removed (SEC-PRV-006), and stored by a
  server-generated content hash in the data directory, never in a library
  folder and never under a name from the request (SEC-API-087); uploads
  count against the per-principal quota (SEC-API-088).
- **Tests (real SQLite and files).** An SVG upload, an HTML file named
  `.jpg` and a polyglot are refused; an upload with EXIF, XMP and IPTC
  comes back without them, checked at byte level; a format field in the
  upload like the one in Jellyfin CVE-2026-35031 never reaches a path; an
  upload past the quota is refused; a body over the cap is cut off while
  it streams.

#### WP-145 History import (split from WP-108 and WP-080)

- **Release** R1.1 · **Wave** not yet scheduled · **Size** M ·
  **Depends on** WP-057, WP-061, WP-068, WP-070, WP-078, WP-079, WP-080,
  WP-086, WP-108.
- **Owns** `crates/gunmetal-server/src/history_import/`,
  `crates/gunmetal-worker/src/jobs/import_file.rs`, and the "Coming from
  another server?" step in setup's step list (WP-080), added under the
  merge protocol's interface-change rule.
- **Serves** ADM-042, ADM-030, MUS-189, INT-107; API-USR-06, API-SET-11
  (listening-service files).
- **Security.** Boundaries TB4, TB6, TB10; threats TM-T09, TM-T18,
  TM-T21. Verifies SEC-API-085, SEC-MED-018, SEC-MED-020, SEC-MED-023 (for
  this upload and its worker job).
- **Why it exists.** Importing Last.fm and ListenBrainz files and the
  setup step that offers it are R1.1 (register D-10). WP-108 keeps the
  personal export in R1.
- **Scope.** As WP-108 first specified it: upload Last.fm or ListenBrainz
  export files through an upload route that declares its types and byte
  cap and decides the type from content (SEC-API-085); the server stores
  the upload in scratch space and never parses it. The `ImportFile` worker
  job receives it by read-only descriptor (SEC-MED-020), decodes JSON or
  CSV and runs WP-057's parsers there under the step budget, a memory cap
  and the pool's deadline (SEC-MED-018), and returns typed rows that the
  server revalidates before matching (SEC-MED-023). Match them in a job
  with progress (the import-matching task kind, added to WP-070's
  enumeration), write imported plays marked as imported, remove an import
  as a batch. The setup step offers this import and, once WP-112 is in,
  the playlist import; the importer framework for other servers is R2
  (WP-229).
- **Tests (real SQLite and log files).** Importing the same file twice adds
  nothing the second time; removing an import removes exactly its plays;
  unmatched rows land in the unmatched list with reasons; a round trip of
  random activity through WP-108's export and this import gives the same
  activity. Hostile input, with the test-hook feature: an import worker
  that panics on a hostile export, and one that loops past its deadline
  on a deeply nested JSON file, each fail that import with a typed problem
  while the server keeps serving and a second import completes; a fake
  worker returning a row with an out-of-range timestamp or an oversized
  string is refused by the server's revalidation; WP-001's dependency
  check shows `serde_json` decoding of uploads has no call path in the
  server crate.

#### WP-146 Release groups, editions, grouping reasons and artist images (split from WP-040, WP-053, WP-076 and WP-079)

- **Release** R1.1 · **Wave** not yet scheduled · **Size** M ·
  **Depends on** WP-040, WP-053, WP-076, WP-079, WP-084, WP-102.
- **Owns** `crates/gunmetal-core/src/music/release_groups.rs`,
  `crates/gunmetal-core/src/catalog/release_group.rs`, and, under the
  merge protocol's interface-change rule, the release-group tables in the
  catalogue store (WP-067), the grouping result (WP-076) and the artwork
  order step of the scan (WP-102).
- **Serves** MUS-005, MUS-008, MUS-010, MUS-013, MUS-024, MUS-053,
  MUS-055, LIB-098, LIB-187; API-CAT-02 (release groups).
- **Security.** Boundaries TB6, TB9; threats TM-T20, TM-T28. Verifies no
  requirement of its own: artist images go through WP-079's artwork job
  and WP-103's derivatives unchanged, which prove the image rules.
- **Why it exists.** Release groups and editions, release types, original
  versus release dates, roles, the credits panel, one artist page across
  libraries, artist images and "every decision explains itself" are R1.1
  (register D-10). R1 already maps and keeps every tag (WP-049, LIB-059),
  so this package adds grouping and views, not parsing.
- **Scope.** Group albums into release groups by MusicBrainz release-group
  ID, then by album artist, title and original date, erring on the side
  of not merging; release types and original dates on the group; the
  release-group record and its field IDs in the synced library; roles
  (composer, conductor, lyricist, producer, remixer, performer) resolved
  into credits; one artist across libraries for a person who can see
  several; a reason recorded for every grouping decision (LIB-098), which
  WP-076 did not keep in R1; `artist.jpg` attached to its artist in the
  scan's artwork order.
- **Tests.** Two editions of one release group, with and without a
  release-group ID; a remaster with a later release date and the same
  original date; an artist credited as composer on one track and
  performer on another; an artist with tracks in two libraries, seen by a
  person granted both and by one granted one; every grouping decision on
  a small library carries its literal reason; an `artist.jpg` in an
  artist folder becomes that artist's image. Property: release-group
  grouping is independent of input order.

#### WP-147 Browse, search and Home extras (split from WP-036, WP-054 and WP-059)

- **Release** R1.1 · **Wave** not yet scheduled · **Size** M ·
  **Depends on** WP-036, WP-054, WP-059, WP-088.
- **Owns** `crates/gunmetal-core/src/collate_jump.rs`, and, under the
  merge protocol's interface-change rule, the query options in
  `crates/gunmetal-core/src/search/` (WP-054), a row source in
  `crates/gunmetal-core/src/home/` (WP-059) and their functions in the
  WASM facade (WP-088).
- **Serves** DIS-101, CLI-040, DIS-086, DIS-087, DIS-088, DIS-091,
  DIS-071.
- **Security.** Boundaries TB4; threats TM-T09, TM-T15. Verifies
  SEC-STD-011 for the new query options (still no regular-expression
  engine and the same caps).
- **Why it exists.** The alphabet jump, mood and tag search, people by
  role, scoping a search to one library, finding inside a list and your
  top tracks by an artist are R1.1 (register D-10).
- **Scope.** The letter used by the alphabet jump, from WP-036's sort keys;
  the mood field in the search index; filters by role and by library;
  find inside a list over the same folded matching; the "your top tracks
  by an artist" row source, from the person's own plays.
- **Interface sketch.** `pub fn jump_letter(k: &SortKey) -> JumpLetter`;
  `Index::query` gains role and library filters.
- **Tests.** The jump letter for "Björk", "The The" and a Japanese title;
  a mood query finds the tag and not a title containing the word; a role
  filter returns only that role; a library scope never returns an item
  from another library; top tracks by an artist for a person with no
  plays is the designed empty state; each example with its literal
  expected result.

#### WP-148 Installable web app and service worker (split from WP-072)

- **Release** R1.1 · **Wave** not yet scheduled · **Size** S ·
  **Depends on** WP-072.
- **Owns** `crates/gunmetal-server/src/webapp/service_worker.rs`, and the
  web app manifest in WP-072's build-time manifest, under the merge
  protocol's interface-change rule.
- **Serves** CLI-003, and the server side of CLI-024 to CLI-026 (offline
  loading).
- **Security.** Boundaries TB2, TB4, TB5; threats TM-T07, TM-T08.
  Verifies SEC-CLI-011 (the service-worker clause: it never serves a
  bundle older than the running server's), SEC-API-044 for the manifest
  and worker script.
- **Why it exists.** Installing the web app and loading it offline are
  R1.1 (register D-10). R1's web client registers no service worker
  (WP-072).
- **Scope.** As WP-072 first specified it: the service-worker scope
  rules, under which a service worker never caches a capability URL and
  never serves a bundle older than the server's; the web app manifest;
  the worker script served under the same Content Security Policy.
- **Tests.** The worker script's cache list contains no capability-URL
  pattern; a stand-in bundle upgraded under an open tab is replaced, not
  served from the worker's cache (the browser half is the client plan's);
  the manifest and worker are served with the exact CSP.

#### WP-149 Tag problems and missing files in library health (split from WP-110)

- **Release** R1.1 · **Wave** not yet scheduled · **Size** S ·
  **Depends on** WP-077, WP-110.
- **Owns**, under the merge protocol's interface-change rule, two entry
  kinds in `crates/gunmetal-server/src/health/` (WP-110's).
- **Serves** LIB-034, LIB-194.
- **Security.** Boundaries TB9; threats TM-T56. Verifies no requirement
  of its own: the per-role response rules it relies on are proved by
  WP-044 and WP-131.
- **Why it exists.** The missing-files list and tag problems are R1.1
  (register D-10); WP-110 keeps the R1 health report.
- **Scope.** As WP-110 first specified them: tag problems with suggested
  fixes (tracks missing an album artist, albums with inconsistent tags,
  ambiguous artist splits, guessed compilations, same-name collisions)
  and missing and moved files, with when and where.
- **Tests.** A synthetic library with one of each new problem yields
  exactly one entry of each kind; a file moved by a rename appears as
  moved, not missing.

#### WP-150 Moving the server: root relocation and restore onto new hardware (split from WP-099 and WP-109)

- **Release** R1.1 · **Wave** not yet scheduled · **Size** S ·
  **Depends on** WP-060, WP-077, WP-099, WP-109.
- **Owns**, under the merge protocol's interface-change rule, the
  relocation routes in `crates/gunmetal-server/src/libraries/` (WP-099's)
  and the root-remapping step in `crates/gunmetal-server/src/restore/`
  (WP-109's).
- **Serves** LIB-031, ADM-051; API-LIB-05.
- **Security.** Boundaries TB9, TB11; threats TM-T56, TM-T60. Verifies
  SEC-MED-037 and SEC-TM-069 for relocation (a new location obeys the
  root refusal rules, and nothing is removed while a root is being
  moved), with relocation a fresh-uv action (SEC-IAM-041).
- **Why it exists.** Moving the server and keeping the library are R1.1
  (register D-10). In R1 a restore keeps each root's recorded path, and a
  missing root shows as offline (WP-109).
- **Scope.** As WP-099 and WP-109 first specified them: changing a root's
  location with a preview, and remapping library roots in a dry run
  during a restore onto a new machine, operating system or container.
- **Tests (real SQLite and filesystem).** Moving a root to a copy matches
  every file in the preview by its fingerprint and then by content
  identity, so identities and history are kept; a new location inside the
  data directory is refused; a restore onto a machine with different
  paths remaps each root in a dry run before anything is written.

### R1.2, the household and the admin

OIDC, music share links, a second administrator, the admin's live view
with each person's opt-in for titles, stopping a stream, an arrangeable
Home, diagnostics, restore from the UI and translations (register D-10,
"R1.2, the household and the admin"). Requirements due here with their
surfaces: SEC-TM-022, SEC-IAM-026 to SEC-IAM-036, SEC-STD-025 and
SEC-CLI-026 (OIDC, WP-096); SEC-API-097 and SEC-STD-008 (share links,
WP-134); SEC-PRV-046 and SEC-OPS-030 (diagnostic bundles, WP-155).
Translations (CLI-146) are client work and need no package here.

#### WP-096 Single sign-on

- **Release** R1.2 · **Wave** not yet scheduled (was 3) · **Size** L ·
  **Depends on** WP-046, WP-047, WP-048, WP-062, WP-118.
- **Owns** `crates/gunmetal-server/src/oidc/`.
- **Serves** ACC-057; API-AUTH-06, API-SET-03. (ACC-003, sign-in with no
  internet, is R1 and served by passkeys, WP-081.)
- **Why R1.2.** The owner adopted the smaller R1 (register D-10), which
  puts OIDC in R1.2 with the requirements whose only surface it is
  (SEC-TM-022, SEC-IAM-026 to SEC-IAM-036, SEC-STD-025, SEC-CLI-026); they
  are due in R1.2 and that release cannot ship without them. R1's own
  rules that also mention OIDC (fresh verification, WP-062; the step-up
  route, WP-106) are already in place, so this package adds to R1's
  packages only the OIDC egress purpose (WP-048), the `Oidc` pathway of
  the credential verifier (WP-064), OIDC links on the sign-in methods
  page (WP-106) and on invitation redemption (WP-094), and OAuth codes
  and states in the log canary (WP-117), each under the merge protocol's
  interface-change rule.
- **Security.** Boundaries TB4, TB8; threats TM-T07, TM-T30, TM-T32.
  Verifies SEC-IAM-026, SEC-IAM-027, SEC-IAM-028, SEC-IAM-029, SEC-IAM-030,
  SEC-IAM-031, SEC-IAM-032, SEC-IAM-033, SEC-IAM-034, SEC-IAM-035,
  SEC-IAM-036, SEC-IAM-107, SEC-TM-022, SEC-STD-025, SEC-CLI-026,
  SEC-HIS-032, SEC-API-070, SEC-API-080, SEC-API-081, SEC-OPS-017.
- **Scope.** Authorization-code flow with PKCE (S256), state and nonce
  against the household's own provider, with the server as a confidential
  client so no OAuth token reaches browser JavaScript, and implicit and
  hybrid responses refused (SEC-IAM-026, SEC-CLI-026, SEC-TM-022);
  discovery and keys fetched through the egress gate's OIDC purpose with
  TLS verified and no redirect to another host (SEC-IAM-032); ID tokens
  verified with keys from the provider's key set using algorithms pinned
  per provider (never `none`, never a public key as an HMAC secret), with
  `iss`, `aud`, `azp`, `exp`, `iat` and `nonce` checked (SEC-IAM-027);
  each authorization request bound to the one provider it was sent to,
  checking `iss` where RFC 9207 is advertised (SEC-IAM-034); requests
  carrying exactly the configured scopes (SEC-STD-025); one exact
  redirect URI on the configured origin and a post-sign-in return target
  validated by the core (SEC-IAM-033, SEC-API-070, SEC-HIS-032);
  identities keyed only by issuer and subject, never by email or name
  (SEC-IAM-028); linking to an existing account only inside that
  account's session after user verification in the last 5 minutes,
  through WP-062's fresh-uv check, or by redeeming an invitation
  (SEC-IAM-029); auto-registration off by
  default, and when on, new accounts get no grants until an admin
  approves (SEC-IAM-030); provider claims never confer the owner role,
  and mapping a claim to administrator is off by default (SEC-IAM-031);
  sessions from OIDC get Gunmetal's lifetimes and end when the link is
  removed or the account disabled (SEC-IAM-035); admin elevation for an
  account with no passkey needs a fresh provider sign-in under 5 minutes
  old and never satisfies a fresh-uv action (SEC-IAM-036, SEC-IAM-107);
  no URL from a claim, such as a picture, is ever fetched (SEC-API-080);
  provider responses decoded into typed structures with size limits
  (SEC-API-081); provider configuration with a test button; the client
  secret in the vault, decrypted only for the token request and handed to
  the egress client as a `Secret` header value (SEC-OPS-017).
- **Tests.** Against a provider simulated in the test (its own key pair and
  discovery document served locally): a valid login; `alg: none`; an
  HMAC-signed token using the public key as the secret; a wrong audience;
  an expired token; a replayed nonce; a redirect to an unlisted URL; a
  hostile provider returning a victim's email under a new subject (no
  link); a mix-up between two configured providers; a provider with an
  untrusted certificate (fails closed); a cross-host redirect from the
  token endpoint (refused); a claim mapped to owner (ignored); elevation
  through a provider that ignores `max_age` (refused); a picture URL in
  the claims (never fetched, checked through the egress record). The
  simulated provider signs with each algorithm R1.2 accepts, which means
  RS256 as well unless the owner limits R1.2 to ES256 and EdDSA
  providers.
- **Risks.** Hand-written versus the `openidconnect` crate is owner
  decision 10, and so is RSA verification, which the proposed crate list
  lacks (see "Missing from this table"). The client secret no longer has
  an open question: WP-047 decrypts it only for the call and the egress
  client is the one other place allowed to expose it, to write the
  header.

#### WP-134 Music share links (added for the security baseline)

- **Release** R1.2 · **Wave** not yet scheduled (was 4) · **Size** M ·
  **Depends on** WP-031, WP-047, WP-064, WP-065, WP-069, WP-082, WP-097,
  WP-118.
- **Owns** `crates/gunmetal-server/src/shares/`.
- **Serves** ACC-086, ACC-087, ACC-088, ACC-089 (moved from R2 for
  music); MUS-151.
- **Security.** Boundaries TB1, TB4; threats TM-T04, TM-T16, TM-T67.
  Verifies SEC-API-097, SEC-PRV-031, SEC-MED-051, SEC-HIS-042, SEC-STD-008,
  SEC-STD-024, SEC-STD-029, SEC-TM-028, SEC-HIS-066 (the replay for the
  share-token incident of SEC-HIS-042, a file this package adds to
  `tests/rivals/`).
- **Why it exists.** The baseline put music share links in R1 (the
  release scope table; SEC-API-097; baseline owner decision 7), where the
  feature map had them in R2; the owner's adopted R1 puts them in R1.2
  (register D-10), with SEC-API-097 and SEC-STD-008 due in R1.2. The
  scope is the recommendation's: music only, listen-only by default.
  Video links stay in R2, off by default (WP-223). It adds to R1's
  packages the `SharePassword` pathway (WP-064), the share landing page
  on the project site (WP-139) and share secrets in the log canary
  (WP-117).
- **Scope.** A person shares one track, album or playlist. The link
  carries a secret of at least 128 bits in the URL fragment, which the
  landing page sends in a request body; the link is scoped to one object
  and its rights (listen-only by default, downloads only when the owner
  allows them server-wide), takes its owner from the session, never from
  the request, and expires after 30 days by default (SEC-API-097). An
  optional password accepts any Unicode with no composition rules and at
  least 64 characters (SEC-STD-008), is stored with the Argon2id helper
  (SEC-STD-024), and is a pathway of the credential verifier with the
  guessable-secret delays. Per-link limits: 2 concurrent streams by
  default, a total-bytes or uses cap consumed by one conditional update
  (SEC-STD-029), and a distinct-address count that suspends the link and
  alerts the sharer when exceeded. Streams under a share use capability
  URLs bound to the share and re-checked on every request, so deleting
  the share stops playback on the next range request (SEC-HIS-042,
  SEC-TM-028); what the link exposes is evaluated with the sharer's
  current rights at request time (SEC-MED-051). The share page reveals
  no username, other users, library size or activity, sends
  `X-Robots-Tag: noindex`, and sends no link-preview metadata unless the
  sharer turns it on (SEC-PRV-031). Links are visible and editable only by
  their owner and admins.
- **Tests (real SQLite).** The cross-principal and revocation suites as for
  SEC-API-011 and SEC-API-028: another person cannot list, edit or delete
  the share; a deleted or expired share fails on the next range request
  over a real socket; the third concurrent stream is refused; the
  distinct-address limit suspends the link and raises exactly one alert;
  a wrong password runs into the delays; preview tags are absent by
  default; the anonymous share page, compared whole, names no user; 64
  concurrent uses of a one-use link give one success.
- **Risks and decisions.** The owner answered the baseline's decision 7
  through D-10: R1.2. The share-link password is the first human-chosen
  secret, so this is the first caller of WP-047's Argon2id helper.

#### WP-151 Path prefix behind a reverse proxy (split from WP-073)

- **Release** R1.2 · **Wave** not yet scheduled · **Size** S ·
  **Depends on** WP-023, WP-073, WP-118.
- **Owns**, under the merge protocol's interface-change rule, the prefix
  setting in `crates/gunmetal-server/src/network/` (WP-073's).
- **Serves** ACC-134; API-SET-01 (prefix).
- **Security.** Boundaries TB1, TB2; threats TM-T05, TM-T11. Verifies
  SEC-NET-015 and SEC-API-069 for prefixed URLs (every absolute URL is
  built from the canonical origin and prefix, never from request
  headers).
- **Why it exists.** Serving under a path prefix behind a reverse proxy is
  R1.2 (register D-10). R1 serves at the root of its origin.
- **Scope.** As WP-073 first specified it: a path prefix applied to every
  route and cookie path, and to the canonical origin every emitted URL is
  built from; the shipped proxy configurations gain a prefixed example.
- **Tests.** The prefix applies to redirects and the cookie path; a forged
  `Host` and forged forwarding headers never change an emitted URL under
  a prefix; the proxy CI job runs the prefixed example.

#### WP-152 Several administrators (split from WP-094)

- **Release** R1.2 · **Wave** not yet scheduled · **Size** S ·
  **Depends on** WP-033, WP-062, WP-094, WP-097.
- **Owns**, under the merge protocol's interface-change rule, the role
  routes in `crates/gunmetal-server/src/users/` (WP-094's).
- **Serves** ACC-040.
- **Security.** Boundaries TB4, TB11; threats TM-T13, TM-T14. Verifies,
  for the role change: SEC-IAM-073, SEC-IAM-074, SEC-IAM-075 (the
  owner-only capabilities are never granted), SEC-IAM-098 and SEC-OPS-034
  (the new-admin notice and alert), SEC-TM-017 (making an administrator
  is a fresh-uv action).
- **Why it exists.** A second administrator is R1.2 (register D-10). In
  R1 the owner is the only administrator; the policy (WP-033) already
  treats roles as capability presets.
- **Scope.** Make an administrator and remove one, as an owner, fresh-uv
  action, notifying every admin and the person, and raising the alert
  that can never be switched off.
- **Tests.** Making an administrator without fresh verification is
  refused; an administrator can never be given an owner-only capability
  (property over grant sequences); the new-admin alert reaches the owner
  even after a flood of failed sign-ins.

#### WP-153 Admin live view and stopping a session (split from WP-104)

- **Release** R1.2 · **Wave** not yet scheduled · **Size** M ·
  **Depends on** WP-055, WP-083, WP-087, WP-104.
- **Owns** `crates/gunmetal-server/src/nowplaying/`.
- **Serves** ADM-099, ADM-100, ADM-102, ACC-072, ACC-073, ACC-116,
  MUS-235, INT-134, MUS-190; API-SES-02.
- **Security.** Boundaries TB4, TB11; threats TM-T17, TM-T18. Verifies
  SEC-PRV-025 (the live view's part), SEC-TM-054, SEC-IAM-077,
  SEC-HIS-014.
- **Why it exists.** The admin's live view, each person's opt-in for
  titles and stopping a stream are R1.2 (register D-10). The owner's
  answer on what admins see applies: who is playing and totals, not what,
  unless each person opts in, and no history.
- **Scope.** As WP-104 first specified it: the admin now-playing view,
  which shows user, device, bitrate and playback method and the decision
  reason, but not the title unless that person opted in to showing
  titles, offers no view of anyone's history, and is rate-limited, with
  each admin read recorded in the subject's own security log
  (SEC-PRV-025, SEC-TM-054, SEC-IAM-077); the per-person opt-in setting;
  stopping a session with a plain-text message, which cuts in-flight
  responses and pushes "stopped by the owner" to the device. Only an
  admin may stop another person's session (SEC-HIS-014).
- **Tests (real SQLite).** Stopping a session ends its byte stream over a
  real socket and the device's event stream receives the message as text
  (markup in the message is not interpreted); the admin live view and its
  events carry no title for an adult who has not opted in, and do for one
  who has; a member cannot see or stop another person's session; each
  admin read appears in the subject's own log.

#### WP-154 Arrangeable Home: layouts, pins and row settings (split from WP-059, WP-087 and WP-099)

- **Release** R1.2 · **Wave** not yet scheduled · **Size** M ·
  **Depends on** WP-059, WP-087, WP-099.
- **Owns** `crates/gunmetal-server/src/home_layout/`, and, under the merge
  protocol's interface-change rule, layout evaluation in
  `crates/gunmetal-core/src/home/` (WP-059's) and the keep-off-Home switch
  in `crates/gunmetal-server/src/libraries/` (WP-099's).
- **Serves** MUS-049, DIS-003, DIS-007, DIS-009, DIS-012, DIS-013,
  DIS-015; API-HOME-01, API-HOME-02.
- **Security.** Boundaries TB4; threats TM-T15. Verifies no requirement of
  its own: visibility on every row is WP-065's and is proved there and by
  WP-131.
- **Why it exists.** An arrangeable Home is R1.2 (register D-10). R1 shows
  the default Home (WP-059).
- **Scope.** Home layouts and pins as versioned documents in the person's
  settings, which follow them across devices (moved from WP-087); rows as
  long as the person likes; a home that does not move while it is shown;
  keeping a library off Home (moved from WP-099). A saved filter (DIS-105,
  R1.1) can become a row, evaluated on the device like the filter view it
  came from (DIS-003), in the rule format WP-027 delivers in R1.1. Rows
  built in the rule editor wait for the editor and the rule store in R1.3
  (WP-092).
- **Tests.** A layout saved on one device is the layout on another; a
  library marked "keep off Home" never appears in any row (moved from
  WP-059's tests); a pinned item stays first; reordering rows never drops
  one.

#### WP-155 Admin tasks, health summary, diagnostics and the emergency page (split from WP-095, WP-097, WP-100, WP-110 and WP-116)

- **Release** R1.2 · **Wave** not yet scheduled · **Size** L ·
  **Depends on** WP-070, WP-074, WP-090, WP-095, WP-097, WP-100, WP-106,
  WP-110, WP-116, WP-132.
- **Owns** `crates/gunmetal-server/src/admin_tools/`, and, under the merge
  protocol's interface-change rule, the run and cancel routes beside
  WP-100's job routes, the restart and shut-down routes beside WP-095's
  startup module and the crash records beside WP-097's operations module.
- **Serves** ADM-093, ADM-109, ADM-112, ADM-113, ADM-124, ADM-130,
  CLI-033, ADM-080 (the write-queue view); API-SYS-08, API-SET-08,
  API-SET-10, API-DEV-03, API-HLTH-05.
- **Security.** Boundaries TB10, TB11; threats TM-T53, TM-T57. Verifies
  SEC-OPS-030 and SEC-PRV-046 (both due in R1.2 with this surface),
  SEC-OPS-050 (the emergency page), SEC-OPS-059 (diagnostics endpoints
  off by default), SEC-API-064 and SEC-HIS-013 for the task routes.
- **Why it exists.** The task list with run, cancel and history, the
  health summary on the admin home, restart and shut down from the UI,
  diagnostics, the emergency page and local crash records are R1.2
  (register D-10).
- **Scope.** As the source packages first specified them: the task list
  routes (run, cancel, progress, last run, duration, errors) over WP-070's
  engine; the admin home's roll-up of library health, backups and
  verification, advisories and recovery use (from WP-110); restart and
  shut down (from WP-095); local crash records (from WP-097). From
  WP-116: a diagnostic bundle built from an allowlist, with no database,
  backups or secrets, file paths, titles, user names and addresses
  replaced by per-bundle pseudonyms, and shown in full before download
  (SEC-OPS-030, SEC-PRV-046); a minimal server-rendered emergency page
  with status, recent log lines, a backup and a restart, revealing no
  version, path or stack trace to an unauthenticated client
  (SEC-OPS-050); metrics and diagnostics endpoints off by default and,
  when on, exposing no per-person data, with the scoped-token form
  waiting for R2's keys (SEC-OPS-059); accepting a diagnostics report a
  person built and reviewed on their device; the write-queue view.
- **Tests.** The bundle contains no secret, title, user name or address
  (canary over a bundle generated after the full integration suite) and
  no absolute media paths; the emergency page works with the client
  bundle missing and reveals nothing in any server state; a member
  cannot run or cancel a task; restart without fresh verification is
  refused; the roll-up shows exactly one card per problem class from a
  fixture state.

#### WP-156 File inspector (split from WP-107)

- **Release** R1.2 · **Wave** not yet scheduled · **Size** S ·
  **Depends on** WP-065, WP-067, WP-078, WP-079.
- **Owns** `crates/gunmetal-server/src/inspector/`.
- **Serves** ADM-125, LIB-195; API-CAT-11.
- **Security.** Boundaries TB9, TB11; threats TM-T13. Verifies
  SEC-HIS-013, SEC-API-068.
- **Why it exists.** The file inspector is R1.2 (register D-10), while the
  rest of WP-107, curation, is R1.3.
- **Scope.** As WP-107 first specified it: the file inspector for admins
  (every raw tag, the structure the parsers read, the identification
  decision and "why is this here", errors with offsets), probing the file
  through the worker on demand, with an admin-only response type.
- **Tests.** The inspector shows the exact problem offsets the probe
  reported; a member gets 404; the response type for an admin holds no
  field another role's type lacks without the inspect capability.
- **Not in scope.** A parse summary for members (MUS-114; see Review
  notes).

#### WP-157 Restore from the UI with a restore point and a preview (split from WP-109)

- **Release** R1.2 · **Wave** not yet scheduled · **Size** M ·
  **Depends on** WP-062, WP-095, WP-109.
- **Owns**, under the merge protocol's interface-change rule, the UI
  restore routes in `crates/gunmetal-server/src/restore/` (WP-109's).
- **Serves** ADM-070; API-SET-04 (restore from the UI).
- **Security.** Boundaries TB10, TB11; threats TM-T59, TM-T60. Verifies
  SEC-OPS-043, SEC-OPS-044, SEC-TM-017 (an owner, fresh-uv action) for
  this path.
- **Why it exists.** Restoring from the UI with a restore point and a
  preview is R1.2 (register D-10); R1 restores from the command line and
  the welcome screen (WP-109).
- **Scope.** As WP-109 first specified it: restore from the UI of a
  running server, as an owner, fresh-uv action, after taking a restore
  point, with a preview of what the backup holds; everything else
  (verification, extraction, the jailed open, key rotation and the review
  alert) is WP-109's path, reused unchanged.
- **Tests.** A restore without fresh verification is refused; a failed
  restore leaves the server at its restore point; the preview names the
  server, the date and the accounts the backup holds, compared whole.

#### WP-158 The owner's server export (split from WP-108)

- **Release** R1.2 · **Wave** not yet scheduled · **Size** M ·
  **Depends on** WP-062, WP-090, WP-108.
- **Owns** `crates/gunmetal-server/src/server_export/`.
- **Serves** ADM-074; API-SET-04 (server export).
- **Security.** Boundaries TB10, TB11; threats TM-T18, TM-T59. Verifies
  SEC-PRV-025 (the export part), SEC-API-071, SEC-TM-017.
- **Why it exists.** A full export in documented formats is R1.2
  (register D-10); WP-108 keeps each person's own export in R1, as it
  foresaw splitting.
- **Scope.** As WP-108 first specified it: the owner's server export in
  the same documented formats, as a fresh-uv action: settings without
  secrets, library roots, the household data and the owner's own data,
  never another adult's history, ratings or private playlists
  (SEC-PRV-025); the curation log joins it in R1.3 with WP-107. Moving the
  whole server uses the encrypted backup (WP-090).
- **Tests.** The owner's server export of a two-person server, compared
  with a literal expected document, contains no history event, rating or
  private playlist of the other adult, and no secret (canary); an export
  without fresh verification is refused.

#### WP-159 Household setup steps and deep links (split from WP-080 and WP-089)

- **Release** R1.2 · **Wave** not yet scheduled · **Size** S ·
  **Depends on** WP-080, WP-089, WP-094, WP-239.
- **Owns**, under the merge protocol's interface-change rule, three steps
  in setup's step list (WP-080's), the server name and welcome message in
  `crates/gunmetal-server/src/meta/` (WP-089's), and the deep-link routes
  in `crates/gunmetal-core/src/deeplink.rs` (WP-239's).
- **Serves** ADM-027, ADM-140, CLI-034, INT-147.
- **Security.** Boundaries TB1, TB4; threats TM-T07, TM-T10. Verifies
  SEC-API-005 and SEC-NET-047 (the server name is never shown to an
  unauthenticated caller), SEC-PRV-053 (the welcome message in the
  invitee's privacy notice), SEC-CLI-025 (each new deep link through the
  one parser), SEC-API-046 (the message is stored and shown as text).
- **Why it exists.** Language, region and time zone, a custom server name
  and welcome message, and deep links are R1.2 (register D-10).
- **Scope.** The locale and time-zone step and setting; the server name
  and the welcome message invited people see before they join and
  everyone sees after signing in; deep links to items and pages added to
  the parser's closed route set, never granting access by themselves.
- **Tests.** Unauthenticated `GET /api/v1/server` still matches its
  literal schema with a server name set; a welcome message with markup is
  returned as text; each new deep link parses to its typed route, and
  unrecognised input still maps to "not recognised" (property).

### R1.3, discovery and analysis

The rule editor and smart playlists, library radio and the neighbour
table, measured loudness (if ADR 5 is accepted), folder view for admins,
manual curation and 32-bit ARM (register D-10, "R1.3, discovery and
analysis"). The core rule format they build on, with its parser budgets,
is R1.1 work (WP-027, register D-85). No security requirement moves to
R1.3. Three R1.3 rows need no
package of their own: folder view (LIB-008, DIS-106) reads the folder
paths WP-102 already stores (API-CAT-10) through a route not yet planned;
showing the bytes each scan read (ADM-088) reads the counts WP-102 writes
into its activity entry (WP-100); and the derived-data store kept across
rebuilds (ADM-141) is WP-071's, which arrives in R1.1.

#### WP-029 Loudness meter (conditional on ADR 5)

- **Release** R1.3 · **Wave** not yet scheduled (was 1) · **Size** M ·
  **Depends on** WP-003 (accepted), WP-005.
- **Owns** `crates/gunmetal-core/src/loudness.rs`.
- **Serves** MUS-086; LIB-024.
- **Security.** Boundaries TB6; threats TM-T20. Verifies SEC-MED-001.
- **Scope.** ITU-R BS.1770 integrated loudness with K-weighting and gating
  over blocks of samples fed incrementally, and true peak by oversampling,
  with a checkpointable state so a long analysis can resume.
- **Not in scope.** Decoding (WP-114 in the worker).
- **Interface sketch.** `pub struct Meter; impl Meter { pub fn new(rate: SampleRate, channels: Channels) -> Self; pub fn push(&mut self, frames: &[f32]); pub fn checkpoint(&self) -> MeterState; pub fn finish(self) -> Loudness; }`.
- **Tests.** Synthetic sine waves generated in the test at known amplitudes
  and frequencies, with expected values taken from the standard's
  published conformance description (for example a 997 Hz sine at a given
  level on one channel); silence (gated out entirely, result "too quiet");
  a checkpoint and resume giving the same result as one pass; a true-peak
  case where the sample peak and true peak differ. The expected numbers
  must come from the standard or EBU Tech 3341, not from this code.
- **Risks.** Floating-point results across platforms must be compared with
  a stated tolerance; WASM is not a target for this module.

#### WP-058 Neighbour table and radio

- **Release** R1.3 · **Wave** not yet scheduled (was 2) · **Size** M ·
  **Depends on** WP-005, WP-026 (the `RecentPlays` type), WP-040.
- **Owns** `crates/gunmetal-core/src/radio/`.
- **Serves** DIS-060 to DIS-062, DIS-067, DIS-070, MUS-165, MUS-129;
  API-SYNC-06. It also adds, to R1's packages, the start-radio verb and
  the suggestions that fill the Continue with lane (WP-025), the
  "because you played" row (WP-059) and radio picks in the WASM facade
  (WP-088), each under the merge protocol's interface-change rule.
- **Security.** Boundaries TB4, TB10; threats TM-T18. Verifies no
  requirement of its own: it holds no security control, and the rules it
  relies on are proved by the packages that own them.
- **Scope.** Compute each track's, album's and artist's top N neighbours
  from shared credits, genres and era within a size budget (server side,
  as a job in WP-113); the person's own listening weights it on the
  device. Household co-listening is not built in R1 or R2: one person's
  plays must not shape another's table (SEC-PRV-022, ACC-114; security
  README decision 5, register D-37), so the first draft's co-listening
  input is removed. On the device, pick
  radio tracks from a seed with seeded randomness, avoiding recent plays,
  with a reason label for every pick.
- **Interface sketch.** `pub fn neighbours(lib: &LibraryView, budget: Budget) -> NeighbourTable`;
  `pub fn radio(seed: Seed, table: &NeighbourTable, recent: &RecentPlays, rng_seed: u64, n: u16) -> Vec<Pick>`.
- **Tests.** A tiny library where neighbours are obvious and written out by
  hand; cold start with no plays; a seed with no neighbours (falls back to
  genre, labelled); the size budget enforced. Properties: when the table
  holds at least `n` eligible candidates, exactly `n` picks come back; no
  pick repeats within the window; every pick carries a reason. (Without the
  first, an empty result would satisfy the other two.)

#### WP-092 Rule store and server-side evaluation

- **Release** R1.3 · **Wave** not yet scheduled (was 3) · **Size** M ·
  **Depends on** WP-027, WP-065, WP-066, WP-067, WP-068, WP-118, WP-119.
- **Owns** `crates/gunmetal-server/src/rules/`.
- **Serves** DIS-105 (saved filters are rule documents from R1.1, WP-027;
  from R1.3 this store also holds them beside smart playlists),
  DIS-119 to DIS-122, MUS-143 to MUS-146; API-PL-05, API-PL-06
  (the R1.3 evaluation; tools reading the results are R2). (MUS-149 is R1 without rules, WP-059
  and WP-086; tools reading smart playlists through the API, INT-138, are
  R2 with WP-091.) It registers its projection rebuilder with WP-095 and
  adds saved rules to the personal export (WP-108).
- **Security.** Boundaries TB4; threats TM-T09, TM-T15. Verifies no
  requirement of its own: it holds no security control, and the rules it
  relies on are proved by the packages that own them.
- **Scope.** Save, rename and delete rule trees as versioned documents
  (latest wins); bind the rule language's field IDs to the catalogue
  tables; evaluate smart playlists on the server for tools reading them
  through the API, re-evaluated when the library or the rule changes.
- **Tests (real SQLite).** For a small synthetic library and a set of
  rules, the items the server returns equal literal expected ID lists
  worked out by hand. The server evaluates with the core's `evaluate`, so
  comparing server output with the core's own output over the same data,
  as first planned, would pass even if both were wrong; what can differ
  between the two sides is the field binding, so a property checks that
  for arbitrary rows the server's binding (from cache rows) and the synced
  record's binding (WP-040's field accessors) return the same value for
  every field ID (the same-result promise of DIS-119). An unknown rule
  node from a newer client is kept.

#### WP-107 Curation and the review queue (was: curation and the file inspector)

- **Release** R1.3 · **Wave** not yet scheduled (was 4) · **Size** L (was
  M; it gained the review queue from WP-111, the curation event bodies
  from WP-034 and the overrides in WP-053 and WP-076, and lost the file
  inspector to WP-156 in R1.2) · **Depends on** WP-034, WP-053, WP-065,
  WP-067, WP-068, WP-070, WP-076, WP-077, WP-095, WP-102, WP-111.
- **Owns** `crates/gunmetal-server/src/curation/`,
  `crates/gunmetal-server/src/review/`, the curation bodies in
  `crates/gunmetal-core/src/userdata/` (one file, `curation.rs`, beside
  WP-034's), and the override step in `crates/gunmetal-core/src/music/`
  (`overrides.rs`), which WP-053's credit resolution and WP-076's
  grouping call.
- **Serves** MUS-007, LIB-041, LIB-058, LIB-099, LIB-179; API-CAT-12,
  API-HLTH-03. (The file inspector, ADM-125, LIB-195 and API-CAT-11, is
  R1.2, WP-156; rescanning one item, API-CAT-13, is R1 through WP-098.)
- **Security.** Boundaries TB9, TB11; threats TM-T13. Verifies SEC-HIS-013,
  SEC-API-068.
- **Why R1.3.** The adopted R1 puts manual curation, the review queue and
  "fixes survive everything" in R1.3 (register D-10). R1 groups without
  overrides and never guesses: ambiguous matches become a removal and an
  addition (WP-077) and doubtful groupings stay apart (WP-076). This
  package changes both to send the doubtful case to the review queue.
- **Scope.** The household curation event bodies (merge, split and alias
  of artists and albums, and a review answer) and their merge rules, added
  to the user-event envelope, which keeps unknown bodies and so needed no
  format change (WP-034). Merge, split and alias artists and albums as
  curation-log events that survive rescans and rebuilds, applied on top
  of credit resolution and grouping as overrides. The review queue, moved
  from WP-111: doubtful decisions with their evidence and a proposal;
  accept, reject or choose another, stored in the curation log; WP-076's
  grouping and WP-077's diff emit review items instead of keeping things
  apart. Its projection rebuilder registers with WP-095, and curation
  joins the owner's server export (WP-158).
- **Tests (real SQLite and log files).** A merge is written as one
  curation event and survives a cache rebuild through this package's
  projection rebuilder; an alias makes both names find the artist; an
  override that merges two albums the rules kept apart (moved from
  WP-076's tests); an accepted review survives a rebuild (moved from
  WP-111's tests); end to end over a real server, the artist-merge round
  trip that WP-117 carried: a merge survives a rescan and a rebuild.
- **Not in scope.** The file inspector (WP-156, R1.2).

#### WP-113 Neighbour table and rule re-evaluation jobs

- **Release** R1.3 · **Wave** not yet scheduled (was 5) · **Size** S ·
  **Depends on** WP-058, WP-092, WP-102.
- **Owns** `crates/gunmetal-server/src/derived_jobs/`.
- **Serves** DIS-060, DIS-121; API-SYNC-06, API-PL-06.
- **Security.** Boundaries TB4; threats TM-T09. Verifies no requirement of
  its own: it holds no security control, and the rules it relies on are
  proved by the packages that own them.
- **Scope.** The nightly and after-scan neighbour table rebuild within its
  size budget, synced as a change; re-evaluation of server-side smart
  playlists when the library changes.
- **Tests (real SQLite).** A scan triggers one rebuild, not one per batch;
  the table's size stays inside the budget.

#### WP-114 Loudness analysis job (conditional on ADR 5)

- **Release** R1.3 · **Wave** not yet scheduled (was 5) · **Size** M ·
  **Depends on** WP-003 (ADR 5 accepted), WP-029, WP-071, WP-078, WP-079,
  WP-102.
- **Owns** `crates/gunmetal-worker/src/jobs/loudness.rs`,
  `crates/gunmetal-server/src/loudness/`, and, registered through the
  shared-file table, `crates/gunmetal-fuzz/src/loudness.rs`,
  `crates/gunmetal-fuzz/tests/loudness_corpus.rs`,
  `fuzz/fuzz_targets/loudness.rs` and `fuzz/seeds/loudness/`.
- **Serves** MUS-086, MUS-089, LIB-024, ADM-095.
- **Security.** Boundaries TB6; threats TM-T20, TM-T21. Verifies
  SEC-MED-018, SEC-MED-021, SEC-MED-026.
- **Scope.** At low priority after a scan, throttled and checkpointed,
  decode untagged tracks in the worker and measure them; store results in
  the derived-data store; mark the gain source as measured. The decoder
  crate is admitted only after the recorded review SEC-MED-026 requires
  (`unsafe`, scope, fuzzing, limits, supply chain), behind a Gunmetal
  wrapper that is itself fuzzed. The loudness worker is single-threaded
  and runs under the SEC-MED-021 limits (512 MiB memory, CPU-time, 300 s
  per-file deadline).
- **Tests.** Synthetic tones encoded in the test (FLAC with verbatim
  subframes is simple to write; other formats only if the testkit can
  produce them without an external encoder); a file over 12 hours keeps its
  tags only; a restart resumes from the checkpoint. The SEC-MED-026 review
  record covers `unsafe`, scope, limits and supply chain; a test hook
  reports that the loudness worker is single-threaded and runs under the
  SEC-MED-021 limits, and a hung worker is killed at the 300 s deadline.
  A wrapper fuzz harness in `crates/gunmetal-fuzz` replays its committed
  seeds (SEC-MED-027, SEC-MED-028).

#### WP-160 Builds for small ARM boards, including 32-bit (split from WP-121)

- **Release** R1.3 · **Wave** not yet scheduled · **Size** S ·
  **Depends on** WP-045, WP-110, WP-121, WP-136.
- **Owns**, under the merge protocol's interface-change rule, the ARMv7
  target in the release workflow (WP-136's) and its packaging
  (WP-121's).
- **Serves** ADM-004.
- **Security.** Boundaries TB6, TB12; threats TM-T21, TM-T54. Verifies
  SEC-MED-024 (the 32-bit build is labelled with the reduced isolation
  tier and not claimed as supported until its seccomp answer is
  recorded).
- **Why it exists.** The owner's answer to D-09 makes R1 servers Linux on
  x86-64 and ARM64 plus a Docker image, and the register puts 32-bit ARM
  in R1.3.
- **Scope.** As WP-121 first specified it: a 32-bit ARM build labelled
  with the reduced isolation tier on the health page and not claimed as
  supported until its seccomp answer is recorded (SEC-MED-024; owner
  decision 24), signed and with provenance like every other artefact.
- **Tests.** A smoke run of the 32-bit ARM build under emulation if CI
  can provide one (unverified); the health page on that build shows the
  exact reduced-tier notice.

#### WP-161 People with typed roles, typed links and spoken word (split from WP-040 and WP-099)

- **Release** R1.3 · **Wave** not yet scheduled · **Size** M ·
  **Depends on** WP-040, WP-099, WP-146.
- **Owns** `crates/gunmetal-core/src/catalog/people.rs`,
  `crates/gunmetal-core/src/catalog/links.rs`, and, under the merge
  protocol's interface-change rule, the spoken-word flag in
  `crates/gunmetal-server/src/libraries/` (WP-099's).
- **Serves** LAT-002, LAT-008, LAT-010.
- **Security.** Boundaries TB4, TB9; threats TM-T15. Verifies no
  requirement of its own: it holds no security control, and the rules it
  relies on are proved by the packages that own them.
- **Why it exists.** People with typed roles, typed links between items
  and keeping spoken word out of music are R1.3 (register D-10).
- **Scope.** A person record with typed roles distinct from artist
  credits; typed links between items (for example a live recording and
  its studio original); a library's spoken-word flag, which keeps its
  items out of music browsing, shuffle and radio.
- **Tests.** Constructors refuse invalid combinations; a spoken-word
  library's items never appear in music Home rows or radio picks.

### R2: the name service and its client

The project-run per-server name service, its naming client and its
Certificate Transparency monitoring (owner answer to D-07), beside
built-in remote access (WP-221 in the R2 outline), and the scoped API keys
that moved to R2 for the security baseline (WP-091). Requirements due in
R2 with these surfaces: SEC-NET-010, SEC-NET-011, SEC-NET-012,
SEC-NET-069, SEC-NET-070 and SEC-NET-071 (WP-129, WP-135), and the R2
requirements WP-091 already listed. The four feature rows the register
moves out of the R1 line to R2 (LIB-056, CLI-032, INT-006, INT-138) are
served by WP-091 and the R2 outline. R2's waves and packages are in
[R2 (video) in outline](#r2-video-in-outline).

#### WP-129 Per-server name service (added for the security baseline; R2 by owner decision D-07; the project site's security files split off to WP-139)

- **Release** R2 · **Wave** not yet scheduled (was 1) · **Size** L ·
  **Depends on** WP-005, WP-139 (the zone's HSTS entry joins the site's
  headers); conditional on a legal home for project services (register
  D-41) and a second keyholder (D-62).
- **Owns** `crates/gunmetal-names/` (creates the crate: the label codec,
  the pure DNS-answer function, the registration rules and the service
  binary).
- **Serves** ADM-023 (a per-server HTTPS name from the project name
  service, moved to R2 with this package); ADM-021 and ADM-022 for
  households with no domain, tailnet or reverse proxy of their own.
- **Security.** Boundaries TB1, TB8, TB12; threats TM-T33, TM-T44, TM-T70.
  Verifies, in R2: SEC-NET-011, SEC-NET-012 (the service's CAA record),
  SEC-NET-070, SEC-HIS-061 (the name-service clauses; WP-139 proves the
  site's clause in R1), SEC-STD-016 (the name-service zone; WP-139 proves
  gunmetal.tv's in R1).
- **Why it moved.** The baseline's recommended R1 made HTTPS work for
  ordinary households through a per-server name service run by the
  project. The owner decided otherwise on 2026-10-02 (register D-07): R1
  gets HTTPS through the owner's own domain with automatic certificates
  (WP-101), a tailnet, or the same machine, and the name service, its
  naming client and its Certificate Transparency monitoring move to R2,
  alongside built-in remote access (WP-221). The requirements that
  protect only the name service (SEC-NET-010 to SEC-NET-012, SEC-NET-069
  to SEC-NET-071) move with it and are due in R2; none is weakened. The
  project site's security files are R1 surfaces whatever happens to the
  service, so they split off to WP-139 in wave 1.
- **Scope.** The name service: labels are 128-bit random values the
  server generates; the service answers A and AAAA queries as a pure
  function of the queried name, only for labels that encode an address in
  RFC 1918, RFC 6598 or RFC 4193 space, and answers nothing for any other
  address (SEC-NET-011); it publishes a CAA record per label restricting
  issuance to that server's ACME account and dns-01 (SEC-NET-012); it
  limits registrations per source and per key, requires proof of work or
  a minimum key age, garbage-collects labels that stop renewing, and
  launches only after its zone is on the Public Suffix List
  (SEC-NET-070); it holds only a random label and a public key, with no
  accounts or user data (SEC-HIS-061); its zone sends HSTS with preload
  from the day it is announced (SEC-STD-016).
- **Not in scope.** The server's client for the service (WP-135). The
  project site (WP-139). Deployment and hosting accounts, which are the
  owner's.
- **Tests.** Property tests: the label codec round-trips every local
  IPv4 and IPv6 address it may encode, and the answer function returns no
  address for any public, loopback or link-local value; the registration
  rules refuse a burst from one source and a fresh key; a CAA record for a
  label names exactly that account and `dns-01`. A load test of the
  registration endpoint. The zone's HSTS header in the site checks.
- **Risks and decisions.** An ADR amending ADR 1 decisions 7 and 9 and a
  zone name that is not the docs domain (register D-07), before R2 starts.

#### WP-135 Name-service client and Certificate Transparency monitoring (added for the security baseline; R2 by owner decision D-07)

- **Release** R2 · **Wave** not yet scheduled (was 5) · **Size** M ·
  **Depends on** WP-048, WP-080, WP-097, WP-101, WP-129.
- **Owns** `crates/gunmetal-server/src/naming/`.
- **Serves** ADM-023, and ADM-021 and ADM-022 for a household with no
  domain of its own (R2).
- **Security.** Boundaries TB8, TB12; threats TM-T11, TM-T33. Verifies, in
  R2: SEC-NET-010, SEC-NET-012 (the server's CAA check), SEC-NET-069,
  SEC-NET-071, and the name-service parts of SEC-NET-072 and SEC-OPS-007
  (the label registration as a pre-claim connection, and the claim page's
  note that the name is permanent). In R1, SEC-OPS-007 and SEC-NET-072
  are proved for the paths R1 has by WP-101, WP-109 and WP-117.
- **Why it exists.** With the name service (WP-129, R2), the server needs
  the client side: registering its label, answering DNS-01 challenges
  through the service, and watching for certificates issued for its name
  by anyone else. It moved to R2 with the service (register D-07). It
  adds the naming and CT-monitoring egress purposes to WP-048's
  enumeration.
- **Scope.** At install time, when the owner chose the name service,
  generate a 128-bit random label on the server, not derived from any key
  or address, and register it with the server's public key (SEC-NET-010);
  before the claim, make no outbound connection except this registration
  and ACME DNS-01 through WP-101, and print the name on the console
  (SEC-OPS-007); at each renewal, check the label's CAA record and alert
  if it is missing or wrong (SEC-NET-012); from the claim, monitor
  Certificate Transparency for the label through at least two
  independent monitors over the egress client, raising a critical owner
  alert for a certificate the server did not request (SEC-NET-069); when
  the service refuses, rate-limits or cannot be reached, keep working on
  every other path and explain the failure and the alternatives in plain
  words (SEC-NET-071); the claim page explains that the chosen name is
  permanent for passkeys, and a documented, tested migration moves a
  server to a new origin (SEC-NET-072).
- **Tests.** The registration and TXT-update requests compared literally
  against a local test name service; in an isolated network namespace
  before the claim, the only connections are to the naming and ACME
  purposes; a fake CT feed containing a foreign certificate raises exactly
  one critical alert; a missing CAA record raises the alert; a refusing
  test service leaves localhost and own-domain HTTPS working, with the
  console message compared literally; an end-to-end origin migration.

#### WP-091 Scoped tokens and the tool change feed (moved to R2)

- **Moved to R2 for the security baseline.** The baseline puts API keys,
  app passwords and the adapters in R2, never with administrator scopes
  (release scope table; SEC-EXT-008 to SEC-EXT-017 are R2; baseline owner
  decision 8), so R1 has no scripting API. This package now runs in wave
  7 with the R2 packages, takes over the `gmk_…` key format that WP-031
  dropped, and must meet SEC-EXT-008 to SEC-EXT-017 and SEC-IAM-083. The
  R1 rows it served (ACC-049, INT-006, INT-012, INT-017 to INT-022,
  API-TOK-01, API-TOK-02) move to R2 with it (INT-023 stays R1 with WP-031
  and WP-043); WP-093's tool write API and
  WP-098's token-scoped refresh go with it. The scope below is the first
  draft's and is revised before wave 7 starts.
- **Release** R2 · **Wave** 7 (was 3) · **Size** M · **Depends on** WP-031,
  WP-033, WP-047, WP-062, WP-065, WP-066, WP-118.
- **Owns** `crates/gunmetal-server/src/tokens/`.
- **Serves** ACC-049, INT-006, INT-012, INT-017 to INT-022 (INT-023 stays
  R1 with WP-031 and WP-043); API-TOK-01, API-TOK-02.
- **Security.** Boundaries TB4; threats TM-T14. Verifies, in R2:
  SEC-EXT-008, SEC-EXT-009, SEC-EXT-010, SEC-EXT-011, SEC-EXT-012,
  SEC-EXT-013, SEC-EXT-014, SEC-EXT-015, SEC-EXT-016, SEC-EXT-017,
  SEC-IAM-083.
- **Scope.** Create tokens scoped by library, root and user, with expiry,
  rotation, last use, audit trail, per-token rate limits and revoke one or
  all; the no-escalation rule; authenticating API keys in the request
  pipeline beside WP-062's cookie sessions, yielding a `Principal` with the
  token's scope; the change feed for tools with a cursor per token,
  filtered by scope, in JSON.
- **Tests (real SQLite).** A token cannot be issued with more than its
  creator holds; a token scoped to one library sees only that library's
  changes; a revoked token fails at once; a token in a query string is
  refused.


## R2 (video) in outline

R2 adds movies and TV, the remuxer, sandboxed transcoding, the native apps
and everything that needs them (feature map, "Releases"). It reuses R1's
patterns: synced metadata, signed URLs, the user log, the session registry,
the worker and the task runner. The packages below are outlines: each will
get the same detail as R1 before its wave starts, and some will split. They
are numbered from WP-201 so R1 can grow without renumbering. Waves continue
from R1's last wave; R2 waves may begin while R1 waves 5 and 6 or the
point releases' packages finish, where they share no files. Three R2
packages are specified in full in the R2 group of
[After R1](#after-r1-point-releases-and-later) rather than here: WP-091
(scoped API keys), WP-129 (the project name service) and WP-135 (its
naming client and CT monitoring), the last two moved from R1 by the
owner's answer to D-07, beside built-in remote access (WP-221).

The largest risk is the one ADR 1 names: the pure-Rust remuxer, with Dolby
Vision, lossless audio and image-based subtitles. Jellyfin and Plex already
transcode and remux reliably with FFmpeg, so Gunmetal is behind them until
WP-201 to WP-204 and WP-213 are done.

The order of delivery across these packages, the client's video packages,
and the live-TV packages that follow them, is proposed in
[video-implementation.md](video-implementation.md), for the owner's review.

The **Security** column gives each outline package's trust boundaries,
threats and the requirement IDs its tests must verify. Each package gets
a full Security field, in the R1 form, before its wave starts. Some R2
packages also verify R1 requirements whose surface only arrives in R2
(the iroh endpoint, scrobbling); those are proved by absence in R1
(WP-001, WP-048, WP-131), as the R1 coverage table records, and the R2
packages it names verify the behaviour.

| ID | Title | Wave | Size | Depends on | Owns (outline) | Serves | Security |
|---|---|---:|---|---|---|---|---|
| WP-201 | Matroska structure: segment, seek head, tracks, cues, chapters, tags, attachments | 7 | L (expect a split into structure and index) | WP-004, `ebml.rs` | `core/src/formats/mkv/` (and moves `ebml.rs` under `formats/`) | VID-001, LIB-088, the segment map (ADR 1 decision 4) | TB6, TB9; TM-T20, TM-T29; SEC-MED-075 |
| WP-202 | MP4 video tracks, edit lists and sample tables for video | 7 | L | WP-017, WP-018 | `core/src/formats/mp4/video.rs` | VID-001, VID-003 | TB6, TB9; TM-T20; SEC-MED-001, SEC-MED-008 |
| WP-203 | Segment map builder | 8 | M | WP-201, WP-202 | `core/src/segment_map.rs` | ADR 1 decision 4; LIB-088 | TB6; TM-T20 |
| WP-204 | Remuxer: fragmented MP4 and HLS playlists from the typed model | 8 | L (split by codec family) | WP-203, WP-056 | `core/src/remux/` | VID-003, SEC-MED-032, SEC-MED-074 | TB6; TM-T20; SEC-MED-032, SEC-MED-074 |
| WP-205 | Subtitle parsers (SRT, WebVTT, ASS) and the WebVTT writer | 7 | L | WP-004, WP-005 | `core/src/subtitles/` | VID-069 to VID-076, SEC-MED-052 | TB6, TB9; TM-T07, TM-T28; SEC-API-089, SEC-MED-052, SEC-MED-053, SEC-HIS-039, SEC-CLI-053 |
| WP-206 | XML without DTDs, and NFO | 7 | M | WP-004 | `core/src/xml.rs`, `core/src/nfo.rs` | LIB-097 (R2 part), SEC-MED-056 | TB6, TB9; TM-T26; SEC-MED-056, SEC-TM-038, SEC-HIS-034 |
| WP-207 | Video catalogue model: films, shows, seasons, episodes, versions, naming rules | 7 | L | WP-040 | `core/src/video/` | LIB-002, LIB-149 to LIB-155, VID-013 to VID-015; API-VID-01 | TB9; TM-T15 |
| WP-208 | Video decision engine with stream index | 8 | M | WP-055, WP-201, WP-202 | `core/src/decision/` | VID-002, VID-169, ADM-100 | TB4; TM-T15 |
| WP-209 | Pairing protocol and device keys (core) | 7 | M | WP-031 | `core/src/pairing.rs` | ACC-061, ACC-062, ACC-051; API-AUTH-13 | TB4; TM-T47, TM-T64; SEC-IAM-055, SEC-NET-035 |
| WP-210 | Offline grants and download rules (core) | 7 | M | WP-027, WP-031 | `core/src/offline.rs` | CLI-095, CLI-096, CLI-080; API-DL-01, API-DL-03 | TB5; TM-T39; SEC-IAM-054, SEC-TM-060, SEC-CLI-036 |
| WP-211 | Control commands and the conflict rule (core) | 7 | M | WP-025 | `core/src/control.rs` | CLI-101, CLI-102; API-SES-05 | TB4; TM-T17; SEC-HIS-014 |
| WP-212 | Offline edit replay rules | 7 | M | WP-034 | `core/src/userdata/replay.rs` | CLI-094 | TB4; TM-T18 |
| WP-213 | Remux worker, CUE slice job and segment serving | 9 | L | WP-204, WP-078, WP-082, WP-105 | `worker/src/jobs/remux.rs`, `worker/src/jobs/cue_slice.rs`, `server/src/video_stream/` | VID-003, MUS-041 (re-headed FLAC slices built in the worker, the `CueSlice` job of WP-061), SEC-MED-081; API-STR-07 | TB6; TM-T21, TM-T52; SEC-MED-018, SEC-MED-023, SEC-MED-050, SEC-MED-081 |
| WP-214 | Transcode sandbox launcher and FFmpeg profiles | 8 | L | WP-045 | `worker/src/transcode/`, FFmpeg build allowlist | VID-005, VID-009, SEC-MED-063 to SEC-MED-073 | TB6; TM-T21, TM-T23, TM-T24, TM-T52; SEC-MED-064, SEC-MED-065, SEC-MED-066, SEC-MED-067, SEC-MED-068, SEC-MED-069, SEC-MED-070, SEC-MED-072, SEC-TM-047, SEC-HIS-022, SEC-OPS-062, SEC-SUP-061 |
| WP-215 | Opus encode jobs and cache | 9 | M | WP-214 | `server/src/opus/` | MUS-106, MUS-213; API-STR-06 | TB6; TM-T21, TM-T23; SEC-MED-064, SEC-MED-067 |
| WP-216 | Single-keyframe endpoint | 9 | S | WP-204, WP-213 | `server/src/keyframe/` | VID-097, VID-098; API-STR-08 | TB6; TM-T21; SEC-MED-081 |
| WP-217 | Video scan pipeline extensions | 9 | L | WP-102, WP-207, WP-201 | `server/src/scan/video.rs` | LIB-002, LIB-149 onwards | TB6, TB9; TM-T22, TM-T28; SEC-MED-053, SEC-MED-054 |
| WP-218 | Pairing and device-key sign-in | 8 | M | WP-209, WP-062 | `server/src/pairing/` | ACC-061, ACC-062; API-AUTH-13 | TB3, TB4; TM-T38, TM-T47, TM-T64; SEC-IAM-048, SEC-IAM-049, SEC-IAM-050, SEC-IAM-051, SEC-IAM-052, SEC-IAM-055, SEC-NET-033, SEC-NET-034, SEC-OPS-010, SEC-TM-063 |
| WP-219 | WebSocket control channel, handoff and casting URLs | 8 | L | WP-211, WP-083 | `server/src/control/` | CLI-101 to CLI-111; API-SES-05 to API-SES-07 | TB4; TM-T08, TM-T17; SEC-NET-064, SEC-API-098, SEC-API-041, SEC-API-042, SEC-API-043, SEC-IAM-016 |
| WP-220 | Downloads: grants, resumable transfers, rights | 9 | L | WP-210, WP-082, WP-215 | `server/src/downloads/` | CLI-078 to CLI-097; API-DL-01 to API-DL-05 | TB5; TM-T39; SEC-IAM-054, SEC-CLI-036, SEC-TM-060 |
| WP-221 | iroh remote access and relays | 8 | L | WP-218 | `crates/gunmetal-remote/` | ACC-096, ACC-100, ACC-101; API-SET-12 | TB3; TM-T49, TM-T50, TM-T51; SEC-NET-033, SEC-NET-037, SEC-NET-038, SEC-NET-039, SEC-NET-040, SEC-NET-041, SEC-NET-042, SEC-NET-043, SEC-NET-054, SEC-NET-060, SEC-NET-061, SEC-NET-062, SEC-NET-065, SEC-OPS-040, SEC-PRV-059, SEC-TM-064 |
| WP-222 | Household profiles, policies, parental controls, PINs | 8 | L | WP-033, WP-065 | `server/src/household/` | ACC-016 to ACC-034, ACC-038 to ACC-044; API-USR-08, API-ADM-04, API-AUTH-14 | TB5, TB11; TM-T15, TM-T19; SEC-IAM-061, SEC-IAM-062, SEC-IAM-063, SEC-IAM-064, SEC-IAM-065, SEC-IAM-066, SEC-IAM-109, SEC-IAM-110, SEC-PRV-028, SEC-PRV-029 |
| WP-223 | Video share links (off by default) and the guest capability | 9 | M | WP-222, WP-134 | `server/src/shares/video.rs`, `server/src/guest/` (music share links are R1.2, WP-134) | ACC-091, ACC-092, ACC-135 | TB1, TB4; TM-T16, TM-T67; SEC-API-097 |
| WP-224 | Webhooks and the public event stream | 8 | M | WP-048, WP-083 | `server/src/webhooks/` | INT-030 to INT-049; API-TOK-04 | TB8; TM-T30, TM-T36; SEC-EXT-045, SEC-EXT-046, SEC-EXT-047, SEC-EXT-048, SEC-EXT-049, SEC-EXT-050, SEC-OPS-035 |
| WP-225 | WebAssembly plugin host with grants | 8 | L | WP-048, WP-047 | `crates/gunmetal-plugins/` | INT-054 to INT-069; API-TOK-05 | TB7; TM-T34, TM-T35, TM-T36, TM-T37; SEC-EXT-019, SEC-EXT-020, SEC-EXT-021, SEC-EXT-022, SEC-EXT-023, SEC-EXT-024, SEC-EXT-025, SEC-EXT-026, SEC-EXT-027, SEC-EXT-028, SEC-EXT-029, SEC-EXT-030, SEC-EXT-031, SEC-EXT-032, SEC-EXT-033, SEC-EXT-034, SEC-EXT-035, SEC-EXT-036, SEC-EXT-037, SEC-EXT-038, SEC-EXT-039, SEC-EXT-040, SEC-EXT-041, SEC-EXT-042, SEC-EXT-043, SEC-EXT-044, SEC-TM-065, SEC-TM-066, SEC-HIS-057 |
| WP-226 | Scrobbler and lyrics-lookup plugins | 9 | M | WP-225, WP-086 | `plugins/` | MUS-192 to MUS-195, MUS-162 | TB7, TB8; TM-T33; SEC-PRV-033, SEC-PRV-034, SEC-PRV-035, SEC-PRV-036 |
| WP-227 | OpenSubsonic adapter | 8 | L | WP-065, WP-082, WP-091, WP-093 | `server/src/subsonic/` | INT-086, INT-087, ACC-130; API-TOK-06 | TB1, TB4; TM-T66; SEC-EXT-051, SEC-EXT-052, SEC-EXT-053, SEC-EXT-054, SEC-EXT-055, SEC-EXT-056, SEC-EXT-057, SEC-EXT-058, SEC-EXT-059, SEC-EXT-060, SEC-EXT-061, SEC-EXT-062, SEC-EXT-063, SEC-EXT-064, SEC-EXT-065, SEC-EXT-066, SEC-EXT-067, SEC-EXT-068, SEC-EXT-069, SEC-API-093, SEC-API-094, SEC-HIS-051, SEC-PRV-032, SEC-TM-070 |
| WP-228 | Metadata editing, locks and item history | 8 | M | WP-107 | `server/src/editing/` | LIB-172 to LIB-178; API-CAT-14 | TB4; TM-T14 |
| WP-229 | Importers for Plex, Jellyfin, Navidrome and iTunes data | 8 | L | WP-057, WP-108 | `server/src/importers/` | ADM-036 to ADM-049 | TB10; TM-T60; SEC-STD-031 |
| WP-230 | Intro detection and skip markers | 10 | M | WP-214, WP-217 | `worker/src/jobs/intro.rs`, `server/src/markers/` | VID-110 to VID-116; API-VID-03 | TB6; TM-T21; SEC-MED-064 |
| WP-231 | UniFFI bindings for the native apps | 8 | M | WP-088's surface | `crates/gunmetal-ffi/` | ADR 1 decision 2; CLI-022 on native | TB4, TB5; TM-T38, TM-T62; SEC-STD-039 |
| WP-232 | Resume points, Continue Watching and Next Up | 8 | M | WP-034, WP-207 | `core/src/video/progress.rs`, `server/src/progress/` | VID-118 to VID-122, DIS-024 to DIS-031; API-LOG-07, API-VID-02 | TB4; TM-T18; SEC-PRV-028 |
| WP-233 | Statistics and year in review | 8 | S | WP-034 | `core/src/stats.rs` | MUS-186, MUS-187; API-LOG-06 | TB4; TM-T18 |
| WP-234 | Partial sync and restricted-profile filtering | 9 | M | WP-084, WP-222 | `server/src/sync/partial.rs` | CLI-023, DIS-144, DIS-155; API-SYNC-08, API-SYNC-09 | TB4, TB5; TM-T15; SEC-IAM-064, SEC-IAM-076 |
| WP-091 | Scoped API keys and the tool change feed (moved from R1) | 7 | M | WP-031, WP-033, WP-047, WP-062, WP-065, WP-066, WP-118 | `server/src/tokens/` | ACC-049, INT-006, INT-012, INT-017 to INT-022 (INT-023 stays R1 with WP-031 and WP-043); API-TOK-01, API-TOK-02 | TB4; TM-T14; SEC-EXT-008, SEC-EXT-009, SEC-EXT-010, SEC-EXT-011, SEC-EXT-012, SEC-EXT-013, SEC-EXT-014, SEC-EXT-015, SEC-EXT-016, SEC-EXT-017, SEC-IAM-083 |

R2 also needs the owner's answers on the Linux desktop shell, Dolby Vision
on DV televisions, transcoding scope and relay funding (feature map open
decisions 10, 19, 20 and 21) before the packages that depend on them start.

## Decisions the owner must make

Each item says which packages wait for it. Recommendations are this plan's;
the owner may choose otherwise.

The security baseline has its own list of open owner decisions
([docs/security/README.md](../security/README.md#open-decisions-for-the-owner)),
and the baseline is written as if each recommendation there is accepted.
This plan now follows those recommendations by default. Where one of the
decisions below was already answered by the baseline, the item says so;
the owner confirms the baseline's answer rather than choosing again. The
baseline decisions that change this plan most are 1 (no passwords or
TOTP), 2 (the name service in R1), 7 (music share links in R1), 8 (API
keys in R2), 10 (Linux-only servers in R1), 14 (refuse root with no
override) and 22 (built-in metadata providers).

The owner answered the register's decide-first decisions on 2026-10-02
([Owner answers](../decisions.md#owner-answers-2026-10-02)), and those
answers win over the recommendations below. The ones that reshaped this
plan: D-10 adopted the smaller R1, with the rest in R1.1, R1.2 and R1.3
(see [After R1](#after-r1-point-releases-and-later)), which answers
baseline decisions 7 (music share links: R1.2) and 22 (providers built
in: R1.1); D-07 gives R1 HTTPS through the owner's own domain with
automatic certificates, a tailnet or the same machine, and moves the
name service, its client and CT monitoring to R2, which reverses
baseline decision 2; remote access in R1 goes through the owner's own
reverse proxy or a tailnet, and built-in remote access is R2; D-09 makes
R1 servers Linux on x86-64 and ARM64 plus a Docker image, with no
transcoding; and what admins see is who is playing and totals, not what,
unless each person opts in, and no history. Items 3, 5, 10, 17, 21, 24,
29 and 30 below are updated to match.

1. **Accept ADR 3, durable user state (WP-002).** Blocks WP-034, WP-035,
   WP-046, WP-068 and, through them, most of R1. *Recommendation:* accept
   the shape in this plan: per-profile and household log streams in
   framed, checksummed monthly segments; documents stored as operations
   with periodic snapshots; the identity store as its own SQLite file with
   migrations after R1.
2. **Accept ADR 4, the audio packager (WP-003).** Blocks WP-056 and WP-105.
   *Recommendation:* accept; without it, browser gapless depends on each
   browser's native Media Source Extensions support, which is unverified.
3. **Accept or reject ADR 5, audio decoders in the scan worker (WP-003).**
   Blocks WP-029 and WP-114, which are R1.3 in the adopted R1. *Recommendation:* accept Symphonia in the
   scan worker only, never in the server process, after its review under
   SEC-MED-026 (SEC-MED-018, SEC-MED-024; register D-09, owner to
   confirm); R1 still ships the tag-and-fallback path for Opus and HE-AAC,
   which it reportedly cannot decode. If rejected, R1 uses loudness tags
   and the fallback gain (MUS-089). The first title of this item said
   "in-process audio decoders", although the decoder was always meant to
   run in the worker.
4. **The core's dependencies and the wire format.** The core has none today,
   and SEC-SUP-025 allows only a reviewed allowlist, which WP-001 checks.
   This plan adds `serde`, `postcard`, `sha2` (used only in the core's
   crypto module, WP-122), `unicode-normalization` and, for the
   decompression helper (WP-128), a pure-Rust inflate crate; `sha1` was
   dropped with TOTP, and `hmac` left the core in the second security
   pass, because core functions take a `MacProvider` and the one MAC
   implementation is the secrets crate's (SEC-STD-018). `rusqlite` joins
   `gunmetal-fs`, where the one SQLite connection opener now lives
   (WP-126).
   *Recommendation:* accept these; the alternative
   to `serde` and `postcard` is a hand-written codec for every protocol
   type, which is a large amount of code under the mutation gate for no
   security gain once frames are capped and revalidated. Two further
   questions belong here: whether `serde_json` may join the core for the
   history-import parsers (WP-057; the plan's default keeps JSON decoding
   out of the core and out of the server process, in the worker's import
   job, SEC-MED-018), and the "not used, on purpose" list, which is this
   plan's proposal rather than a settled rule.
5. **The crate layout.** Fourteen crates (twelve, plus the existing
   `gunmetal-fuzz` and the name service's `gunmetal-names`, which is
   created only in R2 now that the name service is R2), with the core
   kept as one crate as ADR 1 says. *Recommendation:* accept, and record it in an ADR that
   extends ADR 1 rather than rewriting it (ADR 6, drafted by WP-003).
6. **How the gate scales.** One full mutation run over a growing workspace
   will outgrow the 30-minute CI job. *Recommendation:* on pull requests,
   mutate only the code the diff touches, sharded across runners, and run
   the full mutation pass on every merge to an integration branch and
   nightly; the definition of done stays "the full gate passes on the
   integration branch". cargo-mutants offers options for diff-scoped and
   sharded runs (unverified against the pinned version); WP-001 confirms
   them before changing `gate.sh`.
7. **HTTP stack and the R1 event channel.** *Recommendation:* axum on
   tokio, and server-sent events for the R1 channel (WP-083), leaving
   WebSockets to the R2 control channel. Server-sent events need no new
   dependency and pass through reverse proxies easily, but they are
   one-way, so R1's "Play here" is an ordinary request.
8. **TLS crypto provider for rustls.** The common providers include C or
   assembly; a pure-Rust provider exists but its maturity is unverified.
   *Recommendation (register D-08):* aws-lc-rs if it builds cleanly for
   the R1 Linux targets (x86-64 and AArch64), otherwise ring, chosen
   through the reviewed crypto allow-list and recorded in the cryptography
   architecture record that WP-125 drafts (SEC-STD-019); revisit when a
   pure-Rust provider is proven. The first recommendation, "whatever
   rustls defaults to", let an upstream default make the choice instead
   of the one reviewed door (principle 8).
9. **WebAuthn implementation and algorithms.** *Recommendation:* hand-written
   verification on RustCrypto primitives for ES256 and EdDSA (WP-041,
   WP-081), and decide on RS256 after checking which authenticators the
   household is likely to use still require it (unverified). Allow recorded
   real ceremonies as test fixtures, since they are not media.
10. **OIDC implementation and RSA.** OIDC is R1.2 in the adopted R1
    (D-10), so this blocks WP-096 in R1.2, not R1. *Recommendation:*
    hand-written, minimal code flow with pinned algorithms (WP-096),
    because the general-purpose crate brings a larger dependency tree than
    the one flow needs. Either way, the owner must also choose whether
    R1.2 verifies RS256, which
    needs an RSA crate the plan did not list, or supports only providers
    configured for ES256 or EdDSA, which would exclude providers left on
    their defaults (unverified per provider).
11. **Public IDs for library items.** Random and stored, or derived from
    content identity with a server key (web-and-api-security.md open
    decision 3). *Answered by the baseline; owner to confirm:* random IDs
    from the CSPRNG, kept in the identity store's public-ID mapping, so
    they survive cache rebuilds (SEC-HIS-012; baseline README decision 25,
    "random IDs with a rebuildable cache"). This plan first recommended
    keyed derivation, which SEC-HIS-012 rules out because the identity
    order can fall back to the path stem.
12. **Update-feed verification and backup encryption.** *Partly answered by
    the baseline:* the feed is TUF 1.0 metadata verified from a root
    compiled into the binary (SEC-SUP-049, SEC-OPS-019; baseline decision
    13 for TUF from R1, owner to confirm), and backups are age v1 to the
    server's backup key and the owner's recovery key (SEC-OPS-042). Still
    open: `tough` if its dependency tree passes review, or a minimal TUF
    client; and the archive inside the encryption (hand-written or the
    `tar` crate), which record 10 (WP-125) must also allow to be extracted
    (SEC-HIS-019).
13. **Default parser and worker limits** (media-and-parser-safety.md open
    decision 5). Blocks WP-004's constants. *Recommendation:* accept the
    table as written.
14. **Two players on one queue in R1.** player.md proposes an active-device
    field; flows G11 proposes that the last device to press Play wins.
    *Recommendation:* both at once: the queue document carries the active
    device (WP-025), set by Play, pushed over the event channel (WP-083).
15. **Sync payload budgets and the reference devices.** Seek indexes,
    lyrics and long histories may not fit in a full sync at 100,000 tracks
    (api-needs.md, "The sync model"). *Recommendation:* fetch seek indexes
    when a track enters the queue; measure lyrics before deciding; sync
    aggregates and a recent window of history; and name the reference
    low-end devices for WP-115's budget tests. The budgets are enforced
    in the R1 gate either way (register D-87); until the devices are
    named, the tests run on the reference low-end profile.
16. **What R1 does with writes while offline.** *Recommendation:* as
    api-needs.md proposes: only plays and positions are queued; loves,
    ratings and playlist edits are disabled with "Needs the server", and
    the event format already allows R2 to queue them.
17. **Rating model** (feature map open decision 12). Blocks the rating
    event body, which moved with ratings to WP-141 (R1.1); it no longer
    blocks WP-034. *Recommendation:* as the feature map recommends.
18. **Server platforms in R1 and the watcher.** *Answered by the baseline;
    owner to confirm:* Linux-only servers in R1; macOS and Windows server
    builds ship only once their worker sandbox profiles exist
    (SEC-MED-082, R2; baseline decision 10). This plan first proposed
    macOS and Windows builds with the reduced tier. Docker Desktop covers
    those hosts in R1. *Recommendation* for the watcher: `notify`.
19. **Who sees a new library** (flows G4). *Recommendation:* the owner and
    admins only, until someone is granted access.
20. **Compressed ID3v2 frames.** *Recommendation:* skip and record them in
    R1, so the core needs no inflater yet.
21. **Built-in HTTPS by ACME in R1** (WP-101). *Answered by the owner on
    2026-10-02 (register D-07):* R1 gets HTTPS through the owner's own
    domain with automatic certificates (WP-101), a tailnet, or the same
    machine; the per-server name service, its naming client and its CT
    monitoring are not in R1 and move to R2 (WP-129, WP-135). The
    baseline had recommended the name service as R1's install-time
    default (baseline decision 2), and this plan had followed it; the
    owner's answer reverses that. This plan first left R1 or a point
    release open. Still open: the ACME client's crates (see "Missing from
    this table").
22. **The benchmark library.** *Recommendation:* the synthetic generator in
    CI for regressions, plus a published comparison run by the owner on a
    real library they own, with its shape described but no files shared.
23. **Configuration format.** *Recommendation:* TOML.
24. **32-bit ARM.** `seccompiler` does not support it. *Answered for R1
    by the owner (register D-09, D-10):* R1 servers are x86-64 and ARM64
    and a Docker image, and 32-bit ARM is R1.3 (WP-160). *Recommendation
    for R1.3:* build it and label the isolation tier as reduced on the
    health page, but do not claim it as supported until the seccomp answer
    for ARMv7 is recorded, as SEC-MED-024 requires (baseline decision 10).
    The first draft said to call it supported with the label.
25. **The integrator and the merge protocol.** Someone has to own the root
    `Cargo.toml`, `scripts/gate.sh` and `ci.yml` after wave 0, resolve
    registry conflicts and run the full gate per wave, and the merge
    protocol itself changes CONTRIBUTING.md, the project's binding rules.
    *Recommendation:* the owner names one maintainer or one dedicated
    agent per wave, and accepts or edits the protocol before WP-001 writes
    it into CONTRIBUTING.md.
26. **The test-support crate under the gate.** *Recommendation:* accept that
    `gunmetal-testkit` is covered and mutation-tested like any crate; it is
    what keeps the oracles honest.
27. **Private session expiry** (API-USR-07). *Recommendation:* end it after
    a period without playback, as the privacy research proposes, and show
    it clearly while on.
28. **Picks survive a new Play** (player.md proposal). *Recommendation:*
    accept; WP-025 implements it.
29. **Playlists from folder files and missing entries** (flows G5 and G6).
    Folder playlists are R1.1 (WP-112); missing entries are R1 (WP-093).
    *Recommendation:* folder playlists are read-only with "Duplicate to
    edit"; a purged track stays in a playlist as a "missing" entry that
    rematches by identity.
30. **What admins and members see of other people's sessions** (feature
    map open decision 13). *Answered by the owner on 2026-10-02:* who is
    playing and totals, not what, unless each person opts in, and no
    history (SEC-PRV-025, SEC-TM-054; baseline decision 5). The live view
    itself is R1.2 (WP-153); in R1 no admin route shows another person's
    sessions or history at all (WP-104). This plan first recommended live
    sessions, totals and security events by default, with titles.
31. **Artwork at cookie-authorised, content-addressed paths** (api-needs.md
    Flags item 7). *Answered by the baseline; owner to confirm:* artwork
    is served only from capability URLs, with a 1-hour lifetime aligned
    to a time bucket so repeat requests reuse one URL and the browser can
    cache it (SEC-API-026, SEC-API-027; baseline decision 25, "media URL
    binding"). This plan first recommended cookie-authorised paths for the
    first-party web client, which would add a second authentication
    mechanism for media; that proposal is withdrawn.
32. **Release signing keys** (feature map open decision 25). *Partly
    answered by the baseline:* release artefacts and images are signed
    keyless with Sigstore and carry SLSA Build Level 3 provenance, so they
    need no long-lived key (SEC-SUP-041 to SEC-SUP-043; WP-136); the
    update feed's TUF roles are signed by offline hardware keys, threshold
    2 once two keyholders exist (SEC-SUP-049). Still open, as the
    baseline's decision 19: finding the second maintainer and keyholders
    before R1.
33. **Scope of the `std::fs` ban** (SEC-MED-033). *Changed for the
    baseline; owner to confirm:* the baseline bans path-based `std::fs`
    outside the filesystem module and requires every file access, library
    or data directory, to go through a directory handle beneath its root
    (SEC-MED-033, SEC-HIS-016). The data-directory modules therefore use
    the data-root handle in `gunmetal-fs` (WP-126). SQLite's own open,
    which takes a path the data-root handle builds from constants, now
    happens only inside `gunmetal-fs` too (the one connection opener,
    WP-126). The first draft said the only exception, checked by an xtask
    so the list cannot grow silently, is the dev-only testkit. The xtask
    list now has eleven entries (record 6 decision 8 names them): the
    doors that exist, plus the testkit, the fuzz corpus replay, the
    xtask's tree module, the filesystem tests, the core's minting test
    helper and the core's bounded-capacity helper. The first draft exempted the
    cache, the identity store, the log writers, the secrets crate and the
    data-directory module from the ban altogether, and the first security
    pass still exempted SQLite's open in two crates.
34. **`unsafe` in the WASM facade.** The workspace forbids `unsafe_code`
    everywhere, while the README promises only "no `unsafe` in the core".
    If `wasm-bindgen`'s generated code trips the forbid lint (unverified),
    WP-088 needs a crate-level exception. *Recommendation:* allow it in
    `gunmetal-wasm` only, where the code is generated glue, and keep it
    forbidden everywhere else. *Answered 2026-10-03 as a delegated
    technical choice (register, technical answers):* the core keeps
    `forbid` with no exception; the facade crate's manifest repeats
    every workspace lint with only `unsafe_code` lowered as far as
    generated code needs, `xtask lint-exceptions` permits that one
    manifest beside the core's, hand-written `unsafe` is refused there
    by an xtask check on the keyword, and WP-235 proves the `wasm32`
    build under it before any other facade slice starts.
35. **Service install and "one command"** (ADM-005). SEC-MED-063 forbids
    the server from starting programs outside the sandbox launcher, so the
    binary cannot create the service user or enable the unit itself.
    *Recommendation:* the binary prints the unit and a shipped install
    script does the privileged steps (WP-121), and ADM-005's "one command"
    is that script.
36. **Baseline rows that disagree with each other** (added for the
    security baseline). Aligning the plan found three pairs of R1 rows
    that set different values for one control, none of which the
    baseline's control-ownership or parameters table resolves: image
    limits (SEC-API-086: 8,192 pixels per side and 40 megapixels;
    SEC-MED-045: 16,384 and 64 megapixels), lyrics caps (SEC-API-090:
    256 KiB and 10,000 lines; SEC-MED-049: 1 MiB and 20,000 lines), and a
    Range header with several ranges (SEC-API-031 and SEC-NET-050: 416;
    SEC-MED-060: the full representation). The plan applies the stricter
    value in each case, which satisfies both rows (WP-079, WP-021,
    WP-023). *Recommendation:* the owner of the baseline records one owner
    for each control in the parameters table, so the docs lint (WP-127,
    SEC-TM-072) can hold them in step; until then, the stricter values
    stand.
37. **Baseline rows whose Release comes before their surface** (added in
    the second security pass; touches baseline owner decision 3). Five
    rows are R1 in the baseline although the surface they protect does
    not exist until R2: SEC-NET-054 and SEC-OPS-040 (the iroh endpoint,
    which baseline decision 3 recommends for R2) and SEC-PRV-034,
    SEC-PRV-035 and SEC-PRV-036 (scrobbling and external-account linking,
    R2 in the feature map). As written they contradict SEC-TM-074, which
    requires a row's Release to be neither earlier nor later than its
    surface, so WP-127's own docs lint will fail against the baseline,
    and the traceability check would fail R1 for want of tests.
    *Applied by default; owner to confirm:* this plan proves each by
    absence in R1, as it already does for XML and archives: WP-001 bans
    `iroh` and its port-mapper crates, WP-131's "routes that must not
    exist" suite asserts there is no external-account linking or
    scrobbling route, and WP-048's closed purpose enum has no scrobbling
    purpose; WP-221 and WP-226 still verify the real behaviour in R2.
    *Recommendation for the baseline's owner:* move these five rows to
    R2. This plan does not edit `docs/security/`.
38. **One crypto module, read as one door in two files** (added in the
    second security pass). SEC-STD-018 says cryptographic crates "may be
    used only from one `crypto` module". The core needs SHA-256 in wave 0
    (the schema digest) and is compiled into the web client, which must
    not depend on the secrets crate, and keys must never leave the
    secrets crate. *Applied; owner to confirm:* cryptographic crates are
    used only from two named modules: `gunmetal-core/src/crypto.rs`
    (WP-122), which holds SHA-256 and nothing else, and
    `gunmetal-secrets/src/crypto/` (WP-047), which holds every other
    algorithm (HMAC, HKDF, XChaCha20-Poly1305, Argon2id, Ed25519, P-256,
    age and the rustls provider and configuration constructors) and names
    SHA-256 only where HMAC and HKDF need it; WP-001's lint enforces both
    and WP-047's xtask diffs the inventory against them. As first
    written, these crates were spread over the core, the secrets crate,
    the server and the egress client with no confinement. The
    alternative that meets the words exactly, every algorithm in the
    core's crypto module behind a server-only cargo feature, would put
    seven more crates on the core's reviewed dependency allowlist
    (SEC-SUP-025).

## Review notes

An adversarial review on 2026-10-02 checked the plan against the rules in
CONTRIBUTING.md and AGENTS.md, the feature map's R1 cut and the R1 rows of
api-needs.md. It changed the plan in place. What changed, and what it
could not settle, is below. A second pass the same day aligned the plan
with the security baseline, and a third, on 2026-10-03, cut the plan down
to the R1 the owner adopted; a fourth, the same day, applied the owner's
answers to the release-ordering questions. Their changes come first,
newest first.

### Release-ordering answers (register D-83 to D-88, 2026-10-03)

The owner accepted every recommendation in D-83 to D-88. The plan now
follows them; wave 0 is unchanged, and no requirement was weakened.

- **D-83, track details in R1.** R1 ships a minimal, read-only details
  view, MUS-236 (title, credits, album, file format, and the playback
  decision with its reason). WP-055 builds its summary beside the
  decision, now also reading WP-028's gain decision (wave 1), and WP-088
  hands it to the web client. The full track info sheet (MUS-114) stays
  R1.1 and extends the same view.
- **D-84, uncertain matches.** No change: WP-076 and WP-077 already keep
  unsure albums apart and never merge an ambiguous file before the review
  queue (WP-107, R1.3).
- **D-85, the rule format.** WP-027 moved whole from R1.3 to R1.1, so
  saved filters are rule documents from R1.1 and the format's budgets
  (SEC-TM-032, SEC-STD-011, SEC-API-066, SEC-IAM-070) are proved with it.
  The rule store, server-side evaluation and smart playlists (WP-092,
  WP-113) stay R1.3. WP-087, WP-059, WP-088, WP-040, WP-154 and the
  coverage table were updated to match. R1.1 now has a third pair that
  must not share a wave: WP-147 follows WP-027.
- **D-86, loading without the server.** No change: WP-148 (R1.1) already
  carries offline loading, and R1's web client registers no service
  worker (WP-072).
- **D-87, speed budgets.** WP-115 now owns the budget tests in the R1
  gate for the search index (WP-054) and the sync snapshot (WP-084); the
  list render budget (DIS-100) is the client plan's test in the same
  gate. Only the published numbers wait for DIS-019 in R1.1.
- **D-88, browsing by mood and label.** R1 browses by genre (WP-040's
  genre index); mood and label browsing arrive with MUS-019 in R1.1.
  WP-049 already maps both fields in R1, so no package changes.

### The adopted R1 and the point releases (owner answers of 2026-10-02)

The owner adopted the smaller R1 in the register's R1 scope section, with
the rest in R1.1, R1.2 and R1.3 (D-10); moved the project name service,
its naming client and its CT monitoring to R2 (D-07); put R1's remote
access through the owner's reverse proxy or a tailnet, with built-in
remote access in R2; and made R1 servers Linux on x86-64 and ARM64 plus a
Docker image (D-09). The plan now follows those answers.

- **Waves 1 to 6 build exactly the adopted R1.** Seventeen packages that
  serve only a point release or R2 left the waves for
  [After R1](#after-r1-point-releases-and-later), with their full
  specifications: to R1.1, WP-022, WP-057, WP-071, WP-112, WP-123 and
  WP-137; to R1.2, WP-096 and WP-134; to R1.3, WP-027, WP-029, WP-058,
  WP-092, WP-107, WP-113 and WP-114; to R2, WP-129 and WP-135. WP-091,
  already R2, moved there too. Thirty-three R1 packages were split, the
  R1 part keeping the ID and the later part going to one of 22 new
  packages (WP-140 to WP-161) or to a moved package; the table in
  [After R1](#after-r1-point-releases-and-later) lists them by release.
- **HTTPS and naming.** WP-129's project-site security files split off to
  WP-139 in wave 1, because they are R1 surfaces whatever happens to the
  name service; WP-129 keeps the service and goes to R2 with WP-135.
  WP-101 (ACME) is now R1's path to HTTPS for an own domain and stays in
  wave 4 (setup does not call it, and it depends on WP-073's listener
  configuration in wave 3). WP-048's egress purposes in R1 are ACME and
  the update feed only; WP-080's claim works over localhost, an own
  domain, a tailnet or the owner's reverse proxy; WP-117 claims over each.
- **Waves re-checked.** WP-059 moved back to wave 2 and WP-088 to wave 3,
  because the packages that held them back (WP-058, WP-027) left R1.
  WP-090, WP-095, WP-102 and WP-103 no longer depend on WP-071, and
  WP-110 no longer depends on the packages its R1.2 roll-up needed. A
  dependency and owned-path check over waves 1 to 6 found no package
  depending on a same-wave or later package and no two same-wave packages
  owning one path. Wave 0 is unchanged.
- **Security coverage rebuilt.** The R1 table now lists 576 requirements:
  the 607 R1 rows of the baseline, less the 25 the register moves with
  their surfaces to R1.1 and R1.2 and the six that protect only the name
  service (SEC-NET-010 to SEC-NET-012, SEC-NET-069 to SEC-NET-071), which
  move to R2. Every one of the 576 still has an R1 package; where a moved
  package was the only carrier, an R1 package that has the surface now
  carries it (SEC-PRV-016 by WP-084 and WP-103; SEC-PRV-031 by WP-094's
  invite landing; SEC-API-085 by WP-109's backup upload; SEC-OPS-007 by
  WP-101; SEC-STD-011's search part by WP-054; the site rows by WP-139).
  No requirement was weakened, and none moved unless every surface it
  protects moved.

What this pass could not settle:

- **SEC-PRV-013's setup-step clause.** The register keeps SEC-PRV-013 in
  R1, but R1 has no provider to list, so its "setup must include a
  provider step that must be answered" clause has no R1 surface; WP-080
  keeps the step's place and says nothing is looked up, and WP-137 makes
  it the required question in R1.1. The security file may want to say so.
- **Wave 0 text that still assumes the name service.** WP-125's record 8
  is described as making the name service the install-time default, and
  WP-008's parser-module list names `import/`, `m3u.rs` and `provider/`.
  Wave 0 is being built and is not changed here. When WP-125 writes
  record 8, it must follow the owner's answer to D-07 (own domain with
  ACME, a tailnet or localhost in R1; the name service in R2); this is an
  input to the record's review, not a change to WP-125. The extra entries
  in WP-008's list only check harnesses for modules that will exist after
  R1.
- **The five rows of D-10** (SEC-NET-054, SEC-OPS-040, SEC-PRV-034 to
  SEC-PRV-036) stay proved by absence in R1, as the register says, until
  the owner moves them.
- **SEC-OPS-007's text** now marks the name-service exception and the
  label claim URL as from R2 (D-07); for R1 the plan proves its pre-claim
  rule for the ACME, tailnet and localhost paths (WP-101, WP-117), and the
  name-service clause from R2 (WP-135).
- **Release values.** WP-127's lint now accepts R1, R1.1, R1.2, R1.3, R2,
  R3, Later and No, keeping Withdrawn for withdrawn requirement rows,
  which the security files still use; whether Withdrawn stays is for the
  owner of those files.

### Alignment with the security baseline

The owner directed that every planning document be built around the
security baseline's first principles. Where this plan contradicted the
baseline, the plan now follows the baseline's recommendation; where that
touches one of the baseline's open owner decisions, the item is marked
for the owner to confirm in [Decisions the owner must make](#decisions-the-owner-must-make).

- **Every package has a Security field** naming its trust boundaries,
  threats and the requirement IDs its tests must verify (SEC-TM-001,
  SEC-STD-004), and [the coverage table](#security-coverage-every-r1-requirement-has-a-package)
  maps all 607 R1 requirements.
- **No passwords and no TOTP** (SEC-IAM-025; baseline decision 1).
  WP-063 keeps only recovery codes and moved to wave 3; WP-038 dropped
  TOTP; WP-120 became sign-out and browser pairing (SEC-IAM-108); WP-080
  lost its password branch for pages that are not a secure context;
  `sha1` left the crate list.
- **One door per risk.** Added the data-root handle (WP-126) so no
  data-directory module needs path-based `std::fs`; made WP-064 the one
  credential verifier with a closed pathway inventory; made WP-045's
  launcher the one typed command builder; added request limits and the
  limits register (WP-130) and the decompression helper (WP-128); gave
  storage a `Permit` only the policy can mint; listed the doors in the
  ground rules.
- **Packages added:** WP-124 repository protections and the security
  process; WP-125 security architecture records; WP-126 data-root
  handle; WP-127 docs lints and traceability; WP-128 decompression
  helper; WP-129 name service and project-site security files; WP-130
  request limits; WP-131 route-table security suites; WP-132 postures,
  the cleartext rule and exposure alerts; WP-133 history deletion,
  retention and account deletion; WP-134 music share links; WP-135 the
  naming client and CT monitoring; WP-136 release provenance, signing
  and the feed publisher; WP-137 metadata providers (conditional).
- **Moved:** WP-091 (API keys) to R2 (baseline decision 8); WP-063 to
  wave 3; trusted proxies from WP-073 to WP-132; WP-101 confirmed in R1;
  the fuzz harnesses to `crates/gunmetal-fuzz` and `fuzz/seeds/`, as the
  repository and the secure-coding guide already have them.
- **Behaviour changed:** unauthenticated discovery no longer reveals the
  version (WP-089); root is refused with no override (WP-043); a Range
  header with several ranges gets 416 (WP-023); `Gunmetal-Request` is
  required on every cookie request, not only mutating ones (WP-044);
  ownership moves only by a confirmed transfer (WP-094); invitations
  default to one use and leave member accounts pending until the inviter
  confirms a matching code (WP-094); public IDs are random, not derived
  (WP-046, WP-077); artwork uses bucketed capability URLs, not cookie
  paths (WP-103); backups are kept for a period, not a count (WP-090);
  the admin session view hides titles (WP-104); alerts are in-app only in
  R1 (WP-097); restore opens restored databases only in a jailed worker
  and waits for an archive record (WP-109); release builds are Linux only
  and 32-bit ARM is not claimed as supported (WP-121); lyrics and image
  limits take the stricter of two baseline rows (WP-021, WP-079).
- **Owner decisions** 11, 12, 18, 21, 24, 30, 31, 32 and 33 are now
  answered or narrowed by the baseline, and decision 36 was added for
  baseline rows that disagree with each other.

### Second security pass: doors that arrived after their users

A review of the aligned plan found that several doors were built a wave
or more after the packages that needed them, so those packages would
either have opened a second door or left a requirement unproven. Each
door now lands first:

- **Fresh user verification** moved from WP-106 (wave 4) to WP-062
  (wave 2); WP-106 keeps the step-up route that renews the assertion.
- **The client-address resolver** moved from WP-132 (wave 3) to WP-118
  (wave 2), with `ClientContext` (WP-006) as the only address input the
  sessions, verifier, audit log and limiters accept and a lint against
  reading the socket peer anywhere else (WP-001). WP-080 gained a test
  that a same-host proxy cannot reach the claim's loopback exemption.
- **The audit log** gained its bus sink and a dependency on the bus
  (WP-069); the typed event catalogue and sink trait moved to the core
  (WP-006), so producers prove only that they emit the event. Addresses
  moved out of the hash-chained records into a side store that can be
  coarsened and deleted on schedule (owner to confirm, baseline decision
  15).
- **The authorisation door** now covers the identity store, the user log
  and the audit log as well as the catalogue (WP-046, WP-065, WP-068,
  WP-069), with a written list of pre-principal lookups and an
  architecture test.
- **The public-route allow-list and the anonymous suite** moved from
  WP-131 to WP-118 (wave 2); the Definition of done now asks a package
  to pass the checks present when it rebases.
- **Rival replays** became a registry directory: WP-131 owns the harness
  and each feature package adds its own incidents' replays. OpenAPI
  generation moved from WP-089 to WP-118.
- **Cryptography** got a door: two named crypto modules (WP-122, WP-047)
  and a lint (WP-001); WP-031 lost its own MAC helper; the egress client
  moved to wave 2 to take its TLS configuration from the secrets crate
  (owner decision 38).
- **SQL** got a wave 0 door: the static-query type and the one
  connection opener, with `secure_delete` checked on every connection,
  moved to WP-126; WP-046 and WP-071 now verify SEC-PRV-050.
- **Public IDs** can be built only from the secrets crate's minting
  function (WP-006, WP-047); WP-046 takes minted IDs as input.
- **The retention schedule** moved from WP-133 (wave 4) to a new wave 1
  package, WP-138.
- **The server-wide counter** for secret-checking endpoints is WP-032's,
  shared by WP-064 and WP-130.
- **Requirements whose surface is absent in R1** (iroh, scrobbling) are
  proved by absence (WP-001, WP-048, WP-131), and the baseline rows'
  Release values are raised with the owner (decision 37).
- **Mappings corrected:** the JavaScript install rules of the release
  workflow (WP-136, WP-124); `Clear-Site-Data` (WP-120) and the CSP half
  of SEC-API-049 (WP-072); the server and client halves of SEC-TM-058,
  SEC-CLI-010 and SEC-TM-053; the audit routes and investigation mode
  (WP-100) and the checkpoint export (WP-090); SEC-PRV-003's session
  clause (WP-062); SEC-HIS-004's pathway half (WP-064); SEC-OPS-033's
  stream half (WP-117).
- **Shared files** added to the registry table: `supply-chain/`,
  `CODEOWNERS`, `REUSE.toml`, `limits.toml`, the security-event
  catalogue, the generated OpenAPI file and the rival replay directory.

### What changed

**Same-wave dependencies and shared files.**

- The route registry and listener could not compile in WP-043, because
  `gunmetal-http` (WP-044) is in the same wave. They moved to a new
  WP-118 (wave 2). The route-registering packages that were in wave 2
  (WP-072, WP-073, WP-074) moved to wave 3, which pushed WP-101 to wave 4.
  Every wave 3 package that owns routes now lists WP-118.
- `formats/mod.rs`, `formats/flac/mod.rs`, `formats/mp4/mod.rs`,
  `tags/mod.rs` and `music/mod.rs` were each owned by one package while
  same-wave siblings had to add to them. Every `mod.rs` and `lib.rs` is
  now a registry file that holds only module lines and that any package
  may create.
- `cli.rs`, `problem.rs`, `gate.sh` and `ci.yml` became registry or
  integrator files; WP-008, WP-088 and WP-121 own their own workflow
  files instead of editing `ci.yml`.
- WP-046 needed `SchemaPart` from WP-042 in the same wave; the part type
  and digest moved to a new core package, WP-122 (wave 0).
- WP-065 owned `Principal` (which WP-062 produces in the same wave),
  returned catalogue rows (WP-067, same wave) and relied on a grants
  table that only WP-099 (a wave later) created. `Principal`,
  `LibrarySet` and `HasLibrary` moved into the core (WP-033), WP-067's
  readers take a `LibrarySet` (now a `Permit`; see the security
  alignment above), and WP-065 owns the grants table.
- WP-067 needed WP-066's `append` in the same wave; it now returns
  `CatalogChange`s (a core type in WP-040) and WP-102 writes them.
- WP-051 called WP-049's mapper in the same wave; embedded ID3 chunks in
  WAV and AIFF are now mapped in WP-075.
- WP-059 needed WP-058 in the same wave; it moved to wave 3 and WP-088
  to wave 4.
- WP-027 and WP-040 both claimed field IDs; WP-027 now owns a plain
  `FieldId` and WP-040 lists numeric codes.
- WP-061's integration test needed the probe (same wave); it uses a test
  parser, and the probe integration moved to WP-079.
- WP-080's passkey branch was "tested in WP-081", which could not see it;
  it moved to WP-106 (`setup/passkey.rs`, wave 4).
- WP-085, WP-095, WP-099, WP-084, WP-093, WP-107 and WP-098 had tests
  that needed same-wave or later packages (the event channel, the queue
  and listening projections, identity and the pool, the scan, the trash,
  token authentication). Each now tests its own part, and the
  cross-package round trips moved to WP-117.
- WP-045 could only finish after WP-079 in a later wave; the syscall
  allowlist file now passes from WP-045 to WP-079.
- Missing dependencies added: WP-063, WP-072 and WP-074 on WP-043 (they
  live in the server crate); WP-070 on WP-042; WP-058 on WP-026 and
  WP-034; WP-053 and WP-076 on WP-034; WP-077 on WP-031; WP-078 on
  WP-060; WP-082 on WP-047, WP-067 and WP-070; WP-089 on WP-039; WP-090 on
  WP-071; WP-094 on WP-064; WP-103 on WP-065, WP-067, WP-071 and WP-078;
  WP-106 on WP-080; WP-107 on WP-076, WP-078 and WP-095; WP-108 on WP-087,
  WP-090 and WP-092; WP-109 on WP-085, WP-086, WP-087 and WP-093; WP-110
  on WP-074, WP-090, WP-091 and WP-106; WP-111 on WP-093; WP-114 on
  WP-078. WP-060 now lists its own `Cargo.toml`.

**Packages added or split.**

- WP-118 route registry and listener; WP-119 synthetic library generator
  (moved forward from WP-115, because waves 3 and 4 needed it); WP-120
  sign-in and sign-out routes (no package owned them); WP-121 release
  builds, container image and service install (ADM-002 to ADM-005 had no
  package); WP-122 schema parts and digest; WP-123 parser-upgrade re-read
  (split from WP-102, the largest package on the critical path).
- WP-003 gained ADR 6, the workspace and dependency record that owner
  decision 5 asked for and no package wrote.
- The full export (ADM-074) moved from WP-090 to WP-108, which grew to L.
- WP-034 gained the curation event bodies that WP-076, WP-107 and WP-111
  use and nobody defined. WP-067 gained the seek and frame index tables
  that WP-105 checks against and nobody stored.

**Rules from CONTRIBUTING.md and AGENTS.md.**

- Keys: core functions took key bytes, which would have forced
  `Secret::expose` outside the secrets crate. They now take a
  `MacProvider`, and the secrets crate performs keyed operations itself.
- The `std::fs` ban, read literally, forbade the data directory's own
  files; WP-001 now scopes it with a checked exception list (owner
  decision 33).
- Tests that read host facts (running as root, network filesystems, watch
  limits) now take them as inputs, and permission-based tests refuse to
  run as root rather than passing for the wrong reason.
- Statistical timing tests and 100,000-track measurements left the test
  suite (they flake, and the gate reruns the suite for every mutant);
  measurements went to WP-115.
- WP-001 no longer writes the merge protocol into CONTRIBUTING.md before
  the owner accepts it.

**Test plans that a wrong implementation could pass.** WP-009 ("never
`Unknown`"), WP-021 (monotonic `position`), WP-026 (orders written down
from the code's own output, which breaks rule 4), WP-028 (a grid without
expected values), WP-037 (properties an all-zero placeholder satisfies),
WP-054 ("found by its exact title"), WP-056 (a round trip through a
reader that cannot read fragments), WP-057 (a property contradicted by
the tie rule), WP-058 (properties an empty result satisfies) and WP-092
(comparing the core's evaluation with itself). Each now has literal
expected values or an independent oracle.

**Gaps against the feature map and api-needs.md.** Every R1 capability in
api-needs.md already had a package. Of the R1 feature rows, the ones no
package named are listed under
[R1 feature rows with no backend package](#r1-feature-rows-with-no-backend-package);
those with server needs are now served, and the rest are client-only,
documentation or reference rows.

**Owner decisions added.** 30 (what others see of sessions, which WP-104
cited but the list lacked), 31 (cookie-authorised artwork paths, which
depart from ADR 1 decision 6), 32 (release signing keys), 33 (scope of the
`std::fs` ban), 34 (`unsafe` in the WASM facade) and 35 (service install
under SEC-MED-063). Decisions 4, 10, 12, 21 and 25 were widened to cover
`serde_json` in the core, RSA for single sign-on, the backup archive, the
ACME crate and the merge protocol.

### What this review could not resolve

- **The single sign-on client secret.** Resolved in the security
  alignment: the vault decrypts it only for the call (SEC-STD-023) and the
  egress client is the one other sanctioned place to expose a `Secret`, to
  write it into the request (WP-047, WP-096).
- **`LibrarySet` is a convention, not a guarantee.** Resolved in the
  security alignment: storage readers take a `Permit` that only the core
  policy function can mint (WP-033, WP-065, WP-067; SEC-API-010).
- **Who consults the free-space guard.** WP-097 decides when a write
  would fill the disk, but the writers belong to packages that merge
  earlier, and no package is assigned to wire the guard into them.
- **The track info sheet for members (MUS-114).** Its source is the
  admin-only inspector. The plan assumes members see synced technical
  fields only; the owner or the UI plan should confirm.
- **Secure context at setup before WP-073.** Resolved in the second
  security pass: the client-address resolver, with its trusted-proxy
  rule, is WP-118's in wave 2, so WP-080 takes the path class from a
  `ClientContext` and tests the same-host proxy case itself; the full
  claim through each real proxy is still proven end to end by WP-073's
  proxy CI job and WP-117.
- **Documentation rows.** ADM-011, CLI-002 and the release notes have no
  owner in a backend plan.
- **Sizes.** WP-052 (the probe) and WP-102 (the scan) remain L and sit on
  the critical path; splitting them further would add a wave. WP-108 grew
  to L and may need the split it describes. These are estimates, not
  measurements.
- **Unverified facts this review leaned on.** Whether `wasm-bindgen`'s
  output trips `forbid(unsafe_code)`; which providers sign ID tokens with
  RS256 by default; whether the `rsa` crate's past timing advisory
  affects verification; which PRNGs publish reference vectors; whether an
  ACME test server can run in CI; whether `image` can write EXIF. Each is
  marked "(unverified)" where it is used.

## Security coverage: every R1 requirement has a package

This table lists every requirement in docs/security that is due in the
adopted R1, 576 in all, generated by a script from the requirement tables
so that none is missed, with the R1 packages (waves 0 to 6) whose tests
must verify it (each such test carries a `Verifies:` line). It was rebuilt
on 2026-10-03 for the owner's answers of 2026-10-02. The baseline has 607
R1 rows; the register moves 25 of them with their surfaces to R1.1 and
R1.2 ([Security requirements for the proposed R1](../decisions.md#security-requirements-for-the-proposed-r1)),
and the owner's answer to D-07 moves the six that protect only the
project name service and its client to R2 (SEC-NET-010, SEC-NET-011,
SEC-NET-012, SEC-NET-069, SEC-NET-070, SEC-NET-071). Those 31 are listed
after the table with the release and package that now carry them; each is
due in that release, which cannot ship without it, and none is weakened.
The security files' Release values should match; this plan does not edit
docs/security.

Of the 576, 560 are assigned to R1 packages and 16 are unassigned, all of
them web-client behaviour that belongs to a client plan. Of the 560, 6
have a server part assigned here and a client part that is unassigned and
named in the row (SEC-TM-053, SEC-TM-058, SEC-API-049, SEC-CLI-009,
SEC-CLI-010, SEC-OPS-075), and 5 JavaScript supply-chain rows are
assigned for the release workflow, with the same rule in the client's own
CI left to the client plan (SEC-SUP-033 to SEC-SUP-036, SEC-CLI-018). The
5 rows that protect surfaces the baseline's own release scope puts in R2
(SEC-NET-054, SEC-OPS-040, SEC-PRV-034 to SEC-PRV-036) are proved by
absence in R1, and their Release values are raised with the owner (owner
decision 37; register D-10). Where a package that moved out of R1 was the
only carrier of an R1 requirement, an R1 package that has the surface now
carries it: SEC-PRV-016 (WP-084, WP-103), SEC-PRV-031 (WP-094's invite
landing), SEC-API-085 (WP-109's backup upload), SEC-OPS-007 (WP-101 and
WP-117), SEC-STD-011's search part (WP-054) and the project site's rows
(WP-139). Requirements a moved package also verified keep their R1
carriers here and are re-proved by the moved package for its own surface.
The traceability check (WP-127, SEC-STD-004) fails the R1 release until
every row has a test or a dated review record, so the unassigned rows and
client parts must be planned in a client plan before R1 can ship.
[client-packages.md](client-packages.md#security-requirements-the-backend-plan-left-to-this-plan)
now assigns each of them to a client package; the rows below still say
what the backend plan itself does not carry. Short
names are the baseline README's abbreviations; the requirement text is
authoritative.

| ID | Short name | Verified by |
|---|---|---|
| SEC-TM-001 | Feature file in docs/features: name the trust boundaries it crosses and the TM-T | WP-127 |
| SEC-TM-002 | This threat model: reviewed before each release tag and whenever an entry | WP-127 |
| SEC-TM-003 | Fixed vulnerability: land with a regression test | WP-124 |
| SEC-TM-004 | Server: reject every request that lacks a valid session | WP-118, WP-131 |
| SEC-TM-005 | Route: registered with an explicit authorization policy | WP-033, WP-044 |
| SEC-TM-006 | Server: never ask a router to open ports | WP-001, WP-132 |
| SEC-TM-009 | HTTP listener: reject a Host that is not localhost | WP-044, WP-131 |
| SEC-TM-010 | When the server terminates TLS: allow only TLS 1.3 and 1.2 with forward-secret suites | WP-073, WP-101 |
| SEC-TM-012 | Shipped binaries: contain no default account | WP-047, WP-121 |
| SEC-TM-014 | Authentication pathway: listed in one inventory and apply the same per-account | WP-064, WP-069, WP-117 |
| SEC-TM-017 | Host-equivalent actions: carry the fresh-uv tag of SEC-IAM-041 | WP-033, WP-062, WP-106, WP-131, WP-132 |
| SEC-TM-024 | Read and write of a user-visible: pass through one authorization layer that takes the subject | WP-033, WP-046, WP-064, WP-065, WP-068, WP-069 |
| SEC-TM-025 | For every route: replay one user's object IDs as a second user | WP-131 |
| SEC-TM-026 | Library grants: applied by the server when building every response | WP-065, WP-084 |
| SEC-TM-027 | Write endpoints: bind only an allowlist of fields | WP-033, WP-131 |
| SEC-TM-028 | Disabling a user: take effect on the next request | WP-062, WP-082, WP-083 |
| SEC-TM-031 | Data from media files: enter as an untrusted type and be converted | WP-005 |
| SEC-TM-032 | Parser of untrusted input: enforce budgets for element size | WP-004, WP-009, WP-010, WP-011, WP-012, WP-013, WP-014, WP-015, WP-016, WP-017, WP-018, WP-019, WP-020, WP-021, WP-023, WP-035, WP-039, WP-041, WP-052 |
| SEC-TM-033 | Parser and decoder entry point: a fuzz target run in CI on each change | WP-008 |
| SEC-TM-034 | Server process: never decode untrusted images | WP-001 |
| SEC-TM-035 | Server: never send attacker-supplied image bytes to clients | WP-079, WP-103 |
| SEC-TM-036 | Clients: render untrusted text | Unassigned: web-client behaviour. The client is outside this backend plan, and no client plan exists yet; it must carry this requirement. |
| SEC-TM-038 | XML parser: DTD processing and external entity resolution disabled | WP-001 |
| SEC-TM-039 | SQL: fixed statements with bound parameters | WP-001, WP-042, WP-046, WP-126 |
| SEC-TM-040 | Errors returned to clients: generic typed codes with no stack traces | WP-044 |
| SEC-TM-041 | Published service units: run the server as a dedicated unprivileged user | WP-043, WP-121 |
| SEC-TM-042 | Server: never create | WP-060, WP-099, WP-117 |
| SEC-TM-043 | File access: resolved beneath a configured root through descriptor-relative opens | WP-024, WP-060, WP-126 |
| SEC-TM-044 | Native decoder: run in a separate jailed process with no network | WP-045 |
| SEC-TM-045 | Features that need isolation: follow the isolation table of SEC-MED-024 | WP-045, WP-110 |
| SEC-TM-046 | API: never set an executable path | WP-043, WP-045 |
| SEC-TM-048 | Server: make no outbound connection | WP-048, WP-117 |
| SEC-TM-049 | Server signing and root keys: live only in key files under SEC-OPS-012 | WP-047 |
| SEC-TM-050 | Stored field and every log: carry a data classification | WP-042, WP-046, WP-122 |
| SEC-TM-051 | Accounts: live in the durable | WP-002, WP-046, WP-095 |
| SEC-TM-052 | Backups containing identity data: encrypted with an authenticated cipher under a key stored | WP-090, WP-109 |
| SEC-TM-053 | Server and clients: send no telemetry | WP-117 (server part: no outbound socket over a full session); the client part is unassigned: web-client behaviour that a client plan must carry (a client run behind a deny-all proxy). |
| SEC-TM-054 | By default admins: see only live sessions | WP-086, WP-087 |
| SEC-TM-055 | History and audit logs: documented retention with automatic deletion | WP-108, WP-133, WP-138 |
| SEC-TM-057 | Logs: make secrets unrepresentable | WP-043, WP-047 |
| SEC-TM-058 | Web client: hold its session only in an HttpOnly cookie | WP-062 (server part: the HttpOnly session cookie); the client part is unassigned: web-client behaviour that a client plan must carry (nothing from the server in browser storage, cleared on sign-out and revocation). |
| SEC-TM-067 | Neither the project nor any third: able to disable | WP-074 |
| SEC-TM-068 | Server: enforce per-user and global limits on concurrent streams | WP-130 |
| SEC-TM-069 | Media root that is missing: marked offline and must never cause deletion of items | WP-077, WP-102, WP-111 |
| SEC-TM-072 | One security-parameters table in this file: hold every numeric and boolean security choice | WP-127 |
| SEC-TM-073 | Security control: exactly one owning requirement in the control-ownership table | WP-127 |
| SEC-TM-074 | Release-scope table in this file: the single source for which surfaces exist | WP-127 |
| SEC-TM-075 | Egress inventory in this file: list every outbound purpose with its default | WP-048, WP-127 |
| SEC-IAM-001 | Identity model: keep credentials | WP-033, WP-046 |
| SEC-IAM-002 | Request: resolve to exactly one principal of a closed set | WP-033, WP-062 |
| SEC-IAM-003 | Exactly one account: hold the owner role at all times | WP-094 |
| SEC-IAM-004 | Accounts: live in durable storage that cache rebuilds never touch | WP-002, WP-046, WP-095 |
| SEC-IAM-005 | Server: ship with no default account | WP-080, WP-121 |
| SEC-IAM-006 | Until it is claimed: answer only the claim page | WP-080 |
| SEC-IAM-007 | Claim code: 128 bits from a CSPRNG | WP-038, WP-080 |
| SEC-IAM-008 | Claim code: accepted only from loopback or from a secure context | WP-064, WP-080 |
| SEC-IAM-009 | Claiming: consume the code | WP-080 |
| SEC-IAM-010 | Server: reject any request whose Host | WP-044 |
| SEC-IAM-013 | Network location and other context signals: never widen what a principal may do | WP-033, WP-080, WP-132 |
| SEC-IAM-014 | Web client: served by the user's own server on the same | WP-044, WP-072 |
| SEC-IAM-015 | HTML response: send a Content-Security-Policy that allows no inline script | WP-072, WP-131 |
| SEC-IAM-016 | WebSocket upgrades: check Origin against the configured origins | WP-083 |
| SEC-IAM-017 | Web client: keep no session or API token in localStorage | Unassigned: web-client behaviour. The client is outside this backend plan, and no client plan exists yet; it must carry this requirement. |
| SEC-IAM-018 | WebAuthn ceremonies: use a relying-party ID fixed at setup | WP-041, WP-081 |
| SEC-IAM-019 | WebAuthn challenges: at least 128 bits from a CSPRNG | WP-041, WP-081 |
| SEC-IAM-020 | Registration: request a discoverable credential with user verification required | WP-041, WP-081 |
| SEC-IAM-021 | Server: store each credential's signature counter and its backup-eligible | WP-041, WP-081 |
| SEC-IAM-022 | Sign-in: usernameless and must not reveal | WP-064, WP-081 |
| SEC-IAM-023 | Account: able to hold several credentials | WP-106 |
| SEC-IAM-024 | Removing an account's last credential: refused unless the account is being deleted | WP-106 |
| SEC-IAM-025 | Server: never offer account passwords | WP-127, WP-131 |
| SEC-IAM-037 | First-party session and access tokens: opaque values with at least 256 bits | WP-062 |
| SEC-IAM-038 | New session token: issued at sign-in | WP-062, WP-106 |
| SEC-IAM-040 | State-changing request authenticated by cookie: carry the client's custom request header and an Origin | WP-044 |
| SEC-IAM-041 | Browser sessions: end after 7 days without use or 30 days | WP-044, WP-062, WP-106, WP-131 |
| SEC-IAM-042 | User: able to list their own sessions | WP-087 |
| SEC-IAM-043 | Revoking a session: make every token | WP-062, WP-082, WP-083 |
| SEC-IAM-044 | Administrators: able to end any or all sessions | WP-094 |
| SEC-IAM-046 | Byte-serving routes: check on every request that the session behind | WP-082 |
| SEC-IAM-047 | Stream signatures: redacted before any log line or diagnostic bundle | WP-043, WP-117 |
| SEC-IAM-056 | Pairing user codes: 8 characters from the RFC 8628 §6.1 base-20 alphabet | WP-038, WP-120 |
| SEC-IAM-057 | Pairing QR code: carry the identity key of the server the requesting | WP-120 |
| SEC-IAM-058 | Approval screen: show the requesting device's self-reported name marked as unverified | WP-120 |
| SEC-IAM-059 | Device or browser authorisation: never grant owner or administrator capabilities | WP-120 |
| SEC-IAM-060 | Approval counts as local: type the code shown on the requesting device rather | WP-120 |
| SEC-IAM-067 | HTTP: declare its authorisation policy in one route table | WP-044, WP-118 |
| SEC-IAM-068 | Authorisation: decided by one deny-by-default pure function in the core | WP-033 |
| SEC-IAM-069 | Error: end in denial and a security-log entry | WP-064, WP-065, WP-069, WP-117 |
| SEC-IAM-070 | Object fetch: pass through one shared visibility predicate built | WP-065, WP-067 |
| SEC-IAM-071 | Cross-principal suite generated from the route: replay every route with object IDs belonging to another | WP-131 |
| SEC-IAM-072 | Request bodies: decoded into per-action types that hold only the fields | WP-044, WP-131 |
| SEC-IAM-073 | Principal: never able to grant | WP-033, WP-094 |
| SEC-IAM-074 | Authorisation code: test capabilities and never role names | WP-033 |
| SEC-IAM-075 | Owner-only capabilities listed in design guidance: never grantable to any other principal | WP-033 |
| SEC-IAM-076 | Changes to roles: apply from the next request of every affected session | WP-062, WP-065 |
| SEC-IAM-077 | Administrator's access to another user's data: recorded in that user's own visible security log | WP-087 |
| SEC-IAM-078 | Invitations: carry a secret of at least 128 bits | WP-094 |
| SEC-IAM-079 | Redeeming an invitation: enrol the invitee's own passkey | WP-094 |
| SEC-IAM-080 | Guests: by default have no household-device access | WP-094 |
| SEC-IAM-081 | Until an accepted architecture record defines: authorised on the strength of another server's assertion about | WP-131 |
| SEC-IAM-089 | Owners and administrators: offered 10 single-use recovery codes of at least 80 | WP-063 |
| SEC-IAM-090 | Recovery by code or link: notify all of the account's devices | WP-106 |
| SEC-IAM-091 | Administrators: issue one-time recovery enrolment links for members and guests | WP-106, WP-109 |
| SEC-IAM-092 | Owner recovery: available only through a command on the host | WP-106 |
| SEC-IAM-094 | Security-log entries: hash-chained | WP-069 |
| SEC-IAM-095 | Secrets: held in a wrapper type that cannot be formatted | WP-047 |
| SEC-IAM-096 | Log fields derived from requests: written as structured | WP-043 |
| SEC-IAM-097 | Users: able to read their own security events | WP-069, WP-106 |
| SEC-IAM-098 | Account's devices: notified of a new device | WP-097 |
| SEC-IAM-099 | Failed sign-in: also be written as single lines in a stable | WP-064 |
| SEC-IAM-101 | Endpoint that checks a secret: rate-limited per source and server-wide | WP-032, WP-064, WP-130 |
| SEC-IAM-102 | Server: enforce documented limits | WP-087, WP-104, WP-130 |
| SEC-IAM-103 | Disabling an account: end its sessions at once | WP-094, WP-133 |
| SEC-IAM-104 | User: able to see a page stating what administrators | WP-087 |
| SEC-IAM-105 | Backups that contain identity data: encrypted before they leave the host | WP-090 |
| SEC-IAM-106 | Credential enrolled through a recovery code: start a recovery hold | WP-106, WP-109 |
| SEC-IAM-107 | Owner-only and fresh-uv actions: satisfied only by a passkey or device key enrolled | WP-062, WP-106 |
| SEC-IAM-108 | Person whose browser cannot use: able to sign it in by approval | WP-120 |
| SEC-API-001 | HTTP and WebSocket route: registered through one typed route table | WP-044 |
| SEC-API-002 | Set of routes whose class: exactly equal a checked-in allow-list file | WP-118, WP-131 |
| SEC-API-003 | Route: answer 401 with a byte-identical body | WP-118, WP-131 |
| SEC-API-004 | Session tokens: accepted only from the session cookie | WP-044, WP-131 |
| SEC-API-005 | Unauthenticated responses: never reveal the software version | WP-089, WP-131 |
| SEC-API-007 | Server: reject with 421 any request whose Host | WP-044 |
| SEC-API-008 | Only the methods declared: used | WP-044, WP-131 |
| SEC-API-009 | HTTP/1.1 requests with both Content-Length: rejected | WP-118 |
| SEC-API-010 | Handlers: obtain stored objects only through an authorisation layer | WP-046, WP-064, WP-065, WP-068, WP-069 |
| SEC-API-011 | For every route that accepts: never see the object must receive the same 404 status | WP-065, WP-131 |
| SEC-API-012 | Request that carries several object identifiers: authorise every identifier and must reject the whole request | WP-065, WP-093 |
| SEC-API-013 | Acting principal: come only from the authenticated credential | WP-044, WP-131 |
| SEC-API-014 | Library visibility: applied inside the authorisation layer to every read path | WP-065, WP-084 |
| SEC-API-015 | Library sync deltas: computed per principal | WP-084 |
| SEC-API-016 | Server-pushed event: filtered per recipient through the same policy layer | WP-065, WP-083 |
| SEC-API-017 | Change to anything authorisation depends: take effect on the next HTTP request | WP-062, WP-083 |
| SEC-API-018 | Byte-serving path: open only the file path recorded at scan time | WP-060, WP-082 |
| SEC-API-019 | Route: declare the capability it requires | WP-044, WP-131 |
| SEC-API-020 | Credential whose scope is narrower: evaluated as the intersection of its scope | WP-033 |
| SEC-API-022 | Folder browser used to choose library: admin-only | WP-099 |
| SEC-API-023 | Identifier visible outside the server: carry at least 128 bits from a CSPRNG | WP-006, WP-046, WP-047, WP-102 |
| SEC-API-024 | Identifiers: typed by kind | WP-006, WP-131 |
| SEC-API-025 | Pagination cursors and other continuation tokens: either opaque server-side handles or MAC-protected | WP-044, WP-066 |
| SEC-API-026 | Audio: served only from capability URLs whose path | WP-031, WP-082, WP-105 |
| SEC-API-027 | Capability URL lifetimes | WP-031, WP-082, WP-103 |
| SEC-API-028 | After the MAC and expiry checks: confirm that the bound session is still active | WP-082, WP-105 |
| SEC-API-029 | Media routes: authenticate only by the URL capability | WP-082 |
| SEC-API-030 | Media URL keys: at least 256 bits from a CSPRNG | WP-047 |
| SEC-API-031 | Server: answer a Range header that names more | WP-023, WP-082 |
| SEC-API-032 | Web client served: authenticate with a single cookie named __Host-gm_session set | WP-062 |
| SEC-API-033 | Cookie-authenticated API request: carry the header Gunmetal-Request | WP-044 |
| SEC-API-034 | Cookie-authenticated request whose Sec-Fetch-Site is cross-site: refused with 403 | WP-044 |
| SEC-API-035 | Routes that take a body: accept only Content-Type | WP-044 |
| SEC-API-036 | GET and HEAD routes: never change application state other than last-seen timestamps and audit | WP-044 |
| SEC-API-038 | HTTPS responses served under a hostname: carry Strict-Transport-Security with max-age of at least one year | WP-044, WP-073 |
| SEC-API-039 | Signing out: invalidate the session on the server | WP-120 |
| SEC-API-040 | API: send no CORS headers by default | WP-044 |
| SEC-API-041 | WebSocket upgrade: require a valid credential | WP-083 |
| SEC-API-042 | Client that cannot send a cookie: authenticate by sending | WP-083 |
| SEC-API-043 | WebSocket message: at most 64 KiB | WP-083 |
| SEC-API-044 | HTML response: carry a Content Security Policy equivalent to default-src 'none' | WP-072 |
| SEC-API-045 | Web client's build: fail on any HTML-string or code-string sink | Unassigned: web-client behaviour. The client is outside this backend plan, and no client plan exists yet; it must carry this requirement. |
| SEC-API-046 | Text that comes from files: rendered as text | Unassigned: web-client behaviour. The client is outside this backend plan, and no client plan exists yet; it must carry this requirement. |
| SEC-API-047 | URL taken from metadata or user: rendered as a link or opened by the app | WP-005, WP-235 |
| SEC-API-048 | Untrusted strings: normalised on ingest | WP-005 |
| SEC-API-049 | Web client: load no third-party scripts | WP-072 (server part: the CSP that blocks other origins); the client part is unassigned: web-client behaviour that a client plan must carry (the all-screens single-origin recording). |
| SEC-API-050 | Message event listener in the web: check event.origin against an exact list and validate | Unassigned: web-client behaviour. The client is outside this backend plan, and no client plan exists yet; it must carry this requirement. |
| SEC-API-051 | Content that comes from users: never served from the application origin with a script-capable type | WP-082, WP-103 |
| SEC-API-052 | Web client: check at start-up for the features it depends | Unassigned: web-client behaviour. The client is outside this backend plan, and no client plan exists yet; it must carry this requirement. |
| SEC-API-053 | Response: carry X-Content-Type-Options | WP-044, WP-131 |
| SEC-API-054 | Response with a body: declare a Content-Type that matches the body | WP-044 |
| SEC-API-055 | Authenticated JSON responses: carry Cache-Control | WP-044, WP-131 |
| SEC-API-056 | Lockout applies only to guessable secrets | WP-032, WP-064 |
| SEC-API-057 | Rate limits: keyed on the principal when there | WP-032, WP-130 |
| SEC-API-058 | Sign-in: indistinguishable in status and body whether the account | WP-064, WP-081, WP-094, WP-120 |
| SEC-API-060 | Server: enforce | WP-044, WP-118 |
| SEC-API-061 | Server: time out header reading after 10 seconds | WP-118 |
| SEC-API-062 | If HTTP/2 is enabled: cap concurrent streams per connection at 100 | WP-118 |
| SEC-API-063 | List and search routes: cap page size at 500 unless the route declares | WP-044, WP-054 |
| SEC-API-064 | Expensive operations: limited to authorised principals | WP-070, WP-098, WP-130 |
| SEC-API-065 | Request bodies with any Content-Encoding: refused with 415 | WP-044 |
| SEC-API-066 | SQL text: static | WP-042, WP-046, WP-067, WP-126 |
| SEC-API-067 | Body: decode into a typed request structure that rejects unknown | WP-044 |
| SEC-API-068 | Responses: built from explicit per-role response types | WP-044, WP-131 |
| SEC-API-069 | Absolute URLs the server generates: built from the configured public URL | WP-073 |
| SEC-API-070 | Post-sign-in return target: a relative path that starts with a single / | WP-005 |
| SEC-API-071 | CSV export: quote fields as RFC 4180 describes and must neutralise | WP-108 |
| SEC-API-072 | Error: an RFC 9457 problem-details object whose type comes | WP-006, WP-044 |
| SEC-API-073 | Last-resort layer: turn any panic or unexpected error in request handling | WP-044 |
| SEC-API-075 | Authentication or authorisation decision: never grant access | WP-033, WP-132 |
| SEC-API-076 | Outbound network request made: go through a single egress crate | WP-001, WP-048 |
| SEC-API-077 | Egress client: resolve names itself and refuse the request | WP-005, WP-048 |
| SEC-API-078 | Egress client: allow only https | WP-048 |
| SEC-API-079 | Outbound purpose: declare its allowed hosts | WP-048 |
| SEC-API-080 | Server: never fetch a URL that a user | WP-102 |
| SEC-API-081 | Responses from external services: decoded into typed structures with size limits | WP-074 |
| SEC-API-085 | Upload route: declare its allowed types | WP-109 |
| SEC-API-086 | Images: decoded only by memory-safe Rust decoders with width | WP-079 |
| SEC-API-087 | Stored uploads and extracted artwork: named by a server-generated content hash inside a server-controlled | WP-103 |
| SEC-API-088 | Uploads: limited per principal by count and total bytes | WP-130 |
| SEC-API-090 | Lyrics from tags: parsed into a timed-line model capped at 256 KiB | WP-021 |
| SEC-API-091 | OpenAPI description: generated from the route table at build time | WP-118 |
| SEC-API-092 | Native API: versioned in its path | WP-044, WP-131 |
| SEC-API-095 | Access log: record method | WP-044 |
| SEC-API-096 | Invitation link: carry a secret of at least 128 bits | WP-094 |
| SEC-NET-001 | Over plaintext HTTP: give every peer other than loopback only a static | WP-132 |
| SEC-NET-002 | TLS listener: negotiate only TLS 1.3 | WP-073 |
| SEC-NET-003 | Server: present the complete certificate chain | WP-101 |
| SEC-NET-004 | Certificate renewal: automatic | WP-101 |
| SEC-NET-005 | When no valid certificate is available: never serve the web client | WP-101, WP-132 |
| SEC-NET-006 | TLS private keys: generated on the server from the operating system's CSPRNG | WP-101 |
| SEC-NET-009 | Outbound TLS: validate certificates against the WebPKI with hostname checks | WP-048 |
| SEC-NET-013 | Browser HTTPS: also work without the project name service | WP-073, WP-101 |
| SEC-NET-014 | Server: answer 421 Misdirected Request to any request whose Host | WP-044 |
| SEC-NET-015 | Absolute URL the server emits: built from the configured canonical origin for the path | WP-073 |
| SEC-NET-016 | Server: ignore Forwarded | WP-023, WP-118 |
| SEC-NET-017 | When a peer: ignore the headers | WP-132 |
| SEC-NET-018 | For requests from trusted proxies: the first address that is not a trusted proxy | WP-023 |
| SEC-NET-019 | Owner: declare each trusted proxy as "private overlay" or "public" | WP-132 |
| SEC-NET-020 | WebSocket upgrades: refused unless Origin | WP-083 |
| SEC-NET-021 | HTTP/1.1 front end: reject ambiguous framing | WP-118 |
| SEC-NET-022 | Project: ship reverse-proxy configurations for Caddy | WP-073 |
| SEC-NET-023 | Server: never treat Tailscale identity headers | WP-132 |
| SEC-NET-024 | In the default home posture: serve only a static help page | WP-132 |
| SEC-NET-025 | Address classification: convert IPv4-mapped IPv6 addresses to IPv4 before classifying them | WP-005 |
| SEC-NET-027 | When a request from a non-local: record a security event | WP-132 |
| SEC-NET-028 | At startup and whenever interfaces change: show the admin which listeners are bound to globally | WP-132 |
| SEC-NET-030 | Server: never request router port mappings | WP-001, WP-132 |
| SEC-NET-031 | In each documented configuration: equal the documented list exactly | WP-132 |
| SEC-NET-032 | In each configuration the server: connect out only to the destinations in the egress | WP-117 |
| SEC-NET-036 | Invite and pairing secrets: travel only in URL fragments | WP-094, WP-120 |
| SEC-NET-045 | Admin operations: refused on internet-posture paths | WP-132 |
| SEC-NET-046 | Metrics: off by default | WP-089 |
| SEC-NET-047 | Unauthenticated responses: never reveal the server version | WP-089, WP-131 |
| SEC-NET-048 | HTTP front end: enforce a global connection cap | WP-118 |
| SEC-NET-049 | HTTP/2: allow at most 100 concurrent streams per connection | WP-118 |
| SEC-NET-050 | Byte-range request: name at most one range | WP-023 |
| SEC-NET-051 | State created for unauthenticated clients: live in fixed-capacity stores with eviction | WP-032, WP-130 |
| SEC-NET-052 | Rate-limit and connection-cap keys: the IPv4 address or hierarchical IPv6 prefixes | WP-032, WP-130 |
| SEC-NET-053 | Expensive authenticated operations: run under per-user and global concurrency limits with queueing | WP-130 |
| SEC-NET-054 | Iroh endpoint: cap total connections | WP-001 (R1 absence proof: `deny.toml` bans iroh, so no endpoint exists); WP-221 verifies the caps in R2. The row's R1 release contradicts SEC-TM-074 and the release scope; owner decision 37. |
| SEC-NET-055 | On the reference low-end profile: play without client buffer underrun while the server receives | WP-117 |
| SEC-NET-056 | Server: log network security events as structured, injection-safe records | WP-132 |
| SEC-NET-057 | Project: keep a cryptographic inventory listing every key | WP-125, WP-136 |
| SEC-NET-058 | Web client used on the LAN: served by the Gunmetal server from the same origin | WP-072 |
| SEC-NET-059 | Core server: never implement or open SSDP | WP-001, WP-132 |
| SEC-NET-068 | Peer whose address equals the default: classified "unknown" and treated as non-local | WP-118, WP-132 |
| SEC-NET-072 | Server: alert the owner 30 and 7 days | WP-101, WP-109 |
| SEC-CLI-001 | Web and native clients: render every string that originates from media files | Unassigned: web-client behaviour. The client is outside this backend plan, and no client plan exists yet; it must carry this requirement. |
| SEC-CLI-002 | URL taken from metadata or user: rendered as a link only if it parses | WP-005, WP-235 |
| SEC-CLI-004 | Server response to clients: carry X-Content-Type-Options | WP-044, WP-131 |
| SEC-CLI-005 | Artwork and other user-derived content: reach clients only as an allow-listed raster type | WP-079, WP-103 |
| SEC-CLI-007 | Server: reject any state-changing request from a browser whose Sec-Fetch-Site | WP-044, WP-131 |
| SEC-CLI-009 | On sign-out: delete that account's data from IndexedDB | WP-120 (server part: the `Clear-Site-Data` header on sign-out); the client part is unassigned: web-client behaviour that a client plan must carry (deleting every store and unregistering the service worker). |
| SEC-CLI-010 | At sign-in the web client: ask | WP-062 (server part: the cookie with no `Max-Age` or `Expires` and the 30-minute idle timeout); the client part is unassigned: web-client behaviour that a client plan must carry (the personal-or-shared question and in-memory storage). |
| SEC-CLI-011 | Server: reject API calls from a web client bundle | WP-072 |
| SEC-CLI-012 | Web client's scripts: ship inside the signed server release and be served | WP-072 |
| SEC-CLI-013 | Secret carried by a link: travel in the URL fragment | Unassigned: web-client behaviour. The client is outside this backend plan, and no client plan exists yet; it must carry this requirement. |
| SEC-CLI-015 | Restriction a client applies or displays: also be enforced by the server on every request | WP-131 |
| SEC-CLI-016 | Client build: never contain an API key | WP-136 |
| SEC-CLI-017 | Client build: produce a CycloneDX SBOM that includes native libraries | WP-136 |
| SEC-CLI-018 | CI: install JavaScript dependencies only from the committed lockfile | WP-136 (the release workflow's install); the same rule in the client's own CI is for the client plan. |
| SEC-CLI-019 | Development servers: bind to loopback in the committed configuration | Unassigned: web-client behaviour. The client is outside this backend plan, and no client plan exists yet; it must carry this requirement. |
| SEC-CLI-020 | Library data the server sends: contain only what the signed-in profile may access | WP-084 |
| SEC-CLI-021 | Clients: decode every server response with schema-validating decoders that bound | WP-039, WP-088, WP-235, WP-236, WP-237 |
| SEC-CLI-024 | Server: assign every enrolled device a class | WP-062, WP-120 |
| SEC-CLI-025 | Inbound links and codes: parsed by one pure function in the core | WP-239 (the parser), WP-089 (resolution) |
| SEC-CLI-027 | Client: never include analytics | Unassigned: web-client behaviour. The client is outside this backend plan, and no client plan exists yet; it must carry this requirement. |
| SEC-CLI-028 | PIN: mask input and turn off autocorrect | Unassigned: web-client behaviour. The client is outside this backend plan, and no client plan exists yet; it must carry this requirement. |
| SEC-MED-001 | Public parsing function in gunmetal-core: return Ok or a typed error for every possible | WP-004, WP-009, WP-010, WP-011, WP-012, WP-013, WP-014, WP-015, WP-016, WP-017, WP-018, WP-019, WP-020, WP-021, WP-023, WP-035, WP-039, WP-041, WP-052, WP-056, WP-128 |
| SEC-MED-002 | Non-test code in gunmetal-core: compile with the Clippy lints unwrap_used | WP-001 |
| SEC-MED-003 | Gunmetal-core: never size any allocation from a declared length or count | WP-001, WP-004 |
| SEC-MED-004 | Offset: combined with checked arithmetic and converted with fallible conversions | WP-001, WP-004 |
| SEC-MED-005 | Core: count nesting depth for every nested structure and return | WP-004, WP-009, WP-010, WP-011, WP-012, WP-013, WP-014, WP-015, WP-016, WP-017, WP-018, WP-019, WP-020, WP-021, WP-023, WP-035, WP-039, WP-041, WP-052 |
| SEC-MED-006 | Core: enforce the element-count | WP-004, WP-009, WP-010, WP-011, WP-012, WP-013, WP-014, WP-015, WP-016, WP-017, WP-018, WP-019, WP-020, WP-021, WP-023, WP-035, WP-039, WP-041, WP-049, WP-050, WP-051, WP-052, WP-053 |
| SEC-MED-007 | Core parse: charge a deterministic step budget and fail | WP-004, WP-009, WP-010, WP-011, WP-012, WP-013, WP-014, WP-015, WP-016, WP-017, WP-018, WP-019, WP-020, WP-021, WP-023, WP-035, WP-039, WP-041, WP-052, WP-056 |
| SEC-MED-008 | Loop over elements: strictly advance or stop | WP-004, WP-009, WP-010, WP-011, WP-012, WP-013, WP-014, WP-015, WP-016, WP-017, WP-018, WP-019, WP-020, WP-021, WP-023, WP-035, WP-039, WP-041, WP-052 |
| SEC-MED-009 | Decompression: go through one streaming helper | WP-128 |
| SEC-MED-010 | Core's sans-I/O interface: never request a read longer than 16 MiB or past | WP-004, WP-052, WP-061 |
| SEC-MED-011 | Formats: detected from content signatures against a closed allowlist | WP-009 |
| SEC-MED-012 | Server: serve bytes only for items that content detection indexed | WP-082 |
| SEC-MED-013 | Text from media metadata: decoded with invalid sequences replaced | WP-005 |
| SEC-MED-014 | Identifiers: validated into typed values with documented ranges when parsed | WP-005, WP-049, WP-050, WP-051 |
| SEC-MED-015 | Player: clamp gain taken from tags to the range −30 | WP-028 |
| SEC-MED-016 | Server: never fetch | WP-102 |
| SEC-MED-017 | When a limit or parse error: keep the rest of the file's metadata | WP-052, WP-075, WP-102 |
| SEC-MED-018 | Parsing of media: run in separate worker processes | WP-003, WP-045, WP-056, WP-061, WP-078, WP-105, WP-114 |
| SEC-MED-019 | File that crashes or times out: quarantined until its size or modification time changes | WP-078, WP-110 |
| SEC-MED-020 | Worker: receive input only as read-only file descriptors | WP-061, WP-105 |
| SEC-MED-021 | Worker: single-threaded and must run | WP-045, WP-078, WP-114 |
| SEC-MED-022 | On Linux the worker: in order | WP-045 |
| SEC-MED-023 | Server: treat messages from workers and sandboxes as untrusted | WP-039, WP-056, WP-061, WP-105 |
| SEC-MED-024 | At startup and on demand: self-test each sandbox profile | WP-003, WP-045, WP-056, WP-105, WP-110, WP-116 |
| SEC-MED-025 | Server and worker binaries: never link C or C++ media | WP-001 |
| SEC-MED-026 | Third-party crate that parses or decodes: admitted only after a recorded review | WP-003, WP-079, WP-114 |
| SEC-MED-027 | Public parsing entry point in gunmetal-core: a fuzz harness whose body is a plain function | WP-008 |
| SEC-MED-028 | Cargo test on stable: replay the committed corpus of every harness | WP-008 |
| SEC-MED-029 | Coverage-guided fuzzing: run on every pull request that changes gunmetal-core | WP-008 |
| SEC-MED-030 | Fuzzing finding: fixed test-first | WP-008 |
| SEC-MED-031 | Container format: also be fuzzed by a structure-aware generator that emits | WP-008, WP-010, WP-012, WP-015, WP-017 |
| SEC-MED-033 | Library access: go through directory handles opened once per configured root | WP-001, WP-060, WP-126 |
| SEC-MED-034 | Symlink: followed only when its whole chain resolves | WP-024, WP-060 |
| SEC-MED-035 | Server: open files with O_NONBLOCK | WP-060 |
| SEC-MED-036 | Before serving bytes: open the item beneath its root and confirm | WP-060, WP-082 |
| SEC-MED-037 | Server: refuse a library root that is a filesystem root | WP-024, WP-099 |
| SEC-MED-038 | Server: open library content read-only and must not create | WP-060 |
| SEC-MED-039 | Names from media: never used as path components | WP-024 |
| SEC-MED-040 | Server: keep file names as raw bytes for access | WP-024, WP-060 |
| SEC-MED-041 | Blocking filesystem calls: run on a bounded pool with per-root concurrency limits | WP-060, WP-078 |
| SEC-MED-042 | Official container image and example compose: run as a non-root user | WP-121 |
| SEC-MED-044 | Artwork decoding: accept only JPEG | WP-079 |
| SEC-MED-045 | Image width and height: read from the header and checked before any pixel | WP-079 |
| SEC-MED-046 | Clients: receive only derivatives the server generated | WP-079, WP-103 |
| SEC-MED-047 | Image endpoints: accept a size only from a fixed enumeration | WP-103 |
| SEC-MED-048 | Derivative cache: bounded in total bytes | WP-103 |
| SEC-MED-049 | LRC: parsed into a typed model within the limits table | WP-021 |
| SEC-MED-051 | Playlist entries: returned only for items in libraries the requesting user | WP-065 |
| SEC-MED-057 | Clients: render media-derived strings only as text nodes inside | Unassigned: web-client behaviour. The client is outside this backend plan, and no client plan exists yet; it must carry this requirement. |
| SEC-MED-058 | URL from metadata: shown as a link only after it has been | Unassigned: web-client behaviour. The client is outside this backend plan, and no client plan exists yet; it must carry this requirement. |
| SEC-MED-059 | Byte-serving responses: carry | WP-082 |
| SEC-MED-060 | Range parser: accept at most one byte range per request | WP-023 |
| SEC-MED-062 | Media-derived strings and paths written: escaped | WP-043 |
| SEC-MED-063 | Server: never start any external program except through the sandbox launcher | WP-001, WP-045 |
| SEC-MED-077 | Clients: apply the core's parsers and limits to everything | WP-039, WP-088, WP-235, WP-236, WP-237 |
| SEC-EXT-001 | Server-initiated outbound network connection: go through the single egress client | WP-001, WP-048 |
| SEC-EXT-002 | Egress client: resolve names itself | WP-048 |
| SEC-EXT-003 | Egress client: never follow redirects unless the caller opts | WP-048 |
| SEC-EXT-004 | Egress client: enforce a connect timeout | WP-048 |
| SEC-EXT-005 | Outbound requests: carry no identifier of the server | WP-048 |
| SEC-EXT-006 | Native API: authenticate requests only from the Authorization header | WP-044, WP-131 |
| SEC-EXT-007 | Credential: carry an immutable kind | WP-062, WP-064 |
| SEC-EXT-018 | Until SEC-EXT-019 to SEC-EXT-034 are implemented: never load or execute any plugin or other third-party code | WP-001 |
| SEC-OPS-001 | Server: never ship | WP-080 |
| SEC-OPS-003 | While unclaimed: answer only the setup page and its assets | WP-080 |
| SEC-OPS-004 | Setup code attempts: compared in constant time and limited as SEC-IAM-008 requires | WP-080 |
| SEC-OPS-005 | Claim: a single atomic transaction | WP-080 |
| SEC-OPS-006 | After the claim: never become reachable again | WP-080 |
| SEC-OPS-007 | Before it is claimed: make no outbound connection | WP-101, WP-117 |
| SEC-OPS-008 | Restoring a backup onto an unclaimed: require the same setup code as claiming | WP-109 |
| SEC-OPS-009 | Owner recovery: possible only through a command on the host | WP-106 |
| SEC-OPS-011 | Server key and secret: come from the OS CSPRNG | WP-047 |
| SEC-OPS-012 | Secrets: live as files with mode 0600 | WP-047, WP-126 |
| SEC-OPS-013 | Secret values: held in a type with no printable | WP-047 |
| SEC-OPS-014 | Server: never accept secrets as command-line arguments | WP-043, WP-045 |
| SEC-OPS-015 | Cryptographic purpose: use its own key | WP-047 |
| SEC-OPS-016 | Session tokens: stored only as keyed hashes | WP-062 |
| SEC-OPS-017 | Secrets the server: replay to other systems | WP-047 |
| SEC-OPS-018 | Owner: able to rotate every server secret in one action | WP-106 |
| SEC-OPS-019 | Trust root for the project's update: compiled into the binary and replaced only through signed | WP-074 |
| SEC-OPS-020 | Server: keep a security audit log | WP-069, WP-131 |
| SEC-OPS-021 | Audit record: carry a sequence number | WP-069 |
| SEC-OPS-022 | Log output: one JSON object per line with every value escaped | WP-043 |
| SEC-OPS-023 | Audit log: tamper-evident | WP-069 |
| SEC-OPS-024 | Backup: include the latest signed audit checkpoint | WP-090, WP-109 |
| SEC-OPS-026 | Retention: enforced automatically on the schedule of SEC-PRV-005 | WP-069, WP-133 |
| SEC-OPS-027 | Only the owner and holders: read the full audit log | WP-069, WP-100, WP-117 |
| SEC-OPS-028 | Failed authentication on any surface: also be written to the diagnostic log | WP-064 |
| SEC-OPS-029 | Diagnostic logging: default to info level and must never record request | WP-043, WP-069, WP-117 |
| SEC-OPS-031 | At startup the server: detect any configuration change made outside | WP-097 |
| SEC-OPS-032 | Server: raise owner alerts | WP-097 |
| SEC-OPS-033 | Alert about a device or credential: offer a one-step "This wasn't me" action that revokes | WP-097, WP-117 |
| SEC-OPS-034 | Alerts for a new admin: never switchable off and must never be suppressed | WP-097 |
| SEC-OPS-037 | Exposure detection: all take the client address and its class | WP-001, WP-069, WP-117, WP-118 |
| SEC-OPS-038 | Home posture: the default in every official package and container image | WP-121, WP-132 |
| SEC-OPS-039 | Server: never ask a router for port mappings | WP-001, WP-132 |
| SEC-OPS-040 | Iroh endpoint: keep router port mapping off unless the owner has | WP-001 (R1 absence proof: `deny.toml` bans iroh and its port-mapper crates); WP-221 verifies the endpoint setting in R2. The row's R1 release contradicts SEC-TM-074 and the release scope; owner decision 37. |
| SEC-OPS-041 | Backups: run by default | WP-090 |
| SEC-OPS-042 | Backup: encrypted in age v1 to the server's backup key | WP-090 |
| SEC-OPS-043 | Backups: signed with a server backup-signing key | WP-090, WP-109 |
| SEC-OPS-044 | After any restore the server: rotate all symmetric keys and invalidate every session | WP-109 |
| SEC-OPS-045 | Downloading a backup: require the owner role and a re-authentication | WP-090 |
| SEC-OPS-046 | Server: never replace or modify its own executable or install directory | WP-121 |
| SEC-OPS-047 | Update and advisory check: a required first-run question with two explicit answers | WP-074, WP-080 |
| SEC-OPS-048 | Before any schema or format migration: take a snapshot and check it with PRAGMA integrity_check | WP-046, WP-095 |
| SEC-OPS-049 | Migration: never make an existing installation less strict | WP-046 |
| SEC-OPS-050 | Startup: never reveal the version | WP-080, WP-095, WP-131 |
| SEC-OPS-051 | Older binary: refuse to open durable state written in a newer | WP-046 |
| SEC-OPS-052 | Release's notes: say whether it migrates data | WP-136 |
| SEC-OPS-053 | On Unix-like systems the server: refuse to start when its effective user is root | WP-043, WP-121 |
| SEC-OPS-054 | Server: open media roots read-only and never create | WP-060, WP-116 |
| SEC-OPS-055 | Media-serving path: open a file only when its fully resolved location | WP-060, WP-082 |
| SEC-OPS-056 | Official systemd unit: run under a dedicated system account with no login | WP-121 |
| SEC-OPS-057 | Official container image: run as a fixed non-root user and work | WP-121 |
| SEC-OPS-059 | Metrics and diagnostics endpoints: off by default | WP-089 |
| SEC-OPS-060 | Server: document every outbound connection it can make | WP-048, WP-117 |
| SEC-OPS-061 | Gunmetal doctor --security and the dashboard: report root and capability state | WP-116 |
| SEC-OPS-064 | Besides GitHub private vulnerability reporting: accept reports at an email alias that reaches | WP-124 |
| SEC-OPS-065 | Report: tracked against the response times in SECURITY.md | WP-124 |
| SEC-OPS-066 | Security fix: begin with a failing regression test that reproduces | WP-124 |
| SEC-OPS-067 | Advisory: also give affected and fixed ranges in OSV form | WP-124 |
| SEC-OPS-068 | Advisories: also go into the signed feed with their severity | WP-074, WP-136 |
| SEC-OPS-069 | Project: notify the packagers of official and known community packages | WP-124 |
| SEC-OPS-070 | Project: publish a supported-version policy | WP-074, WP-124 |
| SEC-OPS-071 | Signing-key compromise playbook: rehearsed at least once a year | WP-136 |
| SEC-OPS-072 | Project: publish a compromise runbook for server owners | WP-124 |
| SEC-OPS-075 | Audit log: an anchor off the host | WP-069, WP-090, WP-100 (server part: the checkpoint head, its route and its export); WP-241 (the extension check); keeping the head and running the check at sign-in is the client plan's CP-052. |
| SEC-PRV-001 | Server: keep a machine-readable data inventory that assigns every persisted | WP-122 |
| SEC-PRV-002 | History event: contain only the profile ID | WP-034, WP-240 |
| SEC-PRV-003 | Client IP addresses: stored only in the active-session table and the security | WP-062, WP-069, WP-133 |
| SEC-PRV-004 | Server: never persist users' search queries | WP-117 |
| SEC-PRV-005 | Retention for every data class: defined in one schedule in code with the defaults | WP-069, WP-133, WP-138 |
| SEC-PRV-007 | With the default configuration: cause no outbound connection or non-local DNS lookup | WP-117 |
| SEC-PRV-008 | Outbound requests from the server: go through one egress component that enforces a per-feature | WP-048, WP-069, WP-117 |
| SEC-PRV-009 | Server and every first-party client: never send telemetry | WP-001, WP-117 |
| SEC-PRV-012 | Admin: able to route all server egress through an HTTP | WP-048 |
| SEC-PRV-013 | Metadata: never enabled by default | WP-048, WP-080 |
| SEC-PRV-016 | Clients: fetch artwork | WP-084, WP-103 |
| SEC-PRV-018 | Web response: send Referrer-Policy | WP-044, WP-131 |
| SEC-PRV-019 | Web client: never keep Activity | Unassigned: web-client behaviour. The client is outside this backend plan, and no client plan exists yet; it must carry this requirement. |
| SEC-PRV-020 | Responses carrying Activity: send Cache-Control | WP-044, WP-131 |
| SEC-PRV-021 | IDs and signed URLs exposed: opaque random values of at least 128 bits | WP-006, WP-046, WP-047, WP-102 |
| SEC-PRV-022 | One user's Activity data: never returned to any other user | WP-086, WP-131 |
| SEC-PRV-023 | Privacy setting: default to its most private value | WP-046, WP-087 |
| SEC-PRV-024 | Client: let a user start a private session | WP-086, WP-240 (the device's half: the sink drops private events) |
| SEC-PRV-025 | Admin interface and admin API: never offer any view | WP-104 |
| SEC-PRV-026 | Product: never offer any way for an admin to sign | WP-094, WP-131 |
| SEC-PRV-027 | User: able to read a "what your admin can see" | WP-087 |
| SEC-PRV-030 | Emails and in-app notifications: never contain another user's Activity data | WP-097 |
| SEC-PRV-031 | Share-link pages and any other page: never reveal the sharer's username | WP-094 |
| SEC-PRV-033 | Scrobbling and every other feature: off for every user by default | WP-087 |
| SEC-PRV-034 | Linking an external scrobbling account: bind the callback to the initiating user's session | WP-048, WP-131 (R1 absence proof: no scrobbling purpose and no external-account linking route); WP-226 verifies the behaviour in R2. Owner decision 37. |
| SEC-PRV-035 | Scrobbling: submit only plays recorded after the link time | WP-048, WP-131 (R1 absence proof: no scrobbling purpose and no scrobbling route); WP-226 verifies the behaviour in R2. Owner decision 37. |
| SEC-PRV-036 | Unlinking an external service: delete its stored credential and discard queued | WP-048, WP-131 (R1 absence proof: no external service can be linked, so none can be unlinked); WP-226 verifies the behaviour in R2. Owner decision 37. |
| SEC-PRV-038 | Secret-store key: generated on first start from a CSPRNG | WP-047 |
| SEC-PRV-039 | Backup archive: encrypted in the age v1 format | WP-090 |
| SEC-PRV-040 | Setup: issue one printable Gunmetal recovery kit holding | WP-080, WP-090 |
| SEC-PRV-041 | Backups: kept for a configurable period | WP-090 |
| SEC-PRV-042 | Logs: never contain passwords | WP-043, WP-047 |
| SEC-PRV-043 | At the default log level: never contain media titles | WP-043 |
| SEC-PRV-044 | Repository: contain a log inventory listing every log event type | WP-043 |
| SEC-PRV-045 | Log files: created readable only by the service account | WP-097, WP-126 |
| SEC-PRV-047 | User: able to export all of their own data | WP-108 |
| SEC-PRV-048 | Starting an export or an account: require authentication within the last 5 minutes | WP-108, WP-133 |
| SEC-PRV-049 | Users: able to delete one history entry | WP-133 |
| SEC-PRV-050 | Database connection: set PRAGMA secure_delete=ON | WP-042, WP-046, WP-068, WP-126, WP-133 |
| SEC-PRV-051 | Deleting an account: disable it and end all its sessions and device | WP-133 |
| SEC-PRV-052 | Tombstones that carry deletions to devices: identify erased events only by ID or ID range | WP-133 |
| SEC-PRV-053 | Before redeeming an invitation: shown a privacy notice generated from the server's actual | WP-094 |
| SEC-PRV-054 | Service the project operates: publish a privacy notice | WP-139 |
| SEC-PRV-055 | Invitation and share secrets carried through: travel only in the URL fragment so they never | WP-139 |
| SEC-SUP-001 | Account that can write: use phishing-resistant MFA | WP-124 |
| SEC-SUP-002 | Default branch: protected by a ruleset that blocks direct pushes | WP-124 |
| SEC-SUP-003 | Release tags: creatable only by maintainers | WP-124 |
| SEC-SUP-004 | Commits on the default branch: carry a verified signature | WP-124 |
| SEC-SUP-005 | Changes to .github/: require approval from a code owner | WP-124 |
| SEC-SUP-006 | Secret-scanning push protection: enabled for the repository | WP-124 |
| SEC-SUP-007 | Private vulnerability reporting: stay enabled | WP-124 |
| SEC-SUP-008 | Project site: serve /.well-known/security.txt over HTTPS with Contact | WP-139 |
| SEC-SUP-009 | Fixed vulnerability in Gunmetal: published as a GitHub Security Advisory with a CVE | WP-124, WP-136 |
| SEC-SUP-010 | Uses: pinned to a full commit SHA | WP-124 |
| SEC-SUP-011 | Tools installed in CI: pinned to exact versions and installed with checksum verification | WP-124 |
| SEC-SUP-012 | Workflow: set permissions | WP-124 |
| SEC-SUP-013 | Workflow: never check out or execute pull-request code under pull_request_target | WP-124 |
| SEC-SUP-014 | Untrusted event text: never expanded with ${{ }} inside run | WP-124 |
| SEC-SUP-015 | Release and publishing workflows: never restore or save GitHub Actions caches | WP-124, WP-136 |
| SEC-SUP-016 | Release workflows: authenticate only with short-lived credentials | WP-136 |
| SEC-SUP-017 | Publishing jobs: run in a protected release environment that accepts | WP-136 |
| SEC-SUP-018 | Zizmor: run on every pull request and block merging | WP-124 |
| SEC-SUP-019 | OpenSSF Scorecard: run weekly and on every push to main | WP-124 |
| SEC-SUP-020 | Cargo.lock: committed | WP-001 |
| SEC-SUP-021 | Cargo deny check: pass on every pull request and daily on main | WP-001 |
| SEC-SUP-022 | Ignored advisory: an entry in supply-chain/exceptions.toml giving the advisory ID | WP-001 |
| SEC-SUP-023 | Vulnerable dependencies: remediated within documented times | WP-001 |
| SEC-SUP-024 | Crate in the dependency graph: covered in cargo-vet by an audit | WP-001 |
| SEC-SUP-025 | Gunmetal-core: no normal dependencies except those in a reviewed allowlist | WP-001, WP-008 |
| SEC-SUP-026 | Only crates on a reviewed allowlist: build scripts | WP-001 |
| SEC-SUP-027 | Pull request that adds or changes: fail unless it carries an override label approved | WP-008 |
| SEC-SUP-028 | Dependabot: cover the cargo | WP-124 |
| SEC-SUP-029 | Rust and JavaScript component: use a licence on the project allowlist of AGPL-3.0-or-later-compatible | WP-001, WP-136 |
| SEC-SUP-030 | File in the repository: carry machine-readable copyright and licence information under REUSE 3.3 | WP-124 |
| SEC-SUP-031 | Server build: embed the source repository URL and exact commit | WP-089 |
| SEC-SUP-032 | Repository: never contain executables | WP-124 |
| SEC-SUP-033 | JavaScript installs in CI and release: frozen to the committed lockfile | WP-136 (the release workflow's install of the web bundle's packages); the same rule in the client's own CI is for the client plan. |
| SEC-SUP-034 | JavaScript package manager: refuse versions published less than 7 days earlier | WP-136 (the release workflow's package-manager configuration); the same rule in the client's own CI is for the client plan. |
| SEC-SUP-035 | Direct JavaScript dependencies: listed in supply-chain/js-direct-deps.toml with a written reason | WP-124 (the list file and its xtask check, which fails for any `package.json` in the repository); the client plan adds its direct dependencies to the list. |
| SEC-SUP-036 | CI: verify registry signatures | WP-136 (the release workflow's install); the same rule in the client's own CI is for the client plan. |
| SEC-SUP-037 | Web client: never load scripts | WP-072 |
| SEC-SUP-038 | Rust toolchain: pinned to an exact version in rust-toolchain.toml | WP-001 |
| SEC-SUP-039 | Release compilation: run with no network access after the locked fetch | WP-136 |
| SEC-SUP-040 | Linux release binaries: reproducible | WP-136 |
| SEC-SUP-041 | Release artifact: SLSA Build Level 3 provenance from an isolated reusable | WP-136 |
| SEC-SUP-042 | Release: publish a SHA256SUMS manifest with a Sigstore bundle | WP-136 |
| SEC-SUP-043 | Container images: signed by digest | WP-136 |
| SEC-SUP-044 | Release: include CycloneDX 1.7 JSON SBOMs covering the Rust crates | WP-136 |
| SEC-SUP-045 | Official container image: built from distroless static or cc | WP-121 |
| SEC-SUP-046 | Image and every compose: work with all capabilities dropped | WP-121 |
| SEC-SUP-048 | Container images and their embedded dependency: scanned before publishing and daily afterwards | WP-136 |
| SEC-SUP-049 | Update notices and advisories: published as TUF 1.0 metadata on the project site | WP-074, WP-136 |
| SEC-SUP-050 | Server's update check: verify the feed from a root embedded at build | WP-074 |
| SEC-SUP-051 | Update check: a plain GET of static metadata files | WP-074 |
| SEC-SUP-052 | Project's names: reserved before announcement on each registry users might search | WP-124 |
| SEC-SUP-053 | Scheduled job: compare published tags | WP-136 |
| SEC-SUP-054 | Supply-chain incident runbook: cover a malicious dependency | WP-124 |
| SEC-SUP-055 | Coding agents and other automation working: run without access to release credentials | WP-124 |
| SEC-SUP-056 | Release: a unique version | WP-124, WP-136 |
| SEC-HIS-001 | Server: never base any authentication or authorization decision | WP-033, WP-132 |
| SEC-HIS-003 | When trusted proxies are configured: found by walking the forwarding chain from right | WP-023 |
| SEC-HIS-004 | Server: no passwordless sign-in for any network | WP-043, WP-064 |
| SEC-HIS-005 | HTTP and WebSocket route: declare an authorization policy when it is registered | WP-044 |
| SEC-HIS-006 | Endpoint that returns media-derived bytes: require a session credential or a signed stream token | WP-082, WP-103, WP-131 |
| SEC-HIS-007 | Endpoint: exist at exactly one route | WP-131 |
| SEC-HIS-009 | Handlers: take the acting user only from the authenticated session | WP-044, WP-131 |
| SEC-HIS-010 | Read or write of a user-owned: pass through one authorization function that checks | WP-065, WP-131 |
| SEC-HIS-011 | Response: never contain another account's credentials | WP-047 |
| SEC-HIS-012 | Object: come from a CSPRNG with at least 128 bits | WP-006, WP-046, WP-047, WP-062, WP-102 |
| SEC-HIS-013 | Administrative functions: require an explicit admin permission checked on the server | WP-033, WP-100, WP-131 |
| SEC-HIS-014 | Remote control of playback sessions: limited to the actor's own sessions unless the target | WP-085, WP-104 |
| SEC-HIS-015 | Server: never build a filesystem path from untrusted data | WP-001, WP-024 |
| SEC-HIS-016 | File access: go through directory handles that confine resolution | WP-060, WP-126 |
| SEC-HIS-017 | Scanner: classify files by the content of the resolved target | WP-009, WP-102 |
| SEC-HIS-019 | Server: never extract archives | WP-001, WP-109, WP-125 |
| SEC-HIS-020 | External programs: started only by one typed command builder | WP-045 |
| SEC-HIS-021 | Paths to external programs: come only from the install or a read-only host | WP-043, WP-045 |
| SEC-HIS-023 | Outbound requests: use one egress client that allows only http | WP-048 |
| SEC-HIS-024 | Server: never fetch a URL taken from an unauthenticated request | WP-102, WP-131 |
| SEC-HIS-026 | TLS clients: verify certificates | WP-001, WP-048 |
| SEC-HIS-027 | Web client: render all text that comes from media files | Unassigned: web-client behaviour. The client is outside this backend plan, and no client plan exists yet; it must carry this requirement. |
| SEC-HIS-028 | HTML response: carry a Content-Security-Policy with no unsafe-inline or unsafe-eval | WP-072, WP-131 |
| SEC-HIS-030 | Images from files or users: decoded by a memory-safe raster decoder with pixel | WP-079, WP-103 |
| SEC-HIS-031 | Response carrying file-derived bytes: set a Content-Type chosen by the server | WP-082, WP-103 |
| SEC-HIS-032 | Redirect target taken from a request: a same-origin relative path that starts with exactly | WP-005 |
| SEC-HIS-033 | Client-reported metadata: validated on the server to a bounded length | WP-087 |
| SEC-HIS-034 | XML parser in the server: reject DOCTYPE declarations and must not resolve external entities | WP-001 |
| SEC-HIS-035 | Server: never deserialise any format that can construct arbitrary types | WP-001, WP-039 |
| SEC-HIS-036 | Parser of untrusted input: live in the core crate under its no-panic | WP-008, WP-009, WP-010, WP-011, WP-012, WP-013, WP-014, WP-015, WP-016, WP-017, WP-018, WP-019, WP-020, WP-021, WP-023, WP-035, WP-039, WP-041, WP-052 |
| SEC-HIS-037 | Request parameter that sizes work: a bounded type with explicit limits | WP-044, WP-103 |
| SEC-HIS-038 | Database access: use parameterised queries whose column names | WP-042, WP-126 |
| SEC-HIS-041 | Native API: never accept session tokens | WP-044, WP-131 |
| SEC-HIS-042 | Stream and share tokens: checked on every request for signature | WP-082 |
| SEC-HIS-043 | Server secret: generated by a CSPRNG at first start | WP-047 |
| SEC-HIS-044 | Session tokens: stored only as keyed hashes | WP-047, WP-062 |
| SEC-HIS-046 | Authentication pathway: go through one credential verifier that applies the same | WP-064, WP-069, WP-117 |
| SEC-HIS-047 | Authentication: fail for a username that does not exist | WP-064 |
| SEC-HIS-052 | Server: never create router port mappings through UPnP-IGD | WP-001 |
| SEC-HIS-053 | LAN discovery: use mDNS | WP-132 |
| SEC-HIS-055 | On every start the server: log | WP-116 |
| SEC-HIS-056 | API: never install | WP-001, WP-131 |
| SEC-HIS-058 | CI workflows: never run untrusted pull-request code with secrets or write tokens | WP-124 |
| SEC-HIS-059 | Releases: signed with build provenance and a software bill | WP-074, WP-136 |
| SEC-HIS-060 | User's play history: never shown or sent to any other non-admin user | WP-086, WP-131 |
| SEC-HIS-061 | Project services: hold only the minimum routing data the per-server name | WP-139 |
| SEC-HIS-063 | Server: notify a user when a device | WP-097 |
| SEC-HIS-064 | Security fix: include a regression test that reproduces the exploit | WP-124 |
| SEC-HIS-065 | Vulnerability fixed in a released version: published as a GitHub security advisory with a CVE | WP-124, WP-136 |
| SEC-HIS-066 | "rival exploit replay" test suite: contain at least one test per incident | WP-102, WP-103, WP-109, WP-127, WP-131 |
| SEC-STD-001 | Repository: hold machine-readable copies of the pinned standard versions | WP-127 |
| SEC-STD-002 | CI: regenerate the coverage tables in this file | WP-127 |
| SEC-STD-003 | Scheduled job: check monthly for new versions of ASVS | WP-127 |
| SEC-STD-004 | SEC requirement whose release: referenced by at least one test | WP-127 |
| SEC-STD-005 | Release: publish | WP-136 |
| SEC-STD-006 | Before any server code stores: decide whether account passwords and TOTP exist in R1 | WP-125, WP-127 |
| SEC-STD-010 | Gunmetal: never include SAML | WP-001 |
| SEC-STD-011 | Regular expressions applied to untrusted input: run only in a linear-time engine | WP-001, WP-054 |
| SEC-STD-012 | Client: keep data keyed by untrusted strings | Unassigned: web-client behaviour. The client is outside this backend plan, and no client plan exists yet; it must carry this requirement. |
| SEC-STD-013 | API responses: JSON with Content-Type | WP-044, WP-131 |
| SEC-STD-014 | Cookie the server sets: a name and value of at most 4096 bytes | WP-062, WP-131 |
| SEC-STD-015 | Links to any origin outside: accept only https and http | WP-005 |
| SEC-STD-016 | Domain the project operates: send Strict-Transport-Security with max-age of at least 63072000 | WP-139 |
| SEC-STD-017 | Server: serve web assets only from a build-time manifest | WP-072 |
| SEC-STD-018 | Project: keep a cryptographic inventory | WP-001, WP-047, WP-122, WP-125, WP-136 |
| SEC-STD-019 | Cryptography: come only from implementations on a reviewed allow-list chosen | WP-001, WP-125 |
| SEC-STD-020 | AEAD key: a nonce strategy fixed in code | WP-047 |
| SEC-STD-021 | Decryption: return one opaque error that does not reveal | WP-047 |
| SEC-STD-022 | Security randomness: come from the operating system CSPRNG through one function | WP-001, WP-047 |
| SEC-STD-023 | Main server process: disable core dumps | WP-043 |
| SEC-STD-024 | Key derived from a human secret: use Argon2id with at least the second recommended RFC | WP-047 |
| SEC-STD-026 | Server: never act as an OAuth authorization server for third-party clients | WP-131 |
| SEC-STD-027 | Flow: never show an approval prompt on a person's device | WP-120 |
| SEC-STD-029 | Single-use or counted secret: consumed by one conditional update inside a single SQLite | WP-063, WP-080, WP-094, WP-108, WP-120 |
| SEC-STD-030 | Project: keep one machine-readable register of business limits | WP-130 |
| SEC-STD-031 | SQLite file the server did: opened read-only in a jailed worker with trusted_schema off | WP-109 |
| SEC-STD-033 | Release binaries: built position-independent with full RELRO and non-executable stacks | WP-001, WP-136 |
| SEC-STD-034 | Before the R1 tag: review the threat model | WP-136 |
| SEC-STD-035 | Source paths that implement authentication: listed in CODEOWNERS | WP-124 |
| SEC-STD-036 | SECURITY.md or a governance file: name the security lead | WP-124 |
| SEC-STD-037 | CONTRIBUTING.md: link a short secure-coding guide drawn from these files | WP-124 |
| SEC-STD-038 | CI: fuzz the running server through its generated OpenAPI description | WP-131 |
| SEC-STD-040 | Server: talk to its scan worker | WP-045, WP-061, WP-132 |

### Requirements that moved with their surface

These 31 rows are not due in R1. Each moves with the only surface it
protects to the release named, is verified there by the package named
(see [After R1](#after-r1-point-releases-and-later)), and blocks that
release under the traceability check until it has its test.

| ID | Short name | Due in | Verified by |
|---|---|---|---|
| SEC-TM-022 | OIDC sign-in: use the authorization code flow with PKCE | R1.2 | WP-096 |
| SEC-IAM-026 | OIDC sign-in: use the authorization code flow with PKCE | R1.2 | WP-096 |
| SEC-IAM-027 | ID tokens: verified with keys from the provider's JWKS using algorithms | R1.2 | WP-096 |
| SEC-IAM-028 | OIDC identity: keyed only by the pair | R1.2 | WP-096 |
| SEC-IAM-029 | Linking an OIDC identity: happen only inside that account's session after user verification | R1.2 | WP-096 |
| SEC-IAM-030 | OIDC auto-registration: off by default | R1.2 | WP-096 |
| SEC-IAM-031 | Provider claims: never confer the owner role | R1.2 | WP-096 |
| SEC-IAM-032 | Server-side calls to an OIDC provider: verify TLS certificates and must not follow redirects | R1.2 | WP-096 |
| SEC-IAM-033 | OIDC redirect URI: one exact registered URL on the configured origin | R1.2 | WP-096 |
| SEC-IAM-034 | Authorization request: bound to the one provider it was sent | R1.2 | WP-096 |
| SEC-IAM-035 | Sessions created through OIDC: lifetimes set by Gunmetal | R1.2 | WP-096 |
| SEC-IAM-036 | Administrator elevation for an account: require a fresh provider sign-in | R1.2 | WP-096 |
| SEC-API-097 | Public share links: use the fragment pattern | R1.2 | WP-134 |
| SEC-NET-010 | If the server uses: 128-bit random | R2 | WP-135 |
| SEC-NET-011 | Name service: answer A and AAAA queries | R2 | WP-129 |
| SEC-NET-012 | Name service: publish a CAA record for each registered label | R2 | WP-129, WP-135 |
| SEC-NET-069 | When the server uses the project: by default monitor Certificate Transparency for its own label | R2 | WP-135 |
| SEC-NET-070 | Project name service: launch only after its zone is on the Public | R2 | WP-129 |
| SEC-NET-071 | When the name service refuses: keep working on every other path | R2 | WP-135 |
| SEC-CLI-026 | When the web client signs: act as a confidential client | R1.2 | WP-096 |
| SEC-MED-050 | Entries in M3U: resolve only to items already indexed in the same | R1.1 | WP-022, WP-112 |
| SEC-MED-061 | Uploaded artwork: size-capped while they stream | R1.1 | WP-144 |
| SEC-OPS-030 | Diagnostic bundles: meet SEC-PRV-046 | R1.2 | WP-155 |
| SEC-PRV-006 | Images uploaded by users: re-encoded with all EXIF | R1.1 | WP-144 |
| SEC-PRV-014 | Provider requests: built only from a typed lookup-evidence value holding normalised | R1.1 | WP-137 |
| SEC-PRV-015 | Provider lookups: run only during scans | R1.1 | WP-137 |
| SEC-PRV-017 | Outbound provider requests: send a User-Agent naming only the project | R1.1 | WP-137 |
| SEC-PRV-046 | Diagnostic bundles: exclude the database | R1.2 | WP-155 |
| SEC-HIS-018 | Playlist files: resolve entries only to items in libraries the playlist | R1.1 | WP-112 |
| SEC-STD-008 | Secret a person chooses: accept any Unicode characters with no composition rules | R1.2 | WP-134 |
| SEC-STD-025 | OIDC authorization requests: carry exactly the scopes in the provider's configuration | R1.2 | WP-096 |

