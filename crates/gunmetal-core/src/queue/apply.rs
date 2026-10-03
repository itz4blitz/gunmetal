//! Applying an operation to a queue.
//!
//! [`apply`] is a pure function of the document and the operation, so a
//! client's optimistic copy and the server's reach the same document from
//! the same version. It works on a copy and returns it only when the whole
//! operation succeeded.

use std::collections::BTreeSet;
use std::mem;

use crate::id::PublicId;

use super::document::{
    ContinueLane, Current, Entry, EntryId, FromLane, Lane, Queue, Repeat, Source, StopAfter,
};
use super::op::{NewEntry, Op, Position, QueueReject};

/// Applies `op`, built on version `based_on`, to `queue`, and returns the
/// queue at the next version.
///
/// # Errors
///
/// [`QueueReject::Stale`] when `based_on` is not the queue's version, and
/// otherwise the reason the operation does not fit the queue. A refused
/// operation changes nothing.
pub fn apply(queue: &Queue, based_on: u64, op: &Op) -> Result<Queue, QueueReject> {
    if based_on != queue.version {
        return Err(QueueReject::Stale {
            current: queue.version,
        });
    }
    let mut next = queue.clone();
    next.version = queue
        .version
        .checked_add(1)
        .ok_or(QueueReject::VersionExhausted)?;
    match op {
        Op::Play {
            source,
            items,
            start,
            by,
        } => play(&mut next, *source, items, *start, *by)?,
        Op::PlayNext { source, items } => {
            let entries = admit(&next, *source, items)?;
            let at = next.cursor.min(next.up_next.len());
            next.cursor = at.saturating_add(entries.len());
            next.up_next.splice(at..at, entries);
        }
        Op::Add { source, items } => {
            let entries = admit(&next, *source, items)?;
            next.up_next.extend(entries);
        }
        Op::PlayLast { source, items } => {
            let entries = admit(&next, *source, items)?;
            next.from.upcoming.extend(entries);
        }
        Op::Move { entry, to } => {
            let moved = take_upcoming(&mut next, *entry)?;
            put(&mut next, moved, *to)?;
        }
        Op::Remove(entries) => remove(&mut next, entries)?,
        Op::ClearUpNext => {
            next.up_next.clear();
            next.cursor = 0;
        }
        Op::Clear => {
            next.up_next.clear();
            next.cursor = 0;
            next.from = FromLane::default();
            next.continue_with = ContinueLane::default();
        }
        Op::Finished(entry) => finished(&mut next, *entry)?,
        Op::Skip(entry) => {
            let skipped = current(&next, *entry)?;
            move_on(&mut next, skipped);
        }
        Op::Resume { by } => {
            if next.current.is_none() {
                start_next(&mut next, true);
            }
            if next.current.is_none() {
                return Err(QueueReject::NothingToPlay);
            }
            next.player = Some(*by);
        }
        Op::SetRepeat(repeat) => next.repeat = *repeat,
        Op::SetStopAfter(stop_after) => next.stop_after = *stop_after,
    }
    Ok(next)
}

/// Plays `items[start]` now, with `items` as the From lane, for session
/// `by`. Up next is kept.
///
/// # Errors
///
/// As [`admit`], and [`QueueReject::StartOutOfRange`] when `start` is not
/// one of the items.
fn play(
    queue: &mut Queue,
    source: Source,
    items: &[NewEntry],
    start: usize,
    by: PublicId,
) -> Result<(), QueueReject> {
    let entries = admit(queue, source, items)?;
    let Some((before, [current, upcoming @ ..])) = entries.split_at_checked(start) else {
        return Err(QueueReject::StartOutOfRange {
            start,
            items: entries.len(),
        });
    };
    queue.current = Some(Current {
        entry: *current,
        lane: Lane::From,
    });
    queue.from = FromLane {
        source: Some(source),
        before: before.to_vec(),
        upcoming: upcoming.to_vec(),
    };
    queue.cursor = 0;
    queue.position = None;
    queue.player = Some(by);
    Ok(())
}

/// Removes `entries`. Removing the current entry starts the next one and
/// keeps nothing of it, so repeat all does not bring it back.
///
/// # Errors
///
/// [`QueueReject::NoItems`] for an empty list, and
/// [`QueueReject::UnknownEntry`] for the first entry that is neither
/// current nor upcoming by the time its turn comes.
fn remove(queue: &mut Queue, entries: &[EntryId]) -> Result<(), QueueReject> {
    if entries.is_empty() {
        return Err(QueueReject::NoItems);
    }
    for entry in entries {
        if queue
            .current
            .is_some_and(|current| current.entry.id == *entry)
        {
            let wrap = queue.repeat == Repeat::All;
            start_next(queue, wrap);
        } else {
            take_upcoming(queue, *entry)?;
        }
    }
    Ok(())
}

