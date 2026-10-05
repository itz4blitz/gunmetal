//! The user log's part of the data root on a real filesystem: the typed
//! paths of a stream's directory and of a segment, the bounded listings and
//! the removal of one stream.
//!
//! Each test plants what it lists or removes by path, under names written
//! out here, so nothing it sets up goes through the code it checks.
#![expect(
    clippy::disallowed_methods,
    reason = "tests of the filesystem door build and inspect hostile layouts on the real filesystem, by path (SEC-MED-033)"
)]

#[allow(dead_code, reason = "each test binary uses part of the helpers")]
mod support;

use std::ffi::OsString;
use std::fs;
use std::io::{ErrorKind, Write};
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::symlink;
use std::path::PathBuf;

use gunmetal_fs::dataroot::{
    DataRoot, DataRootError, Item, Kind, LIST_MAX, Listing, LogDirError, Op,
};
use gunmetal_fs::path::{DataDir, DataPath, LogMonth, LogStream, USER_LOG};
use proptest::prelude::*;
use support::{TempDir, mode, names, open};

const ALICE: LogStream = LogStream::Profile([0xA1; 16]);
const BOB: LogStream = LogStream::Profile([0xB2; 16]);
const CAROL: LogStream = LogStream::Profile([0xC3; 16]);

/// The names of their directories, written out.
const ALICE_DIR: &str = "p-a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1";
const BOB_DIR: &str = "p-b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2";
const CAROL_DIR: &str = "p-c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3";

fn month(year: u16, number: u8) -> LogMonth {
    LogMonth::new(year, number).expect("a month of the calendar")
}

/// A data root whose `durable/log` exists and is empty.
fn log_root(dir: &TempDir) -> DataRoot {
    let root = open(dir).root;
    root.create_dir(&USER_LOG)
        .expect("create the user log's directory");
    root
}

/// Where `durable/log` is on disk.
fn log_path(dir: &TempDir) -> PathBuf {
    dir.join("data/durable/log")
}

/// Plants a directory called `name` in the user log, and in it a file
/// holding `bytes` for each of `files`.
fn plant(dir: &TempDir, name: &str, files: &[(&str, &[u8])]) -> PathBuf {
    let stream = log_path(dir).join(name);
    fs::create_dir(&stream).expect("create the stream's directory");
    for (file, bytes) in files {
        fs::write(stream.join(file), bytes).expect("write the segment");
    }
    stream
}

/// A listing that found `entries` and the names `foreign`.
fn listed<T>(entries: Vec<T>, foreign: &[&str]) -> Listing<T> {
    Listing {
        entries,
        foreign: foreign.iter().copied().map(OsString::from).collect(),
    }
}

fn io(item: Item, op: Op, kind: ErrorKind) -> LogDirError {
    LogDirError::Root(DataRootError::Io { item, op, kind })
}

fn wrong_kind(item: Item, found: Kind) -> LogDirError {
    LogDirError::Root(DataRootError::WrongKind { item, found })
}

/// The bytes as lower-case hexadecimal digits, two for each byte, written
/// with division where the path code formats one number.
fn hex(bytes: [u8; 16]) -> String {
    let mut text = String::new();
    for byte in bytes {
        for nibble in [byte / 16, byte % 16] {
            text.push(
                char::from_digit(u32::from(nibble), 16).expect("a nibble is a hexadecimal digit"),
            );
        }
    }
    text
}

/// Whether a stream's directory can be called `name`: an independent
/// statement of the two forms.
fn is_stream_name(name: &str) -> bool {
    name == "household"
        || name.strip_prefix("p-").is_some_and(|digits| {
            digits.len() == 32
                && digits
                    .bytes()
                    .all(|digit| matches!(digit, b'0'..=b'9' | b'a'..=b'f'))
        })
}

/// Whether a segment can be called `name`: four digits, a dash, a month
/// from `01` to `12` and `.seg`, read octet by octet.
fn is_segment_name(name: &str) -> bool {
    let octets = name.as_bytes();
    octets.len() == 11
        && octets[..4].iter().all(u8::is_ascii_digit)
        && octets[4] == b'-'
        && octets[5..7].iter().all(u8::is_ascii_digit)
        && &octets[7..] == b".seg"
        && (1..=12).contains(&(u32::from(octets[5] - b'0') * 10 + u32::from(octets[6] - b'0')))
}

