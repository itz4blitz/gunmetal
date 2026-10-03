# What the clients need from the server

Written on 2026-10-02. Status: draft for the project owner's review.

This is a list of capabilities, not an API specification. It reads the
four interface documents in [docs/ui](../ui/README.md) and the
[feature map](../features/README.md) and asks, for every screen and every
step of every flow, what the server has to provide. It does not name
routes, payload shapes or wire formats; the protocol types belong in the
shared Rust core (ADR 1, decision 2) and will be written test first. The
purpose is to give the backend work a complete checklist, grouped so that
separate people or agents can take one group each.

The feature map is the source of truth. Where this document goes beyond it,
the text says **Proposal**. Claims the inputs could not confirm are marked
"(unverified)".

## How to read this document

### Releases

The release values are the feature map's: **R1** (music, server and web
client, no remuxer or transcoder), **R2** (video, the remuxer, sandboxed
transcoding, native TV, phone and desktop clients), **R3** (M3U and live
TV), **Later** (wanted, not scheduled) and **No** (deliberately not doing).
A capability is placed in the first release whose screens need it.

### Columns

Every capability table has the same columns.

| Column | Meaning |
|---|---|
| **ID** | A label for this plan only, such as API-QUE-03. It is not a feature ID and may change when the real protocol is written. |
| **Capability** | A short name. |
| **What it does** | The behaviour, in a sentence or two. |
| **Surfaces** | The screens that need it, by SUR-ID from [surfaces.md](../ui/surfaces.md), or a flow from [flows.md](../ui/flows.md). |
| **Features** | The owning feature rows. |
| **Rel.** | R1, R2 or R3. |
| **Offline** | How it behaves with the server gone. **Local**: answered from the device's own copy. **Queue**: the device records the write and sends it later (in R1 this is true only for play events, CLI-093; edits join in R2, CLI-094). **Server**: needs the server at that moment. **Host**: runs on the server for administrators; there is no offline case. |
| **Live** | Whether connected clients must learn of a change without asking. **Push**: the server tells them over a live channel. **Feed**: the change arrives through the sync change feed, nudged by a push when connected. **No**: the client asks when it needs it. |

### Rules that apply to every capability

These come from ADR 1 and the R1 rows that are rules rather than features.
They are stated once here and not repeated in every row.

- Every route needs sign-in (ACC-120), except the setup, sign-in, invite
  landing, startup, emergency and health routes, which reveal nothing about
  the library.
- Every object is authorised against the caller on every request
  (ACC-121, ADR 1 decision 6), and every path obeys the caller's library
  grants and restrictions (ACC-030), including sync payloads and signed
  URLs.
- Public IDs are random and survive cache rebuilds and file replacement
  (INT-008), because they are bound to content identity (LIB-028) in the
  identity store.
- Credentials travel only in headers or in cookies that page scripts
  cannot read (INT-023, ACC-123, ACC-124). Media and images use
  short-lived, session-bound signed URLs (ACC-122). The adapters in R2 are
  the documented exceptions.
- Lists use one paging, filtering and caching convention (INT-007); the
  protocol is versioned in the core and the server keeps the previous
  version for older clients (CLI-032).
- One writer owns the database (ADM-080), so high-rate client reports
  (positions, heartbeats) must be batched rather than written one by one.
- Nothing leaves the house by default (ACC-113, LIB-108): any capability
  that reaches the internet goes through the egress gate and appears on the
  network activity page (ADM-129).

### What "local-first" means for this list

Most reading in Gunmetal is not an API call. Home, every library list,
artist and album pages, search, history, smart playlist previews and radio
picks are computed on the device from the synced library by the core,
running as WebAssembly in browsers and through UniFFI in native apps
(CLI-022, DIS-002, DIS-084, MUS-208). For those features the server's job
is to put the right fields into the synced copy and keep them current, so
the "Catalogue" section below lists fields rather than endpoints. The
endpoints are concentrated in sync, streaming, the queue, writes to the
user log, and administration.

## Capabilities by resource

