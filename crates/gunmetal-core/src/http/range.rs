//! The `Range` request header (RFC 9110, sections 14.1 and 14.2).
//!
//! [`range`] decides how to answer a request for part of a representation
//! whose length is known: the whole representation (200), one range of it
//! (206), or 416 Range Not Satisfiable.
//!
//! - **One range only.** A header that names two or more ranges is answered
//!   416 (SEC-API-031, SEC-NET-050). SEC-MED-060 serves the whole
//!   representation instead; owner decision D-03 takes the stricter answer
//!   wherever two requirements disagree (applied change A-487). Counting
//!   comes before any range is judged, so the overlapping ranges of
//!   CVE-2011-3192 are refused however they are written.
//! - **A malformed header means the whole representation** (SEC-MED-060),
//!   as RFC 9110 allows a server to ignore an invalid `Range`: a header
//!   that does not follow the grammar, a unit other than `bytes`, or one
//!   range whose last position comes before its first. A suffix of an
//!   empty representation is the whole of it, which only a 200 can send.
//! - **An unsatisfiable range is answered 416**: a first position at or past
//!   the end, or a suffix of length zero.
//! - **Checked arithmetic.** A position or length larger than a `u64`
//!   holds, however many digits it takes, reads as `u64::MAX`, which lies
//!   past the end of every representation, so no number wraps
//!   (SEC-MED-060). Two such numbers read as equal.
//!
//! Optional whitespace is allowed around each range, as the list rule of
//! RFC 9110 allows around its commas, and empty list elements are skipped.
//! The unit is compared without regard to case.
//!
//! The parser reads at most [`MAX_RANGE_OCTETS`] octets, the size of the
//! whole header section the listener accepts (SEC-NET-048), so every header
//! that can reach it is judged in full. It charges one step per octet of the
//! header plus one (k = 1, c = 1 in SEC-MED-007's bound), keeps no depth
//! because a range set does not nest, and allocates nothing.

use super::syntax::{count, is_token, split_once, trim};
use crate::parse::{Budget, ParseFault};
use crate::untrusted::Untrusted;

/// The longest `Range` value read, in octets: the 16 KiB header section
/// of SEC-NET-048.
pub const MAX_RANGE_OCTETS: u64 = 16_384;

/// Steps charged per octet of the header (k in SEC-MED-007's bound).
const STEPS_PER_OCTET: u64 = 1;

/// Steps charged beyond those (c in SEC-MED-007's bound).
const FIXED_STEPS: u64 = 1;

/// The octets of a representation to send: `first` to `last` inclusive,
/// with `first <= last < len`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteRange {
    first: u64,
    last: u64,
}

impl ByteRange {
    /// The position of the first octet to send.
    #[must_use]
    pub const fn first(&self) -> u64 {
        self.first
    }

    /// The position of the last octet to send.
    #[must_use]
    pub const fn last(&self) -> u64 {
        self.last
    }

    /// How many octets to send: the `Content-Length` of the 206 response.
    /// Since `last` is below the length, this is at most `u64::MAX`.
    #[must_use]
    pub const fn octets(&self) -> u64 {
        self.last.saturating_sub(self.first).saturating_add(1)
    }
}

/// How to answer a request that carried a `Range` header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeOutcome {
    /// Send the whole representation with 200, as if no `Range` header had
    /// arrived, for the reason given.
    Whole(Ignored),
    /// Send these octets with 206.
    Partial(ByteRange),
    /// Answer 416 Range Not Satisfiable, with no body.
    NotSatisfiable(Refusal),
}

/// Why a `Range` header was ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ignored {
    /// The header is longer than [`MAX_RANGE_OCTETS`], so it was not read.
    TooLong {
        /// The header's length.
        octets: u64,
        /// The longest header read.
        max: u64,
    },
    /// The header does not follow the grammar of a byte range set.
    Malformed {
        /// Where the part that breaks the grammar starts, in octets from
        /// the start of the header.
        offset: u64,
    },
    /// The header names a unit other than `bytes`.
    OtherUnit,
    /// The one range ends before it starts.
    Backwards {
        /// Its first position.
        first: u64,
        /// Its last position.
        last: u64,
    },
    /// The representation is empty and the range is a suffix, which RFC
    /// 9110 answers with the whole (empty) representation.
    EmptyRepresentation,
    /// The parse spent its step budget (SEC-MED-007).
    Fault(ParseFault),
}

