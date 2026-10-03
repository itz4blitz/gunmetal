//! Ogg pages and packets (RFC 3533), the container of Ogg Vorbis, Opus and
//! Ogg FLAC.
//!
//! An Ogg file is a run of pages. Each page starts with the capture pattern
//! `OggS` and carries a checksum over itself, a granule position, the
//! serial number of the logical stream it belongs to, a sequence number
//! within that stream, and a segment table of up to 255 lacing values,
//! followed by that many segments of at most 255 octets. A packet is a run
//! of segments that ends with one shorter than 255 octets, so a packet
//! whose length is a multiple of 255 ends with an empty segment. A packet
//! may go on from one page to the next, which then says it is continued.
//! Several logical streams may be interleaved or chained in one file.
//!
//! - [`pages`] reads pages back to back, checks each one's checksum, and
//!   after anything that is not a well-formed page moves on to the next
//!   capture pattern, reporting what it skipped.
//! - [`packets`] reassembles the packets of one logical stream across
//!   pages, skips the pages of other streams and records the streams
//!   whose first page it saw.
//! - [`last_granule`] reads the last granule position of a stream from the
//!   tail of a file, from which the duration follows.
//! - [`PageIndex`] keeps the pages a seek or the packager can start from.
//!
//! Codec headers inside the packets are read elsewhere.
//!
//! # Work and limits
//!
//! Every walk charges its [`Budget`] one step for each octet of a page
//! whose checksum it computes, one for each octet it skips while looking
//! for a page, and one for each damaged page header it moves past. A
//! well-formed stream therefore costs exactly one step per octet, and a
//! damaged page at most two, so [`STEPS_PER_OCTET`] is two and no fixed
//! steps are needed (k = 2 and c = 0 in SEC-MED-007's terms). Only false
//! pages that overlap one another, which no encoder writes, cost more, and
//! the budget stops them. Ogg has no nesting: a packet spanning many pages
//! is carried from page to page, never by recursion (SEC-MED-005).
//! Allocations follow the octets read: a packet that spans pages is copied
//! into one buffer as its pages arrive, the other streams recorded stop at
//! the children limit, and the page index thins itself at the index limit
//! (SEC-MED-003, SEC-MED-006).

use std::borrow::Cow;
use std::num::NonZeroU64;

use crate::parse::{Budget, Cursor, LimitKind, Limits, ParseFault};

/// Steps per octet of input a walk over Ogg pages needs at most, unless
/// false pages overlap: give each walk `Budget::for_input(len,
/// STEPS_PER_OCTET, 0)`.
pub const STEPS_PER_OCTET: u64 = 2;

/// The capture pattern every page starts with.
const CAPTURE: &[u8; 4] = b"OggS";

/// Octets in a page header before its segment table.
const HEADER_LEN: u64 = 27;

/// Where the checksum lies in the header.
const CRC_FIELD: std::ops::Range<usize> = 22..26;

/// The header type flag of a page that continues a packet.
const CONTINUED: u8 = 0x01;

/// The header type flag of the first page of a logical stream.
const FIRST: u8 = 0x02;

/// The header type flag of the last page of a logical stream.
const LAST: u8 = 0x04;

/// The granule position of a page on which no packet ends: −1.
const NO_GRANULE: u64 = u64::MAX;

/// The longest segment; any shorter one ends a packet.
const FULL_SEGMENT: u8 = 255;

/// The generator polynomial of the Ogg checksum.
const POLYNOMIAL: u32 = 0x04C1_1DB7;

/// The Ogg checksum's lookup table: entry `n` is the checksum of the one
/// octet `n`.
static CRC_TABLE: [u32; 256] = crc_table();

/// Builds [`CRC_TABLE`] when the crate compiles.
#[expect(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "the entry counts from 0 to 255 over a table of 256 entries, and the shifts move bits out of a 32-bit value on purpose"
)]
const fn crc_table() -> [u32; 256] {
    let mut table = [0; 256];
    let mut entry: u32 = 0;
    while entry < 256 {
        let mut crc = entry << 24;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 0x8000_0000 == 0 {
                crc << 1
            } else {
                (crc << 1) ^ POLYNOMIAL
            };
            bit += 1;
        }
        table[entry as usize] = crc;
        entry += 1;
    }
    table
}

/// The Ogg checksum (polynomial 0x04C11DB7, initial value 0, not reflected,
/// no final XOR) of `octets`, continuing from `state`.
fn crc(state: u32, octets: impl IntoIterator<Item = u8>) -> u32 {
    octets.into_iter().fold(state, |crc, octet| {
        let [top, high, low, last] = crc.to_be_bytes();
        #[expect(
            clippy::indexing_slicing,
            reason = "top ^ octet is a u8, so it indexes a table of 256 entries"
        )]
        let entry = CRC_TABLE[usize::from(top ^ octet)];
        u32::from_be_bytes([high, low, last, 0]) ^ entry
    })
}

/// One page, as read from the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page<'a> {
    /// Where the page starts, in octets from the start of the file.
    pub offset: u64,
    /// Whether the page's first segment continues a packet from the page
    /// before it in the same stream.
    pub continued: bool,
    /// Whether this is the first page of its logical stream.
    pub first: bool,
    /// Whether this is the last page of its logical stream.
    pub last: bool,
    /// The granule position after the last packet that ends on this page,
    /// or `None` when the page says no packet ends on it (−1).
    pub granule: Option<u64>,
    /// The serial number of the page's logical stream.
    pub serial: u32,
    /// The page's sequence number within its logical stream.
    pub sequence: u32,
    /// The segment table: one lacing value per segment.
    pub lacing: &'a [u8],
    /// The segments, back to back.
    pub body: &'a [u8],
}

/// Why octets of the input are not a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageError {
    /// Octets that do not start with a capture pattern, up to the next one
    /// or the end of the input.
    Skipped {
        /// Where the octets start, from the start of the file.
        offset: u64,
        /// How many there are.
        len: u64,
    },
    /// A page of a structure version other than 0, the only one there is.
    Version {
        /// Where the page starts.
        offset: u64,
        /// The version it declared.
        version: u8,
    },
    /// A page whose checksum does not match its octets.
    Crc {
        /// Where the page starts.
        offset: u64,
        /// The checksum the page carries.
        stored: u32,
        /// The checksum of its octets.
        computed: u32,
    },
    /// The input ended inside the page that starts at the fault's offset,
    /// which needs the octets the fault names, or the step budget ran out.
    Fault(ParseFault),
}

/// Iterator over the pages of an input. See [`pages`].
#[derive(Debug)]
pub struct Pages<'a, 'b> {
    /// What is left to read.
    rest: Cursor<'a>,
    /// What the walk may spend.
    budget: &'b mut Budget,
}

/// Reads the pages packed back to back in `input`, charging `budget`.
///
/// Each item is a page or what was wrong at that point. After an error the
/// walk moves on by one octet and reports the octets up to the next capture
/// pattern as [`PageError::Skipped`], so a damaged page, a capture pattern
/// inside a packet's data or junk before the first page costs that much and
/// no more. A page the input ends inside, even inside its capture pattern,
/// is reported as truncated, with the octets it needs, so a caller reading
/// the file in windows can read again from its offset. Once the budget runs
/// out the walk reports it and ends.
pub const fn pages<'a, 'b>(input: Cursor<'a>, budget: &'b mut Budget) -> Pages<'a, 'b> {
    Pages {
        rest: input,
        budget,
    }
}

impl<'a> Pages<'a, '_> {
    /// Reads what starts where the walk is: a page, a damaged page, or
    /// octets up to the next capture pattern. Returns it with where the
    /// walk goes on.
    fn read(&mut self) -> (Result<Page<'a>, PageError>, Cursor<'a>) {
        let at = self.rest;
        let offset = at.offset();
        let octets = at.rest();
        let junk = octets
            .windows(CAPTURE.len())
            .position(|window| window == CAPTURE)
            .unwrap_or_else(|| {
                // The input may end inside a capture pattern, which is then
                // a page cut short rather than junk.
                let cut = (1..CAPTURE.len())
                    .find(|&len| {
                        CAPTURE
                            .get(..len)
                            .is_some_and(|start| octets.ends_with(start))
                    })
                    .unwrap_or(0);
                octets.len().saturating_sub(cut)
            });
        let mut after = at;
        // A slice never holds more than u64::MAX octets. Skipping is only
        // ever by a non-zero count, so the walk always moves on.
        if let Some(len) = NonZeroU64::new(u64::try_from(junk).unwrap_or(u64::MAX)) {
            let len = len.get();
            let _ = after.skip(len);
            let item = match self.budget.charge(len, offset) {
                Ok(()) => PageError::Skipped { offset, len },
                Err(fault) => PageError::Fault(fault),
            };
            return (Err(item), after);
        }
        match page_at(at, self.budget) {
            Ok((page, after)) => (Ok(page), after),
            Err(error) => {
                // Moving past a damaged header costs a step, like any octet
                // the walk passes without checking.
                let item = match error {
                    PageError::Version { .. } | PageError::Fault(ParseFault::Truncated { .. }) => {
                        self.budget
                            .charge(1, offset)
                            .map_or_else(PageError::Fault, |()| error)
                    }
                    _ => error,
                };
                let _ = after.skip(1);
                (Err(item), after)
            }
        }
    }
}

