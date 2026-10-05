//! What a probe returns: the facts of one file, its raw tag blocks, its
//! seek index and the problems of the parts that were skipped.

use crate::catalog::{CatalogError, FileFacts};
use crate::formats::aiff::AiffError;
use crate::formats::ape::{ApeError, ApeTag};
use crate::formats::detect::{DetectError, Format};
use crate::formats::flac::metadata::{BlockProblem, FlacError, SeekPoint};
use crate::formats::id3v1::{Id3v1Error, Id3v1Tag};
use crate::formats::id3v2::{Id3v2Error, Id3v2Tag};
use crate::formats::mp4::{IlstItem, Mp4Error, Mp4Problem, SampleTableRanges};
use crate::formats::mpa::{self, MpaError};
use crate::formats::ogg::PacketError;
use crate::formats::opus::OpusError;
use crate::formats::riff::RiffError;
use crate::formats::vorbis::VorbisError;
use crate::formats::vorbis_comment::Comments;
use crate::parse::{DriveError, ParseFault, ReadRequest};
use crate::problem::{Arg, Describe, Problem, ProblemCode};
use crate::values::ValueError;

/// The version of the parser that reads each audio format. A version goes
/// up when a newer parser would read a file differently, so the scan can
/// tell which files to read again.
pub const PARSER_VERSIONS: &[(Format, u16)] = &[
    (Format::Flac, 1),
    (Format::Mpeg, 1),
    (Format::Mp4, 1),
    (Format::Ogg, 1),
    (Format::Wav, 1),
    (Format::Aiff, 1),
];

/// What a probe found in one file.
#[derive(Debug, Clone, PartialEq)]
pub struct Probed {
    /// The format the file's content has.
    pub format: Format,
    /// The catalogue's facts: technical facts, trim, artwork, lyrics,
    /// identity inputs, the parser version and the octets read.
    ///
    /// Artwork is numbered in this order: the pictures the container
    /// itself holds (FLAC `PICTURE` blocks), then those of each tag block
    /// in the order of [`Probed::tags`]. The per-file picture limit counts
    /// them all together: the pictures past it are not listed, and
    /// [`Probed::problems`] holds how many were found.
    ///
    /// The size of an APE cover is its picture's alone: the item's value
    /// without the file name and the zero octet that come before the
    /// picture in it.
    pub facts: FileFacts,
    /// The raw tag blocks, in their order of precedence: the first block
    /// that holds a field wins.
    ///
    /// - MP3: every leading `ID3v2` tag in file order, then APE, then
    ///   `ID3v1`.
    /// - FLAC: the Vorbis comment, then every leading `ID3v2` tag.
    /// - Ogg: the Vorbis comment.
    /// - MP4: the item list, which is empty when the file has none.
    /// - WAV: the `ID3v2` tag of its `id3 ` chunk, then the `INFO` list.
    /// - AIFF: the `ID3v2` tag of its `ID3 ` chunk.
    ///
    /// A file has at most seven leading `ID3v2` tags, since detection
    /// skips no more. A chunk holds one tag: a tag that follows it in the
    /// chunk is not read, and is recorded in [`Probed::problems`].
    ///
    /// The per-file tag-field limit counts the fields of every block
    /// together, in this order: the frames of an `ID3v2` tag, the items of
    /// an APE tag, the comments of a Vorbis comment block that are not
    /// pictures, and the items of an MP4 item list. The fields past the
    /// limit are cut from their blocks, so a block may be left with none,
    /// and [`Probed::problems`] holds how many were found. An `ID3v1` tag
    /// and an `INFO` list do not count: the first has the same few fields
    /// whatever it holds, and the second is passed on as octets, to be
    /// counted where it is read.
    pub tags: Vec<TagBlock>,
    /// Where to start reading to play from a given time.
    pub seek: SeekIndex,
    /// Every optional part that was skipped, in the order found
    /// (SEC-MED-017). What a tag block skipped inside itself is in the
    /// block.
    pub problems: Vec<PartProblem>,
}

/// One tag block of a file, as its parser read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagBlock {
    /// An `ID3v2` tag.
    Id3v2 {
        /// Where the tag starts in the file. Offsets inside the tag count
        /// from here.
        offset: u64,
        /// The tag.
        tag: Id3v2Tag,
    },
    /// An APE tag.
    Ape(ApeTag),
    /// An `ID3v1` tag.
    Id3v1(Id3v1Tag),
    /// A Vorbis comment block. In a FLAC file its offsets are file
    /// offsets; in an Ogg file they count octets of the comment block.
    Vorbis(Comments),
    /// The items of an MP4 file's item lists.
    Mp4(Vec<IlstItem>),
    /// The sub-chunks of a WAV file's `INFO` list, as they are.
    RiffInfo {
        /// Where the sub-chunks start in the file.
        offset: u64,
        /// Their octets.
        octets: Vec<u8>,
    },
}

