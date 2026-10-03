# Feature map

This folder maps every feature Gunmetal might build, area by area, so that
the screens the UI needs and the server work behind them can be read off
one place. It was written on 2026-10-02 from the research in
`docs/research/` and stays inside the two accepted architecture records:
[ADR 1](../adr/0001-architecture.md) (architecture) and
[ADR 2](../adr/0002-music-is-first-class.md) (music is first-class). This
README is the shared key for all ten area files. Where an area file and
this README disagree about a release, this README wins and the area file is
a bug.

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

Every feature table in every area file has the same nine columns.

| Column | Meaning |
|---|---|
| **ID** | A stable identifier (see below). |
| **Feature** | A short name for the feature. |
| **What the user gets** | The outcome in the user's words. For a reference row, it names the owning row. |
| **Rivals today** | What Plex, Jellyfin, Emby, Navidrome and the specialist apps do, from the research. "(unverified)" marks a claim the research could not confirm. |
| **Demand** | The evidence of demand, on the scale below. |
| **Release** | One of the five release values below. |
| **How Gunmetal does it better** | The edge, or an honest "parity" or "behind" where a rival is already good. |
| **Server needs** | What the server (or the shared Rust core) must provide. "None beyond X" means the owning row X covers it. |
| **UI surfaces** | The screens or controls the feature appears on, named so the UI plan can collect them. |

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

The Release cell holds exactly one of these five values.

| Release | What it contains | Clients | Infrastructure |
|---|---|---|---|
| **R1** (music) | The first usable release: the server, the web client and the music library and player. Sign-in, backups and the security baseline. | The web client and the installable web app only (browsers on desktop and phone). | No remuxer and no transcoder (ADR 2, decision 2); one light audio-only packager for browser gapless (MUS-230). No plugin host. No iroh: remote use means the owner's reverse proxy, VPN or domain (ACC-097, ACC-134). |
| **R2** (video) | Movies and TV, plus everything that needs a native app or a sandbox: downloads, handoff and remote control, casting, the plugin host and the scrobblers, the OpenSubsonic adapter, household profiles and parental controls, share links, webhooks, Opus streams. | Android phones and tablets, Android TV and Google TV (and Fire OS), the desktop shell, and the web client. | The pure-Rust remuxer, the sandboxed transcoder (software encoders), iroh remote access for native clients, the WebAssembly plugin host. |
| **R3** (live) | M3U playlists and live TV: sources, guide, lineup, live playback, recording. | As R2. | The live TV ingest and recorder. |
| **Later** | Wanted, not scheduled. Includes the Apple platforms, Samsung and LG TVs, Roku through the Jellyfin adapter's video subset, hardware transcoding, audiobooks and the other adjacent media types, and watch together. | | |
| **No** | Deliberately not doing. The row says why. | | |

Two notes apply across all files:

- **Apple platforms** (iPhone, iPad, Apple TV, CarPlay, AirPlay, Apple
  Watch) are **Later**, with the note "R2 if the App Store licence decision
  allows" (open decision 3). The App Store terms and the AGPL with no
  contributor agreement are the blocker, not engineering.
- **Feature release versus client reach.** A feature is placed in the
  first release in which it ships on at least one client. An R1 feature
  reaches the native apps when they ship in R2; the UI surfaces column says
  where.

How this lines up with the README roadmap: R1 is the roadmap's server and
web client milestones; R2 is "Mobile and Android TV", "Video", the iroh
part of "Remote access", and the OpenSubsonic part of "Adapters"; R3 is
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

Counts are the real rows in each file's feature tables, taken after this
review. Reference rows are counted in their own area and release, and the
last column says how many of each area's rows are references.

