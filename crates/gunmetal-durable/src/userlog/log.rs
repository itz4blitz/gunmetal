//! The user log service: the one writer of a data directory's segments,
//! and their readers (ADR 3, sections 6 to 8).
//!
//! **One writer.** A [`UserLog`] holds everything it knows about the
//! segments behind one lock, so appends, erasures and reads take their turn
//! and nothing else opens a segment for writing. The composition root opens
//! one for the data directory and passes the same data root to every call.
//! Every call blocks until its files are on disk; async callers run it on
//! the blocking pool.
//!
//! **Writing.** [`UserLog::append`] takes a batch. It checks each event
//! against the erasure ledger first, then against the IDs its stream
//! already holds, stamps what is new with the stream's next sequence
//! number and writes it to the stream's newest segment, starting a segment
//! when the server's month has moved on. It then syncs each segment it
//! wrote once and only after that acknowledges the batch. A new segment is
//! created exclusively with its header, and it and its directory are synced
//! before the batch is acknowledged.
//!
//! **Reading.** [`UserLog::read`] takes a [`Permit`] and serves only the
//! stream of the profile that permit was decided for (SEC-TM-024,
//! SEC-API-010); it takes no stream at all. The one reader without a
//! permit is [`UserLog::replay_into`], which hands records only to a
//! [`ProjectionBuilder`] and returns none; only the startup module calls
//! it.
//!
//! **Erasure.** [`UserLog::erase`] is the one operation that removes
//! acknowledged records. It appends the selector to the ledger and syncs
//! it, then rewrites each segment that holds a record the selector covers,
//! or removes a whole profile's directory. Opening the log does the same
//! for every selector in the ledger, so an erasure a crash interrupted is
//! finished before anything is served.
//!
//! **After a failure.** An operation that fails part of the way through
//! may have left the files and the log's memory of them apart. The log then
//! refuses every call with [`LogError::Halted`] until it is opened again,
//! which reads the files afresh.
//!
//! An `fsync` cannot be observed on a real file from a test, and neither
//! can a power loss; the tests show what truncation and a failed step leave
//! behind, and the order of the syncs is this module's code to review.

use std::ops::Range;

use gunmetal_core::authz::Permit;
use gunmetal_core::id::PublicId;
use gunmetal_core::logframe::EncodeError;
use gunmetal_core::time::Timestamp;
use gunmetal_core::userdata::erasure::Selector;
use gunmetal_core::userdata::event::{Event, EventId, ProfileId, Stream};
use gunmetal_fs::dataroot::DataRoot;
use gunmetal_fs::path::LogMonth;

use crate::userlog::error::LogError;
use crate::userlog::record::Stamped;
use crate::userlog::scan::Damage;

/// What became of one event of a batch the log accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appended {
    /// It was written and synced as this record of its stream.
    Stored {
        /// The sequence number it was given.
        seq: u64,
    },
    /// The stream already holds exactly this event, as this record, so a
    /// retry changes nothing.
    Duplicate {
        /// The sequence number it has.
        seq: u64,
    },
    /// An erasure covers it. It is acknowledged, so the device stops
    /// sending it, and it is not stored.
    Erased,
}

/// Why one event of a batch was not stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// The stream holds a different event under the same ID. A retry never
    /// overwrites.
    Conflict {
        /// The sequence number of the event the stream holds.
        seq: u64,
    },
    /// It is larger than one record may be.
    TooLarge(EncodeError),
}

/// The records a read returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// The records of the stream whose sequence numbers are in the range
    /// asked for, in order.
    pub records: Vec<Stamped>,
}

/// What an erasure removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErasureReport {
    /// The IDs of the events removed from the stream, in the order it held
    /// them. Devices are told these and nothing else.
    pub erased: Vec<EventId>,
}

/// A record cut off at the end of a stream's newest segment, which opening
/// the log removed. It was never acknowledged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Torn {
    /// The stream.
    pub stream: Stream,
    /// The segment's month.
    pub month: LogMonth,
    /// The octets that were cut.
    pub range: Range<usize>,
}

/// What opening the log found and did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// The torn tails it cut.
    pub torn: Vec<Torn>,
    /// The damage it found and left as it is.
    pub damage: Vec<Damage>,
    /// The octets it cut from the end of the erasure ledger: an entry a
    /// crash cut off before the erasure was promised.
    pub ledger: Option<Range<usize>>,
}

/// An open log and what opening it found.
#[derive(Debug)]
pub struct Opened {
    /// The log.
    pub log: UserLog,
    /// What opening it found and did.
    pub report: Report,
}

/// Which stream the policy's name for a profile means.
///
/// A [`Permit`] names a profile by its public ID and a stream is kept under
/// the profile's internal ID. The identity store knows both; it implements
/// this, and nothing else should.
pub trait Profiles {
    /// The internal ID of the profile whose public ID is `public`, or
    /// `None` when the server holds no such profile.
    fn profile(&self, public: PublicId) -> Option<ProfileId>;
}

/// What the rebuild and the projections' catch-up feed records into.
pub trait ProjectionBuilder {
    /// Takes the next record of the stream being replayed.
    fn apply(&mut self, record: &Stamped);
}

/// The user log of one data directory.
#[derive(Debug)]
pub struct UserLog;

impl UserLog {
    /// Opens the log beneath `root`: reads the erasure ledger and every
    /// segment, cuts a torn tail off each stream's newest segment, finishes
    /// any erasure the ledger promised, and reports what it found.
    ///
    /// # Errors
    ///
    /// Returns [`LogError::Ledger`] when the ledger cannot be read,
    /// [`LogError::NewerRecord`] when a segment holds a record a newer
    /// version wrote, and [`LogError::Root`], [`LogError::Dir`] or
    /// [`LogError::Io`] when a file or a directory cannot be used.
    pub fn open(root: &DataRoot) -> Result<Opened, LogError> {
        let _ = root;
        Ok(Opened {
            log: Self,
            report: Report::default(),
        })
    }

    /// Appends `events`, one batch, at the server's time `now`, and returns
    /// what became of each once every one that was written is on disk.
    ///
    /// # Errors
    ///
    /// Returns [`LogError::Halted`] after an earlier failure, and
    /// [`LogError::Root`] or [`LogError::Io`] when a segment cannot be
    /// created, written or synced; nothing in the batch is then
    /// acknowledged, and the log halts.
    pub fn append(
        &self,
        root: &DataRoot,
        now: Timestamp,
        events: &[Event],
    ) -> Result<Vec<Result<Appended, Refused>>, LogError> {
        let _ = (root, now, events);
        Ok(Vec::new())
    }

