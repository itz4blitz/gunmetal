//! The credential verifier: the one door every sign-in pathway goes
//! through (SEC-HIS-046, SEC-TM-014).
//!
//! A pathway's package never checks a credential on its own. It implements
//! [`pathway::PathwayCheck`] and hands that, with what the request
//! presented and the `ClientContext` the listener resolved, to
//! [`verify::Verifier::verify`], which does the same things in the same
//! order for all of them:
//!
//! 1. **Limits first.** The per-source and server-wide ceilings of
//!    `crate::limiter`, then, for a short secret, the delay its earlier
//!    wrong guesses have earned (SEC-IAM-101, SEC-API-056). A guess the
//!    delay lets through is counted as a wrong one before it is looked at,
//!    and the count is cleared if it turns out right, so guesses made at
//!    the same moment cannot all be looked at before one is counted. The
//!    counts are kept in memory only: no client address is stored, and a
//!    restart forgets them. A refused attempt never reaches the pathway's
//!    check. The claim code from the server itself is never limited
//!    (SEC-IAM-008).
//! 2. **No empty credential.** A request that presented nothing, or
//!    nothing but an empty value, is refused before the pathway is asked
//!    anything, so no pathway can treat a missing credential as a match
//!    (SEC-HIS-004).
//! 3. **The same work for every credential.** The pathway looks the stored
//!    credential up, only through the [`preauth::PreAuth`] handle the
//!    verifier passes it, and then compares. When nothing is stored the
//!    comparison still runs, against a decoy, and an unknown, a disabled
//!    and a wrong credential all get the one answer (SEC-API-058,
//!    SEC-IAM-022, SEC-HIS-047). A stored credential issued for another
//!    pathway is refused the same way (SEC-EXT-007).
//! 4. **Any fault is a denial.** An error, a timeout or missing data in
//!    the pathway's check ends in the same refusal, never in a weaker
//!    check, and a guess at a short secret whose check failed stays
//!    counted as a wrong one (SEC-IAM-069). The verifier's own bookkeeping
//!    needs nothing that can fail.
//!
//! # What a refused attempt leaves behind
//!
//! A limited attempt is answered [`error::SignInError::Limited`] and
//! emitted as an `excess_rate_limit_exceeded` security event. Every other
//! refusal is answered [`error::SignInError::Refused`], emitted as an
//! `authn_login_fail` security event, and written to the diagnostic log as
//! one line (SEC-IAM-099, SEC-OPS-028):
//!
//! ```text
//! {"ts":"2026-10-03T12:00:00.000Z","level":"error","event":"authn_login_fail","addr":"203.0.113.7","v":1,"pathway":"claim_code","cause":"credential"}
//! ```
//!
//! This is format version 1, which `v` states. The event's name and the
//! order of its fields are fixed, and `addr`, the resolved client address,
//! follows the name directly, so a fail2ban filter anchors on
//! `"event":"authn_login_fail","addr":"<HOST>"`. `pathway` is a name from
//! the inventory. `cause` is `credential` for a credential that was
//! refused, whatever was wrong with it, and `storage`, `timeout` or
//! `missing_data` for a fault, which a filter may choose to leave out. The
//! line is logged as an error so that no configured level drops it.
//! Changing any of this is a new format version.
//!
//! The security events go into the injected `SecuritySink`, which the
//! server wires to its bus; the audit log's sink stores the records
//! (WP-069). A right credential leaves nothing behind here: the session
//! that is then opened records the sign-in.
//!
//! This file is a registry: it holds only `mod` lines.

pub mod error;
pub mod guesses;
pub mod pathway;
pub mod preauth;
#[cfg(test)]
mod testing;
pub mod verify;
