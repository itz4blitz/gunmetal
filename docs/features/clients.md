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

- Releases (R1, R1.1, R1.2, R1.3, R2, R3, Later, No), the Demand scale, row
  ownership and the terms "the user log" and "the identity store" are
  defined once in the [feature map README](README.md); the point releases
  R1.1 to R1.3 are those the owner adopted in
  [D-10](../decisions.md#d-10-r1-scope-and-the-release-table). A row whose
  Release cell would differ between maps names one owning row; the other
  maps point at it.
- **Release** is the first release in which the feature ships on at least
  one client. R1 and its point releases have only the web client (the
  installable web app arrives in R1.1, CLI-003), so a feature marked R1 or
  R1.x reaches native phones and TVs when those clients ship in R2, and
  desktops when the desktop shell ships (Later, following the security
  baseline's release scope); the UI surfaces column names them.
- R1 reaches the server over HTTPS on the owner's own domain with automatic
  certificates, a tailnet name, or localhost on the same machine, and from
  away through the owner's reverse proxy or the tailnet (CLI-150, ACC-097).
  The project's per-server name service and built-in remote access (iroh)
  arrive in R2, as the owner decided
  ([decision register](../decisions.md#owner-answers-2026-10-02)).
- **Security** lists the requirements in [docs/security](../security/README.md)
  that govern the row; a builder must meet all of them, and they win over
  any other cell. "None specific" means only the rules every client follows
  apply: untrusted text rendered as text (SEC-CLI-001), every restriction
  enforced by the server (SEC-CLI-015) and bounded decoding of server
  responses (SEC-CLI-021). The trust boundaries and threats this area
  touches are in [Security notes](#security-notes).
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

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| CLI-001 | Web client served by your own server | The music library and player in any current browser, loaded from your server rather than a vendor's site | Plex yes (default app lives on app.plex.tv); Jellyfin yes; Emby yes | High: every rival has one; Chrome's local-network prompt can push Plex's web app onto its paid remote path | R1 | Parity with Jellyfin and Emby on origin; ahead of Plex (app.plex.tv). The actual edge: the Rust core runs as WASM and browsing reads a local copy instead of fetching each screen | Static asset serving; sync feed; signed byte-range URLs | Every R1 screen | SEC-IAM-014, SEC-NET-058, SEC-CLI-012, SEC-API-044, SEC-API-032, SEC-NET-001 |
| CLI-002 | Published browser support list | Know which browsers are supported | Jellyfin publishes one (two latest Firefox, Chrome, Safari and Edge, plus Firefox ESR); Plex and Emby (unverified) | Low: no request found; basic hygiene | R1 | Parity; the published list is also the test matrix, so "supported" means tested | None | Docs; unsupported-browser notice | SEC-API-052, SEC-CLI-011 |
| CLI-003 | Installable web app | Add Gunmetal to a phone or computer home screen and open it in its own window | Not covered by the research for any rival | Low: no direct evidence; matters while there is no native phone app | R1.1 | Gives phones an installable player from R1.1 (in R1 they use the phone-width web layout, CLI-149), from the same code as the web client. Over HTTPS on the owner's domain or a tailnet name, or on localhost (CLI-150). The limits on phones before the native apps are plain: no offline listening, no car support, and background playback on iPhone is unreliable (unverified); native apps arrive in R2 and the OpenSubsonic adapter is the interim route. The service worker never stores media capability URLs (SEC-API-029), never serves a bundle older than the running server's (SEC-CLI-011), and is removed at sign-out (CLI-156). | Web app manifest, icons and service worker served by the server | Install prompt; home-screen icon; standalone window | SEC-NET-001, SEC-API-044, SEC-API-029, SEC-CLI-009, SEC-CLI-011 |
| CLI-004 | Android phone and tablet app | A native app on Android handsets and tablets | Plex yes (2025 rewrite, heavily criticised; music moved to Plexamp); Jellyfin official is the web UI in a native shell, plus native third-party apps; Emby yes (full playback needs Premiere or an unlock) | High: Jellyfin's most-voted request is Android offline (#218, 1,820 votes) | R2 | One free app with libmpv direct play, the synced library and downloads from its first version | Device registry; sync feed; download support | All phone screens | SEC-IAM-048, SEC-CLI-030, SEC-CLI-035, SEC-CLI-045, SEC-CLI-046, SEC-STD-039 |
| CLI-005 | iPhone and iPad app | A native app on iOS and iPadOS | Plex yes (same rewrite); Jellyfin Swiftfin (no downloads yet), Streamyfin, Finamp; Emby yes (Premiere or unlock); Infuse leads for video (Pro for some formats) | High: Swiftfin's downloads issue has been open since 2021 (145 upvotes) | Later | Later; R2 if the App Store licence decision allows (open decision 3 in the feature map README). Same UI and features as the Android app, including downloads and CarPlay | As CLI-004 | All phone screens | SEC-IAM-048, SEC-CLI-030, SEC-CLI-033, SEC-CLI-045, SEC-CLI-046, SEC-STD-039 |
| CLI-006 | Android TV and Google TV app | A ten-foot app on Android TVs and boxes | Plex yes (redesign; left menu restored Aug 2026); Jellyfin official 0.19.x plus Wholphin and Moonfin; Emby yes | High: users judge servers by their TV apps (pain-points theme 15) | R2 | react-native-tvos with focus tests in the gate; libmpv reports what the device really decodes. The TV is a limited-class device that can never approve devices or administer (SEC-CLI-024); enrolled as a household device it works only on the home network by default and goes dormant after 30 days unused until an adult taps to wake it (SEC-IAM-109). | Sync feed; capability-aware playback decisions | TV screens | SEC-CLI-024, SEC-IAM-055, SEC-IAM-109, SEC-CLI-054, SEC-CLI-066 |
| CLI-007 | Apple TV app | A ten-foot app on tvOS | Plex yes (Sept 2026 rewrite drew a 268-post complaint thread); Jellyfin Swiftfin on the App Store since Jul 2026, Streamyfin; Emby yes; Infuse is the reference | High: Jellyfin request #612, 463 votes | Later | Later; R2 if the App Store licence decision allows (open decision 3 in the feature map README). Shares the TV code with Android TV; libmpv for most files, with an Apple player path where Dolby Vision needs it (see risks) | As CLI-006 | TV screens | SEC-CLI-024, SEC-IAM-055, SEC-IAM-109, SEC-CLI-030, SEC-CLI-050 |
| CLI-008 | Fire TV on Fire OS | The Android TV app on Amazon's Fire OS devices | Plex yes (2026 redesign: lag, and about 2 clicks to a library became about 6); Jellyfin via the Amazon Appstore; Emby yes | Medium: Plex Fire TV backlash ("Hopelessly Crippled" thread) | R2 | Same build as CLI-006, held to a performance budget on the cheapest supported stick | As CLI-006 | TV screens | SEC-CLI-024, SEC-IAM-055, SEC-IAM-109, SEC-CLI-066 |
| CLI-009 | Fire TV on Vega OS | An app for Amazon's newer non-Android Fire TV system | Plex and Emby (unverified); Jellyfin prototype (May 2026) | Low: new platform, no vote evidence | Later | Vega apps are built with a React Native variant, so the shared UI may carry over (effort unverified) | As CLI-006 | TV screens | SEC-CLI-024, SEC-CLI-030, SEC-CLI-045, SEC-CLI-066 |
| CLI-010 | Samsung Tizen and LG webOS TVs | An app from the TV maker's store | Plex yes (Samsung playback broke for many in Aug 2026); Jellyfin in the LG store, and the Samsung store for Tizen 6 and newer since May 2026; Emby yes | Medium: Jellyfin's Samsung request took years (#335, 253 votes) | Later | Later, with VID-179, following the README roadmap, which packages Samsung and LG after the video milestone. The web build packaged for each TV, using the TV's own video element plus the server's remuxer, which runs in a worker process (SEC-MED-081), so a container mismatch is remuxed and a transcode is needed only when the TV cannot decode a codec; weak TV runtimes may still struggle with image subtitles and some audio codecs (VID-179). Each TV holds a non-extractable Web Crypto key, is a limited-class device with no downloads, and a platform ships only after its secure-context test passes (SEC-CLI-070). The server re-encodes all artwork, so the TV's old browser engine never decodes an attacker's image bytes (SEC-TM-035). Record 29: the set is that later app plus a server-side control, the control is not a plugin, and neither starts in the current release. | Remuxer in a worker process; capability profiles per TV model | TV screens | SEC-CLI-070, SEC-CLI-024, SEC-MED-081, SEC-TM-035, SEC-MED-046 |
| CLI-011 | Other smart-TV platforms | VIDAA (Hisense), Titan OS (Philips) and Vizio | Plex lists VIDAA and Vizio; Jellyfin none on VIDAA (#1615, 453 votes), Titan client in testing; Emby (unverified) | Medium: 453 votes for VIDAA | Later | Same packaged web build as CLI-010 where the platform's web runtime can carry it (unverified per platform) and passes the same secure-context test (SEC-CLI-070) | As CLI-010 | TV screens | SEC-CLI-070, SEC-CLI-024, SEC-TM-035 |
| CLI-012 | Roku through the Jellyfin adapter | Use Jellyfin's Roku app against a Gunmetal server | Plex yes; Jellyfin official Roku app (3.1.9); Emby yes | Medium: Roku coverage is expected and no first-party path is planned | Later | Coverage only: Roku users get Jellyfin's app, with more remuxes, transcodes and burn-in than native Gunmetal clients. The edge is that adapter tokens are scoped to read and play and they expire. The Roku signs in by Jellyfin Quick Connect, approved in a Gunmetal app after typing the code the Roku shows (SEC-EXT-071), over HTTPS only (SEC-EXT-066), and only once the owner has turned the adapter on (SEC-EXT-051); stream URLs it receives are Gunmetal's short-lived signed URLs, never its device token (SEC-EXT-074). | Jellyfin API subset; remuxed or HLS output Depends on INT-099 (Jellyfin adapter, video subset, Later) and VID-180. | Jellyfin's Roku app; listed under Settings > Devices | SEC-TM-070, SEC-EXT-051, SEC-EXT-066, SEC-EXT-070, SEC-EXT-071, SEC-EXT-074 |
| CLI-013 | Xbox | A console app | Plex yes; Jellyfin official app (0.9.5, May 2026); Emby yes (Premiere needed) | Low: no vote count found | Later | No edge claimed | As CLI-006 | TV screens | SEC-CLI-024, SEC-IAM-055, SEC-CLI-045 |
| CLI-014 | Desktop app for Windows, macOS and Linux | An installable app that direct plays any file | Plex desktop and Plex HTPC ("Are the Desktop Apps Dead?" thread, 96 posts); Jellyfin Desktop 2.0 (Qt 6, mpv); Emby limits free playback to one minute; Plexamp desktop rebuilt on Tauri | Medium: Plex desktop neglect thread; Feishin's top issue is offline (88) | Later | Later: the security baseline's release scope puts the desktop shell in Later (docs/security/threat-model.md, release scope), and SEC-CLI-069 sets its rules; until then desktops use the web client and the installable web app. Parity with Jellyfin Desktop on direct play (Qt plus mpv, free, MPRIS). The edge is the synced library, handoff and offline downloads shared with the phone apps, and a player that runs as its own sandboxed process with no network and no access to credentials (SEC-CLI-050). React Native has no first-party Linux desktop target (unverified), so the shell is an open decision (the security file recommends Tauri 2 with mpv in a separate sandboxed process; React Native Web in Tauri with libmpv in a native window is unverified) before Linux is promised. | As CLI-001 | All screens; mini player | SEC-CLI-069, SEC-CLI-050, SEC-CLI-030, SEC-CLI-046, SEC-CLI-052 |
| CLI-015 | Ten-foot mode on a PC | A couch interface on a home-theatre PC, driven by a controller | Plex HTPC (refresh-rate switching, controller, multichannel); Jellyfin Desktop TV mode and Emby Theater (unverified) | Low: no vote evidence | Later | The TV layouts already exist in the shared UI; the desktop shell hosts them full screen | None | TV layouts in the desktop app | SEC-CLI-069, SEC-CLI-024 |
| CLI-016 | Headsets | Watch in Vision Pro or a VR headset | Infuse has a native visionOS app; Jellyfin none (#891, 167 votes); Plex Meta Quest request (126) | Low: 167 and 126 votes | Later | No edge claimed | None | Headset app | SEC-IAM-048, SEC-CLI-030, SEC-CLI-045 |
| CLI-017 | Installs outside app stores | APK, F-Droid and sideloaded builds | Jellyfin F-Droid builds of official apps and Tizen 5 sideloading; Streamyfin Android TV by APK | Medium: de-Googled phones and older TVs | R2 | Every Android build plays, syncs and downloads without Google services; only casting is expected to need them (unverified). Release signing fingerprints are published so a sideloaded APK can be checked (SEC-CLI-046), and installed builds never download code at run time (SEC-CLI-045). | Signed release artefacts | Download page on gunmetal.tv | SEC-CLI-046, SEC-CLI-045, SEC-SUP-042, SEC-PRV-054 |
| CLI-018 | Several servers in one app | Your own server and a friend's, side by side | Plex yes (one account lists every server); Jellyfin request #47 (177 votes); Moonfin, Plezy and Infuse combine servers | Medium: 177 votes | R2 | The app holds a separate device key per server, each added by invite link, so no central account is needed to combine them Owns several-servers support; ACC-014 points here. Each server's credentials, library copy, caches and downloads sit in their own partition, and a merged Home only reads those partitions on the device: nothing from one server is ever sent to another (SEC-CLI-044, SEC-TM-061). | Invite redemption; per-server device keys | Server picker; Settings > Servers; optional merged Home | SEC-CLI-044, SEC-TM-061, SEC-CLI-043, SEC-IAM-051, SEC-IAM-081 |
| CLI-019 | PlayStation and Nintendo Switch | Console apps | Plex lists PS4 and PS5; Jellyfin none; Emby lists PS4 and PS3; Plex Switch request (320 votes) | Low: 320 votes for Switch | No | Not built; see Deliberately not doing | None | None | None (No) |
| CLI-020 | Native Linux phone builds | An app for postmarketOS-style phones | None | Low: 20 votes (#222) | No | The installable web app covers them | None | None | None (No) |
| CLI-021 | First-party Kodi add-on | Kodi as the front end | Jellyfin official JellyCon; Plex third-party add-on (unverified); Emby add-on | Low: the research rates it low priority | No | Gunmetal ships its own players; JellyCon may work through the Jellyfin adapter but is untested | None | None | SEC-TM-070 (if used through the adapter) |

### Connection, sync and shared client behaviour

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| CLI-022 | Library synced to the device | Every list, page and search answers from a local copy, with no spinner | Plex, Jellyfin and Emby fetch each screen; Plexamp 4.50 caches library data; Infuse Library Mode caches metadata | High: Jellyfin lazy loading (#216, 1,179 votes); offline is the third-ranked pain theme | R1 | Delta sync from the server's change log into an on-device SQLite store, through the same Rust core (WASM in browsers, UniFFI on native). Size and time budgets (rows, artwork bytes, index build time) are the DIS-019 design goals and are tests in the R1 gate on the reference device; only the published numbers wait for DIS-019 in R1.1 ([D-87](../decisions.md#d-87-speed-budgets-in-r1)). The server builds the copy per profile, so items a profile may not see never arrive (SEC-CLI-020). In a browser marked shared at sign-in (CLI-155) the copy is kept only in memory and is gone when the tab closes (SEC-CLI-010); in a personal browser it is partitioned per account and deleted at sign-out (SEC-IAM-017). | Per-user change log; delta and snapshot sync endpoints, filtered by profile | All browse and search screens | SEC-CLI-020, SEC-API-015, SEC-TM-026, SEC-CLI-021, SEC-IAM-017, SEC-CLI-010 |
| CLI-023 | Partial sync for small devices | Big libraries fit on TVs and in browsers: all metadata, artwork within a budget | None documented | Medium: TV and browser storage limits are flagged in three research files | R2 | Artwork is fetched within a budget and evicted least-recently-seen; missing art shows a placeholder, never a spinner Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Artwork in fixed sizes per device class | Settings > Storage; placeholders | SEC-CLI-020, SEC-MED-047, SEC-API-027, SEC-PRV-016 |
| CLI-024 | Sync and storage status | See what is synced, how much space each part uses and when it last synced | None documented | Low: no direct request; supports trust in offline use | R1.1 | One screen splits metadata, artwork and downloads, with "Sync now" and "Clear cache" | Sync cursor per device | Settings > Storage | SEC-CLI-009, SEC-IAM-017, SEC-CLI-010 |
| CLI-025 | Losing the server never blocks the app | With the server or network gone, the app still browses, searches and plays what it holds | Rivals need the server to browse; Plexamp gained an offline mode in Sept 2026 | High: people want the app to work when the server is gone (pain-points theme 3) | R1.1 | Reads are local, so an outage only removes streaming of items that are not downloaded. In R1 the web client needs the server reachable to load; a tab that is already open keeps browsing and searching its synced copy (CLI-022), and opening the app without the server waits for this row ([D-86](../decisions.md#d-86-loading-the-web-client-without-the-server)). From R1.1 this needs a personal browser on HTTPS or localhost (CLI-150, CLI-155): a service worker loads the app with the server down, and browse and search work, but nothing plays without downloads (R2). A browser marked shared keeps nothing once the tab closes (SEC-CLI-010), and over plain HTTP there is no web client at all, only a help page (SEC-NET-001). | Health endpoint that reports only liveness (SEC-NET-046) | Quiet status banner; dimmed items | SEC-NET-001, SEC-CLI-010, SEC-CLI-011, SEC-API-029, SEC-NET-046 |
| CLI-026 | Offline is not a separate mode | Offline uses the same screens, with a "Downloaded" filter instead of a separate Downloads view | Emby's separate offline view is called inconsistent; Plex's new app shows one ungrouped list; Plexamp has a dedicated offline library | Medium: complaints about Emby and Plex | R1.1 | Every item carries an availability state (streamable, downloaded, unavailable), and dimming and filters come from it. Until native downloads arrive in R2, the only availability states in a browser are streamable and unavailable; "downloaded" arrives with them. | None beyond sync | All lists; "Downloaded" filter | SEC-CLI-020, SEC-CLI-036 |
| CLI-027 | Pair a device by QR code | Scan the code on the TV with a signed-in phone and approve; no password typed with a remote | Plex link code; Jellyfin Quick Connect code, QR version planned (#2642, 225 votes); Emby PIN through Emby Connect | High: 225 votes | R2 | The TV generates its own non-exportable key and shows a QR code and a code; approving on the phone enrols that key for the TV, with no vendor service in the path (SEC-IAM-055). Scanning the QR is enough only when the phone and the TV are on the same home network; otherwise the person types the TV's code and confirms a matching code on both screens (SEC-IAM-060). The approval sheet marks the TV's name as unverified, says "In this home" or "Somewhere else", lists exactly what it will get and needs a fingerprint, face or device PIN check (SEC-IAM-058); a TV never gets administrator rights (SEC-IAM-059). Approval always starts on the phone, so nobody can push a prompt to it (SEC-STD-027). (Protocol owned by the accounts map.) | Device authorisation bound to the TV's own key; short-lived codes; rate limits | TV sign-in screen; phone approval sheet with matching code | SEC-IAM-055, SEC-IAM-057, SEC-IAM-058, SEC-IAM-059, SEC-IAM-060, SEC-STD-027 |
| CLI-028 | Custom address and proxy-friendly connections | Point the app at your own HTTPS address; works behind Authelia or Cloudflare Access with custom headers or client certificates | Plex removed custom server URLs from its new apps; Jellyfin supports them; Streamyfin custom headers; Finamp and Symfonium client certificates | Medium: Plex's removal was criticised | R2 | Each server entry stores its addresses, the iroh node ID and identity key pinned from its invite, and optional proxy headers or a client certificate; nothing forces a vendor broker. A custom address adds a route to a server the app already joined by invite; it never enrols a server by itself (SEC-CLI-043). Only HTTPS with normal certificate checks is accepted, and a self-signed server only through the fingerprint carried in its invite, never through a "trust anyway" button (SEC-CLI-042, SEC-NET-061). Custom headers and certificate keys are credentials, so they live in the platform keystore (SEC-CLI-030). A proxy's own sign-in is an extra gate in front of the server: the app still signs in to Gunmetal with its device key, and the server never treats a proxy's identity headers as sign-in (SEC-NET-023). | None | Add-server screen; Settings > Servers | SEC-CLI-042, SEC-CLI-043, SEC-NET-061, SEC-CLI-030, SEC-NET-023 |
| CLI-029 | Automatic home and away switching | The fast local path at home and the remote path elsewhere, with no setting | Plex automatic (unverified); Streamyfin local auto-switch (0.54.1) | Medium: Streamyfin added it on request | R2 | iroh prefers a direct path when one exists, so the same identity works at home and away. Native clients embed iroh; the project's own connection-success figures will replace iroh's quoted rate of about 90% direct (unverified for mobile carriers and TVs). No open port; whether there is no fee behind CGNAT depends on the relay decision (ACC-101). Browsers still need the owner's proxy or VPN until ACC-102. Being at home grants nothing extra (SEC-IAM-013), and a household TV that turns up on a remote path is suspended until an adult approves it (SEC-IAM-109). The connection indicator says whether the path is direct or relayed and who runs the relay (SEC-NET-039; ACC-105 owns connection status). | iroh endpoint | Connection indicator (direct or relayed, and the relay operator) | SEC-IAM-013, SEC-NET-039, SEC-NET-037, SEC-TM-063, SEC-IAM-109 |
| CLI-030 | Settings that follow you | Set preferences once and every device follows, with "this device only" where it makes sense | Jellyfin only through plugins (Streamyfin, Moonfin); Plex account-level (unverified) | Medium: Moonfin needs two server plugins for this | R1.1 | Settings are synced user data with a user scope or a device scope (audio output, storage, quality). Privacy settings start at their most private value, and no update or new device changes them (SEC-PRV-023). | User and device settings records, kept outside the rebuildable cache | Settings, with a "this device" marker | SEC-PRV-023, SEC-STD-012, SEC-IAM-072, SEC-TM-024 |
| CLI-031 | A layout contract | The player bar, queue, device picker, scrubber and navigation stay put; changes arrive as an opt-in preview with a way back | Plex reverted its TV navigation in Aug 2026; Spotify, Tidal and YouTube Music redesigns drew backlash | High: Plex rollback vote (504); Plex New Experience feedback thread (1,531 posts) | R1 | Visual regression tests pin core controls; layout changes ship behind a preview switch with before-and-after screenshots in the release notes. The preview switch is a local setting inside the signed release; nothing is switched on remotely (SEC-TM-067). | None | Settings > Preview features; release notes | SEC-TM-067, SEC-CLI-045, SEC-CLI-012 |
| CLI-032 | Old clients keep working | A server update does not strand a TV app you cannot update yet | Jellyfin 12 removed legacy endpoints and broke very old clients; Findroid 1.0 needs Jellyfin 10.11 or newer | Medium: Jellyfin 12 breakage | R2 | Protocol types are versioned in the shared core; the server keeps the previous protocol version for native apps and tells older clients plainly when to update. A version the signed advisory feed lists as insecure, or one below the admin's minimum, gets "Update required" instead of service (SEC-CLI-065), and a web tab always reloads to the server's own build (SEC-CLI-011). R2, with the native apps: before then the only client is the web client, which never runs an older build. | Protocol version negotiation; minimum-version and advisory checks | "Update needed" notice | SEC-CLI-011, SEC-CLI-065, SEC-API-092, SEC-HIS-007 |
| CLI-033 | Diagnostics you can read first | Export a sync, download or playback report that shows exactly what it contains; nothing is sent automatically | Plexamp 4.50 sends download telemetry with titles and the account name, with no off switch | Medium: Plexamp backlash; privacy is the second reason people self-host | R1.2 | The report is built on the device, redacted by default, shown in full, and saved as a file the user chooses to send; nothing is sent automatically (SEC-TM-053, SEC-CLI-027) | The user's own request records only, with addresses and titles replaced by pseudonyms as in SEC-PRV-046; a server diagnostic bundle is for admins only (SEC-OPS-030) | Settings > Help > Diagnostics | SEC-TM-053, SEC-CLI-027, SEC-PRV-046, SEC-OPS-030, SEC-IAM-047, SEC-CLI-060 |
| CLI-034 | Deep links | A link to an album, film or playlist opens it in the app, or in the web client | Spotify yes; Infuse deep links by TMDB ID; others (unverified) | Low: no vote evidence | R1.2 | Links carry only a random object ID; opening one still needs a signed-in device, so a link never grants access. Every link is read by one parser into a closed set of routes, and a link that would join a server or approve a device opens a confirmation screen that names the server and says what will happen (SEC-CLI-025, SEC-CLI-013). | Stable random object IDs | Share sheet; OS app links | SEC-CLI-025, SEC-CLI-039, SEC-CLI-013, SEC-API-023, SEC-API-011 |
| CLI-150 | Secure context in R1 | If the address is not secure, a help page says plainly how to reach the secure one; on HTTPS or localhost every web feature works | Every web media server has the same browser rule; none explains it in the UI (unverified) | High: basic expectation | R1 | Browsers allow service workers, installation (CLI-003), offline loading (CLI-025), passkeys (ACC-050), Web Crypto and reliable persistent storage only over HTTPS or on localhost. So there is no plain-HTTP player: over plain HTTP the server gives every peer except loopback a static help page with no sign-in and no cookie, explaining how to reach the HTTPS address (SEC-NET-001), and there is no password to fall back to (SEC-IAM-025). A browser that cannot use a passkey is signed in by approval from the person's phone (SEC-IAM-108). In R1, HTTPS comes from the owner's own domain with automatic certificates, a tailnet name, or localhost on the same machine (SEC-NET-013; ADM-021, ADM-022, ACC-099), and from away the web client is reached through the owner's reverse proxy or the tailnet (ACC-097). The project's per-server name service (SEC-NET-010, ADM-023) is not in R1; it arrives in R2 with built-in remote access. | Static help page on plain HTTP; HTTPS through the owner's domain, a tailnet or localhost (the name service from R2) | Plain-HTTP help page; Settings > About this connection | SEC-NET-001, SEC-IAM-025, SEC-IAM-108, SEC-NET-013, SEC-API-052 |
| CLI-155 | Personal or shared browser | Say whether this is your own browser; on a shared or public computer nothing stays behind | Not covered by the research for any rival | Low: no vote data; required by the security baseline (SEC-CLI-010) | R1 | At sign-in the web client asks one plain question with two explicit answers and no preselection. In shared mode the library copy, artwork and queue live only in memory, the cookie ends with the browser session, and the session ends after 30 minutes idle (SEC-CLI-010); Activity data never touches browser storage (SEC-PRV-019), and the browser is a limited-class device (SEC-CLI-024). In personal mode the library copy is stored, partitioned per account and deleted at sign-out (SEC-IAM-017). Session lifetimes are ACC-079. | Session cookie with no expiry in shared mode; shared-mode idle timeout; device class | Sign-in question; Settings > This browser | SEC-CLI-010, SEC-PRV-019, SEC-PRV-023, SEC-CLI-024, SEC-IAM-017 |
| CLI-156 | Signing out leaves nothing behind | Sign out, switch account or lose a device, and it keeps none of your library, history or downloads | Not covered by the research for any rival | Low: no vote data; required by the security baseline (SEC-CLI-009, SEC-CLI-037) | R1 | In the browser, signing out, switching account or a "session revoked" answer deletes this account's library copy, caches and storage and removes the service worker, even when the server cannot be reached (SEC-CLI-009, SEC-API-039). From R2 native apps offer two actions: "Switch account", which keeps that account's data locked under a key that works only after it signs in again, and "Remove account from this device", which deletes credentials, the library copy, artwork and downloads; a device the server reports as revoked runs the same removal before showing anything else (SEC-CLI-037, SEC-IAM-053). Revoking from another device is ACC-069. | Typed "session revoked" error; Clear-Site-Data on sign-out | Sign out; Switch account; Remove account from this device | SEC-CLI-009, SEC-API-039, SEC-CLI-037, SEC-IAM-053, SEC-PRV-057, SEC-TM-028 |
| CLI-157 | Private session on every device | See ACC-117, which owns this feature. Device specifics: one or two taps from the player on every client, and nothing reaches the device's own system surfaces | None among servers (ACC-117) | Medium: see ACC-117 | R1 | See ACC-117. Every client puts "Private session" within two interactions of the player: the web client in R1, the native, TV and car apps from R2 (SEC-PRV-024). A private play is never offered to Play Next, Top Shelf, Spotlight, assistants or recents lists (SEC-PRV-058), never queued for offline merge (CLI-093) and never kept by "Keep what I played" (CLI-081). | None beyond ACC-117 | Player menu and private indicator on every client | SEC-PRV-024, SEC-PRV-058, SEC-TM-054 |
| CLI-158 | A warning when a server is not the one you joined | If anything pretends to be your server, the app stops and says so | Not covered by the research for any rival | Low: no vote data; required by the security baseline (SEC-CLI-043) | R2 | Each app pins the server's identity key from the invite or pairing, and on every connection, over iroh or HTTPS, the server signs a fresh challenge with it (SEC-CLI-043, SEC-IAM-051). On a mismatch the app stops with a plain explanation and no "continue anyway"; a real key change is accepted only with a statement signed by the old key (SEC-NET-060, SEC-NET-062). A self-signed server is trusted only through the fingerprint in its invite, never by a click-through (SEC-CLI-042). | Identity key in invites; challenge signing; signed key-rotation statement | Full-screen "This is not your server" notice; Settings > Servers > Server identity | SEC-CLI-043, SEC-NET-060, SEC-IAM-051, SEC-TM-063, SEC-CLI-042 |
| CLI-159 | Outside links say where they go | A link from a tag or a provider page shows its destination before it opens; on a TV it becomes a QR code | Not covered by the research for any rival | Low: no vote data; required by the security baseline (SEC-STD-015) | R1 | Only https and http links with a real host are shown as links; anything else is plain text (SEC-API-047, SEC-CLI-002). Opening one shows a sheet naming the destination host with a cancel button, and the page opens without learning the Gunmetal address (SEC-STD-015, SEC-PRV-018). From R2, TV apps show a QR code instead of navigating. | None | Link sheet on every client; QR sheet on TV | SEC-STD-015, SEC-API-047, SEC-CLI-002, SEC-PRV-018 |

### TV (ten-foot)

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| CLI-035 | A left navigation rail that stays | Press left to reach libraries, search and settings, in the same place every release | Plex removed it and restored it in an Aug 2026 preview; Jellyfin Android TV left navigation (unverified); Netflix uses a top bar | High: Plex reversed after months of complaints | R2 | The rail's position is part of the layout contract (CLI-031) and pinned by tests | None | TV rail | SEC-CLI-020, SEC-CLI-001 |
| CLI-036 | Predictable focus with memory | Focus lands where expected, and Back returns to the same row and tile | Plex's Aug 2026 update lists focus fixes; others (unverified) | High: focus bugs are the most common ten-foot complaint (unverified as a measured claim) | R2 | Focus-map tests drive the D-pad through every screen on Apple TV and Android TV and assert where focus lands | None | All TV screens | None specific |
| CLI-037 | Two presses to any library | Libraries within two presses, with favourites pinned to the rail | Plex's Fire TV redesign went from about 2 to about 6 clicks, and pins were added in Aug 2026; Jellyfin home sections request (#1986, 169 votes) | Medium: 169 votes; Fire TV complaints | R2 | Pins and Home sections are synced user data, so they survive reinstalls and updates | Home layout records | TV rail; Pin action | SEC-TM-024, SEC-CLI-020 |
| CLI-038 | Smooth on a cheap stick | No lag when scrolling large poster grids | Plex Fire TV users reported lag and 30-second local starts in 2026 | High: Fire TV complaints; Gunmetal targets cheap hardware | R2 | Rows read from the local store, grids are virtualised, and time to interactive and scroll frame time are tests on the cheapest supported device Budgets: DIS-019. | Artwork in small TV sizes | All TV grids | SEC-TM-035, SEC-MED-047, SEC-CLI-021 |
| CLI-039 | A long-press menu on every card | Mark played, add to a playlist, go to the artist or remove from Continue without opening the item | Plex yes; Jellyfin partial; Infuse context menus for all three servers | Medium: Infuse shipped it on request | R2 | One shared action list drives TV, phone and web menus, so actions match everywhere. The server checks every action for the profile and device class, so a menu that offers more than it should still grants nothing (SEC-CLI-015, SEC-CLI-024). | Same operations as other clients | TV cards | SEC-CLI-015, SEC-CLI-024, SEC-API-012 |
| CLI-040 | Letter jump in long lists | Jump to a letter in long grids and lists | Spotify lacks an alphabet bar (884 votes); Swiftfin letter picker (1.5); Infuse | Medium: 884 votes | R1.1 | Built from the local index, so the jump is instant at any library size; arrives in the web client first, then phone, TV and car TV and remote part of DIS-101, which owns alphabet jump. | Sort keys in the sync payload | Web lists; phone fast scroller; TV letter column; car lists | SEC-CLI-020, SEC-API-048 |
| CLI-041 | Music on the big screen | Artwork-led Now Playing, a visible queue and a dim ambient mode | Spotify TV (queue and dim mode since 2023); Caldera TV on Apple TV and Android TV; no native TV Plexamp | Medium: Plexamp users ask for Google TV support | R2 | The TV joins the same queue as every other device, so it can take over playback or be driven from a phone. Only your own devices can take over or drive your queue (SEC-HIS-014), and on a household TV a profile's queue and history appear only after that profile is unlocked for the session (SEC-IAM-110). | Queue sync; session registry | TV Now Playing; TV queue; ambient mode | SEC-HIS-014, SEC-API-016, SEC-IAM-110, SEC-CLI-063 |
| CLI-042 | Screensaver from your library | Ambient artwork when idle, safe for OLED | Jellyfin OLED-friendly screensaver; Plex (unverified) | Low: one completed Jellyfin request | R2 | Draws from the local artwork cache, which only holds what the profile may see | None | TV screensaver | SEC-CLI-020, SEC-CLI-063, SEC-MED-046 |
| CLI-043 | Text size on TV | Larger text and spacing for reading from the sofa | Emby font size options; Jellyfin request (10 votes) | Low: 10 votes | R2 | Type and spacing are design tokens, so one setting scales everything and each step is tested | None | Settings > Display | None specific |
| CLI-044 | Voice search from the remote | Speak a search into the remote | Jellyfin Android TV 0.19 native voice search; others (unverified) | Medium: Jellyfin shipped it in 2026 | R2 | The recognised text runs against the local, typo-tolerant index, so results appear without a server round trip. The search text stays on the device and is never stored by the server (SEC-PRV-004). | None | TV search | SEC-PRV-004, SEC-CLI-063, SEC-STD-011 |
| CLI-045 | Remote and controller keys | Dedicated play, pause, rewind and fast-forward keys; game controllers | Jellyfin added remote rewind and fast-forward in 12.0; Plex HTPC controller support | Low: no vote evidence | R2 | Parity | None | Player; all TV screens | None specific |
| CLI-046 | Frame-rate and resolution matching | Films play at their own frame rate, without judder | Plex HTPC; Wholphin; Jellyfin Android TV forces PCM audio when it switches (#4067); Apple TV relies on Match Content | Medium: a known Jellyfin side effect | R2 | The player module switches display mode independently of audio routing, so matching never breaks passthrough | Frame rate recorded at scan time | Settings > Playback | SEC-TM-031 |
| CLI-047 | An honest device capability report | The app knows which codecs, HDR formats and audio formats this device and its receiver take, and direct plays whenever the container policy allows | Jellyfin Android TV HDR issue open since 2019 (127 comments), passthrough since 2020 (50 comments), incomplete capabilities (#5316); Infuse best on Apple | High: long-running Jellyfin issues | R2 | The native player probes decoders and outputs and feeds the shared decision engine; a "This device" screen shows the result and the reason for each choice. Direct play is offered for every container the core's parser accepted at scan time; other containers are remuxed or transcoded on the server unless an admin allows direct play for that library (SEC-CLI-049). The device's report is untrusted input to the server and can only change what that device is sent (SEC-HIS-033). | Decision engine that takes device capability sets | Settings > This device; playback info | SEC-CLI-049, SEC-HIS-033, SEC-CLI-015, SEC-TM-031 |
| CLI-048 | Audio output settings per device | Passthrough, channel layout and lip-sync delay saved for this TV and receiver | Infuse (Atmos passthrough with Pro); Jellyfin Shield passthrough issue; Plex lip-sync only on some desktop apps (59 votes since 2014) | Medium: 59 votes; long-open issues | R2 | Stored with device scope (CLI-030), so a soundbar's lag stays with the soundbar | None | Settings > Audio output | None specific |
| CLI-049 | TV home-screen rows | Continue watching on Android TV's Play Next row and on Apple TV's Top Shelf, when the household chooses it | Jellyfin Android TV (completed); Streamyfin Top Shelf and recommendations; Plex (unverified) | Medium: completed Jellyfin requests | R2 | Filled from the synced watch log with no server call; a dismissal also removes the item from the system row. These rows are seen by everyone in the room and by the launcher's vendor, so the TV asks once at pairing ("Show continue-watching on the TV home screen?"), preselecting yes only on a single-profile TV and no when several profiles use it; elsewhere it is opt-in per profile (SEC-CLI-061). Restricted and PIN-protected profiles and private sessions never appear there (SEC-CLI-061, SEC-PRV-058). | None | Pairing question; Android TV launcher; Apple TV Top Shelf; Settings > Privacy | SEC-CLI-061, SEC-PRV-058, SEC-CLI-063, SEC-PRV-023 |
| CLI-050 | Profile picker on shared TVs | Each person picks a profile at launch, with an optional PIN | Plex Home; Jellyfin account switching started (#2353, 154 votes); tvOS 26 profiles | Medium: 154 votes | R2 | Each profile's data sits in its own partition on the TV, and the server builds each partition with only what that profile may see, so a child's copy never contains restricted titles, artwork or search terms: they are absent, not hidden (SEC-CLI-020, SEC-CLI-063). The picker shows names and avatars only (SEC-CLI-064, SEC-IAM-061). PINs are checked by the server only (SEC-IAM-062); a PIN-protected profile's data is stored only encrypted under a key the server releases after the PIN and the TV forgets on switching away (SEC-IAM-065). An adult profile's history stays hidden on a household TV until a PIN or that adult's phone unlocks it, and adding an adult profile preselects "Add a PIN" (SEC-IAM-110). An unprotected adult profile's library copy can still be read by someone with developer access to the TV, and the docs say so (profile model owned by the users map) | Per-profile sync feeds | TV profile picker | SEC-CLI-063, SEC-CLI-064, SEC-IAM-061, SEC-IAM-062, SEC-IAM-065, SEC-IAM-110 |

### Phone and tablet

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| CLI-051 | Music and video in one app | One app, opening to music if that is what you use, with a music-only mode | Plex split music into Plexamp; Jellyfin one app; Emby one app ("Standalone Music App" request, 175 replies) | Medium: opinions are split | R2 | A music-forward Home and an option to hide video libraries, with no second app to install (open decision 8) | None | Home; Settings > Libraries shown | None specific |
| CLI-052 | Full library management on the phone | Add to playlists, reorder, rate and favourite without a computer | Plex mobile could not add to playlists (Jan 2026); Jellyfin yes (web UI); Finamp playlist editing | Medium: Plex "Better Playlists" request (1,425 votes) | R2 | The same operations on every client; edits made offline queue and merge (CLI-094) | Playlist and rating operations | Context menus; playlist editor | SEC-TM-024, SEC-API-012, SEC-IAM-072, SEC-CLI-015 |
| CLI-053 | Tablet layout | A side panel for browsing while the player stays visible, reflowing on rotation | Spotify's tablet app (Apr 2026); Finamp uses its phone layout on tablets (unverified) | Medium: Spotify built one in 2026 | R2 | Layouts per size class over shared components | None | Tablet layouts | None specific |
| CLI-054 | Landscape Now Playing | Artwork and controls side by side on a docked phone, with lyrics beside the art | Apple Music (iOS 27) | Low: new in 2026 | R2 | Parity | None | Now Playing | SEC-MED-049, SEC-CLI-001 |
| CLI-055 | Player gestures | Swipe to seek and change brightness or volume; double-tap to skip | Jellyfin Android 2.7 seek gestures; Streamyfin; Plex's new app lost double-tap; Jellyfin double-tap request (#131, 109 votes) | Medium: 109 votes | R2 | Every gesture has a visible button equivalent (CLI-142) | None | Video player | None specific |
| CLI-153 | Configurable tap and swipe gestures | Choose what a tap, long tap or double tap does | Marvis Pro maps taps and swipes (no vote count in the research) | Low: no vote count | Later | Gestures are actions on the one queue object, so a mapping is a per-device setting | None | Settings > Gestures | None specific |
| CLI-056 | Picture-in-picture | Keep watching in a floating window | Plex unreliable after its redesign; Jellyfin Android yes; Findroid; Streamyfin | Medium: Plex complaints | R2 | Uses the system picture-in-picture window (hand-off of the libmpv surface per platform unverified). It keeps the player's process isolation where the platform has it (SEC-CLI-050). | None | Video player | SEC-CLI-050 |
| CLI-057 | External display | Play on a cabled TV from the phone | Swiftfin wide-ratio output; Jellyfin Android request (#2060) | Low: one open request | Later | Parity. The player never blocks screen capture, so mirroring and external displays work (SEC-CLI-057). | None | Video player | SEC-CLI-057 |
| CLI-058 | Admin on the phone | See sessions, start a scan and manage users from the phone app | Swiftfin admin dashboard and backups; Plex dashboard on mobile and TV (Jul 2026); Streamyfin sessions view | Medium: Plex shipped it in 2026 | R2 | Admin screens are part of the shared UI, so they reach phone and web together (content owned by the operations map). They open only on personal-class devices (a phone with a lock screen, or a browser in personal mode with a passkey); TVs and other limited devices never show them and the server refuses them there (SEC-CLI-024). Each admin session needs a fingerprint, face or device PIN check bound to the device key (SEC-CLI-059, SEC-IAM-041), and is refused on internet-posture paths unless the owner allowed remote administration (SEC-NET-045). The sessions view shows who is playing on which device, never the title unless that person chose to show it (SEC-PRV-025). | Admin API under the same authorisation layer, with device-class and fresh-verification checks | Admin section (phone and web only) | SEC-CLI-024, SEC-IAM-041, SEC-CLI-059, SEC-NET-045, SEC-PRV-025 |
| CLI-059 | Choose where downloads go | Store downloads on an SD card or external storage, encrypted so only this device's app can play them | Plexamp allowed external storage (2020); request (31 posts) | Low: 31 posts | Later | Later: in R2 downloads stay in app-private internal storage (SEC-CLI-035). Removable storage is allowed only once downloads there are encrypted in seekable, authenticated chunks under a key in the platform keystore (SEC-CLI-072), so a lost SD card reveals nothing. Owns download location; MUS-216 points here. | None | Settings > Downloads | SEC-CLI-035, SEC-CLI-072, SEC-CLI-033 |

### Desktop and web

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| CLI-060 | Wide three-pane layout | Library, content and a full-height queue visible together, with resizable panes | Spotify set the pattern (and squeezed its queue in 2024); Feishin copies it; Jellyfin web has no persistent queue (unverified) | Medium: Spotify's 2024 queue complaints | R1 | The queue pane can grow to full height and keeps durations and album names (owned with the music map) | None | Web and desktop layout | SEC-CLI-001 |
| CLI-149 | Phone-width web layout | The web client and installed web app work one-handed on a phone: bottom navigation, a now-playing bar, a full-screen player and a queue sheet, with no horizontal scroll | Plex, Jellyfin and Navidrome web clients are usable on phones (unverified per client) | High: basic expectation | R1 | The only phone experience in R1, so it is designed first rather than shrunk from the desktop layout | None beyond CLI-001 | All screens at 360 to 430 px wide | SEC-CLI-001, SEC-API-052 |
| CLI-061 | Keyboard shortcuts and command palette | Play, seek, queue next, like and search without the mouse | Jellyfin shortcuts and frame stepping (12.0); Spotify hotkey to queue (988 votes); Finamp desktop shortcuts | Medium: 988 votes | R2 | One action list drives shortcuts, menus and the palette, so every action is reachable by keyboard Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | None | Command palette; shortcut help | SEC-CLI-015, SEC-PRV-004 |
| CLI-062 | Multi-select, drag and right-click | Act on many tracks at once; drag to the queue or a playlist | Spotify yes; Jellyfin web playlist multi-select is limited (open issue); Marvis Pro queue drop zones | Medium: open Jellyfin issue | R1.1 | Drop zones for "play next" and "play last"; batch operations Desktop input part of DIS-110, which owns multi-select. | Batch operations | Lists; queue; sidebar | SEC-API-012, SEC-API-063, SEC-TM-068 |
| CLI-063 | Desktop system media panels | Media keys, MPRIS on Linux, Windows media controls and macOS Now Playing in the desktop app | Jellyfin Desktop MPRIS; Jellyfin web works only partly with KDE (#5208); Navidrome's reported state drifts (#4744) | Medium: open issues on two projects | Later | Native APIs in the desktop shell (CLI-014, Later with the shell); state comes from the one queue object, with tests that the system panel matches the player. Owns desktop panels; MUS-081 points here. The R1 browser part is CLI-070 (with MUS-073 for music specifics). | None | System media panel; media keys | SEC-CLI-069, SEC-PRV-058 |
| CLI-064 | Global hotkeys | Control playback from any app | Spotify and Plexamp (scope unverified) | Low: no evidence beyond in-app hotkeys | Later | The shell maps global shortcuts onto the shared action list. Arrives with the desktop shell (CLI-014, Later). | None | Settings > Shortcuts | SEC-CLI-069 |
| CLI-065 | Mini player window | A small always-on-top player | Apple MiniPlayer; Feishin request (32 votes) | Low: 32 votes | Later | A second window of the same app on the same queue state Owns the mini player; MUS-082 points here. Arrives with the desktop shell (CLI-014, Later). | None | Mini player | SEC-CLI-069 |
| CLI-066 | A light desktop shell | Small download, low memory | Plexamp moved from Electron to Tauri for size and memory (Sept 2026) | Medium: a direct signal from Plexamp | Later | The shell is chosen by benchmark, and idle memory is published like scan time. The chosen shell must also meet SEC-CLI-069 (bundled content only, isolated renderers, checked IPC), and a Chromium-based shell must ship browser security fixes within the client patch windows (SEC-CLI-052). | None | None | SEC-CLI-069, SEC-CLI-017, SEC-CLI-052 |
| CLI-067 | Pick and keep the output device | Choose speakers, headphones or a DAC, and keep that choice | Spotify request (5,509 votes); Plexamp and Roon output selection; browsers decide for web clients | High: 5,509 votes | R2 | Output is a device-scoped setting (CLI-030); the bit-perfect path is in the music map Owns output device choice; MUS-200 points here. On desktop it arrives with the shell (CLI-014, Later). | None | Output picker; device picker | None specific |
| CLI-068 | Respect the operating system | The app never launches on a play key or headphone connection unless asked | Apple's Mac Music app does this; a tool to stop it reached 669 points on Hacker News | Medium: 669 points on Hacker News | R2 | Autostart and media-key ownership are opt-in. On Android, letting the play key or a headphone connection resume Gunmetal (media resumption) is an OS-wide surface, so it stays off until the person turns it on (SEC-CLI-061); on desktop the same switches arrive with the shell (CLI-014, Later) | None | Settings > System | SEC-CLI-061, SEC-CLI-054 |

### Background playback and system integration

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| CLI-069 | Background audio with lock-screen controls | Music plays with the screen off, with title, art and controls on the lock screen, notification, Bluetooth displays and watch | Plexamp, Symfonium and Finamp yes; Plex and Jellyfin apps yes | High: a basic expectation; ADR 2 requires it | R2 | Parity. Lock-screen media controls show the title; other notifications do not (SEC-CLI-062), and only system UI, Android Auto and named wearables may browse the library (SEC-CLI-055). | None | Lock screen; notification; Bluetooth displays | SEC-CLI-062, SEC-CLI-055, SEC-CLI-054, SEC-PRV-058 |
| CLI-070 | Lock-screen controls from the web app | Media Session controls when playing in a browser or the installed web app | Navidrome via Media Session (state drift bug); Jellyfin web (unverified) | Medium: open Navidrome bug | R1 | State is derived from the queue object and tested; iOS Safari's background limits are stated plainly. Owns browser media controls in R1; MUS-073 is the music specifics. The web client exists only over HTTPS or on localhost (SEC-NET-001), so these controls work wherever the web client runs, as far as each browser allows (unverified per browser). | None | Lock screen; notification | SEC-NET-001, SEC-API-044, SEC-PRV-024 |
| CLI-071 | Interruptions handled cleanly | Calls and navigation prompts pause or lower the music and then resume; unplugging headphones pauses | Jellyfin Android added handling in 2.7.2; others (unverified) | Medium: Jellyfin fixed it in 2026 | R2 | Parity, with a per-platform test list | None | None | None specific |
| CLI-072 | Audio carries on for video | The soundtrack keeps playing when the app is backgrounded; an audio-only mode for concerts | Swiftfin pauses video in the background; Jellyfin 12.0 fixed iOS background audio; audio-only request (#533, 34 votes) | Low: 34 votes | R2 | Audio-only asks the remuxer for just the audio track, so the phone stops fetching video | Audio-only remux output | Player menu > Audio only | SEC-MED-081, SEC-NET-053, SEC-API-026 |
| CLI-073 | Home-screen widgets | Now playing and quick starts from the phone home screen | Symfonium adaptive widgets; Plexamp Android widget; Finamp request (19 votes); Apple Music widgets (iOS 27) | Medium: Plexamp built one on request | Later | The widget reads the local store, so it shows recent items without a network. It shows only the active profile's items and nothing from private sessions (SEC-CLI-063, SEC-PRV-058). | None | Android and iOS widgets | SEC-PRV-058, SEC-CLI-063, SEC-CLI-054 |
| CLI-074 | Phone voice assistants | "Play my running playlist" through Siri or Google Assistant | Plexamp Siri; Finamp Siri (beta); Jellyfin Android Auto voice | Medium: several apps added it in 2026 | Later | Requests resolve against the local library, so they work offline. Sharing playlist names and plays with the assistant is an OS-wide surface, so it is off until the person turns it on for their profile (SEC-CLI-061), and private sessions are never shared (SEC-PRV-058). | None | System assistants | SEC-CLI-061, SEC-PRV-058, SEC-PRV-004 |
| CLI-075 | NFC tags | Tap a tag to start an album or playlist | Plexamp (free) | Low: no vote evidence | Later | The tag holds only an object ID; playback still needs a signed-in device. The tag is read by the one link parser like any other link (SEC-CLI-025). | None | Context menu > Write to tag | SEC-CLI-025, SEC-API-023 |
| CLI-076 | Local notifications | "Downloads finished" or "Sync paused: storage full", with no push service | Plexamp added specific download errors (4.50); others (unverified) | Low: no vote evidence | R2 | Raised on the device, so no vendor push relay is needed. On the lock screen they name no titles, and security notices say only that something happened (SEC-CLI-062). | None | System notifications | SEC-CLI-062, SEC-PRV-030 |
| CLI-077 | Remote push notifications | Server events reach a phone while the app is closed | Plex (unverified); others (unverified) | Low: no vote evidence | Later | Needs Apple's and Google's push services, which sit uneasily with no central account (open decision 6). If it is ever built, the project runs no relay that holds device push tokens or learns which server a phone uses (SEC-HIS-061): the relay is one the owner runs, and each push carries only an opaque event ID that wakes the app to fetch the event from its own server (SEC-PRV-056, SEC-OPS-036) | Owner-run push relay; opaque event IDs | System notifications | SEC-HIS-061, SEC-PRV-056, SEC-OPS-036, SEC-OPS-035 |

### Offline: downloads and sync

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| CLI-078 | Download music | Albums, artists, playlists and tracks with no signal, free, in the native apps | Plexamp needs Plex Pass; Finamp free; Symfonium one-time purchase; Emby Premiere; no rival downloads in a browser (Jellyfin #3012, 45 votes; Feishin's top issue, 88) | High: paywalls on Plex and Emby; Jellyfin and Feishin requests | R2 | Free and account-free; downloaded items stay in the normal library with a badge. Owns music downloads; MUS-210 points here. Downloads inside the browser are MUS-217 (Later), because browser storage limits, eviction and the secure-context requirement (CLI-150) make them fragile. The server enforces each person's download permission (SEC-CLI-015), and guests cannot download unless their invitation allows it (SEC-IAM-080). | Signed byte-range URLs; offline grants | Download toggle on album, artist and playlist pages; Downloads | SEC-CLI-036, SEC-IAM-054, SEC-CLI-035, SEC-TM-060, SEC-IAM-080, SEC-API-026 |
| CLI-079 | Download the whole music library | Carry everything | Plexamp only since Sept 2026 (206-vote request from 2020); Finamp yes; Emby users report it missing | Medium: 206 votes | R2 | One rule ("all music") with a size estimate before it starts | Size totals per rule | Downloads > Rules | SEC-CLI-036, SEC-NET-053, SEC-TM-068 |
| CLI-080 | Downloads that follow rules | Keep a smart playlist, an artist, favourites or five-star tracks downloaded automatically | Symfonium automatic caching; Plexamp background refresh (Sept 2026); Emby lacks sync by artist, playlist or genre | High: Jellyfin "Offline Sync Feature" (817 votes); Emby complaint thread | R2 | Rules are smart-playlist queries in the shared core, evaluated on the device against the synced library, identically on every client and with the server off | Shared rule language; byte serving | Downloads > Rules; "Keep downloaded" on any smart playlist | SEC-STD-011, SEC-CLI-020, SEC-IAM-054 |
| CLI-081 | Keep what I played | Recently played music stays offline within a cap | Plexamp "Keep Played Music" (Sept 2026); Roon ARC Smart Downloads; Symfonium cache | Medium: two rivals shipped it recently | R2 | Driven by the local listening log, so it rotates with no server work | None | Downloads > Rules | SEC-PRV-024, SEC-CLI-063, SEC-CLI-036 |
| CLI-082 | Storage cap and eviction | Set a limit; automatic downloads make room, and what you chose by hand stays | Plexamp (a cap for Keep Played Music); others (unverified) | Medium: storage limits whole-library rules | R2 | Manual and rule-based downloads are tracked apart, so eviction never removes a manual choice | None | Settings > Storage | SEC-CLI-035 |
| CLI-083 | Smaller music downloads | Opus copies that use a fraction of the space | Plexamp (Opus); Finamp transcoded downloads; Symfonium for Wear OS | Medium: Finamp and Plexamp both offer it | R2 | Opus encoding is cheap on any hardware (ADR 2) and cached on the server, so a second device does not pay again. The encoder runs in the jailed worker (SEC-TM-044), and the cache lives in the data directory, never beside the originals (SEC-TM-042). | Opus encode jobs in the sandboxed worker; cache of encoded files | Download quality setting | SEC-TM-044, SEC-HIS-022, SEC-MED-067, SEC-NET-053, SEC-TM-042 |
| CLI-084 | Download films and episodes as they are | Original files for offline viewing | Plex Pass; Jellyfin Android downloads originals only (#218, 1,820 votes); Swiftfin none; Emby Premiere | High: 1,820 votes | R2 | libmpv plays the original, so the server only reads the disk; free. That holds for every container the core's parser accepted at scan time; other containers are offered as a remuxed download (CLI-085) unless an admin allows direct play for that library (SEC-CLI-049). | Signed byte-range URLs | Download button on films and episodes; Downloads | SEC-CLI-049, SEC-CLI-036, SEC-CLI-047, SEC-CLI-048 |
| CLI-085 | Remuxed video downloads | The same picture and sound in a container the device's player prefers | Not offered as a distinct option by rivals | Medium: part of the demand for compatible downloads | R2 | The server's remuxer, which runs in a worker process and streams its output (SEC-MED-081), produces the file losslessly and cheaply when the original container will not play or may not go to the player (SEC-CLI-049); it writes from the typed model only (SEC-MED-074), and any cached copy lives in the data directory, never beside the original (SEC-TM-042). | Remux job in a worker process, streamed to the device | Download options | SEC-MED-081, SEC-MED-074, SEC-TM-042, SEC-NET-053 |
| CLI-086 | Smaller transcoded video downloads | Fit a season on a phone | Plex yes; Jellyfin planned (#57, 518 votes); Emby Premiere (server conversion); Streamyfin saves an HLS transcode | High: 518 votes; pre-transcoding request (979 votes) | Later | Scheduled, sandboxed jobs with a size and time estimate shown first, cached for other devices (open decision 5) | Scheduled transcode jobs; output cache | Download options with estimate | SEC-TM-044, SEC-TM-047, SEC-MED-067, SEC-MED-072, SEC-API-064 |
| CLI-087 | Season, series and batch downloads | One action for a season or a show, optionally only unwatched episodes | Plex restored season and unwatched-only downloads (2025.18.0); Jellyfin batch download started (#655, 219 votes); Emby series and libraries | Medium: 219 votes | R2 | Expressed as a rule (CLI-080), so new episodes follow automatically | None | Season and show pages | SEC-CLI-036, SEC-NET-053 |
| CLI-088 | Keep the next episodes, drop watched ones | Downloads refill as you watch, and watched episodes delete themselves | Plex lost it in the 2025 rewrite and users still asked in Aug 2026; Emby sync rules; Plezy | High: repeated Plex threads; Jellyfin "Offline Sync Feature" (817 votes) | R2 | Evaluated on the device from the synced watch log, so it refills correctly even after watching offline | None | Show page > Keep next episodes | SEC-CLI-036, SEC-CLI-063 |
| CLI-089 | Downloads manager | Grouped by show and album, with progress percentages, multi-select delete and plain failure reasons | Plex shows one ungrouped list with no bulk delete; Jellyfin Android multi-select (2.7.0), iOS requests (#3955, #3957); Plexamp specific errors (4.50); Emby drill-down | Medium: complaints on Plex and Jellyfin | R2 | Every failure has a typed reason (storage, network rule, access withdrawn, server error) shown in plain words; a device that has been revoked is wiped instead (CLI-156) | Typed error responses | Downloads screen | SEC-API-072, SEC-TM-040, SEC-IAM-076 |
| CLI-090 | Background downloads that resume | Downloads continue with the app closed and survive dropped connections | Infuse (iOS background downloads with a Live Activity); Streamyfin; Plexamp paused when backgrounded (Sept 2026) | Medium: Plexamp complaints | R2 | The OS transfer services (background sessions on iOS, WorkManager on Android) with resumable byte ranges and progress in system UI. URLs handed to the OS transfer service are capability URLs for one item and one device that can be refreshed and revoked, never a session token (SEC-NET-064). | Range requests; refreshable URLs | Notification; Live Activity | SEC-NET-064, SEC-API-027, SEC-API-028 |
| CLI-091 | A mobile-data rule that is kept | Wi-Fi only, charging only, or mobile data allowed, and never broken silently | Plexamp downloaded over cellular while showing "paused" (Sept 2026, confirmed by its co-founder); Finamp requests | Medium: Plexamp bug thread | R2 | One native download module enforces the rule, with tests that simulate network changes | None | Settings > Downloads > Network | None specific |
| CLI-092 | Streaming quality per network | Originals on Wi-Fi, smaller streams on mobile data | Spotify quality settings; Finamp requests (20 and 7 reactions) | Medium: Finamp requests | R2 | Uses the same Opus path as downloads, and never downgrades on Wi-Fi unless asked | On-the-fly Opus encoding | Settings > Playback > Mobile data | SEC-TM-044, SEC-NET-053, SEC-TM-068 |
| CLI-093 | Offline plays and progress merge cleanly | What you played or watched offline appears in history and resume points later | Streamyfin syncs download progress (0.30.2); others (unverified) | High: the heart of offline use | R1 | In R1 the web client holds play events made while the connection drops in memory, never in browser storage (SEC-TM-058), and uploads them on reconnect; closing the tab while offline loses them. From R2 native downloads use the same path, with events kept in app-private storage. Each device appends to the log with its device ID and a hybrid logical clock, merged by one tested rule; the server validates every uploaded event as untrusted input (SEC-HIS-033), and private sessions produce no events (SEC-PRV-024). | Log upload with de-duplication | History (showing the device) | SEC-TM-058, SEC-PRV-002, SEC-PRV-024, SEC-HIS-033 |
| CLI-094 | Offline edits merge later | Playlist edits, ratings and queue changes made offline sync on return | Finamp request (#1065); Spotify (unverified) | Medium: open Finamp request | R2 | Edits are small operations replayed against the server's version, with stated conflict rules Owns offline edits; MUS-152 and MUS-214 point here. Each queued operation is authorised again when it is replayed, against the person's rights at that moment (SEC-TM-024). | Operation endpoint; conflict rules | Rare conflict notice | SEC-TM-024, SEC-API-012, SEC-IAM-072, SEC-CLI-021 |
| CLI-095 | Downloads that outlive the connection | Downloads keep playing when the server is down or you are abroad, for an admin-set period | Spotify requires going online every 30 days; rivals' rules (unverified) | Medium: travel and outage use | R2 | An offline grant: the device key plus a signed record of what it may play and until when, renewed on any contact (open decision 4). The default is 30 days and the admin range 1 to 90, measured by elapsed time so a clock set back does not extend it; expiry never stops a track already playing and never deletes files, and the screen says "Connect to your server once to keep listening offline" (SEC-CLI-036). | Grant issuance and renewal; revocation list | Expiry note on the Downloads screen | SEC-CLI-036, SEC-IAM-054, SEC-TM-060 |
| CLI-096 | See and revoke downloads per device | Each person sees which of their devices hold what and can revoke a lost phone; admins can revoke any device but see only counts | Emby (admins control all synced media); Jellyfin none | Low: Emby only | R2 | The server tracks grants per device. Each person sees their own devices and what each holds, and can revoke any of them (SEC-IAM-042); admins can revoke any device (SEC-IAM-044) but see only how many items and how much space each device holds, never which titles, because what someone downloads is their activity (SEC-PRV-025). Revocation bites on the device's next contact, and an unreachable device's downloads stop when its grant expires; the docs say so. | Grant registry | Account > Devices; Admin > Devices (counts only) | SEC-IAM-042, SEC-IAM-044, SEC-PRV-025, SEC-TM-060, SEC-IAM-076 |
| CLI-097 | Start a download from another device | Queue a phone download from the web while packing | Emby yes; Jellyfin no | Low: Emby only | R2 | Download intents are synced data, and the phone acts on them at its next sync; a person can send them only to their own devices, and the server applies their download permission (SEC-CLI-015) | Download intent records | Download menu > On device | SEC-TM-024, SEC-CLI-015, SEC-IAM-080 |
| CLI-098 | Extras travel with downloads | Lyrics, artwork, chapters, skip markers and previews work offline | Streamyfin keeps skip segments offline; Findroid stores images; Spotify lacks offline lyrics (1,600 votes) | Medium: 1,600 votes | R2 | Lyrics and markers are part of the synced library, so they are offline by default Owns offline extras; MUS-160 points here. | Lyrics and markers in the sync payload | Now Playing; video player | SEC-MED-049, SEC-API-090, SEC-CLI-020, SEC-MED-051 |
| CLI-099 | Fetch ahead on patchy signal | Upcoming tracks or the next episode load early | Plexamp advanced pre-caching (free); Jellyfin pre-buffer request (#400, 222 votes) | Medium: 222 votes | R1.1 | Byte ranges are known from the scan, so the client fetches exactly what comes next. Prefetched bytes are held in memory, never in Cache Storage, and each fetch re-checks its capability URL (SEC-API-029). | Byte-range serving | Settings > Playback > Fetch ahead | SEC-API-027, SEC-API-029, SEC-NET-050, SEC-API-031 |
| CLI-100 | Copy to a folder or drive | Carry chosen downloads on an external drive, playable by your Gunmetal app | Emby Folder Sync (Premiere) | Low: Emby only | Later | The server never writes outside its data directory (SEC-TM-042), and Gunmetal never writes playable media to removable storage in the clear (SEC-CLI-072). So this is not a server copy job: a Gunmetal app copies chosen downloads to the drive in the encrypted, seekable format of SEC-CLI-072, playable only by that app under its offline grant (CLI-059). People who want plain copies already have the files on the host | None beyond CLI-059 | Downloads > Copy to drive | SEC-TM-042, SEC-CLI-072, SEC-CLI-036 |

### Handoff, remote control and casting

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| CLI-101 | Device picker and handoff | See every Gunmetal player you own and move playback there with the same queue and position | Spotify Connect is the reference; Plexamp players; Jellyfin web plays on other sessions; Finamp "Play On" requested (18) | High: ADR 2 commitment; Jellyfin "like Spotify Connect" request (29 votes) | R2 | Owns device handoff; MUS-197 points here. Jellyfin can already remote-control its sessions with no vendor account; Gunmetal's edge is one versioned queue that any device can take over with the same position and lanes, offline-tolerant, which Spotify Connect does with an account and Plexamp only with plex.tv. Moves to R2 with the native clients: in R1 the only players are browser tabs. Commands are a closed set with no free text, only your own sessions can be driven (SEC-HIS-014), and every event is filtered for its recipient (SEC-API-016). | A WebSocket control channel per signed-in session; commands authorised per profile; server-assigned queue versions (MUS-122); a written and tested conflict rule. Must be designed before either CLI-101 or CLI-102 is built. | Device picker in the player bar and Now Playing | SEC-HIS-014, SEC-API-016, SEC-API-043, SEC-IAM-016, SEC-TM-024 |
| CLI-102 | Remote control another player | Drive the TV or desktop from your phone or laptop including the volume of the controlled device | Jellyfin can control any session; Swiftfin session commands (1.5); Plexamp remote control off the home network (2022) | Medium: Finamp request (18) | R2 | The control channel runs through the server (over iroh for native apps), so it works at home and away Covers your own devices only; controlling another person's player is ACC-047 (Later). Owns remote control and remote volume; MUS-198 points here. Symfonium sets the bar for per-speaker volume. | The CLI-101 control channel, with a volume command | Device picker; remote mode in Now Playing | SEC-HIS-014, SEC-API-043, SEC-TM-028 |
| CLI-103 | Continue on this device | Open the app elsewhere and it offers to pick up where you left off | Spotify (unverified); others (unverified) | Medium: implied by handoff demand | R1.1 | The queue and position are already in the local copy; the prompt is UI only. Ships in R1.1, ahead of handoff: it needs only the R1 persistent queue (MUS-122), not the control channel. | Queue sync | Player bar prompt | SEC-PRV-022, SEC-API-016 |
| CLI-104 | Headless and dedicated players | A screenless box, a hi-fi streamer or a player program on the server machine's sound card, controlled from the app | Plexamp headless (Plex Pass); Caldera headless; Navidrome jukebox; Lyrion with Squeezelite | Medium: fits low-hardware users | Later | Free. A headless player is a Gunmetal client like any other, with its own device key, paired as a limited-class device (SEC-IAM-055, SEC-CLI-024). It may run on the server's machine as a separate program under its own account, but the server process itself never plays or decodes audio (SEC-TM-034, SEC-TM-044) | Session registry; device pairing | Device picker | SEC-TM-034, SEC-TM-044, SEC-CLI-024, SEC-IAM-055 |
| CLI-105 | Synchronised multi-room | The same music in several rooms, in step | Spotify request (8,800 votes); Plex "Tandem Playback" (420); Roon and Lyrion do it | High: 8,800 votes | Later | Builds on the queue and session registry with clock sync, as a separate project that does not block handoff | Clock sync; group sessions | Device picker grouping | SEC-HIS-014, SEC-TM-024 |
| CLI-106 | Chromecast from Android and the web | Send music or video to a Chromecast or Google TV | Plex yes (casting broke for many after the 2025 rollout); Jellyfin yes; Emby yes; Symfonium best for music | Medium: Plex casting threads (up to 44 posts) | R2 | Signed URLs the receiver can refresh, and remuxed output when the device cannot open the container; handoff stays the main path between Gunmetal devices Owns casting for music and video; MUS-203 and VID-146 point here. Casting music to Nest and Google Cast speakers is the partial answer to the Google Home demand that CLI-131 declines; music needs only refreshable signed URLs and the original stream, no remuxer. The receiver gets only capability URLs for one item and the cast session, over HTTPS, never a session or device token (SEC-NET-064, SEC-NET-001); a receiver credential for queue control waits for SEC-CLI-071 (Later). From the web client, casting uses the browser's own cast and remote-playback support, never Google's hosted sender script, which the web client's no-third-party-script rule forbids (SEC-API-049; per-browser support unverified). | Refreshable signed URLs; remux output | Cast button; device picker | SEC-NET-064, SEC-API-098, SEC-API-027, SEC-API-049, SEC-NET-001, SEC-MED-081 |
| CLI-107 | Chromecast from iPhone | The same from iOS | Plex yes; Jellyfin's official iOS apps cannot (#466, 104 votes); Streamyfin video | Medium: 104 votes | Later | Later; R2 if the App Store licence decision allows (open decision 3 in the feature map README). The same sender code as Android, through the shared UI | As CLI-106 | Cast button | SEC-NET-064, SEC-API-098, SEC-API-027 |
| CLI-108 | Subtitles while casting | Subtitles appear on the cast device | Plex external subtitles failing on Chromecast; Streamyfin still in progress | Medium: Plex thread (22 posts) | R2 | Text subtitles go as a side track in the receiver's format; image subtitles follow the playback map's rules | Subtitle conversion | Cast subtitle menu | SEC-MED-052, SEC-HIS-039, SEC-NET-064 |
| CLI-109 | AirPlay | Send audio or video to Apple TV and AirPlay speakers | Plexamp free; Infuse (Pro); Jellyfin web; music AirPlay on iOS requested (#3713) | Medium: open Jellyfin request | Later | Later; R2 if the App Store licence decision allows (open decision 3 in the feature map README). Audio parity; video may need Apple's own player path (unverified) | Remuxed fMP4 or HLS | AirPlay button; device picker | SEC-NET-064, SEC-API-026 |
| CLI-110 | Local casting, with no phone relay | Cast devices fetch from the server over HTTPS on the home network | Symfonium proxy mode; Jellyfin request (#993, 5 votes) | Low: 5 votes | R2 | At home the cast device fetches from the server's HTTPS name, which resolves to its home-network address (SEC-NET-011), using capability URLs for one item and one cast session (SEC-NET-064). The phone relay is not built: a phone cannot serve HTTPS with a trusted certificate, so a relay would carry media and a bearer capability in cleartext on someone else's network (SEC-NET-001, SEC-CLI-042). Away from home, casting needs the server reachable over HTTPS (the owner's domain or proxy, or from R2 the browser edge, SEC-NET-041). | The server's HTTPS name on the home network; capability URLs per cast session | None (automatic) | SEC-NET-001, SEC-CLI-042, SEC-NET-064, SEC-NET-011 |
| CLI-111 | Casting that falls back gracefully | Casting still works when the receiver cannot decode the file | Symfonium transcodes for Chromecast; Plex (unverified) | Medium: a Symfonium selling point | R2 | Remux first and sandboxed transcode last, with the reason shown | Receiver profiles in the decision engine | Cast info | SEC-MED-081, SEC-TM-044, SEC-TM-047 |
| CLI-112 | Cast controls in the notification | Stop or adjust casting from the shade or lock screen | Jellyfin request (#258, 10 votes) | Low: 10 votes | R2 | Parity | None | Notification | SEC-CLI-062 |
| CLI-113 | Control UPnP renderers and Sonos | The phone controls network speakers | Symfonium (gapless UPnP, Sonos groups); Plex lists Sonos; Jellyfin has no official support | Medium: a Symfonium selling point | Later | Until then, Sonos is reachable through Music Assistant via the OpenSubsonic adapter, over HTTPS (SEC-EXT-066). When built, the phone is the control point and renderers fetch from the server over HTTPS with capability URLs for one item (SEC-NET-064); renderers that can only fetch plain HTTP are not supported, because the server answers plain HTTP with a help page only (SEC-NET-001). The server itself opens no SSDP or UPnP service (SEC-NET-059). | None | Device picker | SEC-NET-001, SEC-NET-064, SEC-NET-059, SEC-EXT-066 |
| CLI-114 | Watch or listen together | See VID-153, which owns this feature. | Plex removed it (restore request, 2,878 votes); Jellyfin SyncPlay (invite link #971, 267 votes); Spotify Jam (Premium hosts) | High: 2,878 votes | Later | See VID-153. | None beyond VID-153. | Group session sheet | See VID-153 |
| CLI-115 | DLNA server | Old TVs and receivers browse the library over UPnP | Plex is a DLNA server; Jellyfin moved DLNA into a plugin | Low: no vote evidence | No | Not built; see Deliberately not doing | None | None | SEC-NET-059, SEC-NET-066 |

### Car

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| CLI-116 | Android Auto | Browse and play from the car screen on Android | Plexamp free; Symfonium; Jellyfin Android 2.7 redesign; Finamp beta; Emby Premiere | High: core to replacing a streaming app | R2 | Free; a native media-library service fed by the same on-device library. The car service answers only Android Auto and system UI, checked by app signature, so other apps cannot list the library (SEC-CLI-055). | None beyond sync | Car browse tree; car Now Playing | SEC-CLI-055, SEC-CLI-054, SEC-PRV-058 |
| CLI-117 | Apple CarPlay | The same for iPhone | Plexamp free; Jellyfin's official app has none (#744, 38 votes); Finamp beta (Jun 2026); Emby Premiere | High: a Marvis Pro reviewer called its absence a deal breaker | Later | Later; R2 if the App Store licence decision allows (open decision 3 in the feature map README). Free; CarPlay templates fed by the local library; needs Apple's entitlement (unverified) | None | CarPlay screens | SEC-PRV-058, SEC-CLI-020 |
| CLI-118 | Your Home in the car | The car's browse tree mirrors your own Home sections | None | Medium: Home customisation is Spotify's second most-voted idea (15,697) | R2 | Home sections are saved queries, so customising Home customises the car | Home layout records | Car browse tree | SEC-CLI-055, SEC-CLI-020 |
| CLI-119 | Offline in the car | Downloads browse and play with no signal | Plexamp (Sept 2026); some apps show only downloads in CarPlay | Medium: tunnels and rural roads | R2 | The car tree reads the local store, so lost signal changes nothing for downloaded items; other items are marked or hidden | None | Car browse tree | SEC-CLI-036, SEC-CLI-055 |
| CLI-120 | Voice in the car | Ask for an album, artist or playlist while driving | Jellyfin Android Auto voice (2.7.0); Plexamp Siri; Finamp Siri (beta) | Medium: several apps added it in 2026 | R2 | Requests resolve against the local search index | None | Car voice | SEC-PRV-004, SEC-CLI-055 |
| CLI-121 | Quick navigation of long car lists | Get through long lists in the car interface | Emby users ask for it; others (unverified) | Low: Emby forum thread | R2 | Browse nodes grouped by letter where the platform allows it (unverified) | None | Car browse tree | SEC-CLI-055 |
| CLI-122 | Queue modes in the car | Shuffle, repeat and up next on the head unit | Finamp fixed shuffle display in an Aug 2026 beta; Jellyfin request (#1122, 12 votes) | Low: 12 votes | R2 | Parity | None | Car Now Playing | SEC-CLI-055 |
| CLI-123 | Android Automotive OS | A native app in cars with built-in Android | Plexamp none (119 votes since 2020, no staff reply); no confirmed rival | Medium: 119 votes | Later | Reuses the Android Auto media service with extra packaging. A car is a shared device with no lock screen, so it enrols as a limited-class device (SEC-CLI-024) and can be removed from Account > Devices when the car is sold or returned (SEC-IAM-042). | None | Built-in car interface | SEC-CLI-024, SEC-CLI-055, SEC-IAM-042 |
| CLI-124 | Audiobooks in the car | Resume, chapter skip and rewind on resume | Symfonium; Jellyfin Android Auto audiobooks (2.7.0) | Low: arrives with audiobooks | Later | Uses the multi-context queue planned for audiobooks | None | Car browse tree | SEC-CLI-055 |
| CLI-125 | The car's own browser | Play in a car's built-in web browser | Plex request (22 votes) | Low: 22 votes | Later | The web client may already work there; test and document it. It is treated as a shared browser (CLI-155), works only over HTTPS (SEC-NET-001), and is signed in by approval from a phone (SEC-IAM-108). | None | Web client | SEC-CLI-010, SEC-IAM-108, SEC-NET-001 |
| CLI-126 | Video in the car | Watch while parked | None; CarPlay does not allow video; Jellyfin request (#3536, 18 votes) | Low: 18 votes | No | Not built; see Deliberately not doing | None | None | None (No) |

### Watches, voice and smart home

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| CLI-127 | Control from the watch | Pause and skip from the wrist | All, through the system Now Playing app | Low: already works through the OS | R2 | Parity, provided by CLI-069 | None | Watch Now Playing | SEC-CLI-055 |
| CLI-128 | Wear OS app with offline music | Run with music on the watch and no phone | Symfonium (syncs with transcoding); Plex none | Medium: watch apps are wanted mainly for phone-free runs | Later | Uses the same cheap Opus encoding path. The watch enrols its own device key and plays downloads under its own offline grant (SEC-IAM-048, SEC-CLI-036); encoding runs in the server's jailed worker (SEC-TM-044). | Opus encoding | Wear OS app | SEC-IAM-048, SEC-CLI-036, SEC-CLI-024, SEC-TM-044 |
| CLI-129 | Apple Watch app with offline music | The same on Apple Watch | Emby (Premiere only); Plex request (298 votes since 2017) | Medium: 298 votes | Later | Same as CLI-128, with streams starting at the low bitrate Apple recommends for watches. The watch enrols its own device key and plays downloads under its own offline grant (SEC-IAM-048, SEC-CLI-036); encoding runs in the server's jailed worker (SEC-TM-044). | Opus or AAC encoding | watchOS app | SEC-IAM-048, SEC-CLI-036, SEC-CLI-024, SEC-TM-044 |
| CLI-130 | Existing music apps as interim clients | Use Subsonic apps such as Symfonium (with its Wear OS and Android Auto support) before native apps exist | Navidrome with Subsonic clients; Navidrome's Jellyfin music API (0.64) | Medium: users stay for clients, as Navidrome's API work shows | R2 | API-key sign-in only, scoped to read and play, off by default and behind the same authorisation tests (adapter owned by the ecosystem map; see open decision 1) Ships with the adapter in R2. It is the interim answer for iPhone users, whose native app is Later. The owner turns the adapter on with a fresh fingerprint or face check and each person then creates one key per app (SEC-EXT-051); apps connect over HTTPS only, with no local-network exception (SEC-EXT-066); legacy sign-in for older apps stays off unless the person marks one key as legacy (SEC-EXT-069). | OpenSubsonic adapter | Settings > Devices > App passwords | SEC-TM-070, SEC-EXT-051, SEC-EXT-056, SEC-EXT-066, SEC-EXT-067, SEC-EXT-069 |
| CLI-131 | Voice assistant skills | "Alexa, play..." on smart speakers | Plex shut its Alexa skill (Jun 2026); Plex Google Home request (2,461 votes); Jellyfin request (243); Emby Alexa (Premiere) | High: 2,461 votes | No | Not built; see Deliberately not doing The partial answer to the Google Home demand is casting to Nest and Google Cast speakers (CLI-106, R2). This row owns the decision; INT-146 and DIS-181 point here. | None | None | SEC-HIS-061, SEC-TM-067 |

### Live TV on devices

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| CLI-132 | Channel zapping on the remote | Channel up, channel down and last channel from the remote | Plex none (requests with 76 and 7 votes); Jellyfin zapping-list request (28 votes); Channels DVR and TiviMate | Medium: 76 votes | R3 | Native players decode the broadcast on the device, so changing channel needs no server transcode (owned with the live TV map). The stream and the channel logos reach the player only through the server, never from the provider directly (SEC-CLI-067). | Tuner broker | Live player; guide | SEC-CLI-067, SEC-TM-071, SEC-MED-078 |
| CLI-133 | Cast live TV | Send a live channel to a Chromecast | Plex partial (iOS and web only; request had 379 votes); others (unverified) | Medium: 379 votes | R3 | The same casting path as CLI-106, with remuxed live output | Live remux output | Cast button in the live player | SEC-CLI-067, SEC-NET-064, SEC-MED-081 |
| CLI-134 | Multi-view on more devices | Two to four channels on one screen | Channels DVR on Apple TV 4K and iPad only; Plex request (34 votes); Jellyfin none | Low: 34 votes | Later | Several libmpv players on one screen, limited by a decoder probe per device (owned with the live TV map) | Tuner broker | Live player layouts | SEC-CLI-067, SEC-TM-068, SEC-IAM-102 |

### Accessibility and localisation

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| CLI-135 | Screen readers reach every control | VoiceOver, TalkBack and desktop screen readers work everywhere, including over video | Jellyfin web player controls are not accessible (#4504 since 2023, #442 since 2019); Plexamp reported unusable with VoiceOver (2025), fixed in a Sept 2026 beta; Infuse best | Medium: issues open for years; regressions in 2025 and 2026 | R1 | Every interactive component carries a role, label and state; player controls are real views above the libmpv surface | None | All screens | SEC-CLI-001, SEC-MED-057, SEC-CLI-063 |
| CLI-136 | Accessibility as a release gate | An inaccessible control fails the build instead of joining a backlog | None documented | Medium: follows from CLI-135 | R1 | axe checks on the web build, accessibility snapshot tests on native, and a written VoiceOver and TalkBack script run before each release | None | None | None specific |
| CLI-137 | Screen readers on TV | VoiceOver and Hover Text on tvOS, TalkBack on Android TV | Plex's new Apple TV app regressed (Sept 2026) | Low: one regression thread | R2 | Focus and labels come from one model and are tested together on TV | None | TV screens | SEC-CLI-063, SEC-MED-057 |
| CLI-138 | Full keyboard use with visible focus | Everything works without a mouse, with a clear focus ring | Jellyfin 12.0 keyboard and controller fixes | Medium: Jellyfin fixed it in 2026 | R1 | The same focus model as TV (CLI-036), tested on the web | None | All web screens | None specific |
| CLI-139 | Text follows the system size | Text scales with OS or browser settings without clipping | Plex ignored iOS font settings (2025); Swiftfin improved Dynamic Type (1.3); a Plexamp user finds text too small on a car mount | Medium: Plex complaints | R1 | Type sizes are tokens, and layouts are tested at the largest system sizes | None | All screens | None specific |
| CLI-140 | Reduced motion | Animations calm down when the OS asks | No rival documents it; Apple's Liquid Glass was criticised for motion (118 "me too" votes to turn it off) | Low: 118 votes on Apple's forum | R1 | Motion is defined as tokens with a reduced set, and tested | None | All screens | None specific |
| CLI-141 | Themes, including high contrast | Dark, light, high-contrast and OLED-black themes, following the system by default | Spotify has no light mode (7,031 votes); Finamp AMOLED theme; no rival documents high contrast | High: 7,031 votes | R1 | Colour tokens with contrast checked in CI | None | Settings > Appearance | None specific |
| CLI-142 | Motor accessibility | Large targets, and no action that only a gesture can reach | Plex's double-tap fullscreen and picture-in-picture called hard for limited mobility; Marvis Pro configurable gestures | Low: one complaint thread | R1 | Minimum target sizes are checked by tests, and every gesture is mirrored by a button | None | All screens | None specific |
| CLI-151 | Mono audio and channel balance | Play in mono, or shift the balance left or right, for listeners with hearing loss in one ear | Spotify and the phone operating systems offer this (unverified); media servers not covered in the research | Low: no vote data (unverified) | R1.1 | On the web through Web Audio from R1.1, and in the native player modules in R2; a per-device setting | None | Settings > Accessibility | None specific |
| CLI-152 | Search inside settings | Type to find a setting instead of hunting through pages | Symfonium requested (no vote count in the research) | Low: no vote count | R2 | The research flags settings sprawl as a risk; R1 has few settings, so search arrives when the native clients add theirs | None | Settings | None specific |
| CLI-154 | Alternative app icons | Pick a different home-screen icon | Marvis Pro has more than 40 icons; Spotify reverted an icon change within days after backlash (research, no vote count) | Low: no vote count | Later | Shipped as optional variants of the gunmetal mark; the default never changes without notice | None | Settings > Appearance | None specific |
| CLI-143 | Subtitles that follow system caption settings | The system caption style by default, per-user overrides, and no size cap | Plex's new iOS app ignored the settings and broke styling (2025); Jellyfin appearance request (#161, 188 votes); Jellyfin Android TV capped the size (#5842) | High: 188 votes; low-vision complaints | R2 | Reads the OS caption preferences; overrides sync per user; the extremes are tested | None | Player subtitle menu; Settings > Accessibility | SEC-HIS-039, SEC-MED-052, SEC-CLI-053 |
| CLI-144 | Accessible track defaults | Prefer SDH subtitles or audio description automatically | Netflix; Jellyfin label request (#1206, 5 votes) | Low: 5 votes | R2 | Track flags read at scan time drive the default choice (which flags each format carries is unverified) | Track flags in the index | Settings > Accessibility | SEC-TM-031 |
| CLI-145 | Generated captions | Captions for files that have none | No rival; Jellyfin Whisper request (#2143, 61 votes) | Low: 61 votes | Later | An opt-in sandboxed job or plugin, given the CPU cost (owned with the playback map). It runs on the server with no outbound call unless the owner grants one (SEC-TM-048). | Speech-to-text job | Subtitle menu | SEC-TM-044, SEC-TM-065, SEC-TM-048, SEC-NET-053 |
| CLI-146 | Translations with a completeness bar | English at launch; from R1.2 the translation framework is in place and other languages are added with honest labels when partial | Jellyfin lists 90 languages but only 11 are 90% complete or more; Swiftfin is at 48% | Medium: shallow translations | R1.2 | ICU messages with plural rules; a pseudo-locale build in CI catches hard-coded and clipped strings; languages below the bar are labelled partial. Translations are rendered as text, so a hostile translation cannot inject markup (SEC-API-045). | Translatable server messages | Settings > Language | SEC-CLI-001, SEC-API-045, SEC-API-072 |
| CLI-147 | Right-to-left layouts | Arabic and Hebrew mirrored correctly | No rival documents right-to-left testing | Low: no vote evidence | R2 | Right-to-left layout tests in CI from the start R1 keeps logical CSS properties and externalised strings so mirroring is possible later; full mirroring and its tests arrive in R2. | None | All screens | SEC-API-048, SEC-MED-057 |
| CLI-148 | Each person's own language | Every user sees their own language, including server messages | Jellyfin 12 lets clients ask for a response language (earlier request #370, 219 votes) | Medium: 219 votes | R2 | Language is a synced user setting; server errors are typed and translated on the client R2: translated server messages need the typed-error catalogue to settle first; R1 ships English with typed errors. | Typed errors | Settings > Language | SEC-API-072 |

## Differentiators

1. **Offline is the normal state, and it is free (CLI-022, CLI-025, CLI-026,
   CLI-078, CLI-093).** Offline use is the third-ranked pain theme in the
   research and Jellyfin's most-voted request (1,820 votes). Plex and Emby
   charge for downloads, and Plexamp needed a rebuild in September 2026 to
   get an offline library. Gunmetal starts there: the synced library means
   browsing and search never wait for the server, offline uses the same
   screens, and plays made offline merge through the append-only log. In R1
   the web client browses and searches its synced copy (CLI-022) and merges
   plays made while the connection drops (CLI-093); from R1.1 a browser the
   person marked as their own keeps browsing and search working when the
   server is down (over HTTPS or localhost, CLI-025, CLI-150, CLI-155); a
   shared browser keeps nothing; downloads arrive with the native apps in
   R2, and downloads inside the browser stay Later (MUS-217) because
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
   with its position and lanes, offline-tolerant. The persistent queue
   ships in R1 and "continue on this device" in R1.1; handoff and remote
   control arrive with the native clients in R2. Multi-room comes later and builds
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
  undoes per-object authorisation (record 1, decision 6), and the core
  server opens no SSDP, UPnP or DLNA service (SEC-NET-059). If it is ever
  offered, it needs its own architecture record first, and then it is a
  plugin that is off by default, local-only, read-only and limited to the
  libraries the admin picks (SEC-NET-066). Because location is never
  identity (first principle 3), that record must also say how a renderer
  is authorised beyond being on the LAN.
- **A plain-HTTP web client or a password fallback (CLI-150).** Over plain
  HTTP every peer except loopback gets a help page that explains how to
  reach the HTTPS address (SEC-NET-001), and there are no passwords
  (SEC-IAM-025); a browser without passkeys is approved from a phone
  (SEC-IAM-108).
- **A phone relay for casting (CLI-110).** A phone cannot serve HTTPS with a
  trusted certificate, so a relay would carry media and a bearer capability
  in cleartext on someone else's network. Casting away from home needs the
  server reachable over HTTPS.
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
- **Paid unlocks, paid device caps or per-app fees.** Emby caps a household
  at 30 devices and gates full mobile playback; Plex gates downloads and
  remote playback. The AGPL build is the whole product (record 1, decision
  10). This is not the same as the limits on enrolled devices and
  concurrent streams per account that the security baseline requires
  (SEC-IAM-102): those are documented defaults the admin can change, there
  to stop a leaked credential being used without bound, never a paywall.
- **Client telemetry.** Clients send nothing on their own. Diagnostics leave
  the device only as a file the user exports and sends (CLI-033), in contrast
  with Plexamp 4.50's download telemetry that cannot be turned off. No
  client includes an analytics, advertising or crash-reporting SDK
  (SEC-CLI-027, SEC-PRV-009).

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
  not only in the UI, or restricted items reach the device (SEC-CLI-020,
  SEC-TM-026).
- The security baseline in `docs/security/` governs every row through its
  Security column. Its release scope (threat-model.md) puts the desktop
  shell and cast receiver credentials in Later, and native apps, device
  keys, profiles, downloads and iroh in R2; rows here follow it.
- The ecosystem map owns the OpenSubsonic and Jellyfin adapters, which are how
  Roku (CLI-012) and interim third-party clients (CLI-130) arrive.
- Remote access over iroh (record 1, decision 7) underlies home and away
  switching (CLI-029) and remote control. It arrives in R2; until then the
  web client is reached from away through the owner's reverse proxy or a
  tailnet (ACC-097). Browsers on iroh are relay-only, so
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
  drifts. The browser's lock-screen controls (R1) and offline loading
  (R1.1) are best effort on iOS.
- **Offline grants versus stolen devices.** A downloaded item plays without
  asking the server, so a stolen phone keeps its downloads until its grant
  expires (30 days by default, SEC-CLI-036). Revocation only works on next
  contact (CLI-096, CLI-156), and the docs must say so. The real protections
  are the lock screen, OS encryption and app-private storage (SEC-CLI-035).
- **Casting and signed URLs.** Cast receivers fetch URLs themselves and
  cannot join an iroh connection. They get capability URLs for one item and
  one cast session that the sender refreshes (SEC-NET-064, SEC-API-027),
  only over HTTPS, so casting away from home needs the server reachable
  over HTTPS; there is no phone relay (CLI-110).
- **The player is the riskiest code on the device.** libmpv, FFmpeg,
  libass and FreeType parse hostile media in C. The baseline contains them
  in layers: only core-parsed containers by default (SEC-CLI-049), bytes
  only and never a path or URL (SEC-CLI-047), a reduced build (SEC-CLI-048),
  a separate process where the platform allows (SEC-CLI-050) and patch
  windows of 14 or 7 days (SEC-CLI-052). iOS and tvOS cannot isolate the
  player, so the other layers carry it there.
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

- This map has 159 features across more than a dozen platforms. The rivals
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
   *Partly decided in the feature map README:* R1 ships a phone-width
   layout (CLI-149) and browser Media Session controls (CLI-070), and R1.1
   adds the installable web app, which states its phone limits plainly
   (CLI-003). The OpenSubsonic adapter (CLI-130) is R2, not R1, because it
   doubles the authorisation surface and few apps are known to support
   API-key sign-in.
   Still open (register D-14): whether the native Android music app
   (background audio, lock screen, downloads, Android Auto) ships as soon as
   it is ready, as a point release after R1.3, rather than waiting for the
   remuxer; it is not part of R1.1 to R1.3. *Recommendation:*
   yes, provided the native-app requirements of the security baseline that
   are marked R2 (device keys, keystore storage, offline grants, signing and
   the MAS targets, for example SEC-IAM-048, SEC-CLI-030, SEC-CLI-036,
   SEC-CLI-046 and SEC-STD-039) ship with it, and the release scope in
   `docs/security/threat-model.md` is updated to match.
2. **App Store distribution and the licence.** iPhone, iPad, Apple TV,
   CarPlay and Apple Watch all depend on it. Without a contributor licence
   agreement, an app-store permission cannot be added later without every
   contributor's consent. *Recommendation:* get legal advice now, and if it
   supports an additional permission for app-store distribution, add it
   before accepting the first outside contribution. If not, Apple platforms
   move to Later and the docs say why.
3. **Platform order for R2.** *Recommendation:* Android phone first; then
   Android TV, Google TV and Fire OS as one build; then iPhone, iPad and
   Apple TV once decision 2 is settled (Later until then). The desktop shell
   (CLI-014) is Later, because the security baseline's release scope puts
   it there and its rules (SEC-CLI-069) are Later; until then desktops use
   the web client and the installable web app. Samsung and LG packaging
   (CLI-010) and Roku through the Jellyfin adapter (CLI-012, which needs
   INT-099) are Later, following the README roadmap.
   VIDAA, Vega OS, Xbox and native watch apps stay Later, with the
   OpenSubsonic adapter and Symfonium covering Wear OS in the meantime.
4. **How long downloads last without contact, and how they are protected.**
   *Recommendation:* an admin-set grant lifetime with a default of 30 days
   (the window Spotify uses), renewed silently on any contact with the
   server, and revoked on next contact. Store files in app-private storage
   and rely on the operating system's encryption; no per-file encryption or
   DRM, which adds complexity and protects little against someone who
   already holds the unlocked device. This matches the security baseline
   (SEC-CLI-035, SEC-CLI-036), which also rules out removable storage until
   downloads there are encrypted (SEC-CLI-072), so SD-card downloads
   (CLI-059) are Later.
5. **Transcoded video downloads in R2.** They are the most-requested form of
   video download (518 votes) and exactly the server cost Gunmetal wants to
   avoid. *Recommendation:* R2 ships original and remuxed downloads only.
   Transcoded downloads come later as admin-enabled, scheduled, cached jobs
   in the sandbox, off by default.
6. **Remote push notifications.** They need Apple's and Google's push
   services and, in practice, a relay. A relay the project ran would hold
   device push tokens and learn which server each phone uses, which the
   security baseline forbids for project services (SEC-HIS-061).
   *Recommendation:* no remote push in R2; local notifications only
   (CLI-076). Revisit only with a self-hostable relay design, on the same
   terms as iroh relays, with payloads that carry only an opaque event ID
   (SEC-PRV-056, SEC-OPS-036).
7. **Chromecast receiver.** The default media receiver needs nothing from the
   project; a custom receiver would need a registration with Google and a
   hosted receiver page (details unverified). *Recommendation:* start with
   the default receiver and remuxed output. Build a custom receiver only if
   subtitles or queue control require it, hosted as a static page on
   gunmetal.tv that collects nothing and loads no trackers (SEC-HIS-061,
   SEC-PRV-054). A receiver that needs any credential beyond per-item
   capability URLs waits for SEC-CLI-071 (Later).
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
   strings externalised (CLI-147); the translation framework and its
   completeness bar arrive in R1.2 (CLI-146).
10. **Which shell runs the shared UI on desktops (CLI-014, Later)?** React
    Native has no first-party Linux desktop target (unverified).
    *Recommendation:* prototype React Native Web inside Tauri 2, with mpv as
    a separate sandboxed process (unverified), which is also the security
    file's recommendation (client security open decision 12), before any
    desktop is promised; the shell must meet SEC-CLI-069. If it fails,
    desktops keep the web client and the installable web app.

## Security notes

This area crosses five trust boundaries from
[the threat model](../security/threat-model.md): internet to server (TB1)
and LAN to server (TB2) for the web client, casting and adapters; relays
(TB3) for native apps away from home; client to server (TB4) for every
request, sync payload and control message; and device to person (TB5) for
what a phone, TV or browser keeps. Client releases cross project to
installs (TB12). The client-side threats are listed in full in
[client-and-device-security.md](../security/client-and-device-security.md)
(T-CLI-01 to T-CLI-30). These are the ones that matter most here.

| Threat | What it looks like here | How the features answer it |
|---|---|---|
| TM-T07 Script in metadata steals a session | A tag, lyric, device name or translation carries script into the web client | Every client renders untrusted text as text (SEC-CLI-001, SEC-API-045); the session lives only in an HttpOnly cookie (SEC-TM-058); outside links show their destination first (CLI-159); translations are text (CLI-146) |
| TM-T11, TM-T46 Cleartext | A plain-HTTP player, a cast relay or a renderer on the LAN carries a credential in the clear | No plain-HTTP web client and no password (CLI-150, CLI-025, CLI-070); native apps use HTTPS or iroh only (CLI-028); no phone relay (CLI-110); HTTPS-only renderers and adapters (CLI-113, CLI-130) |
| TM-T15, TM-T19 Restrictions skipped on a shared device | A child sees adult titles through the synced copy, the screensaver, OS rows or another profile's cache | The server builds each profile's copy (CLI-022, CLI-050); profiles are partitioned and PIN-protected data is encrypted (CLI-050); OS surfaces are opt-in and never show restricted profiles (CLI-049) |
| TM-T16, TM-T38, TM-T40 Revocation that does not bite; lost or resold devices | A stolen phone, a returned rental car or a shared computer stays signed in | Sign-out and revocation wipe the device (CLI-156); shared browsers keep nothing (CLI-155); capability URLs are re-checked on every request (CLI-090, CLI-099); anyone can revoke their own devices (CLI-096, ACC-069) |
| TM-T39 Offline data read by a thief | Downloads and the synced library on a lost phone or SD card | App-private storage under OS encryption; offline grants expire after 30 days by default (CLI-095); no plain removable storage (CLI-059, CLI-100) |
| TM-T17 One user drives another's player | A housemate takes over someone's queue | Handoff and remote control reach only your own sessions, with a closed command set (CLI-101, CLI-102) |
| TM-T18 History seen by admins or others | An admin's phone view or device list shows what people play or downloaded | Sessions without titles and device lists with counts only (CLI-058, CLI-096); private sessions on every client (CLI-157); no OS donations by default (CLI-049, CLI-074) |
| TM-T28 Hostile media attacks the player | A crafted file, subtitle or font exploits libmpv or a TV's browser engine | Container policy (CLI-047, CLI-084, CLI-085); bytes-only, reduced and isolated player (CLI-014, CLI-056); server-re-encoded artwork for TVs (CLI-010, CLI-038) |
| TM-T47, TM-T62 Rogue or hostile server | An impostor answers on the server's address, or a friend's server attacks a multi-server app | The identity key is pinned and checked on every connection (CLI-158); each server is a separate partition (CLI-018) |
| TM-T64 Device-code phishing | A stranger persuades someone to approve the stranger's "TV" | A typed code and a matching code whenever the approval is not proven local, and approval always starts on the phone (CLI-027) |
| TM-T66 Adapters weaken sign-in | A Roku or Subsonic app gets the account credential or a plaintext path | Per-app keys, HTTPS only, owner-enabled adapters, Quick Connect approved in a Gunmetal app (CLI-012, CLI-130) |
| TM-T33, TM-T49 Third parties learn what the household plays | Analytics SDKs, a project push relay, a relay operator or an assistant | No analytics or crash SDKs (SEC-CLI-027); diagnostics leave only as a file (CLI-033); no project push relay (CLI-077); the relay operator is shown (CLI-029); assistant sharing is opt-in (CLI-074) |
| TM-T21, TM-T56 Native decoding on the server; writes into media roots | A headless player, encoder or copy job runs native code in the server or writes beside the originals | Headless players are separate clients (CLI-104); encoders and the remuxer run in workers (CLI-083, CLI-085); nothing is written under a media root (CLI-085, CLI-100) |
| TM-T45 A bundled native component stays vulnerable | An old libmpv or Chromium in a shipped app | Media-stack fixes ship within 14 days, or 7 if exploited (SEC-CLI-052); insecure client versions get "Update required" (CLI-032) |

Residual risks, stated plainly in the docs: someone with developer access
to a TV can read an unprotected profile's cached library (CLI-050); a
stolen phone that never reconnects plays its downloads until its grant
expires (CLI-095); and iOS and tvOS cannot isolate the player, so its other
layers and the patch windows carry the load there.
