# Pain points and unmet demand

Researched and written on 2026-10-02 with live web access. Most counts below
were pulled the same day straight from primary sources: the Jellyfin feature
board's API, the Plex forum's Discourse JSON, the GitHub API, the Emby forum,
the Hacker News search API and the selfh.st survey data file. Counts change
daily, so read them as a snapshot. Reddit blocked automated access, so
subreddit sentiment appears here only through articles and Hacker News threads
that cite it, and is marked as second-hand where it matters.

## Scope

This file covers what people want and what they hate across self-hosted media
servers and the music apps people leave for them. It looks at:

- the top-voted requests on each rival's own feature board (Plex forum
  "Feature Suggestions", features.jellyfin.org, the Emby "Feature Requests"
  forum, Navidrome's GitHub issues);
- the most common complaints in forums, issue trackers and Hacker News;
- why people switch between products or leave them;
- controversies from 2024 to 2026: pricing and paywalls, redesigns, privacy,
  breaches and vulnerabilities, performance at scale, maintainer turnover,
  and streaming-service price rises that push people towards self-hosting.

Findings are ranked by strength of evidence: how many people voted or
commented, whether the same complaint shows up across several products, and
whether it is getting worse or better in 2025 and 2026.

It does not cover codec-level detail, client-platform engineering, or
metadata providers in depth. Sibling files in `docs/research/` cover those.
Where a row in the inventory touches those areas, it records only what users
ask for and how loudly.

How to read the evidence:

- "votes" means the vote counter on a feature board (Plex forum topic votes,
  Fider votes on features.jellyfin.org).
- "+1" means GitHub thumbs-up reactions on an issue.
- "posts", "replies", "likes" and "views" are the forum's own counters.
- "(unverified)" means I could not confirm the claim from a source during
  this research, usually because it comes from memory or from a single
  user's post.

## Feature inventory

Each row is a feature that users ask for, complain about, or praise. Cell
values: **Yes** (shipped and generally working), **Partial** (exists with
real limits), **No**, **Paid** (behind Plex Pass or Emby Premiere),
**Plugin** (needs an add-on), **Requested** (an open request with the count
given in Notes). Vote counts are as of 2026-10-02.

### Sign-in, accounts and security

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Local accounts with no vendor account | Users sign in to your server, not to a company's cloud | No: every user needs a plex.tv account | Yes | Yes; Emby Connect is optional (unverified) | Jellyfin and Navidrome: fully local | Plex request for a built-in local auth server: 372 votes, open since 2015 |
| Sign-in survives a vendor outage | Playback at home keeps working when the vendor's cloud is down | Partial: a LAN allow-list setting exists (unverified); plex.tv and watch.plex.tv were down for about two hours on 2026-07-14 | Yes | Yes (unverified) | Jellyfin | The outage thread notes the status page lagged reality |
| Two-factor authentication | A second factor at login | Yes, for plex.tv accounts (1,653-vote request, implemented) | No: planned, 1,103 votes, filed 2019 | Requested: 459 replies; current state (unverified) | Plex | Jellyfin's sixth most-wanted item |
| Passkeys (WebAuthn) | Phishing-resistant sign-in with no password | No (unverified); small request exists | No: request filed 2026-09-23, 5 votes | No (unverified) | None of the three | Low vote counts reflect low awareness rather than low value; ADR 0001 already commits Gunmetal to passkeys |
| OIDC / SSO with your own identity provider | Log in through Authelia, authentik, Pocket ID and similar | No | Plugin (third-party SSO plugin); planned natively, 1,191 votes. Native apps lag, per Hacker News users | Requested; topic title says development started (216 replies) (unverified) | Navidrome, partially, via a trusted reverse-proxy header | selfh.st 2025: Authelia 390, Pocket ID 310, authentik 268 respondents |
| LDAP | Use an existing household or company directory | No | Plugin | Plugin (unverified) | Jellyfin (official plugin) | Navidrome's most +1'd issue is LDAP (122 +1, open) |
| Pair a TV with a short code | Sign in on a TV without typing a password | Yes (plex.tv/link) | Yes (Quick Connect); QR code version planned, 225 votes | Yes (unverified) | Plex | |
| Sign out every device / revoke sessions | Kill tokens after a leak | Yes; offered during password change after the 2025 breach | Partial: device list exists; "Revoke Remote Sessions" requested (9 votes, 2026) | Partial (unverified) | Plex | |
| Every media and API endpoint authenticated | Nobody can pull your files or user list without logging in | Partial: CVE-2025-69414 and CVE-2025-69416 let tokens be escalated or harvested | Partial: issue #5415 listed unauthenticated streams, subtitles, images and WebSockets; #13991 (GetUser fully unauthenticated) still open | (unverified) | None demonstrably | Navidrome fixed a run of authorisation bugs (IDOR) in 2026 |
| Scoped API keys | Give a script or tool only the access it needs | No (unverified) | No: issue #13992 open, 23 +1 | No (unverified) | None | Navidrome 0.64 scopes its new Jellyfin-API tokens, but they never expire |
| Login rate limiting | Password guessing gets throttled | (unverified) | (unverified) | (unverified) | Navidrome, since 0.64.1 (2026-09-21) | Navidrome's Subsonic login was unthrottled until then (CVSS 7.4) |
| Secrets kept out of URLs | Tokens do not leak into proxy and browser logs | Partial: tokens commonly travel as URL parameters (unverified) | Partial: `api_key` URL parameter flagged in #5415 | (unverified) | None | Gunmetal plans short-lived signed stream URLs instead |
| Clear security advisories | You know what to patch and why | Partial: forum posts; some CVEs surfaced through the press first (heise, Jan 2026) | Yes: GitHub advisories; 12.0 closed several path traversals | (unverified) | Navidrome: per-release advisory lists with CVSS scores | |
| Sandboxed transcoder | A hostile file cannot take over the server through FFmpeg | No (unverified) | No: FFmpeg runs as the server user (unverified) | No (unverified) | None known | Core of ADR 0001 decision 3 |

### Remote access and networking

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Remote access without port forwarding | Works behind CGNAT and strict routers | Yes: relay fallback, speed-capped (cap unverified) | No: bring your own VPN or reverse proxy | No (unverified) | Plex for ease; Tailscale-style VPNs for do-it-yourself | selfh.st 2025 remote access: VPN 2,902, reverse proxy 2,716, forwarded ports 721, LAN only 510 |
| Free remote streaming | Watch away from home without a subscription | No: Plex Pass or Remote Watch Pass required, rolled out per platform from 2025-04-29 to 2026 | Yes | Yes, though some apps need Premiere or a per-app unlock | Jellyfin | Plex says third-party API clients also fall under the rule in 2026 |
| Remote stream reliability | Remote playback connects every time | Partial: "cannot connect securely" thread (635 posts, Nov 2025); "Remote Access (again)" (263 posts, Aug 2026) | Depends on the admin's own setup | Depends on setup | None | |
| Remote quality defaults to original | No needless transcodes when bandwidth allows | No: "Default All Clients to Max Internet Streaming" has 1,289 votes, 1,728 posts and 70,974 views | Partial (unverified) | Partial: 4.9.3 (Jan 2026) added a per-user auto remote quality option | None clearly | One of Plex's most-voted open requests, and its most-discussed |
| Parallel connections for internet streaming | Faster starts and fewer stalls over long or lossy links | Requested: 195 votes | Planned: 231 votes | (unverified) | None | |
| Per-user bandwidth or stream limits | One user cannot eat the upload | Partial: server-wide caps; per-user limits requested (188 votes) | Yes: per-user remote bitrate limit (unverified) | Requested: simultaneous-stream limit (161 replies) | Jellyfin (unverified) | |
| Web app on the LAN is unaffected by browser local-network prompts | The browser plays from the server in the same house | No: Chrome's Local Network Access prompt can push the Plex web app onto the remote path, which then asks for a pass (Plex, Nov 2025) | Yes: the web app is served by the server itself | Yes (same reason) | Jellyfin and Emby | Structural: Plex's default web app lives on app.plex.tv, not on your server |

### Prices and paywalls

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Subscription price | | $6.99 a month or $69.99 a year since 2025-04-29 | Free | $4.99 a month or $54 a year | Jellyfin | MacRumors: first Plex Pass rise in about a decade |
| Lifetime licence | | $749.99 since 2026-07-01; was $249.99 from 2025-04-29 and $119.99 before | Free | $119 | Jellyfin; Emby is the cheapest paid lifetime | Plex says existing lifetime holders keep their perks |
| Per-viewer remote pass | A friend pays instead of the server owner | Remote Watch Pass: $2.99 a month or $29.99 a year after 2026-06-01 (intro price was $1.99 or $19.99) | Not needed | Not needed | Jellyfin | |
| Hardware transcoding | GPU encode and decode | Paid | Free | Paid | Jellyfin | |
| Intro and credits skip | | Paid | Free (plugin plus media segments) | (unverified) | Jellyfin on price, Plex on polish | |
| Offline downloads | | Paid for Plexamp music; video (unverified) | Free, where the client supports it | Paid | Jellyfin on price | |
| DVR | | Paid | Free | Paid | Jellyfin | |
| Backup and restore | | No built-in tool (unverified) | Free since 10.11 | Paid | Jellyfin | |
| Music power features | Downloads, lyrics, radio, sonic analysis, EQ | Paid in Plexamp | Free, client-dependent | (unverified) | Navidrome plus free clients on price; Plexamp on polish | See the music table |

### Playback, transcoding and hardware

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Direct play of the original file | Full quality, near-zero server CPU | Yes, client-dependent; the new Apple TV app's player hangs on pause for some users (Sept 2026) | Yes, client-dependent; the mpv-based desktop app is strongest | Yes, client-dependent | mpv-based players (Plex HTPC, Jellyfin Desktop) (unverified as a ranking) | This is Gunmetal's core bet |
| Remux when only the container is the problem | Lossless and cheap | Yes ("Direct Stream") | Yes | Yes | All three are comparable | Gunmetal plans to do this in-process in Rust, not by launching FFmpeg |
| HEVC as a transcode target | Smaller remote streams at equal quality | Experimental since 2025-01-21; 491-vote request | Yes (unverified) | Requested: 505 replies; state (unverified) | Jellyfin (unverified) | |
| Pre-transcoded "optimised versions" | Ready-made phone or remote copies | Yes ("Optimize") | No: 979 votes; maintainers declined in Nov 2025 | Yes ("Content Conversion", Paid) | Plex | |
| Turn transcoding off | Fail rather than burn CPU | Partial (unverified) | Yes: per-user permission | Requested: 165 replies | Jellyfin | Plex "Disable 4K transcoding": 249 votes |
| Server-side cap on transcode resolution | Protect a weak CPU | Partial (unverified) | Requested: 131 votes and a 2026 duplicate | (unverified) | None clearly | |
| Hardware-acceleration self-test | Know whether the GPU path works before users hit errors | No (unverified) | Planned: 423 votes | No (unverified) | None | Graceful software fallback planned: 91 votes |
| Pre-buffer the next item | No stall between episodes or after the pre-roll | Requested: buffer the movie during the pre-roll (589 votes) | Requested: 222 votes | (unverified) | None | |
| Intro and credits skip | | Yes (Paid); persistent auto-skip requested (483 votes) | Partial: media segments since 10.10 (Oct 2024), detection by plugin | Yes (unverified) | Plex on polish | Emby's "Show Intro Skip Option" thread has 3.9k replies |
| Trickplay (scrub thumbnails) | See where you are while seeking | Yes | Yes: about 100x faster generation since 10.10 | Yes (unverified) | Plex and Jellyfin | Spoiler-free thumbnails requested on both boards |
| Playback speed | | Yes since 2024 (1,298-vote request) | Yes (unverified) | (unverified) | Plex | |
| Custom skip-back and skip-forward lengths | | Partial: 453-vote request | Yes in the web client (unverified) | (unverified) | Jellyfin (unverified) | |
| "Ends at" clock | Know when the film finishes | Partial: 192-vote request | Yes; an option to hide it was requested in 2026 | (unverified) | Jellyfin | |
| "Are you still watching?" | Stops autoplay running all night | (unverified) | Requested: 111 votes (2025) | (unverified) | Streaming apps | |
| Dialog boost and loudness levelling for films | Hear whispers without being deafened by explosions | Yes (Paid) since 2026-09-15, but it transcodes the audio | No: "volume above 100%" requested (124 votes) | (unverified) | Plex | Plex's own request had 170 votes; Gunmetal could do this on the client with no server work |
| Correct audio format labels (Atmos, DTS:X) | Trust what the info panel says | Partial: 701-vote request | Partial | Requested: 219 replies | None clearly | |
| "Stats for nerds" | See codec, bitrate and why it transcoded | Partial | Partial: Android TV request, 219 votes | (unverified) | None clearly | |

