# Library and metadata

This area covers everything between files on a disk and a library that is
correct and pleasant to browse: adding libraries and folders, noticing new,
moved and missing files, reading names, tags and sidecar files, identifying
items and matching them to online sources, artwork, the music model
(artists, credits, release groups, editions, works), the video model (films,
shows, episodes, versions, editions, extras), collections, manual fixes and
locks, and the reports that find what is broken. It also lists the scan-time
data that playback, sync, search and permissions depend on; those features
themselves belong to sibling maps. The bar is set by the rivals' worst
failures. Rescanning an unchanged library writes nothing. A move, a rename, a
better copy, a server upgrade or a database rebuild never undoes anyone's
history or fixes. Music is modelled at least as well as Navidrome does it and
far better than Plex. Every identification can explain itself. The server
never writes into media folders, and nothing in this area sits behind a
paywall.

## Features

Releases (R1, R2, R3, Later, No), the Demand scale, row ownership and the
terms "the user log" and "the identity store" are defined once in the
[feature map README](README.md). A row whose Release cell would differ
between maps names one owning row; the other maps point at it. In short: R1 is
music only, so anything that needs video, the remuxer or a metadata provider
plugin (the plugin host is R2) is R2 or later. Music modelling and music tag
interpretation are owned by `music.md`; this map owns scanning, change
detection, identity and library health, and its "Music:" sections point at
the owning MUS rows. Claims
about rivals come from `docs/research`; "(unverified)" is carried over from
there or added where no source confirmed a claim. Vote counts are as the
research recorded them on 2026-10-02.

UI surfaces use these names so the UI map can collect them: Admin >
Libraries, Library settings, Admin > Activity, Library health, Admin > Review
queue, Admin > Providers, Admin > Trash, Admin > Integrations, Edit sheet,
Artwork picker, Fix match dialog, Version picker, File inspector, and the
browse and detail screens.

