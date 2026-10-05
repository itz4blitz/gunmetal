//! What a probe has found so far, and how it becomes the result.
//!
//! A container parser's result becomes a [`Draft`]: the facts it gives,
//! and the reads still to make for the tag blocks. Each read adds its
//! block or its problem. [`Draft::finish`] then derives what depends on
//! all of them: the tag fields the file may keep, the artwork references,
//! the lyrics sources, the average bitrate and the identity window.

use std::num::{NonZeroU32, NonZeroU64};
use std::ops::Range;

use crate::catalog::{
    ArtworkRef, ArtworkSource, AudioFormat, Bitrate, ByteRange, Codec, Container, FileFacts,
    IdentityInputs, LyricsOrigin, LyricsSource, LyricsTiming, PictureType, TechInfo, Trim,
};
use crate::formats::ape::{ApeItem, ApeTag, ApeValue};
use crate::formats::detect::Format;
use crate::formats::id3v2::{FrameBody, SyncedLyrics};
use crate::formats::mp4::{FourCc, ItemKey, ItemValue};
use crate::formats::riff;
use crate::lyrics::{self, Lyrics, SyltClock};
use crate::parse::{Budget, LimitKind, Limits, ParseFault, ReadRequest, Window};
use crate::untrusted::Untrusted;
use crate::values::Duration;

use super::facts::{
    PARSER_VERSIONS, PartProblem, ProbeError, Probed, SeekIndex, TagBlock, version,
};

/// Octets of an `ID3v2` tag's header and footer, which a tag takes on top
/// of the size the in-memory limit caps.
const ID3V2_ENDS: u64 = 20;

/// The APE keys that hold a picture, with the picture type each means.
const APE_COVERS: [(&str, u32); 2] = [("Cover Art (Front)", 3), ("Cover Art (Back)", 4)];

/// The Vorbis comment names that hold lyrics.
const VORBIS_LYRICS: [&str; 2] = ["LYRICS", "UNSYNCEDLYRICS"];

/// The MP4 item that holds lyrics, `©lyr`.
const MP4_LYRICS: FourCc = FourCc([0xA9, b'l', b'y', b'r']);

/// The `SYLT` content type for lyrics.
const SYLT_LYRICS: u8 = 1;

/// The picture type of a front cover, which an MP4 `covr` item is.
const FRONT_COVER: u32 = 3;

/// A read still to make for a tag block, and what to do with its octets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Job {
    /// The `ID3v2` tags in front of the audio, back to back.
    Id3v2,
    /// The `ID3v2` tag of a WAV or AIFF file's chunk. A chunk holds one.
    ChunkTag,
    /// The Vorbis comment block of a FLAC file.
    Comment,
    /// The sub-chunks of a WAV file's `INFO` list.
    Info,
    /// The end of an MP3 file: its APE and `ID3v1` tags.
    Tail,
    /// An APE tag that starts before the end of the file that was read
    /// for [`Job::Tail`]: the tag again, from its start to the end of the
    /// file. The `ID3v1` tag was read from the first read.
    Ape,
    /// The end of an Ogg file: the last page of the stream.
    OggTail {
        /// The stream's serial number.
        serial: u32,
        /// Samples before the first one played.
        skip: u64,
    },
}

/// Octets of a file being read into memory: those from `start` up to
/// `end`, of which `octets` are held so far.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Gather {
    /// Where the octets start in the file.
    pub(super) start: u64,
    /// Where they end.
    pub(super) end: u64,
    /// The octets read so far, from `start`.
    pub(super) octets: Vec<u8>,
    /// The read the host was asked for and has not answered yet.
    pub(super) asked: Option<ReadRequest>,
}

impl Gather {
    /// A read of the octets from `start` up to `end`.
    pub(super) const fn new(start: u64, end: u64) -> Self {
        Self {
            start,
            end,
            octets: Vec::new(),
            asked: None,
        }
    }

