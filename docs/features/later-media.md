# Adjacent media types

This map covers the media types that sit next to music, movies and TV:
audiobooks, podcasts, music videos and concerts, home videos, photos, ebooks,
and comics and manga. For each one it records what the user would get, how
the specialist rivals handle it, and whether Gunmetal should build it at all.
The bar differs by type. For audiobooks Gunmetal has to beat Audiobookshelf
as a player, not just Plex and Jellyfin. For music videos and home videos it
has to match Plex while staying free. For podcasts, photos and books it
builds only where its architecture gives a real edge over Pocket Casts,
Immich, Kavita and Komga, and integrates with them everywhere else. Two rules
hold throughout: nothing here may delay R1, and every type must reuse what R1
already builds (the core parsers, the player, the synced library and the
user log). A type that needs its own heavy stack, such as
server-side machine learning or phone backup, is integrated rather than
built.

## Features

Written on 2026-10-02 from `docs/research/adjacent-media.md` and
`docs/research/pain-points-and-demand.md`, with a few rows drawn from the
other research files (clients, discovery, music UX, users and security,
library, video). Vote and upvote counts are as those files recorded them on
2 October 2026. "Rivals today" repeats only what the research found;
"(unverified)" marks what it could not confirm, and "no data" means the
research does not cover that rival for that feature. ABS is Audiobookshelf
and PC is Pocket Casts. Sign-in, sharing, admin and backup features apply to
these types unchanged and are mapped in their own files; rows here cover
only what is specific to each type.

Releases (R1, R2, R3, Later, No), the Demand scale, row ownership and the
terms "the user log" and "the identity store" are defined once in the
[feature map README](README.md). A row whose Release cell would differ
between maps names one owning row; the other maps point at it.

### Shared foundations

