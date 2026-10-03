//! The catalogue's shared vocabulary: the music model as plain data, with
//! constructors that refuse combinations that cannot exist.
//!
//! The catalogue store returns [`CatalogChange`]s. None of it holds logic
//! beyond construction and validation.
//!
//! This file is a registry: it holds only module lines and re-exports.

pub mod artwork;
pub mod change;
mod coded;
pub mod credit;
pub mod error;
pub mod ids;
pub mod kind;
pub mod lyrics;
pub mod playback;
pub mod position;
pub mod release;
pub mod tech;

pub use artwork::{ArtworkRef, ArtworkSource, PictureType};
pub use change::{CatalogChange, ChangeOp};
pub use credit::{Credit, Role};
pub use error::CatalogError;
pub use ids::{AlbumId, ArtistId, LibraryId, RecordId, TrackId};
pub use kind::{ItemKind, RecordKind};
pub use lyrics::{LyricsOrigin, LyricsSource, LyricsTiming, TagLyrics};
pub use playback::{Gain, GainScale, GainTags, Trim};
pub use position::{PositionPart, TrackPosition};
pub use release::{Advisory, PrimaryType, ReleaseType, SecondaryType};
pub use tech::{AudioFormat, Bitrate, Codec, Container, TechInfo};
