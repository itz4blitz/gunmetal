//! `ID3v1` and `ID3v1.1`: the 128-octet tag at the end of an MP3 file.
//!
//! The tag is the file's last 128 octets when they start with `TAG`: a
//! title, an artist and an album of 30 octets each, a year of 4, a comment
//! of 30 and one genre octet (id3.org, `ID3v1`). `ID3v1.1` writes a zero
//! and a track number in the comment's last two octets. Text is ISO 8859-1.
//! A field ends at its first zero octet, and the spaces some writers pad
//! with are trimmed from both ends, as `TagLib` and Mutagen do.
//!
//! The parser reads a [`Window`] over the end of the file, the same tail
//! the APE parser reads ([`super::ape`]), and reports the tag's byte range
//! so the identity window can skip it (LIB-028). The genre octet is kept
//! raw; the tag mapper names it (WP-049).
//!
//! Work: one step of the [`Budget`] when a tag is found and none when there
//! is none, so a parse spends at most one step (SEC-MED-007). Nothing nests
//! inside the tag, so the parse needs no depth.

use std::num::NonZeroU8;
use std::ops::Range;

use crate::parse::{Budget, LimitKind, Limits, ParseFault, Window};
use crate::problem::{Arg, Describe, Problem, ProblemCode};
use crate::text::{self, Encoding, Text};
use crate::untrusted::Untrusted;

/// Octets in a tag.
pub const LEN: u64 = 128;

/// An `ID3v1` or `ID3v1.1` tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Id3v1Tag {
    /// Where the tag lies in the file: always its last 128 octets.
    pub range: Range<u64>,
    /// The title, at most the short-text limit.
    pub title: Text,
    /// The artist, at most the short-text limit.
    pub artist: Text,
    /// The album, at most the short-text limit.
    pub album: Text,
    /// The year as written, at most the short-text limit.
    pub year: Text,
    /// The comment, at most the long-text limit; 28 octets at most in an
    /// `ID3v1.1` tag.
    pub comment: Text,
    /// The `ID3v1.1` track number, when the comment's 29th octet is zero
    /// and its 30th is not.
    pub track: Option<NonZeroU8>,
    /// The genre octet: an index into the Winamp genre list, where 255
    /// usually means none.
    pub genre: u8,
}

/// Why the end of a file could not be read as an `ID3v1` tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Id3v1Error {
    /// The window did not hold the file's last 128 octets
    /// ([`ParseFault::Truncated`]; read them and try again), or the budget
    /// was spent ([`ParseFault::BudgetExceeded`]).
    Fault(ParseFault),
}

impl Describe for Id3v1Error {
    /// A tag at the end of the file was skipped, at the offset where its
    /// reading stopped.
    fn problem(&self) -> Problem {
        let Self::Fault(fault) = self;
        Problem {
            code: ProblemCode::EndTagSkipped,
            args: vec![
                ("tag", Arg::Name("ID3v1")),
                ("offset", Arg::Number(fault.offset())),
            ],
        }
    }
}

/// Reads the `ID3v1` tag that ends the file `window` looks into, if there
/// is one.
///
/// The window must hold the file's last 128 octets, or the whole file when
/// it is shorter.
///
/// # Errors
///
/// [`Id3v1Error::Fault`] with [`ParseFault::Truncated`] when the window
/// does not hold the file's last 128 octets, and with
/// [`ParseFault::BudgetExceeded`] when a tag is found with no step left.
pub fn find_v1(
    window: Window<'_>,
    limits: &Limits,
    budget: &mut Budget,
) -> Result<Option<Id3v1Tag>, Id3v1Error> {
    let Some((at, tag)) = locate(&window).map_err(Id3v1Error::Fault)? else {
        return Ok(None);
    };
    budget.charge(1, at).map_err(Id3v1Error::Fault)?;
    let short = cap(limits, LimitKind::ShortText);
    let [.., marker, number, genre] = *tag;
    let (comment, track) = match (marker, NonZeroU8::new(number)) {
        (0, Some(track)) => (97..125, Some(track)),
        _ => (97..127, None),
    };
    Ok(Some(Id3v1Tag {
        range: at..window.file_len,
        title: field(tag, 3..33, short),
        artist: field(tag, 33..63, short),
        album: field(tag, 63..93, short),
        year: field(tag, 93..97, short),
        comment: field(tag, comment, cap(limits, LimitKind::LongText)),
        track,
        genre,
    }))
}

