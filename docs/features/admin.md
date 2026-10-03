# Setup, administration and operations

This map covers everything an administrator does with Gunmetal other than
listening and watching: installing it, the first run, moving in from Plex,
Jellyfin, Emby or Navidrome, upgrading and rolling back, backing up and
restoring, keeping the database healthy, looking after the folders the
library lives in, scheduled work, the admin dashboard and activity log,
alerts, logs and diagnostics. Sign-in design, user policy, invitations,
library scanning rules and webhooks have their own feature maps; they appear
here only where they change an operational task. The bar is this: a family
admin can install Gunmetal on a small box in minutes without creating an
account anywhere; no upgrade, corrupt database or NAS outage can lose
anything that cannot be rebuilt from the files; history, ratings and
playlists come across from the old server with a report before anything is
written; and the tools people run beside Plex and Jellyfin today (Tautulli,
PlexDBRepair, WatchState) become optional. Everything Plex and Emby charge
for in this area is free. Where rivals are already good (Immich's backups and
restore from the welcome screen, Jellyfin's pre-migration copy and startup
page, Navidrome's small command-line toolset and single binary, Plex's
maintenance window and phone dashboards, Emby's packaging breadth) the aim is
parity first and an edge only where the architecture provides one.

## Features

Conventions used in the tables:

- Releases (R1, R2, R3, Later, No), the Demand scale, row ownership and the
  terms "the user log" and "the identity store" are defined once in the
  [feature map README](README.md). A row whose Release cell would differ
  between maps names one owning row; the other maps point at it. ADR 1 (decision 5)
  makes only watch history durable, so every row that stores other user
  state depends on ADR 3 (see Dependencies and risks).
- UI surfaces are named as screens: the first-run flow is "Welcome", the
  admin area is "Admin > ...", the minimal server-rendered page is the
  "Emergency page", and the page shown before the database opens is the
  "Startup page". "CLI" means a `gunmetal` subcommand on the host.
- Rival evidence comes from the research files in `docs/research/`. Vote
  counts are as read on 2026-10-02. "(unverified)" marks claims the research
  could not confirm.

### Installing and packaging

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| ADM-001 | Single self-contained binary | Download one file for your OS and processor, run it, and have a working music server with nothing else to install. | Plex no; Jellyfin partial (portable Linux builds, runtime bundling unverified); Emby unverified; Navidrome yes, but needs a system FFmpeg | Medium: Navidrome's single binary is the praised reference; Jellyfin is moving away from small hardware | R1 | A static Rust build with the core parsers and the web client compiled in. Browsing and original-quality playback need no FFmpeg, because browsers and phones play FLAC, MP3, AAC and Opus as they are (ADR 2); Navidrome cannot say that for original-quality playback. Opus conversion (MUS-106, ACC-107, both R2) needs the sandboxed encoder: FFmpeg or libopus in the sandbox. | Release pipeline that builds static binaries per target; embedded web client assets; version and build information endpoint | Download page on gunmetal.tv; Admin > About (version, build, target) |
| ADM-002 | Official container image | One `docker run` command or compose file. | Plex yes; Jellyfin yes, with Podman and Kubernetes guidance; Emby yes | High: the selfh.st 2025 survey population is mostly container users | R1 | Mostly parity. The edge is posture: the image holds one static binary, runs as a non-root user by default, needs only its data volume to be writable, and takes media read-only. | Multi-architecture image builds; documented volumes for data, cache and media; health check (ADM-128) | Install docs with a compose example |
| ADM-003 | Container tags that pin a version | Pin an exact version, or follow a major line that never crosses a breaking change. | Plex: the `public` and `beta` tags download the latest server at every start; Jellyfin and Emby unverified; Immich has major-line tags | Medium: Plex's tags mean the image does not fix the version that runs | R1 | Exact-version tags and major-line tags only, both signed. No tag downloads a server at start, so the image you pulled is the server you run. | Release process only | Install docs; release notes |
| ADM-004 | Builds for small ARM boards, including 32-bit | Reuse a Raspberry Pi-class board as a music server. | Plex unverified; Jellyfin dropped 32-bit ARM in 10.11 and calls most boards too slow; Emby unverified; Navidrome ships 32-bit ARM builds and claims the Pi Zero | Medium: Jellyfin's drop of 32-bit ARM; a Pi 4 run out of memory by scans (#11588, 129 comments) | R1 | x86-64 and AArch64 in R1; ARMv7 (32-bit) in R2, because wasmtime support for 32-bit ARM, needed by the plugin host, is unverified. Serving music is a disk read and a network write, and scans launch no process per file, so a small board is a realistic music server. The published footprint numbers (ADM-010) include one. | CI builds and smoke tests per target; a reference ARM board in the benchmark ARMv7 depends on wasmtime ARMv7 support (unverified). | Download page; hardware guide |
| ADM-005 | Install as an OS service with one command | `gunmetal service install` sets up a system service that runs as its own user. | Plex: the installer does it (unverified); Jellyfin: packages ship service units (unverified); Emby unverified; Navidrome: a `service` command | Low: Navidrome is cited as best in class; no vote counts | R1 | Parity with Navidrome, plus the generated systemd unit turns on the service manager's sandboxing options by default. R1 covers systemd only; the Windows service and launchd parts arrive in R2 with ADM-013. | systemd integration in R1; Windows services and launchd in R2 (ADM-013); service-user creation | CLI; install docs |
| ADM-006 | Refuse to run as root by default | A network-facing service that never runs with full privileges by accident. | Plex, Jellyfin and Emby unverified; Navidrome has an `EnforceNonRootUser` option (0.62) | Low: one rival option, no votes | R1 | Refusal is the default, with a documented override flag, rather than an option the admin has to find. | Startup privilege check; override flag | Startup error text; doctor (ADM-123) |
| ADM-007 | Configuration that is checked, not guessed | A typo in the config file is reported with its line instead of being silently ignored. | Plex, Jellyfin and Emby unverified; Navidrome 0.64.0 warns about unknown options | Low: Navidrome evidence only | R1 | Config parses into typed structures, so unknown keys, wrong types and out-of-range values stop startup with the file and line. `gunmetal config check` validates a file without starting the server. | Typed config schema; check subcommand | CLI; Startup page |
| ADM-008 | Settings show their source and whether a restart is needed | See whether a value came from the config file, an environment variable or the UI, and which changes need a restart. | Jellyfin has a "restart pending" notification; others unverified | Low: a Jellyfin request for restart-required notices has 19 votes | R2 | Most settings apply live. Those that cannot are labelled before you change them, and one "restart pending" banner collects them. Values fixed in the file or environment show as locked in the UI, with the reason. Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Settings registry with source, live-apply and restart flags | Admin > Settings; restart banner |
| ADM-009 | Settings export and import with a diff | Copy one server's settings to another, or keep them in version control. | Plex unverified; Jellyfin completed a settings import and export request (899 votes); Emby's configuration backup needs Premiere | High: Jellyfin's 899-vote request | R2 | Settings are one versioned, human-readable document that is also in every backup. Import shows a field-by-field diff and applies only what you tick. Secrets are never exported in clear. | Settings serialiser; diff; secret exclusion | Admin > Settings > Export and import; CLI |
| ADM-010 | Published footprint numbers | Know before buying hardware what Gunmetal needs: idle memory, memory during a scan, bytes read per file and database size per 10,000 items. Also: rebuild time for a 100,000-track library, and the R2 software-transcode capacity (how many 1080p transcodes, possibly none, on each reference machine). | All three size hardware for transcoding; Emby lists 512 MB of RAM on Linux and Jellyfin 8 GB; none publishes what a direct-play-only server needs | Medium: scan memory and time complaints (#15070, 110 comments; #17871); footprint numbers are an open question in the research | R1 | Measured for every release on named reference hardware and against Jellyfin on the same library, and published whether or not the numbers flatter Gunmetal, as the README promises. Rebuild time and transcode capacity are design goals until the benchmark exists. Client budgets are in DIS-019. | Benchmark harness; process self-measurement shared with the dashboard | Benchmarks page on gunmetal.tv; Admin > About shows the live footprint |
| ADM-011 | Hardware guide sized for direct play | Advice that starts from disks and network, with transcoding as an optional extra. | Plex: a PassMark formula per transcode; Jellyfin: named CPUs and honest low-power advice; Emby: CPU and RAM minimums | Medium: no rival says what a server that never transcodes needs | R1 | Because the common case is a disk read, the guide can recommend very small machines for music and adds a separate transcoding section only when video lands in R2. | Data from ADM-010 | Docs; link from the setup checklist (ADM-033) |
| ADM-012 | Signed deb and rpm repositories | `apt` or `dnf` install and update with signature checks. | Plex yes, but 1.43.0 broke repository signing on Debian and RHEL in January 2026; Jellyfin yes; Emby has packages (repository details unverified); Navidrome attaches deb and rpm files | Medium: the Plex 1.43.0 thread has 395 posts and 25,226 views | R2 | Parity on the feature. The edge is process: each release is installed and upgraded on clean machines of every supported distribution in CI before it is published. | Release pipeline; signing keys; repository hosting | Install docs |
| ADM-013 | Windows and macOS installers | Install without a terminal on the desktop systems many Plex owners use. | Plex yes; Jellyfin yes; Emby yes; Navidrome ships an MSI | Medium: every rival offers it | R2 | Parity. The R1 binary (ADM-001) already runs on both; the installer adds the service, a firewall rule and clean removal. | Installer builds; Windows service and launchd integration | Installer; install docs |
| ADM-014 | FreeBSD builds | Run on BSD-based NAS systems. | Plex unverified; Jellyfin no, because .NET does not support FreeBSD; Emby yes; Navidrome users asked in September 2026 (#6213) | Low: one open Navidrome request | R2 | Ahead of Jellyfin and Navidrome (no FreeBSD binaries); parity with Emby, whose download page lists FreeBSD. Music works fully; transcoding is unavailable until ADM-133 has a FreeBSD sandbox. | FreeBSD CI target; sandbox backend later | Download page; dashboard capability notice |
| ADM-015 | NAS container catalog templates | One-click install from the Unraid and TrueNAS app catalogs. | Plex broad NAS support; Jellyfin lists Synology and TrueNAS SCALE; Emby has the broadest vendor list | Medium: Emby's breadth is the best in class | R2 | Parity. Templates point at the same signed image and pre-fill a read-only media mount. | Template maintenance per catalog | Catalog listing; docs |
| ADM-016 | Native NAS vendor packages | Install from the Synology or QNAP package centre. | Plex broad; Jellyfin Synology; Emby broadest; a Synology package has blocked Plex downgrades | Medium: as ADM-015 | Later | Parity, with one rule: no package may block installing an older version, because rollback (ADM-059) depends on it. | Vendor-specific packaging and review | Vendor package centre |
| ADM-017 | Managed hosting listed in the docs | Someone else runs the server for you. | Plex and Jellyfin none; Emby lists hosting partners; Navidrome's docs point to hosting services | Low: not a project goal | Later | Parity: a docs page, no partnership. | None | Docs |

### First-run setup

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| ADM-018 | One-time setup code | See ACC-001, which owns this feature. | Plex: claimed through a plex.tv login; Jellyfin 12.0 fixed a hole that let the wizard be re-run; Navidrome 0.64.2 fixed a failed admin creation | Medium: rivals fixed setup holes in 2026 | R1 | See ACC-001. | None beyond ACC-001. | Welcome > Setup code; console banner |
| ADM-019 | Admin account with no vendor account | See ACC-002, which owns this feature. Admin specifics: the first admin registers a passkey or links an OIDC identity where the page is a secure context, and sets a password where it is not. | Plex no (the server is claimed by a plex.tv account); Jellyfin yes; Emby yes, Emby Connect optional | High: Plex's local-authentication request has 372 votes and 6,077 views; plex.tv was down for about two hours in July 2026 | R1 | See ACC-002. | None beyond ACC-002. | Welcome > Create admin |
| ADM-020 | Setup closes for good once an admin exists | The setup screens cannot be reached or re-run later. | Jellyfin closed a re-run hole in 12.0; Navidrome fixed a failed setup that left a server with no admin | Medium: two rival fixes in 2026 | R1 | Setup routes are not registered once an admin exists, and admin creation is one transaction, so a failure leaves the server in setup mode rather than half-configured. The route-table test that fails the build on any unprotected route covers it. | Setup state machine; conditional route registration; route-table test | None after setup |
| ADM-021 | A secure context for passkeys at setup | Setup that explains itself when it is opened from another computer over plain HTTP. | Plex issues its own HTTPS names (detail unverified); Jellyfin and Emby leave HTTPS to the admin | Medium: WebAuthn needs HTTPS or localhost and a domain, not an IP address (research risk) | R1 | The welcome page detects a non-secure context and offers the paths that work: open it on the server itself, use a domain you own (ADM-022), or, from R2, set up from a native app (ADM-035). Whether R1 offers anything more is an owner decision. R1 also allows a password for the first admin (ACC-002), so setup never dead-ends. | Secure-context detection; TLS listener | Welcome > "Why can't I add a passkey here?" panel |
| ADM-022 | Built-in HTTPS for a domain you own | HTTPS on your own domain without running a reverse proxy, using a certificate you supply. | Plex automatic through Plex-issued names (detail unverified); Jellyfin: bring your own, usually through a reverse proxy; Emby unverified | Medium: 2,716 selfh.st 2025 respondents use a reverse proxy for remote access | R1 | In R1 the server serves a certificate the owner supplies and reloads it when the file changes (ACC-098 owns this). Automatic issuance and renewal through ACME is ACC-099 (Later). Reverse proxies such as Caddy, which obtain certificates themselves, stay the easiest route, and forwarding headers are ignored unless the proxy is listed as trusted (ACC-097). | None beyond ACC-097 and ACC-098 | Welcome > HTTPS; Admin > Settings > Network |
| ADM-140 | Server name and sign-in message | A custom server name on the sign-in page and a short message or disclaimer under it | Jellyfin yes (server name, login disclaimer and custom CSS); others not covered in the research | Low: no vote data | R1 | Plain text only, with no HTML or CSS, so it cannot become an injection route; custom CSS is the INT-081 theme extension point (Later) | Two text settings, served to the sign-in page | Admin > General; sign-in page |
| ADM-023 | Optional per-server HTTPS name from gunmetal.tv | A working HTTPS address on the home network without owning a domain. | Plex's issued names serve this purpose (detail unverified); Jellyfin and Emby none | Medium: without it, a headless server reached by IP cannot use passkeys in a browser | Later | Only if the owner approves it (see Open decisions): opt-in, holds no accounts, and the private key never leaves the server. To be honest, it is a central service and it would see which address each name points to. Stays Later unless an ADR amends ADR 1 decision 7 (no central account) and decision 9 (Cloudflare hosts only the docs and landing page). | Name-issuing service; DNS challenge flow; per-server key | Welcome > HTTPS; Admin > Settings > Network |
| ADM-024 | Headless, declarative setup | Bring a server up from a compose file or automation without a wizard. | Plex: a `PLEX_CLAIM` token, tied to plex.tv; Jellyfin: open the wizard from another machine; Navidrome: first music folder from config | Medium: Plex's claim token is the clean headless path; most self-hosters use containers | R2 | Everything the wizard sets can be seeded from config or environment: libraries, the OIDC provider and the admin's OIDC subject, the update-check choice and the backup folder. Seeding is idempotent and needs no vendor token. Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Seed applier at startup; seeded values marked as from file (ADM-008) | Config docs; Admin > Settings shows seeded values |
| ADM-025 | Add music folders with live checks | Point at your music and see at once whether the server can read it. | Plex yes; Jellyfin: an optional step with a folder picker; Navidrome: from config, and in the UI since 0.58 | Medium: every rival has it; Immich checks folders are readable on restore | R1 | The picker checks read permission and the filesystem type (local, network share, FUSE), sets the I/O profile (ADM-087), and warns before scanning an empty or unreadable folder. Nothing is written into the folder. | Admin-only folder browsing limited to allowed base paths; filesystem detection; root registry in durable settings | Welcome > Libraries; Admin > Libraries |
| ADM-026 | Add movie and TV folders in setup | Video libraries set up the same way. | Plex yes; Jellyfin yes; Emby yes | Medium: expected parity | R2 | As ADM-025, plus the naming report from the library map before the first scan. | As ADM-025 | Welcome > Libraries |
| ADM-027 | Language, region and time zone | Correct language and dates from the first screen. | Jellyfin: a server-wide step; Plex per library (unverified); Emby unverified | Low | R1 | Parity. | Locale and time-zone settings | Welcome > Language |
| ADM-028 | Privacy choices at setup | Decide up front what, if anything, the server may contact on the internet. | Plex is account-based; Jellyfin: no phone-home found; Navidrome sends anonymous usage data by default | Medium: privacy is the second reason to self-host (3,520 of 4,081 selfh.st respondents); Plexamp sends download telemetry with no off switch | R1 | Setup lists every outbound feature (update check, certificates, plugin network grants) and all are off until ticked. All outbound traffic passes one gate, so the list is complete by construction and ADM-129 can show what actually happened. | Egress gate with a registry of outbound features | Welcome > Privacy; Admin > Settings > Network and privacy |
| ADM-029 | Restore from backup on the welcome screen | Rebuild a dead server from a fresh install in a few clicks. | Plex no; Jellyfin: command line only (`--restore-archive`); Emby unverified; Immich: a welcome-screen button that checks the library folders | Medium: Jellyfin's "migrate server" request has 20 votes; Immich is the praised reference | R1 | The backup is small (ADM-065), so restoring is an upload or a file pick. The server checks that every library root is reachable, offers to remap moved roots (ADM-051), then rebuilds the cache from the files. | Restore in setup mode; archive verification; root checks; cache rebuild job | Welcome > Restore; restore progress page |
| ADM-030 | "Coming from another server?" step | Bring history, ratings and playlists across as part of setup. | None built in; third-party tools only | High: WatchState (about 1.6k stars) and JellyPlex-Watched (about 1k) exist because no server imports another's history | R1 | The step queues an import (ADM-036 to ADM-043) to run after the first scan and shows its dry-run report before anything is written. In R1 the step offers only the R1 importers: M3U playlists (ADM-043) and Last.fm or ListenBrainz export files (ADM-042). Importers that read rival databases arrive in R2. | Importer framework | Welcome > Import; Admin > Migration |
| ADM-031 | Usable before the first scan finishes | See LIB-021, which owns this feature. Admin specifics: progress shows files, bytes read and an estimate. | Plex and Jellyfin fill in while scanning (unverified); Navidrome browses as content appears and publishes expected scan times | Medium: Jellyfin scan regressions (#15070, 110 comments; #11588, 129 comments) | R1 | See LIB-021. | None beyond LIB-021. | Welcome > Done; Admin > Dashboard scan card; library empty state |
| ADM-032 | Startup and migration status page | A page that says what the server is doing instead of a refused connection. | Jellyfin 10.11 added a startup page; Immich shows a maintenance page; Plex and Emby unverified | Medium: Jellyfin 10.11 migrations took hours on inconsistent databases | R1 | Parity with Jellyfin, plus each step, the snapshot location (ADM-056), config errors (ADM-007) and an estimate. The page is read-only and shows no secrets. | HTTP listener that starts before the database opens; status model | Startup page |
| ADM-033 | A next-steps checklist that ticks itself | Know what is left: a backup, a second way into the admin account, client apps, the update choice. | Jellyfin's last wizard step points onward; Immich has a post-install guide | Low | R2 | The list reads server state and ticks items as they become true (a backup succeeded, a second passkey exists) instead of being static text. Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Checklist rules over server state | Admin > Dashboard checklist card |
| ADM-034 | Admin recovery from the host | See ACC-004, which owns this feature. Admin specifics: the `gunmetal admin recover` command, and a banner after use. | Plex: email reset; Jellyfin's default reset asks for a file on the server (unverified); Emby unverified | Low: a research risk rather than a vote count | R1 | See ACC-004. | None beyond ACC-004. | CLI; admin banner; Admin > Activity |
| ADM-035 | Set up from a native app with a device key | Set up a headless server from a phone or TV app without HTTPS. | Plex app-driven claim (unverified); others unverified | Medium: it avoids the WebAuthn domain problem for the most common headless case | R2 | The app finds the server on the LAN, takes the setup code and enrols a device-bound key held in the platform's secure storage, which needs no domain (ADR 1, decision 7). | LAN discovery; device-key enrolment | Mobile and TV onboarding |

### Migration and portability

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| ADM-036 | Import music history, ratings and playlists from Plex | Keep play counts, last-played dates, star ratings and playlists when leaving Plex. | Plex's own cross-server sync excludes music; Jellyfin and Navidrome users rely on scripts, such as a one-star ratings tool marked "use at your own risk" | High: switching cost keeps lifetime Plex users in place (pain-points open question 7) | R2 | Reads a copy of Plex's database offline, so it needs no Plex account, token or running server. Every imported fact goes into the user log tagged with its source and an import ID. | Plex schema readers with fixtures from several versions; matcher (ADM-044); import job | Admin > Migration > Plex |
| ADM-037 | Import video watch state from Plex | Watched flags, resume points, play counts and ratings for films and shows. | Jellyfin and Emby: third-party tools only; Plex's own "export watch history" request has 1,700 views | High: the same switching-cost evidence | R2 | As ADM-036, matched by path, then IMDb, TMDb and TVDB IDs. | As ADM-036, plus video identity from the library map | Admin > Migration > Plex |
| ADM-038 | Import music from Jellyfin and Emby | Keep favourites, play counts and playlists. | Third-party tools only; Jellyfin itself cannot import an Emby database | High: moving music from Jellyfin to a music-first server is a common switch | R2 | Reads database copies offline, with fixtures per version, because Jellyfin rewrote its database twice in a year. | Jellyfin and Emby schema readers; matcher | Admin > Migration > Jellyfin or Emby |
| ADM-039 | Import video watch state from Jellyfin and Emby | As ADM-037, for Jellyfin and Emby. | Third-party tools only | High: as ADM-037 | R2 | As ADM-038. | As ADM-038 | Admin > Migration > Jellyfin or Emby |
| ADM-040 | Import from Navidrome and other Subsonic servers | Stars, ratings, play counts, playlists and bookmarks. | None built in | Medium: Navidrome has 656 users in the selfh.st 2025 survey and is the music-first rival | R2 | Reads a copy of Navidrome's database offline; other Subsonic servers go through the API path (ADM-047). Navidrome 0.64 re-encoded every ID, so the reader keys on paths and tags, not IDs. | Navidrome schema reader | Admin > Migration > Navidrome |
| ADM-041 | Import an iTunes or Apple Music library file | Ratings, play counts and playlists from a desktop library. | Plex, Jellyfin and Emby unverified | Low: named as a source in the research, with no counts | R2 | Parses the library XML file locally and matches as ADM-044 does. | XML reader with entity expansion disabled | Admin > Migration > iTunes |
| ADM-042 | Import Last.fm and ListenBrainz export files | Years of listening history appear in your own statistics. | Plex: requested on its forum; Jellyfin no; Navidrome: requested (28 upvotes) | Medium: Navidrome's 28-upvote request and the Plex forum request | R1 | Reads the services' export files locally, so no network grant is needed. Imported plays are marked as imported, so scrobbler plugins never send them back. Pulling history from those services' APIs stays a plugin (ADR 2). Owns history import from export files; MUS-189 and INT-107 point here. | Export-file parsers; an "imported" flag on listen events | Admin > Migration > Listening services |
| ADM-043 | Bulk playlist import with a match report | Bring a folder of M3U playlists across and see which entries matched. | Navidrome imports M3U from the library; Plex has no built-in music playlist export, and one forum user spent weeks rebuilding 614 items | Medium: that Plex thread; Jellyfin playlist import requests (29 and 31 votes) | R1 | Each entry is matched by path relative to a remapped root, then by tags and duration, and listed with its reason; misses go to the review queue. Everyday M3U handling belongs to the music map. Uses the MUS-140 parser and matcher; MUS-140 owns M3U import and export. | M3U and M3U8 parser; matcher | Admin > Migration > Playlists |
| ADM-044 | Matching with reasons and an unmatched queue | Confidence that imported history lands on the right file, and a list of what did not. | Third-party tools; migration guides tell users to fix unmatched titles by hand | High: the switching-cost evidence above | R1 | One matcher for every importer: path relative to the library root, then provider IDs (MusicBrainz, IMDb, TMDb, TVDB), then tags and duration. Each match records its reason and confidence, and low-confidence matches wait for the admin. | Matcher in the core; review-queue storage | Admin > Migration > Review |
| ADM-045 | Dry-run report | See exactly what an import will change before it writes anything. | JellyPlex-Watched has a dry-run mode; no server does | Medium: only a third-party tool offers it | R2 | Built in and always first: counts per category, conflicts with existing Gunmetal data and unmatched items, downloadable as CSV. | Importers that can run in plan mode | Admin > Migration > Report |
| ADM-046 | Undo an import | Back out a bad import completely. | None; Navidrome's path remapping cannot be undone | Medium: Navidrome asks for a backup before remapping for this reason | R2 | Imported events carry an import ID. Undo appends retractions for that ID, so the log stays append-only, and the cache rebuilds. | Import IDs on events; a retraction event type | Admin > Migration > History |
| ADM-047 | Import from a rival's API instead of a database copy | Import from a server running on another machine or an appliance where copying files is hard. | Third-party tools use the APIs | Medium: the research leaves offline reads versus API reads open | R2 | An optional path that uses the admin's own credentials for the old server, declared as an explicit network grant and discarded after the import. | API clients for Plex, Jellyfin, Emby and Subsonic; egress grant | Admin > Migration > Connect to a server |
| ADM-048 | Users brought over as invitations | Household members and friends get an invitation set up with their old library access. | Plex: shares to other accounts may be lost when a server moves; Jellyfin: users travel inside its backup; Wizarr (3.2k stars) fills the invitation gap | Medium: the Plex move-guide warning and Wizarr's popularity | R2 | Credentials never move, since passwords cannot become passkeys. Each old user becomes a pending invitation with library access mapped, and their imported history attaches when they accept. | User mapping; invitations (users map); deferred history attribution | Admin > Migration > Users |
| ADM-049 | Run side by side during the switch | Keep using the old server for a while without history drifting apart. | WatchState syncs between servers; no server does | Medium: WatchState has about 1.6k stars | R2 | Re-running an import appends only events newer than the last run's high-water mark, and stable event IDs make it idempotent. | Per-source cursor; idempotent event IDs | Admin > Migration > Sync again |
| ADM-050 | Import collections and metadata edits | Keep curated collections, match fixes and locked artwork. | Nobody offers a portable format; only by copying whole data directories | Medium: curation lost on moves is part of the Plex backup complaints | Later | Imported curation lands in the curation log, once that ADR exists, so it can be undone like any other import. | Curation log; rival schema readers | Admin > Migration |
| ADM-051 | Move to new hardware, OS or container | Replace the machine, switch from Windows to Linux, or move into Docker, keeping everything. | Plex: documented for the same OS only; Jellyfin: media paths must stay identical; Navidrome: `missing fix` remaps files by hand | High: the Plex and Jellyfin migration limits | R1 | Each library root is stored once and every item is a root plus a relative path plus a content identity, so a move is "point root X at this path" during restore. | Root registry in durable settings; remap step; verification scan | Welcome > Restore > "Where are your libraries now?"; Admin > Libraries > Change location |
| ADM-052 | Hand the server to a new owner | Give a server to a family member with history and access intact. | Plex: unclaim and reclaim, and shares and watch states do not follow; Jellyfin and Emby allow it by design | Low: Plex forum threads, no counts | R1 | Parity with Jellyfin: add a second admin, then remove yourself. Nothing is tied to an outside account. Owns R1 ownership hand-over, using several administrators (ACC-040). | Admin role changes (users map) | Admin > Users |

### Upgrades and rollback

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| ADM-053 | Opt-in update check from a signed feed | Learn that a new version exists, without the server contacting anyone unless you allow it. | Plex unverified; Jellyfin no (#30, 51 votes; #3502, 50 votes); Immich checks GitHub releases | Medium: two Jellyfin requests of about 50 votes each | R1 | Fetches a signed static manifest from gunmetal.tv with no identifiers attached, and nothing installs itself. The choice is made at setup (ADM-028). | Signed feed format; pinned verification key; daily fetch job | Admin > Updates; dashboard update card |
| ADM-054 | Security advisory banner | Be told plainly when the running version has a known security problem. | Plex emailed owners in August 2025, which needs its account system; Jellyfin explains fixes in release posts; Navidrome lists advisories per release | High: about 314,000 Plex servers were still on vulnerable versions weeks after a fix | R1 | The feed carries affected version ranges and severity, and an affected server shows a banner naming the fixed version on every admin screen. The project never learns who runs what. Owns the advisory banner; ACC-127 points here. | Advisory ranges in the feed; version comparison | Admin banner; Admin > Updates |
| ADM-055 | Release channels with free release candidates | Try the next version before everyone else. | Plex beta channel (Plex Pass, unverified); Jellyfin public release candidates; Immich offers release candidates | Low | R2 | Parity. | Channel field in the feed | Admin > Updates |
| ADM-056 | Automatic snapshot before every upgrade | A safety net you did not have to remember. | Plex: only its routine three-day backup; Jellyfin copies the old database before migrating; Navidrome and Immich ask for a manual backup | High: Jellyfin 10.11 migration failures (#15027, 121 comments); a Plex "Corrupted Database Upon Upgrade" thread in April 2026 | R1 | Before any migration the server snapshots the cache with SQLite's online backup and copies the user log and settings, labelled with both versions. The irreplaceable part is small, so many snapshots can be kept. | Online backup; retention; snapshot labels | Startup page; Admin > Backups (pre-upgrade entries) |
| ADM-057 | Check migrations before serving | Run the migration, see the result and the time it took, and only then serve. | Jellyfin 12.0 added a migrate-and-exit mode; others unverified | Medium: hours-long Jellyfin 10.11 migrations | R1 | `gunmetal migrate --check` runs against a copy and reports the result and duration without touching live data. Migrations are pure functions tested on fixtures under the 100% coverage and zero-surviving-mutant gate. | Migration framework that works on copies; fixture corpus | CLI; Startup page |
| ADM-058 | Upgrade straight from any older version | Update a neglected box in one step. | Jellyfin requires 10.10.7 first (or any 10.11.x for 12.0); older Immich installs must finish a migration first; Plex unverified | Medium: the upgrade-path complaints in the research | R1 | An old cache is discarded and rebuilt rather than migrated, and the log format only changes additively, so any version can read any older log. The rebuild reuses stored analysis and provider data from the derived-data store (ADM-141); only parsing is repeated. | Additive, versioned log schema; rebuild fallback | Release notes; Startup page |
| ADM-059 | Roll back by starting the previous version | Undo a bad upgrade without restoring a backup. | Plex: users reinstall older builds by hand, and a Synology package can block it; Jellyfin: only by restoring a backup; Immich: no downgrades; Navidrome 0.58 cannot be downgraded | High: Plex's 1.25.5 regression thread (392 posts) and the 1.43.0 rollback | R1 | An older binary that finds a newer cache throws it away and rebuilds from files plus log, skipping event types it does not know. The pre-upgrade snapshot (ADM-056) is the fast path when its version matches. | Forward-compatible log reader; cache version check | Startup page ("rebuilding: data was written by a newer version") |
| ADM-141 | Derived-data store kept across rebuilds | Upgrades, rollbacks, restores and repairs do not re-decode every untagged track or re-fetch every provider record | Plex and Jellyfin keep derived data in their data directories, which is why those directories are large (research) | High: pain-points item 6 asks for rebuilds in minutes | R1 | Analysis results (loudness from MUS-086, later sonic features and the neighbour table) and provider responses and artwork are stored keyed by content identity plus parser or provider version, outside the SQLite cache. A rebuild reuses them, so LIB-024's "never repeats work" holds; including them in backups is optional, so ADM-065 backups stay small by default | Derived-data store keyed by identity and version; reuse on rebuild; optional inclusion in backups | Admin > Backups (include derived data); Admin > Database |
| ADM-060 | Rollback safety stated for every release | Know before upgrading whether you can go back. | Immich labels breaking changes; Jellyfin release posts have sections for administrators | Medium: the upgrade failures above | R1 | Each release declares the oldest version it can safely roll back to, in its notes and in the feed, so the dashboard can say it next to the upgrade instructions. | Feed field | Admin > Updates |
| ADM-061 | Withdrawn releases flagged | A broken build stops spreading, and servers already on it are told. | Plex rolled its production channel back to 1.42.2 after 1.43.0 | Medium: the 1.43.0 thread (395 posts) | R2 | The feed can mark a release as withdrawn, and servers running it show a banner naming the version to go back to. Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Feed field | Admin banner |
| ADM-062 | Upgrades re-read only what a new parser changes | An upgrade does not mean hours of rescanning. | Jellyfin 10.11 and 12.0 asked for full rescans after upgrading; Plex and Emby unverified | Medium: Jellyfin's release notes and rescan complaints | R2 | When the cache can be kept, each file's stored results carry the version of the parser that produced them, so only files whose format parser changed are read again. This is a design goal the scan benchmark has to prove. Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Parser version stamps per file; selective re-read job | Startup page; dashboard scan card |
| ADM-063 | Plugin compatibility check before upgrading | Know which plugins a new version breaks before you upgrade. | Jellyfin 12.0 required removing every third-party plugin first and rebuilding them for .NET 10 | Medium: the Jellyfin 12.0 instructions; 10.11.9 broke plugin repository lookups | R2 | Plugins target a versioned WebAssembly interface (integrations map), and the update card lists each installed plugin's compatibility with the new version. | Plugin interface versioning | Admin > Updates |
| ADM-064 | Opt-in automatic updates with health-check rollback | Stay patched without effort, without a broken build sticking. | Plex's `public` tag updates at every restart; Jellyfin has no built-in updater | Low: convenience; the Plex 1.43.0 failure shows the risk | Later | Updates install only after a signature check; the new version runs its migration check first, and the server switches back automatically if any check fails. | Updater; atomic binary swap; post-start health probe | Admin > Updates |

### Backup, restore and export

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| ADM-065 | Daily backups on by default | Backups exist without anyone remembering to make them. | Plex: the core database every three days, three copies; Jellyfin: manual only, scheduling requested (#3546, 91 votes); Emby: Premiere only; Immich: daily, keeping 14 | High: Jellyfin's backup request had 142 votes and scheduling has 91; Plex's backup request has four newer duplicates | R1 | A backup holds only the user log, settings, identities and server keys: kilobytes to megabytes, not the tens to hundreds of gigabytes in a Plex or Jellyfin data directory. Daily backups with long retention are therefore a sensible default, and free. Derived data (ADM-141) is excluded by default and can be included. | Backup job; consistent snapshot of log and settings; retention; destination folder | Admin > Backups; dashboard health card |
| ADM-066 | Backup contents shown plainly | Know what a backup holds and what it leaves out. | Plex leaves out metadata and preferences; Jellyfin lets you choose; Navidrome and Immich back up the database only and say so | Medium: a Plex moderator confirmed preferences are not included | R1 | Each archive carries a manifest: users, playlists, the date range of history, settings, and what is deliberately left out (the cache and the media) with the reason. | Manifest writer | Admin > Backups > Backup detail |
| ADM-067 | Optional cache snapshot for a fast restore | Restore a large library without waiting for a full rescan. | Jellyfin can include metadata, subtitles and trickplay images | Medium: rebuild time is the price of a rebuildable cache (research risk) | R2 | Off by default. When included, a restore uses it if its version matches and rebuilds otherwise. Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Online SQLite backup into the archive | Admin > Backups > Settings |
| ADM-068 | Encrypted backups | A backup copied off the machine does not expose the household's viewing history. | None found | Medium: privacy is the second reason people self-host | R2 | Any archive that leaves the machine (a download or an off-site copy) is encrypted with an admin-chosen passphrase or recovery key; local archives can be too. | Archive encryption; passphrase never stored in clear | Admin > Backups; download dialog |
| ADM-069 | Download a backup, upload it elsewhere | Move a backup through the browser. | Plex no; Jellyfin requested (#3789, 7 votes); Immich uploads a database dump in the UI | Low: 7 votes | R1 | Backups are small enough to move in one click either way. | Admin download and upload endpoints | Admin > Backups; Welcome > Restore |
| ADM-070 | Restore from the UI with a restore point and preview | Restore without a shell, see what you are restoring, and survive a failed restore. | Plex no; Jellyfin yes, restarting at once; Immich creates a restore point and rolls back if the restore fails | Medium: Immich is the praised reference | R1 | Parity with Immich, plus a preview of the archive's manifest and root remapping (ADM-051) before anything is replaced. | Restore point; maintenance mode; manifest reader | Admin > Backups > Restore; maintenance page |
| ADM-071 | Restore from the command line | Scriptable recovery. | Plex: manual file copying or PlexDBRepair; Jellyfin `--restore-archive`; Navidrome `backup restore` with the server stopped | Low | R1 | Parity; works with the server stopped or in maintenance mode. | Restore subcommand | CLI |
| ADM-072 | Backups verified after they are written | A backup you can trust, and an alert when one fails. | None found; Plex sends backup and corruption webhook events (Plex Pass) | Medium: Navidrome users recovered from a 2026 scan bug only through automatic backups (#6200) | R1 | After each backup the server reopens the archive, checks its hashes and replays the log into a scratch store. A failure, or a last good backup older than a threshold, raises an alert (ADM-116). | Verification job; alert rule | Admin > Backups status column; dashboard health card |
| ADM-073 | Off-site backup destinations | Backups leave the house automatically. | Plex no; Jellyfin declined S3 as a target (#3855); every rival leaves it to the user | Medium: a gap in every rival | R2 | Small, encrypted archives make S3-compatible and WebDAV targets cheap. R1 already writes to any folder, including a mounted share. | Destination clients; egress grant; encryption (ADM-068) | Admin > Backups > Destinations |
| ADM-074 | Full export in documented formats | Leave Gunmetal, or feed another tool, with nothing locked in. | Plex: no history export; Jellyfin: no built-in export; WatchState makes portable play-state backups | Medium: Plex's export request has 1,700 views; Jellyfin's "Export settings and watched status" has 17 votes | R1 | The user log and settings export as documented, versioned JSON, playlists as M3U, and music listens in a ListenBrainz-style format. | Exporters; published schema | Admin > Backups > Export; per-user export (users map) |
| ADM-075 | Library integrity report | Find files that went missing, appeared untracked or changed since the last scan. | Plex no; Jellyfin no; Immich 3.0 compares disk and database, including checksums | Low: one rival has it | R2 | Built from the file identities the scan already keeps. Full-content checksums are an opt-in job because they read every byte. Duplicate, unmatched and corrupt-file reports belong to the library map. Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Comparison job over the index | Admin > Libraries > Integrity |
| ADM-076 | Point-in-time recovery from the log | Undo a mistake such as a mass "mark unwatched" by going back to a moment. | None | Low: one Plex user needed a 75-post thread to save viewing history from a corrupt database | Later | The append-only log can be replayed up to any time into a scratch cache, and the difference appended as corrections. | Replay to a timestamp; correction events | Admin > Backups > Restore to a point in time |

### Database health and repair

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| ADM-077 | Rebuild the cache instead of repairing it | Fix a corrupt or confused database with one action and lose nothing. | Plex: the third-party PlexDBRepair, or delete the database and recreate the libraries; Jellyfin: restore a backup | High: the PlexDBRepair forum thread has 1,228 posts and 26,794 views; the tool has about 1.8k stars | R1 | SQLite is a cache (ADR 1, decision 5): discard it and rebuild from the files plus the user log. This is only honest once every piece of user state lives in the log. | Rebuild command and job; complete durable state | Admin > Diagnostics > Rebuild; CLI `gunmetal rebuild` |
| ADM-078 | Checksummed user log that recovers from a torn write | A power cut mid-write cannot corrupt history. | None found | Medium: one Plex user needed a 75-post thread to recover history from a corrupt database | R1 | Log records are framed and checksummed. A torn record at the end is cut back to the last good one at startup, and damage elsewhere is reported with what it held, so a backup can fill the gap. | Log format with framing, checksums and a flush policy | Startup page; doctor results |
| ADM-079 | Refuse a data directory on a network filesystem | Stop the most common cause of database corruption before it happens. | Plex warns in its docs; Jellyfin's docs say local storage only; Immich forbids network shares; none was found to detect it | Medium: every rival warns, none enforces | R1 | Detects SMB, NFS, 9p and FUSE under the data directory at startup and refuses with an explanation unless the admin sets a documented override. | Filesystem-type detection per OS | Startup page; doctor |
| ADM-080 | One writer, so no "database is locked" | No intermittent lock errors under load. | Jellyfin added three locking modes as a workaround; Plex and Emby unverified | High: Jellyfin #15101 (229 comments), #11589 (107), #11624 (102) | R1 | One in-process writer serialises all writes and readers use snapshots, and scans write in short batches so readers do not stall. This must be proven under load, because long transactions can still hurt. | Writer task; write-ahead log mode; batch sizing; load tests | Admin > Diagnostics (write queue depth) |
| ADM-081 | Optimise only when needed | The database stays fast without a weekly ritual. | Plex has a weekly optimise task; Jellyfin an "Optimize Database" task | Low | R2 | Mostly parity. The task measures how much it could reclaim first and does nothing if there is nothing to gain. Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Optimise task with a threshold | Admin > Tasks |
| ADM-082 | Bounded growth with size accounting | The data directory never balloons, and you can see what uses the space. | Plex's config can reach hundreds of GB, and PlexDBRepair shrank one database from 31 GB to 206 MB; Jellyfin's database can reach 10 to 100 GB | High: database bloat and size complaints for Plex and Jellyfin | R2 | Data that grows with time (sessions, activity, metrics) has retention limits, and the dashboard shows settings, log, cache and temporary data separately. Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Retention jobs; size accounting | Admin > Dashboard storage card |
| ADM-083 | Free-space guard | A full disk produces a warning, not a broken database. | None found | Low: listed among the research's doctor checks | R1 | Backups, snapshots and, later, transcode scratch space check free space first and refuse cleanly; thresholds raise alerts. | Free-space checks; alert rule | Dashboard; Admin > Alerts |
| ADM-084 | External database engines (PostgreSQL, MySQL) | Run the index on a separate database server. | Plex: SQLite only (request 243 votes); Jellyfin: MySQL request 663 votes, PostgreSQL likely first; Emby: request with 391 replies; Immich requires Postgres | High: 663 votes on Jellyfin | No | Not planned. The demand mostly comes from locking and corruption pain, which ADM-077, ADM-079 and ADM-080 answer, and a second engine doubles the surface the mutation gate must cover. See Open decisions. | None | None |

### Storage and library roots

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| ADM-085 | An offline root never deletes anything | See LIB-032, which owns this feature. | Plex: a trash model, and its move guide says to turn off automatic emptying; Jellyfin's docs warn that tasks can remove items while storage is offline (manual purge requested, #3494) | High: both rivals document the risk | R1 | See LIB-032. | None beyond LIB-032. | Admin > Libraries; greyed-out items in clients |
| ADM-086 | Trash with a grace period and explicit purge | See LIB-033, which owns this feature. | Plex: trash with "empty after every scan"; Navidrome keeps missing files with a never, always or after-full-scan purge policy and CSV export | Medium: the same storage-outage evidence | R1 | See LIB-033. | None beyond LIB-033. | Admin > Libraries > Trash |
| ADM-087 | I/O profile per root | Scans that are gentle on network shares and cloud drives. | None; Jellyfin's docs tell cloud-drive users to turn off image extraction | Medium: an rclone forum thread on Plex scans hitting Google Workspace download limits has 36 posts | R2 | Each root is marked local, network share or cloud drive. The profile caps parallel reads, forbids whole-file reads during scans and keeps frame extraction off for remote roots. | Per-root read scheduler | Admin > Libraries > Root settings |
| ADM-088 | Scans that report bytes read | Proof that a scan read headers, not whole files. | Plex and Jellyfin probe and thumbnail steps read whole files on cloud drives | Medium: the rclone forum threads on scan quotas | R1 | The pure-Rust parsers read tags, headers and index data only, and the scan report shows bytes read per root. Files that would need a full read to index are flagged and handled lazily, which matters for video from R2. | I/O accounting in the reader layer | Dashboard scan card; Admin > Libraries |
| ADM-089 | Media mounted read-only | The server never needs write access to your media. | Plex keeps metadata in its own directory (unverified); Navidrome documents a read-only music folder | Medium: safer network shares and a smaller attack surface (ops and library research) | R1 | Nothing in Gunmetal writes into media folders by default, and the doctor confirms the server works with read-only mounts. Whether sidecar writing is ever offered is a library-map decision. Media is read-only unless the owner enables deletion on that root (ADM-139, Later). | No write paths into roots by default | Install docs; doctor |
| ADM-139 | Delete media from the UI (owner) | The owner removes a bad file from the app instead of opening a file manager | Plex and Jellyfin allow deletion from the UI (research) | Medium: users of both rivals expect it (no vote count in the research) | Later | Off by default. It needs an opt-in writable root per library; deletion goes to a trash with a grace period and an undo, is logged with who did it, and ACC-046 delegates it to trusted users. Every other root stays read-only | Per-root writable flag; trash with grace period; audit event | Item menu (owner); Admin > Libraries > Roots |
| ADM-090 | Separate locations for settings, log, cache and scratch space | Keep the small irreplaceable data on redundant storage and the large rebuildable cache on a fast disk. | Plex and Jellyfin keep one large data directory (hundreds of GB and 10 to 100 GB) | Low | R1 | Each area has its own configurable path and its size on the dashboard, because only the log and settings need backing up. | Path settings; size accounting | Admin > Settings > Storage; install docs |
| ADM-091 | Filesystem checks and tuning guidance | The right settings for ZFS, and a warning about slow disks. | Jellyfin documents ZFS record sizes and warns against SMR drives | Medium: Jellyfin on Proxmox LXC with ZFS hits lock errors (#15101, 229 comments) | R2 | The doctor reports the filesystem under each data path and links the matching guidance. The guidance itself is parity. Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Filesystem detection | Doctor; docs |
| ADM-092 | Copy or sync media between servers and drives | Fill a second server or a portable drive from the server. | Emby Premiere offers folder sync | Low | No | Gunmetal does not manage media files. Copying is a job for rsync and similar tools, and keeping media read-only (ADM-089) is worth more. | None | None |

### Scheduled and background work

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| ADM-093 | One task list with run, cancel and history | See and control maintenance work. | Plex: a Scheduled Tasks page; Jellyfin: any task can be run at once; Immich: job queues | Low: Jellyfin's request for visibility of running work has 9 votes | R1 | Few tasks exist, because there is no per-file probing or image extraction to maintain: backup, rescan, trash purge, optimise and analysis. Each shows progress, last run, duration and errors, and can be cancelled. | Task registry; progress events | Admin > Tasks |
| ADM-094 | Maintenance window and per-task schedules | Heavy work happens at night, at times you choose. | Plex: a maintenance window, 02:00 to 05:00 by default; Navidrome: cron expressions | Low | R2 | Parity. | Scheduler with a window and cron expressions | Admin > Tasks > Schedule |
| ADM-095 | Heavy jobs that are throttled and resumable | Loudness analysis of a large untagged library does not knock over a small server. | Jellyfin 10.9 ran many ffprobe processes at about 700 MB each and exhausted a 4 GB Pi 4 (#11588); Jellyfin 12.0's "Scan All Libraries" leaked memory (#17860) | High: #11588 has 129 comments | R1 | One low-priority queue for loudness analysis and, later, fingerprinting or chapter images. It checkpoints progress, survives restarts and yields when playback needs the disk. | Job queue with checkpoints, priority and an I/O budget | Admin > Tasks |
| ADM-096 | Concurrency limits and a pause switch | Tune background work to the machine, or stop it for an evening. | Immich sets concurrency per job type; others unverified | Low | R2 | Parity on limits, plus one "pause background work until" control. Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Concurrency settings; global pause | Admin > Tasks |
| ADM-097 | Switch whole modules off | A household with no video or no TV tuner never sees, or exposes, those parts. | Jellyfin users ask to turn off live TV (no count); others unverified | Low | R2 | A disabled module registers no routes and runs no jobs, which also shrinks the attack surface. Compatibility adapters ship switched off. | Module registry; conditional route registration | Admin > Settings > Modules |
| ADM-098 | Let the host sleep, and wake it for recordings | A low-power server can sleep without missing a recording. | Plex will not wake the machine (request 210 votes); Emby has a wake option; Jellyfin unverified | Medium: Plex's 210-vote request | R3 | The server blocks sleep only while it is playing, scanning or backing up, and sets an operating-system wake timer before the next recording. Platform support for wake timers is unverified. | Sleep inhibitor and wake timer per OS | Admin > Settings > Power |

### Admin dashboard and activity

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| ADM-099 | Now playing | Who is listening or watching, on which device, right now. | Plex yes, free; Jellyfin yes; Emby yes; Navidrome redesigned its panel in 0.62 | High: every rival has it, and Tautulli and Tracearr exist to show more | R1 | Parity on the list itself; the edge is ADM-100. Owns live sessions; ACC-072, MUS-190 and VID-171 point here. | Session registry; event stream | Admin > Dashboard; Admin > Sessions |
| ADM-100 | Playback decision and its reason, per session | See in plain words why each stream plays as it does. | Plex and Jellyfin show some detail (depth unverified); Emby unverified | High: Jellyfin's "stats for nerds" request has 219 votes; Plex's "default to max quality" request has 1,289 | R1 | The decision engine in the core emits a structured reason (for example "direct: original file" or "cannot play: this browser cannot decode the file's audio"), which is stored per session and shown as written. | Structured reasons from the core decision engine; per-session record | Admin > Sessions; session detail |
| ADM-101 | Direct play, remux and transcode breakdown | A chart of how hard the server works for video, and why. | Plex through Tautulli; Jellyfin through Streamystats or Tracearr; Tracearr across all three | High: Tracearr has 2,680 stars, and the breakdown is the proof of Gunmetal's main claim | R2 | It sits on the admin home, built from the per-session decision records, so the direct-play claim is visible to every admin rather than asserted. | Aggregation over decision records | Admin > Dashboard; Admin > History |
| ADM-102 | Stop a session with a message | End a stuck or unwanted stream and tell the viewer why. | Plex unverified (Tautulli can do it); Jellyfin request #301 has 402 votes and is open | High: 402 votes | R1 | Free. Owns stopping a stream; ACC-073, VID-172 and INT-136 point here. New requests fail at once and an in-flight response is cut by the ACC-122 mechanism. | Session revocation; message delivery to clients | Admin > Sessions > Stop; notice in the client |
| ADM-103 | Message all users and schedule maintenance notices | Warn everyone before a restart or an upgrade. | Plex: "Send server messages" request, 1,206 votes; Jellyfin: per-session messages only, broadcast requested (89 votes); Emby: maintenance notice requested (119 replies) | High: 1,206 votes on Plex | R2 | Notices with a start and end time appear as a banner in every Gunmetal client, and an admin-started restart or upgrade can post one automatically. Owns messages to users; ACC-074 points here. | Notice store with schedule and expiry; push over the event stream | Admin > Notices; client banner |
| ADM-104 | Bandwidth, live and over time | See local and remote traffic now and historically. | Plex: Plex Pass; Jellyfin: not built in (unverified); Tautulli | Medium: Plex's per-user upload-limit request (188 votes) shows admins manage upload closely | R2 | Parity with Jellyfin on price. The edge: the server sends the original bytes, so per-session bitrate is measured exactly and split by local and remote. | Per-session byte counters; time series with retention | Admin > Dashboard; Admin > Sessions |
| ADM-105 | CPU, memory, disk and network charts | Server health at a glance. | Plex: Plex Pass on the web dashboard, also on mobile and TV since July 2026; Jellyfin: no (request 41 votes) | Medium: Jellyfin's 41-vote request; Plex added it in 2026 | R2 | Parity with Jellyfin on price. The edge: Gunmetal's own process is shown apart from the whole machine, so any admin can check the low-footprint claim. | Process and host metrics sampler | Admin > Dashboard |
| ADM-106 | Play history and top users and titles | Who listens and watches most, and what. | Plex: Plex Pass; Jellyfin: a plugin or Jellystat, which needs its own Postgres; Tautulli has about 6.6k stars | High: Tautulli and Jellystat (about 2.5k stars) exist to fill this; Jellyfin's statistics request has 36 votes | R2 | Computed from the append-only log, so there is no second database and no polling helper. What an admin may see of other users follows the users map's privacy rules. Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Queries over the log; privacy filter | Admin > History |
| ADM-107 | Library and storage statistics | Item counts, sizes and disk use. | Jellyfin added storage space and item counts in 10.11; others unverified | Low | R2 | Parity, plus Gunmetal's own data areas (ADM-082). Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Counts from the index; filesystem statistics | Admin > Dashboard storage card |
| ADM-108 | Health of each library root | Which roots are online, when each was last scanned and what failed. | None found | Medium: the storage-outage evidence under ADM-085 | R1 | Per root: online or offline, filesystem type, I/O profile, last scan, bytes read, unreadable files and free space. | Root status model | Admin > Libraries; dashboard |
| ADM-109 | Health summary on the admin home | One card that says whether anything needs attention. | Not found as one view in any rival | Medium: it combines items admins ask for separately (backups, updates, storage) | R1 | Brings together backup age, update and advisory status, doctor warnings, offline roots and failed tasks, each linking to its fix. | Aggregated health model | Admin > Dashboard |
| ADM-110 | Activity and audit log | Sign-ins, failed sign-ins, new devices and every admin change. | Jellyfin has an activity log; Plex and Emby unverified | Medium: the users research found no complete sign-in and admin audit log anywhere | R1 | Append-only and attributed to the device key that acted; clearing it is itself recorded. Owns the audit log; ACC-078 points here. | Audit events with bounded retention | Admin > Activity |
| ADM-111 | Retention and anonymisation of activity | Keep only what you need about the people who use your server. | Jellyfin has a "Clear Activity Logs" task; anonymising is requested (#1161, 33 votes), as is deleting logs (#2444) | Low: 33 votes | R2 | Retention per record type, and IP addresses stored truncated or not at all. Security events and viewing history have separate settings. Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Retention jobs; IP truncation | Admin > Settings > Privacy |
| ADM-112 | Restart and shut down from the UI | Apply a change or stop the server without a shell. | Rivals unverified | Low | R1 | Parity, with an optional automatic notice to users first (ADM-103). | Supervised restart | Admin > Dashboard menu |
| ADM-113 | Emergency page served by the server | Check status, read logs, take a backup or restart even if the main client will not load. | None found | Low: an open question in the research | R1 | A minimal page rendered by the server without the client bundle, so a broken client release cannot lock the admin out. | Server-rendered status, logs, backup and restart | Emergency page |
| ADM-114 | Admin dashboard on phones and TVs | Check the server from the sofa. | Plex: mobile, Fire TV and a tvOS preview since July 2026; Swiftfin (a Jellyfin client) has an admin dashboard and backups | Medium: Plex shipped it in 2026 and Swiftfin added it | R2 | The dashboard is part of the single React Native codebase (ADR 1, decision 8), so every native client gets it rather than a separate app. | The same APIs as the web dashboard | Mobile and TV admin screens |
| ADM-115 | Monitor several servers in one place | One dashboard for a home server and a cabin server. | Plex's mobile dashboard has a server picker; Jellyfin no | Low: the nearest signal is Jellyfin's multi-server request (177 votes) | Later | A client that already holds keys for several servers shows their health cards together; the servers need not know about each other. | None beyond each server's own APIs | Client server list with health cards |

### Alerts

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| ADM-116 | Admin alerts with free destinations | Hear about failed backups, offline roots, low disk space, security advisories and failed tasks. | Plex: backup and corruption webhook events (Plex Pass); Jellyfin: email overhaul requested (#3559, 47 votes), update notifications requested (51 votes); Immich has built-in email | Medium: Jellyfin requests of 47 and 51 votes | R1 | In R1, alerts are delivered in the app only. External destinations (ntfy, a webhook, and email through INT-046) arrive in R2 with the webhook system (INT-030), grouped and rate-limited, with no paywall. Admin destinations may point at the LAN; plugin traffic may not. | Alert rules in R1; outbox and destinations shared with the integrations map from R2 | Admin > Alerts; dashboard |
| ADM-117 | Alert inbox with acknowledge and snooze | One place to see what went wrong and mark it handled. | None found | Low | R2 | Alerts stay until acknowledged or resolved, and resolved ones close themselves. | Alert state | Admin > Alerts |

### Logs, diagnostics and monitoring

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| ADM-118 | Log viewer in the UI | Read logs without a shell. | Jellyfin yes ("go to end" #3912 and a delete button #2444 are requested); Emby has a log page; Dispatcharr has an in-app log browser | Medium: Jellyfin's follow-up requests | R2 | Filter by level, module, session and request ID, follow live, and jump from any dashboard event to its log lines. | Log store indexed by request and session; live tail | Admin > Logs |
| ADM-119 | Structured logs with rotation | Logs that machines and people can both read, which never fill the disk. | Rivals unverified | Low | R1 | JSON or plain text to standard output, the system journal or files, with size-based rotation and retention. | Logging backend; rotation | Admin > Settings > Logging |
| ADM-120 | Log levels per module, with automatic revert | Turn on detail for one part while debugging, and have it switch itself off. | Plex: debug logging on by default plus a verbose switch; Immich: log level in admin settings; Navidrome: in config | Low | R2 | Change at runtime per module, with a timer that turns verbose logging off again. | Runtime filter reload | Admin > Logs > Levels |
| ADM-121 | Secrets that cannot reach logs | See ACC-123, which owns this feature. | Plex logs tokens only when a debug switch is on; Navidrome wrote admin passwords to its logs on failed setup until 0.64.2 | Medium: the Navidrome fix shows how easily this goes wrong | R1 | See ACC-123. | None beyond ACC-123. | None (behaviour) |
| ADM-122 | Sign-in failure lines for fail2ban | Block password guessing and pairing abuse at the firewall. | Jellyfin: requested (#3541); none found built in | Low: one Jellyfin request | R1 | A stable, documented line for every failed sign-in or pairing, with the client address resolved only through listed trusted proxies. A sample fail2ban filter ships in the docs. | Sign-in failure events; trusted-proxy resolution | Docs; Admin > Logs |
| ADM-123 | `gunmetal doctor` | One command, or one button, that finds problems and says how to fix them. | Navidrome has a built-in `doctor`; Plex relies on the third-party PlexDBRepair; Jellyfin has none | High: the PlexDBRepair thread (1,228 posts); Jellyfin's hardware-test request (423 votes) | R1 | Checks database and log integrity, the data directory's filesystem, free space, root reachability and permissions, clock sanity, config validity and listening addresses. R2 adds FFmpeg, sandbox and hardware-encode checks (ADM-132, ADM-133). Each finding links a fix. | Check registry; results model | Admin > Diagnostics; CLI |
| ADM-124 | Diagnostic bundle with preview and masking | Send one file when asking for help, knowing exactly what is in it. | Plex: a download-logs action (unverified); Jellyfin: individual files (unverified) | Medium: the research asks for diagnostics that collect no titles or identities | R1 | One archive with logs, doctor output, versions, platform and settings with secrets removed. The admin sees every file before saving and can mask titles, usernames and paths. | Bundle builder; masking pass | Admin > Diagnostics > Bundle; CLI |
| ADM-125 | File inspector | See exactly how the server read a file, and why it failed. | Navidrome has `inspect`; Jellyfin shows full file information | Low | R1 | Shows parsed tags, artwork, gapless and loudness values, and any parse error with its byte offset, since the core parsers return typed errors with a location. Owns the file inspector; LIB-195 points here, and MUS-114 is the listener-facing track info sheet built on the same inspect API. | Inspect API over the core parsers | Admin > Diagnostics > Inspect a file; CLI |
| ADM-126 | Session trace | Find out why one playback stalled or failed. | Plex and Jellyfin show playback information (detail unverified) | Medium: Jellyfin's "stats for nerds" request (219 votes) | R2 | A short-lived timeline per session that joins the server's decision and bytes sent with the client's buffering and error reports, by session ID. | Trace store with short retention; client event reporting | Admin > Sessions > Trace |
| ADM-127 | OpenMetrics endpoint with a scoped token | Plug the server into Prometheus and Grafana. | Jellyfin has had an exporter since 2020 (request 56 votes; improvement request 63 +1); Navidrome has an opt-in endpoint; Plex none found | Medium: 56 votes and 63 +1 | R2 | Opt-in, read with a metrics-only token rather than an admin key, and includes playback decisions, scan bytes read and task queue depth. Owns metrics; INT-135 points here. | Metrics registry; scoped token | Admin > Diagnostics > Metrics |
| ADM-128 | Health endpoints for containers | Orchestrators restart the server only when it is really unhealthy. | Rivals unverified | Low | R1 | Separate liveness and readiness answers, so a long rebuild is not mistaken for a crash. | Health handlers | Docs |
| ADM-129 | Network activity page | See every outbound connection the server made, and why. | None found | Medium: privacy is the second reason to self-host; Plexamp's telemetry cannot be turned off | R1 | All outbound traffic passes one gate (ADM-028), so the page lists each destination, the feature that caused it, counts and last time. | Egress gate logging | Admin > Settings > Network and privacy |
| ADM-130 | Local crash records | A crash leaves evidence for the admin and the diagnostic bundle. | Rivals unverified | Low | R1 | The core is held to "no panics on any input", but if the server process does fail, a record with the version and backtrace, and no user data, is kept locally and included in the bundle. | Crash handler that writes local records | Admin > Diagnostics |
| ADM-131 | Opt-in crash report sending | Help the project fix crashes without sharing identities. | Navidrome sends anonymous usage data by default; Plexamp sends telemetry with no off switch | Medium: without it the project cannot see what breaks (research risk) | Later | Each report is shown in full and sent only when the admin agrees to that report; titles and usernames are excluded unless ticked. | Report receiver (project side); preview | Admin > Diagnostics |
| ADM-132 | Transcoding self-test | Know the transcode path works before a viewer hits an error. | Jellyfin: planned (#450, 423 votes), plus a transcode test request (#223, 40); Plex: none found | High: 423 votes | R2 | Runs a short encode inside the same sandbox real transcodes use, at startup and on demand. In R2 it tests the software encoder only; hardware encoders are tested once VID-006 ships. No licence check is involved. | Sandboxed test encode; capability store | Admin > Diagnostics; doctor |
| ADM-133 | Sandbox self-test and an honest "transcoding unavailable" | Know whether transcoding can run safely on this kernel. | No rival sandboxes FFmpeg; Jellyfin had FFmpeg argument-injection advisories in 2023, 2025 and 2026 | Medium: that security record | R2 | If the sandbox cannot start, for example on an old NAS kernel, transcoding is reported unavailable with the reason and never runs unsandboxed. | Sandbox probe at startup | Dashboard capability notice; doctor |
| ADM-134 | Remote connectivity check | Know whether devices away from home can reach the server, and how. | Plex users have long remote-access threads (635 posts in November 2025; 263 in August 2026) | High: those Plex threads | R2 | No ports are opened, so the check tests the iroh path instead: direct or relayed, and measured throughput, from the server and from a paired client. | Connectivity probe; client report | Admin > Diagnostics > Connectivity |
| ADM-135 | Ship logs to syslog or a log collector | Send logs to a central system. | Jellyfin: requested (#343, 6 votes) | Low: 6 votes | Later | Parity once built. | Log forwarder | Admin > Settings > Logging |
| ADM-136 | Usage telemetry on by default | Anonymous usage statistics sent to the project. | Navidrome: on by default with an opt-out; Plexamp: download telemetry with no off switch; Jellyfin: none | Medium: demand runs against it; privacy is a top reason to self-host | No | Not doing. Nothing leaves the server unless the admin turns it on (ADM-028). | None | None |

### Several servers

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| ADM-137 | History shared between your own servers | Watched or played on one of your servers means the same on all of them. | Plex: account sync since 2022, excluding managed users and music and depending on plex.tv; Jellyfin: no (#716, 105 votes) | Medium: 105 votes, and 177 for multi-server support (#47) | Later | The append-only log can be copied between a user's servers over iroh and merged by event time, with no central service. It needs cross-server identity and clear merge rules first. | Log replication; merge rules; cross-server identity | Admin > Servers; user settings |
| ADM-138 | Edge or caching servers | A second server near remote viewers caches popular files. | Nobody; Jellyfin request #505 (30 votes) | Low: 30 votes | No | Not doing. Synced libraries and offline downloads cover travel, and a caching tier would add a replication system for little demand. | None | None |

## Differentiators

1. **Backups that are tiny, daily and free, with restore from the welcome
   screen** (ADM-065, ADM-072, ADM-029 in R1; encryption, ADM-068, in R2). Plex backs up only its core
   database every three days and leaves out settings; Jellyfin's backups are
   manual and its scheduling request has 91 votes; Emby charges for them.
   Because Gunmetal's database is a rebuildable cache, the backup is only the
   user log, settings and keys, so it can run daily by default, be
   verified after every run and be restored on a fresh install in minutes.
   This is the clearest answer to the Plex backup and move complaints.
2. **Upgrades that can always be undone** (ADM-056, ADM-057, ADM-058,
   ADM-059, ADM-060). Plex's 1.43.0 packaging failure, Jellyfin's 10.11
   migration failures and Immich's refusal to downgrade show that every rival
   treats an upgrade as one-way. Gunmetal snapshots before migrating, can
   check a migration before serving, and lets an older binary rebuild its
   cache from the files and log. Admins who were burned by an upgrade will
   notice.
3. **Moving in from Plex, Jellyfin, Emby or Navidrome is built in, with a
   dry run and undo** (ADM-036 to ADM-046, ADM-051). Today this takes
   WatchState, JellyPlex-Watched or one-off scripts, and Plex's own sync
   leaves out music. Reading offline database copies needs no account on the
   old server, and every imported fact can be backed out. Switching cost is
   what keeps lifetime Plex Pass holders in place. R1 ships the file-based
   importers (M3U playlists, Last.fm and ListenBrainz exports, ADM-042 to
   ADM-044); the readers for rival databases, the dry run and undo
   (ADM-036 to ADM-041, ADM-045, ADM-046) arrive in R2, because each rival
   schema has several versions to support.
4. **A free dashboard that explains every stream** (ADM-099, ADM-100 and
   ADM-102 in R1; ADM-101, ADM-103 and ADM-106 in R2). Plex puts everything past "now playing"
   behind Plex Pass; Jellyfin users have asked for years to stop a stream
   (402 votes), and Plex users to message everyone (1,206 votes); Tautulli
   and Jellystat exist to fill the gap. Gunmetal shows the playback decision
   and its reason for each session, stops streams for real and computes
   history from the log it already keeps. The direct-play breakdown also
   proves the project's main claim to every admin who opens the page.
5. **Nothing to repair** (ADM-077, ADM-079, ADM-080, ADM-123). PlexDBRepair's
   1,228-post forum thread and Jellyfin's 229-comment "database is locked"
   issue are symptoms of databases that hold irreplaceable data and are
   written from many places. Gunmetal keeps one writer, refuses network
   filesystems for its data, and fixes a bad cache by rebuilding it, with
   `gunmetal doctor` in place of third-party repair tools.
6. **A NAS outage never empties the library, and scans do not hammer
   shares** (LIB-032, which ADM-085 points to, and ADM-088; per-root I/O
   profiles, ADM-087, in R2). Jellyfin's docs warn that tasks can
   remove items while storage is offline, and Plex scans have exhausted cloud
   drive quotas. Gunmetal treats a missing root as offline, never as deleted,
   and its scans read headers only and report the bytes they read.
7. **You own it from the first second** (ACC-001, ACC-002, ADM-028,
   ADM-053). No vendor account, a setup code that stops anyone else claiming
   the server, and nothing contacting the internet until the admin allows
   it. Plex's local-authentication request (372 votes) and its July 2026
   outage are the contrast.

## Deliberately not doing

- **External database engines (ADM-084).** SQLite is a rebuildable cache
  with one writer. Jellyfin's 663-vote MySQL request mostly reflects locking
  and corruption pain that this design is meant to remove, and a second
  engine would double the surface held to the coverage and mutation gate.
- **Copying or syncing media between servers or drives (ADM-092).** Gunmetal
  reads media and does not manage it; general file tools do this better, and
  read-only media is a security property worth keeping.
- **Telemetry on by default (ADM-136).** Nothing leaves the server unless
  the admin turns it on. Crash reports, if they come, are opt-in per report
  (ADM-131).
- **Edge or caching servers (ADM-138).** Little demand, and offline sync
  covers the travel case.
- **Container tags that download a server at start.** The image you pull is
  the version you run (ADM-003).
- **Updates that install themselves by default.** Automatic updates are
  opt-in and must be able to roll themselves back (ADM-064).
- **A project kill switch.** Emby stopped compromised servers from starting
  in 2023. Gunmetal's project will never be able to disable someone's server
  remotely; the advisory banner (ADM-054) is the only channel.
- **Emailing owners about security problems.** Plex can do this because it
  has accounts. Gunmetal has none and will not add them for this; the signed
  feed and banner replace it.
- **A "trust the local network" admin sign-in, or setup without a code.**
  Passwordless local admin access combined with forged headers is how Emby
  servers were compromised in 2023.
- **Packages that block downgrades.** Rollback depends on being able to
  install the previous version (ADM-016, ADM-059).
- **Flatpak and Snap packages for the server.** Emby offers them, but the
  container image and native packages cover the same machines, and there is
  no demand evidence for them in the research.

## Dependencies and risks

- **Blocking: ADR 3, durable user state.** ADR 1 (decision 5) names watch
  history as the only irreplaceable data, but most rows in this map that
  write user data (playlists, loves, ratings, corrections, layouts, shares,
  identities, IDs, settings) need it to survive a cache rebuild. ADR 3 must
  be accepted before any server work that stores user data. It defines the
  two durable stores every map now names the same way: **the user log** and
  **the identity store** (see the [feature map README](README.md)).
- **A new ADR on durable state is a precondition.** ADR 1 names watch history
  as the only irreplaceable data. Small backups (ADM-065), rebuild instead of
  repair (ADM-077), rollback (ADM-059) and undoable imports (ADM-046) are
  only true if ratings, playlists, curation, users, passkey public keys,
  device keys, sharing grants, settings, certificates and server identity
  keys also live outside the SQLite cache. If any of them ends up only in
  SQLite, these features quietly become false.
- **First-run on a LAN address needs an ADR of its own.** WebAuthn needs a
  secure context and a domain, not an IP address. R1 has only a web client,
  so the most common setup (a headless box reached by IP) cannot register a
  passkey without one of: localhost, the admin's own domain (ADM-022), a
  project-issued name (ADM-023), or a weaker fallback. Native-app enrolment
  (ADM-035) solves it only from R2. Whether WebAuthn accepts a self-signed
  certificate after a browser warning was not verified.
- **Log format discipline.** Rollback by starting an older binary assumes
  the log only grows by new, versioned event types that older readers can
  skip. Some new event types, such as import retractions, change meaning
  when ignored; an older version that skips them would show undone imports
  again. Release notes must say this plainly (ADM-060), and some releases
  will not be safe to roll back past.
- **Rebuild time is the price of a rebuildable cache.** A restore or
  rollback means a rescan. That is quick for music but could take hours for
  a large video library on network storage. The scan benchmark must report
  rebuild time, and the optional cache snapshot (ADM-067) is the fallback.
- **Importers read undocumented schemas.** Plex, Jellyfin, Emby and
  Navidrome can change their databases in any release (Jellyfin twice in a
  year; Navidrome re-encoded every ID in 0.64). Importers need fixtures from
  many versions and must read copies, never a running server's live files.
- **Content identity belongs to the library map.** Matching imports, keeping
  history across renames and the integrity report all depend on the file
  identity the library map defines. A wrong match silently moves someone's
  history onto the wrong file.
- **Folder browsing at setup is an attack surface.** Jellyfin published a
  critical path-traversal advisory in library-folder management in
  September 2026. The picker in ADM-025 must be admin-only, limited to
  allowed base paths, and covered by the route-authorisation tests.
- **Alerts and webhooks share plumbing with the integrations map.** Admin
  destinations may target the LAN, while plugin traffic to the LAN is
  blocked by default; the two rules must be different and visible.
- **What the dashboard shows about other users depends on the users map.**
  History, top users and session detail (ADM-106, ADM-126) must follow
  whatever the privacy decision says an admin may see.
- **Music playback decisions depend on the music map.** If R1 includes an
  Opus encoder for mobile data, an encoder must be bundled or sandboxed,
  which changes the "no FFmpeg for music" statement in ADM-001.
- **The transcode sandbox varies by kernel.** Some NAS kernels are old. When
  the sandbox cannot start, transcoding is off and says why (ADM-133), which
  may surprise people who expect Plex-like behaviour.
- **Without telemetry the project is blind.** It will not know which
  versions are deployed or which upgrades fail. The opt-in update check and
  opt-in crash reports are the only substitutes, so release testing has to
  carry more weight.
- **Packaging is a long tail.** Every OS, package repository and NAS catalog
  has its own signing and review rules; Plex's January 2026 signing failure
  shows a large company getting it wrong. Each target added is a target
  tested on every release.
- **SQLite still needs care.** One writer removes internal lock contention,
  but long transactions during scans can still stall readers, and
  environment-specific problems (ZFS, LXC) are hard to reproduce. Load tests
  have to cover them.
- **Release signing keys become critical infrastructure.** The update feed,
  advisory banner, container tags and package repositories all rest on
  them.

## Open decisions for the project owner

1. **Which user state is irreplaceable, and how is it stored?** Recommendation:
   write the durable-state ADR before the server's data model. Treat as
   durable everything a rescan cannot recreate (watch and listen history,
   ratings, playlists, curation, users, credentials' public keys, device
   keys, grants, settings, certificates, server identity), stored as typed
   streams in one log format with one backup, export and replay path.
2. **How does R1 let a browser register a passkey on a server reached by IP
   address?** Options: localhost only, the admin's own domain, an opt-in
   name service on gunmetal.tv (like plex.direct), or a password fallback.
   Recommendation (revised by the feature map README): ship localhost and
   own-domain HTTPS in R1, and let the first admin set a password when the
   setup page is not a secure context (ACC-002, with ACC-052 strength rules
   and the ACC-063 limiter), prompting for a passkey once HTTPS exists.
   Keep ADM-023 at Later: a gunmetal.tv name service conflicts with ADR 1
   decision 7 (no central account) and decision 9 (Cloudflare hosts only the
   docs and landing page), so it needs its own ADR first. Some routers block
   DNS names that resolve to private addresses (unverified), so any such
   service would need a documented fallback.
3. **Is the update check opt-in or opt-out?** Recommendation: an explicit
   choice at setup with nothing preselected, plus a plain statement that the
   request carries no identifiers but that the host serving gunmetal.tv sees
   the requesting address. The advisory banner depends on it, which is
   argument enough for most admins to say yes.
4. **No telemetry at all, or opt-in crash reports?** Recommendation: no
   usage telemetry, ever. Build opt-in, per-report crash sending (ADM-131)
   after R1, with the full report shown before sending.
5. **What ships at R1 for installation?** Recommendation: static binaries for
   Linux (x86-64, AArch64), Windows and macOS, plus the container image, in
   R1, with systemd integration only (ADM-005). ARMv7 waits for R2 because
   wasmtime support for 32-bit ARM is unverified (ADM-004). Signed deb and rpm repositories, the Windows and macOS
   installers, FreeBSD and NAS catalog templates in R2. Native NAS vendor
   packages later.
6. **Which importers ship at R1, and how do they read data?** Decided in the
   feature map README: R1 imports only M3U playlists and Last.fm and
   ListenBrainz export files. The Plex, Jellyfin, Emby and Navidrome
   database readers, iTunes XML, the dry run and undo move to R2, because
   each rival schema has several versions to maintain. Reading
   over a rival's API comes in R2 behind an explicit network grant. Reading
   another product's database format for interoperability, and naming those
   products in the import screens, is believed to be acceptable (unverified;
   this is not legal advice and should be checked once).
7. **What can an admin see about other users by default?** This is shared
   with the users map. Recommendation: live sessions, aggregate counts and
   security events by default; per-title history of other adults only if
   the household turns it on; every user can see what the admin can see.
8. **Where does the admin dashboard live?** Recommendation: the full
   dashboard in the React Native client, so it reaches phones and TVs in R2,
   plus the minimal server-rendered emergency page (ADM-113) for when the
   client is broken.
9. **What rollback guarantee does the project make?** Recommendation:
   upgrading from any older version always works (ADM-058). Rolling back is
   guaranteed to the previous minor version and stated for every release
   (ADM-060). Never ship a package that blocks a downgrade.
10. **Confirm that external database engines are out (ADM-084).**
    Recommendation: confirm "No" for R1 to R3 and revisit only if large
    multi-user installs show SQLite limits that one writer and a rebuildable
    cache do not solve.
11. **Are off-site backup destinations part of the server or a plugin?**
    Recommendation: in the server from R2, limited to S3-compatible storage
    and WebDAV, always encrypted, because backups are core to the project's
    promises. Other destinations can be plugins.
12. **Who holds the release signing keys?** Recommendation: an offline root
    key held by at least two maintainers, short-lived online signing keys for
    the feed, packages and images, and a published rotation and compromise
    procedure before R1 ships.
