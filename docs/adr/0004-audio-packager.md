# 4. The audio packager runs in a worker process

Date: 2026-10-03
Status: accepted, through the owner's answers to D-09 and D-10 on
2026-10-02 ([decision register](../decisions.md#owner-answers-2026-10-02)).
Amends decision 2 of [record 2](0002-music-is-first-class.md). Supersedes
the in-process remuxer of [record 1](0001-architecture.md) and the
"Remux in-process" box of the README diagram.

## Context

Record 2 says a music library needs no remuxer, because browsers and phones
already play FLAC, MP3, AAC and Opus. That holds for playing one file from
start to end. It does not hold for gapless playback in a browser (MUS-067)
or for exact seeking there: both need Media Source Extensions, and which
audio containers each browser's Media Source Extensions accept, including
Safari's ManagedMediaSource, is unverified. Fragmented MP4 is the container
they are expected to share. So R1 needs a light, audio-only packager that
copies frames into fragmented MP4 without re-encoding them (MUS-230, R1).

A packager reads untrusted bytes. The security baseline's second principle
says untrusted input is parsed in a separate process; SEC-MED-018 puts all
media parsing in workers, and SEC-MED-081 asks the same of every remuxer.
The first draft of this record put the packager in the server process. The
owner's answer to D-09 settles it: parsing, remuxing and decoding run in a
separate jailed worker, which supersedes the in-process remux of record 1.
The same answer makes R1 servers Linux only (x86-64, AArch64 and a
container image) and leaves transcoding out of R1.

CUE sheets (MUS-041, R2) raise the same question. A browser cannot play a
sample-accurate part of one FLAC file, so each virtual track is served as a
re-headed FLAC slice, which is packaging too.

## Decisions

1. **One exception to "no remuxer for music".** Record 2 decision 2 is
   amended: music needs no transcoder and one light remuxer, the audio
   packager. It copies FLAC frames, Opus packets and MP3 frames into
   fragmented MP4 (an initialisation segment and numbered media segments)
   and keeps the trim values that make gapless work. It never decodes or
   re-encodes audio. Its code is a sans-I/O function in the core
   (`gunmetal-core/src/package/`, WP-056), under the parsing contract:
   limits, the step budget of SEC-MED-007 and typed errors. WP-056 settles
   which codecs it carries against real browsers; whatever it carries, it
   only copies.
2. **It runs only in a worker process, never in the server process.** The
   worker is the server's own binary, started by the sandbox launcher with
   a hidden subcommand ([record 6](0006-workspace-and-dependencies.md)). It
   is single-threaded and confined as SEC-MED-021 and SEC-MED-022 require.
   Packaging workers come from a small pool of their own, so a long scan
   never starves playback.
3. **Input by descriptor.** The server opens the file beneath its library
   root, checks its identity against the index (SEC-MED-036) and passes a
   read-only descriptor over the worker's socket pair. The worker never
   opens a path (SEC-MED-020). The server and its workers never talk over
   TCP (SEC-STD-040).
4. **Bounded work per stream.** The packager charges the step budget for
   every frame it copies and stops with a typed error when the budget runs
   out. Each packaging worker has a memory cap, and the server's watchdog
   kills a worker that misses its deadline for a segment. WP-105 sets both
   values and registers them with the other limits; until they are
   measured, the memory cap is the worker default of 512 MiB (SEC-MED-021).
   A worker that panics, hangs or passes its cap ends that one stream with
   a typed error. Other streams keep serving, and the next request for the
   file gets a fresh worker (SEC-MED-081).
5. **Segments come back as untrusted input.** The worker returns the
   initialisation segment and each requested media segment as frames over
   the socket pair, each at most 32 MiB; the socket pair is the "pipe" of
   SEC-MED-081. The server decodes every frame with the core's limits and
   checks the segment's box structure, sample count and duration against
   the frame index stored at scan time before it serves a byte
   (SEC-MED-023).
6. **Written from the typed model only.** The packager writes only the
   boxes it needs, from typed values. It never copies an unknown box from
   the source and never writes a data reference to anything outside the
   segment, the rule SEC-MED-074 sets for the remuxer. Every segment it
   writes parses back to the frames that went in, and the writer is fuzzed
   from arbitrary typed models (SEC-MED-032).
7. **The isolation tier of memory-safe parsing, failing closed.** Packaging
   runs at the tier SEC-MED-024 gives memory-safe parsing: process
   separation, rlimits and `no_new_privs` at the floor, seccomp, Landlock
   and namespaces added wherever the host has them, and a "reduced
   isolation" notice when any of those is missing. If the floor itself is
   not met, packaging is off and the player falls back to the original
   file where the browser can play it. Nothing ever falls back to
   packaging in the server process.
8. **The server crate cannot call the packager.** The one production call
   to the packager's entry points is in
   `gunmetal-worker/src/jobs/package.rs` (WP-105). A check in the gate
   fails if any module of the server crate calls them (WP-001's dependency
   check, record 6). The core's own tests and fuzz harness still call
   them.
9. **Every remuxer runs in a worker.** The same rule binds the R2 video
   remuxer (SEC-MED-081). Record 1 decision 2 still holds: the remuxer's
   code lives in the core and is compiled into the one server binary. What
   changes is the process that runs it. Record 1 decision 3 kept only
   FFmpeg out of the server process, and the README diagram says "Remux
   in-process"; both are superseded on this point.
10. **CUE slices take the same path (R2).** The re-headed FLAC slices of
    MUS-041 are built in the same kind of worker, as a `CueSlice` job
    (WP-213), under every rule above. A cue sheet's FILE entry resolves
    only to an indexed file in the same folder; any other path is dropped
    and reported (SEC-MED-050).

## Security review record

Boundary TB6 (server to worker). Threats TM-T20 (a crafted file stalls the
server) and TM-T21 (memory corruption reaches the process that holds the
keys).

This record is the design review that WP-003 owes for the packager. It was
drafted by a coding agent on 2026-10-03 and becomes the review record when
a maintainer approves the pull request that adds it (AGENTS.md). The code
that proves each requirement is named beside it.

Verifies: SEC-MED-018, SEC-MED-024, SEC-MED-026

- **SEC-MED-018.** Packaging is parsing of media. Decisions 2, 7 and 8
  keep it out of the server process, and decision 4 limits a crash, hang
  or memory blow-up to one stream. WP-105's hostile-input tests prove it
  in code: a packaging worker that panics, one that loops past its
  watchdog and one that passes its memory cap each end one stream while a
  second stream keeps serving.
- **SEC-MED-024.** Decision 7 applies the table's row for memory-safe
  parsing. The packager is Gunmetal's own safe Rust and links no native
  code, so it never needs the full jail. WP-045's tier tests and WP-105's
  fallback test prove it in code.
- **SEC-MED-026.** The packager uses no third-party crate that parses or
  decodes media: it is core code under the full gate, with its own fuzz
  harness (WP-056). Adding such a crate to it needs the review SEC-MED-026
  requires and a new record.

SEC-MED-081, SEC-MED-032 and SEC-MED-074 are R2 in the baseline although
the packager ships in R1. Until the security lead moves them to R1 or adds
an R1 row (D-80), WP-056 and WP-105 verify them as if they were R1.

## Consequences

- Each packaged stream costs a worker round trip and one copy of each
  segment: a little latency and memory per stream.
- The worker protocol (WP-061) gains a `Package` job (WP-105) and, in R2, a
  `CueSlice` job (WP-213).
- If the browsers in the R1 test set accept a codec natively, WP-056 may
  carry fewer codecs. The rules here do not change.
- Records 1 and 2 are not edited (AGENTS.md); this record is the
  amendment. The root README's diagram still shows "Remux in-process" and
  needs a matching edit by its owner.
