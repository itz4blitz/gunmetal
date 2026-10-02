# 2. Music is first-class

Date: 2026-10-02
Status: accepted. Supersedes the Scope section of [record 1](0001-architecture.md).

## Context

Record 1 put music after movies, TV and live TV. That would make Gunmetal a
video server with a music add-on, which is what Plex and Jellyfin already
are. We want a full media player: music that works out of the box, with a
player good enough to replace a streaming app.

## Decisions

1. **Music ships in the first version**, alongside movies and TV. M3U and
   live TV remain the first module after that; photos and books come later.
2. **Music is the first usable release.** Browsers and phones already play
   FLAC, MP3, AAC and Opus, so a music library needs no remuxer and no
   transcoder: the server parses tags, builds the library and serves bytes.
   That makes it the shortest path to something people can run, while the
   video remuxer (the largest piece of work) is built behind it.
3. **Pure-Rust audio parsing in the core.** Tags, embedded artwork, gapless
   metadata and loudness are read at scan time by the same crate that parses
   video containers, with the same testing rules.
4. **A real music model**, not files in folders: artists, release groups,
   albums, tracks, playlists and a play queue that syncs across devices.
5. **Player features that matter for music**: gapless playback, loudness
   normalisation, background playback with lock-screen controls, offline
   downloads, synced lyrics, and a queue that hands off between devices.
6. **OpenSubsonic as an optional adapter**, in the same role the Jellyfin
   adapter plays for video, so existing music apps can connect.
7. **The interface is dark-first and music-forward**: a persistent
   now-playing bar and queue, artwork-led browsing. It takes its cues from
   the best streaming apps but has its own identity; we do not copy another
   product's assets or trade dress.

## Consequences

- The build order changes: core parsers, then a music vertical slice
  (server plus web client), then video.
- The core crate grows audio formats (FLAC, MP3, MP4 audio, Ogg) before it
  grows the video remuxer.
- Transcoding audio for mobile data (to Opus) is cheap enough to run on any
  hardware, so it does not conflict with the low-specs goal.
- Scrobbling and metadata lookups reach third-party services, so they belong
  in plugins with explicit network grants, not in the server itself.
