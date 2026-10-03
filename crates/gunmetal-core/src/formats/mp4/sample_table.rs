//! MP4 sample tables, joined into a seek index (ISO/IEC 14496-12, sections
//! 8.6.1.2 and 8.7.3 to 8.7.5).
//!
//! An MP4 file says when each sample of a track plays and where it lies in
//! four tables. `stts` gives every sample's decoding time delta, in runs;
//! `stsc` gives how many samples each chunk holds, in runs; `stsz` or `stz2`
//! gives every sample's size; and `stco` or `co64` gives where every chunk
//! starts. [`seek_index`] reads the bodies of those boxes, which the box
//! walker finds (WP-017), and joins them into [`SeekPoint`]s of play time
//! and file offset, so a player asks for the right byte range first time
//! (MUS-071).
//!
//! The tables are hostile input. Every declared entry count is multiplied
//! by its entry size with `checked_mul` and checked against the octets the
//! body holds before any entry is read; Stagefright's CVE-2015-1538 was this
//! multiplication left unchecked. Nothing is allocated from a declared
//! count: entries are read in place from the borrowed bodies, and the index
//! grows one point at a time, never past the index-entry limit
//! (SEC-MED-003, SEC-TM-032). Every chunk must lie inside the file and start
//! at or after the end of the chunk before it (SEC-MED-008).
//!
//! # Steps
//!
//! A parse charges one step for each `stts` entry, `stsc` entry, chunk,
//! listed sample size and candidate point (SEC-MED-007). A listed size takes
//! at least four bits and every other entry at least four octets, and there
//! are never more candidate points than the index-entry limit, so a parse
//! takes at most [`STEPS_PER_OCTET`] steps for each octet of the four bodies
//! plus that limit. A budget of
//! `Budget::for_input(octets, STEPS_PER_OCTET, limits.get(LimitKind::IndexEntries))`
//! never runs out.

use core::num::NonZeroU32;
use core::slice::ChunksExact;

use crate::parse::{Budget, Cursor, LimitKind, Limits, ParseFault};
use crate::problem::{Arg, Describe, Problem, ProblemCode};
use crate::values::Duration;

/// Steps a parse may charge for each octet of the four bodies, on top of one
/// step for each index entry the limits allow.
pub const STEPS_PER_OCTET: u64 = 2;

/// Which box a table came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Table {
    /// `TimeToSampleBox`.
    Stts,
    /// `SampleToChunkBox`.
    Stsc,
    /// `SampleSizeBox`.
    Stsz,
    /// `CompactSampleSizeBox`.
    Stz2,
    /// `ChunkOffsetBox`.
    Stco,
    /// `ChunkLargeOffsetBox`.
    Co64,
}

/// The body of the box that holds the sample sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleSizes<'a> {
    /// A `SampleSizeBox`: one size for every sample, or a 32-bit size each.
    Stsz(Cursor<'a>),
    /// A `CompactSampleSizeBox`: a 4, 8 or 16-bit size each.
    Stz2(Cursor<'a>),
}

/// The body of the box that holds the chunk offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkOffsets<'a> {
    /// A `ChunkOffsetBox`: 32-bit offsets.
    Stco(Cursor<'a>),
    /// A `ChunkLargeOffsetBox`: 64-bit offsets.
    Co64(Cursor<'a>),
}

/// The sample table bodies of one track.
///
/// A body is everything in its box after the size and type, starting with
/// the full-box version and flags, as a cursor at the file offset where it
/// starts, so every error names an absolute offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleTableBodies<'a> {
    /// The `stts` body.
    pub stts: Cursor<'a>,
    /// The `stsc` body.
    pub stsc: Cursor<'a>,
    /// The `stsz` or `stz2` body.
    pub sizes: SampleSizes<'a>,
    /// The `stco` or `co64` body.
    pub chunks: ChunkOffsets<'a>,
    /// The length of the whole file, which every chunk must lie inside.
    pub file_len: u64,
}

/// A place to start playing from: the sample that starts at `offset` in the
/// file plays at `time`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeekPoint {
    /// When the sample plays, from the start of the track, rounded down to
    /// the millisecond.
    pub time: Duration,
    /// Where the sample starts, in octets from the start of the file.
    pub offset: u64,
}

/// A track's seek index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeekIndex {
    /// Points in decoding order, each later in time and further into the
    /// file than the one before it. A candidate that is neither, such as a
    /// sample of no octets or one that plays in the same millisecond as the
    /// point before it, is left out, because the point before it already
    /// starts there.
    pub points: Vec<SeekPoint>,
    /// How many samples apart the candidate points are: 1 when every sample
    /// is a candidate, more when the track has more samples than the
    /// index-entry limit allows, which thins the index (SEC-MED-006).
    pub stride: u64,
    /// How long the track's samples play together: the sum of their time
    /// deltas.
    pub duration: Duration,
}

/// Why the sample tables could not be joined into a seek index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleTableError {
    /// A body ended early, or the parse spent its whole step budget.
    Fault(ParseFault),
    /// A body declared a version other than 0, the only one defined.
    Version {
        /// Which box it was.
        table: Table,
        /// The version it declared.
        version: u8,
        /// Where the version is.
        offset: u64,
    },
    /// An `stz2` body declared a field size other than 4, 8 or 16 bits.
    FieldSize {
        /// The field size it declared, in bits.
        bits: u8,
        /// Where the field size is.
        offset: u64,
    },
    /// An `stsc` run started at chunk 0, at or before the chunk where the
    /// run before it started, or after the last chunk.
    ChunkRun {
        /// Which run it was, counting from 0.
        run: u32,
        /// The chunk it said it started at, counting from 1.
        first_chunk: u32,
        /// How many chunks the track has.
        chunks: u32,
        /// Where the run's entry is.
        offset: u64,
    },
    /// The tables disagree on how many samples the track has.
    SampleCounts {
        /// The samples `stts` gives a time to.
        time_to_sample: u64,
        /// The samples `stsz` or `stz2` gives a size to.
        sample_sizes: u32,
        /// The samples `stsc` places in chunks.
        sample_to_chunk: u64,
        /// Where `stsz` or `stz2` declares its sample count.
        offset: u64,
    },
    /// The samples together play for longer than any recording does (30
    /// days).
    Duration {
        /// The sum of the time deltas.
        ticks: u64,
        /// Ticks per second.
        timescale: NonZeroU32,
        /// Where the `stts` body starts.
        offset: u64,
    },
    /// A chunk starts before the chunk before it ends.
    ChunkOrder {
        /// Which chunk it was, counting from 1.
        chunk: u32,
        /// Where the chunk's offset entry is.
        offset: u64,
        /// Where the chunk starts.
        start: u64,
        /// Where the chunk before it ends.
        previous_end: u64,
    },
    /// A chunk's samples run past the end of the file.
    ChunkPastEnd {
        /// Which chunk it was, counting from 1.
        chunk: u32,
        /// Where the chunk's offset entry is.
        offset: u64,
        /// Where the chunk starts.
        start: u64,
        /// How many octets its samples take.
        len: u64,
        /// The length of the file.
        file_len: u64,
    },
}

impl From<ParseFault> for SampleTableError {
    fn from(fault: ParseFault) -> Self {
        Self::Fault(fault)
    }
}

impl SampleTableError {
    /// Where the parser was working when it stopped, in octets from the
    /// start of the file.
    const fn offset(&self) -> u64 {
        match *self {
            Self::Fault(fault) => fault.offset(),
            Self::Version { offset, .. }
            | Self::FieldSize { offset, .. }
            | Self::ChunkRun { offset, .. }
            | Self::SampleCounts { offset, .. }
            | Self::Duration { offset, .. }
            | Self::ChunkOrder { offset, .. }
            | Self::ChunkPastEnd { offset, .. } => offset,
        }
    }
}

impl Describe for SampleTableError {
    fn problem(&self) -> Problem {
        Problem {
            code: ProblemCode::SampleTableDamaged,
            args: vec![("offset", Arg::Number(self.offset()))],
        }
    }
}

