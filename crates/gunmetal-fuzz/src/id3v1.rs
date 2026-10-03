//! The harness for the `ID3v1` parser in
//! [`gunmetal_core::formats::id3v1`].

use gunmetal_core::formats::id3v1::{self, Id3v1Error, Id3v1Tag};
use gunmetal_core::parse::{Budget, Limits, ParseFault, Window};

/// Octets the tail case puts before the input.
pub const BEFORE: u64 = 1_000;

/// What the `ID3v1` parser reported for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// [`id3v1::find_v1`] over the input as a whole file.
    pub whole: Result<Option<Id3v1Tag>, Id3v1Error>,
    /// The same over the input as the last octets of a file [`BEFORE`]
    /// octets longer.
    pub tail: Result<Option<Id3v1Tag>, Id3v1Error>,
}

/// Feeds `data` to [`id3v1::find_v1`] as a whole file, and as the last
/// octets of a file [`BEFORE`] octets longer, with one step of budget.
///
/// # Panics
///
/// Panics when the parser breaks an invariant that holds for every input:
/// a tag anywhere but the file's last 128 octets, a field longer than 30
/// Latin-1 characters can be or holding a control other than tab and line
/// feed, more than one step spent, or the two readings disagreeing about
/// the same octets.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let len = u64::try_from(data.len()).unwrap_or(u64::MAX);
    let file_len = BEFORE.saturating_add(len);
    let whole = find(Window {
        offset: 0,
        bytes: data,
        file_len: len,
    });
    let tail = find(Window {
        offset: BEFORE,
        bytes: data,
        file_len,
    });
    for (found, end) in [(&whole, len), (&tail, file_len)] {
        assert!(
            !matches!(
                found,
                Err(Id3v1Error::Fault(ParseFault::BudgetExceeded { .. }))
            ),
            "one step was not enough for {found:?}"
        );
        // Every field is what 30 Latin-1 octets can decode to: at most 60
        // octets of UTF-8, with no control but tab and line feed.
        if let Ok(Some(tag)) = found {
            assert!(
                tag.range == (end.saturating_sub(128)..end)
                    && [&tag.title, &tag.artist, &tag.album, &tag.year, &tag.comment]
                        .iter()
                        .all(|field| {
                            !field.truncated
                                && !field.replaced
                                && field.value.len() <= 60
                                && field
                                    .value
                                    .chars()
                                    .all(|c| !c.is_control() || c == '\t' || c == '\n')
                        }),
                "{tag:?} is not the last 128 octets of {end}, or a field is malformed"
            );
        }
    }
    // The tail shows the same octets, 1,000 further on, when it holds a
    // whole tag's worth of them; otherwise it cannot see where a tag would
    // start.
    let shifted = whole.clone().map(|found| {
        found.map(|tag| Id3v1Tag {
            range: tag.range.start.saturating_add(BEFORE)..tag.range.end.saturating_add(BEFORE),
            ..tag
        })
    });
    let short = Err(Id3v1Error::Fault(ParseFault::Truncated {
        offset: file_len.saturating_sub(128),
        needed: 128,
        available: len,
    }));
    assert!(
        tail == if len < 128 { short } else { shifted },
        "the tail read {tail:?} where the whole file read {whole:?}"
    );
    Outcome { whole, tail }
}

/// [`id3v1::find_v1`] over `window` under the default limits, with the
/// one step it may spend.
fn find(window: Window<'_>) -> Result<Option<Id3v1Tag>, Id3v1Error> {
    let mut budget = Budget::for_input(0, 0, 1);
    id3v1::find_v1(window, &Limits::DEFAULT, &mut budget)
}
