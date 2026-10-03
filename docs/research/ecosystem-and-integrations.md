# Ecosystem and integrations

Research note: written on 2026-10-02 with web access. Web search was used until the
session's shared search budget ran out; after that, pages were read directly, and
GitHub's API and the Jellyfin feature board's API were queried for issue, vote and
star counts. Every count below is as read on 2026-10-02. Claims that could not be
checked against a source are marked "(unverified)".

## Scope

This file covers everything that sits around a media server rather than inside its
player: the public API and API compatibility (the Jellyfin API and OpenSubsonic),
plugins and how they are sandboxed, webhooks and notifications, request managers
(Seerr, formerly Overseerr and Jellyseerr), the download-automation stack (Sonarr,
Radarr, Lidarr, Prowlarr and friends), subtitle automation (Bazarr), watch-state
tracking and sync services (Trakt, Simkl, WatchState), music scrobblers (Last.fm,
ListenBrainz), statistics dashboards (Tautulli, Jellystat, Streamystats, Tracearr),
metadata and collection managers (Kometa, Posterizarr, Maintainerr), smart-home
integration (Home Assistant, Music Assistant, voice assistants), Discord presence,
user onboarding tools, and data portability.

It does not cover casting (Chromecast, AirPlay, DLNA), the sign-in design itself, or
the player, except where an integration depends on them. Sibling research files own
those areas.

The architecture records constrain what Gunmetal can do here, so every
recommendation below is checked against them:

- The native API comes first. It uses random IDs, per-object authorisation and
  short-lived signed stream URLs. Jellyfin API compatibility is an optional adapter
  (record 1, decision 6), and OpenSubsonic plays the same role for music (record 2,
  decision 6).
- Scrobbling and metadata lookups reach third-party services, so they belong in
  plugins with explicit network grants, not in the server itself (record 2,
  consequences).
- SQLite is a rebuildable cache. Watch history is the only irreplaceable data and
  lives in an append-only, exportable log (record 1, decision 5).
- There is no central account. Sign-in is by passkeys and OIDC with device-bound
  keys, and remote access runs over iroh (record 1, decision 7).
- Gunmetal ships its own clients (record 1, decision 1), which means some
  integrations that rivals need a separate helper app for (Discord presence,
  for example) can live in our own client.

## Feature inventory

Columns are Plex, Jellyfin and Emby unless a sub-heading says otherwise. "Best in
class" names whoever does it best today, which is often a third-party tool rather
than a server. Vote counts come from the Jellyfin feature board
(features.jellyfin.org) or the Plex forum; reaction counts come from GitHub.

### Public API and developer surface

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Published API reference | Tools can be built without reverse-engineering the server | Official Plex Media Server API reference published in 2025; versioned, with API 1.0.0 needing PMS 1.41.9 or newer | OpenAPI document generated from the server; 12.0 says endpoints missing from the spec must not be used | API documented on its developer site (depth unverified) | Jellyfin, for a spec that has been public for years; Plex caught up in 2025 | Gunmetal should generate its spec from the Rust protocol types so it cannot drift |
| Official SDKs | Typed client libraries in common languages | None official; python-plexapi is community-run | TypeScript, Kotlin and Swift SDKs maintained by the project, all active in late 2026 | (unverified) | Jellyfin | 12.0 updated its OpenAPI generator, so every SDK had to be regenerated |
| Deprecation policy | Integrations do not break without warning | No published policy (unverified) | New in 12.0: deprecations stay marked for a whole major cycle before removal, but endpoints outside the spec can go in any major release | (unverified) | Jellyfin, on paper | The policy did not stop 12.0 from breaking Sonarr's connector (see pain points) |
| Authentication for third-party tools | A tool proves who it is acting for | `X-Plex-Token` header or query parameter; legacy long-lived tokens plus 7-day JWTs since September 2025 | `Authorization: MediaBrowser` header or `ApiKey` query parameter; older headers and `api_key` disabled by default in 12.0 | Emby-style token headers; Emby 4.9.5 accepts both old and new forms, per a Sonarr PR | Plex is moving to short-lived, refreshable tokens | All three still accept tokens in query strings, which leak into logs |
| Scoped, least-privilege API keys | A dashboard cannot delete your library | No scopes (unverified) | Dashboard API keys act with admin rights (unverified) | (unverified) | No media server; OpenSubsonic's API-key extension at least requires listing and revoking keys | This is the root of the third-party security problems listed under pain points |
| Token lifetime and rotation | A leaked key stops working on its own | 7-day JWTs, refreshable; legacy tokens still accepted | Long-lived keys (unverified) | (unverified) | Plex | Plex's JWT rollout left developers without working examples, per its own forum thread |
| Real-time event stream | Dashboards get pushed events instead of polling | Server notifications stream; Tracearr consumes Plex events over server-sent events | WebSocket session events exist, but Jellystat polled every second until users reported load problems | Tracearr needs a plugin for instant Emby events | Plex | Polling tools have caused thread-pool starvation on Jellyfin (Jellystat issue 328) |
| Delta (incremental) sync | Clients and tools fetch only what changed | (unverified) | Kodi Sync Queue plugin provides it for the Kodi add-on; a general "differential API" request has 6 votes | (unverified) | Jellyfin, via an official plugin | Gunmetal syncs the library to devices, so a delta API is core, not optional |
| Unauthenticated endpoint surface | Strangers cannot map or stream your library | (unverified) | Issue 5415 (2021) catalogued unauthenticated stream, subtitle, image and user endpoints; 12.0 continued path-traversal hardening | (unverified) | — | Gunmetal's signed, short-lived stream URLs avoid this class of problem |
| Library-update API for outside tools | A downloader can say "this folder changed" | Used by the Plex connector in Sonarr and Lidarr (Radarr too, unverified) | `/Library/Media/Updated` with paths, used by Sonarr's "MediaBrowser" connector | Same MediaBrowser connector | Plex and Jellyfin, roughly equal | Lidarr can also notify Subsonic servers through `startScan` |
| Metrics endpoint | Prometheus and Grafana can watch the server | None (unverified) | `/metrics` exporter merged in 2020 | (unverified) | Jellyfin | Request for it had 56 votes |
| Agent and LLM access | Ask an assistant about your library | An OAuth-protected remote MCP server was reported alongside the 2025 API docs (unverified) | Community MCP servers (unverified) | (unverified) | Streamystats ships an AI chat over Jellyfin history | Emerging; low priority but cheap if scoped tokens exist |

### Compatibility with existing clients

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Jellyfin API compatibility | Existing Jellyfin apps (Finamp, Jellify, Feishin, Swiftfin, Kodi add-on) can connect | No | Native | Shared ancestry; 12.0 removed the `/emby` and `/mediabrowser` route prefixes | Navidrome 0.64.0 (September 2026) shipped an experimental Jellyfin music API, tested against Finamp and Jellify | Direct precedent for Gunmetal's adapter: a subset, off by default, with no new data model |
| Subsonic and OpenSubsonic API | Music apps such as Symfonium, Feishin, Amperfy, Supersonic and Tempus connect | No | No; a 77-vote request has been "started" since 2020, pointing to jellysub, archived in 2022 | No (unverified) | Navidrome, gonic, Ampache, LMS and others listed by OpenSubsonic | OpenSubsonic lists ten servers and nine clients as participants |
| Extension discovery | Clients learn which optional features a server has | — | — | — | OpenSubsonic `getOpenSubsonicExtensions` (lyrics, transcoding, playback report, sonic similarity, index-based queue, API keys and more) | Lets Gunmetal advertise only what it really implements |
| Password-free auth for compatible clients | Subsonic clients work without the server storing a password | — | — | — | OpenSubsonic `apiKeyAuthentication`: revocable keys, and a recommendation to drop salted-token auth | The classic Subsonic token scheme needs a plaintext-equivalent password, which a passkey-only account does not have |
| Documented client quirks | Each supported app actually works | — | — | — | Navidrome documents per-client workarounds (ID lengths for Finamp, case-insensitive paths for Jellify, `AlbumIds` for Feishin) | Gunmetal needs the same, plus tests against real apps |
| Kodi as a client | Kodi's native player against your server | Third-party add-on (unverified) | Official Kodi add-on plus Kodi Sync Queue plugin | Add-on (unverified) | Jellyfin | Low priority, since Gunmetal ships its own players |
| Music Assistant source | Play the library through Home Assistant to Sonos, AirPlay, Cast and DLNA players | Supported | Supported | Supported | Music Assistant also supports Subsonic servers, with a scrobbler module | Gunmetal gets this for free through the OpenSubsonic adapter |
| Long-lived compatibility guarantees | Old apps keep working after upgrades | (unverified) | Old third-party clients that relied on legacy routes or auth broke in 12.0 | (unverified) | — | An adapter should pin a target Jellyfin API version and say so |

