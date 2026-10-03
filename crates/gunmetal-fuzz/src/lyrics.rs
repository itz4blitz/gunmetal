//! The harness for the lyrics readers in `gunmetal_core::lyrics`
//! (SEC-MED-049, SEC-API-090).

use std::num::NonZeroU32;

use gunmetal_core::lyrics::{
    self, LRC_FIXED_STEPS, LRC_STEPS_PER_OCTET, Lyrics, Parsed, Source, SyltClock,
};
use gunmetal_core::parse::{Budget, LimitKind, Limits, ParseFault};
use gunmetal_core::untrusted::Untrusted;
use gunmetal_core::values::SampleRate;

/// What the lyrics readers reported for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// [`lyrics::parse_lrc`] of the input.
    pub lrc: Result<Parsed, ParseFault>,
    /// [`lyrics::from_sylt`] of the entries in the input, read as UTF-8
    /// with each invalid sequence replaced and split at each NUL. The first
    /// character of an entry is its time, a thousand units for each step of
    /// its code point, and the rest is its text. The units are MPEG frames
    /// of 1,152 samples at 44.1 kHz when the input's first octet is odd, and
    /// milliseconds otherwise.
    pub sylt: Result<Parsed, ParseFault>,
}

/// Feeds `data` to [`lyrics::parse_lrc`], its text to [`lyrics::from_uslt`]
/// and [`lyrics::from_vorbis`], and the entries [`Outcome::sylt`] describes
/// to [`lyrics::from_sylt`], each under the default limits with the budget
/// the module documents, then asks [`lyrics::position`] where a player is
/// at every line's and word's time and one millisecond before it.
///
/// # Panics
///
/// Panics when a reader breaks an invariant that holds for every input:
/// an LRC text refused for anything but being over 256 KiB; a budget of the
/// documented size that runs out; text from a tag that reads differently
/// from the same text in a file, but for its source; lyrics with more than
/// 10,000 lines or 256 KiB of text, a time past 24 hours or out of order, a
/// word time that goes back or comes before its line, a control character
/// other than tab and line feed, or more faults than there are lyrics
/// limits; or a position that goes back as time moves on.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let lrc = read_lrc(data);
    let text = String::from_utf8_lossy(data);
    let as_file = read_lrc(text.as_bytes());
    let uslt = lyrics::from_uslt(
        Untrusted::new(&text),
        &Limits::DEFAULT,
        &mut lrc_budget(text.as_bytes()),
    );
    let vorbis = lyrics::from_vorbis(
        Untrusted::new(&text),
        &Limits::DEFAULT,
        &mut lrc_budget(text.as_bytes()),
    );
    assert!(
        uslt == as_file.clone().map(|parsed| Parsed {
            source: Source::Uslt,
            ..parsed
        }) && vorbis
            == as_file.clone().map(|parsed| Parsed {
                source: Source::VorbisLyrics,
                ..parsed
            }),
        "the text as a file read as {as_file:?}, from USLT as {uslt:?}, from LYRICS as {vorbis:?}"
    );
    let sylt = read_sylt(data, &text);
    for parsed in [&lrc, &sylt]
        .into_iter()
        .filter_map(|read| read.as_ref().ok())
    {
        let shape = shape(parsed);
        let max_lines = usize::try_from(Limits::DEFAULT.get(LimitKind::LyricsLines)).unwrap();
        let max_octets = usize::try_from(Limits::DEFAULT.get(LimitKind::LyricsBytes)).unwrap();
        let max_at = u32::try_from(Limits::DEFAULT.get(LimitKind::LyricsTimestampMs)).unwrap();
        assert!(
            shape.lines <= max_lines
                && shape.texts.iter().map(|text| text.len()).sum::<usize>() <= max_octets
                && shape.texts.iter().all(|text| {
                    text.chars()
                        .all(|c| !c.is_control() || c == '\t' || c == '\n')
                })
                && shape.times.iter().map(|(at, _)| at).is_sorted()
                && shape.times.iter().all(|(at, words)| {
                    *at <= max_at
                        && words.is_sorted()
                        && words.iter().all(|word| (at..=&max_at).contains(&word))
                })
                && parsed.over_limit.len() <= LimitKind::ALL.len(),
            "{parsed:?} breaks a limit, an order or the text rules"
        );
        assert!(
            {
                let mut times: Vec<u32> = shape
                    .times
                    .iter()
                    .flat_map(|(at, words)| words.iter().chain([at]))
                    .flat_map(|&at| [at, at.saturating_sub(1)])
                    .chain([0, u32::MAX])
                    .collect();
                times.sort_unstable();
                times
                    .iter()
                    .map(|&millis| lyrics::position(&parsed.lyrics, millis))
                    .is_sorted()
            },
            "a position in {parsed:?} goes back as time moves on"
        );
    }
    Outcome { lrc, sylt }
}