    /// The records of one profile's stream whose sequence numbers are in
    /// `range`. The stream is the one `permit` was decided for and no
    /// other: the caller cannot name one.
    ///
    /// # Errors
    ///
    /// Returns [`LogError::Denied`] unless `permit` allows reading a
    /// profile's own data and `profiles` knows that profile, and
    /// [`LogError::Halted`], [`LogError::Root`], [`LogError::Io`] or
    /// [`LogError::NewerRecord`] when the stream cannot be read.
    pub fn read(
        &self,
        root: &DataRoot,
        permit: &Permit,
        profiles: &dyn Profiles,
        range: Range<u64>,
    ) -> Result<Page, LogError> {
        let _ = (root, permit, profiles, range);
        Ok(Page {
            records: Vec::new(),
        })
    }

    /// Feeds `sink` every record of `stream` after sequence number `after`,
    /// in order, and returns none of them. It takes no [`Permit`]: it is for
    /// the rebuild and the projections' catch-up, and only the startup
    /// module may call it.
    ///
    /// # Errors
    ///
    /// Returns [`LogError::Halted`], [`LogError::Root`], [`LogError::Io`]
    /// or [`LogError::NewerRecord`] when the stream cannot be read.
    pub fn replay_into(
        &self,
        root: &DataRoot,
        stream: Stream,
        after: u64,
        sink: &mut dyn ProjectionBuilder,
    ) -> Result<(), LogError> {
        let _ = (root, stream, after, sink);
        Ok(())
    }

    /// Erases what `selector` covers: records it in the ledger, synced,
    /// then removes the records from the segments, or the whole stream's
    /// directory. From the moment the ledger is synced, an event the
    /// selector covers is never stored again.
    ///
    /// # Errors
    ///
    /// Returns [`LogError::Halted`] after an earlier failure, and
    /// [`LogError::Root`], [`LogError::Dir`], [`LogError::Io`] or
    /// [`LogError::NewerRecord`] when the ledger or a segment cannot be
    /// read or written; the log then halts, and opening it again finishes
    /// the erasure if the ledger holds it.
    pub fn erase(&self, root: &DataRoot, selector: Selector) -> Result<ErasureReport, LogError> {
        let _ = (root, selector);
        Ok(ErasureReport { erased: Vec::new() })
    }
}

/// What code outside this crate must not be able to do with the log.
/// Rustdoc on stable does not check which error a compile-fail test
/// produced, so each shares its imports with the control, which compiles.
#[cfg(doctest)]
mod compile_fail {
    /// Control: a handler that asked the policy reads with the permit it
    /// was given.
    ///
    /// ```
    /// use gunmetal_core::authz::Permit;
    /// use gunmetal_durable::userlog::error::LogError;
    /// use gunmetal_durable::userlog::log::{Page, Profiles, UserLog};
    /// use gunmetal_fs::dataroot::DataRoot;
    ///
    /// fn history(
    ///     log: &UserLog,
    ///     root: &DataRoot,
    ///     permit: &Permit,
    ///     profiles: &dyn Profiles,
    /// ) -> Result<Page, LogError> {
    ///     log.read(root, permit, profiles, 1..101)
    /// }
    /// ```
    struct Control;

    /// Verifies: SEC-TM-024, SEC-API-010
    ///
    /// A person's records cannot be read without a `Permit`: the reader
    /// takes one, and there is no other reader that returns records.
    ///
    /// ```compile_fail,E0061
    /// use gunmetal_core::authz::Permit;
    /// use gunmetal_durable::userlog::error::LogError;
    /// use gunmetal_durable::userlog::log::{Page, Profiles, UserLog};
    /// use gunmetal_fs::dataroot::DataRoot;
    ///
    /// fn history(
    ///     log: &UserLog,
    ///     root: &DataRoot,
    ///     profiles: &dyn Profiles,
    /// ) -> Result<Page, LogError> {
    ///     log.read(root, profiles, 1..101)
    /// }
    /// ```
    struct NoReadWithoutAPermit;

    /// Verifies: SEC-TM-024, SEC-API-010
    ///
    /// A caller cannot say whose stream to read: the reader takes no
    /// stream, only the permit.
    ///
    /// ```compile_fail,E0061
    /// use gunmetal_core::authz::Permit;
    /// use gunmetal_durable::userlog::error::LogError;
    /// use gunmetal_durable::userlog::log::{Page, Profiles, UserLog};
    /// use gunmetal_fs::dataroot::DataRoot;
    ///
    /// fn history(
    ///     log: &UserLog,
    ///     root: &DataRoot,
    ///     permit: &Permit,
    ///     profiles: &dyn Profiles,
    ///     other: gunmetal_core::userdata::event::Stream,
    /// ) -> Result<Page, LogError> {
    ///     log.read(root, permit, profiles, other, 1..101)
    /// }
    /// ```
    struct NoStreamOfTheCallersChoosing;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::userlog::error::LedgerFlaw;
    use crate::userlog::scan::Problem;
    use crate::userlog::testing::{
        ALICE, BOB, bytes, data, frame, frame_version, header, love, of, play, record,
    };
    use gunmetal_core::authz::{
        Action, Capability, CapabilitySet, Context, DeviceClass, Elevation, Network, Owner,
        PrincipalFacts, PrincipalKind, Reach, RemoteAdmin, ResourceFacts, UserVerification, decide,
    };
    use gunmetal_core::client_context::PathClass;
    use gunmetal_core::id::IdKind;
    use gunmetal_core::userdata::codec;
    use gunmetal_core::userdata::erasure::Scope;
    use gunmetal_core::userdata::event::{
        Body, DocumentId, DocumentKind, DocumentRef, DocumentSnapshot,
    };
    use gunmetal_core::userdata::hlc::Hlc;
    use gunmetal_fs::dataroot::{DataRootError, Item, Kind, Listing, LogDirError, Op};
    use gunmetal_fs::path::{DataDir, DataPath, LogStream, USER_LOG};
    use proptest::prelude::*;
    use std::io::{self, Write};

    /// The erasure ledger and its directory, written out.
    const LEDGER: DataPath = DataPath::constant(DataDir::Durable, "erasure/ledger");
    const LEDGER_DIR: DataPath = DataPath::constant(DataDir::Durable, "erasure");

    /// 2026-09-30T23:59:59.999Z.
    const SEPTEMBER_END: i64 = 1_790_812_799_999;
    /// 2026-10-01T00:00:00Z.
    const OCTOBER_START: i64 = 1_790_812_800_000;
    /// 2026-10-15T00:00:00Z.
    const OCTOBER: i64 = 1_792_022_400_000;