/// Why a `Range` header is answered 416.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The header names more than one range (SEC-API-031, SEC-NET-050).
    SeveralRanges {
        /// How many ranges it names.
        count: u64,
    },
    /// The range starts at or past the end of the representation.
    StartsPastEnd {
        /// Its first position.
        first: u64,
        /// The representation's length.
        len: u64,
    },
    /// The range is a suffix of length zero.
    EmptySuffix,
}

/// Decides how to answer a request whose `Range` header is `header`, for a
/// representation of `len` octets.
#[must_use]
pub fn range(header: Untrusted<&[u8]>, len: u64) -> RangeOutcome {
    let header = header.into_inner();
    let octets = count(header.len());
    if octets > MAX_RANGE_OCTETS {
        return RangeOutcome::Whole(Ignored::TooLong {
            octets,
            max: MAX_RANGE_OCTETS,
        });
    }
    read(
        header,
        len,
        &mut Budget::for_input(octets, STEPS_PER_OCTET, FIXED_STEPS),
    )
}

/// [`range`] under the budget given, once the header's length is checked.
fn read(header: &[u8], len: u64, budget: &mut Budget) -> RangeOutcome {
    match specs(header, budget) {
        Err(ignored) => RangeOutcome::Whole(ignored),
        Ok((_, count @ 2..)) => RangeOutcome::NotSatisfiable(Refusal::SeveralRanges { count }),
        Ok((spec, _)) => answer(spec, len),
    }
}

/// One range-spec of a byte range set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Spec {
    /// `first-` or `first-last`.
    From {
        /// The first position.
        first: u64,
        /// The last position, when it is given.
        last: Option<u64>,
    },
    /// `-suffix`: the last `suffix` octets.
    Suffix(u64),
}

/// Reads a `bytes` range set: its first range-spec and how many there are.
///
/// Charges the unit and its `=` together, then each list element with the
/// comma after it, so the whole header costs its trimmed length plus one.
fn specs(header: &[u8], budget: &mut Budget) -> Result<(Spec, u64), Ignored> {
    let (lead, value) = trim(header);
    let Some((unit, set)) = split_once(value, b'=') else {
        return Err(Ignored::Malformed { offset: lead });
    };
    let unit_steps = count(unit.len()).saturating_add(1);
    budget.charge(unit_steps, lead).map_err(Ignored::Fault)?;
    if !is_token(unit) {
        return Err(Ignored::Malformed { offset: lead });
    }
    if !unit.eq_ignore_ascii_case(b"bytes") {
        return Err(Ignored::OtherUnit);
    }
    let start = lead.saturating_add(unit_steps);
    let mut at = start;
    let mut first = None;
    let mut specs = 0_u64;
    for piece in set.split(|&byte| byte == b',') {
        let offset = at;
        let steps = count(piece.len()).saturating_add(1);
        at = at.saturating_add(steps);
        budget.charge(steps, offset).map_err(Ignored::Fault)?;
        let (inner, element) = trim(piece);
        if element.is_empty() {
            continue;
        }
        let malformed = Ignored::Malformed {
            offset: offset.saturating_add(inner),
        };
        let spec = spec(element).ok_or(malformed)?;
        specs = specs.saturating_add(1);
        first = first.or(Some(spec));
    }
    first
        .map(|spec| (spec, specs))
        .ok_or(Ignored::Malformed { offset: start })
}

/// Reads one range-spec, or `None` when it is not one.
fn spec(element: &[u8]) -> Option<Spec> {
    let (first, last) = split_once(element, b'-')?;
    if first.is_empty() {
        return number(last).map(Spec::Suffix);
    }
    let first = number(first)?;
    if last.is_empty() {
        return Some(Spec::From { first, last: None });
    }
    Some(Spec::From {
        first,
        last: Some(number(last)?),
    })
}

