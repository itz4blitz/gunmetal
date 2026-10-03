//! The harness for the `ID3v2` parser in
//! [`gunmetal_core::formats::id3v2`].

use std::borrow::Cow;

use gunmetal_core::formats::id3v2::{
    self, BUDGET_FIXED, BUDGET_PER_OCTET, FrameBody, Header, Id3v2Error, Id3v2Tag,
};
use gunmetal_core::parse::{Budget, Limits, ParseFault};

/// What the `ID3v2` parser reported for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome<'a> {
    /// [`id3v2::header`] of the input.
    pub header: Result<Header, Id3v2Error>,
    /// [`id3v2::footer`] of the input.
    pub footer: Result<Header, Id3v2Error>,
    /// [`id3v2::parse`] of the input, under the default limits and the
    /// budget the parser documents for the input's length.
    pub tag: Result<Id3v2Tag, Id3v2Error>,
    /// The octets of every raw body and picture the tag refers to, in
    /// frame order, read through [`Span::read`](id3v2::Span::read).
    pub octets: Vec<Cow<'a, [u8]>>,
}

/// Feeds `data` to [`id3v2::header`], [`id3v2::footer`] and
/// [`id3v2::parse`], then reads every span the tag refers to.
///
/// # Panics
///
/// Panics when the parser breaks an invariant that holds for every input:
/// an error whose offset lies past the input, a parse that runs out of the
/// budget its documentation promises is enough, a tag whose header differs
/// from what [`id3v2::header`] reads, a frame that starts past the input,
/// or a span that does not lie inside it.
#[must_use]
pub fn run(data: &[u8]) -> Outcome<'_> {
    let len = u64::try_from(data.len()).unwrap_or(u64::MAX);
    let header = id3v2::header(data);
    let footer = id3v2::footer(data);
    let mut budget = Budget::for_input(len, BUDGET_PER_OCTET, BUDGET_FIXED);
    let tag = id3v2::parse(data, &Limits::DEFAULT, &mut budget);
    for error in [header.err(), footer.err(), tag.as_ref().err().copied()]
        .into_iter()
        .flatten()
    {
        assert!(
            error.offset() <= len
                && !matches!(error, Id3v2Error::Fault(ParseFault::BudgetExceeded { .. })),
            "{error:?} for {len} octets"
        );
    }
    assert!(
        tag.as_ref().ok().is_none_or(|tag| header == Ok(tag.header)),
        "{tag:?} does not have the header {header:?}"
    );
    let frames = tag.as_ref().map_or(&[][..], |tag| tag.frames.as_slice());
    let mut octets = Vec::new();
    for frame in frames {
        assert!(frame.offset < len, "{frame:?} starts past {len} octets");
        let span = match &frame.body {
            FrameBody::Raw(span) => span,
            FrameBody::Picture(picture) => &picture.data,
            _ => continue,
        };
        let read = span.read(data);
        assert!(read.is_some(), "{span:?} lies outside {len} octets");
        octets.extend(read);
    }
    Outcome {
        header,
        footer,
        tag,
        octets,
    }
}
