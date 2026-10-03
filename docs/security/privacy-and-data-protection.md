# Privacy and data protection

Research date: 2026-10-02. Web tools were available and used. The session's
web-search budget ran out part-way through, after which primary sources were
fetched directly (OWASP repositories, NIST CSRC, MITRE's CWE API, vendor
documentation, GitHub advisories, statute text). Anything not confirmed from a
source is marked "(unverified)". Legal points are an engineer's reading, not
legal advice, and are marked "(unverified legal analysis)" where they go
beyond what a source says.

Standards notation used below:

- **ASVS** means OWASP ASVS 5.0.0 (released 30 May 2025, still the latest
  stable release). IDs for V14 and V16 were checked against the 5.0.0 tag;
  the other chapters were read from the master branch, whose IDs matched
  5.0.0 where compared. Levels: Gunmetal should meet Level 2 overall and
  adopt the Level 3 privacy items cited here (14.2.5 to 14.2.8).
- **Top 10 2025** means the OWASP Top 10:2025 (A01 to A10).
- **API Top 10** means the OWASP API Security Top 10 2023.
- **MASVS** means OWASP MASVS v2.1.0 (January 2024), which added the
  MASVS-PRIVACY controls.
- **Privacy Risks** means the OWASP Top 10 Privacy Risks v2.0 (2021).
- **NIST PF** means the NIST Privacy Framework 1.0 Core (version 1.1 was
  still an initial public draft when checked).
- **CRA** means Regulation (EU) 2024/2847, the Cyber Resilience Act.
- **GDPR** means Regulation (EU) 2016/679.

## Summary

Gunmetal's privacy position fits in one sentence: **nothing leaves the
house unless a named person chose to send it, and nobody in the house sees
another adult's listening or viewing unless that person chose to show it.**
Everything below is how to make that sentence true, testable and still
pleasant to use on a TV.

**The product collects nothing for itself.** There is no telemetry, no
analytics SDK, no crash reporter and no phone-home in R1. The opt-in update
check sends no identifiers. A CI job runs a fresh server and the web client
in a network namespace with only loopback and fails if setup, scanning,
browsing, searching or playback attempts a single outbound connection or DNS
lookup (SEC-PRV-007). All outbound traffic passes through one egress
component that holds per-feature host allowlists, refuses redirects off the
list, cannot disable TLS verification, supports a proxy and an offline
switch, and keeps a ledger the admin can read (SEC-PRV-008, SEC-PRV-012).
If opt-in telemetry is ever added it follows the Go toolchain's model:
counters from a public schema, previewed before sending, never library
contents (SEC-PRV-010, Later).

**Listening and watch history is the most sensitive thing the server
holds.** It reveals sexuality, health, religion and politics by inference,
and its timestamps reveal when someone sleeps or is home. The Netflix Prize
showed that "anonymous" viewing data can be re-identified, and Plex's 2023
"Week in Review" emails showed how much trust one opt-out social feature
costs. History is therefore its own data class (Activity), private to its
profile by default, invisible to other household members, never shown to
the admin as a per-user history view, and never sent anywhere except by the
user's own opt-in (SEC-PRV-022, SEC-PRV-025, SEC-PRV-033). A private session
is one or two presses away on every client (SEC-PRV-024). Each user gets a
"what your admin can see" page generated from the same policy table that
enforces it (SEC-PRV-027). We are honest that the person with root on the
machine can read the database; the promise is about what the product
surfaces, logs and sends.

**Metadata lookups are the main way a library leaks.** A provider that
answers lookups learns the library's contents, the server's IP address and,
with a personal key, the owner's account. Providers are off until the admin
picks them on a screen that lists exactly what each one receives
(SEC-PRV-013). Requests are built from a typed evidence value that cannot
carry paths, file names or release-group tags (SEC-PRV-014). Lookups happen
at scan time only, never at play time, so no provider learns what is
playing right now (SEC-PRV-015). Clients load artwork only from their own
server, so viewers' IP addresses never reach a provider (SEC-PRV-016).

**Secrets and backups are encrypted at rest; the rest relies on the
operating system.** Third-party tokens, signing keys and OIDC secrets are
encrypted in the database under a key kept outside it, which closes the hole
behind Navidrome's CVE-2024-56362 (SEC-PRV-037). Every backup, including
local ones, is an age-format archive encrypted to a server key and to a
recovery key the admin saves at setup (SEC-PRV-039). Whole-database
encryption is not recommended for R1 because the key would sit next to the
database on an unattended server; full-disk encryption is documented instead
(open decision 6).

**Logs are boring by construction.** Secrets live in types that cannot be
formatted into a log line. At the default level, logs carry IDs, not titles,
paths, searches or play events. Application logs keep 14 days and security
events 90 days. Diagnostic bundles are pseudonymised and previewed before
download (SEC-PRV-042 to SEC-PRV-046).

**Users can take their data and can make it go away.** Export is
self-service, documented field by field and round-trip tested, with
ListenBrainz-format listens and M3U8 playlists (SEC-PRV-047). Deletion
reaches every copy: SQLite pages and WAL (with `secure_delete`), the history
log (by an atomic segment rewrite, which is the one sanctioned exception to
"append-only"), derived tables, devices (by content-free tombstones) and
backups (by retention, with the date shown to the user) (SEC-PRV-049 to
SEC-PRV-052).

**The law mostly does not bite a household server, but the product acts as
if it did.** GDPR does not apply to purely personal or household activity
(Article 2(2)(c)), and the CJEU reads that exemption narrowly. A server
shared with family and close friends is plausibly exempt; one opened to a
community or to paying users plausibly is not (unverified legal analysis).
Gunmetal gives every owner the tools a controller would need anyway: a
generated privacy notice shown before an invite is redeemed, self-service
export and deletion, retention defaults and encryption (SEC-PRV-053). The
project itself becomes a controller of IP addresses for anything it runs
(the site, the update manifest, any relay), so those services carry no
trackers and keep logs for at most 30 days (SEC-PRV-054).

There are 58 live requirements: 44 R1, 4 R1.1, 1 R1.2, 0 R1.3, 7 R2, 1 R3 and 1 Later, plus 2 withdrawn rows kept so their IDs stay stable.
Requirements for metadata providers, image uploads and diagnostic bundles
follow those features into the point releases R1.1 and R1.2, and share
links are R1.2.
Requirements for features whose release is not yet fixed (scrobbling,
invitations, profiles) are marked R1 because they bind from the
first release that ships the feature, and the music release is where each
of them most plausibly lands.

## Threats

| ID | Threat | Who (the attacker or failure) | Impact | Likelihood | Mitigated by (requirement IDs) |
|---|---|---|---|---|---|
| T-PRV-01 | A household member sees another member's history, queue, private playlists or "continue" rows through a shared TV, a profile switch, a stale browser cache or a guessed API ID | Curious or controlling household member | Intimate tastes exposed; relationship harm | High | SEC-PRV-019, SEC-PRV-022, SEC-PRV-024, SEC-PRV-028 |
| T-PRV-02 | The server owner monitors adult users' detailed history and daily routines (play timestamps show sleep and presence) | Owner or admin, including a controlling partner | Surveillance and coercive control | Medium | SEC-PRV-002, SEC-PRV-024, SEC-PRV-025, SEC-PRV-026, SEC-PRV-027, SEC-PRV-043 |
| T-PRV-03 | An opt-out social feature, digest email or "popular" row broadcasts one user's history to friends or other users (Plex, November 2023) | Product design failure | Exposure to people outside the household | Medium | SEC-PRV-022, SEC-PRV-023, SEC-PRV-030, SEC-PRV-031 |
| T-PRV-04 | A metadata, artwork, lyrics or subtitle provider learns the library's contents tied to the server's IP and the owner's API key; the data is later breached, sold or demanded by legal process, and used to infer piracy or sensitive traits | Third-party provider, its attackers, litigants | A library profile tied to a household | High once any provider is enabled | SEC-PRV-008, SEC-PRV-012, SEC-PRV-013, SEC-PRV-014, SEC-PRV-016, SEC-PRV-017 |
| T-PRV-05 | Lookups made at play time (lyrics, art, subtitles, "now playing") tell a third party what is playing at this moment | Provider, network observer | Live activity tracking | Medium | SEC-PRV-015, SEC-PRV-035, SEC-PRV-060 |
| T-PRV-06 | Client apps fetch fonts, scripts or artwork directly from third parties, revealing each viewer's IP address and browsing | CDNs, font hosts, metadata providers | Per-viewer tracking across servers | High in typical web apps | SEC-PRV-016, SEC-PRV-018 |
| T-PRV-07 | Telemetry or crash reports carry identifiers, file paths, titles or per-file attributes that fingerprint a library; "anonymous" data is re-identified (Netflix Prize) | The project, its host, or attackers of either | Re-identification; legal exposure for users | Medium | SEC-PRV-007, SEC-PRV-009, SEC-PRV-010, SEC-PRV-011, SEC-PRV-046 |
| T-PRV-08 | An upgrade flips a privacy default or enables a new outbound feature for existing users (Plex, August 2017) | The project | Silent loss of privacy | Medium | SEC-PRV-013, SEC-PRV-023, SEC-PRV-033 |
| T-PRV-09 | Another user or an admin links or hijacks a user's scrobbling account so their plays flow to an attacker (Navidrome GHSA-8jrh-w926-8rvw) | Malicious user or admin | Continuous listening surveillance | Medium | SEC-PRV-033, SEC-PRV-034, SEC-PRV-035, SEC-PRV-036 |
| T-PRV-10 | Secrets leak through a database copy, a backup or a support upload (Navidrome CVE-2024-56362) | Anyone holding a copy | Account takeover; abuse of linked accounts | Medium | SEC-PRV-037, SEC-PRV-038, SEC-PRV-039, SEC-PRV-046 |
| T-PRV-11 | Backup archives synced to a cloud drive, copied to a USB disk or left on a sold NAS expose every user's history | Cloud provider, thief, next owner of the disk | Full history disclosure | High (people sync their backup folders) | SEC-PRV-039, SEC-PRV-040, SEC-PRV-041 |
| T-PRV-12 | Logs hold passwords, tokens, signed URLs, titles, paths and IP addresses, are kept forever and get pasted into public bug reports | Admin sharing logs; anyone who can read the log directory | Credential and library exposure | High | SEC-PRV-042, SEC-PRV-043, SEC-PRV-044, SEC-PRV-045, SEC-PRV-046 |
| T-PRV-13 | Capability URLs leak through Referer headers, browser history or proxies; time-ordered or sequential IDs reveal when people joined or media arrived | Third parties, proxies, other users | Token replay; metadata inference | Medium | SEC-PRV-018, SEC-PRV-020, SEC-PRV-021 |
| T-PRV-14 | "Deleted" data survives in SQLite free pages, the WAL, history-log segments, derived tables, device caches or backups | Anyone with later disk access; a restore | Erasure promises are false | High (SQLite keeps deleted content by default) | SEC-PRV-041, SEC-PRV-049, SEC-PRV-050, SEC-PRV-051, SEC-PRV-052 |
| T-PRV-15 | The export feature is abused: a stolen session bulk-exports history, or an IDOR exports someone else's data | Session thief; malicious user | Bulk exfiltration | Medium | SEC-PRV-022, SEC-PRV-047, SEC-PRV-048 |
| T-PRV-16 | A shared or public computer keeps the library and history in browser storage after sign-out | Next person at the browser | History disclosure | Medium | SEC-PRV-019, SEC-PRV-020 |
| T-PRV-17 | Share pages reveal the sharer and other users, get indexed by search engines, or are unfurled by chat apps | Search engines, chat platforms, link recipients | Exposure beyond what the sharer meant | Medium | SEC-PRV-031, SEC-PRV-055 |
| T-PRV-18 | Push notifications, lock screens and OS "recents" features hand titles to Apple, Google and bystanders | Platform push services; people nearby | Content disclosure | Medium | SEC-PRV-056, SEC-PRV-058 |
| T-PRV-19 | A lost or stolen phone or TV holds tokens, the synced library, history and downloads | Thief | Data exposure and account access | Medium | SEC-PRV-019, SEC-PRV-057 |
| T-PRV-20 | Remote-access discovery publishes the home IP address under the server's key; relay operators (possibly the project) see who connects when | The public; relay operators | Locating the home; connection metadata | Medium | SEC-PRV-054, SEC-PRV-059 |
| T-PRV-21 | An owner who shares widely or for money falls outside the household exemption with no way to answer access or erasure requests; project-run services make the project a controller | Owner; the project | Legal exposure | Low to medium | SEC-PRV-005, SEC-PRV-047, SEC-PRV-049, SEC-PRV-051, SEC-PRV-053, SEC-PRV-054 |
| T-PRV-22 | A compatibility adapter exposes other users' activity because the protocol was designed that way (OpenSubsonic `getNowPlaying` returns every user's playback) | Any signed-in user of the adapter | Real-time activity exposure | High once the adapter ships | SEC-PRV-032 |
| T-PRV-23 | An uploaded avatar or playlist cover carries EXIF GPS coordinates that other users can read | Other users, share-link recipients | Home location exposure | Low | SEC-PRV-006 |
| T-PRV-24 | A managed child profile links an external account or publishes a share link | Child; external services | A child's data leaves the household | Low | SEC-PRV-029 |
| T-PRV-25 | The server stores data no feature needs (IP addresses in history, persisted searches, mandatory email), which later leaks or is demanded | Future breach; legal process | Larger blast radius for every other failure | Medium | SEC-PRV-001, SEC-PRV-002, SEC-PRV-003, SEC-PRV-004 |