| Area | File | R1 | R2 | R3 | Later | No | Total | Of which references |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| Music | [music.md](music.md) | 111 | 79 | 1 | 34 | 7 | 232 | 48 |
| Library and metadata | [library.md](library.md) | 72 | 95 | 1 | 28 | 7 | 203 | 35 |
| Discovery, home and search | [discovery.md](discovery.md) | 58 | 89 | 3 | 28 | 7 | 185 | 32 |
| Clients, devices and offline | [clients.md](clients.md) | 30 | 84 | 2 | 32 | 6 | 154 | 1 |
| Accounts, sharing and security | [accounts.md](accounts.md) | 49 | 57 | 2 | 24 | 3 | 135 | 7 |
| Setup, administration and operations | [admin.md](admin.md) | 73 | 52 | 1 | 11 | 4 | 141 | 7 |
| Integrations and extensibility | [integrations.md](integrations.md) | 22 | 91 | 3 | 35 | 8 | 159 | 5 |
| Adjacent media types | [later-media.md](later-media.md) | 7 | 13 | 0 | 143 | 12 | 175 | 2 |
| Video playback | [video.md](video.md) | 0 | 136 | 1 | 43 | 5 | 185 | 10 |
| Live TV and recording | [live-tv.md](live-tv.md) | 0 | 0 | 140 | 25 | 11 | 176 | 0 |
| **All areas** | | **422** | **696** | **154** | **403** | **70** | **1745** | **147** |

## The R1 cut

This list is the definition of the first release. R1 holds 422 rows,
of which 369 are owning rows and 53 are references to them. Before
this review the maps put about 600 rows in R1; the review moved the
household and parental system, share links, downloads, handoff and remote
control, the plugin host and scrobblers, the OpenSubsonic adapter, the
webhook and public-API platform, rival-database importers, in-app metadata
editing, most observability screens, and the second tier of discovery rows
to R2.

The floor that remains is larger than the 120 to 150 rows a reviewer
proposed, for a reason worth stating: many R1 rows are rules of one
component rather than separate pieces of work (for example ACC-120 "every
endpoint needs sign-in", ACC-123 "secrets never in URLs or logs", ADM-080
"one writer"), and the research shows those rules are where the rivals'
2026 advisories came from. The build order inside R1 should still be: the
music model and scan, the player (gapless, levelling, the honest
unplayable state), the queue and playlists, lyrics, search and home, then
sign-in, backups and the security baseline.

### Music (100 features, plus 11 references)

- MUS-001: Every credited artist linked
- MUS-002: Display credit kept as tagged
- MUS-003: Album artist separate from track artist
- MUS-004: "Appears on"
- MUS-005: Roles: composer, conductor, lyricist, producer, remixer, performer
- MUS-006: Same-name artists kept apart
- MUS-007: Merge, split and alias artists
- MUS-008: Release groups and editions
- MUS-010: Release types
- MUS-011: Compilations and Various Artists
- MUS-012: Multi-disc albums with disc titles
- MUS-013: Original date versus release date
- MUS-017: Multi-valued genres per track
- MUS-019: Moods, styles, labels and grouping
- MUS-020: Sort names and natural sort
- MUS-021: Technical details
- MUS-024: Artist images from local files
- MUS-027: Several music libraries with per-user access
- MUS-032: Core formats
- MUS-034: Multi-value tags in every tag format
- MUS-035: Separator splitting with exceptions
- MUS-036: MusicBrainz IDs as identity
- MUS-037: Tags win over online data
- MUS-039: Album art from file and folder
- MUS-040: Artwork sized for sync
- MUS-044: Library health report
- MUS-047: Explicit flag
- MUS-049: Music home with sections you arrange
- MUS-050: Continue listening by album or playlist
- MUS-051: Artist page
- MUS-052: All songs by an artist
- MUS-053: Sort and filter a discography
- MUS-054: Album page
- MUS-055: Credits panel
- MUS-056: Library views with remembered sort
- MUS-059: Recently added that ignores upgrades
- MUS-060: Genre, mood and label browse
- MUS-061: Music search fields
- MUS-066: Play the original file
- MUS-067: Gapless in the web client
- MUS-069: Encoder delay and padding honoured
- MUS-070: Next track fetched early
- MUS-071: Instant, exact seeking
- MUS-072: Fades on pause, skip and resume
- MUS-073: Browser media controls
- MUS-076: Sleep timer
- MUS-077: Repeat and stop-after
- MUS-079: Damaged files skipped safely
- MUS-084: ReplayGain and R128 tags, track and album
- MUS-085: Opus gain done right
- MUS-086: Loudness measured for untagged files
- MUS-087: Auto, track and album modes
- MUS-088: Target level with no clipping
- MUS-089: Fallback for unmeasured tracks
- MUS-090: Show the gain applied
- MUS-099: A quality badge that tells the truth
- MUS-108: Persistent now-playing bar
- MUS-109: Love from the bar
- MUS-110: Full-screen player
- MUS-113: Layout contract
- MUS-114: Track info sheet
- MUS-116: Three-lane queue
- MUS-117: Play next keeps your order
- MUS-118: Add to queue and play last
- MUS-119: Edit the whole queue
- MUS-120: Reorder while shuffled
- MUS-122: Persistent queue on every device
- MUS-123: "Playing from" on every item
- MUS-125: Save queue as playlist
- MUS-126: Shuffle modes: random and spread out
- MUS-127: Shuffle by album
- MUS-128: Reshuffle the rest
- MUS-129: Suggestions lane, visible and off by default
- MUS-132: Manual playlists
- MUS-133: Add-to-playlist sheet
- MUS-134: Duplicate warning
- MUS-135: Sort, filter and search inside a playlist
- MUS-137: Automatic playlist covers
- MUS-139: Pin and love playlists
- MUS-140: M3U and M3U8 import and export
- MUS-143: Smart playlists
- MUS-144: Visual rule editor
- MUS-145: Rich rule fields
- MUS-146: Limits, sorts and percentages
- MUS-149: Loved tracks as a playlist
- MUS-154: Embedded lyrics
- MUS-155: Synced .lrc sidecars
- MUS-156: Word-by-word lyrics
- MUS-158: Lyrics stay open
- MUS-165: Library radio from any seed
- MUS-180: Loves
- MUS-181: Star ratings
- MUS-182: Play counts, last played and skips
- MUS-183: Listening history by date
- MUS-184: Remove plays from history
- MUS-188: Export listening history
- MUS-208: Instant browsing from the synced library
- MUS-227: Accessible player
- MUS-229: Honest unplayable state
- MUS-230: Audio packaging for the web player

