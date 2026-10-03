# Adjacent media types

## Scope

This document covers the media types that sit next to Gunmetal's first
release: audiobooks, podcasts, music videos and concerts, home videos,
photos, ebooks, and comics and manga. None of them ships in the first
version. Record 1 originally said photos and books come later, and record 2
kept that order. The purpose here is narrower than planning them: it is to
learn what each type needs so the first data model and the first player do
not rule them out, and to judge which of them are worth building at all.

For each type the relevant rivals differ. Plex, Jellyfin and Emby are the
right comparison for music videos and home videos. For audiobooks the
comparison is Audiobookshelf, Plex (with Plexamp and the third-party
Prologue app) and Jellyfin. For podcasts it is Pocket Casts and
Audiobookshelf. For photos it is Immich, Plex and Jellyfin. For ebooks and
comics it is Kavita, Komga and Jellyfin. The tables below change their
columns accordingly.

Method. Facts were checked on 2 October 2026 against release pages, official
documentation, the Plex forum's and Jellyfin's feature-request boards (vote
counts read from their public JSON endpoints that day), GitHub issue
reactions, and app store listings. The web search budget ran out partway
through, so later checks used direct page fetches and public APIs instead.
Reddit could not be reached, so user sentiment comes from feature boards,
issue trackers and forum threads rather than Reddit. Anything that could not
be checked against a source is marked "(unverified)".

Versions current at the time of writing:

| Product | Latest release seen | Date |
|---|---|---|
| Audiobookshelf server | v2.37.1 | 29 Sep 2026 |
| Audiobookshelf mobile apps | v0.13.0-beta (iOS through TestFlight only) | 11 May 2026 |
| Jellyfin | 12.1 (12.0 dropped the leading "10.") | 15 Sep 2026 |
| Immich | v3.2.4 stable, v3.3.0-rc.2 prerelease | 28 Sep / 2 Oct 2026 |
| Kavita | v0.9.1.4 | 2 Sep 2026 |
| Komga | 1.28.1 | 2 Oct 2026 |
| Pocket Casts Android | 8.21 | 29 Sep 2026 |
| Plex Photos (iOS) | 2025.2.0 | 12 Jun 2025 |

## Feature inventory

Each row says what the user gets, how the three relevant rivals handle it,
who does it best and why. "No" means the product does not offer it as far
as its documentation, release notes or request boards show.

### Audiobooks

Rivals: Audiobookshelf (the dedicated self-hosted audiobook server), Plex
(music library plus Plexamp or Prologue) and Jellyfin 12.

| Feature | What the user gets | Audiobookshelf | Plex (+ Plexamp, Prologue) | Jellyfin 12 | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Dedicated audiobook library | Books behave as books, not as albums by an "artist" | Yes, the product's whole purpose | No. The community puts audiobooks in a music library; the request has been open since January 2013 | Partial. A "Books" library type holds audiobooks, ebooks and comics together | Audiobookshelf, because every screen assumes a book | Emby documents an audiobook folder layout; whether it has a distinct library type is (unverified) |
| Multi-file book as one item | A 40-file MP3 rip plays as one book with one progress bar | Yes | No, files are album tracks | One folder per book; users ask for books split over many files to be treated as one (request comments, 2023) | Audiobookshelf | Needs a virtual timeline across files |
| Embedded chapters | Jump by chapter inside an M4B or MP3 | Yes | Plex and Plexamp treat an M4B as one long track and ignore its chapters; Prologue on iOS reads them | Chapter extraction added in 12.0; the web player has no chapter-skip control according to one 2026 guide | Audiobookshelf, and Prologue on iOS | Chapter formats: MP4 Nero `chpl` and QuickTime chapter tracks, ID3v2 `CHAP`/`CTOC`, Vorbis comments, Matroska chapters |
| Chapter editor and lookup | Fix bad chapters or fetch correct ones by ASIN | Yes; since 2.22.0 the lookup can strip Audible intro and outro branding | No | No (unverified) | Audiobookshelf | Lookup depends on Audible-derived community data |
| Metadata matching | Covers, blurbs, series and narrators filled in | Audible matching since 1.7.0, Google Books, iTunes and Open Library (provider list partly unverified), plus custom metadata providers since 2.8.0 | Third-party Audnexus agent pulls Audible's public catalogue | Google Books, Open Library, embedded tags and OPF files; a July 2026 review could not match MP3 audiobooks | Audiobookshelf | Audible-sourced data carries terms-of-service risk |
| Authors and narrators as people | Browse everything one narrator read | Multiple narrators per book since 1.7.0 | Narrator only through tag conventions | Authors tab added in 12.0; narrator browsing (unverified) | Audiobookshelf | |
| Series with reading order | Book 3 follows book 2; gaps visible | Series with sequence numbers; "show missing books in series" requested (26 upvotes) | Through collections by convention (unverified) | Series parsed from filenames in 12.0; a separate series request was filed in September 2026 (4 votes) | Audiobookshelf | Sequence values can be "0", "1.5" or "Prequel"; store a string plus a sort key |
| Resume across devices | Start on the phone, finish on the TV | Yes, per user | Yes; resume position syncs reliably according to a 2026 review | Yes, per user | Audiobookshelf | |
| Finished state and dates | A shelf of finished books with dates | Yes; editable start and finish dates requested (30 upvotes) | Played flag only (unverified) | Played flag (unverified) | Audiobookshelf | |
| Playback speed | Listen at 1.5x without chipmunk voices | Yes (unverified range) | Plexamp has a speed setting; Plex marked a 1,298-vote playback speed request implemented | Depends on the client; commenters call it a must-have (2024) | Audiobookshelf and Prologue | Pitch must be preserved |
| Skip intervals | Jump back 30 seconds after a distraction | Yes (unverified intervals) | Plexamp (unverified) | Client-dependent (unverified) | Tie (unverified) | |
| Sleep timer | Playback stops at the end of the chapter | Presets and custom times since 2.2.19; "end of chapter" since 2.12.0 | Plexamp added one after community requests | A request for an episode-count sleep timer has 11 votes (June 2026) | Audiobookshelf | Pocket Casts also offers "after N chapters" on Android |
| Bookmarks with notes | Mark a passage to return to | Yes; read endpoints added in 2.36.0 | No (unverified) | No (unverified) | Audiobookshelf | |
| Offline downloads | Listen on a plane | Yes in the Android and iOS apps | Plexamp downloads (unverified) | Varies by app; offline mode on Android is Jellyfin's most-voted request (1,820 votes) | Audiobookshelf | |
| Background and lock-screen playback | Lock the phone and keep listening | Yes | Yes in Plexamp | Background audiobook playback on iOS was added in 12.0; earlier users reported audio stopping on lock | Audiobookshelf and Plexamp | |
| CarPlay and Android Auto | Car controls and browsing | Android Auto yes; CarPlay is the app's top request (100 upvotes) | Plexamp (unverified) | Android Auto requested in comments (2025) | Plexamp (unverified) | |
| Casting | Send to a Chromecast speaker | Yes since 1.7.0 | Yes (unverified) | Yes (unverified) | Tie | |
| Queue | Line up a book and two podcast episodes | No; queueing is the app's second-most-upvoted request (99) | Music queue, which mixes with music | Music queue | Nobody | A commenter asked for a widget that resumes the book, not the music |
| Listening statistics | Time listened, a year in review | Stats page and "Year in Review" images since 2.7.0 | No (unverified) | No | Audiobookshelf | |
| Collections and playlists | Group books by theme | Yes | Playlists | Collections tab in 12.0 | Tie | |
| Share link | Send one book to a friend, with expiry | Admin-only share links with expiration since 2.11.0 | Whole-library sharing to Plex accounts | Temporary sharing links are an open request with 580 votes | Audiobookshelf | Must be signed, scoped and expiring |
| RSS feed for a book | Listen in any podcast app | Yes ("open RSS feed"), including MRSS since 2.18.1 | No | No | Audiobookshelf | A public URL is a security trade-off |
| Merge and embed tools | Turn an MP3 folder into one tagged M4B | Yes, merge to M4B and embed metadata | No | No | Audiobookshelf | Writes to library files |
| Per-user permissions | Children cannot see explicit titles | Yes; new users deny explicit content by default since 2.14.0 | Managed users (unverified detail) | Parental ratings (unverified detail) | Audiobookshelf | |
| Single sign-on | Sign in with the household identity provider | OpenID Connect with roles since 2.9.0; validation hardened in 2.36.0 | Plex account only | OIDC is a planned request with 1,191 votes | Audiobookshelf | |
| API keys and session control | Scripts, and "log out all devices" | API keys since 2.26.0, session list and revoke in 2.36.0, websocket auth in 2.37.0 | Account tokens (unverified) | API keys (unverified) | Audiobookshelf | |
| Send to e-reader | Email an ebook to a Kindle | Yes | Not applicable | No (unverified) | Audiobookshelf | |
| Read-along (ebook and audio in sync) | Switch between reading and listening without losing the place | No; requested (91 and 55 upvotes) | No | Not supported, per the books documentation | Storyteller, a separate self-hosted app | Uses EPUB 3 media overlays |
| Native iOS app | Install from the App Store | TestFlight only, capped at 10,000 testers; no release date | Prologue (third party) | Official app (book quality unverified) | Prologue | |
| Format coverage | M4B, MP3, FLAC, Opus, MKA all play | Broad; Opus in Matroska fixed in 2.33.2 | M4B plays | Broad | Tie | Audible AAX is DRM-protected; native support in any of them is (unverified) |
| Transcripts | Read a generated transcript while listening | Requested: Whisper (44 upvotes), transcripts (27) | No | No | Nobody | |
| Scheduled backups | Restore the server after a disk failure | Yes, automated | (unverified) | (unverified) | Audiobookshelf | |
| Remote listening cost | Listen away from home for free | Free | Free; music is exempt from Plex's remote-playback paywall | Free | Tie | Applies because Plex users store audiobooks as music |

