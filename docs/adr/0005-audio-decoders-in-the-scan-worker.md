# 5. Audio decoders run only in the scan worker

Date: 2026-10-03
Status: accepted. On 2026-10-02 the owner answered D-09 by putting
parsing, remuxing and decoding in a separate jailed worker, and accepted
the register's remaining security recommendations as written, among them
D-09's: accept this record for the scan worker only, after review
([decision register](../decisions.md#owner-answers-2026-10-02)). This
record admits a kind of crate under conditions; each crate is admitted
only by its own review record. Measured loudness ships in R1.3 under the
owner's answer to D-10.

## Context

Loudness levelling (MUS-084 to MUS-089) uses ReplayGain and R128 tags where
a file has them. A file without them needs its loudness measured
(MUS-086): integrated loudness and true peak under ITU-R BS.1770, which
means decoding every sample. The core will hold the meter (WP-029), which
only does arithmetic on samples. It will not hold decoders: FLAC, MP3, AAC
and Vorbis decoders written by us would be a large amount of code under the
gate for a job that runs in the background. A third-party pure-Rust decoder
would do it, and the candidate is Symphonia.

Decoding is parsing at its densest, and decoders allocate output buffers.
The baseline admits a third-party crate that decodes untrusted media only
after a recorded review showing that it forbids `unsafe` (or confines it to
reviewed SIMD or FFI code), that it is fuzzed, and that it exposes resource
limits Gunmetal configures, and only behind a Gunmetal wrapper that is
itself fuzzed (SEC-MED-026). Loudness analysis runs in a worker
(SEC-MED-018), at the tier SEC-MED-024 sets for memory-safe parsing. Every
parser of containers and tags lives in the core (SEC-HIS-036), and the
server process never decodes untrusted audio (SEC-TM-034). R1 servers are
Linux only (D-09).

Facts about Symphonia, checked on 2026-10-03 against its README, docs.rs
and the published crate sources:

- Licence MPL-2.0, which is on the project's licence allow-list in
  `deny.toml` (SEC-SUP-029).
- Latest release 0.6.1, published on 2026-08-13, with a minimum Rust
  version of 1.85, the same as the workspace's.
- Its README lists "100% safe Rust", "Be fuzz-tested" and "Prevent
  denial-of-service attacks" among its aims. `symphonia-core` and
  `symphonia-bundle-flac` 0.6.1 both declare `#![forbid(unsafe_code)]`.
- Its codec table rates FLAC, MP3 and Vorbis decoding "Excellent" and
  AAC-LC "Great", and lists Opus and HE-AAC as in work.

Not checked: whether its repository or OSS-Fuzz fuzzes it today (the README
states it as an aim); whether its other crates, and the crates it depends
on, use `unsafe`; which resource limits 0.6 exposes; and whether its
decoders can be fed packets from another demuxer through the 0.6 public
API, as its design suggests.

## Decisions

1. **A reviewed pure-Rust decoder may decode untrusted audio for analysis
   only.** Its output feeds analysis jobs: loudness and true peak first,
   and later any other analysis that needs decoded audio, such as sonic
   similarity (DIS-075), under the same rules. It never feeds playback or
   transcoding (R1 has no transcoding), and decoded audio never leaves the
   worker.
2. **Only in the scan worker.** The decoder is a dependency of
   `gunmetal-worker` alone and is called from one module, the loudness job
   (`gunmetal-worker/src/jobs/loudness.rs`, WP-114), which runs only in a
   confined worker from the scan pool. When the crate lands, `clippy.toml`
   bans its entry points everywhere else, the server crate included
   ([record 6](0006-workspace-and-dependencies.md)). It never joins the
   core's dependency allowlist (SEC-SUP-025) and never runs in the server
   process (SEC-MED-018, SEC-TM-034). The worker is the server's own
   binary, so the decoder's code ships in that binary, but only a worker
   that has already confined itself can reach it.
3. **The core demuxes; the decoder only decodes.** The worker reads the
   file with the core's own container parsers, under the core's limits and
   step budget: FLAC frames, MPEG audio frames, MP4 sample tables and Ogg
   packets. It hands the decoder one packet at a time, with codec
   parameters the core has already validated (sample rate, channel count,
   block size). The decoder crate's container readers and metadata parsers
   are never called, so no third-party code parses a container or a tag
   (SEC-HIS-036). A crate whose decoders cannot be driven this way is not
   admitted. PCM in WAV and AIFF needs no decoder: the worker reads its
   samples directly.
4. **Gunmetal's limits around the decoder.** The wrapper sets every limit
   the crate exposes. Around it, Gunmetal enforces its own: the core's
   limits bound each packet before the decoder sees it; the decoded
   duration is counted, and a file longer than 12 hours keeps its tags
   only; the job runs under the worker's 512 MiB memory limit, a CPU-time
   limit and the 300 s per-file deadline (SEC-MED-021 and the limits table
   in media-and-parser-safety.md). A decoder error, a panic or a limit
   breach ends that file's measurement with a recorded reason. The file
   keeps playing with its tags or the fallback gain (SEC-MED-017), and two
   crashes or timeouts in a row quarantine it (SEC-MED-019).
5. **Two numbers come back.** The job returns integrated loudness and true
   peak, or a typed reason why it could not. The server revalidates both
   as finite values within their documented ranges before storing them in
   the derived-data store (SEC-MED-023, SEC-MED-014).
6. **The isolation tier of memory-safe parsing.** The decoder runs at the
   tier SEC-MED-024 gives memory-safe parsing: process separation, rlimits
   and `no_new_privs` at the floor, seccomp, Landlock and namespaces
   wherever the host has them, and a "reduced isolation" notice when any of
   those is missing. R1 servers run Linux on x86-64 and AArch64, both of
   which `seccompiler` supports, so the normal case is the whole Linux
   sandbox of SEC-MED-022. If the floor is not met, measurement is off and
   says why. A native decoder (C or C++) is never admitted under this
   record: it would need the full jail of SEC-TM-044 and its own record,
   and SEC-MED-025 keeps such libraries out of the binary.
7. **What each crate's review must show.** The pull request that adds a
   decoder crate (WP-114 for the first) carries the review record that
   SEC-MED-026 requires, and the crate arrives through the integrator's
   dependency request (record 6). The review shows:
   - **`unsafe`.** Each crate the build enables forbids `unsafe`, or
     confines it to SIMD or FFI code the reviewer read. The crates it
     depends on are listed with the same check.
   - **Scope.** Default features are off and only the codecs Gunmetal
     needs are enabled. None of the crate's container readers is called.
   - **Fuzzing.** Evidence that the crate is fuzzed where it is developed,
     and a harness for Gunmetal's wrapper in `crates/gunmetal-fuzz`, with
     committed seeds that the gate replays (SEC-MED-027, SEC-MED-028).
   - **Limits.** Every limit the crate exposes and the value Gunmetal
     sets. Where the crate exposes no limit that bounds what one call can
     allocate, the review shows how the wrapper bounds it from validated
     codec parameters before the call. If neither holds, the crate is not
     admitted.
   - **Supply chain.** A licence on the allow-list, the version pinned
     exactly in `Cargo.lock`, cargo-deny and a cargo-vet audit passing
     (SEC-SUP-024), and the seven-day age rule met.
8. **Symphonia is the first candidate, for FLAC, MP3, AAC-LC and Vorbis.**
   Opus and HE-AAC have no admitted decoder. Those files use their tags
   (for Opus, the header gain and R128 tags, MUS-085) or the fallback gain
   (MUS-089). A reviewed decoder for them comes in under decision 7
   without a new record.
9. **When.** Measured loudness (MUS-086) ships in R1.3 under the owner's
   answer to D-10. Until then R1 uses tags and the fallback gain. WP-029
   builds the meter in the core and WP-114 builds the job.

## Security review record

Boundary TB6 (server to worker). Threats TM-T21 (memory corruption in a
decoder), TM-T41 (a malicious or compromised dependency) and TM-T20 (a
crafted file stalls the scan).

This record is the design review that WP-003 owes for audio decoding. It
was drafted by a coding agent on 2026-10-03 and becomes the review record
when a maintainer approves the pull request that adds it (AGENTS.md).

Verifies: SEC-MED-018, SEC-MED-024, SEC-MED-026

- **SEC-MED-018.** Decisions 2 and 5 keep decoding in the scan worker and
  send back only two revalidated numbers; decision 4 limits a failure to
  one file's measurement. WP-114 proves it in code with the pool's
  crash and hang hooks (WP-078).
- **SEC-MED-024.** Decision 6 applies the table's row for memory-safe
  parsing and rules native decoders out of this path. WP-045's tier tests
  prove the tier.
- **SEC-MED-026.** Decision 7 sets out the review. This record is its
  architecture half; the crate's own review is due in the pull request
  that adds the crate, and no crate meets SEC-MED-026 until then.

## Consequences

- Untagged FLAC, MP3, AAC-LC and Vorbis files get measured loudness from
  R1.3. Untagged Opus and HE-AAC files keep the fallback gain until a
  decoder for them passes review.
- One more third-party crate ships in the binary, with its review, its
  cargo-vet audits and its updates to maintain.
- WP-114's Security field lists only SEC-MED-018. Under this record it
  also carries SEC-MED-026 (the review record and the wrapper's harness)
  and SEC-MED-021; the plan is not edited here.
- If the review rejects Symphonia, MUS-086 waits for another candidate and
  R1.x keeps tags and the fallback gain. Nothing else changes.
