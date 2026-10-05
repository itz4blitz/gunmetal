//! Frame bodies: text in each encoding, user text, involved people,
//! comments, lyrics, synchronised lyrics, pictures and unique file
//! identifiers. Every other frame is kept raw.
//!
//! Strings in a body end at a terminator: one zero octet, or two at an
//! even offset in UTF-16. A string the body ends without terminating runs
//! to the end of the body, and the fields after it are empty. A body that
//! ends before a fixed-size field, or names an encoding that does not
//! exist, is kept raw and recorded as malformed.
//!
//! # What one tag may hold
//!
//! A value with no characters takes one octet of the tag and many times
//! that in memory, and so does a line of synchronised lyrics. So both are
//! counted across the whole tag, as its frames and its pictures are
//! (SEC-MED-006, SEC-TM-032):
//!
//! - The values of every text, user text and involved people frame count
//!   together against [`LimitKind::Children`]. The frame that reaches the
//!   limit keeps the values read before it, and each frame from there on
//!   keeps none and records a [`ParseFault::LimitExceeded`] of its own.
//! - The lines of every synchronised lyrics frame count together against
//!   [`LimitKind::LyricsLines`]. The frame that reaches the limit keeps the
//!   lines read before it and says so in [`SyncedLyrics::truncated`], as
//!   does each frame from there on that holds a line.
//!
//! Under the default limits one tag therefore yields at most 65,536 values
//! and 10,000 lines, whatever its frames declare. The limits table has no
//! row for either total; these are the limits the same counts were already
//! held to within one frame.
//!
//! Reading the values or the lines of a frame takes at least one octet of
//! the frame per turn, and every octet of a frame is charged to the step
//! budget before its body is decoded, so the frame's charge covers the
//! work (SEC-MED-007).
//!
//! [`ParseFault::LimitExceeded`]: crate::parse::ParseFault::LimitExceeded

use crate::parse::LimitKind;
use crate::text::{self, Encoding, Text};
use crate::untrusted::Untrusted;

use super::frame::{
    Credit, FrameBody, FrameId, LanguageText, PictureRef, SyncedLyrics, SyncedText,
};
use super::frames::Reader;
use super::source::Source;
use super::tag::TagProblem;

/// The kinds of frame body decoded here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Text,
    UserText,
    People,
    Comment,
    Lyrics,
    SyncedLyrics,
    Picture,
    PictureV22,
    Ufid,
    Raw,
}

/// Which kind of body the frame `id` holds. A text information frame has
/// an identifier that starts with `T`, and four more hold the same body
/// under another letter: `GRP1`, the grouping iTunes writes, and `XSOT`,
/// `XSOP` and `XSOA`, the sort names 2.3 tags hold.
fn kind(id: FrameId) -> Kind {
    match id {
        FrameId::Four(id) => match &id {
            b"TXXX" => Kind::UserText,
            b"TIPL" | b"TMCL" | b"IPLS" => Kind::People,
            [b'T', ..] | b"GRP1" | b"XSOA" | b"XSOP" | b"XSOT" => Kind::Text,
            b"COMM" => Kind::Comment,
            b"USLT" => Kind::Lyrics,
            b"SYLT" => Kind::SyncedLyrics,
            b"APIC" => Kind::Picture,
            b"UFID" => Kind::Ufid,
            _ => Kind::Raw,
        },
        FrameId::Three(id) => match &id {
            b"TXX" => Kind::UserText,
            b"IPL" => Kind::People,
            [b'T', ..] => Kind::Text,
            b"COM" => Kind::Comment,
            b"ULT" => Kind::Lyrics,
            b"SLT" => Kind::SyncedLyrics,
            b"PIC" => Kind::PictureV22,
            b"UFI" => Kind::Ufid,
            _ => Kind::Raw,
        },
    }
}

/// The encoding an encoding octet names, and the width of its
/// terminator. Encodings 2 and 3 are defined only from 2.4, but writers
/// use them in earlier versions too, so they are read in every version.
fn encoding(octet: u8) -> Option<(Encoding, usize)> {
    match octet {
        0 => Some((Encoding::Latin1, 1)),
        1 => Some((Encoding::Utf16Bom, 2)),
        2 => Some((Encoding::Utf16Be, 2)),
        3 => Some((Encoding::Utf8, 1)),
        _ => None,
    }
}

/// Splits `octets` at the first terminator of `width` zero octets, which
/// for UTF-16 starts at an even offset: the string before it, and the
/// octets after it, or `None` when there is no terminator.
pub(super) fn terminated(octets: &[u8], width: usize) -> (&[u8], Option<&[u8]>) {
    let found = octets
        .chunks_exact(width)
        .position(|unit| unit.iter().all(|&octet| octet == 0));
    let Some(index) = found else {
        return (octets, None);
    };
    let at = index.saturating_mul(width);
    (
        octets.get(..at).unwrap_or_default(),
        Some(octets.get(at.saturating_add(width)..).unwrap_or_default()),
    )
}

/// A text with no characters.
fn empty() -> Text {
    Text {
        value: String::new(),
        truncated: false,
        replaced: false,
    }
}