/// Verifies: SEC-HIS-015
#[test]
fn names_the_user_log_and_a_stream_s_directory() {
    assert_eq!(USER_LOG, DataPath::constant(DataDir::Durable, "log"));
    assert_eq!(
        DataPath::log_stream(LogStream::Household),
        DataPath::constant(DataDir::Durable, "log/household")
    );
    let id = [
        0x00, 0x01, 0x0f, 0x10, 0x7f, 0x80, 0x9a, 0xbc, 0xde, 0xf0, 0xff, 0x12, 0x34, 0x56, 0x78,
        0x90,
    ];
    assert_eq!(
        DataPath::log_stream(LogStream::Profile(id)),
        DataPath::constant(DataDir::Durable, "log/p-00010f107f809abcdef0ff1234567890")
    );
    assert_eq!(
        DataPath::log_stream(ALICE),
        DataPath::constant(DataDir::Durable, "log/p-a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1")
    );
}

/// Verifies: SEC-HIS-015
#[test]
fn names_a_segment_by_its_year_and_month() {
    assert_eq!(
        DataPath::log_segment(ALICE, month(2026, 10)),
        DataPath::constant(
            DataDir::Durable,
            "log/p-a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1/2026-10.seg"
        )
    );
    assert_eq!(
        DataPath::log_segment(LogStream::Household, month(7, 1)),
        DataPath::constant(DataDir::Durable, "log/household/0007-01.seg")
    );
    assert_eq!(
        DataPath::log_segment(BOB, month(9999, 12)),
        DataPath::constant(
            DataDir::Durable,
            "log/p-b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2/9999-12.seg"
        )
    );
}

#[test]
fn a_month_is_a_year_to_9999_and_a_month_of_the_calendar() {
    let parts =
        |year, number| LogMonth::new(year, number).map(|found| (found.year(), found.month()));
    assert_eq!(parts(2026, 10), Some((2026, 10)));
    assert_eq!(parts(0, 1), Some((0, 1)));
    assert_eq!(parts(9999, 12), Some((9999, 12)));
    assert_eq!(parts(10_000, 12), None);
    assert_eq!(parts(2026, 0), None);
    assert_eq!(parts(2026, 13), None);
    assert_eq!(parts(u16::MAX, u8::MAX), None);
    assert_eq!(LogMonth::MAX_YEAR, 9999);
    assert_eq!(Some(LogMonth::MIN), LogMonth::new(0, 1));
    let mut months = vec![
        month(2026, 2),
        month(2025, 12),
        month(2026, 10),
        month(2026, 1),
    ];
    months.sort();
    assert_eq!(
        months,
        [
            month(2025, 12),
            month(2026, 1),
            month(2026, 2),
            month(2026, 10)
        ]
    );
}

/// Verifies: SEC-HIS-015
#[test]
fn creates_a_stream_s_directory_and_segments_by_their_typed_paths() {
    let dir = TempDir::new();
    let root = log_root(&dir);
    for (stream, segment, bytes) in [
        (ALICE, month(2026, 10), b"october".as_slice()),
        (
            LogStream::Household,
            month(2026, 9),
            b"september".as_slice(),
        ),
    ] {
        root.create_dir(&DataPath::log_stream(stream))
            .expect("create the stream's directory");
        root.create_new(&DataPath::log_segment(stream, segment))
            .expect("create the segment")
            .write_all(bytes)
            .expect("write the segment");
    }
    let log = log_path(&dir);
    assert_eq!(names(&log), ["household", ALICE_DIR]);
    assert_eq!(names(&log.join(ALICE_DIR)), ["2026-10.seg"]);
    assert_eq!(names(&log.join("household")), ["2026-09.seg"]);
    assert_eq!(
        fs::read(log.join(ALICE_DIR).join("2026-10.seg")).expect("read the segment"),
        b"october"
    );
    assert_eq!(mode(&log.join(ALICE_DIR)), 0o700);
    assert_eq!(mode(&log.join(ALICE_DIR).join("2026-10.seg")), 0o600);
}

