//! The box structure every MP4 file is made of (ISO/IEC 14496-12, section
//! 4.2).
//!
//! A box starts with a 32-bit size and a four-character type. A size of 1
//! means a 64-bit size follows the type, a size of 0 means the box runs to
//! the end of its parent or of the file, and any other size smaller than
//! the header is an error. A `uuid` box has 16 more octets of extended
//! type in its header.
//!
//! [`Children`] reads the boxes inside one parent, one at a time: it charges
//! one step of the budget for each, counts them against the children limit
//! and counts the nesting depth. The probe walks containers from their
//! headers; this module's tests use an explicit-stack walk of the same tree.

use std::ops::Range;

use super::probe::Mp4Error;
#[cfg(test)]
use crate::parse::ParseFault;
use crate::parse::{Budget, Cursor, Depth, LimitKind, Limits};

/// A four-character code naming a box, a brand or a coding.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct FourCc(pub [u8; 4]);

/// The header of one box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BoxHeader {
    /// The box type.
    pub(crate) kind: FourCc,
    /// Where the box starts.
    pub(crate) offset: u64,
    /// The length of the body, or `None` when the box runs to the end of
    /// its parent.
    pub(crate) len: Option<u64>,
}

/// The type of a box with an extended type in its header.
pub(crate) const UUID: FourCc = FourCc(*b"uuid");

/// Reads the header of the box at the cursor and moves past it.
///
/// # Errors
///
/// [`ParseFault::Truncated`] when the header is cut short, and
/// [`Mp4Error::BoxTooSmall`] for a size smaller than the header.
pub(crate) fn read_header(cursor: &mut Cursor<'_>) -> Result<BoxHeader, Mp4Error> {
    let offset = cursor.offset();
    let size = cursor.u32_be()?;
    let kind = FourCc(cursor.array()?);
    let size = match size {
        0 => None,
        1 => Some(cursor.u64_be()?),
        size => Some(u64::from(size)),
    };
    if kind == UUID {
        cursor.skip(16)?;
    }
    // The cursor only moves forward, so this never saturates.
    let header_len = cursor.offset().saturating_sub(offset);
    let len = match size {
        None => None,
        Some(size) => match size.checked_sub(header_len) {
            Some(len) => Some(len),
            None => {
                return Err(Mp4Error::BoxTooSmall {
                    offset,
                    size,
                    header_len,
                });
            }
        },
    };
    Ok(BoxHeader { kind, offset, len })
}

/// The octets a cursor has left, as a range of file offsets.
pub(crate) fn span(cursor: &Cursor<'_>) -> Range<u64> {
    let start = cursor.offset();
    // A cursor's octets lie inside its file, so this never saturates.
    start..start.saturating_add(cursor.remaining())
}

/// A box whose whole body is in memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Mp4Box<'a> {
    /// The box type.
    pub(crate) kind: FourCc,
    /// Where the box starts.
    pub(crate) offset: u64,
    /// How deeply the box is nested; a box at the top of a file is one
    /// level below the root.
    pub(crate) depth: Depth,
    /// The body, after the header.
    pub(crate) body: Cursor<'a>,
}

/// The boxes inside one parent, read one at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Children<'a> {
    /// The octets not read yet. Empty once finished.
    body: Cursor<'a>,
    /// The depth of the parent.
    depth: Depth,
    /// How many children have been read.
    count: u64,
}

impl<'a> Children<'a> {
    /// The boxes in `body`, the body of a parent at `depth`.
    pub(crate) const fn new(body: Cursor<'a>, depth: Depth) -> Self {
        Self {
            body,
            depth,
            count: 0,
        }
    }

    /// The next child, or `None` when the parent has no more.
    ///
    /// # Errors
    ///
    /// The errors of [`Children::required`].
    pub(crate) fn next(
        &mut self,
        limits: &Limits,
        budget: &mut Budget,
    ) -> Result<Option<Mp4Box<'a>>, Mp4Error> {
        if self.body.is_empty() {
            return Ok(None);
        }
        self.required(limits, budget).map(Some)
    }

    /// The next child, which must be there. After an error the parent is
    /// finished, so every later call reads nothing.
    ///
    /// # Errors
    ///
    /// [`ParseFault::BudgetExceeded`] when the step for the child cannot be
    /// paid, [`ParseFault::LimitExceeded`] past the children limit, the
    /// errors of [`read_header`], [`ParseFault::TooDeep`] past the depth
    /// limit, and [`ParseFault::Truncated`] for a child that runs past its
    /// parent, in that order.
    pub(crate) fn required(
        &mut self,
        limits: &Limits,
        budget: &mut Budget,
    ) -> Result<Mp4Box<'a>, Mp4Error> {
        let child = self.read(limits, budget);
        if child.is_err() {
            self.finish();
        }
        child
    }

    /// Reads the next child.
    fn read(&mut self, limits: &Limits, budget: &mut Budget) -> Result<Mp4Box<'a>, Mp4Error> {
        let offset = self.body.offset();
        budget.charge(1, offset)?;
        self.count = self.count.saturating_add(1);
        limits.check(LimitKind::Children, self.count, offset)?;
        let header = read_header(&mut self.body)?;
        let depth = self.depth.descend(limits, offset)?;
        let len = header.len.unwrap_or(self.body.remaining());
        let body = self.body.sub(len)?;
        Ok(Mp4Box {
            kind: header.kind,
            offset,
            depth,
            body,
        })
    }

    /// Skips the rest of the parent.
    pub(crate) fn finish(&mut self) {
        self.body = Cursor::at(&[], self.body.offset());
    }
}

