# Music

This map covers music from the file on disk to the person listening: the
library model (credited artists, release groups, discs, works), tag reading
and scanning, the pages people browse, the player and its audio path, the
queue, playlists and smart playlists, lyrics, radio and mixes, loves,
ratings and listening history, scrobbling, handing playback between devices,
offline downloads, and the phone, car and TV surfaces that carry the player.
Three facts from the research set the bar. Plexamp is the best self-hosted
listening experience, but lyrics, downloads, track radio, the equaliser and
sonic features need Plex Pass, whose lifetime price rose to US$749.99 on
1 July 2026. Navidrome models owned music best (multiple artists, release
types, editions, word-synced lyrics) but relies on other people's clients and
its web player will not do gapless. Jellyfin's most-voted music request,
gapless playback, has been open since 2019 with 647 votes. Gunmetal's first
release has to match Plexamp's free tier wherever a web player can, model
music at least as well as Navidrome, keep everything a person authors safe
across a cache rebuild, and charge for nothing. Where a rival is already
better, the tables say so.

## Features

Conventions used in the tables:

- **Releases.** Releases (R1, R2, R3, Later, No), the Demand scale, row ownership and the
  terms "the user log" and "the identity store" are defined once in the
  [feature map README](README.md). A row whose Release cell would differ
  between maps names one owning row; the other maps point at it. In short: R1
  ships the server and the web client only; R2 adds Android phones and
  tablets, Android TV and Google TV, and the desktop shell, plus iroh remote
  access, the plugin host and the OpenSubsonic adapter. Apple builds are
  Later ("R2 if the App Store licence decision allows"). In R1, remote
  listening means the owner's own reverse proxy or VPN.
- **Rivals.** "Plex" means Plex Media Server with Plexamp. "Jellyfin"
  includes Finamp where it matters. "Navidrome" includes the OpenSubsonic
  clients it relies on. "Pass" means Plex Pass is required. Spotify, Apple
  Music and Roon appear where they set the bar. Vote and reaction counts come
  from the research files and were read on 2026-10-02. "Not covered by the
  research" means no evidence was gathered either way.
- **Architecture terms.** "The core" is the shared Rust crate. "The synced
  library" is the copy of library metadata kept on each device. "The user
  log" is the append-only, exportable store that ADR 1 creates for history;
  this map assumes it is extended to playlists, loves, ratings and
  corrections (open decision 1).
- **Ownership.** Rows that another map owns are listed here only where music
  needs something specific, and the owning row is named (ACC- rows are in
  `accounts.md`).

