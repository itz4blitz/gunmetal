//! The worker protocol: what the server asks of a worker and what a worker
//! answers, as frames of the core's wire codec over the launcher's socket
//! pair (media-and-parser-safety.md, section 4).
//!
//! A request is one [`FrameKind::Request`] frame that holds a [`Request`]:
//! the [`Job`], and how many files travel with it. The files are open
//! descriptors, passed with `SCM_RIGHTS` beside the frame's first octets:
//! the media file first, then the sidecars the server has already resolved
//! under its path rules. A worker is given no path and cannot open one, so
//! the descriptors are its only input, and [`send`] passes none that is
//! open for writing (SEC-MED-020).
//!
//! A worker answers with one [`FrameKind::Response`] frame that holds the
//! job's answer, or with a [`FrameKind::Refusal`] frame that holds a
//! [`Refusal`].
//!
//! Neither side takes the other's word for a size:
//!
//! - **The worker reads a bounded request.** [`receive`] refuses a frame
//!   over [`MAX_REQUEST`] from its length field alone, and decodes it under
//!   the core's limits and step budget. It makes room for [`MAX_FILES`]
//!   descriptors beside each message it reads and counts them across the
//!   messages of one frame: it stops at a control message the system cut
//!   short and at the message that brings one descriptor too many, and it
//!   refuses a request that names more than [`MAX_FILES`] files or does
//!   not come with the number it names. A refused request's descriptors
//!   are closed.
//! - **The server treats an answer as hostile** (SEC-MED-023).
//!   [`read_answer`] refuses a frame over [`MAX_RESPONSE`], the codec's 32
//!   MiB, before it waits for one payload octet, decodes under the same
//!   limits and budget, and returns only what [`Revalidate::revalidate`]
//!   accepted. It reads with plain `read`, so a descriptor a worker passes
//!   beside its answer is discarded by the system and never received.
//!
//! The server's two functions, [`send`] and [`read_answer`], take the
//! [`UnixStream`] that the launcher's socket pair gave the server, so
//! neither can be handed a TCP socket (SEC-STD-040). The worker's
//! functions, [`receive`] and [`answer`], and the host loop around them,
//! take whatever descriptor and writer the worker holds: a worker is
//! started with its end of the pair as its only descriptors.
//!
//! ```compile_fail,E0308
//! use gunmetal_worker::ipc::{Job, send};
//! use std::net::TcpStream;
//!
//! fn over_tcp(socket: &TcpStream) {
//!     let _ = send(socket, Job::Probe { hint: None }, &[]);
//! }
//! ```
//!
//! The same call on the launcher's socket compiles:
//!
//! ```no_run
//! use gunmetal_worker::ipc::{Job, send};
//! use gunmetal_worker::sandbox::Inherited;
//!
//! let (_fds, socket) = Inherited::pair().unwrap();
//! let _ = send(&socket, Job::Probe { hint: None }, &[]);
//! ```
//!
//! A worker answers one request before it reads the next, and nothing
//! here bounds how long either takes: [`send`] and [`read_answer`] wait
//! without a deadline, which belongs to the worker pool (WP-078). An
//! answer carries no request id, so a worker whose exchange failed is
//! discarded and never sent another request. Both functions say so.

use gunmetal_core::parse::{Budget, Limits};
use gunmetal_core::wire::{self, Frame, FrameKind, MAX_FRAME, ProtocolVersion, WireError};
use rustix::fs::{OFlags, fcntl_getfl};
use rustix::io::retry_on_intr;
use rustix::net::{
    RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags, SendAncillaryBuffer,
    SendAncillaryMessage, SendFlags, recvmsg, sendmsg,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::io::{self, IoSlice, IoSliceMut, Read, Write};
use std::mem::MaybeUninit;
use std::ops::Range;
use std::os::fd::{BorrowedFd, OwnedFd};
use std::os::unix::net::UnixStream;

/// The most octets a request frame may hold after its length field: 1 MiB.
/// A request is a job's few parameters, so this is far more than one needs.
pub const MAX_REQUEST: u32 = 1_048_576;

/// The most octets an answer frame may hold after its length field: the
/// wire codec's cap, 32 MiB (SEC-MED-023).
pub const MAX_RESPONSE: u32 = MAX_FRAME;

/// The most files that travel with one request: the media file and its
/// resolved sidecars. A worker may hold 32 descriptors in all.
pub const MAX_FILES: usize = 8;

/// The longest side, in pixels, of one artwork derivative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Size(pub u16);

/// What a worker is asked to do with the files of one request.
///
/// The list is closed. A package that adds a job adds its variant here, one
/// line under the merge protocol, and owns the job's own file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Job {
    /// Read the media file's format, properties and tags.
    Probe {
        /// What the server already takes the format to be, if anything.
        hint: Option<String>,
    },
    /// Decode the artwork and produce a derivative of each size.
    Artwork {
        /// The sizes to produce.
        sizes: Vec<Size>,
    },
    /// Hash a window of the media file's octets, for its content identity.
    HashWindow {
        /// The window, in octets from the start of the file.
        range: Range<u64>,
    },
}

/// One request: a job, and how many files travel with it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    /// What to do.
    pub job: Job,
    /// How many descriptors travel with the request: the media file first,
    /// then the sidecars the server resolved.
    pub files: usize,
}

/// A request as a worker received it.
#[derive(Debug)]
pub struct Received {
    /// What to do.
    pub job: Job,
    /// The files that came with the request, in the order the server
    /// passed them. Each is closed when it is dropped.
    pub files: Vec<OwnedFd>,
}

/// Why a worker did not do a job, as the [`FrameKind::Refusal`] frame it
/// answers with says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Refusal {
    /// The worker could not read the request: it was over the cap, cut
    /// short, malformed, not a request, with more files than one request
    /// carries, or without the files it named. The worker ends after this
    /// answer, because it can no longer tell where the next request starts.
    Request,
    /// The worker does not do this job.
    Unsupported,
    /// The job's answer could not be written as one frame: it was over
    /// the cap, or could not be encoded.
    Answer,
}

/// A job's answer as it comes off the wire: decoded, and not yet believed.
///
/// [`read_answer`] returns only the checked form, so the server has no way
/// to store a value a worker sent without checking it first (SEC-MED-023).
pub trait Revalidate: DeserializeOwned {
    /// The answer once every string, number and enum in it has been
    /// checked.
    type Checked;
    /// Why an answer was not believed.
    type Invalid;

    /// Checks every string, number and enum in the answer through the
    /// server's own typed constructors.
    ///
    /// # Errors
    ///
    /// Returns the reason when any value in the answer is refused.
    fn revalidate(self) -> Result<Self::Checked, Self::Invalid>;
}

/// Why the server could not hand a request to a worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendError {
    /// More files than travel with one request.
    TooManyFiles {
        /// How many there were.
        files: usize,
    },
    /// A file is open for writing. A worker is given read-only descriptors
    /// only (SEC-MED-020).
    NotReadOnly {
        /// The file's position among those passed, counted from 0.
        index: usize,
    },
    /// The request could not be written as a frame, or its frame is over
    /// [`MAX_REQUEST`], which the worker would refuse.
    Wire(WireError),
    /// The socket refused the request, as it does once the worker is gone.
    Io(io::ErrorKind),
}

/// Why a frame could not be read from the socket, or was not what was
/// expected there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelError {
    /// The worker closed the socket where its answer should start.
    Closed,
    /// The socket closed inside a frame.
    Truncated,
    /// The socket could not be read or written.
    Io(io::ErrorKind),
    /// The codec refused the frame or its payload: a length over the cap,
    /// a frame too short for its header, an unknown kind, a payload that
    /// is not the type asked for or that goes past a limit or the step
    /// budget.
    Wire(WireError),
    /// The frame is in a protocol version this build does not speak.
    Version(ProtocolVersion),
    /// The frame is of a kind that has no place here.
    Unexpected(FrameKind),
    /// The request did not come with the number of files it named.
    Descriptors {
        /// How many files the request named.
        expected: usize,
        /// How many descriptors came with it.
        received: usize,
    },
    /// The request names more files than travel with one request,
    /// [`MAX_FILES`].
    TooManyFiles {
        /// How many files the request named.
        files: usize,
    },
    /// More descriptors came with the request's frame than travel with
    /// one request. The worker stops reading at the message that brought
    /// one too many.
    TooManyDescriptors {
        /// How many had come by then.
        received: usize,
    },
    /// The system cut short a control message beside the request's
    /// octets, because it held more than the worker makes room for: more
    /// than [`MAX_FILES`] descriptors with one message, or other control
    /// data besides them. What did not fit was discarded, so the request
    /// is not read.
    ControlTruncated,
}