impl Reader<'_> {
    /// Decodes the body of frame `id`, whose header is at `offset`, from
    /// `payload`, or keeps it raw and records it as malformed.
    pub(super) fn body(&mut self, id: FrameId, offset: u64, payload: Source<'_>) -> FrameBody {
        let data = || payload.decode();
        let decoded = match kind(id) {
            Kind::Raw => return FrameBody::Raw(payload.span()),
            Kind::Text => self.text(&data(), offset),
            Kind::UserText => self.user_text(&data(), offset),
            Kind::People => self.people(&data(), offset),
            Kind::Comment => self
                .language_text(&data(), LimitKind::LongText)
                .map(FrameBody::Comment),
            Kind::Lyrics => self
                .language_text(&data(), LimitKind::LyricsBytes)
                .map(FrameBody::Lyrics),
            Kind::SyncedLyrics => self.synced_lyrics(&data()).map(FrameBody::SyncedLyrics),
            Kind::Picture => self.picture(&data(), offset, payload, false),
            Kind::PictureV22 => self.picture(&data(), offset, payload, true),
            Kind::Ufid => Some(self.ufid(&data())),
        };
        if let Some(body) = decoded {
            return body;
        }
        self.problems.push(TagProblem::Malformed { offset, id });
        FrameBody::Raw(payload.span())
    }

    /// `octets` decoded as `encoding`, capped at the limit `cap`.
    fn decode(&self, octets: &[u8], encoding: Encoding, cap: LimitKind) -> Text {
        let cap = u32::try_from(self.limits.get(cap)).unwrap_or(u32::MAX);
        text::decode(Untrusted::new(octets), encoding, cap)
    }

    /// The strings of `octets` up to each terminator, every one when `all`
    /// is set and only the first otherwise. The values of every frame of
    /// the tag count together against the child limit; reaching it is
    /// recorded against the frame at `offset`, which keeps the values read
    /// before it.
    fn values(
        &mut self,
        mut octets: &[u8],
        (encoding, width): (Encoding, usize),
        cap: LimitKind,
        all: bool,
        offset: u64,
    ) -> Vec<Text> {
        let mut values = Vec::new();
        while !octets.is_empty() {
            let count = self.values.saturating_add(1);
            if let Err(fault) = self.limits.check(LimitKind::Children, count, offset) {
                self.problems.push(TagProblem::Fault(fault));
                break;
            }
            let (value, after) = terminated(octets, width);
            values.push(self.decode(value, encoding, cap));
            self.values = count;
            match after {
                Some(after) if all => octets = after,
                _ => break,
            }
        }
        values
    }

    /// A text information frame: the encoding, then its values.
    fn text(&mut self, data: &[u8], offset: u64) -> Option<FrameBody> {
        let (&octet, rest) = data.split_first()?;
        let encoding = encoding(octet)?;
        let all = self.major == 4;
        Some(FrameBody::Text(self.values(
            rest,
            encoding,
            LimitKind::ShortText,
            all,
            offset,
        )))
    }

    /// A user-defined text frame: the encoding, the description, then its
    /// values.
    fn user_text(&mut self, data: &[u8], offset: u64) -> Option<FrameBody> {
        let (&octet, rest) = data.split_first()?;
        let encoding = encoding(octet)?;
        let (description, after) = terminated(rest, encoding.1);
        let description = self.decode(description, encoding.0, LimitKind::ShortText);
        let all = self.major == 4;
        let values = self.values(
            after.unwrap_or_default(),
            encoding,
            LimitKind::LongText,
            all,
            offset,
        );
        Some(FrameBody::UserText {
            description,
            values,
        })
    }

    /// An involved people list: the encoding, then roles and names in
    /// turn, separated by terminators in every version.
    fn people(&mut self, data: &[u8], offset: u64) -> Option<FrameBody> {
        let (&octet, rest) = data.split_first()?;
        let encoding = encoding(octet)?;
        let mut values = self
            .values(rest, encoding, LimitKind::ShortText, true, offset)
            .into_iter();
        let mut credits = Vec::new();
        while let Some(role) = values.next() {
            let name = values.next().unwrap_or_else(empty);
            credits.push(Credit { role, name });
        }
        Some(FrameBody::People(credits))
    }

    /// A comment or unsynchronised lyrics: the encoding, the language, the
    /// description, then the text, capped at `cap`.
    fn language_text(&self, data: &[u8], cap: LimitKind) -> Option<LanguageText> {
        let (&octet, rest) = data.split_first()?;
        let (encoding, width) = encoding(octet)?;
        let (&language, rest) = rest.split_first_chunk::<3>()?;
        let (description, after) = terminated(rest, width);
        let (text, _) = terminated(after.unwrap_or_default(), width);
        Some(LanguageText {
            language,
            description: self.decode(description, encoding, LimitKind::ShortText),
            text: self.decode(text, encoding, cap),
        })
    }

    /// Synchronised lyrics: the encoding, the language, the time stamp
    /// format, the content type, the description, then lines of terminated
    /// text, each followed by its 32-bit time stamp. The lines of every
    /// frame of the tag count together against the lyrics line limit; lines
    /// past it are dropped and the result says so. The lines of a frame
    /// that is not kept do not count.
    fn synced_lyrics(&mut self, data: &[u8]) -> Option<SyncedLyrics> {
        let (&octet, rest) = data.split_first()?;
        let (encoding, width) = encoding(octet)?;
        let (&[l0, l1, l2, timestamp_format, content_type], rest) =
            rest.split_first_chunk::<5>()?;
        let (description, after) = terminated(rest, width);
        let mut rest = after.unwrap_or_default();
        let mut lines = Vec::new();
        let mut kept = self.lines;
        let mut truncated = false;
        while !rest.is_empty() {
            let count = kept.saturating_add(1);
            if self.limits.check(LimitKind::LyricsLines, count, 0).is_err() {
                truncated = true;
                break;
            }
            let (line, after) = terminated(rest, width);
            let (&time, after) = after?.split_first_chunk::<4>()?;
            lines.push(SyncedText {
                text: self.decode(line, encoding, LimitKind::LyricsLineBytes),
                time: u32::from_be_bytes(time),
            });
            kept = count;
            rest = after;
        }
        self.lines = kept;
        Some(SyncedLyrics {
            language: [l0, l1, l2],
            timestamp_format,
            content_type,
            description: self.decode(description, encoding, LimitKind::ShortText),
            lines,
            truncated,
        })
    }

    /// An attached picture: the encoding, the ISO-8859-1 MIME type (in
    /// 2.2 a three-character image format), the picture type, the
    /// description, then the picture's octets, which are referred to and
    /// never decoded. A picture past the size or count limit is kept raw
    /// and recorded.
    fn picture(
        &mut self,
        data: &[u8],
        offset: u64,
        payload: Source<'_>,
        v22: bool,
    ) -> Option<FrameBody> {
        let (&octet, rest) = data.split_first()?;
        let (encoding, width) = encoding(octet)?;
        let (mime, picture_type, rest) = if v22 {
            let (&[f0, f1, f2, picture_type], rest) = rest.split_first_chunk::<4>()?;
            let format = self.decode(&[f0, f1, f2], Encoding::Latin1, LimitKind::ShortText);
            (format, picture_type, rest)
        } else {
            let (mime, after) = terminated(rest, 1);
            let (&picture_type, rest) = after?.split_first()?;
            let mime = self.decode(mime, Encoding::Latin1, LimitKind::ShortText);
            (mime, picture_type, rest)
        };
        let (description, after) = terminated(rest, width);
        let image = after.unwrap_or_default();
        let len = u64::try_from(image.len()).unwrap_or(u64::MAX);
        let count = self.pictures.saturating_add(1);
        let allowed = self
            .limits
            .check(LimitKind::PictureBytes, len, offset)
            .and(self.limits.check(LimitKind::Pictures, count, offset));
        if let Err(fault) = allowed {
            self.problems.push(TagProblem::Fault(fault));
            return Some(FrameBody::Raw(payload.span()));
        }
        self.pictures = count;
        let prefix = data.len().saturating_sub(image.len());
        Some(FrameBody::Picture(PictureRef {
            mime,
            picture_type,
            description: self.decode(description, encoding, LimitKind::ShortText),
            data: payload
                .skip(u64::try_from(prefix).unwrap_or(u64::MAX))
                .span(),
        }))
    }

    /// A unique file identifier: the ISO-8859-1 owner, then the
    /// identifier's octets.
    fn ufid(&self, data: &[u8]) -> FrameBody {
        let (owner, id) = terminated(data, 1);
        FrameBody::Ufid {
            owner: self.decode(owner, Encoding::Latin1, LimitKind::ShortText),
            id: id.unwrap_or_default().to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{
        contents, frame, limits, raw, read_with, span, steps, text, text_body, texts,
    };
    use super::super::{
        Credit, Frame, FrameBody, Id3v2Error, LanguageText, PictureRef, SyncedLyrics, SyncedText,
        TagProblem,
    };
    use super::*;
    use crate::parse::{Budget, LimitKind, Limits, ParseFault};
    use gunmetal_testkit::bytes::Bytes;
    use gunmetal_testkit::id3v2::{
        self as kit, Encoding as Kit, Tag, Version, comment, picture, picture_v22, synced_lyrics,
        ufid, unsynchronise, user_text,
    };

    /// The frames and problems of a tag of `version` holding one frame.
    /// Its header is at 10 and its body at 16 (2.2) or 20.
    fn one(version: Version, id: &[u8], flags: u16, body: &[u8]) -> (Vec<Frame>, Vec<TagProblem>) {
        contents(&Tag::new(version).frame(id, flags, body).build())
    }

    /// As [`one`], under `limits`.
    fn one_with(
        limits: &Limits,
        version: Version,
        id: &[u8],
        body: &[u8],
    ) -> (Vec<Frame>, Vec<TagProblem>) {
        let tag = read_with(&Tag::new(version).frame(id, 0, body).build(), limits)
            .expect("the tag is readable");
        (tag.frames, tag.problems)
    }

    /// The frame `id` at 10, with `body`, and no problems.
    fn alone(id: &[u8], body: FrameBody) -> (Vec<Frame>, Vec<TagProblem>) {
        (vec![frame(id, 10, 0, body)], vec![])
    }

    /// The frame `id` at 10 kept raw from 20 to `end`, recorded as
    /// malformed.
    fn malformed(id: &[u8], end: u64) -> (Vec<Frame>, Vec<TagProblem>) {
        (
            vec![frame(id, 10, 0, raw(20, end))],
            vec![TagProblem::Malformed {
                offset: 10,
                id: super::super::testing::id(id),
            }],
        )
    }

    fn capped(value: &str) -> Text {
        Text {
            value: value.to_owned(),
            truncated: true,
            replaced: false,
        }
    }

    /// "Café" in every way the versions write it.
    const CAFE_LATIN1: &[u8] = b"\x00Caf\xE9";
    const CAFE_UTF16_LE: &[u8] = b"\x01\xFF\xFEC\x00a\x00f\x00\xE9\x00";
    const CAFE_UTF16_BE_MARKED: &[u8] = b"\x01\xFE\xFF\x00C\x00a\x00f\x00\xE9";
    const CAFE_UTF16_BE: &[u8] = b"\x02\x00C\x00a\x00f\x00\xE9";
    const CAFE_UTF8: &[u8] = b"\x03Caf\xC3\xA9";

    #[test]
    fn decodes_a_2_2_text_frame_in_each_encoding() {
        for body in [CAFE_LATIN1, CAFE_UTF16_LE, CAFE_UTF16_BE_MARKED] {
            assert_eq!(
                one(Version::V22, b"TT2", 0, body),
                (vec![frame(b"TT2", 10, 0, text_body(&["Café"]))], vec![]),
                "{body:02X?}"
            );
        }
    }

    /// 2.3 defines encodings 0 and 1; writers use 2 and 3 as well, and
    /// they are read.
    #[test]
    fn decodes_a_2_3_text_frame_in_each_encoding() {
        for body in [
            CAFE_LATIN1,
            CAFE_UTF16_LE,
            CAFE_UTF16_BE_MARKED,
            CAFE_UTF16_BE,
            CAFE_UTF8,
        ] {
            assert_eq!(
                one(Version::V23, b"TIT2", 0, body),
                alone(b"TIT2", text_body(&["Café"])),
                "{body:02X?}"
            );
        }
    }

    #[test]
    fn decodes_a_2_4_text_frame_in_each_encoding() {
        for body in [
            CAFE_LATIN1,
            CAFE_UTF16_LE,
            CAFE_UTF16_BE_MARKED,
            CAFE_UTF16_BE,
            CAFE_UTF8,
        ] {
            assert_eq!(
                one(Version::V24, b"TIT2", 0, body),
                alone(b"TIT2", text_body(&["Café"])),
                "{body:02X?}"
            );
        }
        // An odd octet at the end of UTF-16 is replaced, and says so.
        assert_eq!(
            one(Version::V24, b"TIT2", 0, b"\x02\x00A\x42"),
            alone(
                b"TIT2",
                FrameBody::Text(vec![Text {
                    value: "A\u{FFFD}".to_owned(),
                    truncated: false,
                    replaced: true,
                }])
            )
        );
    }

    /// ID3v2.4.0 frames section 4.2: multiple values are separated by the
    /// encoding's terminator, with or without one after the last.
    #[test]
    fn splits_2_4_values_at_each_terminator() {
        let cases: [(Vec<u8>, &[&str]); 8] = [
            (b"\x03A\x00B".to_vec(), &["A", "B"]),
            (b"\x03A\x00B\x00".to_vec(), &["A", "B"]),
            (kit::text(Kit::Utf16, &["A", "B"]), &["A", "B"]),
            (
                [kit::text(Kit::Utf16Be, &["A", "B"]), vec![0, 0]].concat(),
                &["A", "B"],
            ),
            (b"\x00A\x00\x00".to_vec(), &["A", ""]),
            (b"\x00\x00".to_vec(), &[""]),
            (b"\x00".to_vec(), &[]),
            // In UTF-16 a terminator starts at an even offset: in 41 00 00
            // 00, the zeros at 1 and 2 are not one, those at 2 and 3 are.
            (
                b"\x01\xFF\xFEA\x00\x00\x00\xFF\xFEB\x00".to_vec(),
                &["A", "B"],
            ),
        ];
        for (body, values) in cases {
            assert_eq!(
                one(Version::V24, b"TPE1", 0, &body),
                alone(b"TPE1", text_body(values)),
                "{body:02X?}"
            );
        }
    }

    /// ID3v2.3.0 section 4.2: whatever follows a terminator is ignored.
    #[test]
    fn keeps_only_the_first_value_before_2_4() {
        assert_eq!(
            one(Version::V23, b"TPE1", 0, b"\x00A\x00B"),
            alone(b"TPE1", text_body(&["A"]))
        );
        assert_eq!(
            one(Version::V22, b"TP1", 0, b"\x00A\x00B"),
            (vec![frame(b"TP1", 10, 0, text_body(&["A"]))], vec![])
        );
    }

    /// `GRP1`, the grouping iTunes writes, and `XSOT`, `XSOP` and `XSOA`,
    /// the sort names 2.3 tags hold, are text information frames in all but
    /// the first letter of their identifier, and are read as such: in each
    /// encoding, with one value before 2.4 and every value from 2.4 on.
    #[test]
    fn decodes_the_grouping_and_the_sort_frames_that_do_not_start_with_t() {
        for id in [b"GRP1", b"XSOT", b"XSOP", b"XSOA"] {
            for body in [
                CAFE_LATIN1,
                CAFE_UTF16_LE,
                CAFE_UTF16_BE_MARKED,
                CAFE_UTF16_BE,
                CAFE_UTF8,
            ] {
                assert_eq!(
                    one(Version::V23, id, 0, body),
                    alone(id, text_body(&["Café"])),
                    "{id:?} {body:02X?}"
                );
            }
            assert_eq!(
                one(Version::V23, id, 0, b"\x00A\x00B"),
                alone(id, text_body(&["A"])),
                "{id:?}"
            );
            assert_eq!(
                one(Version::V24, id, 0, b"\x03A\x00B"),
                alone(id, text_body(&["A", "B"])),
                "{id:?}"
            );
        }
    }

    /// They are held to what every text frame is held to: a value is cut at
    /// the short-text limit and says so, the values count against the child
    /// limit, and a body with an encoding that does not exist is kept raw.
    ///
    /// Verifies: SEC-MED-006
    #[test]
    fn holds_the_grouping_and_the_sort_frames_to_the_limits_of_a_text_frame() {
        let short = limits(LimitKind::ShortText, 3);
        let single = limits(LimitKind::Children, 1);
        for id in [b"GRP1", b"XSOT", b"XSOP", b"XSOA"] {
            assert_eq!(
                one_with(&short, Version::V24, id, b"\x00ABC"),
                alone(id, text_body(&["ABC"])),
                "{id:?}"
            );
            assert_eq!(
                one_with(&short, Version::V24, id, b"\x00ABCD"),
                alone(id, FrameBody::Text(vec![capped("ABC")])),
                "{id:?}"
            );
            assert_eq!(
                one_with(&single, Version::V24, id, b"\x00A\x00B"),
                (
                    vec![frame(id, 10, 0, text_body(&["A"]))],
                    vec![TagProblem::Fault(ParseFault::LimitExceeded {
                        limit: LimitKind::Children,
                        value: 2,
                        max: 1,
                        offset: 10,
                    })]
                ),
                "{id:?}"
            );
            assert_eq!(
                one(Version::V24, id, 0, b"\x04A"),
                malformed(id, 22),
                "{id:?}"
            );
        }
    }

    #[test]
    fn reads_user_text_with_or_without_a_description() {
        let user = |description: &str, values: &[&str]| FrameBody::UserText {
            description: text(description),
            values: texts(values),
        };
        assert_eq!(
            one(
                Version::V24,
                b"TXXX",
                0,
                &user_text(Kit::Latin1, "", &["v"])
            ),
            alone(b"TXXX", user("", &["v"]))
        );
        assert_eq!(
            one(
                Version::V24,
                b"TXXX",
                0,
                &user_text(Kit::Utf16, "key", &["v1", "v2"])
            ),
            alone(b"TXXX", user("key", &["v1", "v2"]))
        );
        assert_eq!(
            one(
                Version::V23,
                b"TXXX",
                0,
                &user_text(Kit::Utf8, "key", &["v1", "v2"])
            ),
            alone(b"TXXX", user("key", &["v1"]))
        );
        assert_eq!(
            one(Version::V24, b"TXXX", 0, b"\x00key"),
            alone(b"TXXX", user("key", &[]))
        );
        assert_eq!(
            one(
                Version::V22,
                b"TXX",
                0,
                &user_text(Kit::Latin1, "k", &["v"])
            ),
            (vec![frame(b"TXX", 10, 0, user("k", &["v"]))], vec![])
        );
    }

    #[test]
    fn pairs_the_roles_and_names_of_involved_people() {
        let credit = |role: &str, name: &str| Credit {
            role: text(role),
            name: text(name),
        };
        let body = kit::text(Kit::Latin1, &["producer", "A", "mix", "B"]);
        for id in [b"TIPL", b"TMCL"] {
            assert_eq!(
                one(Version::V24, id, 0, &body),
                alone(
                    id,
                    FrameBody::People(vec![credit("producer", "A"), credit("mix", "B")])
                )
            );
        }
        // Every version separates the list with terminators.
        assert_eq!(
            one(
                Version::V23,
                b"IPLS",
                0,
                &kit::text(Kit::Utf16, &["guitar"])
            ),
            alone(b"IPLS", FrameBody::People(vec![credit("guitar", "")]))
        );
        assert_eq!(
            one(
                Version::V22,
                b"IPL",
                0,
                &kit::text(Kit::Latin1, &["bass", "C"])
            ),
            (
                vec![frame(
                    b"IPL",
                    10,
                    0,
                    FrameBody::People(vec![credit("bass", "C")])
                )],
                vec![]
            )
        );
    }

    #[test]
    fn reads_comments_and_unsynchronised_lyrics() {
        let language_text = |language: &[u8; 3], description: &str, value: &str| LanguageText {
            language: *language,
            description: text(description),
            text: text(value),
        };
        assert_eq!(
            one(
                Version::V24,
                b"COMM",
                0,
                &comment(Kit::Latin1, *b"eng", "d", "Text")
            ),
            alone(
                b"COMM",
                FrameBody::Comment(language_text(b"eng", "d", "Text"))
            )
        );
        assert_eq!(
            one(
                Version::V23,
                b"USLT",
                0,
                &comment(Kit::Utf16, *b"deu", "", "La\nla")
            ),
            alone(
                b"USLT",
                FrameBody::Lyrics(language_text(b"deu", "", "La\nla"))
            )
        );
        // The text ends at its terminator, and a description with no
        // terminator leaves no text.
        assert_eq!(
            one(Version::V24, b"COMM", 0, b"\x03xxxd\x00Text\x00junk"),
            alone(
                b"COMM",
                FrameBody::Comment(language_text(b"xxx", "d", "Text"))
            )
        );
        assert_eq!(
            one(Version::V24, b"COMM", 0, b"\x03xxxd"),
            alone(b"COMM", FrameBody::Comment(language_text(b"xxx", "d", "")))
        );
        for (id, lyrics) in [(b"COM", false), (b"ULT", true)] {
            let body = comment(Kit::Latin1, *b"eng", "", "T");
            let expected = language_text(b"eng", "", "T");
            let expected = if lyrics {
                FrameBody::Lyrics(expected)
            } else {
                FrameBody::Comment(expected)
            };
            assert_eq!(
                one(Version::V22, id, 0, &body),
                (vec![frame(id, 10, 0, expected)], vec![])
            );
        }
    }

    /// The synchronised lyrics `lines`, in English, timed in milliseconds.
    fn synced(lines: &[(&str, u32)], truncated: bool) -> FrameBody {
        FrameBody::SyncedLyrics(SyncedLyrics {
            language: *b"eng",
            timestamp_format: 2,
            content_type: 1,
            description: text("d"),
            lines: lines
                .iter()
                .map(|&(line, time)| SyncedText {
                    text: text(line),
                    time,
                })
                .collect(),
            truncated,
        })
    }

    #[test]
    fn reads_synchronised_lyrics() {
        let lines = [("La", 1_000), ("Di", 0x0102_0304)];
        for encoding in [Kit::Latin1, Kit::Utf16] {
            let body = synced_lyrics(encoding, *b"eng", [2, 1], "d", &lines);
            assert_eq!(
                one(Version::V24, b"SYLT", 0, &body),
                alone(b"SYLT", synced(&lines, false)),
                "{encoding:?}"
            );
        }
        let body = synced_lyrics(Kit::Latin1, *b"eng", [2, 1], "d", &[]);
        assert_eq!(
            one(Version::V22, b"SLT", 0, &body),
            (vec![frame(b"SLT", 10, 0, synced(&[], false))], vec![])
        );
        // A description with no terminator leaves no lines.
        assert_eq!(
            one(Version::V24, b"SYLT", 0, b"\x00eng\x02\x01d"),
            alone(b"SYLT", synced(&[], false))
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn drops_synchronised_lines_past_the_line_limit_and_says_so() {
        let lines = [("La", 1), ("Di", 2), ("Da", 3)];
        let body = synced_lyrics(Kit::Latin1, *b"eng", [2, 1], "d", &lines);
        assert_eq!(
            one_with(
                &limits(LimitKind::LyricsLines, 3),
                Version::V24,
                b"SYLT",
                &body
            ),
            alone(b"SYLT", synced(&lines, false))
        );
        assert_eq!(
            one_with(
                &limits(LimitKind::LyricsLines, 2),
                Version::V24,
                b"SYLT",
                &body
            ),
            alone(b"SYLT", synced(&lines[..2], true))
        );
    }

    #[test]
    fn keeps_synchronised_lyrics_with_a_broken_line_raw() {
        // The last line has no time stamp, then no terminator.
        let no_time = b"\x00eng\x02\x01d\x00La\x00\x00\x00\x00\x01Di\x00\x00\x00";
        assert_eq!(
            one(Version::V24, b"SYLT", 0, no_time),
            malformed(b"SYLT", 40)
        );
        let no_end = b"\x00eng\x02\x01d\x00Di";
        assert_eq!(
            one(Version::V24, b"SYLT", 0, no_end),
            malformed(b"SYLT", 30)
        );
    }

    /// The picture at 10 with the given fields and its octets from `start`
    /// to `end`.
    fn picture_body(
        mime: &str,
        picture_type: u8,
        description: &str,
        data: (u64, u64, bool),
    ) -> FrameBody {
        FrameBody::Picture(PictureRef {
            mime: text(mime),
            picture_type,
            description: text(description),
            data: span(data.0, data.1, data.2),
        })
    }

    #[test]
    fn reads_pictures_as_references_to_their_octets() {
        let png = [0x89, 0x50, 0x4E, 0x47];
        // 1 + 10 + 1 + 2 octets of fields from 20, then the picture.
        let body = picture(Kit::Utf8, "image/png", 3, "c", &png);
        let tag = Tag::new(Version::V24).frame(b"APIC", 0, &body).build();
        assert_eq!(
            contents(&tag),
            alone(b"APIC", picture_body("image/png", 3, "c", (34, 38, false)))
        );
        assert_eq!(span(34, 38, false).read(&tag).as_deref(), Some(&png[..]));
        // 2.2: 1 + 3 + 1 + 1 octets of fields from 16.
        let body = picture_v22(Kit::Latin1, *b"JPG", 4, "", &[0xFF, 0xD8]);
        assert_eq!(
            one(Version::V22, b"PIC", 0, &body),
            (
                vec![frame(
                    b"PIC",
                    10,
                    0,
                    picture_body("JPG", 4, "", (22, 24, false))
                )],
                vec![]
            )
        );
    }

    #[test]
    fn refers_to_an_unsynchronised_picture_by_its_octets_as_stored() {
        let jpeg = [0xFF, 0xD8, 0xFF, 0xE0];
        // 19 octets as read: 15 of fields ending in "ÿ" and its
        // terminator (FF 00), then the picture. Stored: 16 and 5.
        let body = unsynchronise(&picture(Kit::Latin1, "image/jpeg", 3, "ÿ", &jpeg));
        assert_eq!(body.len(), 21);
        let tag = Tag::new(Version::V24).frame(b"APIC", 0x0002, &body).build();
        assert_eq!(
            contents(&tag),
            (
                vec![frame(
                    b"APIC",
                    10,
                    0x0002,
                    picture_body("image/jpeg", 3, "ÿ", (36, 41, true))
                )],
                vec![]
            )
        );
        assert_eq!(span(36, 41, true).read(&tag).as_deref(), Some(&jpeg[..]));
    }

    #[test]
    fn keeps_a_picture_whose_fields_are_cut_short_raw() {
        // No terminator after the MIME type, so no picture type.
        assert_eq!(
            one(Version::V24, b"APIC", 0, b"\x00image/png"),
            malformed(b"APIC", 30)
        );
        assert_eq!(
            one(Version::V24, b"APIC", 0, b"\x00image/png\x00"),
            malformed(b"APIC", 31)
        );
        assert_eq!(
            one(Version::V22, b"PIC", 0, b"\x00PNG"),
            (
                vec![frame(b"PIC", 10, 0, raw(16, 20))],
                vec![TagProblem::Malformed {
                    offset: 10,
                    id: super::super::testing::id(b"PIC")
                }]
            )
        );
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn keeps_a_picture_over_the_size_limit_raw() {
        let body = picture(Kit::Latin1, "image/png", 3, "", &[0x89, 0x50, 0x4E, 0x47]);
        // 1 + 10 + 1 + 1 octets of fields from 20, then 4 of picture.
        assert_eq!(
            one_with(
                &limits(LimitKind::PictureBytes, 4),
                Version::V24,
                b"APIC",
                &body
            ),
            alone(b"APIC", picture_body("image/png", 3, "", (33, 37, false)))
        );
        assert_eq!(
            one_with(
                &limits(LimitKind::PictureBytes, 3),
                Version::V24,
                b"APIC",
                &body
            ),
            (
                vec![frame(b"APIC", 10, 0, raw(20, 37))],
                vec![TagProblem::Fault(ParseFault::LimitExceeded {
                    limit: LimitKind::PictureBytes,
                    value: 4,
                    max: 3,
                    offset: 10,
                })]
            )
        );
    }

    /// The 32 MiB of SEC-MED-045, which may not be raised, and one octet
    /// more.
    ///
    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn keeps_a_picture_of_32_mib_and_refuses_one_octet_more() {
        const MIB_32: u64 = 33_554_432;
        let fields = 1 + 10 + 1 + 1;
        for (len, kept) in [(MIB_32, true), (MIB_32 + 1, false)] {
            let mut data = Bytes::new();
            data.zeros(usize::try_from(len).unwrap());
            let data = data.into_vec();
            let body = picture(Kit::Latin1, "image/png", 3, "", &data);
            let tag = Tag::new(Version::V24).frame(b"APIC", 0, &body).build();
            let end = 20 + fields + len;
            let expected = if kept {
                alone(
                    b"APIC",
                    picture_body("image/png", 3, "", (20 + fields, end, false)),
                )
            } else {
                (
                    vec![frame(b"APIC", 10, 0, raw(20, end))],
                    vec![TagProblem::Fault(ParseFault::LimitExceeded {
                        limit: LimitKind::PictureBytes,
                        value: MIB_32 + 1,
                        max: MIB_32,
                        offset: 10,
                    })],
                )
            };
            assert_eq!(contents(&tag), expected, "{len}");
        }
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn keeps_16_pictures_and_refuses_the_17th() {
        let body = picture(Kit::Latin1, "", 3, "", b"x");
        // Each picture frame takes 10 + 5 octets.
        let tag = (0..17)
            .fold(Tag::new(Version::V24), |tag, _| {
                tag.frame(b"APIC", 0, &body)
            })
            .build();
        let mut frames: Vec<Frame> = (0..16)
            .map(|index| {
                let start = 10 + 15 * index;
                frame(
                    b"APIC",
                    start,
                    0,
                    picture_body("", 3, "", (start + 14, start + 15, false)),
                )
            })
            .collect();
        frames.push(frame(b"APIC", 250, 0, raw(260, 265)));
        assert_eq!(
            contents(&tag),
            (
                frames,
                vec![TagProblem::Fault(ParseFault::LimitExceeded {
                    limit: LimitKind::Pictures,
                    value: 17,
                    max: 16,
                    offset: 250,
                })]
            )
        );
    }

    #[test]
    fn counts_only_the_pictures_it_keeps() {
        let big = picture(Kit::Latin1, "", 3, "", b"xyz");
        let small = picture(Kit::Latin1, "", 3, "", b"x");
        let tag = Tag::new(Version::V24)
            .frame(b"APIC", 0, &big)
            .frame(b"APIC", 0, &small)
            .frame(b"APIC", 0, &small)
            .build();
        let limits = limits(LimitKind::PictureBytes, 2)
            .with_override(LimitKind::Pictures, 1)
            .unwrap();
        let read = read_with(&tag, &limits).unwrap();
        // The big picture at 10 (17 octets) is refused for its size, the
        // first small one at 27 is kept, and the second at 42 is one too
        // many.
        assert_eq!(
            (read.frames, read.problems),
            (
                vec![
                    frame(b"APIC", 10, 0, raw(20, 27)),
                    frame(b"APIC", 27, 0, picture_body("", 3, "", (41, 42, false))),
                    frame(b"APIC", 42, 0, raw(52, 57)),
                ],
                vec![
                    TagProblem::Fault(ParseFault::LimitExceeded {
                        limit: LimitKind::PictureBytes,
                        value: 3,
                        max: 2,
                        offset: 10,
                    }),
                    TagProblem::Fault(ParseFault::LimitExceeded {
                        limit: LimitKind::Pictures,
                        value: 2,
                        max: 1,
                        offset: 42,
                    }),
                ]
            )
        );
    }

    #[test]
    fn reads_unique_file_identifiers() {
        let mbid = b"f5093c06-23e3-404f-aeaa-40f72885ee3a";
        let expected = |owner: &str, id: &[u8]| FrameBody::Ufid {
            owner: text(owner),
            id: id.to_vec(),
        };
        assert_eq!(
            one(
                Version::V24,
                b"UFID",
                0,
                &ufid("http://musicbrainz.org", mbid)
            ),
            alone(b"UFID", expected("http://musicbrainz.org", mbid))
        );
        assert_eq!(
            one(Version::V24, b"UFID", 0, b"owner"),
            alone(b"UFID", expected("owner", b""))
        );
        assert_eq!(
            one(Version::V22, b"UFI", 0, &ufid("o", b"\x00\x01")),
            (
                vec![frame(b"UFI", 10, 0, expected("o", b"\x00\x01"))],
                vec![]
            )
        );
    }

    #[test]
    fn keeps_every_other_frame_raw() {
        for id in [
            b"POPM", b"WXXX", b"WOAR", b"PRIV", b"GEOB", b"MCDI", b"XYZ1",
        ] {
            assert_eq!(
                one(Version::V24, id, 0, b"\x00http://x"),
                alone(id, raw(20, 29))
            );
        }
        for id in [b"POP", b"WAR", b"CNT"] {
            assert_eq!(
                one(Version::V22, id, 0, b"\x01\x02"),
                (vec![frame(id, 10, 0, raw(16, 18))], vec![])
            );
        }
    }

    #[test]
    fn keeps_a_frame_with_an_unknown_encoding_raw() {
        for (id, body) in [
            (b"TIT2", &b"\x04A"[..]),
            (b"TXXX", b"\x05d\x00v"),
            (b"TIPL", b"\xFFr\x00n"),
            (b"COMM", b"\x09engd\x00t"),
            (b"USLT", b"\x04engd\x00t"),
            (b"SYLT", b"\x04eng\x02\x01d\x00"),
            (b"APIC", b"\x04image/png\x00\x03d\x00x"),
        ] {
            let end = 20 + u64::try_from(body.len()).unwrap();
            assert_eq!(one(Version::V24, id, 0, body), malformed(id, end), "{id:?}");
        }
    }

    #[test]
    fn keeps_a_frame_too_short_for_its_fixed_fields_raw() {
        for (id, body) in [(b"COMM", &b"\x00en"[..]), (b"SYLT", b"\x00eng\x02")] {
            let end = 20 + u64::try_from(body.len()).unwrap();
            assert_eq!(one(Version::V24, id, 0, body), malformed(id, end), "{id:?}");
        }
    }

    /// A 2.3 group octet and nothing after it leaves a frame no body to
    /// read: every kind with an encoding octet is kept raw, and an
    /// identifier with no owner is empty.
    #[test]
    fn reads_a_frame_whose_flags_leave_no_body() {
        for id in [
            b"TIT2", b"TXXX", b"TIPL", b"COMM", b"USLT", b"SYLT", b"APIC",
        ] {
            assert_eq!(
                one(Version::V23, id, 0x0020, b"\x80"),
                (
                    vec![frame(id, 10, 0x0020, raw(21, 21))],
                    vec![TagProblem::Malformed {
                        offset: 10,
                        id: super::super::testing::id(id)
                    }]
                ),
                "{id:?}"
            );
        }
        assert_eq!(
            one(Version::V23, b"UFID", 0x0020, b"\x80"),
            (
                vec![frame(
                    b"UFID",
                    10,
                    0x0020,
                    FrameBody::Ufid {
                        owner: text(""),
                        id: vec![],
                    }
                )],
                vec![]
            )
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn caps_each_text_at_its_limit_and_says_so() {
        let short = limits(LimitKind::ShortText, 3);
        assert_eq!(
            one_with(&short, Version::V24, b"TIT2", b"\x00ABC"),
            alone(b"TIT2", text_body(&["ABC"]))
        );
        assert_eq!(
            one_with(&short, Version::V24, b"TIT2", b"\x00ABCD"),
            alone(b"TIT2", FrameBody::Text(vec![capped("ABC")]))
        );
        let long = limits(LimitKind::LongText, 2);
        assert_eq!(
            one_with(&long, Version::V24, b"TXXX", b"\x00desc\x00value"),
            alone(
                b"TXXX",
                FrameBody::UserText {
                    description: text("desc"),
                    values: vec![capped("va")],
                }
            )
        );
        assert_eq!(
            one_with(&long, Version::V24, b"COMM", b"\x00engdesc\x00text"),
            alone(
                b"COMM",
                FrameBody::Comment(LanguageText {
                    language: *b"eng",
                    description: text("desc"),
                    text: capped("te"),
                })
            )
        );
        let lyrics = limits(LimitKind::LyricsBytes, 2);
        assert_eq!(
            one_with(&lyrics, Version::V24, b"USLT", b"\x00engdesc\x00text"),
            alone(
                b"USLT",
                FrameBody::Lyrics(LanguageText {
                    language: *b"eng",
                    description: text("desc"),
                    text: capped("te"),
                })
            )
        );
        let line = limits(LimitKind::LyricsLineBytes, 1);
        let body = synced_lyrics(Kit::Latin1, *b"eng", [2, 1], "d", &[("La", 5)]);
        let expected = SyncedLyrics {
            language: *b"eng",
            timestamp_format: 2,
            content_type: 1,
            description: text("d"),
            lines: vec![SyncedText {
                text: capped("L"),
                time: 5,
            }],
            truncated: false,
        };
        assert_eq!(
            one_with(&line, Version::V24, b"SYLT", &body),
            alone(b"SYLT", FrameBody::SyncedLyrics(expected))
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn caps_a_title_at_4_kib() {
        let mut body = vec![0x00];
        body.extend([b'A'; 4_097]);
        let a = |count| (0..count).map(|_| 'A').collect::<String>();
        assert_eq!(
            one(Version::V24, b"TIT2", 0, &body[..4_097]),
            alone(b"TIT2", text_body(&[&a(4_096)]))
        );
        assert_eq!(
            one(Version::V24, b"TIT2", 0, &body),
            alone(b"TIT2", FrameBody::Text(vec![capped(&a(4_096))]))
        );
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn keeps_at_most_the_child_limit_of_values_in_one_frame() {
        let few = limits(LimitKind::Children, 2);
        assert_eq!(
            one_with(&few, Version::V24, b"TPE1", b"\x00A\x00B"),
            alone(b"TPE1", text_body(&["A", "B"]))
        );
        assert_eq!(
            one_with(&few, Version::V24, b"TPE1", b"\x00A\x00B\x00C"),
            (
                vec![frame(b"TPE1", 10, 0, text_body(&["A", "B"]))],
                vec![TagProblem::Fault(ParseFault::LimitExceeded {
                    limit: LimitKind::Children,
                    value: 3,
                    max: 2,
                    offset: 10,
                })]
            )
        );
        // 65,536 empty values, then one more.
        for (zeros, kept, problems) in [
            (65_536, 65_536, vec![]),
            (
                65_537,
                65_536,
                vec![TagProblem::Fault(ParseFault::LimitExceeded {
                    limit: LimitKind::Children,
                    value: 65_537,
                    max: 65_536,
                    offset: 10,
                })],
            ),
        ] {
            let body: Vec<u8> = (0..=zeros).map(|_| 0x00).collect();
            let values = (0..kept).map(|_| text("")).collect();
            assert_eq!(
                one(Version::V24, b"TPE1", 0, &body),
                (
                    vec![frame(b"TPE1", 10, 0, FrameBody::Text(values))],
                    problems
                ),
                "{zeros}"
            );
        }
    }

    /// A tag of six values in three frames: two artists at 10, a user text
    /// of two values at 24, and the role and the name of one credit at 40.
    fn six_values() -> Vec<u8> {
        Tag::new(Version::V24)
            .frame(b"TPE1", 0, &kit::text(Kit::Latin1, &["A", "B"]))
            .frame(b"TXXX", 0, &user_text(Kit::Latin1, "k", &["C", "D"]))
            .frame(b"TIPL", 0, &kit::text(Kit::Latin1, &["mix", "E"]))
            .build()
    }

    /// The frames of [`six_values`] when the user text keeps `user` and the
    /// involved people list keeps `credits`.
    fn six_values_read(user: &[&str], credits: Vec<Credit>) -> Vec<Frame> {
        vec![
            frame(b"TPE1", 10, 0, text_body(&["A", "B"])),
            frame(
                b"TXXX",
                24,
                0,
                FrameBody::UserText {
                    description: text("k"),
                    values: texts(user),
                },
            ),
            frame(b"TIPL", 40, 0, FrameBody::People(credits)),
        ]
    }

    /// The problem recorded at the frame at `offset` for the value that
    /// would be the tag's `value`th when it may hold `max`.
    fn too_many_values(value: u64, max: u64, offset: u64) -> TagProblem {
        TagProblem::Fault(ParseFault::LimitExceeded {
            limit: LimitKind::Children,
            value,
            max,
            offset,
        })
    }

    /// The child limit counts the values of the whole tag, not of one
    /// frame. A frame that starts at the limit keeps none of its values
    /// and says so.
    ///
    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn counts_the_values_of_every_frame_against_the_child_limit() {
        let credit = |name: &str| Credit {
            role: text("mix"),
            name: text(name),
        };
        let read = |max: u64| {
            let found = read_with(&six_values(), &limits(LimitKind::Children, max))
                .expect("the tag is readable");
            (found.frames, found.problems)
        };
        assert_eq!(
            read(6),
            (six_values_read(&["C", "D"], vec![credit("E")]), vec![])
        );
        assert_eq!(
            read(5),
            (
                six_values_read(&["C", "D"], vec![credit("")]),
                vec![too_many_values(6, 5, 40)]
            )
        );
        assert_eq!(
            read(3),
            (
                six_values_read(&["C"], vec![]),
                vec![too_many_values(4, 3, 24), too_many_values(4, 3, 40)]
            )
        );
    }

    /// The lyrics line limit counts the synchronised lines of the whole
    /// tag, not of one frame. A frame that starts at the limit keeps none
    /// of its lines and says so.
    ///
    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn counts_the_lines_of_every_frame_against_the_line_limit() {
        let first: [(&str, u32); 2] = [("La", 1), ("Di", 2)];
        let second: [(&str, u32); 2] = [("Da", 3), ("Du", 4)];
        let third: [(&str, u32); 1] = [("Do", 5)];
        let body = |lines: &[(&str, u32)]| synced_lyrics(Kit::Latin1, *b"eng", [2, 1], "d", lines);
        // A body of two lines takes 22 octets, so the frames start at 10,
        // 42 and 74.
        let tag = Tag::new(Version::V24)
            .frame(b"SYLT", 0, &body(&first))
            .frame(b"SYLT", 0, &body(&second))
            .frame(b"SYLT", 0, &body(&third))
            .build();
        let read = |max: u64| {
            let found =
                read_with(&tag, &limits(LimitKind::LyricsLines, max)).expect("the tag is readable");
            (found.frames, found.problems)
        };
        let frames = |middle: FrameBody, last: FrameBody| {
            vec![
                frame(b"SYLT", 10, 0, synced(&first, false)),
                frame(b"SYLT", 42, 0, middle),
                frame(b"SYLT", 74, 0, last),
            ]
        };
        assert_eq!(
            read(5),
            (
                frames(synced(&second, false), synced(&third, false)),
                vec![]
            )
        );
        assert_eq!(
            read(4),
            (frames(synced(&second, false), synced(&[], true)), vec![])
        );
        assert_eq!(
            read(3),
            (
                frames(synced(&second[..1], true), synced(&[], true)),
                vec![]
            )
        );
    }

    /// Only the lines of a frame that is kept count. The first frame here
    /// has one whole line and then a line with no time stamp, so it is kept
    /// raw, and the tag still has room for three lines.
    ///
    /// Verifies: SEC-MED-006
    #[test]
    fn counts_only_the_lines_of_the_frames_it_keeps() {
        let whole: [(&str, u32); 2] = [("La", 1), ("Di", 2)];
        let lines = synced_lyrics(Kit::Latin1, *b"eng", [2, 1], "d", &whole);
        // 20 octets, the last line cut short, then two bodies of 22: the
        // frames start at 10, 40 and 72.
        let broken = b"\x00eng\x02\x01d\x00La\x00\x00\x00\x00\x01Di\x00\x00\x00";
        let tag = Tag::new(Version::V24)
            .frame(b"SYLT", 0, broken)
            .frame(b"SYLT", 0, &lines)
            .frame(b"SYLT", 0, &lines)
            .build();
        let read =
            read_with(&tag, &limits(LimitKind::LyricsLines, 3)).expect("the tag is readable");
        assert_eq!(
            (read.frames, read.problems),
            (
                vec![
                    frame(b"SYLT", 10, 0, raw(20, 40)),
                    frame(b"SYLT", 40, 0, synced(&whole, false)),
                    frame(b"SYLT", 72, 0, synced(&whole[..1], true)),
                ],
                vec![TagProblem::Malformed {
                    offset: 10,
                    id: super::super::testing::id(b"SYLT")
                }]
            )
        );
    }

    /// The worst case under the default limits: one tag holds at most
    /// 65,536 values and 10,000 synchronised lines, 75,536 decoded elements
    /// in all, however many frames declare them. Three frames here hold
    /// 30,000 empty values each, and three more 4,000 empty lines each.
    ///
    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn holds_at_most_75_536_values_and_lines_under_the_default_limits() {
        let values: Vec<u8> = (0..=30_000).map(|_| 0x00).collect();
        let lines: Vec<(&str, u32)> = (0..4_000).map(|_| ("", 0)).collect();
        let lyrics = synced_lyrics(Kit::Latin1, *b"eng", [2, 1], "d", &lines);
        // A frame of values takes 30,011 octets and a frame of lines
        // 20,018.
        let tag = Tag::new(Version::V24)
            .frame(b"TPE1", 0, &values)
            .frame(b"TPE1", 0, &values)
            .frame(b"TPE1", 0, &values)
            .frame(b"SYLT", 0, &lyrics)
            .frame(b"SYLT", 0, &lyrics)
            .frame(b"SYLT", 0, &lyrics)
            .build();
        let (frames, problems) = contents(&tag);
        let values_kept: Vec<usize> = frames
            .iter()
            .filter_map(|found| match &found.body {
                FrameBody::Text(kept) => Some(kept.len()),
                _ => None,
            })
            .collect();
        let lines_kept: Vec<usize> = frames
            .iter()
            .filter_map(|found| match &found.body {
                FrameBody::SyncedLyrics(kept) => Some(kept.lines.len()),
                _ => None,
            })
            .collect();
        assert_eq!(values_kept, [30_000, 30_000, 5_536]);
        assert_eq!(lines_kept, [4_000, 4_000, 2_000]);
        assert_eq!(values_kept.iter().chain(&lines_kept).sum::<usize>(), 75_536);
        assert_eq!(problems, [too_many_values(65_537, 65_536, 60_032)]);
        let artists = |offset: u64, kept: usize| {
            let empties = (0..kept).map(|_| text("")).collect();
            frame(b"TPE1", offset, 0, FrameBody::Text(empties))
        };
        let words = |offset: u64, kept: usize, truncated: bool| {
            frame(b"SYLT", offset, 0, synced(&lines[..kept], truncated))
        };
        assert_eq!(
            frames,
            [
                artists(10, 30_000),
                artists(30_021, 30_000),
                artists(60_032, 5_536),
                words(90_043, 4_000, false),
                words(110_061, 4_000, false),
                words(130_079, 2_000, true),
            ]
        );
    }

    /// Reading the values or the lines of a frame takes at least one octet
    /// of the frame per turn, and a frame's octets are charged before its
    /// body is decoded. So the charge covers the work whether the tag has
    /// room for what the frame holds or not: each 2.4 frame is charged its
    /// octets twice, once to tell how the sizes are written and once to
    /// read it, and the parse fails at the frame the budget cannot pay for.
    ///
    /// Verifies: SEC-MED-007
    #[test]
    fn charges_a_frame_its_octets_whether_its_values_are_kept_or_refused() {
        // Frames of 14, 16 and 16 octets, at 10, 24 and 40.
        for max in [6, 3, 0] {
            assert_eq!(
                steps(&six_values(), &limits(LimitKind::Children, max)),
                92,
                "a child limit of {max}"
            );
        }
        let parse = |allowed: u64| {
            let mut budget = Budget::for_input(0, 0, allowed);
            super::super::parse(&six_values(), &Limits::DEFAULT, &mut budget)
                .map(|tag| tag.frames.len())
        };
        assert_eq!(parse(92), Ok(3));
        assert_eq!(
            parse(91),
            Err(Id3v2Error::Fault(ParseFault::BudgetExceeded { offset: 40 }))
        );
        // Three frames of 4,000 lines, 20,018 octets each.
        let lines: Vec<(&str, u32)> = (0..4_000).map(|_| ("", 0)).collect();
        let lyrics = synced_lyrics(Kit::Latin1, *b"eng", [2, 1], "d", &lines);
        let tag = Tag::new(Version::V24)
            .frame(b"SYLT", 0, &lyrics)
            .frame(b"SYLT", 0, &lyrics)
            .frame(b"SYLT", 0, &lyrics)
            .build();
        for max in [10_000, 4_000, 0] {
            assert_eq!(
                steps(&tag, &limits(LimitKind::LyricsLines, max)),
                120_108,
                "a line limit of {max}"
            );
        }
    }

    #[test]
    fn splits_at_terminators_of_one_or_two_octets() {
        assert_eq!(terminated(b"ab\x00cd", 1), (&b"ab"[..], Some(&b"cd"[..])));
        assert_eq!(terminated(b"ab", 1), (&b"ab"[..], None));
        assert_eq!(terminated(b"\x00", 1), (&b""[..], Some(&b""[..])));
        assert_eq!(
            terminated(b"a\x00\x00b\x00\x00", 2),
            (&b"a\x00\x00b"[..], Some(&b""[..]))
        );
        assert_eq!(terminated(b"a\x00\x00", 2), (&b"a\x00\x00"[..], None));
        assert_eq!(terminated(b"\x00\x00ab", 2), (&b""[..], Some(&b"ab"[..])));
    }
}
