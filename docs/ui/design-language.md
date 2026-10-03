# Design language

This document defines how Gunmetal looks, moves and reads on every screen:
the principles, the colour tokens for each theme, how album artwork colours
the interface, typography, spacing and density per form factor, focus on
keyboards and TV remotes, motion, iconography, the empty, loading and error
states, and the accessibility requirements every surface must meet. It was
written on 2026-10-02 and is a proposal for the project owner to accept.

It sits under the two accepted architecture records. Record 2 (decision 7)
says the interface is dark-first and music-forward, with a persistent
now-playing bar and queue and artwork-led browsing, and that it takes cues
from the best streaming apps without copying another product's assets or
trade dress. The feature map in `docs/features/` is the source of truth for
what the interface must do; features are named here by their IDs. The
layouts of individual screens belong to the surface files in `docs/ui/`;
this file supplies the tokens and rules those layouts are built from.

The security baseline in `docs/security/` sits above the feature map. Where
a design choice here would break a security requirement, this file follows
the requirement and cites its ID (SEC-*AREA*-*NNN*). The concrete rules
the content security policy imposes on the design are in section 1; how
untrusted text is drawn is in section 6; the security prompts, stops and
notices are designed in section 11. Changes that touch one of the owner's
open security decisions are marked "owner to confirm" and listed in
section 14.