## Requirements

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-PRV-001 | The server must keep a machine-readable data inventory that assigns every persisted column, API response field, sync-feed field and log field exactly one data class (Public, Library, Activity, Identity, Secret), and the build must fail when any such field has no class. | ASVS 14.1.1, 14.1.2, 16.1.1; NIST PF ID.IM-P; Top 10 2025 A06 | R1 | CI test that reads the live SQLite schema (`pragma table_info`) and the registered response, sync and log types and fails on any unclassified field; mutation testing of the registry lookup |
| SEC-PRV-002 | A history event must contain only the profile ID, the item's content identity, the device ID, UTC timestamps, the playback position and the completion state, and must not contain IP addresses, location, user-agent strings, network identifiers or free text. | ASVS 14.2.6; GDPR Art. 5(1)(c); CRA Annex I Part I (2)(g); NIST PF CT.DP-P4; CWE-359 | R1 | Unit test in `gunmetal-core` comparing the event's serialised form with a literal; the schema check from SEC-PRV-001 |
| SEC-PRV-003 | Client IP addresses must be stored only in the active-session table and the security audit log, must be removed from the session record when the session ends, and must be coarsened and removed from the audit log on the schedule of SEC-PRV-005. | ASVS 14.2.7; GDPR Art. 5(1)(e); NIST PF CT.DM-P5; CWE-359 | R1 | Integration test against a real SQLite file with an injected clock: end a session, advance time, then scan the raw database and WAL bytes for the sentinel address |
| SEC-PRV-004 | The server must not persist users' search queries; any recent-search list must live only on the device and be clearable, and search terms must not appear in logs. | ASVS 14.2.6, 16.2.5; Privacy Risks P10; CWE-532 | R1 | Integration test that searches for a sentinel string and then scans the database files and all log output for it; web client unit test for clearing recent searches |
| SEC-PRV-005 | Retention for every data class must be defined in one schedule in code with the defaults in the retention table in Design guidance, shown in admin settings and in the server privacy notice, and enforced by an idempotent purge job that runs at least daily. The schedule must include: security events 365 days; source addresses in them coarsened to /24 or /48 after 30 days and removed at 90 days; diagnostic logs 14 days or 100 MB. | ASVS 14.2.7, 16.1.1; GDPR Art. 5(1)(e); NIST PF CT.DM-P5; Privacy Risks P6 | R1 | Core property test that purging twice equals purging once and that nothing younger than its limit is removed; clock-injected integration test for each data class |
| SEC-PRV-006 | Images uploaded by users (avatars, playlist covers, custom artwork) must be re-encoded with all EXIF, XMP and IPTC metadata removed before they are stored or shown to anyone. | ASVS 14.2.8; CWE-212 | R1.1 | Unit test with fixture JPEG, PNG and WebP files carrying GPS EXIF and XMP, asserting at byte level that the stored output contains none of those blocks |
| SEC-PRV-007 | With the default configuration, completing setup, scanning a library, browsing, searching and playing media must cause no outbound connection or non-local DNS lookup from the server or the web client beyond the naming purpose of the egress inventory (SEC-TM-075), and none at all when the install chose own-domain, tailnet or localhost naming. | ASVS 13.1.1, 13.2.4, 14.2.3; CRA Annex I Part I (2)(g); NIST PF CT.DP-P1; Privacy Risks P10; CWE-201 | R1 | CI job that runs the server and a headless browser in a Linux network namespace with loopback only, under `strace -f -e trace=connect,sendto`, and fails on any non-loopback connect or DNS query |
| SEC-PRV-008 | All outbound requests from the server must go through one egress component that enforces a per-feature host allowlist, refuses redirects to hosts outside it, requires TLS with certificate verification that cannot be switched off, and records each destination host and purpose (not the URL) in a ledger shown to admins. | ASVS 13.1.1, 13.2.4, 13.2.5, 15.3.2, 12.3.1, 12.3.2; API Top 10 API7, API10; NIST PF CM.AW-P4; CWE-918 | R1 | `cargo-deny` ban on HTTP client crates except as dependencies of the egress crate (CI); unit tests for the allowlist and redirect refusal; integration test against a local TLS server with an untrusted certificate expecting a typed refusal |
| SEC-PRV-009 | The server and every first-party client must not send telemetry, usage analytics, advertising identifiers or automatic crash reports, and must not include any third-party analytics, advertising or crash-reporting SDK. | ASVS 14.2.3; MASVS-PRIVACY-1; Privacy Risks P4, P10; GDPR Art. 25(2); Directive 2002/58/EC Art. 5(3); Top 10 2025 A03 | R1 | CI deny-list check over `Cargo.lock` and the JavaScript lockfile for analytics, advertising and crash SDKs; the zero-egress job from SEC-PRV-007 |
| SEC-PRV-010 | If opt-in telemetry is ever added, each report must contain only counters from a published schema, must not contain library contents, titles, paths, per-file technical attributes, usernames, unbucketed counts, IP-derived location or a long-lived installation identifier, must be shown to the admin in full before the first send, and every send must be recorded in a local ledger. | ASVS 14.2.3, 14.2.6; MASVS-PRIVACY-2, MASVS-PRIVACY-3; NIST PF CT.DP-P2; CWE-359 | Later | Unit tests that the report type serialises only allowlisted keys; property test that random library content never appears in a report; manual review of every schema change through a public proposal |
| SEC-PRV-011 | **Withdrawn 2026-10-02: merged into SEC-OPS-047, SEC-SUP-051.** The first-run question and the exact request form. | ASVS 14.2.3; Top 10 2025 A08; Privacy Risks P10; CWE-201 | Withdrawn | Proved by the tests of SEC-OPS-047, SEC-SUP-051 |
| SEC-PRV-012 | The admin must be able to route all server egress through an HTTP CONNECT or SOCKS5 proxy with remote DNS resolution, and to turn on an offline mode that blocks all egress while sign-in, scanning, browsing and playback keep working. In proxy mode, because the proxy resolves names, the egress client must allow only exact hostnames from a static allowlist and must refuse IP literals and every destination a non-owner can choose (webhooks, plugin-granted hosts, OIDC issuers); `doctor` must probe the proxy with a canary name that resolves to a private address and refuse proxy mode if the proxy forwards it, and the documentation must say the proxy itself must block private ranges. | ASVS 13.2.4, 16.5.2; NIST PF CT.DP-P1; OWASP User Privacy Protection Cheat Sheet | R1 | Integration test with a local SOCKS5 proxy asserting every egress request traverses it and no direct DNS query occurs; end-to-end test of the R1 flows in offline mode; integration test with a test SOCKS proxy that forwards private ranges, asserting proxy mode is refused |
| SEC-PRV-013 | No metadata, artwork, lyrics or subtitle provider may be enabled by default. Setup must include a provider step that must be answered: providers listed with the exact fields each will receive and a link to its privacy policy, the recommended ones highlighted, and "Turn on" and "Not now" for each; enabling one is always an explicit owner action. | MASVS-PRIVACY-3; Privacy Risks P4, P5; NIST PF CM.AW-P1, CT.DP-P4; GDPR Art. 25(2) | R1 | Unit test that the default configuration enables no provider; component test that setup cannot finish without an explicit answer; zero-egress test of the "Not now" path; snapshot test of the disclosure text generated from each provider's declared fields |
| SEC-PRV-014 | Provider requests must be built only from a typed lookup-evidence value holding normalised title, artist, album, year and external IDs, so that file paths, file and folder names, release-group or source tags, file sizes, hashes and user identifiers cannot be sent. | ASVS 14.2.3, 14.2.6; NIST PF CT.DP-P3; Privacy Risks P10; CWE-201 | R1.1 | Core property test that inserts sentinel tokens into every non-title position of generated paths and tags and asserts that no sentinel occurs in any serialised provider request |
| SEC-PRV-015 | Provider lookups must run only during scans, scheduled refreshes, or an explicit user or admin action that names the provider; playback, browsing, searching and showing lyrics or subtitles must not trigger outbound requests. | ASVS 14.2.3; NIST PF CT.DP-P1; CWE-201 | R1.1 | Integration test with a provider enabled and the egress ledger recording: play, seek, browse, search and open lyrics, then assert the ledger has no new entries |
| SEC-PRV-016 | Clients must fetch artwork, lyrics and metadata only from their own Gunmetal server; the server must download provider artwork once, cache it and serve it from its own origin under an opaque ID. | ASVS 3.4.3, 14.2.3; OWASP User Privacy Protection Cheat Sheet; CWE-201 | R1 | API contract test that every image URL in responses and sync payloads is server-relative; CI scan of the built web bundle for absolute third-party URLs |
| SEC-PRV-017 | Outbound provider requests must send a User-Agent naming only the project, its major and minor version and the project's public contact URL, and must never include the owner's email, the server name, the hostname or any user identifier. | CWE-201; MusicBrainz API rate-limiting rules | R1.1 | Unit test against a literal expected string; egress integration test asserting the full header set |
| SEC-PRV-018 | Every web response must send `Referrer-Policy: no-referrer` and the Content-Security-Policy of SEC-API-044 (no `data:` sources), and the web client must not load any script, style, font or image from another origin. | ASVS 3.4.3, 3.4.5, 3.6.1; Top 10 2025 A02; CWE-201 | R1 | Integration test of response headers for every route class; CI check of the built bundle and HTML for third-party origins |
| SEC-PRV-019 | The web client must not keep Activity, Identity or Secret data in localStorage, IndexedDB or Cache Storage unless the user marked the browser as a personal device at sign-in, and sign-out must delete all of the origin's storage and service workers by explicit deletion, and also by `Clear-Site-Data` where the context is secure. | ASVS 14.3.1, 14.3.3; W3C Clear Site Data; CWE-922 | R1 | Browser end-to-end test (Chromium, Firefox, WebKit) inspecting storage after sign-in in both modes and after sign-out, over HTTPS and over plain HTTP on a LAN address |
| SEC-PRV-020 | Responses carrying Activity, Identity or Secret data must send `Cache-Control: no-store`, and artwork and media responses must not be marked `public`. | ASVS 14.2.2, 14.3.2; CWE-524, CWE-525 | R1 | Integration test that walks the route table by data class and asserts the header on each response |
| SEC-PRV-021 | IDs and signed URLs exposed to clients must be opaque random values of at least 128 bits with no embedded timestamp, sequence number, username, title or path. | ASVS 14.2.1, 11.5.1; RFC 9562 §8; CWE-598, CWE-340 | R1 | Unit test of the ID generator (length, randomness source, not a UUIDv1, v6 or v7 layout); property test that URLs built for random items never contain the item's title, path or owner name |
| SEC-PRV-022 | One user's Activity data (history, ratings, private playlists, queue, now-playing, recommendations) must not be returned to any other user, except the guardian of a managed profile, through any route, sync feed or adapter unless the user it belongs to enabled that specific surface. | ASVS 8.2.2, 8.2.3, 15.3.1; API Top 10 API1, API3; Top 10 2025 A01; CWE-359, CWE-639 | R1 | Cross-user property test over the enumerated route table: seed user A's activity with sentinels, call every route as user B with A's IDs and B's own IDs, assert no sentinel in any response; deny-by-default route-policy test |
| SEC-PRV-023 | Every privacy setting must default to its most private value, and no upgrade, migration or new feature may change an existing user's privacy setting or enable an outbound or social feature for them without their own action. | ASVS 14.2.4; MASVS-PRIVACY-4; Privacy Risks P4, P5; GDPR Art. 25(2); CWE-1188 | R1 | Migration tests over fixture databases from every released schema asserting privacy settings are unchanged; unit test of the defaults table; pull-request checklist item for every new setting |
| SEC-PRV-024 | Every client must let a user start a private session from the player in at most two interactions, during which no history events, recommendation signals or scrobbles are recorded and any live-session view shown to someone else omits the title. | MASVS-PRIVACY-4; NIST PF CT.DP-P4; Privacy Risks P10 | R1 | Core unit tests of the event sink in private mode; integration test that the history log is unchanged and the admin session view has no title; UI test counting interactions on web (R1) and with a D-pad on TV (R2) |
| SEC-PRV-025 | The admin interface and admin API must not offer any view, search or export of another adult user's history, ratings or private playlists; admin activity views must be limited to live sessions and aggregate totals. Admin live-session views and session events must show user, device, bitrate and playback method but not the title unless that person opted in to showing titles, and admin reads of live sessions must be rate-limited and recorded in the subject's own log (SEC-IAM-077). | ASVS 8.2.2, 8.2.3, 14.2.6; Privacy Risks P2; CWE-359 | R1 | Authorization-matrix integration test (an admin token gets 403 or 404 on every other-user Activity route); API contract snapshot of the admin session and statistics responses; integration test that the admin live view and event stream carry no title for an adult who has not opted in |
| SEC-PRV-026 | The product must not offer any way for an admin to sign in as or act as another user, and any admin action that changes another user's credentials, role or access must be shown to that user at their next sign-in and in their account activity. | ASVS 8.2.1, 16.3.3; Privacy Risks P2; CWE-285 | R1 | Route-table test asserting no impersonation route exists; integration test of the notice after an admin-initiated credential reset |
| SEC-PRV-027 | Each user must be able to read a "what your admin can see" page generated from the same policy table the server uses to enforce admin access. | MASVS-PRIVACY-3; NIST PF CM.AW-P1; Privacy Risks P5 | R1 | Core unit tests that the page model is a pure function of the policy table, with literal expected statements for each configuration; mutation testing of the derivation |
| SEC-PRV-028 | History, queue, continue-watching state and recommendations must be stored and synced per profile, and a client must not show one profile's Activity data in another profile's session on the same device; PINs follow SEC-IAM-062. | ASVS 8.2.2, 8.3.1; CWE-639, CWE-307 | R2 | Integration test switching profiles within one device session; client test that a profile switch clears in-memory Activity state |
| SEC-PRV-029 | Managed (child) profiles must not be able to link external accounts, enable outbound sharing or create public share links, and their Activity data must be visible only to their designated guardians. | ASVS 8.2.1, 8.2.2; Privacy Risks P4; CWE-359 | R2 | Authorization-matrix integration test with managed-profile tokens |
| SEC-PRV-030 | Emails and in-app notifications must not contain another user's Activity data, and must not contain media titles unless the recipient enabled that notification type. | ASVS 14.2.3; Privacy Risks P4; CWE-359 | R1 | Type-level test that notification templates cannot take another profile's Activity fields; snapshot test of each template under default settings |
| SEC-PRV-031 | Share-link pages and any other page reachable without signing in must not reveal the sharer's username, other users, library size or Activity data, must send `X-Robots-Tag: noindex`, and must omit link-preview metadata unless the sharer enabled previews for that link. | ASVS 14.2.6; API Top 10 API3; GDPR Art. 25(2); CWE-200 | R1 | Anonymous integration test fetching share pages and asserting the full field set, the headers and the absence of Open Graph tags by default |
| SEC-PRV-032 | Compatibility adapters must apply the same Activity rules: the OpenSubsonic `getNowPlaying` response and the Jellyfin adapter's session endpoints must list only the caller's own sessions unless other users opted in to household now-playing. | ASVS 8.2.2, 8.2.3; API Top 10 API3; CWE-359 | R2 | Adapter integration tests with two users playing at once; the cross-user property test from SEC-PRV-022 extended to adapter routes |
| SEC-PRV-033 | Scrobbling and every other feature that sends a user's activity off the server must be off for every user by default, must be turned on only by that user, and must not be enableable by an admin on another user's behalf. | ASVS 8.2.1; MASVS-PRIVACY-4; Privacy Risks P4; GDPR Art. 25(2) | R1 | Default-state unit test; authorization-matrix test that admin tokens cannot call link or enable routes for other profiles |
| SEC-PRV-034 | Linking an external scrobbling account must bind the callback to the initiating user's session with a single-use, unguessable state value of at least 128 bits, and must reject a callback whose state is missing, reused, expired or issued to another session. | ASVS 8.2.2, 11.5.1; RFC 6749 §10.12; API Top 10 API1; CWE-352, CWE-639 | R1 | Integration tests for each rejection case and the happy path; mutation testing of the state check |
| SEC-PRV-035 | Scrobbling must submit only plays recorded after the link time unless the user picks a backfill range, must skip private sessions, excluded libraries or playlists and managed profiles, and "now playing" updates must be a separate setting that is off by default. | NIST PF CT.DP-P4; Privacy Risks P10 | R1 | Core property test over random event streams and settings that the submitted set equals an independently written reference filter |
| SEC-PRV-036 | Unlinking an external service must delete its stored credential and discard queued, unsent submissions in the same transaction. | ASVS 14.2.7; NIST PF CT.DM-P5; Privacy Risks P6 | R1 | Integration test against real SQLite asserting the credential row and the queue are gone, and a raw-byte scan of the database and WAL finds no token sentinel |
| SEC-PRV-037 | **Withdrawn 2026-10-02: merged into SEC-OPS-012, SEC-OPS-017.** Signing and root keys live only in key files (SEC-OPS-012); only replayed third-party secrets sit in the database, under AEAD bound to their record (SEC-OPS-017). | ASVS 11.3.2, 11.3.3, 13.3.1; Top 10 2025 A04; CRA Annex I Part I (2)(e); CWE-312 | Withdrawn | Proved by the tests of SEC-OPS-012, SEC-OPS-017 |
| SEC-PRV-038 | The secret-store key must be generated on first start from a CSPRNG and must be loadable from a file, systemd credentials or container secrets; file permissions follow the repair-or-refuse rule of SEC-OPS-012. | ASVS 11.5.1, 13.3.1, 13.3.2; CWE-276, CWE-732 | R1 | Loader unit tests for each source; the permission matrix of SEC-OPS-012 |
| SEC-PRV-039 | Every backup archive, local or exported, must be encrypted in the age v1 format to the server's backup key and the admin's recovery key, must contain the secret-store key only in wrapped form, and the server must have no code path that writes an unencrypted backup. | ASVS 11.3.3, 14.2.4; Top 10 2025 A04; NIST PF PR.DS-P1; CRA Annex I Part I (2)(e); CWE-311 | R1 | Integration test: backup bytes contain no sentinel; decrypting with either key round-trips to an equal state; a wrong key fails; mutation testing of the backup writer |
| SEC-PRV-040 | Setup must issue one printable Gunmetal recovery kit holding both the owner's account recovery codes and the backup recovery key; the dashboard must keep a reminder until the owner confirms it is saved, and showing it again must require a fresh-uv check. Where the owner's authenticator supports the WebAuthn Level 3 PRF extension (section 10.1.4; support varies by authenticator, unverified), the backup recovery key must also be wrapped under a PRF-derived key, so that a synced passkey alone can restore on new hardware. | ASVS 7.5.3, 11.1.1; W3C WebAuthn Level 3 §10.1.4 | R1 | Component test of the single kit and the reminder state; integration test of the re-authentication gate; restore-on-fresh-host integration test using only the PRF-derived key with a software authenticator |
| SEC-PRV-041 | Backups must be kept for a configurable period (default 14 days) and expired archives must be deleted on schedule. | ASVS 14.2.7; GDPR Art. 5(1)(e); NIST PF CT.DM-P5 | R1 | Clock-injected integration test |
| SEC-PRV-042 | Logs must never contain passwords, passkey or device-key material, session or refresh tokens, signed-URL signatures or query strings, API keys or third-party tokens, and these values must be held in types that cannot be formatted into log output. | ASVS 16.2.5; Top 10 2025 A09; CWE-532; OWASP Logging Cheat Sheet | R1 | Compile-fail tests that formatting a secret type does not compile (or yields only a redaction marker); integration test that drives setup failure, sign-in failure, playback and account linking with sentinel secrets and scans all log output |
| SEC-PRV-043 | At the default log level, logs must not contain media titles, file paths, search terms or play events, and HTTP request logging must record route templates, not concrete paths or query strings. | ASVS 16.2.5, 14.2.6; NIST PF CT.DM-P8; CWE-532, CWE-779 | R1 | Integration test with sentinel titles, paths and searches at the default level; unit test of the route-template formatter |
| SEC-PRV-044 | The repository must contain a log inventory listing every log event type with its fields, data classes, destination and retention, and a test must fail when code can emit an event type that is not in the inventory. | ASVS 16.1.1, 16.2.3; NIST PF CT.DM-P8 | R1 | CI test comparing the typed log-event enum with the inventory file |
| SEC-PRV-045 | Log files must be created readable only by the service account and must be rotated and deleted on the schedule of SEC-PRV-005. | ASVS 16.4.2, 14.2.7; CWE-276, CWE-532 | R1 | Clock-injected integration test of rotation and deletion; file-permission test |
| SEC-PRV-046 | Diagnostic bundles must exclude the database, backups and secrets, must replace file paths, titles, usernames and IP addresses with per-bundle pseudonyms, and must be shown to the admin in full before download. | ASVS 16.2.5, 14.2.3; MASVS-PRIVACY-2; CWE-200, CWE-532 | R1.2 | Integration test generating a bundle from a server seeded with sentinels and scanning it; component test of the preview |
| SEC-PRV-047 | Every user must be able to export all of their own data, without admin involvement, as a documented and versioned archive (native JSON Lines with a field-by-field README, ListenBrainz-format listens and M3U8 playlists) that contains no other user's personal data and no secrets. | GDPR Art. 15, 20; NIST PF CT.DM-P1, CT.DM-P2, CT.DM-P6; Privacy Risks P9; MASVS-PRIVACY-4 | R1 | Round-trip property test (random activity, export, import into a fresh server, compare state); cross-user test that an export holds no other profile's sentinels; schema snapshot test |
| SEC-PRV-048 | Starting an export or an account deletion must require authentication within the last 5 minutes and be rate-limited per user, and the export download must be single-use, bound to the requesting session and expire within 1 hour. | ASVS 7.5.3; API Top 10 API4, API6; CWE-639 | R1 | Integration tests for stale authentication, the rate limit, a second download, a download from another session and expiry |
| SEC-PRV-049 | Users must be able to delete one history entry, a time range or all history; the deletion must remove the data from the database, the history log, derived tables, search indexes and caches within 24 hours, and must be re-applied automatically if a backup taken before it is restored. | GDPR Art. 17; CRA Annex I Part I (2)(m); NIST PF CT.DM-P4; Privacy Risks P6; ASVS 14.2.7; CWE-212 | R1 | Integration test with raw-byte sentinel scans of every store after the erasure job; core property test that derived state after deletion equals derived state rebuilt from the filtered log; restore-after-erasure integration test |
| SEC-PRV-050 | Every database connection must set `PRAGMA secure_delete=ON`, and the erasure job must checkpoint and truncate the WAL and atomically rewrite affected history-log segments, so erased records do not survive in freed pages, the WAL or superseded segment files. | NIST SP 800-88 Rev. 2; NIST PF PR.DS-P3; CWE-226, CWE-212 | R1 | Integration test reading the pragma on every pooled connection; raw-byte sentinel scan of the database, the `-wal` file and the log directory after erasure |
| SEC-PRV-051 | Deleting an account must disable it and end all its sessions and device grants at once, keep it restorable for a grace period (default 7 days), then erase it through the history-deletion pipeline, and the confirmation must state the date by which it will also have left all retained backups. | ASVS 7.4.2; GDPR Art. 17; CRA Annex I Part I (2)(m); Privacy Risks P6 | R1 | Clock-injected integration test across the grace period; session-termination test; core unit test of the date calculation |
| SEC-PRV-052 | Tombstones that carry deletions to devices must identify erased events only by ID or ID range and contain no erased content, and clients must purge tombstoned events on their next sync. | NIST PF CM.AW-P5; CWE-212 | R1 | Core unit test of the tombstone type's fields; web client sync integration test |
| SEC-PRV-053 | Before redeeming an invitation, the invitee must be shown a privacy notice generated from the server's actual configuration (what is stored about them, who can see what, retention, enabled outbound services, how to export and delete), with an optional owner-written addendum. It must fit on one screen as at most five plain sentences generated from the configuration, with "More details" expandable. | GDPR Art. 13 (as a model); MASVS-PRIVACY-3; NIST PF CM.AW-P1; Privacy Risks P5 | R1 | Core snapshot tests of notice generation for configuration permutations; component test that redemption cannot complete before the notice is shown; readability check (sentence count and grade level) in the snapshot test |
| SEC-PRV-054 | Every service the project operates (gunmetal.tv, the update manifest, invite landing pages, any default relay or discovery server) must publish a privacy notice, load no third-party analytics or trackers, and keep request logs for no more than 30 days. | GDPR Art. 5(1)(c), 5(1)(e), 13; ASVS 14.2.3; CWE-201 | R1 | CI scan of the site build for third-party origins; manual review of hosting log settings at each release |
| SEC-PRV-055 | Invitation and share secrets carried through a project-hosted page must travel only in the URL fragment so they never reach the project's servers or logs. | ASVS 14.2.1; CWE-598 | R1 | Browser end-to-end test that records the landing page's network requests and asserts the secret never appears in them |
| SEC-PRV-056 | Push notification payloads sent through Apple or Google services must contain no media titles, user names or server addresses, only an opaque event ID that the app resolves against the server. | MASVS-PRIVACY-1, MASVS-STORAGE-2; CWE-201 | R2 | Unit test of the payload builder; manual review of the native modules against the MASTG |
| SEC-PRV-057 | Native clients must keep tokens and device private keys only in the platform keystore, keep the synced library, history and downloads in app-private storage excluded from OS media indexing and from device or cloud backups, and wipe all server data on sign-out or revocation. | MASVS-STORAGE-1, MASVS-STORAGE-2, MASVS-PRIVACY-4; CWE-922, CWE-312 | R2 | Per-platform native integration tests; manual MASTG-based review before each release |
| SEC-PRV-058 | Plays in a private session or in a profile marked private must not be donated to OS history features (Siri and Spotlight suggestions, Android TV Play Next, Apple TV Top Shelf, recents lists). | MASVS-PRIVACY-1, MASVS-PRIVACY-4; CWE-359 | R2 | Native unit tests with fake OS adapters asserting no donation calls; manual review |
| SEC-PRV-059 | Remote access must publish only relay records, not the server's direct IP addresses, to public discovery services unless the admin opts in, must support a self-hosted relay and discovery server, and setup must state what the chosen relay operator can see. | ASVS 13.1.1; MASVS-PRIVACY-2; NIST PF CT.DP-P1; CWE-200 | R2 | Integration test decoding the published discovery record; review of the setup copy |
| SEC-PRV-060 | M3U playlists and EPG sources must be refreshed only on a schedule through the egress component, never because a user opens the guide or tunes a channel. | ASVS 13.2.4, 14.2.3; CWE-201 | R3 | Integration test with the egress ledger recording while the guide is browsed and channels are tuned |