### Podcasts

Rivals: Pocket Casts (the reference podcast app), Audiobookshelf (the
self-hosted podcast server people actually use), and the video servers
together, because none of them supports podcasts today.

| Feature | What the user gets | Pocket Casts | Audiobookshelf | Plex, Jellyfin, Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Directory search and subscribe | Find a show by name | Yes | iTunes search with selectable region since 2.10.0 | Plex removed podcasts on 15 April 2022; Jellyfin has none (563 votes, its 16th most-wanted request); Emby (unverified) | Pocket Casts | |
| OPML import and export | Move subscriptions between apps | Yes (unverified) | OPML import since 2.0.18; export (unverified) | Not applicable | Tie | |
| Auto-download new episodes | New episodes are just there | Yes, on the phone | Yes, on the server, on a schedule | Not applicable | Depends on goal | |
| Server-side archive | Keep episodes after the publisher deletes them | No | Yes, files land in your library | Not applicable | Audiobookshelf | |
| Keep the latest N episodes on the phone | Commute is ready without thinking | Yes | Requested in the app (46 upvotes) | Not applicable | Pocket Casts | |
| Up Next queue | Line up episodes from several shows | Yes, with continuous playback | Requested (99 upvotes) | Not applicable | Pocket Casts | |
| Rule-based lists | "Unplayed, under 30 minutes" | Yes (filters; naming may have changed to playlists, unverified) | No (unverified) | Not applicable | Pocket Casts | |
| Played state and archiving | Finished episodes leave the list | Auto-archive rules | Finished-episode handling requested (31 upvotes) | Not applicable | Pocket Casts | |
| Oldest-first order | Serial shows play in order | Per-show sort (unverified) | Requested (35 upvotes) | Not applicable | Pocket Casts | |
| Variable speed | 0.5x to 3x | 0.5x to 3x on mobile, up to 5x on web and desktop | Yes | Not applicable | Pocket Casts | |
| Trim silence | Shorter pauses without changing speech | Three levels: Mild, Medium, Mad Max | No (unverified) | Not applicable | Pocket Casts | Mobile only |
| Volume boost | Quiet hosts become audible | Yes on mobile | Requested in the app (26 upvotes) | Not applicable | Pocket Casts | |
| Per-show effects | Speed 1.8x for one show, 1.0x for another | Yes, global or per podcast | No (unverified) | Not applicable | Pocket Casts | |
| Skip intro and outro per show | Ads and theme music skipped automatically | Yes (unverified) | No (unverified) | Not applicable | Pocket Casts (unverified) | |
| Chapters | Jump to the segment you want | MP3 chapters, Podcast Index JSON and Podlove; iOS also AAC chapters | Episode chapters included in sessions since 2.2.21 | Not applicable | Pocket Casts | Podcasting 2.0 `<podcast:chapters>` |
| Generated chapters | Chapters for shows that have none | Offered on most platforms but temporarily switched off at the time of writing | No | Not applicable | Nobody right now | |
| Transcripts | Read along or search an episode | Yes, from the Podcasting 2.0 transcript tag (2.5 million episodes) | Requested (27 upvotes) | Not applicable | Pocket Casts | |
| Funding and recommendations | Support the creator, discover related shows | Funding tag and podroll | No (unverified) | Not applicable | Pocket Casts | |
| Bookmarks | Save a moment, including from headphones | Paid (Plus or Patron) | Bookmarks exist for items; for podcast episodes (unverified) | Not applicable | Pocket Casts, behind a paywall | |
| Folders | Group subscriptions | Paid; smart folders too | No | Not applicable | Pocket Casts, behind a paywall | |
| Sleep timer | Stop after this episode, or after N chapters | Fixed times, custom, end of episode, N episodes; N chapters on Android | Time and end of chapter | Not applicable | Pocket Casts | |
| Video podcasts | Watch shows that publish video | Yes | Audio focus (unverified) | A Jellyfin commenter asked for video podcasts too | Pocket Casts | |
| Your own files | Play a lecture recording alongside podcasts | Files section, but files cannot go into playlists | It is your server already | Not applicable | Audiobookshelf | |
| Cross-device sync | Same place on phone and web | Free | Yes, your server | Not applicable | Tie | Pocket Casts needs an account |
| Web and desktop | Listen at a desk | Free web listening; effects limited to speed | Web client | Not applicable | Pocket Casts | Sleep timer is mobile-only in Pocket Casts |
| Statistics | Time listened and time saved | Yes, including time saved by effects | Library stats | Not applicable | Pocket Casts | |
| Episode artwork | Each episode shows its own image | Yes (unverified) | Requested (42 upvotes) | Not applicable | Pocket Casts | |
| Open sync protocols | Use any podcast app against your server | Proprietary sync (unverified) | None (unverified) | Not applicable | Nobody | gPodder sync and the Open Podcast API exist |
| Safe feed fetching | The server cannot be tricked into calling internal addresses | Not applicable (cloud) | SSRF filtering for episode downloads added in 2.34.0 | Not applicable | Audiobookshelf | |
| Subsonic-protocol podcasts | Existing Subsonic apps see podcasts | Not applicable | No (unverified) | Not applicable | Not applicable | OpenSubsonic defines eight podcast endpoints |

### Music videos and concerts

