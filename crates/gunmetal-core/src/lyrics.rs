//! Lyrics: one typed model, read from `.lrc` files, `ID3v2` `USLT` and
//! `SYLT` frames and Vorbis `LYRICS` comments (SEC-MED-049, SEC-API-090).
//!
//! The model is [`Lyrics`]: plain lines, lines with a time, or lines whose
//! words each have a time, as Enhanced LRC writes them. Every time is in
//! milliseconds from the start of the track. [`position`] finds the line and
//! word a player is at.
//!
//! # LRC
//!
//! [`parse_lrc`] reads an `.lrc` file, and [`from_uslt`] and
//! [`from_vorbis`] read the text of a tag the same way, because taggers
//! often store LRC in them. The text is UTF-8: invalid sequences become
//! U+FFFD, a leading byte-order mark is dropped, and control characters
//! other than tab are removed (SEC-MED-013).
//!
//! - A line starts with one or more stamps, `[m:ss]` or `[m:ss.f]`, where
//!   `m` is any number of digits, `ss` two, and `f` one to three (tenths,
//!   hundredths or thousandths). A line with several stamps is shown at each
//!   of them. Anything else in brackets is text.
//! - Word stamps, `<m:ss>` or `<m:ss.f>`, may follow in the text of a
//!   stamped line. Each starts a word that runs to the next word stamp; the
//!   text before the first one is a word at the line's own time. Word times
//!   never go back: a word stamped before the word or line it follows is
//!   given that word's or line's time.
//! - A line that is exactly `[name:value]`, for one of the names in
//!   [`TagKey`], is a tag. Any other bracketed name, such as `[Chorus]` or
//!   `[Intro: Drake]`, is text.
//! - `[offset:n]` moves every time `n` milliseconds earlier (a negative `n`
//!   moves them later), clamped to one hour either way. The last offset tag
//!   that is a number applies. A time moved before the start becomes 0, and
//!   one moved past the timestamp limit becomes the limit.
//! - ASCII white space at either end of a line, and between a line's
//!   stamps and its text, is not part of the text.
//!
//! A file with at least one stamped line is timed. Its lines are sorted by
//! time, keeping the file's order for equal times; blank lines are left
//! out; and a line without a stamp is kept at the time of the last stamp
//! before it (0 before the first), so no text is lost. A file without a
//! stamped line is plain: runs of blank lines become one blank line, and
//! blank lines before the first and after the last line of text are left
//! out.
//!
//! # SYLT
//!
//! [`from_sylt`] reads the entries of a `SYLT` frame, each a time in the
//! frame's [`SyltClock`] and a text. The `ID3v2` specification marks the
//! start of a new line with a line feed at the start of an entry. When any
//! entry but the first starts with one, every entry is a word and lines start
//! at those entries; otherwise every entry is a line.
//!
//! # Limits
//!
//! Every limit comes from [`Limits`] (SEC-MED-049, SEC-API-090, owner
//! decision D-03: the stricter value of the two rows applies).
//!
//! - An LRC text longer than [`LimitKind::LyricsBytes`] is refused with
//!   [`ParseFault::LimitExceeded`].
//! - A line, or a `SYLT` entry, longer than [`LimitKind::LyricsLineBytes`]
//!   after decoding is cut to that length (SEC-MED-013). Encoded input is
//!   cut to the same length as a work bound.
//! - A stamp past [`LimitKind::LyricsTimestampMs`] is dropped. A line left
//!   with none of its stamps is dropped too; a dropped word stamp's text
//!   joins the word before it.
//! - The model holds at most [`LimitKind::LyricsLines`] lines, counting
//!   tags, and at most [`LimitKind::LyricsBytes`] octets, counting the text
//!   of every line, word and tag and four octets for every stamp kept. A
//!   line with several stamps is a line in the model for each, so a small
//!   text could otherwise make a large model. A line that does not fit, and
//!   everything after it, is dropped.
//!
//! The last three are reported in [`Parsed::over_limit`]: the first place
//! each limit was passed, as [`ParseFault::LimitExceeded`] (SEC-MED-006).
//! Offsets count octets from the start of the text; for `SYLT` they count
//! entries from the first.
//!
//! # Steps
//!
//! Each parse charges its [`Budget`] (SEC-MED-007). Reading LRC charges
//! every line its length plus one, once to find out whether the text is
//! timed and once to read it, so a text of `n` octets takes at most
//! [`LRC_STEPS_PER_OCTET`] × `n` + [`LRC_FIXED_STEPS`] steps. Reading
//! `SYLT` charges every entry its length plus one. The model's own caps
//! bound the work of copying text into it.

use std::num::NonZeroU32;

use crate::parse::{Budget, LimitKind, Limits, ParseFault};
use crate::text::{self, Encoding, Lines as TextLines};
use crate::untrusted::Untrusted;
use crate::values::{Duration, SampleRate};

/// Steps charged for each octet of an LRC text.
pub const LRC_STEPS_PER_OCTET: u64 = 2;

/// Steps charged for an LRC text on top of its octets.
pub const LRC_FIXED_STEPS: u64 = 2;

/// Where a lyrics text came from, so the player can say so.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// An `.lrc` file beside the track (MUS-155).
    LrcFile,
    /// An `ID3v2` `USLT` frame: unsynchronised lyrics (MUS-154).
    Uslt,
    /// An `ID3v2` `SYLT` frame: synchronised lyrics (MUS-154).
    Sylt,
    /// A Vorbis comment named `LYRICS` (MUS-154).
    VorbisLyrics,
}

/// Lyrics, in the order they are sung.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lyrics {
    /// Lines without times. An empty line separates stanzas.
    Plain(Vec<String>),
    /// Lines with a time each, sorted by time.
    Lines(Vec<Line>),
    /// Lines whose words have a time each, sorted by time.
    Words(Vec<WordLine>),
}

/// A line with a time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// When the line starts, in milliseconds.
    pub at: u32,
    /// The line.
    pub text: String,
}

/// A line whose words have a time each.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordLine {
    /// When the line starts, in milliseconds.
    pub at: u32,
    /// The words in order, with times that never go back and never come
    /// before the line's.
    pub words: Vec<Word>,
}

/// A word, or a syllable, with a time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    /// When the word starts, in milliseconds.
    pub at: u32,
    /// The word, with any space after it.
    pub text: String,
}

/// The names of the LRC tags that are kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagKey {
    /// `al`: the album.
    Album,
    /// `ar`: the artist.
    Artist,
    /// `au`: who wrote the words.
    Author,
    /// `#`: a comment.
    Comment,
    /// `by`: who made the LRC file.
    Creator,
    /// `length`: how long the track is, as written.
    Length,
    /// `offset`: how far to move every time.
    Offset,
    /// `ti`: the title.
    Title,
    /// `re` or `tool`: the program that made the file.
    Tool,
    /// `ve`: the version of that program.
    Version,
}

/// An LRC tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    /// Which tag.
    pub key: TagKey,
    /// Its value as written, as one line of text.
    pub value: String,
}

/// What a lyrics text holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    /// The lyrics.
    pub lyrics: Lyrics,
    /// Where they came from.
    pub source: Source,
    /// The LRC tags, in the order they were written.
    pub tags: Vec<Tag>,
    /// The offset applied to every time, in milliseconds; positive is
    /// earlier.
    pub offset: i32,
    /// The first place the text went past each lyrics limit, in the order
    /// they were found. What lay past a limit was cut or dropped; the rest
    /// was kept.
    pub over_limit: Vec<ParseFault>,
}

/// The unit of a `SYLT` frame's times.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyltClock {
    /// Milliseconds (timestamp format 2).
    Millis,
    /// MPEG frames of the stream the tag belongs to (timestamp format 1).
    MpegFrames {
        /// Samples in one frame, such as 1,152.
        samples_per_frame: NonZeroU32,
        /// The stream's sample rate.
        sample_rate: SampleRate,
    },
}

/// The line, and the word in it, that a player is at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cursor {
    /// The index of the line in the lyrics.
    pub line: usize,
    /// The index of the word in that line, or `None` before its first word
    /// and in lyrics without word times.
    pub word: Option<usize>,
}

/// Reads an `.lrc` file.
///
/// # Errors
///
/// [`ParseFault::LimitExceeded`] when the file is longer than
/// [`LimitKind::LyricsBytes`], and [`ParseFault::BudgetExceeded`] when the
/// budget runs out.
pub fn parse_lrc(
    file: Untrusted<&[u8]>,
    limits: &Limits,
    budget: &mut Budget,
) -> Result<Parsed, ParseFault> {
    lrc(file.into_inner(), Source::LrcFile, limits, budget)
}

/// Reads the text of an `ID3v2` `USLT` frame, as LRC.
///
/// # Errors
///
/// As [`parse_lrc`].
pub fn from_uslt(
    text: Untrusted<&str>,
    limits: &Limits,
    budget: &mut Budget,
) -> Result<Parsed, ParseFault> {
    lrc(text.into_inner().as_bytes(), Source::Uslt, limits, budget)
}

/// Reads the value of a Vorbis `LYRICS` comment, as LRC.
///
/// # Errors
///
/// As [`parse_lrc`].
pub fn from_vorbis(
    text: Untrusted<&str>,
    limits: &Limits,
    budget: &mut Budget,
) -> Result<Parsed, ParseFault> {
    lrc(
        text.into_inner().as_bytes(),
        Source::VorbisLyrics,
        limits,
        budget,
    )
}