    /// The octets read, as a window of a file of `file_len` octets.
    pub(super) fn window(&self, file_len: u64) -> Window<'_> {
        Window {
            offset: self.start,
            bytes: &self.octets,
            file_len,
        }
    }
}

/// What a probe has found so far.
#[derive(Debug)]
pub(super) struct Draft {
    /// The format detected.
    pub(super) format: Format,
    /// The codec, or `None` for one that is not read.
    pub(super) codec: Option<Codec>,
    /// The container.
    pub(super) container: Container,
    /// Where the audio starts, or the part of the file that describes it:
    /// what an error about the codec points at.
    pub(super) at: u64,
    /// The audio format. The bitrate is the one the headers state, if
    /// they state one.
    pub(super) audio: AudioFormat,
    /// Encoder delay and padding.
    pub(super) trim: Option<Trim>,
    /// The MD5 of the decoded audio that a FLAC file states.
    pub(super) md5: Option<[u8; 16]>,
    /// Where the audio starts and ends.
    pub(super) window: (u64, u64),
    /// The seek index.
    pub(super) seek: SeekIndex,
    /// The pictures the container itself holds: the picture type and the
    /// encoded size of each.
    pub(super) pictures: Vec<(u32, u64)>,
    /// The tag blocks read so far.
    pub(super) tags: Vec<TagBlock>,
    /// The covers of the APE tag among `tags`: the picture type and the
    /// encoded size of each, in the order of the tag's items. A cover's
    /// size is measured when the tag is read ([`ape_covers`]), since only
    /// then are its octets at hand.
    pub(super) covers: Vec<(u32, u64)>,
    /// The parts skipped so far.
    pub(super) problems: Vec<PartProblem>,
    /// The reads still to make. The last is made first.
    pub(super) jobs: Vec<(Job, Gather)>,
}

/// Lyrics found in a tag block.
enum Words<'a> {
    /// A text, which may be LRC.
    Text(LyricsOrigin, &'a str),
    /// The entries of a `SYLT` frame.
    Synced(&'a SyncedLyrics),
}

impl TagBlock {
    /// Cuts the block to its first `most` tag fields, and returns how
    /// many it held.
    ///
    /// A tag field is a frame of an `ID3v2` tag, an item of an APE tag, a
    /// comment of a Vorbis comment block that is not a picture, or an item
    /// of an MP4 item list. An `ID3v1` tag has the same few fields
    /// whatever it holds, and the sub-chunks of an `INFO` list are not
    /// read here, so neither counts.
    fn keep(&mut self, most: u64) -> u64 {
        match self {
            Self::Id3v2 { tag, .. } => cut(&mut tag.frames, most),
            Self::Ape(tag) => cut(&mut tag.items, most),
            Self::Vorbis(comments) => cut(&mut comments.fields, most),
            Self::Mp4(items) => cut(items, most),
            Self::Id3v1(_) | Self::RiffInfo { .. } => 0,
        }
    }

    /// The picture type and encoded size of every picture in the block.
    /// Those of an APE tag are `covers`, as many of them as the tag still
    /// has cover items.
    fn pictures(&self, covers: &[(u32, u64)]) -> Vec<(u32, u64)> {
        match self {
            Self::Id3v2 { tag, .. } => tag
                .frames
                .iter()
                .filter_map(|frame| match &frame.body {
                    FrameBody::Picture(picture) => Some((
                        u32::from(picture.picture_type),
                        picture.data.end.saturating_sub(picture.data.start),
                    )),
                    _ => None,
                })
                .collect(),
            Self::Ape(tag) => tag
                .items
                .iter()
                .filter_map(cover)
                .zip(covers)
                .map(|(_, &found)| found)
                .collect(),
            Self::Vorbis(comments) => comments
                .pictures
                .iter()
                .map(|picture| (picture.kind, u64::from(picture.data_len)))
                .collect(),
            Self::Mp4(items) => items
                .iter()
                .flat_map(|item| &item.values)
                .filter_map(|value| match value {
                    ItemValue::Picture(picture) => Some((FRONT_COVER, picture.len)),
                    _ => None,
                })
                .collect(),
            Self::Id3v1(_) | Self::RiffInfo { .. } => Vec::new(),
        }
    }

    /// Every lyrics text or frame in the block.
    fn lyrics(&self) -> Vec<Words<'_>> {
        match self {
            Self::Id3v2 { tag, .. } => tag
                .frames
                .iter()
                .filter_map(|frame| match &frame.body {
                    FrameBody::Lyrics(lyrics) => {
                        Some(Words::Text(LyricsOrigin::Id3Unsynced, &lyrics.text.value))
                    }
                    FrameBody::SyncedLyrics(synced) if synced.content_type == SYLT_LYRICS => {
                        Some(Words::Synced(synced))
                    }
                    _ => None,
                })
                .collect(),
            Self::Ape(tag) => tag
                .items
                .iter()
                .filter(|item| item.key.eq_ignore_ascii_case("Lyrics"))
                .filter_map(|item| match &item.value {
                    ApeValue::Text(values) => Some(values),
                    _ => None,
                })
                .flatten()
                .map(|text| Words::Text(LyricsOrigin::ApeItem, &text.value))
                .collect(),
            Self::Vorbis(comments) => comments
                .fields
                .iter()
                .filter(|field| {
                    VORBIS_LYRICS
                        .iter()
                        .any(|name| name.eq_ignore_ascii_case(&field.key))
                })
                .map(|field| Words::Text(LyricsOrigin::VorbisComment, &field.value.value))
                .collect(),
            Self::Mp4(items) => items
                .iter()
                .filter(|item| item.key == ItemKey::Atom(MP4_LYRICS))
                .flat_map(|item| &item.values)
                .filter_map(|value| match value {
                    ItemValue::Text(text) => Some(Words::Text(LyricsOrigin::Mp4Item, &text.value)),
                    _ => None,
                })
                .collect(),
            Self::Id3v1(_) | Self::RiffInfo { .. } => Vec::new(),
        }
    }
}

