# 23. The web client is the player you open

Date: 2026-10-08
Status: accepted, as the product entry for the player that already runs in
the browser. It has no entry in the [decision register](../decisions.md)
yet; the owner adds one if he wants it tracked there.

## Context

Record 1 puts one React interface on every platform, and the web client is
the R1 desktop experience. The security baseline puts a native desktop
shell in Later (D-10, D-15, SEC-CLI-069). A shell added now would be a
regression, not a shortcut.

The clickable player was built in `clients/apps/demo` so there was a screen
before the server could serve a library. That app is what a person opened.
The web app was still a stub: a wordmark, an uptime clock, and the line
"Library and playback are not wired yet." The server can now serve a folder
library beside the page. The client you open has to be the one that plays
it, on a wide desktop and on a phone, without a new playback engine and
without a native shell.

## Decisions

1. **`clients/apps/web` is the composition root and the client you open.**
   `pnpm dev` and `pnpm build` already pointed here. They stay the default.
   The page is the polished document (title, manifest, icons) and the same
   player shell the demo mounts: library, album, artist, search, settings,
   player bar, queue and lyrics. Wide width is the three-pane layout.
   Phone width is the compact layout, and playing opens the full player.
   No Electron, Tauri, or other native shell is added.

2. **The player modules stay where their tests prove them.** The web app
   imports the demo's composition, playback controller and served-library
   reader. It does not re-implement queue, shuffle, gain or search. A full
   move of those modules would drop the coverage those tests hold, so
   `clients/apps/demo` remains, deprecated: its package description and
   README say to open `clients/apps/web`, and new features do not land
   there. The fixture catalogue is the fallback when `/library.json` is
   missing or refused, and that fallback still shows the "Demo data" label.

3. **The server contract is same-origin, relative URLs only.** The page
   fetches `GET /library.json`. A track's audio is `GET /media/library/{id}`
   where `{id}` is 16 hex characters. A cover, when present, is
   `GET /media/library/covers/{id}.jpg` or `.png`. Production code does not
   name a host. The Vite dev server may proxy those two paths to
   `http://127.0.0.1:8788`. That proxy is not part of the built client.

4. **The web client's security boot stays.** WebAssembly and a secure
   context are still required before the player mounts (SEC-API-052). The
   dev server still binds to loopback (SEC-CLI-019). Untrusted catalogue
   text is still rendered as text.

## Consequences

A person opening the web client sees the player. A person opening the demo
is told it is deprecated. The server that wants to be played must serve the
folder document and the media paths above on the same origin as the page.
Until a later package moves the player modules out of `apps/demo`, the web
bundle includes the fixture catalogue those modules import. That is the
fallback, not a second server.
