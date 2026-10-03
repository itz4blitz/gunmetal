# Interface documents

This folder describes Gunmetal's interface: what screens exist, how they
look, how the player behaves, and the journeys that tie them together. It
was written on 2026-10-02. Everything here sits under the
[feature map](../features/README.md), which is the source of truth for what
each feature does and when it ships, and under the two accepted
architecture records ([ADR 1](../adr/0001-architecture.md) and
[ADR 2](../adr/0002-music-is-first-class.md)). Where a document in this
folder disagrees with the feature map, the map wins. The
[security baseline](../security/README.md) outranks both: where a surface,
a flow or a design rule would break a security requirement, the document
follows the requirement, cites its ID and lists the change in its own
"Changes made to follow the security baseline" section (or, in
player.md and design-language.md, in the text and open questions). Choices
that go beyond the map are marked **Proposal** inside each document.

| Document | What it covers | Read it for |
|---|---|---|
| [surfaces.md](surfaces.md) | Every screen, panel, sheet and persistent control (SUR-000 to SUR-133), the navigation model per form factor, and a coverage check that every R1 and R2 feature has a home | The screen list, and which feature rows each screen serves |
| [player.md](player.md) | The now-playing bar, full-screen player, queue, lyrics, sound path, handoff, video controls, live TV in the player, every player state, and the keyboard, remote, touch and OS media mappings | How playback looks and behaves, and what the player needs from the server and the core |
| [design-language.md](design-language.md) | Principles, colour tokens for four themes (with a System setting that follows the operating system), artwork-derived tints, typography, spacing and density, focus on keyboard and TV, motion, icons and badges, empty, error and security states, accessibility requirements | The tokens and rules every screen is built from |
| [flows.md](flows.md) | Twenty step-by-step journeys (F01 to F20), from first-run setup to live TV, each with what the person does, the screen, what the server does, and what goes wrong; plus twenty gaps the map does not settle (G1 to G20), several now settled by the baseline | The order a person meets screens, and the server behaviour behind each step |

The server work these documents imply is collected in
[docs/plan/api-needs.md](../plan/api-needs.md).

## Navigation model in one paragraph

The same destinations exist on every form factor and only their placement
changes. R1 (a browser-only music release) has three destinations, Home,
Search and Library, plus an account menu that leads to Settings, Account
(with sessions and devices, security events, what admins can see, and
recovery), History, Hidden and, for administrators on a personal device,
Admin. On web and desktop the frame is a sidebar, a content area, a right
pane that holds the queue, lyrics or track info at full height, and a
now-playing bar along the bottom; it collapses through four width classes
(compact, medium, expanded, wide) to a phone layout with a bottom tab bar,
the bar above it, a full-screen player sheet and a queue sheet. Every list
and search answers from the library synced to the device, Back returns to
the exact place the person came from, and one context menu offers the same
actions everywhere. A layout contract, pinned by visual regression tests,
keeps the bar, queue access, lyrics, scrubber, device control and TV rail
in fixed places, so R2 and R3 add things without moving anything: a Music
and Watch switch at the top of Home and Library (absent in music-only
mode), Downloads inside Library on native apps, a TV layout built around a
left rail one press from anywhere whose first entry is always Now Playing,
an Android tablet layout with a player side panel, and a Live destination
that appears only when the live TV module is on. The desktop shell, with
its mini player, is Later; until then desktops use the web client.

## The player in one paragraph

The player is one model, owned by the shared Rust core, viewed through
many surfaces: a versioned, per-profile queue with three lanes ("Up next"
for the person's own picks, "From" for the album or playlist, and an
optional "Continue with" lane of labelled suggestions that is off by
default), the core's playback decision with a structured reason, and a
player state machine. In R1 the browser plays the original file, gaplessly
and with loudness levelling, shows a quality badge that tells the truth,
dims tracks the browser cannot decode with the reason, keeps lyrics from
the files, offers private listening and a sleep timer, explains stream
limits, and works with the browser's media controls. Opening the client
elsewhere offers "Continue on this device", and when two of a profile's
devices press Play, the last one wins. R2 adds native players with
background audio, a device picker with handoff, remote control and
casting, the video player (a statistics overlay, client-rendered
subtitles, skip markers, a pre-play sheet that opens only when there is a
real choice), and music on the TV. R3 adds live TV and internet radio
inside the same two players. Every control is reachable by keyboard,
remote and screen reader, and nothing at play time is paywalled.