### Server, connection and protocol

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-SYS-01 | Health check | Answers liveness and readiness for containers and uptime monitors, and lets clients tell "server down" from "network down". Reveals nothing about the library. | SUR-003, SUR-111 | CLI-025, INT-013, ADM-128 | R1 | Server | No |
| API-SYS-02 | Secure-context report | Tells the client whether the request arrived over HTTPS or localhost, so it can say which web features work on this address. | SUR-070, SUR-077, SUR-082 | CLI-150, ADM-021 | R1 | Server | No |
| API-SYS-03 | Protocol negotiation | Agrees a protocol version with the client and tells an old client plainly which update it needs, while what still works keeps working. | SUR-003 | CLI-032 | R1 | Server | No |
| API-SYS-04 | Capability discovery | Reports the API version, enabled modules, adapters and extensions, and the scopes the caller holds; unauthenticated callers learn nothing. | SUR-112 | INT-005 | R1 | Server | No |
| API-SYS-05 | Who am I | Returns the caller's account, profile, device and token scope, for clients and tools. | SUR-006 | INT-026 | R1 | Local | No |
| API-SYS-06 | Public sign-in facts | The server's name and sign-in message, and nothing else, for the sign-in page. | SUR-070 | ADM-140, ACC-007 | R1 | Server | No |
| API-SYS-07 | Startup status page | Rendered by the server itself while it starts, migrates, rebuilds or restores: each step, estimates, the snapshot location, and refusals such as running as root or a data directory on a network filesystem. Opens before the database. | SUR-080 | ADM-032, ADM-006, ADM-007, ADM-056 to ADM-059, ADM-078, ADM-079 | R1 | Server | Push (it refreshes itself) |
| API-SYS-08 | Emergency page | A minimal server-rendered page, without the client bundle, for status, recent log lines, a backup and a restart. | SUR-081 | ADM-113 | R1 | Server | No |
| API-SYS-09 | API reference | The reference generated from code, served by each server so it matches that version. | SUR-112 | INT-001 | R1 | Server | No |
| API-SYS-10 | Client event channel | One authenticated channel per signed-in session that the server uses to push the events marked "Push" in this document: sync nudges, scan progress, a stopped session, a revoked device, the active player changing. See [Flags](#ui-requirements-the-architecture-makes-hard-or-impossible), item 12. **Proposal** for R1. | SUR-002, SUR-003, SUR-083, SUR-084, SUR-100 | ADM-099, ADM-102, LIB-021, CLI-103 | R1 | Server | Push |

### Setup, sign-in and sessions

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-AUTH-01 | Unclaimed-server guard | While no owner exists, refuses every route except the welcome and status pages, and issues a single-use, expiring setup code to the console and to a file only the service user can read. | SUR-082, SUR-110, F01 | ACC-001, ADM-018 | R1 | Server | No |
| API-AUTH-02 | Setup state | Holds the welcome flow's steps (locale, owner, server name, privacy, import, libraries) and closes setup for good once an admin exists. A half-finished setup and an expired code need a rule (flows G1). | SUR-082 | ADM-020, ADM-027, ADM-140, ADM-028 | R1 | Server | No |
| API-AUTH-03 | Owner creation | Creates the owner in the identity store with a passkey, or a password and two-factor where the page is not a secure context; issues recovery codes; optionally links an OIDC provider. | SUR-082 | ACC-002, ADM-019, ACC-050, ACC-052, ACC-053, ACC-057 | R1 | Server | No |
| API-AUTH-04 | Passkey sign-in | Runs the WebAuthn ceremony where the address allows it. | SUR-070 | ACC-050 | R1 | Server | No |
| API-AUTH-05 | Password and two-factor sign-in | Checks a password, then a TOTP code or a recovery code, with uniform errors and timing. | SUR-070 | ACC-052, ACC-053, ACC-007 | R1 | Server | No |
| API-AUTH-06 | Single sign-on | Completes OIDC with the household's own provider, with state, nonce and redirect allow-list checks; no other outbound call. | SUR-070, SUR-092 | ACC-057, ACC-003 | R1 | Server | No |
| API-AUTH-07 | Guessing limiter | One shared limiter for passwords, codes, setup codes, invite codes and (R2) PINs and pairing codes, with fail2ban-friendly log lines. | SUR-070, SUR-092, SUR-101 | ACC-063, ADM-122 | R1 | Server | No |
| API-AUTH-08 | Browser sessions | Issues cookie sessions that scripts cannot read, with lifetimes for a remembered or a shared computer, and records the device. | SUR-070, SUR-092 | ACC-124, ACC-079, ACC-068 | R1 | Server | No |
| API-AUTH-09 | Session epoch and revocation | Every session and signed URL carries an epoch; a password change, "sign out everywhere", a disabled account or a revoked device bumps it, failing new requests at once and cutting responses already in flight. | SUR-078, SUR-090, player "Signed out" state | ACC-065, ACC-069, ACC-070, ACC-008, ACC-122 | R1 | Server | Push (the signed-out device is told) |
| API-AUTH-10 | Step-up confirmation | Asks for the passkey or password again before a sensitive change. | SUR-008 | ACC-056 | R1 | Server | No |
| API-AUTH-11 | Owner recovery from the host | `gunmetal admin recover` prints a single-use link; its use raises a banner and an audit entry. | SUR-110, SUR-070, SUR-083 | ACC-004, ADM-034 | R1 | Host | No |
| API-AUTH-12 | Help a locked-out user | An admin issues a single-use, short-lived sign-in link with a QR code that leads to enrolling a new passkey. | SUR-090 | ACC-064 | R1 | Server | No |
| API-AUTH-13 | Pairing and device keys | One pairing protocol: a new device (TV, borrowed browser, command-line tool) shows a code, a signed-in device approves it, and the new device enrols its own key pair and signs in by challenge and response. | SUR-061, F04 | ACC-061, ACC-062, ACC-051, CLI-027, INT-027, INT-028 | R2 | Server | Push (the waiting device moves on) |
| API-AUTH-14 | Profile switch and PIN | Lists the profiles a shared device may open and checks a PIN hash under the limiter. | SUR-006, SUR-079 | ACC-019, ACC-020, ACC-021 | R2 | Server | No |

### User, profile and settings

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-USR-01 | Profile identity | Name and picture per profile, separate from the sign-in account. Pictures go through the upload path, are re-encoded, and SVG is refused. | SUR-006, SUR-079 | ACC-011, ACC-017, ACC-125 | R1 | Local (read); Server (write) | Feed |
| API-USR-02 | Settings that follow the person | Settings records with a person scope and a device scope (audio output, storage, theme override), kept outside the rebuildable cache. | SUR-073 to SUR-077 | ACC-012, CLI-030, CLI-141 | R1 | Local (read); Server (write in R1) | Feed |
| API-USR-03 | Sign-in methods | List, add and remove passkeys, the password and two-factor, behind step-up confirmation. | SUR-078 | ACC-055, ACC-050, ACC-052, ACC-053 | R1 | Server | No |
| API-USR-04 | Sign-in history | The person's own audit entries. | SUR-078 | ACC-078, ADM-110 | R1 | Server | No |
| API-USR-05 | Export your data | Builds a documented export of everything the person told the server: history, loves, ratings, playlists, hides, layouts and rules. | SUR-078, F16 | ACC-010, DIS-058, MUS-188, INT-151, LAT-007 | R1 | Server | No |
| API-USR-06 | History import | Takes Last.fm or ListenBrainz export files through the upload path, matches them, and marks every imported listen so a scrobbler never sends it back. Imports can be removed as a batch. | SUR-078, SUR-097, SUR-009 | MUS-189, ADM-042, INT-107 | R1 | Server | Push (job progress) |
| API-USR-07 | Private session flag | Marks the session as not recorded: no history, no recommendations, and from R2 no scrobbles. **Proposal** from the privacy research: it ends after a period without playback. | SUR-002, SUR-010, SUR-006 | ACC-117, MUS-185, DIS-053 | R1 | Local | No |
| API-USR-08 | Household and child profiles | Managed profiles linked to policies, presets, rating ceilings, allow and block rules, schedules and the guardian's view of a child's history. | SUR-079, SUR-091, F18 | ACC-016, ACC-018, ACC-023 to ACC-029, ACC-032, ACC-034 | R2 | Server | Feed |
| API-USR-09 | What the admin can see | A page listing what administrators can see about this person. | SUR-078 | ACC-115 | R2 | Server | No |

### Device

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-DEV-01 | Device registry | Lists each device, app password and token with its last-seen time; removes one device; signs out of all. | SUR-078, F15 | ACC-068, ACC-069, ACC-070 | R1 | Server | No |
| API-DEV-02 | Sync status per device | The device's sync cursor and last sync time, for the storage screen and for admins. | SUR-075 | CLI-024 | R1 | Local | No |
| API-DEV-03 | Diagnostics from a device | Accepts a report the person built and reviewed on the device, with an optional server log excerpt. | SUR-077, F12 | CLI-033 | R1 | Server | No |
| API-DEV-04 | Device capability report | What this device can decode and output, used by the decision engine and shown to the person. | SUR-048, SUR-077 | CLI-047 | R2 | Local | No |
| API-DEV-05 | New-device notice | Tells the person's other devices that a device enrolled. Delivered in the app only; there is no remote push (CLI-077 is Later). | SUR-003 | ACC-071 | R2 | Server | Push |
| API-DEV-06 | Server picker support | Each server holds its own device key for a client that uses several servers. | SUR-072 | CLI-018, ACC-014 | R2 | Local | No |

### Sync and the device copy

The sync model is described in its own section [below](#the-sync-model).
These are the capabilities it needs.

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-SYNC-01 | Snapshot | The first full copy of the profile's synced library and user data, filtered by grants and restrictions as it is built. Proposed budget: 100,000 tracks in under 2 minutes (open decision 16). | SUR-000, SUR-020, SUR-022, F03 | CLI-022, ACC-030, ACC-037, DIS-140 | R1 | Server | No |
| API-SYNC-02 | Delta from a cursor | Everything that changed since the device's cursor, in one ordered request, so a device away for a month catches up at once. A cursor older than the compaction horizon gets a fresh snapshot. | All browse surfaces | LIB-018, CLI-022, INT-006 | R1 | Server | Feed |
| API-SYNC-03 | Grant changes as removals | When a person loses access to a library or an item, the next delta removes it from the device. | SUR-022, F10 step 8 | ACC-037, ACC-030 | R1 | Server | Feed |
| API-SYNC-04 | Profile data in the sync | The profile's slice of the user log (queue, playlists, loves, ratings, hides, layouts, rules, settings, history) travels with the library. See the size note in the sync model. | SUR-020, SUR-029, SUR-011 | DIS-002, DIS-007, MUS-122, ACC-012 | R1 | Server | Feed |
| API-SYNC-05 | Artwork in fixed sizes | Images in fixed sizes per device class, with a tiny placeholder in the metadata so tiles never show a spinner. | SUR-023 | LIB-142, MUS-040, LIB-143 | R1 | Local once cached | No |
| API-SYNC-06 | Neighbour table | A per-household table of each track's, album's and artist's nearest neighbours from credits, genres, era and co-listening, within a size budget per device, for radio and "More like this". | SUR-024, SUR-011 | DIS-060, DIS-067, MUS-165 | R1 | Local | Feed |
| API-SYNC-07 | Prebuilt search index | A fallback: if building the index on the device misses the budget on the reference low-end device, the server ships an index segment instead. | SUR-032 | DIS-084, DIS-019 | R1 (if needed) | Local | Feed |
| API-SYNC-08 | Partial sync | All metadata, but artwork within a budget with eviction, for TVs and small devices. | SUR-075 | CLI-023 | R2 | Local | Feed |
| API-SYNC-09 | Restricted profiles at sync | Kids profiles receive nothing they may not see, so nothing leaks through search, artwork, screensavers or launcher rows. | SUR-055, SUR-060 | DIS-144, DIS-155 | R2 | Server | Feed |

### Library (administration)

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-LIB-01 | Create and configure a library | A library record with a kind (music in R1, room for others), one or more roots, a spoken-word flag, and "keep off Home". | SUR-085, SUR-082 | LIB-001, LIB-003, LIB-004, LIB-011, LAT-010, DIS-012 | R1 | Host | Feed |
| API-LIB-02 | Folder browser with live checks | Lists only allowed base paths, to admins only, and checks readability, emptiness and storage type before saving. | SUR-082, SUR-085 | ADM-025, LIB-015, ADM-089 | R1 | Host | No |
| API-LIB-03 | Root settings | Exclusion patterns, watch for changes, poll interval and parallelism for shares, a safety-net schedule, read-only declared. | SUR-085 | LIB-006, LIB-013, LIB-014, LIB-015, LIB-007 | R1 | Host | No |
| API-LIB-04 | Library grants | Who may see each library; the grants filter sync and every fetch. Who sees a new library by default is unsettled (flows G4). | SUR-085, SUR-090 | ACC-037, MUS-027 | R1 | Host | Feed |
| API-LIB-05 | Change location | Points a root at a new path with a preview, keeping identity and history. | SUR-085, SUR-082 | LIB-031, ADM-051 | R1 | Host | Feed |
| API-LIB-06 | Rebuild a library | Rebuilds from the files plus the curation log, stating that fixes are kept. | SUR-085 | LIB-179, ADM-077 | R1 | Host | Push (progress) |
| API-LIB-07 | Artist splitting rules | Per-library separator and exception rules for artist strings. | SUR-085, SUR-086 | MUS-035, LIB-038 | R1 | Host | Feed |
| API-LIB-08 | Video and other kinds | Film, show and home-video kinds, tag and precedence choices, metadata language, provider order, locks, I/O profiles. | SUR-085 | LIB-002, LIB-010, LIB-055, LIB-128, LIB-183, ADM-087 | R2 | Host | Feed |

### Catalogue: artist, album, track (fields in the synced copy)

These are the fields the R1 screens draw from the device's copy. The
server's job is to derive them at scan time and keep them current through
the feed.

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-CAT-01 | Credits and roles | Every credited artist linked, the display credit as tagged, album artist apart from track artist, roles (composer, conductor, lyricist, producer, remixer, performer), "Appears on", disambiguation for same-name artists. | SUR-023 to SUR-025, SUR-013 | MUS-001 to MUS-006, LIB-187 | R1 | Local | Feed |
| API-CAT-02 | Release model | Release groups and editions, release types, compilations, discs with titles, work groupings, original and release dates, sort names. | SUR-024, SUR-025 | MUS-008, MUS-010 to MUS-013, MUS-020, LIB-045, LIB-051, LIB-056 | R1 | Local | Feed |
| API-CAT-03 | Tags for browse | Multi-valued genres, moods, styles, labels and grouping, the explicit flag, MusicBrainz IDs. | SUR-023, SUR-028 | MUS-017, MUS-019, MUS-047, MUS-036, INT-009 | R1 | Local | Feed |
| API-CAT-04 | Technical and quality data | Codec, container, sample rate, bit depth, channels and bitrate, as data the client turns into badges. | SUR-013, SUR-023, SUR-002 | MUS-021, LIB-146, MUS-099 | R1 | Local | Feed |
| API-CAT-05 | Playback data per file | Encoder delay and padding, ReplayGain or R128 values (track and album), true peak, measured loudness where available, and the seek index. See the size note in the sync model. | Player | MUS-069, MUS-071, MUS-084, MUS-085, MUS-086, MUS-088, LIB-064 | R1 | Local | Feed |
| API-CAT-06 | Lyrics | Embedded lyrics and `.lrc` sidecars, plain, line-timed and word-timed, parsed at scan, with where they came from. | SUR-012 | MUS-154, MUS-155, MUS-156, LIB-067, LIB-068 | R1 | Local | Feed |
| API-CAT-07 | Artwork palette | Up to three colour candidates per album computed from the same decode that makes the fixed sizes, for the artwork tint. | SUR-010, SUR-024, SUR-025 | MUS-110 | R1 | Local | Feed |
| API-CAT-08 | Provenance | How each field was read and where it came from, and "Upgraded on" when a better copy replaced the file. | SUR-013 | MUS-034, MUS-037, LIB-030 | R1 | Local | Feed |
| API-CAT-09 | Availability | Per item: playable, drive offline, damaged, missing; the client adds "cannot decode here" from its own capability probe. | SUR-023, SUR-011, SUR-003 | LIB-032, MUS-079, MUS-229, LIB-193 | R1 | Local | Feed |
| API-CAT-10 | Folder paths | Paths relative to each library root, for folder view and breadcrumbs. Absolute paths stay admin-only. | SUR-022, SUR-025 | LIB-008 | R1 | Local | Feed |
| API-CAT-11 | Inspect a file | For admins: every raw tag, the structure the parsers read, the identification decision and "Why is this here?", errors with their location. Shared by the file inspector, the track info sheet's admin fields and the CLI. | SUR-088, SUR-013, SUR-110 | ADM-125, LIB-195, LIB-059, LIB-097, LIB-098 | R1 | Server | No |
| API-CAT-12 | Merge, split and alias | Admin corrections to artists and albums, stored as curation-log events that survive rescans and rebuilds. | SUR-089, SUR-004 | MUS-007, LIB-041, LIB-058, LIB-179 | R1 | Server | Feed |
| API-CAT-13 | Rescan one item | Re-reads one file or folder on request. | SUR-004, SUR-088 | LIB-012 | R1 | Server | Feed |
| API-CAT-14 | Metadata editing | The edit sheet with field locks and sources, bulk edit, labels, artwork picker, fix match, item history with undo. | SUR-089 | LIB-172 to LIB-178, LIB-138 to LIB-141 | R2 | Server | Feed |

### Streaming

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-STR-01 | Sign a stream URL | A short-lived URL bound to the session, the item and an expiry, checked again for revocation on use. | Player | ACC-122, MUS-066 | R1 | Server | No |
| API-STR-02 | Silent refresh | A fresh URL during a long pause or a three-hour mix without the listener noticing; a named R1 acceptance test (flows G10). | Player "Stream URL expired" state | ACC-122 | R1 | Server | No |
| API-STR-03 | Byte ranges of the original | Serves ranges of the original file, with each range response capped and tracked so revocation can cut it. | Player | MUS-066, ACC-122 | R1 | Server | No |
| API-STR-04 | Audio packaging for browsers | Copies FLAC, Opus and MP3 frames into fragmented MP4 without re-encoding, from the scan's frame index, where a browser's Media Source Extensions need it. Per-browser support is unverified. | Player | MUS-230, MUS-067 | R1 | Server | No |
| API-STR-05 | Artwork bytes | Serves the fixed image sizes under the same authorisation. See the caching note in Flags, item 7. | SUR-023 | LIB-142, ACC-121 | R1 | Local once cached | No |
| API-STR-06 | Opus streams | A sandboxed Opus encode for mobile data or a per-person cap, keeping pre-skip so gapless survives, cached for the next device. | SUR-002, SUR-075 | MUS-106, ACC-107 | R2 | Server | No |
| API-STR-07 | Video delivery | Direct play, in-process remux and sandboxed transcode, with segments cut from the scan-time segment map. | SUR-046 | VID-001, VID-003, VID-005, LIB-088 | R2 | Server | No |
| API-STR-08 | Single keyframe | One keyframe cut by the remuxer, rate-limited, for scrub previews and chapter thumbnails. | SUR-046, SUR-047 | VID-097, VID-098 | R2 | Server | No |
| API-STR-09 | Subtitles and fonts | Embedded and sidecar subtitle streams and attached fonts, under signed URLs, rendered on the client. | SUR-047 | VID-069, VID-072, VID-073 | R2 | Server | No |

### Playback session

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-SES-01 | Session registry | Records who is playing what on which device, the delivery path and the decision reason. The decision is computed by the core on the device (F17 step 2) and reported; the server records what it actually served. | SUR-083, SUR-084 | ADM-099, ADM-100, INT-134 | R1 | Server | Push (admin views) |
| API-SES-02 | Stop a session with a message | An admin ends a session; new requests fail, in-flight responses are cut, and the client shows the message as plain text. | SUR-084, SUR-003, player "Stopped by the owner" | ADM-102, ACC-073 | R1 | Server | Push |
| API-SES-03 | Active player of a queue | Which device is currently playing a profile's queue, so a second browser shows "Playing on *device*" and "Play here" and the first stops when the second takes over. Two mechanisms are proposed (player open question 2, flows G11); pick one. **Proposal.** | SUR-002, player "Playing elsewhere" | CLI-103, MUS-122 | R1 | Server | Push |
| API-SES-04 | Play reporting | Each play with its real timestamp, counts and skips, unless the session is private. | Player, SUR-029 | MUS-182, MUS-183, CLI-093, ACC-117 | R1 | Queue | Feed |
| API-SES-05 | Control channel | A WebSocket per signed-in session; a closed command set (transfer, play, pause, seek, skip, volume, tracks, speed), each authorised per profile; a written and tested conflict rule. | SUR-014, F08 | CLI-101, CLI-102, MUS-197, MUS-198, VID-144, VID-145 | R2 | Server | Push |
| API-SES-06 | Handoff | Moves the queue, lanes, position, shuffle order and repeat mode to another device, which fetches the original itself; the source keeps playing until the target confirms (flows G14). | SUR-014, SUR-061 | CLI-101, VID-144 | R2 | Server | Push |
| API-SES-07 | Casting | Signs refreshable URLs a receiver can fetch on the home network, picks a format it can decode, and lets the phone relay away from home. | SUR-051 | CLI-106, CLI-110, CLI-111, MUS-203 | R2 | Server | Push |
| API-SES-08 | Stream limits and policy refusals | Counts playing leases, not open apps, and returns the reason and the alternatives a policy allows. | SUR-049 | VID-173, ACC-075, VID-010 | R2 | Server | No |
| API-SES-09 | Item changed during play | Tells a live session that its item gained a subtitle or a version. | SUR-049 | VID-076, INT-122 | R2 | Server | Push |

### Queue

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-QUE-01 | Queue document | One versioned queue per profile with three lanes, named listening contexts from the first version, an insertion cursor, a seeded shuffle order, repeat and stop-after modes, and the position. Stored in the user log so a rebuild keeps it. | SUR-011, SUR-002, SUR-010 | MUS-116, MUS-122, LAT-009, MUS-126, MUS-077 | R1 | Local (read) | Feed |
| API-QUE-02 | Queue operations | Small operations (play, play next, add, play last, start radio, move, remove, clear, shuffle, reshuffle) applied optimistically on the device against the last version seen; the server orders them and assigns versions; a multi-item drop is one operation. The verbs' rules live in the core. | SUR-011, SUR-004, SUR-005 | MUS-117 to MUS-120, MUS-128, MUS-063 | R1 | Server in R1; Queue in R2 | Push |
| API-QUE-03 | Stale-version rejection and rebase | Rejects an operation built on an old version; the client rebases and shows the result. Must be written and tested before handoff. | SUR-011 | MUS-122 | R1 | Server | Push |
| API-QUE-04 | Position updates | The playing device writes its position at play, pause, seek and track change, and periodically while playing, batched to respect the single writer. | SUR-002, CLI-103 prompt | MUS-122, CLI-103 | R1 | Queue | Feed |
| API-QUE-05 | Save queue as playlist | Creates a playlist from the current queue. | SUR-011 | MUS-125 | R1 | Server | Feed |
| API-QUE-06 | Undo, history and saved queues | Undo of queue edits, history above the current item, several queues with a switcher, and the music and video contexts side by side. | SUR-011, SUR-049 | MUS-121, MUS-124, MUS-131, VID-181 | R2 | Queue | Feed |

### Playlist and rules

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-PL-01 | Manual playlists | Create, rename, add, reorder and remove, as user-log events that refer to tracks by content identity, not path. | SUR-026, SUR-015, SUR-001 | MUS-132, MUS-133, LIB-028 | R1 | Server in R1; Queue in R2 | Feed |
| API-PL-02 | Pin and love playlists | Pin and love events for the sidebar and Home shortcuts. | SUR-001, SUR-020 | MUS-139, DIS-013 | R1 | Server in R1; Queue in R2 | Feed |
| API-PL-03 | M3U import and export | Imports M3U and M3U8 through the upload path and the shared matcher; exports M3U8 with paths relative to a library root. | SUR-026, SUR-009, SUR-097 | MUS-140, ADM-043, ADM-044 | R1 | Server | Push (import progress) |
| API-PL-04 | Playlists from music folders | `.m3u` files found in media folders appear as playlists. Media is read-only, so they need a read-only rule (flows G5). | SUR-022 | LIB-192, LIB-007 | R1 | Local | Feed |
| API-PL-05 | Rule store | Saved rule trees in the one rule language, for smart playlists, Home rows and saved filters, synced to devices; the core evaluates them on the device with seeded randomness. | SUR-027, SUR-026, SUR-023 | DIS-119 to DIS-122, MUS-143 to MUS-146, DIS-105, MUS-149 | R1 | Local (evaluate); Server (save in R1) | Feed |
| API-PL-06 | Server-side rule evaluation | Evaluates rules on the server when the library changes, for tools reading a smart playlist through the API and, in R2, for adapters and download rules. | SUR-026 | DIS-121, INT-138 | R1 | Host | Feed |
| API-PL-07 | Playlist write API for tools | Tools create and edit playlists with a scoped token; they appear like any other playlist. | SUR-026, SUR-094 | INT-138, ACC-049 | R1 | Server | Feed |
| API-PL-08 | Missing entries | What a playlist shows when a track is purged from the trash. **Proposal** (flows G6): keep the entry as "missing" with its last known title so a later copy rematches. | SUR-026 | MUS-132, LIB-033 | R1 | Local | Feed |
| API-PL-09 | Folders, images, sharing and collaboration | Playlist folders, a custom image, sharing with people on the server and collaborators, offline edits. | SUR-026, SUR-058 | MUS-136, MUS-138, ACC-091, MUS-150, MUS-152 | R2 | Queue | Feed |

### User log: plays, loves, ratings, history and hides

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-LOG-01 | Append user events | One write path for every event a person authors: plays, loves, ratings, dismissals and their reversals, history removals. Each event has a client-generated ID, the device ID and a hybrid logical clock, so replays are idempotent. | SUR-002, SUR-004, SUR-013, SUR-020 | MUS-180, MUS-181, DIS-045, DIS-047, DIS-022, LAT-006, LAT-007 | R1 | Queue for plays; Server for the rest in R1 | Feed |
| API-LOG-02 | Offline plays merge | Ingests plays recorded while disconnected with their real timestamps and removes duplicates. | SUR-029 | CLI-093 | R1 | Queue | Feed |
| API-LOG-03 | Remove a play | Writes a removal event; counts, statistics and recommendations follow; the screen says plays already sent to Last.fm cannot be recalled. See Flags, item 8. | SUR-029, SUR-004 | MUS-184, ACC-118, DIS-052 | R1 | Server in R1 | Feed |
| API-LOG-04 | Derived counts | Play counts, last played and skips per item, derived from the log. | SUR-023, SUR-024 | MUS-182, DIS-051 | R1 | Local | Feed |
| API-LOG-05 | Hidden and dismissed | Dismiss from Continue rows with undo and a Hidden page; hide and snooze in R2. | SUR-020, SUR-030 | DIS-022, DIS-023, DIS-054, MUS-170 | R1 | Server in R1 | Feed |
| API-LOG-06 | Statistics and year in review | Charts by period computed from the person's own log. | SUR-031 | MUS-186, MUS-187 | R2 | Local | Feed |
| API-LOG-07 | Resume points | Positions as events so resume works on any device, offline included, and survives replacing or renaming the file. | SUR-049 | VID-118, VID-120, VID-121 | R2 | Queue | Feed |

### Home, discovery and search

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-HOME-01 | Home layout | Each person's rows, their order, names and rule sources, synced so Home is the same on every device; rows are evaluated on the device. | SUR-020, SUR-021 | DIS-003, DIS-007, MUS-049, DIS-009 | R1 | Local (read); Server (write in R1) | Feed |
| API-HOME-02 | Pinned shortcuts | Pins for items and playlists at the top of Home and in the sidebar. | SUR-020, SUR-001 | DIS-013 | R1 | Local (read) | Feed |
| API-HOME-03 | Recently added without upgrades | An "added" date per album that a better copy does not reset, grouped by album. | SUR-020 | DIS-035, DIS-036, DIS-038, MUS-059 | R1 | Local | Feed |
| API-HOME-04 | Reasons on suggestions | Every suggested row and pick carries a reason the client can show. | SUR-020, SUR-011 | DIS-061, DIS-062 | R1 | Local | Feed |
| API-HOME-05 | Search | Nothing at query time: the core builds the index from the synced copy. Recent searches stay on the device (design-language section 11), so the server stores none. | SUR-032 | DIS-083 to DIS-089 | R1 | Local | No |
| API-HOME-06 | Household defaults and curation | Default Home layouts, copying a layout to someone, genre merges, the owner's picks. | SUR-107, SUR-021 | DIS-005, DIS-006, DIS-108, DIS-127 | R2 | Server | Feed |
| API-HOME-07 | Follows and alerts | Follow records in the user log and an in-app inbox that syncs like the library, optionally to the person's own ntfy topic. | SUR-003, SUR-024 | INT-050 | R2 | Local (inbox) | Feed |

### Scan jobs and tasks

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-SCAN-01 | Start a scan | A full or library scan on request. | SUR-085 | LIB-012 | R1 | Host | Push |
| API-SCAN-02 | Path-scoped refresh | Refreshes one path, for admins and for tools such as Lidarr with a scoped token. | SUR-094, SUR-100 | INT-011 | R1 | Server | Push |
| API-SCAN-03 | Scan progress | Files found, bytes read per root, an estimate, and batches committed so the library fills in while the scan runs. | SUR-083, SUR-100, SUR-022, SUR-003 | LIB-021, LIB-022, ADM-088, ADM-031, MUS-043 | R1 | Host | Push |
| API-SCAN-04 | Task list | One list of tasks (backup, scan, purge, analysis, rebuild) with run, cancel, progress, last run, duration and errors. | SUR-100 | ADM-093, ADM-095 | R1 | Host | Push |
| API-SCAN-05 | Activity and audit log | Scans, "0 changed" rescans, moves, re-reads after a parser update, imports, and sign-in, admin and recovery events. | SUR-100, SUR-083 | ADM-110, LIB-016, LIB-017, LIB-025, LIB-029, ACC-078 | R1 | Host | Push |
| API-SCAN-06 | Reprioritise, schedule and pause | Cancel and reprioritise jobs, a maintenance window, concurrency and pause. | SUR-100 | LIB-023, ADM-094, ADM-096 | R2 | Host | Push |

### Library health, review queue and trash

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-HLTH-01 | Health report | Damaged and unreadable files, tag problems with fixes, same-name collisions, sidecar problems, files the supported browsers cannot decode, missing files, moved files, offline roots, watch warnings. | SUR-086 | MUS-044, LIB-193, LIB-194, LIB-034, LIB-032, LIB-014, MUS-229 | R1 | Host | Feed |
| API-HLTH-02 | Root health | Each root's reachability and state, offline detection that greys items rather than removing them. | SUR-083, SUR-085 | ADM-108, LIB-032 | R1 | Host | Push |
| API-HLTH-03 | Review queue | Doubtful decisions with evidence and a proposal; accept, reject or choose another; answers kept in the curation log. | SUR-087 | LIB-099, LIB-051 | R1 | Host | Feed |
| API-HLTH-04 | Trash | Items whose files went missing, with when they will be purged; restore and purge now; never purges while a root is offline. | SUR-105 | LIB-033, ADM-086 | R1 | Host | Feed |
| API-HLTH-05 | Health summary | The admin home's roll-up: backups and verification, roots, free space, scan state, alerts, advisories, token expiry, recovery used. | SUR-083 | ADM-109, ADM-065, ADM-072, ADM-083, ACC-127, INT-019, ADM-034 | R1 | Host | Push |

### Users and invitations (administration)

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-ADM-01 | Local users | List users with their libraries, enable or disable without deleting, make an administrator, hand the server to a new owner by adding an admin and removing yourself. | SUR-090 | ACC-006, ACC-008, ACC-040, ADM-052 | R1 | Host | No |
| API-ADM-02 | Invitations | An invite is a capability with libraries, a use count and an expiry; a link and a QR code carry the server's public address; redemption is logged. The invite screen should show and check that address (flows G7). | SUR-090, SUR-071 | ACC-080 | R1 | Host | No |
| API-ADM-03 | Invite landing | Checks the invite and reveals nothing else; creates the account from the invite's policy. | SUR-071 | ACC-080, ACC-006 | R1 | Server | No |
| API-ADM-04 | Policies and rights | Named policies; playback, download, quality, device and remote-access rights; memberships that end on a date; stream and transcode limits. | SUR-090, SUR-091 | ACC-038, ACC-043, ACC-044, ACC-081, ACC-103, ACC-107, ACC-108, ACC-111 | R2 | Host | Feed |

### Server settings and operations

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-SET-01 | Network settings | Trusted reverse proxies, a path prefix, HTTPS with the owner's certificate. | SUR-093, SUR-082 | ACC-097, ACC-134, ACC-098, ADM-022 | R1 | Host | No |
| API-SET-02 | Privacy and the egress gate | Every outbound feature listed and off by default; grants recorded; the network activity page accounts for every connection. | SUR-093, SUR-082 | ADM-028, ACC-113, LIB-108, ADM-129 | R1 | Host | No |
| API-SET-03 | Sign-in settings | OIDC provider configuration with a test, the limiter's state, session lifetimes. | SUR-092 | ACC-057, ACC-063, ACC-079 | R1 | Host | No |
| API-SET-04 | Backups | List with status and verification, back up now, contents in plain words, download, upload, restore with a restore point and preview, full export. | SUR-098, SUR-082 | ADM-065, ADM-066, ADM-069, ADM-070, ADM-072, ADM-074, ACC-013, ADM-141 | R1 | Host | Push (progress) |
| API-SET-05 | Restore at setup | Restore from the welcome screen behind the setup code, with a dry-run remap of library roots and a warning when the domain changed (flows G3, G15). | SUR-082, F14 | ADM-029, ADM-051 | R1 | Host | Push (progress) |
| API-SET-06 | Updates | An opt-in check of a signed feed, advisories against the running version, and the rollback-safety field per release. | SUR-099, SUR-083 | ADM-053, ADM-054, ADM-060, ACC-126 | R1 | Host | No |
| API-SET-07 | Alerts and logs | Alert rules and free destinations, free-space alerts, log settings with rotation. | SUR-101 | ADM-116, ADM-083, ADM-119 | R1 | Host | Push |
| API-SET-08 | Diagnostics | Run the doctor, build a masked bundle with a preview, list local crash records, show the write queue and the derived-data store, rebuild the cache. | SUR-102, SUR-110 | ADM-123, ADM-124, ADM-130, ADM-080, ADM-141, ADM-077 | R1 | Host | Push (progress) |
| API-SET-09 | Server identity and about | Server name and sign-in message, storage locations, version, build, target and live footprint. | SUR-103 | ADM-140, ADM-090, ADM-001, ADM-010 | R1 | Host | No |
| API-SET-10 | Restart and shut down | From the UI and the emergency page. | SUR-083, SUR-081 | ADM-112, ADM-113 | R1 | Host | Push (clients see the startup page) |
| API-SET-11 | Migration imports | Listening-service files and playlists in R1; Plex, Jellyfin, Emby, Navidrome and iTunes data, dry runs and undo in R2; one matcher with reasons and an unmatched queue. | SUR-097 | ADM-030, ADM-042 to ADM-044; ADM-036 to ADM-049 in R2 | R1 | Host | Push (progress) |
| API-SET-12 | Remote access | iroh with no open ports, self-hosted and default relays, upload budget. | SUR-093 | ACC-096, ACC-100, ACC-101, ACC-109 | R2 | Host | No |

### Tokens and integrations

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-TOK-01 | Scoped tokens | Create tokens scoped by library, root and user, with expiry, rotation, last use, an audit trail, per-token rate limits, and revoke one or all; a token can never grant more than its creator holds. | SUR-094, SUR-078 | ACC-049, INT-017 to INT-022, INT-012 | R1 | Host | No |
| API-TOK-02 | Change feed for tools | The same change log that syncs devices, with a cursor per token, filtered by scope. | SUR-094 | INT-006 | R1 | Server | Feed |
| API-TOK-03 | Stable deep links | Links to an album, artist or playlist that open the app or the web client and never grant access on their own. | SUR-004, SUR-057 | CLI-034, INT-147 | R1 | Local | No |
| API-TOK-04 | Webhooks and event stream | Webhooks with event picker, templates, signing, delivery log and retry; a server-sent event stream for tools. | SUR-094 | INT-030 to INT-049 | R2 | Host | Push |
| API-TOK-05 | Plugin host | WebAssembly plugins with grants, a network allowlist and log, per-user secrets; providers, scrobblers, lyrics lookup. | SUR-095 | INT-054 to INT-069, MUS-162 | R2 | Host | No |
| API-TOK-06 | Compatibility adapters | OpenSubsonic and the Jellyfin music subset, off by default, behind the same policy layer, with per-app keys. | SUR-096, F19 | INT-086, INT-087, INT-098, ACC-130 | R2 | Server | No |

### Download (R2)

| ID | Capability | What it does | Surfaces | Features | Rel. | Offline | Live |
|---|---|---|---|---|---|---|---|
| API-DL-01 | Offline grant | The device key plus a signed record of what the device may play and until when, renewed on any contact; revocation applies on next contact. | SUR-050, SUR-091 | CLI-095, CLI-096, ACC-045 | R2 | Local (play) | Feed |
| API-DL-02 | Download files | Resumable ranges of originals, remuxed copies where the device prefers another container, Opus copies encoded once and cached; markers, chapters, subtitles and fonts travel with them; streams take priority. | SUR-050 | CLI-078, CLI-084, CLI-085, CLI-090, MUS-213, VID-175, ACC-110 | R2 | Server | No |
| API-DL-03 | Download rules | Rules in the rule language ("keep Loved tracks", "keep the next episodes") evaluated against the synced copy. | SUR-050, SUR-027 | CLI-080, CLI-081, CLI-088, MUS-211, MUS-212 | R2 | Local | Feed |
| API-DL-04 | Download on another device | Starts a download on another of the person's devices. | SUR-050, SUR-004 | CLI-097 | R2 | Server | Push |
| API-DL-05 | Download rights | A per-person download right, with the reason shown when it is missing. | SUR-050 | ACC-044 | R2 | Server | Feed |

### Video and live TV, in outline

Video and live TV need the same patterns as music (synced metadata, signed
URLs, the user log, the session registry), plus their own resources. They
are listed in outline here, because R1 should leave room for them rather
than build them.

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
requires. Where the feature map or the UI documents settle a point, it is
cited; where they do not, the text says **Proposal**.

### What is copied to the device

Per profile, filtered by the profile's grants and restrictions while the
payload is built (ACC-030; DIS-144 adds kids profiles in R2):

- **The catalogue** for every library the profile may see: the fields in
  API-CAT-01 to API-CAT-10, the neighbour table (API-SYNC-06) and, in R1,
  the lyrics and the per-file playback data the player needs (player.md,
  "What the player needs from the server and the core").
- **Artwork** in the device class's fixed sizes, fetched on first display
  and cached. R1 has no artwork budget, because partial sync (CLI-023) is
  R2.
- **The profile's own data** from the user log: the queue document, manual
  playlists, saved rules, Home layout and pins, loves, ratings, dismissals,
  settings (person scope and this device's scope) and history.
- **Facts about the server** the client needs offline: its name, the
  capabilities it reported, and the protocol version.

Never copied: absolute file paths for non-admins (folder view uses paths
relative to the root), stream URLs (always signed on demand), secrets of
any kind, other people's data, and anything the egress gate would not let
out. Recent searches are created and kept only on the device
(design-language section 11).

Three items in that list may be too large to copy in full, and the
documents do not yet budget them:

1. **Seek indexes.** player.md puts each file's seek index in the synced
   library. At 100,000 tracks that may be large (unverified). **Proposal:**
   sync codec, trim, gain and true peak for every track, and fetch the seek
   index for a track when it enters the queue, so the device holds it
   before play and offline playback in R2 gets it with the download.
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

- **One ordered change log.** The scan diff is the feed (LIB-018). Every
  change to items, artwork, relations and the profile's user data gets a
  sequence number. A device holds a cursor (CLI-024) and asks for
  everything after it in one request (API-SYNC-02).
- **Snapshot first.** A new device takes a snapshot, then deltas. A device
  whose cursor is older than the compaction horizon is told to take a fresh
  snapshot. Until the first snapshot lands, the device must say it is not
  yet available offline (flows G12).
- **Nudges, not polling.** While connected, the server pushes a "changes
  available" nudge over the client event channel (API-SYS-10); the client
  then pulls the delta. Without a channel the client polls, and things that
  depend on freshness (the "Playing on" note, a removed library) are only
  as quick as the poll (player.md marks this unverified).
- **Filtering is per profile.** The change log is shared, but each
  profile's delta is filtered by its grants. When grants shrink, the server
  must send removals for everything the profile can no longer see
  (API-SYNC-03). **Proposal:** treat a grant change as a reason to resend a
  snapshot of the affected library rather than computing a precise removal
  set, because a missed removal is a disclosure.
- **Secure context.** In browsers, keeping the copy across reloads,
  loading with the server down and installing the app all need HTTPS or
  localhost (CLI-150). Over plain HTTP on a LAN address the web client is
  online-only, as the map states.

### How changes flow from the device to the server

There are three kinds of write, and each has its own merge rule.

1. **Events** (plays, loves, ratings, dismissals and their reversals,
   history removals, resume points in R2). Appended to the user log with a
   client-generated event ID, the device ID and a hybrid logical clock
   (CLI-093). Replays are idempotent because the event ID is the key.
   Derived values (counts, last played) are recomputed from the log.
2. **Operations on versioned documents** (the queue, playlists, the Home
   layout, rule trees). Each operation names the version it was built on.
   The server orders operations and assigns versions; for the queue it
   rejects stale operations and the client rebases (MUS-122). In R2,
   operations made offline are replayed on return under stated conflict
   rules (CLI-094).
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
promises.

| Data | Rule | Why |
|---|---|---|
| Plays | Union of all events, de-duplicated by event ID | A play happened; nothing should lose it |
| History removal | The removal hides the play whatever order they arrive in | MUS-184 promises removal reaches every device |
| Love, rating | The event with the latest hybrid clock wins, per person and item | A toggle has one current value |
| Dismiss, hide, snooze | Latest wins; undo is a reversal event | DIS-023 needs undo to survive sync |
| Queue (online) | Server order; stale operations rejected and rebased | MUS-122 |
| Queue (two players) | The device that last pressed Play becomes the active player; the other pauses and shows "Playing on *device*" | player.md open question 2, flows G11 |
| Queue (offline edits, R2) | Replayed as operations on the current version; edits to items that no longer exist are dropped; the active player's position wins | Edits are rare offline; position belongs to whoever is playing |
| Playlist entries | Entries have their own IDs. Concurrent adds are both kept; a remove beats a concurrent move; a reorder applies relative to neighbours, not indexes | Index-based edits corrupt under concurrency |
| Playlist name, rule tree, Home layout | Latest wins for the whole field | Merging two rule trees produces rules nobody wrote |
| Edit to a playlist deleted elsewhere | The deletion stands and the device shows the rare conflict notice (CLI-094) | The person who deleted it meant it |
| Settings | Latest wins per key and scope | CLI-030 |
| Resume points (R2) | Latest event wins, per item and version | VID-118, VID-120 |
| Household curation (merges, splits, locks) | Admin-only and online; server order | Curation is never edited offline |

The map's own rows already settle one more point: offline downloads stop
when the grant expires and a revocation applies on next contact (CLI-095,
CLI-096), and the documentation must say so.

## Background jobs the server runs

| Job | Trigger | What it produces | Features | Rel. |
|---|---|---|---|---|
| Library scan | Manual, after adding a library, at first run | The catalogue, identity, health records and change-log entries, committed in batches; header-only reads; no helper process per file; bytes read per root | LIB-012, LIB-019, LIB-020, LIB-021, ADM-088 | R1 |
| Change detection | File watcher on local disks; polling on shares and cloud drives; a scheduled safety-net scan | Rescans of only what changed; moves and renames that keep identity; better copies recorded as upgrades | LIB-013 to LIB-017, LIB-029, LIB-030 | R1 |
| Path-scoped refresh | An admin or a tool with a scoped token | A rescan of one path | INT-011 | R1 |
| Parser-upgrade re-read | Startup after an upgrade | Re-reads only files whose parser version changed, keeping derived data | LIB-025, ADM-141 | R1 |
| Artwork processing | During scan | Fixed sizes per device class, the tiny placeholder and the palette, from one safe decode | LIB-142, LIB-143, MUS-110 | R1 |
| Loudness analysis | After scan, at low priority, throttled and checkpointed | Measured loudness for untagged tracks; depends on the decoder decision (open decision 8), otherwise nothing runs and the fallback gain applies | MUS-086, MUS-089, LIB-024, ADM-095 | R1 |
| Neighbour table rebuild | Nightly and after a scan | The table radio and "More like this" read | DIS-060 | R1 |
| Server-side rule evaluation | When the library or a rule changes | Smart playlist contents for tools (and in R2 for adapters and download rules) | DIS-121, INT-138 | R1 |
| Playlist files in folders | During scan | Playlists from `.m3u` files found in music folders | LIB-192 | R1 |
| Import matching | After an upload of history or playlists | Matches with reasons and confidence; misses sent to the review queue | ADM-042 to ADM-044, MUS-140 | R1 |
| Change-log compaction | Scheduled | A bounded log; cursors older than the horizon are told to resnapshot | LIB-018, INT-006 | R1 |
| Root health | Continuous and on access | Offline roots marked, items greyed, alerts raised; trash purges held | LIB-032, ADM-108 | R1 |
| Trash purge | After the grace period | Entries for missing files removed, never while a root is offline | LIB-033 | R1 |
| Daily backup and verification | Daily by default | A consistent snapshot of the user log, identity store and settings with a manifest, verified after writing; alert on failure | ADM-065, ADM-066, ADM-072 | R1 |
| Pre-upgrade snapshot and migration check | Startup on a new version | A snapshot, migrations run on a copy, serving only after they pass | ADM-056, ADM-057, ADM-058 | R1 |
| Cache rebuild | Older binary on newer data, or on request | The cache rebuilt from the files plus the log, reusing derived data | ADM-059, ADM-077 | R1 |
| User-log recovery | Startup | Recovery from a torn write using checksums | ADM-078 | R1 |
| Update and advisory check | Daily, only when the owner allowed it | The signed feed verified against a pinned key; advisory and rollback-safety banners | ADM-053, ADM-054, ADM-060 | R1 |
| Free-space guard | Continuous | Alerts, and refusal of writes that would fill the disk | ADM-083 | R1 |
| Alert dispatch | On events | Delivery to the free destinations the owner chose | ADM-116 | R1 |
| Log rotation | Scheduled | Structured logs rotated | ADM-119 | R1 |
| Expiry sweeps | Scheduled | Lapsed setup codes, invitations, recovery and help links, pairing requests (R2), sessions and tokens; token-expiry banners | ACC-001, ACC-080, ACC-064, ACC-079, INT-019 | R1 |
| Open-response tracking | Continuous | Range responses per session, aborted on revocation | ACC-122 | R1 |
| Crash records | On a crash | A local record for the diagnostics page | ADM-130 | R1 |
| Opus encoding | On demand, cached | Opus streams and download copies in the sandbox | MUS-106, MUS-213 | R2 |
| Remux and transcode workers | On demand | Segments for browsers, TVs and casting; transcodes in the sandbox only | VID-003, VID-005 | R2 |
| Sandbox self-test | Startup and on request | Whether transcoding can run, shown to the owner | VID-009, ADM-132 | R2 |
| Intro detection | After scan, per season | Skip markers | VID-112 | R2 |
| Smaller versions | At idle, under the owner's rules | Pre-made versions for remote and mobile play | VID-026 | R2 |
| Metadata providers and lookups | On scan and refresh, only with grants | Provider data in the sandboxed plugin host | LIB-107, LIB-111, LIB-112 | R2 |
| Scrobble and webhook delivery | On events, with retries | Deliveries with a log | INT-030 to INT-047, MUS-192 to MUS-195 | R2 |
| Follow alerts | On new arrivals | Inbox entries for followers | INT-050 | R2 |
| Offline grant renewal and revocation list | On contact | Renewed grants; revoked grants refused | CLI-095, CLI-096 | R2 |
| Off-site backups | Scheduled | Encrypted copies at the owner's destination | ADM-068, ADM-073 | R2 |
| Guide and source refresh | Scheduled | Guide diffs applied; sources re-probed | LIV-035, LIV-003 | R3 |
| Recording scheduler and recorder | Scheduled | Recordings, conflicts, recording alerts | LIV-102 to LIV-141 | R3 |
| Time-shift buffer | While a channel plays | A rolling buffer for pause and start over | LIV-095, LIV-097 | R3 |

## UI requirements the architecture makes hard or impossible

Each item names the interface requirement, the architecture record or rule
that gets in its way, and what the documents propose.

1. **Durable user data beyond watch history.** The UI keeps queues,
   playlists, loves, ratings, layouts, hides, settings and corrections
   across rebuilds and restores. ADR 1 (decision 5) makes SQLite a
   rebuildable cache and only watch history irreplaceable. Without ADR 3
   (feature map open decision 1) almost every R1 write in this list has
   nowhere durable to live. Severity: blocking for R1.
2. **Gapless playback in the browser.** The player promises gapless
   playback in a browser tab (MUS-067). Where Media Source Extensions do
   not accept raw FLAC, Ogg Opus or MP3, that needs the light audio
   packager (MUS-230), which contradicts ADR 2 (decision 2, "no remuxer").
   Open decision 9 proposes amending ADR 2. Per-browser support is
   unverified, including Safari.
3. **"Measured" loudness.** The track info sheet shows whether gain was
   tagged, measured or estimated. Measuring needs a full decode with a
   third-party crate, which ADR 1 (decision 3) and the pure-Rust parsing
   rule do not yet allow (open decision 8). Without that decision R1
   shows only "tagged" or "estimated".
4. **A track the browser cannot play.** The UI dims it with the reason
   (MUS-229). ADR 2 (decision 2) means R1 has no transcoder, so R1 can only
   explain, never fix (flows G13). Which core formats each browser lacks is
   unverified.
5. **Passkeys, offline loading and installation in R1.** All need a secure
   context (CLI-150). ADR 1 (decision 7) rules out a central account, and a
   project-issued HTTPS name (ADM-023) needs its own ADR (open decision 7).
   A plain-HTTP LAN install therefore gets password plus two-factor, sends
   that password unencrypted on the home network (flows G2), and, as the
   map states, cannot keep its synced copy across reloads, so "instant Home
   with no spinner" (DIS-002) and "browsing with the server down" (CLI-025)
   do not hold there. Whether a browser keeps a non-persistent store across
   reloads on plain HTTP was not checked here (unverified).
6. **Passkeys after a move.** A passkey belongs to its domain. Restoring a
   backup under a new address breaks every member's passkeys (flows G3).
   The proposed remedy is a warning at restore and one-time sign-in links.
7. **Short-lived signed URLs.** ADR 1 (decision 6) and ACC-122 make media
   and image URLs short-lived and session-bound. That collides with several
   UI promises: a long pause or a long mix outliving a URL (needs silent
   refresh, API-STR-02); artwork handed to the operating system's media
   controls landing a signature in an OS cache (player.md proposes local
   blob artwork, whose browser support is unverified); cast receivers that
   cannot refresh easily and cannot join iroh (R2); and browser caching of
   artwork, because a URL that changes at every expiry defeats the HTTP
   cache. **Proposal** for the last: for the first-party web client on its
   own origin, serve artwork at content-addressed paths authorised by the
   session cookie, so the image is cacheable and no signature appears in a
   URL.
8. **Removing a play from an append-only log.** The history page promises
   "Remove this play" (MUS-184). ADR 1 (decision 5) makes the log
   append-only, so the map implements removal as a new event that hides
   the play. The privacy research argues that real erasure needs one
   sanctioned rewrite of the log and of every copy, including backups by
   retention. Until an ADR says so, a removed play still exists on disk and
   in backups, and the UI must not claim more than that.
9. **Notices when the app is closed.** The notice centre carries new-device
   alerts and follow alerts (ACC-071, INT-050, R2), and the R3 system
   surfaces list recording alerts (LIV-111, LIV-155). Remote push needs
   Apple's and Google's services, which ADR 1 (decision 7) and the map set
   aside (CLI-077 and INT-052 are Later). These notices therefore arrive
   only while the app holds a connection or next syncs. Scheduled events
   such as reminders can be local notifications from synced data (LIV-075);
   unexpected failures such as a recording that broke cannot.
10. **Remote access from a browser.** iroh (ADR 1, decision 7) reaches only
    native apps; browsers would be relay-only (ACC-102, Later). In R1 and
    for the web client in R2, remote use depends on the owner's proxy, VPN
    or domain, so an invite may carry an address the friend cannot reach
    (flows G7).
11. **One React Native codebase under a strict content security policy.**
    ADR 1 (decision 8) puts the web client on React Native Web; the
    security baseline allows only `style-src 'self'`. Per-album tints must
    go through the CSSOM, and whether React Native Web's style injection
    works under that policy is unverified (design-language open question
    8). The Linux desktop shell has no first-party React Native target
    (open decision 21, unverified).
12. **Live updates in R1.** Several R1 screens need the server to speak
    first: stopping a session with a message (ADM-102), cutting a revoked
    device (ACC-069), the "Playing on *device*" note (player.md), the first
    scan filling the library in (LIB-021), and the admin's now-playing list
    (ADM-099, whose server need names an event stream). The map places the
    public event stream (INT-048) and the control channel (CLI-101) in R2.
    This is a gap in the release cut rather than an ADR conflict.
    **Proposal:** a private, first-party client event channel in R1
    (API-SYS-10), which R2's control channel and INT-048 then extend.
13. **Device-side computation within budget.** Search, Home rows, smart
    playlist previews and radio all run in the core on the device (ADR 1,
    decision 2). The proposed budgets (Home under 200 ms, search under
    50 ms at 100,000 tracks on the reference low-end device; open decision
    16) are unmeasured, and the reference devices are not yet named. The
    prebuilt index (API-SYNC-07) is the only stated fallback.
14. **Third-party apps and credentials in URLs.** The OpenSubsonic adapter
    (ADR 2, decision 6) serves apps whose streaming and sign-in put
    credentials in query strings, which ACC-123 and INT-023 forbid on
    native routes. The map makes the adapter routes documented exceptions
    with per-app keys (INT-087) and keeps legacy password-derived sign-in
    Later and off (INT-088). Which Subsonic apps support API keys is
    unverified, so F19 may work for fewer apps than hoped.
15. **High-rate writes and one writer.** Position updates, play events and
    (R2) remote-control reports all write user data, and ADM-080 allows one
    writer. The player only needs positions at state changes and
    occasionally while playing, so the client must batch, and the server
    must never write a position per second per session.

Two UI requirements have no feature row behind them yet and so no server
capability here: the TV rail's Now Playing entry and lyrics on TV
(surfaces.md open question 4). Both appear to need nothing beyond the queue
document and the synced lyrics.