    /// The data root's names for Alice's and Bob's streams, written out.
    const ALICE_DIR: LogStream = LogStream::Profile([0xA1; 16]);
    const BOB_DIR: LogStream = LogStream::Profile([0xB2; 16]);

    fn at(millis: i64) -> Timestamp {
        Timestamp::from_millis(millis).expect("a time in the years 0000 to 9999")
    }

    fn segment(dir: LogStream, year: u16, number: u8) -> DataPath {
        DataPath::log_segment(dir, of(year, number))
    }

    /// Opens the log, which must find nothing to report.
    fn opened(root: &DataRoot) -> UserLog {
        let opened = UserLog::open(root).expect("the log opens");
        assert_eq!(opened.report, Report::default());
        opened.log
    }

    /// Appends `events` in the middle of October 2026, and returns what
    /// became of each.
    fn stored(log: &UserLog, root: &DataRoot, events: &[Event]) -> Vec<Result<Appended, Refused>> {
        let outcomes = log
            .append(root, at(OCTOBER), events)
            .expect("the batch is written");
        assert_eq!(outcomes.len(), events.len(), "one outcome for each event");
        outcomes
    }

    /// The records a rebuild is fed for `stream` after number `after`.
    struct Seen(Vec<Stamped>);

    impl ProjectionBuilder for Seen {
        fn apply(&mut self, record: &Stamped) {
            self.0.push(record.clone());
        }
    }

    fn replayed(log: &UserLog, root: &DataRoot, stream: Stream, after: u64) -> Vec<Stamped> {
        let mut seen = Seen(Vec::new());
        log.replay_into(root, stream, after, &mut seen)
            .expect("the stream replays");
        seen.0
    }

    fn stamped(seq: u64, event: &Event) -> Stamped {
        Stamped {
            seq,
            event: event.clone(),
        }
    }

    /// The public ID of profile number `n`.
    fn public(n: u8) -> PublicId {
        PublicId::parse(&format!("prf_{n:026}"), IdKind::Profile).expect("a profile's public ID")
    }

    fn account() -> PublicId {
        PublicId::parse(&format!("usr_{:026}", 1), IdKind::User).expect("an account's public ID")
    }

    /// The identity store's side of the tests: profile 1 is Alice, 2 is
    /// Bob, 3 has written nothing, and there is no other.
    struct Known;

    impl Profiles for Known {
        fn profile(&self, public_id: PublicId) -> Option<ProfileId> {
            [(1, [0xA1; 16]), (2, [0xB2; 16]), (3, [0xC3; 16])]
                .into_iter()
                .find(|(n, _)| public(*n) == public_id)
                .map(|(_, id)| ProfileId::new(id))
        }
    }

    /// The permit the policy gives profile `n` for `action` on `resource`.
    fn permit(n: u8, action: Action, resource: &ResourceFacts) -> Permit {
        let facts = PrincipalFacts {
            kind: PrincipalKind::Member,
            account: Some(account()),
            profile: Some(public(n)),
            capabilities: CapabilitySet::of(&[
                Capability::OwnRead,
                Capability::OwnWrite,
                Capability::LibraryRead,
            ]),
            libraries: Vec::new(),
            device: DeviceClass::Personal,
            elevation: Elevation::Ordinary,
            verification: UserVerification::Stale,
            reach: Reach::Anywhere,
            scope: None,
        };
        let context = Context {
            path: PathClass::Loopback,
            network: Network::Same,
            remote_admin: RemoteAdmin::Allowed,
        };
        decide(&facts, action, resource, &context).expect("the policy allows it")
    }

    /// The permit profile `n` gets for reading its own data.
    fn own(n: u8) -> Permit {
        permit(
            n,
            Action::ReadOwnData,
            &ResourceFacts::Owned(Owner::Profile(public(n))),
        )
    }

    /// A ledger entry for Alice's stream, octet by octet: the floor, tag 0
    /// and her profile ID, then `scope`.
    fn alice_entry(floor: u64, scope: &[u8]) -> Vec<u8> {
        frame(&[&floor.to_le_bytes()[..], &[0], &[0xA1; 16], scope].concat())
    }

    /// The scope of an erasure of the one event with ID `id` repeated.
    fn one_event(id: u8) -> Vec<u8> {
        [&[0][..], &[id; 16]].concat()
    }

    fn io_error(item: Item, op: Op, kind: io::ErrorKind) -> DataRootError {
        DataRootError::Io { item, op, kind }
    }

    #[test]
    fn appends_each_event_to_the_segment_of_its_stream() {
        let data = data();
        let log = opened(&data.root);
        let batch = [
            play(1, ALICE, 1_000),
            play(2, ALICE, 2_000),
            play(1, BOB, 1_000),
        ];
        assert_eq!(
            log.append(&data.root, at(OCTOBER), &batch),
            Ok(vec![
                Ok(Appended::Stored { seq: 1 }),
                Ok(Appended::Stored { seq: 2 }),
                Ok(Appended::Stored { seq: 1 }),
            ])
        );
        let curation = love(9, Stream::Household, 3_000);
        let third = play(3, ALICE, 3_000);
        assert_eq!(
            log.append(&data.root, at(OCTOBER), &[curation.clone(), third.clone()]),
            Ok(vec![
                Ok(Appended::Stored { seq: 1 }),
                Ok(Appended::Stored { seq: 3 }),
            ])
        );
        assert_eq!(log.append(&data.root, at(OCTOBER), &[]), Ok(vec![]));
        assert_eq!(
            bytes(&data.root, &segment(ALICE_DIR, 2026, 10)),
            [
                header([0xA1; 16], 2026, 10),
                record(1, &batch[0]),
                record(2, &batch[1]),
                record(3, &third),
            ]
            .concat()
        );
        assert_eq!(
            bytes(&data.root, &segment(BOB_DIR, 2026, 10)),
            [header([0xB2; 16], 2026, 10), record(1, &batch[2])].concat()
        );
        assert_eq!(
            bytes(&data.root, &segment(LogStream::Household, 2026, 10)),
            [header([0x00; 16], 2026, 10), record(1, &curation)].concat()
        );
        assert_eq!(
            data.root.log_streams(),
            Ok(Listing {
                entries: vec![LogStream::Household, ALICE_DIR, BOB_DIR],
                foreign: vec![],
            })
        );
    }

