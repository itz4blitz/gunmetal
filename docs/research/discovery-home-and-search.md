# Discovery, home screen and search

## Scope

This file covers how people find something to watch or listen to on a media
server: the home screen and its rows, continue watching and continue
listening, next up, recently added, watchlists, recommendations and how they
are computed, search, browse/filter/sort, collections and playlists used as
discovery, kids mode and per-user homes, seasonal or editorial rows, and the
conventions of a good ten-foot (TV) interface. Playback itself, the music
player's queue and the details of sign-in are covered by sibling files and
appear here only where they touch discovery.

Rivals studied: Plex (including the 2025 "new experience" redesign and the
reaction to it), Jellyfin (web, the 12.0 release, official and third-party
clients such as Swiftfin, Streamyfin, Wholphin and Finamp, and the plugins
people install to fill gaps), Emby, Infuse, Netflix, the Apple TV app, Kodi
skins, and for music Plexamp, Symfonium, Finamp and Navidrome.

Research notes, as of 2026-10-02:

- Web search and page fetching were available. The shared web-search budget
  ran out part-way through, so later facts were gathered by fetching known
  pages and the public JSON endpoints of the Plex forum, the Infuse forum and
  the Jellyfin feature board.
- Several primary sources refused automated access: support.plex.tv, the Plex
  blog, the Netflix tech blog, Apple's design guidelines and the Kodi wiki.
  Claims that would normally come from those pages are marked "(unverified)"
  unless a second source confirmed them.
- Vote counts below are what each board showed on the day of research. They
  change daily.

What the architecture records already decide, and why it matters here:

- The library is synced to the device, so browsing is instant and works
  offline (README; ADR 1). Home rows, search and filters can run on the
  device instead of asking the server for every screen.
- Watch history is the only irreplaceable data and lives in an append-only,
  exportable log; SQLite is a rebuildable cache (ADR 1, decision 5). Continue
  watching, next up and history are all derived from that log.
- The native API uses per-object authorisation (ADR 1, decision 6), which is
  what kids mode and per-user libraries must rest on.
- There is no central account (ADR 1, decision 7), so there is no vendor
  cloud to compute recommendations or a universal watchlist.
- Metadata lookups and scrobbling reach third parties, so they belong in
  plugins with explicit network grants (ADR 2, consequences).
- The interface is dark-first and music-forward, with its own identity; it
  must not copy another product's trade dress (ADR 2, decision 7).

## Feature inventory

Legend for the rival columns: **Yes** means built in; **Partial** means built
in with real limits; **Plugin** means only through a community plugin or
external tool; **No** means not available; **Pass** means Plex Pass required.
"(unverified)" marks anything not confirmed from a source.

### Home screen structure and customisation

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Row-based home of "hubs" | A home made of horizontal rows (continue watching, recently added and so on) | Yes. Since the new experience, rows come from libraries the user marks as favourites, plus Plex's own Discover, Live and On Demand rows | Yes. Fixed section slots: My Media, Continue Watching, Next Up, Latest Media, Live TV | Yes. Per-user sections; the legacy editor allowed up to 7 section types in 4.8/4.9 | Netflix for polish and relevance ordering; Kodi skins for raw flexibility (unverified) | The row is the universal pattern. The disagreements are about who controls the rows and what is allowed in them |
| User chooses and orders rows | Each person decides which rows appear and in what order | Partial. Users pin or favourite libraries; hub order inside a library is set by the server admin | Partial. Users choose from a fixed menu of section types per slot | Yes. Per-user section list with sort, image type and item limit in the newer editor (documented 2026) | Emby, for a native per-user editor | Plex removed manual home management in April 2021, which drew a 36-vote request to restore it |
| Admin default layout for new users | The server owner sets a sensible home for everyone | Partial. Admin hub order and published collections apply to all users | Plugin (Streamyfin plugin, Home Screen Sections) | Plugin (Home Screen Companion) | No one does this cleanly in core | Matters most for families, where one person maintains the server |
| Copy one user's layout to others | Set up once, apply to the whole household | No (unverified) | No | Plugin (Home Screen Companion mirrors a layout to many users) | Emby plus plugin | Emby users report setting up each account by hand without the plugin |
| Exclude a library from home rows | Keep a library (home videos, adult, kids) off the home screen | Yes, by not favouriting it | Yes, per-user "latest media" exclusions (unverified detail) | Yes (unverified detail) | Tie | |
| Hide platform or upsell rows | Home shows only my media | Partial. Discover, On Demand and Live can be disabled, but on Roku in 2025 doing so with no favourite libraries left a blank home | Not applicable (no upsell content) | Not applicable | Jellyfin and Emby, by not having any | The most-voted Plex discovery complaint (1,000 votes) |
| Custom rule-based rows | "Unwatched 4K comedies", "Short films under 90 minutes" as home rows | Partial. Smart collections published to home, Pass | Plugin (Home Screen Sections, KefinTweaks); a "smart filter rows" request has 12 votes | No native filtered sections as of May 2025; plugin adds tag and collection rows | Plex, for smart collections that update themselves | Users want this in core; every server forces a detour through collections or plugins |
| Collection or playlist as a home row | Pin "Marvel in release order" to home | Yes, "Visible on Home" for collections, Pass | Plugin (collection-sections plugin, Home Screen Sections) | Yes, via sections (unverified detail); plugin automates it | Plex, though paywalled | |
| Items-per-row limit | Rows long enough to be useful on a TV | Fixed (unverified) | Fixed; a request to raise "recently added" counts has 28 votes | Configurable limit in newer editor | Emby | Infuse users have asked since 2020 for 25/50/100/200 options |
| Artwork shape per row | Choose posters, landscape thumbs or banners per row | Partial (unverified) | Partial; Home Screen Sections switches to landscape cards | Yes (Auto, Primary, Thumb) | Emby | Wholphin (Jellyfin Android TV client) lets users pick image type per row |
| "See all" from a row | Jump from a row into the full filtered list | Yes | Yes | Yes | Tie | Infuse added "See All" for favourites in 8.4.3 |
| Hide watched items from rows | Recommendation rows stop showing what I have seen | No; 38-vote request since 2019 (status in 2026 unverified) | Partial; "hide watched from latest media" option (unverified) | Partial (unverified) | Netflix, which simply does not re-recommend watched titles (unverified) | |
| Home layout follows the user to every device | Same home on phone, TV and web | Partial; pinned libraries vanished on several TV clients after a July 2026 update | Partial; settings are stored per client app, so the web and Android TV homes differ (unverified) | Partial; the docs describe per-device options | Netflix and Apple TV, where the profile carries the layout | Infuse users report layouts not syncing across devices (2025) |
| Per-device override | A denser layout on the TV than on the phone | No (unverified) | Implicitly, because each client stores its own | Yes, per-device options in the editor | Emby | |
| Empty-state guidance | A new user sees how to fill the home | Weak; in April 2025 an iOS user found the home blank until they favourited libraries via an undocumented long-press | Plain "My Media" row always present | Similar to Jellyfin (unverified) | Jellyfin, by default content always showing | Plex staff called the missing onboarding a likely bug |
| Spotlight or hero banner | A large featured title at the top | Partial; hero on Discover pages (unverified for libraries) | Plugin; requests for a featured header (13 votes) and random spotlight (9 votes) | Coming: a spotlight section is tied to Emby 4.10 | Netflix | Infuse users ask for a hero section (2025) |
| Choose the start screen | Open straight into Music or Movies instead of Home | Partial (unverified) | Request (1 vote web; Android TV request 2 votes) | Unverified | Infuse, which can hide the Library icon and pin favourites | Small, but loved by people who only use one library |
| Layout stability across updates | My home does not change unless I change it | Poor in 2025–2026: mobile, Roku, Fire TV and TV redesigns, then a partial reversal | Good in core; plugins can break on upgrade | Good | Jellyfin and Emby | Plex's churn is the strongest lesson in this domain |