## Decisions that made the four documents consistent

On 2026-10-02 the four documents were checked against each other after
they had been changed to follow the security baseline. Each contradiction
was settled by one decision, chosen as the safer and simpler option and the
one closer to the feature map, and applied in every document. Decisions 1
to 12 settle the contradictions the first review listed, in its order.
Decisions 13 to 23 settle contradictions that the security edits
introduced or that the second check found. A decision marked **owner to
confirm** is a product choice the owner may still reverse; the others are
corrections that follow the feature map or the baseline.

1. **Leaving the video player** (owner to confirm). surfaces.md returned
   the bar to the paused music context; player.md kept the paused film in
   the bar on web and desktop. *Decided:* surfaces.md's model, on every
   client. Starting a film pauses the music context (LAT-009); leaving the
   video player pauses the film and saves its resume point (VID-118)
   unless picture-in-picture or listen-only is on; the bar returns to the
   paused music context, one tap from resuming, and shows a video only in
   listen-only mode (VID-065). The film resumes from its title page,
   Continue Watching or the queue switcher (MUS-131). This keeps the bar a
   music control, which LAT-039 ("a book never hijacks the music queue")
   will also need. Applied in player.md (the bar, picture-in-picture, TV
   remote, open question 10) and surfaces.md (How music and video coexist,
   SUR-002).
2. **Undo for queue removals in R1.** design-language.md listed "remove
   from queue" among the actions that offer Undo; player.md and
   surfaces.md put queue undo in R2. *Decided:* the feature map places
   queue undo in R2 (MUS-121). In R1, removing one row acts at once with no
   prompt, and "Clear Up next", "Clear queue" and removing a multi-selection
   confirm first and say how many items will go; from R2 they all act at
   once and offer Undo. Applied in design-language.md (section 11, rule 6),
   player.md (The queue) and surfaces.md (SUR-011).
3. **Swipe to remove a queue row.** player.md put it in R1 under MUS-119;
   surfaces.md in R2 under MUS-065. *Decided:* R2, with swipe actions on
   rows (MUS-065), which the feature map places in R2. In R1 a row is
   removed with its remove button, always shown on touch screens, or from
   its menu. Applied in player.md (The queue, Touch).
4. **The TV rail's Now Playing entry.** design-language.md made it the
   fixed first entry; surfaces.md showed it only while something played,
   which would shift every other entry. *Decided:* always the first entry,
   whether or not anything plays; with nothing queued it opens the
   empty-queue state. No rail entry moves (CLI-035, the layout contract).
   Applied in surfaces.md (Navigation model, TV; What is always on screen;
   SUR-002; SUR-010; open question 4), design-language.md (section 8,
   rule 6) and player.md (Music on a TV).
5. **Back from TV Now Playing.** player.md went to the TV music home;
   design-language.md went "up one level"; surfaces.md said only "Back
   returns"; DIS-112 promises the exact previous place. *Decided:* Back
   goes back one step, to where the person came from, with the same row
   and card focused (DIS-112, CLI-036): from TV Now Playing or any other
   screen opened from another, to that screen; from a top-level screen's
   content, to the rail; from the rail, to Home; and from Home it asks
   before leaving. Music keeps playing. Applied in design-language.md
   (section 8, rule 7), player.md (Music on a TV, TV remote) and
   surfaces.md (Navigation model, TV; What is always on screen; SUR-010).
6. **A keyboard shortcut for search in R1.** surfaces.md gave the web
   search field one in R1; player.md put every shortcut in R2.
   *Decided:* no shortcut in R1. The search shortcut arrives in R2 with
   DIS-090 and CLI-061, which the feature map places there; in R1 the
   field is reached by Tab like any control. player.md's open question 1,
   on bringing a few player shortcuts into R1, stays open for the owner.
   Applied in surfaces.md (Web and desktop, SUR-032) and player.md (open
   question 1).
7. **Where the sleep timer lives.** surfaces.md put it in the queue menu;
   player.md only in the player's options menu. *Decided:* both, as one
   action from the one action list, because MUS-076 names the player menu
   and the queue menu. Applied in player.md (the queue menu) and
   surfaces.md (SUR-011).
