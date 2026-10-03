# Surfaces and navigation

This document lists every screen, panel, sheet and persistent control that
Gunmetal's interface needs, and the navigation that holds them together, on
four form factors: TV, phone, tablet, and web or desktop. It was written on
2026-10-02 and is derived from the [feature map](../features/README.md),
which is the source of truth. Every surface below names the feature rows it
serves, by ID, so the UI plan and the backend plan can be read off the same
key.

The method was mechanical where it could be. Every R1, R2 and R3 row in the
ten area files was read, its "UI surfaces" cell was split into the surfaces
it names, and each one was assigned to an entry below. Where the feature map
names a surface loosely ("detail pages", "Settings"), this document decides
which concrete surface that means and says so. Where this document goes
beyond the feature map, for example the exact tabs on a phone, the text says
**Proposal**, so the project owner can tell design choices from settled
scope. Everything here stays inside the two accepted architecture records:
[ADR 1](../adr/0001-architecture.md) (one React Native UI codebase for every
platform, decision 8) and [ADR 2](../adr/0002-music-is-first-class.md)
(music first, a persistent now-playing bar and queue, artwork-led browsing,
dark-first, decision 7).

The [security baseline](../security/README.md) outranks both this document
and the feature map. The owner has directed that every plan be built
around it, so where a surface and the baseline disagree, the surface
follows the baseline and cites the requirement by its ID
(SEC-<AREA>-<NNN>). The surfaces the baseline adds, the ones it changes and
the data it takes off screens are listed in
[Changes made to follow the security baseline](#changes-made-to-follow-the-security-baseline);
the ones that rest on an open owner decision are marked "owner to confirm"
there.

## How to read this document

### Surface entries

Each surface has a stable ID of the form SUR-nnn. IDs are never renumbered;
a new surface takes the next free number in its group. A group whose range
is full (sign-in, settings and account ends at SUR-079) takes its next
surfaces from SUR-130 upward, and they are listed under their own group.
Every entry gives:

- **Release**: the first release in which the surface exists on at least
  one client: R1, a point release (R1.1, R1.2 or R1.3), R2, R3, Later or
  No. A part of a surface can arrive later, and the entry says so, naming
  the release of each part.
- **Purpose**: why the surface exists, in one or two sentences.
- **Shows**: what is on it.
- **Actions**: what a person can do there.
- **Form factors**: how it differs on TV, phone, tablet, and web or desktop.
- **Security**, where the baseline shapes what the surface may show or do:
  the rules, by requirement ID. Each ID is a test the surface must pass
  (SEC-STD-004).
- **Serves**: the feature rows whose "UI surfaces" cell names this surface,
  grouped by the release in which each feature reaches it. A feature that is
  older than the surface reaches it when the surface ships: for example
  loves (DIS-045) are R1, but they reach the film page (SUR-040) in R2, so
  DIS-045 is listed there under R2. The point releases work the same way:
  the track info sheet (SUR-013) arrives in R1.1, so the R1 rows it shows
  are listed there under R1.1.

The [surface index](#surface-index) lists every surface with its release and
the number of rows it serves. The [coverage](#coverage) section checks
that every feature of R1, its point releases and R2 with a UI entry has a
home, and ends with the features that do not.

### Releases

The release key is the feature map's
([README, Releases](../features/README.md#releases)), with the point
releases the owner adopted on 2026-10-02 (D-10 in the
[decision register](../decisions.md#owner-answers-2026-10-02)). R1 is the
smaller first release that the register's
[R1 scope](../decisions.md#r1-scope) lists, and the rest of the earlier R1
arrives in R1.1, R1.2 and R1.3, exactly as that section lists them. The
release values are R1, R1.1, R1.2, R1.3, R2, R3, Later and No.

| Release | What it contains | Clients |
|---|---|---|
| **R1** (music) | Server, web client, music library and player, as the R1 scope lists them. No remuxer or transcoder. HTTPS through the owner's own domain with automatic certificates, a tailnet name or localhost, and remote use through the owner's reverse proxy or a tailnet (D-07). | The web client, in browsers on desktops, laptops, tablets and phones, reached only over HTTPS or on localhost (SEC-NET-001). |
| **R1.1** (bring your music in) | Playlist files and history imports, built-in MusicBrainz and cover-art lookups, ratings, richer credits and browsing, offline loading and installing the web app, profile pictures, continue on this device. | As R1, plus the installable web app (CLI-003). |
| **R1.2** (the household and the admin) | Single sign-on through the household's identity provider, music share links, a second administrator, the admins' live view with each person's choice to show titles, stopping a stream, an arrangeable Home, diagnostics, restore from the admin screens, translations. | As R1.1. |
| **R1.3** (discovery and analysis) | The rule language and smart playlists, library radio and suggestions, measured loudness (if ADR 5 is accepted), folder view, manual curation, 32-bit ARM builds. | As R1.1. |
| **R2** (video) | Movies and TV, the remuxer, sandboxed transcoding, native TV and mobile clients, and everything that needs a native app or a sandbox (downloads, handoff, casting, plugins, video share links, household profiles, API keys and adapters). Also the project's per-server HTTPS name service with its naming client and Certificate Transparency monitoring, and built-in remote access over iroh with relays and the browser edge (D-07). | Android phones and tablets, Android TV, Google TV and Fire OS, and the web client. |
| **R3** (live) | M3U playlists and live TV. | As R2. |
| **Later** | Wanted, not scheduled. Includes the Apple platforms, Samsung and LG TVs, the desktop shell (the security baseline's release scope, SEC-TM-074; owner to confirm), audiobooks, podcasts, photos and watch together. | |
| **No** | Deliberately not doing. | |

### Form factors

| Form factor | Input and setting | R1 | R2 | Later |
|---|---|---|---|---|
| **Web and desktop** | Mouse or trackpad, keyboard, a large window, usually one person | The web client served by the server (CLI-001), in a browser tab; from R1.1 also installed as a web app in its own window (CLI-003) | The web client and the installable web app, as in R1.1, with the R2 web features | The desktop shell for Windows, macOS and Linux (CLI-014), Later because the security baseline puts it there (SEC-TM-074, SEC-CLI-069; owner to confirm); ten-foot mode on a PC (CLI-015) |
| **Phone** | Touch, one hand, small screen, often moving, often on mobile data | The web client at phone width (CLI-149), installable as a web app from R1.1. No offline listening, no car, and background playback on iPhone is unreliable (unverified), as CLI-003 states. | The Android app (CLI-004), with downloads, background audio, lock screen and Android Auto | iPhone (CLI-005), "R2 if the App Store licence decision allows" |
| **Tablet** | Touch, often landscape, sometimes a keyboard, often shared in a household | The web client, laid out by the width classes below; no tablet-specific layout ships in R1. | The Android tablet layout (CLI-053): a side panel for browsing while the player stays visible, reflowing on rotation | iPad |
| **TV** | A D-pad remote with Back, Home and media keys; viewed from about ten feet; shared by a household | None. No TV client exists in R1. | Android TV, Google TV and Fire OS (CLI-006, CLI-008) | Apple TV, Samsung and LG, Roku through the Jellyfin adapter |

**Width classes.** The web client and the native phone and tablet apps lay
out by window width, not by device type, using the four classes proposed in
the sibling [design language](design-language.md#breakpoints-and-density):
compact (under 600 px), medium (600 to 1023), expanded (1024 to 1439) and
wide (1440 and over). This document owns what each class shows; the
navigation sections below say it per form factor. The TV is a separate
layout, not a width class.

Several surfaces live outside the app window: the lock screen and system
media controls, the car, the TV launcher, a watch, the command line and the
project site. They are listed as surfaces too, because features name them and
someone has to design them.

## Navigation model

### Rules the feature map already sets

These rules are decided in the feature map and constrain every layout below.

1. **Nothing core moves between releases.** The player bar, queue access,
   lyrics, device picker, scrubber and navigation keep their places across
   releases; changes ship as an opt-in preview for one release with a way
   back, and visual regression tests pin the positions (CLI-031, MUS-113,
   DIS-015). This is the lesson of Plex's 2025 TV redesign: the research
   records a 504-vote "roll back" thread and Plex restoring its left
   navigation in an August 2026 preview. The consequence for R1 is that
   its navigation has to leave room for the point releases R1.1 to R1.3,
   video, live TV and native features without moving anything when they
   arrive.
2. **Back returns to the exact place**, including the scroll position
   (DIS-112) and, on TV, the focused row and tile (CLI-036).
3. **One context menu everywhere** with the same actions on every item
   (DIS-111, MUS-062).
4. **Every list and search answers from the library synced to the device**
   (CLI-022, DIS-084, MUS-208), so navigation never waits on the server.
5. **Music and video are kept apart**: separate Music and Watch homes
   sharing one search, with a combined view only if wanted (DIS-017;
   discovery open decision 10).
6. **One app, with a music-only mode** that hides video libraries and opens
   on the music Home (CLI-051; clients open decision 8). Each person chooses
   which media kinds they see, and navigation adapts (LAT-012).
7. **On TV, a left rail that stays**, one press away, with every library
   within two presses and favourites pinned to it (CLI-035, CLI-037).
8. **A wide three-pane layout on web and desktop**: library, content and a
   full-height queue together, with resizable panes (CLI-060).
9. **A one-handed phone layout designed first, not shrunk**: bottom
   navigation, a now-playing bar, a full-screen player and a queue sheet
   (CLI-149).

### Rules the security baseline sets

These come from the baseline's first principles and constrain every
surface below as firmly as the rules above.

1. **Nothing before sign-in.** Over plain HTTP every peer except loopback
   gets a redirect or help page and nothing else (SEC-NET-001); in the home
   posture, non-local addresses get a static help page (SEC-NET-024). A
   surface a person sees before signing in shows no user list, server
   name, library name or version (SEC-IAM-022, SEC-NET-047, SEC-API-005).
2. **Untrusted text is text.** Every string from files, providers, other
   people or devices is rendered as text, never markup (SEC-CLI-001,
   SEC-API-046), and a link from metadata is a link only if it is an
   absolute `https:` URL (SEC-CLI-002).
3. **The client is never the gatekeeper.** Every restriction a surface
   applies or displays is also enforced by the server on every request, so
   hiding a control grants and protects nothing (SEC-CLI-015).
4. **Admin screens need their own session, and host-equivalent actions need
   step-up.** Admin surfaces open only in a separate admin session created
   by a passkey check (15 idle minutes, 1 hour at most), and the actions
   listed in SEC-IAM-041 also need a passkey check in the previous 5
   minutes. Most are the owner's alone (SEC-IAM-075, SEC-TM-017).
5. **Limited devices never administer.** TVs, browsers in shared mode,
   browsers signed in by approval and other limited devices never show or
   reach admin, device-approval or account-security surfaces, whatever the
   client shows (SEC-CLI-024, SEC-IAM-108).
6. **Admins see who is playing, not what.** No admin surface shows another
   adult's history, ratings or private playlists; live sessions show the
   title only when that person has chosen to show it (SEC-PRV-025).
7. **Everyone can see and end their own sessions and devices, read their
   own security events, and read what admins can see about them**
   (SEC-IAM-042, SEC-IAM-097, SEC-IAM-104).

### Top-level destinations

The same destinations exist on every form factor; only their placement
changes. **Proposal:** R1 has three destinations plus an account menu, and
later releases add to them without moving them.

| Destination | Release | What it opens | Notes |
|---|---|---|---|
| **Home** | R1 | Home (SUR-020) | In R2 it gains the Music and Watch switch. |
| **Search** | R1 | Search (SUR-032) | Shared by music, video and, in R3, live TV. Kind chips scope it (LAT-013). |
| **Library** | R1 | Library (SUR-022) | Per-kind tabs. In R2 it gains the same Music and Watch switch, and a Downloads entry on devices that download. |
| **Live** | R3 | Live TV home (SUR-120) | Present only when the live TV module is on (LIV-001). |
| **Account menu** | R1 | Account menu (SUR-006) | Settings, Account (with Sessions and devices, Security events, What admins can see and Recovery), History, Hidden (from R1.1), Admin (administrators on personal devices only, behind the admin session), Sign out. R2 adds profile and server switching. |

Pinned items (DIS-013 from R1.2; CLI-037 on TV) and playlists (MUS-132;
pinned playlists from R1.1, MUS-139) are not destinations, but wide layouts and the TV rail show them in the navigation
itself so that a daily album or library is one or two presses away.

### How music and video coexist

R1 is music only. Video arrives in R2 into an app that people already use
for music, so the coexistence model has two jobs: keep the music app intact
for people who never watch anything, and give households a proper video app
without a second install.

**One switch, two homes, one search.** **Proposal:** Home and Library carry
a Music and Watch switch at the top of their content (DIS-017). The switch
changes what Home and Library show and nothing else: Search, the
now-playing bar, the queue, the account menu and the navigation stay where
they are. The choice is remembered per device, so a TV can open on Watch and
a phone on Music (DIS-008). An optional "All" position gives the combined
home for people who want one. The switch appears only when a person can see
both kinds; in music-only mode (CLI-051, LAT-012) it is absent and the app
looks exactly as it did in R1. That is how R2 adds video without breaking
the layout contract for anyone who only listens.

**One queue, separate contexts.** The video queue reuses the music queue
object and its named listening contexts (VID-181, MUS-122, LAT-009).
**Proposal**, settled across the interface documents (README, decision 1;
owner to confirm): starting a film or an episode pauses the music context
rather than replacing it. When the video ends or the player is closed, the
now-playing bar returns to the music context with one tap to resume; the
bar never holds the paused film, which resumes from its title page,
Continue Watching or the queue switcher. The queue switcher (MUS-131) lists
the contexts, so a person can see both. This is also the shape Later's
audiobooks need ("a book never hijacks the music queue", LAT-039).

**Who owns the screen.** Music plays behind every screen and is controlled
from the bar. Video takes the screen: the video player (SUR-046) hides the
navigation and the bar. **Proposal:** leaving the video player pauses the
film and saves its resume point (VID-118) unless picture-in-picture
(VID-134) or listen-only mode (VID-065) is on; in listen-only mode the
session moves to the music player's bar and lock-screen controls, which is
what VID-065 asks for, and that is the only time a video appears in the
bar.

**Shared surfaces.** Search, the context menu, lists and grids, the rule
editor, history, settings and the account surfaces are one surface each,
with video fields added in R2. There is no separate "video settings" area:
video settings are sections in the same Settings (SUR-074).

### Web and desktop

**Frame.** At the wide class, a left sidebar, a content area, a right pane
and the now-playing bar along the bottom (CLI-060). At the expanded class
the right pane overlays the content instead of sitting beside it, and at the
medium class the sidebar becomes a navigation rail and the queue opens as a
side sheet (**Proposal**, following the width classes). The sidebar holds Home, Search
and Library at the top, then pinned items (from R1.2), then playlists
(MUS-132, MUS-139; folders from R2, MUS-136), and the account menu and status
indicator at the bottom (**Proposal** for the order). The right pane holds
the queue at full height (MUS-119), or lyrics, or the track info sheet; it
can be resized or closed. Spotify set this pattern and Feishin copies it,
according to the music UX research; the research also records users
objecting when Spotify squeezed its queue into a narrow sidebar in March
2024, which is why the queue pane here can grow to full height.

**Always on screen.** The sidebar (collapsible to icons), the now-playing
bar whenever anything is queued, and the status indicator. The right pane
stays open if the person left it open.

**Not on screen.** In the video player, the sign-in and setup screens, and
the server-rendered startup, emergency and help pages, nothing persistent
is shown.

**Keyboard.** Every control is reachable by keyboard with a visible focus
ring (CLI-138). R1 has no keyboard shortcuts beyond each control's standard
keys and the hardware media keys ([player.md](player.md), Input mappings).
From R2, shortcuts and the command palette (SUR-007) reach any command or
item (CLI-061, DIS-090). The
desktop shell's media keys, global hotkeys and mini player window
(CLI-063, CLI-064, CLI-065) are Later with the shell.

**R2 and R3 additions.** The Music and Watch switch appears at the top of
Home and Library. Downloads does not appear on the web: the web client
cannot hold downloads (MUS-217 is Later), and the desktop shell that could
is Later. Live appears in the sidebar under Library when the module is on.

### Phone

**Frame.** A bottom tab bar with Home, Search and Library, and the
now-playing bar just above it (CLI-149). The account menu opens from an
avatar at the top of Home (**Proposal**). The full-screen player opens from
the bar as a sheet that swipes down to close, and the queue opens as a sheet
from the player or the bar (CLI-149). The research notes that Spotify keeps
to a few tabs, each with one clear job; three tabs follow that lesson.

**Always on screen.** The tab bar and the now-playing bar, except in the
full-screen player, the video player, sheets, and the sign-in and setup
flows. Status appears as a quiet banner only when something is wrong
(CLI-025).

**R1 limits.** The phone in R1 is a browser, and from R1.1 also an
installed web app. It
gets lock-screen controls through the browser (CLI-070, MUS-073) but no
downloads, no car and unreliable background playback on iPhone
(unverified), and the app says so (CLI-003).

**R2 additions.** The Android app adds the Music and Watch switch, Downloads
as the first entry inside Library (**Proposal**; offline is not a separate
mode, CLI-026, so no Downloads tab is added), the device picker in the bar
and player, a cast button, swipe actions and long-press previews, and the
lock screen, notification, Android Auto and widgets that live outside the
app. **Proposal for R3:** a fourth tab, Live, appears only when the live TV
module is on, to the right of Library, so no existing tab moves.

### Tablet

**Frame in R1.** The web client lays out by width class, so most portrait
tablets get the medium layout (a navigation rail, content, and the queue as
a side sheet) and most landscape tablets get the expanded layout (sidebar
and content, with the queue pane over the content). Small tablets in
portrait may fall into the compact phone layout. Exact device widths are
not checked here (unverified).

**Frame in R2.** The Android tablet layout (CLI-053): a navigation rail on
the left edge with the same destinations as the phone tabs, a browsing area,
and a player side panel that stays visible while browsing, reflowing on
rotation. In landscape the player shows artwork and lyrics side by side
(MUS-112, CLI-054). The research records that Spotify's April 2026 tablet
app is built this way, reconfiguring rather than stretching; that is the bar
to meet.

**Always on screen.** The rail and either the now-playing bar (portrait) or
the player side panel (landscape).

### TV

**Frame.** A left rail that is one press away from anywhere (CLI-035).
**Proposal for its order**, shared with the design language: Now Playing
always first, present whether or not anything is playing, so playback is
always one press away and no entry moves when playback starts or stops;
then Search, Home, then one entry for each library the profile can see (music libraries open the music Home for that library;
film and show libraries open their grids), then pinned items (CLI-037), then
Live (R3), then Settings and the profile. Every library is within two
presses: left, then down to it (CLI-037). The rail's position and order are
part of the layout contract and pinned by tests. The earlier draft showed
Now Playing only while something played, which would have shifted every
other entry and broken that contract.

**Always on screen.** On browse screens, the rail collapsed to a narrow
strip of icons at the left edge, which expands to show labels when focus
moves into it (**Proposal**). The rail's first entry opens TV Now Playing
(there is no now-playing bar on TV); with nothing queued it shows the
empty-queue state. The remote's play and pause keys control playback from
any screen (CLI-045). In the video player and the screensaver nothing
persistent is shown.

**Launch.** On a shared TV the profile picker comes first (CLI-050,
DIS-142), then the Home the device remembers: Watch by default on a TV that
can see video libraries (**Proposal**).

**Focus.** Focus lands where a person expects and Back returns to the same
row and tile (CLI-036): from any screen, including TV Now Playing, Back
goes to the screen and card the person came from, with playback
continuing, and from a top-level screen's content to the rail (the design
language, section 8, rule 7). Every card has a long-press menu, which is the TV
form of the context menu (CLI-039). Every grid has a letter column
(CLI-040, DIS-163). Text size can be raised (CLI-043).

**What the TV does not do.** **Proposal:** the TV does not host the rule
editor, the metadata edit sheet's free-text fields, the webhook and plugin
screens, or the dense admin editors. It shows the results of all of them.
The TV is a limited device, so it never shows an admin surface, approves
another device or changes account security, and the server refuses those
routes from it whatever the client shows (SEC-CLI-024, SEC-IAM-049). On a
TV, alerts appear only as a banner on an admin's profile (ADM-114), and
every admin task offers a QR code that opens it on a phone or computer.
This replaces the earlier admin dashboard, simplified edit sheet (LIB-172)
and live TV set-up wizard (LIV-002) on TV, which need admin rights.
Jellyfin's Android TV app already keeps a left navigation, according to
the discovery research; Gunmetal matches it and adds the layout contract
and focus tests.

### What is always on screen

| State | Web and desktop | Phone | Tablet (R2 native) | TV (R2) |
|---|---|---|---|---|
| Browsing | Sidebar (a rail at the medium class), now-playing bar, status indicator, right pane if open | Tab bar, now-playing bar | Navigation rail, now-playing bar or player side panel | Collapsed rail, with Now Playing always its first entry |
| Full-screen music player | Sidebar and right pane stay; the player fills the content area | Nothing else; swipe down to return | Rail stays | TV Now Playing fills the screen; Back returns to the screen and card the person came from, with playback continuing |
| Video player | Nothing; full window or full screen | Nothing | Nothing | Nothing |
| Offline or server unreachable | A quiet banner and dimmed unplayable items (CLI-025) | Same | Same | Same |
| Sign-in, setup, startup, emergency and help pages | Nothing | Nothing | Nothing | Nothing |

## Surface index

| ID | Surface | Group | First release | Feature rows |
|---|---|---|---|---:|
| SUR-000 | Every surface | Shell and persistent controls | R1 | 14 |
| SUR-001 | Navigation shell | Shell and persistent controls | R1 | 14 |
| SUR-002 | Now-playing bar | Shell and persistent controls | R1 | 18 |
| SUR-003 | Status layer and notice centre | Shell and persistent controls | R1 | 27 |
| SUR-004 | Context menu | Shell and persistent controls | R1 | 37 |
| SUR-005 | Selection bar and drag and drop | Shell and persistent controls | R1.1 | 6 |
| SUR-006 | Account menu and profile picker | Shell and persistent controls | R1 | 13 |
| SUR-007 | Command palette and shortcut help | Shell and persistent controls | R2 | 4 |
| SUR-008 | Confirmation and step-up prompt | Shell and persistent controls | R1 | 1 |
| SUR-009 | Upload dialog | Shell and persistent controls | R1 | 1 |
| SUR-010 | Full-screen player | Listening | R1 | 38 |
| SUR-011 | Queue | Listening | R1 | 20 |
| SUR-012 | Lyrics view | Listening | R1 | 12 |
| SUR-013 | Track info sheet | Listening | R1.1 | 28 |
| SUR-014 | Device picker and remote mode | Listening | R2 | 11 |
| SUR-015 | Add-to-playlist sheet | Listening | R1 | 3 |
| SUR-020 | Home | Home, library, browse and search | R1 | 51 |
| SUR-021 | Home editor | Home, library, browse and search | R1.2 | 7 |
| SUR-022 | Library | Home, library, browse and search | R1 | 30 |
| SUR-023 | Lists, grids, rows and browse controls | Home, library, browse and search | R1 | 60 |
| SUR-024 | Artist page | Home, library, browse and search | R1 | 40 |
| SUR-025 | Album page | Home, library, browse and search | R1 | 34 |
| SUR-026 | Playlist page | Home, library, browse and search | R1 | 10 |
| SUR-027 | Rule editor | Home, library, browse and search | R1.3 | 12 |
| SUR-028 | Browse pages | Home, library, browse and search | R1 | 12 |
| SUR-029 | History page | Home, library, browse and search | R1 | 8 |
| SUR-030 | Hidden page | Home, library, browse and search | R1.1 | 3 |
| SUR-031 | Statistics page | Home, library, browse and search | R2 | 2 |
| SUR-032 | Search | Home, library, browse and search | R1 | 18 |
| SUR-040 | Title page | Watching | R2 | 34 |
| SUR-041 | Show, season and episode pages | Watching | R2 | 18 |
| SUR-042 | Collection page and editor | Watching | R2 | 14 |
| SUR-043 | Person page | Watching | R2 | 2 |
| SUR-044 | Watchlist page | Watching | R2 | 2 |
| SUR-045 | Pre-play sheet and version picker | Watching | R2 | 20 |
| SUR-046 | Video player | Watching | R2 | 38 |
| SUR-047 | Player sheets | Watching | R2 | 36 |
| SUR-048 | Playback information overlay | Watching | R2 | 15 |
| SUR-049 | Player cards and prompts | Watching | R2 | 20 |
| SUR-050 | Downloads | Devices, offline and the operating system | R2 | 13 |
| SUR-051 | Cast controls | Devices, offline and the operating system | R2 | 4 |
| SUR-052 | Mini player window | Devices, offline and the operating system | Later | 3 |
| SUR-053 | System media controls | Devices, offline and the operating system | R1 | 18 |
| SUR-054 | Car | Devices, offline and the operating system | R2 | 11 |
| SUR-055 | TV launcher rows | Devices, offline and the operating system | R2 | 3 |
| SUR-056 | Watch controls | Devices, offline and the operating system | R2 | 1 |
| SUR-057 | Install, deep links and operating-system hooks | Devices, offline and the operating system | R1.1 | 4 |
| SUR-058 | Share sheet | Devices, offline and the operating system | R1.2 | 10 |
| SUR-059 | Public share page | Devices, offline and the operating system | R1.2 | 1 |
| SUR-060 | TV screensaver | Devices, offline and the operating system | R2 | 3 |
| SUR-061 | Pairing and approval | Devices, offline and the operating system | R1 | 7 |
| SUR-070 | Sign-in | Sign-in, settings and account | R1 | 9 |
| SUR-071 | Invite landing and first launch | Sign-in, settings and account | R1 | 3 |
| SUR-072 | Server picker | Sign-in, settings and account | R2 | 5 |
| SUR-073 | Settings | Sign-in, settings and account | R1 | 16 |
| SUR-074 | Settings: playback, sound and subtitles | Sign-in, settings and account | R1 | 43 |
| SUR-075 | Settings: data, downloads and storage | Sign-in, settings and account | R1.1 | 10 |
| SUR-076 | Settings: appearance, accessibility and language | Sign-in, settings and account | R1 | 11 |
| SUR-077 | Settings: this device, connection and help | Sign-in, settings and account | R1 | 8 |
| SUR-078 | Account | Sign-in, settings and account | R1 | 47 |
| SUR-079 | Profiles and guardian controls | Sign-in, settings and account | R1.1 | 26 |
| SUR-080 | Startup page | Setup and administration | R1 | 12 |
| SUR-081 | Emergency page | Setup and administration | R1.2 | 1 |
| SUR-082 | Welcome (first-run setup) | Setup and administration | R1 | 27 |
| SUR-083 | Admin dashboard | Setup and administration | R1 | 30 |
| SUR-084 | Sessions | Setup and administration | R1.2 | 17 |
| SUR-085 | Libraries and library settings | Setup and administration | R1 | 37 |
| SUR-086 | Library health | Setup and administration | R1 | 27 |
| SUR-087 | Review queue | Setup and administration | R1.3 | 3 |
| SUR-088 | File inspector | Setup and administration | R1.2 | 11 |
| SUR-089 | Corrections and metadata editor | Setup and administration | R1.3 | 33 |
| SUR-090 | Users and invitations | Setup and administration | R1 | 17 |
| SUR-091 | Household, policies, devices and shares | Setup and administration | R2 | 13 |
| SUR-092 | Sign-in and security settings | Setup and administration | R1 | 5 |
| SUR-093 | Network and remote access | Setup and administration | R1 | 15 |
| SUR-094 | Integrations, tokens and webhooks | Setup and administration | R2 | 42 |
| SUR-095 | Plugins and providers | Setup and administration | R2 | 22 |
| SUR-096 | Compatibility adapters | Setup and administration | R2 | 7 |
| SUR-097 | Migration | Setup and administration | R1.1 | 11 |
| SUR-098 | Backups and export | Setup and administration | R1 | 12 |
| SUR-099 | Updates | Setup and administration | R1 | 6 |
| SUR-100 | Tasks and activity | Setup and administration | R1 | 22 |
| SUR-101 | Alerts and logs | Setup and administration | R1 | 14 |
| SUR-102 | Diagnostics | Setup and administration | R1 | 11 |
| SUR-103 | Server settings and about | Setup and administration | R1 | 10 |
| SUR-104 | Totals and statistics for administrators | Setup and administration | R2 | 2 |
| SUR-105 | Trash | Setup and administration | R1 | 2 |
| SUR-106 | Video tools | Setup and administration | R2 | 3 |
| SUR-107 | Household curation | Setup and administration | R2 | 5 |
| SUR-108 | Security log | Setup and administration | R1 | 1 |
| SUR-109 | Plain-HTTP and home-posture help pages | Setup and administration | R1 | 3 |
| SUR-110 | Command line | Outside the client | R1 | 21 |
| SUR-111 | Project site and documentation | Outside the client | R1 | 32 |
| SUR-112 | API reference page | Outside the client | R1 | 4 |
| SUR-113 | Other apps' own interfaces | Outside the client | R2 | 2 |
| SUR-120 | Live TV home | Live TV | R3 | 8 |
| SUR-121 | Guide | Live TV | R3 | 20 |
| SUR-122 | Live player | Live TV | R3 | 30 |
| SUR-123 | Recordings | Live TV | R3 | 30 |
| SUR-124 | Live TV administration | Live TV | R3 | 59 |
| SUR-125 | Radio | Live TV | R3 | 4 |
| SUR-130 | Sessions and devices | Sign-in, settings and account | R1 | 4 |
| SUR-131 | Security events | Sign-in, settings and account | R1 | 2 |
| SUR-132 | What admins can see, and privacy choices | Sign-in, settings and account | R1 | 3 |
| SUR-133 | Recovery | Sign-in, settings and account | R1 | 3 |

## Surface inventory

### Shell and persistent controls

#### SUR-000 Every surface
- **Release:** R1. Lists that keep working without the server for what
  the device holds arrive in R1.1 (CLI-026).
- **Purpose:** Collects the features that are rules for every screen rather
  than a place on one: accessibility, text size, motion, keyboard and focus,
  right-to-left layouts, offline behaviour and per-person discovery. A
  surface that breaks one of these is broken.
- **Shows:** Nothing of its own.
- **Actions:** None of its own.
- **Form factors:** *TV:* focus memory and remote and controller keys work
  on every screen (CLI-036, CLI-045). *Phone and tablet:* text follows the
  system size (CLI-139) and every control is reachable by TalkBack, and by
  VoiceOver once Apple builds exist (CLI-135). *Web and desktop:* full
  keyboard use with a visible focus ring (CLI-138); WCAG 2.2 AA is the
  recommended release bar (clients open decision 9). *All:* reduced motion
  (CLI-140), motor accessibility (CLI-142), and, from R1.1, every list keeps
  working without the server for what the device holds (CLI-026; MUS-209
  in R2).
- **Serves:** R1: CLI-135, CLI-138, CLI-139, CLI-140, CLI-142, DIS-140. R1.1: CLI-026. R2: CLI-036, CLI-045, CLI-137, CLI-147, DIS-160, MUS-209. Later: CLI-014 (the desktop shell, SEC-TM-074).

#### SUR-001 Navigation shell
- **Release:** R1 (web, wide and phone-width). R1.2 extends the promise
  that nothing moves to the Home layout (DIS-015). R2 adds the TV rail, the
  tablet layout, the native phone frame and the Music and Watch switch.
- **Purpose:** The frame that holds every destination and keeps each one in
  the same place across releases.
- **Shows:** The top-level destinations, pinned items and playlists on wide
  layouts, the account menu entry and the status indicator, as described in
  the [navigation model](#navigation-model).
- **Actions:** Go to a destination; go back and forward to the exact scroll
  position (DIS-112); resize or collapse panes (CLI-060); switch between
  Music and Watch (DIS-017, R2).
- **Form factors:** *TV:* left rail, one press away, with libraries within
  two presses (CLI-035, CLI-037, DIS-159). *Phone:* bottom tab bar
  (CLI-149; CLI-004 in R2). *Tablet:* the web layout chosen by width in R1;
  a side navigation rail with a browsing side panel in R2 (CLI-053). *Web
  and desktop:* sidebar, content, right pane and bottom bar (CLI-060).
- **Serves:** R1: CLI-001, CLI-060, CLI-149, DIS-112. R1.2: DIS-015. R2: CLI-004, CLI-006, CLI-008, CLI-035, CLI-037, CLI-053, DIS-017, DIS-159, LAT-012.

#### SUR-002 Now-playing bar
- **Release:** R1. The device slot's "Continue on this device" prompt
  arrives in R1.1 (CLI-103), and the source label for radio in R1.3
  (DIS-067).
- **Purpose:** Keeps playback visible and controllable on every screen, in
  a fixed place (MUS-108, MUS-113).
- **Shows:** Artwork, title, the credit line with every artist linked
  (MUS-001), a thin progress line, play and pause, next, the love heart
  (MUS-109), the compact quality badge that says whether the original file
  is playing ("Original"; "Converted" from R2) on every layout, phone width
  included (MUS-099, MUS-066), the private-session indicator while one is on
  (ACC-117), the source label for radio (DIS-067, from R1.3), and the
  device slot, which keeps its place from R1 and is empty in R1; from R1.1
  it holds only a "Continue on this device" prompt when another of the
  profile's sessions last played the queue (CLI-103). When another session
  starts playback, this one pauses at its point: the last Play wins
  (MUS-122), and from R1.1 the bar shows the same prompt. From R2:
  the Devices button in the same slot, with the output device (MUS-197),
  the Opus badge on mobile data (MUS-106) and a per-person quality cap
  (ACC-107). A video appears in the bar only in listen-only mode (VID-065).
- **Actions:** Play or pause, skip, love, open the full-screen player, open
  the queue, continue on this device (R1.1); on wide layouts, open the player's
  options menu, whose first item is Private session; from R2, open the
  device picker and return to the paused music context after a video.
- **Form factors:** *TV:* no bar. **Proposal:** the rail's fixed Now
  Playing entry and the remote's media keys stand in for it. *Phone:* a
  compact bar above the tabs; tap opens the player, and the options menu
  is in the full player. *Tablet:* the bar in portrait; in landscape on R2
  the player side panel replaces it. *Web and desktop:* a full-width bar
  with the lyrics and queue toggles, Options and volume on the right, and
  the device slot at the end of that group, as [player.md](player.md)
  fixes.
- **Security:** the private-session indicator stays visible for as long as
  the session is private, so nobody is private by accident; the session is
  started from the player's options menu in at most two interactions, which
  on wide layouts means from the bar's Options button (SEC-PRV-024).
  Titles and credit lines are rendered as text (SEC-CLI-001).
- **Serves:** R1: ACC-117, DIS-045, DIS-053, LAT-006, MUS-001, MUS-066, MUS-099, MUS-108, MUS-109, MUS-122, MUS-180. R1.1: CLI-103. R1.3: DIS-067. R2: ACC-107, LAT-098, MUS-101, MUS-106. R3: LIV-167.

#### SUR-003 Status layer and notice centre
- **Release:** R1, including the notice centre, because R1 delivers
  security alerts in the app only (SEC-OPS-032, SEC-IAM-098). R1.1 adds
  the quiet "server unreachable" banner and the undo toast (CLI-025,
  DIS-023); R1.2 adds banners from an administrator who stopped a stream
  (ACC-073, ADM-102). "Update needed" arrives with the native apps in R2
  (CLI-032).
- **Purpose:** Tells the truth about the connection, sync, scans, server
  messages and security events without blocking anything.
- **Shows:** The connection indicator (ACC-003; home or away and relay
  state from R2, CLI-029, ACC-105, ACC-096), sync status (DIS-002,
  MUS-214), the quiet "server unreachable" banner (CLI-025, from R1.1) and
  dimmed items that cannot play (ADM-085, LIB-032), the first-scan progress
  banner (MUS-043, MUS-042), "Update needed" for old native clients
  (CLI-032, R2), the unsupported-browser notice (CLI-002), the undo toast
  (DIS-023, from R1.1; queue undo MUS-121 in R2), a notice when a damaged file was skipped, linking to the
  health report (MUS-079), the "too many streams" notice when a stream
  limit refuses music playback, saying which limit and what to do
  (ACC-075, SEC-TM-068, SEC-API-031; the player's "Not allowed" state), and
  banners from the administrator (ACC-073,
  ADM-102, from R1.2; scheduled notices ADM-103 and messages ACC-074 in R2). From R1,
  the notice centre keeps security notices for this account: a new device
  or credential, a removed credential, use of recovery and the recovery
  hold, a changed identity-provider link (from R1.2), a role change and bursts of
  failed attempts (ACC-071, SEC-IAM-098); for the owner and admins, the
  owner alerts of SEC-OPS-032, with a persistent banner for critical ones.
  It also shows "Your session has ended" when this device was signed out
  elsewhere (SEC-IAM-043). From R2 it keeps follow alerts (INT-050), and
  banners report transcoding being off (VID-009) or a setting that needs a
  restart (ADM-008).
- **Actions:** Dismiss, undo, open the detail, retry now. On every alert
  about a device or credential: **It was me** and **It wasn't me**; the
  second revokes it, ends its sessions and makes its stream URLs fail
  (SEC-OPS-033). On a recovery-hold notice: end the hold and revoke the new
  credential with one tap (SEC-IAM-106).
- **Form factors:** *TV:* toasts in a corner that never take focus
  (**Proposal**, to keep CLI-036's focus promise); security alerts appear
  only on an admin's profile and never interrupt playback. *Phone:* banners
  at the top of the content; toasts above the now-playing bar. *Tablet:* as
  phone. *Web and desktop:* the indicator in the sidebar footer, banners at
  the top of the content area, toasts above the bar.
- **Security:** notices never carry another person's activity, and carry
  titles only when the recipient turned that notice type on (SEC-PRV-030).
  Re-signing in with a known passkey on a known device raises nothing, and
  non-critical notices are batched into a daily summary (SEC-IAM-098). The
  critical owner alerts cannot be muted, only batched (SEC-OPS-034). On a
  lock screen, security notices say only that something happened
  (SEC-CLI-062). Managed profiles never receive security alerts.
- **Serves:** R1: ACC-003, ACC-071, ACC-075, ADM-085, CLI-002, DIS-002, LIB-032, MUS-042, MUS-043, MUS-079, MUS-229. R1.1: CLI-025, DIS-023. R1.2: ACC-073, ADM-102. R2: ACC-074, ACC-096, ACC-105, ADM-008, ADM-103, CLI-029, CLI-032, CLI-094, INT-050, MUS-209, MUS-214, VID-009.

#### SUR-004 Context menu
- **Release:** R1. R1.1 adds Rate, Info, Dismiss and, for administrators,
  "Look up on MusicBrainz" on albums (DIS-047, MUS-181, DIS-022, LIB-111);
  R1.2 adds Pin, the Home row menu, Copy link, Share for music and, for
  administrators, File info (DIS-013, DIS-003, CLI-034, INT-147, ACC-086,
  LIB-195); R1.3 adds Start radio and, for administrators, Merge and Split
  (MUS-165, DIS-067, LIB-041, LIB-058).
- **Purpose:** The same actions on every item, everywhere (DIS-111,
  MUS-062), so a large app feels small.
- **Shows:** For music items: Play next (MUS-117), Add to queue and Play last
  (MUS-118), Add to playlist (opens SUR-015), Love (DIS-045), Go to artist
  or album, and "Copy ID" in developer mode (INT-008). On history rows:
  Remove this play (MUS-184, DIS-052). For administrators, in an admin
  session on a personal device: Rescan (LIB-012). From R1.1: Rate (DIS-047,
  MUS-181), Info (opens SUR-013), Dismiss on Continue rows (DIS-022), and
  for administrators "Look up on MusicBrainz" on albums (LIB-111). From
  R1.2: Pin (DIS-013), the Home row menu (DIS-003), Copy link (CLI-034,
  INT-147), Share for music tracks, albums and playlists (opens SUR-058;
  ACC-086), and for administrators File info (LIB-195). From R1.3: Start
  radio (MUS-165, DIS-067), and for administrators Merge and Split
  (LIB-041, LIB-058). From R2: Tag (MUS-023),
  Hide, Snooze, Less like this and Keep out of my taste (MUS-170, DIS-054,
  DIS-055, DIS-057), Mark played (DIS-033), Watch video (MUS-029, LIB-189),
  Download and Download to another device (CLI-097), Share for films and
  episodes when the owner has enabled video links (ACC-092), Add to
  collection, Pin as a Home row (DIS-126), Allow for a child (ACC-029), Lock
  (LIB-175), and the video queue actions (VID-181).
- **Actions:** As listed. Destructive or sensitive actions confirm
  (SUR-008). A copied link never grants access by itself (CLI-034).
- **Form factors:** *TV:* long-press on every card opens the menu as a side
  sheet (CLI-039, DIS-162). *Phone:* a "more" button on every row and tile,
  and long-press; from R2, swipe actions on rows (MUS-065) and a long-press
  preview sheet (MUS-064). *Tablet:* as phone, plus right-click with a
  pointer. *Web and desktop:* right-click and a "more" button (MUS-062).
- **Serves:** R1: DIS-045, DIS-052, DIS-111, INT-008, LIB-012, MUS-062, MUS-117, MUS-118, MUS-184. R1.1: DIS-022, DIS-047, LIB-111. R1.2: CLI-034, DIS-003, DIS-013, INT-147. R1.3: DIS-067, MUS-165. R2: CLI-037, CLI-039, CLI-052, DIS-027, DIS-033, DIS-039, DIS-054, DIS-055, DIS-057, DIS-126, DIS-162, LIB-175, LIB-189, MUS-023, MUS-029, MUS-064, MUS-170, VID-181. R3: LIV-103.

#### SUR-005 Selection bar and drag and drop
- **Release:** R1.1, with multi-select and drag and drop (DIS-110,
  CLI-062, MUS-063), which moved from R1.
- **Purpose:** Act on many items at once (DIS-110, CLI-062).
- **Shows:** The number selected and the actions that apply to all of them:
  play, add to queue, add to playlist, love, remove. From R2: mark played
  (DIS-033), add to collection (LIB-163), add a label (LIB-180) and bulk
  edit (LIB-176).
- **Actions:** Select a range, drag selected tracks onto a sidebar playlist
  or into the queue pane (MUS-063).
- **Form factors:** *TV:* not offered (**Proposal**). The earlier exception,
  the live TV lineup editor's remote multi-select (LIV-059), goes too,
  because the lineup editor is an admin surface and a TV never shows one
  (SEC-CLI-024). *Phone:* a "Select" action in list headers enters selection mode,
  because long-press already opens the context menu (**Proposal**); queue
  and playlist rows have drag handles. *Tablet:* as phone, and in R2 drag
  from the browsing panel onto the player panel's queue. *Web and desktop:*
  click, Shift-click and Ctrl- or Cmd-click, and drag.
- **Serves:** R1.1: CLI-062, DIS-110, MUS-063. R2: DIS-033, LIB-163, LIB-180.

#### SUR-006 Account menu and profile picker
- **Release:** R1 (account menu). R1.1 adds the profile's name and
  picture (ACC-011) and the Hidden page. R2 (profile picker, PIN and
  switching).
- **Purpose:** Shows who is using the app and leads to personal and server
  settings.
- **Shows:** The person's name, with the profile's own name and picture
  from R1.1 (ACC-011), the private-session
  badge while one is active (DIS-053, MUS-185), and links to Settings,
  Account, Sessions and devices (SUR-130), Security events (SUR-131), What
  admins can see (SUR-132), History, Hidden (from R1.1), Admin (administrators on a
  personal device only; opening it asks for the passkey and starts the
  admin session, SEC-IAM-041, SEC-CLI-024) and Sign out. Signing out ends
  the session on the server and clears this account's data from the
  browser (SEC-API-039, SEC-CLI-009). R1
  ships profiles separate from sign-in as a data model (ACC-017), but no
  picker. From R2: Switch profile with an optional PIN (ACC-019, ACC-020,
  DIS-153), the household's profiles (ACC-016, ACC-018), and Switch server
  (SUR-072).
- **Actions:** Open a destination, switch profile (R2), sign out.
- **Form factors:** *TV:* a "Who's watching" picker at launch (CLI-050,
  DIS-142) and a profile entry at the foot of the rail; PIN entry on a
  number pad. *Phone:* an avatar at the top of Home (**Proposal**).
  *Tablet:* the same avatar at the foot of the rail. *Web and desktop:* the
  avatar at the foot of the sidebar.
- **Serves:** R1: ACC-017, ACC-117, DIS-053, MUS-185. R1.1: ACC-011. R2: ACC-016, ACC-018, ACC-019, ACC-020, CLI-050, DIS-141, DIS-142, DIS-153.

#### SUR-007 Command palette and shortcut help
- **Release:** R2.
- **Purpose:** Keyboard-first control for people who live on a computer
  (CLI-061, MUS-080, DIS-090).
- **Shows:** A search-as-you-type list of commands and library items; a
  shortcut reference; the keyboard help overlay in the video player
  (VID-105).
- **Actions:** Run a command, jump to an item, queue it.
- **Form factors:** *TV:* not offered. *Phone:* not offered. *Tablet:* only
  with a hardware keyboard (**Proposal**). *Web and desktop:* a global
  shortcut opens it.
- **Serves:** R2: CLI-061, DIS-090, MUS-080, VID-105.

#### SUR-008 Confirmation and step-up prompt
- **Release:** R1.
- **Purpose:** Confirms a sensitive change by asking for the person's
  passkey again (ACC-056), opens the separate admin session, and confirms
  anything that cannot be undone. There is no password to ask for
  (SEC-IAM-025).
- **Shows:** What will change, in plain words, and the passkey prompt. The
  three kinds: opening admin screens, which starts an admin session that
  ends after 15 idle minutes or 1 hour; step-up, a passkey check in the
  previous 5 minutes for the actions SEC-IAM-041 lists (trusted proxies,
  posture and remote administration, TLS and naming, egress policy, plugins,
  adapters, backup download and restore, ownership transfer, key rotation,
  creating or promoting administrators, adding or removing library roots,
  browsing the file system); and the same 5-minute check for adding or
  removing a sign-in method, exporting data or deleting the account
  (SEC-IAM-023, SEC-PRV-048).
- **Actions:** Confirm with the passkey, or cancel.
- **Form factors:** *TV:* not offered. A TV is a limited device that can
  never perform these actions, so the TV shows a QR code that opens the
  action on a phone or computer instead (SEC-CLI-024). This replaces the
  earlier proposal to approve a TV's sensitive change from a phone. *Phone
  and tablet:* a dialog with the platform passkey prompt; from R2 the
  native apps use a biometric or device PIN bound to the device key
  (SEC-CLI-059). *Web and desktop:* a dialog.
- **Security:** an identity-provider sign-in alone never satisfies step-up
  (SEC-IAM-107). Every step-up action is audited, and host-equivalent ones
  are announced to every admin (SEC-TM-017). A shared-mode browser or a
  browser signed in by approval cannot reach this prompt for admin or
  account-security actions (SEC-CLI-024, SEC-IAM-108).
- **Serves:** R1: ACC-056.

#### SUR-009 Upload dialog
- **Release:** R1.
- **Purpose:** The one way files enter the server from the interface, built
  so an upload cannot reach the filesystem by name or attack the parser
  (ACC-125).
- **Shows:** The chosen file, what it will be used for, and a typed error if
  it cannot be read. R1 uses it for backup uploads (ADM-069). From R1.1 it
  also takes M3U imports (MUS-140), listening-history files (ADM-042,
  INT-107) and playlist imports (ADM-043), and from R2 artwork (LIB-138)
  and subtitle files (VID-075).
- **Actions:** Choose or drop a file, cancel.
- **Form factors:** *TV:* not offered (**Proposal**). *Phone and tablet:*
  the system file picker. *Web and desktop:* a file picker and a drop zone.
- **Security:** each upload route declares its allowed types and size, and
  uploads are capped per person by count and bytes (SEC-API-085,
  SEC-API-088); files are stored by a server-generated content hash, never
  by name (SEC-API-087); uploaded images are re-encoded with their metadata
  removed (SEC-PRV-006); a backup uploaded at setup is accepted only with
  the setup code (SEC-OPS-008).
- **Serves:** R1: ACC-125.

### Listening

#### SUR-010 Full-screen player
- **Release:** R1. R1.1 adds shuffle by album, the sleep timer and the
  info button (MUS-127, MUS-076, MUS-114); R1.3 adds radio as a source
  (DIS-067).
- **Purpose:** The music player at its fullest: artwork-led, honest about
  what it is playing, and quick to control (MUS-110).
- **Shows:** Large artwork with colours computed at scan time (MUS-110,
  MUS-039), the title and the credit exactly as tagged with every name
  linked (MUS-002), a "Playing from" link to the album or playlist
  (MUS-123), an always-visible scrubber with exact seeking (MUS-071),
  transport controls, shuffle and repeat (MUS-126, MUS-077; shuffle by
  album from R1.1, MUS-127), love (MUS-109), the quality badge (MUS-099),
  the buffer state (MUS-070), and the gain mode in the player info
  (MUS-087). The player menu holds Private session (ACC-117, MUS-185) and,
  from R1.1, the sleep timer (MUS-076). Buttons open the queue, lyrics and,
  from R1.1, info. From R2: info cards under the player
  (MUS-111), landscape (MUS-112, CLI-054), a large-text layout for car
  mounts (MUS-228), the equaliser quick toggle (MUS-094), remote mode when
  controlling another device (CLI-102, MUS-198), and "watch the video" for
  songs that have one (LAT-091, LAT-098).
- **Actions:** All transport actions, swipe the artwork to skip, love,
  rate (R1.1), start a private session, set a sleep timer (R1.1), open
  queue, lyrics, info (R1.1) and (R2) the device picker.
- **Form factors:** *TV:* TV Now Playing (R2), the rail's fixed first
  entry: artwork-led, with a rail of the next queue items below it and a
  dim ambient mode (CLI-041, MUS-223, DIS-167); remote keys drive it, and
  Back returns to where the person came from with playback continuing. *Phone:* a full-screen sheet over the tabs; swipe
  down to close. *Tablet:* in R2 the player can live in the side panel;
  landscape shows artwork and lyrics side by side. *Web and desktop:* fills
  the content area while the sidebar and right pane stay; the research
  credits Plexamp's free UltraBlur backgrounds and Finamp's artwork colours
  as the strongest existing examples.
- **Security:** Private session is at most two interactions from playback
  on every client, the first item of the player's options on TV; while it
  is on, nothing is recorded in history, recommendations or scrobbles, and
  any admin's live view omits the title (SEC-PRV-024). It ends when turned
  off or after a period without playback the person picks, 6 hours by
  default. From R2, private plays are never handed to the operating
  system's recents or home-screen rows (SEC-PRV-058).
- **Serves:** R1: ACC-117, MUS-001, MUS-002, MUS-039, MUS-070, MUS-071, MUS-077, MUS-099, MUS-109, MUS-110, MUS-113, MUS-116, MUS-122, MUS-123, MUS-126, MUS-185, MUS-227. R1.1: MUS-076, MUS-127. R1.3: DIS-067. R2: ACC-107, CLI-041, CLI-045, CLI-054, CLI-098, CLI-102, DIS-055, DIS-167, INT-105, LAT-091, LAT-099, MUS-068, MUS-078, MUS-094, MUS-111, MUS-112, MUS-223, MUS-228.

#### SUR-011 Queue
- **Release:** R1, with the "Up next" and "From" lanes. R1.1 adds
  reordering while shuffled, reshuffling, saving the queue as a playlist,
  the sleep timer and multi-select with drag and drop (MUS-120, MUS-128,
  MUS-125, MUS-076, MUS-063, CLI-062); R1.3 adds the "Continue with" lane
  (MUS-129, DIS-070, MUS-165).
- **Purpose:** Shows and edits what plays next, with the three lanes that
  keep a person's own picks separate from the album and from suggestions
  (MUS-116).
- **Shows:** "Up next" (your picks), "From" (the album or playlist), and,
  from R1.3, an optional "Continue with" lane that is off by default
  (MUS-129, DIS-070, MUS-165). Every item says where it came from (MUS-123). Unplayable items
  are dimmed with the reason (MUS-229). From R2: queue history above the
  current track (MUS-124), the queue switcher for saved queues and the
  music and video contexts (MUS-131), and video items (VID-181).
- **Actions:** Reorder (even while shuffled from R1.1, MUS-120); remove;
  edit the whole queue (MUS-119). From R1.1: save it as a playlist
  (MUS-125); reshuffle the rest (MUS-128); set a sleep timer from the queue
  menu, the same action as in the player's options menu, since MUS-076
  names both (MUS-076). From R1.3: turn the suggestions lane on. In R1,
  removing one row acts at once, and "Clear Up next", "Clear queue" and,
  from R1.1, removing a multi-selection ask first, because nothing can
  bring them back. From R2: undo an edit, which replaces those
  confirmations (MUS-121), and swipe rows to remove them (MUS-065).
- **Form factors:** *TV:* the queue rail below TV Now Playing (CLI-041,
  MUS-223); reorder with the remote. *Phone:* a sheet from the player or the
  bar (CLI-149). *Tablet:* the player side panel in R2. *Web and desktop:*
  the right pane at full height, resizable, keeping durations and album
  names (CLI-060, MUS-119).
- **Serves:** R1: LAT-006, MUS-116, MUS-119, MUS-122, MUS-123, MUS-229. R1.1: CLI-062, MUS-063, MUS-076, MUS-120, MUS-125, MUS-128. R1.3: DIS-070, MUS-129, MUS-165. R2: CLI-041, MUS-065, MUS-124, MUS-131, MUS-223.

#### SUR-012 Lyrics view
- **Release:** R1. R1.1 adds word-by-word timing and a view that stays open
  across tracks (MUS-156, MUS-158).
- **Purpose:** Lyrics that are simply there, free, from the files
  (MUS-154 to MUS-156).
- **Shows:** Embedded lyrics (MUS-154, LIB-067), synced LRC lyrics with the
  current line highlighted (MUS-155), and, from R1.1, word-by-word timing
  where the file has it (MUS-156) and a view that stays open across tracks
  when a person wants it to (MUS-158). From R2: multi-voice TTML (MUS-157, LIB-069), full-screen
  lyrics with tap to seek (MUS-159), lyrics offline (MUS-160), and a "Find
  lyrics" action backed by a plugin (MUS-162, INT-078, LIB-070).
- **Actions:** Scroll, tap a line to seek (R2), keep open (R1.1), find
  lyrics (R2).
- **Form factors:** *TV:* **Proposal:** a lyrics panel inside TV Now
  Playing; no feature row names TV lyrics yet. *Phone:* a panel in the
  player; beside the artwork in landscape (R2). *Tablet:* in the player
  side panel. *Web and desktop:* the right pane or the full content area.
- **Security:** lyrics are parsed into a timed-line model within size
  limits and rendered only as text (SEC-MED-049, SEC-API-090, SEC-CLI-001).
- **Serves:** R1: LIB-067, MUS-154, MUS-155. R1.1: MUS-156, MUS-158. R2: INT-078, LIB-069, LIB-070, MUS-157, MUS-159, MUS-160, MUS-162.

#### SUR-013 Track info sheet
- **Release:** R1.1, with the sheet itself (MUS-114), which moved from R1;
  the R1 rows it shows reach it then, and "Why is this here?" joins it
  (LIB-098). Measured loudness joins in R1.3 (MUS-086, LIB-066).
- **Purpose:** Says everything about a track's file, its tags and how it is
  being played, so a person never has to guess (MUS-114).
- **Shows:** Format and technical details (MUS-021, MUS-032, LIB-063), how
  each field was read and where it came from (MUS-034, MUS-037), MusicBrainz
  IDs (MUS-036, INT-009), the credits panel with roles (MUS-055, MUS-005),
  the gain source and the gain applied, or "estimated", or from R1.3
  "measured" (MUS-084, MUS-089, MUS-090, LIB-065; LIB-066, MUS-086), "Why
  is this here?" with the reasons but no file paths (LIB-098), gapless and
  encoder-delay status (MUS-067, MUS-069), the rating control (MUS-181),
  and "Upgraded on" when a better copy replaced the file (LIB-030). From R2:
  BPM and key (MUS-022), more formats (MUS-033), the source file of a CUE
  album (MUS-041), and the signal path (MUS-103).
- **Actions:** Rate, copy a field, open File info (administrators, from
  R1.2, SUR-088).
- **Form factors:** *TV:* a side sheet from the long-press menu's Info.
  *Phone:* a bottom sheet. *Tablet:* a sheet or the side panel. *Web and
  desktop:* the right pane or a dialog.
- **Serves:** R1.1: INT-009, LIB-030, LIB-063, LIB-065, LIB-098, MUS-005, MUS-021, MUS-032, MUS-034, MUS-036, MUS-037, MUS-055, MUS-067, MUS-069, MUS-084, MUS-087, MUS-089, MUS-090, MUS-099, MUS-114, MUS-181. R1.3: LIB-066, MUS-086. R2: INT-120, MUS-022, MUS-033, MUS-041, MUS-103.

#### SUR-014 Device picker and remote mode
- **Release:** R2.
- **Purpose:** Choose where playback happens and control other players,
  including handing the queue from one device to another (MUS-197,
  CLI-101).
- **Shows:** This device and, on native clients, its output devices
  (MUS-200, CLI-067); the other Gunmetal players signed in to this profile;
  Chromecast targets (MUS-203, CLI-106); and household devices this
  profile may use that another profile is using now, greyed, not
  selectable and labelled only "In use", with no profile name or title.
  Other people's own players never appear, and controlling another
  person's player is Later (ACC-047, SEC-HIS-014, SEC-PRV-022,
  SEC-API-068). In remote mode, the player shows which device it is
  driving and that device's volume (CLI-102).
- **Actions:** Hand off, take over, control remotely, choose an output.
- **Form factors:** *TV:* a target more than a controller; an accept prompt
  appears when another device hands playback to it (VID-144). *Phone:* a
  sheet from the bar or player, and a remote screen for driving the TV
  (VID-145). *Tablet:* as phone. *Web and desktop:* a popover from the bar;
  the output picker (CLI-067) and exclusive output (MUS-102) arrive with
  the desktop shell, Later.
- **Security:** the list comes from session events filtered per recipient,
  so it holds only what this profile may see (SEC-API-016); a handoff
  carries item IDs and the position, never a stream URL or token, and the
  target asks for its own capability URLs (SEC-API-026).
- **Serves:** R2: CLI-067, CLI-101, CLI-102, CLI-106, INT-137, MUS-197, MUS-198, MUS-200, MUS-203, VID-144, VID-145.

#### SUR-015 Add-to-playlist sheet
- **Release:** R1. The duplicate warning arrives in R1.1 (MUS-134).
- **Purpose:** Adds items to a playlist quickly, warning about duplicates
  (MUS-133, MUS-134).
- **Shows:** Recent and pinned playlists, a search field, "New playlist",
  and, from R1.1, the duplicate warning. From R2 the same sheet adds to collections
  (DIS-128).
- **Actions:** Add, create and add, skip duplicates or add anyway (R1.1).
- **Form factors:** *TV:* a side sheet (R2). *Phone and tablet:* a bottom
  sheet. *Web and desktop:* a popover, plus, from R1.1, dragging onto a
  sidebar playlist.
- **Serves:** R1: MUS-133. R1.1: MUS-134. R2: DIS-128.

### Home, library, browse and search

#### SUR-020 Home
- **Release:** R1 (the music Home). R1.2 makes it arrangeable and the same
  on every device, with pinned shortcuts and uncapped rows (DIS-003,
  MUS-049, DIS-007, DIS-013, DIS-009); R1.3 adds "Because you played" rows
  (DIS-061, DIS-062). R2 adds the Watch Home, the kids Home and the
  combined option. R3 adds live TV rows.
- **Purpose:** A start page made only of the household's own media
  (DIS-001), drawn instantly from the synced library (DIS-002), and from
  R1.2 arranged by each person (DIS-003, MUS-049) and the same on every
  device (DIS-007).
- **Shows:** In R1: Continue listening as the first row (MUS-050,
  DIS-020), Recently played (DIS-021), Recently added grouped by album,
  where upgrades do not count as new (DIS-035, DIS-036, DIS-038, MUS-059),
  Loved tracks (MUS-149), and helpful first-run cards and empty states
  (DIS-004). From R1.2: pinned shortcuts at the top (DIS-013) and rows as
  long as you like with "See all" (DIS-009). From R1.3: "Because you
  played" rows (DIS-061) with a reason on every suggestion (DIS-062). From R2: the Music and Watch switch (DIS-017),
  Continue Watching, the "Next to watch" row (the feature map's Next Up,
  DIS-025, and the next film of a started series, DIS-031) and new
  episodes (DIS-024 to DIS-026,
  DIS-029 to DIS-031, DIS-037), the watchlist row (DIS-048), recently released (DIS-040),
  mixes, rediscovery, On this day and charts (MUS-166 to MUS-168, DIS-063
  to DIS-066, DIS-069), plugin rows such as new releases and similar
  artists (MUS-175, MUS-176, DIS-043), seasonal rows (DIS-123), the owner's
  picks (DIS-127), a featured spotlight (DIS-016), Surprise me (DIS-072),
  the kids Home (DIS-152, DIS-154), discovery for restricted profiles
  (ACC-031), and an optional merged Home across servers (CLI-018). From
  R3: On now and starting soon (DIS-041).
- **Actions:** Play, open; from R1.1, dismiss from a Continue row with
  undo (DIS-022, DIS-023); from R1.2, open "See all" and enter the Home
  editor.
- **Form factors:** *TV:* the backdrop follows focus (DIS-164), rows stay
  smooth on cheap sticks (DIS-161, CLI-038), and the TV can keep its own
  denser layout (DIS-008); the launcher rows (SUR-055) mirror it. *Phone:*
  vertical rows with the pinned grid on top. *Tablet:* more tiles per row.
  *Web and desktop:* rows with scroll arrows. The research records that
  Spotify's "Customize Start Screen" idea has 15,697 votes and is marked
  "Not Right Now", and that Plexamp's arrangeable Home needs Plex Pass, so a
  free, arrangeable Home is a real edge.
- **Serves:** R1: DIS-001, DIS-002, DIS-004, DIS-020, DIS-021, DIS-035, DIS-036, MUS-050, MUS-059, MUS-149. R1.2: DIS-007, DIS-009, DIS-013, MUS-049. R1.3: DIS-061, DIS-062. R2: ACC-031, CLI-051, DIS-016, DIS-024, DIS-025, DIS-026, DIS-029, DIS-030, DIS-031, DIS-037, DIS-040, DIS-043, DIS-048, DIS-063, DIS-064, DIS-065, DIS-066, DIS-069, DIS-072, DIS-123, DIS-126, DIS-127, DIS-152, DIS-154, DIS-161, DIS-164, DIS-167, INT-120, LAT-011, LAT-012, MUS-166, MUS-167, MUS-168, MUS-175, MUS-176.

#### SUR-021 Home editor
- **Release:** R1.2, with the arrangeable Home (DIS-003, MUS-049), which
  moved from R1.
- **Purpose:** Lets each person add, remove, rename and reorder Home rows,
  and turn any filter into a row (DIS-003, MUS-049).
- **Shows:** The rows in order with drag handles, an add-row sheet listing
  row types and saved rules, and a per-library "keep off Home" switch
  (DIS-012). From R2: row settings for artwork shape and hiding finished
  items (DIS-010, DIS-011), per-device overrides (DIS-008), and "reset to
  household default" (DIS-005). From R3: live TV rows by choice (LIV-077).
- **Actions:** Add a row (opening SUR-027 for a new rule, from R1.3),
  rename, reorder, remove, reset.
- **Form factors:** *TV:* **Proposal:** reorder and hide rows with the
  remote; creating rules happens on another device. *Phone and tablet:* an
  edit mode with drag handles. *Web and desktop:* inline edit mode.
- **Serves:** R1.2: DIS-003, DIS-012, MUS-049. R2: DIS-005, DIS-008, DIS-010, DIS-011.

#### SUR-022 Library
- **Release:** R1. R1.1 adds playlist files found in music folders,
  remembered sorts, pinned playlists and multi-select (LIB-192, MUS-056,
  MUS-139, MUS-063, CLI-062); R1.3 adds the Folders tab, smart playlists
  and the Spoken word list (LIB-008, MUS-028, DIS-106, DIS-121, LAT-010).
  Watch tabs and Downloads are R2.
- **Purpose:** The whole collection, by kind, for people who browse rather
  than search.
- **Shows:** Music tabs (**Proposal** for the set and order): Artists,
  Albums, Songs, Playlists, Folders from R1.3 (LIB-008, MUS-028, DIS-106)
  and Loved (DIS-046), with, from R1.3, a plain Spoken word list when the
  admin has flagged spoken-word folders (LAT-010). A library switcher when the person can see
  several libraries (MUS-027, LIB-004). During the first scan the library
  fills in live (MUS-043, LIB-021), and an empty library explains what is
  happening (ADM-031). From R1.1, playlist files found in music folders
  appear as playlists (LIB-192). From R2: playlists written through the API
  (INT-138), Watch tabs for films, shows, collections and
  home videos (DIS-118, LIB-010), music videos (LAT-095), playlist folders
  (MUS-136), video playlists (VID-183), service playlists (INT-109), and
  Downloads on devices that download. Later: the household blend
  (DIS-184), if the owner adds household activity features.
- **Actions:** Open a tab, open an item, and use the shared list controls
  (SUR-023).
- **Form factors:** *TV:* each library is its own rail entry (CLI-037), and
  its grid has a letter column. *Phone:* tabs as chips under the switch.
  *Tablet:* as phone, with more columns. *Web and desktop:* the sidebar
  lists playlists (MUS-132) and, from R1.1, pinned playlists (MUS-139),
  and the tabs sit at the top of the content.
- **Serves:** R1: ADM-031, DIS-046, LIB-004, LIB-021, MUS-027, MUS-043, MUS-132, MUS-149, MUS-208. R1.1: CLI-062, LIB-192, MUS-056, MUS-063, MUS-139. R1.3: DIS-106, DIS-121, LAT-010, LIB-008, MUS-028. R2: DIS-072, DIS-118, DIS-124, INT-109, INT-138, LAT-095, LIB-010, LIB-150, MUS-136, VID-183. Later: DIS-184.

#### SUR-023 Lists, grids, rows and browse controls
- **Release:** R1: endless lists and grids, track rows, hearts, tiles,
  quality badges and the sort menu. R1.1 adds the explicit badge, the view
  toggle, the filter sheet, the alphabet jump and find in a list (MUS-047,
  DIS-107, DIS-102, DIS-103, DIS-105, DIS-101, CLI-040, DIS-091), with the
  release-type, date, mood and role fields they use (MUS-010, MUS-013,
  MUS-019, MUS-005); R1.3 adds the marker on items waiting in the review
  queue (LIB-099).
- **Purpose:** The shared components that every list and grid is made of,
  so sorting, filtering, jumping and row content behave the same
  everywhere.
- **Shows:** Endless smooth lists and grids (DIS-100); track rows with
  every credited artist linked, play counts and, from R1.1, the explicit
  badge (MUS-001, MUS-002, MUS-182; MUS-047); hearts on rows (MUS-180); tiles with
  artwork sized for the device (LIB-142, MUS-040, LIB-134) and quality
  badges (LIB-146); and a sort menu with natural sort (DIS-104, MUS-020),
  which from R1.1 remembers its choice and sorts by original or release
  date (MUS-056, MUS-013). From R1.1: a view toggle for grid, list and
  compact (DIS-107, MUS-057); a filter sheet with chips (DIS-102),
  remembered per view (DIS-103) and savable (DIS-105); an alphabet jump
  (DIS-101, MUS-058, CLI-040); and find inside a list (DIS-091). From R2: the "Downloaded" filter (CLI-026), language and
  quality filters (DIS-113), progress on cards (DIS-032), box sets that
  collapse (DIS-116), a "plays directly here" filter (VID-015), and "Edited"
  and "Unmatched" chips (LIB-174, LIB-102).
- **Actions:** Sort; from R1.1, filter, save a filter as a view (and as a
  Home row from R1.2), switch view, jump to a letter and find in the list.
- **Form factors:** *TV:* a letter column on every grid (DIS-163, CLI-040)
  and virtualised grids that stay smooth on the cheapest supported stick
  (CLI-038). *Phone:* a fast scroller at the edge (CLI-040, from R1.1).
  *Tablet:* as phone. *Web and desktop:* from R1.1, type to jump, plus a
  letter column on long lists.
- **Serves:** R1: DIS-035, DIS-038, DIS-051, DIS-100, DIS-104, LIB-044, LIB-053, LIB-063, LIB-142, LIB-146, MUS-001, MUS-002, MUS-017, MUS-020, MUS-021, MUS-040, MUS-059, MUS-180, MUS-182, MUS-229. R1.1: CLI-026, CLI-040, DIS-047, DIS-091, DIS-101, DIS-102, DIS-103, DIS-105, DIS-107, DIS-163, LIB-052, LIB-054, MUS-005, MUS-010, MUS-013, MUS-019, MUS-047, MUS-056, MUS-057, MUS-058. R1.3: LIB-099. R2: ACC-024, CLI-023, CLI-038, DIS-032, DIS-037, DIS-040, DIS-113, DIS-116, DIS-150, DIS-161, LAT-013, LAT-091, LIB-087, LIB-102, LIB-174, LIB-180, LIB-197, MUS-078, VID-015.

#### SUR-024 Artist page
- **Release:** R1. R1.1 adds release-type sections and release groups, role
  tabs, the artist image, "Your top tracks", ratings, sorting the
  discography and one page across libraries (MUS-010, MUS-008, MUS-005,
  MUS-024, DIS-071, DIS-047, MUS-053, LIB-187); R1.3 adds More like this
  and, for administrators, merge, split and alias (DIS-060, MUS-007,
  LIB-041).
- **Purpose:** One page per artist that gets credits, roles and editions
  right (MUS-051), across every library the person can see from R1.1
  (LIB-187).
- **Shows:** The header with a disambiguation line for same-name artists
  (MUS-006, LIB-040), and the local artist image from R1.1 (MUS-024); the
  albums, in sections by release type with one tile per release group
  from R1.1 (MUS-010, LIB-048, MUS-008, LIB-047); "Appears on" (MUS-004,
  MUS-011, LIB-042, LIB-049); from R1.1, role tabs for composer,
  conductor, producer and the rest (MUS-005, LIB-043); "All songs"
  (MUS-052); "Your top tracks" from R1.1 (DIS-071); your plays and loves
  (DIS-050, DIS-045, DIS-051), with ratings from R1.1 (DIS-047); and More
  like this from R1.3 (DIS-060). From R2: biography and photos (MUS-025, LIB-113, LIB-127), a
  follow button with new releases (MUS-176, DIS-043, INT-050), "where to
  buy" for missing albums (MUS-231), and a videos shelf (LAT-092, LAT-094,
  LIB-189, MUS-029). From R1.1 the artist page with role tabs is also
  the person page for music (DIS-087).
- **Actions:** Play, shuffle, love; sort the discography from R1.1
  (MUS-053); pin from R1.2; start radio from R1.3; for administrators,
  merge, split and alias from R1.3 (MUS-007, LIB-041).
  From R2: follow, download, edit.
- **Form factors:** *TV:* Play and Shuffle take focus first (**Proposal**).
  *Phone:* a collapsing header. *Tablet:* the header beside the sections in
  landscape. *Web and desktop:* a wide header with sections below.
- **Serves:** R1: DIS-045, DIS-050, DIS-051, LIB-040, LIB-042, LIB-049, MUS-001, MUS-003, MUS-004, MUS-006, MUS-011, MUS-051, MUS-052, MUS-180, MUS-182. R1.1: DIS-047, DIS-071, DIS-087, LIB-043, LIB-047, LIB-048, LIB-187, MUS-005, MUS-008, MUS-010, MUS-024, MUS-053. R1.3: DIS-060, LIB-041, MUS-007. R2: DIS-043, LAT-092, LAT-094, LIB-113, LIB-127, LIB-189, MUS-025, MUS-029, MUS-176, MUS-231.

#### SUR-025 Album page
- **Release:** R1. R1.1 adds original and release dates, the "Other
  versions" row, album details such as label and mood, and rating
  (MUS-013, MUS-008, LIB-054, DIS-047); R1.3 adds the folder breadcrumb,
  More like this and, for administrators, merge and split (LIB-008,
  DIS-060, LIB-058). Work headings arrive in R2 (LIB-056, MUS-014).
- **Purpose:** An album as it was released: discs, works, editions and
  credits (MUS-054).
- **Shows:** Artwork from the file or folder (MUS-039, LIB-135), the album
  artist apart from track artists (MUS-003, LIB-039) and linked credits in
  the header (MUS-001, MUS-002, LIB-037), disc headers with disc titles
  and "Play disc" (MUS-012, LIB-050), technical badges (MUS-021, LIB-063)
  and your plays (DIS-050); from R1.1, original and release dates (MUS-013,
  LIB-052), an "Other versions" row (MUS-008, LIB-047) and album details
  such as label and mood (LIB-054); and from R1.3 a breadcrumb to the
  folder (LIB-008). From R2: per-disc art (LIB-137), tracks grouped under
  work headings from tags (LIB-056, MUS-014), CUE-sheet albums that look
  like any other album (MUS-041, LIB-071), and "where to buy" (MUS-231).
- **Actions:** Play, shuffle, queue, love, add to playlist; rate from
  R1.1; for administrators, merge and split from R1.3 (LIB-058). From R2: download and edit.
- **Form factors:** *TV:* artwork left, tracks right, Play focused first
  (**Proposal**). *Phone:* artwork above the track list. *Tablet and web:*
  artwork beside the track list.
- **Serves:** R1: DIS-045, DIS-050, DIS-051, LIB-030, LIB-037, LIB-039, LIB-050, LIB-063, LIB-134, LIB-135, LIB-146, MUS-001, MUS-002, MUS-003, MUS-011, MUS-012, MUS-021, MUS-039, MUS-054, MUS-180. R1.1: DIS-047, LIB-047, LIB-052, LIB-054, MUS-008, MUS-013. R1.3: DIS-060, LIB-008, LIB-058. R2: LIB-056, LIB-071, LIB-137, MUS-014, MUS-041.

#### SUR-026 Playlist page
- **Release:** R1 for manual playlists (MUS-132). R1.1 adds the automatic
  cover, search, sort and filter inside a playlist, M3U import and export,
  pinning and loving (MUS-137, MUS-135, MUS-140, MUS-139); R1.3 adds smart
  playlists (MUS-143, MUS-144). Playlists written through the API appear
  from R2 (INT-138).
- **Purpose:** Manual and smart playlists, with the tools to manage long
  ones (MUS-132, MUS-143).
- **Shows:** The tracks; from R1.1, an automatic cover (MUS-137) and
  search, sort and filter inside the playlist (MUS-135); and from R1.3,
  for a smart playlist, its rules and when it last refreshed (MUS-143,
  DIS-121). From R2: playlists written through the API, which appear like
  any other (INT-138), a custom image
  (MUS-138), collaborators and sharing (ACC-091, MUS-150), offline editing
  (MUS-152), "Keep downloaded" (CLI-080) and "keep out of my taste"
  (DIS-057).
- **Actions:** Play, shuffle, reorder, remove, rename; from R1.1, import
  and export M3U (MUS-140), pin and love (MUS-139); from R1.3, edit rules
  (opens SUR-027).
- **Form factors:** *TV:* playing and light reordering only; editing
  happens on phone or web (**Proposal**; CLI-052 puts full library
  management on the phone). *Phone and tablet:* drag handles in an edit
  mode. *Web and desktop:* drag and drop and multi-select, from R1.1
  (CLI-062).
- **Serves:** R1: MUS-132. R1.1: MUS-135, MUS-137, MUS-140. R1.3: MUS-143, MUS-144. R2: ACC-091, CLI-052, DIS-057, MUS-152.

#### SUR-027 Rule editor
- **Release:** R1.3, with the rule language and smart playlists (DIS-119,
  MUS-143), which moved from R1.
- **Purpose:** One visual editor for the one rule language (DIS-119) behind
  smart playlists, Home rows and saved filters, with a live preview
  (DIS-120, MUS-144).
- **Shows:** Conditions on rich fields (MUS-145, MUS-017), limits, sorts
  and percentages (MUS-146, DIS-122), and a live list of what matches. From
  R2 the same editor builds snapshot refreshes (MUS-147), seasonal rows
  (DIS-123), BPM, key and personal-tag conditions (MUS-022, MUS-023), smart
  collections (LIB-166), download rules (MUS-212) and child-profile rules
  (ACC-028). In R3 recording rules use it too (LIV-119).
- **Actions:** Add, change and remove conditions; preview; save as a
  playlist, a Home row or a filter.
- **Form factors:** *TV:* not offered (**Proposal**); rules made elsewhere
  still apply on the TV. *Phone:* a full-height sheet with stacked
  conditions. *Tablet:* a sheet with the preview beside it. *Web and
  desktop:* a dialog with the preview beside it. The research notes that
  Navidrome makes people write JSON for smart playlists and that Plexamp's
  are good; the editor has to be at least as easy as Plexamp's.
- **Serves:** R1.3: DIS-119, DIS-120, DIS-121, DIS-122, MUS-017, MUS-144, MUS-145, MUS-146. R2: DIS-123, MUS-022, MUS-023, MUS-147.

#### SUR-028 Browse pages
- **Release:** R1 (genre pages). R1.1 adds mood, style and label pages and
  role pages such as Composers (MUS-019, LIB-043).
- **Purpose:** Pages built from the household's own tags: genre, mood,
  decade, label, and people by role (DIS-109, MUS-060).
- **Shows:** Genre pages from multi-valued genres (MUS-017, LIB-053), and,
  from R1.1, mood, style and label pages (MUS-019) and a Composers page and
  other role pages (LIB-043). From R2: personal tags (MUS-023), stations (DIS-068),
  studio, network and country pages (DIS-114), genre, mood and decade rows
  (DIS-063), and a genre page that notes what was merged into it (DIS-108).
- **Actions:** Open, play; pin from R1.2; start radio from R1.3.
- **Form factors:** *TV:* reached from Home rows and search rather than the
  rail (**Proposal**). *Phone:* a "Browse" section at the top of Search
  (**Proposal**). *Tablet and web:* the same, plus a Browse link in the
  Library header.
- **Serves:** R1: DIS-109, LIB-053, MUS-017, MUS-060. R1.1: LIB-043, MUS-019. R2: DIS-063, DIS-068, DIS-108, DIS-114, LIB-102, MUS-023.

#### SUR-029 History page
- **Release:** R1.
- **Purpose:** Each person's listening record, by date, under their control
  (MUS-183, DIS-050).
- **Shows:** Plays grouped by day with the device each came from, including
  offline plays merged later (CLI-093). From R2: which app recorded a play
  through an adapter (INT-093) and a charts link (DIS-066).
- **Actions:** Remove a play, a range of dates or everything (MUS-184,
  ACC-118, DIS-052), play again, export (in SUR-078).
- **Form factors:** *TV:* read-only (**Proposal**), and on a household TV
  an adult profile's history stays hidden until that adult's PIN or phone
  unlocks it (SEC-IAM-110). *Phone, tablet, web:* full.
- **Security:** each person's own history only; no other person, admins
  included, can open it, except a managed profile's designated guardians
  (SEC-PRV-022, SEC-PRV-025, SEC-PRV-029). A removed play leaves the
  database, logs, indexes and caches within 24 hours and stays removed
  after an older backup is restored (SEC-PRV-049).
- **Serves:** R1: ACC-118, CLI-093, DIS-021, DIS-050, DIS-052, MUS-183. R2: DIS-066, INT-093.

#### SUR-030 Hidden page
- **Release:** R1.1, with dismissing and undo (DIS-023), which moved from
  R1. Hide and snooze arrive in R2.
- **Purpose:** One place to see and undo what a person has dismissed or
  hidden (DIS-023).
- **Shows:** Items dismissed from Continue rows; from R2, hidden and snoozed
  items and their snooze dates (DIS-054, MUS-170).
- **Actions:** Restore, change a snooze.
- **Form factors:** The same list on every form factor.
- **Serves:** R1.1: DIS-023. R2: DIS-054, MUS-170.

#### SUR-031 Statistics page
- **Release:** R2.
- **Purpose:** Charts by period and a year in review at any time, computed
  from the person's own log (MUS-186, MUS-187, DIS-183).
- **Shows:** Top artists, albums and tracks for a chosen period; a year in
  review.
- **Actions:** Change the period, play a chart, save it as a playlist.
- **Form factors:** *TV:* read-only cards. *Phone, tablet, web:* full.
- **Serves:** R2: MUS-186, MUS-187.

#### SUR-032 Search
- **Release:** R1. R1.1 adds tags, genres and moods, people by role, the
  scope selector and recent searches (DIS-086, DIS-087, DIS-088, DIS-089).
- **Purpose:** One search box over everything a person can see, answered on
  the device (DIS-083, DIS-084).
- **Shows:** Results grouped by type with type chips (MUS-061, DIS-083)
  and forgiving matches (DIS-085). From R1.1: tags, genres and moods
  (DIS-086), people by role (DIS-087), a scope selector (DIS-088) and
  recent searches when the box is empty (DIS-089). From R2: transliteration (DIS-092), a Lyrics chip
  (DIS-093, MUS-161), voice search (DIS-094, CLI-044), duplicates collapsed
  (DIS-095), and kind chips (LAT-013). From R3: a Live TV chip and guide
  results (DIS-096, LIV-072).
- **Actions:** Type, scope (R1.1), open, play, queue, save a search as a
  rule (R3, LIV-119).
- **Form factors:** *TV:* a rail entry with an on-screen keyboard and the
  remote's microphone (CLI-044). *Phone:* the Search tab, with Browse above
  the results when empty (**Proposal**). *Tablet:* as phone. *Web and
  desktop:* a field at the top of the sidebar, reached by Tab like any
  control in R1; from R2 also a keyboard shortcut and the command palette
  (DIS-090, CLI-061), which the feature map places in R2.
- **Security:** recent searches live only on the device and can be cleared;
  the server never stores or logs search terms (SEC-PRV-004). The index on
  the device holds only what this profile may see, artwork and search terms
  included (SEC-CLI-020).
- **Serves:** R1: CLI-022, DIS-083, DIS-084, DIS-085, MUS-061. R1.1: DIS-086, DIS-087, DIS-088, DIS-089. R2: CLI-044, DIS-092, DIS-093, DIS-094, DIS-095, LAT-013, MUS-161. R3: LIV-072, LIV-119.

### Watching

Every surface in this group is R2. On TV these are the main surfaces; on a
phone they share the app with music under the Music and Watch switch.

#### SUR-040 Title page
- **Release:** R2.
- **Purpose:** The page for a film, and for a music video or concert
  (LAT-096): what it is, whether it will play directly here, and every
  version, extra and track it has.
- **Shows:** Backdrop and poster (LIB-144); a "plays directly here" badge
  before pressing play (VID-015); the version and edition switcher
  (LIB-151); resume position and progress per edition (VID-118, DIS-024,
  LIB-155); content rating (LIB-121); both the original and translated
  title where wanted (LIB-185); technical and file information (VID-170,
  LIB-087); local trailers and an Extras row with real titles (VID-161,
  VID-162, LIB-157, LIB-159, DIS-073); chapter thumbnails (VID-097); the
  cast row (LIB-188); "Part of" a collection (DIS-133); provider IDs
  (INT-010); a watchlist button (DIS-048); and the loves, ratings, plays and
  More like this rows that music pages already have (DIS-045, DIS-047,
  DIS-050, DIS-051, DIS-060). Recordings from R3 appear here like any other
  film (LIB-191).
- **Actions:** Play, resume, choose a version, add to watchlist, download,
  make a smaller copy (VID-026), add a subtitle file (VID-075), request or
  open in Seerr (INT-126), and for a guardian, allow one title for a child
  (DIS-149).
- **Form factors:** *TV:* a full-screen backdrop with Play focused; extras
  and cast as rows below. *Phone:* a scrolling page with a sticky Play
  button. *Tablet:* poster and details side by side. *Web and desktop:* a
  wide header with rows below.
- **Serves:** R2: DIS-024, DIS-034, DIS-045, DIS-047, DIS-048, DIS-050, DIS-051, DIS-060, DIS-073, DIS-133, DIS-149, INT-010, INT-126, LAT-096, LIB-087, LIB-121, LIB-144, LIB-146, LIB-155, LIB-157, LIB-159, LIB-185, LIB-188, VID-001, VID-013, VID-015, VID-026, VID-075, VID-097, VID-118, VID-161, VID-162, VID-170. R3: LIB-191.

#### SUR-041 Show, season and episode pages
- **Release:** R2.
- **Purpose:** Television as it aired, with the right episode always ready
  to start (DIS-034).
- **Shows:** The show page with seasons, or one flat list for single-season
  shows (DIS-115, LIB-081); season pages with season art (LIB-145), specials
  placed where they aired (LIB-077), missing-episode placeholders (LIB-200)
  and files that hold several episodes marked (LIB-079); episode lists with
  spoiler protection (DIS-117); editions and extras at every level
  (LIB-152, LIB-158); rewatch and specials handling (DIS-030); and show
  settings for episode order (LIB-103). Recordings join these pages in R3
  (LIB-191).
- **Actions:** Play the right next episode, shuffle a show or season
  (VID-182), stop following (DIS-027), follow for alerts (INT-050),
  download a season or the next episodes (CLI-087, CLI-088).
- **Form factors:** *TV:* season tabs across the top, episodes as a focused
  list. *Phone:* season picker and a vertical episode list. *Tablet and
  web:* seasons beside episodes.
- **Serves:** R2: CLI-087, CLI-088, DIS-027, DIS-030, DIS-034, DIS-115, DIS-117, INT-050, LIB-077, LIB-079, LIB-081, LIB-103, LIB-145, LIB-152, LIB-158, LIB-200, VID-182. R3: LIB-191.

#### SUR-042 Collection page and editor
- **Release:** R2.
- **Purpose:** Manual, smart and automatic collections of films and shows
  (LIB-163, LIB-166, DIS-129).
- **Shows:** Members in the collection's own order (DIS-131, LIB-164),
  films and shows together (DIS-132, LIB-169), child collections (DIS-130,
  LIB-168), collection artwork (LIB-165), and "Watch the next film" for a
  film series the person has started (DIS-031). Collections written through the API appear here
  (INT-139).
- **Actions:** Play in order, shuffle (VID-182), edit membership and order,
  turn a filter into a smart collection (LIB-166), set artwork.
- **Form factors:** *TV:* view and play; editing on phone or web
  (**Proposal**). *Phone, tablet, web:* full editing.
- **Serves:** R2: DIS-031, DIS-124, DIS-128, DIS-129, DIS-130, DIS-131, DIS-132, INT-139, LIB-163, LIB-164, LIB-165, LIB-166, LIB-168, LIB-169.

#### SUR-043 Person page
- **Release:** R2. From R1.1, the artist page's role tabs serve music
  people (DIS-087).
- **Purpose:** Everything one person made or appeared in, by role (LIB-188,
  LAT-002).
- **Shows:** Photo, roles as tabs (actor, director, writer, composer and so
  on), and their films, shows and recordings in the library.
- **Actions:** Play, open, filter by role.
- **Form factors:** As the artist page (SUR-024).
- **Serves:** R2: DIS-087, LIB-188.

#### SUR-044 Watchlist page
- **Release:** R2.
- **Purpose:** Films and shows saved for later (DIS-048).
- **Shows:** The list, sortable and filterable like any list, and, where
  Seerr is connected, items requested through it (INT-125).
- **Actions:** Play, remove, reorder, turn into a Home row.
- **Form factors:** The same list on every form factor.
- **Serves:** R2: DIS-048, INT-125.

#### SUR-045 Pre-play sheet and version picker
- **Release:** R2.
- **Purpose:** Says before playback which version will play and why, and
  lets a person change it, pick tracks or play privately (VID-013,
  VID-014).
- **Shows:** The chosen version and the reason, with "Best for this device"
  marked (LIB-154); every version with HDR and Dolby Vision badges, audio
  format labels, source library and parts (VID-036, VID-039, VID-055,
  LIB-149, LIB-150, LIB-080, LIB-153); a warning when the connection is
  slower than the file, with honest choices (VID-024); audio and subtitle
  pickers (VID-053, VID-054); resume per version (VID-120); the private
  viewing switch (VID-130); and "play the smaller copy" where one exists
  (VID-026).
- **Actions:** Play, choose a version, choose tracks, play privately.
- **Form factors:** **Proposal:** the sheet opens only when there is a real
  choice or a warning; otherwise Play starts at once. *TV:* a side sheet.
  *Phone:* a bottom sheet. *Tablet and web:* a dialog or popover.
- **Serves:** R2: LIB-080, LIB-087, LIB-149, LIB-150, LIB-151, LIB-153, LIB-154, VID-002, VID-013, VID-014, VID-015, VID-024, VID-026, VID-036, VID-039, VID-053, VID-054, VID-055, VID-120, VID-130.

#### SUR-046 Video player
- **Release:** R2.
- **Purpose:** Plays the original wherever it can, through libmpv on native
  clients and the browser player on the web (VID-001, VID-176, VID-177),
  with every subtitle rendered on the client (VID-069 to VID-071).
- **Shows:** The picture; a transport bar with time remaining and "ends at"
  (VID-106), skip buttons at configurable intervals (VID-101) and a speed
  indicator (VID-133); a seek bar with chapter names and scrub previews
  (VID-096, VID-098, LIB-096) that seeks without restarting the stream
  (VID-008); the skip-intro and skip-credits button driven by markers
  (VID-110 to VID-112, VID-114, LIB-094); the queue panel, which the feature
  map calls "Player > Up next" and which is labelled "Queue" on screen, with
  the Up next lane of the viewer's picks and the From lane of the season or
  collection (VID-181); a loading
  state that says what it is doing (VID-012); and the same controls offline
  for downloaded titles (VID-175).
- **Actions:** Play, pause, seek, skip markers, change speed (VID-131),
  hold for double speed (VID-132), zoom and aspect (VID-137), frame step on
  a keyboard (VID-103), sleep timer (VID-126), report a wrong marker
  (VID-116), listen only (VID-065), picture-in-picture (VID-134), hand off
  (VID-144), and open the player sheets (SUR-047) and the information
  overlay (SUR-048).
- **Form factors:** *TV:* remote, keyboard and controller keys (VID-105,
  CLI-045); no touch gestures. *Phone:* double-tap and swipe to seek
  (VID-102), brightness and volume gestures (VID-138), picture-in-picture
  (CLI-056). *Tablet:* as phone. *Web and desktop:* keyboard shortcuts with
  a help overlay (VID-105). Every player surface works with a screen reader
  and offers reduced motion and large targets (VID-140, VID-141).
- **Serves:** R2: CLI-045, CLI-055, CLI-056, CLI-072, CLI-098, LIB-094, LIB-096, VID-001, VID-008, VID-012, VID-065, VID-069, VID-070, VID-071, VID-096, VID-098, VID-101, VID-102, VID-103, VID-105, VID-106, VID-110, VID-111, VID-112, VID-114, VID-116, VID-126, VID-132, VID-133, VID-134, VID-137, VID-138, VID-140, VID-141, VID-175, VID-176, VID-177, VID-181.

#### SUR-047 Player sheets
- **Release:** R2.
- **Purpose:** The quick menus inside the video player: tracks, subtitles,
  audio, quality, chapters and speed.
- **Shows:** The tracks sheet with clear labels, converted tracks marked,
  external audio files and "use for this series" (VID-052, VID-054, VID-055,
  VID-063, VID-064, VID-004, LIB-092). The subtitle quick menu with sidecar
  files, a secondary track, forced and SDH labels, offset with growing
  steps and "save for series", automatic re-timing, search through a
  provider plugin, "add file", and restyling with a live preview (VID-068,
  VID-072 to VID-075, VID-081 to VID-085, VID-087, VID-088, VID-091,
  VID-092, LIB-091, LIB-093, INT-079, INT-122, CLI-143). The audio quick
  menu with dialogue lift, volume boost and audio delay (VID-059 to
  VID-061). The quality sheet that says what "Original" is and shows
  transcode tiers only when allowed (VID-025, VID-005). The chapters panel
  (VID-095, VID-097, LIB-090). The speed control (VID-131). The cast
  subtitle menu while casting (CLI-108).
- **Actions:** Choose, adjust, save per series, search, add a file.
- **Form factors:** *TV:* side sheets, with offset and delay also on remote
  keys (VID-061, VID-083). *Phone:* bottom sheets. *Tablet and web:*
  popovers over the player.
- **Serves:** R2: CLI-108, CLI-143, INT-079, INT-122, LIB-090, LIB-091, LIB-092, LIB-093, VID-004, VID-005, VID-025, VID-052, VID-054, VID-055, VID-059, VID-060, VID-061, VID-063, VID-064, VID-068, VID-072, VID-073, VID-074, VID-075, VID-081, VID-082, VID-083, VID-084, VID-085, VID-087, VID-088, VID-091, VID-092, VID-095, VID-097, VID-131.

#### SUR-048 Playback information overlay
- **Release:** R2.
- **Purpose:** Says plainly how the file is being played and why: direct,
  remuxed, audio converted, or transcoded (VID-168, VID-169).
- **Shows:** The delivery path and its reason (VID-001, VID-003 to
  VID-005), the decision inputs from the core (VID-002), codecs including
  AV1 (VID-016), HDR, Dolby Vision and their fallbacks (VID-036, VID-039,
  VID-040, VID-044), passthrough state (VID-057), audio labels (VID-055),
  and the device capability report (CLI-047).
- **Actions:** Copy the details; open the playback diagnostic bundle from
  an error (VID-174, in SUR-049).
- **Form factors:** The same overlay on every client; on TV it opens from a
  remote key and closes with Back.
- **Serves:** R2: CLI-047, VID-001, VID-002, VID-003, VID-004, VID-005, VID-016, VID-036, VID-039, VID-040, VID-044, VID-055, VID-057, VID-168, VID-169.

#### SUR-049 Player cards and prompts
- **Release:** R2.
- **Purpose:** The cards that appear over or after playback: resume,
  autoplay, limits, errors and messages.
- **Shows:** The resume prompt with a small rewind (VID-118, VID-119), also
  for long audio (LAT-011); the post-play screen, which counts a film as
  watched when the credits start (VID-122); the autoplay countdown that
  handles gaps, specials and spoilers (VID-123, VID-127, VID-128); "Are you
  still watching?" (VID-125); the error card with alternatives and a
  diagnostic bundle (VID-010, VID-169, VID-174); the stream-limit card
  (VID-173, ACC-075); messages about a quality cap, transcode rights, a
  blocked time or an administrator's stop (VID-028, ACC-043, ACC-108,
  ACC-032, VID-172); a notice when a new subtitle arrives (VID-076); and
  the queue undo notice (MUS-121).
- **Actions:** Resume or start over, play next now, cancel autoplay,
  choose an alternative, send a diagnostic bundle.
- **Form factors:** *TV:* cards take focus only when they need an answer
  (**Proposal**). *Phone, tablet, web:* cards over the player.
- **Serves:** R2: ACC-032, ACC-043, ACC-075, ACC-108, LAT-011, MUS-121, VID-010, VID-028, VID-076, VID-118, VID-119, VID-122, VID-123, VID-125, VID-127, VID-128, VID-169, VID-172, VID-173, VID-174.

### Devices, offline and the operating system

#### SUR-050 Downloads
- **Release:** R2.
- **Purpose:** Free offline copies that manage themselves under a storage
  cap (CLI-078 to CLI-081). The research records offline as Jellyfin's
  most-voted request (1,820 votes) and that Plex and Emby charge for
  downloads.
- **Shows:** Download toggles on album, artist, playlist, film and episode
  pages (CLI-078, CLI-084, MUS-210); the Downloads screen with progress,
  size and clear status (CLI-089, MUS-215); rules such as the whole music
  library, a smart playlist, "keep what I played" and "keep the next
  episodes" (CLI-079 to CLI-081, MUS-211, MUS-212, CLI-088); download
  options for original or remuxed video and Opus music (CLI-085, MUS-213);
  and how long downloads keep working without contact (CLI-095).
- **Actions:** Download, remove, pause, add a rule, start a download on
  another of your devices (CLI-097).
- **Form factors:** *TV:* not offered (**Proposal**; TVs stream on the home
  network). *Phone:* the first entry inside Library (**Proposal**), plus
  toggles on pages. *Tablet:* as phone. *Web and desktop:* not offered: the
  web client cannot hold downloads (MUS-217 is Later), and the desktop
  shell is Later (SEC-TM-074). Progress continues in system notifications
  (SUR-053).
- **Security:** downloads play offline only under a server-signed grant
  bound to the device key, 30 days by default within the admin's range;
  expiry never interrupts what is playing, deletes nothing, and the screen
  says "Connect to your server once to keep listening offline"
  (SEC-IAM-054, SEC-CLI-036). Downloads stay in app-private internal
  storage, out of backups and media indexing, with no SD-card option until
  an encrypted format exists (SEC-CLI-035, SEC-CLI-072). A revoked device
  deletes its downloads at its next contact (SEC-IAM-053).
- **Serves:** R2: CLI-078, CLI-079, CLI-080, CLI-081, CLI-084, CLI-085, CLI-089, CLI-095, CLI-097, MUS-210, MUS-211, MUS-212, MUS-215.

#### SUR-051 Cast controls
- **Release:** R2.
- **Purpose:** Play to a Chromecast from Android and the web (CLI-106,
  VID-146).
- **Shows:** The cast button, the cast controls while casting, subtitle
  choice on the cast device (VID-149), and what happens when the receiver
  cannot decode the file (CLI-111).
- **Actions:** Start and stop casting, control playback, pick subtitles.
- **Form factors:** *TV:* not offered; the TV is a target. *Phone and
  tablet:* the cast button in the player and the bar; controls also in the
  notification (CLI-112, in SUR-053). *Web and desktop:* the cast button in
  the player (browser support for casting is unverified outside
  Chromium-based browsers).
- **Serves:** R2: CLI-106, CLI-111, VID-146, VID-149.

#### SUR-052 Mini player window
- **Release:** Later, with the desktop shell. The first draft placed it in
  R2; the security baseline's release scope puts the desktop shell in Later
  (SEC-TM-074), its controls are SEC-CLI-069 (Later), and the feature map
  now places these rows Later too (owner to confirm). The ID is kept so the
  surface needs no new number when the shell arrives.
- **Purpose:** A small always-on-top player in the desktop shell, sharing
  the main window's queue (CLI-065, MUS-082).
- **Shows:** Artwork, title, transport, love.
- **Actions:** Control playback, return to the main window.
- **Form factors:** Desktop shell only. Until it exists, the web client's
  bar is the mini player on every desktop.
- **Security:** the shell loads only bundled content, keeps renderers
  isolated and checks every IPC message (SEC-CLI-069).
- **Serves:** Later: CLI-014, CLI-065, MUS-082.

#### SUR-053 System media controls
- **Release:** R1 through the browser's media controls (MUS-073, CLI-070).
  R2 natively.
- **Purpose:** Control playback from outside the app: the lock screen, the
  notification shade, the operating system's media panel, media keys,
  Bluetooth and car displays.
- **Shows:** Artwork, title, transport and position, as the platform
  allows. From R2: native lock-screen and notification controls with
  background playback (MUS-074, CLI-069), video and cast controls in the
  notification (VID-135, VID-151, CLI-112), the platform's media keys and
  now-playing panel for video (VID-139), system picture-in-picture
  (VID-134), local
  notifications such as "downloads finished" (CLI-076), and download
  progress (CLI-090). From R3: reminders and recording alerts (LIV-075,
  LIV-111, LIV-155). The desktop shell's own media panels (MPRIS, Windows
  media controls, macOS Now Playing) are Later with the shell (CLI-063,
  MUS-081, SEC-TM-074).
- **Actions:** As the platform allows.
- **Form factors:** *TV:* the Android TV system media controls where they
  exist (unverified). *Phone and tablet:* lock screen and notification.
  *Web and desktop:* the browser's media session, which the OS media panel
  and media keys reach (CLI-070).
- **Security:** notifications other than active media controls show no
  titles, other people's names or device details on the lock screen
  (SEC-CLI-062). From R2, push messages carry no content and only wake the
  app, which fetches the details over its own connection (SEC-OPS-036,
  SEC-PRV-056).
- **Serves:** R1: CLI-070, MUS-073. R2: CLI-069, CLI-072, CLI-076, CLI-090, CLI-112, MUS-074, VID-065, VID-134, VID-135, VID-139, VID-151. R3: LIV-075, LIV-111, LIV-155. Later: CLI-063, MUS-081 (the desktop shell).

#### SUR-054 Car
- **Release:** R2 (Android Auto). CarPlay is Later.
- **Purpose:** Safe music in the car from the person's own Home rows,
  offline when downloaded (CLI-116, CLI-118, CLI-119, DIS-171, MUS-220).
- **Shows:** The browse tree built from Home rows and playlists, with
  downloaded items marked; long lists with quick navigation (CLI-121,
  CLI-040); car Now Playing with queue modes (CLI-122).
- **Actions:** Browse, play, voice requests (CLI-120), shuffle and repeat.
- **Form factors:** Android Auto only in R2; the car's own templates decide
  the layout. Video in the car is a No (CLI-126).
- **Serves:** R2: CLI-040, CLI-116, CLI-118, CLI-119, CLI-120, CLI-121, CLI-122, DIS-171, MUS-074, MUS-219, MUS-220.

#### SUR-055 TV launcher rows
- **Release:** R2.
- **Purpose:** Continue watching on Android TV's home screen without opening
  the app (CLI-049, DIS-168).
- **Shows:** Items from the synced watch log; a dismissal in the app removes
  the item there too. Restricted artwork never appears (DIS-155). The rows
  appear only if the person said yes when the TV was paired, which defaults
  to yes on a single-profile TV and no when several profiles exist; they
  never include restricted or PIN-protected profiles (SEC-CLI-061), or
  anything played in a private session (SEC-PRV-058).
- **Actions:** Open an item straight into the player.
- **Form factors:** TV only. Apple TV's Top Shelf, named in CLI-049, waits
  for Apple builds.
- **Serves:** R2: CLI-049, DIS-155, DIS-168.

#### SUR-056 Watch controls
- **Release:** R2.
- **Purpose:** Pause and skip from the wrist (CLI-127).
- **Shows:** Whatever the watch's own media controls show for the phone's
  playback; CLI-127 is "provided by CLI-069", so there is no Gunmetal watch
  app in R2 (native watch apps are Later, CLI-128 and CLI-129).
- **Actions:** Play, pause, skip.
- **Form factors:** Watches paired with an Android phone only.
- **Serves:** R2: CLI-127.

#### SUR-057 Install, deep links and operating-system hooks
- **Release:** R1.1 for installing the web app (CLI-003); R1.2 for links
  that open an item (CLI-034, DIS-173, INT-147). Both moved from R1.
- **Purpose:** Get Gunmetal onto a device and open the right item from a
  link or an automation (CLI-003, CLI-034, DIS-173).
- **Shows:** The browser's install prompt, the home-screen icon and the
  standalone window (CLI-003); links that open an album, artist, film or
  playlist in the app or the web client and never grant access by
  themselves (CLI-034, INT-147). From R2: Android intents and NFC-style
  automation hooks through the system settings and share sheet (INT-148).
- **Actions:** Install, open a link, run an automation.
- **Form factors:** *TV:* not applicable in R2. *Phone and tablet:* home
  screen and app links. *Web and desktop:* the installed web app from R1.1;
  desktop installers arrive with the desktop shell, Later (SEC-TM-074).
- **Security:** every inbound link and code is parsed by one function in
  the core into a closed set of routes, and none changes anything without a
  confirmation screen (SEC-CLI-025). A secret carried by a link
  (invitation, pairing, recovery) travels in the fragment, is removed from
  the address bar before any request, and is redeemed only after a screen
  that names the server and says what will happen (SEC-CLI-013). Native
  apps from R2 accept links only through verified app links, never a
  custom URL scheme (SEC-CLI-039).
- **Serves:** R1.1: CLI-003. R1.2: CLI-034, DIS-173. R2: INT-148.

#### SUR-058 Share sheet
- **Release:** R1.2 for music tracks, albums and playlists (ACC-086),
  where the adopted R1 scope places music share links (D-10); SEC-API-097
  and SEC-STD-008 ship with them. R2 adds film and episode links, off
  until the owner enables them (ACC-092), the guest capability (ACC-135),
  sharing a playlist with people on the server, and the native share
  sheets. Without a share link, "Copy link" in the context menu still needs
  a signed-in device (CLI-034).
- **Purpose:** Share an item with someone, with or without an account, as a
  scoped, expiring and revocable capability (ACC-086, ACC-135).
- **Shows:** Link options: expiry (30 days by default), an optional
  password (ACC-087), and a download switch that appears only when the
  owner allows downloads server-wide (ACC-088); the per-link limits (two
  streams at once, a cap on uses or bytes); whether chat apps may show a
  preview, off by default; and the person's existing links with their use
  (ACC-089). From R2: sharing a playlist with people on the server and
  inviting collaborators (MUS-150); film and episode links (ACC-092); and
  the OS share targets.
- **Actions:** Create, copy, revoke.
- **Form factors:** *TV:* not offered (**Proposal**). *Phone and tablet:*
  the system share sheet with Gunmetal's options first (R2), a dialog in
  the web client. *Web and desktop:* a dialog.
- **Security:** a link carries a 128-bit secret in its fragment, covers one
  object, takes its sharer from the session, and stops working on the next
  request after revocation; a link that reaches too many distinct addresses
  suspends itself and alerts the sharer (SEC-API-097, SEC-API-028). A
  share password follows the guessing delays and never disables the link
  (SEC-API-056, SEC-STD-008). Only the sharer and admins can see or change
  a link. Managed profiles cannot create links (SEC-PRV-029).
- **Serves:** R1.2: ACC-086, ACC-087, ACC-088, CLI-034, INT-147, MUS-151. R2: ACC-092, ACC-135, INT-148, MUS-150.

#### SUR-059 Public share page
- **Release:** R1.2 for music (ACC-086; D-10). R2 adds the guest player
  that ACC-135 defines.
- **Purpose:** What someone without an account sees when they open a share
  link: a minimal listening page (ACC-086) or the guest player that
  ACC-135 defines.
- **Shows:** Artwork, title, a player, the link's expiry, and a password
  field when the link has one. Nothing else of the server: not the
  sharer's name, other people, the library's size, its name or the
  server's name (SEC-PRV-031, SEC-NET-047).
- **Actions:** Play; download if the link allows it.
- **Form factors:** Any browser, over HTTPS only (SEC-NET-001). It is an
  unauthenticated surface, so it is kept deliberately small.
- **Security:** the secret stays in the fragment, so it never reaches a
  request line or a log (SEC-API-097, SEC-IAM-047); the page carries
  `noindex` and no
  link-preview metadata unless the sharer turned previews on
  (SEC-PRV-031); its media URLs are short-lived capabilities re-checked on
  every request (SEC-API-026, SEC-API-028); its strings are rendered as
  text (SEC-CLI-001).
- **Serves:** R1.2: ACC-086.

#### SUR-060 TV screensaver
- **Release:** R2.
- **Purpose:** Ambient artwork from the library when the TV is idle, safe
  for OLED panels (CLI-042, DIS-165), never showing restricted artwork
  (DIS-155).
- **Shows:** Slowly changing artwork; the current track when music plays
  (**Proposal**, overlapping with ambient mode in CLI-041).
- **Actions:** Any key returns to where the person was; the play or pause
  key also plays or pauses, as it does on every screen (CLI-045), and TV
  Now Playing's ambient mode follows the same rule.
- **Form factors:** TV only.
- **Serves:** R2: CLI-042, DIS-155, DIS-165.

#### SUR-061 Pairing and approval
- **Release:** R1 for signing in a browser that cannot use a passkey, by
  approval from a signed-in device (ACC-062, SEC-IAM-108). R2 for TVs,
  native apps and command-line tools. Companion-tool sign-in (INT-028) is
  Later.
- **Purpose:** Sign in devices and tools without any password: a borrowed
  or old browser from a signed-in phone or computer, a TV by QR code, a
  command-line tool by device code (ACC-062, ACC-061, CLI-027, INT-027).
- **Shows:** On the new device, a QR code and an 8-character code from the
  base-20 alphabet, valid for 10 minutes (SEC-IAM-056). On the approving
  device, an approval sheet with the requesting device's self-reported name
  marked as unverified, its type, "In this home" or "Somewhere else", how
  long ago it asked, and exactly which account, profiles and capabilities
  it will get, with addresses only behind "Details" (SEC-IAM-058). When the
  two devices are not on the same local network, the sheet asks the
  approver to type the code shown on the new device and confirm a matching
  code shown on both screens, worded neutrally: "Type the code on the TV to
  confirm" (SEC-IAM-060). For household devices, which profiles appear
  (ACC-021); for native apps, device-key enrolment and the key's protection
  level (ACC-051, SEC-CLI-031); for TVs, the one-time question about
  home-screen rows (SEC-CLI-061); for handoff, the accept prompt on the TV
  (VID-144).
- **Actions:** Approve with a passkey or biometric check, deny, choose
  which profile the device gets.
- **Form factors:** *TV:* shows the QR code and the code; never approves.
  *Phone:* approves. *Tablet:* approves. *Web and desktop:* approves when
  it is a personal device, or shows the code when it is the new device.
- **Security:** approval needs a passkey check in the previous 5 minutes
  (SEC-IAM-058) and always starts on the approving device, by scanning or
  typing; nothing a stranger sends can make a prompt appear on anyone's
  device (SEC-STD-027, SEC-IAM-052). A code dies after 5 wrong guesses
  (SEC-IAM-056). No approval ever grants owner or administrator
  capabilities, and a typed-code or remote enrolment never creates a
  household device and alerts every adult (SEC-IAM-059). A browser signed in
  this way is a limited device that can play but never administer, approve
  other devices or change account security (SEC-IAM-108). Limited devices
  never show the approval sheet (SEC-CLI-024). From R2, code screens set
  `FLAG_SECURE` and the approve control ignores touches while obscured
  (SEC-CLI-057, SEC-CLI-058).
- **Serves:** R1: ACC-062. R2: ACC-021, ACC-051, ACC-061, CLI-027, INT-027, VID-144.

### Sign-in, settings and account

#### SUR-070 Sign-in
- **Release:** R1. Sign-in through the household's identity provider, and
  the server's name and message after sign-in, arrive in R1.2 (ACC-057,
  ADM-140).
- **Purpose:** Modern sign-in with no vendor account and no password:
  passkeys, single sign-on through the owner's identity provider (from
  R1.2), and approval from a signed-in device for a browser that cannot use a passkey
  (ACC-050, ACC-057, ACC-062). Passwords and authenticator-app codes
  (ACC-052, ACC-053) are not offered (SEC-IAM-025; owner to confirm).
- **Shows:** A generic page: no list of users (ACC-007), and no server name,
  message, library name or version before sign-in (SEC-NET-047,
  SEC-API-005); from R1.2, a device that has signed in before shows the
  name it remembers, and the name and message appear after sign-in
  (ADM-140). The passkey button; from R1.2, "Continue with" the configured
  provider; "Use another
  device" (ACC-062, opening SUR-061); one uniform message for any failure
  and plain delay messages after repeated failures (ACC-063); after sign-in,
  the one plain question "Is this your own device?" (ACC-079); "Use a
  recovery code" (SUR-133); and the enrolment screen for an owner
  recovering from the host (ACC-004).
- **Actions:** Sign in, use another device, use a recovery code.
- **Form factors:** *TV:* the TV shows a QR code instead (SUR-061).
  *Phone and tablet:* the platform passkey sheet. *Web and desktop:* the
  browser's passkey prompt. Over plain HTTP the page does not exist: every
  peer but loopback gets the help page (SUR-109, SEC-NET-001).
- **Security:** sign-in is usernameless and reveals nothing about whether an
  account or credential exists, by content, status or timing (SEC-IAM-022,
  SEC-API-058). Passkey failures are throttled per source but never disable
  a credential (SEC-API-056). From R1.2, the identity-provider flow keeps
  every token on the server (SEC-CLI-026). The session cookie is `__Host-`, HttpOnly
  and never readable by scripts (SEC-API-032). The answer to "Is this your
  own device?" sets the session: own device, 30 days with 7 idle and
  persistent storage; shared, memory only, a browser-session cookie and 30
  idle minutes (SEC-IAM-041, SEC-CLI-010; the default answer is owner to
  confirm). A post-sign-in return target is only a relative path from an
  allowlist (SEC-IAM-033).
- **Serves:** R1: ACC-004, ACC-007, ACC-050, ACC-062, ACC-063, ACC-079, CLI-150. R1.2: ACC-057, ADM-140.

#### SUR-071 Invite landing and first launch
- **Release:** R1 (the invite landing in the web client). R2 (app first
  launch).
- **Purpose:** Turn an invitation link or QR code into a working account
  with as few steps as possible (ACC-080), without letting a forwarded link
  quietly admit a stranger.
- **Shows:** Who invited the person and to which server (by the name the
  owner gave it from R1.2, ADM-140), what will happen,
  and a privacy notice generated from the server's real settings: what is
  stored about them, who can see what, retention, what leaves the server,
  and how to export and delete, in at most five plain sentences with "More
  details" (SEC-PRV-053). Then account creation with the person's own
  passkey, or, from R1.2, their link to the owner's identity provider; no
  password
  (SEC-IAM-079). For an invitation that makes the person a household member
  or grants more than one library, a waiting screen with a short matching
  code to read to the inviter, until the inviter confirms (SEC-IAM-079).
  From R2: "install this app, then tap here" (ACC-082), and an app first
  launch that finds the server on the home network or reads the invite's
  address (ACC-104, ACC-083).
- **Actions:** Accept, create the account, read the matching code, install
  the app.
- **Form factors:** *TV:* **Proposal:** the TV first launch offers only
  pairing (SUR-061) and finding the server. *Phone, tablet, web:* the full
  landing.
- **Security:** served only over HTTPS, and redeemable only there or in a
  native app (SEC-IAM-078). The page removes the secret from the address
  bar before any request and redeems nothing until the person confirms on
  a screen that names the server (SEC-CLI-013, SEC-NET-036); it sends the
  secret in a POST body, rate-limited, with the same response for any
  failure (SEC-API-096, SEC-API-058). From R1.2, the server's name
  appears here because the invitation authorises it (owner to confirm). A pending
  account holds no grants until confirmed (SEC-IAM-079).
- **Serves:** R1: ACC-080. R2: ACC-082, ACC-104.

#### SUR-072 Server picker
- **Release:** R2.
- **Purpose:** Use several servers in one app, each with its own device
  key (CLI-018, ACC-014).
- **Shows:** The servers, each one's connection state, "Add server" by
  invite, discovery or a custom address (CLI-028), and an optional merged
  Home.
- **Actions:** Switch, add, remove, edit the address.
- **Form factors:** *TV:* a list in Settings and at launch when several
  servers exist. *Phone, tablet, desktop:* in the account menu and in
  Settings. The web client belongs to the server that serves it, so it has
  no picker.
- **Serves:** R2: ACC-014, ACC-083, ACC-104, CLI-018, CLI-028.

#### SUR-073 Settings
- **Release:** R1. R1.1 adds the client side of settings that follow the
  person, with per-device settings marked "this device" (CLI-030).
- **Purpose:** Each person's preferences, which follow them across devices,
  with anything that applies to one device marked "this device" (ACC-012,
  CLI-030). This entry is the Settings home and its general sections; the
  larger sections have their own entries (SUR-074 to SUR-077).
- **Shows:** Section list; Privacy, which says what leaves the house and
  that there is no social feed (ACC-113, ACC-114); the quality-badge
  setting (LIB-146). From R2: the start screen (DIS-014), which media kinds
  and libraries are shown (LAT-012, CLI-051), recommendation reset
  (DIS-056), "Next to watch" and abandoned-show settings (DIS-026, DIS-028),
  spoiler protection (DIS-117), per-device Home overrides (DIS-008), and
  search inside settings (CLI-152). From R3: Live TV as a start screen
  (LIV-076) and recording notifications (LIV-111).
- **Actions:** Change a setting, reset to defaults.
- **Form factors:** *TV:* a focused list with large text; only settings that
  make sense on a TV. *Phone:* a stacked list. *Tablet:* list and detail
  side by side. *Web and desktop:* sections in a side list.
- **Serves:** R1: ACC-012, ACC-113, ACC-114, LIB-146. R1.1: CLI-030. R2: CLI-051, CLI-152, DIS-008, DIS-014, DIS-026, DIS-028, DIS-056, DIS-117, LAT-012. R3: LIV-076, LIV-111.

#### SUR-074 Settings: playback, sound and subtitles
- **Release:** R1. R1.1 adds fades, lyrics staying open and fetching ahead
  on patchy signal (MUS-072, MUS-158, CLI-099). Video, audio-device and
  subtitle sections are R2.
- **Purpose:** How things sound and play.
- **Shows:** In R1: loudness mode and target level (MUS-087, MUS-088) and
  shuffle mode (MUS-126). From R1.1: fades (MUS-072), lyrics staying open
  (MUS-158), and fetching ahead on patchy signal (CLI-099). From R2:
  crossfade and smart fades (MUS-091, MUS-092), the equaliser (MUS-094),
  sample-rate matching and bit-perfect output (MUS-100, MUS-101; exclusive
  output, MUS-102, is Later with the desktop shell), output device
  (MUS-200, CLI-048), ratings that steer playback
  (MUS-171), and the video sections: default quality, deinterlacing, frame
  rate and dynamic range matching, Dolby Vision and tone mapping per device
  (VID-019, VID-023, VID-041, VID-043, VID-045, VID-046, CLI-046),
  preferred audio language, downmix, dialogue lift and per-device audio
  passthrough and delay (VID-051, VID-056 to VID-059, VID-062, VID-066),
  subtitle modes, fallbacks, appearance and position (VID-047, VID-079,
  VID-080, VID-082, VID-091, VID-093, VID-094), skipping, resume rewind,
  autoplay spoilers and background audio (VID-101, VID-114, VID-119,
  VID-128, VID-135), all following the person (VID-142). From R3: automatic
  break skipping (LIV-147).
- **Actions:** Change a setting; per-device settings are marked.
- **Form factors:** *TV:* audio output, passthrough, frame-rate matching and
  subtitle size matter most here and come first (**Proposal**). *Phone:*
  bit-perfect output on Android (MUS-101). *Web and desktop:* the browser
  chooses the output; exclusive output arrives with the desktop shell,
  Later (MUS-102). Settings the device cannot honour
  are hidden, with a note (**Proposal**).
- **Serves:** R1: MUS-087, MUS-088, MUS-126. R1.1: CLI-099, MUS-072, MUS-158. R2: CLI-046, CLI-048, MUS-091, MUS-092, MUS-094, MUS-100, MUS-101, MUS-171, MUS-200, VID-019, VID-023, VID-041, VID-043, VID-045, VID-046, VID-047, VID-051, VID-056, VID-057, VID-058, VID-059, VID-062, VID-066, VID-079, VID-080, VID-082, VID-091, VID-093, VID-094, VID-101, VID-114, VID-119, VID-128, VID-135, VID-142. R3: LIV-147. Later: MUS-102 (the desktop shell).

#### SUR-075 Settings: data, downloads and storage
- **Release:** R1.1 (storage and sync status, CLI-024, which moved from
  R1). Downloads and data use are R2.
- **Purpose:** What the device keeps and what it spends on the network.
- **Shows:** From R1.1: what is synced, how much space each part uses, when it
  last synced, "Sync now" and "Clear cache" (CLI-024). On a shared browser
  it says that nothing is kept after the tab closes (SEC-CLI-010). From R2:
  partial sync for small devices (CLI-023), the storage cap and eviction
  (CLI-082), download quality including Opus (CLI-083, MUS-213), the
  mobile-data rule for downloads (CLI-091),
  streaming quality per network (CLI-092, VID-031), Opus streams on mobile
  data (MUS-106), and the network policy (MUS-107).
- **Actions:** Sync now, clear cache, set caps and rules.
- **Form factors:** *TV:* storage and sync status only. *Phone and
  tablet:* everything. *Web and desktop:* storage and sync status; downloads
  arrive with the desktop shell, Later (SEC-TM-074).
- **Security:** there is no download-location choice in R2. Downloads stay
  in app-private internal storage (SEC-CLI-035), and choosing an SD card or
  other removable storage (CLI-059, MUS-216) waits until downloads there are
  encrypted (SEC-CLI-072, Later; owner to confirm).
- **Serves:** R1.1: CLI-024. R2: CLI-023, CLI-082, CLI-083, CLI-091, CLI-092, MUS-106, MUS-107, MUS-213, VID-031.

#### SUR-076 Settings: appearance, accessibility and language
- **Release:** R1 (themes). R1.1 adds mono audio and channel balance
  (CLI-151); R1.2 adds the language list (CLI-146).
- **Purpose:** Make the interface readable and usable for each person.
- **Shows:** Themes including high contrast (CLI-141); from R1.1, mono
  audio and channel balance (CLI-151); and from R1.2, the language list
  with a completeness bar (CLI-146; R1 and R1.1 ship English only, with
  the framework in place). From R2:
  text size on TV (CLI-043, DIS-166), subtitles that follow system caption
  settings and accessible track defaults (CLI-143, CLI-144), each person's
  own language (CLI-148), region and rating system (ACC-027), and metadata
  language and original titles (LIB-184, LIB-185).
- **Actions:** Change a setting.
- **Form factors:** The same sections on every form factor; TV adds text
  size.
- **Serves:** R1: CLI-141. R1.1: CLI-151. R1.2: CLI-146. R2: ACC-027, CLI-043, CLI-143, CLI-144, CLI-148, DIS-166, LIB-184, LIB-185.

#### SUR-077 Settings: this device, connection and help
- **Release:** R1. Help > Diagnostics arrives in R1.2 (CLI-033).
- **Purpose:** Facts about this device and this connection, help, and the
  preview switch for layout changes.
- **Shows:** "About this connection", which says which address this is,
  that it is secure, whether this browser is marked as the person's own or
  shared and what that keeps, and when the session ends (CLI-150,
  SEC-CLI-010, SEC-IAM-041); Preview features, where
  layout changes arrive as opt-in previews with a way back (CLI-031); and,
  from R1.2, Help > Diagnostics, redacted by default and shown in full before it is
  sent (CLI-033, SEC-PRV-046). From R2: the
  connection's real state and route (ACC-105), the device capability
  report (CLI-047), and whether the app may start itself on a play key or a
  headphone connection (CLI-068). Global hotkeys (CLI-064) arrive with the
  desktop shell, Later.
- **Actions:** Turn a preview on or off, view or send diagnostics (R1.2).
- **Form factors:** The same sections on every form factor; launch
  behaviour only in the native apps, and hotkeys only in the desktop shell
  once it exists.
- **Serves:** R1: CLI-031, CLI-150. R1.2: CLI-033. R2: ACC-105, CLI-047, CLI-068, LIB-109. Later: CLI-064 (the desktop shell).

#### SUR-078 Account
- **Release:** R1. R1.1 adds history import from export files (MUS-189,
  INT-107); R1.2 adds a linked identity provider, the person's share links
  and the documented full-export formats (ACC-057, ACC-089, MUS-151,
  ADM-074).
- **Purpose:** A person's identity and data: sign-in methods, sessions and
  devices, security, shares, exports and imports, and connected services.
  It is the hub for SUR-130 to SUR-133.
- **Shows:** Sign-in methods: the person's passkeys, named, and, from R1.2,
  any linked identity provider (ACC-055, ACC-050; ACC-057), with a gentle prompt while
  only one passkey exists; there is no password or authenticator-app code
  (SEC-IAM-025). Links to Sessions and devices (SUR-130), Security events
  (SUR-131), What admins can see and Privacy (SUR-132) and Recovery
  (SUR-133). From R1.2, the person's share links with their use, and
  revoke (ACC-089, MUS-151). Your data: a documented export of everything the person told
  the server (ACC-010, DIS-058, MUS-188, INT-151, LAT-007; the documented
  full-export formats of ADM-074 from R1.2), history import from Last.fm
  and ListenBrainz files from R1.1 (MUS-189, INT-107), and deleting the
  account. The owner's account details and, for an incoming
  ownership transfer, Accept (ACC-002, ADM-052). From R2: offline copies per
  device (ACC-045), device keys (ACC-051), apps and API keys (ACC-049,
  INT-017, INT-021), the single-item guest links (ACC-135), connected
  services for scrobbling, loved-track sync and similar plugins with their
  delivery status (ACC-119, INT-058, INT-064, INT-075, INT-102 to INT-106,
  INT-110, INT-111, MUS-192 to MUS-195), app keys and "connect a music app"
  for Subsonic apps (ACC-129, CLI-130, INT-024, INT-087, INT-096),
  notifications and per-person webhooks (INT-050, INT-051), the person's own
  security events sent to their own webhooks (INT-036), and API and Trakt
  history imports (INT-108, INT-152, INT-076). Companion-tool sign-ins
  (INT-028) are Later.
- **Actions:** Add or remove a passkey, link or unlink a provider (R1.2),
  revoke a share (R1.2) or key (R2), export, import (R1.1), connect or
  disconnect a service (R2), delete the account.
- **Form factors:** *TV:* not offered. A TV is a limited device, so it
  never shows account-security screens (SEC-CLI-024); its own menu offers
  sign-out only. *Phone, tablet, web:* everything, on a personal device; a
  shared-mode browser or one signed in by approval shows the data sections
  but none of the security sections (SEC-IAM-108).
- **Security:** adding or removing a sign-in method needs a passkey check in
  the previous 5 minutes, tells the account's other devices, and offers to
  end all other sessions; the last credential, and the owner's last
  passkey, cannot be removed (SEC-IAM-023, SEC-IAM-024, SEC-IAM-042,
  SEC-IAM-107). From R1.2, linking a provider happens only inside this
  account's session after a passkey check (SEC-IAM-029). Starting an export or a
  deletion needs the same check and is rate-limited; the download works
  once, only in this session, and expires within an hour; the export holds
  no one else's data and no secrets (SEC-PRV-047, SEC-PRV-048). Deletion
  disables the account at once, keeps it restorable for 7 days, offers an
  export first, and states when it will have left the backups
  (SEC-PRV-051, SEC-IAM-103). During a recovery hold the export and
  credential removal are refused (SEC-IAM-106). Scrobbling and every other
  outbound feature start off and are turned on only by the person, never by
  an admin (SEC-PRV-033). An API key's scopes can never exceed the person's
  own rights or include administration, and creating one needs a passkey
  check (SEC-EXT-010, SEC-EXT-012).
- **Serves:** R1: ACC-002, ACC-010, ACC-017, ACC-050, ACC-055, ACC-065, DIS-058, INT-151, LAT-007, MUS-188. R1.1: INT-107, MUS-189. R1.2: ACC-089, ADM-074, MUS-151. R2: ACC-045, ACC-049, ACC-051, ACC-119, ACC-129, ACC-135, CLI-130, INT-017, INT-021, INT-024, INT-036, INT-050, INT-051, INT-058, INT-064, INT-075, INT-076, INT-087, INT-096, INT-102, INT-103, INT-104, INT-105, INT-106, INT-108, INT-110, INT-111, INT-152, MUS-192, MUS-193, MUS-194, MUS-195.

#### SUR-079 Profiles and guardian controls
- **Release:** R1.1 (profile name and picture, explicit-content display;
  ACC-011 and MUS-047 moved from R1). R2 (household profiles, children and
  guardians).
- **Purpose:** Each profile's identity, and for children, the limits a
  guardian sets.
- **Shows:** From R1.1: the profile editor with name and picture (ACC-011) and
  how explicit tracks are shown (MUS-047). From R2: profile creation and
  PIN (DIS-143, ACC-020); child-profile settings with presets, maximum
  ratings, rating systems by country, allow and block rules, unrated items,
  single-title exceptions, explicit filter, schedules and viewing hours
  (ACC-023 to ACC-029, ACC-032, DIS-145 to DIS-151, LAT-014); the
  designated guardians' view of a child's history, which no other adult or
  admin gets (ACC-034, DIS-156); and a profile's year in review
  (DIS-183). From R3: live TV limits (ACC-036, LIV-164). Later: each
  person's own consent to household blends (DIS-184), if the owner adds
  household activity features.
- **Actions:** Edit, set limits, allow a title, review activity.
- **Form factors:** *TV:* profile name, picture and PIN only; guardians
  set limits on phone or web (**Proposal**). *Phone, tablet, web:*
  everything.
- **Security:** a managed profile's history is visible only to its
  designated guardians, and the child's own screens say so (SEC-PRV-029,
  SEC-PRV-022). Managed profiles cannot link outside accounts, enable
  outbound sharing or create share links (SEC-PRV-029). PINs are checked
  only by the server, delay per device and profile, never lock forever, and
  only switch profiles (SEC-IAM-062, SEC-IAM-063); adding an adult profile
  to a TV preselects "Add a PIN" (SEC-IAM-110). Content limits are enforced
  in the one visibility predicate, so no path around them exists
  (SEC-IAM-064). A household blend includes a person only after their own
  consent (SEC-PRV-022, SEC-PRV-023).
- **Serves:** R1.1: ACC-011, MUS-047. R2: ACC-020, ACC-023, ACC-024, ACC-025, ACC-026, ACC-027, ACC-028, ACC-029, ACC-032, ACC-034, DIS-143, DIS-145, DIS-146, DIS-147, DIS-148, DIS-149, DIS-150, DIS-151, DIS-156, DIS-183, LAT-014. R3: ACC-036, LIV-164. Later: DIS-184.

#### SUR-130 Sessions and devices
- **Release:** R1. Device keys and API keys join the list in R2.
- **Purpose:** Every person can see where they are signed in and end any of
  it, with effect on the next request (first principle 11; SEC-IAM-042).
- **Shows:** Every session and device on the account (from R2, every API
  key and app key too) with its name, device class (personal or limited),
  key protection level, network type, rough location and last use; "This
  device" marked; new devices also announced in the notice centre
  (ACC-068, ACC-071). From R2, each native device's offline copies and when
  they expire (ACC-045), and household devices, with dormant ones marked
  and a one-tap wake for adults (SEC-IAM-109).
- **Actions:** Remove one device or session; "Sign out everywhere else"
  (ACC-069, ACC-070). Wake a dormant household device (R2).
- **Form factors:** *TV:* not offered; a TV offers only "Sign out of this
  TV". *Phone, tablet, web:* the full list on personal devices.
- **Security:** removing a session or device makes its next request fail
  and closes its open streams and sockets within 5 seconds, its stream URLs
  included (SEC-IAM-043, SEC-API-028); the removed browser deletes this
  account's data when it is told (SEC-CLI-009). This list shows only a
  rough location; the person's own security events show full addresses
  (SEC-IAM-042, SEC-OPS-027). Admins can end any non-owner account's
  sessions from Users (SUR-090), and only the owner can end the owner's
  (SEC-IAM-044).
- **Serves:** R1: ACC-068, ACC-069, ACC-070, ACC-071.

#### SUR-131 Security events
- **Release:** R1.
- **Purpose:** Each person can read the security log entries about their own
  account, so a sign-in they did not make, or an admin looking at their
  sessions, is never hidden from them (SEC-IAM-097).
- **Shows:** Sign-ins and failed attempts, new and removed devices and
  passkeys, recovery and the recovery hold, changes to their role, library
  access or identity-provider link, and every admin action that read or
  changed something of theirs, including reading their live sessions, each
  with time, device and full address (SEC-IAM-077, SEC-OPS-027,
  SEC-PRV-026). A guardian also sees the entries for their managed
  profiles.
- **Actions:** Filter by kind; from an entry about a device or credential,
  "This wasn't me", which revokes it (SEC-OPS-033).
- **Form factors:** *TV:* not offered. *Phone, tablet, web:* a list on
  personal devices.
- **Security:** generated from the hash-chained audit log (SEC-IAM-094),
  showing only this person's records; secrets and media titles are never
  in it (SEC-OPS-021). Retention follows the one schedule, 365 days with
  addresses shortened after 30 and removed at 90 (SEC-PRV-005).
- **Serves:** R1: ACC-078, ADM-110.

#### SUR-132 What admins can see, and privacy choices
- **Release:** R1. The switch to show titles to admins arrives in R1.2,
  with the admins' live view (ACC-116, MUS-235, ADM-099).
- **Purpose:** The page that tells each person exactly what admins and
  guardians on this server can see about them, generated from the same
  policy the server enforces, so it can never drift from the truth
  (SEC-IAM-104, SEC-PRV-027).
- **Shows:** In plain words, generated from what the running release lets
  admins see. In R1 admins have no live view of anyone's playback, so the
  page says that admins see the person's account, its device count and
  last sign-in, but not what they play, their history, ratings or private
  playlists, and that whoever controls the computer Gunmetal runs on could
  read its files directly. From R1.2, when the live view arrives (ADM-099),
  for example: "Your admin can see that you are streaming, on which device
  and at what quality. Your admin sees the title only if you turn on the
  switch below. Your admin cannot see your history, ratings or private
  playlists in Gunmetal. Your admin controls the computer Gunmetal runs on,
  so a determined admin could read its files directly." Below it: the
  private-session default end time; from R1.2, the switch "Show what I'm
  playing to admins", off by default (ACC-116, MUS-235); and, from R2,
  sharing what one plays with the household, off by default.
- **Actions:** Turn the title switch on or off (R1.2); open Security
  events (SUR-131).
- **Form factors:** The same page on every form factor; on TV, read-only.
- **Security:** every privacy setting starts at its most private value, and
  no upgrade or admin can change it for the person (SEC-PRV-023,
  SEC-PRV-033). No server setting can widen what admins see beyond live
  sessions and totals (SEC-PRV-025). The same statements appear, shortened,
  in the invitation's privacy notice (SEC-PRV-053).
- **Serves:** R1: ACC-115. R1.2: ACC-116, MUS-235.

#### SUR-133 Recovery
- **Release:** R1.
- **Purpose:** The recovery ladder in one place: another passkey first,
  then recovery codes, then the identity provider (from R1.2), then an
  administrator's
  link, and for the owner only, the host (SEC-IAM-089 to SEC-IAM-092).
- **Shows:** How many passkeys the account holds, with a prompt to add a
  second; recovery codes (10, single-use, offered at enrolment to owners and
  administrators and available here to everyone), shown once and
  regenerated on request (SEC-IAM-089); for the owner, the recovery kit
  status and "Show the kit again" (SEC-PRV-040); an explanation of
  administrator recovery links and the 72-hour hold; and, during a hold,
  what is restricted and when it ends (SEC-IAM-106). On the sign-in page,
  "Use a recovery code" opens a session that can do nothing but enrol a new
  passkey.
- **Actions:** Generate new codes, show the kit again, end a hold from an
  existing device.
- **Form factors:** *TV:* not offered. *Phone, tablet, web:* on personal
  devices; from R2 the code screen sets `FLAG_SECURE` (SEC-CLI-057).
- **Security:** showing codes or the kit again needs a passkey check in the
  previous 5 minutes (SEC-PRV-040, SEC-CLI-059). Codes are stored only as
  peppered hashes (SEC-IAM-089), entry is rate-limited (SEC-IAM-101), and
  using one notifies every device on the account and lists every existing
  credential to review (SEC-IAM-090). A new credential enrolled by a code
  or an admin link cannot remove others, elevate, export history or create
  invitations until the hold ends, and after an admin's link the history
  stays hidden from it (SEC-IAM-106).
- **Serves:** R1: ACC-137, ACC-138, ADM-143.

### Setup and administration

Administration is reached from the account menu by administrators only, on
a personal device, in a separate admin session that a passkey check opens
and that ends after 15 idle minutes or 1 hour; a media session never opens
an admin route (SEC-IAM-041). The actions SEC-IAM-041 lists also need a
passkey check in the previous 5 minutes (the step-up prompt, SUR-008), and
the owner-only capabilities (administrators, ownership, security settings,
identity providers, plugins) are never available to anyone else
(SEC-IAM-075). Admin routes are refused on internet-posture paths unless
the owner has turned remote administration on (SEC-NET-045). Every
state-changing admin action writes an audit record, and does not happen if
the record cannot be written (SEC-OPS-020).

**One rule covers every admin surface's form factors, and each entry notes
only its exceptions.** In R1 the admin surfaces are part of the web client
and work in both the wide layout and the phone-width layout, because
CLI-149 covers every screen. In R2 they reach the native phone and tablet
apps because they are part of the one shared UI (CLI-058, ADM-114); admin
rights there need a hardware-backed device key and a biometric or device
PIN per action (SEC-IAM-049, SEC-CLI-059). The TV never shows an admin
surface: it is a limited device, and the server refuses admin routes from
it whatever the client shows (SEC-CLI-024). An admin's profile on a TV
shows security alerts as a banner (ADM-114), and **Proposal:** every admin
task on TV is a short "open this on your phone or computer" note with a QR
code to the page. Dense editors (webhook templates, rule builders, the live
TV lineup editor) are designed for wide screens first and remain usable at
phone width.

Four surfaces in this group are not part of the client at all: the startup
page, the emergency page (from R1.2) and the help pages (SUR-109) are
rendered by the server itself, and the welcome flow runs before any account
exists.

#### SUR-080 Startup page
- **Release:** R1.
- **Purpose:** Says what the server is doing instead of refusing the
  connection while it starts, migrates, rebuilds or restores (ADM-032).
- **Shows:** Each step with an estimate, in plain words: starting,
  migrating, checking (ADM-057, ADM-058), rebuilding the cache (ADM-059),
  recovering the user log after a torn write (ADM-078), restoring (ADM-029;
  a restore started from the admin screens from R1.2, ADM-070), or in
  maintenance after a failed migration, and when a
  problem stops the start, that there is one and that the console and
  `gunmetal doctor` give the details. The details themselves, such as
  configuration errors with their line (ADM-007), the pre-upgrade
  snapshot's location (ADM-056), a data directory on a network filesystem
  (ADM-079) and the refusal to run as root (ADM-006), go to the console and
  the log, not to the page. From R2: upgrades that re-read only what a new
  parser changes (ADM-062). The page is read-only.
- **Actions:** None; it refreshes itself.
- **Form factors:** A plain server-rendered page in any browser. **Proposal:**
  native apps show the same states in their own words when they reach a
  server that is starting.
- **Security:** the page is unauthenticated, so it shows no version, build,
  file path, stack trace, database error or secret (SEC-OPS-050,
  SEC-NET-047). Over plain HTTP only loopback sees it; every other peer gets
  the help page (SEC-NET-001). This moves configuration errors, the
  snapshot path and the root refusal off the page.
- **Serves:** R1: ADM-006, ADM-007, ADM-029, ADM-032, ADM-056, ADM-057, ADM-058, ADM-059, ADM-078, ADM-079. R1.2: ADM-070. R2: ADM-062.

#### SUR-081 Emergency page
- **Release:** R1.2, with the emergency page itself (ADM-113), which moved
  from R1. Until then the owner uses the console and the command line
  (SUR-110).
- **Purpose:** Lets an administrator check status, read logs, take a backup
  or restart even when the main client will not load (ADM-113).
- **Shows:** Before sign-in, only a passkey sign-in button. After an owner
  or administrator signs in, which opens an admin session: server status,
  recent diagnostic log lines (which at the default level hold no titles,
  paths, search terms or secrets), "Back up now", and restart.
- **Actions:** Sign in, back up now, download a backup (owner only, with a
  passkey check in the previous 5 minutes), restart.
- **Form factors:** A minimal server-rendered page, deliberately without the
  client bundle.
- **Security:** it is not a back door: it needs its own admin session
  (SEC-IAM-041), is refused on internet-posture paths unless remote
  administration is on (SEC-NET-045), and over plain HTTP exists only for
  loopback (SEC-NET-001). Log lines follow SEC-PRV-043 and SEC-OPS-029.
  Downloading a backup is owner-only step-up, audited and alerted
  (SEC-OPS-045). Every action writes an audit record (SEC-OPS-020).
- **Serves:** R1.2: ADM-113.

#### SUR-082 Welcome (first-run setup)
- **Release:** R1. R1.1 adds "Coming from another server?", the provider
  lookups in the privacy step and remapping libraries during a restore
  (ADM-030, LIB-111, LIB-112, ADM-051); R1.2 adds language, region and
  time zone (ADM-027). The project's per-server name joins the HTTPS
  choices in R2 (ADM-023, D-07).
- **Purpose:** Take a fresh install to a playing library, safely: nobody
  else on the network can claim the server first, and nothing leaves the
  house unless the owner chooses (ACC-001, LIB-108, ACC-113).
- **Shows:** The steps, in order (**Proposal** for the order): from R1.2,
  language, region and time zone (ADM-027), held by the page until the
  claim; the one-time setup code, arriving in the claim link's fragment or
  typed from the console, with a checksum that catches typos (ACC-001,
  ADM-018); "Where will people reach this server?", which names the
  address the passkey will belong to and says it is permanent for
  passkeys, sends a localhost visit to the configured HTTPS name first,
  and explains the paths to HTTPS when there is none: the owner's own
  domain with a certificate the server obtains and renews itself, a
  tailnet name, or this machine, directly or through an SSH tunnel
  (ADM-021, ADM-022; the project's per-server name, ADM-023, from R2); a
  prompt to trust a reverse proxy that is in front of the server but not
  declared; create the owner account with a passkey only (ACC-002,
  ADM-019, ACC-050); the recovery kit, confirmed by typing its last four
  characters, and an offer of a second passkey (ADM-143); privacy choices
  and the required security-fix question with no answer preselected
  (ADM-028, LIB-108, ACC-113), with, from R1.1, the MusicBrainz and Cover
  Art Archive lookups, off, each with what it would send (LIB-111,
  LIB-112); music folders with live checks (ADM-025, LIB-001, LIB-005);
  from R1.1, "coming from another server?" (ADM-030); and done, with the
  library usable while the first scan runs (ADM-031, LIB-021). "Restore
  from a backup" is offered on the first screen, behind the same setup
  code, and asks for the recovery key and, from R1.1, where the libraries
  are now (ADM-029, ADM-069; ADM-051). From R2: movie and TV folders
  (ADM-026, LIB-002), a transcoding self-test (VID-009), an import summary
  of ratings found in tags (MUS-045), and setup from the Android app with a
  device key (ADM-035). The welcome flow closes for good once the owner
  exists (ADM-020).
- **Actions:** Each step's choices; skip optional steps; restore instead.
- **Form factors:** *TV:* not offered. A TV never claims a server, because
  it is a limited device that can never hold owner or administrator
  capabilities (SEC-CLI-024, SEC-IAM-059); the owner claims from a phone or
  computer and pairs the TV afterwards. This replaces the earlier proposal
  of TV setup through ADM-035. *Phone and tablet:* the web flow at phone
  width in R1, usually opened by scanning the console's QR code; the native
  onboarding in R2. *Web and desktop:* the main path.
- **Security:** until the claim the server answers only this page, its
  assets, the claim and restore endpoints and a health check, and accepts
  no sign-in (SEC-IAM-006, SEC-OPS-001, SEC-OPS-003). The page works only
  in a secure context, the owner's domain or tailnet name over HTTPS, or
  localhost, with a recognised Host (SEC-IAM-008, SEC-IAM-010). The code is
  128 bits, lasts 24 hours, survives restarts, never appears in a response or the log, and is moved
  from the fragment into a request body and off the address bar
  (SEC-IAM-007, SEC-CLI-013); wrong codes are delayed per source, shown on
  the host console and never use it up (SEC-IAM-008, SEC-OPS-004). The
  claim consumes the code, creates the owner and enrols the passkey in one
  transaction, after which every setup route is gone for good (SEC-IAM-009,
  SEC-OPS-005, SEC-OPS-006). There is no password path (SEC-IAM-025). A
  restore needs the same code (SEC-OPS-008) and checks the backup's
  signature before anything else (SEC-OPS-043). Before the claim the server
  makes no outbound connection, except that an own-domain install obtains
  its certificate from the configured CA, which the egress inventory allows
  (SEC-OPS-007, SEC-TM-075; [flows G21](flows.md#gaps-and-questions-the-flows-expose));
  the name service's registration arrives with the name service in R2.
  Browsing folders and adding roots is step-up (SEC-IAM-041).
- **Serves:** R1: ACC-001, ACC-002, ACC-050, ACC-113, ADM-018, ADM-019, ADM-021, ADM-022, ADM-025, ADM-028, ADM-029, ADM-031, ADM-069, ADM-143, LIB-001, LIB-005, LIB-021, LIB-108. R1.1: ADM-030, ADM-051. R1.2: ADM-027. R2: ADM-023, ADM-026, ADM-035, LIB-002, MUS-045, VID-009.

#### SUR-083 Admin dashboard
- **Release:** R1, with the security card (ADM-142). R1.2 adds the health
  summary, who is listening now, and restart and shut down (ADM-109,
  ADM-099, MUS-190, ADM-112); R1.3 adds bytes read on the scan card
  (ADM-088).
- **Purpose:** The admin home: is the server healthy, who is using it, and
  what needs attention (ADM-142; ADM-109 from R1.2).
- **Shows:** Backup and verification state (ADM-065, ADM-072), library
  roots (ADM-108), free space (ADM-083) and a scan card (ADM-031; bytes
  read from R1.3, ADM-088), gathered from R1.2 into a health summary
  (ADM-109); a security card (ADM-142): "Reachable from the internet: no",
  or through which proxy, and which listeners hold globally routable
  addresses (SEC-NET-028, SEC-OPS-038), the isolation
  tier (SEC-MED-024), root and capability state, audit verification, backup
  age and encryption, version support and clock skew (SEC-OPS-061), whether
  the security-fix check is off (SEC-OPS-047), a reminder until the
  recovery kit is confirmed (SEC-PRV-040), and the certificate's expiry
  (SEC-NET-072); from R1.2, who is listening now, without titles unless
  each person chose to show them (ADM-099, MUS-190); owner alerts
  (ADM-116), with critical ones as a persistent banner; banners for known
  advisories (ACC-127, ADM-054) and for recovery having been used
  (ADM-034); an update card (ADM-053); and, from R1.2, a menu to restart or
  shut down (ADM-112). For the owner: "Rotate all server secrets"
  (SEC-OPS-018). From R2: a checklist that
  ticks itself (ADM-033), direct play, remux and transcode breakdown
  (ADM-101), bandwidth and system charts (ADM-104, ADM-105), storage
  statistics (ADM-082, ADM-107), capability and withdrawn-release notices
  (ADM-014, ADM-133, ADM-061), API-key expiry banners (INT-019), and server
  events (INT-037). From R3: live TV sessions (LIV-161).
- **Actions:** Open any card's detail, acknowledge, restart (R1.2), and
  for the owner rotate secrets (step-up).
- **Form factors:** The general admin rule. Not on TV, which shows only the
  alert banner on an admin's profile (ADM-114, SEC-CLI-024).
- **Security:** from R1.2, the "listening now" card shows person, device,
  quality and playback method, and the title only for people who turned on
  "Show what I'm playing to admins"; private sessions never show one; each admin read
  of live sessions is rate-limited and appears in that person's own
  security events (SEC-PRV-025, SEC-PRV-024, SEC-IAM-077). The version
  appears only here and in other signed-in admin views (SEC-NET-047).
  Rotating secrets invalidates every session and stream URL, keeps device
  keys enrolled, and alerts the owner and admins (SEC-OPS-018).
- **Serves:** R1: ACC-127, ADM-031, ADM-034, ADM-054, ADM-065, ADM-072, ADM-083, ADM-108, ADM-116, ADM-142, MUS-042. R1.2: ADM-099, ADM-109, ADM-112, MUS-190. R1.3: ADM-088. R2: ADM-033, ADM-061, ADM-062, ADM-082, ADM-101, ADM-104, ADM-105, ADM-107, ADM-114, ADM-133, CLI-058, INT-019, INT-037. Later: ADM-014.

#### SUR-084 Sessions
- **Release:** R1.2, with the admins' live view (ADM-099, ADM-100,
  ADM-102), which moved from R1. In R1 an administrator ends a person's
  sessions from Users (SUR-090), without any live view (SEC-IAM-044).
- **Purpose:** Live sessions, each with its playback decision and the reason
  for it, and the power to stop one with a message (ADM-099, ADM-100,
  ADM-102).
- **Shows:** Who is playing on which device, at what quality and by which
  playback method, the delivery path and its reason (ADM-100, INT-134,
  VID-002), and the title only for people who chose to show it; a private
  session shows as private, with no title. From R2, the remux and transcode
  worker state (VID-005, VID-007, VID-169, VID-171), bandwidth (ADM-104) and
  a session trace (ADM-126), which names the title only when the viewer
  sent a report; messages to one person or everyone and scheduled
  maintenance notices (ACC-074, ADM-103).
- **Actions:** Stop a session with a message (ACC-073, ADM-102, VID-172),
  open the session detail, message people (R2).
- **Form factors:** The general admin rule. Not on TV (SEC-CLI-024).
- **Security:** this replaces "who is playing what": an admin sees that a
  person is playing, not what, unless that person opted in (SEC-PRV-025;
  owner to confirm, security README decision 5). Admin reads of live
  sessions are rate-limited and recorded in each person's own security
  events (SEC-IAM-077). Stopping a session refuses its next request and cuts
  its open stream within 5 seconds (SEC-IAM-043); administrators can stop
  any session except the owner's (SEC-IAM-044). The message is plain text,
  shown as text (SEC-API-046). There is no way to act as another person
  (SEC-PRV-026).
- **Serves:** R1.2: ACC-072, ACC-073, ADM-099, ADM-100, ADM-102, INT-134. R2: ACC-074, ADM-103, ADM-104, ADM-126, INT-136, VID-002, VID-005, VID-007, VID-169, VID-171, VID-172.

#### SUR-085 Libraries and library settings
- **Release:** R1. R1.1 adds exclusions and changing a library's location
  (LIB-006, LIB-031, ADM-051); R1.2 adds "keep off Home" (DIS-012); R1.3
  adds scan bytes, background analysis, the spoken-word flag and "Rebuild
  library" with fixes kept (ADM-088, LIB-024, LAT-010, LIB-179).
- **Purpose:** Add, configure and scan libraries, while media stays
  read-only (LIB-007).
- **Shows:** The library list with each root's health (ADM-108) and, from
  R1.3, scan bytes (ADM-088); per library: folders (LIB-003), exclusions
  (LIB-006, R1.1), schedule (LIB-013), "watch for changes" (LIB-014),
  storage type per folder (LIB-015), background analysis (LIB-024, R1.3),
  artist splitting rules (LIB-038, MUS-035), artwork order (LIB-136),
  "Media is read-only" (LIB-007), "keep off Home" (DIS-012, R1.2), the
  spoken-word flag (LAT-010, R1.3), "change location" with a preview
  (LIB-031, ADM-051, R1.1), and "Rebuild library", which states that fixes
  are kept (LIB-179, R1.3). From R2: library
  kinds for video and home video (LIB-002, LIB-010), music tag choices
  (LIB-055, MUS-016), ratings from tags (LIB-061), precedence and embedded
  titles (LIB-128, LIB-129), metadata language (LIB-183), collections and
  franchises (LIB-167, DIS-134), music video folders (LAT-093),
  recommendation plugins (DIS-076), locks (LIB-175), I/O profile per root
  (ADM-087), and refresh modes (LIB-119).
- **Actions:** Add a library, scan now (LIB-012), rescan a folder or item,
  edit settings, change location (R1.1), rebuild (R1.3), choose who may
  see a library.
- **Form factors:** The general admin rule. "Scan now" is also on the phone
  dashboard in R2 (CLI-058); not on TV (SEC-CLI-024).
- **Security:** adding, removing or moving a root, and opening the folder
  picker, are step-up actions (SEC-IAM-041, SEC-TM-017). The picker lists
  directories only, inside the configured browse roots, and refuses a
  filesystem root, a system directory, or a folder that holds the server's
  own data, cache, configuration or logs, saying why (SEC-API-022,
  SEC-MED-037). Roots are opened read-only, and the page warns when the
  service account could write to one (SEC-MED-038, SEC-OPS-054). A new
  library is visible only to the owner and admins until someone is granted
  it (SEC-IAM-070); a grant applies from the next request (SEC-IAM-076).
  From R1.1, turning on MusicBrainz or cover-art lookups for a library is
  an egress grant, shown with the fields each provider receives
  (SEC-PRV-013, SEC-PRV-014).
- **Serves:** R1: ADM-025, ADM-085, ADM-108, LIB-001, LIB-003, LIB-004, LIB-007, LIB-012, LIB-013, LIB-014, LIB-015, LIB-038, LIB-136, MUS-035. R1.1: ADM-051, LIB-006, LIB-031. R1.2: DIS-012. R1.3: ADM-088, LAT-010, LIB-024, LIB-179. R2: ADM-087, DIS-076, DIS-134, LAT-093, LIB-002, LIB-010, LIB-055, LIB-061, LIB-119, LIB-128, LIB-129, LIB-167, LIB-175, LIB-183, MUS-016.

#### SUR-086 Library health
- **Release:** R1. Tag problems and missing files arrive in R1.1 (LIB-194,
  LIB-034).
- **Purpose:** Every problem the scanner found, in one place, each with a
  fix where one exists (MUS-044).
- **Shows:** Damaged and unreadable files (LIB-193, MUS-229, MUS-079),
  files quarantined after crashing or hanging the scan worker twice, with
  the reason and a retry (SEC-MED-019), the scan worker's isolation tier
  with a "reduced isolation" notice when the host cannot apply the full
  sandbox (SEC-MED-024), tag problems with fix actions (from R1.1, LIB-194;
  LIB-038, LIB-049), same-name artist collisions (MUS-006), sidecar problems such as
  LRC files (LIB-068,
  MUS-155), missing files (LIB-034, R1.1), moved files (MUS-038), offline roots
  (LIB-032) and watch warnings (LIB-014). From R2: remap a missing file by
  hand (LIB-035), naming (LIB-084), files that need indexing (LIB-089),
  unmatched items (LIB-102), duplicates (LIB-196), incomplete metadata
  (LIB-197), technical quality (LIB-198), orphaned sidecars (LIB-199),
  missing episodes (LIB-200), an integrity report (ADM-075), and "Export" on
  every report (LIB-203).
- **Actions:** Filter, fix, ignore, open in the file inspector.
- **Form factors:** The general admin rule.
- **Serves:** R1: LIB-014, LIB-032, LIB-038, LIB-049, LIB-068, LIB-193, MUS-006, MUS-035, MUS-038, MUS-044, MUS-079, MUS-155, MUS-229. R1.1: LIB-034, LIB-194. R2: ADM-075, LIB-035, LIB-084, LIB-089, LIB-091, LIB-102, LIB-196, LIB-197, LIB-198, LIB-199, LIB-200, LIB-203.

#### SUR-087 Review queue
- **Release:** R1.3, with the review queue itself (LIB-099), which moved
  from R1.
- **Purpose:** Decisions the scanner was not sure about, waiting for a
  person (LIB-099).
- **Shows:** Each item with the evidence and the proposed decision, for
  example two same-titled albums that might be one (LIB-051), and
  MusicBrainz lookup results (LIB-111, whose built-in lookups exist from
  R1.1). Affected items carry a marker in
  browse.
- **Actions:** Accept, reject, choose another match.
- **Form factors:** The general admin rule.
- **Serves:** R1.3: LIB-051, LIB-099, LIB-111.

#### SUR-088 File inspector
- **Release:** R1.2, with the file inspector itself (ADM-125, LIB-195),
  which moved from R1. "Why is this here?" reaches the track info sheet
  in R1.1 (LIB-098, SUR-013).
- **Purpose:** Shows exactly what the server read from one file and why it
  made each decision (ADM-125, LIB-195, LIB-098).
- **Shows:** Every raw tag (LIB-059), how the item was identified without
  the internet (LIB-097), "Why is this here?" (LIB-098), the exclusion rule
  that skipped a file (LIB-006), and IDs (LIB-046). From R2: the source file
  of a CUE album (LIB-071), ID hints (LIB-075), NFO sources (LIB-126) and
  external IDs (LIB-105).
- **Actions:** Copy, rescan this file, open the item.
- **Form factors:** The general admin rule. Opened from an item's context
  menu as "File info", or from Diagnostics; the same report is a CLI
  command (SUR-110).
- **Serves:** R1.2: ADM-125, LIB-006, LIB-046, LIB-059, LIB-097, LIB-098, LIB-195. R2: LIB-071, LIB-075, LIB-105, LIB-126.

#### SUR-089 Corrections and metadata editor
- **Release:** R1.3 (merge, split and alias for artists and albums, which
  moved from R1). R2 (the edit sheet, artwork picker, fix match and edit
  history).
- **Purpose:** Fix the library without touching the files; every fix is a
  logged event that survives rescans and rebuilds (MUS-007, LIB-179).
- **Shows:** From R1.3: "Merge with" and "Split" on artists and albums (LIB-041,
  LIB-058, MUS-007), the admin corrections list, and artwork taken from the
  folder (LIB-135). From R2: the edit sheet on every item with a lock per
  field, its source, and "Unlock all" (LIB-172, LIB-173, LIB-174, LIB-118,
  MUS-046, LIB-121, LIB-186, LIB-048); bulk edit (LIB-176) and labels
  (LIB-180); the artwork picker with candidates, upload, link and lock
  (LIB-112, LIB-138 to LIB-141, LIB-144, MUS-025, INT-141); the fix-match
  dialog (LIB-100, LIB-101); merge and split versions (LIB-156); "look up on
  MusicBrainz" (LIB-111); "mark as explicit" (ACC-024); the genre edit menu
  (MUS-018); the playlist image editor (MUS-138); and item history with
  undo plus a recent-changes list (LIB-178, LIB-030, INT-120).
- **Actions:** Merge, split, alias, edit, lock, choose artwork, fix a match,
  undo.
- **Form factors:** *TV:* not offered. LIB-172 names the TV, but editing
  the library is an administrator capability, and a TV is a limited device
  that the server refuses admin operations from (SEC-CLI-024,
  SEC-IAM-049); the TV offers a QR code to open the item's edit sheet on a
  phone or computer. Otherwise the general admin rule.
- **Security:** every edit is a logged event attributed to the admin who
  made it (SEC-OPS-020). Uploaded artwork goes through the upload route and
  is re-encoded by memory-safe decoders (SEC-API-085, SEC-MED-061,
  SEC-PRV-006); an artwork link is fetched by the server only through the
  egress client, never from a URL a user typed into a request
  (SEC-API-080). Names and fields are stored as text and never written back
  into media folders (SEC-MED-038).
- **Serves:** R1.3: LIB-041, LIB-048, LIB-058, LIB-135, MUS-007. R2: ACC-024, INT-141, LIB-100, LIB-101, LIB-111, LIB-112, LIB-113, LIB-118, LIB-119, LIB-121, LIB-126, LIB-138, LIB-139, LIB-140, LIB-141, LIB-144, LIB-156, LIB-172, LIB-173, LIB-174, LIB-176, LIB-178, LIB-180, LIB-186, MUS-018, MUS-025, MUS-046, MUS-138.

#### SUR-090 Users and invitations
- **Release:** R1. R1.2 adds several administrators and everyone's share
  links (ACC-040, ACC-089).
- **Purpose:** Local accounts with no vendor account, invited by link or QR
  code (ACC-006, ACC-080).
- **Shows:** Users with their libraries (ACC-037, MUS-027), their role,
  their device count and last sign-in, a device cap per person (ACC-076;
  allow-lists from R2), and an enable switch (ACC-008); from R1.2, several
  administrators (ACC-040); "help sign in" for a locked-out member
  or guest, which shows a one-time recovery QR code on this screen for the
  person to scan while present, or sends it to a device they already
  approved (ACC-064); handing the server to a new owner by a transfer the
  recipient accepts (ADM-052); invitations with their address, expiry, uses
  and status, including those waiting for the inviter to confirm a
  matching code (ACC-080); and, from R1.2, everyone's share links, with
  revoke (ACC-089). From R2: per-person playback, download, quality, device and
  remote-access rights (ACC-043, ACC-044, ACC-103, ACC-107, ACC-108); memberships that end on a date (ACC-081); and invite options
  and QR codes that carry the server's address (ACC-083). Re-inviting people
  from an old server (ACC-084) is Later, with the rival importers.
- **Actions:** Invite, confirm a pending invitee's matching code, disable,
  end someone's sessions, change access, help sign in, make an
  administrator (owner only, R1.2), transfer ownership (owner only),
  revoke a share (R1.2).
- **Form factors:** The general admin rule; managing users from the phone is
  named in CLI-058. In R1 the "End sessions" control on a person's page is
  how an administrator stops someone's playback, because the live view
  (SUR-084) arrives in R1.2 (SEC-IAM-044). Not on TV (SEC-CLI-024).
- **Security:** an invitation's preset can never exceed the inviter's own
  rights; it carries a 128-bit secret in the fragment, lasts 7 days and one
  use by default, and the screen shows the address the link carries
  (SEC-IAM-073, SEC-IAM-078, SEC-NET-015). An invitation that confers member
  or household access, or more than one library, leaves the new account
  pending with no grants until the inviter confirms a short matching code
  after seeing the redeeming device's description (SEC-IAM-079). Recovery
  links: administrators for members and guests, the owner for
  administrators, nobody for the owner; redeemed in person or on an
  approved device; they start the 72-hour hold, during which the account's
  history stays hidden from the new credential (SEC-IAM-091, SEC-IAM-106).
  Only the owner creates or promotes administrators, with step-up, and
  every administrator is alerted (SEC-IAM-075, SEC-OPS-034). Ownership moves
  only when the current owner and the recipient each confirm with a passkey
  check in the previous 5 minutes; the owner can never be deleted, disabled
  or demoted (SEC-IAM-003). Disabling ends the account's sessions at once
  (SEC-IAM-103), and any change to a person's credentials, role or access is
  shown to them at their next sign-in (SEC-PRV-026). This page never shows
  anyone's history, and there is no "sign in as" (SEC-PRV-025,
  SEC-PRV-026). This replaces "add an admin and remove yourself".
- **Serves:** R1: ACC-006, ACC-008, ACC-037, ACC-064, ACC-076, ACC-080, ADM-052, MUS-027. R1.2: ACC-040, ACC-089. R2: ACC-043, ACC-044, ACC-081, ACC-083, ACC-103, ACC-107, ACC-108.

#### SUR-091 Household, policies, devices and shares
- **Release:** R2.
- **Purpose:** The household layer: shared profiles and children, named
  access policies, household devices, and every share link in one place.
- **Shows:** The household and its profiles, with "add a child" (ACC-016,
  ACC-018); household devices (ACC-021); named policies and their defaults
  (ACC-038, ACC-039); stream and transcode limits (ACC-075, ACC-111,
  VID-173); playback-mode rights and quality floors (VID-010, VID-029);
  devices and their offline copies, with revoke (ACC-045, CLI-096); and
  everyone's shares, including video links once the owner enables them
  (ACC-089). The earlier "what admins may see" setting (ACC-116) is gone
  from here: no server setting can widen what admins see, and the one
  choice belongs to each person (SUR-132, SEC-PRV-025).
- **Actions:** Create and assign policies, revoke a device's downloads,
  wake or remove a dormant household device, revoke a share.
- **Form factors:** The general admin rule.
- **Security:** household devices work only on the home network by default,
  go dormant after 30 days unused, and never carry administrator
  capabilities (SEC-IAM-109, SEC-IAM-049; owner to confirm, security README
  decision 16). Enrolling one needs the household-device capability, and a
  remote or typed-code enrolment can never create one (SEC-IAM-059).
  Policies can never grant more than their creator holds (SEC-IAM-073), and
  a policy default never sets a privacy setting to anything but its most
  private value (SEC-PRV-023).
- **Serves:** R2: ACC-016, ACC-018, ACC-021, ACC-038, ACC-039, ACC-045, ACC-075, ACC-089, ACC-111, CLI-096, VID-010, VID-029, VID-173.

#### SUR-092 Sign-in and security settings
- **Release:** R1. Single sign-on through the owner's OIDC provider arrives
  in R1.2 (ACC-057).
- **Purpose:** The server's sign-in rules. Owner only (SEC-IAM-075).
- **Shows:** From R1.2, single sign-on through the owner's OIDC provider
  (ACC-057), with automatic sign-up off and claims mapped to administrator
  off (SEC-IAM-030, SEC-IAM-031); protection against guessing and its current
  state (ACC-063); session lifetimes, which the owner may shorten but never
  lengthen beyond 30 days for browsers and 15 minutes idle or 1 hour for
  admin sessions (ACC-079, SEC-IAM-041); the recovery-hold length, 24 to
  72 hours (SEC-IAM-106); and remote administration, off by default
  (SEC-NET-045). From R2: requiring a passkey rather than the provider alone
  (ACC-054) and provider claims mapped to policies (ACC-058).
- **Actions:** Configure, test the provider (R1.2), shorten lifetimes,
  turn remote administration on or off.
- **Form factors:** The general admin rule.
- **Security:** every change here is an owner step-up action, audited, and
  a loosening raises an owner alert (SEC-IAM-041, SEC-OPS-031). The
  provider's client secret is kept encrypted and never shown again
  (SEC-OPS-017). A provider can never make anyone the owner, and a provider
  sign-in alone never satisfies step-up (SEC-IAM-031, SEC-IAM-107). There
  is no switch for passwords, passwordless "home network" sign-in or
  disabling authentication (SEC-IAM-025, SEC-HIS-004).
- **Serves:** R1: ACC-063, ACC-079. R1.2: ACC-057. R2: ACC-054, ACC-058.

#### SUR-093 Network and remote access
- **Release:** R1 (posture, trusted proxies for the owner's reverse proxy
  or tailnet, HTTPS for the owner's own domain, privacy and network
  activity). R1.2 adds a path prefix (ACC-134). R2 adds the project's
  per-server name with its Certificate Transparency monitoring, built-in
  remote access over iroh, relays, the browser edge and upload budgets
  (D-07).
- **Purpose:** How the server is reached, who can reach it, and a truthful
  list of every outbound connection it makes.
- **Shows:** The posture in plain words, "Reachable from the internet: no",
  or through which proxy, with any listener on a globally routable address
  and a note that the server cannot see the router (SEC-NET-028,
  SEC-OPS-038); trusted proxies, each declared as a private overlay or as
  public, with any undeclared proxy seen and a one-step "Trust it"
  (ACC-097, SEC-NET-017, SEC-NET-019); a detected container gateway or NAT
  front-end with its fixes (SEC-NET-068); from R1.2, a path prefix
  (ACC-134); HTTPS: the owner's own domain with automatic certificates, a
  certificate the owner supplies, or a tailnet name, with the
  certificate's expiry (ACC-098, ACC-099, ADM-022, SEC-NET-072); the
  reverse-proxy and tailnet recipes for use away from home (SEC-NET-022);
  privacy choices (ADM-028); and the network activity page, which lists
  every outbound destination and the feature that caused it (ADM-129,
  SEC-NET-032). From R2: the project's per-server name, with Certificate
  Transparency monitoring for it (ADM-023, SEC-NET-069); remote access with
  no open ports (ACC-096); the browser edge, off until the owner turns it
  on (ACC-102, SEC-NET-043); self-hosted and default relays with what the
  relay operator can see (ACC-100, ACC-101, SEC-NET-039); the upload budget
  (ACC-109); and remux and transcode bandwidth limits (VID-028, VID-030).
- **Actions:** Configure, trust a proxy, renew a certificate now, test (the
  connectivity check is in SUR-102).
- **Form factors:** The general admin rule.
- **Security:** owner only. Trusted proxies, posture and remote access,
  TLS and naming, and egress policy are step-up actions, audited and
  announced to every admin (SEC-IAM-041, SEC-TM-017). Leaving the home
  posture raises an alert at the first internet-posture request
  (SEC-OPS-038). There is no port-mapping switch for the web listeners, and
  iroh's, from R2, is off unless the owner turns it on (SEC-NET-030,
  SEC-OPS-040).
  The trusted-proxy list starts empty, and forwarding headers from anyone
  else are ignored (SEC-NET-016).
- **Serves:** R1: ACC-097, ACC-098, ACC-099, ADM-022, ADM-028, ADM-129. R1.2: ACC-134. R2: ACC-096, ACC-100, ACC-101, ACC-102, ACC-109, ADM-023, VID-028, VID-030.

#### SUR-094 Integrations, tokens and webhooks
- **Release:** R2. API keys move from R1 to R2 with the baseline (security
  README decision 8, owner to confirm), so this surface has nothing to show
  in R1.
- **Purpose:** Let other tools in, narrowly, and send events out, safely.
- **Shows:** The key list, editor and detail, with scope by library and
  root within the creator's own rights (INT-017, INT-018, ACC-049), expiry
  and rotation (INT-019), last use and audit trail (INT-020), usage and
  throttling (INT-012), change-feed position (INT-006), revoke one or all
  (INT-021), and the no-escalation error (INT-022). Also connection recipes for
  Lidarr, Sonarr, Radarr, Bazarr, Seerr, Music Assistant and statistics
  tools (INT-025, INT-011, INT-097, INT-118, INT-119, INT-121, INT-124,
  INT-133); the integrations inventory (INT-029) and tokens still calling
  deprecated routes (INT-002); webhooks with an event picker, filters,
  templates with a live preview, signed payloads, a delivery log and a test
  button (INT-030 to INT-047, INT-049); the library event log (LIB-027);
  refresh tokens for download tools (LIB-026); and the Prometheus switch
  (INT-135). From R3: live TV and recording events (INT-053).
- **Actions:** Create, scope, rotate and revoke keys; create, test and
  redeliver webhooks.
- **Form factors:** The general admin rule; the template editor is a
  wide-screen editor.
- **Security:** a key is a 256-bit secret shown once and stored only as a
  keyed hash; its scopes come from a fixed list with no "all" and never
  include administrator, owner-only or host-equivalent rights, and never
  exceed the creator's own (SEC-EXT-008, SEC-EXT-010, SEC-IAM-083).
  Creating or changing a key needs an interactive session with step-up;
  no key can manage credentials (SEC-EXT-012). Keys expire, 365 days by
  default, and are disabled after 180 days unused, with a warning 14 days
  ahead (SEC-EXT-013). Webhooks stay off until the owner enables them with
  a list of allowed destination hosts; payloads carry only the fields
  listed for each event type, and events about a person's playback go only
  to their own webhooks or to admin webhooks they opted into; every
  delivery is signed; destinations are masked after creation; and the
  delivery log and test button show only success or failure (SEC-EXT-045
  to SEC-EXT-048). Metrics are off by default, and when on listen only on
  loopback or need a metrics-only key, with no per-user data
  (SEC-NET-046, SEC-OPS-059).
- **Serves:** R2: ACC-049, INT-006, INT-011, INT-012, INT-017, INT-018, INT-019, INT-020, INT-021, INT-022, INT-002, INT-025, INT-029, INT-030, INT-031, INT-032, INT-033, INT-034, INT-035, INT-036, INT-037, INT-038, INT-039, INT-040, INT-041, INT-042, INT-043, INT-044, INT-045, INT-049, INT-097, INT-118, INT-119, INT-121, INT-124, INT-133, INT-135, LIB-027. R3: INT-053. Later: INT-046, INT-047, LIB-026.

#### SUR-095 Plugins and providers
- **Release:** R2.
- **Purpose:** The WebAssembly plugin host and the metadata providers, each
  with its permissions shown before it is enabled (INT-054, INT-055,
  LIB-107).
- **Shows:** The plugin list with compatibility and publisher badges
  (INT-060, INT-065), the permission review sheet (INT-055), plugin detail
  with network allowlist, network log, limits, data and status (INT-056,
  INT-057, INT-061 to INT-063), generated settings forms (INT-059),
  first-party plugins (INT-066), third-party plugins from a plugin index
  the owner added after confirming its root key fingerprint (INT-067), a
  persistent banner naming every unsigned plugin when developer mode is on
  in the server's configuration file (INT-069); pending updates, which stay
  inactive until approved; the providers list with TMDB and its
  attribution (LIB-109), order and fallback per library (LIB-116, LIB-117,
  INT-077) and provider status (LIB-120); and the grant prompts that appear
  when a feature needs a network grant (VID-087, MUS-162; LIV-015 in R3).
- **Actions:** Review and grant, enable, disable, configure, approve an
  update, add an index, reorder providers.
- **Form factors:** The general admin rule. Per-user plugin consent happens
  on phone, web or desktop, never on a TV (SEC-EXT-039).
- **Security:** installing, updating, granting permissions to and removing
  plugins, and adding an index, are owner-only step-up actions, audited and
  alerted without a mute (SEC-EXT-038, SEC-OPS-034). Nothing runs until the
  owner has read the plain-language permission screen generated from the
  manifest (SEC-EXT-039). A plugin installs only if its signature verifies
  through a trusted index; there is no install from a bare URL, and
  developer mode cannot be switched on from any screen (SEC-EXT-035,
  SEC-EXT-037). Updates wait for approval unless the owner opted that
  plugin in to unchanged-permission updates after 72 hours (SEC-EXT-040).
  An index can only advise; it can never disable anything (SEC-EXT-041).
  Until the plugin host's requirements pass, no plugin runs at all
  (SEC-EXT-018).
- **Serves:** R2: INT-054, INT-055, INT-056, INT-057, INT-059, INT-060, INT-061, INT-062, INT-063, INT-065, INT-066, INT-067, INT-069, INT-077, LIB-107, LIB-109, LIB-116, LIB-117, LIB-120, MUS-162, VID-087. R3: LIV-015.

#### SUR-096 Compatibility adapters
- **Release:** R2.
- **Purpose:** The OpenSubsonic adapter and the Jellyfin adapter's music
  subset, off by default and under the same authorisation rules (ACC-130,
  INT-086, INT-098).
- **Shows:** Each adapter's switch and guardrails (INT-094), its own port
  and the HTTPS address apps must use, the apps that have connected
  (INT-096), certified clients and their quirks (INT-095), the music
  settings for the adapter (MUS-207), and any app key a person has marked
  as legacy.
- **Actions:** Enable, disable, revoke an app.
- **Form factors:** The general admin rule.
- **Security:** enabling an adapter is an owner step-up action, enrols
  nobody, and disabling it ends all its sessions at once (SEC-EXT-051). An
  adapter listens on its own port, never reads or sets cookies, exposes no
  administrative, user-management, file-browsing or share-management
  endpoint, and refuses every credential over plain HTTP (SEC-EXT-052,
  SEC-EXT-055, SEC-EXT-066). "Now playing" lists only the caller's own
  sessions (SEC-PRV-032). Token-and-salt sign-in stays refused unless the
  owner approves a written exception (SEC-EXT-069; owner to confirm,
  security README decision 8).
- **Serves:** R2: ACC-130, INT-086, INT-094, INT-095, INT-096, INT-098, MUS-207.

#### SUR-097 Migration
- **Release:** R1.1 (listening-service files and playlists, which moved
  from R1). R2 (an iTunes
  library file, dry runs, undo and mirror mode). Importers for rival
  servers' databases are Later, as the baseline's release table places them
  (SEC-TM-074).
- **Purpose:** Bring a household's history and playlists over from where
  they were, with reasons for every match (ADM-030, ADM-044).
- **Shows:** From R1.1: Last.fm and ListenBrainz export files, imported into the
  importing person's own history only (ADM-042), bulk playlist import with a
  match report (ADM-043), and the review of unmatched items (ADM-044). From
  R2: an iTunes library file (ADM-041), a dry-run report (ADM-045), undo
  (ADM-046), mirror mode (INT-116), Navidrome smart playlists (MUS-148,
  DIS-125), and the first-run import summary (MUS-045). Later: Plex,
  Jellyfin, Emby and Navidrome database imports (ADM-036 to ADM-040,
  MUS-141), importing over a rival's API (ADM-047), users as invitations
  (ADM-048, ACC-084) and syncing again during a switch (ADM-049).
- **Actions:** Upload or connect, review matches, import, undo (R2).
- **Form factors:** The general admin rule.
- **Security:** files arrive only through the upload route, within type,
  size and per-person limits, and are parsed in the core under its budgets
  (SEC-API-085, SEC-API-088). An admin's import writes only the admin's own
  history; each other adult imports their own from Account > Your data,
  because no admin may write, read or export another adult's history
  (SEC-PRV-022, SEC-PRV-025). Imported playlist entries match only items the
  importer can see (SEC-MED-051). When the Later importers come, a rival's
  database is opened read-only in a jailed worker (SEC-STD-031), and
  reading a rival's API is an owner step-up egress grant (SEC-IAM-041).
- **Serves:** R1.1: ADM-030, ADM-042, ADM-043, ADM-044. R2: ADM-041, ADM-045, ADM-046, DIS-125, INT-116, MUS-045, MUS-148.

#### SUR-098 Backups and export
- **Release:** R1. R1.2 adds restoring from these screens with a restore
  point and preview, and the documented export formats (ADM-070, ADM-074);
  R1.3 adds derived data to backups (ADM-141). Until R1.2 a restore runs at
  setup or with `gunmetal restore` on the host (ADM-029, ADM-071).
- **Purpose:** Tiny daily backups on by default, encrypted, signed,
  verified, downloadable and restorable with a preview (ADM-065, ADM-068,
  ADM-070).
- **Shows:** The backup list with status, verification and encryption
  (ADM-065, ADM-072, ADM-068), pre-upgrade snapshots (ADM-056), each
  backup's contents in plain words, including the audit log and its latest
  signed checkpoint (ADM-066, SEC-OPS-024), the identity store (ACC-013) and,
  from R1.3, derived data (ADM-141) included, how long backups are kept, 14 days by
  default (SEC-PRV-041), the recovery kit's status and "Make a new kit"
  (SEC-PRV-040), download and upload (ADM-069), and, from R1.2, restore
  with a restore point and preview (ADM-070) and the server's own settings
  export (ADM-074). From R2: cache snapshots (ADM-067) and off-site destinations,
  which receive only the encrypted file (ADM-073).
- **Actions:** Back up now, download (owner), restore (owner, R1.2), make a
  new recovery kit, export settings (R1.2), configure.
- **Form factors:** The general admin rule.
- **Security:** every backup is encrypted in age v1 to the server's backup
  key and the owner's recovery key, and signed; there is no unencrypted
  option (SEC-PRV-039, SEC-OPS-042, SEC-OPS-043). Encryption moves from R2
  to R1. Downloading or restoring a backup is an owner-only step-up action,
  audited and alerted (SEC-OPS-045, SEC-IAM-041). Restore checks the
  signature first, shows which server made the backup and its key
  fingerprint, and afterwards rotates all keys, invalidates every session,
  holds any setting looser than today's defaults and suspends restored
  devices until confirmed (SEC-OPS-043, SEC-OPS-044). Backups are never
  written under a web-served path or into a media root (SEC-OPS-045). The
  export here is the server's settings and the owner's own data only: each
  person exports their own data from Account > Your data, and no admin can
  export another adult's history (SEC-PRV-025, SEC-PRV-047). This replaces
  the earlier whole-server "full export".
- **Serves:** R1: ACC-013, ADM-056, ADM-065, ADM-066, ADM-068, ADM-069, ADM-072. R1.2: ADM-070, ADM-074. R1.3: ADM-141. R2: ADM-067, ADM-073.

#### SUR-099 Updates
- **Release:** R1.
- **Purpose:** An opt-in update check from a signed feed, with the rollback
  safety of every release stated (ADM-053, ADM-060).
- **Shows:** Whether the security-fix check is on, as the owner answered at
  setup (SEC-OPS-047); the current version, available updates, advisories
  that affect it and its end-of-support date (ADM-054, SEC-OPS-070); "Can't
  confirm you're up to date" after 7 days without a valid feed
  (SEC-SUP-050); and each release's upgrade notes: whether it migrates
  data, whether it can be rolled back, and which security defaults changed
  (ADM-060, SEC-OPS-052). From R2: release channels (ADM-055) and plugin
  compatibility before upgrading (ADM-063, INT-072).
- **Actions:** Check now, read notes, turn the check on or off (owner,
  step-up, since it is an egress grant), switch channel (R2).
- **Form factors:** The general admin rule.
- **Security:** the feed is static, signed and checked against a trust root
  built into the binary; the request carries no version or identifier, and
  matching happens on the server (SEC-OPS-019, SEC-SUP-051). The server
  never updates itself; this page only informs (SEC-OPS-046). The version
  is shown only to signed-in admins (SEC-NET-047).
- **Serves:** R1: ADM-053, ADM-054, ADM-060. R2: ADM-055, ADM-063, INT-072.

#### SUR-100 Tasks and activity
- **Release:** R1 (activity). R1.1 adds re-reads after a parser update
  (LIB-025); R1.2 adds the one task list with run, cancel and history
  (ADM-093); R1.3 adds loudness analysis progress (MUS-086, LIB-066,
  LIB-024).
- **Purpose:** What the server is doing and has done: one task list and one
  activity and audit log (ADM-093, ADM-110).
- **Shows:** From R1.2, tasks with run, cancel and history (ADM-093);
  throttled and resumable jobs (ADM-095); scan and job activity with an indicator in the admin
  header (LIB-022), including "0 changed", bytes read, moved files and
  re-reads after a parser update from R1.1 (LIB-016, LIB-017, LIB-019,
  LIB-029; LIB-025); loudness analysis progress from R1.3 (MUS-086,
  LIB-066, LIB-024);
  upgrades marked as upgrades (DIS-038); and sign-in, admin and recovery
  events (ACC-078, ADM-034), which open the security log (SUR-108). From
  R2: path-scoped refreshes (INT-011), reprioritise and cancel (LIB-023),
  schedules and a maintenance window (ADM-094), concurrency and pause
  (ADM-096), optimisation (ADM-081), plugin tasks (INT-071) and intro
  detection jobs (VID-112).
- **Actions:** Filter, open the item; run and cancel from R1.2.
- **Form factors:** The general admin rule. Not on TV (SEC-CLI-024).
- **Security:** activity entries name files and libraries but never what a
  person played (SEC-PRV-025). Security events live in the audit log, read
  in SUR-108.
- **Serves:** R1: ACC-078, ADM-034, ADM-095, ADM-110, DIS-038, LIB-016, LIB-017, LIB-019, LIB-022, LIB-029. R1.1: LIB-025. R1.2: ADM-093. R1.3: LIB-024, LIB-066, MUS-086. R2: ADM-081, ADM-094, ADM-096, INT-011, INT-071, LIB-023, VID-112.

#### SUR-101 Alerts and logs
- **Release:** R1.
- **Purpose:** Tell the administrator when something needs attention,
  through destinations that cost nothing (ADM-116), and keep structured
  logs (ADM-119).
- **Shows:** Alert rules and destinations (ADM-116), with the security
  alerts the owner receives and which of them can be muted: in R1, the
  in-app notice centre on every device the recipient is signed in on
  (SUR-003); free-space alerts (ADM-083); log settings with rotation
  (ADM-119); and sign-in failure lines for fail2ban (ADM-122). The owner's
  security alerts, in R1: a new device or credential on any admin account,
  and on any account to its holder; repeated failed sign-ins; a new user,
  an invitation redeemed, a role or library access raised, a new admin;
  internet exposure or an undeclared proxy; owner recovery; a backup
  downloaded or restored, and the "review devices and access" list after a
  restore; keys rotated; a security setting loosened outside the app; the
  certificate expiring in 30 or 7 days; an audit verification failure; and
  a running version affected by an advisory or out of support
  (SEC-OPS-031, SEC-OPS-032, SEC-OPS-044, SEC-NET-027, SEC-NET-072). From
  R2, with the project's per-server name and its Certificate Transparency
  monitoring, also a certificate for that name that the server did not ask
  for (SEC-NET-069). From R2: an alert inbox with
  acknowledge and snooze (ADM-117), opt-in email and webhook destinations
  that carry only the event type, time and a link (SEC-OPS-035), a log
  viewer (ADM-118), log levels per module with automatic revert (ADM-120),
  and plugin failures (INT-062). From R3: source, guide and recording
  alerts (LIV-044, LIV-060, LIV-061, LIV-132, LIV-155, LIV-156).
- **Actions:** Configure, acknowledge (R2), read and filter logs (R2). On a
  device or credential alert, "It was me" or "It wasn't me" (SEC-OPS-033).
- **Form factors:** The general admin rule.
- **Security:** alerts for a new admin, a new credential or device on an
  owner or admin account, owner recovery, internet exposure, a plugin
  install or permission grant, and an audit integrity failure have no mute
  switch; they can only be batched, with every distinct device listed
  (SEC-OPS-034). Re-signing in with a known passkey on a known device
  raises nothing, nobody is alerted about their own interactive action,
  and non-critical alerts form a daily summary (SEC-OPS-032, SEC-OPS-034).
  Diagnostic logs default to info, never record request bodies, query
  strings, cookies or titles, are readable only by the service account,
  and debug level switches itself off within 24 hours (SEC-OPS-029,
  SEC-PRV-043, SEC-PRV-045). The fail2ban line format is stable and
  documented (SEC-IAM-099, SEC-OPS-028).
- **Serves:** R1: ADM-083, ADM-116, ADM-119, ADM-122. R2: ADM-117, ADM-118, ADM-120, INT-062. R3: LIV-044, LIV-060, LIV-061, LIV-132, LIV-155, LIV-156.

#### SUR-102 Diagnostics
- **Release:** R1 (the doctor, the write queue and "rebuild the cache").
  R1.2 adds the diagnostic bundle, "inspect a file" and local crash records
  (ADM-124, ADM-125, ADM-130); R1.3 adds the derived-data store
  (ADM-141).
- **Purpose:** Find out what is wrong without a forum thread: the doctor,
  the diagnostic bundle, crash records and the rebuild button.
- **Shows:** Doctor results (ADM-123), the write queue (ADM-080), the
  database, and "rebuild the cache" (ADM-077); from R1.2, a diagnostic
  bundle with a preview and masking (ADM-124), "inspect a file" (ADM-125,
  opening SUR-088) and local crash records (ADM-130); and from R1.3, the
  derived-data store (ADM-141). From R2:
  the metrics endpoint's token (ADM-127), a remote connectivity check
  (ADM-134), the transcoding and sandbox self-tests (ADM-132, VID-009), and
  playback diagnostic bundles sent from players (VID-174).
- **Actions:** Run the doctor, build a bundle (R1.2), rebuild.
- **Form factors:** The general admin rule.
- **Security:** the doctor includes the security checks: root and
  capability state, file permissions, writable media or binary, internet
  exposure, trusted proxies, backup age and encryption, audit verification,
  version support and clock skew (SEC-OPS-061). A bundle is built from an
  allowlist of fields with no database, backups or secrets, replaces paths,
  titles, user names and addresses with pseudonyms, and is shown in full
  before download (SEC-PRV-046, SEC-OPS-030). The metrics token is a
  metrics-only key, never an admin key, and metrics carry no per-user data
  (SEC-OPS-059). The remote connectivity check runs only when the owner
  asks and sends only the address and port (SEC-OPS-074).
- **Serves:** R1: ADM-077, ADM-080, ADM-123. R1.2: ADM-124, ADM-130. R1.3: ADM-141. R2: ADM-127, ADM-132, ADM-134, VID-009, VID-174.

#### SUR-103 Server settings and about
- **Release:** R1. R1.2 adds the server's name and welcome message and the
  live footprint in About (ADM-140, ADM-010).
- **Purpose:** The server's own settings and identity.
- **Shows:** From R1.2, the server name and welcome message (ADM-140),
  with the note that neither is shown before sign-in; storage locations for settings, log,
  cache and scratch space (ADM-090); retention, with the private defaults:
  security events 365 days with addresses shortened after 30 and removed at
  90, diagnostic logs 14 days, backups 14 days, history until each person
  deletes it; the owner may shorten these, and the change is audited
  (ADM-111, SEC-PRV-005); stream limits, server-wide and per guest, with
  the recommended defaults (ACC-075, SEC-IAM-102, SEC-TM-068; named
  policies carry their own limits from R2, SUR-091); and About with the
  version, build and target (ADM-001), and from R1.2 the live footprint
  (ADM-010). From R2:
  each setting's
  source and whether it needs a restart (ADM-008, ADM-024), export and
  import with a diff (ADM-009), and switching whole modules off (ADM-097).
- **Actions:** Change, export, import.
- **Form factors:** The general admin rule.
- **Security:** the name and message are plain text, shown only to people
  who are signed in, on the invitation page, and as a remembered name on a
  device that has signed in before (SEC-NET-047, SEC-API-005; owner to
  confirm). The version is shown only here and in other signed-in admin
  views (SEC-NET-047). A settings import passes the live validators and
  never makes the server less strict without the owner's confirmation
  (SEC-OPS-049).
- **Serves:** R1: ACC-075, ADM-001, ADM-090, ADM-111. R1.2: ADM-010, ADM-140. R2: ADM-008, ADM-009, ADM-024, ADM-097.

#### SUR-104 Totals and statistics for administrators
- **Release:** R2.
- **Purpose:** How much the server is used and how it delivers, over time
  (ADM-106, ADM-101), without showing what anyone listened to or watched.
  The earlier entry, "play history and top users and titles across the
  server", is removed: the baseline forbids any admin view, search or
  export of another adult's history (SEC-PRV-025; owner to confirm,
  security README decision 5).
- **Shows:** Charts and tables by period: plays and hours for the server and
  per person, with no titles; the direct play, remux and transcode
  breakdown. A "popular on this server" list is Later, behind the owner's
  decision on household activity features (ACC-114); if it comes, it is
  built only from people who opted in and only from items at least three
  of them played, so it cannot single out one person (ADM-106).
- **Actions:** Change the period, filter by delivery method.
- **Form factors:** The general admin rule.
- **Security:** no per-person history, no top titles per person, and no
  export of anyone's activity (SEC-PRV-025, SEC-TM-054). Private sessions
  would never count towards a later "popular on this server" list, and
  joining it would be each person's choice, off by default (SEC-PRV-023,
  SEC-PRV-024).
- **Serves:** R2: ADM-101, ADM-106.

#### SUR-105 Trash
- **Release:** R1.
- **Purpose:** Items whose files were deleted keep their entries here for a
  grace period, so a file restored in time returns with everything attached;
  purging is an explicit action (LIB-033, ADM-086).
- **Shows:** Items, when they went missing, and when they will be purged.
- **Actions:** Restore, purge now.
- **Form factors:** The general admin rule.
- **Serves:** R1: ADM-086, LIB-033.

#### SUR-106 Video tools
- **Release:** R2.
- **Purpose:** The admin side of two video features: rules for pre-made
  smaller versions (VID-026) and the skip-marker editor (VID-110, VID-116).
- **Shows:** Optimisation rules and their jobs; markers per episode with
  their source and history.
- **Actions:** Add a rule, edit a marker, accept a reported fix.
- **Form factors:** The general admin rule; the marker editor needs a
  player, so it is not offered on TV (**Proposal**).
- **Serves:** R2: VID-026, VID-110, VID-116.

#### SUR-107 Household curation
- **Release:** R2.
- **Purpose:** The owner's curation for everyone: genre clean-up, the
  household's default Home, and the owner's picks (DIS-108, DIS-005,
  DIS-127).
- **Shows:** The genre list with merges (DIS-108, MUS-018), default Home
  layouts and copying a layout to someone (DIS-005, DIS-006), and the
  owner's picks row (DIS-127).
- **Actions:** Merge genres, set and copy defaults, pick items.
- **Form factors:** The general admin rule.
- **Security:** a default Home layout carries layout only; it never sets
  anyone's privacy settings (SEC-PRV-023). The owner's picks row shows each
  viewer only the picks they are allowed to see (SEC-IAM-070).
- **Serves:** R2: DIS-005, DIS-006, DIS-108, DIS-127, MUS-018.

#### SUR-108 Security log
- **Release:** R1.
- **Purpose:** The owner's view of the server's security audit log: every
  sign-in, failure, new device, recovery, grant, admin action and
  configuration change, so the owner can see what happened and prove the
  record was not altered (SEC-OPS-020, SEC-OPS-023).
- **Shows:** Events newest first, each with its time, kind, who acted from
  which device, what it touched and the outcome; other people's addresses
  shortened to /24, /48 or a country (SEC-OPS-027); noisy events such as
  failed authorisations folded into counts; the result of the last chain
  verification and the latest signed checkpoint (SEC-OPS-023); and the
  retention in force (SEC-PRV-005). An event that names a person links to
  that person in Users, never to their history.
- **Actions:** Filter by kind, person, device or time; verify the chain now;
  export checkpoints into the recovery kit or a second backup destination
  (SEC-OPS-075); for the owner, switch on an investigation mode that shows
  full addresses.
- **Form factors:** The general admin rule. Not on TV (SEC-CLI-024).
- **Security:** readable only by the owner and holders of the audit
  capability (SEC-OPS-027, SEC-IAM-097). Investigation mode is itself
  audited, and each person whose full address it reveals is told
  (SEC-OPS-027). Records carry no secrets, tokens, setup codes, media
  titles or file paths (SEC-OPS-021). Nobody can clear or edit the log;
  records leave only on the retention schedule, each prune writing a signed
  checkpoint (SEC-OPS-026). Exporting the log is recorded where every
  affected person can see it. A verification failure raises an owner alert
  that cannot be muted (SEC-OPS-034).
- **Serves:** R1: ADM-110.

#### SUR-109 Plain-HTTP and home-posture help pages
- **Release:** R1.
- **Purpose:** What a browser sees when it reaches the server by a door that
  carries nothing sensitive: plain HTTP from anywhere but loopback
  (SEC-NET-001), a non-local address while the server is in the home
  posture (SEC-NET-024), or a reverse proxy the owner has not declared
  (SEC-NET-017). It replaces every sign-in form, player and API on those
  doors, and it never dead-ends.
- **Shows:** A static page that sets no cookie and holds no form. Over plain
  HTTP from the home network, an immediate redirect to the server's HTTPS
  address when one exists (the owner's domain or tailnet name), and if
  that address has not loaded within a few seconds, plain steps: use the
  tailnet name, or open Gunmetal on the server itself (SEC-NET-001). With
  no HTTPS address yet, how to get one: a domain the owner holds, a
  tailnet name, or an SSH tunnel to localhost for setup (ADM-021,
  ACC-098). From R2, with the project's per-server name, the steps also
  say how to allow that name in the router's DNS rebinding settings
  (SEC-NET-071). When the certificate is missing or expired, that the server
  cannot be reached securely right now and that the owner has been told
  (SEC-NET-005). From outside the home in the home posture, only that this
  server cannot be reached from here (SEC-NET-024). Through an undeclared
  proxy, "Configure your proxy", with no detail about the server
  (SEC-NET-017).
- **Actions:** Follow the link to the HTTPS address.
- **Form factors:** Any browser; native apps never use plain HTTP
  (SEC-NET-061).
- **Security:** the page shows no server name, version, library or user,
  and accepts no credential of any kind: no passkey ceremony, cookie,
  invitation, PIN, share password, pairing or setup code (SEC-NET-001,
  SEC-NET-047). API routes over plain HTTP answer 403; browser navigations
  may only redirect to the configured HTTPS URL. Every non-local request
  that reaches a home-posture listener writes a security event and an
  owner alert, at most once per listener per day with a count
  (SEC-NET-027). The help pages are the only part of the client a stranger
  can reach, so the route-table replay over the plain listener proves this
  surface for every route (SEC-NET-001).
- **Serves:** R1: ACC-098, ADM-021, CLI-150.

### Outside the client

#### SUR-110 Command line
- **Release:** R1. The diagnostic bundle and file inspector commands
  arrive in R1.2 (ADM-124, ADM-125).
- **Purpose:** Everything an operator needs when the interface is not
  available, or when scripting: install as a service, check configuration,
  recover the owner, back up and restore, rebuild, and diagnose.
- **Shows:** The claim link, its terminal QR code and the setup code on the
  console, each failed code attempt with its address and time, and the
  "claimed by" line after the claim; `gunmetal claim-code` to show the code
  again or mint a new one after it expires (ACC-001, ADM-018, SEC-IAM-007,
  SEC-OPS-004); `gunmetal doctor` and `doctor --security` (ADM-123, with the
  checks in ADM-006, ADM-078, ADM-079, ADM-089, and SEC-OPS-061); `gunmetal
  trust-proxy` (SEC-NET-017); `gunmetal audit verify` (SEC-OPS-023); owner
  recovery, over a local socket only the service account can open, printing
  a 15-minute enrolment link and alerting every admin (ACC-004, ADM-034,
  SEC-IAM-092); service installation
  (ADM-005); configuration and migration checks (ADM-007, ADM-057); restore
  (ADM-071); `gunmetal rebuild` (ADM-077); and, from R1.2, the diagnostic
  bundle (ADM-124) and the file inspector (ADM-125, LIB-195). From R2: settings export and
  import (ADM-009), filesystem guidance (ADM-091) and the transcoding and
  sandbox self-tests (ADM-132, ADM-133).
- **Actions:** Run commands.
- **Form factors:** A terminal on the server host.
- **Security:** no command takes a secret as a command-line argument; secrets
  come from files or systemd credentials (SEC-OPS-014). Owner recovery and
  the claim code are reachable only on the host, never over the network
  (SEC-IAM-092, SEC-IAM-007). The server refuses to start as root
  (SEC-OPS-053).
- **Serves:** R1: ACC-001, ACC-004, ADM-005, ADM-006, ADM-007, ADM-018, ADM-034, ADM-057, ADM-071, ADM-077, ADM-078, ADM-079, ADM-089, ADM-123. R1.2: ADM-124, ADM-125, LIB-195. R2: ADM-009, ADM-091, ADM-132, ADM-133.

#### SUR-111 Project site and documentation
- **Release:** R1. R1.1 adds published speed numbers (DIS-019); R1.2 adds
  footprint numbers, the hardware guide and Home layout notes (ADM-010,
  ADM-011, DIS-015); R1.3 adds 32-bit ARM builds (ADM-004).
- **Purpose:** The pages on gunmetal.tv and in the docs that features
  depend on: downloads, install guides, the hardware guide, benchmarks,
  advisories and release notes.
- **Shows:** The download page and builds (ADM-001; 32-bit ARM from R1.3,
  ADM-004; installers and
  outside-store installs in R2, ADM-013, CLI-017, ADM-014), install docs
  with a compose example (ADM-002, ADM-003, ADM-005, ADM-089, ADM-090), the
  hardware guide sized for direct play (ADM-011, R1.2), published footprint
  and speed numbers (ADM-010, R1.2; DIS-019, R1.1; time to first frame in
  R2, VID-012), the reverse-proxy and Tailscale Serve recipes for use away
  from home in R1 (SEC-NET-022), the
  browser support list (CLI-002), an advisory for every fix (ACC-126),
  release notes with before-and-after screenshots for any layout change
  (DIS-015, CLI-031, MUS-113), health endpoints and fail2ban guidance
  (ADM-128, ADM-122), and the docs site's API pages (INT-001). From R2: the
  integrator changelog, conformance suite, SDKs and plugin SDK (INT-003,
  INT-004, INT-015, INT-069), client quirk notes (INT-095), signed
  repositories and NAS templates (ADM-012, ADM-015), and declarative setup
  (ADM-024).
- **Actions:** Read, download.
- **Form factors:** A website for any browser.
- **Serves:** R1: ACC-126, ADM-001, ADM-002, ADM-003, ADM-005, ADM-058, ADM-089, ADM-090, ADM-122, ADM-128, CLI-002, CLI-031, INT-001, MUS-113. R1.1: DIS-019. R1.2: ADM-010, ADM-011, DIS-015. R1.3: ADM-004. R2: ADM-012, ADM-013, ADM-015, ADM-024, ADM-091, CLI-017, INT-003, INT-004, INT-015, INT-069, INT-095, VID-012. Later: ADM-014.

#### SUR-112 API reference page
- **Release:** R1.
- **Purpose:** The API reference generated from code, served by each server
  so it always matches that server's version (INT-001).
- **Shows:** Endpoints and types; from R2, deprecations (INT-002), a built-in
  explorer (INT-014) and the event catalogue (INT-031).
- **Actions:** Read; try a call in the explorer (R2).
- **Form factors:** Web and desktop browsers.
- **Serves:** R1: INT-001. R2: INT-002, INT-014, INT-031.

#### SUR-113 Other apps' own interfaces
- **Release:** R2.
- **Purpose:** Some features are delivered through someone else's interface
  by design: Sonos and whole-home audio through Music Assistant (MUS-206,
  INT-097), and existing Subsonic apps through the adapter. Gunmetal does
  not design these screens; it designs how they connect (SUR-094, SUR-096).
- **Shows:** Not Gunmetal's to define.
- **Actions:** Not Gunmetal's to define.
- **Form factors:** Whatever the other app supports.
- **Serves:** R2: INT-097, MUS-206.

### Live TV

Every surface in this group is R3, and appears only when the live TV module
is switched on (LIV-001). The feature map contains no R1 or R2 live TV rows;
these entries are here so that R1 and R2 navigation leaves room for them.

#### SUR-120 Live TV home
- **Release:** R3.
- **Purpose:** What is on now and soon, as rows, from the household's own
  sources (LIV-071, DIS-041).
- **Shows:** Rows such as on now, starting soon, favourites (LIV-055) and
  recordings; kids limits applied (DIS-158).
- **Actions:** Watch, open the guide, record.
- **Form factors:** *TV:* a rail entry. *Phone:* the Live tab (**Proposal**).
  *Tablet and web:* a sidebar or rail entry.
- **Serves:** R3: ACC-036, DIS-041, DIS-096, DIS-158, LIV-001, LIV-055, LIV-071, LIV-077.

#### SUR-121 Guide
- **Release:** R3.
- **Purpose:** A grid guide that is fast with a remote and usable with touch,
  drawn from data synced to the device (LIV-064 to LIV-066).
- **Shows:** Channels and programmes with artwork (LIV-045), a now line and
  day and time pickers (LIV-067), filters (LIV-069), "already in your
  library" badges (LIV-074), catch-up and past programmes (LIV-099,
  LIV-100), programme details (LIV-070), and a preview pane or background
  video while browsing (LIV-068).
- **Actions:** Watch, start over (LIV-097), record (LIV-102, LIV-107),
  remind (LIV-075), search and jump by channel number (LIV-072, LIV-073).
- **Form factors:** *TV:* the grid built for a remote (LIV-064). *Phone,
  tablet, web:* the touch and pointer grid (LIV-066).
- **Serves:** R3: CLI-132, LIV-045, LIV-054, LIV-055, LIV-064, LIV-065, LIV-066, LIV-067, LIV-068, LIV-069, LIV-070, LIV-072, LIV-073, LIV-074, LIV-097, LIV-099, LIV-100, LIV-102, LIV-107, LIV-169.

#### SUR-122 Live player
- **Release:** R3.
- **Purpose:** Play the broadcast as it is on native clients (LIV-078), with
  fast channel changes (LIV-080).
- **Shows:** A mini guide over playback (LIV-082), captions and audio menus
  (LIV-083, LIV-084), a stats overlay (LIV-086), a timeline for pause and
  rewind (LIV-095), "start over" and "go to live" (LIV-097, LIV-098), a
  tuner-busy sheet with choices (LIV-087, LIV-030), which names another
  person or their channel only if that person shares what they play with
  the household, and otherwise says only that the tuner is busy
  (SEC-PRV-022; owner to confirm), a warning when a recording takes
  priority (LIV-127), the browser capacity warning (LIV-079), and parental
  blocks (LIV-164). Channel names, logos and programme text come through
  the server and are shown as text (SEC-CLI-067).
- **Actions:** Channel up and down, last channel and number entry (LIV-081,
  CLI-132), record (LIV-088), picture-in-picture (LIV-089), cast (LIV-090,
  CLI-133), sleep timer (LIV-092), skip a break (LIV-146, VID-117).
- **Form factors:** *TV:* remote keys and an on-screen number pad. *Phone and
  tablet:* touch, picture-in-picture. *Web:* works, with a capacity warning:
  the server remuxes when only the container blocks the browser, but MPEG-2,
  some AC-3 and interlaced channels must be transcoded in the sandbox, which
  costs CPU (LIV-079).
- **Serves:** R3: CLI-132, CLI-133, LIV-005, LIV-030, LIV-078, LIV-079, LIV-080, LIV-081, LIV-082, LIV-083, LIV-084, LIV-085, LIV-086, LIV-087, LIV-088, LIV-089, LIV-090, LIV-091, LIV-092, LIV-095, LIV-097, LIV-098, LIV-125, LIV-127, LIV-139, LIV-146, LIV-164, LIV-169, MUS-177, VID-117.

#### SUR-123 Recordings
- **Release:** R3.
- **Purpose:** One hub for recordings: in progress, upcoming, conflicts,
  rules and trash (LIV-141).
- **Shows:** Tabs for recorded, in progress (LIV-098), upcoming with agenda
  and week views (LIV-130), conflicts (LIV-128) and trash (LIV-136); series
  rule screens with every option (LIV-112 to LIV-118, LIV-121) and a rule
  builder with live preview (LIV-119); recording detail with health and
  gaps (LIV-122, LIV-153, LIV-154, LIV-157); "Mine" and "Household" filters
  (LIV-166). Finished recordings also join the ordinary film and show pages
  (LIV-135).
- **Actions:** Record, skip one airing (LIV-120), start now (LIV-131),
  reorder rules (LIV-129), stop or cancel (LIV-109), restore.
- **Form factors:** *TV:* view, play and simple scheduling. *Phone:*
  scheduling from anywhere (LIV-107). *Tablet and web:* everything,
  including the rule builder.
- **Serves:** R3: LIV-044, LIV-046, LIV-098, LIV-103, LIV-104, LIV-108, LIV-109, LIV-112, LIV-113, LIV-114, LIV-115, LIV-116, LIV-117, LIV-118, LIV-119, LIV-120, LIV-121, LIV-122, LIV-128, LIV-129, LIV-130, LIV-131, LIV-132, LIV-135, LIV-136, LIV-141, LIV-153, LIV-154, LIV-157, LIV-166.

#### SUR-124 Live TV administration
- **Release:** R3.
- **Purpose:** Sources, tuners, guide data, the lineup and recording
  defaults.
- **Shows:** A guided set-up wizard for phone, tablet and web on a
  personal device (LIV-002);
  sources with filters, refresh, mirrors, logins, headers, connection limits
  and a test panel (LIV-003, LIV-007, LIV-008, LIV-010, LIV-012, LIV-013,
  LIV-016, LIV-126); tuners (LIV-024, LIV-025, LIV-029); guide sources and
  settings (LIV-035, LIV-037 to LIV-043); channel mapping (LIV-050, LIV-051);
  the lineup editor (LIV-011, LIV-053, LIV-054, LIV-056, LIV-059); channel
  detail (LIV-052, LIV-057, LIV-058); time-shift, recording destinations,
  format, naming and defaults (LIV-096, LIV-105, LIV-108, LIV-134, LIV-138);
  power and wake (ADM-098, LIV-110); history with undo (LIV-063); rights and
  channel profiles (ACC-048, LIV-162, LIV-163); and an activity log
  (LIV-160).
- **Actions:** Add, test, map, edit, reorder, undo.
- **Form factors:** The general admin rule: the set-up wizard is built for
  touch as well as wide screens (LIV-002), and the lineup editor supports
  keyboard and touch multi-select (LIV-059), on phones, tablets and the
  web on personal devices. Neither appears on TV, which is a limited
  device and never shows admin surfaces; the server refuses these routes
  from it whatever the client shows (SEC-CLI-024). On a TV, Live TV
  settings offer only an "Open on your phone" QR code that opens this
  surface on a phone or computer, as every admin task on TV does. LIV-002
  and LIV-059 no longer name the TV.
- **Security:** each source URL is an egress grant, which is an owner
  step-up action; fetches refuse loopback, link-local, the server's own and
  cloud metadata addresses unless the owner allows one named LAN tuner, and
  entries outside a source's grant are not fetched (SEC-EXT-075,
  SEC-NET-067, SEC-IAM-041). Provider logins and headers are stored
  encrypted and never sent to clients (SEC-EXT-050, SEC-CLI-067). Guide
  files with a DOCTYPE are rejected (SEC-MED-056). Recordings go only to a
  dedicated recordings folder with a quota (SEC-OPS-063).
- **Serves:** Later: INT-157 (a reference to LIV-171, which waits for its export-credential record). R3: ACC-048, ADM-098, INT-084, LIV-001, LIV-002, LIV-003, LIV-006, LIV-007, LIV-008, LIV-009, LIV-010, LIV-011, LIV-012, LIV-013, LIV-014, LIV-015, LIV-016, LIV-017, LIV-024, LIV-025, LIV-026, LIV-029, LIV-035, LIV-037, LIV-038, LIV-039, LIV-040, LIV-041, LIV-042, LIV-043, LIV-047, LIV-050, LIV-051, LIV-052, LIV-053, LIV-054, LIV-056, LIV-057, LIV-058, LIV-059, LIV-061, LIV-062, LIV-063, LIV-096, LIV-104, LIV-105, LIV-108, LIV-110, LIV-126, LIV-134, LIV-137, LIV-138, LIV-140, LIV-156, LIV-160, LIV-161, LIV-162, LIV-163.

#### SUR-125 Radio
- **Release:** R3.
- **Purpose:** Radio channels and internet radio stations open in the music
  player, not the video player (LIV-167, LIV-168, MUS-177).
- **Shows:** A Radio section in Music with stations, a station editor, and
  virtual channels (LIV-169).
- **Actions:** Play in the now-playing bar, add and edit stations.
- **Form factors:** As the music surfaces.
- **Serves:** R3: LIV-167, LIV-168, LIV-169, MUS-177.

### Surfaces that Later rows would add

These are named so that R1 and R2 leave room for them; they are not
designed here and have no IDs yet.

| Surface | Later rows that name it | Room left for it now |
|---|---|---|
| Books tab, book page, chapter sheet, bookmarks, book mode in the player | LAT-015 to LAT-039 | The queue's named contexts (LAT-009) and the Music and Watch switch, which can take a third position for spoken word |
| Podcast show page, a next-episodes list across shows, effects sheet, podcast search | LAT-058 to LAT-090 | The same switch and contexts; the plugin host in R2 |
| Photos tab, grid, viewer and date timeline | LAT-106, LAT-113 to LAT-119 | Kind chips in search and library (LAT-013) |
| Watch together and listen together: a group session sheet and a guest join screen | ACC-093, ACC-094, CLI-114 | The single-item guest capability in R2 (ACC-135) and the public share page (SUR-059) |
| Apple platforms: iPhone, iPad, Apple TV, CarPlay, Top Shelf | CLI-005, CLI-007, CLI-117, CLI-049 | Every surface above is written for the shared UI; only the system surfaces differ |
| Watch apps with offline music | CLI-128, CLI-129 | SUR-056 covers control only |
| Home-screen widgets, configurable gestures, alternative icons | CLI-073, CLI-153, CLI-154 | Settings sections (SUR-073, SUR-076) |
| Multi-room grouping and other renderers in the device picker | CLI-105, CLI-109, CLI-113 | The device picker (SUR-014) |
| Ten-foot mode on a PC | CLI-015 | The TV layouts are the same code |
| Admin roles, join requests, recently deleted users | ACC-041, ACC-085, ACC-009 | Users and invitations (SUR-090) |

## Coverage

### Method and result

Every row in the ten area files was parsed, and each R1, R1.1, R1.2, R1.3,
R2 and R3 row's "UI surfaces" cell was split into the surfaces it names.
Each named surface was assigned to an entry above, by rule and then by hand where a name was
ambiguous (for example "detail pages" means the album and artist pages in
R1 and also the title page in R2). A row counts as having a home when at
least one of its named surfaces is an entry above that exists in the row's
own release. Rows whose cell says "None" were checked one by one: those with
a qualifier that names a surface ("None beyond the lyrics view", "None
directly; feeds the pre-play sheet") were given that surface, and the rest
are listed in the behaviour-only table below.

| Release | Rows | Rows with a home in their release | Rows with no UI entry (behaviour only) | Rows with no home yet |
|---|---:|---:|---:|---:|
| R1 | 296 | 233 | 26 | 37 |
| R1.1 | 79 | 78 | 0 | 1 |
| R1.2 | 43 | 43 | 0 | 0 |
| R1.3 | 32 | 30 | 2 | 0 |
| R2 | 685 | 644 | 27 | 14 |
| R3 | 157 | 147 | 0 | 10 |

The counts were recomputed on 2026-10-03 from the Serves lines above, with
each row in the release the owner's answers give it: the register's
[R1 scope](../decisions.md#r1-scope) for the rows of the earlier R1 (with
the per-server name, ADM-023, in R2 under D-07), and the feature map for
every other row. The first draft's counts (422, 696 and 154) predate both
the security alignment and the point releases.

The rows with no home yet are of three kinds, and none is hidden:

1. **Fourteen R1 rows whose only named surface moved to a point release**
   with that surface's own row: the track info sheet (SUR-013, R1.1), the
   file inspector (SUR-088, R1.2) and the review queue (SUR-087, R1.3).
   Each still works in R1; [the table below](#r1-rows-whose-detail-surface-moved-to-a-point-release)
   says where its result shows.
2. **One R1.1 row, LIB-112**, whose named surface is the R2 artwork picker;
   its R1.1 reach is in the table of rows whose named surface arrives
   later.
3. **Rows that no Serves line names yet, most of them added to the
   feature map or moved by the security alignment of 2026-10-02.** In R1: ACC-005, ACC-009, ACC-136,
   ACC-139, ADM-144 to ADM-148, CLI-155 to CLI-157, CLI-159,
   DIS-186 to DIS-189, LIB-204 to LIB-207, MUS-233 and MUS-234. In R2:
   ACC-131, ACC-132, CLI-158, DIS-190, INT-088, INT-160 to INT-164,
   MUS-216 and VID-186 to VID-188. In R3: LIV-004, LIV-036, LIV-133,
   LIV-158, LIV-159, LIV-165 and LIV-177 to LIV-180. Most already appear in
   an entry's text ("Rotate all server secrets" in SUR-083 is ADM-144, "Is
   this your own device?" in SUR-070 is CLI-155, deleting a period of
   history in SUR-029 is MUS-233); giving them Serves lines is the next
   coverage pass, not a release question.

Every R1.2 and R1.3 row with a UI entry has a home in its own release. The
remaining tables record the rows that have no UI by design, the rows whose
named surface only partly exists in their release, and the features that
have no home of Gunmetal's own.

The security alignment also moved the desktop shell and its rows to Later (CLI-014,
CLI-063, CLI-064, CLI-065, CLI-066, MUS-081, MUS-082, MUS-102), because the
baseline's release scope puts the shell there (SEC-TM-074) and its controls
are SEC-CLI-069, also Later (owner to confirm). The Serves lines above list
those rows under "Later", and SUR-052 keeps its ID with the release Later.
The parts of CLI-067 and CLI-068 that need a native app stay in R2 on
Android.

### Behaviour-only rows

These rows have no UI entry, because they are rules of a component
or invisible by design. They need no surface, but most need a test, and some
are promises that belong in documentation.

| ID | Feature | Release | What the map says instead of a surface |
|---|---|---|---|
| ACC-030 | Every path obeys the restrictions | R1 | None (behaviour), tested on every browse screen |
| ACC-110 | Downloads give way to streams | R2 | None (behaviour) |
| ACC-120 | Every endpoint needs sign-in | R1 | None (behaviour) |
| ACC-121 | Authorization on every object | R1 | None (behaviour) |
| ACC-122 | Short-lived, session-bound stream URLs | R1 | None |
| ACC-123 | Secrets never in URLs or logs | R1 | None |
| ACC-124 | Browser sessions that page scripts cannot steal | R1 | None |
| ACC-128 | Nobody can switch your server off remotely | R1 | None (published promise) |
| ADM-020 | Setup closes for good once an admin exists | R1 | None after setup |
| ADM-121 | Secrets that cannot reach logs | R1 | None (behaviour) |
| CLI-066 | A light desktop shell | Later (with the shell, SEC-TM-074) | None |
| CLI-071 | Interruptions handled cleanly | R2 | None |
| CLI-110 | Local casting and a phone relay | R2 | None (automatic) |
| CLI-136 | Accessibility as a release gate | R1 | None |
| DIS-144 | Limits enforced at sync | R2 | None visible: the absence is the feature |
| INT-005 | Capability discovery | R1 | None (API only) |
| INT-007 | Consistent list endpoints | R1 | None (API only) |
| INT-013 | Health check for uptime monitors | R1 | None |
| INT-023 | Credentials in headers only | R1 | None (docs) |
| INT-026 | "Who am I" check for tools | R2 | None (API only) |
| INT-048 | Server-sent event stream | R2 | None (API only) |
| INT-089 | Honest extension discovery | R2 | None |
| INT-090 | OpenSubsonic lyrics, playback report, index-based queue and transcoding | R2 | None |
| LAT-001 | Item kinds in the data model | R1 | None in R1 |
| LAT-002 | People with typed roles | R1.3 | None in R1.3; person pages from R2 |
| LAT-008 | Typed links between items | R1.3 | None in R1.3 |
| LAT-009 | Listening contexts in the queue protocol | R1 | None in R1 |
| LAT-103 | Free remote viewing of music videos | R2 | None |
| LIB-011 | Room for other media kinds | R1 | None in R1 |
| LIB-018 | Library change feed | R1 | None directly; feeds every browse screen |
| LIB-020 | No helper process per file | R1 | None |
| LIB-028 | Stable identity for every file | R1 | None directly |
| LIB-036 | Multi-value tags in every format | R1 | None directly |
| LIB-045 | Albums built from tags | R1 | None directly |
| LIB-064 | Gapless data | R1 | None here; the player uses them |
| LIB-073 | More audio formats | R2 | None |
| LIB-074 | Movie names | R2 | None (results appear in browse) |
| LIB-076 | Episode names | R2 | None |
| LIB-078 | Date-based episodes | R2 | None |
| LIB-088 | Segment map | R2 | None |
| LIB-104 | New episodes get real titles | R2 | None |
| LIB-143 | Safe image handling | R1 | None |
| MUS-075 | Interruptions handled | R2 | None (behaviour) |
| MUS-085 | Opus gain done right | R1 | None (behaviour) |
| MUS-230 | Audio packaging for the web player | R1 | None (used by the player) |
| VID-011 | Short-lived, session-bound stream URLs | R2 | None visible |
| VID-018 | Correct shape and orientation | R2 | None |
| VID-032 | Fewer stalls on long or lossy links | R2 | None |
| VID-033 | Playback has priority over downloads and syncs | R2 | None |
| VID-035 | Remote playback is free | R2 | None |
| VID-037 | HDR10+ without black screens | R2 | None |
| VID-038 | HLG | R2 | None |
| VID-121 | Progress survives replacing or renaming the file | R2 | None |
| VID-124 | Next item ready before it starts | R2 | None |
| VID-136 | Interruptions handled | R2 | None |
| VID-150 | Casting that works when the server is far away | R2 | None |

### R1 rows whose detail surface moved to a point release

These fourteen R1 rows still ship in R1, but the surface that shows their
detail moved to a point release with its own row. Each line says where
their result shows in R1.

| ID | Surface that moved | What R1 shows |
|---|---|---|
| INT-009, MUS-036 | Track info sheet (SUR-013, R1.1) | MusicBrainz IDs on music items in the API (SUR-112), and the identity they give albums and artists on the artist and album pages (SUR-024, SUR-025) |
| MUS-032, MUS-034, MUS-037 | Track info sheet (SUR-013, R1.1) | Their results: core formats that play, with the format badge on rows and tiles (SUR-023), and multi-value credits as tagged on artist and album pages (SUR-024, SUR-025) |
| MUS-067, MUS-069, MUS-084, MUS-089, LIB-065 | Track info sheet (SUR-013, R1.1) | Gapless playback and gain in the player, with the gain mode in the full-screen player's info (SUR-010, MUS-087) |
| LIB-046, LIB-059, LIB-097 | File inspector (SUR-088, R1.2) | Their results in browse: albums grouped by MusicBrainz IDs and identified without the internet (SUR-022, SUR-024, SUR-025); every raw tag is kept for the inspector |
| LIB-051 | Review queue (SUR-087, R1.3) | Same-titled albums kept apart on the album grid and the artist page (SUR-022, SUR-024); doubtful cases wait for the review queue |

### Rows whose named surface arrives later

These rows have a home in their own release, but their "UI surfaces" cell
also names a surface, or a part of one, that does not exist until a later
release. Each line says what the home in the row's own release is. The
feature map is not wrong here: a feature is placed in the first release in
which it ships on at least one client
([README](../features/README.md#releases)). This table makes that reach
explicit for the people building R1 and its point releases.

| ID | Named surface that arrives later | Home in the row's own release |
|---|---|---|
| ACC-011, ACC-017 | Profile picker (ACC-019, R2) | Account menu (SUR-006) and the profile editor (SUR-079) |
| ACC-079 | Admin > Policies > Sessions (policies are ACC-038, R2) | Sign-in and security settings (SUR-092) and "Is this your own device?" on sign-in (SUR-070) |
| ADM-011 | The setup checklist (ADM-033, R2) | Documentation (SUR-111); **Proposal:** a link from the welcome flow (SUR-082) |
| CLI-026 | The "Downloaded" filter (downloads are R2) | The rule that lists keep working offline (SUR-000, SUR-023); the filter has nothing to show until R2 |
| CLI-034, INT-147 (R1.2) | The native share sheets and film and episode links (R2) | "Copy link" in the context menu (SUR-004) and the R1.2 share sheet for music (SUR-058) |
| LIB-098 (R1.1) | The file inspector (SUR-088, R1.2) | "Why is this here?" in the track info sheet, without file paths (SUR-013) |
| LIB-111 (R1.1) | Admin > Review queue (SUR-087, R1.3) | "Look up on MusicBrainz" in an album's context menu, for administrators (SUR-004), switched on in Welcome > Privacy and Network (SUR-082, SUR-093) |
| LIB-112 (R1.1) | The artwork picker (SUR-089, R2) | Covers fetched for albums with no local art, shown on the album page and every tile (SUR-025, SUR-023), switched on in Welcome > Privacy and Network (SUR-082, SUR-093) |
| CLI-040, DIS-163 | TV letter column and car lists (R2) | The web letter column and the phone-width fast scroller (SUR-023) |
| DIS-045, DIS-047, DIS-050, DIS-051, DIS-060, LIB-146 | "Detail pages", which for films are R2 | Artist and album pages (SUR-024, SUR-025) |
| DIS-087 | Person pages (R2) | The artist page's role tabs (SUR-024) |
| LIB-041, LIB-048 | The edit sheet (LIB-172, R2) | "Merge with" and "Split" in the context menu and the corrections list (SUR-089); release types come from tags (SUR-024) |
| LIB-135 | The artwork picker (R2) | Folder artwork shown on the album page and every tile (SUR-025, SUR-023) |
| MUS-047 | Child-profile settings (R2) | The explicit badge on track rows (SUR-023) |
| MUS-073, CLI-070 | Native lock screens (R2) | The browser's media controls, where the browser supports them; unreliable on iPhone (unverified, CLI-003) |

### Features with no home

Only one R1 or R2 feature has no surface of Gunmetal's own, and it has none
by design. Four more have a home for part of what they name, while the rest
belongs to a platform that is Later. None of them is a gap in this
inventory; each is a decision to note.

| ID | Release | What has no home | Why | What to do |
|---|---|---|---|---|
| MUS-206 | R2 | Sonos and whole-home audio | It is delivered entirely through Music Assistant's own interface over the OpenSubsonic adapter (owned by INT-097) | Nothing in the UI; keep the connection recipe (SUR-094) and the adapter switch (SUR-096) |
| CLI-049 | R2 | The Apple TV Top Shelf half of "TV home-screen rows" | Apple platforms are Later | Ship the Android TV launcher rows (SUR-055); move Top Shelf with the Apple rows |
| CLI-090 | R2 | The Live Activity half of "background downloads that resume" | Live Activities are an iOS feature; iPhone is Later | Android uses the notification (SUR-053) |
| DIS-171 | R2 | The CarPlay half of "car browsing from your home" | CarPlay is Later (CLI-117) | Android Auto ships in R2 (SUR-054) |
| INT-148 | R2 | The Siri Shortcuts part of "operating-system automation hooks" | Apple platforms are Later | Android intents and NFC ship in R2 (SUR-057) |

**Recommendation:** split CLI-049, CLI-090, DIS-171 and INT-148 so that
their Apple halves become Later rows of their own, which would make every
R2 row fully homed. That edit belongs in the feature map, not here.

## Where rivals are already good

The research shows that much of this navigation is not new, and Gunmetal
should say so rather than claim it.

- **Spotify's desktop three-pane layout** (library, content, now playing or
  queue, and a bottom bar) set the pattern that Feishin and others copy, and
  it is the model for CLI-060. The edge Gunmetal can claim is narrow: a
  queue pane that can grow to full height after Spotify's 2024 change drew
  complaints, and no ads or podcasts in the Home.
- **Spotify's phone app** keeps a few tabs with one clear job each, and its
  April 2026 tablet app reconfigures for the extra width rather than
  stretching. Gunmetal's phone and tablet frames follow the same approach.
- **Plexamp** is, by the music map's account, the best self-hosted
  listening experience, and its UltraBlur backgrounds are free. Its lyrics,
  equaliser and arrangeable Home need Plex Pass, according to the research;
  that is where Gunmetal differs, not in the player's looks.
- **Finamp** already says plainly when the server is transcoding; the
  quality badge (MUS-099) follows that example.
- **Apple Music** in iOS 27 has a landscape player with lyrics beside the
  artwork, according to the research, and the landscape player (MUS-112,
  CLI-054) is parity with it.
- **Jellyfin's Android TV app** keeps a left navigation, and Plex is
  restoring one after its 2025 redesign was rejected. The TV rail is parity;
  the layout contract and focus tests are the edge.
- **Navidrome** models music better than Plex and Jellyfin (multiple
  artists, release types, editions), but relies on other people's clients.
  The artist and album pages here have to be at least as good as what the
  best Subsonic apps show for Navidrome's data.

## Open questions for the interface

These are design questions this inventory could not settle from the feature
map. None changes the scope of a feature.

1. **Layout breakpoints.** This document adopts the design language's four
   width classes, but neither document has tested them on real tablets.
   *Recommendation:* test them at the 360 to 430 px range CLI-149 names, at
   320 px for reflow, and at common tablet widths in both orientations, then
   pin the class boundaries under the layout contract.
2. **Live TV on the phone.** A fourth tab only when the module is on, or an
   entry inside Home. *Recommendation:* the fourth tab, so no existing tab
   moves and people who never enable live TV never see it.
3. **The combined Home.** Whether "All" is offered by default or only as a
   setting. *Recommendation:* a setting, following discovery open decision
   10.
4. **Now playing on TV.** No feature row defines how music playing in the
   background is shown on TV, or whether TV Now Playing shows lyrics.
   *Recommendation:* add a clients row for the rail's now-playing entry,
   fixed as the rail's first entry whether or not anything plays, and a
   music row for TV lyrics, both R2.
5. **The pre-play sheet.** Whether it opens on every play or only when a
   choice or a warning exists. *Recommendation:* only when needed, with a
   long-press or menu route to it at any time, so one press still plays.
6. **The profile picker on the web.** ACC-019 puts it at launch; on a
   personal browser that is a wasted step. *Recommendation:* show it at
   launch only on devices marked as shared or household devices (ACC-021),
   and in the account menu everywhere.
7. **Admin on TV.** *Settled by the security baseline:* a TV is a limited
   device, so it shows no admin surface at all, and the server refuses
   admin routes from it (SEC-CLI-024). An admin's profile on a TV shows
   security alerts as a banner (ADM-114), and every admin task is a QR
   hand-off to a phone or computer, as proposed above. The earlier
   recommendation of the dashboard, sessions and activity in full on TV is
   withdrawn.
8. **Command palette versus search.** DIS-090 and DIS-083 overlap.
   *Recommendation:* one index behind both, with the palette adding commands
   and the search adding browse; never two different result sets for the
   same text.
9. **Linux desktop.** The mini player, media keys and global hotkeys
   arrive with the desktop shell, which is Later on every desktop
   (SEC-TM-074; owner to confirm). Until it ships, Windows, macOS and Linux
   users all get the installable web app's surfaces, and the Linux question
   (feature map open decision 21) waits for the shell.
10. **Where admin screens live in the browser.** The baseline's network
    file recommends serving the admin UI on a separate origin under the
    same certificate, so a script injected into the media pages cannot
    reach the admin cookie (network decision 6, part of security README
    decision 25). *Recommendation:* design the admin surfaces so they can
    run in that admin mode of the same bundle, and settle it with the owner
    before the R1 admin screens are built.

## Changes made to follow the security baseline

On 2026-10-02 this inventory was changed to follow the
[security baseline](../security/README.md). The numbered owner decisions
are those in the baseline's
[open decisions](../security/README.md#open-decisions-for-the-owner); the
changes that rest on one are applied as the baseline recommends and need
the owner to confirm them.

**Surfaces added.** SUR-108 Security log, SUR-109 Plain-HTTP and
home-posture help pages, SUR-130 Sessions and devices, SUR-131 Security
events, SUR-132 What admins can see, and privacy choices, and SUR-133
Recovery, all R1. Every entry that the baseline shapes now has a
**Security** line.

| Where | Was | Now | Requirements | Owner to confirm |
|---|---|---|---|---|
| SUR-070, SUR-078, SUR-082, SUR-008 | Passwords with authenticator-app codes where passkeys could not work; re-authentication by password | Passkeys, the household's identity provider, and approval from a signed-in device; no password anywhere | SEC-IAM-025, SEC-IAM-108 | Yes, decision 1 |
| SUR-070, SUR-082, SUR-109 | Over plain HTTP on a LAN address, password sign-in and an online-only player | Over plain HTTP, every peer but loopback gets the help page (new SUR-109) | SEC-NET-001, SEC-NET-024 | No |
| SUR-070, SUR-103 | The sign-in page shows the server's name and message | A generic sign-in page; the name appears after sign-in, on invitations and as a remembered name | SEC-NET-047, SEC-API-005 | Yes, decision 25 (web decision 5) |
| SUR-070, SUR-077 | "Remember this browser" | "Is this your own device?", with memory-only storage and 30 idle minutes on shared browsers | SEC-CLI-010, SEC-IAM-041 | Yes, decision 25 (the default answer) |
| SUR-061 | Browser approval from a signed-in device in R2 | R1, with the unverified-name approval sheet and the typed and matching code off the local network | SEC-IAM-056 to SEC-IAM-060, SEC-IAM-108 | No |
| SUR-003 | The notice centre in R2; new-device alerts in R2 | Both R1, with "This wasn't me" and the unmutable critical alerts | SEC-IAM-098, SEC-OPS-032 to SEC-OPS-034 | No |
| SUR-008 | On TV, approve a sensitive change from a phone | No sensitive change on TV at all; a QR hand-off instead; step-up defined | SEC-CLI-024, SEC-IAM-041 | No |
| Navigation; admin group; SUR-005, SUR-083, SUR-084, SUR-085, SUR-089, SUR-100, SUR-124; open question 7 | The admin dashboard, sessions and activity on TV; a TV edit sheet; the live TV wizard and lineup multi-select on TV | No admin surface on TV; alerts as a banner on an admin's profile; QR hand-off | SEC-CLI-024, SEC-IAM-049 | No |
| SUR-083, SUR-084, SUR-010, SUR-002 | Who is playing what | Who is playing, on which device and how; the title only if that person opts in; admin reads recorded in the person's own log | SEC-PRV-025, SEC-PRV-024, SEC-IAM-077 | Yes, decision 5 |
| SUR-104 | Play history and top users and titles across the server, with export | Totals only; no export; "popular on this server" is Later, and then only from opted-in people with a three-person threshold | SEC-PRV-025, SEC-TM-054 | Yes, decision 5 |
| SUR-091, SUR-132 | The owner chooses what admins see, up to full history | No server setting widens it; each person alone chooses to show titles | SEC-PRV-025, SEC-PRV-027 | Yes, decision 5 |
| SUR-078, SUR-132 | "What your admin can see" in R2 | R1 | SEC-IAM-104, SEC-PRV-027 | No |
| SUR-078, SUR-131 | Security events in R2; sign-in history only | Each person's own security events in R1, including admin reads of their data | SEC-IAM-097, SEC-IAM-077, SEC-OPS-027 | No |
| SUR-079 | A child's history visible to the household's adults | Only the child's designated guardians | SEC-PRV-029, SEC-PRV-022 | No |
| SUR-080 | Configuration errors, the snapshot path and the root error on the unauthenticated startup page | State and progress only; details on the console, in the log and in `doctor` | SEC-OPS-050, SEC-NET-047 | No |
| SUR-081 | Emergency page showing status and logs, with backup and restart | Nothing before an admin passkey sign-in; backup download owner-only with step-up | SEC-IAM-041, SEC-OPS-045 | No |
| SUR-082 | A TV could complete setup with a device key | A TV never claims; the owner claims from a phone or computer | SEC-CLI-024, SEC-IAM-059 | No |
| SUR-082, SUR-099 | The update check offered as a choice | A required question with two answers and none preselected | SEC-OPS-047 | Yes, decision 4 |
| SUR-090 | Hand over the server by adding an admin and removing yourself | A transfer the current owner and the recipient each confirm with a passkey | SEC-IAM-003 | No |
| SUR-090, SUR-071 | One-step invitation redemption; "help sign in" by link | Matching-code confirmation for members and multi-library invitations; a privacy notice first; recovery links redeemed in person, with the 72-hour hold | SEC-IAM-079, SEC-PRV-053, SEC-IAM-091, SEC-IAM-106 | Yes, decision 6 (recovery) |
| SUR-093 | Proxies, HTTPS and privacy settings | Posture and exposure status, proxies declared private or public, naming and certificates in R1, all owner step-up (the naming part is superseded by D-07: own-domain certificates in R1, the project name in R2; see the next section) | SEC-NET-019, SEC-NET-028, SEC-IAM-041, SEC-TM-017 | Yes, decision 2 (the name service) |
| SUR-094, SUR-078 | Scoped tokens in R1 | API keys in R2, never administrative | SEC-IAM-083, SEC-EXT-010 | Yes, decision 8 |
| SUR-095 | Third-party plugins installed by URL; developer mode as a screen | Plugins only through a signed index the owner added; developer mode only in the configuration file, with a banner | SEC-EXT-035, SEC-EXT-037, SEC-EXT-038 | Yes, decision 13 |
| SUR-097 | Rival-server imports in R2; the admin imports history for the household | Rival-database importers Later; each person imports their own history | SEC-TM-074, SEC-STD-031, SEC-PRV-025 | Yes, decision 5 |
| SUR-098 | Encryption in R2; a whole-server "full export" | Encrypted, signed backups in R1; the export holds the server's settings and the owner's own data only | SEC-PRV-039, SEC-OPS-042, SEC-PRV-025, SEC-PRV-047 | No |
| SUR-058, SUR-059, SUR-004 | Share links in R2 | R1 for music, with per-link limits; the public page shows nothing of the sharer or the server | SEC-API-097, SEC-PRV-031, SEC-NET-047 | Yes, decision 7 |
| SUR-075, SUR-050 | Choosing the SD card for downloads in R2 | App-private storage only; removable storage Later and encrypted | SEC-CLI-035, SEC-CLI-072 | Yes, decision 25 (client decision 3) |
| SUR-122 | The tuner-busy sheet shows who holds each tuner | Names a person or channel only for people who share what they play | SEC-PRV-022 | Yes, decision 5 |
| Releases, form factors, SUR-000, SUR-014, SUR-050, SUR-052, SUR-053, SUR-057, SUR-074, SUR-075, SUR-077, open question 9 | The desktop shell, its mini player, media panels, hotkeys, installers, downloads and exclusive output in R2 | Later, with the shell; desktops use the web client until then | SEC-TM-074, SEC-CLI-069 | Yes (the release scope; the feature map now agrees) |
| SUR-002 | Private session reached from the full player's menu only | Also from an Options button on the wide bar, Private session first, so it is two interactions from the bar | SEC-PRV-024 | No |
| SUR-003, SUR-103 | Stream limits only in R2 policies and the video limit card | R1 limits, server-wide and per guest, with a "too many streams" notice for music | SEC-TM-068, SEC-IAM-102, SEC-API-031 | Yes, decision 25 (the default values) |
| SUR-014 | Other Gunmetal players signed in as the same person; nothing about household devices in use | The profile's own players, plus household devices in use shown only as "In use"; other people's players never listed | SEC-PRV-022, SEC-API-016, SEC-API-068, SEC-HIS-014 | No |

The interface documents were then made consistent with each other. The
decisions, which touch SUR-002, SUR-010, SUR-011, SUR-014, SUR-020,
SUR-032, SUR-042, SUR-046 and the navigation model, are recorded in the
[interface README](README.md#decisions-that-made-the-four-documents-consistent).

## Changes made to follow the owner's answers

On 2026-10-03 this inventory was changed to follow the owner's answers of
2026-10-02 in the [decision register](../decisions.md#owner-answers-2026-10-02).
Every requirement a moved surface carries moves with it and keeps every
protection; nothing here weakens a security line. The surface IDs are
unchanged.

| Where | Was | Now | Decision |
|---|---|---|---|
| Releases, surface entries, the surface index and every Serves line | Release values R1, R2, R3, Later and No | R1.1, R1.2 and R1.3 added. Each Serves line lists a feature under the release in which it reaches the surface, so the R1 groups of 55 surfaces were split, and each Release line names what each point release adds | D-10 |
| SUR-101 | The owner's R1 alerts included a certificate the server did not ask for | That alert comes from the name service's Certificate Transparency monitoring, so it is R2 (SEC-NET-069); the expiry alerts stay R1 (SEC-NET-072) | D-07 |
| Releases, SUR-082, SUR-093, SUR-109 | The per-server HTTPS name as the R1 install-time default once its zone is on the Public Suffix List, with Certificate Transparency monitoring | R1 HTTPS through the owner's own domain with automatic certificates, a tailnet name or localhost; the name service, its naming client and its Certificate Transparency monitoring are R2, and ADM-023 is listed under R2 | D-07 |
| Releases, SUR-093, SUR-111 | Remote use in R1 through a declared proxy or tailnet, owner to confirm | Confirmed: the owner's reverse proxy or a tailnet in R1, with the shipped recipes (SEC-NET-022); built-in remote access, relays and the browser edge in R2 (ACC-102 added to SUR-093's R2 line) | Remote access answer |
| SUR-005, SUR-013, SUR-030, SUR-057, SUR-075, SUR-079, SUR-097 | R1 | R1.1, with the row that owns each surface | D-10 |
| SUR-021, SUR-058, SUR-059, SUR-081, SUR-084, SUR-088 | R1 | R1.2, with the row that owns each surface; in R1 an administrator ends a person's sessions from Users (SUR-090), which keeps SEC-IAM-044 in R1 | D-10 |
| SUR-027, SUR-087, SUR-089 | R1 | R1.3, with the rule language, the review queue and manual curation | D-10 |
| SUR-132 | The R1 page described the admins' live view and the title switch | In R1 admins have no live view, and the page says so; the live view and the title switch (ACC-116, MUS-235) arrive in R1.2 | D-10 |
| SUR-133 | No feature row yet | Serves ACC-137, ACC-138 and ADM-143, which the feature map now has; SUR-082 also serves ADM-143 | Consistency (flows G18) |
| SUR-004, SUR-013, SUR-132 | LIB-111, LIB-098 and MUS-235 had no home in their release | "Look up on MusicBrainz" in the album menu (R1.1), "Why is this here?" in the track info sheet (R1.1) and the title switch (R1.2) | D-10 |
| SUR-090 | ACC-076 listed under R2 | R1 for the device cap, allow-lists from R2, as the feature map places it | Consistency (A-557) |
| SUR-022, SUR-025, SUR-026 | INT-138 and LIB-056 under R1 | R2, where the R1 scope moves them | D-10 |
| Surface index | SUR-003 and SUR-103 counted 26 and 9 rows | 27 and 10, counting ACC-075, which was added to both | Consistency |
| Coverage | Counts from the first draft; INT-006 among the R1 rows | Counts recomputed per release; INT-006 dropped from the table because it moved to R2; a table for the R1 rows whose detail surface moved to a point release; the rows added by the security alignment without a Serves line listed | D-10 |