### Continue watching and next up

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Continue Watching row | Resume anything half-watched | Yes, across libraries and servers | Yes | Yes | Tie | |
| Next Up row | The next unwatched episode of shows in progress | Merged into Continue Watching since 2021 | Yes, separate row | Yes, separate row | Apple TV app, which merges both into one row with context (unverified) | |
| Merge or split the two rows | One "what's next" row, or two, as the user prefers | Merged, no toggle (users asked for one in 2021) | Split; a request to merge has 203 votes | Split (unverified) | Wholphin (Jellyfin client) offers an option to combine them | A toggle satisfies both camps |
| Dismiss an item | Remove something I abandoned | Yes, via long-press menu; works inconsistently on some clients | No; 1,725-vote request, "planned" since 2020 | Yes; Infuse 8.3.2 added removal for Emby and Plex, not Jellyfin | Plex | Jellyfin also lacks an API for it (a separate request asks for one) |
| Undo a dismissal | Get back something removed by mistake | No documented way to restore | Not applicable | Unverified | No one | Plex users report accidental removals and ask for confirmation |
| Stop tracking a show in Next Up | Never suggest the next episode of a show I quit | Same as dismiss | No; a 2026 request was declined | Unverified | Plex | |
| Abandoned-show cutoff | Shows untouched for N days drop off Next Up | Unverified | Yes, "max days in Next Up" setting | Unverified | Jellyfin | Jellyfin users note the cutoff ignores gaps between seasons |
| Rewatching | Next Up works when I restart a finished show | Unverified | Yes, completed request | Unverified | Jellyfin | |
| New episode bumps the show | A new episode of a show I follow moves it to the front | Unverified | No; requests to sort Next Up by availability | Plugin ("continue watching bump" in Home Screen Companion) | Netflix and Apple TV app (unverified) | |
| Specials in Next Up | Specials placed where they aired | Unverified | Yes (completed request) | Unverified | Jellyfin | |
| Next Up across movie collections | "Watch the next Harry Potter film" | No (unverified) | No; 36-vote request | No (unverified) | No one | Natural fit for ordered collections |
| Rich progress cards | Progress bar, time remaining, episode label | Yes | Yes | Yes | Apple TV app (tvOS 26 cards add descriptions and progress) | |
| Resume thresholds | What counts as started and finished | Yes, server setting (unverified) | Yes, server settings (unverified) | Yes (unverified) | Tie | Credits detection makes "finished" more accurate; Infuse added skip-credits settings in 2026 |
| Bulk mark watched or unwatched | Fix history after importing a library | Yes (unverified) | Yes (unverified) | Yes (unverified) | Infuse (multi-select added 8.4.3) | |
| Continue across sources | One resume row for several servers | Yes | No (single server per client) | No | Infuse and Plex | Jellyfin "multiple servers support" request: 177 votes |
| Play next episode from a "latest" card | Selecting a new show plays where I am, not episode 1 | Unverified | No; 11-vote request | Unverified | Netflix (unverified) | |

### Recently added and what's new

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Recently added per library | See what arrived | Yes | Yes | Yes | Tie | |
| Grouped episodes | Ten new episodes of one show appear as one card | Yes (unverified) | Yes, grouped by show | Yes (unverified) | Tie | |
| Card opens the series | Selecting a grouped show opens the show page | Unverified | No; 126-vote request (July 2026) | Unverified | Unverified | One of the fastest-growing Jellyfin requests of 2026 |
| Upgrades are not "new" | Replacing a 1080p file with 4K does not re-announce it | Unverified | No; 62-vote request | Unverified | No one confirmed | Infuse users report duplicates in "Recently Added" from pinned Plex collections |
| Remove one item from recently added | Hide a mistaken import | Unverified | No; 10-vote request | Unverified | No one confirmed | |
| Configurable count | Show 50 recent items, not 16 | Unverified | Partial; requests at 28 and 26 votes | Yes (limit option) | Emby | |
| Recently released vs recently added | Sort by air or release date, not file date | Partial; Plex has a "Recently Released" hub (unverified) | Request (3 votes) | Unverified | Plex (unverified) | |
| New-episode badge | "3 new" on a show card | Unverified | Unverified | Unverified | Netflix ("New episodes" labels) | Netflix 2025 redesign added at-a-glance labels such as recently added |

### Watchlist, favourites, history and ratings

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Watchlist for my library | Save things to watch later | Yes; Watchlist is a top-level tab in the 2025 mobile app | No; 1,294-vote request, planned | Partial via favourites (unverified) | Plex | KefinTweaks (682 stars) adds a watchlist to Jellyfin web |
| Watchlist for titles I don't have | Save a film I heard about even before it is on the server | Yes, universal watchlist across streaming services since 2022 | Plugin (Seerr requests) | Plugin | Plex | Needs an external metadata source |
| Shareable watchlist link | Send my list to a friend | Pass (unique watchlist URLs) | No | No | Plex | |
| Favourites | Heart a title, artist or library | Yes | Yes; 10.11 extended favourites to more types | Yes | Tie | Infuse added favourite sync for Emby and Jellyfin in 2025 |
| Watch history page | See what I watched and when | Yes, in the user menu | No; 830-vote request, planned | Unverified | Plex | ADR 1 makes history a first-class log for Gunmetal |
| Personal ratings | Rate titles and filter by my own score | Yes (stars) (unverified) | No; 453-vote request, planned | Unverified | Plex (unverified) | |
| Thumbs or "not interested" | Teach the recommender | No (unverified) | No | No | Netflix (thumbs feed recommendations) | |
| Trakt or external sync | Keep history in step with Trakt | Plugin or third-party tool (unverified) | Plugin | Plugin | Infuse (built in, with Trakt VIP display) | Third-party network calls; a plugin concern for Gunmetal |
| Export and import history | Leave without losing history | Partial (unverified) | Plugin | Unverified | KefinTweaks exports its watchlist to JSON | Gunmetal's append-only log is designed for this |

### Recommendations

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| "Because you watched" rows | Picks seeded by a recent title | Partial; library "Recommended" hubs (exact hub list unverified) | Partial; a Suggestions tab, which in 10.11.6 showed the same 12 alphabetical films for every seed (bug, January 2026) | Unverified | Netflix | Jellyfin home recommendations requests: 88, 85 and 42 votes |
| More like this on detail page | Similar titles on every item | Yes | Yes | Yes | Netflix | Jellyfin's similar items are said to work while the aggregate Suggestions page was broken |
| Configurable recommendation sources | Choose which provider drives suggestions per library | No | Yes, new in 12.0 (September 2026), with ListenBrainz for music | No (unverified) | Jellyfin 12.0 | |
| Genre rows on home | "Comedies", "Documentaries" rows | Yes (library recommended hubs, unverified) | Plugin | Yes, via genre sections (unverified) | Netflix | |
| Explain why | "Because you watched X", "Award winner", "Leaving soon" | Partial (row titles only) | Partial (row titles only) | Partial | Netflix 2025 labels on every tile | |
| In-session responsiveness | Picks change as I browse in this session | No | No | No | Netflix (2025: reacts to searches, trailer views and thumbs during the session) | |
| Exclude a library from suggestions | Home videos never recommended | Unverified | No; 12-vote request | Unverified | Unverified | |
| Rediscover / watch again | Old favourites surfaced again | Yes, "Rediscover" hub | Plugin (Home Screen Sections "Watch Again") | Unverified | Plex | |
| Random pick | "Surprise me" | Unverified | Plugin; random spotlight request 9 votes | Unverified | Kodi skins (random widgets, unverified) | Infuse users asked for randomised home lists in 2020 |
| Popular on this server | What the household is watching | Unverified | No | No | Netflix Top 10 (global, not household) | Plex request to recommend server content to friends: 253 votes |
| Streaming-service recommendations | What to watch elsewhere | Yes, Discover | No | No | Plex, for those who want it | Many self-hosters actively do not want this |
| Social discovery | Friends' activity, reviews, discussion threads | Yes; public profiles and reviews (January 2025) and Discussions | No | No | Plex, if wanted | Android Authority criticised Discussions for clutter, public-by-default posts and spoilers |
| Personalised artwork | Different art per person | No | No | No | Netflix (unverified) | Out of scope for a personal server |
| How it is computed | The signal behind the picks | Plex metadata similarity and Plex's cloud for Discover (unverified) | Overlap of genres, tags, studios and people (unverified); provider-based from 12.0 | Unverified | Netflix: viewing history, ratings, similar members, title metadata, time of day, device, language; not age or gender | Netflix's own help page lists these signals |