impl From<io::Error> for ChannelError {
    fn from(error: io::Error) -> Self {
        Self::Io(error.kind())
    }
}

/// Why the server has no checked answer from a worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnswerError<I> {
    /// No answer could be read from the socket.
    Channel(ChannelError),
    /// The worker refused the request.
    Refused(Refusal),
    /// The answer decoded, and a value in it was refused when the server
    /// checked it.
    Invalid(I),
}

/// Whether `file` is open for reading only. A descriptor whose flags
/// cannot be read is not passed on either.
fn read_only(file: BorrowedFd<'_>) -> bool {
    fcntl_getfl(file).is_ok_and(|flags| flags.intersection(OFlags::RWMODE) == OFlags::RDONLY)
}

/// The frame of `request`, when it is one a worker will read.
fn request_frame(request: &Request) -> Result<Vec<u8>, SendError> {
    wire::write_frame(ProtocolVersion::CURRENT, FrameKind::Request, request)
        .and_then(|frame| {
            // The codec's own reader says whether the worker's cap admits
            // the frame, so nothing a worker would refuse is sent.
            let refused =
                wire::read_frame(&frame, MAX_REQUEST, &mut Budget::for_input(0, 0, 1)).err();
            refused.map_or(Ok(frame), Err)
        })
        .map_err(SendError::Wire)
}

/// Sends `job` to the worker at the other end of `socket`, with `files` as
/// the descriptors it works on: the media file first, then its sidecars.
///
/// The descriptors travel with `SCM_RIGHTS` beside the frame's first
/// octets. The worker gets its own descriptors for the same open files, so
/// the caller keeps and closes its own. A first call that a signal
/// interrupts before it has written anything is made again.
///
/// The call waits, without a deadline, until the socket has taken the
/// whole frame: a worker that has stopped reading keeps it waiting for as
/// long as the socket's buffer is full. A write timeout on the socket
/// bounds each write, not the request. The deadline on a file, and killing
/// the worker that misses it, belong to the worker pool (WP-078).
///
/// A request carries no id, and neither does its answer: [`read_answer`]
/// takes the next frame on the socket for the answer to the request sent
/// last. So the caller reads each answer before it sends the next request,
/// and after [`SendError::Io`], when part of the frame may have been
/// written, it discards the worker and never sends it another request.
///
/// # Errors
///
/// Returns [`SendError::TooManyFiles`] for more than [`MAX_FILES`] files,
/// [`SendError::NotReadOnly`] for a file open for writing, and
/// [`SendError::Wire`] when the request's frame would be over
/// [`MAX_REQUEST`], all before anything is written; and [`SendError::Io`]
/// when the socket refuses the request, after which the worker must be
/// discarded.
pub fn send(socket: &UnixStream, job: Job, files: &[BorrowedFd<'_>]) -> Result<(), SendError> {
    if files.len() > MAX_FILES {
        return Err(SendError::TooManyFiles { files: files.len() });
    }
    if let Some(index) = files.iter().position(|file| !read_only(*file)) {
        return Err(SendError::NotReadOnly { index });
    }
    let frame = request_frame(&Request {
        job,
        files: files.len(),
    })?;
    let mut space = [MaybeUninit::<u8>::uninit(); rustix::cmsg_space!(ScmRights(MAX_FILES))];
    let mut control = SendAncillaryBuffer::new(&mut space);
    if !files.is_empty() {
        // The buffer has room for `MAX_FILES` descriptors and there are no
        // more than that. Were one left out, the worker would refuse the
        // request for the files it names and did not get.
        control.push(SendAncillaryMessage::ScmRights(files));
    }
    let mut rest = socket;
    transmit(
        &frame,
        || {
            sendmsg(
                socket,
                &[IoSlice::new(&frame)],
                &mut control,
                SendFlags::NOSIGNAL,
            )
        },
        &mut rest,
    )
    .map_err(|error| SendError::Io(error.kind()))
}

/// Writes `frame`: `first` writes its first octets, with the descriptors
/// beside them, and says how many it wrote, and `rest` takes the octets
/// after those.
///
/// `first` is made again for as long as the system answers `EINTR`: a
/// signal arrived before it wrote anything, so nothing was sent and no
/// descriptor left. The writes to `rest` are plain writes, which the
/// standard library makes again by itself.
fn transmit(
    frame: &[u8],
    first: impl FnMut() -> rustix::io::Result<usize>,
    rest: &mut impl Write,
) -> io::Result<()> {
    let sent = retry_on_intr(first)?;
    rest.write_all(frame.get(sent..).unwrap_or_default())
}

/// The octets of one control message that holds [`MAX_FILES`]
/// descriptors.
const ROOM: usize = rustix::cmsg_aligned_space!(ScmRights(MAX_FILES));

/// Room for one control message of [`MAX_FILES`] descriptors, and no
/// more. It is aligned for the message's header, so none of it is lost to
/// alignment and the system can put exactly that many descriptors in it,
/// wherever it lies. Room sized with `cmsg_space!` holds up to two more,
/// depending on its address.
#[repr(C, align(8))]
struct Room([MaybeUninit<u8>; ROOM]);

/// The worker's end of the socket, read with `recvmsg` so that the
/// descriptors beside a request's octets are kept.
struct Receiving<'socket> {
    /// The socket.
    socket: BorrowedFd<'socket>,
    /// The descriptors received so far.
    files: Vec<OwnedFd>,
    /// Why the request is refused, once what came beside its octets shows
    /// that it is. The read then ends with an error, and [`receive`]
    /// returns this in its place.
    refused: Option<ChannelError>,
}

impl Read for Receiving<'_> {
    fn read(&mut self, octets: &mut [u8]) -> io::Result<usize> {
        // Room for `MAX_FILES` descriptors beside this message. The system
        // closes any that do not fit and marks the control message as cut
        // short.
        let mut room = Room([MaybeUninit::uninit(); ROOM]);
        let mut control = RecvAncillaryBuffer::new(&mut room.0);
        #[expect(
            clippy::disallowed_methods,
            reason = "descriptors arrive only with recvmsg, and the launcher's socket pair has no peer address for it to return (SEC-MED-020, SEC-OPS-037)"
        )]
        let received = recvmsg(
            self.socket,
            &mut [IoSliceMut::new(octets)],
            &mut control,
            RecvFlags::CMSG_CLOEXEC,
        )?;
        for message in control.drain() {
            if let RecvAncillaryMessage::ScmRights(files) = message {
                self.files.extend(files);
            }
        }
        // Either refusal ends the read here, so nothing more is read and
        // the reader holds the descriptors of two messages at most. They
        // are closed when the reader is dropped.
        if received.flags.contains(ReturnFlags::CTRUNC) {
            self.refused = Some(ChannelError::ControlTruncated);
            return Err(io::ErrorKind::InvalidData.into());
        }
        if self.files.len() > MAX_FILES {
            self.refused = Some(ChannelError::TooManyDescriptors {
                received: self.files.len(),
            });
            return Err(io::ErrorKind::InvalidData.into());
        }
        Ok(received.bytes)
    }
}

/// Reads the octets of one frame from `channel`, its length field
/// included, or `None` when the channel ends where a frame would start.
fn read_octets(mut channel: impl Read, max: u32) -> Result<Option<Vec<u8>>, ChannelError> {
    let mut octets = Vec::new();
    channel.by_ref().take(4).read_to_end(&mut octets)?;
    let Ok(length) = <[u8; 4]>::try_from(octets.as_slice()) else {
        return if octets.is_empty() {
            Ok(None)
        } else {
            Err(ChannelError::Truncated)
        };
    };
    // The codec refuses a length over the cap from the length field alone,
    // so nothing is read, held or waited for on the peer's say-so.
    if let Err(refused @ WireError::TooLarge { .. }) =
        wire::read_frame(&octets, max, &mut Budget::for_input(0, 0, 1))
    {
        return Err(ChannelError::Wire(refused));
    }
    let declared = u64::from(u32::from_le_bytes(length));
    let read = channel.take(declared).read_to_end(&mut octets)?;
    if u64::try_from(read).ok() != Some(declared) {
        return Err(ChannelError::Truncated);
    }
    Ok(Some(octets))
}

