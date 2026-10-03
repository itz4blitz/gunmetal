//! The retention schedule: how long each class of data is kept, and the
//! purge decision every sweep takes from it.
//!
//! SEC-PRV-005 asks for one schedule in code, and the first principles ask
//! for one value per parameter. The audit log's pruning and address
//! coarsening (WP-069), log rotation (WP-097), backup expiry (WP-090) and
//! the daily sweep (WP-133) read their periods from [`Schedule`] and ask
//! [`decide`] what to do with each item, so none of them holds a number of
//! its own. Running a purge is their business; this module only decides.
//!
//! The defaults are the baseline's ([`DEFAULT`]). The owner may shorten a
//! period, never lengthen it (ADM-111), so each setting's ceiling is its
//! default; its floor is one unit, so a period can never be zero. History is
//! the person's own: it is kept until they delete it unless they choose 90
//! days, 1 year or 2 years ([`HistoryRetention`]).
//!
//! The decision depends only on the item's class, age and size and on the
//! schedule, never on what an earlier sweep did, so running a sweep twice
//! leaves the same data as running it once.

use core::time::Duration;

use crate::time::Timestamp;

/// Seconds in an hour.
const SECS_PER_HOUR: u64 = 3_600;
/// Seconds in a day.
const SECS_PER_DAY: u64 = 86_400;

/// A class of data with its own retention period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataClassRetention {
    /// A record in the security audit log, kept for
    /// [`Setting::SecurityEventDays`].
    SecurityEvent,
    /// The source address held beside a security event: coarsened to a /24
    /// or /48 after [`Setting::AddressCoarsenDays`] and removed after
    /// [`Setting::AddressRemoveDays`].
    EventAddress,
    /// A diagnostic (application) log file, kept for
    /// [`Setting::DiagnosticLogDays`] while the logs together stay within
    /// [`Setting::DiagnosticLogBytes`].
    DiagnosticLog,
    /// An HTTP access log file, when the owner has switched access logging
    /// on, kept for [`Setting::AccessLogDays`].
    AccessLog,
    /// A diagnostic bundle, kept for [`Setting::DiagnosticBundleHours`] if
    /// nobody downloads it first.
    DiagnosticBundle,
    /// A backup archive, kept for [`Setting::BackupDays`] (SEC-PRV-041).
    Backup,
    /// An invitation that has expired, kept for
    /// [`Setting::ExpiredInvitationDays`] after it expired.
    ExpiredInvitation,
    /// A person's listening history, kept as their [`HistoryRetention`]
    /// says.
    History,
}

/// What a sweep does with one item.
///
/// The variants are ordered by how much they take away, so an item's
/// decision only ever moves from `Keep` towards `Remove` as it ages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Purge {
    /// Leave the item as it is.
    Keep,
    /// Shorten the item's address to its /24 or /48. Coarsening an address
    /// that is already coarse leaves it unchanged.
    Coarsen,
    /// Delete the item.
    Remove,
}

/// How long a person keeps their listening history. They choose; the
/// owner cannot (baseline owner decision 15, ACC-118).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HistoryRetention {
    /// Until the person deletes it: the default, an owner-approved
    /// exception to the most-private default (decision D-36).
    UntilDeleted,
    /// 90 days.
    Days90,
    /// 365 days.
    Year1,
    /// 730 days.
    Years2,
}

impl HistoryRetention {
    /// Every choice, in the order a settings screen lists them.
    pub const ALL: [Self; 4] = [Self::UntilDeleted, Self::Days90, Self::Year1, Self::Years2];

    /// The period in days, or `None` for history kept until deleted.
    #[must_use]
    pub const fn days(self) -> Option<u64> {
        match self {
            Self::UntilDeleted => None,
            Self::Days90 => Some(90),
            Self::Year1 => Some(365),
            Self::Years2 => Some(730),
        }
    }
}

/// One period or cap in the schedule that the owner may shorten.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Setting {
    /// Days a security event is kept.
    SecurityEventDays,
    /// Days after which an event's source address is coarsened.
    AddressCoarsenDays,
    /// Days after which an event's source address is removed.
    AddressRemoveDays,
    /// Days a diagnostic log file is kept.
    DiagnosticLogDays,
    /// Bytes of diagnostic logs kept, newest first.
    DiagnosticLogBytes,
    /// Days an access log file is kept.
    AccessLogDays,
    /// Hours a diagnostic bundle is kept.
    DiagnosticBundleHours,
    /// Days a backup archive is kept.
    BackupDays,
    /// Days an invitation is kept after it expired.
    ExpiredInvitationDays,
}

