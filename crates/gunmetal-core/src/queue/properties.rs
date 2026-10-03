//! Properties of the queue over generated documents and operations.
//!
//! Reachable documents are built the way a real queue is: from the empty
//! queue, by applying generated operations and keeping those that are
//! accepted. Garbage documents set every field at random, cursor past the
//! end and repeated entry IDs included, because a stored document is input
//! like any other and the operations must not panic on it.

use proptest::collection::vec;
use proptest::prelude::*;

use super::apply::apply;
use super::document::{
    ContinueLane, Current, Entry, EntryId, FromLane, Lane, Queue, Repeat, Source, StopAfter,
};
use super::fixtures::{album, empty, id, new_entries, playlist, session, track};
use super::op::{Op, Position, QueueReject};
use super::rebase::{Dropped, PendingOp, Rebased, rebase};

/// An entry an operation names: the current entry, the `n`th entry the
/// queue could address (wrapping round), or any ID. The first two make most
/// generated operations hit.
#[derive(Debug, Clone, Copy)]
enum Target {
    Current,
    Queued(usize),
    Any(u8),
}

/// An operation as generated, before it is pointed at a queue.
#[derive(Debug, Clone)]
enum Plan {
    Play(Source, Vec<u8>, usize, u8),
    PlayNext(Source, Vec<u8>),
    Add(Source, Vec<u8>),
    PlayLast(Source, Vec<u8>),
    MoveBefore(Target, Target),
    MoveToEnd(Target, Lane),
    Remove(Vec<Target>),
    ClearUpNext,
    Clear,
    Finished(Target),
    Skip(Target),
    Resume(u8),
    SetRepeat(Repeat),
    SetStopAfter(StopAfter),
}

/// The entries an operation can name: the current one and the upcoming
/// ones, in queue order.
fn addressable(queue: &Queue) -> Vec<EntryId> {
    queue
        .current
        .iter()
        .map(|current| &current.entry)
        .chain(&queue.up_next)
        .chain(&queue.from.upcoming)
        .map(|entry| entry.id)
        .collect()
}

fn resolve_target(target: Target, queue: &Queue) -> EntryId {
    let queued = addressable(queue);
    match target {
        Target::Current => queue.current.map_or(id(0), |current| current.entry.id),
        Target::Queued(n) => n.checked_rem(queued.len()).map_or(id(0), |at| queued[at]),
        Target::Any(n) => id(n),
    }
}

/// The operation `plan` stands for, against `queue`.
fn resolve(plan: &Plan, queue: &Queue) -> Op {
    let target = |target: &Target| resolve_target(*target, queue);
    match plan {
        Plan::Play(source, items, start, by) => Op::Play {
            source: *source,
            items: new_entries(items),
            start: *start,
            by: session(*by),
        },
        Plan::PlayNext(source, items) => Op::PlayNext {
            source: *source,
            items: new_entries(items),
        },
        Plan::Add(source, items) => Op::Add {
            source: *source,
            items: new_entries(items),
        },
        Plan::PlayLast(source, items) => Op::PlayLast {
            source: *source,
            items: new_entries(items),
        },
        Plan::MoveBefore(entry, before) => Op::Move {
            entry: target(entry),
            to: Position::Before(target(before)),
        },
        Plan::MoveToEnd(entry, lane) => Op::Move {
            entry: target(entry),
            to: Position::End(*lane),
        },
        Plan::Remove(entries) => Op::Remove(entries.iter().map(target).collect()),
        Plan::ClearUpNext => Op::ClearUpNext,
        Plan::Clear => Op::Clear,
        Plan::Finished(entry) => Op::Finished(target(entry)),
        Plan::Skip(entry) => Op::Skip(target(entry)),
        Plan::Resume(by) => Op::Resume { by: session(*by) },
        Plan::SetRepeat(repeat) => Op::SetRepeat(*repeat),
        Plan::SetStopAfter(stop_after) => Op::SetStopAfter(*stop_after),
    }
}

fn source() -> impl Strategy<Value = Source> {
    prop_oneof![(0_u8..3).prop_map(album), (0_u8..3).prop_map(playlist)]
}

/// New entry IDs: enough of them that most additions are accepted, few
/// enough that some collide.
fn new_ids() -> impl Strategy<Value = Vec<u8>> {
    vec(0_u8..64, 0..5)
}

fn target() -> impl Strategy<Value = Target> {
    prop_oneof![
        2 => Just(Target::Current),
        3 => (0_usize..64).prop_map(Target::Queued),
        1 => (0_u8..64).prop_map(Target::Any),
    ]
}

fn lane() -> impl Strategy<Value = Lane> {
    prop_oneof![Just(Lane::UpNext), Just(Lane::From)]
}

fn repeat() -> impl Strategy<Value = Repeat> {
    prop_oneof![Just(Repeat::Off), Just(Repeat::One), Just(Repeat::All)]
}