#[test]
fn lists_the_streams_that_have_a_directory_in_name_order() {
    let dir = TempDir::new();
    let root = log_root(&dir);
    assert_eq!(root.log_streams(), Ok(listed(vec![], &[])));
    for name in [BOB_DIR, "household", ALICE_DIR] {
        plant(&dir, name, &[]);
    }
    assert_eq!(
        root.log_streams(),
        Ok(listed(vec![LogStream::Household, ALICE, BOB], &[]))
    );
}

/// Verifies: SEC-HIS-015
#[test]
fn lists_only_a_directory_whose_name_a_stream_has() {
    let dir = TempDir::new();
    let root = log_root(&dir);
    plant(&dir, ALICE_DIR, &[]);
    let log = log_path(&dir);
    let mut foreign = vec![
        // Upper case, in the digits and in the prefix.
        OsString::from(format!("p-{}", "A1".repeat(16))),
        OsString::from(format!("P-{}", "a1".repeat(16))),
        // Too few digits, too many, and none.
        OsString::from(format!("p-{}", "a1".repeat(15))),
        OsString::from(format!("p-{}a", "a1".repeat(16))),
        OsString::from("p-"),
        // Not hexadecimal, and a sign, which a number may carry.
        OsString::from(format!("p-g{}", "1".repeat(31))),
        OsString::from(format!("p-+{}", "1".repeat(31))),
        // Near the household's name.
        OsString::from("households"),
        OsString::from("Household"),
        OsString::from(".household"),
        // Not text.
        OsString::from_vec(vec![b'p', b'-', 0xFF]),
    ];
    for name in &foreign {
        fs::create_dir(log.join(name)).expect("create the directory");
    }
    // A file where a stream's directory belongs is not a stream either.
    fs::write(log.join(BOB_DIR), b"").expect("write a file");
    foreign.push(OsString::from(BOB_DIR));
    foreign.sort();
    assert_eq!(
        root.log_streams(),
        Ok(Listing {
            entries: vec![ALICE],
            foreign
        })
    );
}

#[test]
fn lists_segments_oldest_first_and_everything_else_as_foreign() {
    let dir = TempDir::new();
    let root = log_root(&dir);
    let alice = plant(
        &dir,
        ALICE_DIR,
        &[
            ("2026-10.seg", b""),
            ("2025-12.seg", b""),
            ("2026-01.seg", b""),
        ],
    );
    let mut foreign: Vec<OsString> = [
        // What an interrupted replace leaves beside a segment.
        ".2026-10.seg.tmp",
        // No suffix, and no dash.
        "2026-10",
        "202610.seg",
        // A year and a month that are not numbers, or too large for one.
        "year-10.seg",
        "2026-mm.seg",
        "99999-10.seg",
        "2026-999.seg",
        // A year after 9999 and months outside the calendar.
        "10000-10.seg",
        "2026-00.seg",
        "2026-13.seg",
        // Other spellings of a month that has a name.
        "2026-1.seg",
        "02026-10.seg",
        "+026-10.seg",
        "2026-+1.seg",
        "2026-10.SEG",
        "2026-10.seg.bak",
        "notes.txt",
    ]
    .into_iter()
    .map(OsString::from)
    .collect();
    // Not text.
    foreign.push(OsString::from_vec(vec![b'2', 0xFF]));
    for name in &foreign {
        fs::write(alice.join(name), b"").expect("write the file");
    }
    // A directory named like a segment is not a segment.
    fs::create_dir(alice.join("2024-05.seg")).expect("create the directory");
    foreign.push(OsString::from("2024-05.seg"));
    foreign.sort();
    assert_eq!(
        root.log_segments(ALICE),
        Ok(Listing {
            entries: vec![month(2025, 12), month(2026, 1), month(2026, 10)],
            foreign
        })
    );
}

