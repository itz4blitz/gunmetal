# Discovery, home and search

This map covers every way a person finds something to play in Gunmetal: the
home screen and its rows, continue listening and watching, what is new,
loves, ratings, watchlists and history, recommendations and library radio,
search, browsing and filters, the rule language behind smart playlists and
smart collections, curated rows, profiles and kids mode, the ten-foot TV
interface, and the operating-system and car surfaces that start playback.
The bar is this: every screen that helps someone choose is computed from
their own library on their own device, so it opens instantly and works
offline; nothing on it comes from a vendor; and the controls that Jellyfin
users have requested since 2019 and that Plex keeps behind a pass (dismiss,
undo, watchlist, history, ratings, custom rows, smart playlists) are built in
and free. It was written on 2026-10-02 from the files in `docs/research/`,
mainly `discovery-home-and-search.md`, `music-ux.md` and
`pain-points-and-demand.md`, and it stays inside ADR 1 and ADR 2.

## Features

How to read the tables:

- **Rivals today** uses the research files' terms: yes, partial, plugin,
  Pass (Plex Pass needed), no. For music features the rivals named are
  Plexamp, Jellyfin with Finamp, Navidrome and Spotify, because those are
  what people compare a music player with.
- Releases (R1, R2, R3, Later, No), the Demand scale, row ownership and the
  terms "the user log" and "the identity store" are defined once in the
  [feature map README](README.md). A row whose Release cell would differ
  between maps names one owning row; the other maps point at it.
- **The log** means the user log. Many features here assume it holds all
  user intent, not only watch history; that needs ADR 3 (open decision 1).
- **The sync feed** means the per-profile copy of the library that the
  README and ADR 1 sync to each device.
- **The rule language** means DIS-119.
- Where a feature also belongs to another area (the play queue, playlist
  editing, profiles and sign-in, client platforms, plugins), this map covers
  only the part that decides what someone finds and plays.