/// Cuts `fields` to its first `most`, and returns how many it held.
fn cut<T>(fields: &mut Vec<T>, most: u64) -> u64 {
    let held = u64::try_from(fields.len()).unwrap_or(u64::MAX);
    fields.truncate(usize::try_from(most).unwrap_or(usize::MAX));
    held
}

/// Keeps the first tag fields of `tags`, as many as a file may hold.
///
/// [`LimitKind::TagFields`] is the file's limit, so the fields of every
/// block count together, in the order of the blocks. The block in which
/// the limit is reached is cut there, and every block after it is left
/// with no field.
///
/// # Errors
///
/// [`ParseFault::LimitExceeded`] at offset 0, the file's, with how many
/// fields were found, when that is more than the limit. The blocks are
/// cut all the same.
fn cut_fields(tags: &mut [TagBlock], limits: &Limits) -> Result<(), ParseFault> {
    let max = limits.get(LimitKind::TagFields);
    let mut found = 0_u64;
    for block in tags {
        found = found.saturating_add(block.keep(max.saturating_sub(found)));
    }
    limits.check(LimitKind::TagFields, found, 0)
}

/// The picture type of `item` and where its value lies in the file, when
/// the item is a cover: a binary item whose key names one.
fn cover(item: &ApeItem) -> Option<(u32, &Range<u64>)> {
    let &(_, kind) = APE_COVERS
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(&item.key))?;
    match &item.value {
        ApeValue::Binary(range) => Some((kind, range)),
        _ => None,
    }
}

/// The covers of `tag`, an APE tag that `window` holds whole: the picture
/// type and the encoded size of each, in the order of the tag's items.
pub(super) fn ape_covers(tag: &ApeTag, window: Window<'_>) -> Vec<(u32, u64)> {
    tag.items
        .iter()
        .filter_map(cover)
        .map(|(kind, range)| (kind, picture_len(window, range)))
        .collect()
}