/// Verifies: SEC-HIS-016, SEC-TM-043
#[test]
fn never_lists_a_symbolic_link_as_a_stream_or_a_segment() {
    let dir = TempDir::new();
    let root = log_root(&dir);
    let alice = plant(&dir, ALICE_DIR, &[("2026-10.seg", b"alice")]);
    plant(&dir, BOB_DIR, &[("2026-10.seg", b"bob")]);
    fs::write(dir.join("outside"), b"outside").expect("write a file outside the root");
    // In Alice's directory, each named like a segment: a link to Bob's
    // segment, and a link that leaves the data directory.
    symlink(
        format!("../{BOB_DIR}/2026-10.seg"),
        alice.join("2026-11.seg"),
    )
    .expect("plant a link to another stream's segment");
    symlink(dir.join("outside"), alice.join("2026-12.seg")).expect("plant a link out of the root");
    // In the log, named like Carol's directory: a link to Alice's.
    symlink(ALICE_DIR, log_path(&dir).join(CAROL_DIR)).expect("plant a link to another stream");
    assert_eq!(
        root.log_segments(ALICE),
        Ok(listed(
            vec![month(2026, 10)],
            &["2026-11.seg", "2026-12.seg"]
        ))
    );
    assert_eq!(
        root.log_streams(),
        Ok(listed(vec![ALICE, BOB], &[CAROL_DIR]))
    );
    assert_eq!(
        root.log_segments(CAROL),
        Err(wrong_kind(
            Item::Path(DataPath::log_stream(CAROL)),
            Kind::Symlink
        ))
    );
}

#[test]
fn reports_a_log_or_a_stream_that_has_no_directory() {
    let dir = TempDir::new();
    let root = open(&dir).root;
    let no_log = io(Item::Path(USER_LOG), Op::Inspect, ErrorKind::NotFound);
    assert_eq!(root.log_streams(), Err(no_log.clone()));
    assert_eq!(root.log_segments(ALICE), Err(no_log.clone()));
    assert_eq!(root.remove_log_stream(ALICE), Err(no_log));
    root.create_dir(&USER_LOG)
        .expect("create the user log's directory");
    assert_eq!(
        root.log_segments(ALICE),
        Err(io(
            Item::Path(DataPath::log_stream(ALICE)),
            Op::Inspect,
            ErrorKind::NotFound
        ))
    );
}

/// Verifies: SEC-HIS-016, SEC-TM-043
#[test]
fn refuses_a_symbolic_link_where_the_user_log_belongs() {
    let dir = TempDir::new();
    let root = open(&dir).root;
    let elsewhere = dir.join("elsewhere");
    fs::create_dir(&elsewhere).expect("create the target");
    fs::create_dir(elsewhere.join(ALICE_DIR)).expect("create a directory there");
    fs::write(elsewhere.join(ALICE_DIR).join("2026-10.seg"), b"kept").expect("write a file");
    symlink(&elsewhere, log_path(&dir)).expect("plant the link");
    let refused = wrong_kind(Item::Path(USER_LOG), Kind::Symlink);
    assert_eq!(root.log_streams(), Err(refused.clone()));
    assert_eq!(root.log_segments(ALICE), Err(refused.clone()));
    assert_eq!(root.remove_log_stream(ALICE), Err(refused));
    assert_eq!(names(&elsewhere.join(ALICE_DIR)), ["2026-10.seg"]);
}

#[test]
fn refuses_a_file_where_a_stream_s_directory_belongs() {
    let dir = TempDir::new();
    let root = log_root(&dir);
    let log = log_path(&dir);
    fs::write(log.join(ALICE_DIR), b"not a directory").expect("write a file");
    let refused = wrong_kind(Item::Path(DataPath::log_stream(ALICE)), Kind::File);
    assert_eq!(root.log_segments(ALICE), Err(refused.clone()));
    assert_eq!(root.remove_log_stream(ALICE), Err(refused));
    assert_eq!(
        fs::read(log.join(ALICE_DIR)).expect("read the file"),
        b"not a directory"
    );
}

