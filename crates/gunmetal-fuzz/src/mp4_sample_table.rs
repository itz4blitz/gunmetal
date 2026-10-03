//! The harnesses for the MP4 sample table parser in
//! [`gunmetal_core::formats::mp4::sample_table`].
//!
//! [`run`] cuts arbitrary bodies out of its input. [`structured`] writes
//! valid tables for a track its input describes and then changes one
//! octet, so that fuzzing reaches the walk over the chunks instead of
//! stopping at the first count that does not fit (SEC-MED-031); the MP4
//! structure-aware harness runs it.
//!
//! Both join the tables with the budget the parser documents and check the
//! outcome. They panic when the parser runs out of that budget; reports an
//! error at an offset past the octets the bodies came from; or returns an
//! index whose points do not strictly increase in time and offset, number
//! more than the index-entry limit, start past the end of the file or play
//! after the track ends, or whose stride is zero.

use std::num::NonZeroU32;

use gunmetal_core::formats::mp4::sample_table::{
    self, ChunkOffsets, STEPS_PER_OCTET, SampleSizes, SampleTableBodies, SampleTableError,
    SeekIndex,
};
use gunmetal_core::parse::{Budget, Cursor, LimitKind, Limits, ParseFault};
use gunmetal_core::problem::{Arg, Describe};

/// What the parser reported for one input.
pub type Outcome = Result<SeekIndex, SampleTableError>;

/// Octets of the settings [`run`] reads before the bodies.
pub const SETTINGS: usize = 11;

/// Cuts four sample table bodies out of `data` and joins them.
///
/// The first [`SETTINGS`] octets, read as zeros where `data` is shorter,
/// are: a flags octet whose bit 0 picks `stz2` over `stsz` and whose bit 1
/// picks `co64` over `stco`; a 16-bit timescale, where 0 counts as 1; the
/// index-entry limit; the 32-bit length of the file; and the lengths of the
/// `stts`, `stsc` and sample size bodies. The chunk offset body takes the
/// rest. Each body is read at its own offset in `data`.
///
/// # Errors
///
/// Returns the parser's error for the tables, which the harness reports
/// rather than treating as a finding.
///
/// # Panics
///
/// Panics when the outcome breaks an invariant the module lists.
pub fn run(data: &[u8]) -> Outcome {
    let mut settings = [0; SETTINGS];
    for (slot, &octet) in settings.iter_mut().zip(data) {
        *slot = octet;
    }
    let [
        flags,
        scale_high,
        scale_low,
        limit,
        l0,
        l1,
        l2,
        l3,
        stts,
        stsc,
        sizes,
    ] = settings;
    let mut bodies = Bodies {
        rest: data.get(SETTINGS..).unwrap_or_default(),
        offset: SETTINGS,
    };
    let stts = bodies.next(stts);
    let stsc = bodies.next(stsc);
    let sizes = bodies.next(sizes);
    let chunks = Cursor::at(bodies.rest, offset(bodies.offset));
    let timescale = NonZeroU32::new(u32::from(u16::from_be_bytes([scale_high, scale_low])))
        .unwrap_or(NonZeroU32::MIN);
    let tables = SampleTableBodies {
        stts,
        stsc,
        sizes: if flags & 1 == 1 {
            SampleSizes::Stz2(sizes)
        } else {
            SampleSizes::Stsz(sizes)
        },
        chunks: if flags & 2 == 2 {
            ChunkOffsets::Co64(chunks)
        } else {
            ChunkOffsets::Stco(chunks)
        },
        file_len: u64::from(u32::from_be_bytes([l0, l1, l2, l3])),
    };
    // Settings missing from a short input read as zeros, so the bodies
    // start at SETTINGS whatever its length.
    check(
        tables,
        timescale,
        u64::from(limit),
        data.len().max(SETTINGS),
    )
}

/// The bodies [`run`] cuts from its input, in order.
struct Bodies<'a> {
    /// The octets not cut yet.
    rest: &'a [u8],
    /// Where `rest` starts in the input.
    offset: usize,
}

impl<'a> Bodies<'a> {
    /// Cuts the next body, of `len` octets or what is left.
    fn next(&mut self, len: u8) -> Cursor<'a> {
        let (body, rest) = self.rest.split_at(usize::from(len).min(self.rest.len()));
        let cursor = Cursor::at(body, offset(self.offset));
        self.offset += body.len();
        self.rest = rest;
        cursor
    }
}

/// `at`, a position in a slice, as a file offset.
fn offset(at: usize) -> u64 {
    u64::try_from(at).expect("a slice position fits 64 bits")
}

