//! Why a catalogue value was refused.

use crate::id::IdKind;

use super::position::PositionPart;
use super::tech::{Codec, Container};

/// Why a catalogue constructor refused its inputs.
///
/// Like [`ValueError`](crate::values::ValueError), this is reported where a
/// value from a file is dropped, as a tag or probe problem of the package
/// that reads the file; it never reaches a client on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogError {
    /// An identifier of another kind where one kind was expected, such as
    /// an album's identifier given as a track's.
    WrongIdKind {
        /// The kind expected.
        expected: IdKind,
    },
    /// A track or disc number or total outside 1 to 9,999.
    OutOfRange {
        /// Which part of the position.
        part: PositionPart,
        /// The value given.
        value: u16,
    },
    /// A codec the container cannot carry.
    CodecNotInContainer {
        /// The codec given.
        codec: Codec,
        /// The container given.
        container: Container,
    },
    /// A bit depth for a lossy codec, which has none.
    BitDepthOnLossy {
        /// The codec given.
        codec: Codec,
    },
    /// A bitrate of zero.
    ZeroBitrate,
    /// A byte range that ends before it starts.
    BackwardsRange {
        /// The first octet's offset.
        start: u64,
        /// The offset just past the last octet.
        end: u64,
    },
    /// A credit with an empty or blank name.
    BlankName,
    /// Lyrics from an `ID3v2` `SYLT` frame, which is always timed, read as
    /// plain text.
    UntimedSyncedLyrics,
}