References that ship with their owning rows: MUS-028 (see LIB-008), MUS-038 (see LIB-028), MUS-042 (see LIB-016), MUS-043 (see LIB-021), MUS-057 (see DIS-107), MUS-058 (see DIS-101), MUS-062 (see DIS-111), MUS-063 (see DIS-110), MUS-185 (see ACC-117), MUS-189 (see ADM-042), MUS-190 (see ADM-099).

### Library and metadata (47 features, plus 25 references)

- LIB-001: Music libraries
- LIB-003: Several folders per library
- LIB-004: Several libraries
- LIB-005: Add a library at first run
- LIB-006: Exclusion rules
- LIB-007: Read-only media
- LIB-008: Folder view
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
- LIB-024: Background analysis that resumes
- LIB-025: Upgrades without full rescans
- LIB-028: Stable identity for every file
- LIB-029: Moves and renames keep everything
- LIB-030: Better copies replace, not duplicate
- LIB-031: Move the server, keep the library
- LIB-032: An offline drive never empties the library
- LIB-033: Trash with a grace period
- LIB-034: Missing files list
- LIB-045: Albums built from tags
- LIB-051: Same-titled albums stay apart
- LIB-058: Merge and split albums by hand
- LIB-059: Every raw tag kept
- LIB-097: Identify without the internet
- LIB-098: Every decision explains itself
- LIB-099: Review queue
- LIB-108: Nothing leaves by default
- LIB-134: Embedded cover art
- LIB-135: Local artwork files
- LIB-136: Predictable artwork order
- LIB-142: Images sized for each device
- LIB-143: Safe image handling
- LIB-146: Quality badges as data
- LIB-179: Fixes survive everything
- LIB-187: One artist page across libraries
- LIB-192: Playlist files in music folders
- LIB-193: Damaged and unreadable files
- LIB-194: Tag problems

References that ship with their owning rows: LIB-036 (see MUS-034), LIB-037 (see MUS-001), LIB-038 (see MUS-035), LIB-039 (see MUS-003), LIB-040 (see MUS-006), LIB-041 (see MUS-007), LIB-042 (see MUS-004), LIB-043 (see MUS-005), LIB-044 (see MUS-020), LIB-046 (see MUS-036), LIB-047 (see MUS-008), LIB-048 (see MUS-010), LIB-049 (see MUS-011), LIB-050 (see MUS-012), LIB-052 (see MUS-013), LIB-053 (see MUS-017), LIB-054 (see MUS-019), LIB-056 (see MUS-014), LIB-063 (see MUS-021), LIB-064 (see MUS-069), LIB-065 (see MUS-084), LIB-066 (see MUS-086), LIB-067 (see MUS-154), LIB-068 (see MUS-155), LIB-195 (see ADM-125).