/// Reads one or more decimal digits, saturating at `u64::MAX`, or `None`
/// for anything else.
fn number(digits: &[u8]) -> Option<u64> {
    if digits.is_empty() {
        return None;
    }
    digits.iter().try_fold(0_u64, |value, &digit| {
        char::from(digit)
            .to_digit(10)
            .map(|digit| value.saturating_mul(10).saturating_add(u64::from(digit)))
    })
}

/// How to answer the one range `spec` of a representation of `len` octets.
fn answer(spec: Spec, len: u64) -> RangeOutcome {
    // Only read where `len` is at least one.
    let end = len.saturating_sub(1);
    match spec {
        Spec::From {
            first,
            last: Some(last),
        } if last < first => RangeOutcome::Whole(Ignored::Backwards { first, last }),
        Spec::From { first, .. } if first >= len => {
            RangeOutcome::NotSatisfiable(Refusal::StartsPastEnd { first, len })
        }
        Spec::From { first, last } => RangeOutcome::Partial(ByteRange {
            first,
            last: last.map_or(end, |last| last.min(end)),
        }),
        Spec::Suffix(0) => RangeOutcome::NotSatisfiable(Refusal::EmptySuffix),
        Spec::Suffix(_) if len == 0 => RangeOutcome::Whole(Ignored::EmptyRepresentation),
        Spec::Suffix(suffix) => RangeOutcome::Partial(ByteRange {
            first: len.saturating_sub(suffix),
            last: end,
        }),
    }
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::*;
    use proptest::collection::vec;
    use proptest::prelude::*;
    use proptest::sample::select;

    /// The stack size SEC-MED-001 names, in octets.
    const SMALL_STACK: usize = 262_144;

    /// Runs `work` on a 256 KiB stack so a recursive Range parse fails its
    /// test instead of passing on the runner's larger stack (SEC-MED-001).
    fn on_small_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(SMALL_STACK)
            .spawn(work)
            .expect("the test thread starts")
            .join()
            .expect("the code under test returned instead of panicking")
    }

    fn outcome(header: &str, len: u64) -> RangeOutcome {
        range(Untrusted::new(header.as_bytes()), len)
    }

    const fn part(first: u64, last: u64) -> RangeOutcome {
        RangeOutcome::Partial(ByteRange { first, last })
    }

    const fn whole(ignored: Ignored) -> RangeOutcome {
        RangeOutcome::Whole(ignored)
    }

    const fn malformed(offset: u64) -> RangeOutcome {
        RangeOutcome::Whole(Ignored::Malformed { offset })
    }

    const fn refused(refusal: Refusal) -> RangeOutcome {
        RangeOutcome::NotSatisfiable(refusal)
    }

    const fn several(count: u64) -> RangeOutcome {
        RangeOutcome::NotSatisfiable(Refusal::SeveralRanges { count })
    }

    /// `piece` written `times` times.
    fn repeated(piece: &str, times: usize) -> String {
        (0..times).map(|_| piece).collect()
    }

    /// A budget too large to run out, to measure what a parse spends.
    fn unlimited() -> Budget {
        Budget::for_input(0, 0, u64::MAX)
    }

    #[test]
    fn a_range_reports_its_ends_and_its_length() {
        let cases = [
            (ByteRange { first: 0, last: 0 }, (0, 0, 1)),
            (ByteRange { first: 5, last: 9 }, (5, 9, 5)),
            (
                ByteRange {
                    first: 0,
                    last: u64::MAX - 1,
                },
                (0, u64::MAX - 1, u64::MAX),
            ),
        ];
        for (range, expected) in cases {
            assert_eq!((range.first(), range.last(), range.octets()), expected);
        }
    }

    /// Verifies: SEC-MED-060
    #[test]
    fn serves_one_satisfiable_range() {
        let cases = [
            ("bytes=0-", 1_000, part(0, 999)),
            ("bytes=0-0", 1_000, part(0, 0)),
            ("bytes=100-199", 1_000, part(100, 199)),
            ("bytes=5-5", 1_000, part(5, 5)),
            ("bytes=5-6", 1_000, part(5, 6)),
            ("bytes=998-999", 1_000, part(998, 999)),
            ("bytes=999-", 1_000, part(999, 999)),
            ("bytes=999-999", 1_000, part(999, 999)),
            ("bytes=0-998", 1_000, part(0, 998)),
            ("bytes=0-999", 1_000, part(0, 999)),
            ("bytes=0-1000", 1_000, part(0, 999)),
            ("bytes=0-5000", 1_000, part(0, 999)),
            ("bytes=-500", 1_000, part(500, 999)),
            ("bytes=-999", 1_000, part(1, 999)),
            ("bytes=-1000", 1_000, part(0, 999)),
            ("bytes=-1001", 1_000, part(0, 999)),
            ("bytes=-500", 300, part(0, 299)),
            ("bytes=-1", 1, part(0, 0)),
            ("bytes=0-", 1, part(0, 0)),
            ("bytes=0-", u64::MAX, part(0, u64::MAX - 1)),
            ("bytes=-1", u64::MAX, part(u64::MAX - 1, u64::MAX - 1)),
        ];
        for (header, len, expected) in cases {
            assert_eq!(outcome(header, len), expected, "{header} of {len}");
        }
    }

    /// Two or more ranges are answered 416, however they are written and
    /// whatever they would select. SEC-MED-060 would serve the whole
    /// representation; D-03 takes the stricter answer.
    ///
    /// Verifies: SEC-API-031, SEC-NET-050
    #[test]
    fn answers_416_to_two_or_more_ranges() {
        let cases = [
            ("bytes=0-1,5-9", several(2)),
            ("bytes=0-0,0-0", several(2)),
            ("bytes=0-,-1", several(2)),
            ("bytes=-1,-1,-1", several(3)),
            ("bytes=0-1 , 5-9", several(2)),
            ("bytes=0-1,\t5-9", several(2)),
            ("bytes=0-1,,5-9,", several(2)),
            ("bytes=5000-6000,7000-", several(2)),
            ("bytes=9-5,0-1", several(2)),
            ("BYTES=0-1,5-9", several(2)),
        ];
        for (header, expected) in cases {
            assert_eq!(outcome(header, 1_000), expected, "{header}");
        }
    }

    /// The killapache request of CVE-2011-3192: `0-` and then `5-0` to
    /// `5-1299`, 1,301 overlapping ranges in all, some of them backwards.
    ///
    /// Verifies: SEC-NET-050, SEC-API-031
    #[test]
    fn answers_416_to_the_overlapping_ranges_of_cve_2011_3192() {
        let ranges: Vec<String> = (0..1_300).map(|last| format!("5-{last}")).collect();
        let header = format!("bytes=0-,{}", ranges.join(","));
        assert_eq!(outcome(&header, 1_000_000), several(1_301));
        assert_eq!(outcome(&header, 0), several(1_301));
    }

    /// Verifies: SEC-MED-060
    #[test]
    fn answers_416_to_an_unsatisfiable_range() {
        let cases = [
            (
                "bytes=1000-",
                1_000,
                refused(Refusal::StartsPastEnd {
                    first: 1_000,
                    len: 1_000,
                }),
            ),
            (
                "bytes=1000-2000",
                1_000,
                refused(Refusal::StartsPastEnd {
                    first: 1_000,
                    len: 1_000,
                }),
            ),
            (
                "bytes=1001-1001",
                1_000,
                refused(Refusal::StartsPastEnd {
                    first: 1_001,
                    len: 1_000,
                }),
            ),
            (
                "bytes=0-",
                0,
                refused(Refusal::StartsPastEnd { first: 0, len: 0 }),
            ),
            (
                "bytes=0-0",
                0,
                refused(Refusal::StartsPastEnd { first: 0, len: 0 }),
            ),
            ("bytes=-0", 1_000, refused(Refusal::EmptySuffix)),
            ("bytes=-0", 0, refused(Refusal::EmptySuffix)),
            ("bytes=-000", 1_000, refused(Refusal::EmptySuffix)),
        ];
        for (header, len, expected) in cases {
            assert_eq!(outcome(header, len), expected, "{header} of {len}");
        }
    }

    /// A suffix of an empty representation is the whole of it, which only
    /// a 200 can send.
    #[test]
    fn serves_an_empty_representation_whole_for_a_suffix() {
        assert_eq!(outcome("bytes=-5", 0), whole(Ignored::EmptyRepresentation));
        assert_eq!(outcome("bytes=-1", 0), whole(Ignored::EmptyRepresentation));
    }

    /// Verifies: SEC-MED-060
    #[test]
    fn serves_the_whole_representation_for_a_malformed_header() {
        let cases = [
            ("", malformed(0)),
            ("bytes", malformed(0)),
            ("  bytes", malformed(2)),
            ("=0-1", malformed(0)),
            ("by tes=0-1", malformed(0)),
            ("bytes =0-1", malformed(0)),
            ("\"bytes\"=0-1", malformed(0)),
            ("bytes=", malformed(6)),
            ("bytes=,", malformed(6)),
            ("bytes= , ,", malformed(6)),
            ("bytes=x", malformed(6)),
            ("bytes=0", malformed(6)),
            ("bytes=-", malformed(6)),
            ("bytes=--5", malformed(6)),
            ("bytes=0--1", malformed(6)),
            ("bytes=1-2-3", malformed(6)),
            ("bytes=a-b", malformed(6)),
            ("bytes=0x10-", malformed(6)),
            ("bytes=+5-", malformed(6)),
            ("bytes=5-+6", malformed(6)),
            ("bytes=0 -1", malformed(6)),
            ("bytes=0- 1", malformed(6)),
            ("bytes=0-1;x", malformed(6)),
            ("bytes=0-1,x", malformed(10)),
            ("bytes=0-1, x", malformed(11)),
            ("bytes=0-1,5-9,x", malformed(14)),
            ("bytes=0-1,\t\tx", malformed(12)),
            (" bytes=0-1,x", malformed(11)),
            ("bytes=0-1\"", malformed(6)),
            ("bytes=\u{663}-", malformed(6)),
        ];
        for (header, expected) in cases {
            assert_eq!(outcome(header, 1_000), expected, "{header:?}");
        }
        let invalid_utf8 = range(Untrusted::new(b"bytes=\xff-"), 1_000);
        assert_eq!(invalid_utf8, malformed(6));
    }

    /// RFC 9110 calls a range whose last position comes before its first
    /// invalid, and a server may ignore an invalid header.
    ///
    /// Verifies: SEC-MED-060
    #[test]
    fn serves_the_whole_representation_for_a_backwards_range() {
        let cases = [
            ("bytes=500-100", 500, 100),
            ("bytes=6-5", 6, 5),
            ("bytes=1-0", 1, 0),
            ("bytes=2000-1000", 2_000, 1_000),
        ];
        for (header, first, last) in cases {
            assert_eq!(
                outcome(header, 1_000),
                whole(Ignored::Backwards { first, last }),
                "{header}"
            );
        }
    }

    /// Positions and lengths of 30 digits, and leading zeros, read without
    /// wrapping.
    ///
    /// Verifies: SEC-MED-060
    #[test]
    fn reads_numbers_of_any_length_with_checked_arithmetic() {
        let thirty_nines = "999999999999999999999999999999";
        let cases = [
            (format!("bytes=0-{thirty_nines}"), part(0, 999)),
            (format!("bytes=-{thirty_nines}"), part(0, 999)),
            (
                format!("bytes={thirty_nines}-"),
                refused(Refusal::StartsPastEnd {
                    first: u64::MAX,
                    len: 1_000,
                }),
            ),
            (
                format!("bytes={thirty_nines}-{thirty_nines}"),
                refused(Refusal::StartsPastEnd {
                    first: u64::MAX,
                    len: 1_000,
                }),
            ),
            (
                format!("bytes={thirty_nines}-5"),
                whole(Ignored::Backwards {
                    first: u64::MAX,
                    last: 5,
                }),
            ),
            (
                String::from("bytes=000000000000000000000000000005-6"),
                part(5, 6),
            ),
            (
                String::from("bytes=0-000000000000000000000000000006"),
                part(0, 6),
            ),
            (
                String::from("bytes=18446744073709551615-"),
                refused(Refusal::StartsPastEnd {
                    first: u64::MAX,
                    len: 1_000,
                }),
            ),
            (
                String::from("bytes=18446744073709551616-"),
                refused(Refusal::StartsPastEnd {
                    first: u64::MAX,
                    len: 1_000,
                }),
            ),
        ];
        for (header, expected) in cases {
            assert_eq!(outcome(&header, 1_000), expected, "{header}");
        }
        assert_eq!(
            outcome(&format!("bytes=0-{thirty_nines}"), u64::MAX),
            part(0, u64::MAX - 1)
        );
        assert_eq!(
            outcome("bytes=18446744073709551614-", u64::MAX),
            part(u64::MAX - 1, u64::MAX - 1)
        );
    }

    #[test]
    fn reads_the_unit_without_regard_to_case_and_ignores_other_units() {
        let cases = [
            ("bytes=0-1", part(0, 1)),
            ("BYTES=0-1", part(0, 1)),
            ("Bytes=0-1", part(0, 1)),
            ("bYtEs=0-1", part(0, 1)),
            ("items=0-1", whole(Ignored::OtherUnit)),
            ("byte=0-1", whole(Ignored::OtherUnit)),
            ("bytess=0-1", whole(Ignored::OtherUnit)),
            ("none=", whole(Ignored::OtherUnit)),
            ("items=a,b,c", whole(Ignored::OtherUnit)),
        ];
        for (header, expected) in cases {
            assert_eq!(outcome(header, 1_000), expected, "{header}");
        }
    }

    #[test]
    fn allows_whitespace_around_ranges_and_skips_empty_elements() {
        let cases = [
            (" bytes=0-1 ", part(0, 1)),
            ("\tbytes=0-1\t", part(0, 1)),
            ("bytes= 0-1", part(0, 1)),
            ("bytes=0-1 ", part(0, 1)),
            ("bytes=,0-1", part(0, 1)),
            ("bytes=0-1,", part(0, 1)),
            ("bytes= , 0-1 , ", part(0, 1)),
            ("bytes=,,,0-1,,,", part(0, 1)),
        ];
        for (header, expected) in cases {
            assert_eq!(outcome(header, 1_000), expected, "{header:?}");
        }
    }

    /// A header of 16 KiB is read in full; one octet more is not read at
    /// all.
    ///
    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn reads_a_header_at_the_octet_limit_and_ignores_one_past_it() {
        let at_limit = format!("bytes={}5-6", repeated("0", 16_375));
        assert_eq!(at_limit.len(), 16_384);
        assert_eq!(outcome(&at_limit, 1_000), part(5, 6));
        let past_limit = format!("bytes={}5-6", repeated("0", 16_376));
        assert_eq!(
            outcome(&past_limit, 1_000),
            whole(Ignored::TooLong {
                octets: 16_385,
                max: 16_384,
            })
        );
        let ranges_at_limit = format!("bytes=0-0{}", repeated(",0-", 5_459));
        assert_eq!(ranges_at_limit.len(), 16_386);
        assert_eq!(outcome(&ranges_at_limit[..16_384], 1_000), several(5_459));
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn the_budget_runs_out_at_exactly_the_step_it_should() {
        // "bytes=" costs 6 steps at offset 0, then "0-1" and "5-9" cost 4
        // each, at offsets 6 and 10: 14 steps for 13 octets.
        let header = b"bytes=0-1,5-9";
        assert_eq!(
            read(header, 1_000, &mut Budget::for_input(0, 0, 14)),
            several(2)
        );
        let cases = [(13, 10), (10, 10), (9, 6), (6, 6), (5, 0), (0, 0)];
        for (steps, offset) in cases {
            assert_eq!(
                read(header, 1_000, &mut Budget::for_input(0, 0, steps)),
                whole(Ignored::Fault(ParseFault::BudgetExceeded { offset })),
                "{steps} steps"
            );
        }
        // Leading whitespace moves every offset along.
        assert_eq!(
            read(b"  bytes=0-1", 1_000, &mut Budget::for_input(0, 0, 9)),
            whole(Ignored::Fault(ParseFault::BudgetExceeded { offset: 8 }))
        );
        assert_eq!(
            read(b"  bytes=0-1", 1_000, &mut Budget::for_input(0, 0, 10)),
            part(0, 1)
        );
    }

    /// Verifies: SEC-MED-007, SEC-MED-008
    #[test]
    fn spends_one_step_per_octet_plus_one_on_separators_alone() {
        for header in ["bytes=,,,,,,,,", "bytes=", "bytes= , , ,"] {
            let mut budget = unlimited();
            assert_eq!(read(header.as_bytes(), 1_000, &mut budget), malformed(6));
            assert_eq!(u64::MAX - budget.remaining(), header.len() as u64 + 1);
        }
        let mut budget = unlimited();
        assert_eq!(read(b"bytes", 1_000, &mut budget), malformed(0));
        assert_eq!(budget.remaining(), u64::MAX);
    }

    /// A range set does not nest, so the parser keeps no depth and never
    /// recurses: text that looks like deep nesting comes back as a typed
    /// result on a 256 KiB stack.
    ///
    /// Verifies: SEC-MED-005, SEC-MED-001
    #[test]
    fn never_recurses_on_text_that_looks_deeply_nested() {
        for opening in ["(", "[", "{", "\"", "\\", "<", "-", "bytes="] {
            let header = format!("bytes={}", repeated(opening, 16_378 / opening.len()));
            assert!(header.len() > 16_000 && header.len() <= 16_384, "{opening}");
            let answer = on_small_stack(move || outcome(&header, 1_000));
            assert_eq!(answer, malformed(6), "{opening}");
        }
    }

    /// A range-spec as a test writes it: decimal text that may run past
    /// what a `u64` holds.
    #[derive(Debug, Clone)]
    enum Written {
        /// `first-` or `first-last`.
        From(String, Option<String>),
        /// `-suffix`.
        Suffix(String),
    }

    impl Written {
        fn text(&self) -> String {
            match self {
                Self::From(first, None) => format!("{first}-"),
                Self::From(first, Some(last)) => format!("{first}-{last}"),
                Self::Suffix(suffix) => format!("-{suffix}"),
            }
        }
    }

    /// Decimal numbers: zero, small ones, any `u64`, up to 38 digits, and
    /// small ones with leading zeros.
    fn number() -> impl Strategy<Value = String> {
        prop_oneof![
            select(vec!["0", "00", "1"]).prop_map(String::from),
            (0_u64..2_000).prop_map(|n| n.to_string()),
            any::<u64>().prop_map(|n| n.to_string()),
            "[0-9]{1,38}",
            (0_u64..2_000, 1_usize..30)
                .prop_map(|(n, zeros)| format!("{}{n}", repeated("0", zeros))),
        ]
    }

    fn written() -> impl Strategy<Value = Written> {
        prop_oneof![
            (number(), proptest::option::of(number()))
                .prop_map(|(first, last)| Written::From(first, last)),
            number().prop_map(Written::Suffix),
        ]
    }

    /// Representation lengths, with the edges forced in.
    fn length() -> impl Strategy<Value = u64> {
        prop_oneof![0_u64..3_000, any::<u64>(), Just(0), Just(1), Just(u64::MAX)]
    }

    /// What RFC 9110 says one range asks of a representation of `len`
    /// octets, worked out with `u128` arithmetic and the standard library's
    /// own number parser. A number past `u64::MAX` counts as `u64::MAX`.
    fn reference(spec: &Written, len: u64) -> RangeOutcome {
        let read = |text: &str| u64::try_from(text.parse::<u128>().unwrap()).unwrap_or(u64::MAX);
        match spec {
            Written::From(first, last) => {
                let first = read(first);
                let last = last.as_deref().map(read);
                match last {
                    Some(last) if last < first => whole(Ignored::Backwards { first, last }),
                    _ if first >= len => refused(Refusal::StartsPastEnd { first, len }),
                    _ => part(first, last.unwrap_or(u64::MAX).min(len - 1)),
                }
            }
            Written::Suffix(suffix) => match read(suffix) {
                0 => refused(Refusal::EmptySuffix),
                _ if len == 0 => whole(Ignored::EmptyRepresentation),
                suffix => part(len - suffix.min(len), len - 1),
            },
        }
    }

    /// The ways the unit may be written.
    fn unit() -> impl Strategy<Value = &'static str> {
        select(vec!["bytes", "BYTES", "Bytes", "bYtEs"])
    }

    /// What may stand before or after a range: whitespace and empty
    /// elements.
    fn padding() -> impl Strategy<Value = &'static str> {
        select(vec!["", " ", "\t", " \t ", ",", " , ", ",,", ", \t,"])
    }

    /// Whitespace around the whole header.
    fn outer() -> impl Strategy<Value = &'static str> {
        select(vec!["", " ", "\t", "  "])
    }

    /// Short headers made of the characters a range set uses, so that
    /// satisfiable ranges turn up among the malformed ones.
    fn rangey() -> impl Strategy<Value = Vec<u8>> {
        "(bytes|BYTES|items)?=?[0-9, \\-]{0,24}".prop_map(String::into_bytes)
    }

    /// Long, repetitive headers: the shape that makes a parser do the most
    /// work per octet.
    fn repetitive() -> impl Strategy<Value = Vec<u8>> {
        (
            select(vec![",", "0-", " ", "-", "0-0,", "9", ", ,", "\""]),
            0_usize..3_000,
        )
            .prop_map(|(piece, times)| format!("bytes={}", repeated(piece, times)).into_bytes())
    }

    proptest! {
        /// Verifies: SEC-MED-060
        #[test]
        fn agrees_with_rfc_9110_on_any_one_range(
            unit in unit(),
            spec in written(),
            before in padding(),
            after in padding(),
            outside in outer(),
            len in length(),
        ) {
            let header = format!("{outside}{unit}={before}{}{after}{outside}", spec.text());
            prop_assert_eq!(outcome(&header, len), reference(&spec, len), "{}", header);
        }

        /// Verifies: SEC-NET-050, SEC-API-031
        #[test]
        fn refuses_any_header_that_names_several_ranges(
            unit in unit(),
            specs in vec((written(), padding()), 2..8),
            len in length(),
        ) {
            let set: Vec<String> = specs
                .iter()
                .map(|(spec, padding)| format!("{padding}{}", spec.text()))
                .collect();
            let header = format!("{unit}={}", set.join(","));
            prop_assert_eq!(outcome(&header, len), several(specs.len() as u64), "{}", header);
        }

        /// Whatever the header, a range the parser serves lies inside the
        /// representation, and it returns rather than panics.
        ///
        /// Verifies: SEC-MED-001, SEC-MED-060
        #[test]
        fn returns_for_any_header_and_serves_only_octets_that_exist(
            header in prop_oneof![vec(any::<u8>(), 0..64), rangey()],
            len in length(),
        ) {
            let answer = on_small_stack(move || range(Untrusted::new(&header), len));
            if let RangeOutcome::Partial(served) = answer {
                prop_assert!(served.first() <= served.last(), "{:?}", served);
                prop_assert!(served.last() < len, "{:?} of {}", served, len);
                prop_assert_eq!(served.octets(), served.last() - served.first() + 1);
            }
        }

        /// Verifies: SEC-MED-007, SEC-MED-008, SEC-TM-032
        #[test]
        fn spends_at_most_one_step_per_octet_plus_one(
            header in prop_oneof![vec(any::<u8>(), 0..64), rangey(), repetitive()],
            len in length(),
        ) {
            let mut budget = unlimited();
            let answer = read(&header, len, &mut budget);
            let spent = u64::MAX - budget.remaining();
            prop_assert!(spent <= header.len() as u64 + 1, "{} steps for {} octets", spent, header.len());
            let mut exact = Budget::for_input(header.len() as u64, STEPS_PER_OCTET, FIXED_STEPS);
            prop_assert_eq!(read(&header, len, &mut exact), answer);
        }
    }
}