fn stop_after() -> impl Strategy<Value = StopAfter> {
    prop_oneof![
        Just(StopAfter::Off),
        Just(StopAfter::Item),
        Just(StopAfter::Source)
    ]
}

fn plan() -> impl Strategy<Value = Plan> {
    prop_oneof![
        (source(), new_ids(), 0_usize..6, 0_u8..2)
            .prop_map(|(source, items, start, by)| Plan::Play(source, items, start, by)),
        (source(), new_ids()).prop_map(|(source, items)| Plan::PlayNext(source, items)),
        (source(), new_ids()).prop_map(|(source, items)| Plan::Add(source, items)),
        (source(), new_ids()).prop_map(|(source, items)| Plan::PlayLast(source, items)),
        (target(), target()).prop_map(|(entry, before)| Plan::MoveBefore(entry, before)),
        (target(), lane()).prop_map(|(entry, lane)| Plan::MoveToEnd(entry, lane)),
        vec(target(), 0..4).prop_map(Plan::Remove),
        Just(Plan::ClearUpNext),
        Just(Plan::Clear),
        target().prop_map(Plan::Finished),
        target().prop_map(Plan::Skip),
        (0_u8..2).prop_map(Plan::Resume),
        repeat().prop_map(Plan::SetRepeat),
        stop_after().prop_map(Plan::SetStopAfter),
    ]
}

/// Applies `plans` in turn, keeping the queue as it was when one is
/// refused.
fn play_out(start: Queue, plans: &[Plan]) -> Queue {
    plans.iter().fold(start, |queue, plan| {
        apply(&queue, queue.version, &resolve(plan, &queue)).unwrap_or(queue)
    })
}

/// A queue some sequence of accepted operations reaches.
fn reachable() -> impl Strategy<Value = Queue> {
    vec(plan(), 0..24).prop_map(|plans| play_out(empty(), &plans))
}

fn garbage_entry() -> impl Strategy<Value = Entry> {
    (0_u8..8, 0_u8..8, source()).prop_map(|(entry, item, source)| Entry {
        id: id(entry),
        item: track(item),
        source,
    })
}

/// A document with every field set at random.
fn garbage() -> impl Strategy<Value = Queue> {
    let current = proptest::option::of((garbage_entry(), lane()))
        .prop_map(|current| current.map(|(entry, lane)| Current { entry, lane }));
    let lanes = (
        vec(garbage_entry(), 0..5),
        0_usize..8,
        proptest::option::of(source()),
        vec(garbage_entry(), 0..5),
        vec(garbage_entry(), 0..5),
        vec(garbage_entry(), 0..3),
    );
    let modes = (
        repeat(),
        stop_after(),
        proptest::option::of((0_u8..2).prop_map(session)),
    );
    (0_u64..4, current, lanes, modes).prop_map(
        |(
            version,
            current,
            (up_next, cursor, source, before, upcoming, suggested),
            (repeat, stop_after, player),
        )| Queue {
            version,
            current,
            up_next,
            cursor,
            from: FromLane {
                source,
                before,
                upcoming,
            },
            continue_with: ContinueLane { entries: suggested },
            repeat,
            stop_after,
            player,
            ..empty()
        },
    )
}

/// Every entry ID in a queue, sorted, written independently of `apply`.
fn all_ids(queue: &Queue) -> Vec<EntryId> {
    let mut ids: Vec<EntryId> = queue
        .current
        .iter()
        .map(|current| current.entry)
        .chain(queue.up_next.iter().copied())
        .chain(queue.from.before.iter().copied())
        .chain(queue.from.upcoming.iter().copied())
        .chain(queue.continue_with.entries.iter().copied())
        .map(|entry| entry.id)
        .collect();
    ids.sort_unstable();
    ids
}

/// The entry IDs `op` should leave in `queue`, sorted: those it does not
/// mean to remove, and those it adds.
fn expected_ids(queue: &Queue, op: &Op) -> Vec<EntryId> {
    let ids = |entries: &[Entry]| entries.iter().map(|entry| entry.id).collect::<Vec<_>>();
    let current: Vec<EntryId> = queue
        .current
        .iter()
        .map(|current| current.entry.id)
        .collect();
    let current_pick: Vec<EntryId> = queue
        .current
        .iter()
        .filter(|current| current.lane == Lane::UpNext)
        .map(|current| current.entry.id)
        .collect();
    let removed: Vec<EntryId> = match op {
        Op::Play { .. } => [current, ids(&queue.from.before), ids(&queue.from.upcoming)].concat(),
        Op::Remove(entries) => entries.clone(),
        Op::ClearUpNext => ids(&queue.up_next),
        Op::Clear => [
            ids(&queue.up_next),
            ids(&queue.from.before),
            ids(&queue.from.upcoming),
            ids(&queue.continue_with.entries),
        ]
        .concat(),
        Op::Finished(_) if queue.repeat == Repeat::One => vec![],
        Op::Finished(_) | Op::Skip(_) => current_pick,
        _ => vec![],
    };
    let added: Vec<EntryId> = match op {
        Op::Play { items, .. }
        | Op::PlayNext { items, .. }
        | Op::Add { items, .. }
        | Op::PlayLast { items, .. } => items.iter().map(|item| item.id).collect(),
        _ => vec![],
    };
    let mut kept = all_ids(queue);
    for gone in removed {
        let at = kept
            .iter()
            .position(|id| *id == gone)
            .expect("a removed entry was in the queue");
        kept.remove(at);
    }
    kept.extend(added);
    kept.sort_unstable();
    kept
}