### Plugins and the extension model

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Third-party plugins | Extend the server without forking it | No third-party plugin system today; legacy plugins and channels were retired years ago (date unverified) | Yes: .NET DLLs dropped into the plugins folder; official and third-party repositories | Yes: plugin catalog; some features gated behind Premiere (secondary source) | Jellyfin for breadth, Navidrome for safety | Jellyfin's official list includes LDAP, Trakt, Playback Reporting, Reports, Kodi Sync Queue, OpenSubtitles and anime metadata providers |
| Sandbox | A buggy or hostile plugin cannot take over the server | n/a | None; plugins run in the server process and can add REST controllers and background services | None (unverified) | Navidrome: WebAssembly via Extism, no direct filesystem access beyond granted library paths, plugins isolated from each other | Matches record 2's "explicit network grants" exactly |
| Declared permissions | The admin sees what a plugin can touch before enabling it | n/a | No permission model | (unverified) | Navidrome manifest: required hosts, storage, library read, user scoping, scheduler, WebSocket, cache, each with a stated reason | — |
| Network allowlist with SSRF guard | A plugin can call only the hosts it declared, and not your LAN | n/a | None | None | Navidrome 0.64.0 checks the resolved IP when connecting and blocks private and loopback addresses unless listed | This came after two advisories showing DNS names, redirects and WebSockets bypassed the first guard |
| Per-user plugin settings | Each person links their own Last.fm or Trakt account | n/a | Missing; the ListenBrainz plugin says the admin must configure it for every user; two requests (4 votes each) | (unverified) | Navidrome's built-in scrobblers authorise per user | Also a privacy issue: the admin holds everyone's tokens |
| Settings UI generated from a schema | Plugins get settings screens without shipping their own web code | n/a | Plugins ship their own HTML and JavaScript config pages | (unverified) | Navidrome 0.60.0 renders plugin settings with a JSONForms-based interface | Safer than letting plugins inject script into the admin UI |
| Stable plugin ABI across upgrades | Upgrading the server does not break every plugin | n/a | 12.0 needs all third-party plugins removed first and rebuilt for .NET 10; 10.11.9 also broke repository lookups (issue 16905) | (unverified) | Nobody has solved this | A versioned WebAssembly interface (WIT) is the obvious answer |
| Catalog trust | Know who built a plugin and that it was not tampered with | n/a | Official manifest URL; extra repositories added by URL; installer rejected unsafe package names only from 12.0 | Curated catalog (secondary source) | No signing anywhere (unverified) | Gunmetal should sign first-party plugins and show the publisher |
| Plugin languages | Community can write plugins in a familiar language | n/a | C# only | .NET (unverified) | Navidrome PDKs for Go, Rust, Python and JavaScript | — |
| Extension points | What a plugin may hook into | n/a | Metadata, auth providers, notifications, scheduled tasks, REST endpoints, and from 12.0 search providers and recommendation data | (unverified) | Jellyfin for breadth; Navidrome covers metadata agents, scrobblers, lyrics, sonic similarity, scheduler and task queues | Start narrow and safe |
| Isolated plugin storage | Removing a plugin removes its data | n/a | Per-plugin config files (unverified) | (unverified) | Navidrome: per-plugin cache namespace and, from 0.64.0, plugin storage | — |
| Resource limits | A plugin cannot pin the CPU or eat memory | n/a | None | None | None documented by any rival | WebAssembly runtimes support fuel or epoch limits and memory caps |
| Web-client UI extensions | Customise the home screen or player | No | Community plugins inject scripts into the web client (Jellyfin-Enhanced has 1,869 stars) | (unverified) | — | Demand is real, but script injection is a security hole; prefer declarative home rows and themes |

### Webhooks, events and notifications

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Outgoing webhooks | Trigger automations when things happen | Yes, but needs an active Plex Pass on the owner account | Official Webhook plugin, free | Part of Notifications in recent versions; Emby's help article says Premiere is required (whether 4.8 and later still gate it is unverified) | Jellyfin, because it is free and flexible | A lifetime Plex Pass has cost $749.99 since 1 July 2026 |
| Event coverage | Choice of triggers | Play, pause, resume, stop, scrobble (past 90%), rate, new library item, new On Deck item, database backup, database corruption, new device, playback by a shared user | 23 notifier types: playback start, progress and stop; item added and deleted; sign-in success and failure; user created, updated, deleted and locked out; plugin installs and updates; task completed; subtitle download failure; restart pending; user data saved | Playback, new media, user sign-in, scan completed, server start and stop, uploads (partly secondary source) | Jellyfin for breadth; Plex's database-corruption event is a nice touch | — |
| Built-in destinations | Messages arrive where you already look | Generic HTTP only | Discord, Gotify, Pushbullet, Pushover, Slack, SMTP, MQTT, generic JSON and generic form | Generic HTTP (others unverified) | Apprise, a library covering most notification services through one URL format | — |
| Payload templating | Shape each message for its destination | Fixed JSON | Handlebars templates with helpers | Fixed JSON (unverified) | Jellyfin | Tautulli's evaluated notification text was a remote-code-execution bug in 2026; templates must be logic-less |
| Test button and sample payload | Check a webhook works before relying on it | (unverified) | Missing; issues 29 (48 reactions) and 210 (28) ask for it | "View sample payload" button added in the 4.8 cycle | Emby | Cheap to build, clearly wanted |
| Filters by event, user and library | Only the events you care about | Per-account webhooks; owner also gets server-wide events | Per-webhook item-type and user filters (unverified) | Per-event URLs plus user and library multi-select | Emby | — |
| Grouping and debounce | "Season 3 added" instead of ten episode messages | No (unverified) | Requested in issues 31 (7 reactions) and 329 (7) | Delays events until metadata is complete | — | Gunmetal knows when a scan batch ends, so it can group |
| Complete payloads | IDs and metadata are filled in when the event fires | (unverified) | Users ask for more template variables (issue 147) | Early events lacked provider IDs until 4.8 | — | Fire after metadata, not after file discovery |
| Authenticity (signing) | The receiver can prove the server sent it | Not documented (unverified) | Custom headers only | Users asked for custom headers to carry API keys | Standard Webhooks: HMAC-SHA256 with id, timestamp and signature headers, plus replay protection | None of the three sign payloads, as far as could be checked |
| Retries and delivery log | Missed events are not lost | (unverified) | (unverified) | (unverified) | Standard Webhooks guidance | WatchState recommends scheduled polling as a backstop because webhooks get missed |
| Artwork in payload | Rich Discord or chat embeds | Thumbnail included (unverified) | Request to add poster images (1 vote) | (unverified) | — | Use signed, short-lived image URLs, never a token |
| Played and unplayed events | Sync tools react to manual "mark watched" | (unverified) | Requested (issue 118, 8 reactions) | (unverified) | — | Comes free from Gunmetal's watch log |
| Watchlist events | Request tools react to watchlist changes | Requested on the forum (9 votes) | n/a | n/a | — | — |
| Notifications to viewers | "A new episode of your show is here" | (unverified) | Subscribe-to-show request 50 votes; mobile push 48 votes | Push to Emby apps (secondary source) | Seerr tells requesters when their title is available | Push on iOS needs Apple's service, which sits awkwardly with "no central account" |
| Admin notifications | Know about updates, restarts and failures | Database backup and corruption events; Home Assistant shows server updates | Update notification 51 votes (planned); restart-required 19 | (unverified) | Plex | — |
| MQTT | Home automation without glue code | No | Through the Webhook plugin | (unverified) | Jellyfin | — |

### Request management

None of the three servers has requests built in, so the best-in-class column is
about request managers. Seerr is the merger of Overseerr and Jellyseerr announced
in February 2026; Overseerr's repository is archived and Seerr has 12,770 stars.

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Built-in requests | Ask for a title inside the server's own app | No | No; a built-in request feature was declined (24 votes) | No | Seerr, with Ombi (4,105 stars) as the older alternative | Gunmetal should integrate, not rebuild |
| Server sign-in and user import | Same accounts in the request tool | Seerr supports it | Seerr supports it | Seerr supports it | Seerr | With passkey-only accounts, Seerr's password sign-in will not work as-is |
| Library availability sync | The request tool knows what you already have | Seerr scans the server | Seerr scans the server | Seerr scans the server | Seerr | Needs external IDs (TMDB, TVDB, IMDb) on every item |
| Per-season and per-movie requests | Ask for exactly what you want | Via Seerr | Via Seerr | Via Seerr | Seerr | Single-episode requests: issue 264, 72 reactions |
| Separate 4K requests | 4K goes to a separate library or instance | Via Seerr | Via Seerr | Via Seerr | Seerr has dedicated 4K request and auto-approve permissions | — |
| Auto-approval | Trusted users skip the queue | Via Seerr | Via Seerr | Via Seerr | Seerr, with separate movie, TV and 4K flags | — |
| Quotas | Limit requests per user per period | Via Seerr | Via Seerr | Via Seerr | Seerr: movie and TV limits over a number of days | — |
| Watchlist auto-request | Adding to a watchlist files a request | Plex watchlist feeds Seerr's auto-request | Jellyfin has no watchlist yet (1,294 votes, planned) | No | Seerr with Plex | Gunmetal's own watchlist could feed Seerr the same way |
| Issue reporting | Users flag broken audio or bad subtitles | Via Seerr | Via Seerr | Via Seerr | Seerr | Could also live in Gunmetal's player |
| Requester notifications | Told when the title arrives | Via Seerr | Via Seerr | Via Seerr | Seerr: email, web push, Discord, webhook, Gotify, ntfy, Pushbullet, Pushover, Slack, Telegram | — |
| Music requests | Request albums through Lidarr | Not in Seerr | Not in Seerr | Not in Seerr | Nobody mainstream; Seerr's music PR was closed in November 2025 in favour of a new one; forks such as SeerrNG exist | Issue 96: 134 reactions, 78 comments |
| Removal requests | Ask for something to be deleted | No | No | No | Nobody; Seerr issue 308 has 93 reactions | Maintainerr fills part of this |
| OIDC sign-in | One login across tools | No | No | No | Nobody; Seerr issue 183 has 277 reactions | — |
| Blocklist | Hide titles from discovery | Via Seerr | Via Seerr | Via Seerr | Seerr | — |