These rows are the cheap insurance the research asks for. Only the
schema-only parts ship in R1 (LAT-001, LAT-002's role column, LAT-006,
LAT-007, LAT-008's relation table, LAT-009 and a simple LAT-010 folder flag),
so later types arrive without a migration. New parsers (LAT-003 to LAT-005)
wait for the types that need them, because no parser is cheap under the
100% mutation gate. LAT-010 and LAT-011 (through MUS-078, now R2) are the
only rows R1 users would notice.

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LAT-001 | Item kinds in the data model | New media types arrive later without a schema migration or a full rescan | Jellyfin: separate library types including books, photos and music videos; Plex: no audiobook type | High: fragile upgrades are a top pain point (Jellyfin 10.11 migration failures, #15027, 121 comments) | R1 | Every item carries a kind (track, audiobook, podcast episode, video, photo, book, comic) from the first schema, so SQLite stays a rebuildable cache across releases | Kind field on items; versioned kind enum in the core protocol types | None in R1 |
| LAT-002 | People with typed roles | Browse by author, narrator, host, guest, writer or penciller, not only "artist" | ABS: several narrators per book; Plex: narrator only by tag convention; Jellyfin: authors tab (12.0) | Medium: Plex users rely on tag conventions for narrators | R1 | One person record with a role on each credit, shared with music's composer and performer roles, so a narrator who also records music is one person | R1: the role column on credits (shared with music's composer and performer roles). OPF and ComicInfo role mapping arrives with books. | None in R1; person pages from R2 |
| LAT-003 | Series with a free-form sequence | Books numbered "1.5", "0" or "Prequel" sort correctly in their series | ABS: sequence numbers; Kavita and Komga: yes; Jellyfin: series from filenames (12.0) | Medium: ABS missing-books request (26); Jellyfin series request (4, Sept 2026) | Later | The sequence is kept as the original string plus a sort key computed by one tested core function Arrives with books; nothing in R1 needs it. | Series and membership tables; sort-key function | None in R1 |
| LAT-004 | One item spanning many files | A 40-file rip is one book with one timeline | ABS: yes; Plex: no; Jellyfin: one folder per book, with requests to handle split books better | Medium: Jellyfin commenters (2023) | Later | An item is an ordered manifest of files with durations read at scan, giving one global timeline; positions are global milliseconds Arrives with audiobooks; R1 multi-file items are albums. | Manifest table; per-file duration and gapless data from the core parsers | None in R1 |
| LAT-005 | Chapters as a shared structure | A chapter list and chapter skip for any audio file that has chapters, such as a DJ mix or a live set | Plex and Plexamp: ignore M4B chapters; Jellyfin: chapter extraction (12.0); ABS: yes | Medium: Plex chapter complaints; Jellyfin auto-advance request (26) | Later | Chapters parsed at scan in pure Rust (MP4 `chpl` and QuickTime chapter tracks, ID3v2 `CHAP` and `CTOC`, Vorbis comments, Matroska through the existing EBML code), never by FFmpeg; one structure (start, end, title, image, link) serves books, podcasts, concerts and films Later, with the media types that need chapters: new parsers are not cheap under the 100% mutation gate, and R1 music does not need them. | Chapter table keyed to item, file and offset; parser fuzzing | Chapter list in now playing; chapter ticks on the seek bar |
| LAT-006 | Typed positions in the user log | Your place is kept for audio, text and pages alike, and survives any rebuild | Komga: KOReader and Kobo sync keep only chapter starts; Kavita: encodes OPDS progress in titles | Medium: ABS read-along requests (91, 55) depend on it | R1 | The append-only user log from record 1 stores a position as a time offset, a Readium-style text locator, a page of a total, or a percentage, with device and time on every event | Typed position events; event schema versioning | None beyond music resume in R1 |
| LAT-007 | All user-made data in the user log | Bookmarks, notes, finished dates, subscriptions, albums and hidden items survive upgrades and can be exported | Immich: its database backup excludes the photos themselves; Jellyfin 12.0: one-way upgrade that needs a backup first | High: fragile upgrades are a top pain point | R1 | Every user-made fact is a user-log event, never only a SQLite row, so "rebuild from files plus user log" stays true; exports use OPML, KOReader progress or JSON | Event types added per kind as each ships; export and import jobs | Export page in settings |
| LAT-008 | Typed links between items | A song links to its video, an ebook to its audiobook, an interview to its artist | Plex: links videos to tracks by file name; Jellyfin: no; read-along needs a separate app (Storyteller) | Medium: Jellyfin requests (38, 9); ABS read-along (91) | R1 | A relation table with typed edges (video of, audio of, extra for) in the first schema; scanners and users fill it later | R1: the relation table only. Inference jobs per kind arrive with each kind. | None in R1 |
| LAT-009 | Listening contexts in the queue protocol | Nothing visible in R1; later, a book and an album each keep their own queue and place | ABS: no; Jellyfin: one music queue; Symfonium (a client): several queues | Medium: ABS queue request (99) | R1 | The cross-device queue from record 2 carries named contexts from its first version, so LAT-039 needs no protocol break | Context id on queue state and hand-off messages | None in R1 |
| LAT-010 | Spoken word kept out of music | Audiobooks found in an R1 library stay out of shuffles, radio and album grids | Plex: no book type, so books live in music libraries; Spotify: "hide podcasts" idea has 8,815 votes | Medium: Spotify 8,815 votes; a Jellyfin commenter wants a book resume that does not hijack music | R1 | R1 keeps this to a simple per-folder flag set by the admin, which music surfaces filter on; automatic classification of M4B files and audiobook tags arrives with audiobooks. | Per-folder kind flag | Library settings toggle; a plain "Spoken word" list in R1 |
| LAT-011 | Long audio remembers its place | See MUS-078, which owns this feature. Later-media specifics: a resume threshold per kind. | Plexamp: resumes long audio; Jellyfin: partial | Medium: discovery research; Jellyfin chapter auto-advance request (26) | R2 | See MUS-078. | Per-kind resume threshold; otherwise none beyond MUS-078 | Resume prompt in the player; Continue row |
| LAT-012 | Choose which media kinds you see | Someone who only wants music never sees books or podcasts | Spotify: no (8,815-vote idea, under consideration); Apple: podcasts live in a separate app | High: Spotify 8,815 and 5,223 votes | R2 | Visible kinds are a per-user setting applied when the device's synced library is built, so on a device used by one person hidden kinds are not even downloaded | Per-user kind filter in the sync feed | Settings toggle; navigation and Home adapt |
| LAT-013 | Kind chips in search and library | One search over songs, books and episodes, narrowed with a tap | Spotify: chips for music and podcasts; Jellyfin: separate libraries | Medium: music UX research | R2 | Search runs on the synced library on the device, so it works offline across kinds | Kind facet in the synced index | Search chips; library chips |
| LAT-014 | Content limits that know each kind | Children see children's books but not explicit ones, and unrated home videos are handled on purpose | ABS: explicit content denied to new users by default (2.14.0); Jellyfin: blocks unrated items per media type; Komga: limits enforced for Kobo too | Medium: users and security research | R2 | Limits apply when the synced library is built and on every signed URL, so restricted items never reach the device and no adapter or protocol can bypass them | Per-user rules by kind, rating and explicit flag, applied to sync, adapters and URLs | Profile and parental controls |

### Audiobooks

Rivals: ABS (the purpose-built server), Plex with Plexamp or the
third-party Prologue app, and Jellyfin 12. The full book experience ships
with the native mobile clients in R2; see the first open decision.

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LAT-015 | Audiobook library | Books behave as books, with covers, authors, narrators, series and progress, not as albums by an "artist" | ABS: yes; Plex: no (request open since 2013); Jellyfin: partial ("Books" type) | High: Plex 2,264 votes; Jellyfin 857 ("started"); ABS used by 840 of 4,081 selfh.st respondents | Later | Same app, server and synced library as music, so books browse offline and need no second server; built on LAT-001 to LAT-011 | Audiobook kind; grouping by folder and tags | Books tab; book page; book mode in now playing |
| LAT-016 | Multi-file books play as one | One progress bar and no gaps across 40 files | ABS: yes; Plex: no; Jellyfin: partial | Medium: Jellyfin commenters (2023) | Later | Music's gapless engine plays across the LAT-004 manifest | Manifest; durations; gapless data | One seek bar and time remaining for the whole book |
| LAT-017 | Chapter navigation | Next and previous chapter, a chapter list, and automatic advance | ABS: yes; Plex and Plexamp: ignore M4B chapters (Prologue on iOS reads them); Jellyfin: extracts chapters, but one 2026 guide says its web player has no chapter skip | High: Plex chapter complaints; Jellyfin auto-advance (26) | Later | LAT-005 chapters are mapped to file and offset, so skipping works across multi-file books | Chapter-to-file mapping | Chapter buttons; chapter sheet; lock-screen chapter skip |
| LAT-018 | Chapter-scaled seek bar | Switch the seek bar between the whole book and the current chapter | No data | Low: no requests found | Later | Parity | Chapter boundaries in the synced item | Seek bar toggle |
| LAT-019 | Chapter fixes without touching files | Rename, add or move chapters when a rip has bad ones | ABS: chapter editor; Plex: no; Jellyfin: no (unverified) | Low: no counts found | Later | Fixes are user-log overlays on the parsed chapters, so files stay untouched and fixes survive rescans | Chapter overlay events | Chapter editor (web) |
| LAT-020 | Chapter lookup by Audible ASIN | Correct chapters fetched from a community database, with Audible branding trimmed | ABS: yes (2.22.0); Plex: no; Jellyfin: no (unverified) | Low: no counts found | No | Not first-party: the data is Audible-derived and carries terms-of-service risk; the plugin interface would not stop a third party offering it | — | — |
| LAT-021 | Metadata from embedded tags and OPF | Title, author, narrator, series and cover filled from the files, with no internet | ABS: yes; Jellyfin: embedded tags and OPF; Plex: read as music tracks | Medium: a July 2026 review found Jellyfin could not match MP3 audiobooks online | Later | The core parses MP4, ID3 and Vorbis tags and OPF sidecars at scan; its XML parser refuses external entities and entity expansion | Tag-to-book field mapping; OPF parser | Book page showing the source of each field |
| LAT-022 | Online book metadata through plugins | Covers, blurbs and series filled in when the files lack them | ABS: Audible, Google Books, iTunes, Open Library (partly unverified) and custom providers; Plex: third-party Audnexus agent; Jellyfin: Google Books, Open Library | Medium: Jellyfin's failed MP3 matching | Later | First-party plugins use open sources under an explicit network grant (record 2); results are user-log overlays tagged by source, so a bad match is undone in one step | Plugin host; match queue; overlay events | Match dialog; unmatched list; per-field lock |
| LAT-023 | Author and narrator pages | Everything one narrator has read; several narrators per book | ABS: since 1.7.0; Plex: tag conventions; Jellyfin: authors tab (12.0), narrators (unverified) | Medium: ABS ships it as a core feature | Later | Typed roles from LAT-002 | Credits by role | Person page with role tabs |
| LAT-024 | Series in listening order | Book 3 follows book 2, and the series page shows where you are | ABS: yes; Jellyfin: series from filenames (12.0) | Medium: Jellyfin series request (4); ABS series features | Later | LAT-003 sort key; the next unfinished book is computed from the user log | Series ordering; next-in-series query | Series page; "Next in series" after finishing |
| LAT-025 | Missing books in a series | Gaps in a series are shown | ABS: requested (26) | Low: 26 | Later | Needs an outside catalogue, so it comes from a metadata plugin; nothing is fetched without the grant | Series catalogue from a plugin | Placeholder tiles on the series page |
| LAT-026 | Resume across devices | Start on the phone, finish on the TV or the web | ABS, Plex (reliable per a 2026 review) and Jellyfin: yes | High: a basic expectation in every rival | Later | Parity | Position events; merge rule from LAT-027 | Resume button; Continue listening row |
| LAT-027 | Position conflicts handled openly | When two devices disagree you choose, instead of silently losing an hour | No data | Low: raised as an open question, no votes | Later | User-log events carry device and time; when recent positions on two devices differ by more than a chapter, the player asks | Divergence check over position events | Resume prompt naming each device; position history with undo |
| LAT-028 | Rewind on resume | After a pause the book backs up a few seconds | Symfonium: yes; others: no data | Low: clients research only | Later | Parity | None (synced client setting) | Player setting |
| LAT-029 | Finished books with dates | A finished shelf with start and finish dates you can edit | ABS: yes, editable dates requested (30); Plex and Jellyfin: played flag (unverified) | Medium: 30 | Later | Started, finished and re-listened are user-log events, editable and exportable | Finished events; per-kind finished rule | Finished shelf; date editor; Mark finished |
| LAT-030 | Set a book aside | Drop an abandoned book from Continue listening without marking it finished | No data for books; Jellyfin's video version is its second most-wanted request (1,725) | Medium: 1,725 for video, by analogy | Later | A set-aside user-log event, separate from finished | Set-aside events | Row long-press menu |
| LAT-031 | Speed with pitch kept | 1.5x without chipmunk voices, remembered per book | ABS and Plexamp: yes; Plex: 1,298-vote speed request implemented; Jellyfin: depends on client | High: 1,298 | Later | Parity (libmpv on native clients; the browser's pitch-preserving rate on web, unverified on every TV) | Per-item playback settings in the user log | Speed control |
| LAT-032 | Skip intervals you choose | Jump back 30 seconds after a distraction, at lengths you set | ABS: yes (intervals unverified); Plexamp (unverified) | Medium: Plex's custom skip-length request (453, for video) | Later | Parity | Synced client setting | Skip buttons; lock-screen and headphone skips |
| LAT-033 | Chapter-aware sleep timer | Stop after a time, at the end of the chapter, or after N chapters | ABS: times and end of chapter (2.12.0); Plexamp: yes; PC: after N chapters on Android; Jellyfin: episode-count request (11) | Medium: 11, and every specialist ships it | Later | One sleep timer shared with music, chapter-aware through LAT-005 | None | Sleep timer sheet; countdown in now playing |
| LAT-034 | Bookmarks with notes | Mark a passage and come back to it | ABS: yes; Plex and Jellyfin: no (unverified); PC: paid | Medium: PC charges for it | Later | Free; bookmarks are user-log events, exportable, and visible to third-party apps through OpenSubsonic's bookmark endpoints | Bookmark events | Bookmark button; bookmark list per book |
| LAT-035 | Bookmark from lock screen or headphones | Save the moment without unlocking the phone | PC: yes (paid); ABS: no data | Low: no counts found | Later | A native media-session action writes the bookmark offline and syncs it later | Bookmark events | Lock-screen action; headphone gesture setting |
| LAT-036 | Offline books | Download a book and listen on a plane | ABS: yes; Plexamp downloads (unverified); Jellyfin: varies (offline is its top request, 1,820) | High: 1,820 | Later | Same download engine as music; the original, or Opus to save space (cheap, per record 2); book metadata is already on the device | Download manifests; optional Opus job | Download button; downloads screen; storage use |
| LAT-037 | Lock-screen and background playback | Lock the phone and keep listening, with skip-back controls | ABS and Plexamp: yes; Jellyfin: iOS background audiobook playback only since 12.0 | High: Jellyfin users reported audio stopping on lock | Later | Parity | None | Lock screen; notification controls |
| LAT-038 | Android Auto and CarPlay for books | Resume the book from the car screen, offline | ABS: Android Auto yes, CarPlay is its top app request (100); Jellyfin: audiobooks in Android Auto (Android 2.7.0); Plexamp (unverified) | High: 100 | Later | Android Auto for books: parity with Audiobookshelf, with the edge that books and music share one app with separate queues (LAT-039). CarPlay for books would put Gunmetal ahead of Audiobookshelf, whose top request is CarPlay, but it waits for the Apple builds. Both are Later, with audiobooks (ADR 2 puts live TV first). | Car browse tree | CarPlay and Android Auto screens |
| LAT-039 | A book never hijacks the music queue | Play an album mid-book; the book waits at its place, and Resume means the book | ABS: no (queue request 99); Jellyfin: one queue; Symfonium (a client): several queues | Medium: 99, plus Jellyfin commenters | Later | Built on LAT-009: each context keeps its own queue and position and hands off between devices; no server does this today | Context-aware queue sync | Context switcher in now playing; Resume book widget and shortcut |
| LAT-040 | Casting books | Send a book to a Chromecast speaker | ABS: since 1.7.0; Plex and Jellyfin (unverified) | Low: no counts found | Later | Parity | Signed URLs the receiver can refresh | Cast button |
| LAT-041 | Personal ratings for books | Rate books for yourself and sort by it | ABS app: self-rating requested (75) | Medium: 75 | Later | Ratings are user-log events, shared with music ratings | Rating events | Rating control; sort option |
| LAT-042 | Book collections | Group books by theme | ABS: yes; Plex: playlists; Jellyfin: collections tab (12.0) | Low: every rival has it | Later | Parity | Collection events in the user log | Collections page |
| LAT-043 | Listening statistics | Time listened, books finished, a year in review | ABS: stats and Year in Review (2.7.0); Plex and Jellyfin: no | Low: no requests found | Later | Computed from the local user log; nothing leaves the server Uses the DIS-183 statistics engine. | Aggregation job | Stats screen |
| LAT-044 | Share a book by expiring link | Send one book to a friend, with an end date | ABS: admin-only links with expiry (2.11.0); Jellyfin: share-link request open (580) | Medium: 580 across all media | Later | A signed, scoped, expiring capability, re-checked on every request (Navidrome's shared streams outlived deletion until Sept 2026) | Share capabilities; revocation | Share sheet; My shares |
| LAT-045 | Private podcast feed for a book | Listen to a book in any podcast app | ABS: yes, including MRSS (2.18.1); Plex and Jellyfin: no | Low: no counts found | Later | A per-book, read-only, revocable capability URL with an expiry, off by default; it is the one long-lived URL in this map, so it needs an owner decision | Feed generation; capability store | Share sheet option with a warning |
| LAT-046 | Merge and embed tools | Turn an MP3 folder into one tagged M4B file | ABS: yes | Low: no counts found | No | Gunmetal never writes to library files; fixes live as user-log overlays (LAT-019, LAT-022) | — | — |
| LAT-047 | Switch between ebook and audiobook by chapter | Stop reading at chapter 12 and carry on listening at chapter 12 | ABS: no (requests 91 and 55); Jellyfin: not supported; Storyteller: a separate app | Medium: 91 and 55 | Later | LAT-008 links the two editions; matching chapter titles map a text locator to a time | Chapter mapping between linked items | Continue in audio; Continue in text |
| LAT-048 | Sentence-level read-along | The text highlights as the narrator reads | Storyteller: yes (EPUB 3 media overlays); others: no | Medium: the same requests (91, 55) | Later | Plays EPUB 3 media overlays that already exist; creating them needs speech recognition, so only as an opt-in, sandboxed plugin | Media-overlay parsing; optional alignment plugin | Reader with audio highlighting |
| LAT-049 | Generated transcripts for books | Read or search what was said | ABS: requested (Whisper 44, transcripts 27) | Low: 44 and 27 | Later | Opt-in and sandboxed, ideally on a client device; never a server requirement | Optional job; transcript storage | Transcript panel |
| LAT-050 | Books in a store-listed iOS app | Install from the App Store, not a capped beta | ABS: TestFlight only (10,000-tester cap, no date); Prologue: third-party; Jellyfin: official app (book quality unverified) | High: ABS's release checklist issue has 96 comments | Later | Later; R2 if the App Store licence decision allows (open decision 3 in the feature map README). Books ride in the same native iOS app as music; App Store approval is not guaranteed | None | iOS app |
| LAT-051 | Audiobook formats | M4B, M4A, MP3, FLAC, Opus, Ogg and MKA all play | ABS: broad (Opus in Matroska fixed in 2.33.2); Plex: M4B plays; Jellyfin: broad | Medium: ABS was still fixing format gaps in 2026 | Later | Parity in coverage; parsed by the core without FFmpeg and sent as original bytes | Shared audio parsers | None |
| LAT-052 | DRM-protected audiobooks | Play Audible AAX files directly | Native support in any rival (unverified) | Low: no counts found | No | Supporting DRM removal is a legal risk for an open project | — | — |
| LAT-053 | Progress survives replaced files | Re-ripping or upgrading a book keeps your place | Jellyfin 10.11: lost watch state on replace (#15001, 61 +1); others: no data | Medium: 61 | Later | Positions are keyed to content identity; when the total duration changes, the position maps by chapter title and proportion, and the old value stays in the user log | Remap job on file change | "Your place was adjusted" notice with undo |
| LAT-054 | Import from Audiobookshelf | Progress, finished dates and bookmarks come across in one step | No importer found in the research | Medium: ABS has 840 selfh.st users, so switching cost matters | Later | Reads ABS through its API (keys since 2.26.0) with a grant to one host; imported events are tagged by source and can be undone | Import job; source-tagged events | Import wizard |
| LAT-055 | Audiobookshelf-compatible API | ABS's own apps connect to Gunmetal | None | Low: no requests found | Later | Not recommended for scheduling: ABS says its API is still being standardised, and adapters inherit protocol weaknesses | Adapter endpoints | None (third-party apps) |
| LAT-056 | Lectures, courses and audio dramas | Spoken word that is not a book still gets speed, resume, chapters and bookmarks | No data | Low: raised as an open question | Later | Book behaviour belongs to the kind, so a folder can opt in without pretending a lecture is a book | Kind per folder | Library settings |
| LAT-057 | Books on TV clients | Listen to a book on the living-room TV | Jellyfin: Android TV audiobook request (5); ABS: no data | Low: 5 | Later | Same synced library and player on the TV | None | TV book shelf and player |

### Podcasts

Rivals: PC (the reference app), ABS (the self-hosted podcast server people
use), and the video servers, none of which supports podcasts today: Plex
removed them in April 2022 and Jellyfin has none. Everything here is Later
and depends on the plugin host and the safe fetcher (see Dependencies and
risks). Pocket Casts is free and excellent; the case for Gunmetal is no
account, your own archive, and one app for music, books and podcasts.

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LAT-058 | Podcasts as a plugin with a network grant | Podcasts, without the server reaching the internet unless the admin allows it | ABS: built in; Plex: removed (April 2022, citing low usage); Jellyfin: none | High: Jellyfin podcast support 563 votes; podcasts are Navidrome's most +1'd open request | Later | Feed fetching is a plugin holding an explicit outbound grant (record 2); feed parsing is pure logic in the core Interim answer until then: OpenSubsonic podcast endpoints through LAT-089, or Audiobookshelf beside Gunmetal (open decision). | Plugin host; grant settings | Admin plugin page |
| LAT-059 | Safe feed fetching | A hostile feed cannot make the server call your router or other internal addresses | ABS: SSRF filtering for downloads (2.34.0); PC: not applicable | Medium: ABS's 2026 fix; Navidrome's 2026 SSRF advisories | Later | The fetcher refuses private and loopback addresses, checks the resolved address at connect time and after every redirect, and caps size and time | Guarded HTTP client in the plugin host | Fetch errors on the show page |
| LAT-060 | Directory search and subscribe | Find a show by name and follow it | PC: yes; ABS: iTunes search with region (2.10.0) | Medium: Jellyfin 563 | Later | The search query leaves the server only through the plugin's grant | Directory search through the plugin | Podcast search screen |
| LAT-061 | Subscribe by feed URL | Add any feed by its address | PC and ABS: yes (unverified) | Medium: the basic way to add a show | Later | Subscriptions are user-log events | Subscription events | Add by URL |
| LAT-062 | OPML import and export | Move subscriptions in or out in one file | PC: yes (unverified); ABS: import since 2.0.18, export (unverified) | Medium: the standard switching path | Later | Export is always available because subscriptions are user-log events | OPML reader and writer in the core | Import and export in settings |
| LAT-063 | Podcasting 2.0 metadata | Seasons, episode numbers, people, GUIDs and funding links appear | PC: funding tag and podroll; ABS: no data | Low: no counts found | Later | RSS and Podcasting 2.0 tags parsed in the core with hardened XML; the plugin only does the I/O | Feed parser | Show and episode pages; funding link |
| LAT-064 | Server archive of episodes | Episodes stay after the publisher deletes them | ABS: yes; PC: no | Medium: ABS ships it and a cloud app cannot | Later | Archived episodes become library files with per-show retention rules | Download and retention jobs; storage quota | Per-show archive setting; storage view |
| LAT-065 | Stream without archiving | Play an episode without storing it on the server | PC: yes; ABS: no data | Low: no counts found | Later | Items backed by a remote URL, played through the server or straight from the publisher per show (a privacy trade-off; see open decisions) | Remote-URL items; optional relay through the server | Per-show playback source |
| LAT-066 | Keep the latest N episodes on the phone | The commute is ready without thinking | PC: yes; ABS: requested (46) | Medium: 46 | Later | Rule-based downloads shared with music | Download rules | Per-show download rule |
| LAT-067 | Up Next across shows and books | Line up episodes from several shows, and a book | PC: yes; ABS: requested (99) | Medium: 99 | Later | The spoken-word context from LAT-009 holds episodes and books together, apart from music | Context queue | Up Next screen |
| LAT-068 | Rule-based episode lists | "Unplayed, under 30 minutes" | PC: filters; ABS: no (unverified) | Low: no counts found | Later | Reuses the music smart-playlist rules, evaluated on the device's synced library | Rule definitions in the user log | Filter builder |
| LAT-069 | Played state and auto-archive | Finished episodes leave the list | PC: auto-archive rules; ABS: requested (31) | Medium: 31 | Later | Played and archived are user-log events | Events; archive rules | Swipe actions; per-show rule |
| LAT-070 | Oldest-first per show | Serial shows play in order | PC: per-show sort (unverified); ABS: requested (35) | Medium: 35 | Later | Parity | Per-show sort setting | Sort control on the show page |
| LAT-071 | Speed per show | 1.8x for one show, 1.0x for another | PC: global or per show, 0.5x to 3x on mobile and up to 5x on web; ABS: per show no (unverified) | Medium: a PC headline feature | Later | Parity; per-show settings live in the user log and apply on every device | Per-show settings | Effects sheet |
| LAT-072 | Trim silence | Shorter pauses without changing the voice | PC: three levels, mobile only; ABS: no (unverified) | Medium: a PC headline feature | Later | libmpv audio filters on native clients (unverified per platform); the web needs Web Audio work, so mobile first, as with PC | None | Effects sheet |
| LAT-073 | Volume boost | Quiet hosts become audible; works for books too | PC: mobile; ABS: requested (26) | Low: 26 | Later | Same filter path as LAT-072, for all spoken word | None | Effects sheet |
| LAT-074 | Skip intro and outro per show | Theme music and fixed openers are skipped | PC: yes (unverified); ABS: no (unverified) | Low: no counts found | Later | Parity | Per-show settings | Effects sheet |
| LAT-075 | Episode chapters | Jump to the segment you want | PC: MP3 chapters, Podcast Index JSON and Podlove; ABS: since 2.2.21 | Medium: both specialists ship it | Later | LAT-005 parsers plus Podcast Index JSON chapters in the core | JSON chapter fetch through the plugin | Chapter sheet |
| LAT-076 | Generated chapters | Chapters for shows that publish none | PC: offered but switched off at the time of writing | Low: no counts found | No | Needs machine learning on the server, against the low-hardware goal | — | — |
| LAT-077 | Feed transcripts | Read along with an episode | PC: yes, from the Podcasting 2.0 tag (2.5 million episodes); ABS: requested (27) | Medium: 27, plus PC's coverage | Later | The transcript tag is parsed in the core; the text is fetched by the plugin and shown in time with playback | Transcript storage | Transcript panel |
| LAT-078 | Search inside transcripts | Find the episode where a topic came up | PC: search within an episode | Low: no counts found | Later | Indexed on your server across all episodes; no outside service | Transcript index | Search results with timestamps |
| LAT-079 | Free episode bookmarks | Save a moment in an episode | PC: paid (Plus or Patron); ABS: for episodes (unverified) | Medium: PC charges for it | Later | Free user-log events, shared with LAT-034 | Bookmark events | Bookmark button |
| LAT-080 | Free folders for shows | Group subscriptions | PC: paid, with smart folders; ABS: no | Low: no counts found | Later | Free user-log events | Folder events | Folder management |
| LAT-081 | Episode sleep timer | Stop after this episode or after N episodes | PC: many modes, mobile only; ABS: time and end of chapter; Jellyfin: episode-count request (11) | Medium: 11 | Later | Same timer as LAT-033, on every client | None | Sleep timer sheet |
| LAT-082 | Video podcasts | Watch shows that publish video | PC: yes; ABS: audio focus (unverified); Jellyfin: one commenter asked | Low: one comment | Later | Plays through the R2 video path; LAT-099 lets you just listen | None extra | Episode page |
| LAT-083 | Your own recordings in Up Next | A lecture you recorded sits in the same queue as episodes | PC: files cannot go into playlists; ABS: it is your server already | Low: no counts found | Later | Any spoken-word item can join the spoken-word context | None | Add to Up Next |
| LAT-084 | Sync with no account | Same place on phone and web, free, with no third-party account | PC: free but needs an account; ABS: yes | Medium: privacy is the second most common reason to self-host (selfh.st) | Later | Your server is the account, and the user log syncs over iroh | None extra | None |
| LAT-085 | Full player on web and desktop | Sleep timer and effects at a desk | PC: web limited to speed effects, sleep timer mobile only | Low: no counts found | Later | Parity at first: the player code is shared, but web effects wait on Web Audio work | None | Web player |
| LAT-086 | Podcast statistics | Time listened and time saved by effects | PC: yes; ABS: library stats | Low: no counts found | Later | Computed from the local user log | Aggregation job | Stats screen |
| LAT-087 | Episode artwork | Each episode shows its own image | PC: yes (unverified); ABS: requested (42) | Medium: 42 | Later | Images are fetched by the plugin, thumbnailed in the sandbox and kept as rebuildable cache | Image fetch; thumbnail job | Episode rows |
| LAT-088 | Open podcast sync protocol | AntennaPod-style apps sync with Gunmetal | Nobody; gPodder sync and the Open Podcast API exist | Low: one Jellyfin commenter | Later | Pick one protocol after checking its support (details unverified) | Sync endpoints | None (third-party apps) |
| LAT-089 | Podcasts through OpenSubsonic | Existing Subsonic apps see your podcasts | ABS: no (unverified); Navidrome: podcast request (43) | Low: 43 | Later | Comes with the planned adapter: OpenSubsonic defines eight podcast endpoints | Adapter endpoints | None (third-party apps) |
| LAT-090 | Who may manage podcasts | Only chosen users add or delete shows; subscriptions can be personal or household | ABS: management permissions requested (43) | Low: 43 | Later | Per-object authorisation on every show and subscription | Permissions per show | User permissions in admin |

### Music videos and concerts

Rivals: Plex, Jellyfin and Emby. Plex is good here; the aim is to match it,
beat Jellyfin, and stay free. All of it needs the R2 video path; only the
relationships (LAT-008) exist from R1.

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LAT-091 | Music video on the song | The song and the now-playing screen offer its video | Plex: yes, by file name next to the track; Jellyfin: no (9); Plexamp: does not show them (2026 request) | Medium: Jellyfin 38 and 9 | R2 | A LAT-008 link filled by Plex's naming rule (easy migration) or by hand | Link inference by name and folder; manual link events | Video badge on track rows; Watch video in now playing |
| LAT-092 | Typed artist extras | Live sets, concerts, interviews, behind-the-scenes and lyric videos on the artist page | Plex: yes, by file-name suffix; Jellyfin: no; Emby (unverified) | Medium: Jellyfin users ask to combine music and video (38) | R2 | Plex's suffixes (`-live`, `-concert`, `-interview`, `-behindthescenes`, `-lyrics`, `-video`) are accepted and stored as "extra for" links | Suffix parsing in the core | Videos shelf on the artist page, with a type filter |
| LAT-093 | Music video folders per library | Keep videos apart from audio files | Plex: one global path for the server; Jellyfin: its own library type | Low: no counts found | R2 | Per-library video folders feed the same artist model | Library roots for music video | Library settings |
| LAT-094 | One artist page for songs and videos | No second library to browse | Plex: yes; Jellyfin: requested since 2020 (38) | Medium: 38 | R2 | Parity with Plex, through the same music model | Artist query across kinds | Artist page |
| LAT-095 | Browse music videos | By artist, year and genre | Plex: through the music library; Jellyfin: requested (15) | Low: 15 | R2 | Parity | Facets in the synced index | Music videos view |
| LAT-096 | Music video metadata from files | Titles, artists and dates from tags and NFO files | Jellyfin: NFO only, nothing online by default; Plex (unverified) | Medium: Jellyfin documents NFO as the only route | R2 | The core reads MP4 and Matroska tags, and NFO through hardened XML | Tag and NFO parsing | Video detail page |
| LAT-097 | Online music video metadata | Artwork and credits filled in | Jellyfin: none by default; Plex and Emby (unverified) | Low: no counts found | Later | Plugin with a network grant | Plugin | Match dialog |
| LAT-098 | Swap a song for its video | Flip the playing track to its video and back | Plex (unverified); Jellyfin and Emby: no; Spotify and YouTube Music mix videos in | Low: no counts found | R2 | The queue item keeps the track identity and the player switches source, at the start of the video unless an alignment is known | Link lookup | Now-playing toggle |
| LAT-099 | Listen-only mode for videos | A concert film plays with the screen off and lock-screen controls | Jellyfin: requested (34); nobody ships it | Low: 34 | R2 | libmpv drops the video track and the item moves to the music player | None | Listen-only toggle |
| LAT-100 | Concert chapters mapped to songs | Jump to a song in a two-hour concert and see its setlist | Nobody links chapters to songs (unverified) | Low: no counts found | Later | LAT-005 chapters linked to tracks through LAT-008, matched by title or by hand | Chapter-to-track mapping | Setlist on the concert page |
| LAT-101 | Video plays count as song plays | Watching a concert adds to those songs' play counts | No data | Low: an open question | Later | Optional per user; scrobbling stays opt-in | Play events per mapped chapter | Setting |
| LAT-102 | Multiple video angles | Switch angles on concert discs | Jellyfin: requested (16, 2026); nobody ships it | Low: 16 | Later | The core indexes every video track; libmpv switches tracks on native clients (unverified per platform) | Track index | Angle picker |
| LAT-103 | Free remote viewing of music videos | Watch away from home with no pass | Plex: personal video needs a pass since 29 April 2025 (whether music videos count is unverified); Jellyfin and Emby: free | Medium: Plex's paywall is the top pain point | R2 | Nothing to paywall (AGPL, no central account). Native clients embed iroh; the project's own connection-success figures will replace iroh's quoted rate of about 90% direct (unverified for mobile carriers and TVs). No open port; whether there is no fee behind CGNAT depends on the relay decision (ACC-101). Browsers still need the owner's proxy or VPN until ACC-102. | None extra | None |

### Home videos

Rivals: Plex, Jellyfin and Emby; Immich appears where people now keep phone
clips. Home video is personal video with no lookups, so it is mostly a
date-first view over the R2 video path.

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LAT-104 | Home video library with no online lookup | Family videos never get matched to films | Plex: Personal Media agent; Jellyfin: Home Videos and Photos type; Emby (unverified) | Medium: a Jellyfin request asks to switch fetching off entirely in mixed libraries | Later | The kind has no lookup path, so no plugin is ever called for it | Library kind | Library setup |
| LAT-105 | Dates and titles from the file | Clips sort by when they were filmed, not when they were copied | Plex: reads MP4, M4V and MOV tags; Jellyfin and Emby (unverified) | Medium: every date view (LAT-106) depends on it | Later | The core reads MP4, QuickTime and Matroska dates at scan, with filename dates as a fallback and timezones kept when present | Capture-date extraction | Detail page; date fix as a user-log overlay |
| LAT-106 | Date timeline | Scroll through years and months of clips | Plex: timeline in the Plex Photos app; Jellyfin: no timeline (unverified); Immich: best | Medium: Immich sets the expectation | Later | Library metadata is on the device, so scrubbing through years is local and instant | Date index | Timeline with year scrubber |
| LAT-107 | Folder view | See LIB-008, which owns this feature. Applies to home videos when that library type ships. | Jellyfin: for home video and photo libraries (12.0), with a general request still open (438); Emby: yes | High: 438 | Later | See LIB-008. | None beyond LIB-008. | Folder browser |
| LAT-108 | Free remote viewing with no setup | Grandparents watch from their own house, free | Plex: paid for personal video since 29 April 2025; Jellyfin: free, but you bring your own VPN or reverse proxy; Emby: free | High: Plex's paywall is the top pain point | Later | No paywall. Native clients embed iroh; the project's own connection-success figures will replace iroh's quoted rate of about 90% direct (unverified for mobile carriers and TVs). No open port; whether there is no fee behind CGNAT depends on the relay decision (ACC-101). Browsers still need the owner's proxy or VPN until ACC-102. | None extra | Device pairing |
| LAT-109 | Share a clip by expiring link | Send one video to someone with no account; the link dies on its own | Jellyfin: requested (580); Plex: share-a-movie request (294); Immich: yes | High: 580 and 294 | Later | A signed, scoped, expiring capability for one item, re-checked on every request, with an optional password and download switch; it opens in a minimal guest player | Share capabilities; guest player route; revocation Built on ACC-135. | Share sheet; guest web page; My shares |
| LAT-110 | Phone HEVC and HDR clips | iPhone clips play right on any screen | Plex, Jellyfin and Emby: transcode; Immich: dark HDR complaints (15) | Medium: 15 | Later | Native clients direct-play through libmpv and browsers get a remux when they can decode the codec; the sandboxed tone-mapping transcode is the last resort, and there it is parity | Playback decisions | None |
| LAT-111 | Keep home videos off shared surfaces | Family clips stay out of recommendations, Home rows and screensavers unless you add them | Home-row exclusion: all three; suggestion exclusion: Jellyfin no (12) | Low: 12 | Later | Off by default for this kind | Per-kind surface rules | Library settings |
| LAT-112 | Copy media to another drive | Folder sync and backup of media | Emby: Folder Sync (Premiere) | Low: no counts found | No | Gunmetal never writes to media folders; backup tools do this better | — | — |

### Photos

Rivals: Immich, Plex and Jellyfin. Immich is far ahead and moving fast, and
its machine learning contradicts the low-hardware goal. Gunmetal's job is to
show photos well on every screen, especially TVs, and to integrate with
Immich rather than replace it. Everything here is Later; server-side machine
learning and phone backup are No.

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LAT-113 | Immich on the big screen | Your Immich albums and memories on Gunmetal's TV and phone apps | No integration found in the research | Medium: Plex's slideshow request (121); Plex removed photo libraries from its main iOS and Fire TV apps (Aug 2026) | Later | A plugin with a grant to one host and a scoped Immich API key; Gunmetal shows, Immich keeps owning the photos | Plugin; image proxy behind signed URLs | TV photos tab; albums; memories |
| LAT-114 | Read-only folder photo library | Point at a NAS folder and browse without importing | Plex and Jellyfin: folders; Immich: external libraries, where edits fail silently (27) | Medium: 27, plus Plex's photo regressions | Later | Never writes to the folder; EXIF and XMP parsed in the core; no machine learning, so it runs on small boxes (Immich needs 6 GB RAM, or 4 GB with machine learning off) | Photo scan; metadata index | Photos tab; grid; viewer |
| LAT-115 | Thumbnails in a sandbox | Fast grids without exposing the server to image-decoder bugs | Immich: an SVG upload reached ImageMagick for remote code execution (Sept 2026) | Medium: 2026 image-handling advisories at Immich and Jellyfin | Later | Pixel decoding runs in a sandboxed worker like FFmpeg (needs an ADR amendment); the core never decodes pixels; thumbnails are rebuildable cache | Sandboxed thumbnail job; cache | None |
| LAT-116 | Photo viewer | Swipe, zoom and an info panel with camera details | Immich: yes; Plex: Photos app rated 1.3 of 5, no update since June 2025; Jellyfin (unverified) | Medium: Plex's app ratings | Later | One viewer on every client, TV included | EXIF fields | Viewer; info panel |
| LAT-117 | HEIC and HEIF | iPhone photos open everywhere | Plex: requested since 2017 (637, open); Immich: yes, but HDR HEIC previews look flat (13) | High: 637 | Later | Original bytes to clients that decode HEIC natively; a sandboxed conversion for the rest | Format detection; sandboxed conversion | None |
| LAT-118 | RAW files | Camera RAWs show up | Immich: yes, some DNGs fail (32, 112 comments); Plex: CR3 requested since 2018 (26) | Medium: 32 and 26 | Later | Shows the preview a RAW file embeds where it has one (coverage per format unverified), decoded in the sandbox | RAW container parsing; preview extraction | Grid; viewer |
| LAT-119 | Embedded keywords | Lightroom keywords show up and can be searched | Plex: requested since 2016 (283); Jellyfin: tag search requested (14); Immich: tags on web | High: 283 | Later | XMP keywords parsed in the core with hardened XML | Keyword index | Tag filter; search |
| LAT-120 | Live and motion photos | The short clip behind a still plays back | Immich: yes; Plex (unverified); Jellyfin: no (unverified) | Low: no counts found | Later | The core finds the appended MP4, or the `mpvd` box in HEIC and AVIF, without decoding, and sends the clip as original bytes | Motion-photo detection | Press and hold to play |
| LAT-121 | Photo timeline | Fly through 20 years by date | Immich: yes; Plex: Photos app timeline; Jellyfin: no (unverified) | Medium: Immich sets the bar | Later | The synced library makes scrubbing local and instant | Date index | Timeline with scrubber |
| LAT-122 | Photo folder view | Browse folders as you made them | Immich, Plex and Jellyfin (12.0): yes | Low: every rival has it | Later | Parity | Folder tree | Folder browser |
| LAT-123 | Albums of photos and clips | Curated sets, such as one trip, mixing photos and videos | Immich: yes; Plex: yes (detail unverified); Jellyfin: collections (unverified) | Medium: a basic expectation | Later | Albums are user-log events, so they survive any rebuild | Album events | Album pages |
| LAT-124 | Shared albums | Family sees and adds to the same album | Immich: yes, but sharing changes are frozen pending a redesign (749, 260 comments); Plex: library-level; Jellyfin: per-user library access | High: 749 | Later | Per-object authorisation on every photo; parity at best, since Immich shows how hard sharing is to get right | Album permissions | Share sheet; shared album view |
| LAT-125 | Public album links | Send an album to someone without an account, with expiry and an optional password | Immich: yes, with password and download switch; Plex: no (unverified); Jellyfin: requested (580) | High: 580 | Later | Same capability links as LAT-109 | Share capabilities | Share sheet; guest gallery |
| LAT-126 | Partner sharing | Two people see each other's whole photo library | Immich: yes; Plex and Jellyfin: no | Low: no counts found | Later | A household grant over both libraries | Library grants | Settings |
| LAT-127 | Slideshow with music on the TV | Photos on the living-room screen, with a playlist underneath | Plex: "slideshow for all clients" requested (121); Jellyfin: slideshow with delay (12.0), with music requested (5); Immich: mobile slideshow (3.0.0) | Medium: 121 | Later | Gunmetal's music player runs under the slideshow on the same TV client | Album or memory feed | Slideshow controls; music picker |
| LAT-128 | Photo frame and ambient mode | An idle TV cycles an album | Jellyfin: requested (12); nobody ships it | Low: 12 | Later | Idle mode obeys content limits and surface rules (LAT-014, LAT-111) | Album feed | Screensaver settings |
| LAT-129 | Memories | "Five years ago today" | Immich: yes; Plex: Recommended view; Jellyfin: no | Low: no counts found | Later | Computed locally from capture dates; nothing leaves the server | Date queries | Home row |
| LAT-130 | Map of photos and clips | See where photos and videos were taken | Immich: yes; Plex: launched Places in 2017, since withdrawn; Jellyfin: no | Low: no counts found | Later | GPS is read in the core; map tiles come from outside, so through a plugin with a grant | GPS index; tile plugin | Map view |
| LAT-131 | Stacks | RAW and JPEG, or a burst, shown as one | Immich: yes, "stacks on all views" requested (98); Plex and Jellyfin: no | Medium: 98 | Later | Grouped at scan by file name and capture time; Immich plans automatic burst stacking, so parity at best | Stack grouping | Stack badge; expand |
| LAT-132 | Rotate and crop without touching the original | Fix a sideways photo | Immich: editing on web (2.5.0) and mobile (3.0.0); Plex: rotate requested (9) | Low: 9 | Later | Stored as a user-log overlay; the file is never edited | Overlay events | Viewer edit menu |
| LAT-133 | Favourites, archive and hidden | Star the good ones, hide screenshots | Immich: yes; Jellyfin: favourites; Plex (unverified) | Medium: a basic expectation | Later | User-log events | Events | Viewer actions; filters |
| LAT-134 | Download originals | Get the full-resolution file back | Plex: requested since 2014 (38); Immich: yes | Low: 38 | Later | The original is what Gunmetal serves anyway, behind a signed URL | Signed download | Download action |
| LAT-135 | Duplicate report | Find copies of the same photo | Immich: resolution (2.7.0); Plex and Jellyfin: no | Low: no counts found | Later | Report only; Gunmetal never deletes from a read-only folder | Content-hash index | Duplicates report |
| LAT-136 | 360-degree photos | Pan around a panorama | Immich: web only; Plex and Jellyfin: no | Low: no counts found | Later | Parity at best | Projection metadata | Panorama viewer |
| LAT-137 | Phone photo and video backup | Photos and clips leave the phone by themselves, then free up space | Immich: yes (freeing space since 2.5.0); Emby: camera upload; Plex: historic (current status unverified) | High: Immich's popularity (115,500 GitHub stars) | No | Immich does this well; owning irreplaceable originals needs backup guarantees outside Gunmetal's read-only, rebuildable design | — | — |
| LAT-138 | Face recognition on the server | Every photo of grandma | Immich: yes; Plex: never shipped (60 votes, plus a closed thread with 110 likes); Jellyfin: requested (15) | Medium: 60 and 110 | No | Needs machine learning (Immich: 6 GB RAM with it on); revisit only on client devices (feasibility unverified) or through Immich | — | — |
| LAT-139 | Smart search and text in images | Type "dog", or a receipt's words, and find the photo | Immich: CLIP search and OCR; Plex: auto-tagging launched in 2016, since withdrawn; Jellyfin: requested (13) | Low: 13 | No | Same reason as LAT-138 | — | — |
| LAT-140 | Editing suite, storage templates and automation | Immich-style editing, upload folder layouts and workflows | Immich: yes (workflows in preview, 3.0.0) | Low: no counts found | No | Gunmetal does not manage photo files; that is the photo manager's job | — | — |

### Ebooks

Rivals: Kavita, Komga and Jellyfin 12, with ABS and Booklore in notes.
Plex's most-voted request ever is a reader (3,182 votes), but Kavita and
Komga are good. Protocols (OPDS, KOReader sync, later Kobo sync) deliver most
of the value with little client work, so they come first and Gunmetal's own
readers come last, if at all.

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LAT-141 | OPDS catalogue | E-reader apps browse and download your books | Komga: OPDS 1.2 and 2 with page streaming; Kavita: OPDS; Jellyfin: no (unverified); ABS: requested (73) | High: Plex reader request (3,182); ABS 73 | Later | The catalogue is built from the same index and every entry passes per-object authorisation; readers use scoped, revocable per-device keys because they cannot do passkeys (unverified per app) | OPDS 1.2 and 2 feeds; device keys; signed download URLs | Device keys page; QR code to add a reader |
| LAT-142 | KOReader progress sync | Progress shared with KOReader devices, to the paragraph | Kavita: yes; Komga: yes, but EPUB progress only at chapter starts; Jellyfin: no (unverified) | Medium: Komga documents the lost positions | Later | Stores KOReader's progress string and percentage plus a Readium locator, so a position is never cut back to a chapter start | Sync endpoint; index of KOReader's MD5-based document hashes; locator events | Device keys page |
| LAT-143 | Kobo sync | A Kobo shows your library as if it were the store | Komga: yes (EPUB converted to KEPUB, chapter-level progress); Kavita: planned (unverified); ABS: its most-upvoted request (258) | High: 258 | Later | Only after the protocol and its legal position are checked (unverified); KEPUB copies would be cache files, never written into the library | Kobo endpoints; conversion job | Device setup |
| LAT-144 | Ebook formats | EPUB, PDF, MOBI and AZW3 appear with covers | Jellyfin: the widest list; Kavita and Komga: EPUB and PDF | Medium: Plex reader request (3,182) | Later | EPUB parsed in the core; other formats served as original bytes | Format detection; cover extraction | Books grid |
| LAT-145 | Embedded ebook metadata | Title, author and series from the book's OPF | Kavita, Komga and Jellyfin (12.0): yes | Medium: every rival has it | Later | Hardened XML in the core: no external entities, no entity expansion | OPF parsing | Book page |
| LAT-146 | Free online book metadata | Covers and blurbs fetched | Kavita: paid (Kavita+); Jellyfin: free (Google Books, Open Library, ComicVine) | Medium: Kavita charges for it | Later | A first-party plugin with a grant, never a paid tier | Plugin | Match dialog |
| LAT-147 | Series and reading order | Books in order, with gaps visible | Kavita and Komga: yes; Jellyfin: from filenames (12.0) | Medium: every rival has it | Later | LAT-003 | Series ordering | Series page |
| LAT-148 | Reading progress and sessions | Progress, reading sessions and re-read counts | Kavita: since 0.8.9; Komga: progress | Medium: a basic expectation | Later | Locator events in the user log | Position events | Book page |
| LAT-149 | Want-to-read shelf | A wishlist of books | Kavita: yes; Komga (unverified); Jellyfin: no | Low: no counts found | Later | User-log events | Events | Shelf |
| LAT-150 | Web EPUB reader | Read in a browser | Kavita: overhauled (0.8.8); Komga: yes; Jellyfin: redesigned (12.0) | Medium: Plex reader request | Later | The lowest-priority reader; engine choice is open (likely WebView-based, unverified); positions written as locators | None beyond LAT-148 | Reader |
| LAT-151 | Phone ebook reader | Read in the Gunmetal phone app | Jellyfin: official apps (unverified); third-party OPDS readers | Low: good OPDS readers already exist | Later | Built only if OPDS readers prove not enough | None | Reader |
| LAT-152 | Typography controls | Font, size, margins and themes | Kavita: custom fonts; Jellyfin: font size | Low: no counts found | Later | Parity | None | Reader settings |
| LAT-153 | Highlights and annotations | Mark and note passages | Kavita: since 0.8.8; Booklore: yes; ABS: requested (25, 33) | Medium: 25 and 33 | Later | User-log events with locator ranges, exportable | Annotation events | Reader; notes page |
| LAT-154 | Send to Kindle or email | One tap sends a book to a device | ABS and Booklore: yes; Kavita (unverified) | Low: no counts found | Later | A plugin with a mail grant | Plugin | Send button |
| LAT-155 | Reading statistics | Time read and a shareable profile | Kavita: since 0.8.9 | Low: no counts found | Later | Computed from the local user log | Aggregation job | Stats screen |
| LAT-156 | Smart filters and collections | Saved searches and collections | Kavita: yes; Komga and Jellyfin: collections | Low: every rival has it | Later | Same rule engine as music smart playlists | Rule definitions | Filter builder |
| LAT-157 | Content limits in book protocols | Age limits hold in OPDS and sync, not just in the app | Komga: enforced for Kobo (1.25.0); Kavita: yes (detail unverified) | Medium: Komga added it deliberately | Later | LAT-014 applied to the OPDS, KOReader and Kobo endpoints, each in the per-endpoint authorisation test matrix | Shared limit filter | Profile settings |
| LAT-158 | Download a whole series | Take a series offline | Kavita: multi-download (0.9.0); Jellyfin: request (1) | Low: 1 | Later | Same download engine as music | Download manifests | Series page |
| LAT-159 | Reading scrobbles | Progress posted to Hardcover or AniList | Kavita: paid (Kavita+) | Low: no counts found | Later | A free plugin, with each user's secrets kept per user | Plugin | Account links |
| LAT-160 | PDFs open on your device | PDFs read in the device's own viewer | No data | Low: no counts found | Later | Original bytes are sent; the server never renders PDFs | Signed download | Open in viewer |
| LAT-161 | Server-side PDF rendering | The server turns PDF pages into images | No data | Low: no counts found | No | A large, risky parser; the device renders PDFs | — | — |
| LAT-162 | Ebook reader on TV | Read books on the TV | Jellyfin: books library on TV (unverified) | Low: little use on a TV | No | Of little use on TV; the protocols serve reading devices | — | — |
| LAT-163 | DRM-protected ebooks | Open store-bought DRM books | No data | Low: no counts found | No | Supporting DRM removal is a legal risk | — | — |

### Comics and manga

Rivals: Kavita, Komga and Jellyfin 12. Comics fit "send the original"
unusually well: a CBZ page is a stored file inside a ZIP archive and can be
sent without decoding.

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LAT-164 | CBZ, CBT and CB7 archives | Common comic archives open | Kavita: yes (CB7 unverified); Komga: CBZ and CBR; Jellyfin: cb7, cbr, cbt, cbz | Medium: Plex reader request (3,182) | Later | CBZ and CBT read by the core with limits on entry count and size; CB7 needs a decompressor (maturity unverified) | Archive index | Comics grid |
| LAT-165 | CBR archives | RAR-based comics open | Komga: RAR5 in plain Java (1.26.0), solid RAR4 (1.25.0); Kavita and Jellyfin: yes | Medium: all three rivals support it | Later | A sandboxed decoder, or a lower tier: the reference unrar code is C under a restrictive licence (details unverified) and a mature pure-Rust decoder is unverified | Sandboxed extraction; page cache | None |
| LAT-166 | Page streaming | Apps fetch single pages without downloading the whole archive | Kavita and Komga: through OPDS; Jellyfin: no (unverified) | Medium: both specialists ship it | Later | Pages are sent as stored bytes with no decoding, matching "send the original" | Page endpoint behind signed URLs | None (third-party apps and LAT-171) |
| LAT-167 | ComicInfo metadata | Writers, artists and issue numbers from the file | Kavita and Komga: yes; Jellyfin: native (12.0), with artist gaps (5) and a ComicInfo request (14) | Medium: 14 | Later | Hardened XML in the core | ComicInfo parsing | Issue page |
| LAT-168 | Volume and chapter numbers | "Vol 2 Ch 14.5" sorts right | Kavita: yes, with open edge-case issues; Komga: yes; Jellyfin: since 12.0 (order request, 4) | Medium: Kavita's open edge cases | Later | One pure core function under the full gate (100% coverage, zero surviving mutants), with every reported edge case kept as a test | Number parser | Series page |
| LAT-169 | Page counts | Know how long an issue is | Kavita, Komga and Jellyfin (12.0): yes | Low: every rival has it | Later | Counted from the archive directory at scan, without extracting | Archive index | Issue tile |
| LAT-170 | Covers | A cover for every issue | All three: yes; Kavita+ full-size covers cannot be reclaimed (open issue) | Low: every rival has it | Later | The first page is sent as is when it is already JPEG or PNG; resizing runs in the sandbox; covers are rebuildable cache | Thumbnail job | Grid |
| LAT-171 | Comic reader modes | Single page, double page and right-to-left | Kavita: yes; Komga (detail unverified); Jellyfin: unified reader, richer reader started (44) | Medium: 44 | Later | A reader for tablets, phones and web, fed by LAT-166 | None | Reader |
| LAT-172 | Webtoon scroll | Long strips scroll smoothly | Kavita: improved (0.9.0); Jellyfin: requested (12) | Low: 12 | Later | Pages streamed in order as original bytes | None | Reader mode |
| LAT-173 | Pinch zoom | Read small lettering on a phone | ABS app: lacks it (29); others (unverified) | Low: 29 | Later | Parity | None | Reader gesture |
| LAT-174 | Reading lists and CBL import | Follow a crossover event in order | Kavita: CBL import; Komga: read lists; Jellyfin: no | Low: no counts found | Later | Lists are user-log events; CBL files parsed in the core | List events | Reading list page |
| LAT-175 | Online comic and manga metadata | Covers and credits from comic databases | Kavita: paid (Kavita+: MangaBaka, ComicBookRoundup); Jellyfin: ComicVine | Low: no counts found | Later | A free plugin with a grant | Plugin | Match dialog |

## Differentiators

These are the features in this area most likely to make someone switch.

1. **Audiobooks in the music app, in a real store app (LAT-015 to LAT-038,
   LAT-050; Later, after live TV as ADR 2 orders it).** Plex users have asked for audiobooks since 2013 (2,264 votes);
   Jellyfin's request has 857 votes and its maintainers call the support
   basic. ABS is excellent but its iOS app has sat in a capped TestFlight
   beta for years, and CarPlay is its top app request. Many people run
   Jellyfin or Plex for video, Navidrome for music and ABS for spoken word
   side by side. Gunmetal can collapse that into one server and one app,
   because chapters come out of the same pure-Rust parsers as music tags and
   the book player reuses gapless playback, offline downloads, lock-screen
   controls and car support that music already needs.
2. **Books, podcasts and music that do not fight over one queue (LAT-009,
   LAT-039, LAT-067).** No server keeps separate resumable contexts today.
   ABS's queue request has 99 upvotes, Jellyfin commenters ask for a book
   resume that does not hijack music, and only one client app (Symfonium)
   does it. Because the queue protocol carries contexts from R1, this is a
   UI feature by the time books ship, not a migration.
3. **Home videos the family can watch remotely for free, with no setup, and
   share by expiring link (LAT-108, LAT-109; Later).** Plex has charged for remote
   personal video since 29 April 2025. Jellyfin is free but expects users to
   run their own VPN or reverse proxy, and its share-link request (580) is
   still open. Gunmetal's iroh remote access and signed, scoped, expiring
   capabilities answer both, and the grandparents need no account.
4. **Progress you own, down to the sentence (LAT-006, LAT-007, LAT-142,
   LAT-053).** Komga's KOReader and Kobo sync lose mid-chapter positions,
   Kavita encodes progress in OPDS titles, and Jellyfin lost watch state on
   file replacement in 10.11. One typed user log keeps time offsets, text
   locators and pages together, survives every upgrade, and exports to OPML,
   KOReader progress and JSON.
5. **Photos on the TV, with music (LAT-113, LAT-127).** Plex has retreated
   from photos: its standalone app is rated 1.3 out of 5 and photo libraries
   disappeared from its main iOS and Fire TV apps in August 2026 (staff said
   they would return). Gunmetal
   does not try to out-build Immich; it puts Immich albums and memories on
   the living-room TV with a playlist underneath, which nobody does well.
6. **Podcasts with no paywall, no account and your own archive (LAT-064,
   LAT-079, LAT-080, LAT-084).** Plex dropped podcasts and Jellyfin's
   request has 563 votes. Pocket Casts is free and excellent for basics but
   charges for bookmarks and folders and needs an account. This pulls in
   people who already self-host, not Pocket Casts loyalists, and the map
   says so.

## Deliberately not doing

- **Writing to library files (LAT-046, LAT-112).** Gunmetal reads media
  folders and never writes to them. Merging to M4B, embedding tags and
  copying media between drives are jobs for other tools. Fixes to chapters,
  metadata and photo orientation are user-log overlays instead (LAT-019,
  LAT-022, LAT-132).
- **DRM (LAT-052, LAT-163).** Supporting removal of Audible or ebook DRM is a
  legal risk for an open project.
- **Server-side machine learning (LAT-076, LAT-138, LAT-139).** Generated
  chapters, face recognition, smart search and OCR need models that Immich
  sizes at 6 GB of RAM. That contradicts the low-hardware promise. Speech
  recognition for read-along and transcripts (LAT-048, LAT-049) stays Later
  and only as an opt-in, sandboxed plugin or on the client.
- **Photo management (LAT-137, LAT-140).** Phone backup, freeing phone
  space, editing suites, storage templates and automation belong to Immich,
  which does them well. Owning irreplaceable originals also needs backup
  guarantees that a read-only, rebuildable design does not make.
- **First-party Audible-derived data (LAT-020).** ASIN chapter lookup and
  Audible metadata carry terms-of-service risk. Third-party plugins can
  offer them; the project does not ship them.
- **Server-side PDF rendering (LAT-161).** It is a large, risky parser; the
  device renders PDFs from the original bytes.
- **An ebook reader on TV (LAT-162).** It has little use, and the protocols
  serve real reading devices.
- **Paid tiers for any of this.** Kavita charges for metadata and
  scrobbling, and Pocket Casts for bookmarks and folders. Under AGPL with no
  central account there is nothing to gate, and every feature in this map
  is free.

## Dependencies and risks

- **Blocking: ADR 3, durable user state.** ADR 1 (decision 5) names watch
  history as the only irreplaceable data, but most rows in this map that
  write user data (playlists, loves, ratings, corrections, layouts, shares,
  identities, IDs, settings) need it to survive a cache rebuild. ADR 3 must
  be accepted before any server work that stores user data. It defines the
  two durable stores every map now names the same way: **the user log** and
  **the identity store** (see the [feature map README](README.md)).
- **Native mobile clients, not the remuxer, are what audiobooks wait for.**
  Books are a phone and car medium, so LAT-015 to LAT-057 need the native
  mobile app with background playback, lock-screen controls, downloads and
  car support. They are now Later, because ADR 2 orders M3U and live TV
  before books; bringing them forward with the first mobile client would
  need an ADR that changes that order (open decision 1).
- **The user log has to be typed from R1 (LAT-006, LAT-007).** If R1 ships a
  music-only user log and later types add tables only in SQLite, the
  rebuildable-cache promise quietly breaks and every new type needs a
  migration, which is exactly the failure Jellyfin 10.11 and 12.0 show.
- **A plugin host with network grants.** Online metadata (LAT-022, LAT-025,
  LAT-097, LAT-146, LAT-175), the ABS importer (LAT-054), podcasts (LAT-058
  onward), Immich (LAT-113), map tiles (LAT-130) and send-to-device (LAT-154)
  all need it. Record 2 requires plugins with explicit grants but no ADR yet
  defines the runtime. The ecosystem research recommends a WebAssembly
  sandbox with declared grants, following Navidrome. LAT-022 and LAT-054
  make this an R2 dependency.
- **One safe outbound fetcher.** R3's M3U and EPG sources need an
  SSRF-guarded HTTP client (Jellyfin's M3U tuner had an SSRF advisory in
  2026), and podcasts need the same thing (LAT-059). Build it once for R3
  and podcasts follow it.
- **A sandbox for untrusted decoders.** Record 1 sandboxes only FFmpeg.
  Thumbnails (LAT-115), HEIC conversion (LAT-117), RAW previews (LAT-118),
  CBR and CB7 (LAT-164, LAT-165) and cover resizing (LAT-170) all decode
  untrusted bytes with large codebases. Immich's 2026 SVG-to-ImageMagick
  remote code execution is the warning. This needs an ADR amendment before
  any photo or comic work.
- **XML everywhere.** OPF, ComicInfo, NFO, RSS, XMP and CBL are all XML. One
  hardened parser in the core must refuse external entities and entity
  expansion, and be fuzzed like the media parsers.
- **Archives.** CBZ needs limits on entry count and expanded size. The RAR
  licence and the maturity of pure-Rust RAR and 7z decoders are unverified;
  CBR may have to be a lower tier.
- **Long-lived credentials for outside apps.** OPDS readers, KOReader, Kobo,
  podcast apps and private book feeds (LAT-045, LAT-141 to LAT-143, LAT-088,
  LAT-089) cannot use passkeys or short-lived signed URLs. They need
  per-device keys scoped to one protocol, revocable, with expiry. Immich's
  CVE-2026-23896 (a key raising its own permissions) and Navidrome's
  never-expiring Jellyfin-API tokens are the mistakes to avoid.
- **Authorisation surface grows with every type.** ABS fixed IDOR bugs in
  progress and session endpoints in 2026, Kavita shipped a critical hotfix
  in May 2026, and Navidrome's share endpoints trusted client input. Every
  new endpoint, including OPDS and sync, goes into the per-endpoint
  authorisation test matrix.
- **Position stability.** Re-ripped audiobooks change durations, a replaced
  EPUB changes its KOReader hash, and edited EPUBs can invalidate locators.
  LAT-053 covers audio; text needs its own recovery rule before LAT-142
  ships.
- **Outbound privacy.** Fetching podcasts exposes the server's IP address to
  publishers and their analytics prefixes; fetching from the phone exposes
  each device instead.
- **Metadata legality.** Audible- and Amazon-derived data reaches users
  through unofficial channels. Keeping it in third-party plugins limits the
  project's exposure but does not remove it.
- **Platform gaps.** Pitch-preserving speed on every TV browser, trim
  silence and volume boost through libmpv on every platform, and Web Audio
  effects on the web are all unverified. App Store review of the iOS app is
  not guaranteed.
- **Timezones.** Phone photos and videos often carry local time without an
  offset, so ordering a trip across timezones can go wrong (unverified for
  current phones).
- **Scope and people.** This map has 175 features against a strict
  test-first gate. ABS's single maintainer has kept the iOS app in beta for
  years, Booklore entered maintenance mode, and Jellyfin's leader stepped
  down from burnout in July 2026. Only the R1 and R2 rows should be planned
  now.
- **Rivals are moving.** Jellyfin 12.0 improved books, comics and
  audiobooks; ABS ships steadily (v2.37.1 on 29 Sept 2026); Immich is very
  active. Parity claims here will age.

## Open decisions for the project owner

1. **When audiobooks ship (LAT-015 to LAT-057).** Options: in R1 (web only),
   with R2, or as their own release alongside the first native mobile app.
   *Decided in the feature map README:* R1 carries only the schema-only
   foundations (LAT-001, LAT-002's role column, LAT-006 to LAT-009, and a
   simple LAT-010 folder flag). The audiobook experience (LAT-015 to
   LAT-057), lectures (LAT-056) and the home video library (LAT-104 to
   LAT-111) are Later, because ADR 2 puts M3U and live TV first and books
   after. Pulling books ahead of live TV would need an ADR that changes that
   order.
2. **Spoken word as a library type or an item kind (LAT-010, LAT-056).**
   *Recommendation:* a kind on each item, defaulted by library and
   overridable per folder, so lectures, audio dramas and courses get book
   behaviour without being called books.
3. **Read-only libraries for good (LAT-019, LAT-046, LAT-112, LAT-132).**
   *Recommendation:* yes. Never write to media folders; keep fixes as
   user-log overlays; any export writes new files to a separate export folder
   on explicit request.
4. **Long-lived credentials for outside apps (LAT-045, LAT-088, LAT-089,
   LAT-141 to LAT-143).** *Recommendation:* per-device keys scoped to one
   protocol, read-only where possible, listed and revocable by the user and
   expiring; never the main session token. Private book feeds stay off by
   default and warn when switched on.
5. **Audible- and Amazon-derived metadata (LAT-020, LAT-022).**
   *Recommendation:* no first-party plugin uses these sources. Third-party
   plugins may, and the plugin page shows where data comes from. Take legal
   advice before any first-party use.
6. **Who fetches podcasts (LAT-058, LAT-064, LAT-065).** Server fetching
   exposes the server's IP address; client fetching exposes each device's.
   *Recommendation:* the server fetches feeds, archiving is opt-in per show,
   and playback goes through the server by default so phones stay private;
   streaming straight from the publisher is a per-show choice.
7. **Podcast sync protocol (LAT-088, LAT-089).** *Recommendation:*
   OpenSubsonic first, because that adapter is already planned. Evaluate
   gPodder sync or the Open Podcast API only when podcasts ship, and pick at
   most one.
8. **How far photos go (LAT-113, LAT-114, LAT-137 to LAT-140).**
   *Recommendation:* the Immich plugin first and the read-only folder
   library second, both Later. Formally rule out phone backup and
   server-side machine learning so the low-hardware promise holds.
9. **Gunmetal's own readers (LAT-150, LAT-151, LAT-171).**
   *Recommendation:* protocols first (LAT-141, LAT-142, LAT-166). A comic
   reader for tablets and the web is the most likely first reader. Build an
   ebook reader only if users show that OPDS readers fall short, and never
   on TV.
10. **Kobo sync (LAT-143).** It is ABS's top request (258) and Komga shows
    it can be done, but its legal position is unverified.
    *Recommendation:* commission a protocol and legal review before any
    code.
11. **Audiobookshelf: importer or adapter (LAT-054, LAT-055).**
    *Recommendation:* an importer with the book release; no adapter, since
    ABS says its API is still being standardised.
12. **A sandbox for untrusted decoders.** *Recommendation:* write an ADR
    that defines one sandboxed decoder worker (images, RAW, RAR, 7z) next to
    the FFmpeg sandbox, before any photo or comic work starts.
13. **Machine learning (LAT-048, LAT-049, LAT-076, LAT-138, LAT-139).**
    *Recommendation:* never a server requirement. It is allowed only as an
    opt-in, sandboxed plugin or on client devices, and not before R2 ships.
14. **Video plays as song plays (LAT-101).** *Recommendation:* count them
    locally when a mapped chapter is watched past the same threshold as
    audio, and never scrobble video plays unless the user opts in.
15. **Position conflicts and "finished" (LAT-027, LAT-029).**
    *Recommendation:* the latest report wins by default; the player asks
    when two recent positions differ by more than a chapter. "Finished" is
    an explicit mark or reaching the final minutes of the last chapter,
    never a single seek to the end.
16. **An interim answer for podcasts (LAT-058).** Podcasts are Navidrome's
    most +1'd open request and Jellyfin's request has 563 votes.
    *Recommendation:* until the podcast plugin exists, document running
    Audiobookshelf beside Gunmetal; OpenSubsonic podcast endpoints (LAT-089)
    are the later route through existing apps.