### Discovery, home and search (54 features, plus 4 references)

- DIS-001: Home made only of your media
- DIS-002: Instant home
- DIS-003: Build your own home
- DIS-004: Good default home and empty states
- DIS-007: Layout follows you
- DIS-009: Rows as long as you like
- DIS-012: Keep a library off home
- DIS-013: Pinned shortcuts
- DIS-015: A home that does not move
- DIS-019: Speed you can check
- DIS-020: Continue listening
- DIS-021: Recently played
- DIS-022: Dismiss from Continue rows
- DIS-023: Undo, and a Hidden page
- DIS-035: Recently added
- DIS-036: Arrivals grouped by album
- DIS-038: Upgrades are not "new"
- DIS-045: Loves
- DIS-046: Loved songs as a list
- DIS-047: Personal ratings
- DIS-050: History by date
- DIS-051: Your play counts
- DIS-052: Remove a play
- DIS-058: Export everything you told the server
- DIS-060: More like this
- DIS-061: "Because you played" rows
- DIS-062: Every suggestion says why
- DIS-067: Radio from anything
- DIS-070: Suggestions after the queue ends
- DIS-071: Your top tracks by an artist
- DIS-083: One search box
- DIS-084: Search on the device
- DIS-085: Forgiving matching
- DIS-086: Search tags, genres and moods
- DIS-087: Search people by role
- DIS-088: Scope a search
- DIS-089: Recent searches
- DIS-091: Find inside a list
- DIS-100: Endless, smooth lists
- DIS-101: Alphabet jump
- DIS-102: Filters on what the scanner knows
- DIS-103: Filters remembered
- DIS-104: Sorts that matter
- DIS-105: Save a filter
- DIS-107: Grid, list and compact
- DIS-109: Browse pages
- DIS-110: Multi-select
- DIS-111: The same menu everywhere
- DIS-112: Back keeps your place
- DIS-119: One rule language
- DIS-120: Rule editor with live preview
- DIS-121: Smart playlists
- DIS-122: Limits, order and refresh
- DIS-140: Discovery per person

References that ship with their owning rows: DIS-053 (see ACC-117), DIS-106 (see LIB-008), DIS-163 (see CLI-040), DIS-173 (see CLI-034).

### Clients, devices and offline (30 features)

- CLI-001: Web client served by your own server
- CLI-002: Published browser support list
- CLI-003: Installable web app
- CLI-022: Library synced to the device
- CLI-024: Sync and storage status
- CLI-025: Losing the server never blocks the app
- CLI-026: Offline is not a separate mode
- CLI-030: Settings that follow you
- CLI-031: A layout contract
- CLI-032: Old clients keep working
- CLI-033: Diagnostics you can read first
- CLI-034: Deep links
- CLI-040: Letter jump in long lists
- CLI-060: Wide three-pane layout
- CLI-062: Multi-select, drag and right-click
- CLI-070: Lock-screen controls from the web app
- CLI-093: Offline plays and progress merge cleanly
- CLI-099: Fetch ahead on patchy signal
- CLI-103: Continue on this device
- CLI-135: Screen readers reach every control
- CLI-136: Accessibility as a release gate
- CLI-138: Full keyboard use with visible focus
- CLI-139: Text follows the system size
- CLI-140: Reduced motion
- CLI-141: Themes, including high contrast
- CLI-142: Motor accessibility
- CLI-146: Translations with a completeness bar
- CLI-149: Phone-width web layout
- CLI-150: Secure context in R1
- CLI-151: Mono audio and channel balance

### Accounts, sharing and security (45 features, plus 4 references)