/// Reads the frame that `octets` hold, and refuses one in a protocol
/// version other than this build's.
fn open(octets: &[u8], max: u32) -> Result<Frame<'_>, ChannelError> {
    let frame = wire::read_frame(octets, max, &mut Budget::for_input(0, 0, 1))
        .map_err(ChannelError::Wire)?;
    if frame.version != ProtocolVersion::CURRENT {
        return Err(ChannelError::Version(frame.version));
    }
    Ok(frame)
}

/// Decodes a frame's payload as one `T`, under `limits` and the codec's
/// step budget for a payload of its length.
fn payload<T: DeserializeOwned>(frame: &Frame<'_>, limits: &Limits) -> Result<T, ChannelError> {
    wire::decode(
        frame.payload,
        limits,
        &mut wire::payload_budget(frame.payload),
    )
    .map_err(ChannelError::Wire)
}

/// Reads the next request from the worker's `socket`, with the files that
/// came with it. `None` means the server closed the socket between
/// requests, which is how it tells a worker to end.
///
/// # Errors
///
/// Returns the [`ChannelError`] for a socket that cannot be read or closes
/// inside a frame, a frame over [`MAX_REQUEST`], a frame in another
/// protocol version or of another kind, a payload that is not a
/// [`Request`] or goes past `limits`, a control message the system cut
/// short, more than [`MAX_FILES`] descriptors in all, a request that names
/// more than [`MAX_FILES`] files, and a request that came with more or
/// fewer files than it names. The descriptors that came with a refused
/// request are closed before this returns. After any of these errors the
/// worker cannot tell where the next request starts, and must end.
pub fn receive(socket: BorrowedFd<'_>, limits: &Limits) -> Result<Option<Received>, ChannelError> {
    let mut channel = Receiving {
        socket,
        files: Vec::new(),
        refused: None,
    };
    let read = read_octets(&mut channel, MAX_REQUEST);
    // What came beside the octets is the reason a read it refused ended;
    // the read's own error says only that it ended.
    let Some(octets) = read.map_err(|error| channel.refused.take().unwrap_or(error))? else {
        return Ok(None);
    };
    let frame = open(&octets, MAX_REQUEST)?;
    if frame.kind != FrameKind::Request {
        return Err(ChannelError::Unexpected(frame.kind));
    }
    let request: Request = payload(&frame, limits)?;
    if request.files > MAX_FILES {
        return Err(ChannelError::TooManyFiles {
            files: request.files,
        });
    }
    if channel.files.len() != request.files {
        return Err(ChannelError::Descriptors {
            expected: request.files,
            received: channel.files.len(),
        });
    }
    Ok(Some(Received {
        job: request.job,
        files: channel.files,
    }))
}

/// Writes a worker's answer to `out`, its socket: the job's answer as a
/// [`FrameKind::Response`] frame, or the refusal as a
/// [`FrameKind::Refusal`] frame.
///
/// An answer that cannot be written as one frame, because it is over
/// [`MAX_RESPONSE`] or cannot be encoded, is answered with
/// [`Refusal::Answer`] instead, so the server learns why it got none.
///
/// # Errors
///
/// Returns [`ChannelError::Io`] when the socket cannot be written.
pub fn answer<A: Serialize>(
    out: &mut impl Write,
    reply: &Result<A, Refusal>,
) -> Result<(), ChannelError> {
    let frame = match reply {
        Ok(output) => wire::write_frame(ProtocolVersion::CURRENT, FrameKind::Response, output),
        Err(refusal) => wire::write_frame(ProtocolVersion::CURRENT, FrameKind::Refusal, refusal),
    };
    frame
        .or_else(|_| {
            wire::write_frame(
                ProtocolVersion::CURRENT,
                FrameKind::Refusal,
                &Refusal::Answer,
            )
        })
        .map_err(ChannelError::Wire)
        .and_then(|frame| {
            out.write_all(&frame)
                .and_then(|()| out.flush())
                .map_err(ChannelError::from)
        })
}

/// Reads a worker's answer from the server's `socket`, and returns it only
/// once it has been checked (SEC-MED-023).
///
/// The frame is refused from its length field alone when it is over
/// [`MAX_RESPONSE`]; its payload is decoded under `limits` and the codec's
/// step budget; and the decoded answer goes through
/// [`Revalidate::revalidate`] before the caller sees any of it.
///
/// The call waits, without a deadline, for a whole frame: a worker that
/// never answers, or stops inside an answer, keeps it waiting for ever. A
/// read timeout on the socket bounds each read, not the answer, and ends
/// the call with [`ChannelError::Io`]. The deadline on a file, and killing
/// the worker that misses it, belong to the worker pool (WP-078).
///
/// An answer carries no request id: it is taken for the answer to the
/// request sent last only because a worker answers one request before it
/// reads the next. So after [`AnswerError::Channel`], whatever its reason,
/// the caller discards the worker and never sends it another request. The
/// server no longer knows where the next frame starts, and an answer that
/// came late would be read as the answer to the request sent after it.
///
/// # Errors
///
/// Returns [`AnswerError::Channel`] when no answer could be read: the
/// worker closed the socket ([`ChannelError::Closed`]), which is also what
/// a worker that died looks like, or the frame or its payload was refused.
/// Returns [`AnswerError::Refused`] when the worker answered with a
/// refusal, and [`AnswerError::Invalid`] when the answer decoded and a
/// value in it was refused.
pub fn read_answer<A: Revalidate>(
    socket: &UnixStream,
    limits: &Limits,
) -> Result<A::Checked, AnswerError<A::Invalid>> {
    let octets = read_octets(socket, MAX_RESPONSE)
        .and_then(|octets| octets.ok_or(ChannelError::Closed))
        .map_err(AnswerError::Channel)?;
    let frame = open(&octets, MAX_RESPONSE).map_err(AnswerError::Channel)?;
    match frame.kind {
        FrameKind::Response => payload::<A>(&frame, limits)
            .map_err(AnswerError::Channel)?
            .revalidate()
            .map_err(AnswerError::Invalid),
        FrameKind::Refusal => Err(payload::<Refusal>(&frame, limits)
            .map_or_else(AnswerError::Channel, AnswerError::Refused)),
        other => Err(AnswerError::Channel(ChannelError::Unexpected(other))),
    }
}

/// Files and sockets for this crate's unit tests.
#[cfg(test)]
pub(crate) mod testing {
    use rustix::fs::{CWD, MemfdFlags, Mode, OFlags, memfd_create, openat};
    use std::fs::File;
    use std::io::Write;
    use std::os::fd::{AsRawFd, OwnedFd};
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    /// A new file in memory that holds `octets`, open for reading and
    /// writing.
    pub(crate) fn memory_file(octets: &[u8]) -> OwnedFd {
        let file = memfd_create("gunmetal-worker-test", MemfdFlags::CLOEXEC).unwrap();
        let mut writer = File::from(file);
        writer.write_all(octets).unwrap();
        OwnedFd::from(writer)
    }

