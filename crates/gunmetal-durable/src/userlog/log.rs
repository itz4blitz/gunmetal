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
//! finished before anything is served. The household's stream is never
//! erased whole: that selector is refused before anything is written, so
//! the ledger never holds a promise the log could not keep.
//!
//! **After a failure.** An operation that fails part of the way through
//! may have left the files and the log's memory of them apart. The log then
//! refuses every call with [`LogError::Halted`] until it is opened again,
//! which reads the files afresh.
//!
//! An `fsync` cannot be observed on a real file from a test, and neither
//! can a power loss; the tests show what truncation and a failed step leave
//! behind, and the order of the syncs is this module's code to review.

use std::collections::BTreeMap;
use std::collections::btree_map::Entry as Slot;
use std::fs::File;
use std::io::{Read, Write};
use std::ops::Range;
use std::sync::{Mutex, PoisonError};

use gunmetal_core::authz::{Action, Owner, Permit};
use gunmetal_core::crypto::sha256;
use gunmetal_core::id::PublicId;
use gunmetal_core::logframe::{self, EncodeError};
use gunmetal_core::time::Timestamp;
use gunmetal_core::userdata::codec;
use gunmetal_core::userdata::erasure::{Ledger, Scope, Selector};
use gunmetal_core::userdata::event::{Event, EventId, ProfileId, Stream};
use gunmetal_fs::dataroot::DataRoot;
use gunmetal_fs::path::{DataPath, LogMonth, USER_LOG};

use crate::userlog::error::LogError;
use crate::userlog::ledger::{self, Entry};
use crate::userlog::place;
use crate::userlog::record::{self, Stamped};
use crate::userlog::scan::{self, Damage, Held};

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

/// What the log knows about one stream.
#[derive(Debug, Default)]
struct Shelf {
    /// The months it has a segment for, oldest first.
    months: Vec<LogMonth>,
    /// The sequence number its next record gets.
    next: u64,
    /// The events it holds: each ID with its sequence number and the
    /// SHA-256 of the event's octets.
    ids: BTreeMap<EventId, (u64, [u8; 32])>,
}

impl Shelf {
    /// The IDs of the events the stream holds, in the order of their
    /// sequence numbers.
    fn in_order(self) -> Vec<EventId> {
        let mut numbered: Vec<(u64, EventId)> = self
            .ids
            .into_iter()
            .map(|(id, (seq, _))| (seq, id))
            .collect();
        numbered.sort_unstable();
        numbered.into_iter().map(|(_, id)| id).collect()
    }
}

/// What reading one segment at opening, or after an erasure, left.
struct Settled {
    /// The records the segment still holds.
    kept: Vec<Held>,
    /// The records the ledger covers, which were rewritten away.
    erased: Vec<Held>,
    /// The sequence number of the last record read, kept or not.
    last: Option<u64>,
}

/// Everything the writer knows, behind the log's one lock.
#[derive(Debug, Default)]
struct State {
    /// Whether an operation failed part of the way through.
    halted: bool,
    /// The selectors of every erasure, as the ledger file holds them.
    ledger: Ledger,
    /// For each stream an erasure names, the highest sequence number the
    /// stream had reached when one was recorded.
    floors: BTreeMap<Stream, u64>,
    /// The streams that have a directory.
    streams: BTreeMap<Stream, Shelf>,
}

