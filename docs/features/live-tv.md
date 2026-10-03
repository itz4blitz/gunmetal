# Live TV and recording

This area covers everything a household needs to watch and record linear TV
through a Gunmetal server. That means IPTV playlists and tuner hardware as
sources, guide data, the channel lineup a household curates, the programme
guide as a screen, live playback with pause and start-over, one-off and
rule-based recording, and the reliability, access control and
interoperability around them. Nothing in this area ships before R3. The bar
is this: an antenna or IPTV household gets the polish of Channels DVR, the
dependability of TVHeadend and the playlist control of Dispatcharr, free and
on a server sized for reading disks. No provider change, guide refresh,
library scan or upgrade should ever silently cost them a recording.

## Features

Evidence in the "Rivals today" and "Demand" columns comes from
`docs/research/`, mainly `live-tv-and-dvr.md` and `pain-points-and-demand.md`.
Vote and reaction counts are as those files read them on 2026-10-02. "No
evidence in research" means the research did not cover that rival for that
feature; it does not mean the rival lacks it. Releases (R1, R2, R3, Later, No), the Demand scale, row ownership and the
terms "the user log" and "the identity store" are defined once in the
[feature map README](README.md). A row whose Release cell would differ
between maps names one owning row; the other maps point at it.

### Setting up the module

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIV-001 | Live TV as a module you can switch off | Households without TV never see Live TV, and the server does no TV work | Plex partial (its own Live rows can be hidden, after complaints), Jellyfin no (request, 7 votes), Emby unverified | Medium: Jellyfin request (7 votes); Plex users complained about live rows they could not remove | R3 | Off by default. When off, no Live TV routes, background jobs, tuner discovery or screens exist, so a music-only server pays nothing for it | Module flag read at start-up; route and job registration gated on it | Admin > Modules toggle; navigation and home leave out Live TV when off |
| LIV-002 | Guided set-up | Choose antenna, IPTV playlist or TVHeadend, find tuners, pick a guide, check matches, finish | Plex yes (postal-code set-up with a bundled guide), Jellyfin yes (manual), Emby yes | Medium: first run is where Plex leads, and the research expects Gunmetal's to be weaker without a bundled guide | R3 | Parity at best. There is no postal-code shortcut because there is no bundled guide. The edge is a dry-run preview of channel count and guide coverage before anything is saved | Tuner discovery, source probe, guide-coverage calculation, one transactional save | Admin > Live TV > Set-up wizard, in TV and touch layouts |

### Sources: playlists and IPTV

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIV-003 | M3U playlist from a file or URL | Add an IPTV provider or a TVHeadend export as channels | Plex no (needs an HDHomeRun emulator; 119 votes, open since 2018), Jellyfin yes, Emby yes | High: Plex's gap created a whole category of proxy tools (xTeVe, Threadfin, Dispatcharr) | R3 | A streaming Rust parser under the no-panic, 100% coverage and zero-mutant gate; fetched only by the server's guarded fetcher (LIV-015) | Source record, scheduled fetch, playlist parser, channel update applied as a diff | Admin > Live TV > Sources > Add playlist |
| LIV-004 | MPEG-TS over HTTP and HTTPS | Ordinary IPTV and tuner streams play and record | Plex only through HDHomeRun or a proxy, Jellyfin yes, Emby yes | High: the baseline transport for tuners and IPTV | R3 | A Rust MPEG-TS demuxer (program tables, clock references, continuity counters) in the core. FFmpeg never opens a network connection, unlike the FFmpeg or VLC engines that Dispatcharr, Threadfin and Jellyfin hand streams to | Upstream HTTP client, TS demuxer, per-channel fan-out buffer | None directly; feeds every playback screen |
| LIV-005 | Live HLS channels | Channels delivered as m3u8 play and record, including AES-128 encrypted segments (which are not DRM) | Plex only through a proxy, Jellyfin request (5 votes, 2020; current state unverified), Emby unverified, Channels DVR yes | Medium: many free and legal streams are HLS | R3 | A live HLS client in Rust handles playlist reloads, discontinuities and AES-128 segments, so no outside tool with network access is involved | HLS playlist parser, segment fetcher, variant choice, segment decryption | Stats overlay shows the variant in use (LIV-086) |
| LIV-006 | Playlist attributes honoured | Channel numbers, logos, groups, guide IDs, time shift, catch-up and radio flags arrive intact | Plex only through a proxy, Jellyfin partial (tvg-id, number, logo; exact set unverified), Emby yes (name, ID, number, shift, group, logo), Kodi IPTV Simple the broadest | Medium: dropped attributes cause most of the manual mapping work | R3 | Matches Kodi IPTV Simple's attribute set, including catch-up and several groups per channel. Unknown attributes are kept, so a later release can use them without a re-import | Attribute storage on channel and stream records | Channel detail shows the raw attributes |
| LIV-007 | Custom headers and user agent | Providers that reject default clients still work | Jellyfin partial (user agent per tuner; header requests, 9 and 2 votes), Emby partial (user agent and referer per tuner), Kodi honours per-stream option lines | Medium: wrong headers give HTTP 403, per Emby's docs; Jellyfin requests are open | R3 | Headers per source and per stream, read from the playlist's own option lines. They stay on the server and are redacted from logs | Header sets on source and stream; log redaction | Source > Advanced: headers, user agent, referer |
| LIV-008 | Import filters | Keep only the groups and names you want from a 20,000-line playlist | Plex no, Jellyfin no (HDHomeRun "favourites only" import), Emby no (unverified), Dispatcharr ordered regex filters | Medium: huge provider playlists are normal; Threadfin and Dispatcharr exist partly for this | R3 | Filters run inside the parser before anything is stored, so unwanted channels never reach SQLite or devices. A preview shows how many channels each rule keeps | Ordered include and exclude rules on group, name and URL; preview endpoint | Source > Filters, with a live match count |
| LIV-009 | Large playlists stay usable | Thousands of channels import, map and browse without stalling | Plex partial (a 480-channel DVR limit is widely reported, unverified), Jellyfin and Emby no documented cap, Channels DVR caps each M3U at 750 | Medium: caps documented by Plex users and Channels DVR | R3 | Bounded-memory streaming parse, filtering before storage and delta-synced guides keep the cost proportional to the channels kept, not the playlist's size | Benchmarks against large captured playlists | Lineup screens use virtualised lists |
| LIV-010 | Playlist refresh on its own schedule | Playlists follow rotating tokens, independently of guide refreshes | Jellyfin requested (8 and 2 votes), Emby unverified, Dispatcharr interval or cron per account | Low: two small Jellyfin requests | R3 | Each source has its own interval, and every refresh is applied as a diff that never deletes channels or rules (LIV-060, LIV-061) | Per-source refresh job; diff engine | Source > Refresh schedule; "Refresh now" |
| LIV-011 | Several sources in one lineup | Antenna, IPTV and TVHeadend channels sit in one guide; the same channel from two sources becomes one channel with two streams | Jellyfin and Emby accept several tuners (Emby picks guides per tuner), TiviMate mixes playlists in Premium | Medium: mixing providers is a TiviMate paid feature | R3 | Channels are Gunmetal's own objects, so streams from any source attach to one channel and fail over between each other | Channel-to-stream links across sources | Lineup editor shows each channel's streams and sources |
| LIV-012 | Mirror URLs for one provider | When the main host fails, the next one is tried | Nobody; Dispatcharr request (25 reactions, open since October 2025) | Medium: clear unmet demand in the leading IPTV proxy | R3 | Mirrors are alternate base URLs on one source. The broker fails over on connection errors and detected stalls, and each recording notes which mirror served it | Mirror list per source; failover in the broker; health per mirror | Source > Mirrors |
| LIV-013 | Pooled logins for one provider | Two subscriptions with the same provider act as one larger connection pool | Plex, Jellyfin and Emby no; Dispatcharr account profiles and server groups | Low: a Dispatcharr feature with no vote counts in the research | R3 | Limits are counted per login and pooled per provider, so the broker places each new stream on a login with spare capacity | Login records with limits; pool allocator | Source > Logins |
| LIV-014 | Provider credentials stay on the server | Clients and shared users never see IPTV usernames, passwords or tokens | Not applicable to Plex; Jellyfin and Emby unverified; Dispatcharr in proxy mode | Medium: M3U URLs usually embed the username and password | R3 | Clients only ever get short-lived signed Gunmetal URLs. Provider URLs live only inside the broker, are redacted from logs and are left out of exports unless the admin asks | Secret storage outside client APIs; redaction | Source screen shows credentials masked |
| LIV-015 | Guarded fetching for every source | A playlist or guide URL cannot be used to reach the server's LAN or read its files | Jellyfin had SSRF and file read through its Live TV M3U tuner (fixed in 10.11.7, April 2026); Navidrome's plugin network guard is the model to follow | Medium: security complaints grew fastest in 2025 and 2026, and the Jellyfin flaw sat in this exact feature | R3 | Each source is a declared outbound host under record 2's network-grant model. Resolved addresses are checked at connect time and after redirects, private ranges need an explicit admin grant (for tuners and TVHeadend), and file sources are limited to declared folders | Grant model extended to sources; connect-time address checks | Source screen lists its granted hosts; a grant prompt when adding a LAN source |
| LIV-016 | Test a source before saving | See channel count, a test tune and any HTTP error before committing | No evidence in research for Plex, Jellyfin or Emby; Dispatcharr reports refresh errors (v0.31.0) | Medium: 403s from wrong headers and failed refreshes are common reports | R3 | A dry run through the same parser and fetcher used in production, plus a short probe of one stream that says whether native clients can play it as-is | Probe endpoint using the demuxer; no writes until confirmed | Add-source wizard: test results panel |
| LIV-017 | A single stream URL as a channel | Add one camera, event or station stream by URL | Plex no (IP cameras request, 50 votes), Jellyfin and Emby through M3U, Channels DVR custom channels | Medium: 50-vote Plex request | R3 | A manual channel is an ordinary channel object, with permissions, placeholder guide and recording | Manual channel record | Channels > Add channel by URL |
| LIV-018 | RTSP camera sources | Cameras that only speak RTSP work as channels | No evidence in research beyond M3U support | Low: only the camera request above, and only for cameras without HTTP streams | Later | A Rust RTSP client would keep FFmpeg off the network; worth building only if camera demand appears after R3 | RTSP client in the core | Same as LIV-017 |
| LIV-019 | UDP and RTP multicast | ISP IPTV and multicast headends on the LAN | Plex no (14 votes), Jellyfin no (7 votes), Emby unverified, TVHeadend yes | Low: small requests; common for European ISP TV | Later | The same Rust TS demuxer reads multicast. Containers likely need host networking for it (unverified) | Multicast group join; interface choice | Source > Multicast address |
| LIV-020 | Xtream Codes login | Sign in with server, username and password instead of a playlist URL | Plex no, Jellyfin through a third-party plugin, Emby no (unverified), Dispatcharr and TiviMate yes | Medium: common among paid IPTV services, many of them unlicensed (unverified) | Later | Parity if built. Held back for a policy decision (see open decisions) | Xtream API client mapped onto the source model | Add-source wizard option |
| LIV-021 | Stalker or MAC portals | Set-top-box style portals | Plex, Jellyfin and Emby no; TiviMate yes; Dispatcharr request (11 reactions) | Low: 11 reactions | No | Works by imitating set-top-box identities; legally fraught and tied to unlicensed services | None | None |
| LIV-022 | Redirect mode | Clients connect straight to the provider | Dispatcharr offers redirect or proxy, per user since v0.31.0 | Low: no vote data | No | Exposes provider URLs and credentials to clients and bypasses connection counting and per-channel permissions | None | None |
| LIV-023 | Xtream video-on-demand catalogues | Provider film and series catalogues | Outside the research's scope | Low: not measured | No | A separate, large catalogue product, not live TV; the research names it as scope creep | None | None |