### Search

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| One search across everything | Films, shows, episodes, people, music in one box | Yes; relevance-ranked since September 2023, with a source picker | Yes | Yes | Plex | |
| Speed on large libraries | Results in well under a second | Good (unverified) | Historically slow; "greatly improved" in 10.11 (October 2025), with regressions reported in release candidates | Unverified | Plex (unverified) | Jellyfin web fired one query per content type in sequence |
| Typo tolerance | "Shawshank Redemtion" still works | Unverified | No; 46-vote request since 2021 | Unverified | Netflix (unverified) | |
| Accents, transliteration, punctuation | "Amelie" finds "Amélie"; curly and straight apostrophes match | Unverified | Partial; 10.11.0 broke diacritics search; requests open for transliteration and apostrophes | Unverified | Infuse (added Simplified/Traditional Chinese transliteration, December 2025) | |
| Search by person | Actor, director, composer | Yes | Yes | Yes | Tie | Jellyfin asks for a searchable "character" field (7 votes) |
| Search by genre or tag | "Christmas" finds the tag, not only titles | Unverified | No; 256-vote request since 2019 | Unverified | Plex (filters) | Tag search slowed Jellyfin 10.9 and got an off switch |
| Scope to one library or type | Search only Music | Yes (filters on full search page) | Yes (completed 2025) | Unverified | Tie | |
| Grouped results with filters | Results split by type with chips | Yes | Partial; list-view request 14 votes | Unverified | Infuse 8.5 (organised category groups, July 2026) | |
| No arbitrary result cap | See all matches | Fixed in 2023 (was 30 per source) | Yes (unverified) | Unverified | Tie | |
| Search as you type with debounce | Results update while typing without lag | Yes | Yes (completed) | Yes (unverified) | Tie | |
| Keyboard shortcut on desktop | Ctrl+K opens search | No (unverified) | No; 3-vote request | Unverified | Unverified | Cheap and loved on web |
| Voice search on TV | Speak a title on the remote | Partial (platform dependent, unverified) | Partial; Google Assistant and Alexa request 243 votes | Unverified | Apple TV and Google TV system search (unverified) | |
| Natural-language or mood search | "Scary but funny" | No | No | No | Netflix (OpenAI-powered, opt-in beta, 2025) | |
| Results outside my library | Find a film I don't own and see where to get it | Yes (Discover, can be disabled) | Plugin (Seerr, via Streamyfin, KefinTweaks) | Plugin | Plex, for those who want it | Infuse shows a TMDB details page for deep-linked titles not in the library |
| Pluggable search providers | Plugins add search sources | No | Yes, new in 12.0 | Unverified | Jellyfin 12.0 | Community search accelerators exist: Jellysearch, Meilisearch plugin |
| Disambiguate duplicates | Same title in two libraries shown clearly | Partial (merged results) | No; open request (2 votes) | Unverified | Plex | |
| Search that works offline | Find something on a plane | No | No | No | No one among servers | Gunmetal's synced library makes this possible |

### Browse, filter and sort

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Endless grid without pagination | Scroll a 10,000-film library smoothly | Yes (unverified) | No; 1,179-vote request, "started" | Unverified | Plex (unverified) | Apple TV app beta capped personal library categories at 75 items |
| Alphabet jump | Jump to "S" | Yes (unverified) | Yes | Yes (unverified) | Infuse (alphabet scrollbar, 8.1) | |
| Rich filters | Genre, year, rating, resolution, HDR, codec, unwatched | Yes (unverified detail) | Partial; "friendlier filters" request 137 votes | Unverified | Plex (unverified) | |
| Filter by audio or subtitle language | "Films with a French track" | Unverified | No; 172-vote request since 2019 | Unverified | Unverified | Gunmetal's parser knows every track at scan time |
| Remember filter state | The library opens with my last filter | Unverified | Partial; 73-vote request, started | Unverified | Unverified | |
| Save a filter as a smart collection | One click from filter to living collection | Yes | No | Unverified | Plex | |
| Multi-key sort | Sort by year, then title | Unverified | No; 12-vote request | Unverified | Unverified | |
| Useful sort keys | Date added, release, last episode added, unwatched count | Yes (unverified detail) | Partial; "sort by last episode added" completed; unwatched-count sort requested | Unverified | Unverified | |
| Folder view | Browse by disk folder | No (unverified) | Requested; 438 votes and 134 comments for a folder view in libraries (any existing partial support unverified) | Yes (unverified) | Kodi (file view, unverified) | Popular with people who curate by folder |
| Show grouped by collection | Box sets collapse into one tile | Yes (unverified) | Yes, 10.11 | Yes (unverified) | Tie | |
| Flatten single-season shows | Skip a pointless season screen | Unverified | No; 172-vote request | Unverified | Unverified | |
| Multi-select actions | Mark, add to playlist or collection in bulk | Yes (unverified) | Partial (unverified) | Unverified | Infuse (select and select-all, 8.3.6) | |
| Genre, studio and network pages | Browse by genre | Yes | Partial; genre page for all types requested | Yes (unverified) | Plex (unverified) | Wholphin adds genre and studio browsing |
| List and grid toggle | Dense list for big libraries | Yes (unverified) | Yes | Yes (unverified) | Tie | |

### Collections and playlists as discovery

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Manual collections | Group titles by hand | Yes | Yes | Yes | Tie | |
| Automatic box sets | Franchises grouped from metadata | Yes (unverified) | Yes (unverified) | Yes (unverified) | Tie | Needs external metadata |
| Smart collections | Collections defined by rules that update themselves | Yes (server 1.22.3+) | No in core | No (unverified) | Plex | |
| Nested collections | "Star Wars" containing trilogies | No; 514 votes, 13,407 views | No | Unverified | No one | |
| Custom order inside a collection | Machete order, chronological order | Yes (implemented request) | No; collection sort request 148 votes | Unverified | Plex | |
| Smart playlists for video | A rule-based queue | Yes | No; 588-vote request, planned | Unverified | Plex | |
| Collections on detail pages | "Part of: The Godfather" | Yes (unverified) | Yes, 12.0 | Unverified | Tie | |
| Publish a collection to everyone's home | Server owner curates a row for all users | Pass ("Visible On" Home, Library, Shared Users' Home) | Plugin | Plugin | Plex, though paywalled | Kometa and Agregarr automate this for Plex and also need Pass |
| Import external lists | Trakt or MDBList lists as collections | External tool (Kometa) | Plugin | Plugin (Home Screen Companion) | Kometa, for breadth | Third-party network access |
| Hide the Collections view | Keep collections inside a library only | Unverified | No; 18-vote request | Unverified | Unverified | |

### Profiles, per-user homes and kids mode

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Separate users with their own home | My history and rows are mine | Yes (Plex Home and friends) | Yes | Yes | Netflix (profiles are instant and cheap) | |
| Fast switching with PIN | Switch person on the couch | Yes (Plex Home PINs) | Partial; multiple-account switching request 154 votes, started | Yes (unverified) | Netflix and tvOS 26 (profile picker at wake) | Plex users asked to hide PIN entry from others in the room |
| Profile picker at launch | The TV asks who is watching | Partial (unverified) | Client dependent | Unverified | Apple TV (tvOS 26 option to show profiles on wake) | |
| Rating-based restriction presets | Pick "Younger kid" and done | Yes: Younger Kid, Older Kid, Teen | Partial; max rating per user | Yes, ratings and blocked libraries | Plex | |
| Tag or label restrictions | Block a label such as "horror" | Pass (advanced restrictions) | Yes, allowed and blocked tags (unverified) | Yes (unverified) | Jellyfin, for being free | |
| Per-title exceptions | Allow one PG-13 film for a 10-year-old | Via labels, Pass | No; requests at 12 and 7 votes | Unverified | Netflix (block individual titles) | |
| Viewing schedules | Kids profile only works 7am–8pm | No (unverified) | Yes, access schedule (unverified) | Yes, watch schedules | Emby | |
| Dedicated kids interface | Big, simple, bright, no grown-up rows | No (unverified) | No; "child mode" request 77 votes | No (unverified) | Netflix Kids | |
| Lock the parent profile | Kids cannot hop into the adult profile | Yes, via PIN | No; 62-vote request | Yes (unverified) | Netflix (Profile Lock PIN) | |
| Restricted users still get discovery | Kids get recommendations too | Discover was missing for managed users; request thread has 104 posts | Yes (same rows) | Yes (unverified) | Netflix | |
| Adult artwork kept off shared surfaces | Screensavers and home never show explicit art | Unverified | Yes for home (completed); screensaver rating filter declined | Unverified | Unverified | |
| Hidden libraries per user | Each user sees only granted libraries | Yes | Yes | Yes | Tie | Gunmetal: must also hold for the synced device cache |

