# Video implementation plan

Status: proposed, for the owner's review. This document sequences video
delivery in the order the owner asked for: movies and TV on demand in the
player that exists today, then M3U live TV, then tuners and recordings. It
does not replace the master plan: the R2 packages it cites are already
outlined in [work-packages.md](work-packages.md), and the R1 client packages
it builds on live in [client-packages.md](client-packages.md). The tables
that are new here — the client video packages (CP-1xx) and the live-TV
server packages (WP-3xx) — join those two documents when the owner accepts
this plan. Nothing below changes a feature row; where a row would need to
change, it is listed under [Decisions](#decisions-the-owner-must-make).

The order follows three things already decided:

- ADR 1's scope note: the first version covers movies and TV shows, and
  M3U with live TV is the first module after that.
- [live-tv.md](../features/live-tv.md) open decision 1: live TV does not
  start until R2's remuxer and sandbox are stable.
- The owner's web-player answer of 2026-10-03 (decision register): the
  client is built in parallel against a small fake server and moves to the
  real server as its waves land. The video client reuses that pattern, so
  the player work never waits on the server chain.

## Milestones

| Milestone | Ships | Feature anchor | Waves |
|---|---|---|---|
| **M1** | Films and episodes on demand: catalogue, scan, playback decision, delivery (direct play, remux, sandboxed transcode), the video player in the web client | [video.md](../features/video.md) R2 rows; [library.md](../features/library.md) LIB-002, LIB-149 onwards | 7-10 (WP-2xx, already outlined) and C4-C5 (CP-1xx, proposed here) |
| **M2** | M3U live TV: sources under network grants, guide, channels, live playback in the same player, module off by default | [live-tv.md](../features/live-tv.md) LIV-001, LIV-003 to LIV-017, LIV-073, LIV-079 to LIV-098 | 11-12 (WP-301 to WP-309, proposed) and C6 (CP-111 to CP-113) |
| **M3** | Tuners and recordings: HDHomeRun by address, the TVHeadend preset, recording rules, catch-up | LIV-024 to LIV-026, LIV-087, LIV-088, LIV-098, LIV-099, LIV-108, LIB-191 | 13-14 (WP-311 to WP-315) and CP-114 |

M2 starts only when M1's remuxer (WP-204, WP-213) and transcode sandbox
(WP-214) have passed the full gate on the wave branch and survived a real
library's playback, because live delivery reuses both.

## What the player already gives video

The player was built music-first but media-agnostic. Video extends these
seams; nothing here forks a second player.

| Seam | Today | What video does with it |
|---|---|---|
| `core/src/player.rs` | The state machine's event is "first audio or first frame"; states, not media kinds | Untouched in M1 and M2. Live TV needs no new state: the live edge is an action ("go to live"), not a state, and Buffering, Stalled and Stopped already cover a stream that dies |
| `core/src/queue/document.rs` | `Context::Music` only; the doc comment reserves video (VID-181) and spoken word (LAT-009) | `Context` gains `Video` in M1 and `Live` in M2; lanes, verbs, rebase and the operation tests extend per variant |
| `core/src/decision.rs` | `decide_audio`, `PlayCap {None, MseOnly, Native}`, `PackageFormat::FragmentedMp4` | WP-208's `decide_video` reuses `PlayCap` and the packager precedent: Direct, Remux, Audio-only conversion, Transcode, Refused, each with typed reasons (VID-002, SEC-TM-040) |
| `core/src/ebml.rs` | The EBML reader the music scan uses | WP-201 moves it under `core/src/formats/mkv/` and grows segments, tracks, cues, chapters and attachments |
| `core/src/probe/` | MP4 and MPEG probes for audio | The video scan (WP-217) extends them into the stream index: tracks, codecs, bitrates, HDR, rotation, captured at scan, never probed at play time (VID-001) |
| Video catalogue | Does not exist | WP-207's `core/src/video/`: films, shows, seasons, episodes, versions, naming rules (LIB-002, LIB-149 to LIB-155) |
| `ports/src/provisional/player.ts` | Provisional display types under the CP-005 rule | Video display fields stay provisional until WP-236 regenerates declarations from the core, then the provisional mirrors shrink |
| `clients/apps/demo/src/demo-audio.ts` + `src/browser/audio-element.ts` | One engine, one audio-element adapter, replaced by CP-019 and WP-030 | The real engine keeps the adapter shape and gains a video-element adapter beside the audio one; the engine chooses per item from the decision, and Media Source Extensions carry remuxed fMP4 exactly as the audio packager does today |
| `clients/packages/ui/src/shell/PlayerBar.tsx`, `PlayerFull.tsx` | Music presentation | Video takes the screen: SUR-046 hides the navigation and the bar; the bar keeps the paused music context one tap away (MUS-131), with listen-only (VID-065) as the only overlap |

This is the same one playback model [player.md](../ui/player.md) commits
to: one queue, one decision, one player state, viewed by every surface.

## M1: films and episodes on demand

### Server and core chain

Already outlined as WP-201 to WP-234 in
[work-packages.md](work-packages.md). The order that unblocks a playing
screen soonest:

1. WP-207 (video catalogue model) beside WP-201 and WP-202 (Matroska and
   MP4 video), wave 7.
2. WP-203 (segment map) then WP-204 (the remuxer), wave 8, with WP-208
   (the video decision engine) in parallel off the parsers.
3. WP-213 (remux worker, segment serving) in wave 9: the first real
   playback through signed, session-bound URLs (VID-011, SEC-API-026 to
   SEC-API-029). WP-214 (the sandbox and its FFmpeg allowlist) lands in
   wave 8 so browsers without a codec still play.
4. WP-217 (the video scan) fills the catalogue; WP-216 (single keyframe)
   feeds scrub previews; WP-232 (resume, Continue Watching, Next Up)
   closes the loop with the user log.

Everything else in the R2 outline — downloads (WP-220), handoff and cast
(WP-219), iroh (WP-221), household policies (WP-222), importers (WP-229),
markers (WP-230) — trails behind first playback in the outline's own
waves.

### Client packages

Proposed, numbered from CP-101 so R1's CP-001 to CP-062 can grow:

| ID | Title | Wave | Size | Depends on | Owns (outline) | Serves | Security |
|---|---|---|---|---|---|---|---|
| CP-101 | The video engine: per-kind element adapters, the decision preview, the MSE remux path | C4 | L | CP-019, WP-030, WP-236's regenerated types | `clients/packages/player/` | VID-001 to VID-003, VID-015 | SEC-CLI-015 |
| CP-102 | The Watch home and the video catalogue against the fake server | C4 | M | CP-101 | `clients/apps/web/src/` video destinations | API-VID-01, DIS rows for Watch | None specific |
| CP-103 | Title, show, season, episode and collection pages | C5 | L | WP-207's API, CP-102 | The Watching destinations | SUR-040 to SUR-043, VID-013, LIB-149 onwards | SEC-API-015 |
| CP-104 | Pre-play sheet, the video player, player sheets, the info overlay, cards | C5 | L | CP-101, WP-213 | The player surfaces | SUR-045 to SUR-049, VID-168 | SEC-API-026 to SEC-API-029 |
| CP-105 | Resume, Continue Watching, Next Up, autoplay and the countdown card | C5 | M | WP-232, CP-104 | Progress destinations | VID-118 to VID-127 | SEC-PRV-028 |
| CP-106 | Subtitles in the web player, through the browser's text-track pipeline | C5 | M | WP-205, CP-104 | Subtitle rendering | VID-069 to VID-093 | SEC-API-089, SEC-CLI-053 |

Wave C4 runs against the fake server, like C2 did; C5 moves to the real
server as WP-213 and WP-217 land. Every surface follows the coexistence
rules [surfaces.md](../ui/surfaces.md) already fixes: separate Music and
Watch homes, one queue with separate contexts, and the video player taking
the whole screen.

## M2: M3U live TV

The module ships off by default (LIV-001): a video-only household that
never adds a source pays nothing. Every host a source may contact is a
network grant under record 2, added by an admin, with LAN grants reserved
to the owner behind a fresh passkey check (LIV-015, LIV-180).

### Server packages

Proposed, numbered from WP-301, waves 11-12:

| ID | Title | Wave | Size | Depends on | Owns (outline) | Serves | Security |
|---|---|---|---|---|---|---|---|
| WP-301 | The Live TV module flag: routes, jobs and screens gated on it | 11 | S | WP-043's app registry | `server/src/live/` | LIV-001 | SEC-TM-005, SEC-TM-074 |
| WP-302 | Source records, the guarded fetcher's source purpose, the upload route, grant review and revocation | 11 | M | WP-048, WP-301 | `server/src/live/sources/` | LIV-003, LIV-014, LIV-015, LIV-180 | SEC-TM-048, SEC-NET-067, SEC-HIS-025, SEC-EXT-050 |
| WP-303 | The M3U parser as a worker job, with attributes, import filters and scheduled refresh | 11 | L | WP-302, WP-061, WP-078 | `worker/src/jobs/m3u.rs` | LIV-003, LIV-006, LIV-008 to LIV-010 | SEC-MED-078, SEC-TM-031, SEC-TM-032 |
| WP-304 | Channels and the lineup: merge across sources, diffs that never delete, favourites | 12 | M | WP-303 | `server/src/live/lineup/` | LIV-011, LIV-060, LIV-061, LIV-055 | SEC-TM-024 |
| WP-305 | The XMLTV guide parser (worker) and guide matching, DOCTYPE refused with its reason | 12 | L | WP-302 | `worker/src/jobs/xmltv.rs` | LIV-035 and the guide rows | SEC-MED-056, SEC-HIS-034 |
| WP-306 | The MPEG-TS demuxer in the core: program tables, clock references, continuity counters, bounded resynchronisation | 12 | L | WP-201's parser budgets | `core/src/formats/ts/` | LIV-004 | SEC-MED-079, SEC-MED-078 |
| WP-307 | The live HLS client in the core: playlist reloads, discontinuities, AES-128 segments, variant choice | 12 | L | WP-306 | `core/src/live/hls/` | LIV-005 | SEC-EXT-075, SEC-API-078 |
| WP-308 | The broker: stream allocation, mirror and login failover, tuner priorities, the per-channel fan-out buffer | 12 | L | WP-304, WP-306, WP-307 | `server/src/live/broker/` | LIV-012, LIV-013, LIV-029, LIV-030 | SEC-TM-068, SEC-TM-071 |
| WP-309 | Live delivery: routes, live decision variants, stats fields, the capacity warning | 12 | M | WP-308, WP-213's serving shape | `server/src/live/stream/` | LIV-079, LIV-086 | SEC-API-078, SEC-API-079 |

WP-306 and WP-307 are new parser classes under the mutation gate; they get
the same treatment as WP-201 and WP-202: property tests, committed
regressions, hostile-input corpora, and benchmarks at playlist sizes
LIV-009 names.

### Client packages

Wave C6, small, because the player already exists:

| ID | Title | Wave | Size | Depends on | Owns (outline) | Serves | Security |
|---|---|---|---|---|---|---|---|
| CP-111 | Live presentation in the video player: channel up, down, last and number entry; the live-edge timeline with start over and go to live; the "Live" label where a duration would be | C6 | M | CP-104, WP-309 | The player's live mode | LIV-073, LIV-081, LIV-095 to LIV-098 | None specific |
| CP-112 | The Live home and the guide: SUR-120, the mini guide over the picture, favourites | C6 | M | WP-304's API, CP-111 | The Live destinations | SUR-120, LIV-082, LIV-055 | SEC-PRV-022 |
| CP-113 | Radio stations from the same ingest in the music player, with the now-playing bar and lock-screen controls | C6 | S | WP-309, CP-019 | The music player's radio mode | MUS-177, LIV-168 | SEC-CLI-067 |

Two honesty rules carry over unchanged: live TV in the browser is
best-effort with the server's capacity warning shown plainly (LIV-079,
open decision 6), and internet sources are HTTPS-only while plain HTTP
stays a granted LAN exception that carries no credential (LIV-004, open
decision 17).