/// Joins one track's sample tables into its seek index, with times
/// measured in `timescale` ticks a second, thinned to the index-entry limit
/// in `limits`, and charging `budget` as the module documentation sets out.
///
/// # Errors
///
/// Returns the [`SampleTableError`] for the first problem found. The four
/// bodies are read in the order of [`SampleTableBodies`]' fields; then the
/// `stsc` runs are checked against the chunks, the sample counts against
/// each other and the duration against its range; then the chunks are
/// walked in order.
pub fn seek_index(
    tables: SampleTableBodies<'_>,
    timescale: NonZeroU32,
    limits: &Limits,
    budget: &mut Budget,
) -> Result<SeekIndex, SampleTableError> {
    let stts = table(tables.stts, Table::Stts, 64)?;
    let stsc = table(tables.stsc, Table::Stsc, 96)?;
    let (samples, counted_at, sizes) = sample_sizes(tables.sizes)?;
    let (chunks, width) = match tables.chunks {
        ChunkOffsets::Stco(body) => (table(body, Table::Stco, 32)?, 4),
        ChunkOffsets::Co64(body) => (table(body, Table::Co64, 64)?, 8),
    };
    let (timed, ticks) = time_totals(stts, budget)?;
    let placed = placed_samples(stsc, chunks.count, budget)?;
    if timed != u64::from(samples) || placed != u64::from(samples) {
        return Err(SampleTableError::SampleCounts {
            time_to_sample: timed,
            sample_sizes: samples,
            sample_to_chunk: placed,
            offset: counted_at,
        });
    }
    let duration =
        Duration::from_ticks(ticks, timescale).map_err(|_| SampleTableError::Duration {
            ticks,
            timescale,
            offset: tables.stts.offset(),
        })?;
    let limit = limits.get(LimitKind::IndexEntries);
    let stride = u64::from(samples).div_ceil(limit.max(1)).max(1);
    let mut walk = Walk {
        clock: Clock::new(stts.bytes),
        sizes,
        timescale,
        duration,
        stride,
        limit,
        candidate: 0,
        taken: 0,
        points: Vec::new(),
    };
    walk.chunks(chunks, width, stsc.bytes, tables.file_len, budget)?;
    Ok(SeekIndex {
        points: walk.points,
        stride,
        duration,
    })
}

/// The entries of one table.
#[derive(Debug, Clone, Copy)]
struct Entries<'a> {
    /// How many entries the table declared.
    count: u32,
    /// The entries, which are exactly as long as the count declares.
    bytes: &'a [u8],
    /// Where the first entry starts.
    offset: u64,
}

/// Reads a full-box header: version 0, then flags, which say nothing for
/// these boxes.
fn open(body: &mut Cursor<'_>, table: Table) -> Result<(), SampleTableError> {
    let offset = body.offset();
    let version = body.u8()?;
    if version != 0 {
        return Err(SampleTableError::Version {
            table,
            version,
            offset,
        });
    }
    body.skip(3)?;
    Ok(())
}

/// Takes `count` entries of `bits` bits each from `body`, checking the
/// product against the octets the body holds before reading any entry.
fn take_entries<'a>(body: &mut Cursor<'a>, count: u32, bits: u64) -> Result<&'a [u8], ParseFault> {
    // A product past u64::MAX is past the end of any body.
    let octets = u64::from(count)
        .checked_mul(bits)
        .map_or(u64::MAX, |bits| bits.div_ceil(8));
    body.take(octets)
}

/// Reads a table whose body holds a header, a 32-bit entry count and
/// entries of `bits` bits each.
fn table(mut body: Cursor<'_>, table: Table, bits: u64) -> Result<Entries<'_>, SampleTableError> {
    open(&mut body, table)?;
    let count = body.u32_be()?;
    let offset = body.offset();
    let bytes = take_entries(&mut body, count, bits)?;
    Ok(Entries {
        count,
        bytes,
        offset,
    })
}

/// Reads the sample size body: how many samples it sizes, where it says so,
/// and the sizes, in order.
fn sample_sizes(sizes: SampleSizes<'_>) -> Result<(u32, u64, Sizes<'_>), SampleTableError> {
    match sizes {
        SampleSizes::Stsz(mut body) => {
            open(&mut body, Table::Stsz)?;
            let size = body.u32_be()?;
            let counted_at = body.offset();
            let count = body.u32_be()?;
            let sizes = if size == 0 {
                listed(take_entries(&mut body, count, 32)?, 32)
            } else {
                Sizes::Constant(u64::from(size))
            };
            Ok((count, counted_at, sizes))
        }
        SampleSizes::Stz2(mut body) => {
            open(&mut body, Table::Stz2)?;
            body.skip(3)?;
            let offset = body.offset();
            let bits = body.u8()?;
            if !matches!(bits, 4 | 8 | 16) {
                return Err(SampleTableError::FieldSize { bits, offset });
            }
            let counted_at = body.offset();
            let count = body.u32_be()?;
            let fields = take_entries(&mut body, count, u64::from(bits))?;
            Ok((count, counted_at, listed(fields, bits)))
        }
    }
}

/// Every sample's size, in order.
enum Sizes<'a> {
    /// Every sample has this many octets.
    Constant(u64),
    /// Each sample's size in turn.
    Listed(Box<dyn Iterator<Item = u64> + 'a>),
}

/// The sizes in `fields`, each `bits` bits wide: 4, 8, 16 or 32. Fields of
/// 4 bits are packed two to an octet, the first in the high half.
fn listed(fields: &[u8], bits: u8) -> Sizes<'_> {
    if bits == 4 {
        Sizes::Listed(Box::new(
            fields
                .iter()
                .flat_map(|&octet| [octet >> 4, octet & 0x0F])
                .map(u64::from),
        ))
    } else {
        Sizes::Listed(Box::new(
            fields
                .chunks_exact(usize::from(bits / 8))
                .map(|field| u64::from_be_bytes(right_aligned(field))),
        ))
    }
}

impl Sizes<'_> {
    /// The octets the next `samples` samples take together, charging a
    /// step at `offset` for each listed size.
    fn span(&mut self, samples: u64, budget: &mut Budget, offset: u64) -> Result<u64, ParseFault> {
        match self {
            Self::Constant(size) => Ok(samples.saturating_mul(*size)),
            Self::Listed(sizes) => {
                let mut octets: u64 = 0;
                // The sample counts were checked, so the sizes never run out.
                for size in sizes
                    .by_ref()
                    .take(usize::try_from(samples).unwrap_or(usize::MAX))
                {
                    budget.charge(1, offset)?;
                    octets = octets.saturating_add(size);
                }
                Ok(octets)
            }
        }
    }
}

/// `octets`, of which there are at most `N`, right-aligned in `N` zero
/// octets, so that reading the result big-endian gives the integer they
/// encode.
fn right_aligned<const N: usize>(octets: &[u8]) -> [u8; N] {
    let mut padded = [0; N];
    for (slot, &octet) in padded.iter_mut().rev().zip(octets.iter().rev()) {
        *slot = octet;
    }
    padded
}

/// Reads the next 32-bit field of an entry and moves past it. Entries come
/// from [`take_entries`], which checked their length, so a field is never
/// missing.
fn field(entry: &mut &[u8]) -> u32 {
    let (head, rest) = entry.split_first_chunk::<4>().unwrap_or((&[0; 4], &[]));
    *entry = rest;
    u32::from_be_bytes(*head)
}

/// Adds up the `stts` runs: how many samples they time and how many ticks
/// those samples play for, charging a step for each run.
fn time_totals(runs: Entries<'_>, budget: &mut Budget) -> Result<(u64, u64), ParseFault> {
    let mut samples: u64 = 0;
    let mut ticks: u64 = 0;
    let mut offset = runs.offset;
    for mut entry in runs.bytes.chunks_exact(8) {
        budget.charge(1, offset)?;
        let count = u64::from(field(&mut entry));
        let delta = u64::from(field(&mut entry));
        samples = samples.saturating_add(count);
        ticks = ticks.saturating_add(count.saturating_mul(delta));
        offset = offset.saturating_add(8);
    }
    Ok((samples, ticks))
}

