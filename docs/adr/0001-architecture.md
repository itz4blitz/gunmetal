# 1. Architecture

Date: 2026-10-02
Status: accepted

## Context

Plex and Jellyfin both lean on transcoding, run FFmpeg with the server's full
privileges, and fetch every screen from the server. Plex also depends on a
central account service. We want something that needs far less hardware, has
a much smaller attack surface, and stays fully open source.

## Decisions

1. **Own both ends.** We ship the server and the clients. A client with its
   own player (libmpv) can play almost any original file, so transcoding
   becomes the exception.
2. **Rust core, shared everywhere.** Protocol types, the playback decision
   engine, container parsers and the remuxer live in one crate, compiled
   into the server, the native clients (UniFFI) and the web client (WASM).
3. **FFmpeg off the hot path.** Scanning, probing, direct play and remuxing
   use pure-Rust parsers. FFmpeg runs only for real transcodes, in a
   sandboxed worker process.
4. **Segment map built at scan time.** Each file's keyframe index is read
   once and stored, so playlists are exact and any segment can be produced
   independently.
5. **SQLite, treated as a rebuildable cache.** Watch history is the only
   irreplaceable data and lives in an append-only, exportable log.
6. **Native API first.** Random IDs, per-object authorisation, short-lived
   signed stream URLs. Jellyfin API compatibility is an optional adapter, not
   the foundation; it also covers platforms we have no client for (Roku).
7. **No central account.** Passkeys and OIDC, device-bound keys, remote
   access over iroh.
8. **One React Native UI codebase** in TypeScript for phones, TVs, browsers
   and a desktop shell, with native player modules per platform.
9. **Hosting on Cloudflare** for the docs and landing page.
10. **AGPL-3.0-or-later, DCO, no CLA.**

## Scope

The first version covers movies and TV shows. M3U and live TV are the first
module after that. Music, photos and books come later.

## Consequences

- The pure-Rust remuxer is the largest piece of work and the main schedule
  risk (Dolby Vision, lossless audio, image-based subtitles).
- Transcode speed and codec support will match the rivals, not beat them:
  everyone uses the same FFmpeg and the same GPUs.
- The performance claims (scan time, time to first frame, idle footprint) are
  design expectations until benchmarked against Jellyfin on the same library.
  That benchmark is the first milestone.