8. **A device control in the R1 bar** (owner to confirm). player.md
   proposed an R1 slot reading "Playing on *device*" with "Play here";
   surfaces.md gave R1 only the "Continue on this device" prompt.
   *Decided:* the bar's device slot exists from R1 and keeps its place, but
   in R1 it holds only the CLI-103 prompt, "Continue on this device" with
   the item and position and no device name, when another of the profile's
   sessions last played the queue; otherwise it is empty. The Devices
   button fills the same slot from R2 (CLI-101, MUS-197). CLI-103 is the R1
   row, and its prompt needs no device name. Applied in player.md (the bar,
   the full player, the layout contract, device handoff, states) and
   surfaces.md (SUR-002).
9. **Two players on one queue in R1** (owner to confirm). player.md
   proposed an active-device field; flows.md (G11) proposed that the last
   tab to press Play takes the queue. *Decided:* flows.md's mechanism. Play
   and resume are ordinary operations on the versioned queue (MUS-122),
   which records the issuing session by an opaque identifier that is not a
   credential; the last Play wins, and any other session that sees this on
   its next sync pauses at its point and shows the decision 8 prompt. There
   is no separate active-device record. Only the profile's own sessions
   can write its queue, and queue events reach only them (SEC-HIS-014,
   SEC-API-016). Applied in player.md (the bar, device handoff, states,
   what the player needs, open question 2) and flows.md (F05, F08, G11).
10. **"Up next" means one thing** (owner to confirm). The queue lane, the
    video queue panel and autoplay surface (VID-181) and the Next Up row
    (DIS-025) shared the words. *Decided:* on screen, "Up next" names only
    the lane of a person's own picks. The video queue panel, which the
    feature map calls "Player > Up next", is labelled "Queue", like the
    music queue, with the Up next and From lanes; the end-of-episode
    surfaces are the countdown card and the post-play screen. The Home row
    fed by DIS-025 and DIS-031 is labelled "Next to watch", and the
    collection page offers "Watch the next film" (DIS-031). Feature IDs and
    the map's own names are unchanged. Applied in player.md (on-screen
    controls; "Next episode, autoplay and the post-play screen"),
    surfaces.md (SUR-020, SUR-042, SUR-046, SUR-073, the Later podcast row)
    and flows.md (F17).
11. **The quality badge in the bar** (owner to confirm). player.md showed
    it at phone width only when playback was not the original played
    directly, and used "FLAC 24/96" as the compact form; surfaces.md and
    design-language.md showed the compact "Original" badge as the normal
    case. *Decided:* the compact quality badge shows in the bar on every
    layout, phone width included, because MUS-099 names the bar and one
    rule is simpler: "Original" in brass when the original plays directly,
    "Converted" from R2 otherwise, with the full sentence on hover, focus
    or tap. "FLAC 24/96" is the separate format badge for rows, tiles and
    the track info sheet. Applied in player.md (the bar, the quality badge
    table), surfaces.md (SUR-002) and design-language.md (Badges).
12. **One string per message.** player.md and design-language.md worded
    the same messages differently. *Decided:* design-language.md's copy,
    which owns the tone: "This file has no lyrics." and "Can't play in this
    browser: ALAC". Applied in player.md (Lyrics, the quality badge table,
    States).
13. **The desktop shell's release** (owner to confirm). After the security
    edits, player.md and design-language.md placed the shell in Later, but
    surfaces.md and flows.md still had the shell, its mini player
    (SUR-052), desktop media panels, global hotkeys, installers, downloads
    and exclusive output in R2, and F08 handed playback to "the desktop".
    *Decided:* Later everywhere, as the baseline's release scope says
    (SEC-TM-074) and the shell's own controls require (SEC-CLI-069, Later);
    the feature map now agrees. SUR-052 keeps its ID with release Later;
    Serves lines list CLI-014, CLI-063, CLI-064, CLI-065, MUS-081, MUS-082
    and MUS-102 under Later; desktops use the web client until the shell
    ships; and F08 hands playback to the web client on a laptop. Applied in
    all four documents.
14. **The public share page's release** (owner to confirm, security
    decision 7). player.md said surfaces.md still placed SUR-059 in R2;
    surfaces.md already had it in R1. *Decided:* R1 for music in both
    (SEC-API-097, SEC-PRV-031). Applied in player.md (Listening through a
    share link).
