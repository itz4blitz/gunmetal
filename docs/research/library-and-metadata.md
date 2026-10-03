# Library and metadata

Research date: 2026-10-02. Written with live web access (search and page
fetches). Some vendor pages refused automated fetching (Plex support
articles, the Kodi wiki, fanart.tv). Where a claim about them comes from a
search-engine summary of the page rather than the page itself, the Sources
section says so. Anything marked "(unverified)" comes from the author's prior
knowledge and was not confirmed against a source during this research.
"Not found in sources" means the sources reviewed said nothing either way.

## Scope

This document covers everything between "files on a disk" and "a browsable,
correct library": how the server finds files and notices changes, how file
and folder names are parsed, how items are identified and matched to online
databases, where metadata and artwork come from (online providers, NFO
sidecars, embedded tags), how one title with several files is modelled
(versions, editions, multi-part), extras and trailers, collections, how users
fix mistakes and stop the server from undoing them, setups with several
libraries, and the health tools that find duplicates, gaps and broken files.

It covers music as well as movies and TV, because
[record 2](../adr/0002-music-is-first-class.md) makes music first-class. Two
architecture records shape the recommendations:

- [Record 1](../adr/0001-architecture.md): the scan uses pure-Rust parsers
  and builds a segment map, FFmpeg is off the hot path, SQLite is a
  rebuildable cache, watch history is the only irreplaceable data and lives
  in an append-only log, and the library is synced to devices for instant
  and offline browsing.
- [Record 2](../adr/0002-music-is-first-class.md): there is a real music model
  (artists, release groups, albums, tracks), tags, artwork, gapless data and
  loudness are read at scan time, and metadata lookups that reach third-party
  services belong in plugins with explicit network grants, not in the server.

Products studied: Plex, Jellyfin, Emby, Kodi, Infuse, tinyMediaManager (TMM),
Kometa and Navidrome (as the strongest tag-driven music server). Providers
studied: TMDB, TheTVDB, MusicBrainz, Cover Art Archive, fanart.tv,
TheAudioDB and AcoustID.

Out of scope here and covered by sibling documents: playback, transcoding,
users and permissions, sync protocol details, and the player UI. Some rows
touch those areas where the library has to supply the data.

## Feature inventory

Columns follow the template. Where the video servers are not the relevant
rivals (the music identification table), the three rival columns are Plex,
Jellyfin and Navidrome instead. Kodi, Infuse, TMM and Kometa appear in the
"Best in class" and "Notes" columns where they lead.

