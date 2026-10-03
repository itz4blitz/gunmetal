# Interface documents

This folder describes Gunmetal's interface: what screens exist, how they
look, how the player behaves, and the journeys that tie them together. It
was written on 2026-10-02. Everything here sits under the
[feature map](../features/README.md), which is the source of truth for what
each feature does and when it ships, and under the two accepted
architecture records ([ADR 1](../adr/0001-architecture.md) and
[ADR 2](../adr/0002-music-is-first-class.md)). Where a document in this
folder disagrees with the feature map, the map wins. Choices that go beyond
the map are marked **Proposal** inside each document.

| Document | What it covers | Read it for |
|---|---|---|
| [surfaces.md](surfaces.md) | Every screen, panel, sheet and persistent control (SUR-000 to SUR-125), the navigation model per form factor, and a coverage check that every R1 and R2 feature has a home | The screen list, and which feature rows each screen serves |
| [player.md](player.md) | The now-playing bar, full-screen player, queue, lyrics, sound path, handoff, video controls, live TV in the player, every player state, and the keyboard, remote, touch and OS media mappings | How playback looks and behaves, and what the player needs from the server and the core |
| [design-language.md](design-language.md) | Principles, colour tokens for five themes, artwork-derived tints, typography, spacing and density, focus on keyboard and TV, motion, icons and badges, empty and error states, accessibility requirements | The tokens and rules every screen is built from |
| [flows.md](flows.md) | Twenty step-by-step journeys (F01 to F20), from first-run setup to live TV, each with what the person does, the screen, what the server does, and what goes wrong; plus seventeen gaps the map does not settle (G1 to G17) | The order a person meets screens, and the server behaviour behind each step |

The server work these documents imply is collected in
[docs/plan/api-needs.md](../plan/api-needs.md).

## Navigation model in one paragraph

The same destinations exist on every form factor and only their placement
changes. R1 (a browser-only music release) has three destinations, Home,
Search and Library, plus an account menu that leads to Settings, Account,
History, Hidden and, for administrators, Admin. On web and desktop the
frame is a sidebar, a content area, a right pane that holds the queue,
lyrics or track info at full height, and a now-playing bar along the
bottom; it collapses through four width classes (compact, medium, expanded,
wide) to a phone layout with a bottom tab bar, the bar above it, a
full-screen player sheet and a queue sheet. Every list and search answers
from the library synced to the device, Back returns to the exact scroll
position, and one context menu offers the same actions everywhere. A
layout contract, pinned by visual regression tests, keeps the bar, queue
access, lyrics, scrubber, device control and TV rail in fixed places, so
R2 and R3 add things without moving anything: a Music and Watch switch at
the top of Home and Library (absent in music-only mode), Downloads inside
Library, a TV layout built around a left rail one press from anywhere, an
Android tablet layout with a player side panel, and a Live destination
that appears only when the live TV module is on.

## The player in one paragraph

The player is one model, owned by the shared Rust core, viewed through
many surfaces: a versioned, per-profile queue with three lanes ("Up next"
for the person's own picks, "From" for the album or playlist, and an
optional "Continue with" lane of labelled suggestions that is off by
default), the core's playback decision with a structured reason, and a
player state machine. In R1 the browser plays the original file, gaplessly
and with loudness levelling, shows a quality badge that tells the truth,
dims tracks the browser cannot decode with the reason, keeps lyrics from
the files, offers private listening and a sleep timer, and works with the
browser's media controls. Opening the client elsewhere offers "Continue on
this device". R2 adds native players with background audio, a device
picker with handoff, remote control and casting, the video player (a
statistics overlay, client-rendered subtitles, skip markers, a pre-play
sheet that opens only when there is a real choice), and music on the TV.
R3 adds live TV and internet radio inside the same two players. Every
control is reachable by keyboard, remote and screen reader, and nothing at
play time is paywalled.

## Contradictions between the four documents

