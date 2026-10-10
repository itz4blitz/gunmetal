# 29. An LG television is a later app, plus a server control

Date: 2026-10-10
Status: accepted, through the owner's request on 2026-10-10 that the
television decision be recorded and that no LG work start in the current
release. It has no entry in the [decision register](../decisions.md) yet;
the owner adds one if he wants it tracked there.

## Context

The owner bought a 2026 LG OLED C6 (webOS 26). The question was whether
the plugin host is complete enough to add an LG plugin, and whether
Gunmetal should also be an app on the set.

[Record 22](0022-client-plugins-are-declarative-data.md) makes a client
extension plain data. [Record 23](0023-web-client-is-the-player.md) makes
the browser the player you open. [Record 27](0027-extensions-review-on-personal-github.md)
leaves SEC-EXT-018 in force: this build does not load or run a plugin.
The only plugin worlds the code accepts are `scrobbler@1` and
`music-metadata-provider@1`. A guest is specified with no sockets and no
inbound requests (INT-054, SEC-EXT-031).

CLI-010 and VID-179 already schedule Samsung and LG as Later, after the
video milestone: a packaged web build, the television's own video
element, and the server's remuxer. The R2 television column is Android
TV, Google TV, and Fire OS. No work-package title names LG or webOS.

The set's own documents split the job in two. An app in the LG store is
how a person browses and watches. A separate local control, pairing once
and then speaking the television's second-screen socket, is how another
machine turns the panel on, changes the input, and opens that app. The
phone-branded ThinQ cloud API is an appliance interface, and the profile
list that was read does not include a television. Developer Mode can
hold a test package for one session; LG staff say that package is
removed when Developer Mode ends.

Published playback for webOS 26 is the television's own player. The
useful ceiling is HEVC at 2160p60, Dolby Digital Plus, and WebVTT.
Dolby Vision that LG staff have described for the built-in player is
profiles 5, 6, 8, 9, and 10 in MP4 or MPEG-TS. TrueHD and Dolby Vision
profile 7 are outside those lists. The Magic Remote that ships with the
C6 has a pointer and five-way keys, and no play, pause, or seek keys.

## Decisions

1. **Do not start an LG app or an LG plugin in the current release.**
   CLI-010 and VID-179 stay Later. The web player and the remux worker
   come first.
2. **The television, when it is built, is two parts.** The app is the
   web client packaged for the maker's store (CLI-010). The control that
   turns the set on, changes input, and opens the app is first-party
   server code beside the later device handoff (CLI-101, CLI-102). It
   is not a plugin world, and it is not added to WP-225.
3. **The app does not become the system player.** It is a store package.
   It reports what the panel can play, and the server remuxes to that.
   It does not add codecs the television's player omits.

## Consequences

No LG crate, package, or plugin world is added by this record. A future
server control has to be allowed to open a home-network connection of
its own; the plugin guest contract does not allow that. A future app
has to pass the television maker's store, and a Developer Mode install
does not count as shipping it.