### Seasonal, editorial and curated rows

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Date-scheduled rows | Christmas films appear in December, then vanish | External tool (Kometa schedule ranges) | Plugin (KefinTweaks seasonal sections) | Plugin (Home Screen Companion scheduling) | Netflix editorial rows (unverified) | No server does it in core |
| Admin-curated row for all users | "Movie night picks" chosen by the server owner | Pass | Plugin | Plugin | Plex | |
| Anniversaries | "On this day" albums or films | Plexamp ("On This Day" albums) | No | No | Plexamp | Works well for music; cheap to compute locally |
| Ranked list rows | "Top 10" style numbered posters | No (unverified) | No | Plugin ("Top-List" sections) | Netflix Top 10 | Must not copy Netflix's trade dress |
| Upcoming releases | Episodes airing soon for shows I follow | Partial, Discover (unverified) | Plugin (Home Screen Sections "Upcoming" via *arr apps) | Unverified | Unverified | Requires external schedules |
| Requests as discovery | Ask the server owner for a missing title from the app | Partial (watchlist used by request tools, unverified) | Plugin (Seerr in Streamyfin, Wholphin, Home Screen Sections) | Plugin | Streamyfin and Wholphin, for in-app requests | |
| Admin-defined "featured" banner | The owner picks this week's spotlight | Unverified | Request (13 votes) | Spotlight tied to 4.10 | Unverified | |

### The TV (ten-foot) interface

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Left rail navigation | Press left to reach libraries and search | Removed in 2025, restored in an August 2026 preview on Apple TV, Roku and Android TV; Fire TV later | Yes (Android TV app) | Yes (unverified) | Plex's own reversal is the evidence | Netflix moved to a top bar in 2025 and did not reverse (as of research) |
| Few clicks to my library | Library reachable in one or two presses | Fire TV 2026 redesign turned roughly 2 clicks into roughly 6 | Good | Unverified | Plex classic and the 2026 restoration | |
| Focus memory | Back returns to the same row and column | Unverified | Unverified | Unverified | Netflix (unverified) | Hard to get right in React Native on TV |
| Expanding focused tile | Focusing a tile reveals description and trailer | No | No | No | Netflix (2025 redesign) | Nice but costly on weak TV hardware |
| Backdrop follows focus | Full-screen art of the focused title | Yes (unverified) | Yes (unverified) | Yes (unverified) | Kodi skins and Infuse (unverified) | Infuse users asked for blurred fanart behind home rows |
| Autoplay previews with an off switch | Trailers on focus, or silence | Partial (cinema trailers before films, Pass) | No | No | Netflix (has a toggle) | |
| Long-press context menu | Mark watched, remove, add to list without opening the item | Yes | Partial | Unverified | Infuse (context menus for Emby, Jellyfin, Plex, 2025) | |
| Remote-friendly alphabet jump | Letter picker on TV | Yes (unverified) | Yes (unverified) | Unverified | Infuse | |
| Smooth scrolling with large posters | No lag on a cheap stick | Fire TV 2026 users reported lag and 30-second local starts | Varies by client | Unverified | Apple TV app (unverified) | Hardware ceiling matters: Gunmetal targets cheap devices |
| Poster orientation choice | Portrait posters or landscape thumbs | Unverified | Yes on Roku 3.1.9 and Wholphin | Yes | Wholphin | tvOS 26 switched the TV app to portrait to fit more per row |
| Screensaver from library art | Ambient art when idle, OLED-safe | Yes (unverified) | Yes; OLED-friendly screensaver completed | Unverified | Apple TV aerials (system) | |
| UI scale and text size | Readable from the sofa | Unverified | No; 10-vote request for scaling | Yes, font size options | Emby | |

### Operating-system surfaces

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Android TV "Play Next" row | My continue watching on the TV's own home screen | Unverified | Yes (completed request) | Unverified | Netflix and other big apps (unverified) | Android supports four types: continue, next, new and watchlist |
| Android TV home channels | App rows on the launcher | Unverified | Yes (completed) | Unverified | Unverified | |
| Apple TV Top Shelf and TV app Up Next | Resume from the Apple TV home | Unverified | Unverified | Unverified | Infuse (users ask to keep Up Next on the Apple TV home only) | Top Shelf details unverified (Apple docs not readable) |
| Deep links into titles | Open a film straight from another app or the system | Unverified | Unverified | Unverified | Infuse (TMDB ID deep links, 2025) | |
| Phone home-screen widgets | Resume or a mix from the phone launcher | Plexamp widget on Android (implemented) | Client dependent | Unverified | Symfonium (many resizable widgets) | Music matters most here |

### Music discovery and continue listening

Here the relevant rivals are music apps, not the three video servers.

| Feature | What the user gets | Plexamp | Jellyfin with Finamp or Symfonium | Navidrome with Subsonic clients | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Recently played and continue listening | Return to the album or playlist I was in | Yes | Partial; "last played audio on home" request (4 votes), per-user recently played request (2 votes) | Yes, album lists (unverified detail) | Spotify and Apple Music (unverified) | Low vote counts, but every streaming app leads with this |
| Resume long audio | Audiobooks and podcasts resume | Yes (view-offset resume) | Partial; "continue listening" section (unverified) | Unverified | Plexamp | Jellyfin requests autoplay of the next audiobook chapter (26 votes) |
| Recently added albums | New arrivals | Yes | Yes | Yes | Tie | |
| Library radio and stations | Endless play seeded by artist, album, style or mood | Yes; style and mood stations, multi-artist mix builder | Partial (instant mix) (unverified detail) | Partial (client dependent, unverified) | Plexamp | |
| Sonic similarity | Tracks that sound alike, not just tagged alike | Pass (neural analysis of the library) | No; 98-vote request | No | Plexamp | Heavy compute; not part of Gunmetal's first release |
| Personal mixes | Mixes built from my heavy rotation | Pass (mix builders) | Symfonium (adaptive mixes) | Unverified | Plexamp, Symfonium | |
| Natural-language playlists | "Upbeat 90s indie for a run" | Pass (Sonic Sage) | No | No | Plexamp | Needs an AI provider; a plugin question for Gunmetal |
| Smart playlists | Rule-based music playlists | Yes | Symfonium yes; Jellyfin core no (588-vote request covers all media) | Yes | Navidrome (core, free) and Symfonium | |
| "On this day" | Albums released on this date | Yes | No | No | Plexamp | |
| Customisable music home | Choose the music home's sections | Yes ("All Music on home" option and more) | Symfonium (widgets) | Client dependent | Symfonium | |
| Listening-history recommendations | Picks from ListenBrainz or Last.fm | Unverified | Yes, ListenBrainz provider in 12.0 (28-vote request preceded it) | Plugin or client (unverified) | Jellyfin 12.0 for open services | Third-party network calls: plugin territory for Gunmetal |
| Per-user play counts, ratings, favourites | My stats, not the household's | Yes | Yes | Yes | Tie | |

### Patterns from streaming apps and players