/// Where to start reading to play from a given time (MUS-071), as far as
/// the file's headers say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeekIndex {
    /// The headers hold none: an Ogg file is searched by its pages, and
    /// PCM is found by arithmetic.
    None,
    /// The seek points of a FLAC file's `SEEKTABLE` block. Their offsets
    /// count from the first frame, where the audio window starts.
    Flac(Vec<SeekPoint>),
    /// The index of an MP3 file: from its Xing table of contents, or from
    /// its frame headers.
    Mpeg(mpa::SeekIndex),
    /// Where an MP4 file's sample tables are, from which the index is
    /// built.
    Mp4(SampleTableRanges),
}

/// An optional part of a file that was skipped, or a value that was
/// dropped, while the rest of the file was kept (SEC-MED-017).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartProblem {
    /// A FLAC seek table, picture or duplicate block.
    Flac(BlockProblem),
    /// Part of an MP4 file's metadata, or an extra audio track.
    Mp4(Mp4Problem),
    /// The walk over a WAV or AIFF file's chunks stopped early.
    Stopped(ParseFault),
    /// An encoder header in an MP3 file's first frame.
    Encoder(ParseFault),
    /// An `ID3v2` tag.
    Id3v2 {
        /// Where the tag starts in the file.
        offset: u64,
        /// Why it could not be read.
        error: Id3v2Error,
    },
    /// An APE tag.
    Ape(ApeError),
    /// An `ID3v1` tag.
    Id3v1(Id3v1Error),
    /// A Vorbis comment block.
    Comment(ParseFault),
    /// An Ogg stream has no comment header packet, because of this.
    Ogg(PacketError),
    /// The framing of an Opus comment header.
    Opus(OpusError),
    /// The framing of a Vorbis comment header.
    Vorbis(VorbisError),
    /// A lyrics text.
    Lyrics(ParseFault),
    /// A value outside its range, such as a duration (SEC-MED-014).
    Value(ValueError),
    /// A limit of the probe's own was reached. The fault says which.
    ///
    /// [`ParseFault::LimitExceeded`] names the limit:
    ///
    /// - The in-memory tag limit, where an APE tag or an `INFO` list
    ///   starts: the block is larger than a tag block may be in memory,
    ///   and is not read. The fault holds how many octets the read of it
    ///   would have been: the length of the list, or the octets from the
    ///   start of the APE tag to the end of the file.
    /// - The in-memory tag limit, at offset 0 of an Ogg file: the stream's
    ///   comment header was not found in as many octets of the file's
    ///   start as a tag block may take in memory, so the file has no
    ///   comments. The fault holds the length of the file.
    /// - The tag-field limit or the picture limit, at offset 0, the
    ///   file's: the tag blocks together hold more fields, or the file
    ///   more pictures, than a file may. The fault holds how many were
    ///   found, and those past the limit are left out.
    ///
    /// [`ParseFault::BudgetExceeded`] has two meanings, told apart by the
    /// format of the file:
    ///
    /// - In an Ogg file the step budget was spent in the search for the
    ///   stream's last page, so the file has no duration (SEC-MED-007).
    /// - In any other file more `ID3v2` tags lie back to back than are
    ///   read from one place: seven in front of the audio, and one in a
    ///   chunk of a WAV or AIFF file. The fault's offset is where the
    ///   first tag left unread starts, and every tag after it is unread
    ///   too. No step was spent on it. What ran out is the count of tags
    ///   for that place, which no row of the limits table holds, so the
    ///   fault that names a limit cannot say it, and this one carries
    ///   neither how many tags there are nor how many may be read. It is
    ///   the fault detection gives a file with more leading tags than it
    ///   skips. A step budget that runs out while a tag is read is
    ///   [`PartProblem::Id3v2`] instead, with the tag's offset beside the
    ///   fault.
    Fault(ParseFault),
    /// A read that the limits refuse (SEC-MED-010): the read of a tag
    /// block, of the end of an Ogg file, or of more of the start of an Ogg
    /// file when what was read of it holds no comment header.
    Read(DriveError),
    /// The end of an Ogg file holds no page of its stream on which a packet
    /// ends, so how long the file plays is not known.
    NoLastPage {
        /// Where the octets that were searched start: the last 64 KiB of
        /// the file, or the whole file when it is shorter.
        offset: u64,
        /// The serial number of the stream.
        serial: u32,
    },
}

