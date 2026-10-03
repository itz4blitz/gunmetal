//! The MP4 audio probe: the sans-I/O entry point of the MP4 parser.
//!
//! [`Probe`] walks the boxes at the top of a file one header at a time,
//! asking its host for each header and for the body of the `ftyp` box and
//! of the small movie boxes it must parse (`mdhd`, `hdlr`, `stsd`, item
//! text and `data` headers), wherever they lie. The media data is never
//! read, and neither is a `covr` payload: a picture is a range taken from
//! the box header (SEC-MED-010, SEC-MED-017). A `moov` after
//! `mdat` costs more header requests, not a second scan of the file.
//! Every box at the top must fit the file, so a file cut short anywhere
//! inside a box is reported as [`ParseFault::Truncated`] at that box.
//!
//! The `moov` box is walked depth first: the first audio track gives the
//! [`AudioTrack`] and the [`SampleTableRanges`], and the item lists in
//! `moov/udta/meta` give the [`IlstItem`]s. A part of the metadata that
//! cannot be read is skipped with a recorded [`Mp4Problem`], and so is an
//! audio track after the first (SEC-MED-017); a malformed movie or audio
//! track fails the file.
//!
//! # Budget
//!
//! The probe charges one step of its [`Budget`] for every box it reads,
//! for every compatible brand of the `ftyp` box and for every descriptor
//! of an `esds` box. A box takes at least 8 octets, a brand 4 and a
//! descriptor 2, and none is charged twice, so a probe of a file of `n`
//! octets takes at most `n / 2` steps. A budget of
//! [`Budget::for_input`]`(n, `[`STEPS_PER_BYTE`]`, `[`FIXED_STEPS`]`)`
//! is never spent by any file (SEC-MED-007).

use super::audio::{AudioTrack, SampleTableRanges, TrackParts};
use super::boxes::{
    Flow, FourCc, HDLR, ILST, MDIA, META, Mp4Box, STBL, TRAK, UDTA, is_container, read_header,
};
use super::ilst::{COVR, DATA, IlstItem, ItemList, MEAN, NAME};
use crate::parse::{
    Budget, Cursor, Depth, LimitKind, Limits, ParseFault, ReadRequest, SansIo, Step, Window,
};
use crate::values::ValueError;

/// Steps of the budget a probe may need for each octet of its file.
pub const STEPS_PER_BYTE: u64 = 1;

/// Steps of the budget a probe may need whatever the size of its file.
pub const FIXED_STEPS: u64 = 8;

/// What the probe found in an MP4 audio file.
#[derive(Debug, Clone, PartialEq)]
pub struct Mp4Audio {
    /// The file type box, when the file has one.
    pub file_type: Option<FileType>,
    /// The first audio track.
    pub track: AudioTrack,
    /// The items of the movie's item lists, in file order.
    pub items: Vec<IlstItem>,
    /// Where the first audio track's sample tables are.
    pub sample_tables: SampleTableRanges,
    /// What was skipped or dropped on the way.
    pub problems: Vec<Mp4Problem>,
}

/// The file type box.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileType {
    /// The major brand, such as `M4A ` or `isom`.
    pub major: FourCc,
    /// The minor version.
    pub minor: u32,
    /// The compatible brands.
    pub compatible: Vec<FourCc>,
}

/// Why an MP4 file could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mp4Error {
    /// A failure every parser shares: truncation, the budget, the depth or
    /// a limit.
    Fault(ParseFault),
    /// A box declared a size smaller than its own header.
    BoxTooSmall {
        /// Where the box starts.
        offset: u64,
        /// The size it declared.
        size: u64,
        /// The length of its header.
        header_len: u64,
    },
    /// A field held a value this parser does not accept.
    Unexpected {
        /// The box the field is in.
        kind: FourCc,
        /// Where the field starts.
        offset: u64,
        /// The value it held.
        found: u64,
    },
    /// A box the audio track needs is missing.
    Missing {
        /// The box that is missing.
        kind: FourCc,
        /// Where the box that should hold it starts.
        offset: u64,
    },
    /// The audio track's media header gives a timescale of zero.
    ZeroTimescale {
        /// Where the timescale field starts.
        offset: u64,
    },
    /// The movie holds no audio track.
    NoAudioTrack {
        /// Where the `moov` box starts.
        offset: u64,
    },
    /// The file holds no `moov` box.
    NoMovie {
        /// The length of the file, where the search ended.
        offset: u64,
    },
}

impl Mp4Error {
    /// Where in the file the error was found.
    #[must_use]
    pub const fn offset(&self) -> u64 {
        match *self {
            Self::Fault(fault) => fault.offset(),
            Self::BoxTooSmall { offset, .. }
            | Self::Unexpected { offset, .. }
            | Self::Missing { offset, .. }
            | Self::ZeroTimescale { offset }
            | Self::NoAudioTrack { offset }
            | Self::NoMovie { offset } => offset,
        }
    }
}

/// Something the probe skipped or dropped, while keeping the rest of the
/// file (SEC-MED-017).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mp4Problem {
    /// An audio track after the first, which is not used.
    ExtraAudioTrack {
        /// Where its `trak` box starts.
        offset: u64,
    },
    /// A value outside its range, dropped (SEC-MED-014).
    Value {
        /// Where the box that held it starts.
        offset: u64,
        /// Why it was dropped.
        error: ValueError,
    },
    /// Part of the metadata that could not be read and was skipped.
    Metadata(Mp4Error),
}

impl From<ParseFault> for Mp4Error {
    fn from(fault: ParseFault) -> Self {
        Self::Fault(fault)
    }
}

/// The file type box.
const FTYP: FourCc = FourCc(*b"ftyp");
/// The movie box.
const MOOV: FourCc = FourCc(*b"moov");

/// The longest box header: a 64-bit size and an extended type.
const LONGEST_HEADER: u64 = 32;

/// A probe of one MP4 audio file, driven through the sans-I/O protocol.
///
/// Resume it with [`Window::start`] first; it then asks for each box
/// header at the top of the file and inside the first `moov`, and for
/// the bodies of the first `ftyp` and of the small boxes the movie
/// needs. Every request is at least one octet, inside the file, and
/// within the read limits, so a host that enforces them never has to
/// refuse one (SEC-MED-010). Once it is done, resuming it again starts
/// a new parse with what is left of its budget.
#[derive(Debug)]
pub struct Probe {
    /// The limits the probe runs under.
    limits: Limits,
    /// The steps it has left.
    budget: Budget,
    /// Where the parse is.
    state: State,
}

/// Where a probe's parse is.
#[derive(Debug, Default)]
enum State {
    /// Before the first window.
    #[default]
    Start,
    /// Waiting for the header of the box at `offset`.
    Header {
        /// Where the box starts.
        offset: u64,
        /// Open movie boxes, outermost first; empty at the top of the file.
        stack: Vec<Open>,
        /// What the parse has found so far.
        found: Found,
    },
    /// Waiting for a small body.
    Body {
        /// What to parse from the window.
        need: Need,
        /// Where the next sibling starts.
        next: u64,
        /// Open movie boxes, outermost first.
        stack: Vec<Open>,
        /// What the parse has found so far.
        found: Found,
    },
}

/// An open container whose children are read one header at a time.
#[derive(Debug, Clone)]
struct Open {
    /// The box type.
    kind: FourCc,
    /// Where the box starts.
    offset: u64,
    /// How deeply the box is nested.
    depth: Depth,
    /// Where the next child starts.
    at: u64,
    /// The end of this box's body.
    end: u64,
    /// Children counted so far.
    children: u64,
}

/// A small body the probe asked for.
#[derive(Debug, Clone, Copy)]
enum Need {
    /// The `ftyp` box.
    FileType,
    /// An `mdhd` or `hdlr` box.
    Media {
        /// The box type.
        kind: FourCc,
        /// Where the box starts.
        offset: u64,
        /// Its depth.
        depth: Depth,
    },
    /// An `stsd` box.
    SampleDescription {
        /// Where the box starts.
        offset: u64,
        /// Its depth.
        depth: Depth,
    },
    /// A `mean`, `name` or non-picture `data` box.
    ItemLeaf {
        /// The box type.
        kind: FourCc,
        /// Where the box starts.
        offset: u64,
        /// Its depth.
        depth: Depth,
    },
    /// The type and locale of a `covr` `data` box.
    PictureHeader {
        /// Where the picture octets start.
        payload_offset: u64,
        /// How many picture octets there are.
        payload_len: u64,
    },
    /// The first eight octets of a `meta` box, to tell a full box from a
    /// plain one.
    MetaPeek {
        /// Where the `meta` box starts.
        offset: u64,
        /// Its depth.
        depth: Depth,
        /// The end of its body.
        end: u64,
    },
}

