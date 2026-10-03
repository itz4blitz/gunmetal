# Supply chain, build and release

Status: proposed security design, 2026-10-02. Written with web search and
page fetches. The session's web-search quota ran out partway through, so
later facts come from direct fetches of primary pages (OWASP, NIST, OpenSSF,
GitHub changelogs and docs, vendor postmortems and advisories). Anything not
confirmed against a source in this pass is marked "(unverified)".

## Summary

Gunmetal will be installed by people who are not security experts, often on
a NAS that is reachable from the internet, and it will hold a household's
viewing and listening history. If our build or release path is compromised,
every one of those servers runs attacker code with access to that data and to
the owner's media. The last eighteen months show that this is now the main
way open-source projects get hurt: worms that spread through npm install
scripts (Shai-Hulud in September and November 2025, Mini Shai-Hulud in May
2026, ChainDrop in August 2026), a compromised crates.io maintainer whose
crate pulled in a typosquatted build-script payload (arrayref, August 2026),
"pwn request" workflows that let a fork steal publishing rights (Nx in August
2025, TanStack in May 2026) and a widely used GitHub Action whose tags were
rewritten to leak secrets (tj-actions, March 2025). This document sets the
rules so that the release path is treated as production from the first
commit.

The key decisions:

1. **Few dependencies, all accounted for.** `gunmetal-core` ships inside
   every client (UniFFI and WASM), so it keeps zero runtime dependencies
   unless an allowlist entry is reviewed. Every Rust dependency is checked by
   `cargo-deny` (advisories, licences, sources, build scripts) and recorded
   in `cargo-vet`. New versions wait seven days before Dependabot proposes
   them. Release builds compile with the network switched off, so a build
   script cannot fetch a payload. JavaScript installs are frozen to the
   lockfile, with install scripts off and a seven-day minimum release age.
2. **CI with least privilege.** No `pull_request_target` with fork code, no
   untrusted text in shell, no caches in release workflows, no long-lived
   publishing tokens, every action and tool pinned. zizmor, CodeQL and
   OpenSSF Scorecard enforce this on every pull request.
3. **Every artifact is traceable and checkable.** Linux binaries are built
   twice on separate runners and must match bit for bit. Each artifact gets
   SLSA Build Level 3 provenance (GitHub artifact attestations from an
   isolated reusable workflow), a Sigstore signature in the public
   transparency log and a CycloneDX 1.7 SBOM. The server binary also carries
   its own dependency list (`cargo-auditable`).
4. **Two separate trust roots.** Provenance proves which workflow built an
   artifact, not that a human meant to release it: ChainDrop and TanStack
   both shipped malware with valid provenance. So the feed that servers
   trust for update notices and security advisories is a TUF repository
   whose release and advisory roles are signed with offline hardware keys.
   Taking over a maintainer's GitHub account is not enough to push anything
   to installed servers.
5. **Hardened containers by default.** The official image is distroless
   (or scratch), has no shell, runs as a non-root user, works with a
   read-only root filesystem and all capabilities dropped. The compose and
   NAS templates we publish ship with those settings, so the secure setup is
   the one people copy.
6. **Advisories that reach admins without a central account.** Plex can
   email owners because it knows who runs what; Gunmetal cannot. Fixes are
   published as GitHub Security Advisories with a CVE, mirrored to OSV (and
   RustSec for any published crate), and pushed into the signed in-product
   feed that the dashboard shows. The update check is off until the admin
   turns it on at first-run setup, and it sends no identifiers.

R1 (music) already contains a server binary, a web client bundled into it,
a container image, release downloads and the advisory feed, so almost every
requirement here is R1. R2 adds native mobile and TV apps (store signing,
Gradle and CocoaPods locking, no over-the-air code) and the bundled FFmpeg.
R3 (M3U and live TV) adds no new code-distribution path. Later covers
automatic installation of updates, the desktop shell and signed plugins.

**Add these to the repository now, before more code lands** (details in
Design guidance, step 1): `deny.toml` and `cargo deny` in the gate; `--locked`
everywhere; an exact `rust-toolchain.toml`; pinned tool versions in CI;
`.github/dependabot.yml` with a cooldown; zizmor, actionlint, CodeQL and
Scorecard workflows; `CODEOWNERS`; a DCO check; `cargo vet init`; REUSE
licensing metadata; rulesets for `main` and release tags, signed commits,
immutable releases, the SHA-pinning Actions policy and secret-scanning push
protection; phishing-resistant MFA on every account that can publish; an
expanded `SECURITY.md`; and reserved package names. All of these are cheap
while the dependency tree is one crate and one dev-dependency, and expensive
to retrofit.

One honest limit: today the project appears to have a single maintainer
(inferred from the repository, unverified). Required
reviews, two-party approval and threshold signing cannot fully work with one
person. The design below works with one maintainer and tightens
automatically when a second one is added; recruiting that second person is
the most valuable single security step left to the owner (see Open
decisions).

## Threats

| ID | Threat | Who (the attacker or failure) | Impact | Likelihood | Mitigated by (requirement IDs) |
|---|---|---|---|---|---|
| T-SUP-01 | A new release of a Rust dependency carries a malicious `build.rs` or proc-macro that runs on maintainer machines and CI (arrayref, internment and append-only-vec, August 2026) | Criminal or state actor holding a stolen crates.io maintainer account | Theft of maintainer and CI credentials; backdoor compiled into a release | High | SEC-SUP-020, 024, 026, 027, 039, 055 |
| T-SUP-02 | A typosquatted or AI-hallucinated package name is added as a dependency (proc-macro1 posing as proc-macro2; "slopsquatting") | Opportunistic attacker registering lookalike names; a coding agent suggesting a package that does not exist | Same as T-SUP-01 | Medium | SEC-SUP-024, 027, 035, 052, 055 |
| T-SUP-03 | An npm worm runs through install scripts or an implicit `node-gyp` build while the web client's dependencies install (Shai-Hulud 2.0 preinstall, June 2026 binding.gyp campaign, ChainDrop) | Worm operators using stolen maintainer tokens | CI and developer secrets stolen; our own packages republished with malware | High | SEC-SUP-016, 033, 034, 036, 039 |
| T-SUP-04 | A dependency with a published vulnerability ships in a release, or stays in one for months | Attacker exploiting a public advisory; failure to notice | Remote code execution or denial of service on servers parsing hostile media | High (over time) | SEC-SUP-021, 022, 023, 028, 044, 048 |
| T-SUP-05 | A fork's pull request runs with write tokens, secrets or OIDC (`pull_request_target`, `workflow_run`), or injects shell through a PR title or branch name (Nx, August 2025) | Anonymous contributor | Repository write access, stolen publishing credentials, poisoned caches | Medium (High if the pattern ever lands) | SEC-SUP-012, 013, 014, 018 |
| T-SUP-06 | A third-party GitHub Action or CI tool is replaced with a malicious version (tj-actions/changed-files, CVE-2025-30066) | Compromised action maintainer | Secrets exposed in logs; tampered builds | Medium | SEC-SUP-010, 011, 012, 016 |
| T-SUP-07 | A cache written by an untrusted workflow is restored by the release workflow (TanStack May 2026, Ultralytics December 2024) | Fork author exploiting cache scope | Malware in a release that carries valid provenance | Medium | SEC-SUP-015, 039, 041 |
| T-SUP-08 | A maintainer account is phished or its session stolen; the attacker pushes a tag and the legitimate pipeline publishes malware with valid provenance (ChainDrop, August 2026) | Phishing crew; infostealer operator | Signed, attested malicious release | Medium | SEC-SUP-001, 003, 017, 049, 053 |
| T-SUP-09 | A long-lived publishing or signing token stored in CI is stolen (Nx npm token; Ultralytics leftover PyPI token) | Anyone who gets code execution in CI | Publishing outside the pipeline, no provenance | Medium | SEC-SUP-006, 016 |
| T-SUP-10 | Release assets are replaced, a tag is moved, or a mirror serves modified files | Attacker with repository or CDN access; network attacker | Users install a tampered binary or image | Low | SEC-SUP-003, 041, 042, 043, 063 |
| T-SUP-11 | The update and advisory feed is rolled back, frozen or mixed so admins never see a security advisory, or are told an old vulnerable version is current | Network attacker; compromised CDN or site account | Servers stay vulnerable while showing "up to date" | Medium | SEC-SUP-049, 050 |
| T-SUP-12 | The update channel delivers a malicious server binary (once automatic install exists) | Attacker holding an online key or the CI pipeline | Every opted-in server compromised at once | Low (Later) | SEC-SUP-049, 064, 065 |
| T-SUP-13 | A floating toolchain or unpinned tool produces binaries nobody can reproduce, hiding tampering | Failure; compromised tool distribution | Tampering undetectable; releases unverifiable | Medium | SEC-SUP-011, 038, 040 |
| T-SUP-14 | A backdoor hides in binary test fixtures or other opaque files that the build reads (xz, CVE-2024-3094) | Long-term insider or patient contributor | Backdoored binary that passes review | Low | SEC-SUP-026, 030, 032 |
| T-SUP-15 | After a parser bug gives code execution, the attacker benefits from root, a writable root filesystem, extra capabilities or `privileged: true` | Attacker who delivered a hostile media file | Host takeover, deletion of media and backups | Medium | SEC-SUP-045, 046, 047 |
| T-SUP-16 | The web client loads a script, font or style from a third-party origin that later turns hostile (polyfill.io, June 2024) | Whoever controls that origin | Account takeover in every browser that opens the server | Low (if we never do it) | SEC-SUP-037 |
| T-SUP-17 | A dependency under an AGPL-incompatible licence, or a non-free FFmpeg build, forces a release to be withdrawn | Failure | Releases pulled; legal exposure for distributors | Medium | SEC-SUP-029, 030, 031, 061 |
| T-SUP-18 | A vulnerability report is lost, disclosed before a fix, gets no CVE, or the fix never reaches admins | Failure; impatient reporter | Exploitation of servers (and, from R2, client apps) that never learn they need to patch | Medium | SEC-SUP-007, 008, 009, 050, 056, 058, 062 |
| T-SUP-19 | A secret is committed to the repository | Failure | Credential abuse | Medium | SEC-SUP-006, 016 |
| T-SUP-20 | Repository protections are switched off "temporarily" and never restored | Failure under time pressure | Every other control silently bypassed | Medium | SEC-SUP-002, 019 |
| T-SUP-21 | A native app update is pushed outside the store (over-the-air JavaScript), or an app signing key is stolen | Attacker with the OTA channel or key | Malicious app on every phone and TV | Low (R2) | SEC-SUP-057, 059 |
| T-SUP-22 | A vulnerable bundled C library ships (SQLite amalgamation, FFmpeg, libmpv) without anyone tracking it | Failure; attacker using a public CVE | Code execution through the C code we chose to bundle | Medium | SEC-SUP-044, 048, 060, 061 |
| T-SUP-23 | A third-party plugin carries malware or is swapped after review | Plugin author or anyone who takes over their account | Server compromise through a feature the admin trusted | Medium (Later) | SEC-SUP-066 |
| T-SUP-24 | A coding agent, steered by text in an issue, dependency README or web page, edits workflows, adds a dependency or uses credentials it can reach | Prompt-injection author | Same as T-SUP-05 or T-SUP-08 | Medium | SEC-SUP-005, 024, 055 |
| T-SUP-25 | A malicious publish under our names goes unnoticed for hours (TanStack had no internal alerting) | Any of the above | Wider spread before revocation | Medium | SEC-SUP-053 |
| T-SUP-26 | An actively exploited vulnerability is not reported on time where the EU Cyber Resilience Act applies | Failure | Regulatory exposure for whoever is the manufacturer | Low to Medium (depends on owner decision) | SEC-SUP-054 |