    #[test]
    fn a_month_boundary_starts_a_new_segment_and_a_restart_reads_both() {
        let data = data();
        let log = opened(&data.root);
        let first = play(1, ALICE, 1_000);
        let second = play(2, ALICE, 2_000);
        let third = play(3, ALICE, 3_000);
        let fourth = play(4, ALICE, 4_000);
        assert_eq!(
            log.append(&data.root, at(SEPTEMBER_END), &[first.clone()]),
            Ok(vec![Ok(Appended::Stored { seq: 1 })])
        );
        assert_eq!(
            log.append(&data.root, at(OCTOBER_START), &[second.clone()]),
            Ok(vec![Ok(Appended::Stored { seq: 2 })])
        );
        // The server's clock steps back a month: the record still goes
        // after the last one, in the newest segment.
        assert_eq!(
            log.append(&data.root, at(SEPTEMBER_END), &[third.clone()]),
            Ok(vec![Ok(Appended::Stored { seq: 3 })])
        );
        let september = [header([0xA1; 16], 2026, 9), record(1, &first)].concat();
        let october = [
            header([0xA1; 16], 2026, 10),
            record(2, &second),
            record(3, &third),
        ]
        .concat();
        assert_eq!(bytes(&data.root, &segment(ALICE_DIR, 2026, 9)), september);
        assert_eq!(bytes(&data.root, &segment(ALICE_DIR, 2026, 10)), october);
        assert_eq!(
            data.root.log_segments(ALICE_DIR),
            Ok(Listing {
                entries: vec![of(2026, 9), of(2026, 10)],
                foreign: vec![],
            })
        );
        // After a restart everything acknowledged is there, in order, and
        // the numbers go on.
        drop(log);
        let log = opened(&data.root);
        assert_eq!(
            replayed(&log, &data.root, ALICE, 0),
            [stamped(1, &first), stamped(2, &second), stamped(3, &third)]
        );
        assert_eq!(
            stored(&log, &data.root, &[fourth.clone()]),
            [Ok(Appended::Stored { seq: 4 })]
        );
        assert_eq!(bytes(&data.root, &segment(ALICE_DIR, 2026, 9)), september);
        assert_eq!(
            bytes(&data.root, &segment(ALICE_DIR, 2026, 10)),
            [october, record(4, &fourth)].concat()
        );
    }

    #[test]
    fn the_same_event_twice_is_stored_once_and_another_under_its_id_is_refused() {
        let data = data();
        let log = opened(&data.root);
        let first = play(1, ALICE, 1_000);
        let second = play(2, ALICE, 2_000);
        assert_eq!(
            stored(&log, &data.root, &[first.clone()]),
            [Ok(Appended::Stored { seq: 1 })]
        );
        // A retry, then another event under the same ID.
        assert_eq!(
            stored(&log, &data.root, &[first.clone()]),
            [Ok(Appended::Duplicate { seq: 1 })]
        );
        assert_eq!(
            stored(&log, &data.root, &[play(1, ALICE, 1_001)]),
            [Err(Refused::Conflict { seq: 1 })]
        );
        // The same within one batch; and the same ID in another stream,
        // which is another event.
        assert_eq!(
            stored(
                &log,
                &data.root,
                &[
                    second.clone(),
                    second.clone(),
                    love(2, ALICE, 2_000),
                    play(1, BOB, 1_001),
                ]
            ),
            [
                Ok(Appended::Stored { seq: 2 }),
                Ok(Appended::Duplicate { seq: 2 }),
                Err(Refused::Conflict { seq: 2 }),
                Ok(Appended::Stored { seq: 1 }),
            ]
        );
        let held = [
            header([0xA1; 16], 2026, 10),
            record(1, &first),
            record(2, &second),
        ]
        .concat();
        assert_eq!(bytes(&data.root, &segment(ALICE_DIR, 2026, 10)), held);
        // A restart forgets none of it.
        drop(log);
        let log = opened(&data.root);
        assert_eq!(
            stored(
                &log,
                &data.root,
                &[first, play(2, ALICE, 9_999), play(3, ALICE, 3_000)]
            ),
            [
                Ok(Appended::Duplicate { seq: 1 }),
                Err(Refused::Conflict { seq: 2 }),
                Ok(Appended::Stored { seq: 3 }),
            ]
        );
    }

    #[test]
    fn an_event_too_large_for_a_record_is_refused_and_the_rest_is_stored() {
        let data = data();
        let log = opened(&data.root);
        let mut snapshot = play(1, ALICE, 1_000);
        // A payload of a record's whole cap, one mebibyte.
        snapshot.body = Body::DocumentSnapshot(DocumentSnapshot {
            document: DocumentRef {
                kind: DocumentKind::Playlist,
                id: DocumentId::new([3; 16]),
            },
            version: 1,
            payload: [0_u8; 16].repeat(65_536),
        });
        let small = play(2, ALICE, 2_000);
        let len = codec::encode(&snapshot).len() + 8;
        assert_eq!(
            stored(&log, &data.root, &[snapshot, small.clone()]),
            [
                Err(Refused::TooLarge(EncodeError::TooLong {
                    len,
                    max: 1_048_576
                })),
                Ok(Appended::Stored { seq: 1 }),
            ]
        );
        assert_eq!(
            bytes(&data.root, &segment(ALICE_DIR, 2026, 10)),
            [header([0xA1; 16], 2026, 10), record(1, &small)].concat()
        );
    }

    #[test]
    fn a_torn_final_record_is_cut_back_at_reopen_and_reported() {
        let data = data();
        let log = opened(&data.root);
        let first = play(1, ALICE, 1_000);
        let second = play(2, ALICE, 2_000);
        assert_eq!(
            stored(&log, &data.root, &[first.clone(), second.clone()]),
            [
                Ok(Appended::Stored { seq: 1 }),
                Ok(Appended::Stored { seq: 2 }),
            ]
        );
        drop(log);
        let path = segment(ALICE_DIR, 2026, 10);
        let whole = [header([0xA1; 16], 2026, 10), record(1, &first)].concat();
        // A crash cut the second record five octets short.
        let written = bytes(&data.root, &path);
        let end = written.len() - 5;
        data.root
            .replace(&path, &written[..end])
            .expect("the segment is cut");
        let opened = UserLog::open(&data.root).expect("the log opens");
        assert_eq!(
            opened.report,
            Report {
                torn: vec![Torn {
                    stream: ALICE,
                    month: of(2026, 10),
                    range: whole.len()..end,
                }],
                damage: vec![],
                ledger: None,
            }
        );
        assert_eq!(bytes(&data.root, &path), whole);
        // The record was never acknowledged: its number and its ID are free.
        assert_eq!(
            stored(&opened.log, &data.root, &[second.clone()]),
            [Ok(Appended::Stored { seq: 2 })]
        );
        assert_eq!(
            bytes(&data.root, &path),
            [whole, record(2, &second)].concat()
        );
    }