/// What a parse has found so far.
#[derive(Debug, Default)]
struct Found {
    /// Boxes at the top of the file.
    boxes: u64,
    /// Octets asked for.
    read: u64,
    /// The first file type box.
    file_type: Option<FileType>,
    /// What the first movie holds.
    audio: Option<Mp4Audio>,
    /// The movie being walked.
    movie: Movie,
}

/// Type and locale of a `data` box, before the value.
const DATA_HEADER: u64 = 8;

/// What a probe does after a window.
enum Next {
    /// Asks for a read and waits in a state.
    Read(Box<State>, ReadRequest),
    /// Finishes.
    Done(Box<Mp4Audio>),
}

/// A box whose header has been read and whose body fits its parent.
#[derive(Clone, Copy)]
struct SizedBox {
    /// The box type.
    kind: FourCc,
    /// Where the box starts.
    offset: u64,
    /// How deeply the box is nested.
    depth: Depth,
    /// Where the body starts.
    start: u64,
    /// The length of the body.
    len: u64,
    /// Where the next sibling starts.
    next: u64,
}

impl Probe {
    /// A probe under `limits`, spending `budget`.
    #[must_use]
    pub const fn new(limits: Limits, budget: Budget) -> Self {
        Self {
            limits,
            budget,
            state: State::Start,
        }
    }

    /// Takes in `window` in `state`.
    fn step(&mut self, state: State, window: Window<'_>) -> Result<Next, Mp4Error> {
        match state {
            State::Start => self.next_header(0, Vec::new(), Found::default(), window.file_len),
            State::Header {
                offset,
                stack,
                found,
            } => self.header(window, offset, stack, found),
            State::Body {
                need,
                next,
                stack,
                found,
            } => self.body(window, need, next, stack, found),
        }
    }

    /// Asks for the header of the box at `offset`, leaves a finished
    /// parent, or finishes at the end of the file.
    fn next_header(
        &mut self,
        offset: u64,
        mut stack: Vec<Open>,
        mut found: Found,
        file_len: u64,
    ) -> Result<Next, Mp4Error> {
        let end = stack.last().map_or(file_len, |parent| parent.end);
        if offset >= end {
            let Some(child) = stack.pop() else {
                let Some(mut audio) = found.audio else {
                    return Err(Mp4Error::NoMovie { offset });
                };
                audio.file_type = found.file_type;
                return Ok(Next::Done(Box::new(audio)));
            };
            return self.leave(&child, stack, found, file_len);
        }
        let request = self.request(
            &mut found,
            offset,
            end.saturating_sub(offset).min(LONGEST_HEADER),
        )?;
        Ok(Next::Read(
            Box::new(State::Header {
                offset,
                stack,
                found,
            }),
            request,
        ))
    }

    /// A request for `len` octets at `offset`, within the read limits.
    fn request(&self, found: &mut Found, offset: u64, len: u64) -> Result<ReadRequest, Mp4Error> {
        self.limits.check(LimitKind::ReadBytes, len, offset)?;
        let read = found.read.saturating_add(len);
        self.limits.check(LimitKind::FileBytes, read, offset)?;
        found.read = read;
        Ok(ReadRequest {
            offset,
            // The read limit is at most 16 MiB, so this always fits.
            len: u32::try_from(len).unwrap_or(u32::MAX),
        })
    }

    /// Reads the header of the box at `offset` from `window`, and asks for
    /// a small body, enters a container, or skips to the next sibling.
    fn header(
        &mut self,
        window: Window<'_>,
        offset: u64,
        mut stack: Vec<Open>,
        mut found: Found,
    ) -> Result<Next, Mp4Error> {
        self.budget.charge(1, offset)?;
        let nested = !stack.is_empty();
        if let Err(error) = self.count_child(&mut stack, &mut found, offset) {
            return self.recover(error, nested, stack, found, window.file_len);
        }
        let sized = match self.sized_header(window, offset, &stack) {
            Ok(sized) => sized,
            Err(error) => {
                return self.recover(error, nested, stack, found, window.file_len);
            }
        };
        match stack.last().map(|open| open.kind) {
            Some(parent) => self.movie_box(sized, parent, stack, found, window.file_len),
            None => self.top_box(sized, stack, found, window.file_len),
        }
    }

    /// Counts the box at `offset` against the children limit of its parent,
    /// or of the file when it is at the top.
    fn count_child(
        &self,
        stack: &mut [Open],
        found: &mut Found,
        offset: u64,
    ) -> Result<(), Mp4Error> {
        if let Some(open) = stack.last_mut() {
            open.children = open.children.saturating_add(1);
            self.limits
                .check(LimitKind::Children, open.children, offset)?;
            return Ok(());
        }
        found.boxes = found.boxes.saturating_add(1);
        self.limits
            .check(LimitKind::Children, found.boxes, offset)
            .map_err(Into::into)
    }

    /// The box at `offset` once its header is read and its body is known to
    /// fit its parent.
    fn sized_header(
        &self,
        window: Window<'_>,
        offset: u64,
        stack: &[Open],
    ) -> Result<SizedBox, Mp4Error> {
        let mut cursor = window.cursor();
        let header = read_header(&mut cursor)?;
        let depth = match stack.last() {
            Some(open) => open.depth.descend(&self.limits, offset)?,
            None => Depth::CONTAINER_ROOT.descend(&self.limits, offset)?,
        };
        let start = cursor.offset();
        let available = stack
            .last()
            .map_or(window.file_len, |open| open.end)
            .saturating_sub(start);
        let len = header.len.unwrap_or(available);
        if len > available {
            return Err(ParseFault::Truncated {
                offset: start,
                needed: len,
                available,
            }
            .into());
        }
        Ok(SizedBox {
            kind: header.kind,
            offset,
            depth,
            start,
            len,
            next: start.saturating_add(len),
        })
    }

    /// A nested header that could not be read: inside user data it is a
    /// skipped part, elsewhere it fails the file.
    fn recover(
        &mut self,
        error: Mp4Error,
        nested: bool,
        stack: Vec<Open>,
        mut found: Found,
        file_len: u64,
    ) -> Result<Next, Mp4Error> {
        if nested {
            Self::broken(&stack, &mut found, error)?;
            let offset = Self::parent_end(&stack, file_len);
            self.next_header(offset, stack, found, file_len)
        } else {
            Err(error)
        }
    }

    /// Enters the first `ftyp` or `moov` at the top of the file, or skips
    /// any other top-level box.
    fn top_box(
        &mut self,
        sized: SizedBox,
        stack: Vec<Open>,
        found: Found,
        file_len: u64,
    ) -> Result<Next, Mp4Error> {
        match sized.kind {
            FTYP if found.file_type.is_none() => self.ask(
                Need::FileType,
                sized.start,
                sized.len,
                sized.next,
                stack,
                found,
                file_len,
            ),
            MOOV if found.audio.is_none() => self.enter(
                Open {
                    kind: MOOV,
                    offset: sized.offset,
                    depth: sized.depth,
                    at: sized.start,
                    end: sized.next,
                    children: 0,
                },
                stack,
                found,
                file_len,
            ),
            _ => self.next_header(sized.next, stack, found, file_len),
        }
    }

    /// Enters a movie container, peeks at a `meta` box, or reads a leaf.
    fn movie_box(
        &mut self,
        sized: SizedBox,
        parent: FourCc,
        stack: Vec<Open>,
        mut found: Found,
        file_len: u64,
    ) -> Result<Next, Mp4Error> {
        if sized.kind == META {
            return self.ask(
                Need::MetaPeek {
                    offset: sized.offset,
                    depth: sized.depth,
                    end: sized.next,
                },
                sized.start,
                sized.len.min(DATA_HEADER),
                sized.next,
                stack,
                found,
                file_len,
            );
        }
        if is_container(parent, sized.kind) {
            if parent == ILST {
                let item = Mp4Box {
                    kind: sized.kind,
                    offset: sized.offset,
                    depth: sized.depth,
                    body: Cursor::at(&[], sized.start),
                };
                if found
                    .movie
                    .items
                    .enter(item, &self.limits, &mut found.movie.problems)
                    == Flow::SkipRest
                {
                    return self.next_header(
                        Self::parent_end(&stack, sized.next),
                        stack,
                        found,
                        file_len,
                    );
                }
            }
            if parent == MOOV && sized.kind == TRAK {
                found.movie.track = TrackParts::new(sized.offset);
            }
            return self.enter(
                Open {
                    kind: sized.kind,
                    offset: sized.offset,
                    depth: sized.depth,
                    at: sized.start,
                    end: sized.next,
                    children: 0,
                },
                stack,
                found,
                file_len,
            );
        }
        self.leaf(
            sized.kind,
            sized.offset,
            sized.depth,
            sized.start,
            sized.len,
            sized.next,
            stack,
            found,
            file_len,
        )
    }