### Download automation (the *arr stack)

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Refresh on import | New downloads appear within seconds | Sonarr and Lidarr have a Plex connector (Radarr too, unverified) | Sonarr's "MediaBrowser" connector (Radarr's unverified) | Same connector | Plex and Jellyfin, roughly equal | Lidarr also has a Subsonic connector, so an OpenSubsonic adapter gets Lidarr refreshes for free |
| Path-scoped refresh | Only the changed folder is rescanned | Partial scans (detail unverified) | Updated-media endpoint takes paths | Same | Jellyfin's endpoint is simple and explicit | — |
| Connector stability | Upgrading the server does not silently break imports | — | 12.0's auth change broke the connector until a Sonarr fix merged on 2026-07-27 (v5 branch, backport promised) | Accepts both header styles | — | The old connector's "Test" button passed even with a fake key (Sonarr issue 8805) |
| Filesystem watching | Changes are picked up without any helper | Automatic scanning option (unverified) | Real-time monitoring option (unverified) | (unverified) | — | Watching is unreliable on SMB and NFS mounts, which is why people rely on connectors |
| Quality upgrades | A better copy does not reset progress or show as new | (unverified) | "Recently Added shows upgrades" 62 votes; "keep watched state on new version" 5 votes | (unverified) | — | Gunmetal can match on external IDs and keep the watch log entry |
| External IDs on every item | *arr apps, Seerr and Maintainerr can match items | Exposed as GUIDs | Exposed as provider IDs | Exposed as provider IDs | All three | Maintainerr cross-checks matches by release year |
| Delete propagation | Deleting in one place cleans up the others | No | No | No | Maintainerr and Janitorr remove files, unmonitor in *arr and clear Seerr requests | — |
| Missing-episode awareness | See which episodes are missing | (unverified) | "Display missing episodes" 343 votes (started); pulling Sonarr's schedule was declined | (unverified) | Sonarr's own calendar | — |
| Music metadata dependency | Music downloads import reliably | — | — | — | — | Lidarr reads from its own lidarr.audio cache; issue 5498 (37 comments, 15 reactions) records outages, and issue 5733 shows albums that exist on MusicBrainz but cannot be imported |
| Book automation | Books arrive like films do | — | — | — | — | Readarr was retired in June 2025; out of Gunmetal's first-version scope anyway |
| Quality-profile tooling | Sensible download profiles | — | — | — | Recyclarr (2,128 stars) syncs community profiles into Sonarr and Radarr | Not the server's job; listed so the boundary is clear |

### Subtitle automation (Bazarr)

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Refresh after a subtitle download | New subtitles show up immediately | Supported by Bazarr | Bazarr's Jellyfin support merged in April 2026; a fallback-refresh bug was fixed in September 2026 | Not supported by Bazarr | Plex, by maturity | Bazarr refreshes Jellyfin items by IMDb ID |
| Search when you press play | Missing subtitles are fetched on demand | Bazarr listens for Plex play and resume webhooks; needs Plex Pass | Not supported | Not supported | Plex plus Bazarr | Bazarr's webhook URL carries its API key in the query string |
| Pick up new subtitles mid-playback | No need to stop and restart | No; Bazarr's docs say to stop and resume | (unverified) | (unverified) | Nobody | Gunmetal owns the player, so it can offer the new track live |
| In-player subtitle search | Search from the playback screen | Built in (unverified) | Declined (2 votes); OpenSubtitles plugin exists | (unverified) | Plex (unverified) | — |