Rivals: the three video servers, which are the products that handle this.

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Video linked to a track | The music video appears on the song | Yes; the file name must start with the track's file name and sit in the same folder | No; a 2026 request to match music videos to the music library has 9 votes | (unverified) | Plex | |
| Typed artist extras | Live sets, concerts, interviews, behind-the-scenes, lyric videos on the artist page | Yes, through suffixes such as `-live`, `-concert`, `-interview`, `-behindthescenes`, `-lyrics`, `-video` | No | (unverified) | Plex | A file-naming convention Gunmetal could accept for easy migration |
| Separate music video folder | Keep videos away from audio files | A single global music video path for the whole server | Its own library type | Music video library type (unverified) | Jellyfin, because the folder is per library | |
| Online metadata for music videos | Titles and artwork filled in | (unverified) | None by default; NFO files only | (unverified) | (unverified) | |
| Combined music and video browsing | One artist page for songs and videos | Yes | Requested (38 votes since 2020) | (unverified) | Plex | |
| More views for music videos | Browse by artist, year, genre | Through the music library | Requested (15 votes) | (unverified) | Plex | |
| Multiple video angles | Concert discs with alternate angles | (unverified) | Requested (16 votes, 2026) | (unverified) | Nobody | |
| Concert films with per-song chapters | Jump to a song in a two-hour concert | Chapters if embedded (unverified) | Chapters if embedded (unverified) | (unverified) | Nobody links chapters to songs (unverified) | |
| Video mode inside a music queue | Swap the audio for its video while queued | (unverified) | No | No (unverified) | (unverified) | |
| Remote viewing cost | Watch away from home | Personal video needs Plex Pass or Remote Watch Pass on updated apps since 29 April 2025; whether music videos count as video is (unverified) | Free | Free | Jellyfin and Emby | |

### Home videos

Rivals: the three video servers. Immich is mentioned in notes because many
people now keep phone videos there.

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Library with no online lookup | Family videos are not mismatched to films | "Personal Media" agent under a Movies library; file name becomes the title | "Home Videos and Photos" library type | Home videos and photos type (unverified) | Tie | |
| Embedded metadata | Titles and dates from the file itself | Local Media Assets reads MP4, M4V and MOV tags | (unverified) | (unverified) | Plex | |
| Date timeline | Browse by when it was filmed | Timeline view in the Plex Photos app, which also shows videos | Folder view and default tabs added in 12.0; no timeline (unverified) | (unverified) | Immich | |
| Photos and videos together | One place for a trip | Photo libraries show both | Same library type | (unverified) | Tie | |
| Upload from the phone | New clips arrive without effort | Camera upload existed as a Plex Pass feature in 2016 and 2017; status in the current apps (unverified) | No built-in camera upload (unverified) | Camera upload on Android and iOS, with per-device folders | Immich | |
| Remote viewing cost | Grandparents watch from their house | Paid since 29 April 2025 for personal video on updated apps | Free | Free | Jellyfin and Emby | |
| Phone HDR and HEVC handling | iPhone clips look right on any screen | Transcodes | Transcodes | Transcodes; hardware acceleration is a Premiere feature | Tie | Immich has open complaints about dark HDR video (15 upvotes) |
| Share one clip by link | Send a link that expires | (unverified) | Temporary file sharing links are an open request (580 votes) | (unverified) | Immich | |
| Where it was filmed | Map of clips | "Places" launched in 2017 and has since been withdrawn | No | No (unverified) | Immich | |
| Folder sync and backup | Copy media to another drive | No (unverified) | No (unverified) | Folder Sync is a Premiere feature | Emby | |

### Photos

Rivals: Immich (the leading self-hosted photo service), Plex and Jellyfin.

| Feature | What the user gets | Immich | Plex | Jellyfin | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Automatic phone backup | Photos leave the phone without thinking | Foreground and background backup, selective albums | Camera upload historically (current status unverified) | No (unverified) | Immich | Emby documents camera upload |
| Backup robustness | Big videos upload over bad links | Resumable uploads requested (38 upvotes); chunked uploads planned; backup cut-off date requested | (unverified) | Not applicable | Immich, with gaps | |
| Free phone space | Delete originals once safe | Since v2.5.0 | No | No | Immich | |
| Timeline with scrubbing | Fly through 20 years by date | Virtual scroll and draggable scrollbar | Timeline view in the Plex Photos app | No timeline (unverified) | Immich | |
| Folder view | Browse the folders you made | Yes | "Library view" by folders and albums | Added in 12.0 | Tie | |
| Albums | Curated sets | Yes | Yes (unverified detail) | Collections (unverified) | Immich | |
| Sharing with other users | Family sees the same album | Shared albums; a sharing redesign is under feature freeze (749 upvotes, 260 comments) | Library-level sharing (unverified detail) | Per-user library access | Immich | |
| Public links | Send an album to someone without an account | Yes; custom link slugs on mobile in v2.6.0 | No (unverified) | Requested (580 votes) | Immich | |
| Partner sharing | Two people see each other's whole library | Yes | No | No | Immich | |
| Face recognition | Every photo of grandma | Yes, with clustering and manual tagging | Never shipped; staff said in 2017 it was "on the list"; requests total 60 votes plus a closed thread with 110 likes | Requested (15 votes) | Immich | |
| Search by content | Type "dog" and find dogs | CLIP-based smart search | Auto-tagging launched in 2016 and has since been withdrawn | Requested (13 votes) | Immich | |
| Text in images (OCR) | Find the receipt by its words | Web since v2.2.0, mobile since v3.0.0 | No | No | Immich | |
| Map | See where photos were taken | Global map, side panel since v2.6.0 | "Places" launched in 2017 and has since been withdrawn | No | Immich | |
| Memories | "Five years ago today" | Yes | "Recommended view" of curated highlights | No | Immich | |
| Live and motion photos | Short clips attached to stills play back | Backup and playback | (unverified) | No (unverified) | Immich | |
| RAW files | Camera RAWs are visible | Yes; some DNG files fail (32 upvotes, 112 comments) | Canon CR3 requested since 2018 (26 votes) | (unverified) | Immich | |
| HEIC and HEIF | iPhone photos open everywhere | Yes; HDR HEIC previews look flat (13 upvotes) | Requested since 2017 (637 votes, still open) | (unverified) | Immich | |
| 360-degree photos | Pan around a panorama | Web only | No | No | Immich | |
| Stacks | RAW and JPEG, or a burst, shown as one | Yes; "stacks on all views" (98 upvotes); automatic burst stacking planned | No | No | Immich | |
| Non-destructive editing | Crop and rotate without touching the original | Web since v2.5.0, mobile since v3.0.0 | Rotate and zoom requested (9 votes) | No | Immich | |
| Tags and embedded keywords | Keywords written by Lightroom show up | Tags on web | "Use embedded tags" requested since 2016 (283 votes) | Tag search requested (14 votes) | Immich | |
| Slideshow on a TV | Photos on the living-room screen, with music | Mobile slideshow since v3.0.0; a first-party TV app (unverified) | "Slideshow for all clients" requested (121 votes) | Slideshow with configurable delay added in 12.0; screensaver suppressed while viewing | Jellyfin on TV (unverified across platforms) | Jellyfin users ask for slideshow with background music (5 votes) |
| Digital photo frame | An idle screen cycles photos | (unverified) | (unverified) | Requested (12 votes) | Nobody | |
| Duplicates | One copy of each photo | Upload de-duplication; duplicate resolution in v2.7.0 | No | No | Immich | |
| Existing folders, read-only | Point at a NAS folder without importing | External libraries; edits on read-only libraries fail silently (27 upvotes) | Libraries are folders | Libraries are folders | Tie, with Immich's caveat | |
| Storage layout | Uploaded files land in a folder scheme you choose | User-defined storage template | No | No | Immich | |
| Archive, favourites, trash | Hide screenshots, star the good ones | Yes | (unverified) | Favourites | Immich | |
| Download originals | Get the full-resolution file back | Yes | Requested since 2014 (38 votes) | (unverified) | Immich | |
| Automation | "Add every photo of Sam to the Sam album" | Workflows preview in v3.0.0 | No | No | Immich | |
| Integrity and database backups | Know the library is intact | Integrity checks in v3.0.0; database backup and restore in v2.5.0 | (unverified) | (unverified) | Immich | Immich says its database backup does not include the photos |
| Video playback for phone clips | Clips stream smoothly | Real-time HLS transcoding as a preview in v3.0.0 | Yes | Yes | Tie | |
| Hardware floor | Runs on a small box | 6 GB RAM minimum (4 GB with machine learning off), 2 cores minimum, Postgres on local SSD | (unverified) | (unverified) | Jellyfin and Plex are lighter (unverified) | |
| Remote access cost | View photos away from home | Free | Free; photos are exempt from the remote-playback paywall | Free | Tie | |
| App quality and commitment | The app keeps improving | Very active; 115,500 GitHub stars | Plex Photos for iOS rated 1.3 out of 5 from 293 ratings and last updated June 2025; photo libraries removed from the main iOS and Fire TV apps in August 2026 | Steady but minimal | Immich | |