    /// Enters `child` and asks for its first child's header.
    fn enter(
        &mut self,
        child: Open,
        mut stack: Vec<Open>,
        found: Found,
        file_len: u64,
    ) -> Result<Next, Mp4Error> {
        let at = child.at;
        stack.push(child);
        self.next_header(at, stack, found, file_len)
    }

    /// Leaves `child` and asks for the next sibling's header.
    fn leave(
        &mut self,
        child: &Open,
        stack: Vec<Open>,
        mut found: Found,
        file_len: u64,
    ) -> Result<Next, Mp4Error> {
        if child.kind == TRAK && stack.last().is_some_and(|open| open.kind == MOOV) {
            found.movie.end_track()?;
        }
        if stack.last().is_some_and(|open| open.kind == ILST) {
            found.movie.items.leave(&mut found.movie.problems);
        }
        // A movie is only entered at the top of the file.
        if child.kind == MOOV {
            found.audio = Some(found.movie.take_audio(child.offset)?);
        }
        self.next_header(child.end, stack, found, file_len)
    }

    /// Records a child that could not be read: inside user data it is a
    /// skipped part, elsewhere it fails the file.
    fn broken(stack: &[Open], found: &mut Found, error: Mp4Error) -> Result<(), Mp4Error> {
        if stack.iter().any(|open| open.kind == UDTA) {
            found.movie.problems.push(Mp4Problem::Metadata(error));
            return Ok(());
        }
        Err(error)
    }

    /// The end of the innermost open box, or `fallback` when none is open.
    fn parent_end(stack: &[Open], fallback: u64) -> u64 {
        stack.last().map_or(fallback, |open| open.end)
    }

    /// Asks for a small body of `len` at `start`, or continues when the
    /// body is empty.
    #[expect(clippy::too_many_arguments, reason = "the walk names each field")]
    fn ask(
        &mut self,
        need: Need,
        start: u64,
        len: u64,
        next: u64,
        stack: Vec<Open>,
        mut found: Found,
        file_len: u64,
    ) -> Result<Next, Mp4Error> {
        if len == 0 {
            let empty = Window {
                offset: start,
                bytes: &[],
                file_len,
            };
            return self.body(empty, need, next, stack, found);
        }
        let request = match self.request(&mut found, start, len) {
            Ok(request) => request,
            Err(error) => match need {
                Need::ItemLeaf { .. } | Need::PictureHeader { .. } | Need::MetaPeek { .. } => {
                    found.movie.problems.push(Mp4Problem::Metadata(error));
                    return self.next_header(next, stack, found, file_len);
                }
                Need::FileType | Need::Media { .. } | Need::SampleDescription { .. } => {
                    return Err(error);
                }
            },
        };
        Ok(Next::Read(
            Box::new(State::Body {
                need,
                next,
                stack,
                found,
            }),
            request,
        ))
    }

    /// Parses a small body and continues at `next`.
    fn body(
        &mut self,
        window: Window<'_>,
        need: Need,
        next: u64,
        stack: Vec<Open>,
        mut found: Found,
    ) -> Result<Next, Mp4Error> {
        match need {
            Need::FileType => {
                found.file_type = Some(file_type(window.cursor(), &mut self.budget)?);
            }
            Need::Media {
                kind,
                offset,
                depth,
            } => {
                found.movie.track.media(Mp4Box {
                    kind,
                    offset,
                    depth,
                    body: window.cursor(),
                })?;
            }
            Need::SampleDescription { offset, depth } => {
                found.movie.track.set_entry(super::audio::sample_entry(
                    Mp4Box {
                        kind: FourCc(*b"stsd"),
                        offset,
                        depth,
                        body: window.cursor(),
                    },
                    &self.limits,
                    &mut self.budget,
                    &mut found.movie.problems,
                ));
            }
            Need::ItemLeaf {
                kind,
                offset,
                depth,
            } => {
                found.movie.items.leaf(
                    Mp4Box {
                        kind,
                        offset,
                        depth,
                        body: window.cursor(),
                    },
                    &self.limits,
                    &mut found.movie.problems,
                );
            }
            Need::PictureHeader {
                payload_offset,
                payload_len,
            } => {
                if self.take_picture(window, payload_offset, payload_len, &mut found)
                    == Flow::SkipRest
                {
                    return self.next_header(
                        Self::parent_end(&stack, next),
                        stack,
                        found,
                        window.file_len,
                    );
                }
            }
            Need::MetaPeek { offset, depth, end } => match meta_children_start(window.cursor()) {
                Ok(at) => {
                    return self.enter(
                        Open {
                            kind: META,
                            offset,
                            depth,
                            at,
                            end,
                            children: 0,
                        },
                        stack,
                        found,
                        window.file_len,
                    );
                }
                Err(error) => {
                    Self::broken(&stack, &mut found, error)?;
                }
            },
        }
        self.next_header(next, stack, found, window.file_len)
    }

    /// Records a picture from the `data` type field, without the payload.
    fn take_picture(
        &self,
        window: Window<'_>,
        payload_offset: u64,
        payload_len: u64,
        found: &mut Found,
    ) -> Flow {
        let mut body = window.cursor();
        match body.u32_be() {
            Ok(type_code) => found.movie.items.picture(
                type_code,
                payload_offset,
                payload_len,
                &self.limits,
                &mut found.movie.problems,
            ),
            Err(fault) => {
                found
                    .movie
                    .problems
                    .push(Mp4Problem::Metadata(fault.into()));
                Flow::Continue
            }
        }
    }

    /// Asks for a small body of a leaf, or records a sample-table range
    /// without reading it.
    #[expect(clippy::too_many_arguments, reason = "the walk names each field")]
    fn leaf(
        &mut self,
        kind: FourCc,
        offset: u64,
        depth: Depth,
        start: u64,
        len: u64,
        next: u64,
        stack: Vec<Open>,
        mut found: Found,
        file_len: u64,
    ) -> Result<Next, Mp4Error> {
        let parent = stack.last().map(|open| open.kind);
        let in_mdia = parent == Some(MDIA);
        let in_stbl = parent == Some(STBL);
        // Children of `ilst` are entered as items, so a leaf under `ilst`
        // is inside one of them.
        let in_item = stack.iter().any(|open| open.kind == ILST);
        if in_mdia && (kind.0 == *b"mdhd" || kind == HDLR) {
            let extra = found.movie.audio.is_some();
            if extra && kind.0 == *b"mdhd" {
                return self.next_header(next, stack, found, file_len);
            }
            return self.ask(
                Need::Media {
                    kind,
                    offset,
                    depth,
                },
                start,
                len,
                next,
                stack,
                found,
                file_len,
            );
        }
        if in_stbl {
            found.movie.track.record_table(kind, start..next);
            let want_stsd =
                kind.0 == *b"stsd" && found.movie.audio.is_none() && found.movie.track.is_audio();
            if want_stsd {
                return self.ask(
                    Need::SampleDescription { offset, depth },
                    start,
                    len,
                    next,
                    stack,
                    found,
                    file_len,
                );
            }
            return self.next_header(next, stack, found, file_len);
        }
        if in_item && (kind == MEAN || kind == NAME || kind == DATA) {
            if kind == DATA && found.movie.items.current_kind() == COVR {
                let payload_len = len.saturating_sub(DATA_HEADER);
                let payload_offset = start.saturating_add(DATA_HEADER.min(len));
                return self.ask(
                    Need::PictureHeader {
                        payload_offset,
                        payload_len,
                    },
                    start,
                    len.min(DATA_HEADER),
                    next,
                    stack,
                    found,
                    file_len,
                );
            }
            return self.ask(
                Need::ItemLeaf {
                    kind,
                    offset,
                    depth,
                },
                start,
                len,
                next,
                stack,
                found,
                file_len,
            );
        }
        self.next_header(next, stack, found, file_len)
    }
}

impl SansIo for Probe {
    type Output = Result<Mp4Audio, Mp4Error>;

    fn resume(&mut self, window: Window<'_>) -> Step<Self::Output> {
        let state = std::mem::take(&mut self.state);
        match self.step(state, window) {
            Ok(Next::Read(state, request)) => {
                self.state = *state;
                Step::Need(request)
            }
            Ok(Next::Done(audio)) => Step::Done(Ok(*audio)),
            Err(error) => Step::Done(Err(error)),
        }
    }
}

/// Reads the body of a file type box, charging a step for each
/// compatible brand. Octets after the last whole brand are ignored.
fn file_type(mut body: Cursor<'_>, budget: &mut Budget) -> Result<FileType, Mp4Error> {
    let major = FourCc(body.array()?);
    let minor = body.u32_be()?;
    let mut compatible = Vec::new();
    loop {
        let at = body.offset();
        let Ok(brand) = body.array() else {
            break;
        };
        budget.charge(1, at)?;
        compatible.push(FourCc(brand));
    }
    Ok(FileType {
        major,
        minor,
        compatible,
    })
}