**Bound by requirements in [standards-coverage.md](standards-coverage.md).** SEC-STD-023 (secrets kept out of core dumps) and SEC-STD-024 (passphrase-derived backup keys).
The 2026-10-02 challenge review merged duplicated controls into one owner each; withdrawn rows above say where their content went, and [threat-model.md](threat-model.md#control-ownership) lists every owner.

## Design guidance

### 1. Data classes and the inventory

Every field the server stores, returns, syncs or logs carries one class. The
class decides who can read it, whether it may leave the server, whether it
may be logged and how long it is kept. Pure logic for classes (the enum, the
retention function, redaction, the notice generator) belongs in
`gunmetal-core`; the registry of which column or field has which class is
checked by a server test against the live schema (SEC-PRV-001).

| Class | Examples | Who can read it in the product | May leave the server | In logs at default level | Default retention |
|---|---|---|---|---|---|
| Public | Server display name, if the owner sets one | Anyone who can reach the server | Only if the owner publishes it | Yes | Until changed |
| Library | Titles, artists, artwork, technical attributes, file paths, provider IDs | Users granted that library; file paths only admins | Only as lookup evidence (normalised title, artist, album, year, IDs) to providers the admin enabled | IDs only | While the file exists, plus the trash grace period |
| Activity | Play events, resume points, ratings, likes, "not interested", playlists marked private, the queue, now-playing, recommendations | The user; guardians for managed profiles; admins see live sessions and aggregates only | Only by the user's own opt-in (scrobbling, sharing) | Never | History: until the user deletes it (user can choose 90 days, 1 year or 2 years). Live-session detail: until the session ends |
| Identity | Usernames, display names, optional email, avatars, device names and platforms, invitations, sessions with IP addresses, security audit events | The user; admins see the account list, devices and audit events | Never | Usernames and IP addresses only in security events | Audit events: 90 days. Accounts: until deletion plus the 7-day grace period. Expired invitations: purged 30 days after expiry |
| Secret | Password hashes, session and refresh tokens, signing keys, third-party tokens, API keys, the secret-store key, the recovery key | Nobody through the product | Only a token to its own service | Never | Per the authentication design; third-party tokens until unlinked |

Other retention defaults (SEC-PRV-005): application logs 14 days; HTTP
access logs off, 7 days if switched on; backups 14 days; diagnostic bundles
deleted after download or after 24 hours; metadata cache refreshed within
each provider's terms (TMDB requires refresh within six months, per the
library research). Every number is configurable; none may be "forever"
except the user's own history, which is the user's to keep.