- ACC-001: Claim a new server with a one-time setup code
- ACC-002: Owner account with no vendor account
- ACC-003: Sign-in and playback with no internet
- ACC-004: Owner recovery from the host
- ACC-006: Local user accounts
- ACC-007: No user list before sign-in
- ACC-008: Disable an account without deleting it
- ACC-010: Export your own data
- ACC-011: Name and picture for each profile
- ACC-012: Preferences that follow you
- ACC-013: Identity data that survives a cache rebuild
- ACC-017: Profiles separate from sign-in
- ACC-030: Every path obeys the restrictions
- ACC-037: Library access per person
- ACC-040: Several administrators
- ACC-049: Scoped tokens for integrations
- ACC-050: Passkeys
- ACC-052: Password sign-in as a fallback
- ACC-053: Two-factor codes with recovery codes
- ACC-055: Manage your own sign-in methods
- ACC-056: Confirm sensitive changes
- ACC-057: Single sign-on with your own identity provider
- ACC-063: Protection against guessing
- ACC-064: Help a locked-out user
- ACC-065: Sign out everywhere when a credential changes
- ACC-068: Your devices, in one list
- ACC-069: Revoke one device
- ACC-070: Sign out of all sessions
- ACC-079: Session lifetimes
- ACC-080: Invite by link or QR code
- ACC-097: Works behind your own reverse proxy or VPN
- ACC-098: HTTPS served by the server
- ACC-113: Nothing leaves the house by default
- ACC-114: No social feed
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
- ACC-134: Serve under a path prefix behind a reverse proxy

References that ship with their owning rows: ACC-072 (see ADM-099), ACC-073 (see ADM-102), ACC-078 (see ADM-110), ACC-127 (see ADM-054).

### Setup, administration and operations (66 features, plus 7 references)

- ADM-001: Single self-contained binary
- ADM-002: Official container image
- ADM-003: Container tags that pin a version
- ADM-004: Builds for small ARM boards, including 32-bit
- ADM-005: Install as an OS service with one command
- ADM-006: Refuse to run as root by default
- ADM-007: Configuration that is checked, not guessed
- ADM-010: Published footprint numbers
- ADM-011: Hardware guide sized for direct play
- ADM-020: Setup closes for good once an admin exists
- ADM-021: A secure context for passkeys at setup
- ADM-022: Built-in HTTPS for a domain you own
- ADM-025: Add music folders with live checks
- ADM-027: Language, region and time zone
- ADM-028: Privacy choices at setup
- ADM-029: Restore from backup on the welcome screen
- ADM-030: "Coming from another server?" step
- ADM-032: Startup and migration status page
- ADM-042: Import Last.fm and ListenBrainz export files
- ADM-043: Bulk playlist import with a match report
- ADM-044: Matching with reasons and an unmatched queue
- ADM-051: Move to new hardware, OS or container
- ADM-052: Hand the server to a new owner
- ADM-053: Opt-in update check from a signed feed
- ADM-054: Security advisory banner
- ADM-056: Automatic snapshot before every upgrade
- ADM-057: Check migrations before serving
- ADM-058: Upgrade straight from any older version
- ADM-059: Roll back by starting the previous version
- ADM-060: Rollback safety stated for every release
- ADM-065: Daily backups on by default
- ADM-066: Backup contents shown plainly
- ADM-069: Download a backup, upload it elsewhere
- ADM-070: Restore from the UI with a restore point and preview
- ADM-071: Restore from the command line
- ADM-072: Backups verified after they are written
- ADM-074: Full export in documented formats
- ADM-077: Rebuild the cache instead of repairing it
- ADM-078: Checksummed user log that recovers from a torn write
- ADM-079: Refuse a data directory on a network filesystem
- ADM-080: One writer, so no "database is locked"
- ADM-083: Free-space guard
- ADM-088: Scans that report bytes read
- ADM-089: Media mounted read-only
- ADM-090: Separate locations for settings, log, cache and scratch space
- ADM-093: One task list with run, cancel and history
- ADM-095: Heavy jobs that are throttled and resumable
- ADM-099: Now playing
- ADM-100: Playback decision and its reason, per session
- ADM-102: Stop a session with a message
- ADM-108: Health of each library root
- ADM-109: Health summary on the admin home
- ADM-110: Activity and audit log
- ADM-112: Restart and shut down from the UI
- ADM-113: Emergency page served by the server
- ADM-116: Admin alerts with free destinations
- ADM-119: Structured logs with rotation
- ADM-122: Sign-in failure lines for fail2ban
- ADM-123: `gunmetal doctor`
- ADM-124: Diagnostic bundle with preview and masking
- ADM-125: File inspector
- ADM-128: Health endpoints for containers
- ADM-129: Network activity page
- ADM-130: Local crash records
- ADM-140: Server name and sign-in message
- ADM-141: Derived-data store kept across rebuilds

