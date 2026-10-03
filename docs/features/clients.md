# Clients, devices and offline

This map covers everything a listener or viewer holds or looks at: which
platforms Gunmetal runs on, the TV, phone, tablet, desktop and web
experiences, casting and handing playback between devices, offline
downloads and sync, background playback and lock-screen controls, car
integration, watches, and accessibility and localisation. The bar is set by
the best single-purpose apps, not by the media servers: Plexamp for music
(free CarPlay and Android Auto, and since September 2026 a real offline
library), Symfonium for Android music (casting, Wear OS, automatic caching),
Infuse for Apple video (format coverage, VoiceOver, background downloads)
and Emby for server-managed sync. Every Gunmetal client should keep working
with the server switched off, put nothing behind a paywall or vendor
account, be usable with a screen reader from the first release, and never be
worse at the basics than those apps on the same device. Where the research
shows a rival already does something well, the row says "parity" rather than
claiming an edge.

## Features

How to read the tables:

- Releases (R1, R2, R3, Later, No), the Demand scale, row ownership and the
  terms "the user log" and "the identity store" are defined once in the
  [feature map README](README.md). A row whose Release cell would differ
  between maps names one owning row; the other maps point at it.
- **Release** is the first release in which the feature ships on at least
  one client. R1 has only the web client (including the installable web
  app), so a feature marked R1 reaches native phones, TVs and desktops when
  those clients ship in R2; the UI surfaces column names them.
- Apple platforms (iPhone, iPad, Apple TV, CarPlay, Apple Watch) are Later,
  with the note "R2 if the App Store licence decision allows" (open decision
  2 here, open decision 3 in the README).
- Several neighbouring maps own parts of this area. This map lists only the
  device-specific part and says so: the music player map owns the queue,
  gapless and loudness; the playback map owns subtitles, skipping and the
  decision engine; the users and security maps own sign-in, profiles and
  device keys; the ecosystem map owns the compatibility adapters; the live
  TV map owns the guide and tuners.