/// Joins `tables` under an index-entry limit of `limit` and the budget the
/// parser documents, and checks the outcome against the invariants the
/// module lists, where `input_len` is how many octets the bodies came from.
fn check(
    tables: SampleTableBodies<'_>,
    timescale: NonZeroU32,
    limit: u64,
    input_len: usize,
) -> Outcome {
    let limits = Limits::DEFAULT
        .with_override(LimitKind::IndexEntries, limit)
        .expect("the harness keeps the limit under its ceiling");
    let octets = [
        tables.stts,
        tables.stsc,
        match tables.sizes {
            SampleSizes::Stsz(body) | SampleSizes::Stz2(body) => body,
        },
        match tables.chunks {
            ChunkOffsets::Stco(body) | ChunkOffsets::Co64(body) => body,
        },
    ]
    .iter()
    .map(Cursor::remaining)
    .sum();
    let mut budget = Budget::for_input(octets, STEPS_PER_OCTET, limit);
    let outcome = sample_table::seek_index(tables, timescale, &limits, &mut budget);
    match &outcome {
        Ok(index) => assert!(
            index
                .points
                .iter()
                .zip(index.points.iter().skip(1))
                .all(|(before, after)| before.time < after.time && before.offset < after.offset)
                && index.points.len() <= usize::try_from(limit).unwrap_or(usize::MAX)
                && index
                    .points
                    .iter()
                    .all(|point| point.offset <= tables.file_len && point.time <= index.duration)
                && index.stride > 0,
            "{index:?}"
        ),
        Err(error) => assert!(
            !matches!(
                error,
                SampleTableError::Fault(ParseFault::BudgetExceeded { .. })
            ) && error
                .problem()
                .args
                .iter()
                .all(|&(name, arg)| name == "offset"
                    && matches!(arg, Arg::Number(at) if at <= offset(input_len))),
            "{error:?} from {input_len} octets"
        ),
    }
    outcome
}

/// Writes valid sample tables for the track `data` describes, changes one
/// octet of the body `data` picks, and joins them.
///
/// `data` is read in order, as zeros once it runs out: the track, as
/// [`Generated::read`] reads it; then which body to change (0 none, 1
/// `stts`, 2 `stsc`, 3 the sizes, 4 the chunk offsets), a 16-bit position
/// in it, and the octet to put there. The bodies lie one after another, in
/// that order, from offset 0.
///
/// # Errors
///
/// Returns the parser's error for the tables, which the harness reports
/// rather than treating as a finding.
///
/// # Panics
///
/// Panics when the outcome breaks an invariant the module lists, or when no
/// body was changed and the parser refuses the tables or returns a point
/// that is not the start and decoding time of one of the samples.
pub fn structured(data: &[u8]) -> Outcome {
    let mut choices = Choices(data);
    let track = Generated::read(&mut choices);
    let mut bodies = track.bodies();
    let which = choices.octet() % 5;
    let at = usize::from(choices.word());
    let octet = choices.octet();
    if which > 0 {
        let body = &mut bodies[usize::from(which - 1)];
        let len = body.len();
        body[at % len] = octet;
    }
    let [stts, stsc, sizes, chunks] = &bodies;
    let stsc_at = stts.len();
    let sizes_at = stsc_at + stsc.len();
    let chunks_at = sizes_at + sizes.len();
    let size_cursor = Cursor::at(sizes, offset(sizes_at));
    let chunk_cursor = Cursor::at(chunks, offset(chunks_at));
    let tables = SampleTableBodies {
        stts: Cursor::at(stts, 0),
        stsc: Cursor::at(stsc, offset(stsc_at)),
        sizes: if track.kind < 2 {
            SampleSizes::Stsz(size_cursor)
        } else {
            SampleSizes::Stz2(size_cursor)
        },
        chunks: if track.large {
            ChunkOffsets::Co64(chunk_cursor)
        } else {
            ChunkOffsets::Stco(chunk_cursor)
        },
        file_len: track.file_len,
    };
    let input_len = bodies.iter().map(Vec::len).sum();
    let outcome = check(tables, track.timescale, track.limit, input_len);
    if which == 0 {
        assert!(
            outcome
                .as_ref()
                .is_ok_and(|index| index.points.iter().all(|point| {
                    track.samples.iter().any(|&(start, ticks)| {
                        point.offset == start
                            && point.time.millis()
                                == ticks * 1_000 / u64::from(track.timescale.get())
                    })
                })),
            "unchanged tables gave {outcome:?}"
        );
    }
    outcome
}

/// The octets of a harness input, read in order, as zeros once they run
/// out.
struct Choices<'a>(&'a [u8]);

impl Choices<'_> {
    /// The next octet.
    fn octet(&mut self) -> u8 {
        let Some((&first, rest)) = self.0.split_first() else {
            return 0;
        };
        self.0 = rest;
        first
    }

    /// The next two octets, as a big-endian integer.
    fn word(&mut self) -> u16 {
        u16::from_be_bytes([self.octet(), self.octet()])
    }
}