### Ebooks

Rivals: Kavita, Komga and Jellyfin 12. Audiobookshelf and Booklore appear in
notes.

| Feature | What the user gets | Kavita | Komga | Jellyfin 12 | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Formats | EPUB, PDF and Kindle formats are recognised | EPUB, PDF, comics and images | EPUB, PDF, CBZ, CBR | azw, azw3, epub, mobi, pdf plus comic archives | Jellyfin by breadth of list | Audiobookshelf reads EPUB, PDF, MOBI, CBR, CBZ; Emby lists pdf, epub, mobi, cbr, cbz, azw3 |
| Web EPUB reader | Read in a browser | Overhauled in v0.8.8, variable fonts in v0.9.1 | Yes; table-of-contents parsing fixed in 1.24.4 | Redesigned in 12.0, with font size control | Kavita | |
| Typography controls | Font, size, margins, themes | Custom fonts including Google Fonts | (unverified) | Font size | Kavita | |
| Highlights and annotations | Mark and note passages | Yes since v0.8.8 | No (unverified) | No | Kavita | Booklore has them too; Audiobookshelf users request them (25 and 33 upvotes) |
| Progress in the server | Pick up where you stopped on another device | Yes, with reading sessions and re-read counts since v0.8.9 | Yes | Progress indicators "work again" in 12.0 | Kavita | |
| OPDS catalogue | E-reader apps browse and download | OPDS and page streaming; progress shown as title glyphs | OPDS 1.2 with page streaming, and OPDS 2 | No (unverified; a plugin may exist) | Komga, for having both OPDS versions | Audiobookshelf users request OPDS (73 upvotes) |
| KOReader sync | Progress shared with KOReader devices | Yes, widened across file types in v0.8.9 | Yes; EPUB progress only at chapter starts | No (unverified) | Kavita (unverified edge) | Komga warns that mid-chapter positions are lost |
| Kobo sync | A Kobo shows the server library as if it were the store | Planned (unverified) | Yes, EPUB only, converted to KEPUB; chapter-level progress for reflowable books | No | Komga | Kobo sync is Audiobookshelf's most-upvoted request (258) |
| Send to Kindle or email | One tap to a device | (unverified) | No (unverified) | No (unverified) | Audiobookshelf and Booklore | |
| Embedded metadata | Titles and authors from the file | Yes | Yes | OPF, ComicInfo and ComicBookInfo read natively since 12.0 | Tie | |
| Online metadata | Covers and blurbs fetched | Paid through Kavita+ (Hardcover, MangaBaka, ComicBookRoundup) | (unverified) | Google Books, Open Library, ComicVine | Jellyfin, free | Kavita says no base feature will be paywalled |
| Series and reading order | Books in order, with gaps | Yes | Yes | Series from filenames in 12.0 | Kavita and Komga | |
| Reading lists | Cross-series reading orders | Overhauled in v0.9.0, with CBL import | Read lists | No (unverified) | Kavita | |
| Want to read | A wishlist shelf | Yes | No (unverified) | No | Kavita | |
| Statistics | Time read, shareable profile | Since v0.8.9 | (unverified) | No | Kavita | |
| Smart filters and collections | Saved searches | Yes | Collections; metadata editing in NextUI | Collections tab | Kavita | |
| Content restrictions | Age limits per user | Yes (unverified detail) | Yes, also enforced for Kobo since 1.25.0 | Parental ratings (unverified detail) | Komga | |
| Bulk download | Take a whole series offline | Multi-download in v0.9.0 | (unverified) | A request asks to let users download ebooks (1 vote) | Kavita | |
| Single sign-on | OIDC login | Since v0.8.8 (release notes say it had 170+ upvotes) | OAuth2 login | Planned (1,191 votes) | Kavita | |
| External scrobbling | Progress goes to Hardcover or AniList | Paid (Kavita+) | No (unverified) | No | Kavita, behind a paywall | |
| Read-along with audio | Listen and read in one place | No | No | Not supported | Storyteller | |
| Security record | Patched quickly | Critical security hotfix in v0.9.0.2 (May 2026), details initially withheld | (unverified) | Security fixes in 12.0 (unverified) | Not comparable | |

### Comics and manga

Rivals: Kavita, Komga and Jellyfin 12.

| Feature | What the user gets | Kavita | Komga | Jellyfin 12 | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Archive formats | CBZ, CBR, CB7 all open | Yes (CB7 unverified) | CBZ and CBR; RAR5 in plain Java since 1.26.0, solid RAR4 since 1.25.0 | cb7, cbr, cbt, cbz | Komga, for RAR without native code | |
| ComicInfo metadata | Writers, artists, issue numbers from the file | Yes | Yes | Native since 12.0; artist metadata gaps reported (5 votes) | Kavita and Komga | |
| Page reader modes | Single, double page, right-to-left manga | Yes | Yes (unverified detail) | Unified fullscreen and swipe; a "richer comic reader" is started (44 votes) | Kavita | |
| Webtoon vertical scroll | Long strips scroll smoothly | Yes, improved in v0.9.0 | (unverified) | Requested (12 votes) | Kavita | |
| Page streaming to apps | Apps fetch pages without downloading the archive | OPDS page streaming | OPDS page streaming 1.2 | No (unverified) | Komga and Kavita | |
| Cover generation | A cover for every issue | Yes; Kavita+ covers stored at full size cannot be reclaimed (open issue) | Yes | Automatic posters since 12.0 | Tie | |
| Number parsing | "Vol 2 Ch 14.5" sorts right | Yes, with known edge cases (open issues) | Yes | Volume and chapter from filenames since 12.0; number-order request (4 votes) | Kavita | |
| Page counts | Know how long an issue is | Yes | Yes | Since 12.0 | Tie | |
| Mobile reader apps | Read on a phone app | Third-party apps through OPDS or extensions (unverified) | Third-party apps through OPDS or extensions (unverified) | Official apps (unverified) | Tie (unverified) | |
| Pinch zoom on mobile | Read small lettering | (unverified) | (unverified) | (unverified) | (unverified) | Audiobookshelf's app lacks it (29 upvotes) |
| Reading-list import | Follow an event crossover in order | CBL import | Read lists | No | Kavita | |
| New interface | Usable on phones | Responsive (unverified) | NextUI beta since 1.26.0 | Modern books layout in 12.0 | Komga (unverified) | |

## Pain points and unmet demand

Counts were read on 2 October 2026.

### Audiobooks

- **Plex has never had audiobooks.** "Support for audiobooks" has 2,264 votes
  and 1,157 posts since January 2013, and was still being posted to on
  29 September 2026. Recent posters say they use Prologue or Audiobookshelf
  instead. https://forums.plex.tv/t/support-for-audiobooks/27518
- **Plexamp does not understand books either.** "Audiobook Support in
  Plexamp" has 136 votes.
  https://forums.plex.tv/t/feature-request-audiobook-support-in-plexamp/577242
- **Plex ignores chapters.** Plex and Plexamp play an M4B as one continuous
  track, so users need third-party apps for chapter navigation.
  https://www.own.audio/learn/plex-for-audiobooks/
