# Live TV, IPTV and recording

Researched on 2026-10-02 with web access. The session's web-search quota ran
out partway through, so the later checks were made by fetching primary pages
directly (vendor documentation, source code, issue trackers, feature boards
and their JSON APIs). Vote, reaction and comment counts were read from those
pages on 2026-10-02. Anything not confirmed from a source is marked
"(unverified)".

## Scope

This file covers everything a person needs to watch and record broadcast or
streamed linear TV through their own server:

- Sources: M3U and M3U8 playlists, Xtream Codes logins, HLS, MPEG-TS over HTTP,
  UDP and RTP multicast, and network or local tuner hardware.
- Guide data: XMLTV files and URLs, Schedules Direct, bundled commercial guide
  data, and guide data carried inside the broadcast itself.
- Channel management: mapping tuner channels to guide stations, renaming,
  renumbering, grouping, favourites, hiding and logos.
- The programme guide as a screen: grid, discovery rows, search and filters.
- Watching live: tuning speed, zapping, captions, audio, casting.
- Time-shifting, start-over and provider catch-up.
- Recording: one-off, manual, series and rule-based recordings, conflicts,
  retention, formats and post-processing.
- Commercial detection.
- Multi-view and picture-in-picture.
- Stream proxying, connection limits and stream sharing.
- Access control and security specific to live TV.
- Reliability problems users report.