### Watch-state tracking and sync services

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Trakt scrobbling | Plays logged on Trakt automatically | Trakt now lets a free account sync with one Plex server (2026, per a user's screenshot); PlexTraktSync has 2,151 stars | Official Trakt plugin (299 stars) | Community plugin (secondary source) | Plex, because Trakt integrates with it directly | Trakt began requiring VIP to create API apps in August 2026 |
| Two-way Trakt sync | Import history and progress from Trakt | PlexTraktSync | Plugin import capped at the latest 100 items after Trakt API changes (issue 299); duplicate history bug (issue 226, 18 comments) | (unverified) | — | — |
| Several household members on Trakt | Each person scrobbles to their own account | Per-user in PlexTraktSync (unverified) | Multi-user handling issue 221 | (unverified) | — | Needs per-user plugin settings |
| Simkl | A cheaper or free alternative tracker | (unverified) | Simkl plugin exists (request marked complete) | (unverified) | (unverified) | PlexTraktSync users asked for Simkl support (issue 2501) |
| Letterboxd | Films logged to a diary | No official integration (unverified) | Request 75 votes; Letterboxd collections 24 | (unverified) | Kometa reads Letterboxd lists for collections | Letterboxd's API is not openly available (unverified) |
| Anime trackers (AniList, MyAnimeList, Kitsu) | Anime progress kept in sync | Third-party tools (unverified) | Request 14 votes | (unverified) | MALSync (2,993 stars) for web players | Kometa also has a MyAnimeList connector |
| Cross-server watch-state sync | Same progress on two servers, or during a migration | Account-level sync for matched items (unverified) | None | None | WatchState (1,571 stars) syncs play state and resume points between Plex, Jellyfin and Emby without third-party services; JellyPlex-Watched (1,049) and CrossWatch (924) do similar jobs | — |
| Self-hosted tracker | Keep a Trakt-like history without Trakt | — | — | — | Yamtrack (3,665 stars) and Ryot (3,616) | Popularity tracks the Trakt price rises |
| Watch-history view | "What did I watch, and when?" | (unverified) | "Watched History" 830 votes (planned) | (unverified) | Trakt | Gunmetal's watch log is this data |
| Watch-history export | Take your history with you | (unverified) | "Export settings and watched status" 17 votes | (unverified) | WatchState backs up play state in portable formats | Record 1 already promises an exportable log |
| Resume-position sync | Continue on another server where you stopped | (unverified) | No | No | WatchState | — |
| Ratings sync | Ratings follow you to trackers | PlexTraktSync | (unverified) | (unverified) | CrossWatch syncs ratings, watchlists, history and progress across many trackers | — |
| Rewatch handling | Rewatching does not corrupt counts or progress | (unverified) | "Re-watch mode" 14 votes | (unverified) | — | An append-only log records rewatches naturally |

### Music scrobbling and listening services

For music the relevant rivals are Plex (through Plexamp), Jellyfin and Navidrome, so
those are the three product columns in this table.

| Feature | What the user gets | Plex (Plexamp) | Jellyfin | Navidrome | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Last.fm scrobbling | Plays counted on Last.fm | Built in (confirmed by a 2025 forum post) | Third-party plugin; the original is archived (last pushed February 2026) and a fork is active; it cannot ship with Jellyfin for licence reasons | Built in, authorised per user | Navidrome | Request on the Jellyfin board: 27 votes |
| ListenBrainz scrobbling | Plays counted on the open MusicBrainz service | Not supported; requested since 2018 (23 votes); a bridge called Eavesdrop.FM uses Plex Pass webhooks | Third-party plugin (219 stars) | Built in, per user | Navidrome | Request on the Jellyfin board: 28 votes |
| Now-playing updates | Friends see what is playing now | (unverified) | ListenBrainz plugin sends them | Scrobbler capability includes now playing | Navidrome and the Jellyfin plugin | — |
| Loved-track sync | Hearts match between server and service | Requested on the forum, closed as a duplicate | ListenBrainz plugin syncs both ways when MusicBrainz IDs are present | (unverified) | Jellyfin ListenBrainz plugin | — |
| Import play counts from a service | Years of Last.fm history visible in your player | Requested on the forum | No | (unverified) | Nobody | Gunmetal could import into the watch log, marked as imported |
| Offline scrobble queue | Plays made offline or during an outage still count | (unverified) | ListenBrainz plugin caches listens while the service is down and can back them up to files | (unverified) | Jellyfin ListenBrainz plugin | Last.fm's rules say cached scrobbles must be sent in order, oldest first |
| Per-user credentials | Each person links their own account | Per Plex account (unverified) | Admin configures every user | Per user | Navidrome | — |
| Spec-compliant scrobble threshold | Skipped tracks do not count | (unverified) | (unverified) | (unverified) | Both services set the rule: tracks over 30 seconds, counted after half the track or four minutes, whichever comes first | Gunmetal's client knows exact play time, including offline |
| MusicBrainz IDs in submissions | Scrobbles link to the right recording | Plex matches music to MusicBrainz (forum claim) | Optional in the ListenBrainz plugin | (unverified) | — | Gunmetal reads MBIDs from tags at scan time |
| Service playlists into the library | ListenBrainz daily and weekly mixes appear as playlists | Requested (7 votes) | ListenBrainz plugin syncs generated playlists | Plugin with 283 stars | Navidrome plugin | — |
| Scrobble hub | One place that relays to many services | — | — | — | multi-scrobbler (1,248 stars) relays from many sources to many destinations | — |
| Listening year-in-review | A "Wrapped" for your own library | (unverified) | jellyfin-rewind (376 stars) | Spindle stats plugin (81 stars) | — | Strong demand on every server |
| Scrobbles from third-party apps | Plays from Symfonium or Finamp count too | — | Playback reporting from Jellyfin clients | Subsonic `scrobble` endpoint | Navidrome | Gunmetal's adapters must feed the same watch log |

### Statistics, monitoring and dashboards

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Live sessions | Who is playing what, right now | Built-in dashboard (detail unverified) | Built-in dashboard | Built in | Tautulli and Tracearr show more detail | — |
| Per-user playback history | What each person watched | Basic history (unverified) | Official Playback Reporting plugin | Plugin (unverified) | Tautulli | — |
| Usage graphs and library stats | Trends over time | Tautulli (6,598 stars) | Jellystat (2,485 stars, now mid-rebuild), Streamystats (818), finstats; built-in stats request has 36 votes | Jellystat has an Emby mode; Tracearr | Tautulli for Plex; Tracearr across all three | Users want this built in |
| Direct play, remux and transcode breakdown | See how hard the server is working | Tautulli | Streamystats and Tracearr | Tracearr | Tracearr | This is Gunmetal's core claim, so it should be front and centre |
| Bandwidth and codec stats | Plan hardware and upload capacity | Tautulli | Tracearr | Tracearr | Tracearr | — |
| Account-sharing detection | Spot a shared password being abused | Tracearr | Tracearr | Tracearr | Tracearr (2,680 stars): impossible-travel alerts, concurrent-stream limits, trust scores | Gunmetal's device-bound keys make device identity strong |
| Stop a stream | Kick a misbehaving session | Via Tautulli (detail unverified) | Request 402 votes, open | (unverified) | Tracearr automations | — |
| Recently-added newsletters | Weekly email of new arrivals | Tautulli | No | No | Tautulli | Tautulli fixed a remote-code-execution bug in newsletter templates in May 2026 |
| Year in review | A shareable summary of the year | Wrapperr (456 stars), plex-rewind (304) | jellyfin-wrapped (85) | No | — | Should be built in |
| Server resource graphs | CPU, RAM and network over time | (unverified) | Request 41 votes | (unverified) | — | Prometheus covers admins who want it |
| Prometheus metrics | Plug into existing monitoring | No (unverified) | Exporter merged in 2020 | (unverified) | Jellyfin | — |
| Push-based collection | Stats tools do not hammer the server | Server event stream | Jellystat polled every second (issue 298); thread-pool starvation when both ran (issue 328, 34 comments) | Plugin needed | Tracearr uses Plex's event stream | — |
| Natural-language queries | Ask "what did my kids watch last week?" | No | Streamystats AI chat | No | Streamystats | Niche |
| Tool security | The stats tool does not become the way in | Tautulli fixed more than a dozen CVE-numbered or pending issues in 2026 | Jellystat lists security testing as unfinished | — | — | See pain points |

### Metadata managers, collections and library maintenance

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Collections from external lists | Collections built from IMDb, Trakt, MDBList, Letterboxd, TMDB and anime lists | Kometa (3,442 stars) | Kometa support in progress on a branch (issue 3351, July 2026); smaller tools such as Jellyfin Collection | Kometa (unverified) | Kometa | Kometa's config also lists Simkl, Yamtrack and other trackers as sources |
| Poster overlays | Badges for 4K, HDR, audio format or ratings | Kometa | None built in; a Kometa integration request was declined | (unverified) | Kometa | Overlays are baked into poster files; Gunmetal can draw them in the client instead |
| Poster and title-card generation | Consistent artwork | Posterizarr (931 stars) | Posterizarr | Posterizarr | Posterizarr | — |
| Smart collections | Collections defined by rules | Smart collections (unverified) | Request 104 votes | (unverified) | Plex (unverified) | — |
| Nested and cross-library collections | Franchises across films and shows | (unverified) | Nested 44 votes; cross-library 44; multiple sets 46 | (unverified) | — | — |
| Customisations that survive database loss | Rebuilding the server keeps your edits | No | NFO sidecar files | NFO (unverified) | Kometa, which keeps customisations outside the server database | Fits Gunmetal's "SQLite is a cache" rule |
| External ratings | IMDb, Rotten Tomatoes, Letterboxd scores | Some sources shown (unverified) | Request 196 votes; a community ratings plugin has 126 stars | (unverified) | — | Fetching ratings needs network grants |
| "Leaving soon" cleanup | Unwatched titles warned, then removed | Maintainerr (2,315 stars) | Maintainerr; Janitorr (757) | Maintainerr | Maintainerr: rule builder across server, *arr apps, Seerr and stats tools | Needs per-item last-played and play counts in the API |
| Bulk metadata editing | Fix many items at once | (unverified) | Request 186 votes | (unverified) | — | — |
| Unmatched-item filter | Find items metadata lookup missed | (unverified) | Request 101 votes | (unverified) | — | — |
| Pluggable metadata providers | Swap or add sources | Agents (detail unverified) | Metadata plugins (in-process) | Plugins | Navidrome metadata agents, sandboxed | — |

### Smart home and voice

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Home Assistant integration | Media shows up in home automations | Core integration: a media player per client, active-stream and library sensors, update entity, library refresh | Core integration: a media player per session, remote entities, media source | Integration exists (scope unverified) | Plex | Jellyfin's request for it gathered 355 votes and is now marked complete |
| Play by search | "Play the latest episode of X in the lounge" | JSON search payload with filters by genre, actor, collection and unwatched state | (unverified) | (unverified) | Plex | — |
| Media browser in HA | Browse the library from HA dashboards | Yes | Music, movies and TV only | (unverified) | Plex | — |
| Music Assistant source | Whole-home audio from your library | Yes | Yes | Yes | Music Assistant (3,129 stars), which also takes Subsonic servers | Free via OpenSubsonic |
| Sonos playback | Send music to Sonos | Via HA's Plex and Sonos link | Via Music Assistant | Via Music Assistant | Plex | — |
| Voice assistants | "Alexa, play..." | Alexa skill (unverified) | Requests: 243 and 50 votes, both open | (unverified) | Plex (unverified) | Voice platforms need a cloud endpoint, which conflicts with "no central account" |
| MQTT events | Events onto an MQTT broker | No | Webhook plugin | (unverified) | Jellyfin | — |
| Control playback devices | Pause the TV from HA | Clients controllable (unverified) | Remote entities | (unverified) | — | Gunmetal's own clients can expose control through the server |

### Social presence (Discord)

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Native rich presence | Friends see what you are playing | No; forum request with 100 votes since 2018; separate Plexamp request | No; request 5 votes | No | Nobody among servers | — |
| Desktop bridge app | Presence via a companion app | discord-rich-presence-plex (437 stars) needs Discord's desktop app on the same machine | Jellyfin-RPC (archived, last pushed September 2026); MBCord (100 stars) | MBCord | — | Bridges need a token for the server and another app running all day |
| Server-side presence | Presence without a desktop app | — | — | — | Navidrome plugin (175 stars) works server-side but stores the user's Discord token, which its own docs say may break Discord's terms | Not acceptable for Gunmetal |
| Album art in presence | Cover shown in Discord | Bridge uploads covers to public image hosts | — | — | Navidrome plugin uses Cover Art Archive URLs | Public URLs only; never expose the server |
| Channel announcements | New arrivals posted to a Discord channel | Webhooks (Plex Pass) or Tautulli | Webhook plugin's Discord destination | Webhooks (Premiere) | Jellyfin | — |

### User onboarding and identity integrations

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Invitations | Friends join with a link | Invites through plex.tv accounts | Sign-up page request 227 votes | (unverified) | Wizarr (3,207 stars) works with all three | Gunmetal can issue invite links natively |
| OIDC single sign-on | Use Authentik, Authelia, Keycloak or Google | Plex account only | 1,191 votes (planned); the main SSO plugin is archived (last pushed May 2026) | (unverified) | — | Native in Gunmetal per record 1 |
| LDAP | Directory-backed accounts | No | Official LDAP plugin | (unverified) | Jellyfin | — |
| Device sign-in codes | Sign in a TV without typing a password | Link codes (unverified) | Quick Connect; QR code request 225 votes | (unverified) | — | Navidrome's Jellyfin adapter also implements Quick Connect |
| User groups | Manage family and friends as groups | (unverified) | Request 202 votes | (unverified) | — | Useful for scoping integrations too |

### Data portability and migration

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Settings export and import | Move or restore configuration | (unverified) | Completed (899 votes) | (unverified) | Jellyfin | — |
| Watch-state migration | Switch servers without losing progress | — | — | — | WatchState pulls from Plex into Jellyfin or Emby | Gunmetal needs Plex and Jellyfin importers on day one |
| Playlist import | Bring Spotify or M3U playlists | (unverified) | Spotify import 29 votes; import from Plex, Trakt or IMDb 31 | (unverified) | — | M3U is on Gunmetal's roadmap |
| Playlist export | Take playlists elsewhere | (unverified) | 23 votes | (unverified) | — | — |
| Backup and restore | Recover from disk loss | Scheduled database backup with webhook event | Backup and restore 142 votes (started); scheduled backups 91 (planned) | (unverified) | Plex | Gunmetal only needs to back up the watch log and config |
| Open history format | History readable by other tools | No | No | No | ListenBrainz's documented listen JSON is the nearest thing | Gunmetal's log format should be documented and versioned |

## Pain points and unmet demand

1. **Trakt has become expensive and restrictive.** VIP renewals moved to a $60
   standard rate in 2025. Trakt's 2026 limits (posted 27 February 2026) cap free
   accounts at 250 watchlist items and five personal lists. In early August 2026,
   creating API apps started to require VIP, and existing apps disappeared for
   some developers. PlexTraktSync issue 2548 (48 comments, 26 reactions) asks for
   another way in; the Jellyfin Trakt plugin's imports were cut to the newest 100
   items by API changes (issue 299). Interest in self-hosted trackers (Yamtrack
   3,665 stars, Ryot 3,616) and sync tools that avoid Trakt (WatchState,
   CrossWatch) follows from this.
   Evidence: https://forums.trakt.tv/t/updating-trakt-limits-for-2026/101592,
   https://selfh.st/weekly/2026-08-07/,
   https://github.com/Taxel/PlexTraktSync/issues/2548,
   https://github.com/jellyfin/jellyfin-plugin-trakt/issues/299,
   https://forums.trakt.tv/t/upcoming-vip-renewal-pricing-changes-effective-june-12-2025/56649.

2. **Integration features sit behind paywalls.** Plex webhooks need Plex Pass, and
   Bazarr's search-on-play depends on them. Plex Pass rose in April 2025, remote
   streaming became paid, the Remote Watch Pass went up 50% from June 2026, and
   a lifetime Plex Pass rose to $749.99 on 1 July 2026. Emby's help article says
   its webhooks need Premiere. Jellyfin's are free.
   Evidence: https://support.plex.tv/articles/115002267687-webhooks/,
   https://wiki.bazarr.media/Additional-Configuration/Webhooks/,
   https://9to5mac.com/2025/03/19/plex-price-increase-remote-streaming-changes/,
   https://www.androidauthority.com/plex-remote-watch-pass-price-increase-3663060/,
   https://www.plex.tv/blog/new-lifetime-plex-pass-pricing/,
   https://emby.freshdesk.com/support/solutions/articles/44001848859-webhooks.

3. **Third-party tools hold admin keys and keep getting breached.** In February
   2026 Huntarr was found to let anyone on the network call any endpoint without
   credentials, leaking its own passwords and the API keys of every connected
   *arr app; a public review lists 21 findings, and the maintainer pulled the
   repository. Tautulli shipped more than a dozen security fixes in 2026, including
   remote code execution through notification text (CVE-2026-28505) and newsletter
   templates (CVE-2026-41065), SQL injection (CVE-2026-31799), unauthenticated path
   traversal (CVE-2026-31831) and Plex token leakage. Bazarr's Plex webhook puts
   its API key in the URL. The common cause is that servers only offer
   all-powerful keys, so every helper becomes a master key.
   Evidence: https://piunikaweb.com/2026/02/24/huntarr-security-vulnerability-arr-api-keys-exposed/,
   https://github.com/rfsbraz/huntarr-security-review,
   https://github.com/Tautulli/Tautulli/blob/master/CHANGELOG.md.

4. **Jellyfin plugins are unsandboxed and break on upgrade.** Plugins are .NET DLLs
   loaded into the server process and can register their own web endpoints. 12.0
   required removing every third-party plugin before upgrading, and 10.11.9 broke
   repository lookups (issue 16905). 12.0 also fixed path traversal through plugin
   endpoints and stopped the installer accepting unsafe package names.
   Evidence: https://github.com/jellyfin/jellyfin/releases/tag/v12.0,
   https://github.com/jellyfin/jellyfin/issues/16905,
   https://github.com/jellyfin/jellyfin-plugin-template.

5. **API churn breaks the ecosystem.** Jellyfin 12.0 disabled legacy auth headers
   (deprecated since 10.11, announced to developers for about two years) and
   removed the `/emby` routes. Sonarr's Jellyfin connector needed a fix (issue
   8805, PR 8816, merged 2026-07-27), and the old connector's test button passed
   even with a fake key. Plex's move to 7-day JWTs left developers asking for
   working examples, and Tautulli added a "token expired" notification.
   Evidence: https://github.com/jellyfin/jellyfin/pull/13306,
   https://github.com/jellyfin/jellyfin/pull/15559,
   https://github.com/Sonarr/Sonarr/issues/8805,
   https://github.com/Sonarr/Sonarr/pull/8816,
   https://forums.plex.tv/t/jwt-authentication/931646.

