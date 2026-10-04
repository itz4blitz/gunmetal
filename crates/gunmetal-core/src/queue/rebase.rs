//! Rebuilding a client's pending operations on the server's version.
//!
//! A client applies its operations to its own copy at once and sends them,
//! each built on the version before it. When the server refuses one as
//! stale, or pushes a newer version, the client calls [`rebase`] with the
//! operations the server has not acknowledged. It applies them in order to
//! the server's queue with the server's own [`apply`], so what the client
//! then shows is exactly what the server will hold once it accepts them.
//!
//! An operation whose acknowledgement was lost may already be in the
//! server's queue. Re-applying it is safe: an operation that adds entries
//! is refused because their IDs are taken, one that names the current or
//! an upcoming entry is refused once that entry has moved on or gone, and
//! the rest give the same document again.

use super::apply::apply;
use super::document::Queue;
use super::op::{Op, QueueReject};

/// An operation the client has applied to its own copy and sent, or will
/// send, built on `based_on`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingOp {
    /// The version the operation was built on.
    pub based_on: u64,
    /// The operation.
    pub op: Op,
}

/// A pending operation that no longer fits the server's queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dropped {
    /// The operation.
    pub op: Op,
    /// Why it no longer fits.
    pub reason: QueueReject,
}

/// The client's queue after a rebase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rebased {
    /// The server's queue with the pending operations that still fit
    /// applied: what the client shows.
    pub queue: Queue,
    /// Those operations, each built on the version before it, to send.
    pub pending: Vec<PendingOp>,
    /// The operations that no longer fit, in order, for the client to say so.
    pub dropped: Vec<Dropped>,
}

/// Rebuilds `pending` on `server`, in order. Each operation's old
/// `based_on` is ignored: it is built on the version the operations before
/// it reach. With nothing pending the result is the server's queue.
#[must_use]
pub fn rebase(pending: &[PendingOp], server: &Queue) -> Rebased {
    let start = Rebased {
        queue: server.clone(),
        pending: Vec::new(),
        dropped: Vec::new(),
    };
    pending
        .iter()
        .fold(start, |mut rebased, PendingOp { op, .. }| {
            let based_on = rebased.queue.version;
            match apply(&rebased.queue, based_on, op) {
                Ok(queue) => {
                    rebased.queue = queue;
                    rebased.pending.push(PendingOp {
                        based_on,
                        op: op.clone(),
                    });
                }
                Err(reason) => rebased.dropped.push(Dropped {
                    op: op.clone(),
                    reason,
                }),
            }
            rebased
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::document::{Current, FromLane, Lane};
    use crate::queue::fixtures::{
        album, empty, entries, entry, id, new_entries, playlist, session,
    };
    use crate::queue::op::Position;

    /// The server's queue: entry 9 of album 2 playing, pick 5 waiting.
    fn server() -> Queue {
        Queue {
            version: 8,
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

    fn add(numbers: &[u8]) -> Op {
        Op::Add {
            source: playlist(2),
            items: new_entries(numbers),
        }
    }

    fn pending(based_on: u64, op: Op) -> PendingOp {
        PendingOp { based_on, op }
    }

    #[test]
    fn rebasing_nothing_gives_the_server_queue() {
        assert_eq!(
            rebase(&[], &server()),
            Rebased {
                queue: server(),
                pending: vec![],
                dropped: vec![],
            }
        );
    }

    #[test]
    fn pending_operations_are_rebuilt_in_order_on_the_server_version() {
        let local = [
            pending(5, add(&[1])),
            pending(
                6,
                Op::Move {
                    entry: id(1),
                    to: Position::Before(id(5)),
                },
            ),
        ];
        assert_eq!(
            rebase(&local, &server()),
            Rebased {
                queue: Queue {
                    version: 10,
                    up_next: [entries(&[1], playlist(2)), entries(&[5], playlist(1))].concat(),
                    cursor: 2,
                    ..server()
                },
                pending: vec![
                    pending(8, add(&[1])),
                    pending(
                        9,
                        Op::Move {
                            entry: id(1),
                            to: Position::Before(id(5)),
                        },
                    ),
                ],
                dropped: vec![],
            }
        );
    }

    #[test]
    fn an_operation_that_no_longer_fits_is_dropped_with_its_reason() {
        let local = [
            pending(6, Op::Remove(vec![id(11)])),
            pending(7, add(&[1])),
            pending(8, Op::Skip(id(10))),
        ];
        assert_eq!(
            rebase(&local, &server()),
            Rebased {
                queue: Queue {
                    version: 9,
                    up_next: [entries(&[5], playlist(1)), entries(&[1], playlist(2))].concat(),
                    ..server()
                },
                pending: vec![pending(8, add(&[1]))],
                dropped: vec![
                    Dropped {
                        op: Op::Remove(vec![id(11)]),
                        reason: QueueReject::UnknownEntry { entry: id(11) },
                    },
                    Dropped {
                        op: Op::Skip(id(10)),
                        reason: QueueReject::NotCurrent { entry: id(10) },
                    },
                ],
            }
        );
    }

    #[test]
    fn an_operation_the_server_already_applied_is_not_applied_twice() {
        // The acknowledgement of the first Add was lost, so the client still
        // holds it although the server's queue has its entry.
        let mut applied = server();
        applied.up_next.push(entry(1, playlist(2)));
        assert_eq!(
            rebase(&[pending(8, add(&[1]))], &applied),
            Rebased {
                queue: applied.clone(),
                pending: vec![],
                dropped: vec![Dropped {
                    op: add(&[1]),
                    reason: QueueReject::DuplicateEntry { entry: id(1) },
                }],
            }
        );
    }
}
