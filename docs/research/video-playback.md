# Video playback

Researched on 2026-10-02 with live web access. Facts drawn from a fetched page
are linked in the Sources section. Anything marked "(unverified)" comes from
general knowledge or a secondary summary that I could not confirm on a
primary page. Several primary sites (support.plex.tv, kodi.wiki, kodi.tv)
refused automated fetches, so some Plex and most Kodi details rest on search
summaries or memory and are marked accordingly. The session's web-search
budget ran out partway through; later facts were gathered by fetching known
URLs directly.

## Scope

This file covers everything that happens between pressing play on a movie or
episode and the credits ending:

- how the file reaches the screen: direct play, remux and transcode;
- quality and bandwidth selection, local and remote;
- HDR10, HDR10+, HLG, Dolby Vision and tone mapping;
- audio track selection, passthrough, downmixing, dialogue and sync tools;
- subtitles of every kind: plain text, styled (ASS/SSA), image-based (PGS,
  VobSub, DVB), forced and SDH tracks, search and download, timing
  correction and appearance;
- chapters, seeking and scrub previews;
- intro, recap and credits skipping;
- resume, continue watching, next up and autoplay;
- playback speed, picture-in-picture, background play and casting;
- watch-together;
- trailers, pre-rolls and extras;
- player statistics overlays and playback diagnostics.

Music playback, live TV and DVR are covered elsewhere. Library browsing,
metadata and downloads are covered elsewhere except where they change what
happens at playback time.

The rivals studied were Plex, Jellyfin and Emby (servers with their own
apps), Infuse (an Apple-only player that connects to all three), Kodi and
mpv (local players known for format coverage), and Netflix and the Apple TV
app as references for usability rather than as rivals.

Context that matters for every row below: Gunmetal's architecture records
say the native clients play the original file through libmpv, the server
remuxes in-process with pure-Rust code when only the container is the
problem, and FFmpeg runs only for real transcodes in a sandbox. Each file's
keyframe index (the segment map) is stored at scan time, watch history is an
append-only log, remote access goes over iroh, and anything that talks to a
third-party service belongs in a plugin with an explicit network grant.

## Feature inventory

Legend: "Yes" means the feature exists in the vendor's current apps as far as
I could confirm. "Plex Pass" and "Premiere" mean the feature is behind that
subscription. Vote counts are from the vendors' own request boards on
2026-10-02.

