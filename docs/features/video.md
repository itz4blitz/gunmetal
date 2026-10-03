# Video playback

This map covers everything that happens after someone presses play on a film
or episode: how the file reaches the screen (direct play, remux or
transcode), quality and remote bandwidth, HDR and Dolby Vision, audio tracks
and sound fixes, subtitles of every kind, seeking, chapters and previews,
skip markers, resume and autoplay, player controls, casting and handoff,
watch-together, trailers and extras, and the statistics that explain what the
player is doing. Home rows, watchlists, history browsing, library
organisation, download rules, user policy and live TV have their own feature
maps; they appear here only where they change what happens during playback.
The bar is this: on a first-party client no film ever needs a transcode
because of its subtitles, its container or its HDR format; every decision
the player makes can be explained in one plain sentence; and nothing Plex or
Emby charge for at playback time costs anything. Where rivals are already
good (Infuse on Apple TV, Kodi and mpv on format coverage, Plex on the polish
of its skip markers, Jellyfin on free hardware transcoding and its open
device profiles), the aim is parity first and an edge only where the
architecture actually provides one.

## Features

Release assumptions used in the tables:

- Releases (R1, R2, R3, Later, No), the Demand scale, row ownership and the
  terms "the user log" and "the identity store" are defined once in the
  [feature map README](README.md). A row whose Release cell would differ
  between maps names one owning row; the other maps point at it.
- No video feature is R1. Several mechanisms these rows reuse arrive with
  music in R1: the user log, signed stream URLs, settings sync, the sleep
  timer, the persistent queue and the playback decision type. The native
  background audio and lock-screen module (MUS-074) and the handoff control
  channel (CLI-101) arrive with the R2 native clients.
- R2 native clients are taken to mean Android phones and tablets, Android TV
  and Google TV, the web client and the desktop shell. The README roadmap
  places Apple builds and Samsung and LG packaging after the video
  milestone, so rows that only matter on those platforms are marked Later.
- Rows about remote playback assume iroh remote access ships in R2, as the
  release key says. If it slips, those rows apply on the local network
  until it lands.
- UI surface names are used consistently so the UI map can group them:
  "Player" surfaces are inside the playback screen, "Pre-play sheet" is the
  sheet between the title page and playback, and "Admin" surfaces are owner
  screens.

