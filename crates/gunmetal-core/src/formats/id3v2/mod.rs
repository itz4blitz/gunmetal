//! `ID3v2` tags: versions 2.2, 2.3 and 2.4.
//!
//! [`parse`] reads a whole tag, from its `ID3` header to its last frame,
//! into typed frames: text with each encoding and the 2.4 null separator,
//! user text, comments, lyrics, synchronised lyrics, involved people,
//! unique file identifiers, and pictures as references to their octets.
//! Every other frame is kept raw for the file inspector. Compressed and
//! encrypted frames are skipped and recorded, and `CHAP` and `CTOC` frames
//! are walked within the depth limit and kept raw.
//!
//! [`header`] and [`footer`] read the ten octets at either end of a tag,
//! so a caller holding only the start or the end of a file knows how many
//! octets the tag takes before it reads them.
//!
//! What a frame means for the music model is decided elsewhere; this module
//! only reads what the tag holds.
//!
//! This file is a registry: it holds only module lines and re-exports.

mod body;
mod frame;
mod frames;
mod source;
mod tag;
#[cfg(test)]
mod testing;

pub use frame::{
    Credit, Frame, FrameBody, FrameId, LanguageText, PictureRef, SyncedLyrics, SyncedText,
};
pub use tag::{
    BUDGET_FIXED, BUDGET_PER_OCTET, ExtendedHeader, Header, Id3v2Error, Id3v2Tag, Span, TagProblem,
    footer, header, parse,
};