### Subtitles and audio tracks

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Subtitles load and stay in sync, including on resume | | Yes in most clients (unverified) | No: #2547 open since 2020 (103 +1, 151 comments); web #4346 (62 comments) | (unverified) | Plex | The most-cited Jellyfin complaint in the 12.0 Hacker News thread |
| Adjust subtitle offset during playback | Fix drift yourself | Partial: the new Apple TV app dropped it (Sept 2026 thread) | Partial: web yes; Android TV requested (179 votes) | Requested: 229 replies | mpv-based players (unverified) | |
| Automatic subtitle sync | Server aligns a subtitle to the audio | Yes since Sept 2024 (687-vote request) | No | No (unverified) | Plex | |
| Image and styled subtitles rendered on the client | No burn-in transcodes for PGS or ASS | Partial (unverified) | Partial: client-side PGS in web since 10.10 | (unverified) | mpv-based players | Burn-in is a leading cause of avoidable transcodes (unverified as a measured share) |
| Subtitle appearance options | Size, colour, position | Partial | Requested: 188 votes | (unverified) | (unverified) | |
| Per-series and second-language defaults | Right subtitles chosen automatically | Yes: per-series control since 2023 (701-vote request) | Partial: second preferred language requested (95 votes) | (unverified) | Plex | |
| Track names shown everywhere | Tell "Commentary" from "English 5.1" | Partial: 417-vote request still open | Yes (unverified) | (unverified) | Jellyfin (unverified) | |
| External audio tracks | Side-loaded dubs or commentaries | Requested: 329 votes | Partial: subfolder search requested (9 votes) | (unverified) | (unverified) | |

