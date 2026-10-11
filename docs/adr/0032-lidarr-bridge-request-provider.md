# 32. The Lidarr bridge is a request-provider plugin in R2

Date: 2026-10-09

Status: proposed, for the owner's review. Prepared 2026-10-09 at the
owner's request, after the plugin platform review. The number follows
record 31, which the GitHub parity merge added. It has no entry in the
[decision register](../decisions.md) yet; the owner adds one if he wants
it tracked there.

## Context

INT-128 ("Music requests through Lidarr") is the feature the owner wants
first: request an album you do not have, follow an artist, and see what
Lidarr is still looking for — from the player, not from another app. The
platform facts this record builds on:

- The plugin host (WP-225) parses a closed manifest, admits exact public
  hosts over HTTPS on port 443, admits a LAN destination only as an exact
  host and port an admin granted (`LanGrant`, `admit_lan`), reads
  owner-scoped secrets (`SecretDecl { per_user: false }`), and runs worlds
  from a closed list. Today that list is `scrobbler@1` and
  `music-metadata-provider@1`; both link only `egress`, `kv`, `log` and
  `secrets` (`crates/gunmetal-plugins`, landed with the plugin host rules).
- The occupant catalogue renders six slots; none is request-shaped
  (`gunmetal-core/src/slots.rs`, `catalogue.ts`).
- SEC-EXT-076: a new capability arrives as a new versioned interface with
  its own permission name and consent text through an architecture record.
  This is that record.
- Lidarr's current line is 3.1.x. The calls this bridge needs are the v1
  REST API behind `X-Api-Key`: artist lookup by `mbid:`, `POST /artist`
  with `addOptions.albumsToMonitor` (there is no `specificAlbum` monitor
  value), `POST /command` (`AlbumSearch`, `MissingAlbumSearch`),
  `GET /wanted/missing` and `GET /calendar`. Lidarr webhooks are
  fire-and-forget (no retry, 100-second timeout), so no state may depend
  on them.
- Import refresh stays where INT-118 put it: Lidarr's custom-script
  connection calls the server's native path-scoped refresh (INT-011). The
  OpenSubsonic adapter cannot carry it (`startScan` is outside its
  allowlist; A-321). The refresh-only key's scope is still open (D-55,
  LIB-026); this record does not close it and the bridge does not depend
  on it.

## Decisions

1. **A new world, `request-provider@1`.** It exports
   `request-album(evidence)`, `follow-artist(evidence, monitor)` and
   `wanted(evidence)`. It imports the four host interfaces every world
   links (`egress`, `kv`, `log`, `secrets`) and nothing else. Evidence is
   the MBIDs, names, year and track count the host passes; the plugin
   reads no files, no library and no paths.
2. **A seventh slot, `request-provider`.** The catalogue, the client
   types and the demo slot table gain the slot together; its feature is
   INT-128 and its vacancy detail is `request-plugin`. The six-slot
   literal is amended, not silently widened.
3. **The trigger path is host-owned.** "Request album" and "Follow in
   Lidarr" are core UI actions dispatched to a host route; the client
   plugin contract does not change (record 22 stands: contributions are
   data). A third-party action descriptor is not part of this record.
4. **The Lidarr API key is an owner-scoped secret** (`per_user = false`),
   entered in the schema-drawn settings form, encrypted at rest, never
   returned and never logged.
5. **The network grant is the owner's LAN grant.** The manifest names the
   use; the owner grants the exact Lidarr host and port; the host refuses
   a fetch whose URL does not match the grant. Loopback, link-local and
   metadata addresses can never be granted.
6. **One member's action changes the household's Lidarr.** The consent
   text says so, and the default policy is owner-only until the owner
   decides otherwise.
7. **Wanted and monitored states arrive as typed annotation data**, drawn
   by Gunmetal's clients; the shapes are validated per SEC-EXT-028 and
   added when a test needs them.
8. **The plugin receives no inbound calls.** Lidarr cannot notify it
   (SEC-EXT-031). Refresh stays native (INT-118); wanted state refreshes
   on the plugin's scheduled task, and every write is idempotent because
   Lidarr does not redeliver.
9. **The store lists the record before it is built.** `lidarr-bridge`
   (plane server, slot `request-provider`, status `not-in-build`) is
   published through `itz4blitz/gunmetal-extensions`; listing installs
   nothing.
10. **The bridge ships after the host.** The work is WP-243, a package
    after WP-225; no WebAssembly runtime is named before SEC-EXT-019 to
    SEC-EXT-034 pass (SEC-EXT-018), and no client contract change rides
    along.

## Consequences

- The plugin platform's first action-shaped world exists; the world and
  slot additions follow SEC-EXT-076 and are versioned.
- INT-128 moves from Later to R2; INT-118 is unchanged and remains the
  import-refresh path. The bridge works with manual rescans until D-55
  resolves.
- The feature is not first to market: Seerr merged Lidarr connectivity on
  2026-06-08 and still ships no music requests, and the SeerrNG fork
  ships them; no player does them in-app. The difference this record buys
  is in-player requests with a sandboxed plugin, exact LAN grants and
  owner-scoped secrets.
- Tests follow CONTRIBUTING: pure logic in host-testable code, a fake
  Lidarr HTTP server, fuzzed response decoding, sandbox and egress tests,
  and a log canary for the key.
