//! The sans-I/O protocol between a parser and whoever holds the file.
//!
//! The core does no I/O. A parser is a state machine that is fed byte
//! windows and answers either "read `len` octets at `offset`" or a final
//! result. The worker host serves those requests with `pread` on the
//! descriptor it was given, and [`drive`] serves them from memory for tests
//! and for callers that already hold the bytes. Either way a [`ReadGuard`]
//! admits each request first, so no parser can read more than 16 MiB at a
//! time, past the end of its file, or more than 256 MiB of one file
//! (SEC-MED-010). Files with their metadata at the end, such as MP4 `moov`,
//! stay bounded without being loaded whole.
//!
//! # The protocol
//!
//! 1. The host calls [`SansIo::resume`] with [`Window::start`]: no octets at
//!    offset 0, so the parser learns how long the file is.
//! 2. The parser answers [`Step::Need`] with one [`ReadRequest`], or
//!    [`Step::Done`] with its result, which carries the parser's own errors.
//! 3. The host admits the request through its [`ReadGuard`], reads the
//!    octets, and calls `resume` again with them. A request the guard
//!    refuses ends the parse with a [`DriveError`].
//!
//! Every admitted request reads at least one octet and counts against the
//! per-file cap, so a parser that never stops asking is stopped after at
//! most that many reads (SEC-MED-008).

use super::cursor::Cursor;
use super::limits::{LimitKind, Limits};

/// A parser's request for `len` octets starting at file offset `offset`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadRequest {
    /// Where the octets start, from the start of the file.
    pub offset: u64,
    /// How many octets to read.
    pub len: u32,
}

/// What a parser answers each time it is resumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step<T> {
    /// Resume me with these octets.
    Need(ReadRequest),
    /// The parse is over.
    Done(T),
}

/// Octets of a file handed to a parser, with where they start and how long
/// the whole file is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window<'a> {
    /// The file offset of the first octet in `bytes`.
    pub offset: u64,
    /// The octets the parser asked for.
    pub bytes: &'a [u8],
    /// The length of the whole file.
    pub file_len: u64,
}

impl<'a> Window<'a> {
    /// The first window of every parse: no octets, at offset 0, in a file of
    /// `file_len` octets.
    #[must_use]
    pub const fn start(file_len: u64) -> Self {
        Self {
            offset: 0,
            bytes: &[],
            file_len,
        }
    }

    /// A cursor over this window's octets that reports absolute file
    /// offsets.
    #[must_use]
    pub const fn cursor(&self) -> Cursor<'a> {
        Cursor::at(self.bytes, self.offset)
    }
}

/// A parser written as a sans-I/O state machine.
pub trait SansIo {
    /// What the parse produces, including the parser's own errors.
    type Output;

    /// Continues the parse with the octets of the previous request, or with
    /// [`Window::start`] the first time.
    fn resume(&mut self, window: Window<'_>) -> Step<Self::Output>;
}

/// Why a host refused a parser's read request and ended the parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveError {
    /// The request was for zero octets, which would never advance.
    Empty {
        /// Where the request started.
        offset: u64,
    },
    /// The request was longer than one read may be (SEC-MED-010).
    TooLong {
        /// Where the request started.
        offset: u64,
        /// How many octets it asked for.
        len: u32,
        /// The most one read may ask for.
        max: u64,
    },
    /// The request ran past the end of the file.
    PastEnd {
        /// Where the request started.
        offset: u64,
        /// How many octets it asked for.
        len: u32,
        /// The length of the file.
        file_len: u64,
    },
    /// The request would take the octets read from this file past the
    /// per-file cap (SEC-MED-010).
    OverFileCap {
        /// Where the request started.
        offset: u64,
        /// How many octets it asked for.
        len: u32,
        /// Octets already read from this file.
        read: u64,
        /// The most that may be read from one file.
        max: u64,
    },
}

/// Admits the read requests of one parse of one file.
///
/// Like a [`Budget`](super::Budget), a guard is deliberately not `Copy` or
/// `Clone`, so its count of octets read cannot be duplicated and reset.
#[derive(Debug, PartialEq, Eq)]
pub struct ReadGuard {
    file_len: u64,
    max_read: u64,
    max_total: u64,
    read: u64,
}