References that ship with their owning rows: ADM-018 (see ACC-001), ADM-019 (see ACC-002), ADM-031 (see LIB-021), ADM-034 (see ACC-004), ADM-085 (see LIB-032), ADM-086 (see LIB-033), ADM-121 (see ACC-123).

### Integrations and extensibility (20 features, plus 2 references)

- INT-001: API reference generated from code
- INT-005: Capability discovery
- INT-006: Change feed (delta sync)
- INT-007: Consistent list endpoints
- INT-008: IDs that survive rebuilds and file replacement
- INT-009: MusicBrainz IDs on music items
- INT-011: Path-scoped library refresh
- INT-012: Per-token rate limits
- INT-013: Health check for uptime monitors
- INT-018: Scope limits by library, root and user
- INT-019: Expiry and rotation
- INT-020: Last use and audit trail per token
- INT-021: Revoke one or revoke all
- INT-022: No-escalation rule
- INT-023: Credentials in headers only
- INT-026: "Who am I" check for tools
- INT-134: Playback decision in the API
- INT-138: Playlist write API
- INT-147: Stable deep links
- INT-151: Documented history export

References that ship with their owning rows: INT-017 (see ACC-049), INT-107 (see ADM-042).

### Adjacent media types (7 features)

- LAT-001: Item kinds in the data model
- LAT-002: People with typed roles
- LAT-006: Typed positions in the user log
- LAT-007: All user-made data in the user log
- LAT-008: Typed links between items
- LAT-009: Listening contexts in the queue protocol
- LAT-010: Spoken word kept out of music

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
2. **Gapless, levelled music in the browser** (MUS-067, MUS-230, MUS-086,
   MUS-087; R1). Pain: gapless is Jellyfin's most-voted music request (647
   votes, open since 2019), and Navidrome's web player will not do it.
   Plexamp already does this well, but only in its own apps. Risks: the
   audio packager's per-browser support is unverified, and scan-time
   loudness needs an ADR on in-process decoders.
3. **Instant browsing and search from a library synced to the device**
   (CLI-022, DIS-084, DIS-002, MUS-208; R1). Pain: Jellyfin's lazy-loading
   request has 1,179 votes and its search regressed repeatedly in 2025.
   Every list and search answers locally, and keeps working when the server
   is down (over HTTPS or localhost, CLI-150).
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
   decision, and browsers still need the owner's proxy (ACC-102 is Later).
6. **Modern sign-in, and revocation that really stops playback**
   (ACC-050, ACC-053, ACC-057, ACC-120 to ACC-122; R1). Pain: single
   sign-on (1,191 votes) and two-factor (1,103) are Jellyfin's most-voted
   security requests; plex.tv has been breached twice; the rivals' 2026
   advisories are mostly authorisation mistakes. Passkeys where the server
   has a domain, password plus two-factor elsewhere (parity with Plex on
   two-factor), OIDC, per-object authorisation tested on every route, and
   stream URLs whose in-flight responses are cut on revocation.
7. **History and fixes that survive every move, upgrade and rebuild**
   (LIB-028, LIB-029, LIB-179, ADM-059, ADM-141; R1). Pain: the "Plex
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
   MUS-005, MUS-008, MUS-010, MUS-012; R1). Pain: Plex has refused multiple
   artists since 2015 (520 votes, plus 866 for robust tags), and Jellyfin
   regressed in 10.11. Navidrome is already good here, so this beats
   Navidrome only together with Gunmetal's own player.
10. **Smart playlists and home rows from one free rule language** (MUS-143,
    MUS-144, DIS-119 to DIS-121; R1). Pain: Jellyfin has no smart playlists
    (588 votes) and Navidrome makes people write JSON. Plexamp's are good.
    The same rules drive home rows now and download rules in R2.
11. **Lyrics that are simply there, free** (MUS-154 to MUS-156, MUS-158;
    R1). Pain: Plexamp charges for lyrics and Plex users want them shown
    automatically (260 votes). Embedded, LRC and word-timed lyrics are read
    at scan time.
12. **Your own record, under your control** (MUS-184, DIS-050, ACC-117,
    ACC-010, DIS-022, DIS-023; R1). Pain: removing a play has 5,458 Spotify
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
  between drives or servers (ADM-092, LAT-112). Media stays read-only; the
  one planned exception is opt-in deletion on a writable root (ADM-139,
  Later).