Minimisation decisions that fall out of the classes:

- **Email is optional.** Sign-in uses passkeys, OIDC and device keys
  (ADR 1), so no feature needs an email address. Ask for one only when the
  admin turns on email notifications, and say why.
- **History events carry no network data** (SEC-PRV-002). The security
  audit log has IP addresses for sign-ins and admin actions; the history
  log never does. Two stores, two purposes, two retention periods.
- **Search stays on the device.** The library is synced to clients
  (ADR 1), so search can run locally; if the R1 web client searches on the
  server, the query is processed and dropped (SEC-PRV-004).
- **No geolocation lookups.** If a dashboard ever shows where a stream
  comes from, it uses a local database and shows "home network" or
  "remote", not a city.

### 2. The egress component

R1 needs very little egress: the opt-in update check and, if the owner
chooses, a music metadata or cover-art provider. Build the component before
the first of these, not after the fifth.

- **One crate owns the HTTP client.** Put the outbound client in a server
  crate (for example `gunmetal-egress`). In `deny.toml`, ban the HTTP client
  crates with `wrappers = ["gunmetal-egress"]`, so any other crate pulling
  one in fails CI (SEC-PRV-008). Plugins (ADR 2) get egress only through the
  host API, never their own sockets.
- **Grants, not URLs.** A grant names a feature, a purpose in plain words,
  the hosts it may reach and the fields it may send. The provider screen
  (SEC-PRV-013) and the server notice (SEC-PRV-053) are rendered from the
  same grant records, so the disclosure cannot drift from what is enforced.