| Feature | What the user gets | Netflix | Apple TV app | Infuse | Kodi skins | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|---|
| Featured hero card | One big title at the top | Yes; 2025 redesign isolates it in a large card | Yes (unverified) | No; users asked for one in 2025 | Yes, spotlight widgets (unverified) | Netflix | |
| Expanding focused tile | Info and trailer in place | Yes | No (unverified) | No | Skin dependent (unverified) | Netflix | |
| "Why watch" labels | Recently added, Top 10, awards, leaving soon | Yes | Partial (badges for new, time left) | No | No (unverified) | Netflix | |
| Responsive in-session picks | Picks change while browsing | Yes | No (unverified) | No | No | Netflix | |
| Natural-language search | Search by mood | Opt-in beta | No (unverified) | No | No | Netflix | |
| Profiles at wake | The device asks who is watching | Profile gate on launch | Yes, tvOS 26 | Plex Home managed profiles (January 2026) | Profiles (unverified) | Apple TV | |
| Kids interface | A separate, simpler experience | Yes, Kids profiles | Partial (Screen Time restrictions, unverified) | No | No (unverified) | Netflix | |
| One Up Next across sources | Resume from every app or server in one row | No (single service) | Yes, across participating apps (unverified) | Yes, across shares and servers | Partial (unverified) | Apple TV app | |
| Home built from arbitrary widgets | Any list, filter or add-on as a row | No | No | Partial (favourites, pinned folders and collections) | Yes; widgets from library nodes, smart playlists and add-ons such as TMDb Helper | Kodi | Arctic Fuse 2 was archived in January 2026 in favour of Arctic Fuse 3 |
| Pinned favourites with custom art | Big tiles for "Kids films", "4K" | No | No | Yes (8.4–8.5: pin, reorder, custom artwork, hide individual favourites) | Yes (unverified) | Infuse | Shipped steadily through 2026 |
| Navigation placement | Where the menu lives | Top bar since 2025 | Top tabs (unverified) | Tabs (Library tab renamed Search in December 2025) | Usually a left or bottom menu (unverified) | Disputed; Plex's users rejected a top bar on TV | |
| Portrait posters for density | More titles per row | Mixed | Yes, tvOS 26 | Poster favourites added 8.4.1 | Configurable | Apple TV | |
| Vertical clip feed on mobile | Swipe through short clips to find something | Tested on mobile in 2025 | No | No | No | Netflix (as an experiment) | Needs clips; not a fit for a personal library at first |

## Pain points and unmet demand

Each item gives evidence: a link and, where the source shows them, votes,
replies or views.