/// What a walk of a movie has found.
#[derive(Debug, Default)]
struct Movie {
    /// The parts of the track being walked.
    track: TrackParts,
    /// The first audio track and its sample tables.
    audio: Option<(AudioTrack, SampleTableRanges)>,
    /// The items of the item lists.
    items: ItemList,
    /// What was skipped or dropped.
    problems: Vec<Mp4Problem>,
}

impl Movie {
    /// Reads the track just left if it is the first audio track, and
    /// records it if it is a later one.
    fn end_track(&mut self) -> Result<(), Mp4Error> {
        let track = std::mem::take(&mut self.track);
        if !track.is_audio() {
            return Ok(());
        }
        if self.audio.is_some() {
            self.problems.push(Mp4Problem::ExtraAudioTrack {
                offset: track.offset,
            });
            return Ok(());
        }
        self.audio = Some(track.audio()?);
        Ok(())
    }

    /// The movie as the probe reports it, or [`Mp4Error::NoAudioTrack`].
    fn take_audio(&mut self, offset: u64) -> Result<Mp4Audio, Mp4Error> {
        let Some((track, sample_tables)) = self.audio.take() else {
            return Err(Mp4Error::NoAudioTrack { offset });
        };
        Ok(Mp4Audio {
            file_type: None,
            track,
            items: std::mem::take(&mut self.items).into_items(),
            sample_tables,
            problems: std::mem::take(&mut self.problems),
        })
    }
}

