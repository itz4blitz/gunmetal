# Clients, platforms and offline

Research date: 2026-10-02. Web access was available and used. The shared web
search budget ran out part way through, so later facts were checked by
fetching primary pages directly (release APIs, feature boards, forum JSON,
vendor pages). Anything that could not be checked against a source is marked
"(unverified)".

## Scope

This file covers what a viewer or listener holds in their hand or sees on
their screen: which platforms each rival has an app on and how good it is,
the ten-foot TV experience and remote-control navigation, phone, tablet,
desktop and web apps, casting (Chromecast, AirPlay, DLNA/UPnP), offline
downloads and sync (including automatic rules and transcoded downloads),
background audio and lock-screen controls, CarPlay and Android Auto,
watches, accessibility and localisation. It also records the paywalls that
decide which of those features a user actually gets.

Products studied: Plex (including Plex HTPC and Plexamp), Jellyfin (official
clients plus Swiftfin, Findroid, Streamyfin, Finamp and other third-party
apps such as Moonfin, Wholphin, Jellify and Plezy), Emby, Infuse and
Symfonium.

Out of scope here, and covered by sibling research files: server-side
transcoding, metadata, library modelling, live TV guides, sign-in design and
remote-access transport. They appear below only where they change what a
client can do.

Gunmetal's constraints come from the README and the two architecture records:
one React Native UI codebase with native player modules (libmpv) per platform,
a Rust core shared by server and clients, the library synced to the device,
an append-only watch log, no central account, remote access over iroh,
Jellyfin and OpenSubsonic compatibility as optional adapters, and music as a
first-class part of the first release.

## Feature inventory

Legend: "Yes" means the feature exists and a source confirms it. "No" means a
source confirms it is missing. "(unverified)" means I could not confirm it
from a source. Vote counts are from the source on the date above.