proptest! {
    #[test]
    fn every_accepted_operation_adds_exactly_one_to_the_version(
        queue in prop_oneof![reachable(), garbage()],
        plan in plan(),
    ) {
        let op = resolve(&plan, &queue);
        match apply(&queue, queue.version, &op) {
            Ok(next) => prop_assert_eq!(Some(next.version), queue.version.checked_add(1)),
            Err(reason) => prop_assert_ne!(reason, QueueReject::Stale { current: queue.version }),
        }
    }

    #[test]
    fn an_operation_on_any_other_version_is_stale(
        queue in reachable(),
        plan in plan(),
        based_on in any::<u64>(),
    ) {
        prop_assume!(based_on != queue.version);
        prop_assert_eq!(
            apply(&queue, based_on, &resolve(&plan, &queue)),
            Err(QueueReject::Stale { current: queue.version })
        );
    }

    #[test]
    fn no_operation_loses_or_duplicates_an_entry_it_did_not_mean_to_remove(
        queue in reachable(),
        plans in vec(plan(), 12),
    ) {
        // Every case tries each plan, and finishing and skipping the current
        // entry, under each repeat mode, so every kind of operation is
        // accepted in some case of every run.
        let finishing = [Plan::Finished(Target::Current), Plan::Skip(Target::Current)];
        for repeat in [Repeat::Off, Repeat::One, Repeat::All] {
            let queue = Queue { repeat, ..queue.clone() };
            for plan in plans.iter().chain(&finishing) {
                let op = resolve(plan, &queue);
                if let Ok(next) = apply(&queue, queue.version, &op) {
                    prop_assert_eq!(all_ids(&next), expected_ids(&queue, &op), "{:?}", op);
                }
            }
        }
    }

    #[test]
    fn a_reachable_queue_has_unique_entries_and_its_cursor_within_up_next(
        queue in reachable(),
    ) {
        let ids = all_ids(&queue);
        let mut unique = ids.clone();
        unique.dedup();
        prop_assert_eq!(ids, unique);
        prop_assert!(queue.cursor <= queue.up_next.len());
    }

    #[test]
    fn the_same_operation_on_the_same_version_gives_the_same_queue(
        queue in reachable(),
        plan in plan(),
    ) {
        let op = resolve(&plan, &queue);
        prop_assert_eq!(
            apply(&queue, queue.version, &op),
            apply(&queue.clone(), queue.version, &op.clone())
        );
    }

    #[test]
    fn an_operation_applied_again_changes_nothing_or_is_refused(
        queue in reachable(),
        plan in plan(),
    ) {
        let op = resolve(&plan, &queue);
        if let Ok(once) = apply(&queue, queue.version, &op) {
            if let Ok(twice) = apply(&once, once.version, &op) {
                prop_assert_eq!(Queue { version: once.version, ..twice }, once, "{:?}", op);
            }
        }
    }

    #[test]
    fn rebasing_nothing_gives_the_server_queue(server in reachable()) {
        prop_assert_eq!(
            rebase(&[], &server),
            Rebased { queue: server.clone(), pending: vec![], dropped: vec![] }
        );
    }

    #[test]
    fn a_rebased_client_shows_what_the_server_will_hold(
        base in reachable(),
        theirs in vec(plan(), 0..4),
        ours in vec(plan(), 0..4),
    ) {
        // The client applies its own operations optimistically.
        let mut local = base.clone();
        let mut pending = vec![];
        for plan in &ours {
            let op = resolve(plan, &local);
            if let Ok(next) = apply(&local, local.version, &op) {
                pending.push(PendingOp { based_on: local.version, op });
                local = next;
            }
        }
        // Meanwhile the server accepted another device's operations, then
        // takes the client's in order, each on its latest version.
        let server = play_out(base, &theirs);
        let mut held = server.clone();
        let mut accepted = vec![];
        let mut refused = vec![];
        for PendingOp { op, .. } in &pending {
            match apply(&held, held.version, op) {
                Ok(next) => {
                    accepted.push(PendingOp { based_on: held.version, op: op.clone() });
                    held = next;
                }
                Err(reason) => refused.push(Dropped { op: op.clone(), reason }),
            }
        }
        prop_assert_eq!(
            rebase(&pending, &server),
            Rebased { queue: held, pending: accepted, dropped: refused }
        );
    }
}