/// Where the file's `ID3v1` tag starts, and its octets, when the file ends
/// with one.
///
/// # Errors
///
/// [`ParseFault::Truncated`] when the window does not hold the file's last
/// 128 octets.
pub(crate) fn locate<'a>(window: &Window<'a>) -> Result<Option<(u64, &'a [u8; 128])>, ParseFault> {
    let Some(at) = window.file_len.checked_sub(LEN) else {
        return Ok(None);
    };
    let tag = array::<128>(window, at)?;
    Ok(tag.starts_with(b"TAG").then_some((at, tag)))
}

/// The octets of `window` that start at file offset `at`.
fn rest_at<'a>(window: &Window<'a>, at: u64) -> Option<&'a [u8]> {
    at.checked_sub(window.offset)
        .and_then(|skip| usize::try_from(skip).ok())
        .and_then(|skip| window.bytes.get(skip..))
}

/// Why a read of `len` octets at `at` from `window` was truncated.
fn truncated(window: &Window<'_>, at: u64, len: u64) -> ParseFault {
    let start = window.offset;
    let end = start.saturating_add(u64::try_from(window.bytes.len()).unwrap_or(u64::MAX));
    ParseFault::Truncated {
        offset: at,
        needed: len,
        available: at
            .saturating_add(len)
            .min(end)
            .saturating_sub(at.max(start)),
    }
}

/// The `len` octets of the file from offset `at`, from `window`.
///
/// # Errors
///
/// [`ParseFault::Truncated`] when the window does not hold them all. Its
/// `available` counts how many of them the window does hold, so a window
/// that starts inside the octets, as a tail of the file may, reports the
/// octets after its start.
pub(crate) fn octets<'a>(window: &Window<'a>, at: u64, len: u64) -> Result<&'a [u8], ParseFault> {
    rest_at(window, at)
        .zip(usize::try_from(len).ok())
        .and_then(|(rest, len)| rest.get(..len))
        .ok_or_else(|| truncated(window, at, len))
}

/// The `N` octets of the file from offset `at`, from `window`.
///
/// # Errors
///
/// [`ParseFault::Truncated`] when the window does not hold them all, as
/// [`octets`] reports it.
pub(crate) fn array<'a, const N: usize>(
    window: &Window<'a>,
    at: u64,
) -> Result<&'a [u8; N], ParseFault> {
    rest_at(window, at)
        .and_then(|rest| rest.split_first_chunk())
        .map(|(head, _)| head)
        .ok_or_else(|| truncated(window, at, u64::try_from(N).unwrap_or(u64::MAX)))
}

/// A text limit as the cap the text decoder takes. Every limit is at most
/// its ceiling, which fits.
fn cap(limits: &Limits, kind: LimitKind) -> u32 {
    u32::try_from(limits.get(kind)).unwrap_or(u32::MAX)
}

