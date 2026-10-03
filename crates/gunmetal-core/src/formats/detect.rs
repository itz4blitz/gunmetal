//! Which allowlisted format a file is, decided by its content (SEC-MED-011,
//! SEC-HIS-017).
//!
//! Detection compares the first octets of a file with the signatures of the
//! formats on a closed allowlist: FLAC, MP3, MP4 audio, Ogg, WAV and AIFF;
//! and the sidecars JPEG, PNG, WebP, GIF, LRC and M3U. A file that matches
//! none is [`DetectError::Unknown`] and must be skipped, never handed to a
//! more general decoder.
//!
//! # What the extension does
//!
//! The extension can rule formats out but never rule one in. It names the
//! kind of file its name claims, and only the formats of that kind are
//! candidates, so a file can be detected only as a format its content has
//! and its name claims. A PNG named `.flac` is unknown; a FLAC named `.mp3`
//! is FLAC. The names, compared without regard to ASCII case and given
//! without the dot, are:
//!
//! | Kind | Extensions | Candidates |
//! |---|---|---|
//! | Audio | `aif`, `aifc`, `aiff`, `flac`, `m4a`, `m4b`, `mp3`, `mp4`, `oga`, `ogg`, `opus`, `wav` | FLAC, MP3, MP4 audio, Ogg, WAV, AIFF |
//! | Image | `gif`, `jpeg`, `jpg`, `png`, `webp` | JPEG, PNG, WebP, GIF |
//! | Lyrics | `lrc` | LRC |
//! | Playlist | `m3u`, `m3u8` | M3U |
//!
//! Any other extension leaves no candidate. With no extension, as for a
//! picture embedded in a tag, every format with a binary signature is a
//! candidate. No two binary signatures can match the same octets, so the
//! order in which they are tried cannot change the answer.
//!
//! LRC and M3U have no signature, so only their own names admit them, and
//! then for any text: a head with no binary signature whose octets are all
//! tabs, line breaks, printable ASCII or octets from `0x80` up (UTF-8, or a
//! legacy single-byte encoding).
//!
//! # Leading `ID3v2` tags
//!
//! Where audio is a candidate, an `ID3v2` tag of version 2.2, 2.3 or 2.4 at
//! the start of the file is skipped by its declared size, and the detector
//! reads on after it. Only FLAC and MP3 may follow a tag: `ID3v2` is defined
//! as a prefix of MPEG audio, and taggers put it in front of FLAC too.
//! [`Detected::start`] says where the format's own content begins, so the
//! octets before it are the tags. A tag that runs past the end of the file
//! is [`ParseFault::Truncated`].
//!
//! # Work
//!
//! The detector is a sans-I/O parser ([`SansIo`]). It asks for at most 512
//! octets at a time, always inside the file and each read after the last,
//! and for at most eight reads: the start of the file and what follows each
//! of up to seven leading tags. Each read costs one step of its [`Budget`],
//! so its step bound is k = 0 steps per octet and c = 8 (SEC-MED-007); a
//! file with more leading tags than that is [`ParseFault::BudgetExceeded`].

use crate::parse::{Budget, Cursor, ParseFault, ReadRequest, SansIo, Step, Window};

/// A format on the allowlist (SEC-MED-011), with the signature it is
/// detected by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// FLAC: `fLaC`.
    Flac,
    /// MP3: the header of an MPEG-1, MPEG-2 or MPEG-2.5 Layer III frame
    /// with a bitrate other than free or forbidden and a known sample rate.
    /// Layers I and II are not on the allowlist.
    Mpeg,
    /// MP4 audio: an `ftyp` box of 16 to 4,096 octets whose major brand is
    /// `M4A `, `M4B `, `isom`, `iso2`, `mp41` or `mp42`.
    Mp4,
    /// Ogg: the capture pattern `OggS` and stream structure version 0.
    Ogg,
    /// WAV: a `RIFF` or `RF64` file of form type `WAVE`.
    Wav,
    /// AIFF: an IFF `FORM` of type `AIFF` or `AIFC`.
    Aiff,
    /// JPEG: the start-of-image marker and the first octet of another.
    Jpeg,
    /// PNG: the eight-octet PNG signature.
    Png,
    /// WebP: a `RIFF` file of form type `WEBP`.
    Webp,
    /// GIF: `GIF87a` or `GIF89a`.
    Gif,
    /// LRC lyrics, which have no signature: text named `.lrc`.
    Lrc,
    /// An M3U or M3U8 playlist, which has no signature: text named `.m3u`
    /// or `.m3u8`.
    M3u,
}

impl Format {
    /// Whether this is an audio format rather than an image or text.
    const fn is_audio(self) -> bool {
        matches!(
            self,
            Self::Flac | Self::Mpeg | Self::Mp4 | Self::Ogg | Self::Wav | Self::Aiff
        )
    }

    /// Whether this is an image format rather than audio or text.
    const fn is_image(self) -> bool {
        matches!(self, Self::Jpeg | Self::Png | Self::Webp | Self::Gif)
    }
}

/// What detection found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Detected {
    /// The format.
    pub format: Format,
    /// The file offset where the format's own content starts: 0, or the end
    /// of the `ID3v2` tags in front of it.
    pub start: u64,
}

/// Why detection found no format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectError {
    /// The content at `offset` matched no candidate, so the file must be
    /// skipped (SEC-MED-011).
    Unknown {
        /// Where that content starts: 0, or the end of the leading `ID3v2`
        /// tags.
        offset: u64,
    },
    /// A leading `ID3v2` tag had a size that is not syncsafe or ran past the
    /// end of the file, or the file held more leading tags than the budget
    /// allows.
    Fault(ParseFault),
}

/// The most octets the detector asks for at once: enough for every binary
/// signature, and the head of a text file that must be text throughout.
const HEAD: u32 = 512;

/// The reads one detection may make, as its step budget: the start of the
/// file and what follows each of up to seven leading `ID3v2` tags.
const READS: u64 = 8;

/// The major brands of MP4 audio files.
const MP4_BRANDS: [[u8; 4]; 6] = [*b"M4A ", *b"M4B ", *b"isom", *b"iso2", *b"mp41", *b"mp42"];

/// Which formats a detection may answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Candidates {
    /// No extension: every format with a binary signature.
    Binary,
    /// An audio extension: the audio formats.
    Audio,
    /// An image extension: the image formats.
    Image,
    /// The extension of a text format: that format, for any text.
    Text(Format),
    /// The octets after a leading `ID3v2` tag: FLAC or MP3, or another tag.
    Tagged,
    /// An extension outside the allowlist: nothing.
    Nothing,
}

/// Every extension on the allowlist, with the candidates it names.
const EXTENSIONS: [(&str, Candidates); 20] = [
    ("aif", Candidates::Audio),
    ("aifc", Candidates::Audio),
    ("aiff", Candidates::Audio),
    ("flac", Candidates::Audio),
    ("gif", Candidates::Image),
    ("jpeg", Candidates::Image),
    ("jpg", Candidates::Image),
    ("lrc", Candidates::Text(Format::Lrc)),
    ("m3u", Candidates::Text(Format::M3u)),
    ("m3u8", Candidates::Text(Format::M3u)),
    ("m4a", Candidates::Audio),
    ("m4b", Candidates::Audio),
    ("mp3", Candidates::Audio),
    ("mp4", Candidates::Audio),
    ("oga", Candidates::Audio),
    ("ogg", Candidates::Audio),
    ("opus", Candidates::Audio),
    ("png", Candidates::Image),
    ("wav", Candidates::Audio),
    ("webp", Candidates::Image),
];

impl Candidates {
    /// The candidates for a file with extension `ext_hint`.
    fn named(ext_hint: Option<&str>) -> Self {
        ext_hint.map_or(Self::Binary, |ext| {
            EXTENSIONS
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(ext))
                .map_or(Self::Nothing, |&(_, candidates)| candidates)
        })
    }

    /// Whether an `ID3v2` tag may be skipped here: only where audio is a
    /// candidate.
    const fn admit_tags(self) -> bool {
        matches!(self, Self::Binary | Self::Audio | Self::Tagged)
    }

    /// The candidate that `bytes`, a head that does not start with a tag,
    /// are.
    fn classify(self, bytes: &[u8]) -> Option<Format> {
        let signed = signature(bytes);
        match self {
            Self::Binary => signed,
            Self::Audio => signed.filter(|format| format.is_audio()),
            Self::Image => signed.filter(|format| format.is_image()),
            Self::Text(format) => (signed.is_none() && is_text(bytes)).then_some(format),
            Self::Tagged => signed.filter(|format| matches!(format, Format::Flac | Format::Mpeg)),
            Self::Nothing => None,
        }
    }
}

/// Detects the format of one file: a sans-I/O parser made by [`detect`].
///
/// Like a [`Budget`], a detector is deliberately not `Clone`: one file has
/// one detection.
#[derive(Debug)]
pub struct Detector {
    /// The formats it may still answer.
    candidates: Candidates,
    /// The reads it may still make.
    budget: Budget,
}

/// A detector for one file, whose name has the extension `ext_hint`
/// (without the dot), or no extension.
#[must_use]
pub fn detect(ext_hint: Option<&str>) -> Detector {
    Detector {
        candidates: Candidates::named(ext_hint),
        budget: Budget::for_input(0, 0, READS),
    }
}