/// The user log of one data directory.
#[derive(Debug)]
pub struct UserLog {
    state: Mutex<State>,
}

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
        // A directory that is already there, or cannot be made, shows when
        // it is listed or read below. The data root opens no layout
        // directory itself, so one made here is not synced into `durable`.
        drop(root.create_dir(&USER_LOG));
        drop(root.create_dir(&ledger::DIR));
        let mut report = Report::default();
        let mut state = State::default();
        state.read_ledger(root, &mut report)?;
        let streams = root.log_streams().map_err(LogError::Dir)?.entries;
        for dir in streams {
            state.load(root, place::stream(dir), &mut report)?;
        }
        Ok(Opened {
            log: Self {
                state: Mutex::new(state),
            },
            report,
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
        self.with(|state| state.append(root, now, events))
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
        let stream = match (permit.action(), permit.owner()) {
            (Action::ReadOwnData, Some(Owner::Profile(public))) => {
                profiles.profile(public).map(Stream::Profile)
            }
            _ => None,
        };
        let stream = stream.ok_or(LogError::Denied)?;
        let records = self.with(|state| state.records(root, stream))?;
        Ok(Page {
            records: records
                .into_iter()
                .filter(move |record| range.contains(&record.seq))
                .collect(),
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
        let records = self.with(|state| state.records(root, stream))?;
        for record in records.iter().filter(|record| record.seq > after) {
            sink.apply(record);
        }
        Ok(())
    }

    /// Erases what `selector` covers: records it in the ledger, synced,
    /// then removes the records from the segments, or the whole stream's
    /// directory. From the moment the ledger is synced, an event the
    /// selector covers is never stored again.
    ///
    /// # Errors
    ///
    /// Returns [`LogError::Household`] when `selector` names the household's
    /// whole stream, which is never erased; nothing is then written and the
    /// log goes on. Returns [`LogError::Halted`] after an earlier failure,
    /// and [`LogError::Root`], [`LogError::Dir`], [`LogError::Io`] or
    /// [`LogError::NewerRecord`] when the ledger or a segment cannot be
    /// read or written; the log then halts, and opening it again finishes
    /// the erasure if the ledger holds it.
    pub fn erase(&self, root: &DataRoot, selector: Selector) -> Result<ErasureReport, LogError> {
        if ledger::accepts(selector) {
            self.with(|state| state.erase(root, selector))
        } else {
            Err(LogError::Household)
        }
    }

    /// Runs `work` on the writer's state, one caller at a time, unless the
    /// log has halted; and halts it when `work` fails.
    fn with<R>(&self, work: impl FnOnce(&mut State) -> Result<R, LogError>) -> Result<R, LogError> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if state.halted {
            return Err(LogError::Halted);
        }
        let outcome = work(&mut state);
        state.halted = outcome.is_err();
        outcome
    }
}

impl State {
    /// Reads the ledger file, which it makes if it is not there: cuts an
    /// entry a crash cut off, then takes every entry in.
    fn read_ledger(&mut self, root: &DataRoot, report: &mut Report) -> Result<(), LogError> {
        let bytes = new_ledger(root).and_then(|()| read(root, &ledger::FILE))?;
        let parsed = ledger::parse(&bytes, &mut scan::budget(&bytes)).map_err(LogError::Ledger)?;
        report.ledger.clone_from(&parsed.torn);
        parsed
            .torn
            .map_or(Ok(()), |torn| {
                root.replace(&ledger::FILE, bytes.get(..torn.start).unwrap_or_default())
            })
            .map_err(LogError::Root)
            .and_then(|()| {
                parsed
                    .entries
                    .into_iter()
                    .try_for_each(|entry| self.adopt(root, entry))
            })
    }

    /// Takes in an entry the ledger file holds. A whole profile's directory
    /// is removed here if it is still there; the records other entries
    /// cover go when their segments are read. No entry names the
    /// household's whole stream: the ledger's reader takes none for one.
    fn adopt(&mut self, root: &DataRoot, entry: Entry) -> Result<(), LogError> {
        let removed = if entry.selector.scope == Scope::Stream {
            root.remove_log_stream(place::dir(entry.selector.stream))
        } else {
            Ok(())
        };
        removed.map_err(LogError::Dir).map(|()| self.record(entry))
    }

    /// Holds `entry` in memory, as the ledger file already does.
    fn record(&mut self, entry: Entry) {
        let floor = self.floors.entry(entry.selector.stream).or_default();
        *floor = entry.floor.max(*floor);
        self.ledger.record(entry.selector);
    }

    /// The lowest sequence number `stream`'s next record may get: 1, or the
    /// number the stream had reached when an erasure was recorded, so that
    /// the number of an erased record is never given out again.
    fn floor(&self, stream: Stream) -> u64 {
        self.floors.get(&stream).copied().unwrap_or(0).max(1)
    }

    /// Reads every segment of `stream` from disk, as [`State::settle`]
    /// leaves them, and holds what the stream now is. Returns the IDs of
    /// the events the ledger covered, which are gone.
    fn load(
        &mut self,
        root: &DataRoot,
        stream: Stream,
        report: &mut Report,
    ) -> Result<Vec<EventId>, LogError> {
        let months = root
            .log_segments(place::dir(stream))
            .map_err(LogError::Dir)?
            .entries;
        let mut held = Shelf {
            months: months.clone(),
            next: self.floor(stream),
            ids: BTreeMap::new(),
        };
        let mut erased = Vec::new();
        let mut last = None;
        for month in &months {
            let newest = months.last() == Some(month);
            let settled = self.settle(root, (stream, *month, newest), last, report)?;
            last = settled.last;
            for kept in settled.kept {
                let Stamped { seq, event } = kept.stamped;
                held.next = seq.saturating_add(1).max(held.next);
                held.ids
                    .insert(event.id, (seq, sha256(&codec::encode(&event))));
            }
            erased.extend(settled.erased.iter().map(|gone| gone.stamped.event.id));
        }
        self.streams.insert(stream, held);
        Ok(erased)
    }

    /// Reads the segment `at` names (a stream, a month, and whether it is
    /// the stream's newest) and leaves it as it should be: without the
    /// records the ledger covers and, if it is the newest, without a torn
    /// tail and with its header. Damage stays where it is and is reported.
    fn settle(
        &self,
        root: &DataRoot,
        at: (Stream, LogMonth, bool),
        last: Option<u64>,
        report: &mut Report,
    ) -> Result<Settled, LogError> {
        let (stream, month, newest) = at;
        let path = DataPath::log_segment(place::dir(stream), month);
        read(root, &path).and_then(|bytes| {
            scan::scan(stream, month, &bytes, last, &mut scan::budget(&bytes)).and_then(|found| {
                let last = found
                    .records
                    .last()
                    .map_or(last, |held| Some(held.stamped.seq));
                let (kept, erased): (Vec<Held>, Vec<Held>) = found
                    .records
                    .into_iter()
                    .partition(|held| self.ledger.admits(&held.stamped.event));
                let cut = found.torn.filter(|_| newest);
                let end = cut.as_ref().map_or(bytes.len(), |tail| tail.start);
                let gone: Vec<Range<usize>> =
                    erased.iter().map(|held| held.range.clone()).collect();
                let mut left = without(&bytes, end, &gone);
                if newest && left.is_empty() {
                    left = place::header(stream, month);
                }
                // The torn tail is the scan's last piece of damage; it is
                // reported as cut, not as damage.
                let mut damage = found.damage;
                damage.truncate(damage.len().saturating_sub(usize::from(cut.is_some())));
                report.damage.extend(damage);
                report.torn.extend(cut.map(|range| Torn {
                    stream,
                    month,
                    range,
                }));
                let rewritten = if left == bytes {
                    Ok(())
                } else {
                    root.replace(&path, &left)
                };
                rewritten
                    .map_err(LogError::Root)
                    .map(|()| Settled { kept, erased, last })
            })
        })
    }

    /// Appends one batch and syncs it.
    fn append(
        &mut self,
        root: &DataRoot,
        now: Timestamp,
        events: &[Event],
    ) -> Result<Vec<Result<Appended, Refused>>, LogError> {
        let month = place::month(now);
        let mut open = BTreeMap::new();
        let outcomes: Result<Vec<_>, LogError> = events
            .iter()
            .map(|event| self.one(root, month, event, &mut open))
            .collect();
        // Group commit: each segment written is synced once, and only then
        // is anything in the batch acknowledged (ADR 3, section 6).
        outcomes.and_then(|outcomes| {
            open.values()
                .try_for_each(File::sync_data)
                .map_err(LogError::from)
                .map(|()| outcomes)
        })
    }

    /// Decides what becomes of `event` and, if it is new, writes its record
    /// to its stream's segment, which stays in `open` until the batch is
    /// synced.
    fn one(
        &mut self,
        root: &DataRoot,
        now: LogMonth,
        event: &Event,
        open: &mut BTreeMap<Stream, File>,
    ) -> Result<Result<Appended, Refused>, LogError> {
        // The erasure ledger first: an erased event never comes back.
        if !self.ledger.admits(event) {
            return Ok(Ok(Appended::Erased));
        }
        let octets = codec::encode(event);
        let digest = sha256(&octets);
        let known = self.streams.get(&event.stream);
        let held = known.and_then(|shelf| shelf.ids.get(&event.id)).copied();
        if let Some((seq, stored)) = held {
            return Ok(if stored == digest {
                Ok(Appended::Duplicate { seq })
            } else {
                Err(Refused::Conflict { seq })
            });
        }
        let seq = known.map_or(self.floor(event.stream), |shelf| shelf.next);
        let frame = match logframe::encode(&record::payload(seq, &octets)) {
            Ok(frame) => frame,
            Err(error) => return Ok(Err(Refused::TooLarge(error))),
        };
        let file = match open.entry(event.stream) {
            Slot::Occupied(held) => held.into_mut(),
            Slot::Vacant(slot) => slot.insert(self.segment(root, event.stream, now)?),
        };
        file.write_all(&frame).map_err(LogError::from).map(|()| {
            let stream_state = self.streams.entry(event.stream).or_default();
            stream_state.next = seq.saturating_add(1);
            stream_state.ids.insert(event.id, (seq, digest));
            Ok(Appended::Stored { seq })
        })
    }

    /// Opens the segment `stream` appends to in month `now`, making the
    /// stream's directory and the segment when they are not there yet.
    fn segment(
        &mut self,
        root: &DataRoot,
        stream: Stream,
        now: LogMonth,
    ) -> Result<File, LogError> {
        let dir = place::dir(stream);
        let folder = DataPath::log_stream(dir);
        let made = if self.streams.contains_key(&stream) {
            Ok(())
        } else {
            // A stream's first record: its directory, synced into the
            // log's own before anything in it is acknowledged.
            root.create_dir(&folder)
                .and_then(|()| root.open_read(&USER_LOG))
                .map_err(LogError::Root)
                .and_then(|log| log.sync_all().map_err(LogError::from))
        };
        made?;
        // `next` is filled by the caller after a successful write; only the
        // months list is used here.
        let stream_state = self.streams.entry(stream).or_default();
        // The server's clock can step back; a stream's records stay in
        // order all the same, in its newest segment.
        let newest = stream_state.months.last().copied();
        let month = newest.map_or(now, |newest| newest.max(now));
        let path = DataPath::log_segment(dir, month);
        if newest == Some(month) {
            return root.append(&path).map_err(LogError::Root);
        }
        // A new segment: created exclusively with its header, then synced
        // with its directory (ADR 3, section 6).
        root.create_new(&path)
            .map_err(LogError::Root)
            .and_then(|mut file| {
                file.write_all(&place::header(stream, month))
                    .and_then(|()| file.sync_all())
                    .map_err(LogError::from)
                    .and_then(|()| root.open_read(&folder).map_err(LogError::Root))
                    .and_then(|synced| synced.sync_all().map_err(LogError::from))
                    .map(|()| {
                        stream_state.months.push(month);
                        file
                    })
            })
    }

    /// The records `stream` holds, read from its segments.
    fn records(&self, root: &DataRoot, stream: Stream) -> Result<Vec<Stamped>, LogError> {
        let months = self
            .streams
            .get(&stream)
            .map(|shelf| shelf.months.clone())
            .unwrap_or_default();
        let mut records = Vec::new();
        let mut last = None;
        for month in months {
            let path = DataPath::log_segment(place::dir(stream), month);
            let found = read(root, &path).and_then(|bytes| {
                scan::scan(stream, month, &bytes, last, &mut scan::budget(&bytes))
            })?;
            last = found
                .records
                .last()
                .map_or(last, |held| Some(held.stamped.seq));
            records.extend(found.records.into_iter().map(|held| held.stamped));
        }
        Ok(records)
    }

    /// Records `selector` in the ledger, synced, then removes what it
    /// covers (ADR 3, section 8, steps 1 and 3).
    fn erase(&mut self, root: &DataRoot, selector: Selector) -> Result<ErasureReport, LogError> {
        let entry = Entry {
            selector,
            floor: self
                .streams
                .get(&selector.stream)
                .map_or(0, |shelf| shelf.next),
        };
        root.append(&ledger::FILE)
            .map_err(LogError::Root)
            .and_then(|mut file| {
                file.write_all(&ledger::frame(&entry))
                    .and_then(|()| file.sync_data())
                    .map_err(LogError::from)
            })
            .and_then(|()| {
                self.record(entry);
                self.sweep(root, selector)
            })
            .map(|erased| ErasureReport { erased })
    }

    /// Removes from disk what `selector`, now in the ledger, covers, and
    /// returns the IDs of the events removed.
    fn sweep(&mut self, root: &DataRoot, selector: Selector) -> Result<Vec<EventId>, LogError> {
        let stream = selector.stream;
        if selector.scope == Scope::Stream {
            let erased = self
                .streams
                .remove(&stream)
                .map(Shelf::in_order)
                .unwrap_or_default();
            return root
                .remove_log_stream(place::dir(stream))
                .map_err(LogError::Dir)
                .map(|()| erased);
        }
        if self.streams.contains_key(&stream) {
            self.load(root, stream, &mut Report::default())
        } else {
            Ok(Vec::new())
        }
    }
}

/// Makes the ledger's file if it is not there, and syncs it and the
/// directory that holds it, so that an erasure promised in it later is not
/// lost with the file. A ledger that is already there, or cannot be made,
/// shows when it is read.
fn new_ledger(root: &DataRoot) -> Result<(), LogError> {
    root.create_new(&ledger::FILE).map_or(Ok(()), |file| {
        file.sync_all()
            .map_err(LogError::from)
            .and_then(|()| root.open_read(&ledger::DIR).map_err(LogError::Root))
            .and_then(|dir| dir.sync_all().map_err(LogError::from))
    })
}

/// Everything the file at `path` holds.
fn read(root: &DataRoot, path: &DataPath) -> Result<Vec<u8>, LogError> {
    root.open_read(path)
        .map_err(LogError::Root)
        .and_then(|mut file| {
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)
                .map(|_| bytes)
                .map_err(LogError::from)
        })
}