/// Where the children of a `meta` box start: after the version and flags
/// of a full box, or at the first octet when the body starts with `hdlr`.
fn meta_children_start(mut body: Cursor<'_>) -> Result<u64, Mp4Error> {
    let start = body.offset();
    if body.rest().get(4..8) == Some(HDLR.0.as_slice()) {
        return Ok(start);
    }
    body.skip(4)?;
    Ok(body.offset())
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use std::num::NonZeroU32;

    use super::*;
    use crate::formats::mp4::audio::{
        AudioEntry, AudioSpecificConfig, ChunkOffsets, CodecConfig, Esds, SampleSizes, Specific,
    };
    use crate::formats::mp4::ilst::{ItemKey, ItemValue, PictureRef};
    use crate::parse::{DriveError, LimitKind, ReadRequest, drive};
    use crate::text::Text;
    use crate::values::{BitDepth, Channels, SampleRate};
    use gunmetal_testkit::mp4::{
        self as kit, SampleEntry, data, freeform, ftyp, full_box, hdlr, large_box, mp4_box,
        open_box, stsd, trak, udta,
    };
    use proptest::collection::vec;
    use proptest::prelude::*;

    /// The stack size SEC-MED-001 names, in octets.
    const SMALL_STACK: usize = 262_144;

    /// Runs `work` on a 256 KiB stack so a recursive probe fails its test
    /// instead of passing on the runner's larger stack (SEC-MED-001).
    fn on_small_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(SMALL_STACK)
            .spawn(work)
            .expect("the test thread starts")
            .join()
            .expect("the code under test returned instead of panicking")
    }

    /// Probes `file` under `limits` with `budget`, serving its requests
    /// from memory.
    fn run(
        file: &[u8],
        limits: &Limits,
        budget: Budget,
    ) -> Result<Result<Mp4Audio, Mp4Error>, DriveError> {
        drive(Probe::new(*limits, budget), file, limits)
    }

    /// The budget the module documentation promises is enough for `file`.
    fn enough(file: &[u8]) -> Budget {
        Budget::for_input(file.len() as u64, STEPS_PER_BYTE, FIXED_STEPS)
    }

    /// Probes `file` under the default limits with the promised budget.
    fn probe(file: &[u8]) -> Result<Mp4Audio, Mp4Error> {
        run(file, &Limits::DEFAULT, enough(file))
            .expect("the probe asks only for what the host allows")
    }

    /// `Limits::DEFAULT` with `kind` lowered to `value`.
    fn lowered(kind: LimitKind, value: u64) -> Limits {
        Limits::DEFAULT
            .with_override(kind, value)
            .expect("the test lowers a limit")
    }

    fn fault(fault: ParseFault) -> Mp4Error {
        Mp4Error::Fault(fault)
    }

    fn truncated(offset: u64, needed: u64, available: u64) -> Mp4Error {
        fault(ParseFault::Truncated {
            offset,
            needed,
            available,
        })
    }

    fn text(value: &str) -> Text {
        Text {
            value: value.to_owned(),
            truncated: false,
            replaced: false,
        }
    }

    /// The file type box of the standard file: 28 octets.
    fn brand() -> Vec<u8> {
        ftyp(*b"M4A ", 0x200, &[*b"M4A ", *b"mp42", *b"isom"])
    }

    fn brand_read() -> FileType {
        FileType {
            major: FourCc(*b"M4A "),
            minor: 0x200,
            compatible: vec![FourCc(*b"M4A "), FourCc(*b"mp42"), FourCc(*b"isom")],
        }
    }

    /// Media data of 24 octets.
    fn mdat() -> Vec<u8> {
        mp4_box(*b"mdat", &[0xAA; 16])
    }

    /// The sample description and tables of the standard track: AAC LC at
    /// 44.1 kHz in stereo, ten samples of 16 octets in one chunk.
    fn tables() -> Vec<u8> {
        let esds = kit::esds(&kit::Esds {
            object_type: 0x40,
            max_bitrate: 128_000,
            avg_bitrate: 96_000,
            specific: Some(&kit::audio_specific_config(2, 4, 2)),
            width: 4,
        });
        let entry = kit::sample_entry(&SampleEntry {
            format: *b"mp4a",
            version: 0,
            channels: 2,
            bits: 16,
            rate: 44_100,
            children: &esds,
        });
        [
            stsd(&[&entry]),
            full_box(*b"stts", 0, 0, &[0, 0, 0, 1, 0, 0, 0, 10, 0, 0, 4, 0]),
            full_box(
                *b"stsc",
                0,
                0,
                &[0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 10, 0, 0, 0, 1],
            ),
            full_box(*b"stsz", 0, 0, &[0, 0, 0, 16, 0, 0, 0, 10]),
            full_box(*b"stco", 0, 0, &[0, 0, 0, 1, 0, 0, 0, 0]),
        ]
        .concat()
    }

    /// A track of 292 octets with `handler` and the standard tables.
    fn track(handler: [u8; 4], timescale: u32) -> Vec<u8> {
        trak(handler, &kit::mdhd(timescale, 441_000), &tables())
    }

    /// The `mdia` box of the standard audio track.
    fn audio_mdia() -> Vec<u8> {
        mp4_box(
            *b"mdia",
            &[
                kit::mdhd(44_100, 441_000),
                hdlr(*b"soun"),
                mp4_box(*b"minf", &mp4_box(*b"stbl", &tables())),
            ]
            .concat(),
        )
    }

    /// The standard item list: a title, a freeform item and artwork.
    fn items() -> Vec<u8> {
        [
            mp4_box(*b"\xA9nam", &data(1, b"Song")),
            freeform(Some("com.apple.iTunes"), Some("iTunSMPB"), &[data(1, b"x")]),
            mp4_box(*b"covr", &data(13, &[0xFF, 0xD8])),
        ]
        .concat()
    }

    /// The standard movie, 488 octets: an audio track, then user data with
    /// the standard item list in a full `meta` box.
    fn movie() -> Vec<u8> {
        mp4_box(
            *b"moov",
            &[track(*b"soun", 44_100), udta(false, &items())].concat(),
        )
    }

    /// What the probe should find in the standard movie when its body
    /// starts at `body` - 8, as it does for a movie with an 8-octet header
    /// at `body` - 8.
    fn expected(at: u64, file_type: Option<FileType>) -> Mp4Audio {
        // The track starts at `t`: its sample description 97 octets in,
        // with the specific info 95 octets into that, and its tables after
        // it. The user data starts 292 octets after the track, and the
        // artwork's octets 186 octets into that.
        let t = at + 8;
        let u = t + 292;
        Mp4Audio {
            file_type,
            track: AudioTrack {
                offset: t,
                timescale: NonZeroU32::new(44_100).expect("non-zero"),
                duration: Some(441_000),
                entry: AudioEntry {
                    format: FourCc(*b"mp4a"),
                    channels: Channels::new(2).ok(),
                    bits: BitDepth::new(16).ok(),
                    sample_rate: SampleRate::new(44_100).ok(),
                    config: CodecConfig::Esds(Esds {
                        object_type: 0x40,
                        max_bitrate: 128_000,
                        avg_bitrate: 96_000,
                        specific: Some(Specific {
                            range: t + 97 + 95..t + 97 + 97,
                            audio: Some(AudioSpecificConfig {
                                object_type: 2,
                                sample_rate: SampleRate::new(44_100).ok(),
                                channel_config: 2,
                                extension: None,
                            }),
                        }),
                    }),
                },
            },
            items: vec![
                IlstItem {
                    key: ItemKey::Atom(FourCc(*b"\xA9nam")),
                    values: vec![ItemValue::Text(text("Song"))],
                },
                IlstItem {
                    key: ItemKey::Freeform {
                        mean: Some(text("com.apple.iTunes")),
                        name: text("iTunSMPB"),
                    },
                    values: vec![ItemValue::Text(text("x"))],
                },
                IlstItem {
                    key: ItemKey::Atom(FourCc(*b"covr")),
                    values: vec![ItemValue::Picture(PictureRef {
                        type_code: 13,
                        offset: u + 186,
                        len: 2,
                    })],
                },
            ],
            sample_tables: SampleTableRanges {
                stts: Some(t + 208..t + 224),
                stsc: Some(t + 232..t + 252),
                sizes: Some(SampleSizes::Stsz(t + 260..t + 272)),
                offsets: Some(ChunkOffsets::Stco(t + 280..t + 292)),
            },
            problems: vec![],
        }
    }

    /// Verifies: SEC-MED-001
    #[test]
    fn reads_a_file_with_the_movie_before_the_media_data() {
        let file = [brand(), movie(), mdat()].concat();
        assert_eq!(movie().len(), 488);
        assert_eq!(probe(&file), Ok(expected(28, Some(brand_read()))));
    }

    #[test]
    fn reads_a_file_with_the_movie_after_the_media_data() {
        let file = [brand(), mdat(), movie()].concat();
        assert_eq!(probe(&file), Ok(expected(52, Some(brand_read()))));
    }

    /// Verifies: SEC-MED-010
    #[test]
    fn asks_only_for_box_headers_and_the_bodies_it_reads() {
        // The media data comes first and is never read.
        let file = [brand(), mdat(), movie()].concat();
        let mut probe = Probe::new(Limits::DEFAULT, enough(&file));
        let mut window = Window::start(file.len() as u64);
        let mut asked = Vec::new();
        let result = loop {
            match probe.resume(window) {
                Step::Done(result) => break result,
                Step::Need(request) => {
                    asked.push(request);
                    let start = usize::try_from(request.offset).expect("the offset fits");
                    window = Window {
                        offset: request.offset,
                        bytes: &file[start..start + request.len as usize],
                        file_len: file.len() as u64,
                    };
                }
            }
        };
        assert_eq!(result, Ok(expected(52, Some(brand_read()))));
        assert_eq!(asked[0], ReadRequest { offset: 0, len: 32 });
        assert_eq!(asked[1], ReadRequest { offset: 8, len: 20 });
        assert_eq!(
            asked[2],
            ReadRequest {
                offset: 28,
                len: 32
            }
        );
        assert_eq!(
            asked[3],
            ReadRequest {
                offset: 52,
                len: 32
            }
        );
        // The movie is walked by headers and small bodies, never as one
        // blob, and the media data after its header is never read.
        assert!(asked.iter().all(|request| request.len <= 95), "{asked:?}");
        assert!(
            asked.iter().all(|request| {
                request.offset == 28
                    || request.offset + u64::from(request.len) <= 36
                    || request.offset >= 52
            }),
            "{asked:?}"
        );
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn reads_boxes_of_size_zero_at_the_end_of_the_file() {
        let open_mdat = open_box(*b"mdat", &[0xAA; 16]);
        let file = [brand(), movie(), open_mdat].concat();
        assert_eq!(probe(&file), Ok(expected(28, Some(brand_read()))));
        let open_movie = open_box(*b"moov", &movie()[8..]);
        let file = [brand(), mdat(), open_movie].concat();
        assert_eq!(probe(&file), Ok(expected(52, Some(brand_read()))));
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn reads_boxes_with_64_bit_sizes() {
        let file = [brand(), large_box(*b"mdat", &[0xAA; 16]), movie()].concat();
        assert_eq!(probe(&file), Ok(expected(60, Some(brand_read()))));
        // A movie with a 16-octet header: its body starts 8 octets later.
        let file = [brand(), large_box(*b"moov", &movie()[8..])].concat();
        assert_eq!(probe(&file), Ok(expected(36, Some(brand_read()))));
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn refuses_a_box_of_size_7() {
        let file = [brand(), b"\x00\x00\x00\x07free".to_vec(), movie()].concat();
        assert_eq!(
            probe(&file),
            Err(Mp4Error::BoxTooSmall {
                offset: 28,
                size: 7,
                header_len: 8,
            })
        );
    }

    #[test]
    fn reads_a_file_without_a_file_type_box() {
        let file = [movie(), mdat()].concat();
        assert_eq!(probe(&file), Ok(expected(0, None)));
    }

    #[test]
    fn keeps_the_first_file_type_box_and_the_first_movie() {
        let other_brand = ftyp(*b"isom", 1, &[]);
        let other_movie = mp4_box(*b"moov", &track(*b"soun", 8_000));
        let file = [brand(), other_brand, movie(), other_movie].concat();
        assert_eq!(probe(&file), Ok(expected(44, Some(brand_read()))));
    }

    #[test]
    fn reads_a_file_type_box_with_no_brands_or_a_partial_one() {
        let file = [ftyp(*b"isom", 7, &[]), movie()].concat();
        let read = probe(&file).map(|audio| audio.file_type);
        assert_eq!(
            read,
            Ok(Some(FileType {
                major: FourCc(*b"isom"),
                minor: 7,
                compatible: vec![],
            }))
        );
        // Two octets after the last brand are not one.
        let file = [mp4_box(*b"ftyp", b"M4A \x00\x00\x00\x00mp42is"), movie()].concat();
        let read = probe(&file).map(|audio| audio.file_type);
        assert_eq!(
            read,
            Ok(Some(FileType {
                major: FourCc(*b"M4A "),
                minor: 0,
                compatible: vec![FourCc(*b"mp42")],
            }))
        );
    }

    /// Verifies: SEC-MED-001, SEC-MED-008
    #[test]
    fn reads_boxes_with_empty_bodies_without_asking_for_nothing() {
        // An empty ftyp lacks its major brand; an empty moov has no track.
        let file = [mp4_box(*b"ftyp", &[]), movie()].concat();
        assert_eq!(probe(&file), Err(truncated(8, 4, 0)));
        let file = [brand(), mp4_box(*b"moov", &[])].concat();
        assert_eq!(probe(&file), Err(Mp4Error::NoAudioTrack { offset: 28 }));
    }

    #[test]
    fn reads_a_plain_meta_box() {
        let file = [
            brand(),
            mp4_box(
                *b"moov",
                &[track(*b"soun", 44_100), udta(true, &items())].concat(),
            ),
        ]
        .concat();
        let mut wanted = expected(28, Some(brand_read()));
        // Without its version and flags the artwork is 4 octets earlier.
        wanted.items[2].values[0] = ItemValue::Picture(PictureRef {
            type_code: 13,
            offset: 28 + 8 + 292 + 182,
            len: 2,
        });
        assert_eq!(probe(&file), Ok(wanted));
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn refuses_an_audio_track_with_a_timescale_of_zero() {
        let file = [brand(), mp4_box(*b"moov", &track(*b"soun", 0))].concat();
        // moov 28, trak 36, mdia 44, mdhd 52 with its timescale 20 in.
        assert_eq!(probe(&file), Err(Mp4Error::ZeroTimescale { offset: 72 }));
    }

    #[test]
    fn uses_the_first_audio_track_and_records_the_second() {
        let file = [
            brand(),
            mp4_box(
                *b"moov",
                &[
                    track(*b"soun", 44_100),
                    track(*b"soun", 8_000),
                    udta(false, &items()),
                ]
                .concat(),
            ),
        ]
        .concat();
        let mut wanted = expected(28, Some(brand_read()));
        // The second track moves the artwork 292 octets on.
        wanted.items[2].values[0] = ItemValue::Picture(PictureRef {
            type_code: 13,
            offset: 28 + 8 + 292 + 292 + 186,
            len: 2,
        });
        wanted.problems = vec![Mp4Problem::ExtraAudioTrack { offset: 36 + 292 }];
        assert_eq!(probe(&file), Ok(wanted));
    }

    #[test]
    fn passes_over_tracks_that_are_not_audio() {
        let file = [
            brand(),
            mp4_box(
                *b"moov",
                &[
                    track(*b"vide", 0),
                    mp4_box(*b"trak", &[]),
                    track(*b"soun", 44_100),
                    udta(false, &items()),
                ]
                .concat(),
            ),
        ]
        .concat();
        // The video track and an empty track come first: 300 octets.
        assert_eq!(probe(&file), Ok(expected(28 + 300, Some(brand_read()))));
    }

    #[test]
    fn refuses_a_file_without_a_movie_or_an_audio_track() {
        assert_eq!(probe(&[]), Err(Mp4Error::NoMovie { offset: 0 }));
        assert_eq!(
            probe(&[brand(), mdat()].concat()),
            Err(Mp4Error::NoMovie { offset: 52 })
        );
        let file = [
            brand(),
            mp4_box(
                *b"moov",
                &[track(*b"vide", 1), udta(false, &items())].concat(),
            ),
        ]
        .concat();
        assert_eq!(probe(&file), Err(Mp4Error::NoAudioTrack { offset: 28 }));
    }

    /// `levels` boxes of type `kind`, each inside the one before, with an
    /// empty `free` box in the innermost.
    fn nested(kind: [u8; 4], levels: usize) -> Vec<u8> {
        (0..levels).fold(mp4_box(*b"free", &[]), |inner, _| mp4_box(kind, &inner))
    }

    /// Verifies: SEC-MED-005, SEC-TM-032
    #[test]
    fn refuses_tracks_nested_33_levels_deep() {
        // moov is level 1; 30 nested tracks reach level 31 and their free
        // box level 32, which is allowed.
        let file = [
            brand(),
            mp4_box(
                *b"moov",
                &[track(*b"soun", 44_100), nested(*b"trak", 30)].concat(),
            ),
        ]
        .concat();
        assert!(probe(&file).is_ok());
        // With 31 the free box, at 28 + 300 + 8 * 31, is level 33.
        let file = [
            brand(),
            mp4_box(
                *b"moov",
                &[track(*b"soun", 44_100), nested(*b"trak", 31)].concat(),
            ),
        ]
        .concat();
        assert_eq!(
            probe(&file),
            Err(fault(ParseFault::TooDeep {
                limit: LimitKind::ContainerDepth,
                depth: 33,
                max: 32,
                offset: 28 + 300 + 8 * 31,
            }))
        );
    }

    /// Verifies: SEC-MED-005
    #[test]
    fn skips_user_data_nested_33_levels_deep_and_keeps_the_rest() {
        let file = [
            brand(),
            mp4_box(
                *b"moov",
                &[track(*b"soun", 44_100), nested(*b"udta", 31)].concat(),
            ),
        ]
        .concat();
        let mut wanted = expected(28, Some(brand_read()));
        wanted.items = vec![];
        wanted.problems = vec![Mp4Problem::Metadata(fault(ParseFault::TooDeep {
            limit: LimitKind::ContainerDepth,
            depth: 33,
            max: 32,
            offset: 28 + 300 + 8 * 31,
        }))];
        assert_eq!(probe(&file), Ok(wanted));
    }

    #[test]
    fn skips_a_broken_part_of_the_metadata_and_keeps_the_rest() {
        // An item of size 7 after the standard items, at 28 + 300 + 188.
        let broken = [items(), b"\x00\x00\x00\x07\xA9day".to_vec()].concat();
        let file = [
            brand(),
            mp4_box(
                *b"moov",
                &[track(*b"soun", 44_100), udta(false, &broken)].concat(),
            ),
        ]
        .concat();
        let mut wanted = expected(28, Some(brand_read()));
        wanted.problems = vec![Mp4Problem::Metadata(Mp4Error::BoxTooSmall {
            offset: 28 + 300 + 188,
            size: 7,
            header_len: 8,
        })];
        assert_eq!(probe(&file), Ok(wanted));
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn refuses_a_short_file_type_box_or_track_handler() {
        // An ftyp with its major brand and no minor version, at 12.
        let file = [mp4_box(*b"ftyp", b"M4A "), movie()].concat();
        assert_eq!(probe(&file), Err(truncated(12, 4, 0)));
        // moov 28, trak 36, mdia 44, and an hdlr at 52 whose handler type,
        // at 68, has one octet.
        let handler = full_box(*b"hdlr", 0, 0, &[0, 0, 0, 0, b's']);
        let file = [
            brand(),
            mp4_box(*b"moov", &mp4_box(*b"trak", &mp4_box(*b"mdia", &handler))),
        ]
        .concat();
        assert_eq!(probe(&file), Err(truncated(68, 4, 1)));
    }

    #[test]
    fn refuses_a_broken_box_elsewhere_in_the_movie() {
        let file = [
            brand(),
            mp4_box(
                *b"moov",
                &[track(*b"soun", 44_100), b"\x00\x00\x00\x07free".to_vec()].concat(),
            ),
        ]
        .concat();
        assert_eq!(
            probe(&file),
            Err(Mp4Error::BoxTooSmall {
                offset: 28 + 300,
                size: 7,
                header_len: 8,
            })
        );
    }

    /// The top-level boxes of a file, as (start, length), and which one is
    /// the movie.
    struct Layout {
        file: Vec<u8>,
        boxes: Vec<(u64, u64)>,
        movie: usize,
        wanted: Mp4Audio,
    }

    fn layout(pieces: &[Vec<u8>], movie: usize, wanted: Mp4Audio) -> Layout {
        let mut boxes = Vec::new();
        let mut start = 0;
        for piece in pieces {
            boxes.push((start, piece.len() as u64));
            start += piece.len() as u64;
        }
        Layout {
            file: pieces.concat(),
            boxes,
            movie,
            wanted,
        }
    }

    /// What an independent model of the probe expects when `layout`'s file
    /// is cut after `cut` octets.
    fn cut_outcome(layout: &Layout, cut: u64) -> Result<Mp4Audio, Mp4Error> {
        let index = layout
            .boxes
            .iter()
            .position(|&(start, len)| cut < start + len)
            .expect("the cut is inside the file");
        let (start, len) = layout.boxes[index];
        let into = cut - start;
        match into {
            // A cut between boxes leaves a shorter, well-formed file.
            0 if index > layout.movie => Ok(layout.wanted.clone()),
            0 => Err(Mp4Error::NoMovie { offset: cut }),
            1..4 => Err(truncated(start, 4, into)),
            4..8 => Err(truncated(start + 4, 4, into - 4)),
            _ => Err(truncated(start + 8, len - 8, into - 8)),
        }
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn reports_every_cut_of_a_valid_file_exactly() {
        for layout in [
            layout(
                &[brand(), movie(), mdat()],
                1,
                expected(28, Some(brand_read())),
            ),
            layout(
                &[brand(), mdat(), movie()],
                2,
                expected(52, Some(brand_read())),
            ),
        ] {
            for cut in 0..layout.file.len() as u64 {
                assert_eq!(
                    probe(&layout.file[..usize::try_from(cut).expect("the cut fits")]),
                    cut_outcome(&layout, cut),
                    "cut at {cut}"
                );
            }
        }
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn runs_out_of_budget_at_exactly_the_step_it_should() {
        // Three top-level boxes, three brands, 23 boxes in the movie, the
        // sample entry, the esds box and its three descriptors: 34 steps,
        // the last for the mdat header at 516.
        let file = [brand(), movie(), mdat()].concat();
        assert_eq!(
            run(&file, &Limits::DEFAULT, Budget::for_input(0, 0, 34)),
            Ok(Ok(expected(28, Some(brand_read()))))
        );
        assert_eq!(
            run(&file, &Limits::DEFAULT, Budget::for_input(0, 0, 33)),
            Ok(Err(fault(ParseFault::BudgetExceeded { offset: 516 })))
        );
        // The brands are charged one by one: the third is at 24.
        assert_eq!(
            run(&file, &Limits::DEFAULT, Budget::for_input(0, 0, 3)),
            Ok(Err(fault(ParseFault::BudgetExceeded { offset: 24 })))
        );
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn counts_the_boxes_at_the_top_against_the_children_limit() {
        let free = mp4_box(*b"free", &[]);
        let file = [brand(), movie(), mdat(), free.clone(), free.clone(), free].concat();
        // The sample table's five children are the most any box has.
        assert!(
            run(&file, &lowered(LimitKind::Children, 6), enough(&file))
                .is_ok_and(|probe| probe.is_ok())
        );
        assert_eq!(
            run(&file, &lowered(LimitKind::Children, 5), enough(&file)),
            Ok(Err(fault(ParseFault::LimitExceeded {
                limit: LimitKind::Children,
                value: 6,
                max: 5,
                offset: 556,
            })))
        );
        // The sample table's fifth child is refused when the limit is 4.
        assert_eq!(
            run(&file, &lowered(LimitKind::Children, 4), enough(&file)),
            Ok(Err(fault(ParseFault::LimitExceeded {
                limit: LimitKind::Children,
                value: 5,
                max: 4,
                offset: 308,
            })))
        );
    }

    /// Verifies: SEC-MED-005
    #[test]
    fn counts_the_boxes_at_the_top_as_the_first_level() {
        let file = [brand(), movie()].concat();
        assert_eq!(
            run(&file, &lowered(LimitKind::ContainerDepth, 0), enough(&file)),
            Ok(Err(fault(ParseFault::TooDeep {
                limit: LimitKind::ContainerDepth,
                depth: 1,
                max: 0,
                offset: 0,
            })))
        );
    }

    /// Verifies: SEC-MED-017, SEC-MED-010
    #[test]
    fn keeps_the_track_when_artwork_makes_the_movie_larger_than_one_read() {
        // A 512-octet JPEG cover makes the movie larger than a 256-octet
        // read (the sample description is 95 octets). Asking for the whole
        // `moov` would fail the file; walking headers leaves the track
        // and a range for the picture.
        let cover = [0xFF; 512];
        let items = [
            mp4_box(*b"\xA9nam", &data(1, b"Song")),
            mp4_box(*b"covr", &data(13, &cover)),
        ]
        .concat();
        let file = [
            brand(),
            mp4_box(
                *b"moov",
                &[track(*b"soun", 44_100), udta(false, &items)].concat(),
            ),
        ]
        .concat();
        let limits = lowered(LimitKind::ReadBytes, 256);
        let audio = run(&file, &limits, enough(&file))
            .expect("the probe asks only for what the host allows")
            .expect("the track is kept");
        assert_eq!(audio.track.timescale.get(), 44_100);
        assert_eq!(audio.track.entry.format, FourCc(*b"mp4a"));
        assert_eq!(
            audio.items[0],
            IlstItem {
                key: ItemKey::Atom(FourCc(*b"\xA9nam")),
                values: vec![ItemValue::Text(text("Song"))],
            }
        );
        let start = file
            .windows(512)
            .position(|window| window == cover)
            .expect("the cover octets are in the file");
        assert_eq!(
            audio.items[1],
            IlstItem {
                key: ItemKey::Atom(FourCc(*b"covr")),
                values: vec![ItemValue::Picture(PictureRef {
                    type_code: 13,
                    offset: u64::try_from(start).expect("the offset fits"),
                    len: 512,
                })],
            }
        );
    }

    /// Verifies: SEC-MED-017, SEC-MED-010
    #[test]
    fn skips_an_item_body_larger_than_the_read_limit() {
        let long = [b'a'; 300];
        let items = mp4_box(*b"\xA9nam", &data(1, &long));
        let file = [
            brand(),
            mp4_box(
                *b"moov",
                &[track(*b"soun", 44_100), udta(false, &items)].concat(),
            ),
        ]
        .concat();
        let audio = run(&file, &lowered(LimitKind::ReadBytes, 256), enough(&file))
            .expect("the probe asks only for what the host allows")
            .expect("the track is kept");
        assert_eq!(audio.track.entry.format, FourCc(*b"mp4a"));
        assert_eq!(
            audio.items,
            vec![IlstItem {
                key: ItemKey::Atom(FourCc(*b"\xA9nam")),
                values: vec![],
            }]
        );
        assert_eq!(
            audio.problems,
            vec![Mp4Problem::Metadata(fault(ParseFault::LimitExceeded {
                limit: LimitKind::ReadBytes,
                value: 308,
                max: 256,
                offset: 405,
            }))]
        );
    }

    /// Verifies: SEC-MED-010
    #[test]
    fn refuses_a_sample_description_larger_than_the_read_limit() {
        let file = [brand(), movie()].concat();
        assert_eq!(
            run(&file, &lowered(LimitKind::ReadBytes, 64), enough(&file)),
            Ok(Err(fault(ParseFault::LimitExceeded {
                limit: LimitKind::ReadBytes,
                value: 95,
                max: 64,
                offset: 141,
            })))
        );
    }

    /// Verifies: SEC-MED-017, SEC-MED-006
    #[test]
    fn skips_the_rest_of_the_list_past_the_tag_field_limit() {
        let file = [brand(), movie()].concat();
        let audio = run(&file, &lowered(LimitKind::TagFields, 2), enough(&file))
            .expect("the probe asks only for what the host allows")
            .expect("the track is kept");
        assert_eq!(audio.items.len(), 2);
        assert_eq!(
            audio.problems,
            vec![Mp4Problem::Metadata(fault(ParseFault::LimitExceeded {
                limit: LimitKind::TagFields,
                value: 3,
                max: 2,
                offset: 490,
            }))]
        );
    }

    /// Verifies: SEC-MED-017, SEC-MED-006
    #[test]
    fn skips_a_picture_past_the_size_or_count_limit() {
        let file = [brand(), movie()].concat();
        let audio = run(&file, &lowered(LimitKind::PictureBytes, 1), enough(&file))
            .expect("the probe asks only for what the host allows")
            .expect("the track is kept");
        assert_eq!(audio.items[2].values, vec![]);
        assert_eq!(
            audio.problems,
            vec![Mp4Problem::Metadata(fault(ParseFault::LimitExceeded {
                limit: LimitKind::PictureBytes,
                value: 2,
                max: 1,
                offset: 514,
            }))]
        );
        let audio = run(&file, &lowered(LimitKind::Pictures, 0), enough(&file))
            .expect("the probe asks only for what the host allows")
            .expect("the track is kept");
        assert_eq!(audio.items[2].values, vec![]);
        assert_eq!(
            audio.problems,
            vec![Mp4Problem::Metadata(fault(ParseFault::LimitExceeded {
                limit: LimitKind::Pictures,
                value: 1,
                max: 0,
                offset: 514,
            }))]
        );
    }

    #[test]
    fn keeps_the_outer_track_when_a_trak_is_nested_inside_it() {
        // An empty trak as the first child of the audio track: starting a
        // new TrackParts on every trak, or ending a track that is not a
        // child of moov, would drop the outer offset.
        let inner = mp4_box(*b"trak", &[]);
        let inner_len = inner.len() as u64;
        let audio_trak = mp4_box(*b"trak", &[inner, audio_mdia()].concat());
        let file = [
            brand(),
            mp4_box(*b"moov", &[audio_trak, udta(false, &items())].concat()),
        ]
        .concat();
        let mut wanted = expected(28, Some(brand_read()));
        let t = 36;
        wanted.track.entry.config = CodecConfig::Esds(Esds {
            object_type: 0x40,
            max_bitrate: 128_000,
            avg_bitrate: 96_000,
            specific: Some(Specific {
                range: t + 97 + 95 + inner_len..t + 97 + 97 + inner_len,
                audio: Some(AudioSpecificConfig {
                    object_type: 2,
                    sample_rate: SampleRate::new(44_100).ok(),
                    channel_config: 2,
                    extension: None,
                }),
            }),
        });
        wanted.sample_tables = SampleTableRanges {
            stts: Some(t + 208 + inner_len..t + 224 + inner_len),
            stsc: Some(t + 232 + inner_len..t + 252 + inner_len),
            sizes: Some(SampleSizes::Stsz(t + 260 + inner_len..t + 272 + inner_len)),
            offsets: Some(ChunkOffsets::Stco(t + 280 + inner_len..t + 292 + inner_len)),
        };
        wanted.items[2].values[0] = ItemValue::Picture(PictureRef {
            type_code: 13,
            offset: t + 292 + inner_len + 186,
            len: 2,
        });
        assert_eq!(probe(&file), Ok(wanted));
    }

    #[test]
    fn does_not_count_a_trak_inside_user_data_as_an_extra_audio_track() {
        let meta = full_box(
            *b"meta",
            0,
            0,
            &[hdlr(*b"mdir"), mp4_box(*b"ilst", &items())].concat(),
        );
        let user = mp4_box(*b"udta", &[meta, track(*b"soun", 8_000)].concat());
        let file = [
            brand(),
            mp4_box(*b"moov", &[track(*b"soun", 44_100), user].concat()),
        ]
        .concat();
        assert_eq!(probe(&file), Ok(expected(28, Some(brand_read()))));
    }

    #[test]
    fn ignores_a_handler_box_outside_the_media_box() {
        let extra = hdlr(*b"vide");
        let extra_len = extra.len() as u64;
        let audio_trak = mp4_box(*b"trak", &[audio_mdia(), extra].concat());
        let file = [
            brand(),
            mp4_box(*b"moov", &[audio_trak, udta(false, &items())].concat()),
        ]
        .concat();
        let mut wanted = expected(28, Some(brand_read()));
        wanted.items[2].values[0] = ItemValue::Picture(PictureRef {
            type_code: 13,
            offset: 28 + 8 + 292 + extra_len + 186,
            len: 2,
        });
        assert_eq!(probe(&file), Ok(wanted));
    }

    #[test]
    fn keeps_every_picture_in_a_cover_item() {
        let jpeg = [0xFF, 0xD8, 0xAA];
        let png = [0x89, 0x50, 0xBB];
        let cover = mp4_box(*b"covr", &[data(13, &jpeg), data(14, &png)].concat());
        let items = [mp4_box(*b"\xA9nam", &data(1, b"Song")), cover].concat();
        let file = [
            brand(),
            mp4_box(
                *b"moov",
                &[track(*b"soun", 44_100), udta(false, &items)].concat(),
            ),
        ]
        .concat();
        let audio = probe(&file).expect("the track is kept");
        let jpeg_at = u64::try_from(
            file.windows(jpeg.len())
                .position(|window| window == jpeg)
                .expect("the JPEG octets are in the file"),
        )
        .expect("the offset fits");
        let png_at = u64::try_from(
            file.windows(png.len())
                .position(|window| window == png)
                .expect("the PNG octets are in the file"),
        )
        .expect("the offset fits");
        assert_eq!(
            audio.items,
            vec![
                IlstItem {
                    key: ItemKey::Atom(FourCc(*b"\xA9nam")),
                    values: vec![ItemValue::Text(text("Song"))],
                },
                IlstItem {
                    key: ItemKey::Atom(FourCc(*b"covr")),
                    values: vec![
                        ItemValue::Picture(PictureRef {
                            type_code: 13,
                            offset: jpeg_at,
                            len: 3,
                        }),
                        ItemValue::Picture(PictureRef {
                            type_code: 14,
                            offset: png_at,
                            len: 3,
                        }),
                    ],
                },
            ]
        );
    }

    #[test]
    fn does_not_read_a_data_box_that_is_not_inside_an_item() {
        let junk = mp4_box(*b"data", &[0xAA; 300]);
        let junk_len = junk.len() as u64;
        let file = [
            brand(),
            mp4_box(
                *b"moov",
                &[track(*b"soun", 44_100), junk, udta(false, &items())].concat(),
            ),
        ]
        .concat();
        let junk_at = 28 + 8 + 292;
        let mut wanted = expected(28, Some(brand_read()));
        wanted.items[2].values[0] = ItemValue::Picture(PictureRef {
            type_code: 13,
            offset: junk_at + junk_len + 186,
            len: 2,
        });
        let mut probe = Probe::new(Limits::DEFAULT, enough(&file));
        let mut window = Window::start(file.len() as u64);
        let mut asked = Vec::new();
        let result = loop {
            match probe.resume(window) {
                Step::Done(result) => break result,
                Step::Need(request) => {
                    asked.push(request);
                    let start = usize::try_from(request.offset).expect("the offset fits");
                    window = Window {
                        offset: request.offset,
                        bytes: &file[start..start + request.len as usize],
                        file_len: file.len() as u64,
                    };
                }
            }
        };
        assert_eq!(result, Ok(wanted));
        assert!(
            asked.iter().all(|request| {
                request.offset + u64::from(request.len) <= junk_at
                    || (request.offset == junk_at && request.len <= 32)
                    || request.offset >= junk_at + junk_len
            }),
            "{asked:?}"
        );
    }

    #[test]
    fn skips_a_short_picture_header_or_meta_box() {
        let short_cover = mp4_box(*b"covr", &mp4_box(*b"data", &[0, 0]));
        let file = [
            brand(),
            mp4_box(
                *b"moov",
                &[track(*b"soun", 44_100), udta(false, &short_cover)].concat(),
            ),
        ]
        .concat();
        let audio = probe(&file).expect("the track is kept");
        assert_eq!(audio.track.entry.format, FourCc(*b"mp4a"));
        assert_eq!(audio.problems.len(), 1);
        let short_meta = mp4_box(*b"udta", &mp4_box(*b"meta", &[0, 0]));
        let file = [
            brand(),
            mp4_box(*b"moov", &[track(*b"soun", 44_100), short_meta].concat()),
        ]
        .concat();
        let audio = probe(&file).expect("the track is kept");
        assert_eq!(audio.track.entry.format, FourCc(*b"mp4a"));
        assert_eq!(audio.problems.len(), 1);
        // A truncated `meta` outside user data fails the file.
        let file = [
            brand(),
            mp4_box(
                *b"moov",
                &[track(*b"soun", 44_100), mp4_box(*b"meta", &[0, 0])].concat(),
            ),
        ]
        .concat();
        assert!(probe(&file).is_err());
    }

    #[test]
    fn parent_end_is_the_innermost_end_or_the_fallback() {
        assert_eq!(Probe::parent_end(&[], 9), 9);
        let stack = [Open {
            kind: FourCc(*b"ilst"),
            offset: 1,
            depth: Depth::CONTAINER_ROOT,
            at: 2,
            end: 7,
            children: 0,
        }];
        assert_eq!(Probe::parent_end(&stack, 9), 7);
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn refuses_to_ask_for_more_than_the_read_limits_allow() {
        let file = [brand(), movie(), mdat()].concat();
        // Walking headers, a 479-octet read is enough; the movie is not
        // asked for as one 480-octet blob.
        assert_eq!(
            run(&file, &lowered(LimitKind::ReadBytes, 479), enough(&file)),
            Ok(Ok(expected(28, Some(brand_read()))))
        );
        let mut probe = Probe::new(Limits::DEFAULT, enough(&file));
        let mut window = Window::start(file.len() as u64);
        let mut asked = Vec::new();
        loop {
            match probe.resume(window) {
                Step::Done(_) => break,
                Step::Need(request) => {
                    asked.push(request);
                    let start = usize::try_from(request.offset).expect("the offset fits");
                    window = Window {
                        offset: request.offset,
                        bytes: &file[start..start + request.len as usize],
                        file_len: file.len() as u64,
                    };
                }
            }
        }
        let total: u64 = asked.iter().map(|request| u64::from(request.len)).sum();
        let last = *asked.last().expect("the probe reads something");
        assert!(
            run(&file, &lowered(LimitKind::FileBytes, total), enough(&file))
                .is_ok_and(|probe| probe.is_ok())
        );
        assert_eq!(
            run(
                &file,
                &lowered(LimitKind::FileBytes, total - 1),
                enough(&file)
            ),
            Ok(Err(fault(ParseFault::LimitExceeded {
                limit: LimitKind::FileBytes,
                value: total,
                max: total - 1,
                offset: last.offset,
            })))
        );
        // A header read is refused like any other.
        assert_eq!(
            run(&file, &lowered(LimitKind::ReadBytes, 27), enough(&file)),
            Ok(Err(fault(ParseFault::LimitExceeded {
                limit: LimitKind::ReadBytes,
                value: 32,
                max: 27,
                offset: 0,
            })))
        );
    }

    #[test]
    fn reports_where_each_kind_of_error_happened() {
        let kind = FourCc(*b"mdhd");
        let errors = [
            fault(ParseFault::BudgetExceeded { offset: 1 }),
            Mp4Error::BoxTooSmall {
                offset: 2,
                size: 3,
                header_len: 8,
            },
            Mp4Error::Unexpected {
                kind,
                offset: 3,
                found: 9,
            },
            Mp4Error::Missing { kind, offset: 4 },
            Mp4Error::ZeroTimescale { offset: 5 },
            Mp4Error::NoAudioTrack { offset: 6 },
            Mp4Error::NoMovie { offset: 7 },
        ];
        assert_eq!(errors.map(|error| error.offset()), [1, 2, 3, 4, 5, 6, 7]);
    }

    /// Box headers, likely ones weighted up, so random files reach past the
    /// top level.
    fn header() -> impl Strategy<Value = Vec<u8>> {
        let kind = prop_oneof![
            Just(*b"moov"),
            Just(*b"trak"),
            Just(*b"mdia"),
            Just(*b"minf"),
            Just(*b"stbl"),
            Just(*b"stsd"),
            Just(*b"mdhd"),
            Just(*b"hdlr"),
            Just(*b"udta"),
            Just(*b"meta"),
            Just(*b"ilst"),
            Just(*b"ftyp"),
            Just(*b"uuid"),
            any::<[u8; 4]>(),
        ];
        (prop_oneof![0_u32..64, Just(0), Just(1), any::<u32>()], kind)
            .prop_map(|(size, kind)| [size.to_be_bytes().to_vec(), kind.to_vec()].concat())
    }

    proptest! {
        /// Verifies: SEC-MED-001, SEC-MED-007, SEC-MED-008, SEC-MED-010
        #[test]
        fn returns_for_any_bytes_without_a_refused_read_or_a_spent_budget(
            pieces in vec(prop_oneof![header(), vec(any::<u8>(), 0..24)], 0..24),
        ) {
            let file: Vec<u8> = pieces.concat();
            let result = on_small_stack(move || {
                let budget = enough(&file);
                run(&file, &Limits::DEFAULT, budget).map(Result::err)
            });
            // The host never refuses a request, and the promised budget is
            // never spent.
            prop_assert!(result.is_ok(), "{:?}", result);
            prop_assert!(
                !matches!(result, Ok(Some(Mp4Error::Fault(ParseFault::BudgetExceeded { .. })))),
                "{:?}",
                result
            );
        }
    }
}