impl ReadGuard {
    /// A guard for a file of `file_len` octets, with the read and per-file
    /// caps of `limits`.
    #[must_use]
    pub const fn new(file_len: u64, limits: &Limits) -> Self {
        Self {
            file_len,
            max_read: limits.get(LimitKind::ReadBytes),
            max_total: limits.get(LimitKind::FileBytes),
            read: 0,
        }
    }

    /// Admits `request`, counting its octets against the per-file cap.
    ///
    /// # Errors
    ///
    /// Returns the [`DriveError`] for a request of zero octets, one longer
    /// than one read may be, one that runs past the end of the file, or one
    /// that would take this file past the per-file cap, checked in that
    /// order. A refused request counts for nothing.
    pub fn admit(&mut self, request: ReadRequest) -> Result<(), DriveError> {
        let ReadRequest { offset, len } = request;
        let wide = u64::from(len);
        if wide == 0 {
            return Err(DriveError::Empty { offset });
        }
        if wide > self.max_read {
            return Err(DriveError::TooLong {
                offset,
                len,
                max: self.max_read,
            });
        }
        // An end past u64::MAX is past the end of any file.
        if offset
            .checked_add(wide)
            .is_none_or(|end| end > self.file_len)
        {
            return Err(DriveError::PastEnd {
                offset,
                len,
                file_len: self.file_len,
            });
        }
        let total = self.read.saturating_add(wide);
        if total > self.max_total {
            return Err(DriveError::OverFileCap {
                offset,
                len,
                read: self.read,
                max: self.max_total,
            });
        }
        self.read = total;
        Ok(())
    }

    /// Octets admitted so far.
    #[must_use]
    pub const fn bytes_read(&self) -> u64 {
        self.read
    }
}

/// Runs `parser` to the end over `file`, which is already in memory,
/// serving every read request the guard admits under `limits`.
///
/// # Errors
///
/// Returns the [`DriveError`] for the first request the guard refuses.
pub fn drive<P: SansIo>(
    mut parser: P,
    file: &[u8],
    limits: &Limits,
) -> Result<P::Output, DriveError> {
    // A slice never holds more than u64::MAX octets.
    let file_len = u64::try_from(file.len()).unwrap_or(u64::MAX);
    let mut guard = ReadGuard::new(file_len, limits);
    let mut window = Window::start(file_len);
    loop {
        match parser.resume(window) {
            Step::Done(output) => return Ok(output),
            Step::Need(request) => {
                guard.admit(request)?;
                window = Window {
                    offset: request.offset,
                    bytes: admitted(file, request),
                    file_len,
                };
            }
        }
    }
}