## Requirements

Releases: R1 Music, its point releases R1.1, R1.2 and R1.3, R2 Video, R3
Live, Later. Standards cite OWASP ASVS
5.0.0 requirement numbers, OWASP Top 10 2025, the OWASP Top 10 CI/CD
Security Risks (CICD-SEC-n), the OpenSSF OSPS Baseline v2026.08.28 (current
on 2026-10-02), NIST SP 800-218 SSDF v1.1 tasks (v1.2 exists only as the
December 2025 initial public draft, SP 800-218r1 ipd; no final found on
CSRC), SLSA v1.2 tracks, OpenSSF Scorecard v5 checks, OWASP MASVS v2 (minor
version unverified), CWE IDs and RFCs.

Many "Verified by" entries name a CI check script. Those scripts are code,
so they live in a Rust `xtask` crate under the same gate as everything else
(test first, 100% coverage, zero surviving mutants); see Design guidance,
step 6.

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-SUP-001 | Every account that can write to the repository or organisation, approve a release, publish to a registry or change the project site or domain (GitHub, crates.io via GitHub, npm, GHCR and any Docker Hub namespace, Cloudflare, the domain registrar, and later Apple and Google developer accounts) must use phishing-resistant MFA (passkey or security key); SMS and TOTP must not be the only factor. | OSPS-AC-01.01; NIST SP 800-63B-4 §2.3.2 and §3.2.5 (AAL3, phishing resistance); CICD-SEC-2; A03:2025; SSDF PO.5.2 | R1 | Organisation "require 2FA" setting checked by the scheduled settings-drift job; quarterly manual review of each account's registered factors, recorded in an access register |
| SEC-SUP-002 | The default branch must be protected by a ruleset that blocks direct pushes, force-pushes and deletion, requires a pull request, and requires the gate, supply-chain, CodeQL and DCO checks to pass. | OSPS-AC-03.01, OSPS-AC-03.02, OSPS-QA-03.01; SLSA v1.2 Source L3; Scorecard Branch-Protection; CICD-SEC-1 | R1 | Scheduled settings-drift job compares the live ruleset with a checked-in expected policy and fails on any difference; Scorecard Branch-Protection |
| SEC-SUP-003 | Release tags (`v*`) must be creatable only by maintainers, must not be movable or deletable, and every release must be published as a GitHub immutable release. | OSPS-BR-02.01, OSPS-BR-03.02; SLSA v1.2 Source L2; CICD-SEC-9 | R1 | Settings-drift job checks the tag ruleset and the immutable-releases setting; the release workflow aborts if immutability is off |
| SEC-SUP-004 | Commits on the default branch must carry a verified signature, and every commit in a pull request must carry a DCO `Signed-off-by` trailer. | OSPS-LE-01.01; SLSA v1.2 Source L2; CONTRIBUTING.md sign-off rule | R1 | Ruleset "require signed commits" checked by the settings-drift job; DCO check in CI with unit tests for trailer parsing (missing, malformed, mismatched author) |
| SEC-SUP-005 | Changes to `.github/`, `scripts/`, `deny.toml`, `supply-chain/`, `rust-toolchain.toml`, lockfiles, container and release files, `SECURITY.md` and `CODEOWNERS` itself must require approval from a code owner; once two maintainers exist, the approver must not be the author. Security-sensitive source paths and code written by coding agents are covered by SEC-STD-035. | SLSA v1.2 Source L4 (two-party review); OSPS-QA-07.01; A03:2025 (separation of duties); CICD-SEC-1 | R1 | CI check that every protected path has an owner in `CODEOWNERS`; settings-drift job checks "require code owner review" |
| SEC-SUP-006 | Secret-scanning push protection must be enabled for the repository, and no credential may be committed. | OSPS-BR-07.01; ASVS 5.0 13.3.1; CWE-798; CICD-SEC-6 | R1 | Settings-drift job; annual manual test that pushing a known test token pattern is blocked |
| SEC-SUP-007 | Private vulnerability reporting must stay enabled, and `SECURITY.md` must state scope, supported versions, response timeframes (acknowledge within 7 days, triage within 14, fix or mitigation targeted within 90), the embargo and credit policy, and how CVEs are requested. | OSPS-VM-01.01, OSPS-VM-02.01, OSPS-VM-03.01; SSDF RV.1.3; ISO/IEC 29147:2018; ISO/IEC 30111:2019 | R1 | CI check that `SECURITY.md` contains each required section; settings-drift job checks private reporting; manual review at each release |
| SEC-SUP-008 | The project site must serve `/.well-known/security.txt` over HTTPS with `Contact`, `Expires` (less than a year ahead), `Policy`, `Canonical` and `Preferred-Languages`. | RFC 9116 §2.5.2, §2.5.3, §2.5.5, §3 | R1 | Site build test validates the file; a scheduled job fetches the live file and fails 30 days before `Expires` |
| SEC-SUP-009 | Every fixed vulnerability in Gunmetal must be published as a GitHub Security Advisory with a CVE requested, affected and fixed versions, CWE and CVSS v4.0 score, and added to the signed advisory feed (SEC-SUP-049) within 24 hours of the fixed release. | OSPS-VM-04.01; SSDF RV.2.2; CVSS v4.0 | R1 | Release workflow refuses to finish a release whose changelog lists a security fix without a linked advisory ID; integration test of feed generation from advisory data |
| SEC-SUP-010 | Every `uses:` reference in workflows must be pinned to a full commit SHA, every container image used in CI or as a base image must be pinned by digest, and the repository's Actions policy must enforce SHA pinning. | CICD-SEC-3, CICD-SEC-8; Scorecard Pinned-Dependencies; CWE-829; A03:2025 | R1 | zizmor `unpinned-uses` and `unpinned-images` as a required check; settings-drift job checks the Actions policy |
| SEC-SUP-011 | Tools installed in CI (cargo-llvm-cov, cargo-mutants, cargo-deny, cargo-vet, cargo-auditable, zizmor, reuse, scanners) must be pinned to exact versions and installed with checksum verification. | CWE-494; SSDF PO.3.2; CICD-SEC-3 | R1 | zizmor `unpinned-tools` and `adhoc-packages` as a required check |
| SEC-SUP-012 | Every workflow must set `permissions: {}` at the top level and grant `contents: read`, write scopes, `id-token: write` or `attestations: write` only on the job that needs them. | OSPS-AC-04.01, OSPS-AC-04.02; CICD-SEC-5; Scorecard Token-Permissions | R1 | zizmor `excessive-permissions`; Scorecard Token-Permissions must score 10 (SEC-SUP-019) |
| SEC-SUP-013 | No workflow may check out or execute pull-request code under `pull_request_target` or `workflow_run`, and workflows triggered by untrusted events must have no secrets, no write token and no OIDC token. | OSPS-BR-01.03; CICD-SEC-4; CWE-829 | R1 | zizmor `dangerous-triggers`; Scorecard Dangerous-Workflow must score 10 |
| SEC-SUP-014 | Untrusted event text (PR titles and bodies, branch names, commit messages, issue text) must never be expanded with `${{ }}` inside `run:` scripts; it must be passed through environment variables and quoted. | OSPS-BR-01.01; CWE-78; CWE-94; CICD-SEC-4 | R1 | zizmor `template-injection`; CodeQL Actions queries as a required check |
| SEC-SUP-015 | Release and publishing workflows must not restore or save GitHub Actions caches and must not consume artifacts produced by workflows triggered from pull requests. | CICD-SEC-4, CICD-SEC-9; CWE-345 | R1 | zizmor `cache-poisoning`; xtask policy test that parses the release workflows and fails on any cache action or cross-workflow artifact download |
| SEC-SUP-016 | Release workflows must authenticate only with short-lived credentials (the job's `GITHUB_TOKEN`, Sigstore keyless OIDC, registry trusted publishing); any long-lived secret must appear in a secrets register with owner, scope and rotation date, and no long-lived registry publishing token may exist. | OSPS-BR-07.02; ASVS 5.0 13.2.1, 13.3.2, 13.3.4; CICD-SEC-6 | R1 | Scheduled job lists repository and environment secret names via the API and compares them with the register; zizmor `use-trusted-publishing`, `secrets-outside-env`, `overprovisioned-secrets` |
| SEC-SUP-017 | Publishing jobs must run in a protected `release` environment that accepts only protected release tags and requires a maintainer's approval; once two maintainers exist, self-approval must be blocked. | CICD-SEC-1; SLSA v1.2 Source L4; SSDF PS.1.1 | R1 | Settings-drift job checks the environment's rules; release checklist rehearsal shows a run from a non-tag ref is refused |
| SEC-SUP-018 | zizmor, actionlint and CodeQL (Rust, JavaScript/TypeScript and GitHub Actions) must run on every pull request and block merging on high-severity findings. | OSPS-VM-06.02; Scorecard SAST; SSDF PW.7.2 | R1 | Required status checks on the default branch |
| SEC-SUP-019 | OpenSSF Scorecard must run weekly and on every push to `main`, and CI must fail if Dangerous-Workflow, Token-Permissions, Pinned-Dependencies, Binary-Artifacts, Security-Policy, Dependency-Update-Tool or License score below 10, or Branch-Protection, Signed-Releases or SBOM fall below their recorded baseline. | OpenSSF Scorecard v5 checks; OSPS-QA-03.01 | R1 | Scorecard action output parsed by an xtask check with unit tests over recorded result files |
| SEC-SUP-020 | `Cargo.lock` must be committed, every cargo command in the gate, CI and release must run with `--locked`, and release compilation must also run `--offline` after a locked fetch. | OSPS-QA-02.01, OSPS-BR-05.01; SSDF PW.4.1; CWE-1357 | R1 | Gate self-test: a CI job edits a manifest without updating the lockfile and asserts the gate fails |
| SEC-SUP-021 | `cargo deny check` (advisories, bans, licenses, sources) must pass on every pull request and daily on `main`, with vulnerabilities denied, `unmaintained = "all"`, `unsound = "all"`, `yanked = "deny"`, wildcards denied, and only the crates.io registry allowed (`unknown-registry` and `unknown-git` denied). | ASVS 5.0 15.2.1, 15.2.4; OSPS-VM-05.02, OSPS-VM-05.03; A03:2025; CWE-1395; CWE-1104 | R1 | Required status check; the daily run opens an issue on failure |
| SEC-SUP-022 | Each ignored advisory must have an entry in `supply-chain/exceptions.toml` giving the advisory ID, why Gunmetal is not affected or how it is mitigated, an owner and a review-by date at most 90 days ahead; CI must fail on an ignore without an entry or past its date. | ASVS 5.0 15.1.1; OSPS-VM-05.01; OSPS-VM-04.02 | R1 | xtask check with unit tests (missing entry, expired date, malformed ID, entry with no matching ignore) |
| SEC-SUP-023 | Vulnerable dependencies must be remediated within documented times: critical or high and reachable from shipped code, a fixed release within 7 days of the advisory; medium within 30 days; low within 90 days; an unmaintained crate replaced, forked or justified within 180 days. | ASVS 5.0 15.1.1, 15.2.1; OSPS-VM-05.01; SSDF RV.2.2 | R1 | The daily cargo-deny issue records its opening date; the release checklist's manual step confirms no issue is past its deadline |
| SEC-SUP-024 | Every crate in the dependency graph must be covered in `cargo-vet` by an audit (`safe-to-deploy` for normal and build dependencies, `safe-to-run` for dev-dependencies) from the project or an imported trusted auditor set, or by a time-limited exemption; the reasoning for each new runtime dependency must be written in the pull request using the dependency checklist. | SSDF PW.4.1, PW.4.4; OSPS-DO-06.01; A03:2025; CWE-1357 | R1 | `cargo vet --locked` as a required status check; code-owner review of `supply-chain/` (SEC-SUP-005) |
| SEC-SUP-025 | `gunmetal-core` must have no normal dependencies except those in a reviewed allowlist (empty today), because it is compiled into every client. | ASVS 5.0 15.1.4; CWE-1357; ADR 1 decision 2 | R1 | xtask check comparing `cargo tree -p gunmetal-core -e normal` with the allowlist, with unit tests |
| SEC-SUP-026 | Only crates on a reviewed allowlist may have build scripts, native executables in build-time crates must be denied (`[bans.build]` with `executables = "deny"` and `include-dependencies = true`), and no workspace build script may read test fixtures or touch the network. | A08:2025; CWE-506; CWE-829; CICD-SEC-3 | R1 | `cargo deny check bans`; xtask test that scans workspace `build.rs` files for fixture paths and network APIs |
| SEC-SUP-027 | A pull request that adds or changes a crate version published less than 7 days earlier must fail unless it carries an override label approved by a code owner (used for urgent security fixes). | A03:2025; CWE-1357; SSDF PW.4.4 | R1 | xtask check that diffs `Cargo.lock` and reads publish times from the crates.io API, unit-tested against recorded API responses |
| SEC-SUP-028 | Dependabot must cover the cargo, github-actions, docker and (once it exists) JavaScript ecosystems with grouped weekly version updates and `cooldown.default-days` of at least 7; security updates are not delayed. | Scorecard Dependency-Update-Tool; OSPS-VM-05.03 | R1 | zizmor `dependabot-cooldown` and `dependabot-execution`; xtask lint of `dependabot.yml` |
| SEC-SUP-029 | Every Rust and JavaScript component in a shipped artifact must use a licence on the project allowlist of AGPL-3.0-or-later-compatible licences; GPL-2.0-only, EPL, CDDL, SSPL, BUSL, the JSON licence, the original OpenSSL licence, BSD-4-Clause, Apache-1.1, MPL-1.1, MPL-2.0 marked "Incompatible With Secondary Licenses", non-commercial or missing licences must fail CI; OFL-1.1 is allowed for font files only. | OSPS-LE-02.01, OSPS-LE-02.02; SSDF PW.4.4; FSF licence list | R1 | `cargo deny check licenses`; a JavaScript licence check against the same allowlist file in CI; code-owner review of every `clarify` entry |
| SEC-SUP-030 | Every file in the repository, including binary test fixtures and artwork, must carry machine-readable copyright and licence information under REUSE 3.3, and each binary media fixture must record its provenance (generator script, or source and licence). | OSPS-LE-03.01; OSPS-QA-05.02; REUSE Specification 3.3 | R1 | `reuse lint` as a required status check |
| SEC-SUP-031 | Every server build must embed the source repository URL and exact commit (from build-time metadata, never wall-clock time), and the web client must show a "Source code" link for that commit to every signed-in user; a modified build must point to its own source. | AGPL-3.0 §13; OSPS-LE-03.02; ASVS 5.0 13.4.6 (version shown only after sign-in) | R1 | Unit test of the build-info module; integration test of the about endpoint (signed in versus signed out); web client end-to-end test |
| SEC-SUP-032 | The repository must not contain executables, archives or generated build outputs; binary media fixtures must be generated by checked-in deterministic scripts where feasible, listed in a SHA-256 manifest, and excluded from any published package. | OSPS-QA-05.01, OSPS-QA-05.02; Scorecard Binary-Artifacts; CWE-506 | R1 | Scorecard Binary-Artifacts; CI regenerates fixtures and compares hashes with the manifest; `cargo package --list` check if a crate is ever published |
| SEC-SUP-033 | JavaScript installs in CI and release must be frozen to the committed lockfile, resolve only from `registry.npmjs.org` (no git, tarball-URL or other exotic sources, direct or transitive), and run with dependency lifecycle scripts and implicit `node-gyp` builds disabled except for a reviewed allowlist. | ASVS 5.0 15.2.4; A03:2025; CWE-506; CWE-829; CICD-SEC-3 | R1 | xtask lint of the package-manager config; CI canary job installs a local test package whose `preinstall` script and `binding.gyp` each try to write a marker file, and asserts neither marker exists |
| SEC-SUP-034 | The JavaScript package manager must refuse versions published less than 7 days earlier, and (with pnpm) must refuse a version whose publishing trust level dropped. | A03:2025; CWE-1357 | R1 | xtask lint of the package-manager config, unit-tested |
| SEC-SUP-035 | Direct JavaScript dependencies must be listed in `supply-chain/js-direct-deps.toml` with a written reason; CI must fail when a manifest adds a direct dependency that is not listed. | A03:2025; CWE-1357; OSPS-DO-06.01 | R1 | xtask check with unit tests |
| SEC-SUP-036 | CI must verify registry signatures (and provenance attestations where present) of every installed npm package. | A08:2025; CWE-345; CWE-494 | R1 | `npm audit signatures` (or the chosen package manager's equivalent) as a CI step |
| SEC-SUP-037 | The web client must not load scripts, styles, fonts or other code from any third-party origin at runtime: everything is bundled into the server binary and served from the same origin under a CSP with no external script source. | ASVS 5.0 3.6.1, 3.4.3; A08:2025; CWE-830 | R1 | Build test scans the bundle for absolute external URLs in script, style and font references; integration test asserts the CSP header |
| SEC-SUP-038 | The Rust toolchain must be pinned to an exact version in `rust-toolchain.toml`, and release builds must run in a builder image pinned by digest. | SSDF PO.3.2, PW.6.1; CWE-494 | R1 | xtask check of the toolchain file; zizmor `unpinned-images` |
| SEC-SUP-039 | Release compilation (Rust and the web bundle) must run with no network access after the locked fetch step. | CWE-494; CWE-506; CICD-SEC-4; SLSA v1.2 Build L3 | R1 | CI self-test builds a canary crate whose `build.rs` attempts a network connection inside the release build container and asserts the build fails |
| SEC-SUP-040 | Linux release binaries (x86-64, AArch64, ARMv7) must be reproducible: two builds of the same tag on separate runners must produce identical SHA-256 digests, or the release fails; the steps to reproduce must be documented. | SSDF PS.3.1, PW.6.1; OSPS-DO-07.01; SLSA v1.2 Build track (verification) | R1 | Release workflow compares digests from the two build jobs; a weekly job rebuilds the latest release and compares |
| SEC-SUP-041 | Every release artifact (archives, binaries, container images, SBOMs) must have SLSA Build Level 3 provenance from an isolated reusable build workflow, recorded in the Sigstore public transparency log. | SLSA v1.2 Build L3; OSPS-BR-06.01; SSDF PS.2.1, PS.3.2; CICD-SEC-9 | R1 | Release workflow runs `gh attestation verify` with the expected signer workflow on every artifact before publishing; a post-release job repeats it on the public downloads |
| SEC-SUP-042 | Every release must publish a `SHA256SUMS` manifest with a Sigstore bundle, and the docs must give copy-paste verification commands naming the expected signer identity (repository, workflow path, tag ref) and OIDC issuer. | OSPS-BR-06.01, OSPS-DO-03.01, OSPS-DO-03.02; SSDF PS.2.1 | R1 | Post-release job extracts the commands from the docs and runs them unchanged against the published release |
| SEC-SUP-043 | Container images must be signed by digest (Sigstore keyless) with provenance and SBOM attestations for the multi-arch index and each platform image, and an exact-version tag must never be pushed twice. | A08:2025; CWE-494; OSPS-BR-03.02 | R1 | Post-release verification job; the publish step checks the registry and refuses to overwrite an existing exact-version tag |
| SEC-SUP-044 | Each release must include CycloneDX 1.7 JSON SBOMs covering the Rust crates (with licences and hashes), the JavaScript packages bundled into the web client, bundled C code (SQLite and, from R2, FFmpeg) and the container image contents, and the server binary must embed its dependency list with `cargo-auditable`. | ASVS 5.0 15.1.2; OSPS-QA-02.02; SSDF PS.3.2; Scorecard SBOM; A03:2025 | R1 | CI validates each SBOM against the CycloneDX 1.7 schema and an xtask comparator checks it lists exactly the locked packages; `cargo audit bin` reads the built binary |
| SEC-SUP-045 | The official container image must be built from distroless static or cc (Debian 13) or scratch, pinned by digest, contain no shell or package manager, run as a fixed non-root UID and GID by default, and work with a read-only root filesystem with writable mounts only for config, cache and a tmpfs `/tmp`. | NIST SP 800-190 (2017; no newer revision found, unverified); CWE-250; ASVS 5.0 15.2.3 | R1 | Container test runs the image with `--read-only --cap-drop=ALL --security-opt=no-new-privileges` and a smoke test; image inspection test asserts a non-zero user and no `/bin/sh` |
| SEC-SUP-046 | The image and every compose, Unraid and TrueNAS template we publish must work with all capabilities dropped, `no-new-privileges` and the default seccomp profile, and must never require `privileged: true` or host networking; GPU access (R2) must use device mappings only. | CWE-250; CWE-269; NIST SP 800-190 | R1 | xtask lint of shipped templates with unit tests; container smoke test using the templates' settings |
| SEC-SUP-047 | **Withdrawn 2026-10-02: merged into SEC-OPS-053.** No `--allow-root` override; official templates set a non-root user. | CWE-250; CWE-269 | Withdrawn | Proved by the tests of SEC-OPS-053 |
| SEC-SUP-048 | Container images and their embedded dependency data must be scanned before publishing and daily afterwards; a fixable critical or high finding must block the release. | A03:2025; CWE-1395; OSPS-VM-05.02 | R1 | Pinned scanner step in the release workflow; scheduled scan of published images opens an issue on findings |
| SEC-SUP-049 | Update notices and advisories must be published as TUF 1.0 metadata on the project site, with root, release-targets and advisory roles signed by offline hardware keys (threshold 2 once two keyholders exist) and only snapshot and timestamp signed online. The threshold keys must be held by distinct people (SEC-STD-036). | TUF specification 1.0.36; SSDF PS.2.1; A08:2025; CWE-347 | R1 | Integration tests against a local TUF fixture repository: rollback, freeze, mix-and-match, oversized metadata, wrong-key and root-rotation cases; key ceremony reviewed manually and recorded |
| SEC-SUP-050 | The server's update check (its default is the first-run question of SEC-OPS-047) must verify the feed from a root embedded at build time, reject rollback, expired, inconsistent or oversized metadata, never act on unverified data, and show a plain-language warning when it cannot get a fresh, valid feed for 7 days, except in offline mode (SEC-PRV-012), where a quiet status line ("Security updates: offline mode, last checked <date or never>") replaces the warning. | TUF 1.0.36; CWE-345; CWE-347; CWE-494 | R1 | Integration tests with tampered fixtures; property tests and a fuzz target for the metadata parser; web client test of the stale-feed warning; banner unit tests over the offline and online modes |
| SEC-SUP-051 | The update check must be a plain GET of static metadata files with no query string, cookie, instance identifier or installed-version information; matching versions and advisories happens on the server. | ASVS 5.0 13.1.1, 13.2.4; OPS-11 (setup research) | R1 | Integration test records outgoing requests against a local test server and asserts their exact form |
| SEC-SUP-052 | The project's names must be reserved before announcement on each registry users might search (crates.io names for the server and core, the npm scope, the container registry namespaces). | A03:2025; CWE-1357 | R1 | Manual checklist in the release runbook, re-checked before each release |
| SEC-SUP-053 | A scheduled job must compare published tags, releases, container tags and any registry versions with the release workflow's own log, and alert maintainers within one hour of anything it did not produce. | CICD-SEC-10; A03:2025 | R1 | xtask check unit-tested against recorded API responses; manual rehearsal once before R1 |
| SEC-SUP-054 | A supply-chain incident runbook must cover a malicious dependency, a compromised maintainer account, a leaked or lost signing key (including TUF key rotation and revoking images) and, if the owner decides the EU Cyber Resilience Act applies, its reporting deadlines; it must be rehearsed before R1. | SSDF RV.2.2, RV.3; Regulation (EU) 2024/2847 Art. 14, Art. 24; OSPS-VM-04.01 | R1 | Manual tabletop exercise recorded in the runbook; the TUF root-rotation integration test from SEC-SUP-049 |
| SEC-SUP-055 | Coding agents and other automation working on the repository must run without access to release credentials, signing keys or maintainers' authenticated browser and CLI sessions, and their changes go through the same reviews and checks as any contributor's. | CICD-SEC-2, CICD-SEC-6; SSDF PO.5.2; A03:2025 | R1 | Manual review of the access register; CODEOWNERS and required checks (SEC-SUP-002, 005) |
| SEC-SUP-056 | Every release must have a unique version, a changelog with a Security section listing fixed advisories, and `SECURITY.md` must state which versions receive security fixes and until when. | OSPS-BR-02.01, OSPS-BR-02.02, OSPS-BR-04.01, OSPS-DO-04.01, OSPS-DO-05.01 | R1 | Release workflow lint of the changelog entry, unit-tested; manual review of the supported-versions table |
| SEC-SUP-057 | Native mobile and TV apps must receive code only through signed store or package channels (App Store, Google Play, signed APKs on GitHub releases or F-Droid) and must not download or run JavaScript bundles or native code at runtime. | MASVS-CODE-2, MASVS-RESILIENCE-2; A08:2025; CWE-494 | R2 | Dependency allowlist check fails if an over-the-air update module is linked; manual review per release |
| SEC-SUP-058 | The server must publish a minimum native-client version and refuse sessions from client versions named as vulnerable in the signed advisory feed, with an "update this app" screen that works with a remote and links to the right store. | MASVS-CODE-2 | R2 | Integration test with a vulnerable client version; UI test of the screen on TV focus navigation |
| SEC-SUP-059 | App signing keys must live only in hardware tokens or the store's key service (Play App Signing with a separate upload key), and the release workflow must check that each built app's signing certificate matches the published fingerprint. | MASVS-RESILIENCE-2; SSDF PS.1.1; CWE-522 | R2 | Release step runs the platform's signature check (for example `apksigner verify --print-certs`) and compares with the documented fingerprint; manual key-custody review per release |
| SEC-SUP-060 | Native build dependencies must be locked and checksum-verified: Gradle dependency verification metadata and wrapper checksum validation, committed `Podfile.lock` and `Package.resolved`, and libmpv and its libraries built from source archives pinned by SHA-256. | A03:2025; CWE-494; SSDF PW.4.4 | R2 | CI builds with strict dependency verification; wrapper validation step; xtask hash check of vendored source archives |
| SEC-SUP-061 | Official packages must ship an FFmpeg built from a SHA-256-pinned source archive with a recorded configure line that never enables non-free components, listed in the SBOM and tracked for advisories; whichever FFmpeg runs, the server must report its version and build in `gunmetal doctor`. | A03:2025; CWE-1395; CWE-494; ASVS 5.0 15.2.5 | R2 | xtask hash and configure-line check; SBOM comparator; doctor integration test |
| SEC-SUP-062 | Each release must publish a VEX document for advisories that affect a dependency but not Gunmetal. | OSPS-VM-04.02; CycloneDX 1.7 | R2 | CI validates the VEX document and checks every entry in `supply-chain/exceptions.toml` appears in it |
| SEC-SUP-063 | Windows and macOS binaries must be code-signed (and notarised on macOS) in the release environment, carry the same provenance and checksums as Linux builds, and be checked for a valid signature before publishing. | SSDF PS.2.1; OSPS-BR-06.01 | Later | Release step verifies the platform signatures; post-release verification job |
| SEC-SUP-064 | Automatic installation of server updates must be opt-in, verify the target through TUF and its provenance, never install a lower version automatically (a manual downgrade stays an explicit admin action), keep the previous binary, snapshot the database before migrating, roll back automatically when the health check fails, and be unavailable inside containers. | TUF 1.0.36; CWE-494; CWE-1329; MASVS-CODE-2 (by analogy) | Later | Integration tests with fake releases (downgrade refused, bad signature refused, failed start rolled back); end-to-end upgrade and rollback test |
| SEC-SUP-065 | The desktop shell's updater must verify signatures before installing, refuse downgrades and use the same TUF repository. | TUF 1.0.36; CWE-347; CWE-494 | Later | Integration tests with tampered and older updates |
| SEC-SUP-066 | Plugins must be distributed as delegated TUF targets signed by the plugin author's key and verified before the server loads them; loading an unsigned plugin must need an explicit per-plugin admin override. | A08:2025; CWE-494; CWE-829 | Later | Integration tests with signed, unsigned, tampered and revoked plugins |

**Bound by requirements in [standards-coverage.md](standards-coverage.md).** SEC-STD-005 (security evidence bundle per release), SEC-STD-033 (binary hardening), SEC-STD-035 (CODEOWNERS for security-sensitive source and agent-written code) and SEC-STD-036 (roles, distinct TUF keyholders, succession).
The 2026-10-02 challenge review merged duplicated controls into one owner each; withdrawn rows above say where their content went, and [threat-model.md](threat-model.md#control-ownership) lists every owner.

Count: 65 live requirements: 55 R1, 6 R2, 0 R3 and 4 Later, plus 1 withdrawn rows kept so their IDs stay stable.

## Design guidance

### 1. What to add now, in order

Each item is small today and grows painful later. Write the checks test
first like any other code (step 6).

1. **Accounts.** Turn on "require two-factor authentication" for the
   PremierStudio organisation, register passkeys or security keys on every
   account listed in SEC-SUP-001, and remove TOTP where the service allows
   it. Start an access register (who holds what, which factor, when last
   reviewed); OSPS-GV-01.01 asks for this list anyway.
2. **Rulesets and settings** (SEC-SUP-002 to 006, 010, 017): a `main`
   ruleset, a `v*` tag ruleset, signed commits on `main`, code-owner review,
   immutable releases, the Actions SHA-pinning policy, secret-scanning push
   protection, private vulnerability reporting (already on, according to
   `SECURITY.md`), and an empty `release` environment with required
   reviewers. Check the current settings before changing them; this
   document did not inspect them.
3. **Gate changes** in `scripts/gate.sh`: add `--locked` to every cargo
   command and add `cargo deny check` and `cargo vet --locked`. Keep the
   gate as the single definition of done; give it an offline switch for
   machines without network (cargo-deny can skip fetching and warns when its
   database is older than its staleness limit, 90 days by default).
4. **New files:** `rust-toolchain.toml` with an exact version (the
   `dtolnay/rust-toolchain` step currently installs whatever "stable" is
   that day); `deny.toml`; `supply-chain/` from `cargo vet init`;
   `supply-chain/exceptions.toml`; `.github/dependabot.yml`;
   `.github/CODEOWNERS`; `REUSE.toml` plus `LICENSES/`; workflows for
   supply-chain checks, CodeQL and Scorecard.
5. **CI fixes** in `.github/workflows/ci.yml`: the file already pins actions
   by SHA, sets `contents: read` and disables credential persistence, which
   is a good start. Change the top level to `permissions: {}` with
   `contents: read` on the job, pin the tools passed to
   `taiki-e/install-action` to exact versions (`cargo-llvm-cov@x.y.z`), and
   keep `Swatinem/rust-cache` for pull-request CI only, never in release.
6. **`SECURITY.md`**: add the content in step 11.
7. **Names**: reserve the crates.io, npm and container namespaces
   (SEC-SUP-052).

### 2. Repository protections that survive one tired evening

- Express every setting as a checked-in expected policy
  (`.github/settings-policy.toml`, read by the settings-drift job). The job
  needs a read-only token with repository administration read access; it is
  the one long-lived secret we accept, and it goes in the secrets register.
  The OpenSSF Allstar app is an alternative if the owner prefers an app
  over a token.
- Signed commits on `main` cost contributors nothing: GitHub signs the
  squash merges it creates. Contributors' own branches need not be signed.
- Release tags are signed by the maintainer (SSH signing is the least
  friction). The release workflow verifies the tag signature against
  `.github/allowed_signers`, which is a code-owned file.
- With one maintainer, required reviews cannot stop that maintainer's own
  account from merging. That is why the trust that installed servers rely
  on is anchored in offline keys (step 10), not in GitHub.

### 3. Workflow layout

| Workflow | Trigger | Permissions | Job |
|---|---|---|---|
| `ci.yml` | `pull_request`, push to `main` | `contents: read` | The gate |
| `supply-chain.yml` | `pull_request`, push to `main`, daily | `contents: read`; `issues: write` only on the scheduled job | cargo-deny, cargo-vet, exceptions check, lockfile age check, zizmor, actionlint, `reuse lint`, DCO, licence checks |
| `codeql.yml` | `pull_request`, push to `main`, weekly | `security-events: write` on the analysis job | Rust, JS/TS, Actions |
| `scorecard.yml` | push to `main`, weekly | `security-events: write`, `id-token: write` on that job | Scorecard with published results |
| `settings-drift.yml` | daily | `contents: read`, `issues: write` | Compares live settings with the policy file |
| `release.yml` | push of a `v*` tag | `{}` at top; per job below | Orchestrates the release |
| `build.yml` | `workflow_call` only | `contents: read`, `id-token: write`, `attestations: write` | The isolated reusable build, so provenance reaches Build L3 |

Rules that apply to all of them:

- Never `pull_request_target` or `workflow_run` for anything that touches
  PR code. GitHub's December 2025 change (workflows for that trigger always
  come from the default branch) narrows the problem but does not make
  running fork code safe.
- Pass event text through `env:` and quote it in the shell
  (`TITLE: ${{ github.event.pull_request.title }}` in `env`, then `"$TITLE"`).
- `persist-credentials: false` on every checkout.
- `timeout-minutes` on every job; `concurrency` on CI.
- First-time contributors' workflow runs need approval (repository setting).

### 4. Release pipeline

1. **`verify-tag` job** (`contents: read`): the tag points to a commit on
   `main`, its CI run passed, and the tag signature verifies against
   `.github/allowed_signers`.
2. **`build` jobs** call `build.yml` for each Linux target: check out at the
   tag, `cargo fetch --locked`, fetch JavaScript packages with the frozen
   lockfile, then compile inside the builder image pinned by digest with
   `docker run --network=none`. The web bundle is built first and embedded
   into the server binary, so one artifact carries both.
3. **`rebuild` jobs** run the same build on fresh runners. A comparison job
   fails the release unless every SHA-256 matches (SEC-SUP-040).
4. **Attest** each artifact with `actions/attest-build-provenance` inside
   `build.yml`. GitHub documents this as SLSA Build L2 by default and L3
   when the build runs in a reusable workflow isolated from the caller.
5. **`sbom` job**: CycloneDX 1.7 for Rust (from `Cargo.lock`), the
   JavaScript bundle (from the lockfile) and C sources; attach as
   attestations too.
6. **`container` job**: build the image from the verified binaries (never
   recompile), push by digest, sign with cosign (version 3 uses the Sigstore
   bundle format by default), attach SBOM and provenance, then add tags.
7. **`publish` job** in the `release` environment (manual approval): create
   a draft GitHub release, upload archives, `SHA256SUMS` and its Sigstore
   bundle, the SBOMs and the changelog section, then publish it as
   immutable.
8. **`verify-published` job**: download everything from the public URLs and
   run the documented verification commands (SEC-SUP-042).
9. **TUF step**: open a pull request to the TUF repository adding the
   release manifest; a keyholder signs it with a hardware key (step 10).
   Until that signature lands, servers do not see the release in their
   update notices, but people can already download and verify it.

None of these jobs restores a cache. Registry pushes use `GITHUB_TOKEN`
with `packages: write`, signing uses OIDC, and publishing to crates.io or
npm (if ever needed) uses trusted publishing, which both registries support
from GitHub Actions.

### 5. Dependency policy

**Rust.** Before adding a crate, the pull request answers this checklist:
what test needs it; why a few dozen lines of our own code would not do;
who maintains it and how many people can publish it; whether it has a
build script, proc-macros or `unsafe`; its licence; how many transitive
crates it adds; whether an imported audit exists. `gunmetal-core` stays
dependency-free unless an allowlist entry is approved. A `deny.toml`
outline:

```toml
[advisories]
unmaintained = "all"
unsound = "all"
yanked = "deny"
ignore = []            # each entry must also exist in supply-chain/exceptions.toml

[licenses]
allow = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib",
         "BSL-1.0", "0BSD", "Unicode-3.0", "CC0-1.0", "Unlicense", "MPL-2.0",
         "LGPL-2.1-or-later", "LGPL-3.0-or-later", "GPL-3.0-or-later",
         "AGPL-3.0-or-later"]

[bans]
wildcards = "deny"
multiple-versions = "warn"
allow-build-scripts = []   # generate from today's graph, then review each

[bans.build]
executables = "deny"
include-dependencies = true

[sources]
unknown-registry = "deny"
unknown-git = "deny"
```

Today's graph is `gunmetal-core` plus proptest and its dev-only tree, which
already includes crates with build scripts and a proc-macro; seed the
allowlist and the cargo-vet exemptions from the real graph, not from this
document. For cargo-vet, import audits from organisations that publish them
(Mozilla, Google and the Bytecode Alliance are the usual set; confirm each
import URL when setting it up) and require `safe-to-deploy` for anything
that ships.

Cargo has no stable release-age gate yet: RFC 3923 (February 2026) proposes
`min-publish-age`, and only an unstable `--publish-time` option exists for
lockfile generation. Until it stabilises, the cooldown comes from Dependabot
(three days by default since July 2026; we set seven) plus the lockfile age
check in SEC-SUP-027. The malicious versions in the 2026 incidents cited
here were spotted within minutes to hours (TanStack's within about 20
minutes; the arrayref versions were removed after 86 to 107 minutes,
according to The Hacker News), so seven days is a wide margin.

**JavaScript.** Choose the package manager before the web client's first
test (Open decisions). Both serious choices now block install scripts by
default: pnpm since version 10, npm since version 12 (released July 2026),
which also blocks implicit `node-gyp` builds, git dependencies and remote
tarballs unless allowed. Settings for pnpm, as an outline:

```yaml
# pnpm-workspace.yaml
minimumReleaseAge: 10080   # minutes, 7 days
trustPolicy: no-downgrade
blockExoticSubdeps: true
allowBuilds: {}            # add reviewed entries only
```

For npm 12: `min-release-age=7` in `.npmrc`, an `allowScripts` allowlist
committed through `npm approve-scripts`, and `npm ci` in CI. Either way,
the CI canary in SEC-SUP-033 proves scripts do not run, because reports
disagreed about whether older `--ignore-scripts` behaviour stopped the June
2026 `binding.gyp` technique (Snyk says the flag blocks the implicit
rebuild; StepSecurity reported a bypass). Do not trust a flag you have not
tested.

React Native adds many transitive packages. Keep direct dependencies few
and listed (SEC-SUP-035), prefer packages published with trusted publishing
and provenance, and remember that a provenance check only proves which
pipeline published a package, not that its maintainer meant to.

### 6. Check tools are code

The checks this document asks for (exceptions file, lockfile age, settings
drift, SBOM comparator, template lint, changelog lint, publish monitor) are
small programs. Put them in a Rust `xtask` crate when the first one is
needed, so they get the same gate: test first, 100% coverage, zero
surviving mutants. Network-facing checks take recorded API responses as
test fixtures. Shell one-liners in YAML escape both review and testing; keep
them to calls into `xtask`.

### 7. Reproducible builds

- Same toolchain (SEC-SUP-038), same builder image, `--locked --offline`,
  same target triple, and the same absolute paths, or remove them: pass
  `--remap-path-prefix` for the workspace and `CARGO_HOME` through
  `RUSTFLAGS` (Cargo's `trim-paths` is still unstable).
- No build timestamps. Build metadata (SEC-SUP-031) uses the commit hash and
  commit time. Archives are created with sorted entries, fixed owner and
  `SOURCE_DATE_EPOCH` set to the commit time; BuildKit honours the same
  variable for image timestamps (exact BuildKit options not verified).
- Bundled C code (the SQLite amalgamation through `rusqlite`'s bundled
  feature, if chosen) is compiled by the pinned image's C compiler, which is
  why the image is pinned by digest.
- Start with static Linux targets (musl). If a target cannot be made
  reproducible in time for R1, the release still ships it, but the
  comparison job lists it as an exception approved by the owner.

### 8. Provenance, signatures and SBOMs for users

Most users will never run a verification command, so the defaults have to
protect them: images are pulled over TLS from GHCR by tag, release pages
are immutable, and the in-product notice (step 10) shows the exact image
digest for each release. For people who do verify, the docs give commands
in this shape (identity values are examples to adapt):

```bash
gh attestation verify gunmetal-x86_64-linux.tar.gz \
  --repo PremierStudio/gunmetal \
  --signer-workflow PremierStudio/gunmetal/.github/workflows/build.yml

cosign verify ghcr.io/premierstudio/gunmetal@sha256:<digest> \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --certificate-identity-regexp '^https://github.com/PremierStudio/gunmetal/\.github/workflows/build\.yml@refs/tags/v'
```

SBOMs are CycloneDX 1.7 JSON (released October 2025; SPDX 3.1 was still a
release candidate in January 2026, so CycloneDX is the safer pick). The
server binary also embeds its dependency list through `cargo-auditable`,
which Trivy, Grype, OSV-Scanner and `cargo audit bin` read, so admins'
existing scanners can see what is inside a running server.

### 9. Containers

```dockerfile
FROM gcr.io/distroless/static-debian13:nonroot@sha256:<digest>
COPY gunmetal /usr/local/bin/gunmetal
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/gunmetal", "serve"]
```

The compose template we publish:

```yaml
services:
  gunmetal:
    image: ghcr.io/premierstudio/gunmetal:1
    user: "1000:1000"            # match the owner of your media files
    read_only: true
    cap_drop: [ALL]
    security_opt: ["no-new-privileges:true"]
    tmpfs: ["/tmp"]
    volumes:
      - ./config:/config
      - ./cache:/cache
      - /srv/music:/media/music:ro
    ports: ["<port>:<port>"]
```

- Media is mounted read-only by default; R1 never writes to media files.
- Tags: exact versions (`1.2.3`, never re-pushed) and major lines (`1`).
  No tag that downloads the server at container start (the research found
  Plex's `public` tag does this, so the image does not fix the version).
- NAS users fight with file ownership, which is why many images start as
  root and drop privileges. We do not: the templates expose the user and
  group, the docs show the Unraid (`99:100`) and Synology values, and
  `gunmetal doctor` names the exact unreadable path and the user it runs as.
- The R2 transcode sandbox must work inside this container with no extra
  privileges. Docker's default seccomp profile blocks creating user
  namespaces without extra capabilities (unverified for every runtime), so
  the sandbox design should rely on mechanisms an unprivileged process can
  apply to itself (seccomp filters, Landlock) rather than on namespaces
  alone. That design belongs to the transcoding security document.
- Hardware transcoding (R2) maps `/dev/dri` and nothing else. Guides that
  say "just use `privileged: true`" are exactly what our docs must replace.

### 10. Updates and advisories

The feed is a TUF repository served as static files from the project site
(ADR 1, decision 9), for example under `https://gunmetal.tv/tuf/`.

| Role | Signs | Key | Expiry |
|---|---|---|---|
| root | All other roles' keys | Offline hardware keys; 2 of 3 once there are two keyholders, kept in separate places | 1 year |
| targets | Delegations only | Offline hardware keys | 1 year |
| `releases` (delegated) | One manifest per release: version, artifact names, SHA-256, sizes, image digests, provenance bundle hashes, "safe to roll back" flag (OPS-3), oldest version it upgrades from | Offline hardware key | 1 year |
| `advisories` (delegated) | One record per advisory: ID, CVE, severity, affected and fixed ranges, plain-language summary, action | Offline hardware key | 1 year |
| snapshot, timestamp | Freshness | Online, signed by CI | 7 days for snapshot; 1 day for timestamp, re-signed daily |

- tuf-on-ci (from the TUF project; Sigstore and GitHub use it in
  production) supports hardware keys for offline roles and CI signing for
  online ones. On the server, the `tough` crate (AWS Labs, a TUF client
  library in Rust, under active development) is the first candidate; it
  goes through cargo-vet like any other dependency. `rust-tuf` describes
  itself as not production-ready.
- The first root is compiled into the server (SEC-SUP-050); later roots
  arrive through TUF's root-rotation chain, which the integration tests
  exercise.
- An online key can only freshen what offline keys already signed. A stolen
  GitHub account or CI secret cannot add a release or an advisory.
- The check runs at most daily with random jitter, as a GET of static files
  (SEC-SUP-051). Cloudflare still sees the server's IP address; the
  first-run choice says so in one sentence.
- Clock trouble is common on NAS boxes. When metadata looks expired because
  the local clock is wrong, the dashboard says "your server's clock appears
  to be wrong" rather than "update feed invalid", and `gunmetal doctor`
  already checks the clock (OPS-10).
- What admins see: a small badge for a new version; a banner for an
  advisory that matches the installed version, saying how bad it is and
  exactly what to do (with the image digest or download link). Banners for
  low-severity advisories can be dismissed per version; critical ones stay
  until fixed. Household members and TV users see nothing.
- Advisory text is shown as text. Links render only to the project site and
  the GitHub repository, so even a valid but mistaken advisory cannot send
  admins elsewhere.
- Automatic installation (Later, SEC-SUP-064) reuses the same verification,
  adds Sigstore provenance checking, and follows OPS-3: snapshot, migrate
  check, start, health check, automatic revert. It is never offered inside a
  container, where the image tag is the update mechanism.

### 11. Disclosure, CVEs and advisories

`SECURITY.md` should say, in this order: how to report (GitHub's private
reporting, plus a `security@` address for people without a GitHub account,
which the owner must create); what is in scope (the server, clients, the
adapters, the official images and templates, the project site); timeframes
(SEC-SUP-007); an embargo of up to 90 days by default, shorter if the issue
is being exploited, longer only by agreement; credit for reporters; a
safe-harbour statement for good-faith research; and which versions get
fixes (pre-1.0: the latest release only).

The flow for a report:

1. Acknowledge within 7 days; open a draft repository security advisory.
2. Reproduce and fix in the advisory's temporary private fork, with a
   regression test that fails before the fix.
3. Request a CVE from the advisory (GitHub acts as the CNA for these;
   unverified in this pass). Score with CVSS v4.0 and assign a CWE.
4. Release the fix, publish the advisory, add the TUF advisory record, and
   link it all from the changelog's Security section.
5. If a published crate is affected, also file with RustSec; GitHub's
   database flows into OSV automatically.
6. Vulnerabilities in our dependencies go to their maintainers privately; if
   Gunmetal users are affected, we publish our own advisory pointing to the
   upstream one.

Do not depend on any single identifier system. The CVE programme nearly
lost its funding in April 2025, when MITRE's contract was about to expire;
CISA exercised an option that ran to 16 March 2026, and what happened after
that was not established in this pass (unverified). Publishing to GHSA, OSV and our own signed feed
means a lapse in CVE does not stop admins from learning about fixes.

The EU Cyber Resilience Act (Regulation (EU) 2024/2847) requires
manufacturers to report actively exploited vulnerabilities (an early warning
within 24 hours, a notification within 72 hours) from 11 September 2026.
Volunteer projects without commercial activity are outside it; open-source
stewards (legal persons that support free software intended for commercial
use) have lighter duties under Article 24, and the date those apply was
reported inconsistently (one source says 11 December 2027; unverified).
Whether Premier Studio is a manufacturer, a steward or neither is a legal
question for the owner (Open decisions). The runbook (SEC-SUP-054) holds
the steps either way.

### 12. Native clients (R2) and later

- Distribution: App Store and TestFlight, Google Play, signed APKs on
  GitHub releases, F-Droid if the owner chooses it (F-Droid can check
  reproducible builds). Google's developer verification for apps installed
  outside Play started on 30 September 2026 in four countries and expands
  globally in 2027; it has a dedicated path for open-source apps, so the
  sideloaded APK needs a registered developer identity before then.
- No over-the-air JavaScript. Expo's EAS Update supports code signing, but
  it is optional and tied to paid plans; store releases are slower but keep
  one trust path.
- Gradle: commit `gradle/verification-metadata.xml` and validate the wrapper
  checksum. iOS: commit `Podfile.lock` and `Package.resolved`. libmpv and its
  libraries: build from pinned source archives in CI, listed in the SBOM.
- Minimum client version (SEC-SUP-058): the server reads it from the
  signed advisory feed, so a client with a known hole is told to update.
  The TV screen must be usable with a remote and must not block playback of
  content already downloaded for offline use.

### 13. Developer machines and coding agents

The arrayref payload ran at compile time on whoever built the project, and
the Nx malware turned AI command-line tools into search engines for
credentials. The maintainer's workstation is therefore part of the supply
chain.

- Build and run coding agents in a container or VM that holds no
  password-manager session, no SSH or tag-signing key, no registry token and
  no cloud credentials. Agents open pull requests; humans merge and tag.
- Treat any new dependency an agent proposes as unverified until a human has
  checked it exists, is the intended project and passes the checklist. A
  USENIX Security 2025 study found code-generating models suggest packages
  that do not exist (about 5% for commercial models, about 22% for open
  ones).
- Release signing keys (tag signing, TUF) live on hardware tokens, never
  in files on a laptop.

### 14. Keeping it usable

| Control | Who could be annoyed | How it stays usable |
|---|---|---|
| Seven-day cooldowns | Maintainers wanting a new feature | Security updates skip the cooldown; a code-owner label overrides it for urgent cases |
| cargo-vet | Contributors adding crates | Imported audits cover common crates; exemptions are allowed with a date; few crates by design |
| Script blocking in npm/pnpm | Web developers | A reviewed allowlist for the few packages that need a build |
| Signed commits on `main` | Contributors | GitHub signs squash merges; branches need nothing |
| Release approval and hardware keys | The maintainer | One approval click and one key touch per release |
| Non-root, read-only container | NAS users | Templates set it all; `doctor` explains permission errors in plain words |
| Update check off by default | Admins who want notices | Offered at first-run setup with one sentence about what it sends (nothing but a fetch) |
| Advisory banners | Admins | Dismissable unless critical; never shown to household members |
| Minimum client version | TV and phone users | Only for versions with a known security flaw; offline downloads still play |

## Anti-patterns

- **Running fork code under `pull_request_target`.** Nx (August 2025) used
  that trigger with a shell-injectable PR title and lost its npm token
  (GHSA-cxm3-wv7p-598c). TanStack (May 2026) ran fork code under the same
  trigger, which poisoned the pnpm store in a cache the release workflow
  restored, and an OIDC token read from runner memory published 84
  malicious versions. Ultralytics (December 2024) was hit through
  Actions cache poisoning too.
- **Expanding `${{ github.event... }}` inside `run:`.** The Nx injection
  path. CWE-78.
- **Pinning actions by tag.** tj-actions/changed-files had its tags
  rewritten to a commit that dumped runner secrets into public logs
  (CVE-2025-30066, March 2025; the action was used in over 23,000
  repositories).
- **Caches shared between untrusted and release workflows.** TanStack and
  Ultralytics, above.
- **Long-lived publishing tokens in CI.** Ultralytics' attackers used a
  PyPI token left over from before trusted publishing. npm is removing
  tokens that bypass 2FA from publishing (around January 2027) for the same
  reason.
- **Treating provenance as authorisation.** ChainDrop (August 2026) took
  over a maintainer's GitHub account, pushed tags and let legitimate
  pipelines publish malware with valid SLSA attestations through trusted
  publishing; the worm then reached 444 packages. TanStack's malicious
  versions were also attested. Provenance says where something was built, not whether
  anyone meant to release it.
- **Letting dependencies execute code at install or build time with network
  and secrets available.** Shai-Hulud 2.0 (November 2025) moved to
  `preinstall` and reached hundreds of packages and tens of thousands of
  repositories; the June 2026 campaign hid in `binding.gyp`; arrayref's
  injected dependency ran its payload from `build.rs`.
- **Trusting one switch to stop install scripts.** Reports on the
  `binding.gyp` campaign disagreed about whether `--ignore-scripts` was
  enough. Test the behaviour you rely on (SEC-SUP-033).
- **Upgrading to a version published minutes ago.** The 2026 incidents
  above were detected within minutes to hours; a wait of a few days would
  have kept their malicious versions out of our lockfiles.
- **Loading code from someone else's CDN.** The polyfill.io domain changed
  hands and served malicious redirects to visitors of over 100,000 sites
  (June 2024).
- **Opaque binary blobs that the build can read.** The xz backdoor
  (CVE-2024-3094) was hidden in test files and pulled into the library by
  build scripts in the release tarballs. Media servers need binary test
  fixtures, so ours are generated, hashed, licensed and never touched by a
  build script.
- **TOTP or SMS as maintainers' only second factor.** A maintainer of
  `debug` and `chalk` was compromised in September 2025 (the phishing detail
  is unverified in this pass); crates.io users were targeted from a
  lookalike domain the same month; GitHub is deprecating TOTP for npm in
  favour of FIDO.
- **Accepting AI-suggested package names without checking them.** See the
  hallucination rates in Design guidance, step 13.
- **Container tags that download the server at start, root containers and
  `privileged: true`.** The first makes versions unpinnable (Plex's `public`
  tag, per the operations research); the others turn a parser bug into a
  host compromise (CWE-250).
- **Repository signing keys nobody has rotated before.** Plex 1.43.0 broke
  installs from its Debian and RHEL repositories in January 2026 after a
  signing change and had to roll back (operations research, P5). Rehearse
  key rotation (SEC-SUP-054) before it is needed.
- **Automatic updates without rollback.** Same incident: automatic channels
  spread a broken build quickly.
- **Update checks that identify the installation.** They become telemetry.
  Gunmetal's check is a static fetch (SEC-SUP-051).
- **Ignoring an advisory forever.** Every exception expires (SEC-SUP-022).
- **Over-the-air app updates without signatures.** They create a second,
  weaker distribution channel next to the stores (SEC-SUP-057).
- **Depending only on CVE.** Its funding nearly lapsed in April 2025; GHSA,
  OSV and our own feed carry the same information.

## Open decisions for the project owner

> **Status, 2026-10-02.** The owner decisions from every file in
> `docs/security/` are consolidated and de-duplicated in
> [README.md](README.md#open-decisions-for-the-owner). Where the baseline
> has since chosen, or where a recommendation below disagrees with the
> README, the README and the requirement tables win.

1. **A second maintainer for releases and keys.** Recommendation: find one
   trusted person before R1 to approve releases, hold one of the TUF root
   keys and review protected paths. Trade-off: shared control and
   coordination, against the alternative of one stolen laptop or account
   being able to stop or subvert releases.
2. **JavaScript package manager.** Recommendation: pnpm, for its release-age
   gate (on by default in pnpm 11, per secondary sources), trust policy and
   exotic-source blocking; npm 12 is now a reasonable second choice since it
   also blocks scripts. Trade-off: React Native and Expo tooling is most
   often documented with npm or Yarn, so pnpm may need extra setup
   (unverified).
3. **TUF from R1, or a simpler signed feed.** Recommendation: TUF via
   tuf-on-ci with the `tough` client. Trade-off: a key ceremony and more
   moving parts, against a hand-rolled minisign feed that has no standard
   answer for rollback, freeze or key rotation.
4. **What ships in R1.** Recommendation: static Linux binaries for three
   architectures plus the container image, with checksums, provenance and
   SBOMs. Defer deb and rpm repositories (they need their own signing keys),
   Windows and macOS (code-signing certificates and the Apple developer
   programme) and NAS packages. Trade-off: some users wait, against keys and
   channels we cannot yet protect.
5. **Container registry.** Recommendation: GHCR only at first (OIDC push,
   attestations, the same account boundary as the code). Trade-off: Docker
   Hub is where many NAS users look; a signed mirror can follow.
6. **Container user model.** Recommendation: a fixed non-root user set
   through `user:`, no root entrypoint that switches users. Trade-off: NAS
   users must match media ownership themselves; templates and `doctor`
   carry that burden.
7. **cargo-vet now.** Recommendation: yes, while the graph is tiny.
   Trade-off: every new crate costs a review or an exemption.
8. **Signed commits on `main`.** Recommendation: yes. Trade-off: almost none
   with squash merges; contributors who merge locally are blocked.
9. **FFmpeg in R2.** Recommendation: official packages bundle a pinned
   FFmpeg build; a system FFmpeg is allowed with a `doctor` warning.
   Trade-off: we take on tracking FFmpeg advisories, against running
   whatever the host has.
10. **Over-the-air app updates.** Recommendation: never. Trade-off: slower
    fixes through store review.
11. **App-store licence permission.** The client research flags that the
    AGPL may conflict with Apple's store terms. It decides whether iOS and
    tvOS signed builds exist at all, and must be settled before outside
    contributions arrive (legal advice needed).
12. **Publishing crates or npm packages.** Recommendation: publish nothing
    until someone needs it, but reserve the names now. Trade-off: none
    beyond the reservation chores.
13. **Cyber Resilience Act status.** Is Premier Studio a manufacturer
    (monetising Gunmetal through hosting, support or paid features), an
    open-source steward, or neither? This needs legal advice; reporting
    deadlines for manufacturers have applied since 11 September 2026.
14. **Who sees the version.** Recommendation: show version and commit only
    to signed-in users (ASVS 5.0 13.4.6) and keep the source link on the
    sign-in page generic. Trade-off: a reporter or admin troubleshooting
    from outside sees less.
15. **Coding-agent sandbox.** Recommendation: agents and builds run in a
    container or VM without the password manager, signing keys or registry
    tokens. Trade-off: some setup and less convenience on the main
    workstation.
16. **A `security@` mailbox.** Recommendation: create one on the project
    domain for reporters without a GitHub account. Trade-off: one more inbox
    to watch.
17. **Network egress control in CI.** Recommendation: rely on the native
    `--network=none` build step rather than a third-party egress-control
    action, which would itself need deep trust. Trade-off: fetch steps stay
    unmonitored, but they only resolve locked, checksummed packages.

## Sources

Project files: `README.md`, `CONTRIBUTING.md`, `AGENTS.md`, `SECURITY.md`,
`Cargo.toml`, `Cargo.lock`, `.github/workflows/ci.yml`, `scripts/gate.sh`,
`docs/adr/0001-architecture.md`, `docs/adr/0002-music-is-first-class.md`,
`docs/research/setup-migration-and-operations.md` (OPS-3, OPS-10, OPS-11,
OPS-12, P5), `docs/research/clients-platforms-and-offline.md`,
`docs/research/users-sharing-and-security.md`.

Standards and specifications:

- OWASP Top 10 2025, A03 Software Supply Chain Failures: https://top10.owasp.org/2025/A03_2025-Software_Supply_Chain_Failures
- OWASP Top 10 2025, A08 Software or Data Integrity Failures: https://top10.owasp.org/2025/A08_2025-Software_or_Data_Integrity_Failures
- OWASP Top 10 2025 introduction: https://owasp.org/Top10/2025/0x00_2025-Introduction/
- OWASP ASVS 5.0, V15, V13 and V3: https://raw.githubusercontent.com/OWASP/ASVS/master/5.0/en/0x24-V15-Secure-Coding-and-Architecture.md, https://raw.githubusercontent.com/OWASP/ASVS/master/5.0/en/0x22-V13-Configuration.md, https://raw.githubusercontent.com/OWASP/ASVS/master/5.0/en/0x12-V3-Web-Frontend-Security.md
- OWASP Top 10 CI/CD Security Risks: https://owasp.org/www-project-top-10-ci-cd-security-risks/
- OWASP MASVS controls: https://raw.githubusercontent.com/OWASP/masvs/master/controls/MASVS-CODE-2.md, https://raw.githubusercontent.com/OWASP/masvs/master/controls/MASVS-CODE-3.md, https://raw.githubusercontent.com/OWASP/masvs/master/controls/MASVS-RESILIENCE-2.md
- OpenSSF OSPS Baseline v2026.08.28: https://baseline.openssf.org/versions/2026-08-28
- OpenSSF Scorecard checks: https://github.com/ossf/scorecard/blob/main/docs/checks.md
- SLSA v1.2: https://slsa.dev/spec/v1.2/whats-new, https://slsa.dev/spec/v1.2/source-requirements, https://slsa.dev/blog/2025/11/announce-slsa-v1.2
- NIST SSDF draft v1.2 and news: https://csrc.nist.gov/pubs/sp/800/218/r1/ipd, https://csrc.nist.gov/projects/ssdf/news
- NIST SP 800-63B-4: https://csrc.nist.gov/pubs/sp/800/63/b/4/final, https://pages.nist.gov/800-63-4/sp800-63b.html
- The Update Framework specification 1.0.36: https://theupdateframework.github.io/specification/latest/
- RFC 9116: https://www.rfc-editor.org/rfc/rfc9116.html
- CycloneDX 1.7: https://cyclonedx.org/news/cyclonedx-v1.7-released
- SPDX 3.1 release candidate: https://spdx.dev/spdx-3-1-ontology-and-schema-available-for-review/
- REUSE Specification 3.3: https://reuse.software/spec-3.3/
- FSF licence list: https://www.gnu.org/licenses/license-list.en.html

Tools and platform documentation:

- cargo-deny: https://embarkstudios.github.io/cargo-deny/checks/index.html, https://embarkstudios.github.io/cargo-deny/checks/advisories/cfg.html, https://embarkstudios.github.io/cargo-deny/checks/bans/cfg.html
- RustSec advisory database: https://github.com/RustSec/advisory-db
- cargo-vet: https://github.com/mozilla/cargo-vet
- cargo-auditable and scanner support: https://github.com/rust-secure-code/cargo-auditable, https://trivy.dev/docs/v0.67/coverage/language/rust/
- Cargo unstable features (publish-time, trim-paths): https://doc.rust-lang.org/nightly/cargo/reference/unstable.html
- RFC 3923, Cargo minimum publish age: https://rust-lang.github.io/rfcs//3923-cargo-min-publish-age.html
- Package manager cooldowns: https://nesbitt.io/2026/03/04/package-managers-need-to-cool-down
- pnpm supply-chain security: https://pnpm.io/supply-chain-security
- npm 12 install-time defaults: https://github.blog/changelog/2026-07-08-npm-install-time-security-and-gat-bypass2fa-deprecation/, https://www.infoq.com/news/2026/08/npm-12-released/
- GitHub's npm hardening plan: https://github.blog/security/supply-chain-security/our-plan-for-a-more-secure-npm-supply-chain/
- Dependabot default cooldown: https://github.blog/changelog/2026-07-14-dependabot-version-updates-introduce-default-package-cooldown/
- Actions SHA-pinning policy: https://github.blog/changelog/2025-08-15-github-actions-policy-now-supports-blocking-and-sha-pinning-actions/
- pull_request_target change: https://github.blog/changelog/2025-11-07-actions-pull_request_target-and-environment-branch-protections-changes/
- Immutable releases: https://github.blog/changelog/2025-10-28-immutable-releases-are-now-generally-available/
- Artifact attestations: https://docs.github.com/en/actions/concepts/security/artifact-attestations
- Environments: https://docs.github.com/en/actions/reference/workflows-and-actions/deployments-and-environments
- Push protection: https://docs.github.com/en/code-security/secret-scanning/introduction/about-push-protection
- Requesting a CVE from a repository advisory: https://docs.github.com/en/code-security/security-advisories/working-with-repository-security-advisories/publishing-a-repository-security-advisory
- CodeQL supported languages: https://codeql.github.com/docs/codeql-overview/supported-languages-and-frameworks/
- zizmor audits: https://docs.zizmor.sh/audits/
- Sigstore Rekor v2 and cosign v3: https://blog.sigstore.dev/rekor-v2-ga/, https://blog.sigstore.dev/
- tuf-on-ci: https://github.com/theupdateframework/tuf-on-ci
- tough: https://github.com/awslabs/tough
- rust-tuf: https://docs.rs/crate/tuf
- crates.io trusted publishing: https://blog.rust-lang.org/2025/07/11/crates-io-development-update-2025-07/
- Distroless: https://github.com/GoogleContainerTools/distroless
- Docker reproducible builds: https://docs.docker.com/build/ci/github-actions/reproducible-builds/
- Expo EAS Update code signing: https://docs.expo.dev/eas-update/code-signing/
- Android developer verification: https://developer.android.com/developer-verification

Incidents and advisories:

- tj-actions/changed-files, CVE-2025-30066: https://github.com/advisories/GHSA-mrrh-fwg8-r2c3
- Nx compromise: https://github.com/nrwl/nx/security/advisories/GHSA-cxm3-wv7p-598c
- Ultralytics analysis (PyPI): https://blog.pypi.org/posts/2024-12-11-ultralytics-attack-analysis/
- xz, CVE-2024-3094: https://github.com/advisories/GHSA-rxwq-x6h5-x525
- polyfill.io: https://sansec.io/research/polyfill-supply-chain-attack
- debug 4.4.2 report: https://github.com/debug-js/debug/issues/1005
- CISA alert on Shai-Hulud: https://www.cisa.gov/news-events/alerts/2025/09/23/widespread-supply-chain-compromise-impacting-npm-ecosystem
- Shai-Hulud 2.0: https://blog.checkpoint.com/research/shai-hulud-2-0-inside-the-second-coming-the-most-aggressive-npm-supply-chain-attack-of-2025/
- TanStack postmortem: https://tanstack.com/blog/npm-supply-chain-compromise-postmortem
- Mini Shai-Hulud: https://www.stepsecurity.io/blog/mini-shai-hulud-is-back-a-self-spreading-supply-chain-attack-hits-the-npm-ecosystem
- binding.gyp campaign: https://snyk.io/blog/node-gyp-supply-chain-compromise-self-propagating-npm-worm-binding-gyp, https://www.stepsecurity.io/blog/binding-gyp-npm-supply-chain-attack-spreads-like-worm
- ChainDrop: https://stepsecurity.io/blog/chaindrop-npm-worm
- arrayref and related crates: https://thehackernews.com/2026/08/rust-supply-chain-attack-puts-build.html, https://semgrep.dev/blog/2026/rust-crates-arrayref-append-only-vec-compromised-proc-macro1, https://osv.dev/vulnerability/MAL-2026-14333
- faster_log and async_println: https://blog.rust-lang.org/2025/09/24/crates.io-malicious-crates-fasterlog-and-asyncprintln
- crates.io phishing campaign: https://blog.rust-lang.org/2025/09/12/crates-io-phishing-campaign/
- Package hallucination study (USENIX Security 2025): https://arxiv.org/abs/2406.10279
- CVE programme funding: https://therecord.media/cisa-extends-cve-program-contract-with-mitre
- Cyber Resilience Act reporting (secondary sources): https://www.noze.it/en/insights/cyber-resilience-act-24-hours-start-today/, https://www.ecija.com/en/news-and-insights/cyber-resilience-act-obligations-to-report-vulnerabilities-and-incidents-from-september-2026/