These are places where the documents in this folder disagree with each
other. They are listed as found; none has been resolved here, and each
needs one decision that the losing document then follows.

1. **Leaving the video player.** surfaces.md (How music and video coexist)
   says starting a film pauses the music context, and closing the video
   player returns the now-playing bar to the music context with one tap to
   resume. player.md (Picture-in-picture, background audio and
   listen-only, and open question 10) says leaving the video player on web
   and desktop pauses the film and the bar shows the film with a resume
   control. The bar cannot show both.
2. **Undo for queue removals in R1.** design-language.md (section 11,
   rule 6) says reversible actions, naming "remove from queue", act at once
   and offer Undo, and that only irreversible actions confirm. player.md
   (The queue) and surfaces.md (SUR-003, SUR-011) put queue undo (MUS-121)
   in R2 and have "Clear queue" ask for confirmation in R1.
3. **Swipe to remove a queue row.** player.md (Touch) lists "swipe a queue
   row to remove it" as R1 under MUS-119. surfaces.md (SUR-004, SUR-011)
   makes swipe actions on rows R2 under MUS-065.
4. **The TV rail's Now Playing entry.** design-language.md (section 8,
   rule 6, and the density table) makes Now Playing the rail's fixed first
   entry. surfaces.md (Navigation model, TV) puts it first "whenever
   something is playing", which would shift every other rail entry when
   playback starts or stops; that conflicts with the layout contract both
   documents rely on.
5. **Back from TV Now Playing.** player.md (Music on a TV, TV remote) says
   Back returns to the TV music home with playback continuing.
   design-language.md (section 8, rule 7) says Back always goes up exactly
   one level, from content to the rail. surfaces.md (What is always on
   screen) says only "Back returns", and DIS-112 promises the exact
   previous place.
6. **A keyboard shortcut for search in R1.** surfaces.md (SUR-032) gives
   the web search field "a keyboard shortcut" in R1. player.md (Input
   mappings) puts every shortcut, including "/" for search, in R2, and
   asks in open question 1 whether a few player shortcuts should come
   forward.
7. **Where the sleep timer lives.** surfaces.md (SUR-011) puts "set a sleep
   timer from the queue menu" in R1. player.md lists the queue menu's
   contents without it and places the sleep timer in the full player's
   options menu.
8. **A device control in the R1 bar.** player.md (The persistent
   now-playing bar, open question 2) proposes an R1 device slot reading
   "Playing on *device*" with "Play here". surfaces.md (SUR-002) makes the
   device button R2 and gives R1 only the "Continue on this device" prompt.
9. **Two players on one queue in R1.** player.md (open question 2)
   proposes an active-device field on the queue. flows.md (G11) proposes
   that the tab which pressed Play last takes the queue and that this
   "needs only the R1 queue versions". The outcome is the same; the
   mechanism differs, and the server design needs one.
10. **"Up next" means two things.** In the music queue it is the lane for
    the person's own picks (all four documents). For video, surfaces.md
    (SUR-046) and player.md (On-screen controls) also call the queue panel
    and the autoplay surface "Up next" (VID-181), whose first lane is the
    picks and whose second is the season. With DIS-025's "Next Up" row as
    well, the same words name three things.
11. **The quality badge in the phone bar.** player.md shows it in the
    phone-width bar only when playback is not the original played
    directly. surfaces.md (SUR-002) lists the badge on the bar without a
    phone exception, and design-language.md shows the compact "Original"
    badge as the normal case.
12. **Copy for missing lyrics.** player.md uses "No lyrics in this file";
    design-language.md uses "This file has no lyrics." Small, but the
    copy should be one string.

Points that look like contradictions but are not: the flows name screens
with the feature map's surface names ("Admin > Libraries") rather than
surfaces.md's IDs (SUR-085), and surfaces.md and player.md both mark TV
lyrics and the TV now-playing entry as proposals because no feature row
defines them yet (surfaces.md open question 4).