- **TLS is not optional.** Use rustls with certificate verification and no
  configuration switch to turn it off. Redirects are followed only to hosts
  inside the same grant, at most three hops.
- **The ledger.** Record (hour bucket, grant, host, request count, bytes),
  never full URLs or query strings. Show it as "Outbound connections" in the
  admin dashboard: each row says who enabled it and when it last ran.
- **Proxy and offline mode.** Accept `socks5h://` and `http://` proxy URLs
  so DNS goes through the proxy. Offline mode empties the grant set at
  runtime; every feature must degrade to "not available offline", never to
  an error page (SEC-PRV-012).
- **Proving zero egress.** The CI job for SEC-PRV-007 runs the server and a
  headless browser under `unshare --net` with only loopback, wrapped in
  `strace -f -e trace=connect,sendto`. It scripts setup, a scan of a fixture
  library, browsing, searching and playback, then fails if the trace shows a
  connect to any non-loopback address or a DNS query. Because the
  namespace has no route out, a regression shows up as a failed connect in
  the trace rather than as real traffic.

### 3. Metadata lookups

What each kind of provider learns, and what limits it:

| Lookup | The provider learns | Limits |
|---|---|---|
| Music metadata (for example MusicBrainz) | Artist, album and track names or MusicBrainz IDs for the whole library; the server's IP; timing of additions | Scan-time only; IDs where tags have them; project User-Agent; proxy; optional local mirror |
| Cover art (for example the Cover Art Archive) | Release IDs in the library; the server's IP | Server fetches and caches once; clients never contact it |
| Movie and TV metadata (R2, for example TMDB) | Titles and years of the video library; the server's IP; the API key's owner if the key is personal | As above, plus the key decision in open decision 3 |
| Lyrics (online) | Each track whose lyrics are fetched | Fetch at scan time for tracks without sidecar or embedded lyrics, never on display |
| Subtitles (R2, online) | Each film or episode searched | Only on an explicit "search subtitles online" action or a scheduled job, never at play start |
| Acoustic fingerprinting (opt-in plugin) | Audio fingerprints and durations | Off by default (ADR 2); per-library opt-in |

Rules for building it:

- **Evidence is a type.** A `LookupEvidence` value holds a kind, a
  normalised title, optional artist, album and year, and external IDs. Its
  text fields can only be built by the tag and filename parsers, which strip
  release-group, source, resolution and codec tokens. There is no field for
  a path, file name, size or hash, so none can be sent (SEC-PRV-014). Scene
  release names in file names say where a file came from; a provider never
  needs them.
- **When lookups run.** New items are looked up by a background job shortly
  after a scan (so new albums get their art quickly), refreshes run in the
  maintenance window, and nothing runs because someone pressed play
  (SEC-PRV-015). An admin who also wants to hide acquisition times can set
  "look up new items only in the maintenance window".
- **Cache and proxy artwork.** The server downloads each image once, decodes
  it with size and pixel limits in memory-safe code (the image-handling
  security document owns those limits), stores resized derivatives under a
  content hash and serves them at `/art/{opaque-id}` (SEC-PRV-016).
- **User-Agent.** MusicBrainz asks for an application name, version and a
  contact URL or email. Send `Gunmetal/<major>.<minor> ( https://gunmetal.tv )`
  and nothing about the owner (SEC-PRV-017).
- **Batching and decoys.** Batching requests does not stop the provider
  learning the library; it only blurs timing. Decoy lookups would waste a
  volunteer-run service's capacity and probably breach its terms. Neither is
  worth building; minimum fields, scan-time scheduling, caching, a proxy and
  local data are the real levers.
- **Local data.** MusicBrainz publishes its core database as CC0 dumps with
  a replication feed, so a "local mirror" provider can answer every lookup
  with zero disclosure (Later; it is a large download). TMDB's daily exports
  contain only IDs and a few attributes, so they cannot replace lookups.

### 4. Activity: history, profiles and what the admin sees

- **History log layout.** Keep one append-only log per profile, in monthly
  segments. Each event has a random 128-bit ID, a hybrid logical clock
  timestamp for merging offline devices, the device ID and the fields in
  SEC-PRV-002. Per-profile segments make export a file copy and make
  erasure of a profile a directory removal. Record this in a new ADR that
  extends ADR 1: the log is append-only in normal operation, and erasure is
  the one sanctioned rewrite (see section 10).
- **Private session.** The event sink in the core takes a mode; in private
  mode it drops play, recommendation and scrobble events and emits only a
  transient bandwidth record. On web and phone, the toggle sits in the
  now-playing overflow menu; on TV, it is the first item of the player's
  options menu, two D-pad presses from playback. A persistent icon shows
  while it is on. It ends when the user turns it off or after a period
  without playback that the user picks (default 6 hours), so nobody stays
  in private mode by accident for weeks.
- **Admin views.** Live sessions show user, device, bitrate and playback
  method, plus the title unless the session is private; this is enough to
  stop a stream or find a bandwidth hog. Statistics are aggregates. A
  household "popular on this server" row includes only users who opted in,
  and only items played by at least three of them, so it cannot single out
  one person in a small house. There is no per-user history page for
  admins, no impersonation, and no admin export of someone else's data
  (SEC-PRV-025, SEC-PRV-026). A user who loses all their devices gets back
  in through a fresh invitation; their history is still theirs.
- **"What your admin can see."** Render it from the policy table, in plain
  words: "Your admin can see that you are streaming, on which device and at
  what quality. Your admin can see the title unless you use a private
  session. Your admin cannot see your history, ratings or private playlists
  in Gunmetal. Your admin controls the computer Gunmetal runs on, so a
  determined admin could read its files directly." The last sentence is the
  honesty the project owes users.
- **Profiles on shared devices.** The TV profile picker shows names and
  avatars only. Each profile's synced data lives in its own store; switching
  profile drops the previous profile's in-memory state. A PIN is checked by
  the server with a lockout after a few failures (SEC-PRV-028), because a
  four-digit PIN checked on the device can be brute-forced or read from the
  cache. On the web client, a profile switch reloads the app state.
- **Managed profiles.** Guardians can see a managed profile's history, and
  the managed profile's interface says so in age-appropriate words. Managed
  profiles cannot link external accounts or create share links
  (SEC-PRV-029).
- **Adapters (R2).** The OpenSubsonic `getNowPlaying` endpoint is specified
  to return what every user is playing, with usernames. The adapter must
  filter it to the caller (SEC-PRV-032); the same applies to the adapter's
  `scrobble` endpoint, which must write only into the caller's own history
  and must not forward to Last.fm unless that user linked it.

### 5. Scrobbling and other outbound sharing

ADR 2 already puts scrobbling in plugins with explicit network grants. The
privacy rules on top:

- **The user links, not the admin.** The admin installs a scrobbler plugin,
  which makes "Connect Last.fm" or "Connect ListenBrainz" appear in each
  user's settings. Nothing is sent until that user connects it
  (SEC-PRV-033).
- **Bind the link flow.** Generate a 128-bit state value tied to the
  session, store its hash with a 10-minute expiry, and accept the callback
  only from the same session with a matching, unused state (SEC-PRV-034).
  Navidrome's link callback trusted a user ID from the query string, which
  let any user "silently redirect another user's scrobbling" (Navidrome
  advisory GHSA-8jrh-w926-8rvw).
- **Filter in the core.** The scrobble filter is a pure function: events
  plus link time plus exclusions to events to submit. It is property-tested
  against an independently written reference (SEC-PRV-035). "Now playing"
  is separate and off, because it is real-time activity.
- **Tokens are secrets.** Last.fm session keys have an infinite lifetime by
  default (Last.fm authentication guide), so they are stored encrypted
  (SEC-PRV-037) and deleted on unlink (SEC-PRV-036). Settings say plainly
  that scrobbles already sent stay with the service and link to its own
  deletion page.
- **Consent records.** Store each opt-in as (profile, feature, settings
  version, time). Migrations copy these unchanged; a new outbound feature
  starts off for everyone, existing users included (SEC-PRV-023).

### 6. Secrets at rest

- **Key hierarchy.** A 256-bit secret-store key lives outside the database:
  by default in `<data_dir>/secret.key` with mode 0600, created on first
  start, or loaded from systemd credentials (`LoadCredential=`) or a
  container secret (SEC-PRV-038). The server refuses to start if the file is
  readable by group or others.
