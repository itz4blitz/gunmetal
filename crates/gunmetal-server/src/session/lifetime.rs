//! How long a browser session lasts (SEC-IAM-041, SEC-CLI-010).
//!
//! The limits are the session rows of the security parameters table
//! (`session.browser`), and for a browser marked shared at sign-in the
//! 30 minutes of SEC-CLI-010. A session ends when it has gone unused for
//! its idle limit or has lasted its total limit, whichever comes first.

use gunmetal_core::authz::DeviceClass;

/// Seven days, in milliseconds.
const WEEK_MS: i64 = 604_800_000;

/// Thirty minutes, in milliseconds.
const HALF_HOUR_MS: i64 = 1_800_000;

/// Thirty days, in milliseconds.
const MONTH_MS: i64 = 2_592_000_000;

/// The mode a browser signed in with: the answer to the one question the
/// web client asks at sign-in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lifetime {
    /// One person's own browser. Its session ends after 7 days without use
    /// or 30 days in all, and its cookie outlives the browser window.
    Personal,
    /// A browser several people use. Its session ends after 30 minutes
    /// without use, and its cookie goes when the browser closes.
    Shared,
}

impl Lifetime {
    /// Both modes.
    pub const ALL: [Self; 2] = [Self::Personal, Self::Shared];

    /// How many milliseconds a session of this mode may go unused.
    #[must_use]
    pub const fn idle_ms(self) -> i64 {
        match self {
            Self::Personal => WEEK_MS,
            Self::Shared => HALF_HOUR_MS,
        }
    }

    /// How many milliseconds a session of this mode may last in all.
    #[must_use]
    pub const fn total_ms(self) -> i64 {
        match self {
            Self::Personal | Self::Shared => MONTH_MS,
        }
    }

    /// Whether a device of `class` may hold a session of this mode. A
    /// browser in shared mode is a limited-class device, whatever it says
    /// of itself (SEC-CLI-024).
    #[must_use]
    pub fn fits(self, class: DeviceClass) -> bool {
        self == Self::Personal || class == DeviceClass::Limited
    }

    /// Whether a session of this mode has ended, `age_ms` milliseconds
    /// after it was issued and `unused_ms` milliseconds after it was last
    /// used.
    #[must_use]
    pub const fn ended(self, age_ms: i64, unused_ms: i64) -> bool {
        unused_ms >= self.idle_ms() || age_ms >= self.total_ms()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies: SEC-IAM-041, SEC-CLI-010
    #[test]
    fn the_limits_are_those_of_the_parameters_table() {
        assert_eq!(
            Lifetime::ALL.map(|mode| (mode, mode.idle_ms(), mode.total_ms())),
            [
                (Lifetime::Personal, 604_800_000, 2_592_000_000),
                (Lifetime::Shared, 1_800_000, 2_592_000_000),
            ]
        );
    }

    /// Verifies: SEC-IAM-041, SEC-CLI-010
    #[test]
    fn a_session_ends_at_its_idle_limit_or_its_total_limit() {
        // The mode, the session's age, how long it has gone unused, and
        // whether it has ended.
        let cases = [
            (Lifetime::Personal, 0, 0, false),
            (Lifetime::Personal, 604_799_999, 604_799_999, false),
            (Lifetime::Personal, 604_800_000, 604_800_000, true),
            (Lifetime::Personal, 2_000_000_000, 604_800_001, true),
            (Lifetime::Personal, 2_591_999_999, 0, false),
            (Lifetime::Personal, 2_592_000_000, 0, true),
            (Lifetime::Personal, 2_592_000_001, 5, true),
            (Lifetime::Shared, 1_799_999, 1_799_999, false),
            (Lifetime::Shared, 1_800_000, 1_800_000, true),
            (Lifetime::Shared, 2_591_999_999, 1_799_999, false),
            (Lifetime::Shared, 2_592_000_000, 0, true),
            // A clock that stepped back makes a session look younger than
            // it is, never ended.
            (Lifetime::Shared, -5, -5, false),
        ];
        let ended = cases.map(|(mode, age, unused, _)| mode.ended(age, unused));
        assert_eq!(ended, cases.map(|(.., expected)| expected));
    }

    /// Verifies: SEC-CLI-024
    #[test]
    fn only_a_limited_class_device_holds_a_shared_session() {
        let fits = |mode: Lifetime| {
            [DeviceClass::Personal, DeviceClass::Limited].map(|class| mode.fits(class))
        };
        assert_eq!(Lifetime::ALL.map(fits), [[true, true], [false, true]]);
    }
}
