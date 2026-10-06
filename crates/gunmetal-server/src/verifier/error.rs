//! The two answers a refused sign-in gets.
//!
//! Whatever was wrong with a credential, the answer is
//! [`SignInError::Refused`]: one value, with nothing in it that depends on
//! whether the account or the code exists, is disabled or is wrong, so the
//! response rendered from it is the same bytes in every case (SEC-API-058,
//! SEC-IAM-022). An attempt the limiter refused is told only how long to
//! wait (SEC-API-057).

use gunmetal_core::problem::{Arg, Describe, Problem, ProblemCode};

/// Why the verifier did not accept a credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignInError {
    /// The credential was refused: it is unknown, disabled, wrong, missing
    /// or empty, or was issued for another pathway, or the check could not
    /// be finished. Nothing says which. Answered 401.
    Refused,
    /// Too many attempts: the source's or the server's ceiling was
    /// reached, or an earlier wrong guess at a short secret has not been
    /// waited out. The credential was not looked at. Answered 429 with
    /// `Retry-After`.
    Limited {
        /// Milliseconds until an attempt would be looked at.
        retry_after_ms: u64,
    },
}

impl SignInError {
    /// The value of the `Retry-After` header: the wait in whole seconds,
    /// rounded up, for a limited attempt.
    #[must_use]
    pub const fn retry_after_seconds(self) -> Option<u64> {
        match self {
            Self::Refused => None,
            Self::Limited { retry_after_ms } => Some(retry_after_ms.div_ceil(1_000)),
        }
    }
}

impl Describe for SignInError {
    fn problem(&self) -> Problem {
        match self.retry_after_seconds() {
            None => Problem {
                code: ProblemCode::Unauthenticated,
                args: Vec::new(),
            },
            Some(seconds) => Problem {
                code: ProblemCode::TooManyAttempts,
                args: vec![("retry_after_seconds", Arg::Number(seconds))],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use gunmetal_core::problem::{Arg, Describe, Problem, ProblemCode};

    use super::*;

    /// Verifies: SEC-API-058
    #[test]
    fn a_refusal_is_the_one_unauthenticated_problem_with_nothing_in_it() {
        assert_eq!(
            SignInError::Refused.problem(),
            Problem {
                code: ProblemCode::Unauthenticated,
                args: vec![],
            }
        );
        assert_eq!(SignInError::Refused.retry_after_seconds(), None);
        // What a client is told: the status and the code's own text.
        assert_eq!(
            (
                ProblemCode::Unauthenticated.code(),
                ProblemCode::Unauthenticated.status(),
                ProblemCode::Unauthenticated.text(),
            ),
            ("unauthenticated", Some(401), "Sign in to continue.")
        );
    }

    #[test]
    fn a_limited_attempt_is_told_how_long_to_wait_in_whole_seconds() {
        // The wait in milliseconds, and the seconds a client is told.
        let waits = [
            (0, 0),
            (1, 1),
            (999, 1),
            (1_000, 1),
            (1_001, 2),
            (6_000, 6),
            (29_999, 30),
            (30_000, 30),
            (900_000, 900),
        ];
        for (retry_after_ms, seconds) in waits {
            let limited = SignInError::Limited { retry_after_ms };
            assert_eq!(limited.retry_after_seconds(), Some(seconds));
            assert_eq!(
                limited.problem(),
                Problem {
                    code: ProblemCode::TooManyAttempts,
                    args: vec![("retry_after_seconds", Arg::Number(seconds))],
                },
                "{retry_after_ms}"
            );
        }
        assert_eq!(
            (
                ProblemCode::TooManyAttempts.code(),
                ProblemCode::TooManyAttempts.status(),
                ProblemCode::TooManyAttempts.text(),
            ),
            (
                "too_many_attempts",
                Some(429),
                "Too many attempts. Wait a little, then try again."
            )
        );
    }
}