6. **Key integrations depend on one tired volunteer.** The Jellyfin SSO plugin
   (1,456 stars) is archived, last pushed in May 2026, with a notice saying its
   author was tired of working on it, while OIDC is the board's fourth-most-wanted
   feature (1,191 votes). Jellyfin-RPC is archived (last pushed September 2026),
   as is the original Jellyfin Last.fm plugin (last pushed February 2026) and
   jellysub, Subsonic for Jellyfin (last pushed 2022), even though its 77-vote
   request still says "started". Jellystat's README says it is being rebuilt and
   updates are paused.
   Evidence: https://github.com/9p4/jellyfin-plugin-sso,
   https://features.jellyfin.org/posts/230/support-for-oidc-oauth-sso,
   https://github.com/justradical/jellyfin-rpc,
   https://github.com/jesseward/jellyfin-plugin-lastfm,
   https://features.jellyfin.org/posts/19/subsonic-client-support,
   https://github.com/CyferShepard/Jellystat.

7. **Integrations are configured by the admin for everyone.** The Jellyfin
   ListenBrainz plugin's README says the admin has to enter each user's settings
   because Jellyfin has no per-user plugin settings; two requests ask for this (4
   votes each). The admin ends up holding everyone's tokens.
   Evidence: https://github.com/lyarenei/jellyfin-plugin-listenbrainz,
   https://features.jellyfin.org/posts/4180/let-plugins-show-a-settings-page-to-non-admin-users,
   https://features.jellyfin.org/posts/2166/allow-plugins-to-expose-a-config-page-to-non-admin-users.

8. **Polling helpers overload the server.** Jellystat queried every second (issue
   298, 9 reactions); running it with Jellyfin caused thread-pool starvation (issue
   328, 34 comments, 10 reactions). WatchState recommends scheduled polling as a
   backstop because webhooks get missed.
   Evidence: https://github.com/CyferShepard/Jellystat/issues/298,
   https://github.com/CyferShepard/Jellystat/issues/328,
   https://github.com/arabcoders/watchstate/blob/master/FAQ.md.

9. **Webhooks are hard to trust and debug.** The two most-reacted issues on the
   Jellyfin Webhook plugin ask for a test button (48 and 28 reactions). Others ask
   for grouped notifications, played and unplayed events and more template
   variables. Emby users reported events firing before metadata was ready. No
   server signs its payloads, as far as could be checked.
   Evidence: https://github.com/jellyfin/jellyfin-plugin-webhook/issues/29,
   https://github.com/jellyfin/jellyfin-plugin-webhook/issues/210,
   https://github.com/jellyfin/jellyfin-plugin-webhook/issues/329,
   https://github.com/jellyfin/jellyfin-plugin-webhook/issues/118,
   https://emby.media/community/topic/113134-giving-webhooks-some-attention/.

10. **Request managers still lack music, OIDC and removals.** Seerr's top issues are
    OIDC sign-in (277 reactions, 89 comments), music support (134 reactions, 78
    comments), removal requests (93), single-episode requests (72) and Tracearr
    support (50). Jellyfin declined to build requests in (24 votes).
    Evidence: https://github.com/seerr-team/seerr/issues/183,
    https://github.com/seerr-team/seerr/issues/96,
    https://github.com/seerr-team/seerr/issues/308,
    https://github.com/seerr-team/seerr/issues/264,
    https://github.com/seerr-team/seerr/issues/2449,
    https://features.jellyfin.org/posts/604/request-shows-movies-music-from-sonarr-radarr-lidarr.

11. **Kometa, the collections tool, only fully supports Plex.** Its Jellyfin and
    Emby request has 33 reactions (issue 321), and generalised server support is
    still a worklist on a branch (issue 3351, July 2026). Meanwhile Jellyfin users
    ask for smart collections (104 votes), nested collections (44), cross-library
    collections (44), multiple collection sets (46) and external ratings (196).
    Evidence: https://github.com/Kometa-Team/Kometa/issues/321,
    https://github.com/Kometa-Team/Kometa/issues/3351,
    https://features.jellyfin.org/posts/593/dynamic-smart-collections-defined-by-custom-queries,
    https://features.jellyfin.org/posts/4039/support-nested-collections-collections-within-collections,
    https://features.jellyfin.org/posts/463/imbd-rating-and-rotten-tomatoes-audiance-rating-and-fresh-rating-on-movies-and-tv-shows.