/// How many octets the picture of the cover at `range` takes, read from
/// `window`.
///
/// A cover's value is a file name, a zero octet, then the picture, so the
/// picture is what follows the first zero octet. A value with no zero
/// octet has no file name, and all of it is the picture.
fn picture_len(window: Window<'_>, range: &Range<u64>) -> u64 {
    let mut cursor = window.cursor();
    let value = cursor
        .skip(range.start.saturating_sub(window.offset))
        .and_then(|()| cursor.take(range.end.saturating_sub(range.start)))
        .unwrap_or_default();
    let name = value
        .iter()
        .position(|&octet| octet == 0)
        .map_or(0, |end| end.saturating_add(1));
    u64::try_from(value.len().saturating_sub(name)).unwrap_or(u64::MAX)
}

impl Words<'_> {
    /// Where the lyrics were found.
    const fn origin(&self) -> LyricsOrigin {
        match self {
            Self::Text(origin, _) => *origin,
            Self::Synced(_) => LyricsOrigin::Id3Synced,
        }
    }

    /// How the lyrics are timed, found by reading them under the lyrics
    /// limits.
    fn timing(&self, limits: &Limits, budget: &mut Budget) -> Result<LyricsTiming, ParseFault> {
        let parsed = match self {
            Self::Text(_, text) => {
                lyrics::parse_lrc(Untrusted::new(text.as_bytes()), limits, budget)
            }
            Self::Synced(synced) => {
                let entries: Vec<(u32, &str)> = synced
                    .lines
                    .iter()
                    .map(|line| (line.time, line.text.value.as_str()))
                    .collect();
                // Only the unit of the times depends on the clock, never
                // whether lines or words have them.
                lyrics::from_sylt(
                    SyltClock::Millis,
                    Untrusted::new(entries.as_slice()),
                    limits,
                    budget,
                )
            }
        }?;
        Ok(match parsed.lyrics {
            Lyrics::Plain(_) => LyricsTiming::Plain,
            Lyrics::Lines(_) => LyricsTiming::Line,
            Lyrics::Words(_) => LyricsTiming::Word,
        })
    }
}

impl Draft {
    /// The result, once every read is made. `budget` pays for reading the
    /// lyrics, and `bytes_read` is how many octets of the file were read.
    ///
    /// The fields of every tag block count together against
    /// [`LimitKind::TagFields`]: those past it are cut from their blocks,
    /// before the pictures and the lyrics are looked for, and the breach
    /// is recorded. The pictures of the container and of every tag block
    /// count together against [`LimitKind::Pictures`]: those past it are
    /// left out of the artwork, and the breach is recorded (SEC-MED-006).
    ///
    /// # Errors
    ///
    /// [`ProbeError::Unsupported`] when the codec is not one that is read,
    /// and [`ProbeError::Catalog`] when the facts do not fit together.
    pub(super) fn finish(
        self,
        limits: &Limits,
        budget: &mut Budget,
        bytes_read: u64,
    ) -> Result<Probed, ProbeError> {
        let Self {
            format,
            codec,
            container,
            at,
            mut audio,
            trim,
            md5,
            window: (start, end),
            seek,
            pictures: mut found,
            mut tags,
            covers,
            mut problems,
            jobs: _,
        } = self;
        // A tag that claims to start before the audio leaves no audio.
        let end = end.max(start);
        audio.bitrate = audio
            .bitrate
            .or(average(end.saturating_sub(start), audio.duration));
        problems.extend(cut_fields(&mut tags, limits).err().map(PartProblem::Fault));
        let mut lyrics = Vec::new();
        for block in &tags {
            found.extend(block.pictures(&covers));
            for words in block.lyrics() {
                match words.timing(limits, budget) {
                    // A source that cannot be: untimed text from a frame
                    // that only holds timed text is not a source.
                    Ok(timing) => lyrics.extend(LyricsSource::new(words.origin(), timing)),
                    Err(fault) => problems.push(PartProblem::Lyrics(fault)),
                }
            }
        }
        // The picture limit is the file's, so it counts the pictures of
        // every block together. The first ones are kept, in the order they
        // are numbered, and a breach is recorded against the file, with
        // how many were found. The limit is far below the 65,536 numbers
        // there are, so every picture kept has one.
        let count = u64::try_from(found.len()).unwrap_or(u64::MAX);
        if let Err(fault) = limits.check(LimitKind::Pictures, count, 0) {
            problems.push(PartProblem::Fault(fault));
        }
        let most = usize::try_from(limits.get(LimitKind::Pictures)).unwrap_or(usize::MAX);
        found.truncate(most);
        let artwork = (0..=u16::MAX)
            .zip(found)
            .map(|(index, (kind, byte_len))| ArtworkRef {
                source: ArtworkSource::Embedded { index },
                picture_type: picture_type(kind),
                byte_len,
            })
            .collect();
        let (tech, audio_window) = codec
            .ok_or(ProbeError::Unsupported { format, offset: at })
            .and_then(|codec| TechInfo::new(codec, container, audio).map_err(ProbeError::Catalog))
            .and_then(|tech| {
                ByteRange::new(start, end)
                    .map(|window| (tech, window))
                    .map_err(ProbeError::Catalog)
            })?;
        Ok(Probed {
            format,
            facts: FileFacts {
                tech,
                trim,
                artwork,
                lyrics,
                identity: IdentityInputs {
                    audio_md5: md5,
                    audio_window,
                },
                parser_version: version(PARSER_VERSIONS, format),
                bytes_read,
            },
            tags,
            seek,
            problems,
        })
    }
}

