//! Opening one file beneath a root: regular files only, read-only, and by
//! the bytes of their names.

use std::ffi::OsStr;
use std::fs;
use std::io::ErrorKind;
use std::mem::MaybeUninit;
use std::os::unix::ffi::OsStrExt as _;
use std::path::PathBuf;

use gunmetal_fs::open::FileKind;
use gunmetal_fs::pool::Pool;
use gunmetal_fs::root::{FsError, LinkPolicy, Op};
use rustix::fs::OFlags;
use rustix::fs::inotify::{self, CreateFlags, ReadFlags, WatchFlags};
use rustix::io::{Errno, FdFlags};

use crate::support::{
    LONG, Scratch, assert_not_root, at, contents, fifo, identity, io, rel, set_mode, socket,
};

/// What opening `path` beneath `scratch`'s library gives, without the file.
fn opening(scratch: &Scratch, path: &str) -> Result<(), FsError> {
    scratch.root().open_file(&at(path)).map(|_| ())
}

#[test]
fn opens_a_regular_file_and_reads_any_part_of_it() {
    let scratch = Scratch::new("fs-open-read");
    scratch.dir("music/Artist/Album");
    scratch.file("music/Artist/Album/01 Track.flac", b"fLaC and the rest");
    let file = scratch
        .root()
        .open_file(&at("Artist/Album/01 Track.flac"))
        .expect("the file opens");
    assert_eq!(
        file.identity(),
        identity(&scratch.path("music/Artist/Album/01 Track.flac"))
    );
    assert_eq!(file.links(), 1);
    let mut four = [0; 4];
    assert_eq!(file.read_at(&mut four, 0), Ok(4));
    assert_eq!(&four, b"fLaC");
    assert_eq!(file.read_at(&mut four, 9), Ok(4));
    assert_eq!(&four, b"the ");
    // The file ends two bytes into this read, so the rest of the buffer
    // keeps what it held, and nothing lies beyond the end.
    assert_eq!(file.read_at(&mut four, 15), Ok(2));
    assert_eq!(&four, b"ste ");
    assert_eq!(file.read_at(&mut four, 17), Ok(0));
    assert_eq!(&four, b"ste ");
    let mut three = [0; 3];
    assert_eq!(file.read_exact_at(&mut three, 5), Ok(()));
    assert_eq!(&three, b"and");
    assert_eq!(
        file.read_exact_at(&mut three, 15),
        Err(io(Op::Read, ErrorKind::UnexpectedEof))
    );
}

/// Verifies: SEC-MED-035, SEC-MED-038, SEC-OPS-054
#[test]
fn opens_read_only_without_blocking_and_closed_on_exec() {
    let scratch = Scratch::new("fs-open-flags");
    scratch.file("music/track.flac", b"fLaC");
    let file = scratch
        .root()
        .open_file(&at("track.flac"))
        .expect("the file opens");
    let status = rustix::fs::fcntl_getfl(&file).expect("the status flags");
    assert_eq!(
        (
            status.intersection(OFlags::ACCMODE),
            status.contains(OFlags::NONBLOCK)
        ),
        (OFlags::RDONLY, true)
    );
    assert_eq!(rustix::io::fcntl_getfd(&file), Ok(FdFlags::CLOEXEC));
    // Whoever holds the descriptor, it cannot write.
    assert_eq!(rustix::io::write(&file, b"x"), Err(Errno::BADF));
    assert_eq!(contents(&file), b"fLaC");
    assert_eq!(
        fs::read(scratch.path("music/track.flac")).expect("read the file back"),
        b"fLaC"
    );
}

/// Opening a FIFO for reading waits for a writer unless the open asks not
/// to, and no writer ever comes here. The open runs on a pool so that a
/// door that did wait would fail this test instead of hanging it.
///
/// Verifies: SEC-MED-035
#[test]
fn refuses_a_fifo_named_like_a_track_without_waiting_for_a_writer() {
    let scratch = Scratch::new("fs-open-fifo");
    fifo(&scratch.path("music/track.flac"));
    let root = scratch.root();
    let opened = Pool::new(1).run(LONG, move || root.open_file(&at("track.flac")).map(|_| ()));
    assert_eq!(
        opened,
        Ok(Err(FsError::NotRegular {
            found: FileKind::Fifo
        }))
    );
}

/// Verifies: SEC-MED-035
#[test]
fn refuses_a_socket_named_like_a_cover() {
    let scratch = Scratch::new("fs-open-socket");
    socket(&scratch.path("music/cover.jpg"));
    assert_eq!(
        opening(&scratch, "cover.jpg"),
        Err(FsError::NotRegular {
            found: FileKind::Socket
        })
    );
}

/// Verifies: SEC-MED-035, SEC-API-018
#[test]
fn refuses_a_directory_and_the_root_itself() {
    let scratch = Scratch::new("fs-open-dir");
    scratch.dir("music/Album");
    let root = scratch.root();
    for path in [at("Album"), rel(&[])] {
        assert_eq!(
            root.open_file(&path).map(|_| ()),
            Err(FsError::NotRegular {
                found: FileKind::Dir
            })
        );
    }
}