/// Checks the `stsc` runs against a track of `chunks` chunks and counts the
/// samples they place, charging a step for each run.
///
/// Each run must start after the run before it, the first at chunk 1 or
/// later, and none after the last chunk. Chunks before the first run hold
/// no samples.
fn placed_samples(
    runs: Entries<'_>,
    chunks: u32,
    budget: &mut Budget,
) -> Result<u64, SampleTableError> {
    let mut samples: u64 = 0;
    // The first chunk and the samples per chunk of the run before.
    let mut previous: Option<(u32, u32)> = None;
    let mut run: u32 = 0;
    let mut offset = runs.offset;
    for mut entry in runs.bytes.chunks_exact(12) {
        budget.charge(1, offset)?;
        let first_chunk = field(&mut entry);
        let per_chunk = field(&mut entry);
        let lowest = previous.map_or(1, |(first, _)| u64::from(first).saturating_add(1));
        if u64::from(first_chunk) < lowest || first_chunk > chunks {
            return Err(SampleTableError::ChunkRun {
                run,
                first_chunk,
                chunks,
                offset,
            });
        }
        if let Some((first, per)) = previous {
            samples = samples.saturating_add(placed(first, first_chunk, per));
        }
        previous = Some((first_chunk, per_chunk));
        run = run.saturating_add(1);
        offset = offset.saturating_add(12);
    }
    if let Some((first, per)) = previous {
        samples = samples.saturating_add(placed(first, chunks.saturating_add(1), per));
    }
    Ok(samples)
}

/// The samples a run of `per_chunk` samples a chunk places in the chunks
/// from `first` up to, but not including, `end`.
fn placed(first: u32, end: u32, per_chunk: u32) -> u64 {
    u64::from(end.saturating_sub(first)).saturating_mul(u64::from(per_chunk))
}

/// Decoding times from the `stts` runs, for samples asked for in increasing
/// order.
struct Clock<'a> {
    /// The runs not reached yet.
    runs: ChunksExact<'a, u8>,
    /// The first sample of the current run.
    first: u64,
    /// The first sample after the current run.
    end: u64,
    /// The time delta of each sample in the current run.
    delta: u64,
    /// The decoding time of `first`, in ticks.
    ticks: u64,
}

impl<'a> Clock<'a> {
    /// A clock before the first of the `stts` entries in `runs`.
    fn new(runs: &'a [u8]) -> Self {
        Self {
            runs: runs.chunks_exact(8),
            first: 0,
            end: 0,
            delta: 0,
            ticks: 0,
        }
    }

    /// The decoding time of `sample`, in ticks. The sample counts were
    /// checked, so the runs never end before the sample.
    fn ticks(&mut self, sample: u64) -> u64 {
        while sample >= self.end
            && let Some(mut entry) = self.runs.next()
        {
            let elapsed = self.end.saturating_sub(self.first);
            self.ticks = self
                .ticks
                .saturating_add(elapsed.saturating_mul(self.delta));
            self.first = self.end;
            self.end = self.end.saturating_add(u64::from(field(&mut entry)));
            self.delta = u64::from(field(&mut entry));
        }
        let into_run = sample.saturating_sub(self.first);
        self.ticks
            .saturating_add(into_run.saturating_mul(self.delta))
    }
}

/// The walk over a track's chunks that builds its seek index.
struct Walk<'a> {
    /// Decoding times.
    clock: Clock<'a>,
    /// Sample sizes, from the first sample not yet passed.
    sizes: Sizes<'a>,
    /// Ticks per second.
    timescale: NonZeroU32,
    /// How long the track plays, which no sample's time exceeds.
    duration: Duration,
    /// Samples between candidate points.
    stride: u64,
    /// The most candidate points the index may hold.
    limit: u64,
    /// The next candidate sample.
    candidate: u64,
    /// Candidates taken so far.
    taken: u64,
    /// The index so far.
    points: Vec<SeekPoint>,
}

