//! Builders for `ID3v2` tags, written from the ID3v2.2.0, ID3v2.3.0 and
//! ID3v2.4.0 specifications (id3.org, mirrored by mutagen-specs).
//!
//! A tag is built from frames, and a frame from its body. The body builders
//! lay out each frame kind's fields as the specifications list them; the
//! frame and tag builders add the headers, sizes, padding, footer and the
//! unsynchronisation scheme. Nothing here shares code with the parser in
//! `gunmetal-core`.
//!
//! ```
//! use gunmetal_testkit::id3v2::{Encoding, Tag, Version, text};
//!
//! let tag = Tag::new(Version::V24)
//!     .frame(b"TIT2", 0, &text(Encoding::Utf8, &["Hi"]))
//!     .build();
//! assert_eq!(tag, b"ID3\x04\x00\x00\x00\x00\x00\x0DTIT2\x00\x00\x00\x03\x00\x00\x03Hi");
//! ```

use crate::bytes::Bytes;

/// The version of `ID3v2` a tag or a frame is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    /// ID3v2.2.0: three-character frame IDs and three-octet frame sizes.
    V22,
    /// ID3v2.3.0: four-character frame IDs and 32-bit frame sizes.
    V23,
    /// ID3v2.4.0: four-character frame IDs and syncsafe frame sizes.
    V24,
}

impl Version {
    /// The major version octet of the tag header.
    fn major(self) -> u8 {
        match self {
            Self::V22 => 2,
            Self::V23 => 3,
            Self::V24 => 4,
        }
    }
}

/// A text encoding, as a frame's encoding octet names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// `$00`: ISO-8859-1, terminated by one zero octet.
    Latin1,
    /// `$01`: UTF-16 with a byte-order mark, terminated by two zero octets.
    /// The builders write it little-endian, after the mark `FF FE`, as most
    /// writers do.
    Utf16,
    /// `$02`: UTF-16 big-endian without a mark (2.4), terminated by two zero
    /// octets.
    Utf16Be,
    /// `$03`: UTF-8 (2.4), terminated by one zero octet.
    Utf8,
}

impl Encoding {
    /// The encoding octet that names this encoding.
    #[must_use]
    pub fn byte(self) -> u8 {
        match self {
            Self::Latin1 => 0,
            Self::Utf16 => 1,
            Self::Utf16Be => 2,
            Self::Utf8 => 3,
        }
    }

    /// `text` in this encoding, without a terminator. Each UTF-16 string
    /// starts with its own byte-order mark, as ID3v2.4.0 section 4.2 asks.
    ///
    /// # Panics
    ///
    /// Panics when `text` holds a character that ISO-8859-1 cannot encode.
    #[must_use]
    pub fn encode(self, text: &str) -> Vec<u8> {
        match self {
            Self::Latin1 => text
                .chars()
                .map(|c| u8::try_from(u32::from(c)).expect("the text fits ISO-8859-1"))
                .collect(),
            Self::Utf16 => [0xFF, 0xFE]
                .into_iter()
                .chain(text.encode_utf16().flat_map(u16::to_le_bytes))
                .collect(),
            Self::Utf16Be => text.encode_utf16().flat_map(u16::to_be_bytes).collect(),
            Self::Utf8 => text.as_bytes().to_vec(),
        }
    }

    /// The terminator of a string in this encoding: one zero octet, or two
    /// for UTF-16.
    #[must_use]
    pub fn terminator(self) -> &'static [u8] {
        match self {
            Self::Latin1 | Self::Utf8 => &[0],
            Self::Utf16 | Self::Utf16Be => &[0, 0],
        }
    }

    /// `text` in this encoding, followed by its terminator.
    #[must_use]
    pub fn terminated(self, text: &str) -> Vec<u8> {
        let mut bytes = self.encode(text);
        bytes.extend_from_slice(self.terminator());
        bytes
    }

    /// `texts` in this encoding, each but the last followed by a terminator.
    fn joined(self, texts: &[&str]) -> Vec<u8> {
        let encoded: Vec<Vec<u8>> = texts.iter().map(|text| self.encode(text)).collect();
        encoded.join(self.terminator())
    }
}