impl Setting {
    /// Every setting, in the order the settings screen and the privacy
    /// notice list them.
    pub const ALL: [Self; 9] = [
        Self::SecurityEventDays,
        Self::AddressCoarsenDays,
        Self::AddressRemoveDays,
        Self::DiagnosticLogDays,
        Self::DiagnosticLogBytes,
        Self::AccessLogDays,
        Self::DiagnosticBundleHours,
        Self::BackupDays,
        Self::ExpiredInvitationDays,
    ];

    /// The setting's name in the configuration and in error messages.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::SecurityEventDays => "retention.security_events.days",
            Self::AddressCoarsenDays => "retention.event_addresses.coarsen_days",
            Self::AddressRemoveDays => "retention.event_addresses.remove_days",
            Self::DiagnosticLogDays => "retention.diagnostic_logs.days",
            Self::DiagnosticLogBytes => "retention.diagnostic_logs.bytes",
            Self::AccessLogDays => "retention.access_logs.days",
            Self::DiagnosticBundleHours => "retention.diagnostic_bundles.hours",
            Self::BackupDays => "retention.backups.days",
            Self::ExpiredInvitationDays => "retention.expired_invitations.days",
        }
    }

    /// The smallest value the owner may set.
    #[must_use]
    pub const fn min(self) -> u64 {
        match self {
            Self::DiagnosticLogBytes => 1_000_000,
            _ => 1,
        }
    }

    /// The largest value the owner may set: the default, because the owner
    /// may shorten a period but never lengthen it (ADM-111).
    #[must_use]
    pub const fn max(self) -> u64 {
        DEFAULT.value(self)
    }
}

/// Why an override was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetentionError {
    /// The value lies outside the setting's floor and ceiling.
    OutOfRange {
        /// The setting the value was for; [`Setting::name`] names it.
        setting: Setting,
        /// The value refused.
        value: u64,
        /// The setting's floor.
        min: u64,
        /// The setting's ceiling.
        max: u64,
    },
    /// Addresses would be removed before, or on the day, they are
    /// coarsened, so the coarsening would never happen.
    CoarsenNotBeforeRemoval {
        /// [`Setting::AddressCoarsenDays`] as configured.
        coarsen_days: u64,
        /// [`Setting::AddressRemoveDays`] as configured.
        remove_days: u64,
    },
    /// An address would outlive the security event it belongs to.
    AddressOutlivesEvent {
        /// [`Setting::AddressRemoveDays`] as configured.
        remove_days: u64,
        /// [`Setting::SecurityEventDays`] as configured.
        event_days: u64,
    },
}

/// The retention schedule: one period per data class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Schedule {
    security_event_days: u64,
    address_coarsen_days: u64,
    address_remove_days: u64,
    diagnostic_log_days: u64,
    diagnostic_log_bytes: u64,
    access_log_days: u64,
    diagnostic_bundle_hours: u64,
    backup_days: u64,
    expired_invitation_days: u64,
    history: HistoryRetention,
}

/// The baseline's schedule (SEC-PRV-005, SEC-PRV-041, privacy design
/// guidance section 1).
pub const DEFAULT: Schedule = Schedule {
    security_event_days: 365,
    address_coarsen_days: 30,
    address_remove_days: 90,
    diagnostic_log_days: 14,
    diagnostic_log_bytes: 100_000_000,
    access_log_days: 7,
    diagnostic_bundle_hours: 24,
    backup_days: 14,
    expired_invitation_days: 30,
    history: HistoryRetention::UntilDeleted,
};

