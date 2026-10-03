# Integrations and extensibility

This area covers everything that lets other software work with Gunmetal: the
native API and its documentation, the tokens that tools use, events and
webhooks, the plugin sandbox and its extension points, the OpenSubsonic and
Jellyfin compatibility adapters, and the third-party tools self-hosters
already run. Those tools include scrobblers, trackers, the *arr stack,
Bazarr, Seerr, stats dashboards, collection managers, Home Assistant and
Discord. We are setting this bar. Every integration is free. Every tool works
through a credential that can do only its own job and can be revoked. No
plugin can take over the server, and no upgrade silently breaks an
integration. The apps and tools people already use connect through adapters
from the first release that can support them, and nothing leaves the server
unless a person allowed it. The rivals are good in places. Jellyfin has free
webhooks, a public OpenAPI spec and maintained SDKs. Plex has the most
complete Home Assistant and Trakt story. Navidrome already ships sandboxed
plugins, per-user scrobbling and OpenSubsonic. Some user-facing features
belong to a sibling feature map, such as sign-in, importers from Plex and
Jellyfin, the admin dashboard, metadata providers and lyrics. For those, the
rows here cover only the integration surface.

## Features

Claims about rivals come from `docs/research/`. "(unverified)" marks
anything the research could not confirm. When a cell names another feature
by its ID (for example INT-064), that ID is a row in this file. 
Releases (R1, R2, R3, Later, No), the Demand scale, row ownership and the
terms "the user log" and "the identity store" are defined once in the
[feature map README](README.md). A row whose Release cell would differ
between maps names one owning row; the other maps point at it.