12. **Music listening services are second-class on video servers.** Plex has had a
    ListenBrainz request open since 2018 (23 votes) and users want Last.fm loved
    tracks and play counts brought in. Jellyfin relies on third-party plugins (27
    and 28 votes for Last.fm and ListenBrainz). Navidrome does both natively and per
    user.
    Evidence: https://forums.plex.tv/t/feature-request-support-listenbrainz-replacement-for-last-fm/226426,
    https://forums.plex.tv/t/plex-music-and-last-fm-integration/902672,
    https://features.jellyfin.org/posts/51/last-fm-scrobbling,
    https://features.jellyfin.org/posts/2411/integration-with-listenbrainz,
    https://www.navidrome.org/docs/usage/integration/external-services/.

13. **Discord presence needs a fragile helper.** The Plex forum thread has 100 votes
    since 2018. Desktop bridges need Discord running on the same machine and
    upload covers to public image hosts. The server-side alternative stores users'
    Discord tokens, which its own docs flag as a possible terms violation.
    Evidence: https://forums.plex.tv/t/plex-discord-integration/287994,
    https://github.com/phin05/discord-rich-presence-plex,
    https://github.com/navidrome/discord-rich-presence-plugin.

14. **Lidarr's music metadata is fragile.** Lidarr depends on its own lidarr.audio
    cache; outages return server errors (issue 5498, 37 comments, 15 reactions),
    and albums present on MusicBrainz can be unimportable until the cache catches
    up (issue 5733).
    Evidence: https://github.com/Lidarr/Lidarr/issues/5498,
    https://github.com/Lidarr/Lidarr/issues/5733.

15. **Admins want stats and control without extra apps.** Jellyfin requests include
    killing a user's stream (402 votes), usage statistics (36), resource graphs (41)
    and update notifications (51). Year-in-review tools exist for every server
    (Wrapperr 456 stars, plex-rewind 304, jellyfin-rewind 376), showing demand for
    a built-in version.
    Evidence: https://features.jellyfin.org/posts/301/option-to-kill-a-stream-from-a-user,
    https://features.jellyfin.org/posts/65/feature-request-server-user-usage-statistics,
    https://features.jellyfin.org/posts/1376/statistics-graph-of-cpu-ram-and-network-information,
    https://github.com/aunefyren/wrapperr, https://github.com/RaunoT/plex-rewind,
    https://github.com/Chaphasilor/jellyfin-rewind.

16. **Watch history is not portable.** Jellyfin's "Watched History" has 830 votes,
    "Export settings and watched status" 17, Spotify playlist import 29 and
    Letterboxd 75. WatchState, JellyPlex-Watched and CrossWatch exist to move or
    mirror history between servers.
    Evidence: https://features.jellyfin.org/posts/633/watched-history,
    https://features.jellyfin.org/posts/1754/export-settings-and-watched-status,
    https://features.jellyfin.org/posts/684/letterboxd-integration,
    https://github.com/arabcoders/watchstate.

17. **Voice and smart-home control is uneven.** Home Assistant support for Jellyfin
    gathered 355 votes and is now marked complete; Alexa and Google Assistant
    requests (243 and 50 votes) are still open.
    Evidence: https://features.jellyfin.org/posts/27/home-assistant-integration,
    https://features.jellyfin.org/posts/275/support-google-assistant-and-alexa,
    https://features.jellyfin.org/posts/64/feature-request-amazon-alexa-support.

18. **Quality upgrades look like new arrivals.** "Recently Added" showing upgraded
    files has 62 votes; keeping watched state when Sonarr or Radarr replaces a file
    has 5.
    Evidence: https://features.jellyfin.org/posts/2329/improve-recently-added-to-not-show-upgraded-items,
    https://features.jellyfin.org/posts/1588/keep-seen-status-when-sonarr-radarr-download-a-new-video-version.

## Where Gunmetal can be clearly better

Each idea names the pain point it answers and what has to be true for it to work.

1. **Scoped tokens for every integration (pain points 3, 5, 7).** Instead of one
   admin key, each integration gets a token limited to what it needs: "refresh
   these library paths" for Sonarr, "read playback history" for a stats tool,
   "read library and availability" for Seerr. Tokens are revocable, listed in the
   UI with last-used time, sent in headers only, and can expire. Technically, the
   native API's per-object authorisation (record 1, decision 6) needs a capability
   model with named scopes, and the Jellyfin and OpenSubsonic adapters must map
   their own key concepts onto those scopes rather than onto admin rights. When a
   compromised tool such as Huntarr leaks a key, the damage stays within that
   key's scope.

2. **Plugins in a WebAssembly sandbox with declared grants (pain points 4, 6, 7).**
   Follow Navidrome's model, which already shows it works: a manifest that declares
   hosts, storage, library access and per-user settings, a host-side HTTP and
   WebSocket function that checks the resolved IP when connecting (not only the
   hostname), and settings forms generated from a schema. Go further than Navidrome
   with a versioned interface defined in WIT so a server upgrade does not break
   plugins, CPU and memory limits, signed first-party plugins, and per-user secret
   storage so the admin never sees a user's Last.fm session. Technically this needs
   a Rust WebAssembly runtime such as wasmtime in the server, a stable host API
   with its own test suite, and a commitment that first-party plugins (Last.fm,
   ListenBrainz, Trakt, Simkl, metadata sources) are maintained by the project, so
   the integrations people depend on are not one volunteer's side project. This is
   what record 2 already asks for.

3. **The watch log drives every tracker (pain points 1, 12, 16).** Record 1 makes
   watch history an append-only, exportable log. Scrobblers and trackers become
   readers of that log with their own cursor, so a service outage, a Trakt API
   change or a phone that was offline never loses a play: the plugin resumes from
   its cursor, in order, as Last.fm's rules require. The same log answers "what did
   I watch", feeds year-in-review, and exports in documented formats. Because the
   log is the source of truth, Gunmetal is a self-hosted tracker in its own right,
   and Trakt or Simkl become optional mirrors rather than the record. Technically,
   each event needs a stable ID for idempotent delivery, client-side buffering of
   offline plays with real timestamps, and an importer for Last.fm, ListenBrainz,
   Trakt, Plex and Jellyfin history that marks imported entries as such.

4. **Webhooks that are free, signed and debuggable (pain points 2, 8, 9).** Ship
   webhooks in the server, not as a paid feature or a plugin. Sign payloads using
   the Standard Webhooks scheme, retry with backoff, keep a delivery log, offer a
   test button and sample payloads, filter by event, user and library, and group
   episode and track additions into one "season added" or "album added" event that
   fires after metadata is complete. Add a server-sent events stream for
   dashboards so nothing has to poll. Use logic-less templates so a notification
   can never execute code. Images in payloads should be signed, short-lived URLs.
   Technically, this needs an outbox table derived from scan and playback events,
   a versioned event schema, and first-party destinations for the common cases
   (generic HTTP, ntfy, Discord, email, MQTT), with Apprise-style breadth left to a
   plugin.

5. **Built-in stats that make Tautulli-style tools optional (pain points 3, 8,
   15).** Live sessions, per-user history, a direct-play, remux and transcode
   breakdown, bandwidth, new-device and concurrent-stream alerts, stopping a
   session, and a music and video year in review, all computed from the watch log
   and session data the server already has. The transcode breakdown is also the
   proof of Gunmetal's main claim, so it belongs on the admin home screen. Expose a
   Prometheus endpoint for people who already run Grafana. Technically, the server
   records the playback decision (direct, remux, transcode, and why) per session,
   and device-bound keys (record 1, decision 7) give reliable device identity for
   sharing alerts.

