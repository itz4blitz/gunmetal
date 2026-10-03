# Music library and playback

## Scope

This file covers everything between a folder of audio files and a person
listening to it: the music data model (artists, album artists, release
groups, editions, compilations, classical works), tag handling and its edge
cases, playlists and smart playlists, the play queue, gapless playback,
crossfade and transitions, loudness normalisation, lossless, high-resolution
and bit-perfect output, equalisers, lyrics, radio and mixes, sonic
similarity, scrobbling, ratings and favourites, listening history and
statistics, the sleep timer, offline downloads, device handoff and car
integration.

It does not cover sign-in, remote access transport, search ranking, video,
podcasts or audiobooks, except where a music feature depends on them.

Products studied: Plex with Plexamp, Jellyfin with Finamp and Symfonium,
Navidrome, Emby, Spotify, Apple Music, Tidal, Roon, Lyrion Music Server and
foobar2000.

How this was researched (2026-10-02): web search and direct page fetches,
the public JSON APIs of the Jellyfin feature board and the Plex forum (for
vote counts), and the GitHub API (for issue reactions and release dates).
The shared web-search budget ran out part way through, so later facts come
from pages fetched directly from known primary URLs. Anything not confirmed
from a source in this session is marked "(unverified)". Vote and reaction
counts were read on 2026-10-02 and will drift.

Architecture constraints that shape the recommendations below (from
`docs/adr`): the server and clients are both ours; a pure-Rust core parses
tags, artwork, gapless metadata and loudness at scan time; SQLite is a
rebuildable cache and history lives in an append-only, exportable log; the
library is synced to devices so browsing works offline; scrobbling and
metadata lookups are plugins with explicit network grants; OpenSubsonic and
Jellyfin APIs are optional adapters; audio transcoding to Opus is acceptable
because it is cheap.

## Feature inventory

### Rivals at a glance (October 2026)

- **Plex with Plexamp.** Plexamp has been free to use since v4.8.0 (July
  2023), but a long list of its best features still needs Plex Pass:
  downloads, lyrics, the ten-band equaliser, sample-rate matching, track and
  album radio, sonic similarity, Guest DJ and headless Plexamp, according to
  the Plexamp product page. Plex raised the lifetime Pass from US$249.99 to
  US$749.99 on 1 July 2026; monthly and annual prices did not change in that
  announcement. Remote streaming of music stayed free when Plex put remote
  video behind a paywall in April 2025. Plexamp went more than a year
  without a release until v4.50.3 on 1 September 2026, a rebuild onto Tauri
  (desktop) and Expo (mobile) that removed the three-day download limit,
  added "Keep Played Music", offline Home and Library, on-device loudness
  analysis, and dropped the Sonic Sage AI playlist feature. A forum post
  quoting Plexamp's creator says he is no longer at Plex as of 2026; he now
  also ships a separate player family called Caldera Music (TV, desktop and
  headless).
- **Jellyfin.** Jellyfin 12.0 shipped on 7 September 2026 (12.1 followed on
  15 September).
  For music it added ListenBrainz as a recommendation source, reads album
  ReplayGain, stores playlist items as rows (duplicates now allowed) and made
  music lookups faster. Jellyfin 10.11 added word-level lyric timing. Its
  own apps are thin for music; most people use Finamp (free, open source;
  the redesign is still beta, 1.0.1-beta on 2026-09-02), Symfonium
  (Android, paid) or Feishin (desktop).
- **Navidrome.** The most active self-hosted music server: five feature
  releases (0.60 to 0.64) plus patch releases between February and
  September 2026. It added a WebAssembly
  plugin system (0.60), server-side transcode decisions (0.61), the
  OpenSubsonic sonic-similarity and playback-report extensions (0.62),
  word-synced TTML/ELRC lyrics (0.63) and an experimental Jellyfin music API
  so Finamp and Jellify can connect to it (0.64). It also fixed a long list
  of authorisation bugs along the way. Its built-in web player is basic; the
  ecosystem relies on OpenSubsonic clients.
- **Emby.** Emby 4.6 rewrote music scanning to be tag-driven and supports
  multiple MusicBrainz IDs and composers, but a December 2025 thread on its
  own forum lists gapless playback, ReplayGain, a persistent queue, release
  type grouping and ratings as still missing. Because of that, and because
  people choosing a music server rarely shortlist it, the inventory below
  replaces the Emby column with Navidrome and mentions Emby in the Notes
  column where there is evidence.