### Home screen and layout

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| DIS-001 | Home made only of your media | Rows come from your own libraries; there are no vendor catalogue, live channel, rental or upsell rows to hide | Plex yes, mixed with its own Discover, Live and On Demand rows that some clients could not remove in 2025; Jellyfin yes; Emby yes | High: Plex "disable Discover" request reached 1,000 votes; 2025 Roku complaints about rows that could not be removed | R1 | Parity with Jellyfin and Navidrome, which have no streaming catalogue either; ahead of Plex, whose Discover rows drew a 1,000-vote request to disable them. Gunmetal has no catalogue and no vendor cloud (ADR 1, decision 7), so there is nothing to promote; outside content can arrive only as a labelled plugin row (DIS-018) | Row definitions; library index in the sync feed | Home |
| DIS-002 | Instant home | Home opens without a spinner and still works with no connection | Plex, Jellyfin and Emby fetch each screen from the server; Plexamp 4.50 now caches library data | High: Jellyfin lazy loading 1,179 votes; Jellyfin offline mode 1,820 votes | R1 | Rows are evaluated by the core crate (WASM on the web) against the library already synced to the device, so drawing the home needs no server call | Delta sync feed of library metadata and the profile's log; per-device sync cursor | Home; sync status indicator |
| DIS-003 | Build your own home | Add, remove, rename and reorder rows; any filter can become a row | Plex partial (pin libraries; manual home management removed in 2021); Jellyfin partial (fixed section slots, plugins); Emby yes; Spotify no; Plexamp Pass | High: Spotify "Customize Start Screen" 15,697 votes; Jellyfin home sections 169 votes, planned | R1 | Free, and every row is a saved rule (DIS-119), so a new kind of row needs no plugin and no release | Layout stored as user intent in the log; rule validation | Home edit mode; row menu; add-row sheet |
| DIS-004 | Good default home and empty states | A new user gets useful rows with no setup, and an empty or scanning library says what is happening | Plex weak (an iOS home stayed blank until libraries were favourited with an undocumented long-press, April 2025); Jellyfin always shows My Media; Emby similar (unverified) | Medium: Plex staff called the missing onboarding a likely bug | R1 | Default rows ship as definitions with stable IDs (continue listening, recently added, recently played, rediscover, a mix); during the first scan Home shows scan progress instead of nothing | Scan progress events; shipped default layout | Home; first-run cards |
| DIS-005 | Household default layout | The owner sets the starting home for everyone; each person can still change theirs | Plex partial (admin hub order applies to all); Jellyfin plugin; Emby plugin (Home Screen Companion) | Medium: Jellyfin "allow global defaults" 258 votes and "default user settings" 130 | R2 | A person's layout stores only its differences from the household default, so the owner's later changes still reach rows that person never touched Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Household layout; per-profile overrides | Admin: home defaults; "reset to household default" |
| DIS-006 | Copy a layout to others | Set up one person's home and apply it to several people | Plex no (unverified); Jellyfin no; Emby plugin mirrors a layout to many users | Low: Emby users set up each account by hand without the plugin | R2 | Layouts are data, so copying is a single server operation | Admin-only copy operation | Admin: user page |
| DIS-007 | Layout follows you | The same rows, pins and order on every browser and, from R2, every device | Plex partial (pinned libraries vanished for more than ten users of one server after a July 2026 update); Jellyfin partial (settings stored per client, unverified); Emby partial; Infuse users report layouts not syncing | Medium: the Plex and Infuse threads above | R1 | The layout syncs like history and is never rewritten by an upgrade without an explicit, tested migration (DIS-015) | Layout sync; merge rule per row | Home on every client |
| DIS-008 | Per-device overrides | A denser TV home, or landscape art on the TV and posters on the phone | Plex no (unverified); Jellyfin implicitly, because each client stores its own; Emby yes | Low: offered by Emby and Wholphin | R2 | An override layer per device class sits on top of the profile layout, so it never forks it | Device-class overlay in the layout | TV and phone home settings |
| DIS-009 | Rows as long as you like | Uncapped rows, and "See all" opens the full list | Emby configurable limit; Jellyfin fixed (28 and 26-vote requests); Plex fixed (unverified); Infuse users have asked since 2020 | Medium: the Jellyfin requests and the Infuse thread | R1 | Rows are local queries over synced data, so length costs the server nothing; long rows are virtualised | None beyond the sync feed | Home row; See all page |
| DIS-010 | Artwork shape per row | Posters, square covers or landscape thumbnails, chosen per row | Emby yes; Jellyfin partial (plugin; Wholphin per row); Plex partial (unverified) | Low: offered by Emby and Wholphin | R2 | parity | Artwork variants per item | Row settings |
| DIS-011 | Hide finished items in a row | Rows stop offering films and episodes you have already watched | Plex no (38-vote request since 2019); Jellyfin partial (unverified); Emby partial (unverified) | Low: 38 votes | R2 | An "unfinished only" condition in the row's rule, evaluated on the device | Watch state in the sync feed | Row settings toggle |
| DIS-012 | Keep a library off home | A library (home videos, the kids' music, sleep sounds) stays out of rows, suggestions and radio seeds | Plex yes (by not favouriting it); Jellyfin yes for latest media, exclusion from suggestions requested (12 votes); Emby yes (unverified) | Low: 12 votes | R1 | One per-profile switch covers home rows, recommendations and radio together | Per-profile library flags | Library menu; home settings |
| DIS-013 | Pinned shortcuts | A fixed grid of the playlists, albums, artists or libraries you open daily | Spotify shortcut grid; Apple Music pins (iOS 26); Plex pins (lost on some TV clients in 2026); Infuse pinned favourites with custom art | Medium: Infuse shipped pinning through 2026; the Plex pin loss | R1 | Pins are user intent in the log, so an update cannot drop them | Pin list per profile | Top of Home; "Pin" in every context menu |
| DIS-014 | Choose the start screen | Open on Home, Library, Search or wherever you left off | Plex partial (unverified); Jellyfin requests (1 and 2 votes); Infuse can hide its Library icon | Low: single-digit votes | R2 | parity Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Per-device preference | Settings |
| DIS-015 | A home that does not move | Rows, pins and navigation stay put across upgrades; layout changes arrive as opt-in previews with a way back | Plex poor (2025 to 2026 redesigns, partly reversed in August 2026); Jellyfin good in core, plugins can break on upgrade; Emby good | High: "Vote to roll back to Plex Classic" 504 votes; new-experience feedback thread 1,531 posts | R1 | Rows have stable IDs and a versioned schema with tested migrations; the positions of core controls are covered by visual regression tests | Versioned layout schema and migrations | All navigation; release notes with before and after screenshots |
| DIS-016 | Featured spotlight | One large item at the top: the owner's pick, a random unwatched film or a new album | Plex partial (Discover pages); Jellyfin plugin (13 and 9-vote requests); Emby spotlight tied to 4.10; Netflix | Low: the small Jellyfin and Infuse requests | R2 | The spotlight's source is a rule, and its design is Gunmetal's own rather than another product's hero card (ADR 2, decision 7) | Owner pick; rule | Home hero card |
| DIS-017 | Music and video kept apart | Separate music and watch homes, with a combined view only if wanted; spoken word never appears in music rows unless asked for | Spotify mixes podcasts into Home; Apple keeps podcasts in another app; Plex moved music out to Plexamp | High: Spotify "hide podcasts entirely" 8,815 votes | R2 | Every item carries a media kind and every default row set is per kind, chosen per profile | Media kind on every item | Top-level Music and Watch switch |
| DIS-018 | Plugin rows | A plugin can add a row (a trending list, upcoming releases); the row says where it came from and can be removed | Plex ships its own rows; Jellyfin plugins (Home Screen Sections, 519 stars, web only, needs two other plugins); Emby plugin | Medium: plugin popularity on Jellyfin and Emby | Later | A typed row interface; a plugin row needs an explicit network grant (ADR 2) and always shows a source badge | Plugin row API; grant checks | Row header badge; row menu |
| DIS-019 | Speed you can check | Published numbers for time to home and search latency on a large library, measured on the cheapest supported devices | None found in the research | High: performance at scale is the seventh-ranked theme in the demand research | R1 | Budgets are tests in the gate, like coverage. Proposed design goals, unmeasured until the benchmark exists: home rendered from the local store in under 200 ms and browse or search answers in under 50 ms at 100,000 tracks on the reference low-end device; initial sync of 100,000 tracks in under 2 minutes on a home network; time to interactive on the cheapest supported stick under 3 seconds. The README already commits to publishing benchmark numbers either way. CLI-022, CLI-038, DIS-161 and ADM-010 point here. | Benchmark library fixtures | Docs; release notes |

### Continue listening, continue watching and Up Next

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| DIS-020 | Continue listening | Albums, playlists and long recordings you were part-way through, resumed where you stopped | Plexamp yes; Jellyfin partial (requests of 4 and 2 votes); Navidrome album lists (unverified detail); Spotify and Apple Music lead | Medium: few votes, but every streaming app opens with it | R1 | Derived from the log with the album or playlist as the unit, not the track; a position is kept only above a length threshold; computed on the device | Listen events that carry their context (album, playlist, radio); resume positions come from MUS-078 | Home row; resume card |
| DIS-021 | Recently played | The last things you played, one tap away | Spotify shortcut grid; Apple Music shelf; Plexamp (unverified); Finamp sections (unverified) | Medium: the music research puts it first on any home | R1 | Read straight from the local log, so plays made offline show at once | None beyond the log | Home row; History |
| DIS-022 | Dismiss from Continue rows | Remove something you abandoned from Continue Listening, Continue Watching or Up Next without marking it played | Plex yes (inconsistent on some clients); Jellyfin no (1,725 votes, planned since 2020, and no API for it); Emby yes; Infuse added it for Plex and Emby only | High: Jellyfin's second most-wanted request | R1 | Dismissing writes an event to the log, so it holds on every device, offline and in OS rows; the native API exposes it | Dismiss event type; API endpoint | Card context menu; swipe action |
| DIS-023 | Undo, and a Hidden page | An undo right after any dismiss or hide, and a page that lists everything hidden so it can be restored | Plex has no documented undo; no rival found | Medium: Plex users ask how to revert removals and want a confirmation | R1 | Restoring is another log event; the Hidden page is a query over the log | Restore event | Undo toast; Hidden page |
| DIS-024 | Continue Watching | Resume half-watched films and episodes | Plex yes; Jellyfin yes; Emby yes | Medium: expected by everyone | R2 | Same derivation as DIS-020; progress is keyed to content identity, so replacing or renaming a file keeps it (Jellyfin #15001, 61 +1) | Watch log; content identity from the scanner | Home row; detail page resume |
| DIS-025 | Next Up | The next unwatched episode of each show in progress | Plex yes (inside Continue Watching); Jellyfin yes; Emby yes | Medium: expected | R2 | The ordering rules are pure, documented functions in the core crate, tested to the mutation standard | Episode order and air dates | Home row |
| DIS-026 | One Up Next row, or two | One row of in-progress items and next episodes by default, with a switch to split them | Plex merged with no toggle (36-vote request); Jellyfin split (merge request 203 votes); Emby split (unverified); Wholphin offers both | Medium: 203 and 36 votes | R2 | A toggle satisfies both camps; the merged order follows written rules (recency, new-episode bump, decay) | None | Home; settings toggle |
| DIS-027 | Stop following a show | Never be offered the next episode of a show you gave up, without hiding the show | Plex via dismiss; Jellyfin declined (2026); Emby "Forget a series in Next Up" 166 replies | Medium: 166 Emby replies | R2 | A reversible unfollow event in the log | Unfollow event | Show page; card menu |
| DIS-028 | Abandoned shows fade | Shows untouched for a while drop off Up Next, but not merely because a season break is long | Jellyfin "max days in Next Up" (ignores gaps between seasons); Plex (unverified) | Low: Jellyfin users report the gap problem | R2 | The rule uses the last air date as well as the last watch | Air dates | Settings |
| DIS-029 | New episodes jump the queue | A new episode of a show you follow moves it to the front | Jellyfin no (requests to sort by availability); Emby plugin; Netflix and Apple TV app (unverified) | Medium: open Jellyfin requests | R2 | Episode arrival time is an input to the Up Next order | Added-at per episode | Home |
| DIS-030 | Rewatches and specials | Restart a finished show and Up Next follows; specials sit where they aired | Jellyfin yes (both completed requests); Plex (unverified) | Low: completed requests | R2 | parity; rewatches are separate log entries, so counts stay right | Air-order data | Home; show page |
| DIS-031 | Up Next for film series | "Watch the next film" in an ordered collection you started | No rival (Jellyfin 36-vote request) | Low: 36 votes | R2 | Ordered collections (DIS-131) feed the same Up Next rule | Collection order | Home; collection page |
| DIS-032 | Progress on cards | Progress bar, time left, episode label and an "ends at" time | Plex, Jellyfin and Emby yes; Apple TV app the richest | Low: parity | R2 | parity | None | Cards |
| DIS-033 | Mark played or unplayed in bulk | Fix history after importing a library, or mark a whole season watched | Plex, Jellyfin and Emby yes (unverified); Infuse multi-select | Medium: needed after every migration | R2 | Each mark is a reversible log event, so a mistaken bulk mark can be undone | Batched events | Context menu; selection bar |
| DIS-034 | The right episode starts | Choosing a show from any row plays your next unwatched episode | Jellyfin no (11-vote request; web bug #8496 starts the newest episode) | Low: 11 votes plus a 2026 bug | R2 | One "play show" rule in the core, used by every surface | None | Show cards; play buttons |

### What is new

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| DIS-035 | Recently added | New arrivals per library | Plex, Jellyfin and Emby yes | Low: parity | R1 | parity | Added-at per item | Home row; library sort |
| DIS-036 | Arrivals grouped by album | A new album shows as one card, and new tracks added to an album you already have show as that album with a count | Plex, Jellyfin and Emby group new episodes by show (Plex and Emby unverified) | Low | R1 | parity | Grouping rule | Home row |
| DIS-037 | New episodes as one show card | "3 new" on a show card, which opens the show | Jellyfin groups but the card does not open the show (126 votes, July 2026); Netflix labels new episodes | Medium: one of the fastest-growing Jellyfin requests of 2026 | R2 | The card links to the show and its badge counts only episodes you have not seen | None | Home row; badge |
| DIS-038 | Upgrades are not "new" | Replacing an MP3 rip with FLAC, or 1080p with 4K, neither re-announces the item nor resets its history | Jellyfin no (62 votes); Infuse users see duplicates from pinned Plex collections; Plex (unverified) | Medium: 62 votes | R1 | The scanner recognises a replacement as a new version of the same item by content identity, not by path, and records it as an upgrade | Content identity; version history | Optional "upgraded" badge; admin activity |
| DIS-039 | Hide one arrival | Drop a mistaken import from Recently added | Jellyfin no (10 votes) | Low | R2 | A hide event per profile; the owner can also hide it for everyone Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Hide event | Card menu |
| DIS-040 | Recently released | A row ordered by release date, separate from the date the file arrived | Plex "Recently Released" hub (unverified); Jellyfin request (3 votes) | Low | R2 | Release dates from tags and MusicBrainz IDs are stored apart from added-at Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Release date field | Home row option; sort key |
| DIS-041 | On now and starting soon | Live TV rows: on now, starting soon, new tonight | Plex yes (a Recommended view with seven rows); Jellyfin "On Now" row; Channels DVR "On Later" | Medium: Plex's live discovery view is called the richest | R3 | Rows are computed on the device from a synced guide window, so they appear instantly | Guide window in the sync feed | Home rows; Live TV |
| DIS-042 | What's-new digest | An optional weekly note of new arrivals, by push or email | Tautulli newsletters for Plex; Discord announcements by webhook; not built into any server | Low: third-party tools fill it | Later | A notification plugin with an explicit grant; each digest respects that person's library access | Digest job; notification plugin API | Notification settings |
| DIS-043 | New releases from artists you follow | Know when a followed artist releases something you do not own | Spotify Release Radar; servers know only what has been added | Medium: Spotify's model, and a limit the music research names | R2 | A MusicBrainz plugin with a network grant, off by default; results are marked "not in your library" R2, since the MusicBrainz plugin (LIB-111) is R2. Owns new releases; MUS-176 points here. | Follow list; plugin | Artist page follow; plugin row |
| DIS-044 | Upcoming episodes | Episodes airing soon for shows you follow | Plex partial (unverified); Jellyfin plugin fed by Sonarr and similar tools; Sonarr's own calendar | Low | Later | A labelled plugin row | Plugin schedule data | Home row; show page |

### Loves, ratings, watchlist and history

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| DIS-045 | Loves | One tap to love a track, album, artist, playlist or title | Plex, Jellyfin, Emby and Navidrome yes; Jellyfin loses song favourites during playback (#14981, 25 +1) | Medium: Spotify's "Bring back the heart button!" 5,769 votes shows how much this control matters | R1 | Loves are user intent in the log, outside the rebuildable cache, so neither a rebuild nor a playback race can lose them | Love events | Now-playing bar; context menu; detail pages |
| DIS-046 | Loved songs as a list | Everything you loved, as a list you can shuffle, filter by genre and download | Spotify Liked Songs; Jellyfin requested (25 votes); Navidrome starred view | Medium: Spotify "organise Liked Songs by genre" 6,198 votes | R1 | It is a built-in smart playlist, so genre and every other filter work on it | None | Library: Loved |
| DIS-047 | Personal ratings | Rate things yourself and sort or filter by your own score | Plex yes; Jellyfin no (453 votes, planned); Navidrome yes, per user | High: 453 votes | R1 | Ratings are user intent in the log and can be used in any rule Owns personal ratings across media; MUS-181 lists what music can rate. | Rating events | Context menu; detail pages; filters |
| DIS-048 | Watchlist | Save films and shows for later | Plex yes (a top-level tab); Jellyfin no (1,294 votes, planned); Emby partial via favourites (unverified) | High: 1,294 votes | R2 | The watchlist is user intent and a rule source, so "on my watchlist, under 100 minutes" is one row | Watchlist events | Detail page button; Watchlist page; Home row |
| DIS-049 | Watchlist titles you do not own | Save a film you heard about; it can become a request to the owner | Plex yes (universal watchlist since 2022); Jellyfin plugin (Seerr); Emby plugin | Medium: Seerr's watchlist auto-request is widely used with Plex | Later | Needs a metadata plugin with a network grant, off by default; entries are marked "not on this server" and can feed a request plugin (DIS-137) | External IDs; plugin | External search results; Watchlist |
| DIS-185 | Share a watchlist or list by expiring link | Send a friend your watchlist or a list of films | Plex Pass has shareable watchlist links (research) | Low: no vote count | Later | Uses the ACC-086 capability model: a signed, expiring, revocable link that shows titles only | None beyond ACC-086 | Watchlist > Share |
| DIS-050 | History by date | Everything you played or watched, day by day, going back years | Plex yes; Jellyfin no (830 votes, planned); Spotify limited (2,035 votes) | High: 830 and 2,035 votes | R1 | History is the log itself (ADR 1), synced, so it is complete, works offline and can be exported | Log queries | History page; "your plays" on detail pages |
| DIS-051 | Your play counts | Play count and last played on every track, album and artist | Plex, Jellyfin and Navidrome store them; Spotify does not show them (963 and 901 votes in 2022) | Medium: the Spotify requests | R1 | Aggregates are computed on the device from the log | None | Detail pages; sort keys |
| DIS-052 | Remove a play | Take one play out of history, statistics and recommendations | Spotify missing (5,458 votes); no rival found | High: 5,458 votes | R1 | A removal event masks the play in every derived view while the log itself stays append-only | Removal event | History; recently played menu |
| DIS-053 | Private session | See ACC-117, which owns this feature. | No rival found in the research; proposed in the users and security research | Medium: tied to the 5,458-vote history request | R1 | See ACC-117. | None beyond ACC-117. | Now-playing toggle; profile badge while active |
| DIS-183 | Your viewing statistics and year in review | Your own film and TV totals, top shows and a year in review, any time | Wrapperr (456 stars), plex-rewind (304) and jellyfin-wrapped exist as add-ons (research) | Medium: three add-on projects show demand; the research says it should be built in | R2 | Built in and computed from the log, with private sessions excluded and a shareable image made only when asked. The shared engine for MUS-187 (music) and LAT-043 (books) | Per-profile aggregates from the log; report job | Profile > Your year |
| DIS-054 | Hide or snooze | Stop seeing a track, album, artist or title in suggestions and radio, for 30 days or for good | Spotify hide and 30-day snooze (2025); Jellyfin "hide a movie or show" 131 votes; Plex (unverified) | Medium: 131 votes | R2 | Hide and snooze are log events with an optional expiry that every rule and radio respects Owns hide and snooze; MUS-170 points here. | Hide event with expiry | Context menu; Hidden page |
| DIS-055 | Less like this | Tell radio and recommendations to steer away from something | Spotify hides, snoozes and excludes tracks from the taste profile; Netflix thumbs; Plex, Jellyfin and Emby no | Medium: Spotify's controls and the requests behind them | R2 | Feedback is a log event used by the local recommender, listed on the Hidden page and reversible Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Feedback event | Context menu; radio now-playing |
| DIS-056 | Reset recommendations | Start suggestions fresh without deleting history | Spotify missing ("Reset Taste Profile / History" 5,871 votes) | High: 5,871 votes | R2 | A reset marker in the log; recommenders ignore signals from before it while history stays intact Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Marker event | Recommendation settings |
| DIS-057 | Keep things out of your taste | Keep a Christmas album, the kids' music or sleep sounds out of shuffle, radio, charts and recommendations | Plex requested (44 votes); Spotify excludes tracks from the taste profile | Medium: 44 votes | R2 | One per-item flag that recommenders, mixes and charts all read | Exclusion flag | Context menu; playlist settings |
| DIS-058 | Export everything you told the server | History, loves, ratings, hides, watchlist, pins, layouts and rules as files | Plex partial (unverified); Jellyfin plugin; KefinTweaks exports its watchlist to JSON | Medium: export requests on every board | R1 | Exports come straight from the log in a documented format | Export job | Your data page |
| DIS-059 | Trakt and tracker sync | History and watchlist kept in step with Trakt or a similar service | Plex third-party (PlexTraktSync); Jellyfin plugin; Infuse built in | Medium: Plex "full Trakt integration" 376 votes | Later | A per-user plugin with a network grant that reads and writes the log; Trakt has required VIP to create API apps since August 2026 | Plugin hooks on log events | Connected services |

### Recommendations and library radio

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| DIS-060 | More like this | Similar albums, artists or titles on every detail page | Plex, Jellyfin and Emby yes; Spotify "Fans also like" | Medium: expected everywhere | R1 | Similarity comes from tags, credits and the household's own co-listening, precomputed into a neighbour table and synced, so it works offline | Owns the neighbour table: a per-household table of the top N neighbours for each track, album and artist, from shared credits, genres, era and co-listening, plus MUS-175 data when that plugin is enabled. Rebuilt nightly and after a scan, with a size budget per device; per-profile weighting is applied on the device. Cold start: with no listening history, neighbours come only from credits, genres and era, so picks are weak until plays accumulate or MUS-175 is enabled. | Detail pages |
| DIS-061 | "Because you played" rows | Rows seeded by something you played recently | Plex partial; Jellyfin partial, and broken in 10.11.6 (the same 12 films for every seed, January 2026); Netflix | Medium: Jellyfin 88, 85 and 42-vote requests | R1 | Seeds come from the local log and candidates from the neighbour table; tests assert that results change with the seed, which would have caught Jellyfin's bug | Neighbour table | Home rows |
| DIS-062 | Every suggestion says why | "Same composer", "unplayed from an album you started", "you loved this in 2024" | Netflix labels every tile; the servers use row titles only | Medium: Netflix's labels set the expectation | R1 | Recommenders return a reason code with each item, shown as a short label | Reason codes | Card label; row subtitle |
| DIS-063 | Genre, mood and decade rows | Rows such as "Jazz", "Late night" or "The 90s" from your own tags | Plex yes (unverified); Jellyfin plugin; Emby yes (unverified) | Medium: Jellyfin "genres and recommendations on home" 88 votes | R2 | Genres are normalised first (DIS-108), so messy tags do not produce near-duplicate rows Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Genre index | Home rows; Browse |
| DIS-064 | Rediscover | Loved but not played lately, and owned but never played | Plex "Rediscover" hub; Plexamp Aural Fixations (Pass); Jellyfin plugin; Roon ARC "forgotten gems" | Medium: named as the natural discovery surface for an owned collection | R2 | Plain queries over the log and loves, and free Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Rules evaluated by the core over the synced library and log; no server work | Home rows |
| DIS-065 | On this day | Albums released on today's date, and what you played on this day in past years | Plexamp yes; Jellyfin no; Navidrome no | Low: a Plexamp favourite, cheap to compute | R2 | Computed on the device from release dates and the log Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Rules evaluated by the core over the synced library and log; no server work | Home row |
| DIS-066 | Your charts | Most played this week, month and year | Plex weekly and monthly charts; Jellyfin plugin | Low | R2 | Local aggregates over the log; private sessions and removed plays never count Owns charts; MUS-186 points here. | Per-profile play aggregates by week, month and year, rebuilt from the log | Home row; History |
| DIS-067 | Radio from anything | Endless play seeded by a track, album, artist or genre, from your own library | Plexamp artist radio (track and album radio need Pass); Jellyfin Instant Mix; Navidrome Instant Mix via Last.fm, Deezer or plugins | High: the most-replied comment in the "Jellyfin as a Spotify alternative" thread says radio is what you lose | R1 | Runs on the device from synced similarity data, so it works offline; free; respects hides and exclusions; never adds music you do not own Quality is at best parity with Jellyfin's Instant Mix until MUS-175 (similarity data plugin) or DIS-075 (sonic analysis) exists; being free and offline does not help if the picks are poor. Owns radio behaviour; MUS-165 points here. | Radio rules in the core over the DIS-060 neighbour table | "Start radio" action; now-playing source label |
| DIS-068 | Stations | Ready-made stations by era and genre, such as "1970s soul" | Plexamp Time Travel radio (unverified); Spotify Daylist | Low | R2 | A station is a rule plus a spread-out shuffle that keeps one artist from playing twice in quick succession Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | A built-in station rule set shipped in the core, versioned | Browse: Stations |
| DIS-069 | Your mixes | Mixes of your heavy rotation plus related tracks, refreshed daily or weekly | Plexamp Mixes for You (Pass); Symfonium adaptive mixes; Roon Daily Mixes; Jellyfin ListenBrainz recommendations (12.0) | Medium: every streaming app has them | R2 | Mixes are smart playlists with snapshot refresh (DIS-122), so a downloaded mix does not change mid-journey Owns personal mixes; MUS-168 points here. | Snapshot job | Home "your mixes" row |
| DIS-184 | Household blend | A playlist built from two or more people's loves and plays, refreshed regularly | Spotify Blend (research); no media server found | Low: named in the "why people switch back to streaming" table, no vote count | R2 | Spotify Blend without an account or data leaving the house: computed on the server from consenting profiles only, with no name shown per track unless each person opts in | Blend job over consenting profiles' logs | Playlists; consent setting per profile |
| DIS-070 | Suggestions after the queue ends | When your queue runs out, suggested tracks appear in their own lane before they play; off by default | Spotify shows them since May 2025 (Premium); Plexamp autoplay needs Pass; Apple Music users report unrelated tracks after an album | Medium: hidden autoplay is a long-standing complaint | R1 | Suggestions come from the local recommender with a source label; the queue lane itself belongs to the queue's feature map | None | Queue "continue with" lane; toggle |
| DIS-071 | Your top tracks by an artist | An artist page that starts with your most-played songs by that artist | Spotify uses global plays; Symfonium users ask to use the server's top songs | Low | R1 | The honest owned-library version of "Popular", from the local log | None | Artist page |
| DIS-072 | Surprise me | A random unplayed album or unwatched film | Jellyfin plugin (9 votes); Kodi skins (unverified); Infuse users asked in 2020 | Low | R2 | A random rule that respects hides Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Rules evaluated by the core over the synced library and log; no server work | Home button; library header |
| DIS-073 | Local trailers and extras | Trailers and extras you own, on the detail page | Plex, Jellyfin and Emby yes; Infuse 8.4 shows extras from all three | Low: parity | R2 | parity; nothing is fetched from the internet | Extras in the scan index | Detail page |
| DIS-074 | Mix builder | Pick several artists or albums and get one mix | Plexamp only (Pass) | Low | Later | A radio rule with several seeds Owns multi-seed mixes; MUS-169 points here. | None | Mix builder sheet |
| DIS-075 | Sonic similarity | Radio and "similar" chosen by how tracks sound, not only by their tags | Plexamp (Pass; analysis can take hours or days); Jellyfin no (98 votes); Navidrome plugin | Medium: 98 votes; radio is the top gap for people leaving streaming | Later | An opt-in, low-priority, resumable server job, ideally in the same decode pass as loudness; free; the licence of any reused library (bliss-rs is GPL-3.0) is checked first | Analysis job; feature vectors | Library settings; radio |
| DIS-076 | Recommendation plugins | ListenBrainz, Last.fm or a metadata provider feed radio and rows | Jellyfin 12.0 configurable sources including ListenBrainz; Navidrome Instant Mix via Last.fm | Low: the ListenBrainz request had 28 votes | R2 | Plugins with network grants, chosen per library; local sources stay the default The first such plugin is MUS-175. | Provider interface | Library settings |
| DIS-077 | Service playlists matched to your files | ListenBrainz daily and weekly mixes appear as playlists of tracks you own | Jellyfin plugin; Navidrome plugin (283 stars) | Low | Later | Matched by the MusicBrainz IDs read at scan | Plugin; matcher | Playlists |
| DIS-078 | Popular in this household | What people on the server play most, drawn only from those who opt in | Plex request to recommend server content to friends (253 votes); Netflix Top 10 is global | Medium: 253 votes | Later | Aggregates from consenting profiles only, never showing who played what | Consent flags; aggregate job | Home row |
| DIS-079 | Send to someone in the house | Recommend an album or film to another person on the server | Plex request (253 votes) | Medium: 253 votes | Later | Delivered inside the server as a "from Sam" item on that person's home; no email or vendor service | Per-profile inbox | "Send to" menu; Home row |
| DIS-080 | Picks that react in a session | Radio shifts as you skip and love tracks during a session | Netflix only (2025) | Low | Later | Skip and love events adjust weights on the device | None | Radio |
| DIS-081 | Natural-language and mood requests | "Rainy Sunday jazz" builds a list | Plex removed Sonic Sage in 2026 as not consistent enough; Spotify AI Playlist; Netflix opt-in beta | Low: Plex withdrew its version | Later | Only as a plugin with an explicit grant to a model provider the owner chooses; nothing in the core | Plugin hook | Search; new playlist |
| DIS-082 | Online trailers and previews | Trailers fetched online, and optional previews when a tile is focused | Plex cinema trailers (Pass); Jellyfin trailers plugin request 585 votes; Netflix autoplay with a toggle | Medium: 585 votes | Later | A plugin with a network grant; previews off by default, since they are costly on weak TV hardware | Plugin | Detail page; TV focus |

### Search

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| DIS-083 | One search box | Artists, albums, tracks, playlists and, from R2, films, shows, episodes and people, grouped with type chips and no result cap | Plex yes (relevance-ranked since 2023, per-source cap removed); Jellyfin yes; Emby yes; Infuse grouped results (8.5) | Medium: expected | R1 | Grouping and ranking are core code, identical on every client | None | Search screen; type chips |
| DIS-084 | Search on the device | Results as you type with no network wait, including on a plane | No server among the rivals; Plexamp 4.50 offline mode includes search | High: Jellyfin slow-search issues and a slower 10.11 release candidate; offline requests 1,820 votes | R1 | The index is built in the core crate (WASM on the web, UniFFI on native) from the synced library; its size is measured against a cheap TV's memory | R1 builds the search index on the device from the synced library. If the build exceeds the DIS-019 budget on the reference low-end device, the server ships a prebuilt index segment instead. | Search screen; global search field |
| DIS-085 | Forgiving matching | Typos, accents ("Amelie" finds "Amélie"), curly and straight apostrophes, punctuation | Jellyfin no typo tolerance (46 votes) and broke accent matching in 10.11.0; Spotify typo-tolerant (unverified) | Medium: 46 votes and repeated bugs | R1 | Folding and edit-distance matching in the core index, checked against a regression corpus so fixed bugs stay fixed | None | Search |
| DIS-086 | Search tags, genres and moods | "Christmas" finds the tag, not just titles with the word | Jellyfin no (256 votes since 2019; tag search slowed 10.9 and gained an off switch); Plex via filters | High: 256 votes | R1 | Tags and genres are indexed fields on the device, so they cost the server nothing | None | Search; genre chips |
| DIS-087 | Search people by role | Artist, composer, conductor, producer; from R2 actor, director and character | Plex, Jellyfin and Emby yes for cast; Roon cross-links credits; Jellyfin "character" field request (7 votes) | Medium: Plex "By Composer" 89 votes | R1 | Multi-value credits from Gunmetal's own tag parsing are indexed with their role | Credits in the sync feed | Search; person pages |
| DIS-088 | Scope a search | Search only Music, or one library | Plex yes; Jellyfin yes (2025); Emby (unverified) | Low: parity | R1 | parity | None | Scope selector |
| DIS-089 | Recent searches | Repeat a search in one tap, or clear the list | Spotify and Apple Music yes; Jellyfin no (unverified) | Low | R1 | Kept only on the device and never sent to the server | None | Search empty state |
| DIS-090 | Keyboard search and command palette | See CLI-061, which owns this feature. Discovery specifics: search is the palette's default action. | Jellyfin request (3 votes); Spotify queue hotkey request 988 votes (2022) | Low: small but loved on the web | R2 | See CLI-061. | None beyond CLI-061. | Global shortcut; palette overlay |
| DIS-091 | Find inside a list | Filter a 2,000-track playlist, a box set or a collection | Plex "Playlist sorting and searching" 183 votes; Spotify desktop yes | Medium: 183 votes | R1 | A local filter on the open list | None | List header search |
| DIS-092 | Transliteration and CJK | Japanese, Chinese and Korean titles found by script or by romanisation | Infuse added Chinese transliteration (December 2025); Jellyfin transliteration requests open | Low | R2 | A separate tokeniser for CJK text in the core index | None | Search |
| DIS-093 | Search by lyrics | Find a song from a line you remember | Apple Music (unverified); Symfonium request (10 posts); AudioMuse-AI add-on | Low | R2 | Lyrics come from tags and .lrc files read at scan; the index runs on the server if it proves too large for phones and TVs | Lyrics text index | "Lyrics" chip |
| DIS-094 | Voice search | Speak a title into the remote or the phone | Jellyfin Android TV native voice search; Jellyfin Google Assistant and Alexa request 243 votes | Medium: 243 votes | R2 | The operating system's own speech input fills the local search; Gunmetal runs no voice cloud | None | TV and phone search microphone |
| DIS-095 | Clear duplicates | The same title in two libraries, or two editions, shown as one result with its versions | Plex partial (merges results); Jellyfin no (2 votes) | Low | R2 | Results group versions under one item using content identity Grouping in results only; finding duplicates to delete is LIB-196. | Version grouping | Search results |
| DIS-096 | Live TV in search | Programmes on air or coming up appear in search | Plex yes (guide results in global search); Jellyfin basic | Low | R3 | The guide window in the sync feed is indexed with the library | Guide data | "Live TV" chip |
| DIS-097 | Results outside your library | Find a title you do not own, see details, watchlist it or request it | Plex Discover (can be disabled); Infuse TMDB pages for deep links; Jellyfin plugin (Seerr) | Low: many self-hosters actively do not want it | Later | Plugin only, off by default, with results in a separate, labelled section | Plugin search provider | "Elsewhere" section |
| DIS-098 | Search plugins | Plugins add search sources | Jellyfin 12.0 yes; community Meilisearch plugin | Low | Later | A typed provider interface | Provider API | Search |
| DIS-099 | Search several servers | Your server and a friend's in one list of results | Plex yes; Jellyfin no (177 votes); Infuse, Moonfin and Plezy aggregate servers | Medium: 177 votes | Later | Each server's synced library is indexed separately on the device and merged at query time | Multi-server client | Source filter |

### Browse, filter and sort

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| DIS-100 | Endless, smooth lists | Scroll 100,000 tracks or 10,000 films with no pages and no lag | Jellyfin no (1,179 votes, started); Plex yes (unverified); a tvOS 26 beta capped personal categories at 75 items | High: 1,179 votes | R1 | Virtualised lists over local data, with a render budget tested in the gate (DIS-019) | None | All grids and lists |
| DIS-101 | Alphabet jump | Jump straight to "M" | Spotify missing (884 votes); Jellyfin yes; Infuse alphabet scrollbar | Medium: 884 votes | R1 | parity; it also jumps by year when sorted by year | None | Fast scroller |
| DIS-102 | Filters on what the scanner knows | Genre, year, format, bit depth, sample rate, loudness, played, rating, loved, date added | Plex yes (detail unverified); Jellyfin partial ("friendlier filters" 137 votes) | Medium: 137 votes | R1 | Gunmetal's parsers read technical fields at scan, so "24-bit FLAC" needs no extra probe | Technical fields in the sync feed | Filter sheet; chips |
| DIS-103 | Filters remembered | A view opens with the filter and sort you left it on | Jellyfin partial (73 votes, started); Spotify does not keep sort per view | Medium: 73 votes | R1 | Saved per view and per profile, and synced | View state | Every browse view |
| DIS-104 | Sorts that matter | Release year, original year, date added, last played, play count, your rating, and a second sort key; video keys such as last episode added arrive in R2 | Spotify cannot sort albums by year (1,738 votes); Jellyfin partial, with multi-key sort requested (12 votes) | High: 1,738 votes | R1 | Sort keys are computed locally from the scan and the log | None | Sort menu |
| DIS-105 | Save a filter | One step from a filter to a home row, smart playlist or collection | Plex yes (filter to smart collection); Jellyfin no | Medium | R1 | Filters and rows share the rule language, so saving is a copy, not a translation | Rule store | "Save as" in the filter sheet |
| DIS-106 | Folder view | See LIB-008, which owns this feature. Discovery specifics: hidden for restricted profiles, because folder names cannot be filtered. | Jellyfin requested (438 votes, 134 comments); Emby yes (unverified); Plex hides folder view while restrictions are on | High: 438 votes | R1 | See LIB-008. | None beyond LIB-008. | Library "Folders" tab |
| DIS-107 | Grid, list and compact | Visual or dense views, chosen per view | Finamp per tab; Marvis Pro adds Cover Flow; Jellyfin yes | Low | R1 | parity | None | View toggle |
| DIS-108 | Genre clean-up | "Hip-Hop", "Hip Hop" and "Rap/Hip-Hop" show as one genre, and you can merge your own | No rival covered in the research; the music research notes genre tags in owned files are messy | Medium: affects every genre row and filter | R2 | An alias table in the core, with the owner's merges stored as user intent; files are never rewritten Owns genre clean-up; MUS-018 points here. | Alias table | Admin: genres; a genre page notes what was merged |
| DIS-109 | Browse pages | Genre, mood, decade and record-label pages built from your tags | Spotify browse tiles; Plexamp genres, moods and styles (unverified); Jellyfin genres | Medium | R1 | parity, using the normalised genres | None | Browse |
| DIS-110 | Multi-select | Select many items to queue, add to a playlist or collection, love, hide or mark | Spotify yes; Infuse select-all; Jellyfin web limited | Medium: Spotify "add one song to several playlists" 1,026 votes (2022) | R1 | Bulk actions are one batch of log events, undoable together | Batch endpoint | Selection bar |
| DIS-111 | The same menu everywhere | Right-click or long-press gives the same actions on every item: play next, add to, go to artist, love, hide, dismiss | Spotify consistent; Infuse context menus (2025); Jellyfin partial | Medium: consistency is what makes a large app feel small | R1 | One action registry shared by every surface, including TV long-press (DIS-162) | None | Context menu |
| DIS-112 | Back keeps your place | Back and forward return to the exact scroll position | Spotify desktop; browsers | Low | R1 | Scroll state kept in the client's route stack | None | Navigation |
| DIS-113 | Filter by language and quality | "Films with a French track", "4K HDR", "Atmos", "has subtitles" | Jellyfin no (audio language 172 votes since 2019); Plex (unverified) | Medium: 172 votes | R2 | Track languages, codecs and HDR flags come from Gunmetal's own container parsers | Track data in the sync feed | Filter sheet |
| DIS-114 | Studio, network and country pages | Browse films and shows by studio, network or country | Plex yes; Jellyfin partial; Wholphin adds genre and studio browsing | Low | R2 | parity | None | Browse |
| DIS-115 | Flatten single-season shows | Skip a pointless season screen | Jellyfin no (172 votes) | Medium: 172 votes | R2 | A display rule | None | Show page |
| DIS-116 | Box sets collapse | A franchise shows as one tile in a grid | Plex (unverified), Jellyfin (10.11) and Emby (unverified) yes | Low: parity | R2 | parity | None | Library grid option |
| DIS-117 | Spoiler protection | Thumbnails and synopses of unwatched episodes are blurred | Jellyfin no (118 votes); Plex requested (118 votes, for sports) | Medium: 118 and 118 votes | R2 | The client knows watch state locally and blurs without asking the server | None | Episode lists; settings |
| DIS-118 | Home videos by date | A timeline of personal videos by when they were filmed | Plex timeline in its Photos app; Jellyfin no timeline (unverified); Immich leads | Low | R2 | Capture dates come from the MP4 and QuickTime metadata the core already parses; no online lookup | Capture date | Home videos library |

### The rule language: smart playlists, smart collections and saved filters

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| DIS-119 | One rule language | The same rules drive home rows, smart playlists, smart collections, rule-based downloads and, from R2, profile restrictions | Plex smart filters and collections (publishing them to home needs Pass); Jellyfin none in core; Navidrome smart playlists as hand-edited JSON | High: Jellyfin smart playlists 588 votes, planned | R1 | A versioned rule format in the core crate, evaluated identically on server and device, with forward-compatible serialisation people can keep for years | Rule store; re-evaluation when the library changes | Every place rules appear |
| DIS-120 | Rule editor with live preview | Build rules from menus and watch the matching items update | Navidrome has no editor in its UI; Marvis Pro Smart Rules; Plex filter UI | Medium: Navidrome's smart playlist issue drew 232 comments before it shipped | R1 | The preview runs on the device against the synced library | Rule evaluation on the device; server evaluation only for adapters and sync filters | Rule editor sheet |
| DIS-121 | Smart playlists | Music playlists that maintain themselves from tags, plays, last played, ratings, loves, date added, format, loudness and other playlists | Plex yes (free); Jellyfin no (588 votes); Navidrome yes (80+ fields) | High: 588 votes; Plex "playlist presence" rule 101 votes | R1 | Free, and personal fields such as your plays and ratings are evaluated per profile | Rule evaluation on the device; server evaluation only for adapters and sync filters | Playlists; rule editor |
| DIS-122 | Limits, order and refresh | "50 random unplayed tracks" or "top 5% by plays", refreshed live or as a stable daily or weekly snapshot | Navidrome limits, percentage limits and refresh delay; Plex limit, sort and live refresh | Medium: snapshots matter for downloaded lists (Plexamp fixed this in its 4.50 betas) | R1 | Seeded randomness in the core, so a list reproduces exactly on every device; snapshots are stored with their timestamp Limits and order ship in R1; stable snapshots arrive in R2 with MUS-147, which owns snapshot refresh. | Snapshot job | Rule editor |
| DIS-123 | Seasonal rows | A row or playlist active only between set dates, such as December | No server does it in core; Kometa, KefinTweaks and Home Screen Companion schedule rows | Medium: every rival needs a plugin or external tool | R2 | Date conditions in the rule language, evaluated against the device's local date Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | None | Rule editor; Home |
| DIS-124 | Smart collections and video playlists | Rule-based collections and queues for films and shows | Plex yes; Jellyfin no (smart collections 104 votes; smart playlists 588) | Medium: 104 and 588 votes | R2 | The same language, with video fields from Gunmetal's parsers such as audio languages and HDR | None | Collections; playlists |
| DIS-125 | Import Navidrome smart playlists | Bring .nsp rules across from Navidrome | Navidrome only | Low | R2 | An importer that reports any field it cannot map Owns .nsp import; MUS-148 points here. R2, alongside the Navidrome importer (ADM-040). | Importer | Import dialog |

### Collections and curated rows

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| DIS-126 | Pin a list as a row | Any playlist, smart playlist or (from R2) collection becomes a home row | Plex "Visible on Home" (Pass); Jellyfin plugin; Emby via sections (unverified) | Medium: Plex's paywall on it | R2 | Free, one tap from the item's menu Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | None | Context menu; Home |
| DIS-127 | Owner's picks | The owner publishes a curated row to everyone; anyone can hide it | Plex (Pass); Jellyfin plugin; Emby plugin | Medium: Kometa and Agregarr also need Pass to do this on Plex | R2 | Free; a household row carrying an owner label Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Household rows | Admin: home; Home row |
| DIS-128 | Collections | Group films and shows by hand | Plex, Jellyfin and Emby yes | Low: parity | R2 | parity; collections are user intent in the log | Collection store | Collections; "add to" sheet |
| DIS-129 | Automatic franchises | Box sets built from metadata, with a minimum size so one-film collections do not appear | Plex, Jellyfin and Emby yes (unverified); Emby minimum size; Plex hides single-item collections (195-vote request, implemented) | Low: 195 votes | R2 | Needs a metadata plugin; the minimum size is a setting | Plugin metadata | Collections |
| DIS-130 | Nested collections | "Star Wars" holding its trilogies | No rival; Plex 514 votes and 13,407 views; Emby 97 replies; Jellyfin 44 votes | Medium: 514 votes | R2 | Collections may contain collections from the first schema, and the core rejects cycles | None | Collection page |
| DIS-131 | Order inside a collection | Release, chronological or your own order | Plex yes; Jellyfin no (148 votes) | Medium: 148 votes | R2 | The order is user intent, and Up Next for film series (DIS-031) follows it | None | Collection edit |
| DIS-132 | Films and shows together | One franchise holding both | Jellyfin since 10.11; a Jellyfin request to tag films as part of a show has 104 votes | Low | R2 | parity | None | Collection page |
| DIS-133 | "Part of" on detail pages | See which collections a title belongs to | Plex (unverified); Jellyfin 12.0 | Low: parity | R2 | parity | None | Detail page |
| DIS-134 | Hide the collections view | Keep collections inside a library only | Jellyfin no (18 votes) | Low | R2 | parity | None | Library settings |
| DIS-135 | Cross-library collections | One collection spanning several libraries | Jellyfin requests (44 votes) | Low | Later | Collections reference items by ID, not by library | None | Collection page |
| DIS-136 | Collections from online lists | IMDb, Trakt, Letterboxd or MDBList lists kept in sync | Kometa (3,442 stars, Plex only); Jellyfin plugins | Medium: Kometa's popularity | Later | A plugin with a network grant; list collections are labelled and refresh on a schedule | Plugin | Collections |
| DIS-137 | Requests | Ask the owner for something missing and follow its status | Jellyfin plugin (Seerr in Streamyfin and Wholphin), and a built-in feature was declined (24 votes); Plex via watchlist tools (unverified) | Medium: Seerr has 12,770 stars | Later | Integrate Seerr through a plugin instead of rebuilding it; Seerr's password sign-in does not fit passkey accounts as it stands | Plugin | Search; detail page |
| DIS-138 | Leaving-soon labels | Titles due for removal are marked | Netflix labels; Maintainerr | Low | Later | A cleanup plugin sets a flag the rule language can read | Flag | Card label; row |
| DIS-139 | Numbered top lists | A ranked row of the household's most played | Emby plugin; Netflix Top 10 | Low | Later | Household data only, drawn in Gunmetal's own style | None | Home row |

### Profiles and kids mode

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| DIS-140 | Discovery per person | Each profile has its own history, rows, mixes and suggestions | Plex, Jellyfin and Emby yes (separate users); Netflix profiles | Low: parity | R1 | Each profile has its own log, and recommenders never mix profiles | Per-profile log | Everywhere |
| DIS-141 | Quick profile switching | See ACC-019, which owns this feature. | Plex yes; Jellyfin partial (154 votes, started); Emby yes (unverified) | Medium: 154 votes | R2 | See ACC-019. | None beyond ACC-019. | Profile picker |
| DIS-142 | Who's watching at launch | See ACC-019, which owns this feature. | Apple TV (tvOS 26); Plex partial (unverified) | Medium | R2 | See ACC-019. | None beyond ACC-019. | TV launch screen |
| DIS-143 | Kids profiles | See ACC-018, which owns this feature. | Plex yes (managed accounts); Jellyfin no ("child mode" 77 votes, user groups 202); Emby no (unverified) | Medium: 202 and 77 votes | R2 | See ACC-018. | None beyond ACC-018. | Profile creation |
| DIS-144 | Limits enforced at sync | Nothing a kids profile may not see ever reaches the device, so nothing leaks through search, artwork, screensavers or OS rows | The rivals filter in the API and UI; Jellyfin #5415 found unauthenticated image and stream endpoints; Plex removes folder view under restrictions | High: the research calls kids mode security, not styling | R2 | Per-object authorisation (ADR 1, decision 6) is applied when the sync payload is built and when stream URLs are signed; tests replay restricted IDs as the kids profile Discovery-specific part of ACC-030; on shared devices see the limits stated there. | Filtered sync payload; signed-URL checks | None visible: the absence is the feature |
| DIS-145 | Restriction presets | See ACC-023, which owns this feature. | Plex yes; Jellyfin no (8 votes); Emby no (unverified) | Medium | R2 | See ACC-023. | None beyond ACC-023. | Profile settings |
| DIS-146 | Ratings by country | See ACC-027, which owns this feature. | Plex by country (unverified); Jellyfin many (12.0 fixed four); Emby yes | Medium | R2 | See ACC-027. | None beyond ACC-027. | Profile settings |
| DIS-147 | Allow and block tags | See ACC-028, which owns this feature. | Plex labels (custom rules need Pass); Jellyfin yes (allow-tags request 86 votes; a typo silently breaks the filter, #4136); Emby yes | Medium: 86 votes | R2 | See ACC-028. | None beyond ACC-028. | Profile settings |
| DIS-148 | Unrated items | See ACC-025, which owns this feature. | Jellyfin per media type; Emby yes; Plex undocumented | Low | R2 | See ACC-025. | None beyond ACC-025. | Profile settings |
| DIS-149 | One-title exceptions | See ACC-029, which owns this feature. | No rival (Jellyfin 7 and 26-vote requests); Netflix blocks single titles | Low | R2 | See ACC-029. | None beyond ACC-029. | Detail page (guardian); profile settings |
| DIS-150 | Explicit-content filter | See ACC-024, which owns this feature. | No rival found | Low: named in the research as a music-first gap | R2 | See ACC-024. | None beyond ACC-024. | Profile settings; explicit badge |
| DIS-151 | Viewing hours | See ACC-032, which owns this feature. | Jellyfin yes; Emby yes; Plex no (unverified) | Low | R2 | See ACC-032. | None beyond ACC-032. | Profile settings |
| DIS-152 | Kids home | A bigger, simpler, brighter home with no grown-up rows | Netflix Kids; no server (Jellyfin "child mode" 77 votes) | Medium: 77 votes | R2 | A separate default layout and theme for the kids profile type | None | Kids home |
| DIS-153 | Adult profiles locked | See ACC-020, which owns this feature. | Plex PIN (hide-PIN request 111 votes); Jellyfin no (62 votes); Emby yes (unverified) | Medium: 62 and 111 votes | R2 | See ACC-020. | None beyond ACC-020. | Profile picker; PIN pad |
| DIS-154 | Kids still discover | See ACC-031, which owns this feature. | Plex managed users lacked Discover (104 posts); Jellyfin yes | Low | R2 | See ACC-031. | None beyond ACC-031. | Kids home |
| DIS-155 | Clean shared screens | Screensavers, OS rows and the living-room home never show restricted artwork | Jellyfin yes for home; a screensaver rating filter was declined | Low | R2 | The same sync filter; the screensaver draws only from the active profile's items | None | Screensaver; OS rows |
| DIS-156 | Guardian activity view | See ACC-034, which owns this feature. | Plex dashboard (unverified); Jellyfin activity log (unverified) | Low | R2 | See ACC-034. | None beyond ACC-034. | Guardian: activity |
| DIS-157 | Daily screen time | See ACC-033, which owns this feature. | No rival (Jellyfin 29 votes) | Low: 29 votes | Later | See ACC-033. | None beyond ACC-033. | Profile settings |
| DIS-158 | Live TV limits for kids | See ACC-036, which owns this feature. | TiviMate parental controls; an Emby thread reports tag exclusions failing for live TV | Low | R3 | See ACC-036. | None beyond ACC-036. | Live TV |

### The TV interface

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| DIS-159 | Navigation one press away | See CLI-035 and CLI-037, which owns this feature. | Plex removed its left navigation, then restored it in an August 2026 preview; its Fire TV redesign turned about two clicks into about six; Jellyfin Android TV yes | High: Plex rollback vote 504 votes; Fire TV backlash | R2 | See CLI-037. | None beyond CLI-037. | TV left rail |
| DIS-160 | Focus memory | See CLI-036, which owns this feature. | Netflix (unverified); others unverified | Medium: hard to get right in React Native on TV | R2 | See CLI-036. | None beyond CLI-036. | Every TV screen |
| DIS-161 | Fast on cheap sticks | See CLI-038, which owns this feature. | Plex's 2026 Fire TV app lagged and took 30 seconds to start local playback; Jellyfin varies by client | High: the Fire TV redesign was called unusable | R2 | See CLI-038. | None beyond CLI-038. | TV home; grids |
| DIS-162 | Long-press on TV | See CLI-039, which owns this feature. | Plex yes; Jellyfin partial; Infuse context menus | Medium | R2 | See CLI-039. | None beyond CLI-039. | TV cards |
| DIS-163 | Letter picker | See CLI-040, which owns this feature; the letter picker on TV follows DIS-101. | Swiftfin (1.5); Infuse; Jellyfin (unverified) | Medium | R1 | See CLI-040. | None beyond CLI-040. | TV grids |
| DIS-164 | Backdrop follows focus | Full-screen art of the focused item, optionally blurred | Plex, Jellyfin and Emby yes (unverified); Infuse users asked for blurred art | Low | R2 | parity | Backdrop images | TV home |
| DIS-165 | Library screensaver | See CLI-042, which owns this feature. | Plex (unverified); Jellyfin OLED-friendly screensaver | Low | R2 | See CLI-042. | None beyond CLI-042. | TV screensaver |
| DIS-166 | Readable from the sofa | See CLI-043, which owns this feature. | Emby font size options; Jellyfin no (10 votes) | Low | R2 | See CLI-043. | None beyond CLI-043. | TV settings |
| DIS-167 | Music on the TV | See CLI-041, which owns this feature. | Spotify TV redesign (2023) and its dim mode; Symfonium Android TV interface | Medium | R2 | See CLI-041. | None beyond CLI-041. | TV music home; now playing |

### Operating-system and car surfaces

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| DIS-168 | Android TV launcher rows | See CLI-049, which owns this feature. | Jellyfin yes; Streamyfin | Medium | R2 | See CLI-049. | None beyond CLI-049. | Android TV launcher |
| DIS-169 | Apple TV Top Shelf | Resume from the Apple TV home screen | Infuse; Streamyfin (0.54.1) | Low | Later | Later; R2 if the App Store licence decision allows (open decision 3 in the feature map README). Same mechanism as DIS-168 | None | tvOS Top Shelf |
| DIS-170 | Phone widgets | See CLI-073, which owns this feature. | Plexamp Android widget; Symfonium resizable widgets; Apple Music iOS 27 widgets; Finamp request (19 votes) | Medium | Later | See CLI-073. | None beyond CLI-073. | Phone widgets |
| DIS-171 | Car browsing from your home | CarPlay and Android Auto show your home rows, with downloaded items marked | Plexamp free; Finamp beta; Symfonium advanced Android Auto | High: Finamp CarPlay request open from 2021 (57 votes); Android Automotive request 119 votes | R2 | The same row definitions become the car's browse tree, usable offline | None | CarPlay and Android Auto browse tree |
| DIS-172 | Voice intents | See CLI-074, which owns this feature. | Plexamp Siri; Finamp Siri commands (0.9.24-beta) | Medium: important in cars | Later | See CLI-074. | None beyond CLI-074. | OS voice |
| DIS-173 | Deep links | See CLI-034, which owns this feature. | Infuse TMDB deep links | Low | R1 | See CLI-034. | None beyond CLI-034. | Link handler |
| DIS-174 | System search | Gunmetal items appear in the phone's own search | Not covered in the research | Low | Later | Only items in the active profile's allowed set are exposed | None | OS search |
| DIS-175 | NFC tags | See CLI-075, which owns this feature. | Plexamp (free) | Low | Later | See CLI-075. | None beyond CLI-075. | Phone |

### Considered and declined

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| DIS-176 | Streaming catalogue and "where to watch" | Rows and search results for titles on streaming services | Plex Discover (can be switched off since a 1,000-vote request) | Low: many self-hosters actively do not want it | No | Gunmetal has no catalogue and no vendor cloud, and Plex's most-voted discovery complaint is about exactly this content | None | None |
| DIS-177 | Public social layer | Public profiles, reviews, discussion threads and friend activity feeds | Plex yes (reviews and public profiles 2024 to 2025, Lists and Discussions 2026) | Low: "Discussions is a nightmare" has 441 likes; privacy is the second reason people self-host (3,520 of 4,081 in the selfh.st survey) | No | Household features stay inside the server and are opt-in (DIS-078, DIS-079) | None | None |
| DIS-178 | Artwork chosen per person | Different artwork for each viewer | Netflix (unverified) | Low | No | Out of scope for a personal server | None | None |
| DIS-179 | Vertical clip feed | Swipe through short clips to find something | Netflix tested it on mobile (2025) | Low | No | Needs clips that a personal library does not have | None | None |
| DIS-180 | Trending across servers | "Popular on Gunmetal" drawn from other people's servers | Netflix Top 10 is global | Low | No | It would need a central service collecting viewing data, which ADR 1 rules out | None | None |
| DIS-181 | Voice-assistant skills run by the project | See CLI-131, which owns this feature. | Plex shut its Alexa skill on 2026-06-15; Plex Google Home request 2,461 votes; Emby Alexa (paid) | High: 2,461 votes | No | See CLI-131. | None beyond CLI-131. | None |
| DIS-182 | Suggestions inserted into your playlists | Recommended tracks mixed into a playlist you made | Spotify Smart Shuffle (Premium) | Low | No | A person's own lists are never edited by the recommender; suggestions stay in a separate, labelled lane (DIS-070) | None | None |

## Differentiators

These are the features in this area most likely to make someone switch.

1. **Search and browsing that answer instantly, offline, and forgive
   mistakes** (DIS-002, DIS-084 to DIS-087, DIS-100). Jellyfin's slowest
   problems live here: 1,179 votes for lazy loading, 256 for tag search, 46
   for typo tolerance, and repeated search regressions in 2025. Offline is
   Jellyfin's most-voted request (1,820), and no server among the rivals
   searches with no connection. Gunmetal gets this from decisions already
   made (the library synced to the device, and one core crate compiled into
   every client), so ranking is the same everywhere and needs no server call.
   The caveat is that the index must fit a cheap TV's memory, which has not
   been measured.

2. **You control your own record** (DIS-022, DIS-023, DIS-047, DIS-048,
   DIS-050, DIS-052 to DIS-056, DIS-058). Dismiss with undo and a Hidden page
   (Jellyfin's 1,725-vote request; Plex has no undo), history by date (830
   Jellyfin votes, 2,035 Spotify votes), removing a play (5,458), personal
   ratings (453) and an export of all of it in R1; resetting
   recommendations (5,871), hide and snooze, and a watchlist (1,294) in R2. Each is an event in the append-only
   log, so it syncs, works offline and survives a cache rebuild. These are
   the basics that send people from Jellyfin back to Plex.

3. **One free rule language** (DIS-003, DIS-119 to DIS-124, DIS-126,
   DIS-127, DIS-144). Home rows, smart playlists, smart collections,
   seasonal rows and kids limits all use the same rules. Smart playlists
   have 588 Jellyfin votes; home customisation has 15,697 Spotify votes;
   Plex needs a Plex Pass to publish a collection to home; and nobody
   schedules a December row without an external tool. With one engine,
   every new rule field improves all of these at once.

4. **Music discovery from your own library, free** (DIS-060, DIS-061,
   DIS-067 in R1; mixes, charts, rediscover and on this day in R2). Radio is
   what people leaving Spotify say they miss most, and Plexamp charges for
   track and album radio, mixes and its DJ. Gunmetal's radio runs offline
   from the synced library, never adds music you do not own, and says why
   each suggestion appeared. The honest limit: R1 radio uses only tags,
   credits and co-listening, so for a new or one-person library it is at
   best parity with Jellyfin's Instant Mix and behind Jellyfin 12.0
   (ListenBrainz), Navidrome (Last.fm, Deezer) and Plexamp (sonic analysis)
   until the similarity-data plugin (MUS-175, R2) or sonic analysis
   (Later) exists.

5. **A home that is only yours and stays where you put it** (DIS-001,
   DIS-007, DIS-013, DIS-015, DIS-159). Plex's 2025 to 2026 redesigns drew a
   1,000-vote request to remove its own content, a 504-vote rollback vote, a
   Fire TV app where two clicks became six, and pinned libraries that
   vanished. Gunmetal has no vendor rows to add, gives every row a stable
   ID, and tests where core controls sit.

6. **Kids profiles enforced at sync** (ACC-018, DIS-144, ACC-029, DIS-152,
   ACC-020; R2). The rivals filter in the interface and API, and Jellyfin's
   security review found image and stream endpoints that skipped checks.
   Gunmetal never sends a restricted item to a device only a child uses, so
   search, artwork and screensavers cannot leak it there; on a shared device
   the item is hidden rather than absent (ACC-030). It adds a real kids home (77
   Jellyfin votes) and one-title exceptions, which no rival offers.

## Deliberately not doing

- **Streaming-service content and "where to watch"** (DIS-176). Gunmetal
  has no catalogue. This is the content Plex users voted 1,000 times to
  switch off. A third party could write a plugin; the project will not.
- **A public social layer** (DIS-177). Public profiles, reviews,
  discussions and friend feeds were badly received on Plex, and privacy is
  the second most common reason people self-host. Household sharing stays
  inside the server and opt-in.
- **Per-person artwork** (DIS-178) and **a vertical clip feed** (DIS-179).
  Both depend on assets a personal library does not have.
- **Trending across servers** (DIS-180). It would need a central service
  that collects viewing data.
- **Voice-assistant skills run by the project** (DIS-181). Demand is real
  (2,461 votes on Plex), but these skills need a cloud endpoint, and Plex
  itself shut its Alexa skill in June 2026. OS voice intents and Home
  Assistant cover the need without a central service.
- **Recommendations that edit your playlists** (DIS-182). Suggestions only
  ever appear in a separate, labelled lane.
- **Not in the core, plugin only:** natural-language search (DIS-081),
  results from outside the library (DIS-097), online trailers and focus
  previews (DIS-082), and every recommendation source that calls a third
  party (DIS-076). Each needs an explicit network grant, as ADR 2 requires.
- **No paid tier for any discovery feature.** Custom rows, smart playlists,
  radio, mixes and published rows are free in the AGPL build.

## Dependencies and risks

- **Blocking: ADR 3, durable user state.** ADR 1 (decision 5) names watch
  history as the only irreplaceable data, but most rows in this map that
  write user data (playlists, loves, ratings, corrections, layouts, shares,
  identities, IDs, settings) need it to survive a cache rebuild. ADR 3 must
  be accepted before any server work that stores user data. It defines the
  two durable stores every map now names the same way: **the user log** and
  **the identity store** (see the [feature map README](README.md)).
- **User intent must not live only in the rebuildable cache.** ADR 1 treats
  SQLite as rebuildable and names watch history as the only irreplaceable
  data. This map also relies on loves, ratings, hides, dismissals, pins,
  layouts, rules, watchlists and collections surviving a rebuild. Without a
  new record, a cache rebuild could quietly erase them. See open decision 1.
- **The sync protocol.** Nearly every R1 feature assumes a delta sync of
  library metadata and the profile's log to each device. Its size and
  battery cost on phones, and its memory and storage cost on cheap TV
  sticks, are unmeasured. Apple TV may purge caches. A metadata-only sync
  for TVs may be needed, which would weaken the offline promise there.
- **Offline in the browser.** R1 ships a web client. Instant, offline
  browsing in a browser needs the page itself cached as well as the data,
  and browser storage can be cleared without warning.
- **Merging intent from offline devices.** Dismissals, hides, ratings and
  layout edits made offline on two devices must merge by a stated rule.
  Getting this wrong produces the "my pinned libraries vanished" complaints
  Plex gets.
- **Content identity in the scanner** (library map). DIS-024 and DIS-038
  need the scanner to recognise a replaced file as the same item, using
  MusicBrainz IDs and tags first, not file paths.
- **The music model** (music maps). Search by role, artist top tracks,
  genre rows and radio depend on multi-value credits, release types,
  MusicBrainz IDs and genre normalisation being right at scan time.
- **Profiles and per-object authorisation** (users and security map).
  Kids mode is security, not styling. The filtered sync payload, signed
  stream URLs and the profile gate must share one authorisation layer and
  its cross-user tests.
- **The plugin system** (ecosystem map). Twenty-odd Later features wait on
  plugins with explicit network grants. Third-party services also change
  under us: Trakt began requiring VIP for new API apps in August 2026, a
  TMDB change broke Jellyfin's images in 2026, and Seerr's password sign-in
  does not fit passkey accounts.
- **Native clients and TV focus** (clients map). R2's TV and OS features
  depend on React Native TV focus handling and large virtualised grids
  performing on the cheapest devices, which is exactly where Plex's Fire TV
  app failed.
- **Recommendation quality with little data.** A household of three
  produces weak co-listening signals, and shared profiles pollute them.
  Metadata similarity is predictable but bland, and depends on tags or on a
  metadata plugin the owner may decline.
- **Search across languages.** Typo tolerance, accent folding,
  transliteration and CJK tokenisation are each real projects; Jellyfin and
  Infuse show these bugs come back.
- **Rating data and explicit flags.** Country rating systems need a data
  source whose licence has not been checked, and which tags reliably mark
  explicit music is unverified.
- **Sonic analysis.** Decoding every track strains low-power servers, and
  the candidate library's GPL-3.0 licence needs checking against AGPL use.
- **Customisation sprawl versus good defaults.** Plex was punished both for
  removing customisation (2021) and for changing defaults (2025). Strong
  defaults and a few powerful controls (rules, layouts) beat many toggles.
- **Trade dress.** Hero cards, numbered rows and shortcut grids are common
  patterns, but ADR 2 forbids copying another product's look; every such
  surface needs Gunmetal's own design.
- **Scope against the gate.** This map holds 76 R1 features, each held to
  test-first, full coverage and zero surviving mutants. The plan will need
  to order them carefully (see open decision 2).

## Open decisions for the project owner

1. **Make all user intent irreplaceable data.** Should a new architecture
   record extend ADR 1's append-only, exportable log from watch history to
   every kind of user intent: loves, ratings, hides, dismissals, pins,
   layouts, rules, playlists, collections and watchlists?
   *Recommendation:* yes, and decide it before the R1 music model ships,
   because R1 already depends on it.

2. **Where R1 stops.** The map puts 76 discovery features in R1.
   *Recommendation:* treat home (DIS-001 to DIS-004, DIS-007, DIS-009,
   DIS-013, DIS-015), continue and dismiss (DIS-020 to DIS-023), loves,
   ratings and history (DIS-045 to DIS-047, DIS-050 to DIS-054), radio and
   more-like-this (DIS-060, DIS-061, DIS-067), search (DIS-083 to DIS-087),
   browse (DIS-100 to DIS-106) and the rule language (DIS-119 to DIS-121) as
   the core. *Resolved in the feature map README:* the deferrals listed
   here (DIS-005, DIS-040, DIS-066, DIS-068, DIS-069, DIS-072, DIS-090) moved
   to R2, along with hide and snooze (DIS-054, DIS-057, DIS-055, DIS-056),
   genre clean-up (DIS-108), rediscover and on this day (DIS-064, DIS-065),
   seasonal and pinned rows (DIS-123, DIS-126, DIS-127), DIS-014 and DIS-039.

3. **Up Next, merged or split by default** (R2). *Recommendation:* one
   merged row by default with a switch to split it, since a toggle answers
   both the 203-vote Jellyfin request and Plex's 36-vote complaint.

4. **How much each device syncs.** Full library everywhere, or metadata
   only on TVs, with artwork on demand? This decides how far the offline
   promise holds. *Recommendation:* full metadata on phones and desktops,
   metadata with an artwork cache budget on TVs, and publish the measured
   limits rather than promise more.

5. **Titles you do not own.** Watchlisting unowned titles, results from
   outside the library, upcoming episodes and new releases all need
   third-party data. *Recommendation:* plugin only, off by default, always
   labelled "not on this server", and never in a default row.

6. **Household visibility and social features.** Should "popular in this
   household" and "send to someone" exist, and what can a guardian or
   owner see of other people's history? Is private listening on or off by
   default? *Recommendation:* the social features are Later and opt-in per
   person; guardians see a kids profile's history; no adult's history is
   shown to anyone else in the product; private sessions are off by default
   but one tap away and clearly shown while active.

7. **Kids profile model and rating data.** Is a kids profile its own type,
   which rating systems ship first, and from which data source? The
   licence of rating data is unchecked, which is a legal exposure.
   *Recommendation:* a separate profile type built on the rule language;
   ship only rating systems from a source whose licence has been checked,
   start with a few countries, and add more by contribution.

8. **Explicit music in kids profiles.** Most files carry no explicit flag
   at all (which tags do is unverified). *Recommendation:* allow unflagged
   music by default, let guardians block by artist or album, and say
   plainly in the interface that unflagged does not mean clean.

9. **AI search and voice.** Natural-language search through a model
   provider, a local model, or not at all; and whether the project ever
   runs voice-assistant skills. *Recommendation:* nothing in the core; a
   plugin hook for natural-language search later; no project-run voice
   skills, with OS voice intents and Home Assistant as the alternatives.

10. **Music and video homes.** When video arrives in R2, is music a mode of
    one home or its own home? *Recommendation:* separate Music and Watch
    homes sharing one search, with an optional combined home, so neither
    medium crowds out the other.

11. **Audio-based similarity.** Whether to analyse audio at all, where it
    runs, and under which licence. *Recommendation:* Later, as an opt-in,
    low-priority server job, only after confirming that the chosen library's
    licence is compatible with AGPL; R1 radio uses tags and co-listening,
    and similarity data from ListenBrainz or Last.fm (MUS-175) arrives as a
    plugin in R2.

12. **Rating model.** Loves only, five stars, or both. *Recommendation:*
    both: loves as the one-tap control everywhere, stars available for
    people who carry ratings from years of use.

13. **Online trailers and previews.** They need third-party fetching and
    bandwidth, and autoplay is costly on weak TVs. *Recommendation:* local
    trailers and extras in R2 (DIS-073); online trailers as a plugin Later,
    with previews off by default.