/// The body of a text information frame (`T***`, or `T**` in 2.2): the
/// encoding octet, then the values separated by terminators, with none
/// after the last.
#[must_use]
pub fn text(encoding: Encoding, values: &[&str]) -> Vec<u8> {
    let mut body = vec![encoding.byte()];
    body.extend(encoding.joined(values));
    body
}

/// The body of a user-defined text frame (`TXXX`, or `TXX` in 2.2): the
/// encoding octet, the terminated description, then the values separated
/// by terminators.
#[must_use]
pub fn user_text(encoding: Encoding, description: &str, values: &[&str]) -> Vec<u8> {
    let mut body = vec![encoding.byte()];
    body.extend(encoding.terminated(description));
    body.extend(encoding.joined(values));
    body
}

/// The body of a comment (`COMM`, `COM`) or unsynchronised lyrics frame
/// (`USLT`, `ULT`): the encoding octet, the language, the terminated
/// description, then the text.
#[must_use]
pub fn comment(encoding: Encoding, language: [u8; 3], description: &str, text: &str) -> Vec<u8> {
    let mut body = vec![encoding.byte()];
    body.extend(language);
    body.extend(encoding.terminated(description));
    body.extend(encoding.encode(text));
    body
}

/// The body of a synchronised lyrics frame (`SYLT`, `SLT`): the encoding
/// octet, the language, the time stamp format, the content type, the
/// terminated description, then each line as terminated text followed by
/// its 32-bit time stamp.
#[must_use]
pub fn synced_lyrics(
    encoding: Encoding,
    language: [u8; 3],
    format_and_type: [u8; 2],
    description: &str,
    lines: &[(&str, u32)],
) -> Vec<u8> {
    let mut body = Bytes::new();
    body.u8(encoding.byte())
        .bytes(&language)
        .bytes(&format_and_type)
        .bytes(&encoding.terminated(description));
    for &(line, time) in lines {
        body.bytes(&encoding.terminated(line)).u32_be(time);
    }
    body.into_vec()
}

/// The body of an attached picture frame (`APIC`): the encoding octet, the
/// terminated ISO-8859-1 MIME type, the picture type, the terminated
/// description, then the picture data.
#[must_use]
pub fn picture(
    encoding: Encoding,
    mime: &str,
    picture_type: u8,
    description: &str,
    data: &[u8],
) -> Vec<u8> {
    let mut body = vec![encoding.byte()];
    body.extend(Encoding::Latin1.terminated(mime));
    body.push(picture_type);
    body.extend(encoding.terminated(description));
    body.extend_from_slice(data);
    body
}

/// The body of a 2.2 attached picture frame (`PIC`): the encoding octet, a
/// three-character image format such as `PNG`, the picture type, the
/// terminated description, then the picture data.
#[must_use]
pub fn picture_v22(
    encoding: Encoding,
    format: [u8; 3],
    picture_type: u8,
    description: &str,
    data: &[u8],
) -> Vec<u8> {
    let mut body = vec![encoding.byte()];
    body.extend(format);
    body.push(picture_type);
    body.extend(encoding.terminated(description));
    body.extend_from_slice(data);
    body
}

/// The body of a unique file identifier frame (`UFID`, `UFI`): the
/// terminated ISO-8859-1 owner, then the identifier.
#[must_use]
pub fn ufid(owner: &str, id: &[u8]) -> Vec<u8> {
    let mut body = Encoding::Latin1.terminated(owner);
    body.extend_from_slice(id);
    body
}

/// The body of a chapter frame (`CHAP`, `ID3v2` Chapter Frame Addendum 1.0):
/// the terminated element ID, the start and end times and the start and
/// end offsets, then the embedded frames.
#[must_use]
pub fn chapter(element_id: &str, times_and_offsets: [u32; 4], frames: &[u8]) -> Vec<u8> {
    let mut body = Bytes::new();
    body.bytes(&Encoding::Latin1.terminated(element_id));
    for value in times_and_offsets {
        body.u32_be(value);
    }
    body.bytes(frames);
    body.into_vec()
}

