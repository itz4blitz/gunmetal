# What the clients need from the server

Written on 2026-10-02. Status: draft for the project owner's review.
Revised on 2026-10-02 to follow the security baseline in
[docs/security](../security/README.md). Revised on 2026-10-03 for the
owner's answers of 2026-10-02 in [the decision register](../decisions.md#owner-answers-2026-10-02):
the smaller R1 with point releases R1.1 to R1.3 (D-10), and HTTPS in R1
without the project name service (D-07).

This is a list of capabilities, not an API specification. It reads the
four interface documents in [docs/ui](../ui/README.md) and the
[feature map](../features/README.md) and asks, for every screen and every
step of every flow, what the server has to provide. It does not name
routes, payload shapes or wire formats; the protocol types belong in the
shared Rust core (ADR 1, decision 2) and will be written test first. The
purpose is to give the backend work a complete checklist, grouped so that
separate people or agents can take one group each.

The feature map is the source of truth for features. The security baseline
is the source of truth for who may do what, how, and how often. Where the
two disagree, this document follows the baseline and records the change in
[Conflicts with the security baseline](#conflicts-with-the-security-baseline).
Where this document goes beyond both, the text says **Proposal**. Claims
the inputs could not confirm are marked "(unverified)".

## How to read this document

### Releases

The release values are the feature map's: **R1** (music, server and web
client, no remuxer or transcoder; HTTPS through the owner's own domain, a
tailnet or the same machine, and remote use only through the owner's
reverse proxy or tailnet), **R1.1**, **R1.2** and **R1.3** (the point
releases after R1 that the owner adopted on 2026-10-02, decision D-10:
bring your music in; the household and the admin; discovery and
analysis), **R2** (video, the remuxer, sandboxed transcoding, native TV,
phone and desktop clients, built-in remote access, and the project-run
per-server name service with its naming client and certificate-transparency
monitoring, decision D-07), **R3** (M3U and live TV), **Later** (wanted,
not scheduled) and **No** (deliberately not doing).
A capability is placed in the first release whose screens need it, or the
first release in which the baseline requires it, whichever is earlier.
Where part of a capability arrives later, the release cell names each
release with the part it covers in brackets. The baseline's release-scope
table (SEC-TM-074) decides which surfaces exist in each release.

### Columns of the capability tables

Every capability table has the same columns.

| Column | Meaning |
|---|---|
| **ID** | A label for this plan only, such as API-QUE-03. It is not a feature ID and may change when the real protocol is written. |
| **Capability** | A short name. |
| **What it does** | The behaviour, in a sentence or two. |
| **Surfaces** | The screens that need it, by SUR-ID from [surfaces.md](../ui/surfaces.md), or a flow from [flows.md](../ui/flows.md). |
| **Features** | The owning feature rows. |
| **Rel.** | R1, R1.1, R1.2, R1.3, R2, R3, Later or No; several values, each with the part it covers, when part of the capability arrives later. |
| **Offline** | How it behaves with the server gone. **Local**: answered from the device's own copy. **Queue**: the device records the write and sends it later (in R1 this is true only for play events, CLI-093; edits join in R2, CLI-094). **Server**: needs the server at that moment. **Host**: runs on the server for administrators; there is no offline case. |
| **Live** | Whether connected clients must learn of a change without asking. **Push**: the server tells them over a live channel. **Feed**: the change arrives through the sync change feed, nudged by a push when connected. **No**: the client asks when it needs it. |

### Columns of the security tables

Each group of capabilities is followed by a security table with one row per
capability (or per run of capabilities that share the same answer).