impl<'a> Iterator for Pages<'a, '_> {
    type Item = Result<Page<'a>, PageError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }
        let (item, after) = self.read();
        // A spent budget ends the walk; anything else moves it on by at
        // least one octet.
        self.rest = if matches!(
            item,
            Err(PageError::Fault(ParseFault::BudgetExceeded { .. }))
        ) {
            Cursor::at(&[], after.offset())
        } else {
            after
        };
        Some(item)
    }
}

/// Reads the page that starts at `at`, whose first octets are the capture
/// pattern, and returns it with a cursor just past it. The page is charged
/// to `budget` before its checksum is computed.
fn page_at<'a>(at: Cursor<'a>, budget: &mut Budget) -> Result<(Page<'a>, Cursor<'a>), PageError> {
    let offset = at.offset();
    let available = at.remaining();
    let truncated = |needed| {
        PageError::Fault(ParseFault::Truncated {
            offset,
            needed,
            available,
        })
    };
    let mut cursor = at;
    let head: [u8; 27] = cursor.array().map_err(|_| truncated(HEADER_LEN))?;
    let [
        _,
        _,
        _,
        _,
        version,
        flags,
        g0,
        g1,
        g2,
        g3,
        g4,
        g5,
        g6,
        g7,
        s0,
        s1,
        s2,
        s3,
        n0,
        n1,
        n2,
        n3,
        c0,
        c1,
        c2,
        c3,
        segments,
    ] = head;
    if version != 0 {
        return Err(PageError::Version { offset, version });
    }
    let header_len = HEADER_LEN.saturating_add(u64::from(segments));
    let lacing = cursor
        .take(u64::from(segments))
        .map_err(|_| truncated(header_len))?;
    let body_len = lacing.iter().map(|&value| u64::from(value)).sum();
    let len = header_len.saturating_add(body_len);
    let body = cursor.take(body_len).map_err(|_| truncated(len))?;
    budget.charge(len, offset).map_err(PageError::Fault)?;
    // The checksum covers the whole page with its own field read as zeros.
    let zeroed = head
        .iter()
        .enumerate()
        .map(|(at, &octet)| if CRC_FIELD.contains(&at) { 0 } else { octet });
    let computed = crc(
        crc(crc(0, zeroed), lacing.iter().copied()),
        body.iter().copied(),
    );
    let stored = u32::from_le_bytes([c0, c1, c2, c3]);
    if computed != stored {
        return Err(PageError::Crc {
            offset,
            stored,
            computed,
        });
    }
    let granule = u64::from_le_bytes([g0, g1, g2, g3, g4, g5, g6, g7]);
    let page = Page {
        offset,
        continued: flags & CONTINUED != 0,
        first: flags & FIRST != 0,
        last: flags & LAST != 0,
        granule: (granule != NO_GRANULE).then_some(granule),
        serial: u32::from_le_bytes([s0, s1, s2, s3]),
        sequence: u32::from_le_bytes([n0, n1, n2, n3]),
        lacing,
        body,
    };
    Ok((page, cursor))
}

/// One packet of a logical stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet<'a> {
    /// Where the page the packet starts on starts, from the start of the
    /// file.
    pub offset: u64,
    /// The granule position at the end of the packet, when it is the last
    /// packet to end on its page and that page gives one.
    pub granule: Option<u64>,
    /// The packet's octets: borrowed from the input when they lie on one
    /// page, copied together when they span pages.
    pub data: Cow<'a, [u8]>,
}

/// Why a packet was lost, or what else a walk over packets met.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketError {
    /// Octets of the input that are not a page.
    Page(PageError),
    /// A page of the stream arrived out of sequence, so any packet that
    /// spans the missing pages is lost.
    Gap {
        /// Where the page that arrived starts.
        offset: u64,
        /// The sequence number the stream needed next.
        expected: u32,
        /// The sequence number the page carries.
        found: u32,
    },
    /// A packet that the next page of its stream did not continue, or that
    /// the input ended inside.
    Unfinished {
        /// Where the page the packet started on starts.
        offset: u64,
    },
    /// A page continued a packet whose start the walk did not see; the
    /// rest of that packet is skipped.
    Orphan {
        /// Where the page starts.
        offset: u64,
    },
    /// The input started more other logical streams than the children
    /// limit allows to be recorded.
    Fault(ParseFault),
}

/// Iterator over the packets of one logical stream. See [`packets`].
#[derive(Debug)]
pub struct Packets<'a, 'b> {
    /// The pages being read.
    pages: Pages<'a, 'b>,
    /// The logical stream whose packets are wanted.
    serial: u32,
    /// The limits of this walk.
    limits: Limits,
    /// The serial numbers of other streams' first pages.
    others: Vec<u32>,
    /// What is left of the stream's current page.
    page: Segments<'a>,
    /// A packet that goes on past the pages read so far.
    partial: Option<Partial<'a>>,
    /// The sequence number the stream's next page should carry.
    expected: Option<u32>,
}

/// The segments of a page not yet split into packets.
#[derive(Debug, Clone, Copy, Default)]
struct Segments<'a> {
    /// Where the page starts.
    offset: u64,
    /// The page's granule position.
    granule: Option<u64>,
    /// The lacing values not yet used.
    lacing: &'a [u8],
    /// The octets of those segments.
    body: &'a [u8],
}

impl<'a> Segments<'a> {
    /// Takes the next run of segments: up to and including the first one
    /// shorter than 255 octets, or to the end of the page. Returns its
    /// octets and whether a packet ends with it, or `None` when no segment
    /// is left.
    #[expect(
        clippy::indexing_slicing,
        reason = "count is a position in the lacing table plus one or the table's length, and the page was read with a body as long as its lacing values say"
    )]
    fn run(&mut self) -> Option<(&'a [u8], bool)> {
        if self.lacing.is_empty() {
            return None;
        }
        let end = self.lacing.iter().position(|&value| value < FULL_SEGMENT);
        let count = end.map_or(self.lacing.len(), |at| at.saturating_add(1));
        let size = self.lacing[..count]
            .iter()
            .map(|&value| usize::from(value))
            .sum();
        let octets = &self.body[..size];
        self.lacing = &self.lacing[count..];
        self.body = &self.body[size..];
        Some((octets, end.is_some()))
    }

    /// Whether another packet ends on what is left of the page.
    fn ends_later(&self) -> bool {
        self.lacing.iter().any(|&value| value < FULL_SEGMENT)
    }
}

/// A packet whose end has not been read yet.
#[derive(Debug)]
struct Partial<'a> {
    /// Where the page it started on starts.
    offset: u64,
    /// Its octets so far.
    data: Cow<'a, [u8]>,
}

impl<'a> Partial<'a> {
    /// `partial` with `octets` added, or a packet that starts with them on
    /// the page at `offset`.
    fn joined(partial: Option<Self>, offset: u64, octets: &'a [u8]) -> Self {
        match partial {
            Some(mut partial) => {
                partial.data.to_mut().extend_from_slice(octets);
                partial
            }
            None => Self {
                offset,
                data: Cow::Borrowed(octets),
            },
        }
    }
}

/// Reassembles the packets of the logical stream `serial` from the pages
/// in `input`, charging `budget`.
///
/// Pages of other streams are skipped, and the serial number of each other
/// stream whose first page the input holds is recorded, up to the
/// children limit in `limits` ([`Packets::other_streams`]). A page out of
/// sequence, a packet that is not continued where it should be, a page that
/// continues a packet the walk never saw, and every page error are
/// reported where they occur, and the walk goes on with the next packet.
pub fn packets<'a, 'b>(
    input: Cursor<'a>,
    serial: u32,
    limits: &Limits,
    budget: &'b mut Budget,
) -> Packets<'a, 'b> {
    Packets {
        pages: pages(input, budget),
        serial,
        limits: *limits,
        others: Vec::new(),
        page: Segments::default(),
        partial: None,
        expected: None,
    }
}

impl<'a> Packets<'a, '_> {
    /// The serial numbers of the other logical streams whose first page the
    /// walk has passed, in the order of those pages.
    #[must_use]
    pub fn other_streams(&self) -> &[u32] {
        &self.others
    }

    /// The next packet that ends on the current page. A packet that goes on
    /// past the page becomes the partial packet, and then there is none.
    fn next_on_page(&mut self) -> Option<Packet<'a>> {
        let (octets, ends) = self.page.run()?;
        let joined = Partial::joined(self.partial.take(), self.page.offset, octets);
        if !ends {
            self.partial = Some(joined);
            return None;
        }
        // Only the last packet to end on a page ends at its granule
        // position.
        let granule = self.page.granule.filter(|_| !self.page.ends_later());
        Some(Packet {
            offset: joined.offset,
            granule,
            data: joined.data,
        })
    }

    /// Takes `page` as the current page, or records it when it belongs to
    /// another stream. Returns what was lost or wrong on the way.
    fn begin(&mut self, page: Page<'a>) -> Option<PacketError> {
        if page.serial != self.serial {
            if page.first {
                // A vector never holds more than u64::MAX entries.
                let count = u64::try_from(self.others.len())
                    .unwrap_or(u64::MAX)
                    .saturating_add(1);
                if let Err(fault) = self.limits.check(LimitKind::Children, count, page.offset) {
                    return Some(PacketError::Fault(fault));
                }
                self.others.push(page.serial);
            }
            return None;
        }
        let gap = self
            .expected
            .filter(|&expected| expected != page.sequence)
            .map(|expected| PacketError::Gap {
                offset: page.offset,
                expected,
                found: page.sequence,
            });
        self.expected = Some(page.sequence.wrapping_add(1));
        let unfinished = self
            .partial
            .as_ref()
            .filter(|_| !page.continued)
            .map(|partial| PacketError::Unfinished {
                offset: partial.offset,
            });
        // A packet goes on only into a continued page that follows its
        // page directly.
        if gap.is_some() || !page.continued {
            self.partial = None;
        }
        self.page = Segments {
            offset: page.offset,
            granule: page.granule,
            lacing: page.lacing,
            body: page.body,
        };
        let orphan = page.continued && self.partial.is_none();
        if orphan {
            let _ = self.page.run();
        }
        gap.or(unfinished).or_else(|| {
            orphan.then_some(PacketError::Orphan {
                offset: page.offset,
            })
        })
    }
}

