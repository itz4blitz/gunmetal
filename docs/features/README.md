# Feature map

This folder maps every feature Gunmetal might build, area by area, so that
the screens the UI needs and the server work behind them can be read off
one place. It was written on 2026-10-02 from the research in
`docs/research/` and stays inside the two accepted architecture records:
[ADR 1](../adr/0001-architecture.md) (architecture) and
[ADR 2](../adr/0002-music-is-first-class.md) (music is first-class). It is
built on the [security baseline](../security/README.md): its first
principles are absolute, and a row that breaks one is a defect in the row.
This README is the shared key for all ten area files. Where an area file and
this README disagree about a release, this README wins and the area file is
a bug; where either disagrees with the baseline's release-scope table, the
table wins (SEC-TM-074). Open questions are settled in the
[decision register](../decisions.md).

| Area | File | What it covers |
|---|---|---|
| Music | [music.md](music.md) | The music model, browsing, the player and audio path, queue, playlists, lyrics, radio, history, scrobbling, handoff and offline music |
| Library and metadata | [library.md](library.md) | Libraries and folders, scanning, file identity, identification, providers, artwork, versions, curation and library health |
| Discovery, home and search | [discovery.md](discovery.md) | Home, continue rows, what is new, loves and history, recommendations, search, browse, the rule language, the TV interface |
| Clients, devices and offline | [clients.md](clients.md) | Platforms, sync, TV, phone, desktop and web, background playback, downloads, handoff and casting, car, accessibility |
| Accounts, sharing and security | [accounts.md](accounts.md) | Ownership, accounts, profiles, parental controls, sign-in, devices, invitations, sharing, remote access, privacy |
| Setup, administration and operations | [admin.md](admin.md) | Installing, first run, migration, upgrades, backups, database health, storage, tasks, dashboard, alerts, logs |
| Integrations and extensibility | [integrations.md](integrations.md) | The native API, tokens, webhooks, plugins, the OpenSubsonic and Jellyfin adapters, third-party tools |
| Adjacent media types | [later-media.md](later-media.md) | Audiobooks, podcasts, music videos, home videos, photos, ebooks, comics |
| Video playback | [video.md](video.md) | Direct play, remux and transcode, HDR, audio, subtitles, seeking, markers, resume, casting, watch together |
| Live TV and recording | [live-tv.md](live-tv.md) | IPTV and tuners, guide data, lineup, live playback, recording, reliability |

## How to read the map

### The columns

Every feature table in every area file has the same ten columns.

| Column | Meaning |
|---|---|
| **ID** | A stable identifier (see below). |
| **Feature** | A short name for the feature. |
| **What the user gets** | The outcome in the user's words. For a reference row, it names the owning row. |
| **Rivals today** | What Plex, Jellyfin, Emby, Navidrome and the specialist apps do, from the research. "(unverified)" marks a claim the research could not confirm. |
| **Demand** | The evidence of demand, on the scale below. |
| **Release** | One of the eight release values below. |
| **How Gunmetal does it better** | The edge, or an honest "parity" or "behind" where a rival is already good. |
| **Server needs** | What the server (or the shared Rust core) must provide. "None beyond X" means the owning row X covers it. |
| **UI surfaces** | The screens or controls the feature appears on, named so the UI plan can collect them. |
| **Security** | The SEC requirement IDs the row's tests must verify (SEC-TM-001, SEC-STD-004). A row with none uses one of three fixed forms, which the docs lint accepts: `None (No)` for a No row, which is never built; `See X` for a reference row, whose tests are the owning row's; and a cell starting `None specific` for a row that adds no security control or trust boundary of its own, such as a client-side sleep timer. Every row also falls under its file's Security notes, which name the trust boundaries and threats the file's features cross. |

### Demand

The Demand cell always starts with one of three words, followed by a colon
and the evidence when there is any:

- **High**: a top request on a rival's board (typically hundreds or thousands
  of votes), a named pain point in the research, or a basic expectation
  every rival meets.
- **Medium**: a real but smaller request, or a basic expectation for one
  audience.
- **Low**: little or no evidence of demand; often built because the
  architecture needs it.

Vote and reaction counts are as the research files recorded them on
2026-10-02. "no vote data" or "no vote count" means the research found
none, not that nobody wants it.

### Releases