6. **Compatibility adapters scoped like Navidrome's (pain points 6, 12, 17).**
   Navidrome's experimental Jellyfin music API shows a non-Jellyfin server can
   serve Finamp, Jellify and Feishin from a subset of endpoints with no new data
   model. Gunmetal should do the same for music first, then video, and implement
   OpenSubsonic with `apiKeyAuthentication` only, since passkey accounts have no
   password to hash. That one adapter brings dozens of music apps, Music Assistant
   (and so Home Assistant and Sonos) and Lidarr's Subsonic refresh connector.
   Technically, adapters must translate IDs (Navidrome packs its IDs into
   Jellyfin's GUID format), never expose endpoints that are unauthenticated in
   Jellyfin, document client quirks, and run end-to-end tests against real client
   versions in CI.

7. **A first-class citizen of the *arr stack (pain points 5, 14, 18).** Accept
   library-update calls from Sonarr, Radarr and Lidarr through the adapters
   (Jellyfin's path-based update endpoint, Subsonic's `startScan`), plus a native
   path-scoped refresh endpoint and filesystem watching where it works. Treat a
   quality upgrade as a replacement: keep identity, watch state and position, and
   do not announce it as new. Read music identity from tags and MusicBrainz IDs at
   scan time so Gunmetal never depends on Lidarr's metadata cache. Technically,
   items need a stable identity built from external IDs and file fingerprints that
   survives renames and replacements, and refresh calls need scoped tokens.

8. **Be the best backend for Seerr rather than a rival (pain point 10).** Expose
   external IDs, availability per quality, and a watchlist that Seerr can read for
   auto-requests, and let users open Seerr from a title page in Gunmetal's
   clients. Technically, either Seerr learns Gunmetal's native API or the Jellyfin
   adapter covers what Seerr's Jellyfin integration calls, and sign-in for Seerr
   must work without passwords (for example through OIDC, which Seerr users are
   also asking for).

9. **Collections and overlays as code, drawn by the client (pain point 11).**
   Declarative collections and smart playlists live in a config file that survives
   a database rebuild, which matches the "SQLite is a cache" rule. External list
   sources (Trakt, MDBList, Letterboxd, TMDB) are plugins with network grants.
   Quality and rating badges are drawn by Gunmetal's own clients from technical
   metadata instead of being baked into poster files, so artwork stays clean.
   Technically, this needs a small query language for smart collections and a
   stable on-disk format.

10. **Discord presence from our own clients (pain point 13).** Gunmetal's desktop
    client can talk to the local Discord app directly, with no extra helper and no
    tokens on the server, using cover art from Cover Art Archive when a MusicBrainz
    ID is known and no image otherwise. It should be opt-in, per library, with a
    private-listening switch. Technically, this is a client feature; mobile
    presence may not be possible without a Discord partnership (unverified).

11. **Home Assistant from the server and from the clients (pain point 17).** Each
    Gunmetal client session appears as a controllable media player, the library can
    be browsed and searched from HA, and Music Assistant works through
    OpenSubsonic. MQTT events come from the webhook system. Technically, Gunmetal
    needs either its own HA integration (maintained in HA's repository or as a
    custom component) or full coverage of what HA's Jellyfin integration calls.
    Voice assistants need a cloud endpoint and are better left to Home Assistant's
    own voice features.

12. **Live subtitle pickup (subtitle pain in the Bazarr rows).** When Bazarr writes
    a new sidecar and calls the refresh endpoint, Gunmetal's player can offer the
    new track mid-playback instead of requiring stop and resume. Technically, this
    needs an item-changed event to the active session and a scoped token for
    Bazarr.

13. **A stable, honest API contract (pain point 5).** Generate the OpenAPI spec from
    the Rust protocol types, version it with semantic versioning, keep deprecated
    endpoints for at least one major release, publish a changelog aimed at
    integrators, and run a conformance suite in CI. Never accept credentials in
    query strings on the native API; signed stream URLs cover the cases where a
    header is impossible.

## Risks and hard parts

- **The plugin runtime is a large project.** A WebAssembly host with a stable
  interface, SDKs, permission checks, resource limits, signing and a catalog is
  months of work, and the ecosystem will be empty at launch. The first-party
  plugins have to carry it. Navidrome needed two security fixes to its network
  guard after launch, which shows how easy the details are to get wrong.
- **The Jellyfin API is big, loosely specified and still changing.** 12.0 changed
  what `GetItems` returns for the same query and removed routes and auth methods.
  An adapter has to pick a target version and keep up, and real clients rely on
  quirks the spec does not describe. Copying Jellyfin's behaviour too closely
  risks copying its unauthenticated endpoints.
- **Some compatible clients may not support OpenSubsonic API keys.** Refusing the
  classic salted-token scheme protects passkey accounts but may lock out older
  Subsonic apps. Which clients support the extension today was not verified.
- **Third-party terms and keys.** Trakt now requires VIP to create API apps, so a
  single project-wide client ID may not be possible or may be revoked; users may
  have to bring their own. Last.fm and Trakt client secrets cannot be kept secret
  inside an open-source binary. Discord forbids using user tokens. Letterboxd has no
  open API (unverified). Each service can change its rules, as Trakt did twice in
  two years.
- **Push notifications and voice assistants need vendor clouds.** iOS push goes
  through Apple's service and voice skills need a public endpoint. Both sit
  uneasily with "no central account" and may need an optional relay or be left out.
- **Network grants versus home networks.** Admin-configured webhooks legitimately
  target LAN services such as Home Assistant, while plugin traffic to the LAN should
  be blocked by default. The rules for the two must be different and clearly shown.
- **Built-in features compete with popular tools.** Tautulli, Seerr, Kometa and
  Maintainerr have large communities. Users will run them alongside Gunmetal during
  any migration, so the adapters must serve them well even while Gunmetal builds
  native alternatives.
- **Filesystem watching is unreliable on network shares**, so the *arr connectors
  remain the dependable path and must keep working across releases.
- **Scope creep.** This area alone could fill years. The roadmap already puts the
  adapters late; the order in which integrations arrive needs to be decided
  explicitly.
- **Plugin licensing under AGPL** is not settled. Whether WebAssembly plugins
  loaded by an AGPL server must themselves be AGPL-compatible needs a clear answer
  before a catalog opens.

## Open questions

1. Which Jellyfin endpoints does the adapter cover first, and which client versions
   are certified? Navidrome's music subset (browsing, search, favourites, ratings,
   playlists, lyrics, Instant Mix, playback reporting, Quick Connect) is an obvious
   starting list.
2. Should the Jellyfin adapter cover what Seerr, Jellystat, Tracearr, Maintainerr,
   Bazarr and Home Assistant call, including session and user-listing endpoints,
   and under which scopes?
3. Do we ship a project-wide Trakt client ID, ask each user to bring their own, or
   support Trakt only as an import and export format?
4. Which notification destinations are first-party, and which are plugins?
5. How do users sign in to tools such as Seerr that expect a password, given
   passkey-only accounts: app passwords, OIDC, or device-code flows?