### Delivery and the playback decision

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| VID-001 | Direct play of the original file | The untouched file plays at full quality; the server does a disk read and a network write | Plex yes when container, codecs, bitrate and resolution all suit the client; Jellyfin yes; Emby yes; Infuse and Kodi best, because their decoders take nearly anything | High: needless transcoding is the sixth-ranked pain theme; Plex "default to max quality" request has 1,289 votes | R2 | Parity with mpv-based players (Plex HTPC, Jellyfin Desktop), Kodi and Infuse on native direct play. The edge is that every first-party client defaults to the original (VID-023), never burns in subtitles (VID-069, VID-070), and the remuxer replaces the transcode for browsers and TVs (VID-003). Tracks were indexed at scan, so nothing is probed at play time. | Byte-range serving; per-file stream index (tracks, codecs, bitrates) from the scan; device capability profile | Play button; Player; Player: info overlay shows "Direct play" |
| VID-002 | Playback decision made in the shared core from measured capabilities | The right path (direct, remux, audio conversion, transcode) is chosen without guesswork | Plex yes (detail unverified); Jellyfin yes, with documented open device profiles; Emby yes (unverified) | High: Jellyfin Android TV capability bugs block direct play (HDR issue #49, 127 comments; #5316) | R2 | One Rust decision function is compiled into server and clients, so both reach the same verdict; capabilities are reported by our own player rather than inferred | Decision engine taking stream index, device profile, user policy and live bandwidth budget, returning a verdict plus a structured reason list | None directly; feeds the pre-play sheet, info overlay and admin sessions |
| VID-003 | In-process remux for browsers and TV web runtimes | Files in a container the device cannot open still play, picture and sound untouched | Plex yes ("Direct Stream"); Jellyfin yes ("Remux"); Emby yes (unverified) | High: container mismatch is named as a leading cause of avoidable transcodes | R2 | Pure-Rust remuxer inside the server, no FFmpeg process launched; the scan-time segment map makes playlists exact and lets any segment be produced on its own | Remuxer from MKV and MP4 to fragmented MP4 and HLS; segment map; small segment cache with eviction | Transparent; Player: info overlay names the reason |
| VID-004 | Audio-only conversion with the video copied | A film with TrueHD or DTS plays in a browser that cannot decode them, picture untouched | Plex yes (part of Direct Stream); Jellyfin yes (#516, completed); Emby yes (unverified) | Medium: common for browsers and TVs; no vote count found | R2 | Parity on method; the audio decode runs in the sandboxed worker and is cheap on any hardware | Sandboxed FFmpeg audio job fed by the remuxer, output muxed back into fMP4 | Player: info overlay; Player: tracks sheet marks converted tracks |
| VID-005 | Full video transcode as a sandboxed last resort | Anything plays on anything, at a cost in quality and CPU | Plex, Jellyfin and Emby yes, all on FFmpeg; Jellyfin has had FFmpeg argument-injection advisories in 2023, 2025 and 2026 | High: needed for weak devices and links; transcoder isolation is listed as a gap at every rival | R2 | FFmpeg runs with no network and access only to the one file it was handed, and command lines are never built from metadata; speed and quality are parity (same FFmpeg, same encoders) | Sandboxed worker per OS; job supervisor; HLS output; kill on session end; software encoders | Player: info overlay; Player: quality sheet shows transcode tiers only when allowed; Admin: sessions |
| VID-185 | Transcode target codec, with HDR kept on HEVC output | When a transcode is needed, the output is HEVC or AV1 where the client decodes it, saving upload bandwidth, and HEVC output keeps HDR | Plex request 491 votes; Emby thread 505 replies; Jellyfin #1694 and #3269 (research) | High: 491 votes | Later | The decision engine picks the output codec from the client's measured decoders (CLI-047 capability report), so HEVC is used only where it plays; parity on encoder speed. Later, with VID-006, because software HEVC and AV1 encoding is too slow for small servers | Codec choice in the decision engine; HDR metadata passthrough in the sandboxed encoder | Player > Playback info; Admin > Transcoding |
| VID-006 | Hardware-accelerated transcoding, free | More simultaneous transcodes on an integrated GPU | Plex yes (Plex Pass); Jellyfin yes and free (Intel, AMD, Nvidia, Apple, Rockchip); Emby yes (Premiere) | High: Plex AMD VCN request 848 votes, Raspberry Pi 4 request 409; paywall theme | Later | Behind Jellyfin until this ships: Jellyfin has free Intel, AMD, Nvidia, Apple and Rockchip acceleration, while R2 transcodes are software-only in the sandbox, so a low-power server may manage one 1080p transcode or none (capacity published in ADM-010). When it ships, the price is parity with Jellyfin and the edge is that the GPU path also runs inside the sandbox, which needs its own isolation design first. | GPU device access inside the sandbox; per-vendor back ends; software fallback | Admin: transcoding; Admin: health |
| VID-007 | Transcode only a short way ahead of the viewer | A paused or abandoned transcode stops burning CPU | Plex (unverified); Jellyfin partial (#474, 106 votes, started); Emby (unverified) | Medium: 106 votes; matters on shared low-power hosts | R2 | Because any segment can be produced independently from the segment map, the worker only works in a window around the playhead and stops when the client stops asking | Look-ahead window per session; idle timeout; segment cache | Admin: sessions shows worker state |
| VID-008 | Seek without restarting the stream on remux and transcode paths | Jumping around a remuxed or transcoded film feels like direct play | Not covered for any rival; Plex's new Apple TV player is reported to scrub worse than the native tvOS player | Medium: scrubbing complaints in the 268-post Plex Apple TV thread | R2 | The segment map gives exact keyframe boundaries, so a seek asks for the segment that holds the target and nothing upstream restarts | Random-access segment production from the segment map | Player seek bar |
| VID-009 | Transcode sandbox self-test and an honest "unavailable" | The owner knows transcoding works before a guest hits an error, and is told plainly when it cannot | Plex no (unverified); Jellyfin planned (#450, 423 votes; transcode test #223, 40); Emby (unverified) | High: 423 votes | R2 | At start-up and on demand the server proves the sandbox can launch on this kernel and runs a short encode; if it cannot, transcoding is reported off instead of being run unsandboxed | Start-up check; short test encode; health record shared with the operations map's doctor check | Admin: health; setup wizard; banner when transcoding is off |
| VID-010 | Per-user playback-mode rights | The owner decides who may trigger remuxes, audio conversions or video transcodes | Plex no per-user control (unverified); Jellyfin yes, separate switches; Emby similar (unverified) | Medium: Emby "force direct play only" 165 replies; Plex "disable 4K transcoding" 249 votes | R2 | The policy is an input to the core decision, so a refused stream comes back with its reason and the alternatives (another version, a download) instead of a generic error | User policy store; decision engine input | Admin: user policies; Player error card with alternatives |
| VID-011 | Short-lived, session-bound stream URLs | Nobody can pull a film or subtitle by guessing or replaying a URL, and tokens never land in logs | Plex tokens commonly travel in URL parameters (unverified); Jellyfin #5415 listed unauthenticated stream and subtitle endpoints and an api_key URL parameter; Navidrome shared streams kept working after deletion | High: security is the fourth-ranked pain theme; #5415 had 114 +1 | R2 | Per ADR 1, every stream, subtitle, font and preview URL is signed, scoped to one object and one session, and refreshed during long films; revoking a session stops playback: new requests fail at once, and an in-flight response is cut by the ACC-122 mechanism | URL signing and silent refresh; per-object authorisation on every media route; cross-user route tests | None visible |
| VID-012 | Fast start, measured and published | The picture appears quickly after pressing play | Not measured for any rival in the research | Medium: the demand research proposes time to first frame as a published benchmark | R2 | Direct play needs no probe and no process launch, and the first segment's byte range is already in the segment map; this stays a design goal until benchmarked | Benchmark harness; first-segment byte ranges | Player loading state; published benchmark page |
| VID-013 | Multiple versions with an informative picker | Choose 4K or 1080p, theatrical or extended, seeing codec, HDR, audio and size | Plex yes ("More info in Play Version" request, 266 votes); Jellyfin yes, episode versions in 12.0; Emby yes (unverified) | Medium: 266 votes | R2 | Every fact comes from the scan-time parse, plus whether this device will direct play each version, computed on the client from the synced index | Versions grouped under one title (library map); per-version stream index | Pre-play sheet; Title detail page |
| VID-014 | Best existing version chosen automatically | A phone on mobile data gets the 1080p copy instead of a transcode of the 4K one | Not covered for any rival | Medium: follows from the 1,289-vote "default to max" request and the 188-vote per-user cap request | R2 | The decision engine ranks existing versions before it considers any transcode | Per-version decision preview; bandwidth estimate | Pre-play sheet says which version and why |
| VID-015 | "Plays directly here" indicator before pressing play | Know before you start whether this device will need a remux or a transcode | Plex no (unverified); Jellyfin requested (#2832, 5 votes); Emby (unverified); nobody does it well | Low: 5 votes, though it answers the "why does it play like this" pain point | R2 | The core decision runs locally against the synced stream index, so the badge costs no server call and works offline | Stream index in the sync payload | Title detail page badge; Pre-play sheet; optional library filter |
| VID-016 | AV1 direct play and remux | Modern AV1 files play natively | Plex AV1 on some clients (unverified); Jellyfin AV1 direct play on TV clients and AV1 stream copy into fMP4 in 12.0; Emby (unverified) | Medium: browser and device codec support keeps moving | R2 | Parity; libmpv decodes AV1 where the device can, and the remuxer carries it into fMP4 for browsers | AV1 in the parser and remuxer; capability reporting | Player: info overlay |
| VID-017 | VVC (H.266) playback | Newest-codec files play on capable clients | Infuse 8.4 added initial VVC decoding; others not covered | Low: no demand signal found | Later | Parity at best, depending on the decoders libmpv carries | Parser support | Player: info overlay |
| VID-018 | Correct shape and orientation | Anamorphic and rotated video display correctly | Jellyfin 12.0 fixed anamorphic direct play on Tizen and added a rotated-video option for Android TV; others (unverified) | Low: small but real bugs elsewhere | R2 | The parser reads pixel aspect and rotation at scan and every player applies them; parity | Aspect and rotation in the stream index | None |
| VID-019 | Deinterlacing on the client | DVD rips and recordings look smooth without a server transcode | Not covered for files; for live TV Jellyfin has a deinterlacing request (1 vote) | Low: 1 vote | R2 | libmpv deinterlaces on native clients; browsers that cannot fall back to the sandboxed transcode (whether browsers deinterlace is unverified) | Interlace flag in the stream index | Settings: playback (auto, on, off) |
| VID-020 | Disc folders and images | Ripped DVDs and Blu-rays (ISO, BDMV, VIDEO_TS) play their main title | Plex limited (unverified); Jellyfin limited (#179, 70 votes); Infuse lists ISO, BDMV and VIDEO_TS | Low: 70 votes | Later | Parity at best; native clients could hand the structure to libmpv (unverified over a network) | Disc structure detection at scan; main-title selection | Title detail page; Pre-play sheet (title choice) |
| VID-021 | Full disc menus | Blu-ray and DVD menus as on a disc player | Jellyfin request #4124 (4 votes); others not covered | Low: 4 votes | No | Tiny demand, impossible in browsers, and Blu-ray menus need a Java runtime on every client (unverified) | n/a | n/a |
| VID-022 | Open in an external player | Hand a film to VLC, mpv or Infuse | Plex (unverified); Jellyfin desktop request #227 (65 votes); Infuse is itself the external player many people choose | Low: 65 votes | Later | The external player gets a short-lived URL scoped to one file, never a long-lived token | Signed URL long enough for a full film, revocable | Title detail page "Open in" menu |

### Quality, bandwidth and remote playback

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| VID-023 | Original quality by default, remote included | No needless transcode because a client defaulted low | Plex apps long defaulted to 2 Mbps or 720p remotely; Jellyfin partial (unverified); Emby added a per-user automatic remote quality option in 4.9.3 | High: Plex "Default All Clients to Max Internet Streaming" has 1,289 votes, 1,728 posts and 70,974 views | R2 | First-party clients start with the original and step down only on a measured shortfall; the default is never 720p | Decision engine default; link measurement | Settings: playback (default quality) |
| VID-024 | Link check and honest choices when the original will not fit | Instead of a silent downgrade, a short choice: original with a bigger buffer, another version, audio-only conversion, or download for later | Plex "automatically adjust quality" turns on transcoding; Jellyfin has no adaptive streaming (#3436, 55; #3680, 56); Emby (unverified) | High: remote quality is a named pain point in the playback research | R2 | The client measures the link (over iroh when remote) and compares it with the file's peak bitrate per segment from the segment map, so the warning is specific to this film | Peak bitrate per segment at scan; bandwidth probe endpoint; decision engine | Pre-play sheet ("this connection is slower than this film"); choice remembered per device |
| VID-025 | Quality picker that says what "Original" is | Each option shows its real bitrate and codec, starting with the source | Plex yes, but the new Apple TV app hides what Original means; Jellyfin yes; Emby yes; Infuse shows extra video details | Medium: Apple TV users say they cannot tell what Original means | R2 | The list holds only real options (the original, other versions, transcode tiers when allowed), each with its true figures | Per-version bitrates; policy check | Player: quality sheet |
| VID-026 | Pre-made optimised versions | Smaller copies made at idle, so remote or mobile play needs no live transcode | Plex yes, "Optimize" (unverified); Jellyfin no (#570, 979 votes, declined Nov 2025); Emby yes, "Convert" (Premiere) | High: 979 votes | R2 | Made by the sandboxed worker on a schedule and cached, so a second device never pays again; the same job produces transcoded downloads for the offline map R2 as an opt-in, low-priority job ("make a smaller copy" per title or by rule): slowness on a small server matters less at idle. Jellyfin declined this in November 2025, so it is a clear opening. | Scheduled transcode jobs; resumable output files; size and time estimates; storage accounting | Title detail page ("make a smaller copy"); Admin: optimisation rules; Pre-play sheet option |
| VID-027 | Adaptive bitrate ladder | Quality follows the network second by second | Plex auto-adjust (by transcoding); Jellyfin no (#3436, 55; #3680, 56); Netflix uses per-title ladders (unverified) | Medium: 55 and 56 votes | Later | Parity at best: real adaptive streaming needs several renditions, which means transcoding; a ladder could be built from existing and pre-made versions instead | Multi-rendition HLS from existing versions | Player: quality sheet ("Auto") |
| VID-028 | Upload budget and per-user caps that respect direct play | One remote viewer cannot starve the house, and a cap never quietly starts a GPU transcode | Plex global cap only (Plex Pass; per-user request 188 votes); Jellyfin per-user remote cap; Emby per-user (unverified) | Medium: 188 votes; Jellyfin fair-split requests #1504 and #1012 | R2 | The cap is a decision input: it picks among existing versions or suggests a download, and transcodes only if the owner allows that for the user | Live upload accounting; fair share; per-user policy | Admin: bandwidth; Player notice when a cap applies |
| VID-029 | Owner quality floor and default | The owner sets a better default and a minimum for remote viewers | Plex requested since 2017 (138 posts, 27,989 views); Jellyfin #3763 (12) and #2911 (9); Emby per-user automatic remote quality (4.9.3) | Medium: long-running Plex thread | R2 | A policy rule in the same decision engine, not a per-app setting each guest has to find | Policy store | Admin: user policies |
| VID-030 | Separate limits for remux and transcode | Remux at full bitrate, cap only transcodes | Plex (unverified); Jellyfin requested (#2596, 11); Emby (unverified) | Low: 11 votes | R2 | Each delivery path has its own limit in the policy model | Policy fields | Admin: bandwidth |
| VID-031 | Lower quality on mobile data only | Save data on cellular without changing home playback | Plex, Jellyfin and Emby yes (all unverified) | Medium: basic expectation | R2 | Parity; on cellular the client prefers an existing smaller version before asking for a transcode | Network type reported by the client | Settings: playback (mobile data) |
| VID-032 | Fewer stalls on long or lossy links | Faster starts and steadier remote playback | Plex requested (195 votes); Jellyfin planned (#2176, 231 votes); Emby (unverified) | Medium: 231 and 195 votes | R2 | iroh's QUIC transport can carry several segment fetches at once without one lost packet holding up the rest; expected, not yet measured | Parallel segment requests over iroh | None |
| VID-033 | Playback has priority over downloads and syncs | An offline sync never makes a live stream stutter | Plex yes, downloads scale back; Jellyfin and Emby not checked | Low: no vote count | R2 | The upload scheduler ranks live sessions above download and sync transfers | Transfer priority classes | None |
| VID-034 | Read-ahead from slow or sleeping disks | No spin-up stall when the next episode starts | Plex no (unverified); Jellyfin requested (#2534, 25); Emby (unverified) | Low: 25 votes | Later | The segment map gives the next file's first byte ranges, so the server can read them into memory during the credits | Small read-ahead cache tied to autoplay | None |
| VID-035 | Remote playback is free | Watch your own server away from home without a subscription | Plex paid since April 2025 (Plex Pass, or Remote Watch Pass at $2.99 a month or $29.99 a year since 1 June 2026); Jellyfin free; Emby free (unverified) | High: Plex's paywall is the top-ranked pain theme | R2 | No central account and an AGPL licence leave nothing to paywall; against Jellyfin the edge is setup, not price. Native clients embed iroh; the project's own connection-success figures will replace iroh's quoted rate of about 90% direct (unverified for mobile carriers and TVs). No open port; whether there is no fee behind CGNAT depends on the relay decision (ACC-101). Browsers still need the owner's proxy or VPN until ACC-102. | iroh endpoint; device keys | None |

### HDR, Dolby Vision and display

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| VID-036 | HDR10 direct play | HDR on HDR screens | Plex, Jellyfin and Emby yes; Infuse on Apple TV and Kodi best | High: basic expectation | R2 | Parity; capability reporting by our own player avoids the wrong-capability transcodes reported on Jellyfin Android TV | HDR metadata in the stream index | Player: info overlay; HDR badge in the version picker |
| VID-037 | HDR10+ without black screens | Dynamic metadata used where supported and safely dropped where not | Plex (unverified); Jellyfin 10.11 detects HDR10+ and strips conflicting metadata; Infuse on compatible hardware | Low: no vote count | R2 | Parity; the parser flags HDR10+ at scan and the remuxer can drop it for clients that misbehave | HDR10+ flag; remuxer strip option | None |
| VID-038 | HLG | Broadcast HDR looks right | Jellyfin 12.0 tone maps HLG with BT.2446 Method B; others (unverified) | Low: no vote count | R2 | Parity; libplacebo handles HLG on native clients | HLG flag | None |
| VID-039 | Dolby Vision profiles 5 and 8 on native clients | Streaming-style DV looks right on any display, with no green-and-purple picture | Plex's new apps reported failing on profile 5, and Plex cannot transcode it; Jellyfin software DV tone mapping (10.10) and a dvh1 HLS variant (12.0); Infuse full P5 on Apple TV (Pro) | High: Dolby Vision is a top playback pain point | R2 | libmpv reshapes profiles 5 and 8 and tone maps on the client, so no display shows green and purple. It does not output a Dolby Vision signal; Infuse and Plex on Apple TV do, through Apple's player, so for DV-capable TVs this is behind them. True DV output on Android TV is VID-184; on Apple devices it is part of VID-178. | DV profile and RPU presence in the stream index | Player: info overlay; DV badge in the version picker |
| VID-184 | True Dolby Vision output on Android TV | A DV-capable TV receives a real Dolby Vision signal, not a tone-mapped picture | Plex and Infuse output DV on Apple TV (research); Android TV players (unverified) | High: Dolby Vision is a top playback pain point | Later | Uses the platform decoder for DV titles on devices that support it. Later unless the native player module can switch engines per title (open decision); until then VID-039 is behind Infuse and Plex for DV-TV owners | DV signalling in the stream index (VID-039) | Player settings > Dolby Vision |
| VID-040 | Dolby Vision fallback to HDR10 in the remuxer | A device that cannot take DV gets the HDR10 base layer instead of a broken picture, with no transcode | Jellyfin handles P7 on non-DV HDR displays (#2557); Plex fails on some DV files (feedback thread); Emby (unverified) | High: same pain point | R2 | The remuxer removes DV metadata from files that carry an HDR10 base layer; this is byte work, not decoding | HEVC NAL unit handling in the remuxer | Player: info overlay names the fallback |
| VID-041 | Dolby Vision profile 7 converted to 8.1 | UHD Blu-ray remuxes play as single-layer DV on devices that accept it | Nobody fully: Jellyfin request #3838; people convert offline with dovi_tool or DV7toDV8; Apple TV cannot take dual layer, so Infuse falls back to HDR10 | Medium: active threads and tools, no single vote count | R2 | The remuxer rewrites the RPU and drops the enhancement layer on the fly, the transformation dovi_tool performs; its MIT-licensed Rust library may be usable if it meets the core's no-unsafe and no-panic rules | Per-frame RPU parsing and rewriting; correct dvh1 and dvhe sample entries in fMP4 and HLS | Settings: playback (DV preference per device) |
| VID-042 | Profile 7 enhancement layer composed in full | Disc-grade DV for purists | Nobody: Jellyfin's enhancement-layer work is an opt-in pull request that warns it can exhaust GPU memory; mpv does not compose the layer | Low: enthusiast niche | No | Needs two simultaneous HEVC decodes per stream, which no target client can promise | n/a | n/a |
| VID-043 | HDR-to-SDR tone mapping on the client | HDR films look right on SDR screens while the server stays idle | Plex, Jellyfin and Emby do it on the server; mpv does it on the client with libplacebo | High: server tone mapping is Plex Pass and Emby Premiere, and costs GPU time everywhere | R2 | libmpv tone maps on the device and the server sends the original | None beyond the stream index | Settings: playback (tone-mapping quality on weak devices) |
| VID-044 | Server tone mapping as a fallback | A browser on an SDR screen still gets a correct picture from an HDR file | Plex yes (Plex Pass); Jellyfin yes and free, with hardware paths; Emby yes (Premiere) | Medium: only for clients that cannot tone map | R2 | Parity at best, and slower than Jellyfin's GPU paths until VID-006; it runs in the sandbox and costs nothing | Sandboxed transcode with tone-mapping filters | Player: info overlay |
| VID-045 | Match frame rate and dynamic range | The TV switches to the film's frame rate and HDR mode, so there is no judder | Client dependent everywhere; Plex HTPC switches; Jellyfin Android TV has it but forces PCM audio as a side effect (#4067); Apple TV "Match Content" | Medium: judder and capability complaints on Android TV | R2 | Owning the player means calling the OS display-mode APIs directly; an acceptance test checks that passthrough survives the switch | Frame rate and HDR fields in the stream index | Settings: playback (match frame rate, match dynamic range) |
| VID-046 | Choose DV or HDR10 per device | Force HDR10 on a TV with a poor DV mode | Plex (unverified); Jellyfin requested (#2511, 19); nobody | Low: 19 votes | R2 | A per-device preference feeds the decision, and the remuxer already knows how to drop DV | Per-device settings | Settings: playback (per device) |
| VID-047 | Comfortable subtitles on HDR | White subtitles do not glare at full HDR brightness | Plex users call them blindingly bright on Apple TV; Jellyfin requested (#3358, 3); Infuse 8.2 has a setting; mpv 0.41 has subtitle peak controls | Medium: a comfort complaint with a low vote count | R2 | Dimmer subtitles on HDR content by default through libmpv's subtitle peak control, adjustable | None | Settings: subtitles (HDR brightness) |
| VID-048 | Display resampling and interpolation | Judder-free 24p on a fixed 60 Hz screen | mpv display-resample; Infuse relies on Apple TV Match Content | Low: no vote count | Later | Parity with mpv in the desktop shell | None | Settings: playback (desktop) |
| VID-049 | AI upscaling | Low-resolution files look sharper | Infuse 8.2; mpv with Nvidia RTX and Intel VSR on Windows | Low: no vote count | Later | Parity where the platform offers it | None | Settings: playback |
| VID-050 | Native HDR on Linux desktops | Real HDR output on Wayland and DRM, not only tone mapping | mpv 0.40 and 0.41; Kodi 22 on embedded Linux (release candidate) | Low: no vote count | Later | Parity, inherited from libmpv in the desktop shell | None | None |

### Audio

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| VID-051 | Preferred audio language | The right dub, or the original language, by default | Plex, Jellyfin and Emby yes | High: basic expectation | R2 | Parity; the language list is part of the synced user settings | User language preferences | Settings: audio |
| VID-052 | Track choices remembered for a series | Pick the Japanese track once for the whole show | Plex per-series control since 2023; Jellyfin done (#194, 335 votes); Emby (unverified) | Medium: 335 votes | R2 | Choices are events in the user's log, so they survive a database rebuild and reach every device | Per-series track preference events | Player: tracks sheet ("use for this series") |
| VID-053 | Track picker before playback | Choose audio, subtitles and version before pressing play | Plex missing in the new iOS app preview; Jellyfin web yes; Emby (unverified) | Medium: listed among Plex's iOS regressions | R2 | Tracks come from the synced index, so the sheet opens instantly and offline | Stream index in the sync payload | Pre-play sheet |
| VID-054 | Clear labels for audio and subtitle tracks | Tell "Director's commentary" from "English 5.1", and audio description from the main mix | Plex "show names for audio tracks and subtitles in all apps" open with 417 votes; Jellyfin #1206 (5) and #164 (7); a new Plex Android build reportedly listed tracks as unknown | High: 417 votes | R2 | The parser reads MKV track names and flags (default, forced, commentary, hearing impaired, visual impaired, original) at scan and builds labels from them, grouped by language | Track names and flags in the stream index | Player: tracks sheet; Pre-play sheet |
| VID-055 | Accurate audio format labels | Trust that "Atmos" or "DTS:X" on screen is true | Plex partial (701-vote request); Jellyfin partial; Emby requested (219 replies) | High: 701 votes | R2 | The core inspects audio stream headers at scan; whether every Atmos and DTS:X variant can be detected without decoding is (unverified) | Audio format detail in the stream index | Version picker; Player: tracks sheet; Player: info overlay |
| VID-056 | Compressed surround passthrough | AC3, E-AC3 (including Atmos) and DTS reach the receiver untouched | Client dependent at Plex, Jellyfin and Emby; Kodi and Infuse best | High: the home-theatre baseline | R2 | Parity with Kodi; our player reports its real passthrough ability, so the server never converts audio the receiver could take | Per-device passthrough capabilities | Settings: audio devices |
| VID-057 | Lossless passthrough | TrueHD and DTS-HD MA, including Atmos and DTS:X, bit for bit | Plex's new apps reported transcoding TrueHD; Jellyfin Shield passthrough issue #281 open since 2020 (50 comments); Kodi best on Android TV boxes (unverified) | High: long-running issues at both rivals | R2 | libmpv passes through where the platform allows, Android TV and desktop first; on Apple TV it depends on tvOS | Per-device capabilities | Settings: audio devices; Player: info overlay |
| VID-058 | Stereo downmix on the client | Clearer dialogue from 5.1 on TV speakers, with no server work | Jellyfin offers downmix algorithms (10.9, 10.10); Plex and Emby (unverified) | Medium: Jellyfin expanded it across two releases | R2 | libmpv downmixes on the device with a selectable mix, so a stereo device never forces an audio conversion | None | Settings: audio (downmix style) |
| VID-059 | Dialogue lift and night mode | Hear whispers without explosions waking the house | Plex dialog boost and loudness levelling (Plex Pass, since 2026-09-15) transcode the audio; Jellyfin no; Apple TV "Enhance Dialogue" best | High: Plex's own request had 170 votes; Netflix says nearly half of US viewing hours use subtitles, partly for this reason | R2 | Built from libmpv's audio filters on the client: no server transcode and no paywall | None | Player: audio quick menu; Settings: audio |
| VID-060 | Volume boost | Quiet files made louder | Infuse 8.0 with quick access; Jellyfin "volume above 100%" requested (124 votes) | Medium: 124 votes | R2 | Client gain with a limiter, optionally saved per title | None | Player: audio quick menu |
| VID-061 | Audio delay during playback | Fix lip sync caused by Bluetooth or a receiver | Plex only on some desktop clients (request open since 2014, 59 votes) and missing from the new Apple TV app; Jellyfin requested (#3451, 19); mpv and Kodi best | Medium: 59 and 19 votes, still active in Sept 2026 | R2 | libmpv's audio delay on every client, with steps that grow on repeated presses | None | Player: audio quick menu; remote keys |
| VID-062 | Audio delay saved per output device | A soundbar's lag is fixed once and applied to every film | Nobody; Jellyfin per-codec presets (#3119, 3) and per-series sync (#2163, 18) requested | Low: 18 and 3 votes | R2 | The delay is stored per user, device and audio output, because the lag belongs to the speaker, not the film; it syncs as a setting | Per-output settings in sync | Settings: audio devices |
| VID-063 | External audio files | Separate commentary or dub files beside the video play as tracks | Plex requested (329 votes); Jellyfin subfolder support requested (#2562, 17); Infuse 8.4 added it | Medium: 329 votes | R2 | The scanner pairs audio sidecars with the video; native clients load them directly and browsers get them muxed in by the remuxer | Sidecar audio matching; remux of an external track | Player: tracks sheet |
| VID-064 | Switch audio track mid-playback | Change language without restarting | Plex, Jellyfin and Emby yes (unverified); Infuse 8.5.4 added it for server transcodes | Medium: no vote count | R2 | Instant on direct play; on the remux path the segment map lets the server produce segments with the new track from the current position | Per-segment remux with track selection | Player: tracks sheet |
| VID-065 | Listen-only mode for a video | Hear a concert film or stand-up special with the screen off | Jellyfin requested (#533, 34); nobody | Medium: 34 votes | R2 | Hands the session to the music player's background audio and lock-screen controls; when remote, the remuxer sends only the audio track | Audio-only remux | Player menu ("listen only"); Lock screen and notification |
| VID-066 | Core-only audio per device | Avoid a soundbar's bad Atmos decoder | Infuse 8.1.7 ("disable Atmos") | Low: no vote count | R2 | A per-device preference makes the player send the core stream; parity | None | Settings: audio devices |
| VID-067 | Seamless audio format changes on Apple TV | No dropouts when switching between stereo and Atmos | tvOS 26.4 "Continuous Audio Connection" | Low: no vote count | Later | Inherited from the platform by the Apple client | None | None |

### Subtitles: formats, rendering and files

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| VID-068 | Plain text subtitles everywhere | SRT and WebVTT work on every client | Plex, Jellyfin and Emby yes | High: basic expectation | R2 | Parity; parsed by the memory-safe core and never passed to FFmpeg by path (Jellyfin had a subtitle-path argument-injection advisory in June 2026) | Subtitle parsing in the core | Player: subtitle quick menu |
| VID-069 | Styled ASS/SSA exactly as authored | Anime signs, karaoke and positioned lines look as intended, using the fonts shipped in the file | Plex's Android TV player cannot render ASS (staff confirmed, in a 397-post thread with 27,358 views) and the new iOS app mis-renders it; Jellyfin web renders ASS with libass-wasm; mpv, Kodi and Infuse best | High: the biggest subtitle gap in Plex; burn-in is the most expensive kind of transcode | R2 | libass inside libmpv on native clients; in browsers the remuxer passes subtitle packets through untouched as a side stream and a WebAssembly libass renders them; MKV font attachments are indexed at scan and served by signed URL Parity with Jellyfin web in browsers, which already renders ASS with libass-wasm; ahead of Plex on Android TV and iOS. Whether font-attachment indexing differs from Jellyfin's is unverified. | Font attachment index; subtitle side stream; signed font URLs | Player |
| VID-070 | Image subtitles rendered on the client | PGS, VobSub and DVB subtitles never force a burn-in | Plex often burns them in (unverified); Jellyfin web renders PGS (10.10) and VobSub (12.0), with PGS problems reported on Android TV; mpv, Kodi and Infuse best | High: a named pain point in the playback research | R2 | libmpv on native clients; pure-Rust PGS, VobSub and DVB decoders in the core, compiled to WebAssembly for the web client Parity with Jellyfin web, which renders PGS on the client; the edge is the native TV clients. | Image subtitle side stream; decoders in the core | Player |
| VID-071 | Subtitles that load fast and stay in sync after seek and resume | No late or drifting subtitles after jumping around | Jellyfin #2547 open since 2020 (103 +1, 151 comments, its most-reacted open server issue) and web #4346 (62 comments); Plex yes in most clients (unverified) | High: a Hacker News commenter called subtitles Jellyfin's "Achilles heel" | R2 | Native clients read subtitles from the file itself through libmpv; on the web, cues ride the same segment timeline as picture and sound, so a seek fetches the cues for that segment | Subtitle packets indexed per segment | Player |
| VID-072 | Sidecar subtitle files | `.srt` and `.ass` files beside the video, or in a subtitles folder, are picked up | Plex yes; Jellyfin yes (subfolder support requested, #2562, 17); Emby yes (unverified); mpv best | Medium: 17 votes for subfolders | R2 | Parity plus subfolders; language and flags are read from the file name | Sidecar matching at scan | Player: subtitle quick menu |
| VID-073 | One subtitle for every version | A single `.srt` serves both the 1080p and 4K copies | Jellyfin requested (#1613, 22); nobody | Low: 22 votes | R2 | A sidecar attaches to the title rather than the file, with a per-version offset when the cuts differ | Title-level sidecar link; per-version offset | Player: subtitle quick menu |
| VID-074 | Two subtitles at once | Learn a language with both tracks on screen | Jellyfin primary plus secondary since 10.9 (separate offsets requested, #3275, 6); mpv secondary-sid | Low: 6 votes | R2 | Parity with mpv and Jellyfin, with an offset per track | None | Player: subtitle quick menu (secondary track) |
| VID-075 | Add a subtitle file from the app | Upload a subtitle found elsewhere | Jellyfin has an upload right that was part of a 2026 remote code execution chain (CVE-2026-35031, CVSS 9.9) | Medium: a security lesson more than a vote count | R2 | Uploads are parsed by the core and stored under content-hash names, so no file name ever reaches the filesystem | Upload handler in the core; content-addressed storage; per-user right | Player: subtitle quick menu ("add file"); Title detail page |
| VID-076 | New subtitles appear mid-playback | A subtitle fetched by Bazarr or a plugin shows up without restarting the film | Plex no (Bazarr's docs say to stop and resume); Jellyfin and Emby (unverified); nobody | Low: no vote count | R2 | The server sends an item-changed event to the live session and the player offers the new track | Session event channel | Player notice ("new subtitle available") |
| VID-077 | Burn-in fallback for clients that cannot render | Third-party clients reached through the Jellyfin adapter, such as Roku, still show image subtitles | Plex has a burn-in setting users are told to avoid (51-post thread), and its new TV app broke forced burn-in; Jellyfin can disable burn-in; Swiftfin 1.6 adds forced burn-in | Medium: recurring complaints, no single vote count | Later | Only adapter clients ever need it; it runs in the sandbox and the owner can switch it off | Sandboxed burn-in transcode | Admin: transcoding |
| VID-078 | Machine-generated subtitles | Subtitles for files that have none | Nobody among the rivals; Jellyfin Whisper request (#2143, 61); Bazarr Whisper providers (unverified) | Low: 61 votes | Later | An opt-in plugin that runs on a capable client or in the sandbox, never by default on a low-power server | Plugin with compute and no network | Player: subtitle quick menu ("generate") |

### Subtitles: choice, timing, search and appearance

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| VID-079 | Subtitle modes by language | Off, only with foreign audio, only forced lines, or always | Plex yes (wording unverified); Jellyfin "Smart" mode (unverified); mpv best (subs-with-matching-audio) | High: basic expectation | R2 | Parity | User preferences | Settings: subtitles |
| VID-080 | Fallback subtitle languages | A second and third choice when the first is missing | Jellyfin requested (#3675, 8; a second preferred language request has 95 votes); nobody | Medium: 95 votes | R2 | An ordered language list in the synced user settings | User preferences | Settings: subtitles |
| VID-081 | Forced tracks found in badly tagged files | Foreign-language lines appear automatically even when the file's flags are wrong | Plex turns on tracks tagged forced (unverified); Jellyfin requested (#3404, 15); mpv subs-fallback-forced | Medium: badly tagged files are the norm | R2 | At scan the parser compares cue counts per track (from mkvmerge's statistics tags, unverified) to tell a forced track from a full one, with no decoding | Per-track cue counts in the stream index | Player: subtitle quick menu ("forced" label) |
| VID-082 | SDH or dialogue-only, labelled and preferred | Choose captions with sound cues, or plain dialogue | Netflix ships a dialogue-only track beside CC on new originals; Jellyfin detection requested (#3404, 15) | Low: 15 votes | R2 | Hearing-impaired flags and track names read at scan feed a "prefer SDH" setting | Track flags | Settings: subtitles; Player: subtitle quick menu |
| VID-083 | Subtitle offset with growing steps | Fix a three-second error in a few presses | Plex 50 ms steps (larger steps requested, 79 votes) and offset dropped from the new Apple TV app; Jellyfin web yes, Android TV requested (179 votes); Emby "adjust subtitle delay" 229 replies; mpv best | High: 179 votes and 229 replies | R2 | libmpv's subtitle delay on every client; steps grow with repeated presses | None | Player: subtitle quick menu; remote keys |
| VID-084 | Saved subtitle offsets | The fix sticks for that file, or for the whole series | Plex no (unverified); Jellyfin requested (#2158, 5; #2163, 18); nobody | Low: 18 and 5 votes | R2 | Offsets are events in the user's log keyed to the subtitle track, so they sync and survive rebuilds | Offset events | Player: subtitle quick menu ("save for series") |
| VID-085 | Automatic re-timing against a reference track | A downloaded subtitle is aligned without fiddling | Plex shipped automatic sync in Sept 2024 per the demand research (687-vote request), though the playback research did not confirm it; Jellyfin no; Emby no (unverified); ffsubsync and alass exist outside | High: 687 votes at Plex | R2 | Aligns against a correctly timed text track already in the file, which needs no audio decoding; alass (Rust, GPL-3.0) supports this mode if its licence is confirmed compatible | Alignment job; reference track selection | Player: subtitle quick menu ("sync to embedded track") |
| VID-086 | Re-timing against speech | Out-of-sync subtitles fixed even when no reference track exists | Plex as in VID-085; ffsubsync does it as an outside tool | Medium: same demand as VID-085 | Later | Needs voice detection on decoded audio, so it waits for the analysis decision (sandbox or capable client) | Decoded audio in the sandbox | Player: subtitle quick menu |
| VID-087 | Subtitle search from the player | Find a missing subtitle mid-film | Plex yes with OpenSubtitles (missing in the new iOS preview); Jellyfin plugin from the item menu, not the player (#892, 30; #1713, 28); Emby Premiere only; Infuse free one-tap | Medium: 30 and 28 votes | R2 | A first-party plugin with an explicit network grant, so the user can see that a search tells a third party what they are watching | Plugin framework with network grants | Player: subtitle quick menu ("search"); grant prompt |
| VID-088 | Exact-release matches | Results that match this exact file are marked | Plex yes (a star marks hash matches); Jellyfin via plugin (unverified) | Medium: no vote count | R2 | The OpenSubtitles file hash (said to need only the file size and its first and last 64 KiB, unverified) is computed during the scan, so matches come back instantly | Hash per file at scan | Subtitle search results |
| VID-089 | More subtitle providers | Better coverage for small languages | Plex OpenSubtitles only; Jellyfin requested (#2225, 29); Bazarr | Low: 29 votes | Later | More plugins, each with its own grant | Plugin framework | Subtitle search results |
| VID-090 | Background fetch of missing subtitles | Missing subtitles appear without asking | Plex no (unverified); Jellyfin plugin task; Bazarr (unverified as best) | Low: requests declined or open | Later | A plugin job, or Bazarr through the ecosystem integration, with VID-076 showing results live | Scheduled plugin jobs | Admin: plugins |
| VID-091 | Subtitle appearance that starts from the OS caption settings | Size, font, colour, background, outline and position, starting from what the user already chose system-wide | Plex yes, but the new Apple TV app lost outline and size control and the new iOS app ignored OS font settings; Jellyfin web yes (#161, 188 votes) and Android TV capped size in a Sept 2026 regression; Jellyfin Roku 3.1.9 styling | High: 188 votes and accessibility complaints | R2 | One style model applied by libass and libmpv on native clients and by the web renderer, seeded from the OS caption preferences | Synced style settings | Settings: subtitles; Player: subtitle quick menu |
| VID-092 | Restyle during playback with a live preview | Adjust subtitles without leaving the film | Apple TV app (tvOS 26.4) best; Emby docks its dialog so subtitles stay visible; Jellyfin preview requested (#3224, 16) | Low: 16 votes | R2 | Client feature; parity with the Apple TV app | None | Player: subtitle quick menu |
| VID-093 | Subtitles on mute or after a skip back | The line you missed appears automatically | Apple TV (tvOS 18) does both; Emby 4.11 beta on skip back; Plex and Jellyfin no | Medium: Apple TV and Emby are moving the bar | R2 | Client feature; parity with the Apple TV app | None | Settings: subtitles |
| VID-094 | Subtitle position for wide films | Subtitles inside the picture or in the black bars | Jellyfin wide-aspect positioning PR (#4816, 17 reactions) | Low: 17 reactions | R2 | Client feature | None | Settings: subtitles |

### Seeking, chapters and previews

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| VID-095 | Chapter list in the player | Jump to a scene | Plex yes (unverified in the new apps); Jellyfin web yes, Android TV quick access requested (#3604, 3); Kodi and mpv best | Medium: expected by disc rippers | R2 | Chapters are parsed at scan and synced, so the list opens instantly and offline | Chapters in the stream index | Player: chapters panel |
| VID-096 | Chapter names on the seek bar | Know which scene you are scrubbing into | Jellyfin 12.0; others (unverified) | Low: no vote count | R2 | Parity | Chapters in the stream index | Player seek bar |
| VID-097 | Chapter thumbnails without stored images | A picture for each chapter | Plex yes (unverified); Jellyfin chapter image extraction; Emby option during scans | Medium: no vote count | R2 | The thumbnail is the nearest keyframe, cut by the remuxer and decoded on the client, the same path as VID-098 | Single-keyframe endpoint | Player: chapters panel; Title detail page |
| VID-098 | Scrub previews on demand | Pictures while dragging the timeline, with no pre-generation and nothing stored | Plex pre-generates them (10+ CPU minutes and 10–50 MB per film) and the new iOS preview shipped without them; Jellyfin pre-generates, about 100x faster since 10.10, and on-demand is requested (#4143, 4); mpv's thumbfast does it on demand | High: Plex's cost, and scrub thumbnails are among Plex's iOS regressions | R2 | The segment map knows where every keyframe is; the server cuts one into a tiny file with the remuxer and the client decodes it on its own hardware (a second small libmpv, or WebCodecs in browsers) Per-device probe: many cheap sticks have one hardware decoder instance, so devices that cannot decode a second stream fast enough get no previews until VID-099 (Later). TV clients in R2 may therefore ship without scrub previews. | Single-keyframe endpoint; request rate limiting | Player seek bar |
| VID-099 | Pre-generated previews as a fallback | Previews on weak TVs and slow links | Plex and Jellyfin yes | Medium: needed only where on-demand decoding is too slow | Later | Sprite sheets made by the sandboxed worker at idle with keyframe-only decoding, as Jellyfin 10.10 does, and only for clients that need them | Sandbox analysis job; sprite storage | Player seek bar |
| VID-100 | Spoiler-safe previews | Thumbnails of the unseen part can be blurred | Requested on both the Plex and Jellyfin boards (counts not recorded) | Low: requested on both boards | Later | Client blur driven by the watch position | Watch position | Settings: playback |
| VID-101 | Configurable skip intervals | Choose 10, 15 or 30 second jumps | Plex partial (453-vote request; 10 and 30 second buttons missing in the iOS preview); Jellyfin web done (#3456) | High: 453 votes | R2 | Parity; synced per user | User settings | Settings: playback; Player transport bar |
| VID-102 | Double-tap and swipe to seek | Phone-style seeking gestures | Plex's new app lost gestures; Jellyfin requested (#131, 109), and Android 2.7.0 added horizontal seek; Streamyfin | Medium: 109 votes | R2 | Parity | None | Player (touch) |
| VID-103 | Frame stepping | Step forward and back one frame | Jellyfin 12.0 (comma and full stop keys); mpv also steps backwards | Low: no vote count | R2 | libmpv steps both ways; parity with mpv | None | Player (keyboard, while paused) |
| VID-104 | A-B loop | Repeat a passage | mpv only | Low: no vote count | Later | libmpv; useful for language learners | None | Player menu |
| VID-105 | Keyboard, remote keys and controllers | Full control without a mouse; dedicated rewind and fast-forward keys work | Plex HTPC controller support; Jellyfin 12.0 remote keys and controller and keyboard fixes; mpv best | Medium: Jellyfin keyboard requests (#267, 11) | R2 | Parity; one input map shared by every client, with an on-screen help card | None | Player; keyboard help overlay |
| VID-106 | Time remaining and "ends at" | Know when the film finishes, or hide the clock | Plex partial (192-vote request); Jellyfin yes (option to hide requested, #3987) | Medium: 192 votes | R2 | Parity, and the clock accounts for playback speed | None | Player transport bar |
| VID-107 | Ordered chapters, editions and linked segments | MKV releases that rely on them play as authored | Jellyfin hidden chapters requested (#2963, 6); mpv (unverified) | Low: 6 votes | Later | Parser support for MKV editions and linked segments, native clients first | Edition and segment-link index | Pre-play sheet (edition choice) |
| VID-108 | Private scene bookmarks | Mark a moment and come back to it | Netflix "Moments" (with sharing); no server does it | Low: no vote count | Later | Bookmarks are events in the user's log | Bookmark events | Player menu; Title detail page |
| VID-109 | Clip and share a scene | Trim a moment and send it to someone | Netflix "Moments"; no server does it | Low: no vote count | No | Sharing clips cut from personal copies of commercial films raises rights questions the project should not take on | n/a | n/a |

### Skip intro, recap and credits

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| VID-110 | Skip markers as editable, logged data | Intro, recap, preview, credits and commercial segments that survive rebuilds and can be exported | Jellyfin media segments framework (10.10) with these types; Plex and Emby keep markers internal | High: Jellyfin "skip intro and credits" (#45) had 1,089 votes | R2 | Segments, and every correction to them, are events in an append-only log rather than rows only in the SQLite cache | Segment store and events; segment API for clients | Player skip button; Admin: marker editor |
| VID-111 | Markers from chapter titles | Files with chapters named "Opening" or "Credits" get markers instantly | Jellyfin's official plugin detects from chapters; others (unverified) | Medium: no vote count | R2 | The core's MKV parser already reads chapter titles at scan, so this costs nothing extra and needs no FFmpeg | Chapter-name rules in several languages | Player skip button |
| VID-112 | Intro detection across a season | "Skip intro" on TV episodes that have no chapters | Plex yes (Plex Pass), matching the theme across a season; Emby yes (Premiere), scanning the first 10 minutes and needing two episodes; Jellyfin via the community Intro Skipper plugin (2.8k stars) | High: 1,089 votes; Plex persistent auto-skip 483 votes; Emby skip thread 3.9k replies | R2 | A background job compares audio across a season; it needs decoded audio, so it runs in the sandbox (an ADR change) or on a client that is already decoding, and it is free either way | Analysis job; fingerprint store; season grouping | Player skip button; Admin: job status |
| VID-113 | Credits detection from the picture | "Skip credits" on films, including mid- and post-credits scenes | Plex yes (Plex Pass), finding mid- and post-credits scenes; Jellyfin via Intro Skipper outros; Emby not documented | Medium: no vote count beyond the skip requests | Later | Needs video analysis (black frames, on-screen text); parity with Plex at best | Sandbox video analysis | Player skip button |
| VID-114 | Skip policy per type, plus "ask to not skip" | Skip automatically, show a button, or ignore, set per segment type; auto-skip can show a short prompt to cancel | Plex button (auto-skip unverified; persistent auto-skip requested, 483); Jellyfin per client; Emby three-way setting; Infuse auto-skip (8.1.5); "ask to not skip" requested at Jellyfin (#2954, 30) and offered by nobody | High: 483 votes | R2 | One free policy per user and type, synced to every client and to downloads | Policy in user settings | Settings: playback (skipping); Player skip button and cancel prompt |
| VID-115 | Crowd-sourced markers | Markers without any local analysis | Infuse 8.4 uses IntroDB and TheIntroDB, matched by IMDb ID; IntroDB lists Jellyfin among its integrations | Medium: no vote count | Later | An opt-in plugin with an explicit network grant, because it tells a third party what you watch | Plugin; external ID lookup | Grant prompt; Admin: plugins |
| VID-116 | Fix a wrong marker in the app | Move a marker to the right place | Jellyfin via plugin (unverified); IntroDB lets anyone submit or challenge a timestamp | Low: no vote count | R2 | Edits are log events made by users with the right permission, so they survive rebuilds | Marker edit right; events | Player menu ("marker is wrong"); Admin: marker editor |
| VID-117 | Skip commercials in recordings | A skip button over ad breaks in DVR recordings, with the file untouched | Plex yes (the viewer needs Plex Pass); Jellyfin third-party Comskip plugin; Emby none built in (unverified) | Medium: see the live TV research | R3 | Reuses the segment model and the skip policy; detection belongs to the live TV map | Commercial segments from the DVR | Player skip button |

### Resume, completion and autoplay

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| VID-118 | Resume anywhere, offline included | Carry on where you stopped on any device, even after watching on a plane | Plex, Jellyfin and Emby yes; Streamyfin syncs offline progress | High: offline is Jellyfin's top request (1,820 votes) | R2 | Each session writes position events to the append-only log with a device ID and a hybrid logical clock, so offline events merge deterministically | Event ingest; merge rule; derived resume view | Resume prompt; Title detail page |
| VID-119 | Rewind a little on resume | Back up a few seconds for context | Plex yes (Plex Pass); others (unverified) | Low: no vote count | R2 | Free and set per user | User setting | Resume prompt; Settings: playback |
| VID-120 | Resume per version | Progress follows the copy you were watching | Jellyfin 12.0 tracks the version actually watched; others (unverified) | Medium: no vote count | R2 | Position is shared between versions whose running times match and kept apart when the cuts differ (rule to be confirmed) | Per-version positions | Pre-play sheet |
| VID-121 | Progress survives replacing or renaming the file | Upgrading to a better copy keeps your place | Plex yes (unverified); Jellyfin 10.11.0 lost watched state on replace or rename (#15001, 61 +1) | Medium: 61 +1 | R2 | The log keys progress to content identity as defined by LIB-028, not to the file path | Content identity from the library map | None |
| VID-122 | Finished means reaching the credits | A film counts as watched when its credits start, not at an arbitrary percentage | All three use server thresholds (unverified); Jellyfin request to count a credits skip as watched (#2298, 6) | Low: 6 votes | R2 | The credits marker, where known, sets the completion point; otherwise a threshold applies | Completion rule using segments | Post-play screen |
| VID-123 | Autoplay the next episode | Binge without the remote, with a countdown that starts at the credits | Plex yes ("Pass Out Protection" customisation is Plex Pass); Jellyfin yes; Emby yes | High: basic expectation | R2 | The countdown starts at the credits marker rather than a fixed time before the end, and every setting is free | Next-episode resolution | Post-play screen; Countdown card |
| VID-124 | Next item ready before it starts | No spinner between episodes, or between a pre-roll and the film | Plex "buffer the movie during the pre-roll" requested (589 votes); Jellyfin requested (#400, 222); nobody confirmed | High: 589 and 222 votes | R2 | During the credits the client fetches the next file's first segments, whose byte ranges the segment map already holds | First-segment byte ranges | None |
| VID-125 | "Are you still watching?" | Autoplay stops for a sleeping viewer | Plex customisation is Plex Pass; Jellyfin 12.0 and Android TV 0.19 (by time or episode count); a 2025 request had 111 votes | Medium: 111 votes | R2 | Parity, and free | User setting | Prompt card in the player |
| VID-126 | Sleep timer | Stop after N minutes, at the end of this episode, or after N episodes | Plex missing in the new iOS preview; Jellyfin Android TV "stop after this episode" requested (#3970, 3); an episode-count sleep timer request has 11 votes | Low: 11 and 3 votes | R2 | Shares the music player's sleep timer | None | Player menu |
| VID-127 | Autoplay that handles gaps and specials | Missing episodes are flagged and specials play where they aired | Jellyfin missing-episode handling requested (#341, 27) and specials in Next Up completed; others (unverified) | Low: 27 votes | R2 | The synced library knows the gaps, so autoplay asks before jumping one | Episode order with gaps (library map) | Countdown card ("episode 5 is missing") |
| VID-128 | Spoiler-free continuous play | The next episode's title and thumbnail stay hidden | Infuse 8.2 | Low: no vote count | R2 | Client setting | None | Countdown card; Settings: playback |
| VID-129 | Next film in a collection | "Play the next film in the series" when a movie ends | Jellyfin request (36 votes); nobody | Low: 36 votes | Later | Ordered collections from the library map drive the same post-play screen | Collection order | Post-play screen |
| VID-130 | Private viewing | See ACC-117, which owns this feature. ACC-117 applied to video; the UI says plainly that the owner still controls the machine. | Nobody among the servers (unverified for streaming apps) | Low: (unverified) | R2 | See ACC-117. | None beyond ACC-117. | Pre-play sheet toggle |
| VID-181 | Video queue: play next, add to queue, play all | Queue episodes and films the way music is queued, and play a whole season or collection in order | Plex and Jellyfin yes (research) | Medium: basic expectation | R2 | Reuses the MUS-122 queue object and LAT-009 listening contexts, so a video queue syncs and hands off like music | None beyond MUS-122 | Context menu; Player > Up next |
| VID-182 | Shuffle a show, season or collection | Random episodes of a sitcom, or a shuffled film collection | Plex and Jellyfin yes (research) | Medium: basic expectation | R2 | The same queue object with a seeded shuffle from the core, so the order reproduces on every device | None beyond MUS-122 | Show, season and collection pages |
| VID-183 | Manual video playlists | Hand-made lists of films and episodes | Plex and Jellyfin yes (research) | Medium: basic expectation | R2 | The same playlist object as music (MUS-132), stored in the user log; rule-based video playlists are DIS-124 | Playlist store shared with music | Playlists |

### Player controls and comfort

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| VID-131 | Playback speed with pitch correction | Watch at 0.75x, 1.25x, 2x and beyond with voices sounding normal | Plex yes (Plex Pass; 1,298-vote request); Jellyfin yes, finer steps requested (#3147, 23; #2662, 20); mpv best (0.01x to 100x with scaletempo2) | High: 1,298 votes | R2 | libmpv's scaletempo2 on every client, in fine steps, free, remembered per user if wanted | None | Player: speed control |
| VID-132 | Hold for 2x | Temporary fast-forward while pressing | Infuse 8.0.5, with haptics in 8.0.6; servers no (unverified) | Low: no vote count | R2 | Client gesture; parity with Infuse | None | Player (touch, held remote key) |
| VID-133 | Speed indicator | See at a glance that you are not at 1x | Jellyfin requested (#3435, 6) | Low: 6 votes | R2 | Client feature | None | Player transport bar |
| VID-134 | Picture-in-picture | Keep watching in a floating window | Plex missing or unreliable in the new apps; Jellyfin Android yes (#369), Findroid and Streamyfin; Apple's system PiP (unverified) | Medium: listed among Plex's regressions | R2 | Android first through the platform API; iOS waits for the Apple client and may need Apple's player (unverified) | None | Player; system PiP window |
| VID-135 | Background audio for video | The film's sound continues with the screen off, with lock-screen controls | Swiftfin pauses video in the background; Jellyfin 12.0 fixed iOS background playback; Plex (unverified) | Medium: no vote count | R2 | Reuses the native background audio and lock-screen module that ships with the R2 music clients (MUS-074), as a per-user choice | None | Lock screen and notification; Settings: playback |
| VID-136 | Interruptions handled | Calls and navigation prompts pause and resume cleanly; unplugging headphones pauses | Jellyfin Android 2.7.2 added interruption handling; others (unverified) | Medium: no vote count | R2 | Shared with the music player | None | None |
| VID-137 | Zoom and aspect modes | Fill a phone screen, crop black bars or fix a bad aspect ratio | Infuse 8.1.7 aspect modes; Plex pinch zoom broken in the new iOS preview | Low: no vote count | R2 | libmpv zoom and pan; parity with Infuse | None | Player (pinch, menu) |
| VID-138 | Brightness and volume gestures | Swipe on the left or right edge of a phone screen | Streamyfin gesture controls; others (unverified) | Low: no vote count | R2 | Client feature | None | Player (touch) |
| VID-139 | OS media controls and media keys | Media keys, the system now-playing panel, and MPRIS on Linux | Jellyfin Desktop 2.0 has MPRIS; others (unverified) | Medium: no vote count | R2 | Shared with the music player | None | OS media overlay |
| VID-140 | Player controls that work with screen readers | Blind and low-vision users can run the player | Jellyfin player controls are not accessible (#4504 open since 2023; #442 since 2019); Plex's new Apple TV app regressed hover text (Sept 2026); Infuse best with VoiceOver | High: long-open issues at both rivals | R2 | Controls are real React Native views over the libmpv surface with roles, labels and states, and an inaccessible control fails CI | Accessibility tests in CI | Every player surface |
| VID-141 | Reduced motion and large targets | Calm animations and controls that are easy to hit | No rival documents reduced-motion support; Plex's double-tap and PiP were called hard for people with limited mobility | Low: no vote count | R2 | The OS settings are honoured and tested | None | Every player surface |
| VID-142 | Player settings follow you | Subtitle style, speed, skip and audio settings are set once for every device | Only through plugins at Jellyfin (Streamyfin, Moonfin); Plex account-level (unverified) | Medium: two clients built plugins for it | R2 | Settings are ordinary synced data, split by user, device and audio output (see the open decisions) | Settings sync and events | Settings: playback, subtitles and audio |
| VID-143 | External display from a phone | Play on a cabled TV from the phone | Swiftfin wide-ratio external display; Jellyfin Android request #2060 | Low: no vote count | Later | Client feature | None | Player |

### Casting, handoff and remote control

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| VID-144 | Hand off to another Gunmetal device | Move a film from the phone to the TV and carry on with the same position, tracks and settings | Nobody confirmed for video; Plex casting to Plex players (unverified in the new apps); Jellyfin remote control of sessions | Medium: ADR 2 commits to handoff; Spotify Connect is the model users cite | R2 | Uses the same iroh control channel as the music queue handoff; the TV fetches the original itself, so nothing is re-encoded and the phone becomes the remote | Session transfer; device presence | Device picker in the player; accept prompt on the TV |
| VID-145 | Phone as the TV's remote | Browse on the phone and control playback on the TV | Plex yes (unverified); Jellyfin yes for any session (off by default for other users, with a broken-access-control advisory on this API in Sept 2026); Roku remote control requested (#3423, 5) | Medium: no single vote count | R2 | Control needs the same user or an explicit grant, checked per object and tested across users | Remote-control commands behind authorisation | Remote screen on the phone |
| VID-146 | Chromecast from Android and the web | See CLI-106, which owns this feature. | Plex yes, reported broken after its 2025 app rollout; Jellyfin yes on Android and web; Emby yes | Medium: at least nine Plex threads on broken casting | R2 | See CLI-106. | None beyond CLI-106. | Cast button; cast controls |
| VID-147 | Chromecast from iPhone | See CLI-107, which owns this feature. | Plex yes; Jellyfin's official iOS apps no (#466, 104 votes, open since 2020); Streamyfin yes | Medium: 104 votes | Later | See CLI-107. | None beyond CLI-107. | Cast button |
| VID-148 | AirPlay video | See CLI-109, which owns this feature. | Jellyfin web AirPlay since 10.5; Infuse (Pro) | Low: no vote count for video | Later | See CLI-109. | None beyond CLI-109. | AirPlay button |
| VID-149 | Subtitles while casting | See CLI-108, which owns this feature. | Plex external subtitles reported failing on Chromecast; Jellyfin offset requested (#1132, 11); Streamyfin still in progress | Medium: repeated Plex threads | R2 | See CLI-108. | None beyond CLI-108. | Cast controls |
| VID-150 | Casting that works when the server is far away | See CLI-110, which owns this feature. | Jellyfin LAN-direct URLs requested (#993, 5); Symfonium proxy mode | Low: 5 votes | R2 | See CLI-110. | None beyond CLI-110. | None |
| VID-151 | Cast controls in the notification | See CLI-112, which owns this feature. | Jellyfin requested (#258, 10) | Low: 10 votes | R2 | See CLI-112. | None beyond CLI-112. | Lock screen and notification |
| VID-152 | DLNA server in the core | Old TVs browse the library over DLNA | Jellyfin moved DLNA out of its core into a plugin | Low: no vote count | No | DLNA is an unauthenticated LAN protocol; if it is ever offered, it belongs in a plugin with an explicit grant | n/a | n/a |

### Watch together

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| VID-153 | Watch together on every client, TVs included | Friends in different homes watch in step | Plex dropped Watch Together from its new apps in 2025 and kept it on the web only; Jellyfin SyncPlay on web-wrapper clients only, Android TV started (#2282, 37); Emby no native feature (unverified) | High: Plex's restore request has 2,878 votes, its second most-voted suggestion | Later | Everyone direct plays the original from the host's server over iroh; a small control channel carries play, pause and seek; drift is corrected by nudging speed a few percent instead of jumping Owns group sessions (watch and listen together); CLI-114, MUS-202 and ACC-094 point here. Guest access uses ACC-135 (R2). | Group session state; clock sync over iroh | Watch-together lobby; group bar in the player |
| VID-154 | Guest invite links | See ACC-093, which owns this feature. | Plex needs a Plex account for each guest; Jellyfin requested (#971, 267 votes; guests without library access #863, 27); nobody | High: 267 votes | Later | See ACC-093. | None beyond ACC-093. | Invite sheet; guest landing page |
| VID-155 | Ready check | Start only when everyone is ready | Plex requested (forum); Jellyfin requested (#3251, 43) | Medium: 43 votes | Later | Part of the control channel | Ready state | Watch-together lobby |
| VID-156 | Chat and reactions | Talk without a separate call | Jellyfin chat requested (#740, 104); Plex reactions requested; Plex users point to Hulu's watch party | Medium: 104 votes | Later | Messages ride the same control channel; whether they are kept after the session is a design choice to confirm | Ephemeral messages | Group bar in the player |
| VID-157 | Host controls | Only the host can pause or seek, if the host wants that | Jellyfin requested (#747, 13) | Low: 13 votes | Later | Permission flags on the group | Group roles | Watch-together lobby settings |
| VID-158 | Rooms and binge sessions | Name the room, then pick the film; carry on to the next episode together | Jellyfin rooms before playback (#869, 30) and naming (#1093, 23) started; Plex request for several episodes in a session (86 posts) | Low: 30 and 23 votes | Later | The group session holds a queue | Group queue | Watch-together lobby |
| VID-159 | Bandwidth-aware sessions | The host is warned when the home uplink cannot carry everyone's copy | Not covered for any rival | Low: follows from the bandwidth risk in the research | Later | The session compares the number of guests times the file's peak bitrate with the measured uplink and offers a smaller version or audio-only conversion | Uplink estimate | Watch-together lobby warning |
| VID-160 | Same picture on several screens at home | Play in sync on several TVs in one house | Plex requested since 2013 (420 votes); Jellyfin SyncPlay can do it on supported clients | Medium: 420 votes | Later | The same engine as VID-153, on the local network | As VID-153 | Device picker ("add a screen") |

### Trailers, pre-rolls and extras

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| VID-161 | Local trailers on the title page | Watch the trailer before deciding | Plex yes; Jellyfin yes, local or remote (better local handling requested, #3033, 20); Infuse Pro | Medium: 20 votes for local handling | R2 | Parity; local trailer files found by the scanner play through the same player | Trailer files (library map) | Title detail page |
| VID-162 | Extras from the title page | Deleted scenes and featurettes play from the film | Plex, Jellyfin and Emby yes; Infuse 8.4 surfaces extras from all three | Medium: no vote count | R2 | Parity | Extras (library map) | Title detail page |
| VID-163 | Online trailers | Trailers for titles with no local trailer file | Plex trailers before films (Plex Pass); Jellyfin trailers plugin requested (#55, 585 votes) | Medium: 585 votes | Later | A plugin with a network grant, because it fetches from a third party | Plugin | Title detail page |
| VID-164 | Custom pre-roll clips | A studio ident or a "phones off" clip before a film | Plex yes (the clip must sit in a library the viewer can see; reported broken on the new iOS app); Jellyfin local intros plugin (#2710, 29); Emby Premiere | Low: 29 votes | Later | Local files chained before the feature, with the feature buffering during the pre-roll (VID-124) | Pre-roll library and rules | Admin: pre-roll settings |
| VID-165 | Cinema mode | Trailers of other films in your library before the main feature | Plex (Plex Pass); Jellyfin plugin; Emby "Cinema Intros" (Premiere) | Medium: part of the 585-vote trailers request | Later | Local trailers only by default, so nothing leaves the house | Trailer selection rules | Pre-play sheet toggle |
| VID-166 | Theme music and video | Ambience on the title page | Plex removed it in the new apps; Jellyfin yes (random order in 10.11) | Low: no vote count | Later | Parity | Theme files (library map) | Title detail page |
| VID-167 | Who is on screen | Cast and music for the current scene | Apple TV "InSight" (Apple TV+ titles only); Jellyfin requested (#2545, 39) | Low: 39 votes | No | Needs per-scene data that no source provides for personal media, or face recognition the low-spec goal cannot afford | n/a | n/a |

### Statistics, diagnostics and live sessions

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| VID-168 | Playback information overlay on every client | Codec, bitrate, resolution, HDR format, decoder, hardware decoding, dropped frames, buffer and display rate | Plex old apps yes, but the new iOS preview had none and the new Apple TV app hides media info; Jellyfin web compact in 12.0, Android TV planned (#272, 219); mpv's stats page and Infuse 8.3.6's HUD best | High: 219 votes | R2 | Shows libmpv's own figures plus the core's decision reasons, and ships on every client in the first video release rather than later | Decision trace per session; field redaction by role | Player: info overlay |
| VID-169 | A plain reason for every remux or transcode | "Remuxing because this browser cannot open MKV; picture and sound untouched" | Plex dashboard (detail unverified); Jellyfin playback info (detail unverified) | High: a named pain point in the playback research | R2 | The decision engine returns a structured reason list as a protocol type, so the player, error cards and the admin dashboard print the same sentence | Structured reasons in the protocol | Player: info overlay; Admin: sessions; Player error card |
| VID-170 | Full file information | Every stream and flag in the file | Jellyfin done (#1138); Infuse | Medium: no vote count | R2 | Comes from the scan-time index, so it works offline | Stream index | Title detail page (media info) |
| VID-171 | Live sessions with the delivery breakdown | See ADM-099, which owns this feature. Video specifics: the delivery breakdown, with the direct-play share front and centre. | Plex, Jellyfin and Emby yes; Plex bandwidth graphs are Plex Pass | Medium: tie on the basics | R2 | See ADM-099. | None beyond ADM-099. | Admin: sessions |
| VID-172 | Stop a stream with a message | See ADM-102, which owns this feature. | Plex yes (unverified whether paid); Jellyfin requested (#301, 402 votes); Tautulli can | High: 402 votes | R2 | See ADM-102. | None beyond ADM-102. | Admin: sessions; Player message card |
| VID-173 | Concurrent stream limits | A shared login cannot be used by half the street | Jellyfin per-user limit requested (#1444, 85); Emby request (161 replies) | Medium: 85 votes and 161 replies | R2 | Counts playback, not browsing, and is enforced when stream URLs are minted | Concurrency counter per user | Admin: user policies; Player limit card |
| VID-174 | Playback diagnostic bundle | One tap sends the owner exactly what went wrong | Jellyfin Android TV 0.19 added a media capability report | Low: no vote count | R2 | Bundles the device's capability report and the decision trace; redaction is enforced by types and the user previews the bundle first | Redacted report endpoint | Player error card; Admin: health |

### Downloaded playback and player engines

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| VID-175 | Downloaded titles keep every player feature | Offline films keep skip markers, chapters, subtitles, fonts and previews | Streamyfin keeps skip segments in downloads; Findroid stores images locally | Medium: offline is Jellyfin's top request (1,820 votes) | R2 | The synced library already carries segments, chapters and the track index, and previews are decoded from the local file, so nothing extra is stored | Download packaging (offline map) | Player (offline) |
| VID-176 | libmpv player on Android, Android TV and the desktop shell | Nearly any file plays as it is | Infuse, Kodi and mpv best; Jellyfin Desktop 2.0 and Findroid also use mpv | High: the central bet of the project | R2 | Parity with mpv-based players on format breadth; the edge is that the server's decision engine trusts what this player reports | Capability report | Player |
| VID-177 | Browser player | Films play in current browsers without a transcode wherever possible | Jellyfin web (libass-wasm, client-side PGS) is the closest; browser codec support keeps moving (HEVC in Firefox 134 and later) | High: the web client is everyone's fallback | R2 | Remux to fMP4, subtitles rendered in WebAssembly, previews through WebCodecs; HDR and DV in browsers stay weak | Remux path; WebAssembly build of the core | Player (web) |
| VID-178 | Apple TV and iPhone player | Originals on Apple devices, with DV where the hardware allows | Infuse is the reference; Plex's new Apple TV app drew a 268-post complaint thread; Swiftfin on tvOS since July 2026 | High: Jellyfin's Apple TV request had 463 votes | Later | Includes true Dolby Vision output through Apple's player for DV displays. Later; R2 if the App Store licence decision allows (open decision 3 in the feature map README). libmpv for most files and Apple's own player fed by the remuxer for DV and AirPlay (to be decided) | Remuxed HLS with correct DV signalling | Player (tvOS, iOS) |
| VID-179 | Samsung and LG TV player | Smart TVs play through their own video element | Plex and Emby have long store presence; Jellyfin on LG and on Samsung Tizen 6 and newer since May 2026 | High: Jellyfin Samsung request 253 votes; Vidaa 453 | Later | Later; follows the README roadmap, which packages Samsung and LG after the video milestone. CLI-010 points here for playback. Remux path plus WebAssembly subtitles; performance on weak TV chips must be measured | Remux path | Player (TV web runtime) |
| VID-180 | Roku and other adapter clients | Existing Jellyfin apps, Roku included, play from Gunmetal | Roku apps exist for all three rivals | Medium: Roku is the main platform with no Gunmetal client | Later | Reached through the optional Jellyfin adapter; these clients need remuxes, transcodes and burn-in more often | Jellyfin adapter (ecosystem map) | None of ours |

## Differentiators

These are the features in this area most likely to make someone switch.
Most of them follow from owning both ends of the connection; none of them
needs a faster transcoder.

1. **Subtitles never force a transcode and stay in sync** (VID-069, VID-070,
   VID-071, VID-091). Styled and image subtitles are where Plex is weakest:
   staff confirmed its Android TV player cannot render ASS, so users choose
   between a burn-in and losing the styling. Subtitle drift on seek and
   resume is the most-reacted open issue on Jellyfin's server. People who
   watch anime or rip their own discs are exactly the people who run media
   servers, and this is the complaint that sends some of them back from
   Jellyfin to Plex.
2. **Every playback decision is explained** (VID-168, VID-169, VID-015,
   VID-025). The decision engine lives in the shared core and returns its
   reasons, so the player can say why it is remuxing in one sentence and
   show the badge before anyone presses play. Plex's rebuilt apps shipped
   without media info, Jellyfin's Android TV overlay request has 219 votes,
   and the same reasons double as the public proof of the project's central
   claim.
3. **HDR and Dolby Vision handled at the edges** (VID-039, VID-040, VID-041,
   VID-043, VID-047). The edge is the remuxer: it falls back to HDR10 or
   converts profile 7 to 8.1 without decoding, so a DV file plays correctly
   on devices that would otherwise show the wrong colours, and subtitles
   stop glaring on HDR. The rendering itself is not an edge: libmpv reshapes
   profiles 5 and 8 and tone maps on the client, as Infuse, Kodi and mpv
   already do, and it does not send a true Dolby Vision signal to a DV
   display, which Infuse and Plex on Apple TV do (VID-184 and VID-178 cover
   that, both Later). Plex and Emby charge for server tone mapping. This
   needs real DV hardware to verify before it is promised.
4. **Original quality by default, honest about remote limits, and free**
   (VID-023, VID-024, VID-028, VID-035). Plex's most-discussed request asks
   for exactly this default, and Plex now charges for remote playback. The
   honest caveat: on a poor link, rivals with adaptive transcoding still
   give a smoother picture; Gunmetal's answer is a clear choice, not magic.
5. **Free, private skip markers with "ask to not skip"** (VID-110, VID-111,
   VID-112, VID-114, VID-116). Plex and Emby charge for skipping, and
   Jellyfin depends on a community plugin. Markers from chapter titles cost
   nothing, corrections survive rebuilds, and nothing about what you watch
   leaves the house unless you grant a plugin that right.
6. **Fixes that stick** (VID-061, VID-062, VID-083, VID-084, VID-085,
   VID-059). Audio delay saved per output device, subtitle offsets saved
   per series, re-timing against an embedded track and a client-side
   dialogue lift answer requests that have been open since 2014 at Plex,
   and Plex's new dialogue boost is paid and transcodes the audio.
7. **Watch together with guest links on every client, TVs included**
   (VID-153, VID-154). Plex's request to restore Watch Together has 2,878
   votes, and Jellyfin's SyncPlay only runs on web-wrapper clients and has
   no invite link. This is marked Later, but the guest capability it needs
   is now an R2 row (ACC-135), so it is not bolted on.

## Deliberately not doing

- **Full Blu-ray and DVD menus** (VID-021). Four votes, impossible in
  browsers, and Blu-ray menus need a Java runtime on every client
  (unverified).
- **Composing the Dolby Vision profile 7 enhancement layer** (VID-042). It
  needs two simultaneous HEVC decodes, and Jellyfin's own attempt warns it
  can exhaust GPU memory. Converting to 8.1 or falling back to HDR10 covers
  almost everyone.
- **Clipping and sharing scenes** (VID-109). Sharing cuts from personal
  copies of commercial films is a rights question the project should not
  take on. Private bookmarks stay on the Later list.
- **"Who is on screen"** (VID-167). No data source exists for personal
  media, and face recognition does not fit the low-spec goal.
- **DLNA in the core** (VID-152). It is an unauthenticated LAN protocol. If
  it is ever offered, it is a plugin with an explicit grant.
- **Burning in subtitles for first-party clients.** Our own players render
  every format; burn-in exists only for adapter clients (VID-077).
- **A low remote default.** The default is the original; Gunmetal never
  silently drops to 720p.
- **Paid tiers for any playback feature.** Speed, skipping, tone mapping,
  hardware transcoding, remote playback and downloads stay free.
- **Sending viewing data out by default.** Subtitle search, crowd-sourced
  markers and online trailers only run as plugins with an explicit network
  grant the user can read and refuse.
- **Running FFmpeg outside the sandbox.** If the sandbox cannot start on a
  kernel, transcoding is reported as unavailable rather than run without it
  (VID-009).
- **Server-side transcoding or tone mapping as the normal path for clients
  that can do the work themselves.** It exists only as a fallback.

## Dependencies and risks

- **Blocking: ADR 3, durable user state.** ADR 1 (decision 5) names watch
  history as the only irreplaceable data, but most rows in this map that
  write user data (playlists, loves, ratings, corrections, layouts, shares,
  identities, IDs, settings) need it to survive a cache rebuild. ADR 3 must
  be accepted before any server work that stores user data. It defines the
  two durable stores every map now names the same way: **the user log** and
  **the identity store** (see the [feature map README](README.md)).

**Dependencies on other parts of the plan.**

- *R1 music.* The user log, signed stream URLs, settings sync, the sleep
  timer, the persistent queue and the decision reason type arrive with music
  in R1 and are reused here. The native background audio and lock-screen
  module (MUS-074) and the handoff channel (CLI-101) arrive with the R2
  native clients.
  If any of them is designed only for audio, video will pay to rework it.
- *Library map.* Versions, editions, extras, trailers, episode order with
  gaps, the per-file stream index, chapters, font attachments and the stable
  content identity that keeps progress across file replacements.
- *Clients and offline map.* Download packaging and rules, the sync payload
  that carries stream indexes and segments, and the platform order for
  Apple, Samsung, LG and Roku.
- *Users and sharing map.* Playback-mode rights, bandwidth policy, session
  revocation, concurrency limits, parental limits enforced during playback,
  and guest capabilities for watch-together.
- *Operations map.* The doctor check and health page (VID-009) and the
  sessions dashboard (VID-171).
- *Ecosystem map.* The plugin framework and network grants (VID-087,
  VID-115, VID-163), Bazarr integration (VID-076, VID-090) and the Jellyfin
  adapter (VID-077, VID-180).
- *Live TV map.* Commercial detection feeds VID-117.

**Risks.**

- **Remuxer scope.** ADR 1 already names the remuxer as the main schedule
  risk. This map adds HEVC NAL handling, Dolby Vision metadata stripping and
  RPU rewriting, correct DV sample entries in fMP4 and HLS, lossless audio
  in fMP4, subtitle side streams, font attachments and single-keyframe
  extraction. Each needs real sample files, and test media under a licence
  that allows committing it is hard to find.
- **Analysis needs decoding.** Intro detection, the preview fallback and
  speech-based subtitle sync all need decoded audio or video. ADR 1 keeps
  FFmpeg off the scan path, and pure-Rust decoders for AC3, E-AC3, DTS and
  TrueHD do not exist (unverified). Without an ADR amendment, VID-112 can
  only run on clients, which leaves the first episode of each season
  without markers.
- **Apple platforms.** mpv has no official tvOS build (unverified), DV on
  Apple TV probably needs Apple's own player fed by the remuxer, and AirPlay
  likely does too (unverified). That is two players to test on one
  platform. Separately, Gunmetal is AGPL with no contributor licence
  agreement, and GPL code in Apple's App Store has a troubled history
  (unverified legal analysis). An app-store permission, if wanted, has to
  be settled before outside contributions arrive.
- **Browsers and TV web runtimes.** Codec support differs by browser, DV in
  browsers is close to nonexistent, and HDR output is inconsistent. Browsers
  reach iroh only through relays, so remote video in a browser costs relay
  bandwidth on every byte.
- **Remote access timing.** Every remote row in R2 depends on iroh remote
  access, which the README roadmap lists after the video milestone.
- **GPU sandboxing.** Hardware transcoding and GPU tone mapping need access
  to GPU device nodes, which widens the sandbox the README promises. The
  isolation story must be written for the GPU case before VID-006 ships.
- **Watch-together bandwidth.** Direct play to several remote guests
  multiplies the host's upload; a 4K session can exceed common home
  uplinks, and relay traffic may cost someone money when hole-punching
  fails.
- **Dolby trademarks.** Handling DV metadata in open source is common, but
  marketing "Dolby Vision support" may need a trademark check (unverified).
- **Third-party licences.** alass is GPL-3.0 and its compatibility with
  AGPL-3.0-or-later should be confirmed. The dolby_vision crate must meet
  the core's no-unsafe and no-panic rules or live outside the core.
- **Rebuildable cache.** Segment edits, saved offsets, track preferences,
  bookmarks and player settings are user data. If any of them lives only in
  SQLite, the cache stops being rebuildable and ADR 1's migration story
  fails.
- **Player polish with one React Native codebase.** Plex's rebuild shows how
  badly a player rewrite can land even for a funded team. Every platform
  needs its own acceptance list for the player, and accessibility must work
  on controls drawn over a libmpv surface that has no accessibility tree.
- **Scope against the test gate.** 127 of the 180 rows here are marked R2. Under a test-first, zero-surviving-mutant gate, R2 needs a cut
  line (see the open decisions).

## Open decisions for the project owner

1. **Does R2 include full video transcoding?** The research leaves open
   whether the first video release ships only remux and audio-only
   conversion. *Recommendation:* include software video transcoding in the
   sandbox in R2, off by default for everyone but the owner, because
   browsers on SDR screens and some weak devices have no other path. Do not
   promote remote video until pre-made versions (VID-026, now R2 as an
   opt-in job) exist.
2. **May analysis jobs use the sandboxed FFmpeg worker?** This needs an ADR
   amendment. *Recommendation:* yes, for intro fingerprinting, the preview
   fallback and speech sync, at low priority, resumable, with no network
   and a single off switch. Client-computed fingerprints can complement it
   later.
3. **When does hardware transcoding ship?** *Recommendation:* Later, after a
   written isolation design for GPU access. Do not block R2 on it; the
   project's bet is that transcodes are rare.
4. **What is the Apple TV player strategy, and is an app-store permission
   added to the licence?** *Recommendation:* plan for both players, with a
   rule that DV content and AirPlay go through Apple's player fed by the
   remuxer and everything else through libmpv. Take legal advice on an
   app-store additional permission now, before outside contributions make
   it impossible to add.
5. **What is the default for Dolby Vision profile 7?** *Recommendation:*
   convert to 8.1 for clients that report single-layer DV, fall back to the
   HDR10 base layer otherwise, and never compose the enhancement layer.
   Check Dolby's trademark rules before using the name in marketing.
6. **When does watch-together ship, and what can a guest see?**
   *Recommendation:* make it the first feature after R2; the guest
   capability itself ships in R2 (ACC-135). A guest should see only the one title's name and
   artwork, never file names, paths or other library items.
7. **Which playback plugins ship first-party, and who reviews network
   grants?** *Recommendation:* OpenSubtitles search in R2; IntroDB or
   TheIntroDB and online trailers later; the owner reviews every grant's
   wording before a plugin is listed.
8. **Use alass or reimplement the alignment?** *Recommendation:* confirm the
   GPL-3.0 licence is compatible with AGPL-3.0-or-later and use it in its
   own crate outside the core; reimplement only if the check fails.
9. **Where do player settings live?** *Recommendation:* subtitle style,
   languages, skip policy and speed per user; quality and DV preference per
   device; audio delay, passthrough and core-only audio per audio output.
10. **What happens by default when the link cannot carry the original?**
    *Recommendation:* ask once per device and remember the answer, with an
    owner rule that can override it per user.
11. **How much of the playback overlay do non-admin users see?**
    *Recommendation:* everyone sees codecs, figures and decision reasons;
    file paths, server addresses and other users' sessions are admin-only.
12. **Are trailers, pre-rolls and cinema mode in R2?** *Recommendation:*
    local trailers and extras in R2; pre-rolls and cinema mode Later.
13. **Is machine-generated subtitling in scope at all?** *Recommendation:*
    Later, as an opt-in plugin that runs on a capable client or in the
    sandbox, never by default on a low-power server.
14. **Where is the R2 cut line?** *Recommendation:* the must-ship set is the
    decision engine with reasons, direct play, remux, sandboxed transcoding,
    signed URLs, all subtitle rendering and sync rows, track selection and
    labels, HDR and DV on native clients, resume and autoplay, markers from
    chapters with the skip policy, the info overlay, and accessible
    controls. The rest of R2 can follow in point releases.
15. **Does the native player module switch engines for true Dolby Vision
    output (VID-184)?** libmpv tone maps on the client and never sends a DV
    signal. *Recommendation:* spike the Android TV platform decoder for DV
    titles early in R2; if one module can host both engines cleanly, move
    VID-184 to R2, otherwise keep it Later and say plainly that DV-TV owners
    get a tone-mapped picture.
