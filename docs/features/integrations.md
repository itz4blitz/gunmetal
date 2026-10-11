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
The last column, Security, lists the requirements in the
[security baseline](../security/README.md) that a builder must satisfy for
that row; where a row and the baseline disagree, the baseline wins and the
row is a bug. "Security notes" at the end names the threats this area
answers.
Releases (R1, R1.1, R1.2, R1.3, R2, R3, Later, No), the Demand scale, row
ownership and the terms "the user log" and "the identity store" are defined
once in the [feature map README](README.md); the point releases R1.1 to
R1.3 are those the owner adopted in
[D-10](../decisions.md#d-10-r1-scope-and-the-release-table). A row whose
Release cell would differ between maps names one owning row; the other maps
point at it.

### Native API and developer surface

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| INT-001 | API reference generated from code | Developers can build tools without reverse-engineering the server | Plex yes (published 2025); Jellyfin yes (OpenAPI); Emby yes (depth unverified) | Medium: Jellyfin keeps three SDKs on its spec; Plex published one in 2025 | R1 | Parity with Jellyfin's generated OpenAPI spec; the edge is the conformance suite (INT-004) and the CI check that every route is in the spec. The spec is generated from the same Rust protocol types that the server and clients compile, so it cannot drift from what the server does; CI fails if any route is missing from it | Spec generation in the build; the server serves it only to signed-in users, because it names every route and the version (SEC-API-005); the docs site publishes each release's spec | API docs page served by the server to signed-in users only; docs site | SEC-API-091, SEC-API-092, SEC-API-002, SEC-API-005, SEC-TM-005 |
| INT-002 | Versioned API with a deprecation window | Integrations keep working across upgrades | Jellyfin policy new in 12.0, yet 12.0 still broke Sonarr's connector and old clients; Plex no published policy (unverified) | High: Sonarr issue 8805; Jellyfin 12.0 broke old clients | R2 | Semantic versioning. Deprecated routes stay for at least one major release and are flagged in a response header, and the conformance suite (INT-004) pins their behaviour R1 ships a version prefix on every route; the deprecation window and its machinery arrive in R2. | Per-version route table; deprecation header; usage counter for deprecated routes | API docs page; Admin > Integrations lists tokens still calling deprecated routes | SEC-API-092, SEC-API-091, SEC-TM-005 |
| INT-003 | Integrator changelog | Developers see exactly what changed in each release | None of the three publishes one; Plex's move to JWTs left developers asking for examples | Medium: Plex JWT forum thread; the Sonarr fix after Jellyfin 12.0 | R2 | Generated from a diff of the API spec between releases, so no change goes unlisted | Spec-diff step in the release pipeline | Docs site; release notes | SEC-API-091, SEC-API-092, SEC-OPS-052 |
| INT-004 | Published conformance suite | Tool and adapter authors can test against the contract | None of the three | Low: no direct request; follows from API churn | R2 | The same suite gates Gunmetal's own CI under the 100% coverage and zero-mutant rules | Recorded request and response fixtures; runnable harness | Docs site | SEC-STD-038, SEC-API-091, SEC-EXT-053, SEC-STD-004 |
| INT-005 | Capability discovery | A tool learns the API version, the enabled adapters and extensions, and the scopes it holds | Video servers no; OpenSubsonic `getOpenSubsonicExtensions` is the model | Low | R1 | It reports only what is enabled and what the caller may use. Unauthenticated callers learn nothing, unlike the endpoints listed in Jellyfin issue 5415 | Capability registry | None (API only) | SEC-API-005, SEC-API-002, SEC-API-003, SEC-API-019 |
| INT-006 | Change feed (delta sync) | Tools fetch only what changed since their last visit | Jellyfin only through the Kodi Sync Queue plugin; Plex and Emby (unverified) | Low: "differential API" 6 votes, but every sync tool needs it | R2 | It exposes the same change log that syncs the library to Gunmetal's own devices, with a cursor per consumer and filtering by scope. R2, because its consumers are tools, which reach the API only with keys, and API keys arrive in R2 (security baseline, owner decision 8; D-10). In R1 Gunmetal's own clients sync through the library change feed (LIB-018) | Change-log retention and compaction; cursor per token | Token detail shows feed position | SEC-API-015, SEC-API-025, SEC-PRV-052, SEC-TM-026 |
| INT-007 | Consistent list endpoints | Paging, filtering and caching work the same way everywhere | Not compared in the research | Low | R1 | Cursor pagination, field selection and conditional requests (ETag) are defined once in the protocol types. Authenticated JSON carries `Cache-Control: no-store`, so an ETag only revalidates the client's own synced copy, never a shared cache | Shared query layer | None (API only) | SEC-API-063, SEC-API-025, SEC-API-055, SEC-TM-039 |
| INT-008 | IDs that survive rebuilds and file replacement | References stored in other tools keep working | Jellyfin 10.11 lost watch state on replace or rename (#15001); Plex exposes stable GUIDs | Medium: #15001 had 61 +1; Maintainerr cross-checks matches by year | R1 | Random public IDs are bound to content identity as defined by LIB-028 and kept in the identity store, outside the SQLite cache, so a rebuild reissues the same IDs. | The identity store, included in backups; identity rules from LIB-028 | "Copy ID" on item pages in developer mode | SEC-API-023, SEC-API-024, SEC-PRV-021, SEC-TM-051 |
| INT-009 | MusicBrainz IDs on music items | Scrobblers, Lidarr and other tools match the exact recording, release and artist | Plex matches music to MusicBrainz (forum claim); Jellyfin reads MBIDs; Navidrome uses them | Medium: needed for scrobble linking and loved-track sync | R1 | Read from tags at scan time by the core parsers, with no network lookup | MBID fields in the music model and the API | Album and track info panels | SEC-MED-014, SEC-MED-013, SEC-TM-031, SEC-PRV-007 |
| INT-010 | Video provider IDs on every item | Seerr, Sonarr, Radarr and Maintainerr can match films and episodes | Plex, Jellyfin and Emby all expose them | High: Seerr availability sync and Maintainerr depend on them | R2 | IDs from filenames, NFO files and provider plugins carry their source per field, so a tool can see where an ID came from | Provider ID fields with provenance | Title info panel | SEC-TM-038, SEC-HIS-034, SEC-MED-014, SEC-EXT-028 |
| INT-011 | Path-scoped library refresh | A downloader or tagger says "this folder changed" and the change appears within seconds | Plex yes; Jellyfin yes (takes paths); Emby yes | High: every *arr connector relies on one | R2 | The key holds only the refresh scope (`admin:library`, which only an admin can grant) and is limited to named roots. The tool names a folder; the server matches it against folders it already indexed under those roots and rescans through its own directory handles, so no path from the request is ever opened. A refresh reads only headers and tags with the core parsers, so it stays cheap even on a NAS, and repeated requests join the running job. R2, because API keys arrive in R2 (security baseline, owner decision 8). In R1 the scanner's own change detection is the path | Matching of named folders against indexed folders under the key's roots; debounced, targeted scan job | Connection recipes (INT-025); Admin > Library activity | SEC-EXT-010, SEC-EXT-011, SEC-HIS-015, SEC-TM-043, SEC-API-064, SEC-MED-038 |
| INT-012 | Per-token rate limits | A buggy or greedy tool cannot slow the server for everyone | None documented; Jellystat polled every second and starved Jellyfin | Medium: Jellystat issues 298 and 328 (34 comments) | R2 | Limits apply per key and come with retry headers. The event stream (INT-048) removes the reason to poll. R2, because API keys arrive in R2 (security baseline, owner decision 8). In R1 the same limiter is keyed on the signed-in person | Token buckets; 429 responses with retry-after | Usage and throttling on token detail | SEC-API-057, SEC-TM-068, SEC-NET-053, SEC-STD-030 |
| INT-013 | Health check for uptime monitors | A monitor can see that the server is up | Not compared in the research | Low | R1 | It returns a status and nothing else: no version, user or library data | One minimal route, covered by the route-policy test | None | SEC-API-002, SEC-NET-046, SEC-API-005, SEC-TM-004 |
| INT-014 | Built-in API explorer | Try calls against your own server | Not compared in the research | Low | R2 | Served by the server from its own origin under the web client's Content Security Policy, with no CDN. It sends read-only GET requests with the signed-in session cookie and the `Gunmetal-Request` header; no token is ever minted for page script, so a script bug cannot lift one | Static docs bundle shipped in the signed web-client build | API docs page | SEC-TM-058, SEC-IAM-017, SEC-API-033, SEC-API-044, SEC-API-049 |
| INT-015 | Official SDKs | Typed client libraries for tool authors | Jellyfin yes (TypeScript, Kotlin, Swift); Plex none official; Emby (unverified) | Low | R2 | Parity | SDKs generated from the spec on every release, TypeScript first | Docs site | SEC-SUP-041, SEC-SUP-044, SEC-SUP-052, SEC-CLI-016 |
| INT-016 | Agent access (MCP) | Ask an AI assistant about your library | Plex remote MCP (unverified); Jellyfin community servers (unverified); Streamystats AI chat | Low: emerging | Later | A thin layer over read-only scoped keys, off by default, so an agent can never change anything. It sees only what the key's owner may see, never another person's history, and library text reaches the agent as data. It authenticates with an API key in the `Authorization` header: Gunmetal does not act as an OAuth authorisation server for it (SEC-STD-026) | MCP transport mapped onto read scopes | Admin > Integrations; Account > Apps and tokens | SEC-EXT-010, SEC-EXT-011, SEC-STD-026, SEC-PRV-022, SEC-API-004 |

### Tokens, scopes and access for integrations

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| INT-017 | Scoped tokens | See ACC-049, which owns this feature. Integration specifics: the named scopes map onto the security baseline's vocabulary ("refresh library" is `admin:library`, which only an admin can grant; "read history" is `history:read`, the key owner's own history only; "read library and availability" is `library:read`; "play" is `media:stream`), no key ever holds an administrator, owner-only or host-equivalent scope, and a route-table test fails the build if any route lacks a scope. R2 with API keys (security baseline, owner decision 8); ACC-049 carries the same release. | Plex no (unverified); Jellyfin no (#13992 open); Emby no (unverified); Immich yes | Medium: #13992 23 +1; Huntarr leaked every connected *arr key | R2 | See ACC-049. | None beyond ACC-049. | Admin > API tokens; Account > Apps and tokens | SEC-EXT-008, SEC-EXT-010, SEC-EXT-011, SEC-EXT-012, SEC-API-020, SEC-IAM-073 |
| INT-018 | Scope limits by library, root and user | Seerr sees only the film and TV libraries; Lidarr can refresh only the music folder | None | Medium | R2 | The same object filter that applies user permissions evaluates these limits, and a key's rights are always the intersection of its scopes and its owner's current rights. R2, because API keys arrive in R2 (security baseline, owner decision 8). | Limit checks on queries and refresh paths | Token editor | SEC-EXT-011, SEC-API-020, SEC-API-014, SEC-TM-024 |
| INT-019 | Expiry and rotation | A leaked key stops working on its own | Plex 7-day JWTs plus legacy tokens; Jellyfin long-lived (unverified); Navidrome's Jellyfin tokens never expire | Medium | R2 | Every key expires: 365 days by default (the owner may choose 30 to 730), and a key unused for 180 days is disabled. The owner is told in the app 14 days before either happens. Rotation issues a successor with an overlap window, created in an interactive session with a fresh fingerprint or face check, never by the old key. R2, because API keys arrive in R2 (security baseline, owner decision 8). | Expiry job; rotation route; expiry event | Token list; dashboard banner | SEC-EXT-013, SEC-EXT-012, SEC-EXT-008 |
| INT-020 | Last use and audit trail per token | See which tool did what, and when | None documented; OpenSubsonic's API-key extension requires listing and revoking keys | Medium | R2 | Every call is attributed to a key, and the list shows last use, client and a coarse last-used address. Full addresses stay in the person's own security log and follow the retention schedule. R2, because API keys arrive in R2 (security baseline, owner decision 8). | Audit log with retention | Token detail | SEC-EXT-014, SEC-EXT-017, SEC-PRV-003, SEC-PRV-005, SEC-OPS-027 |
| INT-021 | Revoke one or revoke all | Cut off one tool, or everything after a breach | Plex sign-out-all after its 2025 breach; Jellyfin partial | Medium: the Plex breach response | R2 | Revocation takes effect on the next request, and signed URLs minted under the key stop: new requests fail at once, and an in-flight response is cut by the ACC-122 mechanism. Admins can revoke any user's keys. R2, because API keys arrive in R2 (security baseline, owner decision 8). | Revocation check per request | Token list; Account > Apps and tokens | SEC-EXT-014, SEC-IAM-043, SEC-API-028, SEC-TM-028, SEC-OPS-033 |
| INT-022 | No-escalation rule | No key can create, change, list or reveal any key, including itself, and nobody can make a key wider than their own rights | Immich CVE-2026-23896 let a key raise its own permissions | Medium: a 2026 CVE | R2 | Keys are created and changed only in an interactive session with a fresh fingerprint or face check, never by another key. Every create or update is checked against the creator's current rights, and the checks are mutation-tested. R2, because API keys arrive in R2 (security baseline, owner decision 8). | Policy check on key routes; key-management routes refuse every key | Error in token editor | SEC-EXT-012, SEC-API-020, SEC-IAM-073, SEC-TM-027 |
| INT-023 | Credentials in headers only | Keys never leak into proxy or browser logs | Plex, Jellyfin and Emby all accept tokens in query strings; Bazarr's Plex webhook URL carries its key | Medium: Jellyfin #5415 | R1 | The native API refuses credentials in query strings and rejects the request. Media and images use short-lived capability URLs whose token sits in the path, bound to a session and re-checked on every request. The adapter routes that need query-string keys (R2) are documented exceptions, limited to per-app keys that are removed from the request line before anything is logged | URL signer; refusal path | None (docs) | SEC-API-004, SEC-EXT-006, SEC-API-026, SEC-API-029, SEC-EXT-063, SEC-IAM-047 |
| INT-024 | Per-app credentials for third-party apps | See ACC-129, which owns this feature. | OpenSubsonic API keys; classic Subsonic needs a password equivalent, which Navidrome stores encrypted | Medium | R2 | See ACC-129. | None beyond ACC-129. | Account > Apps and tokens ("Connect a music app", with a QR code) | SEC-EXT-057, SEC-EXT-058, SEC-EXT-007, SEC-API-094, SEC-TM-070 |
| INT-025 | Connection recipes | Pick "Lidarr" or "Symfonium" and get the URL, the header and a token with the right scope | None; Sonarr's old Jellyfin connector test passed with a fake key (#8805) | Medium | R2 | Each recipe creates a least-privilege key (never an administrator scope) in a session with a fresh fingerprint or face check, shows it once with exactly what to paste, and gives the server's HTTPS address. Recipes for video tools arrive with R2 | Recipe definitions mapped to scopes | Admin > Integrations > Connect a tool | SEC-EXT-010, SEC-EXT-012, SEC-EXT-008, SEC-EXT-066 |
| INT-026 | "Who am I" check for tools | A tool's test button proves that the key and its scope work | Sonarr's test passed with a fake key (#8805) | Low | R2 | One route returns the key's identity and scopes, and every failure (unknown, wrong, expired or revoked key) gets the same real 401, so a connection test cannot pass falsely. R2, because API keys arrive in R2 (security baseline, owner decision 8). | Introspection route | None (API only) | SEC-EXT-009, SEC-EXT-065, SEC-EXT-067, SEC-API-003 |
| INT-027 | Device-code sign-in for tools and scripts | A command-line tool or helper gets a token without anyone copying secrets around | Jellyfin Quick Connect and Plex link codes (for apps, not tools) | Low | R2 | Uses the same pairing rules as TVs: the tool shows a code, and the person types it on a signed-in phone or browser (never a link or picture someone sent), sees the tool's self-reported name marked as unverified, picks scopes from those they hold (never an administrator scope) and confirms with a fresh fingerprint or face check. The result is an ordinary API key that the person can list and revoke | Pairing routes; scope picker | Approval sheet on phone and web | SEC-STD-027, SEC-IAM-056, SEC-IAM-058, SEC-IAM-060, SEC-EXT-012, SEC-EXT-010 |
| INT-028 | Companion-tool sign-in without passwords | Users sign in to Seerr and similar tools without a server password | Seerr signs in with server credentials; no OIDC yet | High: Seerr OIDC issue 277 reactions | Later | Not before an architecture record for delegated third-party access that adopts ASVS 10.4, 10.6 and 10.7 at Level 3 (pushed authorisation requests, short-lived codes, consent that can be reviewed and revoked); until then Gunmetal runs no OAuth authorisation server (SEC-STD-026). In R2 a person connects a companion tool with their own scoped key through the device-code flow (INT-027). Signing in to Seerr itself is Seerr's job, through the household's identity provider | Architecture record first; then an authorisation-code flow with pushed requests and revocable consent records | Consent screen; Account > Apps and tokens | SEC-STD-026, SEC-EXT-012, SEC-EXT-010, SEC-PRV-033 |
| INT-029 | Integrations inventory | One page lists every token, webhook, plugin and connected app, what each can do, and when it last acted | None | Low | R2 | Possible because all four go through one scope model. Admins see each credential's kind, scopes, owner, creation and expiry. When a member's own key, app or per-user plugin last acted is shown only to that member, because it would reveal when they listen | Aggregation query | Admin > Integrations | SEC-EXT-014, SEC-EXT-044, SEC-PRV-025, SEC-OPS-027 |
| INT-160 | Alerts for new keys and apps | When a key or app credential is added to my account, gains scopes, or is first used from a new kind of network, I am told and can revoke it in one tap | Not compared in the research | Low: no direct request; the Huntarr key leak (INT-017) shows why | R2 | Every key event (created, scopes changed, first use from a new address, revoked) goes to the person's own security log. Creation and first use from a new kind of network notify the person's devices with a one-step "This wasn't me" that revokes the key, ends its sessions and stops its stream URLs. Actions an admin takes with a key always alert, and a legacy Subsonic key (INT-088) alerts on its first use from a new kind of network | Credential audit events; notices to the account's devices; one-step revoke route | Account > Security; device notice; Account > Apps and tokens | SEC-EXT-017, SEC-HIS-063, SEC-OPS-033, SEC-OPS-034, SEC-IAM-097, SEC-EXT-069 |
| INT-161 | Where my data goes | One page lists every key, app, plugin and automation that can read or receive anything about me, what it gets, and a revoke or off switch for each | Not compared in the research | Low: no direct request; the Plex 2023 exposure (ACC-114) shows the trust cost | R2 | Generated from the same grant records that the policy layer and the egress client enforce, as the plugin consent screen is, so it cannot drift from what really happens. It covers the person's own keys and adapter apps, the per-user plugins they turned on and the admin automations they opted into (INT-162), and names each destination host and the fields sent. It sits beside "What your admin can see" (ACC-115) | Grant introspection across keys, adapters, plugins and webhooks | Account > Privacy > Where my data goes | SEC-PRV-027, SEC-IAM-104, SEC-EXT-014, SEC-EXT-039, SEC-EXT-045, SEC-PRV-053 |

### Events, webhooks and notifications

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| INT-030 | Built-in webhooks, free | Automations trigger when things happen | Plex yes (Plex Pass); Jellyfin free official plugin; Emby Premiere per its help article | Medium: paywall complaints; a lifetime Plex Pass now costs $749.99 | R2 | Parity with Jellyfin's free webhook plugin on price; the edge is the transactional outbox and signed payloads (INT-038). Webhooks stay off until the owner turns them on and lists the destination hosts. The outbox table ships in R1, written in the same transaction as each change, for the server's own sync and event stream; its entries follow the retention schedule, and a webhook receives only events that happen after it is created, so nothing recorded in R1 is later sent anywhere; delivery to destinations arrives in R2. | Outbox table; delivery worker | Admin > Webhooks | SEC-EXT-045, SEC-EXT-001, SEC-OPS-035, SEC-PRV-005, SEC-TM-075 |
| INT-031 | Versioned event catalogue | A clear list of triggers whose payloads do not change shape without notice | Jellyfin 23 notifier types; Plex a shorter list | Medium | R2 | Event types are protocol types with schema versions and appear in the API spec. Music events come in R1, video in R2 and live TV in R3 | Event schemas | Event picker in the webhook editor; API docs page | SEC-EXT-045, SEC-API-091, SEC-PRV-001 |
| INT-032 | Playback events with a "counted" threshold | Start, pause, resume, stop and "counted as played" | Plex counts a scrobble past 90%; Jellyfin start, progress and stop | Medium | R2 | For music, "counted" follows the Last.fm and ListenBrainz rule: tracks over 30 seconds count after half their length or four minutes, whichever comes first. Video uses a set threshold. Both use play time reported by the client, including offline plays uploaded later. Playback events go only to the person's own webhooks, or to admin webhooks they opted into (INT-162); private sessions emit nothing, and admin webhooks carry titles only for people who allow titles | Session tracking; threshold rules in the core | Event picker | SEC-EXT-045, SEC-PRV-024, SEC-PRV-025, SEC-PRV-033, SEC-OPS-035 |
| INT-033 | Library events grouped and sent after metadata | One "album added" message instead of twelve track messages | Jellyfin requests (#31, #329, 7 reactions each); Emby waits for metadata | Low | R2 | The scanner knows where a batch ends, so it emits one event per album (and per season in R2) after tags and artwork are read | Batch boundaries; debounce window | Grouping switch in the webhook editor | SEC-EXT-045, SEC-API-016, SEC-TM-026 |
| INT-034 | Replacement events | An upgraded file is reported as replaced, not new | Jellyfin "Recently Added shows upgrades" 62 votes | Medium: 62 votes | R2 | The identity matcher emits `item.replaced` and keeps history (INT-120) | Identity matcher | Event picker | SEC-EXT-045, SEC-API-016, SEC-TM-069 |
| INT-035 | Played, unplayed, rating and favourite events | Sync tools react when someone marks or rates something by hand | Plex rate event; Jellyfin plugin request (#118, 8 reactions) | Low | R2 | Emitted straight from the log, so every source of a change (app, adapter or import) produces the same event. These are a person's own activity, so they follow the same audience rule as playback events (INT-032) | Log-to-outbox bridge | Event picker | SEC-EXT-045, SEC-PRV-022, SEC-PRV-024, SEC-PRV-033 |
| INT-036 | Account and security events | New device, failed sign-ins, token created, revoked or near expiry | Plex new-device event; Jellyfin sign-in success and failure, lockouts | Medium | R2 | Device-bound keys make "new device" a fact rather than a guess from user agents. A person's own security events go to their own webhooks, and server-wide security alerts go to the owner's. Payloads carry the event type, the time and a link to the server, never addresses, other people's names or secrets | Security event emitter | Event picker; Account > Security | SEC-EXT-045, SEC-OPS-035, SEC-OPS-034, SEC-IAM-098, SEC-OPS-027 |
| INT-037 | Server and maintenance events | Update available, backup done or failed, integrity problem, task failed | Plex backup and database-corruption events; Jellyfin update notice planned | Medium: Jellyfin update notification 51 votes | R2 | Integrity events carry the typed parser or check error and the item ID, so the message says what is wrong and which item, never a file path. "Update available" exists only when the owner answered yes to the update check at first run | Hooks into backup, health checks and the update check (operations map) | Event picker; admin dashboard | SEC-EXT-045, SEC-OPS-035, SEC-OPS-047, SEC-TM-040 |
| INT-038 | Signed payloads | The receiver can prove the event came from your server | Plex, Jellyfin and Emby do not sign (as far as checked) | Low | R2 | Uses the Standard Webhooks scheme: HMAC-SHA256 with ID, timestamp and signature headers, a replay window, and secret rotation that signs with old and new secrets for 24 hours. The secret is shown once at creation and can be rotated, never revealed again | Secret store; signer | Webhook editor (rotate the secret) | SEC-EXT-046, SEC-EXT-050, SEC-EXT-016 |
| INT-039 | Retries, delivery log and redelivery | Missed events are not lost, and failures are visible | Unverified for all three; WatchState recommends polling as a backstop | Medium: WatchState FAQ | R2 | Outbox entries are retried with jittered backoff, honouring `Retry-After`. Each endpoint queues at most 1,000 events, and an endpoint failing for 72 hours is switched off and its owner told. The log shows each delivery's time and whether it succeeded, never the receiver's status code, latency or body, so a webhook cannot be used to probe the network. Any delivery can be sent again | Delivery log with retention; per-endpoint queue cap and auto-disable | Deliveries tab on each webhook | SEC-EXT-048, SEC-EXT-049, SEC-EXT-044 |
| INT-040 | Test button and sample payloads | Check that a webhook works before relying on it | Emby yes (sample payload); Jellyfin missing | Medium: Jellyfin webhook issues with 48 and 28 reactions | R2 | Samples are generated from the schema using real items from libraries the webhook may see, never anyone's activity. A test send reports only success or failure and is limited to 5 a minute per webhook | Sample generator | Webhook editor | SEC-EXT-048, SEC-EXT-045, SEC-EXT-047 |
| INT-041 | Filters by event, user and library | Only the events you care about are sent | Emby yes; Jellyfin item type and user (unverified); Plex per account | Low | R2 | Filters reuse the scope model, so a webhook never carries data from a library it may not see | Filter evaluation | Webhook editor | SEC-EXT-045, SEC-API-016, SEC-TM-026 |
| INT-042 | Templates that cannot run code | Each message is shaped for its destination, safely | Jellyfin Handlebars; Tautulli's evaluated notification text was a remote-code-execution bug (CVE-2026-28505) | Medium: Tautulli's 2026 CVEs | R2 | Templates only substitute values and never evaluate expressions, values are escaped for the destination's format, and templates are checked against the event schema | Template renderer as pure code in the core | Template editor with live preview | SEC-HIS-056, SEC-TM-046, SEC-TM-031, SEC-EXT-045 |
| INT-043 | Artwork in payloads, safely | Rich embeds in Discord and chat apps | Plex thumbnail (unverified); Jellyfin request (1 vote) | Low | R2 | A token or a server URL never appears in a payload: the server's capability URLs are bound to a session and expire, so none is handed to a chat service. Items with a MusicBrainz release ID can carry the public Cover Art Archive image link, which the chat app fetches itself; other items go without artwork | Cover Art Archive link built from the release MBID | Template editor | SEC-API-026, SEC-API-028, SEC-EXT-045, SEC-OPS-035 |
| INT-044 | First-party destinations | Messages arrive where you already look | Jellyfin: Discord, Gotify, Pushbullet, Pushover, Slack, SMTP, MQTT, generic; Plex: generic HTTP only | Medium | R2 | Generic HTTP, ntfy, Discord, Gotify and Slack-compatible presets are all built on one generic sender plus templates, so each preset is small and tested. A destination URL (a Discord webhook URL is itself a secret) is encrypted at rest and shown masked after creation | Preset definitions | Destination picker | SEC-EXT-047, SEC-EXT-001, SEC-EXT-045, SEC-EXT-050 |
| INT-045 | New arrivals posted to a chat channel | "Album added" appears in a Discord channel | Jellyfin via its webhook plugin; Plex via Plex Pass webhooks or Tautulli; Emby (Premiere) | Low | R2 | Grouped events (INT-033) plus the Discord preset (INT-044) mean one message per album, not per track | None beyond webhooks | Webhook editor | SEC-EXT-045, SEC-API-016, SEC-TM-026 |
| INT-046 | Email destination | Event emails to the admin or to users | Jellyfin SMTP notifier; Plex no | Low: Jellyfin email overhaul request 47 votes | Later | Parity, when it comes. Email alerts are Later in the security baseline's release scope, and SMTP is a second outbound protocol that the egress client does not carry. Messages come from a typed builder with fixed subjects, carry no titles or other people's activity, and go only to recipients the owner configured | SMTP with TLS required through the egress client, added to the egress inventory; typed message builder; the same templates | Admin > Notifications settings | SEC-STD-032, SEC-OPS-035, SEC-PRV-030, SEC-OPS-017, SEC-TM-075 |
| INT-047 | MQTT destination | Events go to an MQTT broker for home automation | Jellyfin via its webhook plugin; Plex no | Low | Later | MQTT is a second outbound protocol that the egress client does not carry, so it waits for an architecture record that adds it to the egress inventory with the same address checks; until then a generic HTTPS webhook to an MQTT bridge does the job. A retained "now playing" topic then carries only people who opted in (INT-162), never a private session, and titles only for people who allow them | MQTT client inside the egress client; egress inventory entry | Destination picker | SEC-API-078, SEC-TM-075, SEC-EXT-045, SEC-PRV-033, SEC-PRV-024 |
| INT-048 | Server-sent event stream | Dashboards and tools get events pushed to them instead of polling | Plex event stream; Jellyfin WebSocket (Jellystat polled anyway); Emby needs a plugin for Tracearr | Medium: Jellystat issues 298 and 328 | R2 | Carries the same catalogue as webhooks, filtered by key scope and by the same audience rules (INT-032). After a disconnect, the stream resumes from the outbox, and revoking the key closes the stream within 5 seconds | Fan-out; replay window | None (API only) | SEC-API-016, SEC-EXT-045, SEC-API-017, SEC-NET-053, SEC-TM-068 |
| INT-049 | Network rules for webhooks | Webhooks reach only hosts the owner listed; a LAN service such as Home Assistant is reachable only at an exact host and port the owner entered | None documented | Low | R2 | Webhooks stay off until the owner turns them on and lists the destination hosts. A private address is allowed only when it exactly matches a host and port the owner entered as a LAN destination, never a range, and loopback, link-local and cloud-metadata addresses are never allowed. The check runs on the resolved address at connect time, and redirects are not followed. Per-user webhooks may use only listed hosts, and guests get none by default | Destination allowlist; exact LAN grants; resolved-address classifier in the egress client | Admin > Webhooks > Allowed destinations; warning in the webhook editor | SEC-EXT-045, SEC-EXT-047, SEC-EXT-002, SEC-API-079 |
| INT-050 | Per-user follows and alerts | "New album from an artist I follow", "new episode of my show" | Jellyfin subscribe-to-show requested; Seerr tells requesters when titles arrive | Medium: Jellyfin request 50 votes | R2 | Follows are user data in the user log. Alerts go to an in-app inbox that syncs like the library, and optionally to the user's own ntfy topic on a host the owner has listed (INT-049) | Follow records; fan-out job | Follow button on artist and show pages; Account > Notifications; inbox | SEC-EXT-045, SEC-PRV-030, SEC-API-016, SEC-PRV-022 |
| INT-051 | Per-user webhooks | Each person can wire up their own automations | Plex per-account webhooks; Jellyfin and Emby admin only (unverified) | Low | R2 | Each user's webhooks are limited by scope to that user's own events and may target only hosts the owner listed; guests get none by default | Per-user webhook records | Account > Notifications | SEC-EXT-045, SEC-EXT-047, SEC-EXT-016 |
| INT-052 | Native push to phones | Alerts on the lock screen | Emby pushes to its apps (secondary source); Jellyfin requested | Medium: Jellyfin request 48 votes | Later | Needs Apple's and Google's push services plus a relay, which conflicts with "no central account" (see open decisions) | Relay design | Phone notification settings | SEC-PRV-056, SEC-OPS-036, SEC-TM-053 |
| INT-053 | Live TV and recording events | Recording scheduled, started, failed or finished; tuner conflicts | Not compared for this event set; recordings fail silently on rivals | Medium: silent recording failures in the live TV research | R3 | The recorder's own failure reports become events, so you hear about a failed recording when it happens | Recorder hooks | Event picker | SEC-EXT-045, SEC-OPS-035, SEC-OPS-063 |
| INT-162 | Opt-in before household automations see my plays | The server owner's webhooks, Home Assistant and other automations learn when I play something only if I switch it on | Plex and Jellyfin webhooks report every user's playback to the admin's automations (unverified) | Low: no direct request; the Plex 2023 exposure (ACC-114) shows the cost of the opposite default | R2 | One per-person switch, "Let the server owner's automations know when I start playing something", off by default and never set by an admin. With it on, admin webhooks and the Home Assistant integration receive the person's playback events; titles are included only if the person also allows titles, and private sessions emit nothing either way. Turning it off applies to the next event | Per-profile opt-in read by the event filter; audit entry on change | Account > Privacy; Where my data goes (INT-161) | SEC-EXT-045, SEC-PRV-033, SEC-PRV-023, SEC-PRV-024, SEC-PRV-025, SEC-PRV-032 |

### Plugin platform

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| INT-054 | WebAssembly plugin sandbox | You can extend the server without risking it | Plex no third-party plugins; Jellyfin .NET DLLs in-process; Emby in-process (unverified); Navidrome WebAssembly | Medium: Jellyfin 12.0 plugin removal and plugin path-traversal fixes | R2 | Each plugin runs as a WebAssembly component in its own OS-sandboxed process (no files, no sockets, no process creation, capped memory) and talks to the server only over one IPC channel whose identity the server fixes at spawn. Plugins reach only the host functions they were granted, and all their network traffic goes through the server's egress client. R1 runs no plugins at all: no WebAssembly runtime is linked until the sandbox requirements pass, and platforms without the sandbox get no plugins | One sandboxed plugin-host process per plugin; host API with its own test suite | Admin > Plugins | SEC-EXT-018, SEC-EXT-019, SEC-EXT-020, SEC-EXT-021, SEC-EXT-022, SEC-TM-065 |
| INT-055 | Permissions declared and reviewed before enabling | You see what a plugin can touch before it runs | Navidrome manifest with reasons; Jellyfin none | Low | R2 | The manifest has a closed schema, and an absent permission means not granted. It lists hosts, Gunmetal scopes, the data received, secrets and resources, each with a reason, and the host enforces exactly that list. Only the owner installs and grants, with a fresh fingerprint or face check; a per-user plugin shows each person its own consent screen on phone, web or desktop, never on a TV | Manifest parser in the core | Permission review sheet | SEC-EXT-025, SEC-EXT-039, SEC-EXT-038, SEC-TM-066, SEC-OPS-032 |
| INT-056 | Network allowlist checked on the resolved address | A plugin can call only the hosts it declared and never your LAN | Navidrome yes, after two advisories showed DNS, redirect and WebSocket bypasses; Jellyfin none | Low | R2 | All plugin HTTP runs in the server's egress client, which checks the resolved address at connect time and re-checks every redirect. Grants name exact hosts, never wildcards or IP literals. Private addresses are refused unless the owner granted that exact host and port, and loopback, link-local and cloud-metadata addresses can never be granted. Size and time are capped, and any URL a plugin returns is fetched only under that plugin's own grant | Grants enforced by the server's egress client | Plugin detail > Network | SEC-EXT-002, SEC-EXT-003, SEC-EXT-026, SEC-EXT-027, SEC-TM-048 |
| INT-057 | Plugin network log | You can see which hosts each plugin contacted | None | Low | R2 | The host records the host, time, bytes and status of every outbound request (never bodies) | Log with retention | Plugin detail > Activity | SEC-EXT-016, SEC-PRV-008, SEC-PRV-005 |
| INT-058 | Per-user settings and secrets | Each person links their own Last.fm or Trakt account, and the admin never sees the token | Jellyfin no (the admin enters every user's settings); Navidrome yes | Low: two Jellyfin requests, 4 votes each, but the admin holds everyone's tokens | R2 | Secrets are encrypted per user and handed only to the plugin acting for that user. The admin sees "linked", never the secret, and unlinking deletes it with anything still queued | Per-user secret store | Account > Connected services | SEC-EXT-050, SEC-EXT-030, SEC-EXT-031, SEC-PRV-033, SEC-PRV-036 |
| INT-059 | Settings forms generated from a schema | Plugins get settings screens on every device without shipping web code | Navidrome JSONForms; Jellyfin plugins ship their own HTML and JavaScript | Low | R2 | Gunmetal's own client draws the form on TVs and phones too, so no plugin script ever reaches the admin UI | Schema validation | Plugin settings screens | SEC-EXT-032, SEC-EXT-028, SEC-TM-036 |
| INT-060 | Versioned plugin interface | A server upgrade does not break your plugins | Jellyfin 12.0 required removing all third-party plugins; 10.11.9 broke repository lookups (#16905) | Medium | R2 | The interface is defined in WIT and versioned, and the host supports the current and previous major version. An incompatible plugin is disabled with a reason and never crashes the server | Interface versions and shims | Compatibility badge in the plugin list | SEC-EXT-076, SEC-EXT-025, SEC-EXT-022 |
| INT-061 | Resource limits | A plugin cannot pin the CPU or eat memory | No rival documents any | Low | R2 | Fuel or epoch interruption, memory caps and per-call timeouts | Runtime limits | Plugin detail > Limits | SEC-EXT-023, SEC-EXT-033, SEC-TM-068 |
| INT-062 | Failure isolation and auto-disable | A broken plugin is switched off with a reason, and the server carries on | Not documented in the research | Low | R2 | A crash stays inside the sandbox. A plugin that hits a limit or crashes three times in 10 minutes is suspended with backoff and the owner is told; no plugin failure can stall the scanner or playback | Health tracking | Plugin status; event | SEC-EXT-024, SEC-EXT-044, SEC-OPS-032 |
| INT-063 | Isolated plugin storage | Removing a plugin removes its data | Navidrome per-plugin storage; Jellyfin per-plugin config files (unverified) | Low | R2 | Each plugin gets its own namespace. State it cannot lose, such as a scrobble cursor, lives in the user log, so a cache rebuild keeps it | Storage namespace; durable cursor records | Plugin detail > Data | SEC-EXT-033, SEC-EXT-050, SEC-EXT-030 |
| INT-064 | Durable delivery cursors | After an outage, an API change or a phone being offline, a scrobbler or tracker carries on exactly where it stopped | Jellyfin ListenBrainz plugin caches listens during outages; others not documented | Medium: Trakt API churn; scrobble gaps | R2 | The host reads the append-only log through one cursor per plugin and person, using stable event IDs, and passes the plugin only that person's events: never a private session, and only plays recorded after the person linked the service unless they pick a backfill range. Delivery is in order and never doubled | Cursor per plugin and user; event IDs | "Last sent" on Account > Connected services | SEC-EXT-030, SEC-EXT-029, SEC-PRV-035, SEC-PRV-024 |
| INT-065 | Signed first-party plugins | You know who built a plugin and that nobody tampered with it | No rival signs plugins (unverified) | Low | R2 | Built and signed in CI with build provenance, under a first-party publisher key that the signed (TUF) plugin index delegates; the server checks signature, length and hash against the index at install and at load. The project's release key is not reused for plugins | Signature check | Publisher badge on plugin detail | SEC-EXT-035, SEC-EXT-036, SEC-EXT-043, SEC-OPS-015 |
| INT-066 | Project-maintained first-party plugins | The integrations people rely on do not depend on one tired volunteer | Jellyfin's SSO plugin, original Last.fm plugin, Jellyfin-RPC and jellysub are all archived | Medium: key-person failures in the research | R2 | Last.fm and ListenBrainz ship first, in R2 with the plugin host (R1 runs no plugins), followed by Trakt and the metadata providers. All live in the main repository under the same test gate | Plugin release process | Admin > Plugins | SEC-EXT-018, SEC-EXT-043, SEC-PRV-013 |
| INT-067 | Third-party plugins from an added index | You can add community extensions | Jellyfin extra repositories by URL; Emby curated catalog | Medium | R2 | Same sandbox and review sheet. Plugins install only from a signed index; the owner adds a third-party index after confirming its root key fingerprint, with a fresh fingerprint or face check. An unsigned plugin runs only in developer mode, which only the server's configuration file can switch on, and every client then shows a banner naming it. Updates wait for the owner (INT-163) | Signed-index client; signature and hash check; update hold | Admin > Plugins > Add; Admin > Plugins > Indexes | SEC-EXT-035, SEC-EXT-036, SEC-EXT-037, SEC-EXT-038, SEC-EXT-040 |
| INT-068 | Curated plugin catalog | Browse vetted plugins | Jellyfin official repository; Emby curated catalog | Low | Later | Parity. The index is a static, signed TUF repository; its refreshes send no identifiers and can be switched off, and it can only advise, never disable or change a server | Static signed index on gunmetal.tv; no accounts | Admin > Plugins > Browse | SEC-EXT-036, SEC-EXT-041, SEC-EXT-042, SEC-TM-067 |
| INT-069 | Rust plugin SDK and test harness | Authors can build and test plugins locally | Jellyfin C# plugin template; Navidrome kits in four languages | Low | R2 | The host's own conformance tests ship with the SDK, so authors test against the real contract | SDK; mock host | Docs site; developer mode set only in the server's configuration file, with a banner in every client | SEC-EXT-037, SEC-EXT-034, SEC-STD-037 |
| INT-070 | More plugin languages | Authors can write plugins in Go, Python or JavaScript | Jellyfin C# only; Navidrome Go, Rust, Python, JavaScript | Low | Later | Parity | Language kits | Docs site | SEC-EXT-021, SEC-EXT-022, SEC-EXT-076 |
| INT-071 | Scheduled plugin tasks | Plugins run jobs in the maintenance window | Jellyfin plugins add scheduled tasks | Low | R2 | Plugin tasks appear in the task list with progress and a cancel button (operations map) | Scheduler grant | Admin > Tasks | SEC-EXT-023, SEC-EXT-033, SEC-TM-068 |
| INT-072 | Plugin check before upgrading | Before an upgrade, you see which plugins it would disable | No rival offers one | Medium: Jellyfin 12.0 upgrade pain | R2 | The release feed lists interface versions, and the server compares them with the installed plugins. With the update check off, the comparison runs against the release the owner downloaded | Preflight check | Admin > Updates | SEC-OPS-047, SEC-SUP-050, SEC-EXT-040 |
| INT-073 | In-process native plugins | Plugins that run as native code inside the server | Jellyfin yes; Emby yes (unverified) | Low: nobody asks for in-process plugins as such | No | Not doing: a native plugin could bypass every grant, and the sandbox is the whole point | None | None | SEC-EXT-018, SEC-HIS-056, SEC-TM-065 |
| INT-074 | Plugins that inject scripts into the web client | Home screen and player tweaks through injected JavaScript | Jellyfin community plugins do it (Jellyfin-Enhanced, 1,869 stars) | Medium: the demand is real | No | Not doing: script injection is an XSS hole. Declarative home rows and themes (INT-081) meet the same demand | None | None | SEC-EXT-032, SEC-API-049, SEC-TM-036 |
| INT-163 | Plugin updates wait for the owner | A plugin update never changes what runs on the server until the owner approves it, and an update that asks for more is always held | Not compared in the research; Chrome keeps an extension that asks for new permissions disabled until the user accepts | Low: no direct request; the Cyberhaven extension compromise (December 2024) is the case | R2 | The previous version keeps running until the owner approves. The owner may opt in per plugin to automatic updates that change no permission and come from the same publisher key; those install no sooner than 72 hours after the index published them. An update that adds or widens a permission always waits, and a lower version is refused | Held-update store; permission diff; 72-hour timer | Admin > Plugins > Updates; owner alert | SEC-EXT-040, SEC-EXT-038, SEC-EXT-035, SEC-EXT-036, SEC-TM-066, SEC-OPS-032 |
| INT-164 | Warning when a plugin version is revoked | If a plugin version on the server is found to be malicious or broken, the owner is told why and advised to turn it off; nobody can turn it off remotely | Emby pushed an update in 2023 that stopped compromised servers from starting | Low: no direct request; follows from the Emby 2023 compromise | R2 | The signed index may mark a version revoked with a short reason. The server shows the reason, recommends disabling the plugin and logs the notice, but never disables it itself, so the index cannot act as a kill switch | Revocation field in the index client; owner notice | Admin > Plugins; owner alert | SEC-EXT-041, SEC-TM-067, SEC-EXT-036, SEC-OPS-032 |

### Plugin extension points

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| INT-075 | Scrobbler extension point | Plays are reported to outside services | Jellyfin via plugins; Navidrome scrobbler capability | Medium | R2 | Fed by the log cursor (INT-064), not by live session hooks, so no play is missed | Host interface | Account > Connected services | SEC-EXT-030, SEC-PRV-033, SEC-PRV-035, SEC-PRV-024 |
| INT-076 | History-import extension point | Pull years of history from a service | No rival has one built in | Low | R2 | Imported events are tagged with their source and can be removed as a batch | Import job; source tags | Account > Data > Import | SEC-EXT-030, SEC-EXT-031, SEC-API-081, SEC-TM-031 |
| INT-077 | Metadata and artwork provider extension point | You choose or add the sources of metadata and images | Plex HTTP providers (beta, no music); Jellyfin in-process plugins; Navidrome sandboxed agents | High: nobody accepts a film library without posters (library research) | R2 | Each provider declares its hosts. The host, not each plugin author, enforces the provider's terms, such as TMDB cache age and attribution. The library map owns provider order and field provenance | Provider interface; terms enforcement | Admin > Library > Providers | SEC-EXT-027, SEC-EXT-028, SEC-PRV-013, SEC-PRV-014, SEC-PRV-015 |
| INT-078 | Lyrics provider extension point | Lyrics are found for files that have none | Plex yes (Plex Pass); Jellyfin provider plugins; Navidrome provider plugins | Medium | R2 | A first-party LRCLIB plugin (free, keyless, open source) with a grant for one host. It looks up tracks without lyrics at scan time or when someone presses "Find lyrics on LRCLIB", never because lyrics are shown. Results are parsed as untrusted lyrics and cached in the server's data directory, never written into the media folder | Provider interface | "Find lyrics" in the lyrics view | SEC-PRV-015, SEC-PRV-013, SEC-API-090, SEC-MED-038, SEC-EXT-026 |
| INT-079 | Subtitle provider extension point | You can search for subtitles | Plex built in (OpenSubtitles); Jellyfin plugin from the item menu | Medium | R2 | Searches run only on an explicit "search subtitles online" action that names the provider, never at play start. Results are parsed by the core and kept in the server's data directory, never written into the media folder, and go to the running player, which loads them without a restart (INT-122) | Provider interface | Player subtitle menu | SEC-PRV-015, SEC-API-089, SEC-EXT-028, SEC-MED-038 |
| INT-080 | List-source extension point | Collections fed by Trakt, MDBList, Letterboxd, TMDB or IMDb lists | Kometa for Plex; Kometa support for Jellyfin in progress | Medium: Kometa 3,442 stars | Later | A plugin with a network grant refreshes the list into a declarative collection (INT-140) | Provider interface; schedule | Collection editor "source" | SEC-EXT-026, SEC-EXT-028, SEC-API-081 |
| INT-081 | Home-row and theme extension point | Plugins add labelled, removable home rows and visual themes | Jellyfin Home Screen Sections plugin (web only); Emby Home Screen Companion | Medium | Later | Rows are typed data that Gunmetal's clients draw on every platform, never injected script (the discovery map owns rows). It needs its own versioned plugin interface and permission, added through an architecture record | Row interface | Home screen editor | SEC-EXT-076, SEC-EXT-032 |
| INT-082 | Notification-destination extension point | Notifications can reach any service Apprise reaches | Jellyfin fixed list; Apprise covers most services | Low | Later | Long-tail destinations live in plugins, so the server ships only a tested core set | Destination interface | Destination picker | SEC-EXT-076, SEC-EXT-047, SEC-EXT-026 |
| INT-083 | Search and recommendation provider extension point | Plugins add search sources or "more like this" engines | Jellyfin 12.0 yes | Low | Later | Parity. It needs its own versioned plugin interface, and search text goes to a provider only after the person turns that provider on | Provider interface | Search; radio | SEC-EXT-076, SEC-EXT-030, SEC-PRV-004 |
| INT-084 | Guide-data provider extension point | You can use Schedules Direct or other guide sources | Plex bundled guide (Plex Pass); Jellyfin Schedules Direct, which says Jellyfin ignores its rate limits | Medium: Schedules Direct IP blocks | R3 | The host enforces each provider's rate limits, so users are not blocked | Provider interface; rate limiter | Live TV > Guide sources | SEC-EXT-075, SEC-TM-071, SEC-PRV-060, SEC-EXT-033 |
| INT-085 | Podcast feed plugin | Podcasts in the same app | Plex removed podcasts; Jellyfin requested; Audiobookshelf yes | Medium: Jellyfin request 563 votes | Later | Each feed's host is an exact grant the owner approves, the fetcher refuses private addresses and re-checks redirects, and feed parsing is pure code in the core with external XML entities off (adjacent media map) | Fetcher plugin; feed parser | Podcasts section | SEC-EXT-026, SEC-API-080, SEC-HIS-024, SEC-HIS-034 |

### Compatibility adapters

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| INT-086 | OpenSubsonic adapter | Symfonium, Feishin, Amperfy, Supersonic, Tempus and other Subsonic apps connect, bringing their offline, car and watch support | Plex no; Jellyfin no (request "started" since 2020, jellysub archived); Navidrome, gonic, Ampache, LMS yes | High: Jellyfin request 77 votes; users stay for clients | R2 | Browsing, search, streaming, cover art, playlists, stars, ratings, scrobbles and the play queue are served from the native music model through an ID translation layer, with no second data model. Ships in R2 (see the release key in the feature map README); until then phone users have only the web client, at phone width in the browser in R1 (CLI-149) and as an installable web app from R1.1 (CLI-003). | Adapter routes; ID mapping | Admin > Compatibility | SEC-EXT-051, SEC-EXT-054, SEC-EXT-055, SEC-EXT-056, SEC-EXT-059, SEC-TM-070 |
| INT-087 | API-key-only sign-in for Subsonic apps | Apps connect with a revocable key, not your password | OpenSubsonic API-key extension; classic Subsonic sends a password-derived token | Medium | R2 | Uses per-app keys (INT-024). Token-and-salt sign-in with the account credential is refused. Which Subsonic clients support OpenSubsonic API-key sign-in is unverified; if few do, most apps can connect only through INT-088's legacy keys, which are off by default and work only on the home network. | Key validation | Account > Apps and tokens | SEC-EXT-067, SEC-EXT-057, SEC-EXT-058, SEC-EXT-068, SEC-EXT-064 |
| INT-088 | Legacy Subsonic apps through per-app legacy keys | Older apps without API-key support still work at home | Navidrome and others use the account password, stored reversibly | Medium: which apps support API keys is unverified | R2 | Off by default. A person marks one app key as legacy ("My app only has a password field"); only that key may then be sent in the password field, and it is checked against its stored keyed hash, so nothing reversible is kept and a leak exposes one app. It works only on home-network paths, never from the internet, lives at most 90 days by default, and its first use from a new kind of network raises an alert. Token-and-salt sign-in (MD5) stays refused unless the owner approves a written exception. R2 with the adapter, following the security baseline's recommendation (owner decision 8) | Legacy flag per app key; path-class check; new-network alert | "My app only has a password field" in Account > Apps and tokens | SEC-EXT-069, SEC-EXT-063, SEC-API-094, SEC-NET-024, SEC-OPS-033 |
| INT-089 | Honest extension discovery | Apps see only the features the server really has | Navidrome yes (OpenSubsonic) | Low | R2 | The extension list is generated from what is enabled and implemented, and checked by the conformance suite | Extension registry | None | SEC-EXT-061, SEC-EXT-055, SEC-EXT-060 |
| INT-090 | OpenSubsonic lyrics, playback report, index-based queue and transcoding | Synced lyrics, accurate play reporting, queue hand-off and Opus streams in third-party apps | Navidrome yes | Medium | R2 | Lyrics, playback reporting and the index-based queue ship with the adapter in R2, from the same parsers and queue as Gunmetal's own player. The transcoding extension is served from the Opus path of MUS-106 (R2, sandboxed encoder), and INT-089 advertises it only once that path exists. | Extension handlers | None | SEC-EXT-062, SEC-EXT-056, SEC-API-090, SEC-TM-044 |
| INT-091 | OpenSubsonic sonic similarity | "More like this" in third-party apps | Navidrome via plugins | Low | Later | Served from scan-time audio features, if the music map adopts them | Similarity index | None | SEC-EXT-055, SEC-EXT-054 |
| INT-092 | OpenSubsonic podcasts and bookmarks | Podcasts and audiobook bookmarks in Subsonic apps | Defined by OpenSubsonic (eight podcast endpoints) | Low | Later | Arrives together with podcasts and audiobooks | Endpoints | None | SEC-EXT-055, SEC-EXT-054, SEC-HIS-034 |
| INT-093 | Adapter plays go into the same log | Plays from Symfonium or Finamp count like plays in Gunmetal's apps | Navidrome `scrobble` endpoint; Jellyfin client reporting | Medium | R2 | Adapter plays are written to the append-only log with the device, so scrobblers, stats and history all see them; plays in a private session are not recorded | Log writer | History shows which app was used | SEC-EXT-056, SEC-PRV-002, SEC-PRV-024, SEC-PRV-032 |
| INT-094 | Adapter guardrails | Compatibility does not reopen old security holes | Navidrome had Subsonic sign-in bypasses (2023, 2025) and an unthrottled login (CVSS 7.4) | Medium: advisory history | R2 | Off by default, with no listener bound until the owner enables it with a fresh fingerprint or face check. Each adapter has its own port, ignores cookies, accepts only its own credential kinds and only over HTTPS: there is no plain-HTTP exception on the home network. It shares the native limiter and authorisation layer, every adapter route has cross-user tests, and switching adapters on changes nothing in the native API. No endpoint that is unauthenticated in Jellyfin is copied | Route policies; tests | Switches in Admin > Compatibility | SEC-EXT-051, SEC-EXT-052, SEC-EXT-053, SEC-EXT-064, SEC-EXT-066, SEC-TM-070 |
| INT-095 | Client certification and quirk notes | You know each supported app works | Navidrome documents workarounds per client | Medium | R2 | Contract tests replay request and response traces recorded from each certified app version. Open-source clients (for example Finamp, Feishin and Music Assistant) also run end to end in CI where they can run headless. Closed or paid apps such as Symfonium, Amperfy and play:Sub cannot be pinned and driven in CI (unverified per app), so they are certified by a manual script per release. A public matrix lists the certified versions. | Test rigs | Docs site; Admin > Compatibility | SEC-EXT-054, SEC-EXT-055, SEC-TM-025 |
| INT-096 | Connected-app view | See which apps connect through which adapter, with version and last use | Not documented for rivals | Low | R2 | One list across both adapters, tied to per-app keys | App registry | Admin > Compatibility; Account > Apps and tokens | SEC-EXT-014, SEC-EXT-073, SEC-IAM-042 |
| INT-097 | Music Assistant through OpenSubsonic | Whole-home audio from your library via Home Assistant, including Sonos, AirPlay and Cast | Music Assistant supports Plex, Jellyfin and Emby | Medium: Music Assistant 3,129 stars | R2 | Comes free with INT-086, because Music Assistant already supports Subsonic servers and includes a scrobbler module. It connects with its own app key to the server's HTTPS address | None beyond the adapter | Connection recipe | SEC-EXT-066, SEC-EXT-056, SEC-EXT-058 |
| INT-098 | Jellyfin adapter, music subset | Finamp, Jellify and Feishin connect | Jellyfin native; Navidrome 0.64.0 experimental, off by default | Medium | R2 | The adapter pins and states a target Jellyfin API version and signs apps in with Quick Connect, which ACC-131 owns (R2, with its conditions): the person types the app's code into Gunmetal's own app and sees the app, the device and whether it is on the home network, and requests from outside the home are refused unless the owner allows remote device sign-in. SEC-EXT-071 makes Quick Connect the adapter's default sign-in, so if those apps cannot show an 8-character code (ACC-131), this row waits with it rather than falling back to passwords. Its tokens are scoped, bound to the device and expire, whereas Navidrome's never expire | Adapter routes; Gunmetal's random IDs formatted as Jellyfin GUIDs | Admin > Compatibility | SEC-EXT-070, SEC-EXT-071, SEC-EXT-072, SEC-EXT-073, SEC-EXT-074, SEC-EXT-059 |
| INT-099 | Jellyfin adapter, video subset | Roku, Kodi add-ons and Jellyfin video apps connect | Jellyfin native; nobody else | High: Roku coverage depends on it (record 1) | Later | Built on the remux path and signed URLs underneath (timing is an open decision) | Adapter routes; playback-info mapping | Admin > Compatibility | SEC-EXT-074, SEC-EXT-070, SEC-EXT-055 |
| INT-100 | Jellyfin adapter, tool subset | Tools built for Jellyfin (Seerr, Bazarr, Home Assistant) work within what their scopes allow | Jellyfin native | Medium | Later | Each tool's calls map to a named scope, never to admin rights. User lists, other people's sessions and history, remote control and library refresh are absent from the adapter, not guarded, so tools that need household history, such as Jellystat and Tracearr, see only their key owner's own data | Read routes under scopes; no user-list, session-list or refresh routes | Connection recipes | SEC-EXT-055, SEC-EXT-056, SEC-EXT-061, SEC-PRV-032, SEC-PRV-025, SEC-EXT-010 |
| INT-101 | Plex API compatibility | Plex apps connect to Gunmetal | Plex native only | Medium: many people own Plex devices | No | Not doing: Plex's apps sign in through plex.tv accounts, and Plex applies its remote paywall to third-party API clients, so emulating Plex would mean depending on Plex's cloud | None | None | SEC-TM-067, SEC-TM-048 |

### Music listening services

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| INT-102 | Last.fm scrobbling, per user | Plays are counted on Last.fm | Plexamp built in; Jellyfin third-party plugin (original archived); Navidrome built in, per user | Medium: Jellyfin request 27 votes | R2 | Parity with Navidrome on per-user scrobbling, and probably on retrying after an outage (unverified); Subsonic clients such as Symfonium also submit offline plays with timestamps (unverified). The edge is that native, adapter and imported plays all pass through one log cursor (INT-064), and private sessions (ACC-117) never reach it. R1 has no scrobbling: R1 records every play in the log with its real timestamp, and when a person links Last.fm in R2 the plugin submits only plays recorded after the link, unless the person picks a backfill range (Last.fm refuses plays older than its cut-off; the exact window is unverified). Unlinking deletes the stored key and discards anything queued. Owns Last.fm scrobbling; MUS-192 points here. | First-party plugin; per-user session key | Account > Connected services | SEC-PRV-033, SEC-PRV-034, SEC-PRV-035, SEC-PRV-036, SEC-EXT-031, SEC-EXT-050 |
| INT-103 | ListenBrainz scrobbling, per user | Plays are counted on the open MusicBrainz service | Plex no (requested since 2018); Jellyfin plugin; Navidrome built in | Medium: 28 votes (Jellyfin), 23 (Plex) | R2 | As INT-102, with MusicBrainz IDs from tags in every submission Owns ListenBrainz scrobbling; MUS-193 points here. | First-party plugin; per-user token | Account > Connected services | SEC-PRV-033, SEC-PRV-035, SEC-PRV-036, SEC-EXT-030, SEC-EXT-050 |
| INT-104 | Now-playing updates | Friends see what you are playing right now | Jellyfin ListenBrainz plugin; Navidrome | Low | R2 | Sent from the live session as a separate setting that is off by default, and suppressed in private listening | Session hook | Switch in Connected services | SEC-PRV-035, SEC-PRV-024, SEC-PRV-033, SEC-EXT-030 |
| INT-105 | Scrobble filters and private listening | The kids' profile or one library is never scrobbled; any session can be kept private | Navidrome per-user filters (0.64); others (unverified) | Low | R2 | Filters work by profile, library and device. Private mode keeps a session out of history and out of every plugin's cursor Owns scrobble rules; MUS-195 points here. | Filter rules in the log reader | Account > Connected services; private-listening switch in the player | SEC-PRV-024, SEC-PRV-035, SEC-PRV-029, SEC-EXT-030 |
| INT-106 | Loved-track sync | Hearts match between Gunmetal and the service | Jellyfin ListenBrainz plugin both ways; Navidrome Last.fm loves not planned; Plex requested | Low | R2 | Two-way sync keyed on MusicBrainz IDs, with conflicts settled by event time from the log | Pull job; merge rule | Account > Connected services | SEC-EXT-030, SEC-EXT-050, SEC-PRV-033, SEC-API-081 |
| INT-107 | Import listening history from export files | See ADM-042, which owns this feature. Integration specifics: unmatched items go to a queue, and imported events can be removed as a batch. | Plex no; Jellyfin no; Navidrome requested | Medium: Navidrome request 28 upvotes; Plex requests | R1.1 | See ADM-042. | None beyond ADM-042. | Account > Data > Import | SEC-API-085, SEC-API-088, SEC-TM-031, SEC-TM-032 |
| INT-108 | Import listening history from service APIs | Pull history without downloading files first | None | Low | R2 | A plugin with a network grant, using INT-076 | Plugin | Account > Data > Import | SEC-EXT-030, SEC-EXT-031, SEC-API-081, SEC-PRV-033 |
| INT-109 | Service playlists in the library | ListenBrainz daily and weekly mixes appear as playlists | Navidrome plugin; Jellyfin ListenBrainz plugin; Plex requested (7 votes) | Low | R2 | Matched to your library by MBID, with unmatched tracks shown rather than silently dropped | Plugin; playlist write | Playlists | SEC-EXT-029, SEC-EXT-028, SEC-PRV-033 |
| INT-110 | Self-hosted scrobble targets | Send plays to Maloja or a similar server you run | Navidrome supports Maloja | Low | R2 | The same plugin, pointed at a host the owner grants: a person asks for their Maloja server, and the owner adds that exact host (for a LAN server, its exact host and port) to the plugin's grant with a fresh fingerprint or face check. Nobody can point the server at an arbitrary address | Plugin | Account > Connected services | SEC-EXT-026, SEC-EXT-038, SEC-API-079, SEC-EXT-002 |

### Watch-state trackers and sync

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| INT-111 | Trakt scrobbling and history sync | Plays are logged on Trakt and history stays in step | Plex via PlexTraktSync and Trakt's own sync; Jellyfin official plugin (imports capped at 100 items, duplicates bug) | High: Plex "full Trakt integration" 376 votes | R2 | Delivered from the log cursor, so history is never duplicated or cut short. Accounts are per user, and Trakt is a mirror of your history, not the record | First-party plugin; per-user OAuth | Account > Connected services | SEC-EXT-031, SEC-PRV-034, SEC-PRV-035, SEC-PRV-036, SEC-EXT-050 |
| INT-112 | Simkl | A cheaper or free alternative tracker | Jellyfin plugin; PlexTraktSync users asked for it | Low | Later | Parity | Plugin on the cursor model | Account > Connected services | SEC-EXT-030, SEC-EXT-031, SEC-PRV-033 |
| INT-113 | Letterboxd diary | Films logged to Letterboxd | No official integration (unverified); Jellyfin requested | Medium: Jellyfin request 75 votes | Later | Depends on API access, which is not openly available (unverified) | Plugin | Account > Connected services | SEC-EXT-030, SEC-EXT-031, SEC-PRV-033 |
| INT-114 | Anime trackers (AniList, MyAnimeList, Kitsu) | Anime progress kept in sync | Third-party tools; Jellyfin requested | Low: Jellyfin request 14 votes | Later | Parity | Plugins on the cursor model | Account > Connected services | SEC-EXT-030, SEC-EXT-031, SEC-PRV-033 |
| INT-115 | Ratings and watchlist sync | Ratings and watchlists follow you to trackers | PlexTraktSync; CrossWatch across many trackers | Low | Later | Ratings and watchlist entries are log events, so they sync the same way plays do | Plugin | Account > Connected services | SEC-EXT-030, SEC-EXT-031, SEC-PRV-033 |
| INT-116 | Mirror mode with Plex or Jellyfin | Run both servers during a move and keep progress in step | WatchState (1,571 stars), JellyPlex-Watched and CrossWatch do this between Plex, Jellyfin and Emby | Medium: popularity of these tools | R2 | Pulls from the other server on a schedule through an owner-granted destination (an exact host and port on the LAN), with the other server's credential kept as an encrypted integration secret, and writes the result to the log, tagged with its source so it can be removed as a batch. Responses are decoded as untrusted input. It writes back to the other server only for people who turn that on themselves, because write-back sends their activity off the server | Importer in pull mode (operations map); matching by external IDs | Admin > Migration | SEC-EXT-050, SEC-API-079, SEC-API-081, SEC-PRV-033 |
| INT-117 | History replication between Gunmetal servers | One history across your home and holiday-home servers | Plex account sync (excludes music and managed users); Jellyfin requested | Low: Jellyfin request 105 votes | Later | Append-only logs merge by event time over iroh, with no central account. It needs the federation architecture record first: until then no server accepts another server's word about a person | Replication protocol; merge rules | Account > Servers | SEC-IAM-081, SEC-TM-063, SEC-PRV-022 |

### Download automation, subtitles and requests

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| INT-118 | Lidarr refresh after an import | New albums appear as soon as Lidarr imports them | Lidarr has Plex and Subsonic connectors | Medium | R2 | Not through the OpenSubsonic adapter: its allowlist leaves out `startScan`, and adapter keys can only browse, play and report. Lidarr's custom-script connection calls the native path-scoped refresh (INT-011) with a key that holds only the refresh scope for the music root, and the connection recipe ships the script | None beyond INT-011; recipe script | Connection recipe | SEC-EXT-055, SEC-EXT-056, SEC-EXT-010, SEC-EXT-011 |
| INT-119 | Sonarr and Radarr refresh after an import | New episodes and films appear within seconds | Plex yes; Jellyfin yes, but 12.0 broke it until a Sonarr fix on 2026-07-27; Emby yes | High | R2 | Not a Jellyfin-adapter route: the adapter has no library-refresh endpoint and accepts no API keys. Sonarr's and Radarr's custom-script connection calls the native path-scoped refresh (INT-011) with a refresh-only key, and a native connector contributed upstream would replace the script | None beyond INT-011; recipe script | Connection recipe | SEC-EXT-055, SEC-EXT-007, SEC-EXT-010, SEC-API-004 |
| INT-120 | Upgrade-aware imports | A better copy keeps watch state and position and is not announced as new | Jellyfin lost watch state on replace (#15001); Plex (unverified) | Medium: 62 votes on upgrades in Recently Added | R2 | Identity from fingerprints and external IDs survives replacement, and history points at the identity, not the path Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Identity matcher (library map) | Recently added stays clean; item history | SEC-TM-069, SEC-API-023, SEC-MED-038 |
| INT-121 | Bazarr refresh after a subtitle download | New subtitles appear immediately | Bazarr supports Plex; Jellyfin since April 2026; not Emby | Medium | R2 | Refresh by external ID or path under a refresh-only key on the native API, through a Bazarr connector contributed upstream. Bazarr's Jellyfin connector cannot be used, because the adapter has no refresh route | Refresh by IMDb ID | Connection recipe | SEC-EXT-010, SEC-EXT-011, SEC-TM-043, SEC-EXT-055 |
| INT-122 | Live subtitle pickup | A subtitle Bazarr has just downloaded can be turned on without stopping the film | Nobody; Bazarr's docs say to stop and resume | Low | R2 | An item-changed event reaches the active session, and Gunmetal's own player loads the new sidecar | Session notification | Player subtitle menu marks the new track | SEC-API-016, SEC-API-089, SEC-MED-053 |
| INT-123 | Subtitle search when playback starts | Missing subtitles are fetched the moment you press play | Plex plus Bazarr (needs Plex Pass); Jellyfin and Emby no | Low | Later | A playback-start webhook with a payload preset for Bazarr, sent only for people who let household automations see their plays (INT-162) and never for a private session; Gunmetal itself never looks up subtitles at play start. Whether Bazarr can accept it is unverified | Payload preset | Connection recipe | SEC-EXT-045, SEC-PRV-015, SEC-PRV-024, SEC-PRV-033 |
| INT-124 | Seerr availability | Seerr knows what you already have, and in which quality | Seerr scans Plex, Jellyfin and Emby | High: Seerr 12,770 stars | R2 | A read-only "library and availability" scope with external IDs and quality per version, used by a native Seerr connector (see open decisions) | Availability queries | Connection recipe | SEC-EXT-010, SEC-EXT-011, SEC-API-014, SEC-TM-026 |
| INT-125 | Watchlist feeds requests | Adding a title to your watchlist files a request | Plex yes, via Seerr; Jellyfin has no watchlist yet | Medium: Jellyfin watchlist 1,294 votes | R2 | Each person's watchlist (discovery map) is their own data, so Seerr reads it only with a key that person created for it (INT-027), never with an admin's key | Watchlist read route | Watchlist | SEC-PRV-022, SEC-EXT-011, SEC-PRV-033 |
| INT-126 | Request from the title page | Request a missing title, or open it in Seerr, from Gunmetal's apps | Streamyfin, Moonfin and Plezy integrate Seerr; no server does | Medium | R2 | The admin sets the Seerr address once, and clients deep-link using the item's external ID | Setting; deep-link template | "Request" or "Open in Seerr" on title pages | SEC-API-047, SEC-CLI-002, SEC-CLI-025 |
| INT-127 | Report a problem from the player | Flag broken audio or bad subtitles | Through Seerr for all three | Low | Later | The report carries the exact file, track and timestamp from the player, and goes to Seerr or stays in Gunmetal | Issue records | "Report a problem" in the player menu | SEC-EXT-045, SEC-PRV-030, SEC-TM-036 |
| INT-128 | Music requests through Lidarr | Request an album you do not have, follow an artist, and see what Lidarr is still looking for | Seerr merged Lidarr connectivity in June 2026 and still ships no music requests; the SeerrNG fork ships them; no player does them in-app | Medium: Seerr issue 96, 134 reactions | R2 | A request-provider plugin (record 32) requests an album or follows an artist through Lidarr's API with an owner-scoped key, shows wanted, monitored and missing states, and reaches only the exact LAN host and port the owner granted | Plugin; record 32 | "Request album" and "Follow in Lidarr" on artist pages; wanted chips | SEC-EXT-026, SEC-EXT-050, SEC-EXT-076, SEC-API-079 |
| INT-129 | Removal requests and cleanup | Ask for something to be deleted; old unwatched titles are cleaned up | None built in; Maintainerr (2,315 stars) and Janitorr | Medium: Seerr issue 308, 93 reactions | Later | Tools such as Maintainerr get only household aggregates: per-item play counts and last-played dates built from people who opted in to household statistics, and only when at least three of them contribute, never per-person history. "Request removal" lets people ask directly | Aggregate statistics route under the household aggregate rule | "Request removal" on title pages | SEC-PRV-025, SEC-PRV-022, SEC-HIS-060, SEC-EXT-010 |
| INT-130 | Upcoming episodes from Sonarr | See episodes due soon for shows you follow | Jellyfin declined pulling Sonarr's schedule; community plugin rows | Low | Later | A plugin feeds a labelled home row | Plugin | Home row | SEC-EXT-076, SEC-EXT-026 |
| INT-131 | Built-in request manager | Requests handled inside the media server | None of the three; Jellyfin declined | Low: Jellyfin request 24 votes, declined | No | Not doing: Seerr is good and popular, so Gunmetal integrates with it instead of competing | None | None | None (No) |
| INT-132 | Built-in download automation | Searching and downloading built into the server | None of the three | Low: no request found in the research | No | Not doing: not the server's job, and it would add legal exposure | None | None | SEC-MED-038, SEC-TM-042, SEC-TM-048 |

### Monitoring and statistics tools

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| INT-133 | Read-history scope for stats tools | Stats tools read a person's own plays and sessions with that person's key, plus household totals | Plex via its API (Tautulli); Jellyfin via the Playback Reporting plugin; Tracearr across all three | Medium: Tautulli 6,598 stars; Tracearr 2,680 | R2 | `history:read` covers only the key owner's own log and sessions; with the change feed and the event stream, tools never need to poll. Household statistics are aggregates built from people who opted in, reported only when at least three contribute. No key reads another adult's history, and other people's live sessions show no title unless they allow titles. Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Scope; own-log queries; household aggregates | Connection recipe | SEC-PRV-025, SEC-PRV-022, SEC-HIS-060, SEC-EXT-010, SEC-TM-054 |
| INT-134 | Playback decision in the API | Tools see direct play, remux or transcode, and the reason | Shown by Tautulli, Streamystats and Tracearr | Medium | R1.2 | The decision engine in the core gives a structured reason for every session. From R1.2, with the admin's live view (ADM-100), that is direct play or why a file cannot play, because R1 and its point releases have no transcoder; Opus, remux and transcode follow in R2. Admins see the method and reason without the title unless the person allows titles | Reason codes in sessions and events | Now playing (operations map) | SEC-PRV-025, SEC-TM-054, SEC-API-068 |
| INT-135 | Prometheus metrics | See ADM-127, which owns this feature. Integration specifics: direct-play, remux and transcode counters that test the server's main claim. | Jellyfin exporter since 2020; Navidrome opt-in; Plex none (unverified) | Medium: Jellyfin request 56 votes | R2 | See ADM-127. | None beyond ADM-127. | Switch in Admin > Integrations | SEC-OPS-059, SEC-NET-046, SEC-EXT-016, SEC-PRV-025 |
| INT-136 | Session control API | Automations can stop a stream, with a message | Jellyfin requested (open); Tracearr automations | High: Jellyfin request 402 votes | R2 | The API side of ADM-102: stopping a session revokes it and cuts in-flight responses (ACC-122). A key can stop only its owner's own sessions; stopping someone else's stream needs an admin's interactive session, because no key holds an administrator scope. The message is shown only as plain text | Session control route | Now playing (operations map) | SEC-HIS-014, SEC-EXT-010, SEC-PRV-025, SEC-IAM-043, SEC-OPS-034 |
| INT-137 | Remote control of Gunmetal apps | Pause, skip or send media to a Gunmetal app from another tool | Jellyfin remote entities in Home Assistant; Plex clients controllable (unverified) | Medium | R2 | Gunmetal's own apps accept commands over their existing connection, scoped per user and device: only a person's own sessions, unless the target person granted control, and only from a closed set of commands | Command channel | Device picker | SEC-HIS-014, SEC-API-016, SEC-IAM-043 |

### Collections and library managers

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| INT-138 | Playlist write API | Tools can create and update playlists | Plex and Jellyfin through their APIs | Low | R2 | Playlists are durable user data written through the log, so playlists made by tools survive a rebuild. Gunmetal's own clients use the same playlist routes from R1 (MUS-132); this row, tools writing playlists with `playlists:write` keys, is R2, because API keys arrive in R2 (security baseline, owner decision 8; D-10) | Playlist routes | Playlists | SEC-TM-024, SEC-TM-027, SEC-IAM-072, SEC-API-010, SEC-MED-039 |
| INT-139 | Collection write API | Kometa-style tools can build collections | Kometa fully supports Plex only | Medium: Kometa's Jellyfin and Emby request has 33 reactions | R2 | Collections get the same durable treatment as playlists | Collection routes | Collections | SEC-TM-024, SEC-TM-027, SEC-EXT-010 |
| INT-140 | Collections and smart playlists as a file | Curation lives in a text file you can version, and survives any rebuild | Kometa keeps customisations outside the server database | Medium | Later | The server reads a documented declarative format from its own configuration directory, never from a file it writes into a media folder, and evaluates its rules against the library under parser budgets. "Export as file" is a download | File watcher on the configuration directory; rule engine | "Export as file" in the collection editor | SEC-MED-038, SEC-TM-031, SEC-TM-032, SEC-STD-011 |
| INT-141 | Artwork upload API | Poster tools such as Posterizarr can set artwork | Posterizarr supports all three | Low: Posterizarr 931 stars | R2 | Uploads are decoded with size and pixel limits in the sandbox, re-encoded with metadata removed, and stored under content-hash names in the server's data directory, never the media folder | Upload handler | Artwork picker on title pages | SEC-API-085, SEC-API-086, SEC-API-087, SEC-PRV-006, SEC-MED-061, SEC-TM-035 |
| INT-142 | Kometa support | Kometa runs against Gunmetal | Kometa supports Plex; support for other servers is a work list on a branch | Medium | Later | Contribute a connector for the native API rather than emulate Plex | Native API only | None | SEC-EXT-010, SEC-EXT-011 |
| INT-143 | Poster overlays baked into images | 4K, HDR and audio badges drawn onto poster files | Kometa rewrites posters; Emby overlays (Premiere) | Medium: Jellyfin 4K marker request 146 votes | No | Not doing: Gunmetal's clients draw badges from scan data (library map), so artwork files stay clean | None | None | SEC-MED-038, SEC-TM-042 |

### Smart home, voice and system hooks

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| INT-144 | Home Assistant integration | Your own Gunmetal sessions are media players in Home Assistant, with library browsing, search and sensors | Plex core integration; Jellyfin core integration; Emby exists (scope unverified) | High: Jellyfin request 355 votes, now complete | Later | Behind Plex and Jellyfin on the whole-household view, by design. Maintained by the project and built on a person's own scoped key, the event stream and session control, it shows and controls that person's own sessions. Other people's sessions appear only if they let household automations see their plays (INT-162), without titles unless they allow titles, and sensors report counts, not titles | Integration maintained by the project, built on scoped tokens, the event stream and session control | Home Assistant | SEC-PRV-025, SEC-PRV-032, SEC-EXT-045, SEC-EXT-010, SEC-HIS-014 |
| INT-145 | Play by search | "Play the latest episode of X in the lounge" | Plex JSON search with filters | Low | Later | Parity | Search route with typed filters | Home Assistant | SEC-API-063, SEC-EXT-010, SEC-HIS-014, SEC-PRV-004 |
| INT-146 | Voice-assistant skills | See CLI-131, which owns this feature. | Plex shut its Alexa skill on 2026-06-15; Emby Alexa (Premiere); Jellyfin requested | High: Plex Google Home 2,461 votes; Jellyfin 243 | No | See CLI-131. | None beyond CLI-131. | None | SEC-TM-048, SEC-PRV-007 |
| INT-147 | Stable deep links | Links to an album, artist or film open in the app | Infuse TMDB deep links; others (unverified) | Low | R1.2 | The web URLs are the deep links, from R1.2 with CLI-034, and the native apps register the same paths in R2. The API side of CLI-034, which owns deep links. | Stable routes | Share menus | SEC-CLI-025, SEC-API-023, SEC-API-070 |
| INT-148 | Operating-system automation hooks | Siri Shortcuts, Android intents and NFC tags start playback | Plexamp Siri and NFC; Finamp Siri | Low | R2 | The native apps expose them (clients map), using the same command channel as INT-137 | None beyond INT-137 | System settings; share sheet | SEC-CLI-025, SEC-HIS-014, SEC-PRV-058 |

### Social presence

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| INT-149 | Discord presence from the desktop app | Friends on Discord see what you are playing | No server does it natively; desktop bridges need a server token and a second app | Medium: Plex request 100 votes since 2018 | Later | The desktop app talks to the local Discord app, with no token on the server and cover art only from Cover Art Archive. Opt-in, per library, and private listening is honoured | None (desktop client, clients map) | Account > Connected services; player share menu | SEC-PRV-033, SEC-PRV-024, SEC-PRV-023, SEC-CLI-027 |
| INT-150 | Server-side presence with user tokens | Presence with no desktop app | Navidrome plugin stores the user's Discord token, which its own docs say may break Discord's terms | Medium: presence is wanted, but not this way | No | Not doing: storing user tokens breaks Discord's terms | None | None | SEC-EXT-050, SEC-PRV-033 |

### Data portability

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| INT-151 | Documented history export | Your plays, ratings and playlists as files you own | Plex no; Jellyfin no; Emby (unverified); WatchState portable backups | Medium: Jellyfin export request 17 votes; history view 830 | R1 | The log's format is versioned and documented. Exports come in its native form and also as ListenBrainz listen JSON and CSV | Export job | Account > Data > Export | SEC-PRV-047, SEC-PRV-048, SEC-API-071, SEC-IAM-106, SEC-TM-055 |
| INT-152 | Trakt history import | Bring your Trakt history over | PlexTraktSync; Jellyfin plugin imports only the newest 100 items | Medium | R2 | Reads Trakt's export or its API and removes duplicates against the log | Importer | Account > Data > Import | SEC-API-081, SEC-API-085, SEC-EXT-031, SEC-EXT-050 |
| INT-153 | Streaming playlist import | Bring Spotify playlists over, matched to your library | Jellyfin requested (29 votes; Plex, Trakt or IMDb import 31) | Low | Later | Parity | Plugin or file import; matcher | Playlists > Import | SEC-API-085, SEC-TM-031, SEC-EXT-031 |

### Other protocols and ecosystems

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| INT-154 | DLNA media server plugin | Older TVs and receivers can browse the library | Plex is a DLNA server; Jellyfin moved DLNA to a plugin | Low | Later | Only after its own architecture record, because DLNA has no authentication: serving it lets any device on the network browse without signing in. It would be a new versioned plugin interface that no existing grant implies (plugins cannot receive inbound requests today), never part of the core server. If it comes: off by default, local addresses only, read-only, only the libraries the owner selects, and SSDP answered only on the local link | Architecture record; plugin interface for inbound LAN requests | Admin > Plugins | SEC-NET-059, SEC-NET-066, SEC-HIS-053, SEC-EXT-076, SEC-EXT-031, SEC-IAM-013 |
| INT-155 | OPDS catalogue and KOReader sync | E-reader apps browse, download and sync position | Komga and Kavita yes; Audiobookshelf requested | Low: Audiobookshelf OPDS request 73 upvotes | Later | Arrives with books. Positions are kept as locators, so they are not cut back to the start of a chapter. E-reader apps sign in with per-app keys over HTTPS, never an account password; KOReader's sync protocol sends a password hash, so its key follows the legacy-key limits of INT-088 | Protocol routes | Books (adjacent media map) | SEC-API-094, SEC-EXT-057, SEC-EXT-069, SEC-NET-001, SEC-API-099 |
| INT-156 | Podcast sync protocols | AntennaPod-style apps sync with Gunmetal | gPodder sync and the Open Podcast API exist; rival support (unverified) | Low | Later | Parity, with per-app keys over HTTPS rather than account passwords (gPodder's API signs in with HTTP Basic authentication) | Protocol routes | Podcasts (adjacent media map) | SEC-API-094, SEC-EXT-057, SEC-NET-001 |
| INT-157 | Live TV lineup export | See LIV-171, which owns this feature. | Dispatcharr gives each profile its own M3U, guide and HDHomeRun link | Medium | Later | See LIV-171. Integrations specifics: there is no HDHomeRun-style device (LIV-172 is No) and no key carried in the URL path or query. A 90-day key in a URL is not a short-lived, bound capability, and the legacy-key rules (SEC-EXT-069) cover only Subsonic apps. The export credential is whatever the architecture record that LIV-171 requires defines, and every export route still needs a valid credential (SEC-TM-004). | None beyond LIV-171. | Admin > Export (LIV-171) | SEC-API-004, SEC-NET-059, SEC-TM-004, SEC-NET-001 |
| INT-158 | Plex-style HTTP metadata providers | Reuse community providers written for Plex's new model | Plex custom providers (beta, Dec 2025) | Low | Later | An adapter in the provider host, if the provider API's terms allow it (unverified) | Provider adapter | Admin > Library > Providers | SEC-EXT-026, SEC-EXT-027, SEC-EXT-028 |
| INT-159 | Immich photos on TV | Immich albums and memories show on Gunmetal TV apps | Not offered by rivals | Low | Later | A plugin with a grant for one Immich host (for a LAN server, the exact host and port the owner grants) and the user's own Immich key; photos pass the server's image pipeline before any client sees them | Plugin | Photos (adjacent media map) | SEC-EXT-026, SEC-EXT-050, SEC-EXT-030, SEC-API-086 |

## Differentiators

1. **Scoped, expiring tokens (ACC-049, INT-017 to INT-022 and the
   connection recipes, INT-025, in R2; the header-only rule, INT-023, in
   R1).**
   Every helper tool connected to Plex, Jellyfin or Emby holds what amounts
   to a master key. In 2026 that turned Huntarr's exposure into a leak of
   every connected *arr key. Tautulli also shipped more than a dozen security
   fixes that year. With Gunmetal, the Sonarr key can only refresh one
   folder, the stats key can only read its owner's own history, no key ever
   holds an administrator scope, and every key expires, shows its last use,
   alerts its owner when it is created (INT-160) and can be revoked in one
   click. Connection recipes make the safe setup easier than the unsafe one.
   Security-minded self-hosters have watched these breaches, so this is a
   reason to switch that needs no other feature to land first. API keys
   arrive in R2 (security baseline, owner decision 8), so R1 has no
   scripting API.
2. **OpenSubsonic with API keys only (INT-086 to INT-097; R2).** Users stay
   for their apps, and Navidrome added a Jellyfin API for exactly that
   reason. One adapter gives Gunmetal Symfonium, Feishin, Amperfy and other
   mature apps, along with their offline, car and watch features. It also
   brings Music Assistant (and through it Sonos and Home Assistant) and
   Lidarr's refresh connector, and it is the interim route for iPhone users
   while the Apple builds wait. It ships in R2, not R1, to keep the first
   release's attack surface small; it runs on its own port and only over
   HTTPS, no app ever holds the user's password, and which apps support
   API-key sign-in is unverified (INT-087).
3. **Free, signed and debuggable webhooks, plus a pushed event stream
   (INT-030 to INT-048; the outbox table in R1, delivery in R2).** Plex
   charges for webhooks, and Jellyfin's are free through a plugin, so on
   price this is parity with Jellyfin. Emby's help article
   says they need Premiere. Jellyfin's plugin lacks the test button its users
   ask for most, and polling tools have starved its thread pool. Gunmetal
   sends events from a transactional outbox, signs them with Standard
   Webhooks, retries them, records whether every delivery succeeded, groups
   "album added" after metadata is ready, and never evaluates template code.
   Webhooks reach only hosts the owner listed, and a person's plays reach
   only their own webhooks or automations they opted into (INT-162).
4. **Sandboxed plugins whose interface survives upgrades (INT-054 to
   INT-066; R2).** Jellyfin 12.0 told users to remove every third-party plugin
   before upgrading. Several integrations people relied on are now archived
   volunteer projects. Gunmetal runs each plugin in WebAssembly inside its
   own sandboxed process with declared grants, checks network access on the
   resolved address, keeps per-user secrets the admin never sees, uses a
   versioned interface, holds every update until the owner approves it
   (INT-163), and maintains the important plugins in its own repository.
5. **The log is the record, and trackers are mirrors (INT-064, INT-102 to
   INT-111 in R2; INT-151 in R1).** Navidrome already does per-user
   scrobbling well, so the edge is the single cursor, not scrobbling itself. Trakt now charges for things that used to be free,
   restricts its API, and caps imports. Scrobbles go missing when a service
   is down or a phone is offline. In Gunmetal, every scrobbler and tracker
   is fed from the append-only log through its own cursor, so nothing is
   lost or doubled; only plays after a person links a service are sent,
   unless they pick a backfill range, and private sessions never are. The
   history can be exported in a documented format. Gunmetal works as a
   self-hosted tracker on its own.
6. **A first-class citizen of the *arr stack (INT-011 and INT-118 to
   INT-122, in R2).** Refreshes are scoped to a single folder, through the
   native API rather than the compatibility adapters. Upgraded files keep
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
  into logs. Short-lived capability URLs, with the token in the path, cover
  the cases where a header is impossible.
- **Templates that can evaluate code (INT-042).** Tautulli's 2026
  remote-code-execution bugs came from exactly this.
- **Keys with administrator powers.** No key may hold an administrator,
  owner-only or host-equivalent scope, recipes included (SEC-EXT-010). An
  administrator automation credential would need its own architecture
  record. The narrow admin-granted scopes (refresh and server status) stay.
- **Plugins in R1, or plugins inside the server process (INT-054).** No
  WebAssembly runtime is linked until the sandbox requirements pass, and
  then each plugin runs in its own sandboxed process (SEC-EXT-018,
  SEC-EXT-021).
- **An OAuth authorisation server for third-party tools (INT-028), until its
  architecture record exists.** Companion tools get a person's own scoped
  key through the device-code flow instead (SEC-STD-026).
- **Household history for stats and cleanup tools (INT-129, INT-133,
  INT-144).** Keys read their owner's own history; tools see other people
  only as opted-in aggregates (SEC-PRV-025).
- **Writing into media folders.** Lyrics, subtitles, artwork and curation
  files from integrations live in the server's data directory, never next
  to the media (SEC-MED-038).

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
  inside an open-source binary, so none is shipped: shipped binaries hold no
  key or shared secret (SEC-TM-012), and each owner registers their own
  (open decision 5). Discord forbids using user tokens.
  Letterboxd's API is not openly available (unverified). TMDB limits caching
  and commercial use. Trakt changed its rules twice in two years.
- **Push and voice need vendor clouds.** iOS push goes through Apple, and
  voice skills need a public endpoint. Both sit uneasily with "no central
  account".
- **Webhooks and plugins share one network rule.** Home Assistant and other
  LAN services are legitimate destinations, but only as an exact host and
  port the owner entered, never a range, and never loopback, link-local or
  metadata addresses. The rule must be clear in the UI (INT-049, INT-056).
- **Strict defaults will cost some setups.** HTTPS only for adapters means
  apps pointed at `http://192.168.x.x` stop working (INT-094); webhooks with
  no owner allowlist, MQTT and email wait (INT-046, INT-047, INT-049); and
  Tautulli-style per-person dashboards are not offered (INT-133). Each is
  the security baseline's choice, and the UI has to explain it in plain
  words.
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
- **Scope creep.** This area alone has 164 rows. The order in which they
  arrive has to be decided explicitly, release by release.
- **R1 is now light in this area.** After the security baseline moved API
  keys to R2 and the owner adopted the smaller R1 with point releases
  ([D-10](../decisions.md#d-10-r1-scope-and-the-release-table)), the 8 R1
  rows here are the native API's own rules and the person's own data: the
  generated spec (INT-001), capability discovery (INT-005), consistent
  lists (INT-007), stable IDs (INT-008), MBIDs (INT-009), the health check
  (INT-013), header-only credentials (INT-023) and history export
  (INT-151). The export-file import owned by ADM-042 (INT-107) follows its
  owner to R1.1; playback reasons (INT-134) and deep links (INT-147) arrive
  in R1.2 with the admin's live view (ADM-100) and CLI-034; the change feed
  (INT-006) and tool writes to playlists (INT-138) move to R2, because only
  tools holding keys use them. Scopes, refresh, webhooks, the OpenSubsonic
  core and scrobbling are R2 and arrive together with the key model, the
  egress grants and the plugin sandbox they depend on.

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
   CLI-130 with it. Every adapter and API-key requirement in the security
   baseline (SEC-EXT-007 to SEC-EXT-017 and SEC-EXT-051 to SEC-EXT-069)
   then moves to R1 too; none may be deferred.
2. **Should a minimal plugin host ship in R1, so that Last.fm and
   ListenBrainz scrobbling arrive with the music release?** The alternative
   is an R1 without scrobbling. Building scrobblers into the server would
   contradict record 2. *This map recommended an R1 host; the feature map
   README decides R2:* a wasmtime host with grants, signing, an egress gate
   and per-user secrets is a large R1 item, and the security baseline
   forbids loading any plugin or WebAssembly runtime until the sandbox
   requirements pass (SEC-EXT-018). R1 records every play in the log with
   its real timestamp. When a person links a service in R2, only plays
   after the link are sent unless that person picks a backfill range
   (SEC-PRV-035), so older plays reach a service only by the person's own
   choice.
3. **What happens to Subsonic apps without API-key support?** The adapter
   accepts API keys by default (INT-087). *Recommendation (the security
   baseline's, owner decision 8; owner to confirm):* ship per-app legacy
   keys (INT-088) with the adapter in R2, off by default and turned on per
   app by the person: the app sends its own key in the password field, the
   server checks it against a keyed hash and stores nothing reversible, and
   the key works only on home-network paths, for at most 90 days, with an
   alert on its first use from a new kind of network (SEC-EXT-069).
   Token-and-salt sign-in (an MD5 of the password in the URL) stays refused
   unless a client survey shows an important app that cannot work
   otherwise and the owner approves a written exception. This replaces the
   earlier "Later, per-app generated passwords" recommendation, which the
   feature map README still carries.
4. **What licence terms apply to third-party WebAssembly plugins under the
   AGPL?** There is no CLA, so any plugin exception has to be added before
   outside contributions build up. Adding it later would need every
   contributor's agreement. *Recommendation:* get legal advice now. If
   permissive third-party plugins are wanted, add an explicit plugin
   exception before R1. Keep first-party plugins AGPL.
5. **Should the project hold API keys for Trakt, Last.fm and TMDB, or should
   users bring their own?** *Recommendation (changed to follow the security
   baseline; owner to confirm):* ship no project secret in any binary,
   because shipped binaries must contain no key or shared secret
   (SEC-TM-012) and the baseline asks for per-owner provider keys where a
   provider needs one (owner decision 22). The owner registers the server's
   own Last.fm API account and Trakt client ID once, stored as encrypted
   integration secrets (SEC-EXT-050); each person then links their own
   account. ListenBrainz needs only each person's user token. File import
   and export remain the fallback if a service revokes access. The TMDB
   question belongs to the library map but should be decided at the same
   time.
6. **When should the Jellyfin adapter cover video?** *Recommendation:* ship
   the music subset in R2 (INT-098). Keep the video subset (INT-099) as
   Later, unless Roku is declared a launch platform. If it is, video moves
   into R2 and takes priority over the tool subset.
7. **How should Seerr integrate?** The two options are a native Seerr
   connector contributed upstream, or a Jellyfin-adapter subset that Seerr
   already understands. The second option also needs passwordless sign-in.
   *Recommendation:* contribute an upstream connector that uses a read-only
   availability scope, and let each person give Seerr their own scoped key
   through the device-code flow (INT-027) for their watchlist. The consent
   flow (INT-028) is Later, behind its own architecture record, because the
   baseline forbids running an OAuth authorisation server until then
   (SEC-STD-026). Fall back to the adapter subset only if upstream
   declines; it would carry no user list, sessions or refresh route.
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
    on a schedule (INT-116), which works without a Plex Pass. Do not accept
    inbound Plex webhooks: Plex cannot send an `Authorization` header, so the
    credential would have to sit in the URL (SEC-API-004), and inbound
    webhooks are a new capability that needs its own architecture record
    (SEC-EXT-076).
12. **Should there be a remote MCP endpoint for AI agents?**
    *Recommendation:* Later (INT-016). Make it read-only, off by default,
    and built on scoped keys so it adds no new access paths; it
    authenticates with an API key, never through an OAuth authorisation
    server on Gunmetal (SEC-STD-026).
13. **Tool access in R1.** The security baseline puts API keys in R2
    (owner decision 8), so the R1 native API serves Gunmetal's own web
    client only, and the tool-facing rows (INT-011, INT-012, INT-017 to
    INT-022, INT-026) moved from R1 to R2. The adopted R1 (D-10) moved the
    change feed (INT-006) and tool writes to playlists (INT-138) to R2 for
    the same reason. *Recommendation (the
    baseline's; owner to confirm):* accept, and say plainly in the R1 notes
    that scripts and *arr connectors arrive in R2. ACC-049 and the feature
    map README's R1 cut must move with these rows. *Trade-off:* no
    scripting API and no *arr refresh in R1, against a smaller first
    release with no long-lived bearer secrets.

## Security notes

This area is where other people's code and decisions reach the server: tools
holding keys, apps speaking borrowed protocols, plugins, and outbound
deliveries to services the household chose. It crosses these trust
boundaries from [the threat model](../security/threat-model.md): TB1
(internet to server: keys and adapters reached from outside), TB2 (LAN:
adapters on the home network, webhooks to LAN services), TB4 (client to
server: the native API, adapters and third-party apps), TB7 (server to
plugins), TB8 (server to outbound hosts: webhooks, scrobblers, trackers,
providers), TB10 (storage: key hashes and integration secrets), TB11 (user
to admin to host-equivalent: scopes, plugin installs) and TB12 (project to
installs: the plugin index). The detailed register for plugins, keys,
webhooks and adapters is T-EXT-01 to T-EXT-34 in
[plugins-and-integrations-security.md](../security/plugins-and-integrations-security.md).

| Threat | Where it bites here | How the rows above answer it |
|---|---|---|
| TM-T02: an unauthenticated route leaks data (Jellyfin #5415) | Capability discovery, the health check, the API spec, adapter public endpoints | Nothing is revealed before sign-in: no version, no user list, a liveness-only health check, the spec only for signed-in users, and minimal adapter public fixtures (INT-001, INT-005, INT-013, INT-089, INT-094; SEC-API-002, SEC-API-005, SEC-EXT-061) |
| TM-T12, TM-T15 and TM-T66: one user reaches another's objects, or an adapter or secondary path skips a restriction | Adapters, the event stream, webhooks, the change feed | One authorisation layer for every path; adapters translate and never decide, run on their own port and accept only their own credentials; events are filtered per recipient; every adapter route has cross-user tests (INT-041, INT-048, INT-086, INT-094, INT-095, INT-100; SEC-EXT-054, SEC-EXT-055, SEC-API-016, SEC-TM-025) |
| TM-T14: a key widens itself (Immich CVE-2026-23896) | API keys | No key can manage any key; keys are made only in an interactive session with fresh verification; a key's rights are always the intersection with its owner's current rights; no key holds an administrator scope (INT-017, INT-018, INT-022; SEC-EXT-010, SEC-EXT-011, SEC-EXT-012) |
| TM-T16 and TM-T61: revocation does not bite, or nobody can tell what a key did | Keys, adapter tokens, webhooks | Revocation applies on the next request and cuts open streams; every key event is audited and shown to its owner, with "This wasn't me" (INT-020, INT-021, INT-096, INT-160; SEC-EXT-014, SEC-EXT-017, SEC-IAM-043, SEC-OPS-033) |
| TM-T18: history reaches admins or co-users beyond what people expect | Stats tools, Home Assistant, admin webhooks, cleanup tools, the integrations inventory | Keys read only their owner's history; household views are opted-in aggregates of at least three people; playback reaches admin automations only after a person opts in; each person sees where their data goes (INT-029, INT-129, INT-133, INT-144, INT-161, INT-162; SEC-PRV-025, SEC-PRV-033, SEC-EXT-045) |
| TM-T30: server-side request forgery | Webhooks, plugin grants, URLs returned by plugins, self-hosted scrobble targets, mirror mode | One egress client checks the resolved address at connect time; LAN destinations only as an exact host and port the owner entered; no redirects by default; a plugin's URLs are fetched only under its own grant (INT-049, INT-056, INT-110, INT-116; SEC-EXT-002, SEC-EXT-026, SEC-EXT-027, SEC-EXT-047) |
| TM-T33: lookups and plugins tell third parties what the household owns and plays | Scrobblers, trackers, lyrics and subtitle providers, chat webhooks | Off until each person links a service; only plays after linking unless they pick a backfill range; never a private session; fixed payload fields; lookups only at scan time or on an explicit action (INT-043, INT-064, INT-078, INT-079, INT-102 to INT-105; SEC-PRV-015, SEC-PRV-033, SEC-PRV-035, SEC-EXT-030) |
| TM-T34 to TM-T37: a malicious plugin steals data, escapes, bypasses its network grant or injects script | The plugin platform | No plugin code at all in R1; one sandboxed process per plugin; signed index only; owner-only install with fresh verification; updates held for approval; settings drawn by Gunmetal's own clients (INT-054 to INT-056, INT-059, INT-065, INT-067, INT-074, INT-163; SEC-EXT-018, SEC-EXT-021, SEC-EXT-032, SEC-EXT-035, SEC-EXT-040) |
| TM-T44: the project's channel is used against servers | The plugin index | The index can only advise: a revoked version gets a warning, never a remote disable (INT-068, INT-164; SEC-EXT-041, SEC-TM-067) |
| TM-T04 and TM-T05: guessing, or forged forwarding headers to dodge the limiter | Adapter sign-in and API keys | The native limiter covers keys and adapters, the client address comes from the socket unless a declared proxy sent it, and every failure gets the same answer (INT-026, INT-087, INT-094; SEC-EXT-009, SEC-EXT-015, SEC-EXT-064) |
| TM-T11, TM-T46 and TM-T48: credentials in cleartext, or "local means trusted" | Adapters, legacy keys, lineup export, DLNA, Quick Connect | HTTPS only with no home-network exception; keys that must travel in URLs are limited to home-network paths and 90 days and kept out of logs; location only adds friction, so remote Quick Connect is refused by default and DLNA waits for its own record (INT-088, INT-094, INT-098, INT-154, INT-157; SEC-NET-001, SEC-EXT-066, SEC-EXT-069, SEC-IAM-013) |
| TM-T57: secrets sit in plaintext or logs | Key secrets, webhook secrets and URLs, plugin and third-party tokens | Keys are stored as keyed hashes and shown once; integration secrets are encrypted and never returned; logs carry credential IDs only, and delivery logs only success or failure (INT-038, INT-039, INT-044, INT-058; SEC-EXT-008, SEC-EXT-016, SEC-EXT-050) |
| TM-T64: device-code phishing (Storm-2372) | Device-code sign-in for tools, Quick Connect | The person types the code on their own device, the tool's name is marked unverified, and nothing outside can trigger an approval prompt (INT-027, INT-098; SEC-STD-027, SEC-IAM-060, SEC-EXT-071) |
| TM-T69: a new feature adds an entry point nobody modelled | New plugin capabilities, MQTT and SMTP, DLNA, delegated access | Each waits for an architecture record and an egress-inventory entry before it is built (INT-028, INT-046, INT-047, INT-081, INT-154; SEC-EXT-076, SEC-TM-075, SEC-TM-001) |

Residual risk: some Subsonic and Jellyfin clients build media URLs with the
key in the query string and hand them to players and cast devices, so an
adapter key will sometimes appear in URLs Gunmetal does not control. It can
only read and play, is per app, is revocable with its last use shown, and
expires when idle. And whoever runs the host can read its files directly;
the product promises what it surfaces, and tells each person so.