/// The picture type with the code `kind`, or [`PictureType::Other`] for a
/// code that names none.
fn picture_type(kind: u32) -> PictureType {
    u8::try_from(kind)
        .ok()
        .and_then(PictureType::from_code)
        .unwrap_or(PictureType::Other)
}

/// The average bitrate of `octets` octets of audio that play for
/// `duration`, when the duration is known and not zero and the bitrate is
/// one a catalogue value holds.
fn average(octets: u64, duration: Option<Duration>) -> Option<Bitrate> {
    let millis = NonZeroU64::new(duration?.millis())?;
    let bits_a_second = octets.saturating_mul(8_000) / millis;
    u32::try_from(bits_a_second)
        .ok()
        .and_then(|bits| Bitrate::new(bits).ok())
}

/// How long `ticks` at `per_second` ticks a second play, when both are
/// known. A duration outside its range is dropped and recorded
/// (SEC-MED-014).
pub(super) fn span(
    ticks: Option<u64>,
    per_second: Option<NonZeroU32>,
    problems: &mut Vec<PartProblem>,
) -> Option<Duration> {
    let (ticks, per_second) = ticks.zip(per_second)?;
    match Duration::from_ticks(ticks, per_second) {
        Ok(duration) => Some(duration),
        Err(error) => {
            problems.push(PartProblem::Value(error));
            None
        }
    }
}

/// The read of the `ID3v2` tags in `range`. A tag over the in-memory limit
/// is read only as far as its parser reads it: up to the limit, and its
/// header and footer.
pub(super) fn id3v2(range: riff::ByteRange, limits: &Limits) -> (Job, Gather) {
    let len = range.len.min(
        limits
            .get(LimitKind::Id3v2TagBytes)
            .saturating_add(ID3V2_ENDS),
    );
    (
        Job::Id3v2,
        Gather::new(range.offset, range.offset.saturating_add(len)),
    )
}