6. Should two Gunmetal servers (for example, family members' servers) be able to
   share watch state or recommendations over iroh, replacing WatchState-style
   tools?
7. Is a remote MCP endpoint for AI agents worth exposing, given scoped read-only
   tokens make it cheap?
8. What licence terms apply to third-party WebAssembly plugins under AGPL?
9. Can Discord presence work on mobile without a partnership? (unverified)
10. Which OpenSubsonic clients support `apiKeyAuthentication` today? (unverified)
11. Should Gunmetal accept Plex-style webhooks from Plex itself during a migration
    period, so users can run both servers and keep history in step?

## Sources

Jellyfin

- https://jellyfin.org/posts
- https://jellyfin.org/posts/jellyfin-release-12.0
- https://jellyfin.org/posts/state-of-the-fin-2026-05-24
- https://github.com/jellyfin/jellyfin/releases/tag/v12.0
- https://github.com/jellyfin/jellyfin/pull/13306
- https://github.com/jellyfin/jellyfin/pull/15559
- https://github.com/jellyfin/jellyfin/pull/2985
- https://github.com/jellyfin/jellyfin/issues/5415
- https://github.com/jellyfin/jellyfin/issues/16905
- https://github.com/jellyfin/jellyfin/issues/13595
- https://jellyfin.org/docs/general/server/plugins/
- https://github.com/jellyfin/jellyfin-plugin-template
- https://github.com/jellyfin/jellyfin-plugin-webhook
- https://github.com/jellyfin/jellyfin-plugin-webhook/issues/29
- https://github.com/jellyfin/jellyfin-plugin-webhook/issues/210
- https://github.com/jellyfin/jellyfin-plugin-webhook/issues/118
- https://github.com/jellyfin/jellyfin-plugin-webhook/issues/31
- https://github.com/jellyfin/jellyfin-plugin-webhook/issues/329
- https://github.com/jellyfin/jellyfin-plugin-webhook/issues/147
- https://github.com/jellyfin/jellyfin-plugin-trakt
- https://github.com/jellyfin/jellyfin-plugin-trakt/issues/299
- https://github.com/jellyfin/jellyfin-plugin-trakt/issues/226
- https://github.com/jellyfin/jellyfin-plugin-trakt/issues/221
- https://github.com/jellyfin/jellyfin-plugin-playbackreporting
- https://github.com/jellyfin/jellyfin-sdk-typescript
- https://github.com/jellyfin/jellyfin-sdk-kotlin
- https://github.com/jellyfin/jellyfin-sdk-swift
- https://github.com/lyarenei/jellyfin-plugin-listenbrainz
- https://github.com/jesseward/jellyfin-plugin-lastfm
- https://github.com/danielfariati/jellyfin-plugin-lastfm
- https://github.com/9p4/jellyfin-plugin-sso
- https://github.com/justradical/jellyfin-rpc
- https://github.com/andrewrabert/jellysub
- https://github.com/n00bcodr/Jellyfin-Enhanced
- https://github.com/4lx69/jellyfin-collection
- https://github.com/Druidblack/jellyfin_ratings
- https://features.jellyfin.org/api/v1/posts?view=most-wanted&limit=100
- https://features.jellyfin.org/posts/19/subsonic-client-support
- https://features.jellyfin.org/posts/27/home-assistant-integration
- https://features.jellyfin.org/posts/51/last-fm-scrobbling
- https://features.jellyfin.org/posts/64/feature-request-amazon-alexa-support
- https://features.jellyfin.org/posts/65/feature-request-server-user-usage-statistics
- https://features.jellyfin.org/posts/74/allow-users-to-subscribe-to-tv-shows-to-be-notified-when-new-items-are-added
- https://features.jellyfin.org/posts/230/support-for-oidc-oauth-sso
- https://features.jellyfin.org/posts/275/support-google-assistant-and-alexa
- https://features.jellyfin.org/posts/301/option-to-kill-a-stream-from-a-user
- https://features.jellyfin.org/posts/463/imbd-rating-and-rotten-tomatoes-audiance-rating-and-fresh-rating-on-movies-and-tv-shows
- https://features.jellyfin.org/posts/470/jellyfin-prometheus-exporter
- https://features.jellyfin.org/posts/576/watchlist-like-netflix
- https://features.jellyfin.org/posts/593/dynamic-smart-collections-defined-by-custom-queries
- https://features.jellyfin.org/posts/604/request-shows-movies-music-from-sonarr-radarr-lidarr
- https://features.jellyfin.org/posts/633/watched-history
- https://features.jellyfin.org/posts/684/letterboxd-integration
- https://features.jellyfin.org/posts/1376/statistics-graph-of-cpu-ram-and-network-information
- https://features.jellyfin.org/posts/1588/keep-seen-status-when-sonarr-radarr-download-a-new-video-version
- https://features.jellyfin.org/posts/1754/export-settings-and-watched-status
- https://features.jellyfin.org/posts/2166/allow-plugins-to-expose-a-config-page-to-non-admin-users
- https://features.jellyfin.org/posts/2329/improve-recently-added-to-not-show-upgraded-items
- https://features.jellyfin.org/posts/2411/integration-with-listenbrainz
- https://features.jellyfin.org/posts/2623/add-a-differential-api-like-kodi-sync-queue
- https://features.jellyfin.org/posts/2897/push-notifications-in-mobile-apps
- https://features.jellyfin.org/posts/4039/support-nested-collections-collections-within-collections
- https://features.jellyfin.org/posts/4180/let-plugins-show-a-settings-page-to-non-admin-users

Plex

- https://support.plex.tv/articles/115002267687-webhooks/ (event list read through search results; direct fetch was refused)
- https://developer.plex.tv/pms/
- https://forums.plex.tv/t/jwt-authentication/931646
- https://www.plex.tv/blog/new-lifetime-plex-pass-pricing/
- https://9to5mac.com/2025/03/19/plex-price-increase-remote-streaming-changes/
- https://www.androidauthority.com/plex-remote-watch-pass-price-increase-3663060/
- https://forums.plex.tv/t/plex-discord-integration/287994
- https://forums.plex.tv/t/plexamp-feature-request-discord-integration/381581
- https://forums.plex.tv/t/feature-request-support-listenbrainz-replacement-for-last-fm/226426
- https://forums.plex.tv/t/plex-music-and-last-fm-integration/902672
- https://forums.plex.tv/t/use-listenbrainz-for-recommendations/849874
- https://forums.plex.tv/t/plex-webhooks-trailers-pre-roll-credits/498486
- https://forums.plex.tv/t/add-watchlist-webhook-events/787893

Emby

- https://emby.freshdesk.com/support/solutions/articles/44001848859-webhooks
- https://emby.media/community/topic/113134-giving-webhooks-some-attention/
- https://jellywatch.app/blog/awesome-emby-plugins-ecosystem-complete-guide-2026 (secondary source)

Navidrome and OpenSubsonic

- https://www.navidrome.org/docs/usage/features/plugins/
- https://www.navidrome.org/docs/usage/integration/external-services/
- https://github.com/navidrome/navidrome/releases/tag/v0.60.0
- https://github.com/navidrome/navidrome/releases/tag/v0.64.0
- https://github.com/navidrome/navidrome/pull/5730
- https://github.com/navidrome/navidrome/pull/4833
- https://github.com/navidrome/navidrome/blob/master/server/jellyfin/README.md
- https://github.com/navidrome/discord-rich-presence-plugin
- https://github.com/kgarner7/navidrome-listenbrainz-daily-playlist
- https://github.com/xmorose/spindle
- https://opensubsonic.netlify.app/docs/extensions/
- https://opensubsonic.netlify.app/docs/opensubsonic-api/
- https://github.com/opensubsonic/open-subsonic-api

Request managers and the *arr stack

- https://docs.seerr.dev/
- https://docs.seerr.dev/using-seerr/notifications/
- https://github.com/seerr-team/seerr
- https://github.com/seerr-team/seerr/issues/183
- https://github.com/seerr-team/seerr/issues/96
- https://github.com/seerr-team/seerr/issues/308
- https://github.com/seerr-team/seerr/issues/264
- https://github.com/seerr-team/seerr/issues/2449
- https://github.com/seerr-team/seerr/pull/1238
- https://github.com/sct/overseerr
- https://store.elfhosted.com/blog/2026/02/17/overseerr-and-jellyseerr-merge-into-seerr/
- https://github.com/Ombi-app/Ombi
- https://github.com/Sonarr/Sonarr/tree/v5-develop/src/NzbDrone.Core/Notifications
- https://github.com/Sonarr/Sonarr/issues/8805
- https://github.com/Sonarr/Sonarr/pull/8816
- https://github.com/Lidarr/Lidarr/tree/develop/src/NzbDrone.Core/Notifications
- https://github.com/Lidarr/Lidarr/issues/5498
- https://github.com/Lidarr/Lidarr/issues/5733
- https://wiki.servarr.com/readarr
- https://github.com/recyclarr/recyclarr
- https://piunikaweb.com/2026/02/24/huntarr-security-vulnerability-arr-api-keys-exposed/
- https://github.com/rfsbraz/huntarr-security-review

Subtitles

- https://github.com/morpheus65535/bazarr
- https://wiki.bazarr.media/Additional-Configuration/Webhooks/
- https://github.com/morpheus65535/bazarr/pull/3293
- https://github.com/morpheus65535/bazarr/issues/3609

Trackers, scrobblers and sync

- https://forums.trakt.tv/t/updating-trakt-limits-for-2026/101592
- https://forums.trakt.tv/t/upcoming-vip-renewal-pricing-changes-effective-june-12-2025/56649
- https://www.neowin.net/news/trakt-vip-receives-up-to-300-price-hike-going-back-on-promise-to-honor-legacy-subs/
- https://forums.trakt.tv/t/unable-to-create-a-new-api-application-after-purchasing-trakt-vip-the-create-button-does-nothing/119966
- https://selfh.st/weekly/2026-08-07/
- https://ettayeb.fr/en/selfhosted/trakt-api-paywall-selfhosted-media/
- https://github.com/Taxel/PlexTraktSync/issues/2548
- https://github.com/Taxel/PlexTraktSync/issues/2548#issuecomment-5446805470
- https://github.com/Taxel/PlexTraktSync/issues/2452
- https://github.com/Taxel/PlexTraktSync/issues/2501
- https://github.com/arabcoders/watchstate
- https://github.com/arabcoders/watchstate/blob/master/FAQ.md
- https://github.com/luigi311/JellyPlex-Watched
- https://github.com/cenodude/CrossWatch
- https://github.com/FuzzyGrim/Yamtrack
- https://github.com/IgnisDa/ryot
- https://github.com/MALSync/MALSync
- https://listenbrainz.readthedocs.io/en/latest/users/api/core.html
- https://www.last.fm/api/scrobbling
- https://github.com/FoxxMD/multi-scrobbler
- https://github.com/aubrey-wodonga/eavesdrop.fm

Statistics and maintenance

- https://github.com/Tautulli/Tautulli/blob/master/CHANGELOG.md
- https://github.com/CyferShepard/Jellystat
- https://github.com/CyferShepard/Jellystat/issues/298
- https://github.com/CyferShepard/Jellystat/issues/328
- https://github.com/fredrikburmester/streamystats
- https://github.com/connorgallopo/Tracearr
- https://github.com/finstats/finstats
- https://github.com/aunefyren/wrapperr
- https://github.com/RaunoT/plex-rewind
- https://github.com/Chaphasilor/jellyfin-rewind
- https://github.com/johnpc/jellyfin-wrapped
- https://github.com/Maintainerr/Maintainerr
- https://github.com/Schaka/janitorr

Metadata and collections

- https://kometa.wiki/en/latest/
- https://kometa.wiki/en/latest/config/overview/
- https://github.com/Kometa-Team/Kometa
- https://github.com/Kometa-Team/Kometa/issues/321
- https://github.com/Kometa-Team/Kometa/issues/3351
- https://github.com/Kometa-Team/Kometa/pull/3150
- https://github.com/fscorrupt/posterizarr

Smart home, Discord and onboarding

- https://www.home-assistant.io/integrations/plex/
- https://www.home-assistant.io/integrations/jellyfin/
- https://www.music-assistant.io/music-providers/
- https://github.com/music-assistant/server
- https://github.com/phin05/discord-rich-presence-plex
- https://github.com/oonqt/MBCord
- https://github.com/wizarrrr/wizarr

Clients and standards

- https://github.com/finamp-app/finamp
- https://github.com/Jellify-Music/App
- https://github.com/jeffvli/feishin
- https://www.standardwebhooks.com/
- https://github.com/caronc/apprise