### Library setup

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Library types | Separate libraries for movies, TV, music and other content, each with rules suited to it | Movies, TV, music, photos, other videos (unverified) | Movies, shows, music, music videos, books, photos and a mixed type (mixed type confirmed in a GitHub discussion; full list unverified) | Movies, TV, music and an "unset" type for mixed content (Emby library setup docs) | Jellyfin, for the widest set of types | Gunmetal v1 needs movies, TV and music per record 2. |
| Several folders per library | One library spanning several disks or shares | Yes (unverified) | Yes (unverified) | Yes; several physical paths shown as one library (Emby docs) | Tie | Common for JBOD and mergerfs setups. |
| Mixed-content library | One folder holding movies, shows and home video | No dedicated mixed type (unverified) | Mixed type exists | Unset type exists (Emby docs) | Emby and Jellyfin | Matching quality drops in mixed libraries; a Jellyfin request asks for a switch to turn fetching off entirely there (#3733, 2 votes). |
| Exclusion rules | Tell the scanner to skip folders or files | `.plexignore` files (mentioned in Plex forum workarounds) | Not found in sources | Not found in sources | Navidrome `.ndignore` and Plex `.plexignore`; Infuse also documents excluding files and folders | A gitignore-style file is the expected shape. |
| Per-library language and country | Each library fetches metadata in its own language | Per-library language; the Plex Movie agent also allows a per-movie language and original-title choice (Plex forum, via search summary) | Per-library (unverified) | Not found in sources | Plex, for the per-item override | Users still ask for per-folder language in Plex and Infuse. |
| Several metadata languages at once | Each user sees titles and plots in their own language from one library | No; users create a second library (Plex forum thread 425063) | No; request #610 has 212 votes | Not found in sources | Nobody | A clear gap in every product. |
| Keep the original title | Show "Le Samouraï", not a translated title | Per-movie original-title option with the Plex Movie agent (via search summary) | Request #32 "Keep Original Title" has 315 votes and is open | Not found in sources | Plex | |
| Folder view | Browse the library as the disk tree, useful for home video and stragglers | Not found in sources | Request #224 has 438 votes and 134 comments, open | Folder views exist alongside metadata views (Emby movie naming docs) | Emby | |
| Network storage | Library on a NAS over SMB or NFS | Works; change detection does not (see scanning) | Works; real-time monitoring is not supported over NFS or rclone (Jellyfin troubleshooting docs) | Real-time monitoring needs a supported filesystem (Emby docs) | Nobody solves change detection here | |
| Media stays read-only | The server never writes into media folders unless asked | Metadata lives in Plex's own data directory (unverified) | Writes only if NFO or image saving is enabled (unverified as default) | Same as Jellyfin (unverified) | Plex-style default | Matters for Gunmetal's security goals: media can be mounted read-only. |

### Scanning and change detection

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Manual scan | A button that rescans a library | Yes | Yes | Yes | Tie | |
| Scheduled scan | A periodic safety-net rescan | "Scan periodically" setting (unverified) | Scheduled task (unverified) | Scheduled task (unverified) | Tie | |
| Real-time file watching | New files appear within seconds of landing on disk | "Run a partial scan when changes are detected" (setting named in Plex forum thread 854832); relies on inotify, so local disks only | Real-time monitoring; Linux users often must raise `max_user_watches` to 524288, on the host for Docker (Jellyfin docs) | Real-time monitoring; requires a supported filesystem and a server restart after enabling (Emby docs) | Plex, because it rescans only the changed folder | |
| Partial scan of the changed folder | Adding one episode does not rescan the whole library | Yes | Worked in 10.10.7; in 10.11.x the monitor triggers a full rescan (issue #16729, opened April 2026, open) | Not found in sources | Plex | |
| Change detection on network shares | Files copied to a NAS show up without a manual scan | Not possible by notification: SMB and NFS have no server-to-client change events, and inotify only sees local changes (Plex forum threads 642609 and 930098) | NFS and rclone unsupported; mergerfs suggested as a workaround (docs) | Not found in sources | Nobody | Needs polling with cheap directory checks. |
| Cost of a no-change scan | Rescanning an unchanged library does nothing and tells clients nothing | Generally regarded as quick (unverified) | Issue #18274 (10.11.11, opened 2026-10-02): an unchanged 52,140-item library re-saves about 3,980 items with 4,770 database updates, and Kodi clients then show 1,500+ pending updates | Not found in sources | Nobody publishes numbers | This is the property Gunmetal's scan benchmark should prove. |
| Move and rename detection | Moving or renaming a file keeps its watch history, edits and playlists | Users fall back on the "Plex Dance" (remove, scan, empty trash, re-add) to unstick items; forum threads from 2019 to 2025 | Not found in sources (moved files are generally re-added as new items, unverified) | Not found in sources | Navidrome: persistent IDs match moved files, and a CLI command remaps history when matching fails | |
| Missing and offline media | An unplugged drive does not wipe the library | Missing items go to a trash that can be emptied automatically after scans (unverified) | Not found in sources | Not found in sources | Navidrome: missing files are kept by default, purge policy is never, always or after full scans, and the list exports to CSV | |
| Scan progress and activity | See what the scanner and fetchers are doing | Activity view (unverified) | Scheduled-task progress; request #3843 asks for more visible activity (1 vote) | Not found in sources | Not clear | |
| Cancel or reprioritise a refresh | Stop a long refresh; new files jump the queue | Not found in sources | Request #3771 (2 votes) | Not found in sources | Nobody | |
| Stream analysis at scan | Codec, resolution, HDR, channels and duration known before anyone presses play | Media analysis on add (unverified) | ffprobe at scan (unverified) | ffprobe at scan (unverified) | Tie | Gunmetal does this with its own parsers and also stores the segment map (record 1). |
| Upgrade-driven rescans | Upgrading the server does not force hours of rescanning | Not found in sources | 10.11 told users to run a full scan and a missing-metadata scan for music; migration could take several hours (release notes) | Not found in sources | Not clear | A rebuildable cache makes this a design question for Gunmetal too. |
| Replacing a file with a better copy | History stays; derived data (chapters, thumbnails, subtitles) is regenerated | Not found in sources | 10.11 prunes chapters, trickplay files and extracted subtitles on replacement and regenerates them (release notes) | Not found in sources | Jellyfin 10.11 | |
| Chapter images and preview thumbnails | Scrubbing and chapter menus with pictures | Chapter and preview thumbnails (unverified) | Chapter images; trickplay with roughly 100x faster keyframe-only extraction since 10.10 (release notes) | Option to extract chapter images during scans (Emby docs) | Jellyfin, for the speed-up | Needs decoding; in Gunmetal it would run in the sandbox as background work. |

### Naming conventions and parsing

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Movie naming | `Title (Year)` files and folders are recognised | Yes (unverified) | One folder per movie, `Name (year) [provider id]`, year and ID optional (docs) | `Movies\Name (year)\Name (year).ext` (docs) | Tie | |
| Provider ID hints in names | Put an ID in the name and the match is exact | `{tmdb-…}`, `{imdb-…}`, `{tvdb-…}` (unverified) | `[imdbid-tt…]`, `[tmdbid-…]` on folders and files (docs) | `[tmdbid=…]`, `[tmdb-…]`, `[imdbid=…]`, `[tvdbid=…]`, square or curly brackets (docs) | Emby, for the most forgiving syntax; Infuse also accepts TMDB or IMDb IDs in curly brackets | Jellyfin request #3247 asks it to use the provider whose ID is in the name (3 votes). |
| TV episode naming | `S01E01`, `1x01` and season folders recognised | Yes (unverified) | `Season NN` folders required, `S01` not accepted (docs) | `S01E01`, `1x02`, season folders (docs) | Emby, for accepting more forms | |
| Specials | Specials in Season 00, optionally shown where they aired | Season 00 (unverified) | Season 00, with `airsbefore` placement in the aired season (docs) | Season 0, 00 or a "Specials" folder (docs) | Jellyfin, for in-season placement | Infuse users ask for the same placement (request 20110, 59 likes). |
| Date-based episodes | Daily shows named by air date | Yes (unverified) | Not found in sources | `1996-11-14.ext` (docs) | Emby | |
| Multi-episode files | One file containing two episodes | Implemented (Plex forum request 187347, 255 votes) | Supported, though splitting is recommended (docs) | `S01E02-E03`; must stay within one season (docs) | Tie | The Plex NFO agent needs one NFO per episode for these. |
| Absolute numbering (anime) | Episode 1037 maps to the right season | Through third-party legacy agents such as HAMA (unverified), which lose support when legacy agents are removed | AniDB plugin (docs) | Not found in sources | Not clear | |
| Multi-part movies | `cd1`/`cd2` files play as one movie | Yes (unverified) | `cd`, `dvd`, `part`, `pt`, `disc`, `disk` with flexible separators (docs) | Same words, numbered 1 to 9 or A to D, alone in the folder (docs) | Tie | The Plex NFO agent did not support multi-part files at first. |
| Disc structures | `VIDEO_TS`, `BDMV` and ISO images | Not found in sources | ISO unsupported; disc folders cannot have versions or external subtitles (docs) | Not found in sources | Kodi, which improved disc playback in v21 | Likely out of scope for Gunmetal v1. |
| 3D tags | 3D files recognised as such | Not found in sources | `3D` plus `hsbs`, `fsbs`, `htab`, `ftab`, `mvc` (docs) | `.hsbs` and others (docs) | Jellyfin | |
| Flattened shows | Shows without season folders, or single-season shows shown flat | Not found in sources | Requests #3357 (172 votes) and #8 (160 votes) | Not found in sources | Nobody | |
| Naming diagnostics | The server explains why a file did not match and how to rename it | No | No | No | Nobody; TMM's renamer is the nearest thing | A gap Gunmetal can fill cheaply. |
| Renamer | Rename and move files into a convention | No | No | No (unverified) | TMM renamer (docs) | Gunmetal should not move media by default; a dry-run rename plan is safer. |

### Matching and identification

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Automatic matching | Titles found and filled in without effort | Plex Movie and Plex Series agents backed by Plex's metadata service (unverified) | TMDB and OMDb built in (docs) | TMDB, TheTVDB and OMDb (unverified) | Plex is widely regarded as the most accurate out of the box (unverified) | |
| Fix a wrong match | Search for the right title and apply it | "Fix Match" (unverified) | "Identify" (unverified) | "Identify" (unverified) | Infuse: fixing one episode re-matches the whole series (Firecore docs) | |
| Unmatched items list | See everything that failed to match | Unmatched filter (unverified) | Request #665 (101 votes) | Not found in sources | Not clear | |
| Match confidence and review queue | Doubtful matches are held for review instead of applied silently | Match dialog shows candidate scores (unverified) | No | No | Nobody | Gap. |
| Re-match without losing history | Fixing identification keeps watch state | The Plex Dance, which is itself a workaround | Not found in sources | Not found in sources | Nobody documents this well | |
| Episode orderings | Aired, DVD, absolute and alternate orders | TheTVDB alternate orders since PMS 1.40.4; TMDB episode groups still open (request 537737, 502 votes) | TheTVDB plugin; request #2031 for TMDB episode groups (15 votes) | Not found in sources | Plex for TheTVDB orders; TMM decides aired versus DVD order on import (docs) | Infuse users complained about episode order when it moved TV to TMDB. |
| Missing-provider fallback | Shows not in one database are found in another | Provider chains in custom metadata agents | Provider order per library | Not found in sources | Plex's new agent model and Jellyfin's per-library order | |
| Freshness for new episodes | A new episode gets its real title soon after airing | Not found in sources | Request #3025 asks for automatic refresh of recently aired episodes (1 vote) | Not found in sources | Infuse: TMDB edits reach Infuse within about a day (Firecore docs) | |
| Provider IDs exposed to tools | Other tools (Tautulli, Overseerr) can read IMDb, TMDB and TVDB IDs | Implemented after request 619090 (705 votes) and `includeGuids` (1,579 votes) | Provider IDs stored and exposed (docs) | Not found in sources | Tie | |

### Metadata providers

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Movie metadata | Plot, cast, genres, dates, ratings | Plex's own service (unverified sources) | TMDB, OMDb (docs) | TMDB, OMDb (unverified) | Tie | |
| TV metadata | Series, season and episode details | Plex Series agent; TheTVDB orders available | TMDB built in, TheTVDB plugin (docs) | TheTVDB and TMDB (unverified) | Tie | Infuse moved all TV metadata to TMDB in August 2021. |
| Third-party providers | Community sources for anime, adult, regional, or home media | Custom metadata providers announced 2025-12-09 in PMS 1.43.0 beta: HTTP services in any language, installed by URL; movie and TV only; no authenticated requests or provider preferences yet; legacy agents to be removed in 2026 | Plugins built against the plugin API (docs) | Plugins (unverified) | Plex's HTTP model is the easiest to write and runs out of process; Jellyfin's plugin catalogue is the most mature | Plex's change validates the out-of-process provider idea in record 2. |
| Provider order per library | Choose which source wins | An agent is an ordered list of providers (Plex announcement) | Per-library order (docs) | Not found in sources | Tie | |
| Per-field source choice | Plot from one source, ratings from another | Not found in sources | Requests #970 (5 votes) and #4116 (1 vote) | Not found in sources | Nobody | |
| Provenance | See which source each field came from | No | Request #968 (44 votes) and #1426 (5 votes) | Not found in sources | Nobody | Gap. |
| External ratings | IMDb and Rotten Tomatoes scores | IMDb ratings added to the TV agent (request 726420, 319 votes, implemented) | Request #463 (196 votes) | Not found in sources | Plex | Infuse lists IMDb and Rotten Tomatoes ratings as in progress for 8.5.7; its request has 321 likes. |
| Reviews | Critic and user reviews on detail pages | Shown; request to turn them off has 185 votes | Not found in sources | Not found in sources | Not applicable | Gunmetal should not show reviews by default. |
| Online catalogue mixed into your library | Your own media only, unless you opt in | Discover and streaming results mixed in; request to turn them off has 1,000 votes | Not applicable | Not applicable | Jellyfin and Emby, by not doing it | |
| Local-only operation | A usable library with no internet lookups | Local Media Assets and NFO providers can lead an agent (Plex NFO article, via search summary) | NFO plus fetchers turned off (TMM docs) | Not found in sources | Jellyfin | Record 2 makes this Gunmetal's default. |
| Refresh modes | "Find missing" versus "replace everything" | Refresh and match options (unverified) | "Replace all metadata and images" exists (GitHub discussion 16326); other modes unverified | Not found in sources | Tie | |
| Regional availability | Providers reachable from the user's country | Not found in sources | TMDB and TheTVDB are unreachable from mainland China (docs) | Not found in sources | Not clear | Regional providers are a natural plugin category. |

### Artwork

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Artwork types | Posters, backdrops, logos, clear art, banners, disc art, thumbnails | Posters and backgrounds; others unverified | Cover, backdrop, logo, folder and fanart filenames documented for music; more for video (docs) | Primary images, numbered backdrops (Emby music docs) | TMM: posters, season posters, fanart, banners, clear art, character art, thumbs, clear logos, disc art and key art (docs) | |
| Pick among available images | Browse candidates and choose one | Yes (unverified) | Yes (unverified) | Yes (unverified) | Tie | |
| Upload or link an image | Use your own art | Upload and URL (unverified) | Upload; "by URL" request #270 has 252 votes | Edit images (Emby collections docs) | Plex (unverified) | |
| Delete unwanted uploaded art | Remove a bad poster from the choices | Request 151538 (333 votes) | Not found in sources | Not found in sources | Nobody | |
| Chosen art stays chosen | Refreshes never replace a poster you picked | Users report posters changing despite selection; a user found the poster field does not honour the lock flag (request 755134, 194 votes, no staff reply) | Locks exist (requests about lock indicators confirm it); behaviour for images unverified | Not found in sources | Not clear | |
| Local artwork files | `poster.jpg` next to the media wins | Local Media Assets provider | External images take precedence over embedded ones (docs) | `folder`, `poster`, `cover` and others (docs) | Infuse, for the simplest filename rules, including a per-folder favourite image | |
| Save artwork next to media | Portable art that survives a reinstall | No (unverified) | Optional (unverified); request #2974 asks to choose where (4 votes) | Optional (unverified) | Jellyfin and Emby | |
| fanart.tv art | Logos, clear art and disc art | Not found in sources | fanart.tv plugin (docs) | Not found in sources | Kodi and TMM (unverified for Kodi) | |
| Badges on posters | 4K, HDR, audio and rating badges | Only through Kometa, which rewrites posters | Not found in sources | Cover Art plugin with over 30 styles, Emby Premiere | Emby, built in; Kometa for Plex | Kometa warns that old overlaid images are not cleaned up and that manual poster changes cause double overlays. |
| Per-season art | Different art per season | Season posters (unverified) | Season images (unverified) | Not found in sources | Not clear | Infuse request for per-season fanart has 76 likes. |

### Local metadata: NFO files and embedded tags

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Read NFO for movies and TV | Curated metadata from TMM or Kodi used as is | New in PMS 1.43.1 (preview posted 2026-02-06): Kodi format, movies and TV, can lead an agent | Yes; local NFO always beats remote providers (docs) | Yes (TMM docs say Emby reads NFO natively) | Kodi, which defined the format, and Jellyfin | Plex users waited years (request 467992, 231 votes, opened 2019). |
| Write NFO | Server edits saved as portable sidecars | No; Plex's NFO support is read-only | NFO saver per library (docs) | Yes (unverified) | Jellyfin and Emby; TMM for authoring | |
| NFO for music | `artist.nfo` and `album.nfo` | Not supported and not planned | Supported (docs) | Not found in sources | Jellyfin | |
| NFO for extras | Titles and descriptions for trailers and featurettes | Not supported | Not found in sources | Not found in sources | Nobody | |
| Lock flag in NFO | A sidecar can say "never overwrite me" | Not applicable | `lockdata` tag (docs) | Not found in sources | Jellyfin | |
| NFO dialects | Files from TMM, FileBot and Kodi all work | Kodi dialect; FileBot NFOs lack actor images and unique IDs (preview thread) | Kodi-compatible | Kodi-compatible | TMM writes Kodi, legacy XBMC, Jellyfin/Emby and MediaPortal dialects | |
| NFO limits | | Episode-spanning files need one NFO per episode; cast matched by name only | Only the first `thumb` per artwork type is used; user data import is single-user; NFO reading cannot be turned off globally (docs) | Not found in sources | Not applicable | |
| Embedded video titles | Use the title stored inside an MKV or MP4 | MP4 and M4V only, not MKV, per a Plex moderator (request 467992) | Not found in sources | Option to prefer embedded titles over file names (docs) | Emby | Infuse users ask for MKV titles over file names (9 likes). |
| Prefer embedded over online | Your tags beat the database | Not found in sources | Request #3349 (5 votes); #2720 marked completed | Not found in sources | Not clear | |
| Write tags into media files | The server edits the files themselves | No | Request #1685 (8 votes) | No (unverified) | Dedicated taggers (MusicBrainz Picard, beets) (unverified) | Gunmetal should keep media read-only. |
| NFO in a separate folder | Sidecars without cluttering media folders | Not applicable | Request #3827 (6 votes) | Not found in sources | Nobody | |
| Third-party player overrides | Override metadata without a server | Not applicable | Not applicable | Not applicable | Infuse reads NFO or XML next to movies; grouping TV episodes via local metadata is not yet supported and is listed for 8.6.x | |

### Music identification and tags

The video servers are not the only rivals here, so this table compares Plex,
Jellyfin and Navidrome.

| Feature | What the user gets | Plex | Jellyfin | Navidrome | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Tag-driven library | Albums built from tags, not folder layout | Mix of tags and Plex's online music data (unverified) | Embedded tags win, but one folder must hold one album (docs) | Entirely tag-based; folders ignored (docs) | Navidrome | Emby is also tag-based and made folders optional in 4.6 (docs). |
| Several artists per track | A collaboration appears under each artist | Not supported; request 116658 has 520 votes and dates from 2015 | Partial; request #3440 for collaborative albums (6 votes) | Multi-valued `ARTISTS` tag, with `ARTIST` as the display string; falls back to separators such as " / ", " feat. " and "; " (docs) | Navidrome | Emby uses a semicolon-space separator (docs). |
| Rich classical and credit tags | Composer, conductor, work and movement, original date, per-track genre | Request 106326 has 866 votes and dates from 2015 | Classical view planned (#772, 34 votes); album-level credits requested (#4060) | Not found in sources | Not clear | |
| Release types | Albums, singles, EPs, live and compilations shown apart | Grouped by release type (unverified) | Request #2220 (42 votes) | Not found in sources | Plex (unverified) | |
| Compilations | Various-artists albums stay together | Not found in sources | Not found in sources | `COMPILATION` flag plus "Various Artists" album artist (docs) | Navidrome | |
| Same-named albums | Two "Greatest Hits" by one artist stay separate | Not found in sources | Not found in sources | Not found in sources | Emby separates them with MusicBrainz album IDs (docs) | |
| MusicBrainz IDs | Exact identity from tags written by Picard | Not found in sources | MusicBrainz support improved in 10.9 (release notes); requests for MusicBrainz genres (#3776) and track data (#3731), 7 votes each | Reads them, but grouping relies on text tags (docs) | Not clear | |
| Acoustic fingerprinting | Untagged files identified by how they sound | No (unverified) | No | No | MusicBrainz Picard and beets via AcoustID (unverified) | AcoustID is free for non-commercial use only, at most 3 requests per second. |
| CUE sheets | Single-file albums split into tracks | Request 96352 (507 votes) | Request #1190 (171 votes) | Not found in sources | Nobody among the servers | |
| Album art order | Predictable choice between folder images, embedded art and online art | Not found in sources | Folder images beat embedded art (docs); request #1771 to prefer embedded art (29 votes) | `cover`, `folder`, `front` files, then embedded art, then Last.fm (docs) | Navidrome, for a documented order | |
| Per-disc art | Box sets show each disc's cover | Not found in sources | Not found in sources | `disc*` and `cd*` images per disc (docs) | Navidrome | |
| Artist images and bios | Artist pages that look finished | Artwork and bios are listed among Plex Pass features (Plex Pass page) | TheAudioDB and MusicBrainz (unverified) | Last.fm for external art (docs) | Plex | TheAudioDB's free key allows 30 requests a minute; its premium tier is $8 a month for 100. |
| Cover art by release or release group | The right cover for the exact pressing, or the canonical cover for the album | Not found in sources | Cover Art Archive (unverified) | Not used; Last.fm only (docs) | Cover Art Archive itself: front and back images per release or release group, with 250, 500 and 1200 pixel thumbnails and no rate limit at present | Needs MusicBrainz IDs, which ties cover quality to tag quality. |
| Lyrics files | Synced lyrics from `.lrc` sidecars or tags | Lyrics (unverified details) | `.lrc`, `.elrc`, `.txt` and embedded lyrics; provider plugins since 10.10; word-level timing in 10.11 (docs and notes) | Not found in sources | Jellyfin | |
| Lossless and lossy copies | One album with a FLAC and an Opus copy | Not found in sources | Request #3968 (1 vote) | Not found in sources | Nobody | Same problem as video versions. |
| Music videos linked to tracks | A song page offers its video | Not found in sources | Requests #3782 (9 votes) and #896 (38 votes) | Not found in sources | Nobody | |
| Sonic analysis | Similarity data for radios and mixes, computed at scan | Plexamp members-only features such as Sonic Sage and Guest DJ (Plex Pass page) | Request #3245 (98 votes) | Not found in sources | Plex | Player features live elsewhere; the analysis cost lands on the scan. |
| Moved files keep history | Reorganising files keeps play counts and ratings | Not found in sources | Not found in sources | Persistent IDs and a manual remap command (docs) | Navidrome | |

### Editions and multiple versions

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Versions in one folder | A 4K and a 1080p file appear as one title with a version picker | Yes (unverified naming rules) | Files sharing the folder-name prefix plus a label after a hyphen, dot, underscore or brackets; resolutions sort highest first (docs) | `FolderName - description.ext`, at most 8 versions (docs) | Jellyfin, for no documented cap and resolution-aware sorting | |
| Versions across folders | "Movies" and "Movies 4K" folders merge into one title | Merges matched items within a library (unverified) | Not supported; request #174 has 265 votes, open since July 2019 | Not found in sources | Not clear | |
| Named editions for movies | Director's Cut and Theatrical are distinct entries with their own art and history | `{edition-…}` in the file name; setting or editing editions needs Plex Pass (support article via search summary; "multiple cuts" listed on the Plex Pass page) | Not supported; request #2542 (102 votes) | Versions only, meant mainly for quality differences (docs) | Plex; Infuse also reads `{edition-…}` and recognises Director's Cut, Extended and IMAX on its own | The original Plex request "Multiple Cuts Of Movie" has 1,459 votes. Infuse cannot show Plex editions when browsing a Plex server (request 38976, 43 likes). |
| Editions for TV | Remastered or dubbed versions of a show | Show-level `{edition-…}` on the series folder; Plex Web 4.160 and mobile 2026.13.0; editions cover whole shows only; watch-state sync does not yet work for them (Plex support article) | Episode versions by naming (unverified) | Episode versions, at most 8 (docs) | Emby for per-episode versions; Plex for named show editions | Plex request for TV editions has 124 votes and is still open. |
| Version picker detail | See codec, HDR, audio and size before choosing | Request 312496 "More info in Play Version" (266 votes) | Not found in sources | Not found in sources | Not clear | Infuse users ask for wider version boxes (8 likes). |
| Automatic version choice | The best playable version is chosen per device | Not found in sources | Highest resolution sorts first (docs) | Not found in sources | Not clear | Gunmetal's playback engine can choose the version that direct-plays. |
| Separate history per edition | Watching the Extended cut does not mark the Theatrical cut | Separate for TV editions (support article) | Not found in sources | Not found in sources | Plex | |
| Manual merge and split | Group or ungroup versions by hand | Merge and "split apart" (unverified) | "Group versions" (unverified) | Not found in sources | Not clear | |
| 4K marker | See which titles exist in 4K | Not found in sources | Request #1738 (146 votes) | Not found in sources | Kometa overlays (Plex only) | |

### Extras and trailers

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Local extras for movies | Featurettes and deleted scenes appear under the movie | Folders and suffixes (unverified) | Named folders (`trailers`, `behind the scenes` and others), fixed names (`trailer.mp4`) or suffixes (`-trailer`, no spaces) (docs) | Named subfolders such as extras, featurettes, deleted scenes, interviews and trailers; one level deep only (docs) | Jellyfin, for three ways to declare an extra | Infuse accepts Plex, Jellyfin and Emby naming. |
| Extras for TV | Series, season and episode extras | Request 177179 (434 votes) was closed in 2018; whether that means implemented is unclear (unverified) | Supported (docs) | Series, season and episode levels; episode extras get no online metadata (docs) | Emby | |
| Online trailers | Trailers without downloading files | "Play trailers before movies" listed as a Plex Pass feature | Request #55 "Trailers Plugin" (585 votes), open | Not found in sources | Plex | An Infuse thread from December 2025 reports trailers unavailable (16 likes). |
| Download trailers | Save trailers next to the movie | No | No | No (unverified) | TMM, with trailer sources from TMDB and IMDb (docs) | |
| Pre-rolls and cinema mode | Trailers and intros before the feature | Plex Pass | Plugin (unverified) | "Cinema Intros", Emby Premiere | Plex and Emby, both paid | |
| Theme songs and backdrop videos | Ambient audio and video on detail pages | Theme music for TV (unverified) | `theme-music` and backdrop videos; random order in 10.11 (docs and notes) | Not found in sources | Jellyfin | Jellyfin request #1749 for a theme-song volume control has 103 votes. |
| Metadata for extras | Real titles and descriptions for extras | Not covered by the NFO agent | Not found in sources | None for episode extras (docs) | Nobody | |

### Collections

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Manual collections | Group titles by hand | Yes (unverified) | Yes (unverified) | Add several items at once and create collections on the fly (docs) | Tie | |
| Automatic franchise collections | "Back to the Future" grouped automatically | From Plex's data (unverified) | TMDB box sets (unverified) | From TMDB, with a minimum number of movies per collection (docs) | Emby, for the minimum-size setting | Kodi movie sets (unverified). |
| Smart collections | Rule-based groups that update themselves | Smart collections (unverified) | Request #593 (104 votes) | Not found in sources | Plex | |
| Nested collections | MCU inside Marvel; trilogies inside franchises | Request 354043 (514 votes), open since 2018 | Not found in sources | Not found in sources | Nobody | |
| Hide single-item collections | No one-movie "collections" cluttering the grid | Implemented (request 219565, 195 votes) | Not found in sources | Minimum count for automatic collections (docs) | Plex and Emby | |
| Collection sort order | Sort a collection by release date, title or custom order | Not found in sources | Request #1577 (148 votes) | Not found in sources | Not clear | |
| Shows and movies together | A franchise collection holding films and series | Not found in sources | Shows can be grouped in collections since 10.11 (release notes); request #2214 to tag movies as part of a show (104 votes) | Not found in sources | Jellyfin | |
| Collections from online lists | IMDb Top 250, Letterboxd or Trakt lists kept in sync | Through Kometa: TMDB, IMDb, Trakt, MDBList, Letterboxd, AniList and MyAnimeList | Plugins (unverified) | Not found in sources | Kometa, but it is an external script and supports Plex only | |
| Collection artwork | Custom collection posters | Yes (unverified) | Yes (unverified) | Edit images (docs) | Tie | Infuse users ask for custom collection art (12 likes). |

### Manual fixes and locking

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Edit fields | Change titles, dates, summaries and tags | Yes (unverified) | Yes (unverified) | Yes (unverified) | Tie | |
| Per-field locks | An edited field is never overwritten | Editing flags the field as locked; the poster field reportedly ignores it (request 755134) | Field locks exist (requests #3769 for a lock indicator and #1178 for a lock filter confirm it) | Not found in sources | Not clear | |
| Lock a whole item | Freeze an item entirely | Not found in sources | Yes, including `lockdata` in NFO (docs) | Not found in sources | Jellyfin | |
| Lock a whole library | Freeze a curated library | Not found in sources | Request #2998 (14 votes) | Not found in sources | Nobody | |
| Bulk edit | Change one field on many items at once | Multi-select editing (unverified) | Request #144 (186 votes); request #852 to edit as a text file (23 votes) | Metadata manager (unverified) | Plex (unverified) | |
| Undo and edit history | Revert a bad edit or a bad match | No (unverified) | No | No (unverified) | Nobody | Gap. |
| Edits survive a rebuild | Reinstalling or rebuilding the database keeps your fixes | Fixes live in Plex's database (unverified) | Only if NFO saving is enabled | Only if NFO saving is enabled (unverified) | Jellyfin and Emby, through NFO | Central for Gunmetal, whose database is a rebuildable cache. |
| Delegated editing | Trusted non-admin users fix art or metadata | Not found in sources | Request #3496 (7 votes) | Not found in sources | Nobody | |
| Backup and restore | Snapshot metadata and settings | Not found in sources | Built in since 10.11 (release notes) | "Backup and Restore", Emby Premiere | Jellyfin, because it is free | |

### Several libraries and servers

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| One title in two libraries | "Movies" and "4K Movies" do not show the same film twice | Separate items (unverified) | Separate items; see request #174 | Not found in sources | Nobody | |
| People across libraries | An actor page lists everything they appear in | Implemented (request 185579, 254 votes) | Not found in sources | Not found in sources | Plex | |
| Several servers in one client | Browse a friend's server next to your own | Yes (unverified) | Request #47 (177 votes) | Not found in sources | Plex | |
| Search by tag or genre across libraries | Find all "noir" titles everywhere | Not found in sources | Tag search added in 10.9; request #276 (256 votes) asks for more | Not found in sources | Not clear | |

### Library health

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Missing episodes | Gaps in a season shown as placeholders | Not supported; request 77834 (327 votes, open since 2014); a moderator pointed to a browser extension | Only with the TheTVDB provider, per a maintainer in 2023; request #572 (343 votes) is "started" | Not found in sources | TMM: missing episodes, with toggles for missing specials and unaired episodes (docs) | |
| Hide empty shows | Shows with no files on disk stay hidden | Not found in sources | Request #41 (190 votes, planned) | Not found in sources | Not clear | A side effect of placeholder items. |
| Duplicate finder | A page listing duplicates with codec and size, to choose what to delete | Duplicates filter (unverified) | Request #31 (135 votes) | Not found in sources | Not clear | |
| Unmatched items | A list of everything that failed to identify | Unmatched filter (unverified) | Request #665 (101 votes) | Not found in sources | Not clear | |
| Bad or unplayable files | A report of truncated, corrupt or unparseable files with the reason | No (unverified) | No | No (unverified) | Nobody | Gunmetal's parsers return typed errors with a location, which makes this nearly free. |
| Missing artwork or metadata | Filter for items without posters, plots or IDs | Not found in sources | Not found in sources | Not found in sources | TMM filters (unverified) | |
| Missing files list | See files that vanished, when, and from where | Trash view (unverified) | Not found in sources | Not found in sources | Navidrome: admin-only placeholders with path and time, CSV export (docs) | |
| Technical quality report | Which titles are SD, which lack HDR, which have lossy audio | Not found in sources | Not found in sources | Not found in sources | External tools (unverified) | Gunmetal already parses this at scan. |
| Orphaned sidecars | Subtitles, NFO or art left behind after a video was deleted | No | No | No | Nobody | Gap. |
| Naming report | Files whose names will mismatch or match poorly | No | No | No | Nobody | Gap. |

### Paid gates on library features

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Price of the paid tier | | Plex Pass about $7 a month or $70 a year; lifetime rose from $249.99 to $749.99 on 2026-07-01, plus a new 5-year pass at $249.99 (MacRumors, Plex blog) | Free | Premiere $4.99 a month, $54 a year or $119 lifetime | Jellyfin | TMM PRO costs €12 a year. |
| Library features behind the paywall | | Editions (setting and editing), trailers before movies, artwork and bios, Plexamp mix features | None | Cinema intros, Cover Art overlays, backup and restore | Jellyfin | TMM keeps TheTVDB, OMDb, Trakt and subtitle downloads in PRO; the free tier has TMDB only. |

## Pain points and unmet demand

Vote counts are as shown on the source on 2026-10-02.

1. **Wrong matches are hard to undo cleanly.** Plex users have a named
   ritual, the "Plex Dance", for forcing a re-match, and forum threads about
   it failing run from 2019 to 2025, including "What to do when even the Plex
   Dance doesn't work?" (topic 923883, June 2025) and a show that stopped
   scanning after doing it (topic 923659). Jellyfin's request for an
   unmatched-items filter has 101 votes (#665).
   <https://forums.plex.tv/search.json?q=%22plex%20dance%22>,
   <https://features.jellyfin.org/api/v1/posts?view=most-wanted&limit=100>

2. **Missing episodes are invisible.** Plex request 77834 has 327 votes and
   has been open since 2014. Jellyfin request #572 has 343 votes; a
   maintainer said in 2023 that it works only with the TheTVDB provider.
   <https://forums.plex.tv/t/option-to-show-missing-seasons-and-episodes/77834.json>,
   <https://features.jellyfin.org/api/v1/posts/572>

3. **Plex's music model is too thin.** Request 116658 for several artists per
   track has 520 votes and dates from 2015; request 106326 for composer,
   conductor, work and movement, per-track genre and similar tags has 866
   votes and 255 posts; CUE sheet support (96352) has 507 votes. Jellyfin has
   the same CUE request (#1190, 171 votes) and asks for release-type grouping
   (#2220, 42 votes).
   <https://forums.plex.tv/t/better-support-for-albums-and-tracks-with-multiple-artists/116658.json>,
   <https://forums.plex.tv/t/tag-support-for-robust-music-library-organization/106326.json>,
   <https://forums.plex.tv/c/feature-suggestions/8.json?order=votes>

4. **Versions and editions are half-solved everywhere.** Jellyfin cannot
   merge versions held in different folders (#174, 265 votes, open since
   2019) and has no named editions (#2542, 102 votes). Plex has editions but
   requires Plex Pass to set them, applies TV editions to whole shows only,
   and does not yet sync watch state for them; the TV request still has 124
   open votes. Infuse cannot show Plex editions (43 likes). Emby caps
   versions at 8 and describes them as being for quality differences.
   <https://features.jellyfin.org/api/v1/posts/174>,
   <https://forums.plex.tv/t/add-edition-support-to-tv-shows/843629.json>,
   <https://support.plex.tv/articles/multiple-editions-tv-shows/>,
   <https://community.firecore.com/t/plex-edition-field-support/38976.json>

5. **Curated artwork and edits do not stay put.** Plex request 755134 "Lock
   Posters" has 194 votes and no staff reply in four years; request 151538 to
   delete unwanted posters has 333 votes. An Infuse thread is titled
   "Metadata is getting (wrongly) overwritten" (10 likes). Jellyfin users ask
   to lock a whole library (#2998, 14 votes) and for a lock indicator
   (#3769).
   <https://forums.plex.tv/t/lock-posters/755134.json>,
   <https://community.firecore.com/c/metadata/14.json?order=likes>

6. **Bulk editing and provenance are missing.** Jellyfin's bulk-edit request
   has 186 votes (#144), an "edit as a text file" request has 23 (#852), and
   "make it clear where the metadata was pulled from" has 44 (#968).
   <https://features.jellyfin.org/api/v1/posts?query=metadata&limit=50>

7. **Scanning regressions and wasted work in Jellyfin 10.11.** An unchanged
   library re-saves about 3,980 items with 4,770 database updates per scan
   and floods Kodi clients with 1,500+ pending updates (#18274); the real-time
   monitor triggers full rescans (#16729); new shows stopped appearing for one
   user after the database migration (discussion 16326).
   <https://github.com/jellyfin/jellyfin/issues/18274>,
   <https://github.com/jellyfin/jellyfin/issues/16729>,
   <https://github.com/orgs/jellyfin/discussions/16326>

8. **Change detection does not work on network storage.** Plex forum experts
   explain that SMB and NFS cannot notify the server and inotify only sees
   local changes (topics 642609, 930098, 854832). Jellyfin documents the same
   limit for NFS and rclone.
   <https://forums.plex.tv/search.json?q=partial%20scan%20network%20share%20not%20detecting>,
   <https://jellyfin.org/docs/general/administration/troubleshooting/>

9. **Plex's extension model keeps breaking.** Plugins were retired years ago;
   a 2025 beta broke all third-party agents by accident, and users said they
   felt betrayed. Plex now plans to remove legacy agents entirely in 2026 and
   replace them with HTTP providers that do not yet support music,
   authentication or provider settings. Its NFO support arrived in 2026, is
   read-only, and excludes music and extras.
   <https://forums.plex.tv/t/legacy-agents-removed-already-in-pms-1-41-7-9717-2025-04-23/914518>,
   <https://forums.plex.tv/t/announcement-custom-metadata-providers/934384>,
   <https://forums.plex.tv/t/plex-nfo-agent-forum-preview/936104>,
   <https://forums.plex.tv/t/with-plugin-support-dead-can-plex-finally-have-native-nfo-support/467992.json>

10. **Multilingual households are poorly served.** Jellyfin request #610
    (212 votes) asks for metadata in several languages, and #32 (315 votes)
    for keeping original titles. Plex users create duplicate libraries to get
    a second language.
    <https://features.jellyfin.org/api/v1/posts?view=most-wanted&limit=100>,
    <https://forums.plex.tv/t/download-metadata-in-two-languages/425063>

11. **Episode ordering remains contentious.** Plex added TheTVDB alternate
    orders in PMS 1.40.4, but TMDB episode groups are still open (request
    537737, 502 votes). When Infuse moved TV to TMDB in 2021, international
    and anime users objected over language gaps and episode order (61
    posts).
    <https://forums.plex.tv/t/support-alternate-order-flexible-seasons-tvdb-and-episode-groups-tmdb/537737.json>,
    <https://community.firecore.com/t/switching-tv-show-metadata-providers-thoughts/14473.json>

12. **Trailers and extras.** Jellyfin's trailers request has 585 votes (#55).
    Plex's TV extras request had 434 votes before it was closed in 2018.
    <https://features.jellyfin.org/api/v1/posts?view=most-wanted&limit=100>,
    <https://forums.plex.tv/t/tv-series-dvd-special-features-extras/177179.json>

13. **Collections need structure and rules.** Plex nested collections has 514
    votes (open since 2018); Jellyfin smart collections has 104 (#593) and
    collection sort options 148 (#1577). Kometa fills the gap for Plex only,
    by rewriting posters, and warns that old overlaid images pile up.
    <https://forums.plex.tv/t/nested-collections/354043.json>,
    <https://kometa.wiki/en/latest/files/overlays/>

14. **Health tools are missing.** Jellyfin's duplicate finder request has 135
    votes (#31) and "hide shows with no files" 190 (#41). No server reports
    corrupt or unparseable files.
    <https://features.jellyfin.org/api/v1/posts/31>

15. **Clutter the user did not ask for.** Plex's request to turn off Discover
    and streaming results has 1,000 votes, and the request to turn off
    reviews 185.
    <https://forums.plex.tv/c/feature-suggestions/8.json?order=votes>,
    <https://forums.plex.tv/c/feature-suggestions/8.json?order=votes&page=2>

16. **Paywalls on library basics.** Plex gates edition editing and trailers
    behind a pass whose lifetime price tripled to $749.99 in July 2026; Emby
    gates cinema intros, poster overlays and backup; TMM gates TheTVDB.
    <https://www.macrumors.com/2026/05/19/lifetime-plex-pass-price-increase/>,
    <https://emby.media/premiere.html>,
    <https://www.tinymediamanager.org/purchase/>

17. **Folder browsing.** Jellyfin's folder-view request has 438 votes and 134
    comments (#224).

18. **Ratings.** Jellyfin request #463 for IMDb and Rotten Tomatoes ratings
    has 196 votes; Infuse's equivalent has 321 likes and is now in progress.
    <https://community.firecore.com/c/suggestions/19.json?order=votes>

## Where Gunmetal can be clearly better

Each idea names the pain point it answers and what would have to be true.

1. **A scan that does nothing when nothing changed, and says exactly what
   changed when something did.** (Pain points 7 and 8.) A rescan of an
   unchanged library should make zero database writes and emit zero sync
   events. This needs a per-file fingerprint (size, modification time, file
   ID or inode, and a short content hash), a per-directory short-circuit so
   unchanged folders are skipped without opening files, and a scanner that
   produces a diff rather than re-saving items. The same diff becomes the
   change feed that devices sync from (record 1). Network shares get a
   polling mode with configurable interval that walks directory metadata
   only. This is testable as a property ("scan twice, second scan writes
   nothing"), which suits the project's test-first rules and belongs in the
   scan benchmark against Jellyfin.

2. **File identity that survives moves, renames and database rebuilds.**
   (Pain points 1 and 5.) Watch history in record 1 already lives in an
   append-only log; it should be keyed by content identity, not path, so the
   Plex Dance is never needed. Navidrome shows the model works. Candidate
   identities per format: the MusicBrainz recording ID when tagged, the FLAC
   STREAMINFO MD5 of decoded audio, and the Matroska segment UID (both
   format facts unverified here; check the specifications), with a hash of
   the first and last few hundred kilobytes plus size as the fallback. The
   hard part is deciding when a re-encoded or remuxed file is "the same
   title, new version" rather than a new item; the answer should go through
   the versions model in idea 7.

3. **Manual fixes are irreplaceable data, so treat them like watch history.**
   (Pain point 5.) Record 1 says watch history is the only irreplaceable
   data, but match overrides, field locks, chosen artwork, custom collections
   and edits are just as irreplaceable. They should live in an append-only,
   exportable curation log keyed by content identity, and the SQLite cache
   should be rebuilt from files plus the two logs. This gives undo and full
   edit history for free, survives reinstalls, and makes "lock" mean lock for
   every field including posters. It needs a new architecture record that
   extends record 1. Optional NFO export lets users take their curation to
   Kodi or Jellyfin, but stays off by default so media can stay read-only.

4. **Local-first identification that explains itself.** (Pain points 1, 9
   and 14.) The server core identifies items with no network at all: the
   filename and folder grammar, ID hints in any of the common syntaxes
   (`{tmdb-…}`, `[tmdbid-…]`, `[tmdbid=…]`), NFO `uniqueid` elements, MKV and
   MP4 tags, and MusicBrainz IDs in audio tags. Every decision carries a
   confidence score and a human-readable reason; low-confidence results go
   to a review queue instead of being applied. A naming report tells users
   which files will match poorly and how to rename them. This fits record 2,
   which keeps online lookups out of the server, and gives a useful library
   even before any provider plugin is installed.

5. **Providers as sandboxed plugins with declared network grants.** (Pain
   points 9 and 11.) Record 2 already requires this. A provider plugin
   declares the hosts it may reach (for example only `api.themoviedb.org`),
   receives an item's local evidence, and returns candidates and fields.
   Agents are ordered lists of providers per library, as in Plex's new model,
   with per-field merge rules and provenance recorded for every field
   (answers Jellyfin #968). The host enforces each provider's terms: a TMDB
   cache that refreshes within six months, attribution, and the TheTVDB
   subscriber PIN when a user-supported key is used. Because Plex's new
   providers are plain HTTP services, an adapter that can call a Plex-style
   provider URL might let Gunmetal reuse community providers (open question:
   the provider API's terms and stability).

6. **A real music identity model.** (Pain point 3.) Record 2 calls for
   artists, release groups, albums and tracks. To beat Plex and match
   Navidrome, the core tag parsers must read multi-valued `ARTISTS` and
   `ALBUMARTISTS`, MusicBrainz artist, release-group, release and recording
   IDs, release type, compilation flag, composer, conductor, work and
   movement, original date and per-track genre, and must keep the display
   string separate from the credited artists. CUE sheets become virtual
   tracks, which needs sample-accurate seeking inside a FLAC file; the
   segment map built at scan time is the natural place for those offsets.
   Acoustic fingerprinting (AcoustID) belongs in an opt-in plugin because it
   needs decoding and its free tier is non-commercial.

7. **One model for works, editions and files, across folders and
   libraries.** (Pain point 4.) A title (work) has editions (Theatrical,
   Director's Cut; or Deluxe and Remaster for an album, which MusicBrainz
   already models as releases in a release group), and each edition has one
   or more files (versions). Files group by identity, not by folder, so a 4K
   folder and a 1080p folder merge. Editions get separate watch history; the
   work shows combined progress. The version picker shows codec, HDR, audio
   and size from the scan, at no extra cost. Most importantly, the playback
   decision engine can choose the version the current device can direct-play,
   which no rival does and which directly serves record 1's "play the
   original" goal. None of this sits behind a paywall.

8. **Health tools built into the server.** (Pain points 2 and 14.) Because
   the core parsers return typed errors that say what was wrong and where,
   Gunmetal can list truncated, corrupt and unparseable files with the
   reason, which no rival does. The same scan data powers duplicate,
   unmatched, missing-artwork, technical-quality, orphaned-sidecar and
   missing-file reports (with Navidrome-style retention and CSV export).
   Missing episodes and missing albums come from episode and discography
   lists cached by whichever provider plugin is active, not only TheTVDB, and
   placeholder items never leak into normal browsing (answers Jellyfin #41).

9. **Badges as data, not burned-in images.** (Pain point 13.) Kometa and
   Emby draw 4K, HDR and audio badges onto posters. Gunmetal already knows
   resolution, HDR format and audio layout from its parsers, so clients can
   render badges themselves from the synced library. Nothing is rewritten,
   nothing piles up, and users can switch badges off per device.

10. **Collections with structure.** (Pain point 13.) Nested collections,
    rule-based collections evaluated against the synced library, explicit
    sort orders, mixed movie and TV collections, and a minimum size for
    automatic franchise collections. List-driven collections (IMDb, Trakt,
    Letterboxd) belong in plugins with network grants, because they reach
    third-party services.

11. **Several metadata languages per item.** (Pain point 10.) Store
    localised fields keyed by language, plus the original title, and let
    each user or device choose. The cost is extra provider calls per
    language at match time; TMDB exposes translations, but whether one call
    can return several languages needs checking (open question).

12. **No clutter by default.** (Pain point 15.) No online catalogue mixed
    into the library and no reviews unless a plugin adds them. This follows
    from record 1 (no central account) at no cost.

13. **Nothing in this domain behind a paywall.** (Pain point 16.) Editions,
    trailers from local files, lyrics, backup and health tools are part of
    the AGPL server. This is positioning rather than engineering, but it is a
    clear contrast with Plex Pass, Emby Premiere and TMM PRO.

## Risks and hard parts

- **Provider terms limit what can be shipped.** TMDB's free licence forbids
  commercial use, requires attribution and forbids caching data for more
  than six months, which collides with offline devices that hold a synced
  library and with any future paid hosting. TheTVDB requires every API key
  to have either a commercial licence or end-user subscriptions with a
  subscriber PIN. MusicBrainz allows about one request per second per IP,
  so a first identification of a large untagged music library takes hours
  (5,000 album lookups need at least 83 minutes). AcoustID's free tier is
  non-commercial. Each of these must be handled in the plugin host, not left
  to plugin authors.
- **Record 2 puts the plugin host on the critical path for video.** Music can
  ship tag-only like Navidrome, but nobody will accept a movie library
  without posters. The sandboxed provider plugin system, with network grants,
  has to exist before the video release.
- **Filename parsing is an endless long tail.** Release-group naming, anime,
  daily shows, specials and foreign titles need a large test corpus. Any
  corpus borrowed from other projects needs a licence check.
- **TV ordering is genuinely ambiguous.** Aired, DVD, absolute and provider
  episode groups disagree, and anime is the worst case. Expect a permanent
  stream of edge cases.
- **Untrusted sidecars and images.** NFO files are XML from unknown sources
  and must be parsed with no DTD or entity expansion. Downloaded and local
  artwork must be decoded with size and pixel limits (decompression bombs),
  ideally in memory-safe code or the sandbox, and stored as resized
  derivatives for device sync.
- **Scan cost on slow storage.** Building the segment map means reading cues,
  which in Matroska often sit at the end of the file; on a NAS that means
  seeks per file. Content hashing adds reads. Both must happen once per
  file and be skipped for unchanged files.
- **Loudness needs decoding.** Record 2 reads loudness at scan time. Reading
  existing ReplayGain or R128 tags is cheap, but computing loudness for
  untagged files means decoding every track, which on low-end hardware is
  many hours for a large library. It must be background, resumable and
  low-priority.
- **Identity edge cases.** Remuxes, re-encodes, tag edits (which change file
  bytes but not audio) and duplicated files all stress the identity rules,
  and the wrong rule silently merges or splits watch history.
- **Rebuildable cache versus curation.** If curation does not get its own
  durable log (idea 3), a database rebuild loses every manual fix, which is
  exactly the experience users complain about.
- **Compatibility adapters.** The Jellyfin and OpenSubsonic adapters expect
  their own ID schemes and field shapes (provider ID maps, Subsonic artist
  and album IDs), and existing apps assume behaviours such as one artist per
  album; the music model must map down to them without losing data.
- **Migration.** Users arriving from Plex or Jellyfin will want their matches,
  locks, collections and watch history. Reading NFO helps; importing from a
  rival's database is fragile and version-dependent.

## Open questions

1. Does Gunmetal ship a project TMDB key (allowed for non-commercial use,
   but who is the licensee?) or ask every user to bring their own?
2. Is TheTVDB worth a negotiated open-source licence, or should Gunmetal be
   TMDB-first as Infuse chose in 2021, accepting weaker non-English and
   anime coverage?
3. Should the server ever write into media folders (NFO, artwork, tags)? The
   security case says no by default; portability says offer it as opt-in.
4. Which content identity does each format use, and when is a changed file a
   new version rather than the same file?
5. Should the curation log be a separate record from the watch-history log,
   or one log with typed entries? This needs a new architecture record.
6. Is Plex's custom metadata provider API documented and licensed in a way
   that would let Gunmetal call the same providers?
7. How much music identification belongs in v1: tags only (Navidrome's
   approach), or MusicBrainz lookups through a plugin from day one?
8. Do editions keep fully separate watch history, as Plex TV editions do, or
   share progress with the parent work?
9. How are missing-episode and discography data kept fresh offline, given
   TMDB's six-month cache limit?
10. Which metadata languages are stored by default, and how many extra
    provider calls per item is acceptable?
11. Does the client render badges, collections and smart rules entirely from
    the synced library, or do some need server queries?
12. Which list services (Trakt, Letterboxd, MDBList) deserve first-party
    plugins?

## Sources

Gunmetal records:

- docs/adr/0001-architecture.md
- docs/adr/0002-music-is-first-class.md

Plex:

- https://forums.plex.tv/t/announcement-custom-metadata-providers/934384
- https://www.howtogeek.com/plex-is-overhauling-custom-metadata-providers/
- https://forums.plex.tv/t/plex-nfo-agent-forum-preview/936104
- https://support.plex.tv/articles/using-nfo-metadata-files-with-plex/ (direct fetch refused; facts taken from a search-engine summary)
- https://support.plex.tv/articles/multiple-editions/ (direct fetch refused; facts taken from a search-engine summary)
- https://support.plex.tv/articles/multiple-editions-tv-shows/
- https://forums.plex.tv/t/legacy-agents-removed-already-in-pms-1-41-7-9717-2025-04-23/914518
- https://forums.plex.tv/t/with-plugin-support-dead-can-plex-finally-have-native-nfo-support/467992.json
- https://forums.plex.tv/c/feature-suggestions/8.json?order=votes
- https://forums.plex.tv/c/feature-suggestions/8.json?order=votes&page=1
- https://forums.plex.tv/c/feature-suggestions/8.json?order=votes&page=2
- https://forums.plex.tv/c/feature-suggestions/8/l/top.json?period=all
- https://forums.plex.tv/t/add-edition-support-to-tv-shows/843629.json
- https://forums.plex.tv/t/better-support-for-albums-and-tracks-with-multiple-artists/116658.json
- https://forums.plex.tv/t/tag-support-for-robust-music-library-organization/106326.json
- https://forums.plex.tv/t/option-to-show-missing-seasons-and-episodes/77834.json
- https://forums.plex.tv/t/external-provider-ids-for-native-plex-agents/619090.json
- https://forums.plex.tv/t/lock-posters/755134.json
- https://forums.plex.tv/t/support-alternate-order-flexible-seasons-tvdb-and-episode-groups-tmdb/537737.json
- https://forums.plex.tv/t/tv-series-dvd-special-features-extras/177179.json
- https://forums.plex.tv/t/nested-collections/354043.json
- https://forums.plex.tv/search.json?q=%22plex%20dance%22
- https://forums.plex.tv/search.json?q=partial%20scan%20network%20share%20not%20detecting
- https://forums.plex.tv/t/download-metadata-in-two-languages/425063 (search result summary)
- https://www.plex.tv/plex-pass/
- https://www.macrumors.com/2026/05/19/lifetime-plex-pass-price-increase/ (search result summary)
- https://www.plex.tv/blog/new-lifetime-plex-pass-pricing/ (search result summary)

Jellyfin:

- https://jellyfin.org/posts/jellyfin-release-10.11.0/
- https://jellyfin.org/posts/jellyfin-release-10.10.0/
- https://jellyfin.org/posts/jellyfin-release-10.9.0/
- https://jellyfin.org/docs/general/server/media/movies/
- https://jellyfin.org/docs/general/server/media/shows/
- https://jellyfin.org/docs/general/server/media/music/
- https://jellyfin.org/docs/general/server/metadata/
- https://jellyfin.org/docs/general/server/metadata/nfo/
- https://jellyfin.org/docs/general/administration/troubleshooting/
- https://features.jellyfin.org/api/v1/posts?view=most-wanted&limit=100
- https://features.jellyfin.org/api/v1/posts?query=metadata&limit=50
- https://features.jellyfin.org/api/v1/posts?query=music&limit=50
- https://features.jellyfin.org/api/v1/posts/174
- https://features.jellyfin.org/api/v1/posts/572
- https://features.jellyfin.org/api/v1/posts/31
- https://github.com/jellyfin/jellyfin/issues/18274
- https://github.com/jellyfin/jellyfin/issues/16729
- https://github.com/orgs/jellyfin/discussions/16326

Emby:

- https://emby.media/support/articles/Movie-Naming.html
- https://emby.media/support/articles/TV-Naming.html
- https://emby.media/support/articles/Music-Naming.html
- https://emby.media/support/articles/Library-Setup.html
- https://emby.media/support/articles/Collections.html
- https://emby.media/premiere.html

Kodi:

- https://github.com/xbmc/xbmc/releases/tag/21.0-Omega (the Kodi wiki refused automated fetching, so most Kodi details above are marked unverified)

Infuse:

- https://support.firecore.com/hc/en-us/articles/215090947-Metadata-101
- https://support.firecore.com/hc/en-us/articles/4405042929559-Overriding-Artwork-and-Metadata
- https://support.firecore.com/hc/en-us/articles/4405035033495-Fixing-Mismatched-Titles
- https://community.firecore.com/t/the-new-editions-feature/41441.json
- https://community.firecore.com/t/plex-edition-field-support/38976.json
- https://community.firecore.com/t/switching-tv-show-metadata-providers-thoughts/14473.json
- https://community.firecore.com/t/upcoming-features-updated-10-2-26/12345.json
- https://community.firecore.com/c/suggestions/19.json?order=votes
- https://community.firecore.com/c/metadata/14.json?order=likes

tinyMediaManager and Kometa:

- https://www.tinymediamanager.org/
- https://www.tinymediamanager.org/purchase/
- https://www.tinymediamanager.org/docs/
- https://www.tinymediamanager.org/docs/movies/nfo-formats
- https://www.tinymediamanager.org/docs/tvshows/settings
- https://kometa.wiki/en/latest/
- https://kometa.wiki/en/latest/files/overlays/
- https://github.com/Kometa-Team/Kometa

Navidrome:

- https://www.navidrome.org/docs/usage/library/
- https://www.navidrome.org/docs/usage/library/tagging/
- https://www.navidrome.org/docs/usage/library/missing-files/
- https://www.navidrome.org/docs/usage/library/artwork/

Providers:

- https://developer.themoviedb.org/docs/rate-limiting
- https://www.themoviedb.org/api-terms-of-use
- https://www.thetvdb.com/api-information
- https://support.thetvdb.com/kb/faq.php?id=62
- https://musicbrainz.org/doc/MusicBrainz_API/Rate_Limiting
- https://musicbrainz.org/doc/MusicBrainz_Database/Download
- https://musicbrainz.org/doc/Live_Data_Feed
- https://musicbrainz.org/doc/Cover_Art_Archive/API
- https://www.theaudiodb.com/free_music_api
- https://acoustid.org/webservice

fanart.tv's own pages (fanart.tv/get-an-api-key and its API documentation)
refused automated fetching, so no fanart.tv terms are stated in this
document beyond its use as a Jellyfin plugin.