impl PartProblem {
    /// The name of the part, for the problem catalogue.
    const fn part(&self) -> &'static str {
        match self {
            Self::Flac(_) => "flac_block",
            Self::Mp4(_) => "mp4_metadata",
            Self::Stopped(_) => "chunks",
            Self::Encoder(_) => "encoder_header",
            Self::Id3v2 { .. } => "id3v2_tag",
            Self::Ape(_) => "ape_tag",
            Self::Id3v1(_) => "id3v1_tag",
            Self::Comment(_) => "vorbis_comment",
            Self::Ogg(_) => "ogg_comment_packet",
            Self::Opus(_) => "opus_comment_header",
            Self::Vorbis(_) => "vorbis_comment_header",
            Self::Lyrics(_) => "lyrics",
            Self::Value(_) => "value",
            Self::Fault(_) => "limit",
            Self::Read(_) => "read",
            Self::NoLastPage { .. } => "ogg_last_page",
        }
    }
}

impl Describe for PartProblem {
    /// A skipped part, by name.
    fn problem(&self) -> Problem {
        Problem {
            code: ProblemCode::FilePartSkipped,
            args: vec![("part", Arg::Name(self.part()))],
        }
    }
}

/// Why a file could not be probed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeError {
    /// The content is no format on the allowlist, so the file must be
    /// skipped (SEC-MED-011), or a leading `ID3v2` tag is damaged.
    Detect(DetectError),
    /// The content is a picture, lyrics or a playlist, not audio.
    NotAudio {
        /// The format it is.
        format: Format,
    },
    /// A FLAC file's marker, STREAMINFO or block chain.
    Flac(FlacError),
    /// An MP3 file with no stream this parser accepts.
    Mpeg(MpaError),
    /// An MP4 file's boxes, movie or audio track.
    Mp4(Mp4Error),
    /// A WAV file's form, format or samples.
    Wav(RiffError),
    /// An AIFF file's form, format or samples.
    Aiff(AiffError),
    /// An Ogg file with no packet to read a stream header from, with the
    /// first thing that went wrong, if anything did.
    Ogg(Option<PacketError>),
    /// An Opus identification header.
    Opus(OpusError),
    /// A Vorbis identification header, or the header of a codec that is
    /// neither Opus nor Vorbis.
    Vorbis(VorbisError),
    /// The file holds a codec that is not read.
    Unsupported {
        /// The container.
        format: Format,
        /// Where the audio in that codec starts: the samples of a WAV or
        /// AIFF file, or the audio track of an MP4 file.
        offset: u64,
    },
    /// The step budget was spent (SEC-MED-007).
    Fault(ParseFault),
    /// A read that playback needs is one the limits refuse (SEC-MED-010).
    Read(DriveError),
    /// The host answered a read of the probe's own with a window that is
    /// not the octets asked for: it starts somewhere else, it holds more
    /// octets than were asked for, or it holds none, as when the file was
    /// cut short after its length was taken. Asking again would get no
    /// further, so the file fails at once, whatever the read was for
    /// (SEC-MED-008).
    Unanswered {
        /// The read the probe asked for.
        asked: ReadRequest,
        /// Where the window it was given starts.
        offset: u64,
        /// How many octets that window holds.
        len: u64,
    },
    /// The facts found do not make a catalogue value.
    Catalog(CatalogError),
    /// The probe was resumed after it gave its result. A probe reads one
    /// file once.
    Finished,
}

impl ProbeError {
    /// The name of the reason, for the problem catalogue.
    const fn reason(&self) -> &'static str {
        match self {
            Self::Detect(_) => "unknown_format",
            Self::NotAudio { .. } => "not_audio",
            Self::Flac(_) => "flac",
            Self::Mpeg(_) => "mpeg",
            Self::Mp4(_) => "mp4",
            Self::Wav(_) => "wav",
            Self::Aiff(_) => "aiff",
            Self::Ogg(_) => "ogg",
            Self::Opus(_) => "opus",
            Self::Vorbis(_) => "vorbis",
            Self::Unsupported { .. } => "unsupported_codec",
            Self::Fault(_) => "budget",
            Self::Read(_) => "read",
            Self::Unanswered { .. } => "unanswered_read",
            Self::Catalog(_) => "catalogue",
            Self::Finished => "finished",
        }
    }
}

impl Describe for ProbeError {
    /// An unreadable file, with the reason by name.
    fn problem(&self) -> Problem {
        Problem {
            code: ProblemCode::FileUnreadable,
            args: vec![("reason", Arg::Name(self.reason()))],
        }
    }
}

