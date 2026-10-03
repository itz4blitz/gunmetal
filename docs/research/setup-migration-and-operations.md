# Setup, migration and operations

## Scope

This file covers everything an administrator does with a media server other
than watching or listening: installing it, the first run, sizing the
hardware, upgrading and rolling back, backing up and restoring, keeping the
database healthy, moving in from another server, watching what the server is
doing, reading its logs, its scheduled maintenance, where the files and the
database live (local disks, network shares, cloud drives) and running more
than one server.

Rivals studied:

- **Plex Media Server**, **Jellyfin** and **Emby**, the three video servers
  Gunmetal is measured against.
- **Navidrome**, the music server in the same weight class as Gunmetal's
  first release (a single binary, SQLite, music first).
- **Immich**, a photo server, used as the reference for onboarding, backups
  and upgrades, which self-hosters often praise.
- The third-party tools people install to fill gaps: Tautulli, Jellystat,
  PlexDBRepair, JellyPlex-Watched and WatchState.

Versions current on 2026-10-02: Jellyfin 12.1 (15 September 2026, a week
after 12.0), Navidrome 0.64.2 (24 September 2026), Immich 3.2.4 stable with
3.3.0 release candidates, and Plex Media Server on the 1.43 line (exact
current build unverified). The current Emby version was not checked.

How this was researched, and the limits: web search and page fetches were
both available. The plex.tv and support.plex.tv sites refused automated
fetches (HTTP 403), so the content of Plex support articles comes from
search-engine summaries of those pages, and Plex community evidence comes
from the Plex forum's public JSON. Reddit refused fetches entirely. Most of
Emby's knowledge base could not be retrieved, so many Emby cells say
"(unverified)". The session's web-search quota ran out partway through; later
facts come from direct fetches of official docs, release APIs, GitHub issue
searches and forum JSON. Vote counts on features.jellyfin.org were read
through its API on 2026-10-02; the board's "status" labels are not always
kept current (the backup request still says "started" although backup
shipped in 10.11).

Out of scope here, because sibling research files cover them: sign-in and
remote access, transcoding and playback internals, client apps. They appear
only where they change an operational task.

## Feature inventory

"Yes" and "No" mean the feature is built in and free unless the cell says
otherwise. "(unverified)" means it could not be confirmed from a source
during this research.