/// Reads the entries of an `ID3v2` `SYLT` frame: each a time in `clock`'s
/// unit and a text.
///
/// # Errors
///
/// [`ParseFault::BudgetExceeded`] when the budget runs out.
pub fn from_sylt(
    clock: SyltClock,
    entries: Untrusted<&[(u32, &str)]>,
    limits: &Limits,
    budget: &mut Budget,
) -> Result<Parsed, ParseFault> {
    let entries = entries.into_inner();
    let by_word = entries
        .iter()
        .skip(1)
        .any(|(_, text)| text.starts_with('\n'));
    let mut caps = Caps::new(limits);
    let mut lines: Vec<WordLine> = Vec::new();
    for (index, &(when, text)) in entries.iter().enumerate() {
        let offset = u64::try_from(index).unwrap_or(u64::MAX);
        budget.charge(octets(text.as_bytes()).saturating_add(1), offset)?;
        let (starts_line, text) = match text.strip_prefix('\n') {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        let raw = caps.cut(text.as_bytes(), offset);
        let millis = clock.millis(when);
        if !caps.in_time(millis, offset) {
            continue;
        }
        let at = time(millis);
        let text = caps.decode(raw, offset);
        let size = STAMP_OCTETS.saturating_add(octets(text.as_bytes()));
        match lines.last_mut() {
            Some(line) if by_word && !starts_line => {
                if caps.admit(0, size, offset) {
                    let previous = line.words.last().map_or(line.at, |word| word.at);
                    line.words.push(Word {
                        at: previous.max(at),
                        text,
                    });
                }
            }
            _ => {
                if caps.admit(1, size, offset) {
                    lines.push(WordLine {
                        at,
                        words: vec![Word { at, text }],
                    });
                }
            }
        }
        if caps.full {
            break;
        }
    }
    Ok(Parsed {
        lyrics: timed_lyrics(
            lines,
            by_word,
            0,
            time(limits.get(LimitKind::LyricsTimestampMs)),
        ),
        source: Source::Sylt,
        tags: Vec::new(),
        offset: 0,
        over_limit: caps.over_limit,
    })
}

/// The line, and the word in it, that a player `millis` milliseconds into
/// the track is at: the first line of those with the latest time not after
/// `millis`, and in it the last word whose time is not after `millis`.
/// `None` before the first line and for plain lyrics. A later position is
/// never at an earlier line or word.
#[must_use]
pub fn position(lyrics: &Lyrics, millis: u32) -> Option<Cursor> {
    match lyrics {
        Lyrics::Plain(_) => None,
        Lyrics::Lines(lines) => {
            current(lines, millis, |line| line.at).map(|(line, _)| Cursor { line, word: None })
        }
        Lyrics::Words(lines) => current(lines, millis, |line| line.at).map(|(line, words)| {
            let reached = words.words.partition_point(|word| word.at <= millis);
            Cursor {
                line,
                word: reached.checked_sub(1),
            }
        }),
    }
}

/// The first of `lines`, sorted by `at`, with the latest time not after
/// `millis`, and its index.
fn current<T>(lines: &[T], millis: u32, at: impl Fn(&T) -> u32) -> Option<(usize, &T)> {
    let reached = lines.partition_point(|line| at(line) <= millis);
    let latest = at(reached.checked_sub(1).and_then(|last| lines.get(last))?);
    let first = lines.partition_point(|line| at(line) < latest);
    lines.get(first).map(|line| (first, line))
}

impl SyltClock {
    /// The milliseconds a time in this unit stands for, rounded down and
    /// saturating at `u64::MAX`.
    fn millis(self, when: u32) -> u64 {
        match self {
            Self::Millis => u64::from(when),
            Self::MpegFrames {
                samples_per_frame,
                sample_rate,
            } => {
                let samples = u64::from(when).saturating_mul(u64::from(samples_per_frame.get()));
                Duration::from_ticks(samples, sample_rate.hz()).map_or(u64::MAX, Duration::millis)
            }
        }
    }
}

/// A UTF-8 byte-order mark.
const BYTE_ORDER_MARK: &[u8] = b"\xEF\xBB\xBF";

/// The `[offset:]` tag moves times by at most one hour either way
/// (SEC-MED-049).
const MAX_OFFSET_MS: i32 = 3_600_000;

/// The tag names LRC files use, each with the tag it names. Names are
/// matched without regard to ASCII case.
const TAG_NAMES: [(&[u8], TagKey); 11] = [
    (b"al", TagKey::Album),
    (b"ar", TagKey::Artist),
    (b"au", TagKey::Author),
    (b"#", TagKey::Comment),
    (b"by", TagKey::Creator),
    (b"length", TagKey::Length),
    (b"offset", TagKey::Offset),
    (b"ti", TagKey::Title),
    (b"re", TagKey::Tool),
    (b"tool", TagKey::Tool),
    (b"ve", TagKey::Version),
];

/// A part of a stamped line's text: the word stamp it starts with, if any,
/// and its octets.
type Part<'a> = (Option<u64>, &'a [u8]);

/// Model octets each stamp kept adds: a time takes four.
const STAMP_OCTETS: u64 = 4;

/// Reads LRC text that came from `source`.
fn lrc(
    input: &[u8],
    source: Source,
    limits: &Limits,
    budget: &mut Budget,
) -> Result<Parsed, ParseFault> {
    limits.check(LimitKind::LyricsBytes, octets(input), 0)?;
    let (body, start) = match input.strip_prefix(BYTE_ORDER_MARK) {
        Some(rest) => (rest, 3),
        None => (input, 0),
    };
    let timed = is_timed(body, start, limits, budget)?;
    let mut read = Lrc::new(limits);
    for (offset, piece) in pieces(body, start) {
        budget.charge(octets(piece).saturating_add(1), offset)?;
        let line = read.caps.prepare(piece, offset);
        if timed {
            read.timed(line, offset);
        } else {
            read.plain(line, offset);
        }
        if read.caps.full {
            break;
        }
    }
    Ok(read.finish(source, timed))
}

/// Whether a line of `body`, which starts `start` octets into its text,
/// starts with a stamp. Reads lines up to the first that does.
fn is_timed(
    body: &[u8],
    start: u64,
    limits: &Limits,
    budget: &mut Budget,
) -> Result<bool, ParseFault> {
    let mut caps = Caps::new(limits);
    for (offset, piece) in pieces(body, start) {
        budget.charge(octets(piece).saturating_add(1), offset)?;
        if stamp(caps.prepare(piece, offset), b'[', b']').is_some() {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The limits on the model being built, and the first place each lyrics
/// limit was passed.
struct Caps<'l> {
    /// The limits it reads under.
    limits: &'l Limits,
    /// Lines and tags kept.
    lines: u64,
    /// The model's octets: texts, and four for each stamp kept.
    octets: u64,
    /// The first place each limit was passed.
    over_limit: Vec<ParseFault>,
    /// Whether a line did not fit, so reading stops.
    full: bool,
}

impl<'l> Caps<'l> {
    /// An empty model.
    const fn new(limits: &'l Limits) -> Self {
        Self {
            limits,
            lines: 0,
            octets: 0,
            over_limit: Vec::new(),
            full: false,
        }
    }

    /// Records `fault`, unless the limit it passed was passed before.
    fn note(&mut self, fault: ParseFault) {
        let repeated = self.over_limit.iter().any(|earlier| {
            matches!(
                (earlier, &fault),
                (
                    ParseFault::LimitExceeded { limit: before, .. },
                    ParseFault::LimitExceeded { limit: now, .. },
                ) if before == now
            )
        });
        if !repeated {
            self.over_limit.push(fault);
        }
    }

    /// Makes room for `lines` more lines of `octets` octets, read at
    /// `offset`. When they do not fit, notes the limit they pass, marks the
    /// model full and returns `false`.
    fn admit(&mut self, lines: u64, octets: u64, offset: u64) -> bool {
        let lines = self.lines.saturating_add(lines);
        let octets = self.octets.saturating_add(octets);
        let room = self
            .limits
            .check(LimitKind::LyricsLines, lines, offset)
            .and_then(|()| self.limits.check(LimitKind::LyricsBytes, octets, offset));
        if let Err(fault) = room {
            self.note(fault);
            self.full = true;
            return false;
        }
        self.lines = lines;
        self.octets = octets;
        true
    }

    /// `text`, read at `offset`, cut to the line limit as a work bound,
    /// which is noted when the encoded length passes it. Stored text is
    /// capped after decoding.
    fn cut<'a>(&mut self, text: &'a [u8], offset: u64) -> &'a [u8] {
        let max = self.limits.get(LimitKind::LyricsLineBytes);
        if let Err(fault) = self
            .limits
            .check(LimitKind::LyricsLineBytes, octets(text), offset)
        {
            self.note(fault);
        }
        text.get(..usize::try_from(max).unwrap_or(usize::MAX))
            .unwrap_or(text)
    }

    /// The line-length cap as [`text::decode`] takes it.
    fn line_cap(&self) -> u32 {
        u32::try_from(self.limits.get(LimitKind::LyricsLineBytes)).unwrap_or(u32::MAX)
    }

    /// Notes [`LimitKind::LyricsLineBytes`] when `decoded` was truncated,
    /// and returns the stored text.
    fn take_text(&mut self, decoded: text::Text, offset: u64) -> String {
        if decoded.truncated {
            let max = self.limits.get(LimitKind::LyricsLineBytes);
            self.note(ParseFault::LimitExceeded {
                limit: LimitKind::LyricsLineBytes,
                value: max.saturating_add(1),
                max,
                offset,
            });
        }
        decoded.value
    }

    /// Decodes `raw`, read at `offset`, keeping at most the line-length cap
    /// of UTF-8 octets (SEC-MED-013).
    fn decode(&mut self, raw: &[u8], offset: u64) -> String {
        self.take_text(
            text::decode(Untrusted::new(raw), Encoding::Utf8, self.line_cap()),
            offset,
        )
    }

    /// Normalises a tag value the same way, as a single line.
    fn normalise(&mut self, raw: &[u8], offset: u64) -> String {
        self.take_text(
            text::normalise(Untrusted::new(raw), TextLines::Single, self.line_cap()),
            offset,
        )
    }

    /// A line as it is read: without a carriage return before its line
    /// feed, cut to the line limit, and without spaces at either end.
    fn prepare<'a>(&mut self, piece: &'a [u8], offset: u64) -> &'a [u8] {
        self.cut(piece.strip_suffix(b"\r").unwrap_or(piece), offset)
            .trim_ascii()
    }

    /// Whether a stamp of `millis`, read at `offset`, is within the
    /// timestamp limit, which is noted when passed.
    fn in_time(&mut self, millis: u64, offset: u64) -> bool {
        let checked = self
            .limits
            .check(LimitKind::LyricsTimestampMs, millis, offset);
        if let Err(fault) = checked {
            self.note(fault);
        }
        checked.is_ok()
    }
}

/// The lines of `body`, which starts `start` octets into its text, each
/// with its offset and without its line feed.
fn pieces(body: &[u8], start: u64) -> impl Iterator<Item = (u64, &[u8])> {
    body.split(|&octet| octet == b'\n')
        .scan(start, |offset, piece| {
            let at = *offset;
            *offset = offset.saturating_add(octets(piece)).saturating_add(1);
            Some((at, piece))
        })
}

/// The length of `bytes`.
fn octets(bytes: &[u8]) -> u64 {
    u64::try_from(bytes.len()).unwrap_or(u64::MAX)
}

/// Reads up to `most` ASCII digits from the start of `bytes`: their value,
/// saturating at `u64::MAX`, how many there were, and what follows them.
fn digits(bytes: &[u8], most: usize) -> (u64, usize, &[u8]) {
    let count = bytes
        .iter()
        .take(most)
        .take_while(|octet| octet.is_ascii_digit())
        .count();
    let (number, rest) = bytes.split_at_checked(count).unwrap_or_default();
    let value = number.iter().fold(0_u64, |value, digit| {
        value
            .saturating_mul(10)
            .saturating_add(u64::from(digit.saturating_sub(b'0')))
    });
    (value, count, rest)
}

/// Reads a stamp, `m:ss` or `m:ss.f` between `open` and `close`, from the
/// start of `bytes`: its time in milliseconds, saturating at `u64::MAX`,
/// and its length in octets.
fn stamp(bytes: &[u8], open: u8, close: u8) -> Option<(u64, usize)> {
    let rest = bytes.strip_prefix(&[open])?;
    let (minutes, count, rest) = digits(rest, usize::MAX);
    if count == 0 {
        return None;
    }
    let rest = rest.strip_prefix(b":")?;
    let (seconds, count, rest) = digits(rest, usize::MAX);
    if count != 2 {
        return None;
    }
    let (fraction, rest) = match rest.strip_prefix(b".") {
        Some(fraction) => {
            let (value, count, rest) = digits(fraction, 3);
            let scale = match count {
                0 => return None,
                1 => 100,
                2 => 10,
                _ => 1,
            };
            (value.saturating_mul(scale), rest)
        }
        None => (0, rest),
    };
    let rest = rest.strip_prefix(&[close])?;
    let millis = minutes
        .saturating_mul(60_000)
        .saturating_add(seconds.saturating_mul(1_000))
        .saturating_add(fraction);
    Some((millis, bytes.len().saturating_sub(rest.len())))
}

