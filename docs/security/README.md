# Security baseline

Date: 2026-10-02. Status: proposed; the owner decisions at the end are open.

Web tools were available. The facts this index adds were checked against
primary sources on 2026-10-02: Tailscale's Serve documentation (Funnel
traffic carries no identity headers), Docker's rootless troubleshooting
page (the userland proxy does not pass the client's source address
through), Let's Encrypt's rate-limit page (50 new certificates per
registered domain every 7 days, with registered domains taken from the
Public Suffix List), the W3C WebAuthn Level 3 Recommendation of
25 August 2026 (the PRF extension is section 10.1.4), RFC 9106 section 4,
and NIST's page for SP 800-38D (under revision since March 2024; the
section number of its random-IV limit is unverified). Standard versions
are those pinned in [standards-coverage.md](standards-coverage.md): OWASP
ASVS 5.0.0, Top 10:2025, API Security Top 10 2023, MASVS 2.1.0 with
MASTG 2.0.0, CWE Top 25 (2025) and NIST SP 800-218 SSDF 1.1 (1.2 is still
a draft).

Gunmetal is a media server that non-experts will put on the internet,
that holds what a household watches and listens to, and that parses huge
amounts of untrusted input. Security is therefore designed in before the
features, not added after them. This directory is the baseline every
feature file, plan and line of code is held to.

| File | What it owns |
|---|---|
| [threat-model.md](threat-model.md) | Assets, adversaries, trust boundaries, the threat register, cross-cutting rules, and the single sources: release scope, control ownership, security parameters and the egress inventory |
| [identity-and-access.md](identity-and-access.md) | Accounts, sign-in, sessions, pairing, roles, sharing by invitation, recovery |
| [web-and-api-security.md](web-and-api-security.md) | Routes, authorization, cookies, headers, CSP, capability URLs, rate limits, share links |
| [network-and-remote-access.md](network-and-remote-access.md) | TLS, cleartext, postures, reverse proxies, the name service, iroh, relays |
| [client-and-device-security.md](client-and-device-security.md) | The web client and native apps, device keys, offline grants, TVs |
| [media-and-parser-safety.md](media-and-parser-safety.md) | Parsers, budgets, the scan worker, the FFmpeg jail, isolation tiers, files and symlinks |
| [plugins-and-integrations-security.md](plugins-and-integrations-security.md) | The egress client, API keys, plugins, webhooks, compatibility adapters |
| [operations-and-incident-response.md](operations-and-incident-response.md) | First run, secrets, audit log, alerts, backups, upgrades, host privilege, incident response |
| [privacy-and-data-protection.md](privacy-and-data-protection.md) | Data classes, retention, admin visibility, providers, backups and export |
| [supply-chain-and-release.md](supply-chain-and-release.md) | Repository protections, CI, dependencies, releases, update feed, disclosure |
| [rival-security-history.md](rival-security-history.md) | What went wrong at Plex, Jellyfin, Emby, Navidrome and Immich, and the rules that stop it here |
| [standards-coverage.md](standards-coverage.md) | Coverage against ASVS, Top 10, API Top 10, MASVS, CWE Top 25 and SSDF, and the SEC-STD requirements that fill the gaps |
| [secure-coding.md](secure-coding.md) | The short guide for contributors: forbidden sinks, untrusted input, and how security code is tested (SEC-STD-037) |

## First principles

These rules are absolute. Every feature, document, plan item and line of
code follows them, and a requirement that breaks one is a defect in the
requirement.

1. **Deny by default.** Every route, message and action declares who may
   use it, or it does not build; anything unknown is refused.
2. **Every input is hostile.** Media files, tags, artwork, subtitles,
   playlists, provider responses, backups, plugins, other servers and our
   own clients are parsed by memory-safe code, under budgets, in a
   separate process.
3. **Location is never identity.** A LAN address, a VPN, a proxy header or
   a "home" network can only add friction, never grant access.
4. **No credential crosses cleartext.** Over plain HTTP, every peer but
   loopback gets a help page and nothing else.
5. **No secret in a URL query, a log or an error.** Secrets travel in
   fragments, headers or bodies; capability paths are short-lived, bound
   and re-checked; logs make secrets unrepresentable.
6. **No passwords.** Passkeys, device keys, OIDC and pairing: nothing to
   stuff, phish, sniff or reset by email.
7. **Least privilege for every process and every person.** The server runs
   unprivileged, native decoders run jailed, and host-equivalent actions
   are owner-only with fresh user verification.
8. **One door per risk.** One authorization layer, one egress client, one
   command builder, one credential verifier, one owner per control and
   one value per parameter.
9. **Nothing leaves without the owner's choice, and nothing reaches in.** No
   telemetry, no kill switch, no remote configuration, no code pushed to a
   server.
10. **History is sensitive personal data.** Admins see who is playing and
    totals, never what someone listened to; privacy settings start at
    their most private.
11. **Revocation bites on the next request**, and everyone can see and end
    their own sessions and devices.
12. **Fail closed, visibly.** Missing isolation turns a feature off and
    says so; it never runs unconfined. Errors are typed, and generic to
    clients.
13. **Secure defaults, not options.** A control people will switch off is
    a broken control, so every control is designed for a TV remote and a
    grandparent.
14. **Every requirement is a test.** Each requirement names how it is
    verified, tests carry its ID, and the release fails if one is missing.
15. **Assume the project can be compromised.** Releases are signed,
    reproducible and attested, and no single person, key or service can
    change what runs on someone's server.

## How accounts and access work

This is the recommended model, and the baseline assumes it. It needs the
identity architecture record (SEC-STD-006) before any server code stores a
user.

**There is no Gunmetal account.** Each server is its own island. Accounts
live on the server, and no project service knows who uses it or what they
play. In R1 a server uses no project service by default: HTTPS comes from
the owner's own domain, a tailnet name or the machine itself
(SEC-NET-013; register D-07). From R2 a server may also use the project's
per-server name service, which gives it an HTTPS address and holds a
random label and a public key and nothing about people (SEC-HIS-061).

**Who can sign in.** Only people the owner lets in:

- **The owner** is the person who claimed the server. Only the owner can do
  the things that amount to controlling the host: trusted proxies, remote
  access, plugins, backup restore, ownership transfer.
- **Administrators** manage people, libraries and policies. They can add a
  library folder, but only with a fresh fingerprint or face check.
- **Members** are household and friends. They play what they are granted.
- **Guests** get one library, often for a limited time.
- From R2: **managed profiles** for children, with no credential of their
  own, and **household devices** such as the living-room TV, which show a
  chosen set of profiles and work only on the home network by default.

**Setting up.** On first start the server prints a claim link and a QR code
on its console: `https://<name>/claim#<code>` on the owner's own domain or
tailnet name, or a localhost link on the machine itself (with the name
service, from R2, `https://<label>.<zone>/claim#<code>`).
The owner scans it with a phone, creates a passkey, and is the owner. The
code is 128 bits, lasts 24 hours, appears only on the host, and wrong
guesses from elsewhere cannot use it up (SEC-IAM-007, SEC-IAM-008).

**How people sign in.**

- **Passkeys** (Face ID, fingerprint, Windows Hello, a security key) are
  the normal way. Synced passkeys survive a lost phone.
- **OIDC** (from R1.2), if the owner already runs an identity provider. It never makes
  someone the owner, and it is never enough on its own for the owner's
  most dangerous actions (SEC-IAM-107).
- **Approve from your phone** when a browser cannot use a passkey (an old
  smart-TV browser, a borrowed laptop): the browser shows a code, the
  person approves it from a device where they are signed in, and the
  browser gets a limited session that can play but never administer
  (SEC-IAM-108).
- From R2, **native apps** hold a key in the phone's or TV's secure
  hardware, and TVs pair by showing a QR code that a phone approves. When
  the TV is somewhere else, the person types the TV's code and compares a
  matching code, so a QR picture sent by a stranger is not enough
  (SEC-IAM-060).
- There are no passwords, no authenticator-app codes, no security
  questions and no emailed or texted codes (SEC-IAM-025).

**On which devices.** R1: any current browser over HTTPS, or on the server
itself. R2: Android, iOS, Android TV and Apple TV apps. Later: Samsung and
LG TVs.

**What people see.** Each person sees only the libraries they are granted.
Administrators see who is streaming, on which device and how, but not the
title unless that person chose to show it, and never anyone's history
(SEC-PRV-025). Everyone has a page that shows exactly what the admins can
see about them, and a private listening mode. The owner can read the
security log, with other people's addresses shortened (SEC-OPS-027).

**Sessions.** A browser stays signed in for 30 days (7 if unused).
Administration needs a separate, short admin session (15 minutes idle,
1 hour at most), and the most dangerous actions also need a fingerprint
within the last 5 minutes (SEC-IAM-041). Everyone can see their sessions
and devices and end any of them, and that takes effect on the next request.

**Sharing.**

- **Invitations** are single-use links that last 7 days and carry the
  access they grant. When an invitation gives more than one library, the
  inviter confirms the new person by comparing a short code, so a
  forwarded link cannot quietly let a stranger in (SEC-IAM-079).
- **Share links** (R1.2, music) let anyone listen to one track, album or
  playlist without an account. They are listen-only by default, expire
  after 30 days, can have a password, allow 2 streams at once, and
  suspend themselves and tell the sharer if they spread widely
  (SEC-API-097). Video share links come in R2, off by default.

**Recovery**, from the top rung down:

1. Another passkey: synced passkeys survive a lost phone.
2. Recovery codes, offered to the owner and administrators when they
   enrol, and available to everyone under Account > Recovery.
3. The identity provider, for OIDC accounts (from R1.2).
4. An administrator's recovery link, redeemed in person or on a device the
   person already approved.
5. For the owner only, a command on the host.

A credential added through a code or an admin link starts a 72-hour hold.
During the hold it cannot remove other credentials or export history, the
person's existing devices can cancel it with one tap, and after an admin's
link the person's history stays hidden until the hold ends, so recovery
cannot be used to snoop (SEC-IAM-106). The owner gets one printable recovery
kit that holds both the recovery codes and the backup key (SEC-PRV-040).

## Security in the test-first process

The project builds test first, with 100% coverage and zero surviving
mutants. Security requirements are held to the same rules:

- **Every requirement names its proof.** The "Verified by" column says
  which test or check proves it: unit or property test, integration test
  against the real binary and a real SQLite file, fuzzing, a test in a
  Linux network namespace, a browser end-to-end test, a CI check, or a
  named manual review.
- **Tests carry requirement IDs.** A test proves a requirement by naming
  its ID (in its name or an attribute). The release workflow fails when a
  requirement due in that release has no test or dated review record, and
  publishes the traceability report (SEC-STD-004).