Out of scope here: free ad-supported streaming channels that a vendor
provides (for example Plex's own free live TV), Xtream video-on-demand
catalogues, and TV Everywhere logins, except where they explain a rival's
strengths. Internet radio overlaps with the music research and is mentioned
only where live TV products touch it.

Gunmetal's position: record 2 makes "M3U and live TV" the first module after
the first version, and the README roadmap lists it last. The decisions that
constrain this module are the libmpv player on every client, pure-Rust
parsing with FFmpeg only in a network-less sandbox, a segment map built from
keyframes, SQLite as a rebuildable cache with an append-only log for
irreplaceable data, per-object authorisation with short-lived signed stream
URLs, no central account, and third-party network access only through
explicit grants.

### The products studied, as of October 2026

| Product | What it is | Price for live TV and DVR | State in late 2026 |
|---|---|---|---|
| Plex Media Server | Closed-source media server with Live TV and DVR | Live TV from a tuner is free. Recording needs Plex Pass: $6.99 a month, $69.99 a year or $249.99 lifetime since 29 April 2025. The site warns of a further lifetime price rise from 1 July 2026 (new price unverified). Remote playback of personal media needs Plex Pass or Remote Watch Pass, extended to Fire TV, smart TVs and consoles on 29 April 2026. | Active. No native M3U support. |
| Jellyfin | Open-source (GPL) media server | Free | Active. Live TV is a smaller area of the project. |
| Emby | Closed-source media server | Premiere: $4.99 a month, $54 a year or $119 lifetime, 30-device limit. The feature matrix lists Live TV as free and DVR as Premiere-only; the Live TV setup article says both need Premiere. | Active. |
| Channels DVR | Paid server plus polished apps, US-centric | $8 a month or $80 a year after a free month | Active. Pre-release builds on 1 October 2026 added "Enhanced+" commercial detection. |
| TVHeadend | Open-source (GPL-3.0) TV backend for Linux | Free | Commits are active (last push 29 September 2026), but the only tagged release is the 4.3 pre-release from 2017. About 3.5k GitHub stars. |
| Dispatcharr | Open-source (AGPL-3.0) IPTV stream manager and DVR | Free | Very active: v0.31.0 on 13 September 2026, about 4.2k stars, 292 open issues. |
| Threadfin | Open-source (MIT) M3U proxy, fork of xTeVe | Free | Slowing: last release 1.2.37 on 11 September 2025, last push 3 October 2025. |
| xTeVe | The original M3U proxy | Free | Last release 2.2.0.200 on 29 March 2021. |
| TiviMate | Android TV IPTV player | Free tier plus Premium (third-party guides quote about $9.99 a year or $33.99 lifetime, unverified) | Active: Play Store listing updated 22 June 2026, 5M+ downloads. |
| Kodi IPTV Simple | Kodi's M3U and XMLTV client add-on | Free | Active; the reference for M3U catch-up attributes. |
| HDHomeRun (SiliconDust) | Network tuners | FLEX DUO $109.99 (2 tuners), FLEX QUATRO $149.99 (4), FLEX 4K $199.99 (4, ATSC 3.0 on 2) | DRM-encrypted ATSC 3.0 channels do not work. |
| Schedules Direct | Non-profit guide data for the US and Canada | $35 a year or $9 for two months | Its home page warns that Jellyfin clients get IPs blocked. |

## Feature inventory

Legend: "Yes" means built in and documented. "Partial" means present with a
documented limit. "No" means absent or still an open request. "Plugin" means
only through a third-party add-on. "Paid" means behind the vendor's paid tier.
Where the three video servers are not the relevant rivals, the columns are
replaced and the header says so.

### Sources: M3U, IPTV and stream ingestion

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| M3U playlist as a source (file or URL) | Add an IPTV provider or a TVHeadend export as channels | No. Needs an HDHomeRun emulator such as Threadfin, xTeVe or Dispatcharr. Request has 119 votes, open since April 2018. | Yes, M3U tuner from file or URL | Yes, M3U tuner from file or URL | Dispatcharr, because it adds filtering, failover and connection pooling on top | Plex's gap created a whole category of proxy tools. |
| Xtream Codes API login | Log in with server, username and password instead of a playlist URL | No | No (a third-party plugin, Jellyfin.Xtream, exists) | No (unverified) | Dispatcharr and TiviMate both accept Xtream logins directly | Xtream is common among paid IPTV services, many of them unlicensed (unverified). |
| Stalker or MAC portal | Use set-top-box style portals | No | No | No (unverified) | TiviMate supports Stalker Portal | Dispatcharr's request to emulate STBs has 11 reactions, open. Legally fraught; see Risks. |
| HLS (m3u8) channel streams | Play channels delivered as HLS, not only MPEG-TS | Through a proxy only | Request "M3U Tuner HLS support" (5 votes, 2020); current state unverified | Unverified | Channels DVR documents MPEG-TS and HLS over http and https | Many free and legal streams are HLS. |
| UDP and RTP multicast | Use ISP IPTV or a SAT>IP or multicast headend on the LAN | No; "Multicast RTP" request has 14 votes | No; "UDP protocol" request has 7 votes | Unverified | TVHeadend, whose IPTV input handles multicast networks | Common in Europe for ISP-delivered TV. |
| Custom HTTP headers and user agent | Satisfy providers that block default clients | Not applicable | User agent per tuner; requests for custom headers (9 votes) and per-stream EXTVLCOPT headers (2 votes) are open | User agent and referer per tuner | Kodi IPTV Simple honours per-stream #EXTVLCOPT and #KODIPROP lines | Wrong headers give HTTP 403, per Emby's docs. |
| Scheduled playlist refresh, separate from guide refresh | Playlists and guides update on their own schedules | Not applicable | Requests for separate refresh (8 votes) and scheduled M3U refresh (2 votes) | Unverified | Dispatcharr: per-account interval or cron expression with a builder | Providers rotate tokens and URLs often. |
| Import filters | Keep only the groups or names you want from a 20,000-line playlist | No | No (HDHomeRun has "import favourites only") | No (unverified) | Dispatcharr regex filters on group, title or URL, applied in order | Threadfin also filters. |
| Stale stream handling | Channels that disappear upstream are cleaned up without breaking things | No | Partial. When URLs change, channels are deleted and re-created, which drops their timers (reported in a third-party project's tracker) | Unverified | Dispatcharr: configurable stale retention in days | See Pain points. |
| Several provider URLs for one account | Fall back to a mirror host when the main one fails | No | No | No | Nobody yet. Dispatcharr request has 25 reactions, open since October 2025. | Clear unmet demand. |
| Several logins for one provider | Combine accounts to raise the connection limit | No | No | No | Dispatcharr account profiles (URL search and replace) and server groups | |
| M3U attributes honoured | Channel numbers, logos, groups, time shift and IDs come through intact | Through a proxy only | tvg-id, number and logo (exact set unverified) | tvg-name, tvg-id, tvg-chno, tvg-shift, group and logo documented | Kodi IPTV Simple: the broadest set, including catch-up, radio and multi-group | Channels DVR adds its own tvc-guide-* attributes. |
| Radio channels in a playlist | Audio-only stations appear as radio, not broken video | No | No; "Live radio tab" request has 12 votes | Unverified | Kodi IPTV Simple: radio="true" marks a channel as radio | Natural link to Gunmetal's music player. |
| Large playlists | Thousands of channels stay usable | Partial; a 480-channel DVR limit is widely reported (unverified) | No documented cap | No documented cap (unverified) | Dispatcharr, by filtering before export | Channels DVR documents a 750-channel limit per M3U. |
| Compressed XMLTV (.gz, .xz, .zip) | Large guide files download faster | Unverified | Request for xml.gz (4 votes) open | Unverified | Dispatcharr (gz, zip, xz) and Kodi (gz, xz) | |

### Tuner hardware

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| HDHomeRun network tuners | Plug in a network tuner and it is found | Yes, all models | Yes; legacy units need a first scan with SiliconDust's Windows tool | Yes, models with DLNA 2015 or later | Channels DVR: multiple devices, tuner sharing, sub-second tuning claimed | Network tuners need no drivers on the server. |
| USB and PCIe tuners | Use a Hauppauge or AVerMedia card | Yes on Windows and Linux for a short list of models | No; Hauppauge request has 18 votes | Hauppauge on Windows only, with WinTV 8.5 or later | TVHeadend: any Linux DVB or V4L device | Plex warns that vendor software can conflict with it. |
| Satellite (DVB-S, DVB-S2) and cable (DVB-C) | Record satellite or cable TV | DVB-C on some tuners; no satellite. DVB-S2 request has 63 votes. | No | Through plugins for TVHeadend, DVBLink, DVBViewer and others | TVHeadend, which also does software CSA descrambling | Descrambling has legal limits that vary by country. |
| SAT>IP | Use a SAT>IP box as a network tuner | No | No | Through plugins | TVHeadend acts as both a SAT>IP client and server | Channels DVR accepts satip URLs in custom channels. |
| CableCARD and QAM | Record unencrypted US cable | Yes, with CableCARD tuners | Through HDHomeRun (unverified) | Through HDHomeRun (unverified) | Channels DVR | A shrinking use case. |
| ATSC 3.0 (HEVC video, AC-4 audio) | Watch NextGen TV channels | No AC-4 audio as of April 2024 (current state unverified); a community workaround converts AC-4 to AC-3 | Unverified | Its FFmpeg build could decode AC-4 in 2024 | Unclear; no source compared them directly (unverified) | HDHomeRun FLEX 4K tunes ATSC 3.0 on two of four tuners. |
| DRM-protected ATSC 3.0 channels | Watch encrypted local stations | No | No | No | Nobody. SiliconDust told the FCC in July 2025 that no gateway product has been approved. | Lon.TV posts reported in June 2026 that DRM breaks emergency messages, and in September 2026 that DRM had been removed in Connecticut (details unverified). |
| Other DVR backends as tuners | Keep an existing NextPVR or TVHeadend setup | No | Through TVHeadend's M3U and XMLTV, which its docs say costs more CPU | Plugins for DVBLink, DVBViewer, MediaPortal, NextPVR, ServerWMC, TVHeadend, TVMosaic and Vu+ | Emby, which has the widest plugin list | |
| Tuner priority | Prefer one tuner, or ATSC 3.0 over 1.0 | No; requests have 30 and 9 votes | No; channel-priority request has 3 votes | Unverified | TVHeadend: priorities on DVR profiles and inputs (input detail unverified) | |
| Shared tuner access with other apps | Other apps can use the tuner too | No; Plex expects exclusive access | Unverified | Unverified | HDHomeRun shares itself on the network | |
| IP cameras and webcams as channels | Watch or record a camera like a channel | No; request has 50 votes | Through M3U | Through M3U | Channels DVR names IP cameras as a custom-channel use | |

### Guide data

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Bundled guide data | A working guide with no extra subscription | Yes, with Plex Pass, for many countries; an older forum notice said a provider change removed data for a few countries (unverified, the notice no longer loads) | No | Yes, free for the US, Canada and the UK | Plex and Channels DVR, for breadth of commercial data (Channels detail unverified) | Gunmetal has no central account, so it cannot bundle this. |
| Schedules Direct | Paid, accurate US and Canadian listings | Only through an XMLTV grabber | Yes, but Schedules Direct says Jellyfin ignores rate limits and blocks the IPs | Deprecated in June 2021 per its docs | TVHeadend and other open clients through grabbers (unverified) | $35 a year. Jellyfin issue #13147 had 23 reactions and 146 comments. |
| XMLTV file or URL | Use any guide from any source | Yes, but it replaces Plex's data for that DVR, and switching means deleting the DVR | Yes | Yes | All three are adequate; Dispatcharr adds priority between sources | |
| Several guide sources merged per channel | Fill gaps from a second source, or mix markets | No. "Multiple EPG sources" is the top Plex Live TV request: 611 votes, 279 posts, 19,148 views, open since 2017. | Partial. Each provider can be limited to chosen tuners, though the docs say only one is active (unverified which applies). | Yes. Several guides, chosen per tuner, and several postal codes. | Dispatcharr (priority across sources) and Emby among servers | Mixing Plex data with XMLTV is a separate request with 14 votes. |
| Guide from the broadcast itself (EIT or PSIP) | Free guide data straight from the antenna or satellite | No; request has 19 votes | No. HDHomeRun guide API request has 24 votes. | Unverified | TVHeadend, which fills its guide from DVB over-the-air data | The horizon is usually short (unverified). |
| Placeholder guide for unguided channels | Channels without data still show blocks you can record | No. "Record a programme not in the EPG" has 124 votes; "add channels not in the EPG" has 8. | Unverified | Unverified | Dispatcharr dummy EPG built from channel names with regex templates | Channels DVR adds placeholder blocks of a set length. |
| Time-zone and offset correction | Guides from abroad line up with local time | No | No | tvg-shift | Kodi IPTV Simple: tvg-shift plus catchup-correction | Dispatcharr's offset request has 20 reactions, open. |
| Days of guide kept | See further ahead; keep past days for catch-up | Unverified | GuideDays setting; "store past EPG" request has 7 votes | 1 to 14 days, or automatic by channel count | Emby, for making the trade-off explicit | |
| Category mapping | Programmes land under Movies, Sports, Kids or News | Partial. Requests to use and customise XMLTV categories have 9 and 8 votes. | Yes, per provider | Yes, for XMLTV | Jellyfin and Emby | |
| Guide refresh that never breaks schedules | Updating listings never loses recordings | Unverified | No. Issue #12979: an hourly refresh wiped every scheduled recording. Issue #6103: XMLTV changes need a manual cache purge (20 reactions). | Unverified | No product documents a guarantee | See Pain points. |
| Programme artwork | Posters and stills in the guide | Yes, from its provider | Request for artwork tiles (4 votes) | Unverified | Plex and Channels DVR (Channels has an art picker) | |
| Series identity | The same show is recognised across channels and airings | Yes, from its provider (unverified) | By title or series ID; request to use CRID tags opened September 2026 | Unverified | TVHeadend, which uses series-link data from broadcasters | Series identity drives series recordings. |

### Channel management and mapping

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Mapping screen (tuner channel to guide station) | Fix wrong or missing guide matches | Yes, during setup and later | Yes; request for a search box in mapping (3 votes) | Yes, from the channel list or the guide | Dispatcharr: bulk edits and auto-match | |
| Automatic guide matching | Most channels match without manual work | Yes, by lineup and postal code | By tvg-id and name | Yes, automatic attempt | Dispatcharr EPG auto-match; Kodi does three passes: ID, display name, then channel name | |
| Rename channels | Show "BBC One", not "UK: BBC ONE FHD ᴿᴬᵂ" | No. "Rename and reorder channels" has 491 votes, open since 2018. | Unverified | Unverified | Dispatcharr and Threadfin, with find-and-replace on names | |
| Reorder and renumber | Channel numbers that make sense | No (same 491-vote request) | Request for default channel order (14 votes) | Unverified | Dispatcharr custom numbering | |
| Channel groups | Browse by country, genre or provider group | No. "Guide by channel group or favourites" has 86 votes. | No. "IPTV Channel Groupings" is the top Jellyfin Live TV request: 376 votes, 108 comments, marked planned since 2019. | Groups from M3U (display in the guide unverified) | Channels DVR Channel Collections: manual or rule-based, synced to all devices | |
| Favourites | A short list of channels you actually watch | Yes on big-screen apps, with reordering | Yes; request for IPTV favourites (3 votes) | Unverified | Channels DVR and TiviMate | |
| Hide channels | Remove junk without deleting the source | Uncheck during mapping | Request for a hide option (4 votes) | Unverified | Dispatcharr (active, hidden and empty states) | |
| Several streams per channel | One channel falls back to another stream when the first fails | No | No | No | Dispatcharr: watches for buffering and switches to the next stream | |
| Logos | Correct, consistent channel logos | From its guide provider | Requests to take logos from the M3U (6 votes), add a fallback, and default to logos in the web client (10 reactions) | From guide data or the M3U, selectable | Dispatcharr (upload or guide logos) and Channels DVR's art picker | |
| Channel profiles (subsets) | Different users or devices see different channel sets | No; Live TV can only be shared with Plex Home members | No. "Assign users to TV tuners" has 34 votes; per-tuner access has 5. | Unverified | Dispatcharr: each profile gets its own HDHomeRun, M3U and guide links, plus per-user levels | |
| Bulk editing | Fix hundreds of channels at once | No | No | No | Dispatcharr and Threadfin multi-select editors | |
| Virtual channels from your own library | Turn your shows and films into always-on channels | No; the PseudoTV thread has 237 posts | Request to "make a channel from media" has 8 votes | Unverified | Channels DVR: seven ordering modes, numbers and logos | Tunarr and ErsatzTV also do this (unverified). |
| Export the curated lineup | Feed the cleaned lineup to other apps | No | No (unverified) | No (unverified) | Dispatcharr: M3U, XMLTV, Xtream API and HDHomeRun emulation | Channels DVR also documents exporting channels. |
| Search channels by name or number | Jump to a channel by typing | Request has 6 votes | An issue about it was closed in 2020 (outcome unverified) | Unverified | TiviMate (unverified) | |

### The programme guide as a screen

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Grid guide | The classic channels-by-time grid | Yes; filter by day and HD-only | Yes | Yes | TiviMate, known for a fast grid on Android TV | |
| Current-time line | See where "now" is in the grid | Unverified | Request from 2019 (1 vote); current state unverified | Unverified | Unverified | Small, but users notice. |
| Jump a day ahead | Skip 24 hours in one press | Day filter; "24-hour skip" request has 10 votes | Unverified | Unverified | Unverified | |
| Live preview while browsing | Keep watching while you look through the grid | Unverified | No; request has 11 votes | Unverified | TiviMate and Channels DVR (both unverified); Channels thread "Play current channel while on grid" has 100 posts | |
| Discovery rows | "On now", "Starting soon", "New tonight", upcoming films and sports | Yes: Recommended view with seven rows | "On Now" home row; a sorting fix landed in February 2026 | Unverified | Plex, which has the richest discovery view | Channels DVR has "On Later". |
| Search the guide | Find a show or film across all channels | Yes; guide results appear in global search | Basic | Unverified | TVHeadend for power (regex, content type, duration); Plex for ease | |
| "Already in your library" marker | Avoid recording what you own | No; request has 23 votes | Series option to skip episodes in the library (a bug report says it is not honoured, #13637) | Unverified | Unverified | |
| Filter by group or favourites | Short, relevant grids | Favourites only | Request to filter the home view (14 votes) | Unverified | Channels DVR Channel Collections | |
| Open straight to the guide | TV-style launch | No | Requests: straight to guide (15 votes), remember guide view (4) | Unverified | TiviMate (unverified) | |
| Works with a TV remote on every platform | Guide navigation that never traps focus | Yes | Bugs: Tizen guide would not scroll past the first page (#7198, closed); guide navigation going off-screen (#5705, open) | Unverified | Channels DVR and TiviMate | |
| Programme reminders | A notice when a show starts | Unverified | No (unverified) | Unverified | TiviMate (unverified) | |
| Parental controls on live TV | Block channels or ratings for children | Through sharing restrictions | Through user policy (live TV detail unverified) | A forum thread reports tag exclusions failing for live TV | TiviMate lists parental controls | |
| Hide Live TV entirely | Households without TV don't see it | Unverified | Request to hide Live TV home sections for all users (7 votes) | Unverified | Unverified | |

### Watching live

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Live playback on many devices | Watch on TV boxes, phones and the web | Yes, a long app list | Yes | Yes, Live TV listed as free on all apps | Plex, by device count | |
| Play the broadcast as-is | Over-the-air MPEG-2 plays without a powerful server | No. Plex's docs say Live TV needs a server that can transcode, and over-the-air MPEG-2 often needs transcoding. | Partial (a 2020 report said live TV was always transcoded; current state unverified) | Unverified | Channels DVR apps, which decode on the device (unverified) | The biggest hardware lever for Gunmetal. |
| Tuning speed | Channel changes feel like a TV | Request for faster tuning (9 votes) | Unverified | Forum thread on slow channel changes has 61 replies | Channels DVR, which advertises tuning in under a second | |
| Channel up, down and last channel | Zap with the remote | No; requests have 76 votes and 7 votes | Requests for direct number entry (1 vote, September 2026) and a zapping list (28 votes) | Unverified | TiviMate (unverified) and Channels DVR | |
| Closed captions and DVB subtitles | Captions on live TV | No. Plex's live TV article (last edited March 2023) still says captions are not supported. | DVB subtitles not selectable (#10914, open, 24 comments) | A forum thread on live captions has 14 replies | Channels DVR: full captions and alternate audio | |
| Alternate audio and surround | Second language, 5.1 sound | No per the same Plex article | Unverified | Unverified | Channels DVR (5.1) | |
| Casting live TV | Send live TV to a Chromecast | Partial: from iOS and the web only; the redesigned mobile app could not cast live TV at launch. Request had 379 votes. | Unverified | Unverified | Unverified | |
| Record what you are watching | Press record mid-programme | No; Plex says to stop playback first | Unverified | Unverified | Channels DVR (unverified) | |
| Low-latency mode | Less delay behind the broadcast, for sport | Unverified | Request (3 votes) | Unverified | Unverified | |
| Separate deinterlacing for live TV | Better picture for 1080i broadcasts | Unverified | Request (1 vote) | Unverified | Unverified | With native players this happens on the client. |
| Sleep timer | Stop after a set time | No; request has 6 votes | Unverified | Unverified | Unverified | |

### Time-shifting, start-over and catch-up

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Pause and rewind live TV | Pause for the door; replay a goal | Yes, since server v1.16.1 | No. "Rewind/fast forward live TV" has 8 votes, open since 2022. | Has a live stream buffer setting; pause behaviour unverified | Channels DVR for the experience; TVHeadend for configuration | |
| Configurable buffer length | Choose how far back you can go | No; request has 10 votes | No | Buffer can be limited | TVHeadend: maximum minutes, maximum size, RAM-only or disk, unlimited | |
| Watch from the start | Start a programme that began 20 minutes ago | No; "watch from beginning while recording" has 6 votes and 21 posts | Partial through a third-party plugin (September 2026) | Unverified | Channels DVR (watch while recording) | |
| Watch a recording in progress | Start a recording before it finishes | Unverified; a "Recordings in progress" hub request has 18 votes | No; request has 12 votes, open since 2021 | Unverified | Channels DVR ("Watch While Recording") | |
| Provider catch-up | Play past programmes the provider still holds | No | No; "IPTV Catchup" has 15 votes | No; requested in 2018 and not built as of the thread's last post in 2021 | Kodi IPTV Simple (seven catch-up modes) and TiviMate (catch-up icon in the guide) | Dispatcharr closed its catch-up request (23 reactions) by shipping replay. |
| Rolling recording of everything watched | Any channel you watched can be rewound later | No | Request (6 votes) | No | TVHeadend timeshift to disk comes closest | |

### Recording: one-off and manual

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Record one programme from the guide | One press to record | Paid (Plex Pass) | Yes, free | Paid (Premiere) | Channels DVR for the flow | |
| Manual recording by channel and time | Record without guide data | No; 124 votes | Request to "record any programme on a channel" (4 votes) | Unverified | TVHeadend and Dispatcharr (rules by day, time and dates) | |
| Padding before and after | Do not miss the first or last minutes | Yes, global and per recording, plus "allow partial airings" | Yes, global and per rule, with "required" flags | Yes, global defaults with per-recording override | TVHeadend and Jellyfin | Plex request for "if available" padding (7 votes). |
| Running-late protection for sport | Recordings extend when a match overruns | Manual padding only | Manual | Docs suggest setting 30 to 60 minutes by hand | TVHeadend, which can use the broadcaster's running-status signal | |
| Wake the server to record | A sleeping server still records | No; Plex says it will not wake the machine. Request has 210 votes. | No (unverified) | Yes, an option | Emby | Important for low-power home servers. |
| Where recordings go | Films and shows land in the right library | Choose the library per recording | Separate film and series paths, subfolders; per-recording location request (1 vote) | Default library and subfolder per type | Jellyfin and Emby | |
| Clear wording for stopping | "Stop recording" keeps what was recorded | Unverified | Request to rename "Cancel recording" (5 votes) | Unverified | Unverified | |

### Series recordings and rules

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| New episodes only, or every airing | Avoid re-recording repeats | Yes | Yes (record new only) | Yes (unverified detail) | TVHeadend, with several duplicate-handling modes | |
| Any channel or one channel | Follow a show across channels | Yes ("Limit to channel") | Yes (record any channel) | Unverified | All three servers | |
| Time window and days | Only the evening airing, only weekdays | Unverified | Yes (any time, days of week) | Unverified | TVHeadend: start after, start before, days, duration bounds | |
| Keep limits | Keep the last N episodes, or the last N days | Yes: all, 5, 3, 1, or the past 3, 7 or 30 days; improvement request has 32 votes | Yes (keep up to, keep until) | Unverified | Plex for clarity, TVHeadend for range | |
| Delete after watching | Free space automatically | Yes, but it keys on the server admin's watch status, even if others have not watched | Unverified | Unverified | Unverified | Plex's own docs warn about this. |
| Skip what is in the library | Do not record episodes you own | Yes (request to allow re-recording has 13 votes) | Option exists; bug report says it is ignored | Unverified | Plex | |
| Resolution preference | HD only, prefer HD, replace lower resolution | Yes | Unverified | Unverified | Plex | Plex warns this can replace any library item. |
| Keyword and smart rules | Record anything matching a word, genre, person or rating | No; "Smart recording by keyword" has 55 votes | No; "record series by title" request (1 vote) | Unverified | TVHeadend autorec: regex title, full text, channel, tag, content type, star rating, duration, year, season | |
| Sports team passes | Record every game of a team | No | No | No (unverified) | Channels DVR Team Pass | |
| Skip one episode of a series | Opt out of one airing | No; requests have 25 and 11 votes | Unverified | Unverified | Unverified | |
| Season and episode filters | Only season 5 onward | No; request has 11 votes | No | Unverified | TVHeadend (minimum and maximum season) | |
| Better slot search | Move a rule to another airing to avoid a clash | No | No | Unverified | TVHeadend ("attempt to find better time slots") | |
| Re-record on errors | A damaged recording is retried at a later airing | No (unverified) | No (unverified) | Unverified | TVHeadend: error thresholds, re-scheduling and cloning failed entries | |
| Rules survive source changes | Changing a provider password keeps your rules | Unverified | No; channels are re-created and their timers lost (third-party tracker) | Unverified | No product documents this | |

### Conflicts and tuner allocation

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Conflict warning when scheduling | Know at once that a recording will not happen | Yes, with Cancel, Prefer and Manage | Unverified | Unverified | Plex | |
| Priority list | Decide which rule wins | Yes, drag to reorder | Priority field exists; "scheduling priority" request has 17 votes | Unverified | Plex | |
| Calendar and agenda views | See the week's recordings | Yes, both | Schedule list | Unverified | Plex | |
| Start a recording from the schedule page | Act on a planned item early | No; request has 22 votes | Unverified | Unverified | Unverified | |
| Tuner sharing | Two viewers or a viewer and a recording share one tuner | Unverified | "Allow stream sharing" per tuner; a 2021 issue reported shared viewers were not live | Unverified | Channels DVR and Dispatcharr | |
| Recording takes priority over live viewing | A recording is never lost to someone zapping | Unverified | Unverified | Unverified | Unverified | No product documents a clear policy. |

### Storage, retention and formats

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Container | Recordings in a standard file | MPEG-TS | MPEG-TS | Unverified | TVHeadend: MPEG-TS or Matroska | |
| Convert while or after recording | Smaller files that more devices play | Experimental H.264 transcode while recording; "optimise DVR recordings" request has 6 votes | Through a post-processor | Through a post-processing script | Unverified | Transcoding costs CPU on small servers. |
| Post-processing hooks | Run your own script after recording | Yes, from the server's Scripts folder | Yes, path and arguments | Yes | TVHeadend: pre-processor, post-processor and post-remove commands | |
| Free-space management | Old recordings are pruned before the disk fills | No; request has 28 votes | Unverified | Retention settings (unverified detail) | TVHeadend (keep free space or a used-space cap) and Channels DVR (Auto Prune) | |
| File naming | Names that suit your other tools | Fixed | Requests for naming templates (2 votes) and Windows-safe names (2) | Unverified | TVHeadend: format strings and many toggles | |
| Metadata files and artwork | Recordings carry NFO files and images | Unverified | Yes, NFO and images on by default | Unverified | Jellyfin and TVHeadend | |
| Recordings as normal library items | Recordings sit with the rest of your shows | Yes, in a chosen library | Recordings library; "date added" alignment request (3 votes) | Mixed-content library by default | Plex | |
| Recordings never vanish | Files are only deleted by a rule you set | Unverified | No. #17622 (opened August 2026, open): finished recordings deleted from disk during library scans. | Unverified | No product documents a guarantee | |
| Trim a recording | Cut the pre-roll and overrun | No; request has 8 votes | No | No (unverified) | Unverified | |
| Resume position | Recordings remember where you stopped | Request has 25 votes | Yes (unverified) | Unverified | Unverified | |

### Commercial detection

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Mark commercials for skipping | A skip button over ad breaks, file untouched | Yes since server v1.20.0; skipping needs Plex Pass on the viewer's account | Plugin only. A third-party Comskip plugin exposes breaks as native "Commercial" segments (v1.1.0.0, 28 September 2026). | No built-in (unverified) | Channels DVR: automatic skipping, with "Enhanced+" in pre-release from 1 October 2026 | |
| Remove commercials from the file | Smaller files with no ads | Yes, optional; off by default because detection is imperfect | Through post-processing scripts | Through post-processing scripts | Plex, which explains the risk clearly | |
| Automatic skip | Ads skipped without a button press | No; Skip Ads button only. Auto-skip request (8 votes) closed. | Depends on segment settings | Not applicable | Channels DVR | |
| Detection while still recording | Start a match late and skip ads to catch up | No | Plugin v1.1.0.0 adds it | No | Requested in a Channels DVR thread with 164 posts (status unverified) | |
| Processing cost | Detection that small servers can afford | Plex quotes 2 to 4 minutes per 30-minute recording on a reasonably fast CPU | Depends on Comskip | Not applicable | Channels DVR uses hardware decoding where possible; users report reprocessing taking days | |
| Edit or import markers | Fix bad detection, use EDL files | No; EDL request opened March 2026 | Plugin reads .edl files | Not applicable | Unverified | |
| Detection on ordinary TV libraries | Skip ads in recordings made elsewhere | Yes, optional | Plugin (unverified) | Not applicable | Plex | |

### Multi-view and picture-in-picture

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Multi-view | Two to four channels on one screen | No; request has 34 votes | No; request has 1 vote | No (unverified) | Channels DVR: four channels with several layouts on Apple TV 4K and iPad only; TiviMate Premium on Android TV | Channels' developer said there are no plans for Fire TV, Google TV or Android. |
| Picture-in-picture | Keep a channel playing in a corner | Unverified | Requests for desktop (22 votes) and Android TV (5) | Unverified | Unverified | |

### Stream proxying, sharing and connection limits

These rivals are proxies and DVR middleware, so the columns change.

| Feature | What the user gets | Dispatcharr | Threadfin and xTeVe | Channels DVR | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Respect the provider's connection limit | Never get banned for opening too many streams | Max streams per account | "Number of tuners" setting | Stream limit per source (unverified) | Dispatcharr, because it pools limits across accounts | Jellyfin and Emby have a simple simultaneous-stream limit per M3U. |
| One upstream for many viewers | Two people watching one channel use one connection | Yes | Re-streaming (sharing detail unverified) | Tuner sharing | Dispatcharr | Jellyfin request (3 votes); Jellyfin has a per-tuner sharing switch. |
| Automatic failover | A buffering stream is swapped for a backup | Yes, on detected buffering | No | No | Dispatcharr | |
| Buffering engines | Smooth over flaky streams | FFmpeg, VLC, Streamlink, yt-dlp and plain proxy profiles | FFmpeg or VLC | Unverified | Dispatcharr | These tools run FFmpeg with network access, which Gunmetal's records rule out. |
| HDHomeRun emulation output | Plug into Plex, Emby or Jellyfin as a tuner | Yes | Yes (its main purpose) | Not applicable | Threadfin for simplicity, Dispatcharr for depth | |
| Live stats | See who is streaming what and at what bitrate | Real-time dashboard | Basic | Unverified | Dispatcharr | |
| Per-user or per-client stream profiles | Different devices get suitable streams | Yes (request with 20 reactions shipped) | No | Unverified | Dispatcharr | |
| Redirect versus proxy | Let clients go straight to the provider, or hide the provider | Both; per-user redirect permissions added in v0.31.0 | Proxy | Proxy | Dispatcharr | Redirect exposes provider URLs and credentials to clients. |
| Project health | Will it still be maintained next year? | Releases every two to three weeks in 2026 | Threadfin quiet for a year; xTeVe stopped in 2021 | Commercial, active | Dispatcharr | |

### IPTV players as usability references

These are client apps, so the columns change.

| Feature | What the user gets | TiviMate | Channels app | Kodi IPTV Simple | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Fast grid guide on a TV | A guide that scrolls instantly with a remote | Yes; its signature | Yes | Kodi skin dependent | TiviMate | The reference for feel. |
| Several playlists | Mix providers | Yes (Premium) | Through the server | Several add-on instances (unverified) | TiviMate | |
| Catch-up in the guide | Past programmes marked and playable | Yes, with an icon | No | Yes, seven modes | TiviMate for UX, Kodi for coverage | |
| Recording on the device | Record without a server | Yes | Server only | Through Kodi PVR (unverified) | TiviMate | |
| Multi-view | Several channels at once | Yes | Apple TV 4K and iPad only | No (unverified) | Channels for polish, TiviMate for reach | |
| Platform focus | Where it works well | Android TV only; the listing says it is not optimised for touch | Apple, Android and Fire TV, web (unverified detail) | Everywhere Kodi runs | Channels | Gunmetal's single React Native UI must do both TV and touch. |
| Content-neutral stance | The app ships no channels | Explicit disclaimer in the store listing | Not applicable | Not applicable | TiviMate | A model for Gunmetal's wording. |

### Access control and security

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Live TV for shared users | Family and friends can watch your tuners | Partial: Plex Home members only, and they cannot schedule recordings | Per-user switches for live TV access and recording management | Unverified | Jellyfin among servers; Dispatcharr overall | |
| Per-channel or per-tuner access | Children see only some channels | No | No; requests have 34 and 5 votes | Unverified | Dispatcharr: channel profiles, user levels, mature-content flag | |
| Authenticated stream URLs | Nobody can watch your tuners by guessing a URL | Token based (unverified detail) | Issue #13984 (open, 18 reactions) found unauthenticated video streams; the live TV part was reported fixed in 10.8.9 | Unverified | Unverified | Gunmetal's signed, short-lived URLs address this. |
| Provider credentials kept on the server | Clients never see your IPTV password | Not applicable | Unverified | Unverified | Dispatcharr in proxy mode | M3U URLs usually embed the username and password. |
| Single sign-on | Use your identity provider | Plex account only | Through plugins (unverified) | Unverified | None; Dispatcharr's OIDC request has 21 reactions, open | Gunmetal has OIDC and passkeys by design. |
| Remote live TV | Watch your antenna away from home | Needs Plex Pass or Remote Watch Pass since 2025 (whether this covers live TV is unverified) | Self-managed (port forwarding or a reverse proxy) | Unverified | Channels DVR ("Stream While Away" with hardware transcoding) | |

### Reliability and operations

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Alerts on failed recordings | Find out today, not when you sit down to watch | No; users report finding failures after the fact | Unverified | Unverified | Dispatcharr (notifications, plus refresh-error events in v0.31.0) | |
| Partial-recording detection | Know a recording is short and why | No; 9 to 15-minute partials reported in October 2025 | Unverified | Unverified | TVHeadend: data-error thresholds and re-scheduling | |
| Logs a user can read | Diagnose without shell access | Server logs | Server logs | Server logs | Dispatcharr's in-app log browser (v0.31.0) | |
| Upgrades do not break the DVR | Recording keeps working across versions | Partials persisted across beta and stable (October 2025 thread) | 10.9.2 broke M3U recording with "not implemented" (#11756) | Unverified | Unverified | |
| Polite to guide providers | Guide downloads never get you banned | Unverified | No; Schedules Direct blocks Jellyfin clients for ignoring back-pressure | Unverified | Unverified | |
| Removing a tuner cleans up | Old channels disappear with their tuner | Unverified | Issue #2548: channels remained (17 reactions, 78 comments, 2020) | Forum thread with 26 replies on removed M3U lists | Unverified | |

## Pain points and unmet demand

Each item below gives the evidence that was readable on 2026-10-02.

### 1. Plex users have waited seven to nine years for basic channel control

The three most-voted Plex Live TV requests are all about controlling the
lineup, and all are still open:

- Several guide sources per server: 611 votes, 279 posts, 19,148 views, open
  since May 2017.
  https://forums.plex.tv/t/multiple-epg-sources-for-those-with-multiple-tuners/192844
- Rename and reorder channels: 491 votes, 81 posts, open since July 2018.
  https://forums.plex.tv/t/rename-reorder-channels-in-livetv/282457
- Guide by channel group or favourites: 86 votes.
  https://forums.plex.tv/t/dvr-program-guide-discover-by-channel-group-or-favorites-list/183424
- Channel up, down and last channel: 76 votes.
  https://forums.plex.tv/t/plex-livetv-suggestion-channel-up-down-return-to-previous/496265

Jellyfin's top Live TV request is the same need: "IPTV Channel Groupings" has
376 votes and 108 comments, and has been marked planned since 2019.
https://features.jellyfin.org/posts/186/iptv-channel-groupings

### 2. Native M3U and IPTV handling is missing or thin, so people stack proxies

Plex has never supported M3U playlists directly (119 votes, open since 2018).
https://forums.plex.tv/t/support-for-m3u-playlistss-in-live-tv/232215
The workaround is an HDHomeRun emulator. xTeVe stopped releasing in 2021 and
Threadfin has been quiet since October 2025, which pushes users to
Dispatcharr, now one of the fastest-moving projects in this space (about 4.2k
stars, three releases between August and September 2026).
https://github.com/Dispatcharr/Dispatcharr/releases

Dispatcharr's own tracker shows what is still missing even there: several
provider URLs with failover (25 reactions), an EPG timezone offset (20
reactions) and OpenID Connect (21 reactions).
https://github.com/Dispatcharr/Dispatcharr/issues/504,
https://github.com/Dispatcharr/Dispatcharr/issues/383,
https://github.com/Dispatcharr/Dispatcharr/issues/806

Jellyfin users ask for a zapping-style channel list (28 votes), custom
headers (9 votes), UDP sources (7 votes) and separate playlist and guide
refresh schedules (8 votes).
https://features.jellyfin.org/posts/3678/feature-request-iptv-channel-list-zapping-style-view-in-jellyfin

### 3. Recordings fail silently or get lost

- Plex: partial recordings of 9 to 15 minutes across two HDHomeRun models,
  persisting across beta and stable builds (October 2025, no root cause in
  the thread).
  https://forums.plex.tv/t/live-tv-dvr-failed-to-record-full-episodes/932514
  Other 2025 threads report all recordings stopping at random until a
  restart.
  https://forums.plex.tv/t/dvr-fails-to-record-shows-plex-fails-to-play-live-tv-until-server-restart/930727
- Jellyfin: an hourly guide refresh wiped every scheduled recording (#12979,
  13 comments, closed as stale rather than fixed).
  https://github.com/jellyfin/jellyfin/issues/12979
- Jellyfin: finished recordings deleted from disk during library scans,
  confirmed with a NAS recycle bin (#17622, opened August 2026, still open).
  https://github.com/jellyfin/jellyfin/issues/17622
- Jellyfin: when M3U URLs change because of a rotated token or a new
  password, channels are re-created and their timers and series timers are
  lost. This is documented in a third-party project's tracker.
  https://github.com/lucas-romanenko/jellyfin-tentacle/issues/259
- Jellyfin 10.9.2 broke M3U recording outright with a "not implemented"
  error (#11756).
  https://github.com/jellyfin/jellyfin/issues/11756
- Emby: a forum thread about series not recording because of a wrong date
  has 53 replies.
  https://emby.media/community/forum/97-live-tv/

### 4. Guide data is either paywalled, banned or brittle

- Schedules Direct's home page says, in its words, that all Jellyfin apps
  "have bugs talking to us", and warns that IPs will probably be blocked. Its
  notice says Jellyfin ignores back-pressure and sends 20 to 100 requests a
  second. https://www.schedulesdirect.org/ and
  https://www.schedulesdirect.org/news/063/Jellyfin%2CSageTV-and-disabled-account
- Jellyfin issue #13147 about Schedules Direct setup breaking drew 23
  reactions and 146 comments.
  https://github.com/jellyfin/jellyfin/issues/13147
- Jellyfin issue #6103: XMLTV changes need a manual cache purge (20
  reactions). https://github.com/jellyfin/jellyfin/issues/6103
- Plex can use XMLTV only by replacing its own guide for the whole DVR, and
  switching means deleting the DVR, per Plex's own article.
  https://support.plex.tv/articles/using-an-xmltv-guide/
- People want free guide data from the broadcast or the tuner: Plex "Over the
  air EPG" (19 votes) and Jellyfin "HDHomeRun TV guide API" (24 votes).
  https://forums.plex.tv/t/over-the-air-epg/429205,
  https://features.jellyfin.org/posts/190/add-support-for-hdhomerun-tv-guide-api

### 5. Live TV needs a strong server on Plex

Plex's support pages say live TV requires a server that can transcode, and
that over-the-air MPEG-2 usually needs transcoding for most apps. The same
live TV article still lists closed captions and alternate audio as
unsupported.
https://support.plex.tv/articles/115007689648-watching-live-tv/,
https://support.plex.tv/articles/225877427-supported-dvr-tuners-and-antennas/
This is the opposite of what a low-power server wants.

### 6. No time-shift, catch-up or start-over outside Plex and Channels

- Jellyfin "Rewind/fast forward live TV": 8 votes, open since 2022.
  https://features.jellyfin.org/posts/1830/rewind-fast-forward-live-tv
- Jellyfin "IPTV Catchup": 15 votes.
  https://features.jellyfin.org/posts/1918/iptv-catchup
- Jellyfin "Playback of in-progress DVR recordings": 12 votes, open since
  2021. https://features.jellyfin.org/posts/1142/playback-of-in-progress-dvr-recordings
- Emby catch-up was requested in March 2018, and staff said in January 2021
  it would not ship with the Live TV rework.
  https://emby.media/community/topic/57392-live-tv-catchup-feature/
- Plex "Live TV pause buffer length" (10 votes) and "watch from beginning
  while recording" (6 votes, 21 posts).
  https://forums.plex.tv/t/live-tv-pause-buffer-length/703035

### 7. Connection limits and stream sharing are left to add-ons

IPTV providers cap simultaneous connections, often at a handful (a
third-party guide suggests 3 to 5, unverified). Media servers open one
upstream connection per viewer and one per recording, so households hit the
cap. Dispatcharr exists largely to pool connections and fail over. Jellyfin
has a per-tuner stream-sharing switch, but a 2021 issue reported that shared
viewers were no longer live, and a request for one stream for several users
remains open.
https://github.com/jellyfin/jellyfin/issues/5724,
https://features.jellyfin.org/posts/1555/one-live-tv-channel-stream-for-multiple-users

### 8. Sharing live TV with the household is restricted

Plex lets only Plex Home members watch live TV, and they cannot schedule
recordings. https://support.plex.tv/articles/115007689648-watching-live-tv/
Jellyfin users ask to assign users to specific tuners (34 votes).
https://features.jellyfin.org/posts/461/assign-users-to-tv-tuners

### 9. Commercial detection is weak, slow or missing

Plex marks or removes commercials but warns detection is imperfect and costs
2 to 4 minutes of CPU per 30-minute recording, and skipping needs Plex Pass
on the viewer's account.
https://support.plex.tv/articles/115003944134-removing-commercials/
Jellyfin has only a brand-new third-party plugin.
https://github.com/mackandall/jellyfin-plugin-comskip-segments
Channels DVR leads and is still iterating (Enhanced+ in pre-release on 1
October 2026; users report reprocessing taking days).
https://community.getchannels.com/t/beta-enhanced-commercial-detection/45472/213
Sports viewers want detection during the recording so they can start late
and catch up (164-post thread).
https://community.getchannels.com/t/commercial-detection-for-in-process-recordings-catch-up-to-live-broadcast-especially-useful-for-live-sports-events/28276

### 10. Multi-view is rare and locked to one platform

Plex's request has 34 votes and was active in June 2026.
https://forums.plex.tv/t/watching-multiple-live-tv-channels-at-once-picture-in-picture-side-by-side/635162
Channels DVR offers four-way multi-view only on Apple TV 4K and iPad, with no
plans for Android-based devices (PCWorld, September 2025).
https://www.pcworld.com/article/2912640/tv-antenna-users-theres-finally-a-way-to-do-multiview.html

### 11. Low-power servers cannot sleep

Plex will not wake a sleeping machine for a recording, and the request has
210 votes. https://forums.plex.tv/t/feature-request-allow-plex-media-server-to-sleep-wake-properly-including-dvr-recordings/180912
Emby offers a wake option.
https://emby.media/support/articles/DVR-Settings.html

### 12. Hardware and broadcast changes outpace the servers

ATSC 3.0 brings HEVC, AC-4 audio and DRM. SiliconDust told the FCC in July
2025 that no DRM-capable gateway had been approved.
https://www.tvtechnology.com/news/silicondust-identifies-atsc-3-0-security-authority-as-drm-culprit-in-fcc-comments
Plex lacked AC-4 audio as of April 2024, and users built a converter around
Emby's FFmpeg build.
https://www.vomitron.com/2024/04/22/adding-dolby-ac-4-to-plex-for-atsc-3-0-audio/
European users keep asking for satellite (Plex DVB-S2 request, 63 votes) and
Hauppauge cards on Jellyfin (18 votes).

### 13. TV-platform polish

Jellyfin's guide would not scroll past the first page on Samsung Tizen
(#7198) and navigation can go off-screen (#5705, open). Its live captions for
DVB subtitles are not selectable (#10914, open, 24 comments). Emby users
discuss slow channel changes (61 replies) and buffering delays.
https://github.com/jellyfin/jellyfin-web/issues/7198,
https://github.com/jellyfin/jellyfin/issues/10914

### Where rivals are already good

- Channels DVR is the polish benchmark: sub-second tuning, pause and rewind,
  watch while recording, Team Pass, automatic commercial skipping, Channel
  Collections, virtual channels, and multi-view on Apple devices.
- TVHeadend is the depth benchmark: every tuner type on Linux, over-the-air
  guide data, regex auto-recording, duplicate handling, error-based
  re-recording, and a fully configurable timeshift.
- Dispatcharr is the IPTV benchmark: filtering, failover, pooled limits,
  profiles, dummy guides and every output format.
- Plex has the best discovery view and the clearest recording and conflict
  flows, and a wide device list.
- Emby has the widest tuner-plugin list, several guide sources per tuner and
  wake-for-recording.
- TiviMate sets the bar for how a TV guide should feel with a remote.

## Where Gunmetal can be clearly better

Each idea names the pain point it answers, how it fits the architecture
records, and what has to be true technically.

### 1. Live TV that plays as broadcast on native clients (pain points 5, 12)

Native clients get the transport stream as it arrives and decode it in
libmpv, including MPEG-2 video, AC-3 audio and deinterlacing on the device.
The server does a network read and a network write. This follows record 1's
"own both ends" and "transcode as a last resort" decisions, and it removes
Plex's need for a transcode-capable server.

What must be true: libmpv builds on Android TV, Apple TV and desktop decode
MPEG-2 and H.264 broadcasts and deinterlace well at 1080i on the target
devices (performance on low-end Android TV is unverified). Browsers cannot
play MPEG-2 or interlaced video, so the web client needs the remuxer for
H.264 and AAC channels and the sandboxed transcoder for everything else. The
honest statement is that browser live TV costs CPU on a weak server and
native clients do not.

### 2. Ingest written in Rust, so FFmpeg never touches the network (pain points 2, 7)

Threadfin, Dispatcharr and Jellyfin hand streams to FFmpeg or VLC with
network access. Record 1 runs FFmpeg only in a sandbox with no network, so
Gunmetal's server must fetch HTTP and HLS itself, parse MPEG-TS packets
(program tables, clock references, continuity counters) and feed bytes to a
sandboxed FFmpeg through a pipe only when a transcode is needed. Parsing
provider streams is untrusted-input parsing, which is exactly what the core's
no-panic, fuzzed, memory-safe rules exist for.

What must be true: an MPEG-TS demuxer and an HLS playlist parser in the core
crate, under the same 100% coverage and zero-surviving-mutant gate; a live
HLS client that handles discontinuities, AES-128 encrypted segments (which
are not DRM), and playlist reloads; and a pipe interface into the transcode sandbox.

### 3. A connection and tuner broker (pain points 7, 3)

One upstream connection per channel, shared by every viewer and every
recording on that channel. Per-source connection limits, pooled across
several logins for the same provider. Fixed priorities: recordings first,
then live viewers, then guide previews and multi-view tiles. When everything
is busy, the client is told who is using each tuner and offered the choice to
take one, instead of a vague error. Several streams per channel with
failover, which Dispatcharr users still ask to extend to provider mirrors.

What must be true: a fan-out buffer per channel that serves new readers from
the last keyframe (the segment map idea from record 1 applied to a live
stream), clean back-pressure for slow clients, and a scheduler that reserves
connections ahead of recordings, including padding.

### 4. Channel identity that survives provider churn (pain point 3)

A channel's identity is the user's channel, not the URL. Stream URLs, tokens
and passwords are attributes that can change underneath it. Recording rules
point at channels by Gunmetal's own random ID (record 1, decision 6) and at
programmes by series ID with a title fallback. Playlist and guide refreshes
are computed as differences and never delete rules, scheduled recordings or
favourites; at most they mark a channel as missing and alert the admin.

What must be true: a matching step that maps a refreshed playlist onto
existing channels by guide ID, then normalised name, then group and number,
with the user's manual mappings always winning; and tests that refresh
against recorded real-world playlists.

### 5. Rules, mappings and recordings treated as irreplaceable data (pain point 3)

Record 1 says SQLite is a rebuildable cache and only watch history is
irreplaceable. Recording rules, channel names and numbers, groups,
favourites and the list of recordings are also things a user cannot
rebuild by rescanning. They belong in the same append-only, exportable log as
watch history. Recordings are deleted only by an explicit retention rule or
a user action, each deletion is written to the log, and deleted files go to a
trash folder for a set period. Library scans never delete recordings.

What must be true: a new architecture record extending the "irreplaceable
data" list, because record 1 does not cover this today.

### 6. A recorder that reports its own failures (pain point 3)

Every recording tracks continuity errors, gaps, upstream disconnects, the
bytes written and the expected duration. When a stream drops, the recorder
reconnects (or fails over to another stream or tuner), marks the gap and
carries on. A short or damaged recording is flagged with a reason, the admin
gets a notification on their devices, and, like TVHeadend, the rule can
re-record at a later airing. Before each recording the server checks the
source and disk a few minutes early.

What must be true: a recording state machine with injected clocks so it can
be tested deterministically to the project's standards, and a notification
path to clients that does not depend on a vendor relay.

### 7. A guide built for scale, offline browsing and several sources (pain points 1, 4)

Record 1 syncs the library to the device; the guide should sync the same way,
so scrolling a 500-channel, 14-day grid never waits for the server. Guide
sources merge per channel with a priority order, which answers the
611-vote Plex request. Sources include XMLTV files and URLs, Schedules Direct
with a client that honours rate limits and back-pressure from day one, and
guide data read from the broadcast itself (ATSC PSIP and DVB EIT tables
parsed in Rust from the tuner stream) so antenna users get a free guide.
Placeholder blocks cover channels with no data, so manual recording still
works.

What must be true: a compact, delta-synced guide format small enough for TV
devices (size unverified; it needs measuring), parsers for ATSC and DVB event
tables including ATSC's compressed text strings, and careful time handling
across time zones, daylight-saving changes and the 33-bit timestamp wrap.

### 8. The channel controls people have asked for since 2017 (pain points 1, 2, 13)

Rename, renumber, hide, re-logo and group channels; per-user favourites;
regex filters at import; bulk edit; channel up, down, last and number entry;
a zapping list; a now line and a one-press jump of a day. None of this is
technically hard. It is the most-voted gap in both Plex and Jellyfin, so it
is cheap to win.

What must be true: these features are designed TV-first for the remote and
also work by touch, since Gunmetal has one React Native UI for both.

### 9. Time-shift, start-over and catch-up on every channel (pain point 6)

While a channel is being watched or recorded, the server keeps a rolling
buffer on disk with a keyframe index built as bytes arrive. Pause, rewind and
start-over then work for every viewer of that channel, and the buffer length
is a setting. Provider catch-up is honoured using the M3U attributes Kodi
IPTV Simple documents, and past guide entries stay visible for the catch-up
window.

What must be true: incremental segment maps over a growing file, a disk-write
budget suitable for USB disks and SD cards, and retention of past guide data
for catch-up days.

### 10. Commercial markers that are honest about cost (pain point 9)

Detection never alters the recording; it writes markers that clients can
skip automatically or on request, per user. Detection runs in the transcode
sandbox at low priority, can run during the recording for sports catch-up,
and can be switched off entirely on weak hardware. Markers are editable and
can be imported from EDL files.

What must be true: either an external detector run as a separate sandboxed
process (Comskip's licence needs checking for compatibility with AGPL,
unverified) or Gunmetal's own detector fed decoded frames from the sandbox.
Channels DVR is clearly ahead here; Gunmetal should aim to match Plex's
non-destructive design first and not promise Channels-level accuracy.

### 11. Multi-view on more than Apple devices (pain point 10)

Because native clients already decode with libmpv, two to four tiles are
several players on one screen. Each tile takes a tuner or connection from the
broker, so the broker can refuse or downgrade a tile rather than starve a
recording.

What must be true: a decoder-capability probe per device (many Android TV
boxes have a small number of hardware decoders, unverified per model) and a
fallback to fewer tiles.

### 12. Per-user, per-channel access with credentials that never leave the server (pain points 7, 8)

Channels are objects with their own permissions (record 1, decision 6), so a
household can share live TV with anyone, limit children to some channels and
decide who may schedule. Clients receive short-lived signed Gunmetal URLs,
never provider URLs, so IPTV passwords stay on the server. Each M3U, XMLTV
or guide source is a declared outbound host, in line with record 2's rule
that third-party network access needs an explicit grant.

What must be true: the broker is the only path to upstream streams, and the
network-grant model covers sources, not only plugins.

### 13. Recordings stored losslessly and cheaply (pain points 5, 11)

Record raw transport streams with no CPU cost, then optionally repackage to
Matroska in-process with the remuxer, keeping every audio and caption track
and adding chapters at commercial markers. Wake timers let a low-power server
sleep between recordings, if the operating system allows an unprivileged way
to set them (unverified on each platform).

### 14. Interoperate instead of reinventing hardware support

HDHomeRun is a plain HTTP device and can be supported natively. Satellite,
DVB cards and SAT>IP are better taken from TVHeadend as a source (through its
M3U and XMLTV exports or its own protocol) than by writing kernel-driver
code. Gunmetal could also export its curated lineup as M3U, XMLTV and an
HDHomeRun-style device, with authenticated per-device URLs, so people can
keep TiviMate or Kodi during a migration.

## Risks and hard parts

- **Broken streams are the normal case.** Provider streams change resolution
  mid-stream, jump timestamps, drop packets, insert adverts and wrap the
  33-bit timestamp roughly every 26.5 hours. The parsers must handle all of
  this without panics, which needs large corpora of real captures for tests
  and fuzzing.
- **Browsers are the expensive client.** MPEG-2, AC-3 in some browsers, and
  interlaced video need a transcode. On the hardware Gunmetal targets,
  browser live TV may be limited to one or two streams. That should be stated
  plainly rather than hidden.
- **No bundled guide.** Plex, Emby and Channels DVR bundle commercial guide
  data through their accounts. Gunmetal has no central account and should not
  run a guide service, so US users will pay Schedules Direct or rely on
  broadcast data with a short horizon. First-run experience will be weaker
  than Plex's postal-code setup.
- **ATSC 3.0.** DRM blocks encrypted channels for every third-party product,
  AC-4 decoding depends on FFmpeg and licensing (current FFmpeg AC-4 support
  unverified), and HEVC decode varies by client.
- **Piracy association.** IPTV players and proxies are widely used with
  unlicensed services. Supporting Xtream Codes and Stalker portals invites
  that reputation and could affect app-store listings (unverified). Gunmetal
  should be content-neutral, ship no playlists, and consider leaving out
  set-top-box portal emulation.
- **Commercial detection is a research project.** It is CPU-heavy, error-prone
  and a moving target. Shipping it late is better than shipping it wrong.
- **Scheduling correctness.** Daylight-saving transitions, guide time-zone
  offsets, overlapping or duplicated guide entries (as Jellyfin users see),
  padding across back-to-back recordings, and tuner reservations all interact.
  The project's 100% coverage and zero-mutant gate makes this safer, but it
  needs an injected clock and property tests from the start.
- **Disk I/O on small hardware.** Several recordings plus rolling buffers can
  saturate a USB disk or wear an SD card. Buffers need a write budget and a
  RAM-only option like TVHeadend's.
- **Guide size in SQLite and on devices.** Hundreds of thousands of programme
  rows, refreshed often, mean write churn on the server and memory pressure on
  TV devices. Both need measuring before the format is fixed.
- **Architecture records need extending.** Treating rules and recordings as
  irreplaceable data, and giving guide and playlist sources network grants,
  are new decisions and need their own records.
- **Scope creep.** TV Everywhere (Channels DVR's provider logins), Xtream VOD
  catalogues and free streaming channels are each large on their own and
  outside this module.

## Open questions

1. Should live TV be a module an admin can switch off completely, so
   households without TV never see it (Jellyfin users ask for this)?
2. Which guide sources ship first: XMLTV and Schedules Direct only, or also
   broadcast guide data from the start?
3. Is the web client expected to play live TV on a low-power server, or is
   live TV a native-client feature with the browser as a best effort?
4. Does Gunmetal accept Xtream Codes logins, given their popularity and
   their association with unlicensed services? Stalker portals?
5. Is TVHeadend the official answer for satellite and DVB cards, or should
   Gunmetal ever talk to Linux DVB devices itself?
6. Should Gunmetal expose its lineup as M3U, XMLTV and an HDHomeRun-style
   device for other apps, and if so, how are those URLs authenticated?
7. Where exactly do rules, mappings and recordings live: in the watch-history
   log, or in a second log with the same guarantees?
8. Build commercial detection, wrap an existing detector in the sandbox, or
   defer it?
9. How does remote live TV over iroh handle a 15 Mbit/s broadcast on a mobile
   link: transcode on the server, a lower-bitrate source, or refuse?
10. Default recording format: raw transport stream, or remux to Matroska
    after the recording ends?
11. Should radio channels in a playlist open in the music player with the
    persistent now-playing bar (coordinate with the music research)?
12. How many tiles should multi-view allow per device class, and does a tile
    count against a household's connection limit the same way a viewer does?

## Sources

Plex
- https://support.plex.tv/articles/225877347-live-tv-dvr/
- https://support.plex.tv/articles/115003944134-removing-commercials/
- https://support.plex.tv/articles/using-an-xmltv-guide/
- https://support.plex.tv/articles/225877387-program-guide/
- https://support.plex.tv/articles/115007689648-watching-live-tv/
- https://support.plex.tv/articles/226074728-setting-up-recordings/
- https://support.plex.tv/articles/225976308-manage-your-recordings/
- https://support.plex.tv/articles/225877427-supported-dvr-tuners-and-antennas/
- https://www.plex.tv/blog/important-2025-plex-updates/
- https://www.plex.tv/plans/
- https://www.plex.tv/plex-pass/
- https://forums.plex.tv/tags/c/feature-suggestions/8/livetv-dvr.json?order=votes
- https://forums.plex.tv/t/multiple-epg-sources-for-those-with-multiple-tuners/192844
- https://forums.plex.tv/t/rename-reorder-channels-in-livetv/282457
- https://forums.plex.tv/t/chromecast-live-tv/205948
- https://forums.plex.tv/t/feature-request-add-tv-everywhere-to-live-tv-dvr/464160
- https://forums.plex.tv/t/feature-request-allow-plex-media-server-to-sleep-wake-properly-including-dvr-recordings/180912
- https://forums.plex.tv/t/feature-request-dvr-create-recording-for-program-not-in-epg/185995
- https://forums.plex.tv/t/support-for-m3u-playlistss-in-live-tv/232215
- https://forums.plex.tv/t/dvr-program-guide-discover-by-channel-group-or-favorites-list/183424
- https://forums.plex.tv/t/plex-livetv-suggestion-channel-up-down-return-to-previous/496265
- https://forums.plex.tv/t/support-for-dvb-s2-freesat-when-its-really-time/295371
- https://forums.plex.tv/t/smart-recording-by-keyword/282320
- https://forums.plex.tv/t/watching-multiple-live-tv-channels-at-once-picture-in-picture-side-by-side/635162
- https://forums.plex.tv/t/over-the-air-epg/429205
- https://forums.plex.tv/t/live-tv-pause-buffer-length/703035
- https://forums.plex.tv/t/watch-from-beginning-while-recording/208129
- https://forums.plex.tv/t/live-tv-dvr-failed-to-record-full-episodes/932514
- https://forums.plex.tv/t/dvr-fails-to-record-shows-plex-fails-to-play-live-tv-until-server-restart/930727
- https://forums.plex.tv/t/hdhomerun-plex-unable-to-record-more-than-a-short-period-of-any-show/906401
- https://www.pcworld.com/article/2653520/plexs-new-experience-arrives-6-things-to-know-before-updating.html

Jellyfin
- https://jellyfin.org/docs/general/server/live-tv/setup-guide/
- https://raw.githubusercontent.com/jellyfin/jellyfin/master/MediaBrowser.Controller/LiveTv/SeriesTimerInfo.cs
- https://raw.githubusercontent.com/jellyfin/jellyfin/master/MediaBrowser.Model/LiveTv/LiveTvOptions.cs
- https://raw.githubusercontent.com/jellyfin/jellyfin/master/MediaBrowser.Model/LiveTv/TunerHostInfo.cs
- https://raw.githubusercontent.com/jellyfin/jellyfin/master/MediaBrowser.Model/LiveTv/ListingsProviderInfo.cs
- https://raw.githubusercontent.com/jellyfin/jellyfin/master/MediaBrowser.Model/Users/UserPolicy.cs
- https://features.jellyfin.org/api/v1/posts (queried by keyword, sorted by most wanted)
- https://features.jellyfin.org/posts/186/iptv-channel-groupings
- https://features.jellyfin.org/posts/461/assign-users-to-tv-tuners
- https://features.jellyfin.org/posts/3678/feature-request-iptv-channel-list-zapping-style-view-in-jellyfin
- https://features.jellyfin.org/posts/190/add-support-for-hdhomerun-tv-guide-api
- https://features.jellyfin.org/posts/1918/iptv-catchup
- https://features.jellyfin.org/posts/1830/rewind-fast-forward-live-tv
- https://features.jellyfin.org/posts/1142/playback-of-in-progress-dvr-recordings
- https://features.jellyfin.org/posts/1555/one-live-tv-channel-stream-for-multiple-users
- https://features.jellyfin.org/posts/3468/keep-the-current-channel-playing-while-viewing-the-guide
- https://features.jellyfin.org/posts/36/add-scheduling-priority-for-dvr-series-recordings
- https://github.com/jellyfin/jellyfin/issues/13147
- https://github.com/jellyfin/jellyfin/issues/12979
- https://github.com/jellyfin/jellyfin/issues/17622
- https://github.com/jellyfin/jellyfin/issues/11756
- https://github.com/jellyfin/jellyfin/issues/13984
- https://github.com/jellyfin/jellyfin/issues/13637
- https://github.com/jellyfin/jellyfin/issues/6103
- https://github.com/jellyfin/jellyfin/issues/2548
- https://github.com/jellyfin/jellyfin/issues/10914
- https://github.com/jellyfin/jellyfin/issues/5724
- https://github.com/jellyfin/jellyfin-web/issues/7383
- https://github.com/jellyfin/jellyfin-web/issues/7198
- https://github.com/jellyfin/jellyfin-web/issues/5705
- https://github.com/jellyfin/jellyfin-web/issues/1978
- https://github.com/lucas-romanenko/jellyfin-tentacle/issues/259
- https://github.com/mackandall/jellyfin-plugin-comskip-segments

Emby
- https://emby.media/support/articles/Live-TV.html
- https://emby.media/support/articles/M3U-Tuners.html
- https://emby.media/support/articles/DVR-Settings.html
- https://emby.media/support/articles/Xml-Tv.html
- https://emby.media/support/articles/Live-TV-Channel-Mapping.html
- https://emby.media/support/articles/Live-TV-Plugins.html
- https://emby.media/support/articles/Emby-Premiere.html
- https://support.emby.media/support/articles/Premiere-Feature-Matrix.html
- https://emby.media/premiere.html
- https://emby.media/community/forum/97-live-tv/
- https://emby.media/community/topic/57392-live-tv-catchup-feature/

Channels DVR
- https://getchannels.com/live-tv/
- https://getchannels.com/dvr-server/
- https://getchannels.com/docs/channels-dvr-server/how-to/custom-channels/
- https://getchannels.com/docs/channels-dvr-server/how-to/
- https://getchannels.com/docs/channels-dvr-server/how-to/channel-collections/
- https://getchannels.com/docs/channels-dvr-server/how-to/virtual-channels/
- https://getchannels.com/docs/channels-dvr-server/how-to/smart-rules/
- https://getchannels.com/docs/channels-dvr-server/tv-everywhere/about/
- https://community.getchannels.com/t/beta-enhanced-commercial-detection/45472/213
- https://community.getchannels.com/t/commercial-detection-for-in-process-recordings-catch-up-to-live-broadcast-especially-useful-for-live-sports-events/28276
- https://community.getchannels.com/t/play-current-channel-while-on-grid/418
- https://www.pcworld.com/article/2912640/tv-antenna-users-theres-finally-a-way-to-do-multiview.html

TVHeadend
- https://github.com/tvheadend/tvheadend/blob/master/docs/markdown/introduction.md
- https://docs.tvheadend.org/documentation/configuration/electronic-program-guide
- https://raw.githubusercontent.com/tvheadend/tvheadend/master/src/dvr/dvr_config.c
- https://raw.githubusercontent.com/tvheadend/tvheadend/master/src/dvr/dvr_autorec.c
- https://raw.githubusercontent.com/tvheadend/tvheadend/master/src/timeshift.c
- https://github.com/tvheadend/tvheadend/releases
- https://api.github.com/repos/tvheadend/tvheadend

Dispatcharr, Threadfin and xTeVe
- https://github.com/Dispatcharr/Dispatcharr
- https://github.com/Dispatcharr/Dispatcharr/releases
- https://dispatcharr.github.io/Dispatcharr-Docs/
- https://dispatcharr.github.io/Dispatcharr-Docs/m3u-epg-manager/
- https://dispatcharr.github.io/Dispatcharr-Docs/channels/
- https://dispatcharr.github.io/Dispatcharr-Docs/dvr/
- https://dispatcharr.github.io/Dispatcharr-Docs/connection-security/
- https://github.com/Dispatcharr/Dispatcharr/issues/504
- https://github.com/Dispatcharr/Dispatcharr/issues/133
- https://github.com/Dispatcharr/Dispatcharr/issues/806
- https://github.com/Dispatcharr/Dispatcharr/issues/407
- https://github.com/Dispatcharr/Dispatcharr/issues/383
- https://github.com/Dispatcharr/Dispatcharr/issues/71
- https://github.com/Threadfin/Threadfin
- https://github.com/Threadfin/Threadfin/releases
- https://api.github.com/repos/xteve-project/xTeVe
- https://xteve.org/
- https://cloudje.eu/en/threadfin-config/

Players, tuners and guide data
- https://play.google.com/store/apps/details?id=ar.tvplayer.tv&hl=en_US
- https://github.com/kodi-pvr/pvr.iptvsimple/blob/Piers/README.md
- https://www.silicondust.com/hdhomerun.html
- https://www.tvtechnology.com/news/silicondust-identifies-atsc-3-0-security-authority-as-drm-culprit-in-fcc-comments
- https://blog.lon.tv/tag/atsc-3-0/
- https://www.vomitron.com/2024/04/22/adding-dolby-ac-4-to-plex-for-atsc-3-0-audio/
- https://www.schedulesdirect.org/
- https://www.schedulesdirect.org/news/063/Jellyfin%2CSageTV-and-disabled-account
- https://wiki.xmltv.org/index.php/XMLTVFormat
- https://jellywatch.app/blog/emby-iptv-live-tv-setup-guide-2026 (third-party; used only to cross-check Emby claims)