/// The first `end` octets of `bytes` without the ranges `gone`, which are
/// in order, apart and before `end`.
fn without(bytes: &[u8], end: usize, gone: &[Range<usize>]) -> Vec<u8> {
    let mut left = Vec::new();
    let mut from = 0;
    for range in gone {
        left.extend_from_slice(bytes.get(from..range.start).unwrap_or_default());
        from = range.end;
    }
    left.extend_from_slice(bytes.get(from..end).unwrap_or_default());
    left
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
    use std::slice::from_ref;

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
            log.append(&data.root, at(SEPTEMBER_END), from_ref(&first)),
            Ok(vec![Ok(Appended::Stored { seq: 1 })])
        );
        assert_eq!(
            log.append(&data.root, at(OCTOBER_START), from_ref(&second)),
            Ok(vec![Ok(Appended::Stored { seq: 2 })])
        );
        // The server's clock steps back a month: the record still goes
        // after the last one, in the newest segment.
        assert_eq!(
            log.append(&data.root, at(SEPTEMBER_END), from_ref(&third)),
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
        let log = opened(&data.root);
        assert_eq!(
            replayed(&log, &data.root, ALICE, 0),
            [stamped(1, &first), stamped(2, &second), stamped(3, &third)]
        );
        assert_eq!(
            stored(&log, &data.root, from_ref(&fourth)),
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
            stored(&log, &data.root, from_ref(&first)),
            [Ok(Appended::Stored { seq: 1 })]
        );
        // A retry, then another event under the same ID.
        assert_eq!(
            stored(&log, &data.root, from_ref(&first)),
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
            stored(&opened.log, &data.root, from_ref(&second)),
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
                stored(&log, &data.root, from_ref(&first)),
                [Ok(Appended::Stored { seq: 1 })]
            );
            let written = bytes(&data.root, &path);
            data.root
                .replace(&path, &written[..kept])
                .expect("the segment is cut");
            let opened = UserLog::open(&data.root).expect("the log opens");
            let next = stored(&opened.log, &data.root, from_ref(&first));
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
            stored(&log, &data.root, from_ref(&third)),
            [Ok(Appended::Stored { seq: 3 })]
        );
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
        log.append(&data.root, at(SEPTEMBER_END), from_ref(&first))
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
        stored(&log, &data.root, from_ref(&mine));
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
        log.append(&data.root, at(SEPTEMBER_END), from_ref(&second))
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
        let log = opened(&data.root);
        assert_eq!(stored(&log, &data.root, &[first]), [Ok(Appended::Erased)]);
        assert_eq!(data.root.log_streams(), only_bob);
    }

    #[test]
    fn never_promises_to_erase_the_household_s_whole_stream() {
        let data = data();
        let log = opened(&data.root);
        let whole = Selector {
            stream: Stream::Household,
            scope: Scope::Stream,
        };
        // Refused while the household has written nothing, and once it has.
        assert_eq!(log.erase(&data.root, whole), Err(LogError::Household));
        let curation = love(9, Stream::Household, 3_000);
        assert_eq!(
            stored(&log, &data.root, from_ref(&curation)),
            [Ok(Appended::Stored { seq: 1 })]
        );
        assert_eq!(log.erase(&data.root, whole), Err(LogError::Household));
        // Nothing was promised: the ledger is empty, and the stream is as
        // it was.
        assert_eq!(bytes(&data.root, &LEDGER), Vec::<u8>::new());
        let path = segment(LogStream::Household, 2026, 10);
        let held = [header([0x00; 16], 2026, 10), record(1, &curation)].concat();
        assert_eq!(bytes(&data.root, &path), held);
        // A refusal is not a failure: the log goes on.
        let later = love(10, Stream::Household, 4_000);
        assert_eq!(
            stored(&log, &data.root, from_ref(&later)),
            [Ok(Appended::Stored { seq: 2 })]
        );
        // Only the whole stream is refused. An erasure of the household's
        // history is promised like any other, and removes none of its
        // curation: floor 3, tag 1 for the household, scope 2 and the clock.
        let history = Selector {
            stream: Stream::Household,
            scope: Scope::UpTo(Hlc::new(9_999, 0)),
        };
        assert_eq!(
            log.erase(&data.root, history),
            Ok(ErasureReport { erased: vec![] })
        );
        let promised = frame(
            &[
                &3_u64.to_le_bytes()[..],
                &[1, 2],
                &9_999_u64.to_le_bytes(),
                &0_u32.to_le_bytes(),
            ]
            .concat(),
        );
        assert_eq!(bytes(&data.root, &LEDGER), promised);
        let both = [held, record(2, &later)].concat();
        assert_eq!(bytes(&data.root, &path), both);
        // A restart has no promise about the whole stream to keep, and the
        // refusal stands.
        let log = opened(&data.root);
        assert_eq!(
            replayed(&log, &data.root, Stream::Household, 0),
            [stamped(1, &curation), stamped(2, &later)]
        );
        assert_eq!(log.erase(&data.root, whole), Err(LogError::Household));
        assert_eq!(bytes(&data.root, &LEDGER), promised);
        assert_eq!(bytes(&data.root, &path), both);
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
        stored(&log, &data.root, from_ref(&first));
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
        // Opening made the log's directory, with no stream in it yet.
        assert_eq!(
            data.root.log_streams(),
            Ok(Listing {
                entries: vec![],
                foreign: vec![],
            })
        );
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
        // A ledger that promises the household's whole stream, which this
        // version never promises: floor 0, tag 1 for the household, scope 4.
        let planted = data();
        let promise = frame(&[&0_u64.to_le_bytes()[..], &[1], &[4]].concat());
        planted
            .root
            .create_dir(&LEDGER_DIR)
            .expect("the directory is made");
        planted
            .root
            .replace(&LEDGER, &promise)
            .expect("the ledger is written");
        assert_eq!(
            report(&planted.root),
            Err(LogError::Ledger(LedgerFlaw::NotAnEntry {
                range: 0..promise.len()
            }))
        );
        // A record a newer version wrote, after one this version reads.
        let newer = data();
        let log = opened(&newer.root);
        stored(&log, &newer.root, &[play(1, ALICE, 1_000)]);
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