### Platforms and distribution

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| CLI-001 | Web client served by your own server | The music library and player in any current browser, loaded from your server rather than a vendor's site | Plex yes (default app lives on app.plex.tv); Jellyfin yes; Emby yes | High: every rival has one; Chrome's local-network prompt can push Plex's web app onto its paid remote path | R1 | Parity with Jellyfin and Emby on origin; ahead of Plex (app.plex.tv). The actual edge: the Rust core runs as WASM and browsing reads a local copy instead of fetching each screen | Static asset serving; sync feed; signed byte-range URLs | Every R1 screen |
| CLI-002 | Published browser support list | Know which browsers are supported | Jellyfin publishes one (two latest Firefox, Chrome, Safari and Edge, plus Firefox ESR); Plex and Emby (unverified) | Low: no request found; basic hygiene | R1 | Parity; the published list is also the test matrix, so "supported" means tested | None | Docs; unsupported-browser notice |
| CLI-003 | Installable web app | Add Gunmetal to a phone or computer home screen and open it in its own window | Not covered by the research for any rival | Low: no direct evidence; matters while R1 has no native phone app | R1 | Gives phones a usable player in R1, from the same code as the web client Over HTTPS on a domain, or on localhost (CLI secure-context row). The R1 limits on phones are plain: no offline listening, no car support, and background playback on iPhone is unreliable (unverified); native apps arrive in R2 and the OpenSubsonic adapter is the interim route. | Web app manifest, icons and service worker served by the server | Install prompt; home-screen icon; standalone window |
| CLI-004 | Android phone and tablet app | A native app on Android handsets and tablets | Plex yes (2025 rewrite, heavily criticised; music moved to Plexamp); Jellyfin official is the web UI in a native shell, plus native third-party apps; Emby yes (full playback needs Premiere or an unlock) | High: Jellyfin's most-voted request is Android offline (#218, 1,820 votes) | R2 | One free app with libmpv direct play, the synced library and downloads from its first version | Device registry; sync feed; download support | All phone screens |
| CLI-005 | iPhone and iPad app | A native app on iOS and iPadOS | Plex yes (same rewrite); Jellyfin Swiftfin (no downloads yet), Streamyfin, Finamp; Emby yes (Premiere or unlock); Infuse leads for video (Pro for some formats) | High: Swiftfin's downloads issue has been open since 2021 (145 upvotes) | Later | Later; R2 if the App Store licence decision allows (open decision 3 in the feature map README). Same UI and features as the Android app, including downloads and CarPlay | As CLI-004 | All phone screens |
| CLI-006 | Android TV and Google TV app | A ten-foot app on Android TVs and boxes | Plex yes (redesign; left menu restored Aug 2026); Jellyfin official 0.19.x plus Wholphin and Moonfin; Emby yes | High: users judge servers by their TV apps (pain-points theme 15) | R2 | react-native-tvos with focus tests in the gate; libmpv reports what the device really decodes | Sync feed; capability-aware playback decisions | TV screens |
| CLI-007 | Apple TV app | A ten-foot app on tvOS | Plex yes (Sept 2026 rewrite drew a 268-post complaint thread); Jellyfin Swiftfin on the App Store since Jul 2026, Streamyfin; Emby yes; Infuse is the reference | High: Jellyfin request #612, 463 votes | Later | Later; R2 if the App Store licence decision allows (open decision 3 in the feature map README). Shares the TV code with Android TV; libmpv for most files, with an Apple player path where Dolby Vision needs it (see risks) | As CLI-006 | TV screens |
| CLI-008 | Fire TV on Fire OS | The Android TV app on Amazon's Fire OS devices | Plex yes (2026 redesign: lag, and about 2 clicks to a library became about 6); Jellyfin via the Amazon Appstore; Emby yes | Medium: Plex Fire TV backlash ("Hopelessly Crippled" thread) | R2 | Same build as CLI-006, held to a performance budget on the cheapest supported stick | As CLI-006 | TV screens |
| CLI-009 | Fire TV on Vega OS | An app for Amazon's newer non-Android Fire TV system | Plex and Emby (unverified); Jellyfin prototype (May 2026) | Low: new platform, no vote evidence | Later | Vega apps are built with a React Native variant, so the shared UI may carry over (effort unverified) | As CLI-006 | TV screens |
| CLI-010 | Samsung Tizen and LG webOS TVs | An app from the TV maker's store | Plex yes (Samsung playback broke for many in Aug 2026); Jellyfin in the LG store, and the Samsung store for Tizen 6 and newer since May 2026; Emby yes | Medium: Jellyfin's Samsung request took years (#335, 253 votes) | Later | Later, with VID-179, following the README roadmap, which packages Samsung and LG after the video milestone. The web build packaged for each TV, using the TV's own video element plus the in-process remuxer, so a container mismatch is remuxed and a transcode is needed only when the TV cannot decode a codec; weak TV runtimes may still struggle with image subtitles and some audio codecs (VID-179) | Remuxer; capability profiles per TV model | TV screens |
| CLI-011 | Other smart-TV platforms | VIDAA (Hisense), Titan OS (Philips) and Vizio | Plex lists VIDAA and Vizio; Jellyfin none on VIDAA (#1615, 453 votes), Titan client in testing; Emby (unverified) | Medium: 453 votes for VIDAA | Later | Same packaged web build as CLI-010 where the platform's web runtime can carry it (unverified per platform) | As CLI-010 | TV screens |
| CLI-012 | Roku through the Jellyfin adapter | Use Jellyfin's Roku app against a Gunmetal server | Plex yes; Jellyfin official Roku app (3.1.9); Emby yes | Medium: Roku coverage is expected and no first-party path is planned | Later | Coverage only: Roku users get Jellyfin's app, with more remuxes, transcodes and burn-in than native Gunmetal clients. The edge is that adapter tokens are scoped to read and play and they expire. | Jellyfin API subset; remuxed or HLS output Depends on INT-099 (Jellyfin adapter, video subset, Later) and VID-180. | Jellyfin's Roku app; listed under Settings > Devices |
| CLI-013 | Xbox | A console app | Plex yes; Jellyfin official app (0.9.5, May 2026); Emby yes (Premiere needed) | Low: no vote count found | Later | No edge claimed | As CLI-006 | TV screens |
| CLI-014 | Desktop app for Windows, macOS and Linux | An installable app that direct plays any file | Plex desktop and Plex HTPC ("Are the Desktop Apps Dead?" thread, 96 posts); Jellyfin Desktop 2.0 (Qt 6, mpv); Emby limits free playback to one minute; Plexamp desktop rebuilt on Tauri | Medium: Plex desktop neglect thread; Feishin's top issue is offline (88) | R2 | Parity with Jellyfin Desktop on direct play (Qt plus mpv, free, MPRIS). The edge is the synced library, handoff and offline downloads shared with the phone apps. React Native has no first-party Linux desktop target (unverified), so the Linux shell is an open decision (for example React Native Web in Tauri with libmpv in a native window, unverified) before Linux is promised in R2. | As CLI-001 | All screens; mini player |
| CLI-015 | Ten-foot mode on a PC | A couch interface on a home-theatre PC, driven by a controller | Plex HTPC (refresh-rate switching, controller, multichannel); Jellyfin Desktop TV mode and Emby Theater (unverified) | Low: no vote evidence | Later | The TV layouts already exist in the shared UI; the desktop shell hosts them full screen | None | TV layouts in the desktop app |
| CLI-016 | Headsets | Watch in Vision Pro or a VR headset | Infuse has a native visionOS app; Jellyfin none (#891, 167 votes); Plex Meta Quest request (126) | Low: 167 and 126 votes | Later | No edge claimed | None | Headset app |
| CLI-017 | Installs outside app stores | APK, F-Droid and sideloaded builds | Jellyfin F-Droid builds of official apps and Tizen 5 sideloading; Streamyfin Android TV by APK | Medium: de-Googled phones and older TVs | R2 | Every Android build plays, syncs and downloads without Google services; only casting is expected to need them (unverified) | Signed release artefacts | Download page on gunmetal.tv |
| CLI-018 | Several servers in one app | Your own server and a friend's, side by side | Plex yes (one account lists every server); Jellyfin request #47 (177 votes); Moonfin, Plezy and Infuse combine servers | Medium: 177 votes | R2 | The app holds a separate device key per server, each added by invite link, so no central account is needed to combine them Owns several-servers support; ACC-014 points here. | Invite redemption; per-server device keys | Server picker; Settings > Servers; optional merged Home |
| CLI-019 | PlayStation and Nintendo Switch | Console apps | Plex lists PS4 and PS5; Jellyfin none; Emby lists PS4 and PS3; Plex Switch request (320 votes) | Low: 320 votes for Switch | No | Not built; see Deliberately not doing | None | None |
| CLI-020 | Native Linux phone builds | An app for postmarketOS-style phones | None | Low: 20 votes (#222) | No | The installable web app covers them | None | None |
| CLI-021 | First-party Kodi add-on | Kodi as the front end | Jellyfin official JellyCon; Plex third-party add-on (unverified); Emby add-on | Low: the research rates it low priority | No | Gunmetal ships its own players; JellyCon may work through the Jellyfin adapter but is untested | None | None |

### Connection, sync and shared client behaviour

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| CLI-022 | Library synced to the device | Every list, page and search answers from a local copy, with no spinner | Plex, Jellyfin and Emby fetch each screen; Plexamp 4.50 caches library data; Infuse Library Mode caches metadata | High: Jellyfin lazy loading (#216, 1,179 votes); offline is the third-ranked pain theme | R1 | Delta sync from the server's change log into an on-device SQLite store, through the same Rust core (WASM in browsers, UniFFI on native) Size and time budgets (rows, artwork bytes, index build time) are the DIS-019 design goals and are tests on the reference device. | Per-user change log; delta and snapshot sync endpoints, filtered by profile | All browse and search screens |
| CLI-023 | Partial sync for small devices | Big libraries fit on TVs and in browsers: all metadata, artwork within a budget | None documented | Medium: TV and browser storage limits are flagged in three research files | R2 | Artwork is fetched within a budget and evicted least-recently-seen; missing art shows a placeholder, never a spinner Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Artwork in fixed sizes per device class | Settings > Storage; placeholders |
| CLI-024 | Sync and storage status | See what is synced, how much space each part uses and when it last synced | None documented | Low: no direct request; supports trust in offline use | R1 | One screen splits metadata, artwork and downloads, with "Sync now" and "Clear cache" | Sync cursor per device | Settings > Storage |
| CLI-025 | Losing the server never blocks the app | With the server or network gone, the app still browses, searches and plays what it holds | Rivals need the server to browse; Plexamp gained an offline mode in Sept 2026 | High: people want the app to work when the server is gone (pain-points theme 3) | R1 | Reads are local, so an outage only removes streaming of items that are not downloaded In R1 this needs a secure context (CLI-150): over HTTPS or on localhost a service worker loads the app with the server down, and browse and search work, but nothing plays without downloads (R2). Over plain HTTP on a LAN address the web client is online-only. | Health endpoint | Quiet status banner; dimmed items |
| CLI-026 | Offline is not a separate mode | Offline uses the same screens, with a "Downloaded" filter instead of a separate Downloads view | Emby's separate offline view is called inconsistent; Plex's new app shows one ungrouped list; Plexamp has a dedicated offline library | Medium: complaints about Emby and Plex | R1 | Every item carries an availability state (streamable, downloaded, unavailable), and dimming and filters come from it In R1 the only availability states in a browser are streamable and unavailable; "downloaded" arrives with native downloads in R2. | None beyond sync | All lists; "Downloaded" filter |
| CLI-027 | Pair a device by QR code | Scan the code on the TV with a signed-in phone and approve; no password typed with a remote | Plex link code; Jellyfin Quick Connect code, QR version planned (#2642, 225 votes); Emby PIN through Emby Connect | High: 225 votes | R2 | Approval on the phone mints a device-bound key for the TV, with no vendor service in the path (protocol owned by the security map) | Pairing endpoint; short-lived codes; rate limits | TV sign-in screen; phone approval sheet |
| CLI-028 | Custom address and proxy-friendly connections | Point the app at your own URL; works behind Authelia or Cloudflare Access with custom headers or client certificates | Plex removed custom server URLs from its new apps; Jellyfin supports them; Streamyfin custom headers; Finamp and Symfonium client certificates | Medium: Plex's removal was criticised | R2 | Each server entry stores its address, iroh node ID and optional headers or certificate; nothing forces a vendor broker | None | Add-server screen; Settings > Servers |
| CLI-029 | Automatic home and away switching | The fast local path at home and the remote path elsewhere, with no setting | Plex automatic (unverified); Streamyfin local auto-switch (0.54.1) | Medium: Streamyfin added it on request | R2 | iroh prefers a direct path when one exists, so the same identity works at home and away. Native clients embed iroh; the project's own connection-success figures will replace iroh's quoted rate of about 90% direct (unverified for mobile carriers and TVs). No open port; whether there is no fee behind CGNAT depends on the relay decision (ACC-101). Browsers still need the owner's proxy or VPN until ACC-102. | iroh endpoint | Connection indicator |
| CLI-030 | Settings that follow you | Set preferences once and every device follows, with "this device only" where it makes sense | Jellyfin only through plugins (Streamyfin, Moonfin); Plex account-level (unverified) | Medium: Moonfin needs two server plugins for this | R1 | Settings are synced user data with a user scope or a device scope (audio output, storage, quality) | User and device settings records, kept outside the rebuildable cache | Settings, with a "this device" marker |
| CLI-031 | A layout contract | The player bar, queue, device picker, scrubber and navigation stay put; changes arrive as an opt-in preview with a way back | Plex reverted its TV navigation in Aug 2026; Spotify, Tidal and YouTube Music redesigns drew backlash | High: Plex rollback vote (504); Plex New Experience feedback thread (1,531 posts) | R1 | Visual regression tests pin core controls; layout changes ship behind a preview switch with before-and-after screenshots in the release notes | None | Settings > Preview features; release notes |
| CLI-032 | Old clients keep working | A server update does not strand a TV app you cannot update yet | Jellyfin 12 removed legacy endpoints and broke very old clients; Findroid 1.0 needs Jellyfin 10.11 or newer | Medium: Jellyfin 12 breakage | R1 | Protocol types are versioned in the shared core; the server keeps the previous version and tells older clients plainly when to update | Protocol version negotiation | "Update needed" notice |
| CLI-033 | Diagnostics you can read first | Export a sync, download or playback report that shows exactly what it contains; nothing is sent automatically | Plexamp 4.50 sends download telemetry with titles and the account name, with no off switch | Medium: Plexamp backlash; privacy is the second reason people self-host | R1 | The report is built on the device, redacted by default, and saved as a file the user chooses to send | Server log excerpt on request | Settings > Help > Diagnostics |
| CLI-034 | Deep links | A link to an album, film or playlist opens it in the app, or in the web client | Spotify yes; Infuse deep links by TMDB ID; others (unverified) | Low: no vote evidence | R1 | Links carry only a random object ID; opening one still needs a signed-in device, so a link never grants access | Stable random object IDs | Share sheet; OS app links |
| CLI-150 | Secure context in R1 | The page says plainly which web features work on this address, and how to get the rest | Every web media server has the same browser rule; none explains it in the UI (unverified) | High: basic expectation | R1 | Browsers allow service workers, installation (CLI-003), offline loading (CLI-025), passkeys (ACC-050), Web Crypto and reliable persistent storage only over HTTPS or on localhost. Over plain HTTP on a LAN address, R1 is an online-only web player that signs in with a password (ACC-052). The fixes are a domain with HTTPS (ADM-021, ADM-022) and the owner decision on ADM-023 (a gunmetal.tv-issued name) | Report whether the request arrived over a secure origin | Banner on first sign-in; Settings > About this connection |

### TV (ten-foot)

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| CLI-035 | A left navigation rail that stays | Press left to reach libraries, search and settings, in the same place every release | Plex removed it and restored it in an Aug 2026 preview; Jellyfin Android TV left navigation (unverified); Netflix uses a top bar | High: Plex reversed after months of complaints | R2 | The rail's position is part of the layout contract (CLI-031) and pinned by tests | None | TV rail |
| CLI-036 | Predictable focus with memory | Focus lands where expected, and Back returns to the same row and tile | Plex's Aug 2026 update lists focus fixes; others (unverified) | High: focus bugs are the most common ten-foot complaint (unverified as a measured claim) | R2 | Focus-map tests drive the D-pad through every screen on Apple TV and Android TV and assert where focus lands | None | All TV screens |
| CLI-037 | Two presses to any library | Libraries within two presses, with favourites pinned to the rail | Plex's Fire TV redesign went from about 2 to about 6 clicks, and pins were added in Aug 2026; Jellyfin home sections request (#1986, 169 votes) | Medium: 169 votes; Fire TV complaints | R2 | Pins and Home sections are synced user data, so they survive reinstalls and updates | Home layout records | TV rail; Pin action |
| CLI-038 | Smooth on a cheap stick | No lag when scrolling large poster grids | Plex Fire TV users reported lag and 30-second local starts in 2026 | High: Fire TV complaints; Gunmetal targets cheap hardware | R2 | Rows read from the local store, grids are virtualised, and time to interactive and scroll frame time are tests on the cheapest supported device Budgets: DIS-019. | Artwork in small TV sizes | All TV grids |
| CLI-039 | A long-press menu on every card | Mark played, add to a playlist, go to the artist or remove from Continue without opening the item | Plex yes; Jellyfin partial; Infuse context menus for all three servers | Medium: Infuse shipped it on request | R2 | One shared action list drives TV, phone and web menus, so actions match everywhere | Same operations as other clients | TV cards |
| CLI-040 | Letter jump in long lists | Jump to a letter in long grids and lists | Spotify lacks an alphabet bar (884 votes); Swiftfin letter picker (1.5); Infuse | Medium: 884 votes | R1 | Built from the local index, so the jump is instant at any library size; arrives in the web client first, then phone, TV and car TV and remote part of DIS-101, which owns alphabet jump. | Sort keys in the sync payload | Web lists; phone fast scroller; TV letter column; car lists |
| CLI-041 | Music on the big screen | Artwork-led Now Playing, a visible queue and a dim ambient mode | Spotify TV (queue and dim mode since 2023); Caldera TV on Apple TV and Android TV; no native TV Plexamp | Medium: Plexamp users ask for Google TV support | R2 | The TV joins the same queue as every other device, so it can take over playback or be driven from a phone | Queue sync; session registry | TV Now Playing; TV queue; ambient mode |
| CLI-042 | Screensaver from your library | Ambient artwork when idle, safe for OLED | Jellyfin OLED-friendly screensaver; Plex (unverified) | Low: one completed Jellyfin request | R2 | Draws from the local artwork cache, which only holds what the profile may see | None | TV screensaver |
| CLI-043 | Text size on TV | Larger text and spacing for reading from the sofa | Emby font size options; Jellyfin request (10 votes) | Low: 10 votes | R2 | Type and spacing are design tokens, so one setting scales everything and each step is tested | None | Settings > Display |
| CLI-044 | Voice search from the remote | Speak a search into the remote | Jellyfin Android TV 0.19 native voice search; others (unverified) | Medium: Jellyfin shipped it in 2026 | R2 | The recognised text runs against the local, typo-tolerant index, so results appear without a server round trip | None | TV search |
| CLI-045 | Remote and controller keys | Dedicated play, pause, rewind and fast-forward keys; game controllers | Jellyfin added remote rewind and fast-forward in 12.0; Plex HTPC controller support | Low: no vote evidence | R2 | Parity | None | Player; all TV screens |
| CLI-046 | Frame-rate and resolution matching | Films play at their own frame rate, without judder | Plex HTPC; Wholphin; Jellyfin Android TV forces PCM audio when it switches (#4067); Apple TV relies on Match Content | Medium: a known Jellyfin side effect | R2 | The player module switches display mode independently of audio routing, so matching never breaks passthrough | Frame rate recorded at scan time | Settings > Playback |
| CLI-047 | An honest device capability report | The app knows which codecs, HDR formats and audio formats this device and its receiver take, and direct plays whenever it can | Jellyfin Android TV HDR issue open since 2019 (127 comments), passthrough since 2020 (50 comments), incomplete capabilities (#5316); Infuse best on Apple | High: long-running Jellyfin issues | R2 | The native player probes decoders and outputs and feeds the shared decision engine; a "This device" screen shows the result and the reason for each choice | Decision engine that takes device capability sets | Settings > This device; playback info |
| CLI-048 | Audio output settings per device | Passthrough, channel layout and lip-sync delay saved for this TV and receiver | Infuse (Atmos passthrough with Pro); Jellyfin Shield passthrough issue; Plex lip-sync only on some desktop apps (59 votes since 2014) | Medium: 59 votes; long-open issues | R2 | Stored with device scope (CLI-030), so a soundbar's lag stays with the soundbar | None | Settings > Audio output |
| CLI-049 | TV home-screen rows | Continue watching on Android TV's Play Next row and on Apple TV's Top Shelf | Jellyfin Android TV (completed); Streamyfin Top Shelf and recommendations; Plex (unverified) | Medium: completed Jellyfin requests | R2 | Filled from the synced watch log with no server call; a dismissal also removes the item from the system row | None | Android TV launcher; Apple TV Top Shelf |
| CLI-050 | Profile picker on shared TVs | Each person picks a profile at launch, with an optional PIN | Plex Home; Jellyfin account switching started (#2353, 154 votes); tvOS 26 profiles | Medium: 154 votes | R2 | Switching swaps to that profile's filtered local copy; on a shared TV the other profiles' caches are still on the device, so restricted items are hidden rather than absent (ACC-030) (profile model owned by the users map) | Per-profile sync feeds | TV profile picker |

### Phone and tablet

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| CLI-051 | Music and video in one app | One app, opening to music if that is what you use, with a music-only mode | Plex split music into Plexamp; Jellyfin one app; Emby one app ("Standalone Music App" request, 175 replies) | Medium: opinions are split | R2 | A music-forward Home and an option to hide video libraries, with no second app to install (open decision 8) | None | Home; Settings > Libraries shown |
| CLI-052 | Full library management on the phone | Add to playlists, reorder, rate and favourite without a computer | Plex mobile could not add to playlists (Jan 2026); Jellyfin yes (web UI); Finamp playlist editing | Medium: Plex "Better Playlists" request (1,425 votes) | R2 | The same operations on every client; edits made offline queue and merge (CLI-094) | Playlist and rating operations | Context menus; playlist editor |
| CLI-053 | Tablet layout | A side panel for browsing while the player stays visible, reflowing on rotation | Spotify's tablet app (Apr 2026); Finamp uses its phone layout on tablets (unverified) | Medium: Spotify built one in 2026 | R2 | Layouts per size class over shared components | None | Tablet layouts |
| CLI-054 | Landscape Now Playing | Artwork and controls side by side on a docked phone, with lyrics beside the art | Apple Music (iOS 27) | Low: new in 2026 | R2 | Parity | None | Now Playing |
| CLI-055 | Player gestures | Swipe to seek and change brightness or volume; double-tap to skip | Jellyfin Android 2.7 seek gestures; Streamyfin; Plex's new app lost double-tap; Jellyfin double-tap request (#131, 109 votes) | Medium: 109 votes | R2 | Every gesture has a visible button equivalent (CLI-142) | None | Video player |
| CLI-153 | Configurable tap and swipe gestures | Choose what a tap, long tap or double tap does | Marvis Pro maps taps and swipes (no vote count in the research) | Low: no vote count | Later | Gestures are actions on the one queue object, so a mapping is a per-device setting | None | Settings > Gestures |
| CLI-056 | Picture-in-picture | Keep watching in a floating window | Plex unreliable after its redesign; Jellyfin Android yes; Findroid; Streamyfin | Medium: Plex complaints | R2 | Uses the system picture-in-picture window (hand-off of the libmpv surface per platform unverified) | None | Video player |
| CLI-057 | External display | Play on a cabled TV from the phone | Swiftfin wide-ratio output; Jellyfin Android request (#2060) | Low: one open request | Later | Parity | None | Video player |
| CLI-058 | Admin on the phone | See sessions, start a scan and manage users from the app | Swiftfin admin dashboard and backups; Plex dashboard on mobile and TV (Jul 2026); Streamyfin sessions view | Medium: Plex shipped it in 2026 | R2 | Admin screens are part of the shared UI, so they reach phone, TV and web together (content owned by the operations map) | Admin API under the same authorisation layer | Admin section |
| CLI-059 | Choose where downloads go | Store downloads on an SD card or external storage | Plexamp allowed external storage (2020); request (31 posts) | Low: 31 posts | R2 | Parity Owns download location; MUS-216 points here. | None | Settings > Downloads |

### Desktop and web

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| CLI-060 | Wide three-pane layout | Library, content and a full-height queue visible together, with resizable panes | Spotify set the pattern (and squeezed its queue in 2024); Feishin copies it; Jellyfin web has no persistent queue (unverified) | Medium: Spotify's 2024 queue complaints | R1 | The queue pane can grow to full height and keeps durations and album names (owned with the music map) | None | Web and desktop layout |
| CLI-149 | Phone-width web layout | The web client and installed web app work one-handed on a phone: bottom navigation, a now-playing bar, a full-screen player and a queue sheet, with no horizontal scroll | Plex, Jellyfin and Navidrome web clients are usable on phones (unverified per client) | High: basic expectation | R1 | The only phone experience in R1, so it is designed first rather than shrunk from the desktop layout | None beyond CLI-001 | All screens at 360 to 430 px wide |
| CLI-061 | Keyboard shortcuts and command palette | Play, seek, queue next, like and search without the mouse | Jellyfin shortcuts and frame stepping (12.0); Spotify hotkey to queue (988 votes); Finamp desktop shortcuts | Medium: 988 votes | R2 | One action list drives shortcuts, menus and the palette, so every action is reachable by keyboard Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | None | Command palette; shortcut help |
| CLI-062 | Multi-select, drag and right-click | Act on many tracks at once; drag to the queue or a playlist | Spotify yes; Jellyfin web playlist multi-select is limited (open issue); Marvis Pro queue drop zones | Medium: open Jellyfin issue | R1 | Drop zones for "play next" and "play last"; batch operations Desktop input part of DIS-110, which owns multi-select. | Batch operations | Lists; queue; sidebar |
| CLI-063 | Desktop system media panels | Media keys, MPRIS on Linux, Windows media controls and macOS Now Playing in the desktop app | Jellyfin Desktop MPRIS; Jellyfin web works only partly with KDE (#5208); Navidrome's reported state drifts (#4744) | Medium: open issues on two projects | R2 | Native APIs in the desktop shell (CLI-014); state comes from the one queue object, with tests that the system panel matches the player. Owns desktop panels; MUS-081 points here. The R1 browser part is CLI-070 (with MUS-073 for music specifics). | None | System media panel; media keys |
| CLI-064 | Global hotkeys | Control playback from any app | Spotify and Plexamp (scope unverified) | Low: no evidence beyond in-app hotkeys | R2 | The shell maps global shortcuts onto the shared action list | None | Settings > Shortcuts |
| CLI-065 | Mini player window | A small always-on-top player | Apple MiniPlayer; Feishin request (32 votes) | Low: 32 votes | R2 | A second window of the same app on the same queue state Owns the mini player; MUS-082 points here. | None | Mini player |
| CLI-066 | A light desktop shell | Small download, low memory | Plexamp moved from Electron to Tauri for size and memory (Sept 2026) | Medium: a direct signal from Plexamp | R2 | The shell is chosen by benchmark, and idle memory is published like scan time | None | None |
| CLI-067 | Pick and keep the output device | Choose speakers, headphones or a DAC, and keep that choice | Spotify request (5,509 votes); Plexamp and Roon output selection; browsers decide for web clients | High: 5,509 votes | R2 | Output is a device-scoped setting (CLI-030); the bit-perfect path is in the music map Owns output device choice; MUS-200 points here. | None | Output picker; device picker |
| CLI-068 | Respect the operating system | The app never launches on a play key or headphone connection unless asked | Apple's Mac Music app does this; a tool to stop it reached 669 points on Hacker News | Medium: 669 points on Hacker News | R2 | Autostart and media-key ownership are opt-in | None | Settings > System |

### Background playback and system integration

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| CLI-069 | Background audio with lock-screen controls | Music plays with the screen off, with title, art and controls on the lock screen, notification, Bluetooth displays and watch | Plexamp, Symfonium and Finamp yes; Plex and Jellyfin apps yes | High: a basic expectation; ADR 2 requires it | R2 | Parity | None | Lock screen; notification; Bluetooth displays |
| CLI-070 | Lock-screen controls from the web app | Media Session controls when playing in a browser or the installed web app | Navidrome via Media Session (state drift bug); Jellyfin web (unverified) | Medium: open Navidrome bug | R1 | State is derived from the queue object and tested; iOS Safari's background limits are stated plainly Owns browser media controls in R1; MUS-073 is the music specifics. Needs a secure context for the installed web app (CLI-150); in a plain browser tab over HTTP, Media Session controls still work where the browser allows them (unverified per browser). | None | Lock screen; notification |
| CLI-071 | Interruptions handled cleanly | Calls and navigation prompts pause or lower the music and then resume; unplugging headphones pauses | Jellyfin Android added handling in 2.7.2; others (unverified) | Medium: Jellyfin fixed it in 2026 | R2 | Parity, with a per-platform test list | None | None |
| CLI-072 | Audio carries on for video | The soundtrack keeps playing when the app is backgrounded; an audio-only mode for concerts | Swiftfin pauses video in the background; Jellyfin 12.0 fixed iOS background audio; audio-only request (#533, 34 votes) | Low: 34 votes | R2 | Audio-only asks the remuxer for just the audio track, so the phone stops fetching video | Audio-only remux output | Player menu > Audio only |
| CLI-073 | Home-screen widgets | Now playing and quick starts from the phone home screen | Symfonium adaptive widgets; Plexamp Android widget; Finamp request (19 votes); Apple Music widgets (iOS 27) | Medium: Plexamp built one on request | Later | The widget reads the local store, so it shows recent items without a network | None | Android and iOS widgets |
| CLI-074 | Phone voice assistants | "Play my running playlist" through Siri or Google Assistant | Plexamp Siri; Finamp Siri (beta); Jellyfin Android Auto voice | Medium: several apps added it in 2026 | Later | Requests resolve against the local library, so they work offline | None | System assistants |
| CLI-075 | NFC tags | Tap a tag to start an album or playlist | Plexamp (free) | Low: no vote evidence | Later | The tag holds only an object ID; playback still needs a signed-in device | None | Context menu > Write to tag |
| CLI-076 | Local notifications | "Downloads finished" or "Sync paused: storage full", with no push service | Plexamp added specific download errors (4.50); others (unverified) | Low: no vote evidence | R2 | Raised on the device, so no vendor push relay is needed | None | System notifications |
| CLI-077 | Remote push notifications | Server events reach a phone while the app is closed | Plex (unverified); others (unverified) | Low: no vote evidence | Later | Needs Apple's and Google's push services, which sit uneasily with no central account (open decision 6) | Push relay | System notifications |

### Offline: downloads and sync

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| CLI-078 | Download music | Albums, artists, playlists and tracks with no signal, free, in the native apps | Plexamp needs Plex Pass; Finamp free; Symfonium one-time purchase; Emby Premiere; no rival downloads in a browser (Jellyfin #3012, 45 votes; Feishin's top issue, 88) | High: paywalls on Plex and Emby; Jellyfin and Feishin requests | R2 | Free and account-free; downloaded items stay in the normal library with a badge. Owns music downloads; MUS-210 points here. Downloads inside the browser are MUS-217 (Later), because browser storage limits, eviction and the secure-context requirement (CLI-150) make them fragile. | Signed byte-range URLs; offline grants | Download toggle on album, artist and playlist pages; Downloads |
| CLI-079 | Download the whole music library | Carry everything | Plexamp only since Sept 2026 (206-vote request from 2020); Finamp yes; Emby users report it missing | Medium: 206 votes | R2 | One rule ("all music") with a size estimate before it starts | Size totals per rule | Downloads > Rules |
| CLI-080 | Downloads that follow rules | Keep a smart playlist, an artist, favourites or five-star tracks downloaded automatically | Symfonium automatic caching; Plexamp background refresh (Sept 2026); Emby lacks sync by artist, playlist or genre | High: Jellyfin "Offline Sync Feature" (817 votes); Emby complaint thread | R2 | Rules are smart-playlist queries in the shared core, evaluated on the device against the synced library, identically on every client and with the server off | Shared rule language; byte serving | Downloads > Rules; "Keep downloaded" on any smart playlist |
| CLI-081 | Keep what I played | Recently played music stays offline within a cap | Plexamp "Keep Played Music" (Sept 2026); Roon ARC Smart Downloads; Symfonium cache | Medium: two rivals shipped it recently | R2 | Driven by the local listening log, so it rotates with no server work | None | Downloads > Rules |
| CLI-082 | Storage cap and eviction | Set a limit; automatic downloads make room, and what you chose by hand stays | Plexamp (a cap for Keep Played Music); others (unverified) | Medium: storage limits whole-library rules | R2 | Manual and rule-based downloads are tracked apart, so eviction never removes a manual choice | None | Settings > Storage |
| CLI-083 | Smaller music downloads | Opus copies that use a fraction of the space | Plexamp (Opus); Finamp transcoded downloads; Symfonium for Wear OS | Medium: Finamp and Plexamp both offer it | R2 | Opus encoding is cheap on any hardware (ADR 2) and cached on the server, so a second device does not pay again | Opus encode jobs in the sandboxed worker; cache of encoded files | Download quality setting |
| CLI-084 | Download films and episodes as they are | Original files for offline viewing | Plex Pass; Jellyfin Android downloads originals only (#218, 1,820 votes); Swiftfin none; Emby Premiere | High: 1,820 votes | R2 | libmpv plays the original, so the server only reads the disk; free | Signed byte-range URLs | Download button on films and episodes; Downloads |
| CLI-085 | Remuxed video downloads | The same picture and sound in a container the device's player prefers | Not offered as a distinct option by rivals | Medium: part of the demand for compatible downloads | R2 | The in-process remuxer writes the file, losslessly and cheaply, when the original container will not play | Remux-to-file job | Download options |
| CLI-086 | Smaller transcoded video downloads | Fit a season on a phone | Plex yes; Jellyfin planned (#57, 518 votes); Emby Premiere (server conversion); Streamyfin saves an HLS transcode | High: 518 votes; pre-transcoding request (979 votes) | Later | Scheduled, sandboxed jobs with a size and time estimate shown first, cached for other devices (open decision 5) | Scheduled transcode jobs; output cache | Download options with estimate |
| CLI-087 | Season, series and batch downloads | One action for a season or a show, optionally only unwatched episodes | Plex restored season and unwatched-only downloads (2025.18.0); Jellyfin batch download started (#655, 219 votes); Emby series and libraries | Medium: 219 votes | R2 | Expressed as a rule (CLI-080), so new episodes follow automatically | None | Season and show pages |
| CLI-088 | Keep the next episodes, drop watched ones | Downloads refill as you watch, and watched episodes delete themselves | Plex lost it in the 2025 rewrite and users still asked in Aug 2026; Emby sync rules; Plezy | High: repeated Plex threads; Jellyfin "Offline Sync Feature" (817 votes) | R2 | Evaluated on the device from the synced watch log, so it refills correctly even after watching offline | None | Show page > Keep next episodes |
| CLI-089 | Downloads manager | Grouped by show and album, with progress percentages, multi-select delete and plain failure reasons | Plex shows one ungrouped list with no bulk delete; Jellyfin Android multi-select (2.7.0), iOS requests (#3955, #3957); Plexamp specific errors (4.50); Emby drill-down | Medium: complaints on Plex and Jellyfin | R2 | Every failure has a typed reason (storage, network rule, revoked, server error) shown in plain words | Typed error responses | Downloads screen |
| CLI-090 | Background downloads that resume | Downloads continue with the app closed and survive dropped connections | Infuse (iOS background downloads with a Live Activity); Streamyfin; Plexamp paused when backgrounded (Sept 2026) | Medium: Plexamp complaints | R2 | The OS transfer services (background sessions on iOS, WorkManager on Android) with resumable byte ranges and progress in system UI | Range requests; refreshable URLs | Notification; Live Activity |
| CLI-091 | A mobile-data rule that is kept | Wi-Fi only, charging only, or mobile data allowed, and never broken silently | Plexamp downloaded over cellular while showing "paused" (Sept 2026, confirmed by its co-founder); Finamp requests | Medium: Plexamp bug thread | R2 | One native download module enforces the rule, with tests that simulate network changes | None | Settings > Downloads > Network |
| CLI-092 | Streaming quality per network | Originals on Wi-Fi, smaller streams on mobile data | Spotify quality settings; Finamp requests (20 and 7 reactions) | Medium: Finamp requests | R2 | Uses the same Opus path as downloads, and never downgrades on Wi-Fi unless asked | On-the-fly Opus encoding | Settings > Playback > Mobile data |
| CLI-093 | Offline plays and progress merge cleanly | What you played or watched offline appears in history and resume points later | Streamyfin syncs download progress (0.30.2); others (unverified) | High: the heart of offline use | R1 | In R1 the web client queues play events made while the connection drops and uploads them on reconnect; from R2 native downloads use the same path. Each device appends to the log with its device ID and a hybrid logical clock, merged by one tested rule | Log upload with de-duplication | History (showing the device) |
| CLI-094 | Offline edits merge later | Playlist edits, ratings and queue changes made offline sync on return | Finamp request (#1065); Spotify (unverified) | Medium: open Finamp request | R2 | Edits are small operations replayed against the server's version, with stated conflict rules Owns offline edits; MUS-152 and MUS-214 point here. | Operation endpoint; conflict rules | Rare conflict notice |
| CLI-095 | Downloads that outlive the connection | Downloads keep playing when the server is down or you are abroad, for an admin-set period | Spotify requires going online every 30 days; rivals' rules (unverified) | Medium: travel and outage use | R2 | An offline grant: the device key plus a signed record of what it may play and until when, renewed on any contact (open decision 4) | Grant issuance and renewal; revocation list | Expiry note on the Downloads screen |
| CLI-096 | See and revoke downloads per device | The owner sees which devices hold what, and can revoke a lost phone | Emby (admins control all synced media); Jellyfin none | Low: Emby only | R2 | The server tracks grants per device; revocation takes effect on next contact, and the docs say so | Grant registry | Admin > Devices |
| CLI-097 | Start a download from another device | Queue a phone download from the web while packing | Emby yes; Jellyfin no | Low: Emby only | R2 | Download intents are synced data, and the phone acts on them at its next sync | Download intent records | Download menu > On device |
| CLI-098 | Extras travel with downloads | Lyrics, artwork, chapters, skip markers and previews work offline | Streamyfin keeps skip segments offline; Findroid stores images; Spotify lacks offline lyrics (1,600 votes) | Medium: 1,600 votes | R2 | Lyrics and markers are part of the synced library, so they are offline by default Owns offline extras; MUS-160 points here. | Lyrics and markers in the sync payload | Now Playing; video player |
| CLI-099 | Fetch ahead on patchy signal | Upcoming tracks or the next episode load early | Plexamp advanced pre-caching (free); Jellyfin pre-buffer request (#400, 222 votes) | Medium: 222 votes | R1 | Byte ranges are known from the scan, so the client fetches exactly what comes next | Byte-range serving | Settings > Playback > Fetch ahead |
| CLI-100 | Copy to a folder or drive | Mirror chosen media to an external drive | Emby Folder Sync (Premiere) | Low: Emby only | Later | Parity | Copy job | Admin > Devices |

### Handoff, remote control and casting

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| CLI-101 | Device picker and handoff | See every Gunmetal player you own and move playback there with the same queue and position | Spotify Connect is the reference; Plexamp players; Jellyfin web plays on other sessions; Finamp "Play On" requested (18) | High: ADR 2 commitment; Jellyfin "like Spotify Connect" request (29 votes) | R2 | Owns device handoff; MUS-197 points here. Jellyfin can already remote-control its sessions with no vendor account; Gunmetal's edge is one versioned queue that any device can take over with the same position and lanes, offline-tolerant, which Spotify Connect does with an account and Plexamp only with plex.tv. Moves to R2 with the native clients: in R1 the only players are browser tabs. | A WebSocket control channel per signed-in session; commands authorised per profile; server-assigned queue versions (MUS-122); a written and tested conflict rule. Must be designed before either CLI-101 or CLI-102 is built. | Device picker in the player bar and Now Playing |
| CLI-102 | Remote control another player | Drive the TV or desktop from your phone or laptop including the volume of the controlled device | Jellyfin can control any session; Swiftfin session commands (1.5); Plexamp remote control off the home network (2022) | Medium: Finamp request (18) | R2 | The control channel runs through the server (over iroh for native apps), so it works at home and away Covers your own devices only; controlling another person's player is ACC-047 (Later). Owns remote control and remote volume; MUS-198 points here. Symfonium sets the bar for per-speaker volume. | The CLI-101 control channel, with a volume command | Device picker; remote mode in Now Playing |
| CLI-103 | Continue on this device | Open the app elsewhere and it offers to pick up where you left off | Spotify (unverified); others (unverified) | Medium: implied by handoff demand | R1 | The queue and position are already in the local copy; the prompt is UI only Stays in R1: it needs only the R1 persistent queue (MUS-122), not the control channel. | Queue sync | Player bar prompt |
| CLI-104 | Headless and dedicated players | A screenless box, a hi-fi streamer or the server's own sound card, controlled from the app | Plexamp headless (Plex Pass); Caldera headless; Navidrome jukebox; Lyrion with Squeezelite | Medium: fits low-hardware users | Later | Free; a headless player uses the same core and session protocol | Session registry | Device picker |
| CLI-105 | Synchronised multi-room | The same music in several rooms, in step | Spotify request (8,800 votes); Plex "Tandem Playback" (420); Roon and Lyrion do it | High: 8,800 votes | Later | Builds on the queue and session registry with clock sync, as a separate project that does not block handoff | Clock sync; group sessions | Device picker grouping |
| CLI-106 | Chromecast from Android and the web | Send music or video to a Chromecast or Google TV | Plex yes (casting broke for many after the 2025 rollout); Jellyfin yes; Emby yes; Symfonium best for music | Medium: Plex casting threads (up to 44 posts) | R2 | Signed URLs the receiver can refresh, and remuxed output when the device cannot open the container; handoff stays the main path between Gunmetal devices Owns casting for music and video; MUS-203 and VID-146 point here. Casting music to Nest and Google Cast speakers is the partial answer to the Google Home demand that CLI-131 declines; music needs only refreshable signed URLs and the original stream, no remuxer. | Refreshable signed URLs; remux output | Cast button; device picker |
| CLI-107 | Chromecast from iPhone | The same from iOS | Plex yes; Jellyfin's official iOS apps cannot (#466, 104 votes); Streamyfin video | Medium: 104 votes | Later | Later; R2 if the App Store licence decision allows (open decision 3 in the feature map README). The same sender code as Android, through the shared UI | As CLI-106 | Cast button |
| CLI-108 | Subtitles while casting | Subtitles appear on the cast device | Plex external subtitles failing on Chromecast; Streamyfin still in progress | Medium: Plex thread (22 posts) | R2 | Text subtitles go as a side track in the receiver's format; image subtitles follow the playback map's rules | Subtitle conversion | Cast subtitle menu |
| CLI-109 | AirPlay | Send audio or video to Apple TV and AirPlay speakers | Plexamp free; Infuse (Pro); Jellyfin web; music AirPlay on iOS requested (#3713) | Medium: open Jellyfin request | Later | Later; R2 if the App Store licence decision allows (open decision 3 in the feature map README). Audio parity; video may need Apple's own player path (unverified) | Remuxed fMP4 or HLS | AirPlay button; device picker |
| CLI-110 | Local casting and a phone relay | Cast devices fetch from the server on the home network; away from home the phone relays | Symfonium proxy mode; Jellyfin request (#993, 5 votes) | Low: 5 votes | R2 | Cast receivers cannot speak iroh, so the phone serves a local relay when the server is reachable only over iroh | LAN address advertisement | None (automatic) |
| CLI-111 | Casting that falls back gracefully | Casting still works when the receiver cannot decode the file | Symfonium transcodes for Chromecast; Plex (unverified) | Medium: a Symfonium selling point | R2 | Remux first and sandboxed transcode last, with the reason shown | Receiver profiles in the decision engine | Cast info |
| CLI-112 | Cast controls in the notification | Stop or adjust casting from the shade or lock screen | Jellyfin request (#258, 10 votes) | Low: 10 votes | R2 | Parity | None | Notification |
| CLI-113 | Control UPnP renderers and Sonos | The phone controls network speakers | Symfonium (gapless UPnP, Sonos groups); Plex lists Sonos; Jellyfin has no official support | Medium: a Symfonium selling point | Later | Until then, Sonos is reachable through Music Assistant via the OpenSubsonic adapter | None | Device picker |
| CLI-114 | Watch or listen together | See VID-153, which owns this feature. | Plex removed it (restore request, 2,878 votes); Jellyfin SyncPlay (invite link #971, 267 votes); Spotify Jam (Premium hosts) | High: 2,878 votes | Later | See VID-153. | None beyond VID-153. | Group session sheet |
| CLI-115 | DLNA server | Old TVs and receivers browse the library over UPnP | Plex is a DLNA server; Jellyfin moved DLNA into a plugin | Low: no vote evidence | No | Not built; see Deliberately not doing | None | None |

### Car

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| CLI-116 | Android Auto | Browse and play from the car screen on Android | Plexamp free; Symfonium; Jellyfin Android 2.7 redesign; Finamp beta; Emby Premiere | High: core to replacing a streaming app | R2 | Free; a native media-library service fed by the same on-device library | None beyond sync | Car browse tree; car Now Playing |
| CLI-117 | Apple CarPlay | The same for iPhone | Plexamp free; Jellyfin's official app has none (#744, 38 votes); Finamp beta (Jun 2026); Emby Premiere | High: a Marvis Pro reviewer called its absence a deal breaker | Later | Later; R2 if the App Store licence decision allows (open decision 3 in the feature map README). Free; CarPlay templates fed by the local library; needs Apple's entitlement (unverified) | None | CarPlay screens |
| CLI-118 | Your Home in the car | The car's browse tree mirrors your own Home sections | None | Medium: Home customisation is Spotify's second most-voted idea (15,697) | R2 | Home sections are saved queries, so customising Home customises the car | Home layout records | Car browse tree |
| CLI-119 | Offline in the car | Downloads browse and play with no signal | Plexamp (Sept 2026); some apps show only downloads in CarPlay | Medium: tunnels and rural roads | R2 | The car tree reads the local store, so lost signal changes nothing for downloaded items; other items are marked or hidden | None | Car browse tree |
| CLI-120 | Voice in the car | Ask for an album, artist or playlist while driving | Jellyfin Android Auto voice (2.7.0); Plexamp Siri; Finamp Siri (beta) | Medium: several apps added it in 2026 | R2 | Requests resolve against the local search index | None | Car voice |
| CLI-121 | Quick navigation of long car lists | Get through long lists in the car interface | Emby users ask for it; others (unverified) | Low: Emby forum thread | R2 | Browse nodes grouped by letter where the platform allows it (unverified) | None | Car browse tree |
| CLI-122 | Queue modes in the car | Shuffle, repeat and up next on the head unit | Finamp fixed shuffle display in an Aug 2026 beta; Jellyfin request (#1122, 12 votes) | Low: 12 votes | R2 | Parity | None | Car Now Playing |
| CLI-123 | Android Automotive OS | A native app in cars with built-in Android | Plexamp none (119 votes since 2020, no staff reply); no confirmed rival | Medium: 119 votes | Later | Reuses the Android Auto media service with extra packaging | None | Built-in car interface |
| CLI-124 | Audiobooks in the car | Resume, chapter skip and rewind on resume | Symfonium; Jellyfin Android Auto audiobooks (2.7.0) | Low: arrives with audiobooks | Later | Uses the multi-context queue planned for audiobooks | None | Car browse tree |
| CLI-125 | The car's own browser | Play in a car's built-in web browser | Plex request (22 votes) | Low: 22 votes | Later | The web client may already work there; test and document it | None | Web client |
| CLI-126 | Video in the car | Watch while parked | None; CarPlay does not allow video; Jellyfin request (#3536, 18 votes) | Low: 18 votes | No | Not built; see Deliberately not doing | None | None |

### Watches, voice and smart home

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| CLI-127 | Control from the watch | Pause and skip from the wrist | All, through the system Now Playing app | Low: already works through the OS | R2 | Parity, provided by CLI-069 | None | Watch Now Playing |
| CLI-128 | Wear OS app with offline music | Run with music on the watch and no phone | Symfonium (syncs with transcoding); Plex none | Medium: watch apps are wanted mainly for phone-free runs | Later | Uses the same cheap Opus encoding path | Opus encoding | Wear OS app |
| CLI-129 | Apple Watch app with offline music | The same on Apple Watch | Emby (Premiere only); Plex request (298 votes since 2017) | Medium: 298 votes | Later | Same as CLI-128, with streams starting at the low bitrate Apple recommends for watches | Opus or AAC encoding | watchOS app |
| CLI-130 | Existing music apps as interim clients | Use Subsonic apps such as Symfonium (with its Wear OS and Android Auto support) before native apps exist | Navidrome with Subsonic clients; Navidrome's Jellyfin music API (0.64) | Medium: users stay for clients, as Navidrome's API work shows | R2 | API-key sign-in only, scoped to read and play, off by default and behind the same authorisation tests (adapter owned by the ecosystem map; see open decision 1) Ships with the adapter in R2. It is the interim answer for iPhone users, whose native app is Later. | OpenSubsonic adapter | Settings > Devices > App passwords |
| CLI-131 | Voice assistant skills | "Alexa, play..." on smart speakers | Plex shut its Alexa skill (Jun 2026); Plex Google Home request (2,461 votes); Jellyfin request (243); Emby Alexa (Premiere) | High: 2,461 votes | No | Not built; see Deliberately not doing The partial answer to the Google Home demand is casting to Nest and Google Cast speakers (CLI-106, R2). This row owns the decision; INT-146 and DIS-181 point here. | None | None |

### Live TV on devices

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| CLI-132 | Channel zapping on the remote | Channel up, channel down and last channel from the remote | Plex none (requests with 76 and 7 votes); Jellyfin zapping-list request (28 votes); Channels DVR and TiviMate | Medium: 76 votes | R3 | Native players decode the broadcast on the device, so changing channel needs no server transcode (owned with the live TV map) | Tuner broker | Live player; guide |
| CLI-133 | Cast live TV | Send a live channel to a Chromecast | Plex partial (iOS and web only; request had 379 votes); others (unverified) | Medium: 379 votes | R3 | The same casting path as CLI-106, with remuxed live output | Live remux output | Cast button in the live player |
| CLI-134 | Multi-view on more devices | Two to four channels on one screen | Channels DVR on Apple TV 4K and iPad only; Plex request (34 votes); Jellyfin none | Low: 34 votes | Later | Several libmpv players on one screen, limited by a decoder probe per device (owned with the live TV map) | Tuner broker | Live player layouts |

### Accessibility and localisation

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| CLI-135 | Screen readers reach every control | VoiceOver, TalkBack and desktop screen readers work everywhere, including over video | Jellyfin web player controls are not accessible (#4504 since 2023, #442 since 2019); Plexamp reported unusable with VoiceOver (2025), fixed in a Sept 2026 beta; Infuse best | Medium: issues open for years; regressions in 2025 and 2026 | R1 | Every interactive component carries a role, label and state; player controls are real views above the libmpv surface | None | All screens |
| CLI-136 | Accessibility as a release gate | An inaccessible control fails the build instead of joining a backlog | None documented | Medium: follows from CLI-135 | R1 | axe checks on the web build, accessibility snapshot tests on native, and a written VoiceOver and TalkBack script run before each release | None | None |
| CLI-137 | Screen readers on TV | VoiceOver and Hover Text on tvOS, TalkBack on Android TV | Plex's new Apple TV app regressed (Sept 2026) | Low: one regression thread | R2 | Focus and labels come from one model and are tested together on TV | None | TV screens |
| CLI-138 | Full keyboard use with visible focus | Everything works without a mouse, with a clear focus ring | Jellyfin 12.0 keyboard and controller fixes | Medium: Jellyfin fixed it in 2026 | R1 | The same focus model as TV (CLI-036), tested on the web | None | All web screens |
| CLI-139 | Text follows the system size | Text scales with OS or browser settings without clipping | Plex ignored iOS font settings (2025); Swiftfin improved Dynamic Type (1.3); a Plexamp user finds text too small on a car mount | Medium: Plex complaints | R1 | Type sizes are tokens, and layouts are tested at the largest system sizes | None | All screens |
| CLI-140 | Reduced motion | Animations calm down when the OS asks | No rival documents it; Apple's Liquid Glass was criticised for motion (118 "me too" votes to turn it off) | Low: 118 votes on Apple's forum | R1 | Motion is defined as tokens with a reduced set, and tested | None | All screens |
| CLI-141 | Themes, including high contrast | Dark, light, high-contrast and OLED-black themes, following the system by default | Spotify has no light mode (7,031 votes); Finamp AMOLED theme; no rival documents high contrast | High: 7,031 votes | R1 | Colour tokens with contrast checked in CI | None | Settings > Appearance |
| CLI-142 | Motor accessibility | Large targets, and no action that only a gesture can reach | Plex's double-tap fullscreen and picture-in-picture called hard for limited mobility; Marvis Pro configurable gestures | Low: one complaint thread | R1 | Minimum target sizes are checked by tests, and every gesture is mirrored by a button | None | All screens |
| CLI-151 | Mono audio and channel balance | Play in mono, or shift the balance left or right, for listeners with hearing loss in one ear | Spotify and the phone operating systems offer this (unverified); media servers not covered in the research | Low: no vote data (unverified) | R1 | On the web through Web Audio in R1, and in the native player modules in R2; a per-device setting | None | Settings > Accessibility |
| CLI-152 | Search inside settings | Type to find a setting instead of hunting through pages | Symfonium requested (no vote count in the research) | Low: no vote count | R2 | The research flags settings sprawl as a risk; R1 has few settings, so search arrives when the native clients add theirs | None | Settings |
| CLI-154 | Alternative app icons | Pick a different home-screen icon | Marvis Pro has more than 40 icons; Spotify reverted an icon change within days after backlash (research, no vote count) | Low: no vote count | Later | Shipped as optional variants of the gunmetal mark; the default never changes without notice | None | Settings > Appearance |
| CLI-143 | Subtitles that follow system caption settings | The system caption style by default, per-user overrides, and no size cap | Plex's new iOS app ignored the settings and broke styling (2025); Jellyfin appearance request (#161, 188 votes); Jellyfin Android TV capped the size (#5842) | High: 188 votes; low-vision complaints | R2 | Reads the OS caption preferences; overrides sync per user; the extremes are tested | None | Player subtitle menu; Settings > Accessibility |
| CLI-144 | Accessible track defaults | Prefer SDH subtitles or audio description automatically | Netflix; Jellyfin label request (#1206, 5 votes) | Low: 5 votes | R2 | Track flags read at scan time drive the default choice (which flags each format carries is unverified) | Track flags in the index | Settings > Accessibility |
| CLI-145 | Generated captions | Captions for files that have none | No rival; Jellyfin Whisper request (#2143, 61 votes) | Low: 61 votes | Later | An opt-in sandboxed job or plugin, given the CPU cost (owned with the playback map) | Speech-to-text job | Subtitle menu |
| CLI-146 | Translations with a completeness bar | English at launch, with the translation framework in place; other languages are added with honest labels when partial | Jellyfin lists 90 languages but only 11 are 90% complete or more; Swiftfin is at 48% | Medium: shallow translations | R1 | ICU messages with plural rules; a pseudo-locale build in CI catches hard-coded and clipped strings; languages below the bar are labelled partial | Translatable server messages | Settings > Language |
| CLI-147 | Right-to-left layouts | Arabic and Hebrew mirrored correctly | No rival documents right-to-left testing | Low: no vote evidence | R2 | Right-to-left layout tests in CI from the start R1 keeps logical CSS properties and externalised strings so mirroring is possible later; full mirroring and its tests arrive in R2. | None | All screens |
| CLI-148 | Each person's own language | Every user sees their own language, including server messages | Jellyfin 12 lets clients ask for a response language (earlier request #370, 219 votes) | Medium: 219 votes | R2 | Language is a synced user setting; server errors are typed and translated on the client R2: translated server messages need the typed-error catalogue to settle first; R1 ships English with typed errors. | Typed errors | Settings > Language |

## Differentiators

1. **Offline is the normal state, and it is free (CLI-022, CLI-025, CLI-026,
   CLI-078, CLI-093).** Offline use is the third-ranked pain theme in the
   research and Jellyfin's most-voted request (1,820 votes). Plex and Emby
   charge for downloads, and Plexamp needed a rebuild in September 2026 to
   get an offline library. Gunmetal starts there: the synced library means
   browsing and search never wait for the server, offline uses the same
   screens, and plays made offline merge through the append-only log. In R1
   the browser keeps browsing and search working when the server is down
   (over HTTPS or localhost, CLI-150); downloads arrive with the native apps
   in R2, and downloads inside the browser stay Later (MUS-217) because
   browsers can evict storage.
2. **Downloads that manage themselves (CLI-080, CLI-081, CLI-082, CLI-088,
   CLI-091).** "Keep the next N episodes" is the feature Plex users most
   regret losing, Emby buyers complain that offline music cannot follow an
   artist or playlist, and Plexamp's new downloader used mobile data while
   showing "paused". Rules evaluated on the device, a storage cap that never
   evicts what you chose by hand, and a mobile-data rule enforced by one
   tested module answer all three, for music and TV alike.
3. **Your devices share one queue, with no cloud account (CLI-101, CLI-102,
   CLI-103).** Spotify Connect is the reference everyone measures against,
   and it needs a Spotify account. Jellyfin and Finamp users have asked for
   the same thing for years. Gunmetal's queue is one versioned object on your
   own server. Jellyfin can already remote-control its sessions with no
   vendor account; the edge is that any device takes over the same queue
   with its position and lanes, offline-tolerant. The persistent queue and
   "continue on this device" ship in R1; handoff and remote control arrive
   with the native clients in R2. Multi-room comes later and builds
   on the same object.
4. **A TV app that keeps its promises (CLI-035, CLI-036, CLI-038, CLI-047).**
   Plex had to put its TV navigation back after months of complaints, its
   Fire TV app was called lagging and crippled, and Jellyfin's Android TV
   capability bugs have been open since 2019. Gunmetal commits to a fixed
   left rail, proves focus behaviour with tests, holds a performance budget
   on the cheapest supported stick, and owns the player, so it can report
   exactly what the device decodes.
5. **Free car support that follows your Home and works in tunnels (CLI-116,
   CLI-118, CLI-119 in R2; CarPlay, CLI-117, Later).** Emby charges for
   CarPlay and Android Auto, and Jellyfin's official iOS app has no CarPlay. Plexamp is free and very good here, so on
   price this is parity; the edge is that the car's browse tree is built from
   the user's own Home sections and reads the local library, so it keeps
   working with no signal.
6. **Accessible and stable by rule (CLI-031, CLI-135, CLI-136, CLI-140,
   CLI-141).** Jellyfin's player controls have been unusable with screen
   readers since 2023, Plexamp was reported unusable with VoiceOver, and
   every product studied has broken a habit users relied on. Making
   accessibility and layout stability part of the build gate, and shipping
   reduced motion and a high-contrast theme that no rival documents, is a
   cheap and credible lead.

## Deliberately not doing

- **A DLNA server in the core (CLI-115).** DLNA is an unauthenticated LAN
  protocol: anything on the network can browse and fetch the library, which
  undoes per-object authorisation (record 1, decision 6). If it is ever
  offered, it is a plugin that is off by default with an explicit LAN grant,
  the same treatment record 2 gives third-party services.
- **Voice assistant skills (CLI-131).** Alexa and Google Assistant skills need
  a public cloud endpoint and account linking, which means a central service
  the project would run, and that conflicts with having no central account.
  Plex itself shut its Alexa skill in June 2026. Home Assistant's own voice
  features, through the ecosystem map's integration, are the path instead.
  Phone assistants that run against the local library (CLI-074) are a
  different thing and stay wanted.
- **PlayStation and Nintendo Switch (CLI-019).** Neither runs React Native or
  libmpv through any path the research found, console publishing needs
  closed SDKs and platform approval (unverified for an AGPL project), and
  the evidence of demand is weak next to the TV platforms.
- **Native Linux phone builds (CLI-020).** Twenty votes on Jellyfin's board;
  the installable web app covers these phones.
- **A first-party Kodi add-on (CLI-021).** Gunmetal ships its own players.
  Jellyfin's Kodi add-on may work through the adapter, but it will not be
  tested or supported.
- **Video in the car (CLI-126).** CarPlay does not allow video, the demand is
  small (18 votes) and it invites unsafe use.
- **A separate offline mode.** Offline is the same app with availability
  marked on each item (CLI-026). A distinct "Downloads mode" is the pattern
  users criticise in Emby and Plex.
- **Paid unlocks, device caps or per-app fees.** Emby caps a household at 30
  devices and gates full mobile playback; Plex gates downloads and remote
  playback. The AGPL build is the whole product (record 1, decision 10).
- **Client telemetry.** Clients send nothing on their own. Diagnostics leave
  the device only as a file the user exports and sends (CLI-033), in contrast
  with Plexamp 4.50's download telemetry that cannot be turned off.

## Dependencies and risks

- **Blocking: ADR 3, durable user state.** ADR 1 (decision 5) names watch
  history as the only irreplaceable data, but most rows in this map that
  write user data (playlists, loves, ratings, corrections, layouts, shares,
  identities, IDs, settings) need it to survive a cache rebuild. ADR 3 must
  be accepted before any server work that stores user data. It defines the
  two durable stores every map now names the same way: **the user log** and
  **the identity store** (see the [feature map README](README.md)).

**Other maps this one depends on**

- The music player map owns the queue object, gapless playback, loudness and
  lyrics. Handoff (CLI-101) and offline edits (CLI-094) cannot be promised
  until that map's queue merge rules are designed and tested.
- The playback map owns the decision engine, the remuxer, subtitles and
  skipping. TV capability reporting (CLI-047), casting (CLI-106 to CLI-111)
  and remuxed downloads (CLI-085) all feed it or depend on it.
- The security and users maps own pairing, device keys, passkeys and
  profiles. Profile filtering must be applied when the sync payload is built,
  not only in the UI, or restricted items reach the device.
- The ecosystem map owns the OpenSubsonic and Jellyfin adapters, which are how
  Roku (CLI-012) and interim third-party clients (CLI-130) arrive.
- Remote access over iroh (record 1, decision 7) underlies home and away
  switching (CLI-029) and remote control. Browsers on iroh are relay-only, so
  remote listening in the web client costs relay bandwidth on every byte, and
  remote video in a browser probably needs the user's own domain and proxy.

**Architecture records**

- Record 1 treats SQLite as a rebuildable cache with watch history as the only
  irreplaceable data. This map adds more data that cannot be rebuilt from the
  files: synced settings (CLI-030), Home and pin layouts (CLI-037), download
  rules and intents (CLI-080, CLI-097), offline grants (CLI-095) and queued
  edit operations (CLI-094). They need the same append-only, exportable
  treatment, which needs a new record.
- R1 has no transcoder, so R1 downloads are originals only and streaming on
  mobile data runs at the file's own bitrate. Opus copies (CLI-083, CLI-092)
  wait for the sandboxed worker in R2 unless a pure-Rust encoder path is
  found (unverified).

**Legal and platform**

- **The App Store and the AGPL.** The FSF has argued that Apple's App Store
  terms conflict with the GPL, and Apple removed a GPL app in 2010 rather
  than change them. Gunmetal has no contributor licence agreement, so an
  app-store permission can only be added cheaply before outside
  contributions arrive (unverified legal analysis; needs proper advice). This
  gates CLI-005, CLI-007, CLI-107, CLI-109, CLI-117 and CLI-129.
- **libmpv on Apple platforms** raises licence (LGPL versus GPL build
  options), rendering and HDR questions (unverified). Dolby Vision on Apple TV
  and AirPlay video probably need Apple's own player as a second path
  (unverified), which means two players to test on one platform.
- **TV platforms.** react-native-tvos covers only Apple TV and Android TV.
  Samsung and LG run the web build on weak processors without libmpv, so
  they depend on the remuxer and careful performance work, and store
  approval on Tizen and webOS took Jellyfin years.
- **Native-only surfaces.** CarPlay, Android Auto, widgets, Live Activities,
  watch apps and background transfer services are all native modules, each
  with its own tests. CarPlay audio apps need an entitlement from Apple
  (process unverified).
- **Push and voice need vendor clouds.** Remote push (CLI-077) needs Apple's
  and Google's services; voice skills need a public endpoint.

**Engineering risks**

- **Sync size.** Metadata is cheap but artwork is not. Cheap TV sticks have
  little storage, Apple TV may purge caches, and browsers can evict storage.
  Partial sync (CLI-023) weakens "works offline" on those devices, and the
  limits need measuring and publishing.
- **Browser audio.** Background playback on iPhones is restrictive, the Media
  Session API is not available everywhere, and Navidrome's reported state
  drifts. R1's lock-screen and offline promises in the browser are best
  effort on iOS.
- **Offline grants versus stolen devices.** A downloaded item plays without
  asking the server, so a stolen phone keeps its downloads until its grant
  expires. Revocation only works on next contact (CLI-096), and the docs must
  say so.
- **Casting and signed URLs.** Cast receivers fetch URLs themselves and
  cannot join an iroh connection. URL lifetimes must cover a whole film or be
  refreshable, and remote casting needs the phone relay (CLI-110).
- **Accessibility over video.** libmpv draws into a native view with no
  accessibility tree, so every control must be a real view on top, including
  on TV where focus and screen readers interact.
- **Focus and performance on TV.** Large virtualised grids and focus handling
  in React Native on TV are where Plex's Fire TV app failed; the budget in
  CLI-038 has to be a test, not a goal.
- **Adapter drift.** Jellyfin 12 removed endpoints that older clients relied
  on. Any coverage that comes through an adapter is only as stable as the
  adapter's tracking of upstream.

**Scope**

- This map has 148 features across more than a dozen platforms. The rivals
  each cover a slice: Plexamp and Symfonium do music, Infuse does Apple
  video, and Jellyfin spreads across many separate codebases. A small team
  under a test-first gate with 100% coverage and zero surviving mutants
  needs a strict platform order (open decision 3), or the TV apps, which are
  how users judge servers, will lag.

## Open decisions for the project owner

1. **What do phone users get before native apps?** R1 ships only the web
   client, but record 2 promises background playback, lock-screen controls,
   offline downloads and a queue that hands off, and three of those need a
   native app. The README roadmap builds mobile and Android TV before video,
   while the release plan puts native clients in R2 with video.
   *Partly decided in the feature map README:* R1 ships the installable web
   app with a phone-width layout (CLI-149) and browser Media Session
   controls, and states its phone limits plainly (CLI-003). The
   OpenSubsonic adapter (CLI-130) is R2, not R1, because it doubles the
   authorisation surface and few apps are known to support API-key sign-in.
   Still open: whether the native Android music app (background audio, lock
   screen, downloads, Android Auto) ships as soon as it is ready, as an R1
   point release, rather than waiting for the remuxer. *Recommendation:*
   yes.
2. **App Store distribution and the licence.** iPhone, iPad, Apple TV,
   CarPlay and Apple Watch all depend on it. Without a contributor licence
   agreement, an app-store permission cannot be added later without every
   contributor's consent. *Recommendation:* get legal advice now, and if it
   supports an additional permission for app-store distribution, add it
   before accepting the first outside contribution. If not, Apple platforms
   move to Later and the docs say why.
3. **Platform order for R2.** *Recommendation:* Android phone first; then
   Android TV, Google TV and Fire OS as one build; then the desktop shell;
   then iPhone, iPad and Apple TV once decision 2 is settled (Later until
   then). Samsung and LG packaging (CLI-010) and Roku through the Jellyfin
   adapter (CLI-012, which needs INT-099) are Later, following the README
   roadmap.
   VIDAA, Vega OS, Xbox and native watch apps stay Later, with the
   OpenSubsonic adapter and Symfonium covering Wear OS in the meantime.
4. **How long downloads last without contact, and how they are protected.**
   *Recommendation:* an admin-set grant lifetime with a default of 30 days
   (the window Spotify uses), renewed silently on any contact with the
   server, and revoked on next contact. Store files in app-private storage
   and rely on the operating system's encryption; no per-file encryption or
   DRM, which adds complexity and protects little against someone who
   already holds the unlocked device.
5. **Transcoded video downloads in R2.** They are the most-requested form of
   video download (518 votes) and exactly the server cost Gunmetal wants to
   avoid. *Recommendation:* R2 ships original and remuxed downloads only.
   Transcoded downloads come later as admin-enabled, scheduled, cached jobs
   in the sandbox, off by default.
6. **Remote push notifications.** They need Apple's and Google's push
   services and, in practice, a relay the project would run. *Recommendation:*
   no remote push in R2; local notifications only (CLI-076). Revisit only
   with a self-hostable relay design, on the same terms as iroh relays.
7. **Chromecast receiver.** The default media receiver needs nothing from the
   project; a custom receiver would need a registration with Google and a
   hosted receiver page (details unverified). *Recommendation:* start with
   the default receiver and remuxed output. Build a custom receiver only if
   subtitles or queue control require it, hosted as a static page on
   gunmetal.tv that collects nothing.
8. **One phone app, or a separate music app?** Plex split music into Plexamp;
   Emby users have asked for a standalone music app; Jellyfin is one app.
   *Recommendation:* one app with a music-only mode that hides video libraries
   and opens on the music Home. It keeps one codebase, one sign-in and one
   set of downloads, and matches record 2's "full media player" intent.
9. **The bars for accessibility and translation.** *Recommendation:* make
   WCAG 2.2 AA for the web build and a written VoiceOver and TalkBack script
   per release into release blockers; use Weblate, as Jellyfin does, so
   translators work in an open tool; and label any language below 90%
   complete as "partial" in the language picker. R1 ships English only, with
   the framework in place (CLI-146).
10. **Which shell runs the shared UI on Linux desktops (CLI-014)?** React
    Native has no first-party Linux desktop target (unverified).
    *Recommendation:* prototype React Native Web inside Tauri with libmpv in
    a native window (unverified) before Linux is promised for R2; if it
    fails, Linux keeps the web client and the installable web app.