## M3: tuners and recordings

### Server packages

Waves 13-14, still WP-3xx:

| ID | Title | Wave | Size | Depends on | Owns (outline) | Serves | Security |
|---|---|---|---|---|---|---|---|
| WP-311 | HDHomeRun by address: channel scan, tuner status, stream counting; discovery stays out until the egress and socket inventories name it | 13 | M | WP-308 | `server/src/live/tuners/` | LIV-024, LIV-025 | SEC-TM-017, SEC-API-079 |
| WP-312 | The TVHeadend preset over M3U and XMLTV, credentials as integration secrets, never on plain HTTP | 13 | M | WP-302, WP-305 | The source preset | LIV-026 | SEC-EXT-050, SEC-NET-001 |
| WP-313 | Recording rules, the recorder, the growing-file index, the recordings root with its own quota | 13 | L | WP-308, WP-311 | `server/src/live/dvr/` | LIV-088, LIV-098, LIV-108, LIB-191 | SEC-OPS-063, SEC-TM-042 |
| WP-314 | Catch-up: provider replay templates, past-guide retention, "Play from start" | 14 | M | WP-313 | The DVR's catch-up | LIV-099 | SEC-TM-071 |
| WP-315 | Guide providers as first-party plugins (Schedules Direct) under record 2's grants | 14 | M | WP-225, WP-305 | The guide plugin | Open decision 5 of live-tv.md | SEC-EXT-027, SEC-OPS-017 |

