//! The schedule of a rotating key's generations: which key ID signs now,
//! and which key IDs still verify (SEC-API-030, SEC-OPS-015).
//!
//! A rotating key, such as the URL-signing key, has numbered generations.
//! Each generation's key is derived from the root secret by its number, and
//! what it signs carries the low byte of that number as its key ID. A new
//! generation begins once [`ROTATE_AFTER_MS`] have passed, a day, which is
//! stricter than the 30 days SEC-API-030 allows. The generation it replaces
//! stays answerable for [`OVERLAP_MS`], the longest lifetime of a stream
//! capability (capability.stream, 4 hours), and not a moment longer.
//! Revoking a key ID ends its generation at once; when that is the current
//! one, the next generation begins at the same moment.
//!
//! At most two generations answer at any time and their numbers differ by
//! less than 256 (barring 255 revocations inside one overlap), so their key
//! IDs differ. A key ID seen again 256 generations later selects the new
//! generation, whose key is a different one, so nothing signed under the
//! old generation verifies again.

use gunmetal_core::time::Timestamp;

/// How long a generation signs before the next one begins: a day.
pub const ROTATE_AFTER_MS: i64 = 86_400_000;

/// How long a replaced generation still verifies: 4 hours, the longest
/// lifetime of a stream capability (capability.stream).
pub const OVERLAP_MS: i64 = 14_400_000;

/// The generations of one rotating key that may sign or verify.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeySchedule {
    /// The number of the generation that signs.
    current: u64,
    /// When it began.
    began: Timestamp,
    /// The generation it replaced, and when it was replaced.
    previous: Option<(u64, Timestamp)>,
}

impl KeySchedule {
    /// A schedule whose first generation, number 0, begins at `now`.
    #[must_use]
    pub const fn new(now: Timestamp) -> Self {
        Self {
            current: 0,
            began: now,
            previous: None,
        }
    }

    /// The key ID that signs now.
    #[must_use]
    pub const fn current_kid(&self) -> u8 {
        kid(self.current)
    }

    /// Begins the next generation when the current one has signed for
    /// [`ROTATE_AFTER_MS`] or longer by `now`.
    pub const fn rotate_if_due(&mut self, now: Timestamp) {
        if now.millis().saturating_sub(self.began.millis()) >= ROTATE_AFTER_MS {
            self.previous = Some((self.current, now));
            self.current = self.current.saturating_add(1);
            self.began = now;
        }
    }

    /// Ends the generation with key ID `kid` at `now`, so that nothing it
    /// signed verifies any more. When it is the current one, the next
    /// generation begins at `now`. A key ID that names no answering
    /// generation changes nothing.
    pub fn revoke(&mut self, kid: u8, now: Timestamp) {
        if self
            .previous
            .is_some_and(|(number, _)| self::kid(number) == kid)
        {
            self.previous = None;
        }
        if self::kid(self.current) == kid {
            self.current = self.current.saturating_add(1);
            self.began = now;
        }
    }

    /// The number of the generation whose key verifies what key ID `kid`
    /// signed, at `now`, or `None` when no generation answers to it.
    #[must_use]
    pub fn generation(&self, kid: u8, now: Timestamp) -> Option<u64> {
        if self::kid(self.current) == kid {
            return Some(self.current);
        }
        self.previous
            .filter(|&(number, replaced)| {
                self::kid(number) == kid
                    && now.millis().saturating_sub(replaced.millis()) < OVERLAP_MS
            })
            .map(|(number, _)| number)
    }
}

/// The key ID of generation `number`: its low byte.
const fn kid(number: u64) -> u8 {
    number.to_le_bytes()[0]
}

#[cfg(test)]
mod tests {
    use gunmetal_core::time::Timestamp;

    use super::KeySchedule;

    /// 2026-09-21T00:00:00Z.
    const START: i64 = 1_790_000_000_000;
    /// A day, and four hours, in milliseconds, written out.
    const DAY: i64 = 86_400_000;
    const HOURS_4: i64 = 14_400_000;

    /// The moment `millis` after [`START`].
    fn at(millis: i64) -> Timestamp {
        Timestamp::from_millis(START + millis).unwrap()
    }

    /// The generations answering key IDs 0 to 3 at `millis` after
    /// [`START`].
    fn answers(schedule: &KeySchedule, millis: i64) -> [Option<u64>; 4] {
        [0, 1, 2, 3].map(|kid| schedule.generation(kid, at(millis)))
    }

    #[test]
    fn a_new_schedule_signs_and_verifies_with_generation_zero_only() {
        let schedule = KeySchedule::new(at(0));
        assert_eq!(
            schedule,
            KeySchedule {
                current: 0,
                began: at(0),
                previous: None
            }
        );
        assert_eq!(schedule.current_kid(), 0);
        assert_eq!(answers(&schedule, 0), [Some(0), None, None, None]);
        assert_eq!(schedule.generation(255, at(0)), None);
    }