1. **Personal media pushed aside by the vendor's own content (Plex).** The
   request to fully disable Discover and streaming-service search results
   gathered 1,000 votes, 1,105 posts, 2,207 likes and 29,390 views before it
   was marked implemented
   ([forum](https://forums.plex.tv/t/implemented-fully-disable-discover-search-results-from-streaming-services-more-ways-to-watch/786259)).
   After the 2025 Roku redesign, users again complained that Live, Discover,
   On Demand and Rentals rows could not be removed and that continue
   watching was split across libraries
   ([forum, September 2025](https://forums.plex.tv/t/new-ui-is-an-awful-experience/931048)).
   The complaint is old: a 2021 thread shows a five-year user threatening to
   leave over pushed live channels
   ([forum](https://forums.plex.tv/t/stop-messing-with-my-home-screen/716342)).
   Plex's 2025–2026 social features (public profiles, reviews, Discussions)
   were criticised for cluttering detail pages and being public by default
   ([Android Authority](https://www.androidauthority.com/plex-discussions-hands-on-3677364/)).

2. **Redesign churn on TVs (Plex).** The Roku new experience (v8.6.4,
   September 2025) moved the sidebar to a top bar and was called "hot
   garbage" by users, per PiunikaWeb
   ([article](https://piunikaweb.com/2025/09/17/plex-roku-update-backlash/)).
   Plex said on 18 September 2025 that it was not planning to roll back
   ([forum](https://forums.plex.tv/t/roku-new-experience-release-update/931239));
   that thread was listed with 724 replies and 34,388 views. A "Vote to roll
   back to Plex Classic" thread reached 504 votes, 317 posts, 1,230 likes and
   10,076 views
   ([forum](https://forums.plex.tv/t/vote-to-roll-back-to-plex-classic/931767)).
   The Fire TV version (March 2026) was called unusable: roughly two clicks
   became roughly six, posters lagged, and local playback took 30 seconds to
   start
   ([PiunikaWeb](https://piunikaweb.com/2026/05/04/plex-fire-tv-redesign-unusable-users-slam-navigation/)).
   In August 2026 Plex began restoring the left navigation in a preview for
   Apple TV, Roku and Android TV
   ([XDA](https://www.xda-developers.com/after-months-backlash-plex-rolling-back-controversial-big-screen-app-redesign/),
   [How-To Geek](https://www.howtogeek.com/plex-brings-back-sidebar-navigation-after-users-refuse-to-let-it-go/)).
   A month earlier, pinned libraries vanished for more than ten users of one
   server after an update
   ([forum, July 2026](https://forums.plex.tv/t/recent-update-broke-more-option-pinned-libraries-gone-on-multiple-clients-google-tv-roku/940434)).
   On mobile, the March 2025 app dropped sidebar pinning
   ([PCWorld](https://www.pcworld.com/article/2653520/plexs-new-experience-arrives-6-things-to-know-before-updating.html))
   and left some users with a blank home until they found an undocumented
   long-press
   ([forum, April 2025](https://forums.plex.tv/t/how-to-customize-the-new-home-screen-in-the-app-it-s-horrible/915559)).

3. **Cannot dismiss or undo in Continue Watching.** Jellyfin's request to
   remove an item from Continue Watching has 1,725 votes and 51 comments and
   has been "planned" since March 2020
   ([features](https://features.jellyfin.org/posts/517)). A 2026 request
   asks for a user-level API to manage Continue Watching and Next Up without
   playing anything ([features](https://features.jellyfin.org/posts/4149)),
   and Infuse 8.3.2 could add removal only for Emby and Plex
   ([release notes](https://firecore.com/releases)). Plex supports removal
   but has no documented undo
   ([forum](https://forums.plex.tv/t/how-do-i-revert-removing-something-from-continue-watching/768314)),
   and users have asked for a confirmation step after accidental removals
   ([forum](https://forums.plex.tv/t/remove-from-continue-watching-problems/716534)).

4. **No native watchlist or history in Jellyfin.** Watchlist: 1,294 votes,
   planned ([features](https://features.jellyfin.org/posts/576)). Watched
   history: 830 votes, planned
   ([features](https://features.jellyfin.org/posts/633)). Personal ratings:
   453 votes ([features](https://features.jellyfin.org/posts/1134)). The gap
   is filled by client-side script injection: KefinTweaks (682 stars) adds a
   watchlist, history and statistics pages
   ([GitHub](https://github.com/ranaldsgift/KefinTweaks)).

5. **Next Up and Continue Watching are confusing.** Jellyfin's request to
   combine them has 203 votes and 32 comments
   ([features](https://features.jellyfin.org/posts/1055)); others ask to
   suggest the next film in a collection (36 votes,
   [features](https://features.jellyfin.org/posts/1606)), to sort by when an
   episode became available
   ([features](https://features.jellyfin.org/posts/3158)), and to hide Next
   Up when no TV has been watched (10 votes,
   [features](https://features.jellyfin.org/posts/1665)). Plex went the other
   way and merged On Deck into Continue Watching in 2021; users asked for a
   toggle (36 votes,
   [forum](https://forums.plex.tv/t/give-users-more-control-over-their-hubs-compensate-the-removal-of-the-manually-managed-home-screen/711411)).

6. **Home customisation lives in plugins, paywalls or nowhere.** Jellyfin:
   "New or improved home screen sections" 169 votes, planned
   ([features](https://features.jellyfin.org/posts/1986)); custom carousels
   81 votes ([features](https://features.jellyfin.org/posts/1439)); smart
   filter rows 12 votes
   ([features](https://features.jellyfin.org/posts/3761)). The Home Screen
   Sections plugin (519 stars) is web-only and depends on two other plugins
   ([GitHub](https://github.com/IAmParadox27/jellyfin-plugin-home-sections)).
   Emby: a request for filtered home sections dates from December 2020 and
   was still unresolved in May 2025
   ([forum](https://emby.media/community/topic/92899-customizable-sections-on-home-screen/));
   a third-party plugin exists to sync layouts across users and schedule rows
   ([GitHub](https://github.com/soderlund91/HomeScreenCompanion)). Plex:
   publishing a collection to home needs Plex Pass, and so do Kometa and
   Agregarr when they automate it
   ([Agregarr docs](https://agregarr.org/docs/ui/home/),
   [Kometa docs](https://kometa.wiki/en/latest/files/settings/)). Infuse:
   users have asked since 2020 for longer home rows (452 views, 15 likes,
   [forum](https://community.firecore.com/t/increase-number-of-items-in-home-screen-lists/23941)),
   for a refreshed home with a hero section
   ([forum, 2025](https://community.firecore.com/t/time-for-a-home-screen-ui-refresh/57276)),
   and for layouts that sync across devices
   ([forum](https://community.firecore.com/t/home-screen-layouts-library-lists-not-syncing-across-devices/55695))
   and per Plex Home user
   ([forum](https://community.firecore.com/t/retain-home-screen-customizations-when-switching-plex-home-users/59343)).

7. **Recommendations on self-hosted servers are thin or broken.** Jellyfin
   requests: genres and recommendations on home 88 votes
   ([features](https://features.jellyfin.org/posts/3501)), a local
   recommendation engine 85 votes
   ([features](https://features.jellyfin.org/posts/2737)), home screen
   suggestions 42 votes
   ([features](https://features.jellyfin.org/posts/1369)). In January 2026
   the Suggestions page showed the same first 12 films, alphabetically, for
   every "Because you watched" seed
   ([issue 16088](https://github.com/jellyfin/jellyfin/issues/16088),
   duplicate of [14088](https://github.com/jellyfin/jellyfin/issues/14088)).
   Plex users have long asked to recommend server content to friends (253
   votes, 7,444 views,
   [forum](https://forums.plex.tv/t/recommend-server-content-not-just-queued/87784)).
   To be fair, big-budget recommendations do not please everyone either:
   commenters on Netflix's 2025 Apple TV redesign called the picks poor
   ([MacRumors](https://www.macrumors.com/2025/08/13/netflix-rolls-out-redesigned-interface-apple-tv/)).

8. **Search is slow or brittle on big libraries.** Jellyfin issues and
   threads describe very slow search
   ([issue 12624](https://github.com/jellyfin/jellyfin/issues/12624),
   [discussion 12444](https://github.com/orgs/jellyfin/discussions/12444)),
   a web client that ran one query per content type in sequence
   ([jellyfin-web 6793](https://github.com/jellyfin/jellyfin-web/issues/6793)),
   a 10.11 release candidate that made it slower
   ([issue 14704](https://github.com/jellyfin/jellyfin/issues/14704)) and
   broken accent matching in 10.11.0
   ([issue 15116](https://github.com/jellyfin/jellyfin/issues/15116)).
   Requests: search by tag or genre 256 votes
   ([features](https://features.jellyfin.org/posts/276)), typo tolerance 46
   votes ([features](https://features.jellyfin.org/posts/1141)), language
   audio filter 29 votes
   ([features](https://features.jellyfin.org/posts/3172)). Plex's 2023
   search rewrite removed a 30-results-per-source cap; its feedback thread
   was listed with 195 replies and 6,255 views
   ([forum](https://forums.plex.tv/t/new-search-release/853473)).

9. **Browsing large libraries is slow or capped.** Jellyfin "remove
   pagination, use lazy loading": 1,179 votes, started
   ([features](https://features.jellyfin.org/posts/216)). Filter by audio
   language: 172 votes ([features](https://features.jellyfin.org/posts/39)).
   Friendlier filters: 137 votes
   ([features](https://features.jellyfin.org/posts/462)). Remember filter
   state per library: 73 votes, started
   ([features](https://features.jellyfin.org/posts/143)). In the tvOS 26
   beta, the Apple TV app capped personal movie categories at 75 items
   ([AppleInsider](https://appleinsider.com/articles/25/06/18/tvos-26-hands-on-sleek-liquid-glass-redesign-new-control-center-and-more)).

10. **"Recently added" is noisy.** Jellyfin: open the series page from a
    recently added card, 126 votes since July 2026
    ([features](https://features.jellyfin.org/posts/4021)); do not show
    upgraded files as new, 62 votes
    ([features](https://features.jellyfin.org/posts/2329)); more items, 28
    votes ([features](https://features.jellyfin.org/posts/2373)).

11. **Collections stop short.** Plex nested collections: 514 votes, 13,407
    views ([forum](https://forums.plex.tv/t/nested-collections/354043)).
    Jellyfin collection sort options: 148 votes
    ([features](https://features.jellyfin.org/posts/1577)); smart playlists:
    588 votes, planned ([features](https://features.jellyfin.org/posts/49)).

12. **Kids mode is a set of filters, not an experience.** Jellyfin "child
    mode" 77 votes ([features](https://features.jellyfin.org/posts/531)),
    profile PIN so kids cannot use a parent profile 62 votes
    ([features](https://features.jellyfin.org/posts/2544)), finer parental
    control 25 votes ([features](https://features.jellyfin.org/posts/2580)),
    per-item exceptions 7 votes
    ([features](https://features.jellyfin.org/posts/4030)). Plex users asked
    for Discover to work for managed users (104 posts,
    [forum](https://forums.plex.tv/t/add-discover-to-managed-users/794138)).
    Profile switching in Jellyfin: 154 votes, started
    ([features](https://features.jellyfin.org/posts/2353)).

13. **The best music discovery is paywalled or absent.** Plexamp's sonic
    similarity, Sonic Sage, Guest DJ and mix builders require Plex Pass
    ([Plexamp](https://www.plex.tv/plexamp/)). Jellyfin's sonic analysis
    request has 98 votes
    ([features](https://features.jellyfin.org/posts/3245)); music home
    requests are small but unaddressed
    ([2092](https://features.jellyfin.org/posts/2092),
    [3735](https://features.jellyfin.org/posts/3735)).

14. **Paywalls on basic access.** Plex made remote streaming exclusive to
    Plex Pass and Remote Watch Pass from April 2025
    ([Wikipedia](https://en.wikipedia.org/wiki/Plex)); Plex's own FAQ lists
    Remote Watch Pass at $1.99 a month introductory until 1 June 2026, then
    $2.99 ([Plex Pass page](https://www.plex.tv/plex-pass/)). This is not a
    discovery feature, but it shapes how people judge every Plex paywall
    above.

Where rivals are already good, honestly: Plex's universal watchlist and
relevance-ranked search are polished for people who want streaming-service
content; Plexamp is the best self-hosted music discovery available; Netflix's
recommendations, labels and kids profiles set the bar for a living room;
Infuse has shipped a steady run of home-screen improvements through 2026;
Jellyfin 12.0 (September 2026) added pluggable search and per-library
recommendation providers; Emby already has per-user sections with sort,
artwork and limits plus viewing schedules; Kodi skins remain the most
flexible home screens anywhere.

## Where Gunmetal can be clearly better

Each idea names the pain point it answers, stays inside the architecture
records, and says what has to be true technically.

1. **A home that is yours by default.** (Pain points 1 and 2.) Gunmetal ships
   no vendor content, so the home is only the user's media. Anything from
   outside (trending lists, streaming availability, "upcoming") arrives
   through a plugin with an explicit network grant, off by default.
   *Technically:* the row engine must accept rows from plugins through a
   typed interface, and a plugin row must be visibly labelled and removable.
   This follows ADR 2's rule that third-party lookups live in plugins.

2. **Dismiss, undo and "hidden" for Continue Watching and Next Up.** (Pain
   point 3, 1,725 Jellyfin votes; Plex's missing undo.) Dismissing writes an
   event to the watch log; undo writes another; a "Hidden" view lists
   everything dismissed. The same mechanism stops tracking a show.
   *Technically:* the append-only watch log (ADR 1, decision 5) needs event
   types beyond "played to position": dismiss, restore, stop tracking, mark
   watched, mark unwatched. The core crate derives the rows from the log
   deterministically so every device computes the same answer offline, and
   events from offline devices must merge without losing intent (ordering by
   a per-device sequence plus server receipt, for example).

3. **One "Up Next" with clear rules, and a toggle to split it.** (Pain point
   5.) Default to one row that holds in-progress items and next episodes,
   ordered by recency, with new-episode arrival bumping a followed show, a
   decay for abandoned shows instead of a hard cut-off, rewatch handled, and
   specials placed by air order. Ordered collections (film series) take part.
   *Technically:* the ordering rules live in the core crate as pure functions
   over the log and the library, so they can be tested to the project's
   mutation standard and documented as a spec rather than discovered by
   users.

4. **Native watchlist, history and personal ratings.** (Pain point 4: 1,294,
   830 and 453 Jellyfin votes.) Watchlist and ratings are user intent and
   belong next to the watch log. Titles not yet on the server can be
   watchlisted through a metadata plugin and turn into a request to the
   owner. *Technically:* ADR 1 says watch history is the only irreplaceable
   data, but watchlists, ratings, playlists and home layouts are also
   user-authored and cannot be rebuilt from files. A new record should extend
   the irreplaceable, exportable store to cover all user intent.

5. **Instant, offline, forgiving search.** (Pain point 8.) Because the
   library is synced to the device, search can run locally with no network
   round trip: typo tolerance, accent folding, apostrophe and transliteration
   handling, prefix matching, and fields for title, original title, people,
   character, tags, genres, album and track. Results are grouped by type with
   chips and a scope selector. *Technically:* the index must be built in the
   core crate and compiled to WASM for the web and UniFFI for native, so
   ranking is identical everywhere. Its size must fit a cheap TV stick's
   memory budget (to be measured), which may mean a compact on-disk index
   rather than a full in-memory one, and CJK text needs a different
   tokeniser from Latin text.

6. **One rule language for rows, smart collections, smart playlists and
   restrictions.** (Pain points 6 and 11; 588 votes for smart playlists, 12
   for smart rows, Emby's request open since 2020, Plex's Pass gate.) A
   single rule format ("unwatched, 4K, comedy, under 100 minutes, sorted by
   date added") powers a home row, a smart collection, a smart playlist and a
   kids profile's allowed set. *Technically:* a versioned rule AST in the
   core crate, evaluated identically on server and device, with every field
   the scanner knows, including audio and subtitle languages from Gunmetal's
   own parsers (172-vote Jellyfin request). Rules need a stable,
   forward-compatible serialisation because users will keep them for years.

7. **Per-user home layouts that follow the user, with household defaults.**
   (Pain point 6; Plex's vanished pins, Infuse sync complaints, Emby's
   per-user setup.) The owner defines a default layout; each user can change
   it; the layout syncs to every device the user signs into; a TV can
   override density and artwork shape. Changes never happen silently on
   upgrade. *Technically:* layouts are user data stored with other user
   intent (see idea 4), referenced by stable row IDs, and migrated
   explicitly when the row schema changes.

8. **Local, explainable recommendations.** (Pain point 7.) "Because you
   watched X: same director", "More from this composer", "Unwatched in a
   collection you started", "Watch again", "Popular in this household"
   (opt-in). Every row says why. A "not interested" action writes to the
   log. *Technically:* no central account means no vendor cloud, so
   similarity must come from metadata (genres, keywords, people, studios,
   tags) and household co-watching, computed on the server into a rebuildable
   neighbour table in SQLite and synced to devices. Small catalogues and
   single households make collaborative filtering weak, so metadata
   similarity carries most of the load, and metadata quality depends on
   plugins the user may decline.

9. **Kids profiles enforced at sync, with a real kids interface.** (Pain
   point 12.) A kids profile receives only allowed items in its synced
   library, so nothing restricted is ever on the device to leak through
   search, artwork or screensavers. It gets a simpler, larger interface,
   per-title exceptions, viewing schedules, and a lock that keeps it out of
   adult profiles. *Technically:* per-object authorisation (ADR 1, decision
   6) must apply to the sync feed and to signed stream URLs, not only to API
   calls; artwork and backdrops must be filtered by the same rules; and
   profile switching needs a PIN or device-key gate that cannot be bypassed
   by reading the local cache.

10. **A TV interface built on Plex's lesson.** (Pain point 2 and the TV
    rows.) A left rail that is one press away, libraries reachable in at most
    two presses, focus memory on back, a long-press menu on every card,
    alphabet jump on every grid, uncapped rows, portrait or landscape art per
    row, and rows drawn instantly from the local cache. *Technically:* the
    React Native TV focus system must be measured on the cheapest supported
    devices before visual polish is added; large grids need virtualisation;
    and a performance budget (time to interactive home, scroll frame times)
    should be a test, in keeping with the project's gate.

11. **Recently added that understands files.** (Pain point 10.) Upgrades
    and replacements do not count as new; episodes group by show with a
    "3 new" badge; selecting a grouped show opens the show; release date and
    added date are separate rows. *Technically:* the scanner must recognise
    a replaced file as a new version of an existing item (content identity,
    not path), which fits the scan-time indexing in ADR 1, decision 4.

12. **Seasonal and editorial rows without a paywall or external tool.**
    (Pain points 6 and 14.) Rows can carry a date range (December only) and
    the owner can publish a curated row to everyone, which users can hide.
    *Technically:* date-range conditions in the rule language from idea 6,
    evaluated against the device's local date.

13. **Music discovery that works on day one.** (Pain point 13; ADR 2.)
    Continue listening (albums, playlists, long-form audio with resume),
    recently played, recently added, "On this day", rediscover (loved but not
    played lately), and library radio seeded by artist, album or genre using
    metadata. Sonic analysis and AI playlists can come later as plugins.
    *Technically:* play events for music go into the same log; the
    "continue listening" rule must treat albums and playlists as the unit,
    not tracks; resume positions apply only above a length threshold.

14. **Operating-system integration from local data.** (TV and OS rows.)
    Android TV's Play Next row and Apple TV's Top Shelf can be filled from
    the synced log without a server call. *Technically:* native modules per
    platform behind the React Native UI (ADR 1, decision 8), kept in step
    with dismissals so the OS row never shows something the user hid.

## Risks and hard parts

- **Synced library on weak TVs.** Instant offline browsing and search depend
  on a local copy of the library, its index and some artwork. Cheap sticks
  have little RAM and storage, and Apple TV may purge caches. A partial sync
  for TVs (metadata only, artwork on demand) may be needed, which weakens the
  "works offline" promise on those devices.
- **Merging intent from offline devices.** Dismissals, watchlist changes and
  ratings made offline on two devices must merge predictably. Getting this
  wrong produces exactly the "my pinned libraries vanished" complaints Plex
  receives.
- **User data versus a rebuildable cache.** ADR 1 treats SQLite as
  rebuildable and only watch history as precious. Layouts, watchlists,
  ratings, rules and playlists are also precious. Without a decision, they
  risk being lost on a cache rebuild.
- **Recommendation quality with little data.** A household of three with
  2,000 films does not produce strong collaborative signals. Metadata-based
  similarity is predictable but bland, and depends on external metadata the
  user may decline to fetch. Shared profiles also pollute signals.
- **Search quality across languages.** Typo tolerance, accent folding,
  transliteration and CJK tokenisation are each a project. Jellyfin and
  Infuse show these bugs keep coming back.
- **Kids mode is security, not styling.** If any restricted title, image or
  search hit reaches a kids device, the feature has failed. Client-side
  hiding is not enough, and PINs are guessable.
- **Customisation versus good defaults.** Plex was punished both for removing
  customisation (2021) and for changing defaults (2025). Too many settings is
  its own failure; Gunmetal needs strong defaults and a small set of
  powerful controls (rules, layouts) rather than many toggles.
- **TV focus performance in React Native.** Focus handling and large
  virtualised grids on TV are the most common source of lag; this is where
  the Plex Fire TV app failed.
- **Trade dress.** Hero cards, expanding tiles and numbered rows are common,
  but ADR 2 forbids copying another product's look. The interface needs its
  own visual identity while using familiar interaction patterns.
- **Feature creep from streaming apps.** Vertical clip feeds, AI search and
  social features cost a lot and were poorly received by self-hosters when
  Plex tried social discovery.

## Open questions

1. Should a new architecture record make all user intent (watchlist,
   ratings, dismissals, layouts, rules, playlists) part of the irreplaceable,
   exportable log alongside watch history?
2. Which rows are computed on the device and which on the server? Device
   computation is instant and offline; server computation is cheaper for
   weak TVs and keeps recommendations consistent.
3. How much of the library does a TV sync: everything, metadata only, or a
   window around what the user browses?
4. Is "Up Next" merged by default, with a split toggle, or the reverse?
5. Do we show titles that are not in the library at all (watchlist entries,
   upcoming episodes), and only through plugins?
6. Is a kids profile a separate profile type with its own interface, or an
   ordinary profile with a restriction rule? Which rating systems ship first?
7. Do trailers and backdrop video exist in the first version, given they
   require third-party sources and bandwidth?
8. Is there any household-level social feature ("popular on this server",
   recommend to another user), and is it opt-in per user for privacy?
9. Is music a mode of the same home or its own home tab? ADR 2 says
   music-forward with a persistent player, but the video home and the music
   home want different rows.
10. Where does natural-language search sit, if anywhere: a plugin with an
    explicit grant to a model provider, a local model, or not at all?
11. What are the measurable performance budgets (time to home, search
    latency, scroll frame time) on the cheapest supported TV, and where are
    they tested?

## Sources

- https://www.xda-developers.com/after-months-backlash-plex-rolling-back-controversial-big-screen-app-redesign/
- https://www.howtogeek.com/plex-brings-back-sidebar-navigation-after-users-refuse-to-let-it-go/
- https://piunikaweb.com/2025/09/17/plex-roku-update-backlash/
- https://piunikaweb.com/2026/05/04/plex-fire-tv-redesign-unusable-users-slam-navigation/
- https://www.pcworld.com/article/2653520/plexs-new-experience-arrives-6-things-to-know-before-updating.html
- https://techcrunch.com/2025/03/31/streaming-service-plex-rolls-out-a-revamped-mobile-app
- https://techcrunch.com/2022/04/05/huge-plex-update-adds-a-universal-watchlist-cross-service-search-and-new-discovery-features
- https://techcrunch.com/2025/01/22/streaming-service-plex-gets-more-social-with-public-profiles-and-reviews/embed/
- https://www.androidauthority.com/plex-discussions-hands-on-3677364/
- https://en.wikipedia.org/wiki/Plex
- https://www.plex.tv/plex-pass/
- https://www.plex.tv/plexamp/
- https://support.plex.tv/articles/parental-controls/ (via search summary; direct fetch refused)
- https://support.plex.tv/articles/publishing-collections/ (via search summary; direct fetch refused)
- https://support.plex.tv/articles/201273953-collections/ (via search summary; direct fetch refused)
- https://forums.plex.tv/t/new-ui-is-an-awful-experience/931048
- https://forums.plex.tv/t/roku-new-experience-release-update/931239
- https://forums.plex.tv/t/vote-to-roll-back-to-plex-classic/931767
- https://forums.plex.tv/t/implemented-fully-disable-discover-search-results-from-streaming-services-more-ways-to-watch/786259
- https://forums.plex.tv/t/new-search-release/853473
- https://forums.plex.tv/t/how-to-customize-the-new-home-screen-in-the-app-it-s-horrible/915559
- https://forums.plex.tv/t/stop-messing-with-my-home-screen/716342
- https://forums.plex.tv/t/recent-update-broke-more-option-pinned-libraries-gone-on-multiple-clients-google-tv-roku/940434
- https://forums.plex.tv/t/give-users-more-control-over-their-hubs-compensate-the-removal-of-the-manually-managed-home-screen/711411
- https://forums.plex.tv/t/nested-collections/354043
- https://forums.plex.tv/t/recommend-server-content-not-just-queued/87784
- https://forums.plex.tv/t/replace-libraries-hub-and-menu-bar-with-left-hand-hamburger-menu-new-exp/922415
- https://forums.plex.tv/t/feature-request-provide-a-setting-to-hide-watched-content-on-the-home-screen/443265
- https://forums.plex.tv/t/how-do-i-revert-removing-something-from-continue-watching/768314
- https://forums.plex.tv/t/remove-from-continue-watching-problems/716534
- https://forums.plex.tv/t/add-discover-to-managed-users/794138
- https://forums.plex.tv/t/plexamp-release-notes/221280
- https://forums.plex.tv/search.json (feature-suggestion listings used for thread titles and post counts)
- https://features.jellyfin.org/api/v1/posts (queries: most-wanted, home screen, search, next up, recommendations, kids, filter sort, recently added)
- https://features.jellyfin.org/posts/1439/custom-carousel-options-on-home-screen
- https://features.jellyfin.org/posts/2735/configurable-homescreen
- https://jellyfin.org/posts/
- https://jellyfin.org/posts/jellyfin-release-10.11.0/
- https://jellyfin.org/posts/jellyfin-release-12.0/
- https://jellyfin.org/posts/state-of-the-fin-2026-01-06
- https://jellyfin.org/posts/state-of-the-fin-2026-05-24
- https://jellywatch.app/blog/jellyfin-current-version-latest-features-march-2026
- https://github.com/jellyfin/jellyfin/issues/16088
- https://github.com/jellyfin/jellyfin/issues/14088
- https://github.com/jellyfin/jellyfin/issues/14704
- https://github.com/jellyfin/jellyfin/issues/12624
- https://github.com/jellyfin/jellyfin/issues/15116
- https://github.com/orgs/jellyfin/discussions/12444
- https://github.com/jellyfin/jellyfin-web/issues/6793
- https://github.com/IAmParadox27/jellyfin-plugin-home-sections
- https://github.com/IAmParadox27/jellyfin-plugin-collection-sections
- https://github.com/ranaldsgift/KefinTweaks
- https://github.com/damontecres/Wholphin
- https://github.com/streamyfin/streamyfin
- https://github.com/jellyfin/Swiftfin
- https://github.com/jmshrv/finamp
- https://symfonium.app/
- https://www.navidrome.org/docs/overview/
- https://emby.media/community/blogs/entry/604-designing-the-perfect-emby-home-screen-a-complete-customization-guide/
- https://emby.media/community/topic/92899-customizable-sections-on-home-screen/
- https://emby.media/community/topic/147798-new-emby-server-release-4950/
- https://emby.media/community/topic/146080-plugin-home-screen-companion/
- https://github.com/soderlund91/HomeScreenCompanion
- https://github.com/EmbySupport/Emby.Docs/pull/77
- https://firecore.com/releases
- https://community.firecore.com/t/increase-number-of-items-in-home-screen-lists/23941
- https://community.firecore.com/t/time-for-a-home-screen-ui-refresh/57276
- https://community.firecore.com/t/home-screen-layouts-library-lists-not-syncing-across-devices/55695
- https://community.firecore.com/t/retain-home-screen-customizations-when-switching-plex-home-users/59343
- https://community.firecore.com/search.json (home screen topic listing)
- https://www.engadget.com/entertainment/streaming/netflix-overhauls-its-tv-app-with-a-fresh-ui-and-responsive-recommendations-121511958.html
- https://www.macrumors.com/2025/08/13/netflix-rolls-out-redesigned-interface-apple-tv/
- https://www.cnn.com/2025/05/07/media/netflix-new-home-page-ai-search-vertical-video (via search summary; direct fetch refused)
- https://help.netflix.com/en/node/100639
- https://help.netflix.com/en/node/264
- https://www.apple.com/newsroom/2025/06/apple-tv-brings-a-beautiful-redesign-and-enhanced-home-entertainment-experience/
- https://appleinsider.com/articles/25/06/18/tvos-26-hands-on-sleek-liquid-glass-redesign-new-control-center-and-more
- https://developer.android.com/training/tv/discovery/watch-next-add-programs
- https://github.com/jurialmunkey/skin.arctic.fuse.2
- https://github.com/jurialmunkey/skin.arctic.fuse.3
- https://github.com/jurialmunkey/plugin.video.themoviedb.helper
- https://kometa.wiki/en/latest/files/settings/
- https://agregarr.org/docs/ui/home/