### Tuners and broadcast hardware

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIV-024 | HDHomeRun network tuners | Plug in a network tuner and it is found and used, including unencrypted cable on cable models | Plex yes (all models), Jellyfin yes (legacy units need a first scan with SiliconDust's Windows tool), Emby yes (DLNA 2015 models or later), Channels DVR adds tuner sharing | High: the standard antenna route, supported by all three rivals | R3 | Plain HTTP device support with no drivers. The broadcast plays as-is on native clients (LIV-078), so an MPEG-2 antenna channel costs a network read, not a transcode CableCARD and QAM cable tuners are not planned; see Deliberately not doing. | Discovery, channel scan, tuner status, stream counting per tuner | Wizard tuner list; Admin > Tuners |
| LIV-025 | Add a tuner by address | Tuners on another subnet or behind a container network still work | No evidence in research | Low: follows from container deployments, no direct evidence | R3 | Manual address entry beside discovery, under the same LAN grant as LIV-015 | Tuner record by address; health probe | Tuners > Add by address |
| LIV-026 | TVHeadend as a source | Satellite, cable, DVB cards and SAT>IP through an existing TVHeadend | Plex no (satellite request, 63 votes), Jellyfin through TVHeadend's M3U and XMLTV (its docs say this costs more CPU), Emby through plugins | Medium: 63-vote satellite request; Hauppauge cards on Jellyfin, 18 votes | R3 | A tested preset for TVHeadend's M3U and XMLTV exports. TVHeadend keeps the kernel and driver work; Gunmetal keeps the guide, rules and clients | Preset source type; guide matching tuned to TVHeadend's IDs | Wizard option "I use TVHeadend" |
| LIV-027 | TVHeadend's own protocol | Richer integration than M3U, such as tuner state | Emby plugin; Plex and Jellyfin no | Low: no vote data | Later | Only if the M3U route proves too limited | HTSP client | Same as LIV-026 |
| LIV-176 | Other DVR back ends as sources (NextPVR, DVBViewer and others) | Keep an existing NextPVR, DVBViewer, MediaPortal, ServerWMC or Vu+ setup and watch through Gunmetal | Emby supports the widest range of DVR back ends; Jellyfin has plugins for several (research) | Low: no vote data | Later | No per-product protocol client: these back ends export M3U and XMLTV, which LIV-003 already reads; a tested preset is added per product only if its exports need one | None beyond LIV-003 | Admin > Live TV > Sources |
| LIV-028 | Native SAT>IP client | Use a SAT>IP box without TVHeadend | Plex no, Jellyfin no, Emby through plugins, TVHeadend client and server | Low: no vote data | Later | Shares the RTSP client from LIV-018 | SAT>IP discovery and RTSP | Admin > Tuners |
| LIV-029 | Tuner priority | Prefer one tuner, or ATSC 3.0 over 1.0 | Plex no (requests, 30 and 9 votes), Jellyfin channel-priority request (3 votes), TVHeadend priorities | Medium: 30-vote Plex request | R3 | Ordered tuner and stream preferences are inputs to the broker's allocation | Priority fields used by the broker | Admin > Tuners, drag to reorder |
| LIV-030 | Share tuners with other apps | Other apps can use the tuner, and Gunmetal copes when they do | Plex no (expects exclusive access), Jellyfin and Emby unverified, HDHomeRun shares itself on the network | Low: no vote data | R3 | The broker treats "busy elsewhere" as a normal state and falls back to the next tuner or says plainly what is happening | Tuner status polling; fallback | Tuner-busy sheet (LIV-087) |
| LIV-031 | ATSC 3.0 channels | NextGen TV with HEVC video and AC-4 audio | Plex had no AC-4 audio as of April 2024 (current state unverified), Jellyfin unverified, Emby's FFmpeg build decoded AC-4 in 2024 | Medium: users built their own AC-4 converter for Plex | Later | Video passes through untouched. If a client cannot decode AC-4, only the audio is converted in the sandbox, which costs far less than a full transcode (FFmpeg AC-4 support unverified) | Audio-only sandboxed transcode for live input | Stats overlay shows the audio conversion |
| LIV-032 | Direct USB and PCIe DVB tuners | Gunmetal drives Linux DVB devices itself | Plex yes (a short list of models), Jellyfin no (18 votes), Emby Hauppauge on Windows only, TVHeadend any Linux DVB device | Medium: European satellite and card users | No | Kernel and driver work needs device privileges the server should not hold; TVHeadend already does it well (LIV-026) | None | None |
| LIV-033 | DRM-protected broadcasts | Encrypted ATSC 3.0 stations | Nobody; SiliconDust told the FCC in July 2025 that no gateway had been approved | Medium: affects US antenna users | No | There is no legitimate route for third parties, and circumvention is not an option | None | None |
| LIV-034 | Descrambling | Decrypt scrambled satellite or cable channels | TVHeadend does software descrambling | Low: no vote data | No | Legal limits vary by country; the project should not carry that exposure | None | None |

### Guide data

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIV-035 | XMLTV from a file or URL | Use any guide from any source | Plex yes, but it replaces Plex's data for that DVR and switching means deleting the DVR; Jellyfin yes; Emby yes | High: the main open guide format | R3 | A streaming Rust parser. Each refresh is applied as a diff, so changes appear without the manual cache purge Jellyfin users need (#6103, 20 reactions) | Guide source, parser, diff apply, programme store | Admin > Live TV > Guide sources |
| LIV-036 | Compressed guide files | gz, xz and zip guides download faster | Jellyfin request (4 votes), Dispatcharr and Kodi yes | Low: 4 votes | R3 | Parity | Streaming decompression | None beyond the source form |
| LIV-037 | Several guide sources per channel, in priority order | Fill gaps from a second source, or mix markets | Plex no (611 votes, 279 posts, open since 2017), Jellyfin partial, Emby yes (several guides per tuner) | High: Plex's top Live TV request | R3 | Merged per channel and per time slot in a fixed priority order, with gaps filled from lower sources and a per-channel override | Merge job; source priority; provenance per programme | Guide sources, drag to order; channel detail shows which source filled each slot |
| LIV-038 | Schedules Direct | Accurate paid listings for the US and Canada | Plex only through an XMLTV grabber, Jellyfin yes but Schedules Direct says it ignores back-pressure and blocks its IPs, Emby deprecated it in June 2021 | Medium: Jellyfin #13147 (23 reactions, 146 comments); costs $35 a year | R3 | A client that honours rate limits and back-pressure from day one, with a hard request budget enforced by a contract test in CI; network access through an explicit grant | Account link, lineup choice, budgeted fetch job, back-off | Guide sources > Schedules Direct sign-in and lineup picker |
| LIV-039 | Guide from the broadcast itself | A free guide straight from the antenna (ATSC PSIP first; DVB event data arrives through TVHeadend's export) | Plex no (19 votes), Jellyfin no (HDHomeRun guide request, 24 votes), TVHeadend yes | Medium: two requests, and the only free guide for US antenna users without a bundled guide | R3 | Event tables parsed in Rust from the tuner stream, including ATSC's compressed text. An idle-tuner scan job yields to viewers and recordings. The horizon is usually short (unverified) | Table parsers; idle-tuner scan job; merged as a low-priority source | Guide sources > "Broadcast guide" toggle; coverage per channel |
| LIV-040 | Placeholder guide | Channels with no data still show blocks you can record | Plex no ("record a programme not in the EPG", 124 votes), Jellyfin and Emby unverified, Dispatcharr dummy guides from channel names, Channels DVR placeholder blocks | High: 124-vote Plex request | R3 | Placeholder blocks of a set length, optionally titled from channel-name templates. They are real programme objects, so rules and manual recordings work on them | Placeholder generator; templates | Channel detail: block length and title template |
| LIV-041 | Time-zone and offset correction | Guides from abroad line up with local time | Plex no, Jellyfin no, Emby tvg-shift, Kodi tvg-shift plus catch-up correction, Dispatcharr request (20 reactions) | Medium: 20 reactions | R3 | All times are stored as UTC instants with offsets per source and per channel; daylight-saving handling is property-tested with an injected clock | Offset fields; time conversion in the core | Guide source and channel detail: offset |
| LIV-042 | Guide horizon and history | Choose how far ahead to fetch and how long to keep past days | Plex unverified, Jellyfin GuideDays setting (keep-past request, 7 votes), Emby 1 to 14 days or automatic | Low: 7 votes | R3 | Past days are kept only as long as catch-up or start-over can use them, and the storage cost is shown beside the setting | Retention job; size estimate | Admin > Guide settings |
| LIV-043 | Category mapping | Programmes land under Movies, Sports, Kids or News | Plex partial (requests, 9 and 8 votes), Jellyfin yes, Emby yes | Low: small Plex requests | R3 | Parity | Category map per source | Guide settings > Categories |
| LIV-044 | Guide refreshes never break schedules | Updating listings never loses a scheduled recording | Jellyfin no (#12979: an hourly refresh wiped every scheduled recording, closed as stale), Plex and Emby unverified; no product documents a guarantee | High: one of the recording-loss pain points | R3 | Refresh is a diff. Scheduled recordings re-bind to the new entry by series and episode identity, then by time and title; nothing is deleted, only flagged. Property tests replay refreshes against captured guides | Diff engine; re-binding rules; flags | Admin alerts; a recording notes "moved to a new time" |
| LIV-045 | Programme artwork | Posters and stills in the guide | Plex yes, Jellyfin request (4 votes), Channels DVR art picker | Low: 4 votes | R3 | The server fetches each image once through its grant, resizes it and serves it from cache, so devices never contact third-party image hosts | Image fetch, resize and cache | Guide cells; details panel |
| LIV-046 | Series identity | The same show is recognised across channels and airings | Plex yes (unverified), Jellyfin by title or series ID (CRID request, September 2026), TVHeadend broadcaster series links | Medium: every series recording depends on it | R3 | Identity from series ID, season and episode, CRID where present, then normalised title. It is stored on the programme, so rules never depend on a stream URL | Identity resolver | Series rule screen shows what matched |
| LIV-047 | Polite fetching for every source | Guide and playlist downloads never get the household banned | Jellyfin no (Schedules Direct blocks it), others unverified | Medium: Schedules Direct's public warning | R3 | Conditional requests, one fetch per source per interval, jittered start times and back-off on errors, all inside the shared fetcher | Fetcher with caching headers and back-off | Source screen shows last fetch and next attempt |
| LIV-048 | HDHomeRun's online guide | Use SiliconDust's guide feed | Jellyfin request (24 votes) | Medium: 24 votes | Later | Possible as a first-party plugin with a network grant, if SiliconDust's terms allow third-party use (unverified) | Plugin | Guide sources |
| LIV-049 | Bundled commercial guide | A postal-code guide with no set-up | Plex yes (with Plex Pass), Emby free for the US, Canada and the UK | High: Plex's first-run advantage | No | Gunmetal has no central account and no guide service to run, so it cannot license data per household | None | None |

### Channel lineup

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIV-050 | Mapping screen | Fix wrong or missing guide matches | Plex yes, Jellyfin yes (search-box request, 3 votes), Emby yes, Dispatcharr bulk edits | Medium: every source needs it | R3 | Search, suggested matches with the reason for each, and a live sample of the channel so the admin can confirm | Match suggestions; search over guide stations | Admin > Channels > Mapping |
| LIV-051 | Automatic guide matching | Most channels match with no manual work | Plex yes (by lineup and postal code), Jellyfin by tvg-id and name, Emby automatic attempt, Kodi three passes | Medium: baseline expectation | R3 | Kodi-style passes (ID, display name, normalised name) plus group and number. Manual mappings always win and survive refreshes | Matching passes in the core | Wizard step; mapping screen shows confidence |
| LIV-052 | Rename channels | Show "BBC One", not a provider's decorated name | Plex no (491 votes, open since 2018), Jellyfin unverified, Dispatcharr and Threadfin find-and-replace | High: 491 votes | R3 | Per-channel names, plus find-and-replace rules that also apply to future refreshes | Display-name field; rename rules | Channel detail; bulk rename rule editor |
| LIV-053 | Reorder and renumber | Channel numbers that make sense | Plex no (the same 491-vote request), Jellyfin default-order request (14 votes), Dispatcharr custom numbering | High: the same 491-vote request | R3 | Numbers are the household's data on the channel, not the source's, so refreshes never reshuffle them | Number field; automatic numbering by group | Lineup editor with drag and number entry |
| LIV-054 | Channel groups | Browse by country, genre or provider | Plex no (86 votes), Jellyfin no (376 votes, marked planned since 2019), Emby groups from the M3U, Channels DVR rule-based Collections | High: Jellyfin's top Live TV request | R3 | Groups from the playlist, manual groups and rule-based groups, synced to every device with the guide | Group model; rule evaluation | Lineup editor; guide filter (LIV-069) |
| LIV-055 | Favourites per person | A short list of the channels each person actually watches | Plex yes on big-screen apps, Jellyfin yes, Channels DVR and TiviMate yes | Medium: baseline in IPTV players | R3 | Parity | Ordered favourites per user | Star in guide and player; Favourites filter |
| LIV-056 | Hide channels | Remove junk without deleting the source | Plex untick during mapping, Jellyfin request (4 votes), Dispatcharr channel states | Low: 4 votes | R3 | Hidden is a channel state that survives refreshes | State field | Lineup editor; bulk hide |
| LIV-057 | Several streams per channel with failover | A channel switches to a backup when the first stream fails | Plex, Jellyfin and Emby no; Dispatcharr switches on detected buffering | Medium: a core reason people run Dispatcharr | R3 | Ordered streams per channel. The broker fails over on connection errors, stalls or continuity errors, and recordings log each switch | Stream list; stall detection | Channel detail > Streams |
| LIV-058 | Channel logos | Correct, consistent logos | Plex from its provider, Jellyfin requests (6 votes; 10 reactions for web defaults), Emby selectable, Dispatcharr uploads | Low: small requests | R3 | Logos from the playlist, the guide or an upload, with a fallback; fetched once by the server as in LIV-045 | Logo cache; precedence order | Channel detail > Logo picker |
| LIV-059 | Bulk editing | Fix hundreds of channels at once | Plex, Jellyfin and Emby no; Dispatcharr and Threadfin multi-select | Medium: one-at-a-time editing fails with large playlists | R3 | Each bulk action is one logged transaction, so it can be undone (LIV-063) | Batch operations | Lineup editor multi-select, by remote, keyboard or touch |
| LIV-060 | Channels keep their identity when the provider changes | A new password or a rotated token keeps every rule, number and favourite | Jellyfin no (channels are re-created and their timers lost, per a third-party tracker); no product documents this | High: part of the recordings-lost pain point | R3 | A channel is Gunmetal's own random-ID object; URLs, tokens and credentials are attributes underneath it. Refreshed playlists are matched onto existing channels by guide ID, then normalised name, then group and number | Matching on refresh; replay tests with captured playlists | Admin alert when a match is uncertain |
| LIV-061 | Missing channels are marked, not deleted | Channels that vanish upstream wait for you instead of breaking things | Jellyfin partial (deletes and re-creates), Dispatcharr keeps stale channels for a set number of days | High: the same pain point | R3 | A missing channel keeps its rules and number, raises an admin alert, and is archived only after a set period | Missing state; retention job | Lineup "Missing" filter; admin alert |
| LIV-062 | Removing a source cleans up | Old channels leave with their source | Jellyfin #2548 (17 reactions, 78 comments), Emby forum thread (26 replies) | Medium: long-running complaints | R3 | Removal first lists the channels, rules and scheduled recordings affected, and offers to move rules to another source | Impact report; re-point operation | Remove-source confirmation |
| LIV-063 | Lineup changes are logged and can be undone | Mapping and bulk-edit mistakes can be reversed, and the lineup survives a cache rebuild | No product documents this | Medium: follows from the recordings-lost pain point | R3 | Names, numbers, groups, mappings and favourites are written to an append-only, exportable log, so the SQLite cache stays rebuildable (needs a new architecture record) | Log entries; undo; export | Admin > Live TV > History, with undo |

### The programme guide as a screen

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIV-064 | A grid guide that is fast with a remote | The channels-by-time grid scrolls instantly and never traps focus | Plex yes, Jellyfin yes with bugs (#7198 Tizen paging, closed; #5705 off-screen navigation, open), Emby yes, TiviMate the reference | High: the central screen of the area | R3 | Drawn from guide data held on the device (LIV-065), so scrolling never waits on the server; designed TV-first and tested with remote-only input | Guide sync endpoint | Guide grid (TV) |
| LIV-065 | Guide synced to the device | Browse 14 days of 500 channels instantly, even with the server unreachable | No rival documents this | Medium: TiviMate's speed is its signature; rivals fetch screens from the server | R3 | The same delta-sync approach as the library (README): a compact guide format sent as changes | Compact format, delta endpoint, a size budget per device class (to be measured) | Every guide surface |
| LIV-066 | Touch and pointer guide | The same guide works on phones, tablets and the web | TiviMate's store listing says it is not optimised for touch; others unverified | Medium: Gunmetal's one UI must serve TV and touch | R3 | One React Native guide with touch gestures and pointer support beside remote focus | Same as LIV-065 | Guide grid (phone, tablet, web) |
| LIV-067 | Now line and quick jumps | See "now", jump back to it, skip a day, pick a time | Plex day filter (24-hour skip request, 10 votes), Jellyfin now-line request (1 vote) | Low: small requests | R3 | Parity | None beyond guide data | Guide header: Now, day picker, time picker |
| LIV-068 | Keep watching while browsing | The current channel keeps playing in a corner or behind the guide | Plex unverified, Jellyfin no (11 votes), TiviMate and Channels DVR (both unverified; a 100-post Channels thread) | Medium: 100-post thread and an 11-vote request | R3 | The libmpv player persists across screens, and preview playback has the lowest broker priority, so it never takes a tuner a recording needs | Broker priority classes | Guide with preview pane or background video |
| LIV-069 | Guide filters | Show only a group, favourites, a category or HD | Plex favourites only (group request, 86 votes), Jellyfin request (14 votes), Channels DVR Collections | Medium: 86 and 14 votes | R3 | Filters run on the device over synced data, so switching is instant | None beyond groups | Guide filter bar |
| LIV-070 | Programme details | Synopsis, episode, cast, rating, flags (new, live, premiere, HD, captions) and other airings | Plex, Jellyfin and Emby yes, detail varies | Medium: baseline | R3 | Parity; "other airings" comes from series identity (LIV-046) | Airings index | Details panel and page |
| LIV-071 | Discovery rows | "On now", "Starting soon", "New tonight", films and sport | Plex yes (seven rows, the richest view), Jellyfin "On Now" row, Channels DVR "On Later" | Medium: Plex's strongest guide feature | R3 | Parity with Plex at best; rows draw only from channels the viewer may watch | Row queries over the synced guide | Live TV home with rows |
| LIV-072 | Search the guide | Find a show, film or person across all channels | Plex yes (guide results in global search), Jellyfin basic, TVHeadend the most powerful | Medium: baseline | R3 | Search runs on the device over the synced guide, so it is instant and works offline; a search can be saved as a rule (LIV-119) | Device search index format | Global search; guide search |
| LIV-073 | Find a channel by name or number | Jump to a channel by typing | Plex request (6 votes), Jellyfin issue closed in 2020 (outcome unverified), TiviMate (unverified) | Low: 6 votes | R3 | Runs on device data, so results appear as you type | None | Guide and player search field; number pad |
| LIV-074 | "Already in your library" marker | Avoid recording what you own | Plex no (23 votes), Jellyfin series option reported as ignored (#13637) | Medium: 23 votes plus a bug report | R3 | Matched by external IDs from library metadata, not by title alone | Guide-to-library matcher | Badge on guide cells and details |
| LIV-075 | Programme reminders | A notice when a show starts, with a button to tune | Plex unverified, Jellyfin no (unverified), TiviMate (unverified) | Low: no vote data | R3 | Reminders fire as local notifications from the synced guide, so no vendor push service is needed | None beyond guide sync | Details "Remind me"; notification with "Watch" |
| LIV-076 | Open straight to the guide | A TV-style launch, with the last guide view remembered | Plex no, Jellyfin requests (15 and 4 votes), TiviMate (unverified) | Low: 15 votes | R3 | Parity | Per-device setting | Settings > Start screen |
| LIV-077 | Live TV on the home screen, by choice | Optional rows such as "On now in favourites" and "New recordings" | Plex pushes its own Live rows (complaints since 2021), Jellyfin has an "On Now" home row | Medium: complaints about live rows that could not be removed | R3 | Rows appear only if the person adds them, and draw only from their own channels and recordings | Row definitions | Home editor |

### Watching live

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIV-078 | Play the broadcast as-is on native clients | MPEG-2, H.264, AC-3 and interlaced channels play without a powerful server | Plex no (its docs say Live TV needs a server that can transcode), Jellyfin partial (a 2020 report said always transcoded; current state unverified), Channels DVR apps decode on the device (unverified) | High: the biggest hardware lever in the area | R3 | Native clients get the transport stream and decode and deinterlace it in libmpv. The server does a network read and a network write | Fan-out buffer serving TS to clients | Player |
| LIV-079 | Live TV in the browser | Channels play in a web browser | Plex, Jellyfin and Emby yes, by transcoding | Medium: the web client is a primary client | R3 | Remux in-process when only the container blocks the browser (H.264 with AAC); otherwise transcode in the network-less sandbox, fed by a pipe. Honest limit: MPEG-2, some AC-3 and interlaced channels cost CPU, and a weak server may manage only one or two at once | Live remux and sandbox pipeline | Web player; capacity warning |
| LIV-080 | Fast channel changes | Zapping feels like a TV | Plex request (9 votes), Emby slow-change thread (61 replies), Channels DVR advertises under a second | Medium: 61-reply thread | R3 | A new viewer of a channel that is already open starts from the last keyframe in the shared buffer. Cold tunes depend on the tuner or provider, and the stats overlay says which case applied | Keyframe index on the live buffer | Player |
| LIV-081 | Channel up, down, last and number entry | Zap with the remote | Plex no (76 and 7 votes), Jellyfin number-entry request (1 vote) | Medium: 76 votes | R3 | Parity with a TV box | Per-user channel order | Player: remote keys, on-screen number pad |
| LIV-082 | Mini guide over playback | A channel list or now-and-next strip without leaving the picture | Jellyfin request (28 votes), TiviMate (unverified) | Medium: 28 votes | R3 | Drawn from synced guide data, so it opens instantly | None beyond sync | Player overlay |
| LIV-083 | Captions and broadcast subtitles | CEA-608 and 708 captions, DVB and teletext subtitles | Plex no (its live TV article says captions are unsupported), Jellyfin DVB subtitles not selectable (#10914, open), Emby forum thread (14 replies), Channels DVR full captions | High: an accessibility need, and Plex has none | R3 | libmpv renders captions and DVB subtitles on the device, so captions never force a transcode. The browser path must extract captions to text tracks during remux (still to be built) | Caption extraction in the web remux path | Player captions menu |
| LIV-084 | Alternate audio and surround | Second language, 5.1 and audio description tracks | Plex no (the same article), Channels DVR 5.1 | Medium: listed by Plex as unsupported | R3 | Every audio track passes through and the client picks by the person's preferences | Track list from TS program tables | Player audio menu |
| LIV-085 | Deinterlacing on the device | A clean picture for 1080i channels | Jellyfin request (1 vote) | Low: 1 vote | R3 | libmpv deinterlaces on the client at no server cost (quality on low-end Android TV unverified) | None | Player settings |
| LIV-086 | Stats overlay | Codec, bitrate, tuner, source, errors, and whether it is direct, remuxed or transcoded | Partial across rivals; Jellyfin's Android TV "stats for nerds" request has 219 votes (general playback) | Medium: general demand for playback diagnostics | R3 | The decision engine reports why it chose each path, and the live buffer reports continuity errors | Session diagnostics | Player overlay |
| LIV-087 | Tuner-busy choices | When every tuner is busy, see who has each one and choose | No product documents this | Medium: follows from tuner and connection contention | R3 | The broker reports who holds each tuner and what could be freed; recordings are never offered for takeover | Broker status API | Player sheet with choices |
| LIV-088 | Record what you are watching | Press record mid-programme and keep the part already watched | Plex no (stop playback first), Channels DVR (unverified) | Medium: a documented Plex limitation | R3 | The recording starts from the earliest point in the rolling buffer, not from the button press | Buffer-to-recording promotion | Player record button |
| LIV-089 | Picture-in-picture | Keep a channel playing while using other apps | Jellyfin requests (desktop 22 votes, Android TV 5), Plex unverified | Medium: 22 votes | R3 | Each platform's own picture-in-picture with the same libmpv player used for video | None | Player PiP button |
| LIV-090 | Casting live TV | Send a channel to a Chromecast or AirPlay device | Plex partial (iOS and web only; 379-vote request), Jellyfin and Emby unverified | Medium: 379 votes | R3 | Channels the cast device can decode are remuxed; others go through the sandbox. Depends on general casting arriving with video in R2 | Cast session with live remux | Player cast button |
| LIV-091 | Watch your tuners away from home | Live TV over the internet with no port forwarding and no pass | Plex needs Plex Pass or Remote Watch Pass for remote playback (whether that covers live TV is unverified), Jellyfin self-managed, Channels DVR "Stream While Away" with hardware transcoding | Medium: the remote paywall is the top complaint theme overall | R3 | Remote access over iroh with no vendor account. HLS sources with lower-bitrate variants are switched rather than transcoded; antenna channels on slow links need a sandboxed transcode or are refused with a clear reason | Bandwidth estimate; variant choice; optional transcode | Player quality menu; remote warnings |
| LIV-092 | Sleep timer | Playback stops after a set time | Plex no (6 votes) | Low: 6 votes | R3 | Parity; shared with the main player | None | Player menu |
| LIV-093 | Low-latency mode | Less delay behind the broadcast, for sport | Jellyfin request (3 votes) | Low: 3 votes | Later | A smaller client buffer when the network is steady | Buffer tuning | Player setting |
| LIV-094 | Multi-view | Two to four channels on one screen | Plex no (34 votes), Jellyfin 1-vote request, Channels DVR on Apple TV 4K and iPad only, TiviMate Premium on Android TV | Medium: 34 votes; each rival covers one platform family | Later | Several libmpv players on one screen. Each tile has a lower broker priority, so it cannot starve a recording, and the tile count follows a per-device decoder probe | Tile priority in the broker; decoder capability data | Multi-view layout picker |

### Pause, start-over and catch-up

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIV-095 | Pause and rewind live TV | Pause for the door; replay a goal | Plex yes (since server v1.16.1), Jellyfin no (8 votes, open since 2022), Emby buffer setting (pause behaviour unverified), Channels DVR the best experience | Medium: Jellyfin's gap and a Plex strength | R3 | One rolling buffer per channel, with a keyframe index built as bytes arrive, shared by every viewer of that channel | Rolling buffer writer; index; clean-up job | Player timeline, pause, skip back |
| LIV-096 | Buffer length and storage | Choose how far back you can go, and spare SD cards and USB disks | Plex no (10 votes), Emby buffer limit, TVHeadend caps by time and size, in RAM or on disk | Low: 10 votes | R3 | Time and size caps, a RAM-only mode, and a disk-write budget for small hardware | Write budget; RAM ring buffer | Admin > Live TV > Time-shift |
| LIV-097 | Start over | Watch a programme that began 20 minutes ago from its start | Plex no ("watch from beginning while recording", 6 votes, 21 posts), Jellyfin through a third-party plugin (September 2026), Channels DVR yes | Medium: 21-post thread | R3 | Guide boundaries are mapped onto the buffer or recording index, so starting over is a seek | Programme-to-offset lookup | Player "Start over"; guide action |
| LIV-098 | Watch a recording in progress | Start a recording before it finishes, and catch up to live | Plex unverified ("in progress" hub request, 18 votes), Jellyfin no (12 votes, open since 2021), Channels DVR yes | Medium: 18 and 12 votes | R3 | An incremental segment map over the growing file: record 1's segment map applied to a live write | Growing-file index | Recordings > In progress; player "Go to live" |
| LIV-099 | Provider catch-up | Play past programmes the provider still holds | Plex no, Jellyfin no (15 votes), Emby no (staff said in 2021 it would not ship), Kodi IPTV Simple (seven modes), TiviMate, Dispatcharr replay | Medium: 15 votes; a staple of IPTV players | R3 | Parity with Kodi's catch-up attributes. Catch-up URLs are built on the server, so credentials never reach clients | Catch-up URL templates; past guide retention | Guide catch-up icon; details "Play from start" |
| LIV-100 | Past programmes marked in the guide | See at a glance what can still be played | TiviMate catch-up icon; Plex, Jellyfin and Emby no | Medium: the same demand as LIV-099 | R3 | The guide scrolls back over retained days and marks items playable from catch-up, the buffer or a recording | Availability flags per programme | Past area of the guide grid |
| LIV-101 | Rolling recording of everything watched | Any channel you watched today can be rewound later | Jellyfin request (6 votes), Emby no, TVHeadend comes closest | Low: 6 votes | Later | Buffers kept per channel after viewers leave, within the disk budget | Buffer retention policy | Recordings > Recently watched channels |

### Recording: one-off and manual

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIV-102 | Record a programme from the guide, free | One press to record | Plex paid (Plex Pass), Jellyfin free, Emby paid (Premiere), Channels DVR paid | High: DVR paywalls on Plex and Emby are a recurring complaint | R3 | Parity with Jellyfin on price (its DVR is also free and needs no account); ahead of Plex, which needs Plex Pass. The edge is in the scheduler rows (LIV-104, LIV-122, LIV-132, LIV-136). | Recording object; scheduler; recorder | Guide and details "Record" |
| LIV-103 | Manual recording by channel and time | Record without guide data, once or on chosen weekdays | Plex no (124 votes), Jellyfin request (4 votes), TVHeadend and Dispatcharr rules by day and time | High: 124 votes | R3 | Manual recordings are ordinary recording objects on real or placeholder slots | Time-range recordings; repeats | Channel menu "Record a time"; recording form |
| LIV-104 | Padding | Never miss the first or last minutes | Plex yes (global and per recording; "if available" request, 7 votes), Jellyfin yes (global and per rule, with "required" flags), Emby yes | Medium: baseline | R3 | Required or "if available" padding. Back-to-back recordings on one channel share one upstream, so overlapping padding does not need a second tuner | Padding in the scheduler; shared upstream | Recording options; admin defaults |
| LIV-105 | Extra padding by category | Sport and live events get more end padding automatically | Plex, Jellyfin and Emby manual only (Emby's docs suggest 30 to 60 minutes by hand) | Medium: overruns are a common cause of missed endings | R3 | Default padding per category, applied by rule and shown on the recording | Category padding table | Admin > Recording defaults |
| LIV-106 | Follow the broadcaster's running signal | A recording extends when the broadcast overruns | TVHeadend uses running-status signals; Plex, Jellyfin and Emby no | Medium: the same pain as LIV-105 | Later | Read the running-status field in DVB event data where it is broadcast (an ATSC equivalent is unverified) | Event-table watch during recording | Recording shows "extended" |
| LIV-107 | Schedule from any device, including phones | Set a recording from the phone on the bus | Plex dropped DVR scheduling from its 2025 mobile redesign, per press coverage; Jellyfin and Emby unverified | Medium: listed among the features Plex removed | R3 | One UI codebase, so scheduling is the same on every client, with remote access over iroh | Same scheduling API | Guide and details on mobile |
| LIV-108 | Where recordings go | Films and shows land in the right library and folder | Plex picks a library per recording, Jellyfin separate film and series paths (per-recording location request, 1 vote), Emby default library per type | Medium: baseline | R3 | Parity | Destination rules; library link | Admin > Recording destinations; record dialog |
| LIV-109 | Clear stop and cancel | "Stop and keep" is never confused with "Cancel and delete" | Jellyfin request to rename "Cancel recording" (5 votes) | Low: 5 votes | R3 | Two distinct actions with distinct wording; deleting goes to the trash (LIV-136) | None | Recording controls |
| LIV-110 | Wake the server to record | A sleeping low-power server still records | Plex no (210 votes), Emby yes, Jellyfin no (unverified) | Medium: 210 votes, and it matters most on low-power servers | R3 | The server publishes its next required wake time, and the docs ship recipes for setting the hardware wake timer, so the server itself holds no extra privilege (an unprivileged route on each platform is unverified) | Next-wake calculation exposed by API and file | Admin > Power: next wake time and instructions |
| LIV-111 | "Your recording is ready" | The person who scheduled a recording hears when it finishes | No live-TV evidence; Jellyfin requests for show subscriptions (50 votes) and mobile push (48 votes) | Low: adjacent requests only | R3 | Sent over the same notification path as admin alerts, with no vendor relay where the platform allows | Per-user notification preferences | Notification; Settings > Notifications |

### Series and rule-based recording

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIV-112 | Series recording: new episodes or every airing | Repeats are not recorded twice | Plex yes, Jellyfin yes, Emby yes (detail unverified), TVHeadend several duplicate modes | High: the baseline DVR feature | R3 | Duplicates are detected by series and episode identity (LIV-046) and by the history of what was recorded, kept in the log | Rule engine; duplicate check | Series rule screen |
| LIV-113 | Any channel or one channel | Follow a show across channels, or pin it to one | Plex, Jellyfin and Emby yes | Medium: baseline | R3 | Parity; channel references are random IDs, so they survive source changes | Rule field | Series rule screen |
| LIV-114 | Time window and days | Only the evening airing, only weekdays | Plex unverified, Jellyfin yes, TVHeadend start-after, start-before, days and duration bounds | Medium: baseline | R3 | Parity | Rule fields | Series rule screen |
| LIV-115 | Keep limits | Keep the last N episodes, or the last N days | Plex yes (improvement request, 32 votes), Jellyfin yes | Medium: 32 votes | R3 | Limits act through logged deletions into the trash, never silent removal | Retention job | Series rule screen |
| LIV-116 | Delete after watching, for the whole household | Space is freed only when the right people have watched | Plex yes, but it follows the admin's watch state even when others have not watched (its own docs warn of this); others unverified | Medium: a documented Plex flaw | R3 | The rule chooses "anyone", "everyone" or named people, read from each person's watch log | Per-user watch state per recording | Series rule screen |
| LIV-117 | Skip what is already in the library | Episodes you own are not recorded | Plex yes (re-record option request, 13 votes), Jellyfin option reported as ignored (#13637) | Medium: request plus a bug report | R3 | Matched by external IDs (LIV-074), with an option to re-record when the airing is better quality | Library matcher | Series rule screen |
| LIV-118 | Resolution preference | HD only, or prefer HD | Plex yes (and warns it can replace library items), others unverified | Low: no vote data | R3 | Prefers the HD airing and never replaces a library file automatically | Rule field | Series rule screen |
| LIV-119 | Keyword and smart rules, with a preview | Record anything matching a word, person, genre, rating, year or length | Plex no (55 votes), Jellyfin no (1-vote title-rule request), TVHeadend auto-recording rules the deepest, Channels DVR smart rules | Medium: 55 votes | R3 | Rules are pure functions in the core, evaluated against guide diffs and property-tested. Before saving, a preview lists the airings the rule would record | Rule evaluator; preview endpoint | Rule builder with live preview; "Save search as rule" |
| LIV-120 | Skip one airing | Opt out of one episode without touching the rule | Plex no (25 and 11 votes) | Medium: 25 votes | R3 | A skip is a logged exception on the rule | Exception list | Upcoming list "Skip this one" |
| LIV-121 | Season and episode filters | Only season 5 onward | Plex no (11 votes), Jellyfin no, TVHeadend minimum and maximum season | Low: 11 votes | R3 | Parity | Rule fields | Series rule screen |
| LIV-122 | Re-record after errors | A damaged recording is retried at a later airing | Plex no (unverified), Jellyfin no (unverified), TVHeadend error thresholds and re-scheduling | Medium: tied to the silent-failure pain point | R3 | The recorder's error score (LIV-153) feeds the rule, which books the next airing and keeps the damaged copy until the new one succeeds | Error thresholds; retry scheduling | Recording detail "Will retry on ..." |
| LIV-123 | Better slot search | A rule moves to another airing to avoid a clash | TVHeadend yes; Plex and Jellyfin no | Low: no vote data | Later | The scheduler searches other airings across the guide horizon | Search in the scheduler | Conflict screen suggestion |
| LIV-124 | Team passes | Record every game a team plays | Channels DVR Team Pass; Plex, Jellyfin and Emby no | Low: no vote data | Later | Depends on team data in guide sources (availability unverified); keyword rules cover part of the need in R3 | Team matching | Sports rule screen |

### Conflicts, tuners and connection sharing

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIV-125 | One upstream per channel, shared | Two viewers and a recording on one channel use one tuner or connection | Jellyfin per-tuner sharing switch (a 2021 issue said shared viewers were no longer live; one-stream request, 3 votes), Channels DVR and Dispatcharr yes | High: the connection-limit pain point | R3 | A fan-out buffer per channel serves every reader from the last keyframe, with back-pressure so one slow client cannot stall the rest | Fan-out buffer; reader management | Stats overlay shows a shared stream |
| LIV-126 | Provider connection limits respected | Never get banned for opening too many streams | Jellyfin and Emby a simple limit per playlist, Dispatcharr pooled limits | High: providers cap connections, often at a handful (unverified) | R3 | Limits counted per login and provider, including mirrors and catch-up streams, before any connection opens | Limit counters in the broker | Source > Connection limit |
| LIV-127 | Recordings come first | A recording is never lost to someone zapping | No product documents a clear policy | Medium: follows from the recording-loss pain point | R3 | A fixed priority order: recordings, then live viewers, then previews and multi-view tiles. Viewers are warned before a scheduled recording needs their tuner | Priority classes; pre-emption warnings | Player warning banner |
| LIV-128 | Conflict warning when scheduling | Know at once that a recording will not happen | Plex yes (Cancel, Prefer and Manage), Jellyfin and Emby unverified | Medium: Plex's clear flow is a strength | R3 | Conflicts are computed from tuner counts, connection limits and padding, and the choices are explained in plain words | Conflict calculation | Record dialog; Conflicts screen |
| LIV-129 | Rule priority | Decide which rule wins a clash | Plex drag to reorder, Jellyfin priority field ("scheduling priority" request, 17 votes) | Medium: 17 votes | R3 | Parity | Priority order | Rules list, reorder by drag or remote |
| LIV-130 | Calendar and agenda views | See the week's recordings | Plex both, Jellyfin a schedule list | Medium: baseline | R3 | Parity | Upcoming-recordings query | Recordings > Upcoming, agenda and week views |
| LIV-131 | Start a scheduled recording now | Act on a planned item early | Plex no (22 votes) | Low: 22 votes | R3 | Parity | Start-now operation | Upcoming list action |
| LIV-132 | Reservations and pre-flight checks | Tuners, connections and disk space are reserved and checked before each recording | No product documents this | Medium: tied to silent failures | R3 | A few minutes before the start, the server checks the source, a tuner or connection, the write path and free space, and alerts while there is still time to act | Pre-flight job | Admin alert; recording status "Ready" |

### Recording storage and retention

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIV-133 | Record the original transport stream | Recordings cost no CPU and lose nothing | Plex and Jellyfin record MPEG-TS, TVHeadend TS or Matroska | Medium: baseline | R3 | Parity on format; every audio and caption track is kept | TS writer | None |
| LIV-134 | Lossless repackaging to Matroska | Smaller, seek-friendly files with chapters, without re-encoding | TVHeadend writes Matroska; others unverified | Low: no vote data | R3 | An in-process remux after the recording ends, using the R2 remuxer. It keeps every track and adds chapters at markers; the TS file is kept if anything fails | Post-recording remux job | Admin > Recording format |
| LIV-135 | Recordings are ordinary library items | Recordings sit with the rest of your shows | Plex yes (chosen library), Jellyfin a recordings library, Emby a mixed library | Medium: baseline | R3 | Matched to library metadata and watch state, and protected by LIV-136 | Library linking | Library and show pages include recordings |
| LIV-136 | Recordings never vanish | Files are deleted only by a rule or a person, and can be restored | Jellyfin #17622 (finished recordings deleted during library scans, open since August 2026); no product documents a guarantee | High: the recording-loss pain point | R3 | Library scans never delete recordings. Every deletion is logged with its cause, and files go to a trash folder for a set period (needs a new architecture record) | Trash folder; deletion log; restore | Recordings > Trash, with restore |
| LIV-137 | Free-space management | Old recordings are pruned before the disk fills | Plex no (28 votes), TVHeadend free-space and size caps, Channels DVR Auto Prune | Medium: 28 votes | R3 | Pruning follows a visible order (watched, oldest, lowest priority) and is logged, and a recording never starts without its reserved space | Space monitor; prune job | Admin > Storage |
| LIV-138 | File naming templates | Names that suit your other tools | Plex fixed, Jellyfin requests (2 and 2 votes), TVHeadend format strings | Low: 2-vote requests | R3 | Logic-less templates that produce names safe on every filesystem | Template renderer | Admin > Recording naming |
| LIV-139 | Resume and watched state for recordings | Recordings remember where you stopped | Plex request (25 votes), Jellyfin yes (unverified) | Medium: 25 votes | R3 | Positions go into the append-only watch log, as for all media | Watch log | Player resume prompt |
| LIV-140 | Recording events for automation | Scripts and home automation learn when a recording starts, finishes or fails | Plex, Jellyfin and Emby run post-processing scripts; TVHeadend pre-, post- and remove commands | Low: no vote data | R3 | Events on the server's event stream and webhooks. Scripts run outside the server, so admin code never runs with server privileges | Event emission | Admin > Webhooks |
| LIV-141 | Recordings hub | One place for in progress, recent, by show, failed and scheduled | Plex hubs ("in progress" request, 18 votes), Jellyfin a recordings library | Medium: 18 votes | R3 | Health and failure reasons sit beside each recording | Queries by state | Recordings screen with tabs |
| LIV-142 | Shrink recordings | Convert to a smaller codec in the background | Plex experimental (6-vote request), others through scripts | Low: 6 votes | Later | Sandboxed, low-priority and off by default; the original is kept until the copy is verified | Transcode job in the sandbox | Admin > Recording format |
| LIV-143 | Trim a recording | Cut the pre-roll and the overrun | Plex no (8 votes), Jellyfin no | Low: 8 votes | Later | Non-destructive in and out points first; lossless keyframe cuts later | Trim markers | Recording edit screen |
| LIV-144 | Sidecar metadata files | NFO files and images beside recordings | Jellyfin yes, on by default | Low: no vote data | Later | An optional export for other tools | NFO writer | Admin setting |
| LIV-145 | Scripts run by the server | Run your own program after each recording | Plex, Jellyfin and Emby yes | Low: no vote data | No | Running arbitrary programs inside the server undercuts the sandbox design; LIV-140 events cover the need | None | None |

### Commercial markers

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIV-146 | Import EDL markers and skip | Skip ad breaks found by an external detector | Plex EDL request (opened March 2026), Jellyfin third-party plugin reads EDL files | Medium: a 164-post Channels DVR thread shows the appetite for skipping | R3 | Markers are read from EDL files beside the recording and never alter the file, so households can run a detector of their choice outside the server | EDL reader; marker store | Player "Skip break" button |
| LIV-147 | Automatic skip per person | Breaks skip with no button press, for those who want it | Plex no (Skip Ads button only; auto-skip request closed), Channels DVR yes | Medium: a Channels DVR strength | R3 | A per-user preference applied by the player | Per-user setting | Settings > Playback |
| LIV-148 | Built-in commercial detection | Breaks found automatically | Plex yes (skipping needs Plex Pass on the viewer's account), Jellyfin third-party plugin, Emby no built-in (unverified), Channels DVR the leader (Enhanced+ in pre-release) | Medium: long Channels DVR threads; Plex charges for it | Later | Runs in the sandbox at low priority and can be switched off on weak hardware. It aims at Plex's non-destructive design, not Channels-level accuracy | Detector job in the sandbox | Admin > Commercial detection |
| LIV-149 | Detection while recording | Start a match late and skip ads to catch up | Plex no, Jellyfin plugin v1.1.0.0, Channels DVR thread (164 posts) | Medium: 164 posts | Later | Incremental detection over the growing file | Incremental detector | Player |
| LIV-150 | Edit markers | Fix bad detection by hand | Plex no; Jellyfin plugin reads EDL only | Low: no vote data | Later | Edits are logged and override detection | Marker edit API | Recording edit screen |
| LIV-151 | Detection on ordinary TV files | Skip ads in recordings made elsewhere | Plex yes (optional) | Low: no vote data | Later | The same detector, run on library items by choice | Detector job | Library item menu |
| LIV-152 | Remove commercials from the file | Smaller files with no ads | Plex optional, off by default because detection is imperfect | Low: no vote data | No | It destroys part of an irreplaceable recording on the strength of imperfect detection; markers give the same viewing result | None | None |

### Reliability and alerts

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIV-153 | Recording health record | Each recording knows its gaps, errors and how complete it is | Plex no (9 to 15 minute partials reported in October 2025), TVHeadend data-error thresholds | High: silent partial recordings | R3 | Tracks continuity errors, gaps, disconnects, bytes written and expected length, and stores a reason when a recording is short | Recorder metrics | Recording detail health panel |
| LIV-154 | Reconnect and fail over during a recording | A dropped stream resumes, with the gap marked | No product documents this for recordings; Dispatcharr fails over for viewing | High: the same pain point | R3 | The recorder reconnects, then tries mirrors, other streams and other tuners, and marks the gap instead of ending. It is a state machine with an injected clock, so it can be tested deterministically | Recording state machine | Recording timeline showing gaps |
| LIV-155 | Alerts on failed or partial recordings | Find out today, not when you sit down to watch | Plex no (users find failures afterwards), Dispatcharr notifications | High: the same pain point | R3 | Alerts reach the admin's own devices with the reason and what happens next, without a vendor relay where the platform allows | Notification path | Notification; Admin > Alerts |
| LIV-156 | Source and guide alerts | Know when a playlist returns 403 or the guide goes empty | Dispatcharr refresh-error events (v0.31.0) | Medium: refresh failures are common | R3 | Each refresh result is recorded with its cause and the number of channels affected | Refresh result log | Admin > Alerts; source status |
| LIV-157 | Recordings survive a restart | A crash or update mid-recording resumes into the same recording | Plex threads report recordings stopping until a restart | Medium: 2025 Plex threads | R3 | Recorder state is persisted; on start-up, recordings in progress resume and mark the gap | Persistent recorder state; resume on boot | Recording detail shows the gap |
| LIV-158 | Time handled correctly | No missed recordings at daylight-saving changes or timestamp wraps | Emby forum thread on series not recording because of a wrong date (53 replies) | Medium: 53 replies | R3 | An injected clock, property tests across time-zone and daylight-saving transitions, and handling of the 33-bit broadcast timestamp wrap | Time handling in the core | None |
| LIV-159 | Upgrades do not break recording | Recording keeps working across versions | Jellyfin 10.9.2 broke M3U recording outright (#11756); Plex partials persisted across beta and stable builds | Medium: two documented regressions | R3 | Rules replay from the log after any upgrade, and a corpus of captured streams and guides runs in CI | Regression corpus | None |
| LIV-160 | A Live TV log anyone can read | Diagnose problems without shell access | Plex, Jellyfin and Emby server logs only; Dispatcharr in-app log browser (v0.31.0) | Medium: a Dispatcharr selling point | R3 | Event history per source and per recording inside the app, with credentials redacted | Event history store | Admin > Live TV > Activity |
| LIV-161 | Live sessions for channels and tuners | See who is watching what, on which tuner, at what bitrate, and stop a stream | Plex, Jellyfin and Emby general session dashboards; Dispatcharr real-time stream dashboard; Jellyfin "kill a stream" request (402 votes) | Medium: 402-vote request (general) | R3 | Shows shared upstreams, connection counts against limits and buffer use, beside ordinary sessions | Session and broker statistics | Admin > Dashboard > Live TV |

### Access, privacy and security

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIV-162 | Separate rights to watch, schedule and manage | Family members can watch, schedule or manage recordings as you decide | Plex partial (Plex Home members only, and they cannot schedule), Jellyfin per-user switches for live TV and recording management, Emby unverified | Medium: Plex's restriction is a documented complaint | R3 | Separate rights for watching, scheduling, managing rules and deleting, checked by the same per-object authorisation layer as everything else | Rights model | Admin > Users > Live TV rights |
| LIV-163 | Per-channel access and channel profiles | Children see some channels; guests see another lineup | Plex no, Jellyfin no ("assign users to tuners", 34 votes; per-tuner access, 5), Dispatcharr profiles and user levels | Medium: 34 votes | R3 | Channels are objects with their own permissions (record 1, decision 6); profiles are named channel sets granted to people | Channel permissions; profiles | Admin > Channel profiles; user detail |
| LIV-164 | Parental limits by rating | Block programmes above a rating, with a PIN to override | Plex through sharing restrictions, Jellyfin user policy (live TV detail unverified), Emby tag exclusions reported failing for live TV, TiviMate parental controls | Medium: a documented Emby failure | R3 | Guide ratings are checked per programme at play time, and channels without ratings fall back to channel permissions. Guide ratings are often missing, and the screen says so | Rating map; play-time check | Block screen in the player; profile settings |
| LIV-165 | Signed, short-lived stream URLs | Nobody can watch your tuners by guessing a URL | Jellyfin #13984 found unauthenticated streams (the live TV part was fixed in 10.8.9); Plex token-based (detail unverified) | Medium: 18 reactions on #13984 | R3 | Every live, buffer and recording URL is short-lived, signed and bound to one person and one object (record 1, decision 6) | URL signing | None |
| LIV-166 | Personal and household recordings | Each person's schedules and recordings stay theirs unless shared | No evidence in research | Low: follows from letting people other than the admin schedule | R3 | Recordings and rules have an owner and a visibility, under per-object authorisation | Ownership fields | Recordings filter "Mine" and "Household" |

### Radio

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIV-167 | Radio channels open in the music player | Audio-only stations in a playlist play with the now-playing bar and lock-screen controls | Plex no, Jellyfin no ("Live radio tab", 12 votes), Kodi IPTV Simple radio flag | Low: 12 votes | R3 | Radio-flagged and audio-only channels hand off to the music player from R1, with background playback | Audio-only detection; radio channel type | Music > Radio; now-playing bar |
| LIV-168 | Internet radio stations | Play station URLs alongside your library | Plex requested (272 votes; a separate request closed with 77 posts), Navidrome yes (admin-managed stations), Lyrion and foobar2000 | Medium: 272 votes | R3 | Uses the same guarded fetcher as IPTV, so a station URL cannot reach the LAN. Which area owns it is an open decision | Station list; audio stream proxy | Music > Radio; station editor |

### Channels from your own library

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIV-169 | Virtual channels on Gunmetal clients | Your shows and films run as always-on channels with a guide | Plex no ("PseudoTV", 622 votes), Jellyfin request (8 votes), Channels DVR virtual channels (seven ordering modes), Tunarr and ErsatzTV (unverified) | High: 622 votes; Tunarr drew 209 points on Hacker News | R3 | The server computes a schedule and publishes it as guide entries. Gunmetal clients play the original files at the scheduled offset, so no continuous transcode runs | Schedule generator; guide entries; ordering modes | Virtual channel editor; guide; player |
| LIV-170 | Virtual channels as streams for other apps | Feed virtual channels to TiviMate, Kodi or another server | Channels DVR exports channels; Tunarr (unverified) | Low: no vote data | Later | Needs one continuous stream joined from files: remuxed when codecs match, through the sandbox otherwise | Playout stream builder | Export settings |

### Interoperability and things outside the module

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| LIV-171 | Export the curated lineup | Use Gunmetal's cleaned lineup in TiviMate, Kodi or another app, for example during a migration | Plex no, Jellyfin and Emby no (unverified), Dispatcharr M3U, XMLTV and Xtream output, Channels DVR exports | Medium: Dispatcharr's popularity | Later | M3U and XMLTV with per-device, revocable, live-only credentials, never an admin token | Export endpoints; device credentials | Admin > Export |
| LIV-172 | HDHomeRun emulation | Plex, Emby or Jellyfin can use Gunmetal as a tuner | Dispatcharr, Threadfin and xTeVe yes | Medium: Plex's missing M3U support created this category | Later | Uses the same device credentials as LIV-171 | Emulated device endpoints | Admin > Export |
| LIV-173 | Live TV through the Jellyfin adapter | Jellyfin apps, including on Roku, show channels, guide and recordings | Not applicable to rivals | Low: no direct vote data; Roku has no Gunmetal client (record 1, decision 6) | Later | The adapter maps onto the same broker and permissions and must not bypass per-object checks | Adapter endpoints | Jellyfin clients |
| LIV-174 | Built-in channels or a free streaming catalogue | Channels supplied by the project | Plex runs its own free live TV | Low: no vote data | No | Gunmetal has no central account and stays content-neutral; it ships no channels or playlists | None | None |
| LIV-175 | TV Everywhere provider logins | Watch cable channels with a pay-TV login | Channels DVR yes; a Plex request exists (count not recorded) | Low: no vote data | No | Large and outside this module; the research names it as scope creep | None | None |

## Differentiators

These are the features in this area most likely to make someone switch.

1. **Antenna TV without a transcoding server (LIV-078, LIV-024).** Plex's own
   documentation says Live TV needs a server that can transcode, because
   over-the-air MPEG-2 usually has to be converted. Gunmetal's native clients
   decode the broadcast in libmpv, so a cheap box with an HDHomeRun serves the
   whole house. This is the clearest hardware win in the area. It holds only
   on native clients; browser live TV still costs CPU, and the
   documentation must say so.
2. **The channel control people have asked for since 2017 (LIV-037,
   LIV-052, LIV-053, LIV-054, LIV-081).** Several guide sources per channel
   (611 votes), rename and reorder (491), channel groups (376 on Jellyfin) and
   channel up and down (76) are the most-voted Live TV gaps on both big boards.
   None is technically hard. Shipping all of them on day one, free, is the
   cheapest win in the module.
3. **Recordings you can trust (LIV-044, LIV-060, LIV-061, LIV-136,
   LIV-153 to LIV-155, LIV-122).** Rivals lose recordings to guide refreshes
   (Jellyfin #12979), library scans (#17622), rotated provider tokens and
   unexplained partials (Plex, 2025). Gunmetal keeps channel identity separate
   from URLs, applies every refresh as a diff, keeps rules and the recordings
   list in an append-only log, deletes only by rule into a trash folder, and
   has a recorder that reconnects, marks gaps, alerts and re-records. No rival
   documents this as a guarantee.
4. **One stream per channel, within the provider's limits (LIV-125,
   LIV-126, LIV-127, LIV-012, LIV-057).** Households hit IPTV connection
   caps because servers open one upstream per viewer and per recording. A
   broker that shares one upstream per channel, counts limits across logins
   and mirrors, puts recordings first and fails over between streams does
   what people now run Dispatcharr in front of their server for.
5. **IPTV without handing your network to a playlist (LIV-004, LIV-014,
   LIV-015, LIV-165).** Jellyfin's Live TV M3U tuner was a route to SSRF and
   file reads in 2026, and the proxy tools pass streams to FFmpeg or VLC with
   network access. Gunmetal parses streams in memory-safe Rust, keeps FFmpeg
   off the network, checks every outbound connection against declared hosts,
   keeps provider credentials on the server and gives clients only
   short-lived signed URLs.
6. **A free DVR with pause, start-over and watch-while-recording (LIV-102,
   LIV-095, LIV-097, LIV-098).** Recording needs Plex Pass on Plex and
   Premiere on Emby. Jellyfin is free but has no pause, rewind or in-progress
   playback. Gunmetal offers all of it free, sharing one buffer per channel.

## Deliberately not doing

- **Stalker and MAC portals (LIV-021).** They work by imitating set-top
  boxes, are tied to unlicensed services, and would invite a piracy
  reputation that could hurt app-store listings (unverified).
- **Redirect mode (LIV-022).** Sending clients straight to the provider
  leaks credentials and bypasses connection counting and per-channel
  permissions. The broker is the only path to upstream streams.
- **Xtream video-on-demand catalogues and TV Everywhere logins (LIV-023,
  LIV-175).** Each is a large product of its own, outside live TV.
- **Driving DVB cards directly (LIV-032).** That is kernel and driver work
  needing privileges the server should not hold. TVHeadend is the supported
  route for satellite, cable and DVB hardware.
- **DRM circumvention and descrambling (LIV-033, LIV-034).** There is no
  legitimate route for encrypted ATSC 3.0, and descrambling carries legal
  exposure that varies by country.
- **A bundled guide or channel catalogue (LIV-049, LIV-174).** There is no
  central account and no service to run, and the project stays
  content-neutral: it ships no playlists, channel lists or guide URLs.
- **Scripts run inside the server (LIV-145).** Events and webhooks
  (LIV-140) let admins run whatever they like outside the server.
- **Cutting commercials out of recordings (LIV-152).** Detection is
  imperfect, and recordings are irreplaceable. Markers give the same viewing
  result without destroying anything.
- **CableCARD and QAM cable tuners (noted under LIV-024).** The research
  calls recording unencrypted US cable a shrinking use case, with Channels
  DVR the leader; encrypted cable channels fall under the descrambling rule
  above. A CableCARD HDHomeRun that exposes unencrypted channels as plain HTTP
  streams may work through LIV-024 (unverified), but nothing is built for it. Other DVR back ends are reached through their M3U and XMLTV
  exports instead (LIV-176).
- **FFmpeg, VLC, Streamlink or yt-dlp as network-facing stream engines.**
  The proxy tools use them, but record 1 keeps FFmpeg in a sandbox with no
  network, and the same reasoning applies to the others.

## Dependencies and risks

- **Blocking: ADR 3, durable user state.** ADR 1 (decision 5) names watch
  history as the only irreplaceable data, but most rows in this map that
  write user data (playlists, loves, ratings, corrections, layouts, shares,
  identities, IDs, settings) need it to survive a cache rebuild. ADR 3 must
  be accepted before any server work that stores user data. It defines the
  two durable stores every map now names the same way: **the user log** and
  **the identity store** (see the [feature map README](README.md)).

**Dependencies**

- **R2 video work.** The remuxer and the sandboxed transcoder are needed for
  browser live TV (LIV-079), casting (LIV-090), Matroska repackaging
  (LIV-134), AC-4 audio conversion (LIV-031) and the library features that
  recordings join (LIV-135, LIV-074, LIV-117). The remuxer is already the main
  schedule risk in record 1, and live TV inherits that risk.
- **New core parsers under the full gate.** The module needs an MPEG-TS
  demuxer, a live HLS client, M3U and XMLTV parsers, and ATSC event-table
  parsers in the core, all with 100% coverage, zero surviving mutants and no
  panics. That needs large corpora of real captures and playlists for tests
  and fuzzing.
- **Two new architecture records.** Record 1 says only watch history is
  irreplaceable. Rules, lineup edits, the recordings list and deletions must
  join it (LIV-063, LIV-136), or the "rebuildable cache" claim becomes false.
  Record 2's network grants must be extended from plugins to sources,
  including LAN tuners (LIV-015).
- **Client players.** Native as-broadcast playback assumes libmpv on Android
  TV, Apple TV and desktop decodes MPEG-2 and H.264 and deinterlaces 1080i
  well on target devices. Performance on low-end Android TV boxes is
  unverified.
- **Notifications.** Recording alerts (LIV-155) and "recording ready"
  (LIV-111) need a path to devices. iOS push goes through Apple's service,
  which sits awkwardly with "no central account"; reminders (LIV-075) avoid
  this by firing locally.
- **iroh at broadcast bitrates.** Remote live TV (LIV-091) assumes iroh can
  carry a 15 Mbit/s broadcast reliably; that is unproven (unverified).

**Risks**

- **Broken streams are the normal case.** Provider streams change resolution,
  jump timestamps, drop packets and wrap the 33-bit timestamp roughly every
  26.5 hours. All of it must be handled without panics.
- **Browsers are the expensive client.** On the hardware Gunmetal targets,
  browser live TV may be limited to one or two streams. Hiding that would
  damage trust.
- **No bundled guide.** US users will pay Schedules Direct or rely on a
  short broadcast horizon. First run will be weaker than Plex's postal-code
  set-up.
- **Scheduling correctness.** Daylight-saving changes, guide offsets,
  duplicated guide entries, back-to-back padding and tuner reservations all
  interact. An injected clock and property tests are needed from the first
  line of code.
- **Disk I/O on small hardware.** Several recordings plus rolling buffers can
  saturate a USB disk or wear out an SD card, so buffers need a write budget
  and a RAM-only mode.
- **Guide size.** Hundreds of thousands of programme rows, refreshed often,
  mean SQLite write churn on the server and memory pressure on TV devices.
  Both need measuring before the sync format is fixed.
- **ATSC 3.0.** DRM blocks encrypted channels for every third party, and
  AC-4 decoding depends on FFmpeg support and licensing (unverified).
- **Piracy association.** IPTV players and proxies are widely used with
  unlicensed services. Even M3U support can draw that label; Xtream support
  would draw more.
- **Commercial detection is a research project.** It is CPU-heavy and
  error-prone, and Channels DVR is far ahead. Shipping it late is better than
  shipping it wrong.
- **Scope and velocity.** This map has 140 R3 features, and the strict
  test-first gate slows feature work. Live TV demand is real but smaller than
  demand for offline use, sign-in and watch state, according to the
  pain-points research.

## Open decisions for the project owner

1. **Is live TV still the first module after video?** The pain-points
   research finds live TV demand real but smaller than offline, security and
   watch-state demand. *Recommendation:* keep R3 as live TV, but do not start
   it until R2's remuxer and sandbox are stable, and treat virtual channels
   (LIV-169) as the first cut if R3 runs long.
2. **Xtream Codes logins and Stalker portals.** *Recommendation:* never
   support Stalker portals. Ship R3 with M3U only, and decide on Xtream after
   R3 once the content-neutral stance is established. Most of what Xtream
   adds is convenience.
3. **Content-neutral policy.** *Recommendation:* ship no playlists, channel
   lists or guide URLs. Adopt a store-listing disclaimer like TiviMate's, and
   write the docs around antenna, TVHeadend and the user's own provider.
4. **Irreplaceable Live TV data.** Rules, lineup edits, the recordings list
   and deletions need the same guarantees as watch history. *Recommendation:*
   write a new architecture record before any R3 code, using the same log
   format as watch history in a separate stream.
5. **Sources and the network-grant model.** *Recommendation:* extend record
   2's grants to every playlist, guide and tuner host. LAN addresses are
   allowed only by explicit admin grant. Schedules Direct ships as a
   first-party plugin, since record 2 puts third-party services in plugins.
6. **What the browser promises.** *Recommendation:* call live TV a
   native-client feature with the browser as best effort. State the CPU cost
   in the docs, and show a warning in the web client when the server cannot
   keep up.
7. **Broadcast guide data in R3.** *Recommendation:* ship ATSC guide parsing
   in R3, because it is the only free guide for US antenna users. Leave DVB
   event data to TVHeadend's export.
8. **Satellite and DVB cards.** *Recommendation:* make TVHeadend the
   official answer and never put device-driver code in Gunmetal.
9. **Commercial skipping.** *Recommendation:* EDL import and skipping in R3,
   built-in detection Later. Check whether Comskip's licence is compatible
   with AGPL before wrapping it (unverified).
10. **Default recording format.** *Recommendation:* always record the
    transport stream. Turn on the Matroska repackaging (LIV-134) by default
    only once it passes a round-trip corpus, and keep the TS file whenever
    repackaging fails.
11. **Remote live TV on slow links.** *Recommendation:* switch to a lower
    HLS variant where the source has one. Transcode in the sandbox only when
    the admin has enabled it; otherwise refuse with a clear reason.
12. **Internet radio ownership.** *Recommendation:* build it in R3 on the
    live TV ingest (LIV-168). If the music area wants it in R1, the guarded
    fetcher (LIV-015) must ship in R1 too.
13. **Exporting the lineup to other apps.** *Recommendation:* Later, and
    only with per-device, revocable, live-only credentials. Never use a
    long-lived admin token in an exported URL.
14. **Waking the server for recordings.** *Recommendation:* keep the server
    unprivileged. Publish the next wake time and document the platform
    recipes (LIV-110), rather than shipping a privileged helper.
15. **Is Live TV on or off by default?** *Recommendation:* off. Enable it in
    the set-up wizard, so music-only and video-only households never see it
    or pay for it (LIV-001).