/// The octets of `request`, which the guard has already placed inside
/// `file`, so each conversion and lookup succeeds.
fn admitted(file: &[u8], request: ReadRequest) -> &[u8] {
    let start = usize::try_from(request.offset).unwrap_or(usize::MAX);
    let len = usize::try_from(request.len).unwrap_or(usize::MAX);
    file.get(start..)
        .and_then(|rest| rest.get(..len))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::super::fault::ParseFault;
    use super::super::small_stack::on_small_stack;
    use super::*;
    use proptest::collection::vec;
    use proptest::prelude::*;

    /// 16 MiB, the longest read SEC-MED-010 allows.
    const MAX_READ: u32 = 16_777_216;
    /// 256 MiB, the most SEC-MED-010 lets a parser read of one file.
    const MAX_TOTAL: u64 = 268_435_456;

    /// One window as a test parser saw it.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Seen {
        offset: u64,
        bytes: Vec<u8>,
        file_len: u64,
    }

    impl Seen {
        fn of(window: Window<'_>) -> Self {
            Self {
                offset: window.offset,
                bytes: window.bytes.to_vec(),
                file_len: window.file_len,
            }
        }
    }

    /// A parser that makes a fixed list of requests, then returns every
    /// window it was given.
    struct Script<'a> {
        requests: std::slice::Iter<'a, ReadRequest>,
        seen: Vec<Seen>,
    }

    impl<'a> Script<'a> {
        fn new(requests: &'a [ReadRequest]) -> Self {
            Self {
                requests: requests.iter(),
                seen: Vec::new(),
            }
        }
    }

    impl SansIo for Script<'_> {
        type Output = Vec<Seen>;

        fn resume(&mut self, window: Window<'_>) -> Step<Vec<Seen>> {
            self.seen.push(Seen::of(window));
            self.requests.next().map_or_else(
                || Step::Done(std::mem::take(&mut self.seen)),
                |&request| Step::Need(request),
            )
        }
    }

    /// A parser that asks for the same octet forever. It gives up after
    /// `ceiling` resumes, so a driver that fails to stop it makes the test
    /// fail instead of hang.
    struct Greedy {
        resumes: u64,
        ceiling: u64,
    }

    impl SansIo for Greedy {
        type Output = u64;

        fn resume(&mut self, _: Window<'_>) -> Step<u64> {
            self.resumes += 1;
            if self.resumes > self.ceiling {
                return Step::Done(self.resumes);
            }
            Step::Need(ReadRequest { offset: 0, len: 1 })
        }
    }

    const fn request(offset: u64, len: u32) -> ReadRequest {
        ReadRequest { offset, len }
    }

    fn seen(offset: u64, bytes: &[u8], file_len: u64) -> Seen {
        Seen {
            offset,
            bytes: bytes.to_vec(),
            file_len,
        }
    }

    /// `Limits::DEFAULT` with the read and per-file caps lowered.
    fn lowered(max_read: u64, max_total: u64) -> Limits {
        Limits::DEFAULT
            .with_override(LimitKind::ReadBytes, max_read)
            .and_then(|limits| limits.with_override(LimitKind::FileBytes, max_total))
            .unwrap_or(Limits::DEFAULT)
    }

    /// The octets 0, 1, 2, ... of a `len`-octet file.
    fn file(len: u8) -> Vec<u8> {
        (0..len).collect()
    }

    #[test]
    fn starts_every_parse_with_no_octets_and_the_file_length() {
        assert_eq!(
            Window::start(1_234),
            Window {
                offset: 0,
                bytes: &[],
                file_len: 1_234,
            }
        );
    }

    #[test]
    fn a_window_cursor_reports_absolute_offsets() {
        let window = Window {
            offset: 100,
            bytes: &[0xAB, 0xCD],
            file_len: 1_000,
        };
        let mut cursor = window.cursor();
        assert_eq!(cursor.u8(), Ok(0xAB));
        assert_eq!(
            cursor.u16_be(),
            Err(ParseFault::Truncated {
                offset: 101,
                needed: 2,
                available: 1,
            })
        );
    }

    /// Verifies: SEC-MED-010
    #[test]
    fn serves_each_request_with_exactly_the_octets_it_names() {
        let bytes = file(32);
        let requests = [request(0, 4), request(10, 3), request(28, 4), request(2, 1)];
        assert_eq!(
            drive(Script::new(&requests), &bytes, &Limits::DEFAULT),
            Ok(vec![
                seen(0, &[], 32),
                seen(0, &[0, 1, 2, 3], 32),
                seen(10, &[10, 11, 12], 32),
                seen(28, &[28, 29, 30, 31], 32),
                seen(2, &[2], 32),
            ])
        );
    }

    #[test]
    fn finishes_without_reading_when_the_parser_needs_nothing() {
        assert_eq!(
            drive(Script::new(&[]), &file(5), &Limits::DEFAULT),
            Ok(vec![seen(0, &[], 5)])
        );
        assert_eq!(
            drive(Script::new(&[]), &[], &Limits::DEFAULT),
            Ok(vec![seen(0, &[], 0)])
        );
    }

    /// Verifies: SEC-MED-010
    #[test]
    fn refuses_a_request_over_16_mib() {
        assert_eq!(
            drive(
                Script::new(&[request(0, MAX_READ + 1)]),
                &file(8),
                &Limits::DEFAULT
            ),
            Err(DriveError::TooLong {
                offset: 0,
                len: MAX_READ + 1,
                max: 16_777_216,
            })
        );
    }

    /// Verifies: SEC-MED-010
    #[test]
    fn refuses_a_request_past_the_end_of_the_file() {
        let bytes = file(10);
        for (offset, len) in [(5, 6), (10, 1), (11, 1), (u64::MAX, 1)] {
            assert_eq!(
                drive(
                    Script::new(&[request(offset, len)]),
                    &bytes,
                    &Limits::DEFAULT
                ),
                Err(DriveError::PastEnd {
                    offset,
                    len,
                    file_len: 10,
                }),
                "offset {offset}, len {len}"
            );
        }
        assert_eq!(
            drive(Script::new(&[request(0, 1)]), &[], &Limits::DEFAULT),
            Err(DriveError::PastEnd {
                offset: 0,
                len: 1,
                file_len: 0,
            })
        );
        // Exactly to the end is inside the file.
        assert_eq!(
            drive(Script::new(&[request(4, 6)]), &bytes, &Limits::DEFAULT),
            Ok(vec![seen(0, &[], 10), seen(4, &[4, 5, 6, 7, 8, 9], 10)])
        );
    }

    /// Verifies: SEC-MED-010
    #[test]
    fn refuses_a_parser_that_asks_for_more_than_the_per_file_cap() {
        let requests = [request(0, 4), request(4, 4), request(8, 1)];
        assert_eq!(
            drive(Script::new(&requests), &file(10), &lowered(16, 8)),
            Err(DriveError::OverFileCap {
                offset: 8,
                len: 1,
                read: 8,
                max: 8,
            })
        );
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn refuses_a_request_for_zero_octets() {
        assert_eq!(
            drive(Script::new(&[request(3, 0)]), &file(10), &Limits::DEFAULT),
            Err(DriveError::Empty { offset: 3 })
        );
    }

    /// Verifies: SEC-MED-008, SEC-MED-010
    #[test]
    fn stops_a_parser_that_never_stops_asking() {
        let greedy = Greedy {
            resumes: 0,
            ceiling: 100,
        };
        assert_eq!(
            drive(greedy, &file(10), &lowered(16, 5)),
            Err(DriveError::OverFileCap {
                offset: 0,
                len: 1,
                read: 5,
                max: 5,
            })
        );
        // The parser's own ceiling is real: under a cap it cannot reach,
        // it gives up on its fourth resume, after three reads.
        let greedy = Greedy {
            resumes: 0,
            ceiling: 3,
        };
        assert_eq!(drive(greedy, &file(10), &lowered(16, 100)), Ok(4));
    }

    /// Verifies: SEC-MED-004, SEC-MED-010
    #[test]
    fn admits_reads_of_up_to_16_mib_anywhere_in_a_file_of_u64_max_octets() {
        let mut guard = ReadGuard::new(u64::MAX, &Limits::DEFAULT);
        assert_eq!(guard.admit(request(0, MAX_READ)), Ok(()));
        assert_eq!(
            guard.admit(request(u64::MAX - u64::from(MAX_READ), MAX_READ)),
            Ok(())
        );
        assert_eq!(guard.bytes_read(), 2 * u64::from(MAX_READ));
        assert_eq!(
            guard.admit(request(0, MAX_READ + 1)),
            Err(DriveError::TooLong {
                offset: 0,
                len: MAX_READ + 1,
                max: u64::from(MAX_READ),
            })
        );
        // The end would be one past u64::MAX.
        for offset in [u64::MAX - 10, u64::MAX] {
            assert_eq!(
                guard.admit(request(offset, 11)),
                Err(DriveError::PastEnd {
                    offset,
                    len: 11,
                    file_len: u64::MAX,
                })
            );
        }
        assert_eq!(guard.bytes_read(), 2 * u64::from(MAX_READ));
    }

    /// Verifies: SEC-MED-010
    #[test]
    fn admits_exactly_the_per_file_cap() {
        let mut guard = ReadGuard::new(u64::MAX, &Limits::DEFAULT);
        for index in 0..16 {
            assert_eq!(
                guard.admit(request(index * u64::from(MAX_READ), MAX_READ)),
                Ok(())
            );
        }
        assert_eq!(guard.bytes_read(), MAX_TOTAL);
        assert_eq!(
            guard.admit(request(0, 1)),
            Err(DriveError::OverFileCap {
                offset: 0,
                len: 1,
                read: MAX_TOTAL,
                max: MAX_TOTAL,
            })
        );
        assert_eq!(guard.bytes_read(), MAX_TOTAL);
    }

    #[test]
    fn takes_its_caps_from_the_limits() {
        let mut guard = ReadGuard::new(100, &lowered(4, 6));
        assert_eq!(guard.admit(request(0, 4)), Ok(()));
        assert_eq!(
            guard.admit(request(0, 5)),
            Err(DriveError::TooLong {
                offset: 0,
                len: 5,
                max: 4,
            })
        );
        assert_eq!(guard.admit(request(96, 2)), Ok(()));
        assert_eq!(
            guard.admit(request(0, 1)),
            Err(DriveError::OverFileCap {
                offset: 0,
                len: 1,
                read: 6,
                max: 6,
            })
        );
    }

    /// An independent model of `drive` over a scripted parser.
    fn model(
        bytes: &[u8],
        requests: &[ReadRequest],
        max_read: u64,
        max_total: u64,
    ) -> Result<Vec<Seen>, DriveError> {
        let file_len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        let mut read: u128 = 0;
        let mut windows = vec![seen(0, &[], file_len)];
        for &ReadRequest { offset, len } in requests {
            let wide = u128::from(len);
            if len == 0 {
                return Err(DriveError::Empty { offset });
            }
            if wide > u128::from(max_read) {
                return Err(DriveError::TooLong {
                    offset,
                    len,
                    max: max_read,
                });
            }
            if u128::from(offset) + wide > u128::from(file_len) {
                return Err(DriveError::PastEnd {
                    offset,
                    len,
                    file_len,
                });
            }
            if read + wide > u128::from(max_total) {
                return Err(DriveError::OverFileCap {
                    offset,
                    len,
                    read: u64::try_from(read).unwrap_or(u64::MAX),
                    max: max_total,
                });
            }
            read += wide;
            let start = usize::try_from(offset).unwrap_or(usize::MAX);
            let end = start + usize::try_from(len).unwrap_or(usize::MAX);
            windows.push(seen(offset, &bytes[start..end], file_len));
        }
        Ok(windows)
    }

    /// Checks the model against the driver on one case of each outcome, so
    /// the property below rests on a model known to agree on every branch.
    #[test]
    fn the_model_agrees_with_the_driver_on_each_outcome() {
        let bytes = file(16);
        let cases: [(&[ReadRequest], u64, u64); 5] = [
            (&[request(0, 4), request(12, 4)], 16, 64),
            (&[request(0, 4), request(5, 0)], 16, 64),
            (&[request(0, 17)], 16, 64),
            (&[request(10, 7)], 16, 64),
            (&[request(0, 8), request(8, 8)], 16, 12),
        ];
        let mut outcomes = Vec::new();
        for (requests, max_read, max_total) in cases {
            let expected = model(&bytes, requests, max_read, max_total);
            let actual = drive(Script::new(requests), &bytes, &lowered(max_read, max_total));
            assert_eq!(actual, expected, "{requests:?}");
            outcomes.push(expected.map(|windows| windows.len()));
        }
        assert_eq!(
            outcomes,
            [
                Ok(3),
                Err(DriveError::Empty { offset: 5 }),
                Err(DriveError::TooLong {
                    offset: 0,
                    len: 17,
                    max: 16,
                }),
                Err(DriveError::PastEnd {
                    offset: 10,
                    len: 7,
                    file_len: 16,
                }),
                Err(DriveError::OverFileCap {
                    offset: 8,
                    len: 8,
                    read: 8,
                    max: 12,
                }),
            ]
        );
    }

    /// Requests weighted so that most land inside a small file and under
    /// small caps, while zero lengths, huge lengths and offsets near
    /// `u64::MAX` still turn up in every run.
    fn any_request() -> impl Strategy<Value = ReadRequest> {
        (
            prop_oneof![
                3 => 0_u64..16,
                1 => 0_u64..72,
                1 => any::<u64>(),
                1 => Just(u64::MAX),
            ],
            prop_oneof![3 => 0_u32..8, 1 => 0_u32..24, 1 => any::<u32>()],
        )
            .prop_map(|(offset, len)| request(offset, len))
    }

    proptest! {
        /// Verifies: SEC-MED-001, SEC-MED-008, SEC-MED-010
        #[test]
        fn serves_or_refuses_every_request_exactly_as_the_model_does(
            bytes in vec(any::<u8>(), 0..64),
            requests in vec(any_request(), 0..12),
            max_read in prop_oneof![1_u64..8, 0_u64..32],
            max_total in prop_oneof![0_u64..16, 0_u64..128],
        ) {
            let expected = model(&bytes, &requests, max_read, max_total);
            let limits = lowered(max_read, max_total);
            let actual = on_small_stack(move || drive(Script::new(&requests), &bytes, &limits));
            prop_assert_eq!(actual, expected);
        }
    }
}