/// A track [`structured`] writes tables for.
struct Generated {
    /// The size box: 0 an `stsz` list, 1 an `stsz` constant size, 2, 3 and 4
    /// an `stz2` of 4, 8 and 16 bits.
    kind: u8,
    /// Whether the chunks start past 4 GiB, in a `co64`, or not, in an
    /// `stco`.
    large: bool,
    /// Ticks per second.
    timescale: NonZeroU32,
    /// The index-entry limit.
    limit: u64,
    /// The one size of every sample, for a constant size.
    constant: u16,
    /// Where each chunk starts.
    starts: Vec<u64>,
    /// How many samples each chunk holds.
    counts: Vec<u8>,
    /// Each sample's size.
    sizes: Vec<u16>,
    /// Each sample's time delta.
    deltas: Vec<u16>,
    /// Each sample's start and decoding time, found without the tables.
    samples: Vec<(u64, u64)>,
    /// The length of the file.
    file_len: u64,
}

impl Generated {
    /// Reads a track from `choices`: the number of chunks (0 to 7); the
    /// size box; whether the chunks start past 4 GiB (odd) or not (even); a
    /// 16-bit timescale, where 0 counts as 1; the index-entry limit; the
    /// constant size, where 0 counts as 1; for each chunk, the gap before it
    /// and its number of samples (0 to 5), each a 16-bit size, cut to fit
    /// its field, and a 16-bit time delta; and the octets after the last
    /// chunk.
    fn read(choices: &mut Choices<'_>) -> Self {
        let chunks = choices.octet() % 8;
        let kind = choices.octet() % 5;
        let large = choices.octet() % 2 == 1;
        let timescale = NonZeroU32::new(u32::from(choices.word())).unwrap_or(NonZeroU32::MIN);
        let limit = u64::from(choices.octet());
        let constant = u16::from(choices.octet().max(1));
        let mut track = Self {
            kind,
            large,
            timescale,
            limit,
            constant,
            starts: vec![],
            counts: vec![],
            sizes: vec![],
            deltas: vec![],
            samples: vec![],
            file_len: 0,
        };
        let mut position: u64 = if large { 1 << 32 } else { 0 };
        let mut ticks: u64 = 0;
        for _ in 0..chunks {
            position += u64::from(choices.octet());
            track.starts.push(position);
            let count = choices.octet() % 6;
            track.counts.push(count);
            for _ in 0..count {
                let word = choices.word();
                let size = match kind {
                    1 => constant,
                    2 => word & 0x0F,
                    3 => word & 0xFF,
                    _ => word,
                };
                let delta = choices.word();
                track.samples.push((position, ticks));
                position += u64::from(size);
                ticks += u64::from(delta);
                track.sizes.push(size);
                track.deltas.push(delta);
            }
        }
        track.file_len = position + u64::from(choices.octet());
        track
    }

    /// The track's `stts`, `stsc`, sample size and chunk offset bodies, with
    /// one `stts` entry for each sample and one `stsc` run for each chunk.
    fn bodies(&self) -> [Vec<u8>; 4] {
        let stts = full_box(
            &[],
            self.deltas.len(),
            self.deltas
                .iter()
                .flat_map(|&delta| [1, u32::from(delta)])
                .flat_map(u32::to_be_bytes),
        );
        let stsc = full_box(
            &[],
            self.counts.len(),
            (1_u32..)
                .zip(&self.counts)
                .flat_map(|(chunk, &count)| [chunk, u32::from(count), 1])
                .flat_map(u32::to_be_bytes),
        );
        let count = self.sizes.len();
        let sizes = match self.kind {
            0 => full_box(
                &[0, 0, 0, 0],
                count,
                self.sizes
                    .iter()
                    .flat_map(|&size| u32::from(size).to_be_bytes()),
            ),
            1 => full_box(
                &u32::from(self.constant).to_be_bytes(),
                count,
                std::iter::empty(),
            ),
            2 => full_box(
                &[0, 0, 0, 4],
                count,
                self.sizes.chunks(2).map(|pair| {
                    let high = pair.first().copied().unwrap_or_default();
                    let low = pair.get(1).copied().unwrap_or_default();
                    let [_, packed] = ((high << 4) + low).to_be_bytes();
                    packed
                }),
            ),
            3 => full_box(
                &[0, 0, 0, 8],
                count,
                self.sizes.iter().map(|&size| size.to_be_bytes()[1]),
            ),
            _ => full_box(
                &[0, 0, 0, 16],
                count,
                self.sizes.iter().flat_map(|&size| size.to_be_bytes()),
            ),
        };
        let chunks = if self.large {
            full_box(
                &[],
                self.starts.len(),
                self.starts.iter().flat_map(|&start| start.to_be_bytes()),
            )
        } else {
            full_box(
                &[],
                self.starts.len(),
                self.starts.iter().flat_map(|&start| {
                    u32::try_from(start)
                        .expect("small chunks start under 4 GiB")
                        .to_be_bytes()
                }),
            )
        };
        [stts, stsc, sizes, chunks]
    }
}

/// A full-box body: version 0, no flags, `head`, `count` as a 32-bit
/// integer, then `entries`.
fn full_box(head: &[u8], count: usize, entries: impl Iterator<Item = u8>) -> Vec<u8> {
    let count = u32::try_from(count).expect("a generated table is small");
    let mut body = vec![0, 0, 0, 0];
    body.extend_from_slice(head);
    body.extend_from_slice(&count.to_be_bytes());
    body.extend(entries);
    body
}