/// The budget the lyrics module documents for LRC text of `text`'s length.
fn lrc_budget(text: &[u8]) -> Budget {
    let octets = u64::try_from(text.len()).unwrap_or(u64::MAX);
    Budget::for_input(octets, LRC_STEPS_PER_OCTET, LRC_FIXED_STEPS)
}

/// [`lyrics::parse_lrc`] of `text` under the default limits.
///
/// # Panics
///
/// Panics when the text is refused for anything but being longer than
/// 256 KiB.
fn read_lrc(text: &[u8]) -> Result<Parsed, ParseFault> {
    let read = lyrics::parse_lrc(
        Untrusted::new(text),
        &Limits::DEFAULT,
        &mut lrc_budget(text),
    );
    let octets = u64::try_from(text.len()).unwrap_or(u64::MAX);
    let max = Limits::DEFAULT.get(LimitKind::LyricsBytes);
    assert!(
        read.as_ref().err().copied()
            == (octets > max).then_some(ParseFault::LimitExceeded {
                limit: LimitKind::LyricsBytes,
                value: octets,
                max,
                offset: 0,
            }),
        "{octets} octets read as {read:?}"
    );
    read
}

/// The `SYLT` reading [`Outcome::sylt`] describes of `data`, whose text is
/// `text`.
///
/// # Panics
///
/// Panics when the reading fails, which only a budget that runs out could
/// make it do.
fn read_sylt(data: &[u8], text: &str) -> Result<Parsed, ParseFault> {
    let entries: Vec<(u32, &str)> = text
        .split('\0')
        .map(|entry| {
            let mut chars = entry.chars();
            let when = chars
                .next()
                .map_or(0, |first| u32::from(first).saturating_mul(1_000));
            (when, chars.as_str())
        })
        .collect();
    let clock = match SampleRate::new(44_100) {
        Ok(sample_rate) if data.first().is_some_and(|first| first % 2 == 1) => {
            SyltClock::MpegFrames {
                samples_per_frame: NonZeroU32::MIN.saturating_add(1_151),
                sample_rate,
            }
        }
        _ => SyltClock::Millis,
    };
    let steps = entries.iter().fold(0_u64, |steps, (_, entry)| {
        steps
            .saturating_add(u64::try_from(entry.len()).unwrap_or(u64::MAX))
            .saturating_add(1)
    });
    let read = lyrics::from_sylt(
        clock,
        Untrusted::new(&entries),
        &Limits::DEFAULT,
        &mut Budget::for_input(0, 0, steps),
    );
    assert!(read.is_ok(), "the entries {entries:?} read as {read:?}");
    read
}

/// What the invariants look at in what a reader reported.
struct Shape<'a> {
    /// The lines kept, counting tags.
    lines: usize,
    /// The text of every line, word and tag.
    texts: Vec<&'a str>,
    /// The time of every line with one, and the times of its words.
    times: Vec<(u32, Vec<u32>)>,
}

/// The lines, texts and times in `parsed`.
fn shape(parsed: &Parsed) -> Shape<'_> {
    let (lines, mut texts, times) = match &parsed.lyrics {
        Lyrics::Plain(lines) => (
            lines.len(),
            lines.iter().map(String::as_str).collect::<Vec<_>>(),
            Vec::new(),
        ),
        Lyrics::Lines(lines) => (
            lines.len(),
            lines.iter().map(|line| line.text.as_str()).collect(),
            lines.iter().map(|line| (line.at, Vec::new())).collect(),
        ),
        Lyrics::Words(lines) => (
            lines.len(),
            lines
                .iter()
                .flat_map(|line| line.words.iter().map(|word| word.text.as_str()))
                .collect(),
            lines
                .iter()
                .map(|line| (line.at, line.words.iter().map(|word| word.at).collect()))
                .collect(),
        ),
    };
    texts.extend(parsed.tags.iter().map(|tag| tag.value.as_str()));
    Shape {
        lines: lines.saturating_add(parsed.tags.len()),
        texts,
        times,
    }
}