| Column | Meaning |
|---|---|
| **Who, and the object check** | The capability the caller must hold, from the capability table in [identity-and-access.md](../security/identity-and-access.md#3-capabilities-presets-and-ceilings) (for example `library.manage`), or **Self** for a person acting on their own account, profile, devices and data. Then the object-level check the authorisation layer applies. Anything not named is denied. |
| **Auth** | How the request authenticates. See [Authentication kinds](#authentication-kinds). |
| **Rate** | The rate-limit class. See [Rate-limit classes](#rate-limit-classes). |
| **Capability URL** | Whether the capability returns a URL that is itself a credential, with its lifetime and binding. **No** means it returns none. |
| **Security** | The requirement IDs in [docs/security](../security/README.md) that the capability must meet. The rules in [Rules that apply to every capability](#rules-that-apply-to-every-capability) apply as well and are not repeated. |

### Authentication kinds

| Kind | Meaning |
|---|---|
| **Public** | No credential. Only the endpoints in [The only unauthenticated endpoints](#the-only-unauthenticated-endpoints) may use it. |
| **Exchange** | A credential exchange on the allow-list: a secret (claim code, passkey assertion, invitation secret, pairing code, recovery code, share secret) travels in a `POST` body, never in a URL query or path (SEC-API-004, SEC-NET-036). |
| **Session** | A signed-in principal: in R1 the web client's `__Host-gm_session` cookie with the `Gunmetal-Request: 1` header and the fetch-metadata and Origin checks (SEC-API-032 to SEC-API-034); from R2 also a native device's sender-constrained access token in `Authorization` (SEC-IAM-050). Route tag: none. |
| **Session + uv** | A signed-in session plus user verification with an existing passkey or device key in the previous 5 minutes, without an admin session (used for a person's own sensitive actions, such as adding a credential or exporting data; SEC-IAM-023, SEC-PRV-048). |
| **Elevated** | A session plus the separate admin session: its own `__Host-` cookie, SameSite=Strict, 15 minutes idle and 1 hour in total, started only by a user-verifying sign-in. Route tag: elevated (SEC-IAM-041). |
| **Fresh-uv** | Elevated, plus a user-verified passkey or device-key assertion no older than 5 minutes; an OIDC sign-in alone never satisfies it. Route tag: fresh-uv (SEC-IAM-041, SEC-IAM-107). |
| **Capability** | A capability URL or WebSocket ticket. Cookies and `Authorization` are ignored, and a valid cookie never rescues a bad signature (SEC-API-029, SEC-API-042). |
| **Key** | From R2: a scoped API key in `Authorization`, never with an administrator scope, evaluated as the intersection of its scopes and its owner's current rights (SEC-EXT-008 to SEC-EXT-012, SEC-API-020). |
| **Host** | A command on the server's host that reaches the server over a local socket open only to the service account. There is no network path (SEC-IAM-092). |

Elevated and Fresh-uv routes are refused from limited-class devices
(shared browsers, paired browsers, TVs; SEC-CLI-024, SEC-IAM-108) and on
internet-posture paths unless the owner has turned remote administration on
(SEC-NET-045).

### Rate-limit classes

These are the classes of [web-and-api-security.md](../security/web-and-api-security.md#rate-limiting-that-families-will-not-switch-off),
plus the upload, socket and public ceilings the baseline also requires. The
numbers are to be tuned by measurement; the keys and the behaviour are
fixed. Every limited request gets 429 with `Retry-After` and a body that
reveals nothing (SEC-API-057).

| Class | Key | Behaviour | Requirements |
|---|---|---|---|
| **Public** | Address key, plus one server-wide ceiling | Fixed-capacity stores with eviction for any state kept for unauthenticated callers | SEC-API-057, SEC-NET-051, SEC-NET-052, SEC-NET-068 |
| **Ceremony** | Address key and server-wide | Generous; no per-account lock, because passkeys cannot be guessed and a lock would let anyone lock out the owner | SEC-API-056, SEC-IAM-101 |
| **Secret** | The target, the source or device, and server-wide | The guessable-secret delay schedule (30 s, 1 min, 5 min, then 15 min, never permanent); one alert after 10 failures, in the daily summary | SEC-API-056, SEC-IAM-101 |
| **Read** | Principal | High burst, for scrolling and sync | SEC-API-057, SEC-API-063 |
| **Write** | Principal | Moderate | SEC-API-057 |
| **Image** | Connections per session | No per-request limit; a TV grid makes hundreds a minute | SEC-API-061, SEC-NET-048 |
| **Stream** | Concurrent streams per session, per principal and server-wide | Limits set by the admin; counts playback, not browsing | SEC-API-031, SEC-IAM-102, SEC-TM-068 |
| **Job** | One per kind per principal, plus server-wide concurrency | A repeated request joins the running job | SEC-API-064, SEC-NET-053 |
| **Upload** | Principal, by count and total bytes | Declared types, sizes and pixel limits per route | SEC-API-085, SEC-API-088 |
| **Socket** | Connection and principal | At most 64 KiB per message, messages per second per connection, connections per principal, idle close | SEC-API-043 |

### Rules that apply to every capability

These come from ADR 1, the R1 rows of the feature map that are rules rather
than features, and the security baseline's first principles. They are
stated once here and not repeated in every row.

- **Deny by default.** Every HTTP, WebSocket, iroh and adapter route is
  registered in one typed route table with one access class, the
  capability it needs, one route tag (none, elevated or fresh-uv), a
  rate-limit class and, for a state-changing admin route, the audit event
  it writes; a route without them does not build (SEC-API-001,
  SEC-API-019, SEC-IAM-041, SEC-IAM-067, SEC-OPS-020, SEC-TM-005). Every
  route outside the [unauthenticated list](#the-only-unauthenticated-endpoints)
  answers 401 with one identical body before reading the request body
  (SEC-API-003, SEC-TM-004).
- **One authorisation layer.** Authorisation is one deny-by-default pure
  function in the core over the principal, its ceilings, the action, the
  resource and the context, and every read, list, search and write goes
  through one visibility predicate built from library grants, item shares
  and (from R2) content policy (SEC-IAM-068, SEC-IAM-070). Handlers reach
  stored objects only through that layer (SEC-API-010). Checks test
  capabilities, never role names (SEC-IAM-074). Effective rights are the
  intersection of the account's capabilities, the device ceiling, the
  session state and the token scope.
- **Object checks.** An object the caller may not see gets the same 404 as
  one that does not exist; a visible object the caller may not change gets
  403 (SEC-API-011). A request naming several objects is refused whole if
  any one fails (SEC-API-012). The acting user, profile and device come only
  from the credential; a request field naming another principal is refused
  except on an admin route that targets one (SEC-API-013, SEC-HIS-009).
  Grants and restrictions apply on every path: lists, search, sync
  payloads, pushed events, playlists, capability URLs (SEC-API-014,
  SEC-API-016, SEC-TM-026, SEC-MED-051). Errors in authorisation end in
  denial (SEC-IAM-069).
- **Location is never identity.** A LAN address, VPN, proxy header or
  "home" network never widens access; it can only add a restriction
  (SEC-IAM-013, SEC-API-075, SEC-NET-016).
- **No credential over cleartext.** Over plain HTTP every peer but
  loopback gets only a static redirect or help page (SEC-NET-001). In the
  default home posture, non-local peers get only a static help page
  (SEC-NET-024).
- **Credentials.** Credentials travel only in the session cookie or the
  `Authorization` header, one per request, never in a query string or path
  (SEC-API-004, SEC-EXT-006, SEC-HIS-041). Media, artwork, lyrics files and
  subtitles are served only from capability URLs whose path carries the
  token (SEC-API-026 to SEC-API-029). The R2 adapters are the documented
  exceptions, with per-app keys (SEC-API-094, SEC-EXT-063).
- **Identifiers.** Public IDs carry at least 128 random bits, are typed by
  kind, and an ID of the wrong kind gets the same 404 (SEC-API-023,
  SEC-API-024, SEC-PRV-021). They survive cache rebuilds and file
  replacement (INT-008), because they are bound to content identity
  (LIB-028) in the identity store; whether library item IDs are stored or
  derived is the web document's open decision 3.
- **Lists and inputs.** Lists use one paging, filtering and caching
  convention (INT-007), with a page cap of 500 unless the route declares
  more and cursors that are opaque or MAC-protected and never trusted for
  authorisation (SEC-API-025, SEC-API-063). Bodies are JSON, decoded into
  per-action types that reject unknown fields, within declared size limits
  (SEC-API-035, SEC-API-060, SEC-API-067, SEC-IAM-072). Responses are built
  from per-role types, so paths, addresses and other people's identities
  appear only in admin types (SEC-API-068). Errors are problem-details
  objects from a closed catalogue (SEC-API-072).
- **Protocol.** The protocol is versioned in the core and in the path
  (`/api/v1`) from R1 (SEC-API-092). In R1 the web client always loads the
  server's own build (SEC-CLI-011); from R2, when native apps arrive, the
  server also keeps the previous version for older clients (CLI-032). The
  OpenAPI description is generated from the route table (SEC-API-091).
- **One writer.** One writer owns the database (ADM-080), so high-rate
  client reports (positions, heartbeats) must be batched rather than
  written one by one.
- **Nothing leaves the house by default.** Any capability that reaches the
  internet goes through the one egress client, appears in the egress
  inventory and on the network activity page (ACC-113, LIB-108, ADM-129,
  SEC-TM-048, SEC-TM-075, SEC-API-076, SEC-EXT-001, SEC-PRV-007).
- **History is private.** One person's Activity data (history, ratings,
  private playlists, queue, now-playing, recommendations) is never
  returned to anyone else except a managed profile's guardian; admins see
  live sessions and totals, without titles unless the person opted in
  (SEC-PRV-022, SEC-PRV-025, SEC-TM-054).
- **Revocation bites on the next request**, and closes open sockets and
  in-flight streams within 5 seconds (SEC-IAM-043, SEC-API-017).
- **Logs** record route templates, never tokens, codes, bodies or query
  strings, and no titles at the default level (SEC-API-095, SEC-PRV-042,
  SEC-PRV-043).
- **Every row here is tested.** The route table generates the anonymous
  suite, the cross-principal suite and the role matrix, so every
  capability in this document is covered by them (SEC-IAM-071,
  SEC-API-011, SEC-API-019, SEC-STD-004).

### The only unauthenticated endpoints

In R1 these, and only these, answer without a session. The list is a
checked-in file under CODEOWNERS that the route table is tested against
(SEC-API-002, SEC-TM-004, SEC-IAM-067).

1. **The web client's embedded assets**, served from a build-time manifest
   (SEC-API-002, SEC-STD-017).
2. **`GET /api/v1/server`**: the supported protocol-version range, the
   server instance identifier, whether the server is claimed, and the
   enabled sign-in methods. Nothing else: no version, no display name
   (SEC-API-005, SEC-NET-047).
3. **`GET /healthz`**: liveness only (SEC-NET-046).
4. **First-run setup**: the setup page, the claim and restore-at-setup,
   only while the server is unclaimed, and only from loopback or a secure
   context; they answer 404 for good once claimed (SEC-IAM-006,
   SEC-IAM-008, SEC-IAM-009, SEC-OPS-003).
5. **Sign-in ceremonies**: the passkey challenge and assertion, the OIDC
   start and callback (from R1.2), and redeeming a recovery code or
   recovery link to enrol a new credential (SEC-API-002, SEC-IAM-089,
   SEC-IAM-091, SEC-IAM-092).
6. **Token refresh**: in R1, a paired browser renewing its session by
   signing a fresh server challenge (SEC-IAM-108); from R2, native token
   renewal by device-key signature (SEC-IAM-050).
7. **Invitation landing and redemption**, with the secret in the URL
   fragment and then the `POST` body (SEC-API-096, SEC-PRV-053).
8. **Device-pairing request and poll** (SEC-IAM-056). Approval itself is a
   signed-in action.
9. **Share-link landing and redemption** for music, from R1.2 (SEC-TM-004,
   SEC-API-097, SEC-PRV-031).
10. **Capability routes**: signed media and image URLs and the WebSocket
    ticket, authenticated by the capability rather than a session
    (SEC-API-026 to SEC-API-029, SEC-API-042).

Three responses are not routes at all and carry nothing but static text:
the cleartext redirect or help page (SEC-NET-001), the home-posture help
page (SEC-NET-024), and the "starting" page served while the server starts
or migrates (SEC-OPS-050). From R2 the iroh pairing, invitation and
challenge protocols (SEC-IAM-052, SEC-NET-035), the device authorisation
flow (SEC-IAM-055) and the adapters' public endpoints (SEC-EXT-061) join
the list. Everything else needs a session, a key or a capability.

### What "local-first" means for this list

Most reading in Gunmetal is not an API call. Home, every library list,
artist and album pages, search, history, smart playlist previews and radio
picks are computed on the device from the synced library by the core,
running as WebAssembly in browsers and through UniFFI in native apps
(CLI-022, DIS-002, DIS-084, MUS-208). For those features the server's job
is to put the right fields into the synced copy and keep them current, so
the "Catalogue" section below lists fields rather than endpoints. The
endpoints are concentrated in sync, streaming, the queue, writes to the
user log, and administration. Because the synced copy is the read path,
the sync model carries the authorisation and privacy rules for those
features (see [The sync model](#the-sync-model)).

## Capabilities by resource

### Server, connection and protocol

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-SYS-01 | Health check | Answers liveness for containers and uptime monitors, and lets clients tell "server down" from "network down". Liveness only: no readiness detail, no version, nothing about the library. | SUR-003, SUR-111 | CLI-025, INT-013, ADM-128 | R1 | Server | No |
| API-SYS-02 | Cleartext help page | Over plain HTTP, every peer but loopback gets only a static redirect to the HTTPS address or a help page that sets no cookie and explains in plain words how to reach it; there is no reduced web client on a plain-HTTP address. In the default home posture, non-local peers get only a static help page. The client itself checks for a secure context at start and shows "unsupported" without one. | SUR-070, SUR-077, SUR-082 | CLI-150, ADM-021 | R1 | Server | No |
| API-SYS-03 | Protocol negotiation | `GET /api/v1/server` gives the supported version range, so a client learns which update it needs while what still works keeps working. A web bundle whose build differs from the server's gets a typed "reload required" error on its next call. Both are required in R1 (SEC-API-005, SEC-CLI-011); keeping older versions working for older native clients (CLI-032) matters from R2. | SUR-003 | CLI-032 | R1 | Server | No |
| API-SYS-04 | Capability discovery | For a signed-in caller: the API version, enabled modules, adapters and extensions, and the scopes the caller holds. Unauthenticated callers get only API-SYS-06. | SUR-112 | INT-005 | R1 | Server | No |
| API-SYS-05 | Who am I | Returns the caller's account, profile, device, device class and token scope, for clients and tools. | SUR-006 | INT-026 | R1 | Local | No |
| API-SYS-06 | Public sign-in facts | Before sign-in, only what `GET /api/v1/server` returns: protocol range, instance ID, whether claimed, enabled sign-in methods. The server's display name and sign-in message (from R1.2) are shown after sign-in; a returning client shows the name it remembers, and the invitation landing page may show it because the invitation secret authorises that. | SUR-070 | ADM-140, ACC-007 | R1 | Server | No |
| API-SYS-07 | Startup status page | While the server starts, migrates, rebuilds or restores, every request gets one static "starting, try again shortly" page with no version, build, path, step or error. The detail (each step, estimates, the snapshot location) goes to the host console, the journal and `gunmetal doctor`, and to signed-in admins once the database is open. Refusals (running as root or with capabilities, a data directory on a network filesystem, newer durable state than the binary understands) stop the server with a console message and serve nothing. | SUR-080 | ADM-032, ADM-006, ADM-007, ADM-056 to ADM-059, ADM-078, ADM-079 | R1 | Server | Push (it refreshes itself) |
| API-SYS-08 | Emergency page | A minimal server-rendered admin page, without the client bundle and under the same CSP, behind the admin session: status, recent diagnostic-log lines, back up now and restart. Downloading a backup from it needs the owner and fresh user verification. It is not reachable signed out; the host route to the same help is `gunmetal doctor` and the host commands. | SUR-081 | ADM-113 | R1.2 | Server | No |
| API-SYS-09 | API reference | The reference generated from the route table, served by each server to signed-in callers so it matches that version. | SUR-112 | INT-001 | R1 | Server | No |
| API-SYS-10 | Client event channel | One authenticated WebSocket per signed-in session that the server uses to push the events marked "Push" in this document: sync nudges, scan progress, a stopped session, a revoked device, a new device, the active player changing. Every event is built per recipient by the authorisation layer; there is no broadcast. See [Flags](#ui-requirements-the-architecture-makes-hard-or-impossible), item 12. **Proposal** for R1. | SUR-002, SUR-003, SUR-083, SUR-084, SUR-100 | ADM-099, ADM-102, LIB-021, CLI-103 | R1 | Server | Push |

| ID | Who, and the object check | Auth | Rate | Capability URL | Security |
|---|---|---|---|---|---|
| API-SYS-01 | Anyone; no object | Public | Public | No | SEC-NET-046, SEC-API-002, SEC-TM-004 |
| API-SYS-02 | Anyone; no object. Not a route: served instead of the route table | Public | Public | No | SEC-NET-001, SEC-NET-005, SEC-NET-024, SEC-API-052 |
| API-SYS-03 | Anyone for the version range; Self for the build check | Public; Session | Public; Read | No | SEC-API-005, SEC-API-092, SEC-CLI-011 |
| API-SYS-04 | Any signed-in principal; returns only the caller's own scopes | Session | Read | No | SEC-API-005, SEC-API-068, SEC-NET-047 |
| API-SYS-05 | Self | Session | Read | No | SEC-API-013, SEC-API-068, SEC-CLI-024 |
| API-SYS-06 | Anyone; no object | Public | Public | No | SEC-API-005, SEC-NET-047, SEC-PRV-031 |
| API-SYS-07 | Anyone sees the static page; step detail only on the host and to admins (`server.settings`) | Public (static); Host | Public | No | SEC-OPS-050, SEC-OPS-053, SEC-OPS-048, SEC-OPS-051 |
| API-SYS-08 | Admin with `server.settings`; backup download only the owner | Elevated; Fresh-uv for the backup download | Write | No | SEC-IAM-041, SEC-OPS-045, SEC-NET-045, SEC-OPS-029, SEC-API-044 |
| API-SYS-09 | Any signed-in principal | Session | Read | No | SEC-API-091, SEC-API-005 |
| API-SYS-10 | Self: only events about objects the recipient may see, filtered per recipient | Capability (single-use ticket of 128 bits, 30 seconds, bound to the requesting session) or the session cookie with an exact Origin match | Socket | The WebSocket ticket: single use, at most 30 seconds, bound to one session | SEC-API-016, SEC-API-017, SEC-API-041, SEC-API-042, SEC-API-043, SEC-IAM-016, SEC-IAM-043, SEC-NET-020 |

### Setup, sign-in and sessions

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-AUTH-01 | Unclaimed-server guard | While no owner exists, answers only the setup page and its assets, the claim and restore-at-setup endpoints and the health check, and accepts no sign-in. Issues a 128-bit, single-use claim code that lasts 24 hours, shown only on host channels: the console and journal, a terminal QR code, a claim URL with the code in its fragment, a file only the service user can read, and `gunmetal claim-code`, which mints a new code once the old one has expired. The code never appears in a network response, and failed attempts never rotate it. | SUR-082, SUR-110, F01 | ACC-001, ADM-018 | R1 | Server | No |
| API-AUTH-02 | Setup state | The claim comes first: it creates the owner, binds the first passkey and consumes the code in one transaction, and from then on every setup route answers 404 for good, including after a restart, a failed migration or a restore. The remaining welcome steps (in R1 the required update-and-advisory question, privacy and libraries; from R1.1 the required provider step and import; from R1.2 locale and server name) are ordinary owner settings behind the admin session, so a half-finished setup simply resumes there, and an expired code is replaced by `gunmetal claim-code` (flows G1). | SUR-082 | ADM-020, ADM-027, ADM-140, ADM-028 | R1 | Server | No |
| API-AUTH-03 | Owner creation | Claiming creates the owner in the identity store with a passkey, refused unless the page is a secure context or loopback (on a headless box, an SSH tunnel to localhost). It issues the printable recovery kit (the owner's recovery codes and the backup recovery key) and, from R1.2, may then link an OIDC provider, which never makes anyone the owner. There is no password and no TOTP; ACC-052 and ACC-053 are to be withdrawn from the feature map (owner decision 1). | SUR-082 | ACC-002, ADM-019, ACC-050, ACC-057 | R1 | Server | No |
| API-AUTH-04 | Passkey sign-in | Runs a usernameless WebAuthn ceremony with user verification required, against a relying-party ID fixed at setup. | SUR-070 | ACC-050 | R1 | Server | No |
| API-AUTH-05 | Password and two-factor sign-in | Not built. There are no account passwords and no TOTP codes, for sign-in or recovery (owner decision 1). A browser that cannot use a passkey signs in by approval from the person's phone (API-AUTH-13); a person with no passkey-capable device uses a hardware key or the household's help. | SUR-070 | ACC-052, ACC-053, ACC-007 | No | Server | No |
| API-AUTH-06 | Single sign-on | Completes OIDC with the household's own provider as a confidential client: authorization code with PKCE, state and nonce, an exact redirect URI, identities keyed by issuer and subject, auto-registration off by default, provider claims never conferring the owner role; no other outbound call. An OIDC sign-in never satisfies a fresh-uv action. | SUR-070, SUR-092 | ACC-057, ACC-003 | R1.2 | Server | No |
| API-AUTH-07 | Guessing limiter | One limiter. Guessable secrets (the claim code, pairing codes, share-link passwords from R1.2 and PINs from R2) follow one delay schedule per target and source, never permanent; a pairing code dies after 5 wrong guesses; the claim code and share links are never disabled by failures. Passkey and device-key failures are throttled per source and alerted, never locked. High-entropy secrets (invitations, recovery codes) are rate-limited per source and server-wide. Every failure also writes a fail2ban-friendly line. | SUR-070, SUR-092, SUR-101 | ACC-063, ADM-122 | R1 | Server | No |
| API-AUTH-08 | Browser sessions | Issues the `__Host-gm_session` cookie (Secure, HttpOnly, SameSite=Lax, an opaque 256-bit token stored only as a hash). At sign-in the client asks whether the browser is personal or shared. Personal sessions end after 7 days unused or 30 days in total; shared sessions use a cookie that ends with the browser, end after 30 minutes idle and keep library data in memory only. A new token is issued at sign-in, at elevation and at every profile switch. The device and its class are recorded. | SUR-070, SUR-092 | ACC-124, ACC-079, ACC-068 | R1 | Server | No |
| API-AUTH-09 | Session epoch and revocation | Every session-derived credential (session token, capability URL, WebSocket, in-flight range response) is checked against the live session on every request. A credential change, "sign out everywhere", a disabled account, a revoked device or a key rotation makes them fail on the next request, and closes open sockets and in-flight responses within 5 seconds. Signing out invalidates the session on the server and answers with `Clear-Site-Data`. | SUR-078, SUR-090, player "Signed out" state | ACC-065, ACC-069, ACC-070, ACC-008, ACC-122 | R1 | Server | Push (the signed-out device is told) |
| API-AUTH-10 | Admin session and fresh verification | Administrator rights need a separate admin session (its own `__Host-` cookie, SameSite=Strict, 15 minutes idle, 1 hour in total), started only by a user-verifying passkey, or, from R1.2 and for an account with no passkey, by a fresh provider sign-in less than 5 minutes old. Fresh-uv routes also need a passkey or device-key verification in the previous 5 minutes. A media session never authorises an admin route. | SUR-008 | ACC-056 | R1 | Server | No |
| API-AUTH-11 | Owner recovery from the host | `gunmetal owner recover`, run on the host, reaches the server over a local socket only the service account can open and prints a single-use enrolment link valid for 15 minutes. It works from the locked state, ends every owner session, alerts every administrator, and raises a banner and an audit entry. | SUR-110, SUR-070, SUR-083 | ACC-004, ADM-034 | R1 | Host | No |
| API-AUTH-12 | Help a locked-out user | An admin issues a single-use recovery enrolment link (with a QR code) for a member or guest; the owner issues them for administrators; nobody can issue one for the owner. It is redeemed in person (the QR code on the issuer's screen, with the person present) or on a device the person already approved. The credential it enrols starts a 72-hour recovery hold, during which it cannot remove other credentials, elevate, export or invite, the person's existing devices can cancel it with one tap, and the person's history and private data stay hidden from it. | SUR-090 | ACC-064 | R1 | Server | No |
| API-AUTH-13 | Pairing and device keys | One pairing protocol. **R1, browsers:** a browser that cannot use a passkey shows an 8-character code and a QR code carrying the server's identity key; the person approves from a signed-in personal-class device, which shows the request's claimed name marked unverified, its type and whether it is in this home; a remote approval needs the code typed from the requesting device. The browser then makes a non-extractable Web Crypto key and gets a limited-class session renewed only by signing a fresh challenge; it can never administer, approve devices or change account security. **R2, native:** phones and TVs enrol their own hardware-backed key pair and sign in by challenge and response; TVs use the device authorisation flow. | SUR-061, F04 | ACC-061, ACC-062, ACC-051, CLI-027, INT-027, INT-028 | R1 (browsers); R2 (native devices) | Server | Push (the waiting device moves on) |
| API-AUTH-14 | Profile switch and PIN | Lists the profiles a shared device may open (names and avatars only) and checks a PIN on the server under the limiter; a PIN gates switching into a profile and nothing else. A new session token is issued at each switch. | SUR-006, SUR-079 | ACC-019, ACC-020, ACC-021 | R2 | Server | No |
| API-AUTH-15 | Recovery codes | Ten single-use recovery codes of at least 80 bits, offered to owners and administrators when they enrol and available to everyone under Account > Recovery, stored only as peppered hashes. A code opens a session that can only enrol a new credential, which starts the recovery hold of API-AUTH-12; every device of the account is alerted and shown every credential to review. | SUR-078, SUR-082 | ACC-137 | R1 | Server | No |

| ID | Who, and the object check | Auth | Rate | Capability URL | Security |
|---|---|---|---|---|---|
| API-AUTH-01 | Anyone may load the setup page; only a holder of the claim code, from loopback or a secure context on a configured origin, may claim | Exchange (claim code in the body) | Secret (10 a minute server-wide; per-source delays) | The claim URL carries the code in its fragment only; 24 hours; single use | SEC-IAM-005 to SEC-IAM-008, SEC-OPS-001, SEC-OPS-003, SEC-OPS-004, SEC-OPS-007, SEC-STD-029 |
| API-AUTH-02 | Before the claim: the claim-code holder. After: the owner | Exchange; then Elevated | Secret; then Write | No | SEC-IAM-009, SEC-OPS-005, SEC-OPS-006, SEC-OPS-047, SEC-PRV-013, SEC-PRV-023 |
| API-AUTH-03 | The claim-code holder; creates exactly one owner | Exchange | Secret | No | SEC-IAM-003, SEC-IAM-009, SEC-IAM-018 to SEC-IAM-021, SEC-IAM-025, SEC-IAM-031, SEC-IAM-089, SEC-IAM-107, SEC-PRV-040, SEC-STD-006 |
| API-AUTH-04 | Anyone; the assertion must verify against an enrolled credential, and the response never reveals whether one exists | Exchange | Ceremony | No | SEC-IAM-018 to SEC-IAM-022, SEC-API-056, SEC-API-058, SEC-NET-051 |
| API-AUTH-05 | Not built | None | None | No | SEC-IAM-025, SEC-STD-006 |
| API-AUTH-06 | Anyone; the callback must match the one provider, state and nonce it was issued for; linking to an existing account only inside that account's verified session | Exchange | Ceremony | No | SEC-IAM-026 to SEC-IAM-036, SEC-IAM-107, SEC-CLI-026, SEC-STD-025, SEC-TM-022 |
| API-AUTH-07 | Applies to every route that checks a secret | Not a route | Secret; Ceremony | No | SEC-API-056, SEC-API-057, SEC-API-058, SEC-IAM-099, SEC-IAM-101, SEC-OPS-028, SEC-NET-052, SEC-NET-068, SEC-HIS-046 |
| API-AUTH-08 | Self; the session belongs to the account that signed in | Exchange (issues the session) | Ceremony | No | SEC-API-032, SEC-IAM-037, SEC-IAM-038, SEC-IAM-041, SEC-OPS-016, SEC-CLI-010, SEC-CLI-024, SEC-TM-058 |
| API-AUTH-09 | Self for own sessions; admins with `session.manage` for non-owner accounts; only the owner for the owner's sessions | Session | Write | No | SEC-IAM-043, SEC-IAM-046, SEC-IAM-103, SEC-API-017, SEC-API-028, SEC-API-039, SEC-TM-028 |
| API-AUTH-10 | Self; the account must hold the administrator capability being exercised | Session + uv (starts the admin session) | Ceremony | No | SEC-IAM-036, SEC-IAM-041, SEC-IAM-107, SEC-TM-017, SEC-CLI-024, SEC-NET-045 |
| API-AUTH-11 | Whoever controls the host as the service account; targets only the owner account | Host | None (local socket) | Enrolment link: secret in the fragment, single use, 15 minutes | SEC-IAM-092, SEC-OPS-009, SEC-OPS-032, SEC-OPS-034, SEC-CLI-013 |
| API-AUTH-12 | Admins with `user.recover` for members and guests; the owner (`admin.manage`) for administrators; the target is never the owner | Elevated | Write | Recovery link: secret in the fragment, single use, short-lived; redeemed in person or on an approved device | SEC-IAM-090, SEC-IAM-091, SEC-IAM-106, SEC-IAM-075, SEC-IAM-077, SEC-CLI-013, SEC-STD-029 |
| API-AUTH-13 | Request and poll: anyone. Approve: the account holder on a signed-in personal-class device; the new device joins the approver's own account and never gets owner or admin capabilities | Exchange (request, poll, renewal); Session + uv (approve) | Secret | No URL credential; the QR code carries the server key and an approval URL on the server's own origin, and the code lives 10 minutes | SEC-IAM-056 to SEC-IAM-060, SEC-IAM-108, SEC-CLI-024, SEC-CLI-025, SEC-STD-027, SEC-IAM-048, SEC-IAM-050, SEC-IAM-055 |
| API-AUTH-14 | A household device's enabled profiles only; PIN checked by the server only | Session (device) | Secret | No | SEC-IAM-038, SEC-IAM-061 to SEC-IAM-066, SEC-CLI-028 |
| API-AUTH-15 | Self, for generating codes; anyone holding a valid code, for redemption, which can only enrol a credential on that code's account | Session + uv (generate); Exchange (redeem) | Write; Ceremony | No | SEC-IAM-089, SEC-IAM-090, SEC-IAM-106, SEC-IAM-101, SEC-STD-029 |

### User, profile and settings

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-USR-01 | Profile identity | Name and picture per profile, separate from the sign-in account. Pictures go through the upload path, are decoded in a worker, re-encoded with all metadata removed, and SVG is refused. | SUR-006, SUR-079 | ACC-011, ACC-017, ACC-125 | R1 (the profile record, separate from sign-in, with its account's display name, ACC-017); R1.1 (a chosen name and picture, ACC-011) | Local (read); Server (write) | Feed |
| API-USR-02 | Settings that follow the person | Settings records with a person scope and a device scope (audio output, storage, theme override), kept outside the rebuildable cache. Privacy settings start at their most private value. | SUR-073 to SUR-077 | ACC-012, CLI-030, CLI-141 | R1; R1.1 (device-scope settings) | Local (read); Server (write in R1) | Feed |
| API-USR-03 | Sign-in methods | List, add and remove passkeys and (from R1.2) OIDC links, with user verification in the previous 5 minutes; the last credential cannot be removed; the account's other devices are told. There is no password or two-factor to manage (owner decision 1). Refused while a recovery hold is on. | SUR-078 | ACC-055, ACC-050 | R1 | Server | No |
| API-USR-04 | Sign-in history | The person's own security events, with full addresses, including every time an administrator accessed their data. | SUR-078 | ACC-078, ADM-110 | R1 | Server | No |
| API-USR-05 | Export your data | Builds a documented, versioned export of everything the person told the server (history, loves, ratings, playlists, hides, layouts and rules) and nothing about anyone else. Starting it needs authentication in the previous 5 minutes and is refused during a recovery hold; the download is single use, bound to the requesting session and expires within an hour. | SUR-078, F16 | ACC-010, DIS-058, MUS-188, INT-151, LAT-007 | R1 | Server | No |
| API-USR-06 | History import | Takes Last.fm or ListenBrainz export files through the upload path, parses them under budgets outside the server process, matches them into the person's own profile, and marks every imported listen so a scrobbler never sends it back. Imports can be removed as a batch. | SUR-078, SUR-097, SUR-009 | MUS-189, ADM-042, INT-107 | R1.1 | Server | Push (job progress) |
| API-USR-07 | Private session flag | Starts from the player in at most two interactions. While it is on, the device records no history events, recommendation signals or scrobbles and queues nothing for later upload, the server records no play from that session, and anyone else's live-session view omits the title. It ends when the person turns it off or after a period without playback that the person picks (default 6 hours, from the privacy design guidance). | SUR-002, SUR-010, SUR-006 | ACC-117, MUS-185, DIS-053 | R1 | Local | No |
| API-USR-08 | Household and child profiles | Managed profiles linked to policies, presets, rating ceilings, allow and block rules, schedules and the guardian's view of a child's history. Content policy is enforced inside the shared visibility predicate. | SUR-079, SUR-091, F18 | ACC-016, ACC-018, ACC-023 to ACC-029, ACC-032, ACC-034 | R2 | Server | Feed |
| API-USR-09 | What the admin can see | A page listing what administrators and guardians can see about this person, generated from the same policy the server enforces. | SUR-078 | ACC-115 | R1 | Server | No |
| API-USR-10 | Delete my account | Needs authentication in the previous 5 minutes and offers an export first. Disables the account, ends every session and device grant at once, keeps it restorable for 7 days, then erases it through the history-deletion pipeline; the confirmation states the date. | SUR-078 | ACC-136 | R1 | Server | Push (the account's devices are signed out) |

| ID | Who, and the object check | Auth | Rate | Capability URL | Security |
|---|---|---|---|---|---|
| API-USR-01 | Self: own profile only | Session | Write; Upload for pictures | Pictures are served as artwork capability URLs (1 hour, time-bucketed) | SEC-API-048, SEC-API-085 to SEC-API-088, SEC-PRV-006, SEC-MED-018, SEC-CLI-005 |
| API-USR-02 | Self: own profile; device-scope records only for the calling device | Session | Write | No | SEC-API-013, SEC-IAM-072, SEC-PRV-023 |
| API-USR-03 | Self: own credentials | Session + uv | Write | No | SEC-IAM-023, SEC-IAM-024, SEC-IAM-025, SEC-IAM-029, SEC-IAM-098, SEC-IAM-106, SEC-CLI-024 |
| API-USR-04 | Self: own events only | Session | Read | No | SEC-IAM-097, SEC-IAM-077, SEC-OPS-027 |
| API-USR-05 | Self: own data only | Session + uv | Job (and per-user rate limit) | Export download: single use, bound to the requesting session, at most 1 hour | SEC-PRV-047, SEC-PRV-048, SEC-PRV-022, SEC-IAM-106, SEC-API-064, SEC-API-071, SEC-STD-029 |
| API-USR-06 | Self: imports into own profile only; matches only to items the person can see | Session | Upload; Job | No | SEC-API-060, SEC-API-064, SEC-API-085, SEC-API-088, SEC-API-048, SEC-PRV-035 |
| API-USR-07 | Self: own session only | Session | Write | No | SEC-PRV-024, SEC-PRV-025, SEC-PRV-035, SEC-TM-054, SEC-EXT-045 |
| API-USR-08 | `household.profile` (owner and admins; parents if granted); guardians see only their managed profiles | Elevated | Write | No | SEC-IAM-064, SEC-IAM-097, SEC-PRV-022, SEC-PRV-028, SEC-PRV-029 |
| API-USR-09 | Self | Session | Read | No | SEC-IAM-104, SEC-PRV-027, SEC-TM-054 |
| API-USR-10 | Self: own account; the owner must transfer ownership first | Session + uv | Write (and per-user rate limit) | No | SEC-IAM-003, SEC-IAM-103, SEC-PRV-048, SEC-PRV-049, SEC-PRV-051 |

### Device

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-DEV-01 | Device registry | Lists each of the person's sessions and devices with client name, device class, key level, network type, coarse location and last use; ends one, or all but the current one. From R2 the list also holds API keys and app passwords. | SUR-078, F15 | ACC-068, ACC-069, ACC-070 | R1 | Server | No |
| API-DEV-02 | Sync status per device | The device's sync cursor and last sync time, for the storage screen and for admins. | SUR-075 | CLI-024 | R1 (the cursor and last sync time, kept by sync, WP-084); R1.1 (the storage screen, CLI-024) | Local | No |
| API-DEV-03 | Diagnostics from a device | Accepts a report the person built and reviewed on the device, through the upload path. An excerpt of the server log is attached only when the person sending it is an administrator, and is masked like a diagnostic bundle. | SUR-077, F12 | CLI-033 | R1.2 | Server | No |
| API-DEV-04 | Device capability report | What this device can decode and output, used by the decision engine and shown to the person. It is validated and bounded, and is never used for an authorisation decision. | SUR-048, SUR-077 | CLI-047 | R2 | Local | No |
| API-DEV-05 | New-device notice | Tells the person's other devices that a device enrolled or a credential was added or removed, with a one-step "This wasn't me" that revokes it. In the app only: over the event channel when connected, otherwise at the next sync; there is no remote push (CLI-077 is Later). | SUR-003 | ACC-071 | R1 | Server | Push |
| API-DEV-06 | Server picker support | Each server holds its own device key, credentials, library copy and pins for a client that uses several servers. | SUR-072 | CLI-018, ACC-014 | R2 | Local | No |

| ID | Who, and the object check | Auth | Rate | Capability URL | Security |
|---|---|---|---|---|---|
| API-DEV-01 | Self: own sessions and devices; admins with `session.manage` may end any non-owner session; only the owner ends the owner's | Session (Elevated for another person's) | Read; Write | No | SEC-IAM-042, SEC-IAM-043, SEC-IAM-044, SEC-IAM-077, SEC-OPS-033, SEC-PRV-003 |
| API-DEV-02 | Self: own devices; admins see device rows, never history | Session | Read | No | SEC-API-068, SEC-PRV-025 |
| API-DEV-03 | Self: own report; the log excerpt only for an admin sender | Session | Upload | No | SEC-API-085, SEC-API-088, SEC-OPS-030, SEC-PRV-046 |
| API-DEV-04 | Self: the calling device only | Session | Write | No | SEC-HIS-033, SEC-API-067 |
| API-DEV-05 | Self: notices only to the same account's devices | Session (delivery over API-SYS-10) | Socket | No | SEC-IAM-098, SEC-HIS-063, SEC-OPS-032, SEC-OPS-033, SEC-PRV-030 |
| API-DEV-06 | Self, per server | Session (per server) | Read | No | SEC-CLI-044, SEC-IAM-051, SEC-NET-060 |

### Sync and the device copy

The sync model is described in its own section [below](#the-sync-model).
These are the capabilities it needs.

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-SYNC-01 | Snapshot | The first full copy of the profile's synced library and user data, computed for that principal from its grants and restrictions as it is built. Proposed budget: 100,000 tracks in under 2 minutes (open decision 16). | SUR-000, SUR-020, SUR-022, F03 | CLI-022, ACC-030, ACC-037, DIS-140 | R1 | Server | No |
| API-SYNC-02 | Delta from a cursor | Everything that changed for this principal since the device's cursor, in one ordered request, so a device away for a month catches up at once. The cursor is opaque or MAC-protected and bound to the principal. A cursor older than the compaction horizon gets a fresh snapshot. | All browse surfaces | LIB-018, CLI-022, INT-006 | R1 | Server | Feed |
| API-SYNC-03 | Grant changes as removals | When a person loses access to a library or an item, the next delta removes it from the device, carrying only identifiers, never metadata. | SUR-022, F10 step 8 | ACC-037, ACC-030 | R1 | Server | Feed |
| API-SYNC-04 | Profile data in the sync | The profile's slice of the user log (queue, playlists, loves, ratings, hides, layouts, rules, settings, history) travels with the library, and nothing of anyone else's. Each kind joins in the release that ships it: ratings and dismissals in R1.1, Home layouts in R1.2, rules in R1.3. See the size note in the sync model. | SUR-020, SUR-029, SUR-011 | DIS-002, DIS-007, MUS-122, ACC-012 | R1 | Server | Feed |
| API-SYNC-05 | Artwork in fixed sizes | Images in a fixed set of sizes per device class, fetched by capability URL, with a tiny placeholder in the metadata so tiles never show a spinner. | SUR-023 | LIB-142, MUS-040, LIB-143 | R1 | Local once cached | No |
| API-SYNC-06 | Neighbour table | A table of each visible track's, album's and artist's nearest neighbours from credits, genres, era and the profile's own listening, within a size budget per device, for radio and "More like this". Other people's listening contributes only from those who opted in, and only for items at least three of them played. | SUR-024, SUR-011 | DIS-060, DIS-067, MUS-165 | R1.3 | Local | Feed |
| API-SYNC-07 | Prebuilt search index | A fallback: if building the index on the device misses the budget on the reference low-end device, the server ships an index segment built for that principal instead. | SUR-032 | DIS-084, DIS-019 | R1 (if needed) | Local | Feed |
| API-SYNC-08 | Partial sync | All metadata, but artwork within a budget with eviction, for TVs and small devices. | SUR-075 | CLI-023 | R2 | Local | Feed |
| API-SYNC-09 | Restricted profiles at sync | Kids profiles receive nothing they may not see, so nothing leaks through search, artwork, screensavers or launcher rows. | SUR-055, SUR-060 | DIS-144, DIS-155 | R2 | Server | Feed |
| API-SYNC-10 | Erasure tombstones | When history is deleted, the profile's feed carries a tombstone naming only event IDs or ID ranges, and every device purges those events on its next sync. | SUR-029, SUR-078 | MUS-184 | R1 | Server | Feed |
| API-SYNC-11 | Purge on revocation | A "session revoked" answer, a sign-out, an account or profile switch, or (from R2) a revoked device tells the client to delete that account's synced copy, caches and service worker. | SUR-078, player "Signed out" state | ACC-069 | R1 | Server | Push |

| ID | Who, and the object check | Auth | Rate | Capability URL | Security |
|---|---|---|---|---|---|
| API-SYNC-01 | Self: only items the visibility predicate admits for this principal, and only this profile's user data | Session | Read (the sync route declares its larger page size) | No | SEC-API-014, SEC-API-015, SEC-API-063, SEC-CLI-020, SEC-CLI-021, SEC-IAM-070, SEC-TM-026, SEC-PRV-001 |
| API-SYNC-02 | Self; a cursor replayed by another principal yields only that principal's data | Session | Read | No | SEC-API-015, SEC-API-025, SEC-CLI-020 |
| API-SYNC-03 | Self | Session | Read | No | SEC-API-015, SEC-API-017, SEC-IAM-076 |
| API-SYNC-04 | Self: own profile's Activity only | Session | Read | No | SEC-PRV-019, SEC-PRV-022, SEC-CLI-010, SEC-CLI-020 |
| API-SYNC-05 | Self: artwork only for visible items | Session (to sign); Capability (to fetch) | Read; Image | Artwork URL: 1 hour, aligned to a fixed time bucket so repeat requests reuse the URL; bound to session, image and size | SEC-API-026, SEC-API-027, SEC-API-029, SEC-MED-047, SEC-MED-051 |
| API-SYNC-06 | Self: entries only for visible items | Session | Read | No | SEC-CLI-020, SEC-PRV-022, SEC-PRV-023 |
| API-SYNC-07 | Self: index entries only for visible items | Session | Read | No | SEC-CLI-020, SEC-PRV-004 |
| API-SYNC-08 | Self | Session | Read | No | SEC-CLI-020, SEC-CLI-035 |
| API-SYNC-09 | Self: a managed profile gets only what its content policy admits | Session | Read | No | SEC-IAM-064, SEC-CLI-020, SEC-API-014 |
| API-SYNC-10 | Self: own profile's tombstones | Session | Read | No | SEC-PRV-049, SEC-PRV-052 |
| API-SYNC-11 | Self | Session | Read | No | SEC-CLI-009, SEC-API-039, SEC-IAM-017, SEC-IAM-053, SEC-IAM-076, SEC-PRV-019 |

### Library (administration)

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-LIB-01 | Create and configure a library | A library record with a kind (music in R1, room for others), one or more roots, a spoken-word flag, and "keep off Home". Roots are opened read-only, never a filesystem root. A new library is visible to the owner and administrators only until someone grants it (flows G4). | SUR-085, SUR-082 | LIB-001, LIB-003, LIB-004, LIB-011, LAT-010, DIS-012 | R1; R1.2 ("keep off Home"); R1.3 (spoken-word flag) | Host | Feed |
| API-LIB-02 | Folder browser with live checks | Lists directories only, never file contents, inside admin-configured browse roots after canonicalising and resolving symlinks, and checks readability, emptiness and storage type before saving. | SUR-082, SUR-085 | ADM-025, LIB-015, ADM-089 | R1 | Host | No |
| API-LIB-03 | Root settings | Exclusion patterns, watch for changes, poll interval and parallelism for shares, a safety-net schedule, read-only declared, and symlinks followed only into approved roots. | SUR-085 | LIB-006, LIB-013, LIB-014, LIB-015, LIB-007 | R1; R1.1 (exclusion patterns) | Host | No |
| API-LIB-04 | Library grants | Who may see each library; the grants filter sync, events and every fetch from the next request. By default a new library is granted to no member or guest. An increase in someone's library access raises an owner alert. | SUR-085, SUR-090 | ACC-037, MUS-027 | R1 | Host | Feed |
| API-LIB-05 | Change location | Points a root at a new path with a preview, keeping identity and history. | SUR-085, SUR-082 | LIB-031, ADM-051 | R1.1 | Host | Feed |
| API-LIB-06 | Rebuild a library | Rebuilds from the files plus the curation log, stating that fixes are kept. | SUR-085 | LIB-179, ADM-077 | R1 | Host | Push (progress) |
| API-LIB-07 | Artist splitting rules | Per-library separator and exception rules for artist strings. | SUR-085, SUR-086 | MUS-035, LIB-038 | R1 | Host | Feed |
| API-LIB-08 | Video and other kinds | Film, show and home-video kinds, tag and precedence choices, metadata language, provider order, locks, I/O profiles. | SUR-085 | LIB-002, LIB-010, LIB-055, LIB-128, LIB-183, ADM-087 | R2 | Host | Feed |

| ID | Who, and the object check | Auth | Rate | Capability URL | Security |
|---|---|---|---|---|---|
| API-LIB-01 | `library.manage`; the root must pass the containment and filesystem-root checks | Elevated; Fresh-uv to add or remove a root | Write | No | SEC-IAM-041, SEC-MED-033, SEC-MED-037, SEC-MED-038, SEC-OPS-054, SEC-IAM-070 |
| API-LIB-02 | `library.manage`; only paths inside the configured browse roots | Fresh-uv | Read | No | SEC-API-022, SEC-IAM-041, SEC-HIS-015, SEC-HIS-016 |
| API-LIB-03 | `library.manage`; the root must exist and be visible to the caller | Elevated | Write | No | SEC-MED-034, SEC-MED-041, SEC-STD-011 |
| API-LIB-04 | `user.manage`; a grant can never exceed what the caller holds | Elevated | Write | No | SEC-IAM-073, SEC-IAM-076, SEC-API-017, SEC-OPS-032, SEC-TM-026 |
| API-LIB-05 | `library.manage` | Fresh-uv | Write | No | SEC-IAM-041, SEC-MED-033, SEC-OPS-055 |
| API-LIB-06 | `library.manage` | Elevated | Job | No | SEC-API-064, SEC-NET-053, SEC-MED-018 |
| API-LIB-07 | `library.manage` | Elevated | Write | No | SEC-STD-011, SEC-API-048 |
| API-LIB-08 | `library.manage` | Elevated | Write | No | SEC-PRV-013, SEC-PRV-014 |

### Catalogue: artist, album, track (fields in the synced copy)

These are the fields the R1 screens draw from the device's copy. The
server's job is to derive them at scan time, in the scan worker, and keep
them current through the feed. Every string is normalised on ingest and
rendered as text.

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-CAT-01 | Credits and roles | Every credited artist linked, the display credit as tagged, album artist apart from track artist, roles (composer, conductor, lyricist, producer, remixer, performer), "Appears on", disambiguation for same-name artists. | SUR-023 to SUR-025, SUR-016, SUR-013 | MUS-001 to MUS-006, LIB-187 | R1; R1.1 (roles, one artist page across libraries) | Local | Feed |
| API-CAT-02 | Release model | Release groups and editions, release types, compilations, discs with titles, work groupings, original and release dates, sort names. | SUR-024, SUR-025 | MUS-008, MUS-010 to MUS-013, MUS-020, LIB-045, LIB-051, LIB-056 | R1; R1.1 (release groups, editions, release types, original dates); R2 (work groupings) | Local | Feed |
| API-CAT-03 | Tags for browse | Multi-valued genres, moods, styles, labels and grouping, the explicit flag, MusicBrainz IDs. | SUR-023, SUR-028 | MUS-017, MUS-019, MUS-047, MUS-036, INT-009 | R1; R1.1 (moods, styles, labels, grouping, explicit flag) | Local | Feed |
| API-CAT-04 | Technical and quality data | Codec, container, sample rate, bit depth, channels and bitrate, as data the client turns into badges. | SUR-016, SUR-013, SUR-023, SUR-002 | MUS-021, LIB-146, MUS-099, MUS-236 | R1 | Local | Feed |
| API-CAT-05 | Playback data per file | Encoder delay and padding, ReplayGain or R128 values (track and album), true peak, measured loudness where available, and the seek index. The player clamps gain taken from tags. See the size note in the sync model. | Player | MUS-069, MUS-071, MUS-084, MUS-085, MUS-086, MUS-088, LIB-064 | R1; R1.3 (measured loudness) | Local | Feed |
| API-CAT-06 | Lyrics | Embedded lyrics and `.lrc` sidecars, plain, line-timed and word-timed, parsed at scan into a capped timed-line model, with where they came from. | SUR-012 | MUS-154, MUS-155, MUS-156, LIB-067, LIB-068 | R1; R1.1 (word-timed lyrics) | Local | Feed |
| API-CAT-07 | Artwork palette | Up to three colour candidates per album computed from the same decode that makes the fixed sizes, for the artwork tint. | SUR-010, SUR-024, SUR-025 | MUS-110 | R1 | Local | Feed |
| API-CAT-08 | Provenance | How each field was read and where it came from, and "Upgraded on" when a better copy replaced the file. No file paths for non-admins. | SUR-013 | MUS-034, MUS-037, LIB-030 | R1 | Local | Feed |
| API-CAT-09 | Availability | Per item: playable, drive offline, damaged, missing; the client adds "cannot decode here" from its own capability probe. | SUR-023, SUR-011, SUR-003 | LIB-032, MUS-079, MUS-229, LIB-193 | R1 | Local | Feed |
| API-CAT-10 | Folder paths | Paths relative to each library root, for folder view and breadcrumbs. Absolute paths never enter the synced copy, for anyone; admins read them through API-CAT-11. | SUR-022, SUR-025 | LIB-008 | R1 (relative paths stored by the scan, WP-024, WP-102); R1.3 (folder view and breadcrumbs, LIB-008) | Local | Feed |
| API-CAT-11 | Inspect a file | For admins: every raw tag, the structure the parsers read, the identification decision and "Why is this here?", errors with their location. Shared by the file inspector, the track info sheet's admin fields and the CLI. | SUR-088, SUR-013, SUR-110 | ADM-125, LIB-195, LIB-059, LIB-097, LIB-098 | R1.2 (file inspector, CLI and the track info sheet's admin fields, WP-156); the R1.1 track info sheet shows only synced fields and the grouping reasons WP-146 records, with no admin inspect fields until R1.2 | Server | No |
| API-CAT-12 | Merge, split and alias | Admin corrections to artists and albums, stored as curation-log events that survive rescans and rebuilds. | SUR-089, SUR-004 | MUS-007, LIB-041, LIB-058, LIB-179 | R1.3 | Server | Feed |
| API-CAT-13 | Rescan one item | Re-reads one file or folder on request. | SUR-004, SUR-088 | LIB-012 | R1 | Server | Feed |
| API-CAT-14 | Metadata editing | The edit sheet with field locks and sources, bulk edit, labels, artwork picker, fix match, item history with undo. Edits live in the database, never in media folders. | SUR-089 | LIB-172 to LIB-178, LIB-138 to LIB-141 | R2 | Server | Feed |

| ID | Who, and the object check | Auth | Rate | Capability URL | Security |
|---|---|---|---|---|---|
| API-CAT-01 to API-CAT-10 | Self: fields only for items the visibility predicate admits; travel in the sync payload | Session (through sync) | Read | No | SEC-API-046, SEC-API-048, SEC-API-068, SEC-API-090, SEC-MED-013, SEC-MED-015, SEC-MED-018, SEC-MED-049, SEC-CLI-001, SEC-CLI-020, SEC-TM-031 |
| API-CAT-11 | `library.manage`; the item must exist in a root the caller administers | Elevated | Read | No | SEC-API-068, SEC-API-046, SEC-CLI-001 |
| API-CAT-12 | `library.manage`; every object named must be visible to the caller | Elevated | Write | No | SEC-API-012, SEC-IAM-072, SEC-OPS-020 |
| API-CAT-13 | `library.manage`; the path must lie inside a configured root | Elevated | Job | No | SEC-API-064, SEC-MED-018, SEC-MED-033 |
| API-CAT-14 | `library.manage` | Elevated | Write; Upload for artwork | Artwork as API-SYNC-05 | SEC-API-085, SEC-API-086, SEC-PRV-006, SEC-OPS-054 |

### Streaming

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-STR-01 | Sign a stream URL | A capability URL whose path carries a token of version, key ID, expiry, operation, representation, object ID and session handle under HMAC-SHA-256, checked again for the session and the grant on every use. | Player | ACC-122, MUS-066 | R1 | Server | No |
| API-STR-02 | Silent refresh | A fresh URL during a long pause or a three-hour mix without the listener noticing: the client refreshes URLs for upcoming queue items in the background and asks for a fresh one on the typed "media URL expired" error; a named R1 acceptance test (flows G10). | Player "Stream URL expired" state | ACC-122 | R1 | Server | No |
| API-STR-03 | Byte ranges of the original | Serves one range per request of the file path recorded at scan time, opened beneath its root as a regular file, with a server-chosen content type; each range response is capped and tracked so revocation can cut it. | Player | MUS-066, ACC-122 | R1 | Server | No |
| API-STR-04 | Audio packaging for browsers | Copies FLAC, Opus and MP3 frames into fragmented MP4 without re-encoding, from the scan's frame index, where a browser's Media Source Extensions need it. It runs in a worker process outside the server, streaming over a pipe under a step budget and a memory cap, as the R2 remuxer will. Per-browser support is unverified. | Player | MUS-230, MUS-067 | R1 | Server | No |
| API-STR-05 | Artwork bytes | Serves the fixed image sizes, generated by the server as JPEG, PNG or WebP, by capability URL under the same authorisation, never by cookie. See the caching note in Flags, item 7. | SUR-023 | LIB-142, ACC-121 | R1 | Local once cached | No |
| API-STR-06 | Opus streams | A sandboxed Opus encode for mobile data or a per-person cap, keeping pre-skip so gapless survives, cached for the next device. | SUR-002, SUR-075 | MUS-106, ACC-107 | R2 | Server | No |
| API-STR-07 | Video delivery | Direct play, remux in a worker process and sandboxed transcode, with segments cut from the scan-time segment map under one token per playback. | SUR-046 | VID-001, VID-003, VID-005, LIB-088 | R2 | Server | No |
| API-STR-08 | Single keyframe | One keyframe cut by the remuxer, rate-limited, for scrub previews and chapter thumbnails. | SUR-046, SUR-047 | VID-097, VID-098 | R2 | Server | No |
| API-STR-09 | Subtitles and fonts | Embedded and sidecar subtitle streams, parsed and re-serialised as WebVTT, under capability URLs, rendered on the client. Embedded fonts are off by default; a library may opt in, and each font is then re-serialised before delivery. | SUR-047 | VID-069, VID-072, VID-073 | R2 | Server | No |

| ID | Who, and the object check | Auth | Rate | Capability URL | Security |
|---|---|---|---|---|---|
| API-STR-01 | `library.read` for the item's library; the item must be visible to the principal | Session | Read; Stream | Stream URL: the item's duration plus 10 minutes, at most 4 hours; bound to session handle, item, operation and representation; signing key rotated automatically | SEC-API-026, SEC-API-027, SEC-API-030, SEC-IAM-047, SEC-PRV-021, SEC-OPS-015 |
| API-STR-02 | As API-STR-01 | Session | Read | As API-STR-01 | SEC-API-027, SEC-API-028 |
| API-STR-03 | The holder of a valid URL whose session is still live and whose principal can still see the item | Capability | Stream | Consumes the stream URL; `Cache-Control: private`, no longer than its remaining life | SEC-API-018, SEC-API-028, SEC-API-029, SEC-API-031, SEC-API-055, SEC-IAM-046, SEC-IAM-102, SEC-MED-012, SEC-MED-036, SEC-MED-059, SEC-NET-050, SEC-OPS-055, SEC-HIS-031 |
| API-STR-04 | As API-STR-03 | Capability | Stream; Job (worker concurrency) | As API-STR-01 (representation: packaged) | SEC-MED-018, SEC-MED-024, SEC-MED-007, SEC-NET-053, SEC-API-026 |
| API-STR-05 | As API-STR-03, for images of visible items | Capability | Image | Artwork URL: 1 hour, aligned to a time bucket; bound to session, image and a size from a fixed set | SEC-API-026, SEC-API-027, SEC-API-029, SEC-API-051, SEC-CLI-005, SEC-MED-046, SEC-MED-047, SEC-MED-051, SEC-PRV-016, SEC-TM-035 |
| API-STR-06 | As API-STR-01, within the person's quality right | Session; Capability | Stream; Job | As API-STR-01 (representation: Opus) | SEC-OPS-062, SEC-MED-024, SEC-NET-053, SEC-TM-044 |
| API-STR-07 | As API-STR-01 | Session; Capability | Stream; Job | One stream URL per playback; segment numbers checked against the stored map | SEC-API-026, SEC-MED-081, SEC-MED-024, SEC-TM-044, SEC-OPS-062 |
| API-STR-08 | As API-STR-01 | Capability | Image; Job | Artwork-style URL, as API-STR-05 | SEC-NET-053, SEC-MED-081 |
| API-STR-09 | As API-STR-01; fonts only from libraries that opted in | Capability | Image | Subtitle URL with the subtitle operation, lifetime as the stream | SEC-API-026, SEC-API-089, SEC-MED-054, SEC-CLI-053 |

### Playback session

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-SES-01 | Session registry | Records who is playing on which device, the delivery path, bitrate and the decision reason. The decision is computed by the core on the device (F17 step 2) and reported; the server records what it actually served. Admin views show the title only for people who opted in to showing titles, and never for a private session. R1 needs only the playing leases that stream limits count (API-SES-08). | SUR-083, SUR-084 | ADM-099, ADM-100, INT-134 | R1 (playing leases and the registry, WP-104); R1.2 (admin views) | Server | Push (admin views) |
| API-SES-02 | Stop a session with a message | An admin ends a session; new requests fail, in-flight responses are cut, and the client shows the message as plain text. In R1 an administrator already ends any or all of a non-owner's sessions, without a message or the live view, from Admin > Users > person (API-DEV-01, API-AUTH-09, SEC-IAM-044). | SUR-084, SUR-003, player "Stopped by the owner" | ADM-102, ACC-073 | R1.2 | Server | Push |
| API-SES-03 | Active player of a queue | Which of the person's own devices is currently playing a profile's queue, so a second browser shows "Playing on *device*" and "Play here" and the first stops when the second takes over. Two mechanisms are proposed (player open question 2, flows G11); pick one. **Proposal.** | SUR-002, player "Playing elsewhere" | CLI-103, MUS-122 | R1 | Server | Push |
| API-SES-04 | Play reporting | Each play with its real timestamp, counts and skips, carrying only the profile, the item's content identity, the device, timestamps, position and completion; nothing from a private session. | Player, SUR-029 | MUS-182, MUS-183, CLI-093, ACC-117 | R1 | Queue | Feed |
| API-SES-05 | Control channel | A WebSocket per signed-in session; a closed command set (transfer, play, pause, seek, skip, volume, tracks, speed) with no free text, each authorised per profile and limited to the person's own sessions unless control was granted; a written and tested conflict rule. | SUR-014, F08 | CLI-101, CLI-102, MUS-197, MUS-198, VID-144, VID-145 | R2 | Server | Push |
| API-SES-06 | Handoff | Moves the queue, lanes, position, shuffle order and repeat mode to another of the person's devices, which signs its own stream URL and fetches the original itself; the source keeps playing until the target confirms (flows G14). | SUR-014, SUR-061 | CLI-101, VID-144 | R2 | Server | Push |
| API-SES-07 | Casting | Signs refreshable URLs a receiver can fetch on the home network, scoped to one item and one cast session and never carrying a session token, picks a format it can decode, and lets the phone relay away from home. | SUR-051 | CLI-106, CLI-110, CLI-111, MUS-203 | R2 | Server | Push |
| API-SES-08 | Stream limits and policy refusals | Counts playing leases, not open apps. **R1:** per-account and server-wide concurrent-stream limits, set by the admin, with a typed "too many" refusal. **R2:** the reason and the alternatives a policy allows. | SUR-049 | VID-173, ACC-075, VID-010 | R1 (limits); R2 (policy alternatives) | Server | No |
| API-SES-09 | Item changed during play | Tells a live session that its item gained a subtitle or a version. | SUR-049 | VID-076, INT-122 | R2 | Server | Push |

| ID | Who, and the object check | Auth | Rate | Capability URL | Security |
|---|---|---|---|---|---|
| API-SES-01 | Report: Self, for own sessions. Read: Self sees own sessions; admins see live sessions of others without titles unless opted in, and each admin read is recorded in the subject's own log | Session; Elevated for admin views | Write (batched); Read (admin reads rate-limited) | No | SEC-PRV-024, SEC-PRV-025, SEC-TM-054, SEC-IAM-077, SEC-API-016, SEC-HIS-033 |
| API-SES-02 | `session.manage` for non-owner sessions; only the owner for the owner's | Elevated | Write | No | SEC-IAM-043, SEC-IAM-044, SEC-API-017, SEC-API-048, SEC-CLI-001 |
| API-SES-03 | Self: own profile's queue and own devices | Session | Write | No | SEC-HIS-014, SEC-API-016 |
| API-SES-04 | Self: own profile; every item named must be visible | Session | Write (batched) | No | SEC-PRV-002, SEC-PRV-024, SEC-API-012, SEC-HIS-033 |
| API-SES-05 | Self, or a person who granted control; closed command set | Capability (WebSocket ticket) or Session | Socket | WebSocket ticket as API-SYS-10 | SEC-HIS-014, SEC-API-016, SEC-API-043, SEC-NET-034 |
| API-SES-06 | Self: own devices only | Session | Write | Each device signs its own stream URL; none is handed over | SEC-HIS-014, SEC-API-026 |
| API-SES-07 | Self: own queue, to a cast session the person started | Session | Stream | Cast URL: one item and one cast session; expires within the item's duration plus at most 1 hour, refreshable, revocable; cross-origin headers only on cast representations | SEC-NET-064, SEC-API-027, SEC-API-098 |
| API-SES-08 | Applies to every stream | Not a route | Stream | No | SEC-IAM-102, SEC-TM-068, SEC-API-031, SEC-STD-030 |
| API-SES-09 | Self: own live sessions; event built per recipient | Session (over the event channel) | Socket | No | SEC-API-016 |

### Queue

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-QUE-01 | Queue document | One versioned queue per profile with three lanes, named listening contexts from the first version, an insertion cursor, a seeded shuffle order, repeat and stop-after modes, and the position. Stored in the user log so a rebuild keeps it. Items that stop being visible to the profile drop out of it. | SUR-011, SUR-002, SUR-010 | MUS-116, MUS-122, LAT-009, MUS-126, MUS-077 | R1 | Local (read) | Feed |
| API-QUE-02 | Queue operations | Small operations (play, play next, add, play last, start radio, move, remove, clear, shuffle, reshuffle) applied optimistically on the device against the last version seen; the server orders them and assigns versions; a multi-item drop is one operation, refused whole if any item is not visible. The verbs' rules live in the core. | SUR-011, SUR-004, SUR-005 | MUS-117 to MUS-120, MUS-128, MUS-063, MUS-165 | R1; R1.1 (reshuffle, reorder while shuffled, multi-item drops); R1.3 (start radio) | Server in R1; Queue in R2 | Push |
| API-QUE-03 | Stale-version rejection and rebase | Rejects an operation built on an old version; the client rebases and shows the result. Must be written and tested before handoff. | SUR-011 | MUS-122 | R1 | Server | Push |
| API-QUE-04 | Position updates | The playing device writes its position at play, pause, seek and track change, and periodically while playing, batched to respect the single writer. | SUR-002, CLI-103 prompt | MUS-122, CLI-103 | R1 | Queue | Feed |
| API-QUE-05 | Save queue as playlist | Creates a playlist from the current queue. | SUR-011 | MUS-125 | R1.1 | Server | Feed |
| API-QUE-06 | Undo, history and saved queues | Undo of queue edits, history above the current item, several queues with a switcher, and the music and video contexts side by side. | SUR-011, SUR-049 | MUS-121, MUS-124, MUS-131, VID-181 | R2 | Queue | Feed |

| ID | Who, and the object check | Auth | Rate | Capability URL | Security |
|---|---|---|---|---|---|
| API-QUE-01 to API-QUE-06 | Self: own profile's queue only; every item named must be visible to the profile, and a list with one invisible item is refused whole | Session | Write (positions batched); Read | No | SEC-API-012, SEC-API-013, SEC-API-016, SEC-IAM-070, SEC-PRV-022, SEC-HIS-014 |

### Playlist and rules

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-PL-01 | Manual playlists | Create, rename, add, reorder and remove, as user-log events that refer to tracks by content identity, not path. | SUR-026, SUR-015, SUR-001 | MUS-132, MUS-133, LIB-028 | R1 | Server in R1; Queue in R2 | Feed |
| API-PL-02 | Pin and love playlists | Pin and love events for the sidebar and Home shortcuts. | SUR-001, SUR-020 | MUS-139, DIS-013 | R1.1; R1.2 (Home shortcuts) | Server until R2; Queue in R2 | Feed |
| API-PL-03 | M3U import and export | Imports M3U and M3U8 through the upload path and the shared matcher, resolving entries only to items in libraries the playlist owner can read and dropping URLs, outside paths and artwork directives; exports M3U8 with paths relative to a library root, as a download. | SUR-026, SUR-009, SUR-097 | MUS-140, ADM-043, ADM-044 | R1.1 | Server | Push (import progress) |
| API-PL-04 | Playlists from music folders | `.m3u` files found in media folders appear as playlists to people granted that library, with entries resolved only to items already indexed in the same library. Media is read-only, so they need a read-only rule (flows G5). | SUR-022 | LIB-192, LIB-007 | R1.1 | Local | Feed |
| API-PL-05 | Rule store | Saved rule trees in the one rule language, for smart playlists, Home rows and saved filters, synced to devices; the core evaluates them on the device with seeded randomness. Loved tracks are a fixed query over love events until the rule engine arrives (MUS-149). | SUR-027, SUR-026, SUR-023 | DIS-119 to DIS-122, MUS-143 to MUS-146, DIS-105, MUS-149 | R1.1 (the core rule format and its parser budgets, with saved filters stored in it in the settings records; register D-85); R1.2 (saved filters as Home rows, DIS-003); R1.3 (the rule editor, smart playlists and rule-backed rows) | Local (evaluate); Server (save until R2) | Feed |
| API-PL-06 | Server-side rule evaluation | Evaluates rules on the server when the library changes, for tools reading a smart playlist through the API, for adapters and for download rules. The evaluation and its re-evaluation jobs arrive with smart playlists in R1.3 (WP-092, WP-113), on the rule format that ships in R1.1 (WP-027); tools, adapters and download rules read the results only from R2, with API keys (owner decision 8), and until then devices evaluate every rule they show. | SUR-026 | DIS-121, INT-138 | R1.3 (evaluation and re-evaluation jobs); R2 (tools, adapters and download rules) | Host | Feed |
| API-PL-07 | Playlist write API for tools | Tools create and edit playlists with a scoped API key that never holds an administrator scope; they appear like any other playlist. Moves to R2 with API keys (owner decision 8). | SUR-026, SUR-094 | INT-138, ACC-049 | R2 | Server | Feed |
| API-PL-08 | Missing entries | What a playlist shows when a track is purged from the trash. **Proposal** (flows G6): keep the entry as "missing" with its last known title so a later copy rematches. | SUR-026 | MUS-132, LIB-033 | R1 | Local | Feed |
| API-PL-09 | Folders, images, sharing and collaboration | Playlist folders, a custom image, sharing with people on the server and collaborators, offline edits. Each viewer sees only the entries they may see. | SUR-026, SUR-058 | MUS-136, MUS-138, ACC-091, MUS-150, MUS-152 | R2 | Queue | Feed |

| ID | Who, and the object check | Auth | Rate | Capability URL | Security |
|---|---|---|---|---|---|
| API-PL-01, API-PL-02 | Self: own playlists; every track added must be visible to the profile | Session | Write | No | SEC-API-012, SEC-API-013, SEC-PRV-022, SEC-MED-051 |
| API-PL-03 | Self: into and out of own playlists | Session | Upload; Job | The export is a download on the signed-in route, not a capability URL | SEC-API-085, SEC-API-088, SEC-HIS-018, SEC-MED-050, SEC-MED-051, SEC-MED-062 |
| API-PL-04 | Anyone granted the library; entries filtered per requester | Session (through sync) | Read | No | SEC-HIS-018, SEC-MED-050, SEC-MED-051, SEC-OPS-054 |
| API-PL-05 | Self: own rules | Session | Write | No | SEC-API-067, SEC-STD-011 |
| API-PL-06 | Key holder with a read scope; results filtered by the key's scope and its owner's visibility | Key | Read; Job | No | SEC-EXT-010, SEC-EXT-011, SEC-API-020, SEC-MED-051 |
| API-PL-07 | Key holder with a playlist-write scope; writes only to the key owner's playlists | Key | Write | No | SEC-EXT-010 to SEC-EXT-012, SEC-IAM-083, SEC-API-020, SEC-API-012 |
| API-PL-08 | Self | Session (through sync) | Read | No | SEC-MED-051 |
| API-PL-09 | Owner of the playlist; `playlist.share` to share; viewers and collaborators named by the owner see only entries visible to them | Session | Write; Upload for images | Images as API-SYNC-05 | SEC-MED-051, SEC-API-012, SEC-IAM-073, SEC-PRV-006, SEC-PRV-022 |

### User log: plays, loves, ratings, history and hides

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-LOG-01 | Append user events | One write path for every event a person authors: plays, loves, ratings, dismissals and their reversals. Each event has a client-generated ID, the device ID and a hybrid logical clock, so replays are idempotent, and carries only the fields the privacy baseline allows. | SUR-002, SUR-004, SUR-013, SUR-020 | MUS-180, MUS-181, DIS-045, DIS-047, DIS-022, LAT-006, LAT-007 | R1; R1.1 (ratings, dismissals) | Queue for plays; Server for the rest in R1 | Feed |
| API-LOG-02 | Offline plays merge | Ingests plays recorded while disconnected with their real timestamps and removes duplicates. Plays from a private session were never queued, so none arrive. | SUR-029 | CLI-093 | R1 | Queue | Feed |
| API-LOG-03 | Delete history | Deletes one play, a time range or all history. The erasure pipeline removes the data from the database, the history log, derived tables, indexes and caches within 24 hours; devices get a tombstone (API-SYNC-10) and purge it; the deletion is re-applied if an older backup is restored. The screen says plays already sent to Last.fm cannot be recalled. See Flags, item 8. | SUR-029, SUR-004 | MUS-184, ACC-118, DIS-052 | R1 | Server in R1 | Feed |
| API-LOG-04 | Derived counts | Play counts, last played and skips per item, derived from the profile's own log. | SUR-023, SUR-024 | MUS-182, DIS-051 | R1 | Local | Feed |
| API-LOG-05 | Hidden and dismissed | Dismiss from Continue rows with undo and a Hidden page; hide and snooze in R2. | SUR-020, SUR-030 | DIS-022, DIS-023, DIS-054, MUS-170 | R1.1; R2 (hide and snooze) | Server until R2 | Feed |
| API-LOG-06 | Statistics and year in review | Charts by period computed from the person's own log. | SUR-031 | MUS-186, MUS-187 | R2 | Local | Feed |
| API-LOG-07 | Resume points | Positions as events so resume works on any device, offline included, and survives replacing or renaming the file. | SUR-049 | VID-118, VID-120, VID-121 | R2 | Queue | Feed |

| ID | Who, and the object check | Auth | Rate | Capability URL | Security |
|---|---|---|---|---|---|
| API-LOG-01, API-LOG-02, API-LOG-05, API-LOG-07 | Self: own profile's log; every item named must be visible | Session | Write (batched) | No | SEC-PRV-002, SEC-PRV-024, SEC-API-012, SEC-API-067, SEC-HIS-033 |
| API-LOG-03 | Self: own profile's history only; a managed profile's guardian for that profile | Session | Write; Job (the erasure runs as a job) | No | SEC-PRV-049, SEC-PRV-050, SEC-PRV-052, SEC-PRV-022 |
| API-LOG-04, API-LOG-06 | Self | Session (through sync) | Read | No | SEC-PRV-022, SEC-HIS-060 |

### Home, discovery and search

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-HOME-01 | Home layout | Each person's rows, their order, names and rule sources, synced so Home is the same on every device; rows are evaluated on the device. | SUR-020, SUR-021 | DIS-003, DIS-007, MUS-049, DIS-009 | R1.2 | Local (read); Server (write until R2) | Feed |
| API-HOME-02 | Pinned shortcuts | Pins for items and playlists at the top of Home and in the sidebar. | SUR-020, SUR-001 | DIS-013 | R1.2 | Local (read) | Feed |
| API-HOME-03 | Recently added without upgrades | An "added" date per album that a better copy does not reset, grouped by album. | SUR-020 | DIS-035, DIS-036, DIS-038, MUS-059 | R1 | Local | Feed |
| API-HOME-04 | Reasons on suggestions | Every suggested row and pick carries a reason the client can show. A reason never names another person or reveals what they played. | SUR-020, SUR-011 | DIS-061, DIS-062 | R1 (a reason on each built-in Home row, WP-059); R1.3 (suggestions, DIS-061, DIS-062) | Local | Feed |
| API-HOME-05 | Search | Nothing at query time: the core builds the index from the synced copy. Recent searches stay on the device (design-language section 11), so the server stores none and never logs a query. | SUR-032 | DIS-083 to DIS-089 | R1 | Local | No |
| API-HOME-06 | Household defaults and curation | Default Home layouts, offering a layout to someone (who accepts it), genre merges, the owner's picks. | SUR-107, SUR-021 | DIS-005, DIS-006, DIS-108, DIS-127 | R2 | Server | Feed |
| API-HOME-07 | Follows and alerts | Follow records in the user log and an in-app inbox that syncs like the library. Delivery to the person's own ntfy topic is a webhook the person sets up, under the webhook rules and the egress gate. | SUR-003, SUR-024 | INT-050 | R2 | Local (inbox) | Feed |

| ID | Who, and the object check | Auth | Rate | Capability URL | Security |
|---|---|---|---|---|---|
| API-HOME-01 to API-HOME-04 | Self: own layout, pins and suggestions; only visible items | Session | Write; Read | No | SEC-PRV-022, SEC-CLI-020, SEC-API-012 |
| API-HOME-05 | Self, on the device | None (local) | None | No | SEC-PRV-004, SEC-API-063 |
| API-HOME-06 | `server.settings` for household defaults and merges; a layout reaches a person only when they accept it | Elevated (admin); Session (accept) | Write | No | SEC-PRV-026, SEC-IAM-072 |
| API-HOME-07 | Self: follows and inbox; alerts only for items the follower can see | Session | Write; Read | No | SEC-PRV-030, SEC-EXT-045, SEC-EXT-047, SEC-MED-051, SEC-TM-075 |

### Share links

The baseline made share links for music an R1 feature (security owner
decision 7), which the feature map had in R2 (ACC-086 to ACC-088). The
owner's adopted R1 scope (D-10) puts them in R1.2, and SEC-API-097 and
SEC-STD-008 ship with them; video share links stay R2, off by default.

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-SHR-01 | Create and manage share links | A person shares one track, album or playlist they can see. The link carries a 128-bit secret in its fragment, is listen-only by default (downloads only where the owner allows them server-wide), expires after 30 days by default, may have a password, and is visible and editable only by its creator and admins. Managed profiles cannot create them. Revocation bites on the next request. | SUR-058, SUR-078 | ACC-086, ACC-087, ACC-088 | R1.2 (music); R2 (films and episodes, off by default) | Server | No |
| API-SHR-02 | Public share page | The landing page reads the secret from the fragment and posts it; it shows the shared item and nothing about the sharer, other users, the library or anyone's activity, sends no link-preview metadata unless the sharer turned it on, and is marked noindex. | SUR-059 | ACC-086, ACC-092 | R1.2 (music); R2 (video) | Server | No |
| API-SHR-03 | Share-link limits | Per link: 2 concurrent streams by default, a total-bytes or uses cap, and a distinct-address count that suspends the link and alerts the sharer when exceeded. Password guesses follow the guessable-secret schedule and never disable the link. | SUR-058, SUR-059 | ACC-087 | R1.2 | Server | Push (the sharer's alert) |

| ID | Who, and the object check | Auth | Rate | Capability URL | Security |
|---|---|---|---|---|---|
| API-SHR-01 | Self, on an object the person can see; not managed profiles; never more than the creator holds; edits only by the creator and admins | Session | Write | Share link: 128-bit secret in the fragment; 30 days by default; scoped to one object and its rights | SEC-API-097, SEC-IAM-073, SEC-PRV-029, SEC-PRV-055, SEC-NET-015, SEC-STD-029 |
| API-SHR-02 | The holder of the secret; access re-evaluated with the sharer's current permissions on every request | Exchange (secret posted from the fragment), then Capability for the media | Public; Secret for passwords | Stream and artwork URLs as API-STR-01 and API-STR-05, bound to the share session | SEC-API-097, SEC-PRV-031, SEC-MED-051, SEC-API-056, SEC-STD-008, SEC-TM-004 |
| API-SHR-03 | Applies to every share link | Not a route | Stream; Secret | No | SEC-API-097, SEC-API-056 |

### Scan jobs and tasks

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-SCAN-01 | Start a scan | A full or library scan on request; a repeated request joins the running scan. | SUR-085 | LIB-012 | R1 | Host | Push |
| API-SCAN-02 | Path-scoped refresh | Refreshes one path inside a configured root. **R1:** for admins. **R2:** also for tools such as Lidarr with a scoped API key limited to named roots (owner decision 8). | SUR-094, SUR-100 | INT-011 | R1 (admins); R2 (API keys) | Server | Push |
| API-SCAN-03 | Scan progress | For admins: files found, bytes read per root, an estimate, and batches committed. For everyone else: their granted libraries filling in while the scan runs, with no paths or root detail. | SUR-083, SUR-100, SUR-022, SUR-003 | LIB-021, LIB-022, ADM-088, ADM-031, MUS-043 | R1; R1.3 (bytes read per root) | Host | Push |
| API-SCAN-04 | Task list | One list of tasks (backup, scan, purge, analysis, rebuild, retention, erasure, key rotation) with run, cancel, progress, last run, duration and errors. | SUR-100 | ADM-093, ADM-095 | R1 (the task engine and reading tasks, ADM-095, WP-070, WP-100); R1.2 (the task list with run and cancel, ADM-093) | Host | Push |
| API-SCAN-05 | Activity and audit log | Two views. The activity log (scans, "0 changed" rescans, moves, re-reads after a parser update, imports) for admins. The security audit log (sign-in, admin and recovery events), tamper-evident, readable in full only by the owner and holders of `audit.read`, with other people's addresses shortened. | SUR-100, SUR-083 | ADM-110, LIB-016, LIB-017, LIB-025, LIB-029, ACC-078 | R1 | Host | Push |
| API-SCAN-06 | Reprioritise, schedule and pause | Cancel and reprioritise jobs, a maintenance window, concurrency and pause. | SUR-100 | LIB-023, ADM-094, ADM-096 | R2 | Host | Push |

| ID | Who, and the object check | Auth | Rate | Capability URL | Security |
|---|---|---|---|---|---|
| API-SCAN-01 | `library.manage` | Elevated | Job | No | SEC-API-064, SEC-NET-053, SEC-MED-018, SEC-MED-019 |
| API-SCAN-02 | `library.manage` (R1); a key with a scan scope limited to its roots (R2); the path must resolve inside a configured root | Elevated; Key | Job | No | SEC-API-064, SEC-MED-033, SEC-HIS-015, SEC-EXT-010, SEC-EXT-011 |
| API-SCAN-03 | Admins see root detail; others see only their granted libraries; events built per recipient | Session (over the event channel) | Socket | No | SEC-API-016, SEC-API-068, SEC-CLI-020 |
| API-SCAN-04 | `server.settings` | Elevated | Read; Job | No | SEC-API-064, SEC-OPS-020 |
| API-SCAN-05 | Activity: `server.settings`. Audit: the owner and `audit.read` holders; addresses truncated to /24, /48 or country for others' events | Elevated | Read | No | SEC-OPS-020, SEC-OPS-021, SEC-OPS-023, SEC-OPS-027, SEC-IAM-094, SEC-IAM-097, SEC-PRV-003 |
| API-SCAN-06 | `server.settings` | Elevated | Write | No | SEC-API-064 |

### Library health, review queue and trash

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-HLTH-01 | Health report | Damaged and unreadable files, files quarantined after crashing the parse worker, tag problems with fixes, same-name collisions, sidecar problems, files the supported browsers cannot decode, missing files, moved files, offline roots, watch warnings. | SUR-086 | MUS-044, LIB-193, LIB-194, LIB-034, LIB-032, LIB-014, MUS-229 | R1; R1.1 (tag problems, missing-files list) | Host | Feed |
| API-HLTH-02 | Root health | Each root's reachability and state, offline detection that greys items rather than removing them. | SUR-083, SUR-085 | ADM-108, LIB-032 | R1 | Host | Push |
| API-HLTH-03 | Review queue | Doubtful decisions with evidence and a proposal; accept, reject or choose another; answers kept in the curation log. | SUR-087 | LIB-099, LIB-051 | R1.3 | Host | Feed |
| API-HLTH-04 | Trash | Items whose files went missing, with when they will be purged; restore and purge now; never purges while a root is offline. | SUR-105 | LIB-033, ADM-086 | R1 | Host | Feed |
| API-HLTH-05 | Health summary | The admin home's roll-up: backups and verification, roots, free space, scan state, alerts, advisories, token expiry, recovery used, and the security state: root or capability refusal, internet exposure and listeners on public addresses, trusted proxies, the isolation tier of each sandbox profile, audit-log verification, certificate expiry and version support. The security state is the R1 security summary (ADM-142); the full roll-up around it is R1.2. | SUR-083 | ADM-142, ADM-109, ADM-065, ADM-072, ADM-083, ACC-127, INT-019, ADM-034 | R1 (security summary); R1.2 (health roll-up) | Host | Push |

| ID | Who, and the object check | Auth | Rate | Capability URL | Security |
|---|---|---|---|---|---|
| API-HLTH-01 to API-HLTH-04 | `library.manage` | Elevated | Read; Write | No | SEC-API-068, SEC-MED-019, SEC-TM-069 |
| API-HLTH-05 | `server.settings` | Elevated | Read | No | SEC-OPS-061, SEC-MED-024, SEC-NET-028, SEC-NET-072, SEC-OPS-047 |

### Users and invitations (administration)

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-ADM-01 | Local users | List users with their libraries; enable or disable without deleting (sessions end at once); create, promote and demote administrators (owner only); move ownership only by an explicit transfer that the owner and the recipient each confirm with user verification in the previous 5 minutes, so exactly one owner exists at all times. | SUR-090 | ACC-006, ACC-008, ACC-040, ADM-052 | R1; R1.2 (creating, promoting and demoting administrators) | Host | No |
| API-ADM-02 | Invitations | An invite is a capability with libraries, a preset no greater than the inviter's own rights, a use count (default 1) and an expiry (default 7 days); the secret travels in the link's fragment; a link and a QR code carry the server's configured public address, never one taken from the request; redemption is logged and the inviter is told. An invitation that confers member level or more than one library stays pending until the inviter confirms a short code with the new person. The invite screen should show and check that address (flows G7). | SUR-090, SUR-071 | ACC-080 | R1 | Host | No |
| API-ADM-03 | Invite landing | Before redeeming, shows a privacy notice generated from the server's configuration, and nothing else about the server or its people. Redemption enrols the invitee's own passkey or (from R1.2) OIDC link in the same transaction and creates the account from the invite's policy; it works only in a secure context. | SUR-071 | ACC-080, ACC-006 | R1 | Server | No |
| API-ADM-04 | Policies and rights | Named policies; playback, download, quality, device and remote-access rights; memberships that end on a date; stream and transcode limits. | SUR-090, SUR-091 | ACC-038, ACC-043, ACC-044, ACC-081, ACC-103, ACC-107, ACC-108, ACC-111 | R2 | Host | Feed |

| ID | Who, and the object check | Auth | Rate | Capability URL | Security |
|---|---|---|---|---|---|
| API-ADM-01 | `user.manage` for members and guests; the owner (`admin.manage`) for administrators; the owner and recipient (`ownership.transfer`) for the transfer; nobody acts as another user | Elevated; Fresh-uv for creating or promoting administrators and for the transfer | Write | No | SEC-IAM-003, SEC-IAM-044, SEC-IAM-073, SEC-IAM-075, SEC-IAM-076, SEC-IAM-103, SEC-PRV-026, SEC-OPS-032, SEC-OPS-034, SEC-TM-028 |
| API-ADM-02 | `invite.guest` or `invite.member`; preset and libraries never more than the inviter holds | Elevated | Write | Invitation link: 128-bit secret in the fragment, 7 days, 1 use, revocable until used | SEC-IAM-078, SEC-IAM-079, SEC-IAM-080, SEC-API-069, SEC-API-096, SEC-NET-036, SEC-IAM-073, SEC-STD-029 |
| API-ADM-03 | The holder of the invitation secret | Exchange | Ceremony (redemption rate-limited per source and server-wide) | Consumes the invitation link | SEC-API-096, SEC-API-058, SEC-IAM-079, SEC-PRV-053, SEC-PRV-031, SEC-CLI-013, SEC-NET-047 |
| API-ADM-04 | `user.manage`; never more than the caller holds | Elevated | Write | No | SEC-IAM-073, SEC-IAM-102, SEC-CLI-015 |

### Server settings and operations

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-SET-01 | Network settings | The posture (home by default), trusted reverse proxies each declared "private overlay" or "public", the configured origins and public URL, a path prefix, HTTPS with a certificate the owner supplies or one the server obtains and renews for the owner's domain by ACME DNS-01, remote administration (off by default). In R1 remote use goes through the owner's reverse proxy or a tailnet. The per-server name from the project name service is a choice here only from R2. | SUR-093, SUR-082 | ACC-097, ACC-134, ACC-098, ADM-022, ADM-023 | R1; R1.2 (path prefix); R2 (per-server name) | Host | No |
| API-SET-02 | Privacy and the egress gate | Every outbound feature listed and off by default; the required provider step (from R1.1, with the built-in providers); a proxy for all egress and an offline mode; grants recorded; the network activity page accounts for every connection. | SUR-093, SUR-082 | ADM-028, ACC-113, LIB-108, ADM-129 | R1 | Host | No |
| API-SET-03 | Sign-in settings | OIDC provider configuration with a test, the limiter's state, and session lifetimes, which may be shortened but never lengthened past the baseline's. | SUR-092 | ACC-057, ACC-063, ACC-079 | R1; R1.2 (OIDC provider) | Host | No |
| API-SET-04 | Backups | List with status and verification, back up now, contents in plain words, download, upload, restore with a restore point and preview, and the owner's server export (settings without secrets, library roots, the curation log, household data and the owner's own data, never another adult's history; SEC-PRV-025). Every backup is encrypted and signed; restore verifies the signature, opens the archive in a jailed worker under limits, re-applies history deletions made since, then rotates every key, ends every session and asks the owner to review devices and access. | SUR-098, SUR-082 | ADM-065, ADM-066, ADM-069, ADM-070, ADM-072, ADM-074, ACC-013, ADM-141 | R1; R1.2 (restore from the UI, full export) | Host | Push (progress) |
| API-SET-05 | Restore at setup | Restore from the welcome screen behind the same claim code as claiming, with a dry-run remap of library roots and a warning when the domain changed (flows G3, G15), under the restore rules of API-SET-04. | SUR-082, F14 | ADM-029, ADM-051 | R1; R1.1 (remapping moved roots) | Host | Push (progress) |
| API-SET-06 | Updates | The update and advisory check is a required first-run question with two explicit answers and no preselection. When on, a plain GET of a signed static feed verified against a root compiled into the binary; advisories against the running version and the rollback-safety field per release are shown to admins. The feed can never disable, change or run anything on the server. | SUR-099, SUR-083 | ADM-053, ADM-054, ADM-060, ACC-126 | R1 | Host | No |
| API-SET-07 | Alerts and logs | In-app owner alerts in R1, non-critical ones batched into a daily summary; security alerts the baseline lists can never be switched off; every alert about a device or credential offers "This wasn't me". Outbound alert destinations come in R2, opt-in, through the egress gate, carrying only the event type, the time and a link. Log settings with rotation; debug level switches itself off within 24 hours. | SUR-101 | ADM-116, ADM-083, ADM-119 | R1 (in-app); R2 (outbound destinations) | Host | Push |
| API-SET-08 | Diagnostics | Run the doctor, build a masked bundle (no database, backups or secrets; paths, titles, names and addresses replaced by pseudonyms) shown in full before download, list local crash records, show the write queue and the derived-data store, rebuild the cache. | SUR-102, SUR-110 | ADM-123, ADM-124, ADM-130, ADM-080, ADM-141, ADM-077 | R1; R1.2 (diagnostic bundle, crash records); R1.3 (derived-data store) | Host | Push (progress) |
| API-SET-09 | Server identity and about | Server name and sign-in message, storage locations, version, build, target and live footprint, for signed-in admins. | SUR-103 | ADM-140, ADM-090, ADM-001, ADM-010 | R1; R1.2 (server name and sign-in message, footprint) | Host | No |
| API-SET-10 | Restart and shut down | From the UI and the emergency page. | SUR-083, SUR-081 | ADM-112, ADM-113 | R1.2 | Host | Push (clients see the startup page) |
| API-SET-11 | Migration imports | Listening-service files and playlists in R1.1, into the admin's own profile or as server playlists; iTunes library files, dry runs and undo in R2; Plex, Jellyfin, Emby and Navidrome databases Later, opened read-only in a jailed worker. One matcher with reasons and an unmatched queue. Another person's history is imported only into a pending import that person accepts. | SUR-097 | ADM-030, ADM-042 to ADM-044; ADM-036 to ADM-049 in R2 and Later | R1.1; R2 (iTunes, dry runs, undo); Later (rival databases) | Host | Push (progress) |
| API-SET-12 | Remote access | iroh with no open ports, self-hosted and default relays, upload budget; publishes relay records only by default. Built in from R2; until then remote use goes through the owner's reverse proxy or a tailnet (API-SET-01). | SUR-093 | ACC-096, ACC-100, ACC-101, ACC-109 | R2 | Host | No |
| API-SET-13 | Rotate server secrets | One owner action, from the dashboard and the CLI, that rotates every server secret, re-encrypts stored secrets, invalidates every session and signed URL, and is audited and alerted, without forcing devices with valid keys to pair again. | SUR-092 | ADM-144 | R1 | Host | Push (every client is signed out) |

| ID | Who, and the object check | Auth | Rate | Capability URL | Security |
|---|---|---|---|---|---|
| API-SET-01 | The owner (`security.settings`) | Fresh-uv | Write | No | SEC-IAM-041, SEC-IAM-075, SEC-NET-013, SEC-NET-016 to SEC-NET-019, SEC-NET-024, SEC-NET-045, SEC-OPS-038, SEC-API-038, SEC-API-069 |
| API-SET-02 | The owner (`security.settings`) | Fresh-uv | Write | No | SEC-TM-048, SEC-TM-075, SEC-PRV-007, SEC-PRV-008, SEC-PRV-012, SEC-PRV-013, SEC-API-076, SEC-API-079, SEC-OPS-060 |
| API-SET-03 | The owner (`oidc.configure`, `security.settings`); admins may view the limiter | Elevated | Write; Read | No | SEC-IAM-030, SEC-IAM-031, SEC-IAM-041, SEC-IAM-075, SEC-API-056, SEC-OPS-049 |
| API-SET-04 | Status and back up now: `server.settings`. Download, upload, restore and full export: the owner | Elevated; Fresh-uv for download, upload, restore and export | Read; Job | No; the download is served on the authenticated route and never under a web-served path | SEC-OPS-024, SEC-OPS-041 to SEC-OPS-045, SEC-PRV-039, SEC-PRV-041, SEC-PRV-049, SEC-IAM-105, SEC-STD-031 |
| API-SET-05 | The claim-code holder, from loopback or a secure context, while unclaimed | Exchange | Secret | No | SEC-OPS-003, SEC-OPS-008, SEC-OPS-043, SEC-OPS-044, SEC-STD-031 |
| API-SET-06 | The answer: the claim-code holder in setup, then the owner. Viewing advisories: `server.settings` | Exchange; Fresh-uv to change the answer (egress policy); Elevated to view | Write; Read | No | SEC-OPS-019, SEC-OPS-046, SEC-OPS-047, SEC-OPS-068, SEC-SUP-050, SEC-SUP-051, SEC-TM-067 |
| API-SET-07 | `server.settings` for rules and log settings; the owner for any change that weakens an alert | Elevated | Write | No | SEC-OPS-029, SEC-OPS-032 to SEC-OPS-035, SEC-PRV-030, SEC-PRV-045 |
| API-SET-08 | `server.settings` | Elevated | Job | No; the bundle is downloaded on the authenticated route and deleted after download or 24 hours | SEC-OPS-030, SEC-OPS-061, SEC-PRV-046, SEC-STD-023 |
| API-SET-09 | `server.settings` (paths in admin response types only) | Elevated | Read | No | SEC-API-068, SEC-NET-047 |
| API-SET-10 | `server.settings` | Elevated | Write | No | SEC-NET-045, SEC-OPS-020 |
| API-SET-11 | `user.manage` and `library.manage`; imports only into the caller's own profile or as pending imports for others | Elevated | Upload; Job | No | SEC-API-085, SEC-API-088, SEC-PRV-026, SEC-STD-031, SEC-TM-038, SEC-HIS-034 |
| API-SET-12 | The owner | Fresh-uv | Write | No | SEC-IAM-041, SEC-NET-030, SEC-NET-037 to SEC-NET-043, SEC-OPS-040, SEC-PRV-059 |
| API-SET-13 | The owner; also on the host | Fresh-uv; Host | Write | No | SEC-OPS-015, SEC-OPS-018, SEC-OPS-032, SEC-IAM-041 |

### Tokens and integrations

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-TOK-01 | Scoped tokens | API keys scoped by an explicit list of scopes (library, root), with expiry (365 days by default), automatic disabling after 180 days unused, last use, an audit trail, per-key rate limits, and revoke one or all; never an administrator scope, never more than the creator holds, and no key can manage keys. Moves to R2 (owner decision 8). | SUR-094, SUR-078 | ACC-049, INT-017 to INT-022, INT-012 | R2 | Host | No |
| API-TOK-02 | Change feed for tools | The same change log that syncs devices, with a cursor per key, filtered by the key's scope and its owner's current visibility. Moves to R2 with API keys. | SUR-094 | INT-006 | R2 | Server | Feed |
| API-TOK-03 | Stable deep links | Links to an album, artist or playlist that open the app or the web client and never grant access on their own. | SUR-004, SUR-057 | CLI-034, INT-147 | R1 (the parser, WP-239; resolution / R1's links, WP-089); R1.2 (deep links into the app, WP-159) | Local | No |
| API-TOK-04 | Webhooks and event stream | Webhooks with event picker, templates, Standard Webhooks signing, delivery log (success or failure only) and retry; a server-sent event stream for tools. Private listening emits nothing. | SUR-094 | INT-030 to INT-049 | R2 | Host | Push |
| API-TOK-05 | Plugin host | WebAssembly plugins with grants, a network allowlist and log, per-user secrets; providers, scrobblers, lyrics lookup. No plugin code loads until the plugin sandbox requirements pass. | SUR-095 | INT-054 to INT-069, MUS-162 | R2 | Host | No |
| API-TOK-06 | Compatibility adapters | OpenSubsonic and the Jellyfin music subset, off by default with no listener bound, on their own port, behind the same policy layer, with per-app keys and no plaintext LAN exception. | SUR-096, F19 | INT-086, INT-087, INT-098, ACC-130 | R2 | Server | No |

| ID | Who, and the object check | Auth | Rate | Capability URL | Security |
|---|---|---|---|---|---|
| API-TOK-01 | Self, within own non-administrator rights | Session + uv to create or change scopes; Session to list and revoke | Write | No; the key is shown once | SEC-IAM-083, SEC-EXT-008 to SEC-EXT-017, SEC-API-020, SEC-IAM-073 |
| API-TOK-02 | Key holder with a read scope | Key | Read | No | SEC-API-015, SEC-API-025, SEC-EXT-011 |
| API-TOK-03 | Anyone may hold a link; opening it needs the normal sign-in | None (client-side parse) | None | No; a deep link is not a credential | SEC-CLI-025, SEC-API-070, SEC-CLI-039 |
| API-TOK-04 | Server-wide webhooks: admins, after the owner enables the feature and its host allowlist. A person's own webhooks: Self, for own events, to allowlisted hosts; guests none by default | Elevated (server-wide); Session (own) | Write | No | SEC-EXT-045 to SEC-EXT-050, SEC-EXT-001, SEC-OPS-035 |
| API-TOK-05 | The owner (`plugin.approve`) | Fresh-uv | Write | No | SEC-EXT-018 to SEC-EXT-044, SEC-IAM-075, SEC-IAM-086 |
| API-TOK-06 | Enable: the owner. Use: holders of a per-app key, read and play only, within the person's visibility | Fresh-uv (enable); Key (adapter credential) | Ceremony (failed sign-ins share the limiter); Read; Stream | Adapter stream URLs embed Gunmetal's short-lived capability URLs | SEC-API-093, SEC-API-094, SEC-EXT-051 to SEC-EXT-074, SEC-PRV-032, SEC-NET-001 |

### Download (R2)

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-DL-01 | Offline grant | The device key plus a signed record of what the device may play and until when (30 days by default, 1 to 90 set by the admin), renewed on any contact; revocation applies on next contact. | SUR-050, SUR-091 | CLI-095, CLI-096, ACC-045 | R2 | Local (play) | Feed |
| API-DL-02 | Download files | Resumable ranges of originals, remuxed copies where the device prefers another container, Opus copies encoded once and cached; markers, chapters, subtitles and (where the library opted in) fonts travel with them; streams take priority. | SUR-050 | CLI-078, CLI-084, CLI-085, CLI-090, MUS-213, VID-175, ACC-110 | R2 | Server | No |
| API-DL-03 | Download rules | Rules in the rule language ("keep Loved tracks", "keep the next episodes") evaluated against the synced copy. | SUR-050, SUR-027 | CLI-080, CLI-081, CLI-088, MUS-211, MUS-212 | R2 | Local | Feed |
| API-DL-04 | Download on another device | Starts a download on another of the person's devices. | SUR-050, SUR-004 | CLI-097 | R2 | Server | Push |
| API-DL-05 | Download rights | A per-person download right, with the reason shown when it is missing. | SUR-050 | ACC-044 | R2 | Server | Feed |

| ID | Who, and the object check | Auth | Rate | Capability URL | Security |
|---|---|---|---|---|---|
| API-DL-01 | Self: own device; only items visible to the profile in libraries where it holds `library.download` | Session (device) | Write | No; the grant is a signed record bound to the device key and an item list | SEC-IAM-054, SEC-CLI-036, SEC-CLI-035 |
| API-DL-02 | `library.download` for the item's library; guests and managed profiles have none by default | Session; Capability | Stream; Job | Download URL with the download operation, bound to session, item and representation, lifetime per the stream row of SEC-API-027 | SEC-API-026, SEC-API-027, SEC-API-028, SEC-IAM-080, SEC-MED-054 |
| API-DL-03 | Self | Session (through sync) | Read | No | SEC-CLI-015, SEC-CLI-020 |
| API-DL-04 | Self: own devices only | Session | Write | No | SEC-HIS-014 |
| API-DL-05 | `user.manage` to set; enforced by the server on every request | Elevated | Write | No | SEC-CLI-015, SEC-IAM-073 |

### Video and live TV, in outline

Video and live TV need the same patterns as music (synced metadata,
capability URLs, the user log, the session registry), plus their own
resources. They are listed in outline here, because R1 should leave room
for them rather than build them. Each row brings its own security
requirements with it: the remuxer in a worker (SEC-MED-081), the transcode
jail (SEC-TM-044, SEC-OPS-062), subtitle parsing (SEC-API-089), embedded
fonts off by default (SEC-MED-054), and for live TV the egress rules for
sources and guides (SEC-API-082, SEC-NET-067, SEC-PRV-060, SEC-EXT-075),
proxied logos and streams (SEC-CLI-067) and the recordings root
(SEC-OPS-063).

| ID | Capability | Features | Rel. |
|---|---|---|---|
| API-VID-01 | Films, shows, seasons, episodes, collections, people and versions in the synced copy, including the stream index the core needs to compute "plays directly here" on the device | LIB-002, LIB-149 to LIB-155, VID-013 to VID-015, DIS-129 | R2 |
| API-VID-02 | Watch log, Continue Watching and Next Up rows | DIS-024 to DIS-031, VID-118 to VID-122 | R2 |
| API-VID-03 | Skip markers as logged data, season-wide intro detection, a skip policy in synced settings, marker corrections | VID-110 to VID-116 | R2 |
| API-VID-04 | Subtitle upload and per-file or per-series offsets as user-log events | VID-075, VID-084 | R2 |
| API-VID-05 | Pre-made smaller versions made at idle in the sandbox | VID-026 | R2 |
| API-VID-06 | Playback diagnostic bundles and a short session trace | VID-174, ADM-126 | R2 |
| API-LIV-01 | Live TV module switch, sources, filters, guide sources, channel mapping, lineup | LIV-001 to LIV-063 | R3 |
| API-LIV-02 | A compact guide synced to the device | LIV-064, LIV-065 | R3 |
| API-LIV-03 | Live playback from a fan-out buffer, time-shift, tuner arbitration | LIV-078, LIV-080, LIV-087, LIV-095 | R3 |
| API-LIV-04 | Recordings, series rules, conflicts and the recordings trash | LIV-102 to LIV-141 | R3 |

## The sync model

The interface documents promise that every list and search answers from the
device, that Home opens with no spinner, and that losing the server never
blocks browsing (CLI-022, CLI-025, DIS-002). This section sets out what that
requires. Because the synced copy is how the client reads, it is also where
grants, revocation and private listening must hold; the server cannot
re-check a read the device answers from its own copy, so the copy must
never hold what the profile may not see. Where the feature map, the UI
documents or the security baseline settle a point, it is cited; where they
do not, the text says **Proposal**.

### What is copied to the device

The payload is computed per principal, through the same visibility
predicate as every other read, while it is built (SEC-API-015,
SEC-CLI-020, SEC-IAM-070; DIS-144 adds content policy for kids profiles in
R2). Every field carries a data class (SEC-PRV-001), and the class decides
where the device may keep it.

- **The catalogue** (Library class) for every library the profile may
  see: the fields in API-CAT-01 to API-CAT-10, the neighbour table
  (API-SYNC-06, from R1.3) and, in R1, the lyrics as a parsed timed-line model and
  the per-file playback data the player needs (player.md, "What the player
  needs from the server and the core").
- **Artwork** in the device class's fixed sizes, fetched on first display
  by capability URL (API-SYNC-05) and kept by image ID, not by URL. R1 has
  no artwork budget, because partial sync (CLI-023) is R2.
- **The profile's own data** (Activity class) from the user log: the queue
  document, manual playlists, saved rules, Home layout and pins, loves,
  ratings, dismissals, settings (person scope and this device's scope) and
  history. Each kind joins the copy in the release that ships it (see
  API-SYNC-04).
- **Facts about the server** the client needs offline: its name, the
  capabilities it reported, and the protocol version.

Never copied, for anyone, the owner and administrators included: absolute
file paths (folder view uses paths relative to the root; admins read
absolute paths through API-CAT-11; SEC-API-068), capability URLs (always
signed on demand), secrets of any kind, other people's or other profiles'
data (SEC-PRV-022, SEC-CLI-020), items outside the profile's grants or
content policy, and anything the egress gate would not let out. Recent
searches are created and kept only on the device (design-language
section 11, SEC-PRV-004).

### Where the device may keep it

- **Web client, shared browser.** At sign-in the client asks whether the
  browser is personal or shared (SEC-CLI-010). In shared mode the whole
  copy, catalogue and artwork included, lives in memory only, so it is
  gone when the tab closes; "instant Home" there means instant after the
  first sync of the visit. Which answer is preselected is an open owner
  choice (README item 25: "personal or shared browser default").
- **Web client, personal browser.** The catalogue and artwork may be kept
  in IndexedDB, partitioned per account; Activity data may be kept there
  too, because the person marked the browser as theirs (SEC-PRV-019,
  SEC-IAM-017). No session or API token is ever stored (SEC-IAM-017). The
  service worker and Cache Storage never keep capability URLs
  (SEC-API-029).
- **Native apps (R2).** App-private storage, excluded from OS indexing and
  device backups, one partition per server and per profile, with PIN-gated
  profiles encrypted under a key the server releases after the PIN check
  (SEC-PRV-057, SEC-CLI-035, SEC-CLI-044, SEC-CLI-063, SEC-IAM-065).

Three items in the copy may be too large to copy in full, and the
documents do not yet budget them:

1. **Seek indexes.** player.md puts each file's seek index in the synced
   library. At 100,000 tracks that may be large (unverified). **Proposal:**
   sync codec, trim, gain and true peak for every track, and fetch the seek
   index for a track, through the same object check, when it enters the
   queue, so the device holds it before play and offline playback in R2
   gets it with the download.
2. **Lyrics.** Carrying every lyric file is what makes them instant and
   free. Its size at 100,000 tracks is unmeasured. **Proposal:** measure it
   against the DIS-019 budget before deciding; fetching on first open is the
   fallback, at the cost of lyrics offline in R1.
3. **History.** F16 says history by date is read from the profile's log on
   the device. A person who imports fifteen years of Last.fm scrobbles may
   hold hundreds of thousands of events (unverified). **Proposal:** sync
   per-item aggregates (counts, last played, skips) and a recent window of
   history, and page older history from the server, which history by date
   tolerates because it is rarely browsed offline.

### How changes flow from the server to the device

- **One ordered change log, filtered per principal.** The scan diff is the
  feed (LIB-018). Every change to items, artwork, relations and the
  profile's user data gets a sequence number. A device holds a cursor
  (CLI-024) and asks for everything after it in one request
  (API-SYNC-02). The log is shared, but each delta is computed for the
  requesting principal from its grants at that moment, never filtered on
  the device (SEC-API-015). Cursors are opaque or MAC-protected and carry
  nothing the server trusts for authorisation (SEC-API-025).
- **Snapshot first.** A new device takes a snapshot, then deltas. A device
  whose cursor is older than the compaction horizon is told to take a fresh
  snapshot. Until the first snapshot lands, the device must say it is not
  yet available offline (flows G12).
- **Nudges, not polling.** While connected, the server pushes a "changes
  available" nudge over the client event channel (API-SYS-10), built per
  recipient (SEC-API-016); the client then pulls the delta. Without a
  channel the client polls, and things that depend on freshness (the
  "Playing on" note, a removed library) are only as quick as the poll
  (player.md marks this unverified).
- **Grants that shrink.** When a person loses a library or an item, the
  next delta carries removals that name only identifiers, never titles or
  other metadata (SEC-API-015, API-SYNC-03). A withdrawn library is one
  removal naming the library, after which the device deletes everything it
  holds from it, along with any queue entries, playlist entries, history
  rows and neighbour-table entries that point into it. This replaces the
  earlier proposal to resend a snapshot, which would have worked but sent
  more than needed; the server must still prove by test that no metadata of
  a removed item reaches the device (SEC-API-015 test). A missed removal is
  a disclosure, so the grant change itself also takes effect on the next
  request for every other path (SEC-IAM-076, SEC-API-017).
- **Revocation.** A sync request is an ordinary signed-in request, so a
  revoked session, device or account gets the uniform 401 on its next
  sync and its socket closes within 5 seconds (SEC-IAM-043). On the typed
  "session revoked" answer, at sign-out and at an account or profile
  switch, the web client deletes that account's copy from IndexedDB, OPFS,
  Cache Storage, web storage and memory and unregisters its service worker
  (SEC-CLI-009, API-SYNC-11). From R2, native apps delete the account's
  copy and downloads when told the device is revoked (SEC-IAM-053), and
  offline grants stop at their expiry or at the next contact (SEC-IAM-054).
  A device that never reconnects keeps what it already held until its
  storage is cleared; the device list and the deletion screens say so
  plainly.
- **Erasure.** Deleting history appends a tombstone to the profile's feed
  that names only event IDs or ID ranges; every device purges those events
  on its next sync (SEC-PRV-052, API-SYNC-10). A lost device cannot be
  reached, and the deletion screen says so.
- **Secure context.** Over plain HTTP on anything but loopback there is no
  web client at all, only the help page (SEC-NET-001), so the synced copy
  exists only on HTTPS or localhost origins, where browsers allow keeping
  it across reloads, loading with the server down and installing the app
  (CLI-150).

### Private listening in the synced copy

A private session (API-USR-07) is enforced at both ends, because the
device writes most history.

- **On the device.** The core's event sink takes a mode. In private mode it
  drops play events, skip and completion signals and recommendation
  signals before they reach the outbound queue, so nothing from the
  session is stored for later upload, even if the device goes offline and
  reconnects after the session ends (SEC-PRV-024). Derived values the
  device computes (recently played, counts, radio seeds, "because you
  played") ignore the session.
- **On the server.** The session carries the private flag. The server
  derives no play from the session's stream requests, refuses play events
  tagged with a private session, emits no scrobble or webhook (SEC-PRV-035,
  SEC-EXT-045), and omits the title from every live-session view shown to
  anyone else (SEC-PRV-024, SEC-PRV-025).
- **Queue position** is not history: the queue document and its position
  still sync to the person's own devices, so handoff and "Playing on"
  keep working, and nobody else can read them (SEC-PRV-022).
- **Native apps (R2)** also donate nothing from a private session to OS
  history surfaces (SEC-PRV-058).

### How changes flow from the device to the server

There are three kinds of write, and each has its own merge rule. Every
write is authorised like any other request: the profile comes from the
credential, and every item it names must be visible to that profile, or the
whole write is refused (SEC-API-012, SEC-API-013).

1. **Events** (plays, loves, ratings, dismissals and their reversals,
   resume points in R2). Appended to the user log with a client-generated
   event ID, the device ID and a hybrid logical clock (CLI-093), carrying
   only the fields SEC-PRV-002 allows. Replays are idempotent because the
   event ID is the key. Derived values (counts, last played) are
   recomputed from the log. History deletion is not an event of this kind;
   it is an erasure (API-LOG-03).
2. **Operations on versioned documents** (the queue, playlists, the Home
   layout, rule trees). Each operation names the version it was built on.
   The server orders operations and assigns versions; for the queue it
   rejects stale operations and the client rebases (MUS-122). In R2,
   operations made offline are replayed on return under stated conflict
   rules (CLI-094), and each replayed operation is authorised against the
   profile's grants at replay time, not at the time it was made.
3. **Settings.** One record per key, with a person or device scope
   (CLI-030), replaced whole.

**What works offline in R1.** Only play events and position updates are
queued in R1 (CLI-093). Offline edits are R2 (CLI-094). The UI documents
do not say what a love, a rating or a playlist edit does in R1 while the
server is unreachable. **Proposal:** the client disables those actions with
the reason ("Needs the server") rather than queueing them, and the event
format is designed so that R2 can start queueing them with no protocol
change.

### How conflicts resolve

The feature map requires the rules to be written and tested before handoff
is built (MUS-122, CLI-101) but does not state them. This table is a
**Proposal** for each kind of data, built to match what the interface
promises and what the baseline requires.

| Data | Rule | Why |
|---|---|---|
| Plays | Union of all events, de-duplicated by event ID; events from a private session never exist | A play happened; nothing should lose it, and a private play was never recorded |
| History deletion | The erasure removes the play whatever order events arrive in, and is re-applied after a restore | SEC-PRV-049; MUS-184 promises removal reaches every device |
| Love, rating | The event with the latest hybrid clock wins, per person and item | A toggle has one current value |
| Dismiss, hide, snooze | Latest wins; undo is a reversal event | DIS-023 needs undo to survive sync |
| Queue (online) | Server order; stale operations rejected and rebased | MUS-122 |
| Queue (two players) | The device that last pressed Play becomes the active player; the other pauses and shows "Playing on *device*"; only the person's own devices take part | player.md open question 2, flows G11; SEC-HIS-014 |
| Queue (offline edits, R2) | Replayed as operations on the current version; edits to items that no longer exist or are no longer visible are dropped; the active player's position wins | Edits are rare offline; position belongs to whoever is playing |
| Playlist entries | Entries have their own IDs. Concurrent adds are both kept; a remove beats a concurrent move; a reorder applies relative to neighbours, not indexes | Index-based edits corrupt under concurrency |
| Playlist name, rule tree, Home layout | Latest wins for the whole field | Merging two rule trees produces rules nobody wrote |
| Edit to a playlist deleted elsewhere | The deletion stands and the device shows the rare conflict notice (CLI-094) | The person who deleted it meant it |
| Settings | Latest wins per key and scope | CLI-030 |
| Resume points (R2) | Latest event wins, per item and version | VID-118, VID-120 |
| Household curation (merges, splits, locks) | Admin-only and online; server order | Curation is never edited offline |

The map's own rows already settle one more point: offline downloads stop
when the grant expires and a revocation applies on next contact (CLI-095,
CLI-096, SEC-IAM-054), and the documentation must say so.

## Background jobs the server runs

Jobs that read media run in the scan worker, outside the server process
(SEC-MED-018); jobs that reach the internet go through the egress client
and are off until the owner turns them on (SEC-TM-048).

| Job | Trigger | What it produces | Features | Rel. | Security |
|---|---|---|---|---|---|
| Library scan | Manual, after adding a library, at first run | The catalogue, identity, health records and change-log entries, committed in batches; header-only reads in the worker; no helper process per file; bytes read per root | LIB-012, LIB-019, LIB-020, LIB-021, ADM-088 | R1; R1.3 (bytes read per root) | SEC-MED-018, SEC-MED-033, SEC-MED-038, SEC-API-064, SEC-NET-053 |
| Change detection | File watcher on local disks; polling on shares and cloud drives; a scheduled safety-net scan | Rescans of only what changed; moves and renames that keep identity; better copies recorded as upgrades | LIB-013 to LIB-017, LIB-029, LIB-030 | R1 | SEC-MED-034, SEC-MED-041, SEC-TM-069 |
| Path-scoped refresh | An admin; from R2 a tool with a scoped key | A rescan of one path inside a configured root | INT-011 | R1 (admins); R2 (keys) | SEC-API-064, SEC-MED-033 |
| Parser-upgrade re-read | Startup after an upgrade | Re-reads only files whose parser version changed, keeping derived data | LIB-025, ADM-141 | R1.1 | SEC-MED-018 |
| Artwork processing | During scan | Fixed sizes per device class, the tiny placeholder and the palette, from one safe decode in the worker, re-encoded with metadata removed | LIB-142, LIB-143, MUS-110 | R1 | SEC-MED-018, SEC-MED-044, SEC-MED-045, SEC-API-086, SEC-TM-034 |
| Loudness analysis | After scan, at low priority, throttled and checkpointed | Measured loudness for untagged tracks; depends on the decoder decision (open decision 8), otherwise nothing runs and the fallback gain applies | MUS-086, MUS-089, LIB-024, ADM-095 | R1.3 | SEC-MED-018, SEC-MED-026 |
| Neighbour table rebuild | Nightly and after a scan | The table radio and "More like this" read, from metadata and each profile's own listening; no household co-listening in R1 or R2 (Later, only from people who opted in and only for items at least three of them played) | DIS-060 | R1.3 | SEC-PRV-022, SEC-PRV-023 |
| Server-side rule evaluation | When the library or a rule changes | Smart playlist contents for API keys, adapters and download rules | DIS-121, INT-138 | R2 | SEC-EXT-011, SEC-MED-051 |
| Playlist files in folders | During scan | Playlists from `.m3u` files found in music folders, entries resolved only within the same library | LIB-192 | R1.1 | SEC-MED-050, SEC-HIS-018 |
| Import matching | After an upload of history or playlists | Matches with reasons and confidence; misses sent to the import's unmatched queue (ADM-044; the library review queue, LIB-099, is R1.3) | ADM-042 to ADM-044, MUS-140 | R1.1 | SEC-API-088, SEC-HIS-018 |
| Change-log compaction | Scheduled | A bounded log; cursors older than the horizon are told to resnapshot | LIB-018, INT-006 | R1 | SEC-API-015 |
| Root health | Continuous and on access | Offline roots marked, items greyed, alerts raised; trash purges held | LIB-032, ADM-108 | R1 | SEC-TM-069 |
| Trash purge | After the grace period | Entries for missing files removed, never while a root is offline | LIB-033 | R1 | SEC-TM-069 |
| Daily backup and verification | Daily by default | A consistent snapshot of the user log, identity store, configuration and audit log with a manifest and the latest signed audit checkpoint, encrypted in age to the backup key and the owner's recovery key, signed, verified after writing; alert on failure | ADM-065, ADM-066, ADM-072 | R1 | SEC-OPS-024, SEC-OPS-041 to SEC-OPS-043, SEC-PRV-039, SEC-IAM-105 |
| Pre-upgrade snapshot and migration check | Startup on a new version | A snapshot checked for integrity, migrations run on a copy, serving only after they pass; no migration makes the install less strict | ADM-056, ADM-057, ADM-058 | R1 | SEC-OPS-048, SEC-OPS-049, SEC-OPS-051 |
| Cache rebuild | Older binary on newer data, or on request | The cache rebuilt from the files plus the log, reusing derived data; identities, grants, keys and the audit log untouched | ADM-059, ADM-077 | R1 | SEC-IAM-004, SEC-API-030 |
| User-log recovery | Startup | Recovery from a torn write using checksums | ADM-078 | R1 | SEC-OPS-051 |
| Update and advisory check | Daily, only when the owner answered "Tell me about security fixes" at first run | A plain GET of the static signed feed, verified against the root compiled into the binary; advisory and rollback-safety banners; nothing on the server is ever changed by it | ADM-053, ADM-054, ADM-060 | R1 | SEC-OPS-019, SEC-OPS-047, SEC-OPS-068, SEC-SUP-050, SEC-SUP-051 |
| Free-space guard | Continuous | Alerts, and refusal of writes that would fill the disk, keeping the reserve that lets recovery actions write their audit records | ADM-083 | R1 | SEC-OPS-020 |
| Alert dispatch | On events | In-app alerts in R1, non-critical ones in a daily summary; outbound destinations the owner opted into from R2 | ADM-116 | R1 (in-app); R2 (outbound) | SEC-OPS-032 to SEC-OPS-035 |
| Log rotation | Scheduled | Structured logs, readable only by the service account, rotated and deleted on the retention schedule | ADM-119 | R1 | SEC-PRV-005, SEC-PRV-045 |
| Expiry sweeps | Scheduled, and checked on every use | Lapsed claim codes (24 hours), WebAuthn challenges (5 minutes), WebSocket tickets (30 seconds), pairing codes (10 minutes), browser sessions (7 days unused, 30 in total; shared mode 30 minutes idle), admin sessions (15 minutes idle, 1 hour in total), invitations (7 days; purged 30 days after expiry), recovery and owner-recovery links, export downloads (1 hour), diagnostic bundles (24 hours), ended recovery holds; token-expiry banners | ACC-001, ACC-080, ACC-064, ACC-079, INT-019 | R1 | SEC-IAM-007, SEC-IAM-019, SEC-IAM-041, SEC-IAM-056, SEC-IAM-078, SEC-IAM-092, SEC-IAM-106, SEC-API-042, SEC-CLI-010, SEC-PRV-048 |
| Open-response tracking | Continuous | Range responses and sockets per session, aborted within 5 seconds of revocation | ACC-122 | R1 | SEC-IAM-043, SEC-API-017 |
| Crash records | On a crash | A local record for the diagnostics page, with no core dump of the server process; core dumps are off from R1 (SEC-STD-023) | ADM-130 | R1 (core dumps off); R1.2 (crash records) | SEC-STD-023, SEC-PRV-046 |
| Signing-key rotation | Automatic every 24 hours for stream-URL and request-forgery keys; on the owner's "rotate everything"; after every restore | New key IDs; the previous key kept only for the longest lifetime of what it signed; after a full rotation or restore, every session and signed URL invalidated and the owner alerted | None; required by the baseline | R1 | SEC-OPS-015, SEC-OPS-018, SEC-OPS-044, SEC-API-030 |
| Audit checkpoints | Every 1,000 records or every hour, whichever comes first | A signed checkpoint over the hash chain, which backups carry and admins' clients anchor | None; required by the baseline | R1 | SEC-OPS-023, SEC-OPS-024, SEC-OPS-075, SEC-IAM-094 |
| Retention purge | At least daily, idempotent | Security events removed after 365 days; addresses in them coarsened after 30 days and removed at 90; session addresses removed when the session ends; diagnostic logs after 14 days or 100 MB; backups after 14 days; each prune recorded as a signed checkpoint | None; required by the baseline | R1 | SEC-PRV-003, SEC-PRV-005, SEC-PRV-041, SEC-OPS-026 |
| Erasure | On a history deletion; after an account's grace period; after a restore | Rows deleted under secure delete, the WAL truncated, history-log segments rewritten, derived data recomputed and tombstones appended, all within 24 hours | MUS-184 | R1 | SEC-PRV-049, SEC-PRV-050, SEC-PRV-052 |
| Account deletion | 7 days after a deletion request | The account erased through the erasure job, unless restored during the grace period | None; required by the baseline | R1 | SEC-IAM-103, SEC-PRV-051 |
| Startup security checks | Every start | Refusal to run as root or with capabilities; secrets directory permissions repaired or refused; configuration changes made outside the server audited and alerted when less strict; each sandbox profile self-tested and its isolation tier shown; listeners on public addresses reported | ADM-032 | R1 | SEC-OPS-012, SEC-OPS-031, SEC-OPS-053, SEC-MED-024, SEC-NET-028 |
| Exposure detection | Continuous | A security event and an admin alert within one minute when a non-local request reaches a home-posture listener or an untrusted peer sends forwarding headers | None; required by the baseline | R1 | SEC-NET-017, SEC-NET-027, SEC-OPS-037 |
| Certificates | Two-thirds through each certificate's life, or as the CA's renewal information says; from R2, continuously for CT | Renewed certificates for the owner's domain; owner alerts 30 and 7 days before expiry; from R2, Certificate Transparency watched for the server's own label when it uses the project name service | ADM-022, ADM-023 | R1 (renewal and expiry alerts); R2 (CT monitoring for the name-service label) | SEC-NET-004, SEC-NET-069, SEC-NET-072 |
| Debug-level timer | When debug logging is switched on | Debug logging switched off again within 24 hours, both changes audited | ADM-119 | R1 | SEC-OPS-029 |
| Worker quarantine | When a file crashes or times out the worker twice | The file skipped until its size or modification time changes or an admin retries it, listed in the health report | MUS-044 | R1 | SEC-MED-019 |
| Derivative cache bound | Continuous | Generated image sizes evicted least-recently-used first within a byte budget | LIB-142 | R1 | SEC-MED-048 |
| Metadata providers and lookups | On scan and refresh only, and only for providers the owner turned on in the required setup step | MusicBrainz and cover-art lookups built in from R1.1, sending only normalised lookup evidence; other providers in the sandboxed plugin host from R2 | LIB-107, LIB-111, LIB-112 | R1.1 (built-in providers); R2 (plugins) | SEC-PRV-013 to SEC-PRV-017, SEC-API-079 to SEC-API-081, SEC-EXT-001 to SEC-EXT-005 |
| Opus encoding | On demand, cached | Opus streams and download copies in the sandbox | MUS-106, MUS-213 | R2 | SEC-OPS-062, SEC-MED-024 |
| Remux and transcode workers | On demand | Segments for browsers, TVs and casting; remuxing in a worker, transcodes in the jail only | VID-003, VID-005 | R2 | SEC-MED-081, SEC-TM-044 |
| Transcode sandbox self-test | Startup and on request | Whether transcoding can run, shown to the owner; transcoding off when the jail is missing | VID-009, ADM-132 | R2 | SEC-MED-024, SEC-OPS-062 |
| Intro detection | After scan, per season | Skip markers | VID-112 | R2 | SEC-MED-018 |
| Smaller versions | At idle, under the owner's rules | Pre-made versions for remote and mobile play | VID-026 | R2 | SEC-OPS-062 |
| Scrobble and webhook delivery | On events, with retries | Deliveries with a log; only for people who turned scrobbling on themselves, only plays after the link time, nothing from private sessions | INT-030 to INT-047, MUS-192 to MUS-195 | R2 | SEC-PRV-033 to SEC-PRV-036, SEC-EXT-045 to SEC-EXT-049 |
| Follow alerts | On new arrivals | Inbox entries for followers, only for items they can see | INT-050 | R2 | SEC-PRV-030, SEC-MED-051 |
| Offline grant renewal and revocation list | On contact | Renewed grants; revoked grants refused | CLI-095, CLI-096 | R2 | SEC-IAM-054, SEC-CLI-036 |
| Household-device dormancy | Daily | Household devices unused for 30 days made dormant until an adult approves them; deleted after 365 days dormant | ACC-019 | R2 | SEC-IAM-109 |
| API key expiry | Daily | Keys expired, disabled after 180 days unused, owners told 14 days before | INT-019 | R2 | SEC-EXT-013 |
| Off-site backups | Scheduled | Encrypted copies at the owner's destination, through the egress client | ADM-068, ADM-073 | R2 | SEC-PRV-039, SEC-EXT-001 |
| Guide and source refresh | Scheduled, never because someone opened the guide | Guide diffs applied; sources re-probed | LIV-035, LIV-003 | R3 | SEC-PRV-060, SEC-NET-067, SEC-API-082 |
| Recording scheduler and recorder | Scheduled | Recordings, conflicts, recording alerts, written only to the recordings root | LIV-102 to LIV-141 | R3 | SEC-OPS-063 |
| Time-shift buffer | While a channel plays | A rolling buffer for pause and start over | LIV-095, LIV-097 | R3 | SEC-OPS-063 |

## UI requirements the architecture makes hard or impossible

Each item names the interface requirement, the architecture record or rule
that gets in its way, and what the documents propose.

1. **Durable user data beyond watch history.** The UI keeps queues,
   playlists, loves, ratings, layouts, hides, settings and corrections
   across rebuilds and restores. ADR 1 (decision 5) makes SQLite a
   rebuildable cache and only watch history irreplaceable. Without ADR 3
   (feature map open decision 1) almost every R1 write in this list has
   nowhere durable to live. The baseline already requires accounts,
   credentials, devices, grants, invitations and the security log to live
   in durable storage that rebuilds never touch (SEC-IAM-004), so ADR 3
   must cover identity as well as user data. Severity: blocking for R1.
2. **Gapless playback in the browser.** The player promises gapless
   playback in a browser tab (MUS-067). Where Media Source Extensions do
   not accept raw FLAC, Ogg Opus or MP3, that needs the light audio
   packager (MUS-230), which contradicts ADR 2 (decision 2, "no remuxer").
   Open decision 9 proposes amending ADR 2. The baseline adds that the
   packager reads untrusted files at serve time, so it must run in a worker
   process like the R2 remuxer (SEC-MED-018, SEC-MED-024; security owner
   decision 9), not in the server process. Per-browser support is
   unverified, including Safari.
3. **"Measured" loudness.** The track info sheet shows whether gain was
   tagged, measured or estimated. Measuring needs a full decode with a
   third-party crate, which ADR 1 (decision 3) and the pure-Rust parsing
   rule do not yet allow (open decision 8), and which the baseline admits
   only after a recorded review and only in the worker (SEC-MED-026,
   SEC-MED-018). The adopted scope puts measured loudness in R1.3, and only
   if that review passes (D-10); until then the player shows only "tagged"
   or "estimated", in the R1 track details view (MUS-236, D-83) and, from
   R1.1, in the track info sheet.
4. **A track the browser cannot play.** The UI dims it with the reason
   (MUS-229). ADR 2 (decision 2) means R1 has no transcoder, and the
   baseline confirms no native transcoding in R1 (security owner decision
   11), so R1 can only explain, never fix (flows G13). Which core formats
   each browser lacks is unverified.
5. **Passkeys in R1, offline loading and installation from R1.1.** All
   need a secure context (CLI-150). The baseline removes passwords and TOTP
   (SEC-IAM-025, security owner decision 1) and gives plain-HTTP peers
   other than loopback nothing but a help page (SEC-NET-001). A plain-HTTP
   LAN install therefore has no web client at all off the server itself.
   In R1 the household needs HTTPS through their own domain with automatic
   certificates (ACC-099), a tailnet, a reverse proxy, or the same machine
   (SEC-NET-013; owner answer to D-07, 2026-10-02). This replaces the
   earlier reading, in which a plain-HTTP install fell back to a password
   sent unencrypted on the home network (flows G2). The project-run
   per-server name service (ADM-023), its naming client and its
   certificate-transparency monitoring are R2: ADR 1 (decision 7) rules out
   a central account, so a project-issued HTTPS name needs its own ADR
   first. A household with neither a domain nor a tailnet can therefore
   use the web client in R1 only on the server's own machine, or through an
   SSH tunnel to localhost.
6. **Passkeys after a move.** A passkey belongs to its domain. Restoring a
   backup under a new address breaks every member's passkeys (flows G3).
   The proposed remedy is a warning at restore and one-time sign-in links,
   which must follow the recovery-link rules: redeemed in person or on an
   approved device, starting a recovery hold (SEC-IAM-091, SEC-IAM-106).
7. **Short-lived signed URLs.** ADR 1 (decision 6) and ACC-122 make media
   and image URLs short-lived and session-bound (SEC-API-026,
   SEC-API-027). That collides with several UI promises: a long pause or a
   long mix outliving a URL (needs silent refresh, API-STR-02); artwork
   handed to the operating system's media controls landing a signature in
   an OS cache (player.md proposes local blob artwork, which the CSP allows
   through `img-src blob:`; browser support for the media-session use is
   unverified); cast receivers that cannot refresh easily and cannot join
   iroh (R2, SEC-NET-064); and browser caching of artwork. The earlier
   proposal for the last, serving artwork to the first-party web client at
   cookie-authorised paths, is withdrawn: the baseline makes capability
   URLs the only media and image credential, the web client included
   (SEC-API-029). Instead, artwork URLs are aligned to a fixed time bucket
   (SEC-API-027), so every request in the same bucket reuses the same URL
   and the HTTP cache works within it, and the client keeps decoded images
   by image ID rather than by URL.
8. **Removing a play from an append-only log.** The history page promises
   "Remove this play" (MUS-184). ADR 1 (decision 5) makes the log
   append-only. The baseline requires real erasure in R1: one entry, a
   range or all history removed from the database, the history log,
   derived data and caches within 24 hours, re-applied after a restore,
   and tombstones to devices (SEC-PRV-049, SEC-PRV-050, SEC-PRV-052). That
   needs a new ADR extending ADR 1 so that erasure is the one sanctioned
   rewrite of an otherwise append-only log, as the privacy design guidance
   proposes. Backups taken before the deletion still hold the play until
   they expire under backup retention (SEC-PRV-041), and the UI must say so.
9. **Notices when the app is closed.** The notice centre carries new-device
   alerts and follow alerts (ACC-071, INT-050), and the R3 system
   surfaces list recording alerts (LIV-111, LIV-155). Remote push needs
   Apple's and Google's services, which ADR 1 (decision 7) and the map set
   aside (CLI-077 and INT-052 are Later); if they ever ship, payloads carry
   only an opaque ID (SEC-OPS-036, SEC-PRV-056). These notices therefore
   arrive only while the app holds a connection or next syncs. Scheduled
   events such as reminders can be local notifications from synced data
   (LIV-075); unexpected failures such as a recording that broke cannot.
10. **Remote access from a browser.** iroh (ADR 1, decision 7) reaches only
    native apps; a browser reaches a server without a domain only through
    the project's TLS-passthrough edge (ACC-102, R2), which depends on the
    per-server name service (ADM-023, R2). In R1, remote use goes through
    the owner's reverse proxy or a tailnet (owner answer, 2026-10-02;
    security owner decision 3), built-in remote access arrives in R2
    (API-SET-12), and admin operations are refused on internet-posture
    paths unless the owner turns remote administration on (SEC-NET-045). An invite carries the configured
    public URL, never one taken from the request (SEC-API-069), but that
    address may still be one the friend cannot reach (flows G7).
11. **One React Native codebase under a strict content security policy.**
    ADR 1 (decision 8) puts the web client on React Native Web; the
    security baseline allows only `style-src 'self'` (SEC-API-044). Per-album
    tints must go through the CSSOM, and whether React Native Web's style
    injection works under that policy is unverified (design-language open
    question 8). The web security document's open decision 7 allows
    `style-src 'unsafe-inline'` as a recorded fallback, never for scripts.
    The Linux desktop shell has no first-party React Native target (open
    decision 21, unverified).
12. **Live updates in R1.** Several R1 screens need the server to speak
    first: cutting a revoked device (ACC-069), an administrator ending a
    person's sessions (SEC-IAM-044), the "Playing on *device*" note
    (player.md), the first scan filling the library in (LIB-021) and
    new-device notices (SEC-IAM-098, now R1). R1.2 adds stopping a session
    with a message (ADM-102) and the admin's now-playing list (ADM-099,
    whose server need names an event stream). The map places the public event stream
    (INT-048) and the control channel (CLI-101) in R2. This is a gap in the
    release cut rather than an ADR conflict. **Proposal:** a private,
    first-party client event channel in R1 (API-SYS-10), meeting the
    WebSocket rules of the baseline (SEC-API-041 to SEC-API-043,
    SEC-API-016), which R2's control channel and INT-048 then extend.
13. **Device-side computation within budget.** Search, Home rows, smart
    playlist previews and radio all run in the core on the device (ADR 1,
    decision 2). The proposed budgets (Home under 200 ms, search under
    50 ms at 100,000 tracks on the reference low-end device; open decision
    16) are unmeasured, and the reference devices are not yet named. The
    budget tests are enforced in the R1 gate all the same, and only the
    published numbers wait for R1.1 (register D-87). The prebuilt index
    (API-SYNC-07) is the only stated fallback. In a shared
    browser the copy lives in memory only (SEC-CLI-010), so every visit
    starts with a sync.
14. **Third-party apps and credentials in URLs.** The OpenSubsonic adapter
    (ADR 2, decision 6) serves apps whose streaming and sign-in put
    credentials in query strings, which SEC-API-004 and INT-023 forbid on
    native routes. The baseline makes the adapters R2, off by default, with
    per-app keys that are never the account's credential, redacted from
    logs, and legacy Subsonic sign-in allowed only per key and off by
    default (SEC-API-094, SEC-EXT-063, SEC-EXT-067, SEC-EXT-069; security
    owner decision 8). Which Subsonic apps support API keys is unverified,
    so F19 may work for fewer apps than hoped.
15. **High-rate writes and one writer.** Position updates, play events and
    (R2) remote-control reports all write user data, and ADM-080 allows one
    writer. The player only needs positions at state changes and
    occasionally while playing, so the client must batch, and the server
    must never write a position per second per session.

Two UI requirements have no feature row behind them yet and so no server
capability here: the TV rail's Now Playing entry and lyrics on TV
(surfaces.md open question 4). Both appear to need nothing beyond the queue
document and the synced lyrics.

## Conflicts with the security baseline

Each row is a place where this document said something the baseline does
not allow, and what it says now. "Owner" marks a change that applies a
recommendation from the baseline's
[open decisions for the owner](../security/README.md#open-decisions-for-the-owner),
which the owner must confirm.

| Where | Before | After | Requirements | Owner |
|---|---|---|---|---|
| Rules for every capability | Unauthenticated: setup, sign-in, invite landing, startup, emergency and health routes | The ten-item list in [The only unauthenticated endpoints](#the-only-unauthenticated-endpoints); startup is a static page, emergency needs the admin session | SEC-API-002, SEC-TM-004, SEC-OPS-050 | No |
| API-SYS-01 | Liveness and readiness | Liveness only | SEC-NET-046 | No |
| API-SYS-02 | Reports which web features work on a non-secure address | No client over plain HTTP except to loopback; a static help page instead | SEC-NET-001, SEC-NET-024 | Yes (decision 2) |
| API-SYS-06 | Server name and sign-in message before sign-in | Neither before sign-in | SEC-API-005, SEC-NET-047 | Yes (decision 25, web open decision 5) |
| API-SYS-07 | Server-rendered page with steps, estimates, snapshot location and refusals | Static page with no detail; detail on the host and to admins; refusals stop the server | SEC-OPS-050, SEC-OPS-053 | Yes (decision 14) |
| API-SYS-08 | Unauthenticated page with log lines, a backup and a restart | Admin session; backup download owner with fresh verification | SEC-IAM-041, SEC-OPS-045 | No |
| API-AUTH-02 | Locale, owner, server name and other steps in sequence before setup closes | The claim first; later steps are owner settings behind the admin session | SEC-IAM-006, SEC-IAM-009, SEC-OPS-006 | No |
| API-AUTH-03 | A password and two-factor where the page is not a secure context | Passkey only; claim refused outside loopback or a secure context | SEC-IAM-008, SEC-IAM-025, SEC-STD-006 | Yes (decision 1) |
| API-AUTH-05 | Password and TOTP sign-in, R1 | Not built (No) | SEC-IAM-025, SEC-STD-006 | Yes (decision 1) |
| API-AUTH-07 | Limiter for passwords, codes, setup, invite codes, PINs, pairing codes | Delay schedule only for guessable secrets; passkey failures throttled, never locked | SEC-API-056, SEC-IAM-101 | No |
| API-AUTH-10 | A re-prompt before a sensitive change | Separate short admin session plus fresh-uv routes | SEC-IAM-041, SEC-IAM-107 | No |
| API-AUTH-12 | Admin issues a sign-in link with QR to any locked-out user | Members and guests only (owner for admins, nobody for the owner); in person or on an approved device; 72-hour hold | SEC-IAM-091, SEC-IAM-106 | Yes (decision 6) |
| API-AUTH-13 | Pairing entirely R2 | Browser pairing R1; native device keys R2 | SEC-IAM-056 to SEC-IAM-060, SEC-IAM-108 | No |
| API-AUTH-15 (new) | No recovery codes | Recovery codes R1 | SEC-IAM-089, SEC-IAM-106 | Yes (decision 6) |
| API-USR-03 | Manage the password and two-factor | Passkeys and OIDC links only | SEC-IAM-025 | Yes (decision 1) |
| API-USR-05 | Export behind sign-in | Authentication within 5 minutes; single-use, session-bound download for at most 1 hour | SEC-PRV-048 | No |
| API-USR-09 | R2 | R1 | SEC-IAM-104, SEC-PRV-027 | No |
| API-USR-10 (new) | No account deletion | Self-service deletion with a 7-day grace, R1 | SEC-IAM-103, SEC-PRV-051 | No |
| API-DEV-03 | Optional server log excerpt for anyone's report | Only when the sender is an admin, masked | SEC-PRV-046, SEC-OPS-030 | No |
| API-DEV-05 | New-device notice R2 | R1 | SEC-IAM-098, SEC-OPS-032 | No |
| API-SYNC-03 and the sync model | Resend a snapshot of the affected library on a grant change | Identifier-only removals, library-level purge on the device | SEC-API-015 | No |
| API-SYNC-06 | Household co-listening in every profile's neighbour table | Own listening only in R1 and R2; others Later, only by opt-in with at least three people | SEC-PRV-022, SEC-PRV-023 | Yes (security README decision 5) |
| API-SYNC-10, API-SYNC-11 (new) | No tombstones; no purge rule | Erasure tombstones and purge on revocation, R1 | SEC-PRV-052, SEC-CLI-009 | No |
| Sync model: storage | Copy kept across reloads wherever the browser allows | Shared browsers keep everything in memory; Activity persisted only on personal browsers | SEC-CLI-010, SEC-PRV-019 | Yes (decision 25, personal or shared default) |
| Sync model: never copied | Absolute paths withheld only from non-admins | Never in the synced copy for anyone | SEC-API-068 | No |
| API-LIB-01, API-LIB-04 | Who sees a new library unsettled (flows G4) | Owner and administrators only until granted | SEC-IAM-070, SEC-TM-026 | No |
| API-LIB-01, API-LIB-02, API-LIB-05 | Admin action | Fresh-uv for adding or removing roots and for browsing the filesystem | SEC-IAM-041 | No |
| API-STR-04 | Packaging in the server | Packaging in a worker process | SEC-MED-018, SEC-MED-024 | Yes (decision 9) |
| API-STR-05 and Flags, item 7 | Artwork at content-addressed paths authorised by the session cookie (proposal) | Capability URLs only, aligned to a time bucket | SEC-API-029, SEC-API-027 | Yes (decision 25, web session model and media URL binding) |
| API-STR-09 | Attached fonts delivered | Off by default; opt-in per library with re-serialisation | SEC-MED-054 | Yes (decision 12) |
| API-SES-01 | Admins see who is playing what | No title unless the person opted in; reads rate-limited and logged to the subject | SEC-PRV-025, SEC-TM-054 | Yes (decision 5) |
| API-SES-08 | Stream limits R2 | Concurrent-stream limits R1; policy alternatives R2 | SEC-IAM-102, SEC-TM-068 | No |
| API-LOG-03 and Flags, item 8 | A removal event that hides the play | Erasure within 24 hours with tombstones; needs an ADR extending ADR 1 decision 5 | SEC-PRV-049, SEC-PRV-050, SEC-PRV-052 | Yes (ADR change) |
| API-PL-06, API-PL-07, API-TOK-01, API-TOK-02, API-SCAN-02 (keys) | Scoped tokens and tool APIs in R1 | R2, never with administrator scopes | SEC-IAM-083, SEC-EXT-008 to SEC-EXT-013 | Yes (decision 8) |
| API-HOME-07 | Delivery to the person's ntfy topic | A webhook under the webhook rules and the egress inventory | SEC-EXT-045, SEC-TM-075 | No |
| API-SHR-01 to API-SHR-03 (new) | No share links in this list; feature map has them R2 | Music share links with per-link limits, R1 in the baseline and R1.2 in the adopted scope (D-10); video R2, off by default | SEC-API-097, SEC-PRV-031 | Yes (decision 7) |
| API-ADM-01 | Admins make administrators; ownership handed over by adding an admin and removing yourself | Owner-only administrator changes with fresh-uv; explicit two-party transfer | SEC-IAM-003, SEC-IAM-075, SEC-IAM-041 | No |
| API-ADM-02, API-ADM-03 | Invite with libraries, uses and expiry; landing reveals nothing | Preset capped at the inviter's rights; member or multi-library invites pending a code check; privacy notice before redeeming | SEC-IAM-078, SEC-IAM-079, SEC-PRV-053 | No |
| API-SET-01, API-SET-02 | Admin settings | Owner-only, fresh-uv | SEC-IAM-041, SEC-IAM-075 | No |
| API-SET-04 | Admin backups, download and restore | Download, upload, restore and export owner-only with fresh-uv; keys rotated and sessions ended after restore | SEC-OPS-044, SEC-OPS-045 | No |
| API-SET-06 | Opt-in update check | Required first-run question, no preselection | SEC-OPS-047 | Yes (decision 4) |
| API-SET-07 | Alert destinations the owner chose, R1 | In-app only in R1; outbound destinations R2, opt-in | SEC-OPS-032, SEC-OPS-035 | Yes (decision 25, outbound alert channels) |
| API-SET-11 | Plex, Jellyfin, Emby, Navidrome imports R2 | Later, through a jailed read-only worker; others' history only by their acceptance | SEC-TM-074, SEC-STD-031, SEC-PRV-026 | No |
| API-SET-13 (new) | No secret rotation | Owner rotates every secret in one action, R1 | SEC-OPS-018 | No |
| Background jobs: metadata providers | R2, in the plugin host | Built-in MusicBrainz and cover art, off until the owner turns them on; R1 in the baseline and R1.1 in the adopted scope (D-10) | SEC-PRV-013 to SEC-PRV-015 | Yes (decision 22) |
| Background jobs: sandbox self-test | R2 | R1 for the scan worker profile; R2 for the transcode jail | SEC-MED-024 | No |
| Background jobs: security | None listed | Key rotation, audit checkpoints, retention purge, erasure, account deletion, startup checks, exposure detection, certificates, debug timer, quarantine, cache bound | SEC-OPS-015, SEC-OPS-023, SEC-PRV-005, SEC-PRV-049, SEC-IAM-103, SEC-OPS-031, SEC-NET-027, SEC-NET-004, SEC-OPS-029, SEC-MED-019, SEC-MED-048 | No |