impl Schedule {
    /// The default schedule with the owner's overrides applied in order.
    ///
    /// # Errors
    ///
    /// [`RetentionError::OutOfRange`] for the first value outside its
    /// setting's floor and ceiling, then
    /// [`RetentionError::CoarsenNotBeforeRemoval`] or
    /// [`RetentionError::AddressOutlivesEvent`] if the address periods no
    /// longer fit inside one another.
    pub fn new(overrides: &[(Setting, u64)]) -> Result<Self, RetentionError> {
        let mut schedule = DEFAULT;
        for &(setting, value) in overrides {
            let (min, max) = (setting.min(), setting.max());
            if !(min..=max).contains(&value) {
                return Err(RetentionError::OutOfRange {
                    setting,
                    value,
                    min,
                    max,
                });
            }
            *schedule.slot(setting) = value;
        }
        let Self {
            security_event_days: event_days,
            address_coarsen_days: coarsen_days,
            address_remove_days: remove_days,
            ..
        } = schedule;
        if coarsen_days >= remove_days {
            Err(RetentionError::CoarsenNotBeforeRemoval {
                coarsen_days,
                remove_days,
            })
        } else if remove_days > event_days {
            Err(RetentionError::AddressOutlivesEvent {
                remove_days,
                event_days,
            })
        } else {
            Ok(schedule)
        }
    }

    /// The same schedule with a person's history choice.
    #[must_use]
    pub const fn with_history(mut self, history: HistoryRetention) -> Self {
        self.history = history;
        self
    }

    /// A setting's value in this schedule.
    #[must_use]
    pub const fn value(&self, setting: Setting) -> u64 {
        match setting {
            Setting::SecurityEventDays => self.security_event_days,
            Setting::AddressCoarsenDays => self.address_coarsen_days,
            Setting::AddressRemoveDays => self.address_remove_days,
            Setting::DiagnosticLogDays => self.diagnostic_log_days,
            Setting::DiagnosticLogBytes => self.diagnostic_log_bytes,
            Setting::AccessLogDays => self.access_log_days,
            Setting::DiagnosticBundleHours => self.diagnostic_bundle_hours,
            Setting::BackupDays => self.backup_days,
            Setting::ExpiredInvitationDays => self.expired_invitation_days,
        }
    }

    /// Where a setting's value lives, for [`Self::new`].
    const fn slot(&mut self, setting: Setting) -> &mut u64 {
        match setting {
            Setting::SecurityEventDays => &mut self.security_event_days,
            Setting::AddressCoarsenDays => &mut self.address_coarsen_days,
            Setting::AddressRemoveDays => &mut self.address_remove_days,
            Setting::DiagnosticLogDays => &mut self.diagnostic_log_days,
            Setting::DiagnosticLogBytes => &mut self.diagnostic_log_bytes,
            Setting::AccessLogDays => &mut self.access_log_days,
            Setting::DiagnosticBundleHours => &mut self.diagnostic_bundle_hours,
            Setting::BackupDays => &mut self.backup_days,
            Setting::ExpiredInvitationDays => &mut self.expired_invitation_days,
        }
    }

    /// The history period in this schedule.
    #[must_use]
    pub const fn history(&self) -> HistoryRetention {
        self.history
    }
}

/// How old an item stamped `since` is at `now`. An item stamped in the
/// future, after the clock stepped back, is zero days old, so it is kept.
#[must_use]
pub fn age(since: Timestamp, now: Timestamp) -> Duration {
    let millis = now.millis().saturating_sub(since.millis());
    Duration::from_millis(u64::try_from(millis).unwrap_or(0))
}

/// What a sweep does with one item of `class` that is `age` old.
///
/// `size` matters only for diagnostic logs: it is the bytes of this file
/// together with every newer diagnostic log, so walking the files newest
/// first removes the oldest ones beyond the cap. An item is removed once it
/// has reached its period, never before; an event's address is coarsened
/// before it is removed.
#[must_use]
pub fn decide(class: DataClassRetention, age: Duration, size: u64, schedule: &Schedule) -> Purge {
    let s = schedule;
    match class {
        DataClassRetention::SecurityEvent => expire(age, days(s.security_event_days)),
        DataClassRetention::EventAddress => {
            if age >= days(s.address_remove_days) {
                Purge::Remove
            } else if age >= days(s.address_coarsen_days) {
                Purge::Coarsen
            } else {
                Purge::Keep
            }
        }
        DataClassRetention::DiagnosticLog => {
            if size > s.diagnostic_log_bytes {
                Purge::Remove
            } else {
                expire(age, days(s.diagnostic_log_days))
            }
        }
        DataClassRetention::AccessLog => expire(age, days(s.access_log_days)),
        DataClassRetention::DiagnosticBundle => expire(age, hours(s.diagnostic_bundle_hours)),
        DataClassRetention::Backup => expire(age, days(s.backup_days)),
        DataClassRetention::ExpiredInvitation => expire(age, days(s.expired_invitation_days)),
        DataClassRetention::History => match s.history.days() {
            None => Purge::Keep,
            Some(period) => expire(age, days(period)),
        },
    }
}