/// The current entry, `entry`, played to its end. Repeat one plays it
/// again; otherwise the next entry starts. Then the stop-after mode may
/// stop playback, which it records by leaving the queue without a player.
///
/// # Errors
///
/// [`QueueReject::NotCurrent`] when `entry` is not the current entry.
fn finished(queue: &mut Queue, entry: EntryId) -> Result<(), QueueReject> {
    let done = current(queue, entry)?;
    let last_of_from = done.lane == Lane::From && queue.from.upcoming.is_empty();
    if queue.repeat == Repeat::One {
        queue.position = None;
    } else {
        move_on(queue, done);
    }
    let stops = match queue.stop_after {
        StopAfter::Off => false,
        StopAfter::Item => true,
        StopAfter::Source => last_of_from,
    };
    if stops {
        queue.stop_after = StopAfter::Off;
        queue.player = None;
    }
    Ok(())
}

/// The current entry, which an operation says is `id`.
///
/// # Errors
///
/// [`QueueReject::NotCurrent`] when it is not.
fn current(queue: &Queue, id: EntryId) -> Result<Current, QueueReject> {
    queue
        .current
        .filter(|current| current.entry.id == id)
        .ok_or(QueueReject::NotCurrent { entry: id })
}

/// Lets `done` go and starts the next entry. An entry from the From lane
/// joins the lane's entries before the current one, so the lane can start
/// again; a pick from Up next has played.
fn move_on(queue: &mut Queue, done: Current) {
    if done.lane == Lane::From {
        queue.from.before.push(done.entry);
    }
    let wrap = queue.repeat == Repeat::All;
    start_next(queue, wrap);
}

/// Makes the next entry current: the first pick, or else the From lane's
/// next entry. When the From lane has run out and `wrap` is set, the lane
/// starts again from its first entry. With nothing to play the queue has no
/// current entry.
fn start_next(queue: &mut Queue, wrap: bool) {
    queue.cursor = 0;
    queue.position = None;
    queue.current = take_first(&mut queue.up_next)
        .map(|entry| Current {
            entry,
            lane: Lane::UpNext,
        })
        .or_else(|| {
            if wrap && queue.from.upcoming.is_empty() {
                queue.from.upcoming = mem::take(&mut queue.from.before);
            }
            take_first(&mut queue.from.upcoming).map(|entry| Current {
                entry,
                lane: Lane::From,
            })
        });
}

/// Takes the first of `entries`, if there is one.
fn take_first(entries: &mut Vec<Entry>) -> Option<Entry> {
    (!entries.is_empty()).then(|| entries.remove(0))
}

/// Takes the upcoming entry `id` out of its lane.
///
/// # Errors
///
/// [`QueueReject::UnknownEntry`] when no upcoming entry has that ID.
fn take_upcoming(queue: &mut Queue, id: EntryId) -> Result<Entry, QueueReject> {
    if let Some(at) = queue.up_next.iter().position(|entry| entry.id == id) {
        if at < queue.cursor {
            queue.cursor = queue.cursor.saturating_sub(1);
        }
        Ok(queue.up_next.remove(at))
    } else if let Some(at) = queue.from.upcoming.iter().position(|entry| entry.id == id) {
        Ok(queue.from.upcoming.remove(at))
    } else {
        Err(QueueReject::UnknownEntry { entry: id })
    }
}

/// Puts `entry` at `to`. An entry put at the insertion cursor lands after
/// it.
///
/// # Errors
///
/// [`QueueReject::UnknownEntry`] when `to` is before an entry that is not
/// upcoming.
fn put(queue: &mut Queue, entry: Entry, to: Position) -> Result<(), QueueReject> {
    match to {
        Position::End(Lane::UpNext) => queue.up_next.push(entry),
        Position::End(Lane::From) => queue.from.upcoming.push(entry),
        Position::Before(target) => {
            if let Some(at) = queue.up_next.iter().position(|e| e.id == target) {
                if at < queue.cursor {
                    queue.cursor = queue.cursor.saturating_add(1);
                }
                queue.up_next.insert(at, entry);
            } else if let Some(at) = queue.from.upcoming.iter().position(|e| e.id == target) {
                queue.from.upcoming.insert(at, entry);
            } else {
                return Err(QueueReject::UnknownEntry { entry: target });
            }
        }
    }
    Ok(())
}