/// The rest of `bytes` after a stamp of `len` octets, or `None` when the
/// stamp did not consume any octet, so a loop that always takes the result
/// cannot hang (SEC-MED-008).
fn after_stamp(bytes: &[u8], len: usize) -> Option<&[u8]> {
    bytes.get(len..).filter(|next| next.len() < bytes.len())
}

/// The stamp and the octets after it, or `None` when there is no stamp
/// that consumes at least one octet (SEC-MED-008).
fn stamped_rest(stamp: Option<(u64, usize)>, bytes: &[u8]) -> Option<(u64, &[u8])> {
    let (millis, len) = stamp?;
    Some((millis, after_stamp(bytes, len)?))
}

/// The next stamp at the start of `bytes` and the octets after it, or
/// `None` when there is no stamp that consumes at least one octet.
fn next_stamp(bytes: &[u8], open: u8, close: u8) -> Option<(u64, &[u8])> {
    stamped_rest(stamp(bytes, open, close), bytes)
}

/// The stamps at the start of `line`, and the text after them.
fn line_stamps(line: &[u8]) -> (Vec<u64>, &[u8]) {
    let mut stamps = Vec::new();
    let mut pos = 0;
    while let Some((millis, after)) = line
        .get(pos..)
        .and_then(|rest| next_stamp(rest, b'[', b']'))
    {
        stamps.push(millis);
        let rest = line.get(pos..).unwrap_or_default();
        // At least one octet, so a stamp that does not shrink `rest` cannot
        // hang the loop (SEC-MED-008).
        pos = pos.saturating_add(rest.len().saturating_sub(after.len()).max(1));
    }
    (stamps, line.get(pos..).unwrap_or_default())
}

/// The text of a stamped line, split at its word stamps. The text before
/// the first word stamp, when there is any, comes first without a stamp.
fn parts(text: &[u8]) -> Vec<Part<'_>> {
    let mut parts = Vec::new();
    let mut stamped = None;
    let mut start = 0;
    let mut next = 0;
    while let Some(found) = text
        .get(next..)
        .and_then(|rest| rest.iter().position(|&octet| octet == b'<'))
    {
        let at = next.saturating_add(found);
        let from = text.get(at..).unwrap_or_default();
        match next_stamp(from, b'<', b'>') {
            Some((millis, rest)) => {
                let part = text.get(start..at).unwrap_or_default();
                if stamped.is_some() || !part.is_empty() {
                    parts.push((stamped, part));
                }
                stamped = Some(millis);
                start = at.saturating_add(from.len().saturating_sub(rest.len()));
                next = start.max(at.saturating_add(1));
            }
            None => next = at.saturating_add(1),
        }
    }
    let part = text.get(start..).unwrap_or_default();
    if stamped.is_some() || !part.is_empty() {
        parts.push((stamped, part));
    }
    parts
}

/// The tag `line` is, if it is exactly `[name:value]` for a known name.
fn tag(line: &[u8]) -> Option<(TagKey, &[u8])> {
    let inner = line.strip_prefix(b"[")?.strip_suffix(b"]")?;
    let mut halves = inner.splitn(2, |&octet| octet == b':');
    let (Some(name), Some(value)) = (halves.next(), halves.next()) else {
        return None;
    };
    let &(_, key) = TAG_NAMES
        .iter()
        .find(|(known, _)| name.eq_ignore_ascii_case(known))?;
    Some((key, value))
}

/// The milliseconds an `[offset:]` value says, clamped to an hour either
/// way, or `None` when it is not a number.
fn offset_ms(value: &[u8]) -> Option<i32> {
    let (negative, unsigned) = match value.trim_ascii() {
        [b'-', rest @ ..] => (true, rest),
        [b'+', rest @ ..] => (false, rest),
        value => (false, value),
    };
    let (magnitude, count, rest) = digits(unsigned, usize::MAX);
    if count == 0 || !rest.is_empty() {
        return None;
    }
    let n = i32::try_from(magnitude)
        .unwrap_or(i32::MAX)
        .min(MAX_OFFSET_MS);
    if negative {
        Some(n.saturating_neg())
    } else {
        Some(n)
    }
}

/// A time kept in the model. Times are checked against the timestamp limit
/// before they are kept, so they always fit.
fn time(millis: u64) -> u32 {
    u32::try_from(millis).unwrap_or(u32::MAX)
}

/// The lines of timed lyrics, sorted, with `shift` milliseconds taken off
/// every time and each time held between 0 and `max`; as words when
/// `by_word`, otherwise as lines.
fn timed_lyrics(mut lines: Vec<WordLine>, by_word: bool, shift: i32, max: u32) -> Lyrics {
    lines.sort_by_key(|line| line.at);
    let moved = |at: u32| {
        let shifted = i64::from(at)
            .saturating_sub(i64::from(shift))
            .clamp(0, i64::from(max));
        u32::try_from(shifted).unwrap_or(0)
    };
    for line in &mut lines {
        line.at = moved(line.at);
        for word in &mut line.words {
            word.at = moved(word.at);
        }
    }
    if by_word {
        return Lyrics::Words(lines);
    }
    Lyrics::Lines(
        lines
            .into_iter()
            .map(|line| Line {
                at: line.at,
                text: line.words.into_iter().map(|word| word.text).collect(),
            })
            .collect(),
    )
}

/// What reading one LRC text has kept so far.
struct Lrc<'l> {
    /// The model's limits.
    caps: Caps<'l>,
    /// Timed lines, in the order read.
    lines: Vec<WordLine>,
    /// Plain lines, in the order read.
    plain: Vec<String>,
    /// Tags, in the order read.
    tags: Vec<Tag>,
    /// Whether a kept line has a word stamp.
    by_word: bool,
    /// The last stamp kept, for a line without one.
    last_at: u64,
    /// The offset of the last `[offset:]` tag that is a number.
    shift: i32,
    /// Whether a blank line came after the last plain line.
    stanza_break: bool,
}

impl<'l> Lrc<'l> {
    /// Nothing read yet.
    const fn new(limits: &'l Limits) -> Self {
        Self {
            caps: Caps::new(limits),
            lines: Vec::new(),
            plain: Vec::new(),
            tags: Vec::new(),
            by_word: false,
            last_at: 0,
            shift: 0,
            stanza_break: false,
        }
    }

    /// Reads one line of plain lyrics.
    fn plain(&mut self, line: &[u8], offset: u64) {
        if let Some((key, value)) = tag(line) {
            self.tag(key, value, offset);
            return;
        }
        let text = self.caps.decode(line, offset);
        if text.is_empty() {
            self.stanza_break = !self.plain.is_empty();
            return;
        }
        let lines = u64::from(self.stanza_break).saturating_add(1);
        if self.caps.admit(lines, octets(text.as_bytes()), offset) {
            if self.stanza_break {
                self.plain.push(String::new());
            }
            self.plain.push(text);
        }
        self.stanza_break = false;
    }

    /// Reads one line of timed lyrics.
    fn timed(&mut self, line: &[u8], offset: u64) {
        let (stamps, text) = line_stamps(line);
        if stamps.is_empty() {
            self.untimed(line, offset);
            return;
        }
        let parts = parts(text.trim_ascii_start());
        for at in stamps {
            if self.caps.full {
                break;
            }
            if self.caps.in_time(at, offset) {
                self.last_at = at;
                self.push_line(at, &parts, STAMP_OCTETS, offset);
            }
        }
    }

    /// Reads a line without a stamp in timed lyrics: a tag, a blank line
    /// to leave out, or text kept at the last stamp.
    fn untimed(&mut self, line: &[u8], offset: u64) {
        if let Some((key, value)) = tag(line) {
            self.tag(key, value, offset);
        } else if !self.caps.decode(line, offset).is_empty() {
            self.push_line(self.last_at, &[(None, line)], 0, offset);
        }
    }