impl Detector {
    /// Asks for the head at `offset` of a file of `file_len` octets, or ends
    /// the detection when nothing is left there or the budget is spent.
    fn read_at(&mut self, offset: u64, file_len: u64) -> Step<Result<Detected, DetectError>> {
        let remaining = file_len.saturating_sub(offset);
        if remaining == 0 {
            return Step::Done(Err(DetectError::Unknown { offset }));
        }
        if let Err(fault) = self.budget.charge(1, offset) {
            return Step::Done(Err(DetectError::Fault(fault)));
        }
        // remaining is at least 1. A head of HEAD octets, or the rest of a
        // shorter file, always fits in u32.
        let len = match u32::try_from(remaining) {
            Ok(short) if short <= HEAD => short,
            _ => HEAD,
        };
        Step::Need(ReadRequest { offset, len })
    }
}

impl SansIo for Detector {
    type Output = Result<Detected, DetectError>;

    fn resume(&mut self, window: Window<'_>) -> Step<Self::Output> {
        let Window {
            offset,
            bytes,
            file_len,
        } = window;
        // Only the first window of a parse is empty: every read the
        // detector asks for holds at least one octet.
        if bytes.is_empty() {
            return self.read_at(offset, file_len);
        }
        let tag = if self.candidates.admit_tags() {
            id3v2_end(window)
        } else {
            Ok(None)
        };
        match tag {
            Err(fault) => Step::Done(Err(DetectError::Fault(fault))),
            Ok(Some(end)) => {
                self.candidates = Candidates::Tagged;
                self.read_at(end, file_len)
            }
            Ok(None) => Step::Done(
                self.candidates
                    .classify(bytes)
                    .map(|format| Detected {
                        format,
                        start: offset,
                    })
                    .ok_or(DetectError::Unknown { offset }),
            ),
        }
    }
}

/// Where the `ID3v2` tag at the start of `window` ends, or `None` when the
/// window does not start with the header of an `ID3v2` tag of version 2.2,
/// 2.3 or 2.4: the identifier `ID3`, a major version of 2 to 4 and a
/// revision below `0xFF` (`ID3v2.4` structure, section 3.1).
fn id3v2_end(window: Window<'_>) -> Result<Option<u64>, ParseFault> {
    let Window {
        offset,
        bytes,
        file_len,
    } = window;
    let &[b'I', b'D', b'3', 2..=4, 0..=0xFE, ..] = bytes else {
        return Ok(None);
    };
    let [_, _, _, major, _, flags, size @ ..] = window.cursor().array::<10>()?;
    let size = Cursor::at(&size, offset.saturating_add(6)).syncsafe_u32()?;
    // Only version 2.4 has a footer, which flag bit 4 announces.
    let footer = if major == 4 && flags & 0x10 != 0 {
        10
    } else {
        0
    };
    // At most 10 + (2^28 - 1) + 10, so nothing saturates.
    let len = u64::from(size).saturating_add(10).saturating_add(footer);
    match offset.checked_add(len) {
        Some(end) if end <= file_len => Ok(Some(end)),
        _ => Err(ParseFault::Truncated {
            offset,
            needed: len,
            available: file_len.saturating_sub(offset),
        }),
    }
}

