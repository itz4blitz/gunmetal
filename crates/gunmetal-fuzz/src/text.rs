//! The harness for the text decoders and the ingest normaliser in
//! `gunmetal_core::text` (SEC-MED-013, SEC-API-048).

use gunmetal_core::text::{self, Encoding, Lines, Text};
use gunmetal_core::untrusted::Untrusted;

/// The cap every call uses, small enough that fuzzing reaches it often.
pub const TEXT_CAP: u32 = 64;

/// Every encoding, in the order of [`TextOutcome::decoded`].
pub const ENCODINGS: [Encoding; 5] = [
    Encoding::Utf8,
    Encoding::Utf16Bom,
    Encoding::Utf16Be,
    Encoding::Utf16Le,
    Encoding::Latin1,
];

/// What the text functions reported for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextOutcome {
    /// [`text::decode`] in each of [`ENCODINGS`].
    pub decoded: [Text; 5],
    /// [`text::normalise`] of the input as a single-line field.
    pub single: Text,
    /// [`text::normalise`] of the input as a multi-line field.
    pub multi: Text,
}

/// Feeds `data` to [`text::decode`] in every encoding and to
/// [`text::normalise`] for both kinds of field, all capped at
/// [`TEXT_CAP`] octets.
///
/// # Panics
///
/// Panics when a result breaks an invariant that holds for every input:
/// longer than the cap, a control character other than tab and line feed,
/// in a single-line field any control, bidirectional control or line
/// separator, or a normalised value that changes when normalised again.
#[must_use]
pub fn text(data: &[u8]) -> TextOutcome {
    let cap = usize::try_from(TEXT_CAP).unwrap_or(usize::MAX);
    let decoded = ENCODINGS.map(|encoding| text::decode(data, encoding, TEXT_CAP));
    let single = text::normalise(Untrusted::new(data), Lines::Single, TEXT_CAP);
    let multi = text::normalise(Untrusted::new(data), Lines::Multi, TEXT_CAP);
    for out in decoded.iter().chain([&multi]) {
        assert!(
            out.value.len() <= cap
                && out
                    .value
                    .chars()
                    .all(|c| !c.is_control() || c == '\t' || c == '\n'),
            "{out:?} is over {cap} octets or holds a control"
        );
    }
    assert!(
        single.value.len() <= cap
            && single.value.chars().all(|c| {
                !c.is_control()
                    && !('\u{2028}'..='\u{202E}').contains(&c)
                    && !('\u{2066}'..='\u{2069}').contains(&c)
            }),
        "{single:?} is over {cap} octets or holds a forbidden character"
    );
    for (out, lines) in [(&single, Lines::Single), (&multi, Lines::Multi)] {
        let again = text::normalise(Untrusted::new(out.value.as_bytes()), lines, TEXT_CAP);
        assert!(
            again.value == out.value && !again.truncated && !again.replaced,
            "normalising {out:?} again gave {again:?}"
        );
    }
    TextOutcome {
        decoded,
        single,
        multi,
    }
}
