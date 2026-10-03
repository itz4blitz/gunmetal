# Music player user experience

## Scope

This file covers how the best music apps are laid out and why. It looks at
navigation and information architecture, the home screen, the persistent
now-playing bar, the full-screen player, queue management, search, artist and
album pages, library organisation, playlist creation, discovery, multi-device
control, the desktop, phone and TV layouts, and car modes. For each area it
records what users praise, what they hate, and which recent redesigns caused a
backlash. It was researched on 2026-10-02.

The products studied are Spotify (desktop, mobile, tablet and TV), Apple Music
(iOS 26 and iOS 27, Mac, Android beta, tvOS), Tidal, YouTube Music, Plexamp,
Symfonium, Roon (with Roon ARC), Doppler and Marvis Pro. Because the project
owner wants an interface inspired by Spotify but better, the Jellyfin music
experience (the web client and the Finamp app), Feishin and Navidrome were
also checked, as they are the open-source baseline Gunmetal will be compared
with.

**Which columns the tables use.** For music the relevant rivals are not Plex,
Jellyfin and Emby as video servers. The tables therefore compare **Spotify**
(the design reference the owner named), **Apple Music** (the other mass-market
reference, and the best on several details), **Plexamp** (the strongest
self-hosted music player) and **Jellyfin (web, Finamp)** (the open-source
self-hosted baseline: the stock web client and the leading third-party mobile
app). Tidal, YouTube Music, Symfonium, Roon, Doppler, Marvis Pro, Feishin and
Navidrome appear in the "Best in class" and "Notes" columns where they lead.
Emby's music experience was not researched for this file.

**How to read the cells.** "Premium" means a paid Spotify plan is needed.
"Pass" means a Plex Pass is needed. Anything taken from memory rather than a
source is marked "(unverified)". Vote counts come from public feature-request
boards and are dated where the snapshot is old.

**Limits of this research.** Web search and page fetching were available. The
shared web-search budget for this session ran out partway through, so later
evidence was gathered by fetching known primary pages directly: the Plex forum
and Symfonium forum through their public Discourse JSON, GitHub issue
reactions through the GitHub search API, Hacker News through its search API,
and Spotify's idea board through Internet Archive snapshots (the live board
refuses automated requests). Reddit could not be fetched at all, so Reddit
sentiment appears only where a news article reported it.

## Feature inventory

### Navigation and information architecture

| Feature | What the user gets | Spotify | Apple Music | Plexamp | Jellyfin (web, Finamp) | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|---|
| Phone tab bar | The main places are one tap away from anywhere | Home, Search, Your Library and a Create button (exact set varies by plan; unverified) | Home, New, Radio, Library and a separate Search tab in a floating capsule bar on iOS 26 (tab names unverified) | Bottom navigation (layout unverified) | Finamp: tabs per item type plus a Home tab since 0.9.24-beta (June 2026) | Spotify: few tabs, each with one clear job | Critics say the iOS 26 search tab looks like an action button rather than a tab (Ryan Wesley). |
| Desktop three-pane layout | Library, content and the now-playing or queue panel visible at once | Library sidebar on the left, content in the centre, Now Playing View, Queue or Friend Activity on the right, player bar along the bottom | Sidebar plus content; MiniPlayer is a separate window (unverified) | Desktop app mirrors the mobile layout; a persistent top menu and bottom navigation was requested (71 votes, marked "partially done") | Jellyfin web has no persistent queue pane (unverified); Feishin copies Spotify's three panes | Spotify: it set the pattern that Feishin and others copy | In March 2024 Spotify moved the queue into the narrow right sidebar and removed the full-screen queue; users objected (Windows Latest). |
| Resizable and collapsible panes | Users decide how much room the library and queue get | Yes, the library sidebar resizes and collapses (unverified for the right panel) | Partial (unverified) | Unknown | No (unverified) | Spotify | A fixed-width queue panel was the core of the 2024 complaint. |
| Tablet layout | The extra width is used rather than a stretched phone | New tablet app in April 2026: layout reconfigures between portrait and landscape, collapsible side panel for browsing while something plays | iPad app with sidebar (unverified) | Unknown | Finamp is a phone layout on tablets (unverified) | Spotify, since April 2026 | Spotify describes the tablet app as reconfiguring rather than resizing. |
| Consistent context menu | The same actions (play next, add to playlist, go to artist) everywhere | Yes, three-dot menu and right-click everywhere | Yes, long-press menus | Yes (unverified) | Yes | Spotify | Consistency is what makes a large app feel small. |
| Long-press preview | A quick look at an item without leaving the screen | Removed; "Restore Touch Preview on iOS App" has 4,701 votes (Not Right Now) | Context menu with preview (unverified) | Unknown | No | Apple (unverified) | A loved feature that was taken away. |
| Swipe actions on rows | Fast queueing and removal | Swipe right to add to queue, swipe left to remove from queue | Swipe actions in lists (unverified) | Unknown | Finamp swipe to queue (unverified) | Spotify | Powerful but hidden; needs a visible alternative. |
| Configurable gestures | Users map taps to the actions they use most | No | No | No | No | Marvis Pro: tap, long tap, double tap and triple tap are all configurable | Only third-party clients go this far. |
| Desktop keyboard shortcuts | Control without the mouse | Yes (unverified scope); a hotkey to queue tracks had 988 votes in 2022 | Yes on Mac (unverified) | Unknown | Finamp desktop: Space and Ctrl+N/P since 0.9.23-beta | Apple on Mac (unverified) | Power users ask for queue hotkeys specifically. |
| Back and forward history | Return to exactly where you were | Back and forward buttons on desktop | Yes | Yes (unverified) | Browser history on web | Spotify | Scroll position should be restored too. |
| Settings search | Find a setting by name | Unknown | Uses the system Settings app | Unknown | No | None found | Symfonium users asked for a settings search bar (13 posts); a sign the app has many settings. |
| Share links | A link that opens the item in the app | Yes | Yes | Yes (unverified) | Partial | Spotify | For Gunmetal a link must not grant access by itself (record 1, decision 6). |

### Home screen