### Native API and developer surface

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| INT-001 | API reference generated from code | Developers can build tools without reverse-engineering the server | Plex yes (published 2025); Jellyfin yes (OpenAPI); Emby yes (depth unverified) | Medium: Jellyfin keeps three SDKs on its spec; Plex published one in 2025 | R1 | Parity with Jellyfin's generated OpenAPI spec; the edge is the conformance suite (INT-004) and the CI check that every route is in the spec. The spec is generated from the same Rust protocol types that the server and clients compile, so it cannot drift from what the server does; CI fails if any route is missing from it | Spec generation in the build; spec served by the server and the docs site | API docs page served by the server; docs site |
| INT-002 | Versioned API with a deprecation window | Integrations keep working across upgrades | Jellyfin policy new in 12.0, yet 12.0 still broke Sonarr's connector and old clients; Plex no published policy (unverified) | High: Sonarr issue 8805; Jellyfin 12.0 broke old clients | R2 | Semantic versioning. Deprecated routes stay for at least one major release and are flagged in a response header, and the conformance suite (INT-004) pins their behaviour R1 ships a version prefix on every route; the deprecation window and its machinery arrive in R2. | Per-version route table; deprecation header; usage counter for deprecated routes | API docs page; Admin > Integrations lists tokens still calling deprecated routes |
| INT-003 | Integrator changelog | Developers see exactly what changed in each release | None of the three publishes one; Plex's move to JWTs left developers asking for examples | Medium: Plex JWT forum thread; the Sonarr fix after Jellyfin 12.0 | R2 | Generated from a diff of the API spec between releases, so no change goes unlisted | Spec-diff step in the release pipeline | Docs site; release notes |
| INT-004 | Published conformance suite | Tool and adapter authors can test against the contract | None of the three | Low: no direct request; follows from API churn | R2 | The same suite gates Gunmetal's own CI under the 100% coverage and zero-mutant rules | Recorded request and response fixtures; runnable harness | Docs site |
| INT-005 | Capability discovery | A tool learns the API version, the enabled adapters and extensions, and the scopes it holds | Video servers no; OpenSubsonic `getOpenSubsonicExtensions` is the model | Low | R1 | It reports only what is enabled and what the caller may use. Unauthenticated callers learn nothing, unlike the endpoints listed in Jellyfin issue 5415 | Capability registry | None (API only) |
| INT-006 | Change feed (delta sync) | Tools fetch only what changed since their last visit | Jellyfin only through the Kodi Sync Queue plugin; Plex and Emby (unverified) | Low: "differential API" 6 votes, but every sync tool needs it | R1 | It exposes the same change log that syncs the library to Gunmetal's own devices, with a cursor per consumer and filtering by scope | Change-log retention and compaction; cursor per token | Token detail shows feed position |
| INT-007 | Consistent list endpoints | Paging, filtering and caching work the same way everywhere | Not compared in the research | Low | R1 | Cursor pagination, field selection and conditional requests (ETag) are defined once in the protocol types | Shared query layer | None (API only) |
| INT-008 | IDs that survive rebuilds and file replacement | References stored in other tools keep working | Jellyfin 10.11 lost watch state on replace or rename (#15001); Plex exposes stable GUIDs | Medium: #15001 had 61 +1; Maintainerr cross-checks matches by year | R1 | Random public IDs are bound to content identity as defined by LIB-028 and kept in the identity store, outside the SQLite cache, so a rebuild reissues the same IDs. | The identity store, included in backups; identity rules from LIB-028 | "Copy ID" on item pages in developer mode |
| INT-009 | MusicBrainz IDs on music items | Scrobblers, Lidarr and other tools match the exact recording, release and artist | Plex matches music to MusicBrainz (forum claim); Jellyfin reads MBIDs; Navidrome uses them | Medium: needed for scrobble linking and loved-track sync | R1 | Read from tags at scan time by the core parsers, with no network lookup | MBID fields in the music model and the API | Album and track info panels |
| INT-010 | Video provider IDs on every item | Seerr, Sonarr, Radarr and Maintainerr can match films and episodes | Plex, Jellyfin and Emby all expose them | High: Seerr availability sync and Maintainerr depend on them | R2 | IDs from filenames, NFO files and provider plugins carry their source per field, so a tool can see where an ID came from | Provider ID fields with provenance | Title info panel |
| INT-011 | Path-scoped library refresh | A downloader or tagger says "this folder changed" and the change appears within seconds | Plex yes; Jellyfin yes (takes paths); Emby yes | High: every *arr connector relies on one | R1 | The token is scoped to named roots. A refresh reads only headers and tags with the core parsers, so it stays cheap even on a NAS | Path validation against roots; debounced, targeted scan job | Connection recipes (INT-025); Admin > Library activity |
| INT-012 | Per-token rate limits | A buggy or greedy tool cannot slow the server for everyone | None documented; Jellystat polled every second and starved Jellyfin | Medium: Jellystat issues 298 and 328 (34 comments) | R1 | Limits apply per token and come with retry headers. The event stream (INT-048) removes the reason to poll | Token buckets; 429 responses with retry-after | Usage and throttling on token detail |
| INT-013 | Health check for uptime monitors | A monitor can see that the server is up | Not compared in the research | Low | R1 | It returns a status and nothing else: no version, user or library data | One minimal route, covered by the route-policy test | None |
| INT-014 | Built-in API explorer | Try calls against your own server | Not compared in the research | Low | R2 | Served by the server with no CDN, using a short-lived read-only token derived from the signed-in session | Static docs bundle; session-derived token | API docs page |
| INT-015 | Official SDKs | Typed client libraries for tool authors | Jellyfin yes (TypeScript, Kotlin, Swift); Plex none official; Emby (unverified) | Low | R2 | Parity | SDKs generated from the spec on every release, TypeScript first | Docs site |
| INT-016 | Agent access (MCP) | Ask an AI assistant about your library | Plex remote MCP (unverified); Jellyfin community servers (unverified); Streamystats AI chat | Low: emerging | Later | A thin layer over read-only scoped tokens, off by default, so an agent can never change anything | MCP transport mapped onto read scopes | Admin > Integrations; Account > Apps and tokens |

### Tokens, scopes and access for integrations

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| INT-017 | Scoped tokens | See ACC-049, which owns this feature. Integration specifics: the named scopes ("refresh library", "read history", "read library and availability", "play"), and a route-table test that fails the build if any route lacks a scope. | Plex no (unverified); Jellyfin no (#13992 open); Emby no (unverified); Immich yes | Medium: #13992 23 +1; Huntarr leaked every connected *arr key | R1 | See ACC-049. | None beyond ACC-049. | Admin > API tokens; Account > Apps and tokens |
| INT-018 | Scope limits by library, root and user | Seerr sees only the film and TV libraries; Lidarr can refresh only the music folder | None | Medium | R1 | The same object filter that applies user permissions evaluates these limits | Limit checks on queries and refresh paths | Token editor |
| INT-019 | Expiry and rotation | A leaked key stops working on its own | Plex 7-day JWTs plus legacy tokens; Jellyfin long-lived (unverified); Navidrome's Jellyfin tokens never expire | Medium | R1 | Every token expires, with a default set by its type. Rotation issues a successor with an overlap window, and a reminder event fires before expiry | Expiry job; rotation route; expiry event | Token list; dashboard banner |
| INT-020 | Last use and audit trail per token | See which tool did what, and when | None documented; OpenSubsonic's API-key extension requires listing and revoking keys | Medium | R1 | Every call is attributed to a token, and the list shows last use, address and client | Audit log with retention | Token detail |
| INT-021 | Revoke one or revoke all | Cut off one tool, or everything after a breach | Plex sign-out-all after its 2025 breach; Jellyfin partial | Medium: the Plex breach response | R1 | Revocation takes effect on the next request, and signed URLs minted under the token stop: new requests fail at once, and an in-flight response is cut by the ACC-122 mechanism | Revocation check per request | Token list; Account > Apps and tokens |
| INT-022 | No-escalation rule | A token can never create or widen a token beyond its own rights | Immich CVE-2026-23896 let a key raise its own permissions | Medium: a 2026 CVE | R1 | Every token create or update is checked against the caller's own scopes, and the checks are mutation-tested | Policy check on token routes | Error in token editor |
| INT-023 | Credentials in headers only | Keys never leak into proxy or browser logs | Plex, Jellyfin and Emby all accept tokens in query strings; Bazarr's Plex webhook URL carries its key | Medium: Jellyfin #5415 | R1 | The native API refuses credentials in query strings. Media and images use short-lived signed URLs bound to a session, and the adapter routes that need query-string auth are documented exceptions | URL signer; refusal path | None (docs) |
| INT-024 | Per-app credentials for third-party apps | See ACC-129, which owns this feature. | OpenSubsonic API keys; classic Subsonic needs a password equivalent, which Navidrome stores encrypted | Medium | R2 | See ACC-129. | None beyond ACC-129. | Account > Apps and tokens ("Connect a music app", with a QR code) |
| INT-025 | Connection recipes | Pick "Lidarr" or "Symfonium" and get the URL, the header and a token with the right scope | None; Sonarr's old Jellyfin connector test passed with a fake key (#8805) | Medium | R2 | Each recipe creates a least-privilege token and shows exactly what to paste. Recipes for video tools arrive with R2 | Recipe definitions mapped to scopes | Admin > Integrations > Connect a tool |
| INT-026 | "Who am I" check for tools | A tool's test button proves that the key and its scope work | Sonarr's test passed with a fake key (#8805) | Low | R1 | One route returns the token's identity and scopes, and the adapters return real 401s, so a connection test cannot pass falsely | Introspection route | None (API only) |
| INT-027 | Device-code sign-in for tools and scripts | A command-line tool or helper gets a token without anyone copying secrets around | Jellyfin Quick Connect and Plex link codes (for apps, not tools) | Low | R2 | Uses the same pairing flow as TVs: the tool shows a code, and the user approves it on a signed-in device and picks the scopes | Pairing routes; scope picker | Approval sheet on phone and web |
| INT-028 | Companion-tool sign-in without passwords | Users sign in to Seerr and similar tools without a server password | Seerr signs in with server credentials; no OIDC yet | High: Seerr OIDC issue 277 reactions | R2 | A consent screen lets a tool get a scoped, expiring token for one user. It works alongside OIDC when the household runs an identity provider | Authorisation-code flow; consent records | Consent screen; Account > Apps and tokens |
| INT-029 | Integrations inventory | One page lists every token, webhook, plugin and connected app, what each can do, and when it last acted | None | Low | R2 | Possible because all four go through one scope model | Aggregation query | Admin > Integrations |

### Events, webhooks and notifications

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| INT-030 | Built-in webhooks, free | Automations trigger when things happen | Plex yes (Plex Pass); Jellyfin free official plugin; Emby Premiere per its help article | Medium: paywall complaints; a lifetime Plex Pass now costs $749.99 | R2 | Parity with Jellyfin's free webhook plugin on price; the edge is the transactional outbox and signed payloads (INT-038). The outbox table ships in R1, written in the same transaction as each change, so no event from R1 onward is lost; delivery to destinations arrives in R2. | Outbox table; delivery worker | Admin > Webhooks |
| INT-031 | Versioned event catalogue | A clear list of triggers whose payloads do not change shape without notice | Jellyfin 23 notifier types; Plex a shorter list | Medium | R2 | Event types are protocol types with schema versions and appear in the API spec. Music events come in R1, video in R2 and live TV in R3 | Event schemas | Event picker in the webhook editor; API docs page |
| INT-032 | Playback events with a "counted" threshold | Start, pause, resume, stop and "counted as played" | Plex counts a scrobble past 90%; Jellyfin start, progress and stop | Medium | R2 | For music, "counted" follows the Last.fm and ListenBrainz rule: tracks over 30 seconds count after half their length or four minutes, whichever comes first. Video uses a set threshold. Both use play time reported by the client, including offline plays uploaded later | Session tracking; threshold rules in the core | Event picker |
| INT-033 | Library events grouped and sent after metadata | One "album added" message instead of twelve track messages | Jellyfin requests (#31, #329, 7 reactions each); Emby waits for metadata | Low | R2 | The scanner knows where a batch ends, so it emits one event per album (and per season in R2) after tags and artwork are read | Batch boundaries; debounce window | Grouping switch in the webhook editor |
| INT-034 | Replacement events | An upgraded file is reported as replaced, not new | Jellyfin "Recently Added shows upgrades" 62 votes | Medium: 62 votes | R2 | The identity matcher emits `item.replaced` and keeps history (INT-120) | Identity matcher | Event picker |
| INT-035 | Played, unplayed, rating and favourite events | Sync tools react when someone marks or rates something by hand | Plex rate event; Jellyfin plugin request (#118, 8 reactions) | Low | R2 | Emitted straight from the log, so every source of a change (app, adapter or import) produces the same event | Log-to-outbox bridge | Event picker |
| INT-036 | Account and security events | New device, failed sign-ins, token created, revoked or near expiry | Plex new-device event; Jellyfin sign-in success and failure, lockouts | Medium | R2 | Device-bound keys make "new device" a fact rather than a guess from user agents | Security event emitter | Event picker; Account > Security |
| INT-037 | Server and maintenance events | Update available, backup done or failed, integrity problem, task failed | Plex backup and database-corruption events; Jellyfin update notice planned | Medium: Jellyfin update notification 51 votes | R2 | Integrity events carry the typed parser or check error, so the message says what is wrong and where | Hooks into backup, health checks and the update check (operations map) | Event picker; admin dashboard |
| INT-038 | Signed payloads | The receiver can prove the event came from your server | Plex, Jellyfin and Emby do not sign (as far as checked) | Low | R2 | Uses the Standard Webhooks scheme: HMAC-SHA256 with ID, timestamp and signature headers, a replay window, and secret rotation with overlap | Secret store; signer | Webhook editor (reveal and rotate the secret) |
| INT-039 | Retries, delivery log and redelivery | Missed events are not lost, and failures are visible | Unverified for all three; WatchState recommends polling as a backstop | Medium: WatchState FAQ | R2 | Outbox entries are retried with backoff. The log shows the request, status and latency, and any delivery can be sent again | Delivery log with retention | Deliveries tab on each webhook |
| INT-040 | Test button and sample payloads | Check that a webhook works before relying on it | Emby yes (sample payload); Jellyfin missing | Medium: Jellyfin webhook issues with 48 and 28 reactions | R2 | Samples are generated from the schema using real items from your library | Sample generator | Webhook editor |
| INT-041 | Filters by event, user and library | Only the events you care about are sent | Emby yes; Jellyfin item type and user (unverified); Plex per account | Low | R2 | Filters reuse the scope model, so a webhook never carries data from a library it may not see | Filter evaluation | Webhook editor |
| INT-042 | Templates that cannot run code | Each message is shaped for its destination, safely | Jellyfin Handlebars; Tautulli's evaluated notification text was a remote-code-execution bug (CVE-2026-28505) | Medium: Tautulli's 2026 CVEs | R2 | Templates only substitute values and never evaluate expressions, and they are checked against the event schema | Template renderer as pure code in the core | Template editor with live preview |
| INT-043 | Artwork in payloads, safely | Rich embeds in Discord and chat apps | Plex thumbnail (unverified); Jellyfin request (1 vote) | Low | R2 | Uses signed, expiring image URLs. When the server is not public, items with a MusicBrainz ID use Cover Art Archive. A token never appears in the payload | Image URL signer | Template editor |
| INT-044 | First-party destinations | Messages arrive where you already look | Jellyfin: Discord, Gotify, Pushbullet, Pushover, Slack, SMTP, MQTT, generic; Plex: generic HTTP only | Medium | R2 | Generic HTTP, ntfy, Discord, Gotify and Slack-compatible presets are all built on one generic sender plus templates, so each preset is small and tested | Preset definitions | Destination picker |
| INT-045 | New arrivals posted to a chat channel | "Album added" appears in a Discord channel | Jellyfin via its webhook plugin; Plex via Plex Pass webhooks or Tautulli; Emby (Premiere) | Low | R2 | Grouped events (INT-033) plus the Discord preset (INT-044) mean one message per album, not per track | None beyond webhooks | Webhook editor |
| INT-046 | Email destination | Event emails to the admin or to users | Jellyfin SMTP notifier; Plex no | Low: Jellyfin email overhaul request 47 votes | R2 | Parity | SMTP settings with TLS required; the same templates | Admin > Notifications settings |
| INT-047 | MQTT destination | Events go to an MQTT broker for home automation | Jellyfin via its webhook plugin; Plex no | Low | R2 | Adds a retained "now playing" topic per session, so home automations can read current state | MQTT client | Destination picker |
| INT-048 | Server-sent event stream | Dashboards and tools get events pushed to them instead of polling | Plex event stream; Jellyfin WebSocket (Jellystat polled anyway); Emby needs a plugin for Tracearr | Medium: Jellystat issues 298 and 328 | R2 | Carries the same catalogue as webhooks, filtered by token scope. After a disconnect, the stream resumes from the outbox | Fan-out; replay window | None (API only) |
| INT-049 | Network rules for webhooks | Admin webhooks can reach Home Assistant on the LAN; other traffic cannot | None documented | Low | R2 | Admin-created webhooks may target private addresses. Per-user webhooks and plugins may not, unless the admin allows it, and the check runs on the resolved address | Destination classifier | Warning in the webhook editor |
| INT-050 | Per-user follows and alerts | "New album from an artist I follow", "new episode of my show" | Jellyfin subscribe-to-show requested; Seerr tells requesters when titles arrive | Medium: Jellyfin request 50 votes | R2 | Follows are user data in the user log. Alerts go to an in-app inbox that syncs like the library, and optionally to the user's own ntfy topic | Follow records; fan-out job | Follow button on artist and show pages; Account > Notifications; inbox |
| INT-051 | Per-user webhooks | Each person can wire up their own automations | Plex per-account webhooks; Jellyfin and Emby admin only (unverified) | Low | R2 | Each user's webhooks are limited by scope to that user's own events | Per-user webhook records | Account > Notifications |
| INT-052 | Native push to phones | Alerts on the lock screen | Emby pushes to its apps (secondary source); Jellyfin requested | Medium: Jellyfin request 48 votes | Later | Needs Apple's and Google's push services plus a relay, which conflicts with "no central account" (see open decisions) | Relay design | Phone notification settings |
| INT-053 | Live TV and recording events | Recording scheduled, started, failed or finished; tuner conflicts | Not compared for this event set; recordings fail silently on rivals | Medium: silent recording failures in the live TV research | R3 | The recorder's own failure reports become events, so you hear about a failed recording when it happens | Recorder hooks | Event picker |

### Plugin platform

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| INT-054 | WebAssembly plugin sandbox | You can extend the server without risking it | Plex no third-party plugins; Jellyfin .NET DLLs in-process; Emby in-process (unverified); Navidrome WebAssembly | Medium: Jellyfin 12.0 plugin removal and plugin path-traversal fixes | R2 | A wasmtime host inside the server. Plugins can reach only the host functions they were granted. R1 runs first-party plugins only | Runtime; host API with its own test suite | Admin > Plugins |
| INT-055 | Permissions declared and reviewed before enabling | You see what a plugin can touch before it runs | Navidrome manifest with reasons; Jellyfin none | Low | R2 | The manifest lists hosts, library read, user data, storage and schedule, each with a reason, and the host enforces exactly that list | Manifest parser in the core | Permission review sheet |
| INT-056 | Network allowlist checked on the resolved address | A plugin can call only the hosts it declared and never your LAN | Navidrome yes, after two advisories showed DNS, redirect and WebSocket bypasses; Jellyfin none | Low | R2 | The host's HTTP function checks the resolved address when connecting and re-checks every redirect. It blocks private and loopback addresses unless granted, and caps size and time | Egress layer in the host | Plugin detail > Network |
| INT-057 | Plugin network log | You can see which hosts each plugin contacted | None | Low | R2 | The host records the host, time, bytes and status of every outbound request (never bodies) | Log with retention | Plugin detail > Activity |
| INT-058 | Per-user settings and secrets | Each person links their own Last.fm or Trakt account, and the admin never sees the token | Jellyfin no (the admin enters every user's settings); Navidrome yes | Low: two Jellyfin requests, 4 votes each, but the admin holds everyone's tokens | R2 | Secrets are encrypted per user and handed only to the plugin acting for that user. The admin sees "linked", never the secret | Per-user secret store | Account > Connected services |
| INT-059 | Settings forms generated from a schema | Plugins get settings screens on every device without shipping web code | Navidrome JSONForms; Jellyfin plugins ship their own HTML and JavaScript | Low | R2 | Gunmetal's own client draws the form on TVs and phones too, so no plugin script ever reaches the admin UI | Schema validation | Plugin settings screens |
| INT-060 | Versioned plugin interface | A server upgrade does not break your plugins | Jellyfin 12.0 required removing all third-party plugins; 10.11.9 broke repository lookups (#16905) | Medium | R2 | The interface is defined in WIT and versioned, and the host supports the current and previous major version. An incompatible plugin is disabled with a reason and never crashes the server | Interface versions and shims | Compatibility badge in the plugin list |
| INT-061 | Resource limits | A plugin cannot pin the CPU or eat memory | No rival documents any | Low | R2 | Fuel or epoch interruption, memory caps and per-call timeouts | Runtime limits | Plugin detail > Limits |
| INT-062 | Failure isolation and auto-disable | A broken plugin is switched off with a reason, and the server carries on | Not documented in the research | Low | R2 | A crash stays inside the sandbox. Repeated failures disable the plugin and emit an event | Health tracking | Plugin status; event |
| INT-063 | Isolated plugin storage | Removing a plugin removes its data | Navidrome per-plugin storage; Jellyfin per-plugin config files (unverified) | Low | R2 | Each plugin gets its own namespace. State it cannot lose, such as a scrobble cursor, lives in the user log, so a cache rebuild keeps it | Storage namespace; durable cursor records | Plugin detail > Data |
| INT-064 | Durable delivery cursors | After an outage, an API change or a phone being offline, a scrobbler or tracker carries on exactly where it stopped | Jellyfin ListenBrainz plugin caches listens during outages; others not documented | Medium: Trakt API churn; scrobble gaps | R2 | Each sync plugin reads the append-only log through its own cursor, using stable event IDs, so delivery is in order and never doubled | Cursor per plugin and user; event IDs | "Last sent" on Account > Connected services |
| INT-065 | Signed first-party plugins | You know who built a plugin and that nobody tampered with it | No rival signs plugins (unverified) | Low | R2 | Signed with the project's release key and checked at install and at load | Signature check | Publisher badge on plugin detail |
| INT-066 | Project-maintained first-party plugins | The integrations people rely on do not depend on one tired volunteer | Jellyfin's SSO plugin, original Last.fm plugin, Jellyfin-RPC and jellysub are all archived | Medium: key-person failures in the research | R2 | Last.fm and ListenBrainz ship in R1, followed by Trakt and the metadata providers. All live in the main repository under the same test gate | Plugin release process | Admin > Plugins |
| INT-067 | Third-party plugins installed by URL | You can add community extensions | Jellyfin extra repositories by URL; Emby curated catalog | Medium | R2 | Same sandbox and review sheet. An unsigned plugin needs an explicit confirmation, and updates are opt-in | Fetch, verify, update check | Admin > Plugins > Add |
| INT-068 | Curated plugin catalog | Browse vetted plugins | Jellyfin official repository; Emby curated catalog | Low | Later | Parity | Static signed index on gunmetal.tv; no accounts | Admin > Plugins > Browse |
| INT-069 | Rust plugin SDK and test harness | Authors can build and test plugins locally | Jellyfin C# plugin template; Navidrome kits in four languages | Low | R2 | The host's own conformance tests ship with the SDK, so authors test against the real contract | SDK; mock host | Docs site; developer mode in Admin > Plugins |
| INT-070 | More plugin languages | Authors can write plugins in Go, Python or JavaScript | Jellyfin C# only; Navidrome Go, Rust, Python, JavaScript | Low | Later | Parity | Language kits | Docs site |
| INT-071 | Scheduled plugin tasks | Plugins run jobs in the maintenance window | Jellyfin plugins add scheduled tasks | Low | R2 | Plugin tasks appear in the task list with progress and a cancel button (operations map) | Scheduler grant | Admin > Tasks |
| INT-072 | Plugin check before upgrading | Before an upgrade, you see which plugins it would disable | No rival offers one | Medium: Jellyfin 12.0 upgrade pain | R2 | The release feed lists interface versions, and the server compares them with the installed plugins | Preflight check | Admin > Updates |
| INT-073 | In-process native plugins | Plugins that run as native code inside the server | Jellyfin yes; Emby yes (unverified) | Low: nobody asks for in-process plugins as such | No | Not doing: a native plugin could bypass every grant, and the sandbox is the whole point | None | None |
| INT-074 | Plugins that inject scripts into the web client | Home screen and player tweaks through injected JavaScript | Jellyfin community plugins do it (Jellyfin-Enhanced, 1,869 stars) | Medium: the demand is real | No | Not doing: script injection is an XSS hole. Declarative home rows and themes (INT-081) meet the same demand | None | None |

### Plugin extension points

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| INT-075 | Scrobbler extension point | Plays are reported to outside services | Jellyfin via plugins; Navidrome scrobbler capability | Medium | R2 | Fed by the log cursor (INT-064), not by live session hooks, so no play is missed | Host interface | Account > Connected services |
| INT-076 | History-import extension point | Pull years of history from a service | No rival has one built in | Low | R2 | Imported events are tagged with their source and can be removed as a batch | Import job; source tags | Account > Data > Import |
| INT-077 | Metadata and artwork provider extension point | You choose or add the sources of metadata and images | Plex HTTP providers (beta, no music); Jellyfin in-process plugins; Navidrome sandboxed agents | High: nobody accepts a film library without posters (library research) | R2 | Each provider declares its hosts. The host, not each plugin author, enforces the provider's terms, such as TMDB cache age and attribution. The library map owns provider order and field provenance | Provider interface; terms enforcement | Admin > Library > Providers |
| INT-078 | Lyrics provider extension point | Lyrics are found for files that have none | Plex yes (Plex Pass); Jellyfin provider plugins; Navidrome provider plugins | Medium | R2 | A first-party LRCLIB plugin (free, keyless, open source) with a grant for one host, caching results with the item | Provider interface | "Find lyrics" in the lyrics view |
| INT-079 | Subtitle provider extension point | You can search for subtitles | Plex built in (OpenSubtitles); Jellyfin plugin from the item menu | Medium | R2 | Results go to the running player, which can load them without a restart (INT-122) | Provider interface | Player subtitle menu |
| INT-080 | List-source extension point | Collections fed by Trakt, MDBList, Letterboxd, TMDB or IMDb lists | Kometa for Plex; Kometa support for Jellyfin in progress | Medium: Kometa 3,442 stars | Later | A plugin with a network grant refreshes the list into a declarative collection (INT-140) | Provider interface; schedule | Collection editor "source" |
| INT-081 | Home-row and theme extension point | Plugins add labelled, removable home rows and visual themes | Jellyfin Home Screen Sections plugin (web only); Emby Home Screen Companion | Medium | Later | Rows are typed data that Gunmetal's clients draw on every platform, never injected script (the discovery map owns rows) | Row interface | Home screen editor |
| INT-082 | Notification-destination extension point | Notifications can reach any service Apprise reaches | Jellyfin fixed list; Apprise covers most services | Low | Later | Long-tail destinations live in plugins, so the server ships only a tested core set | Destination interface | Destination picker |
| INT-083 | Search and recommendation provider extension point | Plugins add search sources or "more like this" engines | Jellyfin 12.0 yes | Low | Later | Parity | Provider interface | Search; radio |
| INT-084 | Guide-data provider extension point | You can use Schedules Direct or other guide sources | Plex bundled guide (Plex Pass); Jellyfin Schedules Direct, which says Jellyfin ignores its rate limits | Medium: Schedules Direct IP blocks | R3 | The host enforces each provider's rate limits, so users are not blocked | Provider interface; rate limiter | Live TV > Guide sources |
| INT-085 | Podcast feed plugin | Podcasts in the same app | Plex removed podcasts; Jellyfin requested; Audiobookshelf yes | Medium: Jellyfin request 563 votes | Later | The fetcher refuses private addresses and re-checks redirects, and feed parsing is pure code in the core (adjacent media map) | Fetcher plugin; feed parser | Podcasts section |

### Compatibility adapters

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| INT-086 | OpenSubsonic adapter | Symfonium, Feishin, Amperfy, Supersonic, Tempus and other Subsonic apps connect, bringing their offline, car and watch support | Plex no; Jellyfin no (request "started" since 2020, jellysub archived); Navidrome, gonic, Ampache, LMS yes | High: Jellyfin request 77 votes; users stay for clients | R2 | Browsing, search, streaming, cover art, playlists, stars, ratings, scrobbles and the play queue are served from the native music model through an ID translation layer, with no second data model Ships in R2 (see the release key in the feature map README); until then R1 phone users have only the installable web app (CLI-003). | Adapter routes; ID mapping | Admin > Compatibility |
| INT-087 | API-key-only sign-in for Subsonic apps | Apps connect with a revocable key, not your password | OpenSubsonic API-key extension; classic Subsonic sends a password-derived token | Medium | R2 | Uses per-app keys (INT-024). Token-and-salt sign-in with the account credential is refused Which Subsonic clients support OpenSubsonic API-key sign-in is unverified; if few do, most apps can connect only through INT-088, which is Later and off by default (open decision in the feature map README). | Key validation | Account > Apps and tokens |
| INT-088 | Legacy Subsonic apps through per-app passwords | Older apps without API-key support still work | Navidrome and others use the account password, stored reversibly | Medium: which apps support API keys is unverified | Later | Optional (see open decisions). Each legacy app gets a random password that works only through the adapter, so a leak exposes one app Later and off by default: legacy clients send a token and salt derived from the password in the URL, which ACC-123 forbids for native routes, so enabling it is a separate owner decision. | Reversible per-app secret store | Legacy switch in Account > Apps and tokens |
| INT-089 | Honest extension discovery | Apps see only the features the server really has | Navidrome yes (OpenSubsonic) | Low | R2 | The extension list is generated from what is enabled and implemented, and checked by the conformance suite | Extension registry | None |
| INT-090 | OpenSubsonic lyrics, playback report, index-based queue and transcoding | Synced lyrics, accurate play reporting, queue hand-off and Opus streams in third-party apps | Navidrome yes | Medium | R2 | Lyrics, playback reporting and the index-based queue ship with the adapter in R2, from the same parsers and queue as Gunmetal's own player. The transcoding extension is served from the Opus path of MUS-106 (R2, sandboxed encoder), and INT-089 advertises it only once that path exists. | Extension handlers | None |
| INT-091 | OpenSubsonic sonic similarity | "More like this" in third-party apps | Navidrome via plugins | Low | Later | Served from scan-time audio features, if the music map adopts them | Similarity index | None |
| INT-092 | OpenSubsonic podcasts and bookmarks | Podcasts and audiobook bookmarks in Subsonic apps | Defined by OpenSubsonic (eight podcast endpoints) | Low | Later | Arrives together with podcasts and audiobooks | Endpoints | None |
| INT-093 | Adapter plays go into the same log | Plays from Symfonium or Finamp count like plays in Gunmetal's apps | Navidrome `scrobble` endpoint; Jellyfin client reporting | Medium | R2 | Adapter plays are written to the append-only log with device and app, so scrobblers, stats and history all see them | Log writer | History shows which app was used |
| INT-094 | Adapter guardrails | Compatibility does not reopen old security holes | Navidrome had Subsonic sign-in bypasses (2023, 2025) and an unthrottled login (CVSS 7.4) | Medium: advisory history | R2 | Off by default and rate-limited, behind the same authorisation layer, with cross-user tests on every adapter route. No endpoint that is unauthenticated in Jellyfin is copied | Route policies; tests | Switches in Admin > Compatibility |
| INT-095 | Client certification and quirk notes | You know each supported app works | Navidrome documents workarounds per client | Medium | R2 | Contract tests replay request and response traces recorded from each certified app version. Open-source clients (for example Finamp, Feishin and Music Assistant) also run end to end in CI where they can run headless. Closed or paid apps such as Symfonium, Amperfy and play:Sub cannot be pinned and driven in CI (unverified per app), so they are certified by a manual script per release. A public matrix lists the certified versions. | Test rigs | Docs site; Admin > Compatibility |
| INT-096 | Connected-app view | See which apps connect through which adapter, with version and last use | Not documented for rivals | Low | R2 | One list across both adapters, tied to per-app keys | App registry | Admin > Compatibility; Account > Apps and tokens |
| INT-097 | Music Assistant through OpenSubsonic | Whole-home audio from your library via Home Assistant, including Sonos, AirPlay and Cast | Music Assistant supports Plex, Jellyfin and Emby | Medium: Music Assistant 3,129 stars | R2 | Comes free with INT-086, because Music Assistant already supports Subsonic servers and includes a scrobbler module | None beyond the adapter | Connection recipe |
| INT-098 | Jellyfin adapter, music subset | Finamp, Jellify and Feishin connect | Jellyfin native; Navidrome 0.64.0 experimental, off by default | Medium | R2 | The adapter pins and states a target Jellyfin API version and maps Quick Connect onto Gunmetal pairing. Its tokens are scoped and expire, whereas Navidrome's never expire | Adapter routes; IDs packed into Jellyfin's GUID format | Admin > Compatibility |
| INT-099 | Jellyfin adapter, video subset | Roku, Kodi add-ons and Jellyfin video apps connect | Jellyfin native; nobody else | High: Roku coverage depends on it (record 1) | Later | Built on the remux path and signed URLs underneath (timing is an open decision) | Adapter routes; playback-info mapping | Admin > Compatibility |
| INT-100 | Jellyfin adapter, tool subset | Tools built for Jellyfin (Seerr, Jellystat, Tracearr, Maintainerr, Bazarr, Home Assistant) work unchanged | Jellyfin native | Medium | Later | Each tool's calls map to a named scope, never to admin rights | Session and user-list routes under scopes | Connection recipes |
| INT-101 | Plex API compatibility | Plex apps connect to Gunmetal | Plex native only | Medium: many people own Plex devices | No | Not doing: Plex's apps sign in through plex.tv accounts, and Plex applies its remote paywall to third-party API clients, so emulating Plex would mean depending on Plex's cloud | None | None |

### Music listening services

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| INT-102 | Last.fm scrobbling, per user | Plays are counted on Last.fm | Plexamp built in; Jellyfin third-party plugin (original archived); Navidrome built in, per user | Medium: Jellyfin request 27 votes | R2 | Parity with Navidrome on per-user scrobbling, and probably on retrying after an outage (unverified); Subsonic clients such as Symfonium also submit offline plays with timestamps (unverified). The edge is that native, adapter and imported plays all pass through one log cursor (INT-064), and private sessions (ACC-117) never reach it. R1 has no scrobbling: R1 records every play in the log with its real timestamp, and the plugin submits what the services still accept when it is enabled in R2 (Last.fm refuses plays older than its cut-off; the exact window is unverified). Owns Last.fm scrobbling; MUS-192 points here. | First-party plugin; per-user session key | Account > Connected services |
| INT-103 | ListenBrainz scrobbling, per user | Plays are counted on the open MusicBrainz service | Plex no (requested since 2018); Jellyfin plugin; Navidrome built in | Medium: 28 votes (Jellyfin), 23 (Plex) | R2 | As INT-102, with MusicBrainz IDs from tags in every submission Owns ListenBrainz scrobbling; MUS-193 points here. | First-party plugin; per-user token | Account > Connected services |
| INT-104 | Now-playing updates | Friends see what you are playing right now | Jellyfin ListenBrainz plugin; Navidrome | Low | R2 | Sent from the live session and suppressed in private listening | Session hook | Switch in Connected services |
| INT-105 | Scrobble filters and private listening | The kids' profile or one library is never scrobbled; any session can be kept private | Navidrome per-user filters (0.64); others (unverified) | Low | R2 | Filters work by profile, library and device. Private mode keeps a session out of history and out of every plugin's cursor Owns scrobble rules; MUS-195 points here. | Filter rules in the log reader | Account > Connected services; private-listening switch in the player |
| INT-106 | Loved-track sync | Hearts match between Gunmetal and the service | Jellyfin ListenBrainz plugin both ways; Navidrome Last.fm loves not planned; Plex requested | Low | R2 | Two-way sync keyed on MusicBrainz IDs, with conflicts settled by event time from the log | Pull job; merge rule | Account > Connected services |
| INT-107 | Import listening history from export files | See ADM-042, which owns this feature. Integration specifics: unmatched items go to a queue, and imported events can be removed as a batch. | Plex no; Jellyfin no; Navidrome requested | Medium: Navidrome request 28 upvotes; Plex requests | R1 | See ADM-042. | None beyond ADM-042. | Account > Data > Import |
| INT-108 | Import listening history from service APIs | Pull history without downloading files first | None | Low | R2 | A plugin with a network grant, using INT-076 | Plugin | Account > Data > Import |
| INT-109 | Service playlists in the library | ListenBrainz daily and weekly mixes appear as playlists | Navidrome plugin; Jellyfin ListenBrainz plugin; Plex requested (7 votes) | Low | R2 | Matched to your library by MBID, with unmatched tracks shown rather than silently dropped | Plugin; playlist write | Playlists |
| INT-110 | Self-hosted scrobble targets | Send plays to Maloja or a similar server you run | Navidrome supports Maloja | Low | R2 | The same plugin with a host the user supplies; reaching a LAN host needs the admin's approval | Plugin | Account > Connected services |

### Watch-state trackers and sync

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| INT-111 | Trakt scrobbling and history sync | Plays are logged on Trakt and history stays in step | Plex via PlexTraktSync and Trakt's own sync; Jellyfin official plugin (imports capped at 100 items, duplicates bug) | High: Plex "full Trakt integration" 376 votes | R2 | Delivered from the log cursor, so history is never duplicated or cut short. Accounts are per user, and Trakt is a mirror of your history, not the record | First-party plugin; per-user OAuth | Account > Connected services |
| INT-112 | Simkl | A cheaper or free alternative tracker | Jellyfin plugin; PlexTraktSync users asked for it | Low | Later | Parity | Plugin on the cursor model | Account > Connected services |
| INT-113 | Letterboxd diary | Films logged to Letterboxd | No official integration (unverified); Jellyfin requested | Medium: Jellyfin request 75 votes | Later | Depends on API access, which is not openly available (unverified) | Plugin | Account > Connected services |
| INT-114 | Anime trackers (AniList, MyAnimeList, Kitsu) | Anime progress kept in sync | Third-party tools; Jellyfin requested | Low: Jellyfin request 14 votes | Later | Parity | Plugins on the cursor model | Account > Connected services |
| INT-115 | Ratings and watchlist sync | Ratings and watchlists follow you to trackers | PlexTraktSync; CrossWatch across many trackers | Low | Later | Ratings and watchlist entries are log events, so they sync the same way plays do | Plugin | Account > Connected services |
| INT-116 | Mirror mode with Plex or Jellyfin | Run both servers during a move and keep progress in step | WatchState (1,571 stars), JellyPlex-Watched and CrossWatch do this between Plex, Jellyfin and Emby | Medium: popularity of these tools | R2 | Pulls from the other server on a schedule and writes the result to the log, tagged with its source. It writes back to the other server only if asked | Importer in pull mode (operations map); matching by external IDs | Admin > Migration |
| INT-117 | History replication between Gunmetal servers | One history across your home and holiday-home servers | Plex account sync (excludes music and managed users); Jellyfin requested | Low: Jellyfin request 105 votes | Later | Append-only logs merge by event time over iroh, with no central account | Replication protocol; merge rules | Account > Servers |

### Download automation, subtitles and requests

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| INT-118 | Lidarr refresh through its Subsonic connector | New albums appear as soon as Lidarr imports them | Lidarr has Plex and Subsonic connectors | Medium | R2 | Comes free with the OpenSubsonic adapter's `startScan`, limited by a refresh-only key to the music root | `startScan` mapped to a targeted scan Depends on INT-086; ships when the adapter does. | Connection recipe |
| INT-119 | Sonarr and Radarr refresh through a Jellyfin-style endpoint | New episodes and films appear within seconds | Plex yes; Jellyfin yes, but 12.0 broke it until a Sonarr fix on 2026-07-27; Emby yes | High | R2 | Implements only the library-update endpoint that Sonarr's "MediaBrowser" connector calls, under a refresh-only token, so the full Jellyfin adapter is not needed | Compatibility route; path mapping | Connection recipe |
| INT-120 | Upgrade-aware imports | A better copy keeps watch state and position and is not announced as new | Jellyfin lost watch state on replace (#15001); Plex (unverified) | Medium: 62 votes on upgrades in Recently Added | R2 | Identity from fingerprints and external IDs survives replacement, and history points at the identity, not the path Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Identity matcher (library map) | Recently added stays clean; item history |
| INT-121 | Bazarr refresh after a subtitle download | New subtitles appear immediately | Bazarr supports Plex; Jellyfin since April 2026; not Emby | Medium | R2 | Refresh by external ID or path under a refresh-only token | Refresh by IMDb ID | Connection recipe |
| INT-122 | Live subtitle pickup | A subtitle Bazarr has just downloaded can be turned on without stopping the film | Nobody; Bazarr's docs say to stop and resume | Low | R2 | An item-changed event reaches the active session, and Gunmetal's own player loads the new sidecar | Session notification | Player subtitle menu marks the new track |
| INT-123 | Subtitle search when playback starts | Missing subtitles are fetched the moment you press play | Plex plus Bazarr (needs Plex Pass); Jellyfin and Emby no | Low | Later | Free playback-start webhooks with a payload preset for Bazarr; whether Bazarr can accept it is unverified | Payload preset | Connection recipe |
| INT-124 | Seerr availability | Seerr knows what you already have, and in which quality | Seerr scans Plex, Jellyfin and Emby | High: Seerr 12,770 stars | R2 | A read-only "library and availability" scope with external IDs and quality per version, used by a native Seerr connector (see open decisions) | Availability queries | Connection recipe |
| INT-125 | Watchlist feeds requests | Adding a title to your watchlist files a request | Plex yes, via Seerr; Jellyfin has no watchlist yet | Medium: Jellyfin watchlist 1,294 votes | R2 | Gunmetal's own watchlist (discovery map) is readable by Seerr under a read scope | Watchlist read route | Watchlist |
| INT-126 | Request from the title page | Request a missing title, or open it in Seerr, from Gunmetal's apps | Streamyfin, Moonfin and Plezy integrate Seerr; no server does | Medium | R2 | The admin sets the Seerr address once, and clients deep-link using the item's external ID | Setting; deep-link template | "Request" or "Open in Seerr" on title pages |
| INT-127 | Report a problem from the player | Flag broken audio or bad subtitles | Through Seerr for all three | Low | Later | The report carries the exact file, track and timestamp from the player, and goes to Seerr or stays in Gunmetal | Issue records | "Report a problem" in the player menu |
| INT-128 | Music requests through Lidarr | Request an album you do not have | Nobody mainstream; Seerr's music pull request closed in Nov 2025 | Medium: Seerr issue 96, 134 reactions | Later | A plugin links MusicBrainz search to Lidarr and holds its own scoped Lidarr key | Plugin | "Request album" on artist pages |
| INT-129 | Removal requests and cleanup | Ask for something to be deleted; old unwatched titles are cleaned up | None built in; Maintainerr (2,315 stars) and Janitorr | Medium: Seerr issue 308, 93 reactions | Later | Last-played times and play counts are already in the API (INT-133), so tools such as Maintainerr have what they need | None beyond INT-133 | "Request removal" on title pages |
| INT-130 | Upcoming episodes from Sonarr | See episodes due soon for shows you follow | Jellyfin declined pulling Sonarr's schedule; community plugin rows | Low | Later | A plugin feeds a labelled home row | Plugin | Home row |
| INT-131 | Built-in request manager | Requests handled inside the media server | None of the three; Jellyfin declined | Low: Jellyfin request 24 votes, declined | No | Not doing: Seerr is good and popular, so Gunmetal integrates with it instead of competing | None | None |
| INT-132 | Built-in download automation | Searching and downloading built into the server | None of the three | Low: no request found in the research | No | Not doing: not the server's job, and it would add legal exposure | None | None |

### Monitoring and statistics tools

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| INT-133 | Read-history scope for stats tools | Stats tools read plays, sessions and last-played times | Plex via its API (Tautulli); Jellyfin via the Playback Reporting plugin; Tracearr across all three | Medium: Tautulli 6,598 stars; Tracearr 2,680 | R2 | One read-only scope covers the log and live sessions. With the change feed and the event stream, tools never need to poll Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Scope; log queries | Connection recipe |
| INT-134 | Playback decision in the API | Tools see direct play, remux or transcode, and the reason | Shown by Tautulli, Streamystats and Tracearr | Medium | R1 | The decision engine in the core gives a structured reason for every session: Opus streams in R1, remux and transcode in R2 | Reason codes in sessions and events | Now playing (operations map) |
| INT-135 | Prometheus metrics | See ADM-127, which owns this feature. Integration specifics: direct-play, remux and transcode counters that test the server's main claim. | Jellyfin exporter since 2020; Navidrome opt-in; Plex none (unverified) | Medium: Jellyfin request 56 votes | R2 | See ADM-127. | None beyond ADM-127. | Switch in Admin > Integrations |
| INT-136 | Session control API | Automations can stop a stream, with a message | Jellyfin requested (open); Tracearr automations | High: Jellyfin request 402 votes | R2 | The API side of ADM-102: stopping a session revokes it and cuts in-flight responses (ACC-122). | Session control route | Now playing (operations map) |
| INT-137 | Remote control of Gunmetal apps | Pause, skip or send media to a Gunmetal app from another tool | Jellyfin remote entities in Home Assistant; Plex clients controllable (unverified) | Medium | R2 | Gunmetal's own apps accept commands over their existing connection, scoped per user and device | Command channel | Device picker |

### Collections and library managers

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| INT-138 | Playlist write API | Tools can create and update playlists | Plex and Jellyfin through their APIs | Low | R1 | Playlists are durable user data written through the log, so playlists made by tools survive a rebuild | Playlist routes | Playlists |
| INT-139 | Collection write API | Kometa-style tools can build collections | Kometa fully supports Plex only | Medium: Kometa's Jellyfin and Emby request has 33 reactions | R2 | Collections get the same durable treatment as playlists | Collection routes | Collections |
| INT-140 | Collections and smart playlists as a file | Curation lives in a text file you can version, and survives any rebuild | Kometa keeps customisations outside the server database | Medium | Later | The server reads a documented declarative format and evaluates its rules against the library | File watcher; rule engine | "Export as file" in the collection editor |
| INT-141 | Artwork upload API | Poster tools such as Posterizarr can set artwork | Posterizarr supports all three | Low: Posterizarr 931 stars | R2 | Uploads are stored under content-hash names and decoded with size limits in the sandbox | Upload handler | Artwork picker on title pages |
| INT-142 | Kometa support | Kometa runs against Gunmetal | Kometa supports Plex; support for other servers is a work list on a branch | Medium | Later | Contribute a connector for the native API rather than emulate Plex | Native API only | None |
| INT-143 | Poster overlays baked into images | 4K, HDR and audio badges drawn onto poster files | Kometa rewrites posters; Emby overlays (Premiere) | Medium: Jellyfin 4K marker request 146 votes | No | Not doing: Gunmetal's clients draw badges from scan data (library map), so artwork files stay clean | None | None |

### Smart home, voice and system hooks

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| INT-144 | Home Assistant integration | Each Gunmetal app session is a media player in Home Assistant, with library browsing, search and sensors | Plex core integration; Jellyfin core integration; Emby exists (scope unverified) | High: Jellyfin request 355 votes, now complete | Later | Parity | Integration maintained by the project, built on scoped tokens, the event stream and session control | Home Assistant |
| INT-145 | Play by search | "Play the latest episode of X in the lounge" | Plex JSON search with filters | Low | Later | Parity | Search route with typed filters | Home Assistant |
| INT-146 | Voice-assistant skills | See CLI-131, which owns this feature. | Plex shut its Alexa skill on 2026-06-15; Emby Alexa (Premiere); Jellyfin requested | High: Plex Google Home 2,461 votes; Jellyfin 243 | No | See CLI-131. | None beyond CLI-131. | None |
| INT-147 | Stable deep links | Links to an album, artist or film open in the app | Infuse TMDB deep links; others (unverified) | Low | R1 | The web URLs are the deep links, and the native apps register the same paths in R2 The API side of CLI-034, which owns deep links. | Stable routes | Share menus |
| INT-148 | Operating-system automation hooks | Siri Shortcuts, Android intents and NFC tags start playback | Plexamp Siri and NFC; Finamp Siri | Low | R2 | The native apps expose them (clients map), using the same command channel as INT-137 | None beyond INT-137 | System settings; share sheet |

### Social presence

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| INT-149 | Discord presence from the desktop app | Friends on Discord see what you are playing | No server does it natively; desktop bridges need a server token and a second app | Medium: Plex request 100 votes since 2018 | Later | The desktop app talks to the local Discord app, with no token on the server and cover art only from Cover Art Archive. Opt-in, per library, and private listening is honoured | None (desktop client, clients map) | Account > Connected services; player share menu |
| INT-150 | Server-side presence with user tokens | Presence with no desktop app | Navidrome plugin stores the user's Discord token, which its own docs say may break Discord's terms | Medium: presence is wanted, but not this way | No | Not doing: storing user tokens breaks Discord's terms | None | None |

### Data portability

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| INT-151 | Documented history export | Your plays, ratings and playlists as files you own | Plex no; Jellyfin no; Emby (unverified); WatchState portable backups | Medium: Jellyfin export request 17 votes; history view 830 | R1 | The log's format is versioned and documented. Exports come in its native form and also as ListenBrainz listen JSON and CSV | Export job | Account > Data > Export |
| INT-152 | Trakt history import | Bring your Trakt history over | PlexTraktSync; Jellyfin plugin imports only the newest 100 items | Medium | R2 | Reads Trakt's export or its API and removes duplicates against the log | Importer | Account > Data > Import |
| INT-153 | Streaming playlist import | Bring Spotify playlists over, matched to your library | Jellyfin requested (29 votes; Plex, Trakt or IMDb import 31) | Low | Later | Parity | Plugin or file import; matcher | Playlists > Import |

### Other protocols and ecosystems

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| INT-154 | DLNA media server plugin | Older TVs and receivers can browse the library | Plex is a DLNA server; Jellyfin moved DLNA to a plugin | Low | Later | A plugin, off by default, with a LAN-only grant and read-only access to chosen libraries, because DLNA itself has no authentication | Plugin with LAN grant | Admin > Plugins |
| INT-155 | OPDS catalogue and KOReader sync | E-reader apps browse, download and sync position | Komga and Kavita yes; Audiobookshelf requested | Low: Audiobookshelf OPDS request 73 upvotes | Later | Arrives with books. Positions are kept as locators, so they are not cut back to the start of a chapter | Protocol routes | Books (adjacent media map) |
| INT-156 | Podcast sync protocols | AntennaPod-style apps sync with Gunmetal | gPodder sync and the Open Podcast API exist; rival support (unverified) | Low | Later | Parity | Protocol routes | Podcasts (adjacent media map) |
| INT-157 | Live TV lineup export | Keep using TiviMate or Kodi during a move, with your curated channels | Dispatcharr gives each profile its own M3U, guide and HDHomeRun link | Medium | R3 | M3U, XMLTV and an HDHomeRun-style device, with authenticated per-device URLs (live TV map) | Export routes | Live TV > Export |
| INT-158 | Plex-style HTTP metadata providers | Reuse community providers written for Plex's new model | Plex custom providers (beta, Dec 2025) | Low | Later | An adapter in the provider host, if the provider API's terms allow it (unverified) | Provider adapter | Admin > Library > Providers |
| INT-159 | Immich photos on TV | Immich albums and memories show on Gunmetal TV apps | Not offered by rivals | Low | Later | A plugin with a grant for one Immich host and the user's own Immich key | Plugin | Photos (adjacent media map) |

## Differentiators

1. **Scoped, expiring tokens (ACC-049, INT-018 to INT-023 in R1; connection
   recipes, INT-025, in R2).**
   Every helper tool connected to Plex, Jellyfin or Emby holds what amounts
   to a master key. In 2026 that turned Huntarr's exposure into a leak of
   every connected *arr key. Tautulli also shipped more than a dozen security
   fixes that year. With Gunmetal, the Sonarr token can only refresh one
   folder, the stats token can only read history, and every token expires,
   shows its last use and can be revoked in one click. Connection recipes
   make the safe setup easier than the unsafe one. Security-minded
   self-hosters have watched these breaches, so this is a reason to switch
   that needs no other feature to land first.
2. **OpenSubsonic with API keys only (INT-086 to INT-097; R2).** Users stay
   for their apps, and Navidrome added a Jellyfin API for exactly that
   reason. One adapter gives Gunmetal Symfonium, Feishin, Amperfy and other
   mature apps, along with their offline, car and watch features. It also
   brings Music Assistant (and through it Sonos and Home Assistant) and
   Lidarr's refresh connector, and it is the interim route for iPhone users
   while the Apple builds wait. It ships in R2, not R1, to keep the first
   release's attack surface small; no app ever holds the user's password,
   and which apps support API-key sign-in is unverified (INT-087).
3. **Free, signed and debuggable webhooks, plus a pushed event stream
   (INT-030 to INT-048; the outbox table in R1, delivery in R2).** Plex
   charges for webhooks, and Jellyfin's are free through a plugin, so on
   price this is parity with Jellyfin. Emby's help article
   says they need Premiere. Jellyfin's plugin lacks the test button its users
   ask for most, and polling tools have starved its thread pool. Gunmetal
   sends events from a transactional outbox, signs them with Standard
   Webhooks, retries them, logs every delivery, groups "album added" after
   metadata is ready, and never evaluates template code.
4. **Sandboxed plugins whose interface survives upgrades (INT-054 to
   INT-066; R2).** Jellyfin 12.0 told users to remove every third-party plugin
   before upgrading. Several integrations people relied on are now archived
   volunteer projects. Gunmetal runs plugins in WebAssembly with declared
   grants, checks network access on the resolved address, keeps per-user
   secrets the admin never sees, uses a versioned interface, and maintains
   the important plugins in its own repository.
5. **The log is the record, and trackers are mirrors (INT-064, INT-102 to
   INT-111 in R2; INT-151 in R1).** Navidrome already does per-user
   scrobbling well, so the edge is the single cursor, not scrobbling itself. Trakt now charges for things that used to be free,
   restricts its API, and caps imports. Scrobbles go missing when a service
   is down or a phone is offline. In Gunmetal, every scrobbler and tracker
   reads the append-only log through its own cursor, so nothing is lost or
   doubled, and the history can be exported in a documented format.
   Gunmetal works as a self-hosted tracker on its own.
6. **A first-class citizen of the *arr stack (INT-011 in R1; INT-118 to
   INT-122 in R2).** Refreshes are scoped to a single folder. Upgraded files keep
   their watch state and are not announced as new, which Jellyfin users have
   asked for. A subtitle Bazarr has just fetched can be switched on without
   stopping the film. Gunmetal can do that last part because it owns the
   player.

## Deliberately not doing

- **In-process native plugins (INT-073).** They would bypass every grant the
  sandbox enforces.
- **Plugins that inject scripts into the web client (INT-074).** Script
  injection is an XSS hole. Declarative home rows and themes meet the same
  demand.
- **Plex API compatibility (INT-101).** Plex's apps sign in through plex.tv,
  and Plex applies its remote paywall to third-party API clients.
- **A built-in request manager (INT-131).** Seerr does this well. Gunmetal
  aims to be its best backend.
- **Built-in download automation (INT-132).** It is not the media server's
  job, and it adds legal exposure.
- **Poster overlays baked into image files (INT-143).** Clients draw badges
  from scan data instead, so artwork stays clean.
- **Voice-assistant skills (INT-146).** They need a public cloud endpoint.
  Home Assistant's voice features are the path instead.
- **Server-side Discord presence using user tokens (INT-150).** It breaks
  Discord's terms.
- **Credentials in query strings on the native API (INT-023).** They leak
  into logs. Signed, short-lived URLs cover the cases where a header is
  impossible.
- **Templates that can evaluate code (INT-042).** Tautulli's 2026
  remote-code-execution bugs came from exactly this.
- **All-powerful keys as the default way to connect a tool.** An admin scope
  can exist for the owner's own scripts, but recipes never hand it out, and
  it expires like every other token.

## Dependencies and risks

- **Blocking: ADR 3, durable user state.** ADR 1 (decision 5) names watch
  history as the only irreplaceable data, but most rows in this map that
  write user data (playlists, loves, ratings, corrections, layouts, shares,
  identities, IDs, settings) need it to survive a cache rebuild. ADR 3 must
  be accepted before any server work that stores user data. It defines the
  two durable stores every map now names the same way: **the user log** and
  **the identity store** (see the [feature map README](README.md)).
- **A user log for integration state needs an architecture record.**
  Record 1 names watch history as the only irreplaceable data. Tokens,
  webhook configurations and secrets, plugin grants and cursors, per-user
  plugin secrets, follows and the public ID map (INT-008) cannot be rebuilt
  from files either. The setup and security research reach the same
  conclusion for users, device keys and curation. This record should be
  written before the server's data model.
- **Other feature maps own parts of this one.** The users and security map
  owns passkeys, OIDC, device keys, pairing and the authorisation layer that
  scopes plug into. The setup and operations map owns the dashboard, the
  importers from Plex and Jellyfin, backups and the update feed. The library
  map owns content identity, fingerprints, external IDs and providers. The
  music map owns the scrobble rules, lyrics and sonic similarity. The
  clients map owns deep links, OS hooks and the desktop shell. The discovery
  map owns the watchlist, history view and home rows. The live TV map owns
  lineup export and guide data.
- **The plugin runtime is a large project.** A host with a stable
  interface, permission checks, resource limits, signing and SDKs is months
  of work, and the ecosystem starts empty. Navidrome needed two security
  fixes to its network guard after launch, which shows how easy the details
  are to get wrong. The first-party plugins have to carry the system at
  launch.
- **The adapters double the authorisation surface.** Navidrome's 2023 to
  2026 advisories include Subsonic sign-in bypasses and an unthrottled
  login. Every adapter route needs the same cross-user and wrong-scope tests
  as the native API.
- **The Jellyfin API is big, loosely specified and still changing.** 12.0
  changed what `GetItems` returns and removed routes and auth methods. Real
  clients rely on quirks the spec does not describe, and copying Jellyfin
  too closely risks copying its unauthenticated endpoints.
- **Some Subsonic apps may not support API keys.** Which ones do was not
  verified. If key-only sign-in is enforced strictly, those apps are locked
  out (INT-088).
- **Third-party terms can change underneath us.** Trakt now requires VIP to
  create API apps. Last.fm and Trakt client secrets cannot be kept secret
  inside an open-source binary. Discord forbids using user tokens.
  Letterboxd's API is not openly available (unverified). TMDB limits caching
  and commercial use. Trakt changed its rules twice in two years.
- **Push and voice need vendor clouds.** iOS push goes through Apple, and
  voice skills need a public endpoint. Both sit uneasily with "no central
  account".
- **Webhooks and plugins need different network rules.** Admin webhooks
  legitimately reach LAN services, while plugin traffic to the LAN must be
  blocked by default. Both rules must be clear in the UI (INT-049, INT-056).
- **Stable IDs depend on identity working.** If the fingerprint or
  external-ID match is wrong, a tool's stored reference silently moves to
  the wrong item.
- **The event stream and outbox cost resources on small hardware.** Many
  SSE consumers, plus a delivery log, add memory and disk use. Retention and
  fan-out limits must be measured on the low-end target.
- **Filesystem watching is unreliable on network shares.** The refresh
  endpoints and connectors (INT-011, INT-118, INT-119) are the dependable
  path and must keep working across releases.
- **Built-in features compete with popular tools.** Tautulli, Seerr, Kometa
  and Maintainerr have large communities, and users will run them alongside
  Gunmetal during any move. Integration has to come before replacement.
- **Scope creep.** This area alone has 159 rows. The order in which they
  arrive has to be decided explicitly, release by release.
- **R1 is heavy in this area.** Most R1 rows here follow cheaply from three
  pieces: the scope model, the outbox and the log cursor. Even so, the plan
  should mark which R1 rows are must-have (scopes, refresh, webhooks with
  signing and retries, the OpenSubsonic core, scrobbling, history export)
  and which can slip to R2 without hurting the release.

## Open decisions for the project owner

1. **Should the OpenSubsonic adapter ship in R1?** The README roadmap places
   the adapters late. R1 has only a web client, so the adapter is how phone
   users would get offline playback, cars and watches on day one.
   *This map recommended yes. The feature map README decides R2 for now*
   (see its open decisions): R1 already carries the sign-in, sync and
   authorisation work, the adapter doubles the authorisation surface, and
   few Subsonic apps are known to support API-key sign-in (unverified). If
   the owner overrides this, ship the core subset (INT-086, INT-087,
   INT-089, the non-transcoding part of INT-090, INT-093 to INT-097) off by
   default with API keys only, and move ACC-129, ACC-130, MUS-207 and
   CLI-130 with it.
2. **Should a minimal plugin host ship in R1, so that Last.fm and
   ListenBrainz scrobbling arrive with the music release?** The alternative
   is an R1 without scrobbling. Building scrobblers into the server would
   contradict record 2. *This map recommended an R1 host; the feature map
   README decides R2:* a wasmtime host with grants, signing, an egress gate
   and per-user secrets is a large R1 item. R1 records every play in the log
   with its real timestamp, so the R2 scrobblers can submit what the
   services still accept.
3. **What happens to Subsonic apps without API-key support?** The adapter
   accepts API keys only (INT-087) when it ships. *Recommendation:* keep
   per-app generated passwords (INT-088) at Later as a separate owner
   decision: off by default, opt-in per user and per app, with a clear
   warning, working only through the adapter, because legacy clients send a
   password-derived token in the URL.
4. **What licence terms apply to third-party WebAssembly plugins under the
   AGPL?** There is no CLA, so any plugin exception has to be added before
   outside contributions build up. Adding it later would need every
   contributor's agreement. *Recommendation:* get legal advice now. If
   permissive third-party plugins are wanted, add an explicit plugin
   exception before R1. Keep first-party plugins AGPL.
5. **Should the project hold API keys for Trakt, Last.fm and TMDB, or should
   users bring their own?** *Recommendation:* register project apps where
   the terms allow per-user authorisation (Last.fm, ListenBrainz), accepting
   that a secret inside an open binary is not secret. For Trakt, ask users
   to bring their own client ID, with file import and export as the fallback
   if Trakt revokes access. The TMDB question belongs to the library map but
   should be decided at the same time.
6. **When should the Jellyfin adapter cover video?** *Recommendation:* ship
   the music subset in R2 (INT-098). Keep the video subset (INT-099) as
   Later, unless Roku is declared a launch platform. If it is, video moves
   into R2 and takes priority over the tool subset.
7. **How should Seerr integrate?** The two options are a native Seerr
   connector contributed upstream, or a Jellyfin-adapter subset that Seerr
   already understands. The second option also needs passwordless sign-in.
   *Recommendation:* contribute an upstream connector that uses a read-only
   availability scope, and offer Seerr users the consent flow (INT-028).
   Fall back to the adapter subset only if upstream declines.
8. **Should the project run a relay for native push notifications?**
   *Recommendation:* not for R2. Use the in-app inbox plus the user's own
   ntfy topic (INT-050), and revisit only if users clearly need lock-screen
   alerts.
9. **Should voice-assistant skills stay out?** *Recommendation:* confirm
   No (INT-146), and point users to Home Assistant.
10. **What API stability promise starts at R1?** *Recommendation:* commit to
    API version 1 at R1 as a version prefix, with at least one major
    release of deprecation notice; the deprecation machinery (INT-002) and
    the conformance suite (INT-004) arrive in R2. Any endpoint not ready for that promise should carry an
    explicit "experimental" marker in the spec.
11. **Should mirror mode accept events pushed from Plex during a move?**
    Inbound Plex webhooks need the owner's Plex Pass. *Recommendation:* pull
    on a schedule (INT-116), which works without a Plex Pass. Treat inbound
    webhooks as an optional extra.
12. **Should there be a remote MCP endpoint for AI agents?**
    *Recommendation:* Later (INT-016). Make it read-only, off by default,
    and built on scoped tokens so it adds no new access paths.