/// What [`walk`] found.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Event<'a> {
    /// A container box, whose children come next.
    Enter(Mp4Box<'a>),
    /// A box that holds no boxes the walk reads.
    Leaf(Mp4Box<'a>),
    /// The end of a container box.
    Leave(Mp4Box<'a>),
    /// A child that could not be read; the rest of its parent is skipped.
    Broken(Mp4Error),
}

/// What the visitor wants [`walk`] to do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Flow {
    /// Go on: enter the box just entered, or read the next sibling.
    Continue,
    /// Skip the box just entered and the rest of its parent.
    SkipRest,
}

/// The visitor [`walk`] calls with the types of the open boxes, from the
/// root down, and what it found inside the last of them.
#[cfg(test)]
pub(crate) type Visitor<'v, 'a> =
    dyn FnMut(&[FourCc], Event<'a>, &mut Budget) -> Result<Flow, Mp4Error> + 'v;

/// The track box.
pub(crate) const TRAK: FourCc = FourCc(*b"trak");
/// The media box.
pub(crate) const MDIA: FourCc = FourCc(*b"mdia");
/// The media information box.
pub(crate) const MINF: FourCc = FourCc(*b"minf");
/// The sample table box.
pub(crate) const STBL: FourCc = FourCc(*b"stbl");
/// The user data box.
pub(crate) const UDTA: FourCc = FourCc(*b"udta");
/// The metadata box.
pub(crate) const META: FourCc = FourCc(*b"meta");
/// The handler reference box.
pub(crate) const HDLR: FourCc = FourCc(*b"hdlr");
/// The item list box, whose every child is an item holding boxes.
pub(crate) const ILST: FourCc = FourCc(*b"ilst");

/// Whether a box of type `kind` is entered when it is found inside
/// `parent`. The containers are the boxes on the way to an audio track's
/// sample table and to the item list, and the items inside an item list.
#[must_use]
pub(crate) fn is_container(parent: FourCc, kind: FourCc) -> bool {
    matches!(kind, META | TRAK | MDIA | MINF | STBL | UDTA | ILST) || parent == ILST
}

/// Where the children of `child`, inside a parent of type `parent`, start,
/// or `None` when the walk reads no boxes inside it.
///
/// A `meta` box is a full box in ISO/IEC 14496-12 and a plain box in
/// `QuickTime`; as `FFmpeg` does, it is read as a plain box when its body
/// starts with the header of an `hdlr` box.
#[cfg(test)]
fn inside(parent: FourCc, child: Mp4Box<'_>) -> Result<Option<Cursor<'_>>, Mp4Error> {
    let mut body = child.body;
    match child.kind {
        META => {
            if body.rest().get(4..8) != Some(HDLR.0.as_slice()) {
                body.skip(4)?;
            }
            Ok(Some(body))
        }
        TRAK | MDIA | MINF | STBL | UDTA | ILST => Ok(Some(body)),
        _ if parent == ILST => Ok(Some(body)),
        _ => Ok(None),
    }
}

/// Walks the boxes inside `root`, depth first, with an explicit stack.
///
/// Every child is read through [`Children`], so the budget, the children
/// limit and the depth limit apply to each. A child that cannot be read is
/// reported as [`Event::Broken`] and ends its parent, which is then left
/// as usual; only a spent budget ends the walk itself. Every box the walk
/// enters is left exactly once, and the root is left last.
///
/// # Errors
///
/// [`ParseFault::BudgetExceeded`] when the budget runs out, and any error
/// the visitor returns.
#[cfg(test)]
pub(crate) fn walk<'a>(
    root: Mp4Box<'a>,
    limits: &Limits,
    budget: &mut Budget,
    visit: &mut Visitor<'_, 'a>,
) -> Result<(), Mp4Error> {
    let mut path = vec![root.kind];
    let mut open = vec![(root, Children::new(root.body, root.depth))];
    while let Some((parent, children)) = open.last_mut() {
        let parent = *parent;
        let (event, enter) = match children.next(limits, budget) {
            Ok(None) => {
                open.pop();
                path.pop();
                visit(&path, Event::Leave(parent), budget)?;
                continue;
            }
            Err(error @ Mp4Error::Fault(ParseFault::BudgetExceeded { .. })) => return Err(error),
            Err(error) => (Event::Broken(error), None),
            Ok(Some(child)) => match inside(parent.kind, child) {
                Ok(None) => (Event::Leaf(child), None),
                Ok(Some(body)) => (Event::Enter(child), Some((child, body))),
                Err(error) => (Event::Broken(error), None),
            },
        };
        let broken = matches!(event, Event::Broken(_));
        let flow = visit(&path, event, budget)?;
        if broken || flow == Flow::SkipRest {
            children.finish();
        } else if let Some((child, body)) = enter {
            open.push((child, Children::new(body, child.depth)));
            path.push(child.kind);
        }
    }
    Ok(())
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::*;
    use gunmetal_testkit::mp4::{large_box, mp4_box, open_box};

    /// The stack size SEC-MED-001 names, in octets.
    const SMALL_STACK: usize = 262_144;

    /// Runs `work` on a 256 KiB stack so a recursive walk fails its test
    /// instead of passing on the runner's larger stack (SEC-MED-001).
    fn on_small_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(SMALL_STACK)
            .spawn(work)
            .expect("the test thread starts")
            .join()
            .expect("the code under test returned instead of panicking")
    }

    fn truncated(offset: u64, needed: u64, available: u64) -> Mp4Error {
        Mp4Error::Fault(ParseFault::Truncated {
            offset,
            needed,
            available,
        })
    }

    /// The header at the start of `bytes`, which start at file offset
    /// `base`, and the offset of the cursor after it.
    fn header_of(bytes: &[u8], base: u64) -> (Result<BoxHeader, Mp4Error>, u64) {
        let mut cursor = Cursor::at(bytes, base);
        let header = read_header(&mut cursor);
        (header, cursor.offset())
    }

    #[test]
    fn reads_a_header_with_a_32_bit_size() {
        let bytes = mp4_box(*b"free", &[1, 2, 3]);
        assert_eq!(
            header_of(&bytes, 100),
            (
                Ok(BoxHeader {
                    kind: FourCc(*b"free"),
                    offset: 100,
                    len: Some(3),
                }),
                108
            )
        );
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn reads_a_size_of_zero_as_running_to_the_end_of_the_parent() {
        let bytes = open_box(*b"mdat", &[1, 2, 3]);
        assert_eq!(
            header_of(&bytes, 7),
            (
                Ok(BoxHeader {
                    kind: FourCc(*b"mdat"),
                    offset: 7,
                    len: None,
                }),
                15
            )
        );
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn reads_a_size_of_one_as_a_64_bit_size_after_the_type() {
        let bytes = large_box(*b"mdat", &[9; 5]);
        assert_eq!(
            header_of(&bytes, 0),
            (
                Ok(BoxHeader {
                    kind: FourCc(*b"mdat"),
                    offset: 0,
                    len: Some(5),
                }),
                16
            )
        );
        // A 64-bit size near u64::MAX is a claim like any other.
        let mut huge = b"\x00\x00\x00\x01mdat".to_vec();
        huge.extend(u64::MAX.to_be_bytes());
        assert_eq!(
            header_of(&huge, 0),
            (
                Ok(BoxHeader {
                    kind: FourCc(*b"mdat"),
                    offset: 0,
                    len: Some(u64::MAX - 16),
                }),
                16
            )
        );
    }

    #[test]
    fn counts_the_extended_type_of_a_uuid_box_in_its_header() {
        let mut bytes = mp4_box(*b"uuid", &[0xEE; 20]);
        assert_eq!(
            header_of(&bytes, 0),
            (
                Ok(BoxHeader {
                    kind: FourCc(*b"uuid"),
                    offset: 0,
                    len: Some(4),
                }),
                24
            )
        );
        // With a 64-bit size the header is 32 octets.
        bytes = large_box(*b"uuid", &[0xEE; 16]);
        assert_eq!(
            header_of(&bytes, 0),
            (
                Ok(BoxHeader {
                    kind: FourCc(*b"uuid"),
                    offset: 0,
                    len: Some(0),
                }),
                32
            )
        );
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn refuses_a_size_smaller_than_the_header() {
        let cases: [(&[u8], u64, u64); 5] = [
            (b"\x00\x00\x00\x07free", 7, 8),
            (b"\x00\x00\x00\x02free", 2, 8),
            (
                b"\x00\x00\x00\x01mdat\x00\x00\x00\x00\x00\x00\x00\x0F",
                15,
                16,
            ),
            (
                b"\x00\x00\x00\x01mdat\x00\x00\x00\x00\x00\x00\x00\x00",
                0,
                16,
            ),
            (b"\x00\x00\x00\x17uuid0123456789abcdef", 23, 24),
        ];
        for (bytes, size, header_len) in cases {
            assert_eq!(
                header_of(bytes, 40).0,
                Err(Mp4Error::BoxTooSmall {
                    offset: 40,
                    size,
                    header_len,
                }),
                "{bytes:02X?}"
            );
        }
        // A size equal to the header is an empty box.
        assert_eq!(
            header_of(b"\x00\x00\x00\x08free", 0),
            (
                Ok(BoxHeader {
                    kind: FourCc(*b"free"),
                    offset: 0,
                    len: Some(0),
                }),
                8
            )
        );
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn reports_each_truncated_header_with_its_exact_offset() {
        let plain = mp4_box(*b"free", &[]);
        for cut in 0..4 {
            assert_eq!(
                header_of(&plain[..cut], 10).0,
                Err(truncated(10, 4, cut as u64)),
                "cut {cut}"
            );
        }
        for cut in 4..8 {
            assert_eq!(
                header_of(&plain[..cut], 10).0,
                Err(truncated(14, 4, cut as u64 - 4)),
                "cut {cut}"
            );
        }
        let large = large_box(*b"mdat", &[]);
        for cut in 8..16 {
            assert_eq!(
                header_of(&large[..cut], 0).0,
                Err(truncated(8, 8, cut as u64 - 8)),
                "cut {cut}"
            );
        }
        let uuid = mp4_box(*b"uuid", &[0; 16]);
        for cut in 8..24 {
            assert_eq!(
                header_of(&uuid[..cut], 0).0,
                Err(truncated(8, 16, cut as u64 - 8)),
                "cut {cut}"
            );
        }
    }

    /// The depth `level` levels below the root of a container.
    fn depth(level: u64) -> Depth {
        (0..level).fold(Depth::CONTAINER_ROOT, |depth, _| {
            depth
                .descend(&Limits::DEFAULT, 0)
                .expect("the test stays within the depth limit")
        })
    }

    /// The box of type `kind` at `offset` in `file`, at `level`, whose body
    /// runs from `body` to `end`.
    fn at(
        file: &[u8],
        kind: [u8; 4],
        offset: u64,
        level: u64,
        body: usize,
        end: usize,
    ) -> Mp4Box<'_> {
        Mp4Box {
            kind: FourCc(kind),
            offset,
            depth: depth(level),
            body: Cursor::at(&file[body..end], body as u64),
        }
    }

    /// `Limits::DEFAULT` with `kind` lowered to `value`.
    fn lowered(kind: LimitKind, value: u64) -> Limits {
        Limits::DEFAULT
            .with_override(kind, value)
            .expect("the test lowers a limit")
    }

    /// Reads every child of `body`, at most `ceiling` times, so a reader
    /// that never finishes fails the test instead of hanging.
    fn collect_children<'a>(
        mut children: Children<'a>,
        limits: &Limits,
        budget: &mut Budget,
        ceiling: usize,
    ) -> Vec<Result<Mp4Box<'a>, Mp4Error>> {
        let mut found = Vec::new();
        while let Some(child) = children.next(limits, budget).transpose() {
            found.push(child);
            assert!(found.len() <= ceiling, "more than {ceiling} children");
        }
        found
    }

    #[test]
    fn reads_the_children_of_a_parent_back_to_back() {
        let file = [
            [0xFF; 50].to_vec(),
            mp4_box(*b"free", &[1, 2]),
            mp4_box(*b"skip", &[]),
            open_box(*b"mdat", &[3, 4]),
        ]
        .concat();
        let children = Children::new(Cursor::at(&file[50..], 50), depth(1));
        let mut budget = Budget::for_input(0, 0, 3);
        assert_eq!(
            collect_children(children, &Limits::DEFAULT, &mut budget, 3),
            vec![
                Ok(at(&file, *b"free", 50, 2, 58, 60)),
                Ok(at(&file, *b"skip", 60, 2, 68, 68)),
                Ok(at(&file, *b"mdat", 68, 2, 76, 78)),
            ]
        );
        // One step a child.
        assert_eq!(budget.remaining(), 0);
        assert_eq!(span(&Cursor::at(&file[58..60], 58)), 58..60);
    }

    #[test]
    fn reads_nothing_from_an_empty_parent() {
        let mut budget = Budget::for_input(0, 0, 0);
        let mut children = Children::new(Cursor::at(&[], 9), depth(1));
        assert_eq!(children.next(&Limits::DEFAULT, &mut budget), Ok(None));
        assert_eq!(
            children.required(&Limits::DEFAULT, &mut Budget::for_input(0, 0, 1)),
            Err(truncated(9, 4, 0))
        );
    }

    /// Verifies: SEC-MED-008, SEC-TM-032
    #[test]
    fn stops_at_a_child_that_runs_past_its_parent() {
        // The second child declares 18 octets, ten of body, and has three.
        let file = [
            mp4_box(*b"free", &[]),
            b"\x00\x00\x00\x12free\x01\x02\x03".to_vec(),
        ]
        .concat();
        let children = Children::new(Cursor::at(&file, 0), depth(0));
        assert_eq!(
            collect_children(
                children,
                &Limits::DEFAULT,
                &mut Budget::for_input(0, 0, 9),
                3
            ),
            vec![
                Ok(at(&file, *b"free", 0, 1, 8, 8)),
                Err(truncated(16, 10, 3))
            ]
        );
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn reads_nothing_more_after_an_error_or_when_finished() {
        let file = [b"\x00\x00\x00\x07free".to_vec(), mp4_box(*b"free", &[])].concat();
        let mut budget = Budget::for_input(0, 0, 9);
        let mut children = Children::new(Cursor::at(&file, 0), depth(0));
        assert_eq!(
            children.next(&Limits::DEFAULT, &mut budget),
            Err(Mp4Error::BoxTooSmall {
                offset: 0,
                size: 7,
                header_len: 8,
            })
        );
        assert_eq!(children.next(&Limits::DEFAULT, &mut budget), Ok(None));
        assert_eq!(budget.remaining(), 8);

        let file = [mp4_box(*b"free", &[]), mp4_box(*b"skip", &[])].concat();
        let mut children = Children::new(Cursor::at(&file, 0), depth(0));
        children.finish();
        assert_eq!(children.next(&Limits::DEFAULT, &mut budget), Ok(None));
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn charges_one_step_a_child_and_stops_when_the_budget_runs_out() {
        let file = [
            mp4_box(*b"free", &[]),
            mp4_box(*b"free", &[]),
            mp4_box(*b"free", &[]),
        ]
        .concat();
        let children = Children::new(Cursor::at(&file, 0), depth(0));
        let mut budget = Budget::for_input(0, 0, 2);
        assert_eq!(
            collect_children(children, &Limits::DEFAULT, &mut budget, 3),
            vec![
                Ok(at(&file, *b"free", 0, 1, 8, 8)),
                Ok(at(&file, *b"free", 8, 1, 16, 16)),
                Err(Mp4Error::Fault(ParseFault::BudgetExceeded { offset: 16 })),
            ]
        );
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn counts_children_against_the_limit() {
        let file = [
            mp4_box(*b"free", &[]),
            mp4_box(*b"free", &[]),
            mp4_box(*b"free", &[]),
        ]
        .concat();
        let limits = lowered(LimitKind::Children, 2);
        let children = Children::new(Cursor::at(&file, 0), depth(0));
        assert_eq!(
            collect_children(children, &limits, &mut Budget::for_input(0, 0, 9), 3),
            vec![
                Ok(at(&file, *b"free", 0, 1, 8, 8)),
                Ok(at(&file, *b"free", 8, 1, 16, 16)),
                Err(Mp4Error::Fault(ParseFault::LimitExceeded {
                    limit: LimitKind::Children,
                    value: 3,
                    max: 2,
                    offset: 16,
                })),
            ]
        );
        // Exactly at the limit is allowed.
        let limits = lowered(LimitKind::Children, 3);
        let children = Children::new(Cursor::at(&file, 0), depth(0));
        assert_eq!(
            collect_children(children, &limits, &mut Budget::for_input(0, 0, 9), 3).len(),
            3
        );
    }

    /// Verifies: SEC-MED-005
    #[test]
    fn counts_the_depth_of_every_child() {
        let file = mp4_box(*b"free", &[]);
        let mut budget = Budget::for_input(0, 0, 9);
        let mut children = Children::new(Cursor::at(&file, 0), depth(31));
        assert_eq!(
            children.next(&Limits::DEFAULT, &mut budget),
            Ok(Some(at(&file, *b"free", 0, 32, 8, 8)))
        );
        let mut children = Children::new(Cursor::at(&file, 4), depth(32));
        assert_eq!(
            children.next(&Limits::DEFAULT, &mut budget),
            Err(Mp4Error::Fault(ParseFault::TooDeep {
                limit: LimitKind::ContainerDepth,
                depth: 33,
                max: 32,
                offset: 4,
            }))
        );
    }

    /// One event as a test records it: the types of the open boxes and
    /// what the walk found.
    type Seen<'a> = (Vec<FourCc>, Event<'a>);

    /// Walks `root`, recording every event, and answers `flow` for the
    /// events `answer` picks.
    fn walk_all<'a>(
        root: Mp4Box<'a>,
        limits: &Limits,
        budget: &mut Budget,
        answer: impl Fn(&[FourCc], &Event<'a>) -> Result<Flow, Mp4Error>,
    ) -> (Result<(), Mp4Error>, Vec<Seen<'a>>) {
        let mut seen = Vec::new();
        let result = walk(root, limits, budget, &mut |path, event, _| {
            seen.push((path.to_vec(), event));
            assert!(seen.len() < 10_000, "the walk does not finish");
            answer(path, &event)
        });
        (result, seen)
    }

    fn path(kinds: &[&[u8; 4]]) -> Vec<FourCc> {
        kinds.iter().map(|&&kind| FourCc(kind)).collect()
    }

    #[expect(
        clippy::unnecessary_wraps,
        reason = "it stands in for a visitor, which returns a Result"
    )]
    fn go_on(_: &[FourCc], _: &Event<'_>) -> Result<Flow, Mp4Error> {
        Ok(Flow::Continue)
    }

    /// A movie with a track, user data with an item list in a full `meta`
    /// box, and a free box, laid out at these offsets: moov 0, trak 8,
    /// mdia 16, mdhd 24, udta 34, meta 42, hdlr 54, ilst 87, an item 95,
    /// its data 103, free 120, and the end at 128.
    fn movie() -> Vec<u8> {
        use gunmetal_testkit::mp4::{data, udta};
        let trak = mp4_box(*b"trak", &mp4_box(*b"mdia", &mp4_box(*b"mdhd", &[1, 2])));
        let item = mp4_box(*b"\xA9nam", &data(1, b"x"));
        let body = [trak, udta(false, &item), mp4_box(*b"free", &[])].concat();
        mp4_box(*b"moov", &body)
    }

    fn root(file: &[u8]) -> Mp4Box<'_> {
        at(file, *b"moov", 0, 1, 8, file.len())
    }

    #[test]
    fn walks_a_tree_of_containers_depth_first() {
        let file = movie();
        let (result, seen) = walk_all(
            root(&file),
            &Limits::DEFAULT,
            &mut Budget::for_input(0, 0, 11),
            go_on,
        );
        let moov = path(&[b"moov"]);
        let ilst = path(&[b"moov", b"udta", b"meta", b"ilst"]);
        assert_eq!(result, Ok(()));
        assert_eq!(
            seen,
            vec![
                (
                    moov.clone(),
                    Event::Enter(at(&file, *b"trak", 8, 2, 16, 34))
                ),
                (
                    path(&[b"moov", b"trak"]),
                    Event::Enter(at(&file, *b"mdia", 16, 3, 24, 34))
                ),
                (
                    path(&[b"moov", b"trak", b"mdia"]),
                    Event::Leaf(at(&file, *b"mdhd", 24, 4, 32, 34))
                ),
                (
                    path(&[b"moov", b"trak"]),
                    Event::Leave(at(&file, *b"mdia", 16, 3, 24, 34))
                ),
                (
                    moov.clone(),
                    Event::Leave(at(&file, *b"trak", 8, 2, 16, 34))
                ),
                (
                    moov.clone(),
                    Event::Enter(at(&file, *b"udta", 34, 2, 42, 120))
                ),
                (
                    path(&[b"moov", b"udta"]),
                    Event::Enter(at(&file, *b"meta", 42, 3, 50, 120))
                ),
                (
                    path(&[b"moov", b"udta", b"meta"]),
                    Event::Leaf(at(&file, *b"hdlr", 54, 4, 62, 87))
                ),
                (
                    path(&[b"moov", b"udta", b"meta"]),
                    Event::Enter(at(&file, *b"ilst", 87, 4, 95, 120))
                ),
                (
                    ilst.clone(),
                    Event::Enter(at(&file, *b"\xA9nam", 95, 5, 103, 120))
                ),
                (
                    path(&[b"moov", b"udta", b"meta", b"ilst", b"\xA9nam"]),
                    Event::Leaf(at(&file, *b"data", 103, 6, 111, 120))
                ),
                (ilst, Event::Leave(at(&file, *b"\xA9nam", 95, 5, 103, 120))),
                (
                    path(&[b"moov", b"udta", b"meta"]),
                    Event::Leave(at(&file, *b"ilst", 87, 4, 95, 120))
                ),
                (
                    path(&[b"moov", b"udta"]),
                    Event::Leave(at(&file, *b"meta", 42, 3, 50, 120))
                ),
                (
                    moov.clone(),
                    Event::Leave(at(&file, *b"udta", 34, 2, 42, 120))
                ),
                (moov, Event::Leaf(at(&file, *b"free", 120, 2, 128, 128))),
                (vec![], Event::Leave(root(&file))),
            ]
        );
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn stops_the_walk_when_the_budget_runs_out() {
        let file = movie();
        let (result, seen) = walk_all(
            root(&file),
            &Limits::DEFAULT,
            &mut Budget::for_input(0, 0, 3),
            go_on,
        );
        // trak, mdia and mdhd cost the three steps; udta has none left.
        assert_eq!(
            result,
            Err(Mp4Error::Fault(ParseFault::BudgetExceeded { offset: 34 }))
        );
        assert_eq!(seen.len(), 5);
    }

    #[test]
    fn hands_on_an_error_from_the_visitor() {
        let file = movie();
        let refusal = Mp4Error::Missing {
            kind: FourCc(*b"mdhd"),
            offset: 16,
        };
        let (result, seen) = walk_all(
            root(&file),
            &Limits::DEFAULT,
            &mut Budget::for_input(0, 0, 99),
            |_, event| {
                if matches!(event, Event::Leaf(_)) {
                    Err(refusal)
                } else {
                    Ok(Flow::Continue)
                }
            },
        );
        assert_eq!(result, Err(refusal));
        assert_eq!(seen.len(), 3);
        let (result, seen) = walk_all(
            root(&file),
            &Limits::DEFAULT,
            &mut Budget::for_input(0, 0, 99),
            |_, event| {
                if matches!(event, Event::Leave(_)) {
                    Err(refusal)
                } else {
                    Ok(Flow::Continue)
                }
            },
        );
        assert_eq!(result, Err(refusal));
        assert!(seen.len() > 3);
    }

    #[test]
    fn reads_a_plain_meta_box_from_its_first_octet() {
        use gunmetal_testkit::mp4::udta;
        // A QuickTime meta box has no version and flags: its first child
        // starts at once. udta 8, meta 16, hdlr 24, ilst 57, end 65.
        let file = mp4_box(*b"moov", &udta(true, &[]));
        let (result, seen) = walk_all(
            root(&file),
            &Limits::DEFAULT,
            &mut Budget::for_input(0, 0, 9),
            go_on,
        );
        assert_eq!(result, Ok(()));
        assert_eq!(
            seen[1..4],
            [
                (
                    path(&[b"moov", b"udta"]),
                    Event::Enter(at(&file, *b"meta", 16, 3, 24, 65))
                ),
                (
                    path(&[b"moov", b"udta", b"meta"]),
                    Event::Leaf(at(&file, *b"hdlr", 24, 4, 32, 57))
                ),
                (
                    path(&[b"moov", b"udta", b"meta"]),
                    Event::Enter(at(&file, *b"ilst", 57, 4, 65, 65))
                ),
            ]
        );
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn skips_the_rest_of_a_parent_after_a_broken_child() {
        // A full meta box too short for its version and flags, then a
        // sibling that is never read; then the walk goes on after udta.
        let udta = mp4_box(
            *b"udta",
            &[mp4_box(*b"meta", &[0, 0]), mp4_box(*b"free", &[])].concat(),
        );
        let file = mp4_box(*b"moov", &[udta, mp4_box(*b"skip", &[])].concat());
        let (result, seen) = walk_all(
            root(&file),
            &Limits::DEFAULT,
            &mut Budget::for_input(0, 0, 9),
            go_on,
        );
        assert_eq!(result, Ok(()));
        assert_eq!(
            seen,
            vec![
                (
                    path(&[b"moov"]),
                    Event::Enter(at(&file, *b"udta", 8, 2, 16, 34))
                ),
                (
                    path(&[b"moov", b"udta"]),
                    Event::Broken(truncated(24, 4, 2))
                ),
                (
                    path(&[b"moov"]),
                    Event::Leave(at(&file, *b"udta", 8, 2, 16, 34))
                ),
                (
                    path(&[b"moov"]),
                    Event::Leaf(at(&file, *b"skip", 34, 2, 42, 42))
                ),
                (vec![], Event::Leave(root(&file))),
            ]
        );
        // A child with a broken header ends its parent the same way.
        let trak = mp4_box(
            *b"trak",
            &[b"\x00\x00\x00\x05free".to_vec(), mp4_box(*b"free", &[])].concat(),
        );
        let file = mp4_box(*b"moov", &trak);
        let (_, seen) = walk_all(
            root(&file),
            &Limits::DEFAULT,
            &mut Budget::for_input(0, 0, 9),
            go_on,
        );
        assert_eq!(
            seen[1..],
            [
                (
                    path(&[b"moov", b"trak"]),
                    Event::Broken(Mp4Error::BoxTooSmall {
                        offset: 16,
                        size: 5,
                        header_len: 8,
                    })
                ),
                (
                    path(&[b"moov"]),
                    Event::Leave(at(&file, *b"trak", 8, 2, 16, 32))
                ),
                (vec![], Event::Leave(root(&file))),
            ]
        );
    }

    #[test]
    fn skips_what_the_visitor_asks_it_to() {
        let file = movie();
        // Skipping at the track's entry leaves the track unread and ends
        // moov; skipping at mdhd ends mdia.
        let (_, seen) = walk_all(
            root(&file),
            &Limits::DEFAULT,
            &mut Budget::for_input(0, 0, 99),
            |_, event| {
                Ok(
                    if matches!(event, Event::Enter(b) if b.kind == FourCc(*b"trak")) {
                        Flow::SkipRest
                    } else {
                        Flow::Continue
                    },
                )
            },
        );
        assert_eq!(
            seen,
            vec![
                (
                    path(&[b"moov"]),
                    Event::Enter(at(&file, *b"trak", 8, 2, 16, 34))
                ),
                (vec![], Event::Leave(root(&file))),
            ]
        );
        let (_, seen) = walk_all(
            root(&file),
            &Limits::DEFAULT,
            &mut Budget::for_input(0, 0, 99),
            |_, event| {
                Ok(if matches!(event, Event::Leaf(_)) {
                    Flow::SkipRest
                } else {
                    Flow::Continue
                })
            },
        );
        assert_eq!(
            seen[2..6],
            [
                (
                    path(&[b"moov", b"trak", b"mdia"]),
                    Event::Leaf(at(&file, *b"mdhd", 24, 4, 32, 34))
                ),
                (
                    path(&[b"moov", b"trak"]),
                    Event::Leave(at(&file, *b"mdia", 16, 3, 24, 34))
                ),
                (
                    path(&[b"moov"]),
                    Event::Leave(at(&file, *b"trak", 8, 2, 16, 34))
                ),
                (
                    path(&[b"moov"]),
                    Event::Enter(at(&file, *b"udta", 34, 2, 42, 120))
                ),
            ]
        );
        // After skipping the rest of udta at its hdlr leaf, the walk goes
        // on to free.
        assert_eq!(
            seen[8..],
            [
                (
                    path(&[b"moov", b"udta"]),
                    Event::Leave(at(&file, *b"meta", 42, 3, 50, 120))
                ),
                (
                    path(&[b"moov"]),
                    Event::Leave(at(&file, *b"udta", 34, 2, 42, 120))
                ),
                (
                    path(&[b"moov"]),
                    Event::Leaf(at(&file, *b"free", 120, 2, 128, 128))
                ),
                (vec![], Event::Leave(root(&file))),
            ]
        );
    }

    /// `levels` `trak` boxes, each inside the one before, with an empty
    /// `free` box in the innermost.
    fn nested(levels: usize) -> Vec<u8> {
        (0..levels).fold(mp4_box(*b"free", &[]), |inner, _| mp4_box(*b"trak", &inner))
    }

    /// Verifies: SEC-MED-005, SEC-TM-032
    #[test]
    fn nests_32_levels_and_refuses_the_33rd() {
        // moov is level 1, so 31 nested tracks reach level 32 and the free
        // box inside them would be level 33.
        let file = mp4_box(*b"moov", &nested(30));
        let (result, seen) = walk_all(
            root(&file),
            &Limits::DEFAULT,
            &mut Budget::for_input(0, 0, 99),
            go_on,
        );
        assert_eq!(result, Ok(()));
        let free_offset: usize = 8 * 31;
        assert_eq!(
            seen[30],
            (
                vec![FourCc(*b"moov")]
                    .into_iter()
                    .chain((0..30).map(|_| FourCc(*b"trak")))
                    .collect(),
                Event::Leaf(at(
                    &file,
                    *b"free",
                    free_offset as u64,
                    32,
                    free_offset + 8,
                    free_offset + 8
                ))
            )
        );
        let file = mp4_box(*b"moov", &nested(31));
        let (result, seen) = walk_all(
            root(&file),
            &Limits::DEFAULT,
            &mut Budget::for_input(0, 0, 99),
            go_on,
        );
        assert_eq!(result, Ok(()));
        let free_offset: usize = 8 * 32;
        assert_eq!(
            seen[31],
            (
                vec![FourCc(*b"moov")]
                    .into_iter()
                    .chain((0..31).map(|_| FourCc(*b"trak")))
                    .collect(),
                Event::Broken(Mp4Error::Fault(ParseFault::TooDeep {
                    limit: LimitKind::ContainerDepth,
                    depth: 33,
                    max: 32,
                    offset: free_offset as u64,
                }))
            )
        );
        // Every open track then ends: 31 Leave events and the root's.
        assert_eq!(seen.len(), 31 + 1 + 31 + 1);
    }

    /// One box of a generated tree: a container with children, or a leaf.
    #[derive(Debug, Clone)]
    enum Tree {
        Container([u8; 4], Vec<Tree>),
        Leaf([u8; 4], Vec<u8>),
    }

    impl Tree {
        fn encode(&self) -> Vec<u8> {
            match self {
                Self::Container(kind, children) => mp4_box(
                    *kind,
                    &children.iter().flat_map(Self::encode).collect::<Vec<_>>(),
                ),
                Self::Leaf(kind, body) => mp4_box(*kind, body),
            }
        }
    }

    /// The events an independent model of the walk expects for `children`,
    /// laid out from `offset`, inside open boxes of types `open`, whose
    /// deepest is at `level`. Returns where the children end. Recursion is
    /// fine here: the trees are at most a few dozen levels deep.
    fn expect<'a>(
        file: &'a [u8],
        children: &[Tree],
        mut offset: usize,
        open: &mut Vec<FourCc>,
        level: u64,
        seen: &mut Vec<Seen<'a>>,
    ) {
        for child in children {
            let len = child.encode().len();
            if level + 1 > 32 {
                seen.push((
                    open.clone(),
                    Event::Broken(Mp4Error::Fault(ParseFault::TooDeep {
                        limit: LimitKind::ContainerDepth,
                        depth: level + 1,
                        max: 32,
                        offset: offset as u64,
                    })),
                ));
                return;
            }
            match child {
                Tree::Leaf(kind, _) => {
                    seen.push((
                        open.clone(),
                        Event::Leaf(at(
                            file,
                            *kind,
                            offset as u64,
                            level + 1,
                            offset + 8,
                            offset + len,
                        )),
                    ));
                }
                Tree::Container(kind, inner) => {
                    let me = at(
                        file,
                        *kind,
                        offset as u64,
                        level + 1,
                        offset + 8,
                        offset + len,
                    );
                    seen.push((open.clone(), Event::Enter(me)));
                    open.push(FourCc(*kind));
                    expect(file, inner, offset + 8, open, level + 1, seen);
                    open.pop();
                    seen.push((open.clone(), Event::Leave(me)));
                }
            }
            offset += len;
        }
    }

    /// Trees of tracks and user data holding free and skip boxes, some
    /// shallow and wide, some as deep as the limit and past it.
    fn tree() -> impl Strategy<Value = Tree> {
        let leaf = (
            prop_oneof![Just(*b"free"), Just(*b"skip")],
            vec(any::<u8>(), 0..4),
        )
            .prop_map(|(kind, body)| Tree::Leaf(kind, body));
        let wide = leaf.clone().prop_recursive(4, 24, 4, |inner| {
            (
                prop_oneof![Just(*b"trak"), Just(*b"udta")],
                vec(inner, 0..4),
            )
                .prop_map(|(kind, children)| Tree::Container(kind, children))
        });
        let deep = (28_usize..36, leaf).prop_map(|(levels, leaf)| {
            (0..levels).fold(leaf, |inner, _| Tree::Container(*b"udta", vec![inner]))
        });
        prop_oneof![wide, deep]
    }

    use proptest::collection::vec;
    use proptest::prelude::*;

    proptest! {
        /// Verifies: SEC-MED-005, SEC-MED-008, SEC-TM-032
        #[test]
        fn walks_every_generated_tree_exactly_as_the_model_does(children in vec(tree(), 0..4)) {
            let body: Vec<u8> = children.iter().flat_map(Tree::encode).collect();
            let file = mp4_box(*b"moov", &body);
            let mut wanted = Vec::new();
            expect(&file, &children, 8, &mut vec![FourCc(*b"moov")], 1, &mut wanted);
            wanted.push((vec![], Event::Leave(root(&file))));
            let run = file.clone();
            let got = on_small_stack(move || {
                let (result, seen) = walk_all(root(&run), &Limits::DEFAULT, &mut Budget::for_input(u64::MAX, 0, u64::MAX), go_on);
                (result, format!("{seen:?}"))
            });
            prop_assert_eq!(got, (Ok(()), format!("{wanted:?}")));
        }

        /// Verifies: SEC-MED-001, SEC-MED-008
        #[test]
        fn returns_for_any_bytes_within_a_bounded_number_of_events(
            body in vec(any::<u8>(), 0..200),
        ) {
            let len = body.len();
            let count = on_small_stack(move || {
                let file = [b"\x00\x00\x00\x00moov".to_vec(), body].concat();
                let (_, seen) = walk_all(root(&file), &Limits::DEFAULT, &mut Budget::for_input(u64::MAX, 0, u64::MAX), go_on);
                seen.len()
            });
            // Every box is at least 8 octets: two events each at most, and
            // at most one broken child for every open box.
            prop_assert!(count <= 3 * (len / 8) + 2, "{} events from {} octets", count, len);
        }
    }
}
