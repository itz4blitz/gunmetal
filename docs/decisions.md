# Decisions for the owner

Date: 2026-10-02. Status: the decide-first decisions are answered (see
[Owner answers](#owner-answers-2026-10-02)); the rest stay open unless an
answer below covers them. This is the single register of every decision
the owner must make before and during the build of Gunmetal.

It collects the open decisions from the
[feature map](features/README.md#open-decisions-for-the-project-owner),
the [security baseline](security/README.md#open-decisions-for-the-owner)
(including its item 25 and the smaller choices in each security file),
the [interface documents](ui/README.md#decisions-that-made-the-four-documents-consistent),
the [API needs](plan/api-needs.md) and the
[work packages](plan/work-packages.md#decisions-the-owner-must-make), plus
every change that the 2026-10-02 security alignment applied to the
planning documents on the owner's behalf. The same question asked in
several documents appears here once, citing every source.

How to read it:

- **Decide first** lists the ten decisions that block the build, in the
  order they block it. Nothing in wave 0 of the plan merges, and no server
  code stores a user, until the first six are answered.
- **R1 scope** is the much smaller first release the owner adopted on
  2026-10-02 (D-10), with the rest in point releases R1.1 to R1.3 and the
  name service in R2 (D-07). The feature map, the security baseline, the
  UI documents, the API needs and the work packages were realigned to it
  on 2026-10-03.
- **Applied by default** lists every change the alignment made, with what
  the document said before, what it says now and the requirements behind
  it. Changes marked "Confirm: yes" touch one of the baseline's open owner
  decisions, so the owner confirms or reverses them. Changes marked "no"
  follow a requirement directly; reversing one means changing that
  requirement first.
- **All other decisions** holds the rest, grouped by theme.

Every decision has an ID (D-01 to D-88) and every applied change an ID
(A-001 to A-590), so answers can be recorded by ID. Where a
recommendation below differs from the one in a source document, the
source's recommendation was written before the security baseline and this
register follows the baseline. Requirement IDs (SEC-...) are those of
[docs/security](security/README.md); feature IDs (MUS-, LIB-, DIS-, CLI-,
ACC-, ADM-, INT-, LAT-, VID-, LIV-) are those of the feature map; WP- IDs
are work packages.

Source abbreviations: "features README n" is open decision n of the
feature map README; "security README n" is item n of the baseline's owner
decisions; "identity OD-n", "network OD-n" and so on are open decisions in
the named security file; "WP n" is item n of "Decisions the owner must
make" in work-packages.md; "api-needs flag n" is item n of "UI
requirements the architecture makes hard or impossible"; "UI decision n" is
item n in docs/ui/README.md.

## Owner answers, 2026-10-02

The owner answered these in conversation on 2026-10-02. Where an answer
differs from a recommendation further down this register, the answer
wins. On 2026-10-03 the planning documents were realigned to the answers
on D-07, D-10 and remote access in R1, as those rows record.

| Decision | Answer |
|---|---|
| D-01 How parallel work merges | Each wave is built on its own branch (`wave-0`, `wave-1`, ...). Package pull requests merge into the wave branch once the gate passes; a dedicated integrator agent owns the shared files (root `Cargo.toml`, `scripts/gate.sh`, `ci.yml`, `supply-chain/`). The owner reviews and merges one pull request per wave into `main`. Pull requests into wave branches use the diff-scoped gate; the wave branch's pull request into `main` runs the full gate. Agents never merge into `main`. |
| D-02 Workspace, dependencies and ADR 6 | Delegated: the owner asked for purely technical choices to be settled with the recommended defaults and recorded as architecture records. Recommendation accepted. |
| D-03 Parser and worker limits | Delegated; recommendation accepted (the stricter value wins wherever two documents disagree). |
| D-04 Public IDs | Delegated; recommendation accepted (random 128-bit IDs). |
| D-05 ADR 3, durable user state | Delegated; recommendation accepted. |
| D-06 Identity and sessions | No passwords, TOTP or emailed codes. Passkeys, OIDC and approval from a signed-in phone, with the recovery ladder. Recommendation accepted. |
| D-07 HTTPS and naming | **Differs from the recommendation.** R1 gets HTTPS through the owner's own domain (automatic certificates), a tailnet, or the same machine. The project-run per-server name service is not in R1; it moves to a later release. **Documents realigned 2026-10-03:** the name service (ADM-023), its naming client and its certificate-transparency monitoring are R2, beside built-in remote access. SEC-NET-010 to SEC-NET-012 and SEC-NET-069 to SEC-NET-071 moved to R2 with them. SEC-NET-013, SEC-NET-072 and SEC-OPS-007 stay R1 because they also protect the own-domain, tailnet and localhost paths. The feature map, the baseline, the UI documents, api-needs.md and work-packages.md (WP-129 and WP-135 to R2) follow this. |
| D-08 Cryptography and backup archives | Delegated; recommendation accepted. |
| D-09 Media processing outside the server process | Parsing, remuxing and decoding run in a separate jailed worker. This supersedes "remux in-process" in ADR 1. R1 servers are Linux only: x86-64, ARM64 and a Docker image. No transcoding in R1. |
| D-10 R1 scope | The smaller R1 proposed in [R1 scope](#r1-scope) is adopted, with the rest in point releases R1.1, R1.2 and R1.3. Built-in metadata lookups (MusicBrainz, cover art) ship in R1.1, behind the setup question that lists what each provider receives. **Documents realigned 2026-10-03:** the allowed Release values are now R1, R1.1, R1.2, R1.3, R2, R3, Later and No (Withdrawn for retired requirements). Every feature row's Release matches [R1 scope](#r1-scope): 296 rows in R1, 79 in R1.1, 43 in R1.2 and 32 in R1.3. The five rows leaving the R1 line are now R2, ADM-023 among them under D-07. 25 requirements moved with their surfaces to R1.1 (7) and R1.2 (18). With D-07's six, 576 of the earlier 607 remain due in R1. The feature map README, the security README, the UI documents, api-needs.md and work-packages.md (wave 0 unchanged) follow this. |
| Remote access in R1 | Through the owner's own reverse proxy or a tailnet. Built-in remote access (iroh) arrives in R2. **Documents realigned 2026-10-03** to this answer. |
| What admins see | Who is playing and totals, not what, unless each person opts in. No history. |
| Update and advisory check | A required setup question with no answer preselected. |
| Remaining security recommendations | Accepted as written, including every change under [Applied by default](#applied-by-default) except where an answer in this table says otherwise. |
| Second maintainer | The build proceeds now; a second trusted maintainer is required before R1 is tagged. |
| Security contact | GitHub only for now; no security@ mailbox yet. |
| GPUI | Not used for the main apps; revisit for a native desktop app later ([record 11](adr/0011-gpui-not-adopted.md)). |
| D-83 to D-88 (answered 2026-10-03) | Every recommendation accepted: a minimal read-only details view in R1 with the full sheet in R1.1; nothing merges silently before the review queue; the rule format and its limits ship in R1.1 work, with the editor and smart playlists in R1.3; R1's web app needs the server reachable to load; speed budgets are enforced by tests in the R1 gate, with published numbers in R1.1; R1 browses by genre, with mood and label in R1.1. |
| FUSE library folders (answered 2026-10-03) | Allowed, read-only, with the same path and symlink checks as any other folder; the library's health page shows the filesystem type. Unraid's `/mnt/user` is FUSE, so refusing it would break most Unraid installs. This changes the default WP-126 shipped in wave 0. |
| DCO sign-off (answered 2026-10-03) | Agent commits carry no `Signed-off-by`. The owner's merge commit for each wave into `main` carries the owner's sign-off, covering the wave. |
| Erasure ledger (answered 2026-10-03) | Kept for the life of the data directory, holding only IDs and clock values, so erased history cannot return from any older backup. |
| Building order (answered 2026-10-03) | Each wave starts on top of the previous wave branch while the owner reviews it; review changes are folded in by the next wave's integrator. |
| Web player (answered 2026-10-03) | Start the web player now. It is planned and built in parallel with the server waves, against a small fake server, so there is a real screen within days, and it moves to the real server as the server waves land. The owner chose this over a throwaway thin slice and over finishing the server first. |
| Facade crates (answered 2026-10-03) | Approved: `tsify` and `serde-wasm-bindgen`, the two Rust crates requested through WP-235 for the WASM facade, which generate the web client's TypeScript types from Rust. Neither is a dependency of the core. |
| ESLint 10 and the React lint rules (answered 2026-10-03) | Approved. The web client lints with ESLint 10, because ESLint 9 is past its end of life and `eslint-plugin-react` supports nothing newer. The two rules of that plugin the baseline names, `react/no-danger` and `react/jsx-no-script-url`, are replaced by the project's own rules of the same effect. This changes how SEC-CLI-001, SEC-API-045, SEC-MED-057 and SEC-HIS-027 are verified, and the owner accepts that change. |

### Technical answers to wave 0's package questions, 2026-10-03

The owner delegated purely technical choices to the recommended defaults (D-02 and the "Tech choices" answer). These settle the questions wave 0's packages raised:

- **Dependencies.** The `[workspace.dependencies]` pins stand. Each crate joins `supply-chain/core-allowlist.toml`, with its whole tree, when its first user adds it; acceptance under D-02 is not a review. XML parsers, YAML and OpenTelemetry stay banned for R1. cargo-deny checks only the shipped and tested targets, and each native client's target is added before it is built. `libc` 0.2.190 is accepted as is, a dev-only crate that passes the seven-day rule on 2026-10-09. The `dependency-age-override` label relies on the code-owner review that `Cargo.lock` already needs.
- **Core values and parsing.**
  - The typed-value ranges WP-005 built stand.
  - Links with non-ASCII hosts stay plain text.
  - The egress client refuses NAT64, 6to4 and Teredo destinations.
  - The ceiling rule stands: limits whose value a requirement states cannot be raised, and others may be raised to four times their default.
  - SEC-MED-003 is proved by exact capacity assertions; there is no `unsafe` counting allocator, not even in test code.
  - There is one structure-aware fuzz harness per container family. WP-005's entry points get harnesses in wave 1.
  - Session and credential ID kinds are added when first needed.
- **Workers and records.**
  - Packaging limits come from measurement, with 512 MiB as the default until then.
  - Third-party decoders follow SEC-MED-024 as written.
  - Record 5 is accepted for the scan worker only.
  - Records 7 to 10 are accepted once a wave 1 follow-up fixes the two inconsistencies review found: record 9 diverges from the baseline's key table, and records 9 and 10 disagree on version bytes.
  - Record 1 gains a one-line pointer to record 3, as its Scope section already points to record 2.
  - SEC-TM-050 adopts SEC-PRV-001's data-class names.
  - Erasure selectors may use clock ranges within one stream.

### Technical answers to the client plan's questions, 2026-10-03

The owner delegated purely technical choices to the recommended defaults (D-02). These settle the questions the web player's plan raised, and [record 12](adr/0012-web-client-toolchain-and-contracts.md) holds the reasoning. The plan itself ([client-packages.md](plan/client-packages.md)) and record 12 as a whole are proposed, for the owner's review.

- **`unsafe` in the WASM facade (WP 34).** The core keeps `unsafe_code = "forbid"` with no exception. A workspace `forbid` cannot be lowered in source, so the facade crate's manifest repeats every workspace lint with only `unsafe_code` lowered as far as `wasm-bindgen`'s generated code needs, with a test that it repeats them all. `xtask lint-exceptions` permits that one manifest beside the core's, and an xtask check on the keyword keeps hand-written `unsafe` out of the facade. WP-235 must prove the `wasm32` build under that lint, exporting WP-005's link filter, before any other facade slice is written.
- **The facade is built in slices by the backend plan.** WP-235 (wave 2) creates `crates/gunmetal-wasm`; WP-236 (wave 2) exports the playback rules and merges into `wave-2` after WP-235; WP-237 (wave 3) converts for the core packages below; WP-088 (wave 3) keeps the rest. Client packages own no Rust.
- **Rules the web client needs get core packages.** The tint rule (WP-238), the inbound-link parser split from WP-089 (WP-239), the event builder and the event sink with its private-mode drop (WP-240) and the QR matrix (WP-242) in wave 2, and the audit-head extension check (WP-241) in wave 3. The requests the client plan made of existing backend packages (WP-056, WP-072, WP-088, WP-095, WP-124, WP-127, WP-132, WP-136) are accepted and written into their entries.
- **Types cross from Rust to TypeScript by generation.** Mirror types in the facade with exhaustive conversions, declarations generated with `tsify` and `serde-wasm-bindgen`, committed and drift-checked in the gate. Neither crate is a dependency of the core.
- **JavaScript supply-chain choices (D-65).** pnpm, as recommended, on Node 24. Every tool is pinned to its newest release at least seven days old. ESLint 10 is used, because ESLint 9 is past its end of life.
- **Branches.** There are no client wave branches. Client packages merge into the open server wave branch through that wave's integrator, and the owner still merges one pull request per wave into `main` (D-01). Wave 1 was closing, so the client's first packages go into `wave-2`.
- **Inbound links.** Only the core parses them (SEC-CLI-025). The client's router matches fixed, secret-free paths.
- **The style-policy spike (D-74).** D-74 names "the spike in WP-072". The spike is the client plan's CP-003; the decision it may lead to is still the owner's.

D-73 (player shortcuts), D-74's fallback and the default answer to "Is this your own device?" stay the owner's. The client plan proceeds on its recommended defaults for the first and the last, which the owner may change.

## Decide first

These block the build in this order. D-01 to D-04 block wave 0 of the plan
itself; D-05 to D-08 block the first server code (SEC-STD-006 forbids any
server code that stores a user before the identity record is accepted);
D-09 blocks the worker and the audio path; D-10 decides what waves 3 to 5
build for R1.

### D-01 How parallel work merges

- **Question.** Who owns the root `Cargo.toml`, `scripts/gate.sh`, `ci.yml`
  and the `supply-chain/` files after wave 0, does the proposed merge
  protocol go into CONTRIBUTING.md, and how does the gate keep up as the
  workspace grows?
- **Options.** (a) One named maintainer integrates each wave. (b) One
  dedicated agent per wave integrates, under a maintainer's review. (c) No
  integrator; every package merges itself and fights over registry files.
  For the gate: full mutation testing on every pull request, or
  diff-scoped, sharded mutation on pull requests with the full pass on every
  merge to an integration branch and nightly.
- **Recommendation.** (a) or (b), named before WP-001 starts; accept the
  protocol with any edits before WP-001 writes it into CONTRIBUTING.md; the
  diff-scoped gate on pull requests with the full gate on the integration
  branch as the definition of done; keep `gunmetal-testkit` under the full
  gate. The integrator also owns `supply-chain/` after wave 0 (applied
  change in theme T14).
- **Blocks.** WP-001, and through it every wave 0 merge.
- **Sources.** WP 6, 25, 26; work-packages.md "Working in parallel".

### D-02 Workspace, dependencies and ADR 6

- **Question.** Accept the crate layout (fourteen crates, the core kept as
  one crate), recorded as ADR 6 extending ADR 1, together with the core's
  first dependencies and the scoped exceptions to the lint bans?
- **Options.** (a) As drafted: `serde`, `postcard`, `sha2` (only in the
  core's crypto module), `unicode-normalization` and a pure-Rust inflate
  crate for the one decompression helper (WP-128) on the core's reviewed
  allow-list (SEC-SUP-025); JSON decoding for history imports kept out of
  the core and out of the server process, in the worker's import job,
  with `serde_json` linked into the worker only for that job (SEC-MED-018,
  SEC-MED-020; WP-057, WP-108); path-based `std::fs` banned everywhere except the dev-only
  testkit, with SQLite opened only through `gunmetal-fs` (SEC-MED-033);
  cryptographic crates used only from two named modules, the core's
  SHA-256 module and the secrets crate's crypto module (SEC-STD-018);
  `unsafe` allowed only in `gunmetal-wasm` if `wasm-bindgen`'s generated
  glue needs it (unverified); TOML configuration. (b) A hand-written codec
  for every protocol type instead of `serde` and `postcard`. (c) Every
  algorithm in the core's crypto module behind a server-only feature,
  which meets SEC-STD-018's words exactly but adds seven crates to the core
  allow-list.
- **Recommendation.** (a). Also confirm the plan's "not used, on purpose"
  crate list as a rule, not just a proposal.
- **Blocks.** WP-001 lints and allow-list check, WP-003 (ADR 6), WP-122,
  WP-126, WP-128, and every wave 1 crate that adds a dependency.
- **Sources.** WP 4, 5, 23, 33, 34, 38; applied changes on the `std::fs`
  ban, the SQL door, the crypto door and the inflate helper (theme T14).

### D-03 Parser and worker limits, and the baseline values that disagree

- **Question.** Accept the default limits table, and which value applies
  where two baseline rows set different values for one control?
- **Options.** For the table: accept as written, with ceilings at four
  times the defaults in the configuration file and no UI control (media
  OD-5), and the 2-core, 1 GiB reference profile until the scan benchmark
  gives real numbers (network OD-11); or set other values. For the
  disagreements, the stricter value, the looser value, or one value per
  context:
  - image limits: SEC-API-086 says 8,192 pixels per side and 40
    megapixels, SEC-MED-045 says 16,384 and 64;
  - lyrics: SEC-API-090 says 256 KiB and 10,000 lines, SEC-MED-049 and the
    limits table say 1 MiB and 20,000 lines;
  - a Range header with several ranges: SEC-API-031 and SEC-NET-050 answer
    416, SEC-MED-060 serves the whole file.
- **Recommendation.** Accept the table. Apply the stricter value in each
  case, as the plan already does (WP-021, WP-023, WP-079), and have the
  baseline's owner record one owning row per value in the parameters table
  (SEC-TM-072) so the docs lint holds them in step. Skip and record
  compressed ID3v2 frames in R1, so the core needs no inflater for tags.
- **Blocks.** WP-004's constants (wave 0), WP-021, WP-023, WP-079 and the
  limits register (WP-130).
- **Sources.** WP 13, 20, 36; media OD-5; network OD-11; security README 25
  (limit defaults); notes from the music, library, UI and plan agents.

### D-04 Public IDs

- **Question.** Are the IDs that leave the server random values kept in a
  durable mapping, or derived from content identity under a server key?
- **Options.** (a) 128 random bits from the CSPRNG, minted once and kept in
  the identity store's public-ID mapping, so they survive cache rebuilds
  and file moves. (b) HMAC over the content fingerprint (web OD-3), which
  SEC-HIS-012 rules out because content identity can fall back to the path
  stem.
- **Recommendation.** (a), as applied by default (SEC-HIS-012,
  SEC-API-023, SEC-PRV-021).
- **Blocks.** WP-006 (wave 0), WP-046, WP-077, WP-102, and where ADR 3
  puts the mapping.
- **Sources.** WP 11; web OD-3; rival OD-6; security README 25 ("random IDs
  with a rebuildable cache"); INT-008.

### D-05 ADR 3: durable user state, erasure and the shape of user events

- **Question.** Accept ADR 3, which makes the user log and the identity
  store durable, and settle the inputs it names?
- **Options and parts.**
  - ADR 3 as drafted: per-profile and household log streams in framed,
    checksummed monthly segments; documents stored as operations with
    periodic snapshots; the identity store as its own SQLite file
    (SEC-IAM-004, SEC-TM-051). The alternative, leaving ADR 1 decision 5
    alone, gives most R1 writes (queues, playlists, loves, accounts,
    grants) nowhere durable to live.
  - Erasure. ADR 1 decision 5 calls the log append-only, but the baseline
    requires real erasure of one play, a range or all history within 24
    hours, tombstones naming only event IDs, and re-application after a
    restore (SEC-PRV-049, SEC-PRV-050, SEC-PRV-052). ADR 3 should record
    erasure as the one sanctioned rewrite of the log. A masking "removal
    event" does not meet the baseline.
  - Ratings: loves everywhere; five stars with half steps, switched on when
    ratings are imported; tracks, albums and artists rated directly;
    strictly per person.
  - History events carry only the fields SEC-PRV-002 allows, so a play
    cannot record its source context; "Continue listening" uses the queue's
    context IDs instead (MUS-050). Storing the source would need
    SEC-PRV-002 amended.
- **Recommendation.** Accept ADR 3 with erasure as the sanctioned rewrite,
  the rating model above, and history fields as SEC-PRV-002 lists them.
- **Blocks.** WP-002's acceptance, then WP-034, WP-035, WP-046, WP-068 and,
  through them, most of R1; WP-133 (erasure).
- **Sources.** Features README 1, 12; WP 1, 17; api-needs flags 1 and 8;
  music, discovery and accounts notes on who owns "remove plays"; applied
  theme T05.

### D-06 Identity and sessions (record 7)

- **Question.** Confirm the identity model that record 7 (WP-125) writes
  down, before any server code stores a user.
- **Options.**
  - Passwords: (a) no account passwords and no TOTP anywhere, with
    passkeys, browser pairing and OIDC (the baseline, SEC-IAM-025);
    ACC-052, ACC-053 and API-AUTH-05 become No. (b) A password with
    optional two-factor wherever passkeys cannot work (the feature map's
    old recommendation), which on a plain-HTTP home network sends the
    password unencrypted and breaks SEC-IAM-025 and SEC-NET-001.
  - Browsers without a passkey: approval from a signed-in device, giving a
    limited session that can play but never administer (SEC-IAM-108, R1).
  - OIDC: in R1 (baseline, identity OD-3) or in a point release (the R1
    proposal below puts it in R1.2).
  - Recovery: the ladder in the baseline (another passkey; recovery codes
    offered to the owner and administrators; the identity provider; an
    administrator's link redeemed in person or on an approved device,
    starting a 72-hour hold; owner recovery only on the host). Recovery
    codes for the owner strongly prompted but skippable, because host
    recovery exists (identity OD-12).
  - Sessions: 7 days idle and 30 days total for browsers; a separate admin
    session of 15 minutes idle and 1 hour total; a passkey check in the
    last 5 minutes for the most dangerous actions (SEC-IAM-041); accept the
    deviation for long playback sessions on devices (threat-model OD-8).
  - Web session and media: one HttpOnly cookie for the API; media and
    artwork only from session-bound capability URLs, with artwork URLs
    aligned to a time bucket so browsers can cache them (web OD-2, OD-4;
    WP 31). The cookie-authorised artwork path the plan first proposed is
    withdrawn.
- **Recommendation.** (a) for passwords; browser pairing in R1; the
  recovery ladder and session table as written; OIDC in R1.2 if the owner
  takes the R1 proposal, otherwise R1. Trade-off: a person with no
  passkey-capable device and no phone needs a hardware key or help from
  someone in the household.
- **Blocks.** WP-125 (record 7), then WP-046 and every package that stores
  a user, WP-063, WP-064, WP-080, WP-081, WP-096, WP-106, WP-120.
- **Sources.** Security README 1, 6; features README 7 (its password part
  is superseded); identity OD-2, OD-3, OD-9, OD-12; threat-model OD-1,
  OD-3, OD-8; standards OD-1; web OD-2, OD-4; WP 31; api-needs flags 5 to
  7; applied theme T01.

### D-07 HTTPS and naming (record 8)

- **Question.** Does the project run a per-server HTTPS name service as the
  install-time default in R1?
- **Options.** (a) Yes, once its zone is on the Public Suffix List, with CT
  monitoring, the pre-claim exception limited to the name service and the
  CA (SEC-OPS-007), and own domain (ACME DNS-01), tailnet and localhost as
  tested alternatives (SEC-NET-010 to SEC-NET-013, SEC-NET-069 to
  SEC-NET-072). (b) No name service: R1 works only for people who can set
  up a domain, a tailnet or a reverse proxy. (c) Self-signed certificates
  or plain HTTP with passwords, which SEC-NET-001 and SEC-NET-005 rule out.
- **Recommendation.** (a), falling back to (b) on its own if the PSL entry
  is missing at launch. It needs an ADR amending ADR 1 decisions 7 (no
  central account) and 9 (Cloudflare hosts only docs and the landing
  page), a zone name that is not the docs domain, Let's Encrypt with a
  second ACME CA as fallback, a legal home for project services (D-41) and
  a second keyholder (D-62). Restoring a server under a new address breaks
  members' passkeys, so restore warns and re-enrolment follows the
  recovery-link rules (SEC-IAM-091, SEC-IAM-106). Remote use in R1 is the
  owner's reverse proxy or tailnet (D-21).
- **Answer (2026-10-02).** Option (b) for R1, with the name service later;
  see [Owner answers](#owner-answers-2026-10-02). The documents now put
  the name service, its naming client and its CT monitoring in R2.
- **Blocks.** WP-125 (record 8), WP-129 (R2), WP-073, WP-080's claim
  flow, WP-101, WP-135, and the first-run flow (F01).
- **Sources.** Security README 2, 3, 21; features README 7; network OD-1;
  identity OD-1; client OD-1; web OD-1; threat-model OD-2; rival OD-1;
  standards OD-2; operations OD-1; WP 21; api-needs flags 5 and 6;
  ADM-023, ACC-099; applied theme T02.

### D-08 Cryptography and backup archives (records 9 and 10)

- **Question.** Which implementations go on the reviewed crypto allow-list
  (SEC-STD-019), and what archive format sits inside the encrypted backup?
- **Options.**
  - rustls provider: aws-lc-rs with the hybrid X25519MLKEM768 group
    (brings C and assembly into the build) or ring, or whatever rustls
    defaults to.
  - WebAuthn: hand-written verification on RustCrypto for ES256 and EdDSA,
    with or without RS256.
  - OIDC: verify RS256 (needs an `rsa` crate whose past timing advisory may
    or may not affect verification, unverified) or support only providers
    set to ES256 or EdDSA (excludes providers left on their defaults,
    unverified per provider).
  - Update feed: `tough` or a minimal TUF client (the feed is TUF 1.0 from
    a compiled-in root either way, SEC-SUP-049, SEC-OPS-019).
  - Backup archive inside age: hand-written or the `tar` crate; record 10
    must allow its extraction with entry paths ignored, links refused and
    counts and sizes capped (SEC-HIS-019).
- **Recommendation.** aws-lc-rs if it builds cleanly for the R1 targets
  (Linux x86-64 and AArch64 under D-09), otherwise ring with the hybrid
  group in R2; ES256 and EdDSA for WebAuthn, adding RS256 only after
  checking which household authenticators need it; RS256 for OIDC only if
  the `rsa` crate passes review; `tough` if its dependency tree passes
  cargo-vet; a small hand-written archive format.
- **Blocks.** WP-125 (records 9 and 10), WP-047 (wave 1), WP-048, WP-073,
  WP-074, WP-081, WP-096, WP-101, WP-109.
- **Sources.** WP 8, 9, 10, 12; network OD-8; supply-chain OD-3; security
  README 13 (TUF from R1).

### D-09 Media processing outside the server process

- **Question.** Confirm that untrusted media is never processed in the
  server process, and on which server platforms R1 runs.
- **Options and parts.**
  - ADR 4, the audio packager for browser gapless (MUS-230) and CUE slices
    (MUS-041): in a worker process streaming over a pipe, under the step
    budget, a memory cap and a watchdog (SEC-MED-018, SEC-MED-081), or
    in-process as the feature map and WP-003's draft still say. If ADR 4 is
    rejected, browser gapless rests on each browser's Media Source
    Extensions (unverified).
  - ADR 5, a third-party pure-Rust decoder (the candidate is Symphonia,
    MPL-2.0) for loudness measurement (MUS-086): admitted only in the scan
    worker after the review SEC-MED-026 requires, or rejected, in which
    case R1 uses loudness tags and the fallback gain (MUS-089). Opus needs
    a memory-safe decoder in the scan worker or a native one in the full
    jail.
  - A record that supersedes ADR 1 decision 3 and the README diagram's
    "remux in-process": the remuxer runs in a worker (security README 9).
  - Isolation: memory-safe parsing may run at a reduced floor with a
    visible notice; native decoders need the full jail; nothing ever runs
    unconfined (SEC-MED-024, SEC-TM-045). Media OD-3's per-feature admin
    override for reduced isolation is not adopted.
  - Platforms: Linux-only server builds in R1 (x86-64 and AArch64, plus the
    container image); macOS and Windows only once their sandbox profiles
    exist (SEC-MED-082, R2); 32-bit ARM built with the reduced label but not
    called supported until its seccomp answer is recorded; FreeBSD Later.
    The alternative (media OD-6, WP 18's first draft) is macOS and Windows
    builds in R1 at the reduced tier.
  - No transcoding of any kind in R1 (security README 11).
- **Recommendation.** Workers everywhere, as applied; accept ADR 5 for the
  scan worker only, after review; Linux-only R1. Fix the baseline mismatch
  at the same time: SEC-MED-081, SEC-MED-032 and SEC-MED-074 are R2 while
  the packager ships in R1, so either they move to R1 or a new R1 row
  covers the packager. WP-003 now drafts ADR 4 as a worker and ADR 5 as
  "audio decoders in the scan worker"; WP-061 carries a `Package` job,
  WP-105 owns it and revalidates every segment (SEC-MED-023), and WP-105
  verifies SEC-MED-081 and SEC-MED-032 as if they were R1 until the
  baseline is fixed. Import files and playlist files are parsed by worker
  jobs too (WP-108, WP-112).
- **Blocks.** WP-003 (wave 0), WP-045 (wave 1), WP-056, WP-105, WP-029,
  WP-114, WP-121.
- **Sources.** Security README 9, 10, 11; features README 8, 9; WP 2, 3,
  18, 24; media OD-1, OD-2, OD-3, OD-6, OD-7, OD-8; threat-model OD-9,
  OD-10; api-needs flags 2 to 4; music and video notes; applied theme T03.

### D-10 R1 scope and the release table

- **Question.** What exactly is R1, and how are point releases recorded
  against the baseline's release values?
- **Options and parts.**
  - The R1 cut: keep the current one (455 feature rows after the
    alignment) or adopt the smaller R1 proposed in the next section, with
    the rest in R1.1 to R1.3.
  - MusicBrainz and Cover Art Archive (LIB-111, LIB-112, WP-137): built
    into the server, off until the owner turns them on in the required
    setup step, at R1 (security README 22), at R1.1 (this proposal), or as
    R2 plugins. Building them in bends ADR 2's "lookups belong in plugins"
    and needs a record that says so.
  - Music share links (ACC-086 to ACC-089, MUS-151): R1 (security README
    7) or R1.2 (this proposal). Either way SEC-API-097 ships with them.
  - OIDC (ACC-057): R1 or R1.2 (see D-06).
  - Point releases: SEC-TM-074 allows only R1, R2, R3, Later and Withdrawn,
    so R1.x needs either new release values in the release-scope table and
    the docs lint, or a rule that a requirement is due in the first point
    release that ships its surface.
  - Five baseline rows are R1 although their surface is R2: SEC-NET-054 and
    SEC-OPS-040 (iroh) and SEC-PRV-034 to SEC-PRV-036 (scrobbling). Move
    them to R2, or keep proving them by absence in R1 (applied by default).
  - The desktop shell is Later in the release-scope table but R2 in the
    feature map README; rival-database importers are Later in the table
    but were R2 in the maps. The alignment applied Later to both.
- **Recommendation.** Adopt the proposed R1 with point releases; providers
  built in at R1.1 behind the egress client; share links and OIDC at
  R1.2; add R1.x values to the release-scope table and the lint; move the
  five rows to R2; desktop shell and rival importers Later.
- **Answer (2026-10-02).** The proposed R1 is adopted with point releases
  R1.1 to R1.3, and providers are built in at R1.1; see
  [Owner answers](#owner-answers-2026-10-02) and [R1 scope](#r1-scope).
  The release values R1.1 to R1.3 are now in use in the feature map and
  the baseline. Two parts of the recommendation were not in the answer
  and are unchanged: the five rows stay R1, proved by absence, until the
  owner moves them; the desktop shell and rival importers stay Later, as
  the alignment applied.
- **Blocks.** Which wave 3 to 5 packages are R1 (among them WP-096,
  WP-112, WP-134, WP-137), WP-127's docs lint, WP-131's absence suites, and
  the traceability check (SEC-STD-004).
- **Sources.** Security README 7, 22, 25 (OIDC in R1, desktop shell);
  features README "The R1 cut" and review log; WP 37; music, library,
  discovery, accounts, admin, integrations and video notes; applied themes
  T06, T08, T11.

## R1 scope

**Adopted by the owner on 2026-10-02 (D-10).** There is one change from
the proposal: under D-07 the project name service (ADM-023) leaves R1 for
R2. This section was proposed and is now the definition of R1 and its
point releases. The feature map's Release cells match it row for row,
and the documents were realigned to it on 2026-10-03.

### What was adopted

When this was proposed, the feature map put 455 rows in R1 and the
baseline 607 requirements. A household needs much less on day one: a
server that installs safely, claims with a passkey over real HTTPS, scans
a music folder with memory-safe parsers in a separate worker, plays the
original files gaplessly with loudness levelling in a browser, keeps a
queue and simple playlists, shows lyrics from the files, lets each person
sign in, invite others, see and erase their own history, and survives a
restore. Everything that makes that safe stays in R1: the security rows
are most of what remains, because the baseline makes them conditions of
shipping anything at all.

The adopted R1 keeps 266 owning rows (and 30 reference rows that ship
with them), against 455 rows before. The proposal had 267, and D-07 moved
ADM-023 to R2. The feature map counts the same 296 rows as 264 owning
rows and 32 references, because two rows this list names as features,
ADM-121 and CLI-157, are references there (to ACC-123 and ACC-117).
On 2026-10-03 D-83 added one owning row, MUS-236 (a minimal, read-only
track details view), so R1 now holds 267 owning rows here and 297 rows
in the feature map (265 owning, 32 references). The rest moved to three
point releases in the order users will miss them:

- **R1.1, bring your music in** (68 rows): playlist files and history
  imports, built-in MusicBrainz and cover-art lookups, ratings, richer
  credits and browsing, offline loading and installing the web app, avatars,
  continue-on-this-device.
- **R1.2, the household and the admin** (38 rows): OIDC, music share
  links, a second administrator, the admin's live view with each person's
  opt-in for titles, stopping a stream, an arrangeable home, diagnostics,
  restore from the UI, translations.
- **R1.3, discovery and analysis** (29 rows): the rule language and
  smart playlists, library radio and the neighbour table, measured loudness
  (if ADR 5 is accepted), folder view for admins, manual curation, 32-bit
  ARM.

Four rows leave the R1 line for R2 because their only users arrive there:
CLI-032 (old clients keep working; the web client always reloads the
server's own build, so it matters only for native apps), INT-006 and
INT-138 (change feed and playlist writes for tools, which need API keys),
and LIB-056 (a reference row whose owner, MUS-014, is R2). A fifth,
ADM-023 (the per-server HTTPS name from the project name service), went
to R2 under D-07, with the naming client and certificate-transparency
monitoring, beside built-in remote access.

The native Android music app (D-14) is not part of R1.x; it would bring R2
requirements for device keys and native storage with it.

### Adopted R1, by feature

**Install, run and upgrade** (25)

- ADM-001: Single self-contained binary
- ADM-002: Official container image
- ADM-003: Container tags that pin a version
- ADM-005: Install as an OS service with one command
- ADM-006: Refuse to run as root
- ADM-007: Configuration that is checked, not guessed
- ADM-079: Refuse a data directory on a network filesystem
- ADM-080: One writer, so no "database is locked"
- ADM-083: Free-space guard
- ADM-089: Media mounted read-only
- ADM-090: Separate locations for settings, log, cache and scratch space
- ADM-095: Heavy jobs that are throttled and resumable
- ADM-119: Structured logs with rotation
- ADM-121: Secrets that cannot reach logs
- ADM-122: Sign-in failure lines for fail2ban
- ADM-123: `gunmetal doctor`
- ADM-128: Health endpoints for containers
- ADM-032: Startup and migration status page
- ADM-056: Automatic snapshot before every upgrade
- ADM-057: Check migrations before serving
- ADM-058: Upgrade straight from any older version
- ADM-059: Roll back by starting the previous version
- ADM-060: Rollback safety stated for every release
- ADM-077: Rebuild the cache instead of repairing it
- ADM-078: Checksummed user log that recovers from a torn write

**Claim, setup and HTTPS** (15; ADM-023, the name service, moved to R2
under D-07)

- ACC-001: Claim a new server with a one-time setup code
- ACC-002: Owner account with no vendor account
- ADM-020: Setup closes for good once an admin exists
- ADM-021: A secure context for passkeys at setup
- ADM-022: Built-in HTTPS for a domain you own
- ACC-097: Works behind your own reverse proxy or VPN
- ACC-098: HTTPS served by the server
- ACC-099: Automatic certificates for your own domain
- CLI-150: Secure context in R1
- ADM-025: Add music folders with live checks
- ADM-028: Privacy choices at setup
- ADM-053: Update check from a signed feed, asked at setup
- ADM-054: Security advisory banner
- ADM-143: Recovery kit at setup
- ACC-113: Nothing leaves the house by default

**Backups and restore** (7)

- ADM-065: Daily backups on by default
- ADM-066: Backup contents shown plainly
- ADM-068: Encrypted backups
- ADM-069: Download a backup, upload it elsewhere
- ADM-071: Restore from the command line
- ADM-072: Backups verified after they are written
- ADM-029: Restore from backup on the welcome screen

**Security operations** (13)

- ADM-108: Health of each library root
- ADM-110: Activity and audit log
- ADM-111: Retention and anonymisation of activity
- ADM-116: Admin alerts with free destinations
- ADM-129: Network activity page
- ADM-142: Security summary on the admin home
- ADM-144: Rotate every server key in one action
- ADM-145: Audit log verification and an anchor off the server
- ADM-146: Outbound proxy and offline mode
- ADM-147: Configuration changes made outside the server are reported
- ADM-148: Compromise runbook
- ACC-126: An advisory for every fix
- ACC-128: Nobody can switch your server off remotely

**Accounts, sign-in and recovery** (33)

- ACC-003: Sign-in and playback with no internet
- ACC-004: Owner recovery from the host
- ACC-005: Hand ownership to another person
- ADM-052: Hand the server to a new owner
- ACC-006: Local user accounts
- ACC-007: No user list before sign-in
- ACC-008: Disable an account without deleting it
- ACC-009: Deletion with a grace period
- ACC-136: Delete your own account
- ACC-010: Export your own data
- ACC-012: Preferences that follow you
- ACC-013: Identity data that survives a cache rebuild
- ACC-017: Profiles separate from sign-in
- ACC-030: Every path obeys the restrictions
- ACC-037: Library access per person
- ACC-050: Passkeys
- ACC-055: Manage your own sign-in methods
- ACC-056: Confirm sensitive changes
- ACC-062: Approve a new browser from a signed-in one
- ACC-063: Protection against guessing
- ACC-064: Help a locked-out user
- ACC-137: Recovery codes
- ACC-138: Recovery hold
- ACC-065: Sign out everywhere when a credential changes
- ACC-068: Your devices, in one list
- ACC-069: Revoke one device
- ACC-070: Sign out of all sessions
- ACC-071: New-device alerts
- ACC-139: Your own security log
- ACC-075: Stream limits that count playing, not browsing
- ACC-076: Device limits and allow-lists
- ACC-079: Session lifetimes
- ACC-080: Invite by link or QR code

**Privacy and the rules every endpoint follows** (10)

- ACC-114: No social feed
- ACC-115: What your admin can see
- ACC-117: Private listening
- ACC-118: Remove plays from history
- ACC-120: Every endpoint needs sign-in
- ACC-121: Authorization on every object
- ACC-122: Short-lived, session-bound stream URLs
- ACC-123: Secrets never in URLs or logs
- ACC-124: Browser sessions that page scripts cannot steal
- ACC-125: Uploads cannot reach the filesystem by name

**The web client** (19)

- CLI-001: Web client served by your own server
- CLI-002: Published browser support list
- CLI-022: Library synced to the device
- CLI-031: A layout contract
- CLI-060: Wide three-pane layout
- CLI-070: Lock-screen controls from the web app
- CLI-093: Offline plays and progress merge cleanly
- CLI-135: Screen readers reach every control
- CLI-136: Accessibility as a release gate
- CLI-138: Full keyboard use with visible focus
- CLI-139: Text follows the system size
- CLI-140: Reduced motion
- CLI-141: Themes, including high contrast
- CLI-142: Motor accessibility
- CLI-149: Phone-width web layout
- CLI-155: Personal or shared browser
- CLI-156: Signing out leaves nothing behind
- CLI-157: Private session on every device
- CLI-159: Outside links say where they go

**Libraries and scanning** (38)

- LIB-001: Music libraries
- LIB-003: Several folders per library
- LIB-004: Several libraries
- LIB-005: Add a library at first run
- LIB-007: Read-only media
- LIB-011: Room for other media kinds
- LIB-012: Manual scan
- LIB-013: Scheduled safety-net scan
- LIB-014: Watch local disks
- LIB-015: Change detection on shares and cloud drives
- LIB-016: Rescans that change nothing
- LIB-017: Rescan only what changed
- LIB-018: Library change feed
- LIB-019: Header-only reads
- LIB-020: No helper process per file
- LIB-021: Usable during the first scan
- LIB-022: Scan and job activity
- LIB-028: Stable identity for every file
- LIB-029: Moves and renames keep everything
- LIB-030: Better copies replace, not duplicate
- LIB-032: An offline drive never empties the library
- LIB-033: Trash with a grace period
- LIB-045: Albums built from tags
- LIB-051: Same-titled albums stay apart
- LIB-059: Every raw tag kept
- LIB-097: Identify without the internet
- LIB-108: Nothing leaves by default
- LIB-134: Embedded cover art
- LIB-135: Local artwork files
- LIB-136: Predictable artwork order
- LIB-142: Images sized for each device
- LIB-143: Safe image handling
- LIB-146: Quality badges as data
- LIB-193: Damaged and unreadable files
- LIB-204: Skipped links and approved link targets
- LIB-205: Quarantined files
- LIB-206: Scanner isolation status
- LIB-207: Unresponsive storage pauses one folder

**The music model** (20)

- MUS-001: Every credited artist linked
- MUS-002: Display credit kept as tagged
- MUS-003: Album artist separate from track artist
- MUS-004: "Appears on"
- MUS-006: Same-name artists kept apart
- MUS-011: Compilations and Various Artists
- MUS-012: Multi-disc albums with disc titles
- MUS-017: Multi-valued genres per track
- MUS-020: Sort names and natural sort
- MUS-021: Technical details
- MUS-027: Several music libraries with per-user access
- MUS-032: Core formats
- MUS-034: Multi-value tags in every tag format
- MUS-035: Separator splitting with exceptions
- MUS-036: MusicBrainz IDs as identity
- MUS-037: Tags win over online data
- MUS-039: Album art from file and folder
- MUS-040: Artwork sized for sync
- MUS-044: Library health report
- LAT-001: Item kinds in the data model

**Browse, home and search** (25)

- MUS-050: Continue listening by album or playlist
- MUS-051: Artist page
- MUS-052: All songs by an artist
- MUS-054: Album page
- MUS-059: Recently added that ignores upgrades
- MUS-060: Genre, mood and label browse
- MUS-061: Music search fields
- MUS-208: Instant browsing from the synced library
- DIS-001: Home made only of your media
- DIS-002: Instant home
- DIS-004: Good default home and empty states
- DIS-020: Continue listening
- DIS-021: Recently played
- DIS-035: Recently added
- DIS-036: Arrivals grouped by album
- DIS-038: Upgrades are not "new"
- DIS-083: One search box
- DIS-084: Search on the device
- DIS-085: Forgiving matching
- DIS-100: Endless, smooth lists
- DIS-104: Sorts that matter
- DIS-109: Browse pages
- DIS-111: The same menu everywhere
- DIS-112: Back keeps your place
- DIS-140: Discovery per person

**The player** (22)

- MUS-066: Play the original file
- MUS-067: Gapless in the web client
- MUS-069: Encoder delay and padding honoured
- MUS-070: Next track fetched early
- MUS-071: Instant, exact seeking
- MUS-073: Browser media controls
- MUS-077: Repeat and stop-after
- MUS-079: Damaged files skipped safely
- MUS-084: ReplayGain and R128 tags, track and album
- MUS-085: Opus gain done right
- MUS-087: Auto, track and album modes
- MUS-088: Target level with no clipping
- MUS-089: Fallback for unmeasured tracks
- MUS-099: A quality badge that tells the truth
- MUS-108: Persistent now-playing bar
- MUS-109: Love from the bar
- MUS-110: Full-screen player
- MUS-113: Layout contract
- MUS-227: Accessible player
- MUS-229: Honest unplayable state
- MUS-230: Audio packaging for the web player
- MUS-236: Track details (minimal and read-only; added 2026-10-03 under D-83)

**Queue, playlists and lyrics** (13)

- MUS-116: Three-lane queue
- MUS-117: Play next keeps your order
- MUS-118: Add to queue and play last
- MUS-119: Edit the whole queue
- MUS-122: Persistent queue on every device
- MUS-123: "Playing from" on every item
- MUS-126: Shuffle modes: random and spread out
- LAT-009: Listening contexts in the queue protocol
- MUS-132: Manual playlists
- MUS-133: Add-to-playlist sheet
- MUS-149: Loved tracks as a playlist
- MUS-154: Embedded lyrics
- MUS-155: Synced .lrc sidecars

**Your listening, and erasing it** (20)

- MUS-180: Loves
- MUS-182: Play counts, last played and skips
- MUS-183: Listening history by date
- MUS-184: Remove plays from history
- MUS-188: Export listening history
- MUS-233: Clear history for a period, or all of it
- MUS-234: Choose how long history is kept
- DIS-045: Loves
- DIS-046: Loved songs as a list
- DIS-050: History by date
- DIS-051: Your play counts
- DIS-052: Remove a play
- DIS-058: Export everything you told the server
- DIS-186: Clear a period or all history
- DIS-187: Choose how long history is kept
- DIS-188: History held during account recovery
- DIS-189: "Only you can see this"
- INT-151: Documented history export
- LAT-006: Typed positions in the user log
- LAT-007: All user-made data in the user log

**The native API** (7)

- INT-001: API reference generated from code
- INT-005: Capability discovery
- INT-007: Consistent list endpoints
- INT-008: IDs that survive rebuilds and file replacement
- INT-009: MusicBrainz IDs on music items
- INT-013: Health check for uptime monitors
- INT-023: Credentials in headers only

**Reference rows that ship with their owners in R1** (30): ACC-078 (see ADM-110), ACC-127 (see ADM-054), ADM-018 (see ACC-001), ADM-019 (see ACC-002), ADM-031 (see LIB-021), ADM-034 (see ACC-004), ADM-085 (see LIB-032), ADM-086 (see LIB-033), DIS-053 (see ACC-117), LIB-036 (see MUS-034), LIB-037 (see MUS-001), LIB-038 (see MUS-035), LIB-039 (see MUS-003), LIB-040 (see MUS-006), LIB-042 (see MUS-004), LIB-044 (see MUS-020), LIB-046 (see MUS-036), LIB-049 (see MUS-011), LIB-050 (see MUS-012), LIB-053 (see MUS-017), LIB-063 (see MUS-021), LIB-064 (see MUS-069), LIB-065 (see MUS-084), LIB-067 (see MUS-154), LIB-068 (see MUS-155), MUS-038 (see LIB-028), MUS-042 (see LIB-016), MUS-043 (see LIB-021), MUS-062 (see DIS-111), MUS-185 (see ACC-117).

### Point releases after R1

**R1.1, bring your music in** (68)

- ACC-011: Name and picture for each profile
- CLI-030: Settings that follow you
- CLI-003: Installable web app
- CLI-040: Letter jump in long lists
- DIS-101: Alphabet jump
- CLI-024: Sync and storage status
- CLI-025: Losing the server never blocks the app
- CLI-026: Offline is not a separate mode
- LIB-025: Upgrades without full rescans
- LIB-187: One artist page across libraries
- LIB-194: Tag problems
- MUS-010: Release types
- MUS-056: Library views with remembered sort
- DIS-019: Speed you can check
- DIS-088: Scope a search
- DIS-107: Grid, list and compact
- MUS-090: Show the gain applied
- MUS-114: Track info sheet
- MUS-120: Reorder while shuffled
- MUS-125: Save queue as playlist
- MUS-134: Duplicate warning
- MUS-158: Lyrics stay open
- MUS-140: M3U and M3U8 import and export
- LIB-192: Playlist files in music folders
- ADM-043: Bulk playlist import with a match report
- ADM-044: Matching with reasons and an unmatched queue
- ADM-042: Import Last.fm and ListenBrainz export files
- ADM-030: "Coming from another server?" step
- LIB-111: MusicBrainz lookups
- LIB-112: Cover Art Archive covers
- MUS-024: Artist images from local files
- MUS-005: Roles: composer, conductor, lyricist, producer, remixer, performer
- MUS-008: Release groups and editions
- MUS-013: Original date versus release date
- MUS-019: Moods, styles, labels and grouping
- MUS-047: Explicit flag
- MUS-053: Sort and filter a discography
- MUS-055: Credits panel
- MUS-135: Sort, filter and search inside a playlist
- MUS-137: Automatic playlist covers
- MUS-139: Pin and love playlists
- MUS-156: Word-by-word lyrics
- MUS-181: Star ratings
- DIS-047: Personal ratings
- MUS-127: Shuffle by album
- MUS-128: Reshuffle the rest
- MUS-072: Fades on pause, skip and resume
- MUS-076: Sleep timer
- DIS-086: Search tags, genres and moods
- DIS-087: Search people by role
- DIS-089: Recent searches
- DIS-091: Find inside a list
- DIS-102: Filters on what the scanner knows
- DIS-103: Filters remembered
- DIS-105: Save a filter
- DIS-110: Multi-select
- CLI-062: Multi-select, drag and right-click
- LIB-006: Exclusion rules
- LIB-034: Missing files list
- LIB-098: Every decision explains itself
- ADM-051: Move to new hardware, OS or container
- LIB-031: Move the server, keep the library
- CLI-099: Fetch ahead on patchy signal
- CLI-103: Continue on this device
- CLI-151: Mono audio and channel balance
- DIS-022: Dismiss from Continue rows
- DIS-023: Undo, and a Hidden page
- DIS-071: Your top tracks by an artist
- References that follow their owners: DIS-163 (see CLI-040), INT-107 (see ADM-042), LIB-043 (see MUS-005), LIB-047 (see MUS-008), LIB-048 (see MUS-010), LIB-052 (see MUS-013), LIB-054 (see MUS-019), MUS-057 (see DIS-107), MUS-058 (see DIS-101), MUS-063 (see DIS-110), MUS-189 (see ADM-042)

**R1.2, the household and the admin** (38)

- ADM-027: Language, region and time zone
- ACC-040: Several administrators
- ACC-057: Single sign-on with your own identity provider
- ACC-086: Share links for music
- ACC-087: Password on a share link
- ACC-088: Download switch on a share link
- ACC-089: Your shares, and everyone's for admins
- MUS-151: Share links for music
- ADM-099: Now playing
- MUS-235: Show what I am playing
- ACC-116: Choose whether admins see what you play
- ADM-102: Stop a session with a message
- ADM-100: Playback decision and its reason, per session
- INT-134: Playback decision in the API
- ADM-093: One task list with run, cancel and history
- ADM-109: Health summary on the admin home
- ADM-112: Restart and shut down from the UI
- ADM-113: Emergency page served by the server
- ADM-124: Diagnostic bundle with preview and masking
- CLI-033: Diagnostics you can read first
- ADM-125: File inspector
- ADM-070: Restore from the UI with a restore point and preview
- ADM-074: Full export in documented formats
- ADM-140: Server name and welcome message
- ACC-134: Serve under a path prefix behind a reverse proxy
- MUS-049: Music home with sections you arrange
- DIS-003: Build your own home
- DIS-007: Layout follows you
- DIS-009: Rows as long as you like
- DIS-012: Keep a library off home
- DIS-013: Pinned shortcuts
- DIS-015: A home that does not move
- CLI-034: Deep links
- INT-147: Stable deep links
- CLI-146: Translations with a completeness bar
- ADM-130: Local crash records
- ADM-010: Published footprint numbers
- ADM-011: Hardware guide sized for direct play
- References that follow their owners: ACC-072 (see ADM-099), ACC-073 (see ADM-102), DIS-173 (see CLI-034), LIB-195 (see ADM-125), MUS-190 (see ADM-099)

**R1.3, discovery and analysis** (29)

- MUS-143: Smart playlists
- MUS-144: Visual rule editor
- MUS-145: Rich rule fields
- MUS-146: Limits, sorts and percentages
- DIS-119: One rule language
- DIS-120: Rule editor with live preview
- DIS-121: Smart playlists
- DIS-122: Limits, order and refresh
- MUS-165: Library radio from any seed
- MUS-129: Suggestions lane, visible and off by default
- DIS-060: More like this
- DIS-061: "Because you played" rows
- DIS-062: Every suggestion says why
- DIS-067: Radio from anything
- DIS-070: Suggestions after the queue ends
- MUS-086: Loudness measured for untagged files
- LIB-024: Background analysis that resumes
- ADM-141: Derived-data store kept across rebuilds
- LIB-008: Folder view
- DIS-106: Folder view
- MUS-007: Merge, split and alias artists
- LIB-058: Merge and split albums by hand
- LIB-099: Review queue
- LIB-179: Fixes survive everything
- ADM-088: Scans that report bytes read
- ADM-004: Builds for small ARM boards, including 32-bit
- LAT-002: People with typed roles
- LAT-008: Typed links between items
- LAT-010: Spoken word kept out of music
- References that follow their owners: LIB-041 (see MUS-007), LIB-066 (see MUS-086), MUS-028 (see LIB-008)

**Out of the R1 line, to R2** (5)

- LIB-056: Works and movements
- CLI-032: Old clients keep working
- INT-006: Change feed (delta sync)
- INT-138: Playlist write API
- ADM-023: Per-server HTTPS name from the project name service (D-07)

<a id="security-requirements-for-the-proposed-r1"></a>

### Security requirements for the adopted R1

Every R1 requirement in the baseline stays mandatory for the adopted R1,
except the ones whose only surface moves to a point release or, under
D-07, to R2. Those become mandatory in the release that ships the
surface, and that release cannot ship without them (SEC-STD-004). None is
weakened or dropped. The baseline's Release cells now match this table
(security README, "The R1 security cut" and "Due after R1").

| Moves to | Surface | Requirements |
|---|---|---|
| R1.1 | Playlist files imported or found in libraries (MUS-140, LIB-192) | SEC-MED-050, SEC-HIS-018 |
| R1.1 | Metadata providers (LIB-111, LIB-112) | SEC-PRV-014, SEC-PRV-015, SEC-PRV-017 |
| R1.1 | Avatar and other image uploads (ACC-011) | SEC-MED-061, SEC-PRV-006 |
| R1.2 | OIDC sign-in (ACC-057) | SEC-TM-022, SEC-IAM-026, SEC-IAM-027, SEC-IAM-028, SEC-IAM-029, SEC-IAM-030, SEC-IAM-031, SEC-IAM-032, SEC-IAM-033, SEC-IAM-034, SEC-IAM-035, SEC-IAM-036, SEC-STD-025, SEC-CLI-026 |
| R1.2 | Music share links (ACC-086 to ACC-089, MUS-151) | SEC-API-097, SEC-STD-008 (the share-link password is the only secret a person chooses; there are no account passwords and backups need no passphrase, ADM-068) |
| R1.2 | Diagnostic bundles (ADM-124, CLI-033) | SEC-PRV-046, SEC-OPS-030 |
| R2 (D-07) | The project name service, its naming client and its certificate-transparency monitoring (ADM-023) | SEC-NET-010, SEC-NET-011, SEC-NET-012, SEC-NET-069, SEC-NET-070, SEC-NET-071 |

That is 25 of the 607 for D-10 and six more for D-07, 31 in all. The
other 576 remain mandatory for R1, including every requirement on: the
claim and setup path; passkeys, browser pairing, recovery codes, recovery
links, the recovery hold and host-only owner recovery; sessions, devices,
revocation and new-device alerts; the cleartext rule, HTTPS through the
owner's own domain, a tailnet or the same machine (SEC-NET-013,
SEC-NET-072), and postures; the route table, the authorisation layer and
the cross-user suites; capability URLs, stream limits and device limits;
the scan worker, its isolation self-test, quarantine and every parser
budget; read-only media and file access through root handles; artwork
re-encoding; the egress
client and the required update-check question; history erasure,
retention, export, account deletion and the "what your admin can see"
page; private listening; encrypted, signed backups and restore at setup;
the audit log, its checkpoints and the security summary; key rotation,
the compromise runbook; and the whole supply-chain and release set.

Five notes on scope:

- SEC-IAM-044 (administrators end any or all sessions of a non-owner
  account) stays in R1 and is not moved, although ADM-102 (stop a session
  with a message) moves to R1.2 with the live view. Its R1 surface is
  ACC-006: an administrator ends a person's sessions from Admin > Users >
  person, without the live view, and cannot end the owner's. WP-094
  builds and tests it (applied change A-588).
- 27 other R1 requirements are cited only by rows this R1 moves to a
  point release. Each still has an R1 carrier, so none moves:
  SEC-API-010 (WP-046, WP-064, WP-065, WP-068, WP-069: the authorisation
  layer every R1 handler uses); SEC-API-045, SEC-CLI-013 and SEC-CLI-027
  (the whole web client, CLI-001; the client plan must carry them, and
  SEC-CLI-013 applies to the claim link and invitations, ACC-001 and
  ACC-080); SEC-API-069 and SEC-NET-015 (WP-073, network settings);
  SEC-API-070 (WP-005, the return-target validator used by passkey
  sign-in, ACC-050); SEC-API-072 (WP-006, WP-044); SEC-API-079 (WP-048,
  the egress client, whose R1 purpose is the update check, ADM-053);
  SEC-API-080 (WP-102, the scan pipeline); SEC-CLI-025 (WP-089, the claim
  and invitation links); SEC-HIS-007 (WP-131); SEC-HIS-013 (WP-033,
  WP-131, every R1 admin route); SEC-HIS-037 (WP-044, WP-103);
  SEC-HIS-042 (WP-082, stream URLs, MUS-066); SEC-IAM-040 (WP-044, every
  cookie-authenticated write); SEC-IAM-076 (WP-062, WP-065, library
  grants, ACC-037); SEC-MED-026 (WP-003, WP-079, the image decoder for
  artwork, LIB-143); SEC-MED-039 (WP-024); SEC-MED-040 (WP-024, WP-060);
  SEC-NET-055 (WP-117, playback of the original file, MUS-066);
  SEC-PRV-033 (WP-087, ACC-113); SEC-STD-019 (WP-001, WP-125);
  SEC-STD-022 (WP-001, WP-047); SEC-STD-023 (WP-043); SEC-TM-049
  (WP-047). A 28th, SEC-STD-008, has no R1 surface once share links move,
  so it moves with them (table above).
- SEC-IAM-003 requires a two-party ownership transfer, so ACC-005 and
  ADM-052 stay in R1 even though few households will use them early.
- SEC-PRV-008 requires an egress ledger shown to admins, so the network
  activity page (ADM-129) stays in R1 although providers move to R1.1.
- The five rows of D-10 (SEC-NET-054, SEC-OPS-040, SEC-PRV-034 to
  SEC-PRV-036) stay proved by absence in R1 until the owner moves them.
- D-07 moves only the requirements that protect nothing but the name
  service. Those that also protect an R1 surface stay R1: SEC-NET-013
  (HTTPS without the name service) and SEC-NET-072 (certificate expiry
  alerts); SEC-OPS-007, SEC-TM-048, SEC-NET-032 and SEC-PRV-007 (no
  outbound connection before the claim, and none by default). Their only
  exception is the naming purpose of an install that chose the name
  service, so in R1 the server makes no outbound connection before the
  claim and none in its default configuration. SEC-STD-016 and
  SEC-HIS-061 also cover gunmetal.tv. SEC-NET-009 and SEC-NET-057 also
  cover ACME and the update feed. Where their text still names the name
  service, that clause applies from R2.

Taking this proposal meant three things, all done on 2026-10-03. First,
the R1.x release values of D-10 came into use. Second, the feature map's
R1 cut and summary counts were rewritten to match. Third, the work
packages for moved rows (for example WP-022's playlist-file use, WP-057,
WP-096, WP-112, WP-134, WP-137, and WP-129 and WP-135 for the name service)
left the R1 waves for their point release or R2. Wave 0 is unchanged.
Waves 1 and 2 hardly change, because most of their packages are the
parsers, stores and doors that the smaller R1 still needs.

## Applied by default

The alignment run changed 567 passages on the owner's behalf, and a
follow-up review of the same day changed 23 more (A-568 to A-590), 590 in
all; 279 of them touch one of the baseline's open owner decisions and are
marked "Confirm: yes". They are grouped below by theme, and each theme names the
register decisions that cover most of its changes; a change marked
"Confirm: yes" is confirmed or reversed with the decision its theme names,
or individually by its A- number. "Where" names the file (under docs/) and the passage;
a "missing" Before means the row or rule was added.

### T01 No passwords: sign-in, pairing and recovery

41 changes, 29 to confirm. Decision: D-06. No account passwords or authenticator codes anywhere; passkeys, browser pairing by approval and (from its release) OIDC; the recovery ladder with codes, links and a hold.

| ID | Where | Before | After | Requirements | Confirm |
|---|---|---|---|---|---|
| A-001 | features/discovery.md: DIS-188 History held during account recovery (new row, R1) | missing | While a hold that began from an admin-issued recovery link runs, history, activity rows, loves, ratings, mixes and private playlists show a 'held until' card and export is refused; browsing and playing still work. | SEC-IAM-106, SEC-IAM-090 | yes |
| A-002 | features/clients.md: CLI-150 Secure context in R1 | Over plain HTTP on a LAN address, R1 is an online-only web player that signs in with a password (ACC-052); fixes are a domain with HTTPS and the ADM-023 decision | No plain-HTTP player. Over plain HTTP every peer except loopback gets a static help page with no sign-in and no cookie, explaining how to reach the HTTPS address. There is no password fallback; a browser without passkeys is approved from the person's phone. HTTPS comes from the per-server name service as the install-time default, or from the owner's domain, a tailnet name or localhost | SEC-NET-001, SEC-IAM-025, SEC-IAM-108, SEC-NET-010, SEC-NET-013 | yes |
| A-003 | features/clients.md: CLI-027 Pair a device by QR code | Approval on the phone mints a device-bound key for the TV; the QR is scanned in every case | The TV generates its own non-exportable key and the phone's approval enrols it. Scanning the QR is enough only on the same home network; otherwise the person types the TV's code and confirms a matching code. The approval sheet marks the device as unverified, needs user verification and never grants admin rights. Approval always starts on the phone | SEC-IAM-048, SEC-IAM-055, SEC-IAM-058, SEC-IAM-059, SEC-IAM-060, SEC-STD-027 | no |
| A-004 | features/accounts.md: ACC-002 Owner account with no vendor account | When the setup page is not a secure context (a browser reaching http://<LAN IP>), the admin may set a password (ACC-052 strength, ACC-063 limiter) and is prompted to add a passkey once HTTPS exists. | Setup runs only on localhost or over HTTPS on a name. The owner's first credential is a passkey, enrolled in the same transaction that uses up the claim code. A browser on plain HTTP from the LAN gets a help page. No password is ever set. | SEC-IAM-025, SEC-IAM-008, SEC-IAM-009, SEC-NET-001, SEC-STD-006 | yes |
| A-005 | features/accounts.md: ACC-050 Passkeys | Elsewhere in R1, sign-in falls back to password plus two-factor (ACC-052, ACC-053), and ADM-023 decides how many people get passkeys. | No password fallback. HTTPS on a name or localhost comes from the name service, the owner's own domain or a tailnet. Browsers that cannot use a passkey use OIDC or approval from a signed-in device (ACC-062). Plain HTTP gets a help page. | SEC-IAM-025, SEC-STD-006, SEC-NET-001, SEC-IAM-108 | yes |
| A-006 | features/accounts.md: ACC-052 Password sign-in as a fallback | R1 password fallback. | No. | SEC-IAM-025, SEC-STD-006 | yes |
| A-007 | features/accounts.md: ACC-053 Two-factor codes with recovery codes | R1 TOTP, the default protection for most LAN installs. | No. Recovery codes become their own row (ACC-137). | SEC-IAM-025, SEC-STD-006 | yes |
| A-008 | features/accounts.md: ACC-054 Require strong sign-in | The owner requires a passkey or a second factor. | The owner can require a passkey on this server instead of OIDC alone. With this on, an OIDC-only account cannot be elevated or keep admin powers until it enrols one. | SEC-IAM-025, SEC-IAM-107, SEC-IAM-036 | yes |
| A-009 | features/accounts.md: ACC-055 Manage your own sign-in methods | Add or remove passkeys, a password, two-factor and identity providers. | Passkeys and identity providers only. Each change needs a passkey check in the last 5 minutes and notifies the other devices. The owner's last passkey cannot be removed, and a credential under a recovery hold cannot remove others. | SEC-IAM-025, SEC-IAM-023, SEC-IAM-107, SEC-IAM-106 | yes |
| A-010 | features/accounts.md: ACC-062 Approve a new browser from a signed-in one | R2, shipping with ACC-061. | R1. A limited-class paired browser with a non-extractable key: it can play but never administer, approve devices or change account security. This replaces the password fallback. | SEC-IAM-108, SEC-IAM-025, SEC-CLI-024 | yes |
| A-011 | features/accounts.md: ACC-063 Protection against guessing | Covers guessing passwords, with lockout notices. | No passwords exist. Claim codes, invitations, recovery codes, PINs, pairing codes and share-link passwords get growing delays that are never permanent. A pairing code dies after 5 wrong guesses, and failures are logged in a fail2ban-ready line. | SEC-IAM-025, SEC-API-056, SEC-TM-014 | yes |
| A-012 | features/accounts.md: ACC-064 Help a locked-out user | The admin issues a single-use, short-lived sign-in link that leads to passkey enrolment. | Admins can issue links for members and guests only, the owner for admins, and nobody for the owner. The link is redeemed in person or on a device the person already approved, and starts a recovery hold that hides history from the new passkey. | SEC-IAM-091, SEC-IAM-106 | yes |
| A-013 | features/accounts.md: ACC-137 Recovery codes (new row) | missing (recovery codes existed only inside the TOTP row ACC-053) | R1. Ten single-use codes, offered to the owner and admins at enrolment and available to everyone under Account > Recovery. The owner's go in a recovery kit with the backup key. A code can only enrol a new passkey, which starts a hold. | SEC-IAM-089, SEC-PRV-040 | yes |
| A-014 | features/accounts.md: ACC-138 Recovery hold (new row) | missing | R1. A 72-hour hold (24 to 72) on credentials enrolled through recovery. Existing devices can cancel it with one tap, and after an admin's link history stays hidden until the hold ends. | SEC-IAM-106, SEC-IAM-090 | yes |
| A-015 | features/accounts.md: ACC-065 Sign out everywhere when a credential changes | A leaked password, a password-change dialog, and an epoch bump on every credential change. | Removing a credential ends the sessions and URLs that came from it and offers to end all other sessions. | SEC-IAM-025, SEC-IAM-042 | yes |
| A-016 | features/accounts.md: ACC-077 Spot shared passwords | A per-account device and location summary. | Renamed 'Spot shared accounts'. Shows only counts of devices and coarse networks, taken from the security log within its retention, and never titles. | SEC-PRV-003, SEC-PRV-005, SEC-OPS-027 | yes |
| A-017 | features/accounts.md: Open decision 1 (passwords) and Differentiator 2 | Recommendation: allow passwords as an R1 fallback with optional two-factor. Differentiator 2 sold two-factor codes. | No passwords and no two-factor codes; browser pairing instead (security OD-1). Differentiator 2 now leads with passkeys and approval from your phone. | SEC-STD-006, SEC-IAM-025 | yes |
| A-018 | features/admin.md: ADM-019 (and ADM-021, open decision 2) | The first admin registers a passkey or links an OIDC identity where the page is a secure context, and sets a password where it is not (via ACC-002 / ACC-052). | The owner claims by registering a passkey in a secure context (HTTPS or localhost); no password at any point; OIDC can be linked afterwards but never replaces the passkey; recovery codes offered. On a non-secure page, setup shows the paths that work instead of a form. | SEC-IAM-025, SEC-NET-001, SEC-IAM-008, SEC-IAM-031, SEC-IAM-107, SEC-IAM-020 | yes |
| A-019 | features/admin.md: ADM-021 | R1 also allows a password for the first admin (ACC-002), so setup never dead-ends; whether R1 offers more is an owner decision. | Paths offered: project name service name, own domain, tailnet name, localhost via SSH tunnel. Plain HTTP gives every non-loopback peer only a help page; no password fallback. | SEC-NET-001, SEC-IAM-025, SEC-IAM-008, SEC-NET-013, SEC-NET-071 | yes |
| A-020 | features/admin.md: ADM-143 (new) | missing | Recovery kit at setup (R1): printable recovery codes plus backup recovery key, confirmation and dashboard reminder, fresh check to show again, optional PRF wrapping. | SEC-PRV-040, SEC-OPS-042, SEC-IAM-089 | no |
| A-021 | ui/flows.md, ui/surfaces.md: flows.md every flow (F01, F03, F10, F14, F15); surfaces.md SUR-008, SUR-070, SUR-078, SUR-082 | Password plus authenticator-app (TOTP) code wherever passkeys could not work, including over plain HTTP; sensitive changes re-confirmed by password | No passwords anywhere: passkeys with user verification, the household's own OIDC provider, or approval from a signed-in device (browser pairing); recovery by the recovery ladder | SEC-IAM-025, SEC-IAM-108, SEC-STD-006 | yes |
| A-022 | ui/flows.md, ui/surfaces.md: flows.md R1 facts, F01, F03, F05, F09; surfaces.md SUR-070, SUR-082, new SUR-109 | Over plain HTTP on a LAN address R1 is an online-only web player that signs in with a password | Over plain HTTP every peer except loopback gets only a static redirect or help page with no cookie, form or API; the home posture shows non-local addresses a static help page (new surface SUR-109) | SEC-NET-001, SEC-NET-024, SEC-NET-005, SEC-NET-027 | no |
| A-023 | ui/flows.md, ui/surfaces.md: flows.md F03 step 2a and step 3, F15; surfaces.md SUR-061, SUR-003 | Approving a new browser from a signed-in device (ACC-062), new-device alerts (ACC-071) and the notice centre list in R2 | All R1, with the unverified-name approval sheet, typed and matching code off the local network, limited-class paired browsers, and 'This wasn't me' on alerts | SEC-IAM-108, SEC-IAM-056, SEC-IAM-058, SEC-IAM-060, SEC-IAM-098, SEC-OPS-032, SEC-OPS-033 | no |
| A-024 | ui/flows.md, ui/surfaces.md: flows.md F10 steps 4-5a; surfaces.md SUR-071, SUR-090 | One-step invitation redemption with passkey or password | HTTPS-only landing that strips the secret, shows a generated privacy notice, enrols the invitee's own credential; member, household or multi-library invites stay pending until the inviter confirms a matching code | SEC-IAM-078, SEC-IAM-079, SEC-PRV-053, SEC-CLI-013, SEC-API-096 | no |
| A-025 | ui/flows.md, ui/surfaces.md: flows.md F14; surfaces.md SUR-098 | Encrypted backups in R2; restore verified only; members fall back to passwords after a domain change | Encrypted and signed backups in R1; restore needs the setup code, checks signature and fingerprint, rotates keys, invalidates sessions, holds looser settings and suspends restored devices; recovery ladder and SEC-NET-072 migration after an address change | SEC-PRV-039, SEC-OPS-008, SEC-OPS-042, SEC-OPS-043, SEC-OPS-044, SEC-OPS-045, SEC-NET-072 | no |
| A-026 | ui/flows.md, ui/surfaces.md: flows.md F15 step 3 | Change the password after losing a phone | Remove a passkey held only on the phone with step-up; alerts carry 'This wasn't me' | SEC-IAM-023, SEC-IAM-042, SEC-OPS-033 | no |
| A-027 | ui/flows.md, ui/surfaces.md: surfaces.md SUR-090; flows.md F03, F15 | Admin 'help sign in' link straight to passkey enrolment | Recovery link redeemed in person or on an approved device, admins for members and guests only, nobody for the owner, 72-hour hold with history hidden | SEC-IAM-091, SEC-IAM-106, SEC-IAM-090 | yes |
| A-028 | ui/player.md, ui/design-language.md: design-language.md: section 11 states table ('Sign-in failed'); section 12 A15; open question 12 | Sign-in failure gives one message for unknown users and wrong passwords. A15: password managers and paste work, and passkeys are offered where the address allows (ACC-050, ACC-052). | There are no passwords. Sign-in is by passkey, identity provider or phone approval on every address the client runs on. One failure message is given with identical response and timing. Paste is allowed in code and share-link password fields. ACC-052 is removed from A15. | SEC-IAM-025, SEC-IAM-022, SEC-API-058, SEC-IAM-108, SEC-CLI-028 | yes |
| A-029 | plan/api-needs.md: API-AUTH-03 Owner creation | Passkey, or a password and two-factor where the page is not a secure context | Passkey only; claim refused outside loopback or a secure context; recovery kit issued; ACC-052/ACC-053 to be withdrawn | SEC-IAM-008, SEC-IAM-025, SEC-STD-006, SEC-PRV-040 | yes |
| A-030 | plan/api-needs.md: API-AUTH-05 Password and two-factor sign-in | R1 password plus TOTP or recovery code sign-in | Release No: not built; browsers without passkeys use browser pairing (API-AUTH-13) | SEC-IAM-025, SEC-STD-006 | yes |
| A-031 | plan/api-needs.md: API-AUTH-07 Guessing limiter | One limiter for passwords, codes, setup codes, invite codes, PINs and pairing codes | Delay schedule only for guessable secrets; pairing code dies after 5 guesses; claim code and share links never disabled; passkey failures throttled per source, never locked | SEC-API-056, SEC-API-057, SEC-IAM-101 | no |
| A-032 | plan/api-needs.md: API-AUTH-10 Step-up confirmation | Asks for passkey or password again before a sensitive change | Separate admin session (SameSite=Strict, 15 min idle, 1 h total) plus fresh-uv routes needing a passkey or device key within 5 minutes; never OIDC alone | SEC-IAM-041, SEC-IAM-036, SEC-IAM-107, SEC-CLI-024 | no |
| A-033 | plan/api-needs.md: API-AUTH-12 Help a locked-out user | Any admin issues a single-use sign-in link with a QR code for any locked-out user | Admins for members and guests only, owner for admins, nobody for the owner; redeemed in person or on an approved device; 72-hour recovery hold hiding history | SEC-IAM-091, SEC-IAM-106, SEC-IAM-090 | yes |
| A-034 | plan/api-needs.md: API-AUTH-13 Pairing and device keys | Whole pairing protocol in R2 | Browser pairing (limited-class, Web Crypto key) in R1; native device keys and TV device flow stay R2 | SEC-IAM-056, SEC-IAM-057, SEC-IAM-058, SEC-IAM-059, SEC-IAM-060, SEC-IAM-108 | no |
| A-035 | plan/api-needs.md: API-AUTH-15 Recovery codes (new) | No recovery-code capability | Ten 80-bit codes, peppered hashes, enrol-only session, recovery hold, R1 | SEC-IAM-089, SEC-IAM-090, SEC-IAM-106 | yes |
| A-036 | plan/api-needs.md: API-USR-03 Sign-in methods | Manage passkeys, the password and two-factor | Passkeys and OIDC links only, with user verification in the last 5 minutes; refused during a recovery hold | SEC-IAM-023, SEC-IAM-025, SEC-IAM-106 | yes |
| A-037 | plan/api-needs.md: Flags item 5 (plain-HTTP installs) | A plain-HTTP LAN install gets password plus two-factor and sends the password unencrypted | No web client over plain HTTP except to loopback; HTTPS via name service, own domain, tailnet or proxy is required | SEC-NET-001, SEC-IAM-025, SEC-NET-013 | yes |
| A-038 | plan/work-packages.md: WP-038, WP-063, WP-080, WP-120, crate table (sha1, argon2), owner decision 4, coverage row API-AUTH-05 | Password sign-in with TOTP two-factor and recovery codes (WP-063, WP-120); TOTP in WP-038 with sha1; owner created with password and two-factor when setup page is not a secure context; no package for browser approval sign-in | No passwords and no TOTP. WP-063 keeps only recovery codes (moved to wave 3); WP-038 formats claim, recovery and RFC 8628 pairing codes; WP-120 is sign-out plus browser pairing by approval from the person's own device; WP-080 creates the owner only with a passkey, claiming from localhost or an HTTPS name; sha1 dropped; argon2 kept only for share-link passwords; ACC-052/ACC-053 and API-AUTH-05 withdrawn from R1 | SEC-IAM-025, SEC-STD-006, SEC-IAM-108, SEC-IAM-056, SEC-IAM-060, SEC-IAM-008, SEC-NET-001, SEC-IAM-089 | yes |
| A-039 | plan/work-packages.md: WP-064 (and every sign-in pathway) | Guessing limiter beside separate verifiers for passwords, codes and sessions | One credential verifier with a closed pathway inventory, same limits, same unknown-account handling, same audit events | SEC-HIS-046, SEC-TM-014, SEC-API-058, SEC-IAM-069 | no |
| A-580 | plan/work-packages.md: WP-109 Restore (scope, Verifies, tests, wave) | Warn when the domain changed and offer one-time sign-in links (flows G3), with no issuer, redemption or hold rules | After a domain change the restore offers WP-106's recovery enrolment links under their rules: admins for members and guests, the owner for admins, nobody for the owner (host recovery); redeemed in person or on an approved device; each starts the 72-hour hold with history hidden. A test refuses a link redeemed elsewhere. WP-109 moves to wave 5 to depend on WP-106 | SEC-IAM-091, SEC-IAM-092, SEC-IAM-106, SEC-NET-072 | no |
| A-582 | features/README.md: open decision 7, Differentiator 6, the R1 cut (ACC-052, ACC-053), review log ('Partly applied'), Releases table R1 row | In R1, sign-in falls back to a password plus two-factor wherever passkeys cannot work, and the first admin may set a password when setup is not a secure context; ADM-023 Later; 'password plus two-factor elsewhere'; ACC-052 and ACC-053 in the R1 cut; ACC-053 'stayed in R1, because password plus two-factor is the R1 default' | No passwords or authenticator codes; a browser without a passkey is approved from a signed-in device and gets a play-only session (ACC-062); ADM-023 is R1 as the install-time default once its zone is on the Public Suffix List, with own domain, tailnet and localhost as alternatives; Differentiator 6 sells passkeys and approval from your phone; ACC-052 and ACC-053 are No and out of the R1 cut; the item points to D-06 and D-07 | SEC-IAM-025, SEC-IAM-108, SEC-NET-001, SEC-STD-006, SEC-NET-070 | yes |

### T02 HTTPS, cleartext and the name service

23 changes, 15 to confirm. Decision: D-07, D-21, D-22. No web client over plain HTTP except to loopback; HTTPS from the name service, the owner's domain, a tailnet or localhost; proxies and remote paths treated as internet.

The owner's D-07 answer overrides the changes below that put the name service in R1. In R1, HTTPS comes from the owner's domain, a tailnet or localhost, and the name service is R2. The documents were realigned on 2026-10-03.

| ID | Where | Before | After | Requirements | Confirm |
|---|---|---|---|---|---|
| A-040 | features/clients.md: CLI-025 Losing the server never blocks the app | Over plain HTTP on a LAN address the web client is online-only | There is no web client over plain HTTP, only a help page. Offline loading needs a personal browser on HTTPS or localhost; a shared browser keeps nothing. The health endpoint reports only liveness | SEC-NET-001, SEC-CLI-010, SEC-NET-046 | no |
| A-041 | features/clients.md: CLI-070 Lock-screen controls from the web app | In a plain browser tab over HTTP, Media Session controls still work where the browser allows them | The web client exists only over HTTPS or on localhost, so these controls work wherever the web client runs | SEC-NET-001 | no |
| A-042 | features/clients.md: CLI-028 Custom address and proxy-friendly connections | Point the app at your own URL; each entry stores address, iroh node ID and optional headers or certificate | HTTPS only, with normal certificate checks. A self-signed server is trusted only through the fingerprint in its invite, never a 'trust anyway' button. A custom address never enrols a server by itself; the identity key is pinned from the invite. Headers and certificate keys live in the keystore. Proxy sign-in never replaces Gunmetal sign-in | SEC-CLI-042, SEC-CLI-043, SEC-NET-061, SEC-CLI-030, SEC-NET-023 | no |
| A-043 | features/clients.md: CLI-106 Chromecast from Android and the web | Casting from the web client (in practice Google's hosted Cast sender script); signed URLs | The receiver gets only per-item, per-cast-session capability URLs over HTTPS, never a session token. The web client uses the browser's built-in cast and remote-playback support, never Google's hosted sender script. Receiver credentials wait for SEC-CLI-071 (Later) | SEC-API-049, SEC-CLI-012, SEC-NET-064, SEC-NET-001 | no |
| A-044 | features/clients.md: CLI-110 Local casting and a phone relay (renamed 'Local casting, with no phone relay') | Away from home, the phone serves a local relay to the cast device; the server advertises its LAN address | The phone relay is dropped because it would carry media and a bearer capability in cleartext. At home the cast device uses the server's HTTPS name; away from home casting needs the server reachable over HTTPS | SEC-NET-001, SEC-CLI-042, SEC-NET-064, SEC-NET-011 | no |
| A-045 | features/clients.md: CLI-113 Control UPnP renderers and Sonos | The phone controls network speakers (renderers implicitly fetch over plain HTTP) | Renderers fetch over HTTPS with per-item capability URLs; plain-HTTP-only renderers are not supported. The server opens no SSDP or UPnP service | SEC-NET-001, SEC-NET-064, SEC-NET-059 | no |
| A-046 | features/accounts.md: ACC-060 Sign-in from a trusted reverse-proxy header | Later, off by default, accepted only from listed proxy addresses. | No. Proxy identity headers, Tailscale's included, are never sign-in. Adding this needs its own architecture record first. | SEC-NET-023, SEC-IAM-013 | yes |
| A-047 | features/accounts.md: ACC-097 Works behind your own reverse proxy or VPN | Forwarding headers are ignored unless the proxy is listed, 'so nobody can forge local'. | Location never grants access. Listed proxies count as public unless declared a private overlay that proves itself on every request. Requests through a public proxy get internet posture, with no admin access unless remote administration is on. Tailscale identity headers are never sign-in. | SEC-IAM-013, SEC-NET-019, SEC-NET-045, SEC-NET-023 | yes |
| A-048 | features/accounts.md: ACC-098 HTTPS served by the server | A certificate the owner supplies, or a self-signed one whose fingerprint setup shows. | Only browser-trusted certificates: the name service, the owner's domain via ACME, or a supplied certificate that validates. No self-signed certificates; plain HTTP gets a help page. | SEC-NET-001, SEC-NET-003, SEC-NET-005 | yes |
| A-049 | features/accounts.md: ACC-099 Automatic certificates for your own domain | Later. | R1. ACME DNS-01, renewal by two-thirds of the certificate's life, and owner alerts 30 and 7 days before expiry. | SEC-NET-013, SEC-NET-004, SEC-TM-010 | yes |
| A-050 | features/accounts.md: ACC-102 Remote access in the browser over iroh | Later. A WASM iroh client in the browser, with relay capacity. | R2, renamed 'without a domain'. A TLS-passthrough project edge: TLS ends on the owner's server, which also serves the web client. It is opt-in and gets internet posture. A web app hosted on a project domain is not built. Depends on the name service. | SEC-NET-041, SEC-NET-043, SEC-NET-058, SEC-IAM-014 | yes |
| A-051 | features/accounts.md: Open decision 2 (per-server HTTPS names) | Recommendation: not in R1; supplied or self-signed certificates and reverse proxies. | Yes in R1 as the install-time default (ADM-023), once the zone is on the Public Suffix List and with CT monitoring. Own domain (ACC-099), tailnet and localhost remain alternatives, with no self-signed certificates. | SEC-NET-010, SEC-NET-013, SEC-NET-070, SEC-OPS-007 | yes |
| A-052 | features/accounts.md: Open decision 8 (LDAP and header sign-in) | Add header sign-in later with a trusted-source list; LDAP if demand persists. | Neither is built without its own architecture record (ACC-059 and ACC-060 are No). | SEC-STD-010, SEC-NET-023 | yes |
| A-053 | features/admin.md: ADM-022 | R1 serves an owner-supplied certificate only; automatic ACME issuance for your own domain is ACC-099 (Later). | R1 serves an owner-supplied certificate or obtains and renews one for the owner's domain via ACME DNS-01, with expiry alerts at 30 and 7 days; TLS, naming and proxy settings are owner-only with a fresh passkey check. | SEC-NET-013, SEC-NET-004, SEC-TM-010, SEC-NET-072, SEC-TM-017 | yes |
| A-054 | features/admin.md: ADM-023 (and open decision 2) | Optional opt-in gunmetal.tv name service; Release Later, stays Later unless an ADR amends ADR 1 decisions 7 and 9. | Release R1 as the install-time default (pre-claim egress limited to name service and CA), only after its zone is on the Public Suffix List, with CT monitoring and fallback paths; if the PSL entry is missing, R1 ships with own domain, tailnet and localhost only. ADR amending ADR 1 decisions 7 and 9 still required first. | SEC-NET-010, SEC-NET-011, SEC-NET-069, SEC-NET-070, SEC-NET-071, SEC-OPS-007, SEC-HIS-061 | yes |
| A-055 | features/live-tv.md: LIV-172 | Release Later: "Uses the same device credentials as LIV-171" | Release No. Emulating an HDHomeRun needs unauthenticated plain-HTTP endpoints and LAN discovery that trusts location. Added to Deliberately not doing. | SEC-TM-004, SEC-NET-001, SEC-IAM-013, SEC-NET-059, SEC-HIS-053 | yes |
| A-056 | ui/flows.md, ui/surfaces.md: flows.md F01 other releases; surfaces.md SUR-093 | Per-server HTTPS name (ADM-023) and automatic certificates for the owner's domain (ACC-099) in Later | Both R1: own-domain ACME DNS-01 with automatic renewal, and the per-server name as install-time default only once its zone is on the Public Suffix List, with CT monitoring; otherwise R1 ships own domain, tailnet and localhost | SEC-NET-004, SEC-NET-010, SEC-NET-013, SEC-NET-069, SEC-NET-070 | yes |
| A-057 | ui/flows.md, ui/surfaces.md: flows.md F10 other releases, Journeys not designed here | Browser remote access over iroh (ACC-102) Later | R2 through the edge, off until the owner enables it, internet posture for edge requests | SEC-NET-041, SEC-NET-042, SEC-NET-043 | yes |
| A-058 | ui/flows.md, ui/surfaces.md: surfaces.md SUR-081; flows.md F13 | Emergency page shows status and log lines, offers backup and restart | Nothing before an admin passkey sign-in; backup download owner-only step-up; refused on internet posture | SEC-IAM-041, SEC-OPS-045, SEC-NET-045, SEC-NET-001 | no |
| A-059 | ui/player.md, ui/design-language.md: player.md: Lock screen > Browser media controls row; Security rules for every surface > Where the player runs; open question 14 | Media Session works only where the browser allows it, and its behaviour in a plain HTTP tab is unverified per browser (this assumed a player could run on a plain-HTTP address). | The web player runs only over HTTPS or on loopback. Every other plain-HTTP peer gets only the server's static help page, so there is no plain-HTTP player and no reduced player. | SEC-NET-001, SEC-NET-005 | yes |
| A-060 | ui/player.md, ui/design-language.md: design-language.md: section 11 states table ('This address limits the web app' row, now 'Not a secure address'); open question 11 | On first sign-in and in About, the client explains which features need HTTPS or localhost on this address (CLI-150). | Plain-HTTP peers other than loopback get only the server's static help page: no sign-in, no server name, no version. The web client never runs there, so there is no reduced app to explain. | SEC-NET-001, SEC-NET-005, SEC-NET-024 | yes |
| A-061 | plan/api-needs.md: API-SYS-02 Secure-context report | Tells the client which web features work on a non-secure address | Renamed Cleartext help page: plain-HTTP peers other than loopback get only a static redirect or help page; home posture gives non-local peers only a static help page | SEC-NET-001, SEC-NET-024, SEC-API-052 | yes |
| A-062 | plan/api-needs.md: API-SET-01 and API-SET-02 Network and egress settings | Admin (Host) settings | Owner-only (security.settings) with fresh-uv; home posture default; proxies declared private overlay or public | SEC-IAM-041, SEC-IAM-075, SEC-NET-019, SEC-OPS-038 | no |

### T03 Media processing outside the server process

32 changes, 17 to confirm. Decision: D-09. Parsing, packaging, remuxing and decoding happen in worker processes or the jail, never in the server process.

| ID | Where | Before | After | Requirements | Confirm |
|---|---|---|---|---|---|
| A-063 | features/music.md: MUS-230 Audio packaging for the web player | FLAC, Opus and MP3 frames are copied into fragmented MP4 in-process | Packaging runs in a worker process that streams to the server over a pipe, under the remuxer's step budget, memory cap and watchdog, never in the server process; a parse-back property test checks its output | SEC-MED-018, SEC-MED-081, SEC-MED-007, SEC-MED-032 | yes |
| A-064 | features/music.md: MUS-041 CUE sheets and single-file albums | The server serves each virtual track as a re-headed FLAC slice; where the slice is built and what a cue sheet's FILE entry may point to were not stated | The slice is built by the same worker as the audio packager (MUS-230), never in the server process. A FILE entry resolves only to an indexed file in the same folder; any other path is dropped and reported | SEC-MED-050, SEC-TM-043, SEC-MED-018, SEC-MED-081 | yes |
| A-065 | features/music.md: MUS-086 Loudness measured for untagged files (also Differentiator 2 and open decision 6) | Depends on an ADR approving in-process pure-Rust decoders for untrusted audio | Decoding runs in the scan worker under its memory and time limits, never in the server process; the ADR only has to admit a decoder crate after the recorded review the baseline requires | SEC-MED-018, SEC-MED-021, SEC-MED-024, SEC-MED-026, SEC-TM-034 | no |
| A-066 | features/music.md: MUS-040 Artwork sized for sync | Thumbnails made by memory-safe decoders with pixel limits; whether this runs in the sandbox was left for the security map to settle | Thumbnails are made in the scan worker, never in the server process. Pixel limits are checked before allocation, and images are re-encoded with metadata stripped to a fixed set of sizes; clients never receive original artwork bytes | SEC-MED-018, SEC-MED-045, SEC-MED-046, SEC-MED-047 | no |
| A-067 | features/library.md: Open decision 5; Dependencies (loudness bullet) | Needs an ADR approving in-process pure-Rust decoders (MUS-086); Opus needs another decoder or the sandboxed worker record 1 reserves for transcodes. | Needs an ADR approving pure-Rust decoders in the scan worker. Opus needs a memory-safe decoder in the scan worker or a native decoder in the full jail; a native decoder never runs in the server or the scan worker. | SEC-MED-018, SEC-MED-026, SEC-TM-034, SEC-TM-044 | yes |
| A-068 | features/library.md: LIB-206 Scanner isolation status (new row, R1) | missing | Admins see the scan worker's isolation tier in plain words, in Library health, on the admin health page and in `gunmetal doctor`. A missing layer shows a 'reduced isolation' notice and is never hidden. | SEC-MED-024, SEC-TM-045, SEC-OPS-061 | yes |
| A-069 | features/library.md: LIB-020 No helper process per file | Probing runs in-process in the core (record 1). | Probing runs in the core inside a small pool of long-lived, single-threaded, memory-capped scan workers, each handling many files; still no process per file and no FFmpeg. | SEC-MED-018, SEC-MED-021, SEC-TM-034 | no |
| A-070 | features/library.md: LIB-143 Safe image handling | Decoding runs in memory-safe code or the sandbox. | Decoding runs only in the scan worker with memory-safe decoders, never in the server process and never with a C image library. Only JPEG, PNG, WebP and first-frame GIF are accepted. Clients receive only re-encoded derivatives. | SEC-MED-018, SEC-MED-025, SEC-MED-044, SEC-MED-046 | no |
| A-071 | features/library.md: LIB-089 Lazy indexing | The segment map is built on first play or in a throttled job. | It is built in the scan worker, on first play or in a throttled job, under the per-user and global job limits. | SEC-MED-018, SEC-NET-053, SEC-API-064 | no |
| A-072 | features/library.md: LIB-205 Quarantined files (new row, R1) | missing | A file that crashes a scan worker twice is set aside with the reason and the scan finishes. An admin can retry it, and the quarantine lifts when the file's size or modification time changes. | SEC-MED-019, SEC-MED-018 | no |
| A-073 | features/discovery.md: DIS-075 Sonic similarity | An opt-in, low-priority, resumable server job, ideally in the same decode pass as loudness. | Decoding runs in the jailed worker or through a reviewed pure-Rust decoder in the scan worker, never in the server process; where the jail is missing the feature stays off and says why. | SEC-TM-034, SEC-TM-044, SEC-TM-045, SEC-MED-018, SEC-MED-025, SEC-MED-026 | yes |
| A-074 | features/clients.md: CLI-010 Samsung Tizen and LG webOS TVs | The TV's video element plus the in-process remuxer | The server's remuxer runs in a worker process. Each TV holds a non-extractable Web Crypto key, is limited-class with no downloads, and ships only after its secure-context test passes. Artwork is always re-encoded by the server | SEC-MED-081, SEC-CLI-070, SEC-TM-035 | yes |
| A-075 | features/clients.md: CLI-104 Headless and dedicated players | Includes playback through the server's own sound card | A headless player is a separate client with its own device key, paired as a limited-class device. It may run on the server's machine under its own account, but the server process never plays or decodes audio | SEC-TM-034, SEC-TM-044, SEC-CLI-024 | no |
| A-076 | features/admin.md: ADM-001, ADM-013, open decision 5 | R1 ships static binaries for Linux, Windows and macOS ('the R1 binary already runs on both'). | R1 is Linux only (x86-64, AArch64) plus the container image; macOS and Windows server builds and installers only once their worker sandbox profiles exist (R2). | SEC-MED-082, SEC-MED-024, SEC-TM-074 | yes |
| A-077 | features/admin.md: ADM-004 | ARMv7 in R2, gated only on wasmtime 32-bit ARM support. | ARMv7 in R2 only once wasmtime support and the scan worker's seccomp answer are recorded; not claimed as supported before that. | SEC-MED-024, SEC-MED-022 | yes |
| A-078 | features/later-media.md: LAT-117 HEIC and HEIF | "Original bytes to clients that decode HEIC natively; a sandboxed conversion for the rest" | Every client gets JPEG, PNG or WebP made by a decoder in the full jail. HEIC bytes never reach a client decoder, and the original is available only as a download (LAT-134). HDR rendering is lost, which puts this behind Immich for HDR. | SEC-MED-046, SEC-CLI-005, SEC-TM-035, SEC-MED-044, SEC-TM-044, SEC-MED-024 | yes |
| A-079 | features/later-media.md: LAT-144 Ebook formats, LAT-164 CBZ/CBT/CB7, LAT-165 CBR, LAT-169 Page counts, the Comics intro and the 'Archives' risk | "EPUB parsed in the core"; "CBZ and CBT read by the core with limits on entry count and size"; CBR via "a sandboxed decoder, or a lower tier" | No archive is read before the architecture record that SEC-HIS-019 requires. Archives are read in memory with caps on entry count, entry size and expanded size. Entry names are never used as paths, link entries are refused, and nothing is written to disk. Native RAR and 7z decoders run only in the full jail and are off without it. Non-EPUB ebook formats are served only as downloads. | SEC-HIS-019, SEC-MED-055, SEC-MED-009, SEC-HIS-015, SEC-MED-025, SEC-TM-044 | no |
| A-080 | features/later-media.md: New row LAT-181 Formats that need the jail say why they are off | missing | HEIC, CBR, CB7 and any other format that needs a native decoder run only in the full jail. Where the jail is missing they are switched off, library health lists the affected files with the reason and the fix, and no setting runs them unconfined. Later. | SEC-TM-045, SEC-MED-024, SEC-TM-044, SEC-MED-025 | yes |
| A-081 | features/video.md: VID-003 | 'In-process remux': the pure-Rust remuxer runs inside the server process. | 'Remux in a worker process': the remuxer runs in a separate worker that streams its output over a pipe, under the step budget, a per-stream memory cap and a watchdog, and writes containers from the typed model only. The row says this replaces the 'remux in-process' wording of ADR 1 and the README diagram. | SEC-MED-081, SEC-MED-018, SEC-MED-024, SEC-MED-074 | yes |
| A-082 | features/video.md: VID-009 | The self-test covered only the transcode sandbox. The remux worker's isolation tier had no visible status. | The same check also reports the remux worker's isolation tier and shows a 'reduced isolation' notice when seccomp, Landlock or namespaces are missing. A banner appears when isolation is reduced, and no setting runs either worker unconfined. | SEC-MED-024, SEC-TM-045, SEC-OPS-062 | yes |
| A-083 | features/video.md: VID-085; VID-041; open decision 8; Risks (third-party licences) | Use alass 'in its own crate outside the core'. The dolby_vision crate must meet the core's rules 'or live outside the core'. | No third-party parser touches untrusted input outside the core. alass may supply only its alignment algorithm, fed cue timings from the core's model, after a recorded review and a licence-allowlist check. dolby_vision is admitted only through the third-party parser review and the core's dependency allowlist; otherwise the RPU parser is written in the core. | SEC-HIS-036, SEC-MED-026, SEC-SUP-025, SEC-SUP-029 | no |
| A-084 | features/video.md: VID-107 | 'Parser support for MKV editions and linked segments, native clients first', relying on the player's own ordered-chapter and linked-segment handling. | The core resolves editions, ordered chapters and linked segments at scan, and resolves linked segments only by SegmentUUID through the same library's index. The remux worker serves the assembled timeline, and libmpv's ordered-chapter and reference handling stays off on every client. | SEC-MED-075, SEC-MED-076, SEC-MED-074, SEC-CLI-048 | no |
| A-085 | features/video.md: VID-187 (new row, R2) | missing | A file that crashes or times out the remux worker twice in a row is quarantined until it changes or an admin retries it. The player shows a generic typed error, and the reason goes to the admin problem list. | SEC-MED-019, SEC-MED-081, SEC-TM-040 | no |
| A-086 | features/live-tv.md: LIV-079, LIV-090, LIV-134, LIV-170, LIV-083 | "Remux in-process when only the container blocks the browser"; LIV-134 "An in-process remux after the recording ends" | Remuxing runs in the remux worker, streaming over a pipe (live browser playback, casting, Matroska repackaging, virtual-channel playout and web caption extraction). The Dependencies section states the placement. | SEC-MED-081, SEC-MED-024, SEC-MED-018 | yes |
| A-087 | ui/flows.md, ui/surfaces.md: flows.md F12 case I, F05 step 3 | In-process remuxer; packager placement unstated | Remuxer and audio packager run in worker processes streaming over a pipe | SEC-MED-081, SEC-MED-018 | yes |
| A-088 | ui/player.md, ui/design-language.md: design-language.md: section 1 (Security rules) and section 5 (Where the colour is computed) | Covers are decoded 'in memory-safe code or the sandbox' (LIB-143), which would allow decoding inside the server process. The palette from that decode is stored and synced without validation. | Covers are decoded only in the scan worker process, never in the server process. The server revalidates the palette numbers, and the client checks them again in its bounded decoder. Contrast is rechecked on the client, so a hostile server can at worst leave a page untinted. | SEC-MED-018, SEC-MED-023, SEC-MED-077, SEC-CLI-021 | no |
| A-089 | plan/api-needs.md: API-STR-04 Audio packaging for browsers | Packaging into fragmented MP4 in the server | Runs in a worker process streaming over a pipe under a step budget and memory cap | SEC-MED-018, SEC-MED-024, SEC-MED-007 | yes |
| A-090 | plan/api-needs.md: Background jobs: sandbox self-test | R2 | R1 for the scan-worker profile at every start (isolation tier shown); R2 for the transcode jail | SEC-MED-024 | no |
| A-576 | plan/work-packages.md: WP-003 (title, ADR file names, scope, Security, risks); 'Decisions the owner must make' item 3 | ADR 4 names 'the light, in-process, audio-only packager'; ADR 5 is 'in-process audio decoders' (`0005-in-process-audio-decoders.md`); Verifies SEC-MED-026 only; a rejected ADR 5 moves WP-029 and WP-114 to R2 | ADR 4 records a worker-process packager that streams over a pipe under the step budget, a memory cap and a watchdog, with the server revalidating each segment; ADR 5 is 'audio decoders in the scan worker' (`0005-audio-decoders-in-the-scan-worker.md`), and item 3 is retitled to match; Verifies SEC-MED-018, SEC-MED-024, SEC-MED-026; a rejected ADR 5 drops WP-029 and WP-114 and R1 uses tags and the fallback gain (D-09) | SEC-MED-018, SEC-MED-024, SEC-MED-026, SEC-MED-081 | yes |
| A-577 | plan/work-packages.md: WP-056, WP-061 (Job enum), WP-079 (Owns), WP-105, WP-213 (R2 outline), security coverage table | WP-061's Job enum holds only Probe, Artwork and HashWindow; WP-105 serves segments from the server crate by calling the core packager in the server process; neither WP-056 nor WP-105 verifies SEC-MED-018, SEC-MED-023 or SEC-MED-024 | The enum gains Package (WP-105), ImportFile (WP-108) and PlaylistFile (WP-112), with CueSlice in R2 (WP-213); WP-105 owns the Package worker job, run under the step budget, a memory cap and a watchdog in its own pool, streaming segments over the socket pair, and the server revalidates each segment before serving it; if the isolation floor is not met, packaging is off, never in-process; a hostile-input test shows a crash, hang or memory blow-up in one packaging worker leaves other streams serving; WP-001's dependency check forbids the server crate from calling the packager | SEC-MED-007, SEC-MED-018, SEC-MED-020, SEC-MED-023, SEC-MED-024, SEC-MED-032, SEC-MED-081 | no |
| A-578 | plan/work-packages.md: WP-057 'JSON in the core', WP-108 (imports), WP-112 (playlist files), the external-crates table (`serde_json`), item 4; decisions.md D-02 | Uploaded Last.fm and ListenBrainz exports decoded with `serde_json` in the server process; uploaded M3U files and `.m3u` sidecars found by the scan parsed by server-crate code; D-02 kept JSON decoding 'out of the core' only | Both run as worker jobs: the file is passed by read-only descriptor, decoded and parsed in the worker under the step budget and the pool's deadline, with `serde_json` linked into the worker only for the import job, and the typed rows are revalidated by the server; crash and hang tests for a hostile export and a hostile M3U; D-02 now says out of the core and out of the server process | SEC-MED-018, SEC-MED-020, SEC-MED-023, SEC-TM-031 | no |
| A-584 | features/README.md: open decisions 8 and 9, Differentiator 2, Releases table R1 row | Open decision 8 'In-process decoders for untrusted audio'; open decision 9 recommends amending ADR 2 to name 'this light, in-process, audio-only packager'; Differentiator 2 says loudness 'needs an ADR on in-process decoders'; the R1 row names 'one light audio-only packager' with no process boundary | Open decision 8 is 'Audio decoders in the scan worker': a reviewed pure-Rust decoder runs only in the scan worker; open decision 9 records the packager and CUE slicer in a worker process streaming over a pipe under the step budget, a memory cap and a watchdog; Differentiator 2 and the R1 row say the packager runs in a worker; both point to D-09 | SEC-MED-018, SEC-MED-023, SEC-MED-024, SEC-MED-026, SEC-MED-081 | yes |

### T04 What admins and other people can see

79 changes, 47 to confirm. Decision: D-34, D-37. Admins see who is playing but not what, unless each person opts in; nobody sees another adult's history; household features only by opt-in.

| ID | Where | Before | After | Requirements | Confirm |
|---|---|---|---|---|---|
| A-091 | features/music.md: MUS-051 Artist page | "Popular" means your own (or the household's) most played | "Popular" means your own most played. Household play counts are not used; a household view could come later only as a per-person opt-in that counts items at least three of those people played | SEC-PRV-022, SEC-HIS-060 | yes |
| A-092 | features/music.md: MUS-165 Library radio (also the radio risk bullet and open decision 10) | Computed from credits, genres, eras and household co-listening | Computed from credits, genres, eras and your own listening. Other people's listening is not used in R1; household co-listening could come later only from people who opt in, with the three-person threshold | SEC-PRV-022, SEC-HIS-060, SEC-PRV-024 | yes |
| A-093 | features/music.md: MUS-114 Track info sheet | Shows every listener the file path, format, size, tags as read, gain applied and identity used | Format, size, tags as read, gain applied and identity used; the file path is shown only to admins. The member response type has no path field, and the path stays in the admin inspector (ADM-125) | SEC-API-068, SEC-TM-040 | no |
| A-094 | features/music.md: MUS-140 M3U and M3U8 import and export | Path match first, then tags and MusicBrainz IDs; a report lists every unmatched line (export contents unstated, and M3U entries are normally file paths) | Matching only against libraries the importer can read; URLs, paths outside those libraries and artwork directives are dropped, never fetched or opened, and reported. Exports describe each entry by title, artist, album, duration and MusicBrainz IDs, never by server file path | SEC-HIS-018, SEC-MED-050, SEC-MED-016, SEC-API-068 | no |
| A-095 | features/music.md: MUS-190 Who is listening now (reference to ADM-099) | Music specifics: private sessions show only as private (implying admins see the titles of all other sessions) | Admins see who is listening, on which device, the bitrate and how it plays, but not the title unless that listener opted in (MUS-235). Private sessions show only as private, and each admin read is recorded in the listener's own security log | SEC-PRV-025, SEC-IAM-077, SEC-TM-054 | yes |
| A-096 | features/music.md: New row MUS-235 Show what I am playing (R1) | missing | A per-person switch, off by default and changeable only by that person, that decides whether admins see titles in the live view; the "what your admin can see" page states the choice | SEC-PRV-025, SEC-PRV-023, SEC-PRV-027, SEC-IAM-104 | yes |
| A-097 | features/library.md: LIB-061 Ratings stored in tags | UI: Admin > Libraries > Import ratings from tags (an admin imports ratings). | Each person imports into their own profile only, from Account settings. An admin can neither import ratings into another adult's profile nor see them. | SEC-PRV-025, SEC-PRV-026, SEC-PRV-022 | yes |
| A-098 | features/library.md: LIB-008 Folder view | Relative paths are already in the synced library, so the tree renders on the device and works offline. Path tree per root in the change feed. | The synced library carries folder nodes (opaque ID, parent, display name), never filesystem paths or root locations, which only admins see. The tree is built per person from visible items, and works offline only on a personal device. | SEC-API-068, SEC-IAM-070, SEC-API-015, SEC-CLI-020, SEC-CLI-010 | no |
| A-099 | features/library.md: LIB-022 Scan and job activity | Errors carry the file and the parser's typed reason, with no limit on where paths appear. | File paths appear only in admin views; the default-level log records the error code without the path or title. | SEC-API-068, SEC-PRV-043 | no |
| A-100 | features/library.md: LIB-098 Every decision explains itself | Item info 'Why is this here?' had no rule on who sees file paths. | Non-admins see the reasons without file paths; the File inspector stays admin-only. | SEC-API-068 | no |
| A-101 | features/library.md: LIB-118 Provenance; LIB-178 Undo and edit history | Each field shows 'a named person's edit'; the item History shows who changed what. | Editors' names appear only in admin views; others see the source without the name. | SEC-API-068 | no |
| A-102 | features/library.md: LIB-166 Smart collections | Rules such as 'unwatched 4K films', with no rule on whose history they use. | Watch-state rules use each viewer's own history, never the creator's, and the server applies each viewer's library access. | SEC-PRV-022, SEC-CLI-015 | no |
| A-103 | features/library.md: UI surfaces paragraph (Features section) | Who may open Library health and the File inspector was unstated. | Library health, the File inspector and every Admin surface are admin-only, because they show file paths and library-root locations. | SEC-API-068 | no |
| A-104 | features/discovery.md: DIS-006 Copy a layout to others | Set up one person's home and apply it to several people; layouts are data, so copying is a single server operation. | The admin applies a household template (DIS-005), never a copy of another adult's own layout; pins or rows pointing at things a target cannot see are dropped for that person; each person can reset to the household default. | SEC-PRV-022, SEC-PRV-025, SEC-IAM-070 | yes |
| A-105 | features/discovery.md: DIS-050 History by date | History is the log itself (ADR 1), synced, so it is complete, works offline and can be exported. | Native apps (R2) sync it and show it offline; browsers never store it; only the person sees it (no admin view, guardians of managed profiles from R2); hidden during a recovery hold (DIS-188); a 'who can see this' line (DIS-189). | SEC-TM-058, SEC-PRV-019, SEC-PRV-025, SEC-IAM-106 | yes |
| A-106 | features/discovery.md: DIS-060 More like this (owner of the neighbour table) | Similarity comes from tags, credits and the household's own co-listening; the neighbour table is built from credits, genres, era and co-listening and synced to every device. | R1 uses only the person's own listening, applied on their device. From R2, household co-listening counts only people who opted in (ACC-114), and only for pairs at least three of them played. Each device receives only neighbours among items its profile may see, and the table is rebuilt after any history erasure. | SEC-PRV-022, SEC-PRV-023, SEC-HIS-060, SEC-CLI-020, SEC-PRV-049 | yes |
| A-107 | features/discovery.md: DIS-078 Popular in this household | Aggregates from consenting profiles only, never showing who played what. | Off for everyone until each person opts in; an item appears only when at least three opted-in people played it; private sessions, removed plays and managed profiles never count. | SEC-PRV-022, SEC-PRV-023, SEC-PRV-029, SEC-HIS-060 | yes |
| A-108 | features/discovery.md: DIS-079 Send to someone in the house | Delivered inside the server as a 'from Sam' item on that person's home, through a 'Send to' menu and a per-profile inbox. | No people picker, because members never see other accounts: the sender shares a Gunmetal link by any chat; a recipient who opens it signed in can add it to a 'Suggested to you' row; the link carries only a random ID; someone without access gets the same not-found as for a missing item. | SEC-API-068, SEC-API-011, SEC-HIS-060, SEC-IAM-080 | yes |
| A-109 | features/discovery.md: DIS-106 Folder view (reference to LIB-008) | Discovery specifics: hidden for restricted profiles, because folder names cannot be filtered (shown to every other member, with relative paths synced per LIB-008). | Offered to administrators only, because folder and file names are file paths that only admin responses may carry; members browse by tags; still hidden for restricted profiles. | SEC-API-068, SEC-PRV-001 | yes |
| A-110 | features/discovery.md: DIS-139 Numbered top lists | A ranked row of the household's most played; household data only. | Built on DIS-078: only opted-in people count, an item needs at least three of them, and the row never shows who played what. | SEC-PRV-022, SEC-PRV-023, SEC-HIS-060 | yes |
| A-111 | features/discovery.md: DIS-174 System search | Gunmetal items appear in the phone's own search; only items in the active profile's allowed set are exposed. | Off until the person turns it on for their profile; never for restricted or PIN-protected profiles; nothing from private sessions is donated; entries are removed on turn-off or sign-out. | SEC-CLI-061, SEC-PRV-058, SEC-PRV-023 | no |
| A-112 | features/discovery.md: DIS-184 Household blend | Computed on the server from consenting profiles only, with no name shown per track unless each person opts in; a consent setting per profile. | Joined by a single-use invitation link (secret in the fragment); consent is per blend and revocable; no names shown; private sessions, removed plays and excluded items never count; guests and managed profiles cannot join. | SEC-HIS-060, SEC-PRV-022, SEC-PRV-023, SEC-API-068, SEC-PRV-029, SEC-CLI-013 | yes |
| A-113 | features/discovery.md: DIS-189 'Only you can see this' (new row, R1) | missing | Every activity screen says who else can see it, generated from the enforcing policy, and links to the 'what your admin can see' page. | SEC-IAM-104, SEC-PRV-027, SEC-HIS-060 | no |
| A-114 | features/clients.md: CLI-093 Offline plays and progress merge cleanly | The R1 web client queues play events made while the connection drops and uploads them on reconnect (storage not stated, implying persistence) | In R1 the queued events are held in memory only, never in browser storage, and are lost if the tab closes. The server validates uploaded events as untrusted input. Private sessions produce no events | SEC-TM-058, SEC-PRV-019, SEC-HIS-033, SEC-PRV-024 | no |
| A-115 | features/clients.md: CLI-049 TV home-screen rows | Filled from the synced watch log (published by default) | Asked once at TV pairing: yes is preselected only on a single-profile TV, no when several profiles use it. Opt-in per profile elsewhere. Never restricted or PIN-protected profiles or private sessions | SEC-CLI-061, SEC-PRV-058, SEC-PRV-023 | no |
| A-116 | features/clients.md: CLI-058 Admin on the phone | Admin screens reach phone, TV and web together | Personal-class devices only (phone, or a personal-mode browser with a passkey), never TVs. User verification is bound to the device key. Refused on internet-posture paths unless remote administration is on. The sessions view has no titles unless the person opted in | SEC-CLI-024, SEC-CLI-059, SEC-IAM-041, SEC-NET-045, SEC-PRV-025 | yes |
| A-117 | features/clients.md: CLI-096 See and revoke downloads per device | The owner sees which devices hold which items (Admin > Devices) | Each person sees what their own devices hold and can revoke them. Admins can revoke any device but see only item counts and space, never titles | SEC-PRV-025, SEC-IAM-042, SEC-IAM-044 | yes |
| A-118 | features/clients.md: CLI-155 Personal or shared browser (new, R1) | missing | A sign-in question with two explicit answers and no preselection. Shared mode keeps everything in memory, uses a session-only cookie, ends after 30 minutes idle and is a limited-class device. Personal mode stores a per-account copy | SEC-CLI-010, SEC-PRV-019, SEC-PRV-023, SEC-CLI-024 | yes |
| A-119 | features/clients.md: CLI-157 Private session on every device (new, R1, points to ACC-117) | missing | Private session within two interactions of the player on every client. Never donated to OS surfaces, queued for offline merge or kept by 'Keep what I played' | SEC-PRV-024, SEC-PRV-058 | no |
| A-120 | features/accounts.md: ACC-034 Parents see what children played | A child's history is visible to the household's adults. | Visible only to the guardians named on the managed profile. Other adults, and admins as such, cannot read it. | SEC-PRV-029, SEC-PRV-022 | yes |
| A-121 | features/accounts.md: ACC-035 Ask a parent | The request is sent as a notice to the household's adults. | The request goes only to the profile's guardians, and only a signed-in child profile can raise one. | SEC-PRV-029, SEC-PRV-030, SEC-STD-027 | yes |
| A-122 | features/accounts.md: ACC-039 Default preferences for new profiles | New profiles copy the household's language, quality and privacy settings. | Only language, theme and quality are copied. Privacy settings always start at their most private value, and only the person can loosen them. | SEC-PRV-023 | no |
| A-123 | features/accounts.md: ACC-045 See and revoke offline copies | The owner sees which devices hold which items. | Each person sees the items on their own devices. Admins see only item and byte counts per device and can revoke them. Grants last 30 days by default (1 to 90). | SEC-PRV-025, SEC-TM-054, SEC-CLI-036 | yes |
| A-124 | features/accounts.md: ACC-139 Your own security log (new row) | missing (the user could read only their sign-in entries, and admin access was not visible to them) | R1. Each person's own log covers sign-ins, devices, credential changes and every admin read or change of their data or access. Admin changes are also shown at next sign-in. | SEC-IAM-077, SEC-IAM-097, SEC-PRV-026 | no |
| A-125 | features/accounts.md: ACC-114 No social feed | Household-visible activity such as 'popular in this household' is opt-in per person. | No household activity features are built. If one is ever added, it is off for everyone and opt-in per person. | SEC-PRV-022, SEC-PRV-023 | yes |
| A-126 | features/accounts.md: ACC-115 What your admin can see | R2. | R1, covering guardians as well as admins. | SEC-IAM-104, SEC-PRV-027, SEC-TM-054 | yes |
| A-127 | features/accounts.md: ACC-116 Choose what admins see | R2. A server setting offering full history, live sessions only or totals only, with the default chosen by the owner. | R1, renamed 'Choose whether admins see what you play'. Admins never see anyone's history, ratings or private playlists. The only choice is each person's own opt-in to showing titles in live views, off by default. | SEC-PRV-025, SEC-TM-054, SEC-PRV-023 | yes |
| A-128 | features/accounts.md: ACC-119 Your own scrobbling accounts | A profile can switch scrobbling off, for example for a child. | Off for everyone by default, and only the person can turn it on. Managed profiles cannot link. The callback is bound to the person's session, only plays after linking are sent, 'now playing' is a separate switch that starts off, and unlinking deletes the token and queue. | SEC-PRV-033, SEC-PRV-029, SEC-PRV-035, SEC-PRV-036 | no |
| A-129 | features/accounts.md: Open decision 5 (what admins see) | Live sessions and totals; full history for managed children or for adults who opt in. | Who is playing and totals; titles only with the person's opt-in; never history; no household activity features. Guardians see managed profiles' history. | SEC-PRV-025, SEC-TM-054 | yes |
| A-130 | features/admin.md: ADM-036 to ADM-049 (migration privacy), ADM-041, ADM-042 | Admin runs imports of everyone's history from Admin > Migration; review queue, dry-run report and Last.fm/ListenBrainz/iTunes imports operated by the admin. | History goes only into the profile it belongs to; each person imports their own export and iTunes files from Account settings; the admin sees counts and library items, never another adult's titles; imported history for old users attaches only when they accept and can be reviewed or deleted first. | SEC-PRV-025, SEC-PRV-026, SEC-PRV-022, SEC-IAM-079 | yes |
| A-131 | features/admin.md: ADM-074 | The user log and settings export as documented JSON (a full server export of everyone's data). | Each person exports their own data (ACC-010); the owner's server export holds settings without secrets, roots, curation log and household data, never another adult's history; whole-server moves use the encrypted backup. | SEC-PRV-025, SEC-PRV-047, SEC-TM-049 | yes |
| A-132 | features/admin.md: ADM-099 | Now playing: who is listening or watching, on which device (rival parity, titles implied). | Person, device, bitrate and playback method; title only if that person opted in; never for private sessions; admin reads rate-limited and logged in the subject's security log. | SEC-PRV-025, SEC-TM-054, SEC-IAM-077, SEC-PRV-024 | yes |
| A-133 | features/admin.md: ADM-106 | Play history and top users and titles, following the users map's privacy rules. | Renamed 'Play totals and household favourites': per-person totals only; no per-title history or per-person top titles; 'popular on this server' counts only opted-in people and items played by at least three of them. | SEC-PRV-025, SEC-PRV-022, SEC-TM-054, SEC-PRV-023 | yes |
| A-134 | features/integrations.md: INT-129, INT-133 | 'Last-played times and play counts are already in the API (INT-133), so tools such as Maintainerr have what they need'; 'One read-only scope covers the log and live sessions'. | history:read covers only the key owner's own history. Household figures are aggregates from people who opted in, reported only when at least three contribute. Other people's sessions show no title unless they allow it. | SEC-PRV-025, SEC-PRV-022, SEC-HIS-060, SEC-TM-054 | yes |
| A-135 | features/integrations.md: INT-134 | 'a structured reason for every session: Opus streams in R1, remux and transcode in R2' | R1 reports direct play or why a file cannot play (R1 has no transcoder); Opus, remux and transcode come in R2. Admins see method and reason without the title unless the person allows titles. | SEC-TM-044, SEC-TM-074, SEC-PRV-025 | yes |
| A-136 | features/later-media.md: LAT-126 Partner sharing | "A household grant over both libraries" | Each partner grants the other read access to their own library. Nobody else, an admin included, can set it up for them. It never exceeds what the granter holds, it shows in both people's sharing settings, and either can end it on the next request. | SEC-IAM-073, SEC-HIS-060, SEC-TM-027, SEC-TM-028 | no |
| A-137 | features/later-media.md: LAT-155 Reading statistics | "Time read and a shareable profile" | Time read and a year in review, shown only to that person. There is no shared profile page; a person can export their figures and share them however they like. | SEC-HIS-060, SEC-PRV-022, SEC-PRV-023, SEC-PRV-025 | yes |
| A-138 | features/later-media.md: New row LAT-178 What admins see of books, podcasts and photos (reference to ACC-115) | missing | Book positions, finished dates, bookmarks, annotations, subscriptions, reading statistics, albums and photo favourites are activity data. Admins see live sessions without titles and totals only, and the what-admins-can-see page lists each kind as it ships. Later. | SEC-PRV-025, SEC-PRV-022, SEC-PRV-027, SEC-IAM-104, SEC-PRV-001, SEC-HIS-060 | yes |
| A-139 | features/later-media.md: New row LAT-179 Photo and clip locations stay private | missing | GPS and other EXIF fields are stripped from every derivative. Share links, guest galleries, household slideshows and memories shown to others carry no location. Location appears only to people who can see the original's library, and in shared albums only if the album owner turns it on. A library can ignore location entirely. Later. | SEC-MED-046, SEC-PRV-031, SEC-PRV-023, SEC-TM-050, SEC-PRV-001 | no |
| A-140 | features/video.md: VID-156 | Whether watch-together chat messages are kept after the session was left as a design choice to confirm. | Messages are not kept after the session ends, which is the most private default. They are length-limited plain text on the control channel and are rendered as text. | SEC-PRV-023, SEC-PRV-005, SEC-TM-036, SEC-API-043 | no |
| A-141 | features/video.md: VID-168; open decision 11 | 'File paths, server addresses and other users' sessions are admin-only', meaning admins see other people's playback overlays, file paths included. | Each viewer sees the overlay for their own session, and file paths and server addresses are shown to admins only. Admins never see another person's overlay. Their view of other people's sessions is the SEC-PRV-025 view (user, device, bitrate and playback method, with no title or path unless that person opted in), and each such read is recorded in that person's own security log. | SEC-PRV-025, SEC-TM-054, SEC-IAM-077, SEC-API-068 | yes |
| A-142 | features/live-tv.md: LIV-062, LIV-063 | LIV-062: removal "lists the channels, rules and scheduled recordings affected" to the admin. LIV-063: "Names, numbers, groups, mappings and favourites are written to an append-only, exportable log" shown in Admin > Live TV > History. | Household rules and recordings are listed in full. Other people's personal rules and recordings appear only as a count, and their owners are told. The source's login is deleted with it. Favourites go to each person's own user log and never into the admin history, and lineup edits are also audit events. | SEC-PRV-025, SEC-PRV-022, SEC-PRV-036, SEC-OPS-020 | yes |
| A-143 | features/live-tv.md: LIV-087, LIV-125, LIV-127, LIV-128, LIV-129, LIV-130, LIV-100 | LIV-087 "When every tuner is busy, see who has each one and choose"; LIV-125 "Stats overlay shows a shared stream"; LIV-128 a Plex-style manage-conflicts screen; LIV-129 one rules list reordered by drag across people | Other people's live viewing and personal recordings appear without name, channel or title unless the person chose to show what they watch. Viewers are never told someone shares their stream; admins see shared-upstream counts. Pre-emption warnings say "a recording". Nobody can cancel or demote another person's personal recording. Priority between people is an order of people set by an admin. Guide marks and calendars show only your own and household recordings. | SEC-PRV-022, SEC-PRV-025, SEC-HIS-060, SEC-IAM-044 | yes |
| A-144 | features/live-tv.md: LIV-101 (Later) | "Buffers kept per channel after viewers leave" feeding a shared "Recordings > Recently watched channels" list | Buffers are kept with no record of who watched. Each person's recently watched list comes from their own history, and private sessions add nothing. | SEC-PRV-022, SEC-PRV-024 | no |
| A-145 | features/live-tv.md: LIV-111, LIV-132, LIV-155 | LIV-155 "Alerts reach the admin's own devices with the reason and what happens next"; LIV-111 sent over the admin alert path; no payload rule for vendor push | The recording's owner gets the alert. Admins get household recordings in full and other people's only as "a personal recording failed" with the cause, never the title. Titles appear only in notification types the recipient turned on, and a push through Apple or Google carries only an opaque ID. | SEC-PRV-030, SEC-PRV-025, SEC-PRV-056, SEC-OPS-035 | yes |
| A-146 | features/live-tv.md: LIV-116 | "The rule chooses 'anyone', 'everyone' or named people, read from each person's watch log" | A personal rule uses its owner's own watch state. A household recording is deleted once everyone who opted in to follow it has watched it or marked it done. Private sessions never count, and the rule reports only "finished by all followers". | SEC-PRV-022, SEC-HIS-060, SEC-PRV-024, SEC-TM-054 | yes |
| A-147 | features/live-tv.md: LIV-160 | "A Live TV log anyone can read": "Event history per source and per recording inside the app, with credentials redacted" | Renamed "A Live TV log you can read in the app". Admins read source, tuner and household-recording events, and each person reads their own recordings' events. Other people's personal recordings appear only as counts, and credentials and provider URLs are redacted. | SEC-HIS-013, SEC-PRV-025, SEC-TM-057, SEC-EXT-016 | yes |
| A-148 | features/live-tv.md: LIV-161 | "See who is watching what, on which tuner, at what bitrate, and stop a stream" | Admins see who is streaming, on which device and tuner, and at what bitrate. The channel and programme appear only if the person opted in, and private sessions show as private. Each look is rate-limited and recorded in the viewed person's own security log, and stopping a stream uses ADM-102. | SEC-PRV-025, SEC-IAM-077, SEC-TM-054, SEC-PRV-024 | yes |
| A-149 | features/live-tv.md: LIV-166 | "Recordings and rules have an owner and a visibility, under per-object authorisation" (no rule for admin visibility, export or account deletion) | Admins see other people's personal recordings and rules only as counts and storage totals. A person's rules, favourites and recording list are in their own export. Deleting an account deletes its personal rules and moves its personal recordings to the trash. | SEC-PRV-025, SEC-PRV-047, SEC-PRV-051, SEC-PRV-022 | yes |
| A-150 | features/live-tv.md: LIV-178 (new reference row to ACC-117, Watching live) | missing | R3 "Private viewing covers live TV": no history, last channel, recently watched entry or resume position. A private session never counts towards delete-after-watching, emits no webhook, and shows to admins only as private. | SEC-PRV-024, SEC-TM-054, SEC-EXT-045, SEC-PRV-025 | no |
| A-151 | features/live-tv.md: LIV-179 (new reference row to ACC-115, Access) | missing | R3 "What admins can see about your live TV": the transparency page lists the live TV entries admins see (sessions without the channel unless opted in, tuner use, storage totals) and never see (personal recordings, rules, favourites, reminders, watch history). | SEC-PRV-027, SEC-IAM-104, SEC-TM-054, SEC-PRV-025 | no |
| A-152 | ui/flows.md, ui/surfaces.md: flows.md F05, F10 step 7, F12; surfaces.md SUR-083, SUR-084, SUR-002, SUR-010 | Admins see who is playing what (titles) on the dashboard and Sessions | Admins see person, device, quality and playback method; title only if that person opts in; private sessions never; each admin read rate-limited and recorded in the person's own security log | SEC-PRV-025, SEC-PRV-024, SEC-IAM-077, SEC-TM-054 | yes |
| A-153 | ui/flows.md, ui/surfaces.md: flows.md F10, F16, gap G8; surfaces.md new SUR-132, SUR-078 | 'What your admin can see' page in R2 (ACC-115) | R1 page generated from the enforcement policy, with the person's own title switch | SEC-IAM-104, SEC-PRV-027 | no |
| A-154 | ui/flows.md, ui/surfaces.md: flows.md F10 other releases; surfaces.md SUR-091, SUR-132 | ACC-116: owner chooses what admins see, up to full history | No server setting widens admin visibility; the only choice is each person's own, to show titles | SEC-PRV-025, SEC-PRV-023 | yes |
| A-155 | ui/flows.md, ui/surfaces.md: surfaces.md SUR-104; flows.md Journeys not designed here | Admin play history and top users and titles across the server, with export | Totals only (server and per person, no titles), delivery breakdown, opt-in 'popular on this server' with a three-person threshold; no per-person history or export | SEC-PRV-025, SEC-TM-054, SEC-PRV-023 | yes |
| A-156 | ui/flows.md, ui/surfaces.md: flows.md F18 step 8; surfaces.md SUR-079 | Household adults read each child's history | Only the child's designated guardians, and the child's screens say so | SEC-PRV-029, SEC-PRV-022 | no |
| A-157 | ui/flows.md, ui/surfaces.md: flows.md F20; surfaces.md SUR-122 | Tuner-busy sheet shows who holds each tuner | Names a person or channel only for people who share what they play with the household | SEC-PRV-022 | yes |
| A-158 | ui/flows.md, ui/surfaces.md: surfaces.md SUR-098 | Whole-server 'full export in documented formats' from the admin backups page | Server settings and the owner's own data only; each person exports their own | SEC-PRV-025, SEC-PRV-047 | no |
| A-159 | ui/player.md, ui/design-language.md: player.md: Foundations > One playback model (Proposal); needs table; open question 13 | The admin session list (ADM-099) derives from the same player-state value as the bar, the lock screen and the remote. | The admin list gets its own admin response type carrying user, device, bitrate and playback method. It shows the title only when the person opted in and is not in a private session. | SEC-PRV-025, SEC-PRV-024, SEC-API-068 | yes |
| A-160 | ui/player.md, ui/design-language.md: player.md: Device sheet ('In use by another profile' row, new explanatory paragraph), Remote control on a TV ('A TV in use by another profile') | Players signed in as someone else are listed in the device sheet, greyed, as 'In use by another profile'. | The sheet shows only this profile's players and the household devices it may use. Another person's own players never appear. A household TV that someone else is using shows only 'In use', with no profile name and no title. Session events are filtered per recipient. | SEC-PRV-022, SEC-API-068, SEC-API-016 | no |
| A-161 | ui/player.md, ui/design-language.md: player.md: now-playing bar table (new Options row), options menu table and paragraph, Security rules > Private listening | Private session sits only in the full player's options menu, which makes it three interactions from the wide bar. No end condition is given. | The wide bar gains an Options button whose first item is Private session. It is at most two interactions from the full player, the wide bar and the TV player. It ends when turned off or after a period without playback that the person chooses (6 hours by default, per the privacy design guidance). | SEC-PRV-024 | no |
| A-162 | ui/player.md, ui/design-language.md: design-language.md: section 11, rule 3 (and the player's 'Details' in the Playback error state) | A 'Details' disclosure shows everyone the error type and location, for diagnostics. | Details shows only the error type from the closed catalogue and the request identifier. It never shows paths, byte offsets, addresses, versions, dependency names, stack traces or submitted values. File locations appear only on admin screens. | SEC-API-072, SEC-TM-040, SEC-API-068 | no |
| A-163 | plan/api-needs.md: API-USR-09 What the admin can see | R2 | R1 | SEC-IAM-104, SEC-PRV-027 | no |
| A-164 | plan/api-needs.md: API-SYNC-06 Neighbour table | Per-household table built partly from co-listening across the household | Built from metadata and the profile's own listening; others' listening only from opted-in people and items played by at least three of them; visible items only | SEC-PRV-022, SEC-PRV-023, SEC-CLI-020 | no |
| A-165 | plan/api-needs.md: Sync model: private listening | Private session flag only, Local | Enforced on device (event sink drops events, nothing queued) and on server (no play derived from streams, private events refused, no scrobble/webhook, titles hidden from others) | SEC-PRV-024, SEC-PRV-025, SEC-PRV-035, SEC-EXT-045 | no |
| A-166 | plan/api-needs.md: Sync model: never copied | Absolute file paths withheld only from non-admins | Absolute paths never in the synced copy for anyone; admins use API-CAT-11 | SEC-API-068 | no |
| A-167 | plan/api-needs.md: API-SES-01 Session registry | Records who is playing what, for admin views | Admin views omit titles unless the person opted in and always for private sessions; admin reads rate-limited and logged to the subject | SEC-PRV-025, SEC-TM-054, SEC-IAM-077 | yes |
| A-572 | features/discovery.md: DIS-060, DIS-184, risks and Security notes; features/admin.md: ADM-106; features/accounts.md: ACC-114; ui/surfaces.md: SUR-104, the library and household surfaces' Serves lines, the alignment table; plan/api-needs.md: neighbour-table job, API-SYNC-06; plan/work-packages.md: WP-058 | DIS-184 household blend R2; DIS-060 cross-person co-listening from R2; ADM-106's 'popular on this server' list in R2; ACC-114 'builds no household activity features' while those rows were scheduled; WP-058 computed neighbours from household co-listening in R1 | Household co-listening, the blend and the popular list are Later, behind D-37 (with DIS-078 and DIS-139, already Later); ACC-114 says none in R1 or R2, and any later one is a per-person opt-in with the three-person threshold, never for managed profiles or private sessions; ADM-106 keeps per-person totals in R2; WP-058's neighbour table takes no other person's plays | SEC-PRV-022, SEC-PRV-023, SEC-HIS-060, SEC-PRV-025 | yes |
| A-583 | features/README.md: open decision 13 | Live sessions, totals and security events by default; full history only for managed child profiles or adults who opt in; ACC-115 in R2 | D-34: admins see who is playing, the device, bitrate and method, and totals; the title only with that person's own opt-in (MUS-235); never anyone's history; each look recorded in the person's security log; only named guardians see a managed profile's history (ACC-034); ACC-115 in R1 | SEC-PRV-025, SEC-TM-054, SEC-IAM-077 | yes |

### T05 History: erasure, retention and export

18 changes, 7 to confirm. Decision: D-05, D-36. History can be erased for real; retention has defaults; each person exports and deletes their own data.

| ID | Where | Before | After | Requirements | Confirm |
|---|---|---|---|---|---|
| A-168 | features/music.md: MUS-050 Continue listening by album or playlist | Uses the source recorded with each play; Server needs: listen events with source context | Uses the context IDs the queue already stores (MUS-122, LAT-009); history events keep only the fields the baseline allows and carry no source | SEC-PRV-002 | no |
| A-169 | features/music.md: MUS-184 Remove plays from history | Removal is a new event; the log stays append-only and exports honour it. Server needs: removal events | Removal erases the play within 24 hours from the log, counts, recommendations, search index and caches. Devices get a tombstone naming only the event ID, and restoring an older backup applies the erasure again. Server needs: erasure job (segment rewrite, secure delete), tombstones, erasure ledger | SEC-PRV-049, SEC-PRV-050, SEC-PRV-052 | no |
| A-170 | features/music.md: MUS-099 A quality badge that tells the truth | Server needs: decision record per play | Decision record per playback session, kept with the live session and never added to history events | SEC-PRV-002, SEC-PRV-005 | no |
| A-171 | features/music.md: New row MUS-233 Clear history for a period, or all of it (R1) | missing | Erase a date range or all history with the MUS-184 erasure, after a confirmation that says what will go | SEC-PRV-049, SEC-PRV-050, SEC-PRV-052, SEC-TM-055 | no |
| A-172 | features/music.md: New row MUS-234 Choose how long history is kept (R1) | missing | Per-person retention choice: until deleted (default, per owner decision 15), 90 days, 1 year or 2 years, enforced by the daily purge job | SEC-TM-055, SEC-PRV-005, SEC-PRV-049 | yes |
| A-173 | features/discovery.md: DIS-052 Remove a play | A removal event masks the play in every derived view while the log itself stays append-only. | Removing a play erases it from the log, derived tables, search indexes and caches within 24 hours; devices get an ID-only tombstone; the removal is re-applied after a restore. Erasure is the one sanctioned rewrite of the log, which ADR 3 must record. | SEC-PRV-049, SEC-PRV-050, SEC-PRV-052, SEC-TM-055 | no |
| A-174 | features/discovery.md: DIS-186 Clear a period or all history (new row, R1) | missing | Delete a day, a date range or all history through DIS-052's erasure, purged from devices and re-applied after restore. | SEC-PRV-049, SEC-PRV-050, SEC-PRV-052, SEC-TM-055 | no |
| A-175 | features/discovery.md: DIS-187 Choose how long history is kept (new row, R1) | missing | Per-person retention: until deleted (default), 90 days, 1 year or 2 years, enforced by a daily purge through the erasure pipeline. | SEC-PRV-005, SEC-TM-055 | yes |
| A-176 | features/accounts.md: ACC-009 Deletion with a grace period | Release Later. A tombstone plus a purge job after an unspecified grace period. | Release R1. Deletion blocks sign-in and ends sessions at once. Data stays restorable for 7 days, then the erasure pipeline removes it. The confirmation states the date the data leaves every backup, and an export is offered first. | SEC-IAM-103, SEC-PRV-051 | no |
| A-177 | features/accounts.md: ACC-136 Delete your own account (new row) | missing | R1. An adult can delete their own account after a passkey check in the last 5 minutes, with an export offered first. The same 7-day grace period and erasure pipeline as ACC-009 apply, no admin approval is needed, and the owner must transfer ownership first. | SEC-TM-055, SEC-PRV-048, SEC-PRV-051 | yes |
| A-178 | features/accounts.md: ACC-078 Sign-in and admin audit log (pointer) | Retention and IP truncation are settings. | One retention schedule (365 days; addresses coarsened at 30 days and removed at 90). The owner sees other people's addresses only shortened, and each person sees their own entries in full. | SEC-PRV-005, SEC-OPS-027 | yes |
| A-179 | features/accounts.md: ACC-118 Remove plays from history | Removing single plays only. | Remove one play, a time range or everything. Erasure completes within 24 hours and is re-applied after a restore. History can also expire automatically after 90 days, 1 year or 2 years. | SEC-PRV-049, SEC-PRV-005 | yes |
| A-180 | features/admin.md: ADM-111 and ADM-082 | Retention and anonymisation in R2 (IPs truncated or not stored); ADM-082 retention limits in R2. | ADM-111 moved to R1 with the baseline defaults (security events 365 days, addresses shortened at 30 and removed at 90, diagnostic logs 14 days or 100 MB, backups 14 days), shown in settings and privacy notice; ADM-082 keeps only size accounting in R2. | SEC-PRV-005, SEC-PRV-003, SEC-TM-055, SEC-OPS-026 | yes |
| A-181 | features/later-media.md: LAT-006, LAT-007 (user log) | "The append-only user log from record 1 stores a position..."; user-made facts are user-log events that can be exported, with nothing said about erasing them | The log is append-only for ordinary writes, but a person can erase their own events (one entry, a range or all), and erasure reaches devices as tombstones that carry only IDs. Position events carry device and time and nothing else. Each person exports or erases without an admin; an export needs a sign-in within the last 5 minutes and downloads once from a link that expires within an hour. | SEC-PRV-049, SEC-PRV-052, SEC-PRV-002, SEC-PRV-047, SEC-PRV-048, SEC-TM-055 | no |
| A-182 | plan/api-needs.md: API-USR-05 Export your data | Export behind normal sign-in | Authentication within 5 minutes, per-user rate limit, refused in recovery hold; download single-use, session-bound, at most 1 hour | SEC-PRV-047, SEC-PRV-048, SEC-IAM-106 | no |
| A-183 | plan/api-needs.md: API-USR-10 Delete my account (new) | No account deletion capability | Self-service deletion, fresh auth, 7-day grace, then erasure; R1 | SEC-IAM-103, SEC-PRV-051, SEC-PRV-048 | no |
| A-184 | plan/api-needs.md: API-LOG-03 and Flags item 8 | Remove a play by writing a hiding event; erasure deferred to a future ADR | Delete one entry, a range or all history with real erasure within 24 hours, tombstones to devices, re-applied after restore; needs an ADR extending ADR 1 decision 5 | SEC-PRV-049, SEC-PRV-050, SEC-PRV-052 | yes |
| A-579 | plan/work-packages.md: WP-108 History import and data export (Scope, Security, Tests), WP-090 (Not in scope), coverage table; plan/api-needs.md: API-SET-04 | 'Build the owner's full export of every person and the server's settings'; the two-person test expects the export to contain 'both people' | The owner's server export holds settings without secrets, library roots, the curation log, household data and the owner's own data, never another adult's history, ratings or private playlists; each person exports their own data; whole-server moves use the encrypted backup; the test checks the server export of a two-person server holds no event, rating or private playlist of the other adult; SEC-PRV-025 added to Verifies | SEC-PRV-025, SEC-PRV-047, SEC-TM-049 | no |

### T06 Share links and invitations

14 changes, 12 to confirm. Decision: D-10, D-75. Music share links under SEC-API-097; invitations with a fragment secret, a capped preset and a privacy notice.

Under the D-10 answer, music share links ship in R1.2 with SEC-API-097 and SEC-STD-008, not in R1. Invitations stay R1.

| ID | Where | Before | After | Requirements | Confirm |
|---|---|---|---|---|---|
| A-185 | features/music.md: MUS-151 Share links for music (reference to ACC-086) | Release R2; pointer row with no music specifics | Release R1. Music specifics added: one track, album or playlist; listen-only by default; expires after 30 days; at most 2 streams at once; optional password; the link suspends itself and tells the sharer when it spreads | SEC-API-097, SEC-PRV-031, SEC-TM-074 | yes |
| A-186 | features/discovery.md: DIS-185 Share a watchlist or list by expiring link | Uses the ACC-086 capability model: a signed, expiring, revocable link that shows titles only. | Follows SEC-API-097: secret in the URL fragment, expiry, revocation on the next request; titles and artwork only, never the sharer's username; not indexed; no chat preview unless turned on; managed profiles cannot create one. | SEC-API-097, SEC-PRV-031, SEC-PRV-055, SEC-CLI-013 | yes |
| A-187 | features/accounts.md: ACC-080 Invite by link or QR code | Redeeming creates the account and enrols a passkey, and the person is in with the right libraries. | A privacy notice is shown before redeeming. A 128-bit secret travels in the fragment, invitations last 7 days with 1 use, and the preset can never exceed the inviter's own access. Invitations for household or member access, or more than one library, stay pending until the inviter confirms by matching code. | SEC-IAM-078, SEC-IAM-079, SEC-PRV-053 | no |
| A-188 | features/accounts.md: ACC-082 Invitee onboarding page | A static gunmetal.tv page reads the invite secret from the URL fragment. | The page runs no script that reads the secret and loads nothing from third parties. The app takes the link itself. | SEC-IAM-078, SEC-PRV-055 | yes |
| A-189 | features/accounts.md: ACC-086 Share links for music | R2, deferred because share links are an unauthenticated surface; default expiry only. | R1, per the baseline. Listen-only, 30 days, 2 streams at once, a cap on uses or bytes, and a distinct-address suspension that alerts the sharer. No username and no preview card unless the sharer turns one on, and managed profiles cannot create links. | SEC-API-097, SEC-PRV-031, SEC-TM-004 | yes |
| A-190 | features/accounts.md: ACC-087 Password on a share link | R2. | R1. Uses the shared delay schedule and the SEC-STD-008 password rules. | SEC-API-097, SEC-STD-008, SEC-API-056 | yes |
| A-191 | features/accounts.md: ACC-088 Download switch on a share link | R2, a per-link switch. | R1. Listen-only by default; the switch appears only when the owner allows share-link downloads server-wide. | SEC-API-097 | yes |
| A-192 | features/accounts.md: ACC-089 Your shares, and everyone's for admins | R2. | R1. Only the sharer and admins can see or change a link, and revoking it takes effect on the next request. | SEC-API-097 | yes |
| A-193 | features/accounts.md: ACC-120 Every endpoint needs sign-in | In R1 the only anonymous routes are sign-in and pairing; share pages join in R2. | A reviewed anonymous list: the sign-in ceremony, the claim, invitation redemption, pairing, the R1 music share-link page, static files, and a health check that reveals no version. | SEC-TM-004, SEC-API-097 | yes |
| A-194 | features/later-media.md: LAT-109 Share a clip by expiring link (and LAT-125 Public album links, which reuses it) | "A signed, scoped, expiring capability for one item... with an optional password and download switch" set by the sharer | Follows the share-link rules: the secret is in the fragment, 30-day default expiry, 2 concurrent streams, the link suspends itself when it spreads, and the page never shows the sharer's name. Video links stay off until the owner turns them on. Downloads work only where the owner allows them server-wide, not per link. Album links show only re-encoded images, with no location or camera data. | SEC-API-097, SEC-PRV-031, SEC-PRV-055, SEC-HIS-042, SEC-STD-008, SEC-MED-046 | yes |
| A-195 | ui/flows.md, ui/surfaces.md: flows.md F10, F06; surfaces.md SUR-058, SUR-059, SUR-004 | Share links (music) in R2; public page minimal | R1 for music with the baseline's per-link limits, suspension alert and preview off; public page shows nothing of the sharer, other users, library or server name; video links R2 off by default | SEC-API-097, SEC-PRV-031, SEC-NET-047, SEC-API-028 | yes |
| A-196 | plan/api-needs.md: API-SHR-01 to API-SHR-03 Share links (new) | No share-link capability; feature map has ACC-086 to ACC-088 in R2 | Music share links in R1 (fragment secret, listen-only, 30 days, optional password, 2 streams, suspension on spread); video R2 off by default | SEC-API-097, SEC-PRV-031, SEC-PRV-055, SEC-TM-004 | yes |
| A-197 | plan/api-needs.md: API-ADM-02 and API-ADM-03 Invitations and landing | Invite with libraries, use count and expiry; landing reveals nothing | Preset capped at inviter's rights; secret in fragment; link built from configured public URL; member or multi-library invites pending a code check; privacy notice before redeeming; secure context only | SEC-IAM-078, SEC-IAM-079, SEC-API-069, SEC-API-096, SEC-PRV-053 | no |
| A-198 | plan/work-packages.md: R2 outline WP-223, new WP-134 | Share links (feature map ACC-086) only in R2 with the guest capability | Music share links in R1 (WP-134): listen-only, 30 days, optional password, per-link limits; WP-223 keeps video links (off by default) and the guest capability | SEC-API-097, SEC-PRV-031, SEC-STD-008 | yes |

### T07 Nothing written into media folders

17 changes, 15 to confirm. Decision: D-43. The server never writes into media folders.

| ID | Where | Before | After | Requirements | Confirm |
|---|---|---|---|---|---|
| A-199 | features/library.md: LIB-007 Read-only media | Read-only unless the owner enables deletion on that root (ADM-139, Later); the scanner itself never writes. Root access check that reports, but never needs, write permission. | No write path to roots at all, with no exception for deletion. Deleting files from the app (ADM-139) cannot ship while SEC-TM-042 forbids writes; it would first need a recorded owner decision that changes SEC-TM-042 for an opt-in root. `doctor` and Library settings warn when the service account could write to a root. | SEC-TM-042, SEC-MED-038, SEC-OPS-054 | yes |
| A-200 | features/library.md: LIB-132 Curation export; open decision 2; Deliberately not doing (writing into media folders) | Export to a separate directory or as a download. Open decision 2: never in place; revisit an opt-in, per-library writer if users ask after R2. | Export as a download only, so the server writes nothing outside its data directory. Open decision 2: never in R1 or R2 (owner decision 23); revisit a per-root writer with a trash only after R2, through a recorded decision that replaces SEC-TM-042 for that root. | SEC-TM-042, SEC-MED-039, SEC-HIS-015 | yes |
| A-201 | features/library.md: LIB-191 Recordings join the library | Recordings arrive with guide IDs and scans never delete them; where they are written was unstated (rivals write into a chosen library folder). | The recorder writes into the server's own recordings store under its data directory, files named by random ID, never into a library root. | SEC-TM-042, SEC-MED-038, SEC-MED-039 | yes |
| A-202 | features/discovery.md: DIS-082 Online trailers and previews | Trailers fetched online, from a plugin with a network grant; previews off by default. | The plugin downloads trailers through the egress client into the server's data directory, never into media folders; they are parsed and served like local extras, so clients never contact the trailer host and nothing is embedded. | SEC-CLI-027, SEC-PRV-016, SEC-TM-042, SEC-API-049 | yes |
| A-203 | features/discovery.md: DIS-138 Leaving-soon labels | A cleanup plugin sets a flag the rule language can read. | A plugin reads a cleanup tool's schedule (such as Maintainerr's) and sets a flag; neither Gunmetal nor its plugins delete or move media files. | SEC-TM-042, SEC-TM-065 | yes |
| A-204 | features/clients.md: CLI-085 Remuxed video downloads | The in-process remuxer writes the file | The server's remuxer runs in a worker process, streams its output and writes from the typed model only. Any cached copy lives in the data directory, never beside the original | SEC-MED-081, SEC-MED-074, SEC-TM-042 | yes |
| A-205 | features/accounts.md: ACC-046 Delete-from-library right | Deletion goes to a trash with an undo, and needs ADM-139 (an opt-in writable media root). | Kept Later, but the server never deletes or changes media files. 'Remove' hides the item from the library, with an undo and an audit entry, and the owner deletes files on the host. ADM-139 is not used. | SEC-TM-042, SEC-MED-038, SEC-OPS-054 | yes |
| A-206 | features/admin.md: ADM-089 | Nothing writes into media folders by default; sidecar writing a library-map decision; media writable if the owner enables deletion on a root (ADM-139). | Nothing ever writes into media folders: no sidecars, tag writing or deletions; doctor warns on writable roots. | SEC-OPS-054, SEC-TM-042, SEC-MED-038 | yes |
| A-207 | features/admin.md: ADM-139 | Delete media from the UI (owner) via an opt-in writable root per library, trash with grace period. | Renamed 'Remove media from the library (owner)': item hidden and trashed with undo and audit; file stays on disk; Admin > Libraries > Trash and a host command list paths for the owner's own tools; server never writes to a media root. Still Later. | SEC-OPS-054, SEC-TM-042, SEC-MED-038 | yes |
| A-208 | features/integrations.md: INT-078, INT-079 | LRCLIB 'caching results with the item'; subtitle search results go to the player, with when and where they are stored unstated. | Lookups run at scan time or on an explicit action naming the provider, never because lyrics are shown or play starts. Results are parsed as untrusted and kept in the server's data directory, never written into the media folder. | SEC-PRV-015, SEC-MED-038, SEC-API-089, SEC-API-090 | yes |
| A-209 | features/integrations.md: INT-140 | 'The server reads a documented declarative format'; 'File watcher; rule engine'; 'Export as file' | The file is read from the server's configuration directory, never written into a media folder, and parsed under budgets. 'Export as file' is a download. | SEC-MED-038, SEC-TM-042, SEC-TM-032 | yes |
| A-210 | features/later-media.md: LAT-064 Server archive of episodes | "Archived episodes become library files with per-show retention rules" | Episodes are written to a separate podcast archive folder with a quota, outside the read-only media folders and the data directory. Files are named by random IDs, never names from the feed, and the core indexes them like library files. Retention deletes only from that folder. | SEC-TM-042, SEC-OPS-054, SEC-MED-038, SEC-MED-039, SEC-TM-068 | yes |
| A-211 | features/live-tv.md: LIV-108, LIV-135, LIV-144, LIV-151 | LIV-108 "Films and shows land in the right library and folder ... Parity"; LIV-135 "Recordings sit with the rest of your shows"; LIV-144 NFO files beside recordings; LIV-151 detection on library items with no storage rule | Recordings are written only to a dedicated recordings root with its own quota, apart from the read-only media folders and the server's state, and are linked into the chosen library. NFO export writes only into the recordings root. Markers for library items are stored in the data directory. | SEC-OPS-063, SEC-TM-042, SEC-OPS-054, SEC-HIS-015 | yes |
| A-212 | ui/flows.md, ui/surfaces.md: flows.md F02 steps 1-2, F01 step 13; surfaces.md SUR-085 | Adding a library and browsing folders needed only an admin | Admin session plus a passkey check in the previous 5 minutes; roots that are filesystem roots or hold the server's own data refused; read-only checks | SEC-IAM-041, SEC-TM-017, SEC-API-022, SEC-MED-037, SEC-MED-038 | no |
| A-573 | features/library.md: LIB-191 Recordings join the library | The recorder writes recordings into 'the server's own recordings store under its data directory' | A dedicated recordings root with its own quota, separate from the media roots and the data directory, files named by random ID, as LIV-108 says; SEC-OPS-063 added | SEC-OPS-063, SEC-TM-042, SEC-MED-039 | no |
| A-574 | ui/flows.md: 'Journeys not designed here', row 'Deleting a file from the UI' | 'Media is read-only; deletion needs an opt-in writable root (open decision 17)' | 'Removing an item from the library': Later; the item is hidden and trashed with undo and audit; the file stays on disk and the owner deletes it on the host (ADM-139) | SEC-TM-042 | yes |
| A-586 | features/README.md: 'Deliberately not doing' and open decision 17 | 'Media stays read-only; the one planned exception is opt-in deletion on a writable root (ADM-139, Later)'; open decision 17 'Media read-only, with one opt-in exception'; six No rows missing from the list | No exception: the server never writes to a media root and ADM-139 only hides and trashes; open decision 17 points to D-43; ACC-052, ACC-053 (passwords and codes), ACC-059, ACC-060 (LDAP and proxy-header sign-in), ADM-131 (crash reports) and LIV-172 (HDHomeRun emulation) are listed with their reasons | SEC-TM-042, SEC-IAM-025, SEC-NET-023, SEC-PRV-009, SEC-NET-001 | yes |

### T08 Outbound traffic, metadata providers and plugins

80 changes, 24 to confirm. Decision: D-10, D-44, D-56, D-57. Every outbound request goes through the egress client to exact hosts the owner approved; providers off until turned on; plugins out of process from R2.

Under the D-10 answer, the built-in MusicBrainz and Cover Art Archive lookups ship in R1.1, not R1, with SEC-PRV-014, SEC-PRV-015 and SEC-PRV-017. R1 has no metadata provider.

| ID | Where | Before | After | Requirements | Confirm |
|---|---|---|---|---|---|
| A-213 | features/music.md: MUS-037 Tags win over online data (also open decision 5) | The core makes no network calls at all; providers can only fill gaps, through plugins | Providers fill gaps only after the owner turns one on in the required setup step, only during a scan, a scheduled refresh or an action that names the provider, and only through the one egress client with the fields sent listed; nothing is looked up at play or browse time. Whether MusicBrainz and Cover Art Archive lookups are built in for R1 or come as R2 plugins is owner decision 22 (LIB-111, LIB-112) | SEC-PRV-013, SEC-PRV-014, SEC-PRV-015, SEC-TM-048 | yes |
| A-214 | features/music.md: MUS-194 Delayed plays sent in order, and open decision 5 | Plays recorded in R1 stay in the log and can be submitted when the plugins arrive | Plays are sent starting from the moment the person linked the service; R1 plays are submitted only if the person picks a backfill range when linking | SEC-PRV-035, SEC-PRV-033 | no |
| A-215 | features/music.md: MUS-162 Online lyrics lookup | LRCLIB plugin off by default and granted one host; when lookups happen was not stated | Off until the owner turns it on. It fetches at scan time for tracks with no lyrics, or when someone presses "Find lyrics on LRCLIB", never when lyrics are shown | SEC-PRV-013, SEC-PRV-015 | no |
| A-216 | features/library.md: LIB-108 Nothing leaves by default | No lookup until the admin enables a provider; no provider code in the server core (record 2); empty provider registry; the first-run wizard explains what each provider would send. | Every provider is off until the owner turns it on in a required setup step that lists the exact fields each provider receives. Only MusicBrainz and Cover Art Archive are built in; all others are plugins. Admin > Providers shows the egress ledger (host and purpose). | SEC-PRV-013, SEC-PRV-008, SEC-TM-048, SEC-PRV-023 | yes |
| A-217 | features/library.md: LIB-111 MusicBrainz lookups | R2 plugin: one shared limiter at about one request per second; results become review-queue suggestions. | R1, built into the server as a first-party provider behind the egress client, limited to MusicBrainz hosts and off until the owner turns it on. Requests carry only normalised names, years and MusicBrainz IDs, and run only after scans, in scheduled refreshes or from an action that names MusicBrainz. Responses are decoded into typed, size-limited structures. Moved from R2 to R1. | SEC-PRV-013, SEC-PRV-014, SEC-PRV-015, SEC-PRV-017, SEC-API-079, SEC-API-081 | yes |
| A-218 | features/library.md: LIB-112 Cover Art Archive covers | R2 plugin; covers fetched by release ID; artwork cache. | R1, built in beside LIB-111 and off until the owner turns it on. The server downloads each cover once from allow-listed hosts, re-encodes it and serves it from its own origin; clients never contact the archive. Moved from R2 to R1. | SEC-PRV-013, SEC-PRV-016, SEC-API-079, SEC-API-080, SEC-TM-035 | yes |
| A-219 | features/library.md: Open decision 3; Features intro; Deliberately not doing (lookups in the server core); Dependencies (plugin host bullet) | Recommendation: tags only in R1, with MusicBrainz, Cover Art Archive and artist images as the first R2 plugins. No lookups in the server core. | Follows owner decision 22: MusicBrainz and Cover Art Archive are built into the server for R1 behind the egress client and the required provider step, with no plugin host in R1. Artist images (LIB-113) stay R2 plugins. This bends record 2 for these two providers and needs a record that says so. | SEC-PRV-013, SEC-EXT-018, SEC-TM-048 | yes |
| A-220 | features/library.md: LIB-109 TMDB provider; open decision 7; Dependencies (provider terms bullet) | Open decision 7 recommended applying for a non-commercial TMDB key in the project's name and letting users override it with their own. | No project key ships in any release. Each owner enters their own TMDB key, stored as a server secret. LIB-109 gains a key field in Admin > Providers. | SEC-TM-012, SEC-OPS-017 | yes |
| A-221 | features/library.md: LIB-139 Image by link | Paste any link; the server fetches it through the plugin host's guarded HTTP client, refusing private and loopback addresses. | The server never fetches a URL a person supplies. Only links to an enabled provider's site are accepted: the server reads the image ID, rebuilds the address on that provider's allow-listed host and fetches it through the egress client. Any other link is refused with a prompt to download the image and upload it (LIB-138). | SEC-API-080, SEC-HIS-024, SEC-PRV-016, SEC-API-079 | no |
| A-222 | features/library.md: LIB-140 Choose among candidates | Candidates from every enabled provider in one picker, with no rule on how they are fetched. | Provider images are fetched and re-encoded by the server, never loaded by the client from the provider. | SEC-PRV-016, SEC-TM-035 | no |
| A-223 | features/library.md: LIB-161 Online trailers | A plugin links to trailers the provider lists; the 'Trailer' button could embed a player. | The button opens the link outside the client, which never embeds or loads another site's player. | SEC-PRV-018, SEC-API-047 | no |
| A-224 | features/library.md: LIB-184 Several languages at once | Extra provider calls per language, with no rule on when they happen. | Made only in scans and scheduled refreshes, never because someone changed their language. | SEC-PRV-015 | no |
| A-225 | features/library.md: LIB-200 Missing episodes | Episode lists come from the active provider plugin, with no rule on when they are fetched. | Cached lists are refreshed only in scans and scheduled refreshes, so showing missing episodes never triggers a lookup. | SEC-PRV-015 | no |
| A-226 | features/library.md: LIB-015 Change detection on shares and cloud drives | Storage profile 'cloud drive' (could imply talking to a cloud API). | Cloud drives are mounted by the host; the server never talks to a cloud service itself. | SEC-PRV-007, SEC-TM-048 | no |
| A-227 | features/discovery.md: DIS-042 What's-new digest | A weekly note of new arrivals by push or email, from a notification plugin; each digest respects that person's library access. | Off for each person until they turn it on; never mentions what anyone else played; push payloads carry only an opaque ID the app resolves; no titles on lock screens; an email digest lists titles only if the person chose that. | SEC-PRV-030, SEC-PRV-056, SEC-CLI-062, SEC-PRV-023 | no |
| A-228 | features/discovery.md: DIS-043 New releases from artists you follow | A MusicBrainz plugin with a network grant, off by default; results marked 'not in your library'. | MusicBrainz lookups (LIB-111) turned on by the owner in the provider setup step; the scheduled refresh asks about the library's artists, never anyone's follow list; following is applied on the device; results are untrusted provider data with artwork re-encoded and served by the server. | SEC-PRV-013, SEC-PRV-014, SEC-PRV-015, SEC-PRV-033, SEC-TM-035, SEC-PRV-016 | yes |
| A-229 | features/discovery.md: DIS-049 Watchlist titles you do not own | Needs a metadata plugin with a network grant, off by default; entries can feed a request plugin (DIS-137). | The outside search runs only when the person presses a button that names the provider, never while typing; entries reach a request plugin only for people who turn that on for themselves. | SEC-PRV-015, SEC-PRV-033, SEC-EXT-030 | no |
| A-230 | features/discovery.md: DIS-059 Trakt and tracker sync | A per-user plugin with a network grant that reads and writes the log. | Each person turns it on for their own account; the link callback is bound to their session; it sends only plays after the link (or a backfill range they pick), never private sessions; imports arrive as their own log events; unlinking deletes the token and queued submissions. | SEC-PRV-033, SEC-PRV-034, SEC-PRV-035, SEC-PRV-036, SEC-EXT-030, SEC-EXT-031 | no |
| A-231 | features/discovery.md: DIS-076 Recommendation plugins | Plugins with network grants, chosen per library; local sources stay the default. | Library-level sources are turned on by the owner and receive only lookup evidence; sources built on a person's own listening (such as ListenBrainz recommendations) work only for people who link their own account; each source kind is a versioned plugin interface with its own consent text. | SEC-PRV-014, SEC-PRV-033, SEC-EXT-030, SEC-EXT-076 | no |
| A-232 | features/discovery.md: DIS-081 Natural-language and mood requests | Only as a plugin with an explicit grant to a model provider the owner chooses; nothing in the core. | Also used only by people who turn it on for themselves; the request text goes to the provider only on a button that names it; the provider returns criteria that the rule language evaluates on the device, so the library and history are never sent. | SEC-PRV-033, SEC-PRV-015, SEC-EXT-030, SEC-TM-065 | no |
| A-233 | features/discovery.md: DIS-097 Results outside your library | Plugin only, off by default, with results in a separate, labelled section. | Also used only by people who turn it on; nothing leaves the server while typing; the outside search runs only on a button that names the provider; search terms are never stored or logged. | SEC-PRV-015, SEC-PRV-004, SEC-PRV-033 | no |
| A-234 | features/discovery.md: DIS-098 Search plugins | A typed provider interface. | A versioned interface with its own consent text; a search plugin receives queries only from people who turned it on and may not keep them; one with a network grant is queried only on a button that names it. | SEC-EXT-076, SEC-PRV-004, SEC-PRV-015, SEC-EXT-030 | no |
| A-235 | features/accounts.md: ACC-058 Provider claims map to policies | People from the identity provider land in the right policy and libraries automatically. | Auto-registered accounts wait with no libraries until an admin approves them. Claims never make anyone the owner, and mapping a claim to admin stays off unless the owner turns it on per provider. | SEC-IAM-030, SEC-IAM-031 | no |
| A-236 | features/accounts.md: ACC-059 LDAP directory sign-in | Later, after OIDC. | No. An identity provider in front of the directory, through OIDC, covers it. Adopting LDAP needs its own architecture record first. | SEC-STD-010 | yes |
| A-237 | features/accounts.md: ACC-071 New-device alerts | R2, with email or webhook alerts if the owner configures one. | R1 in-app alerts for every event SEC-IAM-098 lists (new device, credential change, recovery use, identity-provider link change, role change, failed-attempt bursts), each with a one-step 'This wasn't me'. Email and webhooks come later. | SEC-IAM-098, SEC-OPS-032, SEC-OPS-033 | no |
| A-238 | features/accounts.md: ACC-113 Nothing leaves the house by default | Any diagnostic report or update check is opt-in and shows what it would send. | No telemetry and no crash reports. By default the only outbound contact is the naming the install chose. The update check is a required first-run question with no preselection, providers stay off until the owner turns them on, and an offline mode exists. Diagnostic bundles are downloaded by the admin, never sent. | SEC-OPS-047, SEC-PRV-009, SEC-TM-048, SEC-PRV-013 | yes |
| A-239 | features/accounts.md: ACC-127 Known-advisory banner (pointer) | The check is opt-in. | A required first-run question with no preselected answer, sending no identifiers. | SEC-OPS-047 | yes |
| A-240 | features/admin.md: ADM-024 | Everything the wizard sets can be seeded from config/environment, including the OIDC provider and the admin's OIDC subject and the update-check choice. | Libraries, OIDC provider and backup folder can be seeded; the claim (setup code plus passkey), any owner or admin identity, and the update-check answer cannot be seeded; secrets only via *_FILE. | SEC-IAM-008, SEC-IAM-031, SEC-IAM-107, SEC-OPS-047, SEC-OPS-014 | no |
| A-241 | features/admin.md: ADM-028 and ADM-053 | Setup lists every outbound feature and all are off until ticked; update check described as opt-in. | Update check is a required question with two explicit answers and nothing preselected; each metadata/artwork/lyrics provider is a required Turn on / Not now question listing the fields it receives; naming choice made at install time and shown; everything else off. ADM-053 renamed 'Update check from a signed feed, asked at setup' with TUF verification, no-identifier GET, 7-day stale warning. | SEC-OPS-047, SEC-PRV-013, SEC-PRV-007, SEC-TM-075, SEC-OPS-007, SEC-SUP-050, SEC-SUP-051 | yes |
| A-242 | features/admin.md: ADM-116 | External alert destinations including email (INT-046) in R2; admin destinations may point at the LAN. | R1 in-app alerts with the baseline's rule list, 'It was me / It wasn't me', critical alerts unmutable; R2 ntfy and webhooks opt-in, signed and content-free; email moved to Later; a LAN destination needs the owner to grant that exact host. | SEC-OPS-032, SEC-OPS-033, SEC-OPS-034, SEC-OPS-035, SEC-TM-074, SEC-API-079 | yes |
| A-243 | features/admin.md: ADM-142 (new) | missing | Security summary on the admin home (R1): listeners and public bindings, HTTPS and expiry, posture and proxies, adapters and providers, admins with credential types and last use, advisory status, backup age and encryption, audit state, isolation level. | SEC-HIS-055, SEC-OPS-061, SEC-NET-028, SEC-MED-024 | no |
| A-244 | features/admin.md: ADM-146 (new) | missing | Outbound proxy and offline mode (R1): CONNECT or SOCKS5 with remote DNS, offline mode that keeps home use working, doctor probe, offline update status line. | SEC-PRV-012, SEC-SUP-050, SEC-TM-048 | no |
| A-245 | features/integrations.md: INT-046 (Release R2 to Later) | R2 email destination: 'SMTP settings with TLS required; the same templates'. | Later. Email alerts are Later in the release scope, and SMTP is a protocol the egress client does not carry. When it comes: typed builder, fixed subjects, no titles or other people's activity, owner-configured recipients, and an egress inventory entry. | SEC-STD-032, SEC-OPS-035, SEC-TM-075, SEC-API-078 | no |
| A-246 | features/integrations.md: INT-055 | Permission review before enabling, with plugins under Admin > Plugins (any admin). | Closed manifest schema; only the owner installs and grants, with a fresh fingerprint or face check. Per-user plugins get each person's own consent screen on phone, web or desktop, never a TV. | SEC-EXT-025, SEC-EXT-038, SEC-EXT-039 | no |
| A-247 | features/integrations.md: INT-056 | 'The host's HTTP function ... blocks private and loopback addresses unless granted' | All plugin HTTP runs in the server's egress client. Grants name exact hosts (no wildcards or IP literals). Private addresses need an exact owner grant, and loopback, link-local and metadata addresses can never be granted. A plugin's returned URLs are fetched only under its own grant. | SEC-EXT-002, SEC-EXT-026, SEC-EXT-027 | no |
| A-248 | features/integrations.md: INT-064 | 'Each sync plugin reads the append-only log through its own cursor' | The host reads the log through a cursor per plugin and person and passes only that person's events: never a private session, and only plays after the link unless they pick a backfill range. | SEC-EXT-030, SEC-PRV-035, SEC-PRV-024 | no |
| A-249 | features/integrations.md: INT-065 | 'Signed with the project's release key and checked at install and at load' | Built and signed in CI with provenance, under a first-party publisher key that the signed TUF index delegates. The release key is not reused for plugins. | SEC-EXT-035, SEC-EXT-043, SEC-OPS-015 | no |
| A-250 | features/integrations.md: INT-067 (renamed 'Third-party plugins from an added index') | 'Third-party plugins installed by URL ... An unsigned plugin needs an explicit confirmation, and updates are opt-in' | Plugins come only from a signed index; the owner adds a third-party index after confirming its root key fingerprint, with a fresh check. Unsigned plugins run only in developer mode, set in the configuration file, with a banner in every client. Updates wait for the owner (INT-163). | SEC-EXT-035, SEC-EXT-037, SEC-EXT-038, SEC-EXT-040 | yes |
| A-251 | features/integrations.md: INT-069 (UI surfaces) | 'developer mode in Admin > Plugins' | Developer mode is set only in the server's configuration file, with a banner in every client. | SEC-EXT-037 | no |
| A-252 | features/integrations.md: INT-085 | 'The fetcher refuses private addresses and re-checks redirects' | Each feed's host is an exact owner-approved grant, and feed XML is parsed with external entities off. | SEC-EXT-026, SEC-API-080, SEC-HIS-034 | no |
| A-253 | features/integrations.md: INT-102, INT-104, open decision 2 | 'R1 records every play ... and the plugin submits what the services still accept when it is enabled in R2'; now-playing default unstated. | When a person links a service, only plays after the link are sent unless they pick a backfill range. Unlinking deletes the stored key and discards the queue. Now-playing is a separate setting, off by default. | SEC-PRV-035, SEC-PRV-036, SEC-PRV-033 | no |
| A-254 | features/integrations.md: INT-110 | 'The same plugin with a host the user supplies; reaching a LAN host needs the admin's approval' | The owner adds the exact host (for a LAN server, its exact host and port) to the plugin's grant with a fresh check. Nobody can point the server at an arbitrary address. | SEC-EXT-026, SEC-EXT-038, SEC-API-079 | no |
| A-255 | features/integrations.md: INT-154 | 'A plugin, off by default, with a LAN-only grant and read-only access to chosen libraries' | Only after its own architecture record, as a new versioned plugin interface (plugins cannot receive inbound requests today), never in the core. If it comes: local addresses only, read-only, owner-selected libraries, SSDP only on the local link. | SEC-NET-059, SEC-NET-066, SEC-HIS-053, SEC-EXT-076, SEC-EXT-031, SEC-IAM-013 | yes |
| A-256 | features/integrations.md: INT-163 (new, Plugin updates wait for the owner, R2) | missing | Updates stay inactive until the owner approves. An optional per-plugin automatic update applies only to same-permission updates from the same publisher key, after 72 hours. Widened permissions always wait, and downgrades are refused. | SEC-EXT-040, SEC-EXT-038, SEC-TM-066 | yes |
| A-257 | features/integrations.md: INT-164 (new, Warning when a plugin version is revoked, R2) | missing | The signed index may mark a version revoked. The server shows the reason and recommends disabling it, but never disables it itself. | SEC-EXT-041, SEC-TM-067 | yes |
| A-258 | features/later-media.md: LAT-054 Import from Audiobookshelf | "Reads ABS through its API (keys since 2.26.0) with a grant to one host", with no limit on whose history is imported | Each person imports only their own history, using their own ABS key, which is discarded after the import. Nobody, an admin included, can import another person's history. ABS is reached only at the exact host and port the owner grants (plain HTTP only for that LAN address). Replies are decoded into typed, size-capped structures. An import purpose must join the egress inventory. | SEC-PRV-025, SEC-PRV-022, SEC-API-079, SEC-EXT-004, SEC-API-081, SEC-TM-075 | yes |
| A-259 | features/later-media.md: LAT-058 Podcasts as a plugin | "Feed fetching is a plugin holding an explicit outbound grant (record 2)" | The plugin gets its own plugin interface, defined by an architecture record. Its grant names exact hosts: each show's hosts join the grant only when the owner approves them (LAT-180), and nothing is fetched before that. | SEC-EXT-026, SEC-TM-017, SEC-EXT-076, SEC-HIS-024, SEC-TM-075 | no |
| A-260 | features/later-media.md: LAT-059 Safe feed fetching | Server needs: "Guarded HTTP client in the plugin host" | Feeds go through the server's one egress client, with redirects re-checked against the show's approved hosts (three hops at most). The plugin host has no HTTP client of its own. | SEC-EXT-001, SEC-API-076, SEC-EXT-002, SEC-EXT-003, SEC-EXT-004, SEC-API-077 | no |
| A-261 | features/later-media.md: LAT-061 Subscribe by feed URL, LAT-062 OPML import | "Add any feed by its address": subscriptions are user-log events, and the pasted URL is implicitly fetched | A pasted address, or a feed named in an imported OPML file, is never fetched as a side effect of the request. It becomes a request to approve that host (LAT-180), and the first fetch waits for the owner. Feeds served only over plain HTTP are refused. | SEC-API-080, SEC-HIS-024, SEC-TM-017, SEC-EXT-026, SEC-API-078 | no |
| A-262 | features/later-media.md: LAT-065 Stream without archiving (and later-media open decision 6) | "played through the server or straight from the publisher per show (a privacy trade-off)" | Always through the server. The episode is fetched from an approved host into a temporary, size-capped, expiring cache and parsed like a library file. Fetching happens on the show's schedule or on an explicit Fetch that names the publisher, never on browse or play. Clients never contact the publisher. The direct-from-publisher option is removed. | SEC-CLI-044, SEC-CLI-048, SEC-CLI-049, SEC-PRV-015, SEC-HIS-024 | no |
| A-263 | features/later-media.md: LAT-113 Immich on the big screen | "A plugin with a grant to one host and a scoped Immich API key" (one shared key) | The Immich host gets an owner-approved grant (a separate LAN grant when it runs at home). Each person links their own read-only Immich key, encrypted so admins cannot read it, and sees only their own Immich photos. Images are re-encoded and served by Gunmetal; clients never contact Immich. | SEC-EXT-029, SEC-EXT-050, SEC-EXT-026, SEC-TM-035, SEC-PRV-016 | no |
| A-264 | features/later-media.md: LAT-130 Map of photos and clips | "map tiles come from outside, so through a plugin with a grant" (silent on who fetches them) | A tile plugin fetches tiles through the egress client under an owner-approved grant to named hosts. Its consent screen says tile requests reveal which places people look at. Each person turns the map on for themselves, and clients load tiles only from the Gunmetal server. Location is shown only as LAT-179 allows. | SEC-PRV-016, SEC-EXT-005, SEC-EXT-026, SEC-EXT-039, SEC-IAM-087 | no |
| A-265 | features/later-media.md: LAT-154 Send to Kindle or email | "A plugin with a mail grant" | The plugin gets an owner-approved grant to one mail service. Each person turns it on for themselves and registers their own device addresses, and a book goes only to those addresses. | SEC-IAM-087, SEC-TM-017, SEC-EXT-026, SEC-EXT-039 | no |
| A-266 | features/later-media.md: New row LAT-177 Private sessions for books and podcasts (reference to ACC-117) | missing | A private session keeps the place only on the device. It writes no position, bookmark, finished or statistics event and sends nothing to scrobbling plugins; leaving private mode asks whether to keep the place. Later. | SEC-PRV-024, SEC-TM-054, SEC-EXT-030, SEC-PRV-035 | no |
| A-267 | features/later-media.md: New row LAT-180 Podcast hosts the owner approves | missing | The owner sees every host a new show uses (feed, episodes, images, chapters, transcripts) and approves or refuses it with fresh user verification. Members' shows wait until then. A redirect to an unapproved host stops the fetch and queues the host. The outbound ledger shows hosts and purposes, never full URLs. Later. | SEC-TM-017, SEC-EXT-026, SEC-EXT-003, SEC-PRV-008, SEC-TM-075, SEC-API-079 | no |
| A-268 | features/video.md: VID-088 | The OpenSubtitles file hash (file size plus the first and last 64 KiB) is computed at scan so that exact matches come back at once, which implies sending the hash to the provider. | No file hash or size is sent. Requests carry only title, year, external IDs and language. The server marks a result as an exact match when the provider's own release details match this file, which is unverified for OpenSubtitles; otherwise results are ranked by release name. | SEC-PRV-014, SEC-PRV-015 | no |
| A-269 | features/video.md: VID-115 | An opt-in plugin that 'tells a third party what you watch' by looking up markers by IMDb ID as titles are played. | The owner turns the plugin on. Lookups run only during scans or scheduled refreshes and send only external IDs, so the service learns what the household owns but never what anyone watches or when. The UI is the setup provider step and Admin: plugins. | SEC-PRV-013, SEC-PRV-014, SEC-PRV-015 | no |
| A-270 | features/video.md: VID-152; Deliberately not doing (DLNA) | If DLNA is ever offered, 'it belongs in a plugin with an explicit grant'. | The core never speaks SSDP, UPnP or DLNA. DLNA can come only after its own architecture record, as a plugin under SEC-NET-066: off by default, local addresses only, admin-chosen libraries, read-only. | SEC-NET-059, SEC-HIS-053, SEC-NET-066, SEC-EXT-076 | yes |
| A-271 | features/video.md: VID-163; Deliberately not doing (sending viewing data out) | Online trailers came from a plugin that 'fetches from a third party', with no rule on who fetches or when. The rivals' pattern streams the trailer in the client while someone browses. | The server downloads trailers through the plugin's grant during scheduled refreshes, then checks and indexes them like local trailers. Clients play them from the server, never contact the trailer site, and opening a title page fetches nothing. | SEC-PRV-015, SEC-PRV-016, SEC-API-049, SEC-EXT-027 | no |
| A-272 | features/live-tv.md: LIV-015, LIV-025, LIV-002, LIV-017 | "private ranges need an explicit admin grant (for tuners and TVHeadend)"; LIV-025 "under the same LAN grant as LIV-015" | Only admins add sources. A LAN address needs the owner's grant, with a fresh passkey check, for one exact address and port. Loopback, link-local, metadata and the server's own addresses are never allowed, so a same-machine TVHeadend is reached by its LAN or container-network address. Every new grant is audit-logged and announced to all admins. | SEC-TM-017, SEC-API-079, SEC-NET-067, SEC-HIS-025, SEC-OPS-020 | no |
| A-273 | features/live-tv.md: LIV-004, LIV-005, LIV-014, LIV-168 | "MPEG-TS over HTTP and HTTPS" from any source; HLS segments and internet radio station URLs over any scheme | Feature renamed "MPEG-TS over HTTPS, and HTTP on the LAN". Internet sources, HLS segments and radio stations must use HTTPS. Plain HTTP is accepted only from a LAN device the owner granted by exact address and port, so a provider login never crosses the internet in cleartext. Added as a risk and as open decision 17. | SEC-API-078, SEC-TM-048, SEC-TM-071 | yes |
| A-274 | features/live-tv.md: LIV-005, LIV-012, LIV-057, LIV-099 | LIV-005's segment fetcher fetches variant, segment and key URLs wherever the playlist points; mirrors, backup streams and catch-up URLs had no host rule | Every variant, segment, key, mirror, backup-stream and catch-up URL is fetched only if its host is inside the source's grant. A new host (for example a CDN) waits for an admin's approval and is listed on the source screen. | SEC-EXT-075, SEC-API-080, SEC-API-079 | no |
| A-275 | features/live-tv.md: LIV-007 | "Headers per source and per stream, read from the playlist's own option lines." | A playlist may set only User-Agent and Referer, checked for line breaks and shown to the admin. Cookies, authorisation and any other header are admin-only. The default is the generic Gunmetal user agent with no Referer. Header values that hold secrets are encrypted, never shown again after entry, and redacted from logs. | SEC-EXT-005, SEC-TM-031, SEC-EXT-050, SEC-TM-057 | no |
| A-276 | features/live-tv.md: LIV-014 | Provider URLs "are redacted from logs and are left out of exports unless the admin asks" | Provider URLs and logins are encrypted at rest and never returned by any API after entry. They are always left out of logs, exports and diagnostic bundles; only the encrypted backup carries them. | SEC-TM-049, SEC-EXT-050, SEC-EXT-016 | no |
| A-277 | features/live-tv.md: LIV-024, LIV-002 | "Plug in a network tuner and it is found and used"; discovery as an automatic server need; the wizard "find tuners" | Tuners are added by address, or picked from a one-shot HDHomeRun discovery the owner starts on an interface they pick. It is never a background listener, and it ships only once the threat model's egress and socket inventories list it. A found tuner is used only after the owner grants its exact address and port. | SEC-TM-006, SEC-TM-075, SEC-TM-048, SEC-TM-017, SEC-API-079 | no |
| A-278 | features/live-tv.md: LIV-018, LIV-019, LIV-027, LIV-028 (all Later) | An RTSP client, a multicast group join ("Containers likely need host networking"), an HTSP client, and SAT>IP discovery plus RTSP, all presented as straightforward later work | Each needs its own egress purpose and an architecture record first, because the egress client accepts only http and https and refuses multicast addresses. Multicast must not need extra server privileges. SAT>IP boxes are added by address because their discovery is SSDP. All stay Later. | SEC-API-078, SEC-HIS-025, SEC-EXT-002, SEC-NET-059, SEC-HIS-053, SEC-TM-048 | no |
| A-279 | features/live-tv.md: LIV-045, LIV-058 | LIV-045: "The server fetches each image once through its grant, resizes it and serves it from cache"; LIV-058: logos "fetched once by the server", uploads unspecified | Images are fetched during the scheduled guide refresh, never on guide view, and only from hosts inside the guide source's grant or separately approved. Each image is decoded within pixel limits and re-encoded. Uploaded logos are size-capped, stripped of metadata and re-encoded. | SEC-API-080, SEC-EXT-075, SEC-PRV-060, SEC-TM-035, SEC-PRV-006, SEC-MED-061 | no |
| A-280 | features/live-tv.md: LIV-086 | Stats overlay shows "Codec, bitrate, tuner, source, errors", with no limit on what "source" shows | Sources and tuners appear by their Gunmetal names, never by provider host or URL, and the overlay describes only the viewer's own stream. | SEC-TM-071, SEC-CLI-067, SEC-PRV-022 | no |
| A-281 | features/live-tv.md: LIV-168 | "Uses the same guarded fetcher as IPTV, so a station URL cannot reach the LAN", with a station editor under Music; who adds stations and what happens to URLs in stream metadata unstated | Admins add stations, and each station's host is a grant. ICY stream metadata is parsed in the core and any URL in it is never fetched. Internet stations must use HTTPS. The station editor moves to Admin > Radio stations. | SEC-HIS-025, SEC-API-080, SEC-EXT-075 | no |
| A-282 | features/live-tv.md: LIV-177 (new row, Sources) | missing | R3 "What each source can learn": each source screen states what the provider sees (public address, which channel is tuned and when, user agent, login). Refreshes run only on a schedule, and every contact appears on the network activity page (ADM-129). | SEC-PRV-060, SEC-PRV-008, SEC-TM-075, SEC-OPS-060, SEC-EXT-005 | no |
| A-283 | features/live-tv.md: LIV-180 (new row, Sources) | missing | R3 "Review and revoke source grants": every host Live TV may contact, with who granted it, its last contact and a revoke button. LAN grants are owner-only with a passkey check, and new grants are audit-logged and announced to admins. Revoking stops the next fetch and closes any stream that uses it. | SEC-TM-017, SEC-EXT-075, SEC-API-079, SEC-OPS-020, SEC-PRV-008 | no |
| A-284 | ui/flows.md, ui/surfaces.md: flows.md F01 step 11, F13; surfaces.md SUR-082, SUR-099 | Owner 'decides whether to allow' the signed update check | A required first-run question with two explicit answers and none preselected; dashboard and doctor flag it while off | SEC-OPS-047, SEC-OPS-019, SEC-SUP-051 | yes |
| A-285 | ui/flows.md, ui/surfaces.md: surfaces.md SUR-095 | Third-party plugins added by URL; developer mode as a screen | Plugins only through a signed index the owner added (root key fingerprint confirmed); developer mode only in the config file with a banner; owner-only step-up | SEC-EXT-035, SEC-EXT-037, SEC-EXT-038, SEC-EXT-040 | yes |
| A-286 | ui/flows.md, ui/surfaces.md: surfaces.md SUR-092 | Sign-in and security settings under the general admin rule | Owner-only, step-up, provider claims never confer owner, lifetimes may only be shortened | SEC-IAM-075, SEC-IAM-031, SEC-IAM-041, SEC-OPS-031 | no |
| A-287 | plan/api-needs.md: API-AUTH-02 Setup state | Locale, owner, server name, privacy, import, libraries as setup steps before setup closes | Claim first; setup routes 404 forever after; remaining steps (incl. required update question and provider step) are owner settings behind the admin session; expired code replaced by gunmetal claim-code | SEC-IAM-006, SEC-IAM-009, SEC-OPS-006, SEC-OPS-047, SEC-PRV-013 | no |
| A-288 | plan/api-needs.md: API-SET-06 Updates | An opt-in check of a signed feed | Required first-run question with two explicit answers and no preselection; plain GET; feed can never change the server | SEC-OPS-047, SEC-SUP-051, SEC-OPS-019 | yes |
| A-289 | plan/api-needs.md: Background jobs: metadata providers and lookups | R2, in the sandboxed plugin host | Built-in MusicBrainz and cover-art lookups in R1, off until the owner turns them on in the required provider step, scans only; plugins R2 | SEC-PRV-013, SEC-PRV-014, SEC-PRV-015, SEC-API-079 | yes |
| A-570 | features/live-tv.md: LIV-026 TVHeadend as a source, LIV-004, open decision 17, Security notes residual risk | Over plain HTTP the TVHeadend login is sent 'across the LAN in cleartext' with an on-screen warning and a suggestion of a streaming-only account | A source login is sent only over HTTPS. Over plain HTTP no credential is sent: a plain-HTTP TVHeadend works only with its anonymous streaming access restricted to the server's address, and the wizard refuses a login on an http URL and says why | SEC-API-078, SEC-TM-071, SEC-NET-001, SEC-EXT-050 | no |
| A-587 | features/README.md: open decision 15 (Telemetry) | 'No usage telemetry, ever; opt-in, per-report crash sending after R1, with the full report shown first' | D-38: no telemetry and no crash sending to the project in R1 and R2; crash records go into the diagnostic bundle (ADM-124), which the admin may attach to a report themselves; project services hold only the name service's routing data | SEC-PRV-009, SEC-TM-053, SEC-HIS-061 | yes |
| A-590 | features/README.md: open decision 6 (plugin host and scrobblers) | 'A wasmtime host'; 'R1 records every play ... so the R2 scrobblers can submit what Last.fm and ListenBrainz still accept' | D-56: each plugin in its own OS-sandboxed process, no plugin runtime in R1; scrobblers send only plays after the person links a service, or a backfill range they pick | SEC-PRV-033, SEC-EXT-018 | no |

### T09 API keys, adapters and webhooks

56 changes, 28 to confirm. Decision: D-54, D-55, D-58. API keys and adapters in R2 with no administrator scopes; webhooks owner-enabled and limited by audience.

| ID | Where | Before | After | Requirements | Confirm |
|---|---|---|---|---|---|
| A-290 | features/music.md: MUS-207 Music through the OpenSubsonic adapter | API-key sign-in only (ACC-129); default state, transport and now-playing scope unstated | Off until the owner turns it on; API-key sign-in only, over HTTPS only with no local-network exception; getNowPlaying lists only the caller's own sessions unless others opted in | SEC-TM-070, SEC-EXT-051, SEC-EXT-066, SEC-PRV-032 | yes |
| A-291 | features/library.md: LIB-026 Refresh API for download tools | R2: a native endpoint behind a token scoped to 'refresh these roots'; the Jellyfin and Subsonic adapters map onto it later. | Later. Starting a scan is an administrator capability and no API key may hold one, so this waits for the architecture record SEC-EXT-010 requires for an administrator automation credential. Adapters never expose it. Until then, watching and polling (LIB-014, LIB-015) pick up downloads. Moved from R2 to Later. | SEC-EXT-010, SEC-EXT-055, SEC-API-064 | yes |
| A-292 | features/accounts.md: ACC-049 Scoped tokens for integrations | R1. Tokens 'can expire', and a stats tool or admin integration was implied. | R2. No admin, owner-only or host-equivalent scope. Tokens expire after 365 days by default (30 to 730) and are disabled after 180 days unused. Creating one needs a fresh passkey check, and no token can manage any credential, its own included. | SEC-IAM-083, SEC-EXT-010, SEC-EXT-012, SEC-EXT-013 | yes |
| A-293 | features/accounts.md: ACC-129 App passwords for third-party apps | Older Subsonic clients get a random per-app password. | A per-app key the person marks as legacy: home network only, 90 days by default, and an alert on first use from a new kind of network. Token-and-salt stays refused. | SEC-EXT-069, SEC-API-094 | yes |
| A-294 | features/accounts.md: ACC-131 Quick Connect through the Jellyfin adapter | Uses the same pairing flow as ACC-061, which binds a device key. | Same approval rules (8-character code, unverified name, typed and matching code when remote), but approval grants only a revocable adapter token because Jellyfin apps hold no device key. Not built if those apps cannot show an 8-character code. | SEC-IAM-056, SEC-IAM-055, SEC-EXT-070 | no |
| A-295 | features/integrations.md: INT-011, INT-012, INT-017, INT-018, INT-019, INT-020, INT-021, INT-022, INT-026 (Release); tool-access notes on INT-138; Differentiators 1 and 6; new open decision 13 | Scoped tokens, path-scoped refresh, per-token limits, expiry, audit, revocation, no-escalation and the 'who am I' check are R1, and the differentiators say 'INT-018 to INT-023 in R1' and 'INT-011 in R1'. | These rows are R2, because API keys arrive in R2. In R1 the native API serves Gunmetal's own clients only, the limiter is keyed on the signed-in person, and the header-only rule (INT-023) stays R1. INT-138 says tools reach playlist routes with keys from R2. New open decision 13 records the trade-off. | SEC-EXT-008, SEC-EXT-010, SEC-EXT-013, SEC-EXT-014, SEC-TM-074, SEC-API-057 | yes |
| A-296 | features/integrations.md: INT-017 (What the user gets) | Named scopes 'refresh library', 'read history', 'read library and availability', 'play', with no limit on whose history. | The scopes map to the baseline vocabulary (admin:library granted only by admins, history:read for the key owner's own history only, library:read, media:stream). No key ever holds an administrator, owner-only or host-equivalent scope. ACC-049 must carry the same R2 release. | SEC-EXT-010, SEC-EXT-011, SEC-PRV-025 | yes |
| A-297 | features/integrations.md: Deliberately not doing, last bullet | 'All-powerful keys as the default way to connect a tool. An admin scope can exist for the owner's own scripts, but recipes never hand it out.' | 'Keys with administrator powers': no key may hold an administrator, owner-only or host-equivalent scope, and an administrator automation credential would need its own architecture record. The narrow admin-granted refresh and status scopes stay. Added bullets: no plugins in R1 or in-process, no OAuth server before its record, no household history for tools, no writing into media folders. | SEC-EXT-010, SEC-EXT-018, SEC-STD-026, SEC-PRV-025, SEC-MED-038 | yes |
| A-298 | features/integrations.md: INT-022 | 'A token can never create or widen a token beyond its own rights'; checked against the caller's own scopes. | No key can create, change, list or reveal any key, including itself. Keys are made only in an interactive session with a fresh fingerprint or face check, and never wider than the creator's current rights. | SEC-EXT-012, SEC-API-020, SEC-IAM-073 | no |
| A-299 | features/integrations.md: INT-019 | 'Every token expires, with a default set by its type. Rotation issues a successor...' | 365 days by default (30 to 730), disabled after 180 days unused, 14-day notice. The successor is created interactively with a fresh check, never by the old key. | SEC-EXT-013, SEC-EXT-012 | no |
| A-300 | features/integrations.md: INT-020 | 'the list shows last use, address and client' | The list shows last use, client and a coarse last-used address. Full addresses stay in the person's own security log under the retention schedule. | SEC-EXT-014, SEC-PRV-003, SEC-OPS-027 | yes |
| A-301 | features/integrations.md: INT-016 and open decision 12 | 'A thin layer over read-only scoped tokens, off by default' (auth method unstated). | Authenticates with an API key in the Authorization header, never through an OAuth authorisation server on Gunmetal. It sees only the key owner's own data. | SEC-STD-026, SEC-PRV-022, SEC-API-004 | no |
| A-302 | features/integrations.md: INT-027 | 'the tool shows a code, and the user approves it on a signed-in device and picks the scopes' | The person types the tool's code on their own signed-in device, sees the tool's name marked unverified, picks only scopes they hold (never an administrator scope) and confirms with a fresh check. The result is an ordinary, revocable API key. | SEC-STD-027, SEC-IAM-058, SEC-IAM-060, SEC-EXT-012 | no |
| A-303 | features/integrations.md: INT-028 (Release R2 to Later) and open decision 7 | R2: 'A consent screen lets a tool get a scoped, expiring token for one user'; authorisation-code flow; Seerr users offered the consent flow. | Later, behind an architecture record for delegated third-party access (ASVS 10.4, 10.6, 10.7 at Level 3). Until then there is no OAuth authorisation server. In R2 a person gives a tool their own key through the device-code flow (INT-027). | SEC-STD-026, SEC-TM-074 | no |
| A-304 | features/integrations.md: INT-029 | Inventory shows every token, webhook, plugin and app and 'when it last acted', for everyone, to admins. | Admins see kind, scopes, owner, creation and expiry. When a member's own key, app or per-user plugin last acted is shown only to that member, because it would reveal when they listen. | SEC-PRV-025, SEC-EXT-014 | yes |
| A-305 | features/integrations.md: INT-030 | 'The outbox table ships in R1 ... so no event from R1 onward is lost; delivery to destinations arrives in R2' (implying R1 events are delivered later). | Webhooks stay off until the owner enables them and lists destination hosts. The R1 outbox serves the server's own sync and event stream under the retention schedule. A webhook receives only events after it is created, so nothing recorded in R1 is later sent anywhere. | SEC-EXT-045, SEC-PRV-005, SEC-PRV-023, SEC-OPS-035 | no |
| A-306 | features/integrations.md: INT-032, INT-035 | Playback, played, rating and favourite events offered to webhooks with no audience rule. | A person's events go only to their own webhooks or to admin webhooks they opted into (INT-162). Private sessions emit nothing, and admin webhooks carry titles only for people who allow titles. | SEC-EXT-045, SEC-PRV-025, SEC-PRV-033, SEC-PRV-024 | yes |
| A-307 | features/integrations.md: INT-036 | New device, failed sign-ins and token events offered to webhooks without limits on audience or payload. | A person's own security events go to their own webhooks; server-wide alerts go to the owner's. Payloads carry only type, time and a link, never addresses, other people's names or secrets. | SEC-EXT-045, SEC-OPS-035, SEC-OPS-027 | no |
| A-308 | features/integrations.md: INT-037 | 'Integrity events carry the typed parser or check error, so the message says what is wrong and where' | They carry the typed error and the item ID, never a file path. 'Update available' exists only if the owner said yes to the update check at first run. | SEC-EXT-045, SEC-TM-040, SEC-OPS-047 | no |
| A-309 | features/integrations.md: INT-038 | 'secret rotation with overlap'; UI 'Webhook editor (reveal and rotate the secret)' | The secret is shown once and can be rotated (old and new both sign for 24 hours) but is never revealed again. | SEC-EXT-046, SEC-EXT-050 | no |
| A-310 | features/integrations.md: INT-039 and Differentiator 3 | 'The log shows the request, status and latency'; 'logs every delivery' | The delivery log shows only time and success or failure, never status code, latency or body. Queues hold at most 1,000 events per endpoint, and an endpoint failing for 72 hours is switched off and its owner told. | SEC-EXT-048, SEC-EXT-049 | no |
| A-311 | features/integrations.md: INT-040 | 'Samples are generated from the schema using real items from your library' (test send behaviour unstated) | Samples use items from libraries the webhook may see, never anyone's activity. A test send reports only success or failure and is limited to 5 a minute. | SEC-EXT-048, SEC-EXT-045 | no |
| A-312 | features/integrations.md: INT-043 | 'Uses signed, expiring image URLs. When the server is not public, items with a MusicBrainz ID use Cover Art Archive.' | No server URL or token ever appears in a payload, because capability URLs are session-bound. Items with a release MBID may carry the public Cover Art Archive link; others go without artwork. | SEC-API-026, SEC-API-028, SEC-EXT-045 | no |
| A-313 | features/integrations.md: INT-047 (Release R2 to Later) | R2 MQTT destination that 'Adds a retained "now playing" topic per session'. | Later, until an architecture record adds MQTT to the egress client and inventory; an HTTPS webhook to a bridge covers it until then. A now-playing topic would carry only opted-in people, never a private session, and titles only if they allow them. | SEC-API-078, SEC-TM-075, SEC-EXT-045, SEC-PRV-033 | yes |
| A-314 | features/integrations.md: INT-049, INT-050, INT-051; Dependencies and risks (network rules bullet) | 'Admin-created webhooks may target private addresses. Per-user webhooks and plugins may not, unless the admin allows it'; the user's own ntfy topic and per-user webhooks to any host; 'Admin webhooks legitimately reach LAN services'. | Webhooks stay off until the owner enables them and lists destination hosts. A private address works only as an exact host and port the owner entered, never a range, and never loopback, link-local or metadata addresses. Redirects are not followed. Per-user webhooks and ntfy topics may use only listed hosts, and guests get none by default. | SEC-EXT-045, SEC-EXT-047, SEC-EXT-002, SEC-API-079 | yes |
| A-315 | features/integrations.md: INT-054, INT-066, open decision 2 | 'A wasmtime host inside the server ... R1 runs first-party plugins only'; 'Last.fm and ListenBrainz ship in R1'. | Each plugin runs in its own OS-sandboxed process over one IPC channel. No WebAssembly runtime or plugin code is linked in R1. Last.fm and ListenBrainz ship in R2 with the host. | SEC-EXT-018, SEC-EXT-021, SEC-EXT-022, SEC-TM-065 | yes |
| A-316 | features/integrations.md: INT-088 (Release Later to R2; renamed), INT-087, open decision 3 | Later: 'Each legacy app gets a random password that works only through the adapter'; Server needs 'Reversible per-app secret store'; legacy clients send a password-derived token and salt in the URL. | R2 with the adapter, off by default. A person marks one app key as legacy; it may be sent in the password field and is checked against a keyed hash, so nothing reversible is stored. It works on home-network paths only, lasts at most 90 days, and alerts on first use from a new kind of network. Token-and-salt (MD5) stays refused unless the owner approves a written exception. | SEC-EXT-069, SEC-TM-070, SEC-NET-024, SEC-API-094 | yes |
| A-317 | features/integrations.md: INT-094, INT-097, Differentiator 2 | 'Off by default and rate-limited, behind the same authorisation layer' (port and transport unstated). | No listener until the owner enables it with a fresh check. Each adapter has its own port, ignores cookies, accepts only its own credentials and only over HTTPS, with no plain-HTTP exception on the home network. Turning adapters on changes nothing in the native API. Music Assistant connects to the HTTPS address. | SEC-EXT-051, SEC-EXT-052, SEC-EXT-053, SEC-EXT-066, SEC-NET-001 | yes |
| A-318 | features/integrations.md: INT-098 | 'maps Quick Connect onto Gunmetal pairing' (no approval rules) | The person types the app's code into Gunmetal's own app and sees the app, the device and whether it is on the home network. Remote Quick Connect is refused unless the owner allows remote device sign-in. | SEC-EXT-071, SEC-EXT-072, SEC-EXT-073 | no |
| A-319 | features/integrations.md: INT-100 | Seerr, Jellystat, Tracearr, Maintainerr, Bazarr and Home Assistant 'work unchanged'; 'Session and user-list routes under scopes'. | Tools work within what their scopes allow. User lists, other people's sessions and history, remote control and refresh are absent from the adapter, so history-dashboard tools see only their key owner's own data. | SEC-EXT-055, SEC-EXT-056, SEC-EXT-061, SEC-PRV-032, SEC-PRV-025 | yes |
| A-320 | features/integrations.md: INT-116 | 'Pulls from the other server on a schedule ... It writes back to the other server only if asked' | Pulls go to an owner-granted exact destination, the credential is an encrypted integration secret, and responses are treated as untrusted and can be removed as a batch. Write-back happens only for people who turn it on themselves. | SEC-PRV-033, SEC-EXT-050, SEC-API-079, SEC-API-081 | no |
| A-321 | features/integrations.md: INT-118 (renamed 'Lidarr refresh after an import') | 'Comes free with the OpenSubsonic adapter's startScan, limited by a refresh-only key to the music root' | Not through the adapter: its allowlist leaves out startScan, and adapter keys can only browse, play and report. Lidarr's custom-script connection calls the native refresh (INT-011) with a refresh-only key. | SEC-EXT-055, SEC-EXT-056, SEC-EXT-007 | no |
| A-322 | features/integrations.md: INT-119 (renamed 'Sonarr and Radarr refresh after an import'), INT-121 | A Jellyfin-style library-update endpoint for Sonarr's 'MediaBrowser' connector 'under a refresh-only token'; Bazarr refresh under a refresh-only token. | No refresh route on the Jellyfin adapter, which accepts no API keys. Sonarr and Radarr use a custom-script connection to the native refresh (INT-011), and Bazarr needs an upstream native connector. | SEC-EXT-055, SEC-EXT-007, SEC-EXT-070 | no |
| A-323 | features/integrations.md: INT-123 | 'Free playback-start webhooks with a payload preset for Bazarr' (for every play) | Sent only for people who opted in (INT-162), never for a private session. Gunmetal itself never looks up subtitles at play start. | SEC-EXT-045, SEC-PRV-015, SEC-PRV-033 | yes |
| A-324 | features/integrations.md: INT-125 | 'Gunmetal's own watchlist ... is readable by Seerr under a read scope' | Seerr reads a person's watchlist only with a key that person created (INT-027), never an admin's key. | SEC-PRV-022, SEC-EXT-011 | no |
| A-325 | features/integrations.md: INT-136 | 'Automations can stop a stream, with a message' via the API, for any session. | A key stops only its owner's own sessions. Stopping someone else's stream needs an admin's interactive session, and the message is shown only as plain text. | SEC-HIS-014, SEC-EXT-010 | no |
| A-326 | features/integrations.md: INT-144 | 'Each Gunmetal app session is a media player in Home Assistant'; 'Parity' | Behind on the household view, by design: a person's key shows and controls their own sessions. Others appear only if they opted in (INT-162), without titles unless they allow them, and sensors show counts. | SEC-PRV-025, SEC-PRV-032, SEC-EXT-045 | yes |
| A-327 | features/integrations.md: INT-155, INT-156 | OPDS, KOReader sync and podcast sync protocols with sign-in unstated (these protocols use passwords). | Per-app keys over HTTPS, never account passwords. KOReader's password-hash scheme follows the legacy-key limits of INT-088. | SEC-API-094, SEC-EXT-057, SEC-NET-001 | no |
| A-328 | features/integrations.md: INT-157 (superseded by A-568) | 'M3U, XMLTV and an HDHomeRun-style device, with authenticated per-device URLs' | Each player gets a revocable key in the URL path under the legacy URL-key rules: HTTPS, home-network paths only, 90 days, redacted from logs. Exported URLs point at the server's proxied streams, never the provider, and no discovery responder is opened. | SEC-API-094, SEC-EXT-069, SEC-TM-071, SEC-CLI-067, SEC-NET-001 | yes |
| A-329 | features/integrations.md: Open decision 5; Dependencies and risks (third-party terms) | 'register project apps where the terms allow per-user authorisation (Last.fm, ListenBrainz), accepting that a secret inside an open binary is not secret' | No project secret ships in any binary. The owner registers the server's own Last.fm API account and Trakt client ID, stored as encrypted integration secrets; each person links their own account. | SEC-TM-012, SEC-CLI-016, SEC-EXT-050 | yes |
| A-330 | features/integrations.md: INT-160 (new, Alerts for new keys and apps, R2) | missing | Key creation, scope changes, first use from a new network and revocation go to the person's security log. Creation and new-network use alert their devices with a one-step 'This wasn't me'. | SEC-EXT-017, SEC-HIS-063, SEC-OPS-033, SEC-OPS-034, SEC-IAM-097 | no |
| A-331 | features/integrations.md: INT-161 (new, Where my data goes, R2) | missing | A page lists every key, adapter app, per-user plugin and opted-in automation that receives the person's data, with the host and fields, generated from the enforced grants, and lets the person revoke or switch off each one. | SEC-PRV-027, SEC-IAM-104, SEC-EXT-014, SEC-EXT-039, SEC-EXT-045 | no |
| A-332 | features/integrations.md: INT-162 (new, Opt-in before household automations see my plays, R2) | missing | A per-person switch, off by default and never set by an admin, decides whether admin webhooks and Home Assistant get that person's playback events. Titles need a separate allowance, and private sessions emit nothing. | SEC-EXT-045, SEC-PRV-033, SEC-PRV-023, SEC-PRV-025 | yes |
| A-333 | features/later-media.md: LAT-034 Bookmarks with notes | Bookmarks "visible to third-party apps through OpenSubsonic's bookmark endpoints" | Bookmarks are private user-log events. They reach OpenSubsonic apps only through the person's own app key, and only once the adapter's endpoint allowlist and grant include bookmarks. Today SEC-EXT-056 does not list them. | SEC-EXT-055, SEC-EXT-056, SEC-PRV-022 | no |
| A-334 | features/later-media.md: LAT-045 Private podcast feed for a book (and later-media open decision 4) | "A per-book, read-only, revocable capability URL with an expiry... the one long-lived URL in this map" | Changed how it works. No secret goes in the URL. The feed and its episode files accept only a server-generated per-feed key, sent as an HTTP Basic password in the Authorization header over HTTPS. The key is read-only, expires, is listed and revocable (LAT-176), and the feed is off by default. Apps that can only hold a secret URL are not supported (unverified per app). Stays Later. | SEC-IAM-083, SEC-EXT-006, SEC-EXT-007, SEC-EXT-013, SEC-EXT-014, SEC-NET-001 | yes |
| A-335 | features/later-media.md: LAT-089 Podcasts through OpenSubsonic | "OpenSubsonic defines eight podcast endpoints", implying all of them are served | Reading and playing work through each person's app key. createPodcastChannel adds a request to the owner's host-approval queue instead of fetching. Deleting follows LAT-090's permissions. | SEC-API-080, SEC-EXT-055, SEC-EXT-054, SEC-EXT-056 | no |
| A-336 | features/later-media.md: LAT-141 OPDS catalogue, LAT-142 KOReader sync (LAT-088 also gets the adapter constraints) | Readers use "scoped, revocable per-device keys"; the UI has a "QR code to add a reader"; nothing says off by default or HTTPS only | These are adapters. They stay off until the owner enables them and run on their own listener, through the same authorisation layer and cross-user tests. Each reader gets a server-generated per-app key sent in the Authorization header over HTTPS, never in a URL or in the QR code. Readers that cannot do HTTPS are not supported. KOReader gets no account-creation endpoint and accepts progress only for books the key's owner can see. | SEC-IAM-084, SEC-EXT-051, SEC-EXT-054, SEC-EXT-055, SEC-EXT-006, SEC-EXT-066, SEC-NET-001 | yes |
| A-337 | features/later-media.md: LAT-143 Kobo sync (and later-media open decision 10) | "Only after the protocol and its legal position are checked (unverified)" | Changed how it works, with a path to No. Rivals appear to put a long-lived key inside the sync URL (unverified), which the baseline forbids. Kobo sync is built only if the device can send its key in a header; otherwise the row becomes No. The server never relays the device's traffic to the Kobo store, and KEPUB copies stay in cache. | SEC-EXT-006, SEC-IAM-084, SEC-EXT-055, SEC-TM-042 | yes |
| A-338 | features/later-media.md: New row LAT-176 Keys for reading and podcast apps (reference to ACC-129) | missing | Each OPDS reader, KOReader device, podcast sync app and private feed gets its own key: generated by the server, shown once, read-only to one protocol, sent in a header and never in a URL or QR code. Keys expire with a 14-day warning, are listed with last use, and stop working on the next request when revoked. First use from a new address appears in the person's security log. Later. | SEC-IAM-083, SEC-EXT-006, SEC-EXT-007, SEC-EXT-013, SEC-EXT-014, SEC-EXT-017 | no |
| A-339 | features/live-tv.md: LIV-140 | "Events on the server's event stream and webhooks" with no audience or payload rules | Events are filtered per recipient. Events about a person's own recordings go only to their own webhooks, or to admin webhooks they opted into; household recordings go to admin webhooks. Payloads carry only the listed fields, with no titles by default, and are signed. Private sessions emit nothing (LIV-178). | SEC-EXT-045, SEC-OPS-035, SEC-API-016, SEC-EXT-046 | no |
| A-340 | features/live-tv.md: LIV-171 | "M3U and XMLTV with per-device, revocable, live-only credentials, never an admin token" (Later) | Stays Later, but waits for an architecture record that defines a playback-only export credential the baseline allows, because the baseline refuses credentials in URL paths and queries, which is where IPTV players put them. The credential must be scoped to one person's channels, never an admin scope, listable and revocable, and served over HTTPS only. Open decision 13 updated. | SEC-API-004, SEC-EXT-006, SEC-HIS-041, SEC-EXT-010, SEC-NET-001 | yes |
| A-341 | ui/flows.md, ui/surfaces.md: flows.md F06 other releases; surfaces.md SUR-094, SUR-078, SUR-083 | R1 playlist write API with scoped tokens; token list and editor in R1 | API keys and the scripting surface in R2, never administrative, created only with step-up | SEC-IAM-083, SEC-EXT-008, SEC-EXT-010, SEC-EXT-012, SEC-EXT-013 | yes |
| A-342 | plan/api-needs.md: API-PL-06, API-PL-07, API-TOK-01, API-TOK-02, API-SCAN-02 (key part), server-side rule evaluation job | Scoped tokens, tool change feed, tool playlist API and server-side rule evaluation in R1 | R2 with API keys, never administrator scopes, intersection with owner's rights | SEC-IAM-083, SEC-EXT-008, SEC-EXT-010, SEC-EXT-011, SEC-EXT-012, SEC-EXT-013, SEC-API-020 | yes |
| A-343 | plan/api-needs.md: API-HOME-07 Follows and alerts | Optionally delivered to the person's own ntfy topic (an outbound purpose not in the egress inventory) | ntfy delivery is a webhook the person sets up under the webhook rules and egress gate | SEC-EXT-045, SEC-EXT-047, SEC-TM-075 | no |
| A-568 | features/integrations.md: INT-157 Live TV lineup export (supersedes A-328); ui/surfaces.md: SUR-124 Serves | R3: M3U, XMLTV and an HDHomeRun-style device, with a revocable per-player key carried in the URL path for 90 days on home-network paths (citing SEC-EXT-069) | A reference row: 'See LIV-171, which owns this feature', Release Later. No HDHomeRun-style device (LIV-172 is No) and no key in a URL path or query; the export credential is whatever the architecture record LIV-171 requires defines | SEC-API-004, SEC-NET-059, SEC-TM-004, SEC-NET-001 | no |
| A-569 | features/accounts.md: ACC-131 Quick Connect through the Jellyfin adapter (Release Later to R2); features/integrations.md: INT-098 | INT-098 (R2) 'maps Quick Connect onto Gunmetal pairing' while ACC-131, which owns Quick Connect through the adapter, was Later | ACC-131 moves to R2 with INT-098 and keeps its conditions (8-character code, unverified name, typed and matching code when remote, adapter token only, remote requests refused unless the owner allows them); INT-098 points to it. SEC-EXT-071 (R2) makes Quick Connect the adapter's default sign-in, so if Jellyfin apps cannot show an 8-character code, INT-098 waits with ACC-131 rather than signing apps in with passwords | SEC-EXT-071, SEC-EXT-072, SEC-IAM-060, SEC-EXT-070 | yes |

### T10 Browsers, devices and TVs

48 changes, 18 to confirm. Decision: D-25, D-26, D-30, D-31. What browsers, phones and TVs may keep, how they pair, and what shared screens show.

| ID | Where | Before | After | Requirements | Confirm |
|---|---|---|---|---|---|
| A-344 | features/music.md: MUS-183 Listening history by date | The log is the history; queries run on the device | Adds: in a browser, history is held in memory for the session, never written to browser storage, and sent by the server marked not to be cached | SEC-TM-058, SEC-PRV-019, SEC-PRV-020 | no |
| A-345 | features/music.md: MUS-208 Instant browsing from the synced library | Delta sync into the core's store on the device (WASM in the browser) | Delta sync is computed per person, so an item someone may no longer see arrives only as a removal. A browser keeps the store on disk only when marked personal at sign-in; a shared browser keeps it in memory; sign-out or a revoked session deletes it | SEC-API-015, SEC-CLI-009, SEC-CLI-010, SEC-IAM-017 | no |
| A-346 | features/library.md: LIB-018 Library change feed | The scan diff is the feed; browsing works offline on phones, TVs and browsers. | The feed is computed per person through the visibility predicate. When an item stops being visible, the feed carries only its ID as a removal. Cursors are opaque or MAC-protected. A browser marked shared keeps the library in memory only, so offline browsing needs a personal device. | SEC-API-015, SEC-API-025, SEC-CLI-010, SEC-CLI-020 | no |
| A-347 | features/discovery.md: DIS-002 Instant home (What the user gets; How Gunmetal does it better; Server needs) | Home opens without a spinner and still works with no connection, on every client including the R1 web client; the sync feed was not scoped per principal. | Offline only where the device may keep data: native apps (R2) keep the library and log; a browser marked personal keeps the library but never history; a shared browser keeps everything in memory only. Sync deltas are computed per principal, an item that becomes invisible is sent only as an ID removal, and the sync cursor is opaque or MAC-protected. | SEC-CLI-010, SEC-TM-058, SEC-PRV-019, SEC-API-015, SEC-API-025 | yes |
| A-348 | features/discovery.md: DIS-084 Search on the device | Results with no network wait, including on a plane; if the on-device build is too slow, the server ships a prebuilt index segment. | Offline search on native apps and personal browsers only; a shared browser keeps the index in memory; the index holds only what the profile may see; any prebuilt segment is built per profile, never one household index. | SEC-CLI-020, SEC-CLI-010, SEC-API-015 | yes |
| A-349 | features/discovery.md: DIS-169 Apple TV Top Shelf | Same mechanism as DIS-168 (no consent or profile rule stated). | Asked once at TV pairing, on by default only for a single-profile TV, never for restricted or PIN-protected profiles, and private-session plays never appear. | SEC-CLI-061, SEC-PRV-058 | no |
| A-350 | features/discovery.md: DIS-190 Your rows stay private on the shared TV (new row, R2) | missing | On a household TV, browsing and playing a profile stay one tap, but an adult's continue, recent, history and mix rows appear only after a PIN or phone approval; a PIN-protected profile's synced history is encrypted on the TV. | SEC-IAM-110, SEC-IAM-065, SEC-CLI-063, SEC-CLI-064 | yes |
| A-351 | features/clients.md: CLI-022 Library synced to the device | Delta sync into an on-device SQLite store, persisted in every browser | The server builds the copy per profile. In a browser marked shared it is kept only in memory; in a personal browser it is partitioned per account and deleted at sign-out | SEC-CLI-010, SEC-IAM-017, SEC-CLI-020 | no |
| A-352 | features/clients.md: CLI-032 Old clients keep working | A server update does not strand a TV app; the server keeps the previous version | The previous protocol version is kept for native apps, except versions the signed advisory feed lists as insecure or that are below the admin minimum: those get 'Update required'. Web tabs always reload to the server's own build | SEC-CLI-065, SEC-CLI-011 | no |
| A-353 | features/clients.md: CLI-047 An honest device capability report | Direct plays whenever it can | Direct play only for containers the core's parser accepted at scan time. Other containers are remuxed or transcoded unless an admin allows direct play per library. The device's report is untrusted input | SEC-CLI-049, SEC-HIS-033 | yes |
| A-354 | features/clients.md: CLI-084 Download films and episodes as they are | libmpv plays the original, so the server only reads the disk | Holds only for core-parsed containers. Other containers are offered as a remuxed download unless an admin allows direct play for that library | SEC-CLI-049 | yes |
| A-355 | features/clients.md: CLI-050 Profile picker on shared TVs | Other profiles' caches are still on the device, so restricted items are hidden rather than absent (ACC-030) | Per-profile partitions built by the server, so restricted items are absent, not hidden. The picker shows names and avatars only. PINs are checked by the server only. PIN-protected data is encrypted under a key the server releases. Adding an adult profile preselects 'Add a PIN'. The remaining risk (developer access to an unprotected profile) is stated | SEC-CLI-020, SEC-CLI-063, SEC-CLI-064, SEC-IAM-062, SEC-IAM-065, SEC-IAM-110 | yes |
| A-356 | features/clients.md: CLI-059 Choose where downloads go | R2: store downloads on an SD card or external storage | Moved to Later. R2 downloads stay in app-private internal storage; removable storage only once downloads there are encrypted in seekable authenticated chunks | SEC-CLI-035, SEC-CLI-072 | yes |
| A-357 | features/clients.md: CLI-100 Copy to a folder or drive | Parity with Emby Folder Sync: a server copy job, Admin > Devices | Kept in Later, but no server copy job. A Gunmetal app copies chosen downloads to the drive only in the encrypted, seekable format, playable under its offline grant | SEC-TM-042, SEC-CLI-072 | yes |
| A-358 | features/clients.md: CLI-074 Phone voice assistants | Assistant integration with donation defaults not stated | Sharing playlist names and plays with the assistant is off until the person turns it on for their profile. Private sessions are never shared | SEC-CLI-061, SEC-PRV-058 | no |
| A-359 | features/clients.md: CLI-156 Signing out leaves nothing behind (new, R1) | missing | Sign-out, account switch or revocation wipes the browser's storage and service worker. From R2 native apps offer 'Switch account' (data stays encrypted) and 'Remove account from this device'. A revoked device wipes itself on next contact | SEC-CLI-009, SEC-API-039, SEC-CLI-037, SEC-IAM-053 | no |
| A-360 | features/clients.md: CLI-158 A warning when a server is not the one you joined (new, R2) | missing | The identity key is pinned from the invite and the server signs a fresh challenge on every connection. A mismatch stops the app with no 'continue anyway'; a key change needs a statement signed by the old key | SEC-CLI-043, SEC-NET-060, SEC-IAM-051, SEC-CLI-042 | no |
| A-361 | features/clients.md: CLI-159 Outside links say where they go (new, R1) | missing | Only https and http links with a real host are links. Opening one shows a sheet naming the destination host, with no referrer. TV apps show a QR code instead | SEC-STD-015, SEC-API-047, SEC-CLI-002 | no |
| A-362 | features/accounts.md: ACC-019 Profile picker and instant switching | Switching opens another profile's synced data on the device, so it does not wait on the server for anything except the PIN check. | Each switch gets a new session token from the server, and the old token stops working. A PIN-protected profile's data stays encrypted on the household device until the server releases its key after the PIN check (ACC-132). | SEC-IAM-038, SEC-IAM-065 | no |
| A-363 | features/accounts.md: ACC-020 Profile PIN | The server rate-limits PIN attempts with the shared account-wide limiter (ACC-063). | Only the server checks the PIN, stored as a peppered Argon2id hash. Delays apply per (device, profile), up to 15 minutes and never a permanent lock. One alert goes out after 10 failures, the profile's owner can always get in from their own phone, and a PIN only unlocks switching. | SEC-IAM-062, SEC-IAM-063, SEC-API-056 | yes |
| A-364 | features/accounts.md: ACC-021 Household devices | A household TV with PINs on adult profiles. No location limit, no dormancy rule, and no rule on how it is enrolled. | Works only on the home network by default; if seen on a remote path it is suspended and the adults are alerted. It goes dormant after 30 days unused and can never be created by a remote or typed-code enrolment. Adding an adult profile preselects 'Add a PIN', and adult history stays hidden until unlocked. | SEC-IAM-109, SEC-IAM-110, SEC-IAM-059 | yes |
| A-365 | features/accounts.md: ACC-022 Guest mode on a shared device | A 'Guest' entry on the household picker, limited by a guest policy. | A household-owned 'Visitor' profile that records nothing. It is not a guest account, and guest accounts never appear on a household picker. | SEC-IAM-061 | no |
| A-366 | features/accounts.md: ACC-030 Every path obeys the restrictions | On a shared device, restricted items are hidden by the client, and a child with file access could read the adult's cache. Encryption is ACC-132 (Later). | Hiding in the client is never the control. In R1, a browser marked as shared keeps no library data at rest, and each account's caches are partitioned and deleted at sign-out. From R2, PIN-protected profile data on a household device is encrypted under a key the server releases. | SEC-TM-026, SEC-IAM-065, SEC-IAM-017, SEC-PRV-019 | no |
| A-367 | features/accounts.md: ACC-132 Per-profile local stores encrypted on shared devices | Release Later. | Release R2, shipping with households. The key is released after the PIN check or an approval from the adult's phone, and the device forgets it when it switches away. | SEC-IAM-065, SEC-IAM-110 | yes |
| A-368 | features/accounts.md: ACC-061 Sign in a TV with a QR code | The phone shows the TV's name and profile, and a short typed code is the fallback. | The TV's name is marked unverified and the screen says whether it is in this home. Approval needs a fresh biometric check. When the devices are not proven to be on the same network, the person types the 8-character code and confirms a matching code. A remote TV gets one profile at most, never household status, and never admin powers. | SEC-IAM-058, SEC-IAM-059, SEC-IAM-060, SEC-IAM-056 | no |
| A-369 | features/accounts.md: ACC-079 Session lifetimes | Lifetimes are a policy field. | Fixed maximums: 7 days idle and 30 days total, admin session 15 minutes idle and 1 hour total. The owner may shorten them but never lengthen them. A personal-or-shared question at sign-in puts shared browsers in a memory-only mode with a 30-minute idle timeout. | SEC-IAM-041, SEC-CLI-010, SEC-PRV-019 | yes |
| A-370 | features/admin.md: ADM-035 | Set up a headless server from a phone or TV app 'without HTTPS', via LAN discovery and a device key. | Phones and tablets only (a TV never claims a server); mDNS discovery, PAKE keyed by the setup code with server-key pinning, hardware-backed device key, iroh's encrypted channel with no plaintext path. | SEC-OPS-010, SEC-NET-061, SEC-HIS-053, SEC-IAM-049, SEC-IAM-059, SEC-CLI-024 | no |
| A-371 | features/admin.md: ADM-114 | Admin dashboard on phones and TVs; every native client gets it. | Renamed 'Admin dashboard on phones and tablets': admin needs a hardware-backed device key with per-action biometric; TVs and limited devices never get admin rights (alert banner only). | SEC-IAM-049, SEC-CLI-024, SEC-IAM-059 | no |
| A-372 | features/integrations.md: INT-014 | 'using a short-lived read-only token derived from the signed-in session' | Read-only GET requests sent with the session cookie and the Gunmetal-Request header, under the web client's CSP. No token is ever minted for page script. | SEC-TM-058, SEC-IAM-017, SEC-API-033, SEC-API-044 | no |
| A-373 | features/later-media.md: LAT-040 Casting books | "Signed URLs the receiver can refresh" | The receiver gets a capability URL for one item and one cast session, never an account or device token. The sender refreshes it, so a long book needs the sender in contact at least every 4 hours. Revoking the sender's session ends the cast. | SEC-CLI-071, SEC-NET-064, SEC-API-027, SEC-HIS-042 | no |
| A-374 | features/video.md: Intro paragraph; VID-001; VID-176 (What the user gets) | On a first-party client no film ever needs a transcode because of its container. Native clients direct play the untouched file as Kodi and Infuse do, and 'nearly any file plays as it is'. | Native players get the original only for containers the core parsed at scan (Matroska and MP4 in R2), and only through a stream callback for one item. Other containers (AVI, WMV, MPEG-TS and so on) go through the remux worker or the sandboxed transcode by default, unless an admin allows direct play for that library. The intro now limits its promise to containers the core parses. | SEC-CLI-049, SEC-CLI-047 | yes |
| A-375 | features/video.md: VID-002; VID-176 | Client and server reach the same verdict from one shared decision function, and 'the server's decision engine trusts what this player reports'. | The server re-runs the decision for every stream it issues and enforces the user's policy itself. The client's verdict is only a preview. Its capability report is validated, untrusted input that can choose among the paths the policy allows but never widen them, and the reasons sent to clients are typed codes with no paths. | SEC-CLI-015, SEC-TM-031, SEC-HIS-033, SEC-TM-040 | no |
| A-376 | features/video.md: VID-186 (new row, R2) | missing | Files outside the core's container list, with their cost and a per-library choice. The admin problem list shows the affected files per library with their count and cost, and offers a per-library switch to allow direct play. | SEC-CLI-049 | yes |
| A-377 | features/live-tv.md: LIV-074, LIV-117 | "Matched by external IDs from library metadata" across all libraries | Matched only against libraries the viewer, or the rule's owner, may read, so the badge never reveals what is in a library they cannot see. | SEC-TM-026, SEC-API-014, SEC-CLI-020 | no |
| A-378 | ui/flows.md, ui/surfaces.md: flows.md F03 step 3; surfaces.md SUR-070, SUR-077 | "Remember this browser" choice | One plain question 'Is this your own device?': own device 30 days/7 idle with storage; shared device memory-only, session cookie, 30 idle minutes | SEC-CLI-010, SEC-IAM-041 | yes |
| A-379 | ui/flows.md, ui/surfaces.md: flows.md F04; surfaces.md SUR-061, SUR-091 | Scanning the TV's QR code approves it wherever the TV is; household TVs unrestricted | Approval sheet with unverified name and In this home/Somewhere else; typed code plus matching code when not on the same local network; remote enrolment one profile at most, never household class, all adults alerted; household TVs home-network only and dormant after 30 days | SEC-IAM-055, SEC-IAM-056, SEC-IAM-057, SEC-IAM-058, SEC-IAM-059, SEC-IAM-060, SEC-IAM-109, SEC-STD-027 | yes |
| A-380 | ui/flows.md, ui/surfaces.md: flows.md F07 step 3; surfaces.md SUR-075, SUR-050 | Downloads to the SD card in R2 (CLI-059) | App-private internal storage only; removable storage Later and only encrypted | SEC-CLI-035, SEC-CLI-072 | yes |
| A-381 | ui/flows.md, ui/surfaces.md: flows.md F18 other releases | Encrypted per-profile stores on shared devices (ACC-132) Later | R2 for PIN-protected profiles on household devices | SEC-IAM-065 | no |
| A-382 | ui/flows.md, ui/surfaces.md: surfaces.md navigation (TV), admin group intro, SUR-005, SUR-083, SUR-084, SUR-085, SUR-089, SUR-100, SUR-124, open question 7 | Admin dashboard, sessions and activity in full on TV; simplified TV edit sheet (LIB-172); live TV wizard and lineup multi-select on TV (LIV-002, LIV-059) | No admin surface on TV; alerts as a banner on an admin's profile; QR hand-off to phone or computer | SEC-CLI-024, SEC-IAM-049 | no |
| A-383 | ui/flows.md, ui/surfaces.md: surfaces.md SUR-008 | On TV, approve a sensitive change on a signed-in phone | TVs never perform sensitive or admin actions; QR hand-off; step-up defined, never satisfied by OIDC alone | SEC-CLI-024, SEC-IAM-041, SEC-IAM-107 | no |
| A-384 | ui/flows.md, ui/surfaces.md: surfaces.md SUR-082; flows.md F01 other releases | A TV could complete setup through the device-key flow | A TV never claims; the owner claims from phone or computer; native claim uses a PAKE | SEC-CLI-024, SEC-IAM-059, SEC-OPS-010 | no |
| A-385 | ui/player.md, ui/design-language.md: player.md: Lock screen and system integrations > TV launcher rows and Background audio rows; Security rules for every surface > Shared screens and lock screens | Continue items go on Android TV's Play Next row, filled from the local log, for everyone with no opt-in. Media resumption is not mentioned. | OS-wide surfaces, including Play Next, Top Shelf, Spotlight and media resumption, are used only when the profile opts in. A TV asks once at pairing and defaults to on only for a single-profile TV. Restricted and PIN-protected profiles never publish, and private-session plays are never donated. | SEC-CLI-061, SEC-PRV-058 | no |
| A-386 | ui/player.md, ui/design-language.md: player.md: States > Server unreachable; Security rules > Shared screens and lock screens | Plays are kept on the device and uploaded on reconnect, with no distinction for shared computers. | On a computer marked as shared, pending plays, the queue copy and artwork are kept in memory only and never written to browser storage. The session ends after 30 minutes idle. | SEC-PRV-019, SEC-CLI-010 | no |
| A-387 | ui/player.md, ui/design-language.md: player.md: States > Restored; Music on a TV (new 'A household TV' bullet); Security rules > Shared screens | On opening, the bar always shows the restored queue and the 'Continue' prompt, on any device. | On a household device, an adult profile's restored queue, Continue prompts, history and private playlists stay hidden until a PIN or phone approval unlocks the profile for the session. Browsing and playing stay one tap. | SEC-IAM-110 | no |
| A-388 | ui/player.md, ui/design-language.md: player.md: States > Signed out | Playback ends and a card says 'This device was signed out' with a sign-in action. Nothing is said about clearing data first. | Playback stops at the next range request. The client first deletes the account's data and clears the media session (native apps also delete downloads), then shows the sign-in screen with one sentence and no titles. | SEC-CLI-009, SEC-CLI-037, SEC-IAM-043 | no |
| A-389 | plan/api-needs.md: API-SYNC-10 and API-SYNC-11 (new) | No erasure tombstones or purge-on-revocation capability | Tombstones naming only event IDs; client deletes the account's copy on revocation, sign-out or switch | SEC-PRV-052, SEC-CLI-009, SEC-IAM-017, SEC-IAM-053 | no |
| A-390 | plan/api-needs.md: Sync model: where the device keeps the copy | Copy kept across reloads wherever the browser allows (HTTPS or localhost) | Shared browsers keep catalogue, artwork and Activity in memory only; Activity persisted only on browsers marked personal; no tokens stored; service worker never caches capability URLs | SEC-CLI-010, SEC-PRV-019, SEC-IAM-017, SEC-API-029 | yes |
| A-571 | ui/surfaces.md: SUR-124 Live TV administration (Shows, Form factors); features/live-tv.md: LIV-002, LIV-059 (UI surfaces); features/library.md: LIB-172 (UI surfaces) | 'A guided set-up wizard in TV and touch layouts (LIV-002)'; the lineup editor 'by remote'; the edit sheet 'on web, phone and TV' | Phone, tablet and web on a personal device only; on TV, only an 'Open on your phone' QR hand-off, because a TV is a limited device and the server refuses admin routes from it | SEC-CLI-024, SEC-IAM-049 | no |

### T11 Releases moved to match the baseline release scope

14 changes, 11 to confirm. Decision: D-10. Releases moved so the maps match the baseline's release-scope table (desktop shell and rival importers Later; R1-tagged R2 rows proved by absence).

| ID | Where | Before | After | Requirements | Confirm |
|---|---|---|---|---|---|
| A-391 | features/music.md: Desktop shell: MUS-068, MUS-081, MUS-082, MUS-102 and the Releases convention | R2 includes the desktop shell; MUS-081, MUS-082 and MUS-102 were R2, and MUS-068 promised desktop gapless in R2 | The desktop shell is Later per the baseline's release-scope table: MUS-081, MUS-082 and MUS-102 move to Later; MUS-068 covers Android phones and TV in R2, and desktop follows the shell; the convention bullet says the map follows the baseline's scope | SEC-TM-074, SEC-CLI-069 | yes |
| A-392 | features/music.md: MUS-141 Import from Plex, Jellyfin and Navidrome (reference to ADM-036) | Release R2 | Release Later, because the baseline puts importers for rival databases in Later | SEC-STD-031, SEC-TM-074 | yes |
| A-393 | features/clients.md: CLI-014 Desktop app, with CLI-063, CLI-064, CLI-065, CLI-066 and open decisions 3 and 10 | Desktop shell, system media panels, global hotkeys, mini player and light shell in R2 | All moved to Later, because the security release scope puts the desktop shell in Later with its rules in SEC-CLI-069. Until then desktops use the web client. Linux is no longer promised for R2 | SEC-CLI-069, SEC-TM-074, SEC-CLI-050 | yes |
| A-394 | features/accounts.md: ACC-084 Re-invite people from your old server | R2. | Later, because importers for rival databases are Later in the release-scope table. A rival's database is opened read-only in a jailed worker. | SEC-TM-074, SEC-STD-031 | no |
| A-395 | features/accounts.md: Open decision 15 (R1 scope) | ACC-086 left the R1 floor because share links are unauthenticated; ACC-053 stays in R1 because password plus two-factor is the R1 default. | The full list of release moves forced by the baseline, now reflected in this file. The feature map README's R1 cut needs the same changes. | SEC-TM-074, SEC-API-097, SEC-IAM-025 | yes |
| A-396 | features/admin.md: ADM-014 | FreeBSD builds in R2; music works fully, transcoding unavailable. | Release Later: not before a FreeBSD worker sandbox profile (Capsicum plus limits) is specified and tested against the isolation table and the release-scope table lists the platform. | SEC-MED-024, SEC-MED-082, SEC-TM-074 | yes |
| A-397 | features/admin.md: ADM-036 to ADM-040, ADM-047, ADM-048, ADM-049 | Rival database importers, API import, users-as-invitations and side-by-side sync in R2. | Release Later (threat model release scope lists importers for rival databases as Later; SEC-STD-031 binds them 'when they ship (Later)'); database copies read only in the jailed hostile-SQLite worker; API import needs an owner grant for one host and an egress inventory row. | SEC-TM-074, SEC-STD-031, SEC-TM-048, SEC-TM-075, SEC-API-079 | no |
| A-398 | features/video.md: Release assumptions; VID-176; new open decision 17 | R2 native clients include the desktop shell, and VID-176 ships the libmpv player on Android, Android TV and the desktop shell in R2. | R2 clients are Android phones and tablets, Android TV and Google TV, and the web client. The desktop shell is Later, as the baseline's release-scope table and SEC-CLI-069 say. VID-176 is renamed 'libmpv player on Android and Android TV', and the shell gets the same player when it ships. Open decision 17 asks the owner to settle the disagreement with the feature map README, which lists the shell in R2. | SEC-TM-074, SEC-CLI-069, SEC-CLI-050 | yes |
| A-399 | ui/flows.md, ui/surfaces.md: flows.md F11; surfaces.md SUR-097 | Admin imports listening history for the household; rival-database importers in R2 | Each person imports only their own history; rival-database importers Later per the baseline's release scope, opened in a jailed worker when they come | SEC-PRV-022, SEC-PRV-025, SEC-STD-031, SEC-TM-074 | yes |
| A-400 | ui/player.md, ui/design-language.md: player.md: Releases and platforms (table and new paragraph), Surfaces table (mini player, system media controls, full-screen player), Mini player, Lock screen table (desktop media panels, global hotkeys), Keyboard shortcuts heading, device sheet 'This device' row, open question 11 | The desktop shell ships in R2, with its mini player window (CLI-065), desktop media panels (CLI-063, CLI-068) and global hotkeys (CLI-064). | The desktop shell and all three surfaces are Later. Bringing them to R2 means moving SEC-CLI-069 and the desktop player sandbox to R2 with them. | SEC-TM-074, SEC-CLI-069 | yes |
| A-401 | ui/player.md, ui/design-language.md: design-language.md: section 2 Apple Music table (CLI-068), section 13 R2 and Later lists, open question 10 | The desktop app (CLI-068) and desktop layouts are R2. | The desktop shell, its layouts and its mini player are Later, matching the release-scope table, unless SEC-CLI-069 moves to R2 with them. | SEC-TM-074, SEC-CLI-069 | yes |
| A-402 | plan/api-needs.md: API-SET-11 Migration imports | Plex, Jellyfin, Emby, Navidrome and iTunes imports in R2 | Rival database imports Later (release scope), opened read-only in a jailed worker; iTunes files R2; another person's history only via a pending import they accept | SEC-TM-074, SEC-STD-031, SEC-PRV-026 | no |
| A-403 | plan/work-packages.md: Coverage rows SEC-NET-054, SEC-OPS-040, SEC-PRV-034, SEC-PRV-035, SEC-PRV-036; WP-001, WP-048, WP-131; owner decision 37 | Five R1 requirements were unassigned because their surface (iroh, scrobbling) arrives only in R2, so the traceability check would fail R1. Their R1 Release values contradict SEC-TM-074 and the release scope. | R1 absence proofs: WP-001's deny.toml bans iroh and its port-mapper crates (SEC-NET-054, SEC-OPS-040). WP-131's 'routes that must not exist' suite has no linking or scrobbling route, and WP-048's closed purpose enum has no scrobbling purpose (SEC-PRV-034 to SEC-PRV-036). WP-221 and WP-226 keep the R2 behaviour. Owner decision 37 asks the baseline's owner to move these rows to R2; this plan does not edit docs/security. | SEC-NET-054, SEC-OPS-040, SEC-PRV-034, SEC-PRV-035, SEC-PRV-036, SEC-TM-074 | yes |
| A-589 | features/README.md: Releases table R2 and Later rows, open decisions 4 and 21; features/video.md: Scope note | R2 lists 'share links' and 'the desktop shell'; open decision 4 recommends 'Legacy password sign-in for older apps (INT-088) ... Later'; open decision 21 promises Linux in R2 | R2 lists video share links (off by default) and the Jellyfin music subset; the desktop shell is in the Later row; open decision 4 points to INT-088 as it reads (per-app legacy key, off by default, home-network paths, 90 days, token-and-salt refused) and to D-54 and D-55; open decision 21 says the shell is Later | SEC-TM-074, SEC-API-097, SEC-EXT-069 | yes |

### T12 Setup, backups, updates and operations

49 changes, 13 to confirm. Decision: D-38, D-59, D-60, D-61. Setup, startup pages, backups, updates, audit, alerts and diagnostics.

| ID | Where | Before | After | Requirements | Confirm |
|---|---|---|---|---|---|
| A-404 | features/clients.md: CLI-033 Diagnostics you can read first | Server needs: server log excerpt on request | Only the user's own request records, pseudonymised. A server diagnostic bundle is for admins only | SEC-PRV-046, SEC-OPS-030, SEC-OPS-027 | no |
| A-405 | features/clients.md: CLI-077 Remote push notifications, and open decision 6 | Needs a push relay, in practice one the project would run | No project-run relay holding device push tokens. Only a relay the owner runs, carrying an opaque event ID that wakes the app. Stays Later | SEC-HIS-061, SEC-PRV-056, SEC-OPS-036 | yes |
| A-406 | features/accounts.md: ACC-105 Connection status that tells the truth | The project publishes connection success rates, without saying where the data comes from. | The rates come only from the project's own test runs; servers report nothing. | SEC-TM-053, SEC-PRV-009 | yes |
| A-407 | features/accounts.md: ACC-122 Short-lived, session-bound stream URLs | A leaked stream link stops working within minutes. | The link is bound to the session and lasts the item's length plus 10 minutes, 4 hours at most. Revocation applies on the next request, and in-flight responses close within 5 seconds. | SEC-API-027, SEC-IAM-043 | no |
| A-408 | features/accounts.md: Open decision 7 (telemetry) | Offer opt-in crash and diagnostic reports that preview their contents. | None in R1 and R2 and no automatic crash reports. Diagnostic bundles are downloaded by the admin. | SEC-PRV-009, SEC-TM-053, SEC-PRV-046 | yes |
| A-409 | features/admin.md: ADM-140 | Custom server name on the sign-in page and a message or disclaimer under it. | Neither appears on the sign-in page (unauthenticated responses never reveal the display name); both appear on the invitation landing page and after sign-in; the message doubles as the owner's addendum to the invitee privacy notice. | SEC-NET-047, SEC-API-005, SEC-PRV-053 | yes |
| A-410 | features/admin.md: ADM-029 | Restore on the welcome screen is an upload or a file pick, then root checks and cache rebuild. | Restore at setup needs the setup code, verifies the signature (foreign keys need a typed fingerprint, other machines need the recovery kit), parses under limits, then rotates keys, ends sessions, re-applies later deletions and starts restored devices and admins suspended. | SEC-OPS-008, SEC-OPS-043, SEC-OPS-044, SEC-STD-031, SEC-PRV-049 | no |
| A-411 | features/admin.md: ADM-032 | Startup page shows each step, the snapshot location, config errors and an estimate. | Unauthenticated visitors see only the state and an estimate; steps, snapshot location and config errors go to the console, the log and a host status command. | SEC-OPS-050, SEC-NET-047, SEC-TM-040 | no |
| A-412 | features/admin.md: ADM-006 | Refuse to run as root by default, with a documented override flag. | Refuse to run as root or with any capability, no override; templates set a non-root user. Feature renamed 'Refuse to run as root'. | SEC-OPS-053, SEC-TM-041 | yes |
| A-413 | features/admin.md: ADM-009 | Settings export: secrets are never exported in clear; import applies ticked fields. | Secrets and host-only settings are never exported at all; imports pass live validators and any loosened security setting needs the owner with a fresh passkey check. | SEC-TM-049, SEC-OPS-041, SEC-OPS-044, SEC-IAM-075 | no |
| A-414 | features/admin.md: ADM-056 | The irreplaceable part is small, so many snapshots can be kept. | Snapshots are integrity-checked (migration aborts if either step fails), protected like live data, and kept until 7 days after the next successful start. | SEC-OPS-048, SEC-PRV-005 | no |
| A-415 | features/admin.md: ADM-059 | An older binary throws away a newer cache and rebuilds from files plus log, skipping event types it does not know. | Cache is rebuilt, but if durable stores hold anything unreadable it refuses to start and names the pre-upgrade snapshot; only event types marked safe to skip are skipped; never drops identity, audit or security settings. | SEC-OPS-051, SEC-OPS-048 | no |
| A-416 | features/admin.md: ADM-064 | Opt-in automatic updates with an updater doing an atomic binary swap and health-check rollback. | No self-updater: automatic updates only through the OS package manager or a container update tool; Gunmetal provides the migration check, snapshot and a post-start health result. | SEC-OPS-046, SEC-HIS-056 | yes |
| A-417 | features/admin.md: ADM-065 | Backups hold the user log, settings, identities and server keys; daily with long retention. | Backups hold identity store, settings, user log, audit log with signed checkpoint and wrapped secrets; encrypted and signed; kept 14 days by default (configurable); never under a web path or media root. | SEC-OPS-041, SEC-OPS-042, SEC-PRV-041, SEC-PRV-051, SEC-OPS-045 | no |
| A-418 | features/admin.md: ADM-067 | An included cache snapshot is used on restore if its version matches. | It is read as untrusted input, read-only in the jailed SQLite worker, and its rows written into a fresh cache. | SEC-STD-031 | no |
| A-419 | features/admin.md: ADM-068 | Encrypted backups in R2, for archives leaving the machine, with an admin-chosen passphrase or recovery key; local archives optionally. | Release R1: every archive, local or exported, is age v1 encrypted to the server's backup key and the owner's recovery key; no unencrypted code path; passphrase not needed (Argon2id if ever offered). | SEC-OPS-042, SEC-PRV-039, SEC-TM-052, SEC-IAM-105, SEC-STD-024 | no |
| A-420 | features/admin.md: ADM-069, ADM-070 | Admin download and upload endpoints; restore from the UI by an admin. | Download is owner-only with a fresh passkey check, audited and alerted, via a single-use session-bound link; upload checks type, size and signature; restore is owner-only with fresh check, then key rotation, session invalidation, held less-strict settings and suspended restored devices. | SEC-OPS-045, SEC-TM-017, SEC-IAM-041, SEC-OPS-043, SEC-OPS-044 | no |
| A-421 | features/admin.md: ADM-098 | Sets an operating-system wake timer before the next recording. | Wake requested only through an unprivileged OS interface; the server never gains a capability for it; reported unavailable where none exists. | SEC-OPS-053, SEC-OPS-056 | no |
| A-422 | features/admin.md: ADM-110 | Append-only activity log; clearing it is itself recorded. | Hash-chained with signed checkpoints; nobody can clear it; records leave only by retention prunes with signed checkpoints; audit-capability holders see others' addresses shortened; each person sees their own records in full. | SEC-OPS-020, SEC-OPS-023, SEC-OPS-026, SEC-OPS-027, SEC-IAM-097 | no |
| A-423 | features/admin.md: ADM-113 | Server-rendered emergency page to check status, read logs, take a backup or restart (no authentication stated). | Needs an admin session from its own passkey sign-in, same CSP with no inline script, refused on internet-facing paths unless remote administration is on, nothing shown before sign-in, backup download owner-only, addresses shortened in log lines. | SEC-TM-004, SEC-API-044, SEC-NET-045, SEC-OPS-045, SEC-OPS-050 | no |
| A-424 | features/admin.md: ADM-117 | Alert inbox with acknowledge and snooze (any alert). | Only non-critical alerts can be snoozed; critical ones cannot, and a snooze never hides a different device or target. | SEC-OPS-034 | no |
| A-425 | features/admin.md: ADM-124 | Diagnostic bundle where the admin can mask titles, usernames and paths. | Built from a field allowlist; never the database, backups or secrets; paths, titles, usernames and IPs always pseudonymised; full preview. | SEC-PRV-046, SEC-OPS-030 | no |
| A-426 | features/admin.md: ADM-128 | Separate unauthenticated liveness and readiness answers. | Public endpoint answers liveness only; readiness only on loopback (reachable by in-container health checks). | SEC-NET-046, SEC-API-005 | no |
| A-427 | features/admin.md: ADM-131 | Opt-in crash report sending (Later) to a project-side receiver; titles and usernames excluded unless ticked. | Release No: a project crash receiver would hold data about people's servers and automatic crash reports are banned; crash records go into the pseudonymised diagnostic bundle (ADM-124) the admin can attach themselves. | SEC-PRV-009, SEC-TM-053, SEC-HIS-061 | yes |
| A-428 | features/admin.md: ADM-145 (new) | missing | Audit log verification and an off-server anchor (R1): audit verify command and button, verification against backups, web-client checkpoint anchor, checkpoint export, critical alert on failure. | SEC-OPS-023, SEC-OPS-024, SEC-OPS-075, SEC-IAM-094 | no |
| A-429 | features/admin.md: ADM-147 (new) | missing | Configuration changes made outside the server are audited with old and new values and alerted when less strict (R1). | SEC-OPS-031, SEC-OPS-049 | no |
| A-430 | features/admin.md: ADM-148 (new) | missing | Compromise runbook (R1), executed end to end in CI, linked from security alerts and advisories. | SEC-OPS-072, SEC-OPS-067 | no |
| A-431 | features/integrations.md: INT-001 | 'spec served by the server and the docs site' (no sign-in stated) | The server serves the spec only to signed-in users, because it names every route and the version. The docs site publishes each release's spec. | SEC-API-005, SEC-API-002 | no |
| A-432 | features/integrations.md: Open decision 11 | 'Treat inbound webhooks as an optional extra.' | Do not accept inbound Plex webhooks: the credential would have to sit in the URL, and inbound webhooks need their own architecture record. | SEC-API-004, SEC-EXT-076 | no |
| A-433 | ui/flows.md, ui/surfaces.md: flows.md F01 step 2 and failure paths | Refuses to run as root unless overridden | Refuses to run as root or with any capability, with no override | SEC-OPS-053 | yes |
| A-434 | ui/flows.md, ui/surfaces.md: flows.md F01 steps 3 and 5, failure paths, gap G1 | Single-use code with an expiry; G1 recommended printing a fresh code on every restart and binding the claim to the browser session | 128-bit code lasting 24 hours that survives restarts, reissued by `gunmetal claim-code` after expiry; claim, owner and passkey in one transaction so a half-finished setup commits nothing; wrong guesses delayed per source and never use up the code | SEC-IAM-007, SEC-IAM-008, SEC-IAM-009, SEC-OPS-004, SEC-OPS-005 | no |
| A-435 | ui/flows.md, ui/surfaces.md: flows.md F01 step 10, F03 step 1; surfaces.md SUR-070, SUR-103 | Sign-in page shows the server's name and sign-in message | Generic sign-in page with no name, message or version; name shown after sign-in, as a remembered name, and on invitation pages | SEC-NET-047, SEC-API-005 | yes |
| A-436 | ui/flows.md, ui/surfaces.md: flows.md F13 step 7 | An older binary rebuilds from the files plus a newer log, skipping unknown event types | An older binary may rebuild the cache but refuses newer durable state and points to the pre-upgrade snapshot; never discards identity, audit or security config | SEC-OPS-051, SEC-OPS-048 | no |
| A-437 | ui/flows.md, ui/surfaces.md: surfaces.md SUR-080 | Unauthenticated startup page shows configuration errors, the snapshot location and the root error text | State and progress only; details on console, log and doctor | SEC-OPS-050, SEC-NET-047 | no |
| A-438 | ui/player.md, ui/design-language.md: design-language.md: section 11 states table ('Server starting or migrating') | The startup page shows each step, the snapshot location and an estimate, with no access rule, so anyone can see them. | Visitors who are not signed in see only that the server is starting and an estimate. Steps, the snapshot location and configuration errors appear on the host console and to the owner after sign-in. | SEC-OPS-050 | no |
| A-439 | ui/player.md, ui/design-language.md: design-language.md: section 11 states table ('Client will not load' emergency page) and the server-rendered pages paragraph | A minimal server-rendered page offers status, logs, backup and restart, with no access rule stated. | The page requires an admin session and shows the logs the person's role may read. Backup download is owner-only and needs a fresh fingerprint or face check. Everyone else gets the standard refusal. Server-rendered pages carry no inline script. | SEC-IAM-041, SEC-OPS-045, SEC-OPS-027, SEC-OPS-050, SEC-HIS-028 | no |
| A-440 | plan/api-needs.md: Rules that apply to every capability (unauthenticated routes) | Every route needs sign-in except the setup, sign-in, invite landing, startup, emergency and health routes | A ten-item list of the only unauthenticated endpoints (web assets, GET /api/v1/server, /healthz, setup while unclaimed, sign-in ceremonies incl. recovery redemption, token refresh, invitation landing, pairing request/poll, share-link landing, capability routes); startup is a static non-route page; emergency needs the admin session | SEC-API-002, SEC-API-003, SEC-TM-004, SEC-OPS-050, SEC-IAM-067 | no |
| A-441 | plan/api-needs.md: API-SYS-01 Health check | Liveness and readiness | Liveness only, no version | SEC-NET-046 | no |
| A-442 | plan/api-needs.md: API-SYS-06 Public sign-in facts | Server name and sign-in message shown before sign-in | Only protocol range, instance ID, claimed flag and sign-in methods before sign-in; name shown after sign-in or on the invitation landing page | SEC-API-005, SEC-NET-047 | yes |
| A-443 | plan/api-needs.md: API-SYS-07 Startup status page | Unauthenticated server-rendered page with steps, estimates, snapshot location and refusals such as running as root | One static 'starting' page with no detail; detail on host console, doctor and to admins; refusals stop the server with a console message | SEC-OPS-050, SEC-OPS-053, SEC-OPS-051 | yes |
| A-444 | plan/api-needs.md: API-SYS-08 Emergency page | Unauthenticated page with status, log lines, a backup and a restart | Admin session required; backup download needs the owner with fresh user verification | SEC-IAM-041, SEC-OPS-045, SEC-NET-045 | no |
| A-445 | plan/api-needs.md: API-DEV-03 Diagnostics from a device | Optional server log excerpt attached to any person's report | Log excerpt only when the sender is an admin, masked like a diagnostic bundle | SEC-PRV-046, SEC-OPS-030 | no |
| A-446 | plan/api-needs.md: API-DEV-05 New-device notice | R2 | R1, in-app over the event channel or next sync, with 'This wasn't me' | SEC-IAM-098, SEC-OPS-032, SEC-OPS-033 | no |
| A-447 | plan/api-needs.md: API-SYNC-03 and the sync model (grant changes) | Proposal: resend a snapshot of the affected library when grants shrink | Identifier-only removals (a withdrawn library is one removal naming the library), device purges everything from it; grant change effective on next request on every path | SEC-API-015, SEC-IAM-076, SEC-API-017 | no |
| A-448 | plan/api-needs.md: API-STR-05 Artwork bytes and Flags item 7 | Proposal: first-party web client fetches artwork at content-addressed paths authorised by the session cookie | Proposal withdrawn; capability URLs only, 1-hour lifetime aligned to a time bucket; client keeps images by image ID | SEC-API-026, SEC-API-027, SEC-API-029 | yes |
| A-449 | plan/api-needs.md: API-SET-04 Backups | Admin list, back up, download, upload, restore and export | Download, upload, restore and full export owner-only with fresh-uv; restore verifies signature, opens in a jailed worker, re-applies deletions, rotates keys and ends sessions | SEC-OPS-043, SEC-OPS-044, SEC-OPS-045, SEC-STD-031, SEC-PRV-049 | no |
| A-450 | plan/api-needs.md: API-SET-07 Alerts and logs | Alert rules with free destinations the owner chose, R1 | In-app alerts only in R1 (security alerts never switchable off, daily summary, 'This wasn't me'); outbound destinations R2 opt-in through egress | SEC-OPS-032, SEC-OPS-033, SEC-OPS-034, SEC-OPS-035 | yes |
| A-451 | plan/api-needs.md: API-SET-13 Rotate server secrets (new) | No secret-rotation capability | Owner rotates every secret in one action, fresh-uv or host; sessions and signed URLs invalidated; R1 | SEC-OPS-018, SEC-OPS-015 | no |
| A-452 | plan/api-needs.md: Background jobs: security jobs | Only generic expiry sweeps listed | Added signing-key rotation, audit checkpoints, retention purge, erasure, account deletion, startup security checks, exposure detection, certificates and CT, debug-level timer, worker quarantine, derivative cache bound; R2 household-device dormancy and API key expiry; advisory check runs only when the owner answered yes | SEC-OPS-015, SEC-OPS-023, SEC-PRV-005, SEC-OPS-026, SEC-PRV-049, SEC-IAM-103, SEC-OPS-031, SEC-OPS-053, SEC-NET-027, SEC-NET-004, SEC-NET-069, SEC-OPS-029, SEC-MED-019, SEC-MED-048, SEC-OPS-047 | no |

### T13 Parsers, files, paths and images

31 changes, 10 to confirm. Decision: D-03, D-47. Parser limits, file and path rules, XML, archives, images and outside links.

| ID | Where | Before | After | Requirements | Confirm |
|---|---|---|---|---|---|
| A-453 | features/music.md: MUS-232 Album booklet and liner notes from local files | Read-only display of PDF and image files found in the album folder | Image pages are shown only as server-made derivatives, never the original files; PDFs go to the device as downloads under a sandbox policy and are never rendered by the server or on the application origin | SEC-MED-046, SEC-MED-059, SEC-API-099 | no |
| A-454 | features/library.md: LIB-148 Animated covers | Later, parity with Navidrome; an animated artwork derivative. | Later, and only as a short video sidecar indexed and served through the normal media path. Artwork decoding stays first-frame only, so no animated image is decoded or re-encoded as artwork. The way it works changed; the release did not. | SEC-MED-044, SEC-MED-012, SEC-TM-035 | yes |
| A-455 | features/library.md: LIB-005 Add a library at first run | Admin-only server folder browser; scan start. | The folder browser is admin-only, lists directories only and stays inside the configured browse roots, under a fresh passkey check (met by the passkey just created). A filesystem root, system directory or data directory is refused, and the picker says why. Each choice writes an audit record. | SEC-API-022, SEC-IAM-041, SEC-TM-017, SEC-MED-037, SEC-OPS-020 | no |
| A-456 | features/library.md: LIB-059 Every raw tag kept | The scan stores all raw tag frames per file. | Every tag field is stored as decoded, length-capped text within the limits table. Binary frames such as pictures are recorded by type and size only. | SEC-MED-006, SEC-MED-013, SEC-TM-031 | no |
| A-457 | features/library.md: LIB-091 Sidecar subtitles | .srt and .ass files beside a video or in a subfolder attach. | Only regular files in the video's own folder, matched by base name, attach. Subfolders such as `Subs` are not searched; that capability is dropped. | SEC-MED-053 | no |
| A-458 | features/library.md: LIB-192 Playlist files in music folders | Uses the MUS-140 parser and matcher; paths resolve through relative paths and identity. | Entries resolve only to items already indexed in the same library, by normalised relative path. URLs, paths outside the library, `..` escapes and artwork directives are dropped, never fetched or opened, and listed in Library health. Each person sees only the entries they may access. | SEC-MED-050, SEC-MED-016, SEC-MED-051, SEC-HIS-018 | no |
| A-459 | features/library.md: LIB-193 Damaged and unreadable files | missing: no report of skipped parts in files that stay playable | Each skipped part (artwork, lyrics, one tag) of a playable file is listed with a plain-language reason. | SEC-MED-017 | no |
| A-460 | features/library.md: LIB-204 Skipped links and approved link targets (new row, R1) | missing | Links leading outside a library are skipped and listed, grouped by target. An admin can allow a target folder for that library in one step, under a fresh passkey check; system and data directories are refused. | SEC-MED-034, SEC-TM-043, SEC-IAM-041, SEC-MED-037 | no |
| A-461 | features/library.md: LIB-207 Unresponsive storage pauses one folder (new row, R1) | missing | A hung network root stalls only its own scans and streams; after two lost workers its scan pauses with a 'storage not responding' status. | SEC-MED-041, SEC-TM-068 | no |
| A-462 | features/discovery.md: DIS-122 Limits, order and refresh | Seeded randomness in the core, so a list reproduces exactly on every device. | A stored random seed is hashed with each item's ID by an allow-listed hash to give the order, so lists reproduce without a seeded generator, which the build bans. | SEC-STD-022, SEC-STD-019 | no |
| A-463 | features/accounts.md: ACC-123 Secrets never in URLs or logs | Tokens travel only in headers or cookies. | Session tokens travel only in the cookie or the Authorization header, and credential names in a query string are refused. Media capabilities sit in the URL path and are redacted from logs. Invite and share secrets travel in the fragment. | SEC-API-026, SEC-NET-036, SEC-EXT-006 | no |
| A-464 | features/accounts.md: ACC-125 Uploads cannot reach the filesystem by name | SVG is never rasterised in the server process. | SVG is refused outright. | SEC-API-086, SEC-TM-035 | no |
| A-465 | features/integrations.md: INT-011 (How) | 'The token is scoped to named roots' and the tool says 'this folder changed'; Server needs 'Path validation against roots'. | The server matches the named folder against folders it already indexed under the key's roots and rescans through its own directory handles, so no path from the request is ever opened. Repeated requests join the running job. | SEC-HIS-015, SEC-TM-043, SEC-API-064 | no |
| A-466 | features/later-media.md: XML rows LAT-021, LAT-063, LAT-096, LAT-119, LAT-145, LAT-167, LAT-174, plus the 'XML everywhere' risk | Hardened XML that "refuses external entities and entity expansion" | One core XML parser that rejects any document containing a DOCTYPE and limits size, depth and element count. URLs inside the files (NFO thumbnails, OPF links) are never fetched. OPML, SMIL and CBL are added to the list of XML formats. | SEC-MED-056, SEC-HIS-034, SEC-TM-038, SEC-MED-016 | no |
| A-467 | features/later-media.md: LAT-134 Download originals | "The original is what Gunmetal serves anyway, behind a signed URL" | The original is served only as a download: an attachment with a server-chosen name and a sandbox policy, behind a short-lived capability URL, never shown inline. Guests and share links get it only where the owner allows downloads. | SEC-MED-059, SEC-API-051, SEC-MED-046, SEC-API-026, SEC-IAM-080 | yes |
| A-468 | features/later-media.md: LAT-150 Web EPUB reader, LAT-151 Phone ebook reader | "engine choice is open (likely WebView-based, unverified)" | Book pages render only on a separate sandbox origin that receives no cookies and runs no book script, or from a safe model the core makes. Never on the application origin, and never in a WebView that holds the app's session. Embedded fonts are not delivered by default. | SEC-API-099, SEC-API-051, SEC-TM-036, SEC-MED-057, SEC-MED-054 | no |
| A-469 | features/later-media.md: LAT-160 PDFs open on your device | "Original bytes are sent; the server never renders PDFs" (implying inline opening) | Original bytes are sent only as a download (an attachment with a sandbox policy) for the device's own viewer, never shown on the application origin. | SEC-API-051, SEC-API-099, SEC-MED-059 | no |
| A-470 | features/later-media.md: LAT-166 Page streaming, LAT-170 Covers, LAT-172 Webtoon scroll, and the Comics intro | "Pages are sent as stored bytes with no decoding, matching 'send the original'"; "The first page is sent as is when it is already JPEG or PNG" | Every page and cover is decoded by the memory-safe decoder and re-encoded at fixed sizes before any client sees it, then cached by content hash. This is a CPU cost the original design avoided. | SEC-TM-035, SEC-MED-046, SEC-CLI-005, SEC-HIS-030, SEC-MED-047 | yes |
| A-471 | features/video.md: VID-020 | Native clients could hand the disc structure (ISO, BDMV, VIDEO_TS) to libmpv. | The core would parse the disc structure under its budgets, and the remux worker would serve the main title as one stream. libmpv is never handed a disc image, folder or playlist, and its disc support is not built. The row stays Later. | SEC-CLI-047, SEC-CLI-048, SEC-TM-043 | no |
| A-472 | features/video.md: VID-063 | Native clients 'load [external audio files] directly'. | The scanner pairs audio sidecars that sit in the video's own directory. Native clients play each one as a second opaque stream through the player's stream callback, and libmpv's own sidecar search stays off. Browsers get the sidecars muxed in by the remuxer. | SEC-CLI-047, SEC-CLI-048, SEC-MED-012 | no |
| A-473 | features/video.md: VID-069; new open decision 16 | 'Styled ASS/SSA exactly as authored', using the fonts shipped in the file. MKV font attachments were indexed at scan and served by signed URL, and the original ASS packets went untouched to a WebAssembly libass. | Renamed 'Styled ASS/SSA without burn-in'. The core parses every embedded ASS track under the subtitle limits, and clients receive a re-serialisation with a fixed allowlist of styles and override tags, never the original bytes. Browsers run the WebAssembly libass in a web worker. Clients use a bundled, pinned font set with libmpv's embedded-font loading off, and embedded fonts are delivered only when a library opts in and has them re-serialised (VID-188). The row says plainly that some typesetting looks plainer. Open decision 16 asks whether the allowlisted ASS re-serialisation satisfies SEC-HIS-039 and SEC-API-089. | SEC-MED-054, SEC-MED-052, SEC-HIS-039, SEC-CLI-053, SEC-TM-062 | yes |
| A-474 | features/video.md: VID-072 | Sidecars beside the video 'or in a subtitles folder' were picked up ('parity plus subfolders'), and .ass sidecars rendered with their styling. | Only sidecars in the video's own directory are discovered, matched by base name; subfolders are not searched unless SEC-MED-053 changes. Sidecars are served as the core's WebVTT re-serialisation, so an .ass sidecar keeps its text and timing but not its full styling until open decision 16 is settled. | SEC-MED-053, SEC-API-089, SEC-HIS-039 | yes |
| A-475 | features/video.md: VID-188 (new row, R2) | missing | Embedded subtitle fonts can be turned on per library and are off by default. When on, each font is subset and rewritten by a memory-safe parser in the parse worker before delivery, and other attachments are never delivered. | SEC-MED-054, SEC-TM-062 | yes |
| A-476 | features/live-tv.md: LIV-003, LIV-035, LIV-015 | "M3U playlist from a file or URL"; "XMLTV from a file or URL"; LIV-015: "file sources are limited to declared folders" | "M3U playlist from an upload or URL" and "XMLTV from an upload or URL": sources are http or https URLs or files uploaded through the admin screen (upload route with type and size caps). The server never reads a source from a path; file paths and file: URLs are refused. | SEC-HIS-025, SEC-API-082, SEC-API-085, SEC-HIS-015 | no |
| A-477 | features/live-tv.md: LIV-035 | "A streaming Rust parser" (silent on DOCTYPE; many real XMLTV files begin with a DOCTYPE line) | DTD processing and entity expansion are off. Following the baseline, a guide that carries a DOCTYPE is refused with a clear reason. New open decision 16 asks whether a DOCTYPE with no internal subset may be skipped unread. | SEC-MED-056, SEC-HIS-034, SEC-TM-038 | yes |
| A-478 | features/live-tv.md: LIV-036 | "gz, xz and zip guides download faster"; "Parity" | gz and xz only, decompressed through the core's one streaming helper with an output cap. Zip is an archive, so zip guides are refused with a reason until an architecture record allows archive extraction. | SEC-HIS-019, SEC-MED-009 | no |
| A-479 | features/live-tv.md: LIV-138 | "Logic-less templates that produce names safe on every filesystem" for the stored recording files | Recordings are stored under server-generated names. Templates name a recording only when it is downloaded or exported, with every character escaped for the target filesystem. | SEC-HIS-015, SEC-MED-039 | no |
| A-480 | features/live-tv.md: LIV-162, LIV-163, LIV-017 | Rights listed with no defaults; channel profiles with no rule for channels that appear in a refresh; manual channels visible like any other | New users start with no Live TV management or scheduling rights. Adding sources is admin-only, and a change of rights applies on the next request. A channel that appears in a refresh, or is added by URL, joins no profile until an admin adds it. | SEC-HIS-013, SEC-HIS-025, SEC-TM-024, SEC-IAM-076 | no |
| A-481 | ui/flows.md, ui/surfaces.md: flows.md F17 step 4 | Serves the fonts attached to the file | No embedded fonts by default; per-library opt-in with memory-safe rewriting | SEC-MED-054 | yes |
| A-482 | ui/player.md, ui/design-language.md: player.md: Video > Audio and subtitles > Style; needs table (stream index row); open question 12 | The stream index carries font attachments for the player, implying embedded fonts are delivered by default. | The player uses a bundled, pinned subtitle font set by default. Embedded fonts are delivered only for libraries the owner opted in, after the parse worker rewrites them. | SEC-MED-054 | yes |
| A-483 | plan/api-needs.md: API-STR-09 Subtitles and fonts | Attached fonts delivered with subtitles | Embedded fonts off by default; per-library opt-in with re-serialisation | SEC-MED-054, SEC-API-089 | yes |

### T14 The build plan: packages, waves and doors

44 changes, 18 to confirm. Decision: D-01, D-02, D-04. Changes inside the work-package plan: package scope, waves, and one door per risk.

| ID | Where | Before | After | Requirements | Confirm |
|---|---|---|---|---|---|
| A-484 | plan/work-packages.md: WP-089 (server facts and discovery) | Unauthenticated discovery reveals the version and sign-in methods; old clients told the version they need | GET /api/v1/server returns only protocol range, instance ID, claimed flag and sign-in methods; no software version, server name or Server header to unauthenticated callers; health returns liveness only | SEC-API-005, SEC-NET-047, SEC-OPS-050, SEC-NET-046 | no |
| A-485 | plan/work-packages.md: WP-043 (server skeleton) tests | A probed UID of 0 refused unless the documented override | Root or any effective/permitted capability refused with no override | SEC-OPS-053, SEC-TM-041 | yes |
| A-486 | plan/work-packages.md: WP-044 (HTTP foundation) | Cookie requests without the Gunmetal-Request header refused for mutating routes only | Every cookie-authenticated request, whatever its method, needs Gunmetal-Request: 1, checked with 403 before authentication | SEC-API-033 | no |
| A-487 | plan/work-packages.md: WP-023 (HTTP header parsers) | A Range header naming several ranges (bytes=0-1,5-9) serves the full representation | Two or more ranges answer 416; a malformed header still means the full representation; recorded as a baseline inconsistency in owner decision 36 | SEC-API-031, SEC-NET-050, SEC-MED-060 | no |
| A-488 | plan/work-packages.md: WP-021 (lyrics parser) | Limits tested at 20,001 lines and 4 KiB lines (SEC-MED-049 values) | Stricter of the two baseline rows applied: 256 KiB, 10,000 lines, 4 KiB per line | SEC-API-090, SEC-MED-049 | no |
| A-489 | plan/work-packages.md: WP-079 (worker artwork job) | Dimensions checked from header with image::Limits; test refused only a 20,000 x 20,000 PNG | Stricter limits applied: 8,192 px per side, 40 MP, 32 MiB encoded, max_alloc 256 MiB; formats limited to JPEG, PNG, WebP, GIF first frame; metadata stripped | SEC-API-086, SEC-MED-045, SEC-MED-044, SEC-MED-046 | no |
| A-490 | plan/work-packages.md: WP-094 (users and invitations) | Hand over ownership by adding an admin and removing yourself | Ownership moves only by a transfer both parties confirm with fresh user verification; exactly one owner at all times; invitations default to 7 days, 1 use, secret in the fragment, member invitations pending until the inviter confirms a matching code | SEC-IAM-003, SEC-IAM-078, SEC-IAM-079, SEC-API-096 | no |
| A-491 | plan/work-packages.md: Owner decision 11, WP-077, WP-006, WP-046, Databases section | Public IDs derived from content identity with a keyed MAC (recommended) | Public IDs are 128 random bits from the CSPRNG, minted once and kept in the identity store's public-ID mapping; WP-077 no longer derives IDs | SEC-HIS-012, SEC-API-023, SEC-PRV-021 | yes |
| A-492 | plan/work-packages.md: Owner decision 31, WP-103 (artwork) | First-party web artwork at cookie-authorised content-addressed paths (recommended) | Artwork only from capability URLs with a 1-hour lifetime aligned to a time bucket so browsers can cache; cookie-path proposal withdrawn | SEC-API-026, SEC-API-027, SEC-API-029, SEC-HIS-006 | yes |
| A-493 | plan/work-packages.md: Owner decision 30, WP-104 (session registry) | Admins see live sessions, totals and security events by default (titles shown) | Admin live view shows user, device, bitrate and method but no title unless the person opted in; no history; reads rate-limited and logged in the subject's log | SEC-PRV-025, SEC-TM-054, SEC-IAM-077 | yes |
| A-494 | plan/work-packages.md: WP-097 (alerts) | Alert destinations the owner chooses: in-app always, others through the egress gate, in R1 | In-app alerts only in R1; outbound alert channels are R2/Later; never-suppressible alerts, This-wasn't-me action, config-change detection added | SEC-OPS-035, SEC-OPS-032, SEC-OPS-033, SEC-OPS-034, SEC-TM-074 | yes |
| A-495 | plan/work-packages.md: WP-090 (backups) | Retention keeps the configured count | Kept for a configurable period, 14 days by default, deleted on schedule | SEC-PRV-041 | no |
| A-496 | plan/work-packages.md: Owner decision 12, crates section, WP-074, WP-090 | Update feed verification (tough/TUF or a simpler signed document) and backup encryption format left to the owner | Feed is TUF 1.0 verified from a compiled-in root; backups are age v1 to the backup key and owner's recovery key; still open: tough vs minimal TUF client and the archive format | SEC-SUP-049, SEC-OPS-019, SEC-SUP-050, SEC-OPS-042, SEC-PRV-039 | yes |
| A-497 | plan/work-packages.md: WP-109 (restore) | Restore parses the archive with limits and writes the restored data directly | Extraction waits for an architecture record allowing it (WP-125 record 10); restored SQLite files opened read-only in a jailed worker with trusted_schema off before any query; keys rotated and sessions invalidated after restore | SEC-HIS-019, SEC-STD-031, SEC-OPS-044, SEC-OPS-043 | yes |
| A-498 | plan/work-packages.md: Owner decision 18, WP-121 | macOS and Windows server builds run with the reduced isolation tier | Linux-only server builds in R1; macOS/Windows only once their sandbox profiles exist (R2) | SEC-MED-024, SEC-MED-082 | yes |
| A-499 | plan/work-packages.md: Owner decision 24, WP-045, WP-121 | Build 32-bit ARM and label the isolation tier as reduced (supported) | Build it with the reduced tier label but do not claim it supported until the ARMv7 seccomp answer is recorded | SEC-MED-024 | yes |
| A-500 | plan/work-packages.md: Owner decision 21, WP-101 | Built-in HTTPS by ACME in R1 only if WP-073 lands early, otherwise a point release | R1: ACME for the project name service and own domain, with renewal, chain checks, expiry alerts; added WP-129 (name service) and WP-135 (naming client and CT monitoring) | SEC-NET-010, SEC-NET-004, SEC-NET-003, SEC-TM-010, SEC-NET-013, SEC-NET-069, SEC-NET-072 | yes |
| A-501 | plan/work-packages.md: WP-091, WP-031, WP-093, WP-098, WP-110, WP-117, coverage rows API-TOK-01/02 | Scoped API tokens and the tool change feed in R1 (wave 3); API key format in WP-031; tool writes and token-scoped refresh in R1 | WP-091 moved to R2 (wave 7) with the gmk_ key format; tool writes and token refresh go with it; dependencies on WP-091 removed | SEC-EXT-008, SEC-EXT-010, SEC-EXT-012, SEC-IAM-083, SEC-TM-074 | yes |
| A-502 | plan/work-packages.md: New WP-137, owner decisions intro | No metadata provider in R1 (ADR 2: lookups belong in plugins) | Conditional WP-137 builds MusicBrainz/cover-art providers off by default behind the egress client and the required setup step; drops to R2 plugins if the owner declines | SEC-PRV-013, SEC-PRV-014, SEC-PRV-015, SEC-PRV-016, SEC-API-079, SEC-API-080, SEC-API-081 | yes |
| A-503 | plan/work-packages.md: Owner decision 33, WP-001, WP-060, crate table | Store, identity store, log writers, secrets crate, datadir module and testkit exempt from the path-based std::fs ban | Data directory accessed through a data-root handle in gunmetal-fs (new WP-126, wave 0); only SQLite's own open and the testkit are exceptions | SEC-MED-033, SEC-HIS-016, SEC-TM-043, SEC-OPS-012 | yes |
| A-504 | plan/work-packages.md: WP-008, shared-file table, wave 1 intro, parsers ground rule | Harness registry in crates/gunmetal-core/src/harness.rs with corpus in crates/gunmetal-core/corpus/ | Harnesses in crates/gunmetal-fuzz (one module per parser), seeds in fuzz/seeds/<harness>/, as the repository and secure-coding guide already have them; registry lines in gunmetal-fuzz/src/lib.rs and fuzz/Cargo.toml | SEC-MED-027, SEC-MED-028, SEC-MED-029 | no |
| A-505 | plan/work-packages.md: Crates section ('no inflate in the core'), WP-079, new WP-128 | PNG inflate happens inside the image crate; no decompression helper | One streaming decompression helper (WP-128) bounds PNG inflate before image decodes; needs an inflate crate on the core allowlist | SEC-MED-009, SEC-SUP-025 | yes |
| A-506 | plan/work-packages.md: WP-033, WP-065, WP-067, Review notes | LibrarySet had a public core constructor; 'only the access module builds one' was a convention plus review | Storage readers take a Permit that only the core policy function can mint (compile-fail tested) | SEC-API-010, SEC-IAM-070, SEC-IAM-068 | no |
| A-507 | plan/work-packages.md: WP-121, owner decision 32, new WP-136 | Signing releases out of scope until owner decides keys | Sigstore keyless signing, SLSA L3 provenance, reproducible builds, SBOMs and image scanning in R1 (WP-136); only the TUF feed needs offline keys | SEC-SUP-040, SEC-SUP-041, SEC-SUP-042, SEC-SUP-043, SEC-SUP-044, SEC-SUP-049 | yes |
| A-508 | plan/work-packages.md: WP-086 (removing plays), new WP-133 | Removing a play only hides it through a removal event | History deletion (entry, range, all) erases from log, cache, indexes within 24 h with WAL truncation; tombstones carry only IDs; retention schedule and account deletion added | SEC-PRV-049, SEC-PRV-050, SEC-PRV-052, SEC-PRV-005, SEC-PRV-051 | no |
| A-509 | plan/work-packages.md: WP-106 (wave 4), WP-062 (wave 2), One door per risk table; wave 3 users WP-090, WP-094, WP-096, WP-099, WP-131, WP-132 | The check that a fresh-uv route has a user-verified passkey assertion no older than 5 minutes, never from OIDC, was built only in WP-106 (wave 4). Wave 3 packages that ship fresh-uv routes and test their refusal could not depend on it, so nothing enforced fresh-uv between waves 3 and 4. | WP-062 (wave 2) records each session's last user-verified assertion time and credential kind (set only through record_user_verification, called by WP-081, WP-106 and setup) and provides the pipeline check for RouteTag::FreshUv: refused when the assertion is older than 5 minutes or came from OIDC. WP-106 keeps the step-up route that renews the assertion, the sign-in methods page and recovery. Wave 3 packages test with a test enroller. WP-090, WP-099, WP-100, WP-132 now depend on WP-062. New door row 'Fresh user verification'. | SEC-IAM-041, SEC-IAM-107, SEC-TM-017 | no |
| A-510 | plan/work-packages.md: One door row 'The client address and its path class'; WP-132, WP-118, WP-005/WP-006, WP-001, WP-062, WP-064, WP-069, WP-130, WP-080 | The server-side resolver (socket peer, trusted-proxy list, forwarding-header rule, gateway 'unknown' rule) was in WP-132 (wave 3), after the wave 2 sessions, verifier, audit log, request limits and listener cap that need a resolved address. WP-080 decided 'loopback' for the claim code without it, so a same-host reverse proxy could make internet requests look like loopback. The case was deferred to WP-117. | WP-118 (wave 2, the only module that sees the socket) owns the resolver: it calls WP-023's path-class function and attaches a ClientContext (defined in WP-006, pub(crate) constructor). WP-062, WP-064, WP-069 and WP-130 accept only ClientContext. WP-001 bans ConnectInfo and peer_addr outside listener.rs. A loopback peer that sends forwarding headers without being a trusted proxy is classified internet, and WP-080 tests that such a peer cannot claim. WP-132 keeps postures, the cleartext rule, exposure alerts and the trusted-proxy settings routes. SEC-NET-016 and SEC-OPS-037 moved from WP-132 to WP-118, with the cross-subsystem test in WP-117. | SEC-OPS-037, SEC-NET-016, SEC-NET-017, SEC-NET-018, SEC-NET-068, SEC-IAM-008, SEC-IAM-013 | no |
| A-511 | plan/work-packages.md: WP-069 Audit log; WP-043, WP-048, WP-064, WP-065; WP-006; One door row 'The audit log' | WP-043, WP-048, WP-064 and WP-065 claimed requirements proved by an audit record and said the audit log 'records' their events, but WP-069 owned no subscriber and did not depend on WP-043, where the bus lives. WP-069 also extended app.rs and cli.rs without that dependency. | WP-069 depends on WP-043 and owns crates/gunmetal-server/src/audit_sink/, which records every security event from the bus and is tested with literal bus events. The typed SecurityEvent catalogue and the SecuritySink trait moved to the core (WP-006, wave 0, a registry file), so the egress crate and WP-044's route specs can name events. Producers now prove only that they emit the event. The stored-record part of these requirements is WP-069's (the sink) and WP-117's (end to end). | SEC-OPS-029, SEC-PRV-008, SEC-IAM-069, SEC-TM-014, SEC-HIS-046, SEC-OPS-020 | no |
| A-512 | plan/work-packages.md: WP-069 (addresses and retention), WP-133, Databases section | Audit records carried source addresses inside the hash chain. WP-069 planned only whole-record pruning, and WP-133 claimed to coarsen addresses in WP-069's store, editing another package's files. Either the privacy schedule failed or `audit verify` broke at day 30. | Records hold only a keyed commitment (HMAC under a dedicated key) over the address and a per-record salt. The address and salt live in a side store (durable/audit/addresses.db, secure_delete on). It is coarsened to /24 or /48 at 30 days and both are deleted at 90, and each run is a signed checkpoint. A manual-clock test does a raw-byte scan at day 91 and runs `audit verify` after each run. WP-133 only calls WP-069's retention API. This applies the baseline's recommendation for owner decision 15. | SEC-PRV-003, SEC-PRV-005, SEC-IAM-094, SEC-OPS-023, SEC-OPS-026 | yes |
| A-513 | plan/work-packages.md: WP-033/WP-065/WP-067 Permit design; WP-046, WP-068, WP-069 readers; One door row 'Authorisation' | Only the catalogue store's readers (WP-067) required a &Permit. Readers in the identity store, the user log and the audit log took none, so a handler could read another person's Activity or Identity data without asking the policy. | Every reader that returns a user-visible object takes a &Permit. WP-046 keeps its general reader pub(crate), and WP-065 adds the public Permit-taking reader in crates/gunmetal-durable/src/identity/permitted.rs, because Permit (WP-033) and WP-046 are both wave 1. WP-068 and WP-069 readers take a Permit. There is a written list of exemptions: the PrePrincipal lookups in session/, verifier/ (through a pre-authentication handle) and access/ (the grant read), plus the rebuild replay and the host-only audit verify. WP-065 owns an architecture test that scans the server crate. SEC-TM-024 and SEC-API-010 are added to WP-046, WP-064, WP-068 and WP-069. | SEC-TM-024, SEC-API-010 | no |
| A-514 | plan/work-packages.md: Registry table (public-routes.txt), Definition of done, WP-131, WP-118 | WP-131 (wave 3) created security/public-routes.txt and the anonymous suite, while about twenty wave 3 route packages had to add lines and pass those suites without depending on it. WP-118 (wave 2) claimed SEC-IAM-067 before any allow-list existed. | WP-118 (wave 2) creates public-routes.txt, the allow-list check and the anonymous-request suite for no and malformed credentials. WP-131 extends the suite with the expired, revoked and disabled-account states and keeps the cross-principal matrix and other generated suites. The Definition of done now asks a package to pass the checks present when it rebases. A WP-131 failure on another package's route goes back to that route's owner as a new small package. | SEC-API-002, SEC-API-003, SEC-IAM-067, SEC-TM-004 | no |
| A-515 | plan/work-packages.md: WP-131 (rival replays, API fuzzing), WP-089, WP-118, WP-102, WP-103, WP-109, WP-112, WP-134 | WP-131 (wave 3) claimed one rival replay per incident, though many incidents concern features built later. It fuzzed the server through an OpenAPI description that WP-089 built in the same wave. | tests/rivals/ is a registry directory. WP-131 owns the harness and the replays for wave 3 features. WP-102, WP-103, WP-109, WP-112 and WP-134 each add their own incident's replay and list SEC-HIS-066. OpenAPI generation, the CI diff and the release-route check (SEC-API-091) moved from WP-089 to WP-118, and openapi.json is a generated shared file. WP-089 now only serves the API reference. | SEC-HIS-066, SEC-STD-038, SEC-API-091 | no |
| A-516 | plan/work-packages.md: One door table (new 'Cryptography' row), crate table, WP-001, WP-122, WP-047, WP-031, WP-048 (moved to wave 2), WP-073, WP-074, WP-081 | Cryptographic crates were spread over core (hmac, sha2), secrets, server (p256, age, ed25519-dalek) and server/egress (rustls), with no confinement lint. WP-031 tested its own HMAC helper in the core. | Two named crypto modules: gunmetal-core/src/crypto.rs (WP-122, SHA-256 only) and gunmetal-secrets/src/crypto/ (WP-047, every other algorithm and the rustls provider and configuration constructors). WP-001 bans the crates' types outside them, and WP-047's xtask diffs the inventory against them. hmac left the core. WP-031 tests through a fake MacProvider with literal RFC 4231 tags. WP-048 moved to wave 2 so it can take its TLS configuration from WP-047. This reads 'one crypto module' as one door in two files, recorded as owner decision 38. | SEC-STD-018, SEC-STD-019 | yes |
| A-517 | plan/work-packages.md: One door row 'SQL', WP-042, WP-046, WP-071, WP-126, WP-001 exception list, owner decision 33 | WP-042's sql.rs was 'the one door for SQL', but WP-046 (same wave 1) could not depend on it and would have needed its own wrapper and opener. SEC-PRV-050 was not checked on the identity store's or the derived-data store's connections. | The static-query type and the one SQLite connection opener moved to WP-126 (wave 0, gunmetal-fs). The opener sets and reads back the pragmas (secure_delete, foreign_keys, trusted_schema) and has a read-only profile for untrusted files, which WP-109 uses. WP-042, WP-046 and WP-071 use it. SEC-API-066 and SEC-TM-039 were added to WP-046, and SEC-PRV-050 to WP-046 and WP-071, each with a pragma test on every pooled connection. SQLite's open no longer needs a std::fs exception. | SEC-API-066, SEC-TM-039, SEC-HIS-038, SEC-PRV-050 | no |
| A-518 | plan/work-packages.md: WP-006 PublicId, WP-046, WP-047, WP-062, WP-102 | PublicId::new(kind, bytes) was public, so any code could build an ID from a path hash or zeros. WP-046 minted IDs 'from the CSPRNG' without depending on WP-047, the one CSPRNG function. | A PublicId is built only from a Minted value. Minted::from_os_random may be called only from WP-047's minting function, enforced by a WP-001 disallowed-methods entry. WP-046 takes minted IDs as input. The 'from the OS CSPRNG' part of these requirements is WP-047's, plus WP-062 for device IDs and WP-102 for content IDs. | SEC-HIS-012, SEC-API-023, SEC-PRV-021, SEC-STD-022 | no |
| A-519 | plan/work-packages.md: WP-133 (retention.rs), new WP-138 (wave 1), WP-069, WP-090, WP-097; new door row 'Retention schedule' | The one retention schedule was built in WP-133 (wave 4), after the audit log, log rotation and backup expiry that need it, so each would have hard-coded its own values. | New package WP-138 (wave 1, core) owns retention.rs: the defaults table and the idempotent purge decision, with SEC-PRV-005's core property test. WP-069, WP-090, WP-097 and WP-133 depend on it. WP-133 keeps the sweep job, erasure and account deletion. | SEC-PRV-005, SEC-TM-055, SEC-OPS-026, SEC-PRV-041, SEC-PRV-045 | no |
| A-520 | plan/work-packages.md: WP-032, WP-064, WP-130 | WP-130 'provides the server-wide counters' that WP-064, in the same wave and not depending on it, was supposed to share. That forced a second limiter or left the server-wide half unproven. | The server-wide counter is WP-032's keyed GCRA under a LimitKey::Global key, used by both WP-064 and WP-130. The sentence was removed from WP-130. SEC-IAM-101 is under WP-032, WP-064 and WP-130. | SEC-IAM-101 | no |
| A-521 | plan/work-packages.md: Coverage rows SEC-SUP-033 to SEC-SUP-036, SEC-CLI-018, SEC-SUP-029; WP-136; WP-124 | These rows were unassigned as web-client behaviour, although WP-136's release workflow builds the web bundle and so installs npm packages. | WP-136 verifies the release-workflow part of SEC-SUP-033, SEC-SUP-034, SEC-SUP-036 and SEC-CLI-018 and the JavaScript licence check of SEC-SUP-029, with xtask lints of build-reusable.yml's package-manager configuration and a lifecycle-script canary. WP-124 creates supply-chain/js-direct-deps.toml and its xtask check (SEC-SUP-035). The client-CI part is left to the client plan. | SEC-SUP-033, SEC-SUP-034, SEC-SUP-035, SEC-SUP-036, SEC-CLI-018, SEC-SUP-029 | no |
| A-522 | plan/work-packages.md: WP-069, WP-100, WP-090, WP-117; coverage rows SEC-OPS-027, SEC-OPS-075 | SEC-OPS-027 and SEC-OPS-075 were mapped only to WP-069, which has no routes and no alerts. No package planned the investigation mode, the head route or the checkpoint export. | WP-100 owns the audit routes with per-role replay tests and truncated response types (SEC-OPS-027). It also owns the investigation mode: an owner, fresh-uv action that is audited and publishes one alert event per affected person, with the end-to-end alert in WP-117. WP-100 also serves the checkpoint head. WP-069 builds the head reader and the readers' truncation. WP-090 exports checkpoints to a second destination and to a printable sheet for the recovery kit. The admin client's check of the head at sign-in is marked unassigned (client plan). | SEC-OPS-027, SEC-OPS-075 | no |
| A-523 | plan/work-packages.md: Coverage rows SEC-TM-058, SEC-CLI-010, SEC-TM-053, SEC-CLI-009, SEC-API-049; WP-062, WP-117, WP-120, WP-072 | SEC-TM-058 and SEC-CLI-010 were mapped wholly to WP-062, and SEC-TM-053 wholly to WP-117, though those packages can prove only the server half. SEC-CLI-009 and SEC-API-049 were wholly unassigned, though WP-120 and WP-072 implement their server clauses. | Each row now names its server part (WP-062, WP-117, WP-120 for Clear-Site-Data, WP-072 for the CSP) and marks the client part unassigned (client plan). WP-120 and WP-072 list the IDs. The table introduction was recounted: 591 assigned, 16 unassigned, 6 rows with an unassigned client part and 5 JavaScript rows with a client-CI part. | SEC-TM-058, SEC-CLI-010, SEC-TM-053, SEC-CLI-009, SEC-API-049 | no |
| A-524 | plan/work-packages.md: Working in parallel registry table; WP-001, WP-124, WP-130 | supply-chain/ was owned by WP-001 in wave 0 only, while about eight wave 1 packages add crates that need vet entries. CODEOWNERS (WP-124, wave 1) had no owner for the security paths created later. REUSE.toml had to change as wave 1 parsers add binary seeds. limits.toml was created by WP-130 while WP-064, in the same wave, had limits to register. | supply-chain/ is integrator-owned after wave 0 and updated in the same dependency-request step as the root Cargo.toml (js-direct-deps.toml excepted). WP-124 pre-lists every planned security-sensitive path in CODEOWNERS, which then becomes a registry. REUSE.toml has a fuzz/seeds/** annotation from the start, and other binaries get .license sidecars. limits.toml is a registry any package may create. Also added to the table: audit_event.rs, openapi.json (generated) and tests/rivals/. | SEC-SUP-024, SEC-SUP-025, SEC-STD-035, SEC-SUP-030, SEC-STD-030 | no |
| A-525 | plan/work-packages.md: WP-097, WP-117; coverage row SEC-OPS-033 | WP-097 (wave 3) tested that 'This wasn't me' with a stream playing makes the next range request fail, which needs WP-082's stream route from the same wave. | WP-097 proves the action revokes the device, ends its sessions and bumps the epoch through WP-062's API (WP-062 added to its dependencies). The 'next range request fails within one URL lifetime' assertion moved to WP-117. | SEC-OPS-033 | no |
| A-526 | plan/work-packages.md: WP-062, WP-064, WP-133, WP-043; coverage rows SEC-PRV-003, SEC-HIS-004 | SEC-PRV-003's session-record clause was mapped only to WP-133, which cannot edit WP-062's session files. SEC-HIS-004's 'every sign-in pathway rejects an empty or missing credential' had no package (only WP-043's config schema). | WP-062 removes the address from a session record when it ends, tested by a raw-byte scan. WP-133 keeps the sweep. WP-064 enumerates Pathway and asserts that an empty and a missing credential are refused for every variant. | SEC-PRV-003, SEC-HIS-004 | no |
| A-581 | plan/work-packages.md: 'Decisions the owner must make' item 8 (TLS crypto provider) | 'Use the provider rustls defaults to, and revisit when a pure-Rust provider is proven' | D-08: aws-lc-rs if it builds cleanly for the R1 Linux targets, otherwise ring, chosen through the reviewed crypto allow-list and recorded in the cryptography architecture record (WP-125) | SEC-STD-019 | yes |

### T15 Interface consistency between the UI documents

23 changes, 10 to confirm. Decision: D-70, D-77. Settlements of contradictions between the four interface documents (UI README decisions 1 to 23).

| ID | Where | Before | After | Requirements | Confirm |
|---|---|---|---|---|---|
| A-527 | ui (README decisions): player.md (now-playing bar behaviour, picture-in-picture section, TV remote Back row, open question 10); surfaces.md (How music and video coexist, SUR-002); README decision 1 | surfaces.md returned the bar to the paused music context after a film; player.md kept the paused film in the bar with a resume control on web and desktop. The bar could not show both. | On every client, leaving the video player pauses the film and saves its resume point (VID-118) unless picture-in-picture or listen-only is on. The bar returns to the paused music context, one tap from resuming, and shows a video only in listen-only mode (VID-065). The film resumes from its title page, Continue Watching or the queue switcher (MUS-131). | None (consistency) | yes |
| A-528 | ui (README decisions): design-language.md section 11 rule 6; player.md (The queue); surfaces.md SUR-011; README decision 2 | design-language.md said "remove from queue" acts at once and offers Undo; player.md and surfaces.md put queue undo (MUS-121) in R2 and confirmed only "Clear queue" in R1. | Following the feature map (MUS-121 is R2): in R1, removing one row acts at once with no prompt, and "Clear Up next", "Clear queue" and removing a multi-selection confirm first and say how many items will go. From R2 they all act at once and offer Undo. Rule 6 now says so. | None (consistency) | no |
| A-529 | ui (README decisions): player.md (The queue editing rules, Touch table); README decision 3 | player.md listed "swipe a queue row to remove it" as R1 under MUS-119; surfaces.md made swipe actions on rows R2 under MUS-065. | Swipe-to-remove is R2 under MUS-065, which the feature map places in R2. In R1 a row is removed with its remove button (always shown on touch screens) or from its row menu. | None (consistency) | no |
| A-530 | ui (README decisions): surfaces.md (Navigation model TV, What is always on screen, SUR-002, SUR-010, open question 4); design-language.md section 8 rule 6; player.md (Music on a TV); README decision 4 | design-language.md made Now Playing the TV rail's fixed first entry; surfaces.md showed it first only "whenever something is playing", which would shift every rail entry and break the layout contract. | Now Playing is always the rail's first entry, present whether or not anything plays. With nothing queued it opens the empty-queue state. No rail entry ever moves (CLI-035, layout contract). | None (consistency) | no |
| A-531 | ui (README decisions): design-language.md section 8 rule 7; player.md (Music on a TV focus, TV remote Back row); surfaces.md (TV focus, What is always on screen, SUR-010); README decision 5 | player.md: Back from TV Now Playing goes to the TV music home. design-language.md: Back always goes up one level, content to rail. surfaces.md: "Back returns". DIS-112 promises the exact previous place. | Back goes back one step, to where the person came from, with the same row and card focused (DIS-112, CLI-036). From a screen opened from another (including TV Now Playing) it returns to that screen; from a top-level screen's content to the rail; from the rail to Home; from Home it asks before leaving. Music keeps playing. | None (consistency) | no |
| A-532 | ui (README decisions): surfaces.md (Web and desktop keyboard paragraph, SUR-032); player.md open question 1; README decision 6 | surfaces.md gave the R1 web search field a keyboard shortcut; player.md put every shortcut, including "/", in R2. | No shortcut in R1. The search shortcut arrives in R2 with DIS-090 and CLI-061, which the feature map places in R2; in R1 the field is reached by Tab. player.md open question 1 (bringing a few player shortcuts forward) stays open and excludes "/". | None (consistency) | no |
| A-533 | ui (README decisions): player.md (queue menu); surfaces.md SUR-011; README decision 7 | surfaces.md put "set a sleep timer from the queue menu" in R1; player.md had the sleep timer only in the full player's options menu. | The sleep timer is in both the player's options menu and the queue menu, as one action from the shared action list, because MUS-076's UI surfaces cell names both menus. | None (consistency) | no |
| A-534 | ui (README decisions): player.md (bar table and behaviour, full player bottom row, layout contract, device handoff R1, Surfaces table, States); surfaces.md SUR-002; README decision 8 | player.md proposed an R1 device slot reading "Playing on <device>" with "Play here"; surfaces.md made the device button R2 and gave R1 only the "Continue on this device" prompt. | The bar's device slot exists from R1 and keeps its place, but in R1 it holds only the CLI-103 "Continue on this device" prompt (item and position, no device name) when another of the profile's sessions last played the queue; otherwise it is empty. The Devices button (CLI-101, MUS-197) fills the slot from R2. | None (consistency) | yes |
| A-535 | ui (README decisions): player.md (bar behaviour, device handoff, States, What the player needs, open question 2); flows.md (F05 failure cases, F08 R1 section, G11); README decision 9 | player.md proposed a separate active-device field on the queue; flows.md G11 proposed that the tab that pressed Play last takes the queue using only queue versions. Same outcome, two server mechanisms. | flows.md's mechanism: play and resume are ordinary operations on the versioned queue (MUS-122), which records the issuing session by an opaque identifier that is not a credential. The last Play wins; any other session pauses on its next sync and shows the decision-8 prompt. There is no separate active-device record and no device name. Only the profile's own sessions write its queue, and events reach only them. | SEC-HIS-014, SEC-API-016 | yes |
| A-536 | ui (README decisions): player.md (On-screen controls, section renamed "Next episode, autoplay and the post-play screen"); surfaces.md (SUR-020, SUR-042, SUR-046, SUR-073, Later podcast row); flows.md F17; README decision 10 | "Up next" named the music lane of the person's picks, the video queue panel and autoplay surface (VID-181), and, with DIS-025's "Next Up" row and DIS-031's "Up Next", two more things. | On screen, "Up next" names only the lane of a person's own picks. The video queue panel (the feature map's "Player > Up next") is labelled "Queue". The Home row fed by DIS-025 and DIS-031 is labelled "Next to watch", and the collection page offers "Watch the next film". Feature IDs and the map's names are unchanged. | None (consistency) | yes |
| A-537 | ui (README decisions): player.md (bar table, quality badge table); surfaces.md SUR-002; design-language.md Badges; README decision 11 | player.md showed the badge in the phone-width bar only when playback was not the original played directly, with "FLAC 24/96" as the compact form; surfaces.md listed it on the bar without exception; design-language.md showed the compact "Original" badge as normal. | The compact quality badge shows in the bar on every layout, phone width included, because MUS-099 names the bar. It reads "Original" (brass) when the original plays directly and "Converted" from R2 otherwise, with the full sentence on hover, focus or tap. "FLAC 24/96" is the separate format badge for rows, tiles and track info. | None (consistency) | yes |
| A-538 | ui (README decisions): player.md (Lyrics state table, quality badge table, Cannot play here state); README decision 12 | player.md used "No lyrics in this file" and "Cannot play here: this browser cannot decode ALAC"; design-language.md used "This file has no lyrics." and "Can't play in this browser: ALAC". | One string each, taken from design-language.md, which owns the copy tone: "This file has no lyrics." and "Can't play in this browser: ALAC". | None (consistency) | no |
| A-539 | ui (README decisions): surfaces.md (Releases, Form factors, Web and desktop, SUR-000, SUR-014, SUR-050, SUR-052, SUR-053, SUR-057, SUR-074, SUR-075, SUR-077, behaviour-only table, coverage, open question 9, changes table); flows.md (R1 facts, F08, flow index, changes table); player.md and design-language.md wording; README decision 13 | After the security edits, player.md and design-language.md put the desktop shell in Later. surfaces.md and flows.md still had the shell, its mini player (SUR-052, R2), desktop media panels, global hotkeys, installers, downloads and exclusive output in R2, and F08 handed playback to "the desktop" app. | The desktop shell is Later everywhere, following the baseline's release scope; the feature map now agrees. SUR-052 keeps its ID with release Later; Serves lines list CLI-014, CLI-063, CLI-064, CLI-065, MUS-081, MUS-082 and MUS-102 under Later. Desktops use the web client until the shell ships, and F08 hands playback to the web client on a laptop. | SEC-TM-074, SEC-CLI-069 | yes |
| A-540 | ui (README decisions): player.md (Listening through a share link); README decision 14 | player.md said the public share page SUR-059 "still" sat in R2 in surfaces.md, but surfaces.md already had it in R1. | Both documents place the public share page in R1 for music. Owner decision 7 is already flagged owner to confirm. | SEC-API-097, SEC-PRV-031 | yes |
| A-541 | ui (README decisions): surfaces.md SUR-002 (and its changes-table row); player.md volume row; README decision 15 | player.md added an Options button to the wide bar (Private session first, two interactions from the bar); surfaces.md SUR-002 listed no such control and said the private session starts from the player menu. | The Options button is part of the wide bar, in the right-hand group before the device slot. It opens the player's options menu with Private session first. SUR-002 lists it and its security line cites the bar route. | SEC-PRV-024 | no |
| A-542 | ui (README decisions): surfaces.md (SUR-003, SUR-103, changes table); flows.md (F05 failure cases, F10 other releases, F12 listener note and case L, changes table); README decision 16 | player.md and design-language.md made the "too many streams" state R1. flows.md treated it as an R2 video case and listed ACC-075 under R2, and surfaces.md had no R1 home for stream limits. | Stream limits apply to music from R1, matching ACC-075's R1 placement. SUR-003 shows the "too many streams" notice, SUR-103 holds the server-wide and per-guest limits, F05 and F12 cover the music case, and F10 lists ACC-075 under R1. Default values remain the owner's (security decision 25). | SEC-TM-068, SEC-API-031, SEC-IAM-102 | yes |
| A-543 | ui (README decisions): surfaces.md SUR-014; flows.md F08 steps 2 and 3 and changes table; README decision 17 | player.md listed only the profile's own players plus household devices shown as "In use". surfaces.md SUR-014 had no "In use" entry. flows.md F08 step 2 listed players another person had granted control of, which its own failure case and ACC-047 place in Later. The handoff contents were unstated. | Everywhere: only the profile's own players, plus household devices in use shown greyed as "In use" with no name or title; controlling another person's player is Later (ACC-047). The handoff carries item IDs and the position, never a stream URL or token, and the target requests its own capability URLs. | SEC-HIS-014, SEC-PRV-022, SEC-API-016, SEC-API-068, SEC-API-026 | no |
| A-544 | ui (README decisions): player.md (Casting); flows.md F08 step 6 and failure case; README decision 18 | player.md cited SEC-CLI-071 (a Later requirement) for R2 casting and bound cast URLs to the sender's session; flows.md cited SEC-NET-064 and "one cast session" without saying who refreshes them. | Cast URLs are capability URLs scoped to one item and to the cast session the sender started. The sender mints and refreshes them within the SEC-API-027 lifetimes, which stay inside SEC-NET-064's bound. The receiver holds no credential, because cast credentials are Later. Ending the sender's session ends the cast at the next range request. | SEC-NET-064, SEC-API-027, SEC-API-098, SEC-API-028, SEC-TM-074, SEC-CLI-071 | no |
| A-545 | ui (README decisions): player.md (Security rules for every surface: Where the player runs); design-language.md section 11 (Not a secure address); README decision 19 | player.md and design-language.md said every non-loopback peer over plain HTTP gets a static help page; surfaces.md SUR-109, flows.md and SEC-NET-001 allow a redirect to the HTTPS address first. | Over plain HTTP, every peer but loopback gets a redirect to the HTTPS address when one exists, otherwise the static help page, and never the web client. | SEC-NET-001, SEC-NET-005 | no |
| A-546 | ui (README decisions): flows.md F10 (Other releases) and changes table; README decision 20 | F10 listed re-inviting people from an old server (ACC-084, ADM-048) as R2, while F11 and the feature map place them in Later with the rival-database importers. | Re-inviting people from an old server is Later, with the rival-database importers. | SEC-TM-074 | no |
| A-547 | ui (README decisions): flows.md F20 (When it goes wrong); README decision 21 | F20 said the viewer "sees who holds each one" when every tuner is busy, then said the sheet names a person only by that person's choice. | The viewer sees a sheet of choices, which names a person or channel only for people who chose to share what they play with the household; otherwise it says only that the tuner is busy. This matches surfaces.md SUR-122. Security decision 5. | SEC-PRV-022 | yes |
| A-548 | ui (README decisions): player.md (Music on a TV, ambient mode); design-language.md section 8 rule 8; surfaces.md SUR-060; README decision 22 | player.md said any key wakes TV Now Playing's ambient mode without acting on playback; design-language.md said the play and pause keys control playback on every screen (CLI-045). | In ambient mode and the TV screensaver, the play or pause key wakes the screen and acts, showing the brief symbol; every other key only wakes it. | None (consistency) | yes |
| A-549 | ui (README decisions): surfaces.md SUR-010 and SUR-011 (TV form factor); README decision 23 | player.md placed TV Now Playing's queue rail below the artwork and transport; surfaces.md said the queue sat beside it. | The queue rail sits below, as player.md lays it out. | None (consistency) | no |

### T16 Owners, administrators, access and limits

19 changes, 5 to confirm. Decision: D-27. Ownership transfer, administrators, fresh user verification for host actions, and device and stream limits.

| ID | Where | Before | After | Requirements | Confirm |
|---|---|---|---|---|---|
| A-550 | features/library.md: LIB-180 Labels | Anyone with edit rights can make a label, and labels are usable in parental allow and block lists (ACC-028). | A label that a parental allow or block list uses can be added or removed only by someone allowed to change that restriction, so edit rights never loosen a child's limits. | SEC-IAM-064, SEC-TM-027 | yes |
| A-551 | features/library.md: LIB-003 Several folders per library; LIB-031 Move the server | Adding or relocating a root had no step-up or audit stated. | Adding, removing or relocating a root is an admin action under a fresh passkey check, with an audit record. | SEC-IAM-041, SEC-TM-017, SEC-OPS-020 | no |
| A-552 | features/clients.md: Deliberately not doing: paid unlocks | No device caps | No paid device caps. The admin-adjustable limits on enrolled devices and concurrent streams that the baseline requires are kept as a safety control, not a paywall | SEC-IAM-102 | yes |
| A-553 | features/accounts.md: ACC-005 Hand ownership to another person | Release Later. Until it ships, ADM-052 (add a second admin and remove yourself) is the way to hand a server over. | Release R1. This transfer is the only way ownership moves. Both people confirm with a passkey check in the last 5 minutes, the transfer is audited and all admins are alerted. The owner can never be removed or demoted, so ADM-052's method cannot work. | SEC-IAM-003, SEC-TM-017, SEC-IAM-041 | no |
| A-554 | features/accounts.md: ACC-040 Several administrators | 'Parity' with Jellyfin. | Only the owner can create or promote an admin, with a fresh passkey check. All admins get an alert that cannot be switched off, and admins never hold owner-only powers. | SEC-IAM-075, SEC-IAM-041, SEC-OPS-034 | no |
| A-555 | features/accounts.md: ACC-041 Scoped admin roles | Roles are built from the same scope list as tokens, so every role power has a token scope. | Roles come from one capability catalogue. Owner-only capabilities can never be put in a role, and every admin capability is marked as unavailable to tokens. | SEC-IAM-075, SEC-EXT-010 | yes |
| A-556 | features/accounts.md: ACC-075 Stream limits that count playing, not browsing | R2. | R1. Documented, admin-adjustable limits; the recommended default is 3 concurrent streams per guest. | SEC-IAM-102, SEC-TM-068, SEC-API-031 | yes |
| A-557 | features/accounts.md: ACC-076 Device limits and allow-lists | R2. | R1 device cap (25 per member and 5 per guest recommended). Allow-lists arrive with device keys in R2. | SEC-IAM-102 | yes |
| A-558 | features/admin.md: ADM-052 | Hand-over by adding a second admin, then removing yourself (uses ACC-040). | Two-party ownership transfer: owner starts, recipient accepts, each with a fresh passkey check; exactly one owner throughout; all admins alerted. | SEC-IAM-003, SEC-IAM-041, SEC-IAM-075, SEC-TM-017 | no |
| A-559 | features/admin.md: ADM-144 (new) | missing | Rotate every server key in one action (R1, owner, fresh check), without re-pairing device-key devices. | SEC-OPS-018, SEC-OPS-015, SEC-IAM-041 | no |
| A-560 | features/video.md: VID-173 | Concurrent stream limits were 'enforced when stream URLs are minted'. | The limit is checked at minting and again on every stream request, so URLs minted earlier cannot be used to exceed it. The stream over the limit gets a typed 'too many' error, and adapter streams count against the same limit. | SEC-API-031, SEC-TM-068, SEC-EXT-062 | no |
| A-561 | features/live-tv.md: LIV-169 | "Gunmetal clients play the original files at the scheduled offset", with no visibility rule | A virtual channel is visible only to people who can read every library it draws from, and rating limits apply to each item. | SEC-TM-026, SEC-API-014 | no |
| A-562 | ui/flows.md, ui/surfaces.md: surfaces.md SUR-090 | Hand the server to a new owner by adding an admin and removing yourself | Ownership transfer confirmed by current owner and recipient with fresh user verification; owner cannot be removed | SEC-IAM-003 | no |
| A-563 | ui/player.md, ui/design-language.md: player.md: States > Not allowed; new 'Stream limits are R1' paragraph; needs table | The 'Not allowed' state (concurrent stream limit) does not apply in R1; it is R2 only, following VID-173. | In R1 the music player explains stream-limit refusals: per guest, per share link and server-wide. Default values remain the owner's (owner decision 25). The early fetch of the next item for gapless playback must count as the same playback. | SEC-TM-068, SEC-API-031, SEC-API-097 | no |
| A-564 | plan/api-needs.md: API-LIB-01 and API-LIB-04 (flows G4) | Who sees a new library unsettled | Owner and administrators only until a grant is made; no member or guest by default | SEC-IAM-070, SEC-TM-026, SEC-IAM-073 | no |
| A-565 | plan/api-needs.md: API-LIB-01, API-LIB-02, API-LIB-05 | Host admin actions | Fresh-uv for adding or removing roots, changing location and file-system browsing | SEC-IAM-041, SEC-API-022 | no |
| A-566 | plan/api-needs.md: API-SES-08 Stream limits and policy refusals | R2 | Per-account and server-wide concurrent-stream limits in R1; policy alternatives R2 | SEC-IAM-102, SEC-TM-068, SEC-API-031 | no |
| A-567 | plan/api-needs.md: API-ADM-01 Local users | Admins make an administrator; ownership handed over by adding an admin and removing yourself | Creating or promoting administrators is owner-only with fresh-uv; ownership moves only by explicit two-party transfer with user verification; exactly one owner always | SEC-IAM-003, SEC-IAM-075, SEC-IAM-041, SEC-OPS-034 | no |
| A-588 | decisions.md: 'Security requirements for the proposed R1'; features/accounts.md: ACC-006 | The proposal moved ADM-102 (stop a session) to R1.2, leaving SEC-IAM-044 with no R1 surface while saying all 583 remaining requirements stay mandatory; 27 other R1 requirements were cited only by moved rows; SEC-STD-008 stayed in R1 with no surface | ACC-006 carries an R1 'End sessions' control on a person's admin page (WP-094 builds and tests it), so SEC-IAM-044 stays in R1 unmoved; the note names the R1 package or row that carries each of the 27; SEC-STD-008 moves with share links to R1.2, so 25 requirements move and 582 remain | SEC-IAM-044, SEC-STD-008, SEC-STD-004 | no |

### T17 The feature map's own key and traceability

2 changes, 0 to confirm. Decision: D-10, D-81. The feature map README's columns, counts and R1 cut, and the Security column that the docs lint reads.

| ID | Where | Before | After | Requirements | Confirm |
|---|---|---|---|---|---|
| A-575 | features/later-media.md: Security cells of LAT-020, LAT-033, LAT-052, LAT-072, LAT-073, LAT-076, LAT-081, LAT-138, LAT-139, LAT-162, LAT-163; the Security cells of 27 other No rows in the music, library, discovery, clients, admin, integrations, video and live TV maps | '—', 'n/a', 'None', 'None (not built)', 'None; not built' and 'None: declined ...' in the Security column | One convention, defined in the README's column table: 'None (No)' for No rows, 'See X' for reference rows (CLI-114 keeps 'See VID-153'), and 'None specific: <reason>' for rows with no control of their own (the LAT sleep timers and audio filters are client-side). The IDs the issue suggested (SEC-CLI-015, SEC-PRV-024) do not apply to a sleep timer or an audio filter, so none was added | SEC-TM-001, SEC-STD-004 | no |
| A-585 | features/README.md: 'The columns', 'Summary by area', 'The R1 cut', opening paragraph | 'Every feature table ... has the same nine columns' with no Security column; summary 1,745 rows and 422 in R1; an R1 cut listing INT-011, INT-012, INT-018 to INT-022, INT-026 and ACC-049 (now R2) and omitting 86 R1 rows | Ten columns, with Security defined; the summary counted mechanically (1,791 rows, 455 in R1, after this run's moves); the R1 cut generated from the R1 rows (401 owning rows and 54 references) with a pointer to D-10's smaller proposal; the README says the baseline's release-scope table wins over it | SEC-TM-001, SEC-TM-074, SEC-STD-004 | no |

## All other decisions

Each entry gives the question, the options, the recommendation and the
sources. Where the alignment already applied the recommendation, the entry
says so and points at the theme in "Applied by default".

### Product, releases and licence

**D-11 The root README's roadmap and diagram.** Options: update now, or
at R1. *Recommendation:* update now: iroh remote access and the
OpenSubsonic adapter inside the R2 milestone; Apple builds, Samsung and LG
packaging, the Jellyfin video adapter and the desktop shell under Later;
and replace the diagram's "Remux in-process" once D-09's record is
accepted. *Sources:* features README 2; security README 9.

**D-12 App Store distribution and the licence.** With no contributor
agreement, an extra permission for app-store distribution cannot be added
once outside contributions arrive. Options: get legal advice now and add a
permission before the first outside contribution; or accept that Apple
builds may never exist. *Recommendation:* advice now; if it supports a
permission, add it first, then move the Apple rows from Later to R2.
*Sources:* features README 3; supply-chain OD-11; clients and video
decisions it cites.

**D-13 A licence exception for third-party WebAssembly plugins.** Options:
first-party plugins only, all AGPL; or a plugin exception added before
outside contributions build up. *Recommendation:* legal advice now; add
the exception only if permissive third-party plugins are wanted; keep
first-party plugins AGPL. *Sources:* features README 11.

**D-14 A native Android music app as an early point release.** Options:
ship it after R1 as a point release; or wait for R2. *Recommendation:*
worth doing, but only with the R2 native-app requirements it needs
(SEC-IAM-048, SEC-CLI-030, SEC-CLI-036, SEC-CLI-046, SEC-STD-039) moved
with it in the release-scope table; schedule it after R1.3, not inside
R1.x. *Sources:* features README 5; clients notes.

**D-15 The desktop shell's technology.** Its release is Later (D-10,
applied). Options: Tauri 2 running the web client with mpv as a separate
sandboxed process; a React Native desktop target (none for Linux,
unverified); the web client only. *Recommendation:* prototype Tauri 2 with
React Native Web and libmpv before promising any date, and bring the shell
forward only together with SEC-CLI-069 and the desktop player sandbox.
*Sources:* features README 21; client OD-12; video notes (open decision
17); api-needs flag 11; UI decision 13.

**D-16 Transcoding, GPU access and FFmpeg packaging (R2).** Options:
software transcoding in the sandbox, off for everyone but the owner, with
hardware transcoding and HEVC or AV1 output Later; or hardware transcoding
in R2. GPU access: off by default, render nodes only, owner-enabled per
device. FFmpeg: a pinned build bundled in official packages, a system
FFmpeg allowed with a `doctor` warning. *Recommendation:* as listed; say
plainly that a small server may manage one 1080p transcode or none. R1 has
no transcoding (D-09). *Sources:* features README 20; security README 11,
25 (GPU); media OD-2, OD-9; threat-model OD-9, OD-11; supply-chain OD-9.

**D-17 Dolby Vision on DV televisions.** libmpv tone maps and never sends a
DV signal. *Recommendation:* spike the Android TV platform decoder early
in R2 and move VID-184 to R2 if one player module can host both engines;
on Apple, plan for Apple's player once Apple builds exist. *Sources:*
features README 19.

**D-18 Watch together.** *Recommendation:* ship the guest capability in R2
(ACC-135) and make watch together the first feature after R2; chat is not
kept after the session (VID-156, applied). *Sources:* features README 18.

**D-19 What comes after video, and podcasts meanwhile.** *Recommendation:*
keep ADR 2's order (live TV in R3, then books and other media types),
start R3 only once the remuxer and sandbox are stable, and document running
Audiobookshelf beside Gunmetal until the podcast plugin exists. Podcasts
now also need their own plugin interface record, an egress inventory row
and per-show host approval by the owner (LAT-058, LAT-180), which makes
members' subscriptions wait for the owner. *Sources:* features README 23,
24; later-media notes.

**D-20 Performance budgets, reference devices and sync sizes.** Options:
accept the proposed goals (home under 200 ms and search under 50 ms at
100,000 tracks on the reference low-end device, initial sync of 100,000
tracks in under 2 minutes, time to interactive under 3 seconds on the
cheapest stick) and name the devices; or set others. For sync size: fetch
a track's seek index when it enters the queue; measure lyrics before
deciding whether to sync them all; sync play aggregates plus a recent
window of history and page older history from the server. In a shared
browser every visit starts with a sync, because the copy lives in memory
only. *Recommendation:* accept, name the devices, and publish the measured
numbers either way. *Sources:* features README 16; WP 15; api-needs flag
13 and "The sync model".

### Remote access and network

**D-21 Remote access for browsers, and iroh.** Options for R1: the owner's
reverse proxy or tailnet only (some owners will port-forward, which the
internet posture and exposure alerts are built for); or a TLS-passthrough
project edge in R1. For R2: the edge, relays and address lookup run by the
project only if funded, with a published log policy (connection metadata
only, at most 30 days); self-hosted relays first-class; iroh port mapping
off by default with a one-click offer. *Recommendation:* proxy or tailnet
in R1; edge, relays and the browser path in R2 (ACC-102, applied), and
self-hosted relays first-class. *Answer (2026-10-02):* the owner's reverse
proxy or a tailnet in R1; built-in remote access (iroh), relays and the
edge in R2 (see [Owner answers](#owner-answers-2026-10-02)). *Sources:*
security README 3; features README 10; network OD-2, OD-3, OD-4, OD-5;
threat-model OD-12; privacy OD-11; api-needs flag 10.

**D-22 Network defaults.** One confirmation covers: the home posture as a
hard default in every package (operations OD-2); untrusted forwarding
headers ignored with internet posture, now fixed by SEC-NET-017 (network
OD-7); trusted proxies as a manual list with recipes, presets for known
tunnels later (web OD-10); a redirect-only plaintext listener and default
ports that do not clash with Plex, Jellyfin or Emby (network OD-9); an
opt-in external exposure check run from the project edge (network OD-10);
a 5-second revocation target (network OD-12); no SSDP or DLNA before its
own record (rival OD-9). *Recommendation:* accept all as written. Note for
the owner: in home posture, uptime monitors outside the LAN see the help
page, not `/healthz`. *Sources:* security README 25; the files named.

**D-23 Phones on the home Wi-Fi with global IPv6 addresses, and names
during an internet outage.** SEC-NET-024 treats only private, shared,
unique-local and link-local ranges as local, so a phone using a global IPv6
address on the same Wi-Fi gets the help page. Options: leave it; or also
treat addresses inside the host's own on-link prefixes as local, which a
remote attacker cannot complete a handshake from. Separately, per-server
names may not resolve while the internet is down (unverified).
*Recommendation:* adopt the on-link rule through a change to SEC-NET-024,
and test name resolution during an outage before launch. *Sources:*
operations OD-15; flows gaps G19 and G20.

**D-24 A separate origin for the admin interface.** Options: yes for
per-server names and documented for own-domain setups; or one origin.
*Recommendation:* yes, as network OD-6 says; one extra passkey tap is the
cost. *Sources:* network OD-6; surfaces open question 10; security README
25 (admin origin).

### Accounts, devices and sessions

**D-25 Profiles, PINs and parental filters.** Options: R1 or R2.
*Recommendation:* R2 with household devices; in R1 a child uses a separate
account limited to chosen libraries, and the R1 data model keeps
credentials apart from profiles (ACC-017). *Sources:* security README 17;
identity OD-4; client OD-10.

**D-26 Household devices.** *Recommendation:* home network only by
default; dormant after 30 days unused and woken with one tap; software
keys allowed but capped below administrator (applied in ACC-021).
Identity OD-7's older 90-day expiry is replaced. *Sources:* security
README 16; identity OD-7, OD-8.

**D-27 Device and stream limit defaults.** Limits exist in R1 (SEC-IAM-102,
SEC-TM-068, applied). Options for the defaults: 25 devices per member and
5 per guest, 3 concurrent streams per guest, no per-member stream limit; or
other values. *Recommendation:* those values, adjustable by admins, with
the gapless early fetch of the next track counted as the same playback.
*Sources:* identity OD-13; security README 25; UI decision 16; player
notes.

**D-28 Profile PIN lockout.** Options: lock after 5 failures until a
parent unlocks it (web OD-9); or growing delays per device and profile up
to 15 minutes, never a permanent lock, with an alert after 10 failures
(SEC-API-056, applied in ACC-020). *Recommendation:* the delays.
*Sources:* web OD-9; security README 25 (PIN back-off); applied theme T10.

**D-29 Step-up on phones: biometric only, or device PIN too.**
*Recommendation:* allow the device PIN, because many people never enrol
biometrics and the server-side limits still apply. *Sources:* client OD-9;
security README 25.

**D-30 Offline grants and downloads (R2).** *Recommendation:* grants last
30 days, renewed on contact, admin range 1 to 90; downloads in app-private
internal storage relying on the platform's encryption, with revocation on
next contact; removable storage Later and only in an encrypted, seekable
format (CLI-059, applied). *Sources:* client OD-2, OD-3; threat-model
OD-13; privacy OD-10; security README 25.

**D-31 The default answer to "personal or shared computer?".** Options:
preselect personal (client OD-4); default to shared, memory only (privacy
OD-9); or two explicit answers with nothing preselected (applied in
CLI-155). The baseline also disagrees with itself: SEC-TM-058 never allows
history in browser storage while SEC-PRV-019 allows Activity data on a
personal device, and SEC-CLI-010 keeps even the library in memory in
shared mode while the privacy guidance allows the catalogue.
*Recommendation:* no preselection, as applied; history never in browser
storage (the stricter rule, which the maps follow); and have the baseline
reconcile the two pairs. *Sources:* security README 25; client OD-4;
privacy OD-9; clients and discovery notes.

**D-32 Federation, LDAP, proxy-header sign-in and one sign-in across
servers.** *Recommendation:* none of them without its own architecture
record; point directory users at an OIDC provider that fronts the
directory (ACC-059 and ACC-060 are now No, applied); design multi-server
sign-in as a device key trusted by several servers, Later. *Sources:*
features README 22; identity OD-10, OD-11; security README 25.

**D-33 One name for the host recovery command.** The baseline uses
`gunmetal recover-owner`, `gunmetal owner recover` and ADM-034's
`gunmetal admin recover`. *Recommendation:* pick one; the flows use
ADM-034's. *Sources:* UI notes.

### Privacy

**D-34 What admins see of other people's activity.** Options: (a) who is
playing, on which device, bitrate and method, with the title only if that
person opts in, never history, no household activity features, each
admin look recorded in the person's own log (baseline, applied across the
maps); (b) titles shown unless the session is private (privacy OD-1, the
privacy design guidance); (c) live sessions with titles, totals, and full
history for managed children or adults who opt in (the feature map README's first recommendation for its item 13, which now gives (a)).
Guardians of managed profiles (R2) see those profiles' history.
*Recommendation:* (a). Its largest cost: Tautulli-, Jellystat- and
Tracearr-style per-person dashboards no longer work (INT-100, INT-129,
INT-133, INT-144). The privacy guidance and privacy OD-1 should be
corrected to match SEC-PRV-025. *Sources:* security README 5; features
README 13; WP 30; identity OD-5; privacy OD-1; threat-model OD-6;
integrations notes; applied theme T04.

**D-35 Private listening: default and end.** Options: off by default and
one tap away (features README 14); end it after a period without playback.
*Recommendation:* off by default, at most two interactions from any
player, clearly shown while on, ending after 6 hours without playback by
default (from the privacy guidance, which has no requirement or parameter
row yet; add one, D-81). *Sources:* features README 14; WP 27; player and
api-needs notes.

**D-36 Retention defaults.** Options: security events 365 days, addresses
coarsened at 30 days and removed at 90, the owner seeing other people's
addresses shortened, history kept until the person deletes it with 90-day,
1-year and 2-year choices (baseline); or shorter. The "until deleted"
history default goes against SEC-PRV-023 ("every privacy setting defaults
to its most private value"). *Recommendation:* the baseline values, and
record "until deleted" as an explicit, owner-approved exception to
SEC-PRV-023 (applied in MUS-234 and DIS-187, marked to confirm). The audit
log keeps addresses outside the hash chain so coarsening never breaks
`audit verify` (applied in WP-069). *Sources:* security README 15;
identity OD-6; operations OD-6; privacy OD-8; music and discovery notes.

**D-37 Household activity features.** "Popular in this household", the
household blend (DIS-184), household radio (MUS-165) and top lists.
Options: none (security README 5); opt-in per person, counting only items
at least three opted-in people played, not in R1 (privacy OD-13).
*Recommendation:* none in R1; later only as per-person opt-ins with the
three-person threshold, never for managed profiles or private sessions,
and add the threshold to the parameters table. Applied as none in R1 or
R2: DIS-060's co-listening, DIS-184 and ADM-106's popular list are Later
with DIS-078 and DIS-139, and ACC-114 says so (A-572); choosing R2 for
any of them reverses that change. *Sources:* security README
5; privacy OD-13; ACC-114; DIS-078, DIS-139, DIS-184; MUS-051, MUS-165;
integrations notes.

**D-38 Telemetry and crash reports.** *Recommendation:* none in R1 and R2,
no automatic crash reports (ADM-131 is now No), diagnostic bundles
downloaded by the admin; revisit only with a public proposal on the Go
toolchain model. The feature map's "opt-in crash sending after R1" is
replaced. *Sources:* security README 24; features README 15; privacy OD-4;
accounts and admin decisions it cites.

**D-39 Whole-database encryption, and history the admin cannot read.**
*Recommendation:* no whole-database encryption in R1 (secrets and backups
are encrypted; recommend full-disk encryption); per-profile history
encryption only as research, Later. *Sources:* privacy OD-6, OD-7;
security README 25.

**D-40 Account deletion and paid sharing.** *Recommendation:* adults delete
their own accounts with a 7-day grace period and no admin approval
(ACC-136, applied); never add billing or "split the cost" features.
*Sources:* privacy OD-12, OD-14; security README 25.

**D-41 Legal position and a legal home.** Is Premier Studio a manufacturer
or an open-source steward under the EU Cyber Resilience Act (manufacturer
reporting has applied since 11 September 2026)? Who holds project services
legally? *Recommendation:* stay non-commercial for now and record it; get
legal advice before the name service launches, before any paid builds,
hosted relays or feature-tied sponsorship; give project services a legal
home before R1 ships the update check. *Sources:* security README 21;
operations OD-10; privacy OD-15; supply-chain OD-13.

**D-42 Personal data the baseline does not yet cover.** (1) GPS and other
location metadata in home videos and phone-recorded MP4 or QuickTime files
(photos are covered by LAT-179); options: drop it at scan, or keep it
admin-only. (2) Acoustic fingerprints for LIB-106: SEC-PRV-014's evidence
type has no fingerprint field; options: add one by recorded decision, or
make LIB-106 No. (3) Personal recordings, series rules, reminders and
favourites in live TV are treated as Activity data; SEC-PRV-001's
inventory should say so. *Recommendation:* drop video location at scan
unless the owner wants it; add a fingerprint field only with a consent
text naming it; classify the live TV data explicitly. *Sources:* library
and live TV notes.

### Library, media and files

**D-43 Never write into media folders.** Options: never in R1 or R2,
revisiting a per-root write grant with a trash only after R2 through a
record that replaces SEC-TM-042 for that root; or an opt-in writable root
for deletion (the old ADM-139 and the first version of features README 17, which now points here). *Recommendation:*
never in R1 or R2, as applied: "Remove" hides and trashes an item and the
owner deletes files with their own tools; recordings, podcast archives and
trailers go to the server's own stores. *Sources:* security README 23;
features README 17; media OD-14; operations OD-11; threat-model OD-15;
applied theme T07.

**D-44 Provider API keys.** Options: ship a project key where a provider's
terms allow it, treated as a public client identifier (privacy OD-3); or
no project key in any binary, each owner entering their own (applied in
LIB-109 and the integrations map, SEC-TM-012). *Recommendation:* no project
key. *Sources:* privacy OD-3; security README 22; library open decision 7;
integrations open decision 5.

**D-45 Symbolic links that leave a library.** *Recommendation:* the
approved-target model: links leading outside are skipped and listed, and
an admin can allow a target folder in one step under a passkey check
(LIB-204, applied). *Sources:* media OD-4; security README 25.

**D-46 Images: formats, originals, colour profiles and the cost of
re-encoding.** *Recommendation:* no new image formats in R1, and any later
one decoded only in the jail; no original-artwork download in R1; ignore
ICC profiles in R1; always re-encode for clients, including every comic
page and HEIC photo later, accepting the CPU cost and the loss of HDR
photos (LAT-117, LAT-166, applied). *Sources:* media OD-11, OD-12, OD-13;
threat-model OD-7; security README 25; later-media notes.

**D-47 Subtitles: embedded fonts, ASS styling and sidecar folders.**
Options for fonts: on by default (client OD-7) or off by default with a
per-library opt-in and memory-safe rewriting (security README 12, applied).
For ASS: SEC-HIS-039 and SEC-API-089 turn subtitles into text cues, while
SEC-MED-052 and SEC-CLI-053 expect styled ASS; the video map applies an
allowlisted ASS re-serialisation as a middle path. For sidecars:
SEC-MED-053 allows only the video's own folder, which drops `Subs`
subfolders (Jellyfin #2562). *Recommendation:* fonts off by default;
confirm the allowlisted re-serialisation and have the baseline say so;
consider allowing a fixed list of subfolder names under the same root.
Also add libmpv's `embeddedfonts=no` to the owning requirement.
*Sources:* security README 12; client OD-7; video open decision 16;
library and video notes.

**D-48 Which containers native players get directly.** *Recommendation:*
only containers the core parsed at scan; others are remuxed or transcoded
unless an admin allows direct play for that library (CLI-047, VID-001,
VID-186, applied). *Sources:* client OD-5; security README 25.

**D-49 Playlists from folder files, and missing entries.**
*Recommendation:* playlists found in music folders are read-only with
"Duplicate to edit"; a purged track stays in a playlist as a "missing"
entry that rematches by identity. *Sources:* WP 29; api-needs API-PL-08;
flows gaps G5 and G6.

**D-50 Who sees a new library.** *Recommendation:* the owner and
administrators only, until someone is granted access (applied in
api-needs). *Sources:* WP 19; flows gap G4.

**D-51 Live TV sources (R3).** (1) Many XMLTV files start with a DOCTYPE,
which SEC-MED-056 and SEC-HIS-034 reject; options: refuse them, or skip a
DOCTYPE with no internal subset unread (needs the requirement changed).
(2) Internet IPTV sources and radio stations must use HTTPS under
SEC-API-078, which may exclude many providers (unverified), while
SEC-API-082 and SEC-HIS-025 accept http. (3) Exporting a lineup to TiviMate
or Kodi needs a playback-only credential in a URL, which no requirement
allows. *Recommendation:* decide (1) and (2) before R3 work starts; keep
(3) Later behind a record. *Sources:* live TV open decisions 13, 16, 17;
live TV notes.

**D-52 Book and podcast protocols (Later).** OPDS, KOReader, podcast-sync
apps and private book feeds need credential kinds that SEC-EXT-007's closed
list does not have; OpenSubsonic bookmarks are not in SEC-EXT-056's grants;
Kobo sync may need a key in the URL, which the baseline forbids.
*Recommendation:* add the credential kinds and adapter rules before those
adapters are planned; make Kobo sync No if the device cannot send a header.
*Sources:* later-media notes and open decisions 4 and 10.

**D-53 Patch windows and fuzzing.** *Recommendation:* fix within 14 days of
an FFmpeg, libass, FreeType or HarfBuzz security release affecting an
enabled component, and 72 hours for anything in CISA's Known Exploited
Vulnerabilities catalogue; plain-function harnesses with `cargo-fuzz` in
ClusterFuzzLite on a pinned nightly, `afl.rs` locally, OSS-Fuzz after R1
has users. *Sources:* media OD-10, OD-15; security README 25.

### Integrations and plugins

**D-54 The OpenSubsonic adapter and legacy sign-in.** Options: R1 or R2;
API keys only, or a per-key legacy password (`p`) for older apps, or the
MD5 token-and-salt scheme; a plain-HTTP option on the LAN or HTTPS only.
*Recommendation:* R2, off until the owner enables it, its own port,
API keys plus a per-key legacy mode (home network only, 90 days, alert on
first use from a new network), token-and-salt refused, HTTPS only (INT-088
moved from Later to R2, applied). Which Subsonic apps support API keys is
unverified. The features README's "legacy sign-in Later" is replaced.
*Sources:* features README 4; security README 8; plugins OD-2, OD-3,
OD-4; rival OD-3; api-needs flag 14; integrations, accounts and music
notes.

**D-55 API keys and the two admin-granted scopes.** API keys arrive in R2
with no administrator, owner-only or host-equivalent scope (SEC-EXT-010),
which removes the scripting API from R1 (applied). Open interpretation:
the scope vocabulary has two admin-granted scopes, `admin:library`
(refresh) and `admin:status`. Options: allow those two narrow scopes, which
the *arr refresh recipes need; or read SEC-EXT-010 strictly and give
download tools a non-admin "refresh hint" scope that only makes the poller
look sooner. *Recommendation:* the strict reading with a refresh-hint
scope, reviewed into the vocabulary, because the baseline forbids admin
scopes on keys and an admin automation credential needs its own record;
until then LIB-026 waits (Later, applied). *Sources:* security README 8;
plugins OD-11; integrations open decision 13; library and integrations
notes.

**D-56 The plugin host and scrobblers.** *Recommendation:* R2, each plugin
in its own OS-sandboxed process, no WebAssembly runtime linked in R1;
Last.fm and ListenBrainz send only plays after the person links (or a
backfill range they pick), so plays recorded in R1 are not sent unasked.
The feature map's "R2 scrobblers submit what the services still accept" is
replaced. The owner registers the server's own Last.fm and Trakt client
credentials; no project secret ships. *Sources:* features README 6;
plugins OD-1; MUS-194; INT-054, INT-102; integrations open decision 5.

**D-57 Plugin updates, the index and publishers.** Options: automatic
updates by default for unchanged permissions (plugins OD-7) or owner
approval with an opt-in per plugin after 72 hours; index may disable
revoked versions (plugins OD-6) or only advise; third-party plugins behind
a per-plugin override (threat-model OD-14) or only from a signed index the
owner adds. *Recommendation:* owner approval with the 72-hour opt-in;
advise only, never disable; plugins only from signed indexes, unsigned
ones only in developer mode set in the config file; one sandboxed process
per plugin, unavailable where no OS sandbox exists; a narrow egress
interface for plugin HTTP in R2; the publisher policy of plugins OD-12.
*Sources:* security README 13, 25; plugins OD-5 to OD-9, OD-12;
threat-model OD-14.

**D-58 Webhooks, outbound alerts and push.** Options: webhooks in R2 with
per-user webhooks to public HTTPS hosts and admin webhooks to LAN hosts
(plugins OD-10); or owner-enabled webhooks to listed hosts only, a private
address only as an exact owner-entered host and port (applied). Outbound
alert channels: in-app only in R1; ntfy and webhooks R2; email Later.
Push: no project-run relay holding device tokens; only an owner-run relay
carrying an opaque ID, Later. *Recommendation:* as applied. *Sources:*
plugins OD-10; operations OD-14; api-needs flag 9; security README 25;
CLI-077, INT-046, INT-049 to INT-051.

### Operations, releases and people

**D-59 The update and advisory check.** Options: a required first-run
question with two explicit answers and nothing preselected (SEC-OPS-047,
applied); on by default (rival OD-4, threat-model OD-5); opt-in (privacy
OD-5, features map). No self-updater; automatic updates only through the
OS package manager or a container update tool. *Recommendation:* the
required question, no self-updater. *Sources:* security README 4; operations
OD-3; rival OD-4, OD-10; privacy OD-5; threat-model OD-5.

**D-60 Running as root.** Options: refuse with no override (applied); an
override flag; start as root and drop privileges (threat-model OD-4).
*Recommendation:* refuse, no override, templates set a non-root user.
*Sources:* security README 14; operations OD-12; threat-model OD-4.

**D-61 Backups: the recovery key and the server identity key.** Options:
generate the recovery key in the owner's browser and keep no private half
on the server, offering "make a new kit" instead of "show it again"
(operations OD-4); or let the owner show the kit again after a fresh check
(applied in ADM-143, following SEC-PRV-040). Whether backups contain the
server identity key: yes, encrypted, so clients reconnect without pairing.
*Recommendation:* settle SEC-PRV-040 against operations OD-4 in the
baseline; include the identity key. *Sources:* operations OD-4, OD-5;
security README 25.

**D-62 People and keys.** *Recommendation:* find a second maintainer before
R1 to review security-sensitive changes, approve releases and hold one TUF
root key and the name-service keys; sign release artefacts keyless with
Sigstore and SLSA provenance (applied in WP-136) and the TUF roles with
offline hardware keys, threshold 2; deposit a sealed root key with a
trusted third party; get a non-author design review before the R1 tag and
seek funding for an external assessment before R2. *Sources:* security
README 19; features README 25; WP 32; operations OD-13; supply-chain
OD-1; standards OD-4.

**D-63 Disclosure.** *Recommendation:* GitHub private reporting plus a
`security@` alias reaching two people, a signed `security.txt`, published
response targets, CVEs for every fix through GitHub's CNA, a 90-day maximum
embargo, credit and a hall of fame, no paid bounty until funded, "latest
release only" support until 1.0, a private packager list. *Sources:*
security README 20; operations OD-7, OD-8, OD-9; supply-chain OD-16; rival
OD-8.

**D-64 Verification targets.** Options: ASVS Level 3 except chapter 2
(Level 2) and WebRTC (security README 18); or Level 2 with named Level 3
items (web OD-12). MAS-L2 and MAS-P for native apps, MAS-R excluded.
*Recommendation:* the README's targets. *Sources:* security README 18;
standards OD-3, OD-5; web OD-12; client OD-14.

**D-65 Supply-chain choices.** *Recommendation:* pnpm for its release-age
gate; GHCR as the only registry at first; a fixed non-root container user;
cargo-vet now; signed commits on `main`; R1 ships static Linux binaries
and the container image with checksums, provenance and SBOMs (no deb, rpm,
Windows, macOS or NAS packages yet); reserve crate and npm names but
publish nothing; version shown only to signed-in users; agents and builds
without access to keys or tokens; the native `--network=none` build step
for CI egress. *Sources:* security README 25; supply-chain OD-2 to OD-8,
OD-12, OD-14, OD-15, OD-17.

**D-66 Native app policy (R2).** *Recommendation:* fund a spike on
Android `isolatedProcess` for libmpv before R2; distribute through Google
Play, F-Droid and signed GitHub APKs, registering under Google's developer
verification before the 2027 rollout; publish minimum OS versions set by
the controls; never ship over-the-air updates; no root, jailbreak or tamper
detection, every control enforced on the server; record the client trust
model as an architecture record. *Sources:* client OD-6, OD-8, OD-11,
OD-13, OD-14, OD-15; supply-chain OD-10; security README 25.

**D-67 "Install with one command".** SEC-MED-063 forbids the server from
starting programs, so it cannot create its service user. *Recommendation:*
the binary prints the unit file and a shipped install script does the
privileged steps; that script is ADM-005's one command. *Sources:* WP 35.

**D-68 The HTTP stack and the R1 event channel.** Options: axum on tokio
with server-sent events for R1 (one-way, so "Play here" is an ordinary
request) and WebSockets from R2; or a WebSocket channel in R1 (api-needs
API-SYS-10). *Recommendation:* server-sent events in R1, with a test that
no upgrade is accepted. Clients batch position writes, because one writer
must never take a write per second per session. *Sources:* WP 7; api-needs
API-SYS-10 and flags 12 and 15.

**D-69 The benchmark library.** *Recommendation:* the synthetic generator
in CI for regressions, plus a published comparison the owner runs on a real
library, describing its shape but sharing no files. *Sources:* WP 22.

### Interface and client behaviour

**D-70 Two players on one queue in R1.** The plan (WP 14) recommends an
active-device field on the queue, while the UI documents settled on the
flows' mechanism: Play and resume are ordinary queue operations that
record the issuing session by an opaque ID, the last Play wins, the other
session pauses and shows "Continue on this device", and there is no
separate active-device record (UI decisions 8 and 9, owner to confirm).
Also: do a person's picks survive a new Play (WP 28)? *Recommendation:*
the UI documents' mechanism, with WP-025 and api-needs API-SES-03 updated
to drop the active-device field; picks survive a new Play. *Sources:* WP
14, 28; UI decisions 8, 9; api-needs API-SES-03; player open question 2;
flows gap G11.

**D-71 What R1 does with writes while offline.** *Recommendation:* only
plays and positions are queued, in memory; loves, ratings and playlist
edits are disabled with "Needs the server", and the event format already
lets R2 queue them. *Sources:* WP 16; api-needs "How changes flow from the
device to the server".

**D-72 How synced data resolves conflicts.** *Recommendation:* accept the
api-needs table: plays are a union by event ID; erasure wins whatever order
events arrive in; loves and ratings take the latest clock; playlist entries
have their own IDs; whole-field last-writer-wins for names, rule trees and
Home layouts; household curation is admin-only and online. *Sources:*
api-needs "How conflicts resolve".

**D-73 Keyboard shortcuts in R1.** The search shortcut is R2 (UI decision
6), and until the owner decides no shortcut ships in R1. Options: none in
R1; or the player's core keys. *Recommendation:* ship Space, Shift+Left,
Shift+Right, Shift+N, Shift+P and M in R1 as part of MUS-227's keyboard
promise, with the off switch WCAG requires; the palette, "/" and the rest
stay in R2. *Sources:* player open question 1; UI decision 6.

**D-74 The content security policy and React Native Web.** Whether React
Native Web's run-time styles work under `style-src 'self'` is unverified.
Options: try `style-src 'self'` (with a hash if needed), falling back to
`'unsafe-inline'` for styles only and never for scripts; no CSP reporting
endpoint in R1, relying on the end-to-end CSP test. *Recommendation:* as
listed, and decide the fallback once the spike in WP-072 runs. *Sources:*
web OD-6, OD-7; design-language open question 8; api-needs flag 11.

**D-75 The server's name before sign-in, and the invitation page.**
*Recommendation:* no name, message or version before sign-in; the name is
shown after sign-in, remembered by the client, and on the invitation page,
which the server itself serves and which runs no script that reads the
secret (ADM-140, ACC-082, applied). *Sources:* web OD-5, OD-13; security
README 25.

**D-76 Cross-origin reads for cast representations (R2).**
*Recommendation:* allow them for cast representations only (SEC-API-098).
*Sources:* web OD-11; security README 25.

**D-77 Interface details the documents left open.** None changes a
feature's scope; each has a recommendation in its document, which this
register adopts: new plays keep the person's picks and replace only the
From lane; OK on a TV remote shows the controls without pausing; next and
previous mean chapters on a TV remote during video (long press for the
next item) and items on lock screens; handing off to a TV asks first, with
a per-TV setting for the person's own devices; skip markers show a button
by default; shuffle defaults to "Spread out"; non-admins see codecs and
reasons but never paths, addresses or other people's sessions in the
statistics overlay and track info (SEC-API-068, SEC-PRV-025); test the four
width classes on real phones and tablets before pinning them; live TV gets
a fourth phone tab only when the module is on; "All" on the combined Home
is a setting; the pre-play sheet opens only when there is a choice or a
warning; the web profile picker appears at launch only on shared or
household devices; the command palette and search share one index.
*Sources:* player open questions 3 to 9; surfaces open questions 1 to 3,
5, 6 and 8.

### Corrections to the baseline itself

The alignment found places where baseline rows disagree with each other or
with their own guidance. The planning documents followed the stricter or
the owning requirement in each case. These are decisions for the owner of
the baseline; nothing in docs/security was changed.

**D-78 Values that disagree.** Beyond D-03:

| Control | Disagreement | Recommendation |
|---|---|---|
| Cast and transfer URL lifetimes | SEC-NET-064: item length plus at most 1 hour; SEC-API-027: plus 10 minutes, at most 4 hours | One owning row in the parameters table; SEC-API-027's value |
| Artwork URL expiry | Web guidance: round up to 6 hours; SEC-API-027: 1 hour aligned to a time bucket | SEC-API-027; fix the guidance |
| Where stream capabilities travel | Privacy guidance section 11 and SEC-EXT-006: a query parameter; SEC-API-026: the URL path | The path; fix SEC-EXT-006 and the guidance |
| Cookie for media | Client guidance section 4: cookie as well; SEC-API-029: capability only | SEC-API-029 |
| Setup-code attempts | SEC-OPS-004: 10 a minute server-wide; SEC-IAM-008: no server-wide limit one source can exhaust, loopback never delayed | SEC-IAM-008 |
| Claim code on restart | Operations guidance: a new code each restart, no loopback exemption; SEC-IAM-007 and SEC-IAM-008: 24 hours across restarts | The requirements |
| Outside links | SEC-CLI-002: https only; SEC-API-047, SEC-MED-058, SEC-STD-015: http and https | https only, which satisfies all four (applied in design-language) |
| libmpv options | SEC-MED-076 and SEC-CLI-048 list different options; neither has `embeddedfonts=no`; SEC-MED-076 lacks `audio-file-auto=no` and `cover-art-auto=no` | One list in one row |
| Who grants network destinations | SEC-TM-017: owner with a fresh check; SEC-API-078, SEC-API-079, SEC-NET-067, SEC-HIS-025, SEC-EXT-075 say "admin" | Owner, as applied in live-tv.md |
| Plain http to sources | SEC-API-078: only to admin-entered LAN targets; SEC-API-082, SEC-HIS-025: http accepted | SEC-API-078 (see D-51) |

**D-79 Rules that disagree.** Each line ends with the rule the planning
documents followed; the baseline should make it the only one.

- History in browsers: SEC-TM-058 (never) against SEC-PRV-019 (personal
  devices may keep Activity data). Followed: SEC-TM-058 (D-31).
- Library in a shared browser: SEC-CLI-010 (memory only) against privacy
  guidance section 11 (the catalogue may stay). Followed: SEC-CLI-010.
- Titles in admin views: privacy guidance section 4 and privacy OD-1
  (shown unless private) against SEC-PRV-025 (only with opt-in). Followed:
  SEC-PRV-025 (D-34).
- Webhooks: SEC-API-083 (admin-only, Later, records a status code) against
  SEC-EXT-045 and SEC-EXT-048 (R2, per-user, success or failure only).
  Followed: SEC-EXT-045 and SEC-EXT-048.
- Admin scopes: the plugins guidance's `admin:library` scope against
  SEC-EXT-010 (no administrator scope on any key). Open (D-55).
- Revoked plugin versions: plugins guidance (the server disables them)
  against SEC-EXT-041 (advise only). Followed: SEC-EXT-041.
- Unsigned plugins: SEC-SUP-066 (an admin override) against SEC-EXT-037
  (configuration file only). Followed: SEC-EXT-037.
- API keys: the plugins file summary ("from R1") and the identity principal
  table ("Later") against SEC-EXT-008 to SEC-EXT-017 (R2). Followed: R2.
- Plain HTTP for adapters: the plugins file's one-click LAN option against
  SEC-EXT-066 and SEC-NET-001. Followed: SEC-NET-001.
- Embedded fonts: client OD-7 (on by default) against security README 12
  and SEC-MED-054 (off). Followed: off (D-47).
- ASS styling: SEC-MED-052 and SEC-CLI-053 (styled ASS reaches clients)
  against SEC-HIS-039 and SEC-API-089 (text cues only). Followed: an
  allowlisted re-serialisation, to confirm (D-47).
- The web client as an audit anchor: operations guidance (not an auditor)
  against SEC-OPS-075 (an anchor off the host). Followed: SEC-OPS-075
  (ADM-145).
- Addresses in diagnostic logs: SEC-PRV-003 (only sessions and the audit
  log) against SEC-OPS-028 (fail2ban lines). Followed: both, with full lines
  only on the host; the baseline should state the exception.
- Public links, adapters and keys: the identity file's scope note says
  Later, while SEC-API-097 and the release scope say R1 and R2; SEC-IAM-084
  to SEC-IAM-087 are Later although their surfaces are R2. Followed: the
  release scope.
- Stale open decisions: threat-model OD-2, OD-3, OD-5 and OD-6, and privacy
  OD-5 and OD-13, still describe proposals the README overrode. Followed:
  the README.
- DLNA: SEC-NET-066 allows a LAN-serving plugin, which sits uneasily with
  principle 3. Followed: no DLNA before its own record, which must say how
  a renderer is authorised beyond being on the LAN.
- User agent: SEC-EXT-005 allows a header a destination requires, which
  IPTV sources use, while SEC-PRV-017 sends a project-only user agent.
  Followed: SEC-PRV-017 for metadata providers only; confirm that scope.

**D-80 Release values out of step with the release-scope table.**

- SEC-MED-081, SEC-MED-032, SEC-MED-074 are R2, but the audio packager runs
  in a worker in R1 (D-09).
- SEC-CLI-071 (cast credentials) is Later while casting is R2; the maps use
  sender-refreshed capability URLs only. SEC-NET-064 and SEC-API-098 are R2.
- SEC-CLI-072 (encrypted removable storage) is Later; choosing a download
  location moved to Later with it (applied).
- SEC-PRV-056 and SEC-OPS-036 (push payloads) are R2, but push is not in
  the release-scope table and is Later in the maps.
- SEC-NET-054, SEC-OPS-040, SEC-PRV-034 to SEC-PRV-036 (D-10).
- The release-scope table lists artwork uploads in R1 (LIB-138 is R2),
  lyrics providers in the R1 egress inventory (MUS-162 is R2), and iOS and
  tvOS in R2 (Apple is Later in the maps until D-12); SEC-CLI-051 runs
  sanitizer tests on a "Linux desktop" in R2 though the shell is Later.
- The feature map README listed the desktop shell and share links under
  R2 and said it wins on release conflicts; it now lists video share links
  in R2 and the shell in Later, and says the release-scope table wins
  (SEC-TM-074; A-589).

*Recommendation for D-78 to D-80:* the security lead makes each fix in
docs/security, with the docs lint (SEC-TM-072 to SEC-TM-075) extended to
catch the value pairs.

**D-81 Missing parameters, inventory rows and requirements.**

- Parameters with no row in the parameters table: the 6-hour end of a
  private session; the three-person household threshold; the 30-minute
  idle end of a shared browser (SEC-CLI-010); the lifetime of an admin's
  recovery link (SEC-IAM-091 says only "short-lived").
- SEC-API-002's literal R1 allow-list lacks the share-link landing page and
  recovery redemption (api-needs lists them).
- Egress inventory rows missing for off-site backup destinations (ADM-073),
  API import from a rival server (ADM-047), log forwarding (ADM-135),
  HDHomeRun discovery (LIV-024), the Audiobookshelf importer (LAT-054) and
  podcasts; each needs a row before it ships (SEC-TM-075, SEC-NET-032).
- SEC-OPS-063 names recordings only; LIV-095 also puts time-shift buffers
  and the recording trash in the recordings root.
- SEC-TM-001's lint wants trust boundaries and threats per feature entry;
  the maps give them once per file in "Security notes". Decide per file or
  per row.
- SEC-STD-022 bans seeded generators; the keyed-hash ordering used for
  shuffles (DIS-122) should go into the secure-coding guide.

*Recommendation:* add them. *Sources:* the agents' notes on every map.

### Document alignment

**D-82 Approve the follow-up edits in documents this run did not change.**
These need no new choice, only a go-ahead for whoever owns each file:

- Done in the follow-up review (A-568 to A-590): the feature map README's
  summary counts, R1 cut, ten columns with Security, Releases table and
  open decisions 4, 6, 7, 8, 9, 13, 15, 17 and 21; WP-003's draft of ADR 4
  (now a worker); and the TV admin surfaces of LIB-172, LIV-002 and
  LIV-059.
- ADR 1 decision 3 and the root README diagram still say the remuxer runs
  in-process (D-09); they are outside the planning documents.
- api-needs.md API-SES-03 still offers two mechanisms (D-70).
- INT-006 is R1 but its only surface (token detail) is R2. Resolved
  2026-10-03: INT-006 is now R2 (D-10).
- ADM-030 shows a dry-run report in R1 that depends on ADM-045 (R2).
  ADM-030 is now R1.1 (D-10); the dependency on ADM-045 (R2) remains.
- ACC-084 duplicates ADM-048; ADM-123 and ADM-133 should point at LIB-206
  for the scan worker's isolation status.
- ADM-102's stop message must be plain text in its own field (SEC-HIS-014).
- MUS-045 should import ratings per person only; MUS-140's tag and
  MusicBrainz matching applies only to imported playlists, never to
  playlists found in a library (SEC-MED-050).
- LIB-056 is R1 but its owner MUS-014 is R2. Resolved 2026-10-03: LIB-056
  is now R2 (D-10).
- No package wires the free-space guard (WP-097) into the writers that
  merge earlier; ADM-011, CLI-002 and the release notes have no owner in a
  backend plan.
- The TV rail's Now Playing entry and TV lyrics have no feature row.
- docs/ui/README.md could mention the new security states in
  design-language.md section 11.

*Sources:* the agents' notes; work-packages.md "What this review could not
resolve"; surfaces open question 4.

## Release-ordering questions, 2026-10-03

Applying the smaller R1 left some R1 rows depending on features that now
ship in a point release. Each question below has a recommendation. The
owner accepted all six on 2026-10-03, and the documents now follow them,
as the line under each says.

### D-83 Track details in R1

- **Question.** MUS-114, the track info sheet, is R1.1, but ten R1 rows
  show their details there: MUS-021, MUS-032, MUS-034, MUS-036, MUS-037,
  MUS-067, MUS-069, MUS-084, MUS-089 and MUS-099.
- **Recommendation.** R1 ships a minimal, read-only details view: title,
  credits, album, file format, and the playback decision with its reason.
  The full sheet stays in R1.1.

Answered 2026-10-03: recommendation accepted and applied.

### D-84 Uncertain matches before the review queue

- **Question.** The review queue (LIB-099) is R1.3, but LIB-028, LIB-030
  and LIB-051 (R1) and LIB-111 (R1.1) send uncertain matches to it.
- **Recommendation.** Until R1.3, nothing merges silently: an unconfirmed
  match arrives as a separate item, uncertain albums stay apart, and
  low-confidence lookups are kept as suggestions and not applied. This is
  already written into library.md.

Answered 2026-10-03: recommendation accepted and applied.

### D-85 When the rule format ships

- **Question.** The rule language (DIS-119, MUS-143) is R1.3, but saved
  filters (DIS-105, R1.1) and the home you arrange (DIS-003, MUS-049, R1.2)
  are stored in it.
- **Recommendation.** The core rule format and its parser budgets
  (SEC-TM-032, SEC-STD-011, SEC-API-066, SEC-IAM-070) ship in R1.1 work;
  the rule editor and smart playlists stay R1.3. discovery.md already reads
  this way; the plan's point-release groups need the same split.

Answered 2026-10-03: recommendation accepted and applied.

### D-86 Loading the web client without the server

- **Question.** Offline loading (CLI-003, CLI-024 to CLI-026, CLI-099) is
  R1.1, so the R1 web client needs the server reachable to load.
- **Recommendation.** Accept for R1; notes to this effect are already in
  discovery.md.

Answered 2026-10-03: recommendation accepted and applied.

### D-87 Speed budgets in R1

- **Question.** DIS-019 (published speed numbers) is R1.1, but DIS-084,
  DIS-100 and CLI-022 in R1 rely on its budgets.
- **Recommendation.** The budget tests stay in the R1 gate; only the
  published numbers wait for R1.1.

Answered 2026-10-03: recommendation accepted and applied.

### D-88 Browsing by mood and label

- **Question.** MUS-060 (genre, mood and label browse) is R1, but the mood
  and label fields come from MUS-019, which is R1.1.
- **Recommendation.** R1 browses by genre; mood and label browsing arrive
  with their fields in R1.1.

Answered 2026-10-03: recommendation accepted and applied.
