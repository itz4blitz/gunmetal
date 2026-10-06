//! The worker's host loop: it serves a parser's read requests with `pread`
//! on a descriptor the server passed, and it serves the server's requests
//! one after another (media-and-parser-safety.md, sections 2 and 4).
//!
//! The core does no I/O. A parser there is a state machine that asks for
//! "`len` octets at `offset`" and is resumed with them. [`serve_one`] is the
//! other half inside a worker: it reads the file's length from its
//! descriptor, admits each request through the core's
//! [`ReadGuard`], reads exactly the octets asked for, and resumes the
//! parser. The guard refuses a request for no octets, for more than 16 MiB,
//! for octets past the end of the file, and any request that would take the
//! parse past 256 MiB of one file (SEC-MED-010), so a hostile file cannot
//! make its parser read without bound. Nothing here opens a path: the only
//! files a worker reads are the descriptors of a request (SEC-MED-020).
//!
//! [`serve`] is the loop around it: read a request, hand it to the job,
//! write the answer, until the server closes the socket.

use crate::ipc::{self, ChannelError, Received, Refusal};
use gunmetal_core::parse::{DriveError, Limits, ReadGuard, ReadRequest, SansIo, Step, Window};
use rustix::fs::fstat;
use rustix::io::pread;
use serde::Serialize;
use std::io::{self, Read, Write};
use std::os::fd::BorrowedFd;

/// Why the host could not run a parser to its end over one file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostError {
    /// The file's length could not be read from its descriptor.
    Size {
        /// The system's error number, when it gave one.
        errno: Option<i32>,
    },
    /// The parser asked for a read the host does not serve: no octets, too
    /// many at once, past the end of the file, or past the per-file cap
    /// (SEC-MED-010).
    Refused(DriveError),
    /// The system refused a read the host had admitted. A descriptor that
    /// is not open for reading answers `EBADF`, error number 9, as a
    /// closed one does.
    Read {
        /// Where the read started.
        offset: u64,
        /// The system's error number, when it gave one.
        errno: Option<i32>,
    },
    /// The file ended before a read the host had admitted did: it has
    /// become shorter since its length was read.
    Short {
        /// Where the read started.
        offset: u64,
        /// How many octets it asked for.
        len: u32,
    },
}

/// A file's octets: how many there are, and the ones at an offset.
///
/// [`drive`] holds the loop and the decisions and is tested against a
/// source that plays a script. A descriptor is the real one: each method
/// is one call with no decision in it.
trait Source {
    /// The length of the file.
    fn size(&self) -> io::Result<u64>;

    /// Reads octets from `offset` octets into the file and says how many
    /// it read: fewer than asked when it pleases, none only at the end of
    /// the file.
    fn read_at(&self, octets: &mut [u8], offset: u64) -> io::Result<usize>;
}

impl Source for BorrowedFd<'_> {
    fn size(&self) -> io::Result<u64> {
        // No file has a negative length; one that claimed to would read
        // as empty, and every request would be past its end.
        fstat(self)
            .map(|stat| u64::try_from(stat.st_size).unwrap_or(0))
            .map_err(io::Error::from)
    }

    fn read_at(&self, octets: &mut [u8], offset: u64) -> io::Result<usize> {
        pread(self, octets, offset).map_err(io::Error::from)
    }
}

/// A source read from an offset onwards, so that the standard library's
/// `read_exact` does the looping: it asks again after a short read or an
/// interruption, and reports a file that ends early.
struct At<'source, S> {
    /// The file.
    source: &'source S,
    /// Where the next read starts.
    offset: u64,
}

impl<S: Source> Read for At<'_, S> {
    fn read(&mut self, octets: &mut [u8]) -> io::Result<usize> {
        let count = self.source.read_at(octets, self.offset)?;
        self.offset = self
            .offset
            .saturating_add(u64::try_from(count).unwrap_or(u64::MAX));
        Ok(count)
    }
}

/// Reads every octet of `request`, which the guard has admitted, so it is
/// for at most 16 MiB.
fn read(source: &impl Source, request: ReadRequest) -> Result<Vec<u8>, HostError> {
    let mut octets = vec![0; usize::try_from(request.len).unwrap_or(usize::MAX)];
    let mut file = At {
        source,
        offset: request.offset,
    };
    file.read_exact(&mut octets)
        .map(|()| octets)
        .map_err(|error| {
            if error.kind() == io::ErrorKind::UnexpectedEof {
                HostError::Short {
                    offset: request.offset,
                    len: request.len,
                }
            } else {
                HostError::Read {
                    offset: request.offset,
                    errno: error.raw_os_error(),
                }
            }
        })
}