/// The version in `versions` of the parser for `format`, or 0 for a
/// format that has none.
pub(super) fn version(versions: &[(Format, u16)], format: Format) -> u16 {
    versions
        .iter()
        .find(|(known, _)| *known == format)
        .map_or(0, |&(_, version)| version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::id3v2::Id3v2Error;
    use crate::parse::LimitKind;
    use crate::values::Field;

    const FAULT: ParseFault = ParseFault::BudgetExceeded { offset: 7 };

    fn named(code: ProblemCode, key: &'static str, name: &'static str) -> Problem {
        Problem {
            code,
            args: vec![(key, Arg::Name(name))],
        }
    }

    #[test]
    fn lists_a_parser_version_for_every_audio_format() {
        assert_eq!(
            PARSER_VERSIONS,
            [
                (Format::Flac, 1),
                (Format::Mpeg, 1),
                (Format::Mp4, 1),
                (Format::Ogg, 1),
                (Format::Wav, 1),
                (Format::Aiff, 1),
            ]
        );
    }

    #[test]
    fn finds_the_version_of_the_format_asked_for() {
        let versions = [(Format::Flac, 3), (Format::Mpeg, 5), (Format::Ogg, 8)];
        assert_eq!(version(&versions, Format::Flac), 3);
        assert_eq!(version(&versions, Format::Mpeg), 5);
        assert_eq!(version(&versions, Format::Ogg), 8);
        assert_eq!(version(&versions, Format::Png), 0);
    }

    #[test]
    fn describes_every_skipped_part_by_name() {
        let drive = DriveError::Empty { offset: 1 };
        let value = ValueError::OutOfRange {
            field: Field::Duration,
            value: 9,
        };
        let cases = [
            (
                PartProblem::Flac(BlockProblem::LinkedPicture { block: 1 }),
                "flac_block",
            ),
            (
                PartProblem::Mp4(Mp4Problem::ExtraAudioTrack { offset: 1 }),
                "mp4_metadata",
            ),
            (PartProblem::Stopped(FAULT), "chunks"),
            (PartProblem::Encoder(FAULT), "encoder_header"),
            (
                PartProblem::Id3v2 {
                    offset: 0,
                    error: Id3v2Error::Fault(FAULT),
                },
                "id3v2_tag",
            ),
            (
                PartProblem::Ape(ApeError::NotAFooter { offset: 1 }),
                "ape_tag",
            ),
            (PartProblem::Id3v1(Id3v1Error::Fault(FAULT)), "id3v1_tag"),
            (PartProblem::Comment(FAULT), "vorbis_comment"),
            (
                PartProblem::Ogg(PacketError::Orphan { offset: 1 }),
                "ogg_comment_packet",
            ),
            (
                PartProblem::Opus(OpusError::Fault(FAULT)),
                "opus_comment_header",
            ),
            (
                PartProblem::Vorbis(VorbisError::Fault(FAULT)),
                "vorbis_comment_header",
            ),
            (PartProblem::Lyrics(FAULT), "lyrics"),
            (PartProblem::Value(value), "value"),
            (PartProblem::Fault(FAULT), "limit"),
            (PartProblem::Read(drive), "read"),
            (
                PartProblem::NoLastPage {
                    offset: 1,
                    serial: 7,
                },
                "ogg_last_page",
            ),
        ];
        for (problem, part) in cases {
            assert_eq!(
                problem.problem(),
                named(ProblemCode::FilePartSkipped, "part", part)
            );
        }
    }

    #[test]
    fn describes_every_failure_by_reason() {
        let cases = [
            (
                ProbeError::Detect(DetectError::Unknown { offset: 0 }),
                "unknown_format",
            ),
            (
                ProbeError::NotAudio {
                    format: Format::Png,
                },
                "not_audio",
            ),
            (
                ProbeError::Flac(FlacError::ForbiddenBlockType { offset: 4 }),
                "flac",
            ),
            (ProbeError::Mpeg(MpaError::NoFrames { offset: 0 }), "mpeg"),
            (ProbeError::Mp4(Mp4Error::NoMovie { offset: 0 }), "mp4"),
            (ProbeError::Wav(RiffError::Fault(FAULT)), "wav"),
            (ProbeError::Aiff(AiffError::Fault(FAULT)), "aiff"),
            (ProbeError::Ogg(None), "ogg"),
            (ProbeError::Opus(OpusError::Fault(FAULT)), "opus"),
            (ProbeError::Vorbis(VorbisError::Fault(FAULT)), "vorbis"),
            (
                ProbeError::Unsupported {
                    format: Format::Wav,
                    offset: 12,
                },
                "unsupported_codec",
            ),
            (ProbeError::Fault(FAULT), "budget"),
            (
                ProbeError::Read(DriveError::TooLong {
                    offset: 0,
                    len: 2,
                    max: LimitKind::ReadBytes.ceiling(),
                }),
                "read",
            ),
            (
                ProbeError::Unanswered {
                    asked: ReadRequest { offset: 4, len: 2 },
                    offset: 4,
                    len: 0,
                },
                "unanswered_read",
            ),
            (ProbeError::Catalog(CatalogError::ZeroBitrate), "catalogue"),
            (ProbeError::Finished, "finished"),
        ];
        for (error, reason) in cases {
            assert_eq!(
                error.problem(),
                named(ProblemCode::FileUnreadable, "reason", reason)
            );
        }
    }
}
