//! The frames a tag holds, decoded as far as this module decodes them.

use crate::text::Text;

use super::tag::Span;

/// A frame's identifier, as the tag wrote it: three characters in 2.2,
/// four in 2.3 and 2.4.
///
/// A frame read into [`Frame`] always has an identifier of capital letters
/// and digits; a [`TagProblem`](super::TagProblem) may carry any octets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameId {
    /// A 2.2 identifier, such as `TT2`.
    Three([u8; 3]),
    /// A 2.3 or 2.4 identifier, such as `TIT2`.
    Four([u8; 4]),
}

/// One frame of a tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// Which frame it is.
    pub id: FrameId,
    /// Where the frame header starts, in octets from the start of the tag.
    pub offset: u64,
    /// The frame's two flag octets as written, most significant first; 0
    /// in 2.2, which has none.
    pub flags: u16,
    /// What the frame holds.
    pub body: FrameBody,
}

/// What a frame holds.
///
/// Every text is decoded, stripped of control characters and capped by the
/// [`Limits`](crate::parse::Limits); a capped text says so in its
/// `truncated` flag. The values of a tag's text, user text and involved
/// people frames are counted together, and so are the lines of its
/// synchronised lyrics frames: a frame may hold fewer than were written, or
/// none, once the tag has reached a limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameBody {
    /// A text information frame (`T***` and 2.2's `T**`): its values. A 2.4
    /// frame holds every value separated by a terminator; 2.2 and 2.3 hold
    /// one value and ignore whatever follows its terminator, as their
    /// specifications say. `GRP1`, the grouping iTunes writes, and `XSOT`,
    /// `XSOP` and `XSOA`, the sort names 2.3 tags hold, are read as text
    /// information frames too.
    Text(Vec<Text>),
    /// A user-defined text frame (`TXXX`, `TXX`).
    UserText {
        /// What the values are.
        description: Text,
        /// The values, separated by terminators in 2.4 and single before.
        values: Vec<Text>,
    },
    /// An involved people list (`TIPL`, `TMCL`, 2.3's `IPLS` and 2.2's
    /// `IPL`): pairs of a role and a name.
    People(Vec<Credit>),
    /// A comment (`COMM`, `COM`).
    Comment(LanguageText),
    /// Unsynchronised lyrics (`USLT`, `ULT`).
    Lyrics(LanguageText),
    /// Synchronised lyrics or text (`SYLT`, `SLT`).
    SyncedLyrics(SyncedLyrics),
    /// An attached picture (`APIC`, `PIC`). The picture itself is never
    /// decoded here.
    Picture(PictureRef),
    /// A unique file identifier (`UFID`, `UFI`), such as a `MusicBrainz`
    /// recording ID under the owner `http://musicbrainz.org`.
    Ufid {
        /// Who issued the identifier.
        owner: Text,
        /// The identifier's octets.
        id: Vec<u8>,
    },
    /// Any other frame, and any frame whose fields could not be read, as
    /// the octets of its body. Chapters (`CHAP`), tables of contents
    /// (`CTOC`), ratings (`POPM`) and links (`W***`) are among them; a
    /// link is never followed.
    Raw(Span),
}

/// One credit of an involved people list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credit {
    /// What the person did, such as "producer" or "guitar".
    pub role: Text,
    /// Who did it. Empty when the list ended after a role.
    pub name: Text,
}

/// A text in a language with a short description, as comments and
/// unsynchronised lyrics hold them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageText {
    /// The ISO 639-2 language code as written. These octets are not
    /// checked.
    pub language: [u8; 3],
    /// The short content description.
    pub description: Text,
    /// The text.
    pub text: Text,
}

/// Synchronised lyrics or text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncedLyrics {
    /// The ISO 639-2 language code as written.
    pub language: [u8; 3],
    /// The unit of the time stamps: 1 for MPEG frames, 2 for milliseconds.
    pub timestamp_format: u8,
    /// What the text is, such as 1 for lyrics.
    pub content_type: u8,
    /// The content description.
    pub description: Text,
    /// The lines, each with its time stamp, in the order written.
    pub lines: Vec<SyncedText>,
    /// Whether lines of this frame were dropped because the tag had
    /// reached the lyrics line limit, which counts the lines of all its
    /// synchronised lyrics frames together.
    pub truncated: bool,
}

/// One line of synchronised text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncedText {
    /// The text.
    pub text: Text,
    /// When it starts, in the unit of the frame's time stamp format.
    pub time: u32,
}

/// An attached picture: what it says about itself, and where its octets
/// are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureRef {
    /// The MIME type the frame declares, or in 2.2 its three-character
    /// image format. Only the picture's own octets say what it is.
    pub mime: Text,
    /// The picture type, such as 3 for a front cover.
    pub picture_type: u8,
    /// The picture's description.
    pub description: Text,
    /// Where the picture's octets are.
    pub data: Span,
}