15. **Private listening from the wide bar.** player.md gave the wide bar an
    Options button with Private session first, so private listening is two
    interactions from the bar (SEC-PRV-024); surfaces.md's SUR-002 had no
    such button. *Decided:* the button is part of the wide bar, in the
    right-hand group before the device slot. Applied in surfaces.md
    (SUR-002).
16. **Stream limits in R1** (owner to confirm the default values, security
    decision 25). player.md and design-language.md made the "too many
    streams" state R1, as the baseline requires (SEC-TM-068, SEC-API-031,
    SEC-IAM-102) and the feature map now does (ACC-075); flows.md treated
    it as a video case and listed ACC-075 under R2, and surfaces.md had no
    R1 home for it. *Decided:* R1 for music. The status layer shows the
    notice (SUR-003), the server settings hold the server-wide and
    per-guest limits (SUR-103), F05 and F12 cover the music case, and F10
    lists ACC-075 under R1.
17. **What the device picker lists.** player.md listed only the profile's
    own players, plus household devices in use shown only as "In use";
    surfaces.md (SUR-014) had no "In use" entry; flows.md (F08, step 2) let
    the picker list players another person had granted control of, which
    its own failure case and ACC-047 place in Later. *Decided:* player.md's
    list everywhere, with control of another person's player Later
    (ACC-047, SEC-HIS-014, SEC-PRV-022, SEC-API-016, SEC-API-068), and a
    handoff that carries item IDs and the position, never a stream URL or
    token (SEC-API-026). Applied in surfaces.md (SUR-014) and flows.md
    (F08).
18. **Cast URLs.** player.md cited SEC-CLI-071, a Later requirement, and
    bound cast URLs to the sender's session; flows.md cited SEC-NET-064 and
    a cast session without saying who refreshes them. *Decided:* capability
    URLs scoped to one item and to the cast session the sender started,
    minted and refreshed by the sender within the SEC-API-027 lifetimes,
    which stay inside SEC-NET-064's bound; the receiver holds no credential,
    because cast credentials are Later (SEC-TM-074, SEC-CLI-071). Applied in
    player.md (Casting) and flows.md (F08).
19. **The plain-HTTP answer.** player.md and design-language.md said every
    peer but loopback gets a help page; surfaces.md (SUR-109), flows.md and
    SEC-NET-001 allow a redirect to the HTTPS address first. *Decided:* a
    redirect when an HTTPS address exists, otherwise the static help page.
    Applied in player.md (Security rules for every surface) and
    design-language.md (section 11).
20. **Re-inviting people from an old server.** flows.md put ACC-084 and
    ADM-048 in R2 under F10, while F11 and the feature map place them in
    Later with the rival-database importers (SEC-TM-074). *Decided:*
    Later. Applied in flows.md (F10).
21. **The tuner-busy sheet** (owner to confirm, security decision 5).
    flows.md (F20) said the viewer "sees who holds each one", then that the
    sheet names a person only by that person's choice. *Decided:* the
    second rule, which surfaces.md (SUR-122) already states
    (SEC-PRV-022). Applied in flows.md (F20).
22. **Play and pause on a sleeping TV** (owner to confirm). player.md said
    any key wakes TV Now Playing's ambient mode without acting on playback;
    design-language.md said the play and pause keys control playback on
    every screen (CLI-045). *Decided:* the play or pause key wakes the
    screen and acts, with the brief symbol; every other key only wakes it.
    The screensaver (SUR-060) follows the same rule. Applied in player.md
    (Music on a TV), design-language.md (section 8, rule 8) and surfaces.md
    (SUR-060).
23. **TV Now Playing's queue.** player.md placed the queue rail below the
    artwork and transport; surfaces.md placed the queue beside it.
    *Decided:* below, as player.md lays it out. Applied in surfaces.md
    (SUR-010, SUR-011).

Points that look like contradictions but are not: the flows name screens
with the feature map's surface names ("Admin > Libraries") rather than
surfaces.md's IDs (SUR-085); surfaces.md and player.md both mark TV
lyrics and the TV now-playing entry as proposals because no feature row
defines them yet (surfaces.md open question 4); and player.md's remote
mode in R2 still offers "Play here" after contact is lost, which is the R2
device picker's take-over action, not the R1 prompt that decision 8
dropped.