impl<'a> Iterator for Packets<'a, '_> {
    type Item = Result<Packet<'a>, PacketError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(packet) = self.next_on_page() {
                return Some(Ok(packet));
            }
            match self.pages.next() {
                None => {
                    return self.partial.take().map(|partial| {
                        Err(PacketError::Unfinished {
                            offset: partial.offset,
                        })
                    });
                }
                Some(Err(error)) => return Some(Err(PacketError::Page(error))),
                Some(Ok(page)) => {
                    if let Some(error) = self.begin(page) {
                        return Some(Err(error));
                    }
                }
            }
        }
    }
}

/// The granule position of the last page of the logical stream `serial`
/// in `tail` on which a packet ends, charging `budget`.
///
/// `tail` may start anywhere, even inside a page; what comes before the
/// first capture pattern is skipped. A last page with no end-of-stream
/// flag still counts, as a file cut short or still being written has none.
/// `None` means the tail holds no such page, and the caller may read
/// further back.
///
/// # Errors
///
/// Returns [`ParseFault::BudgetExceeded`] when the budget runs out.
pub fn last_granule(
    tail: Cursor<'_>,
    serial: u32,
    budget: &mut Budget,
) -> Result<Option<u64>, ParseFault> {
    let mut last = None;
    for item in pages(tail, budget) {
        match item {
            Ok(page) if page.serial == serial => last = page.granule.or(last),
            Err(PageError::Fault(fault @ ParseFault::BudgetExceeded { .. })) => return Err(fault),
            _ => {}
        }
    }
    Ok(last)
}

/// A page a seek or the packager can start from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexEntry {
    /// Where the page starts, from the start of the file.
    pub offset: u64,
    /// The granule position at the end of the last packet that ends on it.
    pub granule: u64,
}

/// The pages of one logical stream on which a packet ends, in the order
/// they were pushed, kept within the index limit.
///
/// When a page arrives that would take the index past the limit, every
/// other entry is dropped and from then on only every other page is kept,
/// so the entries stay spread over the whole stream and their number never
/// passes the limit (SEC-MED-006). [`PageIndex::stride`] says how many
/// pages each entry stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageIndex {
    /// The logical stream indexed.
    serial: u32,
    /// The most entries kept.
    max: u64,
    /// Every how many eligible pages one is kept.
    stride: u64,
    /// Eligible pages pushed so far.
    seen: u64,
    /// The pages kept.
    entries: Vec<IndexEntry>,
}

impl PageIndex {
    /// An empty index of the logical stream `serial`, kept within the index
    /// limit of `limits`.
    #[must_use]
    pub fn new(serial: u32, limits: &Limits) -> Self {
        Self {
            serial,
            max: limits.get(LimitKind::IndexEntries),
            stride: 1,
            seen: 0,
            entries: Vec::new(),
        }
    }

    /// Adds `page` when it belongs to the indexed stream and a packet ends
    /// on it, thinning the index first when it is full.
    pub fn push(&mut self, page: &Page<'_>) {
        let Some(granule) = page.granule.filter(|_| page.serial == self.serial) else {
            return;
        };
        let seen = self.seen;
        self.seen = seen.saturating_add(1);
        if self.is_full() && self.keeps(seen) {
            // Keep the entries at even positions, which are the pages on
            // the doubled stride.
            let mut keep = false;
            self.entries.retain(|_| {
                keep = !keep;
                keep
            });
            self.stride = self.stride.saturating_mul(2);
        }
        if self.keeps(seen) && !self.is_full() {
            self.entries.push(IndexEntry {
                offset: page.offset,
                granule,
            });
        }
    }

    /// Whether the index holds as many entries as the limit allows.
    fn is_full(&self) -> bool {
        // A vector never holds more than u64::MAX entries.
        u64::try_from(self.entries.len()).unwrap_or(u64::MAX) >= self.max
    }

    /// Whether the eligible page numbered `seen` falls on the stride.
    fn keeps(&self, seen: u64) -> bool {
        // The stride starts at 1 and only doubles, so it is never zero.
        seen.checked_rem(self.stride) == Some(0)
    }

    /// The pages kept, in the order they were pushed.
    #[must_use]
    pub fn entries(&self) -> &[IndexEntry] {
        &self.entries
    }

    /// How many of the stream's pages with a granule position each entry
    /// stands for: 1 until the index first thins, then doubling each time.
    #[must_use]
    pub const fn stride(&self) -> u64 {
        self.stride
    }

    /// The last entry whose granule position is at or below `granule`: the
    /// latest kept page that ends no later than that point, from which a
    /// reader can go forward to it.
    #[must_use]
    pub fn seek(&self, granule: u64) -> Option<IndexEntry> {
        let after = self
            .entries
            .partition_point(|entry| entry.granule <= granule);
        after
            .checked_sub(1)
            .and_then(|last| self.entries.get(last))
            .copied()
    }
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::*;
    use gunmetal_testkit::checksum::crc32_ogg;
    use gunmetal_testkit::ogg::{self as kit, NO_GRANULE};
    use proptest::collection::vec;
    use proptest::prelude::*;

    // -----------------------------------------------------------------------
    // Helpers. Expected values come from literals and from the testkit's
    // builder, which shares no code with the parser.
    // -----------------------------------------------------------------------

    /// The flags the testkit writes.
    const CONTINUED: u8 = kit::CONTINUED;
    const FIRST: u8 = kit::FIRST;
    const LAST: u8 = kit::LAST;

    /// The stack SEC-MED-001's property tests run on: 256 KiB.
    const SMALL_STACK: usize = 262_144;