### Platform coverage

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Android phone and tablet app | Browse and play on Android handsets and tablets | Yes. Rewritten app since March 2025; video only, music moved to Plexamp | Official app is the web UI inside a native shell with a native Media3 player (2.7.x, Aug to Sep 2026). Native third-party apps: Findroid, Streamyfin, Finamp, Symfonium, Jellify | Yes. Full playback needs Premiere or an in-app unlock | Jellyfin ecosystem for breadth of choice; Symfonium for music | Plex's rewrite was heavily criticised at launch (see pain points) |
| iPhone and iPad app | Browse and play on iOS and iPadOS | Yes, same rewrite as Android | Official "Jellyfin for iOS" (web-UI based, unverified for the current build) plus Swiftfin (native SwiftUI, 1.6.1 in Aug 2026), Streamyfin, Finamp, Jellify | Yes. Premiere or unlock for full playback | Infuse for video playback quality (paid Pro for some formats); Swiftfin as the free native option | |
| Android TV and Google TV | Ten-foot app on Android-based TVs and boxes | Yes. Redesign in beta; top navigation rolled back to a left menu in Aug 2026 | Official app 0.19.x (0.19.10 in Aug 2026); third-party Wholphin, Moonfin, Streamyfin (sideloaded) | Yes. Free-tier limits apply (detail unverified) | No single winner; Jellyfin users have the most choice | |
| Fire TV on Fire OS | Ten-foot app on Android-based Fire TV | Yes. Redesign shipped 1 Apr 2026 with music but without photos | Android TV app through the Amazon Appstore | Yes | Plex for polish (contested by users) | |
| Fire TV on Vega OS | Amazon's newer non-Android Fire TV OS | (unverified) | Prototype client (May 2026) | (unverified) | None yet | Vega apps are built with a React Native variant, which matches Gunmetal's client stack |
| Apple TV | Ten-foot app on tvOS | Yes. Redesign in TestFlight preview; accessibility regression reported in Sept 2026 | Swiftfin for tvOS reached the App Store on 16 Jul 2026 (request #612, 463 votes); Streamyfin tvOS since Jun 2026; Moonfin; Neptune in TestFlight | Yes. Full playback free | Infuse: mature, broad format support, works with Plex, Emby and Jellyfin | Infuse puts Dolby Vision and Atmos passthrough behind Pro |
| Roku | Ten-foot app on Roku | Yes. Redesign in preview | Official app 3.1.9 (speed control, subtitle styling) | Yes. Full playback free | Plex (longest-standing, unverified as best) | Gunmetal plans to reach Roku through its Jellyfin adapter |
| Samsung Tizen TVs | Native TV app | Yes | Official app in the Samsung store for Tizen 6 and newer since May 2026, model-dependent; Tizen 5 must sideload | Yes | Plex and Emby (longer store presence) | Jellyfin request #335 (253 votes) took years |
| LG webOS TVs | Native TV app | Yes | Official app in the LG store (#42, 229 votes, completed); better Dolby Vision in MKV on webOS 25+ in 12.0 | Yes | Tie | |
| Hisense VIDAA TVs | Native TV app | Listed on Plex's devices page | None (#1615, 453 votes, open) | (unverified) | Plex | |
| Vizio TVs | Native TV app | Yes (remote paywall enforced on Vizio from Mar 2026) | None official (unverified) | (unverified) | Plex | |
| Philips Titan OS TVs | Native TV app | (unverified) | Client in testing (May 2026) | (unverified) | None yet | |
| Xbox | Console app | Yes | Official Xbox app (0.9.5 announced May 2026) | Yes. Full playback on consoles needs Premiere | Plex (unverified) | |
| PlayStation | Console app | PS4 and PS5 listed | None official (unverified) | PS4 and PS3 listed | Plex | |
| Windows desktop | Desktop app | Desktop app plus Plex HTPC | Jellyfin Desktop 2.0 (Qt 6 with mpv, Dec 2025); a Chromium Embedded Framework rewrite is in development builds | Windows app; free tier limited to one minute of playback | Plex HTPC for a TV-style desktop; Jellyfin Desktop for mpv playback | |
| macOS desktop | Desktop app | Desktop app plus Plex HTPC | Jellyfin Desktop | macOS app; one-minute free limit | Infuse for Mac (unverified as best) | |
| Linux desktop | Desktop app | Plex HTPC via Snap and Flathub | Jellyfin Desktop with MPRIS media controls | Linux app; one-minute free limit | Jellyfin | Plexamp's new Tauri desktop build includes Linux (Sept 2026) |
| Web browser | Play from any browser | app.plex.tv | Jellyfin web; "Modern" layout is the default in 12.0; supports the two latest versions of Firefox, Chrome, Safari and Edge | Web app; full playback free | Jellyfin: self-hosted with no vendor sign-in | |
| Kodi | Use Kodi as the front end | Third-party add-on (unverified) | JellyCon (official) and Jellyfin for Kodi; Media Segments and Quick Connect support added | Emby for Kodi listed | Jellyfin | |
| Apple Vision Pro and VR headsets | Watch in a headset | (unverified) | No (VR request #891, 167 votes) | (unverified) | Infuse has a native visionOS app | |
| Linux phones | App for postmarketOS-style phones | No (unverified) | No (#222, 20 votes) | No (unverified) | None | Low demand |
| Voice assistants | Ask a smart speaker to play | Alexa listed | None (#275, 243 votes) | Alexa skill; Premiere advertises Echo and Google Home control | Emby | |
| Sonos | Play to Sonos speakers | Listed on Plex's devices page | None official (unverified) | (unverified) | Symfonium: grouping and per-speaker volume | |
| Install outside app stores | Sideload, F-Droid, APK | (unverified) | F-Droid builds of official apps; Tizen 5 sideload; Streamyfin Android TV by APK | (unverified) | Jellyfin | Matters for de-Googled phones and older TVs |

### Ten-foot TV experience and remote navigation

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Stable primary navigation | The menu stays where muscle memory expects it | Moved the menu to the top in the TV redesign, then restored the left menu in Aug 2026 after months of complaints | Left-side navigation on Android TV (unverified) | (unverified) | None stands out; Plex's episode shows the cost of moving it | |
| Pinned favourite libraries | Jump straight to a library from home | Yes, added to the redesign in Aug 2026 | Home sections are configurable (unverified detail); "New or Improved Home Screen Sections" #1986 has 169 votes | (unverified) | Plex | |
| Predictable D-pad focus | Focus lands where expected, never gets lost | Plex's Aug 2026 update lists fixes for "focus issues" | (unverified) | (unverified) | Infuse on Apple TV (unverified, reputation only) | Focus bugs are the most common ten-foot complaint across apps (unverified as a measured claim) |
| TV sign-in without typing a password | Pair the TV from a phone | Code-based linking (unverified detail) | Quick Connect code; QR version #2642 (225 votes, planned); Android TV issue #5745 proposed QR, persistent Quick Connect and passkeys (closed Aug 2026, outcome unverified) | (unverified) | Jellyfin Quick Connect works without a vendor account | Gunmetal's passkey and device-key plan fits this well |
| Voice search on TV | Search by speaking into the remote | (unverified) | Native voice search in Android TV 0.19 that does not depend on the keyboard's microphone | (unverified) | Jellyfin Android TV | |
| "Are you still watching?" | Stops autoplay marking a whole series watched overnight | (unverified) | Android TV 0.19 (by time or episode count) and web 12.0; Streamyfin also | (unverified) | Jellyfin | |
| Skip intro and credits | One press to skip | Yes (Plex Pass requirement unverified) | Media Segments API used by Android TV, Findroid, Streamyfin, Kodi | Premiere feature | Jellyfin (free, many clients) | |
| Refresh-rate and resolution matching | Film plays at its native frame rate, no judder | Plex HTPC switches refresh rate; other Plex TV apps (unverified) | Android TV app has it, with a known side effect forcing PCM audio (#4067, 13 reactions); Wholphin switches rate and resolution | (unverified) | Plex HTPC and Wholphin by documentation; Apple TV relies on the OS "match content" setting (unverified) | |
| Accurate HDR and Dolby Vision detection | Direct play instead of a needless transcode | (unverified) | Improved in Android TV 0.19; HDR issue #49 open since 2019 with 127 comments; #5316 incomplete capabilities prevent direct play (11 reactions) | (unverified) | Infuse (Dolby Vision profiles 5 and 8 with Pro) | Capability reporting is where Gunmetal's own player helps most |
| Lossless audio passthrough | TrueHD, DTS-HD and Atmos reach the receiver untouched | (unverified) | Shield passthrough issue #281 open since 2020 with 50 comments | (unverified) | Infuse (Atmos via E-AC3 passthrough with Pro); Moonfin claims passthrough on Android TV | |
| Remote rewind and fast-forward buttons | Dedicated remote keys work | (unverified) | Added in 12.0 | (unverified) | Jellyfin | |
| Game controller navigation | Use a gamepad as a remote | Plex HTPC advertises controller support | Improved in 12.0 | (unverified) | Plex HTPC | |
| Trickplay thumbnails | Picture previews while scrubbing | (unverified) | Yes since 10.9 | (unverified) | Jellyfin | |
| Chapter names while seeking | Know where you are jumping to | (unverified) | Chapter name in the seek preview (12.0) | (unverified) | Jellyfin | |
| Letter picker | Jump to a letter in a long list | (unverified) | Swiftfin finished its letter picker in 1.5 | (unverified) | Swiftfin | Emby users ask for this in Android Auto too |
| Home-screen integration | Top Shelf on tvOS, recommendations row on Android TV | (unverified) | Streamyfin adds Top Shelf and Android TV recommendations (0.54.1) | (unverified) | Streamyfin | |
| Playback statistics overlay | "Stats for nerds": codec, bitrate, direct or transcode | (unverified) | Requested for Android TV (#272, 219 votes, planned); Android TV 0.19 added a media capability report for troubleshooting | (unverified) | (unverified) | |
| Profile picker on TV | Each household member gets their own state | Plex Home (unverified detail) | Multiple account switch #2353 (154 votes, started) | (unverified) | Plex (unverified) | |
| Subtitle size and position on TV | Readable subtitles from a sofa | (unverified) | Wide-aspect positioning PR #4816 (17 reactions); a Sept 2026 regression capped size at 32 (#5842) | (unverified) | (unverified) | Treated as an accessibility issue by users |
| Version picker | Choose 4K or 1080p, theatrical or extended | (unverified) | Multiple versions in Swiftfin; 12.0 groups multiple versions of episodes | (unverified) | Jellyfin | |

### Phone and tablet apps

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Native interface | Fast, platform-feeling UI | One rewritten codebase for all mobile platforms, planned to reach TVs | Official mobile apps wrap the web UI; native alternatives are third-party | Native (unverified) | Swiftfin, Findroid and Streamyfin for Jellyfin | |
| Music in the main app | One app for everything | Removed; music lives only in Plexamp | Yes, through the web UI | Yes | Emby and Jellyfin by availability; Plexamp by quality | |
| Playlist editing on the phone | Add to playlists without a computer | Cannot add items to playlists from the mobile app (Jan 2026); playlists grid-only with truncated titles | Yes (web UI) | (unverified) | Jellyfin | |
| Custom server address | Point the app at your own URL or reverse proxy | Removed from the new mobile apps | Required and supported | Yes (unverified) | Jellyfin | Plex now depends on its own connection broker |
| Automatic LAN/WAN switching | Fast local playback at home, remote elsewhere | Automatic via Plex connection discovery (unverified) | Streamyfin added local network auto-switch (0.54.1) | (unverified) | Plex (unverified) | |
| Several servers in one app | Browse friends' and your own servers together | Yes (unverified detail) | #47 (177 votes, open); Moonfin and Plezy aggregate servers | (unverified) | Infuse, Moonfin and Plezy (cross-server home and search) | |
| Picture-in-picture | Keep watching while using other apps | Reported broken or unreliable after the redesign | Official Android has it (#369 completed); Findroid; Streamyfin via KSPlayer | (unverified) | Findroid | |
| Player gestures | Swipe to seek, brightness, volume | (unverified) | Horizontal seek gestures in Android 2.7.0; Streamyfin gesture controls | (unverified) | Streamyfin | |
| Playback speed | 0.5x to 2x | (unverified) | Roku 3.1.9; Swiftfin remembers speed; Streamyfin | (unverified) | Swiftfin | |
| Chapter markers | See and jump to chapters | (unverified) | Streamyfin 0.54.1; Findroid | (unverified) | Findroid | |
| External display | Play on a cabled TV from the phone | (unverified) | Swiftfin wide-ratio external display; Android request #2060 | (unverified) | Swiftfin | |
| Admin tools on the phone | Watch sessions, restart scans, manage users | (unverified) | Swiftfin admin dashboard (1.3) and server backups (1.6); Streamyfin sessions view | (unverified) | Swiftfin | |
| Media requests | Ask for a film to be added (Seerr) | No | Streamyfin, Moonfin, Plezy integrate Seerr | No (unverified) | Streamyfin | |
| Reverse-proxy friendly auth | mTLS client certificates or custom headers | No (unverified) | Streamyfin custom headers; Finamp mTLS | (unverified) | Symfonium and Finamp (mTLS) | Common for people behind Cloudflare Access or Authelia |
| Settings sync across devices | Configure once, every device follows | Account-level (unverified) | Only through plugins (Streamyfin plugin, Moonfin plugin) | (unverified) | Moonfin (with its plugin) | |

### Desktop and web

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Native desktop player | Plays originals without a transcode | Plex desktop app and Plex HTPC (engine unverified) | Jellyfin Desktop 2.0 with mpv | Emby desktop apps; one-minute free limit | Jellyfin Desktop (mpv, free) | |
| Ten-foot mode on a PC | Couch UI on an HTPC | Plex HTPC with refresh-rate switching, controller support and multichannel audio | Jellyfin Desktop TV mode (unverified) | Emby Theater (unverified) | Plex HTPC | |
| OS media controls | Media keys, MPRIS on Linux, system now-playing | (unverified) | MPRIS in Jellyfin Desktop 2.0 | (unverified) | Jellyfin Desktop | |
| Keyboard shortcuts | Frame step, seek, subtitles without a mouse | (unverified) | Frame-by-frame with comma and full stop in 12.0; fixes for non-Latin keyboard layouts | (unverified) | Jellyfin | |
| Browser support policy | Know which browsers work | (unverified) | Two latest versions of Firefox, Firefox ESR, Chrome, Chrome for Android, Safari, Edge | (unverified) | Jellyfin (published) | |
| Offline in the browser | Download to the browser and play on a plane | No (unverified) | No; requests #3012 (45 votes) and #642 (7 votes) | Web cannot download, but can start downloads to other devices | None | |
| Desktop music app | Dedicated music player on a computer | Plexamp desktop rebuilt on Tauri for Windows x64 and ARM64, macOS and Linux (Sept 2026) | Finamp desktop (third-party) | (unverified) | Plexamp | |
| Desktop downloads | Offline video on a laptop | (unverified) | Jellyfin MPV Shim offers offline sync (third-party, unverified detail) | Emby for Windows and macOS (per Emby's docs) | Emby | |

### Casting and multi-device

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Chromecast from Android | Fling to a Chromecast or Google TV | Yes, but many threads reported casting broken after the 2025 rollout | Yes; session persistence fixes in Android 2.7.2 | Yes (Chromecast app listed) | Symfonium for music; no clear winner for video | |
| Chromecast from iPhone | Same, from iOS | Yes, with early "cast button does nothing" reports | Not in official iOS apps (#466, 104 votes, open since 2020); Swiftfin briefly had it in 2021 and removed it; Streamyfin has video casting | (unverified) | Plex | |
| Chromecast from the browser | Cast from Chrome | (unverified) | Yes | (unverified) | (unverified) | |
| Subtitles while casting | Subtitles appear on the cast device | External subtitles failing on Chromecast (thread with 22 posts) | Offset control requested (#1132, 11 votes); Streamyfin cast subtitles still in progress | (unverified) | (unverified) | |
| AirPlay | Send to Apple TV or AirPlay speakers | (unverified) | Web AirPlay since 10.5 (#132); music AirPlay on iOS requested (#3713) | (unverified) | Infuse (AirPlay and Google Cast, Pro); Plexamp (free) | |
| DLNA server | Old TVs and receivers browse the library | (unverified) | Moved out of core into a separate DLNA plugin (version unverified) | (unverified) | (unverified) | An unauthenticated LAN protocol; a security trade-off |
| Control UPnP and Sonos renderers | Use the phone as a remote for speakers | Sonos integration listed | No official | (unverified) | Symfonium: Chromecast with transcoding, UPnP with gapless, Sonos groups, Kodi, Plex players | |
| Remote-control another client | Drive the TV app from the phone | Cast to Plex players (unverified in new apps) | Web remote control of sessions; Swiftfin 1.5 added session commands | (unverified) | Jellyfin (any session can be controlled) | |
| Queue handoff between devices | Move what is playing from phone to speaker or TV | (unverified) | No (unverified) | (unverified) | None confirmed | An explicit Gunmetal goal in record 2 |
| Watch together | Synchronised viewing with friends | Watch Together removed in 2025 | SyncPlay; invite link requested (#971, 267 votes); watch with multiple accounts (#389, 164 votes) | (unverified) | Jellyfin SyncPlay; Plezy adds Watch Together for Plex | |
| LAN-direct casting URLs | Cast devices fetch from the server over the LAN, not the internet | (unverified) | Requested (#993, 5 votes) | (unverified) | Symfonium (proxy mode when renderers cannot reach the server) | |
| Cast controls in the notification | Stop casting from the shade | (unverified) | Disconnect button requested (#258, 10 votes) | (unverified) | (unverified) | |
| Casting with fallback transcode | Cast still works when the device cannot decode | Yes (unverified) | Device profiles (unverified detail) | (unverified) | Symfonium (transcodes for Chromecast when needed) | |

### Offline downloads and sync: video

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Download films and episodes | Watch with no connection | Plex Pass required; iOS and Android; movies and episodes only | Official Android downloads the original file only (#218, 1,820 votes, the most-voted request, "started"); Android 2.7.0 rewrote downloads; Findroid and Streamyfin download; Swiftfin cannot yet (#57, 145 upvotes, open since 2021) | Premiere; Android, iOS and Windows (Emby's feature matrix also lists macOS and Android TV) | Emby for depth of controls; Streamyfin for Jellyfin | |
| Whole season or series | One action for a season or show | Season downloads restored after the rewrite; whole-show remains a forum request | Batch download (#655, 219 votes, started) | Files, series, seasons or whole libraries | Emby | |
| Only unwatched episodes | Skip what you have seen | Re-added in 2025.18.0 for a season | No (unverified) | (unverified) | Plex | |
| Keep the next N unwatched episodes | Downloads refill themselves as you watch | Existed in the old app; missing in the new apps since 2025; still asked about in Aug 2026 | No | Auto sync rules (detail unverified) | Old Plex; Plezy advertises sync rules | The best-loved feature that Plex lost |
| Remove watched downloads automatically | Storage cleans itself | Requested (Aug 2026) | No | (unverified) | (unverified) | |
| Lower-quality downloads | Smaller files for phones and data caps | Global quality setting; transcodes when needed | Requested (#57, 518 votes, planned); Streamyfin does it by saving a server-transcoded HLS stream to a file | Pick resolution and bitrate; server converts first | Emby (server-managed conversion jobs) | Costly for low-power servers |
| Pre-transcoded versions | Ready-made mobile versions on the server | Optimised versions (unverified current status) | Pre-transcoding (#570, 979 votes, open) | Folder Sync creates alternate versions | Emby | |
| Downloads grouped by show | Find downloaded episodes easily | New app shows one long ungrouped list (Dec 2025 complaint) | (unverified) | Offline view drills down by category and title | Emby | |
| Offline browsing | The app is usable with no server | Downloads view (unverified detail) | Partial on Android | Separate Downloads view when offline; users call it inconsistent with the online UI | Infuse Library Mode caches metadata locally for offline browsing | |
| Offline watch-state sync | Progress made offline reaches the server later | (unverified) | Streamyfin syncs downloads with the server (0.30.2) | (unverified) | Streamyfin | Gunmetal's append-only watch log fits this |
| Background downloads | Downloads continue with the app closed | (unverified) | Streamyfin background download module (0.28, rebuilt 0.47.1); Jellyfin Android 2.7.x | Background sync required (per Emby's docs) | Infuse: background downloads on iOS 26+ and a Live Activity for progress | |
| Skip segments and trickplay in downloads | Offline playback keeps skip-intro and previews | (unverified) | Streamyfin (0.30.2); Findroid stores images locally | (unverified) | Streamyfin | |
| Bulk download management | Multi-select delete, progress percentages | No bulk delete (forum complaint) | Android 2.7.0 multi-select delete; iOS requests for percentage (#3955) and multi-select (#3957) | (unverified) | Jellyfin Android | |
| Admin control of downloads | Owner sees and manages every device's downloads | (unverified) | No | Yes; admins control all synced media | Emby | |
| Start a download from another device | Queue a phone download from the web | (unverified) | No | Yes | Emby | |
| Folder sync to disk | Mirror media to a drive or another folder | No (unverified) | No | Folder Sync (Premiere) | Emby | |
| Downloads cost money | Whether offline needs a subscription | Plex Pass | Free | Premiere | Jellyfin | |

### Offline downloads and sync: music

The relevant rivals here are the music players, not the video servers.

| Feature | What the user gets | Plexamp | Symfonium | Finamp | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Download albums, artists and playlists | Music with no signal | Yes, with Plex Pass | Yes, manual or automatic caching | Yes, free | Symfonium (rules) and Finamp (free) | |
| Download a whole library or very long playlists | Everything on the phone | A per-item time window blocked it for years (request with 206 votes since 2020); removed in the Sept 2026 release | Automatic rules (whole-library detail unverified) | (unverified) | Plexamp since Sept 2026 | Emby users say library-wide music sync is missing |
| Downloads follow the source | New albums by a downloaded artist arrive automatically | Complaint in Apr 2026; Sept 2026 release adds background refresh | Automatic caching rules | (unverified) | Symfonium | |
| Keep what I played | Recently played tracks stay offline within a storage cap | "Keep Played Music" (Sept 2026) | Playback cache | (unverified) | Plexamp | |
| Transcoded music downloads | Smaller files, often Opus | Yes (Opus downloads seen in Sept 2026 thread) | Transcoding used for Wear OS sync; phone downloads (unverified) | Yes, transcoded downloads | Finamp (free) | Record 2 says Opus transcodes are cheap on any hardware |
| Offline home, search and queue | A full player with no server | Dedicated offline library with home, search and local queues (Sept 2026) | Local database (offline-first design unverified) | Offline mode (unverified detail) | Plexamp | |
| Honour "no downloads on cellular" | Data caps respected | A Sept 2026 bug fetched over cellular while showing paused; co-founder confirmed and promised a fix | (unverified) | (unverified) | (unverified) | |
| Search within downloads | Find offline music quickly | Requested (Feb 2025) | (unverified) | (unverified) | (unverified) | |
| On-device loudness analysis | Consistent volume even for unanalysed tracks | Yes (Sept 2026) | ReplayGain from tags | ReplayGain from server | Plexamp | |
| Price of offline | Whether downloads need payment | Plex Pass | One-time purchase | Free | Finamp | |

### Background audio, lock screen and notification controls

| Feature | What the user gets | Plexamp | Symfonium | Finamp | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Background playback with lock-screen controls | Music keeps going with controls on the lock screen | Yes (unverified detail) | Yes (unverified detail) | Yes (unverified detail) | All three | Basic expectation |
| Background audio for video apps | A film's sound continues when the app is backgrounded | Plex (unverified) | n/a | n/a | (unverified) | Swiftfin 1.6.1 pauses video in the background; Jellyfin 12.0 fixed background audiobook playback on iOS |
| Handling interruptions | Calls and navigation prompts pause and resume cleanly | (unverified) | (unverified) | (unverified) | (unverified) | Jellyfin Android 2.7.2 added interruption handling |
| Home-screen widgets | Control without opening the app | (unverified) | Adaptive, resizable Material You widgets | (unverified) | Symfonium | |
| Voice control | "Play my running playlist" | Siri | (unverified) | Siri commands (0.9.24 beta) | Plexamp | Jellyfin Android 2.7.0 added voice control in Android Auto |
| NFC tags | Tap a tag to start a playlist | Yes | (unverified) | (unverified) | Plexamp | |
| Gapless and loudness | No gaps between tracks, even volume | Gapless, loudness levelling, Sweet Fades (free) | Gapless, ReplayGain, smart crossfades | Gapless, ReplayGain | Plexamp and Symfonium | Covered in more depth by the music research file |
| Several queues | Keep an audiobook queue separate from music | (unverified) | Multiple media queues | (unverified) | Symfonium | |
| Pre-caching for patchy signal | Next tracks fetched early | Advanced pre-caching (free) | Playback cache | (unverified) | Plexamp | |

### CarPlay, Android Auto and in-car

| Feature | What the user gets | Plexamp | Symfonium | Finamp | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Apple CarPlay | Browse and play from the car screen | Yes, free | Not applicable (Android only) | TestFlight beta since 0.9.24 (16 Jun 2026); store version lacks it | Plexamp | Emby CarPlay needs Premiere; official Jellyfin iOS has none (#744, 38 votes); Jellify, Manet and Finer Player offer it |
| Android Auto | Same on Android | Yes, free | "Advanced" Android Auto support | Beta (shuffle display fixed Aug 2026) | Symfonium | Jellyfin Android 2.7.0 redesigned Android Auto with audiobooks and voice; Emby needs Premiere |
| Offline browsing in the car | Downloads usable with no signal on the head unit | Home, Library, Charts and downloads offline (Sept 2026) | (unverified) | (unverified) | Plexamp | Some apps (CLAUDIO) show only downloaded content in CarPlay |
| Fast navigation of long lists | Alphabet jump in the car UI | (unverified) | (unverified) | (unverified) | (unverified) | Emby users ask for it |
| Android Automotive OS | Native app in cars with built-in Android | No (request with 119 votes since 2020) | (unverified) | (unverified) | None confirmed | Jellyfin request #3181 (4 votes) |
| Video while parked | Watch in a parked car | No | n/a | n/a | None | CarPlay does not allow video; Jellyfin request #3536 (18 votes) |
| Shuffle and repeat in the car | Playlist modes in the head unit | (unverified) | (unverified) | Fixed in Aug 2026 beta | (unverified) | Jellyfin request #1122 (12 votes) |
| Audiobooks in the car | Resume-friendly playback in the car | (unverified) | Audiobook queue and rewind-on-resume | (unverified) | Symfonium | Jellyfin Android 2.7.0 added audiobooks to Android Auto |

### Watches

| Feature | What the user gets | Plexamp | Symfonium | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Apple Watch app | Music on the wrist | None (request with 298 votes since 2017, still active Sept 2026) | Not applicable | Apple Watch support, Premiere only | Emby | In 2017 Plex said Apple's APIs blocked it; watchOS 6 added third-party streaming audio |
| Wear OS app | Music on Android watches | None found (unverified) | Yes: syncs music and playlists, with transcoding | (unverified) | Symfonium | |
| Offline sync to the watch | Run without the phone | No | Yes | (unverified) | Symfonium | The main reason users ask for watch apps |
| Phone-free streaming on LTE watches | Stream directly on the watch | No | (unverified) | (unverified) | None confirmed | Apple recommends starting at 64 kbps on watch streams |
| Remote control from the watch | Pause and skip from the wrist | Through the OS "Now Playing" app | Through the OS (unverified) | (unverified) | All, through the OS | |

### Accessibility

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Screen reader support on phones | Blind users can browse and play | VoiceOver and TalkBack fixes shipped after launch complaints in 2025; Plexamp reported unusable with VoiceOver (Aug and Nov 2025) | Web UI mostly navigable, but player controls are not accessible (#4504 open since Apr 2023; #442 open since 2019) | (unverified) | Infuse: VoiceOver support (8.0.1) and rotor actions (7.6.5) | |
| Screen reader on TV | VoiceOver and Hover Text on tvOS | New Apple TV app regressed: buttons with no or nonsense hover text (Sept 2026) | (unverified) | (unverified) | (unverified) | |
| Subtitle styling | Size, colour, background and position | New iOS app broke styling in 2025 | Long-standing request (#161, 188 votes); Swiftfin subtitle management; Streamyfin customisation; Roku 3.1.9 styling | (unverified) | Streamyfin and Jellyfin Roku (unverified ranking) | |
| Respect OS caption settings | System caption preferences carry over | New iOS app ignored iOS font settings (2025) | (unverified) | (unverified) | (unverified) | |
| Default subtitle behaviour | Right language, on only when wanted | (unverified) | Android TV issues with wrong language (#2883) and always-on subtitles (#4383); Swiftfin unified default track settings | (unverified) | (unverified) | |
| Dynamic type and font scaling | UI text follows system size | Ignored in places (2025) | Swiftfin improved dynamic type (1.3) | (unverified) | Swiftfin | |
| Reduced motion | Animations calm down when the OS asks | Not documented | Not documented | Not documented | None found | A gap nobody documents |
| High contrast and dark themes | Readable for low vision; OLED-friendly | (unverified) | Themes (unverified detail) | (unverified) | Finamp AMOLED theme (0.9.24) | No rival documents a high-contrast mode |
| Motor accessibility | Large targets, reliable gestures | Double-tap fullscreen and PiP problems called out as hard for limited mobility | (unverified) | (unverified) | (unverified) | |
| Forced burned-in subtitles | Subtitles on devices that cannot render them | (unverified) | Swiftfin 1.6 adds forced burn-in | (unverified) | Swiftfin | |
| Generated subtitles | Captions for files with none | No (unverified) | Whisper request (#2143, 61 votes) | (unverified) | None | Server feature; would run in the transcode sandbox |
| Keyboard-only use | Full use without a mouse | (unverified) | 12.0 keyboard and controller fixes | (unverified) | Jellyfin | |

### Localisation

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Number of UI languages | The app in your language | (unverified) | 90 languages on the main Weblate project; only 11 at 90% or more and 41 below 50% | (unverified) | (unverified) | |
| Per-client translation completeness | Every app translated, not just the web UI | (unverified) | Weblate totals: Xbox 98%, Jellycon 80%, Roku 77%, Android 73%, Vue 71%, Jellyfin 57%, Swiftfin 48% | (unverified) | Jellyfin for openness | |
| Translation platform | Community can contribute | (unverified) | Weblate (self-hosted); Streamyfin uses Crowdin | (unverified) | Jellyfin | Symfonium adds languages per release (Czech in 14.1) |
| Per-user language | Each user sees their own language and server messages | (unverified) | 12.0 lets clients ask for responses in a language; earlier request #370 (219 votes) | (unverified) | Jellyfin 12 | |
| Metadata language per user | Titles and overviews in each user's language | (unverified) | Request #610 (212 votes, open); "Keep original title" #32 (315 votes) | (unverified) | (unverified) | Overlaps the metadata research file |
| Right-to-left layouts | Arabic and Hebrew mirrored correctly | (unverified) | (unverified) | (unverified) | (unverified) | No rival documents RTL testing |

### Paywalls that touch clients

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Remote playback | Watch your server away from home | Plex Pass ($6.99 a month, $69.99 a year; lifetime $749.99 from 1 Jul 2026, up from $249.99) or Remote Watch Pass ($2.99 a month or $29.99 a year from 1 Jun 2026); enforced on LG, Samsung and Vizio from 23 Mar 2026, all TV platforms by end of 2026 | Free | Free (unverified) | Jellyfin | |
| Offline downloads | Watch or listen with no connection | Plex Pass (video app and Plexamp) | Free | Premiere ($4.99 a month, $54 a year, $119 lifetime, 30 devices) | Jellyfin | |
| Full playback on mobile | Play more than a preview | Free locally (unverified) | Free | Premiere or in-app unlock | Jellyfin | |
| CarPlay and Android Auto | Use it in the car | Free in Plexamp | Free where available | Premiere | Plexamp and Jellyfin | |
| Desktop playback | Use the desktop app | Free (unverified) | Free | One-minute limit without Premiere | Jellyfin | |
| Third-party player costs | Best player on a platform | n/a | n/a | n/a | Infuse Pro ($1.99 a month, $16.99 a year, $99.99 lifetime); Symfonium one-time; Neptune announced $1.99 to $129.99 | Users pay twice: once for a server licence, once for a good player |

### Third-party Jellyfin clients

The relevant rivals here are the three best-known native Jellyfin video clients.

| Feature | What the user gets | Swiftfin | Findroid | Streamyfin | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Platforms | Where it runs | iOS, iPadOS, tvOS (tvOS on the App Store from Jul 2026) | Android phones; Android TV planned | iOS, Android, tvOS (App Store, Jun 2026), Android TV by APK | Streamyfin | |
| Player engine | What decodes the file | Native player and VLCKit | ExoPlayer and mpv; mpv.conf editable (1.1.0) | MPV on iOS and Android (0.54.1); KSPlayer and VLC options | Findroid (mpv with user config) | |
| Downloads | Offline playback | Not yet; split into sub-tasks in 2025 and 2026 | Yes, original files only | Yes | Streamyfin | |
| Transcoded downloads | Smaller offline files | No | No | Yes, through the server's HLS transcode | Streamyfin | |
| Chromecast | Cast to TVs | No (removed in 2022) | Planned | Video yes; subtitles in progress | Streamyfin | |
| Music | Music playback | Mixed libraries supported (music player depth unverified) | Not mentioned | Beta since 0.51 (Jan 2026), though the README still says music is unsupported | None; users pick Finamp or Symfonium | |
| Picture-in-picture | Floating player | (unverified) | Yes | Yes with KSPlayer | Findroid | |
| Skip intro and credits | Media Segments support | (unverified) | Yes | Yes, including in downloads | Streamyfin | |
| Licence | Can it be forked and audited | MPL-2.0 | GPL-3.0 | (unverified) | All open source | |
| Server version floor | Which servers it supports | (unverified) | 1.0 needs Jellyfin 10.11 or newer | (unverified) | (unverified) | Jellyfin 12 removed legacy endpoints, which breaks very old clients |
| Release cadence in 2026 | How alive it is | 1.5 (Jul), 1.6 (Aug), 1.6.1 (Aug) | 1.0.2 (Jan), 1.1.0 (Jul) | 0.51.0 (Jan), 0.54.1 (Jun) | Swiftfin | |
| Settings sync | Same settings everywhere | (unverified) | (unverified) | Through a Jellyfin plugin | Streamyfin | |

## Pain points and unmet demand

1. **Plex's rewritten mobile apps shipped with regressions.** The forum thread
   "Releasing the new app was a crazy decision" has 474 posts, 25,607 views
   and 2,717 likes. Complaints cover slow navigation, lost list views,
   broken subtitle styling, unreliable picture-in-picture and missing season
   downloads.
   https://forums.plex.tv/t/releasing-the-new-app-was-a-crazy-decision/910913
   A follow-up thread about Plex's official response has 225 posts, 10,864
   views and 1,167 likes.
   https://forums.plex.tv/t/912745
2. **Casting broke on Plex's new apps.** At least nine threads from March to
   May 2025 report Chromecast discovery failing, external subtitles not
   loading or nothing playing, including threads with 36 and 44 posts, and a
   general "This new android app is awful" thread with 211 posts.
   https://forums.plex.tv/search?q=cast%20new%20app%20chromecast
3. **Plex removed automatic episode downloads and has not restored them.**
   Users relied on "keep the next N unwatched episodes" for travel. A request
   to restore it (7 votes, Dec 2025) still had frustrated replies in June and
   July 2026, and an Aug 2026 thread asks for an ETA and for automatic removal
   of watched episodes. One user says they disabled updates to keep the old
   app. Plex did restore "download only unwatched episodes in a season" in
   2025.18.0.
   https://forums.plex.tv/t/request-to-restore-smart-episode-downloads/934312
   https://forums.plex.tv/t/eta-for-automaticatic-unwatched-downloads/941310
   https://forums.plex.tv/t/please-allow-downloading-of-unwatched-episodes-again/919694
4. **Plex moved TV navigation and had to move it back.** The big-screen
   redesign put the menu at the top; after months of complaints Plex restored
   the left menu in Aug 2026.
   https://www.androidauthority.com/plex-brings-back-left-side-menu-3700033/
   https://www.xda-developers.com/after-months-backlash-plex-rolling-back-controversial-big-screen-app-redesign/
5. **Plex's paywall keeps widening.** Remote playback needs Plex Pass or a
   Remote Watch Pass, enforcement spread to smart TV platforms in March 2026,
   the Remote Watch Pass rose 50% on 1 June 2026 and the lifetime Plex Pass
   tripled to $749.99 on 1 July 2026. Downloads also need Plex Pass.
   https://www.howtogeek.com/plex-fire-tv-redesign-remote-streaming-limits/
   https://www.xda-developers.com/plex-slaps-price-increase-on-remote-watch-pass/
   https://9to5mac.com/2026/05/19/plex-increasing-lifetime-plex-pass-cost-to-whopping-750/
6. **Plex mobile lost everyday features.** Custom server URLs were removed,
   music moved to Plexamp, Watch Together was dropped, and in Jan 2026 the
   mobile app still could not add items to a playlist.
   https://www.howtogeek.com/plex-is-fixing-its-unpopular-new-mobile-apps/
   https://www.howtogeek.com/plex-is-ruining-one-of-my-favorite-features/
   https://cordcuttersnews.com/plex-announces-sleek-new-mobile-app-redesign-but-removes-some-features/
7. **Offline is Jellyfin's most-wanted feature by a wide margin.** "Support
   offline mode on Android mobile" leads the board with 1,820 votes (status
   "started" since 2020, when only full original files could be downloaded).
   Related: "Offline Sync Feature" 817 votes, "Pre-transcoding" 979,
   "Support download of transcoded files" 518 (open since 2019), "Download a
   batch" 219, "Sync playlists for offline listening" 36, offline in the web
   client 45.
   https://features.jellyfin.org/posts/218
   https://features.jellyfin.org/posts/1341
   https://features.jellyfin.org/posts/57
   https://features.jellyfin.org/posts/570
8. **Swiftfin still has no downloads.** The "Local Downloads" issue has 145
   upvotes and has been open since June 2021; work was split into sub-tasks in
   2025 and 2026.
   https://github.com/jellyfin/Swiftfin/issues/57
9. **Jellyfin TV coverage arrived late and still has gaps.** Apple TV support
   (463 votes) only reached the App Store in July 2026; Samsung (253 votes)
   reached the Samsung store in 2026 and only for some models; VIDAA
   (453 votes) has no client.
   https://github.com/jellyfin/Swiftfin/discussions/1294
   https://jellyfin.org/posts/state-of-the-fin-2026-05-24/
   https://features.jellyfin.org/posts/1615
10. **Jellyfin on iOS cannot cast to Chromecast.** 104 votes, open since 2020;
    the admin reply points users to VLC or Infuse instead.
    https://features.jellyfin.org/posts/466
11. **Emby's paid offline music does not do what buyers expected.** A March
    2026 thread lists missing automatic sync by artist, playlist and genre,
    no library-wide sync, no artist-album-track navigation offline and no
    alphabet jump in Android Auto. Emby staff replied in September 2026 that
    they are focusing on stability before features.
    https://emby.media/community/topic/146746-offline-music-sync-not-viable-in-2026-despite-statements-back-to-2022-that-its-planned-for-the-future/
12. **Plexamp music downloads were capped for six years.** "Download entire
    library or playlist in Plexamp" has 206 votes and 87 posts since 2020.
    The September 2026 release finally removed the time limit. The new
    background downloader then used cellular data while showing "paused"
    (Sept 2026; the co-founder confirmed the cause) and paused when the app
    was backgrounded.
    https://forums.plex.tv/t/646153
    https://forums.plex.tv/t/943398
    https://thedesk.net/2026/09/plexamp-new-app-desktop-windows-mac-linux/
13. **Nobody serves the watch well.** "Apple Watch Client App Request" on the
    Plex forum has 298 votes, 141 posts and 9,896 views since 2017, with
    activity in September 2026. Among the products studied, only Symfonium
    (Wear OS) and Emby (Apple Watch, Premiere only) offer watch apps.
    https://forums.plex.tv/t/206439
14. **Cars with built-in Android are ignored.** "Adapt Plexamp for Android
    Automotive OS" has 119 votes and 109 posts since 2020 with no staff reply.
    Official Jellyfin has no CarPlay (38 votes), and Emby charges for it.
    https://forums.plex.tv/t/649618
    https://features.jellyfin.org/posts/744
15. **Accessibility is treated as an afterthought.** Jellyfin's player
    controls are not usable with screen readers (GitHub #4504, open since
    April 2023; #442 open since 2019). Plexamp is reported unusable with
    VoiceOver (Aug 2025). Plex's new Apple TV app regressed Hover Text in
    September 2026. Jellyfin Android TV capped subtitle size in a way users
    called unusable for low vision (Sept 2026). Subtitle appearance is a
    188-vote Jellyfin request.
    https://github.com/jellyfin/jellyfin-web/issues/4504
    https://forums.plex.tv/t/927882
    https://forums.plex.tv/t/943164
    https://github.com/jellyfin/jellyfin-androidtv/issues/5842
16. **Translations are wide but shallow.** Jellyfin's main Weblate project
    lists 90 languages, but only 11 are at 90% or more and 41 are below 50%.
    Swiftfin sits at 48%.
    https://translate.jellyfin.org/projects/
17. **Android TV playback capability reporting is unreliable.** HDR support
    issue open since 2019 with 127 comments; Shield passthrough open since
    2020 with 50 comments; incomplete capabilities preventing direct play
    (Jan 2026, 11 reactions).
    https://github.com/jellyfin/jellyfin-androidtv/issues/49
    https://github.com/jellyfin/jellyfin-androidtv/issues/281
    https://github.com/jellyfin/jellyfin-androidtv/issues/5316
18. **Signing in on a TV and juggling servers is still clumsy.** Quick Connect
    by QR code has 225 votes; support for several servers has 177; a SyncPlay
    invite link has 267.
    https://features.jellyfin.org/posts/2642
    https://features.jellyfin.org/posts/47
    https://features.jellyfin.org/posts/971
19. **The Jellyfin ecosystem is fragmented.** Good features live in different
    third-party apps (downloads in Streamyfin, mpv tuning in Findroid, tvOS in
    Swiftfin, music in Finamp), several need extra server plugins, and
    Jellyfin 12 removed legacy endpoints so very old clients stop working.
    https://jellyfin.org/posts/jellyfin-release-12.0/

Where rivals are already good, and Gunmetal should not pretend otherwise:
Infuse is the reference video player on Apple devices (format coverage,
VoiceOver, background downloads, Live Activities). Plexamp is an excellent
music player with free CarPlay and Android Auto and, since September 2026, a
real offline library. Symfonium is the most capable music client on Android,
with casting to Chromecast, UPnP, Sonos and Kodi plus Wear OS. Emby has the
most complete server-managed sync (quality conversion, admin control, folder
sync). Jellyfin's breadth of platforms, free downloads and open feature board
are real strengths, and its official Android 2.7.0 release in August 2026
narrowed several gaps.

## Where Gunmetal can be clearly better

Each idea names the pain point it answers and what would have to be true
technically. All are consistent with records 1 and 2.

1. **Offline is the normal state, not a separate mode.** (Pain points 7, 11.)
   The README already commits to syncing the library to the device. If the
   client always browses its local copy, then offline browsing uses the same
   screens as online browsing, which fixes Emby's "separate Downloads view"
   problem and Plex's ungrouped download list. Technically: a delta sync
   protocol from the server's change log into an on-device SQLite store
   (the Rust core via UniFFI on native, WASM on web); partial sync for very
   large libraries (metadata first, artwork by budget); each item carries a
   local availability flag so the UI can dim what cannot play offline.
2. **Rule-based automatic downloads on every client.** (Pain points 3, 11, 12.)
   Users want "keep the next N unwatched episodes", "remove watched",
   "keep this artist and playlist in sync", "keep what I played" and a
   storage cap. Technically: rules are evaluated on the device against the
   synced library, so they work the same on every platform and keep working
   when the server is offline; they need a download queue with storage,
   Wi-Fi-only and charging-only conditions, and an explicit cellular policy
   with tests that prove it is honoured (Plexamp's September bug is the
   counter-example).
3. **Downloads are free, need no vendor account and survive an unreachable
   server.** (Pain point 5.) Plex and Emby both charge for downloads. With no
   central account (record 1, decision 7), an offline item needs a local
   grant: a device-bound key plus a signed record of what this device may
   play. Technically: the grant format, its expiry policy (admin-set,
   default long) and revocation on next contact must be designed, and the
   files are stored unencrypted or encrypted to the device key (an open
   question below).
4. **Lighter downloads without punishing small servers.** (Pain points 7, 12.)
   For music, Opus downloads are cheap to produce on any hardware (record 2).
   For video, offer three honest choices: original (no server cost), remuxed
   to a container the device prefers (cheap, lossless, uses the same remuxer
   as streaming), or transcoded, which runs in the sandboxed FFmpeg worker as
   a background job, can be scheduled for idle hours and is cached so a
   second device does not pay again. Technically: transcode jobs that
   produce a resumable file (not a live stream), plus a server-side size and
   time estimate shown before the user commits. Streamyfin's trick of saving
   an HLS stream proves demand but ties the download to a live transcode.
5. **Watch progress made offline merges cleanly.** (Pain point 7.) The
   append-only watch log in record 1 maps directly to offline use: each
   device appends events while offline and uploads them later. Technically:
   events need device IDs and a hybrid logical clock so merges are
   deterministic, and the "resume position" rule (latest event wins, or
   furthest progress wins) must be decided once and tested.
6. **Background transfers done properly.** (Pain point 12.) Technically: use
   the OS transfer services (background URL sessions on iOS, WorkManager or
   foreground services on Android), resumable byte-range requests (the server
   already serves bytes), and progress surfaced in system UI where the OS
   offers it (Infuse uses a Live Activity). Each of these is a native module
   behind the React Native UI.
7. **A ten-foot UI with a navigation contract.** (Pain points 4, 18.) Commit
   to a left navigation rail on TV and never move it without a migration
   path. Add focus-map tests that drive the D-pad through every screen and
   assert where focus lands. Sign-in on TV by QR code and passkey from a
   phone. Technically: react-native-tvos supports Apple TV and Android TV with
   focus guides and remote-event hooks; the focus tests need a harness that
   can run on both. Refresh-rate and resolution matching and accurate HDR
   capability reporting come from Gunmetal owning the player (libmpv plus OS
   display-mode APIs), which is exactly where Jellyfin Android TV struggles.
8. **One casting story with handoff as the default.** (Pain points 2, 10.)
   Gunmetal-to-Gunmetal handoff of the queue (record 2) is the primary path;
   Chromecast and AirPlay are secondary targets. Technically: a cast receiver
   app that accepts the remuxed fMP4 or HLS output, signed URLs that the
   receiver can refresh, LAN-direct URLs when sender and receiver share a
   network, and a phone-side proxy when the server is reachable only over
   iroh (Chromecast receivers cannot speak iroh). DLNA should stay out of the
   core because it is an unauthenticated LAN protocol; if offered, it belongs
   in a plugin with an explicit grant, as record 2 does for scrobbling.
9. **CarPlay and Android Auto in the first mobile music release, offline in
   the car, free.** (Pain point 14.) Technically: CarPlay templates and the
   Android Auto media library service are native-only, so they are native
   modules fed by the same on-device library. Jellify shows a React Native
   app can ship both. Android Automotive OS can reuse the Android Auto media
   service with extra packaging; it should be a stated later target because
   Plexamp has ignored it for six years.
10. **Watch apps that sync playlists for phone-free runs.** (Pain point 13.)
    Technically: watchOS and Wear OS apps must be native (React Native does
    not target watches). The server's cheap Opus or AAC transcodes suit small
    storage; Apple's guidance starts watch streams at 64 kbps. Until then,
    the OpenSubsonic adapter (record 2) lets Symfonium's Wear OS app work
    against Gunmetal, which is a cheap early answer.
11. **Accessibility as a gate, not a backlog.** (Pain point 15.) Treat an
    inaccessible control as a failing test, as a Jellyfin maintainer argued
    when moving the screen-reader request to a bug. Technically: every
    interactive React Native component carries role, label and state;
    automated checks (axe for the web build, accessibility snapshot tests for
    native) run in CI; the player's controls are React Native views drawn over
    the libmpv surface so they stay reachable; subtitle styling reads the OS
    caption settings by default; reduced-motion and larger-text settings are
    honoured and tested. No rival documents reduced-motion support, so this
    is an easy lead.
12. **Localisation with a completeness bar.** (Pain point 16.) Technically:
    ICU message format with plural rules, a pseudo-locale build in CI that
    catches hard-coded strings and clipped layouts, right-to-left layout tests,
    per-user UI language and server responses in the requested language, and
    a published threshold below which a language is labelled "partial".
13. **Platform reach through the stack already chosen.** (Pain point 9.)
    React Native covers iOS, Android, Apple TV and Android TV directly, and
    Amazon's Vega OS apps are themselves built with a React Native variant.
    Samsung and LG would use the web build with the platform's own video
    element and the server's remux path, as the README already plans for
    smart TVs. Roku is reached through the Jellyfin adapter (record 1,
    decision 6). This gives a credible path to more TV platforms than
    Jellyfin's separate per-platform codebases, provided the adapters track
    upstream changes such as Jellyfin 12's endpoint removals.
14. **Several servers and settings sync built in.** (Pain points 18, 19.)
    Moonfin needs two server plugins to sync settings across devices.
    Gunmetal can sync per-user client settings as ordinary library data, and
    let one client hold keys for several servers, since there is no central
    account to aggregate them.
15. **Admin-visible downloads and remote revocation.** (Emby is best here.)
    Owners see which devices hold which items and can revoke a lost device's
    key. Technically: the server tracks issued offline grants per device, and
    revocation takes effect on next contact.

## Risks and hard parts

- **App store distribution and the licence.** Gunmetal is AGPL-3.0 with no
  contributor licence agreement. The FSF has argued that Apple's App Store
  terms conflict with the GPL, and Apple removed a GPL app rather than change
  them (2010). Swiftfin is MPL-2.0 and Jellify is MIT, so they avoid this. If
  Gunmetal wants iOS and tvOS store builds, an explicit additional permission
  for app-store distribution likely has to be added before outside
  contributions arrive, because without a CLA every contributor would have to
  agree later. This is a legal question and needs proper advice
  (unverified legal analysis).
- **libmpv on Apple platforms.** Shipping libmpv inside iOS and tvOS apps
  raises licence (LGPL versus GPL build options), rendering and HDR questions
  (unverified detail). Swiftfin notes that Dolby Vision and Atmos face
  licensing barriers. AirPlay video normally goes through Apple's own player,
  so an mpv-based player may need a separate AVPlayer path for AirPlay
  (unverified).
- **React Native on TVs beyond Apple and Android.** react-native-tvos covers
  only Apple TV and Android TV. Samsung and LG would run a web build on low-end
  TV processors without libmpv, so they depend on the remuxer and on careful
  performance work. Store approval on Tizen and webOS took Jellyfin years.
- **Native-only surfaces.** CarPlay, Android Auto, watch apps, widgets, Live
  Activities and background transfer services are all native code. Each is a
  separate module with its own tests, and CarPlay audio apps need Apple's
  approval (process detail unverified).
- **Library sync size.** Syncing the whole library to a phone or TV is cheap
  for metadata but not for artwork. Very large libraries and small TV storage
  need partial sync, eviction and clear limits.
- **Offline grants versus security.** Offline playback means the device can
  play without asking the server, so a stolen phone keeps its downloads until
  the grant expires. The expiry and revocation policy must be explicit.
- **Transcoded downloads versus the low-hardware goal.** Video transcodes are
  exactly the cost Gunmetal wants to avoid. They must stay optional, sandboxed,
  scheduled and cached, and the default should be original or remux.
- **Casting with signed, short-lived URLs and iroh.** Cast receivers fetch
  URLs themselves and cannot join an iroh connection. Remote casting needs a
  relay through the phone or an HTTP endpoint, and URL lifetimes must cover a
  whole film or be refreshable.
- **Accessibility over a video surface.** libmpv draws into a native view
  with no accessibility tree, so every control must be a real UI element on
  top, including on TV where focus and screen readers interact.
- **Adapter drift.** The Jellyfin and OpenSubsonic adapters are how Gunmetal
  reaches Roku, Wear OS (via Symfonium) and existing apps early. Jellyfin 12
  shows that upstream APIs change and break older clients.
- **Scope.** The rivals each cover a subset: Plexamp and Symfonium do music,
  Infuse does Apple video, Jellyfin spreads across a dozen codebases. Matching
  all of them across every platform is a large surface for a small team; the
  plan needs a strict platform order.

## Open questions

1. Which platforms ship with the first music release? The roadmap says web
   first, then mobile and Android TV; does that include CarPlay and Android
   Auto from day one?
2. Will the project add an app-store distribution permission to the licence
   now, before external contributions, or avoid Apple's stores?
3. What is the default lifetime of an offline grant, and are downloaded files
   encrypted to the device key or stored in the clear for portability?
4. Does the first video release offer transcoded downloads at all, or only
   original and remux?
5. Is DLNA ever offered, and if so as a plugin with what grant?
6. Should Gunmetal write its own Chromecast receiver or rely on the default
   receiver with remuxed output?
7. How is remote casting handled when the server is only reachable over iroh?
8. Are watch apps a Gunmetal deliverable, or is the OpenSubsonic adapter plus
   Symfonium enough for the first year?
9. What accessibility standard is the gate: WCAG 2.2 AA for the web build,
   plus a written VoiceOver and TalkBack test script for native?
10. Which translation platform (self-hosted Weblate or a hosted service), and
    what completeness threshold marks a language as supported?
11. How much of the library do TVs with small storage sync, and what is
    fetched on demand?
12. How does the queue handoff protocol relate to Chromecast and AirPlay
    sessions: one abstraction or separate paths?

## Sources

- https://www.howtogeek.com/plex-is-fixing-its-unpopular-new-mobile-apps/
- https://www.howtogeek.com/plex-is-ruining-one-of-my-favorite-features/
- https://www.howtogeek.com/plex-fire-tv-redesign-remote-streaming-limits/
- https://www.howtogeek.com/plexamp-is-now-free-for-all/
- https://cordcuttersnews.com/plex-announces-sleek-new-mobile-app-redesign-but-removes-some-features/
- https://www.androidauthority.com/plex-brings-back-left-side-menu-3700033/
- https://www.xda-developers.com/after-months-backlash-plex-rolling-back-controversial-big-screen-app-redesign/
- https://www.xda-developers.com/plex-slaps-price-increase-on-remote-watch-pass/
- https://www.xda-developers.com/this-jellyfin-app-made-me-ditch-other-streaming-client/
- https://9to5mac.com/2026/05/19/plex-increasing-lifetime-plex-pass-cost-to-whopping-750/
- https://thedesk.net/2026/09/plexamp-new-app-desktop-windows-mac-linux/
- https://www.plex.tv/plexamp/
- https://www.plex.tv/apps-devices/
- https://support.plex.tv/articles/downloads-overview/ (read through search excerpts; direct fetch was blocked)
- https://support.plex.tv/articles/htpc-getting-started/ (read through search excerpts)
- https://forums.plex.tv/t/releasing-the-new-app-was-a-crazy-decision/910913
- https://forums.plex.tv/t/912745
- https://forums.plex.tv/t/request-to-restore-smart-episode-downloads/934312
- https://forums.plex.tv/t/please-allow-downloading-of-unwatched-episodes-again/919694
- https://forums.plex.tv/t/eta-for-automaticatic-unwatched-downloads/941310
- https://forums.plex.tv/t/206439
- https://forums.plex.tv/t/649618
- https://forums.plex.tv/t/646153
- https://forums.plex.tv/t/943398
- https://forums.plex.tv/t/943164
- https://forums.plex.tv/t/927882
- https://forums.plex.tv/search?q=cast%20new%20app%20chromecast (Discourse search results, read as JSON)
- https://forums.plex.tv/search?q=accessibility%20voiceover (Discourse search results, read as JSON)
- https://jellyfin.org/downloads/clients/
- https://jellyfin.org/docs/general/clients/
- https://jellyfin.org/posts/jellyfin-release-12.0/
- https://jellyfin.org/posts/state-of-the-fin-2026-05-24/
- https://jellyfin.org/posts/androidtv-v0.19.0/
- https://linuxiac.com/after-years-of-waiting-jellyfin-finally-lands-on-samsung-tizen-tvs/
- https://features.jellyfin.org/ (most-wanted list and individual posts read through its public API)
- https://features.jellyfin.org/posts/218
- https://features.jellyfin.org/posts/1341
- https://features.jellyfin.org/posts/57
- https://features.jellyfin.org/posts/466
- https://features.jellyfin.org/posts/1717
- https://translate.jellyfin.org/projects/
- https://translate.jellyfin.org/api/projects/jellyfin/languages/
- https://github.com/jellyfin/jellyfin-plugin-dlna
- https://github.com/jellyfin/jellyfin-android/releases
- https://github.com/jellyfin/jellyfin-desktop/releases
- https://github.com/jellyfin/jellyfin-web/issues/4504
- https://github.com/jellyfin/jellyfin-androidtv/issues/5842
- https://github.com/jellyfin/jellyfin-androidtv/issues/49
- https://github.com/jellyfin/jellyfin-androidtv/issues/281
- https://github.com/jellyfin/jellyfin-androidtv/issues/5316
- https://github.com/jellyfin/Swiftfin
- https://github.com/jellyfin/Swiftfin/releases
- https://github.com/jellyfin/Swiftfin/issues/57
- https://github.com/jellyfin/Swiftfin/discussions/1294
- https://github.com/jarnedemeulemeester/findroid
- https://github.com/jarnedemeulemeester/findroid/releases
- https://github.com/streamyfin/streamyfin
- https://github.com/streamyfin/streamyfin/releases
- https://github.com/finamp-app/finamp
- https://github.com/finamp-app/finamp/releases
- https://github.com/finamp-app/finamp/discussions/1749
- https://github.com/Jellify-Music/App
- https://github.com/edde746/plezy
- https://github.com/react-native-tvos/react-native-tvos
- https://www.tobiasreithmeier.de/en/blog/finamp-carplay-jellyfin-in-the-car
- https://perfectmediaserver.com/jellyfinjune/clients/tvos/
- https://swiftlyplayer.com/blog/jellyfin-android-tv-clients-compared/ (written by a competing client's developer; used for feature claims only)
- https://moonfin.io/
- https://emby.media/premiere.html
- https://emby.media/support/articles/Premiere-Feature-Matrix.html
- https://emby.media/support/articles/Sync-Introduction.html
- https://emby.media/support/articles/Offline-Access.html
- https://emby.media/download.html
- https://emby.media/community/topic/146746-offline-music-sync-not-viable-in-2026-despite-statements-back-to-2022-that-its-planned-for-the-future/
- https://firecore.com/infuse
- https://firecore.com/releases
- https://firecore.com/blog/infuse-85-smarter-streaming
- https://support.firecore.com/hc/en-us/articles/360006462093-Streaming-from-Plex-Emby-and-Jellyfin
- https://symfonium.app/
- https://symfonium.app/news/version-1410/
- https://symfonium.app/android-music-player-sonos-chromecast-dlna/
- https://developer.amazon.com/apps-and-games/vega
- https://developer.apple.com/videos/play/wwdc2019/716/
- https://www.fsf.org/blogs/licensing/more-about-the-app-store-gpl-enforcement
