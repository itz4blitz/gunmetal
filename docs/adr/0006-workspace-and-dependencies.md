# 6. Workspace, crates and dependencies

Date: 2026-10-03
Status: accepted, through the owner's answer to D-02 on 2026-10-02
([decision register](../decisions.md#owner-answers-2026-10-02)): the owner
delegated these technical choices and accepted the recommended defaults.
Extends decision 2 of [record 1](0001-architecture.md) and does not
replace it.

## Context

Record 1 decision 2 puts protocol types, the decision engine, the parsers
and the remuxer in one core crate, compiled into the server, the native
apps and the web client. It says nothing about the rest of the workspace.
Three things shape the rest:

- The security baseline enforces many of its rules per crate or per
  module: only one module may open files, start a process, make an
  outbound request or use a cryptographic crate (first principle 8, "one
  door per risk").
- Many coding agents build packages in parallel, and each needs whole files
  it alone owns.
- Every crate is under the same gate: 100% coverage and zero surviving
  mutants.

The build plan proposed a layout, a dependency direction and a list of
external crates ([work-packages.md](../plan/work-packages.md#crate-layout)).
This record writes down what the owner accepted, with the owner's other
answers applied: no project name service in R1 (D-07), R1 servers on Linux
only with media processed in a jailed worker (D-09), and records
[4](0004-audio-packager.md) and [5](0005-audio-decoders-in-the-scan-worker.md).

## Decisions

1. **The core stays one crate.** `gunmetal-core` does no I/O, has no
   `unsafe` and never panics on any input. Formats live under
   `src/formats/`, tag mapping under `src/tags/`, the music model under
   `src/music/`, and every other subsystem in a module of its own. The
   existing `src/ebml.rs` stays where it is until the Matroska work in R2
   moves it under `src/formats/`.
2. **Fourteen crates, each with one reason to exist.** Each is created by
   the package whose first failing test needs it (AGENTS.md), never ahead
   of time.

   | Crate | Kind | Holds | Why separate |
   |---|---|---|---|
   | `gunmetal-core` | library, pure | Parsers, tag mapping, the music model, the queue, rules, search, gain and loudness arithmetic, the decision engine, the player state, tokens, the authorisation policy, user-event merge rules, log framing, the wire codec, the audio packager (record 4), protocol types | Record 1: one core for the server, the web client (WASM) and, from R2, the native apps |
   | `gunmetal-fuzz` | library, dev-only (exists) | One plain-function fuzz harness per parse entry point, replayed by stable `cargo test` over `fuzz/seeds/`; the nightly project in `fuzz/` calls the same functions | Harnesses callable from stable tests (SEC-MED-027, SEC-MED-028), kept out of the shipped crates |
   | `gunmetal-testkit` | library, dev-only | Builders for synthetic media, independent checksums, temporary directories, a manual clock, the synthetic library generator | Shared test oracles; `publish = false`, used only as a dev-dependency, under the gate like any crate |
   | `gunmetal-store` | library, I/O | The rebuildable SQLite cache: the single writer, schema registration, the change log, the catalogue tables | One writer in one place (ADM-080) |
   | `gunmetal-durable` | library, I/O | The identity store, the user log segments, the audit log, the derived-data store | Durable state has its own rules (fsync, migrations, never discarded), set by record 3 |
   | `gunmetal-secrets` | library, I/O | The root secret, key derivation and rotation, `Secret<T>`, the vault, the one CSPRNG function, public-ID minting, and the crypto module | `Secret::expose` and every cryptographic crate but SHA-256 stay inside it |
   | `gunmetal-fs` | library, I/O | The data-root handle, library-root handles, the open rules, symlink policy, walking, fingerprints, watching, and the one SQLite connection opener | Path-based `std::fs` and `rusqlite::Connection` opens are banned everywhere else (SEC-MED-033, SEC-HIS-016) |
   | `gunmetal-worker` | library, I/O | The sandbox launcher, the IPC framing, the Linux sandbox, the worker pools and quarantine, the jobs (probe, artwork, hashing, packaging, loudness, imports, playlist files) | The only place a process starts (SEC-MED-063); the image decoder and any audio decoder link only here |
   | `gunmetal-http` | library | The route table type, the request pipeline, security headers, body limits, the problem renderer | The route table is the single source for the allow-list, role-matrix and header tests and the API reference |
   | `gunmetal-egress` | library, I/O | The only outbound HTTP client, with its egress gate and its record of connections | Makes the ban on every other outbound client enforceable (SEC-EXT-001, SEC-API-076) |
   | `gunmetal-names` | library and a service binary | The project's per-server name service: label codec, DNS answers, registration rules | A project service, not part of the shipped server. Not in R1: it waits for the name service (D-07) |
   | `gunmetal-server` | library and the `gunmetal` binary | Application state, every route handler, the jobs, the command line | The composition root; each feature package owns one module directory in it |
   | `gunmetal-wasm` | library, `cdylib` | A thin `wasm-bindgen` facade over the core for the web client | Keeps WASM bindings, and any `unsafe` they need, out of the core |
   | `xtask` | binary, dev-only | Repository checks: the harness registry, lockfile age, lint exceptions, the core allowlist, docs lints and traceability, repository settings and releases, the limits register, the benchmark runner | The baseline asks for these checks in Rust under the same gate |

3. **One shipped binary.** The worker is the `gunmetal` binary started by
   the sandbox launcher with a hidden subcommand, so only one binary ships
   to households (media-and-parser-safety.md, section 4).
4. **Dependency direction.** The core depends on nothing in the workspace.
   Every I/O crate depends on the core, and on the crate that owns a door
   it passes through: every SQLite user on `gunmetal-fs`, the egress client
   on `gunmetal-secrets` for its TLS configuration. `gunmetal-server`
   depends on all of them. `gunmetal-wasm` depends only on the core.
   `gunmetal-testkit` depends on nothing in the workspace, so it can never
   borrow the code it checks. `gunmetal-fuzz` depends on the core and, for
   one purpose only, on `gunmetal-worker`: SEC-MED-026 requires Gunmetal's
   wrappers around third-party decoders (the artwork job's image decoder,
   WP-079, and the audio decoder of record 5) to be fuzzed, and those
   wrappers live in the worker. `xtask` may depend on any workspace crate.
   Nothing depends on `gunmetal-fuzz`, `gunmetal-testkit` or `xtask` except
   as a dev-dependency, and there are no cycles.
5. **External crates.** The table lists every crate the owner approved and
   where it is used. Where a crate may be used in one place only, the
   entry says "only", and decision 8 enforces it. Approval means a crate
   may be proposed; it lands only when a test needs it, through a
   dependency request that the integrator applies to the root `Cargo.toml`
   and `supply-chain/`, after the checklist in
   [supply-chain-and-release.md](../security/supply-chain-and-release.md#5-dependency-policy):
   cargo-deny, a cargo-vet audit (SEC-SUP-024) and the seven-day age rule.
   Versions are pinned in `[workspace.dependencies]`, never here.

   | Crate | Used by | Why |
   |---|---|---|
   | `proptest` | every crate, dev-only | Property tests; already in use |
   | `serde` with derive | core, server, durable, worker | Protocol types serialise to JSON for the API and to a compact binary form for sync and IPC, without a hand-written codec per type |
   | `postcard` | core | Compact binary frames for sync and worker messages, decoded inside the 32 MiB frame cap and revalidated through typed constructors (SEC-MED-023) |
   | `serde_json` | server, durable, worker | JSON for the HTTP API, the audit log and exports. Uploaded history exports are hostile, so they are decoded only in the worker's import job, never in the server process (SEC-MED-018, SEC-MED-020) |
   | `sha2` | core (`src/crypto.rs` only), secrets (`src/crypto/` only) | SHA-256 for the schema digest and content-identity windows, and where HMAC and HKDF need it |
   | `hmac`, `hkdf`, `chacha20poly1305`, `argon2`, `ed25519-dalek`, `p256`, `age` | secrets (`src/crypto/` only) | Keyed hashes, key derivation, secrets at rest, the optional share-link password (R1.2 under D-10), signatures, passkey assertions, encrypted backups. Algorithms and parameters are record 9's |
   | `getrandom` | secrets (the one CSPRNG function) | The operating system's CSPRNG (SEC-STD-022) |
   | `rustls`, `tokio-rustls`, and the provider record 9 chooses (`aws-lc-rs` if it builds cleanly for the R1 targets, otherwise `ring`, D-08) | server and egress use them; the provider and configuration constructors only in secrets (`src/crypto/`) | HTTPS with the owner's certificate and outbound TLS, verified against the WebPKI with no "dangerous" configuration |
   | `unicode-normalization` | core | Diacritic-insensitive search and natural sort need canonical decomposition |
   | A pure-Rust inflate crate (candidate `miniz_oxide`) | core (`src/inflate.rs` only) | The one bounded decompression helper (WP-128, SEC-MED-009) |
   | `tokio` | server, egress, fs | The async runtime, timers and blocking pool |
   | `axum` on `hyper` and `tower` | http, server | Composable middleware and an enumerable router; server-sent events for the R1 event channel |
   | `rusqlite` with `bundled` | fs opens connections; store and durable use them | SQLite with the amalgamation built in, so every platform runs the same version |
   | `cap-std` | fs | Opens beneath a root with `openat2` and `RESOLVE_BENEATH` (SEC-MED-033, SEC-HIS-016) |
   | `rustix` | fs, worker | Safe wrappers for `pread`, `fstat`, descriptor passing, rlimits and `prctl`, with no `unsafe` in Gunmetal's code |
   | `landlock`, `seccompiler` | worker | Filesystem, network and system-call confinement of workers (SEC-MED-022) |
   | `notify` | fs | File-change notification for library roots |
   | `toml` | server | The configuration file is TOML |
   | `image` (JPEG, PNG, WebP and GIF only) | worker only (the artwork job) | Artwork decoding and re-encoding with `image::Limits`, after the review SEC-MED-026 requires (WP-079) |
   | `symphonia` (FLAC, MP3, AAC-LC and Vorbis only) | worker only (the loudness job) | Decoding for loudness measurement, on the conditions of record 5 |
   | `wasm-bindgen` | wasm | The web client calls the core through it |

   The owner's answer to D-08 settles the cryptographic choices that
   records 9 and 10 write up; where those records say more, they win. Two
   crates are approved only on a condition: `rsa`, for verifying RS256 ID
   tokens from OIDC providers, only if it passes review; and `tough`, for
   the TUF update feed, only if its dependency tree passes cargo-vet,
   otherwise a minimal TUF client is written. Passkey verification is
   written by hand on `p256` and `ed25519-dalek`, the OIDC code flow is
   written by hand as the plan recommends (its decision 10), and the
   backup archive is a small hand-written format, so none of these needs a
   crate of its own. The ACME client for certificates on the owner's own
   domain (WP-101) is not chosen: that package brings a dependency request
   for a crate such as `instant-acme` with `rcgen`, or writes the client on
   the primitives above.
6. **The core's reviewed allowlist.** The core's normal dependencies are
   `serde`, `postcard`, `sha2`, `unicode-normalization` and the inflate
   crate, with what they pull in, and nothing else (SEC-SUP-025). JSON
   decoding never enters the core. The allowlist itself is kept in
   `supply-chain/` and checked by an xtask (WP-001, WP-008).
7. **Not used, on purpose.** This is a rule, not a proposal; changing it
   takes a new record. It governs what Gunmetal's own crates depend on
   directly. A crate that arrives underneath an approved one (`tempfile`
   under `proptest`, say) is judged in that crate's review instead.
   - No `chrono`, `time` or `jiff`: the server needs only Unix timestamps
     and RFC 3339, which the core implements.
   - No `clap`: the command line has a handful of fixed subcommands.
   - No `tracing-subscriber`, `log` or `env_logger`: one small structured
     logger meets the escaping rules more easily than a configurable one.
   - No `anyhow` or `thiserror`.
   - No `uuid`: MusicBrainz IDs are parsed into 16 bytes by hand.
   - No `base64` crate: the core owns its two small codecs.
   - No `tempfile`: the testkit creates temporary directories.
   - No `reqwest`: the egress crate uses `hyper`'s client directly.
   - No decompression of ID3v2 frames in R1: compressed frames are skipped
     and recorded (D-03).
8. **Each ban has one owner, and every exception is visible.** A
   `disallowed-methods` or `disallowed-types` entry in `clippy.toml` bans
   an operation in the whole workspace. The one module that may perform it
   marks its sanctioned call with a module-level
   `#[expect(clippy::disallowed_methods, reason = "...")]`, and an xtask
   check (WP-008) fails when such an exception appears in a module that is
   not on this list:

   | Operation | Only in | Requirements |
   |---|---|---|
   | Path-based `std::fs` | `gunmetal-fs`, and the dev-only `gunmetal-testkit` (the one exception, owner decision 33) | SEC-MED-033, SEC-HIS-016 |
   | Opening a `rusqlite::Connection` | The connection opener in `gunmetal-fs` (WP-126) | SEC-API-066, SEC-TM-039 |
   | Starting a process | The sandbox launcher in `gunmetal-worker/src/sandbox/` | SEC-MED-063 |
   | Outbound sockets and HTTP clients | `gunmetal-egress` | SEC-EXT-001, SEC-API-076 |
   | Security randomness | The one CSPRNG function in `gunmetal-secrets` | SEC-STD-022 |
   | Cryptographic crates | `gunmetal-core/src/crypto.rs` (SHA-256 only) and `gunmetal-secrets/src/crypto/` (everything else, including the rustls provider and configuration constructors) | SEC-STD-018, SEC-STD-019 |
   | `Secret::expose` | `gunmetal-secrets` | SEC-OPS-013 |
   | Building a `Minted` public-ID value | The minting function in `gunmetal-secrets` | SEC-HIS-012 |
   | Reading a client address (`ConnectInfo`, `peer_addr`, forwarding headers) | `gunmetal-server/src/listener.rs` and the core's `http/forwarded.rs` | SEC-OPS-037, SEC-NET-016 |
   | Pre-sizing a buffer, in the core | The core's one bounded-capacity helper (WP-004) | SEC-MED-003 |
   | Calling an audio decoder crate | `gunmetal-worker/src/jobs/loudness.rs`, added when the crate lands (record 5) | SEC-MED-018, SEC-MED-026 |

   Record 4 adds one more door that is not a third-party API: production
   code calls the core's audio packager only from
   `gunmetal-worker/src/jobs/package.rs`, and a check in the gate fails if
   the server crate calls it (SEC-MED-018). The core's own tests and fuzz
   harness still call it, so that check looks at the server crate rather
   than banning the functions everywhere.

   A later package that needs one of these operations calls the module
   that owns it. It never adds an exception; a change to the list is its
   own small package under the merge protocol.
9. **`unsafe` is forbidden in every crate.** The workspace sets
   `unsafe_code = "forbid"`. The one possible exception is `gunmetal-wasm`,
   and only if the glue `wasm-bindgen` generates trips the lint, which is
   unverified; it would be a crate-level override that states the reason.
   macOS and Windows worker sandboxes need FFI. Where that `unsafe` may
   live is decided by its own record before those server builds exist
   (SEC-MED-082, R2).
10. **Platforms.** R1 server builds are Linux on x86-64 and AArch64, plus
    the container image (D-09). The worker's sandbox is Linux-only in R1.
    The core also builds for `wasm32` for the web client, and its tests run
    on `i686` in CI (SEC-MED-004). 32-bit ARM server builds (ADM-004,
    R1.3) report reduced isolation and are not called supported until the
    seccomp answer for them is recorded (SEC-MED-024).

## Security review record

Boundary TB12 (project to installs). Threat TM-T41 (a malicious or
compromised dependency ships in a release).

This record is the design review that WP-003 owes for the workspace. It
was drafted by a coding agent on 2026-10-03 and becomes the review record
when a maintainer approves the pull request that adds it (AGENTS.md).

Verifies: SEC-MED-018, SEC-MED-024, SEC-MED-026

- **SEC-MED-018.** Untrusted media is parsed by the core's sans-I/O code
  and by the reviewed decoders, and only inside a worker: every decoding
  dependency belongs to `gunmetal-worker` alone (decisions 2, 5 and 8).
  The server crate links the core but has no sanctioned call to the
  packager or a decoder.
- **SEC-MED-024.** The sandbox launcher, the confinement and the tier
  table live in one crate, `gunmetal-worker`, so there is one
  implementation of the tiers to test (decision 2).
- **SEC-MED-026.** The only third-party crates that decode media, `image`
  and `symphonia`, are listed with that review as their condition, only in
  the worker, and their wrappers' fuzz harnesses have a home in
  `gunmetal-fuzz` (decisions 4 and 5).

## Consequences

- Fourteen `Cargo.toml` files, each owned by one package, so parallel work
  never edits another package's manifest. New crates need no edit to the
  root `Cargo.toml`, whose `members = ["crates/*"]` already picks them up.
- The gate runs over a growing workspace; how it scales is the owner's
  answer to D-01 (the diff-scoped gate on package pull requests, the full
  gate on each wave branch's pull request into `main`).
- A crate not in the table needs a dependency request and a new record
  that amends this one.
- Record 1 is not edited (AGENTS.md); this record extends it.