- **Field encryption.** Encrypt each Secret-class value with an AEAD
  (XChaCha20-Poly1305 or AES-256-GCM from an audited library), with a random
  nonce per value and associated data of `table || row-id || purpose ||
  key-id` (SEC-PRV-037). The key ID enables rotation: new writes use the new
  key, a background job re-encrypts, and the old key is destroyed once no
  row references it.
- **In memory.** Keep secrets in a `Secret<T>` wrapper whose `Debug`
  prints a fixed marker and which has no `Display`, and zeroise on drop.
- **What is not encrypted.** Password hashes (Argon2id, owned by the
  authentication design) and passkey public keys are not reversible
  secrets; they stay Secret-class for logging and export purposes but need
  no field encryption. Library and Activity data rely on the operating
  system: the admin guide recommends full-disk encryption (LUKS, BitLocker,
  FileVault) and says why.

### 7. Backups

- **What is in a backup.** Configuration, the identity tables, the history
  and curation logs, and the secret-store key wrapped to the backup
  recipients. Not media, and not the rebuildable cache (ADR 1).
- **Format.** A tar stream, compressed, then encrypted with age v1 to two
  X25519 recipients: the server's own backup key (held in the secret store,
  so local restores need no input) and the admin's recovery key (so a copy
  on a cloud drive is useless without the recovery kit) (SEC-PRV-039). age
  authenticates its payload in 64 KiB chunks with ChaCha20-Poly1305 and MACs
  the header, so tampering fails loudly. The age specification now defines
  ML-KEM-768 hybrid recipients; switch to them once the Rust implementation
  supports them, which keeps the crypto inventory's post-quantum plan simple
  (ASVS 11.1.4).
- **Recovery kit.** Setup shows the recovery key as text and a QR code with
  "download recovery kit" and "I've saved it" buttons. Skipping is allowed,
  because local backups still work, but a dashboard banner stays until the
  admin confirms (SEC-PRV-040). The kit says, in one paragraph, that losing
  it means off-site backups cannot be restored.
- **Retention and erasure.** Backups are kept 14 days by default
  (SEC-PRV-041), so an erased profile has left every backup at most 14 days
  after erasure, and the deletion confirmation shows that date
  (SEC-PRV-051). An erasure ledger (profile IDs and event-ID ranges, never
  content) lives outside the backups and is re-applied after any restore
  (SEC-PRV-049).
- **File names.** Backup files are named by timestamp only, never by server
  name or user.

### 8. Logs and diagnostics

- **Types do the redaction.** Secret-class values are unprintable
  (SEC-PRV-042). Titles and paths are wrapped in a `Sensitive<T>` whose
  `Display` prints the item or file ID. Raw values are available only in a
  time-limited "verbose diagnostics" mode that the admin turns on, which
  shows a banner and turns itself off after 24 hours.
- **Requests.** Log the matched route template
  (`GET /api/items/{id}/stream`), status, duration and user ID, never the
  concrete path or query string, which can hold signatures (SEC-PRV-043).
- **The inventory.** Log events are variants of one typed enum; a test
  compares the variants with an inventory file in the repository that lists
  each event's fields, classes, destination and retention (SEC-PRV-044).
- **Security events.** Sign-in success and failure, admin actions and
  egress-grant changes go to the security log with user ID and IP address,
  in a fail2ban-friendly line format, kept 90 days (SEC-PRV-045).
- **Retention in containers.** When logs go to stdout, retention belongs
  to the container runtime; the docs say so and show how to cap it.
- **Diagnostic bundles.** The bundle holds the configuration with secrets
  removed, recent logs, `gunmetal doctor` output and version information.
  Paths, titles, usernames and IP addresses become per-bundle pseudonyms
  (`file-0007`, `user-02`, `ip-03`), stable within the bundle so a
  maintainer can follow a thread. The admin sees the whole bundle before
  downloading it (SEC-PRV-046).

### 9. Export