Releases are used as defined in the feature map README: **R1** (music: the
server and the web client, including the installable web app), **R2**
(video, and the native Android and Android TV clients), **R3** (live TV),
**Later** (wanted, not scheduled, including the desktop shell, which the
security baseline's release scope places there, SEC-TM-074) and **No**
(deliberately not doing).

Contrast ratios in this file were computed with the WCAG 2.2 relative
luminance formula, compositing in sRGB as browsers do. They are the
starting values for the colour check that CLI-141 puts in CI; the test must
recompute them from the token file rather than trust this table.

## Contents

1. Where the language comes from
2. What we learn from Spotify, Apple Music and the best TV interfaces
3. Design principles
4. Colour
5. How artwork drives colour
6. Typography
7. Shape, spacing and density
8. Focus: keyboard, touch and TV remote
9. Motion and reduced motion
10. Iconography and badges
11. Empty, loading, error and security states
12. Accessibility requirements
13. What each release needs from this document
14. Open questions for the project owner
15. Sources

## 1. Where the language comes from

### The brand that already exists

The README banner (`docs/banner.svg`) and the icon (`docs/icon.svg`) set the
identity. The mark is a hexagonal steel nut, lit from the top left, with a
brass play triangle sitting in its bore. The wordmark is set in Inter at
weight 800 with wide letter spacing and a polished-metal gradient. The
colours are:

| Element | Values in the banner | What the UI takes from it |
|---|---|---|
| Background | `#101418` to `#1c232a` to `#28313a` | The dark canvas and surface steps |
| Frame line | `#36404a` | Decorative dividers |
| Nut face | `#55626f`, `#2e3842`, `#151a1f` | The steel greys of controls and borders |
| Machined edge | `#e8f1f8` fading to transparent | A faint top highlight on raised surfaces |
| Wordmark | `#e4eaef` to `#5d6b78` | Primary text is a cool near-white, never pure white |
| Tagline | `#8f9dab` | Muted text sits at almost exactly this value |
| Brass | `#f3cd77`, `#d4952f`, `#94601d` | The one accent; `#d4952f` is the accent fill and `#f3cd77` the focus ring |

The palette below stays inside this family. The canvas `#0f1317` is the
banner's darkest stop to within 1.01:1, and muted text `#8f9ba7` is the
tagline grey to within a few points.

### What the feature map asks of the interface

The rows that set rules for the whole interface, rather than for one
screen, are:

- **Stability:** a persistent now-playing bar (MUS-108) and a layout
  contract that keeps the bar, queue, lyrics, device picker and scrubber in
  place across releases (MUS-113, CLI-031, DIS-015), all R1.
- **Honesty:** a quality badge that tells the truth (MUS-099), an honest
  unplayable state (MUS-229), the gain applied (MUS-090), a reason on every
  suggestion (DIS-062), all R1.
- **Speed:** instant browsing from the synced library with no spinner
  (CLI-022, MUS-208, DIS-002, DIS-100) and published budgets (DIS-019).
- **Accessibility:** screen readers reach every control (CLI-135), a build
  gate (CLI-136), full keyboard use with visible focus (CLI-138), text that
  follows the system size (CLI-139), reduced motion (CLI-140), dark, light,
  high-contrast and OLED-black themes (CLI-141), large targets and no
  gesture-only actions (CLI-142), all R1.
- **Form factors:** a phone-width web layout designed first (CLI-149) and a
  wide three-pane layout (CLI-060) in R1; tablet (CLI-053), TV rail, focus
  and performance (CLI-035 to CLI-038) and TV text size (CLI-043) in R2.
- **Artwork:** colours computed at scan and synced so the player draws at
  once (MUS-110); images in fixed sizes with a tiny placeholder (LIB-142);
  quality badges drawn by clients from data, never into posters (LIB-146).

### Security rules that shape the visual design

Several security requirements decide design details, so they are stated
here rather than discovered during the build.

**The content security policy, directive by directive.** Every HTML
response carries the policy of SEC-API-044. Each directive becomes a
design rule:

| Directive | What it allows | What it means for the design |
|---|---|---|
| `font-src 'self'` | Font files from the server's own origin | Inter ships as a WOFF2 file inside the signed server release and is served from the build-time asset manifest (SEC-CLI-012, SEC-STD-017). No web-font service, no stylesheet or `@import` from another origin, and no font as a `data:` URI. Native apps bundle the same file in the app package and use no downloadable-font provider, which would contact a third party (SEC-CLI-027). |
| `style-src 'self'` | Stylesheets served from the bundle | Tokens live in static stylesheets in the bundle. There is no `<style>` element with text in it, no `style="..."` attribute in markup and no `setAttribute('style', ...)`. Values that change at run time (the artwork tint, a progress position, a drag offset) are written as CSS custom properties or single properties through the CSSOM, or chosen by class. Whether React Native for Web's run-time style injection fits this policy is unverified (open question 8). |
| `img-src 'self' blob:` | Images from the server, and object URLs the client made | Artwork comes only through the server's capability URLs (SEC-API-026), as server-made JPEG, PNG or WebP derivatives in a fixed set of sizes (SEC-CLI-005, SEC-MED-046, SEC-MED-047), or as `blob:` URLs the client made from those bytes. Placeholders are gradients, not images. |
| No directive allows `data:` | Nothing | No `data:` image, font, icon or CSS background anywhere (SEC-API-044, SEC-PRV-018). Icons are SVG elements compiled into the bundle (section 10). |
| `script-src 'self' 'wasm-unsafe-eval'` | Script files from the bundle and WebAssembly compilation | No inline script and no inline event handler, including on the server-rendered help, startup and emergency pages (section 11), which work with no script at all or with script files from the bundle (SEC-HIS-028). No script from any other origin (SEC-SUP-037). |
| `connect-src 'self'` and `media-src 'self' blob:` | The server's own origin | Nothing on any screen is fetched from another origin: no CDN, no avatar service, no map tiles, no analytics (SEC-API-049, SEC-CLI-027). |
| `frame-ancestors 'none'` | No framing | Gunmetal is never embedded in another page, so no design depends on it. |

The end-to-end test that visits every screen in Chromium, Firefox and
WebKit with the production build fails on any policy violation
(SEC-API-044), so every component in this file is proven under the real
policy rather than assumed to fit it.

The other rules:

- Fonts must carry a licence on the project allowlist; OFL-1.1 is allowed
  for font files only (SEC-SUP-029).
- Artwork is decoded in the scan worker, a separate process, never in the
  server process (SEC-MED-018), with size limits checked first (LIB-143,
  SEC-API-086). Only JPEG, PNG, WebP and the first frame of a GIF are
  decoded (SEC-MED-044), SVG is never accepted as artwork (SEC-TM-035), and
  clients receive only re-encoded derivatives with EXIF, XMP, ICC and text
  chunks removed (SEC-MED-046). Colour profiles are therefore ignored in
  R1, so wide-gamut covers will look slightly duller (media and parser
  safety, question 13). The earlier draft said "in memory-safe code or the
  sandbox", which would have allowed decoding inside the server process.
- Uploaded profile pictures are re-encoded to a raster format with their
  metadata removed, and SVG is refused (ACC-011, SEC-API-086,
  SEC-PRV-006).
- No string from a file, a provider or another person ever reaches a style
  value, a class name, an element ID or a URL the client builds. Colour
  comes from validated numbers, and placeholder hues from a hash of an
  item's ID (sections 5 and 6).

## 2. What we learn from Spotify, Apple Music and the best TV interfaces

The owner asked for an interface inspired by Spotify but better. The
research in `docs/research/music-ux.md` and
`docs/research/discovery-home-and-search.md` shows what each reference does
well, what its users dislike, and therefore what Gunmetal does instead. Vote
counts are as those files recorded them.

### Spotify

| What it does well | What users dislike | What Gunmetal does |
|---|---|---|
| A stable, quiet now-playing bar on every screen | The heart became a plus in 2023; "Bring back the heart button!" has 5,769 votes | Love stays a heart, one tap from the bar (MUS-109) |
| The same context menu everywhere, by right-click or three dots | Long-press preview was removed; restoring it has 4,701 votes | One action registry for every surface (DIS-111); preview returns in R2 (MUS-064) |
| The three-pane desktop layout that others copy | In March 2024 the queue was squeezed into a fixed side panel and lost durations and album names | The queue pane can grow to full height and keeps durations and albums (CLI-060, MUS-119) |
| "Next in queue" shown above "Next from" the source | "Queue to Next or Last" had 1,635 votes; queue control is a paid feature | Three labelled lanes and documented verbs, free (MUS-116 to MUS-118) |
| Few tabs, each with one clear job | Home cannot be customised: 15,697 votes, marked Not Right Now; podcasts fill it (8,815 votes to hide them) | Home is built by the user from saved rules (DIS-003, MUS-049); music and spoken word stay apart (LAT-010, DIS-017) |
| The player takes its colour from the artwork | No light theme: 7,031 votes | Light, OLED-black and high-contrast themes in R1 (CLI-141) |
| A 2023 TV redesign with recent items first and a dim mode | No alphabet scroll bar (884 votes); albums cannot be sorted by release year (1,738) | Alphabet jump (DIS-101) and real sorts (DIS-104) in R1 |
| A distinctive app icon | A May 2026 icon change was reverted within days after backlash | The default icon never changes without notice (CLI-154) |

### Apple Music

| What it does well | What users dislike | What Gunmetal does |
|---|---|---|
| Separate Play Next and Play Last verbs | Several "Play Next" picks play in reverse order | Play next keeps the order you chose (MUS-117) |
| Word-by-word lyrics, translation and pronunciation | Unrelated tracks start after an album ends | Word-timed lyrics in R1 (MUS-156); suggestions are a visible lane, off by default (MUS-129) |
| A landscape player with lyrics beside the art (iOS 27) | The iOS 26 Liquid Glass look was criticised by NN/g for readability and motion; a request to turn it off has 118 "me too" votes | Opaque surfaces with computed contrast; no translucency under text (section 4) |
| Lossless and hi-res badges | NN/g described the mini player's title as ticking along "like a stock-market ticker", and the bar jumps when scrolling | No marquee text anywhere; the bar never moves (section 9) |
| It follows the system's light or dark setting | The Mac app launches on the play key or when headphones connect (a tool to stop it reached 669 points on Hacker News) | Themes follow the system by default (CLI-141); the desktop app, when it exists, never launches itself (CLI-068; the desktop shell is Later, see section 13) |

### Other music apps

- **Tidal's 2026 redesign** removed the "Playing from" label, hid the
  scrubber behind a tap and shipped a mini player with no skip buttons.
  Each of those is now a layout-contract item: "Playing from" on every
  queue item (MUS-123), an always-visible scrubber (MUS-110) and skip in the
  bar (MUS-108).
- **YouTube Music's April 2026 redesign** made the full-screen queue
  reachable only by a double swipe. The Gunmetal queue is one action away on
  every form factor.
- **Plexamp's UltraBlur** gives the player a strong identity from the
  artwork alone, at no cost to the user. That is the right instinct.
  Gunmetal reaches the same goal differently, with a pre-computed tint
  rather than a live blur, so it costs nothing on a cheap TV stick
  (section 5).
- **Finamp** says plainly above the progress bar when the server is
  transcoding. Gunmetal makes this a first-class badge (MUS-099).
- **Symfonium** users publish themes that recreate Spotify and Apple Music,
  which shows how much familiar patterns matter; its users also ask for a
  search bar in settings, which shows the cost of too many options.

### The best TV interfaces

| Reference | What it does well | What went wrong or costs too much | What Gunmetal does |
|---|---|---|---|
| Plex | A left navigation rail and pinned libraries | The rail was removed in 2025 and restored in an August 2026 preview after months of complaints (rollback vote 504); the Fire TV redesign turned about two clicks into about six, lagged, and took 30 seconds to start local playback | A left rail fixed by the layout contract (CLI-035), libraries within two presses (CLI-037), and frame-time budgets as tests (CLI-038) |
| Netflix | Labels on tiles that say why something is shown; a profile gate at launch | Expanding focused tiles with autoplaying trailers are costly on weak hardware; it moved to a top bar in 2025 | Every suggestion carries a reason (DIS-062); previews stay off by default (DIS-082, Later); a left rail, not a top bar |
| Apple TV app (tvOS 26) | Portrait posters for density; profiles at wake; a live preview while restyling subtitles | Not covered as a problem in the research | Artwork shape per row (DIS-010), profile picker (CLI-050), live subtitle preview (VID-092), all R2 |
| Infuse | Context menus on every card; an alphabet scrollbar; pinned favourites | No hero section, which users ask for | Long-press menu everywhere (CLI-039); letter column (CLI-040) |
| Spotify TV | A visible queue and a dim mode | Before 2023 it favoured art over usability | TV Now Playing with the queue and an ambient mode (CLI-041) |
| Jellyfin and Wholphin | OLED-friendly screensaver; image type chosen per row | Android TV capability bugs open since 2019 and 2020 (HDR detection, passthrough) | Screensaver from the profile's own art (CLI-042); an honest capability report (CLI-047); focus-map tests (CLI-036) |

### Trade dress: what we never use

Familiar interaction patterns (a persistent bar, three panes, rows of
cards, a left rail) are common property. Their specific looks are not. The
research names the risk in detail, so these are firm rules:

- No green-and-black scheme, no round green play button and no Spotify
  product names (Connect, Jam, Canvas, Daylist). Gunmetal's names are the
  device picker, the "Up next", "From" and "Continue with" lanes, and loves.
- No translucent glass layers in the style of Liquid Glass.
- No numbered "Top 10" poster treatment and no copy of Netflix's hero card;
  DIS-016 and DIS-139 require Gunmetal's own design for those.
- No live blur of the artwork in the style of Plexamp's UltraBlur.
- No platform icon sets whose licences restrict use to one vendor's
  platforms, and no icons, artwork or illustrations taken from any rival.

Gunmetal's own signatures are steel surfaces with one brass accent, the
hexagonal nut shape, the anodised artwork tint, the machined top edge on
raised surfaces, and square artwork with a small radius.

## 3. Design principles

1. **The art brings the colour; the interface is steel and brass.**
   Album artwork is the only source of colour variety. Everything else is a
   neutral steel grey with a single brass accent, so a library of thousands
   of covers never fights the controls around it.
2. **Brass is scarce.** Brass marks only three things: the primary action
   (play), where you are (the playing item, progress, the selected section,
   switches that are on) and the original file (the quality badge). If
   everything were brass, nothing would be.
3. **Instant, or say why not.** Anything read from the synced library
   appears with no spinner (CLI-022). Anything that waits on the server says
   what it is waiting for, and how far along it is when that is known.
4. **Controls stay where you left them.** The bar, queue access, lyrics,
   scrubber, device picker and TV rail keep their positions across releases
   (MUS-113, CLI-031). A layout change ships first as an opt-in preview with
   a way back.
5. **Tell the truth on screen.** The interface shows what actually
   happened: "Original FLAC, 24-bit, 96 kHz, played directly" (MUS-099), the
   gain applied (MUS-090), why a track cannot play (MUS-229), why a
   suggestion appears (DIS-062). Anything the user did not choose is
   labelled.
6. **Calm by default.** Opaque surfaces, no scrolling tickers, no
   autoplaying previews, no bar that moves on scroll, no motion that is not
   explaining a change of place.
7. **One action, one meaning, everywhere.** The same verbs, icons and menu
   on web, phone, desktop and TV (DIS-111), so the product feels small.
8. **Cheap hardware sets the effects budget.** Visual effects must cost
   nothing at run time on the cheapest supported TV stick: pre-computed
   colours instead of blur, transforms and opacity instead of animated
   shadows, and frame-time budgets that are tests (DIS-019, CLI-038).
9. **Accessible by gate, not by backlog.** An inaccessible control fails the
   build (CLI-136). Every rule in this document is written so it can be
   tested.

## 4. Colour

### The model

There are three layers. **Steel** is the neutral greys of surfaces, text and
controls. **Brass** is the single accent. **Artwork** tints surfaces on the
pages and players that belong to one album, artist or playlist (section 5).
Status colours (danger, warning, success, information) exist only for
status and always come with an icon and words.

### Tokens

Every value below is a design token. Components use token names only, never
raw values, so a theme is a different token file and nothing else.

| Token | Dark (default) | Light | OLED black | High contrast | Use |
|---|---|---|---|---|---|
| `bg.canvas` | `#0f1317` | `#f3f5f7` | `#000000` | `#000000` | App background |
| `bg.raised` | `#161c21` | `#ffffff` | `#0c1013` | `#000000` | Sidebar, cards, the bar |
| `bg.overlay` | `#1f262d` | `#ffffff` | `#161c21` | `#0f0f0f` | Menus, sheets, dialogs |
| `bg.inset` | `#0a0d10` | `#e6eaee` | `#000000` | `#000000` | Wells, input fields, the player's control area |
| `line.subtle` | `#262e36` | `#dde3e8` | `#1f262d` | `#8a8a8a` | Decorative dividers only |
| `line.control` | `#6b7783` | `#7a8693` | `#6b7783` | `#ffffff` | Input, toggle and checkbox edges |
| `text.primary` | `#e9eef2` | `#11161b` | `#e9eef2` | `#ffffff` | Titles, body, track names |
| `text.secondary` | `#b0bbc5` | `#3a4550` | `#b0bbc5` | `#f0f0f0` | Artist names, supporting lines |
| `text.muted` | `#8f9ba7` | `#56626e` | `#8f9ba7` | `#d6d6d6` | Durations, counts, metadata |
| `text.disabled` | `#5a6570` | `#9aa4ae` | `#5a6570` | `#9e9e9e` | Inactive controls only |
| `accent.fill` | `#d4952f` | `#b8781a` | `#d4952f` | `#ffc857` | The play button and other primary fills |
| `accent.on` | `#121518` | `#121518` | `#121518` | `#000000` | Text and glyphs on `accent.fill` |
| `accent.text` | `#e0ad55` | `#85550f` | `#e0ad55` | `#ffd37a` | Brass text: the playing track, links, the "Original" badge |
| `accent.indicator` | `#d4952f` | `#85550f` | `#d4952f` | `#ffc857` | Progress fill, selected-section bar, switch on |
| `progress.track` | `#2f3842` | `#e3e8ec` | `#2f3842` | `#5c5c5c` | Unfilled part of progress and sliders |
| `focus.ring` | `#f3cd77` | `#7a4c0c` | `#f3cd77` | `#ffe9a8` | Keyboard and TV focus ring |
| `focus.plate` | `#e9eef2` | `#11161b` | `#e9eef2` | `#ffffff` | Inverted fill for focused text controls on TV |
| `focus.onPlate` | `#0f1317` | `#f3f5f7` | `#000000` | `#000000` | Text on `focus.plate` |
| `status.danger` | `#ff7b74` | `#b8322a` | `#ff7b74` | `#ff8a80` | Errors, destructive actions |
| `status.warning` | `#ff9b55` | `#a14f12` | `#ff9b55` | `#ffb36b` | Warnings |
| `status.success` | `#5cc98f` | `#1d7348` | `#5cc98f` | `#7fe0a8` | Completed, healthy |
| `status.info` | `#7ab8ee` | `#1d5f9f` | `#7ab8ee` | `#9fd0ff` | Information, the offline banner |
| `edge.highlight` | `#e8f1f8` at 6% | none | `#e8f1f8` at 6% | none | One-pixel top line on raised surfaces |
| `scrim.text` | `bg.canvas` at 90% | `bg.canvas` at 90% | `bg.canvas` at 90% | `bg.canvas` at 100% | Behind any text that overlaps artwork or video |

Warning is deliberately orange rather than amber, so it does not read as
brass, and it never appears without its icon and a label. In the light
theme `bg.raised` and `bg.overlay` are both white; overlays are separated by
a shadow and a `line.subtle` border. In the high-contrast theme surfaces are
separated by white borders rather than shades.

### Interaction states

States are mixes over the surface they sit on, so they work on every theme
and on artwork-tinted surfaces.

| State | Rule | Worst case checked |
|---|---|---|
| Hover | `text.primary` at 6% over the surface | `text.muted` stays at 4.58:1 on hovered `bg.overlay` (dark) |
| Pressed | `text.primary` at 10% over the surface | As hover, for the instant it is shown |
| Selected | `accent.fill` at 14% over the surface, plus a 3 px `accent.indicator` bar on the leading edge | `text.primary` 13.01:1 and `text.muted` 5.36:1 (dark); 14.40:1 and 4.94:1 (light) |
| Disabled | `text.disabled`, no hover | Exempt under WCAG 1.4.3, but still 3.14:1 in dark |
| Unavailable item (CLI-026) | Artwork dimmed to 40%, an icon and a reason; text keeps its normal colour | Text contrast unchanged, because only the art dims |

### Measured contrast

The minimum across `bg.canvas`, `bg.raised` and `bg.overlay` for each pair:

| Pair | Dark | Light | OLED | High contrast | WCAG floor |
|---|---:|---:|---:|---:|---|
| `text.primary` on any surface | 13.09 | 16.64 | 14.71 | 19.17 | 4.5:1 |
| `text.secondary` on any surface | 7.83 | 8.95 | 8.80 | 16.82 | 4.5:1 |
| `text.muted` on any surface | 5.40 | 5.71 | 6.07 | 13.19 | 4.5:1 |
| `text.muted` on `bg.inset` | 6.88 | 5.16 | 7.41 | 14.45 | 4.5:1 |
| `accent.text` on any surface | 7.48 | 5.82 | 8.41 | 13.56 | 4.5:1 |
| `status.danger` as text | 6.07 | 5.45 | 6.82 | 8.40 | 4.5:1 |
| `status.warning` as text | 7.33 | 5.28 | 8.24 | 10.87 | 4.5:1 |
| `status.success` as text | 7.43 | 5.34 | 8.35 | 11.99 | 4.5:1 |
| `status.info` as text | 7.22 | 6.02 | 8.12 | 11.81 | 4.5:1 |
| `accent.on` on `accent.fill` | 7.10 | 5.01 | 7.10 | 13.65 | 4.5:1 |
| `accent.fill` against `bg.canvas` | 7.23 | 3.35 | 8.14 | 13.65 | 3:1 |
| `accent.indicator` against `progress.track` | 4.61 | 5.15 | 4.61 | 4.35 | 3:1 |
| `accent.indicator` against `bg.canvas` | 7.23 | 5.82 | 8.14 | 13.65 | 3:1 |
| `line.control` on any surface | 3.34 | 3.40 | 3.76 | 19.17 | 3:1 |
| `focus.ring` against any surface | 10.06 | 6.70 | 11.30 | 15.95 | 3:1 |
| `focus.onPlate` on `focus.plate` | 15.97 | 16.64 | 17.98 | 21.00 | 4.5:1 |

Every pair meets WCAG 2.2 Level AA. Primary and secondary text also meet the
Level AAA figure of 7:1 in every theme. Muted text meets AA only, so it is
never used for instructions or anything a person must read to complete a
task.

Two pairs are deliberately low and are handled by layout rather than by
colour. `focus.ring` against `accent.fill` is only 1.7:1 in dark, so the
ring is always drawn with a gap of surface colour between it and the
component (section 8). `text.primary` on `accent.fill` is only 2.21:1 in
dark, so text on brass is always `accent.on`.

### Choosing a theme

- The default is **System**: dark or light following the operating system,
  and dark when the system states no preference. Dark is the theme designed
  and reviewed first.
- Settings > Appearance offers System, Dark, Light, OLED black and High
  contrast (CLI-141). The choice is a synced user setting with an optional
  "this device only" override (CLI-030), because a phone and a living-room
  TV often want different themes.
- When the operating system asks for more contrast (`prefers-contrast: more`
  on the web, and the native equivalents in R2), the high-contrast theme is
  used. When Windows forced colours are active, the web client uses the
  system colour keywords and keeps every focus ring and border visible.
- TVs default to Dark regardless of the system. All themes work on TV; the
  light theme is not recommended there (open question 5).
- The kids profile type gets its own default layout and theme in R2
  (DIS-152). It must be built from the same tokens and pass the same checks.

## 5. How artwork drives colour

### The idea: anodised steel

On pages and players that belong to one album, artist or playlist, the
artwork's colour tints the steel, the way anodising colours a metal part.
The tint is dark and low in chroma in the dark theme and pale in the light
theme. It changes the surfaces only. Text, controls, focus, status and
brass never change colour with the artwork. In one sentence: the art tints
the metal; it never paints the controls.

This differs from the references on purpose. Spotify takes a colour from
the art, Apple animates artwork, and Plexamp blurs the image itself. A
low-chroma tint computed once on the server gives each album its own feel,
guarantees contrast, and needs no image work on the device, which matters
on a TV stick where a live blur would cost frame time.

### Where the colour is computed

1. At scan, the scan worker already decodes each cover once to make its
   fixed sizes (LIB-142), in memory-safe code in a process separate from
   the server (LIB-143, SEC-MED-018). The palette is taken from that same
   decode, so it adds no extra decode and no new attack surface. Colour
   profiles are ignored in R1.
2. The worker quantises the smallest size, ignores near-greys, and returns
   up to three candidates in OKLCH (a base, a vivid and a deep colour). The
   server treats the worker's numbers as untrusted, checks each against its
   range, and only then stores them in the album's metadata (SEC-MED-023).
   Artists and playlists take the colour of their most representative
   album (for a playlist, the first album shown in its automatic cover,
   MUS-137).
3. The candidates travel in the synced library (MUS-110), so the client
   draws the tinted page or player on the first frame with no network call.
   The client decodes them with the same bounded, schema-validating decoder
   as everything else a server sends and drops any value out of range
   (SEC-CLI-021, SEC-MED-077). Because the contrast check below runs on the
   client, a buggy or hostile server cannot make text unreadable; the worst
   it can do is leave a page untinted.

### How a tint becomes a surface

The client derives the surfaces from the base candidate, with fixed rules:

| Derived token | Dark theme | Light theme |
|---|---|---|
| `art.surface` | OKLCH lightness 0.26, chroma capped at 0.07, the art's hue | OKLCH lightness 0.95, chroma capped at 0.04, the art's hue |
| `art.wash` | `art.surface` at the top of a header, blending to `bg.canvas` over 60% of the header height | Same |
| Player background | `art.surface` behind the artwork, blending to `bg.inset` behind the controls | Same |

Out-of-gamut values are brought into sRGB by reducing chroma first. Then
the actual sRGB colour is checked against every text and control token
that will sit on it. If any pair fails, lightness steps by 0.01 toward the
canvas until it passes. Covers with almost no chroma (black-and-white
photography, for example) get no tint at all, which is the honest result.
The client writes the derived colours as CSS custom properties through the
CSSOM, never as a style attribute or a `<style>` element (SEC-API-044,
section 1).

### The guarantee

Sweeping the hue in 5-degree steps, at zero, half and full chroma cap,
gives these worst cases:

| Pair on the tinted surface | Dark, lightness 0.26 | Light, lightness 0.95 | WCAG floor |
|---|---:|---:|---|
| `text.primary` | 12.57 | 15.15 | 4.5:1 |
| `text.secondary` | 7.52 | 8.15 | 4.5:1 |
| `text.muted` | 5.18 | 5.19 | 4.5:1 |
| `accent.fill` (play button) | 5.69 | 3.05 | 3:1 |
| `accent.text` | 7.19 | 5.29 | 4.5:1 |
| `focus.ring` | 9.66 | 6.10 | 3:1 |
| `line.control` | 3.21 | 3.09 | 3:1 |

At a lightness of 0.30 instead of 0.26, muted text falls to 4.58:1 on teal
in the dark theme, and at 0.34 it fails on every hue (the best case is
4.32:1). That is why the band is fixed at 0.26. In the light theme the brass
fill has the least margin (3.05:1 against a 3:1 floor), so the play button
on a light tinted surface also carries a 1 px `accent.indicator` edge. The
CI test must repeat this sweep from the token file, in finer steps, rather
than rely on these numbers.

### Text over artwork

Text never sits directly on artwork or video. Where a design overlaps them
(a TV backdrop, DIS-164; a video player overlay in R2), `scrim.text` sits
behind the text. A canvas scrim at 90% keeps primary text at 12.22:1,
secondary at 7.31:1 and muted at 5.04:1 over a pure white image in the dark
theme, and muted text at 4.54:1 over pure black in the light theme. A scrim
at 75% would keep only primary text above 4.5:1, so 90% is the rule.

### Fallbacks and switches

- **No artwork.** The tile shows a steel placeholder: a flat `bg.raised`
  plate with the hexagon outline and the album's initials, tinted with a hue
  derived from a hash of the album's ID at chroma 0.03, so a shelf of
  untagged albums is not one grey block. The initials come from the title,
  so they are drawn as a text node like any other untrusted string
  (section 6). No noise textures; the banner's brushed grain is for the
  brand only.
- **Artwork still loading.** The tiny placeholder from LIB-142 is decoded by
  the core within its limits (SEC-MED-077) into a few colour stops and
  drawn as a gradient, not as a `data:` image, which the content security
  policy forbids (SEC-API-044). The image then fades in over 200 ms, or
  appears at once with reduced motion. Missing art on small devices
  (CLI-023, R2) keeps the placeholder; it never shows a spinner.
- **The now-playing bar stays steel.** The bar is the stable anchor of the
  layout contract, so it does not change colour with every track (open
  question 4). The full-screen player, the album, artist and playlist pages
  and TV Now Playing are tinted.
- **A switch to turn it off.** Settings > Appearance > "Colour from
  artwork" (on by default). Off gives plain steel everywhere.
- **High contrast.** No tint at all.
- **Track changes.** The player crossfades from one tint to the next over
  400 ms; with reduced motion the change is instant.

## 6. Typography

### Typeface

- **Inter**, bundled with the client and served by the Gunmetal server. It
  is the typeface of the wordmark, it is released under the SIL Open Font
  License 1.1, which SEC-SUP-029 allows for font files, and it is a variable
  font with tabular figures (rsms/inter repository). The web client ships it
  as a WOFF2 variable file limited to weights 400 to 700, embedded in the
  signed server release and served from the asset manifest under
  `font-src 'self'` (SEC-CLI-012, SEC-STD-017, SEC-API-044); its size must
  be measured against the web client's load budget (DIS-019). Native apps
  bundle the same file and never fetch fonts at run time (SEC-CLI-027).
- **Fallback:** the platform's system UI font for any script Inter does not
  cover. Which scripts Inter covers is unverified; Chinese, Japanese,
  Korean, Arabic, Hebrew and the Indic scripts are assumed to need the
  system font. Bundling a large CJK font is not planned.
- **Monospace** for file paths, codec strings and log lines (the track info
  sheet MUS-114, admin logs): the platform's monospace font, not a bundled
  one. File paths appear only on admin screens (SEC-API-068).
- **Subtitle fonts (R2).** Styled subtitles are drawn with a bundled,
  pinned subtitle font set. Fonts embedded in a video file are used only
  for a library whose owner opted in, after the server's parse worker has
  parsed and rewritten them (SEC-MED-054; owner to confirm, open
  question 9).
- **The wordmark** (weight 800, wide tracking, metal gradient) appears only
  in the logo. Interface text never uses gradients, outlines or weight 800.

### Scale

Sizes are in CSS pixels on the web and density-independent pixels or scaled
pixels in the native apps, so system text settings scale them (CLI-139).
Line heights follow each size.

| Style | Phone (web in R1, native in R2) | Wide web, tablet and desktop | TV, 1920 by 1080 canvas (R2) | Weight | Use |
|---|---|---|---|---|---|
| `type.display` | 32/38 | 44/52 | 56/64 | 700 | Artist, album and playlist names in page headers |
| `type.title1` | 24/30 | 30/38 | 44/52 | 700 | Page titles |
| `type.title2` | 20/26 | 22/28 | 34/42 | 600 | Section and row headings |
| `type.title3` | 17/22 | 16/22 | 28/34 | 600 | Card titles, sheet titles, the bar's track title |
| `type.body` | 16/24 | 15/22 | 26/34 | 400 | Paragraphs, list rows |
| `type.bodyStrong` | 16/24 | 15/22 | 26/34 | 500 | Track titles in rows |
| `type.small` | 14/20 | 13/18 | 24/30 | 400 | Artist lines, durations, metadata |
| `type.label` | 12/16 | 12/16 | 24/28 | 600 | Badges, section labels; small capitals with 0.06 em tracking |

The TV column is a starting point for a sofa-distance test on the reference
devices, not a measured result. No TV text is smaller than 24 px on the
1920 by 1080 canvas. Platform guidance figures for TV text sizes were not
checked for this document (unverified). The TV text-size setting (CLI-043)
scales every style in steps of 100%, 115% and 130%, and each step is
tested.

### Rules

- **Figures.** Durations, track numbers, counts, times and technical values
  use tabular figures so columns line up. Running text uses proportional
  figures.
- **Truncation, never marquee.** Titles truncate with an ellipsis: one line
  in rows, two lines under tiles. The full text is always in the accessible
  name, in a tooltip on pointer devices, and on TV the focused tile's title
  may wrap to three lines. Nothing scrolls sideways on its own.
- **Line length.** Paragraphs (help text, and biographies once a metadata
  plugin exists in R2) stop at about 70 characters per line.
- **Case.** Sentence case for everything, including buttons and headings.
  Small capitals appear only in `type.label`, echoing the wordmark without
  copying it.
- **Text spacing.** Every layout must survive the WCAG 1.4.12 overrides
  (line height 1.5 times the font size, paragraph spacing 2 times, letter
  spacing 0.12 times, word spacing 0.16 times) without clipping or overlap.
- **Language and direction.** All strings are external (CLI-146). R1 uses
  logical properties (start and end, not left and right) so mirroring for
  right-to-left languages in R2 (CLI-147) needs no layout rewrite. Icons
  that imply direction (back, next, the queue's drag handle) mirror; media
  transport icons (play, skip) do not.

### Untrusted text

Most of the words on a Gunmetal screen were written by someone else: a
tagger, a metadata provider, another member, a device naming itself. The
type system draws all of it the same way.

- **Text nodes only.** Titles, artists, album and playlist names, tags,
  lyrics, chapter and track names, profile and device names, an
  administrator's message and error text are drawn as plain text, never
  parsed as HTML or Markdown, in every component and on every platform
  (SEC-API-046, SEC-CLI-001, SEC-TM-036, SEC-HIS-027, SEC-MED-077). The
  build fails on any HTML-string sink (SEC-API-045).
- **Inside a bidirectional isolate.** Each untrusted string sits in its own
  isolate (`dir="auto"` on the web, the platform equivalent natively), so a
  right-to-left title displays correctly and cannot reorder the words
  around it (SEC-MED-057). Bidirectional override and control characters
  are already removed from single-line fields on ingest (SEC-API-048).
- **Multi-line text** (lyrics, and biographies once a provider supplies
  them in R2) keeps its line breaks through pre-wrapped text, never
  inserted line-break elements. Rich text arrives as the server's
  validated structure of paragraphs, emphasis and links, drawn by
  components (SEC-API-046).
- **Truncation is visual only.** The ellipsis rule above never cuts the
  string itself: the accessible name and the tooltip carry the full
  normalised text, and tooltips are text too.
- **Names that claim an identity** (device names on approval screens, user
  names, share recipients) are shown as claimed, next to facts the server
  knows, such as the device type and "In this home" or "Somewhere else"
  (SEC-IAM-058, SEC-TM-036). A device name is never styled as if it were
  verified.
- **Links.** A URL from metadata becomes a link only when the core's
  validator parses it as `https` with a host (SEC-CLI-002, SEC-API-047,
  SEC-MED-058); anything else is plain text. A link to another origin opens
  only after a sheet that shows the destination host with Cancel focused,
  and on a TV it is shown as a QR code instead (SEC-STD-015). Links use
  `noopener` and `noreferrer`.
- **Never into styles or addresses.** No untrusted string becomes a style
  value, class name, element ID or URL (section 1).

## 7. Shape, spacing and density

### Spacing

A 4 px base grid. Tokens: `space.0` 0, `space.1` 2, `space.2` 4, `space.3`
8, `space.4` 12, `space.5` 16, `space.6` 20, `space.7` 24, `space.8` 32,
`space.9` 40, `space.10` 48, `space.11` 64, `space.12` 96. Components use
tokens only, so the TV text-size setting and density modes scale them
together.

### Shape

| Token | Radius | Used for |
|---|---|---|
| `radius.xs` | 2 | Badges |
| `radius.s` | 4 | Album and playlist artwork, tiles, selected row backgrounds |
| `radius.m` | 6 | Buttons, inputs, chips |
| `radius.l` | 10 | Menus, popovers, dialogs |
| `radius.xl` | 16 | Top corners of bottom sheets on phones |

Corners are small and machined rather than soft. Buttons and chips are not
pills. Two shapes carry the identity:

- **The nut.** A flat-topped hexagon with rounded corners, taken from the
  mark. It is used for the primary play and pause button (brass fill,
  `accent.on` triangle) and for people: artist photos and profile pictures
  (ACC-011). Records are squares and people are nuts, which separates them
  at a glance. Whether the hexagonal crop works for portraits needs a
  usability test (open question 2). The hit area of a nut button is always
  the full square around it.
- **The square.** Albums, tracks and playlists use square artwork with
  `radius.s`. Video uses 2:3 posters and 16:9 stills in R2.

### Elevation

- **Dark and OLED:** depth comes from surface steps (`bg.canvas`, then
  `bg.raised`, then `bg.overlay`) and the one-pixel `edge.highlight` on the
  top edge of raised surfaces, like the lit edge of the nut. Shadows are
  almost invisible on dark surfaces and are not used.
- **Light:** two shadows, a small one for raised surfaces and a larger,
  softer one for overlays.
- **TV:** no shadows and no blur of any kind; focus is shown by scale, ring
  and plate (section 8).

### Breakpoints and density

| Class | Width | Layout (owned by the surface files) |
|---|---|---|
| Compact | under 600 | Phone layout with bottom navigation, the bar above it, a full-screen player and a queue sheet (CLI-149) |
| Medium | 600 to 1023 | A navigation rail and content; the queue opens as a side sheet |
| Expanded | 1024 to 1439 | Sidebar and content; the queue pane overlays the content |
| Wide | 1440 and over | Three panes: library, content and a resizable queue that can grow to full height (CLI-060) |

CLI-149 designs the phone layout for 360 to 430 px. WCAG 1.4.10 requires
reflow at 320 CSS px, so every compact screen is also tested at 320.

| Measure | Phone | Wide web and desktop | Tablet (R2) | TV (R2) |
|---|---|---|---|---|
| Side margin | 16 | 24 in content | 24 | 96 horizontal, 54 vertical (the common 5% overscan-safe margin; platform guidance unverified) |
| Track row height | 56 | 48 comfortable, 36 compact (DIS-107) | 52 | 72 |
| Minimum target | 44 by 44 | 32 by 32 | 44 by 44 | Every focusable item at least 64 tall |
| Grid tile | 2 columns, 12 gap | 160 to 200 wide, 24 gap | 3 to 5 columns | About 220 square tiles, 24 gap, 7 to a row |
| Gap between tappable targets | 8 | 4 | 8 | 24 between cards |
| Now playing | Bar 64 high above a 56 navigation bar | Bar 80 high across the full width | Bar 72 | A Now Playing entry fixed at the top of the rail; no overlay bar |

WCAG 2.5.8 sets a Level AA minimum of 24 by 24 CSS px for pointer targets.
Gunmetal's touch minimum of 44 matches the stricter Level AAA criterion
2.5.5 (its 44 px figure was not re-checked for this document, unverified),
and the 32 px pointer minimum on wide screens is well above AA. CLI-142
checks target sizes in tests.

On TV, rows keep vertical room for the focus scale (6% of the card height)
plus the ring and its gap, so a focused card is never clipped by the row
above or below.

Car screens (Android Auto, CLI-116, R2) are drawn by the platform from a
browse tree. This language supplies only the artwork sizes and monochrome
icons there; the colours and layout are the platform's.

## 8. Focus: keyboard, touch and TV remote

### Keyboard focus on web and desktop

- Focus is shown with `:focus-visible` (or the native equivalent), so a
  mouse click does not draw a ring but every keyboard move does (CLI-138).
- The ring is 2 px of `focus.ring` drawn outside the component with a 2 px
  gap of surface colour, and its corners follow the component's radius plus
  the gap. That meets the WCAG 2.4.13 figure (a 2 px perimeter at 3:1 or
  more) at Level AAA, not only the AA rule that focus be visible.
- Inside scrolling lists, where an outside ring would be clipped, the ring is
  drawn 2 px inside the row on top of the selected background.
- Sticky elements (the bar, page headers) must never cover the focused item
  (WCAG 2.4.11). Scroll padding equals their height.
- Focus order follows reading order. Modal sheets and dialogs hold focus
  until closed and close with Escape; nothing else traps focus (WCAG 2.1.2).
- A "Skip to content" link and a "Skip to player" link are the first two
  focusable items on the web.

### Touch

No focus ring is drawn for touch, but the pressed state is always shown.
When a hardware keyboard or switch access is in use on a phone or tablet,
keyboard focus rules apply. Screen-reader focus uses the operating system's
own indicator.

### TV remote focus (R2)

The TV is where users judge a media server, and where Plex's 2025 to 2026
redesigns failed. The rules:

1. **Exactly one thing is focused, always.** Focus never disappears, never
   lands on something that cannot be acted on, and starts on the most likely
   action (the first card of the first row, or Play on a detail page).
2. **Focus is unmistakable, without colour.** Three treatments, by kind of
   element:
   - **Artwork cards:** the card scales to 106%, a 4 px `focus.ring` is
     drawn with a 4 px gap, and the title under the card changes from
     `text.secondary` to `text.primary`.
   - **Text controls** (rail items, buttons, list rows, menu items, guide
     cells in R3): the **plate**. The control fills with `focus.plate` and
     its text turns `focus.onPlate`, at 15.97:1 in the dark theme. An
     inverted plate reads from across a room and does not rely on colour
     vision.
   - **The brass play button:** keeps its fill and gains the ring with its
     gap, because a plate would hide the brass meaning.
3. **With reduced motion,** cards do not scale; the ring thickens to 6 px
   instead.
4. **Focus memory.** Back returns to the same row and the same card
   (CLI-036). Each row remembers its last focused card when focus leaves and
   returns.
5. **Rows scroll under a fixed focus column.** The focused card stays in the
   first fully visible column while the row moves beneath it, which keeps
   the eye still. This is a proposal to be tested against focus that moves
   until the edge (open question 3).
6. **The rail.** A left rail, collapsed to icons, one press of Left away from
   any top-level screen, expanding to show labels when focused (CLI-035).
   Its first entry is always Now Playing, present whether or not anything
   is playing, so playback is always one press away and no entry moves when
   playback starts or stops; with nothing queued it opens the empty-queue
   state. The rail's position and order are part of the layout contract.
7. **Back is predictable.** Back always goes back one step, to where the
   person came from: from a screen opened from another (a detail page, TV
   Now Playing, a player), to that screen with the same row and card
   focused (DIS-112, CLI-036); from the content of a top-level screen, to
   the rail; from the rail, to Home; and from Home it asks before leaving
   the app. Back never skips a step and never stops music; leaving the
   video player pauses the film and saves its place.
8. **The Play and Pause keys work on every screen** and control the current
   playback (CLI-045), including the screensaver and TV Now Playing's
   ambient mode, where they also wake the screen; every other key there
   only wakes it.
9. **Every card has the same long-press menu** as the phone and web
   (CLI-039, DIS-111). A one-time hint says "Hold OK for more".
10. **Grids have a letter column** at the trailing edge (CLI-040).
11. **Focus-map tests** drive the D-pad through every screen and assert where
    focus lands, and the frame time of a focus move is held to the DIS-019
    budget on the cheapest supported stick (CLI-036, CLI-038). Focus
    animations use only transform and opacity.

## 9. Motion and reduced motion

### Tokens

| Token | Duration | Easing | Use |
|---|---|---|---|
| `motion.instant` | 0 ms | none | The substitute for every other token under reduced motion |
| `motion.quick` | 100 ms | standard | Hover, press, toggles |
| `motion.focus` | 150 ms | standard | TV focus scale and ring |
| `motion.base` | 200 ms | enter or exit | Menus, sheets, artwork fade-in |
| `motion.slow` | 300 ms | enter or exit | Expanding the bar into the full-screen player, page transitions |
| `motion.tint` | 400 ms | linear | Artwork tint crossfade on track change |

Easing curves: standard `cubic-bezier(0.2, 0, 0, 1)`, enter
`cubic-bezier(0, 0, 0, 1)`, exit `cubic-bezier(0.3, 0, 1, 1)`.

### Rules

- **Motion explains a change of place.** A sheet comes from the control that
  opened it; the player grows out of the bar and shrinks back into it.
  Motion for decoration is not used.
- **Nothing moves on its own behind content.** No looping artwork, no
  marquee titles, no parallax, no autoplaying previews. The visualiser
  (MUS-083, Later) is opt-in.
- **The bar never moves.** It does not shrink, float, hide or merge with
  navigation when the page scrolls.
- **Lyrics.** Synced lyrics (MUS-155) scroll smoothly to keep the current
  line a third of the way down. Scrolling by hand pauses the follow and
  shows "Back to current line", which meets WCAG 2.2.2 for content that
  moves for more than five seconds. Word-by-word lyrics (MUS-156) highlight
  by colour and weight, never by scaling or sweeping gradients.
- **The playing indicator** (three bars beside the playing track) animates
  only while audio plays, and is static with reduced motion.
- **No flashing.** Nothing flashes more than three times a second (WCAG
  2.3.1). The visualiser, when it exists, must enforce this.
- **Progress is not animation.** The progress line and lyric highlighting
  continue under reduced motion, because they are the information.

### Reduced motion (CLI-140, R1)

Reduced motion follows the operating system (`prefers-reduced-motion` on
the web; the platform settings in R2), with an in-app override in Settings >
Appearance > Motion: System, Reduced or Full. When reduced:

- every motion token resolves to `motion.instant`, except that sheets and
  dialogs may fade over `motion.quick` so their appearance is not jarring;
- TV focus does not scale (the ring thickens, section 8);
- lyrics jump line by line;
- the artwork tint and artwork changes are instant;
- the playing indicator is static.

The reduced set is tested like any other behaviour (CLI-140), which also
meets the Level AAA criterion 2.3.3 for motion triggered by interaction.

## 10. Iconography and badges

### Icons

- **Grid and stroke.** A 24 px grid with 2 px padding, a 2 px stroke at
  24 px, and round caps and joins, matching the rounded joins of the mark.
  Sizes: 16 for inline badges, 20 for dense rows, 24 by default, 32 on TV,
  40 for TV player transport.
- **Outline means available, filled means on.** A heart outline is "Love"; a
  filled heart is "Loved". The state is also exposed to assistive
  technology (pressed or checked), so fill is never the only signal. The
  filled heart is `text.primary`, not brass, which keeps brass scarce.
- **Base set.** Lucide, whose icons are under the ISC licence, with a subset
  derived from Feather under MIT (lucide.dev). Both must pass the licence
  allowlist check (SEC-SUP-029). Gunmetal-specific glyphs are drawn on the
  same grid: the three queue lanes, a gapless join, "original", "converted",
  private session, the device picker, the health report and the nut play
  button.
- **Delivery.** Every icon is an SVG element compiled into the bundle from
  the project's own source. Icons are never `data:` URIs, CSS background
  images, icon fonts or sprites from another origin, and carry no `<style>`
  element or style attribute inside them (SEC-API-044, SEC-API-049). An SVG
  that came from a file or a person is never drawn at all (SEC-TM-035).
- **Labels.** Every icon-only button has an accessible name and a tooltip on
  pointer devices; on TV its label appears when it is focused. Navigation
  items always show their text label on phone and in the expanded rail.
- **Media kinds.** Distinct glyphs for music, films, TV shows, live TV (R3)
  and spoken word (Later), so mixed results are scannable and spoken word
  never looks like music (LAT-010).
- **The mark.** The full hexagonal nut with its brass play triangle appears
  only as the app icon, on the sign-in page and in About. Alternative icons
  (CLI-154, Later) are variants of the same mark.

### Badges

Badges are drawn by clients from synced data and can be switched off per
device; nothing is drawn into posters (LIB-146; LIB-147 is No). Every badge
is text, so it survives colour blindness and screen readers.

| Badge | Look | Text example | Feature |
|---|---|---|---|
| Original | `accent.text` on a 1 px `accent.text` outline | "Original" in compact form; the full sentence on hover, focus or tap: "Original FLAC, 24-bit, 96 kHz, played directly" | MUS-099 |
| Converted (R2) | `text.secondary` outline | "Converted" in compact form; the full sentence on hover, focus or tap: "Opus 160 kbps, converted for mobile data" | MUS-099, MUS-106 |
| Format and lossless or hi-res | `text.secondary` outline | "FLAC 24/96", "Lossless", "Hi-res" | MUS-021, LIB-146 |
| Explicit | `text.secondary` filled square, `bg.canvas` letter | "E", with the accessible name "Explicit" | MUS-047 |
| Suggested | `text.muted` label beside the reason | "Suggested: same composer" | MUS-129, DIS-062 |
| Unavailable | Icon plus `text.secondary` reason | "Can't play in this browser: ALAC" | MUS-229, CLI-026 |
| Drive offline | `status.info` icon plus label | "Drive offline" | LIB-032 |
| Private session | Icon plus "Private" in `text.primary` on `bg.overlay` | Shown in the bar and the full player for as long as it is on; it is never hidden by a layout or theme | ACC-117, SEC-PRV-024 |

The "Original" badge is the one place where brass means the file itself,
tying the badge to the brass triangle in the mark and the README's promise
to play the original. The compact quality badge ("Original" or
"Converted") appears in the now-playing bar on every layout, the phone
width included, and under the scrubber in the full player, because MUS-099
names the bar; the format badge ("FLAC 24/96") is a separate badge for
rows, tiles and the track info sheet.

## 11. Empty, loading, error and security states

### Rules for every state

1. **Say what happened, what still works and what to do,** in that order,
   in plain sentence-case words. No "Oops", no exclamation marks, no blame.
2. **Offer one action** that fixes or fills the state, when one exists.
3. **Typed errors have details, and no more.** Every error the server
   returns is a type from a closed catalogue with a request identifier
   (SEC-API-072). The client maps the type to the plain sentence, and a
   "Details" disclosure shows the error type and the request identifier,
   which the owner can find in the server log (CLI-033). Details never show
   a file path, byte offset, server address, version, dependency name,
   stack trace or the value the person submitted (SEC-API-072,
   SEC-TM-040). Where a file and location matter, as in library health,
   they appear only on admin screens built from admin response types
   (SEC-API-068). The earlier draft showed "the error type and location"
   to everyone.
4. **No spinner for local data.** Lists, pages and search read the synced
   library (CLI-022). Skeletons appear only during the very first sync on a
   device, with the sync progress from CLI-024.
5. **Waiting on the network is announced, late.** A spinner appears only
   after 400 ms, so fast operations do not flash. After 10 s the text says
   what is being waited for. Progress is determinate whenever the server
   reports it.
6. **Undo beats confirm.** Reversible actions (dismiss, hide, and from R2
   remove from queue) act at once and offer Undo (DIS-023, MUS-121). An
   undo notice stays for at least 10 s, stays while it is hovered or
   focused, can be reached by keyboard, and is announced politely to screen
   readers. The Hidden page is the permanent undo. Irreversible actions
   confirm first and name exactly what will happen. Queue undo arrives in
   R2 (MUS-121), so in R1 removing one queue row acts at once with no
   prompt, while clearing Up next, clearing the queue and removing a
   multi-selection confirm first and say how many items will go; from R2
   they act at once and offer Undo instead.
7. **Status messages are announced** without moving focus (WCAG 4.1.3).
8. **A security stop has no way past it.** When going on would be unsafe,
   the screen explains and offers the safe next step; it never offers
   "continue anyway" (see
   [Security prompts, stops and notices](#security-prompts-stops-and-notices)).

### The states

| State | Where | What the screen shows | Feature, release |
|---|---|---|---|
| First scan running | Home, library | Scan progress with counts ("1,240 of about 8,000 files"); albums appear as they are found. Never an empty grid | DIS-004, LIB-021, R1 |
| No music folder yet | Setup | The add-folder step with live checks for readability and filesystem type | ADM-025, R1 |
| No libraries shared with this person | Home | "Your account has no libraries yet. Ask the server's owner to share one." | ACC-037, R1 |
| A new person's home | Home | The default rows (DIS-004); rows with nothing in them yet stay hidden, with one first-run card that explains how to add rows | DIS-004, DIS-003, R1 |
| Search with no results | Search | "No matches for 'beyonse' in Music", the nearest forgiving matches, and a button to search everything (DIS-085, DIS-088) | R1 |
| Search with nothing typed | Search | Recent searches, kept only on this device (only in memory on a computer marked as shared), with "Clear" (DIS-089, SEC-PRV-004, SEC-CLI-010) | R1 |
| Empty queue | Queue | "Nothing queued." with recently played items to start from | MUS-116, R1 |
| No lyrics | Lyrics view | "This file has no lyrics." From R2, "Find lyrics" appears only if the lyrics plugin has been granted (MUS-162); it names the provider and runs only when pressed (SEC-PRV-015) | MUS-154, R1 |
| Nothing hidden | Hidden page | "Nothing hidden. Items you dismiss or hide appear here so you can bring them back." | DIS-023, R1 |
| Private session on | History, bar | The private indicator; history explains that this session is not recorded and that admins do not see its titles | ACC-117, SEC-PRV-024, R1 |
| Artwork loading | Every tile | The placeholder gradient, then a fade-in | LIB-142, R1 |
| Buffering | Bar, player | The buffered range shows in the progress track; "Buffering" appears in the bar after 1 s | MUS-070, R1 |
| Track cannot play in this browser | Rows, queue | Dimmed artwork, an icon and the reason; the queue skips it with a notice and a link to the health report | MUS-229, R1 |
| Damaged file | Player | "Skipped a damaged file" with a link to the health report; nothing is played | MUS-079, R1 |
| Server unreachable | Every screen | A quiet `status.info` banner: "Can't reach the server. Browsing and search still work; playing needs the server." Items that cannot play are dimmed (CLI-025, CLI-026) | R1 |
| Drive offline | Library | Items greyed with "Drive offline"; nothing is removed | LIB-032, R1 |
| Not a secure address | Help page rendered by the server | Over plain HTTP from anywhere but the server's own machine, the server sends only a redirect to its HTTPS address when one exists, or otherwise a static help page: how to reach the secure address, in plain words, with no sign-in, no server name and no version (SUR-109). The web client never runs there, so there is no reduced app to explain (owner to confirm, open question 11) | CLI-150, SEC-NET-001, SEC-NET-005, SEC-NET-024, R1 |
| Client too old for the server | Every screen | "Update needed", with what still works | CLI-032, R1 |
| Sign-in failed | Sign-in | One message, "That didn't work. Try again, or sign in another way.", whatever the cause, with the same response and timing whether or not the account exists; there is no password, so there is no "wrong password" (owner to confirm, open question 12) | ACC-007, SEC-IAM-025, SEC-IAM-022, SEC-API-058, R1 |
| Known security advisory | Admin screens | A `status.warning` banner naming the fixed version | ADM-054, R1 |
| Server starting or migrating | Startup page | To anyone not signed in: "This server is starting. It will be ready in about *n* minutes." and nothing else. The steps, the snapshot location and configuration errors appear on the host console and to the owner after sign-in | ADM-032, SEC-OPS-050, R1 |
| Client will not load | Emergency page | A minimal page rendered by the server, after sign-in with an admin session: status, restart, and the logs the person's role may read. Downloading a backup is the owner's alone and asks for a fresh fingerprint or face check. Anyone else gets the same answer as for any route they may not use | ADM-113, SEC-IAM-041, SEC-OPS-027, SEC-OPS-045, SEC-OPS-050, R1 |
| Library problems | Library health | Each file, what is wrong and where, and the fix where one exists | LIB-193, LIB-194, R1 |

The help, startup and emergency pages are rendered by the server without
the client bundle. They use the same tokens through one small stylesheet
served from the server's own origin (inline styles are not allowed by
SEC-API-044) and the system font stack, so they stay readable and tiny even
when the client is broken. They carry no inline script (SEC-HIS-028): the
help and startup pages need no script at all, and the emergency page's
sign-in uses script files from the server's own origin. Pages shown to
anyone not signed in never reveal the version, build, file paths, stack
traces or database errors (SEC-OPS-050).

The earlier draft of this table had three rows the baseline does not
allow. "This address limits the web app" assumed the client runs on a
plain-HTTP LAN address with fewer features; SEC-NET-001 gives such peers
only the help page. "Sign-in failed" spoke of passwords, which do not
exist (SEC-IAM-025). The startup page showed the snapshot location to
anyone, which is a file path (SEC-OPS-050), and the emergency page had no
access rule.

### Security prompts, stops and notices

The baseline's thirteenth first principle says a control people will
switch off is a broken control, and the same is true of a warning people
learn to click past. So security has a small, fixed set of patterns, each
designed to be answered correctly by someone holding a TV remote for the
first time. They appear rarely, and only for things that matter, so nobody
learns to dismiss them.

**1. Step-up: "Confirm it's you".**

- *When.* Only before the actions the baseline tags fresh-uv (trusted
  proxies, remote administration, TLS and naming, egress, plugins,
  adapters, backup download and restore, ownership transfer, key rotation,
  creating or promoting administrators, library folders and folder
  browsing), when entering admin screens after the 15-minute admin session
  has lapsed, and when approving a device (SEC-IAM-041, SEC-IAM-058). At
  most once every 5 minutes for fresh-uv actions. Never for playing,
  browsing, queueing or personal settings.
- *Look.* A sheet on `bg.overlay` that names the action and its object in
  one sentence ("Confirm it's you to add a music folder"), with "Continue"
  as a steel primary button (brass means play, section 3) and "Cancel".
  Continue hands over to the browser's or the operating system's own
  passkey, fingerprint, face or device-PIN prompt, bound to a key on
  native personal devices (SEC-CLI-059). Gunmetal never draws its own
  fingerprint or face prompt, and step-up never asks for a password
  (SEC-IAM-025) or a typed code, so a step-up that does is visibly not
  Gunmetal. The only codes anyone types are a pairing code shown on
  another device (SEC-IAM-060) and a recovery code during recovery.
- *On a limited device* (a TV, a computer marked as shared, a browser
  signed in by approval from a phone), step-up cannot happen
  (SEC-CLI-024, SEC-IAM-108). The sheet reads "Finish this on your phone"
  and shows a QR code that opens the same screen on a personal device. The
  code carries only the screen's address, never a secret, and nothing
  changes on the limited device.
- *Cancel or failure* returns to where the person was with "Not
  confirmed. Nothing was changed." There is no partial change and no
  retry loop.

**2. Stops: warnings with no way past.**

- When going on would expose the person or the household, Gunmetal stops
  instead of warning. A stop says what happened, what it protects, and the
  safe next step. It has no "continue anyway", "trust", "ignore" or hidden
  "advanced" path, in any client, for anyone, the owner included
  (SEC-CLI-042, SEC-CLI-043; first principles 12 and 13).
- The stops are: a server whose identity cannot be confirmed (pattern 6);
  an address that is not secure (the help page, above); a feature that is
  off because its isolation is missing, for example "Transcoding is off
  because this server can't sandbox it", with what would turn it on and no
  switch to run it unconfined (SEC-MED-024); and a household device that
  is dormant or away from home (SEC-IAM-109).
- *Look.* A full screen or full-width card with the `status.danger` icon
  and a heading in `text.primary`, not red text. The safe action is the
  only primary button.
- Anything that is not a stop is a notice (pattern 4). Gunmetal never uses
  a warning dialog for information, which is what teaches people to click
  through warnings.

**3. Confirming a choice that grants or removes access.**

- Approving a device, inviting someone, raising a role and revoking all
  devices each confirm by naming the exact result: who or what, which
  profiles and libraries, which capabilities.
- *Approving a device* shows the requesting device's name labelled "Name
  given by the device", its type, "In this home" or "Somewhere else", how
  long ago it asked and exactly what it will get, with addresses only
  behind "Details" (SEC-IAM-058). When the approval is not proven local,
  the person types the code shown on the other device and confirms a
  matching code, worded neutrally: "Type the code on the TV to confirm"
  (SEC-IAM-060). Approvals always start on the approving device, so nobody
  without a session can put an approval prompt on a person's screen
  (SEC-STD-027).
- Buttons are verbs that say the result ("Give access", "Don't give
  access"). The safe choice has initial focus, nothing is preselected, and
  nothing accepts itself after a countdown.
- On Android these controls ignore touches while another app draws over
  them (SEC-CLI-058), and screens showing a recovery or approval code are
  hidden from screenshots and the app switcher (SEC-CLI-057).

**4. Notices: what changed on your account.**

- Each account's devices are told about a new device, a credential added
  or removed, use of recovery, a change of identity-provider link, a change
  of role and bursts of failed attempts. Signing in again on a known device
  produces nothing, and non-critical notices are batched into a daily
  summary (SEC-IAM-098). Owner alerts follow SEC-OPS-032, and the ones
  SEC-OPS-034 names cannot be switched off.
- Every notice about a device or credential offers "This wasn't me", one
  step that revokes it, ends its sessions and stops its streams
  (SEC-OPS-033), beside "It was me", which dismisses it. During a recovery
  hold, each existing device's notice can end the hold with one tap
  (SEC-IAM-106).
- An administrator's change to a person's credentials, role or access is
  shown to that person at their next sign-in and in their account activity
  (SEC-PRV-026), and an administrator's access to their data appears in
  their own security log (SEC-IAM-077, SEC-IAM-097).
- On a lock screen a notice says only that something happened, for
  example "Something changed on your Gunmetal account", with no titles,
  names or device details (SEC-CLI-062, SEC-PRV-056).
- *Look.* An entry in the account menu's notices list with a count; the
  `status.warning` icon and words for security notices, `status.info` for
  the daily summary. Addresses appear in full only to the person they
  concern; the owner sees them shortened (SEC-OPS-027).

**5. Revocation, seen from the device that lost access.**

- The device deletes the account's data first, then draws the sign-in
  screen with one sentence, "This device was signed out." It shows no
  titles, no library and no earlier screen, on the web (SEC-CLI-009) and
  natively, where downloads are deleted too (SEC-CLI-037). Playback stops
  at the next range request (SEC-IAM-043).
- It does not say who signed it out; the person can read that in their own
  security log after signing in again (SEC-IAM-097).

**6. Certificates and server identity.**

- *Web, R1.* Gunmetal's own pages never run on an untrusted certificate:
  the server does not serve the web client without a valid one
  (SEC-NET-005), and browsers stop first. Nothing in Gunmetal, its help
  page or its documentation tells anyone to click past a browser's
  certificate warning. The help page explains how to reach the secure
  address.
- *Native apps, R2.* A server whose certificate fails validation, or whose
  identity key does not match the key pinned at enrolment, gets a stop:
  "Gunmetal can't confirm this is your server. Nothing was sent to it."
  The actions are "Try again" and, when downloads exist, "Keep listening
  offline"; "Details" shows the expected and presented key fingerprints
  for the owner. There is no way to continue (SEC-CLI-042, SEC-CLI-043,
  SEC-NET-060). A self-signed server is trusted only through the key
  fingerprint carried in its invitation or QR code, never by a prompt
  (SEC-CLI-042), and a key change signed by the old key is followed with
  no prompt at all (SEC-NET-062).
- *For the owner.* Alerts 30 and 7 days before the certificate expires,
  with "Renew now" (SEC-NET-072), and a critical alert when a certificate
  for the server's name appears that the server did not request
  (SEC-NET-069).

**7. Leaving Gunmetal.** A link to another origin opens only after the
sheet in section 6 that names the destination host, with Cancel focused;
on a TV it is a QR code (SEC-STD-015).

**The security states.**

| State | Where | What the screen shows | IDs, release |
|---|---|---|---|
| Confirm it's you | Before fresh-uv actions, admin screens after the admin session lapses, device approvals | Pattern 1 | SEC-IAM-041, SEC-IAM-058, R1 |
| Finish this on your phone | The same, on a TV, a shared computer or a phone-approved browser | A QR code to the same screen on a personal device; nothing changes here | SEC-CLI-024, SEC-IAM-108, R1 (browsers), R2 (TVs) |
| This device was signed out | Any screen, after revocation | Pattern 5 | SEC-CLI-009, SEC-CLI-037, R1 (web), R2 (native) |
| Something changed | Notices list, lock screen | Pattern 4 | SEC-IAM-098, SEC-OPS-033, R1 |
| Recovery hold | Existing devices; the new credential's screens | Existing devices: "A new way to sign in was added through recovery. If this wasn't you, end it now." with one button. The new credential: what it cannot do yet, and when the hold ends | SEC-IAM-106, R1 |
| Not a secure address | Help page | See the states table above | SEC-NET-001, R1 |
| Feature off for safety | Admin health and the feature itself | What is off, why, and what would turn it on; a "reduced isolation" notice when the scan worker runs with less confinement | SEC-MED-024, R1 (notice), R2 (transcoding) |
| Too many streams | Player | The player's "Not allowed" state | SEC-TM-068, R1 |
| Can't confirm this server | Native apps | Pattern 6 | SEC-CLI-042, SEC-CLI-043, R2 |
| Offline grant expired | Native player | "Connect to your server once to keep listening offline", in `status.info`, not as an error | SEC-CLI-036, R2 |
| TV asleep or away from home | Household TV | "This TV has been asleep for a while. Someone in your household can wake it from their phone." A TV seen away from home is suspended and the adults are told | SEC-IAM-109, R2 |

## 12. Accessibility requirements

WCAG 2.2 Level AA is the floor for every surface, with the Level AAA items
named below where they are cheap. CLI-136 makes these a release gate: axe
checks on the web build, accessibility snapshot tests on native, and a
written VoiceOver and TalkBack script before each release.

| # | Requirement | Standard | Feature | How it is checked |
|---|---|---|---|---|
| A1 | Text meets 4.5:1 (3:1 for large text); controls, graphics and focus indicators meet 3:1 against adjacent colours, in every theme and on every artwork tint | WCAG 1.4.3, 1.4.11 | CLI-141 | Token pair test in CI; hue sweep for artwork tints |
| A2 | Primary and secondary text meet 7:1 in every theme | WCAG 1.4.6 (AAA) | CLI-141 | Token pair test |
| A3 | Colour is never the only signal: badges have text, states have icons or words, focus has shape | WCAG 1.4.1 | CLI-026, MUS-099 | Review checklist; greyscale screenshot test |
| A4 | Every control has a role, an accessible name and its state, including player controls drawn over video | WCAG 4.1.2 | CLI-135, MUS-227, VID-140 | axe; native accessibility snapshots |
| A5 | Everything works by keyboard with a visible focus ring; nothing traps focus except modals, which close with Escape | WCAG 2.1.1, 2.1.2, 2.4.7 | CLI-138 | Keyboard-only end-to-end tests |
| A6 | The focus ring is at least 2 px with a gap and 3:1, and sticky elements never hide the focused item | WCAG 2.4.13 (AAA), 2.4.11 | CLI-138 | Focus screenshot tests |
| A7 | On TV, focus is always on one actionable element, returns where it was on Back, and is announced by TalkBack | Platform | CLI-036, CLI-137, R2 | Focus-map tests |
| A8 | Text scales to 200% on the web and to the largest system size natively, without clipping or loss | WCAG 1.4.4 | CLI-139 | Layout tests at the largest sizes |
| A9 | Screens reflow at 320 CSS px with no horizontal scrolling, and survive the text-spacing overrides | WCAG 1.4.10, 1.4.12 | CLI-149 | Viewport tests |
| A10 | Pointer targets are at least 24 by 24 CSS px; Gunmetal sets 44 for touch and 32 for pointers | WCAG 2.5.8 | CLI-142 | Target-size tests |
| A11 | Every gesture and every drag has a single-pointer, non-dragging alternative (for example "Move up", "Move to top of Up next" in the queue menu) | WCAG 2.5.1, 2.5.7 | CLI-142, MUS-119 | Review checklist; end-to-end tests |
| A12 | Motion follows the system setting and the in-app override; moving content longer than 5 s can be paused; nothing flashes more than three times a second | WCAG 2.2.2, 2.3.1, 2.3.3 (AAA) | CLI-140, VID-141 | Tests with reduced motion forced on |
| A13 | Status messages (track changes when the user asks for them, undo notices, sync state) are announced without moving focus | WCAG 4.1.3 | CLI-135 | Screen-reader script |
| A14 | Tooltips and hover cards can be dismissed, hovered and stay until dismissed | WCAG 1.4.13 | CLI-138 | End-to-end tests |
| A15 | Sign-in needs no memory or puzzle test: it is a passkey, an identity provider's own page, or approval from the person's phone, on every address the client runs on, with no password to remember (SEC-IAM-025, SEC-IAM-108). Code and share-link password fields allow paste and password managers (SEC-CLI-028). Owner to confirm (open question 12) | WCAG 3.3.8 | ACC-050 | End-to-end tests |
| A16 | Time limits can be extended; undo notices stay at least 10 s and while focused | WCAG 2.2.1 | DIS-023 | End-to-end tests |
| A17 | Mono audio and left-right balance are available | Not a WCAG item | CLI-151 | Unit tests of the audio graph |
| A18 | A high-contrast theme, and Windows forced colours keep borders and focus visible | Not a WCAG item | CLI-141 | Screenshot tests in forced colours |
| A19 | Subtitles start from the system caption settings, with no size cap (R2) | Platform | CLI-143, VID-091 | Native tests at the extremes |
| A20 | Strings are translatable and pseudo-locale builds catch clipping; R1 uses logical layout properties so right-to-left works in R2 | Not a WCAG item | CLI-146, CLI-147 | Pseudo-locale CI build |
| A21 | Security prompts, stops and notices (section 11) are announced when they appear, hold focus until answered, start with focus on the safe choice, and are covered by the screen-reader script; untrusted names inside them are read in their own isolate | WCAG 4.1.2, 4.1.3, 2.4.3 | CLI-135, SEC-IAM-058, SEC-MED-057 | Screen-reader script; end-to-end tests of each pattern |

## 13. What each release needs from this document

- **R1 (music, web client).** All four themes and the token file with its
  contrast test; the artwork tint pipeline and its hue sweep; Inter bundled
  under the security policy; every component passing the content security
  policy test on every screen (SEC-API-044); the untrusted-text rules of
  section 6; the compact and wide layouts with their density tables;
  keyboard focus; motion tokens and reduced motion; the icon set and
  badges; every R1 state in section 11, including the security patterns
  (step-up, stops, approvals, notices, revocation, the help page); the
  server-rendered help, startup and emergency pages; accessibility
  requirements A1 to A6, A8 to A18, A20 and A21.
- **R2 (video and native clients).** The TV type scale, TV focus system,
  rail and text-size setting (CLI-035 to CLI-043); native phone and tablet
  layouts; car artwork and icons for Android Auto; scrims over video and
  subtitle styling with the bundled subtitle fonts (VID-091, CLI-143,
  SEC-MED-054); TV ambient mode, screensaver and backdrops (CLI-041,
  CLI-042, DIS-164); the converted badge; the kids theme (DIS-152);
  right-to-left mirroring (CLI-147); the native security states (the
  server-identity stop, "Finish this on your phone" on TVs, the offline
  grant notice, the dormant TV); requirements A7 and A19.
- **R3 (live TV).** The guide grid uses the same tokens: cells are
  `bg.raised`, focused cells use the plate, the "now" line (LIV-067) is
  `accent.indicator`, past programmes use `text.muted`, and flags such as
  "New" and "Live" are text badges. The stats overlay (LIV-086) sits on
  `scrim.text`.
- **Later.** Alternative icons (CLI-154), the visualiser with a flash limiter
  (MUS-083), the plugin row's source badge (DIS-018), a numbered list style
  of Gunmetal's own (DIS-139), the Apple platforms, and the desktop shell's
  layouts and mini player. The first draft of the feature map put the
  desktop shell in R2; the security release-scope table lists it as Later
  (SEC-TM-074), its controls are SEC-CLI-069, also Later, and the feature
  map now follows them (owner to confirm, open question 10).
- **No.** Copying any product's trade dress (section 2), overlays burned into
  artwork (LIB-147), and any visual treatment for social feeds (DIS-177,
  MUS-191) or vendor content rows (DIS-176).

## 14. Open questions for the project owner

1. **Inter as the interface typeface.** Recommendation: yes; it is the
   wordmark's typeface and is OFL-1.1. Its script coverage and the size of
   the bundled file are unverified and must be measured.
2. **The nut shape for artists and profiles.** Recommendation: prototype it
   and run a short usability test against circles before R1 screens are
   built; keep it for the play button either way.
3. **TV row scrolling.** Recommendation: test a fixed focus column against
   focus that moves until the edge, on the cheapest supported stick, and
   pick by measured frame time and a small user test.
4. **Artwork tint in the now-playing bar.** Recommendation: no; the bar stays
   steel as the stable anchor.
5. **Light theme on TV.** Recommendation: available, never the default.
6. **Kids theme (DIS-152).** Recommendation: the same tokens with a larger
   type step and bigger tiles, designed with the R2 kids work rather than
   now.
7. **Icon set.** Recommendation: Lucide plus Gunmetal glyphs, subject to the
   licence check.
8. **React Native Web and the content security policy.** Recommendation:
   prove before R1 that styling and per-album colours work under
   `style-src 'self'`; if they do not, the policy and the styling approach
   must be decided together, not loosened quietly. The baseline's own
   recommendation for that case (web open decision 7, kept by owner
   decision 25 unless the owner objects) is to try a hash of an empty style
   element first, then accept `'unsafe-inline'` for `style-src` only,
   recorded, and never for `script-src` (SEC-HIS-028). The design rules in
   section 1 do not depend on that fallback: tokens are static stylesheets
   and run-time values go through the CSSOM.
9. **Embedded subtitle fonts (owner to confirm, owner decision 12).**
   Recommendation: the baseline's. Styled subtitles use the bundled, pinned
   font set, and a library's embedded fonts are used only after the owner
   opts that library in and the parse worker rewrites each font
   (SEC-MED-054). Some typeset subtitles look plainer until then.
10. **The desktop shell's release (owner to confirm).** The first draft of
    the feature map said R2; the security release-scope table (SEC-TM-074)
    and SEC-CLI-069 say Later, and the feature map now agrees.
    Recommendation: keep it Later and design desktop layouts with the
    shell. If the owner wants it in R2, SEC-CLI-069 and the desktop player
    sandbox move with it.
11. **No web client on a plain-HTTP address (owner to confirm, owner
    decision 2).** Recommendation: the baseline's. Over plain HTTP every
    peer but loopback gets only the static help page (SEC-NET-001), so the
    "This address limits the web app" state of CLI-150 becomes the help
    page.
12. **No passwords (owner to confirm, owner decision 1).** Recommendation:
    the baseline's. Sign-in is by passkey, identity provider or approval
    from a phone (SEC-IAM-025, SEC-IAM-108), so the sign-in failure copy
    and accessibility requirement A15 no longer mention passwords, and the
    password fallback (ACC-052) has no screen here.

## 15. Sources

Project files:

- `README.md`, `docs/banner.svg`, `docs/icon.svg`
- `docs/adr/0001-architecture.md`, `docs/adr/0002-music-is-first-class.md`
- `docs/features/README.md` and the area files `music.md`, `clients.md`,
  `discovery.md`, `library.md`, `accounts.md`, `admin.md`, `video.md`,
  `live-tv.md` and `later-media.md`
- `docs/research/music-ux.md`, `docs/research/discovery-home-and-search.md`,
  `docs/research/clients-platforms-and-offline.md`
- `docs/security/README.md`, `docs/security/threat-model.md`,
  `docs/security/web-and-api-security.md`,
  `docs/security/client-and-device-security.md`,
  `docs/security/identity-and-access.md`,
  `docs/security/network-and-remote-access.md`,
  `docs/security/privacy-and-data-protection.md`,
  `docs/security/operations-and-incident-response.md`,
  `docs/security/supply-chain-and-release.md`,
  `docs/security/media-and-parser-safety.md`,
  `docs/security/standards-coverage.md`

External pages read on 2026-10-02:

- Web Content Accessibility Guidelines 2.2: https://www.w3.org/TR/WCAG22/
- Understanding Success Criterion 2.5.8:
  https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html
- Inter typeface repository: https://github.com/rsms/inter
- Lucide licence: https://lucide.dev/license

Vote counts, redesign histories and rival behaviour are taken from the
research files above, which cite their own sources.
