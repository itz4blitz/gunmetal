//! Where a file's artwork is (API-CAT-07's input).
//!
//! An [`ArtworkRef`] says where a picture is and what it shows; it never
//! holds the picture. Decoding and resizing happen in the worker
//! (WP-079).

use super::coded::coded;

coded! {
    /// What a picture shows: the picture types of `ID3v2`'s `APIC` frame,
    /// which FLAC's `PICTURE` block shares.
    PictureType: u8 {
        /// Another picture.
        Other = 0,
        /// A 32 by 32 pixel PNG file icon.
        FileIcon = 1,
        /// Another file icon.
        OtherFileIcon = 2,
        /// The front cover.
        FrontCover = 3,
        /// The back cover.
        BackCover = 4,
        /// A leaflet page.
        Leaflet = 5,
        /// The media itself, such as the label side of a CD.
        Media = 6,
        /// The lead artist or soloist.
        LeadArtist = 7,
        /// The artist or performer.
        Artist = 8,
        /// The conductor.
        Conductor = 9,
        /// The band or orchestra.
        Band = 10,
        /// The composer.
        Composer = 11,
        /// The lyricist.
        Lyricist = 12,
        /// The recording location.
        RecordingLocation = 13,
        /// A picture taken during recording.
        DuringRecording = 14,
        /// A picture taken during a performance.
        DuringPerformance = 15,
        /// A screen capture from a film or video.
        ScreenCapture = 16,
        /// A bright coloured fish, as the `ID3v2` specification lists.
        BrightColouredFish = 17,
        /// An illustration.
        Illustration = 18,
        /// The band's or artist's logo.
        BandLogo = 19,
        /// The publisher's or studio's logo.
        PublisherLogo = 20,
    }
}

/// Where a picture is.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArtworkSource {
    /// Embedded in the file: the picture at this position among the file's
    /// pictures, counting from 0.
    Embedded {
        /// The position.
        index: u16,
    },
    /// An image file beside the audio file, by its name in the same
    /// folder, such as `cover.jpg`. The library walk finds it (WP-060).
    Sidecar {
        /// The file's name.
        name: String,
    },
}

/// A picture for an item: where it is, what it shows and how big it is
/// encoded.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArtworkRef {
    /// Where the picture is.
    pub source: ArtworkSource,
    /// What it shows.
    pub picture_type: PictureType,
    /// Its encoded size in octets.
    pub byte_len: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picture_types_have_the_id3v2_codes() {
        let types: Vec<(PictureType, u8)> =
            PictureType::ALL.iter().map(|t| (*t, t.code())).collect();
        assert_eq!(
            types,
            [
                (PictureType::Other, 0),
                (PictureType::FileIcon, 1),
                (PictureType::OtherFileIcon, 2),
                (PictureType::FrontCover, 3),
                (PictureType::BackCover, 4),
                (PictureType::Leaflet, 5),
                (PictureType::Media, 6),
                (PictureType::LeadArtist, 7),
                (PictureType::Artist, 8),
                (PictureType::Conductor, 9),
                (PictureType::Band, 10),
                (PictureType::Composer, 11),
                (PictureType::Lyricist, 12),
                (PictureType::RecordingLocation, 13),
                (PictureType::DuringRecording, 14),
                (PictureType::DuringPerformance, 15),
                (PictureType::ScreenCapture, 16),
                (PictureType::BrightColouredFish, 17),
                (PictureType::Illustration, 18),
                (PictureType::BandLogo, 19),
                (PictureType::PublisherLogo, 20),
            ]
        );
        let read: Vec<Option<PictureType>> = (0..=21).map(PictureType::from_code).collect();
        let mut expected: Vec<Option<PictureType>> =
            PictureType::ALL.iter().copied().map(Some).collect();
        expected.push(None);
        assert_eq!(read, expected);
        assert_eq!(PictureType::from_code(u8::MAX), None);
    }
}