### Delivery: direct play, remux and transcode

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Direct play | The original file is sent untouched: best quality, almost no server CPU | Yes, when container, codecs, bitrate and resolution all suit the client | Yes | Yes | Infuse and Kodi, because their own decoders accept nearly every file, so direct play is the normal case rather than the lucky one | This is Gunmetal's central bet |
| Remux ("direct stream") | Container is rewritten; picture and sound untouched | Yes, called Direct Stream; also used for subtitle format changes | Yes, called Remux; 12.0 lets clients render image subtitles while remuxing | Yes (unverified) | Jellyfin 12.0, because it no longer burns image subtitles just because the container changed | Gunmetal plans a pure-Rust remuxer for this |
| Audio-only transcode | An unsupported audio track is converted while the video is copied | Yes (part of Direct Stream) | Yes ([#516](https://features.jellyfin.org/posts/516/transcode-audio-mux-with-original-video), completed) | Yes (unverified) | Tie | Cheap enough for any hardware |
| Full video transcode | Plays on anything, at a cost in quality and CPU/GPU | Yes | Yes | Yes | Plex for reliability and tuning; Jellyfin for breadth of free hardware back ends | Rivals all use FFmpeg underneath |
| Hardware-accelerated transcoding | The GPU does the encoding, so more streams fit | Plex Pass only | Free: Intel, AMD, Nvidia, Apple, Rockchip | Premiere only (a few devices exempt) | Jellyfin, because it is free and covers the most hardware | |
| HEVC transcode output | Better quality per bit; HDR can be kept | Plex Pass, hardware only, keeps HDR metadata so no tone mapping is needed | Partial; "keep HDR when transcoding" is [#1694](https://features.jellyfin.org/posts/1694/option-to-maintain-hdr-when-transcoding) (55 votes, planned); "always use preferred codec" is [#3269](https://features.jellyfin.org/posts/3269/option-to-always-transcode-with-preferred-codec-e-g-av1-or-hevc) (30) | (unverified) | Plex, the only one that documents HDR-preserving HEVC output | |
| Transcode throttling | Server stops encoding far ahead of the viewer | (unverified) | Partial; "only transcode 60 s ahead" is [#474](https://features.jellyfin.org/posts/474/setting-to-only-allow-60-seconds-or-entered-value-of-transcoding) (106 votes, started) | (unverified) | Unclear | Matters on shared, low-power hosts |
| Multiple versions of a title | Choose 4K or 1080p, theatrical or extended | Yes | Yes; 12.0 extends versions to episodes, with resume tracked per version | Yes (unverified) | Jellyfin 12.0, for per-version resume | |
| Pre-made optimised versions | Files converted ahead of time so remote or mobile play needs no live transcode | Yes, "Optimize" (unverified) | No; "pre-transcoding" is [#570](https://features.jellyfin.org/posts/570) (979 votes, open) | Yes, "Convert" (unverified) | Plex (unverified) | A natural fit for a sandboxed, scheduled transcoder |
| Downloading a smaller copy | Phone-sized file for offline viewing | Plex Pass downloads | Requested as [#57](https://features.jellyfin.org/posts/57) (518 votes, planned) | Premiere (Downloads & Sync) | Plex and Emby | |
| Disc folders and images (ISO, BDMV, VIDEO_TS) | Ripped discs play without conversion | Limited (unverified) | Limited; [#179](https://features.jellyfin.org/posts/179/improved-enhanced-iso-playback) (70 votes); full menus [#4124](https://features.jellyfin.org/posts/4124/feature-idea-full-blu-ray-menu-playback-using-libbluray) (4) | (unverified) | Infuse, which lists ISO, BDMV and VIDEO_TS among supported formats; Kodi (unverified) | |
| Hand-off to an external player | Open the file in VLC, mpv or Infuse | (unverified) | Desktop request [#227](https://features.jellyfin.org/posts/227/external-player-support-desktop) (65 votes) | (unverified) | Infuse, which is itself the external player people choose for all three servers | |
| Device capability profiles | Server knows what each device can decode | Yes (unverified detail) | Client sends a profile; server picks the best option within it | Yes (unverified) | Jellyfin, for its documented, open profile model | Gunmetal's decision engine lives in the shared core |
| Newer codecs (AV1, VVC/H.266) | Modern files play natively | AV1 on some clients (unverified) | AV1 direct play on TV clients and AV1 fMP4 stream copy in 12.0 | (unverified) | Infuse 8.4 added initial VVC decoding | |
| Anamorphic and rotated video | Correct shape and orientation | (unverified) | 12.0 fixed anamorphic direct play on Tizen and added a rotated-video option for Android TV | (unverified) | Jellyfin 12.0 | Small but real bugs elsewhere |
| Firefox HEVC | HEVC plays in Firefox without a transcode | (unverified) | Yes, Firefox 134+ (10.11) | (unverified) | Jellyfin | Browser codec support keeps moving |
| Auto-remux instead of transcode | Remux is preferred whenever possible | Yes | Yes; a request to make it automatic was declined as already covered ([#3654](https://features.jellyfin.org/posts/3654/allow-jellyfin-to-automatically-remux-not-transcode-files-on-the-fly)) | Yes (unverified) | Tie | |
| Hide or flag titles that cannot direct play | User knows before pressing play | No (unverified) | Requested, [#2832](https://features.jellyfin.org/posts/2832/hide-media-which-cannot-be-played-directly-or-requires-video-transcoding) (5 votes) | (unverified) | Nobody does this well | Gunmetal could compute this from the segment map and device profile |

### Quality and bandwidth

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Manual quality picker | Choose a bitrate or resolution | Yes | Yes | Yes | Tie | |
| Default remote quality | Sensible quality without fiddling | Historically 2 Mbps / 720p on new installs, which forces transcodes ([forum](https://forums.plex.tv/t/bug-plex-app-defaults-to-2mbps-720p/449537)) | Client chooses (unverified) | (unverified) | No clear winner | |
| Automatic quality | Quality follows the network | "Automatically adjust quality" switches on transcoding (forum reports) | No true adaptive bitrate; [#3436](https://features.jellyfin.org/posts/3436/implement-full-adaptive-bitrate-streaming-hls-dash-with-manual-quality-override) (55 votes) and bitrate ladder [#3680](https://features.jellyfin.org/posts/3680/bitrate-ladder-support) (56) | (unverified) | Netflix, with per-title encoding ladders (unverified) | Real ABR needs several renditions, which means transcoding |
| Admin bandwidth caps | Owner limits total upload and per-stream bitrate | Plex Pass ("max upload bandwidth and per-stream caps") | Per-user remote bitrate limit (unverified) | Yes (unverified) | Plex | |
| Admin default or minimum quality | Owner sets a better default for remote users | Requested since 2017; thread has 138 posts and 27,989 views ([forum](https://forums.plex.tv/t/force-higher-remote-quality/195464)) | Server-side resolution cap [#3763](https://features.jellyfin.org/posts/3763/server-side-maximum-resolution-cap-for-transcoded-video) (12); default resolution [#2911](https://features.jellyfin.org/posts/2911/set-a-default-video-resolution-overall-or-per-user) (9) | (unverified) | Nobody | |
| Separate limits for direct stream and transcode | Remux at full bitrate, cap only transcodes | (unverified) | Requested, [#2596](https://features.jellyfin.org/posts/2596/separate-bitrate-limit-for-direct-stream-and-transcoding) (11) | (unverified) | Nobody | |
| Remote playback at all | Watching away from home | Paid since April 2025: Plex Pass, or Remote Watch Pass at $2.99/month or $29.99/year from 1 June 2026 | Free | Free (unverified) | Jellyfin | Plex's biggest self-inflicted wound |
| Better internet streaming | Faster starts and fewer stalls on long links | (unverified) | "Multiple parallel connections" [#2176](https://features.jellyfin.org/posts/2176) (231 votes, planned) | (unverified) | Unclear | iroh's QUIC transport is relevant here |
| Cellular quality setting | Lower quality on mobile data only | Yes (unverified) | Yes (unverified) | Yes (unverified) | Tie (unverified) | |
| Server-side pre-caching | Next episode or partly watched title is read ahead from slow disks | No (unverified) | Requested, [#2534](https://features.jellyfin.org/posts/2534/server-side-pre-caching-of-shows-partially-played-movies) (25) | (unverified) | Nobody | Helps spun-down arrays |
| Show what "Original" means | Bitrate and codec of the source shown next to the choice | Hidden in new Apple TV app ([forum](https://forums.plex.tv/t/rant-new-atv-app-no-thanks-i-want-the-old-one-back/943162)) | Shown in item details (unverified) | (unverified) | Infuse, which shows extra video details (8.1.7) | |

### HDR, Dolby Vision and tone mapping

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| HDR10 direct play | HDR on HDR screens | Yes | Yes | Yes | Infuse on Apple TV, and Kodi on capable boxes | |
| HDR10+ | Dynamic metadata on supporting TVs | (unverified) | 10.11 detects HDR10+ properly and strips the conflicting dynamic metadata to avoid black screens | (unverified) | Infuse lists HDR10+ on compatible hardware | |
| HLG | Broadcast HDR | (unverified) | 12.0 uses BT.2446 Method B when tone mapping HLG | (unverified) | Jellyfin 12.0 | |
| Dolby Vision profile 5 | Streaming-style DV with no HDR10 fallback layer | New apps reported failing with a "colour space not supported" error or black screen ([feedback thread](https://forums.plex.tv/t/new-experience-public-release-feedback/910904)); no P5 transcode ([forum](https://forums.plex.tv/t/transcode-dolby-vision/935235)) | Software tone mapping of DV (10.10), RK3588 hardware P5 tone mapping (10.11), spec-compliant `dvh1` HLS variant (12.0) | (unverified) | Infuse, with full P5 on Apple TV; mpv's gpu-next renderer applies the DV reshaping on any GPU | P5 without proper handling shows green and purple pictures |
| Dolby Vision profile 7 (UHD Blu-ray dual layer) | Disc-quality DV from remuxes | (unverified) | P7 on non-DV HDR displays done ([#2557](https://features.jellyfin.org/posts/2557/support-for-playing-dv-profile-7-hybrid-dv-hdr-files-on-non-dv-hdr-device)); enhancement-layer processing is an open, opt-in [PR](https://github.com/jellyfin/jellyfin-ffmpeg/pull/774); remux P7 to 8.1 requested ([#3838](https://features.jellyfin.org/posts/3838/server-user-specific-setting-to-remux-dovi-profile-7-to-8-1-with-dovi-tools)) | (unverified) | Nobody fully. Apple TV cannot take dual layer, so Infuse falls back to HDR10; mpv maps the base layer with DV metadata but does not compose the enhancement layer | Users convert files offline with dovi_tool |
| Dolby Vision profile 8 | Single-layer DV with HDR10 fallback | Yes (unverified) | Yes | (unverified) | Infuse 8.1, which widened support to more P8 variants | |
| Server HDR-to-SDR tone mapping | HDR looks right on SDR screens | Plex Pass | Free; hardware 3D LUT on Intel and RK3588 (10.11); OpenCL, CUDA and Vulkan paths | Premiere | Jellyfin, for free hardware tone mapping on most GPUs | Every rival does this on the server, at a cost |
| Client-side tone mapping | The player converts HDR itself, so the server stays idle | No | No | No | mpv (libplacebo, gpu-next default since 0.41) | A Gunmetal opportunity: libmpv can do this on the client |
| Match dynamic range and frame rate | TV switches to the content's HDR mode and refresh rate | Client dependent (unverified) | Client dependent (unverified) | Client dependent (unverified) | Apple TV "Match Content" setting; Kodi refresh-rate switching (unverified) | Done by the player, not the server |
| Pick DV or HDR10 rendition | Force HDR10 on a TV with a poor DV mode | (unverified) | Requested, [#2511](https://features.jellyfin.org/posts/2511/choose-between-dv-hdr) (19) | (unverified) | Nobody | |
| HDR and DV badges and filters | Find HDR titles at a glance | Yes (unverified) | DV tag done ([#2314](https://features.jellyfin.org/posts/2314/add-dolby-vision-tag-whenever-applicable), 55); HDR filter requested ([#1323](https://features.jellyfin.org/posts/1323/filtering-by-hdr-sdr-movies), 32) | (unverified) | Unclear | Cheap for Gunmetal: the parser already sees the colour metadata |
| Subtitle brightness on HDR | White subtitles do not glare at full HDR brightness | Users report subtitles "blindingly bright" on Apple TV ([forum](https://forums.plex.tv/t/appletv-subtitles-are-blindingly-bright/840174)) | Separate HDR subtitle colour requested ([#3358](https://features.jellyfin.org/posts/3358/set-different-subtitles-color-for-sdr-and-hdr-dv-content), 3) | (unverified) | Infuse 8.2 (HDR subtitle brightness setting); mpv 0.41 added subtitle peak controls | Small fix, big comfort gain |

### Audio

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Preferred audio language | Right dub or original audio by default | Yes | Yes | Yes | Tie | |
| Remember track choices across episodes | Pick once per series | (unverified) | Done ([#194](https://features.jellyfin.org/posts/194/remember-subtitle-and-audio-track-choice-between-episodes), 335 votes) | (unverified) | Jellyfin | |
| Track picker before playback | Choose audio and quality before pressing play | Missing in the new iOS app preview ([forum](https://forums.plex.tv/t/current-missing-features-bugs-with-the-ios-app-experience-preview/896263)) | Yes (web, unverified elsewhere) | Yes (unverified) | Infuse (unverified) | |
| Compressed passthrough (AC3, E-AC3, DTS) | Receiver decodes surround | Client dependent | Client dependent | Client dependent | Kodi; Infuse passes Atmos in E-AC3 to compatible receivers | |
| Lossless passthrough (TrueHD, DTS-HD MA) | Bit-perfect disc audio, including Atmos and DTS:X | New apps reported transcoding TrueHD ([feedback thread](https://forums.plex.tv/t/new-experience-public-release-feedback/910904)); TrueHD Atmos failures ([forum](https://forums.plex.tv/t/atmos-dobly-true-hd-playback-fails/786084)) | Android TV client (unverified) | (unverified) | Kodi on Android TV and CoreELEC boxes (unverified) | Apple TV support depends on tvOS; tvOS 26 reportedly added passthrough (unverified) |
| Seamless format changes on Apple TV | No audio dropouts between stereo and Atmos | n/a | n/a | n/a | tvOS 26.4 "Continuous Audio Connection" using Dolby MAT | Platform feature Gunmetal's Apple client inherits |
| Stereo downmix choice | Clearer dialogue from 5.1 on TV speakers | (unverified) | Choice of downmix algorithms (10.9), expanded in 10.10 | (unverified) | Jellyfin | |
| Dialogue enhancement / night mode | Speech stays clear at low volume | Requested ([forum](https://forums.plex.tv/t/volume-boost-night-mode-industry-standard-av-fx-audio-and-subtitles-offset-settings/233703)) | Downmix options only (unverified) | (unverified) | Apple TV "Enhance Dialogue" (tvOS 18), which works over TV speakers, HomePod, AirPods and receivers | Netflix says nearly half of US viewing hours have subtitles on, partly for this reason |
| Volume boost | Quiet files made louder | (unverified) | (unverified) | (unverified) | Infuse 8.0, with quick access in the player | |
| Audio delay (lip sync) | Fix Bluetooth or receiver lag | Only on some desktop clients; request open since 2014 (59 votes, [forum](https://forums.plex.tv/t/feature-request-lip-synch-audio-offset/76923)) | Requested ([#3451](https://features.jellyfin.org/posts/3451/ability-to-adjust-audio-delay), 19); per-codec presets ([#3119](https://features.jellyfin.org/posts/3119/lipsync-profiles-set-up-multiple-audio-sync-presets-and-auto-apply-them-per-codec), 3) | (unverified) | mpv and Kodi, with keyboard-driven 0.1 s steps (Kodi unverified) | |
| Saved sync per series | Fix once for a whole show | No (unverified) | Requested ([#2163](https://features.jellyfin.org/posts/2163/subtitles-and-audio-sync-per-series), 18) | (unverified) | Nobody | |
| External audio files | Separate commentary or dub files beside the video | (unverified) | Subfolder support requested ([#2562](https://features.jellyfin.org/posts/2562/audio-and-subtitles-in-subfolders), 17) | (unverified) | Infuse 8.4 added external audio tracks | |
| Switching tracks during a transcode | Change language mid-stream without restarting | Yes (unverified) | Yes (unverified) | Yes (unverified) | Infuse 8.5.4 added this for server transcodes | |
| Commentary and description labels | Tell commentary and audio description apart | (unverified) | Requested ([#1206](https://features.jellyfin.org/posts/1206/commentary-support-show-subtitle-and-audio-flags-in-title), 5) | (unverified) | Netflix (unverified) | MKV has flags for this |
| Audio-only mode for a video | Listen to a concert film with the screen off | (unverified) | Requested ([#533](https://features.jellyfin.org/posts/533/audio-only-option-on-video-playback), 34) | (unverified) | Nobody | Pairs with Gunmetal's music player |
| Disable Atmos | Avoid a bad Atmos decoder in a soundbar | n/a | n/a | n/a | Infuse 8.1.7 | |
| Filter library by audio language | Find titles with a given dub | (unverified) | Requested ([#39](https://features.jellyfin.org/posts/39/filter-videos-by-audio-stream-language), 172) | (unverified) | Nobody | Free once tracks are indexed |

### Subtitles

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Plain text (SRT, WebVTT) | Works everywhere without transcoding | Yes | Yes | Yes | Tie | |
| Styled ASS/SSA | Anime signs, karaoke and positioned lines render as authored | Many clients cannot render ASS; staff confirmed the Android TV player cannot, leaving a choice of burn-in or lost styling ([forum](https://forums.plex.tv/t/nvidia-shield-android-app-ssa-ass-format-anime-subtitles-transcoding-problems/446811)); new iOS app mis-renders ASS ([forum](https://forums.plex.tv/t/new-ios-plex-app-not-correctly-displaying-ass-subtitle-fonts-and-styling/914929)) | Web renders ASS with `@jellyfin/libass-wasm`; ASS accuracy during hardware burn-in improved in 10.11 | (unverified) | mpv and Kodi (libass); Infuse | The biggest subtitle gap in Plex |
| Embedded subtitle fonts | ASS uses the fonts shipped in the MKV | (unverified) | Yes on web (unverified) | (unverified) | mpv | |
| PGS (Blu-ray image subtitles) | Disc subtitles without conversion | Often burned in (unverified) | Client-side PGS rendering on web (10.10, via `libbitsub`); Android TV gained support (forum) | (unverified) | mpv, Kodi, Infuse | Burn-in is "the most intensive method of transcoding", per Jellyfin's codec page |
| VobSub and DVB | DVD and broadcast image subtitles | (unverified) | VobSub added in 12.0 | (unverified) | mpv, Kodi, Infuse | |
| Burn-in policy | Control when subtitles force a transcode | Setting exists; users told to avoid it ([forum](https://forums.plex.tv/t/new-option-burn-subtitles-avoid-at-all-costs/320968)); new TV app broke forced burn-in ([forum](https://forums.plex.tv/t/bug-with-subtitles/932598)) | Can disable burn-in; burn-in appearance options requested ([#3151](https://features.jellyfin.org/posts/3151/burn-in-subtitle-appearance-options), 3) | (unverified) | Native players that never need it | |
| Forced subtitles | Only foreign-language lines shown automatically | Turns on tracks tagged forced (forum reports, unverified) | "Only forced" mode (unverified); smarter forced/SDH detection from track titles requested ([#3404](https://features.jellyfin.org/posts/3404/intelligent-subtitle-type-detection-forced-full-sdh-from-embedded-track-titles), 15) | (unverified) | mpv, with `subs-fallback-forced` | Badly tagged files are the norm |
| Smart auto-select | Subtitles on when audio is foreign, off otherwise | Yes, "shown with foreign audio" (unverified wording) | Yes, "Smart" mode (unverified) | (unverified) | mpv (`subs-with-matching-audio`) | |
| Fallback subtitle language | Second choice when the first is missing | (unverified) | Requested ([#3675](https://features.jellyfin.org/posts/3675/fallback-language-for-preferred-subtitles), 8) | (unverified) | Nobody | |
| SDH/CC versus dialogue-only | Choose captions with sound cues or plain dialogue | (unverified) | Detection requested (#3404) | (unverified) | Netflix, which now ships a dialogue-only track beside the CC track on new originals | |
| Track labels | Know which track is which | New Android app reportedly listed every embedded track as "unknown" (forum, unverified) | Show real title instead of language requested ([#164](https://features.jellyfin.org/posts/164/change-subtitle-title-to-actual-title-instead-of-language), 7); group by language ([#4146](https://features.jellyfin.org/posts/4146/group-subtitle-tracks-by-language-in-subtitle-selector), 1) | (unverified) | Unclear | |
| Two subtitles at once | Learn a language with both tracks visible | (unverified) | Primary plus secondary track since 10.9; separate offsets requested ([#3275](https://features.jellyfin.org/posts/3275/allow-different-offsets-for-primary-secondary-subtitles), 6) | (unverified) | mpv (`secondary-sid`) and Jellyfin | |
| Sidecar subtitle files | `.srt` and `.ass` next to the video are picked up | Yes | Yes; subfolder support requested ([#2562](https://features.jellyfin.org/posts/2562/audio-and-subtitles-in-subfolders), 17) | Yes (unverified) | mpv (`sub-auto`) | |
| One subtitle for all versions | A single `.srt` serves the 1080p and 4K files | (unverified) | Requested ([#1613](https://features.jellyfin.org/posts/1613/allow-using-one-single-subtitle-file-for-all-versions-1080p-2160p-etc), 22) | (unverified) | Nobody | |
| Subtitle search in the player | Find missing subtitles mid-film | Yes, OpenSubtitles.com, from pre-play or during playback on most apps; missing in the new iOS preview | Plugin, from the item menu, not the player ([#892](https://features.jellyfin.org/posts/892/download-subtitles-from-the-player-screen), 30; [#1713](https://features.jellyfin.org/posts/1713/search-for-subtitles-from-within-ui), 28) | Premiere only ("in-playback subtitle search") | Plex, with hash-matched results marked; Infuse offers free one-tap OpenSubtitles downloads | |
| File-hash matching | Results that match the exact release | Yes, a star marks hash matches | Via plugin (unverified) | (unverified) | Plex | |
| More subtitle providers | Better coverage for small languages | OpenSubtitles only | Requested ([#2225](https://features.jellyfin.org/posts/2225/support-for-all-the-larger-subtitles-repositories), 29) | (unverified) | Bazarr in the *arr ecosystem (unverified) | |
| Automatic download | Missing subtitles fetched in the background | No (unverified) | Plugin task (unverified); requests declined or open ([#3608](https://features.jellyfin.org/posts/3608/opensubtitles-background-task-for-automatic-downloading)) | (unverified) | Bazarr (unverified) | |
| Manual offset | Shift subtitles earlier or later | Yes, 50 ms steps; larger steps requested (79 votes, [forum](https://forums.plex.tv/t/fix-subtitle-offsets-mod-option-to-increase-step-size/375120)) | Yes (web, unverified elsewhere) | Yes (unverified) | mpv (one key per 0.1 s) | |
| Saved offset | The fix sticks for that file or series | No (unverified) | Requested ([#2158](https://features.jellyfin.org/posts/2158/subtitle-offset-as-a-saved-value), 5; [#2163](https://features.jellyfin.org/posts/2163/subtitles-and-audio-sync-per-series), 18) | (unverified) | Nobody | |
| Automatic re-timing | Out-of-sync subtitles fixed without fiddling | No | No | No | Outside tools: ffsubsync (MIT) and alass (Rust, GPL-3.0) align subtitles to speech or to a reference track | Nobody builds this into the player |
| Appearance settings | Size, font, colour, background, outline, position | Yes; new Apple TV app criticised for missing outline and size control ([forum](https://forums.plex.tv/t/rant-new-atv-app-no-thanks-i-want-the-old-one-back/943162)) | Yes in web; "improve appearance options" [#161](https://features.jellyfin.org/posts/161) (188 votes) | Yes (unverified) | mpv for control; Apple TV for simplicity | |
| Change appearance while watching | Adjust without leaving the film | (unverified) | (unverified) | Appearance dialog now docks to the side so subtitles stay visible while editing (recent release notes) | Apple TV app (tvOS 26.4), with a speech-bubble menu in the player | |
| Appearance preview | See the style before choosing | (unverified) | Requested ([#3224](https://features.jellyfin.org/posts/3224/subtitle-preview), 16) | Partly: the docked dialog shows live subtitles while editing | Emby, for live preview in place | |
| Auto-show on mute or skip back | Subtitles appear for the line you missed | No | No | Added in 4.11 beta ("automatic subtitles on skip back") | Apple TV (tvOS 18), which also shows them when muted | Easy client feature |
| Machine-generated subtitles | Subtitles for files that have none | No | No | No | Nobody among the rivals; Bazarr Whisper providers exist (unverified) | Expensive on weak hardware |
| Subtitles when casting | Same subtitles on a Chromecast | Reported broken from Android ([forum](https://forums.plex.tv/t/subtitle-issues-when-casting-from-android/922901)) | (unverified) | (unverified) | Unclear | |

### Chapters, seeking and scrub previews

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Chapter list in the player | Jump to a scene | Yes (unverified on new apps) | Yes on web; quick access on Android TV requested ([#3604](https://features.jellyfin.org/posts/3604/android-tv-quick-access-to-chapters-and-play-queue-during-playback-plex-like-navigation), 3) | Yes (unverified) | Kodi and mpv | |
| Chapter thumbnails | Picture per chapter | Yes (unverified) | Chapter image extraction (unverified) | Yes (unverified) | Unclear | |
| Chapter names on the seek bar | Know which scene you are scrubbing into | (unverified) | Added to seek preview bubble in 12.0 | (unverified) | Jellyfin 12.0 | |
| Hidden and ordered chapters | MKV editions and linked segments | (unverified) | Hidden chapters requested ([#2963](https://features.jellyfin.org/posts/2963/support-hidden-chapters-and-subchapters), 6) | (unverified) | mpv (unverified) | Niche, used in anime releases |
| Scrub previews (pre-generated) | Thumbnails while dragging the timeline | Yes ("video preview thumbnails"); can take 10+ minutes of CPU per movie and 10–50 MB per item | Yes since 10.9; keyframe-only extraction about 100x faster since 10.10 | Yes (unverified) | Jellyfin for cost; Plex for coverage across apps | New Plex iOS preview shipped without them |
| On-demand scrub previews | No pre-generation and no storage | No | Requested ([#4143](https://features.jellyfin.org/posts/4143/generate-trickplay-file-on-playback), 4) | No | mpv with the thumbfast script, which runs a second background mpv to grab frames as you hover | Gunmetal's segment map makes this cheap (see below) |
| Configurable seek steps | Choose 10 s or 30 s jumps | Yes (unverified); 10/30 s buttons missing in new iOS preview | Done for web ([#3456](https://features.jellyfin.org/posts/3456/allow-configurable-skip-intervals-forward-back-seek-in-the-web-player)) | (unverified) | mpv, Kodi | |
| Double-tap to seek | Phone-style gesture | Missing in new app (forum) | Requested ([#131](https://features.jellyfin.org/posts/131/double-tap-to-fast-forward-or-rewind), 109) | (unverified) | YouTube-style players (unverified) | |
| Frame-by-frame stepping | Pause on the exact frame | (unverified) | Comma and period keys in 12.0 | (unverified) | mpv, which also steps backwards | |
| A-B loop | Repeat a passage | No (unverified) | No (unverified) | No (unverified) | mpv | Useful for language learners |
| Keyboard shortcuts | Full control from a keyboard | Yes on web (unverified) | Yes; more requested ([#267](https://features.jellyfin.org/posts/267/left-right-arrow-keys-should-skip-back-forward), 11) | (unverified) | mpv | |
| Time remaining and "ends at" | When the film will finish | (unverified) | Yes; option to hide requested ([#3987](https://features.jellyfin.org/posts/3987/add-option-to-hide-estimated-playback-end-time-ends-at-in-osd)) | (unverified) | Jellyfin | |

### Intro, recap and credits skipping

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Intro detection | "Skip intro" on TV episodes | Plex Pass; audio matching of the theme across a season's episodes | Media segments framework (10.10); detection from chapters (official plugin) or the community Intro Skipper plugin. "Skip intro/credits" was [#45](https://features.jellyfin.org/posts/45/skip-intro-outro-credits-feature) (1,089 votes, completed) | Premiere; scans the first 10 minutes and needs at least two episodes in the season | Plex, for quality out of the box; Jellyfin, for being free with the plugin | Intro Skipper (2.8k GitHub stars) is described as the most installed Jellyfin plugin (secondary source) |
| Credits detection | "Skip credits" jumps to post-play | Plex Pass; machine learning on text, black frames and other signals; movies and episodes | Intro Skipper detects outros | Not documented | Plex, which also finds mid- and post-credits scenes | |
| Recap and preview segments | Skip "previously on" and next-week trailers | (unverified) | Segment types include Recap, Preview and Commercial | (unverified) | Infuse 8.4 (intros, credits and recaps) | |
| Auto-skip versus button | Choose automatic, ask, or never | Show button (auto unverified) | Per-client action (web fully supported in 10.10; other clients pending at the time) | Ignore, skip automatically, or show a button | Emby, for the clearest three-way setting; Infuse added auto-skip in 8.1.5 | |
| "Ask to not skip" | Auto-skip with a short grace prompt to cancel | No | Requested ([#2954](https://features.jellyfin.org/posts/2954/implement-ask-to-not-skip-for-intro-outro-segments), 30) | No | Nobody | |
| Crowd-sourced markers | Markers without local analysis | No | IntroDB lists Jellyfin among integrations | No | Infuse 8.4, using the community databases IntroDB and TheIntroDB, matched by IMDb ID | Sends what you watch to a third party |
| Manual marker editing | Fix a wrong marker | (unverified) | Via plugin (unverified) | (unverified) | IntroDB, where anyone can submit or challenge a timestamp | |
| Skip marks as watched | Skipping the credits counts as finished | (unverified) | Requested ([#2298](https://features.jellyfin.org/posts/2298/option-to-have-skip-button-mark-media-as-watched-played), 6) | (unverified) | Unclear | |
| Paywall | Whether skipping is free | Plex Pass for detection and for each viewer (or their Home admin) | Free | Premiere | Jellyfin | |

### Resume, next up and autoplay

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Resume position | Carry on where you stopped, on any device | Yes | Yes | Yes | Tie | |
| Rewind on resume | Back up a few seconds for context | Plex Pass | (unverified) | (unverified) | Plex | |
| Continue Watching row | Half-finished titles on the home screen | Yes | Yes | Yes | Netflix, for its removal control (unverified) | |
| Remove from Continue Watching | Hide something you abandoned | Yes (unverified) | Most-voted video request: [#517](https://features.jellyfin.org/posts/517) (1,725 votes, planned) | (unverified) | Netflix (unverified) | |
| Next Up | The next episode of each show in progress | Yes ("On Deck") | Yes; merging with Continue Watching requested ([#1055](https://features.jellyfin.org/posts/1055), 203) | Yes | Unclear | |
| Autoplay next episode | Binge without touching the remote | Yes; "Pass Out Protection" customisation is Plex Pass | Yes; "stop after this episode" on Android TV requested ([#3970](https://features.jellyfin.org/posts/3970/add-stop-after-current-episode-on-android-tv), 3) | Yes | Netflix-style countdown during credits (unverified) | |
| "Still watching?" prompt | Stops a sleeping viewer from burning a season | Plex Pass customisation | Added in 12.0 | (unverified) | Netflix (unverified) | |
| Pre-buffer next episode | No spinner between episodes | (unverified) | Requested ([#400](https://features.jellyfin.org/posts/400/when-play-next-episode-automatically-is-selected-begin-buffering-the-next-episode), 222) | (unverified) | Nobody confirmed | Trivial with direct play and a segment map |
| Watch history | See everything watched, when | (unverified) | Requested ([#633](https://features.jellyfin.org/posts/633), 830, planned) | (unverified) | Trakt-style services (unverified) | Gunmetal's append-only log already holds this |
| Resume per version | 4K and 1080p copies share or keep progress | (unverified) | 12.0 tracks resume for the version actually watched | (unverified) | Jellyfin 12.0 | |
| Sleep timer | Stop after N minutes or at episode end | Missing in new iOS preview | (unverified) | (unverified) | Unclear | |
| Missing-episode handling | Autoplay skips gaps sensibly | (unverified) | Requested ([#341](https://features.jellyfin.org/posts/341/auto-play-next-episode-options-for-handling-missing-episodes), 27) | (unverified) | Unclear | |
| Hide titles during autoplay | No spoilers from the next title card | (unverified) | (unverified) | (unverified) | Infuse 8.2 can hide video titles during continuous play | |

### Playback speed, picture-in-picture and background play

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Playback speed | Watch faster or slower | Plex Pass; 0.5x to 2x on Fire TV, Android, Android TV, Apple TV, iOS and Linux desktop | Yes; finer steps planned ([#3147](https://features.jellyfin.org/posts/3147/more-granular-playback-speed-controls), 23) and requested ([#2662](https://features.jellyfin.org/posts/2662/finer-playback-speeds-0-95x-1x-1-05x-1-1x), 20) | (unverified) | mpv (0.01x to 100x with pitch correction by scaletempo2) | Netflix offers 0.5x to 1.5x |
| Pitch-corrected speed | Voices sound normal when sped up | (unverified) | (unverified) | (unverified) | mpv | |
| Hold for 2x | Temporary fast-forward while held | No (unverified) | No (unverified) | No (unverified) | Infuse 8.0.5, with haptic feedback in 8.0.6 | |
| Speed indicator | Know you are not at 1x | (unverified) | Requested ([#3435](https://features.jellyfin.org/posts/3435/create-visual-indicator-for-playback-speed), 6) | (unverified) | Unclear | |
| Picture-in-picture | Keep watching while using other apps | Missing in new iOS preview; audio kept playing in the background instead | Yes on mobile (unverified) | (unverified) | Apple's system PiP on iOS and tvOS (unverified) | |
| Background playback | Audio continues with the screen off | (unverified) | 12.0 fixed iOS background playback with the screen off | (unverified) | Unclear | |
| Cast and AirPlay | Send to a TV | Casting reported unresponsive in new apps (feedback thread) | Chromecast (unverified) | Yes (unverified) | Plex historically (unverified) | |
| Zoom and aspect modes | Fill a phone screen or fix bad aspect | Pinch zoom broken in new iOS preview | (unverified) | (unverified) | Infuse 8.1.7 added "Aspect Fit" and other zoom modes | |
| Remote control of another device | Phone controls the TV session | Yes (unverified) | Yes; Roku remote control requested ([#3423](https://features.jellyfin.org/posts/3423/enable-remote-playback-control-for-jellyfin-on-roku-devices), 5) | Yes (unverified) | Unclear | |

### Watch-together

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Synchronised group playback | Friends in different homes watch in step | Watch Together; dropped from the new apps in 2025 and kept only on the web app | SyncPlay, on web-wrapper clients only; transcoding can break sync | No native feature (unverified) | Apple SharePlay on Apple devices (unverified); Plex before 2025 | Plex's request to restore it has 2,878 votes |
| TV coverage | Group watch on the living-room box | Lost with the new apps | Android TV SyncPlay started ([#2282](https://features.jellyfin.org/posts/2282/add-syncplay-for-android-tv-devices), 37) | n/a | Nobody today | |
| Invite link | Send a link instead of creating accounts | Invited Plex friends (account needed) | Requested ([#971](https://features.jellyfin.org/posts/971/syncplay-invite-to-watch-link), 267); guests without library access ([#863](https://features.jellyfin.org/posts/863/syncplay-user-without-library-access), 27) | n/a | Nobody | Gunmetal's per-object tokens fit this exactly |
| Chat and reactions | Talk without a separate call | Reactions requested (forum) | Chat requested ([#740](https://features.jellyfin.org/posts/740/add-chat-to-syncplay), 104) | n/a | Unclear; Plex users point to Hulu's watch party as a model (forum) | |
| Ready check | Start only when everyone is ready | Requested (forum) | Requested ([#3251](https://features.jellyfin.org/posts/3251/ready-state-for-syncplay), 43) | n/a | Nobody | |
| Group permissions | Only the host can pause or seek | (unverified) | Requested ([#747](https://features.jellyfin.org/posts/747/syncplay-playback-access-control), 13) | n/a | Nobody | |
| Group naming and rooms before play | Set up the room, then pick the film | (unverified) | Started ([#869](https://features.jellyfin.org/posts/869/syncplay-rooms-before-playback), 30; [#1093](https://features.jellyfin.org/posts/1093/syncplay-name-group), 23) | n/a | Unclear | |
| Same output on several local screens | Play in sync on many TVs in one house | Requested since 2013 ([forum](https://forums.plex.tv/t/tandem-playback-to-several-clients/38777), 420 votes) | SyncPlay can do it on supported clients | n/a | Nobody well | |
| Episodes in a session | Binge together | Request with 86 posts ([forum](https://forums.plex.tv/t/allow-watch-together-for-tv-shows-mod-more-than-1-episode-at-a-time/644669)) | Queue-based (unverified) | n/a | Unclear | |

### Trailers, pre-rolls and extras

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Cinema trailers before a film | A cinema-style run-up | Plex Pass; not all apps support it | Via plugin; a trailers plugin is [#55](https://features.jellyfin.org/posts/55/trailers-plugin) (585 votes) | Premiere ("Cinema Intros") | Plex | |
| Custom pre-roll videos | A studio ident or "phones off" clip | Yes; must sit in a library the viewer can access; reported broken on new iOS app (feedback thread) | Local intros plugin; playback improvements requested ([#2710](https://features.jellyfin.org/posts/2710/improve-playback-for-local-intros-pre-rolls), 29) | Premiere | Plex | |
| Trailer on the detail page | Watch the trailer before deciding | Yes | Yes (local or remote); better local handling requested ([#3033](https://features.jellyfin.org/posts/3033/better-handling-of-local-trailers), 20) | Yes (unverified) | Infuse 7.5 added trailers; now a Pro feature | Remote trailers mean a third-party fetch |
| Extras | Deleted scenes and featurettes | Yes | Yes | Yes | Infuse 8.4, which surfaces extras from all three servers on the title page | |
| Theme music and video | Ambience on the detail page | Removed in new apps (feedback thread) | Yes; random order in 10.11 | Yes (unverified) | Jellyfin | |
| Save and share a scene | Bookmark a moment, trim it, share it | No | No | No | Netflix "Moments", with start and end points and sharing from My Netflix | Sharing clips of personal media raises rights issues |
| Who is on screen | Cast and music for the current scene | No | Requested ([#2545](https://features.jellyfin.org/posts/2545/on-screen-actor-information-during-playback-x-ray), 39) | No | Apple TV "InSight" (Apple TV+ titles only) | Needs data rivals do not have for personal media |

### Statistics and diagnostics

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Statistics overlay | Codec, bitrate, resolution, dropped frames | Old apps yes; new iOS preview had no playback info at all; new Apple TV app hides media info | Web "Playback Info" made more compact in 12.0; Android TV version planned ([#272](https://features.jellyfin.org/posts/272), 219 votes) | (unverified) | mpv's stats overlay (`i` shows, `I` pins); Infuse 8.3.6 added a real-time HUD | |
| Reason for transcoding | "Transcoding because the TV cannot decode TrueHD" | Dashboard (unverified detail) | In playback info (unverified detail) | (unverified) | Jellyfin (unverified) | A Gunmetal differentiator if done well |
| Live sessions for the owner | Who is watching what, and how | Yes | Yes | Yes | Tie | |
| Stop someone's stream | Owner ends a session | (unverified) | Requested ([#301](https://features.jellyfin.org/posts/301), 402) | (unverified) | Unclear | |
| Hardware transcode self-test | Know the GPU path works before guests complain | No (unverified) | Planned ([#450](https://features.jellyfin.org/posts/450), 423 votes); transcode test ([#223](https://features.jellyfin.org/posts/223/transcode-test), 40) | (unverified) | Nobody | |
| Full file information | Every stream and flag in the file | (unverified) | Done ([#1138](https://features.jellyfin.org/posts/1138/add-a-section-with-full-file-infos)) | (unverified) | Jellyfin and Infuse | |

### Player engine and display (local players are the relevant rivals)

| Feature | What the user gets | Infuse | Kodi | mpv | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Format breadth | Nearly any file plays | Very broad list, including VVC, VC-1, ISO and disc folders | Very broad (unverified) | Very broad (FFmpeg) | Tie between all three | Gunmetal inherits mpv's breadth on native clients |
| Renderer quality | Scaling, dithering, colour management | Own engine (unverified detail) | Own renderer (unverified) | libplacebo, default since 0.41 | mpv | |
| Display sync and interpolation | Judder-free 24p on 60 Hz screens | Relies on Apple TV Match Content | Refresh-rate switching (unverified) | `display-resample` with interpolation | mpv | |
| AI upscaling | Lower-resolution files look sharper | Yes (8.2) | No (unverified) | Nvidia RTX and Intel VSR via d3d11va on Windows (0.40) | Infuse on Apple; mpv on Windows | |
| HDR on Linux desktops | Real HDR output, not just tone mapping | n/a | Kodi 22 adds fuller HDR on embedded Linux (RC as of late September 2026) | Native HDR on DRM and Wayland (0.40, 0.41) | mpv | |
| Dolby Vision output | True DV picture | P5 and P8 via Apple's player APIs; no P7 | Fixed DV on Android in Kodi 22 beta 2 | Applies DV metadata in the renderer; no enhancement-layer composition | Infuse on Apple TV | |
| Subtitle engine | ASS, PGS, VobSub, DVB and more | Broad list | libass (unverified) | libass plus image formats | mpv and Infuse | |
| Sync with other viewers | Watch parties | Not advertised | No (unverified) | Through the separate Syncplay program (unverified) | Nobody native | |

### Usability references (streaming apps are the relevant references)

| Feature | What the user gets | Netflix | Apple TV app | Infuse | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Subtitle style changes during play | No trip to Settings | (unverified) | Yes (tvOS 26.4) | Yes (unverified) | Apple TV app | |
| Auto subtitles when muted or after skip back | Never miss a line | No (unverified) | Yes (tvOS 18) | No (unverified) | Apple TV app | Emby is copying the skip-back half |
| Dialogue-only subtitle option | Cleaner subtitles for hearing viewers | Yes (new originals since April 2025) | (unverified) | n/a | Netflix | |
| Clear dialogue | Speech lifted over effects | (unverified) | Enhance Dialogue on any output | Volume boost | Apple TV app | |
| Skip intro, recap and credits | One press, or automatic | Skip intro and recap buttons (unverified) | Apple TV+ only (unverified) | Auto or manual, from crowd-sourced databases | Infuse for personal media | |
| Scene bookmarks | Save and revisit a moment | Moments | No | No | Netflix | |
| On-screen cast and music | Who and what is in this scene | No (unverified) | InSight | No | Apple TV app | |
| Profiles without accounts | Kids and guests get their own state | Profiles | Profiles without an Apple Account (tvOS 26.2) | Multi-user profiles (8.2) | Apple TV app | Relevant to watch-together guests |

## Pain points and unmet demand

Each item gives the complaint, the evidence, and the scale where the source
shows it.

1. **Basic playback features sit behind subscriptions.** Plex charges for
   remote playback since April 2025, either through Plex Pass ($6.99 a month,
   $69.99 a year) or the Remote Watch Pass, which rose to $2.99 a month or
   $29.99 a year on 1 June 2026. The lifetime Plex Pass went from $249.99 to
   $749.99 on 1 July 2026. Plex also gates skip intro and credits, playback
   speed, hardware transcoding, HDR tone mapping, rewind-on-resume and
   autoplay customisation. Emby Premiere ($4.99 a month, $54 a year, $119
   lifetime) gates intro skipping, hardware transcoding, tone mapping,
   in-playback subtitle search and cinema intros, and its matrix limits free
   playback to one minute on the desktop, game console and Android/Fire TV
   apps.
   Evidence: [Plex lifetime pricing announcement](https://forums.plex.tv/t/new-lifetime-plex-pass-pricing/938910),
   [How-To Geek on the lifetime increase](https://www.howtogeek.com/plex-is-tripling-the-price-of-its-lifetime-passand-hinting-it-might-disappear-forever/),
   [Android Authority on the Remote Watch Pass increase](https://www.androidauthority.com/plex-remote-watch-pass-price-increase-3663060/),
   [Plex Pass page](https://www.plex.tv/plex-pass/),
   [Emby feature matrix](https://emby.media/support/articles/Premiere-Feature-Matrix.html),
   [Emby Premiere pricing](https://emby.media/premiere.html).

2. **Watch-together was taken away, and nobody has replaced it well.** Plex
   dropped Watch Together from its rebuilt apps in February 2025. The
   request to restore it has 2,878 votes, 361 posts and 22,207 views, with
   posts as recent as 28 September 2026; the thread announcing the removal
   has 400 posts, 14,973 views and 3,503 likes. A request for synchronised
   playback across several clients has 420 votes and has been open since
   2013. Jellyfin's SyncPlay only works on web-wrapper clients; its top
   follow-up requests are an invite link (267 votes), chat (104), a ready
   check (43) and Android TV support (37, started).
   Evidence: [Plex: Add Watch Together to New Plex Experience](https://forums.plex.tv/t/add-watch-together-to-new-plex-experience/906941),
   [Plex: Watch together going away in app](https://forums.plex.tv/t/watch-together-going-away-in-app/906895),
   [Plex: Tandem playback](https://forums.plex.tv/t/tandem-playback-to-several-clients/38777),
   [Jellyfin #971](https://features.jellyfin.org/posts/971/syncplay-invite-to-watch-link),
   [Jellyfin #740](https://features.jellyfin.org/posts/740/add-chat-to-syncplay),
   [Jellyfin #3251](https://features.jellyfin.org/posts/3251/ready-state-for-syncplay),
   [Jellyfin #2282](https://features.jellyfin.org/posts/2282/add-syncplay-for-android-tv-devices),
   [Android Authority on SyncPlay](https://www.androidauthority.com/jellyfin-syncplay-explained-3530437/).

3. **Styled and image subtitles force transcodes or render wrongly.** A Plex
   thread about ASS subtitles on the Nvidia Shield ran to 397 posts and
   27,358 views; Plex staff said the app's video player cannot render ASS,
   so users must choose between a quality-destroying burn-in and losing the
   styling. The rebuilt iOS app mis-renders ASS fonts and styling (24
   posts), and a 51-post thread advises avoiding the burn-in option
   entirely. Jellyfin's own codec page calls subtitle burn-in the most
   intensive kind of transcoding, and Android TV users report PGS problems.
   Evidence: [Plex Shield ASS thread](https://forums.plex.tv/t/nvidia-shield-android-app-ssa-ass-format-anime-subtitles-transcoding-problems/446811),
   [Plex iOS ASS thread](https://forums.plex.tv/t/new-ios-plex-app-not-correctly-displaying-ass-subtitle-fonts-and-styling/914929),
   [Plex burn-in thread](https://forums.plex.tv/t/new-option-burn-subtitles-avoid-at-all-costs/320968),
   [Jellyfin codec support](https://jellyfin.org/docs/general/clients/codec-support/),
   [Jellyfin forum on PGS and Android TV](https://forum.jellyfin.org/t-android-tv-jellyfin-will-not-display-certain-pgs-subtitles?pid=40805).

4. **Plex's rebuilt apps regressed the player.** The public feedback thread
   for the new experience has 1,531 posts, 42,918 views and 6,486 likes. It
   reports Dolby Vision profile 5 failures, TrueHD being transcoded,
   pre-rolls not playing on iOS, casting buttons not responding and theme
   music removed. The iOS preview's own known-issues list included no
   playback information overlay, no scrub thumbnails, no 10/30-second skip
   buttons, no picture-in-picture, no subtitle search, no pre-play track
   selection and no sleep timer. A thread opened on 21 September 2026
   against the new Apple TV app reached 268 posts and 1,066 likes in under
   two weeks, citing a hanging pause, a buried subtitle offset, hidden media
   info, unreadable subtitles and broken direct play on Apple TV HD.
   Evidence: [New Experience feedback](https://forums.plex.tv/t/new-experience-public-release-feedback/910904),
   [iOS preview missing features](https://forums.plex.tv/t/current-missing-features-bugs-with-the-ios-app-experience-preview/896263),
   [Rant: New ATV app](https://forums.plex.tv/t/rant-new-atv-app-no-thanks-i-want-the-old-one-back/943162).

5. **Dolby Vision is a minefield.** Plex cannot transcode profile 5, and a
   moderator confirmed files must be converted before they are added
   (thread: 8 posts, 574 views, opened January 2026). Apple TV hardware
   cannot play dual-layer profile 7, so Infuse falls back to HDR10, and
   people convert files offline with dovi_tool or DV7toDV8. Jellyfin's
   enhancement-layer work is still an open pull request, marked opt-in
   because it can exhaust GPU memory.
   Evidence: [Plex: Transcode Dolby Vision](https://forums.plex.tv/t/transcode-dolby-vision/935235),
   [Firecore DV profile 7 thread](https://community.firecore.com/t/dolby-vision-profile-7-8-support-ts-mkv-files/19713),
   [jellyfin-ffmpeg PR 774](https://github.com/jellyfin/jellyfin-ffmpeg/pull/774),
   [DV7toDV8](https://github.com/nekno/DV7toDV8),
   [dovi_tool](https://github.com/quietvoid/dovi_tool).

6. **Remote quality is either bad by default or expensive to fix.** Plex
   apps long defaulted to 2 Mbps / 720p remotely, and turning on automatic
   quality makes the server transcode. Admins asked for a server-side
   minimum from 2017 (138 posts, 27,989 views) without getting one.
   Jellyfin users want pre-transcoding (979 votes), downloads of transcoded
   copies (518), parallel connections for better internet streaming (231),
   a bitrate ladder (56) and real adaptive streaming (55).
   Evidence: [Plex 2 Mbps default](https://forums.plex.tv/t/bug-plex-app-defaults-to-2mbps-720p/449537),
   [Plex: Force higher remote quality](https://forums.plex.tv/t/force-higher-remote-quality/195464),
   [Jellyfin #570](https://features.jellyfin.org/posts/570),
   [Jellyfin #57](https://features.jellyfin.org/posts/57),
   [Jellyfin #2176](https://features.jellyfin.org/posts/2176),
   [Jellyfin #3680](https://features.jellyfin.org/posts/3680/bitrate-ladder-support),
   [Jellyfin #3436](https://features.jellyfin.org/posts/3436/implement-full-adaptive-bitrate-streaming-hls-dash-with-manual-quality-override).

7. **Timing fixes are clumsy and do not stick.** Plex's lip-sync request has
   been open since 2014 (59 votes, last post 26 September 2026). Plex's
   subtitle offset moves in 50 ms steps and a request for larger steps has
   79 votes. Jellyfin users ask for audio delay (19), offsets saved per
   series (18) and saved subtitle offsets (5). No rival re-times subtitles
   automatically, although open tools that do it exist.
   Evidence: [Plex lip sync](https://forums.plex.tv/t/feature-request-lip-synch-audio-offset/76923),
   [Plex subtitle step size](https://forums.plex.tv/t/fix-subtitle-offsets-mod-option-to-increase-step-size/375120),
   [Jellyfin #3451](https://features.jellyfin.org/posts/3451/ability-to-adjust-audio-delay),
   [Jellyfin #2163](https://features.jellyfin.org/posts/2163/subtitles-and-audio-sync-per-series),
   [Jellyfin #2158](https://features.jellyfin.org/posts/2158/subtitle-offset-as-a-saved-value),
   [ffsubsync](https://github.com/smacke/ffsubsync),
   [alass](https://github.com/kaegi/alass).

8. **Continue Watching and history are too rigid.** Removing an item from
   Continue Watching is Jellyfin's second most-voted request overall (1,725
   votes). A full watch history has 830 votes, buffering the next episode
   during autoplay 222, and merging Next Up with Continue Watching 203.
   Evidence: [Jellyfin top requests (API listing)](https://features.jellyfin.org/api/v1/posts?view=most-wanted&limit=60),
   [Jellyfin #517](https://features.jellyfin.org/posts/517),
   [Jellyfin #633](https://features.jellyfin.org/posts/633),
   [Jellyfin #400](https://features.jellyfin.org/posts/400/when-play-next-episode-automatically-is-selected-begin-buffering-the-next-episode),
   [Jellyfin #1055](https://features.jellyfin.org/posts/1055).

9. **Skipping depends on paid tiers or on plugins.** Skip intro and credits
   was a 1,089-vote Jellyfin request; it is now met by a framework plus a
   community plugin with 2.8k GitHub stars that needs Jellyfin's own FFmpeg
   build. Plex and Emby charge for it. Users still want an "ask to not
   skip" mode (30 votes).
   Evidence: [Jellyfin #45](https://features.jellyfin.org/posts/45/skip-intro-outro-credits-feature),
   [Intro Skipper](https://github.com/intro-skipper/intro-skipper),
   [Plex credits detection](https://support.plex.tv/articles/credits-detection/),
   [Emby intro skip](https://emby.media/support/articles/Intro-Skip.html),
   [Jellyfin #2954](https://features.jellyfin.org/posts/2954/implement-ask-to-not-skip-for-intro-outro-segments).

10. **People cannot see why playback looks or behaves the way it does.**
    "Stats for nerds" on Jellyfin's Android TV app has 219 votes. Plex's new
    apps shipped without a playback information overlay, and Apple TV users
    complain they cannot tell what "Original" quality means.
    Evidence: [Jellyfin #272](https://features.jellyfin.org/posts/272),
    [Plex iOS preview](https://forums.plex.tv/t/current-missing-features-bugs-with-the-ios-app-experience-preview/896263),
    [Plex ATV rant](https://forums.plex.tv/t/rant-new-atv-app-no-thanks-i-want-the-old-one-back/943162).

11. **Subtitle choice and appearance need work everywhere.** Jellyfin users
    ask for better appearance options (188 votes), more providers (29),
    search from the player (30 and 28), a general subtitle overhaul (46),
    use of one subtitle across versions (22), a preview (16) and forced/SDH
    detection from track titles (15). Emby charges for in-playback search.
    Apple TV and Emby are moving to automatic subtitles after a skip back,
    which shows where the bar is heading.
    Evidence: [Jellyfin #161](https://features.jellyfin.org/posts/161),
    [Jellyfin subtitle requests (API listing)](https://features.jellyfin.org/api/v1/posts?query=subtitle&limit=40),
    [Emby releases](https://github.com/MediaBrowser/Emby.Releases/releases),
    [TechRadar on tvOS 18](https://www.techradar.com/televisions/apple-tv-4k-to-get-ai-enhanced-subtitles-amazon-x-ray-style-info-while-watching-and-clearer-dialogue-in-tvos-18).

12. **Trailers and pre-rolls are an afterthought on free servers.** A
    Jellyfin trailers plugin has 585 votes and better local pre-roll
    playback has 29. Plex users report pre-rolls not firing on the new iOS
    app.
    Evidence: [Jellyfin #55](https://features.jellyfin.org/posts/55/trailers-plugin),
    [Jellyfin #2710](https://features.jellyfin.org/posts/2710/improve-playback-for-local-intros-pre-rolls),
    [Plex feedback thread](https://forums.plex.tv/t/new-experience-public-release-feedback/910904).

13. **Touch and remote ergonomics lag streaming apps.** Double-tap to seek
    has 109 votes on Jellyfin; Plex's new mobile app lost gestures; finer
    playback speeds have 23 and 20 votes on Jellyfin; Plex charges for
    speed control at all.
    Evidence: [Jellyfin #131](https://features.jellyfin.org/posts/131/double-tap-to-fast-forward-or-rewind),
    [Jellyfin #3147](https://features.jellyfin.org/posts/3147/more-granular-playback-speed-controls),
    [Jellyfin #2662](https://features.jellyfin.org/posts/2662/finer-playback-speeds-0-95x-1x-1-05x-1-1x),
    [Plex iOS preview missing gestures](https://forums.plex.tv/t/current-missing-features-bugs-with-the-ios-app-experience-preview/896263).

14. **Owners lack control over live sessions.** Killing a user's stream has
    402 votes on Jellyfin, and an automatic hardware-transcode self-test has
    423 (planned).
    Evidence: [Jellyfin #301](https://features.jellyfin.org/posts/301),
    [Jellyfin #450](https://features.jellyfin.org/posts/450).

## Where Gunmetal can be clearly better

Each idea names the pain point it answers and what would have to be true.

1. **Never burn in a subtitle.** (Pain point 3.) Native clients render ASS,
   PGS, VobSub and DVB through libmpv, so no subtitle ever forces a
   transcode there. In browsers and TV web runtimes, the remuxer delivers
   subtitle packets untouched as a side stream and the client renders them:
   ASS with a WebAssembly build of libass, image formats with a decoder.
   Jellyfin already proves this works in browsers with
   `@jellyfin/libass-wasm` and `libbitsub`. What has to be true: the core
   crate grows pure-Rust PGS, VobSub and DVB decoders (they are simple
   run-length bitmap formats), compiled to WASM for the web client; MKV font
   attachments are indexed at scan time and served with signed URLs; and
   WASM rendering is fast enough on the weakest supported TV, which must be
   measured.

2. **Explain every playback decision.** (Pain points 4 and 10.) The decision
   engine is in the shared core, so the client can show the exact rule that
   chose direct play, remux or transcode, for example: remuxing because this
   browser cannot open MKV, picture and sound untouched. The same overlay
   shows libmpv's own figures (decoder, hardware decoding, dropped frames,
   cache, display rate), as mpv's stats page does. What has to be true: the
   decision function returns a structured reason list rather than a bare
   verdict, that type is part of the protocol, and the overlay is available
   on every client from the first video release, not added later.

3. **Dolby Vision handled at the edges, not on the server GPU.** (Pain point
   5.) Three cheap moves cover most cases. First, native clients render
   profile 5 and 8 correctly on any display because libplacebo applies the
   DV reshaping during rendering, which is how mpv's default renderer works
   since 0.41. Second, when a client can only take single-layer DV, the
   remuxer converts profile 7 to 8.1 on the fly by rewriting the metadata
   and dropping the enhancement layer, the same transformation dovi_tool
   performs; its `dolby_vision` library is Rust and MIT-licensed. Third,
   when a client cannot take DV at all, the remuxer strips the metadata
   from files that have an HDR10 base layer, avoiding green-and-purple
   pictures without a transcode. Only profile 5 to an SDR browser needs the
   sandboxed transcoder. What has to be true: the remuxer parses HEVC NAL
   units well enough to find and rewrite RPU data per frame; it writes the
   correct `dvh1`/`dvhe` sample entries and configuration boxes in fMP4 and
   HLS (Jellyfin had to fix this in 12.0); we confirm the `dolby_vision`
   crate fits the core's no-`unsafe` and no-panic rules or wrap it outside
   the core; and someone with DV hardware checks results on real TVs.

4. **Scrub previews with no pre-generation and no server decoding.** (Pain
   points 4 and 10; Plex spends 10+ CPU minutes and 10–50 MB per film.) The
   segment map already records where every keyframe sits. To show a preview
   the client asks for one keyframe; the server cuts those bytes into a tiny
   one-frame file using the remuxer, which is pure byte work, and the
   client decodes it with its own hardware: a second small libmpv instance
   on native clients (the thumbfast approach), or WebCodecs in browsers that
   support the codec. Pre-generated sprite sheets, made by the sandboxed
   worker at idle using keyframe-only decoding as Jellyfin 10.10 does,
   remain a fallback for weak TVs and slow remote links. What has to be
   true: keyframe-only fetches are small enough to feel instant on a LAN
   (a 4K HEVC keyframe can be a megabyte or more (unverified)), the client
   rate-limits requests while scrubbing, and WebCodecs availability is
   checked per browser and codec.

5. **Skipping that is free, local-first and private.** (Pain point 9.) Use
   the cheapest signal first: chapter titles such as "Opening" or "Credits"
   are read by the core's MKV parser at scan time at no extra cost. Next,
   audio fingerprinting across a season, run as a background job. Then an
   optional IntroDB or TheIntroDB plugin, which by ADR 2's rule needs an
   explicit network grant because it tells a third party what you watch.
   Corrections made in the app go into the append-only log, so they survive
   database rebuilds and can be exported. Offer per-type policies (skip,
   ask, ignore) plus "ask to not skip". What has to be true: audio
   fingerprinting needs decoded audio, and TV episodes often carry AC3 or
   E-AC3, which the pure-Rust scan path cannot decode today (unverified),
   so either the sandboxed FFmpeg worker gains an analysis role (an ADR
   change) or detection waits for the client: a client that is already
   decoding audio while someone watches could compute the fingerprint and
   send it home, at the cost of the first episode having no markers.

6. **Watch-together over iroh with guest links, on every client.** (Pain
   point 2.) The host creates a session; guests get a link carrying a
   capability that is scoped to one title and a time window and can be
   revoked, which is exactly the per-object authorisation and short-lived
   signed URLs in ADR 1. Guests need no account and no library access.
   Everyone direct plays from the host's server over iroh; a small control
   channel carries play, pause, seek, ready checks, reactions and chat.
   Drift is corrected by nudging libmpv's speed by a few percent rather
   than by seeking. What has to be true: a clock-sync exchange over iroh;
   the host's upload can carry N copies of the original bitrate (a 4K remux
   to four guests can exceed a home uplink, so the session must warn and
   offer a lower version or an audio-only transcode); and the feature works
   on the TV clients, which is where Plex and Jellyfin both fall short.

7. **No paywall on playback.** (Pain point 1.) Speed control, skip markers,
   hardware transcoding, tone mapping, remote access and downloads are all
   free because the project is AGPL with no central account. What has to be
   true: nothing, beyond saying it plainly on the site and never adding a
   gated tier.

8. **History and resume built on the log.** (Pain point 8.) The watch log in
   ADR 1 is already the record, so full watch history, "remove from Continue
   Watching" (an event, not a deletion), "mark as unwatched", resume per
   version and offline progress all fall out of one model. Pre-buffering
   the next episode is trivial with direct play: during the credits the
   client fetches the first few seconds of the next file, whose byte range
   the segment map already knows. What has to be true: a merge rule for
   events written offline on two devices (position: latest wins; hide and
   unhide: last event wins), and clear derived views in SQLite.

9. **Sound fixes on the client, saved where they belong.** (Pain points 7
   and 13.) libmpv already has audio delay, pitch-corrected speed, filters
   for loudness and dynamic range, and passthrough on platforms that allow
   it. Gunmetal can add an audio delay saved per output device (a
   soundbar's lag belongs to the soundbar, not the film), a night mode and
   dialogue lift built from mpv's filters, and a "hold for 2x" gesture.
   What has to be true: per-device settings sync with the library, and the
   Apple client accepts that true lossless passthrough depends on tvOS.

10. **Subtitle timing fixed once, automatically if possible.** (Pain point
    7.) Offsets are saved per file and per series in the log. Automatic
    re-timing can align a downloaded subtitle against a correctly timed
    track already inside the file, which needs no audio decoding at all;
    alass supports this reference mode and is Rust. Aligning against speech
    needs voice detection on decoded audio, which has the same decoding
    question as idea 5. Offset steps grow with repeated presses so a
    three-second error takes a few presses, not sixty. What has to be true:
    alass's GPL-3.0 licence is confirmed compatible with AGPL-3.0-or-later
    (it should be), or the alignment algorithm is reimplemented.

11. **Smarter subtitle defaults from data read at scan time.** (Pain point
    11.) The parser reads the forced and default flags, track names and the
    statistics tags that mkvmerge writes (frame counts per track
    (unverified)), so it can tell a forced track (few cues) from a full one
    in the same language and label SDH tracks, without decoding anything.
    The OpenSubtitles file hash, which needs only the file size and the
    first and last 64 KiB (unverified), can be computed during the scan, so
    a subtitle plugin can offer exact-release matches instantly.

12. **Living-room comforts that cost nothing on the server.** (Pain point
    11 and the usability references.) Auto-show subtitles when muted or
    after a skip back, change subtitle style without leaving the film,
    dimmer subtitles on HDR content by default, a dialogue-only versus SDH
    label, and a "still watching?" prompt. All are client features; the
    work is design, not infrastructure.

13. **Honest remote quality instead of silent downgrades.** (Pain point 6.)
    Gunmetal's design favours originals, so this is where rivals with
    adaptive transcoding are strongest and Gunmetal is weakest. The answer
    is to measure and tell: probe the link over iroh, compare it with the
    file's real peak bitrate from the segment map, and offer choices: play
    the original with a bigger buffer, play a pre-made smaller version (made
    by the sandboxed worker on a schedule, which is Jellyfin's 979-vote
    request), switch to an audio-only transcode, or download for later. The
    default is never 720p. What has to be true: peak bitrate per segment is
    recorded at scan time, and the scheduled transcoder exists before
    remote video is promoted.

## Risks and hard parts

- **Apple TV and Dolby Vision.** Infuse shows DV on Apple TV by handing
  frames to Apple's own player APIs. A libmpv renderer is unlikely to output
  a real DV signal through tvOS (unverified), so the Apple TV client may
  need a second path that plays a remuxed fMP4/HLS stream through Apple's
  player for DV and some HDR content. That means two players to test on one
  platform, and Apple TV becomes a remux client for its best content.
- **libmpv on tvOS and in app stores.** mpv has no official tvOS build
  (unverified), and GPL-licensed code in the App Store has a troubled
  history (unverified). Gunmetal itself is AGPL and accepts contributions
  without a CLA, so App Store distribution of the whole client needs a legal
  answer early, not at release time.
- **Browsers and TV web runtimes.** Codec support differs by browser and
  hardware (Firefox only gained HEVC in version 134, per Jellyfin's 10.11
  notes), Dolby Vision in browsers is close to nonexistent, and HDR output
  in browsers is inconsistent. Every gap becomes either a remux, a
  client-side render, or a sandboxed transcode, so the web client will
  always be the hardest to get right.
- **Remuxer scope.** Dolby Vision signalling, RPU rewriting, lossless audio
  in fMP4, image subtitles as side streams and font attachments are all in
  the remuxer, which ADR 1 already names as the main schedule risk. Each
  needs real sample files, and good test media is hard to find under
  licences that allow committing it.
- **Analysis jobs need decoding.** Intro and credits detection, scrub-sheet
  fallback and speech-based subtitle sync all need decoded audio or video.
  ADR 1 keeps FFmpeg off the scan path. Either these jobs run in the
  sandboxed worker on a schedule (an ADR amendment), or they wait for
  pure-Rust decoders, which do not exist for AC3, E-AC3, DTS or TrueHD
  (unverified).
- **Sandboxing a GPU transcoder.** Tone mapping and hardware encoding need
  access to GPU device nodes, which widens the sandbox the README promises.
  The isolation story must be written for the GPU case specifically.
- **Watch-together bandwidth.** Direct play to several remote guests
  multiplies the host's upload. Without transcoding, a 4K session can be
  impossible on common home connections, and iroh relay traffic may cost
  someone money when hole-punching fails.
- **Profile 7 FEL.** Converting to 8.1 discards the enhancement layer.
  Purists will notice, and composing the layer properly needs two
  simultaneous HEVC decodes, which Jellyfin's own pull request warns can
  exhaust GPU memory.
- **Dolby trademarks and licensing.** Handling DV metadata in open source
  is common (dovi_tool, mpv, Jellyfin), but Gunmetal should not claim
  "Dolby Vision support" in marketing without checking trademark rules
  (unverified).
- **Third-party data and privacy.** Subtitle search, crowd-sourced intro
  markers and trailers all reveal viewing to outside services. ADR 2 already
  puts these in plugins with explicit network grants; the UI must make that
  grant understandable rather than a buried toggle.
- **Polish across platforms with one React Native codebase.** Plex's rebuild
  shows how badly a player rewrite can land with users even for a well-funded
  team. The player surface needs its own acceptance list per platform.

## Open questions

1. On Apple TV, is the player libmpv only, Apple's player only, or both with
   a rule for which file goes where?
2. Does the first video release include any video transcoding, or only remux
   plus audio-only transcoding, with full transcoding following later?
3. Should analysis jobs (intro detection, scrub-sheet fallback, speech sync)
   be allowed to use the sandboxed FFmpeg worker? This needs an ADR.
4. Are scrub previews on-demand by default, with sprite sheets as the
   fallback, or the other way round?
5. Which plugins ship first-party: OpenSubtitles, IntroDB or TheIntroDB,
   a trailer source? Who reviews their network grants?
6. Can watch-together guests be people with no account at all, and what can
   a guest capability reveal about the host's library (title, file name,
   other items)?
7. Are subtitle style, audio delay and quality settings stored per user, per
   device, or per user-and-device pair?
8. What is the default for profile 7 files: always convert to 8.1 for
   single-layer clients, or keep the base layer as HDR10?
9. How much of the statistics overlay should non-admin users see? File paths
   and server details may be sensitive.
10. Do trailers and pre-rolls belong in the first video release, or later?
11. What is the remote-quality default when the link cannot carry the
    original: ask every time, remember per device, or follow an owner rule?
12. Is machine-generated subtitling in scope at all, given the low-spec
    hardware goal? If so, does it run on the server, on a capable client,
    or only as a plugin?

## Sources

- https://support.plex.tv/articles/200250387-streaming-media-direct-play-and-direct-stream/ (via search summary; direct fetch was refused)
- https://support.plex.tv/articles/transcoder/ (via search summary)
- https://support.plex.tv/articles/credits-detection/ (via search summary)
- https://support.plex.tv/articles/202197528-video-preview-thumbnails/ (via search summary)
- https://support.plex.tv/articles/202920803-extras/ (via search summary)
- https://support.plex.tv/articles/subtitle-search/ (via search summary)
- https://support.plex.tv/articles/video-playback-speed-controls/ (via search summary)
- https://www.plex.tv/plex-pass/
- https://forums.plex.tv/t/new-lifetime-plex-pass-pricing/938910 (via search summary)
- https://www.howtogeek.com/plex-is-tripling-the-price-of-its-lifetime-passand-hinting-it-might-disappear-forever/ (via search summary)
- https://www.androidauthority.com/plex-remote-watch-pass-price-increase-3663060/ (via search summary)
- https://9to5mac.com/2025/03/19/plex-price-increase-remote-streaming-changes/ (via search summary)
- https://www.androidauthority.com/plex-watch-together-removed-3529939/
- https://forums.plex.tv/t/add-watch-together-to-new-plex-experience/906941
- https://forums.plex.tv/t/watch-together-going-away-in-app/906895
- https://forums.plex.tv/t/watch-together/938790
- https://forums.plex.tv/t/tandem-playback-to-several-clients/38777
- https://forums.plex.tv/t/allow-watch-together-for-tv-shows-mod-more-than-1-episode-at-a-time/644669 (post count from forum search)
- https://forums.plex.tv/search.json?q=watch%20together%20new%20experience
- https://forums.plex.tv/search.json?q=audio%20offset%20lip%20sync
- https://forums.plex.tv/search.json?q=ASS%20subtitles%20styling%20new%20app
- https://forums.plex.tv/t/feature-request-lip-synch-audio-offset/76923
- https://forums.plex.tv/t/audio-video-sync-audio-offset/840240
- https://forums.plex.tv/t/subtitle-offset/233997
- https://forums.plex.tv/t/better-subtitle-offset-experience/737870
- https://forums.plex.tv/t/fix-subtitle-offsets-mod-option-to-increase-step-size/375120
- https://forums.plex.tv/t/force-higher-remote-quality/195464
- https://forums.plex.tv/t/bug-plex-app-defaults-to-2mbps-720p/449537 (via search summary)
- https://forums.plex.tv/t/transcode-dolby-vision/935235
- https://forums.plex.tv/t/new-experience-public-release-feedback/910904
- https://forums.plex.tv/t/current-missing-features-bugs-with-the-ios-app-experience-preview/896263
- https://forums.plex.tv/t/rant-new-atv-app-no-thanks-i-want-the-old-one-back/943162
- https://forums.plex.tv/t/nvidia-shield-android-app-ssa-ass-format-anime-subtitles-transcoding-problems/446811
- https://forums.plex.tv/t/new-ios-plex-app-not-correctly-displaying-ass-subtitle-fonts-and-styling/914929 (post count from forum search)
- https://forums.plex.tv/t/new-option-burn-subtitles-avoid-at-all-costs/320968 (post count from forum search)
- https://forums.plex.tv/t/appletv-subtitles-are-blindingly-bright/840174 (title from forum search)
- https://forums.plex.tv/t/atmos-dobly-true-hd-playback-fails/786084 (title from forum search)
- https://forums.plex.tv/t/subtitle-issues-when-casting-from-android/922901 (title from forum search)
- https://forums.plex.tv/t/volume-boost-night-mode-industry-standard-av-fx-audio-and-subtitles-offset-settings/233703 (title from forum search)
- https://forums.plex.tv/t/bug-with-subtitles/932598 (via search summary)
- https://jellyfin.org/posts/jellyfin-release-12.0/
- https://jellyfin.org/posts/jellyfin-release-10.11.0/
- https://jellyfin.org/posts/jellyfin-release-10.10.0/
- https://jellyfin.org/posts/jellyfin-release-10.9.0/
- https://jellyfin.org/docs/general/clients/codec-support/
- https://jellyfin.org/docs/general/post-install/transcoding/
- https://jellyfin.org/docs/general/server/metadata/media-segments/
- https://raw.githubusercontent.com/jellyfin/jellyfin-web/master/package.json
- https://features.jellyfin.org/api/v1/posts?view=most-wanted&limit=60
- https://features.jellyfin.org/api/v1/posts?query=subtitle&limit=40
- https://features.jellyfin.org/api/v1/posts?query=playback&limit=40
- https://features.jellyfin.org/api/v1/posts?query=audio&limit=40
- https://features.jellyfin.org/api/v1/posts?query=skip&limit=30
- https://features.jellyfin.org/api/v1/posts?query=HDR&limit=30
- https://features.jellyfin.org/api/v1/posts?query=syncplay&limit=30
- https://features.jellyfin.org/api/v1/posts?query=next%20episode&limit=30
- https://features.jellyfin.org/api/v1/posts?query=transcod&limit=20
- https://features.jellyfin.org/api/v1/posts?query=trailer&limit=15
- https://features.jellyfin.org/posts/517
- https://github.com/jellyfin/jellyfin-ffmpeg/pull/774
- https://github.com/intro-skipper/intro-skipper
- https://jellywatch.app/blog/jellyfin-intro-skipper-setup-configuration-troubleshooting-2026 (via search summary)
- https://www.androidauthority.com/jellyfin-syncplay-explained-3530437/
- https://forum.jellyfin.org/t-android-tv-jellyfin-will-not-display-certain-pgs-subtitles?pid=40805 (via search summary)
- https://emby.media/support/articles/Premiere-Feature-Matrix.html
- https://emby.media/premiere.html
- https://emby.media/support/articles/Intro-Skip.html
- https://github.com/MediaBrowser/Emby.Releases/releases
- https://firecore.com/infuse
- https://firecore.com/releases
- https://community.firecore.com/t/dolby-vision-profile-7-8-support-ts-mkv-files/19713
- https://alternativeto.net/news/2026/3/infuse-8-4-introduces-extras-intro-skipping-home-screen-favorites-and-improved-playback
- https://introdb.app
- https://github.com/nekno/DV7toDV8 (via search summary)
- https://github.com/quietvoid/dovi_tool
- https://github.com/xbmc/xbmc/releases
- https://9to5linux.com/kodi-22-beta-improves-linux-support-for-remote-keys-and-hdr-profile-support
- https://alternativeto.net/news/2026/9/kodi-22-piers-beta-2-adds-hdr-screenshots-and-game-tools/
- https://mpv.io/manual/stable/
- https://github.com/mpv-player/mpv/discussions/17158
- https://www.phoronix.com/news/MPV-0.41-Released (via search summary)
- https://github.com/po5/thumbfast
- https://github.com/kaegi/alass
- https://github.com/smacke/ffsubsync
- https://www.macrumors.com/2026/03/25/tvos-26-4-features-and-release-notes/
- https://support.apple.com/en-us/102277
- https://www.techradar.com/televisions/apple-tv-4k-to-get-ai-enhanced-subtitles-amazon-x-ray-style-info-while-watching-and-clearer-dialogue-in-tvos-18 (via search summary)
- https://9to5mac.com/2026/04/03/tvos-26-recently-added-two-apple-tv-4k-features-ive-been-loving/
- https://www.ibc.org/ott-streaming/news/netflix-launches-dialogue-only-subtitles/21853
- https://www.technipages.com/netflix-how-to-change-video-playback-speed
- https://www.soapcentral.com/shows/what-netflix-s-moments-feature-details-new-bookmarking-scene-sharing-facility-explored (via search summary)