### Libraries and folders

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIB-001 | Music libraries | A library kind for audio, with albums and artists built from tags rather than folder layout | Plex yes; Jellyfin yes, but one folder must hold one album; Navidrome yes, tags only | High: Jellyfin users move to Navidrome for tag-based albums (pain-points research, "Why people switch") | R1 | The pure-Rust core reads tags, artwork, gapless data and loudness tags in one pass at scan time, with no FFmpeg and no process per file (records 1 and 2) | Library record (kind, roots, settings); audio scan pipeline for FLAC, MP3, MP4 and Ogg | First-run wizard; Admin > Libraries |
| LIB-002 | Movie and TV libraries | Library kinds for films and series, each with naming and matching rules suited to it | Plex, Jellyfin, Emby yes | High: table stakes for a video server | R2 | Same scanner, identity and curation model as music, plus the keyframe segment map stored at scan (record 1) | Video scan pipeline; naming grammar; provider agents | First-run wizard; Admin > Libraries |
| LIB-003 | Several folders per library | One library spanning several disks or shares | Plex yes (unverified), Jellyfin yes, Emby yes | Medium: common with JBOD and mergerfs setups | R1 | parity | Roots stored once; every item stored as root plus relative path | Library settings > Folders |
| LIB-004 | Several libraries | Separate collections, such as a partner's music or a children's library, each with its own settings | Plex yes, Jellyfin yes, Navidrome yes since 0.58 | High: Navidrome's request had 106 reactions | R1 | Library membership is checked by the per-object authorisation layer (record 1) on every read, the class of leak Jellyfin (#17215) and Navidrome (2026 advisories) had to fix | Library ID on every item; access rules come from the users map | Admin > Libraries; library switcher in browse |
| LIB-005 | Add a library at first run | The library starts filling before the admin leaves setup | Jellyfin optional wizard step with a folder picker; Navidrome takes its first folder from config | Medium: first-run impressions (operations research) | R1 | The scan starts the moment the folder is chosen and albums appear as they are found (LIB-021) | Admin-only server folder browser; scan start | First-run wizard folder picker |
| LIB-006 | Exclusion rules | Skip folders or files with a gitignore-style file or a list in settings | Plex `.plexignore`; Navidrome `.ndignore`; Jellyfin not found in sources | Medium: Plex forum workarounds depend on it | R1 | Honours `.plexignore` and `.ndignore` beside Gunmetal's own file, so migrating users keep their rules | Pattern matcher per root | Library settings > Exclusions; File inspector names the rule that skipped a file |
| LIB-007 | Read-only media | The server never writes, renames or deletes anything in media folders, which can be mounted read-only | Plex keeps metadata in its own directory (unverified); Jellyfin and Emby write when NFO or image saving is on (unverified); Navidrome documents a read-only folder | High: Jellyfin deleted finished recordings during library scans (#17622, August 2026) | R1 | The scanner has no write path to roots at all; every fix lives in the server's own curation log (LIB-179) Read-only unless the owner enables deletion on that root (ADM-139, Later); the scanner itself never writes. | Root access check that reports, but never needs, write permission | Library settings shows "Media is read-only" |
| LIB-008 | Folder view | Browse a library as the disk tree, for stragglers and folder-curated collections | Emby yes; Jellyfin requested (#224, 438 votes, 134 comments); Navidrome through Subsonic folder endpoints | High: Jellyfin #224; Finamp #661 | R1 | Relative paths are already in the synced library, so the tree renders on the device and works offline Owns folder view; MUS-028, DIS-106 and LAT-107 point here. | Path tree per root in the change feed | Browse > Folders tab; breadcrumb on item pages |
| LIB-009 | Mixed movie and TV folders | One folder tree holding films and shows, each identified as the right kind | Jellyfin mixed type; Emby "unset" type; Plex no dedicated type (unverified) | Low: Jellyfin #3733 (2 votes); one Hacker News user went back to Plex partly over mixed folders; also named in the "Jellyfin to Plex" switching evidence (pain-points theme 9) | Later | Kind is decided per file by the naming grammar with a confidence score (LIB-098), not per library Until then, the naming report (LIB-084) flags mixed folders and explains how to split them. | Per-file kind detection | Library settings > Kind: mixed |
| LIB-010 | Home video library | Family videos that are never matched to films, browsed by the date they were filmed | Plex "Personal Media" agent; Jellyfin "Home Videos and Photos" type; Emby type (unverified) | Medium: Plex charges for remote personal video since April 2025; Jellyfin share-link request has 580 votes | R2 | No lookup is ever attempted for this kind; capture date comes from MP4, QuickTime and Matroska fields the core already parses, with file-name dates as fallback | Capture-date extraction; date index | Library kind picker; date-grouped browse |
| LIB-011 | Room for other media kinds | Audiobooks, music videos and recordings can arrive later without a data migration | Not applicable | Low: as a direct request; adjacent-media research calls it cheap insurance | R1 | Items carry a kind, typed relations ("video of" a track, "extra for" an artist), multi-file timelines and a general chapter structure from the first schema | Schema fields only | None in R1 |

### Scanning and change detection

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIB-012 | Manual scan | Rescan a library, a folder or one item | Plex, Jellyfin, Emby yes; Navidrome per-item refresh since 0.64 | Medium: table stakes | R1 | parity | Scan job scoped to library, path or item | Library menu "Scan"; folder and item menu "Rescan" |
| LIB-013 | Scheduled safety-net scan | A periodic rescan catches anything the watcher missed | Plex, Jellyfin, Emby yes (unverified detail); Navidrome cron expressions | Medium: table stakes | R1 | Costs almost nothing because an unchanged scan does nothing (LIB-016) | Scheduler with an interval or cron expression per library | Library settings > Schedule |
| LIB-014 | Watch local disks | New files appear within seconds of landing | Plex partial scan on change; Jellyfin and Emby real-time monitoring (Linux watch limits often need raising; Emby needs a restart after enabling) | High: Jellyfin 10.11's monitor triggers full rescans (#16729, open) | R1 | Watch events become path-scoped diff scans (LIB-017); the server warns when the OS watch limit is too low instead of failing silently | Per-OS file watchers with debounce | Library settings > "Watch for changes"; warning in Library health |
| LIB-015 | Change detection on shares and cloud drives | Files copied to a NAS appear without a manual scan | Nobody: SMB and NFS send no change events (Plex forum); Jellyfin documents NFS and rclone as unsupported | High: repeated Plex forum threads; Jellyfin docs | R1 | A polling mode walks directory metadata only (names, sizes, times) and opens no unchanged file | Per-root storage profile (local, network share, cloud drive) with poll interval and read parallelism | Library settings > Folder > Storage type |
| LIB-016 | Rescans that change nothing | Rescanning an unchanged library makes no database writes and sends devices no updates | Jellyfin re-saves about 3,980 of 52,140 items per unchanged scan and floods Kodi clients (#18274); Plex regarded as quick (unverified) | High: #18274; scan time is record 1's first benchmark | R1 | Per-file fingerprints and per-directory short-circuits; the scanner produces a diff instead of re-saving items; "scan twice and the second scan writes nothing" is a property test Owns no-change rescans with LIB-017; MUS-042 points here. | Fingerprint table (size, modification time, file ID or inode, short content hash); directory summaries | Admin > Activity shows "0 changed" |
| LIB-017 | Rescan only what changed | Adding one album or episode rescans that folder, not the library | Plex yes; Jellyfin regressed in 10.11 (#16729); Emby not found in sources | High: #16729 | R1 | Every trigger (watcher, poll, API, button) resolves to a set of paths and runs the same diff scan | Deduplicated path-scoped jobs | None beyond Admin > Activity |
| LIB-018 | Library change feed | Phones, TVs and browsers receive exactly what changed, so browsing is instant and works offline | Offline browsing: Plex no (unverified), Jellyfin and Emby no; Plexamp 4.50 caches library data (September 2026) | High: offline is Jellyfin's most-voted theme (1,820 and 817 votes) | R1 | The scan diff is the feed (record 1); one sequence number per change, so a device offline for a month catches up in one request | Ordered change log for items, artwork and relations, with compaction | None directly; feeds every browse screen |
| LIB-019 | Header-only reads | Scans on a NAS or cloud drive read tags and indexes, never whole files | Plex and Jellyfin probing and thumbnailing read whole files over rclone; Plex scans have exhausted Google Workspace download quotas | Medium: rclone forum threads (36 and 13 posts); Jellyfin tells cloud users to turn image extraction off | R1 | Pure-Rust parsers seek to the structures they need; bytes read per file is measured and published with the scan benchmark | Per-file read budget; I/O counters per root | Admin > Activity shows bytes read |
| LIB-020 | No helper process per file | A scan cannot exhaust memory on a small box | Jellyfin 10.9 launched many ffprobe processes at about 700 MB each and exhausted a 4 GB Pi 4 (#11588, 129 comments) | High: #11588; one 10.11.0 scan went from 2.5 to 30 minutes (#15070, 110 comments) | R1 | Probing runs in-process in the core (record 1); the scanner never launches FFmpeg | Bounded worker pool; memory ceiling per scan | None |
| LIB-021 | Usable during the first scan | Albums appear and play within minutes while a large library is still scanning | Navidrome yes, and publishes scan times by library size; others unverified | Medium: Navidrome presents it as a strength | R1 | Items commit in batches and flow out through the change feed as they are found Owns first-scan usability; ADM-031 and MUS-043 point here. | Batched commits; folder ordering (recently modified first) | First-run wizard progress; browse fills in live |
| LIB-022 | Scan and job activity | See what the scanner and background jobs are doing, how far along they are and what failed | Plex activity view (unverified); Jellyfin task progress, with more visibility requested (#3843, #526) | Medium: Jellyfin requests; operations research | R1 | Errors carry the file and the parser's typed reason, not a stack trace | Job registry with progress events | Admin > Activity; activity indicator in the admin header |
| LIB-023 | Cancel and reprioritise | Stop a long job; new files jump ahead of slow analysis | Nobody: Jellyfin request #3771 (2 votes) | Low: #3771 | R2 | Two queues: discovery of new and changed files always preempts analysis such as loudness measurement Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Priority job queue with cancel and pause | Admin > Activity row actions |
| LIB-024 | Background analysis that resumes | Slow work such as loudness measurement runs at low priority, survives restarts and stays out of playback's way | Plex says its sonic analysis can take hours or days; Jellyfin scheduled tasks | Medium: the low-hardware goal; Plex's own warning | R1 | Analysis results are keyed by content identity, so a restart or a move never repeats work; optional maintenance window | Persistent job state; throttle; maintenance window | Admin > Activity; Library settings > Analysis |
| LIB-025 | Upgrades without full rescans | A server upgrade never forces hours of rescanning | Jellyfin 10.11 asked for full and missing-metadata scans that could take hours; 12.0 needs a full rescan | High: Jellyfin 10.11 and 12.0 release notes | R1 | Each file records which parser version read it; a release that changes one parser re-reads only that format's files | Parser version per file and field group | Admin > Activity: "Re-reading MP4 files after a parser update" |
| LIB-026 | Refresh API for download tools | Sonarr, Radarr and Lidarr announce a download and it appears at once | Plex and Jellyfin connectors exist in Sonarr and Lidarr; Jellyfin has a path-based endpoint | High: watching is unreliable on network shares, so connectors are the dependable path (ecosystem research) | R2 | A native endpoint behind a token scoped to "refresh these roots"; the Jellyfin and Subsonic adapters map onto it later Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Path refresh endpoint; scoped token | Admin > Integrations > Tokens |
| LIB-027 | Library events after metadata | Integrations hear "album added" or "season 3 added" once, with IDs filled in | Emby delays events until metadata is ready; grouping requested for Jellyfin's webhook plugin (issues 31 and 329) | Medium: webhook plugin issues | R2 | The scanner knows when a batch ends, so it emits one grouped event per album or season after identification Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Outbox of versioned library events (delivery is in the integrations map) | Admin > Integrations > Event log |

### File identity, moves and missing media

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIB-028 | Stable identity for every file | History, fixes and playlists follow the recording or film, not its path | Navidrome persistent IDs; Plex users rely on the "Plex Dance"; Jellyfin re-adds moved files as new (unverified) | High: Plex Dance threads from 2019 to 2025; Jellyfin #15001 (61 +1) lost watch state on rename or replace | R1 | Owns content identity; MUS-038, INT-008 and VID-121 point here. Order: MusicBrainz recording ID; the FLAC STREAMINFO MD5 when it is set; a hash of a fixed window of audio frames, found by skipping the ID3v2, APE and ID3v1 regions by their declared sizes (a bounded read, never the whole file, so retagging does not change it); for video, the Matroska segment UID (unverified); then the path stem without its extension within the same root. A re-encode without MusicBrainz IDs at a new path loses identity unless it is matched by tags and duration, which goes to the review queue. | Identity table; versioned identity rules with property tests; "bytes read per file for identity" in the scan benchmark | None directly |
| LIB-029 | Moves and renames keep everything | Reorganising folders keeps play counts, ratings, locks, playlist positions and the date added | Navidrome automatic, with a manual remap command; Plex Dance otherwise | High: as LIB-028 | R1 | Watch history and the curation log are keyed by identity, so a move is only a path change in the diff | Move detection across roots within one scan | Admin > Activity: "214 files moved" |
| LIB-030 | Better copies replace, not duplicate | Swapping an MP3 album for FLAC (or, from R2, 1080p for 4K) keeps history and is not shown as new | Jellyfin regenerates derived data on replacement (10.11); "Recently Added shows upgrades" request (62 votes) | Medium: Jellyfin's 62-vote request; upgrades by Sonarr and Radarr are routine | R1 | Same relative path, same MusicBrainz recording or same provider ID means a new file for the same item; "date added" keeps the first arrival Limits: an MP3-to-FLAC upgrade with no MusicBrainz IDs at a new path changes both the extension and the audio, so it is matched by tags and duration and goes to the review queue (LIB-028). | Replacement rule; invalidation of derived data for the old file | Item history shows "Upgraded on (date)" |
| LIB-031 | Move the server, keep the library | Move to a new machine, OS or container by pointing each root at its new path | Plex documents a same-OS procedure; Jellyfin needs identical paths; Navidrome `missing fix` remaps | Medium: operations research (Plex move guide, Jellyfin migration docs) | R1 | Items are root plus relative path, so a move is one setting, checked against fingerprints before anything is marked missing | Root relocation with a dry run | Library settings > Folder > "Change location" with a preview |
| LIB-032 | An offline drive never empties the library | An unplugged disk or NAS outage greys items out instead of removing them | Plex trash model; Jellyfin docs warn maintenance can remove items while storage is unavailable | High: Jellyfin storage docs; Plex's move guide says to turn trash emptying off | R1 | Before each scan the server checks the root's filesystem identity and a sample of known entries; no file is written into the media (LIB-007). "Root missing" and "file deleted" are separate, tested events. Owns this behaviour; ADM-085 points here. | Root health check; offline state per root | Library health > Folders; offline badge on items |
| LIB-033 | Trash with a grace period | Deleted files keep their entries for a while; purging is an explicit action | Plex trash, emptied after scans unless switched off; Navidrome keeps missing files by default with a purge policy | Medium: Jellyfin manual-purge request (#3494) | R1 | A file restored within the period returns with everything attached, because its identity was kept Owns trash with a grace period; ADM-086 points here. | Trash state; retention; purge job that never runs while a root is offline | Admin > Trash with restore and purge |
| LIB-034 | Missing files list | See which files vanished, when and from where | Navidrome admin-only placeholders with CSV export; Plex trash view (unverified) | Medium: Navidrome docs | R1 | parity | Missing-since time; last known path | Library health > Missing files |
| LIB-035 | Remap a missing file by hand | When automatic matching fails, point a missing entry at its new file and keep its history | Navidrome `missing fix`, which cannot be undone | Low: Navidrome only | R2 | The remap is a curation-log entry, so it can be undone (LIB-178) Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Remap operation | Library health > Missing files > "Match to..." |

### Music: artists and credits

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIB-036 | Multi-value tags in every format | See MUS-034, which owns this feature. | Plex weak; Jellyfin partial; Navidrome yes, and documents which formats hold true multi-values | High: Plex request 116658 (520 votes, open since 2015) | R1 | See MUS-034. | None beyond MUS-034. | None directly |
| LIB-037 | Credits done properly | See MUS-001 (credits) and MUS-002 (display credit kept as tagged), which own this feature. | Plex one artist per track; Jellyfin 10.11 keeps a random subset of artist tags (#15283); Navidrome yes since March 2025 | High: Plex 520 votes; Jellyfin #15283 and #14622 | R1 | See MUS-001. | None beyond MUS-001. | Track rows and album headers with every name linked |
| LIB-038 | Artist splitting with exceptions | See MUS-035, which owns this feature. | Plex no; Jellyfin stopped splitting "feat." (#14622); Navidrome exception list since 0.63 | Medium: #14622 | R1 | See MUS-035. | None beyond MUS-035. | Admin > Libraries > Artist splitting; Library health > Tag problems |
| LIB-039 | Album artist apart from track artist | See MUS-003, which owns this feature. | Plex, Jellyfin, Navidrome yes; Emby splits an album unless album and album artist match on every track | Medium: table stakes, often broken | R1 | See MUS-003. | None beyond MUS-003. | Album page header |
| LIB-040 | Artists with the same name | See MUS-006, which owns this feature. | Plex weak, per a 2026 Plexamp forum post; Navidrome by MusicBrainz ID (unverified) | Medium: Plexamp forum; only stable IDs can solve it | R1 | See MUS-006. | None beyond MUS-006. | Artist page shows disambiguation when tagged |
| LIB-041 | Merge, split and alias artists | See MUS-007, which owns this feature. | Plex unverified; Jellyfin no (unverified); Navidrome requested (#2138, 9 reactions) | Low: #2138; Emby users asked in December 2025 | R1 | See MUS-007. | None beyond MUS-007. | Artist menu "Merge with..." and "Split"; Edit sheet |
| LIB-042 | "Appears on" | See MUS-004, which owns this feature. | Plexamp yes; Navidrome through multi-artist support (unverified) | Medium: Plex users call it the nearest thing Plex has to multi-artist support | R1 | See MUS-004. | None beyond MUS-004. | Artist page "Appears on" section |
| LIB-043 | Browse by role | See MUS-005, which owns this feature. | Plex "By Composer" requested since 2013 (89 votes); Navidrome composer since 2025; Lyrion 9.0 allows custom roles | Medium: Plex 89 votes; film-score fans must drop all but one composer (2026 Plexamp forum) | R1 | See MUS-005. | None beyond MUS-005. | Artist page role tabs; Browse > Composers |
| LIB-044 | Sort names and natural sort | See MUS-020, which owns this feature. | Plex yes; Jellyfin partial; Navidrome natural sort since 0.64 | Medium: Jellyfin request to sort an artist's albums by year (49 votes) | R1 | See MUS-020. | None beyond MUS-020. | Every browse list |

### Music: albums, releases and tags

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIB-045 | Albums built from tags | An album split across folders, or several albums in one folder, still comes out right | Plex mixes tags with its online data (unverified); Jellyfin needs one folder per album; Navidrome tag-only | High: Jellyfin shows "Disc 1" and "Disc 2" as separate albums (#5605, 71 comments) | R1 | Albums group by MusicBrainz release ID, then album artist plus album title, erring on the side of not merging; release groups sit above them (LIB-047) | Grouping rules with recorded reasons (LIB-098) | None directly |
| LIB-046 | MusicBrainz IDs as identity | See MUS-036, which owns this feature. | Plex partial (unverified); Jellyfin reads them and uses MusicBrainz as a provider; Navidrome reads them but groups on text | Medium: Jellyfin requests #3776 and #3731 (7 votes each) | R1 | See MUS-036. | None beyond MUS-036. | File inspector; IDs in item info |
| LIB-047 | Release groups and editions | See MUS-008, which owns this feature. A per-user preferred edition is MUS-009 (Later), not part of R1. | Plex weak (93 votes since 2015); Jellyfin no; Navidrome yes since 2025 | Medium: Plex 93 votes | R1 | See MUS-008. | None beyond MUS-008. | Album page edition switcher; one tile per release group on artist pages |
| LIB-048 | Release types | See MUS-010, which owns this feature. | Plex groups by type, manual override requested (72 votes); Jellyfin requested (#2220, 42); Navidrome yes since October 2025 | Medium: 72 and 42 votes | R1 | See MUS-010. | None beyond MUS-010. | Artist page sections; Edit sheet "Release type" |
| LIB-049 | Compilations and Various Artists | See MUS-011, which owns this feature. | Plex partial (81-vote request about a composer on a Various Artists album); Jellyfin partial; Navidrome uses the compilation flag | Medium: Plex 81 votes | R1 | See MUS-011. | None beyond MUS-011. | Artist page "Appears on"; Library health > Tag problems |
| LIB-050 | Multi-disc albums | See MUS-012, which owns this feature. | Plex partial (non-numeric disc labels requested, 67 votes); Jellyfin splits some (#5605); Navidrome yes, with per-disc art | Medium: #5605; Plex 67 votes | R1 | See MUS-012. | None beyond MUS-012. | Album page disc headers; "Play disc" action |
| LIB-051 | Same-titled albums stay apart | Two "Greatest Hits" by one artist do not merge | Emby separates them by MusicBrainz album ID; others not found in sources | Low: no vote data | R1 | Different release IDs never merge; without IDs, differing years or track lists keep albums apart and the decision goes to review if unsure | Grouping rule | Admin > Review queue when ambiguous |
| LIB-052 | Original date and release date | See MUS-013, which owns this feature. | Plex and Jellyfin partial (unverified); Navidrome supported (unverified) | Low: no vote data | R1 | See MUS-013. | None beyond MUS-013. | Album page; sort menu |
| LIB-053 | Multi-valued genres per track | See MUS-017, which owns this feature. | Plex mostly per album from its own data; Jellyfin multi-valued (per track unverified); Navidrome multi-valued | Medium: Plex's 866-vote tag request includes per-track genre | R1 | See MUS-017. | None beyond MUS-017. | Browse > Genres; filters |
| LIB-054 | Moods, styles, labels, grouping, BPM and key | See MUS-019 and MUS-022, which own this feature. | Plexamp moods, styles and labels; Navidrome label filter and BPM; Jellyfin unverified | Medium: Plex BPM request (22 votes); 866-vote tag request | R1 | See MUS-019. | None beyond MUS-019. | Browse filters; album details |
| LIB-055 | Grouping or work, your choice | See MUS-016, which owns this feature. | Lyrion 9.1 added a setting; others unverified | Low: Lyrion only | R2 | See MUS-016. | None beyond MUS-016. | Library settings > Music tags |
| LIB-056 | Works and movements | See MUS-014, which owns this feature. | Plex no (part of the 866-vote request); Jellyfin classical view planned (#772, 34 votes); Roon and Lyrion lead | Medium: 866 and 34 votes | R1 | See MUS-014. | None beyond MUS-014. | Album page groups tracks under work headings |
| LIB-057 | Work pages across albums | See MUS-015, which owns this feature. | Lyrion 9.0 and Roon; Plex and Jellyfin no | Medium: classical requests above | Later | See MUS-015. | None beyond MUS-015. | Work page; composer page |
| LIB-058 | Merge and split albums by hand | Fix a wrong grouping without retagging | Not found in sources for the three video servers | Medium: tag heuristics will sometimes guess wrong (music research risks) | R1 | Overrides live in the curation log and survive rescans and rebuilds | Album override records | Album menu "Merge with..." and "Split" |
| LIB-059 | Every raw tag kept | New fields can be used later without rereading every file | Not found in sources | Low: engineering need | R1 | The scan stores all raw tag frames per file, so a new mapping is a query, not a rescan | Raw tag store per file | File inspector |
| LIB-060 | Custom tag fields | Admins promote any tag, such as an "occasion" field, into filters and rules | Navidrome yes (unverified); Plex asked for it within the 866-vote tag request | Medium: 866 votes | Later | Built on raw tags (LIB-059), so turning one on needs no rescan | Field mapping config | Library settings > Custom fields |
| LIB-061 | Ratings stored in tags | See MUS-045, which owns this feature. Library specifics: POPM and RATING are read at scan, and the scale mapping is shown before import. | Plex no (import requested, 41 votes); Jellyfin and Navidrome unverified | Medium: Plex 41 votes | R2 | See MUS-045. | None beyond MUS-045. | Admin > Libraries > Import ratings from tags |
| LIB-062 | Lossless and lossy copies of one album | See MUS-031, which owns this feature. | Nobody; Jellyfin #3968 (1 vote) | Low: 1 vote | Later | See MUS-031. | None beyond MUS-031. | Album page version picker |

### Music: audio data, lyrics and CUE sheets

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIB-063 | Technical details | See MUS-021, which owns this feature. | Plex codec badges, sample-rate filter requested; Jellyfin requested (17 votes each for codec and bitrate); Navidrome requested (#1438) | Medium: those requests | R1 | See MUS-021. | None beyond MUS-021. | Track info; album badges; filters |
| LIB-064 | Gapless data | See MUS-069, which owns this feature. | Plexamp gapless; Jellyfin's request has 647 votes; Navidrome's web player will not do it (#745) | High: Jellyfin's top music request (647 votes) | R1 | See MUS-069. | None beyond MUS-069. | None here; the player uses them |
| LIB-065 | Existing loudness tags honoured | See MUS-084, which owns this feature. | Plex unclear (a June 2026 post asks it to honour tags); Jellyfin since 10.9, album gain since 12.0; Navidrome passes them to clients | High: Emby users name it a top gap (98 replies) | R1 | See MUS-084. | None beyond MUS-084. | Track info shows where the gain came from |
| LIB-066 | Loudness measured when tags are missing | See MUS-086, which owns this feature. | Plex analyses on the server and, since September 2026, on devices; Navidrome relies on tags (unverified) | High: the README promises loudness-normalised music | R1 | See MUS-086. | None beyond MUS-086. | Admin > Activity; track info shows "measured" |
| LIB-067 | Embedded lyrics | See MUS-154, which owns this feature. | Plex partial ("read embedded lyrics" requested, 49 votes); Jellyfin since 10.9 (550-vote request); Navidrome yes | High: Jellyfin 550 votes before it shipped | R1 | See MUS-154. | None beyond MUS-154. | Lyrics view (music map) |
| LIB-068 | Synced lyrics files | See MUS-155, which owns this feature. | Jellyfin `.lrc`, `.elrc` and `.txt`; Navidrome several formats since 0.63, with open reports of `.lrc` files not showing (#4148) | High: Navidrome #1421 (56 reactions) | R1 | See MUS-155. | None beyond MUS-155. | Library health > Sidecar problems |
| LIB-069 | TTML lyrics | See MUS-157, which owns this feature. | Navidrome since 0.63; Jellyfin requested (16 votes) | Low: 16 votes | R2 | See MUS-157. | None beyond MUS-157. | None beyond the lyrics view |
| LIB-070 | Online lyrics lookup | See MUS-162, which owns this feature. The provider extension point is INT-078 (R2). | Plex (Plex Pass); Jellyfin provider plugins since 10.10; Navidrome plugins | Medium: lyrics are a paid Plexamp feature | R2 | See MUS-162. | None beyond MUS-162. | Lyrics view "Find lyrics" |
| LIB-071 | CUE sheets and single-file albums | See MUS-041, which owns this feature. | No server: Plex request 507 votes; Jellyfin #1190 (171); Navidrome not planned | High: 507 and 171 votes | R2 | See MUS-041. | None beyond MUS-041. | Album page as normal; File inspector shows the source file |
| LIB-072 | Sonic features at scan | See MUS-172, which owns this feature. | Plex (Plex Pass), with hours to days of analysis; Jellyfin requested (#3245, 98 votes); Navidrome via plugins | Medium: 98 votes | Later | See MUS-172. | None beyond MUS-172. | None here; radio is in the music map |
| LIB-073 | More audio formats | See MUS-033, which owns this feature. DSD is not covered by MUS-033 and is not scheduled. | Plex and Jellyfin broad through transcoding; Navidrome broad | Medium: foobar2000 and every music server cover them | R2 | See MUS-033. | None beyond MUS-033. | None |

### Video: names and folders

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIB-074 | Movie names | `Title (Year)` files and folders are recognised, with or without a folder per film | Plex yes (unverified), Jellyfin and Emby yes | High: table stakes | R2 | The naming grammar is a pure function in the core, tested against a large, licence-checked corpus and mutation-tested | Filename and folder grammar | None (results appear in browse) |
| LIB-075 | Provider ID hints | Put an ID in the name and the match is exact | Plex `{tmdb-...}` style (unverified); Jellyfin `[tmdbid-...]`; Emby several forms; Infuse curly brackets | Medium: every rival supports some syntax; Jellyfin #3247 (3 votes) asks to honour the provider named in the hint | R2 | Every common syntax is accepted (`{tmdb-...}`, `[tmdbid-...]`, `[tmdbid=...]` and the IMDb and TVDB forms), so files named for any rival work unchanged | ID hint parser | File inspector shows "matched by ID hint" |
| LIB-076 | Episode names | `S01E01`, `1x01`, season folders and bare `S01` folders are recognised | Plex yes (unverified); Jellyfin needs `Season NN` folders; Emby accepts more forms | High: table stakes | R2 | Accepts the union of the rivals' grammars | Episode grammar | None |
| LIB-077 | Specials placed where they aired | Season 0 specials can also appear inside the season they belong to | Jellyfin `airsbefore` placement; Plex and Emby Season 0 | Medium: Infuse request (59 likes) | R2 | parity with Jellyfin; placement comes from NFO or the provider | Placement fields | Season page; show setting "Show specials in season" |
| LIB-078 | Date-based episodes | Daily shows named by air date are recognised | Emby yes; Plex yes (unverified); Jellyfin not found in sources | Low: no vote data | R2 | parity | Date grammar | None |
| LIB-079 | Multi-episode files | One file holding two episodes appears as both | Plex, Jellyfin, Emby yes | Medium: Plex request reached 255 votes before it was implemented | R2 | parity | Episode ranges per file | Episode list marks the shared file |
| LIB-080 | Multi-part films | `cd1`, `part2` and similar files play as one film | Plex yes (unverified), Jellyfin and Emby yes | Medium: common with older rips | R2 | A multi-file item with one timeline, the same structure audiobooks will use (LIB-011) | Part ordering; combined duration | Version picker lists the parts |
| LIB-081 | Shows without season folders | Episodes in one folder work, and single-season shows can be shown flat | Nobody: Jellyfin #3357 (172 votes) and #8 (160 votes) | High: two requests over 160 votes | R2 | The season comes from the file name, not the folder; flattening is a display flag read from synced data | Season inference; per-show flatten flag | Show page; show setting "Show seasons" |
| LIB-082 | Anime absolute numbering | Episode 1037 maps to the right season | Plex through third-party legacy agents (unverified); Jellyfin AniDB plugin | Medium: anime users objected when Infuse moved TV to TMDB ordering | Later | Absolute numbers are kept as parsed and mapped by an anime provider plugin | Absolute number field; mapping from the plugin | Show settings > Episode order |
| LIB-083 | 3D files | 3D files are recognised and labelled | Jellyfin and Emby yes | Low: no demand data | Later | parity | 3D tag parsing | Badge |
| LIB-084 | Naming report | A list of files that will match poorly, why, and a suggested name | Nobody; tinyMediaManager's renamer is the nearest thing | Medium: a gap in every product (library research) | R2 | The grammar returns structured reasons, so the report is a query rather than a heuristic | Per-file parse result with reason codes | Library health > Naming |
| LIB-085 | Renamer | The server renames and moves files into a convention | Plex and Jellyfin no, Emby no (unverified); tinyMediaManager does it | Low: no server request found | No | See "Deliberately not doing" | None | None |
| LIB-086 | Disc folders and images | `VIDEO_TS`, `BDMV` and ISO images play | Jellyfin does not support ISO; Kodi improved disc playback in v21 | Low: no vote data | No | See "Deliberately not doing" | None | None |

### Video: streams, chapters and sidecars

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIB-087 | Stream inventory | Codec, resolution, HDR format, Dolby Vision profile, audio layout, and every track's language and title are known before anyone presses play | Plex, Jellyfin, Emby probe with FFmpeg tools (unverified); Plex audio-label accuracy request (701 votes); Plex track names everywhere (417 votes) | High: 701 and 417 votes; Jellyfin filter by audio or subtitle language (172 votes) | R2 | Read by the core's container parsers at scan; feeds badges, filters and the playback decision without probing at play time | Track table per file | Version picker; filters; detail-page technical info |
| LIB-088 | Segment map | Exact seeking and segments for every file | Rivals' approach not documented in the research | Medium: needed by record 1's remuxer; no user request names it | R2 | Each file's keyframe index is read once at scan and stored (record 1, decision 4) | Keyframe index per video track | None |
| LIB-089 | Lazy indexing for files without an index | Matroska files with no cues still play, and the scan stays cheap | Not found in sources | Low: engineering need (operations research) | R2 | The segment map is built on first play or in a throttled job, and the file is flagged, rather than read in full during the scan | Pending-index flag; background index job | Library health > Needs indexing |
| LIB-090 | Chapters | Named chapters for jumping to scenes | Plex, Jellyfin, Emby yes (detail unverified) | Medium: table stakes | R2 | Matroska and MP4 chapters are parsed by the existing EBML and MP4 code into the general chapter structure (LIB-011) | Chapter list per file | Player chapter list (video map) |
| LIB-091 | Sidecar subtitles | `.srt` and `.ass` files beside a video or in a subfolder attach with language, forced and SDH flags taken from their names | Plex and Jellyfin yes; Jellyfin subfolder support requested (#2562, 17) and forced or SDH detection requested (#3404, 15) | Medium: those requests | R2 | Association rules are pure functions tested on real file names; malformed files are reported with typed errors | Sidecar records per file | Subtitle menu; Library health > Sidecar problems |
| LIB-092 | External audio tracks | Side-loaded dubs and commentaries appear as audio tracks | Plex requested (329 votes); Jellyfin partial (subfolder search requested, 9 votes) | High: Plex 329 votes | R2 | Same association rules as subtitles; libmpv clients load them directly | Sidecar audio records | Audio track menu |
| LIB-093 | One subtitle for every version | A single `.srt` serves the 1080p and 4K files | Nobody; Jellyfin requested (#1613, 22 votes) | Low: 22 votes | R2 | Sidecars attach to the edition rather than the file when their name matches the edition | Sidecar scope | None beyond the subtitle menu |
| LIB-094 | Intro and credits markers from chapters | "Skip intro" works for files whose chapters are named | Jellyfin official plugin reads chapters; Plex detects by audio (Plex Pass); Emby detects (Premiere) | High: Jellyfin's skip request reached 1,089 votes; Plex and Emby charge for detection | R2 | Markers are stored as typed segments on the file; chapter-name rules cost nothing at scan | Segment records | Skip button (video map) |
| LIB-095 | Intro and credits detection by analysis | Markers for files without named chapters | Plex (Plex Pass); Jellyfin community plugin; Emby (Premiere) | High: as above | Later | An opt-in background job in the sandbox, because it needs decoding | Detection job | Admin > Activity |
| LIB-096 | Scrub previews and chapter images | Thumbnails while scrubbing and for each chapter | Plex yes (10 or more minutes of CPU per film, 10 to 50 MB per item); Jellyfin about 100 times faster keyframe-only generation since 10.10 | Medium: on-demand previews requested (#4143, 4 votes) | R2 | Keyframes come straight from the segment map; generation runs in the sandbox and is off by default for network and cloud roots | Preview job or on-demand generation (choice in the video map) | Seek bar (video map) |

### Identification and matching

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIB-097 | Identify without the internet | A correct, browsable library with no lookups at all | Navidrome tags only; Jellyfin with NFO and fetchers off; Plex can put NFO and local assets first (2026) | High: Plex's agent breakages; record 2 | R1 | The core identifies from tags and MusicBrainz IDs in R1, then from names, ID hints, NFO `uniqueid` and container tags in R2, before any plugin runs | Local evidence record per item | File inspector |
| LIB-098 | Every decision explains itself | See why a file landed on this album, artist or film | Nobody | Medium: unmatched-item and provenance requests (Jellyfin #665, 101 votes; #968, 44 votes) | R1 | Each grouping or match stores a confidence and reason codes produced by the core | Decision record per item | Item info "Why is this here?"; File inspector |
| LIB-099 | Review queue | Doubtful groupings and matches wait for a yes instead of being applied silently | Nobody; Plex shows candidate scores when matching (unverified) | Medium: Plex Dance threads | R1 | Confidence thresholds per library; queued items stay browsable with a marker | Pending decisions | Admin > Review queue; marker on affected items |
| LIB-100 | Fix a wrong match | Search for the right title, apply it, and keep all history | Plex Fix Match, Jellyfin and Emby Identify (unverified); Plex users fall back on the Plex Dance | High: Plex Dance threads, including a June 2025 thread about the ritual itself failing | R2 | The fix is a curation-log entry keyed by content identity, so it survives rescans and rebuilds and history never moves | Provider search; match override | Item menu > Fix match dialog |
| LIB-101 | Fix one episode, fix the show | Correcting one episode re-matches the whole series | Infuse yes | Low: Infuse docs only | R2 | Overrides apply at show level when files share a show key | Show-level override | Fix match dialog option "Apply to the whole show" |
| LIB-102 | Unmatched list | Everything that failed to identify, in one place | Plex filter (unverified); Jellyfin requested (#665, 101 votes) | Medium: #665 | R2 | Unmatched items stay playable and browsable | Unmatched state | Library health > Unmatched; filter chip in browse |
| LIB-103 | Episode orders | Aired, DVD, absolute and provider episode-group orders per show | Plex TheTVDB orders since 1.40.4, TMDB groups still requested (502 votes); Jellyfin via the TheTVDB plugin | High: Plex request 537737 (502 votes) | R2 | The order is a per-show setting in the curation log; provider plugins supply the alternatives | Episode order sets per show | Show settings > Episode order |
| LIB-104 | New episodes get real titles | A just-aired episode's placeholder title is refreshed soon after airing | Jellyfin requested (#3025, 1 vote); Infuse picks up TMDB edits within about a day | Low: #3025 | R2 | A refresh job targets recently aired items whose fields are placeholders | Recently-aired refresh job | None |
| LIB-105 | External IDs for other tools | Seerr, Tautulli, Maintainerr and the download tools can read MusicBrainz, IMDb, TMDB and TVDB IDs | Plex yes after requests of 705 and 1,579 votes; Jellyfin and Emby yes | High: Plex `includeGuids` request (1,579 votes) | R2 | MusicBrainz IDs come from tags at scan in R1; video IDs follow in R2 Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | ID map per item in the API | IDs in item info |
| LIB-106 | Acoustic fingerprinting | Untagged files identified by how they sound | No server; Picard and beets via AcoustID (unverified) | Low: no server-side demand data | Later | An opt-in plugin, because it needs decoding and AcoustID's free tier is non-commercial | Fingerprint job in a plugin | Admin > Review queue suggestions |

### Metadata providers

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIB-107 | Sandboxed provider plugins | Metadata sources that can reach only the hosts they declare and cannot harm the server | Plex HTTP providers (December 2025, movie and TV only); Jellyfin in-process plugins; Navidrome WebAssembly agents | High: a 2025 Plex beta broke every third-party agent and Plex removes legacy agents in 2026 | R2 | Record 2: providers are plugins with explicit network grants, checked against the resolved IP; the host, not the plugin author, enforces each service's terms (attribution, cache age, rate limits, keys) | WebAssembly host; manifest with hosts; shared rate limiter; provider cache with age tracking | Admin > Providers, with permissions shown before enabling |
| LIB-108 | Nothing leaves by default | No lookup is made until the admin enables a provider | Jellyfin with fetchers off; Navidrome tags first; Plex agents are backed by Plex's metadata service (unverified) | Medium: privacy is the second reason people self-host (selfh.st, 3,520 of 4,081) | R1 | No provider code lives in the server core (record 2) | Empty provider registry by default | First-run wizard explains what each provider would send |
| LIB-109 | TMDB provider | Posters, plots, cast and dates for films and shows | Jellyfin built in; Emby yes (unverified); Infuse uses it for TV since 2021 | High: nobody accepts a movie library without posters (library research) | R2 | A first-party plugin whose host enforces TMDB's terms, including attribution and refresh within six months | Plugin; cache with age tracking | Admin > Providers > TMDB; attribution on the credits screen |
| LIB-110 | TheTVDB provider | Alternate episode orders and broader TV coverage | Plex orders; Jellyfin plugin; tinyMediaManager only in its paid tier | Medium: episode-order complaints; paywall at tinyMediaManager | Later | User-supplied key and subscriber PIN handled by the host, as TheTVDB's terms require | Plugin with per-user PIN storage | Admin > Providers |
| LIB-111 | MusicBrainz lookups | IDs, release types and credits filled in for partly tagged music | Jellyfin uses MusicBrainz as a provider; Navidrome reads IDs but groups on text | Medium: Jellyfin #3776 and #3731 (7 votes each) | R2 | One shared limiter keeps all lookups to about one request per second; results become review-queue suggestions unless confidence is high | Rate-limited queue; suggestion records | Admin > Review queue; album menu "Look up on MusicBrainz" |
| LIB-112 | Cover Art Archive covers | The right front and back covers for the exact release, up to 1200 pixels | Jellyfin (unverified); Navidrome uses Last.fm only | Medium: missing covers are visible on every grid | R2 | Fetched by the MusicBrainz release or release-group ID read from tags, so there is no fuzzy search | Plugin; artwork cache | Artwork picker |
| LIB-113 | Artist images and biographies | Artist pages that look finished | Plex yes (listed as a Plex Pass perk); Jellyfin TheAudioDB and MusicBrainz (unverified); Navidrome Last.fm and uploads | High: a paid Plex perk; Jellyfin #9406 (204 reactions) put a keyboardist's photos on every Ogg Vorbis file | R2 | Plugins match artists by MusicBrainz ID only, so a name collision cannot attach the wrong photo | Plugin; artist image cache | Artist page; Artwork picker |
| LIB-114 | fanart.tv artwork | Logos, clear art and disc art | Jellyfin plugin; Kodi and tinyMediaManager (Kodi unverified) | Low: no vote data | Later | A plugin with a network grant | Plugin | Artwork picker |
| LIB-115 | External ratings | IMDb and Rotten Tomatoes scores on detail pages | Plex IMDb ratings in its TV agent; Jellyfin requested (#463, 196 votes); Infuse in progress (321 likes) | High: 196 votes and 321 likes | Later | A plugin; ratings are fields with provenance like any other | Plugin | Detail-page rating row; sort and filter |
| LIB-116 | Provider order and fallback per library | Choose which source wins, and fall back to another when a title is missing | Plex agents as ordered provider lists; Jellyfin per-library order | Medium: Plex built its 2025 provider model around it | R2 | parity, with sandboxed providers | Agent definition per library | Library settings > Providers |
| LIB-117 | Per-field sources | Plot from one source, ratings from another | Nobody; Jellyfin #970 (5 votes) and #4116 (1 vote) | Low: 6 votes in total | R2 | Merge rules per field group in the agent definition | Field-level merge rules | Library settings > Providers > Advanced |
| LIB-118 | Provenance | See which source each field came from | Nobody; Jellyfin #968 (44 votes) and #1426 (5 votes) | Medium: #968 | R2 | Every value carries its source: tag, NFO, plugin or a named person's edit | Source per field | Edit sheet shows the source next to each field |
| LIB-119 | Refresh modes | "Fill in what is missing" or "replace everything" for an item or library, never touching locked fields | Plex and Jellyfin both offer modes (detail unverified) | Medium: table stakes | R2 | parity; locks always win (LIB-173) | Refresh job modes | Item and library menus "Refresh metadata" |
| LIB-120 | Provider failures never erase data | An outage or an API change leaves existing metadata and artwork in place | Jellyfin image fetching broke after a 2026 TMDB date-format change (#16722, 77 +1); Lidarr's metadata outages | Medium: #16722 | R2 | Responses that fail validation are rejected as a whole and reported; the last good values stay | Response validation; failure log | Admin > Providers status |
| LIB-121 | Content ratings | Age certifications per country, used by parental limits | Plex, Jellyfin, Emby store and use them | High: maximum-rating limits exist in all three (users research) | R2 | Stored per country with provenance; enforcement is in the users map | Rating field per country | Detail page; Edit sheet |
| LIB-122 | Plex-style provider compatibility | Community HTTP providers written for Plex also work here | Plex only | Low: no demand data | Later | An adapter plugin, if Plex's provider API terms and stability allow it (open in the research) | Adapter plugin | Admin > Providers |
| LIB-123 | Regional and anime providers | Sources for regions where TMDB and TheTVDB are unreachable, and for anime | Jellyfin plugins such as AniDB; TMDB and TheTVDB are unreachable from mainland China (Jellyfin docs) | Medium: regional reach and anime ordering complaints | Later | Community plugins on the same host | Plugins | Admin > Providers |
| LIB-124 | Streaming catalogue mixed in | Search and home rows that mix in streaming services | Plex yes, and it can be switched off after a 1,000-vote request | High: demand to turn it off | No | See "Deliberately not doing" | None | None |
| LIB-125 | Critic and user reviews | Reviews on detail pages | Plex shows them; switching them off was requested (185 votes) | Low: the demand is to turn them off | No | See "Deliberately not doing" | None | None |

### NFO files and embedded video tags

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIB-126 | Read NFO files | Curated metadata from tinyMediaManager, Kodi or Jellyfin is used as is, including `lockdata` | Jellyfin yes, local NFO beats remote; Emby yes; Plex preview in 1.43.1 (February 2026), read-only, no music or extras | High: Plex request 467992 (231 votes, opened 2019) | R2 | XML is parsed in the core with no DTD or entity expansion; NFO `uniqueid` gives exact identity; Kodi, tinyMediaManager and Jellyfin or Emby dialects are accepted | NFO parser; lock import | File inspector shows "from NFO"; lock indicator |
| LIB-127 | Music NFO | `artist.nfo` and `album.nfo` biographies and IDs with no network | Jellyfin yes; Plex not supported and not planned | Low: no vote data | R2 | Gives finished artist pages without any provider | Shared NFO parser | Artist page |
| LIB-128 | Embedded video titles and tags | The title stored inside an MKV or MP4 is used | Emby option; Plex reads MP4 and M4V only, not MKV; Infuse request (9 likes) | Low: 9 likes | R2 | Matroska and MP4 tags come from the same core parsers as the segment map | Container tag fields | Library settings "Prefer embedded titles" |
| LIB-129 | Local beats online | Your NFO files and tags beat online data, per library | Jellyfin local NFO always wins; Navidrome tags first; Jellyfin fetched from providers even when disabled (#5778, 18 reactions) | Medium: #5778; Jellyfin #3349 | R2 | One explicit order (a person's edit, then NFO or tags, then providers) that the admin can see and change; in R1 there is nothing online to override | Precedence rules per library | Library settings > Precedence |
| LIB-130 | NFO for extras | Titles and descriptions for trailers and featurettes from sidecars | Nobody | Low: no vote data | Later | Same parser | NFO association for extras | Extras row |
| LIB-131 | NFO in a separate folder | Sidecars kept out of media folders | Nobody; Jellyfin #3827 (6 votes) | Low: 6 votes | Later | A parallel tree that mirrors media paths | Mirror-tree lookup | Library settings > Folders |
| LIB-132 | Curation export | Take every fix, lock and collection to Kodi, Jellyfin or an archive as NFO or JSON | Jellyfin and Emby write NFO beside media when saving is on | Medium: fixes that survive reinstalls (library research, pain point 5) | Later | Generated from the curation log into a separate directory or a download, so media stays read-only | Export job | Admin > Libraries > Export |
| LIB-133 | Write into media folders | The server writes tags, NFO files or artwork next to your media | Jellyfin and Emby NFO and image savers; tag writing requested from Jellyfin (#1685, 8 votes) | Low: 8 votes for tag writing | No | See "Deliberately not doing" | None | None |

### Artwork

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIB-134 | Embedded cover art | Covers stored inside files are used | Plex, Jellyfin, Navidrome yes | High: table stakes | R1 | Read by the core at scan without decoding pixels; only the resize step decodes (LIB-143) | Artwork records keyed by content hash | Every album tile |
| LIB-135 | Local artwork files | `cover.jpg`, `folder.jpg`, `front.*` and `artist.jpg` (and, from R2, poster and fanart files) are used | Plex Local Media Assets; Jellyfin and Emby documented names; Navidrome `cover`, `folder` and `front` | High: table stakes | R1 | Accepts the union of the rivals' file names, so existing folders work unchanged | Sidecar image association | Artwork picker shows "From folder" |
| LIB-136 | Predictable artwork order | A documented order between folder images, embedded art and provider art, changeable per library | Navidrome documents its order; Jellyfin folder images win, embedded preference requested (#1771, 29 votes) | Medium: Jellyfin requests of 29 and 14 votes | R1 | One precedence list per library, shown in settings, with per-track embedded art as an option | Precedence setting | Library settings > Artwork |
| LIB-137 | Per-disc art | Box sets show each disc's cover | Navidrome `disc*` and `cd*` images; others not found in sources | Low: no vote data | R2 | parity with Navidrome Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Disc artwork slot | Album page disc headers |
| LIB-138 | Your own images | Upload a cover, poster or artist photo, and delete uploads you no longer want | Plex upload (unverified), deleting unwanted posters requested (151538, 333 votes); Navidrome upload since 0.61 | High: Plex 333 votes | R2 | Uploads are stored with the curation log, not in the rebuildable cache, so they survive rebuilds and are backed up Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Uploaded image store | Artwork picker "Upload" and "Remove" |
| LIB-139 | Image by link | Paste a link to use an image | Jellyfin requested (#270, 252 votes); Plex yes (unverified) | High: 252 votes | R2 | Fetched through the plugin host's guarded HTTP client, which refuses private and loopback addresses, then stored as an upload | Guarded fetch | Artwork picker "From link" |
| LIB-140 | Choose among candidates | Browse every available poster or cover and pick one | Plex, Jellyfin, Emby yes (unverified) | Medium: table stakes | R2 | Candidates from local files and every enabled provider in one picker, each showing its source | Candidate list per item | Artwork picker |
| LIB-141 | Chosen art stays chosen | No refresh ever replaces an image you picked | Plex's poster field reportedly ignores its lock (755134, 194 votes, no staff reply); Jellyfin image-lock behaviour unverified | High: 194 votes; Infuse thread "Metadata is getting (wrongly) overwritten" | R2 | Picking an image writes a lock to the curation log; refresh code cannot change a locked slot, which is tested for every field, images included Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Lock per image slot | Lock icon in the Artwork picker |
| LIB-142 | Images sized for each device | Fast grids on phones and TVs, with a blurred placeholder while each image loads | Navidrome blurred placeholders (0.64); Plex's config can reach hundreds of GB and Jellyfin's database 10 to 100 GB, mostly generated images and metadata | Medium: storage footprint (operations research) | R1 | A few fixed sizes per image plus a tiny placeholder hash in the synced library; originals are never copied | Resize job; derivative cache | Every grid |
| LIB-143 | Safe image handling | A malformed or hostile image cannot crash or take over the server | Not found in sources | Medium: decompression bombs and image-decoder exploits (the adjacent-media research cites CVE-2023-4863) | R1 | Size and pixel limits are checked before decoding; decoding runs in memory-safe code or the sandbox | Image decode limits | None |
| LIB-144 | Video artwork types | Posters, backdrops, logos, thumbnails and season posters | Plex posters and backgrounds; Jellyfin many types; tinyMediaManager the widest set | High: table stakes for video | R2 | Same artwork model, locks and derivatives as music | Artwork slots per kind | Detail pages; Artwork picker |
| LIB-145 | Per-season art | Different art for each season | Plex and Jellyfin (unverified) | Low: Infuse request (76 likes) | R2 | parity | Season artwork slots | Season page |
| LIB-146 | Quality badges as data | Lossless and hi-res badges in R1, and 4K, HDR and Atmos badges in R2, with posters left untouched | Kometa (Plex only) and Emby's Cover Art plugin (Premiere) draw onto posters; Kometa warns old overlays pile up | Medium: Jellyfin 4K marker request (#1738, 146 votes) | R1 | Clients draw badges from synced technical fields; nothing is rewritten, and each device can switch badges off | Technical fields in the change feed | Badges on tiles and detail pages; client setting |
| LIB-147 | Overlays burned into posters | Badges drawn into the image files | Kometa for Plex; Emby Cover Art plugin | Low: replaced by LIB-146 | No | See "Deliberately not doing" | None | None |
| LIB-148 | Animated covers | Motion artwork on album pages | Navidrome since 0.61 | Low: Navidrome only | Later | parity | Animated derivative | Album page |

### Versions and editions

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIB-149 | Versions in one folder | A 4K and a 1080p file appear as one title with a choice | Plex yes; Jellyfin yes, with no documented cap; Emby up to 8 | High: table stakes | R2 | Files group by identity and edition, not by name patterns alone | Version records under an edition | Version picker |
| LIB-150 | Versions across folders and libraries | "Movies" and "Movies 4K" folders or libraries show each film once | Jellyfin no (#174, 265 votes, open since 2019); Plex merges within a library (unverified) | High: 265 votes | R2 | Grouping is by work identity (provider ID or NFO ID), so folder and library boundaries do not matter; each person sees only versions they may access | Cross-root work grouping | One tile in browse; Version picker names the source library |
| LIB-151 | Named editions, free | Director's Cut and Theatrical are separate entries with their own art and history | Plex `{edition-...}` names, setting or editing needs Plex Pass; Jellyfin no (#2542, 102 votes); Emby versions only; Infuse reads `{edition-...}` | High: Plex's original "Multiple Cuts" request had 1,459 votes | R2 | Work, edition and file are three levels of the model; Plex's `{edition-...}` naming is accepted as is | Edition entity | Detail-page edition switcher |
| LIB-152 | Editions and versions for TV | Remastered or dubbed versions of a show, and versions of single episodes | Plex show-level editions (July 2026; watch-state sync not yet working); Emby episode versions up to 8; Jellyfin episode versions in 12.0 | Medium: Plex TV editions request (124 votes) | R2 | The same three-level model applies to shows and episodes | Edition and version records for episodes | Show and episode pages |
| LIB-153 | Version picker detail | See codec, HDR, audio and size before choosing | Plex requested (312496, 266 votes); Infuse users ask for wider boxes | High: 266 votes | R2 | Everything comes from the scan (LIB-087) at no extra cost | Version summaries | Version picker |
| LIB-154 | The right version, automatically | Each device plays the version it can play directly | Jellyfin sorts the highest resolution first; Plex not found in sources | Medium: needless transcoding is a top theme (Plex "default to max quality", 1,289 votes) | R2 | The core's playback decision engine ranks versions per device and prefers direct play, which Jellyfin does not do (it sorts by resolution); Plex's selection rule is unverified, so the edge over Plex is (unverified). | Version capabilities per file | Version picker marks "Best for this device" |
| LIB-155 | History per edition | Watching the Extended cut does not mark the Theatrical cut, while the film still shows overall progress | Plex separate for TV editions | Medium: part of the editions requests | R2 | Watch-log entries name the edition; the work's progress is derived from them | Edition key in watch events | Detail-page progress |
| LIB-156 | Merge and split versions by hand | Group or ungroup files when automatic grouping is wrong | Plex merge and "split apart" (unverified); Jellyfin "group versions" (unverified) | Medium: needed whenever grouping guesses wrong | R2 | Stored in the curation log and undoable | Override records | Item menu "Merge versions" and "Split" |

### Extras and trailers

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIB-157 | Local extras for films | Featurettes, deleted scenes, interviews and trailers appear under the film | Plex by folders and suffixes (unverified); Jellyfin and Emby by folders and suffixes; Infuse accepts all three conventions | Medium: table stakes | R2 | Accepts every rival convention, so existing folders work | Extra records with type | Detail page "Extras" row |
| LIB-158 | Extras for TV | Series, season and episode extras | Emby all three levels; Jellyfin supported; Plex request (434 votes) closed in 2018 with an unclear outcome | High: Plex 434 votes | R2 | Extras attach at any level of the show tree | Extra relations | Show, season and episode pages |
| LIB-159 | Real titles for extras | Extras show readable names, not file names | Nobody; Emby gives episode extras no online metadata | Low: no vote data | R2 | Titles come from embedded tags or cleaned file names | Title derivation | Extras row |
| LIB-160 | Theme music and backdrop videos | Ambient audio and video on detail pages | Jellyfin yes; Plex removed it from its new apps | Low: Jellyfin theme volume request (103 votes) shows it is used | Later | Local files only | Theme relations | Detail page; client setting |
| LIB-161 | Online trailers | Trailers without storing files | Plex (Plex Pass); Jellyfin plugin requested (#55, 585 votes); an Infuse thread reports trailers unavailable (December 2025) | High: 585 votes | Later | A plugin with a network grant that links to trailers the provider lists | Plugin | Detail page "Trailer" button |
| LIB-162 | Download trailers into media folders | Trailers saved beside each film | tinyMediaManager does it; Plex and Jellyfin do not, Emby not (unverified) | Low: no vote data | No | See "Deliberately not doing" | None | None |

### Collections

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIB-163 | Manual collections | Group films, shows or albums by hand | Plex, Jellyfin, Emby yes | Medium: table stakes | R2 | Collections live in the curation log, so they survive rebuilds and reinstalls | Collection records | Collection page; "Add to collection" on items and multi-select |
| LIB-164 | Collection order | Sort by release date, title or a custom order such as chronological | Plex custom order implemented; Jellyfin requested (#1577, 148 votes) | High: 148 votes | R2 | parity | Order per collection | Collection editor |
| LIB-165 | Collection artwork | An automatic mosaic or a custom image | Plex, Jellyfin, Emby yes (unverified) | Low: Infuse request (12 likes) | R2 | Uses the upload store (LIB-138) | Collection image slot | Collection editor |
| LIB-166 | Smart collections | Rule-based collections that update themselves, such as "unwatched 4K films" | Plex yes; Jellyfin requested (#593, 104 votes); Kometa fills the gap for Plex | High: 104 votes and Kometa's popularity | R2 | The same rule language as smart playlists, compiled into server and clients from the core, so a smart collection evaluates offline on the device | Rule definitions | Collection editor rule builder; "Save filter as collection" |
| LIB-167 | Automatic franchise collections | Film series grouped automatically, with no one-film collections | Emby with a minimum size; Plex hides single-item collections (195-vote request, implemented) | Medium: 195 votes | R2 | Built from provider collection data with a minimum size per library | Provider collection IDs | Library settings > Collections |
| LIB-168 | Nested collections | Trilogies inside a franchise | Nobody: Plex request 354043 (514 votes, open since 2018) | High: 514 votes | R2 | Collections may contain collections, with a cycle check | Parent relation | Collection page lists child collections |
| LIB-169 | Films and shows together | A franchise collection holds films and series | Jellyfin shows in collections since 10.11; request to tag films as part of a show (#2214, 104 votes) | High: 104 votes | R2 | Collections hold any item kind | Mixed membership | Collection page |
| LIB-170 | Collections from online lists | IMDb, Trakt or Letterboxd lists kept in sync | Kometa for Plex only; Jellyfin plugins (unverified) | Medium: Kometa's popularity; Jellyfin support in progress in Kometa | Later | Plugins with network grants that write ordinary collections | Plugin | Collection editor "Source" |
| LIB-171 | Collections as a config file | Power users define collections declaratively and keep them under version control | Kometa's config files, for Plex | Low: Kometa users | Later | Imports and exports the same records the curation log holds | Import and export | Admin > Libraries > Collections import |

### Editing, locks and curation

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIB-172 | Edit any field | Change titles, artists, dates, genres, summaries and sort names | Plex, Jellyfin, Emby yes (unverified); Plex on phones since March 2026; Plexamp tag editor requested (17 votes) | High: table stakes | R2 | Edits are overrides in the curation log; files are never rewritten (LIB-007) Owns in-app editing; MUS-046 points here. R2: field overrides replayed on rebuild depend on the durable user-state ADR, and R1 keeps only the targeted fixes listed in LIB-179. | Override records with author and time | Edit sheet on every item, on web, phone and TV |
| LIB-173 | Edits lock the field | An edited field is never overwritten by a rescan or refresh | Plex locks on edit, but posters reportedly ignore it; Jellyfin field locks | High: Plex 194 votes; Infuse "overwritten" thread | R2 | One rule for every field, images included, enforced in one place and tested per field | Lock flag per field | Lock icon per field in the Edit sheet |
| LIB-174 | See what is locked | Find edited and locked items, and unlock in one step | Jellyfin requests for a lock indicator (#3769) and a lock filter (#1178) | Low: two small Jellyfin requests | R2 | parity | Locked-field index | Filter chip "Edited"; Edit sheet "Unlock all" |
| LIB-175 | Lock an item or a library | Freeze a curated album, film or whole library | Jellyfin item locks and NFO `lockdata`; library lock requested (#2998, 14 votes) | Low: 14 votes | R2 | parity | Item and library lock flags | Item menu; Library settings |
| LIB-176 | Bulk edit | Change one field on many items at once | Plex multi-select editing (unverified); Jellyfin requested (#144, 186 votes) | High: 186 votes | R2 | One curation-log entry per bulk change, so the whole change undoes in one step | Batch override operation | Multi-select > Edit |
| LIB-177 | Edit as a spreadsheet | Export fields to CSV, edit them elsewhere, import the changes | Jellyfin "edit as a text file" requested (#852, 23 votes) | Low: 23 votes | Later | Import shows a dry-run diff before anything is applied | CSV round trip | Admin > Libraries > Export and import fields |
| LIB-178 | Undo and edit history | Revert any edit, merge, match or lock, and see who changed what | Nobody (Plex and Emby unverified) | Medium: a gap in every product; Jellyfin provenance request (44 votes) | R2 | The curation log is append-only, so history and undo come from it directly | Log queries; revert entries | Item page > History; Admin > Recent changes |
| LIB-179 | Fixes survive everything | Rebuilds, reinstalls, upgrades and restores keep every edit, lock, merge, chosen image and collection | Jellyfin and Emby only when NFO saving is on; Plex keeps fixes in its database (unverified) | High: Plex Dance, lock-poster and backup complaints | R1 | Curation becomes durable, exportable data like watch history, and the SQLite cache is rebuilt from files plus the logs (needs a new record extending record 1) In R1 the replayed fixes are only the targeted ones the heuristics need: artist merge, split and alias (MUS-007), album merge and split (LIB-058), and release-type and explicit overrides (MUS-010, MUS-047). General field edits arrive with LIB-172 in R2. | Append-only curation log keyed by content identity; replay on rebuild | Admin > Libraries > "Rebuild library", which states that fixes are kept |
| LIB-180 | Labels | Tag items with your own words, such as "Christmas" or "kids", for rules and filters | Plex labels (custom label rules need Plex Pass); Jellyfin and Emby tags; Spotify "Tag Music" request (1,888 votes) | High: 1,888 Spotify votes; Plex 866-vote tag request | R2 | A label belongs to the household: it is a curation-log entry made by someone with edit rights, visible to every profile, and usable in smart-playlist rules and parental allow and block lists (ACC-028). A personal tag (MUS-023) belongs to one profile and is private to it. Both ship in R2 with in-app editing (LIB-172) | Label relation | Edit sheet; multi-select > Add label; filter chips |
| LIB-181 | Delegated editing | Trusted people who are not admins can fix art and metadata | Nobody; Jellyfin requested (#3496, 7 votes) | Low: 7 votes | Later | An editor permission from the users map; every edit is attributed in the log | Editor role | Edit sheet for editors |
| LIB-182 | Bring curation from another server | Matches, locks, collections and edits from Plex, Jellyfin, Emby or Navidrome | Nobody offers a portable format; Kometa keeps customisations outside Plex's database | Medium: switching cost (operations research) | Later | Importers write curation entries tagged with their source, removable as a group; watch-history import is in the operations map | Importers that read copies of rival databases | Admin > Import |

### Languages

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIB-183 | Metadata language per library | Each library fetches titles and plots in its own language | Plex per library, plus a per-film choice with the Plex Movie agent; Jellyfin per library (unverified) | Medium: Plex and Infuse users still ask for finer control | R2 | parity | Language and region per library | Library settings > Language |
| LIB-184 | Several languages at once | Each person sees titles and plots in their own language from one library | Nobody: Jellyfin #610 (212 votes); Plex users create a second library | High: 212 votes | R2 | Localised fields are stored per language and synced, and each person or device chooses | Localised field store; extra provider calls per language | User settings > Metadata language |
| LIB-185 | Keep the original title | Show "Le Samouraï" rather than a translated title | Plex per-film option; Jellyfin requested (#32, 315 votes) | High: 315 votes | R2 | The original title is stored beside localised titles; a per-person switch picks which to show and sort by | Original-title field | User settings; detail page shows both |
| LIB-186 | Per-item language | One film or show uses a different language from its library | Plex per film | Low: Plex and Infuse users ask for per-folder language | R2 | Override in the curation log | Per-item override | Edit sheet > Language |

### People across libraries

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIB-187 | One artist page across libraries | An artist page lists everything from every library the viewer may see | Plex people across libraries (185579, 254 votes, implemented); Jellyfin's artist view leaked albums across libraries (#17215) | Medium: 254 votes; #17215 | R1 | Artist identity is server-wide and access is filtered per object (record 1) | Server-wide artist index | Artist page |
| LIB-188 | Cast and crew | Actor and director pages with photos, listing every title they appear in | Plex, Jellyfin, Emby yes | High: table stakes | R2 | People are matched by provider ID, not by name (Plex's NFO agent matches cast by name only) | Person entity with provider IDs | Person page; cast row on detail pages |

### Music videos, recordings and playlist files

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIB-189 | Music videos linked to tracks | A song offers its video, and an artist page lists live sets and interviews | Plex links by file name and typed suffixes; Jellyfin keeps them separate (#896, 38 votes; #3782, 9 votes) | Medium: 38 votes | R2 | Uses the typed relations in the first schema (LIB-011) and accepts Plex's suffixes | Relations between videos and tracks or artists | Track menu "Watch video"; artist page "Videos" |
| LIB-190 | Concert chapters mapped to songs | Jump to a song in a two-hour concert film | Nobody (unverified) | Low: no vote data | Later | Chapters link to tracks through typed relations | Chapter-to-track relations | Concert page track list |
| LIB-191 | Recordings join the library | DVR recordings appear as normal episodes and films, matched to existing shows | Plex yes, in a chosen library; Jellyfin a recordings library; Emby a mixed library by default | Medium: live TV research | R3 | Recordings arrive with guide IDs that identify them exactly, and scans never delete them (LIB-007) | Recording intake with guide metadata | Show and film pages |
| LIB-192 | Playlist files in music folders | `.m3u` and `.m3u8` files found in a library become playlists | Navidrome imports them automatically; Jellyfin creating and editing M3U requested (17 votes) | Low: 17 votes | R1 | Uses the R1 M3U parser and matcher from MUS-140; the live TV module reuses that parser in R3. This is unrelated to IPTV M3U sources (LIV-003). Paths resolve through relative paths and identity. | Playlist import | Playlists list (music map) |

### Library health and reports

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIB-193 | Damaged and unreadable files | A list of truncated, corrupt and unreadable files, with what is wrong and where | Nobody | Medium: no server reports them; a Hacker News user had a corrupt file play at painful volume | R1 | The core returns typed errors with a location, so the report is nearly free | Parse-error records per file | Library health > Problems |
| LIB-194 | Tag problems | Tracks missing an album artist, albums with inconsistent tags, ambiguous artist splits and guessed compilations | Navidrome `inspect` per file; nobody lists them all | Medium: the music research asks for a report of ambiguous splits | R1 | Every grouping decision records its reason, so the report is a query | Decision reasons | Library health > Tag problems, with fix actions |
| LIB-195 | File inspector | See ADM-125, which owns this feature. Library specifics: raw tags beside interpreted values, and the reason for each grouping. | Navidrome `inspect` command | Medium: Navidrome's CLI | R1 | See ADM-125. | None beyond ADM-125. | Item menu > File info; CLI command |
| LIB-196 | Duplicates | Find the same recording or film stored twice, with format and size, to choose what to keep | Plex duplicates filter (unverified); Jellyfin requested (#31, 135 votes; music duplicates 15 votes) | High: 135 votes | R2 | Duplicates are found by identity (MusicBrainz ID, fingerprint and, from R2, provider ID), not by title R2, with a tag-and-MusicBrainz-ID-only rule; matching on audio features is MUS-030 (Later). Owns duplicates. | Duplicate groups | Library health > Duplicates |
| LIB-197 | Missing artwork or metadata | Find items without covers, years, genres or IDs | tinyMediaManager filters (unverified); not found for the three servers | Low: no vote data | R2 | Computed from synced fields, so the filter also works on devices Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Completeness flags | Library health > Incomplete; filter chips |
| LIB-198 | Technical quality report | See which albums are lossy or low bitrate and, from R2, which films are SD or lack HDR | Not found in sources; external tools (unverified) | Low: no vote data | R2 | Uses scan data already stored (LIB-063, LIB-087) Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Quality queries | Library health > Quality |
| LIB-199 | Orphaned sidecars | Lyrics, CUE sheets and artwork (and, from R2, subtitles and NFO files) left behind after their media was removed | Nobody | Low: no vote data | R2 | The scanner already associates sidecars, so unattached ones are known Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Unattached sidecar list | Library health > Orphans |
| LIB-200 | Missing episodes | Gaps in a season shown as placeholders, with switches for specials and unaired episodes | Plex no (77834, 327 votes, open since 2014); Jellyfin only with TheTVDB (#572, 343 votes); tinyMediaManager yes | High: 343 and 327 votes | R2 | Episode lists come from whichever provider plugin is active; placeholders never appear in normal browsing, so shows with no files stay hidden (Jellyfin #41, 190 votes) | Cached episode lists with age; placeholder items | Season page "Show missing"; Library health > Missing episodes |
| LIB-201 | Missing albums | See MUS-026, which owns this feature. | Navidrome requested (#5106, 12 reactions) | Low: 12 reactions | Later | See MUS-026. | None beyond MUS-026. | Artist page switch |
| LIB-202 | Changed-file report | Find files whose content changed without a new modification time | Immich 3.0 compares checksums | Low: one rival | Later | A verification job re-checks the stored content hash | Verify job | Library health > Integrity |
| LIB-203 | Reports export | Any report or filtered list downloads as CSV or JSON | Navidrome missing-files CSV | Low: Navidrome only | R2 | parity Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Export endpoint | "Export" on every report |

## Differentiators

These are the features in this area most likely to make someone switch.

1. **Rescans that cost nothing and lose nothing** (LIB-016, LIB-018, LIB-015,
   LIB-032, LIB-019, LIB-020). Jellyfin 10.11 re-saves thousands of items on
   an unchanged scan (#18274) and turns change events into full rescans
   (#16729), and Jellyfin 10.9 ran a 4 GB Pi out of memory with ffprobe
   processes (#11588). No rival picks up new files on a network share without
   a scan, and Jellyfin's own docs warn that maintenance can remove items
   while storage is offline. Gunmetal's scanner produces a diff, polls shares
   cheaply, never removes anything from an offline root, and turns the same
   diff into the change feed devices sync from. The claim can be measured, and
   the scan benchmark against Jellyfin will publish it.
2. **No Plex Dance, ever** (LIB-028, LIB-029, LIB-030, LIB-031). Plex users
   have a named ritual for unsticking items, and threads about it failing run
   from 2019 to 2025; Jellyfin 10.11 lost watched state when files were
   replaced or renamed (#15001). Keying history and curation to content
   identity rather than paths means moving folders, upgrading to FLAC or 4K,
   or moving the whole server keeps everything.
3. **Fixes that stick, with undo** (LIB-179 in R1 for the targeted fixes;
   LIB-172 to LIB-178 and LIB-141 in R2). Plex's "Lock Posters" request has 194 votes and no staff reply in
   four years, and deleting unwanted posters has 333; Jellyfin's bulk edit has
   186 and provenance 44. Because Gunmetal's database is a rebuildable cache,
   curation has to be durable data anyway; doing it as an append-only log
   gives history and undo for free and makes "locked" mean locked for every
   field.
4. **The music model Plex never built** (owned by `music.md`: MUS-001,
   MUS-005, MUS-008, MUS-010, MUS-012 in R1; works, MUS-014, and CUE
   sheets, MUS-041, in R2). Plex's requests for robust tags (866 votes) and
   several artists per track (520) date from 2015, its CUE sheet request has
   507 votes, and Jellyfin regressed on multi-artist handling in 10.11.
   Matching Navidrome on tags in R1, then adding CUE sheets and works in R2,
   makes Gunmetal credible to exactly the people most likely to try a new
   music server.
5. **Versions and editions that choose themselves, free** (LIB-151, LIB-150,
   LIB-154, LIB-153; R2). Plex needs Plex Pass to set editions, Jellyfin has
   neither named editions (#2542) nor versions across folders (#174, 265
   votes), and Emby caps versions at eight. Picking, per device, the version
   that plays directly is something Jellyfin does not do (Plex's rule is
   unverified), and it serves the project's
   main bet on sending the original.
6. **Health reports nobody else has** (LIB-193, LIB-194 and ADM-125 in R1;
   duplicates, LIB-196, in R2; LIB-200). Missing episodes are requested with 343 and 327 votes and
   duplicates with 135, and no server reports corrupt files. Gunmetal's
   parsers already return typed errors with locations and its grouping already
   records reasons, so these reports are queries over data the scan has.

## Deliberately not doing

- **Writing into media folders** (LIB-133, LIB-162). The server never writes
  tags, NFO files, artwork or downloaded trailers next to media. Read-only
  media is a security property: folders can be mounted read-only, a bug in the
  scanner cannot damage a collection, and Jellyfin's scan that deleted
  finished recordings (#17622) shows what is at stake. People who want
  portable curation get an export to a separate directory (LIB-132) instead.
- **Renaming or moving files** (LIB-085). tinyMediaManager and similar tools
  already do this well, and a server that moves files can lose them. Gunmetal
  explains naming problems instead (LIB-084).
- **Disc folders and disc images** (LIB-086). Supporting `VIDEO_TS`, `BDMV`
  and ISO means a large new parsing surface and disc navigation for a small
  audience; Jellyfin's ISO support is absent and its disc folders cannot have
  versions or external subtitles. Kodi, which improved disc playback in v21,
  serves those users better.
- **A streaming catalogue mixed into the library, and reviews** (LIB-124,
  LIB-125). Plex users voted 1,000 times to switch the catalogue off and 185
  times to switch reviews off. Gunmetal has no central account and no
  catalogue to promote; a plugin can add reviews for people who want them.
- **Badges burned into poster files** (LIB-147). Kometa itself warns that old
  overlays pile up and that manual poster changes cause double overlays.
  Clients draw badges from data instead (LIB-146).
- **Metadata or fingerprint lookups in the server core.** Record 2 puts every
  third-party lookup in a plugin with an explicit network grant, so the core
  never contacts anything on its own.
- **Paywalls.** Editions, artwork, lyrics, health reports and everything else
  in this area are part of the AGPL server; Plex gates edition editing and
  artist art behind Plex Pass, and Emby gates poster overlays and backup.

## Dependencies and risks

- **Blocking: ADR 3, durable user state.** ADR 1 (decision 5) names watch
  history as the only irreplaceable data, but most rows in this map that
  write user data (playlists, loves, ratings, corrections, layouts, shares,
  identities, IDs, settings) need it to survive a cache rebuild. ADR 3 must
  be accepted before any server work that stores user data. It defines the
  two durable stores every map now names the same way: **the user log** and
  **the identity store** (see the [feature map README](README.md)).
- **The curation log needs an architecture record first.** Record 1 names
  watch history as the only irreplaceable data. Edits, locks, merges, chosen
  artwork, labels, collections and uploads are just as irreplaceable, and if
  any of them lives only in SQLite then "rebuildable cache", small backups and
  safe rollback are all false. The record must settle this before the server
  schema is written.
- **Identity rules can silently move history.** Remuxes, re-encodes, tag edits
  and duplicated files all stress them, and a wrong rule merges or splits
  someone's history without anyone noticing. Fingerprints must be cheap on
  network storage and stable across tag edits at the same time. The rules need
  their own record and property tests.
- **The plugin host is on the critical path for R2.** Music can ship
  tags-only, as Navidrome does, but nobody accepts a movie library without
  posters. A WebAssembly host with grants, rate limits and a stable interface
  is months of work, and Navidrome needed two security fixes to its network
  guard after launch.
- **Provider terms limit what can ship.** TMDB's free licence forbids
  commercial use, requires attribution and forbids caching data for more than
  six months, which collides with devices that hold a synced library offline.
  TheTVDB requires a commercial licence or end-user subscriptions with a PIN.
  MusicBrainz allows about one request per second, so a first lookup of 5,000
  albums takes at least 83 minutes. AcoustID's free tier is non-commercial.
- **Untrusted input everywhere.** NFO and TTML files are XML from unknown
  sources and must be parsed with no DTD or entity expansion. Images need size
  and pixel limits before decoding. CUE sheets, LRC files and subtitle
  sidecars must return typed errors like every other parser, and all of them
  should be fuzzed.
- **Scan cost on slow storage.** Matroska cues often sit at the end of the
  file, which means a seek per file on a NAS; content hashing adds reads; some
  files have no index at all. Each must happen once per file and be skipped
  for unchanged files.
- **Loudness measurement needs decoding.** On low-end hardware, measuring a
  large untagged library takes many hours. Symphonia covers FLAC, MP3, AAC and
  Vorbis but lists Opus as in development, so Opus measurement needs another
  decoder or the sandboxed worker that record 1 reserves for transcodes.
- **The naming long tail never ends.** Release-group naming, anime, daily
  shows, specials and foreign titles need a large test corpus, and any corpus
  borrowed from another project needs a licence check. TV episode ordering is
  genuinely ambiguous and will produce edge cases forever.
- **Compatibility adapters must map the model down.** The Jellyfin and
  OpenSubsonic adapters expect their own ID schemes and often assume one
  artist per album; the music model has to translate without losing data.
- **The change feed has to scale.** A library of about a million tracks must
  sync to a phone in a compact form; Navidrome only recently fixed duplicate
  and missing pages in its full-library sync.
- **Sibling maps depend on this one, and this one on them.** Playback uses the
  gapless, loudness, segment-map, version and marker data; clients consume the
  change feed; the users map enforces per-library access, content ratings and
  the editor role; the operations map backs up the logs, runs importers and
  rebuilds; the integrations map delivers library events and issues scoped
  tokens; the discovery map builds search and filters on these fields. R1 also
  depends on the core's FLAC, MP3, MP4 and Ogg tag, artwork and loudness
  parsers from the README roadmap.
- **R1 is large.** It holds 93 of this map's 203 features. Many are small
  queries over scan data, but the plan should mark which ones can slip to a
  point release without hurting the first impression, so the music release is
  not held back by reports and settings.
- **Performance claims are design goals until measured.** The scan benchmark
  against Jellyfin, reporting time, bytes read and memory, is what turns the
  first differentiator from a promise into a fact.

## Open decisions for the project owner

1. **Where curation lives.** Should edits, locks, merges, artwork choices,
   labels and collections go into an append-only log next to watch
   history, extending record 1? *Recommendation:* yes, one user log with
   typed, versioned entries that each carry a scope (one person or the
   household; household entries are the curation log), keyed by content
   identity. One log means one backup, export and rebuild path. This is ADR
   3 in the feature map README; write it before the server schema.
2. **Whether the server ever writes into media folders.** *Recommendation:*
   never in place. Offer export of NFO and JSON to a separate directory or as
   a download (LIB-132), and revisit an opt-in, per-library writer only if
   users ask for it after R2.
3. **How much music metadata R1 fetches.** Tags only, as Navidrome ships, or a
   MusicBrainz and cover-art plugin from day one, which pulls the plugin host
   into R1? *Recommendation:* tags only in R1, with local artwork and artist
   images from folders. Build the plugin host early in R2 and make
   MusicBrainz, Cover Art Archive and artist images its first plugins. Accept
   that R1 artist pages will look less finished than Plex's.
4. **Whether CUE sheets are in R1.** *Decided:* R2, owned by MUS-041 (see
   music open decision 8). A browser cannot play a sample-accurate sub-range
   of a FLAC file from a byte range alone, so each virtual track needs a
   re-headed slice cut at frame boundaries, which shares the frame index
   with the video segment map.
5. **Whether loudness is measured by default.** Decoding every untagged file
   costs hours on small machines. *Recommendation:* on by default at the
   lowest priority, resumable, with an off switch and an optional maintenance
   window; Opus files wait for a suitable decoder. This needs an ADR
   approving in-process pure-Rust decoders first (MUS-086); without it, R1
   uses tags plus the MUS-089 fallback gain.
6. **Which identity rule decides "same item, new file, or new item".**
   *Recommendation:* use the single identity order now written in LIB-028
   (MusicBrainz recording ID, FLAC STREAMINFO MD5, a bounded audio-frame
   hash that skips tag regions, then the path stem). A tag edit never
   creates a new item; a re-encode without MusicBrainz IDs at a new path is
   matched by tags and duration and goes to the review queue, never to a
   silent merge. Record it in an architecture record.
7. **Video providers and keys (legal exposure).** Should Gunmetal ship a
   project TMDB key, and who is the licensee? Is TheTVDB worth a negotiated
   licence? *Recommendation:* apply for a non-commercial TMDB key in the
   project's name, let users override it with their own, and have the host
   enforce attribution and cache age; get a legal reading of the six-month
   cache rule against offline devices before R2. Go TMDB-first, as Infuse did
   in 2021, and offer TheTVDB later as a bring-your-own-key plugin with the
   subscriber PIN, accepting weaker anime and non-English ordering at first.
8. **How editions share history.** *Recommendation:* each edition keeps its
   own watch history, the work shows combined progress, and versions within
   one edition (1080p and 4K) share history.
9. **Which metadata languages are stored by default.** Every extra language
   costs provider calls per item. *Recommendation:* store the server's
   language and the original language by default; other languages are opt-in
   per library.
10. **Online trailers (legal exposure).** *Recommendation:* a later plugin
    that only links to trailers a provider lists and never downloads or
    re-hosts them, so the project carries no redistribution risk.