- **Lowest layer first.** The policy function, parsers, token and
  capability logic, the address classifier, lifetime calculators and
  lockout schedules are pure functions in `gunmetal-core`, proved by unit
  and property tests with zero surviving mutants. Higher layers test the
  wiring: the route table, the cross-user matrix, the egress namespace.
- **Generated suites stop gaps.** The route table generates the
  anonymous-request suite, the cross-user matrix, the route-tag check and
  the cleartext replay, so a new route cannot ship untested.
- **The documents are tested too.** Docs lints check that cited IDs exist,
  that Release values are allowed, that no live row cites a withdrawn one,
  and that no requirement restates a parameter or header differently from
  its owner (SEC-TM-072 to SEC-TM-075, SEC-STD-001, SEC-STD-006). A live
  requirement's Release is R1, R1.1, R1.2, R1.3, R2, R3 or Later; a retired
  one is Withdrawn. The feature map uses the same release values, with No
  for a feature that is never built. R1.1 to R1.3 are the point releases
  the owner adopted on 2026-10-02 (register D-10). A requirement in a point
  release is mandatory for that release, which cannot ship without it
  (SEC-STD-004).

**How the gate grows.** `scripts/gate.sh` stays the definition of done; new
checks join it or run beside it in CI.

| When | Checks added |
|---|---|
| Now, before more code | cargo-deny (advisories, bans, licences, sources) and cargo-vet; `--locked` everywhere; secret scanning with push protection; zizmor, actionlint and CodeQL on workflows; OpenSSF Scorecard; Dependabot with a cooldown; a fuzz target for the EBML parser with a nightly run; the docs lints; release-profile `overflow-checks` |
| With the first server code (R1) | The traceability check; route-table suites (policy declared, auth allowlist, route tags, cross-user matrix, cleartext replay); network-namespace egress and posture tests; header and cookie golden tests; clippy disallowed methods and types for process, socket, HTTP-client, randomness and crypto use; the binary hardening check; dynamic fuzzing of the running API from its OpenAPI document; nightly fuzzing of every parser |
| With the web client (R1) | ESLint security rules (no HTML injection, the regexp super-linear rules, no unsafe merges); a browser run of every screen failing on any CSP violation; the npm signature and release-age checks; a nightly passive web scan |
| R2 | Sanitizer runs of the player over the malformed-media corpus; static analysis of every APK and IPA and a MASTG pass per native release; jail-escape and hostile-plugin suites; the external security assessment |
| Every release | The security evidence bundle (coverage, mutation, fuzz statistics, cargo-deny and cargo-vet, CodeQL, Scorecard, traceability and standards coverage) with provenance (SEC-STD-005); the non-author review before R1 (SEC-STD-034) |

## Requirements by file

Counts are of real table rows, recounted on 2026-10-03 after the
requirements were realigned to the owner's answers to D-07 and D-10.
Withdrawn rows are kept so their IDs stay stable; each points to the
requirement that now owns its content.

| File | IDs | R1 | R1.1 | R1.2 | R1.3 | R2 | R3 | Later | Live total | Withdrawn |
|---|---|---|---|---|---|---|---|---|---|---|
| [Threat model](threat-model.md) | SEC-TM | 48 | 0 | 1 | 0 | 10 | 1 | 0 | 60 | 15 |
| [Identity and access](identity-and-access.md) | SEC-IAM | 70 | 0 | 11 | 0 | 17 | 0 | 5 | 103 | 7 |
| [Web and API](web-and-api-security.md) | SEC-API | 85 | 0 | 1 | 0 | 4 | 1 | 3 | 94 | 5 |
| [Network and remote access](network-and-remote-access.md) | SEC-NET | 43 | 0 | 0 | 0 | 22 | 1 | 1 | 67 | 5 |
| [Clients and devices](client-and-device-security.md) | SEC-CLI | 21 | 0 | 1 | 0 | 35 | 1 | 5 | 63 | 9 |
| [Media and parsers](media-and-parser-safety.md) | SEC-MED | 55 | 2 | 0 | 0 | 22 | 2 | 1 | 82 | 0 |
| [Plugins and integrations](plugins-and-integrations-security.md) | SEC-EXT | 8 | 0 | 0 | 0 | 66 | 1 | 1 | 76 | 0 |
| [Operations and incident response](operations-and-incident-response.md) | SEC-OPS | 64 | 0 | 1 | 0 | 5 | 1 | 2 | 73 | 2 |
| [Privacy and data protection](privacy-and-data-protection.md) | SEC-PRV | 44 | 4 | 1 | 0 | 7 | 1 | 1 | 58 | 2 |
| [Supply chain and release](supply-chain-and-release.md) | SEC-SUP | 55 | 0 | 0 | 0 | 6 | 0 | 4 | 65 | 1 |
| [Rival security history](rival-security-history.md) | SEC-HIS | 50 | 1 | 0 | 0 | 5 | 1 | 0 | 57 | 9 |
| [Standards coverage](standards-coverage.md) | SEC-STD | 33 | 0 | 2 | 0 | 1 | 0 | 1 | 37 | 3 |
| **All files** | | **576** | **7** | **18** | **0** | **200** | **10** | **24** | **835** | **58** |

## The R1 security cut

Nothing ships in R1 until every requirement below is met and its test or
review record exists (SEC-STD-004). There are 576. The short names are
abbreviations of the requirement text, which is authoritative.

