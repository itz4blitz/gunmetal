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
only what is specific to each type. The Security column lists the
requirements in [docs/security](../security/README.md) that a builder must
satisfy for each row; a dash means none beyond the baseline's general
rules. [Security notes](#security-notes) at the end name the threats that
matter most for this area.

Releases (R1, R1.1, R1.2, R1.3, R2, R3, Later, No), the Demand scale, row
ownership and the terms "the user log" and "the identity store" are
defined once in the [feature map README](README.md). A row whose Release
cell would differ between maps names one owning row; the other maps point
at it.

### Shared foundations

These rows are the cheap insurance the research asks for. Only
schema-only parts ship before the later types: R1 carries LAT-001, LAT-006,
LAT-007 and LAT-009, and R1.3 adds LAT-002's typed roles, LAT-008's relation
table and a simple LAT-010 folder flag, so later types arrive without a
migration. New parsers (LAT-003 to LAT-005) wait for the types that need
them, because no parser is cheap under the 100% mutation gate. LAT-010
(R1.3) and LAT-011 (through MUS-078, R2) are the only rows users would
notice before the later types ship.

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| LAT-001 | Item kinds in the data model | New media types arrive later without a schema migration or a full rescan | Jellyfin: separate library types including books, photos and music videos; Plex: no audiobook type | High: fragile upgrades are a top pain point (Jellyfin 10.11 migration failures, #15027, 121 comments) | R1 | Every item carries a kind (track, audiobook, podcast episode, video, photo, book, comic) from the first schema, so SQLite stays a rebuildable cache across releases | Kind field on items; versioned kind enum in the core protocol types | None in R1 | SEC-API-024, SEC-TM-051, SEC-OPS-048, SEC-OPS-049 |
| LAT-002 | People with typed roles | Browse by author, narrator, host, guest, writer or penciller, not only "artist" | ABS: several narrators per book; Plex: narrator only by tag convention; Jellyfin: authors tab (12.0) | Medium: Plex users rely on tag conventions for narrators | R1.3 | One person record with a role on each credit, shared with music's composer and performer roles, so a narrator who also records music is one person | R1.3: the later kinds' roles (author, narrator, host, guest, writer, penciller) on the role column of credits, which music's roles use from R1.1 (MUS-005). OPF and ComicInfo role mapping arrives with books. | None in R1.3; person pages from R2 | SEC-TM-031, SEC-API-048, SEC-TM-036, SEC-TM-039 |
| LAT-003 | Series with a free-form sequence | Books numbered "1.5", "0" or "Prequel" sort correctly in their series | ABS: sequence numbers; Kavita and Komga: yes; Jellyfin: series from filenames (12.0) | Medium: ABS missing-books request (26); Jellyfin series request (4, Sept 2026) | Later | The sequence is kept as the original string plus a sort key computed by one tested core function Arrives with books; nothing in R1 needs it. | Series and membership tables; sort-key function | None in R1 | SEC-MED-001, SEC-TM-032, SEC-API-048 |
| LAT-004 | One item spanning many files | A 40-file rip is one book with one timeline | ABS: yes; Plex: no; Jellyfin: one folder per book, with requests to handle split books better | Medium: Jellyfin commenters (2023) | Later | An item is an ordered manifest of files with durations read at scan, giving one global timeline; positions are global milliseconds Arrives with audiobooks; R1 multi-file items are albums. | Manifest table; per-file duration and gapless data from the core parsers | None in R1 | SEC-API-018, SEC-MED-012, SEC-TM-043, SEC-MED-014 |
| LAT-005 | Chapters as a shared structure | A chapter list and chapter skip for any audio file that has chapters, such as a DJ mix or a live set | Plex and Plexamp: ignore M4B chapters; Jellyfin: chapter extraction (12.0); ABS: yes | Medium: Plex chapter complaints; Jellyfin auto-advance request (26) | Later | Chapters parsed at scan in pure Rust (MP4 `chpl` and QuickTime chapter tracks, ID3v2 `CHAP` and `CTOC`, Vorbis comments, Matroska through the existing EBML code), never by FFmpeg; one structure (start, end, title, image, link) serves books, podcasts, concerts and films. A chapter image is re-encoded like any artwork, and a chapter link is kept as inert text, shown as a link only after it parses as http or https, and never fetched. Later, with the media types that need chapters: new parsers are not cheap under the 100% mutation gate, and R1 music does not need them. | Chapter table keyed to item, file and offset; parser fuzzing | Chapter list in now playing; chapter ticks on the seek bar | SEC-TM-032, SEC-MED-027, SEC-MED-075, SEC-MED-016, SEC-MED-058, SEC-TM-035 |
| LAT-006 | Typed positions in the user log | Your place is kept for audio, text and pages alike, and survives any rebuild | Komga: KOReader and Kobo sync keep only chapter starts; Kavita: encodes OPDS progress in titles | Medium: ABS read-along requests (91, 55) depend on it | R1 | The user log from record 1 stores a position as a time offset, a Readium-style text locator, a page of a total, or a percentage, and each position event carries device and time and nothing else (no address, location or free text). The log is append-only for ordinary writes, but a person can erase their own events, and the erasure reaches devices as tombstones that name only the erased IDs | Typed position events; event schema versioning | None beyond music resume in R1 | SEC-PRV-002, SEC-PRV-001, SEC-PRV-024, SEC-PRV-049, SEC-PRV-052 |
| LAT-007 | All user-made data in the user log | Bookmarks, notes, finished dates, subscriptions, albums and hidden items survive upgrades and can be exported | Immich: its database backup excludes the photos themselves; Jellyfin 12.0: one-way upgrade that needs a backup first | High: fragile upgrades are a top pain point | R1 | Every user-made fact is a user-log event, never only a SQLite row, so "rebuild from files plus user log" stays true. Each person exports or erases their own events without an admin; exports use OPML, KOReader progress or JSON, need a sign-in within the last 5 minutes, and download once from a link that expires within an hour | Event types added per kind as each ships; export, import and erasure jobs | Export page in settings | SEC-PRV-047, SEC-PRV-048, SEC-PRV-049, SEC-TM-055, SEC-PRV-001 |
| LAT-008 | Typed links between items | A song links to its video, an ebook to its audiobook, an interview to its artist | Plex: links videos to tracks by file name; Jellyfin: no; read-along needs a separate app (Storyteller) | Medium: Jellyfin requests (38, 9); ABS read-along (91) | R1.3 | A relation table with typed edges (video of, audio of, extra for) in the schema from R1.3, before any kind that uses it; scanners and users fill it later; a link is shown only to someone who may see both items | R1.3: the relation table only. Inference jobs per kind arrive with each kind. | None in R1.3 | SEC-TM-024, SEC-IAM-070, SEC-MED-051 |
| LAT-009 | Listening contexts in the queue protocol | Nothing visible in R1; later, a book and an album each keep their own queue and place | ABS: no; Jellyfin: one music queue; Symfonium (a client): several queues | Medium: ABS queue request (99) | R1 | The cross-device queue from record 2 carries named contexts from its first version, so LAT-039 needs no protocol break | Context id on queue state and hand-off messages | None in R1 | SEC-HIS-014, SEC-API-016, SEC-API-043 |
| LAT-010 | Spoken word kept out of music | Audiobooks found in a music library stay out of shuffles, radio and album grids | Plex: no book type, so books live in music libraries; Spotify: "hide podcasts" idea has 8,815 votes | Medium: Spotify 8,815 votes; a Jellyfin commenter wants a book resume that does not hijack music | R1.3 | R1.3 keeps this to a simple per-folder flag set by the admin, which music surfaces filter on; automatic classification of M4B files and audiobook tags arrives with audiobooks. | Per-folder kind flag | Library settings toggle; a plain "Spoken word" list in R1.3 | SEC-API-019, SEC-TM-027 |
| LAT-011 | Long audio remembers its place | See MUS-078, which owns this feature. Later-media specifics: a resume threshold per kind. | Plexamp: resumes long audio; Jellyfin: partial | Medium: discovery research; Jellyfin chapter auto-advance request (26) | R2 | See MUS-078. | Per-kind resume threshold; otherwise none beyond MUS-078 | Resume prompt in the player; Continue row | SEC-PRV-002, SEC-PRV-024 |
| LAT-012 | Choose which media kinds you see | Someone who only wants music never sees books or podcasts | Spotify: no (8,815-vote idea, under consideration); Apple: podcasts live in a separate app | High: Spotify 8,815 and 5,223 votes | R2 | Visible kinds are a per-user setting applied when the device's synced library is built, so on a device used by one person hidden kinds are not even downloaded | Per-user kind filter in the sync feed | Settings toggle; navigation and Home adapt | SEC-API-015, SEC-CLI-020, SEC-CLI-015 |
| LAT-013 | Kind chips in search and library | One search over songs, books and episodes, narrowed with a tap | Spotify: chips for music and podcasts; Jellyfin: separate libraries | Medium: music UX research | R2 | Search runs on the synced library on the device, so it works offline across kinds | Kind facet in the synced index | Search chips; library chips | SEC-PRV-004, SEC-CLI-020, SEC-API-015 |
| LAT-014 | Content limits that know each kind | Children see children's books but not explicit ones, and unrated home videos are handled on purpose | ABS: explicit content denied to new users by default (2.14.0); Jellyfin: blocks unrated items per media type; Komga: limits enforced for Kobo too | Medium: users and security research | R2 | Limits apply when the synced library is built and on every signed URL, so restricted items never reach the device and no adapter or protocol can bypass them | Per-user rules by kind, rating and explicit flag, applied to sync, adapters and URLs | Profile and parental controls | SEC-IAM-064, SEC-TM-026, SEC-API-014, SEC-API-028, SEC-TM-051 |

### Privacy and safety across types

These rows carry the security baseline's user-visible promises to the types
in this map. LAT-176 to LAT-178 apply accounts features to these types and
point at their owners; LAT-181 is specific to the formats here that need a
native decoder. All are Later, because they arrive with the types they
protect.

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| LAT-176 | Keys for reading and podcast apps | See ACC-129, which owns per-app credentials. Later-media specifics: each OPDS reader, KOReader device, podcast sync app and private book feed gets its own key, which you can see and end | Immich: a key could raise its own permissions (CVE-2026-23896); Navidrome: Jellyfin-API tokens that never expired | Medium: every outside-app row in this map depends on it | Later | See ACC-129. Keys are generated by the server, shown once, scoped read-only to one protocol, sent in a header and never in a URL or QR code, expire with a warning 14 days before, are listed with last use and a coarse address, and stop working on the next request when revoked; the first use from a new address appears in the person's own security log | None beyond ACC-129, plus one credential kind per protocol | Account > Apps; key shown once; expiry notice | SEC-IAM-083, SEC-EXT-006, SEC-EXT-007, SEC-EXT-013, SEC-EXT-014, SEC-EXT-017 |
| LAT-177 | Private sessions for books and podcasts | See ACC-117, which owns private sessions. Later-media specifics: a private session keeps your place only on that device | None among servers (see ACC-117) | Medium: see ACC-117 | Later | See ACC-117. For kinds that keep a place, a private session holds the position on the device only and writes no position, bookmark, finished or statistics event to the user log; nothing reaches a scrobbling plugin; leaving private mode asks whether to keep the place | None beyond ACC-117 | Player > Private session; "Keep my place?" prompt | SEC-PRV-024, SEC-TM-054, SEC-EXT-030, SEC-PRV-035 |
| LAT-178 | What admins see of books, podcasts and photos | See ACC-115, which owns the page. Later-media specifics: reading progress, annotations, subscriptions, albums and photo favourites stay private from admins | Kavita: shareable reading profiles; Plex: "Week in Review" exposed listening history (2023) | Medium: see ACC-115 | Later | See ACC-115. Book positions, finished dates, bookmarks, annotations, subscriptions, reading statistics, albums and photo favourites are classed as activity data, so admin views show live sessions without titles and totals only, and the what-admins-can-see page lists each kind as it ships | None beyond ACC-115; an activity class on each new event type | Account > What admins can see | SEC-PRV-025, SEC-PRV-022, SEC-PRV-027, SEC-IAM-104, SEC-PRV-001, SEC-HIS-060 |
| LAT-181 | Formats that need the jail say why they are off | Where the server cannot isolate a native decoder, HEIC photos, RAW previews and CBR and CB7 comics stay off and the admin is told why | Immich: an SVG upload reached ImageMagick for remote code execution (Sept 2026) | Low: no counts found; required by the security baseline | Later | Formats whose decoding needs native code run only in the full jail. Where the jail cannot be built, they are switched off, the library health view lists the affected files with the reason and what would turn them on, and no setting runs them unconfined | Jail self-test per decoder; library health entries | Admin > Library health; doctor report | SEC-TM-045, SEC-MED-024, SEC-TM-044, SEC-MED-025 |

### Audiobooks

Rivals: ABS (the purpose-built server), Plex with Plexamp or the
third-party Prologue app, and Jellyfin 12. The full book experience ships
with the native mobile clients in R2; see the first open decision.

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| LAT-015 | Audiobook library | Books behave as books, with covers, authors, narrators, series and progress, not as albums by an "artist" | ABS: yes; Plex: no (request open since 2013); Jellyfin: partial ("Books" type) | High: Plex 2,264 votes; Jellyfin 857 ("started"); ABS used by 840 of 4,081 selfh.st respondents | Later | Same app, server and synced library as music, so books browse offline and need no second server; built on LAT-001 to LAT-011 | Audiobook kind; grouping by folder and tags | Books tab; book page; book mode in now playing | SEC-TM-024, SEC-IAM-070, SEC-TM-026, SEC-MED-011 |
| LAT-016 | Multi-file books play as one | One progress bar and no gaps across 40 files | ABS: yes; Plex: no; Jellyfin: partial | Medium: Jellyfin commenters (2023) | Later | Music's gapless engine plays across the LAT-004 manifest | Manifest; durations; gapless data | One seek bar and time remaining for the whole book | SEC-API-018, SEC-API-026, SEC-MED-012 |
| LAT-017 | Chapter navigation | Next and previous chapter, a chapter list, and automatic advance | ABS: yes; Plex and Plexamp: ignore M4B chapters (Prologue on iOS reads them); Jellyfin: extracts chapters, but one 2026 guide says its web player has no chapter skip | High: Plex chapter complaints; Jellyfin auto-advance (26) | Later | LAT-005 chapters are mapped to file and offset, so skipping works across multi-file books | Chapter-to-file mapping | Chapter buttons; chapter sheet; lock-screen chapter skip | SEC-MED-057, SEC-TM-036 |
| LAT-018 | Chapter-scaled seek bar | Switch the seek bar between the whole book and the current chapter | No data | Low: no requests found | Later | Parity | Chapter boundaries in the synced item | Seek bar toggle | SEC-MED-014 |
| LAT-019 | Chapter fixes without touching files | Rename, add or move chapters when a rip has bad ones | ABS: chapter editor; Plex: no; Jellyfin: no (unverified) | Low: no counts found | Later | Fixes are user-log overlays on the parsed chapters, so files stay untouched and fixes survive rescans | Chapter overlay events | Chapter editor (web) | SEC-TM-042, SEC-MED-038, SEC-TM-024, SEC-TM-027, SEC-API-048 |
| LAT-020 | Chapter lookup by Audible ASIN | Correct chapters fetched from a community database, with Audible branding trimmed | ABS: yes (2.22.0); Plex: no; Jellyfin: no (unverified) | Low: no counts found | No | Not first-party: the data is Audible-derived and carries terms-of-service risk; the plugin interface would not stop a third party offering it | — | — | None (No) |
| LAT-021 | Metadata from embedded tags and OPF | Title, author, narrator, series and cover filled from the files, with no internet | ABS: yes; Jellyfin: embedded tags and OPF; Plex: read as music tracks | Medium: a July 2026 review found Jellyfin could not match MP3 audiobooks online | Later | The core parses MP4, ID3 and Vorbis tags and OPF sidecars at scan; its XML parser rejects any document with a DOCTYPE and limits size, depth and element count, URLs inside the files are never fetched, and covers are re-encoded | Tag-to-book field mapping; OPF parser | Book page showing the source of each field | SEC-MED-056, SEC-HIS-034, SEC-TM-038, SEC-MED-016, SEC-TM-035 |
| LAT-022 | Online book metadata through plugins | Covers, blurbs and series filled in when the files lack them | ABS: Audible, Google Books, iTunes, Open Library (partly unverified) and custom providers; Plex: third-party Audnexus agent; Jellyfin: Google Books, Open Library | Medium: Jellyfin's failed MP3 matching | Later | First-party plugins use open sources under an explicit network grant (record 2), off until the owner turns each one on; a request carries only typed lookup fields (title, author, series, ISBN), never paths or file names; results are user-log overlays tagged by source, so a bad match is undone in one step | Plugin host; match queue; overlay events | Match dialog; unmatched list; per-field lock | SEC-PRV-013, SEC-PRV-014, SEC-PRV-015, SEC-EXT-026, SEC-EXT-027, SEC-EXT-028 |
| LAT-023 | Author and narrator pages | Everything one narrator has read; several narrators per book | ABS: since 1.7.0; Plex: tag conventions; Jellyfin: authors tab (12.0), narrators (unverified) | Medium: ABS ships it as a core feature | Later | Typed roles from LAT-002 | Credits by role | Person page with role tabs | SEC-TM-024, SEC-TM-036 |
| LAT-024 | Series in listening order | Book 3 follows book 2, and the series page shows where you are | ABS: yes; Jellyfin: series from filenames (12.0) | Medium: Jellyfin series request (4); ABS series features | Later | LAT-003 sort key; the next unfinished book is computed from the user log | Series ordering; next-in-series query | Series page; "Next in series" after finishing | SEC-PRV-022, SEC-TM-024 |
| LAT-025 | Missing books in a series | Gaps in a series are shown | ABS: requested (26) | Low: 26 | Later | Needs an outside catalogue, so it comes from a metadata plugin; nothing is fetched without the grant | Series catalogue from a plugin | Placeholder tiles on the series page | SEC-PRV-014, SEC-PRV-015, SEC-EXT-026 |
| LAT-026 | Resume across devices | Start on the phone, finish on the TV or the web | ABS, Plex (reliable per a 2026 review) and Jellyfin: yes | High: a basic expectation in every rival | Later | Parity | Position events; merge rule from LAT-027 | Resume button; Continue listening row | SEC-PRV-002, SEC-PRV-022, SEC-API-016, SEC-PRV-024 |
| LAT-027 | Position conflicts handled openly | When two devices disagree you choose, instead of silently losing an hour | No data | Low: raised as an open question, no votes | Later | User-log events carry device and time; when recent positions on two devices differ by more than a chapter, the player asks | Divergence check over position events | Resume prompt naming each device; position history with undo | SEC-PRV-002, SEC-PRV-022, SEC-PRV-049 |
| LAT-028 | Rewind on resume | After a pause the book backs up a few seconds | Symfonium: yes; others: no data | Low: clients research only | Later | Parity | None (synced client setting) | Player setting | SEC-TM-027, SEC-API-067 |
| LAT-029 | Finished books with dates | A finished shelf with start and finish dates you can edit | ABS: yes, editable dates requested (30); Plex and Jellyfin: played flag (unverified) | Medium: 30 | Later | Started, finished and re-listened are user-log events, editable and exportable | Finished events; per-kind finished rule | Finished shelf; date editor; Mark finished | SEC-PRV-022, SEC-PRV-047, SEC-PRV-049 |
| LAT-030 | Set a book aside | Drop an abandoned book from Continue listening without marking it finished | No data for books; Jellyfin's video version is its second most-wanted request (1,725) | Medium: 1,725 for video, by analogy | Later | A set-aside user-log event, separate from finished | Set-aside events | Row long-press menu | SEC-PRV-022, SEC-PRV-049 |
| LAT-031 | Speed with pitch kept | 1.5x without chipmunk voices, remembered per book | ABS and Plexamp: yes; Plex: 1,298-vote speed request implemented; Jellyfin: depends on client | High: 1,298 | Later | Parity (libmpv on native clients; the browser's pitch-preserving rate on web, unverified on every TV) | Per-item playback settings in the user log | Speed control | SEC-TM-027, SEC-API-067 |
| LAT-032 | Skip intervals you choose | Jump back 30 seconds after a distraction, at lengths you set | ABS: yes (intervals unverified); Plexamp (unverified) | Medium: Plex's custom skip-length request (453, for video) | Later | Parity | Synced client setting | Skip buttons; lock-screen and headphone skips | SEC-TM-027, SEC-API-067 |
| LAT-033 | Chapter-aware sleep timer | Stop after a time, at the end of the chapter, or after N chapters | ABS: times and end of chapter (2.12.0); Plexamp: yes; PC: after N chapters on Android; Jellyfin: episode-count request (11) | Medium: 11, and every specialist ships it | Later | One sleep timer shared with music, chapter-aware through LAT-005 | None | Sleep timer sheet; countdown in now playing | None specific: a client timer, shared with MUS-076 |
| LAT-034 | Bookmarks with notes | Mark a passage and come back to it | ABS: yes; Plex and Jellyfin: no (unverified); PC: paid | Medium: PC charges for it | Later | Free; bookmarks are private user-log events and exportable. They reach OpenSubsonic apps only through the person's own app key, and only once the adapter's endpoint allowlist and grant include bookmarks, which they do not yet | Bookmark events | Bookmark button; bookmark list per book | SEC-PRV-022, SEC-TM-036, SEC-API-048, SEC-EXT-055, SEC-EXT-056 |
| LAT-035 | Bookmark from lock screen or headphones | Save the moment without unlocking the phone | PC: yes (paid); ABS: no data | Low: no counts found | Later | A native media-session action writes the bookmark offline and syncs it later | Bookmark events | Lock-screen action; headphone gesture setting | SEC-CLI-054, SEC-CLI-055, SEC-IAM-050 |
| LAT-036 | Offline books | Download a book and listen on a plane | ABS: yes; Plexamp downloads (unverified); Jellyfin: varies (offline is its top request, 1,820) | High: 1,820 | Later | Same download engine as music, under offline grants that expire and are withdrawn when the device is revoked; the original, or Opus to save space (cheap, per record 2, encoded in the transcode jail); book metadata is already on the device | Download manifests; optional Opus job | Download button; downloads screen; storage use | SEC-IAM-054, SEC-CLI-036, SEC-CLI-035, SEC-TM-060, SEC-TM-044 |
| LAT-037 | Lock-screen and background playback | Lock the phone and keep listening, with skip-back controls | ABS and Plexamp: yes; Jellyfin: iOS background audiobook playback only since 12.0 | High: Jellyfin users reported audio stopping on lock | Later | Parity | None | Lock screen; notification controls | SEC-CLI-054, SEC-CLI-055 |
| LAT-038 | Android Auto and CarPlay for books | Resume the book from the car screen, offline | ABS: Android Auto yes, CarPlay is its top app request (100); Jellyfin: audiobooks in Android Auto (Android 2.7.0); Plexamp (unverified) | High: 100 | Later | Android Auto for books: parity with Audiobookshelf, with the edge that books and music share one app with separate queues (LAT-039). CarPlay for books would put Gunmetal ahead of Audiobookshelf, whose top request is CarPlay, but it waits for the Apple builds. Both are Later, with audiobooks (ADR 2 puts live TV first). | Car browse tree | CarPlay and Android Auto screens | SEC-CLI-055, SEC-CLI-054 |
| LAT-039 | A book never hijacks the music queue | Play an album mid-book; the book waits at its place, and Resume means the book | ABS: no (queue request 99); Jellyfin: one queue; Symfonium (a client): several queues | Medium: 99, plus Jellyfin commenters | Later | Built on LAT-009: each context keeps its own queue and position and hands off between devices; no server does this today | Context-aware queue sync | Context switcher in now playing; Resume book widget and shortcut | SEC-HIS-014, SEC-API-016 |
| LAT-040 | Casting books | Send a book to a Chromecast speaker | ABS: since 1.7.0; Plex and Jellyfin (unverified) | Low: no counts found | Later | Parity, with a narrower credential: the receiver gets a capability URL for one item and one cast session, never an account or device token. The sender refreshes it before it expires, so a long book needs the sender in contact at least every 4 hours, and revoking the sender's session ends the cast | Cast capability URLs scoped to one item and one cast session, refreshed by the sender | Cast button | SEC-NET-064, SEC-CLI-071, SEC-API-027, SEC-API-098, SEC-HIS-042 |
| LAT-041 | Personal ratings for books | Rate books for yourself and sort by it | ABS app: self-rating requested (75) | Medium: 75 | Later | Ratings are user-log events, shared with music ratings | Rating events | Rating control; sort option | SEC-PRV-022, SEC-PRV-047 |
| LAT-042 | Book collections | Group books by theme | ABS: yes; Plex: playlists; Jellyfin: collections tab (12.0) | Low: every rival has it | Later | Parity | Collection events in the user log | Collections page | SEC-TM-024, SEC-API-014, SEC-MED-051 |
| LAT-043 | Listening statistics | Time listened, books finished, a year in review | ABS: stats and Year in Review (2.7.0); Plex and Jellyfin: no | Low: no requests found | Later | Computed from the local user log and shown only to that person; admins see totals across the server, never one person's figures; nothing leaves the server. Uses the DIS-183 statistics engine. | Aggregation job | Stats screen | SEC-PRV-022, SEC-PRV-025, SEC-PRV-024 |
| LAT-044 | Share a book by expiring link | Send one book to a friend, with an end date | ABS: admin-only links with expiry (2.11.0); Jellyfin: share-link request open (580) | Medium: 580 across all media | Later | A share link under the music rules (ACC-086): the secret travels in the URL fragment, the link plays one book with no download unless the owner allows downloads server-wide, expires (30 days by default), allows two streams at once, suspends itself and tells the sharer when it spreads, and is re-checked on every request (Navidrome's shared streams outlived deletion until Sept 2026); the page never shows the sharer's name | Share capabilities; per-link limits; revocation | Share sheet; My shares | SEC-API-097, SEC-HIS-042, SEC-TM-028, SEC-PRV-031, SEC-PRV-055 |
| LAT-045 | Private podcast feed for a book | Listen to a book in a podcast app that can send a password with a feed | ABS: yes, including MRSS (2.18.1); Plex and Jellyfin: no | Low: no counts found | Later | Off by default and switched on per book by the person sharing it. The feed and its episode files accept only a key the server generates for that one feed, sent by the app as an HTTP Basic password in the Authorization header over HTTPS, never in the URL path or query. The key is read-only, expires, is listed with the person's other app keys (LAT-176) and stops working on the next request when revoked. Apps that can only hold a secret URL are not supported (header support per app unverified) | Feed generation; per-feed app keys (ACC-129) | Share sheet option with a warning; key shown once | SEC-IAM-083, SEC-EXT-006, SEC-EXT-007, SEC-EXT-013, SEC-EXT-014, SEC-NET-001 |
| LAT-046 | Merge and embed tools | Turn an MP3 folder into one tagged M4B file | ABS: yes | Low: no counts found | No | Gunmetal never writes to library files; fixes live as user-log overlays (LAT-019, LAT-022) | — | — | SEC-TM-042, SEC-MED-038 |
| LAT-047 | Switch between ebook and audiobook by chapter | Stop reading at chapter 12 and carry on listening at chapter 12 | ABS: no (requests 91 and 55); Jellyfin: not supported; Storyteller: a separate app | Medium: 91 and 55 | Later | LAT-008 links the two editions; matching chapter titles map a text locator to a time | Chapter mapping between linked items | Continue in audio; Continue in text | SEC-TM-024, SEC-IAM-070 |
| LAT-048 | Sentence-level read-along | The text highlights as the narrator reads | Storyteller: yes (EPUB 3 media overlays); others: no | Medium: the same requests (91, 55) | Later | Plays EPUB 3 media overlays that already exist, parsed by the same XML parser as OPF; creating them needs speech recognition, so only on a client device or as an opt-in plugin with no network grant inside the standard plugin limits, never in the server process | Media-overlay parsing; optional alignment plugin | Reader with audio highlighting | SEC-TM-038, SEC-MED-056, SEC-EXT-023, SEC-EXT-030, SEC-TM-044 |
| LAT-049 | Generated transcripts for books | Read or search what was said | ABS: requested (Whisper 44, transcripts 27) | Low: 44 and 27 | Later | Opt-in and sandboxed, ideally on a client device; never a server requirement | Optional job; transcript storage | Transcript panel | SEC-EXT-023, SEC-EXT-030, SEC-TM-044, SEC-TM-036 |
| LAT-050 | Books in a store-listed iOS app | Install from the App Store, not a capped beta | ABS: TestFlight only (10,000-tester cap, no date); Prologue: third-party; Jellyfin: official app (book quality unverified) | High: ABS's release checklist issue has 96 comments | Later | Later; R2 if the App Store licence decision allows (open decision 3 in the feature map README). Books ride in the same native iOS app as music; App Store approval is not guaranteed | None | iOS app | SEC-TM-059, SEC-CLI-017, SEC-SUP-060 |
| LAT-051 | Audiobook formats | M4B, M4A, MP3, FLAC, Opus, Ogg and MKA all play | ABS: broad (Opus in Matroska fixed in 2.33.2); Plex: M4B plays; Jellyfin: broad | Medium: ABS was still fixing format gaps in 2026 | Later | Parity in coverage; parsed by the core without FFmpeg and sent as original bytes | Shared audio parsers | None | SEC-MED-001, SEC-MED-011, SEC-MED-012, SEC-TM-033 |
| LAT-052 | DRM-protected audiobooks | Play Audible AAX files directly | Native support in any rival (unverified) | Low: no counts found | No | Supporting DRM removal is a legal risk for an open project | — | — | None (No) |
| LAT-053 | Progress survives replaced files | Re-ripping or upgrading a book keeps your place | Jellyfin 10.11: lost watch state on replace (#15001, 61 +1); others: no data | Medium: 61 | Later | Positions are keyed to content identity; when the total duration changes, the position maps by chapter title and proportion, and the old value stays in the user log | Remap job on file change | "Your place was adjusted" notice with undo | SEC-TM-069, SEC-PRV-002 |
| LAT-054 | Import from Audiobookshelf | Progress, finished dates and bookmarks come across in one step | No importer found in the research | Medium: ABS has 840 selfh.st users, so switching cost matters | Later | Each person imports their own progress, finished dates and bookmarks with their own ABS API key (keys since 2.26.0), which is used for that import and then discarded; nobody, an admin included, can import another person's history. The server reaches ABS through the egress client only at the exact host and port the owner grants (plain HTTP only for that LAN address) and decodes replies into typed, size-capped structures. Imported events are tagged by source and can be undone | Import job; source-tagged events; an import purpose in the egress inventory | Import wizard in account settings | SEC-PRV-025, SEC-PRV-022, SEC-API-079, SEC-EXT-004, SEC-API-081, SEC-TM-075 |
| LAT-055 | Audiobookshelf-compatible API | ABS's own apps connect to Gunmetal | None | Low: no requests found | Later | Not recommended for scheduling: ABS says its API is still being standardised, and adapters inherit protocol weaknesses | Adapter endpoints | None (third-party apps) | SEC-IAM-084, SEC-TM-070, SEC-EXT-051, SEC-EXT-055 |
| LAT-056 | Lectures, courses and audio dramas | Spoken word that is not a book still gets speed, resume, chapters and bookmarks | No data | Low: raised as an open question | Later | Book behaviour belongs to the kind, so a folder can opt in without pretending a lecture is a book | Kind per folder | Library settings | SEC-API-019, SEC-TM-027 |
| LAT-057 | Books on TV clients | Listen to a book on the living-room TV | Jellyfin: Android TV audiobook request (5); ABS: no data | Low: 5 | Later | Same synced library and player on the TV | None | TV book shelf and player | SEC-CLI-063, SEC-IAM-064, SEC-IAM-109 |

### Podcasts

Rivals: PC (the reference app), ABS (the self-hosted podcast server people
use), and the video servers, none of which supports podcasts today: Plex
removed them in April 2022 and Jellyfin has none. Everything here is Later
and depends on the plugin host and the egress client; the server contacts
only the hosts the owner approved for each show (LAT-180; see Dependencies
and risks). Pocket Casts is free and excellent; the case for Gunmetal is no
account, your own archive, and one app for music, books and podcasts.

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| LAT-058 | Podcasts as a plugin with a network grant | Podcasts, without the server reaching the internet unless the admin allows it | ABS: built in; Plex: removed (April 2022, citing low usage); Jellyfin: none | High: Jellyfin podcast support 563 votes; podcasts are Navidrome's most +1'd open request | Later | Feed fetching is a plugin with its own plugin interface, defined by an architecture record. Its grant names exact hosts, so each show's hosts join the grant only when the owner approves them (LAT-180), and nothing is fetched before that; feed parsing is pure logic in the core. Interim answer until then: OpenSubsonic podcast endpoints through LAT-089, or Audiobookshelf beside Gunmetal (open decision). | Plugin host; podcast plugin interface; per-show exact-host grants | Admin plugin page; podcast host approvals (LAT-180) | SEC-EXT-026, SEC-TM-017, SEC-EXT-076, SEC-HIS-024, SEC-EXT-039, SEC-TM-075 |
| LAT-059 | Safe feed fetching | A hostile feed cannot make the server call your router or other internal addresses | ABS: SSRF filtering for downloads (2.34.0); PC: not applicable | Medium: ABS's 2026 fix; Navidrome's 2026 SSRF advisories | Later | Feeds go through the server's one egress client, the same one every outbound purpose uses: it resolves names itself, refuses private, loopback, link-local and metadata addresses, connects only to the address it checked, re-checks each redirect hop against the show's approved hosts (three hops at most), and caps size and time | None beyond the egress client; the plugin host has no HTTP client of its own | Fetch errors on the show page | SEC-EXT-001, SEC-API-076, SEC-EXT-002, SEC-EXT-003, SEC-EXT-004, SEC-API-077 |
| LAT-060 | Directory search and subscribe | Find a show by name and follow it | PC: yes; ABS: iTunes search with region (2.10.0) | Medium: Jellyfin 563 | Later | The search query leaves the server only when the person searches, only to the directory named on the screen, through the plugin's grant, and is never stored | Directory search through the plugin | Podcast search screen | SEC-PRV-004, SEC-PRV-015, SEC-EXT-005, SEC-EXT-026 |
| LAT-061 | Subscribe by feed URL | Add any feed by its address | PC and ABS: yes (unverified) | Medium: the basic way to add a show | Later | Subscriptions are user-log events. A pasted address is never fetched as a side effect of the request: it becomes a request to approve that feed's host (LAT-180), and the first fetch waits for the owner. Feeds served only over plain HTTP are refused | Subscription events; host-approval queue | Add by URL; waiting-for-approval state | SEC-API-080, SEC-HIS-024, SEC-TM-017, SEC-EXT-026, SEC-API-078 |
| LAT-062 | OPML import and export | Move subscriptions in or out in one file | PC: yes (unverified); ABS: import since 2.0.18, export (unverified) | Medium: the standard switching path | Later | Export is always available because subscriptions are user-log events; an imported file is read by the XML parser that rejects any DOCTYPE, and each feed it names joins the host-approval queue instead of being fetched | OPML reader and writer in the core | Import and export in settings | SEC-MED-056, SEC-TM-038, SEC-API-085, SEC-API-088, SEC-PRV-047, SEC-API-080 |
| LAT-063 | Podcasting 2.0 metadata | Seasons, episode numbers, people, GUIDs and funding links appear | PC: funding tag and podroll; ABS: no data | Low: no counts found | Later | RSS and Podcasting 2.0 tags parsed in the core by the XML parser that rejects any DOCTYPE; the plugin only does the I/O. Funding and people links show as links only after they parse as http or https, and their images are fetched only from approved hosts and re-encoded | Feed parser | Show and episode pages; funding link | SEC-MED-056, SEC-HIS-034, SEC-MED-058, SEC-API-047, SEC-MED-016 |
| LAT-064 | Server archive of episodes | Episodes stay after the publisher deletes them | ABS: yes; PC: no | Medium: ABS ships it and a cloud app cannot | Later | Archived episodes are written to a podcast archive folder with a quota, separate from the read-only media folders and the server's data directory, named by random IDs rather than names from the feed, and indexed by the core like library files; per-show retention rules delete only from that folder | Download and retention jobs; archive folder with a quota | Per-show archive setting; storage view | SEC-TM-042, SEC-OPS-054, SEC-MED-039, SEC-TM-068, SEC-MED-012 |
| LAT-065 | Stream without archiving | Play an episode without archiving it on the server | PC: yes; ABS: no data | Low: no counts found | Later | The server fetches the episode from an approved host into a temporary cache with a size cap and an expiry, parses it like any library file and serves it. Fetching happens on the show's schedule or when the person taps Fetch, which names the publisher; browsing and pressing play never start one. Clients never contact the publisher, so phones never reveal their address to it | Temporary episode cache with a quota and expiry; fetch queue | Per-show archive or temporary setting; Fetch button | SEC-CLI-044, SEC-CLI-048, SEC-CLI-049, SEC-PRV-015, SEC-HIS-024 |
| LAT-066 | Keep the latest N episodes on the phone | The commute is ready without thinking | PC: yes; ABS: requested (46) | Medium: 46 | Later | Rule-based downloads shared with music | Download rules | Per-show download rule | SEC-IAM-054, SEC-CLI-036, SEC-CLI-035 |
| LAT-067 | Up Next across shows and books | Line up episodes from several shows, and a book | PC: yes; ABS: requested (99) | Medium: 99 | Later | The spoken-word context from LAT-009 holds episodes and books together, apart from music | Context queue | Up Next screen | SEC-HIS-014, SEC-API-016 |
| LAT-068 | Rule-based episode lists | "Unplayed, under 30 minutes" | PC: filters; ABS: no (unverified) | Low: no counts found | Later | Reuses the music smart-playlist rules, evaluated on the device's synced library | Rule definitions in the user log | Filter builder | SEC-TM-039, SEC-STD-011 |
| LAT-069 | Played state and auto-archive | Finished episodes leave the list | PC: auto-archive rules; ABS: requested (31) | Medium: 31 | Later | Played and archived are user-log events | Events; archive rules | Swipe actions; per-show rule | SEC-PRV-022, SEC-PRV-049 |
| LAT-070 | Oldest-first per show | Serial shows play in order | PC: per-show sort (unverified); ABS: requested (35) | Medium: 35 | Later | Parity | Per-show sort setting | Sort control on the show page | SEC-TM-027, SEC-API-067 |
| LAT-071 | Speed per show | 1.8x for one show, 1.0x for another | PC: global or per show, 0.5x to 3x on mobile and up to 5x on web; ABS: per show no (unverified) | Medium: a PC headline feature | Later | Parity; per-show settings live in the user log and apply on every device | Per-show settings | Effects sheet | SEC-TM-027, SEC-API-067 |
| LAT-072 | Trim silence | Shorter pauses without changing the voice | PC: three levels, mobile only; ABS: no (unverified) | Medium: a PC headline feature | Later | libmpv audio filters on native clients (unverified per platform); the web needs Web Audio work, so mobile first, as with PC | None | Effects sheet | None specific: client-side audio filters |
| LAT-073 | Volume boost | Quiet hosts become audible; works for books too | PC: mobile; ABS: requested (26) | Low: 26 | Later | Same filter path as LAT-072, for all spoken word | None | Effects sheet | None specific: client-side audio filters |
| LAT-074 | Skip intro and outro per show | Theme music and fixed openers are skipped | PC: yes (unverified); ABS: no (unverified) | Low: no counts found | Later | Parity | Per-show settings | Effects sheet | SEC-TM-027, SEC-API-067 |
| LAT-075 | Episode chapters | Jump to the segment you want | PC: MP3 chapters, Podcast Index JSON and Podlove; ABS: since 2.2.21 | Medium: both specialists ship it | Later | LAT-005 parsers plus Podcast Index JSON chapters in the core; chapter files come only from approved hosts and are decoded into typed, size-capped structures, and chapter images are re-encoded | JSON chapter fetch through the plugin | Chapter sheet | SEC-EXT-027, SEC-API-081, SEC-HIS-024, SEC-TM-035, SEC-MED-058 |
| LAT-076 | Generated chapters | Chapters for shows that publish none | PC: offered but switched off at the time of writing | Low: no counts found | No | Needs machine learning on the server, against the low-hardware goal | — | — | None (No) |
| LAT-077 | Feed transcripts | Read along with an episode | PC: yes, from the Podcasting 2.0 tag (2.5 million episodes); ABS: requested (27) | Medium: 27, plus PC's coverage | Later | The transcript tag is parsed in the core; the file is fetched only from approved hosts, reduced by the core to timed plain text (from SRT, WebVTT, JSON or HTML), and shown as text in time with playback | Transcript storage | Transcript panel | SEC-EXT-027, SEC-MED-052, SEC-TM-036, SEC-API-081 |
| LAT-078 | Search inside transcripts | Find the episode where a topic came up | PC: search within an episode | Low: no counts found | Later | Indexed on your server across all episodes; no outside service | Transcript index | Search results with timestamps | SEC-PRV-004, SEC-IAM-070, SEC-TM-039 |
| LAT-079 | Free episode bookmarks | Save a moment in an episode | PC: paid (Plus or Patron); ABS: for episodes (unverified) | Medium: PC charges for it | Later | Free user-log events, shared with LAT-034 | Bookmark events | Bookmark button | SEC-PRV-022, SEC-PRV-047 |
| LAT-080 | Free folders for shows | Group subscriptions | PC: paid, with smart folders; ABS: no | Low: no counts found | Later | Free user-log events | Folder events | Folder management | SEC-PRV-022 |
| LAT-081 | Episode sleep timer | Stop after this episode or after N episodes | PC: many modes, mobile only; ABS: time and end of chapter; Jellyfin: episode-count request (11) | Medium: 11 | Later | Same timer as LAT-033, on every client | None | Sleep timer sheet | None specific: a client timer, shared with MUS-076 |
| LAT-082 | Video podcasts | Watch shows that publish video | PC: yes; ABS: audio focus (unverified); Jellyfin: one commenter asked | Low: one comment | Later | Plays through the R2 video path; LAT-099 lets you just listen | None extra | Episode page | SEC-CLI-049, SEC-TM-047, SEC-MED-081 |
| LAT-083 | Your own recordings in Up Next | A lecture you recorded sits in the same queue as episodes | PC: files cannot go into playlists; ABS: it is your server already | Low: no counts found | Later | Any spoken-word item can join the spoken-word context | None | Add to Up Next | SEC-TM-024, SEC-HIS-014 |
| LAT-084 | Sync with no account | Same place on phone and web, free, with no third-party account | PC: free but needs an account; ABS: yes | Medium: privacy is the second most common reason to self-host (selfh.st) | Later | Your server is the account, and the user log syncs over iroh | None extra | None | SEC-TM-063, SEC-TM-064, SEC-IAM-050 |
| LAT-085 | Full player on web and desktop | Sleep timer and effects at a desk | PC: web limited to speed effects, sleep timer mobile only | Low: no counts found | Later | Parity at first: the player code is shared, but web effects wait on Web Audio work | None | Web player | SEC-API-044, SEC-API-049 |
| LAT-086 | Podcast statistics | Time listened and time saved by effects | PC: yes; ABS: library stats | Low: no counts found | Later | Computed from the local user log and shown only to that person | Aggregation job | Stats screen | SEC-PRV-022, SEC-PRV-025 |
| LAT-087 | Episode artwork | Each episode shows its own image | PC: yes (unverified); ABS: requested (42) | Medium: 42 | Later | Images are fetched only from approved hosts, re-encoded by the server's image pipeline, kept as rebuildable cache and served from the server; clients never fetch them from the publisher | Image fetch; thumbnail job | Episode rows | SEC-EXT-027, SEC-TM-035, SEC-MED-046, SEC-PRV-016 |
| LAT-088 | Open podcast sync protocol | AntennaPod-style apps sync with Gunmetal | Nobody; gPodder sync and the Open Podcast API exist | Low: one Jellyfin commenter | Later | Pick one protocol after checking its support (details unverified). It is an adapter: off by default, signed in with its own per-app key in a header, never the account's sign-in, with no account-creation endpoint | Sync endpoints | None (third-party apps) | SEC-IAM-084, SEC-TM-070, SEC-EXT-051, SEC-EXT-055, SEC-EXT-006 |
| LAT-089 | Podcasts through OpenSubsonic | Existing Subsonic apps see your podcasts | ABS: no (unverified); Navidrome: podcast request (43) | Low: 43 | Later | Comes with the planned adapter: OpenSubsonic defines eight podcast endpoints. Reading and playing work through each person's app key; creating a channel from an app adds a request to the owner's host-approval queue (LAT-180) rather than fetching, and deleting follows LAT-090's permissions | Adapter endpoints | None (third-party apps) | SEC-EXT-055, SEC-EXT-054, SEC-EXT-056, SEC-API-080, SEC-TM-070 |
| LAT-090 | Who may manage podcasts | Only chosen users add or delete shows; subscriptions can be personal or household | ABS: management permissions requested (43) | Low: 43 | Later | Per-object authorisation on every show and subscription. A personal subscription list is that person's activity, hidden from admins and other users; a new show whose hosts are not yet approved waits for the owner (LAT-180) | Permissions per show | User permissions in admin | SEC-IAM-074, SEC-TM-024, SEC-PRV-022, SEC-PRV-025 |
| LAT-180 | Podcast hosts the owner approves | The server reaches only podcast hosts the owner approved, and the owner can see which ones it contacted | ABS: SSRF filtering for downloads (2.34.0); PC: not applicable | Low: no counts found; required by the security baseline | Later | When a show is added, its feed host and the hosts its episodes, images, chapters and transcripts use are listed for the owner, who approves or refuses them with a fresh fingerprint or face check; members' new shows wait until then; a redirect to a host not yet approved stops that fetch and adds the host to the queue; the outbound ledger shows each host and purpose, never full URLs | Approval queue; per-show exact-host grants in the egress client; a podcast purpose in the egress inventory | Admin > Podcast hosts; "Waiting for approval" on the show | SEC-TM-017, SEC-EXT-026, SEC-EXT-003, SEC-PRV-008, SEC-TM-075, SEC-API-079 |

### Music videos and concerts

Rivals: Plex, Jellyfin and Emby. Plex is good here; the aim is to match it,
beat Jellyfin, and stay free. All of it needs the R2 video path; only the
relationships (LAT-008) exist before it, from R1.3.

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| LAT-091 | Music video on the song | The song and the now-playing screen offer its video | Plex: yes, by file name next to the track; Jellyfin: no (9); Plexamp: does not show them (2026 request) | Medium: Jellyfin 38 and 9 | R2 | A LAT-008 link filled by Plex's naming rule (easy migration) or by hand | Link inference by name and folder; manual link events | Video badge on track rows; Watch video in now playing | SEC-TM-024, SEC-IAM-070, SEC-MED-051 |
| LAT-092 | Typed artist extras | Live sets, concerts, interviews, behind-the-scenes and lyric videos on the artist page | Plex: yes, by file-name suffix; Jellyfin: no; Emby (unverified) | Medium: Jellyfin users ask to combine music and video (38) | R2 | Plex's suffixes (`-live`, `-concert`, `-interview`, `-behindthescenes`, `-lyrics`, `-video`) are accepted and stored as "extra for" links | Suffix parsing in the core | Videos shelf on the artist page, with a type filter | SEC-TM-031, SEC-MED-039 |
| LAT-093 | Music video folders per library | Keep videos apart from audio files | Plex: one global path for the server; Jellyfin: its own library type | Low: no counts found | R2 | Per-library video folders feed the same artist model | Library roots for music video | Library settings | SEC-TM-017, SEC-MED-037, SEC-API-022 |
| LAT-094 | One artist page for songs and videos | No second library to browse | Plex: yes; Jellyfin: requested since 2020 (38) | Medium: 38 | R2 | Parity with Plex, through the same music model | Artist query across kinds | Artist page | SEC-IAM-070, SEC-TM-026 |
| LAT-095 | Browse music videos | By artist, year and genre | Plex: through the music library; Jellyfin: requested (15) | Low: 15 | R2 | Parity | Facets in the synced index | Music videos view | SEC-API-015, SEC-CLI-020 |
| LAT-096 | Music video metadata from files | Titles, artists and dates from tags and NFO files | Jellyfin: NFO only, nothing online by default; Plex (unverified) | Medium: Jellyfin documents NFO as the only route | R2 | The core reads MP4 and Matroska tags, and NFO through the XML parser that rejects any DOCTYPE; thumbnail URLs inside NFO files are never fetched | Tag and NFO parsing | Video detail page | SEC-MED-056, SEC-HIS-034, SEC-MED-016, SEC-TM-043 |
| LAT-097 | Online music video metadata | Artwork and credits filled in | Jellyfin: none by default; Plex and Emby (unverified) | Low: no counts found | Later | Plugin with a network grant | Plugin | Match dialog | SEC-PRV-013, SEC-PRV-014, SEC-EXT-026, SEC-EXT-028 |
| LAT-098 | Swap a song for its video | Flip the playing track to its video and back | Plex (unverified); Jellyfin and Emby: no; Spotify and YouTube Music mix videos in | Low: no counts found | R2 | The queue item keeps the track identity and the player switches source, at the start of the video unless an alignment is known | Link lookup | Now-playing toggle | SEC-API-026, SEC-TM-024 |
| LAT-099 | Listen-only mode for videos | A concert film plays with the screen off and lock-screen controls | Jellyfin: requested (34); nobody ships it | Low: 34 | R2 | libmpv drops the video track and the item moves to the music player | None | Listen-only toggle | SEC-MED-076, SEC-CLI-048 |
| LAT-100 | Concert chapters mapped to songs | Jump to a song in a two-hour concert and see its setlist | Nobody links chapters to songs (unverified) | Low: no counts found | Later | LAT-005 chapters linked to tracks through LAT-008, matched by title or by hand | Chapter-to-track mapping | Setlist on the concert page | SEC-TM-024, SEC-MED-014 |
| LAT-101 | Video plays count as song plays | Watching a concert adds to those songs' play counts | No data | Low: an open question | Later | Optional per user; scrobbling stays opt-in | Play events per mapped chapter | Setting | SEC-PRV-033, SEC-PRV-035, SEC-PRV-024 |
| LAT-102 | Multiple video angles | Switch angles on concert discs | Jellyfin: requested (16, 2026); nobody ships it | Low: 16 | Later | The core indexes every video track; libmpv switches tracks on native clients (unverified per platform) | Track index | Angle picker | SEC-MED-076, SEC-TM-032 |
| LAT-103 | Free remote viewing of music videos | Watch away from home with no pass | Plex: personal video needs a pass since 29 April 2025 (whether music videos count is unverified); Jellyfin and Emby: free | Medium: Plex's paywall is the top pain point | R2 | Nothing to paywall (AGPL, no central account). Native clients embed iroh; the project's own connection-success figures will replace iroh's quoted rate of about 90% direct (unverified for mobile carriers and TVs). No open port; whether there is no fee behind CGNAT depends on the relay decision (ACC-101). Browsers still need the owner's proxy or VPN until ACC-102. | None extra | None | SEC-TM-063, SEC-TM-064, SEC-NET-030, SEC-IAM-013 |

### Home videos

Rivals: Plex, Jellyfin and Emby; Immich appears where people now keep phone
clips. Home video is personal video with no lookups, so it is mostly a
date-first view over the R2 video path.

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| LAT-104 | Home video library with no online lookup | Family videos never get matched to films | Plex: Personal Media agent; Jellyfin: Home Videos and Photos type; Emby (unverified) | Medium: a Jellyfin request asks to switch fetching off entirely in mixed libraries | Later | The kind has no lookup path, so no plugin is ever called for it | Library kind | Library setup | SEC-PRV-013, SEC-TM-048 |
| LAT-105 | Dates and titles from the file | Clips sort by when they were filmed, not when they were copied | Plex: reads MP4, M4V and MOV tags; Jellyfin and Emby (unverified) | Medium: every date view (LAT-106) depends on it | Later | The core reads MP4, QuickTime and Matroska dates at scan, with filename dates as a fallback and timezones kept when present | Capture-date extraction | Detail page; date fix as a user-log overlay | SEC-TM-032, SEC-MED-014, SEC-TM-042 |
| LAT-106 | Date timeline | Scroll through years and months of clips | Plex: timeline in the Plex Photos app; Jellyfin: no timeline (unverified); Immich: best | Medium: Immich sets the expectation | Later | Library metadata is on the device, so scrubbing through years is local and instant | Date index | Timeline with year scrubber | SEC-API-015, SEC-CLI-020 |
| LAT-107 | Folder view | See LIB-008, which owns this feature. Applies to home videos when that library type ships. | Jellyfin: for home video and photo libraries (12.0), with a general request still open (438); Emby: yes | High: 438 | Later | See LIB-008. | None beyond LIB-008. | Folder browser | SEC-TM-026, SEC-API-068 |
| LAT-108 | Free remote viewing with no setup | Grandparents watch from their own house, free | Plex: paid for personal video since 29 April 2025; Jellyfin: free, but you bring your own VPN or reverse proxy; Emby: free | High: Plex's paywall is the top pain point | Later | No paywall. Native clients embed iroh; the project's own connection-success figures will replace iroh's quoted rate of about 90% direct (unverified for mobile carriers and TVs). No open port; whether there is no fee behind CGNAT depends on the relay decision (ACC-101). Browsers still need the owner's proxy or VPN until ACC-102. | None extra | Device pairing | SEC-TM-063, SEC-NET-030, SEC-IAM-078, SEC-IAM-080 |
| LAT-109 | Share a clip by expiring link | Send one video to someone with no account; the link dies on its own | Jellyfin: requested (580); Plex: share-a-movie request (294); Immich: yes | High: 580 and 294 | Later | A share link for one item under the share-link rules: the secret travels in the URL fragment, the link expires (30 days by default), allows two streams at once, suspends itself and tells the sharer when it spreads, takes an optional password, and is re-checked on every request. Video links stay off until the owner turns them on, and downloads work only where the owner allows them server-wide. It opens in a minimal guest player that never shows the sharer's name | Share capabilities; per-link limits; guest player route; revocation. Built on ACC-135. | Share sheet; guest web page; My shares | SEC-API-097, SEC-HIS-042, SEC-TM-028, SEC-PRV-031, SEC-PRV-055, SEC-STD-008 |
| LAT-110 | Phone HEVC and HDR clips | iPhone clips play right on any screen | Plex, Jellyfin and Emby: transcode; Immich: dark HDR complaints (15) | Medium: 15 | Later | Native clients direct-play through libmpv and browsers get a remux when they can decode the codec; the sandboxed tone-mapping transcode is the last resort, and there it is parity | Playback decisions | None | SEC-TM-044, SEC-TM-047, SEC-MED-024, SEC-MED-081, SEC-CLI-049 |
| LAT-111 | Keep home videos off shared surfaces | Family clips stay out of recommendations, Home rows and screensavers unless you add them | Home-row exclusion: all three; suggestion exclusion: Jellyfin no (12) | Low: 12 | Later | Off by default for this kind | Per-kind surface rules | Library settings | SEC-PRV-023, SEC-TM-026 |
| LAT-112 | Copy media to another drive | Folder sync and backup of media | Emby: Folder Sync (Premiere) | Low: no counts found | No | Gunmetal never writes to media folders; backup tools do this better | — | — | SEC-TM-042, SEC-OPS-054 |

### Photos

Rivals: Immich, Plex and Jellyfin. Immich is far ahead and moving fast, and
its machine learning contradicts the low-hardware goal. Gunmetal's job is to
show photos well on every screen, especially TVs, and to integrate with
Immich rather than replace it. Everything here is Later; server-side machine
learning and phone backup are No.

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| LAT-113 | Immich on the big screen | Your Immich albums and memories on Gunmetal's TV and phone apps | No integration found in the research | Medium: Plex's slideshow request (121); Plex removed photo libraries from its main iOS and Fire TV apps (Aug 2026) | Later | A plugin with an owner-approved grant to the one Immich host (a separate LAN grant when Immich runs at home). Each person links their own read-only Immich key (key scopes unverified), kept encrypted where admins cannot read it, and sees only their own Immich photos. Images are re-encoded by Gunmetal and served from Gunmetal, so clients never contact Immich; Immich keeps owning the photos | Plugin; per-person key storage; image re-encoding behind capability URLs | TV photos tab; albums; memories | SEC-EXT-026, SEC-EXT-029, SEC-EXT-050, SEC-EXT-028, SEC-TM-035, SEC-PRV-016 |
| LAT-114 | Read-only folder photo library | Point at a NAS folder and browse without importing | Plex and Jellyfin: folders; Immich: external libraries, where edits fail silently (27) | Medium: 27, plus Plex's photo regressions | Later | Never writes to the folder; EXIF and XMP parsed in the core, with locations kept private (LAT-179); no machine learning, so it runs on small boxes (Immich needs 6 GB RAM, or 4 GB with machine learning off) | Photo scan; metadata index | Photos tab; grid; viewer | SEC-TM-042, SEC-MED-038, SEC-MED-056, SEC-MED-046, SEC-TM-032 |
| LAT-115 | Thumbnails in a sandbox | Fast grids without exposing the server to image-decoder bugs | Immich: an SVG upload reached ImageMagick for remote code execution (Sept 2026) | Medium: 2026 image-handling advisories at Immich and Jellyfin | Later | JPEG, PNG and WebP are decoded by the memory-safe decoder in the scan worker within fixed size and pixel limits; every other format needs a native decoder, which runs only in the full jail and is off where the jail is missing (LAT-181); SVG is never decoded; thumbnails are rebuildable cache | Thumbnail job in the scan worker; jailed decoder for other formats; cache | None | SEC-TM-034, SEC-TM-044, SEC-MED-024, SEC-MED-044, SEC-MED-045, SEC-MED-025 |
| LAT-116 | Photo viewer | Swipe, zoom and an info panel with camera details | Immich: yes; Plex: Photos app rated 1.3 of 5, no update since June 2025; Jellyfin (unverified) | Medium: Plex's app ratings | Later | One viewer on every client, TV included | EXIF fields | Viewer; info panel | SEC-MED-046, SEC-TM-036, SEC-MED-013 |
| LAT-117 | HEIC and HEIF | iPhone photos open everywhere | Plex: requested since 2017 (637, open); Immich: yes, but HDR HEIC previews look flat (13) | High: 637 | Later | Every client gets JPEG, PNG or WebP made by a decoder in the full jail, so HEIC bytes never reach a phone's or TV's image decoder; the original is available only as a download (LAT-134). HDR looks flatter than the original, so this is behind Immich for HDR | Format detection; jailed conversion | None | SEC-MED-046, SEC-CLI-005, SEC-TM-035, SEC-MED-044, SEC-TM-044, SEC-MED-024 |
| LAT-118 | RAW files | Camera RAWs show up | Immich: yes, some DNGs fail (32, 112 comments); Plex: CR3 requested since 2018 (26) | Medium: 32 and 26 | Later | Shows the preview a RAW file embeds where it has one (coverage per format unverified), found by the core and re-encoded before any client sees it | RAW container parsing; preview extraction | Grid; viewer | SEC-TM-032, SEC-TM-035, SEC-MED-045, SEC-TM-033 |
| LAT-119 | Embedded keywords | Lightroom keywords show up and can be searched | Plex: requested since 2016 (283); Jellyfin: tag search requested (14); Immich: tags on web | High: 283 | Later | XMP keywords parsed in the core by the XML parser that rejects any DOCTYPE | Keyword index | Tag filter; search | SEC-MED-056, SEC-HIS-034, SEC-API-048, SEC-TM-036 |
| LAT-120 | Live and motion photos | The short clip behind a still plays back | Immich: yes; Plex (unverified); Jellyfin: no (unverified) | Low: no counts found | Later | The core finds the appended MP4, or the `mpvd` box in HEIC and AVIF, without decoding, and sends the clip as original bytes only when the core's MP4 parser accepted it at scan, like any video | Motion-photo detection | Press and hold to play | SEC-MED-012, SEC-API-018, SEC-CLI-049, SEC-MED-059 |
| LAT-121 | Photo timeline | Fly through 20 years by date | Immich: yes; Plex: Photos app timeline; Jellyfin: no (unverified) | Medium: Immich sets the bar | Later | The synced library makes scrubbing local and instant | Date index | Timeline with scrubber | SEC-API-015, SEC-CLI-020 |
| LAT-122 | Photo folder view | Browse folders as you made them | Immich, Plex and Jellyfin (12.0): yes | Low: every rival has it | Later | Parity | Folder tree | Folder browser | SEC-TM-026, SEC-API-068 |
| LAT-123 | Albums of photos and clips | Curated sets, such as one trip, mixing photos and videos | Immich: yes; Plex: yes (detail unverified); Jellyfin: collections (unverified) | Medium: a basic expectation | Later | Albums are user-log events, so they survive any rebuild | Album events | Album pages | SEC-TM-024, SEC-MED-051, SEC-PRV-047 |
| LAT-124 | Shared albums | Family sees and adds to the same album | Immich: yes, but sharing changes are frozen pending a redesign (749, 260 comments); Plex: library-level; Jellyfin: per-user library access | High: 749 | Later | Per-object authorisation on every photo; parity at best, since Immich shows how hard sharing is to get right | Album permissions | Share sheet; shared album view | SEC-TM-024, SEC-TM-025, SEC-IAM-073, SEC-TM-028 |
| LAT-125 | Public album links | Send an album to someone without an account, with expiry and an optional password | Immich: yes, with password and download switch; Plex: no (unverified); Jellyfin: requested (580) | High: 580 | Later | Same share links as LAT-109, for one album; the guest gallery shows only re-encoded images with no location or camera details (LAT-179) | Share capabilities | Share sheet; guest gallery | SEC-API-097, SEC-PRV-031, SEC-MED-046, SEC-HIS-042, SEC-PRV-055 |
| LAT-126 | Partner sharing | Two people see each other's whole photo library | Immich: yes; Plex and Jellyfin: no | Low: no counts found | Later | Each partner grants the other read access to their own library; nobody else, an admin included, sets it up for them, it never gives more than the granter holds, it appears in both people's sharing settings, and either can end it on the next request | Person-to-person library grants | Settings | SEC-IAM-073, SEC-HIS-060, SEC-TM-027, SEC-TM-028 |
| LAT-127 | Slideshow with music on the TV | Photos on the living-room screen, with a playlist underneath | Plex: "slideshow for all clients" requested (121); Jellyfin: slideshow with delay (12.0), with music requested (5); Immich: mobile slideshow (3.0.0) | Medium: 121 | Later | Gunmetal's music player runs under the slideshow on the same TV client | Album or memory feed | Slideshow controls; music picker | SEC-TM-026, SEC-CLI-063, SEC-IAM-064 |
| LAT-128 | Photo frame and ambient mode | An idle TV cycles an album | Jellyfin: requested (12); nobody ships it | Low: 12 | Later | Idle mode obeys content limits and surface rules (LAT-014, LAT-111) and shows only albums the device's current profile may see | Album feed | Screensaver settings | SEC-TM-026, SEC-IAM-064, SEC-CLI-063, SEC-IAM-109 |
| LAT-129 | Memories | "Five years ago today" | Immich: yes; Plex: Recommended view; Jellyfin: no | Low: no counts found | Later | Computed locally from capture dates; nothing leaves the server | Date queries | Home row | SEC-PRV-022, SEC-TM-024 |
| LAT-130 | Map of photos and clips | See where photos and videos were taken | Immich: yes; Plex: launched Places in 2017, since withdrawn; Jellyfin: no | Low: no counts found | Later | GPS is read in the core and shown only as LAT-179 allows. Map tiles come from outside: a tile plugin fetches them through the egress client under an owner-approved grant to named hosts, its consent screen says that tile requests reveal which places people look at, each person turns the map on for themselves, and clients load tiles only from the Gunmetal server | GPS index; tile plugin; tile cache | Map view | SEC-PRV-016, SEC-EXT-005, SEC-EXT-026, SEC-EXT-039, SEC-IAM-087, SEC-EXT-028 |
| LAT-131 | Stacks | RAW and JPEG, or a burst, shown as one | Immich: yes, "stacks on all views" requested (98); Plex and Jellyfin: no | Medium: 98 | Later | Grouped at scan by file name and capture time; Immich plans automatic burst stacking, so parity at best | Stack grouping | Stack badge; expand | SEC-TM-024, SEC-API-015 |
| LAT-132 | Rotate and crop without touching the original | Fix a sideways photo | Immich: editing on web (2.5.0) and mobile (3.0.0); Plex: rotate requested (9) | Low: 9 | Later | Stored as a user-log overlay; the file is never edited | Overlay events | Viewer edit menu | SEC-TM-042, SEC-MED-038, SEC-TM-027 |
| LAT-133 | Favourites, archive and hidden | Star the good ones, hide screenshots | Immich: yes; Jellyfin: favourites; Plex (unverified) | Medium: a basic expectation | Later | User-log events | Events | Viewer actions; filters | SEC-PRV-022, SEC-PRV-049 |
| LAT-134 | Download originals | Get the full-resolution file back | Plex: requested since 2014 (38); Immich: yes | Low: 38 | Later | The original is served only as a download (an attachment with a server-chosen name and a sandbox policy) behind a short-lived capability URL, never shown inline; guests and share links get it only where the owner allows downloads | Signed download | Download action | SEC-MED-059, SEC-API-051, SEC-MED-046, SEC-API-026, SEC-IAM-080 |
| LAT-135 | Duplicate report | Find copies of the same photo | Immich: resolution (2.7.0); Plex and Jellyfin: no | Low: no counts found | Later | Report only; Gunmetal never deletes from a read-only folder | Content-hash index | Duplicates report | SEC-TM-042, SEC-API-068 |
| LAT-136 | 360-degree photos | Pan around a panorama | Immich: web only; Plex and Jellyfin: no | Low: no counts found | Later | Parity at best | Projection metadata | Panorama viewer | SEC-MED-046, SEC-MED-047 |
| LAT-137 | Phone photo and video backup | Photos and clips leave the phone by themselves, then free up space | Immich: yes (freeing space since 2.5.0); Emby: camera upload; Plex: historic (current status unverified) | High: Immich's popularity (115,500 GitHub stars) | No | Immich does this well; owning irreplaceable originals needs backup guarantees outside Gunmetal's read-only, rebuildable design | — | — | SEC-TM-042, SEC-MED-038 |
| LAT-138 | Face recognition on the server | Every photo of grandma | Immich: yes; Plex: never shipped (60 votes, plus a closed thread with 110 likes); Jellyfin: requested (15) | Medium: 60 and 110 | No | Needs machine learning (Immich: 6 GB RAM with it on); revisit only on client devices (feasibility unverified) or through Immich | — | — | None (No) |
| LAT-139 | Smart search and text in images | Type "dog", or a receipt's words, and find the photo | Immich: CLIP search and OCR; Plex: auto-tagging launched in 2016, since withdrawn; Jellyfin: requested (13) | Low: 13 | No | Same reason as LAT-138 | — | — | None (No) |
| LAT-140 | Editing suite, storage templates and automation | Immich-style editing, upload folder layouts and workflows | Immich: yes (workflows in preview, 3.0.0) | Low: no counts found | No | Gunmetal does not manage photo files; that is the photo manager's job | — | — | SEC-TM-042 |
| LAT-179 | Photo and clip locations stay private | Where a photo or clip was taken is never shown to someone who should not see it | No data | Low: no counts found; required by the privacy baseline | Later | GPS, camera and other EXIF fields are stripped from every image derivative; share links, guest galleries, household-device slideshows and memories shown to others carry no location; the map and info panel show location only to people who can see the original's library, and a shared album shows it only if its owner turns that on; a library can be set to ignore location entirely | Location stored as its own classified field; per-album and per-library switches | Album sharing settings; library settings; info panel | SEC-MED-046, SEC-PRV-031, SEC-PRV-023, SEC-TM-050, SEC-PRV-001 |

### Ebooks

Rivals: Kavita, Komga and Jellyfin 12, with ABS and Booklore in notes.
Plex's most-voted request ever is a reader (3,182 votes), but Kavita and
Komga are good. Protocols (OPDS, KOReader sync, later Kobo sync) deliver most
of the value with little client work, so they come first and Gunmetal's own
readers come last, if at all.

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| LAT-141 | OPDS catalogue | E-reader apps browse and download your books | Komga: OPDS 1.2 and 2 with page streaming; Kavita: OPDS; Jellyfin: no (unverified); ABS: requested (73) | High: Plex reader request (3,182); ABS 73 | Later | An adapter under the same rules as the others: off until the owner enables it, on its own listener, every entry through per-object authorisation and the cross-user tests. Readers cannot do passkeys (unverified per app), so each gets a per-app key the server generates, read-only, expiring and revocable (LAT-176), sent in the Authorization header over HTTPS and never in a URL; readers that cannot do HTTPS are not supported | OPDS 1.2 and 2 feeds on the adapter listener; per-app keys; downloads under the same key or a capability URL | Account > Apps (LAT-176); QR code with the HTTPS catalogue address, never the key | SEC-IAM-084, SEC-EXT-051, SEC-EXT-054, SEC-EXT-055, SEC-EXT-006, SEC-EXT-066 |
| LAT-142 | KOReader progress sync | Progress shared with KOReader devices, to the paragraph | Kavita: yes; Komga: yes, but EPUB progress only at chapter starts; Jellyfin: no (unverified) | Medium: Komga documents the lost positions | Later | Stores KOReader's progress string and percentage plus a Readium locator, so a position is never cut back to a chapter start. It is an adapter: off by default, HTTPS only, signed in with a generated per-app key rather than a chosen password (how KOReader's headers carry it is unverified), with no account-creation endpoint, and it accepts progress only for books the key's owner can see | Sync endpoint; index of KOReader's MD5-based document hashes; locator events | Account > Apps (LAT-176) | SEC-IAM-084, SEC-EXT-054, SEC-EXT-055, SEC-EXT-057, SEC-EXT-066 |
| LAT-143 | Kobo sync | A Kobo shows your library as if it were the store | Komga: yes (EPUB converted to KEPUB, chapter-level progress); Kavita: planned (unverified); ABS: its most-upvoted request (258) | High: 258 | Later | Only after the protocol and its legal position are checked (unverified). Rivals' Kobo sync appears to rely on a long-lived key inside the sync URL (unverified), and Gunmetal never accepts a key in a URL, so this is built only if the device can send its key in a header; otherwise the row becomes No. KEPUB copies would be cache files, never written into the library, and the server would not relay the device's traffic to the Kobo store | Kobo endpoints; conversion job | Device setup | SEC-EXT-006, SEC-IAM-084, SEC-EXT-055, SEC-EXT-051, SEC-TM-042 |
| LAT-144 | Ebook formats | EPUB, PDF, MOBI and AZW3 appear with covers | Jellyfin: the widest list; Kavita and Komga: EPUB and PDF | Medium: Plex reader request (3,182) | Later | EPUB is a ZIP archive, so the core reads it in memory only under the archive rules of LAT-164, once the architecture record for archives exists; other formats are served in their original bytes, only as downloads; covers are re-encoded | Format detection; cover extraction | Books grid | SEC-HIS-019, SEC-MED-055, SEC-MED-009, SEC-MED-011, SEC-API-051, SEC-MED-059 |
| LAT-145 | Embedded ebook metadata | Title, author and series from the book's OPF | Kavita, Komga and Jellyfin (12.0): yes | Medium: every rival has it | Later | The core's XML parser rejects any document with a DOCTYPE and limits size, depth and element count | OPF parsing | Book page | SEC-MED-056, SEC-HIS-034, SEC-TM-038, SEC-HIS-019 |
| LAT-146 | Free online book metadata | Covers and blurbs fetched | Kavita: paid (Kavita+); Jellyfin: free (Google Books, Open Library, ComicVine) | Medium: Kavita charges for it | Later | A first-party plugin with a grant, never a paid tier | Plugin | Match dialog | SEC-PRV-013, SEC-PRV-014, SEC-EXT-026, SEC-EXT-028 |
| LAT-147 | Series and reading order | Books in order, with gaps visible | Kavita and Komga: yes; Jellyfin: from filenames (12.0) | Medium: every rival has it | Later | LAT-003 | Series ordering | Series page | SEC-TM-024, SEC-MED-001 |
| LAT-148 | Reading progress and sessions | Progress, reading sessions and re-read counts | Kavita: since 0.8.9; Komga: progress | Medium: a basic expectation | Later | Locator events in the user log | Position events | Book page | SEC-PRV-002, SEC-PRV-022, SEC-PRV-049 |
| LAT-149 | Want-to-read shelf | A wishlist of books | Kavita: yes; Komga (unverified); Jellyfin: no | Low: no counts found | Later | User-log events | Events | Shelf | SEC-PRV-022 |
| LAT-150 | Web EPUB reader | Read in a browser | Kavita: overhauled (0.8.8); Komga: yes; Jellyfin: redesigned (12.0) | Medium: Plex reader request | Later | The lowest-priority reader. Book pages render only on a separate sandbox origin that receives no cookies and runs no script from the book, or from a safe model the core converts the EPUB into; never on the application origin. Embedded fonts are not delivered by default, as for subtitles. Positions are written as locators | Sandbox origin or EPUB-to-safe-model conversion; otherwise none beyond LAT-148 | Reader | SEC-API-099, SEC-API-051, SEC-TM-036, SEC-MED-057, SEC-MED-054 |
| LAT-151 | Phone ebook reader | Read in the Gunmetal phone app | Jellyfin: official apps (unverified); third-party OPDS readers | Low: good OPDS readers already exist | Later | Built only if OPDS readers prove not enough, under LAT-150's rendering rule (no WebView that holds the app's session) | None | Reader | SEC-API-099, SEC-TM-036, SEC-MED-057 |
| LAT-152 | Typography controls | Font, size, margins and themes | Kavita: custom fonts; Jellyfin: font size | Low: no counts found | Later | Parity | None | Reader settings | SEC-MED-054, SEC-TM-062 |
| LAT-153 | Highlights and annotations | Mark and note passages | Kavita: since 0.8.8; Booklore: yes; ABS: requested (25, 33) | Medium: 25 and 33 | Later | User-log events with locator ranges, exportable | Annotation events | Reader; notes page | SEC-PRV-022, SEC-PRV-047, SEC-TM-036, SEC-API-048 |
| LAT-154 | Send to Kindle or email | One tap sends a book to a device | ABS and Booklore: yes; Kavita (unverified) | Low: no counts found | Later | A plugin with an owner-approved grant to one mail service; each person turns it on for themselves and registers their own device addresses, and a book goes only to those addresses | Plugin; per-person addresses | Send button | SEC-IAM-087, SEC-TM-017, SEC-EXT-026, SEC-EXT-039, SEC-EXT-050 |
| LAT-155 | Reading statistics | Time read and a year in review, for you | Kavita: since 0.8.9 | Low: no counts found | Later | Computed from the local user log and private: shown only to that person, with no shared profile page; a person can export their figures and share them however they like | Aggregation job | Stats screen | SEC-HIS-060, SEC-PRV-022, SEC-PRV-023, SEC-PRV-025, SEC-PRV-047 |
| LAT-156 | Smart filters and collections | Saved searches and collections | Kavita: yes; Komga and Jellyfin: collections | Low: every rival has it | Later | Same rule engine as music smart playlists | Rule definitions | Filter builder | SEC-TM-039, SEC-STD-011 |
| LAT-157 | Content limits in book protocols | Age limits hold in OPDS and sync, not just in the app | Komga: enforced for Kobo (1.25.0); Kavita: yes (detail unverified) | Medium: Komga added it deliberately | Later | LAT-014 applied to the OPDS, KOReader and Kobo endpoints, each in the per-endpoint authorisation test matrix | Shared limit filter | Profile settings | SEC-IAM-064, SEC-TM-026, SEC-EXT-054, SEC-CLI-015 |
| LAT-158 | Download a whole series | Take a series offline | Kavita: multi-download (0.9.0); Jellyfin: request (1) | Low: 1 | Later | Same download engine as music | Download manifests | Series page | SEC-NET-053, SEC-IAM-080, SEC-IAM-054 |
| LAT-159 | Reading scrobbles | Progress posted to Hardcover or AniList | Kavita: paid (Kavita+) | Low: no counts found | Later | A free plugin, off until each person turns it on for themselves, with each user's secrets kept per user and private sessions never sent | Plugin | Account links | SEC-IAM-087, SEC-PRV-033, SEC-PRV-034, SEC-PRV-035, SEC-PRV-036, SEC-EXT-050 |
| LAT-160 | PDFs open on your device | PDFs read in the device's own viewer | No data | Low: no counts found | Later | Original bytes are sent as a download (an attachment with a sandbox policy) for the device's own viewer, never shown on the application origin; the server never renders PDFs | Signed download | Open in the device's viewer | SEC-API-051, SEC-API-099, SEC-MED-059, SEC-API-026 |
| LAT-161 | Server-side PDF rendering | The server turns PDF pages into images | No data | Low: no counts found | No | A large, risky parser; the device renders PDFs | — | — | SEC-MED-044, SEC-MED-025, SEC-API-099 |
| LAT-162 | Ebook reader on TV | Read books on the TV | Jellyfin: books library on TV (unverified) | Low: little use on a TV | No | Of little use on TV; the protocols serve reading devices | — | — | None (No) |
| LAT-163 | DRM-protected ebooks | Open store-bought DRM books | No data | Low: no counts found | No | Supporting DRM removal is a legal risk | — | — | None (No) |

### Comics and manga

Rivals: Kavita, Komga and Jellyfin 12. A CBZ page is a stored file inside
a ZIP archive, so the server can find it without decompressing anything. It
still re-encodes every page before a client sees it, because a page is an
image from a file and hostile images attack phone and TV decoders
(SEC-TM-035). Reading any comic archive waits for the architecture record
the security baseline requires before the server opens archives
(SEC-HIS-019).

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| LAT-164 | CBZ, CBT and CB7 archives | Common comic archives open | Kavita: yes (CB7 unverified); Komga: CBZ and CBR; Jellyfin: cb7, cbr, cbt, cbz | Medium: Plex reader request (3,182) | Later | CBZ and CBT read by the core in memory once the architecture record for archives exists: caps on entry count, entry size and total expanded size, entry names never used as paths, link entries refused, nothing written to disk. CB7 needs a decompressor (maturity unverified); a native one runs only in the full jail | Archive index | Comics grid | SEC-HIS-019, SEC-MED-055, SEC-MED-009, SEC-HIS-015, SEC-TM-032, SEC-TM-033 |
| LAT-165 | CBR archives | RAR-based comics open | Komga: RAR5 in plain Java (1.26.0), solid RAR4 (1.25.0); Kavita and Jellyfin: yes | Medium: all three rivals support it | Later | A native decoder only in the full jail, off where the jail is missing (LAT-181), or a lower tier: the reference unrar code is C under a restrictive licence (details unverified) and a mature pure-Rust decoder is unverified. Pages are never extracted into a library folder, and entry names never become file names | Jailed extraction into memory; page cache in the data directory | None | SEC-MED-025, SEC-TM-044, SEC-MED-024, SEC-HIS-019, SEC-MED-039 |
| LAT-166 | Page streaming | Apps fetch single pages without downloading the whole archive | Kavita and Komga: through OPDS; Jellyfin: no (unverified) | Medium: both specialists ship it | Later | Each page is decoded by the memory-safe decoder and re-encoded at fixed sizes before it is served, then cached, so a hostile page never reaches a tablet's or TV's image decoder. This costs CPU that sending the stored bytes would not | Page endpoint behind capability URLs; page re-encoding and cache | None (third-party apps and LAT-171) | SEC-TM-035, SEC-MED-046, SEC-CLI-005, SEC-MED-047, SEC-API-026 |
| LAT-167 | ComicInfo metadata | Writers, artists and issue numbers from the file | Kavita and Komga: yes; Jellyfin: native (12.0), with artist gaps (5) and a ComicInfo request (14) | Medium: 14 | Later | The core's XML parser, which rejects any DOCTYPE | ComicInfo parsing | Issue page | SEC-MED-056, SEC-HIS-034, SEC-TM-038, SEC-API-048 |
| LAT-168 | Volume and chapter numbers | "Vol 2 Ch 14.5" sorts right | Kavita: yes, with open edge-case issues; Komga: yes; Jellyfin: since 12.0 (order request, 4) | Medium: Kavita's open edge cases | Later | One pure core function under the full gate (100% coverage, zero surviving mutants), with every reported edge case kept as a test | Number parser | Series page | SEC-MED-001, SEC-MED-014, SEC-TM-032 |
| LAT-169 | Page counts | Know how long an issue is | Kavita, Komga and Jellyfin (12.0): yes | Low: every rival has it | Later | Counted from the archive directory at scan, without extracting, within the archive caps of LAT-164 | Archive index | Issue tile | SEC-HIS-019, SEC-MED-055, SEC-TM-032 |
| LAT-170 | Covers | A cover for every issue | All three: yes; Kavita+ full-size covers cannot be reclaimed (open issue) | Low: every rival has it | Later | The first page is always re-encoded to the fixed cover sizes, even when it is already JPEG or PNG; covers are rebuildable cache | Thumbnail job | Grid | SEC-MED-046, SEC-TM-035, SEC-HIS-030, SEC-MED-048 |
| LAT-171 | Comic reader modes | Single page, double page and right-to-left | Kavita: yes; Komga (detail unverified); Jellyfin: unified reader, richer reader started (44) | Medium: 44 | Later | A reader for tablets, phones and web, fed by LAT-166 | None | Reader | SEC-MED-046, SEC-MED-047 |
| LAT-172 | Webtoon scroll | Long strips scroll smoothly | Kavita: improved (0.9.0); Jellyfin: requested (12) | Low: 12 | Later | Pages streamed in order as LAT-166's re-encoded pages | None | Reader mode | SEC-TM-035, SEC-MED-046 |
| LAT-173 | Pinch zoom | Read small lettering on a phone | ABS app: lacks it (29); others (unverified) | Low: 29 | Later | Parity; zoom uses the largest fixed page size | None | Reader gesture | SEC-MED-047 |
| LAT-174 | Reading lists and CBL import | Follow a crossover event in order | Kavita: CBL import; Komga: read lists; Jellyfin: no | Low: no counts found | Later | Lists are user-log events; CBL files are parsed in the core by the XML parser that rejects any DOCTYPE, and entries resolve only to issues the importing person can see | List events | Reading list page | SEC-HIS-018, SEC-MED-050, SEC-API-085, SEC-MED-056 |
| LAT-175 | Online comic and manga metadata | Covers and credits from comic databases | Kavita: paid (Kavita+: MangaBaka, ComicBookRoundup); Jellyfin: ComicVine | Low: no counts found | Later | A free plugin with a grant | Plugin | Match dialog | SEC-PRV-013, SEC-PRV-014, SEC-EXT-026, SEC-EXT-028 |

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
- **Original image bytes on screen (LAT-117, LAT-134, LAT-166, LAT-170).**
  Photos, comic pages and covers reach clients only as JPEG, PNG or WebP
  that the server made, because hostile images have taken over phone and
  TV decoders (libwebp CVE-2023-4863). Originals are available only as
  downloads.
- **Keys in URLs for outside apps (LAT-045, LAT-143).** Readers, podcast
  apps and private feeds send their key in a header. A protocol that only
  works with a key in its URL is not built.
- **Playing podcasts straight from the publisher (LAT-065).** Clients fetch
  only from their own server, so a publisher never learns a phone's address
  and the player never plays bytes the server did not parse.
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
  make this an R2 dependency. Podcasts, Immich, map tiles and send-to-device
  each need a new plugin interface defined by an architecture record
  (SEC-EXT-076), and their grants name exact hosts (SEC-EXT-026).
- **One egress client.** The server has one outbound HTTP client from R1
  (SEC-EXT-001). R3's M3U and EPG sources (Jellyfin's M3U tuner had an SSRF
  advisory in 2026) and podcasts (LAT-059) add purposes to it with
  exact-host grants; neither gets a client of its own.
- **A sandbox for untrusted decoders.** Record 1 sandboxes only FFmpeg.
  HEIC conversion (LAT-117), CBR and CB7 (LAT-164, LAT-165) and any image
  format beyond JPEG, PNG and WebP decode untrusted bytes with large native
  codebases; thumbnails, RAW previews and covers (LAT-115, LAT-118,
  LAT-170) stay on the memory-safe decoder. Immich's 2026
  SVG-to-ImageMagick remote code execution is the warning. The security
  baseline already requires the full jail for any native decoder, and turns
  the feature off without it (SEC-TM-044, SEC-MED-024, LAT-181); the ADR
  amendment records this before any photo or comic work.
- **XML everywhere.** OPF, ComicInfo, NFO, RSS, OPML, XMP, SMIL and CBL are
  all XML. One parser in the core must reject any document with a DOCTYPE,
  limit size, depth and element count (SEC-MED-056, SEC-HIS-034), and be
  fuzzed like the media parsers.
- **Archives.** The server may not read archives at all until an
  architecture record allows it (SEC-HIS-019), and EPUB, CBZ and CBT all
  need it. That record must cap entry count, entry size and expanded size,
  ignore entry names as paths, refuse link entries and keep everything in
  memory (SEC-MED-055). The RAR licence and the maturity of pure-Rust RAR
  and 7z decoders are unverified; CBR may have to be a lower tier.
- **Long-lived credentials for outside apps.** OPDS readers, KOReader, Kobo,
  podcast apps and private book feeds (LAT-045, LAT-141 to LAT-143, LAT-088,
  LAT-089) cannot use passkeys or short-lived signed URLs. They need
  per-app keys scoped to one protocol, revocable, with expiry, sent in a
  header and never in a URL (SEC-EXT-006), and adapters that are off until
  the owner enables them (SEC-IAM-084). Immich's CVE-2026-23896 (a key
  raising its own permissions) and Navidrome's never-expiring Jellyfin-API
  tokens are the mistakes to avoid. Apps that support only a key in the URL
  cannot connect.
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
  publishers and their analytics prefixes. Clients never fetch from
  publishers (LAT-065), so devices are not exposed, and the owner approves
  every host first (LAT-180).
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
- **Scope and people.** This map has 181 features against a strict
  test-first gate. ABS's single maintainer has kept the iOS app in beta for
  years, Booklore entered maintenance mode, and Jellyfin's leader stepped
  down from burnout in July 2026. Only the rows for R1, its point releases
  (R1.1 to R1.3) and R2 should be planned now.
- **Rivals are moving.** Jellyfin 12.0 improved books, comics and
  audiobooks; ABS ships steadily (v2.37.1 on 29 Sept 2026); Immich is very
  active. Parity claims here will age.

## Open decisions for the project owner

1. **When audiobooks ship (LAT-015 to LAT-057).** Options: in R1 (web only),
   with R2, or as their own release alongside the first native mobile app.
   *Decided in the feature map README, with the smaller R1 the owner
   adopted ([D-10](../decisions.md#d-10-r1-scope-and-the-release-table)):*
   R1 carries only the schema-only foundations LAT-001, LAT-006, LAT-007
   and LAT-009; LAT-002's typed roles, LAT-008's relation table and a
   simple LAT-010 folder flag follow in R1.3. The audiobook experience
   (LAT-015 to LAT-057), lectures (LAT-056) and the home video library
   (LAT-104 to LAT-111) are Later, because ADR 2 puts M3U and live TV first
   and books after. Pulling books ahead of live TV would need an ADR that
   changes that order.
2. **Spoken word as a library type or an item kind (LAT-010, LAT-056).**
   *Recommendation:* a kind on each item, defaulted by library and
   overridable per folder, so lectures, audio dramas and courses get book
   behaviour without being called books.
3. **Read-only libraries for good (LAT-019, LAT-046, LAT-112, LAT-132).**
   *Recommendation:* yes. Never write to media folders; keep fixes as
   user-log overlays; any export writes new files to a separate export folder
   on explicit request; archived podcast episodes go to their own archive
   folder (LAT-064).
4. **Long-lived credentials for outside apps (LAT-045, LAT-088, LAT-089,
   LAT-141 to LAT-143).** *Recommendation:* per-app keys (ACC-129, LAT-176)
   scoped to one protocol, read-only where possible, listed and revocable by
   the user and expiring, sent in a header and never in a URL; never the
   main session token. The adapters for these protocols are off until the
   owner enables them and serve HTTPS only. Private book feeds stay off by
   default, warn when switched on, and use a per-feed key in the header. A
   protocol that needs a key in its URL is not built. This follows the
   security baseline (SEC-IAM-083, SEC-IAM-084, SEC-EXT-006) and its open
   decision 8.
5. **Audible- and Amazon-derived metadata (LAT-020, LAT-022).**
   *Recommendation:* no first-party plugin uses these sources. Third-party
   plugins may, and the plugin page shows where data comes from. Take legal
   advice before any first-party use.
6. **Who fetches podcasts (LAT-058, LAT-064, LAT-065).** Server fetching
   exposes the server's IP address; client fetching exposes each device's.
   *Recommendation:* the server fetches feeds and episodes from hosts the
   owner approved (LAT-180), archiving is opt-in per show, and clients
   always play through the server, so phones stay private. Streaming
   straight from the publisher is not offered: clients fetch only from their
   own server and play only what its parser accepted (SEC-CLI-044,
   SEC-CLI-049).
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
   on TV. Every reader renders book pages on a sandbox origin or from a safe
   model the core makes (SEC-API-099).
10. **Kobo sync (LAT-143).** It is ABS's top request (258) and Komga shows
    it can be done, but its legal position is unverified.
    *Recommendation:* commission a protocol and legal review before any
    code. Build it only if the device can send its key in a header
    (LAT-143).
11. **Audiobookshelf: importer or adapter (LAT-054, LAT-055).**
    *Recommendation:* an importer with the book release; no adapter, since
    ABS says its API is still being standardised.
12. **A sandbox for untrusted decoders.** *Recommendation:* write an ADR
    that defines one sandboxed decoder worker (images, RAW, RAR, 7z) next to
    the FFmpeg sandbox, before any photo or comic work starts. The security
    baseline already requires the jail for native decoders (SEC-TM-044,
    SEC-MED-024); the same record should allow reading archives under the
    rules SEC-HIS-019 sets, since EPUB, CBZ and CBT need it.
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

## Security notes

This area crosses six trust boundaries: client to server (TB4) through
reader, podcast and adapter endpoints and share links; device to person
(TB5) through offline books and episodes; server to jail (TB6) for HEIC,
RAR, 7z and Opus; server to plugins (TB7) for podcasts, Immich, metadata,
map tiles and mail; server to outbound hosts (TB8) for feeds, episodes and
imports; and server to file system (TB9) for archives, sidecars and the
podcast archive. These are the threats from the
[threat model](../security/threat-model.md) that matter most here, and the
rows that answer them.

| Threat | Where it shows up here | How this map answers it |
|---|---|---|
| TM-T30 server-side request forgery; TM-T36 a grant bypassed to reach the LAN | Pasted feed URLs, redirects on episode downloads, chapter and transcript files, Immich and Audiobookshelf on the home network | One egress client (LAT-059); exact hosts the owner approves per show (LAT-058, LAT-180); a pasted address is never fetched as a side effect (LAT-061, LAT-062, LAT-089); LAN hosts only by exact grant (LAT-054, LAT-113) |
| TM-T33 lookups tell third parties what the household owns and plays | Metadata plugins, directory search, map tiles, podcast fetching, send-to-device | Off until the owner turns each one on, sending only listed fields (LAT-022, LAT-146, LAT-175); each person opts in to tiles and mail (LAT-130, LAT-154); searches go only to the named directory and are not stored (LAT-060); clients never contact publishers (LAT-065) |
| TM-T26 XML entities | OPF, NFO, RSS, OPML, XMP, SMIL, ComicInfo and CBL | One parser that rejects any DOCTYPE, with limits (LAT-021, LAT-063, LAT-096, LAT-119, LAT-145, LAT-167, LAT-174) |
| TM-T29 decompression bombs; TM-T22 paths from archive entries | EPUB, CBZ, CBT, CB7 and CBR | No archive reading before its architecture record; caps, names never used as paths, link entries refused, nothing written to disk (LAT-144, LAT-164, LAT-165, LAT-169) |
| TM-T21, TM-T52 and TM-T53 native decoder exploits and a missing jail | HEIC, RAR, 7z, Opus | Full jail only, and off with a reason where it is missing (LAT-115, LAT-117, LAT-165, LAT-181) |
| TM-T28 hostile images against client decoders; TM-T07 script in content | Photos, comic pages, covers, EPUB, PDF, transcripts, feed text | Server re-encoded images only (LAT-117, LAT-134, LAT-166, LAT-170, LAT-172); documents on a sandbox origin or as downloads (LAT-150, LAT-160); text shown as text (LAT-063, LAT-077) |
| TM-T56 writes into media roots | Podcast archive, KEPUB copies, chapter and photo fixes, merges | A separate archive folder (LAT-064); overlays in the user log (LAT-019, LAT-132); no writing tools (LAT-046, LAT-112) |
| TM-T12, TM-T15 and TM-T66 object access and restrictions skipped on a new path | OPDS, KOReader, Kobo, podcast sync and OpenSubsonic podcast endpoints; folder views | Adapters off by default under the same authorisation and cross-user tests (LAT-088, LAT-089, LAT-141 to LAT-143); limits on every endpoint and URL (LAT-014, LAT-157) |
| TM-T16 and TM-T67 links that outlive revocation or leak | Book, clip and album share links; private book feeds; reader keys | Share links under the share-link rules (LAT-044, LAT-109, LAT-125); keys in headers, listed and revocable (LAT-045, LAT-176) |
| TM-T18 history disclosed beyond expectation | Reading progress, subscriptions, statistics, annotations, photo locations | Admins see live sessions and totals only (LAT-178); private sessions (LAT-177); no shared reading profile (LAT-155); locations stripped (LAT-179) |
| TM-T34 a malicious plugin; TM-T39 a lost device | Podcast, Immich, metadata, tile, scrobbling and mail plugins; offline books and episodes | Signed plugins with exact-host grants and per-person secrets (LAT-113, LAT-154, LAT-159); offline grants that expire and are withdrawn (LAT-036, LAT-066) |

Residual risks: header support in podcast apps, KOReader and Kobo devices is
unverified, so some outside apps may not connect at all; a native decoder in
the jail still shares the host kernel (TM-T52); and the owner can read the
database directly, which the what-admins-can-see page says (LAT-178).
