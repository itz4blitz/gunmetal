# 25. Media addresses are a closed parameter, and the style is an official extension

Date: 2026-10-08
Status: accepted, through the owner's request on 2026-10-08 that artists,
albums, songs, movies and shows be real addresses, and that UUID versus
unique-name slugs be an official plugin. It has no entry in the
[decision register](../decisions.md) yet; the owner adds one if he wants it
tracked there.

## Context

The shell kept an album in history state on `/library`. A refresh, a pin and
a shared link could not name the item. SEC-CLI-025 requires one pure parse
into a closed set of typed routes. Unrecognised input is not a route. Opening
a page is navigation, not a state change, so it does not need a confirmation
screen.

CLI-034 says a link carries a random object id. The owner also asked for
unique-name slugs, and for that choice to live with the other official
extensions rather than as a hard-coded page list. Record 22 forbids the
client from fetching or running a plugin. Record 24 names
`PremierStudio/gunmetal-extensions` as the review home. This build still does
not download it.

## Decisions

1. **The closed media routes are** `/music/artists/:key`, `/music/albums/:key`,
   `/music/tracks/:key`, `/watch/movies/:key` and `/watch/shows/:key`. One
   segment, lowercase, no query and no fragment. Anything else is not a route.
2. **Both official styles parse.** A slug (`harbour-lights`) and an id
   (`demo-album-01`) open the same item. The address bar shows the canonical
   style. A collision gets a short id suffix so two items do not share a path.
3. **`url-style` is an official extension record**, status on, slot
   `route-style`. `addressStyle()` is the parameter this build reads. It is
   `slug`. Changing it is a data change in that record, not a downloaded
   package. Ids remain valid inbound links (CLI-034).
4. **Movies and shows have addresses before they have a library.** The page
   says they are not connected yet. It does not pretend to play them.
5. **This does not run plugin code.** The player ships the reviewed records.
   The GitHub repository is where a change is reviewed. A merge there does
   not install itself (record 24).

## Consequences

Opening an album or artist pushes a real path. A pin of that page stores the
path. An older pin that stored an item id on `/library` still opens it. A
reload of an id address replaces the bar with the slug. Playback is unchanged:
a link does not start audio.