- **Jellyfin's support is basic.** "Audiobook support" is Jellyfin's eighth
  most-wanted request (857 votes, status "started"). The maintainers'
  response calls the support fairly basic. Commenters list playback speed,
  books split over many files, Android Auto, and a resume widget that does
  not hijack the music queue; one 2026 commenter says it works but feels
  like an afterthought.
  https://features.jellyfin.org/posts/243/audiobook-support
- **Smaller Jellyfin gaps.** Auto-advancing to the next chapter (26 votes),
  audiobook series (4 votes, September 2026), Android TV audiobooks
  (5 votes).
  https://features.jellyfin.org/posts/2552/autoplay-next-audiobook-chapter,
  https://features.jellyfin.org/posts/4148/audiobook-series-support,
  https://features.jellyfin.org/posts/3017/audiobook-support-in-android-tv-app
- **Jellyfin on iOS stopped audio on lock.** A 2026 guide and a July 2026
  review both report audio stopping when the phone locks; 12.0 added
  background audiobook playback on iOS.
  https://jellywatch.app/blog/jellyfin-audiobooks-podcasts-setup-guide-2026,
  https://www.xda-developers.com/jellyfin-finally-convinced-me-to-ditch-three-separate-media-apps/
- **Audiobookshelf has no App Store app.** The iOS app has been in TestFlight
  for years. Apple caps TestFlight at 10,000 testers and the project gives no
  release date. The release checklist issue has 96 comments.
  https://audiobookshelf.org/docs/faq/app-beta/,
  https://github.com/advplyr/audiobookshelf-app/issues/541
- **Audiobookshelf app gaps.** CarPlay (100 upvotes), a queue for books and
  podcast episodes (99), self-rating (75), auto-sync of the latest episodes
  (46), volume boost (26).
  https://github.com/advplyr/audiobookshelf-app/issues/475,
  https://github.com/advplyr/audiobookshelf-app/issues/416,
  https://github.com/advplyr/audiobookshelf-app/issues/236,
  https://github.com/advplyr/audiobookshelf-app/issues/613,
  https://github.com/advplyr/audiobookshelf-app/issues/195
- **Listening and reading do not connect.** Audiobookshelf's oldest popular
  request is syncing ebook and audiobook positions (91 upvotes), with a
  related EPUB 3 request (55). Jellyfin's documentation says read-along is
  not supported, and Jellyfin commenters asked for linking since 2019.
  https://github.com/advplyr/audiobookshelf/issues/189,
  https://github.com/advplyr/audiobookshelf/issues/3084
- **Transcripts.** Audiobookshelf users ask for Whisper transcription (44)
  and transcript viewing (27).
  https://github.com/advplyr/audiobookshelf/issues/1723,
  https://github.com/advplyr/audiobookshelf/issues/2919

### Podcasts

- **Plex removed podcasts in April 2022**, citing low usage, after users had
  reported broken "On Deck" and queue behaviour.
  https://techcrunch.com/2022/04/12/plex-pulls-the-plug-on-podcasts-and-web-shows/
- **Jellyfin has none.** "Podcast Support" has 563 votes (16th most wanted)
  and "Separate Library for Podcasts" has 50. Commenters split: some want it
  native, others say Pocket Casts or Audiobookshelf already do it well. One
  points to the Open Podcast API as the sync standard to follow.
  https://features.jellyfin.org/posts/48/podcast-support,
  https://features.jellyfin.org/posts/460/separate-library-for-podcasts
- **Audiobookshelf is a server, not yet a great podcast player.** Missing:
  a queue (99), episode images (42), oldest-first order (35), finished-episode
  handling (31), podcast-management permissions (43).
  https://github.com/advplyr/audiobookshelf/issues/1573,
  https://github.com/advplyr/audiobookshelf/issues/1321,
  https://github.com/advplyr/audiobookshelf/issues/615,
  https://github.com/advplyr/audiobookshelf/issues/1258
- **Pocket Casts charges for basics and splits features by platform.**
  Bookmarks and folders need Plus or Patron. Trim silence, volume boost and
  the sleep timer are not available on web or desktop. Generated chapters are
  switched off for now.
  https://support.pocketcasts.com/knowledge-base/bookmarks/,
  https://support.pocketcasts.com/knowledge-base/folders/,
  https://support.pocketcasts.com/knowledge-base/playback-effects/,
  https://support.pocketcasts.com/knowledge-base/chapters/

### Music videos and concerts

- **Jellyfin keeps music videos apart from music.** Combining the libraries
  (38 votes since 2020), more views (15), matching videos to tracks (9),
  multiple video tracks (16).
  https://features.jellyfin.org/posts/896/combine-music-and-music-videos-libraries,
  https://features.jellyfin.org/posts/2505/additonal-views-for-music-videos-library,
  https://features.jellyfin.org/posts/3782/match-music-video-to-music-library
- **Jellyfin fetches no music video metadata** by default; NFO files are the
  only route. https://jellyfin.org/docs/general/server/media/music-videos/
- Plex is good here. Its naming scheme links videos to tracks and attaches
  typed extras to artists.
  https://support.plex.tv/articles/205568377-adding-local-artist-and-music-videos/

### Home videos

- **Plex charges for remote personal video.** Since 29 April 2025, updated
  Plex apps need Plex Pass or a Remote Watch Pass to stream personal video
  remotely; music and photos are exempt.
  https://support.plex.tv/articles/requirements-for-remote-playback-of-personal-media/
- **No temporary share links in Jellyfin.** "Temporary direct file sharing
  links" has 580 votes and is open.
  https://features.jellyfin.org/posts/72/temporary-direct-file-sharing-links

### Photos

- **Plex retreated from photos.** Auto-tagging (2016) and Places (2017) both
  carry "no longer available" notices on their launch posts. The standalone
  Plex Photos iOS app is rated 1.3 out of 5 from 293 ratings and has not been
  updated since June 2025. A January 2026 thread asked whether the app was
  discontinued (2,996 views). In August 2026 photo libraries disappeared from
  the main Plex app on iOS and Fire TV; staff said photos would return.
  https://www.plex.tv/blog/introducing-new-game-photo-tag/,
  https://www.plex.tv/blog/put-photos-map/,
  https://apps.apple.com/us/app/plex-photos/id6504519803,
  https://forums.plex.tv/t/has-the-plex-photos-app-been-discontinued/935228,
  https://forums.plex.tv/t/do-not-update-plex-client-is-deleting-its-photo-library-from-plex-app/941381
- **Plex's long-open photo requests.** HEIC support (637 votes, open since
  2017), embedded tags (283, since 2016), slideshows on all clients (121),
  facial recognition (60, plus a closed thread with 110 likes), direct
  download (38), Canon CR3 (26).
  https://forums.plex.tv/t/195514, https://forums.plex.tv/t/168821,
  https://forums.plex.tv/t/327193, https://forums.plex.tv/t/167126,
  https://forums.plex.tv/t/feature-request-add-facial-recognition-and-support-tagging-of-faces/188604,
  https://forums.plex.tv/t/59571, https://forums.plex.tv/t/233284
- **Jellyfin photos are minimal.** Facial recognition (15), automatic tagging
  (13), tag search (14), photo frame mode (12) are all open with low votes,
  which suggests photo users have gone elsewhere.
  https://features.jellyfin.org/posts/3370/photo-facial-recognition,
  https://features.jellyfin.org/posts/796/automatic-photo-tagging-a-la-google-photos,
  https://features.jellyfin.org/posts/1220/read-and-search-by-photo-tags,
  https://features.jellyfin.org/posts/927/digital-photo-frame