This is the cut for the R1 the owner adopted on 2026-10-02 (register
[D-10](../decisions.md#d-10-r1-scope-and-the-release-table) and
[D-07](../decisions.md#d-07-https-and-naming-record-8)). Of the 607
requirements the earlier R1 held, 31 protect only a surface that now ships
later, so they moved with it; they are listed under "Due after R1" below.
None was weakened or dropped. Every other requirement stays
due in R1, including those that also protect the R1 paths to HTTPS (own
domain, tailnet or localhost, SEC-NET-013, SEC-NET-072), the pre-claim
egress rule (SEC-OPS-007) and every requirement the register lists as
having an R1 carrier.

### Threat model (48)

| ID | Short name |
|---|---|
| SEC-TM-001 | Feature file in docs/features: name the trust boundaries it crosses and the TM-T |
| SEC-TM-002 | This threat model: reviewed before each release tag and whenever an entry |
| SEC-TM-003 | Fixed vulnerability: land with a regression test |
| SEC-TM-004 | Server: reject every request that lacks a valid session |
| SEC-TM-005 | Route: registered with an explicit authorization policy |
| SEC-TM-006 | Server: never ask a router to open ports |
| SEC-TM-009 | HTTP listener: reject a Host that is not localhost |
| SEC-TM-010 | When the server terminates TLS: allow only TLS 1.3 and 1.2 with forward-secret suites |
| SEC-TM-012 | Shipped binaries: contain no default account |
| SEC-TM-014 | Authentication pathway: listed in one inventory and apply the same per-account |
| SEC-TM-017 | Host-equivalent actions: carry the fresh-uv tag of SEC-IAM-041 |
| SEC-TM-024 | Read and write of a user-visible: pass through one authorization layer that takes the subject |
| SEC-TM-025 | For every route: replay one user's object IDs as a second user |
| SEC-TM-026 | Library grants: applied by the server when building every response |
| SEC-TM-027 | Write endpoints: bind only an allowlist of fields |
| SEC-TM-028 | Disabling a user: take effect on the next request |
| SEC-TM-031 | Data from media files: enter as an untrusted type and be converted |
| SEC-TM-032 | Parser of untrusted input: enforce budgets for element size |
| SEC-TM-033 | Parser and decoder entry point: a fuzz target run in CI on each change |
| SEC-TM-034 | Server process: never decode untrusted images |
| SEC-TM-035 | Server: never send attacker-supplied image bytes to clients |
| SEC-TM-036 | Clients: render untrusted text |
| SEC-TM-038 | XML parser: DTD processing and external entity resolution disabled |
| SEC-TM-039 | SQL: fixed statements with bound parameters |
| SEC-TM-040 | Errors returned to clients: generic typed codes with no stack traces |
| SEC-TM-041 | Published service units: run the server as a dedicated unprivileged user |
| SEC-TM-042 | Server: never create |
| SEC-TM-043 | File access: resolved beneath a configured root through descriptor-relative opens |
| SEC-TM-044 | Native decoder: run in a separate jailed process with no network |
| SEC-TM-045 | Features that need isolation: follow the isolation table of SEC-MED-024 |
| SEC-TM-046 | API: never set an executable path |
| SEC-TM-048 | Server: make no outbound connection |
| SEC-TM-049 | Server signing and root keys: live only in key files under SEC-OPS-012 |
| SEC-TM-050 | Stored field and every log: carry a data classification |
| SEC-TM-051 | Accounts: live in the durable |
| SEC-TM-052 | Backups containing identity data: encrypted with an authenticated cipher under a key stored |
| SEC-TM-053 | Server and clients: send no telemetry |
| SEC-TM-054 | By default admins: see only live sessions |
| SEC-TM-055 | History and audit logs: documented retention with automatic deletion |
| SEC-TM-057 | Logs: make secrets unrepresentable |
| SEC-TM-058 | Web client: hold its session only in an HttpOnly cookie |
| SEC-TM-067 | Neither the project nor any third: able to disable |
| SEC-TM-068 | Server: enforce per-user and global limits on concurrent streams |
| SEC-TM-069 | Media root that is missing: marked offline and must never cause deletion of items |
| SEC-TM-072 | One security-parameters table in this file: hold every numeric and boolean security choice |
| SEC-TM-073 | Security control: exactly one owning requirement in the control-ownership table |
| SEC-TM-074 | Release-scope table in this file: the single source for which surfaces exist |
| SEC-TM-075 | Egress inventory in this file: list every outbound purpose with its default |

### Identity and access (70)

| ID | Short name |
|---|---|
| SEC-IAM-001 | Identity model: keep credentials |
| SEC-IAM-002 | Request: resolve to exactly one principal of a closed set |
| SEC-IAM-003 | Exactly one account: hold the owner role at all times |
| SEC-IAM-004 | Accounts: live in durable storage that cache rebuilds never touch |
| SEC-IAM-005 | Server: ship with no default account |
| SEC-IAM-006 | Until it is claimed: answer only the claim page |
| SEC-IAM-007 | Claim code: 128 bits from a CSPRNG |
| SEC-IAM-008 | Claim code: accepted only from loopback or from a secure context |
| SEC-IAM-009 | Claiming: consume the code |
| SEC-IAM-010 | Server: reject any request whose Host |
| SEC-IAM-013 | Network location and other context signals: never widen what a principal may do |
| SEC-IAM-014 | Web client: served by the user's own server on the same |
| SEC-IAM-015 | HTML response: send a Content-Security-Policy that allows no inline script |
| SEC-IAM-016 | WebSocket upgrades: check Origin against the configured origins |
| SEC-IAM-017 | Web client: keep no session or API token in localStorage |
| SEC-IAM-018 | WebAuthn ceremonies: use a relying-party ID fixed at setup |
| SEC-IAM-019 | WebAuthn challenges: at least 128 bits from a CSPRNG |
| SEC-IAM-020 | Registration: request a discoverable credential with user verification required |
| SEC-IAM-021 | Server: store each credential's signature counter and its backup-eligible |
| SEC-IAM-022 | Sign-in: usernameless and must not reveal |
| SEC-IAM-023 | Account: able to hold several credentials |
| SEC-IAM-024 | Removing an account's last credential: refused unless the account is being deleted |
| SEC-IAM-025 | Server: never offer account passwords |
| SEC-IAM-037 | First-party session and access tokens: opaque values with at least 256 bits |
| SEC-IAM-038 | New session token: issued at sign-in |
| SEC-IAM-040 | State-changing request authenticated by cookie: carry the client's custom request header and an Origin |
| SEC-IAM-041 | Browser sessions: end after 7 days without use or 30 days |
| SEC-IAM-042 | User: able to list their own sessions |
| SEC-IAM-043 | Revoking a session: make every token |
| SEC-IAM-044 | Administrators: able to end any or all sessions |
| SEC-IAM-046 | Byte-serving routes: check on every request that the session behind |
| SEC-IAM-047 | Stream signatures: redacted before any log line or diagnostic bundle |
| SEC-IAM-056 | Pairing user codes: 8 characters from the RFC 8628 §6.1 base-20 alphabet |
| SEC-IAM-057 | Pairing QR code: carry the identity key of the server the requesting |
| SEC-IAM-058 | Approval screen: show the requesting device's self-reported name marked as unverified |
| SEC-IAM-059 | Device or browser authorisation: never grant owner or administrator capabilities |
| SEC-IAM-060 | Approval counts as local: type the code shown on the requesting device rather |
| SEC-IAM-067 | HTTP: declare its authorisation policy in one route table |
| SEC-IAM-068 | Authorisation: decided by one deny-by-default pure function in the core |
| SEC-IAM-069 | Error: end in denial and a security-log entry |
| SEC-IAM-070 | Object fetch: pass through one shared visibility predicate built |
| SEC-IAM-071 | Cross-principal suite generated from the route: replay every route with object IDs belonging to another |
| SEC-IAM-072 | Request bodies: decoded into per-action types that hold only the fields |
| SEC-IAM-073 | Principal: never able to grant |
| SEC-IAM-074 | Authorisation code: test capabilities and never role names |
| SEC-IAM-075 | Owner-only capabilities listed in design guidance: never grantable to any other principal |
| SEC-IAM-076 | Changes to roles: apply from the next request of every affected session |
| SEC-IAM-077 | Administrator's access to another user's data: recorded in that user's own visible security log |
| SEC-IAM-078 | Invitations: carry a secret of at least 128 bits |
| SEC-IAM-079 | Redeeming an invitation: enrol the invitee's own passkey |
| SEC-IAM-080 | Guests: by default have no household-device access |
| SEC-IAM-081 | Until an accepted architecture record defines: authorised on the strength of another server's assertion about |
| SEC-IAM-089 | Owners and administrators: offered 10 single-use recovery codes of at least 80 |
| SEC-IAM-090 | Recovery by code or link: notify all of the account's devices |
| SEC-IAM-091 | Administrators: issue one-time recovery enrolment links for members and guests |
| SEC-IAM-092 | Owner recovery: available only through a command on the host |
| SEC-IAM-094 | Security-log entries: hash-chained |
| SEC-IAM-095 | Secrets: held in a wrapper type that cannot be formatted |
| SEC-IAM-096 | Log fields derived from requests: written as structured |
| SEC-IAM-097 | Users: able to read their own security events |
| SEC-IAM-098 | Account's devices: notified of a new device |
| SEC-IAM-099 | Failed sign-in: also be written as single lines in a stable |
| SEC-IAM-101 | Endpoint that checks a secret: rate-limited per source and server-wide |
| SEC-IAM-102 | Server: enforce documented limits |
| SEC-IAM-103 | Disabling an account: end its sessions at once |
| SEC-IAM-104 | User: able to see a page stating what administrators |
| SEC-IAM-105 | Backups that contain identity data: encrypted before they leave the host |
| SEC-IAM-106 | Credential enrolled through a recovery code: start a recovery hold |
| SEC-IAM-107 | Owner-only and fresh-uv actions: satisfied only by a passkey or device key enrolled |
| SEC-IAM-108 | Person whose browser cannot use: able to sign it in by approval |

### Web and API (85)

| ID | Short name |
|---|---|
| SEC-API-001 | HTTP and WebSocket route: registered through one typed route table |
| SEC-API-002 | Set of routes whose class: exactly equal a checked-in allow-list file |
| SEC-API-003 | Route: answer 401 with a byte-identical body |
| SEC-API-004 | Session tokens: accepted only from the session cookie |
| SEC-API-005 | Unauthenticated responses: never reveal the software version |
| SEC-API-007 | Server: reject with 421 any request whose Host |
| SEC-API-008 | Only the methods declared: used |
| SEC-API-009 | HTTP/1.1 requests with both Content-Length: rejected |
| SEC-API-010 | Handlers: obtain stored objects only through an authorisation layer |
| SEC-API-011 | For every route that accepts: never see the object must receive the same 404 status |
| SEC-API-012 | Request that carries several object identifiers: authorise every identifier and must reject the whole request |
| SEC-API-013 | Acting principal: come only from the authenticated credential |
| SEC-API-014 | Library visibility: applied inside the authorisation layer to every read path |
| SEC-API-015 | Library sync deltas: computed per principal |
| SEC-API-016 | Server-pushed event: filtered per recipient through the same policy layer |
| SEC-API-017 | Change to anything authorisation depends: take effect on the next HTTP request |
| SEC-API-018 | Byte-serving path: open only the file path recorded at scan time |
| SEC-API-019 | Route: declare the capability it requires |
| SEC-API-020 | Credential whose scope is narrower: evaluated as the intersection of its scope |
| SEC-API-022 | Folder browser used to choose library: admin-only |
| SEC-API-023 | Identifier visible outside the server: carry at least 128 bits from a CSPRNG |
| SEC-API-024 | Identifiers: typed by kind |
| SEC-API-025 | Pagination cursors and other continuation tokens: either opaque server-side handles or MAC-protected |
| SEC-API-026 | Audio: served only from capability URLs whose path |
| SEC-API-027 | Capability URL lifetimes |
| SEC-API-028 | After the MAC and expiry checks: confirm that the bound session is still active |
| SEC-API-029 | Media routes: authenticate only by the URL capability |
| SEC-API-030 | Media URL keys: at least 256 bits from a CSPRNG |
| SEC-API-031 | Server: answer a Range header that names more |
| SEC-API-032 | Web client served: authenticate with a single cookie named __Host-gm_session set |
| SEC-API-033 | Cookie-authenticated API request: carry the header Gunmetal-Request |
| SEC-API-034 | Cookie-authenticated request whose Sec-Fetch-Site is cross-site: refused with 403 |
| SEC-API-035 | Routes that take a body: accept only Content-Type |
| SEC-API-036 | GET and HEAD routes: never change application state other than last-seen timestamps and audit |
| SEC-API-038 | HTTPS responses served under a hostname: carry Strict-Transport-Security with max-age of at least one year |
| SEC-API-039 | Signing out: invalidate the session on the server |
| SEC-API-040 | API: send no CORS headers by default |
| SEC-API-041 | WebSocket upgrade: require a valid credential |
| SEC-API-042 | Client that cannot send a cookie: authenticate by sending |
| SEC-API-043 | WebSocket message: at most 64 KiB |
| SEC-API-044 | HTML response: carry a Content Security Policy equivalent to default-src 'none' |
| SEC-API-045 | Web client's build: fail on any HTML-string or code-string sink |
| SEC-API-046 | Text that comes from files: rendered as text |
| SEC-API-047 | URL taken from metadata or user: rendered as a link or opened by the app |
| SEC-API-048 | Untrusted strings: normalised on ingest |
| SEC-API-049 | Web client: load no third-party scripts |
| SEC-API-050 | Message event listener in the web: check event.origin against an exact list and validate |
| SEC-API-051 | Content that comes from users: never served from the application origin with a script-capable type |
| SEC-API-052 | Web client: check at start-up for the features it depends |
| SEC-API-053 | Response: carry X-Content-Type-Options |
| SEC-API-054 | Response with a body: declare a Content-Type that matches the body |
| SEC-API-055 | Authenticated JSON responses: carry Cache-Control |
| SEC-API-056 | Lockout applies only to guessable secrets |
| SEC-API-057 | Rate limits: keyed on the principal when there |
| SEC-API-058 | Sign-in: indistinguishable in status and body whether the account |
| SEC-API-060 | Server: enforce |
| SEC-API-061 | Server: time out header reading after 10 seconds |
| SEC-API-062 | If HTTP/2 is enabled: cap concurrent streams per connection at 100 |
| SEC-API-063 | List and search routes: cap page size at 500 unless the route declares |
| SEC-API-064 | Expensive operations: limited to authorised principals |
| SEC-API-065 | Request bodies with any Content-Encoding: refused with 415 |
| SEC-API-066 | SQL text: static |
| SEC-API-067 | Body: decode into a typed request structure that rejects unknown |
| SEC-API-068 | Responses: built from explicit per-role response types |
| SEC-API-069 | Absolute URLs the server generates: built from the configured public URL |
| SEC-API-070 | Post-sign-in return target: a relative path that starts with a single / |
| SEC-API-071 | CSV export: quote fields as RFC 4180 describes and must neutralise |
| SEC-API-072 | Error: an RFC 9457 problem-details object whose type comes |
| SEC-API-073 | Last-resort layer: turn any panic or unexpected error in request handling |
| SEC-API-075 | Authentication or authorisation decision: never grant access |
| SEC-API-076 | Outbound network request made: go through a single egress crate |
| SEC-API-077 | Egress client: resolve names itself and refuse the request |
| SEC-API-078 | Egress client: allow only https |
| SEC-API-079 | Outbound purpose: declare its allowed hosts |
| SEC-API-080 | Server: never fetch a URL that a user |
| SEC-API-081 | Responses from external services: decoded into typed structures with size limits |
| SEC-API-085 | Upload route: declare its allowed types |
| SEC-API-086 | Images: decoded only by memory-safe Rust decoders with width |
| SEC-API-087 | Stored uploads and extracted artwork: named by a server-generated content hash inside a server-controlled |
| SEC-API-088 | Uploads: limited per principal by count and total bytes |
| SEC-API-090 | Lyrics from tags: parsed into a timed-line model capped at 256 KiB |
| SEC-API-091 | OpenAPI description: generated from the route table at build time |
| SEC-API-092 | Native API: versioned in its path |
| SEC-API-095 | Access log: record method |
| SEC-API-096 | Invitation link: carry a secret of at least 128 bits |

### Network and remote access (43)

| ID | Short name |
|---|---|
| SEC-NET-001 | Over plaintext HTTP: give every peer other than loopback only a static |
| SEC-NET-002 | TLS listener: negotiate only TLS 1.3 |
| SEC-NET-003 | Server: present the complete certificate chain |
| SEC-NET-004 | Certificate renewal: automatic |
| SEC-NET-005 | When no valid certificate is available: never serve the web client |
| SEC-NET-006 | TLS private keys: generated on the server from the operating system's CSPRNG |
| SEC-NET-009 | Outbound TLS: validate certificates against the WebPKI with hostname checks |
| SEC-NET-013 | Browser HTTPS: also work without the project name service |
| SEC-NET-014 | Server: answer 421 Misdirected Request to any request whose Host |
| SEC-NET-015 | Absolute URL the server emits: built from the configured canonical origin for the path |
| SEC-NET-016 | Server: ignore Forwarded |
| SEC-NET-017 | When a peer: ignore the headers |
| SEC-NET-018 | For requests from trusted proxies: the first address that is not a trusted proxy |
| SEC-NET-019 | Owner: declare each trusted proxy as "private overlay" or "public" |
| SEC-NET-020 | WebSocket upgrades: refused unless Origin |
| SEC-NET-021 | HTTP/1.1 front end: reject ambiguous framing |
| SEC-NET-022 | Project: ship reverse-proxy configurations for Caddy |
| SEC-NET-023 | Server: never treat Tailscale identity headers |
| SEC-NET-024 | In the default home posture: serve only a static help page |
| SEC-NET-025 | Address classification: convert IPv4-mapped IPv6 addresses to IPv4 before classifying them |
| SEC-NET-027 | When a request from a non-local: record a security event |
| SEC-NET-028 | At startup and whenever interfaces change: show the admin which listeners are bound to globally |
| SEC-NET-030 | Server: never request router port mappings |
| SEC-NET-031 | In each documented configuration: equal the documented list exactly |
| SEC-NET-032 | In each configuration the server: connect out only to the destinations in the egress |
| SEC-NET-036 | Invite and pairing secrets: travel only in URL fragments |
| SEC-NET-045 | Admin operations: refused on internet-posture paths |
| SEC-NET-046 | Metrics: off by default |
| SEC-NET-047 | Unauthenticated responses: never reveal the server version |
| SEC-NET-048 | HTTP front end: enforce a global connection cap |
| SEC-NET-049 | HTTP/2: allow at most 100 concurrent streams per connection |
| SEC-NET-050 | Byte-range request: name at most one range |
| SEC-NET-051 | State created for unauthenticated clients: live in fixed-capacity stores with eviction |
| SEC-NET-052 | Rate-limit and connection-cap keys: the IPv4 address or hierarchical IPv6 prefixes |
| SEC-NET-053 | Expensive authenticated operations: run under per-user and global concurrency limits with queueing |
| SEC-NET-054 | Iroh endpoint: cap total connections |
| SEC-NET-055 | On the reference low-end profile: play without client buffer underrun while the server receives |
| SEC-NET-056 | Server: log network security events as structured, injection-safe records |
| SEC-NET-057 | Project: keep a cryptographic inventory listing every key |
| SEC-NET-058 | Web client used on the LAN: served by the Gunmetal server from the same origin |
| SEC-NET-059 | Core server: never implement or open SSDP |
| SEC-NET-068 | Peer whose address equals the default: classified "unknown" and treated as non-local |
| SEC-NET-072 | Server: alert the owner 30 and 7 days |

### Clients and devices (21)

| ID | Short name |
|---|---|
| SEC-CLI-001 | Web and native clients: render every string that originates from media files |
| SEC-CLI-002 | URL taken from metadata or user: rendered as a link only if it parses |
| SEC-CLI-004 | Server response to clients: carry X-Content-Type-Options |
| SEC-CLI-005 | Artwork and other user-derived content: reach clients only as an allow-listed raster type |
| SEC-CLI-007 | Server: reject any state-changing request from a browser whose Sec-Fetch-Site |
| SEC-CLI-009 | On sign-out: delete that account's data from IndexedDB |
| SEC-CLI-010 | At sign-in the web client: ask |
| SEC-CLI-011 | Server: reject API calls from a web client bundle |
| SEC-CLI-012 | Web client's scripts: ship inside the signed server release and be served |
| SEC-CLI-013 | Secret carried by a link: travel in the URL fragment |
| SEC-CLI-015 | Restriction a client applies or displays: also be enforced by the server on every request |
| SEC-CLI-016 | Client build: never contain an API key |
| SEC-CLI-017 | Client build: produce a CycloneDX SBOM that includes native libraries |
| SEC-CLI-018 | CI: install JavaScript dependencies only from the committed lockfile |
| SEC-CLI-019 | Development servers: bind to loopback in the committed configuration |
| SEC-CLI-020 | Library data the server sends: contain only what the signed-in profile may access |
| SEC-CLI-021 | Clients: decode every server response with schema-validating decoders that bound |
| SEC-CLI-024 | Server: assign every enrolled device a class |
| SEC-CLI-025 | Inbound links and codes: parsed by one pure function in the core |
| SEC-CLI-027 | Client: never include analytics |
| SEC-CLI-028 | PIN: mask input and turn off autocorrect |

### Media and parsers (55)

| ID | Short name |
|---|---|
| SEC-MED-001 | Public parsing function in gunmetal-core: return Ok or a typed error for every possible |
| SEC-MED-002 | Non-test code in gunmetal-core: compile with the Clippy lints unwrap_used |
| SEC-MED-003 | Gunmetal-core: never size any allocation from a declared length or count |
| SEC-MED-004 | Offset: combined with checked arithmetic and converted with fallible conversions |
| SEC-MED-005 | Core: count nesting depth for every nested structure and return |
| SEC-MED-006 | Core: enforce the element-count |
| SEC-MED-007 | Core parse: charge a deterministic step budget and fail |
| SEC-MED-008 | Loop over elements: strictly advance or stop |
| SEC-MED-009 | Decompression: go through one streaming helper |
| SEC-MED-010 | Core's sans-I/O interface: never request a read longer than 16 MiB or past |
| SEC-MED-011 | Formats: detected from content signatures against a closed allowlist |
| SEC-MED-012 | Server: serve bytes only for items that content detection indexed |
| SEC-MED-013 | Text from media metadata: decoded with invalid sequences replaced |
| SEC-MED-014 | Identifiers: validated into typed values with documented ranges when parsed |
| SEC-MED-015 | Player: clamp gain taken from tags to the range −30 |
| SEC-MED-016 | Server: never fetch |
| SEC-MED-017 | When a limit or parse error: keep the rest of the file's metadata |
| SEC-MED-018 | Parsing of media: run in separate worker processes |
| SEC-MED-019 | File that crashes or times out: quarantined until its size or modification time changes |
| SEC-MED-020 | Worker: receive input only as read-only file descriptors |
| SEC-MED-021 | Worker: single-threaded and must run |
| SEC-MED-022 | On Linux the worker: in order |
| SEC-MED-023 | Server: treat messages from workers and sandboxes as untrusted |
| SEC-MED-024 | At startup and on demand: self-test each sandbox profile |
| SEC-MED-025 | Server and worker binaries: never link C or C++ media |
| SEC-MED-026 | Third-party crate that parses or decodes: admitted only after a recorded review |
| SEC-MED-027 | Public parsing entry point in gunmetal-core: a fuzz harness whose body is a plain function |
| SEC-MED-028 | Cargo test on stable: replay the committed corpus of every harness |
| SEC-MED-029 | Coverage-guided fuzzing: run on every pull request that changes gunmetal-core |
| SEC-MED-030 | Fuzzing finding: fixed test-first |
| SEC-MED-031 | Container format: also be fuzzed by a structure-aware generator that emits |
| SEC-MED-033 | Library access: go through directory handles opened once per configured root |
| SEC-MED-034 | Symlink: followed only when its whole chain resolves |
| SEC-MED-035 | Server: open files with O_NONBLOCK |
| SEC-MED-036 | Before serving bytes: open the item beneath its root and confirm |
| SEC-MED-037 | Server: refuse a library root that is a filesystem root |
| SEC-MED-038 | Server: open library content read-only and must not create |
| SEC-MED-039 | Names from media: never used as path components |
| SEC-MED-040 | Server: keep file names as raw bytes for access |
| SEC-MED-041 | Blocking filesystem calls: run on a bounded pool with per-root concurrency limits |
| SEC-MED-042 | Official container image and example compose: run as a non-root user |
| SEC-MED-044 | Artwork decoding: accept only JPEG |
| SEC-MED-045 | Image width and height: read from the header and checked before any pixel |
| SEC-MED-046 | Clients: receive only derivatives the server generated |
| SEC-MED-047 | Image endpoints: accept a size only from a fixed enumeration |
| SEC-MED-048 | Derivative cache: bounded in total bytes |
| SEC-MED-049 | LRC: parsed into a typed model within the limits table |
| SEC-MED-051 | Playlist entries: returned only for items in libraries the requesting user |
| SEC-MED-057 | Clients: render media-derived strings only as text nodes inside |
| SEC-MED-058 | URL from metadata: shown as a link only after it has been |
| SEC-MED-059 | Byte-serving responses: carry |
| SEC-MED-060 | Range parser: accept at most one byte range per request |
| SEC-MED-062 | Media-derived strings and paths written: escaped |
| SEC-MED-063 | Server: never start any external program except through the sandbox launcher |
| SEC-MED-077 | Clients: apply the core's parsers and limits to everything |

### Plugins and integrations (8)

| ID | Short name |
|---|---|
| SEC-EXT-001 | Server-initiated outbound network connection: go through the single egress client |
| SEC-EXT-002 | Egress client: resolve names itself |
| SEC-EXT-003 | Egress client: never follow redirects unless the caller opts |
| SEC-EXT-004 | Egress client: enforce a connect timeout |
| SEC-EXT-005 | Outbound requests: carry no identifier of the server |
| SEC-EXT-006 | Native API: authenticate requests only from the Authorization header |
| SEC-EXT-007 | Credential: carry an immutable kind |
| SEC-EXT-018 | Until SEC-EXT-019 to SEC-EXT-034 are implemented: never load or execute any plugin or other third-party code |

### Operations and incident response (64)

| ID | Short name |
|---|---|
| SEC-OPS-001 | Server: never ship |
| SEC-OPS-003 | While unclaimed: answer only the setup page and its assets |
| SEC-OPS-004 | Setup code attempts: compared in constant time and limited as SEC-IAM-008 requires |
| SEC-OPS-005 | Claim: a single atomic transaction |
| SEC-OPS-006 | After the claim: never become reachable again |
| SEC-OPS-007 | Before it is claimed: make no outbound connection |
| SEC-OPS-008 | Restoring a backup onto an unclaimed: require the same setup code as claiming |
| SEC-OPS-009 | Owner recovery: possible only through a command on the host |
| SEC-OPS-011 | Server key and secret: come from the OS CSPRNG |
| SEC-OPS-012 | Secrets: live as files with mode 0600 |
| SEC-OPS-013 | Secret values: held in a type with no printable |
| SEC-OPS-014 | Server: never accept secrets as command-line arguments |
| SEC-OPS-015 | Cryptographic purpose: use its own key |
| SEC-OPS-016 | Session tokens: stored only as keyed hashes |
| SEC-OPS-017 | Secrets the server: replay to other systems |
| SEC-OPS-018 | Owner: able to rotate every server secret in one action |
| SEC-OPS-019 | Trust root for the project's update: compiled into the binary and replaced only through signed |
| SEC-OPS-020 | Server: keep a security audit log |
| SEC-OPS-021 | Audit record: carry a sequence number |
| SEC-OPS-022 | Log output: one JSON object per line with every value escaped |
| SEC-OPS-023 | Audit log: tamper-evident |
| SEC-OPS-024 | Backup: include the latest signed audit checkpoint |
| SEC-OPS-026 | Retention: enforced automatically on the schedule of SEC-PRV-005 |
| SEC-OPS-027 | Only the owner and holders: read the full audit log |
| SEC-OPS-028 | Failed authentication on any surface: also be written to the diagnostic log |
| SEC-OPS-029 | Diagnostic logging: default to info level and must never record request |
| SEC-OPS-031 | At startup the server: detect any configuration change made outside |
| SEC-OPS-032 | Server: raise owner alerts |
| SEC-OPS-033 | Alert about a device or credential: offer a one-step "This wasn't me" action that revokes |
| SEC-OPS-034 | Alerts for a new admin: never switchable off and must never be suppressed |
| SEC-OPS-037 | Exposure detection: all take the client address and its class |
| SEC-OPS-038 | Home posture: the default in every official package and container image |
| SEC-OPS-039 | Server: never ask a router for port mappings |
| SEC-OPS-040 | Iroh endpoint: keep router port mapping off unless the owner has |
| SEC-OPS-041 | Backups: run by default |
| SEC-OPS-042 | Backup: encrypted in age v1 to the server's backup key |
| SEC-OPS-043 | Backups: signed with a server backup-signing key |
| SEC-OPS-044 | After any restore the server: rotate all symmetric keys and invalidate every session |
| SEC-OPS-045 | Downloading a backup: require the owner role and a re-authentication |
| SEC-OPS-046 | Server: never replace or modify its own executable or install directory |
| SEC-OPS-047 | Update and advisory check: a required first-run question with two explicit answers |
| SEC-OPS-048 | Before any schema or format migration: take a snapshot and check it with PRAGMA integrity_check |
| SEC-OPS-049 | Migration: never make an existing installation less strict |
| SEC-OPS-050 | Startup: never reveal the version |
| SEC-OPS-051 | Older binary: refuse to open durable state written in a newer |
| SEC-OPS-052 | Release's notes: say whether it migrates data |
| SEC-OPS-053 | On Unix-like systems the server: refuse to start when its effective user is root |
| SEC-OPS-054 | Server: open media roots read-only and never create |
| SEC-OPS-055 | Media-serving path: open a file only when its fully resolved location |
| SEC-OPS-056 | Official systemd unit: run under a dedicated system account with no login |
| SEC-OPS-057 | Official container image: run as a fixed non-root user and work |
| SEC-OPS-059 | Metrics and diagnostics endpoints: off by default |
| SEC-OPS-060 | Server: document every outbound connection it can make |
| SEC-OPS-061 | Gunmetal doctor --security and the dashboard: report root and capability state |
| SEC-OPS-064 | Besides GitHub private vulnerability reporting: accept reports at an email alias that reaches |
| SEC-OPS-065 | Report: tracked against the response times in SECURITY.md |
| SEC-OPS-066 | Security fix: begin with a failing regression test that reproduces |
| SEC-OPS-067 | Advisory: also give affected and fixed ranges in OSV form |
| SEC-OPS-068 | Advisories: also go into the signed feed with their severity |
| SEC-OPS-069 | Project: notify the packagers of official and known community packages |
| SEC-OPS-070 | Project: publish a supported-version policy |
| SEC-OPS-071 | Signing-key compromise playbook: rehearsed at least once a year |
| SEC-OPS-072 | Project: publish a compromise runbook for server owners |
| SEC-OPS-075 | Audit log: an anchor off the host |

### Privacy and data protection (44)

| ID | Short name |
|---|---|
| SEC-PRV-001 | Server: keep a machine-readable data inventory that assigns every persisted |
| SEC-PRV-002 | History event: contain only the profile ID |
| SEC-PRV-003 | Client IP addresses: stored only in the active-session table and the security |
| SEC-PRV-004 | Server: never persist users' search queries |
| SEC-PRV-005 | Retention for every data class: defined in one schedule in code with the defaults |
| SEC-PRV-007 | With the default configuration: cause no outbound connection or non-local DNS lookup |
| SEC-PRV-008 | Outbound requests from the server: go through one egress component that enforces a per-feature |
| SEC-PRV-009 | Server and every first-party client: never send telemetry |
| SEC-PRV-012 | Admin: able to route all server egress through an HTTP |
| SEC-PRV-013 | Metadata: never enabled by default |
| SEC-PRV-016 | Clients: fetch artwork |
| SEC-PRV-018 | Web response: send Referrer-Policy |
| SEC-PRV-019 | Web client: never keep Activity |
| SEC-PRV-020 | Responses carrying Activity: send Cache-Control |
| SEC-PRV-021 | IDs and signed URLs exposed: opaque random values of at least 128 bits |
| SEC-PRV-022 | One user's Activity data: never returned to any other user |
| SEC-PRV-023 | Privacy setting: default to its most private value |
| SEC-PRV-024 | Client: let a user start a private session |
| SEC-PRV-025 | Admin interface and admin API: never offer any view |
| SEC-PRV-026 | Product: never offer any way for an admin to sign |
| SEC-PRV-027 | User: able to read a "what your admin can see" |
| SEC-PRV-030 | Emails and in-app notifications: never contain another user's Activity data |
| SEC-PRV-031 | Share-link pages and any other page: never reveal the sharer's username |
| SEC-PRV-033 | Scrobbling and every other feature: off for every user by default |
| SEC-PRV-034 | Linking an external scrobbling account: bind the callback to the initiating user's session |
| SEC-PRV-035 | Scrobbling: submit only plays recorded after the link time |
| SEC-PRV-036 | Unlinking an external service: delete its stored credential and discard queued |
| SEC-PRV-038 | Secret-store key: generated on first start from a CSPRNG |
| SEC-PRV-039 | Backup archive: encrypted in the age v1 format |
| SEC-PRV-040 | Setup: issue one printable Gunmetal recovery kit holding |
| SEC-PRV-041 | Backups: kept for a configurable period |
| SEC-PRV-042 | Logs: never contain passwords |
| SEC-PRV-043 | At the default log level: never contain media titles |
| SEC-PRV-044 | Repository: contain a log inventory listing every log event type |
| SEC-PRV-045 | Log files: created readable only by the service account |
| SEC-PRV-047 | User: able to export all of their own data |
| SEC-PRV-048 | Starting an export or an account: require authentication within the last 5 minutes |
| SEC-PRV-049 | Users: able to delete one history entry |
| SEC-PRV-050 | Database connection: set PRAGMA secure_delete=ON |
| SEC-PRV-051 | Deleting an account: disable it and end all its sessions and device |
| SEC-PRV-052 | Tombstones that carry deletions to devices: identify erased events only by ID or ID range |
| SEC-PRV-053 | Before redeeming an invitation: shown a privacy notice generated from the server's actual |
| SEC-PRV-054 | Service the project operates: publish a privacy notice |
| SEC-PRV-055 | Invitation and share secrets carried through: travel only in the URL fragment so they never |

### Supply chain and release (55)

| ID | Short name |
|---|---|
| SEC-SUP-001 | Account that can write: use phishing-resistant MFA |
| SEC-SUP-002 | Default branch: protected by a ruleset that blocks direct pushes |
| SEC-SUP-003 | Release tags: creatable only by maintainers |
| SEC-SUP-004 | Commits on the default branch: carry a verified signature |
| SEC-SUP-005 | Changes to .github/: require approval from a code owner |
| SEC-SUP-006 | Secret-scanning push protection: enabled for the repository |
| SEC-SUP-007 | Private vulnerability reporting: stay enabled |
| SEC-SUP-008 | Project site: serve /.well-known/security.txt over HTTPS with Contact |
| SEC-SUP-009 | Fixed vulnerability in Gunmetal: published as a GitHub Security Advisory with a CVE |
| SEC-SUP-010 | Uses: pinned to a full commit SHA |
| SEC-SUP-011 | Tools installed in CI: pinned to exact versions and installed with checksum verification |
| SEC-SUP-012 | Workflow: set permissions |
| SEC-SUP-013 | Workflow: never check out or execute pull-request code under pull_request_target |
| SEC-SUP-014 | Untrusted event text: never expanded with ${{ }} inside run |
| SEC-SUP-015 | Release and publishing workflows: never restore or save GitHub Actions caches |
| SEC-SUP-016 | Release workflows: authenticate only with short-lived credentials |
| SEC-SUP-017 | Publishing jobs: run in a protected release environment that accepts |
| SEC-SUP-018 | Zizmor: run on every pull request and block merging |
| SEC-SUP-019 | OpenSSF Scorecard: run weekly and on every push to main |
| SEC-SUP-020 | Cargo.lock: committed |
| SEC-SUP-021 | Cargo deny check: pass on every pull request and daily on main |
| SEC-SUP-022 | Ignored advisory: an entry in supply-chain/exceptions.toml giving the advisory ID |
| SEC-SUP-023 | Vulnerable dependencies: remediated within documented times |
| SEC-SUP-024 | Crate in the dependency graph: covered in cargo-vet by an audit |
| SEC-SUP-025 | Gunmetal-core: no normal dependencies except those in a reviewed allowlist |
| SEC-SUP-026 | Only crates on a reviewed allowlist: build scripts |
| SEC-SUP-027 | Pull request that adds or changes: fail unless it carries an override label approved |
| SEC-SUP-028 | Dependabot: cover the cargo |
| SEC-SUP-029 | Rust and JavaScript component: use a licence on the project allowlist of AGPL-3.0-or-later-compatible |
| SEC-SUP-030 | File in the repository: carry machine-readable copyright and licence information under REUSE 3.3 |
| SEC-SUP-031 | Server build: embed the source repository URL and exact commit |
| SEC-SUP-032 | Repository: never contain executables |
| SEC-SUP-033 | JavaScript installs in CI and release: frozen to the committed lockfile |
| SEC-SUP-034 | JavaScript package manager: refuse versions published less than 7 days earlier |
| SEC-SUP-035 | Direct JavaScript dependencies: listed in supply-chain/js-direct-deps.toml with a written reason |
| SEC-SUP-036 | CI: verify registry signatures |
| SEC-SUP-037 | Web client: never load scripts |
| SEC-SUP-038 | Rust toolchain: pinned to an exact version in rust-toolchain.toml |
| SEC-SUP-039 | Release compilation: run with no network access after the locked fetch |
| SEC-SUP-040 | Linux release binaries: reproducible |
| SEC-SUP-041 | Release artifact: SLSA Build Level 3 provenance from an isolated reusable |
| SEC-SUP-042 | Release: publish a SHA256SUMS manifest with a Sigstore bundle |
| SEC-SUP-043 | Container images: signed by digest |
| SEC-SUP-044 | Release: include CycloneDX 1.7 JSON SBOMs covering the Rust crates |
| SEC-SUP-045 | Official container image: built from distroless static or cc |
| SEC-SUP-046 | Image and every compose: work with all capabilities dropped |
| SEC-SUP-048 | Container images and their embedded dependency: scanned before publishing and daily afterwards |
| SEC-SUP-049 | Update notices and advisories: published as TUF 1.0 metadata on the project site |
| SEC-SUP-050 | Server's update check: verify the feed from a root embedded at build |
| SEC-SUP-051 | Update check: a plain GET of static metadata files |
| SEC-SUP-052 | Project's names: reserved before announcement on each registry users might search |
| SEC-SUP-053 | Scheduled job: compare published tags |
| SEC-SUP-054 | Supply-chain incident runbook: cover a malicious dependency |
| SEC-SUP-055 | Coding agents and other automation working: run without access to release credentials |
| SEC-SUP-056 | Release: a unique version |

### Rival security history (50)

| ID | Short name |
|---|---|
| SEC-HIS-001 | Server: never base any authentication or authorization decision |
| SEC-HIS-003 | When trusted proxies are configured: found by walking the forwarding chain from right |
| SEC-HIS-004 | Server: no passwordless sign-in for any network |
| SEC-HIS-005 | HTTP and WebSocket route: declare an authorization policy when it is registered |
| SEC-HIS-006 | Endpoint that returns media-derived bytes: require a session credential or a signed stream token |
| SEC-HIS-007 | Endpoint: exist at exactly one route |
| SEC-HIS-009 | Handlers: take the acting user only from the authenticated session |
| SEC-HIS-010 | Read or write of a user-owned: pass through one authorization function that checks |
| SEC-HIS-011 | Response: never contain another account's credentials |
| SEC-HIS-012 | Object: come from a CSPRNG with at least 128 bits |
| SEC-HIS-013 | Administrative functions: require an explicit admin permission checked on the server |
| SEC-HIS-014 | Remote control of playback sessions: limited to the actor's own sessions unless the target |
| SEC-HIS-015 | Server: never build a filesystem path from untrusted data |
| SEC-HIS-016 | File access: go through directory handles that confine resolution |
| SEC-HIS-017 | Scanner: classify files by the content of the resolved target |
| SEC-HIS-019 | Server: never extract archives |
| SEC-HIS-020 | External programs: started only by one typed command builder |
| SEC-HIS-021 | Paths to external programs: come only from the install or a read-only host |
| SEC-HIS-023 | Outbound requests: use one egress client that allows only http |
| SEC-HIS-024 | Server: never fetch a URL taken from an unauthenticated request |
| SEC-HIS-026 | TLS clients: verify certificates |
| SEC-HIS-027 | Web client: render all text that comes from media files |
| SEC-HIS-028 | HTML response: carry a Content-Security-Policy with no unsafe-inline or unsafe-eval |
| SEC-HIS-030 | Images from files or users: decoded by a memory-safe raster decoder with pixel |
| SEC-HIS-031 | Response carrying file-derived bytes: set a Content-Type chosen by the server |
| SEC-HIS-032 | Redirect target taken from a request: a same-origin relative path that starts with exactly |
| SEC-HIS-033 | Client-reported metadata: validated on the server to a bounded length |
| SEC-HIS-034 | XML parser in the server: reject DOCTYPE declarations and must not resolve external entities |
| SEC-HIS-035 | Server: never deserialise any format that can construct arbitrary types |
| SEC-HIS-036 | Parser of untrusted input: live in the core crate under its no-panic |
| SEC-HIS-037 | Request parameter that sizes work: a bounded type with explicit limits |
| SEC-HIS-038 | Database access: use parameterised queries whose column names |
| SEC-HIS-041 | Native API: never accept session tokens |
| SEC-HIS-042 | Stream and share tokens: checked on every request for signature |
| SEC-HIS-043 | Server secret: generated by a CSPRNG at first start |
| SEC-HIS-044 | Session tokens: stored only as keyed hashes |
| SEC-HIS-046 | Authentication pathway: go through one credential verifier that applies the same |
| SEC-HIS-047 | Authentication: fail for a username that does not exist |
| SEC-HIS-052 | Server: never create router port mappings through UPnP-IGD |
| SEC-HIS-053 | LAN discovery: use mDNS |
| SEC-HIS-055 | On every start the server: log |
| SEC-HIS-056 | API: never install |
| SEC-HIS-058 | CI workflows: never run untrusted pull-request code with secrets or write tokens |
| SEC-HIS-059 | Releases: signed with build provenance and a software bill |
| SEC-HIS-060 | User's play history: never shown or sent to any other non-admin user |
| SEC-HIS-061 | Project services: hold only the minimum routing data the per-server name |
| SEC-HIS-063 | Server: notify a user when a device |
| SEC-HIS-064 | Security fix: include a regression test that reproduces the exploit |
| SEC-HIS-065 | Vulnerability fixed in a released version: published as a GitHub security advisory with a CVE |
| SEC-HIS-066 | "rival exploit replay" test suite: contain at least one test per incident |

### Standards coverage (33)

| ID | Short name |
|---|---|
| SEC-STD-001 | Repository: hold machine-readable copies of the pinned standard versions |
| SEC-STD-002 | CI: regenerate the coverage tables in this file |
| SEC-STD-003 | Scheduled job: check monthly for new versions of ASVS |
| SEC-STD-004 | SEC requirement whose release: referenced by at least one test |
| SEC-STD-005 | Release: publish |
| SEC-STD-006 | Before any server code stores: decide whether account passwords and TOTP exist in R1 |
| SEC-STD-010 | Gunmetal: never include SAML |
| SEC-STD-011 | Regular expressions applied to untrusted input: run only in a linear-time engine |
| SEC-STD-012 | Client: keep data keyed by untrusted strings |
| SEC-STD-013 | API responses: JSON with Content-Type |
| SEC-STD-014 | Cookie the server sets: a name and value of at most 4096 bytes |
| SEC-STD-015 | Links to any origin outside: accept only https and http |
| SEC-STD-016 | Domain the project operates: send Strict-Transport-Security with max-age of at least 63072000 |
| SEC-STD-017 | Server: serve web assets only from a build-time manifest |
| SEC-STD-018 | Project: keep a cryptographic inventory |
| SEC-STD-019 | Cryptography: come only from implementations on a reviewed allow-list chosen |
| SEC-STD-020 | AEAD key: a nonce strategy fixed in code |
| SEC-STD-021 | Decryption: return one opaque error that does not reveal |
| SEC-STD-022 | Security randomness: come from the operating system CSPRNG through one function |
| SEC-STD-023 | Main server process: disable core dumps |
| SEC-STD-024 | Key derived from a human secret: use Argon2id with at least the second recommended RFC |
| SEC-STD-026 | Server: never act as an OAuth authorization server for third-party clients |
| SEC-STD-027 | Flow: never show an approval prompt on a person's device |
| SEC-STD-029 | Single-use or counted secret: consumed by one conditional update inside a single SQLite |
| SEC-STD-030 | Project: keep one machine-readable register of business limits |
| SEC-STD-031 | SQLite file the server did: opened read-only in a jailed worker with trusted_schema off |
| SEC-STD-033 | Release binaries: built position-independent with full RELRO and non-executable stacks |
| SEC-STD-034 | Before the R1 tag: review the threat model |
| SEC-STD-035 | Source paths that implement authentication: listed in CODEOWNERS |
| SEC-STD-036 | SECURITY.md or a governance file: name the security lead |
| SEC-STD-037 | CONTRIBUTING.md: link a short secure-coding guide drawn from these files |
| SEC-STD-038 | CI: fuzz the running server through its generated OpenAPI description |
| SEC-STD-040 | Server: talk to its scan worker |

### Due after R1

These 31 requirements were in the earlier R1 cut. Each protects only a
surface that now ships in a point release or in R2, and is mandatory in the
release that ships that surface, which cannot ship without it
(SEC-STD-004). The R1.1 and R1.2 rows follow the register's "Security
requirements for the adopted R1"; the R2 rows follow D-07. No requirement
is due in R1.3.

| Release | Surface | ID | Short name |
|---|---|---|---|
| R1.1 | Playlist files imported or found in libraries (MUS-140, LIB-192) | SEC-MED-050 | Entries in M3U: resolve only to items already indexed in the same |
| R1.1 | Playlist files imported or found in libraries (MUS-140, LIB-192) | SEC-HIS-018 | Playlist files: resolve entries only to items in libraries the playlist |
| R1.1 | Metadata providers (LIB-111, LIB-112) | SEC-PRV-014 | Provider requests: built only from a typed lookup-evidence value holding normalised |
| R1.1 | Metadata providers (LIB-111, LIB-112) | SEC-PRV-015 | Provider lookups: run only during scans |
| R1.1 | Metadata providers (LIB-111, LIB-112) | SEC-PRV-017 | Outbound provider requests: send a User-Agent naming only the project |
| R1.1 | Avatar and other image uploads (ACC-011) | SEC-MED-061 | Uploaded artwork: size-capped while they stream |
| R1.1 | Avatar and other image uploads (ACC-011) | SEC-PRV-006 | Images uploaded by users: re-encoded with all EXIF |
| R1.2 | OIDC sign-in (ACC-057) | SEC-TM-022 | OIDC sign-in: use the authorization code flow with PKCE |
| R1.2 | OIDC sign-in (ACC-057) | SEC-IAM-026 | OIDC sign-in: use the authorization code flow with PKCE |
| R1.2 | OIDC sign-in (ACC-057) | SEC-IAM-027 | ID tokens: verified with keys from the provider's JWKS using algorithms |
| R1.2 | OIDC sign-in (ACC-057) | SEC-IAM-028 | OIDC identity: keyed only by the pair |
| R1.2 | OIDC sign-in (ACC-057) | SEC-IAM-029 | Linking an OIDC identity: happen only inside that account's session after user verification |
| R1.2 | OIDC sign-in (ACC-057) | SEC-IAM-030 | OIDC auto-registration: off by default |
| R1.2 | OIDC sign-in (ACC-057) | SEC-IAM-031 | Provider claims: never confer the owner role |
| R1.2 | OIDC sign-in (ACC-057) | SEC-IAM-032 | Server-side calls to an OIDC provider: verify TLS certificates and must not follow redirects |
| R1.2 | OIDC sign-in (ACC-057) | SEC-IAM-033 | OIDC redirect URI: one exact registered URL on the configured origin |
| R1.2 | OIDC sign-in (ACC-057) | SEC-IAM-034 | Authorization request: bound to the one provider it was sent |
| R1.2 | OIDC sign-in (ACC-057) | SEC-IAM-035 | Sessions created through OIDC: lifetimes set by Gunmetal |
| R1.2 | OIDC sign-in (ACC-057) | SEC-IAM-036 | Administrator elevation for an account: require a fresh provider sign-in |
| R1.2 | OIDC sign-in (ACC-057) | SEC-STD-025 | OIDC authorization requests: carry exactly the scopes in the provider's configuration |
| R1.2 | OIDC sign-in (ACC-057) | SEC-CLI-026 | When the web client signs: act as a confidential client |
| R1.2 | Music share links (ACC-086 to ACC-089, MUS-151) | SEC-API-097 | Public share links: use the fragment pattern |
| R1.2 | Music share links (ACC-086 to ACC-089, MUS-151) | SEC-STD-008 | Secret a person chooses: accept any Unicode characters with no composition rules |
| R1.2 | Diagnostic bundles (ADM-124, CLI-033) | SEC-PRV-046 | Diagnostic bundles: exclude the database |
| R1.2 | Diagnostic bundles (ADM-124, CLI-033) | SEC-OPS-030 | Diagnostic bundles: meet SEC-PRV-046 |
| R2 | The name service, its naming client and CT monitoring (ADM-023) | SEC-NET-010 | If the server uses: 128-bit random |
| R2 | The name service, its naming client and CT monitoring (ADM-023) | SEC-NET-011 | Name service: answer A and AAAA queries |
| R2 | The name service, its naming client and CT monitoring (ADM-023) | SEC-NET-012 | Name service: publish a CAA record for each registered label |
| R2 | The name service, its naming client and CT monitoring (ADM-023) | SEC-NET-069 | When the server uses the project: by default monitor Certificate Transparency for its own label |
| R2 | The name service, its naming client and CT monitoring (ADM-023) | SEC-NET-070 | Project name service: launch only after its zone is on the Public |
| R2 | The name service, its naming client and CT monitoring (ADM-023) | SEC-NET-071 | When the name service refuses: keep working on every other path |

## Add to the repository now

These come before more code lands. Each satisfies the requirement named.

| # | Add | Satisfies |
|---|---|---|
| 1 | Expand `SECURITY.md`: scope, supported versions, response timeframes, private reporting through GitHub, and named security lead, release manager and incident lead with deputies and a 30-day succession plan | SEC-SUP-007, SEC-STD-036 |
| 2 | Turn on private vulnerability reporting, secret scanning and push protection | SEC-SUP-006, SEC-SUP-007 |
| 3 | A ruleset on `main`: pull request required, the gate required, no force-push or deletion, verified signatures and DCO sign-off; protected `v*` tags; a settings-drift job | SEC-SUP-002, SEC-SUP-003, SEC-SUP-004 |
| 4 | `CODEOWNERS` for `.github/`, `scripts/`, `deny.toml`, `supply-chain/`, `rust-toolchain.toml`, `Cargo.lock`, `SECURITY.md`, `CODEOWNERS`, `docs/security/`, `docs/adr/` and the parser source in `crates/gunmetal-core`; agent-written pull requests labelled and reviewed as outside contributions (add the rule to `AGENTS.md`) | SEC-SUP-005, SEC-STD-035 |
| 5 | Workflow hardening: `permissions: {}` at the top of `ci.yml` (it now grants `contents: read` at the top) with read access per job; keep every action pinned by SHA; never `pull_request_target`; add zizmor, actionlint and CodeQL; no Actions cache in release workflows | SEC-SUP-010, SEC-SUP-012, SEC-SUP-013, SEC-SUP-015, SEC-SUP-018 |
| 6 | `deny.toml`: advisories denied; bans for UPnP and port-mapping crates, backtracking regex engines, non-cryptographic randomness, SAML, LDAP, GraphQL and WebRTC crates and weak crypto; a licence allow-list; crates.io as the only source | SEC-SUP-021, SEC-SUP-029, SEC-TM-006, SEC-STD-010, SEC-STD-011, SEC-STD-019, SEC-STD-022 |
| 7 | `supply-chain/` with cargo-vet and `exceptions.toml` | SEC-SUP-022, SEC-SUP-024 |
| 8 | `rust-toolchain.toml` pinned to an exact version (CI now uses the moving `stable`), CI tools pinned to exact versions, and `--locked` in every cargo command | SEC-SUP-011, SEC-SUP-020, SEC-SUP-038 |
| 9 | Release profile with `overflow-checks = true`, and `clippy.toml` disallowed methods and types for `std::process::Command`, socket connects, HTTP-client constructors and non-cryptographic randomness; deny `await_holding_lock` | SEC-STD-033, SEC-MED-063, SEC-EXT-001, SEC-STD-022, SEC-STD-029, SEC-TM-031 |
| 10 | A fuzz target for the EBML parser now, with committed corpus replay in `cargo test` and a nightly fuzz job | SEC-TM-033, SEC-MED-028 |
| 11 | Dependabot for cargo and github-actions with a cooldown, and a weekly OpenSSF Scorecard workflow | SEC-SUP-028, SEC-SUP-019 |
| 12 | A docs lint (an `xtask`) for requirement IDs, Release values, withdrawn citations, parameter conflicts and feature-to-threat links, with vendored ID lists of the pinned standards | SEC-TM-001, SEC-TM-072, SEC-TM-073, SEC-TM-074, SEC-STD-001, SEC-STD-006 |
| 13 | A test-naming convention that carries requirement IDs, and a short secure-coding guide linked from `CONTRIBUTING.md` | SEC-STD-004, SEC-STD-037 |
| 14 | REUSE licence metadata for every file, fixtures included | SEC-SUP-030 |
| 15 | Hardware security keys required on every account that can write to the repository, publish, or change the site and domain | SEC-SUP-001 |
| 16 | `/.well-known/security.txt` and an HSTS preload submission for gunmetal.tv | SEC-SUP-008, SEC-STD-016 |
| 17 | Architecture records before server code: identity and sessions; HTTPS and naming (OD-1); remuxer placement; the crypto allow-list; client security with the MAS profiles | SEC-STD-006, SEC-NET-010, SEC-MED-081, SEC-STD-019, SEC-STD-039 |
| 18 | A "Last reviewed" check on this threat model in the release workflow | SEC-TM-002 |

## Open decisions for the owner

These consolidate the open decisions of every file in this directory,
with duplicates merged. Each has a recommendation; the baseline is written
as if the recommendation is accepted. The owner answered several of them
on 2026-10-02 ([decision register](../decisions.md#owner-answers-2026-10-02)).
Where an answer differs from the recommendation, the answer wins, and the
item says so.

1. **Identity: no passwords and no TOTP** (SEC-STD-006; identity OD-2,
   threat-model OD-3, standards OD-1). *Recommendation:* adopt it, with
   passkeys, OIDC and browser pairing in R1, and withdraw the accounts
   map's password fallback and TOTP (ACC-052, ACC-053) in the feature map.
   *Trade-off:* a person with no passkey-capable device and no phone needs
   a hardware key or help from someone in the household. *Answered (D-06,
   D-10):* adopted, with passkeys and browser pairing in R1 and OIDC in
   R1.2.
2. **HTTPS for ordinary households** (network OD-1, identity OD-1,
   threat-model OD-2, web OD-1, client OD-1, rival OD-1, standards OD-2).
   Plain HTTP to LAN peers is already ruled out (SEC-NET-001). *Recommendation:* run the
   per-server name service in R1 as the install-time default, with the
   narrow pre-claim exception (SEC-OPS-007), CT monitoring and the
   launch conditions of SEC-NET-070 (Public Suffix List entry first), and
   keep own domain, tailnet and localhost as tested alternatives. If the
   PSL entry is not in place, R1 ships with the alternatives only.
   *Trade-off:* the project runs and pays for DNS and certificate
   automation and becomes a soft dependency that knows which labels
   exist; without it, R1 is only for people who can set up HTTPS.
   *Answered (D-07), differs from the recommendation:* R1 gets HTTPS
   through the owner's own domain with automatic certificates, a tailnet,
   or the same machine (SEC-NET-013). The name service, its naming client
   and its CT monitoring are R2 (SEC-NET-010 to SEC-NET-012, SEC-NET-069 to
   SEC-NET-071). Its launch conditions still apply in full when it ships.
3. **Remote browser access in R1** (network OD-2, OD-3, threat-model
   OD-12, privacy OD-11). *Recommendation:*
   in R1, remote use means the owner's reverse proxy or tailnet; iroh,
   relays and the browser edge come in R2. *Trade-off:* some R1 owners
   will port-forward, which the internet posture and exposure alerts are
   built for. *Answered:* as recommended; built-in remote access (iroh)
   arrives in R2.
4. **The update and advisory check** (operations OD-3, privacy OD-5,
   rival OD-4, threat-model OD-5). *Recommendation:* a required first-run
   question with two explicit answers and no preselection (SEC-OPS-047).
   *Trade-off:* one more setup step, against servers silently missing
   fixes.
5. **What admins see** (privacy OD-1 and OD-13, identity OD-5,
   threat-model OD-6). *Recommendation:* live sessions without
   titles unless each person opts in; no history; no household activity
   features. *Trade-off:* a parent cannot see what a teenager is playing
   without the teenager's consent; managed child profiles (R2) are the
   tool for that.
6. **Recovery** (identity OD-12, threat-model OD-1). *Recommendation:*
   recovery codes offered to owners and administrators at enrolment;
   admin-issued links redeemed in person, with the 72-hour hold; owner
   recovery only on the host. *Trade-off:* a member who loses every
   device waits up to 72 hours to see their history again.
7. **Share links** (threat-model OD-16, rival OD-7, identity OD-14).
   *Recommendation:* R1 for music, listen-only, 30 days, with per-link
   limits; video links in R2, off by default. *Trade-off:* the most
   requested sharing feature ships with friction for heavy sharers.
   *Answered (D-10):* music share links ship in R1.2, with SEC-API-097 and
   SEC-STD-008; the rest as recommended.
8. **API keys and adapters** (plugins OD-2, OD-3, OD-4, OD-11; rival OD-3).
   *Recommendation:* both in R2, never with administrator scopes, no
   plaintext LAN exception, legacy Subsonic sign-in per key, local paths
   only and 90 days. *Trade-off:* no scripting API in R1.
9. **Where the remuxer runs** (media OD-1). *Recommendation:* in a worker
   process streaming over a pipe (SEC-MED-081); record it as a new
   architecture record that supersedes the "remux in-process" wording of
   record 1 and the README diagram. *Trade-off:* a little latency and
   copying per stream.
10. **Isolation on weaker platforms** (media OD-3, OD-6, OD-7, OD-8;
    threat-model OD-9, OD-10). *Recommendation:* the tier table of
    SEC-MED-024 (memory-safe parsing runs at a reduced floor with a
    notice; native decoders need the full jail); Linux-only servers in R1,
    with macOS and Windows server builds only once their sandbox profiles
    exist (SEC-MED-082); ARMv7 only once its seccomp answer is recorded.
    *Trade-off:* no native Mac or Windows server in R1 (Docker Desktop
    covers them).
11. **Native transcoding in R1** (media OD-2, rival OD-2, threat-model
    OD-9). *Recommendation:* none in R1; Opus transcoding for mobile data
    arrives with the jail in R2. *Trade-off:* R1 plays originals only.
12. **Embedded subtitle fonts** (client OD-7). *Recommendation:* off by
    default, opt-in per library with re-serialisation (SEC-MED-054),
    replacing the client file's "on by default". *Trade-off:* some styled
    anime subtitles look plainer until a library opts in.
13. **Plugins: updates and the index** (plugins OD-5, OD-6, OD-7, OD-12;
    threat-model OD-14; supply-chain OD-3 for TUF from R1). *Recommendation:* updates need owner approval,
    with an opt-in per plugin for automatic updates 72 hours after
    publication; the index may only advise, never disable; first-party
    signed plugins first. *Trade-off:* slower plugin fixes for owners who
    do not opt in.
14. **Running as root** (operations OD-12, threat-model OD-4).
    *Recommendation:* refuse with no override; the official templates set
    a non-root user and fix directory ownership. *Trade-off:* hand-rolled
    containers that run as root fail at start with instructions.
15. **Retention and addresses** (identity OD-6, operations OD-6, privacy
    OD-8). *Recommendation:* security events for 365 days, addresses
    coarsened after 30 and removed at 90, the owner seeing shortened
    addresses; history kept until the person deletes it. *Trade-off:*
    less evidence for an old incident.
16. **Household devices** (identity OD-7, OD-8). *Recommendation:* home
    network only by default; dormant after 30 days unused and woken with
    one tap; software keys allowed but capped below administrator.
    *Trade-off:* a holiday-home TV needs a tap from a phone after a long
    absence.
17. **Profiles, PINs and parental filters** (identity OD-4, client OD-10).
    *Recommendation:* R2; in R1, children use separate accounts with
    library grants. *Trade-off:* no quick profile switching in R1.
18. **Verification targets** (standards OD-3, OD-5; web OD-12; client
    OD-14). *Recommendation:* ASVS Level 3 except chapter 2 (Level 2) and
    WebRTC (not applicable); MAS-L2 and MAS-P for the native apps with
    MAS-R excluded. *Trade-off:* about ten extra build checks and unit
    tests.
19. **People and keys** (supply-chain OD-1, operations OD-13, standards
    OD-4). *Recommendation:* find a second maintainer before R1 to review
    security-sensitive changes, hold the second TUF key and the
    name-service keys; get a non-author design review before the R1 tag
    and fund or seek sponsorship for an external assessment before R2.
    *Trade-off:* time and possibly money before the releases that raise
    exposure.
20. **Disclosure** (supply-chain OD-16, operations OD-7, OD-8, OD-9, rival
    OD-8). *Recommendation:* GitHub private reporting plus a `security@`
    mailbox, published response targets, CVEs for every fix, no paid bug
    bounty until funded. *Trade-off:* none of note.
21. **Legal position** (supply-chain OD-13, operations OD-10, privacy
    OD-15). *Recommendation:* get advice on whether the project is a
    manufacturer or open-source steward under the EU Cyber Resilience Act
    and give project services a legal home before the name service
    launches. *Trade-off:* cost and time.
22. **Metadata providers before plugins** (rival OD-5, privacy OD-2, OD-3,
    plugins OD-1, web OD-8). *Recommendation:* MusicBrainz and cover-art
    lookups built in for R1 behind the egress client and the required
    setup question, each listing the fields it sends; per-owner provider
    API keys where a provider needs one. *Trade-off:* a few providers in
    the server rather than in plugins. *Answered (D-10):* built in, but in
    R1.1 rather than R1, behind the setup question that lists what each
    provider receives (SEC-PRV-014, SEC-PRV-015, SEC-PRV-017 move to R1.1;
    SEC-PRV-013 stays R1).
23. **Never write into media folders** (media OD-14, operations OD-11,
    threat-model OD-15). *Recommendation:* never in R1 or R2.
    *Trade-off:* no NFO or tag writing for people who want it.
24. **No telemetry, ever by default** (privacy OD-4). *Recommendation:*
    none in R1 and R2; any future diagnostic shows its payload first.
25. **Smaller choices with a clear recommendation in their own file**
    (keep as recommended unless the owner objects): the home posture as
    the default (operations OD-2); untrusted forwarding headers ignored
    with internet posture rather than refused (network OD-7, now decided
    by SEC-NET-017); accepting the long playback-session deviation
    (threat-model OD-8); GPU access for transcoding off by default
    (threat-model OD-11, media OD-9); symlinks only into approved roots
    (media OD-4); the native direct-play container policy (client OD-5);
    the client trust model as an architecture record (client OD-15);
    offline grant 30 days (client OD-2, threat-model OD-13); download encryption (client OD-3, privacy OD-10);
    personal or shared browser default (client OD-4, privacy OD-9);
    Android isolated process spike (client OD-6); distribution and minimum
    OS versions (client OD-8, OD-11; supply-chain OD-11); step-up method
    (client OD-9); desktop shell (client OD-12); no over-the-air app
    updates (client OD-13, supply-chain OD-10); OIDC in R1 (identity
    OD-3; answered by D-10: R1.2); federation, LDAP and proxy-header sign-in not before their own
    records (identity OD-10, OD-11); device and stream limits (identity
    OD-13); limit defaults (media OD-5, network OD-11); image formats,
    original artwork and colour profiles (media OD-11 to OD-13,
    threat-model OD-7); update windows (media OD-15); fuzzing toolchain
    (media OD-10); address lookup, port mapping, admin origin, crypto
    provider, ports, exposure check and revocation latency (network OD-4
    to OD-12); backups, the recovery key and the identity key in backups
    (operations OD-4, OD-5); outbound alert channels (operations OD-14);
    plugin process and HTTP models (plugins OD-8, OD-9); webhooks in R2
    (plugins OD-10); whole-database encryption (privacy OD-6, OD-7);
    account deletion approval (privacy OD-14); paid sharing never
    (privacy OD-12); random IDs with a rebuildable cache (rival OD-6, web
    OD-3); DLNA not before its own record (rival OD-9); no self-updater
    (rival OD-10, supply-chain items); package manager, registry,
    container user, cargo-vet, signed commits, FFmpeg packaging,
    publishing, version visibility, agent sandbox and CI egress
    (supply-chain OD-2 to OD-9, OD-12, OD-14, OD-15, OD-17); web session
    model, media URL binding, server name on the landing page, CSP
    reporting, inline styles, PIN back-off, proxy presets, cast CORS and
    the invitation landing page (web OD-2, OD-4 to OD-7, OD-9 to OD-11,
    OD-13). Operations OD-15 (conflicts between sibling documents) is
    closed by this review.

## Challenge review, 2026-10-02

124 challenge issues were reviewed. 123 were applied, nine of them with a
recorded change to the proposed fix, and one was rejected.

**Rejected.** SEC-EXT-066 (low), "show the adapter HTTP option only when
no HTTPS name exists": superseded by the canonical cleartext rule
(SEC-NET-001), which removes the local-network HTTP exception for adapters
altogether.

**Applied with a change.**

- Native access-token lifetime: 10 minutes with no refresh tokens
  (SEC-IAM-050), not the 15 minutes one issue proposed, because the owning
  requirement already renews by a cheap signed challenge.
- Default egress: the issue asking for "no contact before the first-run
  choice" yields to the critical pre-claim issue. A headless install
  cannot ask before HTTPS exists, so the choice is made at install time,
  disclosed on the console, and limited to the name service and ACME
  before the claim (SEC-OPS-007).
- Web media: capability URLs only, as SEC-API-029 already said, not
  "cookie plus capability", because one mechanism makes revocation
  uniform and keeps media routes out of cross-site request reasoning; the
  service worker must not cache capability URLs instead.
- Pairing: a matching code is required whenever the approval is not
  proven local, not on both paths, because the local QR path already
  carries the request and the server's identity (as the SEC-IAM-058 issue
  argued).
- Isolation: macOS and Windows worker profiles are due before those
  server builds ship (SEC-MED-082, R2), not in R1, because R1 ships Linux
  servers only.
- Advisory check: an explicit answer with no preselection, rather than a
  preselected choice, to keep "every privacy setting starts at its most
  private value" (SEC-PRV-023) true.
- Gateway peers behind SNAT: per-client limiting is impossible without the
  client's address, so the fix isolates the unknown peer's bucket instead
  (SEC-NET-068).
- Admin API keys: forbidden outright; a short-lived administrator
  automation credential would need its own architecture record rather
  than being specified now.
- One cleartext rule: owned by SEC-NET-001, because the network file owns
  transport; the threat model holds the ownership table that points to it.

## Sources

Primary sources fetched on 2026-10-02 for the facts this index adds:

- Tailscale Serve, identity headers and Funnel:
  https://tailscale.com/kb/1312/serve
- Docker rootless mode troubleshooting (source IP propagation and the
  userland proxy): https://docs.docker.com/engine/security/rootless/troubleshoot/
- Let's Encrypt rate limits: https://letsencrypt.org/docs/rate-limits/
- W3C Web Authentication Level 3 (Recommendation, 25 August 2026):
  https://www.w3.org/TR/webauthn-3/
- RFC 9106, Argon2, section 4: https://www.rfc-editor.org/rfc/rfc9106.html
- NIST SP 800-38D status page: https://csrc.nist.gov/pubs/sp/800/38/d/final

Every other fact is cited in the file that states it.