/// A device is refused for what it is, here through a link into a folder
/// approved for the library.
///
/// Verifies: SEC-MED-035
#[test]
fn refuses_a_device_whatever_led_to_it() {
    let scratch = Scratch::new("fs-open-device");
    scratch.link("/dev/null", "music/tape.flac");
    let root = scratch.root_with(LinkPolicy {
        approved: vec![PathBuf::from("/dev")],
        others: Vec::new(),
    });
    assert_eq!(
        root.open_file(&at("tape.flac")).map(|_| ()),
        Err(FsError::NotRegular {
            found: FileKind::Other
        })
    );
}

/// What the door found an entry to be is enough to refuse it: a FIFO, a
/// socket or a directory is not opened at all, as a watch for opens shows.
/// The same watch does see a regular file opened.
///
/// Verifies: SEC-MED-035
#[test]
fn refuses_what_is_not_a_regular_file_before_opening_it() {
    let scratch = Scratch::new("fs-open-unopened");
    fifo(&scratch.path("music/track.flac"));
    socket(&scratch.path("music/cover.jpg"));
    scratch.dir("music/Album");
    scratch.file("music/real.flac", b"fLaC");
    let root = scratch.root();
    let watch = inotify::init(CreateFlags::NONBLOCK.union(CreateFlags::CLOEXEC))
        .expect("an inotify instance");
    inotify::add_watch(&watch, scratch.path("music"), WatchFlags::OPEN).expect("watch the library");
    for (path, found) in [
        ("track.flac", FileKind::Fifo),
        ("cover.jpg", FileKind::Socket),
        ("Album", FileKind::Dir),
    ] {
        assert_eq!(
            root.open_file(&at(path)).map(|_| ()),
            Err(FsError::NotRegular { found }),
            "{path}"
        );
    }
    drop(root.open_file(&at("real.flac")).expect("the file opens"));
    let mut buffer = [MaybeUninit::<u8>::uninit(); 1024];
    let mut events = inotify::Reader::new(&watch, &mut buffer);
    assert_eq!(
        events.next().map(|event| (
            event.events(),
            event.file_name().map(|name| name.to_bytes().to_vec())
        )),
        Ok((ReadFlags::OPEN, Some(b"real.flac".to_vec())))
    );
    assert_eq!(events.next().map(|event| event.events()), Err(Errno::AGAIN));
}

#[test]
fn reports_a_file_that_is_not_there() {
    let scratch = Scratch::new("fs-open-missing");
    scratch.file("music/track.flac", b"fLaC");
    for (path, kind) in [
        ("missing.flac", ErrorKind::NotFound),
        ("Album/missing.flac", ErrorKind::NotFound),
        ("track.flac/inside", ErrorKind::NotADirectory),
    ] {
        assert_eq!(
            opening(&scratch, path),
            Err(io(Op::Inspect, kind)),
            "{path}"
        );
    }
}

#[test]
fn reports_a_file_it_may_not_read() {
    assert_not_root();
    let scratch = Scratch::new("fs-open-denied");
    scratch.file("music/track.flac", b"fLaC");
    set_mode(&scratch.path("music/track.flac"), 0o000);
    assert_eq!(
        opening(&scratch, "track.flac"),
        Err(io(Op::Open, ErrorKind::PermissionDenied))
    );
}

/// Verifies: SEC-MED-040
#[test]
fn opens_names_that_are_not_utf8_or_hold_controls_by_their_bytes() {
    let scratch = Scratch::new("fs-open-bytes");
    let names: [&[u8]; 3] = [
        b"Bj\xF6rk.flac",
        b"two\nlines.flac",
        b"\x1B[31mred\x1B[0m.flac",
    ];
    let music = scratch.path("music");
    for name in names {
        fs::write(music.join(OsStr::from_bytes(name)), name).expect("write a file");
    }
    let root = scratch.root();
    for name in names {
        let file = root.open_file(&rel(&[name])).expect("the file opens");
        assert_eq!(contents(&file), name);
    }
}

/// A hard link cannot be told from the file it names, so it opens like any
/// other file. What the door can report is that the file has a second
/// name, which here is outside the library.
#[test]
fn reports_how_many_names_a_file_has() {
    let scratch = Scratch::new("fs-open-hard-link");
    scratch.dir("outside");
    scratch.file("outside/original.flac", b"fLaC");
    fs::hard_link(
        scratch.path("outside/original.flac"),
        scratch.path("music/linked.flac"),
    )
    .expect("make a hard link");
    let file = scratch
        .root()
        .open_file(&at("linked.flac"))
        .expect("the file opens");
    assert_eq!((file.links(), contents(&file)), (2, b"fLaC".to_vec()));
}