/// The body of a table of contents frame (`CTOC`): the terminated element
/// ID, the flags, the entry count, each terminated child element ID, then
/// the embedded frames.
///
/// # Panics
///
/// Panics when there are more than 255 children.
#[must_use]
pub fn table_of_contents(element_id: &str, flags: u8, children: &[&str], frames: &[u8]) -> Vec<u8> {
    let mut body = Bytes::new();
    body.bytes(&Encoding::Latin1.terminated(element_id))
        .u8(flags)
        .u8(u8::try_from(children.len()).expect("at most 255 children"));
    for child in children {
        body.bytes(&Encoding::Latin1.terminated(child));
    }
    body.bytes(frames);
    body.into_vec()
}

/// `bytes` with the unsynchronisation scheme applied (ID3v2.4.0 structure
/// section 6.1, ID3v2.3.0 section 5): a zero octet goes after every `FF`
/// that is followed by a zero octet or by an octet of `E0` or more, and
/// after an `FF` at the very end.
#[must_use]
pub fn unsynchronise(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for (index, &octet) in bytes.iter().enumerate() {
        out.push(octet);
        let next = bytes.get(index + 1);
        if octet == 0xFF && next.is_none_or(|&next| next == 0 || next >= 0xE0) {
            out.push(0);
        }
    }
    out
}

/// One frame as `version` writes it: the frame ID, the size of `body`, the
/// flags (2.3 and 2.4 only), then `body` as given. The size is three
/// octets in 2.2, 32 bits in 2.3 and a syncsafe integer in 2.4. A 2.3
/// frame laid into a 2.4 tag is how some writers get the 2.4 size wrong.
///
/// # Panics
///
/// Panics when `body` is too long for the version's size field.
#[must_use]
pub fn frame(version: Version, id: &[u8], flags: u16, body: &[u8]) -> Vec<u8> {
    let size = u32::try_from(body.len()).expect("a frame body fits a 32-bit size");
    let mut frame = Bytes::new();
    frame.bytes(id);
    match version {
        Version::V22 => frame.u24_be(size),
        Version::V23 => frame.u32_be(size).u16_be(flags),
        Version::V24 => frame.syncsafe32(size).u16_be(flags),
    };
    frame.bytes(body);
    frame.into_vec()
}

/// A whole tag: the header, an optional extended header, frames and other
/// octets in the order they were added, padding, and an optional footer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    version: Version,
    flags: u8,
    unsynchronised: bool,
    extended: Vec<u8>,
    frames: Vec<u8>,
    padding: usize,
    footer: bool,
}

impl Tag {
    /// An empty tag of `version`, revision 0, with no flags set.
    #[must_use]
    pub fn new(version: Version) -> Self {
        Self {
            version,
            flags: 0,
            unsynchronised: false,
            extended: Vec::new(),
            frames: Vec::new(),
            padding: 0,
            footer: false,
        }
    }

    /// Sets `flags` in the header's flags octet, alongside any the other
    /// builders set.
    #[must_use]
    pub fn flags(mut self, flags: u8) -> Self {
        self.flags |= flags;
        self
    }

    /// Sets the unsynchronisation flag. In 2.2 and 2.3 the scheme is then
    /// applied to everything after the header when the tag is built; in 2.4
    /// it applies to each frame, so frame bodies go in already
    /// unsynchronised (see [`unsynchronise`]).
    #[must_use]
    pub fn unsynchronised(mut self) -> Self {
        self.unsynchronised = true;
        self.flags(0x80)
    }

    /// Sets the extended header flag and writes `bytes`, laid out by the
    /// caller, right after the header.
    #[must_use]
    pub fn extended_header(mut self, bytes: &[u8]) -> Self {
        self.extended = bytes.to_vec();
        self.flags(0x40)
    }

    /// Appends a frame written by [`frame`] in this tag's version.
    #[must_use]
    pub fn frame(mut self, id: &[u8], flags: u16, body: &[u8]) -> Self {
        self.frames.extend(frame(self.version, id, flags, body));
        self
    }

    /// Appends `bytes` to the frames as they are, such as a frame written
    /// in another version or octets that are not a frame at all.
    #[must_use]
    pub fn bytes(mut self, bytes: &[u8]) -> Self {
        self.frames.extend_from_slice(bytes);
        self
    }

    /// Adds `len` zero octets of padding after the frames.
    #[must_use]
    pub fn padding(mut self, len: usize) -> Self {
        self.padding = len;
        self
    }