    #[test]
    fn a_newest_segment_cut_inside_its_header_gets_its_header_back() {
        let first = play(1, ALICE, 1_000);
        let path = segment(ALICE_DIR, 2026, 10);
        // What opening reports, and what the segment then holds, when a
        // crash left only the segment's first `kept` octets.
        let reopened = |kept: usize| {
            let data = data();
            let log = opened(&data.root);
            assert_eq!(
                stored(&log, &data.root, &[first.clone()]),
                [Ok(Appended::Stored { seq: 1 })]
            );
            drop(log);
            let written = bytes(&data.root, &path);
            data.root
                .replace(&path, &written[..kept])
                .expect("the segment is cut");
            let opened = UserLog::open(&data.root).expect("the log opens");
            let next = stored(&opened.log, &data.root, &[first.clone()]);
            (opened.report, next, bytes(&data.root, &path))
        };
        let whole = [header([0xA1; 16], 2026, 10), record(1, &first)].concat();
        assert_eq!(
            reopened(5),
            (
                Report {
                    torn: vec![Torn {
                        stream: ALICE,
                        month: of(2026, 10),
                        range: 0..5,
                    }],
                    damage: vec![],
                    ledger: None,
                },
                vec![Ok(Appended::Stored { seq: 1 })],
                whole.clone(),
            )
        );
        // An empty segment has nothing to cut, only a header to write.
        assert_eq!(
            reopened(0),
            (
                Report::default(),
                vec![Ok(Appended::Stored { seq: 1 })],
                whole
            )
        );
    }

    #[test]
    fn an_older_segment_is_never_cut_and_its_damage_is_reported() {
        let data = data();
        let log = opened(&data.root);
        let first = play(1, ALICE, 1_000);
        let second = play(2, ALICE, 2_000);
        let third = play(3, ALICE, 3_000);
        assert_eq!(
            log.append(&data.root, at(SEPTEMBER_END), &[first.clone(), second]),
            Ok(vec![
                Ok(Appended::Stored { seq: 1 }),
                Ok(Appended::Stored { seq: 2 }),
            ])
        );
        assert_eq!(
            stored(&log, &data.root, &[third.clone()]),
            [Ok(Appended::Stored { seq: 3 })]
        );
        drop(log);
        let path = segment(ALICE_DIR, 2026, 9);
        let start = [header([0xA1; 16], 2026, 9), record(1, &first)]
            .concat()
            .len();
        let written = bytes(&data.root, &path);
        let cut = written[..written.len() - 5].to_vec();
        data.root.replace(&path, &cut).expect("the segment is cut");
        let opened = UserLog::open(&data.root).expect("the log opens");
        assert_eq!(
            opened.report,
            Report {
                torn: vec![],
                damage: vec![Damage {
                    stream: ALICE,
                    month: of(2026, 9),
                    range: start..cut.len(),
                    after: Some(1),
                    problem: Problem::Unreadable,
                }],
                ledger: None,
            }
        );
        assert_eq!(bytes(&data.root, &path), cut);
        assert_eq!(
            replayed(&opened.log, &data.root, ALICE, 0),
            [stamped(1, &first), stamped(3, &third)]
        );
    }

    #[test]
    fn a_corrupt_middle_record_is_reported_with_its_range_and_the_rest_replays() {
        let data = data();
        let log = opened(&data.root);
        let first = play(1, ALICE, 1_000);
        let second = play(2, ALICE, 2_000);
        let third = play(3, ALICE, 3_000);
        stored(
            &log,
            &data.root,
            &[first.clone(), second.clone(), third.clone()],
        );
        drop(log);
        let path = segment(ALICE_DIR, 2026, 10);
        let start = [header([0xA1; 16], 2026, 10), record(1, &first)]
            .concat()
            .len();
        let end = start + record(2, &second).len();
        let mut damaged = bytes(&data.root, &path);
        damaged[start + 12] ^= 0x40;
        data.root
            .replace(&path, &damaged)
            .expect("the segment is damaged");
        let opened = UserLog::open(&data.root).expect("the log opens");
        assert_eq!(
            opened.report,
            Report {
                torn: vec![],
                damage: vec![Damage {
                    stream: ALICE,
                    month: of(2026, 10),
                    range: start..end,
                    after: Some(1),
                    problem: Problem::Unreadable,
                }],
                ledger: None,
            }
        );
        // Damage is never repaired in place.
        assert_eq!(bytes(&data.root, &path), damaged);
        assert_eq!(
            replayed(&opened.log, &data.root, ALICE, 0),
            [stamped(1, &first), stamped(3, &third)]
        );
        assert_eq!(
            stored(&opened.log, &data.root, &[play(4, ALICE, 4_000)]),
            [Ok(Appended::Stored { seq: 4 })]
        );
    }

    #[test]
    fn replays_a_stream_after_a_sequence_number_into_a_builder() {
        let data = data();
        let log = opened(&data.root);
        let first = play(1, ALICE, 1_000);
        let second = play(2, ALICE, 2_000);
        let third = love(3, ALICE, 3_000);
        let curation = love(9, Stream::Household, 3_000);
        log.append(&data.root, at(SEPTEMBER_END), &[first.clone()])
            .expect("the batch is written");
        stored(
            &log,
            &data.root,
            &[second.clone(), curation.clone(), third.clone()],
        );
        let all = [stamped(1, &first), stamped(2, &second), stamped(3, &third)];
        assert_eq!(replayed(&log, &data.root, ALICE, 0), all);
        assert_eq!(replayed(&log, &data.root, ALICE, 1), all[1..]);
        assert_eq!(replayed(&log, &data.root, ALICE, 2), all[2..]);
        assert_eq!(replayed(&log, &data.root, ALICE, 3), []);
        assert_eq!(
            replayed(&log, &data.root, Stream::Household, 0),
            [stamped(1, &curation)]
        );
        assert_eq!(replayed(&log, &data.root, BOB, 0), []);
    }

    /// Verifies: SEC-TM-024, SEC-API-010
    #[test]
    fn a_reader_serves_the_stream_its_permit_was_decided_for() {
        let data = data();
        let log = opened(&data.root);
        let mine = [
            play(1, ALICE, 1_000),
            play(2, ALICE, 2_000),
            love(3, ALICE, 3_000),
        ];
        let theirs = play(4, BOB, 1_000);
        stored(
            &log,
            &data.root,
            &[
                mine[0].clone(),
                theirs.clone(),
                mine[1].clone(),
                mine[2].clone(),
            ],
        );
        let page = |n: u8, range: Range<u64>| {
            log.read(&data.root, &own(n), &Known, range)
                .map(|page| page.records)
        };
        let all = vec![
            stamped(1, &mine[0]),
            stamped(2, &mine[1]),
            stamped(3, &mine[2]),
        ];
        assert_eq!(page(1, 0..u64::MAX), Ok(all.clone()));
        assert_eq!(page(1, 2..3), Ok(vec![stamped(2, &mine[1])]));
        assert_eq!(page(1, 2..4), Ok(all[1..].to_vec()));
        assert_eq!(page(1, 3..3), Ok(vec![]));
        assert_eq!(page(2, 0..u64::MAX), Ok(vec![stamped(1, &theirs)]));
        // A profile that has written nothing has nothing to read.
        assert_eq!(page(3, 0..u64::MAX), Ok(vec![]));
    }