### Installation and packaging

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Desktop installers | Install on Windows or macOS without a terminal | Yes | Yes, Windows and macOS are in the install docs | Yes, Windows and macOS on the download page | Tie between the three | Navidrome ships an MSI in its release assets. Immich has no native installer and needs Docker. |
| Linux package repositories | `apt` or `dnf` install and update with signature checks | Yes, but release 1.43.0 broke repository signing on Debian and RHEL in January 2026 | Yes, for Debian, Ubuntu and other distributions | Linux packages on the download page; repository details unverified | Jellyfin: official repositories, no recent signing incident found | Navidrome attaches deb and rpm files to each release; a hosted repository was not confirmed (unverified). |
| Single self-contained binary | Copy one file, run it | No | Partial: "portable" Linux builds are listed; whether they bundle the .NET runtime was not checked (unverified) | Unverified | Navidrome: one Go binary per OS and architecture, also packaged as deb, rpm, msi and zip | Navidrome still needs FFmpeg installed on the host. |
| Official container image | One `docker run` or compose file | Yes, `plexinc/pms-docker` | Yes, with Docker, Podman and Kubernetes guidance | Yes, Docker is listed | Jellyfin: official image plus Podman and Kubernetes docs | Immich is container-only and runs as a Docker Compose stack of several containers, including Postgres (exact set unverified). |
| Reproducible container tags | Pin a version, or follow a major line without crossing a breaking change | Version tags exist, but the `public` and `beta` tags contain no server; they download and install the latest build each time the container starts | Unverified | Unverified | Immich: an `IMMICH_VERSION` pin plus major-line tags such as `v3` that never cross into a breaking release | Plex's download-at-start tags mean the image does not fix the version that runs. |
| NAS vendor packages | Install from the NAS app centre | Broad: Synology, QNAP, ASUSTOR, Netgear, WD and others (inferred from PlexDBRepair's supported-platform list) | Synology and TrueNAS SCALE are listed | Broadest list found: ASUSTOR, QNAP, Synology DSM 6, 7 and 7.2+, TerraMaster, Thecus, WD, Netgear, OpenMediaVault, unRAID, TrueNAS and FreeNAS | Emby: the most vendors on its own download page | A Synology package can block downgrades (Plex forum thread 840714). |
| FreeBSD and TrueNAS CORE | Run on BSD-based NAS systems | Unverified | Not supported, because .NET does not support FreeBSD | Yes, FreeBSD and FreeNAS are on the download page | Emby | Navidrome users asked for FreeBSD binaries in September 2026 (#6213). Rust builds for FreeBSD, which is an opening for Gunmetal. |
| 32-bit ARM and old single-board computers | Reuse old Raspberry Pi-class hardware | Unverified | Dropped 32-bit ARM in 10.11.0 (October 2025); the docs say most SBCs, the Pi 5 included, are too slow | Unverified | Navidrome: ships 32-bit ARM builds and says it runs well on a Pi Zero | For a music server, a Pi Zero-class box is a realistic target; for video transcoding it is not. |
| Bundled dependencies | Nothing else to install | Ships its own transcoder (unverified) | Ships its own FFmpeg build (unverified) | Bundled (unverified) | Plex, Jellyfin and Emby: no extra installation steps | Navidrome requires a system FFmpeg; Immich requires Postgres and other companion containers (exact set unverified). |
| Service management from the command line | Install and control the system service with one command | The installer sets it up (unverified) | Packages ship service units (unverified) | Unverified | Navidrome: the `service` command installs and controls the OS service | |
| Refuse to run as root | A safe default for a network-facing service | Unverified | Unverified | Unverified | Navidrome: the `EnforceNonRootUser` option (0.62, June 2026) exits at startup when run as root | |
| Config validation | Typos in config are reported instead of silently ignored | Unverified | Unverified | Unverified | Navidrome: 0.64.0 warns about unrecognised config options | |
| Universal Linux packages | Flatpak or Snap install | Unverified | Unverified | Yes, Flatpak and Snap are on the download page | Emby | |
| Managed hosting | Someone else runs the server | No first-party hosting found | No | Hosting partners listed on the download page (Cloudron, Cloudzy) | Navidrome: its install docs point to PikaPods and Zenith | Not a Gunmetal goal, but cheap to list in the docs. |

### First-run setup

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Browser setup wizard | A guided first run | Yes, in Plex Web after signing in to plex.tv | Yes: language, admin account, libraries, metadata language, networking | Yes (steps unverified) | Jellyfin: short, local and needs no outside account | |
| Admin account without a vendor account | Own the server without signing up anywhere | No: the server is claimed by a plex.tv account | Yes, a local admin | Yes; Emby Connect is optional | Jellyfin, Emby, Navidrome and Immich all qualify | |
| Headless claim | Set up a server that has no screen | A `PLEX_CLAIM` token passed to the container signs it in on first run; the token is short-lived (unverified: about 4 minutes) | Open the wizard from another machine on the LAN | Same as Jellyfin (unverified) | Plex: the token flow is clean, but it ties the server to Plex's account service | |
| Protecting the unclaimed window | Nobody else on the network can take over a fresh server | The claim requires the owner's plex.tv login | 12.0 fixed a hole that allowed the setup wizard to be re-run | Unverified | Plex, by design | Navidrome 0.64.2 fixed a failed admin creation that left a server with no administrator, and stopped logging admin passwords when setup failed. |
| Add libraries during setup | The library is ready on the first visit | Yes | An optional step with a folder picker | Yes (unverified) | Jellyfin | Navidrome takes its first music folder from config; more libraries can be added in the UI since 0.58. |
| Metadata language and region | Correct titles from the start | Per library (unverified) | Yes, a server-wide step | Unverified | Jellyfin | |
| Remote access step | Decide on remote access during setup | Remote access settings exist, but streaming outside the home now needs Plex Pass or a Remote Watch Pass | A networking step; the docs advise against automatic UPnP port mapping | Emby Connect helps clients find the server | Jellyfin, for the honest UPnP warning | Covered in depth by the sibling remote-access research. |
| Restore from backup at first run | Rebuild a dead server from the welcome screen | No | Command line only (`--restore-archive`) | Unverified (a Premiere backup feature exists) | Immich: a "Restore from backup" button on the welcome screen, with checks that the library folders are readable (2.5.0, January 2026) | |
| Startup and migration progress page | See what the server is doing instead of a port that refuses connections | Unverified | 10.11 added a startup page, reachable on the LAN, that shows migration status | Unverified | Jellyfin | Immich shows a maintenance-mode page while restoring. |
| Usable before the first scan finishes | Play something within minutes | Libraries fill in while scanning (unverified) | Same (unverified) | Unverified | Navidrome: browsing works as soon as content appears, and it publishes expected scan times by library size (under a minute for under 1,000 songs; 15 minutes or more past 50,000) | |
| Next-steps guidance | Know what to do after setup | Unverified | The last wizard step points to remote access and hardware acceleration | The quick-start article lists next steps | Immich: a post-install guide covers extra users, the storage template, the mobile app and a 3-2-1 backup plan | |

### Hardware requirements and resource use

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Published CPU sizing | Buy the right machine | A PassMark score of 2,000 per 1080p transcode, multiplied by the number of streams | Named CPUs: Core i5-11400, Pentium Gold G7400, N100, Apple M series | Core 2 Duo at 1.6 GHz without transcoding; 2.4 GHz for HD transcoding | Plex for a simple formula; Jellyfin for naming current parts | All three size hardware for transcoding. None publishes what a direct-play-only server needs. |
| RAM floor | Know whether old hardware will do | No figure found (unverified) | 8 GB; 4 GB may do on a Linux server without a desktop | 512 MB on Linux, 1 GB on Windows or macOS; 2 GB recommended for transcoding | Emby on paper; Navidrome in practice (Pi Zero claim) | Immich needs 6 GB, or 4 GB with machine learning turned off. |
| Low-power guidance | Run on a fanless box | Unverified | Recommends Intel N-series, an M-series Mac mini or an RK3588 board; says most SBCs are too slow | Unverified | Jellyfin, for honest guidance | |
| Hardware transcoding without a paywall | Use the integrated GPU you already own | Requires Plex Pass | Free | Requires Emby Premiere | Jellyfin | |
| Hardware acceleration self-test | Know transcoding works before a viewer hits a problem | Not found (unverified) | Requested: "Automatically test hardware transcoding", #450, 423 votes, planned | Unverified | Nobody | |
| Transcode concurrency caps | One user cannot swamp the machine | Unverified | Requested: limit simultaneous streams per user, #1444, 85 votes | Unverified | Navidrome: `Transcoding.MaxConcurrent` plus a per-user cap (0.62) | |
| Data footprint guidance | Size the SSD | The container docs say `/config` can reach hundreds of GB for large libraries | The docs say a moderate library's database can grow to 10–100 GB, and suggest a 100 GB SSD for OS, Jellyfin and transcode cache | Unverified | Immich: database typically 1–3 GB, documented | Most of the Plex and Jellyfin footprint is generated images and metadata. |
| Scan-time resource behaviour | Scans do not knock the server over | Unverified | 10.9 launched many ffprobe processes at about 700 MB each, exhausting a 4 GB Pi 4 (#11588, 129 comments); one user's 10.11.0 scan went from 2.5 to 30 minutes (#15070, 110 comments) | Unverified | Navidrome: reads tags only and publishes scan expectations | In 12.0, one API request on a 300,000-item library allocated 25 GB and was OOM-killed (#17871, since fixed). |
| Storage medium guidance | Avoid slow disks for the database | Unverified | Put Jellyfin data on an SSD; avoid SMR hard drives | Unverified | Jellyfin | |
| Network guidance | Know whether Wi-Fi will do | Unverified | Gigabit Ethernet; Wi-Fi and powerline are not recommended | 10 Mbit upload for remote viewers; gigabit Ethernet or 802.11ac at home | Jellyfin and Emby | |

### Upgrades and rollback

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| In-app update notice | Learn that a new version exists | Unverified | No. Requests #30 (51 votes, planned) and #3502 (50 votes) | Unverified | Immich: checks GitHub releases and can include release candidates (option added in 3.0) | |
| Automatic updates | Stay patched without effort | The `public` and `beta` container tags update on every restart; desktop auto-update unverified | No built-in updater; the package manager or container does it | Unverified | Plex, for convenience | Auto-update without a rollback is how a broken 1.43.0 package reached users. |
| Release channels | Test a release before everyone gets it | A beta channel (Plex Pass, unverified) | Stable, unstable builds, and public release candidates (12.0 had seven) | Beta (unverified) | Jellyfin and Immich: free public release candidates | |
| Upgrade from any old version | Upgrade a neglected box in one step | Unverified | No. You must be on 10.10.7 first (or any 10.11.x for 12.0) | Unverified | Nobody | Immich installs older than 1.133.0 must finish the VectorChord migration before moving to 3.0. |
| Automatic pre-upgrade backup | A safety net you did not have to make | Only the routine three-day database backup | Yes. It copies the old database (`library.db.old`, plus timestamped backups) before migrating | Unverified | Jellyfin | Navidrome and Immich tell you to back up by hand first. |
| Separate migration step | Run migrations, check them, then start serving | No | 12.0 added `--mode MigrateSystem`, which runs migrations and exits | Unverified | Jellyfin | |
| Migration progress | See that a long migration is alive | Unverified | The 10.11 startup page | Unverified | Jellyfin | 10.11 migrations took minutes for clean databases and hours for inconsistent ones, by the project's own account. |
| Downgrade or rollback | Undo a bad upgrade | Users reinstall the older build, which fixed regressions in the 1.25.5 and 1.43.0 threads; a Synology package can block it | Only by restoring a backup taken before the upgrade | Unverified | Plex, in practice | Immich does not support downgrades, even within a minor version. Navidrome's 0.58 schema changes cannot be undone by downgrading, and 0.64 re-encodes every ID and asks for a backup first. |
| Breaking-change communication | Know in advance what will break | Release notes on the forum (unverified) | Detailed release posts with sections for administrators and developers | Unverified | Immich: release notes plus a breaking-change label on announcements | |
| Plugin safety on upgrade | Plugins do not break the upgrade | Not applicable (unverified) | 12.0: remove third-party plugins before upgrading; plugins must be rebuilt for .NET 10 | Unverified | Nobody | |
| Security update outreach | Learn when you must patch | Emailed owners of affected versions about CVE-2025-34158 (August 2025) | The release post explains the security fixes | Unverified | Plex, because its account system knows who runs which version | Gunmetal cannot do the same without a central account. |
| Pulling a bad release | A broken build stops spreading | Rolled the production channel back to 1.42.2 after the 1.43.0 packaging failure | Unverified | Unverified | Plex | |

### Backup and restore

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Built-in database backup | Survive database corruption | The core database every three days, keeping three copies | Manual backup from the dashboard (10.11 and later) | Server configuration backup and restore, Premiere only | Immich: a daily dump at 02:00, keeping 14 by default | |
| Schedule and retention | Backups happen without anyone remembering | Fixed: every three days, three copies | No built-in schedule; the docs suggest cron. Request #3546 (91 votes, planned) | Unverified | Immich (settings in the UI) and Navidrome (cron schedule and count in config) | |
| Choice of contents | Trade speed for completeness | Core database only; it leaves out metadata and the preferences file or registry keys | Database always; metadata, subtitles and trickplay images optional | Unverified | Jellyfin | Navidrome and Immich back up the database only, and say so plainly. |
| Server settings in the backup | Restore the server, not only the data | No; a forum moderator confirmed preferences are not included | Yes, core configuration | Yes, configuration backup | Jellyfin | |
| Restore from the web UI | Restore without a shell | No | Yes; the server restarts at once | Unverified | Immich: from the maintenance page or the welcome screen, with a restore point | |
| Restore from the command line | Scriptable recovery | Manual file copying; PlexDBRepair can swap in a Plex backup | `--restore-archive` | Unverified | Navidrome: `navidrome backup restore` (the server must be stopped) | |
| Restore point before restoring | A failed restore is not a second disaster | No | Not found (unverified) | Unverified | Immich: creates a restore point and rolls back automatically if the restore fails | |
| Move backups through the UI | Download a backup, or upload one into a new install | No | Requested: download backups, #3789 (7 votes) | Unverified | Immich: upload a `.sql.gz` dump in the UI | |
| Off-site targets | Backups leave the machine | No | No; a request for S3 as a target was declined (#3855) | Unverified | Nobody | Every rival leaves off-site copies to the user. |
| Library integrity report | Find missing, untracked or changed media files | No | No | Unverified | Immich 3.0: compares files on disk with the database, including checksum mismatches | |
| Paywall | | Free | Free | Premiere | Free everywhere except Emby | |

### Database health and repair

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Integrity check | Detect corruption early | Third-party PlexDBRepair `check` | Not found (unverified) | Unverified | Navidrome: a built-in `doctor` command | |
| Repair | Fix a broken database without starting over | Third-party PlexDBRepair (about 1.8k GitHub stars); Plex ships its own SQLite binary that forum helpers use for manual repairs | Restore from a backup | Unverified | PlexDBRepair: automatic mode and undo | The forum thread for that tool has 1,228 posts and 26,794 views. |
| Optimise and vacuum | Keep the database fast | A weekly optimise task | An "Optimize Database" task; the 12.0 notes ask admins to run it after upgrading | Unverified | Tie | |
| Bloat control | Stop the database growing without bound | PlexDBRepair's `deflate` fixes runaway statistics tables (its example: 31 GB down to 206 MB) | Unverified | Unverified | Nobody has it built in | |
| Search index rebuild | Fix broken search | PlexDBRepair rebuilds the full-text index | Unverified | Unverified | Navidrome: `search rebuild` | |
| Locking strategy | Avoid "database is locked" errors | Unverified | Three modes: no locking (default), optimistic retries, or serialised writes | Unverified | Jellyfin, as a pragmatic workaround | |
| External database engine | Use Postgres or MySQL | SQLite only | SQLite; the move to EF Core opens the way to Postgres. Request #315 for MySQL has 663 votes | Unverified | Not applicable | Immich requires Postgres. |
| Refusing unsafe data locations | Stop corruption before it happens | The container docs warn that SMB or NFS for `/config` will corrupt the database | The docs say the database must be on local storage | Unverified | Nobody enforces it (unverified) | Immich's requirements forbid network shares for Postgres. |
| Recovery from a bad scan | Undo damage a scan bug did | Not applicable | Not applicable | Not applicable | Navidrome: users recovered from a 0.64.1 scan bug that corrupted the database by restoring automatic backups (#6200) | |

### Migration and portability

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Import watch history from Plex | Keep what you have watched | Not applicable | Third-party tools only (JellyPlex-Watched, WatchState); the docs point to scripts | Third-party tools only | WatchState: many-way sync, portable play-state backups, health checks (about 1.6k stars) | |
| Import from Jellyfin or Emby | Switch without losing history | Third-party tools only | Moving an Emby database is not supported; the docs point to the Emby2Jelly script | Unverified | WatchState and JellyPlex-Watched | |
| Ratings import | Keep stars and likes | Account sync carries ratings between Plex servers only | Third-party tools only | Unverified | Nobody | Navidrome keeps ratings in its own database, so Plex music ratings move only by scripts such as `transfer_plex_ratings_to_subsonic`. |
| Playlist export and import | Playlists survive a move | No built-in music playlist export; one forum user said rebuilding 614 items was taking weeks | Unverified | Unverified | Navidrome: `pls` exports and imports playlists | |
| Users and shares | Friends keep their access | Library shares to other accounts may not survive a move | Users travel inside a backup | Unverified | Jellyfin, through its backup | |
| Collections and metadata edits | Keep curation work | Only by copying the whole data directory | Only with the data directory or a backup | Unverified | Nobody offers a portable format | |
| Move to a new machine with the same OS | Replace hardware | A documented manual procedure, same OS only | Backup and restore (10.11 and later), or copy the data directory with identical media paths | Configuration backup (Premiere) | Immich: restore from the welcome screen of a fresh install | |
| Cross-OS move and path remapping | Windows to Linux, or bare metal into Docker | Not officially supported | Media paths must stay identical; bare metal to Docker works only by mapping the same paths | Unverified | Navidrome: `missing fix` remaps missing files onto new ones | |
| History survives renames | Reorganising files keeps history | Unverified | Unverified | Unverified | Navidrome: remapping keeps play counts, ratings, stars and bookmarks | Remaps cannot be undone, so Navidrome asks for a backup first. |
| Portable history export | Your history as a file you own | No; the request "Export/Import watch history" has 1,700 views | No built-in export | Unverified | WatchState portable backups | |
| Watch state across servers | Watched on one server means watched on all | Opt-in account sync since 2022; excludes managed users and music, and only works with the newer metadata agents | No; request #716 for seamless access to other servers has 105 votes | Unverified | Plex | It depends on plex.tv being up. |
| Transfer server ownership | Hand a server to someone else | Unclaim and reclaim; shares and watch states do not follow | Not applicable (local admin) | Unverified | Jellyfin and Emby, by design | |
| Dry-run report | See what a migration will change first | Not applicable | Not applicable | Not applicable | JellyPlex-Watched: a dry-run mode | |

### Admin dashboard and activity

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Now playing | Who is watching what, right now | Free | Yes, active devices on the dashboard | Yes (unverified) | Plex | Navidrome redesigned its Now Playing panel in 0.62. |
| Playback decision detail | Why a stream is being transcoded | Shown (detail unverified) | Shown (detail unverified) | Unverified | Unverified | This is where Gunmetal's direct-play story is proved or disproved to the admin. |
| Bandwidth graphs | Local and remote traffic, live and historical | Plex Pass | Not built in (unverified) | Unverified | Plex | |
| CPU and memory charts | Health at a glance | Plex Pass on the web dashboard; also in the mobile and TV dashboards since July 2026 (whether those need Plex Pass is unverified) | No; 10.11 added storage space and item counts | Unverified | Plex | |
| Play history and top users | Who watches most, and what | Plex Pass | A plugin or Jellystat | Unverified | Tautulli (about 6.6k stars) | |
| Dashboard on a phone or TV | Check the server from the sofa | Yes: mobile, Fire TV and a tvOS preview (July 2026), with a server picker for homes with several servers | Unverified | Unverified | Plex | |
| Stop a stream | End a stuck or unwanted session | Unverified (Tautulli can) | Request #301, "Option to kill a stream from a user", 402 votes, open | Unverified | Tautulli | |
| Per-user stream limits | Fair use on a shared server | Unverified | Request #1444 (85 votes) | Unverified | Navidrome's transcoding caps | |
| Activity log | An audit trail of logins and changes | Unverified | Yes, with a "Clear Activity Logs" task | Unverified | Jellyfin | A request to anonymise or disable activity logging (#1161) has 33 votes. |
| Storage and library statistics | Disk use and item counts | Unverified | Added in 10.11 | Unverified | Jellyfin | |
| Admin notifications | Email or webhooks on events | Webhooks (Plex Pass, unverified) | An email overhaul is requested (#3559, 47 votes); webhook plugin unverified | Unverified | Tautulli; Immich has built-in SMTP | |
| Third-party analytics | More monitoring than the server offers | Tautulli | Jellystat (about 2.5k stars; needs Postgres; being rebuilt) | Jellystat supports Emby | Tautulli | |
| Paywall | | Everything past "now playing" needs Plex Pass on the web dashboard | Free | Unverified | Jellyfin | |

### Logs and diagnostics

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Log viewer in the UI | Read logs without a shell | Unverified | Yes (requests for a "go to end" button, #3912, and a delete button, #2444, refer to it) | A log files page exists in the old wiki | Jellyfin | |
| Log bundle download | Send logs to support in one file | A "download logs" action (unverified) | Individual files (unverified) | Unverified | Plex (unverified) | |
| Log level control | More detail when debugging | Debug logging is on by default; verbose logging is a separate switch | Unverified | Unverified | Immich: log level in the admin settings | Navidrome sets `LogLevel` in config. |
| Secret redaction | Logs that are safe to share | Tokens are logged only when `LogTokensForDebug` is switched on | Unverified | Unverified | Plex | Navidrome 0.64.2 stopped writing admin passwords to logs when setup failed, which shows how easily this goes wrong. |
| Metrics endpoint | Prometheus and Grafana | Not found (unverified) | Unverified | Unverified | Navidrome: an opt-in Prometheus `/metrics` endpoint | |
| File and tag inspector | See exactly how the server reads a file | Unverified | Unverified | Unverified | Navidrome: `inspect` | |
| Self-check command | One command that finds problems | Third-party PlexDBRepair | No | Unverified | Navidrome: `doctor` | |
| Auth logs for fail2ban | Block password guessing | Unverified | Requested (#3541) | Unverified | Nobody | |
| Centralised logging | Ship logs to another system | Unverified | Requested (#343, 6 votes) | Unverified | Nobody | |
| Usage telemetry disclosure | Know what is sent home | Plex is an account-based service (telemetry detail unverified) | None found (unverified) | Unverified | Not applicable | Navidrome's anonymous insights collector is on by default (`EnableInsightsCollector`). |

### Scheduled tasks

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Task list with "run now" | See and trigger maintenance | A Scheduled Tasks settings page with toggles | Yes; any task can be run at once, and plugins can add tasks | Yes (the old wiki has a page for it) | Jellyfin | |
| Maintenance window | Heavy work happens at night | A "butler" window, 02:00 to 05:00 by default | Triggers per task | Unverified | Plex | |
| Cron-style schedules | Exact control over timing | Not found (unverified) | Interval, daily and startup triggers (unverified) | Unverified | Navidrome: cron expressions for scans and backups | Immich uses cron for external library rescans. |
| Job concurrency | Tune work to the CPU | Unverified | Unverified | Unverified | Immich: concurrency per job type | |
| Safe cleanup when storage is offline | A NAS outage does not wipe the library | A trash model: "empty trash after every scan" can be switched off, and the move guide says to switch it off | The docs warn that maintenance tasks can remove items while media storage is unavailable; a manual purge option is requested (#3494) | Unverified | Plex, because of its trash model | |
| Visibility of running work | What the server is busy with | Unverified | Requested (#526, 9 votes) | Unverified | Immich job queues (detail unverified) | |

### Storage layouts

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Media on network shares | Media lives on a NAS | Yes | Yes, mounted at the OS level; the docs give NFSv3 locking tips | Yes; optional network paths let clients read a share directly (detail unverified) | Emby, for client-side direct paths | |
| Database on network shares | | Warned against: it corrupts the database | Must be local | Unverified | Immich states the ban most plainly | Every rival tells users not to do this. None was found to detect it. |
| Cloud drives through rclone | Media stored in cloud storage | Common; Plex scans have exhausted Google Workspace download quotas (rclone forum) | Documented: rclone, optionally with mergerfs, and turn off image extraction | Unverified | Jellyfin, for documenting it | |
| Header-only scanning | Scans do not download whole files | Thumbnail and probe steps read whole files (rclone forum) | The same; image extraction reads whole files | Unverified | Nobody, for video | |
| Change detection | New files appear without a manual scan | Unverified | Unverified | Unverified | Navidrome: a file watcher per library with a short delay | File watchers usually miss changes made by other machines on SMB or NFS (unverified). |
| Several folders per library | Spread a library over disks | Yes (unverified) | Yes | Yes (unverified) | Tie | |
| Several libraries with per-user access | Separate children's or shared collections | Yes | Yes | Yes | Tie for video | Navidrome added this in 0.58. |
| Read-only media | The server never writes into media folders | Unverified | Unverified | Unverified | Navidrome: documents a read-only music folder | |
| Filesystem tuning advice | The right settings for ZFS and similar | Unverified | ZFS record size of 4–8 KB for the database and 1 MB for media | Unverified | Jellyfin | Jellyfin on Proxmox LXC with ZFS hits "database is locked" (#15101, 229 comments). |

### Multi-server setups

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Several servers in one client | A home server and a cabin server | Yes; one account lists every server | Apps can hold several servers (unverified); request #47, "Multiple Servers Support", 177 votes, open since 2019 | Emby Connect gives one login across servers | Plex | |
| One library view across servers | Browse everything in one place | Extent unverified | Request #407 (83 votes) | Unverified | Plex (unverified) | |
| Caching or edge servers | Faster playback far from home | No | Request #505 (30 votes) | No | Nobody | |
| Monitoring several servers | One place to watch them all | The mobile dashboard has a server picker | No | Unverified | Plex | |
| Copying media between servers | Fill a second server or a drive | Unverified | Unverified | Premiere: sync media to folders and external drives | Emby | |

## Pain points and unmet demand

Each point gives evidence, with counts where the source shows them.

**P1. Plex locks local playback behind its account service.** The feature
request for a built-in local authentication server, so a plex.tv outage does
not lock owners out of their own LAN server, has 372 votes, 204 likes, 95
posts and 6,077 views. It was opened in 2015, last posted in September 2026,
and has had no formal staff answer
([forum topic 111339](https://forums.plex.tv/t/111339.json)). A 2025 thread
titled "Plex by default is useless even as a local server when internet down"
drew 430 views; the workaround offered was a list of networks allowed
without authentication
([topic 930093](https://forums.plex.tv/t/930093.json)). The 2026 forum
also has threads about playback errors after a June 9 outage and auth API
401 errors in September
([forum search](https://forums.plex.tv/search.json?q=plex.tv%20outage%20local%20server%20unavailable%20order%3Aviews)).

**P2. Plex keeps adding paywalls to server operations.** Streaming outside
the home now needs the owner's Plex Pass or the viewer's Remote Watch Pass,
and the Remote Watch Pass rose 50% on 1 June 2026, to $2.99 a month or
$29.99 a year ([Android Authority](https://www.androidauthority.com/plex-remote-watch-pass-price-increase-3663060/)).
The lifetime Plex Pass went from $249.99 to $749.99 on 1 July 2026; monthly
and yearly stay at $6.99 and $69.99
([9to5Mac](https://9to5mac.com/2026/05/19/plex-increasing-lifetime-plex-pass-cost-to-whopping-750/)).
Dashboard data beyond "now playing" needs Plex Pass, as does hardware
transcoding (Plex support article 200871837, via search summary;
[pms-docker README](https://github.com/plexinc/pms-docker)). Emby puts
hardware transcoding and configuration backup behind Premiere ($4.99 a
month, $54 a year or $119 lifetime,
[emby.media/premiere](https://emby.media/premiere.html)).

**P3. Backing up and moving a Plex server is manual and lossy.** Plex's
built-in backup copies only the core database every three days and leaves
out metadata and preferences (Plex support article 201553286, via search
summary). The canonical feature request for built-in backup, export and
import has 17 votes and 1,059 views; a moderator there confirmed the
preferences gap ([topic 573357](https://forums.plex.tv/t/built-in-backup-export-import-of-metadata-between-platforms-also/573357.json)).
At least four newer requests were pointed back to it as duplicates, the
latest in May 2026 ([938941](https://forums.plex.tv/t/plex-server-backups-automation/938941.json),
[906754](https://forums.plex.tv/t/backup-restore-tool/906754.json),
[864135](https://forums.plex.tv/t/simple-backup-and-restore-setting/864135.json),
[602843](https://forums.plex.tv/t/simplified-cloud-backup-of-server-settings-meta/602843.json)).
The official move guide covers the same OS only and warns that library
shares to other accounts may be lost (support article 201370363, via search
summary). The request to export and import watch history in the UI has
1,700 views ([topic 808477](https://forums.plex.tv/t/808477.json)).
Changing a server's owner means unclaiming it and re-sharing, and watch
states do not follow (forum search results for "change ownership of a
server").

**P4. Plex databases corrupt and bloat, and the fix is a community tool.**
The forum thread for the community DBRepair tool has 1,228 posts, 26,794
views and 665 likes ([topic 822684](https://forums.plex.tv/t/822684.json)),
and the tool itself has about 1.8k GitHub stars. It exists to check, repair,
reindex and shrink databases; its own example shrinks a bloated 31 GB
database to 206 MB ([PlexDBRepair](https://github.com/ChuckPa/PlexDBRepair)).
Users who cannot repair or restore are told to delete the database and
recreate their libraries
([topic 876374](https://forums.plex.tv/t/question-about-corrupt-database/876374),
via search summary; [forum search](https://forums.plex.tv/search.json?q=database%20corruption%20order%3Aviews)).
A user who only wanted to keep viewing history from a corrupt database
needed a 75-post thread to get there ([topic 771222](https://forums.plex.tv/t/771222.json)).

**P5. Upgrades break and rollback is improvised.** Plex 1.43.0 failed to
install from the Debian and RHEL repositories in January 2026 because of a
signing change; the thread has 395 posts, 25,226 views and 284 likes, and
Plex rolled production back to 1.42.2
([topic 935847](https://forums.plex.tv/t/935847.json)). Other regressions
were fixed by users downgrading by hand, for example the 1.25.5 colour-space
thread with 392 posts, and some Synology users could not downgrade at all
([forum search](https://forums.plex.tv/search.json?q=downgrade%20server%20version%20order%3Aviews)).
Jellyfin 10.11's database migration failed for many: issue
[#15027](https://github.com/jellyfin/jellyfin/issues/15027) has 121 comments
and 34 thumbs-up, and [#15060](https://github.com/jellyfin/jellyfin/issues/15060),
[#15388](https://github.com/jellyfin/jellyfin/issues/15388) (still open) and
[#15127](https://github.com/jellyfin/jellyfin/issues/15127) are similar.
Within a week of 12.0 there were reports of a failed migration on bad
timestamps ([#17849](https://github.com/jellyfin/jellyfin/issues/17849)),
SQLite errors after a Docker restart
([#17831](https://github.com/jellyfin/jellyfin/issues/17831), 16 comments)
and a broken dashboard ([#17989](https://github.com/jellyfin/jellyfin/issues/17989),
23 comments). Jellyfin's own docs say the only way back to an older version
is a backup ([backup docs](https://jellyfin.org/docs/general/administration/backup-and-restore/));
Immich does not support downgrades at all
([upgrade docs](https://docs.immich.app/install/upgrading/)). A Jellyfin
request for a native update or rollback option exists (#4001, 3 votes) and
so does one for an automatic backup on update (#3856, 3 votes)
([search API](https://features.jellyfin.org/api/v1/posts?query=update&limit=20)).

**P6. SQLite locking and slow queries hurt Jellyfin.** The grouped issue for
"database is locked" on LXC containers with ZFS has 229 comments; the
maintainers say there is little they can do about that stack
([#15101](https://github.com/jellyfin/jellyfin/issues/15101)). Earlier
lock-ups had 107 and 102 comments
([#11589](https://github.com/jellyfin/jellyfin/issues/11589),
[#11624](https://github.com/jellyfin/jellyfin/issues/11624)), and 10.11.0
had locking without LXC or ZFS
([#15166](https://github.com/jellyfin/jellyfin/issues/15166), 45 comments).
The project wrote a long post on the problem and added three locking modes
([SQLite locking post](https://jellyfin.org/posts/SQLite-locking)). Demand
for a different database is high: the MySQL backend request has 663 votes
([#315](https://features.jellyfin.org/api/v1/posts/315)).

**P7. Scans cost too much memory and time.** Jellyfin 10.9 launched many
ffprobe processes at about 700 MB each and ran a 4 GB Pi 4 out of memory
([#11588](https://github.com/jellyfin/jellyfin/issues/11588), 129 comments).
One 10.11.0 user's scan went from 2.5 to 30 minutes
([#15070](https://github.com/jellyfin/jellyfin/issues/15070), 110 comments).
In 12.0 a single request on a 311,788-item library allocated 25 GB within
30 seconds ([#17871](https://github.com/jellyfin/jellyfin/issues/17871)),
and "Scan All Libraries" leaked memory
([#17860](https://github.com/jellyfin/jellyfin/issues/17860)). Plex's
container docs warn its config can reach hundreds of GB, and Jellyfin's say
a moderate library's database can reach 10–100 GB
([storage docs](https://jellyfin.org/docs/general/administration/storage/)).

**P8. Built-in backups arrived late and are still thin.** Jellyfin only
gained backup and restore in 10.11 (October 2025), after a request with 142
votes ([#1603](https://features.jellyfin.org/api/v1/posts/1603)). Scheduled
backups are still a request (#3546, 91 votes, planned), as are downloading
backups from the web UI (#3789) and a "migrate server" button (#3988, 20
votes); a request for easy settings import and export collected 899 votes
before it was completed (#299)
([search API](https://features.jellyfin.org/api/v1/posts?query=backup&limit=20)).
Navidrome's and Immich's backups cover the database only; Navidrome's
restore requires the server to be stopped
([Navidrome backup docs](https://www.navidrome.org/docs/usage/admin/backup/)).

**P9. Nobody tells the admin an update exists, except Plex and Immich.**
Jellyfin's requests for update notifications have 51 and 50 votes (#30 and
#3502) ([search API](https://features.jellyfin.org/api/v1/posts?query=update&limit=20)).
Plex had to email owners to patch a severe vulnerability in August 2025
([BleepingComputer](https://www.bleepingcomputer.com/news/security/plex-warns-users-to-patch-security-vulnerability-immediately/)).

**P10. Admins lack basic controls and visibility.** Jellyfin requests:
automatically test hardware transcoding (#450, 423 votes), kill a user's
stream (#301, 402 votes), user groups (#493, 202 votes), default settings
for new users (#363, 130 votes), limit simultaneous streams per user (#1444,
85 votes) ([most-wanted list](https://features.jellyfin.org/api/v1/posts?view=most-wanted&limit=100)).
That the best monitoring tools are separate projects is evidence in itself:
Tautulli has about 6.6k stars ([Tautulli](https://github.com/Tautulli/Tautulli))
and Jellystat about 2.5k, and Jellystat needs its own Postgres
([Jellystat](https://github.com/CyferShepard/Jellystat)).

**P11. Privacy of the admin's own logs.** Jellyfin users ask to anonymise
or disable activity logging (#1161, 33 votes) and for a way to delete logs
(#2444) ([search API](https://features.jellyfin.org/api/v1/posts?query=log&limit=30)).
Plex keeps tokens out of its logs unless a debug switch is turned on, and
Navidrome had to stop
logging admin passwords on failed setup in September 2026
([Navidrome releases](https://api.github.com/repos/navidrome/navidrome/releases?per_page=5)).
Navidrome sends anonymous usage data by default
([config options](https://www.navidrome.org/docs/usage/configuration/options/)).

**P12. Migration between servers is left to third parties.** JellyPlex-Watched
(about 1k stars) and WatchState (about 1.6k stars) exist because no server
imports another's history ([JellyPlex-Watched](https://github.com/luigi311/JellyPlex-Watched),
[WatchState](https://github.com/arabcoders/watchstate)). Jellyfin's
migration docs say its database cannot easily be copied or adjusted, media
paths must stay identical, and moving from Emby's database is not supported
([migration docs](https://jellyfin.org/docs/general/administration/migrate/)).
A typical Plex-to-Jellyfin guide covers watch history only, after the user
fixed unmatched titles by hand
([Florian Jensen](https://florianjensen.com/2024/08/21/how-to-migrate-from-plex-to-jellyfin/)).
Moving music ratings from Plex to a Subsonic server needs a script with a
"use at your own risk" warning and one star
([transfer_plex_ratings_to_subsonic](https://github.com/profesaurus/transfer_plex_ratings_to_subsonic)).
Plex's own cross-server sync excludes managed users and music
([topic 800885](https://forums.plex.tv/t/800885.json)).

**P13. Storage going offline can delete the library.** Jellyfin's docs warn
that maintenance tasks can remove library items if they run while media
storage is unavailable ([storage docs](https://jellyfin.org/docs/general/administration/storage/)).
Plex's move guide tells users to turn off automatic trash emptying first
(support article 201370363, via search summary).

**P14. Cloud drives and network shares make scans expensive.** rclone forum
threads include "Hitting Google Workspace download limit daily trying to
scan in Plex" (36 posts) and "Library scans on Jellyfin Media Server
triggers rate limits very quickly" (13 posts); the common fix is warming
rclone's cache before scanning
([rclone forum search](https://forum.rclone.org/search.json?q=jellyfin%20plex%20scan%20order%3Aviews)).
Jellyfin's docs tell cloud users to turn off image extraction because it
downloads whole files.

**P15. One server per person.** Jellyfin's "Multiple Servers Support"
request has 177 votes and has been open since 2019
([#47](https://features.jellyfin.org/api/v1/posts/47)); related requests
have 105 (#716), 83 (#407) and 30 (#505) votes.

**P16. Heavy footprints rule out small hardware.** Jellyfin dropped 32-bit
ARM in 10.11 and says even a Pi 5 is too slow
([10.11 release](https://jellyfin.org/posts/jellyfin-release-10.11.0/),
[hardware guide](https://jellyfin.org/docs/general/administration/hardware-selection/)).
Immich needs 6 GB of RAM ([requirements](https://docs.immich.app/install/requirements/)).
Jellyfin cannot run on FreeBSD or TrueNAS CORE
([install docs](https://jellyfin.org/docs/general/installation/)), and
Navidrome users are asking for FreeBSD binaries
([#6213](https://github.com/navidrome/navidrome/issues/6213)).

Where rivals are already good, and should be matched rather than beaten:
Immich's backup schedule, restore from the welcome screen, restore point
and integrity report; Jellyfin's pre-migration database copy, startup
progress page and separate migration mode; Navidrome's small CLI toolset
(`doctor`, `inspect`, `missing`, `backup`, `pls`) and its single binary;
Plex's maintenance window, mobile dashboards and multi-server client; and
Emby's packaging breadth.

## Where Gunmetal can be clearly better

Each idea names the pain point it answers, how it fits the architecture
records, and what has to be true for it to work. IDs are for the feature
map to reference.

**OPS-1. Own the server from the first second, with no account and no open
setup window** (P1, P2). At first start the server prints a one-time setup
code to its console and log and writes it to a file only the service user
can read. The setup page asks for that code before anything else, so nobody
else on the LAN can claim a fresh server. The admin then registers a
passkey or device key; there is no vendor account at any point (ADR 1,
decision 7). What must be true: a secure context for WebAuthn, which
browsers only grant over HTTPS or on localhost
([MDN](https://developer.mozilla.org/en-US/docs/Web/API/Web_Authentication_API)),
so first-run from another machine over plain `http://192.168.x.x` needs a
plan (see Risks). Setup over the native client, over localhost, or over the
iroh link (ADR 1, decision 7) may avoid it.

**OPS-2. Backups small enough to always be on** (P3, P8). ADR 1 makes
SQLite a rebuildable cache and the watch history an append-only, exportable
log. Taken seriously, the backup is just the log, the configuration and the
identities: kilobytes to megabytes, not the tens to hundreds of GB of Plex
and Jellyfin data directories. Gunmetal can therefore back up on a schedule
by default, keep many copies, offer "download backup" in the UI, and offer
"restore from backup" on the first-run screen, as Immich does, with no
paywall. What must be true: every piece of user-created state lives in the
durable log or config, not only watch history (ratings, playlists,
collections, play queues, metadata overrides, users, device keys, sharing
grants), which needs a new ADR; rebuilding the cache from the files must be
fast enough that a restore is not an overnight job (the scan benchmark in
ADR 1 is the proof); and backups must be encrypted if they leave the
machine, because the log holds viewing history.

**OPS-3. Rollback that works** (P5). Because the cache can be rebuilt, an
older binary can always be started against the same data: it discards the
newer cache and rebuilds from files plus the log. Every upgrade also takes
an automatic snapshot (SQLite's online backup) before migrating, migrations
run in a separate step that can be checked (`gunmetal migrate --check`,
like Jellyfin's `--mode MigrateSystem`), and the startup page shows
progress. Any version can upgrade straight to the latest. What must be
true: the log and config formats only ever change additively, with versioned
event types that older versions skip safely; migrations are pure functions
over test fixtures held to the coverage and mutation rules; and the release
notes say plainly whether a release is safe to roll back.

**OPS-4. Import everything from Plex, Jellyfin, Emby and Navidrome, with a
dry-run report** (P3, P12). Built-in importers bring in watched state,
resume points, play counts, ratings, playlists, collections, and users (as
invitations, because credentials cannot move). Sources: a copy of Plex's
SQLite database read offline, the Jellyfin and Emby databases or APIs,
Navidrome's database or the Subsonic API, iTunes/Apple Music library XML,
and Last.fm or ListenBrainz export files. Matching goes by library-relative
path first, then provider IDs (IMDb, TMDb, TVDB, MusicBrainz), then tags and
duration, with an "unmatched" review queue. Every imported fact goes into
the log tagged with its source, so a bad import can be undone by removing
its events. What must be true: fixture databases from several Plex,
Jellyfin and Emby versions, because none of these schemas is a public
contract; importers that only read copies, never a running server's live
files; and, under ADR 2's rule on network grants, importers that reach
another server's API declare that access explicitly.

**OPS-5. History that survives moves, renames and OS changes** (P3, P12,
P13). Store each library root once and every item as a root plus a relative
path plus a content fingerprint. Moving to a new machine or OS, or from bare
metal into a container, is then "point root X at this path". Renamed or
moved files are matched by fingerprint and keep their history automatically,
the job Navidrome's `missing fix` does by hand. What must be true: a
fingerprint that is cheap and survives tag edits, which rules out hashing
whole files for music; hashing the audio or video payload from the parsers
the core already has is a candidate, but its cost over network storage must
be measured.

**OPS-6. A NAS outage never deletes anything** (P13). Before each scan,
check that each root is mounted and readable (for example a marker file the
server wrote when the root was added). An unavailable root is marked
offline and its items stay, greyed out; nothing is removed while a root is
offline. Deleted files go to a trash with a grace period, and purging is an
explicit action. What must be true: the scanner treats "root missing" and
"file deleted" as different events, and those cases are tested.

**OPS-7. Scans that are cheap on network shares and cloud drives** (P7,
P14). ADR 1 already moves probing to pure-Rust parsers and builds the
segment map once at scan time. Gunmetal should promise, and measure, that a
scan reads only container headers, tags and index data, never whole files,
and never launches a process per file. Thumbnail and frame extraction stay
off by default for remote roots. Each root gets an I/O profile ("local",
"network share", "cloud drive") that limits parallel reads. What must be
true: some files have no index (for example Matroska files without cues),
and building one means reading the whole file, so those files should be
indexed lazily on first play or in a throttled background job and flagged
in the dashboard. The scan benchmark against Jellyfin should report bytes
read, not only time.

**OPS-8. Refuse to put the database somewhere that will corrupt it**
(P4, P6). At startup, detect whether the data directory is on a network
filesystem (SMB, NFS, 9p, FUSE) and refuse to start unless the admin
overrides it, explaining why. Keep a single writer in-process so SQLite
locking problems cannot arise from internal concurrency. What must be true:
filesystem-type detection on each supported OS, and a documented override,
because some users will insist.

**OPS-9. A free, honest dashboard** (P2, P10). Now playing, with the
playback decision and its reason in plain words (for example "remuxing:
this browser cannot open MKV"), bandwidth, CPU and memory, history and top
users, stop-a-stream, per-user stream limits, and per-root health. No
paywall. History is derived from the append-only log, so it costs no extra
storage. An opt-in OpenMetrics endpoint serves people who run Grafana. What
must be true: the decision engine in the core emits structured reasons, and
the activity history has retention and anonymisation settings (P11).

**OPS-10. `gunmetal doctor`, in the CLI and the UI** (P4, P6, P10). One
check run covers: database integrity, data directory on a local filesystem,
free space, root reachability and permissions, clock sanity, FFmpeg
presence and version, whether the transcode sandbox can actually start on
this kernel, and a short hardware-encode test (the Jellyfin request with 423
votes). It produces a redacted diagnostic bundle that the admin previews
before sharing. Logs are structured, never contain secrets or tokens, have a
runtime log level, and write auth failures in a fail2ban-friendly format.
What must be true: redaction is enforced by the logging types (secrets
wrapped so they cannot be printed), not by convention.

**OPS-11. Update notices without phoning home by default** (P9). An opt-in
check fetches a signed release feed from the project's Cloudflare-hosted
site (ADR 1, decision 9) and shows new versions and security advisories in
the dashboard. Nothing installs itself unless the admin turns that on.
Container images have exact-version and major-line tags; there are no
download-at-start tags. What must be true: release signing keys and a feed
format; the check sends no identifiers; and the opt-in is part of first-run
setup, so most admins see the choice.

**OPS-12. Run on anything a Rust binary runs on** (P16). Ship one static
binary for Linux x86-64, AArch64 and 32-bit ARMv7, plus Windows, macOS and
FreeBSD, with deb and rpm repositories, a Windows service installer and a
minimal container image. A music-only install needs no FFmpeg at all,
because browsers and phones play FLAC, MP3, AAC and Opus directly (ADR 2);
Navidrome cannot say that. NAS packages (Synology, QNAP) and catalog
templates (Unraid, TrueNAS) come after. What must be true: the FFmpeg
sandbox has a backend for each OS, and on kernels too old for it,
transcoding is clearly reported as unavailable rather than run without the
sandbox.

**OPS-13. Few scheduled tasks, all visible** (P7, P10, P13). Most of the
maintenance Plex and Jellyfin schedule exists because of per-file probing,
image extraction and database bloat. Gunmetal should need very few tasks
(backup, rescan, trash purge, optional optimise), run them in a maintenance
window, show each one's progress, last run, duration and errors, let any
task be cancelled, and never run a destructive cleanup while a root is
offline.

**OPS-14. Several servers, one library, one history** (P15). Because
clients keep a synced copy of the library (ADR 1), a client can merge
libraries from several servers into one view without the servers knowing
about each other. Watch history is an append-only log, so a user's history
can be replicated between their servers and merged by event time. What must
be true: a server-independent item identity (the fingerprint and provider
IDs from OPS-4 and OPS-5); per-user identity that works across servers
through device keys or OIDC; and clear rules for conflicts such as "marked
unwatched on one server".

## Risks and hard parts

- **Passkeys need HTTPS.** WebAuthn only works in a secure context. A fresh
  server reached at a LAN IP address over plain HTTP cannot register a
  passkey. Options include a self-signed certificate with a fingerprint
  shown at setup (poor browser experience), setup only over localhost or a
  native client, or certificates through some outside service, which
  conflicts with "no central account". This decides the first-run design
  and should get its own ADR.
- **"The database is a cache" is only true if all user state is in the
  log.** ADR 1 names only watch history as irreplaceable. Ratings,
  playlists, collections, metadata edits, users, device keys and sharing
  grants are just as irreplaceable. If any of them lives only in SQLite,
  the small-backup and easy-rollback promises (OPS-2, OPS-3) are false.
- **Rebuild time is the price of a rebuildable cache.** A restore or a
  rollback means a rescan. On a large video library on network storage that
  could take hours, especially for files that need full reads to index. A
  cache snapshot may still be needed as an optional accelerator.
- **Fingerprints are hard to get both cheap and stable.** Hashing whole
  files is costly on network shares and cloud drives; hashing headers
  breaks on tag edits. A wrong match silently moves someone's history onto
  the wrong file.
- **Importers depend on undocumented schemas.** Plex, Jellyfin and Emby can
  change their databases in any release (Jellyfin rewrote its database
  twice in a year). Importers need fixtures from many versions and must
  read copies, never live files.
- **Packaging is a long tail.** Every OS, NAS store and container catalog
  has its own rules, signing and review. Plex's January 2026 signing
  failure shows that even a large company gets this wrong. Each target has
  to be tested on every release.
- **The sandbox varies by kernel.** Some NAS kernels are old. If the
  transcode sandbox cannot start, transcoding has to be off and clearly
  reported, which may surprise users who expect Plex-like behaviour.
- **Without telemetry, the project is blind to what breaks.** Gunmetal will
  not know which versions are deployed or which upgrades fail. Plex used
  exactly that knowledge to email owners about CVE-2025-34158. Opt-in
  crash reports and the opt-in update check are the only substitutes.
- **SQLite still needs care.** Even with one writer, long transactions
  during scans can stall readers. Jellyfin's experience shows locking
  problems surface as intermittent, environment-specific bugs that are hard
  to reproduce.
- **Multi-server history merges can conflict.** "Watched", "unwatched" and
  resume positions recorded on two servers at once need deterministic
  merge rules that users find unsurprising.

## Open questions

1. Which user-created state is irreplaceable, and does it all go into the
   append-only log? This needs an ADR before the server's data model is
   written.
2. How does first-run setup get a secure context for passkeys when the
   admin is on another machine on the LAN?
3. Is the update check opt-in or opt-out, and what exactly does it send?
4. Which packages ship with the first music release: static binaries and
   containers only, or deb, rpm and Windows installers too? When do NAS
   packages come?
5. Does Gunmetal ever write into media folders (NFO files, artwork,
   lyrics)? A strict read-only default makes network shares and shared
   libraries safer but removes a feature some Plex and Jellyfin users rely
   on.
6. Should importers read rival databases offline, call their APIs, or
   both? Offline reads need no credentials; API reads work against a live
   server on another machine.
7. Are users and sharing grants imported, or only recreated as invitations?
8. Does the admin dashboard live in the React Native client (ADR 1,
   decision 8), so it works on phones and TVs as Plex's now does, or in a
   separate web page served by the server for emergencies when the client
   is broken?
9. Is server-to-server history replication in scope for the first version,
   or is client-side aggregation enough?
10. What footprint numbers will Gunmetal publish (idle memory, memory during
    a scan, bytes read per file, database size per 10,000 items), and on
    which reference hardware?
11. Is there any case for supporting an external database such as Postgres?
    Jellyfin's demand for MySQL (663 votes) mostly comes from locking pain,
    which Gunmetal's design should avoid, but large multi-user installs may
    still ask.

## Sources

Plex:
- https://support.plex.tv/articles/201370363-move-an-install-to-another-system/ (content via search summary; direct fetch refused)
- https://support.plex.tv/articles/201553286-scheduled-tasks/ (content via search summary; direct fetch refused)
- https://support.plex.tv/articles/201774043-what-kind-of-cpu-do-i-need-for-my-server/ (content via search summary)
- https://support.plex.tv/articles/200871837-status-and-dashboard/ (content via search summary)
- https://github.com/plexinc/pms-docker
- https://github.com/ChuckPa/PlexDBRepair
- https://www.plexopedia.com/plex-media-server/general/hidden-settings/
- https://forums.plex.tv/t/server-dashboard-release/941143
- https://forums.plex.tv/t/server-dashboard-release/941143.json
- https://forums.plex.tv/t/backup-restore-tool/906754.json
- https://forums.plex.tv/t/simple-backup-and-restore-setting/864135.json
- https://forums.plex.tv/t/simplified-cloud-backup-of-server-settings-meta/602843.json
- https://forums.plex.tv/t/plex-server-backups-automation/938941.json
- https://forums.plex.tv/t/built-in-backup-export-import-of-metadata-between-platforms-also/573357.json
- https://forums.plex.tv/t/935847.json
- https://forums.plex.tv/t/822684.json
- https://forums.plex.tv/t/111339.json
- https://forums.plex.tv/t/930093.json
- https://forums.plex.tv/t/800885.json
- https://forums.plex.tv/t/808477.json
- https://forums.plex.tv/t/771222.json
- https://forums.plex.tv/t/question-about-corrupt-database/876374 (content via search summary)
- https://forums.plex.tv/search.json?q=database%20corruption%20order%3Aviews
- https://forums.plex.tv/search.json?q=downgrade%20server%20version%20order%3Aviews
- https://forums.plex.tv/search.json?q=plex.tv%20outage%20local%20server%20unavailable%20order%3Aviews
- https://forums.plex.tv/search.json?q=export%20watch%20history%20order%3Aviews
- https://forums.plex.tv/search.json?q=migrate%20server%20new%20computer%20feature%20order%3Avotes
- https://www.androidauthority.com/plex-remote-watch-pass-price-increase-3663060/
- https://9to5mac.com/2026/05/19/plex-increasing-lifetime-plex-pass-cost-to-whopping-750/
- https://www.bleepingcomputer.com/news/security/plex-warns-users-to-patch-security-vulnerability-immediately/
- https://github.com/Tautulli/Tautulli

Jellyfin:
- https://jellyfin.org/posts/
- https://jellyfin.org/posts/jellyfin-release-10.11.0/
- https://jellyfin.org/posts/jellyfin-release-12.0/
- https://jellyfin.org/posts/state-of-the-fin-2026-05-24/
- https://jellyfin.org/posts/SQLite-locking
- https://jellyfin.org/docs/general/administration/hardware-selection/
- https://jellyfin.org/docs/general/administration/backup-and-restore/
- https://jellyfin.org/docs/general/administration/storage/
- https://jellyfin.org/docs/general/administration/migrate/
- https://jellyfin.org/docs/general/installation/
- https://jellyfin.org/docs/general/server/tasks/
- https://jellyfin.org/docs/general/post-install/setup-wizard
- https://api.github.com/repos/jellyfin/jellyfin/releases?per_page=6
- https://features.jellyfin.org/api/v1/posts?view=most-wanted&limit=100
- https://features.jellyfin.org/api/v1/posts?query=plex&limit=30
- https://features.jellyfin.org/api/v1/posts?query=postgres&limit=30
- https://features.jellyfin.org/api/v1/posts?query=log&limit=30
- https://features.jellyfin.org/api/v1/posts?query=backup&limit=20
- https://features.jellyfin.org/api/v1/posts?query=update&limit=20
- https://features.jellyfin.org/api/v1/posts?query=unavailable&limit=20
- https://features.jellyfin.org/api/v1/posts/47
- https://features.jellyfin.org/api/v1/posts/315
- https://features.jellyfin.org/api/v1/posts/1603
- https://api.github.com/search/issues?q=repo:jellyfin/jellyfin+is:issue+migration+in:title+created:>2025-09-01&sort=comments&order=desc&per_page=20
- https://api.github.com/search/issues?q=repo:jellyfin/jellyfin+is:issue+%22database+is+locked%22&sort=comments&order=desc&per_page=15
- https://api.github.com/search/issues?q=repo:jellyfin/jellyfin+is:issue+created:>2026-09-06&sort=comments&order=desc&per_page=15
- https://github.com/jellyfin/jellyfin/issues/15027
- https://github.com/jellyfin/jellyfin/issues/15060
- https://github.com/jellyfin/jellyfin/issues/15127
- https://github.com/jellyfin/jellyfin/issues/15388
- https://github.com/jellyfin/jellyfin/issues/15101
- https://github.com/jellyfin/jellyfin/issues/11588
- https://github.com/jellyfin/jellyfin/issues/11589
- https://github.com/jellyfin/jellyfin/issues/11624
- https://github.com/jellyfin/jellyfin/issues/15070
- https://github.com/jellyfin/jellyfin/issues/15166
- https://github.com/jellyfin/jellyfin/issues/17831
- https://github.com/jellyfin/jellyfin/issues/17849
- https://github.com/jellyfin/jellyfin/issues/17860
- https://github.com/jellyfin/jellyfin/issues/17871
- https://github.com/jellyfin/jellyfin/issues/17989
- https://github.com/CyferShepard/Jellystat

Emby:
- https://emby.media/premiere.html
- https://emby.media/download.html
- https://emby.media/support/articles/Home.html
- https://emby.media/support/articles/System-Requirements.html
- https://emby.media/support/articles/Quick-Start.html
- https://emby.media/support/articles/Emby-Connect-for-Users.html
- https://github.com/MediaBrowser/Wiki/wiki

Navidrome:
- https://www.navidrome.org/docs/overview/
- https://www.navidrome.org/docs/installation/
- https://www.navidrome.org/docs/getting-started/
- https://www.navidrome.org/docs/faq/
- https://www.navidrome.org/docs/usage/admin/backup/
- https://www.navidrome.org/docs/usage/admin/cli/
- https://www.navidrome.org/docs/usage/admin/cli/missing/
- https://www.navidrome.org/docs/usage/features/multi-library/
- https://www.navidrome.org/docs/usage/configuration/options/
- https://api.github.com/repos/navidrome/navidrome/releases?per_page=5
- https://api.github.com/repos/navidrome/navidrome/releases/tags/v0.62.0
- https://api.github.com/repos/navidrome/navidrome/releases/tags/v0.63.0
- https://api.github.com/repos/navidrome/navidrome/releases/tags/v0.64.0
- https://api.github.com/search/issues?q=repo:navidrome/navidrome+is:issue+is:open&sort=reactions-%2B1&order=desc&per_page=30
- https://github.com/navidrome/navidrome/issues/6200
- https://github.com/navidrome/navidrome/issues/6213

Immich:
- https://docs.immich.app/install/requirements/
- https://docs.immich.app/install/upgrading/
- https://docs.immich.app/install/post-install/
- https://docs.immich.app/administration/backup-and-restore/
- https://docs.immich.app/administration/system-settings/
- https://github.com/immich-app/immich/pull/23978
- https://api.github.com/repos/immich-app/immich/releases?per_page=4
- https://api.github.com/repos/immich-app/immich/releases/tags/v3.0.0
- https://github.com/immich-app/immich/releases/tag/v3.0.0

Migration tools and storage:
- https://github.com/luigi311/JellyPlex-Watched
- https://github.com/arabcoders/watchstate
- https://github.com/profesaurus/transfer_plex_ratings_to_subsonic
- https://florianjensen.com/2024/08/21/how-to-migrate-from-plex-to-jellyfin/
- https://forum.rclone.org/search.json?q=jellyfin%20plex%20scan%20order%3Aviews

Platform:
- https://developer.mozilla.org/en-US/docs/Web/API/Web_Authentication_API