    /// Verifies: SEC-API-030
    ///
    /// The key rotates after a day, not before, and the key it replaced
    /// still verifies for the four hours of the longest stream capability
    /// and never after.
    #[test]
    fn rotation_keeps_the_previous_key_id_answering_for_the_overlap_only() {
        let mut schedule = KeySchedule::new(at(0));
        schedule.rotate_if_due(at(DAY - 1));
        assert_eq!(schedule, KeySchedule::new(at(0)));
        schedule.rotate_if_due(at(DAY));
        assert_eq!(
            schedule,
            KeySchedule {
                current: 1,
                began: at(DAY),
                previous: Some((0, at(DAY)))
            }
        );
        assert_eq!(schedule.current_kid(), 1);
        assert_eq!(answers(&schedule, DAY), [Some(0), Some(1), None, None]);
        assert_eq!(
            answers(&schedule, DAY + HOURS_4 - 1),
            [Some(0), Some(1), None, None]
        );
        assert_eq!(
            answers(&schedule, DAY + HOURS_4),
            [None, Some(1), None, None]
        );
        // The next rotation counts from when generation 1 began.
        schedule.rotate_if_due(at(2 * DAY - 1));
        assert_eq!(schedule.current_kid(), 1);
        schedule.rotate_if_due(at(2 * DAY));
        assert_eq!(answers(&schedule, 2 * DAY), [None, Some(1), Some(2), None]);
    }

    #[test]
    fn a_rotation_after_a_long_gap_begins_one_generation_at_once() {
        let mut schedule = KeySchedule::new(at(0));
        schedule.rotate_if_due(at(10 * DAY + 5));
        assert_eq!(
            schedule,
            KeySchedule {
                current: 1,
                began: at(10 * DAY + 5),
                previous: Some((0, at(10 * DAY + 5)))
            }
        );
    }

    /// Verifies: SEC-API-030
    ///
    /// Revoking the previous key ID refuses it at once, inside the overlap,
    /// and leaves the current key alone.
    #[test]
    fn revoking_the_previous_key_id_refuses_it_at_once() {
        let mut schedule = KeySchedule::new(at(0));
        schedule.rotate_if_due(at(DAY));
        schedule.revoke(0, at(DAY + 1));
        assert_eq!(
            schedule,
            KeySchedule {
                current: 1,
                began: at(DAY),
                previous: None
            }
        );
        assert_eq!(answers(&schedule, DAY + 1), [None, Some(1), None, None]);
    }

    /// Verifies: SEC-API-030
    ///
    /// Revoking the current key ID refuses it at once and begins the next
    /// generation; the previous key still answers for its overlap.
    #[test]
    fn revoking_the_current_key_id_refuses_it_and_begins_the_next() {
        let mut schedule = KeySchedule::new(at(0));
        schedule.rotate_if_due(at(DAY));
        schedule.revoke(1, at(DAY + 7));
        assert_eq!(
            schedule,
            KeySchedule {
                current: 2,
                began: at(DAY + 7),
                previous: Some((0, at(DAY)))
            }
        );
        assert_eq!(schedule.current_kid(), 2);
        assert_eq!(answers(&schedule, DAY + 7), [Some(0), None, Some(2), None]);
        // The new generation signs for a whole day from its own start.
        schedule.rotate_if_due(at(2 * DAY + 6));
        assert_eq!(schedule.current_kid(), 2);
    }

    #[test]
    fn revoking_a_key_id_no_generation_answers_to_changes_nothing() {
        let mut schedule = KeySchedule::new(at(0));
        schedule.rotate_if_due(at(DAY));
        let before = schedule;
        schedule.revoke(2, at(DAY + 1));
        schedule.revoke(255, at(DAY + 1));
        assert_eq!(schedule, before);
    }

    /// Verifies: SEC-API-030
    ///
    /// After 256 rotations key ID 0 comes round again and selects
    /// generation 256, never generation 0, whose key signed long ago.
    #[test]
    fn a_key_id_that_comes_round_again_selects_the_new_generation() {
        let mut schedule = KeySchedule::new(at(0));
        for day in 1..=256 {
            schedule.rotate_if_due(at(day * DAY));
        }
        assert_eq!(
            schedule,
            KeySchedule {
                current: 256,
                began: at(256 * DAY),
                previous: Some((255, at(256 * DAY)))
            }
        );
        assert_eq!(schedule.current_kid(), 0);
        assert_eq!(schedule.generation(0, at(256 * DAY)), Some(256));
        assert_eq!(schedule.generation(255, at(256 * DAY)), Some(255));
        assert_eq!(schedule.generation(1, at(256 * DAY)), None);
    }
}