/// Removes an item that has reached `period`, and keeps it before then.
fn expire(age: Duration, period: Duration) -> Purge {
    if age >= period {
        Purge::Remove
    } else {
        Purge::Keep
    }
}

/// `n` days. Every setting is at most a few hundred, so this never
/// saturates; saturating only satisfies the lint against unchecked
/// arithmetic.
const fn days(n: u64) -> Duration {
    Duration::from_secs(n.saturating_mul(SECS_PER_DAY))
}

/// `n` hours, as [`days`].
const fn hours(n: u64) -> Duration {
    Duration::from_secs(n.saturating_mul(SECS_PER_HOUR))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// One day.
    const DAY: Duration = Duration::from_secs(86_400);
    /// One millisecond, the smallest step a [`Timestamp`] can take.
    const MS: Duration = Duration::from_millis(1);

    /// One millisecond before `d`.
    fn before(d: Duration) -> Duration {
        d.checked_sub(MS).unwrap()
    }

    /// `n` days, written out for the tests.
    fn days(n: u64) -> Duration {
        Duration::from_secs(n.checked_mul(86_400).unwrap())
    }

    /// Verifies: SEC-PRV-005
    #[test]
    fn the_default_schedule_is_the_baselines() {
        assert_eq!(
            Setting::ALL.map(|s| (s.name(), DEFAULT.value(s))),
            [
                ("retention.security_events.days", 365),
                ("retention.event_addresses.coarsen_days", 30),
                ("retention.event_addresses.remove_days", 90),
                ("retention.diagnostic_logs.days", 14),
                ("retention.diagnostic_logs.bytes", 100_000_000),
                ("retention.access_logs.days", 7),
                ("retention.diagnostic_bundles.hours", 24),
                ("retention.backups.days", 14),
                ("retention.expired_invitations.days", 30),
            ]
        );
        assert_eq!(DEFAULT.history(), HistoryRetention::UntilDeleted);
        assert_eq!(Schedule::new(&[]), Ok(DEFAULT));
    }

    #[test]
    fn each_setting_may_be_shortened_to_one_unit_and_never_lengthened() {
        assert_eq!(
            Setting::ALL.map(|s| (s.min(), s.max())),
            [
                (1, 365),
                (1, 30),
                (1, 90),
                (1, 14),
                (1_000_000, 100_000_000),
                (1, 7),
                (1, 24),
                (1, 14),
                (1, 30),
            ]
        );
    }

    #[test]
    fn history_choices_are_until_deleted_90_days_1_year_and_2_years() {
        assert_eq!(
            HistoryRetention::ALL.map(HistoryRetention::days),
            [None, Some(90), Some(365), Some(730)]
        );
    }

    #[test]
    fn an_override_replaces_only_its_own_setting() {
        let schedule = Schedule::new(&[
            (Setting::SecurityEventDays, 200),
            (Setting::AddressCoarsenDays, 7),
            (Setting::AddressRemoveDays, 60),
            (Setting::DiagnosticLogDays, 3),
            (Setting::DiagnosticLogBytes, 5_000_000),
            (Setting::AccessLogDays, 2),
            (Setting::DiagnosticBundleHours, 6),
            (Setting::BackupDays, 10),
            (Setting::ExpiredInvitationDays, 5),
        ]);
        assert_eq!(
            schedule.map(|s| Setting::ALL.map(|setting| s.value(setting))),
            Ok([200, 7, 60, 3, 5_000_000, 2, 6, 10, 5])
        );
        assert_eq!(
            Schedule::new(&[(Setting::BackupDays, 1)])
                .map(|s| Setting::ALL.map(|setting| s.value(setting))),
            Ok([365, 30, 90, 14, 100_000_000, 7, 24, 1, 30])
        );
    }

    #[test]
    fn a_later_override_of_the_same_setting_wins() {
        assert_eq!(
            Schedule::new(&[(Setting::BackupDays, 3), (Setting::BackupDays, 9)])
                .map(|s| s.value(Setting::BackupDays)),
            Ok(9)
        );
    }

    #[test]
    fn the_floor_and_the_ceiling_themselves_are_accepted() {
        for setting in [
            Setting::DiagnosticLogDays,
            Setting::DiagnosticLogBytes,
            Setting::AccessLogDays,
            Setting::DiagnosticBundleHours,
            Setting::BackupDays,
            Setting::ExpiredInvitationDays,
        ] {
            for value in [setting.min(), setting.max()] {
                assert_eq!(
                    Schedule::new(&[(setting, value)]).map(|s| s.value(setting)),
                    Ok(value)
                );
            }
        }
    }

    #[test]
    fn an_override_below_the_floor_or_above_the_ceiling_is_refused_with_its_name() {
        assert_eq!(
            Schedule::new(&[(Setting::BackupDays, 15)]),
            Err(RetentionError::OutOfRange {
                setting: Setting::BackupDays,
                value: 15,
                min: 1,
                max: 14,
            })
        );
        assert_eq!(
            Schedule::new(&[(Setting::SecurityEventDays, 0)]),
            Err(RetentionError::OutOfRange {
                setting: Setting::SecurityEventDays,
                value: 0,
                min: 1,
                max: 365,
            })
        );
        assert_eq!(
            Schedule::new(&[(Setting::DiagnosticLogBytes, 999_999)]),
            Err(RetentionError::OutOfRange {
                setting: Setting::DiagnosticLogBytes,
                value: 999_999,
                min: 1_000_000,
                max: 100_000_000,
            })
        );
        // The first bad override is the one named, even after a good one.
        assert_eq!(
            Schedule::new(&[
                (Setting::AccessLogDays, 3),
                (Setting::DiagnosticBundleHours, 25),
                (Setting::BackupDays, 0),
            ]),
            Err(RetentionError::OutOfRange {
                setting: Setting::DiagnosticBundleHours,
                value: 25,
                min: 1,
                max: 24,
            })
        );
    }

    #[test]
    fn addresses_must_be_coarsened_before_they_are_removed() {
        assert_eq!(
            Schedule::new(&[(Setting::AddressRemoveDays, 30)]),
            Err(RetentionError::CoarsenNotBeforeRemoval {
                coarsen_days: 30,
                remove_days: 30,
            })
        );
        assert_eq!(
            Schedule::new(&[(Setting::AddressRemoveDays, 10)]),
            Err(RetentionError::CoarsenNotBeforeRemoval {
                coarsen_days: 30,
                remove_days: 10,
            })
        );
        assert_eq!(
            Schedule::new(&[
                (Setting::AddressCoarsenDays, 29),
                (Setting::AddressRemoveDays, 30),
            ])
            .map(|s| (
                s.value(Setting::AddressCoarsenDays),
                s.value(Setting::AddressRemoveDays)
            )),
            Ok((29, 30))
        );
    }

    #[test]
    fn an_address_may_not_outlive_its_event() {
        assert_eq!(
            Schedule::new(&[(Setting::SecurityEventDays, 89)]),
            Err(RetentionError::AddressOutlivesEvent {
                remove_days: 90,
                event_days: 89,
            })
        );
        assert_eq!(
            Schedule::new(&[(Setting::SecurityEventDays, 90)])
                .map(|s| s.value(Setting::SecurityEventDays)),
            Ok(90)
        );
    }

    #[test]
    fn a_history_choice_changes_only_the_history() {
        let mut expected = DEFAULT;
        expected.history = HistoryRetention::Year1;
        assert_eq!(DEFAULT.with_history(HistoryRetention::Year1), expected);
        assert_eq!(
            DEFAULT
                .with_history(HistoryRetention::Days90)
                .with_history(HistoryRetention::UntilDeleted),
            DEFAULT
        );
    }

    #[test]
    fn age_is_the_time_since_the_stamp_and_never_negative() {
        let at = |millis| Timestamp::from_millis(millis);
        assert_eq!(
            at(1_000).and_then(|since| at(86_401_000).map(|now| age(since, now))),
            Ok(DAY)
        );
        assert_eq!(
            at(5).and_then(|t| at(5).map(|now| age(t, now))),
            Ok(Duration::ZERO)
        );
        assert_eq!(
            at(6).and_then(|t| at(5).map(|now| age(t, now))),
            Ok(Duration::ZERO)
        );
        assert_eq!(
            age(Timestamp::MIN, Timestamp::MAX),
            Duration::from_millis(315_569_519_999_999)
        );
        assert_eq!(age(Timestamp::MAX, Timestamp::MIN), Duration::ZERO);
    }

    /// Each class at the edges of its periods under the default schedule.
    #[test]
    fn each_class_is_kept_until_its_period_and_removed_from_then_on() {
        use DataClassRetention as C;
        let cases = [
            (C::SecurityEvent, days(365)),
            (C::DiagnosticLog, days(14)),
            (C::AccessLog, days(7)),
            (C::DiagnosticBundle, Duration::from_secs(86_400)),
            (C::Backup, days(14)),
            (C::ExpiredInvitation, days(30)),
        ];
        for (class, period) in cases {
            assert_eq!(
                [
                    Duration::ZERO,
                    before(period),
                    period,
                    period.checked_add(DAY).unwrap()
                ]
                .map(|age| decide(class, age, 0, &DEFAULT)),
                [Purge::Keep, Purge::Keep, Purge::Remove, Purge::Remove]
            );
        }
    }

    #[test]
    fn an_event_address_is_kept_then_coarsened_then_removed() {
        assert_eq!(
            [
                Duration::ZERO,
                before(days(30)),
                days(30),
                before(days(90)),
                days(90),
                days(400),
            ]
            .map(|age| decide(DataClassRetention::EventAddress, age, 0, &DEFAULT)),
            [
                Purge::Keep,
                Purge::Keep,
                Purge::Coarsen,
                Purge::Coarsen,
                Purge::Remove,
                Purge::Remove,
            ]
        );
    }

    #[test]
    fn diagnostic_logs_beyond_the_size_cap_are_removed_whatever_their_age() {
        let decide_log = |age, size| decide(DataClassRetention::DiagnosticLog, age, size, &DEFAULT);
        assert_eq!(
            [
                decide_log(Duration::ZERO, 100_000_000),
                decide_log(Duration::ZERO, 100_000_001),
                decide_log(before(days(14)), u64::MAX),
                decide_log(days(14), 0),
            ],
            [Purge::Keep, Purge::Remove, Purge::Remove, Purge::Remove]
        );
    }

    #[test]
    fn history_is_kept_until_deleted_or_for_the_chosen_period() {
        let decide_history = |choice, age| {
            decide(
                DataClassRetention::History,
                age,
                u64::MAX,
                &DEFAULT.with_history(choice),
            )
        };
        assert_eq!(
            [Duration::ZERO, days(100_000), Duration::MAX]
                .map(|age| decide_history(HistoryRetention::UntilDeleted, age)),
            [Purge::Keep, Purge::Keep, Purge::Keep]
        );
        for (choice, period) in [
            (HistoryRetention::Days90, days(90)),
            (HistoryRetention::Year1, days(365)),
            (HistoryRetention::Years2, days(730)),
        ] {
            assert_eq!(
                [before(period), period].map(|age| decide_history(choice, age)),
                [Purge::Keep, Purge::Remove]
            );
        }
    }

    #[test]
    fn decisions_follow_the_owners_overrides() {
        use DataClassRetention as C;
        let schedule = Schedule::new(&[
            (Setting::SecurityEventDays, 40),
            (Setting::AddressCoarsenDays, 2),
            (Setting::AddressRemoveDays, 5),
            (Setting::DiagnosticLogDays, 3),
            (Setting::DiagnosticLogBytes, 2_000_000),
            (Setting::AccessLogDays, 1),
            (Setting::DiagnosticBundleHours, 1),
            (Setting::BackupDays, 4),
            (Setting::ExpiredInvitationDays, 6),
        ]);
        let cases = [
            (C::SecurityEvent, days(40)),
            (C::DiagnosticLog, days(3)),
            (C::AccessLog, days(1)),
            (C::DiagnosticBundle, Duration::from_secs(3_600)),
            (C::Backup, days(4)),
            (C::ExpiredInvitation, days(6)),
        ];
        for (class, period) in cases {
            assert_eq!(
                schedule.map(|s| [before(period), period].map(|age| decide(class, age, 0, &s))),
                Ok([Purge::Keep, Purge::Remove])
            );
        }
        assert_eq!(
            schedule.map(|s| {
                [before(days(2)), days(2), before(days(5)), days(5)]
                    .map(|age| decide(C::EventAddress, age, 0, &s))
            }),
            Ok([Purge::Keep, Purge::Coarsen, Purge::Coarsen, Purge::Remove])
        );
        assert_eq!(
            schedule.map(|s| {
                [2_000_000, 2_000_001]
                    .map(|size| decide(C::DiagnosticLog, Duration::ZERO, size, &s))
            }),
            Ok([Purge::Keep, Purge::Remove])
        );
    }

    /// Every class, for the generators.
    const CLASSES: [DataClassRetention; 8] = [
        DataClassRetention::SecurityEvent,
        DataClassRetention::EventAddress,
        DataClassRetention::DiagnosticLog,
        DataClassRetention::AccessLog,
        DataClassRetention::DiagnosticBundle,
        DataClassRetention::Backup,
        DataClassRetention::ExpiredInvitation,
        DataClassRetention::History,
    ];

    fn class() -> impl Strategy<Value = DataClassRetention> {
        proptest::sample::select(CLASSES.to_vec())
    }

    /// Any schedule the owner and a person could configure.
    fn schedule() -> impl Strategy<Value = Schedule> {
        (2_u64..=90)
            .prop_flat_map(|remove| (Just(remove), 1..remove, remove..=365))
            .prop_flat_map(|(remove, coarsen, events)| {
                (
                    Just([
                        (Setting::AddressCoarsenDays, coarsen),
                        (Setting::AddressRemoveDays, remove),
                        (Setting::SecurityEventDays, events),
                    ]),
                    1_u64..=14,
                    1_000_000_u64..=100_000_000,
                    1_u64..=7,
                    1_u64..=24,
                    1_u64..=14,
                    1_u64..=30,
                    proptest::sample::select(HistoryRetention::ALL.to_vec()),
                )
            })
            .prop_map(
                |(ordered, log, bytes, access, bundle, backup, invite, history)| {
                    let [a, b, c] = ordered;
                    let overrides = [
                        a,
                        b,
                        c,
                        (Setting::DiagnosticLogDays, log),
                        (Setting::DiagnosticLogBytes, bytes),
                        (Setting::AccessLogDays, access),
                        (Setting::DiagnosticBundleHours, bundle),
                        (Setting::BackupDays, backup),
                        (Setting::ExpiredInvitationDays, invite),
                    ];
                    Schedule::new(&overrides).map(|s| s.with_history(history))
                },
            )
            .prop_filter_map("every generated schedule is valid", Result::ok)
    }

    /// An oracle for the earliest age at which `class` loses anything,
    /// read from the schedule's values rather than from `decide`.
    fn first_limit(class: DataClassRetention, s: &Schedule) -> Option<Duration> {
        use DataClassRetention as C;
        let day = |setting| Some(days(s.value(setting)));
        match class {
            C::SecurityEvent => day(Setting::SecurityEventDays),
            C::EventAddress => day(Setting::AddressCoarsenDays),
            C::DiagnosticLog => day(Setting::DiagnosticLogDays),
            C::AccessLog => day(Setting::AccessLogDays),
            C::DiagnosticBundle => Some(Duration::from_secs(
                s.value(Setting::DiagnosticBundleHours)
                    .checked_mul(3_600)
                    .unwrap(),
            )),
            C::Backup => day(Setting::BackupDays),
            C::ExpiredInvitation => day(Setting::ExpiredInvitationDays),
            C::History => s.history().days().map(days),
        }
    }

    /// An item a sweep sees: its class, age in milliseconds, own size, and
    /// whether its address has been coarsened.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Item {
        class: DataClassRetention,
        age_ms: u64,
        size: u64,
        coarse: bool,
    }

    /// A model sweep over a store: diagnostic logs are walked newest first
    /// so each sees the bytes of itself and every newer log, as `decide`
    /// asks; every other item is decided on its own.
    fn sweep(items: &[Item], s: &Schedule) -> Vec<Item> {
        let is_log = |item: &Item| item.class == DataClassRetention::DiagnosticLog;
        let decided = items.iter().enumerate().map(|(i, item)| {
            let size = if is_log(item) {
                // This log and every newer one; ties in age go by position.
                items
                    .iter()
                    .enumerate()
                    .filter(|&(j, other)| is_log(other) && (other.age_ms, j) <= (item.age_ms, i))
                    .map(|(_, other)| other.size)
                    .sum()
            } else {
                item.size
            };
            match decide(item.class, Duration::from_millis(item.age_ms), size, s) {
                Purge::Keep => Some(*item),
                Purge::Coarsen => Some(Item {
                    coarse: true,
                    ..*item
                }),
                Purge::Remove => None,
            }
        });
        decided.flatten().collect()
    }

    fn item() -> impl Strategy<Value = Item> {
        (
            class(),
            0_u64..69_120_000_000,
            0_u64..60_000_000,
            any::<bool>(),
        )
            .prop_map(|(class, age_ms, size, coarse)| Item {
                class,
                age_ms,
                size,
                coarse,
            })
    }

    proptest! {
        /// Verifies: SEC-PRV-005
        #[test]
        fn purging_twice_equals_purging_once(
            s in schedule(),
            items in proptest::collection::vec(item(), 0..40),
        ) {
            let once = sweep(&items, &s);
            prop_assert_eq!(sweep(&once, &s), once);
        }

        /// Verifies: SEC-PRV-005
        #[test]
        fn nothing_younger_than_its_limit_is_removed_or_coarsened(
            s in schedule(),
            class in class(),
            fraction in 0.0_f64..1.0,
        ) {
            match first_limit(class, &s) {
                None => {
                    prop_assert_eq!(decide(class, Duration::MAX, u64::MAX, &s), Purge::Keep);
                }
                Some(limit) => {
                    let age = limit.mul_f64(fraction).min(before(limit));
                    let size = s.value(Setting::DiagnosticLogBytes);
                    prop_assert_eq!(decide(class, age, size, &s), Purge::Keep);
                    prop_assert_ne!(decide(class, limit, size, &s), Purge::Keep);
                }
            }
        }

        #[test]
        fn decisions_only_grow_with_age(
            s in schedule(),
            class in class(),
            younger_ms in any::<u64>(),
            extra_ms in any::<u64>(),
            size in any::<u64>(),
        ) {
            let younger = Duration::from_millis(younger_ms);
            let older = younger.saturating_add(Duration::from_millis(extra_ms));
            prop_assert!(decide(class, younger, size, &s) <= decide(class, older, size, &s));
        }

        #[test]
        fn an_address_is_coarsened_before_it_is_removed_never_the_reverse(
            s in schedule(),
            age_ms in any::<u64>(),
        ) {
            let class = DataClassRetention::EventAddress;
            let coarsen = days(s.value(Setting::AddressCoarsenDays));
            let remove = days(s.value(Setting::AddressRemoveDays));
            prop_assert_eq!(decide(class, coarsen, 0, &s), Purge::Coarsen);
            prop_assert_eq!(decide(class, before(remove), 0, &s), Purge::Coarsen);
            let age = Duration::from_millis(age_ms);
            let expected = if age < coarsen {
                Purge::Keep
            } else if age < remove {
                Purge::Coarsen
            } else {
                Purge::Remove
            };
            prop_assert_eq!(decide(class, age, 0, &s), expected);
        }

        #[test]
        fn an_override_outside_the_allowed_range_is_refused_with_its_name(
            index in 0_usize..9,
            below in any::<bool>(),
            distance in 1_u64..1_000_000_000,
        ) {
            let setting = Setting::ALL[index];
            let (name, min, max): (&str, u64, u64) = [
                ("retention.security_events.days", 1, 365),
                ("retention.event_addresses.coarsen_days", 1, 30),
                ("retention.event_addresses.remove_days", 1, 90),
                ("retention.diagnostic_logs.days", 1, 14),
                ("retention.diagnostic_logs.bytes", 1_000_000, 100_000_000),
                ("retention.access_logs.days", 1, 7),
                ("retention.diagnostic_bundles.hours", 1, 24),
                ("retention.backups.days", 1, 14),
                ("retention.expired_invitations.days", 1, 30),
            ][index];
            let value = if below { min.saturating_sub(distance) } else { max.checked_add(distance).unwrap() };
            prop_assume!(value < min || value > max);
            prop_assert_eq!(setting.name(), name);
            prop_assert_eq!(
                Schedule::new(&[(setting, value)]),
                Err(RetentionError::OutOfRange { setting, value, min, max })
            );
        }
    }
}