### Watch state, home screen and discovery

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Remove from Continue Watching | Clear abandoned shows without marking them watched | Yes | No: planned, 1,725 votes (second most wanted) | Requested: "Forget a series in Next Up" (166 replies); state (unverified) | Plex | |
| Watchlist | Save things to watch later | Yes | No: planned, 1,294 votes | Requested: 79 replies (unverified current) | Plex | |
| Watch history | See what you watched and when | Yes | No: planned, 830 votes | (unverified) | Plex, plus Tautulli for admins | |
| Watch state survives file replace or rename | Upgrading a file does not reset progress | Yes (unverified) | Broke in 10.11.0 (#15001, 61 +1, closed) | (unverified) | Plex (unverified) | Gunmetal's append-only watch log targets this directly |
| One combined Next Up / Continue Watching row | | Yes | Requested: 203 votes | (unverified) | Plex | |
| "Recently Added" opens the series, not the latest episode | | Yes (unverified) | Requested: 126 votes (2026); web bug #8496 (2026) | (unverified) | Plex (unverified) | |
| Spoiler protection | Blur thumbnails and synopses of unwatched episodes | Requested: 118 votes (sports) | Requested: 118 votes | (unverified) | None | |
| Hide a title from my own views | | (unverified) | Requested: 131 votes | (unverified) | (unverified) | |
| Personal ratings | Rate for yourself, sort by your rating | Yes | Planned: 453 votes | (unverified) | Plex | |
| Home screen you can arrange | Choose and order the rows | Partial: library pinning was dropped from the 2025 mobile redesign | Planned: 169 votes; modular home requested (92) | Yes: Emby published a full customisation guide in Sept 2025 | Emby | |
| Recommendations from your own library | "Because you watched..." with no outside service | Yes (unverified) | Partial: 12.0 made recommendation sources configurable | (unverified) | Plex (unverified) | |
| Trakt sync | Keep history on Trakt | Third-party; "full Trakt integration" requested (376 votes) | Plugin | Plugin (unverified) | Jellyfin and Emby via plugins | |
| Keep streaming-service content out of my library views | | Partial: Discover can be disabled since 2022 after a 1,000-vote request | Yes: it is never mixed in | Yes | Jellyfin and Emby | |

### Library, metadata and organisation

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Fast browsing of large libraries | Instant scrolling across thousands of items | Yes | Partial: lazy loading "started" (1,179 votes, since 2019); 10.11 regressions (#15141, #15352); much better in 12.0 per Hacker News users | (unverified) | Plex | |
| Choice of database backend | MySQL or PostgreSQL for very large installs | No: request 243 votes | Started: EF Core landed in 10.11; PostgreSQL likely first (663 votes) | No: 391 replies | None | Many of these requests are really about speed and corruption, not the engine |
| Upgrades that cannot brick the library | Safe one-click upgrades | Partial: PMS 1.43.0 package signing rollback (Jan 2026, 395 posts); "Corrupted Database Upon Upgrade" (Apr 2026) | Partial: 10.11 migration failures (#15027, 121 comments); 12.0 rewrite is one-way and needs a backup | (unverified) | None | |
| Built-in backup and restore | | No (unverified) | Yes since 10.11; scheduled backups planned (91 votes) | Yes (Paid) | Jellyfin and Emby | |
| Folder view | Browse exactly as files sit on disk | Partial (unverified) | Requested: 438 votes, 134 comments | Yes (unverified) | Emby (unverified) | |
| Show missing episodes | See gaps in a series | Requested: 327 votes | Started: 343 votes | Yes (unverified) | Emby (unverified) | |
| Versions and editions | Director's cut and theatrical, 4K and 1080p under one title | Yes for movies; TV editions shipped 2026-07-09 | Partial: episode versions new in 12.0; multi-folder versions requested (265) | Yes (unverified) | Plex | |
| Alternate episode orders | DVD, absolute and TMDB episode groups | Partial: TVDB orders yes; TMDB groups still open (502 votes) | (unverified) | Requested: TVDB orders (92 replies), TMDB groups (58) | Plex | |
| NFO files | Keep metadata in sidecar files you control | Preview only (Feb 2026) | Yes | Yes | Jellyfin and Emby | |
| Pluggable metadata sources | | Yes: custom metadata providers (Dec 2025) | Yes (plugins) | Yes (plugins) | Jellyfin | A 2026 TMDB API change broke Jellyfin image fetching (#16722, 77 +1) |
| Bulk metadata editing | Fix 50 items at once | Yes (unverified) | Requested: 186 votes | Requested: 146 replies | Plex | |
| Edit metadata on a phone | | Yes since Mar 2026 | (unverified) | (unverified) | Plex | |
| Original titles and multi-language metadata | | (unverified) | Requested: 315 and 212 votes | (unverified) | (unverified) | |
| Nested collections | Collections inside collections | Requested: 514 votes | (unverified) | Requested: 97 replies | None | |
| Smart collections | Collections defined by a rule | Yes | Requested: 104 votes | (unverified) | Plex | |
| Duplicate finder | | Partial (unverified) | Requested: 135 votes | (unverified) | (unverified) | |
| Search by tag or genre, fuzzy search | | (unverified) | Requested: 256 and 90 votes | (unverified) | (unverified) | |
| Flatten single-season shows | Skip the pointless season level | (unverified) | Requested: 172 votes (2025) | (unverified) | (unverified) | |

### Users, sharing and administration

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Kid and managed profiles with ratings limits | | Yes | Yes; expanded in 12.0 | Yes | All three | |
| Switch profile on a TV with a PIN | | Yes; hide the PIN as you type it requested (111 votes) | Partial: multi-account switch started (154 votes) | Yes (unverified) | Plex | |
| User groups, templates and defaults | Set up ten family members once | No (unverified) | Requested: 202, 130 and 258 votes | Requested: 174 and 55 replies | None | |
| Invite and sign-up flow | Send a link, the person makes an account | Yes, via plex.tv email invites | Requested: 227 votes | (unverified) | Plex | |
| Message all users or post a maintenance notice | | Requested: 1,206 votes | Partial: per-session messages only (unverified); broadcast requested (89 votes) | Requested: 119 replies | None | |
| Stop a user's stream | | Yes (unverified whether Paid) | Requested: 402 votes; what the dashboard can do today (unverified) | Yes (unverified) | Plex | |
| Server health dashboard | Sessions, CPU, bandwidth, top users | Yes, on mobile and TV since 2026-07-29 | Partial: Prometheus metrics improvement requested (63 +1) | Yes | Plex, with Tautulli for history | |
| Expiring share link for one item | Send a film to someone with no account | Requested: 294 votes | Requested: 580 votes | No (unverified) | Navidrome (public share links for music) | |
| Several servers in one app | | Yes | Requested: 177 votes | Yes (unverified) | Plex | |
| Watch together | Synchronised viewing with friends | Removed from the new apps in 2025, web only; restore request 2,878 votes | Yes (SyncPlay); invite link requested (267), chat (104) | Requested: 285 replies | Jellyfin | |

### Clients and platforms

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Apple TV app | | Yes; the Sept 2026 rewrite drew a 268-post, 1,066-like complaint thread | Partial: Swiftfin (463-vote request "started") | Yes: 2.0 released July 2025 | Third-party players such as Infuse (unverified ranking) | |
| Samsung and LG TVs | | Yes; Samsung playback broke for many users in Aug 2026 (250 posts, 504 likes) | Yes: LG; an official Tizen app per a Hacker News user (unverified) | Yes | (unverified) | |
| Hisense Vidaa | | (unverified) | Requested: 453 votes | Requested: 57 replies | None | |
| Roku and Fire TV | | Yes; the 2025 to 2026 redesigns drew heavy complaints ("Fire TV app - Hopelessly Crippled", June 2026) | Yes | Yes | (unverified) | |
| Desktop app | | Partial: "Are the Desktop Apps Dead?" (Jan 2026, 96 posts) | Yes: Qt 6 desktop app | Partial: Linux public beta Jan 2026 | Jellyfin | |
| Voice assistants | | No: Alexa skill shut down 2026-06-15; Google request 2,461 votes | Requested: 243 votes | Yes: Alexa (Paid) | Emby | |
| Chromecast | | Yes | Partial: iOS casting requested (104 votes) | Yes | Plex | |
| Offline video downloads | | Yes, reworked June 2026; Plex Pass needed (unverified) | Partial: Android downloads the full original only; 1,820 and 817 votes | Yes (Paid) | Plex | The single most-voted Jellyfin request |
| Download a smaller transcoded copy | Fit a season on a phone | Yes (unverified whether Paid) | Planned: 518 votes | Yes (Paid) | Plex | |
| Download a whole season or batch | | Yes | Started: 219 votes | (unverified) | Plex | |
| Browse the library with no connection | Instant UI on a plane or in a dead zone | No (unverified) | No | No | Streaming apps; Plexamp 4.50 now caches library data, per a user report | README commits Gunmetal to a synced library |

### Privacy and vendor dependence

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Viewing activity stays on your server | | No: activity flows through plex.tv; the 2023 "Week in Review" emails exposed friends' viewing | Yes | Mostly (unverified) | Jellyfin | |
| Social features you can switch off | | Partial: Discover can be disabled; reviews (2024), Lists (Mar 2026) and Discussions (June 2026) keep arriving | No social features to switch off | No social features to switch off | Jellyfin and Emby | |
| No telemetry, or telemetry you can refuse | | No: Plexamp 4.50 sends download telemetry, including titles and account name, with no off switch | Yes (unverified) | (unverified) | Jellyfin (unverified) | |
| Features not withdrawn by the vendor | | No: Watch Together, the Alexa skill, TIDAL, custom server URLs and mobile DVR scheduling were all removed 2024 to 2026 | Community-run | Vendor-run, quieter | Jellyfin | |

### Music

Here the relevant rivals are music players, not video servers. Columns:
Plex with Plexamp, Jellyfin with its music clients (mainly Finamp), and
Navidrome with Subsonic clients (Symfonium, Feishin, play:Sub, Amperfy,
Tempo). Emby is mentioned in Notes where its board shows demand.

| Feature | What the user gets | Plex + Plexamp | Jellyfin (+ Finamp) | Navidrome (+ Subsonic clients) | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Gapless playback | Live albums and DJ mixes play without clicks | Yes, free | Partial: client-dependent; board item open (647 votes) | Partial: client-dependent | Plexamp | Emby gapless request: 108 replies |
| Loudness levelling | Tracks play at an even volume | Yes, free | Partial, client-dependent (unverified) | Partial: serves ReplayGain, the client applies it | Plexamp: analyses audio itself | Emby ReplayGain/R128 request: 98 replies |
| Crossfade and smart transitions | | Yes ("Sweet Fades") | Requested: 138 votes | Client-dependent | Plexamp | |
| Offline downloads | | Paid; "download entire library" requested (206 votes) | Yes via Finamp (unverified); "Download All Music" requested | Client-dependent (Symfonium and others) (unverified) | Plexamp, with the June 2026 download rework | |
| Synced lyrics | Lines or words highlight in time | Paid; "lyrics always visible" requested (260 votes) | Yes since 10.9; word-level timing since 10.11 | Yes: TTML, ELRC and SRT with word timing since 0.63 | Navidrome on formats; Plexamp on presentation (unverified) | Jellyfin TTML (16 votes) and auto-scroll (8) requests are both from 2025 to 2026 |
| Sonic similarity | "Play more like this" from audio analysis | Paid (Sonic Adventure, Sonic Sage) | Requested: 98 votes | Plugin (sonic similarity plugin) | Plexamp | |
| Radio and mixes | Endless station from a track or artist | Paid (track and album radio, Guest DJ) | Partial: Instant Mix; ListenBrainz-based recommendations in 12.0 | Partial: ListenBrainz and Last.fm similar tracks | Spotify and YouTube Music | The top gap named by people who leave streaming |
| Smart playlists | Rule-based playlists that update themselves | Yes, free | Planned: 588 votes | Yes | Navidrome and Plexamp | Emby request: 170 replies |
| Good playlist management | Sort, search, resume position, cover art | Partial: 1,425, 183 and 237-vote requests | Partial: requests for sorting and display options | Yes (cover art since #406) (unverified) | (unverified) | |
| Multiple artists per track | Collaborations credited properly | Partial: 520-vote request | Partial | Yes | Navidrome | |
| Release types | Albums, singles, EPs, live and compilations kept apart | Partial: manual edit requested (72 votes) | Requested: 42 votes | Yes | Navidrome | |
| Albums grouped by tags, not folders | Multi-disc albums stay one album | Yes (unverified) | Partial: users report "Disc 1" and "Disc 2" splitting into two albums | Yes | Navidrome | |
| CUE sheets | Play single-file rips as tracks | Requested: 507 votes | Requested: 171 votes | (unverified) | Desktop players (unverified) | |
| Composer and other credits | Classical and jazz browsing | Requested: 89 votes | Partial | Yes (roles) | Navidrome | |
| Rich and custom tags | Moods, styles, custom fields | Partial: 866-vote request | Partial | Yes (unverified) | Navidrome (unverified) | |
| Scrobbling | Last.fm, ListenBrainz | Yes (unverified detail) | Plugin | Yes: Last.fm, ListenBrainz, Maloja | Navidrome | ADR 0002 puts this in plugins with network grants |
| Queue hand-off between devices | Start on the phone, continue on the speaker | Partial (unverified) | Partial: Spotify-Connect-style casting requested (29 votes) | Partial: save and restore the play queue | Spotify Connect | ADR 0002 commits to queue hand-off |
| Remote and headless players | Phone controls a Pi or hi-fi box | Yes (headless Plexamp is Paid); Caldera player (2026) | Partial | Yes: jukebox mode plays on the server's audio output | Plexamp | |
| CarPlay and Android Auto | | Yes, free | Partial: CarPlay requested (38 votes) | Client-dependent (play:Sub has CarPlay) | Plexamp | Android Automotive requested from Plex (119 votes) |
| Multi-room and Sonos | | Yes (Sonos) | Requested: 11 votes | Client- or jukebox-dependent | Lyrion Music Server (ex-Logitech), per Hacker News users | |
| Bit-perfect output and sample-rate matching | Audiophile output path | Paid | Requested ("Roon-like", 52 votes) | Client-dependent | Roon (unverified) | |
| Equaliser | | Paid (ten-band) | Client-dependent | Client-dependent | Plexamp | |
| Visualisers | | Yes; Caldera adds a Milkdrop-compatible engine | No | No | Plexamp / Caldera | |
| Multichannel and hi-res music | | Partial: Shield request (216 votes) | (unverified) | (unverified) | (unverified) | |
| Spatial audio | | Requested: 222 votes | No | No | Apple Music | |
| Music in the same app as video | One app for everything | No: music playback moved out of the main mobile app in 2025 | Yes | Not applicable | Jellyfin | Some users prefer separate apps; Emby "Standalone Music App" request has 175 replies |
| Internet radio | | Requested: 272 votes | (unverified) | Yes | Navidrome | |
| Shared listening | Listen along with friends | (unverified) | No | Requested: 23 and 25 +1 | Spotify Jam | |
| Favourites that never get lost | | Yes (unverified) | No: #14981 favourites lost during playback (25 +1, open) | Yes (unverified) | (unverified) | |
| Light server footprint | Runs on a tiny board | Partial (unverified) | Partial: a Raspberry Pi 4 user reports scans struggle | Yes: Navidrome claims it runs on a Raspberry Pi Zero | Navidrome | |

### Audiobooks, podcasts and books

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Audiobooks | Chapters, resume per book, speed | Requested: 2,264 votes, open since 2013 | Partial: basic support under books; 857 votes "started" | Partial (unverified) | Audiobookshelf: purpose-built | selfh.st 2025: Audiobookshelf used by 840 of 4,081 respondents |
| Podcasts | | No (unverified) | Requested: 563 votes | (unverified) | Audiobookshelf | Navidrome podcast request: 43 +1 |
| Ebooks and comics | | Requested: 3,182 votes, the most-voted Plex suggestion ever | Yes: much improved in 12.0 | Partial: book reader requested (59 replies) | Dedicated readers (unverified); Jellyfin among the three | ADR 0002 puts books after v1 |

### Live TV

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| M3U / IPTV with channel groups | | Partial | Planned: 376 votes | Requested: 330 replies | (unverified) | First module after v1 in the ADRs |
| Several EPG sources | | Requested: 611 votes | (unverified) | (unverified) | (unverified) | |
| Rename and reorder channels | | Requested: 491 votes | (unverified) | (unverified) | (unverified) | |
| Channels built from your own library | | Requested: "PseudoTV" (622 votes) | Third-party (Tunarr) | (unverified) | Tunarr (Hacker News, 209 points) | |
| DVR | | Paid | Free | Paid | Jellyfin | |

## Pain points and unmet demand

### Ranked summary

Ranked by strength of evidence: size of the signal, how many products it
spans, and whether it grew in 2025 and 2026.

| Rank | Theme | Strongest evidence | Spans | 2024 to 2026 trend |
|---|---|---|---|---|
| 1 | Paywalls and price rises on Plex | Remote streaming paywalled platform by platform; lifetime tripled to $749.99; 443-post forum thread; Jellyfin now leads Plex two to one in the selfh.st survey | Plex, Emby | Worse every few months |
| 2 | Redesigns that remove features and slow navigation | Plex "New Experience" feedback 1,531 posts and 6,486 likes; "awful experience" 759 posts and 6,434 likes; rollback vote 504; Watch Together restore 2,878 votes | Plex | Ongoing; Plex backtracked on navigation in Aug 2026 |
| 3 | Offline use | Jellyfin's top two offline items: 1,820 and 817 votes; transcoded downloads 518; Plexamp "download everything" 206; Emby charges for it | All three | Persistent |
| 4 | Security and sign-in | Jellyfin OIDC 1,191 and 2FA 1,103 votes; #5415 (114 +1); Plex breach and CVEs; Navidrome's 2026 advisory wave | All four | Worse: more advisories in 2026 than ever |
| 5 | Control over watch state and the home screen | Jellyfin "remove from Continue Watching" 1,725, watchlist 1,294, history 830 | Jellyfin, Emby | Persistent, unmet for 5 to 7 years |
| 6 | Needless transcoding and hardware cost | Plex "default to max quality" 1,289 votes, 1,728 posts, 70,974 views; Jellyfin pre-transcode 979; HEVC transcode demand on Plex and Emby | All three | Persistent |
| 7 | Performance at scale and fragile upgrades | Jellyfin lazy loading 1,179; 10.11 regressions and migration failures; database-engine requests on all three boards | Jellyfin most; Plex and Emby too | Bad in late 2025, improving with Jellyfin 12.0 |
| 8 | Music that matches streaming apps | Plex playlists 1,425, tags 866, multi-artist 520, CUE 507; Jellyfin gapless 647, smart playlists 588; HN "Jellyfin as a Spotify alternative" (458 points, 482 comments) | All | Growing as people leave Spotify |
| 9 | Subtitles | Jellyfin #2547 (151 comments); Plex auto-sync 687 votes; Emby delay 229 replies | Jellyfin most | Persistent |
| 10 | Vendor dependence and reliability | Plex local-auth request; July 2026 outage; remote-access breakages; Chrome LAN prompt; Plexamp certificate bug | Plex | Worse |
| 11 | Privacy and unwanted social features | "Week in Review" leak thread (635 posts, 2,704 likes); "disable Discover" 1,000 votes; Discussions backlash; Plexamp telemetry | Plex | Worse |
| 12 | Audiobooks, podcasts and books | Plex books 3,182 (its most-voted suggestion ever) and audiobooks 2,264 (fourth); Jellyfin audiobooks 857, podcasts 563 | Plex, Jellyfin | Persistent; users run a second server |
| 13 | Streaming-service prices pushing people to self-host | Nine-service basket up $702 a year since 2021; Spotify's third US rise since 2023; Netflix rose again in March 2026 | Market-wide | Worse |
| 14 | Administration gaps | Plex "send server messages" 1,206; Jellyfin share links 580, kill stream 402, user groups 202 | All three | Persistent |
| 15 | Client coverage and client quality | Jellyfin Apple TV 463, Vidaa 453; Plex Samsung breakage, desktop neglect, Fire TV complaints | All three | Mixed |
| 16 | Project health and key-person risk | Jellyfin's leader stepped down from burnout (July 2026); Plexamp's author left Plex; a long-time Plex support engineer moved to legacy support | Jellyfin, Plex | New in 2026 |

### 1. Paywalls and price rises on Plex

What happened, in order:

- **2025-04-29:** Plex Pass rose to $6.99 a month and $69.99 a year, and
  lifetime went from $119.99 to $249.99. Remote playback of personal media
  stopped being free. A server owner's Plex Pass covers all their users;
  otherwise each viewer can buy a Remote Watch Pass. MacRumors and TechCrunch
  both described it as the first Plex Pass price rise in about a decade.
  ([MacRumors](https://www.macrumors.com/2025/03/19/plex-price-increase/),
  [TechCrunch](https://techcrunch.com/2025/03/19/streamer-plex-raises-subscription-price-for-the-first-time-in-a-decade);
  Hacker News: 64 points, 73 comments.)
- **Late Nov 2025:** enforcement reached Roku. Plex said every other TV app,
  and third-party clients that use its API to stream remotely, would follow
  in 2026.
  ([Plex forum](https://forums.plex.tv/t/changes-coming-to-remote-streaming-on-roku/933671))
- **2026-03-23:** smart TVs (Samsung, LG, Vizio) and PlayStation and Xbox.
  ([Plex forum](https://forums.plex.tv/t/changes-coming-to-remote-streaming-on-smart-tvs/937015))
- **April 2026:** Fire TV.
  ([Plex forum](https://forums.plex.tv/t/changes-coming-to-plex-on-fire-tv/937614))
- **2026-06-01:** Remote Watch Pass left its introductory price and now
  renews at $2.99 a month or $29.99 a year.
  ([Plex plans page](https://www.plex.tv/plans/))
- **2026-07-01:** a new lifetime Plex Pass went to $749.99. Plex says it
  considered dropping lifetime entirely. The forum reaction thread, "Elephant
  in the room. Lifetime Plex Pass", has 443 posts, 938 likes and 7,877 views.
  ([announcement](https://forums.plex.tv/t/new-lifetime-plex-pass-pricing/938910),
  [reaction thread](https://forums.plex.tv/t/elephant-in-the-room-lifetime-plex-pass/938935),
  [MacRumors](https://www.macrumors.com/2026/05/19/lifetime-plex-pass-price-increase/);
  Hacker News: 61 points, 112 comments.)

A second-order effect: in Nov 2025 Plex warned that Chrome's new Local Network
Access prompt can stop the Plex web app talking to a server in the same house.
If the user declines the prompt, the app falls back to a remote connection and,
without a pass, asks the user to pay.
([Plex forum](https://forums.plex.tv/t/important-note-about-the-plex-web-app-local-network-access/933264))
The cause is structural: the default Plex web app is loaded from app.plex.tv,
a different origin from the user's server. The server also bundles a local
copy of the web app; whether that copy avoids the prompt is (unverified).

Where people go: in the selfh.st 2025 survey (4,081 responses, published
2025-11-21), 2,487 respondents use Jellyfin, 1,219 Plex, 840 Audiobookshelf,
656 Navidrome, 237 Kodi and 149 Emby. Respondents could pick several. Jellyfin
was also the second "favourite app" after Home Assistant (522 against Plex's
208). ([data file](https://raw.githubusercontent.com/selfhst/cdn/refs/heads/main/assets/surveys/annual/2025-results.json))
The survey population is technical, 94% male and mostly container users, so
it over-represents the people most likely to leave Plex.

To be fair to Plex: local streaming is still free. One Plex Pass covers every
user of the server. Many lifetime holders are happy. A How-To Geek piece argued
the paywall is reasonable for a lot of people
([How-To Geek](https://www.howtogeek.com/plex-remote-streaming-paywall-is-actually-fine/)).
Emby's lifetime price of $119 is now about one sixth of Plex's.

### 2. Redesigns that remove features and slow navigation

Plex's "new experience" rewrite reached phones on 2025-03-31, Roku in Nov
2025, Fire TV from April 2026 and Apple TV in Sept 2026, after previews.

- The "New Experience Public Release Feedback" thread has 1,531 posts, 6,486
  likes and 42,918 views.
  ([Plex forum](https://forums.plex.tv/t/new-experience-public-release-feedback/910904))
- "New UI is an awful experience" (Sept 2025): 759 posts, 6,434 likes.
  ([Plex forum](https://forums.plex.tv/t/new-ui-is-an-awful-experience/931048))
- "Vote to roll back to Plex Classic!": 504 votes, 1,230 likes, still active
  in late Sept 2026.
  ([Plex forum](https://forums.plex.tv/t/vote-to-roll-back-to-plex-classic/931767))
- "Rant: New ATV app: no thanks" (2026-09-21): 268 posts and 1,066 likes in
  under two weeks. The opening post says the new player hangs on pause and
  scrubs worse than the native tvOS player. A later Apple TV thread says
  audio offset and embedded-subtitle adjustment are gone.
  ([Plex forum](https://forums.plex.tv/t/rant-new-atv-app-no-thanks-i-want-the-old-one-back/943162))
- Fire TV: "Fire TV app - Hopelessly Crippled" (176 likes), plus several
  "how do I roll back" threads in mid-2026.
  ([Plex forum](https://forums.plex.tv/t/fire-tv-app-hopelessly-crippled/939710))
- Features removed or moved, according to coverage at the time: Watch
  Together, custom server URLs, music playback (moved to Plexamp), playlist
  sharing, DVR scheduling on mobile, and sidebar library pinning
  ([How-To Geek](https://www.howtogeek.com/plex-is-fixing-its-unpopular-new-mobile-apps/),
  [Cord Cutters News](https://cordcuttersnews.com/plex-announces-sleek-new-mobile-app-redesign-but-removes-some-features/)).
  Plex announced the end of Watch Together itself in Feb 2025
  ([Plex forum](https://forums.plex.tv/t/an-important-watch-together-change/906796)).
  The "Add Watch Together to New Plex Experience" suggestion now has 2,878
  votes and 2,622 likes, the second most-voted Plex suggestion of all time
  ([Plex forum](https://forums.plex.tv/t/add-watch-together-to-new-plex-experience/906941)).
- On 2026-08-17 Plex conceded that navigation feedback had been consistent
  since launch and moved primary navigation back to the left edge in preview
  builds ([Plex forum](https://forums.plex.tv/t/navigation-updates-coming-to-preview-users/941751)).

The recurring complaint is that the apps now put Plex's own streaming catalogue
first and the user's library second. In the "Jellyfin as a Spotify
alternative" Hacker News thread, one commenter compared the rewrite to Sonos's
disastrous app update.

To be fair to Plex: it is shipping steadily. Recent examples are mobile
metadata editing (Mar 2026), improved downloads (June 2026), TV show editions
(July 2026), a server dashboard on TV and mobile (July 2026) and dialog boost
with loudness levelling (Sept 2026).

### 3. Offline use

- Jellyfin's most-wanted request is "Support offline mode on Android mobile":
  1,820 votes and 81 comments, filed 2019. The maintainers' note says only
  full-original downloads exist.
  ([features.jellyfin.org #218](https://features.jellyfin.org/posts/218))
  Related: "Offline Sync Feature", 817 votes
  ([#1341](https://features.jellyfin.org/posts/1341)); "Support download of
  transcoded files", 518 ([#57](https://features.jellyfin.org/posts/57));
  "Download a batch", 219 ([#655](https://features.jellyfin.org/posts/655)).
- Plexamp: "Download entire library or playlist", 206 votes
  ([Plex forum](https://forums.plex.tv/t/download-entire-library-or-playlist-in-plexamp/646153)),
  and "Plexamp Seamless Downloads", 71 votes. The Sept 2026 Plexamp 4.50.3
  release rebuilt downloads. Early replies praise faster loading from library
  data cached on the device.
  ([Plex forum](https://forums.plex.tv/t/plexamp-v4-50-3-ready-or-not/942338))
- Emby charges for offline media (Premiere) ([Emby](https://emby.media/premiere.html)).
- From people leaving streaming apps: one Hacker News commenter's biggest
  frustration with YouTube Music is how slowly it falls back to downloads on a
  bad connection. The author of "I left Spotify" missed seamless offline access
  most ([article](https://coppolaemilio.com/entries/i-left-spotify-what-happened-next/)).

The demand is for more than downloads. People want the app to work when the
server or network is gone: browse, search, queue and play whatever is on the
device.

### 4. Security and sign-in

This is the theme where evidence grew fastest in 2025 and 2026.

**Plex**
- **Sept 2025:** a breach exposed emails, usernames, hashed passwords and
  authentication data. Plex asked everyone to reset passwords and sign out
  all devices. The forum notice has 62,526 views. TechCrunch notes Plex had
  forced password resets after an earlier breach just over three years
  before.
  ([Plex forum](https://forums.plex.tv/t/important-notice-of-security-incident/930523),
  [TechCrunch](https://techcrunch.com/2025/09/09/plex-urges-users-to-change-passwords-after-data-breach/);
  Hacker News: 104 points, 92 comments.)
- **Aug 2025:** a server security update (PMS 1.42.1) followed a bug-bounty
  report ([Plex forum](https://forums.plex.tv/t/plex-media-server-security-update/928341)).
- **Jan 2026:** four CVEs published on 2026-01-02. CVE-2025-69414 lets a
  transient token fetch a permanent one. CVE-2025-69416 lets a non-server
  device token retrieve unrelated tokens from plex.tv
  ([NVD](https://nvd.nist.gov/vuln/detail/CVE-2025-69414),
  [NVD](https://nvd.nist.gov/vuln/detail/CVE-2025-69416)). A user asked on
  the forum why they had learned of them from heise and not from Plex
  ([Plex forum](https://forums.plex.tv/t/information-related-to-security-vulnerabilities/935164)).
- **2026-09-01:** another round of server and desktop fixes (PMS 1.43.3).
  CVEs were "requested" at posting time. The notice has 64,221 views.
  ([Plex forum](https://forums.plex.tv/t/important-security-update-for-plex-media-server-v1-43-2-and-earlier/942319))

**Jellyfin**
- Issue #5415, "Collection of potential security issues", was the
  most-reacted issue on the server repo (114 +1, 88 comments). It lists
  unauthenticated video, audio, subtitle and image endpoints, unauthenticated
  WebSockets, weak checks between users, secrets in URLs, and tokens in
  browser local storage. Some items were fixed in 10.8 and 10.9; the issue was
  closed as a duplicate.
  ([GitHub](https://github.com/jellyfin/jellyfin/issues/5415))
- Still open from April 2025: #13991, `GetUser()` fully unauthenticated (25
  +1) ([GitHub](https://github.com/jellyfin/jellyfin/issues/13991)), and
  #13992, granular API key permissions (23 +1).
- On the feature board: OIDC/SSO has 1,191 votes and 2FA has 1,103. Both are
  "planned" and were filed in 2019. A maintainer wrote that OIDC would have to
  be an authentication plugin.
  ([#230](https://features.jellyfin.org/posts/230), [#26](https://features.jellyfin.org/posts/26))
- Jellyfin 12.0 (2026-09-07) closed several path-traversal holes, stopped
  unauthenticated re-runs of the setup wizard, and fixed XSS
  ([release notes](https://jellyfin.org/posts/jellyfin-release-12.0/)).
- In the 12.0 Hacker News thread (623 points, 355 comments), a top comment
  calls it odd prioritisation to ship comic support before making the server
  safe to expose and before native clients can use OIDC.
- jellyfin-web #4076, "Deceptive Site Ahead" (132 comments): Google Safe
  Browsing flagged self-hosted instances as phishing.
  ([GitHub](https://github.com/jellyfin/jellyfin-web/issues/4076))

**Navidrome**
- Navidrome shipped advisories in 0.62 (June 2026), 0.63, 0.64 and 0.64.1
  (2026-09-21). They covered cross-user share disclosure, player takeover,
  Last.fm session hijack, JWT expiry bypass on shares, SQL injection, SSRF
  through playlist artwork and plugins, cross-library file reads, and an
  unthrottled Subsonic login rated CVSS 7.4.
  ([releases](https://github.com/navidrome/navidrome/releases))
- Navidrome's most +1'd open issues are LDAP (122) and better SSO (55).
  ([GitHub](https://github.com/navidrome/navidrome/issues/141))

**Emby**
- 2FA (459 replies) and centralised authentication (216 replies) are among the
  top-reacted requests ([Emby forum](https://emby.media/community/forum/98-feature-requests/)).

The pattern: most real-world bugs are authorisation mistakes (one user
reaching another's data, or endpoints that skip checks), not exotic memory
bugs. Users ask for modern sign-in (OIDC, 2FA, passkeys) everywhere, and
nobody offers it natively on every client.

### 5. Control over watch state and the home screen

Jellyfin's board is the clearest signal. These are all 2019 to 2020 requests
that are still open:

- "Add an option to remove an item from Continue Watching": 1,725 votes
  ([#517](https://features.jellyfin.org/posts/517))
- "Watchlist (like Netflix)": 1,294 ([#576](https://features.jellyfin.org/posts/576))
- "Watched History": 830 ([#633](https://features.jellyfin.org/posts/633))
- "Personal ratings": 453; "combine Next Up and Continue Watching": 203;
  "hide a movie or show": 131; "avoid spoilers": 118
- "Recently Added opens the full series page": 126 votes, filed July 2026,
  plus web bug #8496 (Sept 2026), where playing a show from Recently Added
  starts the newest episode instead of the next one.

On Emby, "Forget a series in Next Up" (166 replies) and "Watchlist" are both
in the top 25 by reactions.

Of the 50 most-wanted Jellyfin requests that are not yet complete (open,
planned or started), 43 were filed in 2019 or 2020. The top 100 such requests
hold 30,961 votes between them. People have asked for these basics for five
to seven years. Plex already does most of this well, and that is one
reason some users go back to it.

A related data-integrity complaint: in Jellyfin 10.11.0, watched state was
lost when media was replaced or renamed (#15001, 61 +1)
([GitHub](https://github.com/jellyfin/jellyfin/issues/15001)).

### 6. Needless transcoding and hardware cost

- Plex "Default All Clients to Max Internet Streaming": 1,289 votes, 1,728
  posts, 70,974 views. The opening post says the server transcodes for no
  reason because clients default to low remote quality. After the user
  converted files to HEVC, the 720p transcode even had a higher bitrate than
  the original.
  ([Plex forum](https://forums.plex.tv/t/default-all-clients-to-max-internet-streaming/440641))
- Jellyfin "Pre-transcoding": 979 votes. The maintainers declined in Nov 2025
  and pointed users to external tools plus multi-version support
  ([#570](https://features.jellyfin.org/posts/570)). Also: "Automatically test
  hardware transcoding" (423), "Option to kill a stream" (402), "Limit
  selectable video quality server-side" (131), "only allow 60 seconds of
  transcoding" (106), "graceful fallback on software encoding" (91).
- HEVC as a transcode target: Plex 491 votes (experimental since Jan 2025);
  Emby "Transcode in H265", 505 replies.
- Hardware support requests: AMD VCN on Plex (848 votes), Raspberry Pi 4
  hardware transcoding (409), Rockchip (70). Emby: "Force DirectPlay only
  without any detection" (165 replies).
- Paywalls make this worse: hardware transcoding needs Plex Pass on Plex and
  Premiere on Emby.

The data supports Gunmetal's thesis. Much transcoding is caused by client
defaults, subtitle burn-in and container mismatches, not by clients that
cannot decode the video. How much is an open question (see below).

### 7. Performance at scale and fragile upgrades

- Jellyfin "Remove pagination / use lazy loading for library view": 1,179
  votes, "started" since 2020
  ([#216](https://features.jellyfin.org/posts/216)).
- Jellyfin 10.11.0 (2025-10-19) moved to EF Core and warned that the first
  start could take hours on large libraries
  ([release notes](https://jellyfin.org/posts/jellyfin-release-10.11.0/)).
  What followed: "Database Migration Failed on update from 10.10 to 10.11"
  (#15027, 121 comments); "Folders make library loading drastically slower"
  (#15141, 39 +1); "API Performance Degradation / Slow UI" (#15352, 32 +1);
  SQL errors on library updates (#15343, 78 comments). The Jan 2026 "State of
  the Fin" counted over 100 changes across four point releases and said some
  migration failures would never be fully resolved
  ([State of the Fin](https://jellyfin.org/posts/state-of-the-fin-2026-01-06/)).
- In the 12.0 Hacker News thread, several users with large libraries had stayed
  on 10.10.7 until 12.0 and report quick, painless upgrades. Another says a
  release candidate was unusable for a large music library. A third blames
  SQLite for every problem, including bad behaviour on copy-on-write
  filesystems.
- Database-engine requests: Jellyfin MySQL (663 votes; PostgreSQL likely
  first), Plex MySQL/Postgres (243), Emby MySQL (391 replies).
- Plex had a package-signing failure on Debian and RHEL that forced a rollback
  of PMS 1.43.0 (Jan 2026, 395 posts, 25,226 views)
  ([Plex forum](https://forums.plex.tv/t/issue-upgrading-to-pms-1-43-0-10467-on-debian-and-rhel-based-distributions/935847)),
  and a "Corrupted Database Upon Upgrade" thread in April 2026
  ([Plex forum](https://forums.plex.tv/t/corrupted-database-upon-upgrade/937827)).
- Jellyfin 12.0 requires a backup first, because the database rewrite cannot
  be undone, and a full rescan afterwards
  ([release notes](https://jellyfin.org/posts/jellyfin-release-12.0/)).

To be fair to Jellyfin: 12.0 seems to have fixed most of the 10.11
performance complaints, after seven release candidates.

### 8. Music that matches streaming apps

What users ask for:

- **Plex:** "Better Playlists" 1,425 votes; "Tag support for ROBUST music
  library organization" 866; "Better support for albums and tracks with
  multiple artists" 520; "CUE support for FLAC files" 507; "Lyrics always
  visible" 260; Apple Spatial Audio 222; multichannel FLAC on Shield 216;
  "Download entire library" 206; "Global Audio Boost and Normalization" 170;
  Android Automotive 119; "By Composer" 89; release-type editing 72.
- **Jellyfin:** "Gapless Playback" 647 (open, no maintainer response);
  "Dynamic (Smart) Playlists" 588; "cuesheet support" 171; "Audio
  crossfading" 138; "Sonic Analysis (similar to Plexamp)" 98 (2025); "Roon
  Like Music Audio Experience" 52; "Group music releases by type" 42. Open
  bug: #14981, favourites lost during playback.
- **Emby:** "Standalone Music App" 175 replies; "Gapless Playback" 108;
  "ReplayGain or R128 volume normalization" 98.
- **Navidrome** (+1 counts): multiple music folders (106, now done); external
  .lrc files (56, done); multiple artists (50, done); smart playlists (47,
  done); podcasts (43, open); artist detail page (26, open); social listening
  and shared radio (25 and 23, open).

What people who leave streaming say they miss:

- "Jellyfin as a Spotify alternative" (Hacker News, 458 points, 482
  comments, April 2025). The most-replied comment (26 replies) says what you
  lose is "Radio": one click from a song to an endless stream of similar music.
  Others: Jellyfin splits multi-disc albums by folder; friends' playlists and
  collaborative "blends" disappear; latency and downtime compared with a big
  cloud service; one user had a corrupt file played at painful volume. Several
  recommend Navidrome over Jellyfin for music and Audiobookshelf for
  audiobooks.
  ([HN](https://news.ycombinator.com/item?id=43711706))
- "I ditched Spotify and set up my own music stack" (279 points, 296
  comments, Sept 2025). Recurring themes: discovery is the one thing streaming
  does better; buying music legally is hard; one commenter left over Spotify
  CEO Daniel Ek's investment in Helsing, a defence-AI company; Lidarr's
  metadata server had reportedly been down for months (unverified); Lyrion
  Music Server is praised for multi-room.
  ([HN](https://news.ycombinator.com/item?id=45133109))

Plexamp is the bar. Commenters in its own forum call it the best music app,
and its free tier includes gapless playback, loudness levelling, Sweet Fades,
smart playlists, CarPlay and Android Auto. But:

- It went more than a year without a release (July 2025 to Sept 2026). A
  Nov 2025 thread was titled "July 14 2025 was last update to Plexamp"
  ([Plex forum](https://forums.plex.tv/t/july-14-2025-was-last-update-to-plexamp/933278)).
- Its author wrote in June 2026 that, as of 2026, he is no longer with Plex,
  though he still maintains Plexamp. He is building a separate player
  engine, Caldera Music, alongside it
  ([Plex forum](https://forums.plex.tv/t/plexamp-updates-and-caldera-music/939563)).
- Plexamp 4.50.3 (Sept 2026) moved to Expo and Tauri. It was forced out early:
  Let's Encrypt's new roots were not in the trust store built into the old
  binary, so servers became unreachable once their certificates renewed. The
  same release sends download telemetry, including track titles and the
  account name, and it cannot be turned off.
  ([Plex forum](https://forums.plex.tv/t/plexamp-v4-50-3-ready-or-not/942338))
- Downloads, lyrics, radio, sonic features, EQ and headless mode all need
  Plex Pass ([Plexamp page](https://www.plex.tv/plexamp/)).

Demand for new music servers is visible too. Hacker News gave Blackcandy 697
points and 298 comments (Dec 2024) and Meelo 156 points (Jan 2025). Navidrome
added an experimental Jellyfin music API in 0.64 (Sept 2026) so Jellyfin music
clients can connect, which shows that client ecosystems, not servers, are
what users stay for.

### 9. Subtitles

- Jellyfin #2547, "Subtitles load after a while and can be out of sync when
  resuming": open since March 2020, 103 +1, 151 comments, the most-reacted
  open issue on the server repo
  ([GitHub](https://github.com/jellyfin/jellyfin/issues/2547)). jellyfin-web
  #4346, "Sub title desync": 35 +1, 62 comments.
- Jellyfin 12.0 Hacker News thread: "Subtitles are the Achilles heel of
  Jellyfin" (user noisy_boy) describes add and remove loops that fail. Another
  user went back to Plex over poor subtitle support and mixed movie and TV
  folders.
- Plex: "Automatic subtitle synchronisation" (687 votes) shipped in Sept 2024.
  "Show names for audio tracks and subtitles in all apps" (417 votes) is still
  open. The new Apple TV app dropped offset adjustment.
- Emby: "Adjust subtitle delay", 229 replies.
- Jellyfin feature board: subtitle offset on Android TV, 179 votes (Dec 2024);
  subtitle appearance, 188; second preferred subtitle language, 95.

### 10. Vendor dependence and reliability

- Plex request for a built-in local authentication server, so a plex.tv
  outage cannot lock you out of your own server: 372 votes, open since 2015
  ([Plex forum](https://forums.plex.tv/t/feature-request-built-in-local-authentication-server-prevent-plex-tv-outage/111339)).
- Plex outage on 2026-07-14: watch.plex.tv and account sign-in were down for
  about two hours. Users noted the status page stayed green
  ([Plex forum](https://forums.plex.tv/t/plex-outage-approx-4-20pm-gmt-service-restored-6-30pm-gmt/940619)).
- Remote access failures: "Plex remote access issues - cannot connect
  securely" (Nov 2025, 635 posts, 11,536 views) and "Remote Access (again)
  seems to be problematic" (Aug 2026, 263 posts).
- Features withdrawn: the TIDAL integration ended on 2024-10-28
  ([Plex forum](https://forums.plex.tv/t/tidal-integration-with-plex-ending-october-28-2024/885728));
  the Alexa skill was shut down on 2026-06-15 for "low usage"
  ([Plex forum](https://forums.plex.tv/t/important-update-regarding-the-plex-alexa-skill/938054));
  Watch Together (2025); desktop apps left largely unmaintained
  ([Plex forum](https://forums.plex.tv/t/are-the-desktop-apps-dead/935518)).
- Third-party metadata breaks things too: a 2026 change in TMDB's date format
  broke Jellyfin image fetching (#16722, 77 +1).

### 11. Privacy and unwanted social features

- Nov 2023, just before this file's window but still cited: Plex's "Week in
  Review" emails told users what their friends had watched on their own
  servers. The thread "Weekly review emails data leak" has 635 posts, 2,704
  likes and 28,689 views. Its opening post says this defeats the point of
  self-hosting.
  ([Plex forum](https://forums.plex.tv/t/weekly-review-emails-data-leak/860206))
- "Fully Disable Discover & search results from Streaming Services": 1,000
  votes, 1,105 posts, implemented in 2022. "Disable reviews": 185 votes.
- Plex added public reviews (Oct 2024), Lists (Mar 2026) and Discussions (June
  2026). "Discussions is a nightmare" has 441 likes and 128 posts and worries
  about moderation and spoilers.
  ([Plex forum](https://forums.plex.tv/t/discussions-is-a-nightmare/939542))
- Plexamp 4.50 download telemetry cannot be turned off (see theme 8).
- Older, but still cited by users: "Stop adding porn with your promoted
  content" (2019).

Privacy is the second most common reason people self-host in the selfh.st
survey (3,520 of 4,081 respondents; hobby is first at 3,862, cost third at
2,596).

### 12. Audiobooks, podcasts and books

- Plex's most-voted suggestion of all time is not about video or music:
  "PLEXREADER: Comics, Books, PDFs" (3,182 votes). "Support for audiobooks"
  is fourth (2,264 votes, 1,157 posts, still active in Sept 2026).
- Jellyfin: audiobook support "started" (857 votes) and podcast support
  (563). 12.0 folded the Bookshelf plugin into the server and improved books
  and comics.
- In the selfh.st survey, Audiobookshelf (840 users) is used more than
  Navidrome (656), and Hacker News commenters describe running Jellyfin for
  video, Navidrome for music and Audiobookshelf for spoken word side by side.

### 13. Streaming-service prices pushing people to self-host

- A tracked basket of nine services (Netflix, Disney+, Hulu, HBO Max, Apple
  TV+, Paramount+, Peacock, YouTube Premium and Spotify) costs $1,852.92 a
  year against $1,150.92 in March 2021: $702 more, or 27% above inflation.
  ([Honestly Ranked](https://honestlyranked.com/guides/streaming-price-increases/);
  Hacker News: 387 points, 385 comments, Sept 2026.)
- Spotify US Premium went from $12 to $13 and Family from $20 to $22 in Feb
  2026, its third US rise since 2023
  ([Sherwood](https://sherwood.news/markets/spotify-increases-its-us-subscription-prices-for-the-third-time-in-3-years/)).
- Netflix raised every US tier again in March 2026, by up to 12.5% according
  to Ars Technica's headline as listed on Hacker News; exact figures
  unverified. "Netflix Prices Went Up Again – I Bought a DVD Player Instead"
  drew 262 points and 278 comments (Apr 2026)
  ([article](https://aywren.com/2026/04/09/netflix-prices-went-up-again-i-bought-a-dvd-player-instead/)).
- Non-price reasons to leave Spotify also trended: the Helsing investment
  (above); ICE recruitment ads in Oct 2025 (Hacker News, 74 points)
  ([DJ Mag](https://djmag.com/news/spotify-defends-running-ice-recruitment-ads-about-dangerous-illegals-part-of-us-government));
  "The Appalling Stupidity of Spotify's AI DJ" (370 points, Mar 2026);
  catalogue removals (a user whose audiobook vanished mid-listen).

The caveat users raise most: self-hosting replaces the library, not the
catalogue. Getting music legally (Bandcamp, CDs) is the bottleneck, and
discovery is weaker.

### 14. Administration gaps

- Plex "Send server messages": 1,206 votes.
- Jellyfin: "Temporary direct file sharing links" 580; "Option to kill a
  stream from a user" 402; "Allow global defaults" 258; "Create a sign up
  page" 227; "user group support" 202; "default user settings" 130; "Message
  to all users" 89.
- Emby: user templates (174 replies), maintenance notice (119), user groups
  (55).
- Plex: per-user bandwidth limits (188), share-a-movie links for people with
  no account (294), "allow managed users to delete content" (155).

These are small, cheap features that admins of family servers keep asking
for. Nobody covers them all.

### 15. Client coverage and quality

- Jellyfin: Apple TV (463, "started"), Hisense Vidaa (453, 119 comments),
  Samsung (253), voice assistants (243), VR headsets (167), iOS Chromecast
  (104), CarPlay (38).
- Plex: Google Home (2,461), Nintendo Switch (320), Apple Watch (298), Meta
  Quest (126).
- Breakage: Plex for Samsung 5.94.3 broke playback for many users in Aug 2026
  (several threads; the largest has 250 posts and 504 likes).
- Hacker News users on Jellyfin: the server is good but the TV apps lag, and
  several fall back to Kodi or third-party clients (Wholphin, Infuse).

### 16. Project health and key-person risk

- On 2026-07-20 Jellyfin's project leader, Joshua Boniface, stepped down
  citing burnout. A long-time core member left at the same time, after
  another departure a few days earlier. The post stresses the hand-over was
  friendly ([Jellyfin forum](https://forum.jellyfin.org/t-project-leadership-changes);
  Hacker News: 370 points, 338 comments).
- Plexamp's author no longer works at Plex (see theme 8). A long-serving Plex
  Linux and NAS support engineer moved to "legacy support mode" in Feb 2026
  ([Plex forum](https://forums.plex.tv/t/eosl-notice-chuckpa-transition-to-legacy-support-mode/936513)).
- Jellyfin published an LLM contribution policy in Jan 2026 (Hacker News, 207
  points) ([policy](https://jellyfin.org/docs/general/contributing/llm-policies/)).
  On the Plex forum, a thread asked whether the apps were being "vibe coded"
  (Aug 2026).

### Why people switch

| From → to | Main reasons given | Evidence |
|---|---|---|
| Plex → Jellyfin | Remote paywall, lifetime price, redesign, privacy, outages, feeling squeezed | selfh.st share; Plex forum threads above; 12.0 HN thread, where a lifetime Plex Pass holder calls Jellyfin "a soft landing" (user doctoboggan) |
| Jellyfin → Plex (back) | Subtitles, polish of TV and Apple apps, watch-state features, mixed movie and TV folders | 12.0 HN thread; Jellyfin board items 517, 576, 633 |
| Plex app → third-party Plex clients | New official apps slower and missing controls | ATV rant thread: the opening poster found other Apple TV clients that "don't hang" |
| Jellyfin music → Navidrome | Tag-based albums, smart playlists, multi-artist, Subsonic client choice | HN Spotify-alternative thread; Navidrome feature list |
| Spotify / YouTube Music / Apple Music → self-hosted | Price rises, ownership, removed catalogue, artist pay, company politics, poor offline fallback | HN threads 43711706 and 45133109; Sherwood; Honestly Ranked |
| Self-hosted → back to streaming | Discovery and radio, catalogue size, convenience, social playlists, maintenance burden | Same threads; the most-replied comments are about radio and maintenance |

### Controversies, 2024 to 2026

| Date | Product | What happened | Source |
|---|---|---|---|
| 2024-08 | Plex | TIDAL integration ends (Oct 28) | Plex forum |
| 2024-10 | Plex | Public user reviews introduced | Plex forum |
| 2025-02 | Plex | Watch Together ends in new apps | Plex forum |
| 2025-03 | Plex | Price rise and remote paywall announced; mobile redesign ships | MacRumors, Plex forum |
| 2025-06 | YouTube | A self-hosted media-centre (LibreELEC) tutorial removed as "harmful", later restored after human review | [Jeff Geerling](https://www.jeffgeerling.com/blog/2025/self-hosting-your-own-media-considered-harmful-updated/) (HN: 1,634 points) |
| 2025-08 | Plex | Server security update after a bug-bounty report | Plex forum |
| 2025-09 | Plex | Account database breach; forced password resets | Plex forum, TechCrunch |
| 2025-10 | Jellyfin | 10.11 EF Core release; migration and performance regressions | GitHub, State of the Fin |
| 2025-10 | Spotify | ICE recruitment ads | DJ Mag |
| 2025-11 | Plex | Roku remote paywall; Chrome LAN prompt problem; remote access failures | Plex forum |
| 2026-01 | Plex | Four CVEs published; PMS 1.43.0 signing rollback | NVD, Plex forum |
| 2026-02 | Spotify | Third US price rise since 2023 | Sherwood |
| 2026-03 | Plex | Remote paywall on smart TVs and consoles | Plex forum |
| 2026-03 | Netflix | US price rise on every plan (exact figures unverified) | HN listing of Ars Technica, Variety |
| 2026-04 | Plex | Fire TV paywall and redesign; Alexa skill shutdown announced | Plex forum |
| 2026-06 | Plex | Discussions launched; Fire TV app backlash | Plex forum |
| 2026-07 | Plex | Lifetime pass goes to $749.99; two-hour outage | Plex forum, MacRumors |
| 2026-07 | Jellyfin | Project leader steps down | Jellyfin forum |
| 2026-08 | Plex | Samsung playback broken; navigation partly reverted | Plex forum |
| 2026-09 | Plex | Server security update; Apple TV app backlash; Plexamp telemetry with no opt-out | Plex forum |
| 2026-09 | Navidrome | Multiple security advisories, including CVSS 7.4 brute force | GitHub releases |
| 2026-09 | Jellyfin | 12.0 released, well received | Jellyfin blog, HN |

## Where Gunmetal can be clearly better

Each idea is tied to a pain point above and to the architecture records. "What
must be true" says what has to hold technically for the claim to stand.

1. **Free remote access with no vendor relay, account or paywall.**
   Answers theme 1 and theme 10. ADR 0001 decision 7 (no central account,
   iroh remote access) and decision 10 (AGPL) mean there is nothing to
   paywall and no plex.tv to go down.
   *What must be true:* iroh hole-punching has to connect reliably from
   phones on mobile networks and from TVs, with a relay fallback that is
   either self-hostable or run by someone who cannot read the traffic. Pairing
   a new device has to be as easy as plex.tv/link (a short code or QR shown
   on the TV, approved from a signed-in phone). We should publish connection
   success rates the way we will publish scan benchmarks.

2. **Security as a headline feature, and provable.** Answers theme 4. ADR
   0001 decision 6 (random IDs, per-object authorisation, short-lived signed
   stream URLs) and decision 3 (sandboxed FFmpeg), plus memory-safe parsing
   in the core.
   *What must be true:* every route goes through one authorisation layer.
   A test matrix checks every endpoint against unauthenticated, other-user
   and wrong-scope callers, so issues like Jellyfin #13991 and Navidrome's
   IDORs fail CI. Login and pairing are rate-limited from day one. Tokens
   never appear in URLs except as single-use, short-lived, scope-limited
   stream signatures. Passkeys and OIDC work in every first-party client,
   including TVs through the pairing flow, not just the web. Ship a written
   threat model, a security.txt and a GitHub advisory process before the first
   release. The Jellyfin and OpenSubsonic adapters must not reopen holes the
   native API closes (see Risks).

3. **No transcoding as the default outcome.** Answers theme 6. ADR 0001
   decisions 1, 3 and 4: libmpv clients, remux before transcode, segment
   maps at scan time.
   *What must be true:* first-party clients default to original quality and
   only step down when measured bandwidth requires it. This is the inverse of
   Plex's default, which has a 1,289-vote request against it. Subtitles render
   on the client (libmpv handles ASS and PGS), so subtitles never force a
   burn-in. The server reports why it chose transcode over direct play, so
   admins can fix the cause ("stats for nerds", 219 votes). Hardware
   transcoding, when it does run, needs a startup self-test (423 votes) and no
   licence check.

4. **Offline-first clients.** Answers theme 3, the top Jellyfin request.
   The README commits to syncing the library to each device; ADR 0002
   commits to offline downloads.
   *What must be true:* the sync protocol sends small deltas, and the whole
   music library's metadata fits comfortably on a phone (to be measured). It
   must support "download everything" and rules-based downloads (for example,
   "my five-star playlists", as one Hacker News user does by hand). Audio for
   downloads can be transcoded to Opus cheaply, which ADR 0002 already allows.
   Video downloads need a transcode-to-size path, so they are later and gated
   by the sandbox.

5. **Watch state you own and control.** Answers themes 5 and 7. ADR 0001
   decision 5 makes watch history an append-only, exportable log.
   *What must be true:* history entries are keyed to a stable content
   identity (external IDs plus a content fingerprint), not to the file path,
   so replacing or renaming files keeps progress (Jellyfin #15001). Ship from
   v1 the things that have waited years elsewhere: "remove from Continue
   Watching", watchlist, browsable history, personal ratings, and one combined
   Next Up row. Import from Plex and Jellyfin is a switching aid; the 12.0 HN
   thread shows people already do this by hand.

6. **Upgrades that cannot lose your library.** Answers theme 7. ADR 0001
   decision 5 treats SQLite as a rebuildable cache.
   *What must be true:* everything not derivable from the files lives in
   append-only logs: watch history, and also user metadata edits, playlists,
   ratings and settings. Otherwise "rebuildable" is false. Then a migration
   can always be "rebuild the cache from the files plus the logs", and a
   failed upgrade is a rescan, not a restore. Rebuild time on a large
   library has to be minutes, not hours; the scan benchmark against Jellyfin
   should report it.

7. **A music player people would leave Spotify for.** Answers theme 8.
   ADR 0002 decisions 3 to 5: scan-time loudness and gapless metadata, a
   real music model, gapless, normalisation, synced lyrics, queue hand-off.
   *What must be true:* we need parity with Plexamp's free tier on day one
   (gapless, loudness, crossfade, smart playlists, CarPlay and Android Auto)
   plus what Plexamp charges for (downloads, lyrics, EQ), free. The music
   model must handle multiple artists, release types, multi-disc albums by
   tag, composers and roles; this is where Navidrome beats Plex and Jellyfin.
   Lyrics must cover LRC, enhanced LRC and TTML. "Radio" is the hardest gap.
   A local option is audio-feature similarity computed at scan time; that
   means decoding audio, which conflicts with the low-spec goal unless it
   runs as an opt-in, low-priority, resumable job. A network option is
   ListenBrainz through a plugin with an explicit network grant, as ADR 0002
   requires. Queue hand-off must work between our own clients over the same
   iroh channel.

8. **No telemetry, no social feed, nothing leaves by default.** Answers
   theme 11. Follows from no central account (ADR 0001) and plugins with
   network grants (ADR 0002).
   *What must be true:* crash reports and diagnostics are opt-in, show
   exactly what would be sent, and never include titles or usernames unless
   the user ticks a box for that report. Any household "what's on" feature
   stays inside the server.

9. **Admin basics done properly.** Answers theme 14. These are cheap:
   kill a stream, message all users, user templates and groups, invite links,
   expiring share links (which reuse the short-lived signed URLs from ADR
   0001), per-user bandwidth caps, a sessions dashboard.
   *What must be true:* each one goes through the same authorisation layer
   as item 2. Share links must carry scoped, expiring capabilities, not
   reusable tokens.

10. **Compatibility adapters as an on-ramp, not a crutch.** Answers theme 15.
    ADR 0001 decision 6 (Jellyfin adapter, which also covers Roku) and ADR
    0002 decision 6 (OpenSubsonic). Navidrome's experimental Jellyfin music
    API shows users stay for clients (Finamp, Symfonium), not servers.
    *What must be true:* adapters are separately enabled, rate-limited and
    scoped, with tokens that expire. Navidrome's Jellyfin tokens never
    expire, which is the mistake to avoid. Adapter traffic must not bypass
    per-object authorisation.

11. **Watch together with no accounts for guests.** Answers the 2,878-vote
    Plex request and Jellyfin's invite-link request (267 votes).
    *What must be true:* a host can mint an expiring guest capability tied to
    one item and one session, delivered over iroh. Guests need no account and
    see nothing else. This is after v1, but the capability model should allow
    it from the start.

12. **Honest benchmarks.** Answers theme 7 and Gunmetal's own credibility.
    The README already commits to publishing scan-time numbers against
    Jellyfin either way. Add time to first frame, idle memory, library-browse
    latency at 100,000 tracks, and remote connection success.

## Risks and hard parts

- **Client breadth is where rivals actually win.** Users judge servers by
  their TV apps. Samsung, LG, Vidaa and Roku cannot run libmpv, so they fall
  back to the browser-style remux path or to the Jellyfin adapter. Plex, with
  a funded team, is visibly struggling with Apple TV and Fire TV quality in
  2026. A React Native TV client that is worse than Infuse or Plexamp would
  cancel out a better server.
- **The remuxer** (Dolby Vision, lossless audio, image subtitles) is the main
  schedule risk, as ADR 0001 says. Until it exists, browsers and smart TVs
  have no cheap path.
- **Discovery without a catalogue.** The most-replied complaints about leaving
  Spotify are about radio, recommendations and social playlists. A
  self-hosted library can only partly answer this. External data
  (ListenBrainz, MusicBrainz) means network calls that must stay opt-in.
  Sonic analysis means decoding every track, which strains low-power hardware.
- **iroh in the real world.** NAT traversal from carrier networks, mobile
  background limits, TV platforms that restrict sockets, and the question of
  who runs relays (cost, trust, abuse) are all unproven for media streaming
  at video bitrates (unverified).
- **Adapters drag in old protocol weaknesses.** Subsonic's classic token
  scheme is built on the user's password and pushes servers to store
  recoverable passwords (unverified detail; OpenSubsonic has added
  alternatives, unverified). The Jellyfin API's design is the subject of
  #5415. Shipping either adapter risks inheriting the security problems
  Gunmetal is meant to fix.
- **Being "the secure one" draws attention.** Navidrome's 2026 advisories came
  after researchers took a closer look. Gunmetal needs an advisory process,
  fuzzing of the parsers and response capacity before it markets security.
- **"Rebuildable cache" can quietly become false.** If user edits, playlists
  or settings end up only in SQLite, the cache is no longer rebuildable and
  the migration story collapses.
- **Third-party metadata is fragile.** A TMDB format change broke Jellyfin
  images in 2026; Lidarr's metadata server outage (reported, unverified
  duration) stranded music users. Gunmetal must degrade gracefully when
  providers change or vanish.
- **Key-person risk and velocity.** Jellyfin's leader burned out and
  Plexamp's author left Plex. Gunmetal's strict test-first gate (100%
  coverage, zero surviving mutants) protects quality but slows features
  against a demand list in the thousands of votes. The plan has to choose
  carefully.
- **Perception and platform policy.** Self-hosted media is sometimes treated
  as piracy tooling (YouTube's 2025 takedown of a LibreELEC video; Google Safe
  Browsing flagging Jellyfin instances). App-store review and safe-browsing
  flags could hit Gunmetal's clients and default domains (unverified as a
  current risk).
- **Incumbents are moving.** Jellyfin 12.0 was well received and fixed much of
  10.11; Plex is shipping monthly and has walked back some redesign choices;
  Navidrome is shipping security and feature releases monthly.

## Open questions

1. **Audiobooks and podcasts:** do they belong in v1, or do we interoperate
   with Audiobookshelf? Demand is very strong (books are Plex's most-voted
   request and audiobooks its fourth), but ADR 0002 schedules books after v1
   and says nothing about audiobooks or podcasts.
2. **Relays:** do we run any iroh relay infrastructure? If so, who pays, and
   how do we keep it unable to observe or block users? If not, how do
   non-technical users get remote access behind CGNAT?
3. **Measuring avoidable transcodes:** how much of rivals' transcoding is
   avoidable (client defaults, subtitles, containers) and how much is real
   codec incompatibility? Our benchmark should measure it on a real library
   rather than assume it.
4. **Radio and similarity:** should music radio rely on local audio analysis
   (CPU cost, opt-in), on external data through a plugin (privacy), or on
   both?
5. **Watch together:** does it ship in v1? It is a 2,878-vote Plex request and
   a reason some users stay on Jellyfin, but it is not in either ADR.
6. **Which TV platforms at launch,** and is the Jellyfin adapter on Roku good
   enough to count as coverage?
7. **Migration tools:** should we ship importers for Plex and Jellyfin watch
   history, ratings and playlists at v1? Switching cost is the main thing
   that keeps lifetime Plex users where they are.
8. **Sustainability without a paywall:** how is the project funded and staffed
   so it does not end up where Jellyfin's leader did? Jellyfin's 2024 "We're
   good, seriously" post asked people to stop donating, but money did not
   prevent burnout.
9. **Live TV priority:** demand is real (IPTV channel groups 376 votes;
   multiple EPG sources 611; pseudo-channels 622) but smaller than offline,
   auth and watch-state demand. Should M3U stay as the first module after v1?
10. **Diagnostics:** what is the opt-in diagnostics design that lets us debug
    sync and downloads (Plexamp's stated reason for its telemetry) without
    collecting titles or identities?

## Sources

Feature boards and issue trackers (counts pulled 2026-10-02):

- https://features.jellyfin.org/api/v1/posts?view=most-wanted&limit=100
- https://features.jellyfin.org/posts/218
- https://features.jellyfin.org/posts/517
- https://features.jellyfin.org/posts/576
- https://features.jellyfin.org/posts/230
- https://features.jellyfin.org/posts/216
- https://features.jellyfin.org/posts/26
- https://features.jellyfin.org/posts/570
- https://features.jellyfin.org/posts/633
- https://features.jellyfin.org/posts/1341
- https://features.jellyfin.org/posts/57
- https://features.jellyfin.org/posts/655
- https://forums.plex.tv/c/feature-suggestions/8/l/votes.json
- https://forums.plex.tv/c/feature-suggestions/8/l/top.json?period=all
- https://forums.plex.tv/top.json?period=yearly
- https://forums.plex.tv/c/announcements/5.json
- https://emby.media/community/forum/98-feature-requests/
- https://emby.media/community/topic/117120-how-to-look-at-most-upvoted-feature-requests/
- https://api.github.com/search/issues?q=repo:jellyfin/jellyfin+is:issue&sort=reactions-%2B1&order=desc
- https://api.github.com/search/issues?q=repo:jellyfin/jellyfin-web+is:issue&sort=reactions-%2B1&order=desc
- https://api.github.com/search/issues?q=repo:navidrome/navidrome+is:issue&sort=reactions-%2B1&order=desc
- https://github.com/jellyfin/jellyfin/issues/5415
- https://github.com/jellyfin/jellyfin/issues/13991
- https://github.com/jellyfin/jellyfin/issues/2547
- https://github.com/jellyfin/jellyfin/issues/15001
- https://github.com/jellyfin/jellyfin-web/issues/4076
- https://github.com/navidrome/navidrome/issues/141
- https://github.com/navidrome/navidrome/releases

Plex forum topics:

- https://forums.plex.tv/t/plexreader-comics-books-pdfs/26684
- https://forums.plex.tv/t/add-watch-together-to-new-plex-experience/906941
- https://forums.plex.tv/t/google-home-integration/237823
- https://forums.plex.tv/t/support-for-audiobooks/27518
- https://forums.plex.tv/t/implemented-two-factor-authenticator-for-plex-account/125454
- https://forums.plex.tv/t/better-playlists/73590
- https://forums.plex.tv/t/default-all-clients-to-max-internet-streaming/440641
- https://forums.plex.tv/t/send-server-messages/23773
- https://forums.plex.tv/t/implemented-fully-disable-discover-search-results-from-streaming-services-more-ways-to-watch/786259
- https://forums.plex.tv/t/tag-support-for-robust-music-library-organization/106326
- https://forums.plex.tv/t/better-support-for-albums-and-tracks-with-multiple-artists/116658
- https://forums.plex.tv/t/cue-support-for-flac-files/96352
- https://forums.plex.tv/t/vote-to-roll-back-to-plex-classic/931767
- https://forums.plex.tv/t/automatic-subtitle-synchronisation/381054
- https://forums.plex.tv/t/feature-request-built-in-local-authentication-server-prevent-plex-tv-outage/111339
- https://forums.plex.tv/t/download-entire-library-or-playlist-in-plexamp/646153
- https://forums.plex.tv/t/global-audio-boost-and-normalization/700726
- https://forums.plex.tv/t/new-experience-public-release-feedback/910904
- https://forums.plex.tv/t/new-ui-is-an-awful-experience/931048
- https://forums.plex.tv/t/rant-new-atv-app-no-thanks-i-want-the-old-one-back/943162
- https://forums.plex.tv/t/fire-tv-app-hopelessly-crippled/939710
- https://forums.plex.tv/t/navigation-updates-coming-to-preview-users/941751
- https://forums.plex.tv/t/elephant-in-the-room-lifetime-plex-pass/938935
- https://forums.plex.tv/t/new-lifetime-plex-pass-pricing/938910
- https://forums.plex.tv/t/changes-coming-to-remote-streaming-on-roku/933671
- https://forums.plex.tv/t/changes-coming-to-remote-streaming-on-smart-tvs/937015
- https://forums.plex.tv/t/changes-coming-to-plex-on-fire-tv/937614
- https://forums.plex.tv/t/important-note-about-the-plex-web-app-local-network-access/933264
- https://forums.plex.tv/t/important-notice-of-security-incident/930523
- https://forums.plex.tv/t/plex-media-server-security-update/928341
- https://forums.plex.tv/t/important-security-update-for-plex-media-server-v1-43-2-and-earlier/942319
- https://forums.plex.tv/t/information-related-to-security-vulnerabilities/935164
- https://forums.plex.tv/t/issue-upgrading-to-pms-1-43-0-10467-on-debian-and-rhel-based-distributions/935847
- https://forums.plex.tv/t/corrupted-database-upon-upgrade/937827
- https://forums.plex.tv/t/an-important-watch-together-change/906796
- https://forums.plex.tv/t/important-update-regarding-the-plex-alexa-skill/938054
- https://forums.plex.tv/t/tidal-integration-with-plex-ending-october-28-2024/885728
- https://forums.plex.tv/t/are-the-desktop-apps-dead/935518
- https://forums.plex.tv/t/plex-outage-approx-4-20pm-gmt-service-restored-6-30pm-gmt/940619
- https://forums.plex.tv/t/plex-remote-access-issues-cannot-connect-securely/933721
- https://forums.plex.tv/t/remote-access-again-seems-to-be-problematic/942115
- https://forums.plex.tv/t/playback-error-on-samsung-tv-an-unexpected-playback-problem-occurred/941876
- https://forums.plex.tv/t/weekly-review-emails-data-leak/860206
- https://forums.plex.tv/t/discussions-is-a-nightmare/939542
- https://forums.plex.tv/t/introducing-discussions-on-plex/939487
- https://forums.plex.tv/t/july-14-2025-was-last-update-to-plexamp/933278
- https://forums.plex.tv/t/plexamp-updates-and-caldera-music/939563
- https://forums.plex.tv/t/plexamp-v4-50-3-ready-or-not/942338
- https://forums.plex.tv/t/audio-enhancements-boost-dialog-normalize-loudness/942925
- https://forums.plex.tv/t/server-dashboard-release/941143
- https://forums.plex.tv/t/improved-downloads-now-available-on-ios-and-android/939519
- https://forums.plex.tv/t/hevc-encoding-experimental-public-release/903017
- https://forums.plex.tv/t/plex-for-mobile-tvs-android-mobile-ios-android-tv-tvos-fire-tv/909610
- https://forums.plex.tv/t/eosl-notice-chuckpa-transition-to-legacy-support-mode/936513
- https://forums.plex.tv/t/tv-show-editions-release/940424
- https://forums.plex.tv/t/metadata-editing-on-mobile/937193
- https://forums.plex.tv/t/plex-nfo-agent-forum-preview/936104
- https://forums.plex.tv/t/announcement-custom-metadata-providers/934384

Official pages, release notes and project posts:

- https://www.plex.tv/plans/
- https://www.plex.tv/plexamp/
- https://emby.media/premiere.html
- https://emby.media/community/forum/17-announcements/
- https://emby.media/community/topic/145657-emby-server-493-released/
- https://emby.media/community/topic/149213-fresh-new-look/
- https://jellyfin.org/posts/jellyfin-release-12.0/
- https://jellyfin.org/posts/jellyfin-release-10.11.0/
- https://jellyfin.org/posts/jellyfin-release-10.10.0/
- https://jellyfin.org/posts/state-of-the-fin-2026-01-06/
- https://jellyfin.org/docs/general/contributing/llm-policies/
- https://forum.jellyfin.org/t-project-leadership-changes
- https://www.navidrome.org/docs/overview/
- https://nvd.nist.gov/vuln/detail/CVE-2025-69414
- https://nvd.nist.gov/vuln/detail/CVE-2025-69416
- https://raw.githubusercontent.com/selfhst/cdn/refs/heads/main/assets/surveys/annual/2025-results.json
- https://selfh.st/survey/2025-results/

Hacker News (via hn.algolia.com API):

- https://news.ycombinator.com/item?id=49604861 (Jellyfin 12.0)
- https://news.ycombinator.com/item?id=48986091 (Jellyfin leadership change)
- https://news.ycombinator.com/item?id=43711706 (Jellyfin as a Spotify alternative)
- https://news.ycombinator.com/item?id=45133109 (I ditched Spotify)
- https://news.ycombinator.com/item?id=45174707 (Plex security incident)
- https://news.ycombinator.com/item?id=48193055 (New lifetime Plex Pass pricing)
- https://news.ycombinator.com/item?id=43422965 (Plex no longer offers free remote playback)
- https://news.ycombinator.com/item?id=49641215 (streaming subscriptions cost $702 more)
- https://news.ycombinator.com/item?id=47709251 (Netflix prices, DVD player)
- https://news.ycombinator.com/item?id=47544785 (Netflix raises prices, Ars Technica)
- https://news.ycombinator.com/item?id=46649168 (Spotify third US price rise)
- https://news.ycombinator.com/item?id=45664751 (Spotify ICE ads)
- https://news.ycombinator.com/item?id=42512896 (Blackcandy)
- https://news.ycombinator.com/item?id=42850109 (Meelo)
- https://news.ycombinator.com/item?id=39941232 (Kyoo)
- https://news.ycombinator.com/item?id=46801976 (Jellyfin LLM policy)
- https://news.ycombinator.com/item?id=44197932 (YouTube and self-hosting)
- https://hn.algolia.com/api/v1/items/49604861
- https://hn.algolia.com/api/v1/items/43711706
- https://hn.algolia.com/api/v1/items/45133109

Press and articles:

- https://www.macrumors.com/2025/03/19/plex-price-increase/
- https://www.macrumors.com/2026/05/19/lifetime-plex-pass-price-increase/
- https://techcrunch.com/2025/03/19/streamer-plex-raises-subscription-price-for-the-first-time-in-a-decade
- https://techcrunch.com/2025/09/09/plex-urges-users-to-change-passwords-after-data-breach/
- https://www.howtogeek.com/plex-is-fixing-its-unpopular-new-mobile-apps/
- https://www.howtogeek.com/plex-remote-streaming-paywall-is-actually-fine/
- https://cordcuttersnews.com/plex-announces-sleek-new-mobile-app-redesign-but-removes-some-features/
- https://honestlyranked.com/guides/streaming-price-increases/
- https://sherwood.news/markets/spotify-increases-its-us-subscription-prices-for-the-third-time-in-3-years/
- https://aywren.com/2026/04/09/netflix-prices-went-up-again-i-bought-a-dvd-player-instead/
- https://djmag.com/news/spotify-defends-running-ice-recruitment-ads-about-dangerous-illegals-part-of-us-government
- https://coppolaemilio.com/entries/i-left-spotify-what-happened-next/
- https://www.jeffgeerling.com/blog/2025/self-hosting-your-own-media-considered-harmful-updated/
