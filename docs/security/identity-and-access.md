# Identity, accounts and access

Written 2026-10-02 with web access. Standards, release dates and incidents
were checked against primary sources: OWASP's ASVS 5.0 source files on
GitHub, the OWASP Top 10:2025 and API Security Top 10 2023 sites, NIST's
SP 800-63B-4 pages, the RFC Editor and IETF datatracker, the W3C, vendor
security advisories and CVE records. The shared web-search budget ran out
part-way through, so later checks were made by fetching primary pages
directly. Anything not confirmed is marked "(unverified)".

This document designs who can use a Gunmetal server and what they can do.
The architecture records fix only four things: no central account,
passkeys and OIDC, device-bound keys, and remote access over iroh (record
1, decision 7), plus random IDs, per-object authorisation and short-lived
signed stream URLs (record 1, decision 6). Everything else here is a
proposal. Section 18 of the design guidance states it as a draft
architecture record.

## Summary

**One owner, capabilities rather than roles, and credentials kept apart from
people.** A server has exactly one owner, any number of administrators,
household members, managed profiles for children, and guests from outside
the household. Devices, API clients, plugins, other Gunmetal servers and
holders of public links are principals too, each with a hard ceiling on
what it can ever do. Roles are only presets: every check tests a named
capability, and nobody can grant a capability they do not hold. The data
model keeps credentials (passkeys, OIDC links, device keys), accounts (a
person) and profiles (history, queue, restrictions) separate from the first
release. That separation is cheap now and very expensive to retrofit.

**No passwords at all.** Browsers sign in with passkeys. Native apps hold a
non-exportable key pair per device. People who run an identity provider
can use OIDC. TVs pair by QR code, approved from a phone. There is no
password fallback, no security question, no email or SMS code and no
"trusted local network". Passwords are the main way media-server accounts
fall (the plex.tv breaches in 2022 and 2025, Navidrome's September 2026
brute-force advisory), and Emby's 2023 compromise came from passwordless
sign-in for "local" clients. Legacy music apps will later get
server-generated, scoped app passwords; these are never account
credentials.

**The server is claimed with a code printed on its own console, and
browsers need HTTPS.** A fresh server has no default account and serves
nothing but a claim page. The owner proves control of the host by entering
a one-time 128-bit code from the console or a file only the service user
can read. Passkeys only work in a secure context, on a domain name, with a
certificate the browser trusts (Chrome refuses WebAuthn on certificate
errors since version 110). So the web client must be reached over HTTPS on
a name, or on localhost. Release 1 supports the owner's own domain or
proxy, a tailnet name, or localhost. Whether the project runs a naming and
certificate service, as plex.direct does, is the most important decision
left to the owner.

**Short-lived, server-side, revocable sessions.** First-party tokens are
opaque random values stored as hashes and checked on every request; there
are no first-party JWTs. Revocation takes effect on the next request,
including for open streams and sockets. Browser sessions last at most 30
days, and administrator powers need a fresh passkey check that lapses after
15 idle minutes. From release 2, native access tokens last 10 minutes, are
bound to the device key (proof of possession) and are renewed by signing a
fresh challenge, so there are no refresh tokens to steal. Stream URLs are
re-checked against the live session and grant on every byte-range request,
which avoids Navidrome's 2026 stale-share bug.

**Deny by default, proven by tests.** Authorisation is one pure,
deny-by-default function in the core crate, under the existing 100%
coverage and zero-surviving-mutant gate. Every route declares a policy in
one route table, and the build fails if one does not. Every data access
goes through one visibility predicate. That predicate is the only place
library grants and parental filters live, so search, shuffle, shares and
the device sync payload cannot drift apart: Immich's two 2026 locked-folder
bypasses are the lesson. A suite generated from the route table replays
every route as every other principal.

**Households, sharing and recovery without a central account.** A shared
TV is a household device that shows a profile picker. It can never hold
administrator powers, and an optional PIN only stops a child switching into
someone else's profile. Friends get their own credentials on your server
through an invitation, whether or not they run a server of their own; no
server trusts another server's claims about its users until a later record
designs federation. Recovery is a ladder: another passkey, then recovery
codes, then a re-invitation from an administrator. The owner's last resort
is a command run on the host itself. Every security event goes into a
hash-chained, append-only security log, kept apart from listening history.
Each user can read their own events and gets an alert for new devices and
credential changes.

The design has 103 live requirements: 70 R1, 0 R1.1, 11 R1.2, 0 R1.3, 17 R2, 0 R3 and 5 Later, plus 7 withdrawn rows kept so their IDs stay stable.

## Threats

Likelihood is for a typical self-hosted install exposed to the internet by
a non-expert, before the mitigations.

| ID | Threat | Who (the attacker or failure) | Impact | Likelihood | Mitigated by (requirement IDs) |
|---|---|---|---|---|---|
| T-IAM-01 | A fresh, unclaimed server is claimed by someone other than its owner | A LAN neighbour, an internet scanner, or a hostile web page using DNS rebinding against the LAN | Critical: full control of the server and every library | Medium | SEC-IAM-005, 006, 007, 008, 009, 010, 101 |
| T-IAM-02 | Setup runs again after the claim, or a second owner appears through a race | A bug, or a concurrent request | Critical | Medium (Jellyfin fixed a re-runnable setup wizard in 12.0) | SEC-IAM-003, 009, 067 |
| T-IAM-03 | Default or shared credentials are used at scale | A botnet or mass scanner | Critical | Low by design, high in the industry | SEC-IAM-005 |
| T-IAM-04 | Password guessing, credential stuffing or reused passwords | Online attackers | High | High wherever passwords exist | SEC-IAM-025, 022, 101 |
| T-IAM-05 | A look-alike page captures a sign-in | A phisher | High | Medium | SEC-IAM-014, 018, 019, 020, 021, 025 |
| T-IAM-06 | Forged forwarding headers make a remote attacker look local or dodge rate limits | A remote attacker (Emby CVE-2023-33193; Navidrome GHSA-f295-6wp9-qqfg) | Critical | Medium | SEC-IAM-011, 012, 013 |
| T-IAM-07 | A session cookie or token is stolen and replayed | Infostealer malware, XSS, a leaked log or browser trace | High | Medium | SEC-IAM-015, 017, 037, 038, 039, 041, 042, 043, 047; from R2 050 |
| T-IAM-08 | A hostile site drives the signed-in browser (CSRF, cross-origin reads, hijacked WebSocket) | Any website the user visits | High | Medium | SEC-IAM-010, 014, 016, 040 |
| T-IAM-09 | A leaked stream URL keeps working after the session or share ends | Anyone the URL reaches (Navidrome GHSA-wp9c-pw66-c6j2) | Medium | Medium | SEC-IAM-043, 045, 046, 047; Later 082 |
| T-IAM-10 | One user reads or changes another user's items, playlists, shares or sessions by ID | A signed-in member or guest (Navidrome GHSA-82gh-4ggp-gfg5, GHSA-3g4p-jhv2-xrxf) | High | High | SEC-IAM-067, 070, 071, 072 |
| T-IAM-11 | An ordinary user reaches administrator routes or sets their own role | A signed-in member or guest | Critical | Medium | SEC-IAM-001, 002, 067, 068, 072, 074, 075 |
| T-IAM-12 | Delegation grants more than the delegator holds (API key, invitation, share) | A member, or a stolen limited key (Immich CVE-2026-23896) | High | Medium | SEC-IAM-073, 078, 083 |
| T-IAM-13 | A restriction is enforced in one place and forgotten in another (search, shuffle, shares, sync payload, folder view) | A child, a guest, or a bug (Immich CVE-2026-82272 and the /search/random bypass) | High | High | SEC-IAM-064, 070, 071, 076 |
| T-IAM-14 | A child on a shared TV switches into an adult or administrator profile, or guesses a PIN | A household child | Medium | High | SEC-IAM-049, 061, 062, 063, 065, 066 |
| T-IAM-15 | Device-code phishing: a victim approves the attacker's device | A remote phisher (Microsoft's Storm-2372 report, February 2025) | High | Medium | SEC-IAM-055, 056, 057, 058, 059, 060 |
| T-IAM-16 | An impostor server on the LAN collects a pairing approval or credentials | A LAN attacker spoofing mDNS | High | Low | SEC-IAM-051, 052, 057 |
| T-IAM-17 | A lost or stolen phone or TV keeps access and downloads | A thief, or a former household member | High | High | SEC-IAM-017, 042, 043, 048, 050, 053, 054 |
| T-IAM-18 | An OIDC account is taken over through email matching or careless linking | A user of the same or a different identity provider (nOAuth, 2023) | Critical | Medium | SEC-IAM-028, 029 |
| T-IAM-19 | OIDC protocol flaws: open redirect or XSS in the return URL, missing state, nonce or PKCE, mix-up, TLS checks off, SSRF through claim URLs | A remote attacker (Immich CVE-2026-53662, GHSA-qp2h-w794-2vhf, GHSA-hfvf-5c8x-8rc4) | Critical | Medium | SEC-IAM-026, 027, 032, 033, 034 |
| T-IAM-20 | The identity provider side grants power: auto-registration lets anyone in, a claim grants administrator, or the provider is compromised | A careless or compromised provider administrator | High | Medium | SEC-IAM-030, 031, 035, 036 |
| T-IAM-21 | An invitation link is forwarded, leaked or reused | A stranger who sees the link | Medium | Medium | SEC-IAM-078, 079, 080, 101 |
| T-IAM-22 | Recovery is abused: an administrator is talked into it, an administrator recovers the owner's account, or recovery codes are stolen | A social engineer, a rogue administrator, a thief | Critical | Low | SEC-IAM-089, 090, 091, 092, 098 |
| T-IAM-23 | The owner is locked out after losing every credential | Device loss | High | Medium | SEC-IAM-023, 024, 089, 092 |
| T-IAM-24 | An administrator watches users' listening without their knowledge | A curious or controlling administrator | Medium | Medium | SEC-IAM-077, 093, 104 |
| T-IAM-25 | A removed member keeps access through a device, key, socket or open stream | A former member (Jellyfin 12.1 fixed device revocation that left sessions open) | High | Medium | SEC-IAM-016, 043, 044, 103 |
| T-IAM-26 | The security log is altered, or the evidence was never written | An intruder, or a gap in logging | Medium | Low | SEC-IAM-093, 094, 097 |
| T-IAM-27 | Secrets leak into logs or diagnostic bundles | A bug (Navidrome 0.64.2 stopped logging admin passwords on failed setup) | High | Medium | SEC-IAM-007, 047, 095, 096 |
| T-IAM-28 | Server keys are stolen from the database or a backup | Anyone with a copy of the database or backup | High | Low | SEC-IAM-037, 100, 105 |
| T-IAM-29 | A compatibility adapter bypasses native sign-in or rate limits | A remote attacker (Navidrome GHSA-p994-r776-mw52) | Critical | Medium, once adapters ship | SEC-IAM-071, 084, 085, 101 |
| T-IAM-30 | A plugin abuses identity data or tokens, or sends history away | A malicious or compromised plugin | High | Medium, once plugins ship | SEC-IAM-086, 087 |
| T-IAM-31 | Another server impersonates its own users, or ours, to this server | A friend's server or its administrator | High | Low | SEC-IAM-081, 088 |
| T-IAM-32 | One account is shared far beyond the household | Members who share their sign-in | Low | High | SEC-IAM-042, 102 |
| T-IAM-33 | Floods of unauthenticated requests against sign-in, pairing or iroh | A remote attacker | Medium | Medium | SEC-IAM-052, 099, 101 |
| T-IAM-34 | A shared web origin lets one server relay a passkey assertion to another | A malicious server operator, if the project hosted one web app for all servers | Critical | Low (only if that option were chosen) | SEC-IAM-014, 018 |
| T-IAM-35 | Plain-HTTP traffic on a shared Wi-Fi network is read or altered | A housemate, a guest, or a compromised device on the LAN | High | Medium | SEC-IAM-011, 015, 039 |
| T-IAM-36 | Authentication or authorisation fails open when something errors | A bug | Critical | Medium | SEC-IAM-069 |
| T-IAM-37 | Identity data is lost in a cache rebuild or restore, locking everyone out | An operational failure | High | Medium | SEC-IAM-004, 105 |

## Requirements

Notation. "ASVS x.y.z" is OWASP ASVS 5.0.0 requirement v5.0.0-x.y.z, as
published in May 2025 and read from the project's source files on
2026-10-02. "A01:2025" is the OWASP Top 10:2025. "API1:2023" is the OWASP
API Security Top 10 2023. "NIST" is SP 800-63B-4 (final, July 2025, which
supersedes SP 800-63B of 2020). "WebAuthn L3" is the W3C Recommendation of
25 August 2026. "MASVS" is OWASP MASVS 2.1 (version per a secondary source,
unverified). RFC 10027 is BCP 247 on cross-device flows, published in 2026
from draft-ietf-oauth-cross-device-security. "Gate" means `scripts/gate.sh`:
100% coverage and zero surviving mutants.