    /// Verifies: SEC-TM-024, SEC-API-010
    #[test]
    fn a_permit_decided_for_anything_else_reads_nothing() {
        let data = data();
        let log = opened(&data.root);
        stored(&log, &data.root, &[play(1, ALICE, 1_000)]);
        // Decided for writing, for the account, for no owner at all, and
        // for a profile the server does not hold.
        for refused in [
            permit(
                1,
                Action::WriteOwnData,
                &ResourceFacts::Owned(Owner::Profile(public(1))),
            ),
            permit(
                1,
                Action::ReadOwnData,
                &ResourceFacts::Owned(Owner::Account(account())),
            ),
            permit(1, Action::BrowseLibrary, &ResourceFacts::Server),
            own(9),
        ] {
            assert_eq!(
                log.read(&data.root, &refused, &Known, 0..u64::MAX),
                Err(LogError::Denied)
            );
        }
        // A refusal is not a failure: the log goes on.
        assert_eq!(
            stored(&log, &data.root, &[play(2, ALICE, 2_000)]),
            [Ok(Appended::Stored { seq: 2 })]
        );
    }

    /// Verifies: SEC-TM-024
    #[test]
    fn a_record_of_another_profile_planted_in_a_segment_is_never_served() {
        let data = data();
        let log = opened(&data.root);
        let mine = play(1, ALICE, 1_000);
        stored(&log, &data.root, &[mine.clone()]);
        drop(log);
        // Bob's record, whole and numbered to fit, put after Alice's.
        let path = segment(ALICE_DIR, 2026, 10);
        let start = bytes(&data.root, &path).len();
        let planted = record(2, &play(7, BOB, 1_000));
        data.root
            .append(&path)
            .expect("the segment opens")
            .write_all(&planted)
            .expect("the record is planted");
        let opened = UserLog::open(&data.root).expect("the log opens");
        assert_eq!(
            opened.report,
            Report {
                torn: vec![],
                damage: vec![Damage {
                    stream: ALICE,
                    month: of(2026, 10),
                    range: start..start + planted.len(),
                    after: Some(1),
                    problem: Problem::Foreign,
                }],
                ledger: None,
            }
        );
        assert_eq!(
            opened.log.read(&data.root, &own(1), &Known, 0..u64::MAX),
            Ok(Page {
                records: vec![stamped(1, &mine)]
            })
        );
        assert_eq!(
            replayed(&opened.log, &data.root, ALICE, 0),
            [stamped(1, &mine)]
        );
    }

    /// Verifies: SEC-PRV-050
    #[test]
    fn an_erased_event_leaves_no_trace_in_the_stream_s_directory() {
        let data = data();
        let log = opened(&data.root);
        let kept = play(1, ALICE, 1_000);
        let gone = play(2, ALICE, 2_000);
        let last = love(3, ALICE, 3_000);
        stored(
            &log,
            &data.root,
            &[kept.clone(), gone.clone(), last.clone()],
        );
        let selector = Selector {
            stream: ALICE,
            scope: Scope::Event(gone.id),
        };
        assert_eq!(
            log.erase(&data.root, selector),
            Ok(ErasureReport {
                erased: vec![gone.id]
            })
        );
        // The segment is exactly its header and the records that stay,
        // under the numbers they had, and nothing else is in the
        // directory: no copy of the old segment is left beside it.
        let left = [
            header([0xA1; 16], 2026, 10),
            record(1, &kept),
            record(3, &last),
        ]
        .concat();
        assert_eq!(bytes(&data.root, &segment(ALICE_DIR, 2026, 10)), left);
        assert_eq!(
            data.root.log_segments(ALICE_DIR),
            Ok(Listing {
                entries: vec![of(2026, 10)],
                foreign: vec![],
            })
        );
        // The ledger holds the selector and the number the stream had
        // reached, and nothing of the event.
        assert_eq!(bytes(&data.root, &LEDGER), alice_entry(4, &one_event(2)));
        let after = [stamped(1, &kept), stamped(3, &last)];
        assert_eq!(replayed(&log, &data.root, ALICE, 0), after);
        // A restart changes none of it, and the erased record's number is
        // not given out again.
        drop(log);
        let log = opened(&data.root);
        assert_eq!(replayed(&log, &data.root, ALICE, 0), after);
        assert_eq!(
            stored(&log, &data.root, &[play(4, ALICE, 4_000)]),
            [Ok(Appended::Stored { seq: 4 })]
        );
        assert_eq!(
            bytes(&data.root, &segment(ALICE_DIR, 2026, 10)),
            [left, record(4, &play(4, ALICE, 4_000))].concat()
        );
    }

    #[test]
    fn what_an_erasure_covers_is_acknowledged_and_never_stored_again() {
        let data = data();
        let log = opened(&data.root);
        let early = play(1, ALICE, 1_000);
        let inside = play(2, ALICE, 1_500);
        let late = play(3, ALICE, 2_500);
        let loved = love(4, ALICE, 1_500);
        stored(
            &log,
            &data.root,
            &[early.clone(), inside.clone(), late.clone(), loved.clone()],
        );
        let erase = |scope| {
            log.erase(
                &data.root,
                Selector {
                    stream: ALICE,
                    scope,
                },
            )
            .map(|report| report.erased)
        };
        assert_eq!(erase(Scope::Event(early.id)), Ok(vec![early.id]));
        // A range removes the history in it; a love is not history.
        let range = Scope::Range {
            from: Hlc::new(1_200, 0),
            to: Hlc::new(2_000, 0),
        };
        assert_eq!(erase(range), Ok(vec![inside.id]));
        // A retry whose acknowledgement was lost, a play made offline in
        // the range, one under a new ID, and one just after the range.
        let after = play(6, ALICE, 2_001);
        assert_eq!(
            stored(
                &log,
                &data.root,
                &[early, inside, play(5, ALICE, 1_700), after.clone()]
            ),
            [
                Ok(Appended::Erased),
                Ok(Appended::Erased),
                Ok(Appended::Erased),
                Ok(Appended::Stored { seq: 5 }),
            ]
        );
        assert_eq!(
            bytes(&data.root, &segment(ALICE_DIR, 2026, 10)),
            [
                header([0xA1; 16], 2026, 10),
                record(3, &late),
                record(4, &loved),
                record(5, &after),
            ]
            .concat()
        );
        // Everything up to a clock: the newest record goes, and the number
        // it had is not used again.
        assert_eq!(erase(Scope::UpTo(Hlc::new(2_400, 0))), Ok(vec![after.id]));
        assert_eq!(
            stored(
                &log,
                &data.root,
                &[play(7, ALICE, 2_400), play(8, ALICE, 2_401)]
            ),
            [Ok(Appended::Erased), Ok(Appended::Stored { seq: 6 })]
        );
        // An erasure in a stream nobody has written to is promised too.
        assert_eq!(
            log.erase(
                &data.root,
                Selector {
                    stream: BOB,
                    scope: Scope::UpTo(Hlc::new(9_999, 0)),
                }
            ),
            Ok(ErasureReport { erased: vec![] })
        );
        // After a restart the ledger still answers for all of them.
        drop(log);
        let log = opened(&data.root);
        assert_eq!(
            stored(
                &log,
                &data.root,
                &[
                    play(1, ALICE, 1_000),
                    play(9, ALICE, 1_900),
                    play(1, BOB, 5_000),
                    play(2, BOB, 10_000),
                ]
            ),
            [
                Ok(Appended::Erased),
                Ok(Appended::Erased),
                Ok(Appended::Erased),
                Ok(Appended::Stored { seq: 1 }),
            ]
        );
    }