/// The read of the one `ID3v2` tag of the chunk whose body is `range`: the
/// octets [`id3v2`] reads of a tag, for a chunk's job.
pub(super) fn chunk_tag(range: riff::ByteRange, limits: &Limits) -> (Job, Gather) {
    let (_, gather) = id3v2(range, limits);
    (Job::ChunkTag, gather)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::{Field, ValueError};

    fn duration(millis: u64) -> Option<Duration> {
        Duration::from_millis(millis).ok()
    }

    fn rate(per_second: u32) -> Option<NonZeroU32> {
        NonZeroU32::new(per_second)
    }

    #[test]
    fn names_a_picture_type_by_its_code() {
        assert_eq!(picture_type(0), PictureType::Other);
        assert_eq!(picture_type(3), PictureType::FrontCover);
        assert_eq!(picture_type(20), PictureType::PublisherLogo);
    }

    #[test]
    fn calls_a_picture_with_an_unknown_code_other() {
        assert_eq!(picture_type(21), PictureType::Other);
        // 259 is 3 when cut to one octet.
        assert_eq!(picture_type(259), PictureType::Other);
        assert_eq!(picture_type(u32::MAX), PictureType::Other);
    }

    #[test]
    fn averages_the_bitrate_over_the_duration() {
        // 40,000 octets in 2 seconds are 160,000 bits a second.
        assert_eq!(average(40_000, duration(2_000)), Bitrate::new(160_000).ok());
        // 1 octet in 3 milliseconds is 2,666 bits a second, rounded down.
        assert_eq!(average(1, duration(3)), Bitrate::new(2_666).ok());
    }

    #[test]
    fn gives_no_bitrate_without_a_duration_or_for_no_time() {
        assert_eq!(average(40_000, None), None);
        assert_eq!(average(40_000, duration(0)), None);
    }

    #[test]
    fn gives_no_bitrate_of_zero_or_past_what_a_value_holds() {
        // 1 octet in 9 seconds is less than one bit a second.
        assert_eq!(average(1, duration(9_000)), None);
        // u32::MAX bits a second is the most; one octet more is past it.
        assert_eq!(
            average(4_294_967_295, duration(8_000)),
            Bitrate::new(u32::MAX).ok()
        );
        assert_eq!(average(4_294_967_296, duration(8_000)), None);
        assert_eq!(average(u64::MAX, duration(1)), None);
    }

    #[test]
    fn measures_ticks_at_their_rate() {
        let mut problems = Vec::new();
        assert_eq!(
            span(Some(88_200), rate(44_100), &mut problems),
            duration(2_000)
        );
        assert_eq!(problems, []);
    }

    #[test]
    fn gives_no_duration_without_ticks_or_a_rate() {
        let mut problems = Vec::new();
        assert_eq!(span(None, rate(44_100), &mut problems), None);
        assert_eq!(span(Some(88_200), None, &mut problems), None);
        assert_eq!(problems, []);
    }

    /// Verifies: SEC-MED-017
    #[test]
    fn drops_and_records_a_duration_past_thirty_days() {
        let mut problems = Vec::new();
        // 2,592,000,001 ticks at 1,000 a second are one millisecond past
        // 30 days.
        assert_eq!(span(Some(2_592_000_001), rate(1_000), &mut problems), None);
        assert_eq!(
            problems,
            [PartProblem::Value(ValueError::OutOfRange {
                field: Field::Duration,
                value: 2_592_000_001,
            })]
        );
    }

    #[test]
    fn reads_an_id3v2_tag_whole_up_to_the_limit_and_its_ends() {
        let limits = Limits::DEFAULT
            .with_override(LimitKind::Id3v2TagBytes, 100)
            .unwrap();
        let range = |len| riff::ByteRange { offset: 50, len };
        assert_eq!(id3v2(range(30), &limits), (Job::Id3v2, Gather::new(50, 80)));
        assert_eq!(
            id3v2(range(120), &limits),
            (Job::Id3v2, Gather::new(50, 170))
        );
        assert_eq!(
            id3v2(range(121), &limits),
            (Job::Id3v2, Gather::new(50, 170))
        );
        assert_eq!(
            id3v2(range(u64::MAX), &limits),
            (Job::Id3v2, Gather::new(50, 170))
        );
    }
}