- **Immich is strong but has real complaints.** Sharing is confusing enough
  that the team froze changes pending a redesign (749 upvotes, 260 comments).
  Others: stacks everywhere (98), resumable uploads (38), DNG failures (32),
  edits failing silently on read-only libraries (27), unreliable local-URL
  switching (21). Version 3.0.0 required users still on an old vector
  extension to migrate before upgrading.
  https://github.com/immich-app/immich/issues/12614,
  https://github.com/immich-app/immich/issues/16549,
  https://github.com/immich-app/immich/issues/22385,
  https://github.com/immich-app/immich/issues/13029,
  https://github.com/immich-app/immich/issues/10538,
  https://github.com/immich-app/immich/issues/15220,
  https://github.com/immich-app/immich/releases/tag/v3.0.0
- **Immich is heavy for a small box**: 6 GB RAM minimum, Postgres on local
  SSD, and a separate machine-learning container.
  https://docs.immich.app/install/requirements

### Ebooks and comics

- **A reader is Plex's most-voted request.** "PLEXREADER: Comics, Books,
  PDFs" has 3,182 votes and 1,064 posts since January 2013.
  https://forums.plex.tv/t/26684
- **E-reader sync is the top Audiobookshelf request.** Kobo sync has 258
  upvotes; OPDS has 73.
  https://github.com/advplyr/audiobookshelf/issues/3504,
  https://github.com/advplyr/audiobookshelf/issues/1953
- **Sync precision is poor.** Komga's documentation says KOReader and Kobo
  sync of reflowable EPUBs records only chapter starts, so mid-chapter
  positions are lost when switching devices. Kavita encodes progress into
  OPDS titles as glyphs because most OPDS clients cannot sync.
  https://komga.org/docs/guides/koreader,
  https://komga.org/docs/guides/kobo,
  https://wiki.kavitareader.com/guides/features/opds/
- **Jellyfin's comic reader is thin.** A richer reader is "started"
  (44 votes); webtoon scrolling (12) and ComicInfo-specific metadata (14) are
  open. https://features.jellyfin.org/posts/1158/richer-comic-reader,
  https://features.jellyfin.org/posts/3030/seamless-vertical-scrolling-for-comics-drawn-in-webtoon-format,
  https://features.jellyfin.org/posts/2481/comic-manga-comicinfo-specific-metadata-server-plugin-side
- **Metadata is paywalled in Kavita.** External metadata and scrobbling are
  Kavita+ features, though the project promises no base feature will be
  locked. https://wiki.kavitareader.com/kavita+/
- **Projects come and go.** Booklore, an AGPL ebook server with Kobo
  and KOReader sync, now says it is entering maintenance mode in favour of a
  successor. https://github.com/booklore-app/booklore

## Where Gunmetal can be clearly better

### Verdict by media type

| Media type | Worth doing? | When | Reason |
|---|---|---|---|
| Audiobooks | Yes, strongly | Soon after the first music release | Very high demand, weak coverage in Plex and Jellyfin, and most of the work (parsers, gapless audio, offline, lock screen) is already in the music plan |
| Music videos and concerts | Yes | With the video milestone | A natural extension of the music model; Plex shows it works, Jellyfin shows what happens without linking |
| Home videos | Yes, cheaply | With the video milestone | It is personal video with no metadata lookup; mostly a date-first view over the existing video path |
| Podcasts | Yes, as a plugin | After audiobooks | Medium demand; Pocket Casts is free and excellent, so the case is privacy, archiving and one app |
| Ebooks and comics | Protocols first, readers later or never | After video | Plex's top request, but Kavita and Komga are good; OPDS and sync protocols deliver most of the value with little client work |
| Photos | Viewing and integration only | Late | Immich is far ahead and moving fast; competing on backup and machine learning would contradict the low-hardware goal |

### 1. Audiobooks in the music app, done properly

Ties to: Plex's 2,264-vote request, Jellyfin's 857-vote request, Plex
ignoring chapters, Audiobookshelf's missing App Store app, CarPlay (100) and
queue (99) requests.

The idea is that an audiobook is a first-class item in the same app as
music, with chapters, speed, an end-of-chapter sleep timer, bookmarks and
offline downloads, shipped in a real App Store app. Record 2 already commits
to background playback, lock-screen controls and offline downloads for
music; audiobooks reuse all of it.

What has to be true:

- The core crate parses chapters at scan time with no FFmpeg: MP4 `chpl` and
  QuickTime chapter text tracks, ID3v2 `CHAP` and `CTOC` frames (which carry
  start and end times in milliseconds and optional titles, images and URLs),
  Vorbis comment chapter tags, and Matroska chapters through the EBML code
  that already exists. This fits record 2, decision 3.
- A book is an ordered manifest of files with known durations, giving one
  global timeline. Chapters map to a file and an offset. Gapless playback
  across files reuses the music gapless engine.
- Positions are stored as global milliseconds, so re-splitting a book does
  not break them if total duration holds.
- The player changes rate with pitch preserved on every platform: libmpv on
  native clients and the browser's pitch-preserving playback rate on the web
  (unverified on every target TV).
- The play queue supports more than one resumable "listening context", so
  pausing a book to play an album does not lose either. No rival does this,
  and both Audiobookshelf and Jellyfin users ask for it.
- Metadata lookups (Audible-derived data, Open Library) live in plugins with
  explicit network grants, as record 2 requires for third-party services.

### 2. One progress journal for every kind of position

Ties to: Komga losing mid-chapter positions, Kavita encoding progress in
titles, the 91- and 55-upvote read-along requests, and Jellyfin's
long-standing book-linking comments.

Record 1 says watch history is the only irreplaceable data and lives in an
append-only, exportable log. Adjacent media break that assumption:
bookmarks, highlights, annotations, podcast subscriptions, photo albums and
names given to faces are all user data that a rescan cannot rebuild.

What has to be true:

- The log becomes a typed user-state journal. A position is one of: a time
  offset, a text locator, a page index out of a total, or a percentage. The
  Readium Locator model already defines a portable text locator (resource,
  progression within it, progression across the publication, position, and
  surrounding text), so Gunmetal does not need to invent one.
- Journal events also cover finished, bookmark (with note), annotation (with
  range and note), subscription and album membership.
- Each event records device and time so conflicts can be resolved by a
  stated rule.
- Exports exist in standard forms where they exist: OPML for subscriptions,
  KOReader-compatible progress, plain JSON for the rest.

With this in place, handing off between an audiobook and its ebook becomes a
mapping problem, not a schema change. Automatic alignment (what Storyteller
does to produce EPUB 3 media overlays) can come later as an optional,
sandboxed plugin, because it needs speech recognition.

### 3. Podcasts as a plugin with an explicit network grant

Ties to: Plex removing podcasts, Jellyfin's 563-vote request, Audiobookshelf's
missing queue and effects, Pocket Casts paywalling bookmarks and folders.

What has to be true:

- Feed fetching is a plugin with an outbound network grant, as record 2
  requires. The fetcher refuses private and loopback addresses and re-checks
  after every redirect, with size and time caps. Audiobookshelf only added
  SSRF filtering for episode downloads in 2026, which shows the risk is real.
- Feed parsing (RSS plus Podcasting 2.0 tags for chapters, transcripts,
  people, season, episode, GUID and funding) is pure logic and fits the core;
  the plugin does the I/O.
- Episodes can be streamed from the publisher or archived to the library.
- Bookmarks, folders, speed per show and the sleep timer are free, because
  they are features of Gunmetal's player, not a subscription.
- Trim silence and volume boost are possible on native clients through
  libmpv's audio filters (unverified for every platform). On the web they
  need Web Audio work, so they may be mobile-first at first, as in Pocket
  Casts.
- Record 2's OpenSubsonic adapter brings podcast compatibility nearly for
  free: OpenSubsonic defines podcast channel, episode and refresh endpoints,
  plus bookmarks and play-queue endpoints.
- Supporting the gPodder sync API or the Open Podcast API would let
  AntennaPod-style apps sync with Gunmetal (support details unverified).

Pocket Casts is free, excellent and syncs at no cost; Gunmetal should not
pretend otherwise. The pitch is no account, your own archive, and one app
for music, books and podcasts.