/// The entries `items` become, chosen from `source`.
///
/// # Errors
///
/// [`QueueReject::NoItems`] for an empty list, and
/// [`QueueReject::DuplicateEntry`] for the first new ID already in `queue`
/// or earlier in `items`.
fn admit(queue: &Queue, source: Source, items: &[NewEntry]) -> Result<Vec<Entry>, QueueReject> {
    if items.is_empty() {
        return Err(QueueReject::NoItems);
    }
    let mut taken: BTreeSet<_> = queue
        .current
        .iter()
        .map(|current| &current.entry)
        .chain(&queue.up_next)
        .chain(&queue.from.before)
        .chain(&queue.from.upcoming)
        .chain(&queue.continue_with.entries)
        .map(|entry| entry.id)
        .collect();
    items
        .iter()
        .map(|new| {
            if taken.insert(new.id) {
                Ok(Entry {
                    id: new.id,
                    item: new.item,
                    source,
                })
            } else {
                Err(QueueReject::DuplicateEntry { entry: new.id })
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::document::{Current, FromLane, Lane};
    use crate::queue::fixtures::{
        album, empty, entries, entry, id, new_entries, playlist, session, track,
    };
    use crate::queue::op::{NewEntry, Position};

    /// A queue in the middle of an album, with a pick waiting, written out.
    fn playing() -> Queue {
        Queue {
            version: 7,
            current: Some(Current {
                entry: entry(9, album(2)),
                lane: Lane::From,
            }),
            up_next: entries(&[5], playlist(1)),
            cursor: 1,
            from: FromLane {
                source: Some(album(2)),
                before: entries(&[8], album(2)),
                upcoming: entries(&[10], album(2)),
            },
            player: Some(session(1)),
            ..empty()
        }
    }

    fn play(numbers: &[u8], start: usize, by: u8) -> Op {
        Op::Play {
            source: album(1),
            items: new_entries(numbers),
            start,
            by: session(by),
        }
    }

    #[test]
    fn play_starts_the_chosen_item_with_its_source_around_it() {
        assert_eq!(
            apply(&empty(), 0, &play(&[1, 2, 3, 4], 1, 1)),
            Ok(Queue {
                version: 1,
                current: Some(Current {
                    entry: entry(2, album(1)),
                    lane: Lane::From,
                }),
                from: FromLane {
                    source: Some(album(1)),
                    before: entries(&[1], album(1)),
                    upcoming: entries(&[3, 4], album(1)),
                },
                player: Some(session(1)),
                ..empty()
            })
        );
    }

    #[test]
    fn play_keeps_the_picks_replaces_the_from_lane_and_takes_the_queue() {
        let mut queue = playing();
        queue.position = Some(crate::values::Duration::from_millis(1_500).unwrap());
        assert_eq!(
            apply(&queue, 7, &play(&[1, 2], 0, 2)),
            Ok(Queue {
                version: 8,
                current: Some(Current {
                    entry: entry(1, album(1)),
                    lane: Lane::From,
                }),
                up_next: entries(&[5], playlist(1)),
                cursor: 0,
                from: FromLane {
                    source: Some(album(1)),
                    before: vec![],
                    upcoming: entries(&[2], album(1)),
                },
                player: Some(session(2)),
                ..empty()
            })
        );
    }

    #[test]
    fn play_starts_at_the_last_item() {
        assert_eq!(
            apply(&empty(), 0, &play(&[1, 2, 3], 2, 1)),
            Ok(Queue {
                version: 1,
                current: Some(Current {
                    entry: entry(3, album(1)),
                    lane: Lane::From,
                }),
                from: FromLane {
                    source: Some(album(1)),
                    before: entries(&[1, 2], album(1)),
                    upcoming: vec![],
                },
                player: Some(session(1)),
                ..empty()
            })
        );
    }

    #[test]
    fn play_refuses_a_start_that_is_not_one_of_its_items() {
        assert_eq!(
            apply(&empty(), 0, &play(&[1, 2], 2, 1)),
            Err(QueueReject::StartOutOfRange { start: 2, items: 2 })
        );
        assert_eq!(
            apply(&empty(), 0, &play(&[1, 2], 9, 1)),
            Err(QueueReject::StartOutOfRange { start: 9, items: 2 })
        );
    }

    #[test]
    fn play_refuses_an_empty_list() {
        assert_eq!(
            apply(&empty(), 0, &play(&[], 0, 1)),
            Err(QueueReject::NoItems)
        );
    }

    #[test]
    fn a_new_entry_may_not_reuse_an_id_in_the_operation() {
        assert_eq!(
            apply(&empty(), 0, &play(&[1, 2, 1], 0, 1)),
            Err(QueueReject::DuplicateEntry { entry: id(1) })
        );
    }

    #[test]
    fn a_new_entry_may_not_reuse_an_id_anywhere_in_the_queue() {
        let mut queue = playing();
        queue.continue_with.entries = entries(&[11], album(3));
        for taken in [9, 5, 8, 10, 11] {
            assert_eq!(
                apply(&queue, 7, &play(&[1, taken], 0, 1)),
                Err(QueueReject::DuplicateEntry { entry: id(taken) }),
                "entry {taken}"
            );
        }
    }

    #[test]
    fn a_new_entry_may_repeat_an_item_under_a_new_id() {
        let items = vec![
            NewEntry {
                id: id(1),
                item: track(7),
            },
            NewEntry {
                id: id(2),
                item: track(7),
            },
        ];
        let played = apply(
            &empty(),
            0,
            &Op::Play {
                source: album(1),
                items,
                start: 0,
                by: session(1),
            },
        );
        let copy = |n| Entry {
            id: id(n),
            item: track(7),
            source: album(1),
        };
        assert_eq!(
            played,
            Ok(Queue {
                version: 1,
                current: Some(Current {
                    entry: copy(1),
                    lane: Lane::From,
                }),
                from: FromLane {
                    source: Some(album(1)),
                    before: vec![],
                    upcoming: vec![copy(2)],
                },
                player: Some(session(1)),
                ..empty()
            })
        );
    }

    /// Applies each operation in turn, each built on the version before.
    fn apply_all(queue: &Queue, ops: &[Op]) -> Result<Queue, QueueReject> {
        ops.iter()
            .try_fold(queue.clone(), |queue, op| apply(&queue, queue.version, op))
    }

    fn play_next(numbers: &[u8]) -> Op {
        Op::PlayNext {
            source: playlist(2),
            items: new_entries(numbers),
        }
    }

    fn add(numbers: &[u8]) -> Op {
        Op::Add {
            source: playlist(2),
            items: new_entries(numbers),
        }
    }

    fn play_last(numbers: &[u8]) -> Op {
        Op::PlayLast {
            source: playlist(2),
            items: new_entries(numbers),
        }
    }

    /// A queue with one added pick and no "play next" picks yet.
    fn with_an_added_pick() -> Queue {
        Queue {
            up_next: entries(&[6], playlist(1)),
            cursor: 0,
            ..playing()
        }
    }

    #[test]
    fn three_play_next_picks_play_in_the_order_chosen() {
        let picked = apply_all(
            &with_an_added_pick(),
            &[play_next(&[1]), play_next(&[2]), play_next(&[3])],
        );
        assert_eq!(
            picked,
            Ok(Queue {
                version: 10,
                up_next: [entries(&[1, 2, 3], playlist(2)), entries(&[6], playlist(1))].concat(),
                cursor: 3,
                ..with_an_added_pick()
            })
        );
    }

    #[test]
    fn play_next_with_several_items_keeps_their_order() {
        assert_eq!(
            apply(&with_an_added_pick(), 7, &play_next(&[1, 2, 3])),
            Ok(Queue {
                version: 8,
                up_next: [entries(&[1, 2, 3], playlist(2)), entries(&[6], playlist(1))].concat(),
                cursor: 3,
                ..with_an_added_pick()
            })
        );
    }

    #[test]
    fn add_appends_to_up_next_and_leaves_the_cursor() {
        let added = apply_all(&playing(), &[add(&[1, 2]), play_next(&[3])]);
        assert_eq!(
            added,
            Ok(Queue {
                version: 9,
                up_next: [entries(&[5], playlist(1)), entries(&[3, 1, 2], playlist(2)),].concat(),
                cursor: 2,
                ..playing()
            })
        );
    }

    #[test]
    fn play_last_appends_after_the_from_lane() {
        let mut expected = playing();
        expected.version = 8;
        expected.from.upcoming.extend(entries(&[1, 2], playlist(2)));
        assert_eq!(apply(&playing(), 7, &play_last(&[1, 2])), Ok(expected));
    }

    #[test]
    fn picks_wait_in_an_idle_queue() {
        let picked = apply_all(&empty(), &[add(&[1]), play_next(&[2]), play_last(&[3])]);
        assert_eq!(
            picked,
            Ok(Queue {
                version: 3,
                up_next: entries(&[2, 1], playlist(2)),
                cursor: 1,
                from: FromLane {
                    source: None,
                    before: vec![],
                    upcoming: entries(&[3], playlist(2)),
                },
                ..empty()
            })
        );
    }

    #[test]
    fn play_next_after_a_cursor_past_the_end_goes_to_the_end() {
        let queue = Queue {
            cursor: 9,
            ..playing()
        };
        assert_eq!(
            apply(&queue, 7, &play_next(&[1])),
            Ok(Queue {
                version: 8,
                up_next: [entries(&[5], playlist(1)), entries(&[1], playlist(2))].concat(),
                cursor: 2,
                ..playing()
            })
        );
    }

    #[test]
    fn adding_verbs_refuse_an_empty_list_and_a_taken_id() {
        for verb in [play_next, add, play_last] {
            assert_eq!(apply(&playing(), 7, &verb(&[])), Err(QueueReject::NoItems));
            assert_eq!(
                apply(&playing(), 7, &verb(&[1, 9])),
                Err(QueueReject::DuplicateEntry { entry: id(9) })
            );
        }
    }

    /// Three "play next" picks, two added picks and two more of the album.
    fn picks() -> Queue {
        Queue {
            up_next: entries(&[1, 2, 3, 4, 5], playlist(2)),
            cursor: 3,
            from: FromLane {
                source: Some(album(2)),
                before: entries(&[8], album(2)),
                upcoming: entries(&[10, 11], album(2)),
            },
            ..playing()
        }
    }

    fn moving(entry: u8, to: Position) -> Op {
        Op::Move {
            entry: id(entry),
            to,
        }
    }

    #[test]
    fn moving_within_up_next_keeps_the_cursor_after_the_play_next_picks() {
        let cases = [
            (moving(1, Position::Before(id(5))), [2, 3, 4, 1, 5], 2),
            (moving(5, Position::Before(id(2))), [1, 5, 2, 3, 4], 4),
            (moving(4, Position::Before(id(1))), [4, 1, 2, 3, 5], 4),
            (moving(3, Position::End(Lane::UpNext)), [1, 2, 4, 5, 3], 2),
            // Moved to the cursor itself, an entry lands after it.
            (moving(2, Position::Before(id(4))), [1, 3, 2, 4, 5], 2),
            (moving(4, Position::Before(id(5))), [1, 2, 3, 4, 5], 3),
        ];
        for (op, order, cursor) in cases {
            assert_eq!(
                apply(&picks(), 7, &op),
                Ok(Queue {
                    version: 8,
                    up_next: entries(&order, playlist(2)),
                    cursor,
                    ..picks()
                }),
                "{op:?}"
            );
        }
    }

    #[test]
    fn an_entry_moved_to_another_lane_keeps_its_own_source() {
        let moved = apply_all(
            &picks(),
            &[
                moving(1, Position::Before(id(11))),
                moving(10, Position::End(Lane::UpNext)),
                moving(11, Position::Before(id(2))),
                moving(5, Position::End(Lane::From)),
            ],
        );
        assert_eq!(
            moved,
            Ok(Queue {
                version: 11,
                up_next: vec![
                    entry(11, album(2)),
                    entry(2, playlist(2)),
                    entry(3, playlist(2)),
                    entry(4, playlist(2)),
                    entry(10, album(2)),
                ],
                cursor: 3,
                from: FromLane {
                    source: Some(album(2)),
                    before: entries(&[8], album(2)),
                    upcoming: entries(&[1, 5], playlist(2)),
                },
                ..picks()
            })
        );
    }

    #[test]
    fn only_upcoming_entries_move() {
        let mut queue = picks();
        queue.continue_with.entries = entries(&[12], album(3));
        let refused = [
            // Not in the queue.
            (moving(99, Position::End(Lane::UpNext)), 99),
            // Playing now, before the current entry, and a suggestion.
            (moving(9, Position::End(Lane::UpNext)), 9),
            (moving(8, Position::End(Lane::From)), 8),
            (moving(12, Position::End(Lane::From)), 12),
            // Before an entry that is not upcoming, or before itself.
            (moving(1, Position::Before(id(99))), 99),
            (moving(1, Position::Before(id(9))), 9),
            (moving(1, Position::Before(id(1))), 1),
        ];
        for (op, entry) in refused {
            assert_eq!(
                apply(&queue, 7, &op),
                Err(QueueReject::UnknownEntry { entry: id(entry) }),
                "{op:?}"
            );
        }
    }

    /// Entries removed, then Up next, the cursor and the From lane's
    /// upcoming entries afterwards.
    type Removal = (&'static [u8], &'static [u8], usize, &'static [u8]);

    #[test]
    fn removing_entries_keeps_the_cursor_after_the_play_next_picks() {
        let cases: [Removal; 4] = [
            (&[2], &[1, 3, 4, 5], 2, &[10, 11]),
            (&[3], &[1, 2, 4, 5], 2, &[10, 11]),
            (&[4], &[1, 2, 3, 5], 3, &[10, 11]),
            (&[10, 1, 5], &[2, 3, 4], 2, &[11]),
        ];
        for (removed, order, cursor, upcoming) in cases {
            let mut expected = Queue {
                version: 8,
                up_next: entries(order, playlist(2)),
                cursor,
                ..picks()
            };
            expected.from.upcoming = entries(upcoming, album(2));
            assert_eq!(
                apply(
                    &picks(),
                    7,
                    &Op::Remove(removed.iter().map(|&n| id(n)).collect())
                ),
                Ok(expected),
                "{removed:?}"
            );
        }
    }

    #[test]
    fn a_removal_is_refused_whole_when_one_entry_is_not_upcoming() {
        let refused: [(&[u8], QueueReject); 4] = [
            (&[], QueueReject::NoItems),
            (&[1, 99], QueueReject::UnknownEntry { entry: id(99) }),
            (&[1, 8], QueueReject::UnknownEntry { entry: id(8) }),
            (&[1, 1], QueueReject::UnknownEntry { entry: id(1) }),
        ];
        for (removed, reason) in refused {
            assert_eq!(
                apply(
                    &picks(),
                    7,
                    &Op::Remove(removed.iter().map(|&n| id(n)).collect())
                ),
                Err(reason),
                "{removed:?}"
            );
        }
    }

    #[test]
    fn clear_up_next_removes_only_the_picks() {
        assert_eq!(
            apply(&picks(), 7, &Op::ClearUpNext),
            Ok(Queue {
                version: 8,
                up_next: vec![],
                cursor: 0,
                ..picks()
            })
        );
    }

    #[test]
    fn clear_empties_every_lane_and_keeps_the_item_playing() {
        let mut queue = picks();
        queue.continue_with.entries = entries(&[12], album(3));
        queue.position = Some(crate::values::Duration::from_millis(61_000).unwrap());
        assert_eq!(
            apply(&queue, 7, &Op::Clear),
            Ok(Queue {
                version: 8,
                current: Some(Current {
                    entry: entry(9, album(2)),
                    lane: Lane::From,
                }),
                position: Some(crate::values::Duration::from_millis(61_000).unwrap()),
                player: Some(session(1)),
                ..empty()
            })
        );
    }

    /// The album's last entry playing, with nothing picked.
    fn last_of_album() -> Queue {
        Queue {
            up_next: vec![],
            cursor: 0,
            from: FromLane {
                source: Some(album(2)),
                before: entries(&[8], album(2)),
                upcoming: vec![],
            },
            ..playing()
        }
    }

    /// Entry `n` of album 2, playing from the From lane.
    fn from_lane(n: u8) -> Current {
        Current {
            entry: entry(n, album(2)),
            lane: Lane::From,
        }
    }

    /// Pick `n`, from playlist 2, playing from Up next.
    fn pick(n: u8) -> Current {
        Current {
            entry: entry(n, playlist(2)),
            lane: Lane::UpNext,
        }
    }

    fn two_seconds_in() -> crate::values::Duration {
        crate::values::Duration::from_millis(2_000).unwrap()
    }

    #[test]
    fn a_finished_item_gives_way_to_the_picks_then_the_from_lane() {
        let mut queue = picks();
        queue.position = Some(two_seconds_in());
        let after_one = Queue {
            version: 8,
            current: Some(pick(1)),
            position: None,
            up_next: entries(&[2, 3, 4, 5], playlist(2)),
            cursor: 0,
            from: FromLane {
                source: Some(album(2)),
                before: entries(&[8, 9], album(2)),
                upcoming: entries(&[10, 11], album(2)),
            },
            ..picks()
        };
        assert_eq!(
            apply(&queue, 7, &Op::Finished(id(9))),
            Ok(after_one.clone())
        );
        // A pick that has played is done; the From lane keeps its place.
        assert_eq!(
            apply(&after_one, 8, &Op::Finished(id(1))),
            Ok(Queue {
                version: 9,
                current: Some(pick(2)),
                up_next: entries(&[3, 4, 5], playlist(2)),
                ..after_one.clone()
            })
        );
        let only_the_album = Queue {
            up_next: vec![],
            cursor: 0,
            ..queue
        };
        assert_eq!(
            apply(&only_the_album, 7, &Op::Finished(id(9))),
            Ok(Queue {
                version: 8,
                current: Some(from_lane(10)),
                position: None,
                from: FromLane {
                    source: Some(album(2)),
                    before: entries(&[8, 9], album(2)),
                    upcoming: entries(&[11], album(2)),
                },
                ..only_the_album.clone()
            })
        );
    }

    #[test]
    fn the_cursor_resets_when_the_current_item_changes() {
        assert_eq!(
            apply_all(&picks(), &[Op::Finished(id(9)), play_next(&[20])]),
            Ok(Queue {
                version: 9,
                current: Some(pick(1)),
                up_next: entries(&[20, 2, 3, 4, 5], playlist(2)),
                cursor: 1,
                from: FromLane {
                    source: Some(album(2)),
                    before: entries(&[8, 9], album(2)),
                    upcoming: entries(&[10, 11], album(2)),
                },
                ..picks()
            })
        );
    }

    #[test]
    fn each_mode_at_the_end_of_the_from_lane() {
        use crate::queue::document::{Repeat, StopAfter};
        let ended = |before: &[u8]| FromLane {
            source: Some(album(2)),
            before: entries(before, album(2)),
            upcoming: vec![],
        };
        let wrapped = FromLane {
            source: Some(album(2)),
            before: vec![],
            upcoming: entries(&[9], album(2)),
        };
        let playing_on = Some(session(1));
        let cases = [
            (
                Repeat::Off,
                StopAfter::Off,
                None,
                ended(&[8, 9]),
                StopAfter::Off,
                playing_on,
            ),
            (
                Repeat::One,
                StopAfter::Off,
                Some(from_lane(9)),
                ended(&[8]),
                StopAfter::Off,
                playing_on,
            ),
            (
                Repeat::All,
                StopAfter::Off,
                Some(from_lane(8)),
                wrapped.clone(),
                StopAfter::Off,
                playing_on,
            ),
            (
                Repeat::Off,
                StopAfter::Item,
                None,
                ended(&[8, 9]),
                StopAfter::Off,
                None,
            ),
            (
                Repeat::Off,
                StopAfter::Source,
                None,
                ended(&[8, 9]),
                StopAfter::Off,
                None,
            ),
            (
                Repeat::One,
                StopAfter::Item,
                Some(from_lane(9)),
                ended(&[8]),
                StopAfter::Off,
                None,
            ),
            (
                Repeat::All,
                StopAfter::Source,
                Some(from_lane(8)),
                wrapped,
                StopAfter::Off,
                None,
            ),
        ];
        for (repeat, stop_after, current, from, stop_after_then, player) in cases {
            let queue = Queue {
                repeat,
                stop_after,
                position: Some(two_seconds_in()),
                ..last_of_album()
            };
            assert_eq!(
                apply(&queue, 7, &Op::Finished(id(9))),
                Ok(Queue {
                    version: 8,
                    current,
                    position: None,
                    from,
                    stop_after: stop_after_then,
                    player,
                    ..queue.clone()
                }),
                "{repeat:?}, {stop_after:?}"
            );
        }
    }

    #[test]
    fn repeat_one_keeps_the_cursor_and_skipping_moves_on() {
        use crate::queue::document::Repeat;
        let queue = Queue {
            repeat: Repeat::One,
            ..picks()
        };
        assert_eq!(
            apply(&queue, 7, &Op::Finished(id(9))),
            Ok(Queue {
                version: 8,
                ..queue.clone()
            })
        );
        assert_eq!(
            apply(&queue, 7, &Op::Skip(id(9))),
            Ok(Queue {
                version: 8,
                current: Some(pick(1)),
                up_next: entries(&[2, 3, 4, 5], playlist(2)),
                cursor: 0,
                from: FromLane {
                    source: Some(album(2)),
                    before: entries(&[8, 9], album(2)),
                    upcoming: entries(&[10, 11], album(2)),
                },
                ..queue.clone()
            })
        );
    }

    #[test]
    fn repeat_all_waits_for_the_picks_and_the_rest_of_the_lane() {
        use crate::queue::document::Repeat;
        let with_a_pick = Queue {
            repeat: Repeat::All,
            up_next: entries(&[1], playlist(2)),
            ..last_of_album()
        };
        let after_the_album = Queue {
            version: 8,
            current: Some(pick(1)),
            up_next: vec![],
            from: FromLane {
                source: Some(album(2)),
                before: entries(&[8, 9], album(2)),
                upcoming: vec![],
            },
            ..with_a_pick.clone()
        };
        assert_eq!(
            apply(&with_a_pick, 7, &Op::Finished(id(9))),
            Ok(after_the_album.clone())
        );
        assert_eq!(
            apply(&after_the_album, 8, &Op::Finished(id(1))),
            Ok(Queue {
                version: 9,
                current: Some(from_lane(8)),
                from: FromLane {
                    source: Some(album(2)),
                    before: vec![],
                    upcoming: entries(&[9], album(2)),
                },
                ..after_the_album.clone()
            })
        );
        let mid_album = Queue {
            repeat: Repeat::All,
            up_next: vec![],
            cursor: 0,
            ..picks()
        };
        assert_eq!(
            apply(&mid_album, 7, &Op::Finished(id(9))),
            Ok(Queue {
                version: 8,
                current: Some(from_lane(10)),
                from: FromLane {
                    source: Some(album(2)),
                    before: entries(&[8, 9], album(2)),
                    upcoming: entries(&[11], album(2)),
                },
                ..mid_album.clone()
            })
        );
    }

    #[test]
    fn stop_after_the_source_waits_for_the_from_lane_to_end() {
        use crate::queue::document::StopAfter;
        let mid_album = Queue {
            stop_after: StopAfter::Source,
            up_next: vec![],
            cursor: 0,
            ..picks()
        };
        assert_eq!(
            apply(&mid_album, 7, &Op::Finished(id(9))),
            Ok(Queue {
                version: 8,
                current: Some(from_lane(10)),
                from: FromLane {
                    source: Some(album(2)),
                    before: entries(&[8, 9], album(2)),
                    upcoming: entries(&[11], album(2)),
                },
                ..mid_album.clone()
            })
        );
        // A pick that ends after the album has ended does not stop it either.
        let pick_after_the_album = Queue {
            stop_after: StopAfter::Source,
            current: Some(pick(1)),
            ..last_of_album()
        };
        assert_eq!(
            apply(&pick_after_the_album, 7, &Op::Finished(id(1))),
            Ok(Queue {
                version: 8,
                current: None,
                ..pick_after_the_album.clone()
            })
        );
        // The album's last entry stops it even with a pick still to come.
        let last_with_a_pick = Queue {
            stop_after: StopAfter::Source,
            up_next: entries(&[1], playlist(2)),
            ..last_of_album()
        };
        assert_eq!(
            apply(&last_with_a_pick, 7, &Op::Finished(id(9))),
            Ok(Queue {
                version: 8,
                current: Some(pick(1)),
                up_next: vec![],
                from: FromLane {
                    source: Some(album(2)),
                    before: entries(&[8, 9], album(2)),
                    upcoming: vec![],
                },
                stop_after: StopAfter::Off,
                player: None,
                ..last_with_a_pick.clone()
            })
        );
    }

    #[test]
    fn skipping_never_stops_playback() {
        use crate::queue::document::{Repeat, StopAfter};
        for stop_after in [StopAfter::Item, StopAfter::Source] {
            let queue = Queue {
                stop_after,
                repeat: Repeat::All,
                ..last_of_album()
            };
            assert_eq!(
                apply(&queue, 7, &Op::Skip(id(9))),
                Ok(Queue {
                    version: 8,
                    current: Some(from_lane(8)),
                    from: FromLane {
                        source: Some(album(2)),
                        before: vec![],
                        upcoming: entries(&[9], album(2)),
                    },
                    ..queue.clone()
                }),
                "{stop_after:?}"
            );
        }
    }

    #[test]
    fn only_the_current_item_can_finish_or_be_skipped() {
        for (op, entry) in [(Op::Finished(id(10)), id(10)), (Op::Skip(id(1)), id(1))] {
            assert_eq!(
                apply(&picks(), 7, &op),
                Err(QueueReject::NotCurrent { entry }),
                "{op:?}"
            );
            assert_eq!(
                apply(&empty(), 0, &op),
                Err(QueueReject::NotCurrent { entry })
            );
        }
    }

    #[test]
    fn removing_the_current_item_plays_the_next_without_keeping_it() {
        let expected = Queue {
            version: 8,
            current: Some(pick(2)),
            position: None,
            up_next: entries(&[3, 4, 5], playlist(2)),
            cursor: 0,
            ..picks()
        };
        let mut queue = picks();
        queue.position = Some(two_seconds_in());
        for removed in [[9, 1], [1, 9]] {
            assert_eq!(
                apply(
                    &queue,
                    7,
                    &Op::Remove(removed.iter().map(|&n| id(n)).collect())
                ),
                Ok(expected.clone()),
                "{removed:?}"
            );
        }
        let looping = Queue {
            repeat: crate::queue::document::Repeat::All,
            ..last_of_album()
        };
        assert_eq!(
            apply(&looping, 7, &Op::Remove(vec![id(9)])),
            Ok(Queue {
                version: 8,
                current: Some(from_lane(8)),
                from: FromLane {
                    source: Some(album(2)),
                    before: vec![],
                    upcoming: vec![],
                },
                ..looping.clone()
            })
        );
    }

    #[test]
    fn the_last_session_to_play_or_resume_takes_the_queue() {
        let mut queue = playing();
        queue.position = Some(two_seconds_in());
        assert_eq!(
            apply(&queue, 7, &Op::Resume { by: session(2) }),
            Ok(Queue {
                version: 8,
                player: Some(session(2)),
                ..queue.clone()
            })
        );
    }

    #[test]
    fn resuming_an_idle_queue_starts_its_first_pick() {
        assert_eq!(
            apply_all(&empty(), &[add(&[1, 2]), Op::Resume { by: session(1) }]),
            Ok(Queue {
                version: 2,
                current: Some(pick(1)),
                up_next: entries(&[2], playlist(2)),
                player: Some(session(1)),
                ..empty()
            })
        );
    }

    #[test]
    fn resuming_a_finished_queue_plays_the_from_lane_again() {
        let ended = Queue {
            current: None,
            position: None,
            from: FromLane {
                source: Some(album(2)),
                before: entries(&[8, 9], album(2)),
                upcoming: vec![],
            },
            player: None,
            ..last_of_album()
        };
        assert_eq!(
            apply(&ended, 7, &Op::Resume { by: session(2) }),
            Ok(Queue {
                version: 8,
                current: Some(from_lane(8)),
                from: FromLane {
                    source: Some(album(2)),
                    before: vec![],
                    upcoming: entries(&[9], album(2)),
                },
                player: Some(session(2)),
                ..ended.clone()
            })
        );
    }

    #[test]
    fn resuming_with_nothing_queued_is_refused() {
        assert_eq!(
            apply(&empty(), 0, &Op::Resume { by: session(1) }),
            Err(QueueReject::NothingToPlay)
        );
    }

    #[test]
    fn the_modes_are_set_by_operations() {
        use crate::queue::document::{Repeat, StopAfter};
        assert_eq!(
            apply_all(
                &playing(),
                &[
                    Op::SetRepeat(Repeat::All),
                    Op::SetStopAfter(StopAfter::Source)
                ]
            ),
            Ok(Queue {
                version: 9,
                repeat: Repeat::All,
                stop_after: StopAfter::Source,
                ..playing()
            })
        );
        assert_eq!(
            apply_all(
                &playing(),
                &[
                    Op::SetRepeat(Repeat::One),
                    Op::SetStopAfter(StopAfter::Item)
                ]
            ),
            Ok(Queue {
                version: 9,
                repeat: Repeat::One,
                stop_after: StopAfter::Item,
                ..playing()
            })
        );
    }

    #[test]
    fn an_operation_built_on_another_version_is_stale() {
        for based_on in [0, 6, 8, u64::MAX] {
            assert_eq!(
                apply(&playing(), based_on, &play(&[1], 0, 1)),
                Err(QueueReject::Stale { current: 7 }),
                "based on {based_on}"
            );
        }
    }

    #[test]
    fn the_last_version_is_never_passed() {
        let queue = Queue {
            version: u64::MAX,
            ..empty()
        };
        assert_eq!(
            apply(&queue, u64::MAX, &play(&[1], 0, 1)),
            Err(QueueReject::VersionExhausted)
        );
    }
}