/// Runs `parser` to its end over `source`, serving every read request the
/// guard admits under `limits`.
fn drive<P: SansIo>(
    source: &impl Source,
    mut parser: P,
    limits: &Limits,
) -> Result<P::Output, HostError> {
    let file_len = source.size().map_err(|error| HostError::Size {
        errno: error.raw_os_error(),
    })?;
    let mut guard = ReadGuard::new(file_len, limits);
    // The first window of every parse: no octets, at offset 0.
    let mut offset = 0;
    let mut octets = Vec::new();
    loop {
        let window = Window {
            offset,
            bytes: &octets,
            file_len,
        };
        let request = match parser.resume(window) {
            Step::Done(output) => return Ok(output),
            Step::Need(request) => request,
        };
        guard.admit(request).map_err(HostError::Refused)?;
        octets = read(source, request)?;
        offset = request.offset;
    }
}

/// Runs `parser` to its end over `file`, a descriptor the server passed,
/// answering each of its read requests with `pread`.
///
/// Every request goes through the core's [`ReadGuard`] under `limits`
/// first: one read is at most 16 MiB and inside the file, and one parse
/// reads at most 256 MiB of it (SEC-MED-010). The parser's own errors are
/// part of its output.
///
/// # Errors
///
/// Returns [`HostError::Refused`] with the guard's reason for a request it
/// does not admit, [`HostError::Size`] and [`HostError::Read`] when the
/// descriptor cannot be measured or read, and [`HostError::Short`] when
/// the file ends before an admitted read does.
pub fn serve_one<P: SansIo>(
    file: BorrowedFd<'_>,
    parser: P,
    limits: &Limits,
) -> Result<P::Output, HostError> {
    drive(&file, parser, limits)
}

/// Serves the server's requests from `socket`, one after another, until
/// the server closes it: reads a request, hands it to `job` with its
/// files, and writes what `job` returns to `out`, the same socket.
///
/// A worker calls this once it has confined itself. `job` gets each
/// request's files as descriptors and closes them by dropping them.
///
/// Nothing here checks that the worker has confined itself: this function
/// and [`serve_one`] take no proof of confinement, which decision 11 of
/// architecture record 6 asks of the host loop. A follow-up package adds
/// the proof to the sandbox and makes both take it.
///
/// # Errors
///
/// Returns the [`ChannelError`] for a request that could not be read,
/// after answering it with [`Refusal::Request`], and for an answer that
/// could not be written. Either way the worker must end: it can no longer
/// tell where the next request starts, or has nobody to answer.
pub fn serve<A: Serialize>(
    socket: BorrowedFd<'_>,
    out: &mut impl Write,
    limits: &Limits,
    mut job: impl FnMut(Received) -> Result<A, Refusal>,
) -> Result<(), ChannelError> {
    loop {
        let received = match ipc::receive(socket, limits) {
            Ok(Some(received)) => received,
            Ok(None) => return Ok(()),
            Err(error) => {
                // The server may be gone too; the reason this worker ends
                // is the request, whether or not the answer got through.
                let _ = ipc::answer::<A>(out, &Err(Refusal::Request));
                return Err(error);
            }
        };
        ipc::answer(out, &job(received))?;
    }
}

#[cfg(test)]
mod tests {
    use super::{HostError, Source, drive, serve, serve_one};
    use crate::ipc::testing::{memory_file, pair, read_only_file, reopened};
    use crate::ipc::{
        AnswerError, ChannelError, Job, Received, Refusal, Revalidate, read_answer, send,
    };
    use gunmetal_core::parse::{DriveError, LimitKind, Limits, ReadRequest, SansIo, Step, Window};
    use gunmetal_core::wire::WireError;
    use rustix::fs::OFlags;
    use serde::{Deserialize, Serialize};
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::convert::Infallible;
    use std::io::{self, Write};
    use std::net::Shutdown;
    use std::os::fd::AsFd;
    use std::os::unix::net::UnixStream;

    /// What a file that was never read remembers.
    const NO_READS: [(u64, usize); 0] = [];