    /// Keeps a timed line at `at` made of `parts`, read at `offset`, if it
    /// fits. Its own time adds `stamp_octets` to the model.
    fn push_line(&mut self, at: u64, parts: &[Part<'_>], stamp_octets: u64, offset: u64) {
        let line_at = time(at);
        let mut words: Vec<Word> = Vec::new();
        let mut size = stamp_octets;
        let mut stamped = false;
        for &(word_stamp, raw) in parts {
            let text = self.caps.decode(raw, offset);
            size = size.saturating_add(octets(text.as_bytes()));
            match word_stamp.filter(|&millis| self.caps.in_time(millis, offset)) {
                Some(millis) => {
                    let previous = words.last().map_or(line_at, |word| word.at);
                    size = size.saturating_add(STAMP_OCTETS);
                    stamped = true;
                    words.push(Word {
                        at: previous.max(time(millis)),
                        text,
                    });
                }
                // The text before the first word stamp, or after one past
                // the limit, which joins the word before it.
                None => match words.last_mut() {
                    Some(word) => word.text.push_str(&text),
                    None => words.push(Word { at: line_at, text }),
                },
            }
        }
        if self.caps.admit(1, size, offset) {
            self.by_word |= stamped;
            self.lines.push(WordLine { at: line_at, words });
        }
    }

    /// Keeps a tag read at `offset`, if it fits, and applies it when it is
    /// an offset that is a number.
    fn tag(&mut self, key: TagKey, value: &[u8], offset: u64) {
        let text = self.caps.normalise(value.trim_ascii(), offset);
        if !self.caps.admit(1, octets(text.as_bytes()), offset) {
            return;
        }
        if key == TagKey::Offset {
            if let Some(shift) = offset_ms(value) {
                self.shift = shift;
            }
        }
        self.tags.push(Tag { key, value: text });
    }

    /// What was read, from `source`, timed or plain.
    fn finish(self, source: Source, timed: bool) -> Parsed {
        let lyrics = if timed {
            timed_lyrics(
                self.lines,
                self.by_word,
                self.shift,
                time(self.caps.limits.get(LimitKind::LyricsTimestampMs)),
            )
        } else {
            Lyrics::Plain(self.plain)
        };
        Parsed {
            lyrics,
            source,
            tags: self.tags,
            offset: self.shift,
            over_limit: self.caps.over_limit,
        }
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
    use std::fmt::Write as _;

    /// Reads `text` as an `.lrc` file under `limits`, with the budget this
    /// module documents: two steps per octet and two more.
    fn read_under(text: &[u8], limits: &Limits) -> Result<Parsed, ParseFault> {
        let len = u64::try_from(text.len()).unwrap();
        let mut budget = Budget::for_input(len, LRC_STEPS_PER_OCTET, LRC_FIXED_STEPS);
        parse_lrc(Untrusted::new(text), limits, &mut budget)
    }

    /// Reads `text` as an `.lrc` file under the default limits.
    fn read(text: &[u8]) -> Result<Parsed, ParseFault> {
        read_under(text, &Limits::DEFAULT)
    }

    /// An `.lrc` file that holds `lyrics` and nothing else.
    fn file(lyrics: Lyrics) -> Parsed {
        Parsed {
            lyrics,
            source: Source::LrcFile,
            tags: Vec::new(),
            offset: 0,
            over_limit: Vec::new(),
        }
    }

    /// An `.lrc` file that holds `lyrics` and `tags`.
    fn tagged(lyrics: Lyrics, tags: &[(TagKey, &str)], offset: i32) -> Parsed {
        Parsed {
            tags: tags
                .iter()
                .map(|&(key, value)| Tag {
                    key,
                    value: value.to_owned(),
                })
                .collect(),
            offset,
            ..file(lyrics)
        }
    }

    /// An `.lrc` file that holds `lyrics` and went past `over_limit`.
    fn limited(lyrics: Lyrics, over_limit: &[ParseFault]) -> Parsed {
        Parsed {
            over_limit: over_limit.to_vec(),
            ..file(lyrics)
        }
    }

    fn plain(lines: &[&str]) -> Lyrics {
        Lyrics::Plain(lines.iter().map(|&line| line.to_owned()).collect())
    }

    fn lines(lines: &[(u32, &str)]) -> Lyrics {
        Lyrics::Lines(
            lines
                .iter()
                .map(|&(at, text)| Line {
                    at,
                    text: text.to_owned(),
                })
                .collect(),
        )
    }

    fn words(lines: &[(u32, &[(u32, &str)])]) -> Lyrics {
        Lyrics::Words(
            lines
                .iter()
                .map(|&(at, words)| WordLine {
                    at,
                    words: words
                        .iter()
                        .map(|&(at, text)| Word {
                            at,
                            text: text.to_owned(),
                        })
                        .collect(),
                })
                .collect(),
        )
    }

    const fn over(limit: LimitKind, value: u64, max: u64, offset: u64) -> ParseFault {
        ParseFault::LimitExceeded {
            limit,
            value,
            max,
            offset,
        }
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn reads_lines_with_stamps() {
        assert_eq!(
            read(b"[00:01.00]One\n[00:02.50]Two\n"),
            Ok(file(lines(&[(1_000, "One"), (2_500, "Two")])))
        );
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn sorts_lines_written_out_of_order() {
        assert_eq!(
            read(b"[00:03.00]C\n[00:01.00]A\n[00:02.00]B"),
            Ok(file(lines(&[(1_000, "A"), (2_000, "B"), (3_000, "C")])))
        );
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn keeps_lines_with_identical_stamps_in_file_order() {
        assert_eq!(
            read(b"[00:05.00]Hello\n[00:05.00]Bonjour\n[00:01.00]First"),
            Ok(file(lines(&[
                (1_000, "First"),
                (5_000, "Hello"),
                (5_000, "Bonjour")
            ])))
        );
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn reads_tenths_hundredths_and_thousandths() {
        assert_eq!(
            read(b"[00:01.5]a\n[00:01.05]b\n[00:01.005]c\n[00:01]d\n[00:01.000]e"),
            Ok(file(lines(&[
                (1_000, "d"),
                (1_000, "e"),
                (1_005, "c"),
                (1_050, "b"),
                (1_500, "a")
            ])))
        );
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn reads_minutes_of_any_length() {
        assert_eq!(
            read(b"[99:59.99]late\n[0:00.01]early\n[000120:00.00]two hours"),
            Ok(file(lines(&[
                (10, "early"),
                (5_999_990, "late"),
                (7_200_000, "two hours")
            ])))
        );
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn shows_a_line_at_each_of_its_stamps() {
        assert_eq!(
            read(b"[00:10.00][00:30.00]Chorus\n[00:20.00]Verse"),
            Ok(file(lines(&[
                (10_000, "Chorus"),
                (20_000, "Verse"),
                (30_000, "Chorus")
            ])))
        );
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn leaves_out_blank_lines_between_timed_lines() {
        assert_eq!(
            read(b"\n[00:01.00]a\n\n  \n\r\n\t\n[00:02.00]b\n\n"),
            Ok(file(lines(&[(1_000, "a"), (2_000, "b")])))
        );
    }

    /// A stamp with no text clears the line shown, so it is kept.
    ///
    /// Verifies: SEC-MED-049
    #[test]
    fn keeps_a_stamped_line_without_text() {
        assert_eq!(
            read(b"[00:01.00]a\n[00:02.00]\n[00:03.00]  \n[00:04.00]b"),
            Ok(file(lines(&[
                (1_000, "a"),
                (2_000, ""),
                (3_000, ""),
                (4_000, "b")
            ])))
        );
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn keeps_a_line_without_a_stamp_at_the_stamp_before_it() {
        assert_eq!(
            read(b"credits\n[00:01.00]a\nmore a\n[00:05.00][00:09.00]b\nafter b\n[00:02.00]c"),
            Ok(file(lines(&[
                (0, "credits"),
                (1_000, "a"),
                (1_000, "more a"),
                (2_000, "c"),
                (5_000, "b"),
                (9_000, "b"),
                (9_000, "after b")
            ])))
        );
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn trims_spaces_at_the_ends_of_a_line_and_after_its_stamps() {
        assert_eq!(
            read(b"  [00:01.00]  a  b \t\r\n\t[00:02.00][00:03.00] c"),
            Ok(file(lines(&[(1_000, "a  b"), (2_000, "c"), (3_000, "c")])))
        );
    }

    /// Verifies: SEC-MED-049, SEC-MED-013
    #[test]
    fn removes_controls_and_replaces_invalid_utf8() {
        assert_eq!(
            read(b"[00:01.00]a\x00b\x1B\xFFc\td\x7F\xC2\x85e"),
            Ok(file(lines(&[(1_000, "ab\u{FFFD}c\tde")])))
        );
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn drops_a_leading_byte_order_mark() {
        assert_eq!(
            read(b"\xEF\xBB\xBF[00:01.00]a\n\xEF\xBB\xBF[00:02.00]b"),
            Ok(file(lines(&[(1_000, "a"), (1_000, "[00:02.00]b")])))
        );
    }

    /// Each of these is one step short of a stamp, so it is text: a line
    /// without a stamp in a timed file.
    ///
    /// Verifies: SEC-MED-049, SEC-MED-008
    #[test]
    fn reads_what_only_looks_like_a_stamp_as_text() {
        let cases = [
            "[]a",
            "[:00]b",
            "[00]c",
            "[00:]d",
            "[00:1]e",
            "[00:1x]f",
            "[00:01.]g",
            "[00:01.1234]h",
            "[00:01.00i",
            "[00:01:00]j",
            "[00:01,00]k",
            "<00:01.00>l",
            "[00:01.00>m",
            "(00:01.00)n",
            "[ 00:01.00]o",
            "[00:123]p",
            "[",
        ];
        let text = format!("[00:01.00]start\n{}", cases.join("\n"));
        let mut expected = vec![(1_000, "start")];
        expected.extend(cases.iter().map(|&case| (1_000, case)));
        assert_eq!(read(text.as_bytes()), Ok(file(lines(&expected))));
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn reads_text_without_stamps_as_plain_lyrics() {
        assert_eq!(
            read(b"First line\r\n  Second line \n"),
            Ok(file(plain(&["First line", "Second line"])))
        );
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn keeps_one_blank_line_between_stanzas() {
        assert_eq!(
            read(b"\n \nA1\nA2\n\n\n\nB1\n \n\t\x01\nC1\n\n\n"),
            Ok(file(plain(&["A1", "A2", "", "B1", "", "C1"])))
        );
    }

    /// Only ASCII white space is trimmed, so a line of U+00A0 is text.
    ///
    /// Verifies: SEC-MED-049
    #[test]
    fn a_line_of_non_ascii_space_is_text() {
        assert_eq!(
            read("\u{00A0}\n[00:01.00]\u{00A0}".as_bytes()),
            Ok(file(lines(&[(0, "\u{00A0}"), (1_000, "\u{00A0}")])))
        );
        assert_eq!(
            read("a\n\u{00A0}\nb".as_bytes()),
            Ok(file(plain(&["a", "\u{00A0}", "b"])))
        );
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn keeps_every_known_tag_in_order() {
        let text = "[ar:Artist]\n[AL:Album]\n[ti:  Title  ]\n[au:Author]\n[by:Creator]\n\
                    [length:03:25]\n[offset:0]\n[re:Editor]\n[Tool:Tool]\n[ve:1.0]\n\
                    [#:a comment]\n[ti:Song [Live]]\n[ar:a\u{202E}b\tc]\n[ar:]\nWords";
        assert_eq!(
            read(text.as_bytes()),
            Ok(tagged(
                plain(&["Words"]),
                &[
                    (TagKey::Artist, "Artist"),
                    (TagKey::Album, "Album"),
                    (TagKey::Title, "Title"),
                    (TagKey::Author, "Author"),
                    (TagKey::Creator, "Creator"),
                    (TagKey::Length, "03:25"),
                    (TagKey::Offset, "0"),
                    (TagKey::Tool, "Editor"),
                    (TagKey::Tool, "Tool"),
                    (TagKey::Version, "1.0"),
                    (TagKey::Comment, "a comment"),
                    (TagKey::Title, "Song [Live]"),
                    (TagKey::Artist, "abc"),
                    (TagKey::Artist, ""),
                ],
                0
            ))
        );
    }

    /// Section names in brackets are common in plain lyrics, so only the
    /// known tag names make a tag.
    ///
    /// Verifies: SEC-MED-049
    #[test]
    fn reads_other_bracketed_names_as_text() {
        assert_eq!(
            read(b"[Intro: Drake]\n[Chorus]\n[ti]\n[ti:x] after\nbefore [ti:x]\n[xy:z]\n[:z]"),
            Ok(file(plain(&[
                "[Intro: Drake]",
                "[Chorus]",
                "[ti]",
                "[ti:x] after",
                "before [ti:x]",
                "[xy:z]",
                "[:z]"
            ])))
        );
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn keeps_tags_among_timed_lines() {
        assert_eq!(
            read(b"[ti:Song]\n[00:01.00]a\n[ar:Someone]\n[00:02.00]b"),
            Ok(tagged(
                lines(&[(1_000, "a"), (2_000, "b")]),
                &[(TagKey::Title, "Song"), (TagKey::Artist, "Someone")],
                0
            ))
        );
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn a_positive_offset_shows_lines_sooner() {
        assert_eq!(
            read(b"[offset:+500]\n[00:01.00]a\n[00:00.20]b"),
            Ok(tagged(
                lines(&[(0, "b"), (500, "a")]),
                &[(TagKey::Offset, "+500")],
                500
            ))
        );
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn a_negative_offset_shows_lines_later() {
        assert_eq!(
            read(b"[offset: -250 ]\n[00:01.00]a"),
            Ok(tagged(
                lines(&[(1_250, "a")]),
                &[(TagKey::Offset, "-250")],
                -250
            ))
        );
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn an_offset_without_a_sign_shows_lines_sooner() {
        assert_eq!(
            read(b"[00:01.00]a\n[offset:100]"),
            Ok(tagged(
                lines(&[(900, "a")]),
                &[(TagKey::Offset, "100")],
                100
            ))
        );
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn clamps_the_offset_to_one_hour_either_way() {
        assert_eq!(
            read(b"[offset:3600000]\n[60:00.00]a\n[120:00.00]b"),
            Ok(tagged(
                lines(&[(0, "a"), (3_600_000, "b")]),
                &[(TagKey::Offset, "3600000")],
                3_600_000
            ))
        );
        assert_eq!(
            read(b"[offset:+7200000]\n[60:00.00]a\n[120:00.00]b"),
            Ok(tagged(
                lines(&[(0, "a"), (3_600_000, "b")]),
                &[(TagKey::Offset, "+7200000")],
                3_600_000
            ))
        );
        assert_eq!(
            read(b"[offset:-99999999999999999999999]\n[1380:00.00]a\n[1410:00.00]b"),
            Ok(tagged(
                lines(&[(86_400_000, "a"), (86_400_000, "b")]),
                &[(TagKey::Offset, "-99999999999999999999999")],
                -3_600_000
            ))
        );
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn the_last_offset_that_is_a_number_applies() {
        assert_eq!(
            read(b"[offset:100]\n[offset:300]\n[offset:]\n[offset:12x]\n[offset:- 3]\n[offset:+-3]\n[00:01.00]a"),
            Ok(tagged(
                lines(&[(700, "a")]),
                &[
                    (TagKey::Offset, "100"),
                    (TagKey::Offset, "300"),
                    (TagKey::Offset, ""),
                    (TagKey::Offset, "12x"),
                    (TagKey::Offset, "- 3"),
                    (TagKey::Offset, "+-3"),
                ],
                300
            ))
        );
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn reads_word_stamps() {
        assert_eq!(
            read(b"[00:01.00]<00:01.00>Hello <00:01.50>world<00:02.00>\n[00:03.00]A plain line\nno stamp"),
            Ok(file(words(&[
                (1_000, &[(1_000, "Hello "), (1_500, "world"), (2_000, "")]),
                (3_000, &[(3_000, "A plain line")]),
                (3_000, &[(3_000, "no stamp")]),
            ])))
        );
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn text_before_the_first_word_stamp_is_a_word_at_the_line() {
        assert_eq!(
            read(b"[00:01.00] Oh <00:01.20>yeah\n[00:02.00]"),
            Ok(file(words(&[
                (1_000, &[(1_000, "Oh "), (1_200, "yeah")]),
                (2_000, &[]),
            ])))
        );
    }

    /// Two word stamps with nothing between them make a word with no text.
    ///
    /// Verifies: SEC-MED-049
    #[test]
    fn keeps_a_word_stamp_with_no_text() {
        assert_eq!(
            read(b"[00:01.00]<00:01.00><00:01.50>b"),
            Ok(file(words(&[(1_000, &[(1_000, ""), (1_500, "b")])])))
        );
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn word_times_never_go_back() {
        assert_eq!(
            read(b"[00:05.00]<00:06.00>a<00:04.00>b<00:07.00>c\n[00:08.00]<00:01.00>d"),
            Ok(file(words(&[
                (5_000, &[(6_000, "a"), (6_000, "b"), (7_000, "c")]),
                (8_000, &[(8_000, "d")]),
            ])))
        );
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn a_line_with_several_stamps_repeats_its_words_at_each() {
        assert_eq!(
            read(b"[00:10.00][00:20.00]<00:10.00>a<00:10.50>b"),
            Ok(file(words(&[
                (10_000, &[(10_000, "a"), (10_500, "b")]),
                (20_000, &[(20_000, "a"), (20_000, "b")]),
            ])))
        );
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn the_offset_moves_word_times_too() {
        assert_eq!(
            read(b"[offset:500]\n[00:01.00]<00:01.00>a<00:02.00>b<00:00.10>c"),
            Ok(tagged(
                words(&[(500, &[(500, "a"), (1_500, "b"), (1_500, "c")])]),
                &[(TagKey::Offset, "500")],
                500
            ))
        );
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn angle_brackets_that_are_not_stamps_are_text() {
        assert_eq!(
            read(b"[00:01.00]a <b> <00:0x> <00:01.00 c<"),
            Ok(file(lines(&[(1_000, "a <b> <00:0x> <00:01.00 c<")])))
        );
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn reads_tag_text_like_a_file() {
        let mut budget = Budget::for_input(64, 2, 2);
        assert_eq!(
            from_uslt(
                Untrusted::new("[ti:Song]\n[00:01.00]a"),
                &Limits::DEFAULT,
                &mut budget
            ),
            Ok(Parsed {
                source: Source::Uslt,
                ..tagged(lines(&[(1_000, "a")]), &[(TagKey::Title, "Song")], 0)
            })
        );
        assert_eq!(
            from_vorbis(
                Untrusted::new("Plain\n\ntext"),
                &Limits::DEFAULT,
                &mut budget
            ),
            Ok(Parsed {
                source: Source::VorbisLyrics,
                ..file(plain(&["Plain", "", "text"]))
            })
        );
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn reads_nothing_as_no_lyrics() {
        assert_eq!(read(b""), Ok(file(plain(&[]))));
        assert_eq!(read(b"\n\r\n \n"), Ok(file(plain(&[]))));
    }

    /// `count` copies of `c`.
    fn run_of(c: char, count: usize) -> String {
        (0..count).map(|_| c).collect()
    }

    /// `limits` with `kind` set to `value`.
    fn lowered(kind: LimitKind, value: u64) -> Limits {
        Limits::DEFAULT.with_override(kind, value).unwrap()
    }

    /// Verifies: SEC-API-090, SEC-MED-006, SEC-TM-032
    #[test]
    fn reads_256_kib_and_refuses_one_octet_more() {
        let line = run_of('a', 4_095);
        let mut text = String::new();
        for _ in 0..64 {
            text.push_str(&line);
            text.push('\n');
        }
        assert_eq!(text.len(), 262_144);
        let expected: Vec<&str> = (0..64).map(|_| line.as_str()).collect();
        assert_eq!(read(text.as_bytes()), Ok(file(plain(&expected))));
        let over_by_one = format!("{text}b");
        assert_eq!(
            read(over_by_one.as_bytes()),
            Err(over(LimitKind::LyricsBytes, 262_145, 262_144, 0))
        );
        let mut budget = Budget::for_input(u64::MAX, 0, 0);
        assert_eq!(
            from_vorbis(Untrusted::new(&over_by_one), &Limits::DEFAULT, &mut budget),
            Err(over(LimitKind::LyricsBytes, 262_145, 262_144, 0))
        );
    }

    /// Verifies: SEC-MED-049, SEC-MED-006, SEC-TM-032
    #[test]
    fn cuts_a_line_longer_than_4_kib() {
        let a = run_of('a', 4_086);
        let b = run_of('b', 4_087);
        let d = run_of('d', 5_000);
        // The first line is exactly 4 KiB, the third is too once its
        // carriage return is gone, and the second and fourth are longer.
        let text = format!("[00:01.00]{a}\n[00:02.00]{b}\n[00:03.00]{a}\r\n{d}");
        assert_eq!(
            read(text.as_bytes()),
            Ok(limited(
                lines(&[
                    (1_000, &a),
                    (2_000, &run_of('b', 4_086)),
                    (3_000, &a),
                    (3_000, &run_of('d', 4_096)),
                ]),
                &[over(LimitKind::LyricsLineBytes, 4_097, 4_096, 4_097)]
            ))
        );
    }

    /// Invalid UTF-8 becomes U+FFFD (three octets), so a line that fits the
    /// encoded 4 KiB work bound can still exceed it after decoding. The
    /// length cap is applied to the stored text (SEC-MED-013).
    ///
    /// Verifies: SEC-MED-013, SEC-MED-049, SEC-API-090
    #[test]
    fn cuts_decoded_invalid_utf8_to_4_kib() {
        let max = usize::try_from(Limits::DEFAULT.get(LimitKind::LyricsLineBytes)).unwrap();
        assert_eq!(max, 4_096);
        // 1,365 × U+FFFD is 4,095 octets, the most that fits; 1,366 would
        // be 4,098.
        let kept = run_of('\u{FFFD}', max / 3);
        assert_eq!(kept.len(), 4_095);
        let under_encoded: Vec<u8> = (0..1_366).map(|_| 0xFF).collect();
        assert_eq!(under_encoded.len() * 3, 4_098);
        let at_encoded: Vec<u8> = (0..4_096).map(|_| 0xFF).collect();
        let line_over = over(LimitKind::LyricsLineBytes, 4_097, 4_096, 0);

        for raw in [under_encoded.as_slice(), at_encoded.as_slice()] {
            let parsed = read(raw).unwrap();
            assert_well_formed(&parsed, &Limits::DEFAULT);
            assert_eq!(parsed, limited(plain(&[&kept]), &[line_over]));
        }

        let mut timed = b"[00:01.00]".to_vec();
        timed.extend(&under_encoded);
        let parsed = read(&timed).unwrap();
        assert_well_formed(&parsed, &Limits::DEFAULT);
        assert_eq!(parsed, limited(lines(&[(1_000, &kept)]), &[line_over]));

        let mut tagged_line = b"[ti:".to_vec();
        tagged_line.extend(&under_encoded);
        tagged_line.push(b']');
        let parsed = read(&tagged_line).unwrap();
        assert_well_formed(&parsed, &Limits::DEFAULT);
        assert_eq!(
            parsed,
            Parsed {
                over_limit: vec![line_over],
                ..tagged(plain(&[]), &[(TagKey::Title, kept.as_str())], 0)
            }
        );

        let mut words_line = b"[00:01.00]<00:01.00>".to_vec();
        words_line.extend(&under_encoded);
        let parsed = read(&words_line).unwrap();
        assert_well_formed(&parsed, &Limits::DEFAULT);
        assert_eq!(
            parsed,
            limited(words(&[(1_000, &[(1_000, &kept)])]), &[line_over])
        );

        let sylt_under = run_of('\u{FFFD}', 1_366);
        assert_eq!(sylt_under.len(), 4_098);
        let parsed = sylt(&[(0, &sylt_under)]).unwrap();
        assert_well_formed(&parsed, &Limits::DEFAULT);
        assert_eq!(
            parsed,
            synced(
                lines(&[(0, &kept)]),
                &[over(LimitKind::LyricsLineBytes, 4_098, 4_096, 0)]
            )
        );
        let sylt_long = run_of('\u{FFFD}', 4_096);
        assert_eq!(sylt_long.len(), 12_288);
        let parsed = sylt(&[(0, &sylt_long)]).unwrap();
        assert_well_formed(&parsed, &Limits::DEFAULT);
        assert_eq!(
            parsed,
            synced(
                lines(&[(0, &kept)]),
                &[over(LimitKind::LyricsLineBytes, 12_288, 4_096, 0)]
            )
        );
    }

    /// The lines of a timed file in which line `i` is `x` at `i` × 10 ms,
    /// each 12 octets long.
    fn numbered_lines(count: u32) -> String {
        let mut text = String::new();
        for i in 0..count {
            let at = i * 10;
            writeln!(
                text,
                "[{:02}:{:02}.{:02}]x",
                at / 60_000,
                at / 1_000 % 60,
                at % 1_000 / 10
            )
            .unwrap();
        }
        text
    }

    /// Verifies: SEC-API-090, SEC-MED-006, SEC-TM-032
    #[test]
    fn keeps_10_000_lines_and_drops_the_rest() {
        let expected: Vec<(u32, &str)> = (0..10_000).map(|i| (i * 10, "x")).collect();
        assert_eq!(
            read(numbered_lines(10_000).as_bytes()),
            Ok(file(lines(&expected)))
        );
        assert_eq!(
            read(numbered_lines(10_001).as_bytes()),
            Ok(limited(
                lines(&expected),
                &[over(LimitKind::LyricsLines, 10_001, 10_000, 120_000)]
            ))
        );
    }

    /// Verifies: SEC-API-090, SEC-MED-006
    #[test]
    fn counts_tags_and_stanza_breaks_as_lines() {
        let limits = lowered(LimitKind::LyricsLines, 4);
        assert_eq!(
            read_under(b"[ti:T]\na\n\nb\n\nc\nd", &limits),
            Ok(Parsed {
                tags: vec![Tag {
                    key: TagKey::Title,
                    value: "T".to_owned()
                }],
                ..limited(
                    plain(&["a", "", "b"]),
                    &[over(LimitKind::LyricsLines, 6, 4, 13)]
                )
            })
        );
        assert_eq!(
            read_under(b"a\n\nb\n[ti:T]", &limits),
            Ok(tagged(plain(&["a", "", "b"]), &[(TagKey::Title, "T")], 0))
        );
        assert_eq!(
            read_under(b"a\n\nb\nc\n[ti:T]", &limits),
            Ok(limited(
                plain(&["a", "", "b", "c"]),
                &[over(LimitKind::LyricsLines, 5, 4, 7)]
            ))
        );
    }

    /// One line written with two stamps is two lines in the model, so a
    /// text well under 256 KiB can make a model over it.
    ///
    /// Verifies: SEC-API-090, SEC-MED-006, SEC-TM-032
    #[test]
    fn keeps_the_model_within_256_kib() {
        let a = run_of('a', 4_076);
        let b = run_of('b', 1_024);
        // 32 lines of two copies of 4 + 4,076 octets make 261,120. A line
        // without a stamp adds only its text, which brings the model to
        // exactly 262,144, so the next stamp does not fit. Reading stops
        // there: the stamp past 24 hours after it and the tag are not read.
        let doubled = format!("[00:00.00][00:00.01]{a}\n");
        let text = format!(
            "{}{b}\n[00:02.00][1440:00.001]\n[ar:]",
            (0..32).map(|_| doubled.as_str()).collect::<String>()
        );
        let mut expected: Vec<(u32, &str)> = (0..32).map(|_| (0, a.as_str())).collect();
        expected.extend((0..32).map(|_| (10, a.as_str())));
        expected.push((10, &b));
        assert_eq!(
            read(text.as_bytes()),
            Ok(limited(
                lines(&expected),
                &[over(LimitKind::LyricsBytes, 262_148, 262_144, 132_129)]
            ))
        );
    }

    /// Every word stamp kept adds four octets to the model, once for each
    /// stamp of its line.
    ///
    /// Verifies: SEC-API-090, SEC-MED-006
    #[test]
    fn counts_four_octets_for_every_stamp_kept() {
        // Each copy is 4 for the line, and 4 and 1 for each of 4 words: 24.
        /// The copy of the line at `at`: words never before the line.
        fn copy(at: u32) -> (u32, &'static [(u32, &'static str)]) {
            match at {
                0 => (0, &[(0, "a"), (1_000, "b"), (2_000, "c"), (3_000, "d")]),
                1_000 => (
                    1_000,
                    &[(1_000, "a"), (1_000, "b"), (2_000, "c"), (3_000, "d")],
                ),
                2_000 => (
                    2_000,
                    &[(2_000, "a"), (2_000, "b"), (2_000, "c"), (3_000, "d")],
                ),
                _ => (
                    3_000,
                    &[(3_000, "a"), (3_000, "b"), (3_000, "c"), (3_000, "d")],
                ),
            }
        }
        let text = b"[0:00][0:01][0:02][0:03]<0:00>a<0:01>b<0:02>c<0:03>d";
        assert_eq!(
            read_under(text, &lowered(LimitKind::LyricsBytes, 96)),
            Ok(file(words(&[
                copy(0),
                copy(1_000),
                copy(2_000),
                copy(3_000)
            ])))
        );
        assert_eq!(
            read_under(text, &lowered(LimitKind::LyricsBytes, 95)),
            Ok(limited(
                words(&[copy(0), copy(1_000), copy(2_000)]),
                &[over(LimitKind::LyricsBytes, 96, 95, 0)]
            ))
        );
    }

    /// Verifies: SEC-MED-049, SEC-MED-006, SEC-TM-032
    #[test]
    fn drops_a_stamp_past_24_hours() {
        assert_eq!(
            read(b"[1440:00.00]last\n[1440:00.001]late\nafter\n[00:01.00][1440:00.01]both\n[99999999999999999999:00]huge"),
            Ok(limited(
                lines(&[
                    (1_000, "both"),
                    (86_400_000, "last"),
                    (86_400_000, "after")
                ]),
                &[over(LimitKind::LyricsTimestampMs, 86_400_001, 86_400_000, 17)]
            ))
        );
        // Stamps saturate rather than overflow, and a file whose only
        // stamps are dropped is still timed.
        assert_eq!(
            read(b"[99999999999999999999:00]huge\nuntimed"),
            Ok(limited(
                lines(&[(0, "untimed")]),
                &[over(LimitKind::LyricsTimestampMs, u64::MAX, 86_400_000, 0)]
            ))
        );
    }

    /// Verifies: SEC-MED-049, SEC-MED-006
    #[test]
    fn a_word_stamp_past_24_hours_joins_the_word_before() {
        assert_eq!(
            read(b"[00:01.00]<00:01.00>a<1440:00.001>b<00:02.00>c\n[00:03.00]<1440:00.001>d"),
            Ok(limited(
                words(&[
                    (1_000, &[(1_000, "ab"), (2_000, "c")]),
                    (3_000, &[(3_000, "d")]),
                ]),
                &[over(
                    LimitKind::LyricsTimestampMs,
                    86_400_001,
                    86_400_000,
                    0
                )]
            ))
        );
        // A line whose word stamps were all dropped has no word times.
        assert_eq!(
            read(b"[00:03.00]x<1440:00.001>d"),
            Ok(limited(
                lines(&[(3_000, "xd")]),
                &[over(
                    LimitKind::LyricsTimestampMs,
                    86_400_001,
                    86_400_000,
                    0
                )]
            ))
        );
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn reads_every_limit_from_the_limits_given() {
        let limits = lowered(LimitKind::LyricsLineBytes, 14)
            .with_override(LimitKind::LyricsTimestampMs, 2_000)
            .unwrap();
        // The offset moves the first line past the lowered timestamp limit,
        // so it is held at that limit.
        assert_eq!(
            read_under(
                b"[offset:-5000]\n[0:01]abcdefghijkl\n[0:03]x\n[0:02]y",
                &limits
            ),
            Ok(Parsed {
                over_limit: vec![
                    over(LimitKind::LyricsLineBytes, 18, 14, 15),
                    over(LimitKind::LyricsTimestampMs, 3_000, 2_000, 34),
                ],
                ..tagged(
                    lines(&[(2_000, "abcdefgh"), (2_000, "y")]),
                    &[(TagKey::Offset, "-5000")],
                    -5_000
                )
            })
        );
    }

    /// Reads `text` as an `.lrc` file with a budget of `steps`, and returns
    /// what it read and the steps left.
    fn read_with(text: &[u8], limits: &Limits, steps: u64) -> (Result<Parsed, ParseFault>, u64) {
        let mut budget = Budget::for_input(0, 0, steps);
        let parsed = parse_lrc(Untrusted::new(text), limits, &mut budget);
        (parsed, budget.remaining())
    }

    /// Verifies: SEC-MED-007, SEC-TM-032
    #[test]
    fn charges_each_line_its_length_and_one_in_each_pass() {
        // Finding the stamp takes the first line; reading takes both.
        let timed = b"[00:01.00]a\nb";
        assert_eq!(
            read_with(timed, &Limits::DEFAULT, 26),
            (Ok(file(lines(&[(1_000, "a"), (1_000, "b")]))), 0)
        );
        assert_eq!(
            read_with(timed, &Limits::DEFAULT, 25),
            (Err(ParseFault::BudgetExceeded { offset: 12 }), 0)
        );
        // Plain text is read to its end to look for a stamp.
        let plain_text = b"a\nb";
        assert_eq!(
            read_with(plain_text, &Limits::DEFAULT, 8),
            (Ok(file(plain(&["a", "b"]))), 0)
        );
        assert_eq!(
            read_with(plain_text, &Limits::DEFAULT, 5),
            (Err(ParseFault::BudgetExceeded { offset: 0 }), 0)
        );
        assert_eq!(
            read_with(plain_text, &Limits::DEFAULT, 3),
            (Err(ParseFault::BudgetExceeded { offset: 2 }), 0)
        );
        // A byte-order mark is not a line.
        assert_eq!(
            read_with(b"\xEF\xBB\xBFa", &Limits::DEFAULT, 3),
            (Err(ParseFault::BudgetExceeded { offset: 3 }), 0)
        );
        // Lines after the model is full are not read.
        assert_eq!(
            read_with(
                b"[0:01]a\n[0:02]b\n[0:03]c",
                &lowered(LimitKind::LyricsLines, 1),
                100
            ),
            (
                Ok(limited(
                    lines(&[(1_000, "a")]),
                    &[over(LimitKind::LyricsLines, 2, 1, 8)]
                )),
                76
            )
        );
    }

    /// Reads `entries` of a `SYLT` frame under `limits`, with one step for
    /// each octet of text and one for each entry.
    fn sylt_under(
        clock: SyltClock,
        entries: &[(u32, &str)],
        limits: &Limits,
    ) -> Result<Parsed, ParseFault> {
        let steps = entries
            .iter()
            .map(|(_, text)| u64::try_from(text.len()).unwrap() + 1)
            .sum();
        let mut budget = Budget::for_input(0, 0, steps);
        from_sylt(clock, Untrusted::new(entries), limits, &mut budget)
    }

    /// Reads `entries` of a `SYLT` frame in milliseconds.
    fn sylt(entries: &[(u32, &str)]) -> Result<Parsed, ParseFault> {
        sylt_under(SyltClock::Millis, entries, &Limits::DEFAULT)
    }

    /// A `SYLT` frame that holds `lyrics` and went past `over_limit`.
    fn synced(lyrics: Lyrics, over_limit: &[ParseFault]) -> Parsed {
        Parsed {
            source: Source::Sylt,
            ..limited(lyrics, over_limit)
        }
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn reads_sylt_entries_as_lines() {
        assert_eq!(
            sylt(&[
                (1_000, "\none"),
                (3_000, "three"),
                (2_000, "two\u{7}"),
                (2_000, " two again ")
            ]),
            Ok(synced(
                lines(&[
                    (1_000, "one"),
                    (2_000, "two"),
                    (2_000, " two again "),
                    (3_000, "three")
                ]),
                &[]
            ))
        );
        assert_eq!(sylt(&[]), Ok(synced(lines(&[]), &[])));
    }

    /// A line feed at the start of any entry but the first starts a new
    /// line, as the `ID3v2` specification says, and the entries between
    /// are words.
    ///
    /// Verifies: SEC-MED-049
    #[test]
    fn reads_sylt_syllables_as_words() {
        assert_eq!(
            sylt(&[
                (1_000, "Strang"),
                (1_200, "ers"),
                (1_500, "\nin the"),
                (1_300, " night"),
                (1_100, "\nAgain")
            ]),
            Ok(synced(
                words(&[
                    (1_000, &[(1_000, "Strang"), (1_200, "ers")]),
                    (1_100, &[(1_100, "Again")]),
                    (1_500, &[(1_500, "in the"), (1_500, " night")]),
                ]),
                &[]
            ))
        );
    }

    /// MPEG-1 Layer III at 44.1 kHz: 1,152 samples a frame.
    fn mpeg_frames() -> SyltClock {
        SyltClock::MpegFrames {
            samples_per_frame: NonZeroU32::new(1_152).unwrap(),
            sample_rate: SampleRate::new(44_100).unwrap(),
        }
    }

    /// Verifies: SEC-MED-049, SEC-MED-006
    #[test]
    fn converts_mpeg_frames_to_milliseconds() {
        // 100 frames are 115,200 samples, 2,612.2 ms; 3,307,500 frames are
        // exactly 24 hours.
        assert_eq!(
            sylt_under(
                mpeg_frames(),
                &[
                    (0, "a"),
                    (100, "b"),
                    (75_000, "c"),
                    (3_307_500, "d"),
                    (3_307_501, "e"),
                    (u32::MAX, "f")
                ],
                &Limits::DEFAULT
            ),
            Ok(synced(
                lines(&[(0, "a"), (2_612, "b"), (1_959_183, "c"), (86_400_000, "d")]),
                &[over(
                    LimitKind::LyricsTimestampMs,
                    86_400_026,
                    86_400_000,
                    4
                )]
            ))
        );
        // A time too long to be a duration at all saturates.
        assert_eq!(
            sylt_under(mpeg_frames(), &[(u32::MAX, "f")], &Limits::DEFAULT),
            Ok(synced(
                lines(&[]),
                &[over(LimitKind::LyricsTimestampMs, u64::MAX, 86_400_000, 0)]
            ))
        );
    }

    /// Verifies: SEC-MED-049, SEC-MED-006, SEC-TM-032
    #[test]
    fn cuts_a_sylt_entry_longer_than_4_kib() {
        let c = run_of('c', 4_096);
        let b = format!("{}b", run_of('a', 4_095));
        assert_eq!(
            sylt(&[(0, &format!("\n{c}")), (1, &b), (2, &format!("{c}d"))]),
            Ok(synced(
                lines(&[(0, &c), (1, &b), (2, &c)]),
                &[over(LimitKind::LyricsLineBytes, 4_097, 4_096, 2)]
            ))
        );
    }

    /// Verifies: SEC-API-090, SEC-MED-006, SEC-TM-032
    #[test]
    fn keeps_10_000_sylt_lines_and_drops_the_rest() {
        let entries: Vec<(u32, &str)> = (0..10_001).map(|i| (i, "x")).collect();
        let expected: Vec<(u32, &str)> = (0..10_000).map(|i| (i, "x")).collect();
        assert_eq!(
            sylt(entries.get(..10_000).unwrap()),
            Ok(synced(lines(&expected), &[]))
        );
        assert_eq!(
            sylt(&entries),
            Ok(synced(
                lines(&expected),
                &[over(LimitKind::LyricsLines, 10_001, 10_000, 10_000)]
            ))
        );
    }

    /// Words add to the model's octets but are not lines.
    ///
    /// Verifies: SEC-API-090, SEC-MED-006
    #[test]
    fn keeps_sylt_words_within_the_model_limits() {
        let entries = [(0, "ab"), (1, "\ncd"), (2, "e")];
        let both = words(&[(0, &[(0, "ab")]), (1, &[(1, "cd"), (2, "e")])]);
        assert_eq!(
            sylt_under(
                SyltClock::Millis,
                &entries,
                &lowered(LimitKind::LyricsBytes, 17)
            ),
            Ok(synced(both.clone(), &[]))
        );
        assert_eq!(
            sylt_under(
                SyltClock::Millis,
                &entries,
                &lowered(LimitKind::LyricsLines, 2)
            ),
            Ok(synced(both, &[]))
        );
        assert_eq!(
            sylt_under(
                SyltClock::Millis,
                &entries,
                &lowered(LimitKind::LyricsBytes, 16)
            ),
            Ok(synced(
                words(&[(0, &[(0, "ab")]), (1, &[(1, "cd")])]),
                &[over(LimitKind::LyricsBytes, 17, 16, 2)]
            ))
        );
        // Reading stops at the first entry that does not fit: the entry
        // past 24 hours after it is not read.
        assert_eq!(
            sylt_under(
                SyltClock::Millis,
                &[(0, "a"), (1, "b"), (2, "\nc"), (u32::MAX, "\nd")],
                &lowered(LimitKind::LyricsLines, 1)
            ),
            Ok(synced(
                words(&[(0, &[(0, "a"), (1, "b")])]),
                &[over(LimitKind::LyricsLines, 2, 1, 2)]
            ))
        );
    }

    /// Runs `work` on a thread with the 256 KiB stack SEC-MED-001 names, so
    /// a parse that recursed too deeply would fail here.
    fn on_small_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(262_144)
            .spawn(work)
            .expect("the test thread starts")
            .join()
            .expect("the parse returned instead of panicking")
    }

    /// Brackets nested far deeper than any parser could recurse are read
    /// as text, because nothing in the parser recurses.
    ///
    /// Verifies: SEC-MED-005, SEC-MED-001
    #[test]
    fn reads_deeply_nested_brackets_as_text_on_a_small_stack() {
        let nested = format!("{}{}", run_of('[', 100_000), run_of(']', 100_000));
        assert_eq!(
            on_small_stack(move || read(nested.as_bytes())),
            Ok(limited(
                plain(&[&run_of('[', 4_096)]),
                &[over(LimitKind::LyricsLineBytes, 200_000, 4_096, 0)]
            ))
        );
        let angles = format!("[00:00.00]{}{}", run_of('<', 2_000), run_of('>', 2_000));
        let text = angles.clone();
        assert_eq!(
            on_small_stack(move || read(text.as_bytes())),
            Ok(file(lines(&[(0, angles.get(10..).unwrap())])))
        );
    }

    /// Octets that reach the interesting cases more often than random ones
    /// would: stamps of both kinds, their pieces, tags, line ends, a
    /// byte-order mark and invalid UTF-8.
    fn hostile_lrc() -> impl Strategy<Value = Vec<u8>> {
        let piece = prop_oneof![
            any::<u8>().prop_map(|octet| vec![octet]),
            (0_u32..200, 0_u32..60, 0_u32..1_000)
                .prop_map(|(m, s, ms)| format!("[{m}:{s:02}.{ms:03}]").into_bytes()),
            (0_u32..200, 0_u32..60, 0_u32..100)
                .prop_map(|(m, s, cs)| format!("<{m}:{s:02}.{cs:02}>").into_bytes()),
            select(vec![
                b"[".to_vec(),
                b"]".to_vec(),
                b"<".to_vec(),
                b">".to_vec(),
                b":".to_vec(),
                b".".to_vec(),
                b"0".to_vec(),
                b" ".to_vec(),
                b"\n".to_vec(),
                b"\r\n".to_vec(),
                b"\t".to_vec(),
                b"[offset:-1500]".to_vec(),
                b"[offset:+99999999]".to_vec(),
                b"[ti:x]".to_vec(),
                b"[1440:00.001]".to_vec(),
                b"<1440:00.001>".to_vec(),
                b"\xEF\xBB\xBF".to_vec(),
                b"\xFF".to_vec(),
                b"\x00".to_vec(),
                b"\xC2\x85".to_vec(),
            ]),
        ];
        vec(piece, 0..48).prop_map(|pieces| pieces.concat())
    }

    /// Octets made of one piece written many times over: the inputs that
    /// make parsers work hardest per octet.
    fn repetitive_lrc() -> impl Strategy<Value = Vec<u8>> {
        let piece = select(vec![
            b"[0:00]".to_vec(),
            b"[0:00]<0:00>".to_vec(),
            b"<0:00>".to_vec(),
            b"[".to_vec(),
            b"<".to_vec(),
            b"\n".to_vec(),
            b"[0:00]\n".to_vec(),
            b"x\n\n".to_vec(),
            b"[ti:]\n".to_vec(),
        ]);
        (piece, 0_usize..3_000)
            .prop_map(|(piece, count)| (0..count).flat_map(|_| piece.iter().copied()).collect())
    }

    /// What holds for every text read under `limits`: no more lines and
    /// octets than the limits allow, times within the timestamp limit and
    /// sorted, word times that never go back, no control character but tab
    /// and line feed, and at most one fault for each lyrics limit.
    fn assert_well_formed(parsed: &Parsed, limits: &Limits) {
        let max = u32::try_from(limits.get(LimitKind::LyricsTimestampMs)).unwrap();
        let mut texts: Vec<&str> = parsed.tags.iter().map(|tag| tag.value.as_str()).collect();
        let (count, times): (usize, Vec<u32>) = match &parsed.lyrics {
            Lyrics::Plain(lines) => {
                texts.extend(lines.iter().map(String::as_str));
                (lines.len(), Vec::new())
            }
            Lyrics::Lines(lines) => {
                texts.extend(lines.iter().map(|line| line.text.as_str()));
                (lines.len(), lines.iter().map(|line| line.at).collect())
            }
            Lyrics::Words(lines) => {
                for line in lines {
                    let at: Vec<u32> = line.words.iter().map(|word| word.at).collect();
                    assert!(
                        at.iter().all(|&word| word >= line.at && word <= max),
                        "{line:?}"
                    );
                    assert!(at.is_sorted(), "{line:?}");
                    texts.extend(line.words.iter().map(|word| word.text.as_str()));
                }
                (lines.len(), lines.iter().map(|line| line.at).collect())
            }
        };
        assert!(
            times.is_sorted() && times.iter().all(|&at| at <= max),
            "{parsed:?}"
        );
        let lines = u64::try_from(count + parsed.tags.len()).unwrap();
        assert!(lines <= limits.get(LimitKind::LyricsLines), "{parsed:?}");
        let octets: usize = texts.iter().map(|text| text.len()).sum();
        assert!(u64::try_from(octets).unwrap() <= limits.get(LimitKind::LyricsBytes));
        let max_line = usize::try_from(limits.get(LimitKind::LyricsLineBytes)).unwrap();
        assert!(
            texts.iter().all(|text| text.len() <= max_line),
            "{parsed:?}"
        );
        assert!(
            texts
                .iter()
                .all(|text| text.chars().all(|c| !c.is_control() || c == '\t')),
            "{parsed:?}"
        );
        // Every fault is for one of the lyrics limits, each at most once.
        let per_limit = [
            LimitKind::LyricsBytes,
            LimitKind::LyricsLines,
            LimitKind::LyricsLineBytes,
            LimitKind::LyricsTimestampMs,
        ]
        .map(|limit| {
            parsed
                .over_limit
                .iter()
                .filter(|fault| {
                    matches!(fault, ParseFault::LimitExceeded { limit: kind, .. } if *kind == limit)
                })
                .count()
        });
        assert!(
            per_limit.iter().all(|&count| count <= 1)
                && per_limit.iter().sum::<usize>() == parsed.over_limit.len(),
            "{parsed:?}"
        );
    }

    /// Every kind of lyrics, with a tab and a limit passed, holds what the
    /// property below checks of any reading.
    #[test]
    fn example_readings_are_well_formed() {
        for text in [
            &b"Plain\n\nlyrics\n[ti:T]"[..],
            b"[00:02.00]a\tb\n[00:01.00][1440:00.001]c",
            b"[00:01.00]<00:01.00>a<00:02.00>b",
        ] {
            assert_well_formed(&read(text).unwrap(), &Limits::DEFAULT);
        }
    }

    /// Writes `lines` as an `.lrc` file, one stamp with milliseconds a
    /// line: the reference writer the properties check the reader against.
    fn write_lrc(lines: &[(u32, String)]) -> String {
        let mut text = String::new();
        for (at, line) in lines {
            writeln!(
                text,
                "[{}:{:02}.{:03}]{line}",
                at / 60_000,
                at / 1_000 % 60,
                at % 1_000
            )
            .unwrap();
        }
        text
    }

    proptest! {
        /// Verifies: SEC-MED-001, SEC-HIS-036, SEC-MED-006, SEC-TM-032
        #[test]
        fn returns_well_formed_lyrics_for_any_octets(
            bytes in prop_oneof![hostile_lrc(), repetitive_lrc(), vec(any::<u8>(), 0..256)],
        ) {
            let input = bytes.clone();
            let parsed = on_small_stack(move || read(&input));
            let parsed = parsed.unwrap();
            assert_well_formed(&parsed, &Limits::DEFAULT);
            // The same text from a tag reads the same, apart from its source.
            let text = String::from_utf8_lossy(&bytes);
            let mut budget = Budget::for_input(u64::MAX, 0, u64::MAX);
            let from_file = read(text.as_bytes());
            for (read_tag, source) in [
                (from_uslt as fn(Untrusted<&str>, &Limits, &mut Budget) -> _, Source::Uslt),
                (from_vorbis, Source::VorbisLyrics),
            ] {
                prop_assert_eq!(
                    read_tag(Untrusted::new(&text), &Limits::DEFAULT, &mut budget),
                    from_file.clone().map(|parsed| Parsed { source, ..parsed })
                );
            }
        }

        /// Verifies: SEC-MED-007, SEC-MED-008, SEC-TM-032
        #[test]
        fn never_needs_more_than_two_steps_an_octet_and_two(
            bytes in prop_oneof![hostile_lrc(), repetitive_lrc()],
        ) {
            prop_assert!(read(&bytes).is_ok());
        }

        /// Text with no `[` is plain, so both passes read every line: two
        /// steps for each octet and two more, exactly.
        ///
        /// Verifies: SEC-MED-007
        #[test]
        fn plain_text_takes_exactly_two_steps_an_octet_and_two(
            bytes in vec(any::<u8>().prop_filter("no stamp or mark", |&octet| octet != b'[' && octet != 0xEF), 0..512),
        ) {
            let len = u64::try_from(bytes.len()).unwrap();
            let mut budget = Budget::for_input(len, LRC_STEPS_PER_OCTET, LRC_FIXED_STEPS);
            let parsed = parse_lrc(Untrusted::new(&bytes), &Limits::DEFAULT, &mut budget);
            let is_plain = matches!(parsed, Ok(Parsed { lyrics: Lyrics::Plain(_), .. }));
            prop_assert!(is_plain, "{:?}", parsed);
            prop_assert_eq!(budget.remaining(), 0);
        }

        /// Verifies: SEC-MED-049
        #[test]
        fn reads_back_the_lines_written(
            written in vec((0_u32..=86_400_000, "[a-zA-Z0-9 ,.'!?-]{0,16}"), 1..24),
        ) {
            let mut expected: Vec<(u32, &str)> = written
                .iter()
                .map(|(at, line)| (*at, line.trim_matches(' ')))
                .collect();
            expected.sort_by_key(|&(at, _)| at);
            prop_assert_eq!(read(write_lrc(&written).as_bytes()), Ok(file(lines(&expected))));
        }

        /// Verifies: SEC-MED-049
        #[test]
        fn reads_back_the_sylt_lines_written(
            written in vec((0_u32..=86_400_000, "[a-z ]{0,8}"), 0..24),
        ) {
            let entries: Vec<(u32, &str)> =
                written.iter().map(|(at, line)| (*at, line.as_str())).collect();
            let mut expected = entries.clone();
            expected.sort_by_key(|&(at, _)| at);
            prop_assert_eq!(sylt(&entries), Ok(synced(lines(&expected), &[])));
        }

        /// On its own this would pass for a `position` that always returned
        /// `None`; the examples above carry the weight.
        #[test]
        fn a_later_position_is_never_at_an_earlier_line_or_word(
            written in vec((0_u32..10_000, vec((0_u32..10_000, "[a-z]{0,3}"), 0..4)), 1..12),
            mut times in vec(0_u32..12_000, 1..32),
        ) {
            let mut text = String::new();
            for (at, words) in &written {
                write!(text, "[0:{:02}.{:03}]", at / 1_000, at % 1_000).unwrap();
                for (word_at, word) in words {
                    write!(text, "<0:{:02}.{:03}>{word}", word_at / 1_000, word_at % 1_000).unwrap();
                }
                text.push('\n');
            }
            let parsed = read(text.as_bytes()).unwrap();
            times.sort_unstable();
            let cursors: Vec<_> = times.iter().map(|&millis| position(&parsed.lyrics, millis)).collect();
            prop_assert!(cursors.is_sorted(), "{:?} at {:?}: {:?}", parsed.lyrics, times, cursors);
            // The line is the first of those with the latest time reached,
            // found here by a linear search.
            let starts: Vec<u32> = match &parsed.lyrics {
                Lyrics::Lines(lines) => lines.iter().map(|line| line.at).collect(),
                Lyrics::Words(lines) => lines.iter().map(|line| line.at).collect(),
                Lyrics::Plain(_) => Vec::new(),
            };
            for (&millis, cursor) in times.iter().zip(&cursors) {
                let latest = starts.iter().copied().filter(|&at| at <= millis).max();
                let line = latest.and_then(|latest| starts.iter().position(|&at| at == latest));
                prop_assert_eq!(cursor.map(|cursor| cursor.line), line);
            }
        }
    }

    /// A time, and the line and word a player is at then, if any.
    type Expected = (u32, Option<(usize, Option<usize>)>);

    /// Checks `position` in `lyrics` at each time of `cases` against the
    /// line and word written beside it.
    fn assert_positions(lyrics: &Lyrics, cases: &[Expected]) {
        for &(millis, expected) in cases {
            assert_eq!(
                position(lyrics, millis),
                expected.map(|(line, word)| Cursor { line, word }),
                "{millis}"
            );
        }
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn finds_the_line_a_player_is_at() {
        let lyrics = lines(&[(1_000, "a"), (2_000, "b"), (2_000, "c"), (3_000, "d")]);
        assert_positions(
            &lyrics,
            &[
                (0, None),
                (999, None),
                (1_000, Some((0, None))),
                (1_999, Some((0, None))),
                // Of two lines with one time, the first.
                (2_000, Some((1, None))),
                (2_999, Some((1, None))),
                (3_000, Some((3, None))),
                (u32::MAX, Some((3, None))),
            ],
        );
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn finds_the_word_a_player_is_at() {
        let lyrics = words(&[
            (1_000, &[(1_000, "a"), (1_500, "b")]),
            (2_000, &[(2_500, "c")]),
            (3_000, &[]),
            (3_000, &[(3_000, "d")]),
        ]);
        assert_positions(
            &lyrics,
            &[
                (0, None),
                (999, None),
                (1_000, Some((0, Some(0)))),
                (1_499, Some((0, Some(0)))),
                (1_500, Some((0, Some(1)))),
                (1_999, Some((0, Some(1)))),
                // Before the line's first word.
                (2_000, Some((1, None))),
                (2_499, Some((1, None))),
                (2_500, Some((1, Some(0)))),
                (2_999, Some((1, Some(0)))),
                (3_000, Some((2, None))),
                (u32::MAX, Some((2, None))),
            ],
        );
    }

    /// A stamp that consumes no octets is not a stamp: the rest is `None`,
    /// so the loops that read stamps stop instead of hanging.
    ///
    /// Verifies: SEC-MED-008
    #[test]
    fn a_stamp_that_does_not_advance_is_not_a_stamp() {
        assert_eq!(after_stamp(b"", 0), None);
        assert_eq!(after_stamp(b"abc", 0), None);
        assert_eq!(after_stamp(b"abc", 1), Some(&b"bc"[..]));
        assert_eq!(after_stamp(b"abc", 3), Some(&b""[..]));
        assert_eq!(after_stamp(b"abc", 4), None);
        assert_eq!(after_stamp(b"[00:01.00]x", 10), Some(&b"x"[..]));
        assert_eq!(next_stamp(b"", b'[', b']'), None);
        assert_eq!(next_stamp(b"x[00:01.00]", b'[', b']'), None);
        assert_eq!(
            next_stamp(b"[00:01.00]x", b'[', b']'),
            Some((1_000, &b"x"[..]))
        );
        assert_eq!(
            next_stamp(b"<00:01.50>y", b'<', b'>'),
            Some((1_500, &b"y"[..]))
        );
        assert_eq!(next_stamp(b"[00:01.00]x", b'<', b'>'), None);
        assert_eq!(stamped_rest(None, b"abc"), None);
        assert_eq!(stamped_rest(Some((0, 0)), b"abc"), None);
        assert_eq!(stamped_rest(Some((0, 4)), b"abc"), None);
        assert_eq!(stamped_rest(Some((7, 1)), b"abc"), Some((7, &b"bc"[..])));
        assert_eq!(stamped_rest(Some((7, 3)), b"abc"), Some((7, &b""[..])));
    }

    /// A time kept in the model always fits; one that does not saturates.
    ///
    /// Verifies: SEC-MED-004
    #[test]
    fn a_time_past_u32_max_saturates() {
        assert_eq!(time(0), 0);
        assert_eq!(time(86_400_000), 86_400_000);
        assert_eq!(time(u64::from(u32::MAX)), u32::MAX);
        assert_eq!(time(u64::from(u32::MAX).saturating_add(1)), u32::MAX);
        assert_eq!(time(u64::MAX), u32::MAX);
    }

    /// Verifies: SEC-MED-049
    #[test]
    fn plain_lyrics_and_no_lyrics_have_no_position() {
        for lyrics in [plain(&["a", "b"]), plain(&[]), lines(&[]), words(&[])] {
            assert_eq!(position(&lyrics, 0), None, "{lyrics:?}");
            assert_eq!(position(&lyrics, u32::MAX), None, "{lyrics:?}");
        }
    }

    /// Verifies: SEC-MED-007, SEC-TM-032
    #[test]
    fn charges_each_sylt_entry_its_length_and_one() {
        let entries = [(0, "ab"), (1, "c"), (u32::MAX, "")];
        let mut budget = Budget::for_input(0, 0, 6);
        assert_eq!(
            from_sylt(
                SyltClock::Millis,
                Untrusted::new(&entries),
                &Limits::DEFAULT,
                &mut budget
            ),
            Ok(synced(
                lines(&[(0, "ab"), (1, "c")]),
                &[over(
                    LimitKind::LyricsTimestampMs,
                    u64::from(u32::MAX),
                    86_400_000,
                    2
                )]
            ))
        );
        assert_eq!(budget.remaining(), 0);
        let mut budget = Budget::for_input(0, 0, 5);
        assert_eq!(
            from_sylt(
                SyltClock::Millis,
                Untrusted::new(&entries),
                &Limits::DEFAULT,
                &mut budget
            ),
            Err(ParseFault::BudgetExceeded { offset: 2 })
        );
    }
}