### Client

CP-114, wave C6 or C7: the record button that keeps what was already
watched (LIV-088), the tuner-busy sheet (LIV-087), the recordings pages and
catch-up's "Play from start". Recordings join the library as normal
episodes and films (LIB-191), so playback reuses M1 whole.

## Deliberately not in this plan

The feature maps already refuse these, and this plan inherits every refusal
rather than reopening it:

- Stalker portals (LIV-021), redirect mode (LIV-022) and Xtream VOD
  catalogues (LIV-023): identity imitation, credential leaks and scope
  creep.
- HDHomeRun emulation (LIV-172): refused by the owner (A-055); Gunmetal
  speaks to real tuners instead of imitating one for Plex-style apps.
- Direct USB and PCIe DVB drivers (LIV-032): kernel work stays with
  TVHeadend (LIV-026).
- Disc menus (VID-021) and full disc-image parsing in the first cut
  (VID-020 is Later).

## Decisions the owner must make

Each names the open-decision list that owns it; the recommendations are
those lists', restated so this plan is complete on its own.

Before M1:

1. **The R2 cut line** (video.md, open decision 14). Recommendation: the
   must-ship set is the decision engine with reasons, direct play, remux,
   the sandbox, signed URLs, subtitles, track selection, resume and
   autoplay, chapter markers with the skip policy, the info overlay and
   accessible controls; everything else trails in point releases.