/// The text field at `range` of `tag`: up to its first zero, without the
/// ASCII whitespace around it, decoded from ISO 8859-1 and capped at `cap`
/// octets of UTF-8.
fn field(tag: &[u8; 128], range: Range<usize>, cap: u32) -> Text {
    let raw = tag.get(range).unwrap_or_default();
    let ended = raw.split(|&octet| octet == 0).next().unwrap_or_default();
    text::decode(Untrusted::new(ended.trim_ascii()), Encoding::Latin1, cap)
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::*;
    use gunmetal_testkit::id3v1::{Id3v1, Padding};
    use proptest::collection::vec;
    use proptest::prelude::*;

    /// Sixteen octets standing in for the audio before a tag.
    const AUDIO: [u8; 16] = [0xFF; 16];

    /// Runs `work` on a thread with the 256 KiB stack SEC-MED-001 names, so
    /// a parse that recursed would fail here.
    fn on_small_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(262_144)
            .spawn(work)
            .expect("the test thread starts")
            .join()
            .expect("the parse returned instead of panicking")
    }

    /// `AUDIO` followed by `tag`.
    fn file(tag: &[u8]) -> Vec<u8> {
        [&AUDIO[..], tag].concat()
    }

    /// A window over the whole of `bytes`.
    fn whole(bytes: &[u8]) -> Window<'_> {
        Window {
            offset: 0,
            bytes,
            file_len: u64::try_from(bytes.len()).unwrap(),
        }
    }

    /// Finds the tag in the whole of `bytes` under the default limits and
    /// a budget of one step.
    fn find(bytes: &[u8]) -> Result<Option<Id3v1Tag>, Id3v1Error> {
        find_v1(
            whole(bytes),
            &Limits::DEFAULT,
            &mut Budget::for_input(0, 0, 1),
        )
    }

    fn text(value: &str) -> Text {
        Text {
            value: value.to_owned(),
            truncated: false,
            replaced: false,
        }
    }

    fn cut(value: &str) -> Text {
        Text {
            value: value.to_owned(),
            truncated: true,
            replaced: false,
        }
    }

    /// The tag the specification's layout test writes, as it should read.
    fn expected(range: Range<u64>) -> Id3v1Tag {
        Id3v1Tag {
            range,
            title: text("Title"),
            artist: text("Artist"),
            album: text("Album"),
            year: text("2003"),
            comment: text("Comment"),
            track: None,
            genre: 17,
        }
    }

    fn full_tag(padding: Padding) -> Vec<u8> {
        Id3v1::new()
            .title(b"Title")
            .artist(b"Artist")
            .album(b"Album")
            .year(b"2003")
            .comment(b"Comment")
            .genre(17)
            .padding(padding)
            .build()
    }

    #[test]
    fn reads_every_field_of_a_tag_padded_with_zeros() {
        assert_eq!(
            find(&file(&full_tag(Padding::Zeros))),
            Ok(Some(expected(16..144)))
        );
    }

    #[test]
    fn reads_the_same_tag_padded_with_spaces() {
        assert_eq!(
            find(&file(&full_tag(Padding::Spaces))),
            Ok(Some(expected(16..144)))
        );
    }

    #[test]
    fn reads_the_track_number_of_an_id3v1_1_tag() {
        for padding in [Padding::Zeros, Padding::Spaces] {
            let tag = Id3v1::new()
                .comment(b"Comment")
                .track(7)
                .padding(padding)
                .build();
            assert_eq!(
                find(&file(&tag)),
                Ok(Some(Id3v1Tag {
                    range: 16..144,
                    title: text(""),
                    artist: text(""),
                    album: text(""),
                    year: text(""),
                    comment: text("Comment"),
                    track: NonZeroU8::new(7),
                    genre: 0,
                })),
                "{padding:?}"
            );
        }
    }

    #[test]
    fn a_track_needs_a_zero_before_it_and_is_never_zero() {
        // No zero in the 29th octet: the whole comment is text.
        let whole_comment = Id3v1::new().comment(&[b'd'; 30]).build();
        // A zero, then a zero track: no track, and the comment ends early.
        let zero_track = Id3v1::new().comment(&[b'e'; 28]).track(0).build();
        // The largest track number.
        let last_track = Id3v1::new().comment(&[b'f'; 28]).track(255).build();
        let read = |tag: &[u8]| find(tag).map(|found| found.map(|tag| (tag.comment, tag.track)));
        assert_eq!(
            read(&whole_comment),
            Ok(Some((text(&["d"; 30].concat()), None)))
        );
        assert_eq!(
            read(&zero_track),
            Ok(Some((text(&["e"; 28].concat()), None)))
        );
        assert_eq!(
            read(&last_track),
            Ok(Some((text(&["f"; 28].concat()), NonZeroU8::new(255))))
        );
    }

    #[test]
    fn keeps_the_genre_octet_as_written() {
        for genre in [0, 1, 191, 254, 255] {
            let tag = Id3v1::new().genre(genre).build();
            assert_eq!(
                find(&tag).map(|found| found.map(|tag| tag.genre)),
                Ok(Some(genre))
            );
        }
    }

    #[test]
    fn a_field_ends_at_its_first_zero_and_loses_its_outer_spaces() {
        let tag = Id3v1::new()
            .title(b"Abc\0Def")
            .artist(b"  Two words  ")
            .album(b"\0Hidden")
            .year(b" 99 ")
            .comment(b"Tab\tand line\nfeed")
            .build();
        assert_eq!(
            find(&tag),
            Ok(Some(Id3v1Tag {
                range: 0..128,
                title: text("Abc"),
                artist: text("Two words"),
                album: text(""),
                year: text("99"),
                comment: text("Tab\tand line\nfeed"),
                track: None,
                genre: 0,
            }))
        );
    }

    /// Verifies: SEC-MED-013
    #[test]
    fn decodes_iso_8859_1_and_removes_control_characters() {
        let tag = Id3v1::new()
            .title(b"Caf\xE9")
            .artist(b"a\x1Bb\x85c\x7F")
            .album(b"\xA0\xFF")
            .build();
        let found = find(&tag).map(|found| found.map(|tag| (tag.title, tag.artist, tag.album)));
        assert_eq!(
            found,
            Ok(Some((text("Café"), text("abc"), text("\u{A0}ÿ"))))
        );
    }

    /// The caps count decoded UTF-8, so "Café" needs five octets.
    ///
    /// Verifies: SEC-MED-006, SEC-MED-013
    #[test]
    fn caps_short_fields_and_the_comment_at_their_limits() {
        let limits = Limits::DEFAULT
            .with_override(LimitKind::ShortText, 4)
            .and_then(|limits| limits.with_override(LimitKind::LongText, 5))
            .unwrap();
        let tag = Id3v1::new()
            .title(b"Title")
            .artist(b"Caf\xE9")
            .album(b"Alb")
            .year(b"2003")
            .comment(b"Comment")
            .build();
        assert_eq!(
            find_v1(whole(&tag), &limits, &mut Budget::for_input(0, 0, 1)),
            Ok(Some(Id3v1Tag {
                range: 0..128,
                title: cut("Titl"),
                artist: cut("Caf"),
                album: text("Alb"),
                year: text("2003"),
                comment: cut("Comme"),
                track: None,
                genre: 0,
            }))
        );
    }

    #[test]
    fn finds_nothing_without_the_marker() {
        let mut tag = Id3v1::new().title(b"Title").build();
        for marker in [b"tag", b"TAg", b"XAG", b"TXG", b"TAX"] {
            tag[..3].copy_from_slice(marker);
            assert_eq!(find(&tag), Ok(None), "{marker:?}");
        }
    }

    #[test]
    fn finds_nothing_in_a_file_shorter_than_a_tag() {
        let tag = Id3v1::new().build();
        assert_eq!(find(&[]), Ok(None));
        assert_eq!(find(&tag[1..]), Ok(None));
        assert_eq!(
            find(&tag).map(|found| found.map(|tag| tag.range)),
            Ok(Some(0..128))
        );
    }

    #[test]
    fn finds_only_a_tag_that_ends_the_file() {
        // A tag followed by one more octet is not where a tag must be.
        let mut bytes = Id3v1::new().build();
        bytes.push(0);
        assert_eq!(find(&bytes), Ok(None));
    }

    #[test]
    fn reads_a_tail_with_absolute_offsets() {
        let tag = full_tag(Padding::Zeros);
        let window = Window {
            offset: 872,
            bytes: &tag,
            file_len: 1_000,
        };
        assert_eq!(
            find_v1(window, &Limits::DEFAULT, &mut Budget::for_input(0, 0, 1)),
            Ok(Some(expected(872..1_000)))
        );
    }

    /// Every tail that starts inside the tag is reported at the tag's
    /// first octet, with the octets it does hold.
    ///
    /// Verifies: SEC-MED-001
    #[test]
    fn reports_every_tail_that_cuts_the_tag_at_its_start() {
        let mut bytes = [0x55; 872].to_vec();
        bytes.extend(full_tag(Padding::Zeros));
        for start in 873..=1_000 {
            let window = Window {
                offset: start,
                bytes: &bytes[usize::try_from(start).unwrap()..],
                file_len: 1_000,
            };
            assert_eq!(
                find_v1(window, &Limits::DEFAULT, &mut Budget::for_input(0, 0, 1)),
                Err(Id3v1Error::Fault(ParseFault::Truncated {
                    offset: 872,
                    needed: 128,
                    available: 1_000 - start,
                })),
                "tail from {start}"
            );
        }
    }

    /// Verifies: SEC-MED-001
    #[test]
    fn reports_a_window_that_stops_before_the_end_of_the_file() {
        let mut bytes = [0x55; 872].to_vec();
        bytes.extend(full_tag(Padding::Zeros));
        let truncated = |available| {
            Err(Id3v1Error::Fault(ParseFault::Truncated {
                offset: 872,
                needed: 128,
                available,
            }))
        };
        for (end, available) in [(999, 127), (900, 28), (872, 0), (100, 0)] {
            let window = Window {
                offset: 0,
                bytes: &bytes[..end],
                file_len: 1_000,
            };
            assert_eq!(
                find_v1(window, &Limits::DEFAULT, &mut Budget::for_input(0, 0, 1)),
                truncated(available),
                "window to {end}"
            );
        }
    }

    /// Verifies: SEC-MED-007, SEC-TM-032
    #[test]
    fn spends_one_step_on_a_tag_and_fails_without_it() {
        let bytes = file(&full_tag(Padding::Zeros));
        let mut budget = Budget::for_input(0, 0, 0);
        assert_eq!(
            find_v1(whole(&bytes), &Limits::DEFAULT, &mut budget),
            Err(Id3v1Error::Fault(ParseFault::BudgetExceeded { offset: 16 }))
        );
        let mut budget = Budget::for_input(0, 0, 3);
        assert_eq!(
            find_v1(whole(&bytes), &Limits::DEFAULT, &mut budget),
            Ok(Some(expected(16..144)))
        );
        assert_eq!(budget.remaining(), 2);
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn spends_nothing_when_there_is_no_tag_or_no_window() {
        let mut budget = Budget::for_input(0, 0, 3);
        assert_eq!(
            find_v1(whole(&AUDIO), &Limits::DEFAULT, &mut budget),
            Ok(None)
        );
        let mut no_marker = full_tag(Padding::Zeros);
        no_marker[0] = b'X';
        assert_eq!(
            find_v1(whole(&no_marker), &Limits::DEFAULT, &mut budget),
            Ok(None)
        );
        let short = Window {
            offset: 999,
            bytes: &[0],
            file_len: 1_000,
        };
        assert_eq!(
            find_v1(short, &Limits::DEFAULT, &mut budget),
            Err(Id3v1Error::Fault(ParseFault::Truncated {
                offset: 872,
                needed: 128,
                available: 1,
            }))
        );
        assert_eq!(budget.remaining(), 3);
    }

    #[test]
    fn describes_itself_as_a_skipped_end_tag_at_its_offset() {
        let errors = [
            ParseFault::Truncated {
                offset: 872,
                needed: 128,
                available: 1,
            },
            ParseFault::BudgetExceeded { offset: 16 },
        ];
        for (fault, offset) in errors.into_iter().zip([872, 16]) {
            assert_eq!(
                Id3v1Error::Fault(fault).problem(),
                Problem {
                    code: ProblemCode::EndTagSkipped,
                    args: vec![("tag", Arg::Name("ID3v1")), ("offset", Arg::Number(offset))],
                }
            );
        }
    }

    /// Independent model of [`find_v1`] under ample limits and budget:
    /// the octets are sliced by hand, cut at the first zero, trimmed of
    /// ASCII whitespace and decoded from Latin-1 with controls removed.
    fn model(window: Window<'_>) -> Result<Option<Id3v1Tag>, Id3v1Error> {
        let Some(at) = window.file_len.checked_sub(128) else {
            return Ok(None);
        };
        let start = u128::from(window.offset);
        let end = start + window.bytes.len() as u128;
        let (from, to) = (u128::from(at), u128::from(at) + 128);
        if from < start || to > end {
            let available = to.min(end).saturating_sub(from.max(start));
            return Err(Id3v1Error::Fault(ParseFault::Truncated {
                offset: at,
                needed: 128,
                available: u64::try_from(available).unwrap(),
            }));
        }
        let skip = usize::try_from(from - start).unwrap();
        let tag = &window.bytes[skip..skip + 128];
        if &tag[..3] != b"TAG" {
            return Ok(None);
        }
        let field = |range: Range<usize>| {
            let raw = &tag[range];
            let raw = &raw[..raw.iter().position(|&b| b == 0).unwrap_or(raw.len())];
            let value: String = raw
                .trim_ascii()
                .iter()
                .map(|&b| char::from(b))
                .filter(|c| !c.is_control() || *c == '\t' || *c == '\n')
                .collect();
            text(&value)
        };
        let numbered = tag[125] == 0 && tag[126] != 0;
        Ok(Some(Id3v1Tag {
            range: at..window.file_len,
            title: field(3..33),
            artist: field(33..63),
            album: field(63..93),
            year: field(93..97),
            comment: field(97..if numbered { 125 } else { 127 }),
            track: if numbered {
                NonZeroU8::new(tag[126])
            } else {
                None
            },
            genre: tag[127],
        }))
    }

    /// Checks the model on one case of each outcome, so the property below
    /// rests on a model known to agree on every branch.
    #[test]
    fn the_model_agrees_with_the_parser_on_each_outcome() {
        let mut bytes = [0x55; 872].to_vec();
        bytes.extend(full_tag(Padding::Spaces));
        let numbered = Id3v1::new().comment(b"c").track(9).build();
        let windows = [
            whole(&bytes),
            whole(&numbered),
            whole(&AUDIO),
            whole(&bytes[..999]),
            Window {
                offset: 900,
                bytes: &bytes[900..],
                file_len: 1_000,
            },
        ];
        let outcomes: Vec<_> = windows
            .iter()
            .map(|&window| {
                let found = find_v1(window, &Limits::DEFAULT, &mut Budget::for_input(0, 0, 1));
                assert_eq!(found, model(window));
                found.map(|found| found.map(|tag| tag.track))
            })
            .collect();
        assert_eq!(
            outcomes,
            [
                Ok(Some(None)),
                Ok(Some(NonZeroU8::new(9))),
                Ok(None),
                Ok(None),
                Err(Id3v1Error::Fault(ParseFault::Truncated {
                    offset: 872,
                    needed: 128,
                    available: 100,
                })),
            ]
        );
    }

    /// A window over a file that often ends in a tag: random tag bodies
    /// behind `TAG` and random octets, at random places in files of random
    /// lengths, including windows that do not reach the end.
    fn any_window() -> impl Strategy<Value = (u64, Vec<u8>, u64)> {
        let body = prop_oneof![
            vec(any::<u8>(), 125).prop_map(|body| [&b"TAG"[..], &body].concat()),
            vec(prop_oneof![Just(0_u8), Just(b' '), any::<u8>()], 125).prop_map(|body| [
                &b"TAG"[..],
                &body
            ]
            .concat()),
            vec(any::<u8>(), 0..200),
        ];
        (vec(any::<u8>(), 0..40), body, 0_u64..64, 0_u64..3).prop_map(
            |(before, body, offset, shape)| {
                let bytes = [before, body].concat();
                let len = u64::try_from(bytes.len()).unwrap();
                let file_len = match shape {
                    0 => offset + len,
                    1 => offset + len + 1 + offset,
                    _ => offset + len / 2,
                };
                (offset, bytes, file_len)
            },
        )
    }

    proptest! {
        /// Verifies: SEC-MED-001, SEC-MED-013
        #[test]
        fn returns_what_the_model_returns_for_any_window(
            (offset, bytes, file_len) in any_window(),
        ) {
            let window = Window { offset, bytes: &bytes, file_len };
            let expected = model(window);
            let owned = bytes.clone();
            let actual = on_small_stack(move || {
                let window = Window { offset, bytes: &owned, file_len };
                find_v1(window, &Limits::DEFAULT, &mut Budget::for_input(0, 0, 1))
            });
            prop_assert_eq!(actual, expected);
        }
    }
}