    /// Verifies: SEC-PRV-050
    #[test]
    fn erasing_a_whole_profile_removes_its_directory() {
        let data = data();
        let log = opened(&data.root);
        let first = play(1, ALICE, 1_000);
        let second = love(2, ALICE, 2_000);
        let theirs = play(1, BOB, 1_000);
        log.append(&data.root, at(SEPTEMBER_END), &[second.clone()])
            .expect("the batch is written");
        stored(&log, &data.root, &[first.clone(), theirs.clone()]);
        let whole = Selector {
            stream: ALICE,
            scope: Scope::Stream,
        };
        assert_eq!(
            log.erase(&data.root, whole),
            Ok(ErasureReport {
                erased: vec![second.id, first.id]
            })
        );
        // The directory is gone, with both segments; Bob's is as it was.
        let only_bob = Ok(Listing {
            entries: vec![BOB_DIR],
            foreign: vec![],
        });
        assert_eq!(data.root.log_streams(), only_bob);
        assert_eq!(
            data.root.log_segments(ALICE_DIR),
            Err(LogDirError::Root(io_error(
                Item::Path(DataPath::log_stream(ALICE_DIR)),
                Op::Inspect,
                io::ErrorKind::NotFound
            )))
        );
        assert_eq!(
            bytes(&data.root, &segment(BOB_DIR, 2026, 10)),
            [header([0xB2; 16], 2026, 10), record(1, &theirs)].concat()
        );
        assert_eq!(bytes(&data.root, &LEDGER), alice_entry(3, &[4]));
        // From now on everything of that profile is acknowledged and
        // dropped, history or not, and its directory does not come back.
        assert_eq!(
            stored(&log, &data.root, &[love(7, ALICE, 9_000), theirs.clone()]),
            [Ok(Appended::Erased), Ok(Appended::Duplicate { seq: 1 })]
        );
        assert_eq!(
            log.read(&data.root, &own(1), &Known, 0..u64::MAX),
            Ok(Page { records: vec![] })
        );
        drop(log);
        let log = opened(&data.root);
        assert_eq!(stored(&log, &data.root, &[first]), [Ok(Appended::Erased)]);
        assert_eq!(data.root.log_streams(), only_bob);
    }

    #[test]
    fn an_erasure_the_ledger_promised_is_finished_when_the_log_opens() {
        let data = data();
        let log = opened(&data.root);
        let first = play(1, ALICE, 1_000);
        let second = play(2, ALICE, 2_000);
        stored(
            &log,
            &data.root,
            &[first.clone(), second.clone(), play(1, BOB, 1_000)],
        );
        drop(log);
        // What a crash leaves after the ledger was synced and before any
        // segment was touched: one of Alice's events, and all of Bob.
        let bob = frame(&[&2_u64.to_le_bytes()[..], &[0], &[0xB2; 16], &[4]].concat());
        data.root
            .append(&LEDGER)
            .expect("the ledger opens")
            .write_all(&[alice_entry(3, &one_event(1)), bob].concat())
            .expect("the entries are written");
        let log = opened(&data.root);
        assert_eq!(
            bytes(&data.root, &segment(ALICE_DIR, 2026, 10)),
            [header([0xA1; 16], 2026, 10), record(2, &second)].concat()
        );
        assert_eq!(
            data.root.log_streams(),
            Ok(Listing {
                entries: vec![ALICE_DIR],
                foreign: vec![],
            })
        );
        assert_eq!(replayed(&log, &data.root, ALICE, 0), [stamped(2, &second)]);
        assert_eq!(
            stored(
                &log,
                &data.root,
                &[first, play(9, BOB, 1), play(3, ALICE, 3_000)]
            ),
            [
                Ok(Appended::Erased),
                Ok(Appended::Erased),
                Ok(Appended::Stored { seq: 3 }),
            ]
        );
    }

    #[test]
    fn the_ledger_holds_an_erasure_before_any_segment_is_touched() {
        let data = data();
        let log = opened(&data.root);
        let first = play(1, ALICE, 1_000);
        stored(&log, &data.root, &[first.clone()]);
        // The stream's directory goes missing behind the log's back, so
        // the erasure fails at its second step.
        data.root
            .remove_log_stream(ALICE_DIR)
            .expect("the directory is removed");
        let selector = Selector {
            stream: ALICE,
            scope: Scope::Event(first.id),
        };
        assert_eq!(
            log.erase(&data.root, selector),
            Err(LogError::Dir(LogDirError::Root(io_error(
                Item::Path(DataPath::log_stream(ALICE_DIR)),
                Op::Inspect,
                io::ErrorKind::NotFound
            ))))
        );
        let promised = alice_entry(2, &one_event(1));
        assert_eq!(bytes(&data.root, &LEDGER), promised);
        // The files and the log's memory of them may now differ, so it
        // refuses everything.
        assert_eq!(
            log.append(&data.root, at(OCTOBER), &[play(1, BOB, 1_000)]),
            Err(LogError::Halted)
        );
        assert_eq!(log.erase(&data.root, selector), Err(LogError::Halted));
        assert_eq!(
            log.read(&data.root, &own(2), &Known, 0..u64::MAX),
            Err(LogError::Halted)
        );
        assert_eq!(
            log.replay_into(&data.root, BOB, 0, &mut Seen(Vec::new())),
            Err(LogError::Halted)
        );
        assert_eq!(bytes(&data.root, &LEDGER), promised);
        // Opened again, it keeps the promise.
        let log = opened(&data.root);
        assert_eq!(stored(&log, &data.root, &[first]), [Ok(Appended::Erased)]);
    }