The Release cell holds exactly one of these eight values: R1, R1.1, R1.2,
R1.3, R2, R3, Later or No. R1.1 to R1.3 are point releases of R1. On
2026-10-02 the owner adopted the smaller R1 in the decision register's
[R1 scope](../decisions.md#r1-scope) section, with the rest of the earlier
R1 in those three point releases
([D-10](../decisions.md#d-10-r1-scope-and-the-release-table)). The owner
also moved the project-run name service to R2
([D-07](../decisions.md#d-07-https-and-naming-record-8)).

| Release | What it contains | Clients | Infrastructure |
|---|---|---|---|
| **R1** (music) | The first usable release. A server that installs safely and is claimed with a passkey over real HTTPS. It scans a music folder and plays the original files gaplessly in a browser, levelled from their loudness tags. It keeps a queue, manual playlists and lyrics from the files. People sign in by passkey or by approval from a signed-in device (no passwords) and invite others. Each person sees, erases and exports their own history. Backups, restore at setup, and the security baseline. | The web client only, in browsers on desktop and phone. The installable web app arrives in R1.1. | No remuxer and no transcoder (ADR 2, decision 2). One light audio-only packager for browser gapless (MUS-230), which runs in a worker process and streams over a pipe, never in the server process (SEC-MED-018, SEC-MED-081). Linux servers only: x86-64, ARM64 and the container image. No plugin host and no metadata provider. HTTPS comes from the owner's own domain with automatic certificates (ADM-022, ACC-099), a tailnet name, or the machine itself (CLI-150). There is no project name service (ADM-023 is R2). No iroh: remote use goes through the owner's reverse proxy or a tailnet (ACC-097). |
| **R1.1** (bring your music in) | Playlist files and history imports. Built-in MusicBrainz and Cover Art Archive lookups (LIB-111, LIB-112), off until the owner turns them on in the setup step that lists what each provider receives. Ratings, richer credits, editions and browsing, filters and saved filters. Offline loading and the installable web app. Profile pictures. Continue on this device. | As R1, plus the installable web app (CLI-003). | The egress client gains the metadata-provider purposes (SEC-PRV-014, SEC-PRV-015, SEC-PRV-017). Playlist files are parsed in the worker (SEC-MED-050, SEC-HIS-018). Uploaded images are re-encoded (SEC-MED-061, SEC-PRV-006). |
| **R1.2** (the household and the admin) | Single sign-on with OIDC (ACC-057). Music share links (ACC-086 to ACC-089). Several administrators. The admin's live view, with each person's opt-in for titles (ADM-099, MUS-235), and stopping a stream. An arrangeable home. The task list, diagnostic bundles and restore from the UI. Serving under a path prefix behind a reverse proxy (ACC-134). Deep links and translations. | As R1.1. | The OIDC client (SEC-TM-022, SEC-IAM-026 to SEC-IAM-036, SEC-STD-025, SEC-CLI-026). Share-link pages (SEC-API-097, SEC-STD-008). Diagnostic bundles (SEC-PRV-046, SEC-OPS-030). |
| **R1.3** (discovery and analysis) | The rule language and smart playlists. Library radio and suggestions. Loudness measured for untagged files (MUS-086), if ADR 5 is accepted. Folder view. Manual curation, the review queue, and fixes that survive every rebuild. Builds for 32-bit ARM boards (ADM-004). | As R1.1. | One reviewed pure-Rust audio decoder in the scan worker, for loudness (ADR 5, SEC-MED-026). The derived-data store (ADM-141) and the neighbour table. |
| **R2** (video) | Movies and TV, plus everything that needs a native app or a sandbox: downloads, handoff and remote control, casting, the plugin host and the scrobblers, the OpenSubsonic adapter and the Jellyfin adapter's music subset, API keys, household profiles and parental controls, video share links (off by default), webhooks, Opus streams. Built-in remote access. The project-run per-server name service, with its naming client and its certificate-transparency monitoring (ADM-023, register D-07). | Android phones and tablets, Android TV and Google TV (and Fire OS), and the web client. | The pure-Rust remuxer, in a worker process (SEC-MED-081); the sandboxed transcoder (software encoders); iroh remote access for native clients; the name service (SEC-NET-010 to SEC-NET-012, SEC-NET-069 to SEC-NET-071); the plugin host, with each plugin in its own sandboxed process. |
| **R3** (live) | M3U playlists and live TV: sources, guide, lineup, live playback, recording. | As R2. | The live TV ingest and recorder. |
| **Later** | Wanted, not scheduled. Includes the Apple platforms, Samsung and LG TVs, the desktop shell (SEC-TM-074), Roku through the Jellyfin adapter's video subset, hardware transcoding, audiobooks and the other adjacent media types, household activity features, and watch together. | | |
| **No** | Deliberately not doing. The row says why. | | |

Three notes apply across all files:

- **Point releases.** R1.1, R1.2 and R1.3 ship after R1 in that order. They
  use the same server platforms and the same web client, and none adds a
  native app. The native Android music app is not part of R1.x (register
  D-14). A security requirement whose only surface moved to a point release
  is due in that release, and the release cannot ship without it
  (SEC-STD-004; register "Security requirements for the adopted R1"). Every
  other R1 requirement stays due in R1.
- **Apple platforms** (iPhone, iPad, Apple TV, CarPlay, AirPlay, Apple
  Watch) are **Later**, with the note "R2 if the App Store licence decision
  allows" (open decision 3). The App Store terms and the AGPL with no
  contributor agreement are the blocker, not engineering.
- **Feature release versus client reach.** A feature is placed in the
  first release in which it ships on at least one client. An R1 or R1.x
  feature reaches the native apps when they ship in R2; the UI surfaces
  column says where.

How this lines up with the README roadmap: R1 and its point releases are
the roadmap's server and web client milestones. R2 is "Mobile and Android
TV", "Video", the iroh part of "Remote access" (with the project name
service), and the OpenSubsonic part of "Adapters"; R3 is
"M3U playlists and live TV". "Apple builds, Samsung and LG packaging" and
the Jellyfin adapter's video subset are Later. The root README lists remote
access and adapters after video as separate milestones; it should be
updated to match (open decision 2).

### IDs and ownership

- An ID is a three-letter area prefix and a number: ACC (accounts), ADM
  (admin), CLI (clients), DIS (discovery), INT (integrations), LAT (later
  media), LIB (library), LIV (live TV), MUS (music), VID (video).
- IDs are stable. A row is never renumbered or deleted; a new feature gets
  the next free number in its area, so numbers within a file are not always
  in order.
- **Every feature has one owning row.** When the same feature appeared in
  several maps, one row became the owner and the others became **reference
  rows**: their "How Gunmetal does it better" cell begins "See X." and their
  Server needs say "None beyond X". A reference row keeps any detail that
  is specific to its area (for example the music drop zones on a shared
  multi-select). A reference row carries the owner's release, except where
  it applies the owner's feature to a later media type (for example VID-130
  applies ACC-117's private sessions to video in R2).
- The default split is: **accounts** owns identity and security;
  **admin** owns operational screens; **library** owns scanning, change
  detection, file identity and library health; **music** owns the music
  model, music tag interpretation, the player and playlists; **discovery**
  owns home, search, browse, recommendations and the rule language;
  **clients** owns platforms, offline, downloads, handoff, casting, desktop
  and TV behaviour.

### Shared terms

- **The user log**: the append-only, exportable store for everything a
  person authors that a rescan cannot recreate: history, plays, loves,
  ratings, playlists, queue state, hides, layouts, and household curation
  (corrections, locks, labels, collections). Entries are scoped to one
  profile or to the household; the household-scoped entries are called
  **the curation log**. ADR 1 (decision 5) creates this log for watch
  history only; extending it is ADR 3 (open decision 1).
- **The identity store**: the durable store for accounts, credentials'
  public keys, device keys, policies, invitations, grants, server keys and
  the mapping from random public IDs to content identity (INT-008). It also
  needs ADR 3.
- **Content identity**: the single rule in LIB-028.
- **The synced library**: the per-profile copy of library metadata kept on
  each device and read by the shared Rust core (WASM in browsers, UniFFI in
  native apps).
- **The core**: the shared Rust crate.

## Summary by area

Counts are the real rows in each file's feature tables, counted
mechanically on 2026-10-03, after the maps were realigned to the owner's
answers to D-07 and D-10. Reference rows are counted in their own area and
release, and the last column says how many of each area's rows are
references (rows whose "How Gunmetal does it better" cell begins with the
word See followed by a feature ID or a quoted section name, such as
`See "Deliberately not doing"`; cells that begin "Seeded", "Seeds",
"Seerr" or "See episodes" are not references).

| Area | File | R1 | R1.1 | R1.2 | R1.3 | R2 | R3 | Later | No | Total | Of which references |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Music | [music.md](music.md) | 72 | 30 | 4 | 9 | 74 | 1 | 38 | 7 | 235 | 48 |
| Library and metadata | [library.md](library.md) | 54 | 15 | 1 | 7 | 93 | 1 | 29 | 7 | 207 | 42 |
| Discovery, home and search | [discovery.md](discovery.md) | 28 | 17 | 7 | 10 | 89 | 3 | 29 | 7 | 190 | 32 |
| Clients, devices and offline | [clients.md](clients.md) | 20 | 10 | 3 | 0 | 80 | 2 | 38 | 6 | 159 | 2 |
| Accounts, sharing and security | [accounts.md](accounts.md) | 52 | 1 | 10 | 0 | 50 | 2 | 17 | 7 | 139 | 7 |
| Setup, administration and operations | [admin.md](admin.md) | 58 | 5 | 16 | 3 | 42 | 1 | 18 | 5 | 148 | 7 |
| Integrations and extensibility | [integrations.md](integrations.md) | 8 | 1 | 2 | 0 | 105 | 2 | 38 | 8 | 164 | 6 |
| Adjacent media types | [later-media.md](later-media.md) | 4 | 0 | 0 | 3 | 13 | 0 | 149 | 12 | 181 | 5 |
| Video playback | [video.md](video.md) | 0 | 0 | 0 | 0 | 139 | 1 | 43 | 5 | 188 | 10 |
| Live TV and recording | [live-tv.md](live-tv.md) | 0 | 0 | 0 | 0 | 0 | 144 | 24 | 12 | 180 | 2 |
| **All areas** | | **296** | **79** | **43** | **32** | **685** | **157** | **423** | **76** | **1,791** | **161** |

## The R1 cut

This list is the definition of the first release. The owner adopted it on
2026-10-02: it is the R1 in the decision register's
[R1 scope](../decisions.md#r1-scope) section
([D-10](../decisions.md#d-10-r1-scope-and-the-release-table)), without the
name service (ADM-023), which
[D-07](../decisions.md#d-07-https-and-naming-record-8) moved to R2. The
list is generated from the maps' R1 rows, so it cannot drift from them.
R1 holds 296 rows: 264 owning rows and 32 references to them. The
register's list counts two of these references, ADM-121 and CLI-157, as
features. Video and live TV have no R1 rows.

How R1 got here. Before the first review the maps put about 600 rows in
R1. That review moved these to R2: the household and parental system,
downloads, handoff and remote control, the plugin host and scrobblers, the
OpenSubsonic adapter, the webhook and public-API platform (API keys
included), rival-database importers, in-app metadata editing, most
observability screens, and the second tier of discovery rows. The security
alignment then added the rows the baseline requires, which made 455 rows.
It also moved API keys and their rows (ACC-049, INT-011, INT-012, INT-018
to INT-022, INT-026) to R2, and made the password fallback and two-factor
codes No (ACC-052, ACC-053). The owner's answers then moved 154 of those
455 rows to point releases: 79 to R1.1, 43 to R1.2 and 32 to R1.3 (see
"After R1: the point releases" below). Five more went to R2:

- the name service (ADM-023, D-07);
- old clients keep working (CLI-032), which matters only for native apps;
- the change feed and playlist writes for tools (INT-006, INT-138), which
  need API keys;
- LIB-056, whose owning row (MUS-014) is R2.

Even this R1 is larger than the 120 to 150 rows a reviewer proposed, for a
reason worth stating. Many R1 rows are rules of one component rather than
separate pieces of work (for example ACC-120 "every endpoint needs
sign-in", ACC-123 "secrets never in URLs or logs", ADM-080 "one writer"),
and the research shows those rules are where the rivals' 2026 advisories
came from. The build order inside R1 should still be: the music model and
scan, the player (gapless, levelling, the honest unplayable state), the
queue and playlists, lyrics, search and home, then sign-in, backups and the
security baseline. The security doors themselves come first in the plan's
waves, because every feature uses them.

### Music (67 features, plus 5 references)

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
- MUS-050: Continue listening by album or playlist
- MUS-051: Artist page
- MUS-052: All songs by an artist
- MUS-054: Album page
- MUS-059: Recently added that ignores upgrades
- MUS-060: Genre, mood and label browse
- MUS-061: Music search fields
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
- MUS-116: Three-lane queue
- MUS-117: Play next keeps your order
- MUS-118: Add to queue and play last
- MUS-119: Edit the whole queue
- MUS-122: Persistent queue on every device
- MUS-123: "Playing from" on every item
- MUS-126: Shuffle modes: random and spread out
- MUS-132: Manual playlists
- MUS-133: Add-to-playlist sheet
- MUS-149: Loved tracks as a playlist
- MUS-154: Embedded lyrics
- MUS-155: Synced .lrc sidecars
- MUS-180: Loves
- MUS-182: Play counts, last played and skips
- MUS-183: Listening history by date
- MUS-184: Remove plays from history
- MUS-188: Export listening history
- MUS-208: Instant browsing from the synced library
- MUS-227: Accessible player
- MUS-229: Honest unplayable state
- MUS-230: Audio packaging for the web player
- MUS-233: Clear history for a period, or all of it
- MUS-234: Choose how long history is kept

References that ship with their owning rows: MUS-038 (see LIB-028), MUS-042 (see LIB-016), MUS-043 (see LIB-021), MUS-062 (see DIS-111), MUS-185 (see ACC-117).

### Library and metadata (38 features, plus 16 references)

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

References that ship with their owning rows: LIB-036 (see MUS-034), LIB-037 (see MUS-001), LIB-038 (see MUS-035), LIB-039 (see MUS-003), LIB-040 (see MUS-006), LIB-042 (see MUS-004), LIB-044 (see MUS-020), LIB-046 (see MUS-036), LIB-049 (see MUS-011), LIB-050 (see MUS-012), LIB-053 (see MUS-017), LIB-063 (see MUS-021), LIB-064 (see MUS-069), LIB-065 (see MUS-084), LIB-067 (see MUS-154), LIB-068 (see MUS-155).

### Discovery, home and search (27 features, plus 1 reference)

- DIS-001: Home made only of your media
- DIS-002: Instant home
- DIS-004: Good default home and empty states
- DIS-020: Continue listening
- DIS-021: Recently played
- DIS-035: Recently added
- DIS-036: Arrivals grouped by album
- DIS-038: Upgrades are not "new"
- DIS-045: Loves
- DIS-046: Loved songs as a list
- DIS-050: History by date
- DIS-051: Your play counts
- DIS-052: Remove a play
- DIS-058: Export everything you told the server
- DIS-083: One search box
- DIS-084: Search on the device
- DIS-085: Forgiving matching
- DIS-100: Endless, smooth lists
- DIS-104: Sorts that matter
- DIS-109: Browse pages
- DIS-111: The same menu everywhere
- DIS-112: Back keeps your place
- DIS-140: Discovery per person
- DIS-186: Clear a period or all history
- DIS-187: Choose how long history is kept
- DIS-188: History held during account recovery
- DIS-189: "Only you can see this"

References that ship with their owning rows: DIS-053 (see ACC-117).

### Clients, devices and offline (19 features, plus 1 reference)

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
- CLI-150: Secure context in R1
- CLI-155: Personal or shared browser
- CLI-156: Signing out leaves nothing behind
- CLI-159: Outside links say where they go

References that ship with their owning rows: CLI-157 (see ACC-117).

### Accounts, sharing and security (50 features, plus 2 references)

- ACC-001: Claim a new server with a one-time setup code
- ACC-002: Owner account with no vendor account
- ACC-003: Sign-in and playback with no internet
- ACC-004: Owner recovery from the host
- ACC-005: Hand ownership to another person
- ACC-006: Local user accounts
- ACC-007: No user list before sign-in
- ACC-008: Disable an account without deleting it
- ACC-009: Deletion with a grace period
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
- ACC-065: Sign out everywhere when a credential changes
- ACC-068: Your devices, in one list
- ACC-069: Revoke one device
- ACC-070: Sign out of all sessions
- ACC-071: New-device alerts
- ACC-075: Stream limits that count playing, not browsing
- ACC-076: Device limits and allow-lists
- ACC-079: Session lifetimes
- ACC-080: Invite by link or QR code
- ACC-097: Works behind your own reverse proxy or VPN
- ACC-098: HTTPS served by the server
- ACC-099: Automatic certificates for your own domain
- ACC-113: Nothing leaves the house by default
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
- ACC-126: An advisory for every fix
- ACC-128: Nobody can switch your server off remotely
- ACC-136: Delete your own account
- ACC-137: Recovery codes
- ACC-138: Recovery hold
- ACC-139: Your own security log

References that ship with their owning rows: ACC-078 (see ADM-110), ACC-127 (see ADM-054).

### Setup, administration and operations (51 features, plus 7 references)

- ADM-001: Single self-contained binary
- ADM-002: Official container image
- ADM-003: Container tags that pin a version
- ADM-005: Install as an OS service with one command
- ADM-006: Refuse to run as root
- ADM-007: Configuration that is checked, not guessed
- ADM-020: Setup closes for good once an admin exists
- ADM-021: A secure context for passkeys at setup
- ADM-022: Built-in HTTPS for a domain you own
- ADM-025: Add music folders with live checks
- ADM-028: Privacy choices at setup
- ADM-029: Restore from backup on the welcome screen
- ADM-032: Startup and migration status page
- ADM-052: Hand the server to a new owner
- ADM-053: Update check from a signed feed, asked at setup
- ADM-054: Security advisory banner
- ADM-056: Automatic snapshot before every upgrade
- ADM-057: Check migrations before serving
- ADM-058: Upgrade straight from any older version
- ADM-059: Roll back by starting the previous version
- ADM-060: Rollback safety stated for every release
- ADM-065: Daily backups on by default
- ADM-066: Backup contents shown plainly
- ADM-068: Encrypted backups
- ADM-069: Download a backup, upload it elsewhere
- ADM-071: Restore from the command line
- ADM-072: Backups verified after they are written
- ADM-077: Rebuild the cache instead of repairing it
- ADM-078: Checksummed user log that recovers from a torn write
- ADM-079: Refuse a data directory on a network filesystem
- ADM-080: One writer, so no "database is locked"
- ADM-083: Free-space guard
- ADM-089: Media mounted read-only
- ADM-090: Separate locations for settings, log, cache and scratch space
- ADM-095: Heavy jobs that are throttled and resumable
- ADM-108: Health of each library root
- ADM-110: Activity and audit log
- ADM-111: Retention and anonymisation of activity
- ADM-116: Admin alerts with free destinations
- ADM-119: Structured logs with rotation
- ADM-122: Sign-in failure lines for fail2ban
- ADM-123: `gunmetal doctor`
- ADM-128: Health endpoints for containers
- ADM-129: Network activity page
- ADM-142: Security summary on the admin home
- ADM-143: Recovery kit at setup
- ADM-144: Rotate every server key in one action
- ADM-145: Audit log verification and an anchor off the server
- ADM-146: Outbound proxy and offline mode
- ADM-147: Configuration changes made outside the server are reported
- ADM-148: Compromise runbook

References that ship with their owning rows: ADM-018 (see ACC-001), ADM-019 (see ACC-002), ADM-031 (see LIB-021), ADM-034 (see ACC-004), ADM-085 (see LIB-032), ADM-086 (see LIB-033), ADM-121 (see ACC-123).

### Integrations and extensibility (8 features)

- INT-001: API reference generated from code
- INT-005: Capability discovery
- INT-007: Consistent list endpoints
- INT-008: IDs that survive rebuilds and file replacement
- INT-009: MusicBrainz IDs on music items
- INT-013: Health check for uptime monitors
- INT-023: Credentials in headers only
- INT-151: Documented history export

### Adjacent media types (4 features)

- LAT-001: Item kinds in the data model
- LAT-006: Typed positions in the user log
- LAT-007: All user-made data in the user log
- LAT-009: Listening contexts in the queue protocol

### After R1: the point releases

The rest of the earlier R1 ships in three point releases, in this order.
The register's [Point releases after R1](../decisions.md#point-releases-after-r1)
names every row. The IDs below are generated from the maps' Release cells.
The register counts two of the references below, MUS-151 (R1.2) and DIS-106
(R1.3), among its features, so its feature counts for those two releases
are one higher than these; the totals agree.

**R1.1, bring your music in** (79 rows: 68 owning rows and 11 references)

- Music: MUS-005, MUS-008, MUS-010, MUS-013, MUS-019, MUS-024, MUS-047, MUS-053, MUS-055, MUS-056, MUS-072, MUS-076, MUS-090, MUS-114, MUS-120, MUS-125, MUS-127, MUS-128, MUS-134, MUS-135, MUS-137, MUS-139, MUS-140, MUS-156, MUS-158, MUS-181; references MUS-057 (see DIS-107), MUS-058 (see DIS-101), MUS-063 (see DIS-110), MUS-189 (see ADM-042).
- Library and metadata: LIB-006, LIB-025, LIB-031, LIB-034, LIB-098, LIB-111, LIB-112, LIB-187, LIB-192, LIB-194; references LIB-043 (see MUS-005), LIB-047 (see MUS-008), LIB-048 (see MUS-010), LIB-052 (see MUS-013), LIB-054 (see MUS-019).
- Discovery, home and search: DIS-019, DIS-022, DIS-023, DIS-047, DIS-071, DIS-086, DIS-087, DIS-088, DIS-089, DIS-091, DIS-101, DIS-102, DIS-103, DIS-105, DIS-107, DIS-110; references DIS-163 (see CLI-040).
- Clients, devices and offline: CLI-003, CLI-024, CLI-025, CLI-026, CLI-030, CLI-040, CLI-062, CLI-099, CLI-103, CLI-151.
- Accounts, sharing and security: ACC-011.
- Setup, administration and operations: ADM-030, ADM-042, ADM-043, ADM-044, ADM-051.
- Integrations and extensibility: references INT-107 (see ADM-042).

**R1.2, the household and the admin** (43 rows: 37 owning rows and 6 references)

- Music: MUS-049, MUS-235; references MUS-151 (see ACC-086), MUS-190 (see ADM-099).
- Library and metadata: references LIB-195 (see ADM-125).
- Discovery, home and search: DIS-003, DIS-007, DIS-009, DIS-012, DIS-013, DIS-015; references DIS-173 (see CLI-034).
- Clients, devices and offline: CLI-033, CLI-034, CLI-146.
- Accounts, sharing and security: ACC-040, ACC-057, ACC-086, ACC-087, ACC-088, ACC-089, ACC-116, ACC-134; references ACC-072 (see ADM-099), ACC-073 (see ADM-102).
- Setup, administration and operations: ADM-010, ADM-011, ADM-027, ADM-070, ADM-074, ADM-093, ADM-099, ADM-100, ADM-102, ADM-109, ADM-112, ADM-113, ADM-124, ADM-125, ADM-130, ADM-140.
- Integrations and extensibility: INT-134, INT-147.

**R1.3, discovery and analysis** (32 rows: 28 owning rows and 4 references)

- Music: MUS-007, MUS-086, MUS-129, MUS-143, MUS-144, MUS-145, MUS-146, MUS-165; references MUS-028 (see LIB-008).
- Library and metadata: LIB-008, LIB-024, LIB-058, LIB-099, LIB-179; references LIB-041 (see MUS-007), LIB-066 (see MUS-086).
- Discovery, home and search: DIS-060, DIS-061, DIS-062, DIS-067, DIS-070, DIS-119, DIS-120, DIS-121, DIS-122; references DIS-106 (see LIB-008).
- Setup, administration and operations: ADM-004, ADM-088, ADM-141.
- Adjacent media types: LAT-002, LAT-008, LAT-010.

## Differentiators

These are the features across the whole product most likely to make
someone switch, with the pain each one answers. Where a rival is already
good, the entry says so.

1. **Play the original, on hardware sized for reading disks** (MUS-066,
   VID-001, VID-003, ADM-001; R1 for music, R2 for video). Pain: Plex and
   Jellyfin are sized for transcoding; needless transcoding is the
   sixth-ranked pain theme, and Plex's "default to max quality" request has
   1,289 votes. Native clients play the file as it is, and the remuxer
   replaces the transcode for browsers and TVs. Parity with mpv-based
   players, Kodi and Infuse on native direct play; the edge is that every
   first-party client defaults to the original and never burns in
   subtitles.
2. **Gapless, levelled music in the browser** (MUS-067, MUS-230, MUS-087;
   R1, with measured loudness, MUS-086, in R1.3). Pain: gapless is
   Jellyfin's most-voted music request (647 votes, open since 2019), and
   Navidrome's web player will not do it. Plexamp already does this well,
   but only in its own apps. The packager runs in a worker process and
   streams segments over a pipe. From R1.3, loudness is measured by a
   reviewed pure-Rust decoder in the scan worker. Either way no untrusted
   audio is parsed in the server process (SEC-MED-018, SEC-MED-026,
   SEC-MED-081). R1 levels from loudness tags, with the fallback gain for
   untagged tracks (MUS-089). Risks: the packager's per-browser support is
   unverified, and measured loudness needs ADR 5 (open decision 8).
3. **Instant browsing and search from a library synced to the device**
   (CLI-022, DIS-084, DIS-002, MUS-208; R1). Pain: Jellyfin's lazy-loading
   request has 1,179 votes and its search regressed repeatedly in 2025.
   Every list and search answers locally, over HTTPS or on localhost
   (CLI-150). From R1.1 the app also loads and keeps working when the
   server is down (CLI-025).
4. **Free offline downloads that manage themselves** (CLI-078, CLI-080,
   CLI-081; R2). Pain: offline is Jellyfin's most-voted request (1,820
   votes); Plex and Emby charge for downloads, and Plexamp only removed its
   download cap in September 2026. Downloads follow rules (keep the next
   episodes, follow a playlist) under a storage cap.
5. **Remote access with no open port, no account and no fee** (ACC-096,
   ACC-080; R2). Pain: Plex has charged for remote personal video since
   April 2025; Jellyfin leaves families to VPNs and reverse proxies. Native
   apps embed iroh. Honest limits: iroh's quoted direct-connection rate is
   unverified on carriers and TVs, the fee question depends on the relay
   decision, and browsers reach the server from outside only through the
   owner's proxy or a tailnet until browser access without a domain
   (ACC-102, R2) ships.
6. **Sign-in with nothing to steal, and revocation that really stops
   playback** (ACC-050, ACC-062, ACC-120 to ACC-122; R1, with OIDC,
   ACC-057, in R1.2). Pain: single sign-on (1,191 votes) and two-factor
   (1,103) are Jellyfin's most-voted security requests; plex.tv has been
   breached twice; the rivals' 2026 advisories are mostly authorisation
   mistakes. Passkeys everywhere, over real HTTPS from the owner's own
   domain, a tailnet or the machine itself (the per-server name joins them
   in R2); a browser that cannot use a passkey is approved from your phone
   and gets a session that can play but never administer (ACC-062); from
   R1.2, OIDC for households that run an identity provider. There are
   no passwords and no authenticator codes, so there is nothing to stuff,
   phish or leak (SEC-IAM-025); a passkey already proves both the device
   and the face or fingerprint that unlocks it, which is more than
   Plex's two-factor. Per-object authorisation is tested on every route,
   and stream URLs have their in-flight responses cut on revocation.
7. **History and fixes that survive every move, upgrade and rebuild**
   (LIB-028, LIB-029, ADM-059; R1, with LIB-179 and ADM-141 in R1.3).
   Pain: the "Plex
   Dance"; Jellyfin 10.11 lost watched state on file replacement (#15001);
   PlexDBRepair exists because databases hold irreplaceable data. Gunmetal
   keys everything to content identity, keeps user data in the user log,
   and rebuilds its cache instead of repairing it. Depends on ADR 3.
8. **Tiny daily backups and upgrades that can be undone** (ADM-065,
   ADM-056 to ADM-059; R1). Pain: Plex backs up only its core database
   every three days; Jellyfin's 10.11 migrations failed for many; Emby
   charges for backups. A backup is the user log, settings and keys, so it
   can run daily by default; an older binary rolls back by rebuilding.
9. **A music model that gets credits, editions and discs right** (MUS-001,
   MUS-012; R1, with roles, editions and release types, MUS-005, MUS-008
   and MUS-010, in R1.1). Pain: Plex has refused multiple
   artists since 2015 (520 votes, plus 866 for robust tags), and Jellyfin
   regressed in 10.11. Navidrome is already good here, so this beats
   Navidrome only together with Gunmetal's own player.
10. **Smart playlists and home rows from one free rule language** (MUS-143,
    MUS-144, DIS-119 to DIS-121; R1.3). Pain: Jellyfin has no smart
    playlists (588 votes) and Navidrome makes people write JSON. Plexamp's
    are good. The same rules drive home rows and, in R2, download rules.
11. **Lyrics that are simply there, free** (MUS-154, MUS-155; R1, with
    word-timed lyrics and lyrics that stay open, MUS-156 and MUS-158, in
    R1.1). Pain: Plexamp charges for lyrics and Plex users want them shown
    automatically (260 votes). Embedded, LRC and word-timed lyrics are read
    at scan time.
12. **Your own record, under your control** (MUS-184, DIS-050, ACC-117,
    ACC-010; R1, with dismiss and undo, DIS-022 and DIS-023, in R1.1).
    Pain: removing a play has 5,458 Spotify
    votes and browsing history by date 2,035; Jellyfin's dismiss-with-undo
    request has 1,725. Remove plays, browse history by date, private
    sessions (no media server we checked documents them, unverified) and a
    documented export.
13. **Subtitles that never force a transcode** (VID-069, VID-070, VID-071;
    R2). Pain: Plex's Android TV player cannot render ASS, so users choose
    between a burn-in and losing the styling; subtitle drift is the
    most-reacted open issue on Jellyfin's server. Parity with Jellyfin web
    in browsers; the edge is the native TV clients.
14. **Pre-made smaller versions for remote and mobile play** (VID-026; R2,
    opt-in). Pain: 979 Jellyfin votes, and Jellyfin declined it in November
    2025. Made at idle in the sandbox, so slowness on a small server does
    not matter.
15. **Antenna TV without a transcoding server** (LIV-078, LIV-024; R3).
    Pain: Plex's own documentation says Live TV needs a server that can
    transcode, because broadcast MPEG-2 usually has to be converted. Native
    clients decode the broadcast themselves. Browser live TV still costs
    CPU, and the docs must say so.

## Deliberately not doing

Every **No** row in the maps is listed here once, grouped by the reason.

- **Anything that needs a central account or vendor cloud.** Built-in
  "Continue with Google or Apple" buttons (ACC-067), trending across
  servers (DIS-180), voice-assistant skills run by the project (CLI-131,
  which owns the decision; DIS-181 and INT-146 point to it), and server-side
  Discord presence that stores user tokens (INT-150). The project will also
  never hold a kill switch over anyone's server (ACC-128 is the R1 promise)
  or email owners, because it has no accounts. Casting to Nest and Google
  speakers (CLI-106, R2) is the partial answer to the 2,461-vote Google Home
  demand.
- **A content catalogue of any kind.** Streaming-service content and "where
  to watch" (DIS-176), streaming content mixed into the library or radio
  (LIB-124, MUS-178), bundled guides, channels or playlists (LIV-049,
  LIV-174), and suggestions inserted into a person's own playlists (DIS-182,
  MUS-130). Gunmetal plays what the household owns.
- **Writing into media folders.** Writing tags, NFO files or artwork
  (LIB-133, MUS-048), a renamer (LIB-085), downloading trailers into media
  folders (LIB-162), baking overlays into posters (LIB-147, INT-143),
  merge-and-embed tools for books (LAT-046), and copying or syncing media
  between drives or servers (ADM-092, LAT-112). The server never creates,
  changes or deletes anything under a media root, with no exception
  (SEC-TM-042). Removing an item from the app (ADM-139, Later) hides it
  and puts it in a trash with undo and audit; the file stays on disk, and
  the owner deletes it on the host.
- **Passwords and authenticator codes.** Password sign-in as a fallback
  (ACC-052) and two-factor codes (ACC-053): there are no passwords, so a
  code would protect nothing, and both can be stuffed, phished or sniffed.
  Passkeys, OIDC and approval from a signed-in device replace them
  (SEC-IAM-025).
- **Sign-in delegated to something that is not an identity provider.**
  LDAP directory sign-in (ACC-059) and sign-in from a trusted
  reverse-proxy header (ACC-060): a proxy's identity header is never
  authentication, and a directory belongs behind an OIDC provider. Either
  would need its own architecture record first (SEC-NET-023).
- **Protocols and shortcuts that undo per-object authorisation.**
  Passwordless sign-in on the local network (ACC-066), automatic port
  forwarding (ACC-106), a DLNA server in the core (CLI-115, VID-152),
  HDHomeRun emulation, which needs unauthenticated plain-HTTP endpoints and
  LAN discovery (LIV-172; SEC-NET-001), Plex API compatibility (INT-101),
  in-process native plugins (INT-073), plugins that inject scripts into
  the web client (INT-074), scripts run inside the
  server (LIV-145), and Stalker portals and redirect mode for IPTV (LIV-021,
  LIV-022).
- **DRM circumvention and legally exposed data.** DRM-protected audiobooks
  and ebooks (LAT-052, LAT-163), DRM-protected broadcasts and descrambling
  (LIV-033, LIV-034), first-party chapter lookup from Audible data
  (LAT-020), and clipping and sharing scenes from commercial films
  (VID-109).
- **Heavy server stacks that break the low-hardware promise.** External
  database engines (ADM-084), server-side machine learning (LAT-076,
  LAT-138, LAT-139, VID-167), phone photo backup (LAT-137), a photo editing
  suite (LAT-140), server-side PDF rendering (LAT-161), edge or caching
  servers (ADM-138), driving DVB cards directly (LIV-032), composing Dolby
  Vision profile 7's enhancement layer (VID-042), full disc menus (VID-021),
  beat-matched DJ transitions (MUS-093) and karaoke vocal reduction
  (MUS-164).
- **Products that belong to someone else.** A built-in request manager
  (INT-131, Seerr does it), built-in download automation (INT-132), Xtream
  video-on-demand catalogues and TV Everywhere logins (LIV-023, LIV-175), a
  vertical clip feed (DIS-179), artwork chosen per person (DIS-178), an
  ebook reader on TV (LAT-162), critic and user reviews (LIB-125), disc
  folders and images (LIB-086), tour dates (MUS-179), and cutting
  commercials out of recordings (LIV-152, markers instead).
- **Platforms with weak demand or no viable path.** PlayStation and
  Nintendo Switch (CLI-019), native Linux phone builds (CLI-020, the web app
  covers them), a first-party Kodi add-on (CLI-021), video in the car
  (CLI-126), and CableCARD and QAM cable tuners (live-tv.md, Deliberately
  not doing).
- **Social and tracking features.** A public social layer (DIS-177), a
  friends' listening feed (MUS-191), usage telemetry on by default
  (ADM-136), and crash reports sent to the project, even opt-in (ADM-131):
  a crash record goes into the diagnostic bundle (ADM-124), which the admin
  may attach to a report themselves (SEC-PRV-009).
- **Paywalls and vendor caps.** No paid tier, unlock or device cap for any
  feature, in any area (ADR 1, decision 10).

## Open decisions for the project owner

These are the decisions that cross areas or shape the release plan,
de-duplicated from the area files. Each area file keeps its own list for
decisions that affect only that area; where an area decision is settled
here, the area file says so. The [decision register](../decisions.md) is
now the single list of the owner's decisions and cites each item below by
number. Where a recommendation here was written before the security
baseline, the item now gives the register's recommendation and its ID;
"owner to confirm" marks an item that rests on one of the baseline's open
owner decisions, applied as the baseline recommends.

1. **ADR 3: durable user state.** ADR 1 makes only watch history
   irreplaceable, but most R1 rows that write user data depend on more
   (accounts 4, admin 1, discovery 1, library 1, music 1, live TV 4).
   *Recommendation:* write and accept ADR 3 before any server code that
   stores user data. It should define the user log (one append-only,
   exportable format with typed, versioned streams, each entry scoped to a
   profile or the household) and the identity store, with one backup,
   export and replay path for both. Every map now uses these two names.
2. **Update the root README roadmap to the release key.** *Recommendation:*
   reorder the roadmap so iroh remote access and the OpenSubsonic adapter
   sit inside the video milestone (R2), and Apple builds, Samsung and LG
   packaging and the Jellyfin video adapter sit under Later. This review
   did not edit the root README, because it is outside the feature map.
3. **App Store distribution and the licence** (clients 2, video 4). There
   is no contributor licence agreement, so an app-store permission cannot be
   added once outside contributions arrive. *Recommendation:* get legal
   advice now; if it supports an additional permission for app-store
   distribution, add it before the first outside contribution, and then
   move the Apple rows from Later to R2.
4. **When the OpenSubsonic adapter ships** (integrations 1, accounts 12,
   clients 1). The integrations and clients maps argued for R1, because R1
   phone users get only the web app. *Decided here:* R2, off by default,
   API keys only. Reasons: R1 is already large, the adapter doubles the
   authorisation surface, and which Subsonic apps support API-key sign-in is
   unverified. *Recommendation:* keep R2, and state the R1 phone limits
   plainly (CLI-003). Older Subsonic apps get INT-088 as it now reads, in
   R2 with the adapter: a per-app legacy key the person marks as legacy,
   off by default, accepted only on home-network paths, valid for 90 days
   by default, never an account password, with token-and-salt refused
   (SEC-EXT-069). See register D-54 and D-55; owner to confirm (security
   README decision 8).
5. **Ship the native Android music app early, as an R1 point release**
   (clients 1). Background audio, lock screen, downloads and Android Auto
   do not depend on the remuxer, and the app is the best answer to the R1
   phone gap. *Settled in part by register D-10:* the adopted R1 scope
   keeps the app out of R1.x, because it would bring R2 requirements for
   device keys and native storage with it. *Recommendation (register
   D-14):* if it ships early, schedule it after R1.3, with those
   requirements (SEC-IAM-048, SEC-CLI-030, SEC-CLI-036, SEC-CLI-046,
   SEC-STD-039) moved with it in the release-scope table.
6. **When the plugin host and scrobblers ship** (integrations 2, music 5).
   *Decided here:* R2. A plugin host with grants, signing, an egress gate
   and per-user secrets is a large R1 item. *Recommendation (register
   D-56):* keep R2, with each plugin in its own OS-sandboxed process and no
   plugin runtime linked in R1. Scrobblers send only plays made after the
   person links a service, or a backfill range they pick, so plays recorded
   in R1 are never sent unasked (SEC-PRV-033).
7. **Sign-in without passwords, and HTTPS names** (ADM-023; accounts 2,
   admin 2). Without a secure context, browsers cannot use passkeys.
   *Settled by register D-06 and D-07:* there are no passwords and no
   authenticator codes anywhere (ACC-052 and ACC-053 are No,
   SEC-IAM-025). A browser that cannot use a passkey is approved from a
   signed-in device and gets a limited session that can play but never
   administer (ACC-062, SEC-IAM-108). Plain HTTP gives every peer but
   loopback a help page and nothing else (SEC-NET-001). R1 gets HTTPS
   through the owner's own domain with automatic certificates (ADM-022,
   ACC-099), a tailnet name, or the machine itself (CLI-150), and remote
   use goes through the owner's reverse proxy or a tailnet (ACC-097). The
   project-run per-server name service (ADM-023), its naming client and
   its certificate-transparency monitoring are R2, with built-in remote
   access. The name service still launches only once its zone is on the
   Public Suffix List (SEC-NET-070), and it needs an ADR amending ADR 1
   decisions 7 and 9.
8. **Audio decoders in the scan worker** (MUS-086; music 6, library 5,
   later media 12). Scan-time loudness needs a full decode with a
   third-party crate. *Recommendation (register D-09):* ADR 5 admits one
   reviewed pure-Rust decoder (the candidate is Symphonia), with its
   licence, fuzzing and resource limits, and it runs only in the scan
   worker, never in the server process (SEC-MED-018, SEC-MED-024,
   SEC-MED-026). R1 uses loudness tags plus the fallback gain (MUS-089).
   Measured loudness (MUS-086) ships in R1.3 if ADR 5 is accepted (register
   D-10). Owner to confirm (security README decision 9).
9. **Amend ADR 2's "no remuxer for music"** (MUS-230, MUS-041).
   Browser gapless and CUE sheets need audio frames repackaged into
   fragmented MP4 or re-headed FLAC slices. *Recommendation (register
   D-09):* amend ADR 2 through ADR 4 to name a light, audio-only packager
   as the exception, and record that it and the CUE slicer run in a worker
   process that streams over a pipe under the step budget, a memory cap
   and a watchdog, never in the server process (SEC-MED-018,
   SEC-MED-081). The server revalidates every segment it gets back
   (SEC-MED-023). Owner to confirm (security README decision 9).
10. **Who runs and pays for iroh relays** (accounts 3). *Recommendation:*
    make self-hosted relays first-class in R2 (ACC-100), and run one modest
    project relay only if it is funded, with a published policy on what
    metadata it logs and for how long.
11. **A licence exception for third-party WebAssembly plugins**
    (integrations 4). *Recommendation:* get legal advice now and, if
    permissive third-party plugins are wanted, add the exception before
    outside contributions build up. Keep first-party plugins AGPL.
12. **Rating model and what can be rated** (music 3, discovery 12,
    MUS-181). *Recommendation:* loves everywhere by default; five stars
    with half steps, switched on automatically when ratings are imported;
    tracks, albums and artists rated directly, never derived; strictly per
    person.
13. **What admins can see of other people's activity** (accounts 5, admin
    7, discovery 6). *Recommendation (register D-34):* admins see who is
    playing, on which device, the bitrate and the playback method, and
    totals; the title only when that person opts in (MUS-235); never
    anyone's history, ratings or private playlists, and no server setting
    widens this (SEC-PRV-025, SEC-TM-054). Each admin look is recorded in
    the person's own security log (SEC-IAM-077), and every person can see
    what the admin sees (ACC-115, R1). Only a managed profile's named
    guardians see its history (ACC-034, R2). The owner accepted this on
    2026-10-02 (register, "Owner answers"). The admin's live view and the
    title opt-in arrive in R1.2 (ADM-099, MUS-235); R1 has no live view.
14. **Private listening default** (accounts 6, discovery 6).
    *Recommendation:* off by default, one tap away from the player, and
    clearly shown while on.
15. **Telemetry** (accounts 7, admin 4). *Recommendation (register
    D-38):* no telemetry and no crash sending to the project in R1 and
    R2, opt-in or not (ADM-131 is No). A crash record goes into the
    diagnostic bundle (ADM-124, from R1.2), shown in full and
    pseudonymised, which the admin may attach to a report themselves.
    Project services hold only the name service's routing data, from R2
    (SEC-PRV-009, SEC-TM-053, SEC-HIS-061).
    Owner to confirm (security README decision 24).
16. **Performance budgets** (DIS-019). The rows now carry proposed design
    goals: home in under 200 ms and browse or search in under 50 ms at
    100,000 tracks on the reference low-end device, initial sync of 100,000
    tracks in under 2 minutes, and time to interactive under 3 seconds on
    the cheapest supported stick. *Recommendation:* name the reference
    devices, accept these as goals, and publish the measured numbers either
    way, as the README promises.
17. **Media read-only, with no exception** (music 9, library 2, later
    media 3, ADM-139). *Recommendation (register D-43):* the server never
    writes to a media root in R1 or R2 (SEC-TM-042). "Remove" (ADM-139,
    Later) hides and trashes the item with undo and audit, and the owner
    deletes files on the host with their own tools; tags and ratings are
    never written back; recordings, podcast archives and trailers go to the
    server's own stores. Owner to confirm (security README decision 23).
18. **Watch together** (video 6, ACC-135). It is the second most-voted Plex
    suggestion (2,878 votes). *Recommendation:* ship the guest capability
    in R2 (ACC-135) and make watch together the first feature after R2.
19. **Dolby Vision on DV televisions** (video 4 and 15, VID-184).
    libmpv tone maps on the client and never sends a DV signal.
    *Recommendation:* spike the Android TV platform decoder early in R2; if
    one player module can host both engines, move VID-184 to R2. On Apple,
    plan for Apple's player for DV and AirPlay once Apple builds exist.
20. **Transcoding scope in R2** (video 1 and 3, VID-006, VID-185).
    *Recommendation:* software transcoding in the sandbox in R2, off by
    default for everyone but the owner; hardware transcoding and HEVC or
    AV1 output Later, after a written isolation design for GPU access. Say
    plainly that until then a small server may manage one 1080p transcode
    or none.
21. **The Linux desktop shell** (clients 10, CLI-014). React Native has no
    first-party Linux desktop target (unverified). The shell is Later, as
    the baseline's release-scope table says (SEC-TM-074; register D-10,
    D-15). *Recommendation:* prototype the shell (the client security file
    recommends Tauri 2 with mpv in a separate sandboxed process) before
    promising Linux; until then desktops use the web client.
22. **One sign-in across several servers** (accounts 16, ACC-133).
    *Recommendation:* design it as a device key trusted by several
    servers, review it in its own record, and keep it Later.
23. **Order of the modules after video** (live TV 1, later media 1).
    *Recommendation:* keep ADR 2's order: live TV in R3, then books and the
    other media types. Do not start R3 until the remuxer and sandbox are
    stable. Bringing audiobooks forward would need an ADR.
24. **An interim answer for podcasts** (later media 16). *Recommendation:*
    document running Audiobookshelf beside Gunmetal until the podcast plugin
    exists; OpenSubsonic podcast endpoints come later.
25. **Release signing keys** (admin 12). *Recommendation:* an offline root
    key held by at least two maintainers, short-lived online signing keys,
    and a published rotation and compromise procedure before R1 ships.

## Review log

This review applied a set of cross-map review issues to all ten files. The
outcomes that a reader of a single file might find surprising are recorded
here.

- **Applied in another form.** The OpenSubsonic adapter, the plugin host and
  the scrobblers went to R2 (two issues recommended R1). The household and
  parental rows went to R2 rather than pulling discovery's copies into R1.
  In-app metadata editing (LIB-172 to LIB-178), genre clean-up, charts,
  mixes, and hide and snooze went to R2, and the music rows point at those
  owners. Remote volume (CLI-102) and handoff went to R2 with the native
  clients. "Search inside settings" (CLI-152) was added at R2 rather than
  R1, because R1 has few settings. Hardware-dependent HEVC and AV1 output
  became one Later row (VID-185). Duplicates (LIB-196) went to R2 with a
  tag-and-MusicBrainz-ID rule.
- **Partly applied.** The proposed R1 floor of 120 to 150 rows: R1 went from
  about 600 rows to 422 (455 after the security alignment), but an "R1.x"
  release value was not introduced, so that every file keeps the same five
  values; see "The R1 cut" for why the floor is larger, and register D-10
  for the point-release proposal. ACC-052 (password fallback) and ACC-053
  (two-factor) are now No: there are no passwords, so there is nothing for
  a code to protect (SEC-IAM-025; owner to confirm, security README
  decision 1). Loudness measurement (MUS-086) stayed in R1 with a stated
  dependency rather than moving to a point release.
- **Rejected.** Moving folder view to R2: it stays R1 (LIB-008 owns it),
  because paths are already in the synced library and Navidrome users rely
  on it. Moving ADM-052 (hand the server to a new owner) to Later: its
  method, adding a second admin and removing yourself, is trivially R1;
  ACC-005 stays Later and now says what it adds. Setting the music rows to
  R1 wherever a sibling map had R1: the owning rows were moved to R2
  instead where R1 had to shrink. Moving the discovery parental rows to R1:
  the accounts rows moved to R2 and the explicit-filter claim was removed
  from Accounts Differentiator 4. Aligning the scrobbler rows on R1.
- **Ownership split for the music model.** One issue suggested that
  library.md own tag reading. The split chosen is that music.md owns the
  music model and music tag interpretation, and library.md owns scanning,
  change detection, file identity and health, so the music backend has one
  place to read.
- **Realigned to the owner's answers, 2026-10-03.** The owner adopted the
  register's smaller R1 with point releases (D-10) and moved the name
  service to R2 (D-07). That supersedes three outcomes above. The R1.x
  release values now exist, and every file uses the eight values in
  "Releases". Loudness measurement (MUS-086) is R1.3. Folder view
  (LIB-008) is R1.3. The summary, "The R1 cut" and the point-release
  lists were regenerated from the maps' Release cells.
