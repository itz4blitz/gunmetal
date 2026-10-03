# Secure operations and incident response

## Summary

This document sets the security rules for running a Gunmetal server and for
the project's own response to vulnerabilities. It covers the first run,
secrets and keys, the audit log and diagnostic logs, owner alerts, internet
exposure, backups, upgrades and migrations, least privilege on the host, and
the project's incident response. Everything here that exists in R1 (server,
web client, music library and player) is an R1 requirement, even where the
feature grows later.

The key decisions:

1. **Nobody can claim a server without host access.** There is no default
   password, no "first visitor becomes the owner" and no vendor account. On
   first start the server prints a single-use setup code to its console and
   to a file only the service account can read. The setup page asks for that
   code before anything else, and a fresh server is inert until it is
   claimed: no outbound traffic, no remote access, no plugins. After the
   claim, setup is gone for good. A server that loses its owner goes into a
   locked state that only a command on the host can unlock. It never reopens
   setup, which is the mistake Jellyfin fixed in 12.0 and Navidrome in
   0.64.2.
2. **Secrets are generated, stored and typed so they cannot leak.** Keys come
   from the OS random generator. They live in a 0700 directory outside the
   SQLite cache, and the server refuses to start when permissions are too
   broad. They are held in a Rust type that cannot be printed or serialised,
   and a canary test proves that no secret reaches any log, response or
   bundle. Session tokens are stored only as hashes. Short-lived signing keys
   rotate on their own. One action rotates everything and signs everyone out.
3. **A separate, tamper-evident audit log.** Security events go to an
   append-only, hash-chained log. Events use OWASP Logging Vocabulary names,
   records carry signed checkpoints, and `gunmetal audit verify` checks the
   chain. Each backup carries a checkpoint from R1. From R2, admins' native
   apps keep checkpoints too, so a rewritten log is detected outside the
   server. An admin action whose audit record cannot be written does not
   happen.
4. **Alerts the owner sees and acts on in one tap.** The owner is alerted to
   new devices, repeated failed sign-ins, new admins and access grants,
   plugin installs, unexpected internet exposure, owner recovery, backup
   downloads and known advisories. Critical alerts cannot be muted. The rest
   are deduplicated so an attack does not flood the owner. Every device alert
   has a "This wasn't me" button that revokes the device and stops its
   streams.
5. **LAN-only until the owner says otherwise.** The server ships in the
   "home posture" defined in `network-and-remote-access.md`. A request from
   the internet, whether sent directly or through a proxy the owner has not
   declared, gets only a help page, and the owner gets an alert. Leaving
   that posture (declaring a public proxy, or turning on remote access) is
   an audited owner action. The server never asks the router to open ports,
   and it trusts forwarding headers only from declared proxies, which closes
   the hole behind the Emby compromise (CVE-2023-33193). People who cannot
   get remote access easily will port-forward, so this default carries a
   lot of weight.
6. **Backups are on by default, and a stolen backup is useless on its
   own.** Backups are encrypted to the server's backup key and to a
   recovery key the owner keeps (as `privacy-and-data-protection.md`
   requires), and the server signs them. Restore parses them under strict
   limits. A restore rotates keys and asks the owner to review devices,
   because removals made after the backup date come back.
7. **Upgrades never lower security.** The server never updates itself. It
   can check a signed feed (with TUF-style protection against rollback and
   freeze attacks) that sends no identifiers, and the owner chooses at setup
   whether it runs. Every migration starts from a verified snapshot, runs in
   a transaction and fails closed. A property test proves that no migration
   loosens a security setting.
8. **Least privilege by default.** The server refuses to run as root. It
   opens media read-only and only inside configured roots. The shipped
   systemd unit must score 2.0 or lower in `systemd-analyze security`; a
   draft unit in this document scores 1.1 on systemd 261. The container
   image runs as a non-root user with a read-only root filesystem and no
   capabilities.
9. **The project answers reports quickly and publicly, but never acts on
   anyone's server.** Reports come in through GitHub private vulnerability
   reporting and an RFC 9116 `security.txt`. Every fix starts with a failing
   regression test. Each vulnerability gets a GitHub advisory with a CVE, a
   CVSS 4.0 vector, OSV version ranges and audit-log indicators, plus an
   entry in the signed feed. There is deliberately no kill switch: Emby's
   2023 remote shutdown shows why. The supported-version policy is short and
   enforced by in-app banners.

The account model is still open. `identity-and-access.md` proposes
passkeys reached over HTTPS on a name, or on `localhost`, because WebAuthn
needs a secure context and a domain-name relying party, and a browser on a
LAN IP address has neither. The biggest remaining question there is whether
the project runs a naming and certificate service. The requirements here
hold whatever is chosen: the setup code decides *who* may claim the server,
and the authentication design decides *what credential* is bound. See open
decision 1.

**How this document relates to its siblings.** Several controls are
designed in sibling documents in `docs/security/`:

- the claim flow, the security log and user alerts in
  `identity-and-access.md` (SEC-IAM-*);
- address classification and the home and internet postures in
  `network-and-remote-access.md` (SEC-NET-*);
- the advisory feed, `SECURITY.md` and `security.txt` in
  `supply-chain-and-release.md` (SEC-SUP-*);
- backup encryption, retention and diagnostic bundles in
  `privacy-and-data-protection.md` (SEC-PRV-*);
- outbound connections in `plugins-and-integrations-security.md`
  (SEC-EXT-*).

Where a requirement here overlaps one of those, the text names the sibling
requirement. This document adds the operational part: what happens on the
host, across upgrades and restores, and in the project's response. Where the
siblings disagree with each other, this document follows the stricter
value, and open decision 15 lists the conflicts.

Research note: web search and page fetches were available. The search quota
ran out partway through, so later facts were verified by fetching primary
sources directly: the OWASP ASVS 5.0.0 source files, NIST CSRC pages, the
RFC Editor, the MITRE CWE and CVE APIs, vendor docs and GitHub release and
advisory APIs. The systemd exposure score was measured with
`systemd-analyze security --offline=yes` on systemd 261. Facts taken from
the sibling research files in `docs/research/` are attributed to them.
Anything not confirmed is marked "(unverified)".

## Threats