- **Spotify, Apple Music, Tidal.** The bar for "replace a streaming app".
  Spotify added lossless (up to 24-bit/44.1 kHz FLAC) for Premium on
  2025-09-10, DJ-style transitions in its Mix feature (August 2025) and
  Smart Reorder by tempo and key (February 2026). Apple Music added AutoMix
  (beat-matched transitions), lyric translation and pronunciation, and
  playlist import from other services in iOS 26. Tidal offers hi-res FLAC
  up to 24/192 on all plans after dropping MQA (from a third-party review,
  unverified against Tidal's own pages).
- **Roon.** The audiophile reference: signal path, DSP, R128 levelling with
  an automatic track/album mode, a classical "Compositions" browser, Roon
  Radio and the ARC mobile app with CarPlay and Android Auto. US$14.99 a
  month, US$12.49 a month billed yearly or US$829.99 lifetime.
- **Lyrion Music Server.** The open-source successor to Logitech Media
  Server: multi-room synchronised playback, works/performances/roles for
  classical (9.0, November 2024), playlist folders and play-count sorts
  (9.1, February 2026).
- **foobar2000.** The power-user reference on the desktop: gapless,
  ReplayGain scanning with BS.1770, WASAPI exclusive output, a DSP chain,
  query-based autoplaylists, playback statistics, a built-in UPnP renderer
  and server, and no telemetry. Version 2.26 shipped on 2026-10-01.

### How to read the tables

Columns are Plex (meaning Plex Media Server with Plexamp unless stated),
Jellyfin (server plus its official apps; Finamp and Symfonium noted where
they matter) and Navidrome (server, its web UI and what it exposes to
OpenSubsonic clients). "Pass" means the feature needs a paid Plex Pass.
"Client" means the server exposes it but only third-party clients use it.
"Plugin" means it needs a server plugin. "Unverified" means it was not
confirmed from a source in this session. Vote counts are from the vendor's
own request board unless stated.

### Library data model

| Feature | What the user gets | Plex | Jellyfin | Navidrome | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Multiple artists per track | A collaboration appears under every credited artist, not under a fake "A & B" artist | No. One artist per track; request open since 2015 with 520 votes | Partial. Reads multiple artist tags, but 10.11 randomises which ones are kept (issue #15283) and stopped splitting "feat." (#14622) | Yes since March 2025 (#238, 51 reactions) | Spotify, which links every credited artist; Navidrome among self-hosted | The Plex request names Spotify as the model to copy |
| Album artist separate from track artist | Albums stay together under one artist even when tracks have guests | Yes | Yes | Yes; every track needs an album artist tag | Navidrome documents the rules clearly | Emby's guide says album and album artist must match on every track or the album splits |
| Display credit kept as written while each artist stays linked | "Artist A feat. Artist B" reads naturally and both names are clickable | No | Unverified | Yes. The single-value tag is the display name, the multi-value tag gives the links | Navidrome's split between display string and linked artists | Mirrors MusicBrainz artist credits with join phrases |
| Artist roles (composer, conductor, lyricist, producer, remixer, performer) | Browse "everything conducted by X" or "everything X produced" | Partial; "By Composer" request open since 2013 (89 votes) | Unverified | Composer since 2025 (#275); other roles (unverified) | Roon (rich credits) and Lyrion 9.0, which also allows custom roles | Credits drive classical and soundtrack browsing |
| Artists with the same name | Two different bands called the same thing stay separate | Weak; a 2026 Plexamp forum post says these "get messed up almost always" | Unverified | By MusicBrainz ID when tagged (unverified) | Libraries tagged with MusicBrainz artist IDs, used by Roon and Lyrion (unverified) | Only solvable with stable IDs; names alone cannot do it |
| Manual artist merge, split and aliases | Fix "Beatles" vs "The Beatles" without retagging | Unverified | No (unverified) | Requested: manual artist grouping (#2138, 9 reactions) | No clear leader | Emby users asked for merge/split in December 2025 |
| Release groups and editions | The deluxe, the remaster and the original appear as versions of one album, not as duplicates or a merged mess | Weak; request open since 2015 (93 votes) | No; multi-disc albums can even split into separate albums (#5605, open, 71 comments) | Yes (#489 and #1976 completed in 2025) | Navidrome among self-hosted; Roon lets you pick a primary version (unverified) | MusicBrainz models a release group (the album idea) above releases (things you can buy) |
| Release types (album, EP, single, live, compilation, soundtrack, remix, demo, DJ mix) | Artist page split into Albums, EPs, Singles, Live and so on | Yes, "group albums by type"; manual override requested (72 votes) | No; request has 42 votes | Yes since October 2025 (#369) | Plexamp's artist page, plus Apple Music | MusicBrainz has 5 primary and 12 secondary types; Emby users asked for this too |
| Compilations and Various Artists | A compilation stays one album and each contributor can still find their track | Partial; "Brahms from a Various Artists compilation" request has 81 votes | Partial | Yes. Uses the compilation flag and makes every track artist browsable (#211) | Navidrome | Tags: TCMP (ID3, an iTunes extension), cpil (MP4), COMPILATION (Vorbis) |
| "Appears on" section | An artist page lists albums where they only guest | Yes in Plexamp | Unverified | Yes via multi-artist support (unverified) | Spotify | Plex users say "Appears on" is the closest Plex gets to multi-artist support |
| Multi-disc albums and disc subtitles | Disc names like "Live in Tokyo" shown; discs stay in one album | Partial; non-numeric disc labels requested (67 votes) | Bug: multi-disc albums shown as separate albums (#5605) | Yes; per-disc artwork since 0.61 | Lyrion 9.0 (disc subtitles) and Navidrome | Emby users asked to play a single disc of a multi-disc set |
| Original date vs release date | A 2011 remaster still sorts under 1969 | Partial (unverified) | Partial (unverified) | Supported (unverified) | Unverified | Tags: TDOR/originaldate vs TDRC/date |
| Classical: works and movements | Movements group under the work; browse by composition | No | Planned: "Alternate tabs view for classical music" (34 votes) | Unverified | Roon's Compositions view (now filterable in ARC 1.77) and Lyrion 9.0 Works | Tags: WORK, MOVEMENTNAME (MVNM in ID3, ©mvn in MP4), MOVEMENTTOTAL |
| Classical: performances | Compare different recordings of the same work | No | No | Unverified | Lyrion 9.0 (Performances) and Roon | Needs a work identity shared across albums |
| Genres: multi-valued, per track | A track can be both "Jazz" and "Soul" | Genres, styles and moods mostly per album from Plex metadata | Multi-valued (unverified per track) | Multi-valued; index-backed genre filtering in 0.64 | Plexamp, which adds styles and moods | Jellyfin "genre, artist, album hierarchy" request has 17 votes; Emby users asked for sub-genres |
| Moods, styles, record labels, grouping | More ways into a big library | Yes: moods, styles and a record label section | Unverified | Label filter; browse by grouping requested (#2045) | Plexamp | Moods also seed Plexamp stations |
| Sort names and natural sort | "The Beatles" files under B; "Part 2" before "Part 10" | Yes | Partial; sorting an artist's albums by year requested (49 votes) | Yes; natural sort added in 0.64 | Navidrome | Tags: ARTISTSORT, ALBUMARTISTSORT (TSO2 in ID3) |
| Artist images, biographies and external metadata | Artist pages look finished | Yes, from Plex's metadata service | Yes, from MusicBrainz and TheAudioDB; wrong matches happen (Ogg Vorbis files showed photos of a keyboardist named Ogg, #9406, 204 reactions) | Yes, from external agents; custom upload since 0.61 | Plex (curated) and Roon | ADR 2 puts these lookups in plugins with network grants |
| Missing albums | See which studio albums by an artist you do not own | Unverified | No | Requested (#5106, 12 reactions) | Roon, which shows streaming albums next to owned ones (unverified) | Needs MusicBrainz discography lookups |
| Multiple libraries with per-user access | A separate library for kids or for a partner's music | Yes | Yes; artist view leaked albums across libraries (#17215) | Yes since July 2025 (#192, 106 reactions) | Plex | Per-library access also affects playlists and shares |
| Folder view | Browse the files as they sit on disk | Unverified | Yes | Via Subsonic folder endpoints (bug #5232) | foobar2000 | Finamp users asked for folder browsing (#661) |
| Music videos linked to artists | Watch an artist's videos from their page | Library supports them; Plexamp does not (forum request, 2026) | Separate library; combining requested (38 votes) | No | Apple Music and Spotify | Probably outside the music v1 |

### Tags, scanning and files

| Feature | What the user gets | Plex | Jellyfin | Navidrome | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Format coverage | FLAC, MP3, AAC and ALAC in MP4, Ogg Vorbis, Opus, WAV, AIFF, WavPack, APE play | Broad (transcodes what a client cannot play) | Broad (FFmpeg) | Broad (taglib plus FFmpeg for transcoding) | foobar2000, with decoder components | Gunmetal's first core milestone is FLAC, MP3, MP4 and Ogg |
| Multi-value tags across ID3v2.3, ID3v2.4, Vorbis, MP4 and APE | Multiple artists and genres survive whatever tagger was used | Weak | Partial | Yes; documents which formats can hold true multi-values | Navidrome docs and MusicBrainz Picard | ID3v2.3 has no official multi-value support, so separators are the only option there |
| Separator splitting with exceptions | "AC/DC" and "Tyler, The Creator" are not split into two artists | No | Regression: "feat." no longer split (#14622) | Yes; `Scanner.ArtistSplitExceptions` added in 0.63 | Navidrome | Splitting rules are a common source of wrong artists |
| MusicBrainz IDs as identity | Retagged files keep their play counts; same-name artists stay apart | Partial (unverified) | Reads MBIDs; uses MusicBrainz as a provider | Uses MBIDs for grouping (unverified) | Picard-tagged libraries in Roon or Lyrion (unverified) | Release group, release, recording and artist IDs are all available from Picard |
| Tags win over online matches | Your carefully tagged metadata is not silently replaced | Unverified | Providers sometimes fetched even when disabled (#5778, 18 reactions) | Tags first by design | Navidrome and foobar2000 | ADR 2 makes lookups opt-in plugins |
| Embedded and folder artwork | Covers from the file, `cover.jpg` or `folder.jpg`; per-disc art | Yes | Yes; using embedded art per track requested (29 and 14 votes) | Yes; per-disc art and animated covers (0.61), blurred placeholders while loading (0.64) | Navidrome | Gunmetal's core reads embedded art at scan time |
| CUE sheets and single-file albums | A one-file rip with a .cue plays as separate tracks | No; request has 507 votes | No (unverified) | Not planned (#2136) | foobar2000, which plays CUE natively | FLAC can also carry an embedded CUESHEET block |
| Ratings stored in tags | Years of ratings from another player are not lost | No; import requested (41 votes) | Unverified | Unverified | foobar2000 (unverified) | ID3 uses POPM keyed by email with inconsistent scales; Vorbis uses RATING; Picard lists no MP4 field |
| Tag editing from the app | Fix a typo without leaving the player | No; requested for Plexamp (17 votes) | Edits saved to its database (unverified) | No (unverified) | foobar2000 and Picard | Writing to files conflicts with read-only media mounts |
| Fast, incremental scans | New albums appear in seconds; rescans do not take hours | Yes | 10.11 brought performance regressions (#15685, 89 reactions) and "database locked" reports (#15101, 229 comments) | Fast; per-item "Refresh Metadata" in 0.64 | Navidrome | ADR 1 makes scan time the first benchmark |
| Technical details: codec, sample rate, bit depth, bitrate | See and filter "24/96 FLAC" | Codec badges; filtering by sample rate requested (10 posts) | Requested (17 votes each for codec and bitrate display) | Requested (#1438) | Roon (signal path), Plexamp (badges) | Cheap to read at scan time |
| BPM and musical key | Sort or build mixes by tempo | BPM requested (22 votes) | No | Stores BPM (#5747) | Spotify Smart Reorder, which computes tempo and key itself | Useful later for transitions |
| Grouping tag ambiguity (TIT1 vs GRP1) | Grouping and Work do not swap after an iTunes-style retag | Unverified | Unverified | Unverified | Lyrion 9.1 added an option to treat TIT1 as grouping or work | Picard maps grouping to TIT1 or GRP1 depending on settings |
| Duplicate detection | Find the same track ripped twice | Unverified | Requested (15 votes) | Discussion: show only the highest-quality version | No clear leader | Needs acoustic or MBID matching |

### Playlists and smart playlists

| Feature | What the user gets | Plex | Jellyfin | Navidrome | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Manual playlists | Create, reorder, remove tracks | Yes | Yes; 12.0 stores items as rows and allows duplicates | Yes | Spotify | Plex "Better Playlists" (1,425 votes) is mostly about video but asks for sorting and sharing |
| Playlist folders | Hundreds of playlists stay organised | No; requested (43 votes) | No | No; discussion has 32 upvotes | Spotify and Lyrion 9.1 (hierarchical folders) | Small feature, frequent request |
| Playlist artwork | Automatic mosaic or a custom image | Yes; custom posters implemented | Playlist images settable in 12.0 | Yes; upload since 2026 (#406, 49 reactions) | Spotify | |
| Sort and search inside a playlist | Find a song in a 2,000-track playlist; sort by date added | Requested (183 votes) | Dynamic sorting requested (23 votes) | Sortable table in the web UI (unverified) | Spotify | |
| Import and export (M3U, M3U8) | Bring playlists from another player and take them away | Unverified | Creating and editing M3U requested (17 votes) | Auto-imports M3U from the library; faster import in 0.64 | Navidrome | Path matching across machines is the hard part |
| Import from streaming services | Bring Spotify playlists to your own library | No (unverified) | Requested (29 votes) | No | Apple Music, which imports from Spotify, YouTube Music, Tidal, Deezer and Amazon in iOS 26 | Needs fuzzy matching against the local library |
| Shared and collaborative playlists | Family members add to one playlist | Sharing to home users; editing shared playlists requested | Protecting playlists from non-owners requested (19 votes) | Public playlists; collaborative not planned (#1523) | Spotify (collaborative playlists plus Jam) | Needs per-object permissions |
| Smart playlists | Rule-based playlists that update themselves | Yes, built from library filters | No native support; 588 votes, marked planned; community plugin exists | Yes, as hand-edited `.nsp` JSON files with no editor in the UI | foobar2000 autoplaylists (full query language); iTunes-style editors on Apple's desktop app (unverified) | Navidrome's smart playlist issue drew 232 comments before it shipped |
| Smart playlist rule fields | Rules on play count, last played, rating, date added, loudness, missing tags, membership of other playlists | Filters; playlist-membership rules requested (101 votes) | Plugin only | 80+ fields, `inPlaylist`, `isMissing`/`isPresent`, ReplayGain fields (0.62), album fields (0.64) | Navidrome | |
| Smart playlist limits and order | "50 random unplayed tracks" or "top 5 percent by plays" | Limit and sort | Plugin only | Limits, percentage limits (0.61), random sort | Navidrome | |
| Smart playlist refresh policy | Live, or a stable daily or weekly snapshot | Live | Plugin schedule | Per-playlist `refreshDelay` (0.64) | Navidrome | A stable snapshot matters for downloads |
| Smart playlists as download sources | An auto-updating offline mix | Yes: downloaded library filters (fixed in 4.50.20 betas) | Client | Client (Symfonium) | Plexamp 2026 | |
| Favourites as a playlist | "Liked Songs" you can shuffle and download | Via a rating filter | Requested (25 votes; Finamp #133) | Starred view | Spotify Liked Songs | |
| Exclude from shuffle or radio | Keep the Christmas album out of everyday shuffle | Requested (44 votes) | Unverified | No | Spotify (hide song) and Apple Music (suggest less) | |
| Playlist favourites and ratings | Pin the playlists you use most | Unverified | Unverified | Yes in 0.64 | Navidrome | |

### Play queue

| Feature | What the user gets | Plex | Jellyfin | Navidrome | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Two-tier queue: "play next" vs the rest of the album or playlist | Queue a song without losing your place in an album | Yes | Basic; Finamp's redesign added a "Next Up" queue | Play next exists; "play after current album" requested (#627) | Spotify ("Next in queue" above "Next from") | |
| See and edit the whole queue | Reorder, remove, clear upcoming | Requested: show and edit entire queue (42 votes) | Finamp cannot reorder while shuffled (README) | Yes in the web UI | Spotify and foobar2000 | |
| Persistent queue | The queue survives an app restart or a phone reboot | Yes; play queues live on the server | "Resume last music playlist" requested (13 votes); Finamp has "Past Queues" | `savePlayQueue`; remembering position requested (#245) | Plexamp | Emby users asked for this in December 2025 |
| Queue shared across devices | Start on the phone, continue on the desktop at the same track | Yes, through server-side play queues | No | `getPlayQueue` and the OpenSubsonic index-based queue | Spotify Connect | |
| Shuffle modes | Track shuffle, album shuffle, "shuffle the rest" | Shuffle; shuffling the current queue requested (26 posts) | Album shuffle requested (22 votes) | Track shuffle (album shuffle unverified) | Spotify, which also lets you switch its Smart Shuffle mode off (toggle date unverified) | |
| Repeat and stop-after | Repeat one or all; stop after this track or album | Repeat one and all (2020) | Remembering repeat and shuffle requested (11 votes) | Unverified | foobar2000 (stop after current) | |
| Autoplay when the queue ends | Music keeps going with similar tracks | Pass | Unverified | Client | Spotify | |
| Several saved queues | Keep a workout queue and a work queue | "Past queues" style history (unverified) | Finamp "Past Queues" | No | Symfonium (multiple now-playing queues) | |

### Gapless, crossfade and transitions

| Feature | What the user gets | Plex | Jellyfin | Navidrome | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Gapless playback in native apps | Live albums and DJ mixes play without clicks or silence | Yes, free in Plexamp | Not across official apps; request open since 2019 with 647 votes; Finamp and Symfonium are gapless | Web UI: not planned (#745); OpenSubsonic clients do it | foobar2000 and Plexamp | Emby users list it as missing (December 2025) |
| Gapless in the browser | The same, in a web tab | Requested for Chrome (30 votes; current state unverified) | A 2020 web issue was closed as completed, but the board request stays open; support varies by client (unverified) | No | No mainstream web player does this well (unverified) | Possible with Media Source Extensions append windows, per Chrome's guidance |
| Gapless for lossy files and transcodes | MP3, AAC and Opus, including on-the-fly transcodes, stay gapless | Plexamp spent a 2025 release making Opus downloads and conversions gapless | Unverified | MP3 transcodes carry no duration or Xing header (#6170) | Plexamp | Needs LAME/Xing delay and padding, iTunSMPB and Opus pre-skip |
| Crossfade with a fixed length | Songs overlap by N seconds | Sweet Fades (free) | No; request has 138 votes | Client | Spotify (user-set duration) and Apple Music | |
| Album-aware crossfade | Fades between unrelated songs but never inside a continuous album | Sweet Fades, using loudness data | No | Client | Plexamp; Symfonium's Smart Fades; Apple's AutoMix skips transitions within an album | |
| Beat-matched DJ transitions | Tempo- and key-matched blends | No | No | No | Apple Music AutoMix (time stretching and beat matching); Spotify Mix transitions and Smart Reorder (Premium) | Hard to do well; not table stakes for a library player |
| Fades on pause, skip and resume | No pops when pausing or after a Bluetooth disconnect | Yes; Android now fades back in after a Bluetooth pause (4.50.19, September 2026) | Unverified | Playback resumes unexpectedly after Bluetooth standby (#5446) | Plexamp | |

### Loudness normalisation

| Feature | What the user gets | Plex | Jellyfin | Navidrome | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Read ReplayGain tags (track and album) | Existing scans from foobar2000 or dBpoweramp are honoured | Unclear: Plexamp uses its own loudness analysis; a June 2026 forum post asks it to honour ReplayGain tags | Yes since 10.9; album gain read only since 12.0 | Yes; passed to clients | foobar2000 | Emby has none (December 2025 thread) |
| Analysis without pre-tagged files | Normalisation works on an untagged library | Yes; server analysis, plus on-device analysis since September 2026 | Server-side analysis (unverified) | No (unverified); relies on tags | Roon, which measures everything with EBU R128 | ADR 2 says loudness is read at scan time |
| Track, album and automatic modes | Albums keep their internal dynamics; shuffles stay even | Unverified | Unverified | Client | Roon's Auto mode: album gain for adjacent tracks of one album, track gain otherwise; Spotify applies album gain when you play a whole album | |
| Target level and clipping protection | Pick a target; nothing distorts | Never boosts a track (2023 change) | Unverified | Client | Spotify: Loud (-11), Normal (-14) and Quiet (-19) LUFS with a limiter on Loud; Roon targets -14 LUFS and uses true peak | ReplayGain 2.0 uses a -18 LUFS reference (unverified this session) |
| Fallback for unanalysed tracks | A new, unmeasured track is not suddenly louder | On-device analysis fills gaps | Unverified | Unverified | Roon applies -5 dB to tracks with unknown loudness | |
| Opus gain fields | Opus files play at the right level | Unverified | Unverified | Unverified | Unverified | RFC 7845: header output gain always applies; R128_TRACK_GAIN and R128_ALBUM_GAIN add to it |
| Dynamic range compression ("night mode") | Quiet passages audible at low volume | Requested for video (134 votes) | Unverified | No | Spotify's Loud setting with limiter | Mostly a video and car request |

### Output quality, high resolution and equaliser

| Feature | What the user gets | Plex | Jellyfin | Navidrome | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Lossless streaming of the original | FLAC arrives as FLAC | Yes | Yes (direct play) | Yes | Every self-hosted server; among streaming services, Tidal and Apple Music go to 24/192, Spotify to 24/44.1 | Apple notes Bluetooth is never lossless |
| High-resolution playback | 24/96 and 24/192 play at full resolution | Requests for hi-res support (31 votes) and passthrough (49 votes) | Depends on client | Depends on client | Roon and Symfonium | Apple: most devices need an external DAC above 48 kHz |
| Sample-rate matching | Output switches to the file's rate instead of resampling | Pass; macOS and iOS since 2021, strict mode 2022, Android 2024 | Client | Client | Roon and Symfonium | |
| Bit-perfect and exclusive output | Nothing else touches the samples on the way to the DAC | Android bit-perfect with sample-rate matching (2024); Windows exclusive mode in Caldera (forum report, unverified) | Client | Client | Roon (RAAT, end to end); Symfonium (Android 14+ USB bit-perfect); foobar2000 (WASAPI exclusive) | Android exposes a bit-perfect mixer mode from API level 34 |
| Signal path display | See every stage that changed the audio | No | No | No | Roon | Makes "bit-perfect" claims checkable |
| DSD (native or DoP) | DSF and DFF files play | Unverified | Transcoded (unverified) | Transcoded (unverified) | Symfonium and Roon | Niche; likely out of v1 |
| Multichannel and Dolby Atmos music | Surround mixes | Spatial audio requested (120 posts) | Unverified | No | Apple Music | Caldera's TV app advertises surround (unverified detail) |
| Graphic and parametric equaliser | Shape the sound per headphone or speaker | Pass: ten-band EQ with preamp and per-output presets | No; request has 40 votes | No (web) | Roon (parametric plus convolution); Symfonium (parametric, graphic and AutoEQ) | Finamp EQ requested (#453) |
| Headphone correction presets | Pick your headphone model, get a correction curve | ~6,000 headphone presets (4.50.20 beta, September 2026) | No | No | Plexamp and Symfonium (AutoEQ) | |
| Headroom, crossfeed, convolution | Room correction and headphone crossfeed | No | No | No | Roon | Roon warns some DSP is CPU-heavy |
| Per-output settings | Different EQ and gain for car, headphones and DAC | Per-output EQ presets | No | No | Symfonium (per-device output settings) | |
| Visualisers | Something to look at on a big screen | Yes | No | No | Plexamp; Caldera's TV app | A request for MilkDrop-style visuals has 27 posts |

### Lyrics

| Feature | What the user gets | Plex | Jellyfin | Navidrome | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Embedded lyrics | Lyrics stored in the file show up | Partial: local lyrics display exists, but "read embedded lyrics" is still requested (49 votes) | Yes since 10.9 (request had 550 votes) | Yes | Navidrome | Tags: USLT and SYLT (ID3), LYRICS (Vorbis), ©lyr (MP4) |
| Synced sidecar lyrics (.lrc) | Lines highlight in time | Unverified | Yes; auto-scroll when timed (48 votes, done) | Yes since 2025 (#1421, 56 reactions); reports of .lrc not showing (#4148, #5531) | Navidrome 0.63 | Navidrome 0.64 also handles the LRC `[bg:]` tag |
| Word-by-word and multi-voice lyrics | Karaoke-style highlighting; duet parts shown separately | Unverified | Word-level cues since 10.11; TTML requested (16 votes) | Yes: TTML, ELRC, SRT, YAML and LRC with word timing and voices (0.63) | Apple Music | Navidrome helped write the OpenSubsonic lyrics extension for this |
| Online lyrics lookup | Lyrics appear for files that have none | Pass | Plugin (unverified) | Lyrics-provider plugins (0.61) | Spotify and Apple Music (licensed catalogues) | LRCLIB is a free, keyless lyrics API whose server is open source (Rust and SQLite) |
| Lyrics always on screen | The lyrics view stays open from track to track | Requested (260 votes) | Unverified | Unverified | Apple Music and Spotify | Plexamp keeps local lyrics on screen between tracks (2023) |
| Translation and pronunciation | Understand and sing along to other languages | No | No | No | Apple Music (iOS 26) | Needs a translation service; plugin territory |
| Lyrics offline | Lyrics for downloaded songs without a connection | Unverified | Unverified | Client | Apple Music (iOS 26) | Gunmetal's synced library makes this natural |
| Search by lyric | Find a song from a line you remember | Unverified | No | No | Apple Music (unverified); AudioMuse-AI offers lyrics search as an add-on | |

### Radio, mixes, discovery and sonic similarity

| Feature | What the user gets | Plex | Jellyfin | Navidrome | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Library and artist radio | Endless play from your own library around a seed | Yes | Instant Mix (basis unverified); ListenBrainz similar artists in 12.0 | Instant Mix from Last.fm, Deezer or plugins (out of beta in 0.60) | Plexamp (runs on your library, works offline) | |
| Track and album radio | "More like this song" or "albums like this one" | Pass, using sonic analysis | No; sonic analysis requested (98 votes, 2025) | Through sonic-similarity plugins (0.62) | Plexamp | |
| Sonically similar artists, albums and tracks | Related items chosen by sound, not tags | Pass; needs server sonic analysis | No | Via plugin, for example AudioMuse-AI | Plexamp | Plex says analysis can take hours or days and needs a 64-bit server |
| Sonic path between two songs | A journey from one track to another | Pass (Sonic Adventure) | No | `findSonicPath` via plugin | Plexamp | AudioMuse-AI calls this Song Paths |
| Mixes built from your history | Fresh mixes of what you play most, plus related music | Pass (Mixes for You) | ListenBrainz recommendations (12.0) | No (unverified) | Spotify (Daily Mix, Discover Weekly) and Roon (Daily Mixes) | |
| Queue "DJs" | A helper that keeps inserting fitting tracks | Pass: Guest DJ (for example same-artist and same-era variants) | No | No | Plexamp | |
| Rediscovery modes | Forgotten favourites, "this time last year" | Pass (Aural Fixations) | No | No | Plexamp; Roon ARC's Smart Downloads pick "forgotten gems" | |
| Mix builder from several seeds | "Mix of these four artists" | Pass | No | No | Plexamp | |
| Text or mood prompts | "Rainy Sunday jazz" makes a playlist | Sonic Sage removed in 2026 | No | Via plugins (AudioMuse-AI text search) | Apple's Playlist Playground beta (iOS 26.4, unverified) and Spotify (unverified) | |
| Discovery beyond your library | Radio blends in music you do not own | Inserted TIDAL tracks (DJ Doppelgänger, 2024; current status unverified) | No | No | Roon Radio (mixes in Tidal and Qobuz) | Gunmetal has no catalogue; only via plugins |
| Tuning recommendations | Tell it "less of this"; keep a playlist out of your taste | Ratings feed some features (unverified) | No | No | Spotify (taste profile controls; unverified details) | |
| Internet radio stations | Play stream URLs alongside the library | Requested and closed (77 posts) | Unverified | Yes, admin-managed stations | Lyrion (plugins) and foobar2000 (Radio Browser) | Overlaps with the M3U module |

### Ratings, favourites, history, statistics and scrobbling

| Feature | What the user gets | Plex | Jellyfin | Navidrome | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Favourites | Heart a track, album or artist | Yes | Yes; favourites lost during playback (#14981, 25 reactions) | Yes (star) | Spotify | |
| Star ratings | Rate 1 to 5 stars | Yes; half stars requested (24 votes) | Favourites only (unverified) | Yes, per user | foobar2000 (unverified) | Emby users asked for ratings |
| Ratings that change playback | Skip one-star tracks; weight radio by rating | Requested (23 votes) | No | Discussion (6 upvotes) | No clear leader | Plexamp has like/dislike buttons for Android Auto |
| Per-user play counts and dates | Play count, last played, date added | Yes | Yes | Yes | foobar2000 (playback statistics) | |
| Import play history | Bring years of Last.fm history | No | No | Requested (28 upvotes) | No native leader | Last.fm and ListenBrainz both export |
| Listening history view | See what you played and when | Recent plays | Plugin (Playback Reporting) | Scrobble history in the native API (0.64) | Last.fm and ListenBrainz | |
| Charts and statistics | Top artists, albums, tracks this week or month | Yes: weekly and monthly charts | Plugin | Unverified | Spotify Wrapped and Apple Replay (yearly) | ADR 1 keeps history in an exportable log |
| Admin "now playing" | Who is listening to what, right now | Yes (dashboard) | Yes (dashboard) | Redesigned now-playing panel (0.62) | Plex | |
| Last.fm scrobbling | Plays sent to Last.fm | Unverified | Plugin; a native request is still open (27 votes) | Built in | Navidrome; Symfonium scrobbles from the client | Last.fm rule: tracks over 30 s, after half the track or 4 minutes |
| ListenBrainz scrobbling | Plays sent to ListenBrainz | Unverified | Plugin | Built in | Navidrome | ListenBrainz uses the same half-or-4-minutes rule |
| Offline plays reported later, in order | Plays from the plane still count | Unverified | Finamp: syncing offline play counts requested (#194) | Client | Unverified | Last.fm asks clients to cache and send old scrobbles first, up to 50 per batch |
| Per-user scrobble controls | Do not scrobble the kids' account or one library | Unverified | Unverified | Per-user filtering (0.64) | Navidrome | |
| Love sync to Last.fm | Hearting a song also loves it on Last.fm | Unverified | Unverified | Not planned (#1547, 17 reactions) | Unverified | |

### Offline, downloads and data use

| Feature | What the user gets | Plex | Jellyfin | Navidrome | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Download albums and playlists | Music on the phone for the plane | Pass | Finamp yes; official app reportedly no (XDA, March 2026) | Client | Spotify and Plexamp | Spotify caps downloads at 10,000 tracks on each of 5 devices and requires going online every 30 days |
| Download the whole library or a discography | Everything offline, not one album at a time | Since September 2026 (three-day limit removed) | Finamp yes | Client (Symfonium) | Finamp and Plexamp | The Plex request had 206 votes before it closed |
| Smaller copies for downloads | Download as Opus or AAC to save space | Yes | Finamp yes | Transcoded downloads (#573) | Finamp | ADR 2: Opus transcoding is cheap enough for any server |
| Automatic cache of recently played | Recent music stays available offline | "Keep Played Music" (September 2026) | Unverified | Client | Roon ARC Smart Downloads (rotates weekly) and Symfonium | |
| Browse the whole library offline | Search and browse even what is not downloaded | Offline Home, Library and search (September 2026) | Unverified | Client (Symfonium mirrors the library) | Symfonium | Navidrome 0.63 made full-library sync 20 times faster on about one million tracks |
| Data saver and cellular rules | Never touch mobile data; stream originals on Wi-Fi | Forum user reports surprise mobile data use (June 2026) | Finamp requests: disable transcoding on Wi-Fi (20 reactions), transcode on mobile only (7) | Client | Spotify (separate quality settings, auto-adjust) | |
| Edit playlists offline | Changes made offline sync later | Unverified | Finamp request (#1065) | Client | Spotify (unverified) | |
| Choose storage location | Put downloads on the SD card | Requested (31 posts); Plexamp allowed external storage in 2020 | Unverified | Client | Plexamp | |

### Devices: handoff, casting, multi-room and remote control

| Feature | What the user gets | Plex | Jellyfin | Navidrome | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Move playback between devices | Tap a speaker or laptop; music continues there with the same queue and position | Yes, through Plexamp players and casting | Requested "like Spotify Connect" (29 votes); Finamp "Play On" requested (18 reactions) | No | Spotify Connect | Spotify Connect pauses longer than ten minutes may need a reconnect |
| Remote control another player | Use the phone as a remote for the living-room player | Yes, even off the home network (2022) | "Play On" remote control in the web client | Jukebox mode controls server-side output; control broken for some (#2771, 49 comments) | Spotify | |
| Headless player for a Pi or hi-fi | A screenless box that the app controls | Pass (headless Plexamp); Caldera headless daemon | Unverified | Jukebox on the server's own sound card | Lyrion with Squeezelite | |
| Chromecast | Send music to a Cast speaker | Yes | Yes in the web client; Finamp requested (31 reactions) | Not planned (#250) | Symfonium | |
| AirPlay | Send music to AirPlay speakers | Yes on iOS | Unverified | Client | Apple Music | |
| UPnP/DLNA | Play to network renderers | Plex is a DLNA server; acting as a control point requested (190 posts) | Plugin (unverified) | Not planned (#305) | Symfonium (gapless UPnP) and foobar2000 (built-in renderer and server) | |
| Sonos | Play to Sonos | Yes | Unverified | Client | Symfonium (group, ungroup, volume) | |
| Synchronised multi-room | Same music in every room, in sync | Requested "Tandem Playback" (420 votes) | Unverified | No; the jukebox plays on one server output only | Roon and Lyrion | |
| Listen together remotely | Friends hear the same queue and can add to it | Requested (76 votes, closed) | Unverified | Shared listening requested (#1551, 30 reactions) | Spotify Jam (host controls, remote needs Premium) | |
| Share a link | Send a friend an album or track | Yes; shared tracks now highlight on their album (4.50.20 beta) | Unverified | Sharing on by default since 0.63 | Spotify | Share links are a security surface (see pain points) |
| Desktop app with media keys and global shortcuts | Control music from anywhere on the desktop | Yes; now Tauri-based, smaller and lighter | Feishin and others | Web UI; Feishin | foobar2000 | |
| Music on the TV | Artwork-led music player on the big screen | No native TV Plexamp; Caldera TV for Apple TV and Android TV | Official TV apps play music (unverified quality) | Client | Unverified | Plexamp forum users ask for Google TV support |

### Car, lock screen, wearables and small comforts

| Feature | What the user gets | Plex | Jellyfin | Navidrome | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Lock-screen, notification and Bluetooth metadata | Title, art and controls everywhere | Yes | Yes (unverified quality) | Web via the Media Session API; playback state inaccurate (#4744) | Spotify | Media Session is not supported in every browser |
| Android Auto | Browse and play in the car | Yes, free | Conflicting: board marks it completed, XDA (March 2026) says the official app has none; Finamp beta and Symfonium have it | Client | Spotify and Symfonium | |
| CarPlay | The same for iPhone | Yes | No; requested (38 votes); Finamp beta (#24 had 57 reactions) | Client | Spotify and Apple Music; Roon ARC has it too | |
| Offline in the car | Car screens browse downloads with no signal | Yes (September 2026) | Unverified | Client | Plexamp | |
| Android Automotive OS | An app built into the car itself | Requested (119 votes) | No | No | Spotify (unverified) | |
| Tesla and other in-car browsers | Play in the car's own browser | Requested (22 votes) | No | No | Unverified | |
| Wear OS and Apple Watch | Control or play from the wrist | Requested (21 and 141 posts) | No | No | Symfonium on Wear OS | |
| Widgets and quick actions | Home-screen controls | Android widget implemented | Finamp widget requested (19 reactions) | Client | Symfonium (adaptive widgets) | |
| Voice assistants | "Play my running playlist" | Siri | Unverified | No | Spotify (unverified) | A Plex Google Home request has 346 posts |
| Sleep timer | Stop after N minutes, this track or this album | Yes, including "sleep after album" | Unverified | Unverified | Plexamp | |
| Physical triggers (NFC tags) | Tap a tag or a record sleeve to start an album | Yes | No | No | Plexamp | |
| Screen reader access | VoiceOver and TalkBack can reach every control | Fixed in 4.50.20 beta (September 2026) | Unverified | Unverified | Unverified | |
| Large text and car-mount layout | Readable now-playing at arm's length | Dynamic Type support; forum user still finds text too small on a car mount | Unverified | Unverified | Unverified | |

## Pain points and unmet demand

Counts are votes on the vendor's request board (Plex forum, Jellyfin
feature board), reactions on GitHub issues, or upvotes on GitHub
discussions, as read on 2026-10-02.

1. **Plex cannot model music with more than one artist, after a decade of
   asking.** "Better support for albums and tracks with multiple artists"
   has 520 votes and 315 posts since 2015, and "Tag support for robust
   music library organization" has 866 votes. A Plexamp forum regular wrote
   in June 2026 that the community should give up because Plex has said it
   would need a rewrite (second-hand, unverified).
   - https://forums.plex.tv/t/better-support-for-albums-and-tracks-with-multiple-artists/116658
   - https://forums.plex.tv/t/tag-support-for-robust-music-library-organization/106326
   - https://forums.plex.tv/t/plex-amp-what-new-features-would-folks-like/939677
2. **Jellyfin's multi-artist handling regressed.** 10.11 keeps a random
   subset of artist tags and drops the album artist (#15283) and no longer
   splits "feat." (#14622). Multi-disc albums can show as separate albums
   (#5605, 19 reactions, 71 comments, open since 2021).
   - https://github.com/jellyfin/jellyfin/issues/15283
   - https://github.com/jellyfin/jellyfin/issues/14622
   - https://github.com/jellyfin/jellyfin/issues/5605
3. **Gapless is still missing in many places.** Jellyfin's "Gapless
   Playback" request is the board's top music item at 647 votes and 34
   comments, open since 2019. Emby users list gapless and ReplayGain as
   their top two music gaps (December 2025). Navidrome's web player will not
   do it (#745, not planned). Plex's request for gapless in Chrome has 30
   votes.
   - https://features.jellyfin.org/posts/181/gapless-playback
   - https://emby.media/community/topic/144851-unfinished-music-library-business
   - https://github.com/navidrome/navidrome/issues/745
   - https://forums.plex.tv/t/gapless-audio-playback-chrome-web-browser/352032
4. **Smart playlists are either missing or hard to use.** Jellyfin's request
   has 588 votes and is marked planned but not shipped; users rely on a
   community plugin. Navidrome's smart playlists drew 232 comments before
   shipping and still have no editor: you write JSON files by hand. Plex
   users want playlist-membership rules (101 votes).
   - https://features.jellyfin.org/posts/49/add-support-for-dynamic-smart-playlists
   - https://github.com/navidrome/navidrome/issues/1417
   - https://www.navidrome.org/docs/usage/features/smart-playlists/
   - https://forums.plex.tv/t/playlist-presence-filtering-criteria/209068
5. **Lyrics: wanted everywhere, paywalled on Plex, fragile elsewhere.**
   Jellyfin's lyrics request reached 550 votes before shipping; Plex's
   "Lyrics always visible" has 260 votes and "Read embedded lyrics" 49;
   Plexamp lyrics need Plex Pass. Jellyfin users now ask for TTML (16) and
   the Lyricsfile format (16). Navidrome has open reports of `.lrc` files
   not showing (#4148, 14 reactions; #5531).
   - https://features.jellyfin.org/posts/285/add-lyrics-to-songs
   - https://forums.plex.tv/t/lyrics-always-visible-when-song-starts/134519
   - https://forums.plex.tv/t/read-embedded-music-lyrics-from-audio-files/152897
   - https://features.jellyfin.org/posts/3742/support-ttml-lyrics
   - https://github.com/navidrome/navidrome/issues/4148
6. **Single-file albums with CUE sheets are unsupported.** Plex: 507 votes.
   Navidrome: not planned.
   - https://forums.plex.tv/t/cue-support-for-flac-files/96352
   - https://github.com/navidrome/navidrome/issues/2136
7. **A Spotify Connect equivalent does not exist for self-hosted music.**
   Jellyfin "Cast Jellyfin music the way Spotify Connect does it" (29
   votes); Finamp "Play On" compatibility (18 reactions) and Cast support
   (31 reactions); Navidrome declined Chromecast (31 reactions) and UPnP
   (29). Emby users ask for a queue that survives switching devices.
   - https://features.jellyfin.org/posts/519/cast-jellyfin-music-the-way-spotify-connect-does-it-for-spotify
   - https://github.com/finamp-app/finamp/issues/616
   - https://github.com/finamp-app/finamp/issues/50
   - https://github.com/navidrome/navidrome/issues/250
8. **Multi-room and listening together.** Plex "Tandem Playback" has 420
   votes (it covers video too); Plex "Listen Together for Music" 76;
   Navidrome "Shared listening" 30 reactions and social features 25.
   - https://forums.plex.tv/t/tandem-playback-to-several-clients/38777
   - https://forums.plex.tv/t/listen-together-for-music/597875
   - https://github.com/navidrome/navidrome/issues/1551
9. **Car support lags for free clients.** Finamp's CarPlay issue has 57
   reactions; Jellyfin's CarPlay request 38 votes; Plex users want Android
   Automotive OS (119 votes). XDA wrote in March 2026 that the official
   Jellyfin app has no Android Auto, CarPlay or offline downloads.
   - https://github.com/finamp-app/finamp/issues/24
   - https://features.jellyfin.org/posts/744/carplay-support
   - https://forums.plex.tv/t/adapt-plexamp-for-android-automotive-os-not-just-android-auto/649618
   - https://www.xda-developers.com/single-music-app-plex-jellyfin-navidrome/
10. **Crossfade and EQ are missing from Jellyfin.** Crossfade 138 votes,
    equaliser 40, both open. Plexamp has both but its EQ needs Plex Pass.
    - https://features.jellyfin.org/posts/140/audio-crossfading-between-tracks
    - https://features.jellyfin.org/posts/736/add-a-music-equalizer
11. **Sonic similarity is wanted but expensive.** Jellyfin users asked for
    Plexamp-style sonic analysis in 2025 (98 votes). Plex's version needs
    Plex Pass and, by Plex's own description, can take hours or days of
    heavy CPU. The main open alternative, AudioMuse-AI, asks for at least a
    four-core CPU with AVX2, 8 GB of RAM and an NVMe disk.
    - https://features.jellyfin.org/posts/3245/add-sonic-analysis-support-for-music-library-similar-to-plexamp
    - https://support.plex.tv/articles/sonic-analysis-music/
    - https://github.com/NeptuneHub/AudioMuse-AI
12. **Release types, versions and discs.** Jellyfin "Group music releases by
    type" 42 votes; Plex "Music Release Type manual edit" 72; Plex "Better
    support for multiple versions of music albums" 93; Plex "Non-numerical
    labels for discs" 67.
    - https://features.jellyfin.org/posts/2220/group-music-releases-by-type-album-single-live-compilation-etc
    - https://forums.plex.tv/t/music-release-type-category-manual-edit/762258
    - https://forums.plex.tv/t/better-support-for-multiple-versions-of-music-albums/126609
    - https://forums.plex.tv/t/non-numerical-labels-for-discs/296828
13. **Classical music is an afterthought in the video servers.** Plex "By
    Composer" (89 votes, open since 2013) and "Brahms from a Various Artists
    compilation" (81); Jellyfin "Alternate tabs view for classical music"
    (34, planned). A 2026 Plexamp forum post says film-score fans must drop
    all but the first composer.
    - https://forums.plex.tv/t/by-composer/27760
    - https://forums.plex.tv/t/is-it-really-not-possible-to-have-brahms-show-as-an-artist-from-a-various-artists-compilation/125733
    - https://features.jellyfin.org/posts/772/alternate-tabs-view-for-classical-music
14. **Playlists are clumsy.** Plex playlist sorting and search (183 votes),
    playlist folders on Plex (43) and Navidrome (32 upvotes, the most-upvoted
    Navidrome idea in the sample), editing the full queue on Plex (42),
    excluding items from shuffle (44).
    - https://forums.plex.tv/t/playlist-sorting-and-searching-feature-request/116089
    - https://forums.plex.tv/t/can-you-add-playlist-folders-to-plex-or-for-plex-pass-users/430389
    - https://github.com/navidrome/navidrome/discussions/3371
    - https://forums.plex.tv/t/feature-request-display-edit-entire-play-queue/707828
    - https://forums.plex.tv/t/ability-to-exclude-media-items-from-shuffle-modes/357933
15. **Ratings are second-class.** Plex: import ratings from ID3 tags (41
    votes), half stars (24), skip one-star songs (23). Navidrome: skip
    one-star tracks (6 upvotes), rating column in playlists (13 reactions).
    Emby users asked for any rating system.
    - https://forums.plex.tv/t/feature-request-import-music-mp3-ratings-from-files-id3-tags/151767
    - https://forums.plex.tv/t/options-toggle-to-always-ignore-skip-one-star-rated-songs/812254
    - https://github.com/navidrome/navidrome/issues/1473
16. **History is locked in.** Navidrome users ask to pull play counts from
    Last.fm (28 upvotes); Finamp users ask for offline plays to sync later
    (19 reactions). Year-in-review statistics exist only in streaming apps.
    - https://github.com/navidrome/navidrome/discussions/3454
    - https://github.com/finamp-app/finamp/issues/194
17. **Price and stewardship of the best self-hosted player.** Plex's
    lifetime pass tripled to US$749.99 on 1 July 2026, and lyrics,
    downloads, EQ and sample-rate matching in Plexamp still need it.
    Plexamp's creator posted that a Let's Encrypt root change forced the
    September 2026 release and that the previous release was over a year
    earlier; a forum post quotes him as "no longer with Plex" as of 2026.
    One user put it bluntly: "Plex has pretty much abandoned Plexamp"
    (draakko, Plex forum, June 2026). Roon costs US$14.99 a month or
    US$829.99 lifetime.
    - https://rottenwifi.com/plex-pricing-changes-explained-what-the-749-99-lifetime-pass-means-in-2026/
    - https://www.plex.tv/plexamp/
    - https://forums.plex.tv/t/plexamp-v4-50-3-ready-or-not/942338
    - https://forums.plex.tv/t/plex-amp-what-new-features-would-folks-like/939677
    - https://roon.app/en/pricing
18. **Mobile data and loudness trust.** A Plexamp user reports occasional
    surprise mobile data use despite only playing downloads and asks that
    the app fail rather than touch cellular data; another is unsure whether
    Plexamp honours their ReplayGain values (both June 2026). Finamp users
    want originals on Wi-Fi and transcodes only on mobile data (20 and 7
    reactions).
    - https://forums.plex.tv/t/plex-amp-what-new-features-would-folks-like/939677
    - https://github.com/finamp-app/finamp/issues/44
    - https://github.com/finamp-app/finamp/issues/1283
19. **Security bugs in music servers are common.** Navidrome 0.62 fixed
    cross-account share disclosure, player and share IDORs, a Last.fm
    session hijack and a share-token expiry bypass; 0.63 fixed per-library
    access leaks in playlist import and shares; 0.64 fixed SQL injection,
    IDOR and SSRF issues; 0.64.1 patched five more. This is a well-run
    project, which is the point: the attack surface is real.
    - https://github.com/navidrome/navidrome/releases/tag/v0.62.0
    - https://github.com/navidrome/navidrome/releases/tag/v0.63.0
    - https://github.com/navidrome/navidrome/releases/tag/v0.64.0
    - https://freedom.tech/posts/2026-09-21-navidrome-0-64-1/
20. **Scanner and database regressions erode trust.** Jellyfin 10.11
    performance problems (#15685, 89 reactions), "database locked" on some
    storage (#15101, 229 comments), and a metadata provider attaching the
    wrong artist photos to every Ogg Vorbis file (#9406, 204 reactions).
    - https://github.com/jellyfin/jellyfin/issues/15685
    - https://github.com/jellyfin/jellyfin/issues/15101
    - https://github.com/jellyfin/jellyfin/issues/9406
21. **Demand next door (out of this domain).** Podcasts in Navidrome (43
    reactions on the API issue, 23 upvotes on the RSS idea), LDAP and SSO
    (148 and 65), and audiobooks in Plex (over 1,000 posts). Noted so the
    feature map can decide where they live.
    - https://github.com/navidrome/navidrome/issues/793
    - https://github.com/navidrome/navidrome/issues/141

Where rivals are already good, and Gunmetal should not pretend otherwise:
Plexamp's player (Sweet Fades, loudness levelling, sonic radio, Guest DJs,
car support) is the best self-hosted listening experience; Navidrome's tag
handling, lyrics and pace of releases are excellent; Symfonium is the most
complete Android client and works with every server; Roon owns audiophile
DSP and classical browsing; Lyrion owns synchronised multi-room;
foobar2000 owns power-user control; Spotify Connect and Apple Music's lyrics
set the bar for handoff and lyrics.

## Where Gunmetal can be clearly better

Each idea names the pain it answers and what would have to be true
technically. All of them fit the existing records: scan-time parsing in the
Rust core, the device-synced library, the append-only history log, plugins
with network grants and the optional OpenSubsonic and Jellyfin adapters.

1. **Credits done properly from the first release** (pain 1, 2, 13).
   Store each track and album credit as an ordered list of artist, role and
   join phrase, with the display string kept exactly as tagged. Identify
   artists by MusicBrainz ID when present and by normalised name within the
   album-artist context otherwise. Splitting rules come with exceptions and a
   scan report that lists every ambiguous split.
   - Technically: the core must return every value of a multi-valued field
     for ID3v2.3 (separator), ID3v2.4 (null-separated), Vorbis comments
     (repeated fields), MP4 freeform atoms and APE. The splitter must be
     deterministic and covered by mutation tests. The schema needs a credits
     table, not an `artist` text column.
2. **Release groups, editions and release types** (pain 2, 12).
   Group releases under a release group, let each user pick a preferred
   edition, and split artist pages by primary and secondary release type
   from RELEASETYPE and MusicBrainz tags, with manual overrides.
   - Technically: a grouping-key precedence (release group ID, then release
     ID, then normalised album artist plus title with edition suffixes
     stripped) that errs on the side of not merging. Overrides cannot live
     only in SQLite, because ADR 1 treats SQLite as a rebuildable cache (see
     open questions).
3. **Classical as a view of the same model** (pain 13). Works and movements
   from WORK, MOVEMENTNAME/MVNM/©mvn and composer credits; a work page lists
   every performance across albums, in the spirit of Lyrion 9 and Roon.
   - Technically: a setting for the TIT1/GRP1 grouping ambiguity, as Lyrion
     added in 9.1, and a work identity keyed by MusicBrainz work ID or by
     composer plus normalised work title.
4. **Gapless on every client, including the browser** (pain 3). The core
   reads encoder delay and padding at scan time (LAME/Xing, iTunSMPB, Opus
   pre-skip; FLAC is exact) and sends trim values with each track. Native
   clients use libmpv's gapless path; the web client uses Media Source
   Extensions append windows and timestamp offsets, as Chrome's guidance
   describes, or Web Audio scheduling. Opus transcodes keep their pre-skip,
   which Plexamp found worth a whole release to get right.
   - Technically: priming and padding sample counts in the protocol;
     sample-accurate test fixtures that check the join between tracks; a
     plan for iOS Safari, which needs ManagedMediaSource and has tight
     buffer limits.
5. **Loudness that is right without pre-tagged files** (pain 3, 18).
   Measure integrated loudness and true peak (ITU BS.1770 / EBU R128) at
   scan time in Rust, honour existing ReplayGain 2.0 and R128 tags, offer
   Roon-style Auto (album gain for adjacent tracks from one album, track
   gain otherwise), and never push a track into clipping: use only the
   true-peak headroom, or a limiter as Spotify does on its Loud setting.
   Show the gain applied in the player.
   - Technically: full decode of every file once, in the background,
     resumable and throttled. That needs pure-Rust decoders for every
     supported codec; Symphonia (MPL-2.0) covers FLAC, MP3, AAC and Vorbis
     but lists Opus as in development, so Opus needs another decoder or a
     sandboxed worker.
6. **Smart playlists with an editor, evaluated anywhere** (pain 4, 14,
   15). One rule language in the Rust core, compiled into the server and
   the clients, so a smart playlist evaluates identically online and
   offline. A visual editor, live or snapshot refresh, limits and
   percentages, rating- and history-driven rules, and smart playlists as
   download sources. Import Navidrome `.nsp` files as an onramp.
   - Technically: a typed rule syntax tree with stable field names; per-user
     ratings and play history available on the device through the synced
     library.
7. **Offline first, without limits** (pain 9, 16, 18). The synced library
   (ADR 1) makes browsing and search work offline for everything, not just
   downloads. Downloads by rule: a smart playlist, "keep the last N played",
   or the whole library as Opus. A strict cellular policy that refuses to
   use mobile data when told. Plays, ratings and playlist edits made offline
   are appended locally and merged when back online.
   - Technically: a compact delta sync that scales to libraries of about a
     million tracks (Navidrome had to optimise for this); server-side Opus
     download jobs; storage accounting; merge rules for logs written on
     several devices.
8. **A queue that follows you, with no cloud account** (pain 7, 8). The
   queue is a server object with a version number. Any signed-in device can
   become the player, take over with the same position, or act as a remote,
   at home or over iroh. Synchronised multi-room can build on the same
   object later.
   - Technically: optimistic concurrency on the queue, a small control
     protocol, and clock sync if multi-room follows. Offline devices keep a
     local copy and reconcile.
9. **Free, local-first, word-synced lyrics** (pain 5). Read embedded
   lyrics (USLT, SYLT, LYRICS, ©lyr) and sidecar LRC, Enhanced LRC and
   TTML, with word timing and separate voices; keep the lyrics view open
   across tracks; work offline because lyrics travel with the synced
   library. Online lookup (for example LRCLIB, which is free, keyless and
   open source) is a plugin with an explicit network grant. Serve the same
   data through the OpenSubsonic lyrics extension.
   - Technically: LRC, Enhanced LRC and TTML parsers in the core, with
     malformed input returning typed errors like every other parser.
10. **Sonic similarity on modest hardware** (pain 11). Compute a compact
    feature vector (tempo, timbre, chroma and loudness, in the style of
    bliss-rs) during the same decode pass as loudness, so files are decoded
    only once. Use it for track radio, similar albums and a "sonic path",
    and expose it through the OpenSubsonic `sonicSimilarity` extension.
    Heavier models (CLAP, as AudioMuse-AI uses) stay optional plugins.
    - Technically: bliss-rs reports roughly an hour for 10,000 files on its
      test machine; the budget must be measured on Gunmetal's low-end
      target. bliss-rs is GPL-3.0, which should combine with AGPL-3.0 but
      needs a licence check before reuse.
11. **No paywall, and nothing behind a vendor account** (pain 17).
    Lyrics, downloads, EQ, headphone presets, sample-rate matching, radio
    and similarity are all part of the AGPL product. The roadmap already
    says no central account.
12. **Honest output: bit-perfect when asked, and a visible signal path**
    (Plex hi-res requests, Roon's lead). Native clients can choose exclusive
    output (WASAPI exclusive, ALSA hardware devices, CoreAudio, and
    Android's bit-perfect mixer from API level 34). The player shows every
    stage that changed the audio (normalisation, EQ, resampling) and shows a
    bit-perfect badge only when nothing did.
    - Technically: per-platform audio output configuration in the native
      player modules, and a React Native bridge to Android's mixer
      attributes.
13. **Listening history you own** (pain 16). Every play goes into the
    append-only, exportable log that ADR 1 already requires. Weekly,
    monthly and yearly statistics are computed locally. Last.fm and
    ListenBrainz scrobbling are plugins that follow the half-or-four-minutes
    rule and send cached plays oldest first; importing Last.fm or
    ListenBrainz history is a plugin too.
    - Technically: a listen event schema (track and MusicBrainz IDs, start
      time, time played, device, the context it was played from) with
      de-duplication across devices.
14. **CUE sheets and single-file albums** (pain 6). Parse `.cue` files and
    FLAC CUESHEET blocks at scan time into virtual tracks with
    sample-accurate boundaries. Gapless playback across them is then free.
    - Technically: the server must serve a track as a slice of a file. For
      FLAC that needs a frame index built at scan time (the music analogue
      of ADR 1's segment map) and, for clients that cannot seek inside a
      file, an in-process slice that costs no transcode.
15. **Security that holds under scrutiny** (pain 19). Random IDs,
    per-object authorisation, short-lived signed stream URLs (ADR 1) and
    expiring share tokens address the exact classes of bug Navidrome fixed
    in 2026. Plugins that fetch artwork, lyrics or metadata get explicit
    network grants, which limits SSRF.
    - Technically: authorisation tests for every endpoint, including the
      OpenSubsonic and Jellyfin adapters, written test-first like the rest.
16. **Compatibility as the onramp** (pain 7, 9). Navidrome 0.64 showed that
    a non-Jellyfin server can host Finamp and Jellify through a Jellyfin
    music API. Gunmetal's OpenSubsonic and Jellyfin adapters can give users
    Symfonium, Feishin and Finamp on day one, including car support, while
    Gunmetal's own clients mature.
    - Technically: implement the OpenSubsonic extensions clients now rely on
      (song lyrics, playback report, index-based queue, transcoding, sonic
      similarity, API-key authentication).

## Risks and hard parts

- **Browser audio.** Gapless and crossfade in a web page need careful MSE
  or Web Audio work. Chrome's guidance mentions roughly a 12 MB audio
  buffer limit; Safari's ManagedMediaSource only starts in certain
  conditions; background playback on iPhones is restrictive; the Media
  Session API is not available everywhere and Navidrome has an open bug
  where its reported state drifts.
- **Crossfade in native players.** libmpv handles gapless playback, but
  album-aware crossfades like Sweet Fades probably need a custom mixer with
  two decoders (unverified for libmpv). That is a substantial piece of work
  on every platform.
- **Scan-time decoding cost.** Loudness and sonic features need a full
  decode of every file. On a Raspberry Pi-class server and a large library
  this could take many hours; Plex warns of hours or days for its analysis.
  The work must be background, resumable, throttled and visible, and the
  low-specs claim must be benchmarked, not assumed.
- **Pure-Rust decoder coverage.** Symphonia lacks finished Opus and HE-AAC
  support. Analysis of those files needs another decoder or the sandboxed
  FFmpeg worker, which ADR 1 reserves for transcodes.
- **Tag heuristics will sometimes be wrong.** Edition grouping, artist
  splitting and compilation detection all guess. Wrong guesses must be easy
  to see and override, and overrides must survive a database rebuild.
- **User data versus the cache.** ADR 1 says watch history is the only
  irreplaceable data, but ratings, favourites, playlists, smart playlist
  rules and metadata overrides cannot be rebuilt from files either.
- **Licensing and third-party terms.** Lyrics text is copyrighted, so
  online lookup must be a user-enabled plugin. Last.fm needs an API key per
  application, which is awkward for an open-source project (terms
  unverified). MusicBrainz enforces rate limits (unverified). bliss-rs is
  GPL-3.0. Headphone EQ preset data has its own licence (unverified).
- **Handoff consistency.** Two devices editing one queue, devices going
  offline mid-session, and latency over remote links all create conflicts
  that must resolve predictably.
- **Scale of sync.** A million-track library synced to a phone needs a
  compact format and incremental updates; Navidrome only recently fixed
  duplicate and missing pages in full-library sync.
- **Bit-perfect versus DSP.** Normalisation and EQ change samples, so they
  cannot coexist with bit-perfect output. The UI has to make that trade-off
  obvious.
- **Car platforms.** CarPlay and Android Auto require platform templates
  and, for CarPlay, an entitlement from Apple (unverified). React Native
  support for both is uneven (unverified).
- **Scope.** The rivals' combined feature list is enormous. The first music
  release has to pick, and the feature map should mark what is deferred.

## Open questions

1. What is the default loudness target and mode: -14 LUFS like Roon and
   Spotify, or the -18 LUFS ReplayGain 2.0 reference; Auto, track or album?
2. Should ratings, favourites, playlists, smart playlist rules and metadata
   overrides join listening history in the append-only log, which would
   change the wording of ADR 1?
3. Does Gunmetal ever write tags back to files (ratings, play counts,
   fixes), or is the media library always read-only?
4. Is a MusicBrainz lookup plugin enabled by default, and does a
   MusicBrainz ID in a tag always beat heuristics?
5. Where does sonic analysis run (server, a plugin worker, or devices), and
   which algorithm and licence are acceptable?
6. Is crossfade (and album-aware crossfade) in the first music release, and
   does it justify a custom audio engine instead of plain libmpv?
7. Are CUE sheets, DSD and multichannel music in the first release?
8. Does internet radio belong to music or to the M3U and live TV module?
9. Which OpenSubsonic extensions must the adapter support at launch, and
   does the Jellyfin adapter cover music clients such as Finamp from the
   start?
10. Is synchronised multi-room playback a goal, and if so, when?
11. Are ratings and play counts strictly per user, and how do family
    members share playlists?
12. Should the smart playlist language be compatible with Navidrome's
    `.nsp` format, or only importable from it?
13. Do music videos belong in the music library or the video side?
14. Podcasts and audiobooks: still "later", given visible demand?

## Sources

Plex and Plexamp
- https://www.plex.tv/plexamp/
- https://support.plex.tv/articles/sonic-analysis-music/
- https://thedesk.net/2026/09/plexamp-new-app-desktop-windows-mac-linux/
- https://forums.plex.tv/t/plexamp-release-notes/221280
- https://forums.plex.tv/t/plexamp-v4-50-3-ready-or-not/942338
- https://forums.plex.tv/t/plex-amp-what-new-features-would-folks-like/939677
- https://forums.plex.tv/t/caldera-music-release-notes/939620
- https://caldera.homes/music/
- https://www.thurrott.com/music-videos/318677/plex-announces-plex-pass-price-increase-and-makes-remote-access-a-paid-feature
- https://rottenwifi.com/plex-pricing-changes-explained-what-the-749-99-lifetime-pass-means-in-2026/
- https://forums.plex.tv/t/better-support-for-albums-and-tracks-with-multiple-artists/116658
- https://forums.plex.tv/t/tag-support-for-robust-music-library-organization/106326
- https://forums.plex.tv/t/cue-support-for-flac-files/96352
- https://forums.plex.tv/t/better-playlists/73590
- https://forums.plex.tv/t/lyrics-always-visible-when-song-starts/134519
- https://forums.plex.tv/t/read-embedded-music-lyrics-from-audio-files/152897
- https://forums.plex.tv/t/download-entire-library-or-playlist-in-plexamp/646153
- https://forums.plex.tv/t/adapt-plexamp-for-android-automotive-os-not-just-android-auto/649618
- https://forums.plex.tv/t/better-support-for-multiple-versions-of-music-albums/126609
- https://forums.plex.tv/t/by-composer/27760
- https://forums.plex.tv/t/is-it-really-not-possible-to-have-brahms-show-as-an-artist-from-a-various-artists-compilation/125733
- https://forums.plex.tv/t/plexamp-support-for-hi-res-music/729212
- https://forums.plex.tv/t/req-hi-res-audio-please-bring-back-audio-passthrough-support-for-24bit-96-192khz-etc/700026
- https://forums.plex.tv/t/options-toggle-to-always-ignore-skip-one-star-rated-songs/812254
- https://forums.plex.tv/t/plex-player-app-for-tesla/463191
- https://forums.plex.tv/t/10-star-ratings-half-stars/42059
- https://forums.plex.tv/t/tandem-playback-to-several-clients/38777
- https://forums.plex.tv/t/gapless-audio-playback-chrome-web-browser/352032
- https://forums.plex.tv/t/music-release-type-category-manual-edit/762258
- https://forums.plex.tv/t/playlist-sorting-and-searching-feature-request/116089
- https://forums.plex.tv/t/feature-request-display-edit-entire-play-queue/707828
- https://forums.plex.tv/t/ability-to-exclude-media-items-from-shuffle-modes/357933
- https://forums.plex.tv/t/feature-request-import-music-mp3-ratings-from-files-id3-tags/151767
- https://forums.plex.tv/t/bpm-metadata/213684
- https://forums.plex.tv/t/basic-tag-editor-within-plexamp/716704
- https://forums.plex.tv/t/can-you-add-playlist-folders-to-plex-or-for-plex-pass-users/430389
- https://forums.plex.tv/t/listen-together-for-music/597875
- https://forums.plex.tv/t/playlist-presence-filtering-criteria/209068
- https://forums.plex.tv/t/non-numerical-labels-for-discs/296828
- https://forums.plex.tv/t/feature-request-plexamp-seamless-downloads/718064
- https://forums.plex.tv/t/add-support-for-dynamic-audio-range-compression/21701
- https://forums.plex.tv/search.json (Discourse search API, used for vote counts)

Jellyfin, Finamp, Symfonium
- https://jellyfin.org/posts/jellyfin-release-10.11.0/
- https://jellyfin.org/posts/jellyfin-release-12.0/
- https://features.jellyfin.org/api/v1/posts (feature board API, used for vote counts)
- https://features.jellyfin.org/posts/181/gapless-playback
- https://features.jellyfin.org/posts/49/add-support-for-dynamic-smart-playlists
- https://features.jellyfin.org/posts/683/smart-playlists
- https://features.jellyfin.org/posts/285/add-lyrics-to-songs
- https://features.jellyfin.org/posts/140/audio-crossfading-between-tracks
- https://features.jellyfin.org/posts/3245/add-sonic-analysis-support-for-music-library-similar-to-plexamp
- https://features.jellyfin.org/posts/2220/group-music-releases-by-type-album-single-live-compilation-etc
- https://features.jellyfin.org/posts/736/add-a-music-equalizer
- https://features.jellyfin.org/posts/744/carplay-support
- https://features.jellyfin.org/posts/772/alternate-tabs-view-for-classical-music
- https://features.jellyfin.org/posts/519/cast-jellyfin-music-the-way-spotify-connect-does-it-for-spotify
- https://features.jellyfin.org/posts/1654/replaygain-support-in-media-player
- https://features.jellyfin.org/posts/3742/support-ttml-lyrics
- https://github.com/jellyfin/jellyfin/issues/15283
- https://github.com/jellyfin/jellyfin/issues/14622
- https://github.com/jellyfin/jellyfin/issues/5605
- https://github.com/jellyfin/jellyfin/issues/9406
- https://github.com/jellyfin/jellyfin/issues/14981
- https://github.com/jellyfin/jellyfin/issues/15685
- https://github.com/jellyfin/jellyfin/issues/15101
- https://github.com/jellyfin/jellyfin/issues/5778
- https://github.com/jellyfin/jellyfin/issues/17215
- https://github.com/jellyfin/jellyfin-web/issues/1132
- https://jellywatch.app/blog/jellyfin-music-server-complete-guide-clients-scrobbling-2026
- https://jellywatch.app/blog/jellyfin-playlists-smart-playlists-plugin-guide-2026
- https://github.com/finamp-app/finamp
- https://github.com/finamp-app/finamp/discussions/1749
- https://github.com/finamp-app/finamp/issues/24
- https://github.com/finamp-app/finamp/issues/50
- https://github.com/finamp-app/finamp/issues/44
- https://github.com/finamp-app/finamp/issues/194
- https://github.com/finamp-app/finamp/issues/616
- https://github.com/finamp-app/finamp/issues/1283
- https://symfonium.app/
- https://symfonium.app/news/version-1410/
- https://www.xda-developers.com/single-music-app-plex-jellyfin-navidrome/

Navidrome and OpenSubsonic
- https://github.com/navidrome/navidrome/releases/tag/v0.61.0
- https://github.com/navidrome/navidrome/releases/tag/v0.62.0
- https://github.com/navidrome/navidrome/releases/tag/v0.63.0
- https://github.com/navidrome/navidrome/releases/tag/v0.64.0
- https://linuxiac.com/navidrome-0-60-music-server-and-streamer-released/
- https://freedom.tech/posts/2026-09-21-navidrome-0-64-1/
- https://www.navidrome.org/docs/usage/features/
- https://www.navidrome.org/docs/usage/features/smart-playlists/
- https://www.navidrome.org/docs/usage/library/tagging/
- https://github.com/navidrome/navidrome/issues/238
- https://github.com/navidrome/navidrome/issues/1417
- https://github.com/navidrome/navidrome/issues/1421
- https://github.com/navidrome/navidrome/issues/192
- https://github.com/navidrome/navidrome/issues/745
- https://github.com/navidrome/navidrome/issues/250
- https://github.com/navidrome/navidrome/issues/305
- https://github.com/navidrome/navidrome/issues/2136
- https://github.com/navidrome/navidrome/issues/1547
- https://github.com/navidrome/navidrome/issues/369
- https://github.com/navidrome/navidrome/issues/489
- https://github.com/navidrome/navidrome/issues/2771
- https://github.com/navidrome/navidrome/issues/1551
- https://github.com/navidrome/navidrome/issues/1473
- https://github.com/navidrome/navidrome/issues/4148
- https://github.com/navidrome/navidrome/issues/4744
- https://github.com/navidrome/navidrome/issues/6170
- https://github.com/navidrome/navidrome/issues/793
- https://github.com/navidrome/navidrome/issues/141
- https://github.com/navidrome/navidrome/discussions/3371
- https://github.com/navidrome/navidrome/discussions/3454
- https://opensubsonic.netlify.app/docs/extensions/
- https://opensubsonic.netlify.app/docs/extensions/sonicsimilarity/
- https://opensubsonic.netlify.app/docs/extensions/playbackreport/
- https://opensubsonic.netlify.app/docs/extensions/transcoding/

Emby
- https://emby.media/community/topic/144851-unfinished-music-library-business
- https://emby.media/support/articles/Music-Naming.html
- https://emby.media/emby-server-46-released.html

Roon, Lyrion, foobar2000
- https://roon.app/en/pricing
- https://roon.app/en/arc
- https://community.roonlabs.com/t/roon-2-65-and-arc-1-77-are-live/318732
- https://help.roonlabs.com/portal/en/kb/articles/dsp-engine
- https://help.roonlabs.com/portal/en/kb/articles/volume-leveling
- https://lyrion.org/
- https://lyrion.org/getting-started/changelog-lms9/
- https://www.foobar2000.org/
- https://www.foobar2000.org/changelog
- https://www.foobar2000.org/components/view/foo_playcount

Spotify, Apple Music, Tidal
- https://support.spotify.com/us/article/audio-quality/
- https://support.spotify.com/us/article/volume-normalization/
- https://support.spotify.com/us/artists/article/loudness-normalization/
- https://support.spotify.com/us/article/listen-offline/
- https://support.spotify.com/us/article/spotify-connect/
- https://support.spotify.com/us/article/jam/
- https://support.spotify.com/us/article/lyrics/
- https://newsroom.spotify.com/
- https://variety.com/2025/digital/news/spotify-hd-lossless-audio-premium-subscribers-launch-1236513178/
- https://www.androidauthority.com/spotify-smart-reorder-playlist-transitions-3644159/
- https://www.musicradar.com/music-tech/spotify-responds-to-apple-musics-new-automix-feature-by-letting-you-turn-your-playlists-into-ready-made-dj-sets-with-seamless-transitions
- https://musictech.com/news/music/apple-music-ios-26-updates/
- https://support.apple.com/en-us/118295
- https://freeyourmusic.com/blog/tidal-music-review

Standards, data sources and libraries
- https://musicbrainz.org/doc/Release_Group
- https://musicbrainz.org/doc/Release_Group/Type
- https://musicbrainz.org/doc/Work
- https://picard-docs.musicbrainz.org/en/latest/appendices/tag_mapping.html
- https://www.rfc-editor.org/rfc/rfc7845.html
- https://www.last.fm/api/scrobbling
- https://listenbrainz.readthedocs.io/en/latest/users/api/core.html
- https://lrclib.net/api/get (tested directly)
- https://github.com/tranxuanthang/lrclib
- https://github.com/NeptuneHub/AudioMuse-AI
- https://github.com/Polochon-street/bliss-rs
- https://github.com/pdeljanov/Symphonia
- https://developer.chrome.com/blog/media-source-extensions-for-audio
- https://developer.mozilla.org/en-US/docs/Web/API/Media_Session_API
- https://developer.mozilla.org/en-US/docs/Web/API/ManagedMediaSource
- https://developer.android.com/reference/android/media/AudioMixerAttributes