#[test]
fn lists_a_log_of_the_most_streams_and_refuses_one_more() {
    assert_eq!(LIST_MAX, 4_096);
    let dir = TempDir::new();
    let root = log_root(&dir);
    let ids = || (0..4_096_u128).map(u128::to_be_bytes);
    for id in ids() {
        plant(&dir, &format!("p-{}", hex(id)), &[]);
    }
    assert_eq!(
        root.log_streams(),
        Ok(listed(ids().map(LogStream::Profile).collect(), &[]))
    );
    plant(&dir, "one-more", &[]);
    assert_eq!(
        root.log_streams(),
        Err(LogDirError::TooMany {
            item: Item::Path(USER_LOG),
            max: 4_096
        })
    );
}

#[test]
fn lists_a_stream_of_the_most_segments_and_refuses_one_more() {
    let dir = TempDir::new();
    let root = log_root(&dir);
    let alice = plant(&dir, ALICE_DIR, &[]);
    // The 4,096 months from January of year 0: 341 years and four months.
    let months = || {
        (0..4_096_u16).map(|n| {
            (
                n / 12,
                u8::try_from(n % 12).expect("a remainder under twelve") + 1,
            )
        })
    };
    for (year, number) in months() {
        fs::write(alice.join(format!("{year:04}-{number:02}.seg")), b"")
            .expect("write the segment");
    }
    assert_eq!(
        root.log_segments(ALICE),
        Ok(listed(
            months().map(|(year, number)| month(year, number)).collect(),
            &[]
        ))
    );
    fs::write(alice.join("one-more"), b"").expect("write the file");
    let too_many = LogDirError::TooMany {
        item: Item::Path(DataPath::log_stream(ALICE)),
        max: 4_096,
    };
    assert_eq!(root.log_segments(ALICE), Err(too_many.clone()));
    // Nothing is removed from a directory that was not read to its end.
    assert_eq!(root.remove_log_stream(ALICE), Err(too_many));
    assert_eq!(names(&alice).len(), 4_097);
    fs::remove_file(alice.join("one-more")).expect("remove the file");
    assert_eq!(root.remove_log_stream(ALICE), Ok(()));
    assert_eq!(names(&log_path(&dir)), [] as [&str; 0]);
}

#[test]
fn removes_a_stream_s_directory_with_its_segments_and_what_a_replace_left() {
    let dir = TempDir::new();
    let root = log_root(&dir);
    plant(
        &dir,
        ALICE_DIR,
        &[
            ("2026-09.seg", b"september"),
            ("2026-10.seg", b"october"),
            (".2026-10.seg.tmp", b"half a rewrite"),
        ],
    );
    let bob = plant(&dir, BOB_DIR, &[("2026-10.seg", b"bob")]);
    let log = log_path(&dir);
    assert_eq!(root.remove_log_stream(ALICE), Ok(()));
    assert_eq!(names(&log), [BOB_DIR]);
    assert_eq!(names(&bob), ["2026-10.seg"]);
    assert_eq!(
        fs::read(bob.join("2026-10.seg")).expect("read the segment"),
        b"bob"
    );
    assert_eq!(root.log_streams(), Ok(listed(vec![BOB], &[])));
    // A stream with no directory is already removed.
    assert_eq!(root.remove_log_stream(ALICE), Ok(()));
    assert_eq!(names(&log), [BOB_DIR]);
}

/// Verifies: SEC-HIS-016, SEC-TM-043
#[test]
fn removes_a_planted_link_itself_and_never_what_it_leads_to() {
    let dir = TempDir::new();
    let root = log_root(&dir);
    let alice = plant(&dir, ALICE_DIR, &[("2026-10.seg", b"alice")]);
    let bob = plant(&dir, BOB_DIR, &[("2026-10.seg", b"bob")]);
    fs::write(dir.join("outside"), b"outside").expect("write a file outside the root");
    // Each named like a segment: a link to Bob's segment, a link to Bob's
    // directory, and a link that leaves the data directory.
    symlink(
        format!("../{BOB_DIR}/2026-10.seg"),
        alice.join("2026-11.seg"),
    )
    .expect("plant a link to another stream's segment");
    symlink(format!("../{BOB_DIR}"), alice.join("2026-12.seg"))
        .expect("plant a link to another stream's directory");
    symlink(dir.join("outside"), alice.join("2027-01.seg")).expect("plant a link out of the root");
    assert_eq!(root.remove_log_stream(ALICE), Ok(()));
    assert_eq!(names(&log_path(&dir)), [BOB_DIR]);
    assert_eq!(names(&bob), ["2026-10.seg"]);
    assert_eq!(
        fs::read(bob.join("2026-10.seg")).expect("read the segment"),
        b"bob"
    );
    assert_eq!(
        fs::read(dir.join("outside")).expect("read the file"),
        b"outside"
    );
}