Release scope assumed here. R1 is the server and the web client: owner
claim, passkeys, invitations, accounts and library grants, browser
sessions, byte serving, recovery and the security log. R1.2 adds OIDC
sign-in and music share links. R2 adds native phone,
TV and desktop clients, device keys, TV pairing, remote access over iroh,
household devices, managed profiles, parental filters and offline grants.
Adapters, API keys, plugins, public links and server-to-server features are
Later. Any of them that ships earlier brings its requirements with it.

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-IAM-001 | The identity model must keep credentials, accounts and profiles as separate entities: a credential authenticates an account, a profile belongs to an account (or, for a managed profile, to the household), and no profile, grant or session stores a credential. | ASVS 8.1.1; A06:2025; CWE-1220 | R1 | Unit tests on the core identity types; schema review at the R1 security review |
| SEC-IAM-002 | Every request must resolve to exactly one principal of a closed set of kinds (owner, administrator, member, managed profile, guest, device, API client, plugin, peer server, anonymous link holder), and a request presenting no valid credential for a protected route, or credentials for more than one principal, must be rejected. | ASVS 6.1.3, 8.2.1; API2:2023; CWE-287 | R1 | Property test on the principal resolver over every combination of presented credentials |
| SEC-IAM-003 | Exactly one account must hold the owner role at all times, and ownership must move only by a transfer that the current owner and the recipient each confirm with user verification in the previous 5 minutes. | ASVS 2.3.1, 7.5.1, 8.2.1; CWE-269 | R1 | Integration test on a real SQLite database (deleting, disabling or demoting the owner fails); property test that no operation sequence yields zero or two owners |
| SEC-IAM-004 | Accounts, credential public keys, devices, grants, invitations, policies and the security log must live in durable storage that cache rebuilds never touch and that every backup includes. | ASVS 14.1.1, 14.1.2; record 1 decision 5 (needs extending) | R1 | Integration test: rebuild the cache from the media files and assert every identity row and log entry is unchanged; backup and restore round trip |
| SEC-IAM-005 | The server must ship with no default account, password, token or API key, and must hold no credential that a person did not enrol. | ASVS 6.3.2, 13.2.3; A07:2025; CWE-1392, CWE-1393, CWE-798; UK PSTI regime (in force 29 April 2024, context only) | R1 | Integration test on an empty data directory (zero accounts and credentials after start); CI secret scan over the repository and release artefacts |
| SEC-IAM-006 | Until it is claimed, the server must answer only the claim page, its static assets and a health check, and must refuse every other route. | ASVS 8.2.1; A01:2025; CWE-306, CWE-1188 | R1 | Route-table integration test against an unclaimed server |
| SEC-IAM-007 | The claim code must have 128 bits from a CSPRNG, be single use, and expire 24 hours after it is generated (a restart does not change it). It must be shown only through host-side channels: process output and journal, a terminal QR code and a claim URL that carries the code in its fragment (`https://<name>/claim#<code>`), a file readable only by the service user, and `gunmetal claim-code`, which mints a new code once the old one has expired. A typed fallback must use grouped base32 with a checksum. The code must never appear in a network response or the security log, and failed attempts must never rotate or invalidate it. A native client may claim with a short numeric code only through the PAKE of SEC-OPS-010. | ASVS 6.4.1, 6.5.1, 6.5.3, 11.5.1, 16.2.5; SP 800-63B-4 §4.1.2.2; CWE-330, CWE-532, CWE-276 | R1 | Property test of the generator (length, alphabet, checksum) with the RNG injected; integration tests of file mode, expiry with an injected clock, single use and `claim-code` minting; canary crawl of every unauthenticated route; integration test that 50 wrong attempts from a hostile LAN source leave the code valid and undelayed for the owner on another source and on loopback |
| SEC-IAM-008 | The claim code must be accepted only from loopback or from a secure context on a configured origin (never through pairing or the browser edge), compared in constant time, and delayed per source on the SEC-API-056 schedule, with no server-wide limit that one source can exhaust; loopback is never delayed. | ASVS 2.4.1, 6.3.1, 11.2.4; CWE-307, CWE-208 | R1 | Integration tests: plain HTTP from a LAN peer refused (SEC-NET-001); a hostile source's failures do not delay another source or loopback; unit test pinning the constant-time comparator, mutation-tested |
| SEC-IAM-009 | Claiming must consume the code, create the owner and enrol the owner's first credential in one transaction, after which every setup route must return 404 permanently, including after restart and restore. | ASVS 2.3.1, 2.3.3, 15.4.2; CWE-841, CWE-367 | R1 | Integration test with two concurrent claims (exactly one succeeds); restart and restore tests that setup stays gone; generalised to every single-use secret by SEC-STD-029 |
| SEC-IAM-010 | The server must reject any request whose Host or :authority value is not a configured origin, a loopback name, or an address it is bound to. | A01:2025, A02:2025; CWE-346 | R1 | Integration test with a foreign Host header, shaped like DNS rebinding, against the claim page and the API |
| SEC-IAM-011 | **Withdrawn 2026-10-02: merged into SEC-NET-001.** One cleartext rule for the whole baseline. | ASVS 3.3.1, 12.1.1, 12.2.1; A04:2025; CWE-319, CWE-614 | Withdrawn | Proved by the tests of SEC-NET-001 |
| SEC-IAM-012 | **Withdrawn 2026-10-02: merged into SEC-NET-016.** One forwarding-header rule. | ASVS 4.1.3, 15.3.4; A01:2025, A07:2025; CWE-290, CWE-348 | Withdrawn | Proved by the tests of SEC-NET-016 |
| SEC-IAM-013 | Network location and other context signals (LAN, loopback through a proxy, VPN, relay or direct iroh path, home or internet posture, a new network, a new device) must never widen what a principal may do; they may only add friction (step-up, an alert, a shorter lifetime) or narrow access. Using an administrator capability from a network other than the one where the admin session last passed user verification must require fresh user verification. | ASVS 8.1.3, 8.1.4, 8.2.4, 8.4.2; A01:2025, A07:2025; CWE-290 | R1 | Property test of the policy function over generated contexts: adding or changing any signal never turns a denial into an allow; integration test of the network-change step-up |
| SEC-IAM-014 | The web client must be served by the user's own server on the same origin as the API, and the server must not send credentialed CORS responses to any other origin. | ASVS 3.4.2, 3.5.4; A02:2025; CWE-942 | R1 | Integration test of preflight and credentialed requests from a foreign origin; CI check for a wildcard Access-Control-Allow-Origin |
| SEC-IAM-015 | Every HTML response must send a Content-Security-Policy that allows no inline script and sets frame-ancestors 'none', plus Referrer-Policy no-referrer, X-Content-Type-Options nosniff and Cross-Origin-Opener-Policy same-origin; non-loopback HTTPS origins must also send HSTS of at least one year. | ASVS 3.4.1, 3.4.3, 3.4.4, 3.4.5, 3.4.6, 3.4.8; A02:2025; CWE-1021 | R1 | Integration test asserting the headers on every HTML route in the route table |
| SEC-IAM-016 | WebSocket upgrades must check Origin against the configured origins, authenticate the session before processing any message, and close when that session ends. | ASVS 4.4.1, 4.4.2, 4.4.3, 4.4.4, 7.4.1; CWE-1385 | R1 | Integration tests: foreign Origin refused; socket closed within 5 seconds of revocation |
| SEC-IAM-017 | The web client must keep no session or API token in localStorage, sessionStorage or IndexedDB, must partition cached library and history data per account, and must delete that cache at sign-out. | ASVS 14.3.1, 14.3.3; CWE-922 | R1 | Browser end-to-end test inspecting storage after sign-in, account switch and sign-out |
| SEC-IAM-018 | WebAuthn ceremonies must use a relying-party ID fixed at setup to the configured origin's host, and the server must verify the type, challenge, origin, RP ID hash, user-present flag, user-verified flag and signature of every registration and assertion. | WebAuthn L3 §7.1, §7.2; NIST §3.2.5; ASVS 6.3.3; CWE-347 | R1 | Fixture tests replaying recorded ceremonies with each field tampered in turn; fuzz target over the CBOR and COSE parsing path |
| SEC-IAM-019 | WebAuthn challenges must have at least 128 bits from a CSPRNG, be bound to the session or pre-session that requested them, be single use, and be valid for at most 5 minutes. | ASVS 6.7.2, 11.5.1; CWE-294 | R1 | Integration replay tests: same challenge twice, a challenge from another pre-session, an expired challenge (injected clock) |
| SEC-IAM-020 | Registration must request a discoverable credential with user verification required, and the server must reject any assertion whose UV flag is not set. | NIST Appendix B; WebAuthn L3 §7.2; ASVS 6.3.3 | R1 | Fixture test: an assertion with UV unset is rejected |
| SEC-IAM-021 | The server must store each credential's signature counter and its backup-eligible and backup-state flags, and must raise a security alert when a non-zero counter fails to increase. | WebAuthn L3 §6.1, §6.1.3; CWE-294 | R1 | Unit tests on the counter rule; integration test that the alert is logged and delivered |
| SEC-IAM-022 | Sign-in must be usernameless and must not reveal, through content, status or timing, whether an account or credential exists. | ASVS 6.3.8; CWE-204, CWE-208 | R1 | Integration test comparing responses for unknown, revoked and valid credentials; review that all paths do the same work |
| SEC-IAM-023 | An account must be able to hold several credentials, and adding or removing one must require user verification in the previous 5 minutes and must notify the account's other devices. | ASVS 6.3.7, 6.5.6, 7.5.1; NIST §4.1.2 | R1 | Integration tests with an injected clock |
| SEC-IAM-024 | Removing an account's last credential must be refused unless the account is being deleted. | ASVS 6.4.4; CWE-640 | R1 | Integration test |
| SEC-IAM-025 | The server must not offer account passwords, security questions, emailed or texted codes or links, or TOTP, for sign-in or for recovery. This is the decision of SEC-STD-006: there is no password path at all, and a person without a passkey-capable browser signs in by browser pairing (SEC-IAM-108). | ASVS 6.1.3, 6.3.6, 6.4.2; RFC 9700 §2.4; A07:2025; CWE-521, CWE-640 | R1 | Route-table CI check that no such endpoint exists; docs lint (SEC-STD-006) that fails if any password or TOTP requirement is live; review at each release |
| SEC-IAM-026 | OIDC sign-in must use the authorization code flow with PKCE (S256), a state value and a nonce, with the server as a confidential client, and must reject implicit and hybrid responses. | ASVS 10.2.1, 10.4.4, 10.4.6, 10.5.1; RFC 9700 §2.1.1, §2.1.2; RFC 7636; OpenID Connect Core 1.0 §3.1 | R1.2 | Integration tests against a real provider in a container and a hostile test provider (missing or wrong state, nonce or verifier; token in the fragment) |
| SEC-IAM-027 | ID tokens must be verified with keys from the provider's JWKS using algorithms pinned per provider (never "none", never a public key used as an HMAC secret), and iss, aud, azp where present, exp, iat and nonce must all be checked. | ASVS 9.1.1, 9.1.2, 9.1.3, 9.2.1, 9.2.3, 10.5.3, 10.5.4; OpenID Connect Core 1.0 §3.1.3.7; RFC 8725; CWE-347 | R1.2 | Unit tests with forged tokens (alg none, HS256 keyed with the RSA public key, wrong aud or iss, expired, replayed nonce); fuzz target on the JWS parser |
| SEC-IAM-028 | An OIDC identity must be keyed only by the pair (issuer, subject); email, preferred_username and name must never create, find or link an account. | ASVS 6.8.1, 10.3.3, 10.5.2; CWE-287, CWE-290 | R1.2 | Integration test: a hostile provider returns a victim's email under a new subject and gets no access to the victim's account |
| SEC-IAM-029 | Linking an OIDC identity to an existing account must happen only inside that account's session after user verification in the previous 5 minutes, or by redeeming an invitation. | ASVS 6.3.7, 7.5.1; CWE-287 | R1.2 | Integration test |
| SEC-IAM-030 | OIDC auto-registration must be off by default; when it is on, new accounts must get no library grants and only the guest preset until an administrator approves them. | ASVS 8.2.1; A01:2025, A06:2025; CWE-1188 | R1.2 | Integration tests with the setting off and on |
| SEC-IAM-031 | Provider claims must never confer the owner role, and mapping a claim to administrator must be off by default and enabled only by the owner, per provider. | ASVS 6.8.4, 8.2.1; CWE-269 | R1.2 | Unit tests on the claim mapper |
| SEC-IAM-032 | Server-side calls to an OIDC provider must verify TLS certificates and must not follow redirects to another host, and the server must never fetch a URL taken from a claim. | ASVS 12.3.2, 13.2.4, 15.3.2; A01:2025; API7:2023; CWE-295, CWE-918 | R1.2 | Integration tests: a provider with an untrusted certificate fails closed; a cross-host redirect is refused; a picture claim pointing at an internal address causes no request |
| SEC-IAM-033 | The OIDC redirect URI must be one exact registered URL on the configured origin, and any post-sign-in return target must be a relative path from an allowlist of client routes. | ASVS 3.7.2, 10.4.1; RFC 9700 §2.1; CWE-601, CWE-79 | R1.2 | Property test generating hostile return targets (javascript: and data: schemes, //host, /\host, encoded and mixed forms), all rejected |
| SEC-IAM-034 | Each authorization request must be bound to the one provider it was sent to, and the iss response parameter must be checked whenever the provider advertises RFC 9207 support. | ASVS 10.2.2; RFC 9207; RFC 9700 §2.1 | R1.2 | Integration test with two configured providers attempting a mix-up |
| SEC-IAM-035 | Sessions created through OIDC must have lifetimes set by Gunmetal, and disabling the account or removing its OIDC link must end every session created through that link. | ASVS 7.1.3, 7.4.2, 7.6.1 | R1.2 | Integration test |
| SEC-IAM-036 | Administrator elevation for an account with no passkey must require a fresh provider sign-in (max_age=0) whose auth_time is less than 5 minutes old, and must otherwise be refused; it never satisfies a fresh-uv action or the owner role (SEC-IAM-107). | ASVS 6.8.4, 7.5.3, 10.3.4 | R1.2 | Integration test with a provider that ignores max_age (elevation refused) |
| SEC-IAM-037 | First-party session and access tokens must be opaque values with at least 256 bits from a CSPRNG, stored server-side only as SHA-256 hashes and checked by lookup on every request; first-party sessions must not be JWTs. | ASVS 7.2.1, 7.2.2, 7.2.3, 11.5.1; CWE-330, CWE-312 | R1 | Unit test of the generator; integration tests that the database holds no raw token and a modified token fails |
| SEC-IAM-038 | A new session token must be issued at sign-in, at elevation and at every profile switch, and the previous token must stop working. | ASVS 7.2.4; CWE-384 | R1 | Integration test |
| SEC-IAM-039 | **Withdrawn 2026-10-02: merged into SEC-API-032.** One session-cookie definition. | ASVS 3.3.1, 3.3.2, 3.3.3, 3.3.4; CWE-614, CWE-1004, CWE-1275 | Withdrawn | Proved by the tests of SEC-API-032 |
| SEC-IAM-040 | Every state-changing request authenticated by cookie must carry the client's custom request header and an Origin (or Sec-Fetch-Site: same-origin) matching a configured origin, or be refused. | ASVS 3.5.1, 3.5.2, 3.5.3; A01:2025; CWE-352 | R1 | Route-table integration test of every mutating route without the header and from a foreign origin |
| SEC-IAM-041 | Browser sessions must end after 7 days without use or 30 days in total. Administrator rights must need a separate admin session, created only by a user-verifying sign-in (a passkey with user verification, or a device key unlocked by the platform), held in its own `__Host-` cookie with SameSite=Strict, and ending after 15 minutes without an admin action or 1 hour in total; a media session never authorizes an admin route. Every route must carry exactly one tag from a closed set (none, elevated, fresh-uv). Fresh-uv routes need a user-verified assertion no older than 5 minutes: trusted proxies, posture and remote administration, TLS and naming settings, egress policy, plugin installs and grants, enabling adapters, backup download and restore, ownership transfer, key rotation, creating or promoting administrators, adding or removing library roots, and file-system browsing. | ASVS 3.5.4, 7.1.1, 7.3.1, 7.3.2, 7.5.1, 7.5.3, 8.4.2; SP 800-63B-4 §2.2.3; A01:2025; CWE-306 | R1 | Route-table test that every route has exactly one tag and that a media cookie is refused on every admin route; integration tests of both windows with an injected clock; per-route test of the fresh-uv list |
| SEC-IAM-042 | Each user must be able to list their own sessions, devices, API keys and app passwords with client name, device class and key level, network type, coarse location and last use, and revoke any one, or all except the current session; changing or removing a credential must offer to end all other sessions; admins must be able to revoke any user's session or device. | ASVS 7.4.3, 7.5.2, 10.4.9 | R1 | Integration test, including that a revoked device's token and stream URL fail on the next request (SEC-TM-028); browser end-to-end test of the screen |
| SEC-IAM-043 | Revoking a session, device, credential or account, or disabling an account, must make every token, stream URL, WebSocket and in-progress stream derived from it fail on its next request, and within 5 seconds for open connections. | ASVS 7.4.1, 7.4.2, 8.3.2; A07:2025; CWE-613 | R1 | Integration test: start a ranged stream and a socket, revoke, assert the next range request is refused and the socket closes within 5 seconds |
| SEC-IAM-044 | Administrators must be able to end any or all sessions of any non-owner account, and the owner of any account. | ASVS 7.4.5 | R1 | Integration test, including an administrator's attempt on the owner's sessions (refused) |
| SEC-IAM-045 | **Withdrawn 2026-10-02: merged into SEC-API-026, SEC-API-027, SEC-API-028, SEC-API-029.** Every media consumer, the web client included, uses capability URLs; one lifetime table lives in SEC-API-027. | ASVS 9.1.1, 9.2.1, 9.2.2, 14.2.1 (documented exception); CWE-598, CWE-613 | Withdrawn | Proved by the tests of SEC-API-026, SEC-API-027, SEC-API-028, SEC-API-029 |
| SEC-IAM-046 | Byte-serving routes must check on every request that the session behind the stream grant is live and that the principal still holds a grant to the item. | ASVS 8.2.2, 8.3.2; API1:2023; CWE-613 | R1 | Integration test: remove the grant mid-stream and assert the next range request is refused |
| SEC-IAM-047 | Stream signatures, invitation and share codes, OAuth codes and state values in request URLs must be redacted before any log line or diagnostic bundle is written. | ASVS 14.2.1, 16.2.5; CWE-532, CWE-598 | R1 | Log-canary scan over the whole integration suite |
| SEC-IAM-048 | Native clients must authenticate with a per-device key pair generated on the device as non-exportable, in hardware-backed storage where the platform offers it, and must hold no long-lived bearer secret. | MASVS-AUTH-1, MASVS-CRYPTO-2, MASVS-STORAGE-1; ASVS 6.7.1; NIST §3.1 | R2 | Device tests on emulators and reference hardware asserting key attributes; review |
| SEC-IAM-049 | The server must record each device's class and its key's reported protection level, and household devices and devices whose key is software-only must be capped below every administrator capability. | ASVS 8.2.1, 8.4.2; MASVS-AUTH-3; CWE-269 | R2 | Property test on the capability ceiling |
| SEC-IAM-050 | Native access tokens must be sender-constrained to the device key (a DPoP proof on each HTTPS request, or a signature over both iroh endpoint IDs and the TLS exporter on iroh), must expire within 10 minutes, and must be renewed only by signing a fresh server challenge, with no refresh tokens. | ASVS 10.3.5, 10.4.14; RFC 9449; RFC 9266; RFC 8446 §7.5; CWE-294 | R2 | Integration tests: a token used with another key, a replayed proof (same jti), and a proof made for another server are each refused |
| SEC-IAM-051 | Native clients must connect only to the server endpoint ID pinned from an invitation or pairing, and must refuse to authenticate to any other endpoint. | MASVS-NETWORK-2; ASVS 12.3.2; CWE-295, CWE-940 | R2 | Integration test with an impostor endpoint advertising the same name |
| SEC-IAM-052 | Before authentication, an iroh connection must reach only the pairing, invitation and challenge protocols, rate-limited per remote endpoint ID and server-wide, and an unauthenticated request must never create a user-visible prompt or notification (SEC-STD-027). | ASVS 2.4.1, 8.2.1; API4:2023; CWE-306, CWE-770 | R2 | Integration test over a real iroh connection; integration test that 1,000 unauthenticated pairing requests create zero notifications, events or prompts addressed to any user |
| SEC-IAM-053 | Native clients must keep enrolment records and cached tokens only in platform secure storage, and must delete them, with any synced data for that account, at sign-out or when told the device is revoked. | MASVS-STORAGE-1, MASVS-STORAGE-2; ASVS 14.3.1 | R2 | Device tests |
| SEC-IAM-054 | Offline playback grants must be signed by the server, bound to one device key and a list of items, expire within an administrator-set maximum (default 30 days), and be withdrawn at the device's next contact after revocation. | ASVS 9.1.1, 9.2.1; MASVS-STORAGE-1 | R2 | Integration test; device test that withdrawn downloads are deleted |
| SEC-IAM-055 | Devices without a keyboard must sign in through a device authorisation flow profiled from RFC 8628, in which the device_code has at least 256 bits and is bound to the device's public key, so that every poll must be signed by that key. | RFC 8628 §3, §5.2; ASVS 6.6.2; CWE-294 | R2 | Integration test: polling with the right device_code but another key is refused |
| SEC-IAM-056 | Pairing user codes (browser pairing from R1, device authorisation from R2) must be 8 characters from the RFC 8628 §6.1 base-20 alphabet, valid once and for 10 minutes; a code must die after 5 wrong guesses, and code entry must also be delayed per approver on the SEC-API-056 schedule and capped server-wide. | RFC 8628 §5.1, §6.1; ASVS 6.5.1, 6.5.5, 6.6.3; CWE-307 | R1 | Unit test of the generator; integration tests of expiry, the per-code limit and the per-approver schedule (injected clock) |
| SEC-IAM-057 | The pairing QR code must carry the identity key of the server the requesting device is connected to and an approval URL on the server's own HTTPS origin. A native approving client must refuse when that key differs from the one it pinned for the server; any other signed-in personal-class client, the web client included, may approve by opening the URL, where the passkey's origin binding gives the phishing resistance and the user code must be confirmed on screen. | RFC 10027; MASVS-NETWORK-2; ASVS 6.5.5; CWE-940 | R1 | End-to-end test of approval from a phone browser (R1); integration test with an impostor server on the LAN for the native path (R2) |
| SEC-IAM-058 | The approval screen must show the requesting device's self-reported name marked as unverified, its type, "In this home" or "Somewhere else" as decided by the transport (SEC-IAM-060), how long ago the request started, and the exact account or profiles and capabilities it will get, with addresses only behind a "Details" link; approval must require user verification in the previous 5 minutes. A matching code is required only on the path of SEC-IAM-060. | RFC 10027; RFC 8628 §5.4; ASVS 7.5.1, 7.6.2 | R1 | Component test asserting each field, with a hostile name containing bidirectional controls; integration test that approval without fresh verification is refused; manual wording review with non-technical testers |
| SEC-IAM-059 | A device or browser authorisation must never grant owner or administrator capabilities; adding a household device must require the household-device capability; and a remote or typed-code enrolment (SEC-IAM-060) must never create a household-class device: it grants one named profile at most and alerts every adult of the household. | ASVS 8.2.1; RFC 10027; CWE-269 | R1 | Property test over approvers, requested classes and path classes; integration test that a QR-only remote enrolment cannot produce a household device and alerts every adult |
| SEC-IAM-060 | An approval counts as local only when the server saw the approving request and the requesting device on the same local path class (SEC-NET-024) or on the same direct iroh path. Otherwise (different or unknown networks, and every remote enrolment, including the "TV somewhere else" flow) the user must type the code shown on the requesting device rather than follow a link or a QR image, and must confirm a matching code shown on both screens; the prompt is worded neutrally ("Type the code on the TV to confirm"). The web client's typed-code fallback must be offered only on a local path. | RFC 10027; RFC 8628 §5.4; ASVS 6.6.2, 6.6.3; CWE-451 | R1 | Integration test matrix over path classes; manual phishing review of the remote flow, including a QR image sent by an attacker |
| SEC-IAM-061 | A household device's profile picker must list only the household profiles enabled for that device, and must expose no guest, account identifier or credential. | ASVS 8.2.2, 14.2.6; CWE-359 | R2 | Integration test against a server with members, managed profiles and guests |
| SEC-IAM-062 | Profile PINs must be checked only by the server, stored only as Argon2id hashes with a pepper kept outside the database, and never sent to, stored or compared by any client. Failures must follow the SEC-API-056 schedule scoped to the pair (household device, profile), never to the profile everywhere and never permanently; after 10 failures the guardian or profile owner gets one alert, folded into the daily summary; the PIN owner can always enter from their own personal device. | ASVS 6.3.1, 6.5.2, 11.4.2; NIST §3.2.2, §3.2.10; CWE-307, CWE-916 | R2 | Integration test of the schedule with an injected clock; test that the profile stays usable from another device while one device is delayed; test that no API response or sync payload carries a PIN-derived field; unit test of the hash parameters |
| SEC-IAM-063 | A PIN must only gate switching into a profile, and must never authorise administrator capabilities, credential changes, invitations, grant changes or device enrolment. | ASVS 8.2.1; CWE-1390 | R2 | Property test: no action other than a profile switch accepts a PIN |
| SEC-IAM-064 | Content policy (rating ceilings, unrated handling, the explicit-content filter, tag rules and per-item exceptions) must be enforced inside the shared visibility predicate, so that it applies to every list, search, recommendation, artwork, lyrics, subtitle, stream, share and sync payload. | ASVS 8.2.2, 8.3.1; A01:2025; CWE-602, CWE-863 | R2 | Visibility conformance suite: a restricted profile sees exactly the expected item set through every route (set equality against an independent reference) |
| SEC-IAM-065 | A household device may keep at rest the synced library or history of a PIN-protected profile only encrypted under a per-profile key that the server releases after a successful PIN check and that the device forgets when it switches away from the profile. | MASVS-STORAGE-1; CWE-359 | R2 | Device test that the partition is ciphertext with no key present after switching away; timing-budget test for re-entry |
| SEC-IAM-066 | A household device locked to a managed profile must leave it only with another profile's PIN or an approval from a guardian's device. | ASVS 8.2.1 | R2 | Integration and interface test |
| SEC-IAM-067 | Every HTTP, WebSocket, iroh and adapter route must declare its authorisation policy in one route table, and the build must fail if a route has none, or is public without appearing on a reviewed allowlist. | ASVS 8.1.1, 8.2.1; A01:2025; API5:2023, API9:2023; CWE-862 | R1 | CI route-table test |
| SEC-IAM-068 | Authorisation must be decided by one deny-by-default pure function in the core crate that takes the principal, its ceilings, the action, the resource and the context, and returns either allow or a typed denial reason. | ASVS 8.1.1, 8.3.1; A06:2025; CWE-285, CWE-863 | R1 | Unit and property tests under the gate (100% coverage, zero surviving mutants) |
| SEC-IAM-069 | Any error, timeout or missing data during authentication or authorisation must end in denial and a security-log entry, never in a fallback to a weaker check. | ASVS 16.5.2, 16.5.3; A10:2025; CWE-636 | R1 | Fault-injection integration tests (database error, malformed stored policy, missing grant row) |
| SEC-IAM-070 | Every object fetch, list, search and mutation must pass through one shared visibility predicate built from the principal's library grants, item shares and (from R2) content policy. | ASVS 8.2.2; A01:2025; API1:2023; CWE-639 | R1 | Type-level enforcement (repository functions require the predicate) plus the visibility conformance suite |
| SEC-IAM-071 | A cross-principal suite generated from the route table must replay every route with object IDs belonging to another principal and assert denial, for the native API and for every adapter. | ASVS 8.2.2; API1:2023, API5:2023; CWE-639 | R1 | The suite itself, run in the gate against a real SQLite database |
| SEC-IAM-072 | Request bodies must be decoded into per-action types that hold only the fields that action may change and reject unknown fields, so that owner, role, capability, grant and principal-ID fields cannot be set through general update routes. | ASVS 8.2.3, 15.3.3; API3:2023; CWE-915 | R1 | Unit tests per action type; integration tests posting extra fields |
| SEC-IAM-073 | No principal must be able to grant, delegate or mint, through roles, invitations, API keys, shares or plugin approvals, any capability or grant it does not itself hold, on creation or on update. | ASVS 8.2.1; A01:2025; CWE-269 | R1 | Property test over random principals and requested grants, covering create and update paths |
| SEC-IAM-074 | Authorisation code must test capabilities and never role names; roles must exist only as named presets of capabilities. | ASVS 8.1.1; CWE-1220 | R1 | CI lint rejecting role comparisons outside the preset module; unit tests |
| SEC-IAM-075 | The owner-only capabilities listed in design guidance section 3 must not be grantable to any other principal. | ASVS 8.2.1; CWE-269 | R1 | Property test |
| SEC-IAM-076 | Changes to roles, grants, content policy or account status must apply from the next request of every affected session, and (from R2) devices must be told to purge synced data that is no longer visible. | ASVS 8.3.2; CWE-613 | R1 | Integration test |
| SEC-IAM-077 | An administrator's access to another user's data (history, profile, devices, sessions) must be recorded in that user's own visible security log. | ASVS 16.3.3; CWE-359, CWE-778 | R1 | Integration test |
| SEC-IAM-078 | Invitations must carry a secret of at least 128 bits in the URL fragment, an expiry (default 7 days), a use limit (default 1) and a preset no greater than the inviter's own capabilities and grants, must be revocable until used, and must be redeemable only in a native app or a secure context; a project-hosted landing page must run no script that reads the secret. | ASVS 8.2.1, 11.5.1, 14.2.1; CWE-269 | R1 | Unit and integration tests; log-canary scan proving the secret never reaches a server log; CI check that the website's invite page contains no script reading location.hash |
| SEC-IAM-079 | Redeeming an invitation must enrol the invitee's own passkey, OIDC link or device key in the same transaction and must never create or reveal a shared password. An invitation that confers member, household or more-than-one-library grants must leave the new account pending, with no grants, until the inviter confirms it after seeing the redeeming device's description and a short matching code; a guest invitation for a single library may complete in one step. | ASVS 2.3.3, 6.4.1, 8.2.1; CWE-1391, CWE-269 | R1 | Integration tests: pending accounts hold no grants; confirmation with a wrong matching code fails; a guest single-library invitation completes in one step |
| SEC-IAM-080 | Guests must by default have no household-device access, no administrator capabilities, no view of other accounts or their activity, no downloads, and only the libraries named in their invitation. | ASVS 8.2.1, 8.2.2; CWE-359 | R1 | Integration test of the guest preset |
| SEC-IAM-081 | Until an accepted architecture record defines federation, no request must be authorised on the strength of another server's assertion about a user, and friends who run their own server must hold their own credentials on this one. | ASVS 6.8.1, 8.3.3; CWE-290 | R1 | Route-table CI check that no federation route exists; review |
| SEC-IAM-082 | **Withdrawn 2026-10-02: merged into SEC-API-097.** Share links are R1 for music, with per-link limits. | ASVS 8.2.2, 11.5.1; API1:2023 | Withdrawn | Proved by the tests of SEC-API-097 |
| SEC-IAM-083 | API keys and app passwords must be generated by the server with at least 128 bits, shown once, stored hashed, scoped to a subset of the creator's non-administrator capabilities, expire by default, and be listed and revocable alongside the user's devices. | ASVS 7.5.2, 8.2.1, 11.5.1; API2:2023; CWE-269, CWE-522 | R2 | Integration tests, including a key's attempt to widen its own scope |
| SEC-IAM-084 | Compatibility adapters must be off by default, must accept only adapter-specific keys or app passwords and never an account credential, and must call the same policy function as the native API. | ASVS 6.1.3, 6.3.4; API2:2023, API9:2023; OpenSubsonic API key extension | Later | Cross-principal suite run against each adapter; integration test that account credentials are refused |
| SEC-IAM-085 | Legacy Subsonic token-and-salt sign-in, if offered at all, must be off by default, enabled per app, and use a random app password encrypted under a key kept outside the database. | ASVS 11.4.2, 13.3.1; CWE-257 | Later | Integration test; review |
| SEC-IAM-086 | A plugin must hold only the capabilities in its owner-approved manifest, must never hold an identity capability (reading credentials or tokens, creating sessions, changing roles or grants), and must act with the originating user's permissions when it acts for a user. | ASVS 8.3.3, 15.2.5; A03:2025, A08:2025; CWE-250, CWE-269 | Later | Integration tests with a hostile test plugin |
| SEC-IAM-087 | A feature that sends a user's data to a third party through a plugin (such as scrobbling) must be switched on by that user, not by an administrator, and be revocable by that user. | ASVS 14.2.3; CWE-359 | Later | Integration test |
| SEC-IAM-088 | A peer server must be identified only by an iroh endpoint ID that both owners approved, be scoped to named data flows, and never be able to authenticate users. | ASVS 8.3.3, 13.2.1; CWE-290 | Later | Integration test |
| SEC-IAM-089 | Owners and administrators must be offered 10 single-use recovery codes of at least 80 bits each during first enrolment; members and guests find them under Account > Recovery, next to an explanation of admin recovery. Codes must be stored only as peppered hashes and usable only to enrol a new credential, which then starts the recovery hold (SEC-IAM-106). | ASVS 6.5.1, 6.5.2, 6.5.3, 6.5.4; NIST §3.1.2, §4.2 | R1 | Unit test of the generator; integration tests (reuse refused; a recovery session can do nothing except enrol); component test of the enrolment flow per role |
| SEC-IAM-090 | Recovery by code or link must notify all of the account's devices, write a security-log entry, and show the user every existing credential and device to review. | ASVS 6.3.7, 6.4.3; NIST §4.2 | R1 | Integration test |
| SEC-IAM-091 | Administrators may issue one-time recovery enrolment links for members and guests only, the owner may issue them for administrators, and nobody may issue one for the owner. A link must be redeemed either in person (a QR code on the issuer's screen with the account holder present) or on a device the account holder already approved, and the credential it enrols starts the recovery hold (SEC-IAM-106). | ASVS 6.4.4, 8.2.1; CWE-269, CWE-640 | R1 | Property test over issuer and target roles; integration test that an admin redeeming a link they issued cannot read the member's history before the hold ends |
| SEC-IAM-092 | Owner recovery must be available only through a command on the host that reaches the server over a local socket accessible only to the service user; it must produce an enrolment link valid for 15 minutes and must alert every administrator. | ASVS 6.4.4; CWE-306, CWE-640 | R1 | Integration test of socket permissions; route-table check that no network route offers owner recovery; route-table test that owner recovery is the only recovery absent from the network |
| SEC-IAM-093 | **Withdrawn 2026-10-02: merged into SEC-OPS-020.** The audit log owner now lists the required fields. | ASVS 16.2.1, 16.2.2, 16.3.1, 16.3.2, 16.3.3; A09:2025; CWE-778 | Withdrawn | Proved by the tests of SEC-OPS-020 |
| SEC-IAM-094 | Security-log entries must be hash-chained, and a verification command must detect any modified, removed or reordered entry. | ASVS 16.4.2 | R1 | Property test: random tampering is always detected |
| SEC-IAM-095 | Secrets must be held in a wrapper type that cannot be formatted into a log line, error message or diagnostic bundle. | ASVS 13.3.1, 16.2.5; CWE-532 | R1 | The type has no unredacted Display or Debug (compile-time); log-canary scan over the full integration suite |
| SEC-IAM-096 | Log fields derived from requests must be written as structured, escaped values, never concatenated into the line. | ASVS 16.4.1; CWE-117 | R1 | Fuzz target and unit tests with CR, LF and control characters |
| SEC-IAM-097 | Users must be able to read their own security events, guardians those of their managed profiles, and only holders of the audit capability all events. | ASVS 7.5.2, 16.4.2 | R1 | Cross-principal suite covering the log routes |
| SEC-IAM-098 | An account's devices must be notified of a new device, a new or removed credential, use of recovery, a change of OIDC link, a change of role and failed-attempt bursts above the threshold; re-authentication with an existing credential on a known device must produce no notice, and non-critical notices must be batched into a daily summary. | ASVS 6.3.5, 6.3.7 | R1 | Property test over generated event streams asserting the exact notice set; integration test that a known-device re-sign-in produces zero notices |
| SEC-IAM-099 | Failed sign-in, code and PIN attempts must also be written as single lines in a stable, documented format suitable for fail2ban. | ASVS 16.2.4 | R1 | Unit test against the documented format |
| SEC-IAM-100 | **Withdrawn 2026-10-02: merged into SEC-OPS-012.** The key-file rule now lists the stream-URL key and the PIN and recovery-code peppers. | ASVS 11.1.2, 13.3.1, 13.3.4; CWE-312, CWE-522 | Withdrawn | Proved by the tests of SEC-OPS-012 |
| SEC-IAM-101 | Every endpoint that checks a secret (claim, invitation, recovery code, user code, PIN, share password) or starts a sign-in ceremony must be rate-limited per source and server-wide, with the limits documented. | ASVS 2.4.1, 6.1.1, 6.3.1; API4:2023, API6:2023; NIST §3.2.2; CWE-307, CWE-770 | R1 | Route-table-driven integration test that each such route throttles |
| SEC-IAM-102 | The server must enforce documented limits, adjustable by administrators, on enrolled devices and concurrent playback streams per account, counting playback and not browsing. | ASVS 7.1.2; API4:2023 | R1 | Integration test |
| SEC-IAM-103 | Disabling an account must end its sessions at once; deleting one must block sign-in immediately, keep the data for a 7-day grace period, and offer the user an export first. | ASVS 7.4.2 | R1 | Integration test with an injected clock |
| SEC-IAM-104 | Each user must be able to see a page stating what administrators and guardians on the server can see about them, generated from the same policy the server enforces. | ASVS 14.2.6; CWE-359 | R1 | Snapshot test that the page changes when the visibility policy changes |
| SEC-IAM-105 | Backups that contain identity data or the security log must be encrypted before they leave the host. | ASVS 11.3.2, 14.2.4; CWE-311 | R1 | Integration test of the export path |
| SEC-IAM-106 | A credential enrolled through a recovery code or an admin-issued recovery link must start a recovery hold (72 hours by default; the owner may choose 24 to 72). During the hold it cannot remove other credentials, elevate, export history or create invitations; every pre-existing credential and device of the account gets an alert that can end the hold and revoke the new credential with one tap; and when the hold began from an admin-issued link, the account's history, playlists and other private data stay hidden from the new credential until the hold ends. | ASVS 6.4.3, 6.4.4, 6.3.7; SP 800-63B-4 §4.2; CWE-640 | R1 | Integration tests with an injected clock: each restricted action refused during the hold and allowed after; one-tap cancel from an existing device revokes the new credential; an admin who redeems a link they issued sees no history before the hold ends |
| SEC-IAM-107 | Owner-only and fresh-uv actions (SEC-TM-017, SEC-IAM-041) must be satisfied only by a passkey or device key enrolled on this server, never by an OIDC sign-in alone, and the owner account must always hold at least one non-OIDC credential (removing the last one is refused). | ASVS 6.8.4, 7.5.3, 8.2.1; CWE-269, CWE-287 | R1 | Property test over credential types and actions; integration test that removing the owner's last passkey is refused |
| SEC-IAM-108 | A person whose browser cannot use a passkey must be able to sign it in by approval from one of their signed-in personal-class devices (ACC-062), under SEC-IAM-056 to SEC-IAM-060. The paired browser generates a non-extractable Web Crypto key, its session is renewed only by a signature over a fresh server challenge, and it is a limited-class device that can never hold administrator capabilities, approve other devices or change account security. | ASVS 6.3.3, 6.5.5, 8.2.1; RFC 8628 §5.4; CWE-269 | R1 | End-to-end test that pairs a browser with WebAuthn disabled and asserts the limited class; integration tests that the paired session is refused on admin and account-security routes and that renewal without the key fails |
| SEC-IAM-109 | Household-class devices must by default be usable only on local-direct path classes (SEC-NET-024); one that appears on a remote path must be suspended and the household's adults alerted. A household device unused for 30 days must become dormant: its key stays enrolled but is refused until any adult approves it again with one tap from a personal device; a device dormant for 365 days must be deleted. | ASVS 6.5.6, 7.3.1, 8.4.2; CWE-613 | R2 | Integration test that a household device key presented over the edge or a public proxy is refused and suspended; injected-clock tests of dormancy, reactivation and deletion |
| SEC-IAM-110 | On a household device, an adult profile's Activity data (history, continue watching, private playlists) must stay hidden until a PIN or an approval from that adult's phone unlocks the profile for the session, while browsing and playback stay one tap; adding an adult profile to a TV must preselect "Add a PIN". | MASVS-PRIVACY-1; ASVS 8.2.2; CWE-359 | R2 | End-to-end test on the TV build; component test of the preselected PIN step |

**Bound by requirements in [standards-coverage.md](standards-coverage.md).** SEC-STD-006 (the identity architecture record), SEC-STD-008 (user-chosen secrets that remain: share-link passwords and backup passphrases), SEC-STD-025 (OIDC scopes and logout), SEC-STD-026 (Gunmetal is not an OAuth server or OpenID Provider), SEC-STD-027 (no prompts an outsider can trigger) and SEC-STD-029 (single-use secrets under concurrency).
The 2026-10-02 challenge review merged duplicated controls into one owner each; withdrawn rows above say where their content went, and [threat-model.md](threat-model.md#control-ownership) lists every owner.

## Design guidance

### 1. Principals

A principal is anything the server authorises. A person is an **account**.
Listening history and restrictions belong to a **profile**. A **credential**
proves an account. A **device** is both a credential holder and, when it is
a shared household device, a principal in its own right.

| Principal | What it is | Authenticates with | Reach by default | First release |
|---|---|---|---|---|
| Owner | The person who claimed the server. Exactly one. | Passkey or OIDC; recovery codes; host recovery | Everything, including the owner-only capabilities | R1 |
| Administrator | A trusted person who manages people and libraries | Passkey or OIDC; administrator actions need elevation | The administrator preset; never owner-only capabilities | R1 |
| Member | An adult in the household | Passkey, OIDC, or (R2) a device key | Granted libraries; can appear on household devices | R1 |
| Managed profile | A child, or anyone who should not hold credentials | None. Reached from a household device or a guardian's account | Granted libraries through a content policy; no settings | R2 |
| Guest | A friend from outside the household | Passkey, OIDC or device key, enrolled through an invitation | Only the libraries named in the invitation | R1 |
| Device | A phone, TV or desktop app with a device key. Personal devices belong to one account; household devices belong to the household | Device key | Capped by device class and key protection level | R2 |
| API client | A script or third-party app | API key or app password | Declared scope within the creator's capabilities; never administrator | Later |
| Plugin | Code the owner installed | Runs under the plugin host with a manifest | Manifest capabilities; never identity capabilities | Later |
| Peer server | Another Gunmetal server | Pinned iroh endpoint ID approved by both owners | Named data flows only | Later |
| Anonymous link holder | Someone holding a public share link | The link secret | One object | Later |

### 2. Data model sketch

All IDs are random 128-bit values (record 1, decision 6). Identity tables
live in the durable store, not the rebuildable cache (SEC-IAM-004). That
needs an architecture record extending record 1, decision 5, as the
operations research already asks.

- **Account**: id, kind (owner, administrator, member, guest), status
  (active, disabled, pending deletion), display name, created.
- **Credential**: id, account id, type (passkey, OIDC link, device key, app
  password, API key), public material only (COSE public key, (issuer,
  subject), device public key, or a hash), created, last used, protection
  level, WebAuthn flags and counter.
- **Device**: id, class (personal, household), account id or household,
  credential id, self-reported name (always shown as unverified), capability
  ceiling, enrolled, last seen.
- **Profile**: id, owning account (or the household for managed profiles),
  guardians, content policy, optional PIN hash.
- **Session**: token hash, principal, device, profile, created, last seen,
  elevated-until, absolute expiry, source address.
- **Grant**: principal, scope (library, collection, playlist, item), rights
  (read, download, manage).
- **Invitation**: secret hash, preset, grants, expiry, uses left, creator.
- **Recovery code**: account, peppered hash, used at.
- **Security event**: sequence number, previous hash, own hash, fields from
  SEC-IAM-093.

### 3. Capabilities, presets and ceilings

Checks test capabilities, not roles (SEC-IAM-074). Roles are presets that
fill in capabilities when an account is created or changed.

| Capability | Owner | Administrator | Member | Managed | Guest |
|---|---|---|---|---|---|
| library.read (per library) | All | All | Granted | Granted, filtered | Named in invitation |
| library.download (per library) | All | All | Granted | Off | Off |
| library.manage (scan, edit metadata, attach lyrics or artwork) | Yes | Yes | Off | No | No |
| playlist.share (with other accounts on this server) | Yes | Yes | Yes | No | Off |
| invite.guest | Yes | Yes | Off | No | No |
| invite.member | Yes | Yes | No | No | No |
| user.manage (create, disable, delete and set grants for non-administrators) | Yes | Yes | No | No | No |
| user.recover (recovery links for members and guests) | Yes | Yes | No | No | No |
| session.manage (end other non-owner sessions) | Yes | Yes | No | No | No |
| household.device (enrol and remove household devices) | Yes | Yes | Off, can be granted to parents | No | No |
| household.profile (managed profiles, content policy, PINs) | Yes | Yes | Off, can be granted to parents | No | No |
| audit.read (all events) | Yes | Yes | No | No | No |
| server.settings (scanning, playback, non-security settings) | Yes | Yes | No | No | No |

Owner-only capabilities (SEC-IAM-075): **admin.manage** (create, demote or
recover administrators), **ownership.transfer**, **security.settings**
(origins, trusted proxies, session lifetimes, rate limits, adapters),
**oidc.configure**, and **plugin.approve**.

Effective capabilities are the intersection of four sets: the account's
capabilities, the device ceiling, the session state, and the token scope.

| Context | Ceiling |
|---|---|
| Browser session, not elevated | The account's capabilities minus every administrator and owner-only capability |
| Browser session, elevated (fresh passkey or provider check, SEC-IAM-036, 041) | The account's full capabilities |
| Household device (R2) | library.read for the chosen profile, plus profile switching; nothing else |
| Personal native device, hardware key with user verification (R2) | The account's capabilities; administrator actions need on-device biometric or passcode per action |
| Personal native device, software-only key (R2) | No administrator capability (SEC-IAM-049) |
| API key (Later) | Declared scope, never administrator (SEC-IAM-083) |
| Plugin (Later) | Its manifest, never identity (SEC-IAM-086) |

The Jellyfin lesson behind this table: its administrator flag grants every
power, and a delegated "upload subtitles" right was one route into
CVE-2026-35031, a CVSS 9.9 remote code execution flaw (per the users
research file). Capabilities that touch files must take content-addressed
uploads only, never names or paths.

### 4. First run: claiming a server

1. On first start with an empty data directory, the server generates its
   keys into the key file (SEC-IAM-100) and a claim code (SEC-IAM-007). It
   prints the claim URL or URLs and the code to the console. For release 2
   it also prints a terminal QR code for the native-app path. A
   `gunmetal claim-code` command prints the code again from the 0600 file,
   for containers and NAS packages whose console is hard to reach.
2. Until claimed, the server serves the claim page, its assets and a health
   check only (SEC-IAM-006). Scanning may start, but nothing is served.
3. The owner opens the claim page on a secure origin: the configured HTTPS
   name, or `http://localhost:<port>` on the host or through an SSH tunnel.
   The relying-party ID is fixed at this point. If the page is opened on
   localhost but a public name is configured, the page sends the owner to
   that name before enrolment, because a passkey made for "localhost" only
   works on localhost.
4. The owner enters the code. The server returns a WebAuthn registration
   challenge bound to this claim attempt. The passkey is created, and the
   code is consumed, the owner is created and the credential is stored in
   one transaction (SEC-IAM-009). Two simultaneous claims cannot both win.
5. The owner is offered recovery codes and is prompted to add a second
   passkey, for example a hardware key or a passkey in another ecosystem.
6. Setup continues as an elevated owner session. Every setup route is gone
   for good.

Do not accept a claim code from an environment variable or a compose file.
Example files get copied, and "changeme" becomes a default credential.

### 5. A secure context is a prerequisite

Browsers expose WebAuthn and the Web Crypto API only in secure contexts
(MDN). The WebAuthn relying-party ID must be a valid domain, not an IP
address (WebAuthn L3, §4). Since version 110, Chrome refuses WebAuthn on
pages with certificate errors, so a clicked-through self-signed certificate
does not work either (Nina Satragno, Google, on the W3C WebAuthn list,
November 2022). Plain HTTP also exposes session cookies to anyone on the
same Wi-Fi. The web client therefore needs HTTPS on a name, or localhost.

| Path | Who it suits | Cost | Central dependency | Status |
|---|---|---|---|---|
| Own domain, with TLS at a reverse proxy (Caddy, Traefik, nginx) or the server's own ACME client | Self-hosters with a domain | A domain and some setup | None | R1, documented |
| A tailnet HTTPS name, such as Tailscale's certificates for its own names | People already on a tailnet | Each device runs the tailnet client | The tailnet vendor | R1, documented |
| localhost | Single-machine installs; setup over an SSH tunnel | Passkeys valid only on localhost | None | R1, for setup and desktop installs |
| A project naming service: a per-server name under a project domain with a publicly trusted certificate, issued through ACME DNS-01, where the project's DNS accepts only updates signed by the server's own key (the plex.direct pattern) | Everyone, including non-experts | The project runs DNS and certificate automation. Some routers block DNS answers that point at private addresses (unverified detail). Certificate Transparency logs publish each name, so names must be opaque | Yes, but it holds no accounts and no user data, and servers must keep working without it | Owner decision (see Open decisions) |
| Self-signed certificate, or a private CA | Nobody | Browser warnings; Chrome refuses WebAuthn; installing a CA on every family device is unrealistic | None | Rejected |
| Plain HTTP on the LAN | Nobody | No WebAuthn, no Web Crypto, cookies readable on the LAN | None | Rejected for anything authenticated |

If the origin changes later, each user has to enrol a passkey on the new
origin. WebAuthn L3 related origins (§5.11) let one relying-party ID serve a
second origin, such as a LAN name next to a public name, when the owner
controls both. Native apps do not use WebAuthn (they use device keys), so a
change of origin does not affect them.

Do not host one web client for all servers at a project origin. A shared
origin means a shared relying-party ID. A malicious server could then ask a
user's browser to sign a challenge it had fetched from another server, and
origin binding would not stop it (threat T-IAM-34). It would also make one
static site a supply-chain target for every user, and it would need
credentialed CORS.

### 6. Sign-in methods

**Passkeys (browsers, R1).** Ask for discoverable credentials with user
verification required (SEC-IAM-020) and attestation "none". The server has
no need to know the authenticator model, and asking would leak it. Accept
synced passkeys: NIST SP 800-63B-4 Appendix B allows syncable
authenticators up to AAL2 and requires the verifier to inspect the UV flag.
Synced passkeys are also why most lost phones need no recovery at all.
Cross-device sign-in (scanning a QR code with a phone) covers shared
computers. Where browsers support them, use the L3 signal methods
(§5.1.10) to tell authenticators when a credential has been removed.
`webauthn-rs` is a candidate Rust library (maintenance status unverified);
whichever library is chosen, Gunmetal's own fixture tests pin its behaviour
(SEC-IAM-018).

**OIDC (R1).** The owner configures each provider: issuer URL, client ID,
the client secret (kept in the key file), scopes (`openid profile`), pinned
signing algorithms, and claim mapping (off by default). The server is the
confidential client. The flow is code plus PKCE plus state plus nonce, with
the issuer checked for mix-up (SEC-IAM-026, 027, 034). Accounts are keyed
by (issuer, subject) only (SEC-IAM-028). Do not fetch the picture claim.
Avatars are uploaded by users, or left as initials (SEC-IAM-032). In
release 2, native apps sign in through the system browser
(ASWebAuthenticationSession or Custom Tabs, as RFC 8252 describes). The app
starts the flow with its device public key, so the result is bound to that
key, and no token travels through a custom URL scheme. That avoids the
"mobile redirect override" pattern Immich needs. An OIDC-only account
cannot sign in while its provider is down, so every OIDC user should be
encouraged to add a local passkey. The owner must have one.
`openidconnect` is a candidate crate (unverified fit).

**Device keys (native apps, R2).** At enrolment the app generates a
non-exportable P-256 key in the Secure Enclave, Android Keystore (StrongBox
where present) or a TPM. Apple's Secure Enclave only holds P-256 keys, so
the device key is separate from the Ed25519 iroh endpoint key, which iroh
keeps in software. Secure Enclave availability on tvOS is unverified. Many
cheap Android TV boxes may only offer a software keystore (unverified); they
are allowed but capped below administrator (SEC-IAM-049). To authenticate,
the device signs a server nonce together with the context. Over iroh, that
context is a domain-separation label, the server's endpoint ID, the
client's endpoint ID and a TLS exporter value (RFC 8446 §7.5; iroh uses TLS
1.3 with raw public keys per RFC 7250). Over HTTPS it is a DPoP proof (RFC
9449) whose key thumbprint is bound to the access token. Either way, a
token copied off the device is useless without the key (SEC-IAM-050).

**Why there are no passwords (SEC-IAM-025).** Every rival that has
passwords has had to bolt on throttling, breach checks and two-factor
codes, and still lost accounts. A password fallback would become the
weakest path, and ASVS 6.1.3 and 6.3.4 require every path to be equally
strong. The cases people raise all have better answers: an old computer
uses cross-device passkeys from a phone; a TV pairs by QR; a family member
without a smartphone uses a hardware security key or a native app's device
key; a provider user uses OIDC; a legacy music app gets an app password
(Later). Because no password exists, there is nothing to stuff, phish,
reset by email or leak from a backup.

### 7. TVs and other devices without keyboards (R2)

**The TV is on the same network as the server.**

1. The TV generates its device key, finds the server by mDNS (which is
   unauthenticated) or a typed address, and pins whatever server identity
   key it reached.
2. The TV requests a device authorisation and sends its public key. It
   gets a device_code (256 bits, bound to the key), a user code (8
   characters, base-20) and an approval URL (SEC-IAM-055, 056). It then
   polls with signed requests at the interval the server sets.
3. The TV shows a large QR code and the code beneath it. The QR code holds
   the approval URL, the user code and the identity key of the server the
   TV is talking to (SEC-IAM-057).
4. The user scans the QR code with the Gunmetal app, or with the phone's
   camera, which opens the approval page on the server's own HTTPS origin
   (SEC-IAM-057). If the server identity in the QR code differs from the
   one the app has pinned, an impostor is in the middle, and the app
   refuses; in a browser, the passkey's origin binding does the same job.
5. The phone shows what will be granted (SEC-IAM-058): "Living-room TV
   (name not verified), in this home, thirty seconds ago, will
   become a household device showing the profiles Ana, Ben and Kids". The
   user confirms with a biometric. Administrator powers can never be
   granted this way (SEC-IAM-059).

**The TV is somewhere else, such as a grandparent's house.** A QR image
can be sent by anyone ("scan this to fix grandma's TV"), so a remote
enrolment never relies on a scan alone. The user types the code shown on
the TV into the phone and confirms a matching code shown on both screens
(SEC-IAM-060). The phone then gives the TV the server's ticket and a
single-use enrolment invitation bound to that TV's public key, which the TV
presents to the server, signed with its key. A remote enrolment grants one
named profile at most, never household class, and every adult in the
household is alerted (SEC-IAM-059).

**Against device-code phishing (RFC 10027; Microsoft's Storm-2372 report).**
Codes last 10 minutes and are single use. The approval screen describes
the device and says plainly that this is a device joining the account. If
the two devices seem to be on different networks, the user must type the
code; a link will not do (SEC-IAM-060). Every new device triggers an alert
on the account's other devices (SEC-IAM-098). Approval needs a fresh
biometric. Administrator grants are never available through any pairing
flow, and household grants never through a remote or typed-code enrolment. Microsoft's advice after Storm-2372 was to block the device-code
flow wherever possible. In Gunmetal it is limited to keyboardless devices.
Phones and computers use passkeys or the QR handoff.

**Smart TVs on the web build (Samsung, LG).** These hold a non-extractable
Web Crypto key, which is software-backed and therefore capped like any
software key. Whether a packaged TV web app counts as a secure context on
each platform is unverified and must be tested before those builds ship
(Later, SEC-CLI-070).

### 8. Sessions and tokens

| Token | Held by | Form | Lifetime | Bound to | Kept by the server as | Revocation | If stolen |
|---|---|---|---|---|---|---|---|
| Browser session | Browser cookie | Opaque, 256 bits | 7 days idle, 30 days absolute | Nothing in R1 (DBSC later) | SHA-256 hash | Next request | Non-administrator use of that account until revoked or expired |
| Admin session | Its own `__Host-` cookie, SameSite=Strict | Opaque, 256 bits | 15 minutes idle, 1 hour absolute | The media session and a user-verified sign-in | SHA-256 hash | Next request | Administrator use for up to 15 idle minutes, never fresh-uv actions |
| Device enrolment (R2) | Native app | Non-exportable key pair | Until revoked; household devices dormant after 30 days unused (SEC-IAM-109) | The device's hardware | The public key | Next request | Cannot leave hardware-backed storage; a software key could be copied from a rooted device, which is why such devices are capped |
| Device access token (R2) | Native app | Opaque, 256 bits | 10 minutes | The device key, by proof of possession | Hash | Next request | Nothing without the key |
| Stream URL | Every media consumer, the web client included (SEC-API-029) | HMAC capability | Item duration plus 10 minutes, 4 hours at most (SEC-API-027) | Stream grant, item, rendition | Key outside the database | Next range request after the session or grant ends | One item's bytes, only while the session lives |
| Pairing user code (R2) | A person | 8 characters, base-20 | 10 minutes, once | device_code and device key | Hash | Expiry | Useful only if the victim approves it (phishing; see section 7) |
| Invitation | A link | 128 bits, in the URL fragment | 7 days, 1 use (defaults) | Preset and grants | Hash | Revoke | Join as a guest with the preset; the inviter sees who joined |
| Recovery code | The user, on paper | 80 bits | Until used | Account | Peppered hash | Regenerate | Enrol one credential; every device is alerted |
| Claim code | The host console | 128 bits | 24 hours | Unclaimed server | Hash | Single use | Claim the server, but only by someone who can see the host console or file |
| API key or app password (R2) | Script or app | 128 bits or more | 1 year by default | Scope | Hash (encrypted for legacy Subsonic) | Next request | Scoped actions only, never administrator |
| Offline grant (R2) | Device | Signed record | 30 days at most by default | Device key and item list | Record | Next contact | The holder of that device plays its downloads. The files themselves are not protected; this is honest policy, not DRM |

How revocation is fast. The server is one process with one database, so
there is no reason for stateless tokens. Session lookups go through an
in-memory cache keyed by token hash, and revocation events invalidate it.
The byte-serving loop checks a revocation counter between chunks, and
WebSockets subscribe to revocation events (SEC-IAM-043). Jellyfin needed a
12.1 fix because revoking a device left its open sessions running (per the
users research file). That is the regression test.

Stream URLs. Every media consumer, the web client included, fetches media
by capability URL and never by cookie (SEC-API-029), so revocation works the
same way everywhere and media routes need no cross-site request defences. The URL carries a
random stream-grant ID, not the session token. The signature is HMAC-SHA256
over the method, path, expiry and grant ID. The previous key stays valid
for verification for 12 hours after rotation. Every request re-checks the
grant and the session (SEC-IAM-046); a signature alone is never enough.

Lifetimes, justified as ASVS 7.1.1 asks. NIST SP 800-63B-4 §2 says AAL1
reauthentication SHOULD be within 30 days, and AAL2 within 24 hours with
inactivity of at most 1 hour. Playing music is AAL1 risk, so browser
sessions last 30 days. Administrator actions are the AAL2 surface, so
elevation lapses after 15 idle minutes, which is stricter than NIST.
Household TV enrolments last until revoked and go dormant after 30 days
unused (SEC-IAM-109). That goes beyond the 30-day guideline, and is justified because a household device
represents no single person, renews itself with its own key every 10
minutes, and is capped below every administrator and credential
capability.

Later: Device Bound Session Credentials (DBSC). Chrome 146 made it
generally available on Windows on 10 April 2026, with macOS to follow
(Help Net Security). DBSC binds browser cookies to a hardware key, which
would close most of T-IAM-07 for browsers. Add it behind feature detection
once other browsers ship it, and never depend on it.

### 9. Households, profiles, PINs and quick switching (R2)

- **One household per server.** Members join it by default; guests never
  do.
- **Household devices** are enrolled by someone with household.device. The
  device shows a picker of the household profiles enabled for it
  (SEC-IAM-061). Choosing a profile issues a new session token
  (SEC-IAM-038) with the profile's capabilities, intersected with the
  household ceiling.
- **PINs are a lock on switching, not a credential** (SEC-IAM-063). They
  are 4 to 8 digits, optional, checked by the server and throttled
  (SEC-IAM-062). NIST's minimum for an activation secret is 4 characters,
  with 6 preferred (§3.2.10). Gunmetal does not use the PIN as an
  authenticator, only as a speed bump, so 4 digits is acceptable when
  combined with server-side throttling. When an adult's profile is added to
  a household device, the setup offers a PIN with one tap. Plex's own
  support page warns that without a PIN on the admin's account, "a user
  could switch to you and edit server settings". In Gunmetal that cannot
  happen, because household devices never carry administrator
  capabilities.
- **Kids mode.** A household device can be locked to a managed profile. It
  leaves that profile only with an adult's PIN or an approval from a
  guardian's phone (SEC-IAM-066).
- **Several accounts on one personal device.** Each account enrols its own
  device key, and switching between them uses the operating system's
  biometric check for that key.
- **The sync payload is filtered on the server, per profile.** Record 1
  syncs the library to the device, so a client-side filter would put the
  whole catalogue on a child's TV. Household devices do not keep
  PIN-protected profiles' data at rest (SEC-IAM-065).
- **Content policy, in order.** A library grant is required first. Content
  filters then apply: rating ceiling per rating system, unrated handling
  per media type, explicit-content filter, tag rules. Last, per-item
  exceptions set by a guardian override those filters, because they name
  one item deliberately. All of this lives inside the visibility predicate
  (SEC-IAM-064). Defaults for managed profiles: unrated video is hidden;
  music flagged explicit is hidden; unflagged music is shown, since most
  music carries no flag, with a strict mode available. Which tags carry the
  explicit flag is a sibling research question.

### 10. The authorisation engine

- **One function.** `decide(principal, ceilings, action, resource, context)
  -> Decision` lives in `gunmetal-core`. It is pure and total, and matches
  exhaustively on closed enums, so adding an action fails to compile until
  someone decides it (SEC-IAM-068). The context carries time, transport and
  elevation, and transport can only narrow (SEC-IAM-013). Errors become
  denials (SEC-IAM-069).
- **One visibility predicate.** `Visibility::for_principal(..)` compiles to
  a SQL condition. Every repository function that reads or writes user
  data takes a `&Visibility` argument, so handlers cannot call one without
  it. The scanner gets a separate `SystemScope` type that request handlers
  cannot import, enforced by a CI check (SEC-IAM-070). This is the
  structural fix for the class of bug in Immich's /search/random bypass
  (reported in July 2026) and in CVE-2026-82272, where locked assets stayed
  reachable through shared albums and links. It is also the fix for the
  folder-view hole Plex closed by removing folder view for restricted
  users.
- **One route table.** Each route registers its policy:
  `Public(reason)`, or `Authenticated(action, resource extractor)`. A test
  enumerates the table, fails on any route without a policy, and checks
  every `Public` route against a reviewed allowlist file (SEC-IAM-067).
- **Cross-principal suite.** Fixtures create two of each principal kind,
  each owning objects of every kind. For every route and object kind, the
  suite makes a request as one principal with the other's object and
  expects denial. Return 404 for objects the caller cannot see, so
  existence does not leak, and 403 for visible objects where the action is
  forbidden (SEC-IAM-071). Navidrome's September 2026 advisories (share
  creation trusting a client-supplied user ID, share endpoints without
  ownership checks, only the first of several resource IDs validated) are
  the regression cases.
- **No escalation.** Every create and update that hands out a capability
  or grant checks that the result is a subset of what the caller holds
  (SEC-IAM-073). The property test generates random callers and requests.
  Immich CVE-2026-23896 was exactly this check, present on create and
  missing on update.
- **Field-level.** Use per-action request types that reject unknown fields
  (SEC-IAM-072), and per-audience response types (self, administrator,
  other user), so no response can include a field it should not.

### 11. Sharing

- **A friend with no server** gets a guest account through an invitation
  (SEC-IAM-078 to 080). In release 1 they can reach the server only if the
  owner exposes HTTPS. From release 2 the native apps connect over iroh
  with no open ports.
- **A friend with their own Gunmetal server** gets exactly the same: a
  guest account here, with their own credentials. Their app holds both
  servers, with a separate key for each, and merges them in the interface.
  Their server takes no part, so its administrator cannot impersonate them
  here (SEC-IAM-081).
- **Federation (Later, needs its own record).** If it is ever built, trust
  must be per pair of servers and approved by both owners, scoped to named
  remote users and named grants, with each assertion signed by the peer's
  endpoint key and bound to the request. The record must say plainly that
  the peer's administrator could then act as those named users.
- **Invitation links.** The secret sits in the URL fragment, so it never
  appears in a request line or a server log. The landing page on the
  owner's server reads the fragment in script and posts it. If release 2
  uses a static page on gunmetal.tv to open the app, that page must be
  static, with no third-party script and a strict CSP. A compromised page
  could steal unredeemed invitations, so the inviter is told when an
  invitation is redeemed and by which device.
- **Public links (Later)** follow SEC-IAM-082. They are off for video until
  the owner decides otherwise, and chat-app preview cards are off by
  default.

### 12. API clients, adapters, plugins and peer servers (Later)

- **API keys and app passwords** are listed with devices, scoped, expiring
  and never administrator (SEC-IAM-083). A request carrying two kinds of
  credential is rejected (SEC-IAM-002), the same rule as OpenSubsonic's
  "conflicting authentication" error.
- **Adapters** follow SEC-IAM-084 and 085. The OpenSubsonic API key
  extension tells servers to deprecate token-and-salt sign-in, and Gunmetal
  should support only the API key by default. Navidrome keeps reversible
  passwords for token-and-salt, encrypted by default with a shared key that
  its own docs call obfuscation.
- **Plugins** get a manifest the owner approves, never identity
  capabilities, and the originating user's permissions when acting for
  someone (SEC-IAM-086, 087). Authentication providers as plugins
  (directory services and so on) are rejected; see Options rejected.
- **Peer servers** are pinned endpoint IDs with named flows, such as
  replicating one person's history between their own two servers
  (SEC-IAM-088).

### 13. Recovery

Recovery is a ladder. Each rung is used only when the one above it has
failed.

1. **Another credential.** Synced passkeys survive the loss of a phone. The
   interface nags while an account has only one credential, and never
   blocks.
2. **Recovery codes** (SEC-IAM-089, 090). There are ten, printable as a
   sheet, offered at enrolment to owners and administrators. A code opens a
   session that can do nothing except enrol a new credential, which then
   starts a 72-hour recovery hold (SEC-IAM-106). Every device is alerted and
   can cancel it, and the user is shown all existing credentials to prune.
3. **The identity provider**, for OIDC accounts.
4. **An administrator's recovery link** for members and guests, or the
   owner's for administrators (SEC-IAM-091). Inside a household, "the
   administrator knows the person" is real identity proofing (ASVS 6.4.4).
   An administrator can never recover the owner's account, which would
   otherwise be a route from administrator to owner. The link is redeemed
   in person or on a device the account holder already approved, and the
   recovery hold keeps the account's history hidden from the new credential
   until the hold ends, so a curious administrator cannot use recovery to
   read a member's history (SEC-IAM-091, SEC-IAM-106).
5. **Host recovery for the owner** (SEC-IAM-092). Running
   `gunmetal recover-owner` on the host talks to the server over a Unix
   socket (or a named pipe with an ACL on Windows) that only the service
   user can open. It prints an enrolment link valid for 15 minutes, and
   alerts every administrator. Access to the host already means control of
   the server, so this adds no new power.

Never: emailed reset links, SMS, security questions or a vendor support
desk (SEC-IAM-025). There is no vendor in the loop, and email would make
the owner's mailbox a key to the server.

### 14. Audit trail and alerts

- **A separate security log**, not part of the watch-history log, so each
  can have its own retention and privacy rules. It sits in the durable
  store (SEC-IAM-004) and is included in encrypted backups (SEC-IAM-105).
- **Event fields** (SEC-IAM-093): sequence number, UTC time, event type,
  principal, device, profile, source address (from trusted proxies only,
  SEC-IAM-012), target, outcome, denial reason, previous hash and own hash.
- **Hash chain** (SEC-IAM-094). Each entry's hash covers the canonical
  encoding of the entry and the previous entry's hash. A chain cannot show
  that its tail was cut off, so the head hash is also written to the
  ordinary log and into each backup as an anchor.
  `gunmetal audit verify` checks the chain.
- **Who reads it** (SEC-IAM-097): users their own events, guardians their
  managed profiles', holders of audit.read everything. An administrator's
  access to a user's data appears in that user's own feed (SEC-IAM-077).
- **Alerts** (SEC-IAM-098) are in-app notifications on all of the
  account's devices. Push and email can come later through plugins.
- **fail2ban** (SEC-IAM-099): one documented line per failure, for people
  who already run it.
- **Secrets** never reach a log (SEC-IAM-095). A wrapper type with a
  redacting Debug and no Display enforces this at compile time. A canary
  test plants known secret values into the integration suite and fails if
  any log output contains them.

### 15. Staying usable

| Control | How it stays usable |
|---|---|
| Passkeys only | Sign-in is face or fingerprint unlock. There is no password to forget, and synced passkeys follow the user to a new phone |
| Claim code | Shown on the console and by `gunmetal claim-code`; entered once |
| HTTPS required | The first-run page explains the paths, and the docs give copy-paste recipes for a reverse proxy and a tailnet |
| TV pairing | Scan a QR code and confirm with a fingerprint; under 30 seconds. Nobody types on a remote |
| Household TV | Never signs out while it is used at least once every 90 days; children tap their own picture |
| PIN | Optional, offered with one tap, 4 digits allowed |
| Administrator elevation | Asked for only on administrator screens, once per 15 minutes of use |
| 30-day browser sessions | Most people sign in about once a month per browser |
| Recovery | Printable codes, and a household administrator can re-invite anyone except the owner |
| Alerts | Only for security-relevant events, never for routine playback |

### 16. How the requirements become tests

- **Route-table test** (CI): policy coverage, the public allowlist, CSRF
  and HTTP checks on every mutating route, header checks on every HTML
  route, throttling on every secret-checking route.
- **Cross-principal suite**: generated from the route table, run against a
  real SQLite database in the gate.
- **Visibility conformance suite**: for each principal fixture, compare the
  set of item IDs reachable through every route with a reference computed
  by a deliberately simple, separate test-only function (CONTRIBUTING rule
  4).
- **Policy property tests** in the core crate: no escalation, network
  location never widens access, exactly one owner, ceilings respected,
  PINs never authorise anything else. The module is under the
  zero-surviving-mutant gate.
- **WebAuthn fixtures**: ceremonies recorded from real platform and
  hardware authenticators plus a software authenticator, each field
  tampered in turn.
- **Hostile test provider**: an OIDC provider we write that misbehaves on
  purpose, alongside a real provider (for example Dex or Authelia,
  unverified fit) in a container.
- **Fuzz targets**: the WebAuthn CBOR and COSE path, JWS parsing,
  invitation and QR payload decoding, cookie and header parsing, and
  security-log verification.
- **Injected clock** everywhere a lifetime is set.
- **Log-canary scan** over the full integration suite.
- **Browser end-to-end tests** for cookies, storage and the session list.
  **Device tests** for native key storage (R2).
- **Manual review**: a threat-model review of this document against the
  built system before each release, with findings recorded as new tests.

### 17. How the rivals do it

| Topic | Plex | Jellyfin | Emby | Navidrome | Immich | Gunmetal |
|---|---|---|---|---|---|---|
| Where identity lives | Central plex.tv account | Local to each server | Local, with optional Emby Connect | Local | Local | Local; no central account |
| First-run ownership | Claimed with a plex.tv sign-in, or a short-lived claim token for headless installs | LAN setup wizard; 12.0 fixed a wizard that could be re-run | Setup wizard | First user created becomes administrator, including through proxy sign-in | Welcome screen; first user is administrator | Console claim code, one transaction, setup gone afterwards |
| Main sign-in | Password at plex.tv, social sign-in, optional two-factor; breached in 2022 and 2025 | Password; two-factor not built in; the main SSO plugin was archived on 2026-05-12 | Password; optional passwordless sign-in for "local" clients led to CVE-2023-33193 | Password; reverse-proxy header sign-in from trusted sources | Password or built-in OIDC (auto-registration on by default) | Passkeys, device keys, OIDC; no passwords |
| TVs | Code linking (detail unverified) | Quick Connect: 6-character code, on by default, signs the device in as the approving user | PIN through Emby Connect | Quick Connect for its Jellyfin-compatible API | Not applicable | QR plus RFC 8628 profile, bound to the device key, server identity checked by the phone, never administrator |
| Tokens | plex.tv tokens (lifetime unverified) | Tokens stay valid until sign-out or revocation (unverified) | Not checked | Session tokens; Subsonic token-and-salt with a reversibly stored password | Sessions plus scoped API keys | Opaque server-side sessions; proof-of-possession device tokens; no refresh tokens |
| Household profiles | Plex Home, managed users without credentials, optional switch PIN | None (requests open) | Not checked | None | A PIN-elevated session for a locked folder, bypassed twice in 2026 | Household devices, managed profiles, PIN as a switch lock only |
| Authorisation | Per-library shares, rating presets | One administrator flag plus many fine switches; 2026 advisories for IDOR and session control | Per-library, parental ratings | Per-library since 0.58; 19 advisories in September 2026 | Granular API key permissions; key self-escalation (CVE-2026-23896) | Capabilities, one pure policy function, one visibility predicate, generated cross-principal tests |
| Friends | Through plex.tv; remote video now needs a paid pass | Make them an account; Wizarr fills the invitation gap | Emby Connect linking | Accounts, public shares | Partner sharing, shared links | Invitations giving each friend their own credentials; no server-to-server trust |
| Recovery | Email reset at plex.tv | File-based reset on the host (unverified) | Not checked | Administrator resets | Administrator-issued temporary password | Passkeys, then codes, then re-invitation, then host command; never email |
| Audit | Device list and sign-out-everywhere at plex.tv | Activity log | Not checked | Not checked | Not checked | Hash-chained security log; users see their own events; alerts |

Sources for this table are in the users research file and in Sources
below. Immich is the reference for built-in OIDC and scoped keys, and its
2026 advisories form the OIDC test checklist. Plex is the reference for
household profiles and easy friend sharing, which Gunmetal matches without
a central account.

### 18. Recommended model (draft architecture record)

> **Record N: Identity, accounts and access.** Status: proposed.
>
> **Context.** Record 1 commits to no central account, passkeys and OIDC,
> device-bound keys, random IDs, per-object authorisation and short-lived
> signed stream URLs. Self-hosted media servers are exposed to the internet
> by non-experts, and rivals' incidents cluster in sign-in, delegation and
> forgotten authorisation checks.
>
> **Decisions.**
> 1. Principals are the owner (exactly one), administrators, members,
>    managed profiles, guests, devices, API clients, plugins, peer servers
>    and anonymous link holders. Credentials, accounts and profiles are
>    separate entities.
> 2. There are no account passwords. Browsers use passkeys bound to a
>    relying-party ID fixed at setup; native apps use non-exportable device
>    keys; OIDC is optional, keyed by (issuer, subject), with
>    auto-registration and administrator claim mapping off by default.
> 3. A server is claimed with a single-use 128-bit code from its console.
>    There is no default credential, and setup cannot run twice.
> 4. Authenticated web traffic needs a secure context (HTTPS on a name, or
>    localhost). Forwarding headers are ignored unless a trusted proxy is
>    configured, and network location never widens access.
> 5. Sessions are opaque, server-side and revocable on the next request.
>    Browser sessions last 7 days idle and 30 days absolute; administrator
>    elevation needs a fresh check and lapses after 15 idle minutes. Native
>    tokens last 10 minutes and are bound to the device key, with no
>    refresh tokens. Stream URLs are re-checked on every request.
> 6. Keyboardless devices pair through an RFC 8628 profile: the device code
>    is bound to the device key, the QR code carries the server identity,
>    approval needs a fresh biometric, and the flow never grants
>    administrator capabilities.
> 7. Authorisation is capability-based and deny by default, in one pure
>    function in the core crate. Every route declares a policy, all data
>    access goes through one visibility predicate, and no principal can
>    grant more than it holds.
> 8. Households have household devices with profile pickers; PINs only
>    gate switching; managed profiles hold no credentials.
> 9. Friends get their own credentials through invitations. No server
>    trusts another server's assertions about users until a federation
>    record says otherwise.
> 10. Recovery goes: another credential, recovery codes, provider,
>     administrator re-invitation, then host-only recovery for the owner.
>     There is no email, SMS or security-question recovery.
> 11. Security events go to a hash-chained, append-only log, separate from
>     watch history, readable by each user for their own account.
> 12. Identity data is durable state, not cache.
>
> **Consequences.** The web client needs HTTPS on a name, which the
> project must make easy (see the naming-service decision). Families with
> children get managed profiles in release 2, not release 1. Legacy
> clients wait for app passwords. Every route ships with generated
> cross-principal tests, and the policy module carries the full mutation
> gate. A record on identity-data durability, and one on secure-context
> strategy, are needed alongside this one.

### 19. Options rejected

| Option | Why it was rejected |
|---|---|
| A central Gunmetal account (the plex.tv model) | A single breach target (plex.tv in 2022 and 2025); outages lock people out of their own server; contradicts record 1 |
| Passwords as the main method, or as a fallback | Stuffing, reuse and phishing; the fallback becomes the weakest path (ASVS 6.1.3); plain-HTTP LAN setups would send them in the clear |
| Two-factor codes layered on passwords | Still phishable, and it doubles recovery work; passkeys are phishing-resistant from the start |
| Passwordless sign-in for a "trusted local network" | The root of Emby CVE-2023-33193 (CVSS 9.1) through forged proxy headers |
| Built-in LDAP or reverse-proxy header authentication | Navidrome's own docs warn that a proxy which passes client-set headers lets anyone impersonate anyone; Gunmetal users can put an OIDC provider in front of their directory instead |
| Stateless JWT sessions for first-party clients | Revocation lags until expiry, and algorithm confusion is a whole class of bug; one server with one database does not need them |
| Long-lived refresh tokens on native apps | A bearer secret worth stealing; renewal by device-key signature is strictly better |
| One web client hosted at a project origin for all servers | A shared relying-party ID allows cross-server assertion relay; one static site becomes everyone's supply-chain risk; needs credentialed CORS |
| Self-signed certificates with a click-through | Chrome refuses WebAuthn on certificate errors since version 110, and it trains people to ignore warnings |
| Quick Connect as Jellyfin does it | It does not show what is being granted, and the code is not bound to the requesting device's key |
| Children as full accounts with passwords | What Jellyfin does today; children end up knowing a parent's password, and passwords exist again |
| Server federation trusting a peer's assertions now | The peer's administrator could impersonate its users here; deferred to its own record, not rejected for ever |
| Email-based recovery or magic links | Makes a mailbox the key to the server, needs SMTP, and ASVS 6.3.6 prohibits email as an authentication mechanism at level 3 |
| A single "administrator" switch with every power | Jellyfin's model; capabilities with owner-only powers and ceilings instead |
| Authentication providers as plugins | A plugin would sit inside the trust boundary of every sign-in; OIDC already covers directories |
| A general relationship-based permission engine (per-item lists everywhere) | Far larger than the need; library grants, item shares and content filters fit in one predicate that is easy to prove |

## Anti-patterns

- **A default password, or "first person to the setup page wins".** The UK
  PSTI regime (in force 29 April 2024) bans universal default passwords on
  consumer connectable products. Jellyfin 12.0 fixed a re-runnable setup
  wizard. Navidrome makes the first user an administrator, including users
  created through proxy sign-in.
- **Trusting forwarding headers or "local" networks.** Emby CVE-2023-33193
  let forged proxy headers pass remote attackers as local on servers that
  allowed passwordless local sign-in (fixed in 4.7.12). Navidrome
  GHSA-f295-6wp9-qqfg (September 2026) let the same headers bypass its
  login rate limit.
- **Matching identity-provider users by email.** nOAuth (Descope, 2023)
  showed that a mutable, unverified email claim enables account takeover.
  Descope's advice is that it should "never be trusted or used as an
  identifier".
- **Redirecting to a return URL taken from the request.** Immich
  CVE-2026-53662 (Critical, 2026-06-01, affecting main-branch builds only)
  passed an unchecked `continue` parameter to a redirect, which allowed
  script execution and account takeover. The first fix was incomplete
  (GHSA-qp2h-w794-2vhf).
- **Turning off TLS checks for provider calls, or fetching URLs from
  claims.** Immich GHSA-hfvf-5c8x-8rc4 (2026-07-06): OIDC discovery, token,
  userinfo and JWKS fetches ran without certificate verification. The users
  research file also records an SSRF through Immich's OAuth profile-picture
  URL.
- **Auto-registration on by default.** Immich's OIDC auto-registration
  defaults to on, and its own docs advise turning it off so that anyone
  with an account at the provider cannot register.
- **An update path that checks less than the create path.** Immich
  CVE-2026-23896 (2026-01-29, fixed in 2.5.0): a limited API key could give
  itself every permission through the update endpoint.
- **Trusting identifiers from the client.** Navidrome GHSA-82gh-4ggp-gfg5
  (High): share creation trusted a client-supplied user ID.
  GHSA-3rwv-f797-f9p3 checked only the first of several resource IDs.
  GHSA-3g4p-jhv2-xrxf had no ownership checks on share endpoints.
  GHSA-37h4-53gj-cw8m let any user take over another user's player
  records.
- **Enforcing a restriction per endpoint instead of in one place.** Immich
  CVE-2026-82272 (VulnCheck, 2026-08-28): locked assets stayed visible
  through shared albums and links. A separate July 2026 report showed POST
  /search/random ignoring the PIN-elevated session. Plex removes folder
  view for restricted users because folder names cannot be filtered.
- **Checking a signed URL only when it is issued.** Navidrome
  GHSA-wp9c-pw66-c6j2: public share stream URLs kept working after the
  share expired or was deleted.
- **Revoking a device without ending its open sessions.** Jellyfin fixed
  this in 12.1 (per the users research file).
- **Unauthenticated callback endpoints.** Navidrome GHSA-8jrh-w926-8rvw: an
  unauthenticated IDOR in the Last.fm link callback allowed scrobble
  session hijack.
- **Adapters with weaker sign-in than the native API.** Navidrome
  GHSA-p994-r776-mw52 (High, 2026-09-21): its Subsonic API allowed
  unauthenticated password brute force.
- **Reversible password storage under a shared key.** Navidrome encrypts
  stored passwords with a default shared key, so that Subsonic
  token-and-salt can work.
- **Secrets in the database, or in logs.** Per the users and operations
  research files: Navidrome stored its JWT secret in plain text in the
  database (December 2024), and logged administrator passwords when setup
  failed (fixed in 0.64.2).
- **A central password store.** plex.tv was breached in 2022 and again in
  August 2025, and every user had to reset their password.
- **Profile switching that reaches administration.** Plex advises admins to
  set a PIN because, without one, another household member can switch into
  the admin account and change server settings.
- **A device-code flow open to anyone, for anything.** Microsoft's report
  on Storm-2372 (13 February 2025) describes phishing that tricked users
  into approving attacker-started device codes, and recommends blocking the
  flow wherever it is not needed.
- **Failing open.** OWASP Top 10:2025 added A10, Mishandling of Exceptional
  Conditions. An error inside authentication must deny (SEC-IAM-069).
- **Assuming administrator-only flaws do not matter.** Plex CVE-2020-5741,
  an authenticated remote code execution flaw, was added to CISA's Known
  Exploited Vulnerabilities catalogue in March 2023 (per the users research
  file). Keep administrator surfaces small and behind elevation.

## Open decisions for the project owner

> **Status, 2026-10-02.** The owner decisions from every file in
> `docs/security/` are consolidated and de-duplicated in
> [README.md](README.md#open-decisions-for-the-owner). Where the baseline
> has since chosen, or where a recommendation below disagrees with the
> README, the README and the requirement tables win.

1. **Should the project run a naming and certificate service so browsers
   get HTTPS without a domain?** Updated recommendation (2026-10-02): yes,
   decided before release 1, because release 1 is web-only and
   passkey-only. It is the install-time default the first-run output
   presents, with own domain, tailnet and localhost as tested alternatives
   (SEC-NET-010 to SEC-NET-013, SEC-NET-069 to SEC-NET-072, SEC-OPS-007). Trade-off: passkeys work
   for everyone, but the project then runs DNS and certificate automation,
   pays for it, and becomes a soft central dependency that sees which
   servers exist. It must hold no accounts, and servers must work fully
   without it. Without it, release 1's web client is effectively for
   people who can set up HTTPS.
   *Answered (D-07, 2026-10-02):* the name service, its naming client and
   its Certificate Transparency monitoring are R2. R1 gets HTTPS through
   the owner's own domain with automatic certificates, a tailnet, or
   localhost on the same machine (SEC-NET-013).
2. **No account passwords at all?** Recommendation: yes. Trade-off: people
   with no passkey-capable device need cross-device sign-in, a hardware
   key, a native app or OIDC. Legacy apps wait for app passwords. If the
   owner wants a password option anyway, it should be off by default,
   15 characters minimum when it is the only factor (NIST §3.1.1.2), checked
   against a breach list (ASVS 6.2.12), throttled, and never allowed for
   the owner or administrators.
3. **OIDC in release 1 or release 2?** Recommendation: release 1, because
   record 1 names it and self-hosters often already run a provider.
   Trade-off: 11 more release-1 requirements and a real attack surface
   (in 2026 Immich published advisories for its sign-in redirect and its
   OIDC provider calls, and the users research file records an OAuth SSRF).
   Deferring it shrinks
   release 1.
4. **Managed profiles and parental filters in release 1?** Recommendation:
   release 2, alongside household devices. In release 1, a child can use a
   separate account limited to chosen libraries. Trade-off: no
   explicit-lyrics filter for the first music release.
5. **What can administrators see of other users' listening?**
   Recommendation: live sessions (needed for bandwidth) and per-user
   totals; full history only for guardians of managed profiles, or when
   the user opts in. Shown honestly on the "what your admin can see" page
   (SEC-IAM-104), which also says that anyone with host access can read
   everything. Trade-off: owners used to Tautulli-style dashboards will
   want more.
6. **Security-log and IP retention.** Recommendation: keep events for one
   year, and coarsen source addresses (IPv4 /24, IPv6 /48) after 30 days.
   Trade-off: forensic depth against the privacy of household members.
7. **Device inactivity expiry.** Recommendation: 90 days for household and
   personal devices. Trade-off: a TV used only at Christmas has to pair
   again.
8. **Software-only device keys.** Recommendation: allowed, capped below
   administrator. Trade-off: cheap Android TV boxes work, but nobody can
   administer from them.
9. **Browser session lifetime.** Recommendation: 7 days idle, 30 days
   absolute, with 15-minute administrator elevation. Trade-off: longer than
   NIST's AAL2 guidance for the whole session, justified because
   administrator actions are separately elevated.
10. **Federation between servers.** Recommendation: not before its own
    record. Use per-server invitations and multi-server clients instead.
    Trade-off: a friend accepts one invitation per server rather than
    signing in once.
11. **Directory (LDAP) or reverse-proxy header sign-in.** Recommendation:
    never built in; point people to an OIDC provider that fronts their
    directory. Trade-off: some homelab users want direct LDAP.
12. **Recovery codes mandatory for the owner?** Recommendation: strongly
    prompted but skippable, because host recovery exists. Trade-off: an
    owner whose server has no reachable console (some appliance
    installs) depends on codes or a second passkey.
13. **Default device and stream limits** (SEC-IAM-102). Recommendation: 25
    devices per member and 5 per guest, 3 concurrent streams per guest,
    and no stream limit for household members. Trade-off: generosity
    against account sharing.
14. **Public share links for video.** Recommendation: music first, video
    only after its own decision on bandwidth and exposure. Trade-off: a
    much-requested feature (Jellyfin request #72) waits.

## Sources

Project documents (read first):

- README.md, CONTRIBUTING.md, AGENTS.md, SECURITY.md
- docs/adr/0001-architecture.md, docs/adr/0002-music-is-first-class.md
- docs/research/users-sharing-and-security.md (rival inventory and
  incident log; claims attributed to it are as verified there)
- docs/research/setup-migration-and-operations.md (first-run and secure
  context)
- docs/research/clients-platforms-and-offline.md (offline grants, TV
  sign-in)

Standards:

- OWASP ASVS 5.0 source chapters V2, V3, V4, V6, V7, V8, V9, V10, V11, V12,
  V13, V14, V15, V16:
  https://github.com/OWASP/ASVS/tree/master/5.0/en
- OWASP Top 10:2025: https://top10.owasp.org/2025
- OWASP API Security Top 10 2023:
  https://api-security.owasp.org/editions/2023/en/0x11-t10
- OWASP MASVS: https://mas.owasp.org/MASVS/ (control IDs; version 2.1 per
  https://opensecurityarchitecture.org/frameworks/owasp-masvs-v2,
  unverified)
- NIST SP 800-63B-4 (final, July 2025):
  https://csrc.nist.gov/pubs/sp/800/63/b/4/final ,
  https://pages.nist.gov/800-63-4/sp800-63b/aal ,
  https://pages.nist.gov/800-63-4/sp800-63b/authenticators ,
  https://pages.nist.gov/800-63-4/sp800-63b/syncable ,
  https://pages.nist.gov/800-63-4/sp800-63b/passwords
- W3C Web Authentication Level 3 (Recommendation, 25 August 2026):
  https://www.w3.org/TR/webauthn-3/ ,
  https://www.w3.org/standards/history/webauthn-3/
- RFC 8628, OAuth 2.0 Device Authorization Grant:
  https://www.rfc-editor.org/rfc/rfc8628.html
- RFC 10027 (BCP 247), cross-device flows:
  https://www.rfc-editor.org/info/rfc10027 ,
  https://datatracker.ietf.org/doc/draft-ietf-oauth-cross-device-security/
- RFC 9700 (BCP 240), OAuth 2.0 Security Best Current Practice, January
  2025: https://www.rfc-editor.org/rfc/rfc9700.html
- RFC 9449, DPoP, September 2023: https://www.rfc-editor.org/rfc/rfc9449.html
- RFC 7636 (PKCE), RFC 8252 (native apps), RFC 9207 (issuer
  identification), RFC 9266 (TLS 1.3 channel bindings), RFC 7250 (raw
  public keys), RFC 8446 (TLS 1.3), RFC 8725 (JWT BCP; its successor
  draft-ietf-oauth-rfc8725bis was still a draft in August 2026). These
  were cited from knowledge of their content, not re-fetched in this
  session.
- OAuth 2.1 (draft-ietf-oauth-v2-1) is still an Internet-Draft and is not
  cited as a requirement:
  https://datatracker.ietf.org/doc/draft-ietf-oauth-v2-1/
- RFC 6265bis (cookies) was in the RFC Editor queue at last check and is
  not cited by number:
  https://datatracker.ietf.org/doc/draft-ietf-httpbis-rfc6265bis/
- OpenID Connect Core 1.0 (ID token validation §3.1.3.7; cited from
  knowledge, not re-fetched)
- OpenSubsonic API key extension:
  https://opensubsonic.netlify.app/docs/extensions/apikeyauth/
- MDN, features restricted to secure contexts:
  https://developer.mozilla.org/en-US/docs/Web/Security/Secure_Contexts/features_restricted_to_secure_contexts
- UK product security regime:
  https://www.gov.uk/government/publications/the-uk-product-security-and-telecommunications-infrastructure-product-security-regime

Platforms and transports:

- Chrome M110 disables WebAuthn on certificate errors:
  https://lists.w3.org/Archives/Public/public-webauthn/2022Nov/0135.html
- DBSC general availability in Chrome 146 on Windows:
  https://www.helpnetsecurity.com/2026/04/10/google-chrome-device-bound-session-credentials
- iroh 1.0 (15 June 2026): https://iroh.computer/blog/v1 ,
  https://docs.rs/crate/iroh/latest

Rivals and incidents:

- Emby CVE-2023-33193: https://cveawg.mitre.org/api/cve/CVE-2023-33193
- Immich advisories: https://github.com/immich-app/immich/security/advisories
- Immich CVE-2026-23896:
  https://github.com/immich-app/immich/security/advisories/GHSA-237r-x578-h5mv
- Immich CVE-2026-53662:
  https://github.com/immich-app/immich/security/advisories/GHSA-8244-8vpr-vp9c
- Immich CVE-2026-82272: https://cveawg.mitre.org/api/cve/CVE-2026-82272
- Immich /search/random locked-folder report (third-party write-up; fix
  status unverified):
  https://radar.offseq.com/threat/new-exploitable-bola-found-in-immich-self-hosted-m-e6fff14924c9e5d4
- Immich OAuth settings: https://docs.immich.app/administration/oauth/
- Navidrome advisories:
  https://github.com/navidrome/navidrome/security/advisories ,
  https://github.com/navidrome/navidrome/security/advisories?page=2
- Navidrome authentication and security docs:
  https://www.navidrome.org/docs/usage/integration/authentication/ ,
  https://www.navidrome.org/docs/usage/admin/security/
- Jellyfin Quick Connect: https://jellyfin.org/docs/general/server/quick-connect
- Jellyfin Kotlin SDK authentication guide:
  https://kotlin-sdk.jellyfin.org/guide/authentication.html
- Plex fast user switching and PINs:
  https://support.plex.tv/articles/204232453-fast-user-switching/
- Microsoft on Storm-2372 device-code phishing:
  https://www.microsoft.com/en-us/security/blog/2025/02/13/storm-2372-conducts-device-code-phishing-campaign/
- Descope on nOAuth: https://www.descope.com/blog/post/noauth