impl Walk<'_> {
    /// Walks every chunk in `chunks`, whose offsets are `width` octets wide,
    /// placing samples in them by the `stsc` entries in `runs`, which were
    /// checked, and checking each against the file's length.
    fn chunks(
        &mut self,
        chunks: Entries<'_>,
        width: u8,
        runs: &[u8],
        file_len: u64,
        budget: &mut Budget,
    ) -> Result<(), SampleTableError> {
        let mut runs = runs.chunks_exact(12);
        let mut next_run = runs.next();
        let mut per_chunk: u64 = 0;
        let mut first_sample: u64 = 0;
        let mut previous_end: u64 = 0;
        let mut chunk: u32 = 0;
        let mut offset = chunks.offset;
        for entry in chunks.bytes.chunks_exact(usize::from(width)) {
            chunk = chunk.saturating_add(1);
            budget.charge(1, offset)?;
            if let Some(mut run) = next_run
                && field(&mut run) == chunk
            {
                per_chunk = u64::from(field(&mut run));
                next_run = runs.next();
            }
            let start = u64::from_be_bytes(right_aligned(entry));
            if start < previous_end {
                return Err(SampleTableError::ChunkOrder {
                    chunk,
                    offset,
                    start,
                    previous_end,
                });
            }
            let samples = per_chunk;
            let len = self.chunk(first_sample, samples, start, budget, offset)?;
            let Some(end) = start.checked_add(len).filter(|&end| end <= file_len) else {
                return Err(SampleTableError::ChunkPastEnd {
                    chunk,
                    offset,
                    start,
                    len,
                    file_len,
                });
            };
            previous_end = end;
            first_sample = first_sample.saturating_add(samples);
            offset = offset.saturating_add(u64::from(width));
        }
        Ok(())
    }

    /// Indexes the chunk that starts at `start` and holds `samples` samples
    /// from sample `first` on, charging steps at `offset`, and returns how
    /// many octets the chunk's samples take.
    fn chunk(
        &mut self,
        first: u64,
        samples: u64,
        start: u64,
        budget: &mut Budget,
        offset: u64,
    ) -> Result<u64, ParseFault> {
        let end = first.saturating_add(samples);
        let mut position = first;
        let mut len: u64 = 0;
        while self.candidate < end && self.taken < self.limit {
            budget.charge(1, offset)?;
            let before = self.candidate.saturating_sub(position);
            len = len.saturating_add(self.sizes.span(before, budget, offset)?);
            position = self.candidate;
            let ticks = self.clock.ticks(self.candidate);
            // The duration was checked, and no sample plays after it.
            let time = Duration::from_ticks(ticks, self.timescale).unwrap_or(self.duration);
            let at = start.saturating_add(len);
            if self
                .points
                .last()
                .is_none_or(|last| time > last.time && at > last.offset)
            {
                self.points.push(SeekPoint { time, offset: at });
            }
            self.candidate = self.candidate.saturating_add(self.stride);
            self.taken = self.taken.saturating_add(1);
        }
        let rest = end.saturating_sub(position);
        Ok(len.saturating_add(self.sizes.span(rest, budget, offset)?))
    }
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::*;
    use gunmetal_testkit::bytes::Bytes;
    use gunmetal_testkit::mp4_samples::{
        chunk_runs, co64, stco, stsc, stsz, stsz_constant, stts, stz2, time_runs,
    };
    use proptest::collection::vec;
    use proptest::prelude::*;

    /// The stack size SEC-MED-001 names, in octets.
    const STACK: usize = 262_144;

    /// Runs `work` on a fresh thread with a 256 KiB stack, so a parse that
    /// recursed without a bound would fail this test instead of passing on
    /// the runner's larger stack.
    fn on_small_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(STACK)
            .spawn(work)
            .expect("the test thread starts")
            .join()
            .expect("the code under test returned instead of panicking")
    }

    /// Where each test body starts in its pretend file.
    const STTS_AT: u64 = 0x100;
    const STSC_AT: u64 = 0x200;
    const SIZES_AT: u64 = 0x300;
    const CHUNKS_AT: u64 = 0x400;

    /// Which box a test track's sizes or chunk offsets are written in.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Kind {
        Stsz,
        Stz2,
        Stco,
        Co64,
    }

    /// The owned bodies of a test track's sample tables.
    #[derive(Debug, Clone)]
    struct Track {
        stts: Vec<u8>,
        stsc: Vec<u8>,
        sizes: (Kind, Vec<u8>),
        chunks: (Kind, Vec<u8>),
        file_len: u64,
    }

    impl Track {
        /// The bodies, each at its own offset.
        fn bodies(&self) -> SampleTableBodies<'_> {
            let sizes = Cursor::at(&self.sizes.1, SIZES_AT);
            let chunks = Cursor::at(&self.chunks.1, CHUNKS_AT);
            SampleTableBodies {
                stts: Cursor::at(&self.stts, STTS_AT),
                stsc: Cursor::at(&self.stsc, STSC_AT),
                sizes: if self.sizes.0 == Kind::Stz2 {
                    SampleSizes::Stz2(sizes)
                } else {
                    SampleSizes::Stsz(sizes)
                },
                chunks: if self.chunks.0 == Kind::Co64 {
                    ChunkOffsets::Co64(chunks)
                } else {
                    ChunkOffsets::Stco(chunks)
                },
                file_len: self.file_len,
            }
        }

        /// The octets of the four bodies together.
        fn octets(&self) -> u64 {
            [&self.stts, &self.stsc, &self.sizes.1, &self.chunks.1]
                .iter()
                .map(|body| u64::try_from(body.len()).expect("a test body is small"))
                .sum()
        }

        /// Joins the tables under `limits` with a budget the size the
        /// module documents, after checking that a parse with no limit on
        /// its steps gives the same answer within that many steps.
        fn index_with(
            &self,
            timescale: u32,
            limits: &Limits,
        ) -> Result<SeekIndex, SampleTableError> {
            let timescale = NonZeroU32::new(timescale).expect("a test timescale is not zero");
            let entries = limits.get(LimitKind::IndexEntries);
            let mut unlimited = Budget::for_input(0, 0, u64::MAX);
            let free = seek_index(self.bodies(), timescale, limits, &mut unlimited);
            let spent = u64::MAX - unlimited.remaining();
            let bound = STEPS_PER_OCTET * self.octets() + entries;
            assert!(spent <= bound, "spent {spent} steps, more than {bound}");
            let mut documented = Budget::for_input(self.octets(), STEPS_PER_OCTET, entries);
            let bounded = seek_index(self.bodies(), timescale, limits, &mut documented);
            assert_eq!(bounded, free, "the documented budget changed the answer");
            bounded
        }

        /// Joins the tables under the default limits.
        fn index(&self, timescale: u32) -> Result<SeekIndex, SampleTableError> {
            self.index_with(timescale, &Limits::DEFAULT)
        }
    }

    /// The default limits with room for `entries` index entries.
    fn limited(entries: u64) -> Limits {
        Limits::DEFAULT
            .with_override(LimitKind::IndexEntries, entries)
            .expect("the limit is below its ceiling")
    }

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis).expect("a test duration is in range")
    }

    fn point(millis: u64, offset: u64) -> SeekPoint {
        SeekPoint {
            time: ms(millis),
            offset,
        }
    }

    fn truncated(offset: u64, needed: u64, available: u64) -> SampleTableError {
        SampleTableError::Fault(ParseFault::Truncated {
            offset,
            needed,
            available,
        })
    }

    /// A body of version 0 with no flags, then `rest`.
    fn full_box(rest: &[u8]) -> Vec<u8> {
        [&[0x00, 0x00, 0x00, 0x00][..], rest].concat()
    }

    /// Three samples of 10, 20 and 30 octets, 20 ms apart, in two chunks
    /// back to back, the second ending exactly at the end of the file.
    fn minimal() -> Track {
        Track {
            stts: stts(&[(3, 20)]),
            stsc: stsc(&[(1, 2, 1), (2, 1, 1)]),
            sizes: (Kind::Stsz, stsz(&[10, 20, 30])),
            chunks: (Kind::Stco, stco(&[100, 130])),
            file_len: 160,
        }
    }

    /// Verifies: SEC-MED-001, SEC-MED-008, SEC-HIS-036
    #[test]
    fn indexes_a_minimal_track_built_by_the_testkit() {
        assert_eq!(
            minimal().index(1_000),
            Ok(SeekIndex {
                points: vec![point(0, 100), point(20, 110), point(40, 130)],
                stride: 1,
                duration: ms(60),
            })
        );
    }

    #[test]
    fn places_samples_of_one_constant_size() {
        // 1,024 ticks at 44,100 a second is 23.2 ms; times round down.
        let track = Track {
            stts: stts(&[(4, 1_024)]),
            stsc: stsc(&[(1, 2, 1)]),
            sizes: (Kind::Stsz, stsz_constant(25, 4)),
            chunks: (Kind::Stco, stco(&[1_000, 1_100])),
            file_len: 1_150,
        };
        assert_eq!(
            track.index(44_100),
            Ok(SeekIndex {
                points: vec![
                    point(0, 1_000),
                    point(23, 1_025),
                    point(46, 1_100),
                    point(69, 1_125),
                ],
                stride: 1,
                duration: ms(92),
            })
        );
    }

    #[test]
    fn reads_compact_sizes_of_every_field_width_as_listed_ones() {
        let expected = Ok(SeekIndex {
            points: vec![point(0, 100), point(20, 101), point(40, 116)],
            stride: 1,
            duration: ms(60),
        });
        for sizes in [
            (Kind::Stsz, stsz(&[1, 15, 7])),
            (Kind::Stz2, stz2(4, &[1, 15, 7])),
            (Kind::Stz2, stz2(8, &[1, 15, 7])),
            (Kind::Stz2, stz2(16, &[1, 15, 7])),
        ] {
            let track = Track {
                stts: stts(&[(3, 20)]),
                stsc: stsc(&[(1, 3, 1)]),
                sizes: sizes.clone(),
                chunks: (Kind::Stco, stco(&[100])),
                file_len: 123,
            };
            assert_eq!(track.index(1_000), expected, "{sizes:02X?}");
        }
    }

    #[test]
    fn reads_64_bit_chunk_offsets_past_4_gib() {
        let track = Track {
            stts: stts(&[(2, 1)]),
            stsc: stsc(&[(1, 1, 1)]),
            sizes: (Kind::Stsz, stsz(&[16, 16])),
            chunks: (Kind::Co64, co64(&[0x1_0000_0000, 0x1_0000_0010])),
            file_len: 0x1_0000_0020,
        };
        assert_eq!(
            track.index(1),
            Ok(SeekIndex {
                points: vec![point(0, 0x1_0000_0000), point(1_000, 0x1_0000_0010)],
                stride: 1,
                duration: ms(2_000),
            })
        );
    }

    /// Ten samples whose time runs and chunks change shape: a run of one
    /// sample, a chunk of one sample, a chunk of none, and a gap before the
    /// last chunk.
    fn shapes() -> Track {
        Track {
            stts: stts(&[(2, 100), (1, 50), (3, 10), (4, 1_000)]),
            stsc: stsc(&[(1, 3, 1), (2, 1, 1), (3, 0, 1), (4, 6, 1)]),
            sizes: (Kind::Stsz, stsz(&[5, 5, 5, 7, 9, 9, 9, 9, 9, 9])),
            chunks: (Kind::Stco, stco(&[0, 20, 27, 40])),
            file_len: 94,
        }
    }

    #[test]
    fn follows_time_runs_and_chunks_of_every_shape() {
        assert_eq!(
            shapes().index(1_000),
            Ok(SeekIndex {
                points: vec![
                    point(0, 0),
                    point(100, 5),
                    point(200, 10),
                    point(250, 20),
                    point(260, 40),
                    point(270, 49),
                    point(280, 58),
                    point(1_280, 67),
                    point(2_280, 76),
                    point(3_280, 85),
                ],
                stride: 1,
                duration: ms(4_280),
            })
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn a_stride_steps_over_whole_time_runs_and_chunks() {
        // Samples 0, 4 and 8: the run of one sample and the chunks of one
        // and of no samples lie between candidates.
        assert_eq!(
            shapes().index_with(1_000, &limited(3)),
            Ok(SeekIndex {
                points: vec![point(0, 0), point(260, 40), point(2_280, 76)],
                stride: 4,
                duration: ms(4_280),
            })
        );
    }

    /// Ten samples of 10 octets, 100 ms apart, in one chunk.
    fn ten_samples() -> Track {
        Track {
            stts: stts(&[(10, 100)]),
            stsc: stsc(&[(1, 10, 1)]),
            sizes: (Kind::Stsz, stsz_constant(10, 10)),
            chunks: (Kind::Stco, stco(&[0])),
            file_len: 100,
        }
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn thins_to_the_index_entry_limit_and_says_by_how_much() {
        let every: &[u64] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let cases: [(u64, u64, &[u64]); 6] = [
            (11, 1, every),
            (10, 1, every),
            (9, 2, &[0, 2, 4, 6, 8]),
            (3, 4, &[0, 4, 8]),
            (1, 10, &[0]),
            (0, 10, &[]),
        ];
        for (limit, stride, samples) in cases {
            let points = samples
                .iter()
                .map(|&sample| point(sample * 100, sample * 10))
                .collect();
            assert_eq!(
                ten_samples().index_with(1_000, &limited(limit)),
                Ok(SeekIndex {
                    points,
                    stride,
                    duration: ms(1_000),
                }),
                "limit {limit}"
            );
        }
    }

    /// Verifies: SEC-MED-003, SEC-MED-006, SEC-MED-007
    #[test]
    fn keeps_a_track_of_u32_max_samples_to_the_limit_in_a_few_steps() {
        // Four billion one-octet samples in one chunk, declared in 72
        // octets: the index holds four points and allocates for four, and
        // the parse takes seven steps, not one per sample.
        let track = Track {
            stts: stts(&[(u32::MAX, 1)]),
            stsc: stsc(&[(1, u32::MAX, 1)]),
            sizes: (Kind::Stsz, stsz_constant(1, u32::MAX)),
            chunks: (Kind::Stco, stco(&[0])),
            file_len: 1 << 32,
        };
        let mut budget = Budget::for_input(0, 0, 7);
        let index = seek_index(
            track.bodies(),
            NonZeroU32::new(48_000).expect("not zero"),
            &limited(4),
            &mut budget,
        );
        assert_eq!(
            index,
            Ok(SeekIndex {
                points: vec![
                    point(0, 0),
                    point(22_369_621, 1 << 30),
                    point(44_739_242, 1 << 31),
                    point(67_108_864, 3 << 30),
                ],
                stride: 1 << 30,
                duration: ms(89_478_485),
            })
        );
        assert_eq!(index.map(|index| index.points.capacity()), Ok(4));
        assert_eq!(budget.remaining(), 0);
    }

    #[test]
    fn leaves_out_a_sample_that_plays_in_the_same_millisecond_as_the_point_before() {
        // Ticks 0, 0, 1, 2, 3 and 3,003 at 3,000 a second are 0, 0, 0, 0, 1
        // and 1,001 ms.
        let track = Track {
            stts: stts(&[(1, 0), (3, 1), (1, 3_000), (1, 7)]),
            stsc: stsc(&[(1, 6, 1)]),
            sizes: (Kind::Stsz, stsz_constant(2, 6)),
            chunks: (Kind::Stco, stco(&[50])),
            file_len: 62,
        };
        assert_eq!(
            track.index(3_000),
            Ok(SeekIndex {
                points: vec![point(0, 50), point(1, 58), point(1_001, 60)],
                stride: 1,
                duration: ms(1_003),
            })
        );
    }

    #[test]
    fn leaves_out_a_sample_that_starts_where_the_point_before_does() {
        // Samples of 0, 10, 0 and 10 octets: the second starts where the
        // first does, and the fourth where the third does.
        let track = Track {
            stts: stts(&[(4, 10)]),
            stsc: stsc(&[(1, 4, 1)]),
            sizes: (Kind::Stsz, stsz(&[0, 10, 0, 10])),
            chunks: (Kind::Stco, stco(&[100])),
            file_len: 120,
        };
        assert_eq!(
            track.index(1_000),
            Ok(SeekIndex {
                points: vec![point(0, 100), point(20, 110)],
                stride: 1,
                duration: ms(40),
            })
        );
    }

    #[test]
    fn chunks_before_the_first_run_hold_no_samples() {
        let track = Track {
            stts: stts(&[(2, 500)]),
            stsc: stsc(&[(2, 2, 1)]),
            sizes: (Kind::Stsz, stsz(&[5, 5])),
            chunks: (Kind::Stco, stco(&[100, 200])),
            file_len: 210,
        };
        assert_eq!(
            track.index(1_000),
            Ok(SeekIndex {
                points: vec![point(0, 200), point(500, 205)],
                stride: 1,
                duration: ms(1_000),
            })
        );
    }

    #[test]
    fn indexes_a_track_with_no_samples_as_no_points() {
        let nothing = Ok(SeekIndex {
            points: vec![],
            stride: 1,
            duration: ms(0),
        });
        let empty = Track {
            stts: stts(&[]),
            stsc: stsc(&[]),
            sizes: (Kind::Stsz, stsz(&[])),
            chunks: (Kind::Stco, stco(&[])),
            file_len: 0,
        };
        assert_eq!(empty.index(1), nothing);
        // A chunk no run reaches holds no samples, and lies in the file.
        let unreached = Track {
            sizes: (Kind::Stsz, stsz_constant(4, 0)),
            chunks: (Kind::Stco, stco(&[8])),
            file_len: 8,
            ..empty
        };
        assert_eq!(unreached.index(1), nothing);
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn refuses_a_chunk_offset_past_the_end_of_the_file() {
        let past = |file_len, offsets: &[u32]| Track {
            chunks: (Kind::Stco, stco(offsets)),
            file_len,
            ..minimal()
        };
        // The second chunk starts after the end of the file.
        assert_eq!(
            past(160, &[100, 161]).index(1_000),
            Err(SampleTableError::ChunkPastEnd {
                chunk: 2,
                offset: CHUNKS_AT + 12,
                start: 161,
                len: 30,
                file_len: 160,
            })
        );
        // It starts inside the file and ends one octet past it.
        assert_eq!(
            past(159, &[100, 130]).index(1_000),
            Err(SampleTableError::ChunkPastEnd {
                chunk: 2,
                offset: CHUNKS_AT + 12,
                start: 130,
                len: 30,
                file_len: 159,
            })
        );
        // The first chunk does.
        assert_eq!(
            past(160, &[140, 170]).index(1_000),
            Err(SampleTableError::ChunkPastEnd {
                chunk: 1,
                offset: CHUNKS_AT + 8,
                start: 140,
                len: 30,
                file_len: 160,
            })
        );
    }

    /// Verifies: SEC-MED-004, SEC-MED-008
    #[test]
    fn refuses_a_chunk_whose_end_overflows_64_bits() {
        // In a file of u64::MAX octets, a chunk ending past u64::MAX would
        // fit if its end saturated.
        let track = Track {
            stts: stts(&[(1, 1)]),
            stsc: stsc(&[(1, 1, 1)]),
            sizes: (Kind::Stsz, stsz(&[10])),
            chunks: (Kind::Co64, co64(&[u64::MAX - 5])),
            file_len: u64::MAX,
        };
        assert_eq!(
            track.index(1),
            Err(SampleTableError::ChunkPastEnd {
                chunk: 1,
                offset: CHUNKS_AT + 8,
                start: u64::MAX - 5,
                len: 10,
                file_len: u64::MAX,
            })
        );
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn refuses_a_chunk_that_starts_before_the_one_before_it_ends() {
        for start in [129, 50] {
            let track = Track {
                chunks: (Kind::Stco, stco(&[100, start])),
                ..minimal()
            };
            assert_eq!(
                track.index(1_000),
                Err(SampleTableError::ChunkOrder {
                    chunk: 2,
                    offset: CHUNKS_AT + 12,
                    start: u64::from(start),
                    previous_end: 130,
                }),
                "start {start}"
            );
        }
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn refuses_sample_to_chunk_runs_that_reference_chunk_0() {
        let track = Track {
            stsc: stsc(&[(0, 2, 1), (2, 1, 1)]),
            ..minimal()
        };
        assert_eq!(
            track.index(1_000),
            Err(SampleTableError::ChunkRun {
                run: 0,
                first_chunk: 0,
                chunks: 2,
                offset: STSC_AT + 8,
            })
        );
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn refuses_runs_that_do_not_follow_the_run_before_or_pass_the_last_chunk() {
        // Each case: the stsc runs, the chunk offsets and the error.
        type Case = (&'static [(u32, u32, u32)], &'static [u32], SampleTableError);
        let run = |run, first_chunk, chunks, offset| SampleTableError::ChunkRun {
            run,
            first_chunk,
            chunks,
            offset,
        };
        let cases: [Case; 4] = [
            (
                &[(1, 2, 1), (1, 1, 1)],
                &[100, 130],
                run(1, 1, 2, STSC_AT + 20),
            ),
            (
                &[(2, 2, 1), (1, 1, 1)],
                &[100, 130],
                run(1, 1, 2, STSC_AT + 20),
            ),
            (
                &[(1, 2, 1), (3, 1, 1)],
                &[100, 130],
                run(1, 3, 2, STSC_AT + 20),
            ),
            (&[(1, 3, 1)], &[], run(0, 1, 0, STSC_AT + 8)),
        ];
        for (runs, offsets, expected) in cases {
            let track = Track {
                stsc: stsc(runs),
                chunks: (Kind::Stco, stco(offsets)),
                ..minimal()
            };
            assert_eq!(track.index(1_000), Err(expected), "runs {runs:?}");
        }
    }

    #[test]
    fn refuses_tables_that_disagree_on_the_sample_count() {
        let counts = |time_to_sample, sample_sizes, sample_to_chunk| {
            Err(SampleTableError::SampleCounts {
                time_to_sample,
                sample_sizes,
                sample_to_chunk,
                offset: SIZES_AT + 8,
            })
        };
        let short_time = Track {
            stts: stts(&[(2, 20)]),
            ..minimal()
        };
        assert_eq!(short_time.index(1_000), counts(2, 3, 3));
        let long_runs = Track {
            stsc: stsc(&[(1, 2, 1)]),
            ..minimal()
        };
        assert_eq!(long_runs.index(1_000), counts(3, 3, 4));
        let short_sizes = Track {
            sizes: (Kind::Stz2, stz2(8, &[10, 20])),
            ..minimal()
        };
        assert_eq!(short_sizes.index(1_000), counts(3, 2, 3));
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn refuses_samples_that_play_for_more_than_30_days() {
        let lasting = |seconds| Track {
            stts: stts(&[(1, seconds)]),
            stsc: stsc(&[(1, 1, 1)]),
            sizes: (Kind::Stsz, stsz(&[1])),
            chunks: (Kind::Stco, stco(&[0])),
            file_len: 1,
        };
        assert_eq!(
            lasting(2_592_000).index(1),
            Ok(SeekIndex {
                points: vec![point(0, 0)],
                stride: 1,
                duration: ms(2_592_000_000),
            })
        );
        assert_eq!(
            lasting(2_592_001).index(1),
            Err(SampleTableError::Duration {
                ticks: 2_592_001,
                timescale: NonZeroU32::new(1).expect("not zero"),
                offset: STTS_AT,
            })
        );
    }

    /// `minimal` with its sizes or chunk offsets in `body`, a body of the
    /// box `kind`.
    fn with_body(kind: Kind, body: Vec<u8>) -> Track {
        let track = minimal();
        match kind {
            Kind::Stsz | Kind::Stz2 => Track {
                sizes: (kind, body),
                ..track
            },
            Kind::Stco | Kind::Co64 => Track {
                chunks: (kind, body),
                ..track
            },
        }
    }

    /// `body` with its version octet set to `version`.
    fn versioned(version: u8, body: &[u8]) -> Vec<u8> {
        [&[version][..], body.get(1..).expect("a body has a version")].concat()
    }

    #[test]
    fn refuses_every_version_but_0() {
        let error = |table, version, offset| {
            Err(SampleTableError::Version {
                table,
                version,
                offset,
            })
        };
        let stts_body = versioned(1, &stts(&[(3, 20)]));
        let stsc_body = versioned(1, &stsc(&[(1, 2, 1), (2, 1, 1)]));
        assert_eq!(
            Track {
                stts: stts_body,
                ..minimal()
            }
            .index(1_000),
            error(Table::Stts, 1, STTS_AT)
        );
        assert_eq!(
            Track {
                stsc: stsc_body,
                ..minimal()
            }
            .index(1_000),
            error(Table::Stsc, 1, STSC_AT)
        );
        for (kind, table, body, offset) in [
            (Kind::Stsz, Table::Stsz, stsz(&[10, 20, 30]), SIZES_AT),
            (Kind::Stz2, Table::Stz2, stz2(8, &[10, 20, 30]), SIZES_AT),
            (Kind::Stco, Table::Stco, stco(&[100, 130]), CHUNKS_AT),
            (Kind::Co64, Table::Co64, co64(&[100, 130]), CHUNKS_AT),
        ] {
            assert_eq!(
                with_body(kind, versioned(1, &body)).index(1_000),
                error(table, 1, offset)
            );
        }
        assert_eq!(
            Track {
                stts: versioned(0xFF, &stts(&[(3, 20)])),
                ..minimal()
            }
            .index(1_000),
            error(Table::Stts, 0xFF, STTS_AT)
        );
    }

    #[test]
    fn refuses_compact_field_sizes_other_than_4_8_and_16() {
        for bits in [0, 2, 12, 32, 0xFF] {
            let body = full_box(&[0x00, 0x00, 0x00, bits, 0x00, 0x00, 0x00, 0x00]);
            assert_eq!(
                with_body(Kind::Stz2, body).index(1_000),
                Err(SampleTableError::FieldSize {
                    bits,
                    offset: SIZES_AT + 7,
                }),
                "{bits} bits"
            );
        }
    }

    /// A body of `head` after the full-box header, a 32-bit `count`, then
    /// `entries`.
    fn counted(head: &[u8], count: u32, entries: &[u8]) -> Vec<u8> {
        let mut body = Bytes::new();
        body.u32_be(0).bytes(head).u32_be(count).bytes(entries);
        body.into_vec()
    }

    /// `minimal` with each table in turn declaring `count` entries and
    /// holding `held` after the count, with where those octets start: stts,
    /// stsc, stsz, stz2 of 4 and of 16 bits, stco and co64.
    fn declaring(count: u32, held: &[u8]) -> [(Track, u64); 7] {
        [
            (
                Track {
                    stts: counted(&[], count, held),
                    ..minimal()
                },
                STTS_AT + 8,
            ),
            (
                Track {
                    stsc: counted(&[], count, held),
                    ..minimal()
                },
                STSC_AT + 8,
            ),
            (
                with_body(Kind::Stsz, counted(&[0, 0, 0, 0], count, held)),
                SIZES_AT + 12,
            ),
            (
                with_body(Kind::Stz2, counted(&[0, 0, 0, 4], count, held)),
                SIZES_AT + 12,
            ),
            (
                with_body(Kind::Stz2, counted(&[0, 0, 0, 16], count, held)),
                SIZES_AT + 12,
            ),
            (
                with_body(Kind::Stco, counted(&[], count, held)),
                CHUNKS_AT + 8,
            ),
            (
                with_body(Kind::Co64, counted(&[], count, held)),
                CHUNKS_AT + 8,
            ),
        ]
    }

    /// The pattern of Stagefright's CVE-2015-1538: an entry count whose
    /// product with the entry size passes 2^32 by a few octets (by a few
    /// bits, for 4-bit fields). A 32-bit product wraps to those few, which
    /// the body then holds, and a parser that trusted it would read
    /// billions of entries past them.
    ///
    /// Verifies: SEC-TM-032, SEC-MED-004
    #[test]
    fn refuses_entry_counts_whose_size_overflows_32_bits() {
        // For each table of `declaring`: the count, the octets it needs,
        // and the octets a wrapped 32-bit product asks for.
        let cases: [(u32, u64, u64); 7] = [
            (0x2000_0001, 0x1_0000_0008, 8), // stts, 8 octets each
            (0x1555_5556, 0x1_0000_0008, 8), // stsc, 12 octets each
            (0x4000_0001, 0x1_0000_0004, 4), // stsz, 4 octets each
            (0x4000_0002, 0x2000_0001, 1),   // stz2, 4 bits each: 2^32 + 8 bits
            (0x8000_0001, 0x1_0000_0002, 2), // stz2, 2 octets each
            (0x4000_0001, 0x1_0000_0004, 4), // stco, 4 octets each
            (0x2000_0001, 0x1_0000_0008, 8), // co64, 8 octets each
        ];
        for (index, (count, needed, wrapped)) in cases.into_iter().enumerate() {
            let held: Vec<u8> = (0..wrapped).map(|_| 0x01).collect();
            let (track, offset) = declaring(count, &held)
                .into_iter()
                .nth(index)
                .expect("one case per table");
            assert_eq!(
                track.index(1_000),
                Err(truncated(offset, needed, wrapped)),
                "case {index}"
            );
        }
    }

    /// Verifies: SEC-TM-032, SEC-MED-003
    #[test]
    fn refuses_counts_of_u32_max_in_bodies_under_64_octets() {
        let max = u64::from(u32::MAX);
        let needed = [
            max * 8,
            max * 12,
            max * 4,
            (max * 4).div_ceil(8),
            max * 2,
            max * 4,
            max * 8,
        ];
        for ((track, offset), needed) in declaring(u32::MAX, &[]).into_iter().zip(needed) {
            for body in [&track.stts, &track.stsc, &track.sizes.1, &track.chunks.1] {
                assert!(body.len() < 64, "each body is under 64 octets");
            }
            assert_eq!(
                track.index(1_000),
                Err(truncated(offset, needed, 0)),
                "needed {needed}"
            );
        }
        // A constant size takes no list, so the count meets the others.
        assert_eq!(
            with_body(Kind::Stsz, stsz_constant(1, u32::MAX)).index(1_000),
            Err(SampleTableError::SampleCounts {
                time_to_sample: 3,
                sample_sizes: u32::MAX,
                sample_to_chunk: 3,
                offset: SIZES_AT + 8,
            })
        );
    }

    /// The fields of a body as the specification lays them out, each as
    /// where it starts and how many octets it takes, with `entries` octets
    /// of entries last: for stts, stsc, stco and co64 (`None`), or for the
    /// size box `kind`.
    fn layout(kind: Option<Kind>, entries: u64) -> Vec<(u64, u64)> {
        match kind {
            // stsz: version, flags, size, count, entries.
            Some(Kind::Stsz) => vec![(0, 1), (1, 3), (4, 4), (8, 4), (12, entries)],
            // stz2: version, flags, reserved, field size, count, entries.
            Some(Kind::Stz2) => vec![(0, 1), (1, 3), (4, 3), (7, 1), (8, 4), (12, entries)],
            // The others: version, flags, count, entries.
            _ => vec![(0, 1), (1, 3), (4, 4), (8, entries)],
        }
    }

    /// The fault for reading a body laid out as `fields`, which starts at
    /// `at`, cut to `len` octets: the first field the cut falls in.
    fn cut(fields: &[(u64, u64)], at: u64, len: u64) -> SampleTableError {
        let &(start, size) = fields
            .iter()
            .find(|&&(start, size)| start + size > len)
            .expect("the cut falls inside a field");
        truncated(at + start, size, len - start)
    }

    /// Which body of a track a cut shortens.
    #[derive(Debug, Clone, Copy)]
    enum Which {
        Stts,
        Stsc,
        Sizes,
        Chunks,
    }

    /// `track` with the body `which` replaced by `body`.
    fn replaced(track: &Track, which: Which, body: Vec<u8>) -> Track {
        let track = track.clone();
        match which {
            Which::Stts => Track {
                stts: body,
                ..track
            },
            Which::Stsc => Track {
                stsc: body,
                ..track
            },
            Which::Sizes => Track {
                sizes: (track.sizes.0, body),
                ..track
            },
            Which::Chunks => Track {
                chunks: (track.chunks.0, body),
                ..track
            },
        }
    }

    /// Verifies: SEC-TM-032
    #[test]
    fn every_cut_of_a_body_is_truncated_at_the_field_it_cuts() {
        let tracks = [
            (minimal(), Which::Stts, minimal().stts, None, STTS_AT),
            (minimal(), Which::Stsc, minimal().stsc, None, STSC_AT),
            (
                minimal(),
                Which::Sizes,
                minimal().sizes.1,
                Some(Kind::Stsz),
                SIZES_AT,
            ),
            (
                with_body(Kind::Stz2, stz2(4, &[10, 2, 3])),
                Which::Sizes,
                stz2(4, &[10, 2, 3]),
                Some(Kind::Stz2),
                SIZES_AT,
            ),
            (
                minimal(),
                Which::Chunks,
                minimal().chunks.1,
                None,
                CHUNKS_AT,
            ),
            (
                with_body(Kind::Co64, co64(&[100, 130])),
                Which::Chunks,
                co64(&[100, 130]),
                None,
                CHUNKS_AT,
            ),
        ];
        for (track, which, body, kind, at) in tracks {
            assert!(
                track.index(1_000).is_ok(),
                "the whole {which:?} body parses"
            );
            let len = u64::try_from(body.len()).expect("small");
            let header = layout(kind, 0).last().expect("fields").0;
            let fields = layout(kind, len - header);
            for (cut_len, end) in (0_u64..).zip(0..body.len()) {
                let short = body.get(..end).expect("a prefix").to_vec();
                assert_eq!(
                    replaced(&track, which, short).index(1_000),
                    Err(cut(&fields, at, cut_len)),
                    "{which:?} cut to {cut_len}"
                );
            }
        }
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn runs_out_of_budget_at_exactly_the_step_it_should() {
        // `minimal` takes eleven steps: its stts entry, its two stsc runs,
        // then for each chunk the chunk, its candidates and its listed
        // sizes.
        let charged_at = [
            STTS_AT + 8,    // the stts entry
            STSC_AT + 8,    // stsc run 0
            STSC_AT + 20,   // stsc run 1
            CHUNKS_AT + 8,  // chunk 1
            CHUNKS_AT + 8,  // its candidate sample 0
            CHUNKS_AT + 8,  // its candidate sample 1
            CHUNKS_AT + 8,  // the size of sample 0, passed to reach sample 1
            CHUNKS_AT + 8,  // the size of sample 1, to the end of the chunk
            CHUNKS_AT + 12, // chunk 2
            CHUNKS_AT + 12, // its candidate sample 2
            CHUNKS_AT + 12, // the size of sample 2, to the end of the chunk
        ];
        let timescale = NonZeroU32::new(1_000).expect("not zero");
        for (steps, offset) in (0_u64..).zip(charged_at) {
            let mut budget = Budget::for_input(0, 0, steps);
            assert_eq!(
                seek_index(minimal().bodies(), timescale, &Limits::DEFAULT, &mut budget),
                Err(SampleTableError::Fault(ParseFault::BudgetExceeded {
                    offset
                })),
                "{steps} steps"
            );
        }
        let mut budget = Budget::for_input(0, 0, 11);
        assert_eq!(
            seek_index(minimal().bodies(), timescale, &Limits::DEFAULT, &mut budget),
            minimal().index(1_000)
        );
        assert_eq!(budget.remaining(), 0);
    }

    #[test]
    fn describes_every_error_as_a_damaged_sample_table_at_its_offset() {
        let timescale = NonZeroU32::new(1).expect("not zero");
        let errors = [
            (truncated(1, 4, 2), 1),
            (
                SampleTableError::Fault(ParseFault::BudgetExceeded { offset: 2 }),
                2,
            ),
            (
                SampleTableError::Version {
                    table: Table::Stco,
                    version: 1,
                    offset: 3,
                },
                3,
            ),
            (SampleTableError::FieldSize { bits: 5, offset: 4 }, 4),
            (
                SampleTableError::ChunkRun {
                    run: 1,
                    first_chunk: 0,
                    chunks: 1,
                    offset: 5,
                },
                5,
            ),
            (
                SampleTableError::SampleCounts {
                    time_to_sample: 1,
                    sample_sizes: 2,
                    sample_to_chunk: 3,
                    offset: 6,
                },
                6,
            ),
            (
                SampleTableError::Duration {
                    ticks: 1,
                    timescale,
                    offset: 7,
                },
                7,
            ),
            (
                SampleTableError::ChunkOrder {
                    chunk: 2,
                    offset: 8,
                    start: 1,
                    previous_end: 2,
                },
                8,
            ),
            (
                SampleTableError::ChunkPastEnd {
                    chunk: 1,
                    offset: 9,
                    start: 1,
                    len: 2,
                    file_len: 2,
                },
                9,
            ),
        ];
        for (error, offset) in errors {
            assert_eq!(
                error.problem(),
                Problem {
                    code: ProblemCode::SampleTableDamaged,
                    args: vec![("offset", Arg::Number(offset))],
                },
                "{error:?}"
            );
        }
    }

    /// Whether `points` strictly increase in both time and offset.
    fn monotonic(points: &[SeekPoint]) -> bool {
        points
            .iter()
            .zip(points.iter().skip(1))
            .all(|(before, after)| before.time < after.time && before.offset < after.offset)
    }

    /// A body that is arbitrary octets, or the full-box header, `head`, a
    /// small entry count and arbitrary octets, so that parses reach the
    /// entries.
    fn arbitrary_body(head: &'static [u8]) -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            vec(any::<u8>(), 0..40),
            (0_u32..6, vec(any::<u8>(), 0..80))
                .prop_map(move |(count, rest)| counted(head, count, &rest)),
        ]
    }

    /// Arbitrary bodies for every table, each in either of its boxes.
    fn arbitrary_track() -> impl Strategy<Value = Track> {
        let sizes = prop_oneof![
            arbitrary_body(&[0, 0, 0, 0]).prop_map(|body| (Kind::Stsz, body)),
            (1_u32..64, 0_u32..6)
                .prop_map(|(size, count)| (Kind::Stsz, counted(&size.to_be_bytes(), count, &[]))),
            (
                prop_oneof![Just(4_u8), Just(8), Just(16), any::<u8>()],
                0_u32..6,
                vec(any::<u8>(), 0..40),
            )
                .prop_map(|(bits, count, rest)| (
                    Kind::Stz2,
                    counted(&[0, 0, 0, bits], count, &rest)
                )),
        ];
        let chunks = prop_oneof![
            arbitrary_body(&[]).prop_map(|body| (Kind::Stco, body)),
            arbitrary_body(&[]).prop_map(|body| (Kind::Co64, body)),
        ];
        (
            arbitrary_body(&[]),
            arbitrary_body(&[]),
            sizes,
            chunks,
            prop_oneof![any::<u64>(), 0_u64..2_000],
        )
            .prop_map(|(stts, stsc, sizes, chunks, file_len)| Track {
                stts,
                stsc,
                sizes,
                chunks,
                file_len,
            })
    }

    /// A track the testkit built, with the offset and decoding time of
    /// every sample and the track's length in ticks, all found without the
    /// tables.
    #[derive(Debug, Clone)]
    struct Generated {
        track: Track,
        samples: Vec<(u64, u64)>,
        ticks: u64,
        timescale: u32,
        limit: u64,
    }

    prop_compose! {
        /// A track of up to six chunks, each a gap and up to four samples
        /// of a size and a time delta, written with every size box and
        /// both chunk offset boxes.
        fn generated()(
            chunks in vec((0_u64..20, vec((0_u32..300, 0_u32..2_000), 0..5)), 0..7),
            kind in 0_u8..5,
            constant in 1_u32..40,
            large in any::<bool>(),
            extra in 0_u64..10,
            timescale in 1_u32..50_000,
            limit in 0_u64..12,
        ) -> Generated {
            let size = |size: u32| match kind {
                1 => constant,
                2 => size % 16,
                3 => size % 256,
                _ => size,
            };
            let mut position: u64 = if large { 1 << 32 } else { 0 };
            let (mut starts, mut counts, mut sizes, mut deltas) = (vec![], vec![], vec![], vec![]);
            let (mut samples, mut ticks) = (vec![], 0_u64);
            for (gap, chunk) in &chunks {
                position += gap;
                starts.push(position);
                counts.push(u32::try_from(chunk.len()).expect("small"));
                for &(octets, delta) in chunk {
                    samples.push((position, ticks));
                    position += u64::from(size(octets));
                    ticks += u64::from(delta);
                    sizes.push(size(octets));
                    deltas.push(delta);
                }
            }
            let narrow: Vec<u16> = sizes
                .iter()
                .map(|&size| u16::try_from(size).expect("under 300"))
                .collect();
            let sizes = match kind {
                1 => (
                    Kind::Stsz,
                    stsz_constant(constant, u32::try_from(sizes.len()).expect("small")),
                ),
                2 => (Kind::Stz2, stz2(4, &narrow)),
                3 => (Kind::Stz2, stz2(8, &narrow)),
                4 => (Kind::Stz2, stz2(16, &narrow)),
                _ => (Kind::Stsz, stsz(&sizes)),
            };
            let chunks = if large {
                (Kind::Co64, co64(&starts))
            } else {
                let starts: Vec<u32> = starts
                    .iter()
                    .map(|&start| u32::try_from(start).expect("small"))
                    .collect();
                (Kind::Stco, stco(&starts))
            };
            Generated {
                track: Track {
                    stts: stts(&time_runs(&deltas)),
                    stsc: stsc(&chunk_runs(&counts)),
                    sizes,
                    chunks,
                    file_len: position + extra,
                },
                samples,
                ticks,
                timescale,
                limit,
            }
        }
    }

    /// The seek index of `generated`, found from its samples directly:
    /// every sample whose number is a multiple of the stride, up to the
    /// limit, that plays later and starts further on than the point before.
    fn reference(generated: &Generated) -> SeekIndex {
        let count = u64::try_from(generated.samples.len()).expect("small");
        let limit = generated.limit;
        // The smallest stride that leaves no more candidates than the
        // limit; with a limit of zero no stride does, and the stride is the
        // whole track.
        let stride = (1..=count.max(1))
            .find(|stride| count.div_ceil(*stride) <= limit)
            .unwrap_or(count);
        let millis = |ticks: u64| ticks * 1_000 / u64::from(generated.timescale);
        let mut points: Vec<SeekPoint> = vec![];
        for (number, &(offset, ticks)) in (0_u64..).zip(&generated.samples) {
            let time = ms(millis(ticks));
            if number % stride == 0
                && number / stride < limit
                && points
                    .last()
                    .is_none_or(|last| time > last.time && offset > last.offset)
            {
                points.push(SeekPoint { time, offset });
            }
        }
        SeekIndex {
            points,
            stride,
            duration: ms(millis(generated.ticks)),
        }
    }

    /// A track the testkit built with one octet of one body changed, so
    /// that parses of nearly valid tables reach the walk over the chunks.
    fn mutated() -> impl Strategy<Value = Track> {
        (
            generated(),
            0_u8..4,
            any::<prop::sample::Index>(),
            any::<u8>(),
        )
            .prop_map(|(generated, which, at, octet)| {
                let mut track = generated.track;
                let body = match which {
                    0 => &mut track.stts,
                    1 => &mut track.stsc,
                    2 => &mut track.sizes.1,
                    _ => &mut track.chunks.1,
                };
                let at = at.index(body.len());
                *body
                    .get_mut(at)
                    .expect("every body holds at least its header and count") = octet;
                track
            })
    }

    proptest! {
        /// Verifies: SEC-MED-001, SEC-MED-005, SEC-MED-007, SEC-MED-008, SEC-HIS-036
        #[test]
        fn returns_for_any_bodies_on_a_small_stack(
            track in prop_oneof![arbitrary_track(), mutated()],
            timescale in 1_u32..100_000,
            limit in 0_u64..8,
        ) {
            let parsed = track.clone();
            let result = on_small_stack(move || parsed.index_with(timescale, &limited(limit)));
            if let Ok(index) = result {
                prop_assert!(monotonic(&index.points), "{index:?}");
                prop_assert!(u64::try_from(index.points.len()).expect("small") <= limit);
                prop_assert!(index.points.iter().all(|point| point.offset <= track.file_len
                    && point.time <= index.duration));
            }
        }

        /// Verifies: SEC-MED-006, SEC-MED-007, SEC-MED-008
        #[test]
        fn indexes_every_track_the_testkit_builds_monotonically(generated in generated()) {
            let index = generated.track.index_with(generated.timescale, &limited(generated.limit));
            prop_assert_eq!(index.as_ref().map(|index| monotonic(&index.points)), Ok(true));
            prop_assert_eq!(index, Ok(reference(&generated)));
        }
    }
}
