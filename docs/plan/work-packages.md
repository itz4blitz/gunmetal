# Backend work packages

Written on 2026-10-02. Status: draft for the project owner's review,
revised the same day after an adversarial review (see
[Review notes](#review-notes)).

This is the build plan for the backend: the shared Rust core, the server and
the thin layer that hands the core to the web client. It turns the
[feature map](../features/README.md), the [interface documents](../ui/README.md)
and the capability list in [api-needs.md](api-needs.md) into work packages
that many coding agents can build at the same time, each under the rules in
[CONTRIBUTING.md](../../CONTRIBUTING.md) and [AGENTS.md](../../AGENTS.md).
Release R1 (music) is planned in full. Release R2 (video) is planned in
outline at the end.

The client itself (React Native, TypeScript) is not in this plan. Where a
package exists only so the client can call the core on the device, it says
so.

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
15. [R2 (video) in outline](#r2-video-in-outline)
16. [Decisions the owner must make](#decisions-the-owner-must-make)
17. [Review notes](#review-notes)

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
  structure that produces it (for example, that the unknown-user path runs
  the same hash verification as the wrong-password path).
- **Keys never leave the secrets crate.** Core functions that need a keyed
  MAC (capability URLs, token hashes, keyed public IDs) take a
  `MacProvider` trait object (WP-031) instead of key bytes, and
  `gunmetal-secrets` implements the keyed operation itself. No package
  needs `Secret::expose` outside `gunmetal-secrets` to sign, verify or
  hash. Each server package that calls such a core function writes a
  short adapter from the secrets crate's `KeyRing::mac` to the core trait.

### Parsers

Every parser follows the parsing contract in
[media-and-parser-safety.md](../security/media-and-parser-safety.md),
section 2: sans-I/O, `Limits`, `Budget` and `Depth` on every parse,
allocations bounded by bytes rather than declarations, checked arithmetic,
no `as` casts from wide to narrow integers, iteration that always advances,
typed errors that carry offsets, partial results for optional parts, lossy
text decoding with controls stripped, and typed values. Every public parse
entry point gets a fuzz harness registered in the harness registry
(SEC-MED-027) and a property test over arbitrary bytes that asserts it
returns rather than panics.

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

All logs are structured lines written through one logger with escaped,
length-capped fields (SEC-IAM-096, SEC-MED-062). No package formats a log
line by string concatenation, and no secret type can be logged (WP-047).

## Crate layout

**Proposal.** Twelve crates under `crates/`, each with a reason to exist.
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
| `gunmetal-testkit` | library, dev-only | Builders for synthetic FLAC, ID3, MP3, MP4, Ogg, WAV and AIFF; checksums; temporary directories; a manual clock; a synthetic library generator | Shared oracles for core and server tests. `publish = false`, used only as a dev-dependency, still under the gate. |
| `gunmetal-store` | library, I/O | The rebuildable SQLite cache: connections, the single writer, schema registration, the change log and the catalogue tables | ADM-080's one writer lives in one place; schema digest and rebuild logic are tested once. |
| `gunmetal-durable` | library, I/O | The identity store (SQLite with migrations), the user log segment files, the audit log, the derived-data store | Durable state has different rules from the cache (fsync, migrations, never discarded). ADR 3 defines it. |
| `gunmetal-secrets` | library, I/O | The root secret, key derivation and rotation, the `Secret<T>` wrapper, the vault for third-party secrets | `expose()` is reachable only here, enforced by a Clippy `disallowed-methods` entry (operations-and-incident-response.md, section 2). |
| `gunmetal-fs` | library, I/O | Library root handles, the open rules, symlink policy, walking, fingerprints, watching and polling | `std::fs` path functions are banned everywhere else (SEC-MED-033). |
| `gunmetal-worker` | library, I/O | The worker host loop, the IPC framing, the Linux sandbox, the worker pool and quarantine, the probe, artwork and loudness jobs | `std::process::Command` is allowed only here (SEC-MED-063); the image decoder and any audio decoder link only here. |
| `gunmetal-http` | library | The route table type, the request pipeline, security headers, body limits and the problem renderer | The route table is the single source for the allow-list, role-matrix and header tests and for the API reference (web-and-api-security.md, "The route table"). |
| `gunmetal-egress` | library, I/O | The only outbound HTTP client, with the egress gate and its record of connections | Makes the ban on other outbound clients enforceable (SEC-API-076). |
| `gunmetal-server` | library plus the `gunmetal` binary | Application state, every route handler, jobs, the command line | The composition root. Each feature package owns one module directory in it. |
| `gunmetal-wasm` | library, `cdylib` | A thin `wasm-bindgen` facade over the core for the web client | Keeps WASM bindings out of the core so the core stays dependency-light. |
| `xtask` | binary, dev-only | Repository checks: the harness registry, the route inventory, the lockfile-age check, the benchmark runner | The security baseline asks for these checks as Rust under the same gate (supply-chain-and-release.md). |

The worker is not a separate binary. It is the `gunmetal` binary started
with a hidden subcommand, so one binary ships (media-and-parser-safety.md,
section 4).

**Dependency direction.** `core` depends on nothing in the workspace. The
I/O crates depend on `core`. `gunmetal-server` depends on all of them.
`gunmetal-wasm` depends only on `core`. `gunmetal-testkit` depends on
nothing in the workspace, so it can never borrow the code it is meant to
check.

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
| **The derived-data store** | `derived/derived.db` | SQLite | Keyed by content identity, producer kind and producer version, so it never needs migrating: a new producer version simply writes new keys. | Rebuilds. Optional in backups (ADM-141). |

The audit log is a fifth, simpler store: JSON-lines segments of 16 MB in
`durable/audit/`, with `fsync` before critical actions return
(operations-and-incident-response.md, section 3).

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
  on readers only.
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
| `serde_json` | server, durable | JSON for the HTTP API, the audit log and exports. | None reasonable. |
| `hmac`, `sha2`, `sha1` (RustCrypto) | core | HMAC-SHA-256 for capability URLs and token hashes; SHA-256 for content identity windows and the schema digest; HMAC-SHA-1 for TOTP, which authenticator apps expect (RFC 6238). Pure Rust. | `ring`, which includes C and assembly. |
| `unicode-normalization` | core | Diacritic-insensitive search and natural sort (DIS-085, MUS-020) need canonical decomposition. Pure Rust tables. | A hand-maintained folding table, which is smaller but will be wrong for scripts nobody tested. |
| `tokio` | server, egress, fs | The async runtime under the HTTP server, timers and the blocking pool. | None realistic for axum. |
| `axum` (on `hyper` and `tower`) | http, server | The security baseline names it as a fit: composable middleware and an enumerable router. Its server-sent events support covers the R1 event channel without a WebSocket dependency. | Hand-rolled `hyper` service, which is more code under the gate for no security gain. |
| `rusqlite` with `bundled` | store, durable | SQLite, with the amalgamation built in so every platform runs the same version. The supply-chain document already expects this bundled C code. | A pure-Rust SQLite reimplementation, which is not mature enough for the only durable store (unverified for every candidate). |
| `cap-std` | fs | Opening files beneath a root with `openat2` and `RESOLVE_BENEATH` (SEC-MED-033). | Hand-written `openat2` calls through `rustix`. |
| `rustix` | fs, worker | Safe wrappers for `pread`, `fstat`, descriptor passing with `SCM_RIGHTS`, rlimits and `prctl`, with no `unsafe` in Gunmetal's code. | `nix`, which is broader. |
| `landlock` | worker | Filesystem and network confinement of the worker (SEC-MED-022). Maintained under the Landlock project. | None. |
| `seccompiler` | worker | Pure-Rust seccomp-bpf filters (SEC-MED-022). It does not support 32-bit ARM (media-and-parser-safety.md, section 4). | `libseccomp`, which is C. |
| `argon2` (RustCrypto) | server | Password hashing with Argon2id (RFC 9106). | None. |
| `hkdf`, `chacha20poly1305` (RustCrypto) | secrets | Deriving purpose keys from the root secret; encrypting third-party secrets at rest. | `ring`. |
| `ed25519-dalek` | secrets, server | Signing backups and audit checkpoints; verifying the update feed. | `ring`. |
| `p256` (RustCrypto) | server | Verifying ES256 passkey assertions. | See owner decision 9 for RS256. |
| `getrandom` | server, secrets | The operating system's CSPRNG. Already in the lock file through `proptest`. | None. |
| `toml` | server | The configuration file (ADM-007). | A hand-written key-value format, which users would have to learn. |
| `image` (JPEG, PNG, WebP, GIF only) | worker | Decoding artwork with `image::Limits` and re-encoding derivatives (SEC-MED-044 to SEC-MED-046). Runs only inside the worker. | None in pure Rust with this coverage. |
| `notify` | fs | File-change notification on Linux, macOS and Windows (LIB-014). | `rustix` inotify, Linux only (owner decision 18). |
| `rustls`, `tokio-rustls` | server, egress | HTTPS with the owner's certificate (ACC-098) and outbound TLS. Needs a crypto provider (owner decision 8). | None in pure Rust. |
| `wasm-bindgen` | wasm | The web client calls the core through it (ADR 1, decision 2). | None. |
| `symphonia` | worker | Only if ADR 5 is accepted: decoding untagged FLAC, MP3, AAC and Vorbis for loudness measurement. The research notes it lacks Opus and HE-AAC (music.md, MUS-086). | No measurement in R1; tags plus the fallback gain (MUS-089). |

**Missing from this table, and needed.** Three R1 packages need something
the table does not list. Each is part of owner decision 4 or the decision
named:

- **RSA signature verification for single sign-on (WP-096).** The OpenID
  Connect Core specification makes RS256 the algorithm providers must
  support, and common self-hosted providers sign ID tokens with it by
  default (unverified per provider). Without an RSA verifier, R1's single
  sign-on works only with providers configured for ES256 or EdDSA. The
  candidate is the RustCrypto `rsa` crate, used for verification only;
  its past timing advisory concerned decryption (unverified whether it
  affects verification). Owner decision 10.
- **A backup archive container (WP-090, WP-109).** Encrypting with `age`
  still needs an archive format inside it, either a small hand-written
  one or the `tar` crate. Owner decision 12.
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
egress crate uses `hyper`'s client directly. No inflate in the core in R1:
compressed ID3v2 frames are rare, so they are skipped and recorded rather
than decompressed (owner decision 20); PNG inflate happens inside `image`
in the worker.

**Open on purpose.** WebAuthn verification, OIDC, the update feed's
verification and backup encryption each have a crate that would do the job
(`webauthn-rs`, `openidconnect`, `tough`, `age`) and a smaller hand-written
path on top of the RustCrypto primitives above. The plan sizes the
hand-written paths and leaves the choice to the owner (decisions 9, 10 and
12). One fact matters to the choice and was not checked here: whether
`webauthn-rs` still depends on OpenSSL (unverified).

## Working in parallel: shared files and merging

### Ownership

Every path in the repository has at most one owner per wave. The packages
list their owned paths exactly. The only files that more than one package in
a wave may touch are the **registry files** below, and only in the narrow
way described.

| Shared file | Owner | What other packages may do |
|---|---|---|
| Root `Cargo.toml` | WP-001 only, in every wave | Nothing. A package that needs a crate not already in `[workspace.dependencies]` stops and files a dependency request; the owner approves it and the integrator adds it between merges. New crates need no root edit, because `members = ["crates/*"]` already picks them up. |
| A crate's `Cargo.toml` | The package that creates the crate | Add one dependency line, in sorted position, using `workspace = true`. Never change another line. |
| `lib.rs` and every `mod.rs` | Nobody; these are registries | They hold only `mod` and `pub mod` lines and re-exports, never code, so a module's types live in a named file beside it (for example `formats/flac/metadata.rs`, never `formats/flac/mod.rs`). Any package may create a missing one; two packages that both create it resolve the add/add conflict by keeping every line and re-sorting. This is what lets WP-012 and WP-013, or WP-017 and WP-018, share a directory in the same wave. |
| `crates/gunmetal-core/src/harness.rs` | WP-008 | Add one harness registration line per new parse entry point. |
| `crates/gunmetal-testkit/src/lib.rs` | WP-007 | Add one `pub mod` line per builder module. |
| `crates/gunmetal-server/src/app.rs` | WP-043 | Add one line to register a module's state, schema parts or jobs. |
| `crates/gunmetal-server/src/routes.rs` | WP-118 | Add one line to register a module's routes. |
| `crates/gunmetal-server/src/cli.rs` | WP-043 | WP-043 parses every subcommand; later packages add one dispatch line that sends their subcommand to their own module (`doctor` to WP-116, `admin recover` to WP-106, `migrate`, `rebuild` and `snapshot restore` to WP-095, `restore` to WP-109, the hidden worker entry to WP-061, `service` to WP-121). |
| `crates/gunmetal-core/src/problem.rs` | WP-006 | Add one problem code per line, sorted. |
| `scripts/gate.sh`, `.github/workflows/ci.yml` | The integrator | Packages do not edit them. A package that needs a gate change (a cargo feature turned on, a new target) files a request and the integrator applies it between merges, as for the root `Cargo.toml`. A package that needs its own CI job adds its own workflow file instead (WP-008 owns `fuzz.yml`, WP-088 owns `wasm.yml`, WP-121 owns `release.yml`). |
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
proposes how the gate itself scales once the workspace grows.

## Waves at a glance

| Wave | Packages | Count | What it produces |
|---|---|---:|---|
| 0 | WP-001 to WP-008, WP-122 | 9 | Lints and merge rules, ADR 3 to 6, the parse contract, text and typed values, identifiers and the problem catalogue, the testkit, the fuzz harness registry, the schema digest |
| 1 | WP-009 to WP-048 | 40 | Every R1 container parser, lyrics and M3U, the pure logic of the queue, shuffle, rules, gain, tokens, authorisation and the user log; the store, server, HTTP, worker, identity-store, secrets and egress crates |
| 2 | WP-049 to WP-058, WP-060 to WP-071, WP-118, WP-119 | 24 | Tag mapping, the file probe, search, the decision engine, the audio packager, radio; file access, worker IPC, sessions, passwords, the authorisation layer, the change log, the catalogue store, the user log, audit log, task runner and derived-data store; the route registry and listener; the synthetic library generator |
| 3 | WP-059, WP-072 to WP-087, WP-089 to WP-100, WP-120 | 30 | Home rows, web assets, network settings, the update check, the music model, identity and scan diff, the worker pool and jobs, setup, sign-in, passkeys, streaming, the event channel, sync, the queue and listening services, accounts, backups, tokens, rules, playlists, users, startup, SSO, alerts, triggers, library administration, tasks |
| 4 | WP-088, WP-101 to WP-109, WP-121 | 11 | The WASM facade, built-in HTTPS, the scan pipeline and what needs it (artwork serving, session control, the packaging route, account recovery, curation, history import and export, restore), release builds and service install |
| 5 | WP-110 to WP-115, WP-123 | 7 | Library health, review and trash, playlist files, derived jobs, loudness analysis, the scan benchmark, the parser-upgrade re-read |
| 6 | WP-116 to WP-117 | 2 | Doctor and diagnostics, and the R1 flow acceptance tests |

Wave 1 is the widest point: forty packages that need nothing but wave 0.
That is where most parallel agents are useful. The critical path to a
running music server is WP-004 → WP-012 → WP-052 → WP-075 → WP-102, with the
worker chain WP-045 → WP-061 → WP-078 → WP-102 and the store chain
WP-122 → WP-042 → WP-067 → WP-102 beside it.

**Wave 0 and the lints.** WP-004 to WP-008 and WP-122 run beside WP-001,
which turns on the core's deny-level lints. They write their code to the
lint list in WP-001 from the first commit (the list is spelled out there),
so the order in which wave 0 merges does not matter; the gate run on rebase
is what catches a mismatch.

## Wave 0: what everyone agrees on first

### WP-001 Workspace rules, lints and the merge protocol

- **Wave** 0 · **Size** M · **Depends on** nothing.
- **Owns** `Cargo.toml`, `clippy.toml`, `deny.toml`,
  `.github/workflows/ci.yml`, `scripts/gate.sh`,
  `crates/gunmetal-core/Cargo.toml`, `crates/gunmetal-core/src/lib.rs`,
  `crates/gunmetal-core/src/ebml.rs`, and a new "Working in parallel"
  section of `CONTRIBUTING.md`.
- **Serves** SEC-MED-002, SEC-MED-003, SEC-MED-004, SEC-SUP-021,
  SEC-SUP-024; the engineering standards in the README.
- **Scope.** Turn on the core's deny-level lints (`unwrap_used`,
  `expect_used`, `panic`, `unreachable`, `todo`, `unimplemented`,
  `indexing_slicing`, `arithmetic_side_effects`, `large_stack_arrays`) and
  bring `ebml.rs` into line with them, replacing each flagged slice with
  `get`, `split_at_checked` or an `#[expect]` naming its invariant, as
  media-and-parser-safety.md section 2 describes. Add `clippy.toml` entries
  that ban pre-sizing calls in the core, path-based `std::fs` outside
  `gunmetal-fs`, `std::process::Command` outside the worker's sandbox
  module, and `Secret::expose` outside `gunmetal-secrets`. A
  `disallowed-methods` entry applies to the whole workspace, so each
  sanctioned exception is a module-level `#[expect(clippy::disallowed_methods, reason = "…")]`
  and an xtask check (WP-008) fails when an exception appears in any module
  not on a short written list. The `std::fs` ban needs that list from the
  start: the data directory's own files (the cache and identity databases,
  log and audit segments, the root secret, the data-directory layout, the
  testkit's temporary directories) are not library media, yet they are
  written by `gunmetal-store`, `gunmetal-durable`, `gunmetal-secrets`,
  `gunmetal-server/src/datadir.rs` and `gunmetal-testkit`. SEC-MED-033 is
  about library access, so this plan proposes that those named modules are
  the exceptions and library roots stay `gunmetal-fs` only (owner decision
  33). Add
  `overflow-checks = true` to the release profile. Add a 32-bit CI job that
  runs the core's tests on `i686-unknown-linux-gnu`. Set up `cargo-deny`
  and `cargo-vet` while the graph is tiny. Write the approved crate list
  into `[workspace.dependencies]` once the owner approves it. Write the
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
  core, a `std::fs::read` in a stand-in server module) fails the lint step;
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

### WP-003 ADRs 4 to 6: the audio packager, in-process decoders and the workspace

- **Wave** 0 · **Size** M · **Depends on** nothing.
- **Owns** `docs/adr/0004-audio-packager.md`,
  `docs/adr/0005-in-process-audio-decoders.md`,
  `docs/adr/0006-workspace-and-dependencies.md`.
- **Serves** feature map open decisions 8 and 9; MUS-067, MUS-086, MUS-230;
  owner decisions 4 and 5.
- **ADR 6.** Owner decision 5 asks for the crate layout to be recorded in
  an ADR that extends ADR 1 rather than rewriting it, and no package owned
  that record. ADR 6 records the crate table, the dependency direction, the
  approved external crates with their reasons, and the scoped exceptions to
  the `clippy.toml` bans (owner decision 33). Like ADRs 3 to 5, it is a
  draft until the owner accepts it; none of these packages edits ADR 1 or
  ADR 2 (AGENTS.md).
- **Scope.** ADR 4 amends ADR 2 decision 2 to name the light, in-process,
  audio-only packager (FLAC, Opus and MP3 frames copied into fragmented MP4
  without re-encoding) as the one exception to "no remuxer for music". ADR
  5 decides whether a third-party pure-Rust decoder (the candidate is
  Symphonia, MPL-2.0) may decode untrusted audio inside the sandboxed
  worker for loudness measurement, with its licence, fuzzing and resource
  limits, as SEC-MED-026 requires.
- **Not in scope.** The packager itself (WP-056) and the measurement job
  (WP-114).
- **Tests.** Not code.
- **Risks and decisions.** If ADR 4 is rejected, WP-056 and WP-105 drop out
  and browser gapless (MUS-067) rests on whatever each browser's Media
  Source Extensions accept natively, which is unverified per browser. If
  ADR 5 is rejected, WP-029 and WP-114 move to R2 and R1 ships tags plus the
  fallback gain (MUS-089). Owner decisions 2 and 3.

### WP-004 Core parse contract

- **Wave** 0 · **Size** M · **Depends on** nothing.
- **Owns** `crates/gunmetal-core/src/parse/` (`crates/gunmetal-core/src/parse/mod.rs`, `crates/gunmetal-core/src/parse/limits.rs`,
  `crates/gunmetal-core/src/parse/budget.rs`, `crates/gunmetal-core/src/parse/cursor.rs`, `crates/gunmetal-core/src/parse/sansio.rs`, `crates/gunmetal-core/src/parse/capacity.rs`).
- **Serves** SEC-MED-001, SEC-MED-003, SEC-MED-005 to SEC-MED-008,
  SEC-MED-010; LIB-019, LIB-020.
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

### WP-005 Core text, typed values, time and addresses

- **Wave** 0 · **Size** M · **Depends on** nothing.
- **Owns** `crates/gunmetal-core/src/text.rs`, `crates/gunmetal-core/src/values.rs`,
  `crates/gunmetal-core/src/time.rs`, `crates/gunmetal-core/src/base64.rs`, `crates/gunmetal-core/src/net.rs`.
- **Serves** SEC-MED-013, SEC-MED-014; MUS-036, INT-009, MUS-013;
  CLI-150 and ACC-080's address warnings (the classifier).
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
  parser, the egress guard, the secure-context report and invite warnings.
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
  address, IPv4-mapped IPv6, and malformed prefixes.
- **Risks.** Sample-rate and gain ranges become user-visible behaviour; the
  ranges come from SEC-MED-014 and SEC-MED-015.

### WP-006 Core identifiers and problem catalogue

- **Wave** 0 · **Size** M · **Depends on** nothing.
- **Owns** `crates/gunmetal-core/src/id.rs`, `crates/gunmetal-core/src/problem.rs`.
- **Serves** INT-008, ACC-121 (identifier format), SEC-MED-017 (reasons
  admins can read), the problem-type catalogue in web-and-api-security.md.
- **Scope.** Public identifiers: 16 bytes, written as a kind prefix (`trk_`,
  `alb_`, `art_`, `rgp_`, `pls_`, `usr_`, `prf_`, `dev_`, `lib_`, `inv_`,
  `tok_`, and room for R2 kinds) plus 26 lowercase Crockford base32
  characters, so kind confusion is a parse error. The problem catalogue:
  a stable code for every error the system can report, its HTTP status
  where it has one, and its plain-language English text, plus the
  `Describe` trait every core error implements.
- **Not in scope.** How library item IDs are derived (owner decision 11;
  WP-077 implements it). Translations of the problem text (CLI-146 builds on
  the codes).
- **Interface sketch.**

  ```rust
  pub enum IdKind { Track, Album, Artist, ReleaseGroup, Playlist, User, Profile, Device, Library, Invite, Token /* … */ }
  pub struct PublicId { kind: IdKind, bytes: [u8; 16] }
  impl PublicId { pub fn new(kind: IdKind, bytes: [u8; 16]) -> Self;
      pub fn parse(s: &str, expected: IdKind) -> Result<Self, IdError>; }
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
  and, where set, a status from an allowed list. Properties: display then
  parse returns the same identifier for any bytes and kind; parsing never
  panics on arbitrary strings.
- **Risks.** The catalogue becomes a hotspot: every later package adds codes.
  It is a registry file under the merge protocol (one line per code, sorted).

### WP-007 Testkit foundation

- **Wave** 0 · **Size** M · **Depends on** nothing.
- **Owns** `crates/gunmetal-testkit/` (creates the crate; `crates/gunmetal-testkit/Cargo.toml`,
  `crates/gunmetal-testkit/src/lib.rs`, `crates/gunmetal-testkit/src/bytes.rs`, `crates/gunmetal-testkit/src/checksum.rs`, `crates/gunmetal-testkit/src/tempdir.rs`,
  `crates/gunmetal-testkit/src/clock.rs`).
- **Serves** CONTRIBUTING.md rules 3 and 4; the synthetic-media rule of this
  plan.
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
- **Owns** `crates/gunmetal-core/src/harness.rs`,
  `crates/gunmetal-core/tests/corpus.rs`, `crates/gunmetal-core/corpus/`,
  `fuzz/` (outside the workspace), `.github/workflows/fuzz.yml`,
  `crates/xtask/` (WP-115 later adds `crates/xtask/src/bench.rs`).
- **Serves** SEC-MED-027 to SEC-MED-031, SEC-SUP-027.
- **Scope.** A registry of harnesses, each a plain function from bytes to
  nothing that calls one parse entry point and asserts it returned;
  harnesses for `decode_vint`, `decode_element_header` and `elements` now;
  a stable `cargo test` that replays every corpus file for every harness; a
  `cargo-fuzz` target per harness under `fuzz/`; an `xtask check-harnesses`
  that fails when a public parse function in a format module has no
  registered harness; an `xtask lockfile-age` check for the seven-day rule;
  an `xtask lint-exceptions` check that fails when a
  `disallowed_methods` exception appears outside the written list (WP-001).
- **Not in scope.** Harnesses for later parsers (each format package
  registers its own). Nightly fuzzing infrastructure beyond the CI job
  definition.
- **Interface sketch.**

  ```rust
  pub struct Harness { pub name: &'static str, pub run: fn(&[u8]) }
  pub fn all() -> &'static [Harness];
  ```

- **Tests.** The replay test runs every harness over its corpus and over a
  handful of literal inputs. The xtask's own tests use small fake source
  trees built in a temporary directory: one with a missing harness fails,
  one with every harness registered passes; one lock file with a crate
  published three days ago fails, one with eight days passes (publish
  times are injected, never fetched in tests).
- **Risks.** The rule for what counts as a "public parse entry point" must
  be simple enough for xtask to check without a Rust parser; proposal: any
  `pub fn` in `src/formats/**` or in a module on a parser-module list kept
  in the xtask (initially `lyrics.rs`, `m3u.rs`, `http/`, `path.rs`,
  `wire.rs`, `logframe.rs`, `webauthn/`, `import/`). An opt-in constant
  alone would let a new parser outside `formats/` skip its harness
  silently, which is the failure the check exists to prevent.

### WP-122 Schema parts and digest (added in review)

- **Wave** 0 · **Size** S · **Depends on** nothing (needs `sha2`, owner
  decision 4).
- **Owns** `crates/gunmetal-core/src/schema.rs`.
- **Serves** ADM-077, ADM-058; the "schema by parts" rule in
  [The cache](#the-cache) and [The identity store](#the-identity-store).
- **Why it exists.** Both the cache (WP-042) and the identity store
  (WP-046) build their schema from named parts and compare a digest, and
  both are wave 1. Defining `SchemaPart` in `gunmetal-store` would have
  made WP-046 depend on WP-042 in the same wave. The part type and the
  digest are pure, so they belong in the core.
- **Scope.** `SchemaPart { name, sql }`; validation (names unique,
  non-empty, ASCII); the SHA-256 digest over the parts in name order with
  an unambiguous framing (each name and body length-prefixed, so moving
  text from one part to the next changes the digest).
- **Interface sketch.** `pub struct SchemaPart { pub name: &'static str, pub sql: &'static str }`;
  `pub fn digest(parts: &[SchemaPart]) -> Result<[u8; 32], SchemaError>`.
- **Tests.** The digest of a fixed set of parts against a literal value
  computed by an independent SHA-256 run over literally written framed
  bytes; input order does not change the digest; a duplicate name is
  refused; moving one character between two parts changes the digest.

## Wave 1: core leaves and infrastructure crates

Every format package in this wave follows the same pattern: a sans-I/O
parser in `crates/gunmetal-core/src/formats/`, a builder for that format in
`crates/gunmetal-testkit/src/`, a fuzz harness registered in `harness.rs`, a
structure-aware generator for SEC-MED-031, and a property test that the
parser returns for arbitrary bytes. Container parsers return **raw** tag
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
- **Serves** SEC-MED-011, SEC-MED-012; MUS-032.
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
- **Serves** MUS-154, MUS-155, MUS-156, SEC-MED-049; API-CAT-06.
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
- **Tests.** Lines out of order (sorted); identical timestamps; a stamp of
  99:59.99 and one past 24 hours; hundredths and thousandths; an offset of
  plus and minus; a line of 4,097 bytes; 20,001 lines; blank lines between
  timed lines; a file mixing timed and untimed lines. `position` at each
  line's exact timestamp, one millisecond before it, and before the first
  line, each with its literal expected cursor. Property: `position` is
  monotonic as the time increases (on its own this would pass for a
  function that always returned `None`, so the examples carry the weight).

### WP-022 M3U and M3U8 parser and writer

- **Wave** 1 · **Size** M · **Depends on** WP-004, WP-005.
- **Owns** `crates/gunmetal-core/src/m3u.rs`.
- **Serves** MUS-140, LIB-192, SEC-MED-050; API-PL-03, API-PL-04.
- **Scope.** Parse M3U and M3U8 with `#EXTM3U` and `#EXTINF` duration and
  title, into entries that are either a relative path, an absolute path or
  a URL, each classified so the caller can drop what SEC-MED-050 forbids.
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

### WP-023 HTTP header parsers

- **Wave** 1 · **Size** M · **Depends on** WP-004, WP-005.
- **Owns** `crates/gunmetal-core/src/http/` (`crates/gunmetal-core/src/http/mod.rs`, `crates/gunmetal-core/src/http/range.rs`,
  `crates/gunmetal-core/src/http/forwarded.rs`).
- **Serves** SEC-MED-060, ACC-097, ACC-134; API-STR-03, API-SET-01.
- **Scope.** The `Range` header (one range only; any other form means the
  full representation; unsatisfiable means 416), `Forwarded` and
  `X-Forwarded-For` chains walked from the right against a trusted-proxy
  list, using the address types and classifier from WP-005.
- **Not in scope.** Applying any of these to requests (WP-044, WP-073,
  WP-082).
- **Interface sketch.** `pub fn range(header: &[u8], len: u64) -> RangeOutcome`;
  `pub fn client_addr(peer: IpAddr, headers: ForwardedHeaders<'_>, trusted: &[IpNet]) -> IpAddr`.
- **Tests.** `bytes=0-`, `bytes=-500`, `bytes=500-100`, `bytes=0-0`,
  `bytes=0-1,5-9` (full representation), a range starting at the length
  (416), numbers with 30 digits, spaces, upper-case unit. Forwarded chains
  with an untrusted hop in the middle, IPv6 in brackets with ports,
  obfuscated identifiers. Property: a satisfiable range is always inside
  `0..len`.

### WP-024 Path rules

- **Wave** 1 · **Size** M · **Depends on** WP-005.
- **Owns** `crates/gunmetal-core/src/path.rs`.
- **Serves** SEC-MED-034, SEC-MED-037, SEC-MED-039, SEC-MED-040, LIB-006,
  LIB-008; API-CAT-10, API-LIB-02.
- **Scope.** Paths as raw byte components: relative-path normalisation,
  containment ("is this chain beneath that root"), the root refusal rules
  (filesystem root, system directories, the data directory, overlaps),
  display names decoded lossily with controls escaped, and the exclusion
  pattern language (a small glob subset: `*`, `**`, `?`, literal names,
  case-insensitive option).
- **Not in scope.** Touching the filesystem (WP-060).
- **Interface sketch.** `pub struct RelPath(Vec<Vec<u8>>)`;
  `pub fn normalise(components: &[&[u8]]) -> Result<RelPath, PathError>`;
  `pub fn refuse_root(candidate: &RawPath, data_dirs: &[RawPath]) -> Option<RootRefusal>`;
  `pub struct Exclusions; impl Exclusions { pub fn parse(lines: &str) -> Result<Self, PatternError>; pub fn excludes(&self, p: &RelPath) -> bool; }`.
- **Tests.** Non-UTF-8 names; names with newlines and escape characters;
  `.` and `..` at every position; a root that equals, contains and lies
  inside the data directory; `/`, `/etc`, `/proc`; patterns `**/*.tmp`,
  `Extras/`, `?.flac`. Property: a normalised path never contains `..` and
  containment agrees with an independent prefix-on-components oracle.

### WP-025 Queue document and verbs

- **Wave** 1 · **Size** L · **Depends on** WP-005, WP-006.
- **Owns** `crates/gunmetal-core/src/queue/`.
- **Serves** MUS-116 to MUS-120, MUS-122, MUS-123, MUS-077, MUS-129, LAT-009,
  CLI-103; API-QUE-01 to API-QUE-03.
- **Scope.** The versioned queue document: three lanes (Up next, From with
  its source, Continue with), named listening contexts, the insertion
  cursor, repeat and stop-after modes, the current item and position, and
  the active device. The verbs from player.md (play, play next, add, play
  last, start radio, move, remove, clear, clear Up next) as operations
  applied to a version, including multi-item operations as one. Stale
  operations are rejected with the current version so the client can
  rebase, and the rebase function itself, so client and server agree.
  "Picks survive a new Play" is implemented as the player.md proposal
  (owner decision 28).
- **Not in scope.** Shuffle orders (WP-026); radio picks (WP-058); storage
  (WP-085).
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

- **Wave** 1 · **Size** M · **Depends on** WP-005.
- **Owns** `crates/gunmetal-core/src/shuffle.rs`.
- **Serves** MUS-126, MUS-127, MUS-128, MUS-120.
- **Scope.** A small seeded generator owned by the core (so the order is
  the same on every device and every platform), and three modes over the
  From lane: random, spread out (the same artist or album never bunches,
  recent plays later), and by album (random albums, each in order).
  Reshuffle the rest re-seeds from the current item. Turning shuffle off
  restores source order from the current item.
- **Not in scope.** Lane mechanics (WP-025).
- **Interface sketch.** `pub fn order(items: &[ShuffleItem], mode: Mode, seed: u64, recent: &RecentPlays) -> Vec<usize>`.
- **Tests.** The generator is a published algorithm with published
  reference outputs (for example SplitMix64; which algorithms publish
  vectors is unverified), and is tested against those outputs. "Random"
  order is checked against a Fisher–Yates shuffle written independently in
  the test and fed the same generator outputs. Literal orders written down
  from a run of the code under test are not used, because that derives the
  expected value from the code (CONTRIBUTING.md rule 4). Properties: the output is a
  permutation; spread out never puts the same artist adjacent when another
  artist is available at that point; by album keeps each album's tracks in
  order and contiguous; the same seed and input give the same order.
- **Risks.** "Spread out" as default is a proposal from player.md.

### WP-027 Rule language

- **Wave** 1 · **Size** L · **Depends on** WP-005.
- **Owns** `crates/gunmetal-core/src/rules/`.
- **Serves** DIS-119 to DIS-122, MUS-143 to MUS-146, MUS-149, DIS-105;
  API-PL-05, API-HOME-01.
- **Scope.** A versioned rule tree (all, any, not; comparisons on typed
  fields; "in the last N days"; membership in playlists and loved items;
  limits by count, duration or percentage; sorts; seeded random order),
  its validation (depth, node count, known fields for this version),
  forward-compatible serialisation that keeps unknown nodes so a rule
  written by a newer client survives an older one, and evaluation over any
  record type that implements a field-access trait.
- **Not in scope.** The music field catalogue binding, which WP-059 and
  WP-092 provide. The visual editor (client).
- **Same-wave note.** WP-040 is in the same wave, so this package defines
  `FieldId` as a plain `u16` newtype and the value types it compares, and
  imports nothing from `catalog/`. WP-040's field table lists numeric codes
  without importing `rules/`; the binding packages join the two.
- **Interface sketch.** `pub enum Rule { All(Vec<Rule>), Any(Vec<Rule>), Not(Box<Rule>), Cmp { field: FieldId, op: CmpOp, value: Value }, Unknown(RawNode) }`;
  `pub trait Fields { fn get(&self, f: FieldId) -> FieldValue<'_>; }`;
  `pub fn evaluate<'a, R: Fields>(q: &Query, items: impl Iterator<Item = &'a R>, ctx: &EvalCtx) -> Vec<usize>`.
- **Tests.** Each operator on each value type, including missing fields and
  multi-valued fields ("genre is Jazz" on a track with three genres);
  percentage limits rounding; "last 30 days" at the boundary using an
  injected `now`; seeded random limit. Properties: `All([r])` equals `r`;
  `Not(Not(r))` equals `r`; evaluation is deterministic; a rule with an
  unknown node round-trips byte for byte; validation rejects trees deeper
  than the limit without recursing past it.

### WP-028 Gain decision

- **Wave** 1 · **Size** S · **Depends on** WP-005.
- **Owns** `crates/gunmetal-core/src/gain.rs`.
- **Serves** MUS-084, MUS-085, MUS-087, MUS-088, MUS-089, MUS-090,
  SEC-MED-015, CLI-151.
- **Scope.** Given a track's tag gains and peaks, its Opus header gain, the
  mode (auto, track, album, off), the target level and whether the
  previous and next items are from the same album in the From lane, return
  the gain to apply, its source (tagged, measured, estimated) and the
  clamp that was applied. Implements RFC 7845's rule that the Opus header
  gain always applies and R128 tags add to it.
- **Not in scope.** Measuring loudness (WP-029). Applying gain (client).
- **Interface sketch.** `pub fn decide(input: &GainInput, mode: Mode, target: Lufs) -> GainDecision`.
- **Tests.** A grid of gain and peak values, each row with its literal
  expected decision worked out by hand from RFC 7845 and the feature map's
  rules, including +60 dB, −60 dB, NaN
  rejected earlier, a peak of exactly 1.0 and of 1.5; positive gain with no
  peak known (not applied); album gain chosen only inside an album run;
  Opus header gain plus R128 track gain. Property: applied gain never
  takes the recorded peak above full scale.

### WP-029 Loudness meter (conditional on ADR 5)

- **Wave** 1 · **Size** M · **Depends on** WP-003 (accepted), WP-005.
- **Owns** `crates/gunmetal-core/src/loudness.rs`.
- **Serves** MUS-086; LIB-024.
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

### WP-030 Player state machine

- **Wave** 1 · **Size** S · **Depends on** WP-001.
- **Owns** `crates/gunmetal-core/src/player.rs`.
- **Serves** player.md "One playback model" (a proposal); MUS-079,
  ADM-102, ACC-069.
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
- **Serves** ACC-122, ACC-049, INT-023, SEC-EXT-008; API-STR-01, API-TOK-01.
- **Scope.** The capability-URL token layout from web-and-api-security.md
  (version, key ID, expiry, operation, representation, object, session
  handle, truncated HMAC-SHA-256 tag) with verification in the order parse,
  MAC in constant time, expiry; the API key format `gmk_<id>_<secret><checksum>`;
  hashing of session tokens and keys with a derived key. The core never
  sees key bytes: it hands the message to a `MacProvider` and gets the tag
  back, so the server can sign and verify without calling
  `Secret::expose` outside `gunmetal-secrets` (see the ground rules). The
  same trait serves keyed public-ID derivation (WP-077).
- **Not in scope.** Session lookup and visibility checks (WP-082).
- **Interface sketch.** `pub trait MacProvider { fn current_kid(&self) -> u8; fn mac(&self, kid: u8, msg: &[u8]) -> Option<[u8; 32]>; }`;
  `pub fn sign(fields: &CapFields, mac: &dyn MacProvider) -> String`;
  `pub fn verify(token: &str, mac: &dyn MacProvider, now: Timestamp) -> Result<CapFields, CapError>`;
  `pub fn parse_api_key(s: &str) -> Result<ApiKeyParts, KeyFormatError>`.
- **Tests.** RFC 4231 HMAC-SHA-256 vectors through the module's MAC helper;
  a token signed with key 1 verified during rotation overlap; a flipped bit
  in every field rejected; expiry at the boundary second; an unknown
  version; a key with a bad checksum; truncation at every length. Property:
  sign then verify returns the fields.

### WP-032 Rate-limit arithmetic

- **Wave** 1 · **Size** S · **Depends on** WP-005.
- **Owns** `crates/gunmetal-core/src/ratelimit.rs`.
- **Serves** ACC-063, INT-012, SEC-API-056 and SEC-API-057; API-AUTH-07.
- **Scope.** A GCRA limiter over an injected clock, and the back-off
  schedule for sign-in failures.
- **Interface sketch.** `pub fn check(state: Option<Tat>, now: Timestamp, rate: Rate) -> (Decision, Tat)`.
- **Tests.** Bursts at the limit and one over; recovery after exactly the
  emission interval; clock going backwards (treated as no time passed).
  Property: over any sequence of requests, allowed requests never exceed
  the burst plus rate times elapsed time.

### WP-033 Authorisation policy

- **Wave** 1 · **Size** M · **Depends on** WP-006.
- **Owns** `crates/gunmetal-core/src/authz/`.
- **Serves** SEC-IAM-068, ACC-120, ACC-121, ACC-030, ACC-037, ACC-040,
  INT-022; every route.
- **Scope.** One deny-by-default pure function from a principal (role,
  grants, token scope), an action, a resource description and a context to
  allow or a typed denial reason; the no-escalation rule for tokens and
  invites; the scope intersection rule. Three plain types that the server
  packages share, defined here so that WP-062, WP-065 and WP-067 in wave 2
  need nothing from each other: `Principal` (account, profile, device,
  session handle, epoch, token scope and the `PrincipalFacts`), `LibrarySet`
  (the libraries a principal may see) and the `HasLibrary` trait that
  every catalogue row implements.
- **Not in scope.** Database queries (WP-065).
- **Interface sketch.** `pub fn decide(p: &PrincipalFacts, a: Action, r: &ResourceFacts, c: &Context) -> Result<(), Denial>`;
  `pub fn may_issue(creator: &PrincipalFacts, requested: &Scope) -> Result<Scope, Denial>`.
- **Tests.** A role matrix written out literally as a table of expected
  results. Properties: a principal with no grants is denied everything
  except public actions; adding a grant never removes an allowance;
  `may_issue` never returns a scope larger than the creator's.

### WP-034 User events: envelope, clock and merge rules

- **Wave** 1 · **Size** M · **Depends on** WP-002 (accepted), WP-005,
  WP-006.
- **Owns** `crates/gunmetal-core/src/userdata/` (`crates/gunmetal-core/src/userdata/event.rs`, `crates/gunmetal-core/src/userdata/hlc.rs`,
  `crates/gunmetal-core/src/userdata/merge.rs`).
- **Serves** CLI-093, MUS-180 to MUS-184, DIS-022, DIS-023, LAT-006,
  LAT-007; API-LOG-01 to API-LOG-04; the conflict table in api-needs.md.
- **Scope.** The event envelope (event ID, hybrid logical clock, device,
  profile, schema version, typed body), the clock itself, the body types
  for R1 (play, skip, love and unlove, rate, dismiss and undo, remove play,
  settings change, document operation and snapshot, and the household
  curation operations: merge, split and alias of artists and albums, and a
  review answer) and the merge rules from api-needs.md's table as pure
  functions over event sets. The curation bodies were missing: WP-076,
  WP-107 and WP-111 all write or apply them, and none defined them.
- **Not in scope.** Storage (WP-068). The rating scale (owner decision 17
  fixes it before this package starts).
- **Interface sketch.** `pub struct Hlc { wall_ms: u64, logical: u32 }` with
  `send` and `receive`; `pub fn derive_counts(events: &[Event]) -> Counts`;
  `pub fn current_love(events: &[Event], item: ItemRef) -> bool`.
- **Tests.** Clock: receive from a node ahead, behind and equal; logical
  overflow. Merge: plays de-duplicated by ID; a removal hides a play
  whatever the arrival order; latest clock wins for loves and ratings, with
  a tie broken by device ID. Properties: merging is commutative,
  associative and idempotent over event sets (the property that makes
  offline merging safe).

### WP-035 Log segment framing

- **Wave** 1 · **Size** M · **Depends on** WP-002 (accepted), WP-004.
- **Owns** `crates/gunmetal-core/src/logframe.rs`.
- **Serves** ADM-078, ADM-058; LAT-007.
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
- **Serves** MUS-020, DIS-085, DIS-101, CLI-040.
- **Scope.** Search folding (case, diacritics through canonical
  decomposition, width, ligatures, punctuation), sort keys that honour
  sort-name tags and natural number order, leading-article handling with a
  per-language list, and the letter used by the alphabet jump.
- **Interface sketch.** `pub fn fold(s: &str) -> String`; `pub fn sort_key(display: &str, sort_tag: Option<&str>, lang: Lang) -> SortKey`;
  `pub fn jump_letter(k: &SortKey) -> JumpLetter`.
- **Tests.** "Björk" and "Bjork"; "AC/DC"; "The The"; "Track 2" before
  "Track 10"; Japanese, Greek and Cyrillic titles; an empty string; a title
  of only punctuation. Property: `sort_key` order is a total order and
  stable under folding.

### WP-037 Artwork placeholder and palette

- **Wave** 1 · **Size** M · **Depends on** WP-001.
- **Owns** `crates/gunmetal-core/src/imagedata.rs`.
- **Serves** LIB-142, MUS-110; API-SYNC-05, API-CAT-07.
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
- **Serves** ACC-053, ACC-001, ACC-064; API-AUTH-01, API-AUTH-05.
- **Scope.** TOTP (RFC 6238) with a window and replay guard input; recovery
  codes and the setup claim code in the grouped format with a checksum
  character and ambiguity folding (operations-and-incident-response.md,
  section 1).
- **Interface sketch.** `pub fn totp(secret: &[u8], t: Timestamp, step: u32, digits: u8) -> u32`;
  `pub fn verify_totp(..) -> Result<u64, OtpError>` returning the used step;
  `pub fn parse_code(s: &str) -> Result<Code, CodeError>`.
- **Tests.** RFC 6238 appendix test vectors for SHA-1; the step before and
  after the window; a reused step refused; codes typed with O for 0 and
  lower case; a wrong checksum.

### WP-039 Wire codec

- **Wave** 1 · **Size** M · **Depends on** WP-004, WP-006.
- **Owns** `crates/gunmetal-core/src/wire.rs`.
- **Serves** SEC-MED-023, SEC-MED-077, CLI-032; API-SYS-03, API-SYNC-01,
  API-SYNC-02.
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
- **Serves** API-CAT-01 to API-CAT-10 (the types); LAT-001, LAT-002,
  LAT-008; the synced fields behind MUS-051, MUS-054, MUS-056, MUS-060,
  DIS-102 and LIB-146 (artist and album aggregates, sort fields, the genre
  index and technical fields, which the client reads from the synced
  copy).
- **Scope.** The shared vocabulary of the music model, as plain data:
  `TrackTags` (every canonical tag the mappers fill, multi-valued where the
  feature map says so), `Credit` and `Role`, `ReleaseType`, `TechInfo`
  (codec, container, sample rate, bit depth, channels, bitrate, duration),
  `Trim`, `GainTags`, `ArtworkRef`, `LyricsSource`, `FileFacts` (what a
  probe returns), and the synced-library record types (track, album,
  release group, artist) with their field IDs for the rule language and
  search, and `CatalogChange` (what changed: kind, ID, upsert or removal),
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
- **Serves** ACC-050, SEC-IAM-018 to SEC-IAM-021; API-AUTH-03, API-AUTH-04.
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

- **Wave** 1 · **Size** M · **Depends on** WP-007, WP-122.
- **Owns** `crates/gunmetal-store/` (creates the crate: `crates/gunmetal-store/Cargo.toml`,
  `crates/gunmetal-store/src/lib.rs`, `crates/gunmetal-store/src/writer.rs`, `crates/gunmetal-store/src/readers.rs`, `crates/gunmetal-store/src/schema.rs`).
- **Serves** ADM-080, ADM-077, ADM-058, SEC-PRV-050.
- **Scope.** Open the cache with the pragmas in this plan; the single
  writer thread; the reader pool; schema registration by parts and the
  digest; discard-and-rebuild when the digest or generation changes; a
  write-queue depth counter for Admin > Diagnostics.
- **Not in scope.** Any domain table.
- **Interface sketch.** `pub struct Store; impl Store { pub fn open(dir: &Path, parts: &[SchemaPart]) -> Result<Opened, StoreError>; pub async fn write<R: Send + 'static>(&self, f: impl FnOnce(&Transaction) -> Result<R, StoreError> + Send + 'static) -> Result<R, StoreError>; pub async fn read<R>(&self, f: impl FnOnce(&Connection) -> R + Send + 'static) -> Result<R, StoreError>; }`
  where `Opened` says whether the cache was fresh, reused or rebuilt.
- **Tests (real SQLite).** Opening twice reuses; changing one part's SQL
  causes a rebuild; a corrupt file (bytes overwritten in the test) causes a
  rebuild rather than a crash; readers see committed data and never a
  half-written batch; a hundred concurrent writes from tasks are
  serialised with no "database is locked" error; `secure_delete` is on.
  Load test (kept short in CI): readers' latency while a long batch
  commits stays under a stated bound.

### WP-043 Server skeleton, command line and configuration

- **Wave** 1 · **Size** M · **Depends on** WP-005, WP-006, WP-007.
- **Owns** `crates/gunmetal-server/` (creates the crate: `crates/gunmetal-server/Cargo.toml`,
  `crates/gunmetal-server/src/main.rs`, `crates/gunmetal-server/src/lib.rs`, `crates/gunmetal-server/src/cli.rs`, `crates/gunmetal-server/src/config.rs`,
  `crates/gunmetal-server/src/datadir.rs`, `crates/gunmetal-server/src/app.rs`, `crates/gunmetal-server/src/bus.rs`,
  `crates/gunmetal-server/src/clock.rs`, `crates/gunmetal-server/src/log.rs`).
  The route registry and the HTTP listener moved to WP-118 in review: they
  need `gunmetal-http` (WP-044), which is in this same wave.
- **Serves** ADM-001, ADM-006, ADM-007, ADM-079, ADM-090, ADM-119 (the
  logger), SEC-OPS-012; API-SYS-07 (the hooks).
- **Scope.** The `gunmetal` binary and its subcommands (`serve`, `doctor`,
  `admin recover`, `migrate`, `rebuild`, `snapshot restore`, and the hidden
  worker entry), parsed by hand; the configuration file and environment,
  including the `_FILE` rule and refusing plain secret variables; the data
  directory layout and its mode and owner checks; refusing to run as root
  and refusing a data directory on a network filesystem, both as pure
  checks over probed host facts; application state; the subcommand
  dispatch table that later packages extend by one line each; an
  in-process event bus; the system clock; the structured logger.
- **Not in scope.** Any route or feature.
- **Interface sketch.** `pub struct AppState { /* one line per module, appended by later packages */ }`;
  `pub fn parse_args(args: &[OsString]) -> Result<Command, UsageError>`;
  `pub fn load_config(text: &str, env: &Env) -> Result<Config, ConfigError>`.
- **Tests.** Every subcommand and every malformed argument; a config with an
  unknown key (refused with its name); a secret in a plain variable
  (refused, message shows the `_FILE` form); a data directory with group
  read permission (refused with the exact `chmod` hint); a probed UID of 0
  (refused unless the documented override) and a probed network
  filesystem type (refused), passed to the checks as inputs; the logger escaping CR, LF,
  ANSI escapes and capping length. Integration: `gunmetal serve` on an
  empty directory creates the layout with the right modes.

### WP-044 HTTP foundation

- **Wave** 1 · **Size** L · **Depends on** WP-006.
- **Owns** `crates/gunmetal-http/` (creates the crate).
- **Serves** ACC-120, ACC-123, ACC-124 (cookie handling primitives),
  INT-001, INT-007, INT-023; SEC-API pipeline and headers; API-SYS-09.
- **Scope.** The `RouteSpec` table type and a router built only from it;
  the request pipeline in web-and-api-security.md's order (connection and
  size limits, Host allow-list, exact route match and method check,
  credential extraction from headers and cookies only, request-forgery
  checks for cookie credentials, access-class check hook, rate-limit hook,
  content-type and body decoding with per-route caps), the response-header
  layer, the problem renderer, the list paging convention, and an
  in-process test client.
- **Not in scope.** Sessions, principals and authorisation decisions
  (WP-062, WP-065), which plug into the hooks.
- **Interface sketch.** As in web-and-api-security.md: `pub enum Access`,
  `pub struct RouteSpec`; plus `pub fn router(specs: &[RouteEntry], hooks: Hooks) -> axum::Router`
  and `pub struct TestClient`.
- **Tests.** The route-table tests the security baseline asks for: every
  route appears in the table and nowhere else; an unlisted public route
  fails a test; wrong method gives 405 with `Allow`; every response,
  including errors and 404, carries the security headers; bodies over the
  cap give 413 before decoding; a credential in a query string is refused;
  a `Host` not on the allow-list is refused; cookie requests without the
  `Gunmetal-Request` header are refused for mutating routes.

### WP-045 Worker sandbox

- **Wave** 1 · **Size** M · **Depends on** WP-006.
- **Owns** `crates/gunmetal-worker/` (creates the crate: `crates/gunmetal-worker/Cargo.toml`,
  `crates/gunmetal-worker/src/lib.rs`, `crates/gunmetal-worker/src/sandbox/`).
- **Serves** SEC-MED-021, SEC-MED-022, SEC-MED-024, SEC-MED-063.
- **Scope.** Spawning the worker as the same binary with a cleared
  environment and only the IPC socket open; inside the worker, in order:
  rlimits and `PR_SET_DUMPABLE` 0, closing other descriptors,
  `no_new_privs`, Landlock with no filesystem rules, seccomp with the
  allowlist; a self-test that reports the isolation tier reached; a
  reduced tier with the reason on kernels without Landlock or on 32-bit
  ARM.
- **Not in scope.** The protocol (WP-061), the pool (WP-078).
- **Interface sketch.** `pub fn spawn_worker(exe: &Path) -> Result<WorkerChild, SpawnError>`;
  `pub fn confine(profile: Profile) -> Result<Tier, ConfineError>`;
  `pub fn self_test() -> TierReport`.
- **Tests.** Integration on Linux CI with test hooks behind a cargo feature:
  a confined worker that tries to open a path is killed or refused; one
  that tries a network socket is refused; one that exceeds its memory limit
  dies and the parent sees the reason; the environment is empty. Unit: the
  tier table from SEC-MED-024 as a pure function of what was enforced.
  The worker crate has no binary of its own, so the tests spawn the test
  executable itself (`std::env::current_exe()`) with a hidden argument that
  enters the child path; no extra binary ships. The gate must build with
  the test-hook feature for these tests to run at all, which is a gate
  change request to the integrator, not an edit to `gate.sh`.
- **Owns, in detail.** The syscall allowlist lives in its own file,
  `crates/gunmetal-worker/src/sandbox/syscalls.rs`, so that its ownership
  can pass to WP-079 in wave 3.
- **Risks.** CI runners' kernels decide which Landlock ABI is testable
  (unverified for GitHub's runners). The seccomp allowlist is derived from
  running real jobs in audit mode. As first planned, this package could
  only finish after WP-079 in a later wave, which no package may wait for.
  It now ships the filter mechanism and a minimal allowlist proven against
  the host-loop stub; WP-079 takes over `sandbox/syscalls.rs` and extends
  it from audit runs of the probe and artwork jobs.

### WP-046 Identity store

- **Wave** 1 · **Size** M · **Depends on** WP-002 (accepted), WP-005,
  WP-007, WP-122.
- **Owns** `crates/gunmetal-durable/` (creates the crate: `crates/gunmetal-durable/Cargo.toml`,
  `crates/gunmetal-durable/src/lib.rs`, `crates/gunmetal-durable/src/identity/`).
- **Serves** ACC-013, ADM-056, ADM-057, ADM-058; INT-008.
- **Scope.** Open `durable/identity.db` with `synchronous=FULL`; schema by
  parts before R1, numbered migrations after; the migration runner with
  snapshot, integrity check, invariants and refusal of newer versions; the
  public-ID mapping table and its API.
- **Not in scope.** Accounts, sessions and other tables, which their
  packages register as parts.
- **Interface sketch.** `pub struct IdentityStore; impl IdentityStore { pub fn open(dir: &Path, parts: &[SchemaPart], invariants: &[Invariant]) -> Result<Self, IdentityError>; pub async fn write<R>(..); pub async fn read<R>(..); }`.
- **Tests (real SQLite).** A migration that violates an invariant rolls back
  and leaves the snapshot; a file from a "newer" version (built in the
  test) is refused with the restore command in the message; the snapshot
  passes `integrity_check`; a crash simulated between snapshot and commit
  leaves a usable file.

### WP-047 Secrets and keys

- **Wave** 1 · **Size** M · **Depends on** WP-005.
- **Owns** `crates/gunmetal-secrets/` (creates the crate).
- **Serves** SEC-OPS-012, SEC-OPS-013, ACC-123, ACC-122 (key rotation).
- **Scope.** Generate and load the root secret with mode checks; derive
  purpose keys with HKDF and the documented labels; the key ring for
  URL-signing keys with 24-hour rotation and overlap; `Secret<T>` with no
  `Display`, `Serialize` or `PartialEq`, a fixed `Debug`, zeroising on drop
  and `expose()` reachable only here; the vault (AEAD with the record ID as
  associated data). Keyed operations happen inside this crate:
  `root.key_ring(purpose).mac(kid, msg)` returns an HMAC-SHA-256 tag, so the
  server signs capability URLs, hashes session tokens and applies the
  password pepper (as a MAC over the password before Argon2id, WP-063)
  without any key leaving the crate. This replaces an earlier sketch in
  which core functions took key bytes, which would have needed `expose()`
  in the server.
- **Interface sketch.** `pub struct Root; impl Root { pub fn load_or_create(dir: &Path) -> Result<Self, SecretsError>; pub fn key_ring(&self, purpose: Purpose) -> KeyRing; }`;
  `impl KeyRing { pub fn current_kid(&self) -> u8; pub fn mac(&self, kid: u8, msg: &[u8]) -> Option<[u8; 32]>; }`;
  `pub struct Vault; impl Vault { pub fn seal(&self, id: &[u8], plain: &[u8]) -> Vec<u8>; pub fn open(&self, id: &[u8], sealed: &[u8]) -> Result<Secret<Vec<u8>>, SecretsError>; }`.
- **Tests.** HKDF against RFC 5869 vectors; `KeyRing::mac` against RFC 4231
  HMAC-SHA-256 vectors with a test root; rotation keeps the previous kid
  answerable for the overlap and refuses it after; vault round trip; a sealed blob
  moved to another record ID fails; a root file with mode 0644 refused;
  `format!("{:?}", secret)` prints the placeholder. The canary test from
  SEC-OPS-013 is set up here and extended by every later package that
  handles a secret.

### WP-048 Egress client

- **Wave** 1 · **Size** M · **Depends on** WP-005.
- **Owns** `crates/gunmetal-egress/` (creates the crate).
- **Serves** ACC-113, LIB-108, ADM-129, SEC-API-076; API-SET-02.
- **Scope.** The only outbound HTTP client: requests allowed only for a
  purpose the owner has granted, destination checks against private and
  link-local ranges (SSRF guard, using WP-005's address classifier), DNS
  results checked before connecting, response
  size and time caps, a fixed `User-Agent`, and a record of every
  connection for the network activity page.
- **Not in scope.** Any caller (OIDC, updates, alerts).
- **Interface sketch.** `pub struct Egress; impl Egress { pub async fn get(&self, purpose: Purpose, url: &Url) -> Result<Response, EgressError>; pub fn activity(&self) -> Vec<Connection>; }`.
- **Tests.** Against a local test server bound in the test. The resolver
  and the address policy are inputs, so a test can grant loopback to reach
  its own server for the cases that should succeed and leave it ungranted
  for the cases that should be refused; a test DNS map sends allowed names
  to private and metadata addresses (SEC-TM-048). Cases: a purpose not
  granted is refused without a connection; a name resolving to 127.0.0.1 or
  169.254.169.254 without a grant is refused; a redirect to a private address is refused;
  a body over the cap is cut; every attempt, refused or not, appears in the
  activity record.

## Wave 2: tag mapping, probing, search and the server's spine

Four packages numbered in this section now run in wave 3 because of
same-wave dependencies found in review: WP-059 (needs WP-058) and WP-072,
WP-073 and WP-074 (they register routes, so they need WP-118). They stay
here so their numbers keep their place; their own entries give the wave.
WP-118 and WP-119, added in review, are at the end of this section.

### WP-049 ID3 tag mapping

- **Wave** 2 · **Size** M · **Depends on** WP-010, WP-011, WP-040.
- **Owns** `crates/gunmetal-core/src/tags/id3.rs` (`tags/mod.rs` is a
  registry file shared with WP-050 and WP-051).
- **Serves** MUS-001 to MUS-005, MUS-010 to MUS-013, MUS-017, MUS-019,
  MUS-020, MUS-034, MUS-036, MUS-047, MUS-084, LIB-059; API-CAT-01 to
  API-CAT-03.
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
  inputs), LIB-059, LIB-193, SEC-MED-017; API-CAT-04, API-CAT-05,
  API-CAT-11.
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
- **Serves** MUS-001 to MUS-006, MUS-035, LIB-187; API-CAT-01, API-LIB-07.
- **Scope.** Turn tagged artist strings and multi-value fields into linked
  credits while keeping the display credit exactly as tagged: separator
  rules (", ", " & ", " feat. ", " ft. ", " x ", " / ", "; ") with
  per-library exceptions (an artist whose name contains a separator, such
  as a duo with an ampersand in its name), multi-value fields taking
  precedence over splitting, MusicBrainz artist IDs pairing with names by
  position, and same-name artists kept apart when their IDs differ.
  Artist merge and alias overrides from the curation log (WP-034's
  curation bodies) apply on top when credits are resolved to artists.
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
- **Serves** DIS-083 to DIS-089, DIS-091, MUS-061, DIS-084; API-HOME-05,
  API-SYNC-07 (the fallback path, if needed).
- **Scope.** An in-memory index built on the device from the synced
  library: folded prefix and token matching, a small typo tolerance,
  fields (title, artist, album, composer and other roles, genre, mood,
  label), scopes (all, artists, albums, tracks, playlists), ranking that
  prefers exact and prefix matches and the person's own plays, and a
  compact serialised form so the server can ship a prebuilt segment if the
  device misses its budget.
- **Not in scope.** Recent searches, which stay on the device (client).
- **Interface sketch.** `pub struct Index; impl Index { pub fn build(items: impl Iterator<Item = SearchDoc>) -> Self; pub fn query(&self, q: &str, scope: Scope, limit: u16) -> Vec<Hit>; pub fn to_bytes(&self) -> Vec<u8>; pub fn from_bytes(b: &[u8]) -> Result<Self, IndexError>; }`.
- **Tests.** Diacritics, case, a missing letter, a swapped pair, a query of
  one character, a query of 256 characters and 17 terms (capped), a scope
  filter, people by role. Each example asserts the exact ordered hit list
  for a small hand-built index. Properties: a document whose title is
  unique in the index is the first hit for a query of that exact title
  with a limit of 1 (a bare "is found" would pass for an index that
  returned everything); adding documents never removes an existing exact
  hit; the serialised form round-trips. Build time and memory at 100,000
  synthetic tracks for the DIS-019 budget are recorded by the benchmark
  runner (WP-115), not by a test, and stay reported rather than asserted
  until the owner names the reference device (owner decision 15).

### WP-055 Playback decision engine

- **Wave** 2 · **Size** S · **Depends on** WP-040.
- **Owns** `crates/gunmetal-core/src/decision.rs`.
- **Serves** MUS-099, MUS-229, ADM-100, INT-134; API-CAT-09, API-SES-01.
- **Scope.** For music in R1: given a track's technical facts and a device
  capability report, return play directly, play through the packager, or
  cannot play here, with a structured reason list the badge and the admin
  session view both render.
- **Not in scope.** Video, remux and transcode decisions (R2).
- **Interface sketch.** `pub fn decide_audio(t: &TechInfo, d: &DeviceCaps) -> Decision`;
  `pub enum Decision { Direct, Packaged(PackageFormat), CannotPlay(Vec<Reason>) }`.
- **Tests.** Each core format against a capability report that supports it,
  lacks it, supports the codec but not the container, and supports it only
  in Media Source Extensions; ALAC in a browser that cannot decode it
  ("Cannot play here: this browser cannot decode ALAC").

### WP-056 Audio packager (conditional on ADR 4)

- **Wave** 2 · **Size** L · **Depends on** WP-003 (ADR 4 accepted), WP-012,
  WP-013, WP-015, WP-016, WP-017, WP-019.
- **Owns** `crates/gunmetal-core/src/package/`,
  `crates/gunmetal-testkit/src/fmp4.rs`.
- **Serves** MUS-230, MUS-067, MUS-069, MUS-070; API-STR-04.
- **Scope.** Write an initialisation segment and media segments of
  fragmented MP4 audio from frame indexes, copying FLAC frames (`dfLa`),
  Opus packets (`dOps`, keeping pre-skip) and MP3 frames, with edit lists
  or equivalent trim signalling so gapless survives. Segment boundaries
  come from the index, so any segment can be produced independently.
- **Not in scope.** Serving (WP-105). Video (R2).
- **Interface sketch.** `pub fn init_segment(track: &PackTrack) -> Vec<u8>`;
  `pub fn media_segment(track: &PackTrack, index: &FrameIndex, n: u32, source: &[u8]) -> Result<Vec<u8>, PackError>`
  where `source` is the byte range the server read for that segment.
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

### WP-057 Import parsers and matcher

- **Wave** 2 · **Size** M · **Depends on** WP-036, WP-040, WP-005.
- **Owns** `crates/gunmetal-core/src/import/`.
- **Serves** ADM-042, ADM-043, ADM-044, MUS-189, INT-107; API-USR-06,
  API-SET-11, API-PL-03.
- **Scope.** Parsers for the Last.fm and ListenBrainz export files people
  can download (the exact formats must be confirmed against each service's
  current export and are unverified here), and one matcher that maps
  (artist, album, title, duration, MBIDs) to library items with a
  confidence, a reason and an "unmatched" result. Imported plays are
  marked as imported so a scrobbler never resends them.
- **JSON in the core.** Some export files are JSON (unverified for each
  service's current export). The core's proposed dependencies have no JSON
  reader. **Proposal:** the server decodes the file's JSON or CSV into
  plain rows with `serde_json`, under a size cap, and this package's
  parsers take those rows and validate every field into typed values, so
  untrusted values still become domain types in the core (SEC-TM-031).
  The alternative, `serde_json` in the core, is part of owner decision 4.
- **Interface sketch.** `pub fn match_track(q: &MatchQuery, index: &MatchIndex) -> MatchResult`.
- **Tests.** Exact MBID match; title with "(Remastered 2011)"; differing
  case and diacritics; two candidates with equal scores (unmatched, not a
  guess); a duration off by more than the tolerance. Property: in a
  library where no two items share artist, album, title and duration, an
  item always matches a query built from its own tags, and the match is
  that item. (As first written, the property ignored duplicates, for which
  the rules above require "unmatched".)

### WP-058 Neighbour table and radio

- **Wave** 2 · **Size** M · **Depends on** WP-005, WP-026 (the
  `RecentPlays` type), WP-034 (household co-listening is read from play
  events), WP-040.
- **Owns** `crates/gunmetal-core/src/radio/`.
- **Serves** DIS-060, DIS-067, DIS-070, MUS-165, DIS-062; API-SYNC-06.
- **Scope.** Compute each track's, album's and artist's top N neighbours
  from shared credits, genres, era and household co-listening within a
  size budget (server side, as a job in WP-113), and, on the device, pick
  radio tracks from a seed with seeded randomness, avoiding recent plays,
  with a reason label for every pick.
- **Interface sketch.** `pub fn neighbours(lib: &LibraryView, colisten: &CoListen, budget: Budget) -> NeighbourTable`;
  `pub fn radio(seed: Seed, table: &NeighbourTable, recent: &RecentPlays, rng_seed: u64, n: u16) -> Vec<Pick>`.
- **Tests.** A tiny library where neighbours are obvious and written out by
  hand; cold start with no plays; a seed with no neighbours (falls back to
  genre, labelled); the size budget enforced. Properties: when the table
  holds at least `n` eligible candidates, exactly `n` picks come back; no
  pick repeats within the window; every pick carries a reason. (Without the
  first, an empty result would satisfy the other two.)

### WP-059 Home rows

- **Wave** 3 (moved from 2 in review) · **Size** M · **Depends on**
  WP-027, WP-034, WP-040, WP-058. "Because you played" (DIS-061) reads the
  neighbour table, and WP-058 was in the same wave.
- **Owns** `crates/gunmetal-core/src/home/`.
- **Serves** DIS-001, DIS-003, DIS-004, DIS-009, DIS-012, DIS-015, DIS-020,
  DIS-021, DIS-035, DIS-036, DIS-038, DIS-046, DIS-061, DIS-071, MUS-049,
  MUS-050, MUS-059; API-HOME-01 to API-HOME-04, API-LOG-04.
- **Scope.** The default Home layout and the built-in row sources
  (continue listening by album or playlist, recently played, recently
  added grouped by album and ignoring upgrades, loved songs, "because you
  played", your top tracks by an artist), each evaluated on the device
  from the synced library and the person's events, each with a reason, and
  rule-backed custom rows through WP-027.
- **Interface sketch.** `pub fn evaluate_row(row: &RowSpec, lib: &LibraryView, mine: &MyEvents, now: Timestamp) -> Row`.
- **Tests.** An upgrade (same identity, new file) does not appear in
  recently added; a dismissed item stays out of continue listening until
  undone; a library marked "keep off Home" never appears; empty library
  rows give the designed empty state value. Property: row evaluation is
  deterministic for a given input and `now`.

### WP-060 Filesystem roots, opening and walking

- **Wave** 2 · **Size** L · **Depends on** WP-024, WP-005.
- **Owns** `crates/gunmetal-fs/` (creates the crate: `crates/gunmetal-fs/Cargo.toml`, `crates/gunmetal-fs/src/lib.rs`,
  `crates/gunmetal-fs/src/root.rs`, `crates/gunmetal-fs/src/open.rs`, `crates/gunmetal-fs/src/walk.rs`, `crates/gunmetal-fs/src/fingerprint.rs`,
  `crates/gunmetal-fs/src/pool.rs`).
- **Serves** SEC-MED-033 to SEC-MED-038, SEC-MED-040, SEC-MED-041, LIB-006,
  LIB-007, LIB-016, ADM-089; API-LIB-02, API-LIB-03.
- **Scope.** Root handles opened once per configured root; opening beneath
  a root with `O_NONBLOCK`, `O_NOCTTY` and `O_CLOEXEC`; checking the file
  type on the open handle; the symlink policy; walking with exclusions and
  raw-byte names; fingerprints (size, modification time, file ID or inode,
  a short hash of the first and last few kilobytes) and directory
  summaries for no-change short-circuits; the identity check before
  serving bytes (SEC-MED-036); a bounded blocking pool per root that
  pauses a root after two lost workers.
- **Not in scope.** Watching (WP-098). Any database.
- **Interface sketch.** `pub struct Root; impl Root { pub fn open(path: &Path, policy: LinkPolicy) -> Result<Self, FsError>; pub fn open_file(&self, rel: &RelPath) -> Result<MediaFile, FsError>; pub fn walk(&self, excl: &Exclusions) -> Walk; }`;
  `pub fn fingerprint(f: &MediaFile) -> Result<Fingerprint, FsError>`.
- **Tests (real filesystem in a temporary directory).** A FIFO named
  `track.flac` and a socket named `cover.jpg` are skipped without blocking;
  a symlink inside the root followed, one to another library refused and
  listed, one to `/etc/passwd` refused; a hard link to a file outside the
  root (refused by the identity rules, listed); non-UTF-8 names and names
  with newlines; a directory that becomes unreadable mid-walk; a file
  swapped after fingerprinting is caught by the serve check. The
  disallowed-methods lint keeps `std::fs` out of every other crate.
- **Risks.** Windows and macOS behaviour of `cap-std` beneath-root opens
  needs its own tests when those server builds are planned (owner
  decision 18).

### WP-061 Worker IPC and host loop

- **Wave** 2 · **Size** M · **Depends on** WP-004, WP-039, WP-045.
- **Owns** `crates/gunmetal-worker/src/ipc.rs`, `crates/gunmetal-worker/src/host.rs`.
- **Serves** SEC-MED-018, SEC-MED-020, SEC-MED-023, SEC-MED-010.
- **Scope.** The request and response messages between server and worker as
  core wire frames over the socket pair; passing the media file and
  resolved sidecars as descriptors with `SCM_RIGHTS`; the worker's host
  loop that drives any sans-I/O parser by answering read requests with
  `pread`, enforcing the 16 MiB request and per-file caps; the server side
  that revalidates every response.
- **Not in scope.** Which jobs exist (WP-079), supervision (WP-078).
- **Interface sketch.** `pub enum Job { Probe { hint: Option<String> }, Artwork { sizes: Vec<Size> }, HashWindow { range: Range<u64> } }`;
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

- **Wave** 2 · **Size** M · **Depends on** WP-031, WP-033, WP-043, WP-044,
  WP-046, WP-047.
- **Owns** `crates/gunmetal-server/src/session/`.
- **Serves** ACC-124, ACC-079, ACC-065, ACC-068, ACC-069, ACC-070, ACC-008;
  API-AUTH-08, API-AUTH-09, API-DEV-01.
- **Scope.** Browser sessions in `__Host-` cookies with remembered and
  shared-computer lifetimes; session tokens stored only as keyed hashes; a
  per-account and per-device epoch that every session and signed URL
  carries; bumping it on password change, sign out everywhere, disabled
  account and revoked device; an in-memory revocation cache invalidated by
  the write that changes it; the principal extractor that plugs into
  WP-044's pipeline and yields the core's `authz::Principal` (WP-033).
  Session tokens are hashed through the secrets crate's `KeyRing::mac`.
- **Interface sketch.** `pub async fn issue(store: &IdentityStore, account: AccountId, device: DeviceInfo, kind: Lifetime) -> Result<(Cookie, Principal), SessionError>`;
  `pub async fn authenticate(req: &RequestParts) -> Result<Principal, ApiError>`;
  `pub async fn bump_epoch(scope: EpochScope) -> Result<(), SessionError>`.
- **Tests (real SQLite).** A session expires at its lifetime with a manual
  clock; a bumped epoch fails the next request immediately; revoking one
  device leaves the others; the cookie has `Secure`, `HttpOnly`,
  `SameSite` and the `__Host-` prefix; a token in the database is never
  the raw token (the canary test).

### WP-063 Passwords, two-factor and recovery codes

- **Wave** 2 · **Size** M · **Depends on** WP-038, WP-043 (it lives in the
  server crate, which this package did not list as first written),
  WP-046, WP-047.
- **Owns** `crates/gunmetal-server/src/password/`.
- **Serves** ACC-052, ACC-053, ACC-007; API-AUTH-05, API-USR-03 (storage
  side).
- **Scope.** Argon2id hashing with the parameters the security baseline
  sets and a pepper outside the database; TOTP enrolment and verification
  with replay protection; recovery codes stored hashed and single use;
  uniform errors and timing for unknown users and wrong passwords.
- **Interface sketch.** `pub async fn set_password(..)`, `pub async fn verify(..) -> Result<Verified, SignInError>`.
- **Tests (real SQLite).** Correct, wrong and unknown-user attempts return
  the same error; an unknown user runs exactly one Argon2id verification
  against a fixed dummy hash with the same parameters, observed through a
  counting hook on the hasher (a statistical timing test, as first
  planned, would flake in CI and could not kill mutants reliably); a used
  recovery code fails the second time; a TOTP step reused fails; the
  stored hash never contains the password (canary). The pepper is applied
  as `KeyRing::mac` over the password before hashing, never by reading
  the pepper bytes here.
- **Not in scope.** The sign-in and sign-out routes (WP-120, wave 3):
  routes need WP-118, which is in this wave.

### WP-064 Guessing limiter

- **Wave** 2 · **Size** S · **Depends on** WP-032, WP-043.
- **Owns** `crates/gunmetal-server/src/limiter/`.
- **Serves** ACC-063, ADM-122, INT-012; API-AUTH-07.
- **Scope.** One limiter keyed by account, address and code type for
  passwords, two-factor codes, setup codes and invite codes; log lines in
  the fail2ban-friendly format; no permanent lockout.
- **Tests.** The back-off schedule with a manual clock; a different account
  from the same address; the exact log line format.

### WP-065 Authorisation layer

- **Wave** 2 · **Size** M · **Depends on** WP-033, WP-042, WP-043, WP-044,
  WP-046.
- **Owns** `crates/gunmetal-server/src/access/`.
- **Serves** ACC-121, ACC-030, ACC-037, SEC-API-010, SEC-API-011,
  SEC-MED-051.
- **Scope.** The library grants table (a schema part in the identity
  store, where ADR 3 puts grants) and the API to read and change grants,
  which WP-099 calls; building the `LibrarySet` a principal may see; the
  `Visible<T>` and `Editable<T>` wrappers whose constructors only this
  module can call, produced by a generic `check<T: HasLibrary>` that any
  row type implementing the core trait can pass through; the batch rule
  (reject the whole request if any input is not visible); and the
  per-recipient fan-out check for pushed events.
- **Why it changed in review.** As first written, this package owned the
  `Principal` type (which WP-062, in the same wave, must produce),
  returned catalogue rows (which WP-067, in the same wave, defines) and
  relied on a grants table that only WP-099, a wave later, created.
  `Principal`, `LibrarySet` and `HasLibrary` now live in the core
  (WP-033); catalogue readers in WP-067 take a `&LibrarySet` argument and
  have no unfiltered form; and the grants table is owned here.
- **Interface sketch.** `pub struct Visible<T>(T);`
  `pub fn library_set(p: &Principal, grants: &Grants) -> LibrarySet;`
  `pub fn check<T: HasLibrary>(set: &LibrarySet, row: Option<T>) -> Result<Visible<T>, ApiError>;`
  `pub async fn grant(..)`, `pub async fn revoke(..)`.
- **Tests (real SQLite).** With a fixture row type and a test schema part
  holding rows in two libraries: a row in a library the caller cannot see
  gives exactly the same 404 body as a nonexistent ID; a batch with one
  hidden item is rejected whole; revoking a grant takes effect on the next
  request; a compile-fail test (doc test) shows `Visible::new` is not
  callable outside the module. The same 404 test against real catalogue
  rows runs in WP-082 and WP-084, which have both pieces.
- **Risks.** `LibrarySet` has a public constructor in the core, so the rule
  "only the access module builds one from a principal" is a convention
  plus review, not a type guarantee (unresolved; see Review notes).

### WP-066 Change log

- **Wave** 2 · **Size** M · **Depends on** WP-006, WP-040 (`CatalogChange`),
  WP-042.
- **Owns** `crates/gunmetal-store/src/changelog.rs`,
  `crates/gunmetal-store/src/changelog.sql`.
- **Serves** LIB-018, INT-006, CLI-022, CLI-024; API-SYNC-02, API-TOK-02.
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
- **Scope.** Tables for files, tracks, albums, release groups, artists,
  credits, artwork records, lyrics, technical data, availability, and the
  per-file seek and frame indexes (which the packaging route checks
  segment numbers against and the seek-index fetch in WP-084 serves; as
  first written, no package stored them), with batch upserts that touch a
  row only when it really changed and return the list of
  `CatalogChange`s (so a no-change rescan writes nothing), and readers for
  the sync builder and the file inspector. Every reader that returns
  library content takes a `&LibrarySet` (WP-033) and filters in SQL; there
  is no unfiltered reader except the admin-only inspector's, which takes
  an admin proof instead. Every row type implements `HasLibrary`. The
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
  `LibrarySet` without a row's library never returns that row. Commit time
  for 500 files is recorded by the benchmark, not asserted in a test.

### WP-068 User log service

- **Wave** 2 · **Size** M · **Depends on** WP-034, WP-035, WP-046.
- **Owns** `crates/gunmetal-durable/src/userlog/`.
- **Serves** ADM-078, CLI-093, LAT-007, MUS-182, MUS-184; API-LOG-01,
  API-LOG-02.
- **Scope.** One writer task per data directory that appends events to the
  right stream and segment, de-duplicates by event ID, `fsync`s at the
  policy ADR 3 sets, recovers torn tails at startup and reports damage,
  replays streams for projections and for cache rebuilds, and performs the
  sanctioned erasure rewrite (per event, per range, per profile).
- **Interface sketch.** `pub struct UserLog; impl UserLog { pub async fn append(&self, e: Event) -> Result<Appended, LogError>; pub fn replay(&self, stream: StreamId) -> Replay; pub async fn erase(&self, sel: Erasure) -> Result<ErasureReport, LogError>; }`.
- **Tests (real files).** A torn final record (the test truncates the file)
  is cut back at reopen and reported; a corrupt middle record is reported
  with its range and the rest replays; the same event appended twice is
  stored once; a month boundary starts a new segment; an erasure leaves no
  trace of the erased payload in the segment bytes. Power-loss behaviour
  beyond what truncation simulates is out of reach in CI and stated as
  such.

### WP-069 Audit log

- **Wave** 2 · **Size** M · **Depends on** WP-046, WP-005.
- **Owns** `crates/gunmetal-durable/src/audit/`.
- **Serves** ADM-110 (the security part), ACC-078, SEC-OPS-020;
  API-SCAN-05, API-USR-04.
- **Scope.** Append-only JSON-lines segments of 16 MB, one canonical object
  per line, `fsync` before critical actions return and refusal of the
  action if it fails, the event name catalogue, and readers filtered by
  person for "your sign-in history".
- **Tests (real files).** A critical event whose write fails (read-only
  directory in the test) refuses the action; rotation at the size limit;
  control characters in a user agent are escaped; a reader for one person
  never returns another's events.

### WP-070 Task runner

- **Wave** 2 · **Size** M · **Depends on** WP-042 (checkpoints and task
  state live in the cache), WP-043.
- **Owns** `crates/gunmetal-server/src/tasks/`.
- **Serves** ADM-093, ADM-095, LIB-024; API-SCAN-04 (the engine).
- **Scope.** Named task kinds registered by modules; schedules; one-off
  requests (with de-duplication of path-scoped requests); progress,
  cancel, last run, duration and error; throttling and checkpoints so heavy
  jobs resume after a restart; expiry sweeps as a built-in schedule other
  modules register into. The R1 task kinds are a closed enum declared here
  (library scan, path refresh, backup, purge, analysis, rebuild, import
  matching, neighbour rebuild, rule re-evaluation), so a package can
  request a kind before the package that handles it has merged (WP-082
  and WP-099 request scans that WP-102 handles a wave later); a request
  for a kind with no handler stays queued and is reported as waiting.
- **Interface sketch.** `pub trait Task { fn kind(&self) -> TaskKind; async fn run(&self, ctx: TaskCtx) -> Result<Outcome, TaskError>; }`;
  `pub fn request(&self, kind: TaskKind, input: TaskInput) -> TaskHandle`.
- **Tests.** Two requests for the same path collapse into one; cancel
  stops at the next checkpoint; a task that panics is recorded as failed
  and the runner keeps going; a checkpointed task resumes from its
  checkpoint after the runner restarts (state persisted in the cache).

### WP-071 Derived-data store

- **Wave** 2 · **Size** S · **Depends on** WP-046.
- **Owns** `crates/gunmetal-durable/src/derived/`.
- **Serves** ADM-141, LIB-024, LIB-025.
- **Scope.** A SQLite file keyed by (content identity, producer kind,
  producer version) for analysis results and artwork derivatives' metadata,
  read on rebuild so work is never repeated.
- **Tests (real SQLite).** A new producer version misses the old key; a
  rebuild reads existing results; the store reports its file as optional
  for backups. That backups leave it out by default is tested in WP-090,
  which owns backups and comes a wave later.

### WP-072 Web client asset serving

- **Wave** 3 (moved from 2 in review) · **Size** S · **Depends on**
  WP-043, WP-044, WP-118. As first written it depended only on WP-044,
  but it lives in the server crate (WP-043) and registers routes (WP-118).
- **Owns** `crates/gunmetal-server/src/webapp/`.
- **Serves** CLI-001, CLI-003 (served files), SEC-API CSP rules.
- **Scope.** Serve the web client's built bundle from bytes embedded in the
  binary, with the content security policy, cache headers by content hash,
  and the service-worker scope rules. The bundle itself comes from the
  client work; tests use a two-file stand-in bundle.
- **Tests.** No directory listing; unknown paths fall back to the app shell
  only for navigation requests; hashed assets are immutable-cacheable and
  the shell is not.

### WP-073 Network settings and owner HTTPS

- **Wave** 3 (moved from 2 in review) · **Size** M · **Depends on**
  WP-023, WP-043, WP-044, WP-118 (its settings routes, and the TLS
  configuration of the listener WP-118 owns).
- **Owns** `crates/gunmetal-server/src/network/`.
- **Serves** ACC-097, ACC-098, ACC-134, ADM-022 (owner's certificate
  part), CLI-150; API-SET-01, API-SYS-02.
- **Scope.** Trusted reverse proxies and the client address they yield, a
  path prefix applied to every route and cookie path, HTTPS with the
  owner's certificate and key (reloaded on change), and the secure-context
  report for the client.
- **Tests.** Behind a trusted proxy the forwarded address is used; behind
  an untrusted one it is ignored; the prefix applies to redirects and the
  cookie path; a certificate and key that do not match are refused at
  load; the secure-context report for `localhost`, a private address over
  HTTP and a domain over HTTPS.
- **Risks.** The rustls crypto provider is owner decision 8.

### WP-074 Update check and advisories

- **Wave** 3 (moved from 2 in review) · **Size** M · **Depends on**
  WP-005, WP-043, WP-048, WP-118. As first written it lacked WP-043,
  although it lives in the server crate.
- **Owns** `crates/gunmetal-server/src/updates/`.
- **Serves** ADM-053, ADM-054, ADM-060, ACC-126, ACC-128, ACC-127
  (reference to ADM-054); API-SET-06.
- **Scope.** If the owner allowed it, fetch the signed feed once a day with
  jitter through the egress gate, verify it against the root compiled into
  the binary, refuse rollback, expired or oversized metadata, match
  advisories against the running version locally, and show "Can't confirm
  you're up to date" after seven days without a fresh feed.
- **Tests.** Against a local feed server in the test: a valid feed; a feed
  signed by the wrong key; a feed with a lower version than one already
  seen; an expired feed; a feed of 10 MB; the seven-day message with a
  manual clock. Nothing is fetched when the owner has not allowed it.
  ACC-128 (nobody can switch the server off remotely): a validly signed
  feed whose advisory or any other field asks for anything beyond
  displaying a banner changes nothing but the banner; the server keeps
  serving whatever the feed says.
- **Risks.** Full TUF through `tough` or a simpler signed document is owner
  decision 12; the feed itself does not exist yet. The root key compiled
  into the binary presumes the release signing keys exist (owner
  decision 32).

### WP-118 Route registry and HTTP listener (added in review)

- **Wave** 2 · **Size** S · **Depends on** WP-043, WP-044.
- **Owns** `crates/gunmetal-server/src/routes.rs`,
  `crates/gunmetal-server/src/listener.rs`.
- **Serves** ADM-001 (the one binary actually serving); INT-001 (the route
  table the API reference is generated from); API-SYS-09 (the source).
- **Why it exists.** WP-043 owned the route registry, but a registry of
  `gunmetal-http` route entries cannot compile before `gunmetal-http`
  exists, and WP-044 is in the same wave as WP-043.
- **Scope.** The server's route registry (one line per module, the
  registry file in the table above), building the router from it through
  WP-044, binding the configured address, graceful shutdown, and the
  `serve` subcommand's dispatch line. TLS configuration is a hook WP-073
  fills.
- **Tests.** With two stand-in modules registered in the test: both route
  sets are served; a duplicate path and method across modules fails at
  startup with both module names; `serve` on a free port answers a request
  through the full WP-044 pipeline over a real socket; shutdown completes
  in-flight requests within a stated bound.

### WP-119 Synthetic library generator (added in review)

- **Wave** 2 · **Size** M · **Depends on** WP-007, WP-010, WP-011, WP-012,
  WP-013, WP-014, WP-015, WP-016, WP-017, WP-019, WP-020.
- **Owns** `crates/gunmetal-testkit/src/library.rs`.
- **Serves** the synthetic-media rule; the test needs of WP-084, WP-092,
  WP-102, WP-110, WP-111 and WP-117; the benchmark (WP-115).
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

## Wave 3: the music model, sign-in, streaming and sync

WP-059, WP-072, WP-073 and WP-074 also run in this wave (numbered in the
wave 2 section), and WP-120 is at the end of this section. WP-088 and
WP-101, numbered here, now run in wave 4.

### WP-075 Track record derivation

- **Wave** 3 · **Size** M · **Depends on** WP-021, WP-028, WP-049, WP-050,
  WP-051, WP-052, WP-053.
- **Owns** `crates/gunmetal-core/src/music/track.rs`.
- **Serves** MUS-021, MUS-034, MUS-037, MUS-047, MUS-084, MUS-154 to
  MUS-156, LIB-059, LIB-097, LIB-098; API-CAT-01 to API-CAT-08.
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

### WP-076 Album and release grouping

- **Wave** 3 · **Size** L · **Depends on** WP-034 (curation bodies),
  WP-040, WP-053.
- **Owns** `crates/gunmetal-core/src/music/albums.rs`.
- **Serves** MUS-008, MUS-010, MUS-011, MUS-012, MUS-013, MUS-014, LIB-045,
  LIB-051, LIB-098, LIB-099, LIB-187; API-CAT-02, API-HLTH-03.
- **Scope.** Group tracks into albums by MusicBrainz release ID, then by
  album artist and album title, erring on the side of not merging; keep
  same-titled albums apart by release ID, year or track list; group albums
  into release groups; mark compilations and Various Artists; order discs
  and keep disc titles; produce a reason for every decision and a review
  item when unsure. Curation-log merges, splits and aliases apply on top as
  overrides.
- **Interface sketch.** `pub fn group(tracks: &[GroupInput], overrides: &Curation) -> Grouping`
  where `Grouping { albums, release_groups, reasons, review: Vec<ReviewItem> }`.
- **Tests.** An album split across two folders; two albums in one folder;
  "Disc 1" and "Disc 2" folders of one album (the Jellyfin #5605 case);
  two "Greatest Hits" by one artist with different years; a compilation
  with a different artist per track and no album artist; one track missing
  the album artist that every sibling has; an override that merges two
  albums the rules kept apart. Properties: grouping is independent of input
  order; every track is in exactly one album; tracks with different release
  IDs are never in the same album.

### WP-077 Content identity and scan diff

- **Wave** 3 · **Size** L · **Depends on** WP-006, WP-031 (`MacProvider`),
  WP-040, WP-052.
- **Owns** `crates/gunmetal-core/src/music/identity.rs`,
  `crates/gunmetal-core/src/music/diff.rs`.
- **Serves** LIB-016, LIB-017, LIB-025, LIB-028, LIB-029, LIB-030, LIB-031,
  LIB-032, LIB-033, INT-008, DIS-038; API-HOME-03, API-LIB-05.
- **Scope.** LIB-028's identity order as versioned rules (MusicBrainz
  recording ID; FLAC MD5 when set; the hash of the audio window; the path
  stem within the root), the derivation of public IDs from identity (owner
  decision 11; a keyed derivation goes through WP-031's `MacProvider`, so
  the core never holds the server key), and the scan diff: given the previous index and fresh
  results for a set of paths, classify each file as unchanged, changed,
  added, removed, moved, or replaced by a better copy, keep "date added" at
  the first arrival, and send ambiguous matches (same tags and duration, new
  audio, new path) to review rather than guessing. Files under an offline
  root are never removed.
- **Interface sketch.** `pub fn identity(facts: &IdentityInputs, rules: IdentityRules) -> Identity`;
  `pub fn diff(prev: &IndexView, fresh: &[Observed], scope: &PathScope, roots: &RootStates) -> Diff`.
- **Tests.** Retagging an MP3 keeps its identity (the window excludes the
  tags); moving a file keeps identity and history; renaming a folder of
  200 files gives 200 moves, not 200 removals and additions; an MP3
  replaced by a FLAC of the same recording with MBIDs is an upgrade; the
  same without MBIDs goes to review; an offline root removes nothing.
  Properties: diffing an index against itself yields no changes (LIB-016);
  the diff of a scope never touches paths outside the scope (LIB-017);
  applying a diff and diffing again yields no changes.

### WP-078 Worker pool, deadlines and quarantine

- **Wave** 3 · **Size** M · **Depends on** WP-060, WP-061. Pausing a root
  after two lost workers appears in both WP-060 and this package; WP-060
  owns the root's paused state and this package reports lost workers to
  it, so the rule is implemented once.
- **Owns** `crates/gunmetal-worker/src/pool.rs`, `crates/gunmetal-worker/src/quarantine.rs`.
- **Serves** SEC-MED-018, SEC-MED-019, SEC-MED-021, SEC-MED-041, LIB-020.
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
  WP-119.
- **Owns** `crates/gunmetal-worker/src/jobs/` (`crates/gunmetal-worker/src/jobs/mod.rs`, `crates/gunmetal-worker/src/jobs/probe.rs`,
  `crates/gunmetal-worker/src/jobs/artwork.rs`, `crates/gunmetal-worker/src/jobs/hash.rs`),
  and from this wave `crates/gunmetal-worker/src/sandbox/syscalls.rs`,
  handed over by WP-045.
- **Serves** LIB-134, LIB-135, LIB-136, LIB-142, LIB-143, MUS-039, MUS-040,
  MUS-110, MUS-024, SEC-MED-044 to SEC-MED-046; API-SYNC-05, API-CAT-07.
- **Scope.** The jobs a worker runs: probe a file; hash an audio window for
  identity; decode artwork (embedded or a sidecar such as `cover.jpg`,
  `folder.jpg` or `artist.jpg`) after checking dimensions from the header,
  with `image::Limits`, then produce the fixed sizes as stripped JPEG, the
  placeholder and the palette from one decode.
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
  must be absent from the output; a PNG declaring
  20,000 × 20,000 pixels (refused before decoding); an animated GIF (first
  frame only); an SVG named `cover.jpg` (refused by detection); a truncated
  JPEG (problem recorded, file still probed). Output dimensions are exactly
  the fixed set.
- **Risks.** The seccomp allowlist from WP-045 is completed by running
  these jobs in audit mode; this package owns that file from this wave.

### WP-080 Owner claim and setup

- **Wave** 3 · **Size** M · **Depends on** WP-038, WP-062, WP-063, WP-064,
  WP-069, WP-118.
- **Owns** `crates/gunmetal-server/src/setup/` (except `setup/passkey.rs`,
  which WP-106 adds in wave 4).
- **Serves** ACC-001, ACC-002, ADM-018 to ADM-021, ADM-027, ADM-028,
  ADM-030 (the "Coming from another server?" step, which in R1 offers the
  listening-history and playlist imports of WP-108 and WP-112; the
  importer framework for other servers is R2, WP-229), ADM-140;
  API-AUTH-01 to API-AUTH-03.
- **Scope.** While no owner exists, refuse every route except the welcome
  and status pages; issue a single-use setup code valid for 24 hours to the
  console and to a file only the service user can read; bind an accepted
  code to the browser session that entered it (flows G1); walk the welcome
  steps (locale, owner, server name and sign-in message, privacy choices,
  import offer, first library); create the owner with a passkey where the
  page is a secure context, or a password and two-factor where it is not;
  issue recovery codes; close setup for good once an owner exists.
- **Tests (real SQLite).** Every non-setup route refuses while unclaimed; a
  wrong code goes through the limiter; a code used twice fails; a second
  browser cannot finish a claim the first started; after the owner exists,
  setup routes are gone; restarting with the code expired prints a fresh
  one. Owner creation takes a credential-enrolment interface; this package
  implements the password-and-two-factor branch. As first written, "the
  passkey branch is tested in WP-081" could not happen, because WP-081
  does not depend on this package and is in the same wave. WP-106 (wave 4)
  adds the passkey branch in `setup/passkey.rs` and tests it.
- **Risks.** The secure-context rule for the first admin (ACC-002, ADM-021)
  means many home installs will set a password first; the copy that
  explains it is the client's.

### WP-081 Passkeys

- **Wave** 3 · **Size** L · **Depends on** WP-041, WP-062, WP-118.
- **Owns** `crates/gunmetal-server/src/passkey/`.
- **Serves** ACC-050, ACC-055 (passkey part), SEC-IAM-018 to SEC-IAM-021;
  API-AUTH-03, API-AUTH-04.
- **Scope.** Registration and authentication ceremonies with a relying-party
  ID fixed at setup, challenges from the CSPRNG that are single use, bound
  to the pre-session and valid for five minutes, discoverable credentials
  with user verification required, verification of type, challenge,
  origin, RP ID hash, flags and signature, the counter rule and its alert.
- **Not in scope.** CBOR and COSE parsing (WP-041). Pairing devices (R2).
- **Tests (real SQLite).** Ceremonies constructed in the test with a
  software authenticator written for the test (a P-256 key pair signing the
  exact bytes the specification describes); a replayed challenge; a
  challenge from another pre-session; an expired challenge; the UV flag
  unset; a wrong origin; a counter that goes backwards (alert raised).
  Recorded ceremonies from real platform authenticators, which the
  security baseline asks for, are added when available and are not
  synthetic media, so they need the owner's agreement (owner decision 9).

### WP-082 Stream URLs and byte serving

- **Wave** 3 · **Size** L · **Depends on** WP-023, WP-031, WP-047, WP-060,
  WP-062, WP-065, WP-067 (identity check against the index), WP-070 (the
  rescan request), WP-118.
- **Owns** `crates/gunmetal-server/src/stream/`.
- **Serves** MUS-066, MUS-070, CLI-099 (byte ranges for fetching ahead),
  ACC-122, SEC-MED-012, SEC-MED-036, SEC-MED-059, SEC-MED-060;
  API-STR-01 to API-STR-03.
- **Scope.** Sign short-lived, session-bound URLs for an item and a
  representation; a refresh route for silent renewal; the capability route
  that verifies in order (parse, MAC, expiry, session alive, object still
  visible), opens the file beneath its root, checks identity against the
  index, and serves one byte range with the allowlisted content type and
  the required headers; caps each range response; tracks open responses per
  session and aborts them when the epoch changes.
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
  file); revocation cuts an in-flight response (tested over a real TCP
  socket with a slow reader); the "URL expired during playback is
  invisible" acceptance test from flows G10 at the server's level (refresh
  then continue from the same offset).

### WP-083 Client event channel

- **Wave** 3 · **Size** M · **Depends on** WP-062, WP-065, WP-118.
- **Owns** `crates/gunmetal-server/src/events/`.
- **Serves** api-needs.md Flags item 12; ADM-099, ADM-102, LIB-021, CLI-103,
  ACC-069; API-SYS-10.
- **Scope.** One authenticated server-sent events stream per signed-in
  session that carries sync nudges, scan progress, a stopped session, a
  revoked device and the active player changing; each event built per
  recipient through the authorisation layer; heartbeats; the stream closes
  when the session's epoch changes. Other modules publish to the
  in-process bus (WP-043) and this package forwards; so WP-085, WP-104 and
  the scan publish without depending on this package, and their tests
  assert the bus event.
- **Not in scope.** The R2 control channel (WebSocket, two-way).
- **Tests.** A recipient who cannot see a playlist does not receive its
  change event; a revoked session's stream ends with a final "signed out"
  event; a slow consumer is dropped rather than buffering without bound.
- **Risks.** Server-sent events instead of a WebSocket is owner decision 7.

### WP-084 Sync snapshot and delta

- **Wave** 3 · **Size** L · **Depends on** WP-039, WP-065, WP-066, WP-067,
  WP-068, WP-118, WP-119.
- **Owns** `crates/gunmetal-server/src/sync/`.
- **Serves** CLI-022, CLI-024, CLI-025, CLI-026, CLI-032, ACC-030, ACC-037,
  DIS-002, DIS-140, MUS-208, MUS-071 (seek index fetch), LIB-146, DIS-102,
  MUS-056 (the technical and sort fields in the feed); API-SYNC-01 to
  API-SYNC-04, API-DEV-02, API-CAT-05 (seek index).
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
  track in an ungranted library answers as unknown. Bytes and time at
  100,000 synthetic tracks are measured by the benchmark runner (WP-115),
  not by a test.

### WP-085 Queue service

- **Wave** 3 · **Size** M · **Depends on** WP-025, WP-026, WP-065, WP-068,
  WP-118.
- **Owns** `crates/gunmetal-server/src/queue/`.
- **Serves** MUS-116 to MUS-128, CLI-103; API-QUE-01 to API-QUE-04,
  API-SES-03.
- **Scope.** Store each profile's queue as operations and snapshots in the
  user log; accept operations against a version and return the new
  version or the stale-version rejection; take coalesced position updates;
  set the active device when a device presses Play and publish that on the
  in-process bus, which the event channel (WP-083, same wave) forwards so
  the other device pauses (owner decision 14). "Save
  queue as playlist" is not here: the client already holds the queue and
  sends its items to WP-093's create route.
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
- **Serves** MUS-180 to MUS-184, MUS-185, MUS-109 (love events), ACC-117, ACC-118, DIS-022,
  DIS-023, DIS-045 to DIS-052, CLI-093; API-LOG-01 to API-LOG-05,
  API-SES-04, API-USR-07.
- **Scope.** One write path for plays, skips, loves, ratings, dismissals and
  their reversals, and play removals; idempotent by event ID; plays queued
  offline accepted with their real timestamps and merged; the private
  session flag that drops play and recommendation events (with the
  proposed expiry after a period without playback, owner decision 27);
  derived counts projected into the cache.
- **Tests (real SQLite and log files).** The same offline batch uploaded
  twice is counted once; a removal hides the play from counts and history
  whatever the order; a private session records nothing; loves from two
  devices resolve to the later clock; replaying the log into an empty
  cache through this package's projection rebuilder gives the same counts
  and loves, compared with literal expected values.

### WP-087 Accounts, profiles, settings and devices

- **Wave** 3 · **Size** M · **Depends on** WP-062, WP-065, WP-068, WP-118.
- **Owns** `crates/gunmetal-server/src/account/`.
- **Serves** ACC-011, ACC-012, ACC-017, ACC-068 to ACC-070, ACC-114 (the
  per-profile activity-visibility setting), CLI-030, DIS-003, DIS-007,
  DIS-013, DIS-103 (remembered view state), MUS-158 (the lyrics-stay-open
  setting), INT-026; API-SYS-05, API-USR-01 (names),
  API-USR-02, API-DEV-01, API-HOME-01, API-HOME-02.
- **Scope.** Who am I; profile names; settings records with person and
  device scope (latest wins per key); Home layouts and pins as versioned
  documents; the device list with last-seen times, revoke one and sign out
  of all (calling WP-062's epoch bump).
- **Not in scope.** Profile pictures, which need the artwork job's
  re-encode (WP-103 adds the upload route).
- **Tests (real SQLite).** A device-scope setting does not follow the
  person to another device; a person-scope one does; revoking a device
  ends its session immediately; another person's device cannot be revoked
  (404, not 403).

### WP-088 WASM facade

- **Wave** 4 (moved from 3 in review, because WP-059 moved to wave 3) ·
  **Size** M · **Depends on** WP-021, WP-025 to WP-028,
  WP-030, WP-039, WP-040, WP-054, WP-055, WP-058, WP-059.
- **Owns** `crates/gunmetal-wasm/` (creates the crate),
  `.github/workflows/wasm.yml` (the `wasm32` build job; `ci.yml` belongs
  to the integrator).
- **Serves** ADR 1 decision 2; CLI-022, DIS-084, DIS-002, MUS-208.
- **Scope.** A thin `wasm-bindgen` layer so the web client can apply sync
  frames to an in-memory library, query search, evaluate rules and Home
  rows, compute radio picks, apply queue operations optimistically, read
  the decision engine and gain decision, follow the player state and look
  up lyrics positions. Every exported function is a direct call into the
  core with conversion only.
- **Not in scope.** Persistent storage in the browser (client).
- **Tests.** Native unit tests of each conversion (the facade must compile
  and be covered on the host target); a `wasm32` build in CI; a size check
  of the `.wasm` file reported.
- **Risks.** Coverage tooling on the facade's generated glue may count code
  the tests cannot reach (unverified); if so, the glue is kept in a module
  the owner explicitly approves, because the gate allows no exclusions.
  The workspace sets `unsafe_code = "forbid"`, and `wasm-bindgen`'s
  generated code contains `unsafe` blocks; whether the forbid lint rejects
  macro-generated code in this crate is unverified. If it does, this
  crate needs a crate-level exception, which is an owner decision because
  the README's rule is "no `unsafe` in the core" while the workspace
  forbids it everywhere (owner decision 34).

### WP-089 Server facts: health, discovery, negotiation and API reference

- **Wave** 3 · **Size** M · **Depends on** WP-039 (protocol versions),
  WP-044, WP-062, WP-065, WP-118.
- **Owns** `crates/gunmetal-server/src/meta/`.
- **Serves** ADM-128, INT-001, INT-005, INT-013, CLI-032, ADM-140, CLI-034,
  INT-147, ADM-090, ADM-001, ADM-010; API-SYS-01, API-SYS-03, API-SYS-04,
  API-SYS-06, API-SYS-09, API-SET-09, API-TOK-03.
- **Scope.** Liveness and readiness routes that reveal nothing about the
  library; protocol negotiation; capability discovery scoped to the
  caller; the public sign-in facts; the API reference generated from the
  route table; server identity and about (name, version, build, storage
  locations, live footprint); deep-link resolution that never grants
  access by itself.
- **Tests.** Unauthenticated discovery reveals only the version and sign-in
  methods; the API reference lists exactly the routes in the table; an old
  client gets the "update needed" response with the version it needs.

### WP-090 Backups and verification

- **Wave** 3 · **Size** L · **Depends on** WP-046, WP-047, WP-068, WP-069,
  WP-070, WP-071, WP-118.
- **Owns** `crates/gunmetal-server/src/backup/`,
  `crates/gunmetal-secrets/src/export.rs` (a sealed export of the server
  keys for the backup, so the root secret is never exposed outside the
  secrets crate; one `pub mod` line in that crate's `lib.rs`).
- **Serves** ADM-065, ADM-066, ADM-069, ADM-072, ACC-013,
  SEC-OPS-041, SEC-OPS-043; API-SET-04.
- **Scope.** A daily backup by default of the identity store (through the
  online backup API), the user log, configuration and server keys, with a
  manifest, signed and encrypted; verified after writing by reading it
  back; retention; contents described plainly; download and upload.
  Derived data excluded unless chosen.
- **Not in scope.** Restore (WP-109). The full export in documented
  formats (ADM-074), moved to WP-108 in review: it shares its format work
  with the per-person export there, and this package was already the
  largest in its wave.
- **Tests (real SQLite and files).** A backup taken while writes continue is
  consistent; a backup with one flipped byte fails verification; the
  manifest lists every file with its hash; retention keeps the configured
  count; the derived-data store is left out unless chosen; the backup
  never contains the raw root secret unencrypted (canary).
- **Risks.** The encryption format and the archive container inside it
  are owner decision 12.

### WP-091 Scoped tokens and the tool change feed

- **Wave** 3 · **Size** M · **Depends on** WP-031, WP-033, WP-047, WP-062,
  WP-065, WP-066, WP-118.
- **Owns** `crates/gunmetal-server/src/tokens/`.
- **Serves** ACC-049, INT-006, INT-012, INT-017 to INT-023; API-TOK-01,
  API-TOK-02.
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

### WP-092 Rule store and server-side evaluation

- **Wave** 3 · **Size** M · **Depends on** WP-027, WP-065, WP-066, WP-067,
  WP-068, WP-118, WP-119.
- **Owns** `crates/gunmetal-server/src/rules/`.
- **Serves** DIS-105, DIS-119 to DIS-122, MUS-143 to MUS-146, MUS-149,
  INT-138; API-PL-05, API-PL-06.
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

### WP-093 Manual playlists and pins

- **Wave** 3 · **Size** M · **Depends on** WP-065, WP-067, WP-068, WP-118.
- **Owns** `crates/gunmetal-server/src/playlists/`.
- **Serves** MUS-132 to MUS-135, MUS-137, MUS-139, MUS-125, INT-138;
  API-PL-01, API-PL-02, API-PL-07, API-PL-08, API-QUE-05.
- **Scope.** Create, rename, add, reorder and remove, as log operations on
  entries with their own IDs that refer to tracks by content identity;
  concurrent adds both kept, a remove beats a concurrent move, reorders
  relative to neighbours; the duplicate warning; pin and love; missing
  entries kept with their last known title (owner decision 29); creating a
  playlist from a list of items, which is how the client saves its queue;
  the write API for tools under a scoped token.
- **Tests (real SQLite and log files).** Two adds at once keep both; a
  remove and a move at once keep the remove; a track removed from the
  catalogue (the test removes the row; the trash and purge are WP-111, two
  waves later, which repeats the case end to end) becomes a "missing"
  entry and rematches when a row with the same identity returns; a
  playlist with the read-only flag rejects edits (this package owns the
  flag; WP-112 sets it on folder playlists).

### WP-094 Users and invitations

- **Wave** 3 · **Size** M · **Depends on** WP-062, WP-063, WP-064 (invite
  codes go through the guessing limiter), WP-065, WP-069, WP-118.
- **Owns** `crates/gunmetal-server/src/users/`.
- **Serves** ACC-006, ACC-008, ACC-040, ACC-080, ADM-052; API-ADM-01 to
  API-ADM-03.
- **Scope.** List users with their libraries; enable and disable without
  deleting; make an administrator; hand over ownership by adding an admin
  and removing yourself; invitations as capabilities with libraries, a use
  count and an expiry; the link and QR payload with the address it will
  carry and a warning when it is private (flows G7); the invite landing
  that reveals nothing else; redemption logged.
- **Tests (real SQLite).** An invite used past its count or expiry fails
  identically to a wrong code; a disabled user's sessions end; the last
  owner cannot remove themselves; the invite landing reveals no user names.

### WP-095 Startup page, snapshots and cache rebuild

- **Wave** 3 · **Size** M · **Depends on** WP-042, WP-046, WP-068, WP-069,
  WP-070, WP-071, WP-118.
- **Owns** `crates/gunmetal-server/src/startup/`.
- **Serves** ADM-032, ADM-056 to ADM-059, ADM-077, ADM-112, ADM-141;
  API-SYS-07, API-LIB-06, API-SET-10.
- **Scope.** The page the server renders itself while it starts, migrates,
  rebuilds or restores, before the database opens; the pre-upgrade
  snapshot and the migration check; rebuilding the cache from files, the
  log and the derived-data store on request or when an older binary meets
  newer data, by calling the projection rebuilders that modules register
  (queue, listening, playlists, rules, curation) and requesting a full
  library scan from the task runner; restart and shut down.
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

### WP-096 Single sign-on

- **Wave** 3 · **Size** L · **Depends on** WP-046, WP-047, WP-048, WP-062,
  WP-118.
- **Owns** `crates/gunmetal-server/src/oidc/`.
- **Serves** ACC-057, ACC-003; API-AUTH-06, API-SET-03.
- **Scope.** Authorization-code flow with PKCE, state and nonce against the
  household's own provider; discovery and keys fetched through the egress
  gate; ID tokens verified with keys from the provider's key set using
  algorithms pinned per provider, with `iss`, `aud`, `azp`, `exp`, `iat`
  and `nonce` checked; the redirect allow-list; linking to existing
  accounts; provider configuration with a test button; the client secret
  in the vault.
- **Tests.** Against a provider simulated in the test (its own key pair and
  discovery document served locally): a valid login; `alg: none`; an
  HMAC-signed token using the public key as the secret; a wrong audience;
  an expired token; a replayed nonce; a redirect to an unlisted URL. The
  simulated provider signs with each algorithm R1 accepts, which means
  RS256 as well unless the owner limits R1 to ES256 and EdDSA providers.
- **Risks.** Hand-written versus the `openidconnect` crate is owner
  decision 10, and so is RSA verification, which the proposed crate list
  lacks (see "Missing from this table"). Sending the client secret to the
  provider's token endpoint needs its bytes outside `gunmetal-secrets`;
  the plan has no sanctioned path for that yet (see Review notes).

### WP-097 Alerts, log rotation, free space and crash records

- **Wave** 3 · **Size** M · **Depends on** WP-048, WP-069, WP-070, WP-118.
- **Owns** `crates/gunmetal-server/src/ops/`.
- **Serves** ADM-083, ADM-116, ADM-119, ADM-129, ADM-130; API-SET-02
  (network activity page), API-SET-07.
- **Scope.** Alert rules and the free destinations the owner chooses
  (in-app always; others through the egress gate); log rotation; the
  free-space guard that alerts and refuses writes that would fill the disk;
  local crash records; the network activity page reading the egress
  record.
- **Tests.** Free space simulated through an injected probe: an alert at the
  threshold, and the guard's decision refusing a write below the floor;
  rotation at size; an alert to a destination not granted is not sent and
  is shown as blocked.
- **Gap.** The guard decides, but the writers that fill a disk (the cache
  writer, the user log, backups, the artwork cache, uploads) belong to
  other packages, several already merged by this wave. Which of them must
  consult the guard, and who adds that call, is not assigned (see Review
  notes).

### WP-098 Change-detection triggers

- **Wave** 3 · **Size** M · **Depends on** WP-060, WP-065, WP-070, WP-118.
- **Owns** `crates/gunmetal-fs/src/watch.rs`,
  `crates/gunmetal-server/src/triggers/`.
- **Serves** LIB-013, LIB-014, LIB-015, LIB-017, INT-011; API-SCAN-02.
- **Scope.** Watch local disks with debounce, poll shares and cloud mounts
  at the configured interval and parallelism, run the scheduled safety-net
  scan, accept path-scoped refreshes from admins and scoped tokens, and
  turn every trigger into a de-duplicated path set for the task runner;
  warn when the OS watch limit is too low.
- **Tests (real filesystem).** Ten files written in a burst give one
  request; a rename gives a request for both directories; a watch limit
  reported low by an injected probe produces the warning; a path outside
  every root is refused; a member's refresh request answers 404. Refresh
  under a scoped token needs token authentication from WP-091, in the same
  wave, so that case is an acceptance test in WP-117.

### WP-099 Library administration and grants

- **Wave** 3 · **Size** M · **Depends on** WP-024, WP-060, WP-065, WP-067,
  WP-070, WP-118.
- **Owns** `crates/gunmetal-server/src/libraries/`.
- **Serves** LIB-001, LIB-003 to LIB-007, LIB-011, LIB-031, LAT-010, DIS-012,
  ADM-025, ADM-089, ACC-037, MUS-027, MUS-035 (rules storage); API-LIB-01
  to API-LIB-05, API-LIB-07, API-SCAN-01.
- **Scope.** Create and configure libraries (kind, roots, spoken-word flag,
  keep off Home, splitting rules, exclusions, watch and poll settings);
  the folder browser that lists only allowed base paths to admins and runs
  live checks (readable, empty, storage type, read-only); refusing unsafe
  roots; grant and revoke routes, written through WP-065's grants API
  (WP-065 owns the table), with new libraries visible only to the owner and admins
  until granted (owner decision 19); changing a root's location with a
  preview; starting a scan by requesting the scan task kind from the task
  runner, which WP-102 registers.
- **Tests (real SQLite and filesystem).** The data directory as a root is
  refused; a non-admin calling the folder browser gets 404; a new library
  is invisible to members; moving a root to a copy matches every file in
  the preview by its fingerprint (size, modification time and the head
  and tail hash from WP-060), so identities are kept. Content identity
  itself (WP-077) and the worker pool (WP-078) are in this same wave, so
  the preview does not use them.

### WP-100 Task list and activity log

- **Wave** 3 · **Size** M · **Depends on** WP-065, WP-069, WP-070, WP-118.
- **Owns** `crates/gunmetal-server/src/tasklog/`.
- **Serves** ADM-093, ADM-110, LIB-022, ADM-088; API-SCAN-04, API-SCAN-05.
- **Scope.** Routes for the task list (run, cancel, progress, last run,
  duration, errors) and the activity log (scans, "0 changed" rescans,
  moves, re-reads after a parser update, imports) alongside the security
  events from the audit log; an activity-entry interface the scan pipeline
  writes to.
- **Tests (real SQLite).** A member cannot read the task list; activity
  entries page in order; an entry written by a fake producer appears with
  its counts.

### WP-101 Built-in HTTPS by ACME (owner decision)

- **Wave** 4 (moved from 3 in review, because WP-073 moved to wave 3) ·
  **Size** M · **Depends on** WP-047, WP-048, WP-073.
- **Owns** `crates/gunmetal-server/src/acme/`.
- **Serves** ADM-022, ACC-098, ADM-021.
- **Scope.** Obtain and renew a certificate for a domain the owner controls
  through the egress gate, store the key under `secrets/tls/`, and hand it
  to WP-073's listener.
- **Tests.** Against a local ACME test server if one can run in CI
  (unverified which); renewal before expiry with a manual clock. If no
  ACME test server can run inside the gate, the protocol steps are
  tested against a fake written in the test from RFC 8555, and talking to
  a real ACME server becomes a manual check outside the gate, which is a
  weaker guarantee the owner should accept knowingly.
- **Risks.** Whether this is R1 or a point release is owner decision 21,
  and the crate choice for ACME messages is unsized (see "Missing from this
  table").

### WP-120 Sign-in and sign-out routes (added in review)

- **Wave** 3 · **Size** M · **Depends on** WP-062, WP-063, WP-064, WP-069,
  WP-118.
- **Owns** `crates/gunmetal-server/src/signin/`.
- **Serves** ACC-052, ACC-053, ACC-063, ACC-007, ACC-079, ADM-122;
  API-AUTH-05, API-AUTH-07, API-AUTH-08 (the route side).
- **Why it exists.** WP-063 owned password and two-factor verification
  "storage side", WP-062 owned sessions and WP-064 the limiter, but no
  package owned the routes a person actually signs in and out through.
- **Scope.** The password sign-in route, followed by the two-factor or
  recovery-code step, issuing a session through WP-062 with the
  remembered or shared-computer lifetime; sign out of this session; every
  attempt through the limiter; audit entries for success and failure; the
  same response for an unknown account, a wrong password and a disabled
  account; no user list before sign-in.
- **Tests (real SQLite).** A full sign-in over WP-044's test client sets
  the `__Host-` cookie and a second request is authenticated; a wrong
  second factor issues no session; the sixth wrong password within the
  window is limited with the exact back-off; unknown, wrong and disabled
  give byte-identical responses; sign out makes the old cookie fail on
  the next request; each attempt leaves exactly one audit entry.

## Wave 4: scanning and the features that need it

WP-088 and WP-101 (numbered in the wave 3 section) also run in this wave,
and WP-121 is at the end of this section.

### WP-102 Scan pipeline

- **Wave** 4 · **Size** L · **Depends on** WP-060, WP-066, WP-067, WP-070,
  WP-071, WP-075, WP-076, WP-077, WP-078, WP-079, WP-083, WP-100, WP-119.
- **Owns** `crates/gunmetal-server/src/scan/`.
- **Serves** LIB-012, LIB-016, LIB-017, LIB-019 to LIB-022, LIB-029,
  LIB-030, LIB-032, LIB-136, LIB-192 (hand-off), ADM-088, MUS-043,
  MUS-044 (inputs); API-SCAN-01, API-SCAN-03, API-CAT-09, API-CAT-10; the
  "Library scan" job. The parser-upgrade re-read (LIB-025) was split out
  to WP-123 in review to keep this package within one session.
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
  missing); store each file's parser versions so WP-123 can find stale
  ones; never launch a helper per file.
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
  read per file stay under the header-only bound for each format.
  Readers keep answering during a long scan (a timed read in parallel).

### WP-103 Artwork cache and image serving

- **Wave** 4 · **Size** M · **Depends on** WP-065, WP-067, WP-071, WP-078,
  WP-079, WP-082. (WP-078 runs the re-encode of uploads; WP-065 and WP-067
  answer which library an image belongs to.)
- **Owns** `crates/gunmetal-server/src/artwork/`.
- **Serves** LIB-142, LIB-143, MUS-040, SEC-MED-046 to SEC-MED-048,
  SEC-MED-061, ACC-125, ACC-011 (pictures); API-STR-05, API-SYNC-05,
  API-USR-01 (pictures).
- **Scope.** The derivative cache keyed by content hash and size, bounded in
  bytes with least-recently-used eviction; image routes that accept a size
  only from the fixed list; for the first-party web client, content-addressed
  paths authorised by the session cookie so images cache (the api-needs
  proposal in Flags item 7, which departs from ADR 1 decision 6's signed
  URLs for images and so waits for owner decision 31; until then images
  use signed URLs like media); profile picture uploads streamed with a size
  cap, detected by content, re-encoded through the worker, stored by hash.
- **Tests (real SQLite and files).** Sizes −100000, 0, 2^31, `abc` and a
  valid size with an invalid shape give 400; the cache never exceeds its
  bound; an SVG upload is refused; an upload with EXIF comes back without
  it; a member cannot fetch artwork for a library they cannot see.

### WP-104 Playback session registry and stop

- **Wave** 4 · **Size** M · **Depends on** WP-082, WP-083.
- **Owns** `crates/gunmetal-server/src/playback/`.
- **Serves** ADM-099, ADM-100, ADM-102, ACC-072, ACC-073, INT-134,
  MUS-190; API-SES-01, API-SES-02.
- **Scope.** Record who plays what on which device, the delivery path and
  the decision reason the device reported, alongside what the server
  actually served; the admin now-playing view; stopping a session with a
  plain-text message, which cuts in-flight responses and pushes "stopped
  by the owner" to the device.
- **Tests (real SQLite).** Stopping a session ends its byte stream over a
  real socket and the device's event stream receives the message as text
  (markup in the message is not interpreted); a member cannot see others'
  sessions beyond what the feature map's open decision 13 allows. That
  decision was not in this plan's list, though this test cannot be
  written without it; it is now owner decision 30.
- **Note.** "Pushes to the device" means publishing on the bus (WP-043),
  which WP-083 forwards.

### WP-105 Audio packaging route (conditional on ADR 4)

- **Wave** 4 · **Size** S · **Depends on** WP-056, WP-082.
- **Owns** `crates/gunmetal-server/src/packaging/`.
- **Serves** MUS-230, MUS-067; API-STR-04.
- **Scope.** Serve the initialisation segment and numbered media segments
  under one capability token per playback, with the segment number checked
  against the stored frame index.
- **Tests (real files).** The segments served for a synthetic FLAC parse
  back to the original frames; a segment number past the end gives 404; an
  expired token refreshes like a byte stream.

### WP-106 Account recovery and step-up

- **Wave** 4 · **Size** M · **Depends on** WP-062, WP-063, WP-069, WP-080,
  WP-081.
- **Owns** `crates/gunmetal-server/src/recovery/`,
  `crates/gunmetal-server/src/setup/passkey.rs` (the passkey branch of
  owner creation at setup, added here because WP-080 and WP-081 were in
  the same wave).
- **Serves** ACC-002 (passkey part), ACC-004, ACC-055, ACC-056, ACC-064,
  ADM-034, ACC-078; API-AUTH-03 (passkey part), API-AUTH-10 to
  API-AUTH-12, API-USR-03, API-USR-04.
- **Scope.** Step-up confirmation before sensitive changes; the person's
  sign-in methods page (list, add and remove passkeys, password and
  two-factor) behind step-up; an admin's single-use, short-lived sign-in
  link with a QR payload for a locked-out person; `gunmetal admin recover`
  on the host printing a single-use link whose use raises a banner and an
  audit entry; the person's own sign-in history.
- **Tests (real SQLite).** Removing the last sign-in method is refused; a
  sensitive change without a recent verification asks for step-up; a
  recovery link works once and expires; its use appears in the audit log
  and raises the banner flag; at setup on a secure context, the owner is
  created with a passkey from WP-081's software authenticator, and on an
  insecure context the passkey branch is not offered.

### WP-107 Curation and the file inspector

- **Wave** 4 · **Size** M · **Depends on** WP-065, WP-067, WP-068, WP-070,
  WP-076, WP-078 (probing on demand runs through the pool), WP-079,
  WP-095.
- **Owns** `crates/gunmetal-server/src/curation/`.
- **Serves** MUS-007, LIB-041, LIB-058, LIB-179, LIB-012, ADM-125, LIB-195,
  LIB-059, LIB-098; API-CAT-11 to API-CAT-13.
- **Scope.** Merge, split and alias artists and albums as curation-log
  events that survive rescans and rebuilds; rescan one item by requesting a
  path-scoped scan; the file inspector for admins (every raw tag, the
  structure the parsers read, the identification decision and "why is this
  here", errors with offsets), probing the file through the worker on
  demand.
- **Tests (real SQLite and log files).** A merge is written as one
  curation event and survives a cache rebuild through this package's
  projection rebuilder; that it also survives a rescan is proven in the
  core (WP-076 applies overrides on top of grouping) and end to end in
  WP-117, because the scan (WP-102) is in this same wave; an alias makes
  both names find the artist; the inspector
  shows the exact problem offsets the probe reported; a member gets 404
  from the inspector.
- **Not in scope.** A parse summary for members. MUS-114's track info
  sheet names the inspect API as its source, but the inspector is
  admin-only; this plan assumes members see the technical fields already
  in the synced copy, and admins get a link to the inspector (see Review
  notes).

### WP-108 History import and data export

- **Wave** 4 · **Size** L (was M; it gained the full export in review) ·
  **Depends on** WP-057, WP-068, WP-070, WP-086, WP-087, WP-090, WP-092,
  WP-093. (The export covers layouts and settings from WP-087 and saved
  rules from WP-092 as well as plays and playlists.)
- **Owns** `crates/gunmetal-server/src/history_io/`.
- **Serves** ADM-042, ADM-074, MUS-188, MUS-189, ACC-010, DIS-058, INT-151,
  INT-107; API-USR-05, API-USR-06, API-SET-04 (full export),
  API-SET-11 (listening-service files).
- **Scope.** Upload Last.fm or ListenBrainz export files through the upload
  path, match them in a job with progress, write imported plays marked as
  imported, remove an import as a batch; build the person's documented
  export of everything they told the server; build the owner's full
  export of every person and the server's settings in the same documented
  formats. If this proves too large for one session, the full export
  splits off as its own package in wave 5.
- **Tests (real SQLite and log files).** Importing the same file twice adds
  nothing the second time; removing an import removes exactly its plays;
  unmatched rows land in the unmatched list with reasons; the export
  contains a removed play only as its removal, and another person's data
  never; the full export of a two-person server, compared with a literal
  expected document, contains both people and no secret (canary).

### WP-109 Restore

- **Wave** 4 · **Size** L · **Depends on** WP-080, WP-085, WP-086, WP-087,
  WP-090, WP-093, WP-095. (The round-trip test compares queues, loves,
  playlists and accounts, which those packages project.)
- **Owns** `crates/gunmetal-server/src/restore/`.
- **Serves** ADM-029, ADM-051, ADM-070, ADM-071, ACC-013; API-SET-04
  (restore), API-SET-05.
- **Scope.** Restore from the UI with a restore point and a preview, from
  the command line, and from the welcome screen behind the setup code
  (flows G15); verify signature and encryption before anything is
  written; parse the archive with limits; remap library roots in a dry run;
  warn when the domain changed and offer one-time sign-in links (flows
  G3); rebuild the cache after.
- **Tests (real SQLite and files).** A backup made by WP-090 restores to a
  fresh directory and the queue, playlists, loves and accounts match; a
  backup with a bad signature is refused before any write; a hostile
  archive (too many entries, a path with `..`, an entry larger than
  declared) is refused; a restore attempted on an unclaimed server without
  the setup code is refused.

### WP-121 Release builds, container image and service install (added in review)

- **Wave** 4 · **Size** M · **Depends on** WP-043, WP-045, WP-089, WP-118.
- **Owns** `.github/workflows/release.yml`, `packaging/` (the container
  build file, a compose example, the systemd unit template and an
  install script), `crates/gunmetal-server/src/service/`.
- **Serves** ADM-002, ADM-003, ADM-004, ADM-005; ADM-128 (the container
  health check uses WP-089's route). No package served these R1 rows.
- **Scope.** Multi-architecture release builds (x86-64, 64-bit ARM, 32-bit
  ARM with the reduced isolation tier from WP-045 labelled) with the
  release profile's `overflow-checks`; a container image with documented
  volumes for data, cache and media and a health check; version-pinned
  tags; a `gunmetal service print-unit` subcommand that renders the
  systemd unit for the configured paths and user. The privileged steps
  (creating the service user, installing and enabling the unit) are in
  `packaging/install.sh`, not in the binary, because SEC-MED-063 forbids
  the server from starting any program outside the sandbox launcher, so
  the binary cannot run `useradd` or `systemctl` itself. Whether that
  still meets ADM-005's "one command" is owner decision 35.
- **Not in scope.** Signing releases (owner decision 32 decides the keys
  first). Windows services and launchd (R2, ADM-013). The hardware guide
  and install docs (ADM-011, ADM-003's release notes), which are
  documentation, not backend code.
- **Tests.** The rendered unit for a given configuration equals a literal
  expected unit file and sets the hardening options the security baseline
  names; paths with spaces and non-ASCII characters are escaped. The
  install script and the image are outside the Rust gate's coverage and
  mutation runs: they get `shellcheck`, a container smoke test in
  `release.yml` (start the image, wait for the health check, stop it), and
  a smoke run of the 32-bit ARM build under emulation if CI can provide
  one (unverified). That is weaker than the gate, and this package says so
  in its pull request rather than hiding it.

## Waves 5 and 6: health, jobs, benchmark and acceptance

### WP-110 Library health and root health

- **Wave** 5 · **Size** M · **Depends on** WP-060, WP-074, WP-090, WP-091,
  WP-097, WP-102, WP-106, WP-119. The roll-up in API-HLTH-05 reports
  backups and verification (WP-090), advisories (WP-074), token expiry
  (WP-091) and recovery use (WP-106), none of which was a dependency as
  first written.
- **Owns** `crates/gunmetal-server/src/health/`.
- **Serves** MUS-044, LIB-014 (warning), LIB-032, LIB-034, LIB-193,
  LIB-194, ADM-108, ADM-109, MUS-229; API-HLTH-01, API-HLTH-02,
  API-HLTH-05.
- **Scope.** The health report (damaged and unreadable files, tag problems
  with suggested fixes, same-name collisions, sidecar problems, formats a
  supported browser cannot decode, missing and moved files, offline roots,
  watch warnings), each root's reachability, and the admin home's roll-up.
- **Tests (real SQLite).** A synthetic library with one of each problem
  yields exactly one entry of each kind; an offline root shows as offline
  and its items as greyed, not missing.

### WP-111 Review queue, trash and purge

- **Wave** 5 · **Size** M · **Depends on** WP-068, WP-076, WP-093 (the
  purged-track test reads a playlist), WP-102.
- **Owns** `crates/gunmetal-server/src/review/`.
- **Serves** LIB-033, LIB-099, LIB-051, ADM-086; API-HLTH-03, API-HLTH-04.
- **Scope.** Doubtful decisions with their evidence and a proposal; accept,
  reject or choose another, stored in the curation log; the trash of items
  whose files went missing with their purge date; restore and purge now;
  the purge job, which never runs while a root is offline.
- **Tests (real SQLite and log files).** An accepted review survives a
  rebuild; a purge is skipped while a root is offline; a purged track
  becomes a "missing" entry in a playlist.

### WP-112 Playlist files: import, export and folder playlists

- **Wave** 5 · **Size** M · **Depends on** WP-022, WP-057, WP-093, WP-102.
- **Owns** `crates/gunmetal-server/src/playlist_files/`.
- **Serves** MUS-140, LIB-192, ADM-043, ADM-044, SEC-MED-050; API-PL-03,
  API-PL-04.
- **Scope.** Upload M3U and M3U8 files, resolve entries with the matcher,
  report matches and misses; export as M3U8 with paths relative to a
  library root; turn `.m3u` files found during a scan into read-only
  playlists with "Duplicate to edit" (owner decision 29).
- **Tests (real SQLite and files).** Entries pointing outside the library,
  at URLs or with `..` are dropped and reported; a folder playlist is
  read-only; export then import of a playlist gives the same tracks.

### WP-113 Neighbour table and rule re-evaluation jobs

- **Wave** 5 · **Size** S · **Depends on** WP-058, WP-092, WP-102.
- **Owns** `crates/gunmetal-server/src/derived_jobs/`.
- **Serves** DIS-060, DIS-121; API-SYNC-06, API-PL-06.
- **Scope.** The nightly and after-scan neighbour table rebuild within its
  size budget, synced as a change; re-evaluation of server-side smart
  playlists when the library changes.
- **Tests (real SQLite).** A scan triggers one rebuild, not one per batch;
  the table's size stays inside the budget.

### WP-114 Loudness analysis job (conditional on ADR 5)

- **Wave** 5 · **Size** M · **Depends on** WP-003 (ADR 5 accepted), WP-029,
  WP-071, WP-078, WP-079, WP-102.
- **Owns** `crates/gunmetal-worker/src/jobs/loudness.rs`,
  `crates/gunmetal-server/src/loudness/`.
- **Serves** MUS-086, MUS-089, LIB-024, ADM-095.
- **Scope.** At low priority after a scan, throttled and checkpointed,
  decode untagged tracks in the worker and measure them; store results in
  the derived-data store; mark the gain source as measured.
- **Tests.** Synthetic tones encoded in the test (FLAC with verbatim
  subframes is simple to write; other formats only if the testkit can
  produce them without an external encoder); a file over 12 hours keeps its
  tags only; a restart resumes from the checkpoint.

### WP-115 Scan benchmark

- **Wave** 5 · **Size** M · **Depends on** WP-054, WP-067, WP-084, WP-102,
  WP-119.
- **Owns** `crates/xtask/src/bench.rs`. (The generator moved to WP-119 in
  wave 2, because wave 3 and 4 tests needed it.)
- **Serves** README roadmap item "Benchmark: scan time against Jellyfin";
  ADR 1 consequences; ADM-010, LIB-019, DIS-019.
- **Scope.** Generate a library of a chosen size with WP-119; run
  Gunmetal's scan on it and record time, bytes read, peak memory and
  database size; record the measurements other packages deliberately left
  out of their tests (search index build time and memory at 100,000
  tracks for WP-054, sync snapshot bytes and time for WP-084, batch commit
  time for WP-067); document how to run the same library through Jellyfin
  for the comparison, which is a manual run outside CI.
- **Tests.** The benchmark's report format, written literally; the runner
  on a ten-file library produces a report with every field present.
- **Risks.** A synthetic library made of tiny files is not a real library:
  real files are larger and real tags messier, so the published comparison
  needs the owner's choice of library (owner decision 22). Jellyfin may be
  faster on some steps; the README promises to publish the numbers either
  way.

### WP-123 Parser-upgrade re-read (split from WP-102 in review)

- **Wave** 5 · **Size** S · **Depends on** WP-052, WP-070, WP-100, WP-102.
- **Owns** `crates/gunmetal-server/src/reread/`.
- **Serves** LIB-025, ADM-095; the "Parser-upgrade re-read" job.
- **Scope.** At startup after an upgrade, compare each file's stored
  parser versions (WP-102 stores them) with `PARSER_VERSIONS` (WP-052),
  and request a throttled, resumable re-read of only the stale files
  through the scan's path-set entry point; record an activity entry with
  the count.
- **Tests (real SQLite).** With the parser-version table overridden in the
  test: bumping the FLAC version re-reads every FLAC and no other file;
  an unchanged table re-reads nothing; a restart in the middle resumes
  from the checkpoint and re-reads no file twice.

### WP-116 Doctor, diagnostics bundle and emergency page

- **Wave** 6 · **Size** M · **Depends on** WP-095, WP-097, WP-110.
- **Owns** `crates/gunmetal-server/src/diagnostics/`.
- **Serves** ADM-113, ADM-123, ADM-124, ADM-130, ADM-080 (write queue view),
  ADM-141, CLI-033; API-SYS-08, API-SET-08, API-DEV-03.
- **Scope.** `gunmetal doctor` (permissions, isolation tier, roots, free
  space, versions, `--fix-perms`); a masked diagnostic bundle with a
  preview; a minimal server-rendered emergency page with status, recent log
  lines, a backup and a restart; accepting a diagnostics report a person
  built and reviewed on their device.
- **Tests.** The bundle contains no secret (canary) and no absolute media
  paths when masking is on; the emergency page works with the client
  bundle missing; doctor's checks each have a passing and a failing case.

### WP-117 R1 flow acceptance tests

- **Wave** 6 · **Size** M · **Depends on** every R1 package.
- **Owns** `crates/gunmetal-server/tests/flows/`.
- **Serves** flows.md F01 to F03, F05, F06, F10, F12 to F16 (server side).
- **Scope.** One end-to-end test per flow's main path against a real
  server process on a real port with a real data directory and a synthetic
  library: claim and owner creation; add a library and scan; a new device
  signs in and takes a snapshot; play an album (sign, range, refresh,
  packaging); build a playlist; invite a friend who sees only their
  libraries; an unplayable file explained; back up, wipe and restore. Their
  failure branches stay at lower layers, as flows.md asks. Review added
  the cross-package round trips that no single package could test because
  their parts landed in the same wave: a cache rebuild brings back the
  queue, loves and playlists unchanged; an artist merge survives a rescan
  and a rebuild; a path-scoped refresh under a scoped token; the active
  device change reaching a second browser over the event channel.
- **Tests.** These are the tests.

## Coverage check: every R1 capability has an owner

This table maps every R1 capability in [api-needs.md](api-needs.md) to the
packages that deliver it. The R2 rows in that document are covered in the
outline below.

| Capability | Packages |
|---|---|
| API-SYS-01 health; SYS-03 negotiation; SYS-04 discovery; SYS-06 sign-in facts; SYS-09 API reference | WP-089 (with WP-039, WP-044, WP-118) |
| API-SYS-02 secure-context report | WP-073 |
| API-SYS-05 who am I | WP-087 |
| API-SYS-07 startup page | WP-095 |
| API-SYS-08 emergency page | WP-116 |
| API-SYS-10 client event channel | WP-083 |
| API-AUTH-01 to AUTH-03 claim, setup, owner | WP-080 (with WP-063); the passkey branch of owner creation in WP-106 (with WP-081) |
| API-AUTH-04 passkeys | WP-081 (with WP-041) |
| API-AUTH-05 password and two-factor | WP-120 (routes), WP-063 (verification, with WP-038) |
| API-AUTH-06 single sign-on | WP-096 |
| API-AUTH-07 guessing limiter | WP-064 (with WP-032), applied in WP-120, WP-080 and WP-094 |
| API-AUTH-08, AUTH-09 sessions and epochs | WP-062 (sign-in and sign-out routes in WP-120, cut-off in WP-082, push in WP-083) |
| API-AUTH-10 to AUTH-12 step-up and recovery | WP-106 |
| API-USR-01 profile identity | WP-087 (names), WP-103 (pictures) |
| API-USR-02 settings | WP-087 |
| API-USR-03, USR-04 sign-in methods and history | WP-106 (with WP-069) |
| API-USR-05, USR-06 export and history import | WP-108 (with WP-057) |
| API-USR-07 private session | WP-086 |
| API-DEV-01 device registry | WP-087 (with WP-062) |
| API-DEV-02 sync status | WP-084 |
| API-DEV-03 diagnostics from a device | WP-116 |
| API-SYNC-01 to SYNC-04 snapshot, delta, removals, profile data | WP-084 (with WP-066, WP-039) |
| API-SYNC-05 artwork sizes | WP-079, WP-103, WP-037 |
| API-SYNC-06 neighbour table | WP-058, WP-113 |
| API-SYNC-07 prebuilt search index, if needed | WP-054 (serialised form); shipping it is a small follow-up if the budget is missed |
| API-LIB-01 to LIB-05, LIB-07 libraries, folders, roots, grants, location, splitting rules | WP-099 (with WP-053, WP-060, WP-098; the grants table in WP-065) |
| API-LIB-06 rebuild | WP-095 |
| API-CAT-01 to CAT-08 catalogue fields | WP-040, WP-049 to WP-053, WP-075, WP-076, WP-077; seek and frame indexes stored by WP-067 and fetched through WP-084 (CAT-05) |
| API-CAT-09, CAT-10 availability and folder paths | WP-102, WP-110 |
| API-CAT-11 to CAT-13 inspect, merge and split, rescan one | WP-107 |
| API-STR-01 to STR-03 signing, refresh, byte ranges | WP-082 |
| API-STR-04 audio packaging | WP-056, WP-105 |
| API-STR-05 artwork bytes | WP-103 |
| API-SES-01, SES-02 session registry and stop | WP-104 |
| API-SES-03 active player | WP-085 (with WP-083) |
| API-SES-04 play reporting | WP-086 |
| API-QUE-01 to QUE-04 queue document, operations, rebase, positions | WP-025, WP-026, WP-085 |
| API-QUE-05 save queue as playlist | WP-093 |
| API-PL-01, PL-02, PL-07, PL-08 playlists, pins, tool writes, missing entries | WP-093 |
| API-PL-03, PL-04 M3U and folder playlists | WP-022, WP-112 |
| API-PL-05, PL-06 rule store and server evaluation | WP-027, WP-092, WP-113 |
| API-LOG-01 to LOG-05 events, offline merge, removal, counts, hides | WP-034, WP-068, WP-086 |
| API-HOME-01, HOME-02 layout and pins | WP-087, WP-059 |
| API-HOME-03, HOME-04 recently added without upgrades, reasons | WP-077, WP-059, WP-058 |
| API-HOME-05 search on the device | WP-054, WP-088 |
| API-SCAN-01, SCAN-03 scan and progress | WP-099, WP-102 |
| API-SCAN-02 path-scoped refresh | WP-098 |
| API-SCAN-04, SCAN-05 task list and activity | WP-070, WP-100 |
| API-HLTH-01, HLTH-02, HLTH-05 health | WP-110 |
| API-HLTH-03, HLTH-04 review queue and trash | WP-111 |
| API-ADM-01 to ADM-03 users and invitations | WP-094 |
| API-SET-01 network settings | WP-073 |
| API-SET-02 egress gate and network activity | WP-048, WP-097 |
| API-SET-03 sign-in settings | WP-096, WP-062 |
| API-SET-04, SET-05 backups and restore | WP-090, WP-109 (the full export in WP-108) |
| API-SET-06 updates | WP-074 |
| API-SET-07 alerts and logs | WP-097, WP-043 |
| API-SET-08 diagnostics | WP-116, WP-095 |
| API-SET-09 server identity | WP-089 |
| API-SET-10 restart and shut down | WP-095 |
| API-SET-11 listening-service and playlist imports | WP-108, WP-112 |
| API-TOK-01, TOK-02 tokens and tool change feed | WP-091 |
| API-TOK-03 deep links | WP-089 |

The background jobs in api-needs.md map the same way: library scan to
WP-102 and the parser-upgrade re-read to WP-123; change detection and path refresh to
WP-098; artwork processing to WP-079 and WP-103; loudness to WP-114;
neighbours and server-side rules to WP-113; playlist files to WP-112;
import matching to WP-108 and WP-112; change-log compaction to WP-066; root
health to WP-110; trash purge to WP-111; backups to WP-090; pre-upgrade
snapshots and cache rebuilds to WP-095; user-log recovery to WP-068; the
update check to WP-074; the free-space guard, alerts, log rotation and
crash records to WP-097; expiry sweeps to WP-070 with each owner
registering its own; open-response tracking to WP-082.

### R1 feature rows with no backend package

Review compared every R1 ID in the feature map's R1 cut with the IDs the
packages serve. The rows that no package named fall into four groups.

- **Now served.** ADM-002 to ADM-005 (release builds, the container image,
  pinned tags, service install) by the new WP-121; ADM-030 by WP-080;
  ACC-114, DIS-103 and MUS-158 (settings) by WP-087; ACC-128 by a test in
  WP-074; CLI-099 by WP-082; MUS-109 by WP-086; LIB-146, DIS-102, MUS-056
  and CLI-026 (fields and behaviour of the synced copy) by WP-040 and
  WP-084; MUS-051, MUS-054 and MUS-060 (artist and album aggregates and
  the genre index, computed on the device from synced fields) by WP-040;
  MUS-053 (sort keys) and MUS-055 (credits per track) through WP-036 and
  WP-053, which already produce them.
- **Client only.** These need nothing from the server beyond the synced
  copy and the routes above, so they belong to the client plan, not this
  one: CLI-031, CLI-060, CLI-062, CLI-070, CLI-135, CLI-136, CLI-138 to
  CLI-142, CLI-149, DIS-100, DIS-104, DIS-107, DIS-109 to DIS-112, MUS-052,
  MUS-072, MUS-073, MUS-076, MUS-108, MUS-113, MUS-227.
- **Documentation, not code.** ADM-011 (the hardware guide, from WP-115's
  numbers), CLI-002 (the published browser list) and the release notes
  ADM-003 implies. No package owns them; they need a documentation owner
  (see Review notes).
- **Reference rows.** The LIB, MUS, DIS, ACC and ADM references listed
  under the R1 cut ship with their owning rows and need no package of
  their own.
- **Partly served.** MUS-114 (the track info sheet) names the admin-only
  inspect API as its source; see WP-107 and Review notes.

## R2 (video) in outline

R2 adds movies and TV, the remuxer, sandboxed transcoding, the native apps
and everything that needs them (feature map, "Releases"). It reuses R1's
patterns: synced metadata, signed URLs, the user log, the session registry,
the worker and the task runner. The packages below are outlines: each will
get the same detail as R1 before its wave starts, and some will split. They
are numbered from WP-201 so R1 can grow without renumbering. Waves continue
from R1's last wave; R2 waves may begin while R1 waves 5 and 6 finish,
because they share no files.

The largest risk is the one ADR 1 names: the pure-Rust remuxer, with Dolby
Vision, lossless audio and image-based subtitles. Jellyfin and Plex already
transcode and remux reliably with FFmpeg, so Gunmetal is behind them until
WP-201 to WP-204 and WP-213 are done.

| ID | Title | Wave | Size | Depends on | Owns (outline) | Serves |
|---|---|---:|---|---|---|---|
| WP-201 | Matroska structure: segment, seek head, tracks, cues, chapters, tags, attachments | 7 | L (expect a split into structure and index) | WP-004, `ebml.rs` | `core/src/formats/mkv/` (and moves `ebml.rs` under `formats/`) | VID-001, LIB-088, the segment map (ADR 1 decision 4) |
| WP-202 | MP4 video tracks, edit lists and sample tables for video | 7 | L | WP-017, WP-018 | `core/src/formats/mp4/video.rs` | VID-001, VID-003 |
| WP-203 | Segment map builder | 8 | M | WP-201, WP-202 | `core/src/segment_map.rs` | ADR 1 decision 4; LIB-088 |
| WP-204 | Remuxer: fragmented MP4 and HLS playlists from the typed model | 8 | L (split by codec family) | WP-203, WP-056 | `core/src/remux/` | VID-003, SEC-MED-032, SEC-MED-074 |
| WP-205 | Subtitle parsers (SRT, WebVTT, ASS) and the WebVTT writer | 7 | L | WP-004, WP-005 | `core/src/subtitles/` | VID-069 to VID-076, SEC-MED-052 |
| WP-206 | XML without DTDs, and NFO | 7 | M | WP-004 | `core/src/xml.rs`, `core/src/nfo.rs` | LIB-097 (R2 part), SEC-MED-056 |
| WP-207 | Video catalogue model: films, shows, seasons, episodes, versions, naming rules | 7 | L | WP-040 | `core/src/video/` | LIB-002, LIB-149 to LIB-155, VID-013 to VID-015; API-VID-01 |
| WP-208 | Video decision engine with stream index | 8 | M | WP-055, WP-201, WP-202 | `core/src/decision/` | VID-002, VID-169, ADM-100 |
| WP-209 | Pairing protocol and device keys (core) | 7 | M | WP-031 | `core/src/pairing.rs` | ACC-061, ACC-062, ACC-051; API-AUTH-13 |
| WP-210 | Offline grants and download rules (core) | 7 | M | WP-027, WP-031 | `core/src/offline.rs` | CLI-095, CLI-096, CLI-080; API-DL-01, API-DL-03 |
| WP-211 | Control commands and the conflict rule (core) | 7 | M | WP-025 | `core/src/control.rs` | CLI-101, CLI-102; API-SES-05 |
| WP-212 | Offline edit replay rules | 7 | M | WP-034 | `core/src/userdata/replay.rs` | CLI-094 |
| WP-213 | Remux worker and segment serving | 9 | L | WP-204, WP-078, WP-082 | `worker/src/jobs/remux.rs`, `server/src/video_stream/` | VID-003, SEC-MED-081; API-STR-07 |
| WP-214 | Transcode sandbox launcher and FFmpeg profiles | 8 | L | WP-045 | `worker/src/transcode/`, FFmpeg build allowlist | VID-005, VID-009, SEC-MED-063 to SEC-MED-073 |
| WP-215 | Opus encode jobs and cache | 9 | M | WP-214 | `server/src/opus/` | MUS-106, MUS-213; API-STR-06 |
| WP-216 | Single-keyframe endpoint | 9 | S | WP-204, WP-213 | `server/src/keyframe/` | VID-097, VID-098; API-STR-08 |
| WP-217 | Video scan pipeline extensions | 9 | L | WP-102, WP-207, WP-201 | `server/src/scan/video.rs` | LIB-002, LIB-149 onwards |
| WP-218 | Pairing and device-key sign-in | 8 | M | WP-209, WP-062 | `server/src/pairing/` | ACC-061, ACC-062; API-AUTH-13 |
| WP-219 | WebSocket control channel, handoff and casting URLs | 8 | L | WP-211, WP-083 | `server/src/control/` | CLI-101 to CLI-111; API-SES-05 to API-SES-07 |
| WP-220 | Downloads: grants, resumable transfers, rights | 9 | L | WP-210, WP-082, WP-215 | `server/src/downloads/` | CLI-078 to CLI-097; API-DL-01 to API-DL-05 |
| WP-221 | iroh remote access and relays | 8 | L | WP-218 | `crates/gunmetal-remote/` | ACC-096, ACC-100, ACC-101; API-SET-12 |
| WP-222 | Household profiles, policies, parental controls, PINs | 8 | L | WP-033, WP-065 | `server/src/household/` | ACC-016 to ACC-034, ACC-038 to ACC-044; API-USR-08, API-ADM-04, API-AUTH-14 |
| WP-223 | Share links and the guest capability | 9 | M | WP-222 | `server/src/shares/` | ACC-091, ACC-135 |
| WP-224 | Webhooks and the public event stream | 8 | M | WP-048, WP-083 | `server/src/webhooks/` | INT-030 to INT-049; API-TOK-04 |
| WP-225 | WebAssembly plugin host with grants | 8 | L | WP-048, WP-047 | `crates/gunmetal-plugins/` | INT-054 to INT-069; API-TOK-05 |
| WP-226 | Scrobbler and lyrics-lookup plugins | 9 | M | WP-225, WP-086 | `plugins/` | MUS-192 to MUS-195, MUS-162 |
| WP-227 | OpenSubsonic adapter | 8 | L | WP-065, WP-082, WP-093 | `server/src/subsonic/` | INT-086, INT-087, ACC-130; API-TOK-06 |
| WP-228 | Metadata editing, locks and item history | 8 | M | WP-107 | `server/src/editing/` | LIB-172 to LIB-178; API-CAT-14 |
| WP-229 | Importers for Plex, Jellyfin, Navidrome and iTunes data | 8 | L | WP-057, WP-108 | `server/src/importers/` | ADM-036 to ADM-049 |
| WP-230 | Intro detection and skip markers | 10 | M | WP-214, WP-217 | `worker/src/jobs/intro.rs`, `server/src/markers/` | VID-110 to VID-116; API-VID-03 |
| WP-231 | UniFFI bindings for the native apps | 8 | M | WP-088's surface | `crates/gunmetal-ffi/` | ADR 1 decision 2; CLI-022 on native |
| WP-232 | Resume points, Continue Watching and Next Up | 8 | M | WP-034, WP-207 | `core/src/video/progress.rs`, `server/src/progress/` | VID-118 to VID-122, DIS-024 to DIS-031; API-LOG-07, API-VID-02 |
| WP-233 | Statistics and year in review | 8 | S | WP-034 | `core/src/stats.rs` | MUS-186, MUS-187; API-LOG-06 |
| WP-234 | Partial sync and restricted-profile filtering | 9 | M | WP-084, WP-222 | `server/src/sync/partial.rs` | CLI-023, DIS-144, DIS-155; API-SYNC-08, API-SYNC-09 |

R2 also needs the owner's answers on the Linux desktop shell, Dolby Vision
on DV televisions, transcoding scope and relay funding (feature map open
decisions 10, 19, 20 and 21) before the packages that depend on them start.

## Decisions the owner must make

Each item says which packages wait for it. Recommendations are this plan's;
the owner may choose otherwise.

1. **Accept ADR 3, durable user state (WP-002).** Blocks WP-034, WP-035,
   WP-046, WP-068 and, through them, most of R1. *Recommendation:* accept
   the shape in this plan: per-profile and household log streams in
   framed, checksummed monthly segments; documents stored as operations
   with periodic snapshots; the identity store as its own SQLite file with
   migrations after R1.
2. **Accept ADR 4, the audio packager (WP-003).** Blocks WP-056 and WP-105.
   *Recommendation:* accept; without it, browser gapless depends on each
   browser's native Media Source Extensions support, which is unverified.
3. **Accept or reject ADR 5, in-process audio decoders (WP-003).** Blocks
   WP-029 and WP-114. *Recommendation:* accept Symphonia inside the
   sandboxed worker only, after its review under SEC-MED-026; R1 still
   ships the tag-and-fallback path for Opus and HE-AAC, which it reportedly
   cannot decode.
4. **The core's dependencies and the wire format.** The core has none today.
   This plan adds `serde`, `postcard`, `hmac`, `sha2`, `sha1` and
   `unicode-normalization`. *Recommendation:* accept these; the alternative
   to `serde` and `postcard` is a hand-written codec for every protocol
   type, which is a large amount of code under the mutation gate for no
   security gain once frames are capped and revalidated. Two further
   questions belong here: whether `serde_json` may join the core for the
   history-import parsers (WP-057; the plan's default keeps JSON decoding
   in the server), and the "not used, on purpose" list, which is this
   plan's proposal rather than a settled rule.
5. **The crate layout.** Twelve crates, with the core kept as one crate as
   ADR 1 says. *Recommendation:* accept, and record it in an ADR that
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
   *Recommendation:* use the provider rustls defaults to, and revisit when a
   pure-Rust provider is proven.
9. **WebAuthn implementation and algorithms.** *Recommendation:* hand-written
   verification on RustCrypto primitives for ES256 and EdDSA (WP-041,
   WP-081), and decide on RS256 after checking which authenticators the
   household is likely to use still require it (unverified). Allow recorded
   real ceremonies as test fixtures, since they are not media.
10. **OIDC implementation and RSA.** *Recommendation:* hand-written, minimal code
    flow with pinned algorithms (WP-096), because the general-purpose crate
    brings a larger dependency tree than the one flow R1 needs. Either
    way, the owner must also choose whether R1 verifies RS256, which
    needs an RSA crate the plan did not list, or supports only providers
    configured for ES256 or EdDSA, which would exclude providers left on
    their defaults (unverified per provider).
11. **Public IDs for library items.** Random and stored, or derived from
    content identity with a server key (web-and-api-security.md open
    decision 3). *Recommendation:* keyed derivation plus an alias table for
    merges, so IDs survive cache rebuilds without a mapping to back up.
12. **Update-feed verification and backup encryption.** *Recommendation:*
    `tough` for the feed only if its dependency tree passes review,
    otherwise a signed document with an Ed25519 key and the rollback rules;
    the `age` format for backups, because a reviewed format is safer than a
    new envelope. The archive inside the encryption (hand-written or the
    `tar` crate) is part of the same choice and is unsized.
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
    low-end devices so WP-054 and WP-084 can assert their budgets.
16. **What R1 does with writes while offline.** *Recommendation:* as
    api-needs.md proposes: only plays and positions are queued; loves,
    ratings and playlist edits are disabled with "Needs the server", and
    the event format already allows R2 to queue them.
17. **Rating model** (feature map open decision 12). Blocks WP-034's event
    bodies. *Recommendation:* as the feature map recommends.
18. **Server platforms in R1 and the watcher.** *Recommendation:* Linux
    (x86-64 and ARM) is the tested R1 server platform; macOS and Windows
    builds run with the reduced isolation tier and their own beneath-root
    tests before being called supported; use `notify` for watching.
19. **Who sees a new library** (flows G4). *Recommendation:* the owner and
    admins only, until someone is granted access.
20. **Compressed ID3v2 frames.** *Recommendation:* skip and record them in
    R1, so the core needs no inflater yet.
21. **Built-in HTTPS by ACME in R1** (WP-101). *Recommendation:* ship it in
    R1 only if WP-073 and the egress gate land early; otherwise a point
    release, with the docs pointing at a reverse proxy meanwhile. Review
    note: WP-073 now lands in wave 3 and WP-101 in wave 4, and the ACME
    client's dependencies are not yet chosen.
22. **The benchmark library.** *Recommendation:* the synthetic generator in
    CI for regressions, plus a published comparison run by the owner on a
    real library they own, with its shape described but no files shared.
23. **Configuration format.** *Recommendation:* TOML.
24. **32-bit ARM.** `seccompiler` does not support it. *Recommendation:*
    build it, label the isolation tier as reduced on the health page, and
    say so in the hardware guide.
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
    *Recommendation:* folder playlists are read-only with "Duplicate to
    edit"; a purged track stays in a playlist as a "missing" entry that
    rematches by identity.
30. **What admins and members see of other people's sessions** (feature
    map open decision 13). Blocks WP-104's visibility test. It was cited
    there but missing from this list. *Recommendation:* as the feature map
    recommends: live sessions, totals and security events by default.
31. **Artwork at cookie-authorised, content-addressed paths** (api-needs.md
    Flags item 7). ADR 1 decision 6 and ACC-122 call for short-lived
    signed URLs for media and images; the proposal serves first-party web
    artwork at stable paths authorised by the session cookie so the
    browser can cache it. Blocks the caching part of WP-103.
    *Recommendation:* accept for the first-party web client on its own
    origin only, and record it as an amendment in a new ADR rather than
    by editing ADR 1.
32. **Release signing keys** (feature map open decision 25). WP-074
    compiles a feed root into the binary and WP-121 builds releases; both
    presume keys that do not exist yet. *Recommendation:* as the feature
    map recommends: an offline root held by at least two maintainers,
    short-lived online signing keys, and a published rotation and
    compromise procedure before R1 ships.
33. **Scope of the `std::fs` ban** (SEC-MED-033). Read literally, it bans
    path-based `std::fs` everywhere outside the filesystem module, which
    would forbid the cache, the identity store, the user log, the audit
    log, the root secret and the data-directory layout from writing their
    own files. *Recommendation:* the ban covers library roots; the named
    data-directory modules in WP-001 are the only exceptions, checked by
    an xtask so the list cannot grow silently.
34. **`unsafe` in the WASM facade.** The workspace forbids `unsafe_code`
    everywhere, while the README promises only "no `unsafe` in the core".
    If `wasm-bindgen`'s generated code trips the forbid lint (unverified),
    WP-088 needs a crate-level exception. *Recommendation:* allow it in
    `gunmetal-wasm` only, where the code is generated glue, and keep it
    forbidden everywhere else.
35. **Service install and "one command"** (ADM-005). SEC-MED-063 forbids
    the server from starting programs outside the sandbox launcher, so the
    binary cannot create the service user or enable the unit itself.
    *Recommendation:* the binary prints the unit and a shipped install
    script does the privileged steps (WP-121), and ADM-005's "one command"
    is that script.

## Review notes

An adversarial review on 2026-10-02 checked the plan against the rules in
CONTRIBUTING.md and AGENTS.md, the feature map's R1 cut and the R1 rows of
api-needs.md. It changed the plan in place. What changed, and what it
could not settle, is below.

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
  readers take a `LibrarySet`, and WP-065 owns the grants table.
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

- **The single sign-on client secret.** WP-096 must send the client
  secret to the provider, which needs its bytes outside
  `gunmetal-secrets`. Either the egress crate becomes a second sanctioned
  place to expose a secret, or the secrets crate builds that one request
  header itself. Neither is designed here.
- **`LibrarySet` is a convention, not a guarantee.** It has a public
  constructor in the core, so "only the access module builds one from a
  principal" relies on review. A sealed constructor would need the type
  to live in the server, which reintroduces the same-wave tangle this
  review removed. A follow-up could add an xtask check for constructor
  calls outside `access/`.
- **Who consults the free-space guard.** WP-097 decides when a write
  would fill the disk, but the writers belong to packages that merge
  earlier, and no package is assigned to wire the guard into them.
- **The track info sheet for members (MUS-114).** Its source is the
  admin-only inspector. The plan assumes members see synced technical
  fields only; the owner or the UI plan should confirm.
- **Secure context at setup before WP-073.** WP-080 decides whether to
  offer passkeys from the request, but trusted-proxy handling (WP-073) is
  in the same wave, so behind a proxy the decision is only proven end to
  end in WP-117.
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