2. **Transcoding on by default?** (video.md, open decision 1).
   Recommendation: ship it in the sandbox, off by default for everyone but
   the owner.
3. **The desktop shell** (video.md, open decision 17). Recommendation:
   Later, as the security baseline's release-scope table says.
4. **The first video client is the web client** (this plan's proposal,
   following the 2026-10-03 web-player answer). Native Android and TV
   clients follow on the same core through WP-231's UniFFI bindings, and
   their libmpv hardening (SEC-CLI-047, SEC-CLI-048) is theirs, not the
   web player's.

Before M2 (all from live-tv.md's open decisions):

5. **Live TV after a stable R2** (decision 1). This plan encodes it as the
   M2 entry gate.
6. **M3U only at first; Xtream decided after R3** (decision 2).
7. **The module off by default** (decision 15).
8. **The XMLTV DOCTYPE rule** (decision 16): refuse, or allow skipping a
   DOCTYPE with no internal subset; the security lead decides.
9. **The plain-HTTP reconciliation** (decision 17) between SEC-API-078 and
   the requirements that accept http source URLs.
10. **Content-neutral wording** in docs and screens (decision 3).

Before M3 (live-tv.md's open decisions again):

11. **Grants before discovery** (decision 5): tuners by address first;
    discovery only once the threat model's inventories name it.
12. **ATSC guide parsing in R3** (decision 7); DVB events stay with
    TVHeadend's export.
13. **Recordings are transport streams** (decision 10), with Matroska
    repackaging only after a round-trip corpus passes.
14. **Wake recipes, not a privileged helper** (decision 14).

## Ground rules every package inherits

- The testing rules in [CONTRIBUTING.md](../../CONTRIBUTING.md) are
  mandatory: red then green, deep assertions, 100% coverage, zero
  surviving mutants. Parsers (M3U, XMLTV, MPEG-TS, live HLS) also get
  property tests with committed `proptest-regressions`, and the worker
  jobs get crash-and-hang tests against hostile playlists and guides (the
  A-578 shape).
- Every security row cited above needs its `Verifies:` test (SEC-STD-004)
  before its package is done, and SEC-CLI-051's sanitizer playback run
  starts with the first shipped player build.
- New crates and dependencies follow the dependency checklist and the
  allowlist steps (SEC-SUP-024, SEC-SUP-025); processes start only through
  WP-045's command builder, and the worker's closed program list grows one
  reviewed variant at a time.
- Waves, branches and merging follow CONTRIBUTING's "Working in
  parallel": packages branch from their wave, the gate passes before the
  integrator merges, and agents never merge into `main`.
- On acceptance, the CP-1xx tables join
  [client-packages.md](client-packages.md) and the WP-3xx tables join
  [work-packages.md](work-packages.md)'s R3 outline; this file then keeps
  the order of delivery and the seams.

## Risks

- **The remuxer remains the schedule risk** (ADR 1's own consequence).
  The client chain is ordered so CP-101 and CP-102 prove the player
  against the fake server before WP-213 lands, the same way the R1 web
  player was built.
- **Browser codecs decide how much M1 needs the sandbox.** HEVC and
  AC-3/E-AC-3 support is uneven across browsers, so the web client leans
  on remux and audio-only conversion first; full transcode is the last
  resort, which is exactly the decision engine's order.
- **Live parsing is a new class of core parser.** WP-306 and WP-307 are
  budgeted like WP-201 and WP-202, not smaller, because the mutation gate
  costs the same per line.
- **Grants add friction to IPTV sources**: a CDN host behind a playlist
  waits for an admin's approval (LIV-005), so the source screens must
  show pending hosts plainly rather than failing silently.
- **Sandbox tiers differ by host.** R1 servers are Linux (D-09);
  development machines run the reduced tier, and the full tiers are
  proven on the project's own runners, so a passing local gate never
  claims isolation the host does not provide (SEC-MED-024).