/// Verifies: SEC-HIS-016, SEC-TM-043
#[test]
fn refuses_to_remove_through_a_link_at_a_stream_s_name() {
    let dir = TempDir::new();
    let root = log_root(&dir);
    let alice = plant(&dir, ALICE_DIR, &[("2026-10.seg", b"alice")]);
    let log = log_path(&dir);
    symlink(ALICE_DIR, log.join(CAROL_DIR)).expect("plant a link to another stream");
    assert_eq!(
        root.remove_log_stream(CAROL),
        Err(wrong_kind(
            Item::Path(DataPath::log_stream(CAROL)),
            Kind::Symlink
        ))
    );
    assert_eq!(names(&log), [ALICE_DIR, CAROL_DIR]);
    assert_eq!(names(&alice), ["2026-10.seg"]);
}

#[test]
fn leaves_a_stream_s_directory_that_holds_anything_else() {
    let dir = TempDir::new();
    let root = log_root(&dir);
    // Beside two segments, names no segment and no replace of one has.
    let alice = plant(
        &dir,
        ALICE_DIR,
        &[
            ("2026-09.seg", b"september"),
            ("2026-10.seg", b"october"),
            ("notes.txt", b"kept"),
            (".notes.txt.tmp", b"kept"),
            (".2026-13.seg.tmp", b"kept"),
            ("2026-10.tmp", b"kept"),
        ],
    );
    assert_eq!(
        root.remove_log_stream(ALICE),
        Err(io(
            Item::Path(DataPath::log_stream(ALICE)),
            Op::Remove,
            ErrorKind::DirectoryNotEmpty
        ))
    );
    assert_eq!(
        names(&alice),
        [
            ".2026-13.seg.tmp",
            ".notes.txt.tmp",
            "2026-10.tmp",
            "notes.txt"
        ]
    );
}

#[test]
fn leaves_a_link_and_a_directory_whose_names_no_segment_has() {
    let dir = TempDir::new();
    let root = log_root(&dir);
    let alice = plant(&dir, ALICE_DIR, &[("2026-10.seg", b"october")]);
    let bob = plant(&dir, BOB_DIR, &[("2026-10.seg", b"bob")]);
    symlink(format!("../{BOB_DIR}"), alice.join("link")).expect("plant a link to another stream");
    fs::create_dir(alice.join("sub")).expect("create the directory");
    fs::write(alice.join("sub").join("2026-11.seg"), b"kept").expect("write a file");
    assert_eq!(
        root.remove_log_stream(ALICE),
        Err(io(
            Item::Path(DataPath::log_stream(ALICE)),
            Op::Remove,
            ErrorKind::DirectoryNotEmpty
        ))
    );
    // The segment is gone; the link, the directory and what they hold stay,
    // and a listing reports both by name.
    assert_eq!(names(&alice), ["link", "sub"]);
    assert_eq!(names(&alice.join("sub")), ["2026-11.seg"]);
    assert_eq!(names(&bob), ["2026-10.seg"]);
    assert_eq!(
        root.log_segments(ALICE),
        Ok(listed(vec![], &["link", "sub"]))
    );
}

#[test]
fn stops_at_a_directory_named_like_a_segment() {
    let dir = TempDir::new();
    let root = log_root(&dir);
    let alice = plant(
        &dir,
        ALICE_DIR,
        &[("2026-09.seg", b"september"), ("2026-11.seg", b"november")],
    );
    fs::create_dir(alice.join("2026-10.seg")).expect("create the directory");
    assert_eq!(
        root.remove_log_stream(ALICE),
        Err(io(
            Item::Path(DataPath::log_stream(ALICE)),
            Op::Remove,
            ErrorKind::IsADirectory
        ))
    );
    assert_eq!(names(&alice), ["2026-10.seg", "2026-11.seg"]);
}