- **Archive layout.** `manifest.json` (schema version, generation time,
  server version), `README.txt` explaining every file and field,
  `profile.json`, `history.jsonl` (native events), `listens.json` in
  ListenBrainz's import format for music plays, `playlists/*.m3u8` plus
  `playlists.json` with MusicBrainz and other provider IDs, `ratings.jsonl`,
  `devices.json`, `sessions.json` (the user's own sessions for the last 90
  days, including IP addresses, because they are the user's data),
  `linked-services.json` (names and dates, never tokens), `consents.json`
  and `shares.json` (SEC-PRV-047). Items are described by title, artist,
  album, duration and provider IDs, never by server file paths, so the
  export is portable and reveals nothing about the server's layout.
- **Why a README.** Sweden's data protection authority fined Spotify SEK 58
  million in June 2023 partly because its access responses did not explain
  the data, including technical log files. An export nobody can read is not
  much of an export.
- **The flow.** "Download my data" sits in account settings on every
  client; on TV it shows a QR code that opens the page on a phone. It asks
  for re-authentication, builds the archive in the background, and offers a
  single-use link bound to the session that expires in an hour
  (SEC-PRV-048). Admins are not notified; the event goes to the security
  log without contents.
- **Round-trip test.** Generate random activity, export, import into a
  fresh server, export again and compare. This doubles as the migration
  path between Gunmetal servers.

### 10. Deletion and erasure

The erasure pipeline is a job with these steps, each idempotent:

1. **Database rows.** Delete the rows. Every connection sets
   `PRAGMA secure_delete=ON` (it is per connection and off by default), so
   SQLite overwrites deleted content with zeros. Do not use the `FAST`
   setting, which leaves traces on freelist pages. Then run
   `PRAGMA wal_checkpoint(TRUNCATE)` so the WAL no longer holds frames with
   the old rows (SEC-PRV-050). If full-text indexes are used, check how the
   chosen SQLite version handles deleted tokens (FTS5 has a `secure-delete`
   option, unverified) and rebuild the index if needed.
2. **History-log segments.** Write a new version of each affected monthly
   segment without the erased events, `fsync` it, rename it over the old
   one and `fsync` the directory. For a whole profile, remove its directory.
3. **Derived data.** Recommendation neighbours, statistics and caches are
   recomputed from the filtered log. The core property test asserts that
   incremental deletion and full rebuild give equal state (SEC-PRV-049).
4. **Devices.** Append a tombstone (event IDs or ID ranges only) to the
   profile's sync feed; clients purge on next sync (SEC-PRV-052). A revoked
   or lost device cannot be reached; the deletion screen says so.
5. **Third parties.** List linked services and link to their own deletion
   pages; Gunmetal cannot recall scrobbles already sent.
6. **Backups.** Covered by retention and the erasure ledger (section 7).

Account deletion (SEC-PRV-051): adults can delete their own account; the
account is disabled at once, every session and device grant ends (ASVS
7.4.2), and it can be restored by the user or an admin for 7 days. After
that, the pipeline runs. Admin-initiated deletion uses the same path and
notifies the user if they have any way to be reached.

Physical remnants on SSDs (wear levelling, unlinked blocks) are outside
what an application can sanitise; NIST SP 800-88 Rev. 2 treats media-level
sanitisation as a separate activity. The admin guide recommends full-disk
encryption so that retiring a disk is a key-destruction problem.

The test helper every erasure test shares: `assert_no_bytes(dir,
sentinel)`, which scans the database, `-wal`, `-shm`, log segments and any
index files for a byte string. Seed the data with unique sentinels (a
distinctive device name, a playlist title) so the scan is meaningful.

### 11. Client-side data

- **Web client, R1.** At sign-in, a checkbox reads "This is my personal
  device; keep my library available offline". Unticked (the default), the
  client keeps Activity and Identity data in memory only and stores at most
  the Library catalogue; ticked, it may use IndexedDB. Tokens never go in
  localStorage. On sign-out the client deletes IndexedDB, Cache Storage,
  local and session storage and unregisters service workers itself, and the
  server also sends `Clear-Site-Data: "cache", "cookies", "storage"`
  (SEC-PRV-019). The explicit deletion matters because browsers honour
  `Clear-Site-Data` only in secure contexts, and many home servers are first
  reached over plain HTTP on a LAN address.
- **Signed stream URLs.** `<audio>` and `<video>` elements, and cast
  receivers later, cannot send authorisation headers, so stream URLs carry a
  capability in the query string. This is a deliberate exception to ASVS
  14.2.1, mitigated by short lifetimes, binding to the session, opaque IDs
  with no titles (SEC-PRV-021), `Referrer-Policy: no-referrer`
  (SEC-PRV-018) and never logging query strings (SEC-PRV-043).
- **Native clients, R2.** Tokens and device keys in Keychain or Android
  Keystore. The synced library and downloads in app-private storage, marked
  excluded from iCloud and Android backups and invisible to the Android
  media store, so other apps cannot list what was downloaded
  (SEC-PRV-057). Push notifications carry an opaque ID only; the app fetches
  the text from the server (SEC-PRV-056). Lock-screen controls show the
  current track because the user asked to play it, but a private session
  donates nothing to Siri, Spotlight, Play Next or Top Shelf (SEC-PRV-058).
- **Remote access, R2.** iroh publishes a signed record under the node's
  key to n0's DNS server by default; the record needs only the relay URL,
  and direct addresses are optional. Publish relay-only by default, offer
  self-hosted relay and discovery, and say in setup that a relay operator
  can see node IDs and IP addresses but not content (SEC-PRV-059). If the
  project runs default relays, SEC-PRV-054 applies to them.
- **Live TV, R3.** Tuning an IPTV channel necessarily tells the stream's
  source what is being watched; the channel setup screen says so. Guide and
  playlist refreshes are scheduled, not triggered by viewing (SEC-PRV-060).

### 12. Law: household servers, friends abroad and the project's own services

This section is an engineer's reading of the sources, not legal advice
(unverified legal analysis throughout).

- **The household exemption.** GDPR does not apply to processing by a
  natural person in a purely personal or household activity (Article
  2(2)(c)); Recital 18 says this means no connection to professional or
  commercial activity, and that social networking and online activity can
  fall inside it. The CJEU construes the exemption narrowly: in Lindqvist
  (C-101/01) publishing personal data on a website open to an indefinite
  number of people was outside it, and in Ryneš (C-212/13) a home camera
  covering a public path was outside it.
- **Where an owner probably stands.** A server shared with the household
  and a small circle of friends, free of charge, looks like a household
  activity. A server opened to a community, to people the owner does not
  know, or to friends who pay towards it looks less like one, and the owner
  may then be a controller with duties to inform (Article 13), answer access
  (Article 15), erasure (Article 17) and portability (Article 20) requests,
  secure the data (Article 32) and design for data protection by default
  (Article 25). A friend being in another country does not change the first
  question, which is whether the activity is purely personal. The UK GDPR
  keeps the same structure (unverified detail). Switzerland, Brazil and
  India also exclude purely personal processing from their laws
  (unverified).
- **What the product does about it.** Gunmetal behaves as if Article 25
  applied to every server: minimal collection, private defaults, encryption
  and retention. It also gives every owner the tools a controller would
  need: the generated privacy notice shown before an invitation is redeemed
  (SEC-PRV-053), self-service export and deletion (SEC-PRV-047,
  SEC-PRV-049, SEC-PRV-051) and a retention schedule (SEC-PRV-005). It
  never offers billing or paid sharing (open decision 12), which keeps
  ordinary owners well inside household use.
- **Video history is sensitive in law, not only in taste.** The US Video
  Privacy Protection Act (18 U.S.C. § 2710) bars video businesses from
  disclosing which titles a customer obtained and requires destroying that
  data within a year of it no longer being needed. It applies to
  businesses, not households, but it shows how legislators rank this data.
- **The project as a controller.** The project never receives server data,
  so it is not a controller of it. It is a controller of whatever it
  collects through services it runs: the website, the update manifest,
  invite landing pages and any default relay. IP addresses can be personal
  data (CJEU, Breyer, C-582/14; the judgment page refused automated access,
  so unverified here). Those services therefore need a privacy notice, no
  trackers and short log retention (SEC-PRV-054), and invite secrets travel
  in URL fragments so the project never sees who invites whom
  (SEC-PRV-055).
- **The Cyber Resilience Act.** Its reporting obligations apply from 11
  September 2026 and most other obligations from 11 December 2027.
  Non-monetised open-source development is largely outside its scope, and
  open-source stewards have a lighter regime; whether any of it applies to
  Gunmetal needs advice (unverified legal analysis). Its essential
  requirements are good design targets regardless: confidentiality of
  stored data by encryption (Annex I Part I (2)(e)), data minimisation
  ((2)(g)) and letting users remove all data and settings permanently
  ((2)(m)).

### 13. Keeping it usable

Privacy controls that annoy people get switched off. The rules:

- **Private by default needs no action.** Most privacy here is the absence
  of features (no telemetry, no social feed, no admin history view), which
  costs users nothing.
- **Choices appear once, at the right moment, in plain words.** Providers
  are chosen during setup with one "use the recommended providers" button
  that shows what they receive; the update check is a single yes or no in
  setup; scrobbling appears in the user's own settings.
- **TVs hand off to phones.** Anything involving typing or reading long
  text (export, deletion, the privacy notice, linking Last.fm) shows a QR
  code on the TV and continues on a phone. The private-session toggle is the
  exception: it stays on the TV, two presses from the player.
- **Non-technical household members see one sentence, not a matrix.** The
  "what your admin can see" page leads with one sentence and puts the
  detail below it.
- **Defaults that never surprise.** Upgrades never flip a setting
  (SEC-PRV-023). Release notes list any new outbound feature and say it is
  off.

## Anti-patterns

### The record users cite

People who move to self-hosting often point to these incidents.

- **Plex, August 2017: removing the opt-out.** A privacy-policy update
  removed the ability to opt out of data collection. Users feared that
  playback data such as duration, bitrate and resolution could identify the
  files in their libraries, and that data which exists can be subpoenaed.
  Plex restored the opt-out, generalised playback statistics and said it had
  "ZERO interest in knowing" what libraries contain (TechCrunch, 21 August
  2017). Lessons: never collect per-file technical attributes off the
  server (SEC-PRV-010); never take a privacy choice away in an update
  (SEC-PRV-023).
- **Plex, November 2023: "Discover Together" and "Week in Review".** A
  social feature, opt-out by default, emailed users a digest of what friends
  had watched, including from personal servers, and people with library
  shares were added as friends. Onboarding set watch history to
  friends-only unless users changed it before clicking through (404 Media,
  27 November 2023; Plex forum thread 860206). Users also reported email
  preferences being switched back on (forum reports). Lesson: no surface
  ever shows one user's activity to another without that user's explicit,
  per-feature choice (SEC-PRV-022, SEC-PRV-030).
- **Plex, 2022 and September 2025: central account breaches.** plex.tv was
  breached twice; the 2025 breach exposed emails, usernames, hashed
  passwords and authentication data, and Plex told users to reset passwords
  and sign out everywhere (Android Authority, 8 September 2025). Lesson: a
  central account store is a target for every user at once; ADR 1's "no
  central account" is a privacy decision as much as a security one.
- **Spotify, August 2015: the permissions policy.** A new privacy policy
  said the app might ask to access photos, location, voice and contacts;
  after an angry response, the chief executive apologised for poor
  communication and said users did not have to share (9to5Mac and the
  Christian Science Monitor, 21 August 2015). Lesson: ask only for what a
  feature needs (MASVS-PRIVACY-1) and explain changes before shipping them.
- **Spotify, 2021: the speech-recognition patent.** A granted patent
  described inferring "emotional state, gender, age, or accent" from speech
  to recommend music. A coalition of more than 180 musicians and human
  rights groups asked Spotify to commit never to use it; Spotify said it had
  not implemented it (Access Now, April and May 2021). Lesson: Gunmetal's
  recommendations use library metadata and the household's own plays, never
  inferred personal traits (NIST PF CT.DP-P3).
- **Spotify, June 2023: unclear access responses.** Sweden's IMY fined
  Spotify SEK 58 million after a noyb complaint, because its responses to
  access requests did not explain clearly enough how data was used,
  including its technical log files (SecurityWeek, 14 June 2023; EDPB
  news). Lesson: exports come with a README (SEC-PRV-047).
- **Netflix Prize, 2006 to 2010: "anonymous" ratings.** Researchers showed
  that a few known ratings and dates were enough to re-identify subscribers
  in the released dataset, revealing apparent political preferences
  (Narayanan and Shmatikov, IEEE Symposium on Security and Privacy 2008).
  Netflix cancelled the sequel contest after an FTC inquiry and a lawsuit
  under the Video Privacy Protection Act (EPIC, March 2010). Lesson: there
  is no anonymous viewing history; it never leaves the server in any form
  (SEC-PRV-009, SEC-PRV-010).

### Things we must never do

| Anti-pattern | Why (incident or weakness) |
|---|---|
| Ship any social, digest or "friends' activity" feature on by default or opt-out | Plex Week in Review, 2023; CWE-359 |
| Change a privacy default, or enable a new outbound feature for existing users, in an upgrade | Plex 2017; Plex email preferences reported re-enabled, 2023; CWE-1188 |
| Send per-file technical attributes (duration, bitrate, resolution, hashes) off the server | Plex 2017 fingerprinting fears; Netflix Prize re-identification |
| Release or share "anonymised" history, even aggregated, outside the household | Netflix Prize, 2008 to 2010 |
| Store signing keys or third-party tokens in plain text in the database | Navidrome CVE-2024-56362 (GHSA-xwx7-p63r-2rj8): a plaintext JWT secret let anyone with the database forge admin tokens |
| Write passwords or tokens to logs, even on an error path | Navidrome 0.64.2 stopped writing the admin password to the log when initial setup failed; CWE-532 |
| Accept an account-link callback that names its user in the query string | Navidrome GHSA-8jrh-w926-8rvw: any user could redirect another user's scrobbles to their own Last.fm; CWE-639 |
| Let share or stream URLs keep working after the share is revoked | Navidrome GHSA-wp9c-pw66-c6j2: public share stream URLs outlived the share |
| Implement a compatibility API's "everyone's now playing" literally | OpenSubsonic `getNowPlaying` returns every user's playback with usernames |
| Look up lyrics, art or subtitles when the user presses play | Real-time disclosure of activity to third parties (design weakness, no specific incident found) |
| Load fonts, scripts, analytics or artwork in clients from third-party hosts | Leaks every viewer's IP address and browsing; OWASP User Privacy Protection Cheat Sheet |
| Put titles, user names or server addresses in push payloads | Payloads pass through Apple and Google services; MASVS-PRIVACY-1 |
| Offer "sign in as user" or an admin view of another adult's history | Turns the server into a surveillance tool inside a household; Privacy Risks P2 |
| Infer personal traits (mood, gender, age) for recommendations | Spotify patent controversy, 2021 |
| Write unencrypted backups, even "just local" ones | Backup folders are routinely synced to cloud drives; CWE-311 |
| Treat "append-only" as a reason data cannot be erased | Erasure must reach every copy; GDPR Art. 17; CWE-212 |
| Fetch provider data with TLS verification switched off | Immich shipped OIDC fetches with verification off in 2026 (per the users-and-security research); exposes lookups to network observers |
| Give the project a way to switch off or inspect users' servers remotely | Emby's 2023 shutdown of compromised servers showed a vendor kill switch exists in practice (per the users-and-security research) |

## Open decisions for the project owner

> **Status, 2026-10-02.** The owner decisions from every file in
> `docs/security/` are consolidated and de-duplicated in
> [README.md](README.md#open-decisions-for-the-owner). Where the baseline
> has since chosen, or where a recommendation below disagrees with the
> README, the README and the requirement tables win.

1. **What admins see in live sessions.** Recommendation: user, device,
   bitrate, playback method and title, with the title hidden for private
   sessions, and no per-user history view at all. Trade-off: owners who
   want to know exactly what their teenagers or friends watch will find this
   strict; showing titles at all is a concession to bandwidth management and
   stream stopping.
2. **How providers are chosen at setup.** Recommendation: an explicit step
   with nothing pre-ticked and one "use recommended providers" button that
   shows what they receive. Trade-off: one more setup step, and some users
   will finish with no artwork; pre-ticking would be smoother but is opt-out
   collection.
3. **Provider API keys.** Options: a project key shipped in the binary, a
   key each owner supplies, or a project-run lookup proxy. Recommendation:
   ship a project key for providers whose terms allow it, treated as a
   public client identifier and documented as an exception to ASVS 13.3.1,
   with an owner override; never run a project proxy. Trade-off: the
   project becomes the licensee and must police terms; a personal key links
   lookups to the owner's provider account; a proxy would let the project see
   every library.
4. **Telemetry, ever.** Recommendation: none in R1 and R2; revisit only
   with a public proposal following the Go toolchain model (local counters,
   opt-in upload, public schema, published aggregates). Trade-off: the
   project will not know which versions are deployed or which upgrades fail,
   which the operations research flags as a real cost (Plex used that
   knowledge to warn owners about CVE-2025-34158).
5. **The update check.** Recommendation: opt-in, asked once during setup,
   sending no identifiers. Trade-off: fewer owners will learn about security
   releases; opt-out would reach more but sends every server's IP address to
   the project by default.
6. **Whole-database encryption (for example SQLCipher).** Recommendation:
   not in R1; encrypt secrets and backups, recommend full-disk encryption.
   Trade-off: an unattended server must keep the database key next to the
   database, so the gain is mainly against copies of the file, which backups
   and bundles already address; the cost is a C dependency, slower scans and
   harder recovery.
7. **History the admin cannot read.** Encrypting each profile's history to
   keys held on the user's devices would hide it even from root. Recommend
   not pursuing it before Later, and only as research. Trade-off: the server
   still sees which files it streams, so the protection is partial; a lost
   key loses history; server-side recommendations and cross-device "continue"
   move to the clients.
8. **Default history retention.** Recommendation: keep until the user
   deletes it, with 90-day, 1-year and 2-year options per user. Trade-off: a
   finite default is better minimisation, but history is the feature users
   value most ("on this day", year-in-review) and ADR 1 calls it the only
   irreplaceable data.
9. **Web client persistence.** Recommendation: memory-only by default, with
   a "this is my personal device" checkbox for offline use. Trade-off: users
   on their own laptops must tick a box to get offline browsing in the web
   client.
10. **Encrypting offline downloads on phones (R2).** Recommendation: keep
    downloads in app-private storage, unencrypted at the application layer,
    relying on the platform's device encryption, with revocation on next
    contact. Trade-off: a rooted or forensically imaged device exposes the
    files; app-layer encryption costs battery and complicates background
    playback.
11. **Project-run relays and discovery (R2).** Recommendation: if the
    project runs any, publish their log policy (connection metadata only, at
    most 30 days), make self-hosting a first-class option, and publish
    relay-only records by default. Trade-off: cost, a soft central
    dependency, and controller duties for the project.
12. **Paid sharing.** Recommendation: never add billing, payment or
    "split the cost" features. Trade-off: some owners want them; offering them
    pushes owners out of household use and turns Gunmetal into a tool for
    running a streaming business.
13. **Household activity features ("popular on this server", what others
    are playing).** Recommendation: build them only as per-user opt-ins with
    the three-person threshold, and not in R1. Trade-off: families who like
    seeing each other's music lose a small pleasure in R1.
14. **Account deletion approval.** Recommendation: adults can delete their
    own accounts with a 7-day grace period and no admin approval. Trade-off:
    an admin cannot keep someone's history after they leave, which is the
    point.
15. **A legal home for project services.** Running gunmetal.tv, an update
    feed or relays means someone is the controller who signs the privacy
    notice. Recommendation: decide who that is (a person or an association)
    before R1 ships the update check, and get advice on CRA scope at the
    same time. Trade-off: cost and formality for a pre-alpha project.

## Sources

Standards and guidance:

- OWASP ASVS 5.0.0, V14 Data Protection: https://raw.githubusercontent.com/OWASP/ASVS/v5.0.0/5.0/en/0x23-V14-Data-Protection.md
- OWASP ASVS 5.0.0, V16 Security Logging and Error Handling: https://raw.githubusercontent.com/OWASP/ASVS/v5.0.0/5.0/en/0x25-V16-Security-Logging-and-Error-Handling.md
- OWASP ASVS master, V3, V7, V8, V11, V12, V13, V15: https://github.com/OWASP/ASVS/tree/master/5.0/en
- OWASP ASVS releases: https://github.com/OWASP/ASVS/releases
- OWASP Top 10:2025: https://top10.owasp.org/2025
- OWASP API Security Top 10 2023: https://api-security.owasp.org/editions/2023/en/0x11-t10
- OWASP MASVS releases and controls: https://github.com/OWASP/masvs/releases, https://github.com/OWASP/masvs/tree/master/controls
- OWASP Top 10 Privacy Risks v2.0: https://owasp.org/www-project-top-10-privacy-risks/
- OWASP Logging Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html
- OWASP User Privacy Protection Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/User_Privacy_Protection_Cheat_Sheet.html
- OWASP Password Storage Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html
- MITRE CWE REST API (names checked for every CWE cited): https://cwe-api.mitre.org/api/v1/cwe/weakness/
- NIST Privacy Framework 1.0 Core: https://www.nist.gov/document/nist-privacy-framework-version-1-core-pdf
- NIST Privacy Framework (1.1 initial public draft): https://www.nist.gov/privacy-framework
- NIST SP 800-88 Rev. 2 (final, September 2025): https://csrc.nist.gov/pubs/sp/800/88/r2/final
- NIST SP 800-218 SSDF 1.1 (final, February 2022): https://csrc.nist.gov/pubs/sp/800/218/final
- NIST SP 800-218 Rev. 1, SSDF 1.2 (initial public draft, December 2025): https://csrc.nist.gov/pubs/sp/800/218/r1/ipd
- NIST SP 800-63B-4 (final, July 2025): https://csrc.nist.gov/pubs/sp/800/63/b/4/final
- NIST SP 800-92 Rev. 1 (initial public draft, October 2023): https://csrc.nist.gov/pubs/sp/800/92/r1/ipd
- RFC 9562, UUIDs, section 8: https://www.rfc-editor.org/rfc/rfc9562.html
- RFC 6749, OAuth 2.0, section 10.12: https://www.rfc-editor.org/rfc/rfc6749#section-10.12
- age v1 specification: https://github.com/C2SP/C2SP/blob/main/age.md
- Clear-Site-Data (MDN, with link to the W3C specification): https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Clear-Site-Data
- SQLite pragmas (`secure_delete`, `auto_vacuum`): https://sqlite.org/pragma.html

Law and regulation:

- GDPR Recital 18: https://gdpr-info.eu/recitals/no-18/
- GDPR Article 2: https://gdpr-info.eu/art-2-gdpr/
- GDPR Article 25: https://gdpr-info.eu/art-25-gdpr/
- EDPS summary of recent case law (Ryneš): https://edps.europa.eu/sites/default/files/publication/15-06-02_recent_case_law_en_0.pdf
- Hogan Lovells on Ryneš and the household exemption: https://www.hoganlovells.com/en/publications/cctv-cjeu-narrows-the-scope-of-the-household-exemption
- 18 U.S.C. § 2710 (Video Privacy Protection Act): https://www.law.cornell.edu/uscode/text/18/2710
- CRA Annex I Part I (2)(e) text: https://www.cyberday.ai/requirement/cra-article-13-1-2-e-confidentiality
- CRA Annex I Part I (2)(m) text, quoted in ETSI CRA standards work: https://labs.etsi.org/rep/stan4cra/en-304-620-1/-/commit/a492952798dcc5db405aa42f84ca75512c54b47a
- CRA timeline and open-source stewards: https://www.noze.it/en/insights/cyber-resilience-act-sbom/

Incidents and vendor material:

- Plex 2017 privacy policy reversal: https://techcrunch.com/2017/08/21/plex-changes-its-new-privacy-policy-after-backlash-clarified-its-not-trying-to-see-whats-in-your-library/
- Plex 2017 (Betanews): https://betanews.com/2017/08/21/plex-data-collection-opt-out-backtrack/
- Plex Discover Together and Week in Review (404 Media): https://www.404media.co/plex-users-fear-discover-together-week-in-review-feature-will-leak-porn-habits-to-their-friends-and-family/
- Plex forum, "Weekly review emails data leak": https://forums.plex.tv/t/weekly-review-emails-data-leak/860206
- Plex 2025 breach (Android Authority): https://www.androidauthority.com/plex-data-breach-3595999/
- Spotify 2015 apology: https://9to5mac.com/2015/08/21/spotify-privacy-policy-apology/
- Spotify 2015 (Christian Science Monitor): https://csmonitor.com/Technology/2015/0821/Why-Spotify-is-apologizing-to-its-users
- Spotify speech-recognition patent coalition (Access Now): https://www.accessnow.org/spotify-spy-tech-coalition/
- Spotify IMY fine (SecurityWeek): https://www.securityweek.com/spotify-fined-5-million-for-breaching-eu-data-rules/
- Spotify IMY fine (EDPB news): https://www.edpb.europa.eu/news/national-news/2023/imy-issues-administrative-fine-against-spotify-shortcomings-regarding_en
- Netflix Prize de-anonymisation (Narayanan and Shmatikov): https://arxiv.org/pdf/cs/0610105
- Netflix cancels the second contest (EPIC): https://archive.epic.org/2010/03/netflix-cancels-contest-over-p.html
- Navidrome advisory GHSA-8jrh-w926-8rvw (Last.fm link callback): https://github.com/navidrome/navidrome/security/advisories/GHSA-8jrh-w926-8rvw
- Navidrome advisory GHSA-xwx7-p63r-2rj8 (CVE-2024-56362, plaintext JWT secret): https://github.com/navidrome/navidrome/security/advisories/GHSA-xwx7-p63r-2rj8
- Navidrome advisories list (including GHSA-wp9c-pw66-c6j2): https://github.com/navidrome/navidrome/security/advisories?state=published&page=2
- Navidrome 0.64.2 release notes: https://github.com/navidrome/navidrome/releases/tag/v0.64.2
- Navidrome insights collector: https://www.navidrome.org/docs/usage/admin/insights/
- OpenSubsonic `getNowPlaying`: https://opensubsonic.netlify.app/docs/endpoints/getnowplaying/
- OpenSubsonic `scrobble`: https://opensubsonic.netlify.app/docs/endpoints/scrobble/
- Go toolchain telemetry: https://go.dev/doc/telemetry
- Home Assistant analytics: https://www.home-assistant.io/integrations/analytics/
- MusicBrainz rate limiting and User-Agent rules: https://musicbrainz.org/doc/MusicBrainz_API/Rate_Limiting
- MusicBrainz database downloads: https://musicbrainz.org/doc/MusicBrainz_Database/Download
- TMDB daily ID exports: https://developer.themoviedb.org/docs/daily-id-exports
- Last.fm mobile authentication (session key lifetime): https://www.last.fm/api/mobileauth
- ListenBrainz JSON format: https://listenbrainz.readthedocs.io/en/latest/users/json.html
- iroh discovery: https://docs.iroh.computer/concepts/discovery
- iroh relays: https://docs.iroh.computer/concepts/relays

Project documents:

- [ADR 1: Architecture](../adr/0001-architecture.md)
- [ADR 2: Music is first-class](../adr/0002-music-is-first-class.md)
- [Research: users, sharing and security](../research/users-sharing-and-security.md)
- [Research: library and metadata](../research/library-and-metadata.md)
- [Research: setup, migration and operations](../research/setup-migration-and-operations.md)
- [Research: clients, platforms and offline](../research/clients-platforms-and-offline.md)
- [Research: discovery, home and search](../research/discovery-home-and-search.md)