### 4. Music videos and live recordings as part of the music model

Ties to: Jellyfin's unlinked music video library (38, 15, 9 votes) and its
lack of metadata.

What has to be true:

- The music model from record 2 (artists, release groups, albums, tracks)
  allows a video recording to be attached to a track and typed extras
  (live, concert, interview, behind the scenes, lyric video) to be attached
  to an artist. Accepting Plex's filename suffixes would make migration easy;
  a naming convention is not trade dress.
- A concert film carries chapters that can each map to a track, so watching
  a concert can count as plays of those songs. No rival appears to do this
  (unverified).
- The player can swap a queued track for its video and back.
- All of this needs the video remuxer, so it ships with the video milestone.
  Only the relationships need to exist in the first schema.

### 5. Home videos as a date-first personal library

Ties to: Plex's remote paywall for personal video, Jellyfin's 580-vote share
link request, and Plex withdrawing Places.

What has to be true:

- Capture date and location come from the MP4 and QuickTime metadata the
  core already parses (and Matroska's date field), with filename dates as a
  fallback (exact keys unverified). Timezones need care.
- No online lookup is attempted for this library kind.
- Remote viewing is free, as it is for everything in Gunmetal.
- Share links are signed, scoped to one item and expiring, which record 1,
  decision 6 already calls for.

### 6. Photos: view on every screen, do not try to out-build Immich

Ties to: Plex's photo regressions, Jellyfin's minimal photo support, Plex's
121-vote slideshow request, and Immich's hardware floor.

The honest call is that Immich is better than anything Gunmetal could build
soon, and its machine-learning features contradict Gunmetal's low-hardware
goal. What Gunmetal can do well is show photos on TVs and big screens, with
slideshows and music, which Plex has let slide.

What has to be true:

- Option A: an Immich plugin, granted network access to one host and given
  an Immich API key, shows albums and memories on Gunmetal's TV clients.
- Option B: a read-only folder photo library. EXIF and XMP parsing is pure
  logic and fits the core. Thumbnails need image decoding, which is attack
  surface (the 2023 libwebp heap overflow, CVE-2023-4863, is the standard
  example; details unverified here), so it runs in the same kind of sandbox
  as FFmpeg. Clients that decode HEIC natively get the original bytes, in
  keeping with "send the original".
- Motion photos are a still image with an MP4 appended; for HEIC and AVIF the
  video sits in an `mpvd` box. Gunmetal's MP4 parser can find the video
  without decoding anything.
- No phone backup and no face recognition in the first photo work. If they
  come, face recognition should run on the phone, not the server
  (feasibility unverified).

### 7. Ebooks and comics: protocols before readers

Ties to: Plex's 3,182-vote reader request, Audiobookshelf's Kobo (258) and
OPDS (73) requests, chapter-level sync in Komga, and Kavita's paid metadata.

What has to be true:

- An OPDS 2 catalogue and OPDS page streaming. For CBZ, pages are entries in
  a ZIP archive and can be sent as stored bytes without decoding the images,
  which matches Gunmetal's direct-play philosophy.
- A KOReader-compatible sync endpoint. KOReader identifies documents by an
  MD5-based hash and sends a progress string and percentage; Gunmetal can
  store both and keep a Readium locator alongside, so a position is not cut
  back to a chapter start.
- Kobo sync later, after checking the protocol and its legal position
  (unverified).
- EPUB OPF metadata and ComicInfo.xml parsed in the core. XML parsing must
  refuse external entities and entity expansion.
- Readers in Gunmetal's own clients come last, if at all. On TVs they matter
  little; on phones, people already have good OPDS readers.
- Metadata fetching is a free plugin, not a paid tier.

### 8. Cheap insurance for the first data model and player

None of this needs to be built now, but the first schema and player should
not prevent it:

- An item has a kind (music track, audiobook, podcast episode, video, photo,
  book, comic) instead of the schema assuming everything is a track or a
  film.
- People have roles beyond "artist": author, narrator, host, guest, writer,
  penciller.
- Series membership with a string sequence plus a sort key.
- A playable item can span several files with one timeline.
- Chapters are a general structure (start, end, title, optional image and
  link) used by audiobooks, podcasts, concerts and films alike.
- Positions are typed (time, locator, page, percent) in the journal from
  day one.
- Items can link to each other with a typed relation: "video of" a track,
  "audio of" an ebook, "extra for" an artist.
- The play queue supports several resumable contexts.
- Items can be backed by a remote URL (a podcast episode not yet
  downloaded).
- Libraries are not always read-only from the user's point of view: albums,
  annotations and subscriptions are user data stored in the journal, not in
  the rebuildable cache.

## Risks and hard parts

- **Scope.** Each type needs its own screens, metadata sources and edge
  cases. Audiobookshelf's single maintainer has kept the iOS app in beta for
  years; Booklore entered maintenance mode. Gunmetal's music-first schedule
  is the thing to protect.
- **Parser attack surface.** Images, PDFs, RAR archives and XML (EPUB, OPF,
  ComicInfo, RSS, XMP) are all classic sources of exploits. XML parsing in
  the core must disable external entities and entity expansion. Pixel
  decoding and PDF rendering belong in the sandbox, never the core.
- **RAR.** CBR files are RAR archives. The reference unrar code is C with a
  licence that restricts use (details unverified), and a mature pure-Rust RAR
  decoder is (unverified). Komga solved this in Java. Gunmetal may need a
  sandboxed decoder or to treat CBR as a lower tier.
- **PDF.** Rendering PDFs on the server is large and risky. Sending the
  original to clients that render PDF natively avoids most of it.
- **Hardware.** Face recognition, smart search, OCR and transcript alignment
  need machine learning. Immich needs 6 GB RAM with it on. Gunmetal's
  low-hardware promise means any of this must be optional, sandboxed and
  ideally run on client devices.
- **Outbound network.** Podcasts make the server talk to arbitrary hosts.
  That exposes the server's IP address to publishers and their analytics
  prefixes, and invites SSRF. The plugin model in record 2 helps, but the
  grant is broad by nature.
- **Durable user data.** Photos, annotations and albums are irreplaceable.
  Immich's documentation warns that its database backup does not include
  the photos themselves. Gunmetal's "SQLite is a cache" rule must be paired
  with a journal that holds all user state, or Gunmetal must refuse to own
  that data.
- **Position stability.** Re-ripping an audiobook changes durations;
  replacing an EPUB changes its hash and breaks KOReader sync; editing an
  EPUB can invalidate locators. Each needs a recovery rule.
- **Metadata legality.** Audible-derived audiobook data and Amazon-sourced
  book data come through unofficial channels. Keeping them in third-party
  plugins limits Gunmetal's exposure but does not remove it.
- **DRM.** Audible AAX files and DRM-protected ebooks should stay out of
  scope; supporting their removal is a legal risk for an open project.
- **Timezones.** Photos and phone videos often carry local time without an
  offset, so ordering a trip across timezones can be wrong (unverified for
  current phones).
- **Auth surface grows with every type.** Audiobookshelf fixed IDOR bugs in
  progress and session endpoints in 2026 and hardened OIDC in July 2026;
  Kavita shipped a critical security hotfix in May 2026. Every new
  endpoint needs per-object authorisation (record 1, decision 6) and tests.
- **TV and React Native readers.** Ebook readers on TVs are of little use,
  and EPUB rendering in React Native likely means a WebView-based engine
  (unverified choice). This is one reason to lead with protocols.

## Open questions

- Should audiobooks ship inside the first music release, given that chapter
  parsing sits in the same parsers, or be the first thing after it?
- Are audiobooks a separate library type, or a kind on audio items? Where do
  audio dramas, lectures and radio plays go?
- What is the conflict rule when two devices report different positions:
  latest timestamp, furthest position, or ask the user?
- Does Gunmetal ever write to user files (embedding tags, merging to M4B),
  or stay strictly read-only and offer exports instead?
- For podcasts, does the server archive episodes, do clients download from
  the publisher, or both? Who pays the privacy cost of fetching?
- Which podcast sync protocol, if any: gPodder, the Open Podcast API, or only
  OpenSubsonic through the planned adapter?
- For photos, is an Immich integration enough, or is a native read-only
  folder library needed for people who do not run Immich?
- Should Gunmetal build its own ebook and comic readers at all, or stop at
  OPDS, KOReader sync and later Kobo sync?
- Is an Audiobookshelf-compatible API adapter worth it, so its apps could
  connect, the way record 1 plans a Jellyfin adapter? The Audiobookshelf FAQ
  says its API is still being standardised.
- Do plays of a music video or a concert chapter count as plays of the
  track for statistics and scrobbling?
- What counts as "finished" for a book or episode (a percentage, the last
  chapter, or an explicit mark)?

## Sources

- https://github.com/advplyr/audiobookshelf/releases
- https://www.audiobookshelf.org/docs/
- https://audiobookshelf.org/docs/faq/app-beta/
- https://github.com/advplyr/audiobookshelf/issues/3504
- https://github.com/advplyr/audiobookshelf/issues/189
- https://github.com/advplyr/audiobookshelf/issues/1953
- https://github.com/advplyr/audiobookshelf/issues/3084
- https://github.com/advplyr/audiobookshelf/issues/1723
- https://github.com/advplyr/audiobookshelf/issues/2919
- https://github.com/advplyr/audiobookshelf/issues/1573
- https://github.com/advplyr/audiobookshelf/issues/1321
- https://github.com/advplyr/audiobookshelf/issues/615
- https://github.com/advplyr/audiobookshelf/issues/1258
- https://github.com/advplyr/audiobookshelf-app/issues/475
- https://github.com/advplyr/audiobookshelf-app/issues/416
- https://github.com/advplyr/audiobookshelf-app/issues/236
- https://github.com/advplyr/audiobookshelf-app/issues/541
- https://github.com/advplyr/audiobookshelf-app/issues/613
- https://github.com/advplyr/audiobookshelf-app/issues/195
- https://forums.plex.tv/t/support-for-audiobooks/27518
- https://forums.plex.tv/t/feature-request-audiobook-support-in-plexamp/577242
- https://forums.plex.tv/t/26684
- https://forums.plex.tv/t/195514
- https://forums.plex.tv/t/168821
- https://forums.plex.tv/t/327193
- https://forums.plex.tv/t/167126
- https://forums.plex.tv/t/feature-request-add-facial-recognition-and-support-tagging-of-faces/188604
- https://forums.plex.tv/t/59571
- https://forums.plex.tv/t/233284
- https://forums.plex.tv/t/69044
- https://forums.plex.tv/t/has-the-plex-photos-app-been-discontinued/935228
- https://forums.plex.tv/t/do-not-update-plex-client-is-deleting-its-photo-library-from-plex-app/941381
- https://support.plex.tv/articles/requirements-for-remote-playback-of-personal-media/
- https://support.plex.tv/articles/205568377-adding-local-artist-and-music-videos/
- https://support.plex.tv/articles/200265246-personal-media-movies/
- https://www.plex.tv/blog/introducing-new-game-photo-tag/
- https://www.plex.tv/blog/put-photos-map/
- https://apps.apple.com/us/app/plex-photos/id6504519803
- https://techcrunch.com/2022/04/12/plex-pulls-the-plug-on-podcasts-and-web-shows/
- https://www.own.audio/learn/plex-for-audiobooks/
- https://jellyfin.org/posts/jellyfin-release-12.0/
- https://jellyfin.org/docs/general/server/media/books/
- https://jellyfin.org/docs/general/server/media/music-videos/
- https://features.jellyfin.org/posts/243/audiobook-support
- https://features.jellyfin.org/posts/48/podcast-support
- https://features.jellyfin.org/posts/460/separate-library-for-podcasts
- https://features.jellyfin.org/posts/72/temporary-direct-file-sharing-links
- https://features.jellyfin.org/posts/2552/autoplay-next-audiobook-chapter
- https://features.jellyfin.org/posts/4148/audiobook-series-support
- https://features.jellyfin.org/posts/3017/audiobook-support-in-android-tv-app
- https://features.jellyfin.org/posts/3980/episode-limit-sleep-timer-essential-for-audhd-anxiety
- https://features.jellyfin.org/posts/896/combine-music-and-music-videos-libraries
- https://features.jellyfin.org/posts/2505/additonal-views-for-music-videos-library
- https://features.jellyfin.org/posts/3782/match-music-video-to-music-library
- https://features.jellyfin.org/posts/3370/photo-facial-recognition
- https://features.jellyfin.org/posts/796/automatic-photo-tagging-a-la-google-photos
- https://features.jellyfin.org/posts/1220/read-and-search-by-photo-tags
- https://features.jellyfin.org/posts/927/digital-photo-frame
- https://features.jellyfin.org/posts/1158/richer-comic-reader
- https://features.jellyfin.org/posts/3030/seamless-vertical-scrolling-for-comics-drawn-in-webtoon-format
- https://features.jellyfin.org/posts/2481/comic-manga-comicinfo-specific-metadata-server-plugin-side
- https://jellywatch.app/blog/jellyfin-audiobooks-podcasts-setup-guide-2026
- https://www.xda-developers.com/jellyfin-finally-convinced-me-to-ditch-three-separate-media-apps/
- https://github.com/EmbySupport/Emby.Docs/blob/master/Audio-Book-Naming.md
- https://github.com/EmbySupport/Emby.Docs/blob/master/Book-Naming.md
- https://github.com/EmbySupport/Emby.Docs/blob/master/Camera-Upload.md
- https://github.com/EmbySupport/Emby.Docs/blob/master/Library-Setup.md
- https://emby.media/premiere.html
- https://github.com/immich-app/immich
- https://github.com/immich-app/immich/releases/tag/v3.0.0
- https://github.com/immich-app/immich/issues/12614
- https://github.com/immich-app/immich/issues/16549
- https://github.com/immich-app/immich/issues/22385
- https://github.com/immich-app/immich/issues/13029
- https://github.com/immich-app/immich/issues/10538
- https://github.com/immich-app/immich/issues/15220
- https://immich.app/roadmap
- https://docs.immich.app/install/requirements
- https://docs.immich.app/overview/introduction
- https://github.com/Kareadita/Kavita/releases
- https://github.com/Kareadita/Kavita/releases/tag/v0.9.0.2
- https://wiki.kavitareader.com/kavita+/
- https://wiki.kavitareader.com/guides/features/opds/
- https://github.com/gotson/komga/releases
- https://komga.org/docs/introduction
- https://komga.org/docs/guides/kobo
- https://komga.org/docs/guides/koreader
- https://komga.org/docs/guides/opds
- https://github.com/booklore-app/booklore
- https://storyteller-platform.dev/
- https://support.pocketcasts.com/knowledge-base/playback-effects/
- https://support.pocketcasts.com/knowledge-base/sleep-timer/
- https://support.pocketcasts.com/knowledge-base/chapters/
- https://support.pocketcasts.com/knowledge-base/bookmarks/
- https://support.pocketcasts.com/knowledge-base/folders/
- https://support.pocketcasts.com/knowledge-base/files/
- https://automattic.design/2025/08/15/smarter-more-open-podcasting-with-pocket-casts/
- https://github.com/Podcastindex-org/podcast-namespace
- https://openpodcastapi.org/
- https://opensubsonic.netlify.app/docs/endpoints/
- https://github.com/koreader/koreader-sync-server
- https://readium.org/architecture/models/locators/
- https://developer.android.com/media/platform/motion-photo-format
- https://id3.org/id3v2-chapters-1.0
