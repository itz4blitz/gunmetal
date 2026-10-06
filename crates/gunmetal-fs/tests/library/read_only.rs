//! Nothing the door does changes a library: it needs no write permission,
//! and a watch on the library sees no change.

use std::fs::{self, File};
use std::io::Write as _;
use std::mem::MaybeUninit;
use std::os::fd::AsFd as _;
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};

use gunmetal_fs::fingerprint::{Mark, fingerprint};
use gunmetal_fs::walk::Visit;
use rustix::fs::inotify::{self, CreateFlags, ReadFlags, WatchFlags};
use rustix::io::Errno;

use crate::support::{
    Scratch, assert_not_root, at, collect, contents, identity, rel, set_mode, summary,
};

/// Every way a file or a directory can be changed, as inotify names them.
const CHANGES: WatchFlags = WatchFlags::ATTRIB
    .union(WatchFlags::CLOSE_WRITE)
    .union(WatchFlags::CREATE)
    .union(WatchFlags::DELETE)
    .union(WatchFlags::DELETE_SELF)
    .union(WatchFlags::MODIFY)
    .union(WatchFlags::MOVE_SELF)
    .union(WatchFlags::MOVED_FROM)
    .union(WatchFlags::MOVED_TO);

/// The name, mode, size and modification time of everything at and below
/// `top`, without following links.
fn snapshot(top: &Path) -> Vec<(PathBuf, u32, u64, i64, i64)> {
    let mut seen = Vec::new();
    let mut pending = vec![top.to_path_buf()];
    while let Some(next) = pending.pop() {
        let metadata = fs::symlink_metadata(&next).expect("metadata");
        if metadata.is_dir() {
            let entries = fs::read_dir(&next).expect("list the directory");
            pending.extend(entries.map(|entry| entry.expect("a directory entry").path()));
        }
        seen.push((
            next,
            metadata.mode(),
            metadata.size(),
            metadata.mtime(),
            metadata.mtime_nsec(),
        ));
    }
    seen.sort();
    seen
}

/// Verifies: SEC-MED-038, SEC-TM-042, SEC-OPS-054
#[test]
fn walks_and_reads_a_library_it_may_not_write_and_leaves_it_as_it_was() {
    assert_not_root();
    let scratch = Scratch::new("fs-read-only");
    scratch.dir("music/Album");
    scratch.file("music/Album/01.flac", b"fLaC one");
    scratch.file("music/cover.jpg", b"JFIF");
    scratch.link("Album/01.flac", "music/first.flac");
    for path in ["music/Album/01.flac", "music/cover.jpg"] {
        set_mode(&scratch.path(path), 0o444);
    }
    for path in ["music/Album", "music"] {
        set_mode(&scratch.path(path), 0o555);
    }
    let before = snapshot(&scratch.path("music"));
    let watch = inotify::init(CreateFlags::NONBLOCK.union(CreateFlags::CLOEXEC))
        .expect("an inotify instance");
    for path in ["music", "music/Album"] {
        inotify::add_watch(&watch, scratch.path(path), CHANGES).expect("watch the directory");
    }

    let root = scratch.root();
    let track = identity(&scratch.path("music/Album/01.flac"));
    let cover = identity(&scratch.path("music/cover.jpg"));
    assert_eq!(
        collect(root.walk()),
        [
            Visit::Dir {
                path: rel(&[]),
                summary: summary(&[
                    (b"Album".as_slice(), Mark::Dir),
                    (b"cover.jpg".as_slice(), Mark::File(cover)),
                    (b"first.flac".as_slice(), Mark::File(track)),
                ]),
            },
            Visit::File {
                path: at("cover.jpg"),
                identity: cover,
                links: 1,
            },
            Visit::File {
                path: at("first.flac"),
                identity: track,
                links: 1,
            },
            Visit::Dir {
                path: at("Album"),
                summary: summary(&[(b"01.flac".as_slice(), Mark::File(track))]),
            },
            Visit::File {
                path: at("Album/01.flac"),
                identity: track,
                links: 1,
            },
        ]
    );
    for (path, bytes) in [
        ("cover.jpg", b"JFIF".as_slice()),
        ("first.flac", b"fLaC one".as_slice()),
        ("Album/01.flac", b"fLaC one".as_slice()),
    ] {
        let file = root.open_file(&at(path)).expect("the file opens");
        assert_eq!(contents(&file), bytes, "{path}");
        assert_eq!(
            fingerprint(&file).map(|print| print.identity),
            Ok(identity(&scratch.path(&format!("music/{path}")))),
            "{path}"
        );
    }

    let mut buffer = [MaybeUninit::<u8>::uninit(); 1024];
    let mut events = inotify::Reader::new(&watch, &mut buffer);
    assert_eq!(
        events.next().map(|event| event.events()),
        Err(Errno::AGAIN),
        "the library saw a change"
    );
    assert_eq!(snapshot(&scratch.path("music")), before);

    // The watch does report a change when there is one, so its silence
    // above means something. This also lets the scratch directory go.
    set_mode(&scratch.path("music/Album"), 0o755);
    assert_eq!(
        events
            .next()
            .map(|event| event.events().contains(ReadFlags::ATTRIB)),
        Ok(true)
    );
    set_mode(&scratch.path("music"), 0o755);
}

/// A `MediaFile` lends its descriptor, so that it can be handed to a
/// worker, and a borrowed descriptor can be cloned into one that is owned.
/// That one is still open for reading only: it cannot write the file or
/// change its length.
///
/// Verifies: SEC-MED-038, SEC-OPS-054
#[test]
fn a_descriptor_cloned_from_an_open_file_still_cannot_write() {
    let scratch = Scratch::new("fs-read-only-clone");
    scratch.file("music/track.flac", b"fLaC");
    let file = scratch
        .root()
        .open_file(&at("track.flac"))
        .expect("the file opens");
    let mut owned = File::from(
        file.as_fd()
            .try_clone_to_owned()
            .expect("clone the descriptor"),
    );
    assert_eq!(
        owned.write_all(b"x").map_err(|error| error.raw_os_error()),
        Err(Some(Errno::BADF.raw_os_error()))
    );
    assert_eq!(
        owned.set_len(0).map_err(|error| error.raw_os_error()),
        Err(Some(Errno::INVAL.raw_os_error()))
    );
    assert_eq!(
        fs::read(scratch.path("music/track.flac")).expect("read the file back"),
        b"fLaC"
    );
}
