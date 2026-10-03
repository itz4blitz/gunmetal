# Web and API security

Written 2026-10-02. Web tools were available and used. Standards, advisories
and CVE records were checked against primary sources (the OWASP ASVS
repository at tag v5.0.0, the OWASP API Security and Top 10 sites, NIST CSRC,
the RFC Editor, the IETF datatracker, W3C technical reports, the MITRE CVE and
CWE services, the IANA registries, and the vendors' GitHub advisory pages).
The session's web-search budget ran out part-way through; later checks were
made by fetching primary sources directly. Anything not confirmed from a
source is marked "(unverified)".

Versions used, all current as of this date:

- **OWASP ASVS 5.0.0** (released May 2025; still the latest stable release,
  the repository's only newer item being an unstable "bleeding edge" build).
  Requirement numbers are written as `ASVS 8.2.2`; the formal form is
  `v5.0.0-8.2.2`.
- **OWASP API Security Top 10 2023** (still the latest edition), written
  `API1:2023`.
- **OWASP Top 10:2025** (released November 2025), written `A01:2025`. In this
  edition SSRF is folded into A01 and A10 is the new "Mishandling of
  Exceptional Conditions".
- **NIST SP 800-63B-4** (final, July 2025; supersedes 800-63B rev. 3).
- **NIST SP 800-218 (SSDF) version 1.1** (February 2022; the CSRC page lists
  no newer revision).
- **Cookies**: RFC 6265 is still the published standard. Its successor,
  draft-ietf-httpbis-rfc6265bis-22, is in final review at the RFC Editor but
  has **no RFC number yet**; it is cited as an Internet-Draft where it is
  needed (cookie prefixes, `SameSite`).
- **W3C**: Content Security Policy Level 3 (Working Draft, September 2026),
  Trusted Types (Working Draft, June 2026), Fetch Metadata Request Headers
  (Working Draft).
- **CWE** identifiers were each confirmed against the MITRE CWE service.

## Summary

This document covers the HTTP and WebSocket API of the Gunmetal server and
the web client that runs in browsers. It is written before any of that code
exists, so that every control below is a property of the framework layer
(the route table, the middleware chain, the data-access layer and the
client's build rules) rather than something each feature must remember. Most
of the 86 R1 requirements are built once and then hold for every endpoint
added afterwards, and each one becomes a test.

**The account model is not settled yet.** Record 1 points at passkeys, OIDC
and device-bound keys with no central account, and the research suggests
households with profiles, invitations and per-app credentials. This document
does not choose between them. It defines one abstraction, the **principal**
(account, profile, device, credential kind, scope, session, time of last
user verification), and every control is written against it. Whatever the
identity design becomes, it plugs into the same route table and the same
authorisation layer. The sign-in ceremonies themselves belong to the sibling
identity document; this one covers what happens at the API boundary.

The key decisions:

1. **Deny by default, enumerated and tested.** Every route is declared in one
   typed route table with an access class. Only three narrow classes are not
   ordinary signed-in sessions: *public* (the web client's static files, a
   minimal server-identity endpoint and a bodiless health check),
   *credential exchange* (setup code, invitation secret, refresh, pairing
   code, sign-in ceremonies) and *capability* (signed media URLs, WebSocket
   tickets). The set of non-session routes must equal a short, checked-in,
   human-reviewed list, and a test fails the build if it drifts. This answers
   the most common failure in this product category: streams, images and
   subtitles left unauthenticated "because players cannot send headers"
   (Jellyfin issue 5415).
2. **One choke point for objects.** Handlers cannot reach a stored object
   except through an authorisation layer that takes the principal and returns
   a type nothing else can construct. Objects a principal cannot see return
   the same 404 as objects that do not exist. A generated test replays every
   principal's identifiers as every other principal on every route, including
   batch routes, sync, search, now-playing, WebSocket events and media URL
   issuance. The expected results live in literal files, as CONTRIBUTING.md
   rule 4 requires. Broken object-level authorisation is the top API risk
   (API1:2023), and the 2026 advisories from Navidrome and Jellyfin are almost
   all variations on it.
3. **Bytes travel on capability URLs, never on session tokens.** Audio,
   video, artwork and subtitles are fetched from short-lived URLs whose path
   carries a MAC over the object, the representation, the operation, the
   session and an expiry. They work for `<audio>` elements, TV players and
   cast receivers that cannot add headers, and they never expose a reusable
   credential. On every request the server re-checks that the session is
   alive and the object is still visible, so revocation takes effect on the
   next byte range (the Navidrome stale-share lesson). Session tokens, API
   keys and refresh tokens are never accepted in a URL.
4. **The browser client uses a `__Host-` HttpOnly cookie, not a token in
   JavaScript.** Same-origin cookies keep the credential out of reach of
   script. Cross-site request forgery is blocked by a required custom request
   header (works over plain HTTP and in old engines), by `Sec-Fetch-Site`
   checks where browsers send them, by `Origin` checks, and by JSON-only
   bodies. Native apps and cross-origin TV builds use the `Authorization`
   header instead. A request presenting two credentials is refused.
5. **The web client cannot execute injected markup.** A strict Content
   Security Policy with `require-trusted-types-for 'script'`, a build that
   fails on any HTML-string sink, and a rule that every title, description,
   lyric and device name is rendered as text. Rich text (artist biographies)
   travels as a small validated structure, not HTML. Trusted Types reached
   Baseline in February 2026, so enforcement works in all three engines;
   older TV engines still get the lint-level guarantee.
6. **Credentials never cross a cleartext network link.** Over plain HTTP
   from anything other than loopback, the server does not accept sign-in or
   set cookies. Browsers already refuse `Secure` cookies, passkeys, Fetch
   Metadata and `Clear-Site-Data` on such origins, so this is the honest
   default, but it forces a decision from the owner on how LAN browsers get a
   certificate (open decision 1).
7. **Network location never grants access.** Forwarding headers are ignored
   unless the admin lists trusted proxies, and no rule may relax
   authentication because a request looks local. That is the Emby 2023
   compromise (CVE-2023-33193) and the Navidrome September 2026 rate-limit
   bypass in one rule.
8. **One egress client for everything that leaves the server.** Metadata
   providers, cover art, lyrics, update checks, OIDC discovery, webhooks,
   M3U sources and plugin traffic all go through a single crate that resolves
   names itself, refuses any address that is not globally reachable under the
   IANA special-purpose registries (including addresses embedded in IPv6),
   connects to the address it checked, follows no redirects by default and
   always verifies TLS. CI fails if any other crate links an HTTP client.
9. **Uploads and embedded art are rebuilt, not trusted.** Images are decoded
   by memory-safe Rust decoders with dimension limits set before allocation,
   re-encoded, stripped of metadata and stored under content-hash names. SVG
   is refused. Nothing a request supplies ever becomes part of a filesystem
   path.
10. **Errors say nothing useful to an attacker.** RFC 9457 problem details
    drawn from a closed catalogue, with a request ID, and a last-resort
    handler that fails closed.

**Target level.** OWASP ASVS Level 2 as the baseline, plus the Level 3
items that are cheap for a greenfield Rust server and matter for software
that non-experts expose to the internet: HTTP method allow-listing (4.1.4),
request-smuggling defences (4.2.x), `Cross-Origin-Opener-Policy` (3.4.8),
`Cross-Origin-Resource-Policy` (3.5.8), indistinguishable authentication
failures (6.3.8), no version disclosure (13.4.6), metadata stripping (14.2.8),
upload quotas and pixel-flood limits (5.2.4, 5.2.6) and cache-deception
defences (14.2.5). Deviations are listed in the design guidance, as ASVS
asks.

**Usability.** No protection here has an off switch, and none of them asks a
household member to do anything: signed URLs, headers, CSP and egress rules
are invisible. The visible ones are designed for living rooms: TVs sign in
by approving a code from a phone, a child who mistypes a profile PIN is
unlocked by a parent rather than locked out for an hour, passkey sign-in is
never rate-limited per account (it cannot be guessed), image grids are
limited by connection count rather than request count so a TV can scroll
thousands of covers, and there are no CAPTCHAs anywhere.

## Threats

Likelihood is for a typical self-hosted install reachable from the internet
or from a busy home network, judged from the incident record in the
research documents and the advisories cited below.

| ID | Threat | Who (the attacker or failure) | Impact | Likelihood | Mitigated by (requirement IDs) |
|---|---|---|---|---|---|
| T-API-01 | An endpoint is left without authentication: streams, images, subtitles, user lists, system information, a debug route, a route added in a hurry | Internet scanners; anyone on the LAN; a developer mistake | Library contents and listening history exposed; an entry point for injection bugs behind it | High (Jellyfin issue 5415; CVE-2023-49096 was reachable without authentication) | SEC-API-001, SEC-API-002, SEC-API-003, SEC-API-029, SEC-API-091 |
| T-API-02 | Broken object-level authorisation: user B reads or changes user A's playlists, history, queue, devices or shares by supplying A's identifiers | Any signed-in user, a friend with limited access, a compromised household device | Privacy breach, tampering, deletion | High (Jellyfin playlist IDOR GHSA-9x85-gx46-6522; Navidrome share ownership GHSA-3g4p-jhv2-xrxf) | SEC-API-010, SEC-API-011, SEC-API-012, SEC-API-013, SEC-API-023, SEC-API-024, SEC-API-025 |
| T-API-03 | Restrictions bypassed through a side path: search, sync, now-playing, bookmarks, playlists that contain restricted tracks, artwork or media URLs | A child profile; a friend limited to one library | Restricted content reached; parental controls defeated | High (Navidrome GHSA-pcjv-h48m-833g; Plex removing folder view under restrictions) | SEC-API-014, SEC-API-015, SEC-API-016, SEC-API-028 |
| T-API-04 | Function-level escalation: a non-admin calls an admin route, or a scoped credential widens its own scope | A signed-in user; a stolen app credential | Full server control, including choosing library roots on the host filesystem | Medium (Immich API key escalation, CVE-2026-23896) | SEC-API-019, SEC-API-020, SEC-API-021, SEC-API-022 |
| T-API-05 | Property-level flaws: a request sets a field it should not (`isAdmin`, `ownerId`), or a response returns fields the caller should not see (filesystem paths, other users' IP addresses or devices) | A signed-in user | Escalation; disclosure of host layout and other people's activity | Medium | SEC-API-067, SEC-API-068 |
| T-API-06 | A reusable credential leaks through a URL: server and proxy logs, `Referer`, browser history, screenshots, a link pasted into chat, or browser storage read by script | Anyone with log access; third-party sites; injected script | Account takeover | High (Jellyfin issue 5415 lists API keys in URLs and tokens in local storage) | SEC-API-004, SEC-API-026, SEC-API-029, SEC-API-030, SEC-API-032, SEC-API-053, SEC-API-095 |
| T-API-07 | Access survives revocation: a removed device, disabled account, deleted share or withdrawn library grant keeps streaming or keeps receiving events | A former household member; a lost phone | Continued access after the owner believes it ended | High (Navidrome GHSA-wp9c-pw66-c6j2) | SEC-API-017, SEC-API-027, SEC-API-028 |
| T-API-08 | Cross-site request forgery from a web page the user visits, against the server on the internet or on the LAN, including from a sibling app on the same home domain | A malicious or compromised website; another self-hosted app on a sibling subdomain | Actions performed as the user: deleting playlists, approving devices, changing settings | Medium | SEC-API-032, SEC-API-033, SEC-API-034, SEC-API-035, SEC-API-036, SEC-API-040, SEC-API-098 |
| T-API-09 | Cross-site WebSocket hijacking: a page opens the event socket with the user's cookie | A malicious website | Live activity read; playback and sessions controlled | Medium (CVE-2023-0957, Gitpod) | SEC-API-041, SEC-API-042 |
| T-API-10 | DNS rebinding against the server, most dangerously a freshly installed, unclaimed one | A malicious website the owner visits while setting up | Server claimed by the attacker; unauthenticated endpoints read | Medium (CVE-2018-5702, Transmission; CVE-2024-28224, Ollama) | SEC-API-006, SEC-API-007 |
| T-API-11 | Stored cross-site scripting through metadata: tags in a downloaded album, provider biographies, lyrics, another user's playlist name, a device name | Whoever made a file in the library; a compromised metadata provider; another household user | Session riding, admin actions, theft of the synced library from browser storage | High (Jellyfin GHSA-fv79-gmhx-xh2v; Immich GHSA-8244-8vpr-vp9c) | SEC-API-044, SEC-API-045, SEC-API-046, SEC-API-047, SEC-API-048, SEC-API-050, SEC-API-051 |
| T-API-12 | A third-party script or compromised dependency runs inside the web client | Supply-chain attacker | As for cross-site scripting | Medium | SEC-API-044, SEC-API-045, SEC-API-049 |
| T-API-13 | Clickjacking of admin or approval screens | A malicious website | Unintended approvals and setting changes | Low | SEC-API-044, SEC-API-053 |
| T-API-14 | Guessing short secrets (profile PIN, pairing code, setup code, share password) and credential stuffing | A housemate; an internet attacker | Account or profile takeover; parental controls defeated | High (Navidrome GHSA-p994-r776-mw52, unauthenticated brute force through the Subsonic API) | SEC-API-006, SEC-API-056, SEC-API-057, SEC-API-058 |
| T-API-15 | Rate limits bypassed, or the owner locked out, by forging client-address headers | An internet attacker | Unlimited guessing, or denial of service against the owner | High (Navidrome GHSA-f295-6wp9-qqfg) | SEC-API-057, SEC-API-074 |
| T-API-16 | Trust based on location: a remote request is made to look local and gets relaxed sign-in | An internet attacker | Administrative access | Medium (CVE-2023-33193, Emby; about 1,200 servers backdoored according to the research document) | SEC-API-074, SEC-API-075 |
| T-API-17 | Resource exhaustion: huge or deeply nested bodies, slow clients, HTTP/2 stream resets, many-range requests, expensive searches, repeated scans | An internet attacker; a buggy client | Server unavailable for the household | Medium (CVE-2023-44487; CVE-2011-3192) | SEC-API-031, SEC-API-043, SEC-API-060, SEC-API-061, SEC-API-062, SEC-API-063, SEC-API-064, SEC-API-065 |
| T-API-18 | SQL or full-text-search injection | A signed-in user; a crafted tag | Data disclosure or corruption | Medium (Navidrome GHSA-hm54-32q6-3rcr) | SEC-API-063, SEC-API-066 |
| T-API-19 | Server-side request forgery through metadata fetching, OIDC profile pictures, artwork URLs in tags or provider data, SVG references, webhooks, M3U sources or plugins | A user who controls a URL the server will fetch; a malicious provider response | The router's admin page, other LAN services, cloud metadata endpoints reached from inside the house | Medium (CVE-2020-13379, Grafana; Immich GHSA-hq46-gw2v-q86p; Jellyfin GHSA-cf3c-8m59-2vhx) | SEC-API-076, SEC-API-077, SEC-API-078, SEC-API-079, SEC-API-080, SEC-API-082, SEC-API-083, SEC-API-084 |
| T-API-20 | DNS rebinding of the server's own outbound requests (the name resolves to a public address when checked and a private one when used) | An attacker who controls a DNS zone | As for SSRF, bypassing the address check | Medium | SEC-API-077 |
| T-API-21 | Unsafe consumption of third-party APIs: oversized or malformed responses, markup in text fields, or interception because TLS verification was off | A compromised or hostile provider; an on-path attacker | Injection into the library; denial of service; poisoned metadata | Medium (Immich GHSA-hfvf-5c8x-8rc4) | SEC-API-078, SEC-API-081 |
| T-API-22 | Malicious uploads and embedded images: polyglots, scriptable SVG, pixel floods, decoder exploits, filenames that traverse | A signed-in user; whoever made a media file | Code execution on the host; stored XSS; disk exhaustion | High (CVE-2016-3714; Immich GHSA-q89f-h332-8q2h; Jellyfin CVE-2026-35031) | SEC-API-054, SEC-API-085, SEC-API-086, SEC-API-087, SEC-API-088, SEC-API-089, SEC-API-090 |
| T-API-23 | Path traversal or symlink escape when serving files or browsing folders | A signed-in user; a symlink placed in the library | Arbitrary file read or directory deletion on the host | Medium (Jellyfin GHSA-6828-c7cx-hvqm, Critical; Navidrome GHSA-r5qr-m328-qcf4) | SEC-API-018, SEC-API-022, SEC-API-087 |
| T-API-24 | Information leakage: stack traces, paths, versions that let scanners target known flaws, or responses that reveal which accounts exist | Internet scanners; a guessing attacker | Targeted exploitation; user enumeration | High (scanners counted about 314,000 Plex servers on a vulnerable version in August 2025, per the research document) | SEC-API-005, SEC-API-058, SEC-API-072, SEC-API-073 |
| T-API-25 | A shared cache (reverse proxy, CDN, tunnel) stores one user's response and serves it to another, or is tricked by path confusion | An attacker who knows the server sits behind a CDN | Another user's library or session data exposed | Medium (PortSwigger web cache deception research, 2024) | SEC-API-055 |
| T-API-26 | Generated links point to an attacker because they were built from the request's `Host` or `X-Forwarded-Host` | An internet attacker | Phished invitations; stolen invite secrets | Medium | SEC-API-007, SEC-API-069 |
| T-API-27 | Open redirect after sign-in | A phisher | Credential phishing; script injection through the redirect | Medium (Immich GHSA-8244-8vpr-vp9c) | SEC-API-070 |
| T-API-28 | Request smuggling between a reverse proxy and the server | An internet attacker | Requests attributed to other users; cache poisoning | Low | SEC-API-009 |
| T-API-29 | Forgotten, legacy, debug or compatibility endpoints with weaker checks | A developer mistake; an attacker hunting old routes | As for missing authentication | Medium (Optus 2022, a dormant internet-facing API, according to the Australian regulator; Navidrome Subsonic advisories) | SEC-API-008, SEC-API-091, SEC-API-092, SEC-API-093, SEC-API-094 |
| T-API-30 | Device-code phishing: the attacker starts a TV sign-in and persuades the victim to approve the code | A phisher | The attacker's device joins the victim's account | Medium (Microsoft, Storm-2372, February 2025) | SEC-API-059 |
| T-API-31 | Credentials captured on a cleartext link | Anyone on the same Wi-Fi; an on-path attacker | Session theft | Medium | SEC-API-037, SEC-API-038 |
| T-API-32 | A shared computer keeps a signed-out user's library and session data | The next person at the computer | Privacy breach | Medium | SEC-API-039 |
| T-API-33 | Unicode tricks in names (bidirectional overrides, look-alikes) on approval screens, and log injection | A device or user choosing its own name | Misleading approvals; forged log lines | Medium (CVE-2021-42574) | SEC-API-048, SEC-API-095 |
| T-API-34 | Live events reach users who should not see them | A design mistake | Other people's listening activity exposed | Medium (Jellyfin GHSA-4vx8-xhc9-qg6x) | SEC-API-016 |
| T-API-35 | Spreadsheet formula injection through exported titles | Whoever named a file or playlist | Code run in the admin's spreadsheet program | Low | SEC-API-071 |
| T-API-36 | Invitation or share links leak, are guessed or outlive their purpose | Anyone a link is forwarded to | Unwanted members; content shared beyond intent | Medium | SEC-API-096, SEC-API-097 |
| T-API-37 | Parser differentials: duplicate JSON keys, the same parameter in two places | A signed-in user | Authorisation decided on one value and acted on with another | Low (CVE-2017-12635, CouchDB) | SEC-API-067 |
| T-API-38 | An old or unusual browser engine silently lacks a protection (Trusted Types, Fetch Metadata) | Old smart-TV engines; outdated browsers | Weaker defence in depth | Medium | SEC-API-033, SEC-API-045, SEC-API-052 |

## Requirements

"Integration test" means a test that runs the real server binary's router
against a real SQLite database in a temporary directory, with no mocked
storage. "Literal file" means expected values written by a person and
reviewed, never generated from the code under test (CONTRIBUTING.md rule 4).
Pure logic named below (identifier codecs, URL signing, address
classification, string normalisation, header parsing, rate-limit arithmetic)
belongs in `gunmetal-core`, where it is held to 100% coverage and zero
surviving mutants like the parsers.

### Route inventory and authentication

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-API-001 | Every HTTP and WebSocket route must be registered through one typed route table in which each route declares exactly one access class (public, credential exchange, capability, signed-in capability, or admin capability), and it must not be possible to build the router from anything else. | ASVS 8.1.1, 8.2.1, 6.3.4; API5:2023, API9:2023; A01:2025; CWE-862, CWE-306; SP 800-218 PW.1 | R1 | Unit test that enumerates the built router and asserts every route has a declared class; the route constructor takes the access class as a required argument, so an undeclared route does not compile; mutation testing on the router builder. |
| SEC-API-002 | The set of routes whose class is public, credential exchange or capability must exactly equal a checked-in allow-list file; in R1 that list must contain only the embedded web-client assets, `GET /api/v1/server`, `GET /healthz`, the first-run setup routes, the sign-in ceremony routes, token refresh, invitation redemption, device-pairing routes and the signed media routes. | ASVS 8.2.1, 13.4.5; API2:2023, API9:2023; CWE-306 | R1 | Integration test comparing the enumerated non-session routes with the literal allow-list file; the file is under CODEOWNERS so any change needs a maintainer's review. |
| SEC-API-003 | Every route that is not on the allow list must answer 401 with a byte-identical body for each of: no credential, malformed credential, expired credential, revoked credential, and a credential for a disabled or deleted account, and must do so before reading the request body or running handler code. | ASVS 7.2.1, 7.4.1, 7.4.2; API2:2023; A07:2025; CWE-287, CWE-306 | R1 | Integration test generated from the route table that calls every route in each of the five credential states and compares status and body with a literal expected response; a handler-side probe asserts the handler never ran. |
| SEC-API-004 | Session tokens, refresh tokens and API keys must be accepted only from the session cookie (SEC-API-032) or the `Authorization` header; a request carrying a credential-shaped query parameter or path segment, two credentials (cookie plus header, or two `Authorization` headers), or a credential in an unexpected header must be rejected with 400, and the rejected value must not be logged. | ASVS 14.2.1, 7.2.1, 15.3.7; API2:2023; RFC 9700 §4.3.2; RFC 6750 §2.3; CWE-598 | R1 | Integration test over every route with `?token=`, `?access_token=`, `?api_key=`, `?apiKey=`, duplicated `Authorization` and cookie-plus-header variants; a log-capture assertion that no submitted secret appears in output. |
| SEC-API-005 | Unauthenticated responses must not reveal the software version, operating system, dependency names, user names or library statistics; `GET /api/v1/server` must return only the supported protocol-version range, the server instance identifier, whether the server is claimed, and the enabled sign-in methods, and no response may carry a `Server` header with a version. | ASVS 13.4.6, 13.4.5, 14.2.6; API8:2023; A02:2025; CWE-497, CWE-200 | R1 | Integration test asserting the exact JSON of `GET /api/v1/server` against a literal schema with no additional properties, and asserting the absence of `Server` and `X-Powered-By` headers on every route. |
| SEC-API-006 | **Withdrawn 2026-10-02: merged into SEC-IAM-007, SEC-IAM-008.** The claim code is no longer replaced after 5 failures. | ASVS 2.3.1, 6.4.1, 6.6.3, 11.5.1, 11.2.4; API2:2023; CWE-841, CWE-1188, CWE-307 | Withdrawn | Proved by the tests of SEC-IAM-007, SEC-IAM-008 |
| SEC-API-007 | The server must reject with 421 any request whose `Host` (or HTTP/2 `:authority`) is not in the configured set: the configured public hostnames, the machine's own interface addresses, `localhost`, and the configured mDNS name. | ASVS 4.1.3; RFC 9110 §15.5.20; CWE-346, CWE-807 | R1 | Integration test with `Host: attacker.example`, a rebinding-style hostname that resolves to the server, and each allowed form; unit tests of host matching in core, including ports, trailing dots, IPv6 brackets and case. |
| SEC-API-008 | Only the methods declared for a route may be used; any other method, including `TRACE` and `CONNECT`, must receive 405, and method-override headers such as `X-HTTP-Method-Override` must be ignored. | ASVS 4.1.4, 13.4.4; API8:2023; A02:2025 | R1 | Integration test sending every standard method plus an override header to every route and comparing with the route table. |
| SEC-API-009 | HTTP/1.1 requests with both `Content-Length` and `Transfer-Encoding`, with an invalid or repeated `Content-Length`, or with obsolete line folding must be rejected; HTTP/2 requests with CR or LF in a field value or with connection-specific header fields must be rejected. | ASVS 4.2.1, 4.2.3, 4.2.4; CWE-444 | R1 | Integration test that writes raw request bytes to a socket for each malformed framing and asserts a 400 and a closed connection. |

### Object-level authorisation

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-API-010 | Handlers must obtain stored objects only through an authorisation layer that takes the principal and returns a wrapper type (for example `Visible<T>`) whose constructor is private to that layer; handler code must not be able to call the storage layer directly. | ASVS 8.2.2, 8.3.1; API1:2023; A01:2025; CWE-639, CWE-863 | R1 | Module visibility makes direct storage access a compile error; a clippy `disallowed_methods` rule as a CI check; unit and property tests of the policy functions with mutation testing. |
| SEC-API-011 | For every route that accepts an object identifier (in the path, query, body or a WebSocket message), a principal who may not see the object must receive the same 404 status and body as for an identifier that does not exist, and no state may change. | ASVS 8.2.2, 6.3.8; API1:2023; CWE-639, CWE-204 | R1 | Generated cross-principal integration test: a fixture library with an owner, a second adult, a restricted child profile, an admin and an app credential; every identifier owned by each is replayed as each other principal on every route; responses and a database checksum are compared with a literal expectations file. |
| SEC-API-012 | A request that carries several object identifiers (batch actions, adding tracks to a playlist, replacing the queue, creating a share) must authorise every identifier and must reject the whole request if any one fails. | ASVS 8.2.2, 2.3.3; API1:2023; CWE-639 | R1 | Property test that builds lists of permitted identifiers with one forbidden identifier at a random position and asserts rejection with no partial write. |
| SEC-API-013 | The acting principal must come only from the authenticated credential; a request field naming a user, profile, owner or household must be rejected with 400 unless the route is an admin route that targets another principal and the caller holds that capability. | ASVS 8.2.2, 8.3.1, 15.3.3; API1:2023; CWE-639, CWE-807 | R1 | Integration test that injects another principal's identifier into every identity-shaped field of every request schema; a schema check in CI that flags identity-named fields in non-admin request types. |
| SEC-API-014 | Library visibility, per-profile restrictions and parental rules must be applied inside the authorisation layer to every read path: browse, search, sync deltas, recommendations, now-playing, history, bookmarks, playlists that contain restricted items, and the issuing of artwork and media URLs. | ASVS 8.2.2, 8.1.1; API1:2023; CWE-863 | R1 | Integration test in which a restricted profile calls every read route and the sync endpoint against a fixture containing restricted items, asserting none is returned or referenced; the expected visible set is a literal file. |
| SEC-API-015 | Library sync deltas must be computed per principal, and when an item becomes invisible to a principal the delta must carry only its identifier as a removal, never its metadata. | ASVS 8.2.2, 14.2.6; API3:2023; CWE-200 | R1 | Integration test that withdraws a library grant, syncs, and compares the delta with a literal expected payload. |
| SEC-API-016 | Every server-pushed event must be filtered per recipient through the same policy layer before it is written to a socket; the server must have no broadcast-to-all path. | ASVS 8.2.2; API1:2023; CWE-359, CWE-863 | R1 | Integration test with several connected principals generating playback, playlist and session events, asserting each socket receives exactly the literal expected events. |
| SEC-API-017 | A change to anything authorisation depends on (revoked device or session, disabled account, withdrawn library grant, deleted share) must take effect on the next HTTP request, and open WebSocket connections affected by it must be closed within 5 seconds. | ASVS 8.3.2, 7.4.1, 7.4.2; API1:2023; CWE-613 | R1 | Integration test that revokes each kind of grant mid-session, then asserts the next request fails and the socket closes within the deadline (time injected through a test clock). |
| SEC-API-018 | The byte-serving path must open only the file path recorded at scan time, and at open time must confirm the opened file is a regular file whose resolved location is inside a configured library root, refusing symlinks that lead outside it. | ASVS 5.3.2, 15.4.2; CWE-22, CWE-59, CWE-367 | R1 | Integration test with a temporary library containing a symlink to a file outside the root, a symlink to a directory, and a file swapped for a symlink after scanning; property tests for the containment check in core. |

### Function-level authorisation

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-API-019 | Each route must declare the capability it requires, checked by the router before the handler runs, and a literal matrix of expected status codes for every route and every role (anonymous, restricted profile, standard user, admin, app credential, share capability) must match actual behaviour. | ASVS 8.2.1, 8.1.1; API5:2023; A01:2025; CWE-285, CWE-862 | R1 | Integration test that runs the full matrix and compares with the literal file; this test is what kills mutants in the policy code. |
| SEC-API-020 | Any credential whose scope is narrower than its owner's rights must be evaluated as the intersection of its scope and the owner's current rights, and must never be able to create, widen or edit credentials, including its own. | ASVS 8.2.1, 8.3.3; API5:2023; CWE-269, CWE-863 | R1 | Property test over generated scope sets asserting that every effective permission is a subset of both inputs; integration test that a scoped credential calling credential routes receives 403. |
| SEC-API-021 | **Withdrawn 2026-10-02: merged into SEC-IAM-041.** The fresh-uv list lives in SEC-IAM-041. | ASVS 7.5.1, 7.5.3; NIST SP 800-63B-4 §2.2.3; API5:2023; CWE-306 | Withdrawn | Proved by the tests of SEC-IAM-041 |
| SEC-API-022 | The folder browser used to choose library roots must be admin-only, must list directories only (never file contents), and must be confined to admin-configured browse roots after canonicalising the path and resolving symlinks. | ASVS 5.3.2, 8.2.1; API5:2023; CWE-22, CWE-59 | R1 | Integration tests on a temporary tree with `..`, percent-encoded and double-encoded traversal, absolute paths and symlink escapes; property tests of the canonicaliser in core. |

### Identifiers and cursors

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-API-023 | Every identifier visible outside the server must carry at least 128 bits from a CSPRNG, or be derived by HMAC-SHA-256 under a 256-bit server secret from a stable natural key and truncated to at least 128 bits; identifiers must not be sequential and must not encode paths, names or timestamps. A version-4 UUID (122 random bits) does not meet this. | ASVS 11.5.1, 8.2.2; API1:2023; CWE-340, CWE-330 | R1 | Unit tests of the identifier codec (length, alphabet, rejection of non-canonical encodings); a statistical test of a million generated identifiers for collisions; a CI check that no exported identifier type wraps an integer. |
| SEC-API-024 | Identifiers must be typed by kind, and the server must answer an identifier of the wrong kind with the same 404 used for missing objects. | ASVS 1.5.3, 15.3.5; API1:2023; CWE-639 | R1 | Property test that feeds identifiers of every other kind into every identifier slot of every route. |
| SEC-API-025 | Pagination cursors and other continuation tokens must be either opaque server-side handles or MAC-protected, and nothing inside a returned cursor may be trusted for an authorisation decision. | ASVS 9.1.1, 8.3.1; API1:2023; CWE-807 | R1 | Property test that tampered cursors are rejected with 400; integration test that a cursor issued to one principal and replayed by another returns only the second principal's visible data. |

### Signed media and image URLs

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-API-026 | Audio, video, artwork, lyrics files and subtitles must be served only from capability URLs whose path (not query) carries a token made of a version, key identifier, expiry, operation, representation, object identifier and session handle, authenticated by HMAC-SHA-256 with a tag of at least 128 bits and a domain-separation label; the representation must come from a fixed server-defined set (for example a few artwork sizes), and verification must be constant-time. | ASVS 9.1.1, 9.1.2, 9.2.1, 9.2.2, 11.2.4, 14.2.1; API1:2023, API4:2023; CWE-347, CWE-598 | R1 | Unit tests of encode and verify in core; property tests that any single-bit change, any field substitution and any truncation fails; a fuzz target for the token parser; mutation testing on the verifier. |
| SEC-API-027 | Capability URL lifetimes must be: stream, the item's duration plus 10 minutes, capped at 4 hours; artwork, 1 hour, aligned to a fixed time bucket so that repeat requests in the same bucket reuse the same URL; expiry must be checked on every request. Every client and cast sender must refresh an expired URL transparently and resume at the same position with no user-visible error. | ASVS 9.2.1, 7.3.2; API2:2023; CWE-613 | R1 | Unit test of the lifetime calculator against the literal table; property tests (never above the cap, monotonic in duration, bucket alignment); injected-clock end-to-end test that pauses past expiry and resumes; cast-sender integration test with a fake receiver |
| SEC-API-028 | After the MAC and expiry checks, every request to a capability URL must confirm that the bound session is still active and the principal may still see the object, so that revocation stops playback on the next range request. | ASVS 8.3.2, 7.4.1; API1:2023; CWE-613, CWE-863 | R1 | Integration test that revokes the session, withdraws the library grant and deletes the share in turn, then reuses a still-unexpired URL and asserts rejection each time. |
| SEC-API-029 | Media routes must authenticate only by the URL capability: they must ignore cookies and `Authorization`, must never set a cookie, and an unsigned or wrongly signed request must fail even when a valid session cookie is present. This applies to the web client too: same-origin media is fetched by capability URL, never by cookie, and the service worker and Cache Storage must not keep capability URLs. | ASVS 8.3.1, 3.5.8; API2:2023; CWE-306, CWE-352 | R1 | Integration test pairing valid and invalid signatures with present and absent cookies and headers. |
| SEC-API-030 | Media URL keys must be at least 256 bits from a CSPRNG, kept in the server's durable secret store rather than the rebuildable cache, rotated at least every 30 days with an overlap window selected by key identifier, and revocable so that revoking a key invalidates every URL it signed. | ASVS 11.2.3, 11.5.1, 13.3.4, 13.1.4; A04:2025 | R1 | Unit tests for key selection by identifier and overlap; integration test that revoking a key rejects URLs signed with it and that a cache rebuild keeps keys. |
| SEC-API-031 | The server must answer a `Range` header that names more than one range with 416, and must cap concurrent media streams per session and per principal at configured limits. | RFC 9110 §14.2, §15.5.17, §17.15; ASVS 15.2.2; API4:2023; CWE-770 | R1 | Integration tests: one range returns 206, two or more return 416 with no body, and the stream cap returns 429 on the stream above the limit. |

### Browser sessions, request forgery and transport

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-API-032 | The web client served from the server's own origin must authenticate with a single cookie named `__Host-gm_session` set with `Secure`, `HttpOnly`, `SameSite=Lax`, `Path=/` and no `Domain`, holding a random token of at least 256 bits that is stored server-side only as a hash; the token must never be readable from JavaScript or written to web storage. The cookie must never be set on a plaintext response, its Secure attribute must never depend on request headers, and the separate admin cookie of SEC-IAM-041 uses SameSite=Strict. | ASVS 3.3.1, 3.3.2, 3.3.3, 3.3.4, 7.2.3, 14.3.3; NIST SP 800-63B-4 §5.1.1; draft-ietf-httpbis-rfc6265bis-22 (Internet-Draft); CWE-614, CWE-1004, CWE-1275 | R1 | Integration test asserting the exact `Set-Cookie` string; end-to-end browser test (Chromium, Firefox, WebKit) asserting `document.cookie` is empty and that localStorage, sessionStorage and IndexedDB hold no token after sign-in; integration test with a forged X-Forwarded-Proto: http asserting the identical cookie |
| SEC-API-033 | Every cookie-authenticated API request, whatever its method, must carry the header `Gunmetal-Request: 1`; requests without it must be refused with 403 before authentication. | ASVS 3.5.1, 3.5.2; A01:2025; CWE-352 | R1 | Integration test over every cookie-capable route with and without the header; end-to-end test that a form post and an image tag from a second origin are refused. |
| SEC-API-034 | A cookie-authenticated request whose `Sec-Fetch-Site` is `cross-site` or `same-site` must be refused with 403, and a cookie-authenticated request with an unsafe method must also be refused when its `Origin` is present and not one of the configured origins. | ASVS 3.5.1, 3.5.3, 3.5.8; W3C Fetch Metadata; CWE-352, CWE-346 | R1 | Integration test of the full matrix of `Sec-Fetch-Site` values, methods and `Origin` values, including a sibling subdomain of the configured host. |
| SEC-API-035 | Routes that take a body must accept only `Content-Type: application/json` (or the specific upload types declared for upload routes) and must answer anything else with 415 before parsing. | ASVS 3.5.2, 4.1.1; CWE-352 | R1 | Integration test sending form, multipart, `text/plain` and missing content types to every body-taking route. |
| SEC-API-036 | `GET` and `HEAD` routes must not change application state other than last-seen timestamps and audit records; every route that changes state must be declared as mutating and use `POST`, `PUT`, `PATCH` or `DELETE`. | ASVS 3.5.3; RFC 9110 §9.2.1; CWE-352, CWE-650 | R1 | Unit test over the route table asserting no `GET` or `HEAD` route is marked mutating; integration test that a database checksum is unchanged after calling every `GET` route. |
| SEC-API-037 | **Withdrawn 2026-10-02: merged into SEC-NET-001.** One cleartext rule. | ASVS 12.2.1, 4.1.2, 4.4.1; NIST SP 800-63B-4 §5.1; A04:2025; CWE-319 | Withdrawn | Proved by the tests of SEC-NET-001 |
| SEC-API-038 | HTTPS responses served under a hostname must carry `Strict-Transport-Security` with `max-age` of at least one year; `includeSubDomains` must be added only for project-issued per-server names or when the admin confirms they control every subdomain. | ASVS 3.4.1; RFC 6797; CWE-319 | R1 | Integration test of the header for hostname, IP literal and loopback configurations. |
| SEC-API-039 | Signing out must invalidate the session on the server, respond with `Clear-Site-Data: "cache", "cookies", "storage"` on secure origins, and the web client must delete its local library store and unregister any service worker even when the server cannot be reached. | ASVS 14.3.1, 7.4.1, 7.4.4; CWE-613, CWE-525 | R1 | Integration test of the response header and the server-side session state; end-to-end test that, after signing out online and offline, storage is empty and the back button shows no library data. |

### Cross-origin access and WebSockets

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-API-040 | The API must send no CORS headers by default; cross-origin access may be enabled only for an admin-configured list of exact origins, must never reflect the request's `Origin`, never allow `null`, never send `Access-Control-Allow-Credentials`, and never apply to cookie-authenticated requests. | ASVS 3.4.2; API8:2023; A02:2025; CWE-942, CWE-346 | R1 | Integration test with origins that are hostile, `null`, suffix-matching, prefix-matching, scheme-mismatched and port-mismatched, asserting no `Access-Control-Allow-*` headers; unit tests of the configuration validator. |
| SEC-API-041 | A WebSocket upgrade must require a valid credential, and a cookie-authenticated upgrade must be refused with 403 before completion unless its `Origin` exactly matches a configured origin. | ASVS 4.4.2, 4.4.4; RFC 6455 §10.2; CWE-1385 | R1 | Integration test of upgrades with matching, missing, hostile and sibling-subdomain origins. |
| SEC-API-042 | A client that cannot send a cookie or header on the upgrade must authenticate by sending, as its first message within 5 seconds, a single-use ticket of at least 128 bits that expires within 30 seconds and is bound to the session that requested it; the server must process no other message before that and must close the connection on any failure. | ASVS 4.4.3, 4.4.4, 7.2.3; CWE-294, CWE-306 | R1 | Integration tests for a replayed ticket, an expired ticket, a late ticket, another session's ticket, and a message sent before the ticket. |
| SEC-API-043 | Each WebSocket message must be at most 64 KiB, schema-validated and authorised like an HTTP request; the server must limit messages per second per connection, connections per principal, and must close idle connections. | ASVS 2.2.1, 15.2.2; API4:2023; CWE-770, CWE-400 | R1 | Integration tests at and above each limit; a fuzz target for the message decoder. |

### Web client: CSP, Trusted Types and rendering untrusted text

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-API-044 | Every HTML response must carry a Content Security Policy equivalent to `default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; img-src 'self' blob:; media-src 'self' blob:; font-src 'self'; connect-src 'self'; worker-src 'self'; manifest-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'; object-src 'none'; require-trusted-types-for 'script'; trusted-types gunmetal-loader`, with no `'unsafe-inline'` or `'unsafe-eval'` in `script-src`. `connect-src` may add the configured relay origins only while browser remote access is on (R2); no directive allows `data:`. | ASVS 3.4.3, 3.4.6, 1.3.2; W3C CSP Level 3, Trusted Types; A05:2025; CWE-79, CWE-1021 | R1 | Integration test asserting the exact header string; end-to-end test in Chromium, Firefox and WebKit that visits every screen with the production build and fails on any `securitypolicyviolation` event. |
| SEC-API-045 | The web client's build must fail on any HTML-string or code-string sink (`dangerouslySetInnerHTML`, `innerHTML`, `outerHTML`, `insertAdjacentHTML`, `document.write`, `eval`, `new Function`, string arguments to timers, `javascript:` URLs), and the only Trusted Types policy must be `gunmetal-loader`, which accepts only same-origin script URLs under the bundle's asset path. | ASVS 3.2.2, 1.3.2, 3.2.3; W3C Trusted Types; CWE-79 | R1 | CI lint (ESLint rules including `react/no-danger` and `react/jsx-no-script-url`, plus restricted-syntax rules); unit and property tests of the policy function; end-to-end test that creating any other policy throws. |
| SEC-API-046 | All text that comes from files, the internet, users, devices or plugins (titles, names, descriptions, biographies, lyrics, tags, playlist and device names) must be rendered as text; rich text must arrive as a validated structure (paragraphs, emphasis and links only) rendered by components, never as HTML or Markdown parsed in the browser. | ASVS 3.2.2, 1.3.1, 1.3.5, 1.2.1; A05:2025; CWE-79, CWE-116 | R1 | End-to-end test over a fixture library whose tags, filenames, lyrics and provider fields carry a cross-site scripting payload corpus, asserting no script runs and the literal text is shown; unit tests of the rich-text validator in core. |
| SEC-API-047 | A URL taken from metadata or user input may be rendered as a link or opened by the app only if a WHATWG-conformant parse gives the scheme `https` or `http` and a non-empty host; anything else must be shown as plain text, and links must open with `rel="noopener noreferrer"`. | ASVS 1.2.2, 3.7.2; CWE-79, CWE-601 | R1 | Property tests of the validator (in core, compiled into the web client through WASM) with `javascript:`, `data:`, `vbscript:`, `file:`, `intent:` and whitespace- or case-obfuscated forms; end-to-end test of a fixture artist with hostile links. |
| SEC-API-048 | Untrusted strings must be normalised on ingest: invalid UTF-8 replaced, control characters removed (keeping tab and newline only in multi-line fields), bidirectional override and isolate characters removed from single-line fields, and each field capped at a declared length. | ASVS 1.1.1, 1.3.3, 2.2.1; CWE-176, CWE-451, CWE-1007; CVE-2021-42574 | R1 | Property tests in core: the normaliser is idempotent, its output contains none of the forbidden code points and never exceeds the cap; a fuzz target. |
| SEC-API-049 | The web client must load no third-party scripts, styles, fonts, images or analytics, and must make network requests only to its own server's origin. | ASVS 3.6.1, 14.2.3; A03:2025; CWE-829 | R1 | CSP (SEC-API-044) plus an end-to-end test that records every network request across all screens and asserts a single origin. |
| SEC-API-050 | Any `message` event listener in the web client must check `event.origin` against an exact list and validate the message against a schema before acting on it. | ASVS 3.5.5; CWE-346 | R1 | CI lint rule that flags listeners without an origin check; unit tests of each listener with wrong origins and malformed messages. |
| SEC-API-051 | Content that comes from users or files must never be served from the application origin with a script-capable type (JavaScript, HTML, XHTML, XML, SVG, PDF); it must be served with a type from a fixed allow-list (`image/jpeg`, `image/png`, `image/webp`, `text/vtt`, `application/json`), `Content-Security-Policy: default-src 'none'; sandbox`, and, where it is offered as a download, `Content-Disposition: attachment` with a filename encoded per RFC 6266. | ASVS 3.2.1, 5.3.1, 5.4.1, 5.4.2; CWE-79, CWE-434 | R1 | Integration test that uploads and scans polyglot fixtures (HTML in a JPEG comment, SVG named `.png`, JavaScript named `.vtt`) and asserts every served response's headers against the allow-list. |
| SEC-API-052 | The web client must check at start-up for the features it depends on (WebAssembly, a secure context on any non-loopback origin) and show an "unsupported browser" page instead of running without them; protections that some engines lack (Trusted Types, Fetch Metadata) must have a server-side or build-time equivalent so security does not depend on the engine. | ASVS 3.1.1, 3.7.5; CWE-693 | R1 | End-to-end test with WebAssembly disabled and on a cleartext non-loopback origin; review of this document's mapping from each browser feature to its fallback. |

### Security headers and caching

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-API-053 | Every response, including errors (401, 404, 413, 429, 500), must carry `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer` and `Cross-Origin-Resource-Policy: same-origin` (except as SEC-API-098 allows); non-HTML responses must carry `Content-Security-Policy: default-src 'none'; frame-ancestors 'none'; sandbox`; HTML responses must also carry `Cross-Origin-Opener-Policy: same-origin` and a `Permissions-Policy` that denies camera, microphone, geolocation, payment, USB, serial, HID and display capture. | ASVS 3.4.4, 3.4.5, 3.4.6, 3.4.8, 3.5.8; API8:2023; A02:2025; CWE-693, CWE-1021 | R1 | Integration test that drives every route into each of its status codes and compares the header set with a literal expected set. |
| SEC-API-054 | Every response with a body must declare a `Content-Type` that matches the body, with `charset=utf-8` for text types; the server must never choose a type from the extension of user-supplied content. | ASVS 4.1.1, 3.4.4; CWE-436 | R1 | Integration test over every route; property test that served types always come from the allow-list. |
| SEC-API-055 | Authenticated JSON responses must carry `Cache-Control: no-store`; capability media responses must carry `Cache-Control: private` with a `max-age` no longer than the URL's remaining life; the router must match paths exactly, so a path with extra segments, an extension, a `;` parameter, a different case or double encoding returns 404. | ASVS 14.2.2, 14.2.5, 14.3.2; RFC 9111; CWE-524, CWE-525 | R1 | Property test that generates suffix, delimiter and encoding variants of every route path and asserts 404; integration test of cache headers per route class. |

### Rate limiting and brute-force protection

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-API-056 | Lockout applies only to guessable secrets (profile PINs, pairing codes, the claim code, share-link passwords): each failure adds a delay from one schedule (30 seconds, 1 minute, 5 minutes, then 15 minutes, never longer and never permanent) keyed per (target, source or device). A pairing code dies after 5 wrong guesses (a new one is free), the claim code is never rotated or invalidated by failures, and a share link is never disabled by failures. After 10 failures the target's owner gets one alert, folded into a daily summary. Passkey and device-key failures are not guesses: they get per-source throttling and an alert, never a disable. | ASVS 6.1.1, 6.3.1, 6.6.3; NIST SP 800-63B-4 §3.2.2; API2:2023, API6:2023; CWE-307 | R1 | Integration test sending 1,000 forged assertions for a real credential ID, after which the credential still signs in; injected-clock tests of the schedule for each secret type; mutation-tested unit test that the lockout table covers only the guessable-secret kinds |
| SEC-API-057 | Rate limits must be keyed on the principal when there is one and otherwise on the address key derived under SEC-NET-052 (hierarchical IPv6 prefixes); unauthenticated routes must also have a server-wide ceiling; limited requests must receive 429 with `Retry-After` and a body that does not reveal whether an account, code or link exists. | ASVS 2.4.1, 15.3.4, 6.3.8; RFC 6585 §4; API4:2023; CWE-770, CWE-799 | R1 | Property tests of key derivation in core (every address in a /64 maps to one key, distinct /64s do not); integration tests of per-key and global limits. |
| SEC-API-058 | Sign-in, pairing and invitation failures must be indistinguishable in status and body whether the account or code exists, is disabled or is wrong, and the server must run the same verification work in each case. | ASVS 6.3.8; CWE-204, CWE-208 | R1 | Integration test comparing literal responses for each failure cause; unit test that the verification path performs the dummy verification when the record is missing; code review of constant-time comparisons. |
| SEC-API-059 | **Withdrawn 2026-10-02: merged into SEC-IAM-056, SEC-IAM-058, SEC-IAM-060.** One pairing specification. | ASVS 6.5.1, 6.5.5, 6.6.2, 6.6.3, 7.6.2; RFC 8628 §5.1, §5.4; CWE-451 | Withdrawn | Proved by the tests of SEC-IAM-056, SEC-IAM-058, SEC-IAM-060 |

### Request size and complexity

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-API-060 | The server must enforce, before buffering, a header section of at most 16 KiB and 64 fields (431), a request target of at most 4 KiB (414), a JSON body of at most 64 KiB unless the route declares a higher cap (413), JSON nesting of at most 32 levels, arrays of at most 1,000 elements unless the route declares more, and per-field string caps (400). | ASVS 5.2.1, 2.2.1, 15.2.2; API4:2023; CWE-400, CWE-770 | R1 | Integration tests at each limit and one past it; property tests of the depth check in core. |
| SEC-API-061 | The server must time out header reading after 10 seconds, idle request bodies after 30 seconds and non-streaming handlers after 30 seconds, close streaming responses whose client stops reading for 60 seconds, and cap connections in total and per client address. | ASVS 15.2.2, 13.1.3; API4:2023; CWE-400 | R1 | Integration tests with deliberately slow clients using test-scaled timeouts. |
| SEC-API-062 | If HTTP/2 is enabled, the server must cap concurrent streams per connection at 100, cap header-list size and continuation frames, and close connections that reset streams faster than a set threshold. | ASVS 15.2.2; API4:2023; CWE-400; CVE-2023-44487 | R1 | Integration test with an HTTP/2 client that floods resets and continuation frames and asserts `GOAWAY`; CI advisory scan of the HTTP stack (cargo-deny). |
| SEC-API-063 | List and search routes must cap page size at 500 unless the route declares more, cap search input at 256 characters and 16 terms, and pass search text to SQLite FTS5 only as escaped quoted terms, never as query syntax. | ASVS 4.3.1, 1.2.4; API4:2023; CWE-89, CWE-400 | R1 | Property test that no input string produces an FTS syntax error and that results equal a literal-term reference search; integration tests of the caps. |
| SEC-API-064 | Expensive operations (library scans, metadata refreshes, artwork re-fetches, exports, and transcodes in R2) must be limited to authorised principals, run at most one at a time per kind and per principal, and treat a repeated request as joining the running job. | ASVS 15.1.3, 15.2.2, 2.4.1; API4:2023, API6:2023; CWE-770 | R1 | Integration test that fires many concurrent requests and asserts one job and identical job identifiers in every response. |
| SEC-API-065 | Request bodies with any `Content-Encoding` must be refused with 415; the server must not decompress request bodies. | ASVS 5.2.3; CWE-409 | R1 | Integration test with gzip, deflate and br request bodies. |
| SEC-API-066 | All SQL text must be static: the storage layer must accept query text only as `&'static str` with bound parameters, and dynamic sorting or filtering must be chosen from enums. | ASVS 1.2.4; A05:2025; CWE-89 | R1 | The type signature makes dynamic SQL a compile error; CI check for string formatting inside the storage module; property tests that put SQL metacharacters in every string field. |

### Input validation and response shaping

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-API-067 | Every body, query and path input must decode into a typed request structure that rejects unknown fields, duplicate JSON keys, the same parameter given twice, and the same parameter given in more than one location, with explicit bounds on every field. | ASVS 2.2.1, 1.5.3, 15.3.3, 15.3.7; API3:2023; CWE-915, CWE-20; CVE-2017-12635 | R1 | Property tests per request type (unknown field, duplicate key, duplicate parameter and cross-location duplicates each give 400); fuzz targets for request decoding. |
| SEC-API-068 | Responses must be built from explicit per-role response types, never by serialising storage records; filesystem paths, library-root locations, other users' identities, addresses and devices, and admin settings must appear only in admin response types. | ASVS 15.3.1, 8.2.3, 8.1.2, 14.2.6; API3:2023; CWE-200, CWE-359 | R1 | Snapshot test of every response schema per role against literal files; a test that serialises fixture responses for non-admin roles and asserts no value looks like a path or an IP address. |
| SEC-API-069 | Absolute URLs the server generates (invitation and share links, OIDC redirect URIs, links in notifications) must be built from the configured public URL, never from `Host`, `X-Forwarded-Host` or `Forwarded`. | ASVS 4.1.3; CWE-807, CWE-346 | R1 | Integration test that sends forged `Host` and forwarding headers to every link-generating route and asserts the configured origin in the result. |
| SEC-API-070 | A post-sign-in return target must be a relative path that starts with a single `/` and matches a known client route; absolute URLs, protocol-relative values, backslashes and anything with a scheme must be replaced by the home route. | ASVS 3.7.2; CWE-601 | R1 | Property test of the validator against an open-redirect corpus; end-to-end test of sign-in with hostile return targets. |
| SEC-API-071 | Any CSV export must quote fields as RFC 4180 describes and must neutralise cells that begin with `=`, `+`, `-`, `@`, tab or carriage return. | ASVS 1.2.10; RFC 4180; CWE-1236 | R1 | Property test over generated titles. |

### Errors

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-API-072 | Every error must be an RFC 9457 problem-details object whose `type` comes from a closed enumeration, whose `title` and `detail` come from a fixed catalogue, and which carries a request identifier; validation errors may name the field path and the rule broken but must not echo the submitted value; no error may contain a stack trace, panic message, SQL, filesystem path, hostname, dependency name or version. | ASVS 16.5.1, 13.4.2, 13.4.6; RFC 9457; A10:2025; CWE-209, CWE-497 | R1 | Fuzz and property tests that send malformed input to every route and assert the body matches the problem schema and contains no forbidden substrings; unit test that the catalogue is an exhaustive enumeration. |
| SEC-API-073 | A last-resort layer must turn any panic or unexpected error in request handling into a generic 500 problem, log the details server-side under the request identifier, roll back the request's transaction and grant nothing. | ASVS 16.5.3, 16.5.4, 2.3.3; A10:2025; CWE-209 | R1 | Integration test using a route that exists only in test builds and panics mid-transaction, asserting the response, the log entry and an unchanged database. |

### Forwarded headers and network location

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-API-074 | **Withdrawn 2026-10-02: merged into SEC-NET-016.** One forwarding-header rule. | ASVS 4.1.3, 15.3.4; RFC 7239; A01:2025; CWE-348, CWE-290; CVE-2023-33193 | Withdrawn | Proved by the tests of SEC-NET-016 |
| SEC-API-075 | No authentication or authorisation decision may grant access, skip a factor or relax a limit because of network location; location may only add a restriction (such as "home network only" for a user), and must be derived from the transport or from SEC-NET-016, never from a header directly. | ASVS 8.4.2, 8.1.3, 8.1.4; A07:2025; CWE-290, CWE-807 | R1 | Property test over the policy function: for every policy and every location, an absent or invalid credential yields deny, and changing location never turns deny into allow; review of the documented attribute list required by ASVS 8.1.3. |

### Outbound requests and server-side request forgery

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-API-076 | Every outbound network request made by the server or a plugin must go through a single egress crate, and CI must fail if any other crate depends on an HTTP client or opens outbound sockets. | ASVS 13.2.4, 13.2.5, 13.1.1; API7:2023, API10:2023; A01:2025; CWE-918 | R1 | CI check using cargo-deny dependency bans and clippy `disallowed_types` and `disallowed_methods` rules. |
| SEC-API-077 | The egress client must resolve names itself and refuse the request if any resolved address is not globally reachable under the IANA IPv4 and IPv6 special-purpose registries (including loopback, private, shared 100.64.0.0/10, link-local such as 169.254.169.254, unique-local, multicast, documentation, benchmarking, reserved and unspecified) or embeds such an address (IPv4-mapped, 64:ff9b::/96, 64:ff9b:1::/48, 6to4, Teredo); it must connect only to the address it checked and send the original hostname as `Host` and TLS SNI. | ASVS 1.3.6, 13.2.4; RFC 6890; API7:2023; CWE-918, CWE-367 | R1 | Property tests of the address classifier in core against an independent oracle built in the test from the IANA registry files; integration test with a local DNS stub that answers a public address then a private one, proving there is no second resolution. |
| SEC-API-078 | The egress client must allow only `https` (and `http` only for admin-entered LAN targets), only ports 443 and 80 unless the purpose names another, follow no redirects unless the purpose allows them (then at most 3, each re-checked), always verify TLS certificates with no option to disable it, cap response size per purpose, and enforce connect and total timeouts. | ASVS 15.3.2, 12.3.2, 13.2.6, 16.5.2; API10:2023; CWE-918, CWE-295 | R1 | Integration tests against local test servers: a redirect to 127.0.0.1, an oversized body, a slow body, a self-signed certificate and a disallowed port, each refused. |
| SEC-API-079 | Each outbound purpose (metadata provider, cover art, lyrics, update check, OIDC discovery, scrobbling, webhook, M3U source) must declare its allowed hosts, and requests outside them must be refused and logged; private-network targets may be allowed only for purposes that need them and only to the exact host and port an admin entered, never to loopback or link-local addresses. | ASVS 13.2.4, 13.1.1, 1.3.6; API7:2023, API10:2023; CWE-918 | R1 | Unit tests of each purpose's policy; integration tests of refusals and of the admin-entered LAN exception. |
| SEC-API-080 | The server must never fetch a URL that a user, a file, an identity provider or a metadata response supplies (profile-picture claims, URLs in tags, links in playlists, SVG references) as a side effect of a request; remote artwork must come only from allow-listed provider hosts through the egress client and be served from local storage. | ASVS 1.3.6, 13.2.4; API7:2023; CWE-918 | R1 | Integration tests that scan fixture files with URL-bearing tags and sign in with an OIDC fixture that has a picture claim, asserting the egress client records zero requests. |
| SEC-API-081 | Responses from external services must be decoded into typed structures with size limits, normalised as SEC-API-048 requires, and never interpreted as HTML, templates, commands or file paths. | ASVS 1.5.2, 2.2.1, 1.3.7; API10:2023; CWE-20 | R1 | A cargo-fuzz target per provider response decoder; property tests of each decoder with hostile fields. |
| SEC-API-082 | M3U and XMLTV sources and the stream URLs they contain must be fetched only through the egress client under the M3U purpose policy; `file:` and every scheme other than `http` and `https` must be refused, and admin-approved LAN tuners must be matched by exact host and port. | ASVS 1.3.6, 5.3.2; API7:2023; CWE-918, CWE-22 | R3 | Integration tests with playlists containing `file:`, loopback, link-local and unapproved LAN entries. |
| SEC-API-083 | Webhooks must be configurable only by admins, delivered through the egress client under the webhook purpose, signed with HMAC-SHA-256, retried a bounded number of times, and must never show the receiver's response body back to the admin. | ASVS 13.2.4, 4.1.5; API7:2023; CWE-918 | Later | Integration tests of signing, retry limits, and that only a status code is recorded. |
| SEC-API-084 | A plugin's network grants must be enforced by the egress client from the plugin's declared purposes, not by the plugin. | ASVS 13.2.4, 15.2.5; API10:2023; CWE-918 | Later | Integration test with a test plugin that attempts requests outside its grant. |

### Uploads and file-derived content

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-API-085 | Each upload route must declare its allowed types, maximum bytes and, for images, maximum pixel dimensions; the server must decide the type from magic bytes and ignore the client's filename and `Content-Type` for every storage decision. | ASVS 5.1.1, 5.2.1, 5.2.2, 5.4.1; A06:2025; CWE-434 | R1 | Integration tests with mismatched extensions, polyglots, oversized files and truncated files. |
| SEC-API-086 | Images, whether uploaded, embedded in media files or fetched from providers, must be decoded only by memory-safe Rust decoders with width, height (8,192 pixels each) and total pixel (40 megapixels) limits applied before allocation, then re-encoded to a server-chosen format with all metadata removed; SVG must be refused, and no image may be handed to ImageMagick or another C toolkit in the server process. | ASVS 5.2.6, 1.3.4, 14.2.8, 15.2.5; API4:2023; CWE-400, CWE-434; CVE-2016-3714 | R1 | A cargo-fuzz target for the image pipeline; unit tests with headers that declare huge dimensions; property test that re-encoded output contains no EXIF, XMP or text chunks. |
| SEC-API-087 | Stored uploads and extracted artwork must be named by a server-generated content hash inside a server-controlled directory; no request data may become part of a filesystem path anywhere in the HTTP layer. | ASVS 5.3.2, 5.3.1; A01:2025; CWE-22, CWE-73 | R1 | Clippy `disallowed_methods` rules for filesystem and path-joining calls outside the storage module (CI check); integration tests with traversal filenames. |
| SEC-API-088 | Uploads must be limited per principal by count and total bytes. | ASVS 5.2.4; API4:2023; CWE-770 | R1 | Integration test that uploads past each quota. |
| SEC-API-089 | Uploaded and sidecar subtitles must be parsed by the core into a cue model with size and cue-count limits and re-serialised as WebVTT, and must never be stored or read by a user-supplied name. | ASVS 5.2.2, 5.3.2, 1.3.5; CWE-22, CWE-79; CVE-2026-35031 | R2 | A cargo-fuzz target for each subtitle parser; property tests that parsing then serialising is stable; integration tests with traversal names. |
| SEC-API-090 | Lyrics from tags, sidecar files, uploads or providers must be parsed into a timed-line model capped at 256 KiB and 10,000 lines and rendered only as text. | ASVS 2.2.1, 3.2.2; CWE-79, CWE-400 | R1 | A cargo-fuzz target for the LRC parser; property tests of the caps; covered by the payload corpus in SEC-API-046. |

### Inventory, versions and compatibility adapters

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-API-091 | The OpenAPI description must be generated from the route table at build time and CI must fail if the committed description differs; release builds must contain no route absent from it, including no debug or test routes. | ASVS 13.4.2, 13.4.5, 15.2.3; API9:2023; CWE-1059 | R1 | CI diff check; a test that compares the release build's route list with the description. |
| SEC-API-092 | The native API must be versioned in its path (`/api/v1`), and routes removed from a version must stop answering rather than linger. | API9:2023; ASVS 15.2.3 | R1 | Integration test that every removed route listed in the literal inventory history returns 404. |
| SEC-API-093 | Compatibility adapters (OpenSubsonic, Jellyfin) must be off by default, served under their own path prefixes, listed in the inventory, and pass through the same route table, policy layer and cross-principal tests as the native API. | ASVS 6.3.4, 8.2.1; API9:2023; CWE-862 | R2 | The cross-principal and matrix tests (SEC-API-011, SEC-API-019) extended to adapter routes; integration test that adapters answer 404 until enabled. |
| SEC-API-094 | A credential that an adapter protocol requires in a query string must be a per-app random secret of at least 128 bits, scoped to read and play, revocable, never the account's sign-in credential, and redacted from all logs. | ASVS 14.2.1, 16.2.5, 8.2.1; API2:2023; CWE-598, CWE-532 | R2 | Integration tests of scope and revocation; log-capture property test for redaction. |

### Logging at the API boundary

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-API-095 | The access log must record method, route template (never a raw path containing a capability token), status, principal identifier, derived client address and request identifier, and log failed authorisation and authentication; it must never record cookies, `Authorization`, capability tokens, setup or pairing codes, invitation secrets or bodies, and must encode logged strings to prevent log injection. | ASVS 16.2.1, 16.2.5, 16.3.1, 16.3.2, 16.4.1; A09:2025; CWE-532, CWE-117 | R1 | Property test of the log formatter: no generated secret appears in output and control characters are escaped; integration test that a forbidden-object request produces an authorisation-failure entry. |

### Invitations, share links and casting

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-API-096 | An invitation link must carry a secret of at least 128 bits in the URL fragment, which the landing page sends in a `POST` body to redeem; invitations must expire (7 days by default), have a use count, be revocable, and their redemption must be rate-limited. | ASVS 6.4.1, 11.5.1, 14.2.1; API6:2023; CWE-598, CWE-613 | R1 | Integration tests of expiry, use count, revocation and rate limit; access-log test that the secret never reaches the server in a request line. |
| SEC-API-097 | Public share links (R1 for music; video links off until the owner enables them in R2) must use the fragment pattern, carry at least 128 bits, be scoped to one object and its rights (listen-only by default; downloads only when the owner allows them server-wide), take their owner from the session, expire (30 days by default), support an optional password under SEC-STD-008 and SEC-API-056, be visible and editable only by their owner and admins, stop working on the next request after revocation, send no link-preview metadata unless the sharer turns it on, and enforce per-link limits: 2 concurrent streams by default, a total-bytes or uses cap, and a distinct-address count that suspends the link and alerts the sharer when exceeded. Managed profiles cannot create them. | ASVS 8.2.2, 8.3.2, 14.2.1; API1:2023, API4:2023, API6:2023; CWE-639, CWE-613, CWE-770 | R1 | Cross-principal and revocation integration tests as for SEC-API-011 and SEC-API-028; integration test that preview tags are absent by default; integration test of each per-link limit and of the suspension alert |
| SEC-API-098 | Signed media routes may send `Access-Control-Allow-Origin: *` (never with credentials) and `Cross-Origin-Resource-Policy: cross-origin` only for the representations a cast receiver needs. | ASVS 3.4.2, 3.5.8; CWE-942 | R2 | Integration test that only cast representations carry these headers. |
| SEC-API-099 | Document formats (EPUB, PDF and similar) must render only on a separate sandbox origin (a different hostname that receives no cookies) or after conversion to a safe model by the core, never on the application origin. | ASVS 3.2.1, 3.5.8; CWE-79 | Later | Integration test of origins and headers once the feature exists; manual review of the feature design |

**Bound by requirements in [standards-coverage.md](standards-coverage.md).** SEC-STD-008 (share-link passwords), SEC-STD-013 (no JSONP or script inclusion of data), SEC-STD-014 (cookie inventory), SEC-STD-015 (external links), SEC-STD-016 (HSTS preload for project domains), SEC-STD-017 (static assets only from a manifest), SEC-STD-038 (dynamic API fuzzing) and SEC-STD-040 (Unix sockets, never TCP, to workers).
The 2026-10-02 challenge review merged duplicated controls into one owner each; withdrawn rows above say where their content went, and [threat-model.md](threat-model.md#control-ownership) lists every owner.

## Design guidance

### Where each piece lives

- **`gunmetal-core`** (pure, no I/O) gets everything decidable without I/O:
  the identifier codec, the capability-URL token format and its MAC
  verification, the rate-limit arithmetic (a GCRA or token bucket over an
  injected clock), the IP address classifier, `Forwarded` and
  `X-Forwarded-For` chain parsing, the string normaliser, the URL validator,
  the redirect-target validator, the rich-text structure and its validator,
  the JSON depth pre-check, the LRC and subtitle parsers, the path
  containment check, and the problem-type catalogue. These are exactly the
  functions a property test and a mutation tool can pin down, and record 1
  already compiles the core into the web client through WASM, so the URL
  validator and the rich-text validator run identically on both sides.
- **The server crate** holds the route table, the middleware chain, the
  authorisation layer over SQLite, the session store and the media handlers.
- **A separate egress crate** holds the only outbound HTTP client. Keeping it
  separate is what makes the CI ban in SEC-API-076 enforceable.
- No web framework has been chosen. axum on hyper and tower fits the design
  (it already defaults to a 2 MB body limit on its byte-based extractors,
  which SEC-API-060 tightens), but anything with a composable middleware
  chain and an enumerable router will do.

### The route table

Build the router only from a static table, so tests can enumerate it and
nothing can be added beside it:

```rust
pub enum Access {
    Public,                          // must appear on the allow list
    Exchange(ExchangeKind),          // setup code, invite, refresh, pairing, ceremony
    Capability(CapabilityKind),      // signed media URL, WebSocket ticket
    User(Capability),                // signed-in principal holding the capability
    Admin { cap: AdminCapability, step_up: bool },
}

pub struct RouteSpec {
    pub method: Method,
    pub path: &'static str,          // template, matched exactly
    pub access: Access,
    pub mutates: bool,
    pub body: BodyRule,              // None, Json { max }, Upload { types, max, pixels }
    pub rate: RateClass,
    pub page_cap: Option<u32>,
}
```

The OpenAPI document, the allow-list test, the role matrix test, the method
test and the header test all read this table. Per-route overrides (a larger
body for a playlist import, a larger page for sync) are declared here, so
they show up in the API description and in review.

### Request pipeline

Order matters; each step runs before anything more expensive:

1. Connection limits and timeouts (SEC-API-061, SEC-API-062).
2. Framing checks and size limits on the header section and target
   (SEC-API-009, SEC-API-060).
3. `Host` allow-list (SEC-API-007).
4. Route match, exact; then the method check (SEC-API-055, SEC-API-008).
5. Credential extraction: at most one credential, never from the URL
   (SEC-API-004). Capability routes verify the token instead (SEC-API-026).
6. For cookie credentials: the `Gunmetal-Request` header, then
   `Sec-Fetch-Site`, then `Origin` (SEC-API-033, SEC-API-034).
7. Authenticate; derive the principal; uniform 401 on failure
   (SEC-API-003).
8. Access class and capability check from the route table; step-up check
   (SEC-API-019, SEC-API-021).
9. Rate limit (SEC-API-057).
10. Content-type check and body decoding into the typed request
    (SEC-API-035, SEC-API-060, SEC-API-065, SEC-API-067).
11. Handler, which reaches data only through the authorisation layer
    (SEC-API-010).
12. Response headers layer (SEC-API-053 to SEC-API-055), which also applies
    to errors.
13. Error mapper and last-resort handler (SEC-API-072, SEC-API-073).

### The principal and the authorisation layer

```rust
pub struct Principal {
    pub account: AccountId,
    pub profile: ProfileId,
    pub device: DeviceId,
    pub session: SessionHandle,      // random 64-bit handle, never the token
    pub credential: CredentialKind,  // cookie session, device key, app credential, share
    pub scope: Scope,                // intersection rule of SEC-API-020
    pub verified_at: Timestamp,      // last user verification, for step-up
}
```

The authorisation layer is a set of reader and writer handles created from
a principal, for example `Library::for_principal(&p)`. Its queries always
include the principal's visibility predicate (library grants plus profile
restrictions), so a forgotten filter is impossible rather than unlikely.
Objects come back wrapped in `Visible<T>` or `Editable<T>`, whose
constructors are private to the layer. Rules:

- Not visible means 404, exactly like missing. Visible but not permitted
  (editing a playlist shared read-only) means 403.
- Batch inputs are checked in one query that returns the visible subset;
  if its size differs from the input, reject the whole request.
- Server-pushed events are built per recipient by the same layer: an event
  is a description ("playlist X changed"), and the fan-out asks the layer,
  for each subscriber, whether and how that subscriber may see it.
- Authorisation state that the hot path needs (session revoked, profile
  visibility) is cached in memory and invalidated by the write that changes
  it, never by a timer. Correctness first: if the cache cannot be kept
  exact, query SQLite; a lookup by primary key is cheap.

### Identifiers

Use 16 bytes, encoded as 26 lowercase Crockford base32 characters with a
kind prefix (`trk_`, `alb_`, `art_`, `pls_`, `usr_`, `prf_`, `dev_`, `shr_`,
`inv_`). The prefix makes kind confusion a parse error and makes leaked
identifiers easy to recognise in logs. For user-created objects, generate
the bytes from the operating system CSPRNG. For library items, the choice
between random-and-persisted and keyed derivation is open decision 3.
Either way an identifier is not a secret and not an authorisation: Jellyfin's
CVE-2023-49096 was only hard to exploit because item identifiers happened to
be random GUIDs, which is defence in depth, not a control.

### Capability URLs for media and images

Token layout (base64url, about 72 characters):

| Field | Bytes | Notes |
|---|---|---|
| version | 1 | Lets the format change without ambiguity |
| key identifier | 1 | Selects the MAC key during rotation overlap |
| expiry | 8 | Unix seconds |
| operation | 1 | stream, download, image, subtitle, lyrics |
| representation | 2 | original, a remux profile (R2), an artwork size from a fixed set |
| object identifier | 16 | The item, image or subtitle |
| session handle | 8 | Looked up on every request (SEC-API-028) |
| tag | 16 | HMAC-SHA-256 over a label such as `gunmetal-media-v1` and all fields above, truncated |

The route is `/m/v1/<token>`, and for segmented delivery in R2
`/m/v1/<token>/seg/<n>`, where `n` is checked against the stored segment map,
so one token covers a whole playback instead of one per segment. Verify in
this order: parse, MAC (constant time), expiry, session alive, object still
visible. The filename shown to the user comes from the database in
`Content-Disposition`, never from the URL. Artwork sizes are a short fixed
list so nobody can make the server resize to arbitrary dimensions. Artwork
expiry is rounded up to a 6-hour boundary so a TV scrolling a grid reuses
URLs and its HTTP cache. Clients refresh URLs for upcoming queue items in
the background and ask for a fresh one when they get `media-url-expired`.
Live streams in R3 have no duration; give them a short expiry and let the
client refresh while playing.

### Browser sessions and request forgery

The web client is served by the server, from the same origin as the API,
out of an asset bundle embedded in the binary (so there is no filesystem
path to traverse and no directory to list). It uses the `__Host-` cookie.
The fetch wrapper adds `Gunmetal-Request: 1` to every call. Why each layer
exists:

- The custom header works on plain HTTP, on old TV engines and when
  browsers omit `Sec-Fetch-*` (they send it only to secure origins). A
  cross-origin page cannot add a non-safelisted header without a CORS
  preflight, and the API answers no preflights for cookie requests.
- `SameSite=Lax` is site-scoped, not origin-scoped. Home labs commonly run
  many apps on sibling subdomains of one domain, and any one of them with a
  cross-site scripting bug is "same-site" to Gunmetal. That is why the
  server checks `Sec-Fetch-Site` and `Origin` exactly instead of relying on
  `SameSite`, and why the cookie uses `__Host-` so a sibling cannot set or
  overwrite it.
- `Lax` rather than `Strict` keeps a link pasted into a chat app from
  opening the web client signed out; all `GET` routes are safe, so `Lax`
  costs nothing.
- DNS rebinding defeats a custom header on its own (Transmission's
  CVE-2018-5702 relied on a non-forbidden header), because after rebinding
  the attacker's page is "same-origin" with itself. The `Host` allow-list
  closes that, and the victim's cookie is never sent to the attacker's
  origin anyway, so rebinding can only reach unauthenticated routes, which
  is why the setup window needs its code.

Native apps and the cross-origin TV builds (Samsung and LG in R2 run a
packaged web build from their own origin) use bearer credentials in the
`Authorization` header. Whether those packaged apps are subject to CORS at
all differs by platform (unverified); either way the API answers only the
admin-listed origins (SEC-API-040). Whether bearer tokens should be
sender-constrained (DPoP, RFC 9449, or HTTP Message Signatures, RFC 9421)
belongs to the identity document; RFC 9700 recommends sender-constraining
or rotation for public clients.

### Transport for browsers

Browsers grant `Secure` cookies, WebAuthn, Fetch Metadata and
`Clear-Site-Data` only to secure contexts, and `localhost` counts as one.
So:

- First run on the server's own machine works over `http://localhost`.
- Everything else needs HTTPS with a certificate the browser trusts, which
  is open decision 1. Until it is decided, SEC-API-037 makes the server
  explain this rather than downgrade.
- Native clients over iroh are encrypted by the transport and are not
  affected.

### Content Security Policy and Trusted Types in practice

- `'wasm-unsafe-eval'` is required for the WASM core and permits only
  WebAssembly compilation, not JavaScript `eval`.
- CSP Level 3 lets `'self'` match `wss:` for the page's host, so
  `connect-src 'self'` covers the event socket. Older engines may need the
  explicit `wss://host` form (which ones is unverified); the CSP test runs
  on the oldest engine we support.
- `script-src 'self'` is an allow-list, which ASVS 3.4.3 accepts at Level 2.
  Its weakness is that anything served from the origin with a script type
  becomes trusted; SEC-API-051 is what makes `'self'` safe. Level 3 asks for
  nonces or hashes per response; with no inline scripts at all and an
  immutable embedded bundle, we treat this as a documented deviation.
- The bundler's chunk loader and worker creation assign script URLs, which
  are Trusted Types sinks. Create one policy, `gunmetal-loader`, whose
  `createScriptURL` accepts only URLs that resolve to the page's origin and
  to paths under the bundle's asset prefix ending in `.js`, and throws on
  anything else. No `createHTML` and no default policy. Whether the chosen
  bundler (Metro for web, or webpack) can route its loader through a named
  policy without patches is unverified; check this before committing to a
  bundler.
- react-native-web injects its styles at run time. Whether that works under
  `style-src 'self'` without `'unsafe-inline'` is unverified (browsers do
  not block stylesheet insertion through the CSSOM in practice, but the
  `<style>` element it creates may still be checked). The CSP end-to-end
  test settles it; open decision 7 covers the fallback.
- No CSP reporting endpoint in R1 (open decision 6). Violations are caught
  in CI by the end-to-end test, which listens for `securitypolicyviolation`.
- If the WASM core ever needs threads, `SharedArrayBuffer` requires
  cross-origin isolation (`Cross-Origin-Embedder-Policy: require-corp` with
  `Cross-Origin-Opener-Policy: same-origin`); the headers above already
  make that a one-line change.
- Trusted Types became Baseline in February 2026 (Chrome and Edge since
  2020, Safari 26 in September 2025, Firefox in February 2026). Old smart-TV
  engines will ignore the directive, which is why SEC-API-045 makes the
  build, not the browser, the primary guarantee.

### Rendering untrusted text

- React escapes text children and attribute values, so the rule is simply:
  never use a sink that parses HTML. A `SafeText` component is not needed
  for escaping, but render normalised strings (SEC-API-048) and apply
  `dir="auto"` so right-to-left titles display correctly without bidi
  controls.
- Artist biographies and similar rich text from providers are converted by
  the server into a tiny structure: `Paragraph(Vec<Inline>)` where an inline
  is text, emphasis, strong or a link with a validated URL. The client
  renders it with components. There is no Markdown or HTML parser in the
  browser.
- React's own handling of `javascript:` URLs has changed between versions
  (later versions are reported to block them; unverified for the version we
  will use, and for react-native-web's link component), so the client
  validates every external URL itself with the shared validator. On native
  platforms the same validator gates `Linking.openURL`, which would
  otherwise open `intent:` or `file:` URLs on Android.
- Device names, user names and share recipients shown on approval and
  admin screens are labelled as claimed names, and shown next to facts the
  server knows (client type, LAN or remote, time). This is what makes
  device-code phishing (Storm-2372, February 2025) visible to the person
  approving.

### Rate limiting that families will not switch off

| Class | Key | Default (to be tuned by measurement) | Notes |
|---|---|---|---|
| Sign-in ceremonies (passkey, OIDC) | client address and server-wide | Generous; no per-account lock | Passkeys cannot be guessed; per-account locks would only help an attacker lock out the owner |
| Short secrets (setup code, pairing code, PIN, share password) | the target | 5 failures, then invalidate or parent unlock | Per target, so an attacker rotating addresses gains nothing |
| API reads | principal | High burst for scrolling and sync | Libraries of tens of thousands of items must sync quickly |
| API writes | principal | Moderate | |
| Images | connections per session | No per-request limit | A TV grid makes hundreds of requests a minute |
| Media streams | concurrent streams per session and principal | Set by the admin | Also the account-sharing control from the research document |
| Expensive jobs | one per kind per principal | Joins the running job | |

Return 429 with `Retry-After`. IPv6 clients are grouped by /64 because one
household or attacker usually controls a whole /64. Never show a CAPTCHA.

### Forwarded headers and reverse proxies

The configuration has `trusted_proxies`, a list of CIDR ranges, empty by
default. When the TCP peer is in it, parse `Forwarded` (RFC 7239) or
`X-Forwarded-For`, walk from the right, skip addresses that are themselves
trusted proxies, and take the first one that is not. Use that single value
everywhere. Documentation should give tested recipes for Caddy, nginx,
Traefik, Tailscale serve and Cloudflare Tunnel. Two traps to call out:

- `cloudflared` and similar tunnels connect from loopback. Trusting
  127.0.0.1 then trusts every local process; prefer a Unix socket or a
  dedicated address for the tunnel.
- If many requests arrive with forwarding headers from an untrusted peer,
  show the admin a hint ("this server seems to be behind a proxy; set
  trusted proxies so rate limits see real addresses"). Never trust
  automatically.

### The egress client

```rust
pub enum Purpose { CoverArt, Metadata(Provider), Lyrics(Provider), UpdateCheck,
                   OidcDiscovery, Scrobble(Plugin), Webhook, M3uSource }

pub fn fetch(purpose: Purpose, url: &ValidatedUrl) -> Result<Response, EgressError>;
```

- Parse with a WHATWG-conformant URL parser (the `url` crate aims at this;
  unverified here) and classify the parsed host, never the string, so
  oddities such as `0x7f.1` or a bare decimal address are normalised before
  classification.
- Resolve with the server's own resolver, classify every A and AAAA answer,
  and connect to one checked `SocketAddr` with a custom connector, passing
  the hostname for SNI and `Host`. Do not let the HTTP library resolve
  again.
- The classifier blocks everything the IANA registries mark as not globally
  reachable, plus multicast and the limited broadcast address, and unwraps
  IPv4-mapped, NAT64, 6to4 and Teredo addresses before classifying. Note
  that 100.64.0.0/10 is where Tailscale addresses live; a tuner reachable
  over Tailscale is a LAN target an admin must enter explicitly, like any
  other private address.
- TLS uses rustls with the platform or webpki roots; there is no setting to
  disable verification (Immich shipped OIDC fetches with verification off in
  2026).
- Proxy environment variables are not read implicitly; an egress proxy, if
  ever supported, is explicit configuration.
- Every refusal is logged with the purpose and the reason, which also helps
  admins understand why a provider is not working.

### Uploads and embedded images

Stream the body to a temporary file with the route's byte cap. Sniff the
magic bytes. Read the image header and reject declared dimensions over the
caps before decoding anything. Decode with the image crate's limits set
explicitly: its defaults have no width or height limit and allow 512 MiB of
allocation, which is far too generous. Re-encode to the server's chosen
format and size set, which drops EXIF (including GPS position), XMP, text
chunks and any polyglot payload. Hash the result and store it under the hash.
The scanner uses the same pipeline for artwork embedded in music files, so
untrusted bytes from a downloaded album take exactly the same path as an
upload. ASVS 5.4.3 asks for antivirus scanning of untrusted files; we
deviate because re-encoding images and re-serialising subtitles and lyrics
from parsed models removes active content more reliably than signature
scanning, and this deviation should be recorded.

### Errors

Map every error to a variant of one enumeration: not found, unauthenticated,
forbidden, reauthentication required, rate limited, payload too large,
unsupported media type, invalid (with field path and rule, both from
enumerations), conflict, media URL expired, internal. The client maps each
`type` to a localised, friendly sentence, so security does not cost
helpful messages; the server never builds a message from input.

### Test catalogue

| Suite | Kind | Requirements it proves |
|---|---|---|
| `route_inventory` | Unit and integration | 001, 002, 008, 036, 091 |
| `credential_states` | Integration | 003, 004, 029 |
| `authz_matrix` (route by role, literal file) | Integration | 019, 020, 021, 093 |
| `cross_principal_ids` (generated) | Integration | 011, 012, 013, 014, 024, 025, 097 |
| `revocation` | Integration | 017, 027, 028, 039 |
| `events_fanout` | Integration | 016, 041, 042, 043 |
| `csrf_matrix` | Integration | 032 to 035, 040 |
| `headers_every_response` | Integration | 005, 038, 051, 053 to 055 |
| `limits_and_timeouts` | Integration | 031, 060 to 065 |
| `egress` (local DNS stub and test servers) | Integration, property | 076 to 082 |
| `uploads` | Integration, fuzz | 085 to 090 |
| `web_security_e2e` (Chromium, Firefox, WebKit) | End-to-end | 032, 039, 044 to 052, 059, 070 |
| Fuzz targets | cargo-fuzz | Token parser, request decoding, forwarded-chain parser, cursor decoder, provider decoders, image header reader, LRC and subtitle parsers, WebSocket message decoder |
| CI checks | Lints and diffs | cargo-deny bans and advisories, clippy disallowed methods and types, ESLint sink rules, OpenAPI diff, allow-list CODEOWNERS |

The policy module, the token verifier and the address classifier are the
code where a surviving mutant is a security hole. The matrix and
cross-principal suites are what kill those mutants, which is a good reason
to write them before the first handler.

### ASVS deviations to record

- **3.4.1** (Level 2 wants `includeSubDomains`): only for names we issue or
  the admin vouches for, because a user's own domain often hosts other
  services that may not support HTTPS.
- **3.4.3** (Level 3 wants per-response nonces or hashes): an allow-list of
  `'self'` with no inline script and an embedded bundle, see above.
- **3.4.7** (Level 3 wants a CSP report location): none in R1, to keep the
  unauthenticated surface minimal.
- **3.7.4** (HSTS preload): not applicable to user-owned domains.
- **5.4.3** (antivirus scanning): replaced by content disarm through
  re-encoding.

## Anti-patterns

Things we must never do, and where each lesson came from.

1. **Leave stream, image or subtitle endpoints unauthenticated so that
   players work.** Jellyfin issue 5415 (March 2021) catalogued
   unauthenticated stream, subtitle, image, user and system-information
   endpoints. Jellyfin's CVE-2023-49096 then put an FFmpeg argument
   injection behind one of those open stream endpoints; only the randomness
   of item identifiers stood in the way. Capability URLs solve the player
   problem without opening the route.
2. **Put API keys or session tokens in URLs, or in browser storage.** The
   same Jellyfin issue observes that keys in URLs turn every log file into a
   secret, and that tokens in local storage are one cross-site scripting bug
   from theft. RFC 9700 §4.3.2 forbids access tokens in query parameters.
3. **Trust forwarding headers, or treat "local" as trusted.** Emby's
   CVE-2023-33193: spoofed proxy headers made remote requests look local and
   unlocked passwordless sign-in; the research document records about 1,200
   servers backdoored. Navidrome's GHSA-f295-6wp9-qqfg (September 2026):
   spoofed `X-Forwarded-For`, `X-Real-IP` and `True-Client-IP` bypassed the
   sign-in rate limit.
4. **Trust headers that only internal components should set.** Next.js
   CVE-2025-29927: an attacker-supplied `x-middleware-subrequest` header
   skipped authorisation done in middleware. Any header our own components
   use internally must be stripped at the edge.
5. **Take the acting user from the request.** Navidrome GHSA-82gh-4ggp-gfg5
   (High): share creation trusted a client-supplied `userId` and exposed
   other users' libraries. Jellyfin issue 5415 described the same
   separation between authentication and user identifiers.
6. **Check only the first identifier in a batch.** Navidrome
   GHSA-3rwv-f797-f9p3: share creation validated only the first resource.
7. **Apply restrictions in some handlers and not others.** Navidrome
   GHSA-pcjv-h48m-833g: bookmarks, playlist tracks and now-playing skipped
   the library filter. Plex removes folder view entirely under restrictions
   because folder names cannot be filtered.
8. **Issue capability URLs that never re-check their grant.** Navidrome
   GHSA-wp9c-pw66-c6j2: public share stream URLs kept working after the
   share expired or was deleted.
9. **Let a scoped key edit its own scope.** Immich CVE-2026-23896
   (GHSA-237r-x578-h5mv): an API key could raise itself to full access.
10. **Broadcast live events without per-recipient checks.** Jellyfin
    GHSA-4vx8-xhc9-qg6x: broken access control in the session
    remote-control API.
11. **Skip the `Origin` check on WebSockets.** Gitpod CVE-2023-0957: any
    website could open the JSON-RPC socket with the victim's credentials.
12. **Rely on a non-forbidden custom header alone, without `Host`
    validation.** Transmission CVE-2018-5702: a session header plus DNS
    rebinding let any website drive the daemon and write files. Ollama
    CVE-2024-28224: DNS rebinding exposed the full API.
13. **Build filesystem paths from request data.** Jellyfin CVE-2026-35031
    (CVSS 9.9 per the research document): subtitle upload path traversal to
    code execution as root. Jellyfin GHSA-6828-c7cx-hvqm (Critical): path
    traversal in folder management deleted and moved arbitrary directories.
    Navidrome GHSA-r5qr-m328-qcf4: symlinks in the library allowed arbitrary
    file reads.
14. **Hand untrusted images to ImageMagick or render SVG on the server.**
    ImageTragick, CVE-2016-3714: shell metacharacters in crafted images ran
    commands. Immich GHSA-q89f-h332-8q2h (September 2026): an authenticated
    SVG upload reached ImageMagick coders and allowed code execution.
    Jellyfin GHSA-cf3c-8m59-2vhx: external references in user SVGs caused
    memory exhaustion and SSRF.
15. **Fetch a URL because a user or identity provider supplied it.**
    Grafana CVE-2020-13379: the avatar feature let unauthenticated users
    make the server fetch any URL and return the result. Immich
    GHSA-hq46-gw2v-q86p: SSRF through the OAuth profile-picture URL.
16. **Turn off TLS verification for outbound calls.** Immich
    GHSA-hfvf-5c8x-8rc4: OIDC discovery, token, userinfo and key fetches ran
    with verification unconditionally disabled.
17. **Redirect to an unvalidated "continue" parameter.** Immich
    GHSA-8244-8vpr-vp9c (Critical): cross-site scripting in the sign-in
    continue redirect gave one-click account takeover.
18. **Concatenate request data into SQL.** Navidrome GHSA-hm54-32q6-3rcr:
    SQL injection through the artist role parameter, after earlier
    critical SQL injections in 2024 and 2025 recorded in the research
    document.
19. **Let two parsers disagree.** Apache CouchDB CVE-2017-12635: two JSON
    parsers resolved duplicate `roles` keys differently, so authorisation
    checked one value and storage kept the other, giving users admin rights.
20. **Serve many byte ranges per request.** Apache CVE-2011-3192: many
    overlapping ranges exhausted memory and CPU. RFC 9110 §17.15 now says
    servers ought to ignore, coalesce or reject such requests.
21. **Leave HTTP/2 resets and continuation frames unbounded.** HTTP/2 Rapid
    Reset, CVE-2023-44487, was exploited in the wild from August to October
    2023; CVE-2024-27316 is one instance of the 2024 continuation-frame
    flood class (in Apache httpd; whether Rust's HTTP/2 stack was affected
    is unverified).
22. **Keep dormant or forgotten API endpoints reachable.** Optus, 2022:
    according to the Australian regulator's 2024 court filing, as reported
    by CSO Online, an access-control coding error in a dormant,
    internet-facing API exposed the records of about 9.5 million people.
    That is why the route inventory is generated and diffed.
23. **Tell unauthenticated callers which version is running.** Scanners
    counted roughly 314,000 Plex servers on versions vulnerable to
    CVE-2025-34158 weeks after the fix (research document). Version
    information makes that targeting cheap.
24. **Approve devices from screens that only show a code.** Microsoft
    reported Storm-2372 (February 2025) persuading victims to enter
    attacker-generated device codes; RFC 8628 §5.4 recommends showing
    device information and confirming possession.
25. **Show raw Unicode in security decisions.** Trojan Source,
    CVE-2021-42574: bidirectional controls make text display differently
    from its logical order.
26. **Compress responses that mix secrets with attacker-chosen text.**
    BREACH, CVE-2013-3587: compressed sizes leaked secrets. Do not compress
    responses that issue tokens.
27. **Cache authenticated responses in shared caches, or let the router
    accept junk suffixes.** Web cache deception (PortSwigger's 2024
    research) exploits differences between how caches and origin servers
    map paths, so that a private response is stored as a public static file.

## Open decisions for the project owner

> **Status, 2026-10-02.** The owner decisions from every file in
> `docs/security/` are consolidated and de-duplicated in
> [README.md](README.md#open-decisions-for-the-owner). Where the baseline
> has since chosen, or where a recommendation below disagrees with the
> README, the README and the requirement tables win.

1. **How do browsers on the LAN get HTTPS?** This gates secure cookies,
   passkeys, Fetch Metadata and `Clear-Site-Data` for every web user who is
   not on the server itself.
   - *Recommendation:* the project issues per-server names under a domain
     it controls, and each server obtains its own certificate for its name
     through ACME DNS-01; the project's DNS service publishes only the
     server's LAN address and never sees keys or traffic. Also support the
     user's own domain, and loopback HTTP for first run.
   - *Trade-off:* this is the plex.direct model. It adds a central service
     (soft dependency, cost, metadata about which LAN addresses exist), and
     many home routers' DNS rebinding protection blocks public names that
     resolve to private addresses, as Plex and Pi-hole users report; those
     users would need a router exception or would fall back to the native
     apps. The alternatives are worse: self-signed certificates train
     people to click through warnings and are reported not to support
     WebAuthn in Chrome (unverified), and plain HTTP on the LAN would mean
     sessions anyone on the Wi-Fi can capture.
2. **Web client session: HttpOnly cookie or bearer token in JavaScript?**
   *Recommendation:* the cookie, as specified. *Trade-off:* cookies need the
   CSRF layers in this document; bearer tokens avoid CSRF but put the token
   within reach of any injected script and still need somewhere to keep a
   refresh token, which in a browser means storage script can read.
3. **Library item identifiers: random and stored, or derived from a key?**
   *Recommendation:* random 128-bit identifiers for user-created objects;
   for library items, HMAC-SHA-256 under a server identifier key over the
   item's content fingerprint (the research document's OPS-5), so
   identifiers survive a cache rebuild and file moves. *Trade-off:* the
   identifier key becomes irreplaceable data that must be backed up with
   identities; if it is lost, every client resyncs. Random identifiers
   would instead need a durable identifier map outside the rebuildable
   cache, which record 1 does not yet provide. Decide together with the
   identity-data record the research document proposes.
4. **Bind media URLs to the session or only to the principal?**
   *Recommendation:* the session. *Trade-off:* revocation of one lost device
   stops its playback at once, at the price of a session lookup per range
   request (cheap with the in-memory table) and no cache sharing between a
   user's devices.
5. **Should the unauthenticated server endpoint show the server's display
   name?** *Recommendation:* no; the client remembers it after the first
   sign-in, and the invitation landing page can show it because the
   invitation secret authorises that. *Trade-off:* a fresh device sees
   "Gunmetal server" rather than "The Smiths' music" on its first visit.
6. **CSP violation reporting in R1?** *Recommendation:* no endpoint in R1;
   rely on the CI end-to-end test. *Trade-off:* breakage in unusual browsers
   goes unseen, versus adding an unauthenticated write endpoint that must be
   rate-limited and size-capped.
7. **If react-native-web needs inline styles, what then?**
   *Recommendation:* first try `style-src 'self'` (and a hash of an empty
   style element if that is what it takes); if that fails, accept
   `style-src 'self' 'unsafe-inline'` and record it, keeping `script-src`
   strict. *Trade-off:* inline styles allow CSS injection, which matters
   little when there is no HTML injection point and no secrets in the DOM;
   the alternative is a custom build step that extracts styles.
8. **Does R1 make any outbound requests at all** (cover art, lyrics,
   update check)? *Recommendation:* R1 ships with outbound features off
   until the admin enables each one, but the egress crate and its CI ban
   land in R1 regardless. *Trade-off:* a bare library looks less finished
   on day one; every enabled purpose is one more SSRF surface to test.
9. **Profile PIN lockout: parent unlock or timed back-off?**
   *Recommendation:* lock after 5 failures until a parent unlocks it from
   their own session, with a notification. *Trade-off:* a child cannot get
   back in without a parent, but a 4-digit PIN cannot be defended by time
   delays alone against a determined sibling.
10. **Trusted proxies: manual list only, or presets?** *Recommendation:* a
    manual list plus documented recipes and a detection hint; presets for
    known tunnels later. *Trade-off:* manual configuration is a support
    burden, but automatic trust is exactly the Emby failure.
11. **Casting in R2: allow cross-origin reads of cast representations?**
    *Recommendation:* yes, for those representations only (SEC-API-098).
    *Trade-off:* a leaked cast URL can be read from any web page until it
    expires, which is no worse than the URL itself leaking.
12. **ASVS target level.** *Recommendation:* Level 2 with the Level 3 items
    named in the summary. *Trade-off:* full Level 3 adds per-response nonces,
    hardware-backed key storage and multi-user approval of high-value
    actions, which cost more than they return for a household server today.
13. **Invitation landing page: on the server or on gunmetal.tv?**
    *Recommendation:* on the server for R1 (the invitee needs to reach it
    anyway), with an optional static page on gunmetal.tv later for
    "install the app first" guidance. *Trade-off:* a project-hosted page is
    friendlier for remote invitees but becomes a dependency and a template
    for phishing pages that imitate it; either way the secret stays in the
    URL fragment, which never reaches a server log.

## Sources

Standards and guidance:

- OWASP ASVS 5.0.0 chapter sources at tag v5.0.0:
  https://github.com/OWASP/ASVS/tree/v5.0.0/5.0/en (V1, V2, V3, V4, V5, V6,
  V7, V8, V9, V11, V12, V13, V14, V15, V16 chapter files read in full)
- OWASP ASVS releases: https://github.com/OWASP/ASVS/releases
- OWASP API Security Top 10 2023: https://api-security.owasp.org/editions/2023/en/0x11-t10
- OWASP Top 10:2025: https://top10.owasp.org/2025
- OWASP Cross-Site Request Forgery Prevention Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html
- OWASP Server-Side Request Forgery Prevention Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html
- NIST SP 800-63B-4: https://csrc.nist.gov/pubs/sp/800/63/b/4/final and https://pages.nist.gov/800-63-4/sp800-63b.html
- NIST SP 800-218 version 1.1: https://csrc.nist.gov/pubs/sp/800/218/final
- draft-ietf-httpbis-rfc6265bis (status): https://datatracker.ietf.org/doc/draft-ietf-httpbis-rfc6265bis/
- RFC 9110 (HTTP Semantics): https://www.rfc-editor.org/rfc/rfc9110
- RFC 9111 (HTTP Caching): https://www.rfc-editor.org/rfc/rfc9111
- RFC 9700 (OAuth 2.0 Security Best Current Practice): https://www.rfc-editor.org/rfc/rfc9700
- RFC 8628 (Device Authorization Grant): https://www.rfc-editor.org/rfc/rfc8628
- RFC 9457 (Problem Details for HTTP APIs): https://www.rfc-editor.org/rfc/rfc9457
- RFC 7239 (Forwarded HTTP Extension): https://www.rfc-editor.org/rfc/rfc7239
- RFC 6797 (HSTS): https://www.rfc-editor.org/rfc/rfc6797
- RFC 6455 (WebSocket): https://www.rfc-editor.org/rfc/rfc6455
- RFC 6266 (Content-Disposition): https://www.rfc-editor.org/rfc/rfc6266
- RFC 6585 (Additional HTTP Status Codes): https://www.rfc-editor.org/rfc/rfc6585
- RFC 6750 (Bearer Token Usage): https://www.rfc-editor.org/rfc/rfc6750
- RFC 4180 (CSV): https://www.rfc-editor.org/rfc/rfc4180
- RFC 9421 (HTTP Message Signatures): https://www.rfc-editor.org/rfc/rfc9421
- RFC 9449 (DPoP): https://www.rfc-editor.org/rfc/rfc9449
- RFC 6890, RFC 6052, RFC 8215 (special-purpose and translation prefixes): https://www.rfc-editor.org/rfc/rfc6890, https://www.rfc-editor.org/rfc/rfc6052, https://www.rfc-editor.org/rfc/rfc8215
- IANA IPv4 Special-Purpose Address Registry: https://www.iana.org/assignments/iana-ipv4-special-registry/
- IANA IPv6 Special-Purpose Address Registry: https://www.iana.org/assignments/iana-ipv6-special-registry/
- W3C Content Security Policy Level 3: https://www.w3.org/TR/CSP3/
- W3C Trusted Types: https://www.w3.org/TR/trusted-types/
- W3C Fetch Metadata Request Headers: https://www.w3.org/TR/fetch-metadata/
- MDN, Trusted Types API (Baseline 2026): https://developer.mozilla.org/en-US/docs/Web/API/Trusted_Types_API
- MDN, CSP `script-src` (`'wasm-unsafe-eval'`): https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Content-Security-Policy/script-src
- MDN, Clear-Site-Data: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Clear-Site-Data
- Chrome, Local Network Access: https://developer.chrome.com/blog/local-network-access
- PortSwigger, Web cache deception: https://portswigger.net/web-security/web-cache-deception
- MITRE CWE entries (all CWE identifiers above confirmed): https://cwe.mitre.org/

Incidents and advisories:

- CVE records via https://cveawg.mitre.org/api/cve/ for CVE-2011-3192,
  CVE-2013-3587, CVE-2016-3714, CVE-2017-12635, CVE-2018-5702,
  CVE-2020-13379, CVE-2021-42574, CVE-2023-0957, CVE-2023-33193,
  CVE-2023-44487, CVE-2023-49096, CVE-2024-27316, CVE-2024-28224,
  CVE-2025-29927
- Jellyfin advisories: https://github.com/jellyfin/jellyfin/security/advisories
- Jellyfin issue 5415: https://github.com/jellyfin/jellyfin/issues/5415
- Navidrome advisories: https://github.com/navidrome/navidrome/security/advisories and https://github.com/navidrome/navidrome/security/advisories?page=2
- Immich advisories: https://github.com/immich-app/immich/security/advisories
- Microsoft, Storm-2372 device code phishing (February 2025): https://www.microsoft.com/en-us/security/blog/2025/02/13/storm-2372-conducts-device-code-phishing-campaign/
- CSO Online on the regulator's Optus allegations: https://www.csoonline.com/article/2492520/optus-breach-occurred-due-to-a-coding-error-alleges-acma.html
- Plex forum and Pi-hole discussions of DNS rebinding protection and plex.direct: https://forums.plex.tv/t/dns-rebind-protection-detected/149923 and https://discourse.pi-hole.net/t/plex-secure-connections-issues-with-dns-rebinding-possible-fix/15240

Library documentation:

- axum `DefaultBodyLimit` (0.8.9): https://docs.rs/axum/latest/axum/extract/struct.DefaultBodyLimit.html
- image `Limits` (0.25.10): https://docs.rs/image/latest/image/struct.Limits.html

Project documents:

- docs/adr/0001-architecture.md and docs/adr/0002-music-is-first-class.md
- docs/research/users-sharing-and-security.md (incident log, Plex and Emby
  figures, CVE-2026-23896, CVE-2026-35031, CVE-2025-34158, OpenSubsonic API
  key extension)
- docs/research/setup-migration-and-operations.md (first-run setup code,
  OPS-5 fingerprints)
- docs/research/clients-platforms-and-offline.md (casting, TV platforms,
  offline sync)