| Feature | What the user gets | Spotify | Apple Music | Plexamp | Jellyfin (web, Finamp) | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|---|
| Recently played shortcuts | One tap back to the last few things | Grid of recent items at the top of Home (unverified size) | Recently Played shelf | Recently played (unverified) | Recently played sections (unverified) | Spotify: the shortcut grid is the most-used part of its Home (unverified) | Should be first on any Home. |
| Personalised shelves | Mixes and suggestions built for the user | Many: Made For You, Daily Mixes, Daylist, Release Radar | Made for You, Top Picks | Mixes and stations from the user's own library (unverified detail) | Latest and recently played only (unverified) | Spotify for a catalogue; Plexamp for an owned library | Shelves full of podcasts and audiobooks are a top complaint (see pain points). |
| Content-type chips | Show only music | All, Music and Podcasts chips at the top (unverified exact labels) | Not needed: podcasts live in a separate app | Music only | Music libraries are separate | Apple: keeping podcasts out of the music app avoids the problem entirely | "Option to disable or hide podcasts entirely" has 8,815 votes (Under Consideration). |
| User-arranged Home | Choose, order and hide sections | No; "Customize Start Screen" has 15,697 votes and is marked Not Right Now | No section control; pins in Library on iOS 26 | Yes, but Pass only | Finamp: customisable sections since 0.9.24-beta | Marvis Pro: 30 section types, each with its own filters, sort and display style | The second most-voted Spotify idea of all time. |
| Pinned items | Favourites stay on top | Pins in Your Library (limit unverified) | Pins in Library since iOS 26 (up to six per a Tom's Guide summary; unverified) | Unknown | Favourites (unverified) | Tie | Pins are a cheap form of customisation. |
| New releases from followed artists | Never miss a release | Release Radar, Following feeds and an Upcoming Releases hub (2025) | New tab (unverified) | Recently added from the library (unverified) | Recently added | Spotify | An owned library only knows what has been added; a release watcher would need an external lookup. |
| Rediscovery shelves | Forgotten favourites, never-played albums | Limited (unverified) | Replay (unverified) | Library-based stations such as Time Travel radio (unverified) | No | Plexamp (unverified) | This is the natural discovery surface for an owned collection. |
| Speed dial | The handful of things you play daily, fixed in place | Shortcut grid | No | Unknown | No | YouTube Music was reported to be building a speed dial (unverified) | Same need as pins. |

### Persistent now-playing bar

| Feature | What the user gets | Spotify | Apple Music | Plexamp | Jellyfin (web, Finamp) | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|---|
| Bar on every screen | Playback is always visible and controllable | Yes | Yes; on iOS 26 it floats above the tab bar and merges into it when scrolling | Yes | Yes | Spotify: stable and quiet | NN/g says Apple's title "ticks along like a stock-market ticker" and the bubble jumps on scroll. |
| Skip from the bar | Next track without opening the player | Swipe the bar left or right (unverified buttons) | Play and next buttons (unverified) | Yes (unverified) | Yes (unverified) | Spotify | Tidal's 2026 redesign shipped a mini player with no skip buttons; its designer said a fix was coming. |
| Progress line | Position at a glance | Thin progress line (unverified) | Unknown | Unknown | Yes (unverified) | Spotify (unverified) | Small, but users notice when it goes. |
| Output device indicator | Know where the sound is coming from | Shows the Connect device (unverified detail) | AirPlay icon | Player picker (unverified) | No | Spotify | Essential once remote control exists. |
| Save from the bar | One tap to like the current track | Plus button | No (unverified) | Unknown | Finamp favourite (unverified) | Spotify | Spotify replaced its heart with a plus in 2023; "Bring back the heart button!" has 5,769 votes. |

### Full-screen player

| Feature | What the user gets | Spotify | Apple Music | Plexamp | Jellyfin (web, Finamp) | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|---|
| Artwork-led background | The player takes on the album's colours | Colour taken from the artwork | Colour plus animated album art | UltraBlur backgrounds and themes (free) | Finamp: colour scheme taken from the artwork | Plexamp: UltraBlur gives a strong identity without needing extra assets | Tidal's March 2026 redesign (large square art, dynamic colour) was praised for looks and criticised for function. |
| "Playing from" label | Know the context and jump back to it | Yes | Yes (unverified) | Yes (unverified) | Finamp shows the queue source with a link | Spotify | Tidal removed it in 2026 and users asked for it back. |
| Always-visible scrubber | See and change position without extra taps | Yes | Yes | Yes | Yes | Everyone except post-redesign Tidal | Tidal's 2026 player hides progress until the art is tapped, a common complaint. |
| Looping visuals | Motion behind the controls | Canvas looping videos supplied by artists | Animated artwork | Visualizers (free) | No | Plexamp for owned files: it needs no artist-supplied video | Gunmetal cannot rely on artist-supplied loops. |
| Info cards below the player | Lyrics preview, artist bio and credits by scrolling down | Yes, including the SongDNA card (beta from March 2026) | Lyrics and credits (unverified detail) | Artist bio; lyrics are Pass only | Partial | Spotify | Scrolling down from the player is now a standard pattern. |
| Credits | Who wrote, produced and played | Credits plus SongDNA (writers, producers, samples, covers) | Credits (unverified) | Unknown | Tag fields only (unverified) | Roon: cross-linked credits for performers, writers, producers and engineers | Owned files usually carry only artist and composer tags. |
| Format and quality badge | Know whether this is the original file | Lossless badge (unverified) | Lossless, Hi-Res and Dolby Atmos badges | Shows format (unverified) | Finamp shows transcoding status above the progress bar | Finamp: it says plainly when the server is transcoding | Fits Gunmetal's "play the original" promise. |
| Landscape layout | Artwork and controls side by side on a docked phone | No (unverified) | New in iOS 27, with lyrics beside the artwork | Unknown | Unknown | Apple (iOS 27) | 9to5Mac found it good for docked phones. |
| Swipe artwork to skip | Change track with a flick | Yes | Yes (unverified) | Yes (unverified) | Unknown | Spotify | Expected by now. |
| Sleep timer | Stop playback after a time or at the end of the track | Yes on mobile, inside the queue menu since May 2025; desktop timer had 1,025 votes in 2022 | Via the Clock app (unverified) | Yes (unverified) | Finamp: in the player menu | YouTube Music extended its timer to all media (PhoneArena) | Bedtime listening is common. |
| Lock-screen and OS controls | Control without opening the app | Yes | Yes | Yes; Windows artwork in the system panel fixed in 4.50 betas | Finamp yes; Jellyfin web works only partly with KDE controls (open issue) | Apple | Record 2 (decision 5) requires this. |
| Share the current track | Send it to a friend | Yes | Yes | Unknown | No | Spotify | For Gunmetal, sharing means sharing within the household or server. |

### Queue management

| Feature | What the user gets | Spotify | Apple Music | Plexamp | Jellyfin (web, Finamp) | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|---|
| See what plays next | No surprises | Yes | Yes | Yes | Yes | Spotify | YouTube Music's April 2026 redesign put the queue under the controls. |
| Play next versus play last | Choose where a track goes | "Add to queue" only; "Queue to Next or Last" had 1,635 votes in January 2025 (Not Right Now) | Play Next and Play Last | Play next and add to queue (unverified) | Finamp: New Queue, Play Next, Append Next and Play Last (0.9.20-beta) | Finamp: four verbs with clear, documented meaning | Apple's Play Next stacks newest first, so several "play next" picks play in reverse order (PiunikaWeb). |
| User picks kept separate from the source | Manual picks are not lost when the album continues | "Next in queue" is shown above "Next from" the source | Partial (unverified) | Unknown | Finamp: a Next Up section that shuffle and repeat do not touch | Finamp, then Spotify | Separating the two lanes is the single most useful queue idea found. |
| Reorder by dragging | Fix the order | Yes, Premium | Yes | Yes | Yes; Finamp disables dragging while shuffle is on (beta limitation) | Spotify | Reordering a shuffled queue should just work. |
| Remove by swiping | Quick clean-up | Yes | Yes | Yes | Yes | Spotify | — |
| Clear queue | Start again | Yes, with a confirmation on desktop | Yes (unverified) | Yes (unverified) | Yes (unverified) | Spotify | — |
| Queue control on the free tier | Basic control without paying | No: the queue is a Premium feature | Subscription only | Free | Free | Plexamp and Jellyfin | Gating the queue behind payment is a recurring grievance (unverified scale). |
| Suggestions shown before they play | Autoplay is visible and can be turned off | Since May 2025 Premium users see suggested tracks after their queue and can turn Autoplay and Smart Shuffle off | Autoplay toggle; users report unrelated tracks after an album ends (Apple Community) | Autoplay is Pass only | Finamp radio modes (0.9.21-beta); Jellyfin Instant Mix | Spotify, since 2025 | Hidden autoplay is a long-standing complaint. |
| Shuffle modes | Shuffle that feels random, or truly is | "Fewer Repeats" by default for Premium plus "Standard" true random (November 2025 engineering post) | Shuffle (modes unverified) | Shuffle (unverified) | Finamp "Reshuffle" radio mode | Spotify: two documented modes | Spotify's engineers say statistical randomness does not feel random to people. |
| Shuffle by album | Random albums, played in order | No; 1,073 votes in 2022 | No (unverified) | Unknown | Finamp "Album Mix" radio mode | Finamp | Album listeners want this. |
| Smart Shuffle | Recommendations mixed into a playlist | Yes, Premium | No | No | No | Not applicable | Must never be on by default; it changes a user's own playlist. |
| Queue history | See what just played | Recently played (unverified in the queue) | Scroll up for history (unverified) | Unknown | Finamp shows previous tracks above the current one | Finamp | — |
| Save the queue as a playlist | Keep a good session | No (unverified) | Unknown | Unknown | Jellyfin web save queue (unverified) | Unknown | Low-cost, often missing. |
| Queue survives a restart | Pick up where you left off | Yes | Yes | Yes | Finamp: queue restoration fixed in 0.9.23-beta | Spotify | — |
| Queue follows you between devices | Start on the phone, continue on the speaker | Yes, through Connect (the queue lives in the cloud) | Hand-off to HomePod (unverified) | Server-side play queues (unverified) | No; Finamp "Play On" with the web client is requested (18 votes) | Spotify | Record 2 (decision 4) makes this a Gunmetal goal. |
| Several saved queues | Keep an audiobook queue and a music queue | No | No | No | No | Symfonium: multiple media queues, each keeping its own position | Rare and valuable for mixed listening. |
| Play several playlists together | Combine two playlists in one session | No; 879 votes in 2022 | No | Unknown | Multi-select in playlists is limited (open Jellyfin web issue) | Unknown | — |
| Undo a queue edit | Recover from a slip | Not documented (unverified) | Not documented (unverified) | Unknown | Unknown | None found | A gap no one advertises. |

### Search

| Feature | What the user gets | Spotify | Apple Music | Plexamp | Jellyfin (web, Finamp) | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|---|
| Results as you type | Instant feedback | Yes | Yes | Yes | Yes | Spotify | Speed is the feature. |
| Result type chips | Narrow to songs, artists, albums or playlists | Yes | Yes | Yes (unverified) | Tabs (unverified) | Spotify | — |
| Recent searches | Repeat a search in one tap | Yes | Yes | Unknown | No (unverified) | Spotify | — |
| Library versus catalogue scope | Search only what you own | Separate search inside Your Library | Toggle between Apple Music and Your Library | Library only | Library only | Apple: the toggle is explicit | An owned library has only one scope, which is simpler. |
| Typo tolerance | Find "Beyonce" when typing "beyonse" | Yes (unverified) | Partial (unverified) | Unknown | Weak (unverified) | Spotify (unverified) | Local full-text search can do this cheaply. |
| Offline search | Search with no connection | Downloads only (unverified) | Downloads only (unverified) | Offline mode includes search (4.50, September 2026) | Finamp offline mode (unverified scope) | Plexamp 4.50 | Plexamp needed a rebuild to get this. |
| Search by lyrics | Find a song from a line you remember | Yes (unverified) | Yes (unverified) | No | No | Apple (unverified) | Requested in Symfonium (10 posts). |
| Browse by genre and mood | Explore without typing | Browse tiles | Categories | Genres, moods and styles from metadata (unverified) | Genres | Spotify | For owned files, genre tags are messy; needs normalising. |
| Voice search | Search hands-free | Yes (unverified) | Siri | Siri integration (free) | Finamp Siri commands (0.9.24-beta) | Apple | Important in cars. |
| Find inside a playlist or album | Jump to a track in a long list | Yes on desktop (unverified on mobile) | Unknown | Unknown | No (unverified) | Spotify | Plex "Playlist sorting and searching" has 183 votes. |
| Search by credit | Find everything a producer or composer worked on | No search, but SongDNA browsing | No (unverified) | No | No | Roon (cross-linked credits) | Needs a credit-aware data model. |

### Artist pages

| Feature | What the user gets | Spotify | Apple Music | Plexamp | Jellyfin (web, Finamp) | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|---|
| Header with play and shuffle | Start the artist in one tap | Yes | Yes; iOS 27 adds full-screen colour backgrounds and custom name fonts for some artists | Yes | Yes | Apple (iOS 27) | Custom fonts per artist rely on label-supplied assets. |
| Popular tracks | The obvious starting point | Top tracks by global plays | Top Songs | Popular tracks (source unverified) | No; Symfonium asks to use the server's top songs | Spotify | For an owned library, "your most played by this artist" is the honest equivalent. |
| Discography by release type | Albums, singles, compilations and appearances apart | Yes | Yes | Yes; manual release-type editing is requested (50 posts) | Partial | Spotify | Depends on release-type tags. |
| All songs by the artist | A flat list of every track | Missing; "See all Songs on Artist Page" had 1,566 votes in January 2025 | Yes (unverified) | Yes (unverified) | Yes | Owned-library apps | Trivial for a local library. |
| Sort and filter the discography | Oldest first, by type | Yes (unverified) | Unknown | Yes (unverified) | Finamp: sort and filter on artist screens (0.9.25-beta) | Unknown | — |
| Biography | Context about the artist | Yes | Yes | Yes, from Plex metadata | Yes, from server metadata | Roon: bios, reviews and photos | Needs an external metadata source. |
| Similar artists | Where to go next | "Fans also like" | Similar Artists | Sonically similar artists (Pass) | Similar (unverified) | Plexamp: similarity from audio analysis works on owned files | — |
| Artist radio | Endless music in the artist's style | Yes | Station | Artist radio; Mix builders are Pass only | Instant Mix | Plexamp | — |
| Tour dates | Know when they play nearby | Yes | Yes, on artist pages in iOS 27 | No | No | Spotify and Apple | Needs a third-party feed; a plugin for Gunmetal. |
| Several artists per track | Every credited artist gets their page | Yes, from a clean catalogue | Yes | Weak; "Better support for albums and tracks with multiple artists" has 520 votes | Navidrome implemented multi-artist support; Jellyfin (unverified) | Streaming apps | Owned files need multi-value tag parsing. |
| Composer view | Browse by composer, not performer | No | Apple Music Classical is a separate app | Requested (89 votes) | Composer tags shown (unverified) | Roon and Apple Music Classical | Classical listeners are vocal and underserved. |

### Album pages

| Feature | What the user gets | Spotify | Apple Music | Plexamp | Jellyfin (web, Finamp) | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|---|
| Tracks grouped by disc | Multi-disc sets read correctly | Yes | Yes | Yes | Jellyfin sometimes splits multi-disc albums into separate albums (open issue, 19 votes, 71 comments) | Doppler: a Merge Albums action fixes it by hand | Owned-library servers get this wrong most often. |
| Editions and versions | Deluxe, remaster and original kept together | Catalogue versions | Yes | Requested (93 votes) | Unknown | Roon: it separates original release date from version release date | Needs release groups, as record 2 (decision 4) plans. |
| Release details | Date, label, total length | Yes | Yes | Yes | Yes | Roon | — |
| Quality badge | The file's real format | Lossless (unverified) | Yes | Yes (unverified) | Yes (unverified) | Apple | — |
| Remove one track from a saved album | Keep the album, skip the interlude | No; 899 votes in 2022 | Yes, in the library (unverified) | Not applicable to files | Not applicable | Not applicable | For owned files the equivalent is "hide this track". |
| Fix the artwork | Find and set a better cover | Not applicable | Mac lets you edit your own files' art (unverified) | Edit poster | Edit images | Doppler: built-in artwork search | — |
| Merge split albums | Fix a bad tag without retagging files | Not applicable | Not applicable | Unknown | No | Doppler | — |
| Reviews and liner notes | Read about the record | No | Editorial notes | Reviews (unverified) | No | Roon | External data. |
| More by this artist | Keep going | Yes | Yes | Yes | Yes | Spotify | — |

### Library organisation

| Feature | What the user gets | Spotify | Apple Music | Plexamp | Jellyfin (web, Finamp) | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|---|
| Type filter chips | Show only playlists, albums or artists | Playlists, Albums, Artists, Podcasts, Downloaded (unverified exact set) | Editable category list (unverified) | Library sections | Tabs | Spotify | — |
| Sort options | Recent, alphabetical, date added | Recents, Recently added, Alphabetical, Creator (unverified) | Several (unverified) | Many (unverified) | Many; Jellyfin web once forgot filters (closed issue) | Marvis Pro and Symfonium | Sort choice must be remembered per view. |
| Sort albums by release year | Chronological library | Missing; 1,738 votes in January 2025 (Not Right Now) | Yes (unverified) | Yes | Yes | Owned-library apps | — |
| Grid, list and compact views | Dense or visual, by preference | Yes (unverified compact list) | Unknown | Yes | Finamp: list or grid chosen per tab (1.0.0-beta) | Marvis Pro: grid, list and Cover Flow | — |
| Alphabet scroll bar | Jump to "M" in a long list | Missing; 884 votes in 2022 | Index on iOS (unverified) | Yes (unverified) | Finamp fast scroller (unverified) | Apple (unverified) | Vital for large libraries. |
| Playlist folders | Group many playlists | Folders on desktop; creating them on mobile had 1,852 votes in January 2025 (status now unverified) | Folders on Mac; on iPhone from iOS 26 according to one summary (unverified) | No (unverified) | No | Spotify desktop | Folder images: 5,161 votes on Spotify. |
| Liked songs | A one-tap favourites list | Liked Songs, saved with the plus button since 2023 | Favourites | Likes and ratings (unverified) | Favourites; a bug loses song favourites during playback (open, 25 votes) | Spotify | — |
| Ratings | More than like or not | No | Favourite and "suggest less"; star ratings only on Mac (unverified) | Star ratings (unverified) | Favourites plus community rating | Roon (custom ratings) | Owned-library users often carry star ratings from years of use. |
| Filter liked songs by genre or mood | Turn favourites into moods | Genre filter on Liked Songs in some countries (2025) and Smart Filters by activity, mood or genre | No (unverified) | Smart playlists | Genre browsing | Spotify for ease; Marvis Pro for power | "Organize Liked Songs by genre" has 6,198 votes. |
| Rule-based views | Saved filters such as "unplayed jazz added this year" | Smart Filters only | Smart playlists on Mac only (unverified) | Smart playlists (free) | Navidrome smart playlists; Jellyfin needs a plugin | Marvis Pro Smart Rules: any number of grouped filters, sorts and limits | — |
| Downloaded filter | See what works offline | Yes | Yes | Dedicated offline mode (4.50) | Finamp offline mode | Plexamp 4.50 | — |
| User tags | Personal labels such as "wedding" or "gym" | Missing; "Tag Music" had 1,888 votes in January 2025 | No | Requested; "Tag support for robust music library organization" has 866 votes | Server tags (unverified) | Roon (tags) | — |
| Personal play counts | How often you played this | Missing; a personal song counter had 963 votes and a streams counter 901 votes in 2022 | Mac shows plays (unverified) | Stored (unverified) | Stored by the server (unverified) | Owned-library apps | Gunmetal already keeps an append-only listening log (record 1, decision 5). |
| Listening history by date | What did I play last March | Limited; "See what I listened to Days, Months, Years ago" had 2,035 votes in January 2025 | Replay (unverified) | History (unverified) | Finamp basic playback history | Last.fm and ListenBrainz scrobbling | — |
| Remove from history | Keep a guilty pleasure out of the record | Missing; 5,458 votes (Live Idea) | Unknown | Unknown | Unknown | None found | Needs a private-listening switch too. |
| Hide or snooze a track | Stop hearing a song without deleting it | Hide, and Snooze for 30 days (2025) | "Suggest less" (unverified) | Unknown | No | Spotify | Marvis Pro can auto-skip disliked songs. |
| Warning when a track disappears | Know when something is gone | Missing; 887 votes in 2022 | Greyed out | Not applicable | Not applicable | Not applicable | Files do not vanish from licences; deleted files still need reporting. |
| Library opens quickly | No spinner when browsing | Fast | Fast | Plexamp 4.50 aims at faster local browsing | Jellyfin 10.11 brought slow-loading regressions (39 votes on one issue) | Streaming apps | Gunmetal's synced library is meant to make this instant. |

### Playlists

| Feature | What the user gets | Spotify | Apple Music | Plexamp | Jellyfin (web, Finamp) | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|---|
| Create from anywhere | A new playlist in one step | Create button; AI Playlist is also reached from it | New Playlist in Library | Yes | Yes | Spotify | — |
| Add-to-playlist sheet | Pick a destination quickly | A sheet with search and "saved in" ticks (unverified detail) | List of playlists (unverified) | Yes | Yes | Spotify (unverified) | Adding one song to several playlists at once had 1,026 votes in 2022. |
| Duplicate warning | Avoid adding the same track twice | Yes | Yes (unverified) | Unknown | No (unverified) | Spotify | — |
| Sort and reorder | Shape the running order | Add, Edit and Sort buttons at the top of playlists (2025) | Yes | Yes | Finamp playlist editing and reordering (0.9.20-beta) | Spotify | Plex "Better Playlists" has 1,425 votes, the top music request found on that forum. |
| Custom cover | Make a playlist recognisable | Yes, Premium (2025) | Yes (unverified) | Custom posters (implemented) | Yes (unverified) | Roon added custom covers in 2.73 (September 2026) | — |
| Collaborative playlists | Friends add tracks | Yes | Yes (unverified) | No (unverified) | No | Spotify | Household sharing matters more for a home server. |
| Smart playlists | Playlists that maintain themselves | No (Smart Filters only) | Mac only (unverified) | Yes, free; smart playlists now stay downloaded through refreshes (4.50 betas) | Navidrome yes; Jellyfin by plugin | Plexamp | "Playlist presence" as a rule had 101 Plex votes. |
| Import from another service | Bring playlists over | Playlist transfer through TuneMyMusic | Import from Spotify, YouTube Music, Tidal, Deezer and Amazon Music (iOS 26) | No (unverified) | No | Apple | For Gunmetal, import means matching names to files the user owns. |
| Playlists from a prompt | Describe a mood and get a list | AI Playlist and Prompted Playlist | No (unverified) | Sonic Sage was removed in 2026 as not consistent or useful enough | No | Spotify | YouTube Music launched AI Playlist for Premium in February 2026. |
| Designed transitions | DJ-style blends between tracks | Mix: transitions such as fade and rise, set per playlist (2025) | AutoMix: beat-matched, avoids breaking albums; not over AirPlay in iOS 26 | Sweet Fades (free) | No | Apple AutoMix, improved again in iOS 27 and extended to Apple TV and HomePod | — |
| M3U import and export | Move playlists between programs | No | Mac export (unverified) | M3U import (unverified) | M3U support (unverified) | Owned-library servers | Gunmetal's roadmap already lists M3U. |
| Share a playlist | Send a list to someone | Yes | Yes | Unknown | No | Spotify | — |

### Discovery

| Feature | What the user gets | Spotify | Apple Music | Plexamp | Jellyfin (web, Finamp) | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|---|
| Weekly discovery list | New music each week | Discover Weekly, with genre filters since 2025 | New Music Mix (unverified) | Mixes from the user's library (unverified) | No | Spotify | Needs a catalogue; an owned library can only rediscover. |
| Radio from a seed | Endless music from one song or artist | Song and artist radio | Stations | Track and album radio (Pass) | Instant Mix; Finamp radio modes | Plexamp: built on sonic analysis of the user's own files | — |
| A DJ | A guided, changing session | AI DJ with voice and text requests (2025) | No | Guest DJ (Pass) inserts related tracks into the queue | No | Spotify for talk; Plexamp for owned music | — |
| Journey between two songs | A path from one sound to another | No | No | Sonic Adventure (Pass) | No | Plexamp: nobody else has it | — |
| Era and mood stations | Music from a year or a feeling | Daylist | Radio | Time Travel and library radio (unverified) | No | Spotify | — |
| What friends play | Social discovery | Friend Activity, Blend and Listening Activity in Messages (2026) | A Friends playlist updated every Thursday (iOS 27) | No | No | Spotify | — |
| Credit exploration | Follow writers and producers | SongDNA (2026) | No | No | No | Spotify SongDNA and Roon | — |
| Negative feedback | Teach the app what not to play | Hide, Snooze and excluding tracks from the taste profile (2025) | "Suggest less" (unverified) | Unknown | No | Spotify | "Reset Taste Profile / History" has 5,871 votes (Not Right Now). |
| Labels for AI-generated music | Avoid machine-made filler | Not shipped; "Mark / Disable AI Generated Songs" has 13,401 votes | Unknown | Not applicable | Not applicable | Not applicable: an owned library contains what the user put there | A structural advantage for Gunmetal. |

### Lyrics

| Feature | What the user gets | Spotify | Apple Music | Plexamp | Jellyfin (web, Finamp) | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|---|
| Time-synced lyrics | Words highlight in time | Yes | Yes, word by word | Pass only | Built into Jellyfin (from 10.9 or 10.10; version unverified); Finamp timed lyrics | Apple | — |
| Translation | Understand a foreign-language song | Unknown | Yes, between each lyric line (iOS 26) | No | No | Apple | — |
| Pronunciation | Sing along in a script you cannot read | No (unverified) | Yes (iOS 26) | No | No | Apple | — |
| Lyrics offline | Lyrics on a plane | Missing; 1,600 votes in January 2025 | Unknown | Unknown | Embedded tags and .lrc files travel with downloads (unverified) | Owned-library apps | Lyrics stored in files work offline by nature. |
| Always show lyrics | Open the player straight on lyrics | Unknown | Unknown | Requested (260 votes) | Unknown | None found | — |
| Karaoke | Sing with reduced vocals | No | Apple Music Sing; iPhone as a microphone on Apple TV (iOS 26) | No | No | Apple | — |
| Lyrics sources | Where the words come from | Licensed provider (unverified) | Apple | Licensed provider (unverified) | Embedded tags, .lrc files, or the LrcLib plugin | Jellyfin for owned files: no service needed when lyrics are in the files | Lookups are a third-party call; record 2 puts those in plugins. |

### Playback quality

| Feature | What the user gets | Spotify | Apple Music | Plexamp | Jellyfin (web, Finamp) | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|---|
| Gapless playback | Live albums and mixes play without gaps | Yes | Yes | Yes (free) | Finamp yes; the web client depends on the browser | Plexamp | Required by record 2. |
| Crossfade | Smooth changes on playlists | Yes | Crossfade and AutoMix | Sweet Fades (free), which do not fade inside albums (unverified detail) | No (unverified) | Plexamp and Apple | YouTube Music still has no crossfade, per a third-party guide. |
| Loudness normalisation | No jumps in volume | Yes | Sound Check | Loudness leveling (free); on-device analysis when the server has not measured a track (4.50) | Finamp ReplayGain; server normalisation | Plexamp | Symfonium users ask for "loudness leveling like Plexamp". |
| Equaliser | Shape the sound | Yes (unverified) | System presets | Ten-band EQ (Pass) | No (unverified) | Symfonium (parametric EQ and AutoEQ) and Roon (MUSE) | — |
| Lossless and hi-res | The full-quality file | Lossless up to 24-bit/44.1 kHz for Premium, from September 2025 | Lossless, Hi-Res and Atmos | Plays the original file; sample-rate matching (Pass) | Original or transcoded | Roon and Symfonium (bit-perfect paths) | — |
| Exclusive output on desktop | Bit-perfect output to a DAC | Unknown | Unknown | WASAPI exclusive mode (broken in an early 4.50 beta) | No | Roon | — |
| Data-saving quality | Smaller streams on mobile data | Quality settings | Quality settings | Quality settings | Finamp transcoding; skipping transcoding on Wi-Fi is requested (20 votes) | Unknown | Record 2 plans Opus for mobile data. |

### Offline and downloads

| Feature | What the user gets | Spotify | Apple Music | Plexamp | Jellyfin (web, Finamp) | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|---|
| Download albums and playlists | Listen without a connection | Premium | Yes | Pass; limits removed in 4.50 | Finamp: albums, playlists, artists and single songs | Finamp | — |
| Download the whole library | Carry everything | No | Yes, for library items (unverified) | Requested for years (206 votes) and delivered in 4.50 | Finamp: yes | Finamp and Plexamp 4.50 | — |
| Rotating smart downloads | A fresh offline set without effort | Unknown | Unknown | "Keep Played Music" and overnight background refresh (4.50) | No | Roon ARC Smart Downloads: a weekly mix of recent favourites, heavy rotation and forgotten gems | — |
| Offline mode | The app is usable with no server | Offline switch (unverified) | Not applicable | Dedicated offline mode with Library, Home and search (4.50) | Finamp offline mode | Plexamp 4.50 | Feishin's most-voted issue is offline playback and sync (88 votes). |
| Smaller downloads | Save storage | Not applicable | Not applicable | Unknown | Finamp transcodes downloads on request | Finamp | — |
| Clear failure messages | Know why a download failed | Unknown | Unknown | Specific error messages added in 4.50 betas | Unknown | Plexamp 4.50 | Earlier Plexamp downloads could stall silently. |

### Multi-device control and listening together

| Feature | What the user gets | Spotify | Apple Music | Plexamp | Jellyfin (web, Finamp) | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|---|
| Device picker and remote control | Play on any device and control it from another | Spotify Connect: pick a device and the phone becomes the remote | AirPlay | Choose another Plexamp player, including headless ones | Jellyfin web can play on other sessions; Finamp cannot yet (18 votes) | Spotify Connect: the reference everyone else is measured against | — |
| Several speakers at once | The same music in every room | Missing; "Multiple Speakers / Devices simultaneously" has 8,800 votes (Not Right Now) | AirPlay 2 groups | A multiroom feature appears in 4.50 beta reports (unverified) | No | Roon: zones that can be grouped and moved | Plex "Tandem Playback to several clients" has 420 votes. |
| Phone as TV remote | Browse on the phone, play on the TV | Through Connect | AirPlay or Remote | Yes (unverified) | Partial | Spotify | — |
| Choose the output device on desktop | Pick speakers or a DAC | Requested: "Select default sound device for Spotify Connect" has 5,509 votes | System picker on Mac | Output selection | The browser decides | Plexamp and Roon | — |
| Casting to other systems | Sonos, Chromecast, DLNA | Connect speakers and Chromecast | AirPlay | Chromecast and AirPlay (free) | Finamp cast support requested (27 votes) | Symfonium: Sonos, Chromecast, DLNA/UPnP, Kodi and Plex players | — |
| Listening together | A shared queue | Jam: Premium hosts, guests join by QR code, link, Bluetooth or Wi-Fi; Request to Jam from Messages (2026) | SharePlay (unverified) | "Listen Together for Music" requested (76 votes) | Jellyfin SyncPlay is video-oriented (unverified) | Spotify Jam | Free users can join only in person. |
| Friends' activity | See what friends play | Friend Activity on desktop; opt-in Listening Activity in Messages (2026) | Friends playlist (iOS 27) | No | No | Spotify | Privacy controls matter. |
| Dedicated players | A box that just plays | Connect speakers | HomePod | Headless Plexamp on a Raspberry Pi (Pass) | No | Plexamp | Fits Gunmetal's low-hardware goal. |
| Watch apps | Control from the wrist | Yes (unverified) | Yes | Unknown | No | Symfonium (Wear OS sync for phone-free listening) | — |

### Desktop specifics

| Feature | What the user gets | Spotify | Apple Music | Plexamp | Jellyfin (web, Finamp) | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|---|
| Mini player window | A small always-visible player | Unknown (unverified) | MiniPlayer on Mac | Compact player (unverified) | Feishin mini player requested (32 votes) | Apple | — |
| OS media integration | Media keys and system panels work | Yes | Yes | MPRIS fixes on Linux; Windows taskbar and system panel artwork in 4.50 betas | Partial with KDE (open issue) | Apple | Roon 2.73 added Windows taskbar controls. |
| Drag and drop | Drag tracks into playlists and the queue | Yes | Yes | Yes (unverified) | No (unverified) | Spotify | Marvis Pro lets you drop songs on queue zones such as start, end or shuffle. |
| Multi-select | Act on many tracks at once | Yes | Yes | Yes | Limited in Jellyfin web playlists (open issue) | Spotify | — |
| Light theme | A light option for daytime use | Missing; "Light Mode option" has 7,031 votes (Not Right Now) | Follows the system | Themes | Themes | Apple | Dark-first does not have to mean dark-only. |
| Small, fast desktop app | Low memory, small download | Unknown | Native | Moved from Electron to Tauri in 4.50 for smaller downloads and lower memory | Web | Plexamp 4.50 | A direct signal for Gunmetal's desktop shell choice. |

### TV specifics

| Feature | What the user gets | Spotify | Apple Music | Plexamp | Jellyfin (web, Finamp) | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|---|
| Ten-foot home screen | Readable, remote-friendly browsing | Redesigned in November 2023 to match mobile and desktop, with recent items first | tvOS app (unverified detail) | Unknown | Jellyfin TV clients (music detail unverified); Symfonium has an Android TV interface | Spotify | Before 2023 Spotify's TV app favoured art over usability (How-To Geek). |
| Queue on TV | See several upcoming tracks | Yes since 2023; before that only one upcoming track was shown | Yes (unverified) | Unknown | Unknown | Spotify | — |
| Dim or ambient mode | Music on without a bright screen | Dark mode that dims to minimal information | Screensaver with lyrics (unverified) | Visualizers (unverified on TV) | No | Spotify | — |
| Profile switching | Each person gets their own music | Profile icon top right | tvOS users | Plex Home users (unverified) | Users | Spotify | — |
| Sign in from the phone | No typing on a TV keyboard | TV code (unverified) | Not applicable | plex.tv/link code (unverified) | Quick Connect | Jellyfin Quick Connect | Passkeys could do this for Gunmetal. |
| Karaoke and transitions on TV | Party features on the big screen | No | Sing with an iPhone as the microphone; AutoMix on Apple TV (iOS 27 cycle) | No | No | Apple | — |
| Play-screen stability | The TV player stays familiar | A February 2025 Android TV play-screen change drew complaints on Spotify's forum (detail unverified) | Unknown | Unknown | Unknown | Unknown | — |

### Car

| Feature | What the user gets | Spotify | Apple Music | Plexamp | Jellyfin (web, Finamp) | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|---|
| CarPlay | The library on the car screen | Yes | Yes | Yes (free) | Finamp: added in 0.9.24-beta, after a request open since 2021 (57 votes) | Apple | Marvis Pro has no CarPlay app; one reviewer called that a deal breaker. |
| Android Auto | The same on Android cars | Yes | Yes | Yes (free) | Finamp yes | Symfonium (described as advanced Android Auto support) | Car screens draw media apps from a browse tree of items, not the app's own layout. |
| Built-in car systems | A native app with no phone | Yes on some cars (unverified) | No (unverified) | Requested for Android Automotive (119 votes) | No | Spotify (unverified) | — |
| Offline in the car | Music with no signal | Downloads | Downloads | Offline access expanded in CarPlay and Android Auto (4.50) | Finamp offline | Plexamp 4.50 | Tunnels and rural roads make this essential. |
| Voice control | Ask for music while driving | Yes | Siri | Siri | Finamp Siri commands (0.9.24-beta) | Apple | — |
| Dedicated hardware | A dial and buttons for the car | Car Thing: stopped working on 9 December 2024; Spotify offered refunds after a lawsuit | Not applicable | Not applicable | Not applicable | None | A warning about cloud-dependent hardware; an SDK for it had 1,858 votes in January 2025. |

### Widgets and system integration

| Feature | What the user gets | Spotify | Apple Music | Plexamp | Jellyfin (web, Finamp) | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|---|
| Home-screen widgets | Play from the home screen | Yes (unverified) | Three extra-large widgets in iOS 27 | Android widget (implemented after a request) | Finamp widget requested (19 votes) | Symfonium: resizable Material You widgets | — |
| NFC tags | Tap a tag to start a playlist | No | Through Shortcuts (unverified) | Write to NFC tags (free) | No | Plexamp | Charming for a home setup. |
| Year in review | A personal summary | Wrapped; weekly listening stats (2025) | Replay (unverified) | Unknown | No | Spotify | Doppler builds yearly Listening Reports for owned music. |
| Not hijacking the system | The app launches only when wanted | Not an issue found | The Mac Music app launches on play keys or headphone connection; a tool to stop it reached 669 points on Hacker News (June 2026) | Not an issue found | Not applicable | Not applicable | Respecting the OS is part of the experience. |

### Personalisation and accessibility

| Feature | What the user gets | Spotify | Apple Music | Plexamp | Jellyfin (web, Finamp) | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|---|
| Themes and accent colours | The app looks the way the user likes | No | No | Themes | Finamp custom accent colours (0.9.20-beta) | Symfonium: users publish whole styles, including ones named "Spotie" and "Apple Music Theme" | Symfonium users rebuild the big apps' looks, which shows what they find familiar. |
| Alternative app icons | A personal icon | No; the May 2026 disco-ball icon was reverted within days after backlash | No | Unknown | No | Marvis Pro (more than 40 icons) | Users treat the icon as theirs. |
| Choose what the player shows | Hide or show details on the player | No | No | Unknown | Finamp: pick which information appears (0.9.25-beta) | Finamp | — |
| Motion and contrast | Calm, readable interface | System settings | Liquid Glass criticised for weaker contrast and distracting motion; an Apple Community request to turn it off has 118 "me too" votes | Unknown | Unknown | Unknown | NN/g's critique is directly about the Music app's mini player. |
| Screen-reader support | Full use without sight | Yes (unverified) | VoiceOver (unverified depth) | Unknown | Jellyfin web playback controls are not accessible to screen readers (open issue) | Apple (unverified) | — |

## Pain points and unmet demand

Each item below is something users complain about or keep asking for, with
the evidence found. Spotify idea counts come from an Internet Archive snapshot
of the "Top Ideas" board taken on 2026-07-08 unless another date is given.
Plex counts are votes on the Plex forum's feature-suggestion board, read on
2026-10-02. GitHub counts are thumbs-up reactions read on 2026-10-02.

1. **Users cannot shape their own home screen, and it fills up with things
   they did not ask for.** "Customize Start Screen" is Spotify's second
   most-voted idea with 15,697 votes and is marked Not Right Now
   ([idea](https://community.spotify.com/t5/Live-Ideas/All-Platforms-Customize-Start-Screen/idi-p/4528183)).
   Related ideas: "Option to disable or hide podcasts entirely" (8,815 votes,
   Under Consideration,
   [idea](https://community.spotify.com/t5/Live-Ideas/All-Platforms-Podcasts-Option-to-disable-or-hide-podcasts/idi-p/4812243))
   and "Remove or manage 'Your top podcasts' from mobile home screen" (5,223
   votes,
   [idea](https://community.spotify.com/t5/Live-Ideas/Mobile-Other-Remove-or-manage-Your-top-podcasts-from-mobile-home/idi-p/4768099)).
   Plexamp offers home customisation, but only with a Plex Pass
   ([plex.tv/plexamp](https://www.plex.tv/plexamp/)). Marvis Pro sells itself
   largely on a home screen built from 30 section types
   ([MacStories](https://www.macstories.net/reviews/marvis-review-the-ultra-customizable-apple-music-client/)).

2. **Queue behaviour is confusing, inconsistent and moved around.** Apple
   Music's Play Next puts each new pick in front of the last one, so several
   picks play in reverse order; users have complained for years
   ([PiunikaWeb](https://piunikaweb.com/2023/07/03/apple-music-queuing-system-play-next-or-play-last-remains-unfixed-in-ios-17/)).
   Spotify users asked for a choice between "next" and "last" (1,635 votes in
   January 2025,
   [idea](https://community.spotify.com/t5/Live-Ideas/Queue-Queue-to-Next-or-Last/idi-p/120484)).
   In March 2024 Spotify desktop squeezed the queue into a fixed side panel
   and dropped durations, album names and the full-screen queue; one quoted
   user said they want to see what *will* play
   ([Windows Latest](https://windowslatest.com/2024/03/17/spotify-on-windows-11-gets-jam-and-moves-queue-to-the-right-side)).
   YouTube Music's April 2026 redesign made the full-screen queue reachable
   only by a double swipe, which top commenters called a regression
   ([9to5Google](https://9to5google.com/2026/04/23/youtube-music-split-now-playing-redesign/)).
   Finamp cannot reorder the queue while shuffle is on
   ([Jellyfin forum](https://forum.jellyfin.org/t-announcing-finamp-s-redesign-beta)).
   Apple Music users report unrelated tracks starting after an album ends
   ([Apple Community](https://discussions.apple.com/thread/254931720), counts
   not read).

3. **Redesigns move controls people rely on, and users fight back.** Tidal's
   2026 player redesign removed the "Playing from" label, hid the progress bar
   behind a tap, moved lyrics to the top and left the mini player without skip
   buttons; Android users asked for a revert and the designer promised fixes
   ([PiunikaWeb, May 2026](https://piunikaweb.com/2026/05/06/tidal-ios-music-player-redesign-rolling-out/)).
   Spotify replaced its heart with a plus in 2023 and "Bring back the heart
   button!" still has 5,769 votes
   ([idea](https://community.spotify.com/t5/Live-Ideas/Bring-back-the-heart-button/idi-p/5809156)).
   Spotify's May 2026 disco-ball app icon was reverted within days
   ([Variety](https://variety.com/2026/digital/news/spotify-reverts-iphone-app-icon-original-disco-ball-1236779352/)).
   Spotify's 2023 feed-style home redesign drew a 78-comment Hacker News
   thread ([HN](https://news.ycombinator.com/item?id=35122439)). On the video
   side, Plex's "Vote to roll back to Plex Classic!" has 504 votes since
   September 2025
   ([Plex forum](https://forums.plex.tv/t/vote-to-roll-back-to-plex-classic/931767)).
   Apple's iOS 26 Liquid Glass look was criticised for readability and
   motion by NN/g, and an Apple Community request to turn it off has 118
   "me too" votes
   ([NN/g](https://www.nngroup.com/articles/liquid-glass/),
   [Apple Community](https://discussions.apple.com/thread/256239100)).

4. **Shuffle does not feel right, and people want both kinds.** Spotify's
   engineers wrote in November 2025 that true randomness feels patterned to
   people, so the default became a "Fewer Repeats" mode with the old true
   random kept as "Standard"
   ([Spotify Engineering](https://engineering.atspotify.com/2025/11/shuffle-making-random-feel-more-human)).
   Symfonium users asked for a truly random option (15 posts,
   [forum](https://support.symfonium.app/t/option-for-truly-random-shuffle-button/10486)).
   Spotify users asked for shuffle by album (1,073 votes in October 2022,
   [idea](https://community.spotify.com/t5/Live-Ideas/Music-Shuffle-by-Album/idi-p/18457)).
   Roon changed its shuffle in September 2026 to reduce repeated artists and
   albums ([Roon 2.73 notes](https://community.roonlabs.com/t/roon-2-73-and-arc-1-83-are-live/325206)).

5. **Self-hosted servers model owned music poorly.** On the Plex forum,
   "Better Playlists" has 1,425 votes
   ([link](https://forums.plex.tv/t/better-playlists/73590)), "Tag support for
   ROBUST music library organization" 866
   ([link](https://forums.plex.tv/t/tag-support-for-robust-music-library-organization/106326)),
   "Better support for albums and tracks with multiple artists" 520
   ([link](https://forums.plex.tv/t/better-support-for-albums-and-tracks-with-multiple-artists/116658)),
   "CUE support for FLAC files" 507
   ([link](https://forums.plex.tv/t/cue-support-for-flac-files/96352)),
   "Playlist sorting and searching" 183
   ([link](https://forums.plex.tv/t/playlist-sorting-and-searching-feature-request/116089)),
   "Better support for multiple versions of Music Albums" 93
   ([link](https://forums.plex.tv/t/better-support-for-multiple-versions-of-music-albums/126609))
   and "By Composer" 89
   ([link](https://forums.plex.tv/t/by-composer/27760)). Jellyfin still
   splits some multi-disc albums into separate albums (open, 19 votes, 71
   comments, [issue](https://github.com/jellyfin/jellyfin/issues/5605)) and
   loses song favourites during playback (open, 25 votes,
   [issue](https://github.com/jellyfin/jellyfin/issues/14981)). Jellyfin
   10.11 briefly broke playing and shuffling albums in the web client
   ([issue](https://github.com/jellyfin/jellyfin-web/issues/7266)).

6. **Basic music features sit behind paywalls.** Plexamp keeps gapless,
   loudness leveling, Sweet Fades, smart playlists, car support and casting
   free, but lyrics, downloads, home customisation, the equaliser, autoplay,
   track and album radio, Guest DJ and sonic features need a Plex Pass
   ([plex.tv/plexamp](https://www.plex.tv/plexamp/)). Remote music streaming
   to Plexamp stayed free when Plex started charging for remote video in 2025
   ([Plex forum](https://forums.plex.tv/t/plexamp-and-the-plexpass-requirement-for-remote-streaming/916078)).
   Spotify's queue management is a Premium feature
   ([Spotify support](https://support.spotify.com/us/article/play-queue/)), and
   only Premium users can host a Jam
   ([Spotify support](https://support.spotify.com/us/article/jam/)).

7. **Offline is bolted on late.** "Download entire library or playlist in
   Plexamp" collected 206 votes from 2020 until Plexamp 4.50 finally removed
   download limits in September 2026
   ([request](https://forums.plex.tv/t/download-entire-library-or-playlist-in-plexamp/646153),
   [The Desk](https://thedesk.net/2026/09/plexamp-new-app-desktop-windows-mac-linux/)).
   A separate "Plexamp Seamless Downloads" request has 71 votes
   ([link](https://forums.plex.tv/t/feature-request-plexamp-seamless-downloads/718064)).
   Feishin's most-voted issue is offline playback and sync (88 votes,
   [issue](https://github.com/jeffvli/feishin/issues/47)). Spotify users asked
   for lyrics in offline mode (1,600 votes in January 2025,
   [idea](https://community.spotify.com/t5/Live-Ideas/Your-Library-Lyrics-in-offline-mode/idi-p/5539890)).

8. **Multi-room and device control are wanted everywhere.** Spotify's
   "Multiple Speakers / Devices simultaneously" has 8,800 votes
   ([idea](https://community.spotify.com/t5/Live-Ideas/Connect-Multiple-Speakers-Devices-simultaneously/idi-p/614088))
   and "Select default sound device for Spotify Connect" 5,509
   ([idea](https://community.spotify.com/t5/Live-Ideas/Desktop-Select-default-sound-device-for-Spotify-Connect/idi-p/5351)).
   Plex's "Tandem Playback to several clients" has 420 votes
   ([link](https://forums.plex.tv/t/tandem-playback-to-several-clients/38777)).
   Finamp users want to control and be controlled from the Jellyfin web
   client (18 votes, [issue](https://github.com/finamp-app/finamp/issues/616))
   and want casting (27 votes,
   [issue](https://github.com/finamp-app/finamp/issues/50)).

9. **Car support lags, and car hardware died.** Spotify's Car Thing stopped
   working on 9 December 2024, after which Spotify offered refunds following
   a lawsuit ([Wikipedia](https://en.wikipedia.org/wiki/Car_Thing)). Plexamp
   users want a native Android Automotive app (119 votes,
   [link](https://forums.plex.tv/t/adapt-plexamp-for-android-automotive-os-not-just-android-auto/649618)).
   Finamp's CarPlay request was open from 2021 (57 votes,
   [issue](https://github.com/finamp-app/finamp/issues/24)) until a beta
   added it in June 2026
   ([releases](https://github.com/finamp-app/finamp/releases)). Marvis Pro
   has no CarPlay app, which an App Store reviewer called a deal breaker
   ([App Store](https://apps.apple.com/us/app/marvis-pro/id1447768809)).

10. **Recommendations pollute listening, and users want control over their
    record.** "Mark / Disable AI Generated Songs" has 13,401 votes since
    January 2025
    ([idea](https://community.spotify.com/t5/Live-Ideas/Mark-Disable-AI-Generated-Songs/idi-p/6641329)).
    "Reset Taste Profile / History" has 5,871
    ([idea](https://community.spotify.com/t5/Live-Ideas/Profile-Reset-Taste-Profile-History/idi-p/1283748))
    and "Removing items from Recently Played history" 5,458
    ([idea](https://community.spotify.com/t5/Live-Ideas/Removing-items-from-Recently-Played-history/idi-p/4743815)).

11. **People want their own numbers.** Spotify users asked to see personal
    play counts ("Personal Song Counter", 963 votes, and "View how many times
    we streamed a specific Song/Artist/Album", 901 votes, both in October
    2022;
    [idea](https://community.spotify.com/t5/Live-Ideas/Your-Music-Personal-Song-Counter/idi-p/4214339),
    [idea](https://community.spotify.com/t5/Live-Ideas/All-Platforms-Your-Music-View-how-many-times-we-streamed-a/idi-p/4785771))
    and to see what they listened to days, months or years ago (2,035 votes
    in January 2025,
    [idea](https://community.spotify.com/t5/Live-Ideas/Discover-See-what-I-listened-to-Days-Months-Years-ago/idi-p/1344468)).

12. **Library sorting basics are missing in the biggest app.** Spotify users
    asked to sort albums by release year (1,738 votes in January 2025,
    [idea](https://community.spotify.com/t5/Live-Ideas/Your-Library-Order-sort-albums-by-release-date-year/idi-p/744893)),
    to see all songs on an artist page (1,566 votes,
    [idea](https://community.spotify.com/t5/Live-Ideas/Mobile-See-all-Songs-on-Artist-Page/idi-p/1204996)),
    to tag music (1,888 votes,
    [idea](https://community.spotify.com/t5/Live-Ideas/Music-Tag-Music/idi-p/57185)),
    to create playlist folders on mobile (1,852 votes,
    [idea](https://community.spotify.com/t5/Live-Ideas/Mobile-Create-amp-manage-Playlist-Folders/idi-p/51517))
    and for an alphabet scroll bar (884 votes in October 2022,
    [idea](https://community.spotify.com/t5/Live-Ideas/Mobile-Search-Alphabet-scroll-bar/idi-p/4696419)).

13. **Desktop users want power-user basics.** "Light Mode option" has 7,031
    votes
    ([idea](https://community.spotify.com/t5/Live-Ideas/All-Platforms-Light-Mode-option/idi-p/730341)).
    In October 2022, a desktop sleep timer had 1,025 votes
    ([idea](https://community.spotify.com/t5/Live-Ideas/Desktop-Other-Sleep-timer-for-desktop/idi-p/4889106))
    and a keyboard hotkey to queue tracks 988
    ([idea](https://community.spotify.com/t5/Live-Ideas/Desktop-Keyboard-Hotkey-to-queue-Track-s/idi-p/57441)).
    Feishin users want a mini player (32 votes,
    [issue](https://github.com/jeffvli/feishin/issues/1601)). Plexamp desktop
    users asked for persistent navigation (71 votes,
    [link](https://forums.plex.tv/t/please-give-plexamp-for-desktop-persistent-top-menu-bar-and-bottom-navigation-partially-done/631802)).

14. **Lyrics should be there when wanted.** Plex users asked for lyrics to be
    visible automatically when a song starts (260 votes,
    [link](https://forums.plex.tv/t/lyrics-always-visible-when-song-starts/134519)).
    Symfonium users asked to search the library by lyrics
    ([forum](https://support.symfonium.app/t/search-your-library-with-song-lyrics/11731)).

15. **Apps should behave on the operating system.** A small tool that stops
    the Mac Music app from launching on the play key or when headphones
    connect reached 669 points and 278 comments on Hacker News in June 2026
    ([HN](https://news.ycombinator.com/item?id=48447935),
    [tool](https://lowtechguys.com/musicdecoy/)). Jellyfin web works only
    partly with KDE Plasma media controls (open issue,
    [issue](https://github.com/jellyfin/jellyfin-web/issues/5208)).

16. **Deep customisation can turn into settings sprawl.** Symfonium is praised
    for letting users tweak the home screen and the album and artist displays
    ([XDA](https://www.xda-developers.com/single-music-app-plex-jellyfin-navidrome/)),
    but its users have asked for a search bar for the settings page
    ([forum](https://support.symfonium.app/t/please-add-a-search-bar-for-settings-page/1980)).
    The Finamp redesign discussion ran to 279 comments
    ([issue](https://github.com/finamp-app/finamp/issues/220)), showing how
    much users care about the default look.

## Where Gunmetal can be clearly better

Each idea below is tied to a pain point above and checked against the
architecture records. "What must be true" lists the technical conditions.

1. **A queue with three clearly separated lanes.** The queue screen shows
   "Up next" (the user's own picks), then "From" the album or playlist being
   played, then an optional "Continue with" lane of suggestions that is off by
   default and always visible before anything plays. "Play next" adds to the
   top of the user's lane in the order chosen, so picking three songs plays
   them in that order; "Add to queue" appends to the end of that lane. Shuffle
   only affects the source lane, and dragging works while shuffled. Undo is
   offered after every removal or clear. This answers pain point 2 and matches
   record 2 (decision 4: a play queue that syncs). *What must be true:* the
   queue is one versioned object owned by the server, edited through small
   operations that clients apply optimistically and the server orders; the
   queue rules live in the core crate as pure logic, so they get the same
   test and mutation coverage as the parsers and behave identically on every
   client (record 1, decision 2).

2. **Honest shuffle, in the modes people ask for.** Offer true random,
   "spread out" (no same artist or album close together, recently played
   tracks later) and shuffle by album. The queue shows the actual shuffled
   order, so nothing is hidden. This answers pain point 4. *What must be
   true:* a seeded shuffle in the core crate so a shuffled queue reproduces
   exactly on every device, and access to recent plays from the listening log
   (record 1, decision 5).

3. **A home screen the user builds, which also drives the car and TV.** Home is
   a list of sections, each a saved query over the user's own library ("added
   this month", "not played in a year", "most played this week", a pinned
   playlist). Users add, remove, rename and reorder sections; there are no
   ads, podcasts or upsells. The same section definitions become the rows on
   TV and the browse tree in CarPlay and Android Auto. This answers pain
   points 1 and 9 and costs nothing extra, unlike Plexamp's Pass-only
   customisation. *What must be true:* one query engine shared by Home,
   smart playlists and filters, evaluated on the device against the synced
   library, with section definitions stored as user data.

4. **Instant, offline browsing and search.** Because the library is synced to
   the device (README, record 1), every list, page and search answers locally
   with no spinner, and still works on a plane. Search is typo-tolerant, has
   type chips, and can search lyrics stored in the files. Plexamp needed a
   rebuild in 2026 to reach an offline mode; Gunmetal can start there. This
   answers pain points 7, 12 and 14. *What must be true:* a delta sync
   protocol for library metadata, a local full-text index on each client, an
   artwork cache with a size budget, and published numbers for how large a
   library the sync handles on a phone.

5. **A music model that respects owned collections.** Separate pages for every
   credited artist, discs grouped correctly, editions grouped under one
   release with an "Other versions" row, release types, composers and works,
   personal tags, star ratings and loves, and an "All songs" list on every
   artist page. This answers pain points 5 and 12. *What must be true:* the
   scan reads multi-value tags, disc numbers, release-type tags and
   MusicBrainz identifiers (record 2, decisions 3 and 4); user corrections
   such as merges and overrides must survive a rebuild (see Risks).

6. **Smart playlists and filters for everyone, free.** Rules can use any tag,
   play count, last played, date added, rating, format, and presence in
   another playlist (a Plex request with 101 votes). Smart playlists can be
   downloaded and refresh themselves. This answers pain points 5, 6 and 7.
   *What must be true:* the shared query engine from idea 3, plus rule
   evaluation that is cheap enough to rerun on every library change.

7. **No paywall inside the product.** Lyrics, downloads, the equaliser, radio,
   home customisation and remote listening are all in the AGPL build. This
   answers pain point 6 and follows from record 1 (decision 10).

8. **Your devices, controlled from any one of them.** A device picker shows
   every Gunmetal player the user owns (phone, desktop, TV, a headless
   box); picking one moves playback, position and queue there, and the phone
   becomes the remote. Desktop and headless players let the user choose the
   output device and keep that choice. Grouped multi-room playback comes later.
   This answers pain point 8. *What must be true:* a session registry on the
   server and a control channel that works at home and remotely over iroh
   (record 1, decision 7); clients that can be told to play from a given queue
   position; for multi-room, clock synchronisation between players, which is
   a separate, harder project.

9. **Lyrics that are simply there.** Read synced and plain lyrics from tags
   and sidecar .lrc files at scan time, sync them with the library so they
   work offline, and offer an "open on lyrics" setting and a full-screen
   lyrics view. Lookups from online lyric databases are a plugin with an
   explicit network grant (record 2, Consequences). This answers pain point
   14 and the offline-lyrics request in pain point 7.

10. **A quality badge that tells the truth.** The player says "Original FLAC,
    24-bit, 96 kHz, played directly" or "Opus 160 kbps, converted for mobile
    data". Finamp shows transcoding status today; Gunmetal can make it a
    first-class part of the player. This fits the README's promise to play
    the original. *What must be true:* the playback decision engine in the
    core reports its decision and reason to the UI.

11. **Personal statistics from the user's own log.** Play counts on every
    track, album and artist page, a history browsable by date, "on this day",
    and a yearly report available at any time. A private-listening switch
    keeps a session out of the record, and individual plays can be removed.
    This answers pain points 10 and 11. *What must be true:* the append-only
    listening log (record 1, decision 5) supports removal events and private
    sessions, and aggregates are computed on the device or server without
    any third-party service. Scrobbling to Last.fm or ListenBrainz stays a
    plugin (record 2).

12. **A layout contract instead of redesign churn.** Core controls (the
    persistent bar, queue access, lyrics, the device picker, the scrubber)
    keep their place across releases. Layout changes ship as an opt-in
    preview for at least one release, with a way back. This answers pain
    point 3. *What must be true:* layout and component positions are
    covered by visual regression tests in the UI codebase, and release notes
    show before and after screenshots.

13. **A desktop that rewards the keyboard.** A three-pane layout in which the
    queue can grow to full height and shows duration and album; a command
    palette for search and actions; hotkeys for play next, add to queue and
    like; a mini player window; media keys and system media panels on every
    OS. This answers pain point 13. *What must be true:* the desktop shell
    gives the React Native UI access to global shortcuts, a second window and
    the OS media APIs (MPRIS, Windows media controls, macOS Now Playing).
    Plexamp's move from Electron to Tauri for size and memory is a signal for
    this choice.

14. **A car mode that works with no signal.** CarPlay and Android Auto browse
    trees built from the user's Home sections, voice search, and automatic
    offline sets in the spirit of Roon ARC's Smart Downloads and Plexamp's
    "Keep Played Music". An Android Automotive build follows, since the same
    media service powers it. This answers pain point 9. *What must be true:*
    native media-library services on iOS and Android exposed to the React
    Native app, and a download scheduler that runs in the background.

15. **Discovery from the user's own collection, never injected.** Radio from
    any track, album or artist using tag similarity and the user's own
    co-listening history first, and audio analysis later if it earns its
    place. Nothing is ever added to a user's playlist automatically, and
    anything not chosen by the user is labelled. This answers pain point 10.
    *What must be true:* similarity data computed at scan time or from the
    listening log, and a clear "source" field on every queue item.

16. **A light theme from the first release, and calm motion.** Gunmetal stays
    dark-first (record 2, decision 7) but ships a light theme, honours
    reduce-motion and contrast settings, and avoids scrolling tickers and
    jumping bars. This answers the 7,031-vote light-mode request and the
    NN/g critique. *What must be true:* colour and motion defined as design
    tokens from the start.

## Risks and hard parts

- **One React Native codebase across four kinds of screen.** Record 1
  (decision 8) commits to one UI for phones, TVs, browsers and desktop. TV
  needs D-pad focus handling, desktop needs windows, shortcuts and dense
  layouts, and phones need gestures. Spotify built a separate tablet layout as
  recently as April 2026. Expect each form factor to need its own layouts
  over shared components.
- **Gapless and loudness on every platform.** The Jellyfin web client's
  gapless playback depends on the browser
  ([JellyWatch](https://jellywatch.app/blog/jellyfin-music-server-complete-guide-clients-scrobbling-2026)).
  Getting gapless, ReplayGain-style gain and smooth fades right in browsers,
  and in iOS and Android background playback, is real engineering, not a
  setting.
- **Queue sync conflicts.** Two devices editing one queue while one is
  offline will conflict. The merge rules must be designed and tested before
  the UI promises seamless hand-off.
- **Library sync at scale.** Syncing metadata and artwork for very large
  libraries to phones costs storage and battery. Limits and budgets need
  measuring, and partial sync may be needed.
- **User data versus a "rebuildable cache".** Record 1 (decision 5) treats
  SQLite as rebuildable and names watch history as the only irreplaceable
  data. Playlists, ratings, loves, personal tags, Home layouts, and manual
  merges or corrections are also user data that cannot be rebuilt from the
  files. This needs a decision record before the music model ships.
- **Messy tags and bare pages.** Owned files often lack artist photos, bios
  and clean release types. Without network lookups, which record 2 places in
  plugins, artist pages may look empty next to Spotify's. A first-party
  metadata plugin may be needed early, with its network grant explained.
- **Multi-room playback.** Keeping several players in sync to the
  millisecond needs clock synchronisation and buffering. It is a project of
  its own and should not block the device picker.
- **Car platform rules.** CarPlay and Android Auto render media apps from
  templates and browse trees, not the app's own screens
  ([Android developers](https://developer.android.com/training/cars/media)).
  CarPlay audio apps also need an entitlement from Apple (unverified current
  process).
- **App store distribution of an AGPL app.** Copyleft licences and app store
  terms have clashed before (unverified current policy). This needs checking
  before the iOS release is planned.
- **Being inspired by Spotify without copying it.** The owner wants a
  Spotify-like feel with its own identity, and record 2 (decision 7) forbids
  copying assets or trade dress. Gunmetal should not reuse Spotify's green
  and black scheme, its icon shapes, its round green play button, or product
  names such as Connect, Jam, Canvas or Daylist. The three-pane layout and a
  persistent player bar are common patterns; the risk is in the details.
- **Customisation sprawl.** Symfonium shows that deep customisation is loved
  by enthusiasts and can bury ordinary users in settings. Good defaults have
  to come first, with customisation layered on top.
- **Lyrics rights.** Displaying lyrics that a plugin fetches from a third
  party carries licensing questions (unverified); reading lyrics already in
  the user's files does not change who supplied them.
- **Redesign discipline.** Every product studied has broken a habit users
  relied on. Promising a stable layout (idea 12) creates a cost for every
  future design change.

## Open questions

1. Should the "Continue with" suggestions lane be off or on by default, and
   should the answer differ between an album and a playlist?
2. Which rating model should Gunmetal use: loves only, five stars, or both?
   Imported libraries often carry star ratings, and OpenSubsonic clients
   expect both stars and favourites (unverified).
3. Where do playlists, ratings, tags, Home layouts and manual metadata fixes
   live so that a database rebuild cannot lose them?
4. How far should customisation go in the first release: Plexamp-level (Home
   sections and themes) or Symfonium-level (nearly everything)?
5. Should podcasts and audiobooks ever appear inside the music interface, or
   live in a separate area, given how many Spotify users want them hidden?
6. Is any shared-listening feature (a household shared queue) in scope for
   the first version?
7. Which desktop shell hosts the React Native UI on Windows, macOS and
   Linux, given Plexamp's move to Tauri for size and memory?
8. Should multi-room playback be built in, or delegated to AirPlay,
   Chromecast groups and existing systems such as Snapcast at first?
9. Should a metadata plugin (artist images, bios, release types) be part of
   the first music release, so artist pages do not look bare?
10. Will Gunmetal ever integrate a streaming catalogue, as Roon and Plexamp do
    with Tidal or Qobuz, or stay with owned files only?
11. Are music videos in scope for the music player, given that Spotify and
    YouTube Music now mix them in?
12. What does "Home" mean on TV and in the car if a user has not customised
    it: what are the default sections?

## Sources

Spotify:
- https://newsroom.spotify.com/2025/12/29/year-in-features/
- https://newsroom.spotify.com/2026-04-16/new-tablet-app-experience/
- https://newsroom.spotify.com/2026-03-24/songdna-announcement-beta/
- https://newsroom.spotify.com/2026-05-21/studio-by-spotify-labs-launch/
- https://newsroom.spotify.com/2026-01-07/listening-activity-request-to-jam-messages-updates/
- https://engineering.atspotify.com/2025/11/shuffle-making-random-feel-more-human
- https://support.spotify.com/us/article/play-queue/
- https://support.spotify.com/us/article/spotify-connect/
- https://support.spotify.com/us/article/jam/
- https://techcrunch.com/2025/05/07/spotifys-latest-update-gives-users-more-control-over-their-listening-experience
- https://windowslatest.com/2024/03/17/spotify-on-windows-11-gets-jam-and-moves-queue-to-the-right-side
- https://www.howtogeek.com/spotifys-tv-app-isnt-a-mess-anymore/
- https://www.flatpanelshd.com/news.php?subaction=showfull&id=1699595432
- https://en.wikipedia.org/wiki/Car_Thing
- https://variety.com/2026/digital/news/spotify-reverts-iphone-app-icon-original-disco-ball-1236779352/
- https://news.designrush.com/spotify-new-anniversary-disco-ball-logo-sparks-backlash
- https://news.ycombinator.com/item?id=35122439
- https://soundstagesimplifi.com/index.php/feature-articles/293-spotify-lossless-is-here-at-last-was-it-worth-the-wait (search summary only, for the September 2025 lossless rollout)
- https://community.spotify.com/t5/Android/The-New-UI-for-the-Playing-Screen-Needs-Improvement/td-p/6732935 (search result title only)
- Spotify idea board snapshots: https://web.archive.org/web/20260708143207/https://community.spotify.com/t5/Live-Ideas/idb-p/ideas_live/tab/most-kudoed , https://web.archive.org/web/20250125104201/https://community.spotify.com/t5/Live-Ideas/idb-p/ideas_live/tab/most-kudoed/page/4 , https://web.archive.org/web/20221012105003/https://community.spotify.com/t5/Live-Ideas/idb-p/ideas_live/tab/most-kudoed/page/5 (individual idea links are given inline above)

Apple Music:
- https://9to5mac.com/2026/09/18/apple-music-in-ios-27-five-new-features-iphone/
- https://musictech.com/news/music/apple-music-ios-26-updates/
- https://www.hiresaudio.online/apple-music-ios26-new-features/
- https://www.tomsguide.com/phones/iphones/automix-gets-the-hype-in-ios-26s-music-app-but-theres-another-change-thats-even-better (search summary only, for the pin limit)
- https://gadgets.beebom.com/news/apple-music-7-0-beta-brings-liquid-glass-to-android
- https://www.nngroup.com/articles/liquid-glass/
- https://ryanwesley.com/ios-26-tab-bar-beef/
- https://discussions.apple.com/thread/256239100
- https://discussions.apple.com/thread/254931720 (search result only)
- https://piunikaweb.com/2023/07/03/apple-music-queuing-system-play-next-or-play-last-remains-unfixed-in-ios-17/
- https://news.ycombinator.com/item?id=48447935
- https://lowtechguys.com/musicdecoy/

Tidal and YouTube Music:
- https://piunikaweb.com/2026/03/18/tidal-ios-music-player-redesign-rolling-out/
- https://piunikaweb.com/2026/05/06/tidal-ios-music-player-redesign-rolling-out/
- https://9to5google.com/2026/04/23/youtube-music-split-now-playing-redesign/
- https://9to5google.com/2025/12/20/youtube-music-2025-now-playing-redesign/
- https://www.androidauthority.com/youtube-music-now-playing-redesign-split-screen-3657648/
- https://9to5google.com/2026/02/10/youtube-music-adding-ai-playlist-with-text-based-playlist-generation/
- https://www.phonearena.com/news/youtube-music-sleep-timer-let-you-asleep-to-fav-jams_id146792 (search summary only)
- https://www.audfree.com/youtube-music/crossfade-youtube-music.html (search summary only)

Plex and Plexamp:
- https://www.plex.tv/plexamp/
- https://www.plex.tv/blog/important-2025-plex-updates/
- https://thedesk.net/2026/09/plexamp-new-app-desktop-windows-mac-linux/
- https://forums.plex.tv/t/plexamp-release-notes/221280/last
- https://forums.plex.tv/t/plexamp-and-the-plexpass-requirement-for-remote-streaming/916078
- https://forums.plex.tv/t/better-playlists/73590
- https://forums.plex.tv/t/tag-support-for-robust-music-library-organization/106326
- https://forums.plex.tv/t/better-support-for-albums-and-tracks-with-multiple-artists/116658
- https://forums.plex.tv/t/cue-support-for-flac-files/96352
- https://forums.plex.tv/t/lyrics-always-visible-when-song-starts/134519
- https://forums.plex.tv/t/download-entire-library-or-playlist-in-plexamp/646153
- https://forums.plex.tv/t/feature-request-plexamp-seamless-downloads/718064
- https://forums.plex.tv/t/adapt-plexamp-for-android-automotive-os-not-just-android-auto/649618
- https://forums.plex.tv/t/listen-together-for-music/597875
- https://forums.plex.tv/t/please-give-plexamp-for-desktop-persistent-top-menu-bar-and-bottom-navigation-partially-done/631802
- https://forums.plex.tv/t/vote-to-roll-back-to-plex-classic/931767
- https://forums.plex.tv/t/better-support-for-multiple-versions-of-music-albums/126609
- https://forums.plex.tv/t/by-composer/27760
- https://forums.plex.tv/t/playlist-sorting-and-searching-feature-request/116089
- https://forums.plex.tv/t/tandem-playback-to-several-clients/38777
- https://forums.plex.tv/t/playlist-presence-filtering-criteria/209068
- https://forums.plex.tv/t/music-release-type-category-manual-edit/762258
- https://forums.plex.tv/t/beta-multiroom-on-plexamp-4-50-9-doesn-t-display-all-caldera-instances/942827
- https://forums.plex.tv/t/beta-plexamp-4-50-6-windows-app-exclusive-mode-and-wasapi-not-playing-tracks/942506

Jellyfin, Finamp, Feishin and Navidrome:
- https://forum.jellyfin.org/t-announcing-finamp-s-redesign-beta
- https://github.com/finamp-app/finamp/releases
- https://github.com/finamp-app/finamp/issues/24
- https://github.com/finamp-app/finamp/issues/50
- https://github.com/finamp-app/finamp/issues/220
- https://github.com/finamp-app/finamp/issues/616
- https://github.com/finamp-app/finamp/issues/156
- https://github.com/finamp-app/finamp/issues/44
- https://github.com/jellyfin/jellyfin/issues/5605
- https://github.com/jellyfin/jellyfin/issues/14981
- https://github.com/jellyfin/jellyfin/issues/15141
- https://github.com/jellyfin/jellyfin-web/issues/7266
- https://github.com/jellyfin/jellyfin-web/issues/5208
- https://github.com/jellyfin/jellyfin-web/issues/4504
- https://github.com/jellyfin/jellyfin-web/issues/3648
- https://github.com/jellyfin/jellyfin-web/issues/2644
- https://github.com/jeffvli/feishin/issues/47
- https://github.com/jeffvli/feishin/issues/1601
- https://github.com/navidrome/navidrome/issues/238
- https://github.com/navidrome/navidrome/issues/1417
- https://jellywatch.app/blog/jellyfin-music-server-complete-guide-clients-scrobbling-2026

Symfonium:
- https://symfonium.app/
- https://www.xda-developers.com/single-music-app-plex-jellyfin-navidrome/
- https://support.symfonium.app/t/option-for-truly-random-shuffle-button/10486
- https://support.symfonium.app/t/search-your-library-with-song-lyrics/11731
- https://support.symfonium.app/t/please-add-a-search-bar-for-settings-page/1980
- https://support.symfonium.app/t/loudness-leveling-like-plexamp-i-am-aware-of-replaygain-for-symfonium/11350
- https://support.symfonium.app/t/allow-using-the-server-implementation-to-get-artist-top-songs/11566
- https://support.symfonium.app/t/spotie-in-progress/13044
- https://support.symfonium.app/t/apple-music-theme/11164
- https://support.symfonium.app/t/desktop-app-of-symfonium/4190

Roon:
- https://community.roonlabs.com/t/roon-2-73-and-arc-1-83-are-live/325206
- https://blog.roonlabs.net/arc-adds-new-playback-features-and-smart-downloads/
- https://roon.app/en/
- https://roon.app/en/pricing
- https://community.roonlabs.com/t/struggling-to-find-roons-purpose/301625

Doppler and Marvis Pro:
- https://www.macstories.net/reviews/doppler-for-mac-offers-an-excellent-album-and-artist-focused-listening-experience-for-your-owned-music-collection/
- https://brushedtype.co/doppler/features/
- https://apps.apple.com/us/app/marvis-pro/id1447768809
- https://www.macstories.net/reviews/marvis-review-the-ultra-customizable-apple-music-client/

Car platforms:
- https://developer.android.com/training/cars/media