    /// One window as a test parser saw it.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Seen {
        offset: u64,
        bytes: Vec<u8>,
        file_len: u64,
    }

    fn seen(offset: u64, bytes: &[u8], file_len: u64) -> Seen {
        Seen {
            offset,
            bytes: bytes.to_vec(),
            file_len,
        }
    }

    /// A parser that makes a fixed list of requests, then returns every
    /// window it was given.
    struct Script {
        requests: VecDeque<ReadRequest>,
        seen: Vec<Seen>,
    }

    impl Script {
        fn new(requests: &[(u64, u32)]) -> Self {
            Self {
                requests: requests
                    .iter()
                    .map(|&(offset, len)| ReadRequest { offset, len })
                    .collect(),
                seen: Vec::new(),
            }
        }
    }

    impl SansIo for Script {
        type Output = Vec<Seen>;

        fn resume(&mut self, window: Window<'_>) -> Step<Vec<Seen>> {
            self.seen
                .push(seen(window.offset, window.bytes, window.file_len));
            match self.requests.pop_front() {
                Some(request) => Step::Need(request),
                None => Step::Done(std::mem::take(&mut self.seen)),
            }
        }
    }

    /// A parser that asks for the same octets for ever. It gives up after
    /// `ceiling` resumes, so a host that fails to stop it fails the test
    /// instead of hanging it.
    struct Greedy {
        request: ReadRequest,
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
            Step::Need(self.request)
        }
    }

    /// A file that plays a script. It claims a length, or fails to give
    /// one. It answers a read with the next prepared error number while
    /// there is one, and otherwise from its octets, at most `piece` at a
    /// time. It remembers where every read started and how many octets it
    /// was for.
    struct Tape {
        claimed: Result<u64, i32>,
        octets: Vec<u8>,
        piece: usize,
        failures: RefCell<VecDeque<i32>>,
        reads: RefCell<Vec<(u64, usize)>>,
    }

    impl Tape {
        /// A file of the octets 0, 1, 2, ... that says how long it is and
        /// answers every read whole.
        fn counting(len: u8) -> Self {
            Self {
                claimed: Ok(u64::from(len)),
                octets: (0..len).collect(),
                piece: usize::MAX,
                failures: RefCell::default(),
                reads: RefCell::default(),
            }
        }

        /// The reads it was asked for so far.
        fn reads(&self) -> Vec<(u64, usize)> {
            self.reads.borrow().clone()
        }
    }

    impl Source for Tape {
        fn size(&self) -> io::Result<u64> {
            self.claimed.map_err(io::Error::from_raw_os_error)
        }

        fn read_at(&self, octets: &mut [u8], offset: u64) -> io::Result<usize> {
            self.reads.borrow_mut().push((offset, octets.len()));
            if let Some(errno) = self.failures.borrow_mut().pop_front() {
                return Err(io::Error::from_raw_os_error(errno));
            }
            let start = usize::try_from(offset).unwrap().min(self.octets.len());
            let count = (self.octets.len() - start)
                .min(octets.len())
                .min(self.piece);
            octets[..count].copy_from_slice(&self.octets[start..start + count]);
            Ok(count)
        }
    }

    /// `Limits::DEFAULT` with the read and per-file caps lowered.
    fn lowered(max_read: u64, max_total: u64) -> Limits {
        Limits::DEFAULT
            .with_override(LimitKind::ReadBytes, max_read)
            .and_then(|limits| limits.with_override(LimitKind::FileBytes, max_total))
            .unwrap()
    }

    /// The octets 0, 1, 2, ... in a file in memory.
    fn counting_file(len: u8) -> std::os::fd::OwnedFd {
        memory_file(&(0..len).collect::<Vec<u8>>())
    }

    /// The parser is given the file's length before it asks for anything,
    /// and each window holds exactly the octets it asked for.
    ///
    /// Verifies: SEC-MED-010
    #[test]
    fn serves_each_request_with_exactly_the_octets_it_names() {
        let file = counting_file(32);
        let requests = [(0, 4), (10, 3), (28, 4), (2, 1)];
        assert_eq!(
            serve_one(file.as_fd(), Script::new(&requests), &Limits::DEFAULT),
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
    fn a_parser_that_needs_nothing_is_done_without_a_read() {
        let tape = Tape::counting(5);
        assert_eq!(
            drive(&tape, Script::new(&[]), &Limits::DEFAULT),
            Ok(vec![seen(0, &[], 5)])
        );
        assert_eq!(tape.reads(), NO_READS);
        let empty = memory_file(&[]);
        assert_eq!(
            serve_one(empty.as_fd(), Script::new(&[]), &Limits::DEFAULT),
            Ok(vec![seen(0, &[], 0)])
        );
    }

    /// The host reads what the guard admitted and nothing else: one read
    /// per request, at the offset and of the length the parser named.
    ///
    /// Verifies: SEC-MED-010
    #[test]
    fn every_read_of_the_file_is_one_the_parser_asked_for() {
        let tape = Tape::counting(32);
        let requests = [(0, 4), (10, 3), (28, 4), (2, 1)];
        assert_eq!(
            drive(&tape, Script::new(&requests), &Limits::DEFAULT),
            Ok(vec![
                seen(0, &[], 32),
                seen(0, &[0, 1, 2, 3], 32),
                seen(10, &[10, 11, 12], 32),
                seen(28, &[28, 29, 30, 31], 32),
                seen(2, &[2], 32),
            ])
        );
        assert_eq!(tape.reads(), [(0, 4), (10, 3), (28, 4), (2, 1)]);
    }

    /// 17 MiB is 17,825,792 octets. The request is refused before a
    /// buffer is made for it or the file is read.
    ///
    /// Verifies: SEC-MED-010
    #[test]
    fn a_parser_that_asks_for_17_mib_is_refused() {
        let tape = Tape::counting(8);
        assert_eq!(
            drive(&tape, Script::new(&[(0, 17_825_792)]), &Limits::DEFAULT),
            Err(HostError::Refused(DriveError::TooLong {
                offset: 0,
                len: 17_825_792,
                max: 16_777_216,
            }))
        );
        assert_eq!(tape.reads(), NO_READS);
        // One octet over 16 MiB is refused as well, through a descriptor.
        let file = counting_file(8);
        assert_eq!(
            serve_one(
                file.as_fd(),
                Script::new(&[(0, 16_777_217)]),
                &Limits::DEFAULT
            ),
            Err(HostError::Refused(DriveError::TooLong {
                offset: 0,
                len: 16_777_217,
                max: 16_777_216,
            }))
        );
    }

    /// The file's length is the one read from its descriptor.
    ///
    /// Verifies: SEC-MED-010
    #[test]
    fn a_parser_that_asks_past_the_end_of_the_file_is_refused() {
        let file = counting_file(10);
        for (offset, len) in [(5, 6), (10, 1), (11, 1), (u64::MAX, 1)] {
            assert_eq!(
                serve_one(
                    file.as_fd(),
                    Script::new(&[(offset, len)]),
                    &Limits::DEFAULT
                ),
                Err(HostError::Refused(DriveError::PastEnd {
                    offset,
                    len,
                    file_len: 10,
                })),
                "offset {offset}, len {len}"
            );
        }
        // Exactly to the end is inside the file.
        assert_eq!(
            serve_one(file.as_fd(), Script::new(&[(4, 6)]), &Limits::DEFAULT),
            Ok(vec![seen(0, &[], 10), seen(4, &[4, 5, 6, 7, 8, 9], 10)])
        );
        let tape = Tape::counting(10);
        assert_eq!(
            drive(&tape, Script::new(&[(4, 6), (5, 6)]), &Limits::DEFAULT),
            Err(HostError::Refused(DriveError::PastEnd {
                offset: 5,
                len: 6,
                file_len: 10,
            }))
        );
        assert_eq!(tape.reads(), [(4, 6)]);
    }

    #[test]
    fn a_parser_that_asks_for_no_octets_is_refused() {
        let file = counting_file(10);
        assert_eq!(
            serve_one(file.as_fd(), Script::new(&[(3, 0)]), &Limits::DEFAULT),
            Err(HostError::Refused(DriveError::Empty { offset: 3 }))
        );
    }

    /// The host stops a parse at the per-file cap of the limits it is
    /// given, lowered here to five octets so that the test is small. The
    /// cap it is given when nothing is lowered is 256 MiB.
    ///
    /// Verifies: SEC-MED-010
    #[test]
    fn a_parser_that_never_stops_asking_is_stopped_at_the_per_file_cap() {
        assert_eq!(Limits::DEFAULT.get(LimitKind::FileBytes), 268_435_456);
        let request = ReadRequest { offset: 0, len: 1 };
        let tape = Tape::counting(10);
        let greedy = Greedy {
            request,
            resumes: 0,
            ceiling: 100,
        };
        assert_eq!(
            drive(&tape, greedy, &lowered(16, 5)),
            Err(HostError::Refused(DriveError::OverFileCap {
                offset: 0,
                len: 1,
                read: 5,
                max: 5,
            }))
        );
        assert_eq!(tape.reads(), [(0, 1); 5]);
        // The parser's own ceiling is real: under a cap it cannot reach,
        // it gives up on its fourth resume, after three reads.
        let greedy = Greedy {
            request,
            resumes: 0,
            ceiling: 3,
        };
        assert_eq!(drive(&tape, greedy, &lowered(16, 100)), Ok(4));
    }

    #[test]
    fn a_read_answered_in_pieces_is_put_together() {
        let tape = Tape {
            piece: 3,
            ..Tape::counting(32)
        };
        assert_eq!(
            drive(&tape, Script::new(&[(10, 8)]), &Limits::DEFAULT),
            Ok(vec![
                seen(0, &[], 32),
                seen(10, &[10, 11, 12, 13, 14, 15, 16, 17], 32),
            ])
        );
        assert_eq!(tape.reads(), [(10, 8), (13, 5), (16, 2)]);
    }

    /// Error number 4 is `EINTR`: a signal arrived before any octet was
    /// read, and the read is asked again.
    #[test]
    fn an_interrupted_read_is_asked_again() {
        let tape = Tape::counting(8);
        tape.failures.borrow_mut().push_back(4);
        assert_eq!(
            drive(&tape, Script::new(&[(2, 3)]), &Limits::DEFAULT),
            Ok(vec![seen(0, &[], 8), seen(2, &[2, 3, 4], 8)])
        );
        assert_eq!(tape.reads(), [(2, 3), (2, 3)]);
    }

    /// The file claims ten octets and holds six: it shrank after its
    /// length was read.
    #[test]
    fn a_file_that_ends_before_an_admitted_read_does_is_reported() {
        let tape = Tape {
            claimed: Ok(10),
            ..Tape::counting(6)
        };
        assert_eq!(
            drive(&tape, Script::new(&[(4, 6)]), &Limits::DEFAULT),
            Err(HostError::Short { offset: 4, len: 6 })
        );
        assert_eq!(tape.reads(), [(4, 6), (6, 4)]);
    }

    /// Error number 5 is `EIO`.
    #[test]
    fn a_read_the_system_refuses_is_reported_with_its_error_number() {
        let tape = Tape::counting(8);
        tape.failures.borrow_mut().push_back(5);
        assert_eq!(
            drive(&tape, Script::new(&[(2, 3)]), &Limits::DEFAULT),
            Err(HostError::Read {
                offset: 2,
                errno: Some(5),
            })
        );
    }

    /// A descriptor opened with `O_PATH` names a file and cannot read it:
    /// its length can be read, and a read of it fails with `EBADF`, error
    /// number 9, as a read of a closed descriptor does.
    #[test]
    fn a_descriptor_that_cannot_be_read_is_reported() {
        let file = reopened(&[1, 2, 3, 4], OFlags::PATH);
        assert_eq!(
            serve_one(file.as_fd(), Script::new(&[(0, 4)]), &Limits::DEFAULT),
            Err(HostError::Read {
                offset: 0,
                errno: Some(9),
            })
        );
        // A parser that reads nothing of it still learns its length.
        assert_eq!(
            serve_one(file.as_fd(), Script::new(&[]), &Limits::DEFAULT),
            Ok(vec![seen(0, &[], 4)])
        );
    }

    #[test]
    fn a_file_whose_length_cannot_be_read_is_reported() {
        let tape = Tape {
            claimed: Err(5),
            ..Tape::counting(8)
        };
        assert_eq!(
            drive(&tape, Script::new(&[(0, 1)]), &Limits::DEFAULT),
            Err(HostError::Size { errno: Some(5) })
        );
        assert_eq!(tape.reads(), NO_READS);
    }

    /// A stand-in for a job's answer.
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    enum Reply {
        /// The octets of the window asked for.
        Octets(Vec<u8>),
        /// The host loop could not read them.
        Unread,
    }

    impl Revalidate for Reply {
        type Checked = Self;
        type Invalid = Infallible;

        fn revalidate(self) -> Result<Self, Infallible> {
            Ok(self)
        }
    }

    /// A stand-in job: the octets of a hash window of the request's first
    /// file, read through the host loop. Any other request is refused.
    fn window(received: Received) -> Result<Reply, Refusal> {
        let (Job::HashWindow { range }, Some(file)) = (received.job, received.files.first()) else {
            return Err(Refusal::Unsupported);
        };
        let len = u32::try_from(range.end - range.start).unwrap();
        let parser = Script::new(&[(range.start, len)]);
        Ok(
            serve_one(file.as_fd(), parser, &Limits::DEFAULT).map_or(Reply::Unread, |windows| {
                Reply::Octets(windows[1].bytes.clone())
            }),
        )
    }

    /// The server's reading of the next answer on `server`.
    fn reply(server: &UnixStream) -> Result<Reply, AnswerError<Infallible>> {
        read_answer::<Reply>(server, &Limits::DEFAULT)
    }

    /// Runs the worker's loop on `worker` with the stand-in job.
    fn served(worker: &UnixStream) -> Result<(), ChannelError> {
        let mut out = worker;
        serve(worker.as_fd(), &mut out, &Limits::DEFAULT, window)
    }

    #[test]
    fn serves_each_request_in_turn_until_the_server_closes_the_socket() {
        let (server, worker) = pair();
        let digits = read_only_file(b"0123456789");
        let letters = read_only_file(b"abcdef");
        let requests = [
            (Job::HashWindow { range: 2..5 }, vec![digits.as_fd()]),
            (Job::HashWindow { range: 0..6 }, vec![letters.as_fd()]),
            // Past the end of its file: the job's own answer says so.
            (Job::HashWindow { range: 4..8 }, vec![letters.as_fd()]),
            // Not the stand-in's job, and its job without a file.
            (Job::Probe { hint: None }, vec![digits.as_fd()]),
            (Job::HashWindow { range: 0..1 }, vec![]),
            // The loop went on after each refusal.
            (Job::HashWindow { range: 9..10 }, vec![digits.as_fd()]),
        ];
        for (job, files) in requests {
            assert_eq!(send(&server, job, &files), Ok(()));
        }
        server.shutdown(Shutdown::Write).unwrap();
        assert_eq!(served(&worker), Ok(()));
        drop(worker);
        let answers: Vec<_> = (0..7).map(|_| reply(&server)).collect();
        assert_eq!(
            answers,
            [
                Ok(Reply::Octets(b"234".to_vec())),
                Ok(Reply::Octets(b"abcdef".to_vec())),
                Ok(Reply::Unread),
                Err(AnswerError::Refused(Refusal::Unsupported)),
                Err(AnswerError::Refused(Refusal::Unsupported)),
                Ok(Reply::Octets(b"9".to_vec())),
                Err(AnswerError::Channel(ChannelError::Closed)),
            ]
        );
    }

    /// The length field declares 1,048,577 octets, one over the cap on a
    /// request. The worker answers that it could not read the request, and
    /// its loop ends with the reason. The server's end is still open, so a
    /// loop that went on to wait for another request would not return.
    #[test]
    fn a_request_that_cannot_be_read_is_refused_and_ends_the_loop() {
        let (server, worker) = pair();
        let mut writer = &server;
        writer.write_all(&[0x01, 0x00, 0x10, 0x00]).unwrap();
        assert_eq!(
            served(&worker),
            Err(ChannelError::Wire(WireError::TooLarge {
                len: 1_048_577,
                max: 1_048_576,
            }))
        );
        drop(worker);
        assert_eq!(
            [reply(&server), reply(&server)],
            [
                Err(AnswerError::Refused(Refusal::Request)),
                Err(AnswerError::Channel(ChannelError::Closed)),
            ]
        );
    }

    #[test]
    fn an_answer_that_cannot_be_written_ends_the_loop() {
        let (server, worker) = pair();
        let digits = read_only_file(b"0123456789");
        assert_eq!(
            send(&server, Job::HashWindow { range: 2..5 }, &[digits.as_fd()]),
            Ok(())
        );
        drop(server);
        assert_eq!(
            served(&worker),
            Err(ChannelError::Io(io::ErrorKind::BrokenPipe))
        );
    }

    /// The server is gone before the worker can say that it could not read
    /// the request: the loop still ends with the request's fault.
    #[test]
    fn a_refusal_that_cannot_be_written_still_ends_the_loop_with_its_reason() {
        let (server, worker) = pair();
        let mut writer = &server;
        writer.write_all(&[3, 0, 0, 0, 1, 0, 9]).unwrap();
        drop(server);
        assert_eq!(
            served(&worker),
            Err(ChannelError::Wire(WireError::UnknownKind { kind: 9 }))
        );
    }
}