    #[test]
    fn a_failed_write_or_read_halts_the_log_until_it_is_opened_again() {
        let data = data();
        let log = opened(&data.root);
        // Something made Alice's directory behind the log's back.
        let folder = DataPath::log_stream(ALICE_DIR);
        data.root
            .create_dir(&folder)
            .expect("the directory is made");
        let batch = [play(1, BOB, 1_000), play(1, ALICE, 1_000)];
        assert_eq!(
            log.append(&data.root, at(OCTOBER), &batch),
            Err(LogError::Root(io_error(
                Item::Path(folder),
                Op::CreateDir,
                io::ErrorKind::AlreadyExists
            )))
        );
        assert_eq!(
            log.append(&data.root, at(OCTOBER), &[play(2, BOB, 2_000)]),
            Err(LogError::Halted)
        );
        // Opened again, the log finds the directory, and Bob's record,
        // which reached its segment and was never acknowledged.
        let log = opened(&data.root);
        assert_eq!(
            stored(&log, &data.root, &batch),
            [
                Ok(Appended::Duplicate { seq: 1 }),
                Ok(Appended::Stored { seq: 1 }),
            ]
        );
        // A stream whose segment has gone cannot be read.
        data.root
            .remove_log_stream(BOB_DIR)
            .expect("the directory is removed");
        assert_eq!(
            log.read(&data.root, &own(2), &Known, 0..u64::MAX),
            Err(LogError::Root(io_error(
                Item::Path(segment(BOB_DIR, 2026, 10)),
                Op::Open,
                io::ErrorKind::NotFound
            )))
        );
        assert_eq!(
            log.read(&data.root, &own(1), &Known, 0..u64::MAX),
            Err(LogError::Halted)
        );
    }

    #[test]
    fn does_not_open_on_files_it_cannot_use() {
        let report = |root: &DataRoot| UserLog::open(root).map(|opened| opened.report);
        // A file where the log's directory belongs.
        let blocked = data();
        blocked
            .root
            .replace(&USER_LOG, b"")
            .expect("the file is written");
        assert_eq!(
            report(&blocked.root),
            Err(LogError::Dir(LogDirError::Root(DataRootError::WrongKind {
                item: Item::Path(USER_LOG),
                found: Kind::File,
            })))
        );
        // A directory where the ledger belongs.
        let no_ledger = data();
        for path in [LEDGER_DIR, LEDGER] {
            no_ledger
                .root
                .create_dir(&path)
                .expect("the directory is made");
        }
        assert_eq!(
            report(&no_ledger.root),
            Err(LogError::Io(io::ErrorKind::IsADirectory))
        );
        // A ledger damaged before its end.
        let damaged = data();
        let entry = alice_entry(1, &[4]);
        let mut ledger_bytes = [entry.clone(), entry.clone()].concat();
        ledger_bytes[12] ^= 0x40;
        damaged
            .root
            .create_dir(&LEDGER_DIR)
            .expect("the directory is made");
        damaged
            .root
            .replace(&LEDGER, &ledger_bytes)
            .expect("the ledger is written");
        assert_eq!(
            report(&damaged.root),
            Err(LogError::Ledger(LedgerFlaw::Damaged {
                range: 0..entry.len()
            }))
        );
        // A record a newer version wrote, after one this version reads.
        let newer = data();
        let log = opened(&newer.root);
        stored(&log, &newer.root, &[play(1, ALICE, 1_000)]);
        drop(log);
        let path = segment(ALICE_DIR, 2026, 10);
        let offset = bytes(&newer.root, &path).len();
        newer
            .root
            .append(&path)
            .expect("the segment opens")
            .write_all(&frame_version(2, b"a record of a newer format"))
            .expect("the record is written");
        assert_eq!(
            report(&newer.root),
            Err(LogError::NewerRecord {
                stream: ALICE,
                month: of(2026, 10),
                offset,
                version: 2,
            })
        );
    }

    #[test]
    fn an_entry_cut_off_at_the_end_of_the_ledger_is_cut_and_reported() {
        let data = data();
        let log = opened(&data.root);
        let first = play(1, ALICE, 1_000);
        let second = play(2, ALICE, 2_000);
        stored(&log, &data.root, &[first.clone(), second.clone()]);
        log.erase(
            &data.root,
            Selector {
                stream: ALICE,
                scope: Scope::Event(first.id),
            },
        )
        .expect("the erasure is done");
        drop(log);
        // A crash cut a second entry three octets short.
        let whole = bytes(&data.root, &LEDGER);
        let entry = alice_entry(3, &one_event(2));
        let cut = &entry[..entry.len() - 3];
        data.root
            .append(&LEDGER)
            .expect("the ledger opens")
            .write_all(cut)
            .expect("the entry is written");
        let opened = UserLog::open(&data.root).expect("the log opens");
        assert_eq!(
            opened.report,
            Report {
                torn: vec![],
                damage: vec![],
                ledger: Some(whole.len()..whole.len() + cut.len()),
            }
        );
        assert_eq!(bytes(&data.root, &LEDGER), whole);
        // That erasure was never promised: the event is still there.
        assert_eq!(
            replayed(&opened.log, &data.root, ALICE, 0),
            [stamped(2, &second)]
        );
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(16))]

        /// Verifies: SEC-TM-024, SEC-API-010
        #[test]
        fn a_reader_never_returns_an_event_of_another_profile(
            plays in proptest::collection::vec((any::<bool>(), 0..10_000_u64), 0..12),
        ) {
            let data = data();
            let log = opened(&data.root);
            // One event of each profile, then the generated ones.
            let mut events = vec![play(200, ALICE, 1), play(201, BOB, 1)];
            events.extend(
                (1_u8..)
                    .zip(&plays)
                    .map(|(id, (mine, wall_ms))| play(id, [BOB, ALICE][usize::from(*mine)], *wall_ms)),
            );
            stored(&log, &data.root, &events);
            for (n, stream) in [(1, ALICE), (2, BOB)] {
                let expected: Vec<Stamped> = (1_u64..)
                    .zip(events.iter().filter(|event| event.stream == stream))
                    .map(|(seq, event)| stamped(seq, event))
                    .collect();
                let page = log.read(&data.root, &own(n), &Known, 0..u64::MAX);
                prop_assert_eq!(page, Ok(Page { records: expected }));
            }
        }
    }
}
