//! The harness for the Vorbis stream headers in
//! [`gunmetal_core::formats::vorbis`].

use gunmetal_core::formats::vorbis::{self, VorbisError, VorbisIdent};
use gunmetal_core::parse::{Budget, Depth, Limits};

/// What the Vorbis header parsers reported for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome<'a> {
    /// [`vorbis::vorbis_ident`] applied to the whole input.
    pub ident: Result<VorbisIdent, VorbisError>,
    /// [`vorbis::vorbis_comments`] applied to the whole input.
    pub comments: Result<&'a [u8], VorbisError>,
}

/// Feeds `data` to [`vorbis::vorbis_ident`] and
/// [`vorbis::vorbis_comments`] under the default limits, at the top level
/// of a container, each with a budget no input can spend.
///
/// # Panics
///
/// Panics when a parser breaks an invariant that holds for every input:
/// more steps than its documented bound (one for the identification
/// header, a quarter of the input's length plus two for the comment
/// header), an error offset past the end of the input, or a comment block
/// that is not the octets right after the signature and before a framing
/// octet.
#[must_use]
pub fn run(data: &[u8]) -> Outcome<'_> {
    let octets = u64::try_from(data.len()).unwrap_or(u64::MAX);
    let mut budget = Budget::for_input(0, 0, u64::MAX);
    let ident = vorbis::vorbis_ident(data, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT);
    assert_eq!(
        u64::MAX - budget.remaining(),
        1,
        "vorbis_ident spent other than one step"
    );
    let mut budget = Budget::for_input(0, 0, u64::MAX);
    let comments =
        vorbis::vorbis_comments(data, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT);
    assert!(
        u64::MAX - budget.remaining() <= octets / 4 + 2,
        "vorbis_comments spent more than a quarter of {octets} octets plus two steps, {budget:?} left"
    );
    for error in [ident.as_ref().err(), comments.as_ref().err()]
        .into_iter()
        .flatten()
    {
        assert!(
            error.offset() <= octets,
            "{error:?} lies outside {octets} octets"
        );
    }
    if let Ok(block) = comments {
        assert!(
            data.get(7..)
                .is_some_and(|rest| rest.len() > block.len() && rest.starts_with(block)),
            "vorbis_comments returned {block:02X?}, which does not follow the signature"
        );
    }
    Outcome { ident, comments }
}