    /// Runs `work` on a thread with a 256 KiB stack.
    fn on_small_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(SMALL_STACK)
            .spawn(work)
            .expect("the test thread starts")
            .join()
            .expect("the code under test returned instead of panicking")
    }

    fn len_of(bytes: &[u8]) -> u64 {
        u64::try_from(bytes.len()).unwrap()
    }

    /// The budget a caller gives a walk over `bytes`.
    fn budget_for(bytes: &[u8]) -> Budget {
        Budget::for_input(len_of(bytes), STEPS_PER_OCTET, 0)
    }

    /// Collects `items`, failing instead of hanging when there are more
    /// than `ceiling`.
    fn within<T>(items: impl Iterator<Item = T>, ceiling: usize) -> Vec<T> {
        let items: Vec<T> = items.take(ceiling + 1).collect();
        let count = items.len();
        assert!(count <= ceiling, "{count} items, more than {ceiling}");
        items
    }

    /// Every item [`pages`] yields from `bytes` under `budget`. Each item
    /// moves the walk on by at least one octet, so there are at most as
    /// many items as octets.
    fn walk_with<'a>(bytes: &'a [u8], budget: &mut Budget) -> Vec<Result<Page<'a>, PageError>> {
        within(pages(Cursor::new(bytes), budget), bytes.len())
    }

    /// Every item [`pages`] yields from `bytes` under the usual budget.
    fn walk(bytes: &[u8]) -> Vec<Result<Page<'_>, PageError>> {
        walk_with(bytes, &mut budget_for(bytes))
    }

    /// Every item [`packets`] yields for `serial` from `bytes`, and the
    /// other streams it recorded. A packet takes at least one lacing octet
    /// and every error at least one octet, apart from one last unfinished
    /// packet.
    fn reassemble<'a>(
        bytes: &'a [u8],
        serial: u32,
        limits: &Limits,
    ) -> (Vec<Result<Packet<'a>, PacketError>>, Vec<u32>) {
        let mut budget = budget_for(bytes);
        let mut walk = packets(Cursor::new(bytes), serial, limits, &mut budget);
        let items = within(walk.by_ref(), bytes.len() + 1);
        (items, walk.other_streams().to_vec())
    }

    /// The page the testkit wrote as `kit`, as the parser should read it at
    /// `offset`.
    fn read_as(kit: &kit::Page, offset: u64) -> Page<'_> {
        Page {
            offset,
            continued: kit.flags & CONTINUED != 0,
            first: kit.flags & FIRST != 0,
            last: kit.flags & LAST != 0,
            granule: (kit.granule != NO_GRANULE).then_some(kit.granule),
            serial: kit.serial,
            sequence: kit.sequence,
            lacing: &kit.lacing,
            body: &kit.body,
        }
    }

    /// Where each of `pages` starts once written back to back.
    fn offsets(pages: &[kit::Page]) -> Vec<u64> {
        let mut at = 0;
        pages
            .iter()
            .map(|page| {
                let offset = at;
                at += len_of(&page.to_bytes());
                offset
            })
            .collect()
    }

    /// `pages` as the parser should read them back to back.
    fn read_all(pages: &[kit::Page]) -> Vec<Result<Page<'_>, PageError>> {
        pages
            .iter()
            .zip(offsets(pages))
            .map(|(page, offset)| Ok(read_as(page, offset)))
            .collect()
    }

    /// `len` copies of `octet`, for the literal sizes the tests use.
    fn repeated(octet: u8, len: usize) -> Vec<u8> {
        (0..len).map(|_| octet).collect()
    }

    /// `len` octets counting up from `first`.
    fn counting(first: u8, len: usize) -> Vec<u8> {
        (0..len)
            .map(|n| first.wrapping_add(u8::try_from(n % 256).unwrap()))
            .collect()
    }

    /// A packet that ends on one page, borrowed from it.
    fn packet(offset: u64, granule: Option<u64>, data: &[u8]) -> Packet<'_> {
        Packet {
            offset,
            granule,
            data: Cow::Borrowed(data),
        }
    }

    /// The checksum the testkit computes for `page` with its own field
    /// zeroed.
    fn crc_of(page: &[u8]) -> u32 {
        let mut zeroed = page.to_vec();
        zeroed[22..26].fill(0);
        crc32_ogg(&zeroed)
    }

    /// Limits with `kind` lowered to `value`.
    fn lowered(kind: LimitKind, value: u64) -> Limits {
        Limits::DEFAULT.with_override(kind, value).unwrap()
    }

    /// A page of `serial` with one segment of `data` and granule `granule`.
    fn single(flags: u8, granule: u64, serial: u32, sequence: u32, data: &[u8]) -> kit::Page {
        kit::Page {
            flags,
            granule,
            serial,
            sequence,
            lacing: kit::lacing(data.len()),
            body: data.to_vec(),
        }
    }

    // -----------------------------------------------------------------------
    // The checksum.
    // -----------------------------------------------------------------------

    #[test]
    fn the_crc_table_holds_the_checksum_of_each_octet_alone() {
        let table = crc_table();
        for octet in 0..=u8::MAX {
            assert_eq!(table[usize::from(octet)], crc32_ogg(&[octet]), "{octet}");
        }
        assert_eq!(CRC_TABLE, table);
        // The first entries are the polynomial's multiples, as published.
        assert_eq!(table[..3], [0x0000_0000, 0x04C1_1DB7, 0x0982_3B6E]);
    }

    #[test]
    fn checksums_octets_as_the_testkit_does() {
        assert_eq!(crc(0, *b"123456789"), crc32_ogg(b"123456789"));
        assert_eq!(crc(0, *b"123456789"), !0x765E_7680);
        assert_eq!(crc(0, []), 0);
        // Checksumming in two parts carries the state across.
        assert_eq!(crc(crc(0, *b"1234"), *b"56789"), crc32_ogg(b"123456789"));
    }

    // -----------------------------------------------------------------------
    // Pages.
    // -----------------------------------------------------------------------

    /// The 51-octet packet on the first page `ffmpeg -c:a flac -f ogg`
    /// wrote for 64 frames of 16-bit mono silence at 8 kHz.
    const FLAC_HEADER: [u8; 51] = [
        0x7F, b'F', b'L', b'A', b'C', 0x01, 0x00, 0x00, 0x01, b'f', b'L', b'a', b'C', 0x00, 0x00,
        0x00, 0x22, 0x00, 0x40, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x95, 0x01, 0xF4, 0x00,
        0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    /// The one FLAC frame on the last page of the same file.
    const FLAC_FRAME: [u8; 12] = [
        0xFF, 0xF8, 0x64, 0x08, 0x00, 0x3F, 0x5E, 0x00, 0x00, 0x00, 0xC6, 0x3C,
    ];

    /// The first and last pages of that file, octet for octet.
    fn ffmpeg_pages() -> Vec<u8> {
        [
            &b"OggS\x00\x02"[..],
            &[0x00; 8],
            &[0x6A, 0x4C, 0xA9, 0x4C],
            &[0x00; 4],
            &[0x5E, 0xC2, 0x4E, 0x96],
            &[0x01, 0x33],
            &FLAC_HEADER,
            b"OggS\x00\x04",
            &[0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            &[0x6A, 0x4C, 0xA9, 0x4C],
            &[0x02, 0x00, 0x00, 0x00],
            &[0x1D, 0x3F, 0x6F, 0xC7],
            &[0x01, 0x0C],
            &FLAC_FRAME,
        ]
        .concat()
    }

    #[test]
    fn reads_the_pages_ffmpeg_wrote() {
        let bytes = ffmpeg_pages();
        assert_eq!(
            walk(&bytes),
            [
                Ok(Page {
                    offset: 0,
                    continued: false,
                    first: true,
                    last: false,
                    granule: Some(0),
                    serial: 0x4CA9_4C6A,
                    sequence: 0,
                    lacing: &[51],
                    body: &FLAC_HEADER,
                }),
                Ok(Page {
                    offset: 79,
                    continued: false,
                    first: false,
                    last: true,
                    granule: Some(64),
                    serial: 0x4CA9_4C6A,
                    sequence: 2,
                    lacing: &[12],
                    body: &FLAC_FRAME,
                }),
            ]
        );
    }

    #[test]
    fn reads_each_flag_and_field_on_its_own() {
        let pages = [
            single(CONTINUED, 0x0102_0304_0506_0708, 0x0A0B_0C0D, 7, &[0x11]),
            single(FIRST, NO_GRANULE - 1, 1, 0xFFFF_FFFF, &[0x22]),
            single(LAST, NO_GRANULE, 2, 0, &[]),
            // The bits the RFC leaves unused are ignored.
            single(0xF8, 0, 3, 1, &[0x33]),
            single(CONTINUED | FIRST | LAST, 5, 4, 2, &[0x44]),
        ];
        let bytes = kit::write(&pages);
        let lacing = |len: u8| -> &'static [u8] {
            match len {
                0 => &[0],
                _ => &[1],
            }
        };
        assert_eq!(
            walk(&bytes),
            [
                Ok(Page {
                    offset: 0,
                    continued: true,
                    first: false,
                    last: false,
                    granule: Some(0x0102_0304_0506_0708),
                    serial: 0x0A0B_0C0D,
                    sequence: 7,
                    lacing: lacing(1),
                    body: &[0x11],
                }),
                Ok(Page {
                    offset: 29,
                    continued: false,
                    first: true,
                    last: false,
                    granule: Some(0xFFFF_FFFF_FFFF_FFFE),
                    serial: 1,
                    sequence: 0xFFFF_FFFF,
                    lacing: lacing(1),
                    body: &[0x22],
                }),
                Ok(Page {
                    offset: 58,
                    continued: false,
                    first: false,
                    last: true,
                    granule: None,
                    serial: 2,
                    sequence: 0,
                    lacing: lacing(0),
                    body: &[],
                }),
                Ok(Page {
                    offset: 86,
                    continued: false,
                    first: false,
                    last: false,
                    granule: Some(0),
                    serial: 3,
                    sequence: 1,
                    lacing: lacing(1),
                    body: &[0x33],
                }),
                Ok(Page {
                    offset: 115,
                    continued: true,
                    first: true,
                    last: true,
                    granule: Some(5),
                    serial: 4,
                    sequence: 2,
                    lacing: lacing(1),
                    body: &[0x44],
                }),
            ]
        );
    }

    #[test]
    fn reads_a_page_with_no_segments_and_the_largest_page_there_can_be() {
        let empty = kit::Page {
            flags: 0,
            granule: 9,
            serial: 1,
            sequence: 0,
            lacing: vec![],
            body: vec![],
        };
        // 255 segments of 255 octets: 27 + 255 + 65,025 octets.
        let largest = kit::Page {
            flags: 0,
            granule: 10,
            serial: 1,
            sequence: 1,
            lacing: repeated(255, 255),
            body: counting(0, 65_025),
        };
        let pages = [empty, largest];
        let bytes = kit::write(&pages);
        assert_eq!(bytes.len(), 27 + 65_307);
        assert_eq!(walk(&bytes), read_all(&pages));
    }

    #[test]
    fn reads_nothing_from_nothing() {
        assert_eq!(walk(&[]), []);
    }

    #[test]
    fn reports_where_the_input_starts_in_the_file() {
        let page = single(FIRST, 1, 5, 0, &[0x0A]);
        let bytes = page.to_bytes();
        let mut budget = budget_for(&bytes);
        let read = within(pages(Cursor::at(&bytes, 1_000), &mut budget), bytes.len());
        assert_eq!(read, [Ok(read_as(&page, 1_000))]);
    }

    #[test]
    fn reports_octets_before_between_and_after_pages() {
        let first = single(FIRST, 1, 5, 0, &[0x0A]);
        let second = single(LAST, 2, 5, 1, &[0x0B, 0x0C]);
        // An ID3v2 header before the stream, three stray octets between
        // the pages, and a capture pattern cut short at the end.
        let bytes = [
            &b"ID3\x04\x00\x00\x00\x00\x00\x00"[..],
            &first.to_bytes(),
            &[0x00, 0x4F, 0x67],
            &second.to_bytes(),
            b"Ogg",
        ]
        .concat();
        assert_eq!(
            walk(&bytes),
            [
                Err(PageError::Skipped { offset: 0, len: 10 }),
                Ok(read_as(&first, 10)),
                Err(PageError::Skipped { offset: 39, len: 3 }),
                Ok(read_as(&second, 42)),
                // The input may end inside a page's capture pattern.
                Err(PageError::Fault(ParseFault::Truncated {
                    offset: 72,
                    needed: 27,
                    available: 3,
                })),
                Err(PageError::Skipped { offset: 73, len: 2 }),
            ]
        );
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn refuses_a_page_whose_checksum_does_not_match_and_moves_on() {
        let pages = kit::paginate(
            3,
            &[
                (repeated(0x01, 10), 10),
                (repeated(0x02, 20), 20),
                (repeated(0x03, 5), 30),
            ],
            1,
        );
        // Pages of 38, 48 and 33 octets.
        let mut bytes = kit::write(&pages);
        assert_eq!(bytes.len(), 38 + 48 + 33);
        // One octet of the second page's body changes.
        bytes[38 + 28] ^= 0x80;
        let stored = u32::from_le_bytes(bytes[38 + 22..38 + 26].try_into().unwrap());
        let computed = crc_of(&bytes[38..38 + 48]);
        assert_ne!(stored, computed);
        assert_eq!(
            walk(&bytes),
            [
                Ok(read_as(&pages[0], 0)),
                Err(PageError::Crc {
                    offset: 38,
                    stored,
                    computed,
                }),
                Err(PageError::Skipped {
                    offset: 39,
                    len: 47,
                }),
                Ok(read_as(&pages[2], 86)),
            ]
        );
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn refuses_a_page_of_another_version() {
        let page = single(FIRST, 1, 1, 0, &[0x05; 3]);
        for version in [1, 0xFF] {
            let mut bytes = page.to_bytes();
            bytes[4] = version;
            assert_eq!(
                walk(&bytes),
                [
                    Err(PageError::Version { offset: 0, version }),
                    Err(PageError::Skipped { offset: 1, len: 30 }),
                ]
            );
        }
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn reports_a_page_cut_short_with_the_octets_it_needs() {
        let page = kit::Page {
            flags: FIRST,
            granule: 1,
            serial: 1,
            sequence: 0,
            lacing: vec![255, 45],
            body: counting(0, 300),
        };
        let bytes = page.to_bytes();
        assert_eq!(bytes.len(), 27 + 2 + 300);
        let truncated = |needed, available| {
            Err(PageError::Fault(ParseFault::Truncated {
                offset: 0,
                needed,
                available,
            }))
        };
        // Inside the capture pattern, then the rest of the input is skipped.
        assert_eq!(walk(&bytes[..1]), [truncated(27, 1)]);
        // Part of the capture pattern, the capture pattern alone, the fixed
        // header short by one, the segment table short by one, and the body
        // short by one.
        for (cut, needed) in [(2, 27), (3, 27), (4, 27), (26, 27), (28, 29), (328, 329)] {
            assert_eq!(
                walk(&bytes[..cut]),
                [
                    truncated(needed, len_of(&bytes[..cut])),
                    Err(PageError::Skipped {
                        offset: 1,
                        len: len_of(&bytes[1..cut]),
                    }),
                ],
                "cut at {cut}"
            );
        }
    }

    #[test]
    fn does_not_split_a_page_at_a_capture_pattern_inside_its_data() {
        // A packet that holds a whole page of another stream, and a capture
        // pattern followed by junk.
        let inner = single(FIRST | LAST, 4, 99, 0, b"inner").to_bytes();
        let data = [&b"before OggS\x00\x07junk "[..], &inner, b" after"].concat();
        let pages = kit::paginate(1, &[(data.clone(), 50), (vec![0x0D], 60)], 255);
        let bytes = kit::write(&pages);
        assert_eq!(walk(&bytes), read_all(&pages));
        assert_eq!(
            reassemble(&bytes, 1, &Limits::DEFAULT),
            (
                vec![Ok(packet(0, None, &data)), Ok(packet(0, Some(60), &[0x0D])),],
                vec![]
            )
        );
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn charges_one_step_per_octet_of_a_well_formed_stream() {
        let pages = kit::paginate(2, &[(repeated(0x01, 3), 1), (repeated(0x02, 4), 2)], 1);
        let bytes = kit::write(&pages);
        let len = len_of(&bytes);
        let mut budget = Budget::for_input(len, 1, 0);
        assert_eq!(walk_with(&bytes, &mut budget), read_all(&pages));
        assert_eq!(budget.remaining(), 0);
        // One step fewer, and the last page cannot be checked.
        let mut budget = Budget::for_input(len - 1, 1, 0);
        assert_eq!(
            walk_with(&bytes, &mut budget),
            [
                Ok(read_as(&pages[0], 0)),
                Err(PageError::Fault(ParseFault::BudgetExceeded { offset: 31 })),
            ]
        );
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn charges_skipped_octets_and_stops_when_they_run_out() {
        let page = single(0, 1, 1, 0, &[0x01]);
        let bytes = [&[0x00; 10][..], &page.to_bytes()].concat();
        let mut budget = Budget::for_input(9, 1, 0);
        assert_eq!(
            walk_with(&bytes, &mut budget),
            [Err(PageError::Fault(ParseFault::BudgetExceeded {
                offset: 0
            }))]
        );
        let mut budget = Budget::for_input(10, 1, 0);
        assert_eq!(
            walk_with(&bytes, &mut budget),
            [
                Err(PageError::Skipped { offset: 0, len: 10 }),
                Err(PageError::Fault(ParseFault::BudgetExceeded { offset: 10 })),
            ]
        );
        assert_eq!(budget.remaining(), 0);
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn a_damaged_page_costs_at_most_two_steps_per_octet() {
        let mut bytes = single(0, 1, 1, 0, &[0x01; 10]).to_bytes();
        bytes[30] ^= 1;
        let mut budget = budget_for(&bytes);
        // 38 octets: 38 steps to check the page, then one to move past its
        // capture pattern and 37 to skip the rest.
        assert_eq!(walk_with(&bytes, &mut budget).len(), 2);
        assert_eq!(budget.remaining(), 2 * 38 - 38 - 37);
        // A damaged header that is never checked costs one step to move
        // past, and the rest is skipped.
        bytes[4] = 1;
        let mut budget = budget_for(&bytes);
        assert_eq!(walk_with(&bytes, &mut budget).len(), 2);
        assert_eq!(budget.remaining(), 2 * 38 - 1 - 37);
    }

    /// Verifies: SEC-MED-007, SEC-TM-032
    #[test]
    fn stops_overlapping_false_pages_with_the_budget() {
        // Ten false page headers, one every 100 octets, each declaring a
        // 1,000-octet body that reaches over the next nine. Each one's
        // checksum is computed and fails, then the octets up to the next
        // are skipped: 1,031 + 99 steps for every 100 octets.
        let mut bytes = repeated(0_u8, 900 + 1_031);
        for start in (0..1_000).step_by(100) {
            let header = [
                &b"OggS\x00\x00"[..],
                &[0; 8],
                &[0; 4],
                &[0; 4],
                &[0; 4],
                &[4, 255, 255, 255, 235],
            ]
            .concat();
            bytes[start..start + 31].copy_from_slice(&header);
        }
        let crc_at = |start: usize| PageError::Crc {
            offset: len_of(&bytes[..start]),
            stored: 0,
            computed: crc_of(&bytes[start..start + 1_031]),
        };
        assert_eq!(
            walk(&bytes),
            [
                Err(crc_at(0)),
                Err(PageError::Skipped { offset: 1, len: 99 }),
                Err(crc_at(100)),
                Err(PageError::Skipped {
                    offset: 101,
                    len: 99
                }),
                Err(crc_at(200)),
                Err(PageError::Skipped {
                    offset: 201,
                    len: 99
                }),
                // 3,390 of 3,862 steps are spent, and the fourth needs 1,031.
                Err(PageError::Fault(ParseFault::BudgetExceeded { offset: 300 })),
            ]
        );
    }

    /// Pages the testkit writes from arbitrary fields, with bodies that
    /// match their segment tables.
    fn any_kit_pages() -> impl Strategy<Value = Vec<kit::Page>> {
        vec(
            (
                any::<u8>(),
                prop_oneof![Just(NO_GRANULE), any::<u64>()],
                any::<u32>(),
                any::<u32>(),
                vec(prop_oneof![Just(255_u8), any::<u8>()], 0..8),
            )
                .prop_map(|(flags, granule, serial, sequence, lacing)| {
                    let len = lacing.iter().map(|&value| usize::from(value)).sum();
                    kit::Page {
                        flags,
                        granule,
                        serial,
                        sequence,
                        lacing,
                        body: counting(flags, len),
                    }
                }),
            0..5,
        )
    }

    /// Arbitrary octets with whole pages, capture patterns and version
    /// octets planted often enough that the walk meets good, false and
    /// damaged pages. Planted pages belong to stream 0 or 1.
    fn any_octets() -> impl Strategy<Value = Vec<u8>> {
        vec(
            prop_oneof![
                8 => vec(any::<u8>(), 1..4),
                2 => Just(b"OggS\x00".to_vec()),
                2 => Just(vec![255]),
                1 => any_kit_pages().prop_map(|pages| {
                    let pages: Vec<kit::Page> = pages
                        .into_iter()
                        .map(|page| kit::Page {
                            serial: page.serial % 2,
                            ..page
                        })
                        .collect();
                    kit::write(&pages)
                }),
            ],
            0..64,
        )
        .prop_map(|parts| parts.concat())
    }

    proptest! {
        /// Verifies: SEC-MED-007
        #[test]
        fn reads_back_every_page_the_testkit_writes_for_one_step_an_octet(
            pages in any_kit_pages(),
        ) {
            let bytes = kit::write(&pages);
            let mut budget = Budget::for_input(len_of(&bytes), 1, 0);
            prop_assert_eq!(walk_with(&bytes, &mut budget), read_all(&pages));
            prop_assert_eq!(budget.remaining(), 0);
        }

        /// Verifies: SEC-MED-001, SEC-TM-032
        #[test]
        fn a_stream_cut_anywhere_yields_its_whole_pages_then_the_cut_one(
            pages in any_kit_pages().prop_filter("at least one page", |pages| !pages.is_empty()),
            cut_seed in any::<usize>(),
        ) {
            let bytes = kit::write(&pages);
            let cut = cut_seed % bytes.len();
            let starts = offsets(&pages);
            // The page the cut falls inside, worked out from the testkit's
            // lengths: every page before it is whole.
            let inside = starts.iter().rposition(|&start| start <= len_of(&bytes[..cut])).unwrap();
            let whole: Vec<_> = read_all(&pages).into_iter().take(inside).collect();
            let read = walk(&bytes[..cut]);
            prop_assert_eq!(&read[..inside], &whole[..]);
            let start = starts[inside];
            if start < len_of(&bytes[..cut]) {
                // What the cut page needs: its fixed header, then its
                // segment table, then all of it.
                let present = len_of(&bytes[..cut]) - start;
                let table = 27 + len_of(&pages[inside].lacing);
                let needed = if present < 27 {
                    27
                } else if present < table {
                    table
                } else {
                    len_of(&pages[inside].to_bytes())
                };
                prop_assert_eq!(
                    read.get(inside),
                    Some(&Err(PageError::Fault(ParseFault::Truncated {
                        offset: start,
                        needed,
                        available: len_of(&bytes[..cut]) - start,
                    })))
                );
                // Nothing after the cut page is a page.
                prop_assert!(read[inside + 1..].iter().all(Result::is_err));
            } else {
                prop_assert_eq!(read.len(), inside);
            }
        }

        /// Verifies: SEC-MED-001, SEC-MED-007, SEC-MED-008, SEC-HIS-036
        #[test]
        fn returns_and_moves_on_for_any_input(bytes in any_octets()) {
            let read = on_small_stack({
                let bytes = bytes.clone();
                move || {
                    let read: Vec<Result<(u64, u64), PageError>> = walk(&bytes)
                        .into_iter()
                        .map(|item| item.map(|page| {
                            assert_eq!(
                                page.body.len(),
                                page.lacing.iter().map(|&value| usize::from(value)).sum::<usize>()
                            );
                            (page.offset, 27 + len_of(page.lacing) + len_of(page.body))
                        }))
                        .collect();
                    read
                }
            });
            // Each item starts exactly where the one before left off: a
            // page or skipped octets after themselves, anything else one
            // octet on. Unless the budget ran out, they cover the input.
            let mut next = 0;
            for item in &read {
                let (offset, advance) = match *item {
                    Ok(span) => span,
                    Err(PageError::Skipped { offset, len }) => (offset, len),
                    Err(PageError::Version { offset, .. } | PageError::Crc { offset, .. }) => (offset, 1),
                    Err(PageError::Fault(fault)) => (fault.offset(), 1),
                };
                prop_assert_eq!(offset, next);
                prop_assert!(advance >= 1);
                next = offset + advance;
            }
            let spent = matches!(
                read.last(),
                Some(Err(PageError::Fault(ParseFault::BudgetExceeded { .. })))
            );
            prop_assert!(spent || next == len_of(&bytes));
        }
    }

    // -----------------------------------------------------------------------
    // Packets.
    // -----------------------------------------------------------------------

    #[test]
    fn reassembles_a_packet_split_over_three_pages() {
        let data = counting(0, 600);
        let pages = kit::paginate(7, &[(data.clone(), 1_000), (vec![0x01], 1_001)], 1);
        assert_eq!(pages.len(), 4);
        let bytes = kit::write(&pages);
        let (read, others) = reassemble(&bytes, 7, &Limits::DEFAULT);
        assert_eq!(
            read,
            [
                Ok(Packet {
                    offset: 0,
                    granule: Some(1_000),
                    data: Cow::Owned(data),
                }),
                Ok(packet(28 + 255 + 28 + 255 + 28 + 90, Some(1_001), &[0x01])),
            ]
        );
        assert_eq!(others, []);
    }

    #[test]
    fn ends_a_packet_of_exactly_255_octets_with_an_empty_segment() {
        let data = counting(5, 255);
        // On one page, the lacing values are 255 and 0.
        let pages = kit::paginate(1, &[(data.clone(), 9), (vec![0xEE], 10)], 255);
        assert_eq!(pages[0].lacing, [255, 0, 1]);
        let bytes = kit::write(&pages);
        assert_eq!(
            reassemble(&bytes, 1, &Limits::DEFAULT).0,
            [Ok(packet(0, None, &data)), Ok(packet(0, Some(10), &[0xEE]))]
        );
        // With one segment a page, the empty segment ends it on the next.
        let pages = kit::paginate(1, &[(data.clone(), 9), (vec![0xEE], 10)], 1);
        let bytes = kit::write(&pages);
        assert_eq!(
            reassemble(&bytes, 1, &Limits::DEFAULT).0,
            [
                Ok(Packet {
                    offset: 0,
                    granule: Some(9),
                    data: Cow::Owned(data),
                }),
                Ok(packet(28 + 255 + 28, Some(10), &[0xEE])),
            ]
        );
    }

    #[test]
    fn reads_empty_packets() {
        let pages = kit::paginate(1, &[(vec![], 1), (vec![], 2)], 255);
        let bytes = kit::write(&pages);
        assert_eq!(
            reassemble(&bytes, 1, &Limits::DEFAULT).0,
            [Ok(packet(0, None, &[])), Ok(packet(0, Some(2), &[]))]
        );
    }

    #[test]
    fn gives_a_packet_a_granule_only_when_it_is_the_last_to_end_on_its_page() {
        // Two packets end on the first page; the third starts there and
        // ends on the second, which no other packet ends on.
        let pages = [
            kit::Page {
                flags: FIRST,
                granule: 20,
                serial: 4,
                sequence: 0,
                lacing: vec![1, 2, 255],
                body: counting(1, 258),
            },
            kit::Page {
                flags: CONTINUED | LAST,
                granule: 30,
                serial: 4,
                sequence: 1,
                lacing: vec![3],
                body: vec![0xA0, 0xA1, 0xA2],
            },
        ];
        let bytes = kit::write(&pages);
        let third = [&counting(4, 255)[..], &[0xA0, 0xA1, 0xA2]].concat();
        assert_eq!(
            reassemble(&bytes, 4, &Limits::DEFAULT).0,
            [
                Ok(packet(0, None, &[0x01])),
                Ok(packet(0, Some(20), &[0x02, 0x03])),
                Ok(Packet {
                    offset: 0,
                    granule: Some(30),
                    data: Cow::Owned(third),
                }),
            ]
        );
    }

    #[test]
    fn gives_no_granule_where_the_page_says_none() {
        // A page on which a packet ends but which says −1, as a broken
        // muxer might write.
        let pages = [single(FIRST, NO_GRANULE, 1, 0, &[0x01])];
        let bytes = kit::write(&pages);
        assert_eq!(
            reassemble(&bytes, 1, &Limits::DEFAULT).0,
            [Ok(packet(0, None, &[0x01]))]
        );
    }

    /// Two streams' packets laid out on pages, and those pages
    /// interleaved: the first page of each, then the rest in turn.
    fn interleaved() -> (Vec<kit::Page>, Vec<Vec<u8>>, Vec<Vec<u8>>) {
        let audio: Vec<Vec<u8>> = vec![counting(0, 300), repeated(0x01, 4), counting(9, 40)];
        let other: Vec<Vec<u8>> = vec![repeated(0x02, 3), counting(3, 260)];
        let laid_out = |serial, packets: &[Vec<u8>]| {
            let packets: Vec<_> = packets
                .iter()
                .zip(1..)
                .map(|(data, granule)| (data.clone(), granule))
                .collect();
            kit::paginate(serial, &packets, 1)
        };
        let audio_pages = laid_out(10, &audio);
        let other_pages = laid_out(20, &other);
        let mut pages = Vec::new();
        let mut audio_pages = audio_pages.into_iter();
        let mut other_pages = other_pages.into_iter();
        loop {
            match (audio_pages.next(), other_pages.next()) {
                (None, None) => break,
                (a, o) => pages.extend(a.into_iter().chain(o)),
            }
        }
        (pages, audio, other)
    }

    #[test]
    fn follows_one_of_two_interleaved_streams() {
        let (pages, audio, other) = interleaved();
        let bytes = kit::write(&pages);
        let starts = offsets(&pages);
        let serials: Vec<u32> = pages.iter().map(|page| page.serial).collect();
        assert_eq!(serials, [10, 20, 10, 20, 10, 20, 10]);
        assert_eq!(
            reassemble(&bytes, 10, &Limits::DEFAULT),
            (
                vec![
                    Ok(Packet {
                        offset: starts[0],
                        granule: Some(1),
                        data: Cow::Owned(audio[0].clone()),
                    }),
                    Ok(packet(starts[4], Some(2), &audio[1])),
                    Ok(packet(starts[6], Some(3), &audio[2])),
                ],
                vec![20]
            )
        );
        assert_eq!(
            reassemble(&bytes, 20, &Limits::DEFAULT),
            (
                vec![
                    Ok(packet(starts[1], Some(1), &other[0])),
                    Ok(Packet {
                        offset: starts[3],
                        granule: Some(2),
                        data: Cow::Owned(other[1].clone()),
                    }),
                ],
                vec![10]
            )
        );
        // A stream the input does not hold yields nothing and records both.
        assert_eq!(
            reassemble(&bytes, 30, &Limits::DEFAULT),
            (vec![], vec![10, 20])
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn records_other_streams_up_to_the_children_limit() {
        let pages = [
            single(FIRST, 1, 1, 0, &[0x01]),
            single(FIRST, 1, 2, 0, &[0x02]),
            single(FIRST, 1, 3, 0, &[0x03]),
            single(FIRST, 1, 4, 0, &[0x04]),
            single(0, 2, 5, 1, &[0x05]),
            single(FIRST, 1, 6, 0, &[0x06]),
        ];
        let bytes = kit::write(&pages);
        let limits = lowered(LimitKind::Children, 2);
        assert_eq!(
            reassemble(&bytes, 1, &limits),
            (
                vec![
                    Ok(packet(0, Some(1), &[0x01])),
                    Err(PacketError::Fault(ParseFault::LimitExceeded {
                        limit: LimitKind::Children,
                        value: 3,
                        max: 2,
                        offset: 87,
                    })),
                    Err(PacketError::Fault(ParseFault::LimitExceeded {
                        limit: LimitKind::Children,
                        value: 3,
                        max: 2,
                        offset: 145,
                    })),
                ],
                vec![2, 3]
            )
        );
        // At the limit, every other stream is recorded.
        let limits = lowered(LimitKind::Children, 4);
        assert_eq!(reassemble(&bytes, 1, &limits).1, [2, 3, 4, 6]);
    }

    #[test]
    fn reports_a_lost_page_as_a_gap() {
        let data = counting(0, 600);
        let pages = kit::paginate(7, &[(data, 1), (vec![0x01], 2)], 1);
        // The middle page of the first packet is lost.
        let kept = [pages[0].clone(), pages[2].clone(), pages[3].clone()];
        let bytes = kit::write(&kept);
        assert_eq!(
            reassemble(&bytes, 7, &Limits::DEFAULT).0,
            [
                Err(PacketError::Gap {
                    offset: 283,
                    expected: 1,
                    found: 2,
                }),
                Ok(packet(283 + 118, Some(2), &[0x01])),
            ]
        );
    }

    #[test]
    fn reports_a_gap_between_whole_packets() {
        let pages = [
            single(FIRST, 1, 7, 0, &[0x01]),
            single(0, 2, 7, 5, &[0x02]),
            single(LAST, 3, 7, 6, &[0x03]),
        ];
        let bytes = kit::write(&pages);
        assert_eq!(
            reassemble(&bytes, 7, &Limits::DEFAULT).0,
            [
                Ok(packet(0, Some(1), &[0x01])),
                Err(PacketError::Gap {
                    offset: 29,
                    expected: 1,
                    found: 5,
                }),
                Ok(packet(29, Some(2), &[0x02])),
                Ok(packet(58, Some(3), &[0x03])),
            ]
        );
    }

    #[test]
    fn counts_sequence_numbers_past_their_largest_value() {
        let pages = [
            single(0, 1, 7, u32::MAX, &[0x01]),
            single(0, 2, 7, 0, &[0x02]),
        ];
        let bytes = kit::write(&pages);
        assert_eq!(
            reassemble(&bytes, 7, &Limits::DEFAULT).0,
            [
                Ok(packet(0, Some(1), &[0x01])),
                Ok(packet(29, Some(2), &[0x02])),
            ]
        );
    }

    #[test]
    fn reports_a_continued_page_with_no_packet_to_continue() {
        let data = counting(0, 600);
        let pages = kit::paginate(7, &[(data, 1), (vec![0x01], 2)], 1);
        // The walk starts at the second page, inside the first packet.
        let bytes = kit::write(&pages[1..]);
        assert_eq!(
            reassemble(&bytes, 7, &Limits::DEFAULT).0,
            [
                Err(PacketError::Orphan { offset: 0 }),
                Err(PacketError::Orphan { offset: 283 }),
                Ok(packet(283 + 118, Some(2), &[0x01])),
            ]
        );
    }

    #[test]
    fn skips_only_the_orphaned_part_of_a_page() {
        let pages = [kit::Page {
            flags: CONTINUED,
            granule: 8,
            serial: 7,
            sequence: 3,
            lacing: vec![2, 1],
            body: vec![0xAA, 0xAB, 0xBB],
        }];
        let bytes = kit::write(&pages);
        assert_eq!(
            reassemble(&bytes, 7, &Limits::DEFAULT).0,
            [
                Err(PacketError::Orphan { offset: 0 }),
                Ok(packet(0, Some(8), &[0xBB])),
            ]
        );
    }

    #[test]
    fn reports_a_packet_the_next_page_does_not_continue() {
        let pages = [
            kit::Page {
                flags: FIRST,
                granule: NO_GRANULE,
                serial: 7,
                sequence: 0,
                lacing: vec![1, 255],
                body: counting(0, 256),
            },
            single(LAST, 5, 7, 1, &[0x0F]),
        ];
        let bytes = kit::write(&pages);
        assert_eq!(
            reassemble(&bytes, 7, &Limits::DEFAULT).0,
            [
                Ok(packet(0, None, &[0x00])),
                Err(PacketError::Unfinished { offset: 0 }),
                Ok(packet(285, Some(5), &[0x0F])),
            ]
        );
    }

    #[test]
    fn reports_a_packet_the_input_ends_inside() {
        let pages = kit::paginate(7, &[(counting(0, 600), 1)], 1);
        let bytes = kit::write(&pages[..2]);
        assert_eq!(
            reassemble(&bytes, 7, &Limits::DEFAULT).0,
            [Err(PacketError::Unfinished { offset: 0 })]
        );
    }

    #[test]
    fn carries_a_packet_across_a_page_with_no_segments() {
        let pages = [
            kit::Page {
                flags: FIRST,
                granule: NO_GRANULE,
                serial: 7,
                sequence: 0,
                lacing: vec![255],
                body: counting(0, 255),
            },
            kit::Page {
                flags: CONTINUED,
                granule: NO_GRANULE,
                serial: 7,
                sequence: 1,
                lacing: vec![],
                body: vec![],
            },
            single(CONTINUED | LAST, 3, 7, 2, &[0x10]),
        ];
        let bytes = kit::write(&pages);
        assert_eq!(
            reassemble(&bytes, 7, &Limits::DEFAULT).0,
            [Ok(Packet {
                offset: 0,
                granule: Some(3),
                data: Cow::Owned([&counting(0, 255)[..], &[0x10]].concat()),
            })]
        );
    }

    #[test]
    fn passes_page_errors_on_and_loses_the_packet_they_broke() {
        let pages = kit::paginate(7, &[(counting(0, 600), 1), (vec![0x01], 2)], 1);
        let mut bytes = kit::write(&pages);
        // The second page's checksum fails.
        bytes[283 + 30] ^= 1;
        let stored = u32::from_le_bytes(bytes[283 + 22..283 + 26].try_into().unwrap());
        let computed = crc_of(&bytes[283..283 + 283]);
        assert_eq!(
            reassemble(&bytes, 7, &Limits::DEFAULT).0,
            [
                Err(PacketError::Page(PageError::Crc {
                    offset: 283,
                    stored,
                    computed,
                })),
                Err(PacketError::Page(PageError::Skipped {
                    offset: 284,
                    len: 282,
                })),
                Err(PacketError::Gap {
                    offset: 566,
                    expected: 1,
                    found: 2,
                }),
                Ok(packet(566 + 118, Some(2), &[0x01])),
            ]
        );
    }

    /// Verifies: SEC-MED-005
    #[test]
    fn carries_a_packet_over_thousands_of_pages_without_recursing() {
        let data = counting(0, 255 * 2_000);
        let pages = kit::paginate(1, &[(data.clone(), 77)], 1);
        assert_eq!(pages.len(), 2_001);
        let bytes = kit::write(&pages);
        let read = on_small_stack(move || {
            let (read, _) = reassemble(&bytes, 1, &Limits::DEFAULT);
            read.into_iter()
                .map(|item| item.map(|packet| (packet.granule, packet.data.into_owned())))
                .collect::<Vec<_>>()
        });
        assert_eq!(read, [Ok((Some(77), data))]);
    }

    proptest! {
        /// Verifies: SEC-MED-001, SEC-MED-008
        #[test]
        fn reassembles_every_packet_the_testkit_lays_out(
            packets in vec((vec(any::<u8>(), 0..700), 0_u64..NO_GRANULE), 0..6),
            max_segments in prop_oneof![1_u8..4, 1_u8..=255],
            serial in any::<u32>(),
        ) {
            let pages = kit::paginate(serial, &packets, max_segments);
            let bytes = kit::write(&pages);
            let starts = offsets(&pages);
            // Independently of both: where each packet's first and last
            // segments fall.
            let per_page = usize::from(max_segments);
            let mut segment = 0;
            let mut spans = Vec::new();
            for (data, _) in &packets {
                let count = data.len() / 255 + 1;
                spans.push((segment / per_page, (segment + count - 1) / per_page));
                segment += count;
            }
            let expected: Vec<Result<Packet<'_>, PacketError>> = packets
                .iter()
                .enumerate()
                .map(|(index, (data, granule))| {
                    let (first_page, last_page) = spans[index];
                    let last_on_page = spans.get(index + 1).is_none_or(|next| next.1 != last_page);
                    Ok(Packet {
                        offset: starts[first_page],
                        granule: last_on_page.then_some(*granule),
                        data: Cow::Borrowed(&data[..]),
                    })
                })
                .collect();
            let (read, others) = reassemble(&bytes, serial, &Limits::DEFAULT);
            prop_assert_eq!(read, expected);
            prop_assert_eq!(others, Vec::<u32>::new());
        }

        /// Verifies: SEC-MED-001, SEC-MED-006, SEC-MED-008, SEC-HIS-036
        #[test]
        fn every_walk_returns_for_any_input(
            bytes in any_octets(),
            serial in 0_u32..3,
        ) {
            let limits = lowered(LimitKind::Children, 2);
            let (packets, others, last) = on_small_stack({
                let bytes = bytes.clone();
                move || {
                    let (read, others) = reassemble(&bytes, serial, &limits);
                    let packets: Vec<Result<usize, PacketError>> = read
                        .into_iter()
                        .map(|item| item.map(|packet| packet.data.len()))
                        .collect();
                    let last = last_granule(Cursor::new(&bytes), serial, &mut budget_for(&bytes));
                    (packets, others, last)
                }
            });
            let octets: usize = packets.iter().flatten().sum();
            prop_assert!(octets <= bytes.len());
            prop_assert!(others.len() <= 2);
            let returned = matches!(last, Ok(_) | Err(ParseFault::BudgetExceeded { .. }));
            prop_assert!(returned);
        }
    }

    // -----------------------------------------------------------------------
    // The last granule position.
    // -----------------------------------------------------------------------

    /// A stream of `count` one-packet pages of `serial`, with granule
    /// positions 100, 200 and so on.
    fn numbered(serial: u32, count: u32) -> Vec<kit::Page> {
        (0..count)
            .map(|n| single(0, u64::from(n + 1) * 100, serial, n, &counting(0, 40)))
            .collect()
    }

    fn last_of(tail: &[u8], serial: u32) -> Result<Option<u64>, ParseFault> {
        last_granule(Cursor::at(tail, 5_000), serial, &mut budget_for(tail))
    }

    #[test]
    fn reads_the_granule_of_a_last_page_with_no_end_of_stream_flag() {
        let pages = numbered(3, 4);
        assert!(pages.iter().all(|page| page.flags & LAST == 0));
        let bytes = kit::write(&pages);
        assert_eq!(last_of(&bytes, 3), Ok(Some(400)));
    }

    #[test]
    fn finds_the_last_page_from_a_tail_that_starts_inside_another() {
        // The tail starts inside a page whose data holds a capture pattern
        // and a false header.
        let data = [&counting(0, 20)[..], b"OggS\x00\x00", &[0; 30]].concat();
        let pages = [single(0, 7, 3, 0, &data), single(LAST, 9, 3, 1, &[0x01])];
        let bytes = kit::write(&pages);
        assert_eq!(last_of(&bytes[30..], 3), Ok(Some(9)));
        assert_eq!(last_of(&bytes[1..], 3), Ok(Some(9)));
        assert_eq!(last_of(&bytes, 3), Ok(Some(9)));
    }

    #[test]
    fn passes_over_a_last_page_on_which_no_packet_ends() {
        let mut pages = numbered(3, 3);
        pages[2].granule = NO_GRANULE;
        let bytes = kit::write(&pages);
        assert_eq!(last_of(&bytes, 3), Ok(Some(200)));
    }

    #[test]
    fn passes_over_other_streams_and_damaged_pages() {
        let mut pages = numbered(3, 2);
        pages.push(single(0, 999, 4, 0, &[0x01]));
        let mut bytes = kit::write(&pages);
        // A last page of the stream, damaged.
        let damaged = single(0, 888, 3, 2, &[0x02]).to_bytes();
        bytes.extend_from_slice(&damaged[..damaged.len() - 1]);
        assert_eq!(last_of(&bytes, 3), Ok(Some(200)));
        assert_eq!(last_of(&bytes, 4), Ok(Some(999)));
    }

    #[test]
    fn reports_no_granule_when_the_tail_holds_no_page_of_the_stream() {
        assert_eq!(last_of(&[], 3), Ok(None));
        assert_eq!(last_of(&counting(0, 100), 3), Ok(None));
        let bytes = kit::write(&numbered(3, 2));
        assert_eq!(last_of(&bytes, 4), Ok(None));
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn stops_reading_the_tail_when_the_budget_runs_out() {
        let bytes = kit::write(&numbered(3, 3));
        let mut budget = Budget::for_input(len_of(&bytes) - 1, 1, 0);
        assert_eq!(
            last_granule(Cursor::at(&bytes, 5_000), 3, &mut budget),
            Err(ParseFault::BudgetExceeded {
                offset: 5_000 + 136
            })
        );
    }

    // -----------------------------------------------------------------------
    // The page index.
    // -----------------------------------------------------------------------

    /// A page of `serial` at `offset` that ends at `granule`.
    fn ending(offset: u64, serial: u32, granule: Option<u64>) -> Page<'static> {
        Page {
            offset,
            continued: false,
            first: false,
            last: false,
            granule,
            serial,
            sequence: 0,
            lacing: &[],
            body: &[],
        }
    }

    fn entry(offset: u64, granule: u64) -> IndexEntry {
        IndexEntry { offset, granule }
    }

    #[test]
    fn indexes_the_pages_of_its_stream_on_which_a_packet_ends() {
        let mut index = PageIndex::new(3, &Limits::DEFAULT);
        assert_eq!((index.entries(), index.stride()), (&[][..], 1));
        index.push(&ending(0, 3, Some(0)));
        index.push(&ending(50, 3, None));
        index.push(&ending(90, 4, Some(500)));
        index.push(&ending(120, 3, Some(960)));
        assert_eq!(index.entries(), [entry(0, 0), entry(120, 960)]);
        assert_eq!(index.stride(), 1);
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn keeps_every_page_up_to_the_index_limit_and_thins_past_it() {
        let mut index = PageIndex::new(1, &lowered(LimitKind::IndexEntries, 3));
        for n in 0..3 {
            index.push(&ending(n * 10, 1, Some(n * 100)));
        }
        assert_eq!(
            (index.entries(), index.stride()),
            (&[entry(0, 0), entry(10, 100), entry(20, 200)][..], 1)
        );
        // The fourth page would pass the limit: every other entry goes,
        // and from now on every other page is kept.
        index.push(&ending(30, 1, Some(300)));
        assert_eq!(
            (index.entries(), index.stride()),
            (&[entry(0, 0), entry(20, 200)][..], 2)
        );
        index.push(&ending(40, 1, Some(400)));
        index.push(&ending(50, 1, Some(500)));
        assert_eq!(
            (index.entries(), index.stride()),
            (&[entry(0, 0), entry(20, 200), entry(40, 400)][..], 2)
        );
        index.push(&ending(60, 1, Some(600)));
        assert_eq!(
            (index.entries(), index.stride()),
            (&[entry(0, 0), entry(40, 400)][..], 4)
        );
    }

    #[test]
    fn keeps_nothing_under_an_index_limit_of_zero() {
        let mut index = PageIndex::new(1, &lowered(LimitKind::IndexEntries, 0));
        for n in 0..5 {
            index.push(&ending(n, 1, Some(n)));
        }
        assert_eq!(index.entries(), []);
    }

    #[test]
    fn seeks_to_the_last_kept_page_at_or_before_a_granule() {
        let mut index = PageIndex::new(1, &Limits::DEFAULT);
        assert_eq!(index.seek(0), None);
        for (offset, granule) in [(0, 100), (40, 200), (80, 300)] {
            index.push(&ending(offset, 1, Some(granule)));
        }
        assert_eq!(index.seek(0), None);
        assert_eq!(index.seek(99), None);
        assert_eq!(index.seek(100), Some(entry(0, 100)));
        assert_eq!(index.seek(199), Some(entry(0, 100)));
        assert_eq!(index.seek(200), Some(entry(40, 200)));
        assert_eq!(index.seek(301), Some(entry(80, 300)));
        assert_eq!(index.seek(u64::MAX), Some(entry(80, 300)));
    }

    proptest! {
        /// Verifies: SEC-MED-006
        #[test]
        fn keeps_the_pages_on_its_stride_within_the_limit(
            count in 0_u64..200,
            max in 1_u64..12,
        ) {
            let mut index = PageIndex::new(1, &lowered(LimitKind::IndexEntries, max));
            for n in 0..count {
                index.push(&ending(n, 1, Some(n)));
            }
            // The model: the smallest power of two that keeps the pages on
            // its stride within the limit, and those pages.
            let mut stride = 1;
            while count.div_ceil(stride) > max {
                stride *= 2;
            }
            let kept: Vec<IndexEntry> = (0..count)
                .filter(|n| n % stride == 0)
                .map(|n| entry(n, n))
                .collect();
            prop_assert_eq!(index.entries(), &kept[..]);
            prop_assert_eq!(index.stride(), stride);
        }
    }
}