/// A name a directory in the user log might have: a stream's, a near miss,
/// or any other.
fn stream_name() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("household".to_owned()),
        any::<[u8; 16]>().prop_map(|id| format!("p-{}", hex(id))),
        "p-[0-9a-f]{30,34}",
        "p-[0-9a-fA-Fg+]{32}",
        "[a-z0-9][a-z0-9._-]{0,11}",
    ]
}

/// A name a file in a stream's directory might have: a segment's, a near
/// miss, or any other.
fn segment_name() -> impl Strategy<Value = String> {
    prop_oneof![
        (0..=9999_u16, 1..=12_u8).prop_map(|(year, number)| format!("{year:04}-{number:02}.seg")),
        "[0-9]{3,5}-[0-9]{1,3}\\.seg",
        "[0-9+]{4}-[0-9+]{2}\\.(seg|SEG|tmp)",
        "[a-z0-9][a-z0-9._-]{0,11}",
    ]
}

proptest! {
    /// Verifies: SEC-HIS-015
    #[test]
    fn builds_every_name_from_the_typed_values_alone(
        id in any::<[u8; 16]>(),
        year in 0..=9999_u16,
        number in 1..=12_u8,
    ) {
        let stream = LogStream::Profile(id);
        let directory = format!("log/p-{}", hex(id));
        let built = DataPath::log_stream(stream);
        prop_assert_eq!(built.dir(), DataDir::Durable);
        prop_assert_eq!(built.rel(), directory.as_str());
        let segment = format!("{directory}/{year:04}-{number:02}.seg");
        let built = DataPath::log_segment(stream, month(year, number));
        prop_assert_eq!(built.dir(), DataDir::Durable);
        prop_assert_eq!(built.rel(), segment.as_str());
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    /// Verifies: SEC-HIS-015
    #[test]
    fn lists_exactly_the_directories_a_stream_can_have(
        created in proptest::collection::btree_set(stream_name(), 0..8),
    ) {
        let dir = TempDir::new();
        let root = log_root(&dir);
        for name in &created {
            plant(&dir, name, &[]);
        }
        let found = root.log_streams().expect("the user log lists");
        let streams: Vec<String> = found
            .entries
            .iter()
            .map(|stream| DataPath::log_stream(*stream).rel().to_owned())
            .collect();
        let expected: Vec<String> = created
            .iter()
            .filter(|name| is_stream_name(name))
            .map(|name| format!("log/{name}"))
            .collect();
        prop_assert_eq!(streams, expected);
        let foreign: Vec<OsString> = created
            .iter()
            .filter(|name| !is_stream_name(name))
            .map(OsString::from)
            .collect();
        prop_assert_eq!(found.foreign, foreign);
    }

    /// Verifies: SEC-HIS-015
    #[test]
    fn lists_exactly_the_files_a_segment_can_be(
        created in proptest::collection::btree_set(segment_name(), 0..8),
    ) {
        let dir = TempDir::new();
        let root = log_root(&dir);
        let alice = plant(&dir, ALICE_DIR, &[]);
        for name in &created {
            fs::write(alice.join(name), b"").expect("write the file");
        }
        let found = root.log_segments(ALICE).expect("the stream lists");
        let segments: Vec<String> = found
            .entries
            .iter()
            .map(|month| DataPath::log_segment(ALICE, *month).rel().to_owned())
            .collect();
        let expected: Vec<String> = created
            .iter()
            .filter(|name| is_segment_name(name))
            .map(|name| format!("log/{ALICE_DIR}/{name}"))
            .collect();
        prop_assert_eq!(segments, expected);
        let foreign: Vec<OsString> = created
            .iter()
            .filter(|name| !is_segment_name(name))
            .map(OsString::from)
            .collect();
        prop_assert_eq!(found.foreign, foreign);
    }
}