/// The format whose binary signature `bytes` start with. No two signatures
/// match the same octets: their first octets differ, except MP3 and JPEG,
/// whose second octets do, and WAV and WebP, whose form types do.
fn signature(bytes: &[u8]) -> Option<Format> {
    match *bytes {
        [b'f', b'L', b'a', b'C', ..] => Some(Format::Flac),
        // 11 sync bits; version MPEG-2.5 (00), MPEG-2 (10) or MPEG-1 (11),
        // never the reserved 01; Layer III (01); either protection bit. Then
        // a bitrate index of 1 to 14, so neither free (0) nor forbidden (15),
        // and a sample-rate index other than the reserved 3.
        [
            0xFF,
            0xE2 | 0xE3 | 0xF2 | 0xF3 | 0xFA | 0xFB,
            third @ 0x10..=0xEF,
            ..,
        ] if third & 0x0C != 0x0C => Some(Format::Mpeg),
        [0xFF, 0xD8, 0xFF, ..] => Some(Format::Jpeg),
        [s0, s1, s2, s3, b'f', b't', b'y', b'p', b0, b1, b2, b3, ..]
            if is_mp4_audio(u32::from_be_bytes([s0, s1, s2, s3]), [b0, b1, b2, b3]) =>
        {
            Some(Format::Mp4)
        }
        [b'O', b'g', b'g', b'S', 0, ..] => Some(Format::Ogg),
        [
            b'R',
            b'I',
            b'F',
            b'F',
            _,
            _,
            _,
            _,
            b'W',
            b'A',
            b'V',
            b'E',
            ..,
        ]
        | [
            b'R',
            b'F',
            b'6',
            b'4',
            _,
            _,
            _,
            _,
            b'W',
            b'A',
            b'V',
            b'E',
            ..,
        ] => Some(Format::Wav),
        [
            b'R',
            b'I',
            b'F',
            b'F',
            _,
            _,
            _,
            _,
            b'W',
            b'E',
            b'B',
            b'P',
            ..,
        ] => Some(Format::Webp),
        [
            b'F',
            b'O',
            b'R',
            b'M',
            _,
            _,
            _,
            _,
            b'A',
            b'I',
            b'F',
            b'F' | b'C',
            ..,
        ] => Some(Format::Aiff),
        [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n', ..] => Some(Format::Png),
        [b'G', b'I', b'F', b'8', b'7' | b'9', b'a', ..] => Some(Format::Gif),
        _ => None,
    }
}

/// Whether an `ftyp` box of `size` octets with major brand `brand` starts
/// an MP4 audio file. The box holds at least the major and minor brands, 16
/// octets, and real ones hold a few compatible brands more; bounding the
/// size also keeps the box's first octet zero, unlike every other
/// signature's.
fn is_mp4_audio(size: u32, brand: [u8; 4]) -> bool {
    (16..=4_096).contains(&size) && MP4_BRANDS.contains(&brand)
}

/// Whether `bytes` are text: tabs, line breaks, printable ASCII, or octets
/// from `0x80` up.
fn is_text(bytes: &[u8]) -> bool {
    bytes
        .iter()
        .all(|&octet| matches!(octet, b'\t' | b'\n' | b'\r' | 0x20..=0x7E | 0x80..=0xFF))
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::Format::{Aiff, Flac, Gif, Jpeg, Lrc, M3u, Mp4, Mpeg, Ogg, Png, Wav, Webp};
    use super::*;
    use crate::parse::ParseFault::{BudgetExceeded, NotSyncsafe, Truncated};
    use crate::parse::small_stack::on_small_stack;
    use crate::parse::{Limits, ReadGuard, ReadRequest, drive};
    use gunmetal_testkit::bytes::{Bits, Bytes};
    use proptest::collection::vec;
    use proptest::prelude::*;
    use proptest::sample::{Index, select};

    /// What one whole detection returned.
    type Outcome = Result<Detected, DetectError>;

    /// The most reads a test lets one detection make. Detection stops itself
    /// after eight; a detector that never stops fails its test here instead
    /// of hanging it.
    const CEILING: usize = 16;

    /// Runs detection over `file` as [`drive`] does, and returns every read
    /// it asked for with what it returned. A read the host refuses, which a
    /// detector never asks for, fails the test.
    fn trace(file: &[u8], hint: Option<&str>) -> (Vec<ReadRequest>, Outcome) {
        let file_len = u64::try_from(file.len()).expect("a test file fits in u64");
        let mut detector = detect(hint);
        let mut guard = ReadGuard::new(file_len, &Limits::DEFAULT);
        let mut window = Window::start(file_len);
        let mut reads = Vec::new();
        loop {
            match detector.resume(window) {
                Step::Done(output) => return (reads, output),
                Step::Need(request) => {
                    reads.push(request);
                    assert!(reads.len() <= CEILING, "detection did not stop: {reads:?}");
                    assert_eq!(guard.admit(request), Ok(()), "after {reads:?}");
                    let start = usize::try_from(request.offset).expect("inside a test file");
                    let len = usize::try_from(request.len).expect("a read fits in usize");
                    window = Window {
                        offset: request.offset,
                        bytes: &file[start..start + len],
                        file_len,
                    };
                }
            }
        }
    }

    /// What detection returns for `file` named with extension `hint`.
    fn run(file: &[u8], hint: Option<&str>) -> Outcome {
        trace(file, hint).1
    }

    #[expect(
        clippy::unnecessary_wraps,
        reason = "it builds the whole result a detection is compared with"
    )]
    const fn found(format: Format, start: u64) -> Outcome {
        Ok(Detected { format, start })
    }

    const fn unknown(offset: u64) -> Outcome {
        Err(DetectError::Unknown { offset })
    }

    const fn fault(fault: ParseFault) -> Outcome {
        Err(DetectError::Fault(fault))
    }

    const fn read(offset: u64, len: u32) -> ReadRequest {
        ReadRequest { offset, len }
    }

    fn joined(parts: &[&[u8]]) -> Vec<u8> {
        parts.concat()
    }

    // Files as each format's specification lays out its first octets,
    // written independently of the code under test.

    /// The start of a FLAC stream (RFC 9639, section 8): the marker, then a
    /// STREAMINFO block that is also the last metadata block, for 4,096
    /// samples a block at 44.1 kHz, two channels, 16 bits, an unknown length
    /// and no MD5. 42 octets.
    fn flac() -> Vec<u8> {
        let mut info = Bits::new();
        info.put(20, 44_100).put(3, 1).put(5, 15).put(36, 0);
        let mut file = Bytes::new();
        file.bytes(b"fLaC")
            .u8(0x80)
            .u24_be(34)
            .u16_be(4_096)
            .u16_be(4_096)
            .u24_be(0)
            .u24_be(0)
            .bytes(&info.into_vec())
            .zeros(16);
        file.into_vec()
    }

    /// An MPEG audio frame header (ISO/IEC 11172-3, 2.4.1.3) with the given
    /// version, layer, bitrate and sample-rate codes, no CRC, joint stereo
    /// and an original copy, then 32 octets of side information, all zero.
    fn mpeg(version: u64, layer: u64, bitrate: u64, rate: u64) -> Vec<u8> {
        let mut header = Bits::new();
        header
            .put(11, 0x7FF)
            .put(2, version)
            .put(2, layer)
            .put(1, 1)
            .put(4, bitrate)
            .put(2, rate)
            .put(1, 0)
            .put(1, 0)
            .put(2, 0b01)
            .put(2, 0)
            .put(1, 0)
            .put(1, 1)
            .put(2, 0);
        let mut file = header.into_vec();
        file.extend([0; 32]);
        file
    }

    /// An MP3 frame: MPEG-1 Layer III at 128 kbit/s and 44.1 kHz. 36 octets.
    fn mp3() -> Vec<u8> {
        mpeg(0b11, 0b01, 0b1001, 0b00)
    }

    /// An `ID3v2` tag (`ID3v2`.4 structure, section 3): version 2.`major`,
    /// revision 0, `flags`, a syncsafe size and a body of `size` zero
    /// octets, then a footer when `footer` is set.
    fn id3v2(major: u8, flags: u8, size: u32, footer: bool) -> Vec<u8> {
        let mut tag = Bytes::new();
        tag.bytes(b"ID3")
            .u8(major)
            .u8(0)
            .u8(flags)
            .syncsafe32(size)
            .zeros(usize::try_from(size).expect("a small tag"));
        if footer {
            tag.bytes(b"3DI").u8(major).u8(0).u8(flags).syncsafe32(size);
        }
        tag.into_vec()
    }

    /// `count` `ID3v2`.3 tags with empty bodies, ten octets each.
    fn empty_tags(count: usize) -> Vec<u8> {
        (0..count).flat_map(|_| id3v2(3, 0, 0, false)).collect()
    }

    /// An `ftyp` box (ISO/IEC 14496-12, 4.3) with major brand `brand`
    /// followed by `extra` zero octets, so the box is 12 + `extra` octets.
    fn ftyp(brand: [u8; 4], extra: usize) -> Vec<u8> {
        let mut body = Bytes::new();
        body.bytes(&brand).zeros(extra);
        let mut file = Bytes::new();
        file.mp4_box(*b"ftyp", body.as_slice());
        file.into_vec()
    }

    /// The start of an M4A file: an `ftyp` box with major brand `M4A `, minor
    /// version 0 and compatible brands `M4A `, `mp42` and `isom`, then an
    /// empty `free` box.
    fn m4a() -> Vec<u8> {
        let mut body = Bytes::new();
        body.bytes(b"M4A ").u32_be(0).bytes(b"M4A mp42isom");
        let mut file = Bytes::new();
        file.mp4_box(*b"ftyp", body.as_slice())
            .mp4_box(*b"free", &[]);
        file.into_vec()
    }

    /// The first page of an Ogg stream (RFC 3533, section 6): version 0, the
    /// beginning-of-stream flag, one 19-octet segment, holding the start of
    /// an `OpusHead` packet.
    fn ogg() -> Vec<u8> {
        let mut file = Bytes::new();
        file.bytes(b"OggS")
            .u8(0)
            .u8(0x02)
            .u64_le(0)
            .u32_le(1)
            .u32_le(0)
            .u32_le(0)
            .u8(1)
            .u8(19)
            .bytes(b"OpusHead")
            .u8(1)
            .u8(2)
            .u16_le(312)
            .u32_le(48_000)
            .u16_le(0)
            .u8(0);
        file.into_vec()
    }

    /// A RIFF WAVE file: a PCM `fmt ` chunk for two channels of 16 bits at
    /// 44.1 kHz, and an empty `data` chunk.
    fn wav() -> Vec<u8> {
        let mut format = Bytes::new();
        format
            .u16_le(1)
            .u16_le(2)
            .u32_le(44_100)
            .u32_le(176_400)
            .u16_le(4)
            .u16_le(16);
        let mut body = Bytes::new();
        body.bytes(b"WAVE")
            .riff_chunk(*b"fmt ", format.as_slice())
            .riff_chunk(*b"data", &[]);
        let mut file = Bytes::new();
        file.riff_chunk(*b"RIFF", body.as_slice());
        file.into_vec()
    }

    /// An RF64 WAVE file (EBU Tech 3306): a RIFF size of `0xFFFFFFFF`, and the
    /// real sizes in a `ds64` chunk.
    fn rf64() -> Vec<u8> {
        let mut file = Bytes::new();
        file.bytes(b"RF64")
            .u32_le(u32::MAX)
            .bytes(b"WAVE")
            .riff_chunk(*b"ds64", &[0; 28]);
        file.into_vec()
    }

    /// An IFF `FORM` of type `form`, `AIFF` or `AIFC`, holding a `COMM` chunk
    /// for two channels of 16 bits at 44.1 kHz (an 80-bit extended rate).
    fn aiff(form: [u8; 4]) -> Vec<u8> {
        let mut common = Bytes::new();
        common
            .u16_be(2)
            .u32_be(0)
            .u16_be(16)
            .bytes(&[0x40, 0x0E, 0xAC, 0x44, 0, 0, 0, 0, 0, 0]);
        let mut body = Bytes::new();
        body.bytes(&form).aiff_chunk(*b"COMM", common.as_slice());
        let mut file = Bytes::new();
        file.aiff_chunk(*b"FORM", body.as_slice());
        file.into_vec()
    }

    /// The start of a JFIF JPEG file: the SOI marker, then an APP0 segment.
    fn jpeg() -> Vec<u8> {
        let mut file = Bytes::new();
        file.u16_be(0xFFD8)
            .u16_be(0xFFE0)
            .u16_be(16)
            .bytes(b"JFIF\0")
            .u8(1)
            .u8(1)
            .u8(0)
            .u16_be(1)
            .u16_be(1)
            .u8(0)
            .u8(0);
        file.into_vec()
    }

    /// The start of a PNG file: the signature, then the IHDR chunk of a 1×1
    /// RGBA image, its CRC left zero.
    fn png() -> Vec<u8> {
        let mut file = Bytes::new();
        file.bytes(&[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n'])
            .u32_be(13)
            .bytes(b"IHDR")
            .u32_be(1)
            .u32_be(1)
            .u8(8)
            .u8(6)
            .u8(0)
            .u8(0)
            .u8(0)
            .u32_be(0);
        file.into_vec()
    }

    /// A WebP file: a RIFF of type `WEBP` holding the start of a lossless
    /// `VP8L` chunk.
    fn webp() -> Vec<u8> {
        let mut body = Bytes::new();
        body.bytes(b"WEBP")
            .riff_chunk(*b"VP8L", &[0x2F, 0, 0, 0, 0]);
        let mut file = Bytes::new();
        file.riff_chunk(*b"RIFF", body.as_slice());
        file.into_vec()
    }

    /// The start of a GIF file of `version` (`87a` or `89a`): a 1×1 logical
    /// screen with no colour table.
    fn gif(version: [u8; 3]) -> Vec<u8> {
        let mut file = Bytes::new();
        file.bytes(b"GIF")
            .bytes(&version)
            .u16_le(1)
            .u16_le(1)
            .u8(0)
            .u8(0)
            .u8(0);
        file.into_vec()
    }

    /// LRC lyrics.
    const LYRICS: &[u8] = b"[ar:Artist]\n[ti:Title]\n[00:12.34]First line\n[00:15.00]Second\n";
    /// An extended M3U playlist.
    const PLAYLIST: &[u8] = b"#EXTM3U\n#EXTINF:215,Artist - Title\nMusic/Artist/Title.flac\n";
    /// Text that is neither, such as `/etc/passwd`.
    const TEXT: &[u8] = b"root:x:0:0:root:/root:/bin/bash\n";
    /// An HTML page.
    const HTML: &[u8] = b"<!DOCTYPE html>\n<html><body><script>alert(1)</script></body></html>\n";
    /// The start of a PDF file.
    const PDF: &[u8] = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n1 0 obj\n<< /Type /Catalog >>\nendobj\n";

    /// Verifies: SEC-MED-011, SEC-HIS-017
    #[test]
    fn detects_every_allowlisted_format_from_its_content() {
        let cases: [(Vec<u8>, Option<&str>, Format); 24] = [
            (flac(), Some("flac"), Flac),
            (flac(), None, Flac),
            (mp3(), Some("mp3"), Mpeg),
            (mp3(), None, Mpeg),
            (m4a(), Some("m4a"), Mp4),
            (m4a(), None, Mp4),
            (ogg(), Some("ogg"), Ogg),
            (ogg(), None, Ogg),
            (wav(), Some("wav"), Wav),
            (rf64(), Some("wav"), Wav),
            (wav(), None, Wav),
            (aiff(*b"AIFF"), Some("aiff"), Aiff),
            (aiff(*b"AIFC"), Some("aifc"), Aiff),
            (aiff(*b"AIFF"), None, Aiff),
            (jpeg(), Some("jpg"), Jpeg),
            (jpeg(), None, Jpeg),
            (png(), Some("png"), Png),
            (png(), None, Png),
            (webp(), Some("webp"), Webp),
            (webp(), None, Webp),
            (gif(*b"87a"), Some("gif"), Gif),
            (gif(*b"89a"), None, Gif),
            (LYRICS.to_vec(), Some("lrc"), Lrc),
            (PLAYLIST.to_vec(), Some("m3u"), M3u),
        ];
        for (file, hint, format) in cases {
            assert_eq!(
                run(&file, hint),
                found(format, 0),
                "{format:?} named {hint:?}"
            );
        }
    }

    /// The plan's mislabelled files: within the kind a name claims, the
    /// content decides.
    ///
    /// Verifies: SEC-MED-011, SEC-HIS-017
    #[test]
    fn the_content_decides_not_the_extension() {
        assert_eq!(run(&flac(), Some("mp3")), found(Flac, 0));
        assert_eq!(run(&mp3(), Some("flac")), found(Mpeg, 0));
        // A PNG named `cover.jpg`.
        assert_eq!(run(&png(), Some("jpg")), found(Png, 0));
        assert_eq!(run(&ogg(), Some("m4a")), found(Ogg, 0));
        assert_eq!(run(&m4a(), Some("opus")), found(Mp4, 0));
        assert_eq!(run(&wav(), Some("aiff")), found(Wav, 0));
        assert_eq!(run(&gif(*b"89a"), Some("webp")), found(Gif, 0));
    }

    /// A name can rule a format out, never in: content of another kind than
    /// the name claims is no format at all. SEC-MED-011 names a PNG called
    /// `.flac`, HTML called `.mp3` and a PDF called `.gif`; SEC-HIS-017 names
    /// `/etc/passwd` called `passwd.flac`; the plan names a text file called
    /// `.flac`.
    ///
    /// Verifies: SEC-MED-011, SEC-HIS-017
    #[test]
    fn content_of_another_kind_than_its_name_claims_is_unknown() {
        let cases: [(Vec<u8>, &str); 12] = [
            (png(), "flac"),
            (HTML.to_vec(), "mp3"),
            (PDF.to_vec(), "gif"),
            (TEXT.to_vec(), "flac"),
            (flac(), "jpg"),
            (mp3(), "png"),
            (jpeg(), "mp3"),
            (webp(), "wav"),
            (flac(), "lrc"),
            (mp3(), "m3u"),
            (LYRICS.to_vec(), "flac"),
            (PLAYLIST.to_vec(), "jpg"),
        ];
        for (file, hint) in cases {
            assert_eq!(
                run(&file, Some(hint)),
                unknown(0),
                "{file:02X?} named {hint}"
            );
        }
        // Without a name, no text is a format either.
        for file in [TEXT, HTML, PDF] {
            assert_eq!(run(file, None), unknown(0));
        }
    }

    /// Verifies: SEC-MED-011
    #[test]
    fn a_name_outside_the_allowlist_rules_every_format_out() {
        let names = [
            "txt", "", ".flac", "fla", "flac2", "aac", "mp2", "wma", "flac ", "jpg\0",
        ];
        for name in names {
            assert_eq!(run(&flac(), Some(name)), unknown(0), "{name:?}");
            assert_eq!(run(&png(), Some(name)), unknown(0), "{name:?}");
            assert_eq!(run(LYRICS, Some(name)), unknown(0), "{name:?}");
        }
    }

    /// Verifies: SEC-MED-011
    #[test]
    fn names_are_compared_without_regard_to_case() {
        assert_eq!(run(&flac(), Some("FLAC")), found(Flac, 0));
        assert_eq!(run(&mp3(), Some("Mp3")), found(Mpeg, 0));
        assert_eq!(run(&png(), Some("JPG")), found(Png, 0));
        assert_eq!(run(&webp(), Some("WebP")), found(Webp, 0));
        assert_eq!(run(PLAYLIST, Some("M3U8")), found(M3u, 0));
        assert_eq!(run(LYRICS, Some("Lrc")), found(Lrc, 0));
    }

    /// Verifies: SEC-MED-011
    #[test]
    fn every_name_on_the_allowlist_admits_its_kind() {
        let cases: [(&str, Vec<u8>, Format); 20] = [
            ("aif", aiff(*b"AIFF"), Aiff),
            ("aifc", aiff(*b"AIFC"), Aiff),
            ("aiff", aiff(*b"AIFF"), Aiff),
            ("flac", flac(), Flac),
            ("gif", gif(*b"89a"), Gif),
            ("jpeg", jpeg(), Jpeg),
            ("jpg", jpeg(), Jpeg),
            ("lrc", LYRICS.to_vec(), Lrc),
            ("m3u", PLAYLIST.to_vec(), M3u),
            ("m3u8", PLAYLIST.to_vec(), M3u),
            ("m4a", m4a(), Mp4),
            ("m4b", m4a(), Mp4),
            ("mp3", mp3(), Mpeg),
            ("mp4", m4a(), Mp4),
            ("oga", ogg(), Ogg),
            ("ogg", ogg(), Ogg),
            ("opus", ogg(), Ogg),
            ("png", png(), Png),
            ("wav", wav(), Wav),
            ("webp", webp(), Webp),
        ];
        for (name, file, format) in cases {
            assert_eq!(run(&file, Some(name)), found(format, 0), "{name}");
        }
    }

    /// LRC and M3U have no signature, so only their own names admit them,
    /// and then any text is that format.
    ///
    /// Verifies: SEC-MED-011
    #[test]
    fn text_formats_need_their_own_name() {
        assert_eq!(run(LYRICS, Some("lrc")), found(Lrc, 0));
        assert_eq!(run(PLAYLIST, Some("m3u8")), found(M3u, 0));
        assert_eq!(run(LYRICS, None), unknown(0));
        assert_eq!(run(PLAYLIST, None), unknown(0));
        assert_eq!(run(PLAYLIST, Some("lrc")), found(Lrc, 0));
        assert_eq!(run(LYRICS, Some("m3u")), found(M3u, 0));
        assert_eq!(run(TEXT, Some("lrc")), found(Lrc, 0));
    }

    /// Verifies: SEC-MED-011
    #[test]
    fn text_is_tabs_line_breaks_and_printable_octets() {
        let text: Vec<u8> = (0..=255_u8)
            .filter(|&octet| run(&[octet], Some("lrc")) == found(Lrc, 0))
            .collect();
        let others: Vec<u8> = (0..=255_u8)
            .filter(|&octet| run(&[octet], Some("lrc")) == unknown(0))
            .collect();
        let mut expected = vec![b'\t', b'\n', b'\r'];
        expected.extend(0x20..=0x7E);
        expected.extend(0x80..=0xFF);
        assert_eq!(text, expected);
        assert_eq!(text.len() + others.len(), 256);
    }

    /// Octets that are all text by the rule above, but start with a binary
    /// signature, are that format's content and not text.
    ///
    /// Verifies: SEC-MED-011
    #[test]
    fn content_with_a_binary_signature_is_never_text() {
        let flac_text = b"fLaC is a lossless audio format.\n";
        assert_eq!(run(flac_text, Some("lrc")), unknown(0));
        assert_eq!(run(flac_text, Some("flac")), found(Flac, 0));
        assert_eq!(
            run(b"\xFF\xFB\x90\xE4 reads as text", Some("m3u")),
            unknown(0)
        );
        assert_eq!(run(b"GIF89a: an image format\n", Some("m3u")), unknown(0));
    }

    #[test]
    fn checks_the_first_512_octets_for_text() {
        let mut file: Vec<u8> = (0..512).map(|_| b'a').collect();
        file.push(0);
        assert_eq!(
            trace(&file, Some("lrc")),
            (vec![read(0, 512)], found(Lrc, 0))
        );
        file[511] = 0;
        assert_eq!(trace(&file, Some("lrc")), (vec![read(0, 512)], unknown(0)));
    }

    /// Every second and third octet after a first of `0xFF`, decoded field by
    /// field as ISO/IEC 11172-3 and 13818-3 lay out a frame header: only
    /// Layer III of MPEG-1, MPEG-2 or MPEG-2.5, at a bitrate other than free
    /// or forbidden and a known sample rate, is MP3. `FF D8 FF` is JPEG.
    ///
    /// Verifies: SEC-MED-011
    #[test]
    fn exactly_the_layer_iii_frame_headers_are_mp3() {
        let mut headers = 0;
        for second in 0..=255_u8 {
            for third in 0..=255_u8 {
                let sync = second >> 5 == 0b111;
                let version = (second >> 3) & 0b11;
                let layer = (second >> 1) & 0b11;
                let bitrate = third >> 4;
                let rate = (third >> 2) & 0b11;
                let is_mp3 = sync
                    && version != 0b01
                    && layer == 0b01
                    && bitrate != 0
                    && bitrate != 0b1111
                    && rate != 0b11;
                let expected = if is_mp3 {
                    Some(Mpeg)
                } else if (second, third) == (0xD8, 0xFF) {
                    Some(Jpeg)
                } else {
                    None
                };
                assert_eq!(
                    signature(&[0xFF, second, third, 0x44]),
                    expected,
                    "{second:02X} {third:02X}"
                );
                headers += usize::from(is_mp3);
            }
        }
        // Three versions, either protection bit, 14 bitrates, three rates,
        // and any padding and private bit.
        assert_eq!(headers, 3 * 2 * 14 * 3 * 4);
    }

    #[test]
    fn detects_mp3_of_every_version_and_no_other_layer() {
        for version in [0b00, 0b10, 0b11] {
            assert_eq!(
                run(&mpeg(version, 0b01, 0b0101, 0b01), None),
                found(Mpeg, 0)
            );
        }
        // The reserved version; Layers I and II and the reserved layer; the
        // free and forbidden bitrates; the reserved sample rate.
        let refused = [
            mpeg(0b01, 0b01, 0b0101, 0b01),
            mpeg(0b11, 0b11, 0b0101, 0b01),
            mpeg(0b11, 0b10, 0b0101, 0b01),
            mpeg(0b11, 0b00, 0b0101, 0b01),
            mpeg(0b11, 0b01, 0b0000, 0b01),
            mpeg(0b11, 0b01, 0b1111, 0b01),
            mpeg(0b11, 0b01, 0b0101, 0b11),
        ];
        for file in refused {
            assert_eq!(run(&file, Some("mp3")), unknown(0), "{file:02X?}");
        }
        // The lowest and highest bitrates.
        assert_eq!(run(&mpeg(0b10, 0b01, 0b0001, 0b10), None), found(Mpeg, 0));
        assert_eq!(run(&mpeg(0b10, 0b01, 0b1110, 0b00), None), found(Mpeg, 0));
        // A UTF-16 text file's byte order mark reads as a Layer I header.
        assert_eq!(run(b"\xFF\xFEA\0", Some("mp3")), unknown(0));
    }

    /// Verifies: SEC-MED-011
    #[test]
    fn mp4_audio_is_an_ftyp_box_of_16_to_4096_octets_with_an_audio_brand() {
        for brand in [*b"M4A ", *b"M4B ", *b"isom", *b"iso2", *b"mp41", *b"mp42"] {
            assert_eq!(
                run(&ftyp(brand, 4), Some("m4a")),
                found(Mp4, 0),
                "{brand:?}"
            );
        }
        // HEIF and AVIF images, Canon raw, QuickTime, 3GPP, video, protected
        // audio and DASH segments, and a brand in the wrong case.
        let others = [
            *b"heic", *b"mif1", *b"avif", *b"crx ", *b"qt  ", *b"3gp4", *b"M4V ", *b"M4P ",
            *b"dash", *b"isoM",
        ];
        for brand in others {
            assert_eq!(run(&ftyp(brand, 4), Some("m4a")), unknown(0), "{brand:?}");
        }
        // 15 octets leave no room for the minor version; 4,097 is more than a
        // file-type box holds.
        assert_eq!(run(&ftyp(*b"M4A ", 3), Some("m4a")), unknown(0));
        assert_eq!(run(&ftyp(*b"M4A ", 4), Some("m4a")), found(Mp4, 0));
        assert_eq!(run(&ftyp(*b"M4A ", 4_084), Some("m4a")), found(Mp4, 0));
        assert_eq!(run(&ftyp(*b"M4A ", 4_085), Some("m4a")), unknown(0));
        // A size of zero, of one (a 64-bit size follows) and of 2^32 - 1.
        for size in [[0, 0, 0, 0], [0, 0, 0, 1], [0xFF; 4]] {
            let file = joined(&[&size, b"ftypM4A \0\0\0\0"]);
            assert_eq!(run(&file, Some("m4a")), unknown(0), "{size:02X?}");
        }
    }

    #[test]
    fn riff_and_iff_files_are_told_apart_by_their_form_type() {
        assert_eq!(run(&wav(), None), found(Wav, 0));
        assert_eq!(run(&rf64(), None), found(Wav, 0));
        assert_eq!(run(&webp(), None), found(Webp, 0));
        assert_eq!(run(&aiff(*b"AIFF"), None), found(Aiff, 0));
        assert_eq!(run(&aiff(*b"AIFC"), None), found(Aiff, 0));
        // AVI, big-endian RIFX, WebP in RF64, 8SVX audio and a form type in
        // the wrong case.
        let others: [&[u8]; 5] = [
            b"RIFF\x04\0\0\0AVI ",
            b"RIFX\x04\0\0\0WAVE",
            b"RF64\xFF\xFF\xFF\xFFWEBP",
            b"FORM\x04\0\0\08SVX",
            b"RIFF\x04\0\0\0wave",
        ];
        for file in others {
            assert_eq!(run(file, None), unknown(0), "{file:02X?}");
        }
    }

    #[test]
    fn a_signature_must_be_whole() {
        let wholes: [(&[u8], Format); 10] = [
            (b"fLaC", Flac),
            (b"\xFF\xF3\x10", Mpeg),
            (b"\0\0\0\x10ftypisom", Mp4),
            (b"OggS\0", Ogg),
            (b"RIFF\0\0\0\0WAVE", Wav),
            (b"FORM\0\0\0\0AIFC", Aiff),
            (b"\xFF\xD8\xFF", Jpeg),
            (b"\x89PNG\r\n\x1A\n", Png),
            (b"RIFF\0\0\0\0WEBP", Webp),
            (b"GIF87a", Gif),
        ];
        for (file, format) in wholes {
            assert_eq!(run(file, None), found(format, 0), "{file:02X?}");
            for len in 0..file.len() {
                assert_eq!(
                    run(&file[..len], None),
                    unknown(0),
                    "{file:02X?} cut to {len}"
                );
            }
        }
        let near_misses: [&[u8]; 9] = [
            b"fLac",
            b"OggS\x01",
            b"\xFF\xD8\xFE",
            b"\x89PNG\r\n\x1A\x0B",
            b"\x88PNG\r\n\x1A\n",
            b"GIF88a",
            b"GIF89A",
            b"\0\0\0\x10ftipisom",
            b"\xFE\xFB\x90",
        ];
        for file in near_misses {
            assert_eq!(run(file, None), unknown(0), "{file:02X?}");
        }
    }

    /// The plan's "an `ID3v2` tag followed by FLAC (it happens)".
    ///
    /// Verifies: SEC-MED-011, SEC-HIS-017
    #[test]
    fn skips_an_id3v2_tag_in_front_of_flac() {
        let file = joined(&[&id3v2(4, 0, 20, false), &flac()]);
        assert_eq!(
            trace(&file, Some("flac")),
            (vec![read(0, 72), read(30, 42)], found(Flac, 30))
        );
        assert_eq!(run(&file, Some("mp3")), found(Flac, 30));
        assert_eq!(run(&file, None), found(Flac, 30));
    }

    #[test]
    fn skips_an_id3v2_tag_in_front_of_mp3() {
        let file = joined(&[&id3v2(3, 0, 100, false), &mp3()]);
        assert_eq!(
            trace(&file, Some("mp3")),
            (vec![read(0, 146), read(110, 36)], found(Mpeg, 110))
        );
    }

    #[test]
    fn skips_tags_of_versions_2_3_and_4_and_of_no_other() {
        for major in [2, 3, 4] {
            let file = joined(&[&id3v2(major, 0, 5, false), &flac()]);
            assert_eq!(run(&file, None), found(Flac, 15), "version 2.{major}");
        }
        for major in [0, 1, 5, 0xFF] {
            let file = joined(&[&id3v2(major, 0, 5, false), &flac()]);
            assert_eq!(run(&file, None), unknown(0), "version 2.{major}");
        }
        // A revision is never 0xFF (`ID3v2`.4 structure, section 3.1); any
        // other is skipped.
        let mut file = joined(&[&id3v2(4, 0, 5, false), &flac()]);
        file[4] = 0xFF;
        assert_eq!(run(&file, None), unknown(0));
        file[4] = 0xFE;
        assert_eq!(run(&file, None), found(Flac, 15));
    }

    #[test]
    fn only_an_id3v2_4_tag_has_a_footer() {
        let cases: [(Vec<u8>, u64); 6] = [
            (id3v2(4, 0x10, 5, true), 25),
            (id3v2(4, 0xF0, 5, true), 25),
            (id3v2(4, 0x00, 5, false), 15),
            (id3v2(4, 0xE0, 5, false), 15),
            // In versions 2.3 and 2.2 the bit means nothing.
            (id3v2(3, 0x10, 5, false), 15),
            (id3v2(2, 0xF0, 5, false), 15),
        ];
        for (tag, start) in cases {
            let file = joined(&[&tag, &flac()]);
            assert_eq!(run(&file, None), found(Flac, start), "{tag:02X?}");
        }
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn refuses_a_tag_size_that_is_not_syncsafe() {
        let mut file = joined(&[&id3v2(4, 0, 5, false), &flac()]);
        file[7] = 0x80;
        assert_eq!(
            trace(&file, None),
            (
                vec![read(0, 57)],
                fault(NotSyncsafe {
                    offset: 6,
                    octets: [0, 0x80, 0, 5],
                })
            )
        );
    }

    /// The plan's "an `ID3v2` tag whose declared size runs past the file": the
    /// declared length is checked against the file, and nothing past it is
    /// read.
    ///
    /// Verifies: SEC-TM-032, SEC-MED-010
    #[test]
    fn an_id3v2_tag_that_runs_past_the_file_is_truncated() {
        let tag = id3v2(3, 0, 100, false);
        assert_eq!(
            trace(&tag[..15], Some("mp3")),
            (
                vec![read(0, 15)],
                fault(Truncated {
                    offset: 0,
                    needed: 110,
                    available: 15,
                })
            )
        );
        assert_eq!(
            run(&tag[..109], None),
            fault(Truncated {
                offset: 0,
                needed: 110,
                available: 109,
            })
        );
        // A version 2.4 tag that announces a footer and has none.
        assert_eq!(
            run(&id3v2(4, 0x10, 5, false), None),
            fault(Truncated {
                offset: 0,
                needed: 25,
                available: 15,
            })
        );
    }

    #[test]
    fn a_tag_that_fills_the_file_leaves_nothing_to_detect() {
        let tag = id3v2(3, 0, 100, false);
        assert_eq!(trace(&tag, None), (vec![read(0, 110)], unknown(110)));
    }

    /// Verifies: SEC-TM-032
    #[test]
    fn every_truncation_of_a_tagged_flac_file_has_its_exact_result() {
        let file = joined(&[&id3v2(4, 0, 6, false), &flac()]);
        assert_eq!(file.len(), 58);
        for len in 0..=58 {
            let available = u64::try_from(len).expect("small");
            let expected = match len {
                0..=4 => unknown(0),
                5..=9 => fault(Truncated {
                    offset: 0,
                    needed: 10,
                    available,
                }),
                10..=15 => fault(Truncated {
                    offset: 0,
                    needed: 16,
                    available,
                }),
                16..=19 => unknown(16),
                _ => found(Flac, 16),
            };
            assert_eq!(run(&file[..len], None), expected, "{len} octets");
        }
    }

    /// An `ID3v2` tag prefixes MPEG audio, and taggers put one before FLAC too;
    /// nothing else is read after one.
    #[test]
    fn only_flac_or_mp3_may_follow_a_tag() {
        let tag = id3v2(3, 0, 5, false);
        let others = [
            ogg(),
            wav(),
            aiff(*b"AIFF"),
            m4a(),
            jpeg(),
            png(),
            webp(),
            gif(*b"89a"),
            LYRICS.to_vec(),
            TEXT.to_vec(),
        ];
        for content in others {
            assert_eq!(run(&joined(&[&tag, &content]), None), unknown(15));
        }
        assert_eq!(run(&joined(&[&tag, &mp3()]), None), found(Mpeg, 15));
        assert_eq!(run(&joined(&[&tag, &flac()]), None), found(Flac, 15));
    }

    #[test]
    fn tags_are_skipped_only_when_the_name_admits_audio() {
        let file = joined(&[&id3v2(3, 0, 5, false), &flac()]);
        for hint in [None, Some("flac"), Some("mp3"), Some("opus")] {
            assert_eq!(run(&file, hint), found(Flac, 15), "{hint:?}");
        }
        for hint in [
            Some("jpg"),
            Some("png"),
            Some("lrc"),
            Some("m3u"),
            Some("txt"),
        ] {
            assert_eq!(run(&file, hint), unknown(0), "{hint:?}");
        }
    }

    #[test]
    fn skips_several_tags_in_a_row() {
        let file = joined(&[
            &id3v2(3, 0, 4, false),
            &id3v2(4, 0x10, 2, true),
            &id3v2(2, 0, 0, false),
            &mp3(),
        ]);
        assert_eq!(
            trace(&file, None),
            (
                vec![read(0, 82), read(14, 68), read(36, 46), read(46, 36)],
                found(Mpeg, 46)
            )
        );
    }

    /// Detection reads the start of the file and what follows each of up to
    /// seven leading tags, one step of its budget each, and no more. Eight
    /// tags is the element-count bound: a ninth is not visited.
    ///
    /// Verifies: SEC-MED-006, SEC-MED-007, SEC-TM-032
    #[test]
    fn reads_at_most_eight_windows() {
        let seven = joined(&[&empty_tags(7), &flac()]);
        let (reads, outcome) = trace(&seven, None);
        assert_eq!(outcome, found(Flac, 70));
        assert_eq!(
            reads.iter().map(|read| read.offset).collect::<Vec<_>>(),
            [0, 10, 20, 30, 40, 50, 60, 70]
        );
        let eight = joined(&[&empty_tags(8), &flac()]);
        let (reads, outcome) = trace(&eight, None);
        assert_eq!(outcome, fault(BudgetExceeded { offset: 80 }));
        assert_eq!(
            reads.iter().map(|read| read.offset).collect::<Vec<_>>(),
            [0, 10, 20, 30, 40, 50, 60, 70]
        );
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn an_empty_tag_still_moves_past_its_header() {
        let file = joined(&[&id3v2(4, 0, 0, false), &flac()]);
        assert_eq!(
            trace(&file, None),
            (vec![read(0, 52), read(10, 42)], found(Flac, 10))
        );
    }

    /// Verifies: SEC-MED-004, SEC-TM-032
    #[test]
    fn a_tag_whose_end_would_pass_u64_max_is_truncated() {
        let tag = id3v2(4, 0, 100, false);
        let offset = u64::MAX - 20;
        let window = Window {
            offset,
            bytes: &tag[..20],
            file_len: u64::MAX,
        };
        assert_eq!(
            detect(None).resume(window),
            Step::Done(Err(DetectError::Fault(Truncated {
                offset,
                needed: 110,
                available: 20,
            })))
        );
    }

    /// Verifies: SEC-MED-004, SEC-MED-010
    #[test]
    fn a_file_of_u64_max_octets_is_read_from_its_start() {
        assert_eq!(
            detect(None).resume(Window::start(u64::MAX)),
            Step::Need(read(0, 512))
        );
    }

    /// The plan's empty file.
    ///
    /// Verifies: SEC-MED-011
    #[test]
    fn an_empty_file_is_unknown_without_a_read() {
        for hint in [None, Some("flac"), Some("lrc")] {
            assert_eq!(trace(&[], hint), (vec![], unknown(0)), "{hint:?}");
        }
    }

    /// The plan's file of one byte.
    ///
    /// Verifies: SEC-MED-011
    #[test]
    fn a_file_of_one_byte() {
        assert_eq!(trace(b"f", Some("flac")), (vec![read(0, 1)], unknown(0)));
        assert_eq!(trace(&[0xFF], None), (vec![read(0, 1)], unknown(0)));
        assert_eq!(trace(b"a", Some("lrc")), (vec![read(0, 1)], found(Lrc, 0)));
    }

    /// Verifies: SEC-MED-010
    #[test]
    fn reads_at_most_512_octets_at_a_time_and_never_past_the_end() {
        let mut long = Bytes::new();
        long.bytes(&flac()).zeros(2_000);
        let long = long.into_vec();
        assert_eq!(trace(&long, None), (vec![read(0, 512)], found(Flac, 0)));
        let file = joined(&[&id3v2(3, 0, 600, false), &long]);
        assert_eq!(
            trace(&file, None),
            (vec![read(0, 512), read(610, 512)], found(Flac, 610))
        );
        assert_eq!(trace(&flac(), None), (vec![read(0, 42)], found(Flac, 0)));
    }

    #[test]
    fn runs_under_the_drive_of_a_file_in_memory() {
        let file = joined(&[&id3v2(4, 0, 6, false), &flac()]);
        assert_eq!(
            drive(detect(Some("flac")), &file, &Limits::DEFAULT),
            Ok(found(Flac, 16))
        );
    }

    // The property tests' model and generators.

    /// The kinds a name can claim, written independently of the code.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Kind {
        Audio,
        Image,
        Lyrics,
        Playlist,
    }

    const fn kind_of(format: Format) -> Kind {
        match format {
            Flac | Mpeg | Mp4 | Ogg | Wav | Aiff => Kind::Audio,
            Jpeg | Png | Webp | Gif => Kind::Image,
            Lrc => Kind::Lyrics,
            M3u => Kind::Playlist,
        }
    }

    /// Every name on the allowlist, with the kind it claims.
    const NAMES: [(&str, Kind); 20] = [
        ("aif", Kind::Audio),
        ("aifc", Kind::Audio),
        ("aiff", Kind::Audio),
        ("flac", Kind::Audio),
        ("gif", Kind::Image),
        ("jpeg", Kind::Image),
        ("jpg", Kind::Image),
        ("lrc", Kind::Lyrics),
        ("m3u", Kind::Playlist),
        ("m3u8", Kind::Playlist),
        ("m4a", Kind::Audio),
        ("m4b", Kind::Audio),
        ("mp3", Kind::Audio),
        ("mp4", Kind::Audio),
        ("oga", Kind::Audio),
        ("ogg", Kind::Audio),
        ("opus", Kind::Audio),
        ("png", Kind::Image),
        ("wav", Kind::Audio),
        ("webp", Kind::Image),
    ];

    /// The kind `name` claims, or `None` for a name outside the allowlist.
    fn claimed(name: &str) -> Option<Kind> {
        let lower = name.to_ascii_lowercase();
        NAMES
            .iter()
            .find(|(listed, _)| *listed == lower)
            .map(|&(_, kind)| kind)
    }

    /// Every hint the properties try: none, every name in lower and upper
    /// case, and names outside the allowlist.
    fn hints() -> Vec<Option<String>> {
        let mut hints = vec![None];
        for (name, _) in NAMES {
            hints.push(Some(name.to_owned()));
            hints.push(Some(name.to_ascii_uppercase()));
        }
        for name in ["", "txt", ".flac", "mp2", "aac", "flac "] {
            hints.push(Some(name.to_owned()));
        }
        hints
    }

    /// The major brands of MP4 audio, as the model reads them.
    const MP4_BRANDS: [&[u8]; 6] = [b"M4A ", b"M4B ", b"isom", b"iso2", b"mp41", b"mp42"];

    /// The format whose signature `head` starts with, read field by field
    /// from each specification.
    fn model_signature(head: &[u8]) -> Option<Format> {
        let at =
            |start: usize, octets: &[u8]| head.get(start..start + octets.len()) == Some(octets);
        if at(0, b"fLaC") {
            return Some(Flac);
        }
        if let [0xFF, second, third, ..] = *head {
            let version = (second >> 3) & 0b11;
            let layer = (second >> 1) & 0b11;
            let bitrate = third >> 4;
            let rate = (third >> 2) & 0b11;
            if second >> 5 == 0b111
                && version != 0b01
                && layer == 0b01
                && (1..=14).contains(&bitrate)
                && rate != 0b11
            {
                return Some(Mpeg);
            }
        }
        if at(0, &[0xFF, 0xD8, 0xFF]) {
            return Some(Jpeg);
        }
        if let [s0, s1, s2, s3, b'f', b't', b'y', b'p', ..] = *head {
            let size = u32::from_be_bytes([s0, s1, s2, s3]);
            if (16..=4_096).contains(&size) && MP4_BRANDS.iter().any(|brand| at(8, brand)) {
                return Some(Mp4);
            }
        }
        if at(0, b"OggS\0") {
            return Some(Ogg);
        }
        if (at(0, b"RIFF") || at(0, b"RF64")) && at(8, b"WAVE") {
            return Some(Wav);
        }
        if at(0, b"RIFF") && at(8, b"WEBP") {
            return Some(Webp);
        }
        if at(0, b"FORM") && (at(8, b"AIFF") || at(8, b"AIFC")) {
            return Some(Aiff);
        }
        if at(0, b"\x89PNG\r\n\x1A\n") {
            return Some(Png);
        }
        if at(0, b"GIF87a") || at(0, b"GIF89a") {
            return Some(Gif);
        }
        None
    }

    /// An independent model of a whole detection, written from the module
    /// documentation rather than the code: every read it makes and what it
    /// returns.
    fn model(file: &[u8], hint: Option<&str>) -> (Vec<ReadRequest>, Outcome) {
        let file_len = file.len();
        let name = hint.map(claimed);
        let mut reads = Vec::new();
        let mut at = 0;
        let mut tagged = false;
        loop {
            let offset = u64::try_from(at).expect("small");
            if at == file_len {
                return (reads, Err(DetectError::Unknown { offset }));
            }
            if reads.len() == 8 {
                return (reads, Err(DetectError::Fault(BudgetExceeded { offset })));
            }
            let head = &file[at..file_len.min(at + 512)];
            reads.push(read(offset, u32::try_from(head.len()).expect("small")));
            let audio = tagged || matches!(name, None | Some(Some(Kind::Audio)));
            if let [b'I', b'D', b'3', major @ 2..=4, revision, ..] = *head
                && audio
                && revision != 0xFF
            {
                let Some(&[.., flags, s0, s1, s2, s3]) = head.get(..10) else {
                    let available = u64::try_from(head.len()).expect("small");
                    return (
                        reads,
                        Err(DetectError::Fault(Truncated {
                            offset,
                            needed: 10,
                            available,
                        })),
                    );
                };
                if [s0, s1, s2, s3].iter().any(|&octet| octet >= 0x80) {
                    return (
                        reads,
                        Err(DetectError::Fault(NotSyncsafe {
                            offset: offset + 6,
                            octets: [s0, s1, s2, s3],
                        })),
                    );
                }
                let size = [s0, s1, s2, s3]
                    .iter()
                    .fold(0, |size, &octet| size * 128 + usize::from(octet));
                let footer = if major == 4 && flags & 0x10 == 0x10 {
                    10
                } else {
                    0
                };
                let len = 10 + size + footer;
                if at + len > file_len {
                    return (
                        reads,
                        Err(DetectError::Fault(Truncated {
                            offset,
                            needed: u64::try_from(len).expect("small"),
                            available: u64::try_from(file_len - at).expect("small"),
                        })),
                    );
                }
                at += len;
                tagged = true;
                continue;
            }
            let signed = model_signature(head);
            let format = if tagged {
                signed.filter(|&format| format == Flac || format == Mpeg)
            } else {
                match name {
                    None => signed,
                    Some(None) => None,
                    Some(Some(kind)) => {
                        let text = head.iter().all(|&octet| {
                            octet == b'\t'
                                || octet == b'\n'
                                || octet == b'\r'
                                || (0x20..0x7F).contains(&octet)
                                || octet >= 0x80
                        });
                        match kind {
                            Kind::Audio | Kind::Image => {
                                signed.filter(|&format| kind_of(format) == kind)
                            }
                            Kind::Lyrics => (signed.is_none() && text).then_some(Lrc),
                            Kind::Playlist => (signed.is_none() && text).then_some(M3u),
                        }
                    }
                }
            };
            return (
                reads,
                format
                    .map(|format| Detected {
                        format,
                        start: offset,
                    })
                    .ok_or(DetectError::Unknown { offset }),
            );
        }
    }

    /// The model gives the literal results of the examples above, one for
    /// each way a detection ends, so the property below rests on a model
    /// known to agree with them.
    #[test]
    fn the_model_gives_the_results_of_the_examples() {
        let mut not_syncsafe = joined(&[&id3v2(4, 0, 5, false), &flac()]);
        not_syncsafe[7] = 0x80;
        let cases: [(Vec<u8>, Option<&str>, Outcome); 12] = [
            (flac(), Some("mp3"), found(Flac, 0)),
            (png(), Some("jpg"), found(Png, 0)),
            (png(), Some("flac"), unknown(0)),
            (flac(), Some("txt"), unknown(0)),
            (LYRICS.to_vec(), Some("lrc"), found(Lrc, 0)),
            (PLAYLIST.to_vec(), Some("m3u"), found(M3u, 0)),
            (
                joined(&[&id3v2(4, 0x10, 20, true), &flac()]),
                None,
                found(Flac, 40),
            ),
            (
                id3v2(3, 0, 100, false)[..15].to_vec(),
                Some("mp3"),
                fault(Truncated {
                    offset: 0,
                    needed: 110,
                    available: 15,
                }),
            ),
            (
                b"ID3\x04\0\0".to_vec(),
                None,
                fault(Truncated {
                    offset: 0,
                    needed: 10,
                    available: 6,
                }),
            ),
            (
                not_syncsafe,
                None,
                fault(NotSyncsafe {
                    offset: 6,
                    octets: [0, 0x80, 0, 5],
                }),
            ),
            (
                joined(&[&empty_tags(8), &flac()]),
                None,
                fault(BudgetExceeded { offset: 80 }),
            ),
            (id3v2(3, 0, 100, false), None, unknown(110)),
        ];
        for (file, hint, expected) in cases {
            let (reads, outcome) = model(&file, hint);
            assert_eq!(outcome, expected, "{hint:?}");
            assert_eq!(trace(&file, hint), (reads, expected), "{hint:?}");
        }
    }

    /// Every kind of file the examples above use: each format, near misses,
    /// text, and each way a leading tag can end.
    fn examples() -> Vec<Vec<u8>> {
        let mut revision_ff = joined(&[&id3v2(4, 0, 5, false), &flac()]);
        revision_ff[4] = 0xFF;
        let mut not_syncsafe = joined(&[&id3v2(4, 0, 5, false), &flac()]);
        not_syncsafe[7] = 0x80;
        vec![
            Vec::new(),
            b"a".to_vec(),
            flac(),
            mp3(),
            mpeg(0b11, 0b10, 0b0101, 0b01),
            m4a(),
            ftyp(*b"heic", 4),
            ftyp(*b"M4A ", 3),
            ogg(),
            wav(),
            rf64(),
            webp(),
            b"RIFF\x04\0\0\0AVI ".to_vec(),
            aiff(*b"AIFF"),
            aiff(*b"AIFC"),
            jpeg(),
            png(),
            gif(*b"87a"),
            gif(*b"89a"),
            LYRICS.to_vec(),
            PLAYLIST.to_vec(),
            TEXT.to_vec(),
            HTML.to_vec(),
            PDF.to_vec(),
            joined(&[&id3v2(4, 0x10, 20, true), &flac()]),
            joined(&[&id3v2(3, 0, 100, false), &mp3()]),
            joined(&[&id3v2(2, 0, 5, false), &ogg()]),
            joined(&[&id3v2(5, 0, 5, false), &flac()]),
            revision_ff,
            not_syncsafe,
            id3v2(3, 0, 100, false)[..15].to_vec(),
            b"ID3\x04\0\0".to_vec(),
            id3v2(3, 0, 100, false),
            joined(&[&empty_tags(8), &flac()]),
        ]
    }

    /// The model and the code agree on every example under every name, so
    /// no branch of the model is left to the property's random choices.
    #[test]
    fn the_model_agrees_with_the_code_on_every_example_under_every_name() {
        for file in examples() {
            for hint in hints() {
                let hint = hint.as_deref();
                assert_eq!(
                    trace(&file, hint),
                    model(&file, hint),
                    "{file:02X?} named {hint:?}"
                );
            }
        }
    }

    /// Octets that may follow a signature: anything.
    fn tail() -> impl Strategy<Value = Vec<u8>> {
        vec(any::<u8>(), 0..48)
    }

    /// One `ID3v2` tag as a tagger writes it: version 2.2, 2.3 or 2.4, any
    /// revision and flags, a body of up to 40 octets, and for version 2.4 a
    /// footer exactly when the flags announce one.
    fn any_tag() -> impl Strategy<Value = Vec<u8>> {
        (
            2_u8..=4,
            0_u8..=0xFE,
            any::<u8>(),
            any::<bool>(),
            vec(any::<u8>(), 0..40),
        )
            .prop_map(|(major, revision, flags, footer, body)| {
                let footer = footer && major == 4;
                let flags = if footer {
                    flags | 0x10
                } else if major == 4 {
                    flags & !0x10
                } else {
                    flags
                };
                let size = u32::try_from(body.len()).expect("small");
                let mut tag = Bytes::new();
                tag.bytes(b"ID3")
                    .u8(major)
                    .u8(revision)
                    .u8(flags)
                    .syncsafe32(size)
                    .bytes(&body);
                if footer {
                    tag.bytes(b"3DI")
                        .u8(major)
                        .u8(revision)
                        .u8(flags)
                        .syncsafe32(size);
                }
                tag.into_vec()
            })
    }

    /// One octet of text.
    fn text_octet() -> impl Strategy<Value = u8> {
        prop_oneof![
            Just(b'\t'),
            Just(b'\n'),
            Just(b'\r'),
            0x20_u8..=0x7E,
            0x80_u8..=0xFF,
        ]
    }

    /// A file of one format as a builder writes it, with its fields and the
    /// octets after its signature chosen at random, and the offset where the
    /// format's content starts: FLAC and MP3 get up to three `ID3v2` tags in
    /// front.
    fn any_file() -> impl Strategy<Value = (Format, Vec<u8>, u64)> {
        let flac = (any::<bool>(), vec(any::<u8>(), 34), tail()).prop_map(|(last, info, tail)| {
            let mut file = Bytes::new();
            file.bytes(b"fLaC")
                .u8(if last { 0x80 } else { 0 })
                .u24_be(34)
                .bytes(&info)
                .bytes(&tail);
            file.into_vec()
        });
        let mp3 = (
            select(vec![0b00_u64, 0b10, 0b11]),
            1_u64..=14,
            0_u64..=2,
            any::<[bool; 6]>(),
            0_u64..4,
            tail(),
        )
            .prop_map(|(version, bitrate, rate, bits, mode, tail)| {
                let mut header = Bits::new();
                header
                    .put(11, 0x7FF)
                    .put(2, version)
                    .put(2, 0b01)
                    .put(1, u64::from(bits[0]))
                    .put(4, bitrate)
                    .put(2, rate)
                    .put(1, u64::from(bits[1]))
                    .put(1, u64::from(bits[2]))
                    .put(2, mode)
                    .put(2, mode)
                    .put(1, u64::from(bits[3]))
                    .put(1, u64::from(bits[4]))
                    .put(2, u64::from(bits[5]));
                joined(&[&header.into_vec(), &tail])
            });
        let tagged = |format: Format, content: BoxedStrategy<Vec<u8>>| {
            (vec(any_tag(), 0..=3), content).prop_map(move |(tags, content)| {
                let tags = tags.concat();
                let start = u64::try_from(tags.len()).expect("small");
                (format, joined(&[&tags, &content]), start)
            })
        };
        let plain = |format: Format, content: BoxedStrategy<Vec<u8>>| {
            content.prop_map(move |content| (format, content, 0))
        };
        let mp4 = (
            select(MP4_BRANDS.to_vec()),
            any::<u32>(),
            vec(any::<[u8; 4]>(), 0..8),
            tail(),
        )
            .prop_map(|(brand, minor, compatible, tail)| {
                let mut body = Bytes::new();
                body.bytes(brand).u32_be(minor).bytes(&compatible.concat());
                let mut file = Bytes::new();
                file.mp4_box(*b"ftyp", body.as_slice()).bytes(&tail);
                file.into_vec()
            });
        let ogg = (any::<u8>(), vec(any::<u8>(), 22), tail())
            .prop_map(|(kind, header, tail)| joined(&[b"OggS\0", &[kind], &header, &tail]));
        let wav = (any::<bool>(), any::<u32>(), tail()).prop_map(|(rf64, size, tail)| {
            joined(&[
                if rf64 { b"RF64" } else { b"RIFF" },
                &size.to_le_bytes(),
                b"WAVE",
                &tail,
            ])
        });
        let aiff = (any::<bool>(), any::<u32>(), tail()).prop_map(|(aifc, size, tail)| {
            joined(&[
                b"FORM",
                &size.to_be_bytes(),
                if aifc { b"AIFC" } else { b"AIFF" },
                &tail,
            ])
        });
        let jpeg = (any::<u8>(), tail())
            .prop_map(|(marker, tail)| joined(&[&[0xFF, 0xD8, 0xFF, marker], &tail]));
        let png = tail().prop_map(|tail| joined(&[b"\x89PNG\r\n\x1A\n", &tail]));
        let webp = (any::<u32>(), tail())
            .prop_map(|(size, tail)| joined(&[b"RIFF", &size.to_le_bytes(), b"WEBP", &tail]));
        let gif = (any::<bool>(), tail())
            .prop_map(|(old, tail)| joined(&[if old { b"GIF87a" } else { b"GIF89a" }, &tail]));
        let lyrics = vec(text_octet(), 0..600).prop_map(|text| joined(&[b"[", &text]));
        let playlist = vec(text_octet(), 0..600).prop_map(|text| joined(&[b"#", &text]));
        prop_oneof![
            tagged(Flac, flac.boxed()),
            tagged(Mpeg, mp3.boxed()),
            plain(Mp4, mp4.boxed()),
            plain(Ogg, ogg.boxed()),
            plain(Wav, wav.boxed()),
            plain(Aiff, aiff.boxed()),
            plain(Jpeg, jpeg.boxed()),
            plain(Png, png.boxed()),
            plain(Webp, webp.boxed()),
            plain(Gif, gif.boxed()),
            plain(Lrc, lyrics.boxed()),
            plain(M3u, playlist.boxed()),
        ]
    }

    /// What detection must return for a file of `format` whose content
    /// starts at `start`, named `hint`. A file with a binary signature is
    /// its format when the name claims its kind or there is no name. Text
    /// has no signature, so it is whichever text format its name claims.
    /// Anything else is nothing, with the file's first octet as the offset.
    fn expected(format: Format, start: u64, hint: Option<&str>) -> Outcome {
        let claim = hint.map(claimed);
        let answer = match (kind_of(format), claim) {
            (Kind::Lyrics | Kind::Playlist, Some(Some(Kind::Lyrics))) => Some(Lrc),
            (Kind::Lyrics | Kind::Playlist, Some(Some(Kind::Playlist))) => Some(M3u),
            (Kind::Lyrics | Kind::Playlist, _) => None,
            (_, None) => Some(format),
            (kind, Some(claim)) => (claim == Some(kind)).then_some(format),
        };
        answer.map_or(unknown(0), |format| found(format, start))
    }

    /// Files a builder wrote, then damaged: octets overwritten, the file cut
    /// short, or many tags in a row; or plain random octets.
    fn damaged_file() -> impl Strategy<Value = Vec<u8>> {
        let edited = (
            any_file(),
            vec((any::<Index>(), any::<u8>()), 0..3),
            any::<Index>(),
            any::<bool>(),
        )
            .prop_map(|((_, mut file, _), edits, cut, shorten)| {
                for (at, octet) in edits {
                    let len = file.len();
                    file[at.index(len)] = octet;
                }
                if shorten {
                    file.truncate(cut.index(file.len() + 1));
                }
                file
            });
        let chained = (vec(any_tag(), 5..=9), any_file())
            .prop_map(|(tags, (_, file, _))| joined(&[&tags.concat(), &file]));
        prop_oneof![vec(any::<u8>(), 0..64), edited, chained]
    }

    proptest! {
        /// The plan's property, made exact: for any file a builder writes,
        /// detection returns exactly the format it wrote under every name of
        /// that format's kind and with no name, and nothing under any other
        /// name.
        ///
        /// Verifies: SEC-MED-011, SEC-HIS-017
        #[test]
        fn detects_exactly_the_format_a_builder_wrote_under_every_name((format, file, start) in any_file()) {
            for hint in hints() {
                let hint = hint.as_deref();
                prop_assert_eq!(run(&file, hint), expected(format, start, hint), "{:?}", hint);
            }
        }

        /// Any input, named anything, gets exactly the model's reads and
        /// result on a 256 KiB stack: at most eight reads, each inside the
        /// file, each after the last. Detection does not recurse, so a
        /// 256 KiB stack is enough for every input (SEC-MED-005).
        ///
        /// Verifies: SEC-MED-001, SEC-MED-005, SEC-MED-007, SEC-MED-008, SEC-MED-010, SEC-TM-032
        #[test]
        fn reads_and_returns_exactly_as_the_model_does_for_any_input(file in damaged_file(), hint in select(hints())) {
            let (reads, outcome) = model(&file, hint.as_deref());
            let file_len = u64::try_from(file.len()).expect("small");
            let inside = |read: &ReadRequest| {
                (1..=512).contains(&read.len) && read.offset + u64::from(read.len) <= file_len
            };
            prop_assert!(reads.len() <= 8, "{:?}", reads);
            prop_assert!(reads.windows(2).all(|pair| pair[0].offset < pair[1].offset), "{:?}", reads);
            prop_assert!(reads.iter().all(inside), "{:?}", reads);
            let probe = file.clone();
            let traced = on_small_stack(move || trace(&probe, hint.as_deref()));
            prop_assert_eq!(traced, (reads, outcome));
        }
    }
}