    /// Sets the footer flag and writes a footer after the tag (2.4).
    #[must_use]
    pub fn footer(mut self) -> Self {
        self.footer = true;
        self.flags(0x10)
    }

    /// The tag's octets.
    ///
    /// # Panics
    ///
    /// Panics when the tag is too long for its syncsafe size.
    #[must_use]
    pub fn build(&self) -> Vec<u8> {
        let mut body = self.extended.clone();
        body.extend_from_slice(&self.frames);
        body.resize(body.len() + self.padding, 0);
        if self.unsynchronised && self.version != Version::V24 {
            body = unsynchronise(&body);
        }
        let size = u32::try_from(body.len()).expect("a tag fits a syncsafe size");
        let mut tag = Bytes::new();
        tag.bytes(b"ID3")
            .u8(self.version.major())
            .u8(0)
            .u8(self.flags)
            .syncsafe32(size)
            .bytes(&body);
        if self.footer {
            tag.bytes(b"3DI")
                .u8(self.version.major())
                .u8(0)
                .u8(self.flags)
                .syncsafe32(size);
        }
        tag.into_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A complete ID3v2.4.0 tag written out octet by octet from the
    /// structure document: the header (section 3.1), an extended header
    /// marking an update (section 3.2), a `TIT2` text frame in ISO-8859-1
    /// and a `TPE1` frame with two UTF-8 values (section 4 and frames
    /// section 4.2), 4 octets of padding (section 3.3) and the footer
    /// (section 3.4).
    #[test]
    fn writes_a_whole_2_4_tag_exactly_as_the_specification_lays_it_out() {
        let tag = Tag::new(Version::V24)
            .extended_header(&[0x00, 0x00, 0x00, 0x07, 0x01, 0x40, 0x00])
            .frame(b"TIT2", 0x0000, &text(Encoding::Latin1, &["Song"]))
            .frame(b"TPE1", 0x4000, &text(Encoding::Utf8, &["A", "B"]))
            .padding(4)
            .footer()
            .build();
        let expected: Vec<u8> = [
            // "ID3", version 4.0, flags %01010000 (extended header, footer),
            // size 7 + 15 + 14 + 4 = 40 as a syncsafe integer.
            &[0x49, 0x44, 0x33, 0x04, 0x00, 0x50, 0x00, 0x00, 0x00, 0x28][..],
            // Extended header: size 7, one flag byte, %01000000 (update),
            // the update flag's data length 0.
            &[0x00, 0x00, 0x00, 0x07, 0x01, 0x40, 0x00],
            // TIT2, size 5, no flags, ISO-8859-1 "Song".
            &[0x54, 0x49, 0x54, 0x32, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00],
            &[0x00, 0x53, 0x6F, 0x6E, 0x67],
            // TPE1, size 4, %01000000 %00000000 (tag alter preservation),
            // UTF-8 "A" $00 "B".
            &[0x54, 0x50, 0x45, 0x31, 0x00, 0x00, 0x00, 0x04, 0x40, 0x00],
            &[0x03, 0x41, 0x00, 0x42],
            // Padding.
            &[0x00, 0x00, 0x00, 0x00],
            // "3DI", the same version, flags and size.
            &[0x33, 0x44, 0x49, 0x04, 0x00, 0x50, 0x00, 0x00, 0x00, 0x28],
        ]
        .concat();
        assert_eq!(tag, expected);
    }

    /// The size field's form in each version: 2.2 section 3.2 (three
    /// octets), 2.3 section 3.3 (32 bits) and 2.4 structure section 4
    /// (syncsafe). A body of 200 octets tells them apart.
    #[test]
    fn writes_frame_headers_as_each_version_does() {
        let body = [0xAA; 200];
        let mut v22 = b"TT2\x00\x00\xC8".to_vec();
        v22.extend(body);
        assert_eq!(frame(Version::V22, b"TT2", 0xFFFF, &body), v22);
        let mut v23 = b"TIT2\x00\x00\x00\xC8\x12\x34".to_vec();
        v23.extend(body);
        assert_eq!(frame(Version::V23, b"TIT2", 0x1234, &body), v23);
        let mut v24 = b"TIT2\x00\x00\x01\x48\x12\x34".to_vec();
        v24.extend(body);
        assert_eq!(frame(Version::V24, b"TIT2", 0x1234, &body), v24);
    }

    #[test]
    fn writes_each_version_and_no_flags_in_the_header() {
        for (version, major) in [(Version::V22, 2), (Version::V23, 3), (Version::V24, 4)] {
            assert_eq!(
                Tag::new(version).build(),
                [0x49, 0x44, 0x33, major, 0, 0, 0, 0, 0, 0],
                "{version:?}"
            );
        }
    }

    #[test]
    fn writes_raw_octets_and_flags_where_asked() {
        let tag = Tag::new(Version::V23)
            .flags(0x20)
            .flags(0x01)
            .bytes(b"tit2")
            .frame(b"TALB", 0, &[0x00])
            .build();
        assert_eq!(
            tag,
            b"ID3\x03\x00\x21\x00\x00\x00\x0Ftit2TALB\x00\x00\x00\x01\x00\x00\x00"
        );
    }

    /// ID3v2.4.0 structure section 6.1: `FF 00` becomes `FF 00 00`, and
    /// `%11111111 111xxxxx` becomes `%11111111 00000000 111xxxxx`. A final
    /// `FF` is followed by a zero octet too.
    #[test]
    fn unsynchronises_exactly_the_false_synchronisations() {
        let cases: [(&[u8], &[u8]); 7] = [
            (&[], &[]),
            (&[0xFF, 0x00], &[0xFF, 0x00, 0x00]),
            (&[0xFF, 0xE0], &[0xFF, 0x00, 0xE0]),
            (&[0xFF, 0xFF, 0xFB], &[0xFF, 0x00, 0xFF, 0x00, 0xFB]),
            (&[0xFF, 0xDF, 0xFE], &[0xFF, 0xDF, 0xFE]),
            (&[0x01, 0xFF], &[0x01, 0xFF, 0x00]),
            (&[0xFF, 0x01, 0xE0], &[0xFF, 0x01, 0xE0]),
        ];
        for (plain, unsynchronised) in cases {
            assert_eq!(unsynchronise(plain), unsynchronised, "{plain:02X?}");
        }
    }

    /// ID3v2.3.0 section 5: the scheme covers everything after the header,
    /// and the size counts the octets after it was applied.
    #[test]
    fn unsynchronises_the_whole_body_of_a_2_3_tag() {
        let tag = Tag::new(Version::V23)
            .unsynchronised()
            .extended_header(&[0xFF, 0x00])
            .frame(b"PRIV", 0, &[0xFF, 0xE1])
            .padding(1)
            .build();
        assert_eq!(
            tag,
            [
                &b"ID3\x03\x00\xC0\x00\x00\x00\x11"[..],
                &[0xFF, 0x00, 0x00],
                b"PRIV\x00\x00\x00\x02\x00\x00",
                &[0xFF, 0x00, 0xE1, 0x00],
            ]
            .concat()
        );
        let v22 = Tag::new(Version::V22)
            .unsynchronised()
            .frame(b"PRV", 0, &[0xFF])
            .build();
        assert_eq!(
            v22,
            b"ID3\x02\x00\x80\x00\x00\x00\x08PRV\x00\x00\x01\xFF\x00"
        );
    }

    /// In 2.4 the scheme is per frame, so the tag-level flag changes no
    /// octet after the header.
    #[test]
    fn leaves_a_2_4_body_as_given_when_the_flag_is_set() {
        let tag = Tag::new(Version::V24)
            .unsynchronised()
            .frame(b"PRIV", 0x0002, &[0xFF, 0x00, 0x00])
            .build();
        assert_eq!(
            tag,
            b"ID3\x04\x00\x80\x00\x00\x00\x0DPRIV\x00\x00\x00\x03\x00\x02\xFF\x00\x00"
        );
    }

    /// ID3v2.4.0 frames section 4.2: every encoding octet, and the
    /// terminators of section 4.
    #[test]
    fn encodes_text_in_every_encoding() {
        let cases: [(Encoding, u8, &[u8], &[u8]); 4] = [
            (Encoding::Latin1, 0, b"Caf\xE9", &[0]),
            (
                Encoding::Utf16,
                1,
                b"\xFF\xFEC\x00a\x00f\x00\xE9\x00",
                &[0, 0],
            ),
            (Encoding::Utf16Be, 2, b"\x00C\x00a\x00f\x00\xE9", &[0, 0]),
            (Encoding::Utf8, 3, b"Caf\xC3\xA9", &[0]),
        ];
        for (encoding, byte, encoded, terminator) in cases {
            assert_eq!(encoding.byte(), byte, "{encoding:?}");
            assert_eq!(encoding.encode("Café"), encoded, "{encoding:?}");
            assert_eq!(encoding.terminator(), terminator, "{encoding:?}");
            assert_eq!(
                encoding.terminated("Café"),
                [encoded, terminator].concat(),
                "{encoding:?}"
            );
        }
        // A character outside the Basic Multilingual Plane is a surrogate
        // pair.
        assert_eq!(Encoding::Utf16Be.encode("𝄞"), [0xD8, 0x34, 0xDD, 0x1E]);
    }

    #[test]
    #[should_panic(expected = "the text fits ISO-8859-1")]
    fn refuses_latin_1_text_it_cannot_encode() {
        let _ = Encoding::Latin1.encode("€");
    }

    #[test]
    fn writes_text_and_user_text_bodies() {
        assert_eq!(text(Encoding::Latin1, &[]), [0x00]);
        assert_eq!(text(Encoding::Utf8, &["a", "", "b"]), b"\x03a\x00\x00b");
        assert_eq!(
            text(Encoding::Utf16Be, &["a", "b"]),
            b"\x02\x00a\x00\x00\x00b"
        );
        assert_eq!(
            user_text(Encoding::Latin1, "key", &["v1", "v2"]),
            b"\x00key\x00v1\x00v2"
        );
        assert_eq!(user_text(Encoding::Utf8, "", &["v"]), b"\x03\x00v");
    }

    #[test]
    fn writes_comment_and_lyrics_bodies() {
        assert_eq!(
            comment(Encoding::Latin1, *b"eng", "d", "Text"),
            b"\x00engd\x00Text"
        );
        assert_eq!(
            comment(Encoding::Utf16, *b"deu", "", "x"),
            b"\x01deu\xFF\xFE\x00\x00\xFF\xFEx\x00"
        );
        assert_eq!(
            synced_lyrics(
                Encoding::Latin1,
                *b"eng",
                [2, 1],
                "d",
                &[("La", 1_000), ("Di", 0x0102_0304)],
            ),
            b"\x00eng\x02\x01d\x00La\x00\x00\x00\x03\xE8Di\x00\x01\x02\x03\x04"
        );
    }

    #[test]
    fn writes_picture_and_identifier_bodies() {
        assert_eq!(
            picture(Encoding::Utf8, "image/png", 3, "c", &[0x89, 0x50]),
            b"\x03image/png\x00\x03c\x00\x89\x50"
        );
        assert_eq!(
            picture_v22(Encoding::Latin1, *b"JPG", 4, "", &[0xFF, 0xD8]),
            b"\x00JPG\x04\x00\xFF\xD8"
        );
        assert_eq!(
            ufid("http://musicbrainz.org", b"id"),
            b"http://musicbrainz.org\x00id"
        );
    }

    /// The `ID3v2` Chapter Frame Addendum 1.0, sections 3 and 4.
    #[test]
    fn writes_chapter_and_table_of_contents_bodies() {
        assert_eq!(
            chapter("ch1", [1, 2, 0xFFFF_FFFF, 0x0A0B_0C0D], b"TIT2"),
            [
                &b"ch1\x00"[..],
                &[0, 0, 0, 1, 0, 0, 0, 2, 0xFF, 0xFF, 0xFF, 0xFF],
                &[0x0A, 0x0B, 0x0C, 0x0D],
                b"TIT2",
            ]
            .concat()
        );
        assert_eq!(
            table_of_contents("toc", 0x03, &["ch1", "ch2"], b"x"),
            b"toc\x00\x03\x02ch1\x00ch2\x00x"
        );
        assert_eq!(table_of_contents("", 0, &[], &[]), b"\x00\x00\x00");
    }

    #[test]
    #[should_panic(expected = "at most 255 children")]
    fn refuses_more_children_than_the_entry_count_holds() {
        let children = ["c"; 256];
        let _ = table_of_contents("toc", 0, &children, &[]);
    }
}