    /// A new file in memory that holds `octets`, opened again with `flags`
    /// through this process's own descriptor table: read-only, as the
    /// server's path rules open a library file; write-only; or `O_PATH`,
    /// which names the file and cannot read it, so that a read fails with
    /// `EBADF` as it does on a closed descriptor.
    pub(crate) fn reopened(octets: &[u8], flags: OFlags) -> OwnedFd {
        let file = memory_file(octets);
        #[expect(
            clippy::disallowed_methods,
            reason = "the test opens its own in-memory file again through /proc/self/fd, a fixed path, to hold it with other access rights (SEC-MED-020)"
        )]
        let again = openat(
            CWD,
            format!("/proc/self/fd/{}", file.as_raw_fd()),
            flags | OFlags::CLOEXEC,
            Mode::empty(),
        );
        again.unwrap()
    }

    /// A read-only descriptor of a new file that holds `octets`.
    pub(crate) fn read_only_file(octets: &[u8]) -> OwnedFd {
        reopened(octets, OFlags::RDONLY)
    }

    /// A connected socket pair: the server's end, then the worker's. Both
    /// give up a read or a write after two seconds, so a test that would
    /// wait for ever fails instead.
    pub(crate) fn pair() -> (UnixStream, UnixStream) {
        let (server, worker) = UnixStream::pair().unwrap();
        for end in [&server, &worker] {
            end.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
            end.set_write_timeout(Some(Duration::from_secs(2))).unwrap();
        }
        (server, worker)
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{memory_file, pair, read_only_file, reopened};
    use super::{
        AnswerError, ChannelError, Job, MAX_FILES, MAX_REQUEST, MAX_RESPONSE, Received, Refusal,
        Revalidate, SendError, Size, answer, read_answer, receive, send, transmit,
    };
    use gunmetal_core::parse::{LimitKind, Limits, ParseFault};
    use gunmetal_core::wire::{FrameKind, PostcardError, ProtocolVersion, WireError};
    use rustix::fs::OFlags;
    use rustix::io::{Errno, FdFlags, fcntl_getfd, pread};
    use rustix::net::sockopt::set_socket_passcred;
    use rustix::net::{SendAncillaryBuffer, SendAncillaryMessage, SendFlags, sendmsg};
    use serde::{Deserialize, Serialize};
    use std::io::{self, IoSlice, Read, Write};
    use std::mem::MaybeUninit;
    use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    /// The kind octets of the wire codec's frames.
    const REQUEST: u8 = 1;
    const RESPONSE: u8 = 2;
    const PART: u8 = 3;
    const END: u8 = 4;
    const REFUSAL: u8 = 5;

    /// A frame written out by hand: the length of what follows, protocol
    /// version 1, the kind octet and the payload.
    fn frame(kind: u8, payload: &[u8]) -> Vec<u8> {
        let length = u32::try_from(payload.len() + 3).unwrap();
        let mut octets = length.to_le_bytes().to_vec();
        octets.extend([1, 0, kind]);
        octets.extend(payload);
        octets
    }

    /// Writes `octets` to `socket`, as the peer of its other end would.
    fn write(socket: &UnixStream, octets: &[u8]) {
        let mut writer = socket;
        writer.write_all(octets).unwrap();
    }

    /// Writes `octets` to `socket` as one message, with `files` beside
    /// them: nine at most, one more than a worker makes room for.
    fn write_with(socket: &UnixStream, octets: &[u8], files: &[BorrowedFd<'_>]) {
        let mut space = [MaybeUninit::<u8>::uninit(); rustix::cmsg_space!(ScmRights(9))];
        let mut control = SendAncillaryBuffer::new(&mut space);
        assert!(control.push(SendAncillaryMessage::ScmRights(files)));
        let sent = sendmsg(
            socket,
            &[IoSlice::new(octets)],
            &mut control,
            SendFlags::empty(),
        );
        assert_eq!(sent, Ok(octets.len()));
    }

    /// Everything the other end of `socket` wrote before it closed.
    fn all_of(socket: &UnixStream) -> Vec<u8> {
        let mut reader = socket;
        let mut octets = Vec::new();
        reader.read_to_end(&mut octets).unwrap();
        octets
    }

    /// The first octets of the file behind `file`, at most sixteen.
    fn contents(file: &OwnedFd) -> Vec<u8> {
        let mut octets = [0_u8; 16];
        let count = pread(file, &mut octets, 0).unwrap();
        octets[..count].to_vec()
    }

    /// What `receive` returned, with its files counted, so that it can be
    /// compared whole.
    fn arrived(
        received: &Result<Option<Received>, ChannelError>,
    ) -> Result<Option<(Job, usize)>, ChannelError> {
        match received {
            Ok(Some(request)) => Ok(Some((request.job.clone(), request.files.len()))),
            Ok(None) => Ok(None),
            Err(error) => Err(error.clone()),
        }
    }

    /// What the worker receives next on its end of the socket.
    fn incoming(worker: &UnixStream) -> Result<Option<(Job, usize)>, ChannelError> {
        arrived(&receive(worker.as_fd(), &Limits::DEFAULT))
    }

    /// How long `within` waits to show that a copy is still open.
    const BRIEFLY: Duration = Duration::from_millis(20);

    /// How long `within` may wait to show that none is. The end of the
    /// stream is there at once when none is; this only bounds a test that
    /// would otherwise fail by hanging.
    const AMPLY: Duration = Duration::from_secs(2);

    /// What `kept` reads within `wait`: whether a copy of its peer is open
    /// anywhere.
    ///
    /// The tests that show which descriptors are closed pass copies of one
    /// end of a socket pair as the files, and keep the other end. A
    /// descriptor that is passed is a new descriptor for the same socket,
    /// and the socket closes only when its last descriptor does, wherever
    /// that is: in the test, in what a worker holds, or on its way in a
    /// message nobody has read. Once the test has dropped its own copy,
    /// `kept` reads the end of the stream, `Ok(0)`, when no copy is open,
    /// and while one is it reads nothing and gives up after `wait` with
    /// `WouldBlock`.
    fn within(kept: &UnixStream, wait: Duration) -> Result<usize, io::ErrorKind> {
        kept.set_read_timeout(Some(wait)).unwrap();
        let mut reader = kept;
        read_until_answer_or_timeout(&mut reader)
    }

    /// How many times one read may be interrupted before the helper gives
    /// up and reports the interruption. A bound, like every other wait in
    /// the worker's tests, so a broken retry cannot hang the run.
    const INTERRUPTED_RETRIES: u32 = 8;

    /// Reads one octet, retrying while the read reports only an
    /// interruption: a signal (a concurrent build's `SIGCHLD`, say)
    /// interrupts the read before the timeout does, and the caller must
    /// see the socket's answer, `Ok(0)` or the timeout, never the
    /// interruption. Takes any reader so a test can answer with an
    /// interruption first and prove the retry.
    fn read_until_answer_or_timeout(reader: &mut impl io::Read) -> Result<usize, io::ErrorKind> {
        for _ in 0..INTERRUPTED_RETRIES {
            match reader.read(&mut [0_u8; 1]) {
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                other => return other.map_err(|error| error.kind()),
            }
        }
        Err(io::ErrorKind::Interrupted)
    }

    /// A reader with a script: each entry answers one read with an error
    /// of that kind, and when the script runs out, `last` answers every
    /// read after it. Counting the reads it was asked for.
    struct Scripted {
        script: Vec<io::ErrorKind>,
        last: Result<usize, io::ErrorKind>,
        reads: u32,
    }

    impl io::Read for Scripted {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.reads += 1;
            let answer = match self.script.pop() {
                Some(kind) => Err(io::Error::from(kind)),
                None => match self.last {
                    Ok(read) => Ok(read),
                    Err(kind) => Err(io::Error::from(kind)),
                },
            };
            answer.map(|read| read.min(buf.len()))
        }
    }

    /// One interruption is retried and the answer that follows wins: the
    /// reader answers three octets where one was asked, the helper hands
    /// back the one the buffer held, and exactly two reads were made.
    #[test]
    fn an_interrupted_read_is_retried_and_the_next_answer_wins() {
        let mut reader = Scripted {
            script: vec![io::ErrorKind::Interrupted],
            last: Ok(3),
            reads: 0,
        };
        assert_eq!(read_until_answer_or_timeout(&mut reader), Ok(1));
        assert_eq!(reader.reads, 2);
    }

    /// Three interruptions still give way to the answer that follows them,
    /// clamped to the one octet the caller's buffer holds.
    #[test]
    fn three_interrupted_reads_are_all_retried() {
        let mut reader = Scripted {
            script: vec![
                io::ErrorKind::Interrupted,
                io::ErrorKind::Interrupted,
                io::ErrorKind::Interrupted,
            ],
            last: Ok(3),
            reads: 0,
        };
        assert_eq!(read_until_answer_or_timeout(&mut reader), Ok(1));
        assert_eq!(reader.reads, 4);
    }

    /// A failure that is not an interruption reaches the caller as it is.
    #[test]
    fn a_later_failure_is_not_taken_for_an_interruption() {
        let mut reader = Scripted {
            script: vec![io::ErrorKind::Interrupted],
            last: Err(io::ErrorKind::NotConnected),
            reads: 0,
        };
        assert_eq!(
            read_until_answer_or_timeout(&mut reader),
            Err(io::ErrorKind::NotConnected)
        );
        assert_eq!(reader.reads, 2);
    }

    /// A reader that never stops interrupting is reported as interrupted
    /// after the bound, which asked for exactly the bound's reads.
    #[test]
    fn a_read_interrupted_forever_is_reported_after_the_bound() {
        let mut reader = Scripted {
            script: Vec::new(),
            last: Err(io::ErrorKind::Interrupted),
            reads: 0,
        };
        assert_eq!(
            read_until_answer_or_timeout(&mut reader),
            Err(io::ErrorKind::Interrupted)
        );
        assert_eq!(reader.reads, INTERRUPTED_RETRIES);
    }

    /// Runs `transmit` over `whole` with a first call that answers with
    /// `answers` in turn, and returns what it returned, with an error as
    /// its kind, how many times the call was made, and what was written
    /// after it.
    fn transmitted(
        whole: &[u8],
        answers: &[rustix::io::Result<usize>],
    ) -> (Result<(), io::ErrorKind>, usize, Vec<u8>) {
        let mut answers = answers.iter().copied();
        let mut calls = 0;
        let mut rest = Vec::new();
        let result = transmit(
            whole,
            || {
                calls += 1;
                answers.next().unwrap()
            },
            &mut rest,
        );
        (result.map_err(|error| error.kind()), calls, rest)
    }

    /// A stand-in for a job's answer: a level that is believed only up to
    /// 100, and a label.
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Reading {
        level: u8,
        label: String,
    }

    /// A reading whose level has been checked.
    #[derive(Debug, PartialEq)]
    struct Believed {
        level: u8,
        label: String,
    }

    /// A level over 100.
    #[derive(Debug, PartialEq)]
    struct TooHigh(u8);

    impl Revalidate for Reading {
        type Checked = Believed;
        type Invalid = TooHigh;

        fn revalidate(self) -> Result<Believed, TooHigh> {
            if self.level > 100 {
                return Err(TooHigh(self.level));
            }
            Ok(Believed {
                level: self.level,
                label: self.label,
            })
        }
    }

    /// The server's reading of the next answer on `server`.
    fn reading(server: &UnixStream) -> Result<Believed, AnswerError<TooHigh>> {
        read_answer::<Reading>(server, &Limits::DEFAULT)
    }

    /// An answer that cannot be encoded.
    struct Unwritable;

    impl Serialize for Unwritable {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("this answer cannot be written"))
        }
    }

    /// Each job's payload is written here by hand in postcard's encoding: a
    /// variant's index, an option's 0 or 1, lengths and numbers as varints,
    /// and last the count of files.
    #[test]
    fn each_job_is_one_request_frame_of_these_octets() {
        let cases: [(Job, usize, &[u8]); 4] = [
            (Job::Probe { hint: None }, 1, &[0, 0, 1]),
            (
                Job::Probe {
                    hint: Some("flac".to_owned()),
                },
                2,
                &[0, 1, 4, b'f', b'l', b'a', b'c', 2],
            ),
            (
                Job::Artwork {
                    sizes: vec![Size(64), Size(512)],
                },
                3,
                &[1, 2, 0x40, 0x80, 0x04, 3],
            ),
            (
                Job::HashWindow { range: 300..70_000 },
                0,
                &[2, 0xAC, 0x02, 0xF0, 0xA2, 0x04, 0],
            ),
        ];
        for (job, files, payload) in cases {
            let (server, worker) = pair();
            let held: Vec<OwnedFd> = (0..files).map(|_| read_only_file(b"x")).collect();
            let files: Vec<BorrowedFd<'_>> = held.iter().map(AsFd::as_fd).collect();
            assert_eq!(send(&server, job, &files), Ok(()));
            drop(server);
            assert_eq!(all_of(&worker), frame(REQUEST, payload));
        }
    }

    /// The worker reads the files' octets through the descriptors it
    /// received, which is the only way it is given a file.
    ///
    /// Verifies: SEC-MED-020
    #[test]
    fn a_request_arrives_with_its_job_and_its_files_as_descriptors() {
        let (server, worker) = pair();
        let media = read_only_file(b"media");
        let cover = read_only_file(b"cover");
        let job = Job::Artwork {
            sizes: vec![Size(128)],
        };
        assert_eq!(
            send(&server, job.clone(), &[media.as_fd(), cover.as_fd()]),
            Ok(())
        );
        let received = receive(worker.as_fd(), &Limits::DEFAULT);
        assert_eq!(arrived(&received), Ok(Some((job, 2))));
        let files = received.unwrap().unwrap().files;
        let read: Vec<Vec<u8>> = files.iter().map(contents).collect();
        assert_eq!(read, [b"media".to_vec(), b"cover".to_vec()]);
        // Closed on exec, so nothing a worker started could inherit them.
        let flags: Vec<FdFlags> = files
            .iter()
            .map(|file| fcntl_getfd(file).unwrap())
            .collect();
        assert_eq!(flags, [FdFlags::CLOEXEC, FdFlags::CLOEXEC]);
    }

    #[test]
    fn requests_arrive_one_at_a_time_in_the_order_they_were_sent() {
        let (server, worker) = pair();
        let file = read_only_file(b"one");
        assert_eq!(
            send(&server, Job::Probe { hint: None }, &[file.as_fd()]),
            Ok(())
        );
        assert_eq!(send(&server, Job::HashWindow { range: 0..3 }, &[]), Ok(()));
        drop(server);
        assert_eq!(incoming(&worker), Ok(Some((Job::Probe { hint: None }, 1))));
        assert_eq!(
            incoming(&worker),
            Ok(Some((Job::HashWindow { range: 0..3 }, 0)))
        );
        // The server closed the socket between requests: the end.
        assert_eq!(incoming(&worker), Ok(None));
    }

    #[test]
    fn eight_files_travel_with_one_request_and_nine_are_refused() {
        let (server, worker) = pair();
        let held: Vec<OwnedFd> = (0..9).map(|number| read_only_file(&[number])).collect();
        let files: Vec<BorrowedFd<'_>> = held.iter().map(AsFd::as_fd).collect();
        assert_eq!(MAX_FILES, 8);
        assert_eq!(
            send(&server, Job::Probe { hint: None }, &files),
            Err(SendError::TooManyFiles { files: 9 })
        );
        assert_eq!(
            send(&server, Job::Probe { hint: None }, &files[..8]),
            Ok(())
        );
        let received = receive(worker.as_fd(), &Limits::DEFAULT);
        assert_eq!(arrived(&received), Ok(Some((Job::Probe { hint: None }, 8))));
        let read: Vec<Vec<u8>> = received
            .unwrap()
            .unwrap()
            .files
            .iter()
            .map(contents)
            .collect();
        assert_eq!(read, [[0_u8], [1], [2], [3], [4], [5], [6], [7]]);
        // Nothing of the refused request was written.
        drop(server);
        assert_eq!(incoming(&worker), Ok(None));
    }

    /// Verifies: SEC-MED-020
    #[test]
    fn a_file_open_for_writing_is_never_passed_to_a_worker() {
        let (server, worker) = pair();
        let read_only = read_only_file(b"a library file");
        let read_write = memory_file(b"scratch");
        let write_only = reopened(b"scratch", OFlags::WRONLY);
        assert_eq!(
            send(
                &server,
                Job::Probe { hint: None },
                &[read_only.as_fd(), read_write.as_fd()]
            ),
            Err(SendError::NotReadOnly { index: 1 })
        );
        assert_eq!(
            send(&server, Job::Probe { hint: None }, &[write_only.as_fd()]),
            Err(SendError::NotReadOnly { index: 0 })
        );
        // Nothing of either request was written.
        drop(server);
        assert_eq!(incoming(&worker), Ok(None));
    }

    /// A hint of 1 MiB makes a frame of that, the variant's index, the
    /// option's octet, three octets of length, the count of files, and the
    /// version and kind: 1,048,585 octets after the length field.
    #[test]
    fn a_request_the_worker_would_refuse_for_its_size_is_never_sent() {
        let (server, worker) = pair();
        assert_eq!(MAX_REQUEST, 1_048_576);
        let hint = Some("a".repeat(1_048_576));
        assert_eq!(
            send(&server, Job::Probe { hint }, &[]),
            Err(SendError::Wire(WireError::TooLarge {
                len: 1_048_585,
                max: 1_048_576,
            }))
        );
        drop(server);
        assert_eq!(incoming(&worker), Ok(None));
    }

    #[test]
    fn a_request_for_a_worker_that_is_gone_is_an_error() {
        let (server, worker) = pair();
        drop(worker);
        assert_eq!(
            send(&server, Job::Probe { hint: None }, &[]),
            Err(SendError::Io(io::ErrorKind::BrokenPipe))
        );
    }

    /// Error number 4 is `EINTR`: a signal arrived before the call wrote
    /// anything, so nothing was sent and no descriptor left, and the call
    /// is made again. Here it is interrupted twice, then writes the first
    /// four octets of ten, and the other six follow. Error number 32 is
    /// `EPIPE`, the worker is gone, and the call is not made again.
    #[test]
    fn a_send_interrupted_before_any_octet_is_made_again() {
        let whole = frame(REQUEST, &[0, 0, 0]);
        assert_eq!(whole, [6, 0, 0, 0, 1, 0, REQUEST, 0, 0, 0]);
        assert_eq!(
            transmitted(&whole, &[Err(Errno::INTR), Err(Errno::INTR), Ok(4)]),
            (Ok(()), 3, vec![1, 0, REQUEST, 0, 0, 0])
        );
        assert_eq!(
            transmitted(&whole, &[Err(Errno::PIPE), Ok(10)]),
            (Err(io::ErrorKind::BrokenPipe), 1, Vec::new())
        );
    }

    #[test]
    fn a_request_that_ends_early_is_cut_short() {
        let whole = frame(REQUEST, &[0, 0, 0]);
        assert_eq!(whole.len(), 10);
        // Inside the length field, at its end, and inside the frame.
        for cut in [1, 3, 4, 6, 9] {
            let (server, worker) = pair();
            write(&server, &whole[..cut]);
            drop(server);
            assert_eq!(incoming(&worker), Err(ChannelError::Truncated), "{cut}");
        }
    }

    /// The server's end stays open and sends no payload: the worker
    /// refuses on the four octets of the length, without waiting for more.
    #[test]
    fn a_request_frame_over_the_cap_is_refused_from_its_length_alone() {
        let (server, worker) = pair();
        // 1,048,577 octets declared.
        write(&server, &[0x01, 0x00, 0x10, 0x00]);
        assert_eq!(
            incoming(&worker),
            Err(ChannelError::Wire(WireError::TooLarge {
                len: 1_048_577,
                max: 1_048_576,
            }))
        );
    }

    #[test]
    fn a_frame_that_is_not_a_request_is_refused() {
        let kinds = [
            (RESPONSE, FrameKind::Response),
            (PART, FrameKind::Part),
            (END, FrameKind::End),
            (REFUSAL, FrameKind::Refusal),
        ];
        for (octet, kind) in kinds {
            let (server, worker) = pair();
            write(&server, &frame(octet, &[0, 0, 0]));
            assert_eq!(incoming(&worker), Err(ChannelError::Unexpected(kind)));
        }
    }

    #[test]
    fn a_request_the_codec_or_this_build_cannot_read_is_refused() {
        let cases: [(&[u8], ChannelError); 6] = [
            // Protocol version 2.
            (
                &[6, 0, 0, 0, 2, 0, REQUEST, 0, 0, 0],
                ChannelError::Version(ProtocolVersion(2)),
            ),
            // A kind octet that names no kind.
            (
                &[3, 0, 0, 0, 1, 0, 9],
                ChannelError::Wire(WireError::UnknownKind { kind: 9 }),
            ),
            // A length too short for the version and kind.
            (
                &[2, 0, 0, 0, 1, 0],
                ChannelError::Wire(WireError::TooShort { len: 2 }),
            ),
            (
                &[0, 0, 0, 0],
                ChannelError::Wire(WireError::TooShort { len: 0 }),
            ),
            // A payload that ends after the job's variant.
            (
                &[4, 0, 0, 0, 1, 0, REQUEST, 0],
                ChannelError::Wire(WireError::Malformed {
                    offset: 1,
                    reason: PostcardError::DeserializeUnexpectedEnd,
                }),
            ),
            // An octet left after the request.
            (
                &[7, 0, 0, 0, 1, 0, REQUEST, 0, 0, 0, 9],
                ChannelError::Wire(WireError::Trailing { offset: 3, len: 1 }),
            ),
        ];
        for (octets, refused) in cases {
            let (server, worker) = pair();
            write(&server, octets);
            assert_eq!(incoming(&worker), Err(refused));
        }
    }

    /// The hint's four octets start at the payload's fourth octet, and the
    /// limit on a string is lowered to three.
    #[test]
    fn a_request_is_decoded_under_the_limits_it_is_given() {
        let (server, worker) = pair();
        let hint = Some("flac".to_owned());
        assert_eq!(send(&server, Job::Probe { hint }, &[]), Ok(()));
        let limits = Limits::DEFAULT
            .with_override(LimitKind::LongText, 3)
            .unwrap();
        assert_eq!(
            arrived(&receive(worker.as_fd(), &limits)),
            Err(ChannelError::Wire(WireError::Fault(
                ParseFault::LimitExceeded {
                    limit: LimitKind::LongText,
                    value: 4,
                    max: 3,
                    offset: 3,
                }
            )))
        );
    }

    /// A server that closed a file it meant to send, or sent one it did
    /// not announce, is told so: the worker never guesses which file is
    /// which.
    #[test]
    fn a_request_without_the_files_it_names_is_refused() {
        let file = read_only_file(b"media");
        let (server, worker) = pair();
        write_with(&server, &frame(REQUEST, &[0, 0, 2]), &[file.as_fd()]);
        assert_eq!(
            incoming(&worker),
            Err(ChannelError::Descriptors {
                expected: 2,
                received: 1,
            })
        );
        let (server, worker) = pair();
        write(&server, &frame(REQUEST, &[0, 0, 1]));
        assert_eq!(
            incoming(&worker),
            Err(ChannelError::Descriptors {
                expected: 1,
                received: 0,
            })
        );
        let (server, worker) = pair();
        write_with(&server, &frame(REQUEST, &[0, 0, 0]), &[file.as_fd()]);
        assert_eq!(
            incoming(&worker),
            Err(ChannelError::Descriptors {
                expected: 0,
                received: 1,
            })
        );
    }

    /// The worker makes room for eight descriptors beside each message it
    /// reads. Nine beside one message are one too many: the system passes
    /// eight, closes the ninth and marks the control message as cut short,
    /// and the worker refuses the request whether it names eight files or
    /// nine. It never takes the files that fitted for the ones it was
    /// sent.
    ///
    /// Each file here is a copy of one end of a socket pair, and the other
    /// end shows whether a copy is still open (`within`). Eight are read,
    /// and while the worker holds them the other end reads nothing; once
    /// it drops them, the other end reads the end of the stream. After a
    /// refusal it reads the end at once: the worker closed the eight it
    /// had received, and the system the ninth.
    #[test]
    fn a_ninth_file_beside_one_message_is_refused_as_a_control_message_cut_short() {
        let (server, worker) = pair();
        let (kept, passed) = pair();
        write_with(&server, &frame(REQUEST, &[0, 0, 8]), &[passed.as_fd(); 8]);
        let received = receive(worker.as_fd(), &Limits::DEFAULT);
        assert_eq!(arrived(&received), Ok(Some((Job::Probe { hint: None }, 8))));
        drop(passed);
        assert_eq!(within(&kept, BRIEFLY), Err(io::ErrorKind::WouldBlock));
        drop(received);
        assert_eq!(within(&kept, AMPLY), Ok(0));
        for named in [8, 9] {
            let (server, worker) = pair();
            let (kept, passed) = pair();
            write_with(
                &server,
                &frame(REQUEST, &[0, 0, named]),
                &[passed.as_fd(); 9],
            );
            assert_eq!(
                incoming(&worker),
                Err(ChannelError::ControlTruncated),
                "{named}"
            );
            drop(passed);
            assert_eq!(within(&kept, AMPLY), Ok(0), "{named}");
        }
    }

    /// The bound is on the files of one request, however many messages
    /// its frame comes in. Here the length field comes in one message and
    /// the rest of the frame in another, each with files beside it: eight
    /// in all are read, in the order they were sent, and eight then one
    /// more are refused. The worker closed all nine (`within`).
    #[test]
    fn the_files_of_one_request_are_counted_across_the_messages_of_its_frame() {
        let whole = frame(REQUEST, &[0, 0, 8]);
        let held: Vec<OwnedFd> = (0..8).map(|number| read_only_file(&[number])).collect();
        let files: Vec<BorrowedFd<'_>> = held.iter().map(AsFd::as_fd).collect();
        let (server, worker) = pair();
        write_with(&server, &whole[..4], &files[..4]);
        write_with(&server, &whole[4..], &files[4..]);
        let received = receive(worker.as_fd(), &Limits::DEFAULT);
        assert_eq!(arrived(&received), Ok(Some((Job::Probe { hint: None }, 8))));
        let read: Vec<Vec<u8>> = received
            .unwrap()
            .unwrap()
            .files
            .iter()
            .map(contents)
            .collect();
        assert_eq!(read, [[0_u8], [1], [2], [3], [4], [5], [6], [7]]);
        let whole = frame(REQUEST, &[0, 0, 9]);
        let (server, worker) = pair();
        let (kept, passed) = pair();
        write_with(&server, &whole[..4], &[passed.as_fd(); 8]);
        write_with(&server, &whole[4..], &[passed.as_fd()]);
        assert_eq!(
            incoming(&worker),
            Err(ChannelError::TooManyDescriptors { received: 9 })
        );
        drop(passed);
        assert_eq!(within(&kept, AMPLY), Ok(0));
    }

    /// Two messages of one frame, each with eight files beside it, and a
    /// request that names sixteen. The worker closed all sixteen
    /// (`within`).
    #[test]
    fn sixteen_files_in_two_messages_of_one_frame_are_refused() {
        let whole = frame(REQUEST, &[0, 0, 16]);
        let (server, worker) = pair();
        let (kept, passed) = pair();
        write_with(&server, &whole[..4], &[passed.as_fd(); 8]);
        write_with(&server, &whole[4..], &[passed.as_fd(); 8]);
        assert_eq!(
            incoming(&worker),
            Err(ChannelError::TooManyDescriptors { received: 16 })
        );
        drop(passed);
        assert_eq!(within(&kept, AMPLY), Ok(0));
    }

    /// The frame comes in three messages, with eight files, one and one.
    /// The worker stops reading at the second, so it never holds more than
    /// the descriptors of two messages, and the third is left in the
    /// socket: its octets are read afterwards, by a plain read that takes
    /// no descriptor, and the system closes the one beside them. Then no
    /// copy is open (`within`).
    #[test]
    fn the_worker_stops_reading_at_the_message_that_brings_a_file_too_many() {
        let whole = frame(REQUEST, &[0, 0, 10]);
        let (server, worker) = pair();
        let (kept, passed) = pair();
        write_with(&server, &whole[..4], &[passed.as_fd(); 8]);
        write_with(&server, &whole[4..5], &[passed.as_fd()]);
        write_with(&server, &whole[5..], &[passed.as_fd()]);
        assert_eq!(
            incoming(&worker),
            Err(ChannelError::TooManyDescriptors { received: 9 })
        );
        drop(server);
        assert_eq!(all_of(&worker), [0_u8, REQUEST, 0, 0, 10]);
        drop(passed);
        assert_eq!(within(&kept, AMPLY), Ok(0));
    }

    /// One request carries at most eight files, so a request that names
    /// nine is refused for that alone, with eight beside it or with none.
    /// Eight named and eight sent are read (the tests above). The worker
    /// closed the eight it had received (`within`).
    #[test]
    fn a_request_that_names_more_files_than_one_carries_is_refused() {
        let (server, worker) = pair();
        let (kept, passed) = pair();
        write_with(&server, &frame(REQUEST, &[0, 0, 9]), &[passed.as_fd(); 8]);
        assert_eq!(
            incoming(&worker),
            Err(ChannelError::TooManyFiles { files: 9 })
        );
        drop(passed);
        assert_eq!(within(&kept, AMPLY), Ok(0));
        let (server, worker) = pair();
        write(&server, &frame(REQUEST, &[0, 0, 9]));
        assert_eq!(
            incoming(&worker),
            Err(ChannelError::TooManyFiles { files: 9 })
        );
    }

    /// Every other refusal closes the descriptors the request brought as
    /// well: the reader holds them, and `receive` drops it with its error.
    /// One copy of a passed socket comes with each request here, and after
    /// the refusal no copy is open (`within`).
    #[test]
    fn a_refused_request_leaves_none_of_its_files_open() {
        let cases: [(Vec<u8>, ChannelError); 6] = [
            // It names two files, and one came.
            (
                frame(REQUEST, &[0, 0, 2]),
                ChannelError::Descriptors {
                    expected: 2,
                    received: 1,
                },
            ),
            // A frame of another kind.
            (
                frame(RESPONSE, &[0, 0, 1]),
                ChannelError::Unexpected(FrameKind::Response),
            ),
            // Protocol version 2.
            (
                vec![6, 0, 0, 0, 2, 0, REQUEST, 0, 0, 1],
                ChannelError::Version(ProtocolVersion(2)),
            ),
            // A payload that ends after the job's variant.
            (
                frame(REQUEST, &[0]),
                ChannelError::Wire(WireError::Malformed {
                    offset: 1,
                    reason: PostcardError::DeserializeUnexpectedEnd,
                }),
            ),
            // 1,048,577 octets declared.
            (
                vec![0x01, 0x00, 0x10, 0x00],
                ChannelError::Wire(WireError::TooLarge {
                    len: 1_048_577,
                    max: 1_048_576,
                }),
            ),
            // The server closes the socket inside the frame.
            (vec![6, 0, 0, 0, 1, 0], ChannelError::Truncated),
        ];
        for (octets, refused) in cases {
            let (server, worker) = pair();
            let (kept, passed) = pair();
            write_with(&server, &octets, &[passed.as_fd()]);
            drop(server);
            assert_eq!(incoming(&worker), Err(refused), "{octets:?}");
            drop(passed);
            assert_eq!(within(&kept, AMPLY), Ok(0), "{octets:?}");
        }
    }

    /// A socket that asks for its peer's credentials gets them beside
    /// every message it reads, in a control message of their own. They are
    /// not descriptors, and the worker takes none of them for a file.
    #[test]
    fn a_control_message_that_is_not_descriptors_is_passed_over() {
        let (server, worker) = pair();
        set_socket_passcred(&worker, true).unwrap();
        assert_eq!(send(&server, Job::Probe { hint: None }, &[]), Ok(()));
        assert_eq!(incoming(&worker), Ok(Some((Job::Probe { hint: None }, 0))));
    }

    #[test]
    fn a_socket_that_cannot_be_read_is_an_error() {
        let (server, worker) = pair();
        worker
            .set_read_timeout(Some(Duration::from_millis(20)))
            .unwrap();
        // Nothing arrives, then a length field and one octet of six.
        assert_eq!(
            incoming(&worker),
            Err(ChannelError::Io(io::ErrorKind::WouldBlock))
        );
        write(&server, &[6, 0, 0, 0, 1]);
        assert_eq!(
            incoming(&worker),
            Err(ChannelError::Io(io::ErrorKind::WouldBlock))
        );
    }

    #[test]
    fn an_answer_is_one_response_frame_and_a_refusal_one_refusal_frame() {
        let (server, worker) = pair();
        let mut out = &worker;
        let reading = Reading {
            level: 7,
            label: "ok".to_owned(),
        };
        assert_eq!(answer(&mut out, &Ok(reading)), Ok(()));
        for refusal in [Refusal::Request, Refusal::Unsupported, Refusal::Answer] {
            assert_eq!(answer::<Reading>(&mut out, &Err(refusal)), Ok(()));
        }
        drop(worker);
        let expected = [
            frame(RESPONSE, &[7, 2, b'o', b'k']),
            frame(REFUSAL, &[0]),
            frame(REFUSAL, &[1]),
            frame(REFUSAL, &[2]),
        ]
        .concat();
        assert_eq!(all_of(&server), expected);
    }

    #[test]
    fn the_server_reads_an_answer_back_checked() {
        let (server, worker) = pair();
        write(&worker, &frame(RESPONSE, &[7, 2, b'o', b'k']));
        write(&worker, &frame(RESPONSE, &[100, 0]));
        assert_eq!(
            reading(&server),
            Ok(Believed {
                level: 7,
                label: "ok".to_owned(),
            })
        );
        assert_eq!(
            reading(&server),
            Ok(Believed {
                level: 100,
                label: String::new(),
            })
        );
    }

    /// The answer is a well-formed `Reading`, and its level is one the
    /// server does not believe. The caller gets the reason and no part of
    /// the answer.
    ///
    /// Verifies: SEC-MED-023
    #[test]
    fn an_answer_that_decodes_and_fails_the_servers_check_is_refused() {
        let (server, worker) = pair();
        write(&worker, &frame(RESPONSE, &[101, 2, b'o', b'k']));
        assert_eq!(reading(&server), Err(AnswerError::Invalid(TooHigh(101))));
    }

    #[test]
    fn a_workers_refusal_is_reported_as_it_was_sent() {
        let (server, worker) = pair();
        for octet in [0, 1, 2] {
            write(&worker, &frame(REFUSAL, &[octet]));
        }
        assert_eq!(
            [reading(&server), reading(&server), reading(&server)],
            [
                Err(AnswerError::Refused(Refusal::Request)),
                Err(AnswerError::Refused(Refusal::Unsupported)),
                Err(AnswerError::Refused(Refusal::Answer)),
            ]
        );
    }

    /// The server reads with plain `read`, which takes octets and no
    /// control message, so the system discards a descriptor that a worker
    /// passes beside its answer, and the server never holds it.
    ///
    /// The descriptor passed is a copy of one end of a socket pair, and
    /// the test drops its own copy once that one is on its way
    /// (`within`). While the answer waits unread, the copy beside it keeps
    /// the socket open and the other end reads nothing. Once the server has
    /// read the answer, the other end reads the end of the stream: no copy
    /// is open in this process or on its way to it.
    #[test]
    fn a_descriptor_passed_beside_an_answer_never_reaches_the_server() {
        let (server, worker) = pair();
        let (kept, passed) = pair();
        write_with(
            &worker,
            &frame(RESPONSE, &[7, 2, b'o', b'k']),
            &[passed.as_fd()],
        );
        drop(passed);
        assert_eq!(within(&kept, BRIEFLY), Err(io::ErrorKind::WouldBlock));
        assert_eq!(
            reading(&server),
            Ok(Believed {
                level: 7,
                label: "ok".to_owned(),
            })
        );
        assert_eq!(within(&kept, AMPLY), Ok(0));
    }

    /// The worker's end stays open and sends no payload: the server
    /// refuses on the four octets of the length, without waiting for more
    /// and without a buffer for what was declared.
    ///
    /// Verifies: SEC-MED-023
    #[test]
    fn a_response_frame_over_32_mib_is_refused_from_its_length_alone() {
        let (server, worker) = pair();
        assert_eq!(MAX_RESPONSE, 33_554_432);
        // 33,554,433 octets declared.
        write(&worker, &[0x01, 0x00, 0x00, 0x02]);
        assert_eq!(
            reading(&server),
            Err(AnswerError::Channel(ChannelError::Wire(
                WireError::TooLarge {
                    len: 33_554_433,
                    max: 33_554_432,
                }
            )))
        );
    }

    /// A worker that dies closes its socket. The server is told so, before
    /// an answer or inside one, and is left neither waiting nor with half
    /// an answer.
    #[test]
    fn a_worker_that_is_gone_is_an_error_not_an_answer() {
        let (server, worker) = pair();
        drop(worker);
        assert_eq!(
            reading(&server),
            Err(AnswerError::Channel(ChannelError::Closed))
        );
        let whole = frame(RESPONSE, &[7, 2, b'o', b'k']);
        assert_eq!(whole.len(), 11);
        for cut in [2, 4, 7, 10] {
            let (server, worker) = pair();
            write(&worker, &whole[..cut]);
            drop(worker);
            assert_eq!(
                reading(&server),
                Err(AnswerError::Channel(ChannelError::Truncated)),
                "{cut}"
            );
        }
    }

    /// Verifies: SEC-MED-023
    #[test]
    fn a_frame_that_is_not_an_answer_is_refused() {
        let cases: [(&[u8], ChannelError); 7] = [
            (
                &[7, 0, 0, 0, 2, 0, RESPONSE, 7, 2, b'o', b'k'],
                ChannelError::Version(ProtocolVersion(2)),
            ),
            (
                &[3, 0, 0, 0, 1, 0, 0],
                ChannelError::Wire(WireError::UnknownKind { kind: 0 }),
            ),
            (
                &[2, 0, 0, 0, 1, 0],
                ChannelError::Wire(WireError::TooShort { len: 2 }),
            ),
            (
                &[0, 0, 0, 0],
                ChannelError::Wire(WireError::TooShort { len: 0 }),
            ),
            (
                &[6, 0, 0, 0, 1, 0, REQUEST, 0, 0, 0],
                ChannelError::Unexpected(FrameKind::Request),
            ),
            (
                &[3, 0, 0, 0, 1, 0, PART],
                ChannelError::Unexpected(FrameKind::Part),
            ),
            (
                &[3, 0, 0, 0, 1, 0, END],
                ChannelError::Unexpected(FrameKind::End),
            ),
        ];
        for (octets, refused) in cases {
            let (server, worker) = pair();
            write(&worker, octets);
            assert_eq!(reading(&server), Err(AnswerError::Channel(refused)));
        }
    }

    /// Verifies: SEC-MED-023
    #[test]
    fn a_payload_that_is_not_the_answer_asked_for_is_refused() {
        let cases: [(u8, &[u8], WireError); 4] = [
            // The level, and no label.
            (
                RESPONSE,
                &[7],
                WireError::Malformed {
                    offset: 1,
                    reason: PostcardError::DeserializeUnexpectedEnd,
                },
            ),
            // An octet left after the answer.
            (
                RESPONSE,
                &[7, 2, b'o', b'k', 9],
                WireError::Trailing { offset: 4, len: 1 },
            ),
            // A refusal that is none of the three.
            (
                REFUSAL,
                &[3],
                WireError::Malformed {
                    offset: 1,
                    reason: PostcardError::SerdeDeCustom,
                },
            ),
            // A refusal with an octet left over.
            (REFUSAL, &[1, 0], WireError::Trailing { offset: 1, len: 1 }),
        ];
        for (kind, payload, refused) in cases {
            let (server, worker) = pair();
            write(&worker, &frame(kind, payload));
            assert_eq!(
                reading(&server),
                Err(AnswerError::Channel(ChannelError::Wire(refused)))
            );
        }
    }

    /// The label's five octets start at the payload's third octet, and the
    /// limit on a string is lowered to four.
    ///
    /// Verifies: SEC-MED-023
    #[test]
    fn an_answer_is_decoded_under_the_limits_the_server_gives() {
        let (server, worker) = pair();
        let payload = [7, 5, b'h', b'e', b'l', b'l', b'o'];
        write(&worker, &frame(RESPONSE, &payload));
        write(&worker, &frame(RESPONSE, &payload));
        let limits = Limits::DEFAULT
            .with_override(LimitKind::LongText, 4)
            .unwrap();
        assert_eq!(
            read_answer::<Reading>(&server, &limits),
            Err(AnswerError::Channel(ChannelError::Wire(WireError::Fault(
                ParseFault::LimitExceeded {
                    limit: LimitKind::LongText,
                    value: 5,
                    max: 4,
                    offset: 2,
                }
            ))))
        );
        // The same answer under the default limits is read.
        assert_eq!(
            reading(&server),
            Ok(Believed {
                level: 7,
                label: "hello".to_owned(),
            })
        );
    }

    #[test]
    fn an_answer_that_cannot_be_written_is_answered_with_a_refusal() {
        let (server, worker) = pair();
        let mut out = &worker;
        assert_eq!(answer(&mut out, &Ok(Unwritable)), Ok(()));
        assert_eq!(reading(&server), Err(AnswerError::Refused(Refusal::Answer)));
    }

    #[test]
    fn an_answer_for_a_server_that_is_gone_is_an_error() {
        let (server, worker) = pair();
        drop(server);
        let mut out = &worker;
        assert_eq!(
            answer::<Reading>(&mut out, &Err(Refusal::Unsupported)),
            Err(ChannelError::Io(io::ErrorKind::BrokenPipe))
        );
    }

    #[test]
    fn an_answer_that_does_not_come_is_an_error() {
        let (server, _worker) = pair();
        server
            .set_read_timeout(Some(Duration::from_millis(20)))
            .unwrap();
        assert_eq!(
            reading(&server),
            Err(AnswerError::Channel(ChannelError::Io(
                io::ErrorKind::WouldBlock
            )))
        );
    }
}