| ID | Threat | Who (the attacker or failure) | Impact | Likelihood | Mitigated by (requirement IDs) |
|---|---|---|---|---|---|
| T-OPS-01 | Someone else claims a fresh server before the owner does. Home Assistant and Immich onboarding let whoever reaches the page first create the owner or admin account | Another person on the LAN, a compromised IoT device, or a web page using DNS rebinding | Full control of the server and every user's data; a persistent foothold in the home network | Medium | SEC-OPS-002, 003, 004, 005, 007 |
| T-OPS-02 | Default or shared credentials are exploited at scale | Botnets; Mirai used a list of 62 default logins (CISA TA16-288A) | Mass takeover | High wherever defaults exist | SEC-OPS-001 |
| T-OPS-03 | The setup code leaks through shipped logs, a screenshot or a support bundle and is replayed | Anyone who can read the leaked output | Takeover, if the server is still unclaimed | Low | SEC-OPS-002, 005, 013, 030 |
| T-OPS-04 | Setup reopens after a failure or misconfiguration (Jellyfin 12.0 PR #17369; Navidrome 0.64.2) | Remote unauthenticated attacker, triggered by a software failure | Takeover of an already-claimed server | Medium | SEC-OPS-006, 048, 051 |
| T-OPS-05 | An active man-in-the-middle intercepts the claim on the LAN | Attacker on the LAN at the moment of setup | The attacker's credential becomes the owner's | Low (claims need HTTPS or loopback under SEC-IAM-008 and SEC-IAM-011) | SEC-OPS-003, 005, 010 |
| T-OPS-06 | Secrets are read from a copy of the database, a backup or a diagnostics bundle. Navidrome kept its JWT secret in plain text in its database (CVE-2024-56362) | Anyone holding a copy | Forged sessions; decrypted third-party credentials | Medium | SEC-OPS-011, 012, 015, 016, 017, 018, 030, 042 |
| T-OPS-07 | An endpoint, error or log returns secrets. A Plex endpoint handed out the server owner's credentials (CVE-2025-34158), and Navidrome logged an admin password when setup failed (0.64.2) | Authenticated low-privilege user; anyone who reads the logs | Account and server takeover | Medium | SEC-OPS-013, 021, 029, 030 |
| T-OPS-08 | Secrets leak through the process environment, the command line or child processes | Other local users; a compromised helper process | Key theft | Low | SEC-OPS-014 |
| T-OPS-09 | Other accounts on the host read the state directory | Local user on a shared NAS or multi-user host | Full compromise | Medium | SEC-OPS-012, 073 |
| T-OPS-10 | Remote code execution in the server escalates to the host | Attacker holding a server RCE | Host takeover and movement through the home network | Medium | SEC-OPS-053, 055, 056, 057, 062 |
| T-OPS-11 | A compromised server deletes or encrypts the media library | Ransomware; RCE | Loss of irreplaceable media | Medium | SEC-OPS-054, 063 |
| T-OPS-12 | A compromised server persists by rewriting its own binary | Attacker holding a server RCE | Survives restarts and upgrades | Low | SEC-OPS-046 |
| T-OPS-13 | The update and advisory feed is forged, replayed or frozen | Network attacker, compromised host or CDN, stolen signing key | Owners are told they are safe when they are not, or are steered to a vulnerable version | Low | SEC-OPS-019, 047, 071 |
| T-OPS-14 | An attacker erases or rewrites audit records | Attacker with control of the server process or host | The incident goes undetected and cannot be investigated | Medium | SEC-OPS-020, 023, 024, 025 |
| T-OPS-15 | Log injection forges entries or attacks log tooling (Log4Shell, CVE-2021-44228) | Anyone who controls a tag, file name, user name or header | Misleading forensics; code execution in log processors | Medium | SEC-OPS-022 |
| T-OPS-16 | Logs and bundles expose viewing history, IP addresses or tokens | An admin sharing logs for support; a log aggregator | Privacy breach for every household member | Medium | SEC-OPS-021, 026, 027, 029, 030, 036 |
| T-OPS-17 | An account takeover goes unnoticed because a new device or credential was added silently | Credential thief, ex-partner, account sharer | Ongoing access to a person's history and libraries | High | SEC-OPS-032, 033, 072 |
| T-OPS-18 | Password guessing on adapter or share surfaces goes unnoticed. Navidrome's Subsonic API allowed unauthenticated brute force (GHSA-p994-r776-mw52) | Internet attacker | Account compromise | High once exposed | SEC-OPS-028, 032, 038 |
| T-OPS-19 | Privilege abuse: a new admin is created or a malicious plugin installed. The 2023 Emby attackers installed a plugin that harvested every user's credentials | Attacker with an admin session; a rogue household member | Credential theft from every user | Medium | SEC-OPS-032, 034, 058 |
| T-OPS-20 | The server is exposed to the internet by accident: a port forward, a DMZ host, IPv6 without a firewall, or Docker publishing ports past the host firewall | The owner's own misconfiguration | The whole attack surface faces internet-wide scanning | High | SEC-OPS-037, 038, 039, 040, 061, 074 |
| T-OPS-21 | Forged forwarding headers make remote clients look local or slip past rate limits (CVE-2023-33193; Navidrome GHSA-f295-6wp9-qqfg) | Internet attacker | Authentication bypass; rate-limit bypass | Medium | SEC-OPS-037 |
| T-OPS-22 | A backup is stolen | Burglar, cloud storage breach, lost USB drive | Exposure of viewing history, identities and keys | Medium | SEC-OPS-042, 045 |
| T-OPS-23 | A malicious backup is restored: path traversal, a decompression bomb or an attacker's identity store | Social engineering ("restore this file to migrate") | Takeover, file overwrite, denial of service | Low | SEC-OPS-008, 043 |
| T-OPS-24 | Restoring an old backup brings back revoked devices or admins | The owner, restoring after a failure | A thief regains access | Medium | SEC-OPS-044 |
| T-OPS-25 | A failed migration corrupts identity or audit data, or a half-migrated server keeps serving | Software defect (compare Jellyfin's 10.11 and 12.0 migration failures in the sibling research) | Data loss; authentication bypass | Medium | SEC-OPS-041, 048, 051 |
| T-OPS-26 | An upgrade quietly loosens security settings | Software defect | Exposure the owner did not choose | Medium | SEC-OPS-031, 049, 052 |
| T-OPS-27 | Servers stay unpatched for months. On 2025-08-25 Censys still saw about 314,000 Plex instances on vulnerable versions | Owner inertia; no notification channel | Exploitation of known vulnerabilities | High | SEC-OPS-047, 067, 068, 069, 070 |
| T-OPS-28 | A vulnerability report is lost, leaks before the fix, or is buried under low-quality reports, and no advisory follows | Failure of the project's process | Exploitation before a fix; lost trust | Medium | SEC-OPS-064, 065, 066, 067 |
| T-OPS-29 | A project-controlled kill switch is abused, coerced or buggy. Emby shut down compromised servers remotely in 2023 | Project insider, someone holding project keys, legal pressure | Owners lose service; trust in self-hosting is undermined | Low | SEC-OPS-068 |
| T-OPS-30 | The project's signing keys are stolen | Attacker targeting maintainers | Fake advisories or fake "update now" banners | Low | SEC-OPS-019, 071 |
| T-OPS-31 | Debug or metrics endpoints, or version strings, are exposed | Internet scanner | Easier mass targeting; data leaks | Medium | SEC-OPS-050, 059 |
| T-OPS-32 | The owner loses every credential, which leads to permanent lockout or to an insecure remote recovery path | The owner; a social engineer | Lockout or takeover | Medium | SEC-OPS-009 |
| T-OPS-33 | Alert fatigue makes owners mute or ignore alerts | Design failure | Real attacks are missed | High | SEC-OPS-034, 035 |
| T-OPS-34 | Outbound connections nobody documented leak data | Software defect; a malicious dependency | Privacy breach; exfiltration | Low | SEC-OPS-007, 060 |
| T-OPS-35 | A symlink in a media folder makes the server stream its own secrets. Navidrome allowed arbitrary file reads through library symlinks (GHSA-r5qr-m328-qcf4) | Anyone who can write to the media share | Key theft, then takeover | Medium | SEC-OPS-012, 055 |

## Requirements

Standards are cited as: OWASP ASVS 5.0.0 requirement numbers ("ASVS"), OWASP
Top 10:2025 categories ("Top 10"), OWASP API Security Top 10 2023, OWASP
MASVS control IDs, CWE IDs, NIST SP 800-63B-4 (July 2025) sections, NIST SSDF
version 1.1 (SP 800-218, February 2022) task IDs, and RFCs. SSDF 1.2 (SP
800-218r1) was still an initial public draft (December 2025) when checked,
so the final 1.1 IDs are used. "Integration test" means a test against a real
SQLite file and a real process, not a mock.

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-OPS-001 | The server must not ship, generate or accept any default, shared or hard-coded credential, and while unclaimed it must accept no sign-in at all. | ASVS 6.3.2, 13.2.3; Top 10 A07, A02; CWE-1392, CWE-1393, CWE-798; CISA Secure by Design alert on default passwords (2023-12-15) | R1 | Integration test: a fresh state directory holds no credentials, and every sign-in surface (native API, adapters, share links) rejects attempts before the claim. CI secret-scanner run over the release binary and image layers |
| SEC-OPS-002 | **Withdrawn 2026-10-02: merged into SEC-IAM-007.** The claim code definition, including `gunmetal claim-code` and the canary crawl, lives in one place. | ASVS 6.4.1, 6.5.3, 11.5.1; NIST SP 800-63B-4 §4.1.2.2; CWE-330, CWE-338, CWE-200 | Withdrawn | Proved by the tests of SEC-IAM-007 |
| SEC-OPS-003 | While unclaimed, the server must answer only the setup page and its assets, the claim and restore-at-setup endpoints and a health endpoint (SEC-IAM-006). It must accept setup only as SEC-IAM-008 allows (loopback, or a secure context on a configured origin, never through pairing or the edge), with a Host header the server recognises (SEC-IAM-010). | ASVS 15.2.3, 2.3.1; Top 10 A01, A02; API5:2023; CWE-306, CWE-1188 | R1 | Route-table enumeration test: every registered route outside the allowlist returns 503 while unclaimed, and an unclassified new route fails the build. Integration tests per path class, and with DNS-rebinding-style Host values |
| SEC-OPS-004 | Setup code attempts must be compared in constant time and limited as SEC-IAM-008 requires (10 a minute, server-wide), and each failed attempt must be reported on the host console with its source address and time. | ASVS 6.3.1, 6.5.4; NIST SP 800-63B-4 §3.2.2; CWE-307, CWE-208 | R1 | Integration test: the eleventh attempt in a minute is refused, and the console shows every failure. A clippy `disallowed-methods` lint bans `==` on secret types |
| SEC-OPS-005 | The claim must be a single atomic transaction (SEC-IAM-009). It binds the owner's first credential, records the claim in both the identity store and a marker file in the secrets directory, and consumes the code, so that concurrent claims produce exactly one owner. | ASVS 2.3.4, 6.4.1; CWE-362, CWE-367 | R1 | Integration test: 64 concurrent claims with the valid code give exactly one success and 63 typed "already claimed" errors; a claim after a restart also fails |
| SEC-OPS-006 | After the claim, setup must never become reachable again: not through a restart, misconfiguration, a failed or partial migration, a deleted or corrupted identity store, or removal of the last owner credential. A claimed server with no usable owner credential must start in a locked state that only host-side recovery (SEC-OPS-009) can leave. | ASVS 16.5.3, 2.3.1; Top 10 A10, A07; CWE-288, CWE-636, CWE-1188 | R1 | Fault-injection integration tests: delete the owner row, truncate the identity store, delete the database with the marker present, interrupt a migration. Each must end in the locked state, with 404 on the setup routes. Zero surviving mutants on the state machine |
| SEC-OPS-007 | Before it is claimed, the server must make no outbound connection except, for an install using the owner's own domain, obtaining its certificate by ACME DNS-01 through the owner's DNS provider, and, from R2 (D-07), when the install chose the project name service (OD-1), registering its random label and obtaining its certificate by ACME DNS-01; it must then print its claim URL (from R2 for a name-service install, `https://<label>.<zone>/claim#<code>`) and a terminal QR code. It must not enable any remote-access transport, port mapping, plugin, metadata fetch, CT monitoring or update check before the claim. | ASVS 13.1.1, 13.2.4; CWE-1188, CWE-668 | R1 | Integration test in an isolated network namespace with an egress recorder: before the claim, only the ACME CA and the DNS provider are contacted for an own-domain install, and nothing at all for tailnet or localhost naming (WP-101, WP-117); headless-browser end-to-end claim from a LAN peer against Pebble and a test DNS server. From R2, the same test for a name-service install, where only the name-service and ACME hosts are contacted (WP-135) |
| SEC-OPS-008 | Restoring a backup onto an unclaimed server must require the same setup code as claiming. | ASVS 6.4.1; CWE-288 | R1 | Integration test: a restore without the code is rejected; a restore with it succeeds, after which the setup routes return 404 |
| SEC-OPS-009 | Owner recovery must be possible only through a command on the host that reaches the server over a local socket open only to the service account, and that produces a single-use enrolment link valid for 15 minutes (SEC-IAM-092). It must work while the server is in the locked state, and recovery must end every owner session, write a critical audit event and alert every admin. | ASVS 6.4.3, 6.4.4, 6.3.7, 7.4.3; NIST SP 800-63B-4 §4.2.3, §4.6; CWE-640, CWE-288 | R1 | Route enumeration test: no network route starts recovery. Integration tests: the socket refuses other UIDs; recovery works from the locked state; afterwards the old owner sessions are rejected, and the audit record and alerts exist |
| SEC-OPS-010 | A native client claiming a server must run a password-authenticated key exchange keyed by the setup code (SPAKE2, RFC 9382, or an equivalent PAKE) and pin the server identity it learns, so that a network attacker can neither learn the code nor sit in the middle of the claim. | RFC 9382; MASVS-NETWORK-1; CWE-300, CWE-319 | R2 | Core-crate unit tests against the RFC 9382 test vectors. Integration test with an active relaying man-in-the-middle: the claim fails and the code stays unused |
| SEC-OPS-011 | Every server key and secret must come from the OS CSPRNG (symmetric keys of at least 256 bits). None may be derived from predictable values such as time, host name, MAC address or install path, and none may ship inside a binary, package or image. | ASVS 11.5.1, 11.1.1; Top 10 A04; CWE-321, CWE-1394, CWE-330, CWE-798 | R1 | Integration test: two fresh installs share no key bytes. CI check scans built artefacts and image layers for key files and high-entropy literals |
| SEC-OPS-012 | Secrets (the root secret, session and URL-signing keys, the PIN and recovery-code peppers, the audit and backup keys) must live as files with mode 0600, in a dedicated directory with mode 0700, owned by the service account, and never in the SQLite database or any rebuildable cache. When the service account owns a directory or file whose mode is looser, the server must tighten it at startup, write an audit event and continue; it must refuse to start, naming a platform-specific fix, only when it cannot (another owner, ACLs, a read-only mount). | ASVS 13.3.1, 13.3.2; CWE-276, CWE-732, CWE-312, CWE-922; CVE-2024-56362 | R1 | Integration matrix over modes and owners asserting the repair-or-refuse outcome and the audit record; integration test that the SQLite files contain none of the canary key bytes |
| SEC-OPS-013 | Secret values must be held in a type with no printable, serialisable or `==`-comparable form (the wrapper of SEC-IAM-095), so that they cannot reach logs, error messages, API responses, diagnostics bundles or alert payloads. | ASVS 16.2.5, 16.5.1; Top 10 A09; CWE-532, CWE-209, CWE-200; CVE-2025-34158 | R1 | Unit tests of the wrapper's `Debug`, `Display` and serialisation. Canary integration test: the whole integration suite runs with known canary secrets, and no log line, HTTP body or header, bundle or alert payload may contain them in raw, hex, base32 or base64 form |
| SEC-OPS-014 | The server must not accept secrets as command-line arguments. It must take them from the environment only through `*_FILE` indirection or systemd credentials, and must start every child process (plugin host, transcode worker) with a cleared environment holding only the secrets that process needs. | OWASP Secrets Management Cheat Sheet §5.1; ASVS 13.3.2; CWE-214, CWE-526 | R1 | Unit test of the argument parser: no secret-bearing flags exist. Integration test: each child's captured environment equals its allowlist |
| SEC-OPS-015 | Each cryptographic purpose must use its own key, derived from the root secret with HKDF and labelled with a key ID. Short-lived signing keys (stream URLs, CSRF) must rotate automatically, and the previous key may be kept only for the longest lifetime of the tokens it signed. | RFC 5869; ASVS 11.1.1, 11.1.2, 11.2.2, 13.1.4, 13.3.4; NIST SP 800-57 Part 1 Rev. 5 | R1 | Unit tests against the RFC 5869 test vectors. Property test with a controlled clock: a token verifies from issue to expiry and never after, under any rotation schedule |
| SEC-OPS-016 | Session tokens, device-enrolment secrets and verify-only app passwords must be stored only as keyed hashes, and device keys only as public keys, so that a copy of the database grants no access. | ASVS 7.2.3, 11.5.1; CWE-312, CWE-522 | R1 | Integration test: issue tokens, then scan the raw database and secrets files for token bytes (none), and try to authenticate with the stored values (fails) |
| SEC-OPS-017 | Secrets the server must replay to other systems (OIDC client secret, SMTP and webhook credentials, plugin tokens, legacy Subsonic app passwords) must be encrypted at rest with an AEAD key derived from the root secret, using the owning record's ID as associated data. | ASVS 11.3.2, 13.3.1; CWE-311, CWE-312; CVE-2024-56362 | R1 | Unit tests: encryption round-trips, and swapping ciphertexts between records fails authentication. Integration test: the raw database contains no plaintext canary |
| SEC-OPS-018 | The owner must be able to rotate every server secret in one action, from the dashboard and the CLI. The action re-encrypts stored secrets, invalidates all sessions and signed URLs, and is audited and alerted, but must not force devices with valid device keys to pair again. | ASVS 13.3.4, 7.4.3; NIST SP 800-57 Part 1 Rev. 5 | R1 | Integration test: after rotation, old sessions and URLs fail, stored secrets still decrypt, a device key opens a new session with no user action, and the audit record and alert exist |
| SEC-OPS-019 | The trust root for the project's update and advisory feed must be compiled into the binary and replaced only through signed root-rotation metadata (TUF semantics). The server must reject any feed whose signatures, signing threshold, expiry or version monotonicity do not verify. | TUF specification 1.0.36; SSDF 1.1 PS.2.1; Top 10 A08; CWE-347, CWE-494, CWE-345 | R1 | Unit tests over fixture metadata: valid chain, rotated root, rollback, freeze (expired timestamp), mix-and-match, wrong key, threshold not met. Fuzz target on the feed parser |
| SEC-OPS-020 | The server must keep a security audit log separate from diagnostic logs and from the rebuildable cache, recording every event in the catalogue in Design guidance under its OWASP Logging Vocabulary name. Every state-changing admin route must declare the audit event it emits, and the action must not take effect if that record cannot be written. Each record carries UTC time, principal, device, source address, action, target and outcome, with every field encoded against log injection and no credential or token. A pre-allocated reserve must let a fixed list of recovery actions (lowering retention, deleting old backups, the host CLI) write their records when the disk is full. | ASVS 16.1.1, 16.3.1, 16.3.2, 16.3.3, 16.3.4, 16.5.3; Top 10 A09; CWE-778, CWE-223 | R1 | Route-table test: a mutating route without a declared event fails the build. Integration test per catalogue entry: the action produces exactly one record with the expected contents, compared as a whole value. Fault-injection test: with the audit writer failing, the action is refused; full-disk fault-injection test: the recovery actions succeed and are recorded, every other admin action is refused |
| SEC-OPS-021 | Each audit record must carry a sequence number, a UTC RFC 3339 timestamp, the event name, the actor (account, profile, device and credential IDs), the source (resolved address and transport), target object IDs, the outcome and a reason code. It must not carry secrets, tokens, setup codes, media titles or file paths. | ASVS 16.2.1, 16.2.2, 16.2.5, 14.2.6; RFC 3339; CWE-532, CWE-359 | R1 | Unit tests on the record type, whose fields are typed IDs with no field that accepts a title, path or secret type. Property test of serialisation |
| SEC-OPS-022 | All log output must be one JSON object per line with every value escaped, so that attacker-controlled text (user names, tags, file names, user agents) cannot create, split or alter records. The logging path must never evaluate lookups or format directives found in data. | ASVS 16.4.1, 16.2.4; Top 10 A09, A05; CWE-117; CVE-2021-44228 | R1 | Property test: any Unicode, including CR, LF, NUL, U+2028 and ANSI escapes, in any field produces exactly one line that parses back to the same values. Fuzz target on the encoder |
| SEC-OPS-023 | The audit log must be tamper-evident. Each record commits to its predecessor by hash (SEC-IAM-094), the server periodically signs checkpoints with a dedicated audit key, and `gunmetal audit verify` reports the first record at which any modification, deletion, reordering or insertion occurred. | ASVS 16.4.2; Crosby and Wallach (USENIX Security 2009); RFC 9162 (consistency proofs, by analogy); CWE-353, CWE-354 | R1 | Property tests: any random set of edits to a valid log makes verification fail with a typed error naming the first bad sequence number. Unit test vectors. Zero surviving mutants on the verifier |
| SEC-OPS-024 | Every backup must include the latest signed audit checkpoint, and both restore and `gunmetal audit verify --against <backup>` must check that the live log extends that checkpoint. | ASVS 16.4.2, 16.4.3 | R1 | Integration test: rewriting history after a backup makes verification against the backup fail; an untouched log passes |
| SEC-OPS-025 | Native clients signed in as an admin must keep the last audit checkpoint they saw. On each sync they must require proof that the server's new head extends it, and raise a critical alert if it does not. | ASVS 16.4.3; MASVS-STORAGE-1; Crosby and Wallach 2009 | R2 | Core-crate unit and property tests of consistency verification. Integration test against a server whose log was rewritten |
| SEC-OPS-026 | Retention must be enforced automatically on the schedule of SEC-PRV-005, and each prune is recorded as a signed checkpoint so that verification still works. | ASVS 14.2.7, 16.1.1; CWE-779; GDPR Art. 5(1)(e) (not legal advice) | R1 | Integration test with a controlled clock: expired records are removed, a prune checkpoint exists and verification passes. Size-cap test |
| SEC-OPS-027 | Only the owner and holders of the audit capability may read the full audit log, and they see other people's source addresses only truncated (/24 for IPv4, /48 for IPv6, or a country); a security-investigation mode that reveals full addresses must itself be audited and alerted to each person affected. Every user must see the records about their own account, devices and credentials with full addresses, including any admin viewing or exporting their history or reading their live sessions (SEC-IAM-077). | ASVS 16.4.2; Top 10 A01; API1:2023, API5:2023; CWE-284, CWE-359 | R1 | Cross-user authorization tests replaying every audit route as every role; response-type test per audience asserting truncation; integration test that investigation mode alerts the subject |
| SEC-OPS-028 | Every failed authentication on any surface (native API, adapters, share links, setup) must also be written to the diagnostic log as one line in a documented, versioned format that includes the resolved client address, so that fail2ban or CrowdSec can act on it. | ASVS 16.3.1, 16.2.4; NIST SP 800-63B-4 §3.2.2; CWE-307, CWE-778 | R1 | Golden-file test of the line format. CI runs the published fail2ban filter through `fail2ban-regex` against sample output |
| SEC-OPS-029 | Diagnostic logging must default to info level and must never record request bodies, query strings, cookies, authorisation headers or media titles. Debug level must switch itself off within 24 hours, and switching it on must be audited. | ASVS 13.4.2, 16.2.5, 14.2.1; OWASP Logging Vocabulary Cheat Sheet; CWE-532, CWE-489, CWE-215 | R1 | Canary integration test with marker values in bodies, query strings and headers. Controlled-clock test of the automatic revert. The audit record is asserted |
| SEC-OPS-030 | Diagnostic bundles must meet SEC-PRV-046 (no database, backups or secrets; file paths, titles, user names and IP addresses replaced with per-bundle pseudonyms; shown in full before download), and must be built from an allowlist of fields, so that a new field stays out until it has been classified. | ASVS 16.2.5, 14.2.6; CWE-532, CWE-200, CWE-359 | R1.2 | Canary integration test over a bundle generated after the full suite. Snapshot test of the preview |
| SEC-OPS-031 | At startup the server must detect any configuration change made outside it (file edits, environment) and write an audit event with the old and new value of every security-relevant setting that changed. It must alert the owner when any such setting became less strict. | ASVS 16.3.3; Top 10 A02; CWE-15 | R1 | Integration tests that edit the configuration between runs. Property test of the per-setting strictness ordering |
| SEC-OPS-032 | The server must raise owner alerts, in the app from R1, at least for: a new credential or device on any admin account; a new device on any account (sent to that user, and to the owner for managed profiles); failed sign-ins above threshold per account or per source; a new user, an increase in role or library access, or a new admin; a plugin installed, updated or granted new permissions; direct internet exposure or an unconfigured proxy; owner recovery; a backup downloaded or restored; key rotation; an audit verification failure; and a running version affected by an advisory or out of support. Alerts must never fire for re-authentication with an existing credential on a known device, and non-critical alerts must be batched into a daily summary. | ASVS 6.3.5, 6.3.7; NIST SP 800-63B-4 §4.6, §4.2.3; Top 10 A09; CWE-778 | R1 | Integration test per trigger: exactly one alert, with the expected recipients and message key. Mutation testing on every threshold |
| SEC-OPS-033 | Every alert about a device or credential must offer a one-step "This wasn't me" action that revokes it, ends its sessions and makes its signed stream URLs fail within one URL lifetime. | ASVS 7.4.2, 7.5.2; CWE-613 | R1 | Integration test: with a stream playing, the action makes the next segment request fail within the URL lifetime |
| SEC-OPS-034 | Alerts for a new admin, a new credential or device on an owner or admin account, owner recovery, internet exposure, a plugin install or permission grant, and an audit integrity failure must not be switchable off and must never be suppressed, only batched with every distinct device listed. Deduplication may fold only identical events (same type and same target credential or device), and rate limits must be per rule and target. An admin must not be alerted about their own interactive actions, but actions taken with API keys or adapter credentials must always alert. | ASVS 2.4.1, 16.3.3; CWE-778 | R1 | Integration test: a flood of failed sign-ins followed by an admin device enrolment still delivers that enrolment by name; property test that distinct targets are never folded; unit tests that critical rules ignore mute settings |
| SEC-OPS-035 | When outbound alert channels (email, webhooks) exist, they must be opt-in and must go through the single egress client (SEC-EXT-001). By default they must send only the event type, the time and a link to the server, with no titles, addresses or secrets, and webhook requests must be signed with a per-endpoint HMAC key and a timestamp. | ASVS 13.1.1, 16.2.3, 14.2.6; CWE-359, CWE-200 | R2 | Integration test against a local SMTP and webhook sink, asserting exact payloads and valid signatures |
| SEC-OPS-036 | Push notifications to native clients must carry no event content. They must only wake the app, which then fetches the alert from the server over its authenticated channel. | ASVS 14.2.6; CWE-359 | R2 | Integration test with a test push provider: every captured payload equals a fixed, content-free body |
| SEC-OPS-037 | Exposure detection, the rate limiters and the audit log must all take the client address and its class from the one path-class function of `network-and-remote-access.md` (SEC-NET-018, 024, 025; SEC-NET-016), so that forwarding headers are believed only from declared proxies and every subsystem agrees about where a request came from. | RFC 6890, 1918, 6598, 4193, 7239; Top 10 A02; API8:2023; CWE-348, CWE-290; CVE-2023-33193 | R1 | Architecture test: no module other than the classifier reads forwarding headers or peer addresses (a grep-based CI check over the server crate). Integration test: a spoofed header from an untrusted peer leaves the audit record, limiter key and exposure state unchanged |
| SEC-OPS-038 | The home posture (SEC-NET-024) must be the default in every official package and container image, and must survive upgrades and restores. Leaving it, by declaring a public proxy or turning on remote access, must be an owner action that requires re-authentication, is audited, and raises an informational alert at the first internet-posture request. | Top 10 A02; API8:2023; SSDF 1.1 PW.9.1; CWE-1188, CWE-1327, CWE-668 | R1 | Packaging integration tests (deb, rpm, container) assert home posture on first start; the migration property test (SEC-OPS-049) covers upgrades; a restore test covers restores; audit records and alerts are asserted |
| SEC-OPS-039 | The server must never ask a router for port mappings (UPnP IGD, NAT-PMP or PCP) for its HTTP listener (SEC-NET-030). | Top 10 A02; CWE-1188, CWE-668 | R1 | Integration test with a fake gateway answering UPnP, NAT-PMP and PCP in a test network namespace: zero mapping requests |
| SEC-OPS-040 | The iroh endpoint must keep router port mapping off unless the owner has turned it on (SEC-NET-030), overriding iroh's default (enabled as of iroh 1.3.0), and the dashboard and `doctor` must show any active mapping. | Top 10 A02; CWE-1188 | R1 | Unit test: the endpoint builder passes `PortmapperConfig::Disabled` while remote access is off (mutation-tested). Integration test with the fake gateway |
| SEC-OPS-041 | Backups must run by default (daily) and contain all irreplaceable state, including the identity store, configuration, durable logs and the audit log. They must be taken from a consistent snapshot (the SQLite online backup API or `VACUUM INTO`), never by copying a live database file. Host-only settings (developer mode, external program paths, listen sockets) must never be in a backup. | SQLite "How To Corrupt An SQLite Database File" §1.2; ASVS 14.1.2; Top 10 A08 | R1 | Integration test: a backup taken during a concurrent write load restores to a database that passes `PRAGMA integrity_check` and holds a consistent prefix of the writes |
| SEC-OPS-042 | Every backup, including automatic ones kept on the host, must be encrypted in age v1 to the server's backup key and to the owner's recovery key (SEC-PRV-039), so that it can be restored on the same server without the recovery kit and on new hardware with it. If the server keeps the recovery key at all, it must hold it only wrapped by the root secret. | ASVS 11.3.2, 14.1.2; CWE-311, CWE-530; age v1 (C2SP) | R1 | Integration tests: a backup contains no plaintext canary; it decrypts with either key alone; a raw scan of the state directory finds no unwrapped recovery key |
| SEC-OPS-043 | Backups must be signed with a server backup-signing key. Restore must verify that signature and the encryption tags, show which server made the backup and when, and parse the archive with limits on total size, entry count and expansion ratio, rejecting absolute paths, `..` components, links and duplicate entries. A backup signed by a key this server has never used must be refused unless the owner types that key's fingerprint, and any SQLite file in an archive is opened only under SEC-STD-031. | ASVS 5.2.3, 5.2.5, 5.3.2; Top 10 A08; CWE-345, CWE-347, CWE-22, CWE-59, CWE-409 | R1 | Fuzz target on the archive reader, run in CI. A property test per rejection rule. Unit tests of every signature failure path; integration test that a foreign-signed backup is refused without the typed fingerprint |
| SEC-OPS-044 | After any restore the server must rotate all symmetric keys and invalidate every session and signed URL. It must then give the owner a "review devices and access" alert listing every restored device, credential, admin and share, which says that removals made after the backup date have been undone. Every restored setting must pass the live API's validators, and any setting less strict than the current defaults is held until the owner confirms it; restored devices and credentials start suspended until their account holder or the owner confirms them. | ASVS 7.4.2, 13.3.4; CWE-613 | R1 | Integration test: back up, revoke a device, restore; the device's old session fails and the alert lists the device; integration tests that a restored trusted-proxy list and a restored library root containing the data directory are held or refused, and that a device revoked after the backup stays suspended |
| SEC-OPS-045 | Downloading a backup must require the owner role and a re-authentication within the previous 5 minutes, and must be audited and alerted. Backups and snapshots must never be written under a web-served path or into a media root. | ASVS 7.5.3; CWE-530, CWE-219, CWE-552 | R1 | Authorization integration tests per role. Unit tests of the path policy. Integration test: static file serving cannot reach the backup directory |
| SEC-OPS-046 | The server must never replace or modify its own executable or install directory, and must run no code it downloaded except plugins installed by an admin action. Packages must install the binary so that the service account cannot write to it. | SSDF 1.1 PS.2.1; Top 10 A08, A03; CWE-494, CWE-829, CWE-250 | R1 | Packaging integration tests (deb, rpm, container): the binary and install directory are not writable by the service account. A `doctor` check test. Code review: no updater module exists |
| SEC-OPS-047 | The update and advisory check must be a required first-run question with two explicit answers and no preselection: "Tell me about security fixes (recommended): fetches a public file, sends nothing about you" and "Not now". While it is off, the admin dashboard shows a quiet persistent status and `doctor` flags it. It must fetch a static signed feed while sending no identifying data (SEC-SUP-051), verify the feed under SEC-OPS-019, and show admins every advisory that affects the running version. | SSDF 1.1 RV.1.3, PS.2.1; TUF 1.0.36; ASVS 15.2.1; CWE-359 | R1 | Component test that setup cannot finish without an answer; integration test that captures the exact outbound request bytes; fixture feeds drive unit tests of the banner (affected, unaffected, OSV range edge cases) and of the off-state status |
| SEC-OPS-048 | Before any schema or format migration the server must take a snapshot and check it with `PRAGMA integrity_check` (or the store's equivalent), aborting without changes if either step fails. Migrations must run in transactions, and a failed migration must leave the server in maintenance mode on the untouched data, never serving from a partly migrated store. | ASVS 16.5.3; Top 10 A10; CWE-636, CWE-455 | R1 | Fault-injection integration tests that fail each migration step and the snapshot step, asserting that the pre-migration bytes are intact and no request is served |
| SEC-OPS-049 | No migration may make an existing installation less strict. A setting added in a release must default to its strictest value on upgraded installs unless the new behaviour is strictly safer, and explicit owner choices must be preserved. | SSDF 1.1 PW.9.1, PW.9.2; Top 10 A02; CWE-1188 | R1 | Property test per migration: for generated pre-migration configurations, every security setting after migration is at least as strict under the documented ordering |
| SEC-OPS-050 | Startup, maintenance and error pages served to unauthenticated clients must not reveal the version, build, file paths, stack traces or database errors. | ASVS 13.4.6, 16.5.1; CWE-209, CWE-1295, CWE-200 | R1 | Integration tests in every server state (unclaimed, migrating, locked, failed) that scan unauthenticated responses for version strings, paths and error text |
| SEC-OPS-051 | An older binary must refuse to open durable state written in a newer, incompatible format, explaining how to restore the pre-upgrade snapshot. It must never discard identity, audit or security configuration in order to start. | ASVS 16.5.3; CWE-636 | R1 | Integration test with fixture state from a future format version |
| SEC-OPS-052 | Every release's notes must say whether it migrates data, whether it can be rolled back, and which security defaults changed. CI must fail a release whose migrations or security defaults changed without that section. | SSDF 1.1 PW.9.2, PS.3.1 | R1 | CI check on release tags that compares the migration and defaults files against the release-notes section |
| SEC-OPS-053 | On Unix-like systems the server must refuse to start when its effective user is root or when it holds any effective or permitted capability, with no override. Official NAS templates and container images must set a non-root user and the data directory's ownership. | Top 10 A02; OWASP Docker Security Cheat Sheet rules 2 and 3; CWE-250, CWE-269 | R1 | Integration tests in containers started as root and with added capabilities: the server exits with a typed error and a documented exit code |
| SEC-OPS-054 | The server must open media roots read-only and never create, modify, rename or delete anything inside them, and `doctor` must warn when the service account can write to a media root. | OWASP Docker Security Cheat Sheet rule 8; CWE-732, CWE-250 | R1 | Integration test: a full scan and playback on a read-only bind mount. Unit tests: the media file-access module offers only read-only opens. A `doctor` test |
| SEC-OPS-055 | The media-serving path must open a file only when its fully resolved location lies beneath a configured media root, so that no symlink or crafted path can make the server send its own state, secrets or backups. | ASVS 5.3.2; Top 10 A01; CWE-22, CWE-59, CWE-61; GHSA-r5qr-m328-qcf4 | R1 | Integration tests with symlinks pointing into the secrets and backup directories. Property tests on the path resolver. Fuzz target |
| SEC-OPS-056 | The official systemd unit must run under a dedicated system account with no login shell, and must score an overall exposure of 2.0 or lower in `systemd-analyze security --offline=yes`. | Top 10 A02; CWE-250; systemd-analyze(1) | R1 | CI check: `systemd-analyze security --offline=yes --threshold=20` on the packaged unit (the threshold scale is ten times the displayed score, checked on systemd 261) |
| SEC-OPS-057 | The official container image must run as a fixed non-root user and work under any other non-root UID. It must work with a read-only root filesystem, all capabilities dropped and no-new-privileges, and must contain no setuid or setgid files, shell or package manager. It must never require privileged mode, host networking or the container engine's socket, and the reference compose file must set these options. | NIST SP 800-190; OWASP Docker Security Cheat Sheet rules 1, 2, 3, 4 and 8; CWE-250 | R1 | CI job that runs the image with those flags through the smoke test. Image inspection for the user, setuid files and shells. Compose file lint |
| SEC-OPS-058 | **Withdrawn 2026-10-02: merged into SEC-IAM-041, SEC-OPS-020.** Route tags (elevated or fresh-uv) and the audit record each mutating route declares. | ASVS 7.5.3, 16.3.3; Top 10 A01 | Withdrawn | Proved by the tests of SEC-IAM-041, SEC-OPS-020 |
| SEC-OPS-059 | Metrics and diagnostics endpoints must be off by default. When on, they must require a dedicated scoped token and expose no per-user data. Release builds must contain no debug or profiling endpoints. | ASVS 13.4.2, 13.4.5, 15.2.3; API9:2023; CWE-489, CWE-215 | R1 | Route-table test on the release build. Integration test of the token's scope |
| SEC-OPS-060 | The server must document every outbound connection it can make (an egress inventory) and must make no connection outside it. | ASVS 13.1.1, 13.2.4, 13.2.5, 16.2.3; API10:2023 | R1 | Integration harness with an egress recorder: runs the full suite and compares every destination with the inventory file |
| SEC-OPS-061 | `gunmetal doctor --security` and the dashboard must report root and capability state, file permissions, writable media or binary, internet exposure, trusted proxies, backup age and encryption, audit verification, version support and clock skew, and the command must exit non-zero on any failure. | SSDF 1.1 PW.9.2; ASVS 16.2.2 | R1 | Integration test per check: each failure mode produces its typed finding and exit code |
| SEC-OPS-062 | Transcode workers and other helper processes must run with strictly fewer privileges than the server. A feature that needs them must report itself unavailable rather than run without the sandbox. | ADR 1 decision 3; ASVS 15.2.5; CWE-250, CWE-636 | R2 | Integration test on a host where sandbox setup fails: transcoding is refused with a typed error (the sibling sandboxing design covers the rest) |
| SEC-OPS-063 | Live TV recordings must be written only to a dedicated recordings root with a quota, separate from the read-only media roots and the state directory, so that filling it cannot corrupt server state. | ASVS 5.2.4; CWE-400, CWE-770 | R3 | Integration test: fill the recordings quota while the server keeps writing audit and state records |
| SEC-OPS-064 | Besides GitHub private vulnerability reporting and `security.txt` (SEC-SUP-007, SEC-SUP-008), the project must accept reports at an email alias that reaches at least two people, and must list that alias in both `SECURITY.md` and `security.txt`. | RFC 9116; SSDF 1.1 RV.1.1, RV.1.3; OpenSSF vulnerability disclosure guide | R1 | CI check that `SECURITY.md` and the site's `security.txt` name the alias; a manual test message during each release rehearsal |
| SEC-OPS-065 | Every report must be tracked against the response times in `SECURITY.md` (SEC-SUP-007: acknowledge within 7 days, triage within 14, fix or mitigation within 90), with shorter fix targets by severity (recommended: 14 days for Critical, 30 for High) and coordinated disclosure no later than 90 days after the report. | SSDF 1.1 RV.1.3, RV.2.1, RV.2.2; ASVS 15.1.1; OpenSSF vulnerability disclosure guide | R1 | Manual review, plus a scripted quarterly report built from GitHub advisory timestamps |
| SEC-OPS-066 | Every security fix must begin with a failing regression test that reproduces the problem, must include a search for the same weakness elsewhere in the code, and must record its root cause. | SSDF 1.1 RV.2.2, RV.3.1, RV.3.2, RV.3.3, RV.3.4 | R1 | Manual review against a checklist in the private-fork pull request template. The regression test lands with the fix and is held to the mutation gate |
| SEC-OPS-067 | Each advisory (published as SEC-SUP-009 requires) must also give affected and fixed ranges in OSV form, a workaround that needs no update where one exists, and the audit-log events or other indicators owners can check for exploitation, and it must be public no later than the fixed release. | SSDF 1.1 RV.2.2; CVSS v4.0; OSV schema 1.9.1; OpenSSF vulnerability disclosure guide | R1 | CI validates the advisory's OSV JSON against the OSV schema. Manual review against the advisory template |
| SEC-OPS-068 | Advisories must also go into the signed feed with their severity, affected ranges, fixed version and an exploited-in-the-wild flag. Neither the feed nor any other project-controlled channel may disable, change or run code on anyone's server. | SSDF 1.1 RV.2.2; Top 10 A08; CWE-912 | R1 | Unit tests: the feed schema has no command or action fields, and the server ignores unknown fields. Fixture-driven banner tests. Code review |
| SEC-OPS-069 | The project must notify the packagers of official and known community packages (distributions, NAS catalogues, container catalogues) before disclosure, under an embargo of no more than 7 days, using a contact list it maintains. | OpenSSF vulnerability disclosure guide; linux-distros list policy | R1 | Manual review against a per-advisory checklist |
| SEC-OPS-070 | The project must publish a supported-version policy (recommended: before 1.0, only the latest release; from 1.0, the latest minor plus the previous minor for 90 days after its successor ships), must ship security fixes without unrelated migrations, and must put end-of-support dates in the feed so that unsupported servers show a banner. | ASVS 15.1.1, 15.2.1; SSDF 1.1 RV.2.2; CWE-1104 | R1 | Unit tests of the banner logic from fixture feeds. Manual policy review at each release |
| SEC-OPS-071 | The signing-key compromise playbook (SEC-SUP-054) must be rehearsed at least once a year, and each rehearsal must perform a real root rotation on a test feed that a released server build then accepts. | SSDF 1.1 PS.2.1, PS.3.1; TUF 1.0.36; NIST SP 800-57 Part 1 Rev. 5 | R1 | Annual drill record. Integration test: the latest release binary follows the rotated root of the test feed (the SEC-OPS-019 fixtures cover the logic) |
| SEC-OPS-072 | The project must publish a compromise runbook for server owners (verify the audit log, rotate keys, revoke devices, review admins and plugins, restore from a backup older than the compromise, update), and CI must run its commands end to end against a test server. | NIST SP 800-61 Rev. 3 | R1 | Documentation test in CI that executes the runbook's commands in order and asserts each result |
| SEC-OPS-073 | When the owner enables a platform keystore, the server must load its root secret through it (systemd `LoadCredentialEncrypted=` with TPM2, Windows DPAPI or macOS Keychain) and must not keep an unsealed copy on disk. | ASVS 13.3.1, 13.3.3 | Later | Integration test on a systemd host with a software TPM (swtpm) |
| SEC-OPS-074 | An external reachability check must run only when the owner asks for it, must send only the address and port to test, and must not store the result outside the owner's server. | ASVS 13.1.1; CWE-359 | Later | Integration test against a local probe service, plus code review of the probe service |
| SEC-OPS-075 | The audit log must have an anchor off the host: each admin's web client must keep the latest signed checkpoint head in personal-mode storage and check at sign-in that the log extends it, alerting on a mismatch, and the owner must be able to export checkpoints into the recovery kit or a second backup destination. | ASVS 16.4.2, 16.4.3; CWE-353 | R1 | Integration test that a re-signed, truncated log is flagged by a client holding an older head; test of the checkpoint export |

**Bound by requirements in [standards-coverage.md](standards-coverage.md).** SEC-STD-024 (key derivation from a backup passphrase), SEC-STD-031 (hostile SQLite files at restore and import), SEC-STD-032 (email alert header injection, Later) and SEC-STD-036 (named security roles and succession).
The 2026-10-02 challenge review merged duplicated controls into one owner each; withdrawn rows above say where their content went, and [threat-model.md](threat-model.md#control-ownership) lists every owner.

Count: 73 live requirements: 64 R1, 0 R1.1, 1 R1.2, 0 R1.3, 5 R2, 1 R3 and 2 Later, plus 2 withdrawn rows kept so their IDs stay stable.

Release assumptions:

- Built-in remote access (iroh) is R2 (owner answer, 2026-10-02).
  SEC-OPS-040 stays R1 only as an absence proof (`deny.toml` bans iroh
  and its port-mapper crates, WP-001) until the owner moves it (register
  D-10); WP-221 verifies the endpoint setting in R2.
- The project name service, its naming client and Certificate
  Transparency monitoring are R2 (D-07). In R1 the server gets HTTPS
  through the owner's own domain with ACME, a tailnet or the same
  machine, so SEC-OPS-007's name-service exception has nothing to apply
  to before R2.
- Diagnostic bundles (SEC-OPS-030) are R1.2, with the diagnostics
  features that produce them.
- Outbound alert channels are R2, because `plugins-and-integrations-security.md`
  allows only the update check and OIDC as outbound traffic in R1.

## Design guidance

### 1. Server states and the first run

**States.** The server is in exactly one of four states, held in a small
state machine in the server crate and covered by the mutation gate:

- **Fresh**: no secrets directory. On first start the server creates it,
  the root secret and the setup code, then moves to Unclaimed.
- **Unclaimed**: the root secret exists, but there is no claim marker and no
  owner. Only the routes in SEC-OPS-003 answer.
- **Claimed**: the normal state.
- **Locked**: a claim marker exists but no usable owner credential does.
  Nothing works except health and the host-side recovery command.

The claim marker (`secrets/claimed`, holding the server ID and the claim
time) and the identity store's owner record are written in the same claim
transaction. The rule that keeps setup closed: **if either says
"claimed", the server is never Unclaimed.** If the identity store has an
owner but the marker is missing, the server repairs the marker and writes
an audit event. If the marker exists but the owner is gone, the server is
Locked. The only way back to Fresh is `gunmetal reset --everything`, run on
the host as the service account, which asks the operator to type the
server's name.

**The setup code.**

- Use 26 symbols of Crockford Base32 (130 bits, meeting SEC-IAM-007's
  128-bit floor) plus one checksum character, shown in groups such as
  `K7QF3-M9XTP-WB4RZ-H2N8C-V6YDJ-Q`. Input is case-insensitive and folds
  ambiguous characters (O to 0; I and L to 1). The checksum lets the setup
  page catch a typo before an attempt is spent, so a slip does not count
  against the attempt limit.
- `network-and-remote-access.md` (SEC-NET-029) sets a lower floor of 60
  bits. A 60-bit code would also be defensible: it is single-use, checked
  only by one server during its unclaimed window, limited to 10 attempts a
  minute, and above the 40-bit floor NIST SP 800-63B-4 §4.1.2.2 sets for
  binding codes used with an identifier. This document follows the stricter
  128 bits because people rarely type the code (see below). Open decision 15
  asks the owner to settle one number.
- Show it on start as a block on standard output, which reaches the journal
  under systemd and `docker logs` in a container. The block holds the code,
  the setup URLs with the code in the fragment
  (`http://localhost:<port>/setup#code=…`, plus the server's HTTPS name if
  it has one), and the same URL as a terminal QR code for a phone. Also
  write the code to `secrets/claim-code` (0600) and print it on demand with
  `gunmetal claim-code` (for example
  `docker exec gunmetal gunmetal claim-code`). Delete the file when the
  server is claimed.
- Put the code in the URL fragment because browsers never send fragments to
  the server. The setup page moves the code into the POST body and removes
  it from the address bar with `history.replaceState`, so it never lands in
  server logs, proxy logs or Referer headers (ASVS 14.2.1).
- Attempts are limited to 10 a minute, server-wide (SEC-IAM-008).
  Restarting the server makes a new code. Each failure prints
  `setup: wrong code from 192.0.2.44 at 2026-10-02T18:01:07Z` on the console,
  which also tells the owner that someone on the network is trying.
  Loopback gets no exemption, because a reverse proxy or tunnel on the same
  host makes every request look like loopback.
- Setup is accepted only in a secure context (HTTPS or loopback;
  SEC-IAM-008), only from local path classes (SEC-NET-029), and only with a
  recognised Host header (SEC-IAM-010). That stops a web page using DNS
  rebinding from driving the setup endpoints from the owner's own browser.
- For a headless box with no HTTPS name yet, the documented path is an SSH
  tunnel (`ssh -L 8443:localhost:8443 nas`) so that the browser is on
  `localhost`. A passkey is bound to its relying-party ID, so a passkey made
  on `localhost` works only there. The setup flow must say so and prompt the
  owner to add a credential on the real origin once it exists (coordinate
  with `identity-and-access.md`).

**After the claim.** Print on the console: "Claimed by 'Sam's laptop
passkey' from 192.0.2.10 at 18:02. If this wasn't you, stop the server and
run `gunmetal owner recover`." The owner sees in one place that the claim was
theirs. From R2, native clients can claim with a PAKE instead
(SEC-OPS-010): the setup code is the PAKE password, so watching traffic
reveals nothing, and the client pins the server's key in the same exchange.

**What gets bound at the claim** is for the authentication design to decide
(open decision 1). Whatever it is, the claim must also do three things:

- Create the backup recovery key and show the recovery kit
  (SEC-PRV-040). Generating it in the browser, in WebAssembly from the core
  crate, means only the public half reaches the server. That is the
  stronger design, if the owner settles open decision 4 that way.
- Ask the update-check question, as a neutral choice (SEC-OPS-047).
- Write the claim to the audit log and the console.

**Recovery.** `gunmetal owner recover`, run on the host, talks to the
server over a local socket open only to the service account (SEC-IAM-092).
It prints a single-use enrolment link valid for 15 minutes, which binds a
new owner credential to the existing server and leaves all data intact. It
works from the locked state, ends every owner session and alerts every
admin. Nothing reaches this path over the network, and there is no email or
SMS reset.

**How other products handle the first run:**

| Product | How the first administrator is established | Verdict |
|---|---|---|
| Jenkins | Writes a generated password to `secrets/initialAdminPassword` and prints it in the log; the "Unlock Jenkins" page requires it | Closest to the Gunmetal model |
| Jupyter Server | Generates a token at start, prints it to the terminal inside a ready-made URL, and requires it by default | Good, but the token sits in a query string, which ends up in browser history and logs; Gunmetal uses the fragment |
| Portainer | No code. If no admin is created within 5 minutes of the first start, the instance stops, to stop someone else taking over a fresh install | Narrows the window but authenticates nobody, and annoys people who are slow |
| Home Assistant | Whoever reaches onboarding creates the owner account; the docs say the owner's credentials cannot be recovered | First visitor wins |
| Immich | The first user to register becomes the admin | First visitor wins |
| Grafana | Ships `admin`/`admin` and prompts for a change at first sign-in | A default password (CWE-1393) |
| Plex | The server is claimed by a plex.tv account; containers take a claim token from plex.tv/claim; the token's lifetime is not documented in the image's README (unverified) | Strong, but needs a vendor account, which ADR 1 rules out |
| Jellyfin | A setup wizard that is open to the first visitor; 12.0 (2026-09-08) stopped unauthenticated re-runs of the wizard on misconfigured servers (PR #17369) | First visitor wins; the re-run fix shows why SEC-OPS-006 exists |

**Keeping it usable.** Most people never type the code. Desktop installers
(when they ship) open the browser on `localhost` with the code in the
fragment. Phone users scan the terminal QR code. NAS users find the code in
the app's log viewer, and the docs show a screenshot for each platform.
Typing it by hand means 26 characters with typo detection, which is the
price of SEC-IAM-007's 128 bits. On a TV, nobody
claims anything: the owner claims from a phone or computer, and the TV
signs in later by QR approval.

### 2. Secrets and keys

**Inventory.** This table is the cryptographic inventory that ASVS 11.1.2
asks for.

| Secret | Purpose | Generated | Stored | Rotation | In backup | After restore |
|---|---|---|---|---|---|---|
| Root secret (32 bytes) | Source for all derived keys | First start, OS RNG | `secrets/root.key`, 0600, or a systemd credential | By the owner (SEC-OPS-018) or after compromise | Yes, encrypted | Kept; derived keys move to a new epoch |
| Stream-URL and CSRF keys | Signing short-lived URLs and tokens | HKDF from root, info `gunmetal/v1/<purpose>/<kid>` | Derived on demand; key IDs in `secrets/keys.json` | Automatic every 24 hours; the previous key is kept only for the longest URL lifetime | No | New epoch |
| Session tokens | Signed-in sessions | 256-bit random | HMAC-SHA-256 of the token (keyed with a derived key) in the identity store | Expire per the authentication design | No | All invalid |
| Server identity key (Ed25519, RFC 8032) | The iroh endpoint key (SEC-NET-*) | First start | `secrets/identity.key` | Only after compromise (clients must re-pin) | Yes, encrypted (open decision 5) | Kept, so clients reconnect without re-pairing |
| TLS private key | HTTPS listener | ACME or the owner's certificate (per `network-and-remote-access.md`) | `secrets/tls/`, 0600 | With certificate renewal | No (re-issued) | Re-issued |
| Server backup key (X25519) | The server's own recipient for backups (SEC-PRV-039) | HKDF from root | Derived | With the root secret; the server then re-encrypts retained backups | Root is | Kept |
| Audit and backup signing keys (Ed25519) | Checkpoints and backup signatures | HKDF from root, separate labels | Derived | With the root secret | Public keys yes | Kept |
| Recovery key (X25519) | The owner's recipient for backups | At setup, preferably in the owner's browser | `secrets/recovery.pub`; the private half is kept only wrapped by the root secret, if at all (open decision 4) | When the owner makes a new recovery kit | Public key yes | Kept |
| Third-party secrets (OIDC, SMTP, webhooks, plugin tokens, legacy app passwords) | Replayed to other systems | Entered by an admin or issued | AEAD (AES-256-GCM or XChaCha20-Poly1305) under a derived "vault" key, with the record ID as associated data | Re-encrypted when the root rotates | Yes, encrypted | Kept |
| Setup (claim) code | Claiming | Each start while unclaimed | A hash in memory, plaintext in `secrets/claim-code` (0600) | Every restart, and after 24 hours | No | Not applicable |
| Project feed trust root | Verifying advisories and updates | Project ceremony | Compiled into the binary | TUF root rotation | Not applicable | Not applicable |

**Layout:**

```
/var/lib/gunmetal/                 0700 gunmetal:gunmetal
  secrets/                         0700  root.key, identity.key, tls/,
                                         keys.json, claimed, recovery.pub,
                                         claim-code
  durable/                         identity store, configuration, history
                                   log, audit log segments
  cache/                           rebuildable SQLite cache
  snapshots/                       pre-migration snapshots
  backups/                         encrypted, signed backups
```

The server checks modes and owners at start, the way OpenSSH's StrictModes
does, and refuses to run with a message such as "secrets/root.key is
readable by group 'users': run `chmod 600 …`". `gunmetal doctor --fix-perms`
applies the fix.

**Secret types.** Use one `Secret<T>` wrapper, for which the `secrecy` and
`zeroize` crates are reasonable building blocks:

- Implement no `Display`, `Serialize` or `PartialEq`. `Debug` prints a fixed
  placeholder.
- Compare in constant time.
- Zeroise on drop.
- Make `expose()` reachable only from a small crypto module. Add a clippy
  `disallowed-methods` entry so that a call anywhere else fails the lint.

The canary test in SEC-OPS-013 is the backstop. It seeds known values for
the root key, a session token, the setup code and an OIDC secret, runs the
integration suite, then searches every captured output in raw, hex, base32
and base64 encodings.

**Crypto agility.** Prefix every token, ciphertext and signature with a
version byte, and record algorithms in `keys.json`, so that an algorithm
can be replaced without breaking stored data (ASVS 11.2.2).

**Environment and child processes.** Accept `GUNMETAL_<NAME>_FILE` and
systemd credentials (`LoadCredential=`). When a plain `GUNMETAL_<NAME>`
secret variable is set, refuse to start with a message that shows the
`_FILE` form and the Docker Compose `secrets:` stanza. Spawn every child
with `env_clear()`.

**Keeping it usable.** Rotation of short-lived keys is invisible. "Rotate
keys and sign everyone out" is one button with a plain description, and
devices holding device keys reconnect without anyone doing anything.

### 3. The audit log

This is the same log `identity-and-access.md` calls the security log
(SEC-IAM-093 to 097). That document defines who sees what. This section
adds the storage, the checkpoints, the anchors outside the server, and the
event names.

**Storage.** Keep append-only segment files of 16 MB in `durable/audit/`,
one canonical JSON object per line, so standard log tools can read them
(ASVS 16.2.4). A single writer task appends. Events marked critical (claim,
recovery, admin grant, plugin install, key rotation, restore) are flushed
with `fsync` before the action returns. If the write fails, the action is
refused (SEC-OPS-020). The log must not live in the SQLite cache, because
ADR 1 treats that cache as rebuildable.

**Record:**

```json
{"seq":18231,"ts":"2026-10-02T18:02:11.204Z","event":"authz_change",
 "actor":{"account":"acc_7Q…","device":"dev_K2…","credential":"cred_P9…"},
 "source":{"addr":"192.0.2.10","class":"lan","via":"direct"},
 "target":{"account":"acc_Z1…","role_from":"member","role_to":"admin"},
 "outcome":"success","reason":"ok",
 "prev":"9f2c…","hash":"41ab…"}
```

The hash is SHA-256 over the domain separator `gunmetal-audit-v1`, the
previous hash, and the canonical record without its `hash` field. Every
1,000 records, or every hour if sooner, write a `gm_audit_checkpoint` record
holding the sequence number, the head hash and an Ed25519 signature by the
audit key.

**What tamper evidence means here.** An attacker who controls the service
account can rewrite the whole log and re-sign it, because the signing key
lives on the same host. A hash chain cannot stop that. It can only be
caught from outside. In Crosby and Wallach's terms the server is an
untrusted logger, and the auditors are:

- **Backups.** Each backup carries a checkpoint (SEC-OPS-024), and backups
  leave the machine.
- **Admins' native apps, from R2** (SEC-OPS-025). Each app keeps the last
  head it saw and asks for the chain segment since then. A plain hash chain
  is enough at media-server log sizes. A Merkle tree (Crosby and Wallach;
  RFC 9162-style consistency proofs) is the upgrade path if proofs get too
  large.
- **The web client is not an auditor.** The server itself serves the web
  client's code, so a compromised server can serve a version that skips the
  check. That is why client anchoring waits for R2's native apps.

**Event catalogue.** Names come from the OWASP Logging Vocabulary where one
fits; Gunmetal-specific events use a `gm_` prefix.

| Event | When | Alert (SEC-OPS-032) |
|---|---|---|
| `sys_startup`, `sys_shutdown`, `sys_crash` | Process lifecycle, with version and state | No |
| `gm_setup_claimed`, `authn_login_fail:setup` | Claim made; wrong setup code | Claim: console only |
| `gm_owner_recovery_started`, `gm_owner_recovery_completed` | Host-side recovery | Critical |
| `authn_login_success`, `authn_login_fail`, `authn_login_fail_max`, `authn_login_lock` | Every sign-in surface, including adapters and share links | Above threshold |
| `authn_token_created`, `authn_token_revoked` | Credentials, device keys, app passwords, API keys | New device or credential |
| `session_logout`, `gm_sessions_revoked_all` | Sign-out, "sign out everywhere", rotation | No |
| `user_created`, `user_updated`, `user_archived`, `user_deleted` | Account lifecycle | New user |
| `authz_change`, `privilege_permissions_changed`, `authz_admin` | Roles, library access, admin grants | Increases; new admin is critical |
| `authz_fail` | Denied authorization, aggregated per actor and route per 10 minutes, with counts | Above threshold |
| `gm_invite_created`, `gm_invite_redeemed`, `gm_invite_revoked`, `gm_share_created`, `gm_share_revoked` | Invitations and share links | Redeemed invite |
| `gm_plugin_installed`, `gm_plugin_updated`, `gm_plugin_removed`, `gm_plugin_grant_changed` | Plugins and their network or file grants | Critical |
| `gm_config_changed`, `gm_config_changed_offline` | Security-relevant settings, with old and new values | When loosened |
| `gm_backup_created`, `gm_backup_downloaded`, `gm_backup_restored`, `gm_snapshot_created` | Backups and snapshots | Download and restore |
| `gm_keys_rotated` | SEC-OPS-018, SEC-OPS-044 | Yes |
| `gm_migration_started`, `gm_migration_completed`, `gm_migration_failed` | Upgrades | On failure |
| `gm_exposure_detected`, `gm_exposure_allowed`, `gm_unconfigured_proxy` | SEC-OPS-037, SEC-OPS-038 | Critical |
| `excess_rate_limit_exceeded` | Any limiter, aggregated per source | Above threshold |
| `sys_monitor_disabled`, `sys_monitor_enabled`, `gm_debug_logging_enabled` | Alert rules muted, debug switched on | No (audited) |
| `sensitive_read` | An admin views another user's history, exports user data, or exports the audit log | Visible to the subject user |
| `gm_audit_verify_failed`, `gm_audit_pruned` | Verification and retention | Failure is critical |

**Volume.** Aggregate the noisy events (`authz_fail`, rate-limit hits) so
that an attacker cannot fill the disk or bury real events (CWE-779,
CWE-400).

**Retention.** Prune whole segments older than the retention period, then
write a signed `gm_audit_pruned` record carrying the last pruned sequence
number and hash, so verification starts again from that anchor. Records
cannot be edited without breaking the chain, so to keep IP addresses for
less time than the records themselves, encrypt the `source.addr` field with
a per-month key and delete each month's key after 90 days
(crypto-shredding); the ciphertext stays and the chain still verifies.
User deletion keeps a tombstone (display name only) until the last record
that refers to that user ages out.

### 4. Diagnostic logs

- Use `tracing` with a JSON formatter. Under systemd or a container, write
  to standard output and let journald or the engine store it; otherwise
  write to rotating files in `LogsDirectory`. Emit no ANSI colour codes
  unless writing to a terminal.
- Log at info by default. `gunmetal log-level debug --for 2h` raises the
  level and then reverts on its own, with a 24-hour maximum. Debug output
  still never contains secrets, because the types prevent it, but it may
  contain IDs and paths, so it is time-limited.
- The fail2ban line is an ordinary JSON log line with a fixed field order,
  starting with `{"ts":…,"event":"authn_login_fail","addr":"203.0.113.7",`.
  The project publishes a filter whose `failregex` anchors on
  `"event":"authn_login_fail","addr":"<HOST>"`. Changing the field order is
  a breaking change that bumps the format version, and the golden test
  enforces this.
- Diagnostic bundles: a fixed allowlist of fields (version, platform,
  configuration with secrets removed, `doctor` output, recent log lines).
  Paths, titles, user names and addresses are replaced with per-bundle
  pseudonyms (SEC-PRV-046), so the same file appears under the same
  pseudonym throughout one bundle and debugging still works. The admin sees
  the whole bundle before downloading it.

### 5. Owner alerts

**Rules and recipients:**

| Alert | Trigger | Who sees it | Can be muted |
|---|---|---|---|
| New device or credential | `authn_token_created` for a device key, passkey, app password or API key | That user; the owner for managed profiles; all admins when the account is an admin | Not on admin accounts |
| Repeated failed sign-ins | 10 or more failures for one account, or 30 or more from one source, within 15 minutes | Owner and admins; the account holder for their own account | Yes |
| Access granted | New user, invite redeemed, role or library access increased | Owner and admins | New admin: no. Others: yes |
| Plugin change | Install, update, or a new network or file grant | Owner and admins | No |
| Internet exposure | `gm_exposure_detected` or `gm_unconfigured_proxy` | Owner | No, until acknowledged |
| Owner recovery | Recovery started or completed | Every admin | No |
| Backup | Downloaded or restored | Owner | No |
| Keys rotated | `gm_keys_rotated` | Owner and admins | Yes |
| Audit integrity | Verification failure | Owner | No |
| Advisory or end of support | Feed says the running version is affected or unsupported | Owner and admins | Banner only until updated |

**Delivery by release.** In R1, alerts are in-app only: the web client's
notification centre on every device the recipient is signed in on
(SEC-IAM-098), with a persistent banner for critical ones. From R2:

- Opt-in email and webhooks, through the egress client (SEC-OPS-035).
  Webhooks use a generic JSON body that ntfy, Gotify and chat bridges can
  consume.
- Content-free native push (SEC-OPS-036). The push only says "you have an
  alert", and the app fetches the alert over its own authenticated
  connection, so the push provider never sees what happened.

Until then, an owner who rarely opens the app learns about alerts late.
`doctor` and the dashboard banner are the backstop.

**Wording.** Use plain language and one decision per alert. For example:
"A new device signed in to Sam's account: 'Living room TV' (Android TV),
from your home network, at 20:14." followed by two buttons, **It was me**
and **It wasn't me**. The second revokes the device and stops its streams
(SEC-OPS-033). Avoid words like "authenticator" or "session token".

**Keeping it usable.** Self-originated actions do not alert the person who
took them. Deduplicate by (rule, subject, source /24 or /48) per hour, and
send a daily digest for the rest. Children and other managed profiles never
receive security alerts; the owner does. On a TV, alerts appear only on an
admin's profile, as a dismissible banner that does not interrupt playback.

### 6. Internet exposure

`network-and-remote-access.md` owns the model: path classes, the home and
internet postures, and the help page (SEC-NET-017 to 031). This section
covers what operations adds on top.

**One classifier.**

- Every subsystem must get the client address from the same pure function
  in the core crate: the posture check, the rate limiters, the audit log,
  the alert engine and `doctor` (SEC-OPS-037). If the limiter and the audit
  log disagree about where a request came from, the evidence is wrong at the
  worst moment.
- A CI check fails if any other module reads `Forwarded`, `X-Forwarded-For`,
  `X-Real-IP`, `True-Client-IP` or the socket peer address. Navidrome's rate
  limiter was bypassed through exactly those headers
  (GHSA-f295-6wp9-qqfg).

**Alerts and audit.**

- A request that hits the home posture from a non-local address writes
  `gm_exposure_detected` and raises the alert in SEC-NET-027, at most once
  per listener per day, with a count.
- Forwarding headers from an undeclared peer (typical of Caddy, nginx,
  Cloudflare Tunnel or Tailscale Funnel on the same host) write
  `gm_unconfigured_proxy` (SEC-NET-017). The alert offers the fix in one
  step: "Declare this proxy", with a choice of private overlay or public.
- Declaring a public proxy or turning on remote access writes
  `gm_exposure_allowed`, needs re-authentication (SEC-OPS-058), and raises an
  informational alert at the first internet-posture request.
- The dashboard shows a status line, for example "Reachable from the
  internet: no", or "yes, through the public proxy at 10.0.0.5".

**Defaults that must not drift.**

- Every package and image starts in the home posture, and neither an
  upgrade nor a restore may leave it (SEC-OPS-038). The migration
  strictness ordering has `home < internet`.
- Docker-published ports bypass host firewalls such as UFW (OWASP Docker
  Security Cheat Sheet rule 5a). The home posture is the backstop for that,
  which is one more reason it must hold in the container image.

**IPv6 on the LAN.**

- SEC-NET-024 counts only loopback, RFC 1918, RFC 6598, RFC 4193 and
  link-local addresses as local. Many home networks give devices global
  IPv6 addresses (GUAs) and prefer them, so a phone on the same Wi-Fi can
  arrive from a global address and be shown the help page.
- One fix is to also count as local any address inside a prefix assigned to
  one of the host's own interfaces ("on-link"). Read those prefixes through
  netlink on Linux, which is one reason the unit keeps `AF_NETLINK`, or
  `getifaddrs` elsewhere.
- An internet attacker cannot complete a TCP handshake from a spoofed
  on-link address, because the replies go to the real LAN host. Network
  location would still only ever remove access (SEC-NET-026).
- This is open decision 15. Until it is settled, `doctor` should detect
  global-IPv6 clients hitting the help page and explain the effect.

**Port mapping.** Never use UPnP, NAT-PMP or PCP for HTTP (SEC-OPS-039).
Build the iroh endpoint with `PortmapperConfig::Disabled` unless the owner
has turned port mapping on (SEC-OPS-040); iroh 1.3.0 enables it by default.

### 7. Backups and restore

**Two kinds of copy:**

| | Snapshot | Backup |
|---|---|---|
| Purpose | Rolling back an upgrade or a failed migration | Recovering from disk loss, theft or corruption; moving to new hardware |
| Made | Before every migration (SEC-OPS-048) | Daily by default, and on demand |
| Contents | The durable and cache stores, exactly as on disk | Identity store, configuration, durable logs, audit log and checkpoint, encrypted vault. Not the cache, sessions, setup code or short-lived keys |
| Protection | Same as live data (0700, on the host) | Encrypted to the server's backup key and the owner's recovery key, and signed |
| Kept | Until 7 days after the next successful start | 14 days by default, configurable (SEC-PRV-041; Immich also keeps 14 daily dumps, per the sibling research) |

**Format.**

- `gunmetal-<server-id>-<UTC timestamp>.gmb`: a signed manifest (format
  version, server ID, creation time, audit checkpoint, SHA-256 of each
  entry) followed by an age v1 payload with two recipients: the server's
  backup key and the owner's recovery key (SEC-PRV-039).
- The server can therefore restore its own backups after database
  corruption without the owner finding the kit, while a backup copied off
  the host is unreadable without either the server's secrets or the kit.
- age encrypts data but does not authenticate the sender: anyone who knows
  the public recipient key can make a valid file. That is why the server
  also signs the manifest (SEC-OPS-043).
- Inside the payload, use a minimal purpose-built container format: a
  length-prefixed list of entries whose names come from a fixed set. Plain
  tar or zip semantics bring links, absolute paths and traversal with them.
- The owner can add a second destination folder, such as a NAS share. Only
  the encrypted file is written there.

**Recovery kit.**

- At setup the owner's client generates the X25519 key pair, sends the
  public key, and shows a page with the secret key as text and as a QR code,
  with Download and Print buttons (SEC-PRV-040).
- The owner confirms by typing the last four characters. If they move on
  without confirming, the dashboard keeps a reminder until they do
  (SEC-PRV-040), and `doctor --security` reports "recovery kit not
  confirmed".
- Making a new kit later starts a new key. The server re-encrypts the
  backups it still keeps to the new key, using its own backup key, and says
  plainly that copies already taken off the host still need the old kit.
- SEC-PRV-040 lets the owner see the key again after re-authenticating,
  which means the server keeps it. If so, it must be wrapped by the root
  secret (SEC-OPS-042). Whether the server keeps it at all is open
  decision 4.

**Restore flow.**

1. Fresh install, then the setup page, then **Restore from backup**.
2. Enter the setup code (SEC-OPS-008).
3. Upload the file and enter or scan the recovery key. On the same server
   after corruption, the server's own backup key is enough, from the
   dashboard or with `gunmetal backup restore`.
4. The server checks the signature, then decrypts and parses in a stream,
   within the limits in SEC-OPS-043.
5. It shows the result, for example: "Backup of 'Basement server', made
   2026-09-30 03:00 UTC, signed by key 7f:3a:…". It warns when the signing
   key is not one this server has used.
6. On confirmation it restores, writes the post-restore events
   (SEC-OPS-044) and rebuilds the cache by scanning.

The recovery key passes through server memory only during the restore and
is zeroised afterwards.

### 8. Upgrades, migrations and rollback

**Delivery.**

- Updates arrive through the package manager or a new container image.
  The server only notifies.
- Images get exact-version tags and major-line tags (for example `1`).
  There are no tags that download a server when the container starts,
  which the sibling research found in Plex's image.
- Self-update is ruled out (SEC-OPS-046). It would require the service
  account to be able to write the binary, which hands any RCE a persistence
  mechanism.

**The feed.**

- `supply-chain-and-release.md` designs the feed (SEC-SUP-049 to 051):
  static TUF metadata on gunmetal.tv, which ADR 1 decision 9 puts on
  Cloudflare. The root, release-targets and advisory roles are signed with
  offline hardware keys (threshold 2 once two keyholders exist). Only
  snapshot and timestamp are signed online. The targets carry releases,
  advisories in OSV form, and end-of-support dates.
- The server polls once a day with random jitter, using a plain `GET` with
  the fixed `User-Agent: gunmetal`. It sends no version, cookie or query
  string, and verifies TLS.
- It stores the highest versions it has seen, to defeat rollback. When it
  has had no fresh, valid feed for 7 days, it shows "Can't confirm you're up
  to date" (SEC-SUP-050) rather than claiming the server is current.
- Advisory matching happens on the server, so the project never learns
  which version anyone runs. Cloudflare sees only an IP address fetching a
  public file.

**Migrations.**

- Numbered and pure where possible: functions in the core crate from version
  N data to version N+1 data, tested against fixtures from every released
  version. The fixture corpus grows with each release.
- The runner:
  1. Take a snapshot and run `integrity_check` on it.
  2. Begin a transaction and migrate.
  3. Check invariants (an owner exists; the marker and identity store
     agree; every security setting is valid).
  4. Commit and write the audit event.
- `gunmetal migrate --check` prints the plan. `gunmetal migrate` runs it and
  exits, the equivalent of Jellyfin 12.0's `--mode MigrateSystem`. A normal
  start migrates automatically after taking the snapshot, and serves the
  version-free maintenance page meanwhile (SEC-OPS-050).
- Every security setting declares a strictness ordering, for example
  `posture: home < internet` and `subsonic_adapter: off < on`. The
  property test in SEC-OPS-049 generates configurations and checks
  `migrate(c) ≤ c` per setting. Jellyfin 12.0 is a good example of the
  intended behaviour: legacy authorization became disabled by default, and
  a migration switched it off on existing installs too.

**Rollback.** An older binary refuses newer durable formats
(SEC-OPS-051) and prints `gunmetal snapshot restore <id>`. The snapshot is
the pre-migration files, so the older binary can use them as they are.

**Release-notes block (SEC-OPS-052):**

```
Upgrade notes
- Migrates data: yes (identity store v4 to v5)
- Roll back: restore the automatic snapshot; downgrading alone will refuse to start
- Security defaults changed: none for existing installs; new installs get adapters off
```

### 9. Least privilege on the host

**Account.** Create a system account named `gunmetal`, with no shell and
its home in the state directory, through `sysusers.d` or the package
scripts. The server checks `geteuid()` and its capability sets (`capget`)
before it opens any listener. Its default port is above 1024. People who
want port 443 put a reverse proxy in front, or set the
`ip_unprivileged_port_start` sysctl.

**systemd unit.** This draft scored **1.1 (OK)** with
`systemd-analyze security --offline=yes` on systemd 261, and it passes
`--threshold=20`:

```ini
[Service]
Type=notify
ExecStart=/usr/bin/gunmetal serve
User=gunmetal
Group=gunmetal
StateDirectory=gunmetal
StateDirectoryMode=0700
CacheDirectory=gunmetal
LogsDirectory=gunmetal
UMask=0077
NoNewPrivileges=yes
CapabilityBoundingSet=
AmbientCapabilities=
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
PrivateDevices=yes
PrivateUsers=yes
ProtectHostname=yes
ProtectClock=yes
ProtectKernelTunables=yes
ProtectKernelModules=yes
ProtectKernelLogs=yes
ProtectControlGroups=yes
ProtectProc=invisible
ProcSubset=pid
RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6 AF_NETLINK
RestrictNamespaces=yes
RestrictRealtime=yes
RestrictSUIDSGID=yes
LockPersonality=yes
MemoryDenyWriteExecute=yes
RemoveIPC=yes
SystemCallArchitectures=native
SystemCallFilter=@system-service
SystemCallFilter=~@privileged @resources
SystemCallErrorNumber=EPERM
DevicePolicy=closed
KeyringMode=private
ReadOnlyPaths=/srv/media
```

The checks it still fails are inherent to a network server: host network
access, inet sockets and no IP allowlist.

Notes for whoever packages it:

- `ProtectHome=yes` hides `/home`, and desktop users often keep music
  there. When `gunmetal library add` finds a path it cannot read, it prints
  the exact drop-in to add (`BindReadOnlyPaths=/home/sam/Music`). That keeps
  the strict default without stranding anyone.
- `PrivateUsers=yes` maps foreign users and groups to `nobody`. Whether
  media readable only through a supplementary group (such as `media`) stays
  readable has not been tested (unverified). The packaging integration test must cover that
  layout, and must fall back to `PrivateUsers=identity` if it fails.
- `MemoryDenyWriteExecute=yes` breaks JIT compilers. If plugins run in a
  JIT-based WebAssembly runtime, run them in a separate helper unit or
  process with its own profile, not inside the server.
- `LoadCredential=root-key:/etc/gunmetal/credentials/root-key` is the
  alternative to a key file in the state directory. `LoadCredentialEncrypted=`
  with TPM2 is SEC-OPS-073 (Later).
- R2 GPU access (`/dev/dri`) belongs only to the transcode worker's unit,
  never to the server's.

**Container.**

- Ship a static binary on a `scratch` or distroless static base with
  `USER 10001:10001` and one volume, `/data`.
- The image has no shell, so `gunmetal claim-code` and `gunmetal doctor`
  are subcommands of the binary itself.
- Unraid-style `PUID`/`PGID` setups run it with `--user 99:100`; any
  non-zero UID works.

Reference compose file:

```yaml
services:
  gunmetal:
    image: <registry>/gunmetal:1
    user: "10001:10001"
    read_only: true
    cap_drop: [ALL]
    security_opt: ["no-new-privileges:true"]
    tmpfs: ["/tmp:size=64m"]
    volumes:
      - gunmetal-data:/data
      - /srv/music:/media/music:ro
    ports: ["<port>:<port>"]
    restart: unless-stopped
volumes:
  gunmetal-data:
```

**Media access.**

- Open media files only through one module that resolves each path beneath
  its root. On Linux, use `openat2` with `RESOLVE_BENEATH`. Elsewhere,
  canonicalise the path and check its prefix, then open with `O_NOFOLLOW`
  on the final component.
- Symlinks inside a root keep working. Symlinks that leave it are refused
  and reported by `doctor` (SEC-OPS-055).
- The same module refuses any path under the state directory, whatever the
  configured roots say.

**Windows and macOS (Later, when those packages ship).** Use a Windows
service virtual account and a launchd daemon with a dedicated `UserName`,
with the same startup checks. The platform specifics have not been checked
(unverified).

### 10. The project's incident response

**People and intake.**

- The owner is the security lead until there is a second maintainer, and
  should name a backup.
- Reports arrive through GitHub private vulnerability reporting (already
  named in `SECURITY.md`), an email alias such as `security@gunmetal.tv`
  (open decision 9), and `security.txt`.
- The report template asks for the affected version, steps, a
  proof-of-concept against a released build, and the impact.
- curl ended its bug bounty on 2026-01-31, saying that a flood of
  AI-generated reports had pushed the share of genuine findings below 5%.
  A reproducible test case requirement is the cheapest defence.

**Severity.** Score with CVSS v4.0. Then record whether the issue affects
the default configuration: an issue that needs the internet posture is less
severe than one that works in the default home posture.

**Workflow.**

1. Acknowledge the report (within 7 days, SEC-SUP-007) and open a GitHub
   security advisory draft.
2. Triage and score it (within 14 days).
3. Fix it in the advisory's temporary private fork, starting with a failing
   regression test (SEC-OPS-066). Search for variants and run the full gate.
4. Request a CVE through GitHub, which is a CNA and usually reviews requests
   within 72 hours.
5. Pre-notify packagers (7 days at most; the linux-distros list allows 14 at
   most and prefers less than 7).
6. Release. Publish the advisory with the release, and the feed entry
   within 24 hours (SEC-SUP-009), then announce on the project site and its
   feed.
7. Within 30 days for Critical and High issues, publish a short post-mortem
   with the root cause and the process change.

**What an advisory says (SEC-OPS-067).** Affected and fixed ranges, a
workaround that needs no update (for example "remove the public proxy in
Settings > Network to return to the home posture"), and what to look for in
the audit log (for example
"`gm_plugin_installed` events you did not make, or `authn_login_success`
from unknown devices after 2027-03-01"). The audit log is what makes that
last part possible. No other media server can tell its owners what to look
for.

**Exploited in the wild.** Ship an emergency release. Set the feed flag so
that a red banner appears within a day on every server that has the check
on. Post a pinned notice. Do nothing on anyone's server.

**Supported versions.** Before 1.0, only the latest release gets fixes.
From 1.0, the latest minor plus the previous minor for 90 days after its
successor ships. `SECURITY.md` states the policy (SEC-SUP-056).
End-of-support dates go in the feed, and an unsupported server shows a
banner (SEC-OPS-070).

**Project keys.** Key custody and the compromise playbook belong to
`supply-chain-and-release.md` (SEC-SUP-049, SEC-SUP-054). The operational
addition is the annual drill (SEC-OPS-071): rotate the root of a test feed
for real, and check that the latest released server follows the rotation.
That is the only way to know servers in the field can survive a key
compromise.

**Owner runbook (SEC-OPS-072).** "If you think your server was
compromised":

1. Return to the home posture: remove public proxies and turn remote
   access off.
2. Run `gunmetal audit verify --against <your newest off-host backup>`.
3. Run `gunmetal keys rotate --sign-out-everyone`.
4. Review devices, admins, plugins and shares.
5. Update.
6. If the binary or host may have been touched, reinstall and restore from
   a backup made before the first suspicious event.

CI runs these commands against a test server, so the runbook cannot rot.

**EU Cyber Resilience Act.** The CRA entered into force on 10 December
2024. Manufacturers must report actively exploited vulnerabilities from 11
September 2026, and the main obligations apply from 11 December 2027. Free
and open-source software developed outside commercial activity gets
special treatment, and "open-source software stewards" have a lighter role.
Whether Gunmetal is in scope depends on whether the project ever has
commercial activity, such as paid builds or hosted relays. This is not
legal advice; see open decision 10.

## Anti-patterns

- **Shipping a default password, even one that must be changed at first
  sign-in.** Mirai compromised hundreds of thousands of devices with 62
  default logins (CISA alert TA16-288A, 2016-10-14). CISA published a
  Secure by Design alert in December 2023 calling on manufacturers to
  eliminate default passwords. Grafana still ships `admin`/`admin` with a
  change prompt. CWE-1392, CWE-1393.
- **Letting the first visitor become the owner.** Home Assistant and Immich
  both work this way. On a network with a compromised device, or for a page
  using DNS rebinding, a fresh server belongs to whoever gets there first.
  CWE-1188.
- **A setup that can reopen.** Jellyfin 12.0 (2026-09-08) had to stop
  unauthenticated re-runs of its startup wizard on misconfigured servers
  (PR #17369). Navidrome 0.64.2 (2026-09-24) fixed a failed admin creation
  that marked setup as done and left the server with no admin. CWE-288,
  CWE-636.
- **Logging secrets "just this once".** Navidrome 0.64.2 also stopped
  writing the admin password to the log when admin creation failed. Error
  paths are where secrets leak. CWE-532.
- **Keeping secrets in the main database.** Navidrome stored its JWT signing
  secret in plain text in `navidrome.db` (CVE-2024-56362, High), so any
  copy of the database could mint sessions. CWE-312.
- **An endpoint that returns the owner's credentials.** In Plex Media Server
  1.41.7.x to 1.42.0.x, `/myplex/account` gave the server owner's
  credentials to other users (CVE-2025-34158, CVSS 3.1 8.5, CWE-669).
  Weeks after the fix, Censys still counted about 314,000 vulnerable
  instances.
- **Trusting forwarding headers or "local network" status.** Emby's
  passwordless local sign-in, combined with spoofable proxy headers, gave
  attackers admin access (CVE-2023-33193). In 2026 Navidrome's login rate
  limit could be bypassed with `X-Forwarded-For`, `X-Real-IP` and
  `True-Client-IP` (GHSA-f295-6wp9-qqfg). CWE-348, CWE-290.
- **Installing plugins without telling anyone.** The 2023 Emby attackers
  installed a plugin that harvested the credentials of everyone who signed
  in. A mandatory "plugin installed" alert would have shown every owner.
- **A vendor kill switch.** In 2023 Emby pushed an update that shut down
  compromised servers. It helped that time, but a switch that can stop
  everyone's server is a target and a liability. Gunmetal informs owners
  and never acts on their servers (SEC-OPS-068). CWE-912.
- **Logging frameworks that interpret what they log.** Log4Shell
  (CVE-2021-44228) ran attacker-supplied lookups found in log messages.
  CWE-117.
- **Copying a live SQLite file as a backup.** SQLite's own documentation
  warns that a copy taken mid-transaction can be corrupt, and recommends
  the backup API or `VACUUM INTO` (How To Corrupt §1.2).
- **Self-updating binaries and containers that download at start.** The
  sibling research found Plex tags that pull a new server on every container
  start. That means the image does not fix the running version, and
  self-update means the service can write its own code. CWE-494.
- **Releasing from something other than the tagged source.** The xz backdoor
  was in the release tarballs, not visible in the repository in the same
  form (CVE-2024-3094). Releases should be built in CI from the signed tag;
  the sibling supply-chain document covers this.
- **Opening router ports automatically.** Jellyfin's docs advise against
  UPnP (per the sibling research), and iroh enables port mapping by default
  as of 1.3.0. Neither HTTP nor remote access should open ports unless the
  owner chose remote access.
- **Running as root in containers "because permissions are hard".**
  OWASP's Docker Security Cheat Sheet makes setting a non-root user rule
  #2. Root plus a media RCE means the host is lost. CWE-250.
- **Debug logging on by default.** The sibling research found Plex has
  debug logging on by default. Debug logs collect identifiers and paths and
  end up in support threads. CWE-489, CWE-532.
- **Migrations that move security defaults around.** During the Jellyfin
  12.0 cycle, legacy authorization was disabled, re-enabled and disabled
  again (PRs #15559, #16754 and #16992), before 12.0 shipped with a
  migration that also disabled it on existing installs. That last step is
  the right pattern. The general lesson is that defaults drift unless a
  test pins them (SEC-OPS-049).
- **Offering a bug bounty without triage capacity.** curl ended its bounty
  on 2026-01-31 because AI-generated reports overwhelmed the maintainers.

## Open decisions for the project owner

> **Status, 2026-10-02.** The owner decisions from every file in
> `docs/security/` are consolidated and de-duplicated in
> [README.md](README.md#open-decisions-for-the-owner). Where the baseline
> has since chosen, or where a recommendation below disagrees with the
> README, the README and the requirement tables win.

1. **The account model, and how the first owner credential is bound in
   R1.**
   - Context: `identity-and-access.md` proposes passkeys reached over
     HTTPS on a name, or on `localhost`. The deciding question, shared with
     `network-and-remote-access.md`, is whether gunmetal.tv runs a naming
     and certificate service like plex.direct.
   - What operations needs either way:
     - A setup path for headless boxes. The SSH tunnel to `localhost` works
       today, but a passkey made on `localhost` works only there.
     - Docs for each NAS and container platform showing where the setup
       code appears.
   - Recommendation: decide the naming service before the server's
     sign-in work starts, because the first-run flow, the docs and support
     load all depend on it.
   - Trade-off: a naming service gives the best first run, but it is
     infrastructure the project must run and secure indefinitely.
2. **Whether the home posture is a hard default.**
   - Recommendation: yes. SEC-OPS-038 keeps it as the default in every
     package and stops upgrades and restores from leaving it, so exposure is
     a decision the owner makes rather than an accident.
   - Trade-off: people who port-forward see a help page first and must
     declare a proxy or turn on remote access, and some will read that as
     "broken".
3. **How the update and advisory check is offered.** The siblings already
   require it to be off until the owner turns it on (SEC-SUP-050,
   SEC-PRV-011).
   - Recommendation: present it at setup as two equal buttons, "Check
     daily for security updates" and "Don't check", with one sentence
     saying the request carries no identifiers and the host sees only an IP
     address. Do not pre-select either.
   - Trade-off: an opt-in check leaves some servers unaware of advisories.
     The Plex numbers in T-OPS-27 show the cost, but a pre-selected box
     would undercut the privacy promise.
4. **Who can read backups, and whether the server keeps the recovery
   key.**
   - Current design: backups are encrypted to the server's backup key and
     the owner's recovery key (SEC-PRV-039). SEC-PRV-040 lets the owner see
     the recovery key again after re-authenticating, which implies the
     server stores it.
   - Recommendation: generate the recovery key in the owner's browser and
     do not keep the private half on the server. Instead of "show it again",
     offer "make a new kit", which re-encrypts retained backups.
   - Trade-off: keeping it on the server is friendlier for owners who lose
     the kit. Not keeping it means that someone who takes the server's
     secrets cannot decrypt later off-host backups once the server's own
     key has rotated.
   - Alternative: an owner-chosen passphrase (age's scrypt recipient) is
     easier to remember, but weak passphrases make stolen backups
     crackable.
5. **Whether backups contain the server identity key.**
   - Recommendation: yes, encrypted, so clients reconnect after a restore
     without pairing again.
   - Trade-off: anyone holding both a stolen backup and its recovery key
     can impersonate the server to clients. Without the key, every device
     pairs again after a restore.
6. **Audit retention and IP retention defaults.**
   - Recommendation: keep audit records 365 days, and crypto-shred source
     addresses after 90 days.
   - Trade-off: longer retention helps investigations; shorter retention
     protects household members' location history and limits what a
     breach exposes.
7. **Response targets and the supported-version policy (SEC-OPS-065,
   SEC-OPS-070).**
   - `supply-chain-and-release.md` sets acknowledge within 7 days, triage
     within 14, and fix within 90. The severity tiers added here (14 days
     for Critical, 30 for High) are recommendations; one or two volunteer
     maintainers may find them hard.
   - Recommendation: publish targets you can keep, and keep "only the
     latest release" until 1.0.
8. **Bug bounty.**
   - Recommendation: none. Offer credit in advisories and a hall of fame.
   - Trade-off: fewer professional researchers, far less triage load.
9. **Reporting channels and the embargo list.**
   - Recommendation: add `security@gunmetal.tv` (an alias forwarding to
     two people) alongside GitHub reporting, and sign `security.txt`
     (RFC 9116 recommends an OpenPGP signature). Use a private packager list
     before considering the linux-distros list.
   - Trade-off: email reaches people without GitHub accounts, but a PGP
     key and an inbox need upkeep.
10. **CRA position.**
    - Recommendation: stay non-commercial for now and record that decision.
      Get legal advice before any paid builds, hosted relays or sponsorship
      tied to features.
    - Trade-off: commercial activity could make the project a manufacturer
      with 24-hour reporting duties since 11 September 2026.
11. **Writing into media folders (NFO files, artwork, lyrics).**
    - Recommendation: never in R1. If it comes later, use a separate
      writable sidecar root, not the media roots.
    - Trade-off: some Plex and Jellyfin users rely on sidecar files next to
      their media.
12. **An override for running as root.**
    - Recommendation: none. Every legitimate case (low ports, NAS
      permissions) has a non-root answer, and an override is what bad
      tutorials copy.
    - Trade-off: a few exotic setups will need documentation.
13. **Who holds the signing keys.**
    - `supply-chain-and-release.md` sets a threshold of 2 once two
      keyholders exist. Until then, one person holds every root key.
    - Recommendation: deposit a second, sealed root key with a trusted
      third party before R1, so that losing the maintainer's token does not
      strand every server on an unrotatable root.
    - Trade-off: more ceremony per root rotation.
14. **Outbound alert channels.**
    - Recommendation: in-app only in R1, as decided across the siblings. In
      R2, email and generic webhooks through the egress client, and
      content-free native push, which needs either a project-run push relay
      (a central service that sees only wake-ups) or UnifiedPush.
    - Trade-off: in R1, owners only see alerts when they open the app.
15. **Conflicts between sibling security documents.**
    - Setup code entropy: SEC-IAM-007 says at least 128 bits; SEC-NET-029
      says at least 60. Recommendation: 128 bits everywhere. People mostly
      follow the link or scan the QR code, so the extra length costs little.
    - LAN clients with global IPv6 addresses: SEC-NET-024 counts only
      private, shared, unique-local and link-local ranges as local, so a phone
      on the same Wi-Fi using a global IPv6 address gets the help page.
      Recommendation: also treat addresses inside the host's own on-link
      prefixes as local. A remote attacker cannot complete a TCP handshake
      from such an address, and location still only removes access.
    - Recovery key re-display: SEC-PRV-040 against this document's
      preference (open decision 4).

## Sources

Standards and guidance:

- OWASP ASVS 5.0.0 source (V6, V7, V11, V13, V14, V15, V16, V2, V5): https://github.com/OWASP/ASVS/tree/v5.0.0_release/5.0/en and https://github.com/OWASP/ASVS/releases/tag/v5.0.0_release
- OWASP Top 10:2025: https://top10.owasp.org/2025 and https://owasp.org/Top10/2025/A09_2025-Security_Logging_and_Alerting_Failures/
- OWASP API Security Top 10 2023: https://api-security.owasp.org/editions/2023/en/0x11-t10
- OWASP MASVS controls: https://mas.owasp.org/MASVS/controls/MASVS-NETWORK-1/, https://mas.owasp.org/MASVS/controls/MASVS-STORAGE-1/, https://mas.owasp.org/MASVS/controls/MASVS-CODE-2/
- OWASP Logging Vocabulary Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Logging_Vocabulary_Cheat_Sheet.html
- OWASP Docker Security Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Docker_Security_Cheat_Sheet.html
- OWASP Secrets Management Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Secrets_Management_Cheat_Sheet.html
- NIST SP 800-63B-4 (final, 2025-07-31): https://csrc.nist.gov/pubs/sp/800/63/b/4/final, sections at https://pages.nist.gov/800-63-4/sp800-63b.html and https://pages.nist.gov/800-63-4/sp800-63b/events
- NIST SP 800-218 SSDF 1.1: https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-218.pdf; SSDF 1.2 draft status: https://csrc.nist.gov/projects/ssdf/news
- NIST SP 800-61 Rev. 3: https://csrc.nist.gov/pubs/sp/800/61/r3/final
- NIST SP 800-57 Part 1 Rev. 5: https://csrc.nist.gov/pubs/sp/800/57/pt1/r5/final
- NIST SP 800-190: https://csrc.nist.gov/pubs/sp/800/190/final
- NIST SP 800-92 Rev. 1 (still a draft): https://csrc.nist.gov/news/2023/draft-sp-800-92r1-available-for-comment
- CWE entries via the MITRE CWE API: https://cwe-api.mitre.org/api/v1/cwe/weakness/{id}
- RFC 9116, 9382, 5869, 6890, 1918, 6598, 4193, 3339, 8032, 7239, 9162, and RFC 9846 (TLS 1.3, which obsoletes RFC 8446): https://www.rfc-editor.org/rfc/
- The Update Framework specification 1.0.36: https://theupdateframework.github.io/specification/latest/
- age v1 format (C2SP): https://github.com/C2SP/C2SP/blob/main/age.md
- OSV schema 1.9.1: https://ossf.github.io/osv-schema/
- CVSS v4.0: https://www.first.org/cvss/v4-0/
- OpenSSF vulnerability disclosure guide for maintainers: https://github.com/ossf/oss-vulnerability-guide/blob/main/maintainer-guide.md
- GitHub repository security advisories: https://docs.github.com/en/code-security/security-advisories/working-with-repository-security-advisories/about-repository-security-advisories
- linux-distros list policy: https://oss-security.openwall.org/wiki/mailing-lists/distros
- EU Cyber Resilience Act: https://digital-strategy.ec.europa.eu/en/policies/cyber-resilience-act
- systemd-analyze(1): https://man7.org/linux/man-pages/man1/systemd-analyze.1.html; systemd.exec(5): https://man7.org/linux/man-pages/man5/systemd.exec.5.html (option details confirmed against the systemd 261 man pages)
- SQLite, "How To Corrupt An SQLite Database File": https://www.sqlite.org/howtocorrupt.html
- Crosby and Wallach, "Efficient Data Structures for Tamper-Evident Logging", USENIX Security 2009: https://www.usenix.org/legacy/event/sec09/tech/full_papers/crosby.pdf
- iroh `endpoint::Builder::portmapper_config` (iroh 1.3.0): https://docs.rs/iroh/latest/iroh/endpoint/struct.Builder.html

First-run practice in other products:

- Jupyter Server security: https://jupyter-server.readthedocs.io/en/latest/operators/security.html
- Jenkins on Linux (unlocking): https://www.jenkins.io/doc/book/installing/linux/
- Portainer setup timeout: https://docs.portainer.io/faqs/installing/your-portainer-instance-has-timed-out-for-security-purposes-error-fix
- Home Assistant onboarding: https://www.home-assistant.io/getting-started/onboarding/
- Immich post-install: https://docs.immich.app/install/post-install/
- Grafana sign-in: https://grafana.com/docs/grafana/latest/setup-grafana/sign-in-to-grafana/
- Plex container (PLEX_CLAIM): https://github.com/plexinc/pms-docker

Incidents and advisories:

- CISA TA16-288A (Mirai): https://www.cisa.gov/news-events/alerts/2016/10/14/heightened-ddos-threat-posed-mirai-and-other-botnets
- CISA Secure by Design alert on default passwords: https://www.cisa.gov/resources-tools/resources/secure-design-alert-how-manufacturers-can-protect-customers-eliminating-default-passwords
- CVE records via the CVE API (CVE-2021-44228, CVE-2024-3094, CVE-2023-33193, CVE-2025-34158, CVE-2024-56362, CVE-2020-5741, CVE-2026-23896, CVE-2026-35031): https://cveawg.mitre.org/api/cve/
- Plex CVE-2025-34158 exposure count: https://www.helpnetsecurity.com/2025/08/27/plex-media-server-cve-2025-34158-attack/
- Emby 2023 compromise and remote shutdown: https://www.bleepingcomputer.com/news/security/emby-shuts-down-user-media-servers-hacked-in-recent-attack/
- Navidrome advisories (GHSA-xwx7-p63r-2rj8, GHSA-f295-6wp9-qqfg, GHSA-p994-r776-mw52, GHSA-r5qr-m328-qcf4): https://github.com/navidrome/navidrome/security/advisories
- Navidrome 0.64.2 release notes: https://github.com/navidrome/navidrome/releases/tag/v0.64.2
- Jellyfin 12.0 release notes: https://github.com/jellyfin/jellyfin/releases/tag/v12.0
- curl ends its bug bounty: https://daniel.haxx.se/blog/2026/01/26/the-end-of-the-curl-bug-bounty/

Project context:

- `README.md`, `CONTRIBUTING.md`, `SECURITY.md`, `docs/adr/0001-architecture.md`, `docs/adr/0002-music-is-first-class.md`
- `docs/research/setup-migration-and-operations.md` (Plex debug logging and container tags, Jellyfin migration failures and UPnP advice, Immich backup retention)
- `docs/research/users-sharing-and-security.md` (incident log, the WebAuthn secure-context constraint)
- Sibling security documents, read while they were being written: `docs/security/identity-and-access.md`, `docs/security/network-and-remote-access.md`, `docs/security/supply-chain-and-release.md`, `docs/security/privacy-and-data-protection.md`, `docs/security/plugins-and-integrations-security.md`
