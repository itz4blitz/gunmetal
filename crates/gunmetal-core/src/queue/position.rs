//! How far into the current entry its player has got.
//!
//! A position is not an operation. The player reports it at play, pause,
//! seek and track change and every so often while playing, and the server
//! keeps at most one report per batch window (API-QUE-04, ADR 3), so a
//! report never gives the queue a new version and never makes another
//! device's operation stale. Only the session that holds the queue may
//! report: a session that lost it to a later Play reports nothing, even the
//! place where it paused.

use crate::id::PublicId;
use crate::values::Duration;

use super::document::{EntryId, Queue};
use super::op::QueueReject;

/// Records that session `by` has played the current entry, `entry`, up to
/// `at`, and returns the queue at the same version.
///
/// # Errors
///
/// [`QueueReject::NotPlayer`] unless `by` is the queue's player, and
/// [`QueueReject::NotCurrent`] unless `entry` is the current entry.
pub fn report_position(
    queue: &Queue,
    by: PublicId,
    entry: EntryId,
    at: Duration,
) -> Result<Queue, QueueReject> {
    if queue.player != Some(by) {
        return Err(QueueReject::NotPlayer);
    }
    if queue
        .current
        .is_none_or(|current| current.entry.id != entry)
    {
        return Err(QueueReject::NotCurrent { entry });
    }
    Ok(Queue {
        position: Some(at),
        ..queue.clone()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::document::{Current, Lane};
    use crate::queue::fixtures::{album, empty, entry, id, session};

    fn playing() -> Queue {
        Queue {
            version: 4,
            current: Some(Current {
                entry: entry(3, album(1)),
                lane: Lane::From,
            }),
            player: Some(session(1)),
            ..empty()
        }
    }

    fn at(millis: u64) -> Duration {
        Duration::from_millis(millis).unwrap()
    }

    #[test]
    fn the_player_reports_its_place_without_a_new_version() {
        let reported = report_position(&playing(), session(1), id(3), at(83_500));
        assert_eq!(
            reported,
            Ok(Queue {
                position: Some(at(83_500)),
                ..playing()
            })
        );
        assert_eq!(
            report_position(&reported.unwrap(), session(1), id(3), at(0)),
            Ok(Queue {
                position: Some(at(0)),
                ..playing()
            })
        );
    }

    #[test]
    fn only_the_session_playing_the_queue_reports() {
        assert_eq!(
            report_position(&playing(), session(2), id(3), at(1_000)),
            Err(QueueReject::NotPlayer)
        );
        let stopped = Queue {
            player: None,
            ..playing()
        };
        assert_eq!(
            report_position(&stopped, session(1), id(3), at(1_000)),
            Err(QueueReject::NotPlayer)
        );
    }

    #[test]
    fn a_report_for_an_entry_no_longer_current_is_refused() {
        assert_eq!(
            report_position(&playing(), session(1), id(4), at(1_000)),
            Err(QueueReject::NotCurrent { entry: id(4) })
        );
        let idle = Queue {
            player: Some(session(1)),
            ..empty()
        };
        assert_eq!(
            report_position(&idle, session(1), id(3), at(1_000)),
            Err(QueueReject::NotCurrent { entry: id(3) })
        );
    }
}