- **Protocols and shortcuts that undo per-object authorisation.**
  Passwordless sign-in on the local network (ACC-066), automatic port
  forwarding (ACC-106), a DLNA server in the core (CLI-115, VID-152), Plex
  API compatibility (INT-101), in-process native plugins (INT-073), plugins
  that inject scripts into the web client (INT-074), scripts run inside the
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
  friends' listening feed (MUS-191), and usage telemetry on by default
  (ADM-136).
- **Paywalls and vendor caps.** No paid tier, unlock or device cap for any
  feature, in any area (ADR 1, decision 10).

## Open decisions for the project owner

These are the decisions that cross areas or shape the release plan,
de-duplicated from the area files. Each area file keeps its own list for
decisions that affect only that area; where an area decision is settled
here, the area file says so.

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
   plainly (CLI-003). Legacy password sign-in for older apps (INT-088) is a
   separate decision; recommend Later and off by default.
5. **Ship the native Android music app early, as an R1 point release**
   (clients 1). *Recommendation:* yes. Background audio, lock screen,
   downloads and Android Auto do not depend on the remuxer, and this is the
   best answer to the R1 phone gap.
6. **When the plugin host and scrobblers ship** (integrations 2, music 5).
   *Decided here:* R2. A wasmtime host with grants, signing, an egress gate
   and per-user secrets is a large R1 item. *Recommendation:* keep R2; R1
   records every play with its real timestamp so the R2 scrobblers can
   submit what Last.fm and ListenBrainz still accept.
7. **HTTPS names from gunmetal.tv** (ADM-023; accounts 2, admin 2). Without
   a domain, browsers cannot use passkeys. *Recommendation:* keep ADM-023
   Later. It conflicts with ADR 1 decision 7 and decision 9 and needs its own
   ADR. In R1, sign-in falls back to a password plus two-factor wherever
   passkeys cannot work, and the first admin may set a password when setup
   is not a secure context (ACC-002).
8. **In-process decoders for untrusted audio** (MUS-086; music 6, library 5,
   later media 12). Scan-time loudness needs a full decode with a
   third-party crate. *Recommendation:* an ADR that approves the decoder
   crate, its licence, fuzzing and resource limits, and that defines one
   sandboxed decoder worker for images and archives later. If it is not
   accepted before R1, R1 uses loudness tags plus the fallback gain
   (MUS-089).
9. **Amend ADR 2's "no remuxer for music"** (MUS-230, MUS-041).
   Browser gapless and CUE sheets need audio frames repackaged into
   fragmented MP4 or re-headed FLAC slices. *Recommendation:* amend ADR 2
   to name this light, in-process, audio-only packager as the exception.
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
    7, discovery 6). *Recommendation:* live sessions, totals and security
    events by default; full history only for managed child profiles or
    adults who opt in; every person can see what the admin sees (ACC-115,
    R2).
14. **Private listening default** (accounts 6, discovery 6).
    *Recommendation:* off by default, one tap away from the player, and
    clearly shown while on.
15. **Telemetry** (accounts 7, admin 4). *Recommendation:* no usage
    telemetry, ever; opt-in, per-report crash sending after R1, with the
    full report shown first.
16. **Performance budgets** (DIS-019). The rows now carry proposed design
    goals: home in under 200 ms and browse or search in under 50 ms at
    100,000 tracks on the reference low-end device, initial sync of 100,000
    tracks in under 2 minutes, and time to interactive under 3 seconds on
    the cheapest supported stick. *Recommendation:* name the reference
    devices, accept these as goals, and publish the measured numbers either
    way, as the README promises.
17. **Media read-only, with one opt-in exception** (music 9, library 2,
    later media 3, ADM-139). *Recommendation:* media folders stay read-only
    by default; owner deletion from the UI is Later and needs an opt-in
    writable root per library; tags and ratings are never written back.
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
    first-party Linux desktop target (unverified). *Recommendation:*
    prototype React Native Web in Tauri with libmpv before promising Linux
    in R2; fall back to the web client.
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
  about 600 rows to 422, but an "R1.x" release value was not
  introduced, so that every file keeps the same five values; see "The R1
  cut" for why the floor is larger. ACC-053 (two-factor) stayed in R1,
  because password plus two-factor is the R1 default wherever passkeys
  cannot work. Loudness measurement (MUS-086) stayed in R1 with a stated
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