### Library model

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| MUS-001 | Every credited artist linked | A collaboration appears under each credited artist, and each has their own page | Plex no (request open since 2015); Jellyfin partial (10.11 keeps a random subset of artist tags, #15283); Navidrome yes | High: Plex 520 votes for multiple artists and 866 for robust tags; Jellyfin regressions #15283 and #14622 | R1 | Credits are ordered rows of artist, role and join phrase, read by the core from true multi-value tags, never a single artist text column | Credits table; multi-value tag reader; credit rows in the sync feed | Track rows; album header; artist page; now-playing credit line |
| MUS-002 | Display credit kept as tagged | "A feat. B" reads exactly as written, and both names are links | Plex no; Jellyfin unverified; Navidrome yes | Medium: comes with the multi-artist requests | R1 | Parity with Navidrome: the display string is stored beside the linked credits | Display string per track and album; join phrases | Track rows; player; album header |
| MUS-003 | Album artist separate from track artist | An album stays under one artist even when tracks have guests | Plex yes; Jellyfin yes; Navidrome yes | High: basic expectation; Emby splits albums whose tags disagree | R1 | Parity | Album-artist credit; album grouping key | Album page; artist page |
| MUS-004 | "Appears on" | An artist page lists albums where the artist only guests | Plex yes in Plexamp; Jellyfin unverified; Navidrome yes (unverified) | Medium: Plex users call it the nearest thing to multi-artist support | R1 | Parity; it falls out of the credits table | Query over credits where the artist is not an album artist | Artist page section |
| MUS-005 | Roles: composer, conductor, lyricist, producer, remixer, performer | Browse everything one person composed, conducted or produced | Plex partial ("By Composer" open since 2013); Jellyfin unverified; Navidrome composer since 2025, other roles unverified | Medium: Plex 89 and 81 votes; film-score fans have to drop all but the first composer | R1 | Role is a column on the same credits table, so every role is browsable with no schema change | Role tags read at scan; role index | Artist page role tabs; credits panel; role filter |
| MUS-006 | Same-name artists kept apart | Two different bands with one name stay separate | Plex weak (2026 Plexamp forum complaint); Jellyfin unverified; Navidrome by MusicBrainz ID when tagged (unverified) | Medium: recurring Plexamp complaint | R1 | A MusicBrainz artist ID is the identity when tagged; otherwise the name within its album-artist context. Collisions are listed in the health report | Artist identity resolver; collision report | Artist page disambiguation line; library health report |
| MUS-007 | Merge, split and alias artists | Fix "Beatles" versus "The Beatles" without retagging | Plex unverified; Jellyfin no (unverified); Navidrome requested (#2138, 9 reactions) | Low: small requests on Navidrome and Emby | R1 | Corrections are user-log events keyed by identity, so a cache rebuild replays them instead of losing them | Override events applied after every scan | Artist page edit menu; admin corrections list |
| MUS-008 | Release groups and editions | The deluxe, the remaster and the original appear as versions of one album | Plex weak (request open since 2015); Jellyfin no; Navidrome yes (2025) | Medium: Plex 93 votes | R1 | A grouping-key order (release-group ID, then release ID, then album artist plus title with edition words stripped) that errs towards not merging, with a default edition standing for the group | Release-group and release tables; grouping rule in the core | Album page "Other versions" row; artist discography |
| MUS-009 | Pick a preferred edition | Choose which edition represents an album in your library | Not covered for Plex, Jellyfin or Navidrome; Roon lets you pick (unverified) | Low | Later | A per-user pointer in the user log | Preferred-edition event | "Other versions" menu |
| MUS-010 | Release types | Artist pages split into albums, EPs, singles, live, compilations, soundtracks and more | Plex yes (manual edit requested); Jellyfin no; Navidrome yes | Medium: Plex 72 votes; Jellyfin 42 | R1 | RELEASETYPE and MusicBrainz primary and secondary types read at scan; a manual override is a user-log event | Release-type fields; override events | Artist page sections; type chips |
| MUS-011 | Compilations and Various Artists | A compilation stays one album and each contributor can still find their track | Plex partial; Jellyfin partial; Navidrome yes | Medium: Plex "Brahms from a Various Artists compilation" has 81 votes | R1 | The compilation flag (TCMP, cpil, COMPILATION) plus per-track credits | Compilation flag; per-track credits | Album page; contributor's "Appears on" |
| MUS-012 | Multi-disc albums with disc titles | Discs stay in one album, disc names show, and one disc can be played alone | Plex partial (non-numeric disc labels requested); Jellyfin bug (#5605, open since 2021, 71 comments); Navidrome yes | High: the Jellyfin bug and Hacker News complaints about albums split by folder | R1 | Albums are built from tags, never folders; disc number and disc subtitle drive grouping | Disc fields | Album page disc headers; "Play disc" action |
| MUS-013 | Original date versus release date | A 2011 remaster still sorts under 1969 | Plex, Jellyfin and Navidrome partial or supported (all unverified) | Medium: Spotify's sort-by-release-year idea has 1,738 votes | R1 | Both dates kept; sorting uses the original date by default and editions show their own | Original and release date fields | Album details; sort menus |
| MUS-014 | Works and movements on album pages | Movements group under their work heading | Plex no; Jellyfin planned (34 votes); Navidrome unverified | Medium: Plex 89 and 81 votes; Jellyfin 34 | R2 | WORK and movement tags (MVNM, ©mvn) read by the core; grouping only, no new pages Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Work and movement fields | Album page work headers |
| MUS-015 | Work pages with every performance | Compare different recordings of one work across albums | Plex no; Jellyfin no; Navidrome unverified; Lyrion 9.0 and Roon lead | Medium: the classical requests above | Later | A work identity keyed by MusicBrainz work ID, or composer plus normalised title, on the same model | Work table; identity rule | Work page; composer page |
| MUS-016 | Grouping versus work tag setting | Grouping and Work do not swap after an iTunes-style retag | Rivals unverified; Lyrion 9.1 added this option | Low | R2 | A per-library setting the core applies when reading TIT1 and GRP1 | Library setting; rescan of affected files | Library settings |
| MUS-017 | Multi-valued genres per track | A track can be both Jazz and Soul | Plex mostly per album, from its own metadata; Jellyfin multi-valued (per-track unverified); Navidrome multi-valued | Medium: genre hierarchy requests on Jellyfin and Emby | R1 | Per-track genres from tags, rolled up to the album | Genre table and links | Genre browse; filters; rule field |
| MUS-018 | Genre aliases and merges | See DIS-108, which owns this feature. | Not covered by the research; owned-file genre tags are messy | Medium | R2 | See DIS-108. | None beyond DIS-108. | Genre page edit menu; admin genre list |
| MUS-019 | Moods, styles, labels and grouping | More ways into a large library | Plex yes (moods, styles, record labels); Jellyfin unverified; Navidrome label filter, grouping requested (#2045) | Low | R1 | From tags only. Plex fills these from its own metadata service, which Gunmetal does not have, so Plex is richer here for untagged libraries | Mood, label and grouping fields | Browse by label or mood; filters |
| MUS-020 | Sort names and natural sort | "The Beatles" files under B; "Part 2" comes before "Part 10" | Plex yes; Jellyfin partial; Navidrome yes (natural sort in 0.64) | Medium: Jellyfin sort-albums-by-year request has 49 votes | R1 | Parity; sort keys are computed once in the core, so the server and every client sort the same way | Sort-name tags; collation keys | Every list; alphabet jump |
| MUS-021 | Technical details | See and filter by codec, sample rate, bit depth, bitrate and channels | Plex badges, filtering requested; Jellyfin requested (17 votes each for codec and bitrate); Navidrome requested (#1438) | Medium | R1 | Read from stream headers by the core during the scan, at no extra cost | Technical fields per file | Track info sheet; album badge; filters |
| MUS-022 | BPM and key from tags | Sort or filter by tempo | Plex requested (22 votes); Jellyfin no; Navidrome stores BPM | Low | R2 | Read from tags; computing tempo belongs to sonic analysis (MUS-172) Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | BPM and key fields | Track info sheet; rule fields |
| MUS-023 | Personal tags | Labels such as "wedding" or "gym" on any track or album | Plex requested (part of the 866-vote tag request); Jellyfin and Navidrome unverified; Spotify missing | Medium: Spotify "Tag Music" had 1,888 votes | R2 | Personal tags are user-log events, usable in smart playlists and home sections A personal tag belongs to one profile and lives in the user log; a household-wide label is LIB-180. Parental rules use labels, never personal tags. | Tag events per user | Context menu "Tag"; tag browse; rule editor |
| MUS-024 | Artist images from local files | Artist pages show a photo kept in the music folder | Jellyfin documents local image file names for music; Navidrome yes; Plex not covered by the research | Medium: bare artist pages are the main cost of having no lookups | R1 | Read at scan with the same documented order as album art; nothing leaves the server | Artist image discovery | Artist page header |
| MUS-232 | Album booklet and liner notes from local files | The PDF or image booklet that came with a purchased album, on the album page | Roon is best at reviews and liner notes (research); others not covered | Low: no vote data | Later | Read-only display of PDF and image files found in the album folder; critic reviews stay declined (LIB-125). Depends on LAT-160 for PDF viewing | Booklet files indexed during scan | Album page |
| MUS-025 | Artist photos, biographies and custom upload | Artist pages look finished | Plex yes (its own metadata); Jellyfin yes, but a provider once put a keyboardist's photo on every Ogg file (#9406, 204 reactions); Navidrome external agents and upload (0.61) | Medium: Plex and Roon set the bar | R2 | Lookups run in a sandboxed plugin with a network grant (ADR 2), and tags always win over provider data Behind Plex, Jellyfin and Navidrome in R1: artist pages show only local images (artist.jpg in the folder, embedded art) until the plugin host arrives in R2. | Plugin host; provider cache; upload store | Artist page; admin artwork picker |
| MUS-026 | Missing albums | See the studio albums by an artist that you do not own | Plex unverified; Jellyfin no; Navidrome requested (#5106, 12 reactions) | Low | Later | Discography from a MusicBrainz plugin, kept out of normal browsing | Plugin discography cache | Artist page "Not in your library" |
| MUS-027 | Several music libraries with per-user access | A children's library, or a partner's collection kept apart | Plex yes; Jellyfin yes, though its artist view leaked albums across libraries (#17215); Navidrome yes since 2025 | Medium: Navidrome 106 reactions | R1 | Access is applied when the sync feed is built, so a hidden album never reaches the device at all (rules owned by the accounts map) | Per-library grants; filtered sync | Admin library access; library switcher |
| MUS-028 | Folder view | See LIB-008, which owns this feature. | Plex unverified; Jellyfin yes; Navidrome through Subsonic folder endpoints | Low: Finamp request #661; Jellyfin's 438-vote folder view request covers all media | R1 | See LIB-008. | None beyond LIB-008. | Library "Folders" view |
| MUS-029 | Music videos and artist extras | A song offers its video; live sets and interviews sit on the artist page | Plex yes, by file naming; Jellyfin keeps them in a separate library; Navidrome no | Low: Jellyfin 38, 15 and 9 votes | R2 | Typed relations exist in the R1 schema; Plex's file-name suffixes are accepted for easy migration; playback uses the R2 video path | Item relations; extras scan | Track menu "Watch video"; artist extras row |
| MUS-030 | Duplicate finder | Find the same recording ripped twice | Plex unverified; Jellyfin requested (15 votes); Navidrome discussion | Low | Later | Music-specific extension of LIB-196: match on audio features from the analysis pass (MUS-172, Later). | Duplicate report job | Library health report |
| MUS-031 | Lossless and lossy copies grouped | A FLAC and an Opus copy of one album count as one album | Jellyfin request (1 vote); others not covered | Low | Later | The versions model shared with video; the playback decision picks the copy the device plays best | Version grouping | Album page version picker |

### Tags, scanning and files

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| MUS-032 | Core formats | FLAC, MP3, AAC and ALAC in MP4, Ogg Vorbis, Opus and PCM WAV are scanned, and play where the browser can decode them (AIFF too where the browser probe allows it) | Plex, Jellyfin and Navidrome broad, transcoding what a client cannot play | High: the README's first core milestone | R1 | Pure-Rust parsers with typed errors and no FFmpeg at scan time. Which of these a browser can decode varies by browser (unverified per browser) | Parsers in the core; byte serving | Track info sheet |
| MUS-229 | Honest unplayable state | A track the browser cannot decode is marked before play, skipped in the queue with a reason, and listed in the health report by format and browser | Not covered by the research for any rival | High: likely a common first-week complaint from Apple Music (ALAC) libraries (unverified) | R1 | R1 has no transcoder, so this replaces a silent failure with a stated reason and a count the owner can act on; ALAC playback in Chrome and Firefox is unverified | Codec and container per file in the sync feed; a browser capability probe in the client | Track rows (dimmed with a reason); queue; Library health |
| MUS-033 | More formats | WavPack and Monkey's Audio (WAV moves to MUS-032; AIFF too where browsers decode it) | Same as above; foobar2000 is the reference | Low | R2 | Parsers added to the core; anything a client cannot decode goes through the sandboxed transcoder | Parsers; transcode profile | Track info sheet |
| MUS-034 | Multi-value tags in every tag format | Multiple artists and genres survive whatever tagger was used | Plex weak; Jellyfin partial; Navidrome yes, and documents which formats can hold them | High: underlies MUS-001 | R1 | One reader in the core for ID3v2.3 separators, ID3v2.4 null separators, repeated Vorbis fields, MP4 freeform atoms and APE, held to zero surviving mutants | Tag reader | Track info sheet shows how each field was read |
| MUS-035 | Separator splitting with exceptions | "AC/DC" and "Tyler, The Creator" are not split into two artists | Plex no; Jellyfin regression (#14622); Navidrome yes (0.63) | Medium | R1 | A deterministic splitter with an exception list, plus a scan report of every ambiguous split for review | Exception list; split report | Admin scan report; library settings |
| MUS-036 | MusicBrainz IDs as identity | Retagged files keep their plays, loves and playlist places; same-name items stay apart | Plex partial (unverified); Jellyfin reads them; Navidrome uses them for grouping (unverified) | Medium | R1 | Recording, release, release-group and artist IDs are read at scan and used before any name heuristic | ID fields; identity index | Track info sheet |
| MUS-037 | Tags win over online data | Careful tagging is never silently replaced | Plex unverified; Jellyfin fetched from providers even when they were disabled (#5778); Navidrome tags first | Medium: Jellyfin #5778 (18 reactions) | R1 | The core makes no network calls at all; providers can only fill gaps, through plugins, and every field records where it came from | Field provenance | Track info sheet shows each field's source |
| MUS-038 | Identity that survives moves and renames | See LIB-028, which owns this feature. | Plex users fall back on the "Plex Dance"; Jellyfin lost state on replace or rename in 10.11 (#15001); Navidrome persistent IDs | High: Jellyfin #15001 (61 reactions) | R1 | See LIB-028. | None beyond LIB-028. | Library health report "Moved files" |
| MUS-039 | Album art from file and folder | Covers from embedded art or cover and folder images, per disc | Plex yes; Jellyfin prefers folder images (29-vote request to prefer embedded); Navidrome documented order, per-disc art | Medium: Jellyfin 29 and 14 votes | R1 | A documented order that can be flipped per library; dominant colours and a blurred placeholder are computed at scan and synced | Artwork resolver; palette | Album grid; album page; player background |
| MUS-040 | Artwork sized for sync | Phones and TVs get small, fast covers; full size on demand | Navidrome blurred placeholders (0.64); others not covered | Medium: library sync must fit device budgets | R1 | Thumbnails made once at scan by memory-safe decoders with pixel limits (whether this runs in the sandbox is for the security map to settle) | Thumbnail job; per-device artwork budget | Every artwork tile |
| MUS-041 | CUE sheets and single-file albums | A one-file rip with a .cue, or a FLAC with an embedded cue sheet, plays as separate tracks | Plex no; Jellyfin no; Navidrome not planned (#2136) | High: Plex 507 votes; Jellyfin 171 | R2 | Owns CUE sheets; LIB-071 points here. Cues become virtual tracks with sample-accurate bounds. A browser cannot play a sample-accurate sub-range of a FLAC file from a byte range alone, so the server serves each virtual track as a re-headed FLAC slice cut at frame boundaries, with trim values for sample-accurate edges, using the FLAC frame index (the music form of the segment map). Gapless across tracks comes free. Depends on the audio packaging row (MUS-230). | Cue parser; FLAC frame index; slice serving | Album page (looks like any album); track info shows the source file |
| MUS-042 | Fast, incremental scans | See LIB-016, which owns this feature. Music specifics: tag-only reads in Rust with no process per file, measured by the benchmark harness. | Plex yes; Jellyfin 10.11 regressions (#15685, 89 reactions; "database locked" #15101, 229 comments); Navidrome fast | High: scan time is the first benchmark in ADR 1 | R1 | See LIB-016. | None beyond LIB-016. | Scan progress indicator |
| MUS-043 | Usable during the first scan | See LIB-021, which owns this feature. | Navidrome publishes expected scan times by library size; others unverified | Medium | R1 | See LIB-021. | None beyond LIB-021. | Library fills in live; progress banner |
| MUS-044 | Library health report | A list of corrupt, unreadable or oddly tagged files, each with the reason | Navidrome `inspect` and `doctor` commands; others not covered | Medium: Jellyfin #9406; a Hacker News user had a corrupt file played at painful volume | R1 | The core's typed errors say what was wrong and where, so damage is flagged before anyone presses play | Report table; CSV export | Admin library health |
| MUS-045 | Ratings imported from tags | Years of stars from another player carry over | Plex no; others unverified | Low: Plex 41 votes | R2 | Read once at scan into the user log as "imported" events, with the inconsistent POPM scales normalised | Rating mapping; import events | First-run import summary |
| MUS-046 | Fix metadata in the app | See LIB-172, which owns this feature. | Plex requested for Plexamp; Jellyfin saves edits to its database (unverified); Navidrome no (unverified) | Low: Plex 17 votes | R2 | See LIB-172. | None beyond LIB-172. | Edit sheet on track and album |
| MUS-047 | Explicit flag | Explicit tracks are marked and can be kept from children's profiles | No rival verified | Medium: see ACC-024 | R1 | Advisory flags read at scan where tags carry them (which tags do is unverified), plus manual marking; the filter itself is ACC-024 | Explicit field; manual mark events | Track badge; profile settings |
| MUS-048 | Writing tags back into files | The app edits the media files themselves | Plex no; Jellyfin request #1685 (8 votes); dedicated taggers do this | Low | No | Not doing: see below | None | None |

### Browsing the library

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| MUS-049 | Music home with sections you arrange | Continue listening, recently played, recently added, loves, rediscovery and pinned playlists, in your order | Spotify cannot be customised (marked "Not Right Now"); Plexamp customisation needs Pass | High: Spotify "Customize Start Screen" 15,697 votes | R1 | Sections are saved rules over the synced library (engine owned by the discovery map), with no ads, podcasts or upsells; the same definitions later feed TV rows and the car browse tree | Section definitions in user data | Home; section editor |
| MUS-050 | Continue listening by album or playlist | Return to the album or playlist you were in, not a single track | Plexamp yes; Jellyfin low-vote requests (4 and 2); every streaming app leads with this | Medium | R1 | The rule treats albums and playlists as the unit, using the source recorded with each play | Listen events with source context | Home, first row |
| MUS-051 | Artist page | Play and shuffle, your most-played tracks, discography by type, appearances, credits by role | Spotify is the reference; Plexamp good; Navidrome artist detail page request open (26 reactions) | High: the core music page | R1 | "Popular" means your own (or the household's) most played, from the log, which is the honest version for an owned library | Artist aggregate query; play counts | Artist page |
| MUS-052 | All songs by an artist | A flat, sortable list of every track | Spotify missing; owned-library apps yes | Medium: Spotify 1,566 votes | R1 | Parity with owned-library apps; a local query | None beyond the synced library | Artist page "All songs" |
| MUS-053 | Sort and filter a discography | Oldest first, by type or by format | Finamp yes (0.9.25 beta); Jellyfin sort by year requested | Medium: Jellyfin 49 votes | R1 | Runs on the device; the sort choice is remembered per view | Sort keys | Artist page sort menu |
| MUS-054 | Album page | Discs, works, editions, dates, label, length, format badge and more by the artist | All rivals yes, less Jellyfin's disc-split bug | High | R1 | Built from MUS-008 to MUS-014; the badge comes from the playback decision (MUS-099) | Album aggregate | Album page |
| MUS-055 | Credits panel | Who wrote, produced and played, with each name linked | Roon best; Spotify SongDNA; owned files usually carry only artist and composer | Medium | R1 | Every credit row links to a role-filtered artist page | Credits per track | Player info card; track info sheet |
| MUS-056 | Library views with remembered sort | Albums, artists, songs, playlists and genres, sorted by year, date added, name or plays | Spotify lacks sort by release year; owned-library apps yes | High: Spotify 1,738 votes | R1 | Sorting runs on the device, so every sort is instant | Sort fields in the sync feed | Library tabs; sort menu |
| MUS-057 | Grid, list and compact views | See DIS-107, which owns this feature. | Finamp per tab; Marvis Pro grid, list and Cover Flow | Low | R1 | See DIS-107. | None beyond DIS-107. | View toggle |
| MUS-058 | Alphabet jump | See DIS-101, which owns this feature. | Spotify missing; Finamp fast scroller (unverified) | Medium: Spotify 884 votes | R1 | See DIS-101. | None beyond DIS-101. | Side scrubber on lists |
| MUS-059 | Recently added that ignores upgrades | New arrivals only; replacing an MP3 with a FLAC is not "new" | All rivals show recently added | Medium | R1 | Content identity (MUS-038) recognises a replacement as the same item | Added and replaced dates | Home section; library sort |
| MUS-060 | Genre, mood and label browse | Explore without typing | Spotify browse tiles; Plexamp styles and moods | Medium | R1 | From tags, with aliases from MUS-018 once they exist | Genre index | Browse screen |
| MUS-061 | Music search fields | Find by title, artist, album, credit, genre or label, as you type and offline | Spotify fast; Roon cross-linked credits; Plexamp offline search since 4.50 | High: the search engine is owned by the discovery map | R1 | The shared local index includes credits and roles, so "everything produced by X" is just a search | Index fields for credits | Search screen; type chips |
| MUS-062 | One context menu everywhere | See DIS-111, which owns this feature. Music specifics: play next, add to queue, add to playlist, go to artist or album, love, rate and info. | Spotify consistent; others yes | Medium | R1 | See DIS-111. | None beyond DIS-111. | Context menu; right-click on the web |
| MUS-063 | Multi-select and drag and drop | See DIS-110, which owns this feature. Music specifics: drop zones on the queue (next, end), and a bulk edit is one versioned queue operation that undoes in one step. | Spotify yes; Jellyfin web limited in playlists; Marvis Pro drop zones | Medium: Spotify "play several playlists together" 879 votes | R1 | See DIS-110. | None beyond DIS-110. | Lists; sidebar playlists; queue panel |
| MUS-064 | Long-press preview | Peek at an album or artist without leaving the screen | Spotify removed it | Medium: 4,701 Spotify votes to restore it | R2 | Drawn from the synced library with no network call | None beyond the synced library | Touch long-press sheet |
| MUS-065 | Swipe actions on rows | Swipe to queue or remove | Spotify yes; Finamp yes (unverified) | Low | R2 | Parity; the same core operations as the menu, with a visible alternative | None | Track and queue rows on touch screens |

### Playback and player controls

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| MUS-066 | Play the original file | FLAC arrives as FLAC; the server reads bytes and sends them | Plex, Jellyfin and Navidrome yes | High: the README's core promise | R1 | Parity with Navidrome on raw streaming; ahead on safety through short-lived, per-object signed URLs (ADR 1). | Byte-range serving; signed URLs | Quality badge |
| MUS-067 | Gapless in the web client | Live albums and DJ mixes play without gaps in a browser tab | Plex Chrome request (30 votes, current state unverified); Jellyfin web depends on the browser; Navidrome web not planned (#745) | High: Jellyfin's top music request (647 votes); gapless is Emby users' top gap | R1 | Trim values from the scan drive Media Source Extensions append windows or Web Audio scheduling; sample-accurate fixtures test each join; iOS Safari gets its own plan (ManagedMediaSource) | Priming and padding counts in the protocol; seek index Depends on MUS-230 (audio packaging into fragmented MP4); per-browser MSE support is unverified. | Player (gapless shown in track info) |
| MUS-230 | Audio packaging for the web player | Gapless and seeking work in browsers whose Media Source Extensions do not accept raw FLAC, Ogg Opus or MP3 | Not covered by the research | High: gapless is the most-voted Jellyfin music request (647 votes) | R1 | FLAC, Opus and MP3 frames are copied into fragmented MP4 in-process, without re-encoding. This is a light audio-only packager, not the video remuxer; ADR 2's statement that music needs no remuxer should be amended to name it as the exception. Which containers each browser accepts in MSE, including Safari's ManagedMediaSource, is unverified | Audio fMP4 packager in the core; frame index from the scan | None (used by the player) |
| MUS-068 | Gapless in native apps | The same on Android phones, Android TV and desktop | Plexamp yes, free; Jellyfin's official apps no, Finamp and Symfonium yes; Navidrome depends on the client | High: 647 votes | R2 | libmpv's gapless path fed the same trim values | Same as MUS-067 | Player |
| MUS-069 | Encoder delay and padding honoured | MP3, AAC and Opus albums are gapless too, not only FLAC | Plexamp spent a 2025 release on Opus gapless; Navidrome MP3 transcodes lack the header that carries this (#6170) | Medium | R1 | The core reads LAME/Xing, iTunSMPB and Opus pre-skip at scan; FLAC is exact | Trim fields | Track info sheet |
| MUS-070 | Next track fetched early | No stall between tracks on a slow link | Plexamp advanced pre-caching (free); Symfonium playback cache | Medium | R1 | The queue order is known exactly, so the client fetches the next item's opening bytes before the current one ends | None beyond byte serving | Buffer state in the player |
| MUS-071 | Instant, exact seeking | Scrub anywhere without a stall | All rivals yes | Medium | R1 | Parity in feel; seek tables built at scan map time to byte, so the client asks for the right range first time | Seek index per file | Scrubber |
| MUS-072 | Fades on pause, skip and resume | No pops when pausing or after a Bluetooth drop | Plexamp yes (fade-in after Bluetooth pause on Android, Sept 2026); Navidrome resumes unexpectedly after standby (#5446) | Low | R1 | Short gain ramps in the client's player layer | None (client feature) | Settings: playback |
| MUS-073 | Browser media controls | Lock screen, notification, media keys and headset buttons control the web client | Navidrome uses the Media Session API but its state drifts (#4744); not every browser supports it | Medium | R1 | Position comes from the audio clock and is pushed on every queue edit, so system controls stay in step | None (client feature) | OS media panel; lock screen |
| MUS-074 | Background playback with lock-screen controls on phones | Music keeps going with full controls and Bluetooth metadata | Plexamp, Symfonium and Finamp yes | High: ADR 2 requires it | R2 | Parity; a native player module (Android in R2, iPhone with the Apple builds) | None | Lock screen; notification; car and headset displays |
| MUS-075 | Interruptions handled | Calls and navigation prompts pause and resume cleanly | Jellyfin Android added this in 2.7.2; others unverified | Medium | R2 | Parity; native audio-focus handling in the player module | None | None (behaviour) |
| MUS-076 | Sleep timer | Stop after N minutes, after this track, after this album or at the end of the queue | Plexamp yes, including "after album"; Spotify desktop missing (1,025 votes in 2022); Finamp yes | Medium: 1,025 votes | R1 | "End of album" uses the queue's lane boundaries, with a gentle fade at the end | None (client feature) | Player menu; queue menu |
| MUS-077 | Repeat and stop-after | Repeat one or all; stop after this track or album | Plex repeat one and all; Jellyfin request to remember the modes (11 votes); foobar2000 stop-after | Low | R1 | The modes live on the queue object, so they survive restarts and follow a handoff | Queue mode fields | Player controls |
| MUS-078 | Resume long tracks | A two-hour DJ mix or live set resumes where you stopped | Plexamp resumes long audio; music players not covered by the research | Low | R2 | Resume applies only above a length threshold, from position events in the log Owns long-audio resume; LAT-011 adds per-kind thresholds and DIS-020 shows the row. Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Position events | Track row progress; player |
| MUS-079 | Damaged files skipped safely | A corrupt file is skipped with a notice instead of blasting noise | Not covered by the research; one Hacker News user had a corrupt file play at painful volume | Low | R1 | The health report flags damage at scan; the player skips flagged files and keeps a true-peak ceiling | Health flags in the sync feed | Notice; link to the health report |
| MUS-080 | Keyboard shortcuts and command palette | See CLI-061, which owns this feature. | Spotify hotkey to queue requested (988 votes in 2022); Finamp Space and Ctrl+N/P | Medium: 988 votes | R2 | See CLI-061. | None beyond CLI-061. | Web client; shortcut help |
| MUS-081 | Desktop media keys and system panels | See CLI-063, which owns this feature. Music specifics: the module responds only while Gunmetal is the active player and never launches the app uninvited. | Plexamp fixed MPRIS and Windows artwork in 4.50; Jellyfin web only partly works with KDE; a tool to stop the Mac Music app launching on the play key reached 669 points on Hacker News | Medium | R2 | See CLI-063. | None beyond CLI-063. | OS media panels |
| MUS-082 | Mini player window | See CLI-065, which owns this feature. | Apple MiniPlayer; Feishin request (32 votes) | Low | R2 | See CLI-065. | None beyond CLI-065. | Mini player |
| MUS-083 | Visualiser | Something to watch on a big screen | Plexamp yes; Caldera has a MilkDrop-compatible engine; Plex request (27 posts) | Low | Later | Client-side, from the decoded audio | None | Player; TV ambient mode |

### Loudness, transitions and sound

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| MUS-084 | ReplayGain and R128 tags, track and album | Existing scans from foobar2000 or dBpoweramp are honoured | Plex unclear (a June 2026 forum post asks it to honour them); Jellyfin yes, album gain only since 12.0; Navidrome passes them to clients | High: Emby users rank ReplayGain their second music gap (98 replies) | R1 | Read by the core at scan and applied the same way by every Gunmetal client | Gain fields | Track info "gain source" |
| MUS-085 | Opus gain done right | Opus files play at the correct level | Rivals unverified | Low | R1 | The RFC 7845 rule (header output gain always applies, R128 tags add to it) implemented and tested in the core | Opus header gain | None (behaviour) |
| MUS-086 | Loudness measured for untagged files | Normalisation works on a library nobody pre-scanned | Plex yes, on the server and, since Sept 2026, on the device; Jellyfin server-side (unverified); Navidrome relies on tags (unverified) | Medium: Symfonium users ask for Plexamp-style levelling | R1 | Integrated loudness and true peak (BS.1770, EBU R128) measured in Rust by a background, throttled, resumable job; Opus waits for a pure-Rust decoder Owns scan-time loudness measurement; LIB-066 points here. | Decode pass; job queue; progress Depends on an ADR approving in-process pure-Rust decoders for untrusted audio (crate, licence, fuzzing and resource limits); Symphonia (MPL-2.0) lacks Opus and HE-AAC per the research. If that ADR is not accepted before R1, R1 ships with loudness tags plus the MUS-089 fallback gain and this row moves to R2. | Admin job progress; track info |
| MUS-087 | Auto, track and album modes | Albums keep their internal dynamics; shuffles stay even | Roon Auto mode; Spotify applies album gain to whole albums; rivals unverified | Medium | R1 | Auto reads the queue: consecutive tracks from one album in the source lane get album gain, everything else track gain | None (decided on the client from the queue) | Settings: playback; player info |
| MUS-088 | Target level with no clipping | A chosen loudness target, and nothing distorts | Plex never boosts (2023 change); Spotify -11, -14 or -19 LUFS with a limiter on Loud; Roon -14 LUFS using true peak | Medium | R1 | Positive gain is capped by each track's measured true-peak headroom; a limiter runs only on an opt-in louder target | True peak per track | Settings: playback |
| MUS-089 | Fallback for unmeasured tracks | A brand-new track is not suddenly louder than the rest | Roon applies -5 dB; Plexamp analyses on the device | Low | R1 | A fixed attenuation until the measurement lands (the value to be set in listening tests) | Measurement state per track | Track info "estimated" |
| MUS-090 | Show the gain applied | See what normalisation did and why | A Plexamp user is unsure whether their ReplayGain is honoured (June 2026) | Low | R1 | The player shows the source (tag, measured or fallback) and the dB applied | None beyond gain fields | Track info; signal path |
| MUS-091 | Crossfade | Songs overlap by a set number of seconds | Plexamp Sweet Fades (free); Jellyfin no; Navidrome depends on the client | Medium: Jellyfin 138 votes | R2 | One mixer design for the web (Web Audio) and native players, never applied inside gapless albums | None | Settings: playback |
| MUS-092 | Album-aware smart fades | Fades between unrelated songs, never inside a continuous album | Plexamp Sweet Fades; Symfonium Smart Fades; Apple AutoMix skips transitions within an album | Medium | R2 | Uses the scan's gapless and loudness data plus the queue lanes to choose fade shape and length | Fade hints from the analysis pass | Settings: playback |
| MUS-093 | Beat-matched DJ transitions | Tempo- and key-matched blends between songs | Apple AutoMix; Spotify Mix and Smart Reorder (Premium); no self-hosted rival | Low: not table stakes for a library player | No | Not doing: see below | None | None |
| MUS-094 | Equaliser, graphic and parametric | Shape the sound, with presets and a preamp | Plexamp ten-band (Pass); Jellyfin no; Navidrome no in its web player | Medium: Jellyfin 40 votes; Finamp request #453 | R2 | Free; presets sync per user; switching it on removes the bit-perfect badge (MUS-103) | Preset storage | Settings: sound; quick toggle in the player |
| MUS-095 | Headphone correction presets | Pick your headphone model and get a correction curve | Plexamp about 6,000 presets (beta, Sept 2026); Symfonium AutoEQ | Low | Later | Applied through MUS-094; the preset data's licence is unverified | Preset dataset | Settings: sound |
| MUS-096 | Per-output profiles | Different EQ and gain for the car, headphones and a DAC | Plexamp per-output EQ presets; Symfonium per-device settings | Low | Later | Profiles keyed to the output device the native player reports | Profile storage | Settings: sound |
| MUS-097 | Crossfeed, convolution and room correction | Headphone crossfeed and room EQ | Roon | Low | Later | A native DSP chain; Roon warns some DSP is CPU-heavy | None | Settings: sound |
| MUS-098 | Night-mode compression | Quiet passages stay audible at low volume | Plex request (134 votes, mostly about video); Spotify's Loud setting | Low: for music | Later | A client-side compressor shared with the video map's night mode | None | Player quick setting |

### Output quality and data use

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| MUS-099 | A quality badge that tells the truth | "Original FLAC, 24-bit, 96 kHz, played directly", or the reason it was converted | Finamp says when the server is transcoding; Apple shows lossless badges | Medium | R1 | The core's playback decision returns its choice and its reason, and the badge shows exactly that | Decision record per play | Player; now-playing bar; track info |
| MUS-100 | Sample-rate matching | Output switches to each file's rate instead of resampling | Plexamp (Pass) on macOS, iOS and Android; Symfonium; Roon | Medium: Plex hi-res (31 votes) and passthrough (49) requests | R2 | Free; the native player module sets the output rate | None | Settings: sound |
| MUS-101 | Bit-perfect output on Android | Nothing touches the samples on the way to a USB DAC | Plexamp Android (2024); Symfonium on Android 14 and later | Medium: Jellyfin "Roon-like" request (52 votes) | R2 | Android's bit-perfect mixer mode (API level 34) in the native module, free | None | Settings: sound; bit-perfect badge |
| MUS-102 | Exclusive output on desktop | The desktop app takes the DAC exclusively | Plexamp WASAPI exclusive (broken in an early 4.50 beta); foobar2000 WASAPI; Roon | Low | R2 | WASAPI exclusive and ALSA hardware devices in the desktop shell; CoreAudio with the Apple builds | None | Settings: sound |
| MUS-103 | Signal path view | Every stage that changed the audio, and a bit-perfect badge only when none did | Roon only | Low: it makes hi-res claims checkable | R2 | The decision engine and the native output report each stage: normalisation, EQ, resampling, conversion | Decision record | Player info sheet |
| MUS-104 | DSD (DSF and DFF) | DSD files play | Symfonium and Roon; Jellyfin and Navidrome transcode (unverified) | Low: niche | Later | A parser in the core; DoP or PCM conversion in native players | None beyond parsing | Track info |
| MUS-105 | Multichannel and Dolby Atmos music | Surround mixes play as surround | Apple Music best; Plex spatial audio request 222 votes, multichannel FLAC on Shield 216 | Medium: 222 and 216 votes | Later | Native passthrough on TV clients; Atmos licensing needs checking | None beyond parsing | Track info; TV player |
| MUS-106 | Opus streams for mobile data | Smaller streams when bandwidth or data is short | Plex, Finamp and Navidrome yes; a Jellyfin request says a 2 Mbps music cap causes buffering (#3129) | Medium | R2 | A cheap Opus encode in the sandboxed worker (ADR 2) that keeps pre-skip so gapless survives, cached for the next device | Sandboxed transcode job; cache | Quality badge; Settings: data use |
| MUS-107 | Network policy | Originals on Wi-Fi, Opus or nothing on mobile data, and a hard "never use mobile data" switch | Plexamp downloaded over mobile data while showing "paused" (Sept 2026 bug); Finamp requests (20 and 7 reactions) | Medium | R2 | The policy is evaluated on the device, with tests that prove a forbidden network is never touched | None | Settings: data use |

### Now playing

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| MUS-108 | Persistent now-playing bar | Play, skip, progress, love and output device on every screen | Spotify stable; Apple's floating bar criticised by NN/g; Tidal shipped a mini player without skip buttons | High: ADR 2, decision 7 | R1 | A fixed position under the layout contract (MUS-113), and no scrolling ticker | None | Bar on every screen |
| MUS-109 | Love from the bar | One tap loves the current track | Spotify swapped its heart for a plus in 2023 | Medium: "Bring back the heart button!" has 5,769 votes | R1 | A plain love toggle that writes a user-log event | Love events | Now-playing bar; player |
| MUS-110 | Full-screen player | Artwork-led colour, an always-visible scrubber, a "Playing from" link and swipe to skip | Plexamp UltraBlur (free); Finamp artwork colours; Tidal's 2026 redesign hid the scrubber and removed "Playing from" and drew complaints | Medium: the Tidal backlash | R1 | Colours computed at scan are synced, so the player draws at once with no image work on the client; its own look, per ADR 2 | Palette per album | Player screen |
| MUS-111 | Info cards under the player | Scroll down for a lyrics preview, credits, album details and a biography | Spotify, including SongDNA; Plexamp biography, lyrics Pass only | Medium | R2 | Built from local data; the biography appears only once a metadata plugin exists Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | None beyond the synced library | Player screen |
| MUS-112 | Landscape player | Artwork and lyrics side by side on a docked phone | Apple, new in iOS 27 | Low | R2 | Parity; a native phone and tablet layout (wide web layouts get this in R1 anyway) | None | Player, landscape |
| MUS-113 | Layout contract | The bar, queue access, lyrics, device picker and scrubber keep their places across releases | Tidal, Spotify, YouTube Music and Plex redesigns all drew backlash | Medium: Plex rollback vote 504; Spotify heart 5,769 | R1 | Positions are covered by visual regression tests; layout changes ship as an opt-in preview for one release, with a way back | None | All player surfaces; release notes |
| MUS-114 | Track info sheet | File path, format, size, tags as read, gain applied and identity used | Navidrome `inspect` (admin command); others not covered | Low | R1 | Shows the core's own parse result and where each field came from | Parse summary per file Uses the ADM-125 inspect API. | Context menu "Info" |
| MUS-115 | Choose what the player shows | Hide or show format, credits or the next track | Finamp (0.9.25 beta) | Low | Later | A per-user display preference | User setting | Player settings |

### Queue

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| MUS-116 | Three-lane queue | "Up next" (your picks), then "From" the album or playlist, then an optional "Continue with" lane | Spotify two lanes; Finamp "Next Up"; Apple's queue order confuses people | High: queue complaints across Apple, Spotify and YouTube Music | R1 | The queue rules are pure logic in the core, so every client behaves the same and mutation tests pin the behaviour down | Queue object with lanes | Queue panel; player |
| MUS-117 | Play next keeps your order | Three "play next" picks play in the order you chose them | Apple plays them in reverse, a complaint for years | Medium | R1 | An insertion cursor inside the "Up next" lane | None beyond the queue object | Context menu "Play next" |
| MUS-118 | Add to queue and play last | Append to your picks, or to the very end | Finamp has four documented verbs; Spotify only "Add to queue" | Medium: Spotify "Queue to Next or Last" 1,635 votes | R1 | Clear, documented verbs mapped to lane operations | None beyond the queue object | Context menu |
| MUS-119 | Edit the whole queue | Every upcoming track with duration and album; reorder, remove and clear | Plex request (42 votes); Spotify's queue is Premium and was squeezed into a side panel in 2024 | Medium: 42 votes and the 2024 coverage | R1 | A full-height queue view, with nothing paywalled | None beyond the queue object | Queue panel (full height on desktop) |
| MUS-120 | Reorder while shuffled | Dragging still works with shuffle on | Finamp cannot (beta limitation) | Low | R1 | Shuffle is stored as an explicit order on the queue, so a drag is an ordinary move | Shuffle order on the queue | Queue panel |
| MUS-121 | Undo queue edits | Recover from a slip after a remove or a clear | None found | Low: a gap nobody advertises | R2 | Queue operations are logged, and undo applies the inverse Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Operation log per queue | Undo notice |
| MUS-122 | Persistent queue on every device | The queue and position survive a restart, and any signed-in device opens the same queue | Plex server-side queues; Jellyfin request (13 votes), Finamp "Past Queues"; Navidrome `savePlayQueue` | Medium: Emby users asked for this in Dec 2025 | R1 | The queue is a versioned server object edited through small operations that clients apply optimistically and the server orders | A per-profile queue document with lanes (MUS-116) and context IDs (LAT-009); an operation endpoint with server-assigned versions; a rejection and rebase rule for stale versions; stored in the user log, so a cache rebuild keeps it | Player; resume prompt |
| MUS-123 | "Playing from" on every item | Know why a track is playing and jump back to its source | Spotify yes; Tidal removed it and users asked for it back | Medium | R1 | Every queue item carries a source field | Source field | Queue rows; player label |
| MUS-124 | Queue history | See what just played and play it again | Finamp shows previous tracks | Low | R2 | From the queue's played section and the log Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | None beyond the queue and log | Queue panel (scroll up) |
| MUS-125 | Save queue as playlist | Keep a good session | Not covered by the research beyond Jellyfin web (unverified) | Low | R1 | One operation from queue to playlist | None beyond playlists | Queue menu |
| MUS-126 | Shuffle modes: random and spread out | True random, or spread so the same artist or album never bunches and recent plays come later | Spotify "Fewer Repeats" plus "Standard" (Nov 2025); Symfonium request for true random (15 posts); Roon reworked shuffle (Sept 2026) | Medium | R1 | A seeded shuffle in the core gives the same order on every device, and the queue shows it before it plays | Seed on the queue; recent plays | Shuffle button; Settings: playback |
| MUS-127 | Shuffle by album | Random albums, each played in order | Spotify no; Jellyfin request (22 votes); Finamp "Album Mix" | Medium: Spotify 1,073 votes | R1 | An album-level order from the same seeded shuffle | None beyond the queue object | Shuffle menu |
| MUS-128 | Reshuffle the rest | Shuffle what is left without touching your picks | Plex request (26 posts) | Low | R1 | Shuffle only ever touches the source lane | None beyond the queue object | Queue menu |
| MUS-129 | Suggestions lane, visible and off by default | When the queue ends, suggestions you can see before they play, and switch off | Spotify shows them since May 2025; Plexamp autoplay needs Pass; Apple users report unrelated tracks after an album | Medium | R1 | "Continue with" is labelled, comes from library radio (MUS-165), and never lands in a playlist | Radio rule | Queue panel toggle |
| MUS-130 | Suggestions mixed into your own playlists | A service adds tracks to a playlist you made | Spotify Smart Shuffle (Premium) | Low | No | Not doing: see below | None | None |
| MUS-131 | Several saved queues | Keep a workout queue and a work queue, each with its own position | Symfonium multiple queues; Finamp "Past Queues" | Low: Audiobookshelf's queue request has 99 upvotes | R2 | The queue object supports named contexts, which audiobooks and podcasts later reuse | Several queue contexts per user | Queue switcher |

### Playlists and smart playlists

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| MUS-132 | Manual playlists | Create, rename, reorder and remove; duplicates allowed | All rivals; Jellyfin stores items as rows since 12.0 | High: Plex "Better Playlists" 1,425 votes | R1 | Playlists live in the user log, so a cache rebuild cannot lose them, and items point at content identity, so moving files does not break them | Playlist events; identity references | Playlist page; sidebar |
| MUS-133 | Add-to-playlist sheet | Search your playlists, see where a track already is, add to several at once | Spotify sheet | Medium: adding one song to several playlists had 1,026 votes (2022) | R1 | Local search over playlists; one batch edit | None beyond playlists | Add-to-playlist sheet |
| MUS-134 | Duplicate warning | Asked before adding a track that is already there | Spotify yes | Low | R1 | Checked against identity, not file path | None beyond playlists | Add sheet prompt |
| MUS-135 | Sort, filter and search inside a playlist | Find a song in a 2,000-track playlist; sort by date added | Plex requested; Jellyfin dynamic sorting requested (23 votes) | Medium: Plex 183 votes | R1 | Runs on the device over the synced library | None beyond the synced library | Playlist page search and sort |
| MUS-136 | Playlist folders | Hundreds of playlists stay organised | Plex no (43 votes); Navidrome's most-upvoted idea in the sample (32); Spotify desktop only | Medium: Spotify mobile folders 1,852 votes | R2 | Folders are nested user-log entries Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Folder events | Sidebar; playlists tab |
| MUS-137 | Automatic playlist covers | A mosaic of the albums inside | Plex yes; Jellyfin yes (12.0); Navidrome yes | Low | R1 | Parity; drawn on the client from artwork already synced | None | Playlist tiles |
| MUS-138 | Custom playlist image | Upload a picture for a playlist | Plex custom posters; Jellyfin images settable in 12.0; Navidrome upload (#406, 49 reactions) | Low: 49 reactions | R2 | Parity; uploads pass the safe-upload rules (ACC-125) | Upload store | Playlist edit sheet |
| MUS-139 | Pin and love playlists | The playlists you use most stay on top | Navidrome playlist favourites (0.64); Apple pins in iOS 26 | Low | R1 | Parity; pins and loves are user-log flags | Pin events | Sidebar; library |
| MUS-140 | M3U and M3U8 import and export | Bring playlists from another player and take them away | Navidrome imports M3U from the library; Jellyfin request (17 votes); Plex unverified | Medium: switching cost (a Plex user said rebuilding 614 items was taking weeks) | R1 | Path match first, then tags and MusicBrainz IDs; a report lists every unmatched line | Import job; matcher in the core | Playlist menu: import and export |
| MUS-141 | Import from Plex, Jellyfin and Navidrome | See ADM-036, which owns this feature. Also ADM-038 (Jellyfin and Emby) and ADM-040 (Navidrome). Playlists use the MUS-140 matcher. | Navidrome imports and exports its own; Plex has no built-in music playlist export | Medium | R2 | See ADM-036. | None beyond ADM-036. | Setup and migration screens |
| MUS-142 | Import from streaming services | Bring a Spotify or other service playlist, matched to files you own | Apple Music imports from five services (iOS 26); Jellyfin request (29 votes) | Medium: Apple has set the expectation | Later | Matching on title, artist and duration in the core; missing tracks are listed, never fetched | Matcher | Import wizard |
| MUS-143 | Smart playlists | Rule-based playlists that keep themselves up to date | Plex yes (free); Jellyfin none (planned, community plugin); Navidrome yes, as hand-written JSON | High: Jellyfin 588 votes; Navidrome's request drew 232 comments | R1 | One typed rule language in the core, compiled into the server and every client, so a smart playlist gives the same result online and offline | Rule tree; evaluator; change triggers | Playlist page |
| MUS-144 | Visual rule editor | Build rules from menus and see the matches live | Navidrome has none; Marvis Pro Smart Rules | High: same evidence | R1 | The editor writes the typed rule tree and previews it locally | None beyond the rule engine | Smart playlist editor |
| MUS-145 | Rich rule fields | Any tag, plays, last played, date added, rating, love, format, loudness, missing tags, membership of other playlists | Plex filters (playlist-membership rules requested); Navidrome more than 80 fields | Medium: Plex 101 votes | R1 | Every field comes from scan data or the user log, all of it on the device | Field catalogue | Rule editor |
| MUS-146 | Limits, sorts and percentages | "50 random unplayed tracks" or "top 5 percent by plays" | Plex limit and sort; Navidrome limits, percentages and random | Medium | R1 | Seeded randomness keeps a random smart playlist stable until it refreshes | None beyond the rule engine | Rule editor |
| MUS-147 | Snapshot refresh | Live, or a stable daily or weekly snapshot | Navidrome per-playlist refresh delay (0.64) | Low | R2 | A dated snapshot, so downloads from it do not churn | Snapshot job | Rule editor |
| MUS-148 | Navidrome smart playlist import | See DIS-125, which owns this feature. | Navidrome's own format | Low | R2 | See DIS-125. | None beyond DIS-125. | Import wizard |
| MUS-149 | Loved tracks as a playlist | Shuffle or download your loves like any playlist | Spotify Liked Songs; Jellyfin request (25 votes; Finamp #133) | Medium | R1 | A built-in smart playlist over the love flag | None beyond the rule engine | Library; home |
| MUS-150 | Shared and collaborative playlists | See ACC-091, which owns this feature. | Plex shares to home users; Jellyfin request to protect playlists from non-owners (19 votes); Navidrome public only, collaborative not planned (#1523) | Medium: see ACC-091 | R2 | See ACC-091. | None beyond ACC-091. | Playlist share sheet |
| MUS-151 | Share links for music | See ACC-086, which owns this feature. | Navidrome yes (on by default since 0.63, with several 2026 advisories); Plexamp yes | High: see ACC-086 | R2 | See ACC-086. | None beyond ACC-086. | Share sheet; "My shares" |
| MUS-152 | Edit playlists offline | See CLI-094, which owns this feature. Music specifics: playlist edits are user-log events merged by a stated rule. | Finamp request (#1065) | Low | R2 | See CLI-094. | None beyond CLI-094. | Playlist page offline state |
| MUS-153 | Playlist from a prompt | Describe a mood and get a playlist | Plexamp removed Sonic Sage in 2026; Spotify AI Playlist | Low | Later | Only as an opt-in plugin with an explicit grant to a model provider, or a local model | Plugin hook | Create menu |

### Lyrics

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| MUS-154 | Embedded lyrics | Lyrics stored in the file appear | Plex partial (reading embedded lyrics still requested, 49 votes); Jellyfin yes since 10.9; Navidrome yes | High: Jellyfin's lyrics request reached 550 votes | R1 | Parsed at scan from USLT, SYLT, LYRICS and ©lyr, and carried in the synced library; free | Lyrics store | Lyrics view |
| MUS-155 | Synced .lrc sidecars | Lines highlight in time and scroll along | Plex unverified; Jellyfin yes; Navidrome yes, with reports of .lrc files not showing (#4148, #5531) | High: Navidrome .lrc request 56 reactions | R1 | A malformed file returns a typed error that lands in the health report, instead of silently showing nothing | LRC parser | Lyrics view; health report |
| MUS-156 | Word-by-word lyrics | Karaoke-style highlighting from Enhanced LRC | Jellyfin word timing since 10.11; Navidrome yes (0.63); Apple best | Medium | R1 | Word timings parsed in the core | Enhanced LRC parser | Lyrics view |
| MUS-157 | Multi-voice TTML lyrics | Duet parts shown separately | Navidrome yes (0.63); Jellyfin requested (16 votes) | Low: 16 votes | R2 | TTML parsed in the core, refusing external entities | Safe XML parser | Lyrics view |
| MUS-158 | Lyrics stay open | The lyrics view stays up from track to track, with an option to open the player on lyrics | Plex requested; Plexamp keeps local lyrics on screen (2023) | Medium: Plex 260 votes | R1 | A remembered view state on the player | User setting | Lyrics view; Settings: playback |
| MUS-159 | Full-screen lyrics with tap to seek | Read along, and tap a line to jump there | Not covered by the research | Low | R2 | Line times map straight to seek positions Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | None beyond lyrics | Lyrics view |
| MUS-160 | Lyrics offline | See CLI-098, which owns this feature. Music specifics: lyrics travel inside the synced library, so a downloaded track always has them. | Spotify missing; Apple yes (iOS 26) | Medium: Spotify 1,600 votes | R2 | See CLI-098. | None beyond CLI-098. | Lyrics view offline |
| MUS-161 | Search by lyric | Find a song from a line you remember | Apple (unverified); Symfonium request (10 posts); AudioMuse-AI as an add-on | Low | R2 | Lyrics added to the local search index, within its size budget | Index field | Search |
| MUS-162 | Online lyrics lookup | Lyrics appear for files that have none | Plexamp (Pass); Jellyfin LrcLib plugin; Navidrome lyrics-provider plugins (0.61) | Medium | R2 | A first-party LRCLIB plugin (free, keyless, open source), off by default and granted one host | Plugin host; lyrics cache | Lyrics view "Find lyrics"; plugin settings |
| MUS-163 | Translation and pronunciation | Understand and sing along in other languages | Apple only (iOS 26) | Low | Later | Plugin territory, since it needs a translation service | Plugin hook | Lyrics view |
| MUS-164 | Karaoke vocal reduction | Sing along with the vocals turned down | Apple Music Sing | Low | No | Not doing: see below | None | None |

### Radio, mixes and discovery

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| MUS-165 | Library radio from any seed | Endless play from your own music around a track, album, artist or genre | Plexamp yes (track and album radio need Pass); Jellyfin Instant Mix and ListenBrainz (12.0); Navidrome Instant Mix via Last.fm, Deezer or plugins | High: the most-replied comment in Hacker News's "Jellyfin as a Spotify alternative" thread (26 replies) says radio is what you lose | R1 | Music specifics of DIS-067, using the neighbour table owned by DIS-060. Computed locally from credits, genres, eras and household co-listening; works offline and labels every pick. Behind Plexamp (sonic analysis), Jellyfin 12.0 (ListenBrainz) and Navidrome (Last.fm, Deezer) until similarity data exists (MUS-175 in R2, MUS-172 Later); parity at best with a basic tag-based Instant Mix. | None beyond DIS-060 and DIS-067. | Context menu "Start radio"; queue "Continue with" |
| MUS-166 | Rediscovery mixes | Loved but not played lately, albums never played, forgotten favourites | Plexamp (Pass); Roon ARC Smart Downloads | Medium | R2 | Built-in smart playlists over the log, free Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | None beyond the rule engine | Home sections |
| MUS-167 | On this day | Albums released, or played, on this date in earlier years | Plexamp yes | Low | R2 | Date rules in the shared rule language Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | None beyond the rule engine | Home section |
| MUS-168 | Personal mixes | See DIS-069, which owns this feature. | Plexamp Mixes for You (Pass); Spotify Daily Mix | Medium | R2 | See DIS-069. | None beyond DIS-069. | Home section |
| MUS-169 | Mix from several seeds | See DIS-074, which owns this feature. | Plexamp (Pass) | Low | Later | See DIS-074. | None beyond DIS-074. | Mix builder sheet |
| MUS-170 | Hide, snooze or exclude | See DIS-054, which owns this feature. Music specifics: excluding an album from everyday shuffle is DIS-057. | Plex requested; Spotify hide and 30-day snooze; Marvis Pro auto-skips disliked songs | Medium: Plex 44 votes; Spotify "Reset Taste Profile" 5,871 | R2 | See DIS-054. | None beyond DIS-054. | Context menu; hidden items list |
| MUS-171 | Ratings that steer playback | Skip one-star tracks; weight radio by rating | Plex request (23 votes); Navidrome discussion (6) | Low | R2 | An option on shuffle and radio rules | None beyond ratings | Settings: playback |
| MUS-172 | Sonic similarity | Radio, similar albums and similar artists chosen by how the music sounds | Plexamp (Pass; Plex says analysis can take hours or days); Jellyfin requested; Navidrome via a plugin such as AudioMuse-AI | Medium: Jellyfin 98 votes | Later | An opt-in job sharing the loudness decode pass; compact features in the style of bliss-rs (GPL-3.0, licence check needed), measured on low-end hardware first; heavier models only as plugins | Analysis job; feature vectors | Context menu "Sounds like" |
| MUS-173 | Sonic path between two songs | A journey from one track to another | Plexamp Sonic Adventure (Pass); Navidrome `findSonicPath` via a plugin | Low | Later | A path search over the MUS-172 features | None beyond analysis | Mix builder |
| MUS-174 | Guest DJ helper | Keeps adding fitting tracks to the queue | Plexamp Guest DJ (Pass) | Low | Later | Writes only to the labelled "Continue with" lane | None beyond radio | Queue panel |
| MUS-175 | Similar-artist and similar-track data from ListenBrainz or Last.fm | Radio and "more like this" draw on the listening services' similarity data, so a new or one-person library gets good picks | Jellyfin ListenBrainz source (12.0); Navidrome plugin | Low | R2 | A first-party plugin with a network grant, off by default. Results are written into the DIS-060 neighbour table, labelled by source, and stay usable offline. Parity with Jellyfin 12.0 and Navidrome on source data; the edge is that nothing is fetched until the owner grants it. | Plugin host (INT-054); neighbour-table import from the plugin | Home section |
| MUS-176 | New releases from artists you follow | See DIS-043, which owns this feature. | Spotify Release Radar; an owned library only knows what has been added | Low | R2 | See DIS-043. | None beyond DIS-043. | Home section; artist page |
| MUS-231 | Where to buy | Store links (Bandcamp and others) for albums you are missing and new releases from artists you follow | Not covered by the research for any media server | Medium: buying music legally is the biggest gap after leaving streaming (pain-points theme 13) | R2 | A MusicBrainz-relations plugin, labelled and off by default; Gunmetal sells nothing and earns nothing from the links | Plugin reading MusicBrainz URL relations | Album and artist pages (missing albums, new releases) |
| MUS-177 | Internet radio stations | Play stream stations, including channels an M3U marks as radio, alongside the library | Plex requested (272 votes); Navidrome admin-managed stations | Medium: Plex 272 votes | R3 | Stations arrive with the M3U module and play in the music player with the now-playing bar | Station list from M3U | Radio section; player |
| MUS-178 | Streaming catalogue in radio | Radio blends in music you do not own | Roon Radio mixes in Tidal and Qobuz; Plex ended its TIDAL integration in Oct 2024 | Low: among self-hosters | No | Not doing: see below | None | None |
| MUS-179 | Tour dates | When an artist plays nearby | Spotify and Apple | Low | No | Not doing: see below | None | None |

### Loves, ratings, history and statistics

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| MUS-180 | Loves | Heart a track, album, artist or playlist | All rivals; Jellyfin loses song favourites during playback (#14981, 25 reactions) | High | R1 | Loves are user-log events, so a playback race or a cache rebuild cannot lose them | Love events | Heart on rows, pages and the bar |
| MUS-181 | Star ratings | Rate from 1 to 5, with optional half stars | Plex yes (half stars requested); Jellyfin favourites only (unverified); Navidrome yes, per user | Medium: Plex 24 votes; Emby users ask for any ratings | R1 | Per user, in the user log, usable in rules (the rating model is open decision 3) Rateable objects: tracks, albums and artists, each rated directly. An album rating is entered by the user, never derived from its tracks; when an album has none, sorting by rating places it after rated albums. Playlists are not rated. General rating behaviour is DIS-047. | Rating events | Rating control in menu and track info |
| MUS-182 | Play counts, last played and skips | Your own numbers on every track, album and artist | All rivals store them; Spotify does not show them | Medium: Spotify requests with 963 and 901 votes | R1 | Derived from the append-only log by rules in the core; skips keep the time played | Aggregates rebuilt from the log | Track rows; album and artist pages |
| MUS-183 | Listening history by date | What did I play last March | Plex recent plays; Jellyfin plugin; Navidrome scrobble history (0.64) | Medium: Spotify 2,035 votes | R1 | The log is the history; queries run on the device | None beyond the log | History page |
| MUS-184 | Remove plays from history | Keep a guilty pleasure out of your record | Spotify missing | Medium: Spotify 5,458 votes | R1 | Removal is a new event; the log stays append-only and exports honour it | Removal events | History row menu |
| MUS-185 | Private listening | See ACC-117, which owns this feature. | None among servers | Medium: see ACC-117 | R1 | See ACC-117. | None beyond ACC-117. | Player menu; private indicator |
| MUS-186 | Charts by period | See DIS-066, which owns this feature. | Plexamp weekly and monthly charts; Jellyfin plugin | Medium | R2 | See DIS-066. | None beyond DIS-066. | Statistics page |
| MUS-187 | Year in review, any time | A personal summary you can open whenever you like | Spotify Wrapped; Doppler Listening Reports | Medium | R2 | From the log; an image to share only when asked Music view of DIS-183, which owns the statistics engine. | Report job | Statistics page |
| MUS-188 | Export listening history | Take your history anywhere in a documented format | ListenBrainz's listen JSON is the nearest open format | Medium: see ACC-010 | R1 | ADR 1 already makes the log exportable; the format is versioned and documented | Export endpoint | Settings: your data |
| MUS-189 | Import listening history | See ADM-042, which owns this feature. Pulling history from the services' APIs is INT-108 (R2). | Navidrome requested (28 upvotes); Plex requested | Medium: 28 upvotes | R1 | See ADM-042. | None beyond ADM-042. | Settings: import |
| MUS-190 | Who is listening now | See ADM-099, which owns this feature. Music specifics: private sessions show only as private. | Plex dashboard; Jellyfin dashboard; Navidrome panel (0.62) | Medium | R1 | See ADM-099. | None beyond ADM-099. | Admin dashboard |
| MUS-191 | Friends' listening feed | See what friends are playing | Spotify Friend Activity; Apple Friends playlist (iOS 27) | Low: among self-hosters | No | Not doing: see below | None | None |

### Scrobbling

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| MUS-192 | Last.fm scrobbling | See INT-102, which owns this feature. Music specifics: the half-or-four-minutes rule uses exact client play time. | Plex built in (2025 forum post); Jellyfin plugin; Navidrome built in, per user | Medium: Jellyfin native request 27 votes | R2 | See INT-102. | None beyond INT-102. | Settings: connected services |
| MUS-193 | ListenBrainz scrobbling | See INT-103, which owns this feature. Music specifics: MusicBrainz IDs from tags go in every submission. | Plex no (requested since 2018); Jellyfin plugin; Navidrome built in | Medium: Plex 23 votes; Jellyfin 28 | R2 | See INT-103. | None beyond INT-103. | Settings: connected services |
| MUS-194 | Delayed plays sent in order | See INT-102, which owns this feature. Music specifics: plays carry their real timestamps in the log from R1, and the cursor (INT-064) sends the oldest first in batches. | Finamp request (#194, 19 reactions); Jellyfin's ListenBrainz plugin caches during outages | Medium | R2 | See INT-102. | None beyond INT-102. | Connected services status |
| MUS-195 | Scrobble rules per person and library | See INT-105, which owns this feature. | Navidrome per-user filtering (0.64) | Low | R2 | See INT-105. | None beyond INT-105. | Connected services settings |
| MUS-196 | Love sync with services | Loving a song also loves it on Last.fm or ListenBrainz | Navidrome not planned (#1547, 17 reactions); Jellyfin's ListenBrainz plugin syncs both ways | Low | Later | The plugin mirrors love events | None beyond the plugin | Connected services settings |

### Devices, handoff and remote control

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| MUS-197 | Device picker and handoff | See CLI-101, which owns this feature. Music specifics: the queue lanes (MUS-116) and position move with playback; the target device's volume is shown and can be changed. | Spotify Connect is the reference; Plexamp yes; Jellyfin requested; Finamp "Play On" requested; Navidrome no | High: Jellyfin 29 votes, Finamp 18 reactions, Emby asks | R2 | See CLI-101. | None beyond CLI-101. | Device picker in the bar and player |
| MUS-198 | Phone as remote | See CLI-102, which owns this feature. | Plex yes, even away from home; Jellyfin web can control sessions; Navidrome jukebox control broken for some (#2771, 49 comments) | Medium | R2 | See CLI-102. | None beyond CLI-102. | Remote mode in the player |
| MUS-199 | Headless player | A screenless box on a Pi or a hi-fi that the app controls | Plexamp headless (Pass); Caldera headless; Lyrion with Squeezelite | Medium | Later | A small Rust player built on the core and driven through MUS-197 | None beyond the control channel | Device picker |
| MUS-200 | Choose the output device | See CLI-067, which owns this feature. Music specifics: also applies to headless players (CLI-104, Later). | Spotify requested; Plexamp and Roon yes | Medium: Spotify 5,509 votes | R2 | See CLI-067. | None beyond CLI-067. | Device picker; Settings: sound |
| MUS-201 | Synchronised multi-room | The same music in every room, in sync | Spotify missing; Plex "Tandem Playback" requested; Roon and Lyrion lead | High: Spotify 8,800 votes, Plex 420; hard to build | Later | Built on the handoff object plus clock sync; meanwhile Music Assistant through the adapter (MUS-206) | Clock sync | Device picker groups |
| MUS-202 | Listen together | See VID-153, which owns this feature. Music specifics: friends share one queue and add to it. | Plex requested (76 votes); Navidrome requested (30 reactions); Spotify Jam (only Premium users host) | Medium | Later | See VID-153. | None beyond VID-153. | Share queue sheet |
| MUS-203 | Chromecast | See CLI-106, which owns this feature. Music specifics: send music to Google Cast speakers and Nest devices from Android and the web. | Plex yes; Jellyfin web yes, Finamp requested (31 reactions); Navidrome not planned (#250) | Medium | R2 | See CLI-106. | None beyond CLI-106. | Device picker |
| MUS-204 | AirPlay | See CLI-109, which owns this feature. Music specifics: AirPlay speakers. | Plexamp yes on iOS; Apple Music | Medium | Later | See CLI-109. | None beyond CLI-109. | Device picker |
| MUS-205 | UPnP and DLNA renderers | Play to network receivers | Plex is a DLNA server, acting as a controller requested (190 posts); Navidrome not planned (#305); Symfonium best | Low | Later | Only as a plugin with an explicit grant; an unauthenticated LAN protocol stays out of the core | Plugin hook | Device picker |
| MUS-206 | Sonos and whole-home audio via Music Assistant | Play to Sonos, Cast and AirPlay speakers through Home Assistant | Music Assistant supports Plex, Jellyfin and Subsonic servers | Medium | R2 | Comes through the OpenSubsonic adapter at no extra cost; owned by INT-097 | Adapter | Music Assistant's own interface |
| MUS-207 | Music through the OpenSubsonic adapter | Symfonium, Feishin and other apps work, with lyrics, play reporting, the index-based queue and API keys | Navidrome is the reference | Medium: users stay for clients, as Navidrome's Jellyfin music API shows | R2 | API-key sign-in only (ACC-129); adapter calls feed the same queue, log and lyrics and pass the same authorisation tests (owned by the integrations map) Owned by INT-086; same release. | Adapter | Admin adapter settings |

### Offline and downloads

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| MUS-208 | Instant browsing from the synced library | Every list, page and search answers locally, with no spinner | Plexamp needed a 2026 rebuild to cache its library; Jellyfin 10.11 slow-loading regressions | High: Feishin's top issue is offline (88 votes) | R1 | Delta sync into the core's store on the device (WASM in the browser) | Change log; delta sync; size budgets | All library screens |
| MUS-209 | Full offline mode | Browse, search, queue and play what is on the device with no server | Plexamp since Sept 2026; Finamp offline mode | High: Jellyfin's top request, offline on Android, has 1,820 votes | R2 | Music specifics of CLI-025 and CLI-026. In R1, losing the server leaves browse and search working where the browser allows it (CLI-150); playing with no server needs downloads, which arrive in R2 (CLI-078). Unplayable items are dimmed. | Availability flag per item | All screens; offline indicator |
| MUS-210 | Download albums, artists and playlists | See CLI-078, which owns this feature. | Plexamp (Pass); Finamp free | High: paywall theme across Plex and Emby | R2 | See CLI-078. | None beyond CLI-078. | Download buttons; downloads screen |
| MUS-211 | Download everything | See CLI-079, which owns this feature. Music specifics: a storage estimate first, with Opus as the option that makes it fit. | Plexamp since Sept 2026; Finamp yes | High: Plexamp request 206 votes | R2 | See CLI-079. | None beyond CLI-079. | Downloads screen |
| MUS-212 | Download rules | See CLI-080, which owns this feature. Music specifics: rules use the smart-playlist rule language. | Plexamp "Keep Played Music"; Roon ARC Smart Downloads; Emby's paid offline music criticised (Mar 2026) | Medium | R2 | See CLI-080. | None beyond CLI-080. | Downloads screen rule editor |
| MUS-213 | Smaller downloads as Opus | See CLI-083, which owns this feature. Music specifics: encoded once in the sandbox and cached for the next device (MUS-106). | Finamp yes; Plexamp yes | Medium | R2 | See CLI-083. | None beyond CLI-083. | Download quality setting |
| MUS-214 | Offline actions merge later | See CLI-094, which owns this feature. | Finamp request (#194) | Medium | R2 | See CLI-094. | None beyond CLI-094. | Sync status |
| MUS-215 | Clear download status | See CLI-089, which owns this feature. | Plexamp added specific errors in 4.50 betas | Low | R2 | See CLI-089. | None beyond CLI-089. | Downloads screen |
| MUS-216 | Choose the storage location | See CLI-059, which owns this feature. | Plex requested (31 posts); Plexamp allowed external storage in 2020 | Low | R2 | See CLI-059. | None beyond CLI-059. | Settings: downloads |
| MUS-217 | Downloads in the browser | Save music inside the web client | No rival does this; Jellyfin requests (45 and 7 votes) | Low | Later | Browser storage limits and eviction make it fragile The native download feature is CLI-078 (R2); only the browser part is Later. | None | Web downloads |

### Phone, car, TV and wearables

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| MUS-218 | CarPlay | Browse and play from the car screen | Plexamp free; Finamp beta; official Jellyfin none | High: Finamp 57 reactions; Jellyfin 38 votes | Later | Later; R2 if the App Store licence decision allows (open decision 3 in the feature map README). Owned by CLI-117. Comes with the Apple builds: a native template module fed by the on-device library, free | None | CarPlay browse tree; now playing |
| MUS-219 | Android Auto | The same on Android cars | Plexamp free; Symfonium advanced; Finamp beta | High: car support lags in free clients | R2 | Free; a native media library service fed by the on-device library | None | Android Auto browse tree |
| MUS-220 | Car browse from your home sections, offline | The car shows your own sections and works with no signal | Plexamp offline car browsing (Sept 2026) | Medium | R2 | The home section definitions (MUS-049) become the browse tree, and downloads play with no signal | None | Car browse tree |
| MUS-221 | Voice control | See CLI-074, which owns this feature. | Plexamp Siri; Finamp Siri (beta) | Medium: Plex Google Home request 2,461 votes | Later | See CLI-074. | None beyond CLI-074. | Voice intents |
| MUS-222 | Android Automotive OS | An app built into the car | Plexamp requested, no staff reply | Medium: 119 votes | Later | Reuses the Android Auto media service | None | Car app |
| MUS-223 | Music on the TV | See CLI-041, which owns this feature. | Spotify's 2023 TV redesign; Caldera TV for Apple TV and Android TV | Medium | R2 | See CLI-041. | None beyond CLI-041. | TV player; TV queue |
| MUS-224 | Home-screen widgets | See CLI-073, which owns this feature. | Plexamp Android widget; Finamp requested (19 reactions); Symfonium best | Low | Later | See CLI-073. | None beyond CLI-073. | Widgets |
| MUS-225 | NFC tags | See CLI-075, which owns this feature. | Plexamp yes (free) | Low | Later | See CLI-075. | None beyond CLI-075. | NFC write sheet |
| MUS-226 | Watch apps | Control from the wrist, or play offline on a run | Plex Apple Watch request; Symfonium on Wear OS | Medium: Plex 298 votes | Later | Native watch apps; meanwhile Symfonium on Wear OS through the adapter | None | Watch app |
| MUS-227 | Accessible player | Every player and queue control works with a screen reader and the keyboard | Jellyfin web player controls not accessible (#4504); Plexamp reported unusable with VoiceOver (2025) | Medium | R1 | Accessibility checks gate CI like tests (clients map) | None | All player surfaces |
| MUS-228 | Large-text player for car mounts | Now playing is readable at arm's length | Plexamp supports Dynamic Type, still too small for one user | Low | R2 | Honours the OS text size in the native player | None | Player, large layout |

## Differentiators

1. **A music model that gets credits, editions and discs right from day one
   (MUS-001, MUS-005, MUS-008, MUS-010, MUS-012).** Plex has refused
   multiple artists for a decade (520 votes, plus 866 for robust tags),
   Jellyfin regressed in 10.11 and still splits some multi-disc albums, and
   classical listeners are underserved on both. Navidrome is already good
   here, so this does not beat Navidrome on its own; it beats it in
   combination with Gunmetal's own player, and it is what makes a Plex or
   Jellyfin music user move. The scan report and user-log corrections
   (MUS-007, MUS-035) make wrong guesses visible and fixable.
2. **Gapless and levelled playback everywhere, including the browser
   (MUS-067, MUS-069, MUS-086, MUS-087, MUS-230).** Gapless is Jellyfin's most-voted
   music request (647 votes, open since 2019), Navidrome will not do it in
   its web player, and Emby users rank gapless and ReplayGain their top two
   gaps. Measuring loudness at scan time means levelling works on an
   untagged library, and Auto mode keeps albums intact. Plexamp does this
   well already, but only in its own apps. Two dependencies are honest
   risks: browser gapless needs the audio packager (MUS-230), whose
   per-browser support is unverified, and scan-time measurement needs an
   ADR approving in-process decoders (MUS-086).
3. **A queue you can trust, that follows you (MUS-116, MUS-122, MUS-126 in
   R1; handoff, CLI-101, in R2).** Three clearly separated lanes, picks that
   play in the order chosen, shuffle that is shown before it plays, and
   nothing paywalled, in one versioned server object. Jellyfin can already
   remote-control its sessions; Gunmetal's edge is one versioned queue that
   any device can take over with the same position and lanes,
   offline-tolerant, which Spotify Connect does with an account and Plexamp
   only with plex.tv.
4. **Smart playlists with an editor, evaluated anywhere (MUS-143, MUS-144,
   MUS-145, then MUS-212).** Jellyfin has none (588 votes), Navidrome makes
   you write JSON, and Plexamp's are good. Gunmetal's rule language runs in
   the core on the server and on every device, so the same rules power home
   sections and, in R2, offline download rules.
5. **Lyrics that are simply there, free (MUS-154 to MUS-156, MUS-158).** Plexamp
   charges for lyrics, Plex users want them visible automatically (260
   votes), and Navidrome's .lrc handling has open bug reports. Gunmetal reads
   embedded, LRC and word-timed lyrics at scan time, reports malformed files
   instead of hiding them, and keeps the lyrics view open across tracks.
6. **History you own (MUS-180 to MUS-185, MUS-188).** Loves that cannot be
   lost, personal play counts, a history browsable by date, removal of
   single plays, private sessions and a documented export, all from the
   append-only log. Spotify's requests for these hold thousands of votes
   (5,458 to remove plays, 2,035 to browse history), and no media server we
   checked documents private listening (unverified). Private sessions are
   owned by ACC-117.
7. **Offline without a paywall or a cap (MUS-208 in R1; CLI-078 and
   CLI-079 to CLI-094, which MUS-209 to MUS-215 point to, in R2).** Browsing
   is instant from the synced library in R1; in R2 the native apps work as a
   full player with no server, download by rule, and use Opus to fit.
   Plexamp only removed its six-year download cap in September 2026 and
   still charges for downloads; Emby charges too.

## Deliberately not doing

- **Writing tags into media files (MUS-048).** The library stays read-only.
  Corrections live in the user log and can be exported, so Gunmetal can run
  against read-only mounts and can never damage a carefully tagged
  collection. Dedicated taggers such as MusicBrainz Picard already do this
  well.
- **Beat-matched DJ transitions (MUS-093).** Apple and Spotify do this with
  time-stretching and their own analysis. It is hard to do well, is not
  table stakes for a library player, and album-aware fades (MUS-092) cover
  what most listeners want.
- **Suggestions inserted into a person's own playlists (MUS-130).** A
  playlist someone made only changes when they change it. Suggestions live
  in the labelled "Continue with" lane, which is off by default.
- **Karaoke vocal reduction (MUS-164).** It needs source-separation models,
  which conflicts with the low-hardware goal for a small audience.
- **A streaming catalogue (MUS-178).** Gunmetal plays music people own. Plex
  shows the risk of depending on a partner catalogue: its TIDAL integration
  ended in October 2024. A third party could still write a plugin.
- **Tour dates (MUS-179).** Outside the job of a library player; a plugin
  could add it.
- **A social feed of friends' listening (MUS-191).** Plex's weekly review
  emails, which showed people what friends watched on their own servers,
  drew a 635-post complaint thread. Household features stay inside the
  server and are opt-in per person.
- **Paid tiers.** Lyrics, downloads, the equaliser, radio, sample-rate
  matching and handoff are all in the AGPL build. There is nothing to
  unlock.
- **Telemetry that includes titles or account names.** Plexamp 4.50 sends
  download telemetry with track titles and the account name and offers no
  off switch. Gunmetal sends nothing by default (accounts map).
- **Hosting or bundling a lyrics database.** Lyrics text is copyrighted.
  Gunmetal shows lyrics that are in the user's files, and a user can choose
  to enable a lookup plugin (MUS-162).
- **A DLNA or UPnP server in the core.** It is an unauthenticated LAN
  protocol. If it ever exists, it is a plugin with an explicit grant
  (MUS-205).

## Dependencies and risks

- **Blocking: ADR 3, durable user state.** ADR 1 (decision 5) names watch
  history as the only irreplaceable data, but most rows in this map that
  write user data (playlists, loves, ratings, corrections, layouts, shares,
  identities, IDs, settings) need it to survive a cache rebuild. ADR 3 must
  be accepted before any server work that stores user data. It defines the
  two durable stores every map now names the same way: **the user log** and
  **the identity store** (see the [feature map README](README.md)).
- **Durable user data.** Playlists, loves, ratings, folders, corrections
  and home sections are all user-authored and cannot be rebuilt from files,
  but ADR 1 names watch history as the only irreplaceable data. Every R1
  row that writes to "the user log" depends on a new record extending that
  log (open decision 1). The accounts, discovery and library maps raise the
  same point.
- **Shared rule language.** Smart playlists (MUS-143), home sections
  (MUS-049), rediscovery mixes and download rules (MUS-212) assume one rule
  language in the core, shared with the discovery map's rows and the
  accounts map's restrictions. It needs a stable, versioned serialisation,
  because people keep smart playlists for years.
- **Sync and the on-device store.** Instant browsing (MUS-208), local
  search and offline everything depend on the clients map's delta sync.
  Navidrome had to optimise full-library sync for about a million tracks;
  the budget for artwork and very large libraries on phones and TVs has to
  be measured, and partial sync may be needed.
- **Browser audio.** Gapless and fades in a web page need careful Media
  Source Extensions or Web Audio work. Chrome's guidance mentions roughly a
  12 MB audio buffer limit, Safari's ManagedMediaSource starts only in
  certain conditions, background playback on iPhones is restrictive, and
  the Media Session API is not available everywhere.
- **Decoders for analysis.** Measuring loudness (MUS-086) needs a full
  decode of every file. Symphonia (MPL-2.0) covers FLAC, MP3, AAC and
  Vorbis but lists Opus as in development, and HE-AAC is also missing, so
  some files fall back to tags or to the fallback gain until another
  decoder exists.
- **Scan-time cost on small hardware.** Full decodes of a large library on a
  Raspberry Pi-class server could take many hours; Plex warns of hours or
  days for its own analysis. The job must be background, resumable,
  throttled and visible, and its cost belongs in the scan benchmark rather
  than being assumed.
- **The plugin host.** Scrobbling, online lyrics, artist photos, missing
  albums and listening-service recommendations all wait for the sandboxed
  plugin host with network grants that ADR 2 requires (open decision 5).
- **The sandboxed transcoder.** Opus streams and downloads (MUS-106,
  MUS-213) and formats browsers cannot decode (MUS-033) wait for R2's
  sandbox. The accounts map's per-person music quality cap (ACC-107) is
  marked R1 and assumes Opus in R1; the two maps disagree, which is open
  decision 4.
- **Native players.** libmpv handles gapless natively, but album-aware
  fades probably need a custom mixer with two decoders (unverified for
  libmpv), which is substantial work on every platform. Bit-perfect output
  and sample-rate matching are per-platform native modules.
- **Handoff consistency.** Two devices editing one queue, a device going
  offline mid-session and slow remote links all create conflicts that must
  resolve predictably. The merge rules need writing and testing before the
  device picker promises seamless handoff. Remote handoff waits for iroh.
- **Apple platforms.** Following the video map and the README, iPhone, Apple
  TV, CarPlay, AirPlay and Apple Watch come after R2. CarPlay audio apps
  need Apple's approval (process unverified), and distributing an AGPL app
  through Apple's stores is an unresolved legal question for the clients
  map.
- **Third-party terms.** Lyrics text is copyrighted, so lookup is a
  user-enabled plugin. Last.fm requires an API key per application, which is
  awkward for an open-source project (terms unverified). MusicBrainz allows
  about one request per second per IP. bliss-rs is GPL-3.0, which should
  combine with AGPL-3.0 but needs checking. Headphone preset data has its
  own licence (unverified).
- **Heuristics will sometimes be wrong.** Artist splitting, edition grouping
  and compilation detection all guess. Wrong guesses must be easy to see
  (MUS-035, MUS-044) and to correct (MUS-007), and corrections must survive
  a rebuild.
- **Security.** Share links, playlist sharing and the Subsonic-compatible
  API are where music servers were hurt in 2026: Navidrome published 19
  advisories in ten days of September, including share endpoints without
  ownership checks and shared stream URLs that kept working after deletion.
  The accounts map's route-by-route authorisation tests must cover every
  music endpoint and the adapter.
- **Radio is the weakest point against Plexamp.** People leaving Spotify say
  radio is what they miss most. Metadata and co-listening radio (MUS-165)
  is honest but weaker than Plexamp's sound-based radio until MUS-172
  exists, and single households produce little co-listening data.
- **Trade dress.** The interface may take cues from Spotify but must not
  copy its colours, its round green play button or names such as Connect,
  Jam or Daylist (ADR 2, decision 7).
- **Scope.** The R1 cut in the feature map README is the definition of the
  first release. Of the slip candidates listed here before, MUS-014,
  MUS-078, MUS-121, MUS-159, MUS-166, MUS-167 and MUS-198 moved to R2.
  MUS-019 stays (tags only, cheap) and MUS-140 stays because LIB-192 and the
  R1 importers depend on its parser and matcher.

## Open decisions for the project owner

1. **Where do playlists, loves, ratings, folders, corrections and home
   sections live?** ADR 1 treats SQLite as a rebuildable cache and only
   history as irreplaceable. *Recommendation:* a new record, before the
   server stores any of these, that extends the append-only, exportable log
   to all user-authored data. Without it, "rebuildable cache" quietly stops
   being true.
2. **Default loudness target and mode.** *Recommendation:* Auto mode by
   default, with a reference level that matches ReplayGain 2.0 tags (stated
   in the research as -18 LUFS, unverified this session) so pre-tagged and
   measured files agree and no limiter is needed; offer a louder -14 LUFS
   option with true-peak limiting for noisy places.
3. **Rating model: loves, stars or both?** Imported libraries carry star
   ratings, OpenSubsonic clients expect both (unverified), and Spotify
   showed that changing the love control angers people. *Recommendation:*
   loves shown everywhere by default; five stars with half steps available
   and switched on automatically when ratings are imported; both strictly
   per person.
4. **Is Opus transcoding in R1?** ADR 2 calls it cheap, but the release plan
   says R1 needs no transcoder. *Decided:* R1 stays transcoder-free and Opus
   ships with the R2 sandbox (MUS-106); ACC-107 moved to R2 with it. Until
   then, a slow remote listener gets the original.
5. **When does the plugin host ship?** Scrobbling, online lyrics and artist
   photos all wait for it. *Recommendation:* R2, with first-party Last.fm,
   ListenBrainz, LRCLIB and MusicBrainz plus Cover Art Archive plugins
   maintained by the project. R1 relies on tags and local images and designs
   artist pages to look good without photos. *Decided in the feature map
   README:* R2, for the host (INT-054 to INT-066) and the scrobblers
   (INT-102 to INT-105). Plays recorded in R1 stay in
   the log and can be submitted when the plugins arrive (how far back each
   service accepts old plays is unverified).
6. **Is scan-time loudness measurement in R1?** It costs a full decode of
   every file with a third-party decoder (Symphonia, MPL-2.0, which lacks
   Opus and HE-AAC per the research), and no ADR yet covers in-process
   decoding of untrusted audio. *Recommendation:* yes, as a throttled
   background job that the owner can pause, but only once an ADR approves
   the decoder crate, its licence, fuzzing and resource limits. Without that
   ADR, R1 ships tags plus the MUS-089 fallback gain and MUS-086 moves to
   R2. Publish its cost on low-end hardware in the scan benchmark.
7. **Crossfade and the audio engine.** *Recommendation:* R1 ships gapless
   and levelling only. Crossfade and album-aware fades come in R2 after a
   short spike decides whether native clients can do them in libmpv or need
   a custom two-decoder mixer, so the design is done once for web and
   native.
8. **CUE sheets: R1 or R2?** They are a top request nobody fills (Plex 507
   votes, Jellyfin 171). *Recommendation:* R2, because serving a track as a
   slice of a FLAC file shares the frame index with the video segment map;
   say so publicly so cue users know it is coming.
9. **Media read-only forever?** *Recommendation:* yes. Never write tags or
   ratings back into files; offer exports instead.
10. **Sonic analysis.** *Recommendation:* Later, opt-in, as a server job
    after the scan benchmark shows the cost on low-end hardware, with a
    licence review of bliss-rs before reuse. Metadata and co-listening radio
    ship first.
11. **Online lyrics and legal exposure.** *Recommendation:* ship the LRCLIB
    plugin off by default, enabled per server by the owner, never host or
    bundle lyrics, and cache fetched lyrics only on that server.
12. **Multi-room and listening together.** Demand is large (Spotify 8,800
    votes, Plex 420) but synchronised playback is a project of its own.
    *Recommendation:* Later. The device picker moved to R2 with the native
    clients (CLI-101); point people to Music Assistant through the
    OpenSubsonic adapter in R2.
13. **A streaming catalogue, ever?** *Recommendation:* no. Gunmetal plays
    owned music; discovery beyond the library can only come from plugins
    that the user enables.
14. **Internet radio: music or the live module?** *Recommendation:* the R3
    M3U module owns station lists, and stations play in the music player
    with the normal now-playing bar.
